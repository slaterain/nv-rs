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
//!
//! `TESObjectREFR::InitScript` (`00565730`), the script of a form (`004826d0`), the reference's
//! container (`0055d310`) and the actor classes' slot 0x1a4 (`008ae600`) run on a reference, its
//! forms and its extra data list from a seed, the work `InitScript` hands on standing in.
//!
//! The extra data list's functions (`0040ec80` to `00411ea0`) run on a list from a seed: its chain,
//! its type bitmap, the thread's cache, the extra data lock and JIP's head site, the extra data's
//! delete and the reference lists' destructors standing in; its constructors run on junk.

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
    // The cast: the word its object keeps (null for none, as the exe's).
    e.register(0x00ec_43fb, |e, a| {
        ret(if a[0] == 0 {
            0
        } else {
            e.mem.u32(a[0] + ANSWER_CAST)
        })
    });
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
/// What a stand-in slot 0x1a4 answers (`InitScript`'s form).
const ANSWER_SCRIPT_FORM: u32 = 0x120;
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
/// `00b5d9f0`) on its queues, their lock and the list node pool of [`lighting_stand_ins`], and
/// `TESObjectREFR::InitScript` (`00565730`) on the reference, its callees standing in
/// ([`script_stand_ins`]).
const SCENE_DOUBLES: [u32; 7] = [
    0x0056_5210, // TESActorBase::SetStartsDead
    0x00b5_cbd0, // the shadow scene node's actors' add
    0x0056_f700, // the step after the attach
    0x0057_8170, // the master particle addon nodes' removal
    0x0057_8060, // and addition
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
    script_stand_ins(e);
    e.register(VT_FN + 0x1a4, answer_of::<ANSWER_SCRIPT_FORM>);
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
        0x100, 0x1a4, 0x1c8, 0x1d0, 0x1f4, 0x220, 0x224, 0x260, 0x304, 0x308,
    ] {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    if class != 3 {
        // The slot 0x1a4 of `TESObjectREFR` and `Explosion` (0) and of the actor classes (+0x148).
        let script_form = if class == 1 { 0x008a_e600 } else { 0x00ac_bb70 };
        e.mem.set_u32(vt + 0x1a4, script_form);
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
    let r = e.mem.alloc(0x150);
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
    // `InitScript`'s words: the form an actor's slot 0x1a4 (+0x148) and the stand-in's answer (none,
    // the base form, or a form of its own with a script), and now and then a script in the base
    // form's scriptable form (the cast's +4).
    let form = match d.next(4) {
        2 => base,
        3 => script_form(e, 2),
        _ => 0,
    };
    e.mem.set_u32(r + 0x148, form);
    e.mem.set_u32(r + ANSWER_SCRIPT_FORM, form);
    let scriptable = e.mem.u32(base + ANSWER_CAST);
    if scriptable != 0 && d.next(2) == 0 {
        let script = e.mem.alloc(0x10);
        e.mem.set_u32(scriptable + 4, script);
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
    // family's), and `InitScript`'s extra data lookup (the dispatch family's).
    .goes_by_address(&[0x0040_fbf0, 0x0043_a010, 0x0041_0220])
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
    "a script set",
    "the inventory changes' scripts set up",
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
                (0x0041_9ed0, "a script set"),
                (0x004d_1960, "the inventory changes' scripts set up"),
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

// `TESObjectREFR::InitScript` (`00565730`) and what it asks of the reference: the actor classes'
// slot 0x1a4 (`008ae600`), the form's script (`004826d0`) and the base form's container
// (`0055d310`).
//
// Their world, from a seed: a reference, deleted now and then (form flag 0x20, alone or with
// 0x800), whose class's slot 0x1a4 holds `TESObjectREFR`'s body (0), the actor classes' (the word
// at +0x148) or a stand-in answering a word kept in the reference, that word none, the base form
// or a form of its own; its base form (none now and then) of type 0x1b, 0x2a or 0x2b (a container)
// or 0x29, 0x2c, 0x15, 0x1a, 0x1c (none); each form's `TESScriptableForm` (the cast's answer) none,
// one without a script or one with a script; its extra data list's chain of none to three extra
// data of types 0x1d, 0x15 and 0xd (the script extra, its script none or one), the type bitmap
// as the chain says and, now and then, type 0xd's bit the other way. The lookup's thread cache
// starts empty and the extra data lock free. JIP's patches are all on for half the seeds, off for
// the others: nothing of JIP's is reached, and the same code runs either way.

/// The memory contexts' depth, which their doubles keep.
const SCRIPT_SCOPE_DEPTH: u32 = CONTROL + 0x900;

/// What `InitScript` hands on, standing in: the memory contexts (their depth kept, each scope given
/// the context it entered), the script's and the event list's set into the list and the inventory
/// changes' scripts (their words in the call log), the event list made (an answer from the script)
/// and the inventory changes (an answer from the reference).
fn script_stand_ins(e: &mut Engine) {
    e.register(0x0040_4eb0, |e, a| {
        e.mem
            .set_u32(SCRIPT_SCOPE_DEPTH, e.mem.u32(SCRIPT_SCOPE_DEPTH) + 1);
        e.mem.set_u32(a[0], a[1]);
        ret(a[0])
    });
    e.register(0x0040_4ee0, |e, a| {
        e.mem
            .set_u32(SCRIPT_SCOPE_DEPTH, e.mem.u32(SCRIPT_SCOPE_DEPTH) - 1);
        e.mem.set_u32(a[0], 0xdead);
        Ret::default()
    });
    for addr in [0x0041_9ed0, 0x0041_9f80, 0x004d_1960] {
        e.register(addr, nothing);
    }
    e.register(0x005a_bf60, |_, a| ret(a[0] ^ 0x0a00_0000));
    e.register(0x004b_f220, |_, a| ret(a[0] + 0x30));
}

/// A form whose scriptable form (the cast's answer) is none (`scripted` 0), has no script (1) or
/// has one (2).
fn script_form(e: &mut Engine, scripted: u32) -> u32 {
    let form = e.mem.alloc(0x120);
    if scripted != 0 {
        let scriptable = e.mem.alloc(8);
        if scripted == 2 {
            let script = e.mem.alloc(0x10);
            e.mem.set_u32(scriptable + 4, script);
        }
        e.mem.set_u32(form + ANSWER_CAST, scriptable);
    }
    form
}

/// The bitmap's byte and bit of an extra data type.
fn extra_bit(list: u32, kind: u8) -> (u32, u8) {
    (list + 8 + u32::from(kind >> 3), 1 << (kind & 7))
}

/// The script's world of a seed: the reference.
fn script_world(e: &mut Engine, seed: u32) -> u32 {
    stand_ins(e);
    lighting_stand_ins(e);
    script_stand_ins(e);
    e.register(VT_FN + 0x1a4, answer_of::<ANSWER_SCRIPT_FORM>);
    let mut d = Draw(seed.wrapping_mul(0x9e37_79b9) ^ 0x5730);
    e.set_jip_patches(if seed % 2 == 0 { 0 } else { u32::MAX });
    let vt = e.mem.alloc(0x200);
    let slot = [0x00ac_bb70, 0x008a_e600, VT_FN + 0x1a4][d.next(3) as usize];
    e.mem.set_u32(vt + 0x1a4, slot);
    let r = e.mem.alloc(0x150);
    e.mem.set_u32(r, vt);
    e.mem.set_u32(
        r + 8,
        [0, 0, 0, 0x20, 0x820, 0x800, 0x400][d.next(7) as usize],
    );
    let base = if d.next(8) == 0 {
        0
    } else {
        let base = script_form(e, d.next(3));
        let kind = [0x1b, 0x2a, 0x2b, 0x29, 0x2c, 0x15, 0x1a, 0x1c][d.next(8) as usize];
        e.mem.set_u8(base + 4, kind);
        base
    };
    e.mem.set_u32(r + 0x20, base);
    let form = match d.next(4) {
        0 => 0,
        1 => base,
        _ => script_form(e, d.next(3)),
    };
    e.mem.set_u32(r + 0x148, form);
    e.mem.set_u32(r + ANSWER_SCRIPT_FORM, form);
    // The extra data list (+0x44): its chain (+0x48), built from its end, and the type bitmap.
    let list = r + 0x44;
    let mut next = 0;
    for _ in 0..d.next(4) {
        let kind = [0x1d, 0x15, 0xd][d.next(3) as usize];
        let extra = e.mem.alloc(0x18);
        e.mem.set_u32(extra, 0x0101_0000);
        e.mem.set_u8(extra + 4, kind);
        e.mem.set_u32(extra + 8, next);
        if kind == 0xd && d.next(2) == 0 {
            let script = e.mem.alloc(0x10);
            e.mem.set_u32(extra + 0xc, script);
        }
        let (byte, bit) = extra_bit(list, kind);
        e.mem.set_u8(byte, e.mem.u8(byte) | bit);
        next = extra;
    }
    e.mem.set_u32(list + 4, next);
    if d.next(5) == 0 {
        let (byte, bit) = extra_bit(list, 0xd);
        e.mem.set_u8(byte, e.mem.u8(byte) ^ bit);
    }
    r
}

/// The calls `InitScript` does in place: the form type (in the container's test) and the extra
/// data list.
const SCRIPT_INLINED: &[u32] = &[0x0040_1170, 0x005d_43c0];

/// `TESObjectREFR::InitScript` against its translation, over the seeds' worlds.
#[test]
fn reference_init_script() {
    Diff::new(entry!(0x0056_5730, cell_refs::script::init_script(u32)))
        .family(&CELL_REFS)
        .family(&leaf::LEAVES)
        // The extra data lookup (the dispatch family's) on its translation.
        .goes_by_address(&[0x0041_0220])
        .ignore_calls(SCRIPT_INLINED)
        .inputs(&[Input::Of(SEEDS)])
        .setup(|e, case| vec![script_world(e, case.word(0))])
        .run()
        .assert_same();
}

/// The script of a form against its translation: a null form, one without a scriptable form, one
/// whose scriptable form has no script, one with a script.
#[test]
fn the_script_of_a_form() {
    Diff::new(entry!(
        0x0048_26d0,
        cell_refs::script::form_script(u32) -> u32
    ))
    .family(&CELL_REFS)
    .family(&leaf::LEAVES)
    .inputs(&[Input::Of(&[0, 1, 2, 3])])
    .setup(|e, case| {
        stand_ins(e);
        let form = match case.word(0) {
            3 => 0,
            scripted => script_form(e, scripted),
        };
        vec![form]
    })
    .run()
    .assert_same();
}

/// `TESObjectREFR::HasContainer` against its translation: no base form (0x100), and a base form of
/// each type around the three with a container.
#[test]
fn reference_has_container() {
    Diff::new(entry!(
        0x0055_d310,
        cell_refs::script::has_container(u32) -> u32
    ))
    .family(&CELL_REFS)
    .family(&leaf::LEAVES)
    .ignore_calls(&[0x0040_1170])
    .inputs(&[Input::Of(&[
        0, 0x1a, 0x1b, 0x1c, 0x29, 0x2a, 0x2b, 0x2c, 0xff, 0x100,
    ])])
    .setup(|e, case| {
        let r = e.mem.alloc(0x40);
        if case.word(0) < 0x100 {
            let base = e.mem.alloc(0x80);
            e.mem.set_u8(base + 4, case.word(0) as u8);
            e.mem.set_u32(r + 0x20, base);
        }
        vec![r]
    })
    .run()
    .assert_same();
}

/// The actor classes' slot 0x1a4 against its translation.
#[test]
fn actor_script_form() {
    Diff::new(entry!(
        0x008a_e600,
        cell_refs::script::actor_form_148(u32) -> u32
    ))
    .family(&CELL_REFS)
    .inputs(&[Input::Of(&[0, 0x0700_0000, u32::MAX])])
    .setup(|e, case| {
        let actor = e.mem.alloc(0x150);
        e.mem.set_u32(actor + 0x148, case.word(0));
        vec![actor]
    })
    .run()
    .assert_same();
}

/// The branches [`the_script_seeds_reach_every_branch`] asks the seeds to reach.
const SCRIPT_BRANCHES: &[&str] = &[
    "JIP's patches off",
    "JIP's patches on",
    "a deleted reference left",
    "a reference already scripted left",
    "no script extra",
    "a script extra without a script",
    "a script extra its bit does not show",
    "the bit with no script extra",
    "the form of slot 0x1a4",
    "the base form for TESObjectREFR's slot",
    "the base form for an actor's empty word",
    "the base form for a stand-in's empty answer",
    "no form at all",
    "a form without a scriptable form",
    "a scriptable form without a script",
    "a script set",
    "a container's inventory",
    "an NPC's inventory",
    "a creature's inventory",
    "no container",
];

/// The script's worlds reach every branch it has, checked on the translation.
#[test]
fn the_script_seeds_reach_every_branch() {
    let mut reached = std::collections::BTreeMap::<&str, u32>::new();
    for &seed in SEEDS {
        let mut e = Engine::new_untyped_for_test();
        let r = script_world(&mut e, seed);
        let deleted = e.mem.u32(r + 8) & 0x20 != 0;
        let slot = e.mem.u32(e.mem.u32(r) + 0x1a4);
        let (form, base) = (e.mem.u32(r + 0x148), e.mem.u32(r + 0x20));
        // The chain's first script extra, and whether the bitmap shows type 0xd.
        let list = r + 0x44;
        let (byte, bit) = extra_bit(list, 0xd);
        let shown = e.mem.u8(byte) & bit != 0;
        let mut extra = e.mem.u32(list + 4);
        while extra != 0 && e.mem.u8(extra + 4) != 0xd {
            extra = e.mem.u32(extra + 8);
        }
        e.call_log = Some(Vec::new());
        e.call(0x0056_5730, &args![r]);
        let log = e.call_log.take().unwrap_or_default();
        let words = |addr: u32| -> Vec<Vec<u32>> {
            log.iter()
                .filter(|(a, _)| *a == addr)
                .map(|(_, w)| w.clone())
                .collect()
        };
        let called = |addr: u32| !words(addr).is_empty();
        let mut hit = |name| *reached.entry(name).or_default() += 1;
        hit(if e.jip_patches() == 0 {
            "JIP's patches off"
        } else {
            "JIP's patches on"
        });
        assert_eq!(e.mem.u32(SCRIPT_SCOPE_DEPTH), 0, "the memory context left");
        if deleted {
            assert!(!called(0x0040_4eb0));
            hit("a deleted reference left");
            continue;
        }
        let script_of_extra = if shown && extra != 0 {
            e.mem.u32(extra + 0xc)
        } else {
            0
        };
        if script_of_extra != 0 {
            assert!(!called(0x0048_26d0));
            hit("a reference already scripted left");
            continue;
        }
        hit(match (shown, extra != 0) {
            (true, true) => "a script extra without a script",
            (false, true) => "a script extra its bit does not show",
            (true, false) => "the bit with no script extra",
            (false, false) => "no script extra",
        });
        let asked = words(0x0048_26d0)[0][0];
        if form != 0 && slot != 0x00ac_bb70 {
            assert_eq!(asked, form);
            hit("the form of slot 0x1a4");
        } else if base == 0 {
            hit("no form at all");
        } else {
            assert_eq!(asked, base);
            hit(match slot {
                0x00ac_bb70 => "the base form for TESObjectREFR's slot",
                0x008a_e600 => "the base form for an actor's empty word",
                _ => "the base form for a stand-in's empty answer",
            });
        }
        let scriptable = if asked == 0 {
            0
        } else {
            e.mem.u32(asked + ANSWER_CAST)
        };
        if called(0x0041_9ed0) {
            hit("a script set");
        } else if scriptable == 0 {
            hit("a form without a scriptable form");
        } else {
            hit("a scriptable form without a script");
        }
        if called(0x004d_1960) {
            hit(match e.mem.u8(base + 4) {
                0x1b => "a container's inventory",
                0x2a => "an NPC's inventory",
                _ => "a creature's inventory",
            });
        } else if base != 0 {
            hit("no container");
        }
    }
    for name in SCRIPT_BRANCHES {
        assert!(
            reached.contains_key(name),
            "no seed reaches {name}: {reached:?}"
        );
    }
}

// ---- The extra data list (`cell_refs::extra_list`) ----------------------------------------------
//
// Its world, from a seed: an `ExtraDataList` whose chain holds none to five extra data, each of a
// type drawn from `EXTRA_KINDS`, all sharing a vtable whose slot 0 (the delete) counts at +0x14;
// the type bitmap as the chain says, cleared, full, or with one type's bit flipped; the thread's
// cache holding another list, this list with its types' first extra data, this list from before
// the dirty count moved, or this list with no entries (a decoy extra data in every other entry);
// the extra data lock free or held once by this thread; JIP's patches off, on, or on with a
// handler at its head site (`0040f9d7`) answering "to the head"; and a loose extra data of a drawn
// type in no chain, its next null or the chain's head. The functions' other arguments are each
// Diff's inputs.

/// The types the chains are drawn from: those `AddExtra` puts at the head (0xc, 0xd, 0xe, 0x15,
/// 0x2b), JIP's keyed data (0x27), every type `RemoveAllDefault` keeps (0x2e, 0x38, 0x4f, 0x52,
/// 0x54, 0x58, 0x65, 0x7b, 0x84), another (0x43), the cache's last entry (0x92), types past its
/// entries (0x93, 0xa0) and past the bitmap (0xa8, 0xff). The first 16 are in the bitmap.
const EXTRA_KINDS: &[u32] = &[
    0x0c, 0x0d, 0x0e, 0x15, 0x27, 0x2b, 0x2e, 0x38, 0x43, 0x4f, 0x52, 0x54, 0x58, 0x65, 0x7b, 0x84,
    0x92, 0x93, 0xa0, 0xa8, 0xff,
];
const EXTRA_SEEDS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];
/// Where the extra data's delete (slot 0) is registered; it counts at +0x14 of the extra data.
const EXTRA_VT_FN: u32 = 0x00ff_c100;
const EXTRA_DELETES: u32 = 0x14;
/// The extra data lock (its owner, then its count) and `BaseExtraList::iDirty`.
const EXTRA_LOCK: u32 = 0x011c_3920;
const EXTRA_DIRTY: u32 = 0x011c_38e4;
/// The four reference lists' destructors (standing in: each marks +0x14).
const REF_LIST_DROPS: [u32; 4] = [0x0043_4560, 0x0043_4af0, 0x0043_4950, 0x0043_4dc0];

struct ExtraWorld {
    list: u32,
    chain: Vec<u32>,
    loose: u32,
    bitmap: u32,
    cache_state: u32,
    lock_held: bool,
    jip: u32,
}

impl ExtraWorld {
    /// An extra data by index: the chain's (past its end the loose one), 6 null, 7 the loose one.
    fn pick(&self, i: u32) -> u32 {
        match i {
            6 => 0,
            7 => self.loose,
            _ => self.chain.get(i as usize).copied().unwrap_or(self.loose),
        }
    }
}

/// What the list meets: the lock's imports (as the lighting world has them), the extra data's
/// delete, `operator delete` and the reference lists' destructors.
fn extra_stand_ins(e: &mut Engine) {
    lighting_stand_ins(e);
    e.register(EXTRA_VT_FN, |e, a| {
        let n = e.mem.u32(a[0] + EXTRA_DELETES);
        e.mem.set_u32(a[0] + EXTRA_DELETES, n.wrapping_add(1));
        Ret::default()
    });
    e.register(0x0040_1030, nothing);
    for addr in REF_LIST_DROPS {
        e.register(addr, |e, a| {
            e.mem.set_u32(a[0] + 0x14, 0xd00d);
            Ret::default()
        });
    }
}

/// An extra data of `kind` with the world's vtable, a null next and a payload word of its own.
fn extra_data(e: &mut Engine, vt: u32, kind: u32, i: u32) -> u32 {
    let x = e.mem.alloc(0x18);
    for (k, w) in [vt, kind, 0, 0x1000 + i, 0, 0].into_iter().enumerate() {
        e.mem.set_u32(x + 4 * k as u32, w);
    }
    x
}

/// `n` bytes of 0xa5 (what a constructor writes over).
fn junk(e: &mut Engine, n: u32) -> u32 {
    let x = e.mem.alloc(n);
    for i in 0..n {
        e.mem.set_u8(x + i, 0xa5);
    }
    x
}

/// JIP's handler at its head site: to the head.
fn jip_head(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

fn extra_world(e: &mut Engine, seed: u32) -> ExtraWorld {
    use engine_core::extend::{Owner, SiteSlot};
    e.map(0x011c_3000, 0x1000);
    e.map(CONTROL, 0x1000);
    extra_stand_ins(e);
    let mut d = Draw(seed.wrapping_mul(0x9e37_79b9) ^ 0xf680);
    // JIP: off, installed, installed with its head site, or off with the site's handler still there.
    let jip = seed % 4;
    e.set_jip_patches(if jip % 3 == 0 { 0 } else { u32::MAX });
    if jip >= 2 {
        e.on_site(SiteSlot::new(Owner::Jip, 0x0040_f9d7), jip_head);
    }
    e.mem.set_u32(EXTRA_DIRTY, 5);
    let lock_held = d.next(3) == 0;
    e.mem
        .set_u32(EXTRA_LOCK, if lock_held { LOCK_THREAD } else { 0 });
    e.mem.set_u32(EXTRA_LOCK + 4, u32::from(lock_held));
    let vt = e.mem.alloc(0x10);
    e.mem.set_u32(vt, EXTRA_VT_FN);
    let n = [0, 1, 2, 3, 4, 5, 5, 4][d.next(8) as usize];
    // Every third seed ends its chain with one of the kept types, so each is in some chain.
    const KEPT: [u32; 9] = [0x2e, 0x38, 0x4f, 0x52, 0x54, 0x58, 0x65, 0x7b, 0x84];
    let kinds = EXTRA_KINDS.len() as u32;
    let chain: Vec<u32> = (0..n)
        .map(|i| {
            let kind = if i + 1 == n && seed % 3 == 1 {
                KEPT[(seed / 3 % 9) as usize]
            } else {
                EXTRA_KINDS[d.next(kinds) as usize]
            };
            extra_data(e, vt, kind, i)
        })
        .collect();
    for w in chain.windows(2) {
        e.mem.set_u32(w[0] + 8, w[1]);
    }
    let list = e.mem.alloc(0x20);
    e.mem.set_u32(list, 0x0101_43e8);
    e.mem.set_u32(list + 4, chain.first().copied().unwrap_or(0));
    for &x in &chain {
        let kind = e.mem.u32(x + 4);
        if kind >> 3 < 0x15 {
            let flags = e.mem.u8(list + 8 + (kind >> 3)) | 1 << (kind & 7);
            e.mem.set_u8(list + 8 + (kind >> 3), flags);
        }
    }
    let bitmap = d.next(6);
    match bitmap {
        3 => (0..0x15).for_each(|i| e.mem.set_u8(list + 8 + i, 0)),
        4 => (0..0x15).for_each(|i| e.mem.set_u8(list + 8 + i, 0xff)),
        5 => {
            let kind = EXTRA_KINDS[d.next(16) as usize];
            let flags = e.mem.u8(list + 8 + (kind >> 3)) ^ 1 << (kind & 7);
            e.mem.set_u8(list + 8 + (kind >> 3), flags);
        }
        _ => {}
    }
    let cache = e.tls();
    let decoy = extra_data(e, vt, 0xee, 0xdec0);
    for i in 0..0x94 {
        e.mem.set_u32(cache + 0x10 + 4 * i, decoy);
    }
    // The cache: another list, this list with its entries, this list stale, this list empty, another
    // list stale.
    let cache_state = d.next(5);
    let (cached, dirty) = match cache_state {
        0 => (e.mem.alloc(0x20), 5),
        2 => (list, 4),
        4 => (e.mem.alloc(0x20), 4),
        _ => (list, 5),
    };
    e.mem.set_u32(cache + 8, cached);
    e.mem.set_u32(cache + 0xc, dirty);
    if cache_state == 1 {
        for &x in chain.iter().rev() {
            let kind = e.mem.u32(x + 4);
            if kind < 0x93 {
                e.mem.set_u32(cache + 0x10 + 4 * kind, x);
            }
        }
    } else if cache_state == 3 {
        for i in 0..0x93 {
            e.mem.set_u32(cache + 0x10 + 4 * i, 0);
        }
    }
    let loose = extra_data(e, vt, EXTRA_KINDS[d.next(kinds) as usize], 9);
    if d.next(3) == 0 {
        let head = e.mem.u32(list + 4);
        e.mem.set_u32(loose + 8, head);
    }
    ExtraWorld {
        list,
        chain,
        loose,
        bitmap,
        cache_state,
        lock_held,
        jip,
    }
}

/// The families an extra list function reaches: its own, the dispatch family's lock and type
/// getter, the leaves' next and the C runtime's `memset`.
trait ExtraListCase {
    fn extra_list_case(self) -> Diff;
}

impl ExtraListCase for Diff {
    fn extra_list_case(self) -> Diff {
        self.family(&CELL_REFS)
            .family(&engine_typed::dispatch::DISPATCH)
            .family(&leaf::LEAVES)
            .family(&engine_typed::crt::CRT)
    }
}

const FLAGS_WORDS: &[u32] = &[0, 1, 2, 3];
/// A type argument read as a byte or a word (0x127: high bits).
const KIND_WORDS: &[u32] = &[0, 0x0c, 0x27, 0x2b, 0x43, 0x92, 0x93, 0xa0, 0xff, 0x127];
/// A type argument of the cache's `int`: in range, its ends, past them, negative.
const CACHE_KINDS: &[u32] = &[0, 0x0c, 0x92, 0x93, 0xff, 0xffff_ffff, 0x8000_0000];

#[test]
fn extra_data_new() {
    Diff::new(entry!(
        0x0040_ec80,
        cell_refs::extra_list::extra_data_new(u32, u8) -> u32
    ))
    .family(&CELL_REFS)
    .inputs(&[Input::Of(&[0, 0x43, 0xff, 0x143])])
    .setup(|e, case| vec![junk(e, 0x18), case.word(0)])
    .run()
    .assert_same();
}

#[test]
fn extra_data_drop() {
    Diff::new(entry!(
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(u32)
    ))
    .family(&CELL_REFS)
    .setup(|e, _| vec![junk(e, 0x18)])
    .run()
    .assert_same();
}

#[test]
fn extra_data_set_next() {
    Diff::new(entry!(
        0x0040_3550,
        cell_refs::extra_list::set_next(u32, u32)
    ))
    .family(&CELL_REFS)
    .inputs(&[Input::Of(&[0, 0x1234_5678])])
    .setup(|e, case| vec![junk(e, 0x18), case.word(0)])
    .run()
    .assert_same();
}

/// The constructors of one argument over a block of junk.
trait ExtraConstructor {
    fn on_junk(self) -> Diff;
}

impl ExtraConstructor for Diff {
    fn on_junk(self) -> Diff {
        self.family(&CELL_REFS)
            .family(&engine_typed::crt::CRT)
            .setup(|e, _| vec![junk(e, 0x20)])
    }
}

#[test]
fn extra_north_rotation_new() {
    Diff::new(entry!(
        0x0040_f680,
        cell_refs::extra_list::north_rotation_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_detach_time_new() {
    Diff::new(entry!(
        0x0040_f6b0,
        cell_refs::extra_list::detach_time_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_base_list_new() {
    Diff::new(entry!(
        0x0040_f740,
        cell_refs::extra_list::base_list_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_data_list_new() {
    Diff::new(entry!(
        0x0041_0360,
        cell_refs::extra_list::data_list_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_lock_data_new() {
    Diff::new(entry!(
        0x0041_1b00,
        cell_refs::extra_list::lock_data_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_reflected_refs_new() {
    Diff::new(entry!(
        0x0041_1b40,
        cell_refs::extra_list::reflected_refs_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_reflector_refs_new() {
    Diff::new(entry!(
        0x0041_1be0,
        cell_refs::extra_list::reflector_refs_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_water_light_refs_new() {
    Diff::new(entry!(
        0x0041_1c80,
        cell_refs::extra_list::water_light_refs_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_lit_water_refs_new() {
    Diff::new(entry!(
        0x0041_1d20,
        cell_refs::extra_list::lit_water_refs_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_four_words_new() {
    Diff::new(entry!(
        0x0041_1dc0,
        cell_refs::extra_list::four_words_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_word_16_new() {
    Diff::new(entry!(
        0x0041_1e00,
        cell_refs::extra_list::word_16_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_type_92_new() {
    Diff::new(entry!(
        0x0041_1e40,
        cell_refs::extra_list::type_92_new(u32) -> u32
    ))
    .on_junk()
    .run()
    .assert_same();
}

#[test]
fn extra_type_92_drop() {
    Diff::new(entry!(
        0x0041_1ea0,
        cell_refs::extra_list::type_92_drop(u32)
    ))
    .on_junk()
    .run()
    .assert_same();
}

/// The scalar deleting destructors of an extra data over a block of junk, with the flags 0 to 3.
trait ExtraDeleter {
    fn junk_deleted(self) -> Diff;
}

impl ExtraDeleter for Diff {
    fn junk_deleted(self) -> Diff {
        self.family(&CELL_REFS)
            .inputs(&[Input::Of(FLAGS_WORDS)])
            .setup(|e, case| {
                extra_stand_ins(e);
                vec![junk(e, 0x20), case.word(0)]
            })
    }
}

#[test]
fn extra_reflected_refs_delete() {
    Diff::new(entry!(
        0x0041_1bb0,
        cell_refs::extra_list::reflected_refs_delete(u32, u32) -> u32
    ))
    .junk_deleted()
    .run()
    .assert_same();
}

#[test]
fn extra_reflector_refs_delete() {
    Diff::new(entry!(
        0x0041_1c50,
        cell_refs::extra_list::reflector_refs_delete(u32, u32) -> u32
    ))
    .junk_deleted()
    .run()
    .assert_same();
}

#[test]
fn extra_water_light_refs_delete() {
    Diff::new(entry!(
        0x0041_1cf0,
        cell_refs::extra_list::water_light_refs_delete(u32, u32) -> u32
    ))
    .junk_deleted()
    .run()
    .assert_same();
}

#[test]
fn extra_lit_water_refs_delete() {
    Diff::new(entry!(
        0x0041_1d90,
        cell_refs::extra_list::lit_water_refs_delete(u32, u32) -> u32
    ))
    .junk_deleted()
    .run()
    .assert_same();
}

#[test]
fn extra_type_92_delete() {
    Diff::new(entry!(
        0x0041_1e70,
        cell_refs::extra_list::type_92_delete(u32, u32) -> u32
    ))
    .junk_deleted()
    .run()
    .assert_same();
}

/// `ExtraWeaponModFlags`' test: its byte and the mask, each 0, 1, 0x80 or 0xff (the mask's word
/// also 0x100: its byte alone is read).
#[test]
fn extra_weapon_mod_flags_any() {
    Diff::new(entry!(
        0x0041_1e20,
        cell_refs::extra_list::weapon_mod_flags_any(u32, u8) -> bool
    ))
    .family(&CELL_REFS)
    .inputs(&[
        Input::Of(&[0, 1, 0x80, 0xff]),
        Input::Of(&[0, 1, 0x80, 0xff, 0x100]),
    ])
    .setup(|e, case| {
        let x = junk(e, 0x18);
        e.mem.set_u8(x + 0xc, case.word(0) as u8);
        vec![x, case.word(1)]
    })
    .run()
    .assert_same();
}

/// `BSExtraData::Compare`: the chain's head (or the loose one) against every extra data of the
/// world and null.
#[test]
fn extra_compare() {
    Diff::new(entry!(
        0x0040_f700,
        cell_refs::extra_list::extra_compare(u32, u32) -> bool
    ))
    .extra_list_case()
    .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(&[0, 1, 2, 3, 4, 6, 7])])
    .setup(|e, case| {
        let w = extra_world(e, case.word(0));
        vec![w.pick(0), w.pick(case.word(1))]
    })
    .run()
    .assert_same();
}

/// A destructor of the world's list with the flags 0 to 3.
trait ListDeleted {
    fn list_deleted(self) -> Diff;
}

impl ListDeleted for Diff {
    fn list_deleted(self) -> Diff {
        self.extra_list_case()
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(FLAGS_WORDS)])
            .setup(|e, case| vec![extra_world(e, case.word(0)).list, case.word(1)])
    }
}

#[test]
fn extra_base_list_delete() {
    Diff::new(entry!(
        0x0040_f780,
        cell_refs::extra_list::base_list_delete(u32, u32) -> u32
    ))
    .list_deleted()
    .run()
    .assert_same();
}

#[test]
fn extra_data_list_delete() {
    Diff::new(entry!(
        0x0041_0380,
        cell_refs::extra_list::data_list_delete(u32, u32) -> u32
    ))
    .list_deleted()
    .run()
    .assert_same();
}

/// A function of the world's list alone.
trait OnList {
    fn on_list(self) -> Diff;
}

impl OnList for Diff {
    fn on_list(self) -> Diff {
        self.extra_list_case()
            .inputs(&[Input::Of(EXTRA_SEEDS)])
            .setup(|e, case| vec![extra_world(e, case.word(0)).list])
    }
}

#[test]
fn extra_base_list_drop() {
    Diff::new(entry!(
        0x0040_f7b0,
        cell_refs::extra_list::base_list_drop(u32)
    ))
    .on_list()
    .run()
    .assert_same();
}

#[test]
fn extra_data_list_drop() {
    Diff::new(entry!(
        0x0041_03b0,
        cell_refs::extra_list::data_list_drop(u32)
    ))
    .on_list()
    .run()
    .assert_same();
}

#[test]
fn extra_clear_last_extra_all() {
    Diff::new(entry!(
        0x0040_f860,
        cell_refs::extra_list::clear_last_extra_all(u32)
    ))
    .on_list()
    .run()
    .assert_same();
}

#[test]
fn extra_items_in_list() {
    Diff::new(entry!(
        0x0040_fe20,
        cell_refs::extra_list::items_in_list(u32) -> i32
    ))
    .on_list()
    .run()
    .assert_same();
}

#[test]
fn extra_drop_stale_cache() {
    Diff::new(entry!(
        0x0040_f900,
        cell_refs::extra_list::drop_stale_cache()
    ))
    .extra_list_case()
    .inputs(&[Input::Of(EXTRA_SEEDS)])
    .setup(|e, case| {
        extra_world(e, case.word(0));
        Vec::new()
    })
    .run()
    .assert_same();
}

/// The world's list and a type for the cache's `int` (`CACHE_KINDS`).
trait ListAndCacheKind {
    fn list_and_cache_kind(self) -> Diff;
}

impl ListAndCacheKind for Diff {
    fn list_and_cache_kind(self) -> Diff {
        self.extra_list_case()
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(CACHE_KINDS)])
            .setup(|e, case| vec![extra_world(e, case.word(0)).list, case.word(1)])
    }
}

#[test]
fn extra_clear_last_extra() {
    Diff::new(entry!(
        0x0040_f7d0,
        cell_refs::extra_list::clear_last_extra(u32, i32)
    ))
    .list_and_cache_kind()
    .run()
    .assert_same();
}

#[test]
fn extra_cached_extra() {
    Diff::new(entry!(
        0x0040_f9e0,
        cell_refs::extra_list::cached_extra(u32, i32) -> u32
    ))
    .list_and_cache_kind()
    .run()
    .assert_same();
}

/// The cache's store: the world's list (or none) and each extra data of the world (or none).
#[test]
fn extra_cache_extra() {
    Diff::new(entry!(
        0x0040_fa40,
        cell_refs::extra_list::cache_extra(u32, u32)
    ))
    .extra_list_case()
    .inputs(&[
        Input::Of(EXTRA_SEEDS),
        Input::Of(&[0, 1, 2, 6, 7]),
        Input::Flag,
    ])
    .setup(|e, case| {
        let w = extra_world(e, case.word(0));
        let list = if case.word(2) == 0 { w.list } else { 0 };
        vec![list, w.pick(case.word(1))]
    })
    .run()
    .assert_same();
}

/// Whether a type goes to the head, under JIP's three states, for each type and a word with high
/// bits.
#[test]
fn extra_added_at_head() {
    Diff::new(entry!(
        0x0040_f980,
        cell_refs::extra_list::added_at_head(u32) -> bool
    ))
    .extra_list_case()
    .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5]), Input::Of(KIND_WORDS)])
    .setup(|e, case| {
        extra_world(e, case.word(0));
        vec![case.word(1)]
    })
    .run()
    .assert_same();
}

/// The world's list with a flag (`destroy`).
trait ListAndFlag {
    fn list_and_flag(self) -> Diff;
}

impl ListAndFlag for Diff {
    fn list_and_flag(self) -> Diff {
        self.extra_list_case()
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Flag])
            .setup(|e, case| vec![extra_world(e, case.word(0)).list, case.word(1)])
    }
}

#[test]
fn extra_remove_all() {
    Diff::new(entry!(
        0x0040_fae0,
        cell_refs::extra_list::remove_all(u32, bool)
    ))
    .list_and_flag()
    .run()
    .assert_same();
}

#[test]
fn extra_remove_all_default() {
    Diff::new(entry!(
        0x0040_fcb0,
        cell_refs::extra_list::remove_all_default(u32, bool)
    ))
    .list_and_flag()
    .run()
    .assert_same();
}

/// The world's list and every type of `EXTRA_KINDS`.
trait ListAndKind {
    fn list_and_kind(self) -> Diff;
}

impl ListAndKind for Diff {
    fn list_and_kind(self) -> Diff {
        self.extra_list_case()
            .max_cases(EXTRA_SEEDS.len() * EXTRA_KINDS.len())
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(EXTRA_KINDS)])
            .setup(|e, case| vec![extra_world(e, case.word(0)).list, case.word(1)])
    }
}

#[test]
fn extra_has_extra_type() {
    Diff::new(entry!(
        0x0040_fe80,
        cell_refs::extra_list::has_extra_type(u32, u8) -> bool
    ))
    .list_and_kind()
    .run()
    .assert_same();
}

#[test]
fn extra_remove_extra_of_type() {
    Diff::new(entry!(
        0x0041_0140,
        cell_refs::extra_list::remove_extra_of_type(u32, u8)
    ))
    .list_and_kind()
    .run()
    .assert_same();
}

#[test]
fn extra_get_prev_extra_data() {
    Diff::new(entry!(
        0x0041_02d0,
        cell_refs::extra_list::get_prev_extra_data(u32, u8) -> u32
    ))
    .list_and_kind()
    .run()
    .assert_same();
}

/// The bit of a type set or cleared: types in the bitmap, its last byte's (0xa7), past it, and a
/// word with high bits.
#[test]
fn extra_set_extra_flag() {
    Diff::new(entry!(
        0x0040_fee0,
        cell_refs::extra_list::set_extra_flag(u32, u8, bool)
    ))
    .extra_list_case()
    .inputs(&[
        Input::Of(EXTRA_SEEDS),
        Input::Of(&[0x0c, 0x27, 0x92, 0xa0, 0xa7, 0xa8, 0xff, 0x127]),
        Input::Flag,
    ])
    .setup(|e, case| {
        vec![
            extra_world(e, case.word(0)).list,
            case.word(1),
            case.word(2),
        ]
    })
    .run()
    .assert_same();
}

/// `AddExtra` of a new extra data of each type into the world's list.
#[test]
fn extra_add_extra() {
    Diff::new(entry!(
        0x0040_ff60,
        cell_refs::extra_list::add_extra(u32, u32) -> u32
    ))
    .extra_list_case()
    .max_cases(EXTRA_SEEDS.len() * EXTRA_KINDS.len())
    .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(EXTRA_KINDS)])
    .setup(|e, case| {
        let w = extra_world(e, case.word(0));
        let vt = e.mem.u32(w.loose);
        let extra = extra_data(e, vt, case.word(1), 0x20);
        vec![w.list, extra]
    })
    .run()
    .assert_same();
}

/// `RemoveExtra` of each extra data of the world (the chain's first five, none, the loose one),
/// deleted or detached.
#[test]
fn extra_remove_extra() {
    Diff::new(entry!(
        0x0041_0020,
        cell_refs::extra_list::remove_extra(u32, u32, bool)
    ))
    .extra_list_case()
    .inputs(&[
        Input::Of(EXTRA_SEEDS),
        Input::Of(&[0, 1, 2, 3, 4, 6, 7]),
        Input::Flag,
    ])
    .setup(|e, case| {
        let w = extra_world(e, case.word(0));
        vec![w.list, w.pick(case.word(1)), case.word(2)]
    })
    .run()
    .assert_same();
}

/// The list's worlds reach what its functions branch on, checked on the world itself: an empty
/// chain and one of five, a head type behind another extra data, a type twice in a chain, a kept
/// type before one `RemoveAllDefault` removes and after one, a type past the bitmap in a chain,
/// every bitmap and cache state, the lock free and held, JIP's three states, and a loose extra data
/// whose next is the head.
#[test]
fn the_extra_list_seeds_reach_every_branch() {
    let keeps = |k: u32| {
        matches!(
            k,
            0x2e | 0x38 | 0x4f | 0x52 | 0x54 | 0x58 | 0x65 | 0x7b | 0x84
        )
    };
    let mut reached = std::collections::BTreeSet::<String>::new();
    for &seed in EXTRA_SEEDS {
        let mut e = Engine::new_untyped_for_test();
        let w = extra_world(&mut e, seed);
        let kinds: Vec<u32> = w.chain.iter().map(|&x| e.mem.u32(x + 4)).collect();
        reached.insert(format!("a chain of {}", kinds.len()));
        reached.insert(format!("bitmap {}", w.bitmap.max(2) - 2));
        reached.insert(format!("cache state {}", w.cache_state));
        reached.insert(format!("lock held {}", w.lock_held));
        reached.insert(format!("JIP {}", w.jip));
        if kinds
            .iter()
            .skip(1)
            .any(|k| matches!(k, 0x0c | 0x0d | 0x0e | 0x15 | 0x2b))
        {
            reached.insert("a head type behind another".into());
        }
        if kinds
            .iter()
            .enumerate()
            .any(|(i, k)| kinds[..i].contains(k))
        {
            reached.insert("a type twice".into());
        }
        if kinds.windows(2).any(|p| keeps(p[0]) && !keeps(p[1])) {
            reached.insert("a removal after a kept one".into());
        }
        if kinds.windows(2).any(|p| !keeps(p[0]) && keeps(p[1])) {
            reached.insert("a kept one after a removal".into());
        }
        for &k in kinds.iter().filter(|&&k| keeps(k)) {
            reached.insert(format!("kept type {k:#x} in a chain"));
        }
        if kinds.iter().any(|k| k >> 3 >= 0x15) {
            reached.insert("a type past the bitmap".into());
        }
        if !w.chain.is_empty() && e.mem.u32(w.loose + 8) == w.chain[0] {
            reached.insert("a loose one whose next is the head".into());
        }
    }
    let mut wanted: Vec<String> = (0..=5).map(|n| format!("a chain of {n}")).collect();
    wanted.extend((0..=3).map(|n| format!("bitmap {n}")));
    wanted.extend((0..=4).map(|n| format!("cache state {n}")));
    wanted.extend(["lock held true", "lock held false"].map(String::from));
    wanted.extend((0..=3).map(|n| format!("JIP {n}")));
    wanted.extend(
        [0x2e, 0x38, 0x4f, 0x52, 0x54, 0x58, 0x65, 0x7b, 0x84]
            .map(|k| format!("kept type {k:#x} in a chain")),
    );
    wanted.extend(
        [
            "a head type behind another",
            "a type twice",
            "a removal after a kept one",
            "a kept one after a removal",
            "a type past the bitmap",
            "a loose one whose next is the head",
        ]
        .map(String::from),
    );
    for name in wanted {
        assert!(
            reached.contains(&name),
            "no seed reaches {name}: {reached:?}"
        );
    }
}

// ---- The extra data family (`extra_data`) -------------------------------------------------------

use engine_typed::extra_data::{self, EXTRA_DATA};

/// What the extra data family does in place where its translation calls: the one-instruction
/// leaves (an extra data's type and next, the next's store, a node's item and next, a handle's word,
/// a buffer's form), the type bitmap's test, `BSExtraData`'s constructor, a sound handle's empty
/// destructor, the navmesh portal record's copy, the audio buoy marker's empty copy, the
/// family's own sound handle, mask, flag, colour and `sincos` bodies, and the part 2 range's: a
/// form's id, a package's type and time stamp, a stamp's byte, the save/load singleton's test, an
/// `NiPointer`'s compare and a temporary one made from null with its release.
const XD_IN_PLACE: &[u32] = &[
    0x0045_34f0,
    0x004f_1540,
    0x0044_ddc0,
    0x0040_3550,
    0x0068_15c0,
    0x0072_6070,
    0x0055_9450,
    0x007a_f430,
    0x0040_fe80,
    0x0040_ec80,
    0x0048_3710,
    0x0069_a690,
    0x0041_88d0,
    0x0041_8900,
    0x0041_a250,
    0x0041_a1f0,
    0x0041_a220,
    0x0041_8be0,
    0x0041_1e20,
    0x0041_21b0,
    0x0041_69a0,
    0x0041_4430,
    0x0084_e3a0,
    0x0041_ca90,
    0x0047_c850,
    0x0082_2510,
    0x0086_a460,
    0x0063_3c90,
    0x0045_cec0,
    0x0041_d8a0,
];

/// The families an extra data function reaches (the extra list's with its lock, type getter,
/// leaves and `memset`, and its own), with what it does in place.
trait ExtraDataCase {
    fn extra_data_case(self) -> Diff;
    fn extra_data_case_but(self, own: u32) -> Diff;
}

impl ExtraDataCase for Diff {
    fn extra_data_case(self) -> Diff {
        self.extra_list_case()
            .family(&EXTRA_DATA)
            .ignore_calls(XD_IN_PLACE)
    }

    /// For a function the family does in place where others call it: its own address is not ignored.
    fn extra_data_case_but(self, own: u32) -> Diff {
        let others: Vec<u32> = XD_IN_PLACE.iter().copied().filter(|&a| a != own).collect();
        self.extra_list_case()
            .family(&EXTRA_DATA)
            .ignore_calls(&others)
    }
}

/// The world's extra data vtable, whether `operator new` fails, and the stand-ins' call count.
const XD_VT: u32 = CONTROL + 0x300;
const XD_NEW_FAILS: u32 = CONTROL + 0x304;
const XD_NOTES: u32 = CONTROL + 0x308;
/// The world vtable's `Compare` (slot 1) and slot 3 (the type 0x92 copy's).
const XD_COMPARE_FN: u32 = 0x00ff_c108;
const XD_SLOT_3_FN: u32 = 0x00ff_c10c;
/// Payload words: small, a signalling NaN, 1.0, all ones, zero, a word with only bit 15 and bit 0.
const XD_WORDS: [u32; 6] = [0x1000, 0x7fa0_0001, 0x3f80_0000, u32::MAX, 0, 0x8001];

/// A stand-in that counts and answers its first word (a constructor's block, a setter's list).
fn noted(e: &mut Engine, a: &[u32]) -> Ret {
    let n = e.mem.u32(XD_NOTES);
    e.mem.set_u32(XD_NOTES, n.wrapping_add(1));
    ret(a.first().copied().unwrap_or(0))
}

/// `operator new`: a block of junk, or none when the world says so.
fn xd_new(e: &mut Engine, a: &[u32]) -> Ret {
    if e.mem.u32(XD_NEW_FAILS) != 0 {
        return ret(0);
    }
    let block = e.mem.alloc(a[0].max(0x20));
    for i in 0..a[0].max(0x20) {
        e.mem.set_u8(block + i, 0xa5);
    }
    ret(block)
}

/// A constructor of an extra data of type `K`: the world's vtable, the type, no next, the words it
/// was handed from +0xc.
fn built<const K: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let vt = e.mem.u32(XD_VT);
    e.mem.set_u32(a[0], vt);
    e.mem.set_u32(a[0] + 4, K);
    e.mem.set_u32(a[0] + 8, 0);
    for (i, &w) in a.iter().skip(1).take(3).enumerate() {
        e.mem.set_u32(a[0] + 0xc + 4 * i as u32, w);
    }
    ret(a[0])
}

/// `Compare`: whether the two payloads differ in their low bit.
fn xd_compare(e: &mut Engine, a: &[u32]) -> Ret {
    ret((e.mem.u32(a[0] + 0xc) ^ e.mem.u32(a[1] + 0xc)) & 1)
}

/// The get-or-create of `AddExtraCopy`'s type 0x8b: a block.
fn xd_get_or_create(e: &mut Engine, _: &[u32]) -> Ret {
    ret(e.mem.alloc(0x40))
}

/// What the family calls by address in other units: each a stand-in that counts. The `memcpy`
/// wrapper (the C runtime's) runs as itself.
const XD_NOTED: &[u32] = &[
    0x0040_1080,
    0x0040_7a90,
    0x0043_7240,
    0x0041_8d20,
    0x0066_65a0,
    0x0043_8bf0,
    0x0058_9500,
    0x007b_3fa0,
    0x0056_4db0,
    0x005a_bf60,
    0x005b_5e40,
    0x00ec_43fb,
    0x0041_e160,
    0x0042_1540,
    0x0041_e250,
    0x0042_1430,
    0x0042_1b70,
    0x0041_e440,
    0x0042_1e40,
    0x0042_1d50,
    0x0042_1c60,
    0x0042_2220,
    0x0042_2350,
    0x0042_dde0,
    0x0042_2050,
    0x0042_2150,
    0x0042_0f00,
    0x0043_a160,
    0x0043_a810,
    0x0043_8bb0,
    0x0043_8df0,
    0x0067_c650,
    0x0067_c6f0,
    0x0041_fc10,
    0x0058_efd0,
    0x0058_f4c0,
    0x0058_9450,
    0x0058_9770,
    0x0068_0890,
    0x0042_2480,
    0x0041_dc70,
    0x0042_2750,
    0x0043_3b70,
    0x0043_42b0,
    0x0042_e2c0,
    0x0042_e760,
    0x0042_e800,
    0x0043_6f80,
    0x0043_8f50,
    0x0043_9120,
    0x0042_1f30,
    0x0043_46f0,
    0x0043_4c80,
    0x004a_4d40,
    0x0041_fa60,
    0x0042_eb60,
    0x0053_7e90,
    0x0042_0fd0,
    0x0042_ef20,
    0x0041_fec0,
    0x0042_0a60,
    0x0042_0e00,
    0x0042_0c10,
    0x0042_0870,
    0x0042_f200,
    0x0043_4a70,
    0x0043_4ee0,
    0x0047_0470,
    0x0090_5820,
    0x0042_e380,
    XD_SLOT_3_FN,
];

/// The family's stand-ins: `operator new`, the constructors (each giving its type), the callees
/// of other units, the world vtable's slots.
fn xd_stand_ins(e: &mut Engine) {
    e.map(CONTROL, 0x1000);
    e.register(0x0040_1000, xd_new);
    for &addr in XD_NOTED {
        e.register(addr, noted);
    }
    e.register(XD_COMPARE_FN, xd_compare);
    e.register(0x0042_f420, xd_get_or_create);
    macro_rules! constructors {
        ($($addr:expr => $kind:expr),* $(,)?) => { $( e.register($addr, built::<$kind>); )* };
    }
    constructors! {
        0x0043_00c0 => 0x10, 0x0043_0780 => 0x0f, 0x0043_0c70 => 0x2a, 0x0043_1230 => 0x2b,
        0x0043_1360 => 0x2c, 0x0043_14c0 => 0x90, 0x0043_1620 => 0x91, 0x0043_1780 => 0x0e,
        0x0043_1830 => 0x15, 0x0043_1950 => 0x20, 0x0043_1a40 => 0x21, 0x0043_1ae0 => 0x22,
        0x0043_1b80 => 0x23, 0x0043_1c20 => 0x24, 0x0043_1d10 => 0x25, 0x0043_1e20 => 0x26,
        0x0043_1ec0 => 0x27, 0x0043_1f60 => 0x28, 0x0043_2000 => 0x0d, 0x0043_2220 => 0x30,
        0x0043_5150 => 0x3f, 0x0043_5ec0 => 0x46, 0x0043_61e0 => 0x7d, 0x0043_6300 => 0x8a,
        0x0043_6420 => 0x83, 0x0043_6540 => 0x86, 0x0043_5fa0 => 0x4c, 0x0043_38b0 => 0x53,
        0x0043_3ca0 => 0x57,
        0x0043_22c0 => 0x1f, 0x0043_22f0 => 0x16, 0x0043_2320 => 0x17, 0x0043_2350 => 0x3e,
        0x0043_25b0 => 0x31, 0x0043_26c0 => 0x18, 0x0043_08c0 => 0x49, 0x0043_60c0 => 0x4f,
        0x0043_6660 => 0x87, 0x0040_eda0 => 0x01, 0x0040_ec00 => 0x02, 0x0040_ef00 => 0x04,
        0x0040_ef30 => 0x07, 0x0040_efd0 => 0x81, 0x0040_f070 => 0x08, 0x0040_f110 => 0x59,
        0x0040_f1b0 => 0x8c, 0x0040_f440 => 0x0a, 0x0043_27e0 => 0x1c, 0x0043_2870 => 0x19,
        0x0043_28d0 => 0x1a, 0x0043_2aa0 => 0x35, 0x0043_2be0 => 0x2f, 0x0043_2c20 => 0x0c,
        0x0043_2cb0 => 0x14, 0x0043_3010 => 0x1b, 0x0043_3260 => 0x13, 0x0043_3300 => 0x37,
    }
    xe_stand_ins(e);
}

/// The extra list's world (`extra_world`) for the family: page 0 mapped (a failed allocation's
/// null extra data is read and added as the exe does), the read-only floats the getters and the
/// scale setter read, the world vtable's `Compare` and slot 3, the type 0x92 copy's slot 3,
/// `operator new` failing for some seeds, the thread's word at +0x294 in four forms, JIP's patch
/// word in three forms where it is on, and JIP's sites of the family with its head site.
fn xd_world(e: &mut Engine, seed: u32) -> ExtraWorld {
    use engine_core::extend::jip_patches::{FIXES, INSTALLED};
    use engine_core::extend::{Owner, SiteSlot};
    let w = extra_world(e, seed);
    // JIP on: installed with its fixes, installed alone, or its fixes alone, so a seam gated by
    // the other bit shows.
    if w.jip % 3 != 0 {
        e.set_jip_patches([u32::MAX, INSTALLED, FIXES][(seed / 4 % 3) as usize]);
    }
    e.map(0, 0x1000);
    e.map(0x0101_2000, 0x3000);
    e.mem.set_u32(0x0101_2054, (-1.0f32).to_bits());
    e.mem.set_f64(0x0101_2070, 1.0);
    e.mem.set_u32(0x0101_4458 + 0xc, XD_SLOT_3_FN);
    let vt = e.mem.u32(w.loose);
    e.mem.set_u32(vt + 4, XD_COMPARE_FN);
    e.mem.set_u32(vt + 0xc, XD_SLOT_3_FN);
    e.mem.set_u32(XD_VT, vt);
    // A null extra data (a failed allocation's, which the exe adds and calls through as it is)
    // reads the world's vtable from page 0, so its calls reach the stand-ins on both sides.
    e.mem.set_u32(0, vt);
    // The game loader's global `RemoveNonPersistentCellData` reads (its flag test ignores it).
    e.map(0x011d_d000, 0x1000);
    // The save/load singleton's global, which the reference pointer setter's translation reads.
    e.map(0x011d_e000, 0x1000);
    let mut d = Draw(seed.wrapping_mul(0x2545_f491) ^ 0x0041_8200);
    e.mem.set_u32(XD_NEW_FAILS, u32::from(d.next(6) == 0));
    let tls = e.tls();
    e.mem.set_u32(
        tls + 0x294,
        [0, 4, 0xffff_fffb, u32::MAX][d.next(4) as usize],
    );
    xd_stand_ins(e);
    if w.jip >= 2 {
        e.on_site(SiteSlot::new(Owner::Jip, 0x0041_1978), jip_copy_site);
        e.on_site(SiteSlot::new(Owner::Jip, 0x0041_2675), jip_hotkey_site);
        e.on_site(SiteSlot::new(Owner::Jip, 0x0041_271e), jip_owner_site);
        e.on_site(SiteSlot::new(Owner::Jip, 0x0041_c833), jip_persists_site);
    }
    w
}

/// Where JIP's handlers at the family's three sites note each asking: a count, then rows of the
/// site and the four words it was handed (zeros past them), so the Diff compares the words JIP's
/// real handler reads (the copy's list, extra data, caller and the caller's flag).
const XD_SITE_LOG: u32 = CONTROL + 0x320;
const XD_SITE_ROWS: u32 = 6;

fn xd_site_note(e: &mut Engine, site: u32, a: &[u32]) {
    let n = e.mem.u32(XD_SITE_LOG);
    e.mem.set_u32(XD_SITE_LOG, n.wrapping_add(1));
    if n < XD_SITE_ROWS {
        let row = XD_SITE_LOG + 4 + n * 20;
        e.mem.set_u32(row, site);
        for i in 0..4u32 {
            let word = a.get(i as usize).copied().unwrap_or(0);
            e.mem.set_u32(row + 4 + 4 * i, word);
        }
    }
}

/// The rows [`xd_site_note`] wrote: the site and its four words.
fn xd_site_rows(e: &Engine) -> Vec<[u32; 5]> {
    (0..e.mem.u32(XD_SITE_LOG).min(XD_SITE_ROWS))
        .map(|n| {
            let row = XD_SITE_LOG + 4 + n * 20;
            std::array::from_fn(|i| e.mem.u32(row + 4 * i as u32))
        })
        .collect()
}

/// JIP's copy of its keyed data (`0x411978`): noted, done.
fn jip_copy_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    xd_site_note(e, 0x0041_1978, a);
    Some(1)
}

/// JIP's hotkey copy (`0x412675`): noted, done.
fn jip_hotkey_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    xd_site_note(e, 0x0041_2675, a);
    Some(1)
}

/// JIP's owner test (`0x41271E`): noted, an owner is no difference.
fn jip_owner_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    xd_site_note(e, 0x0041_271e, a);
    Some(1)
}

/// JIP's persistence test of the reference pointer setter (`0x41C833`): noted, the reference's
/// bit 2.
fn jip_persists_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    xd_site_note(e, 0x0041_c833, a);
    Some(a[0] >> 2 & 1)
}

/// Where `place_kind` put the extra data: nowhere, at the head, at the tail, or only its bit set.
fn xd_place(seed: u32, kind: u32) -> u32 {
    Draw(seed.wrapping_mul(0x9e37_79b9) ^ kind.wrapping_mul(0x85eb_ca6b)).next(4)
}

/// An extra data of `kind` in the world's list for some seeds ([`xd_place`]), its payload words
/// from [`XD_WORDS`]. Answers it (0 when it is not in the chain).
fn place_kind(e: &mut Engine, w: &ExtraWorld, seed: u32, kind: u32) -> u32 {
    let mut d = Draw(seed.wrapping_mul(0x9e37_79b9) ^ kind.wrapping_mul(0x85eb_ca6b));
    let place = d.next(4);
    if place == 0 {
        return 0;
    }
    if kind >> 3 < 0x15 {
        let flags = e.mem.u8(w.list + 8 + (kind >> 3)) | (1 << (kind & 7));
        e.mem.set_u8(w.list + 8 + (kind >> 3), flags);
    }
    if place == 3 {
        return 0;
    }
    let x = e.mem.alloc(0x40);
    let vt = e.mem.u32(XD_VT);
    for (k, word) in [vt, kind, 0].into_iter().enumerate() {
        e.mem.set_u32(x + 4 * k as u32, word);
    }
    for offset in [0xc, 0x10, 0x14] {
        let word = XD_WORDS[d.next(6) as usize];
        e.mem.set_u32(x + offset, word);
    }
    let head = e.mem.u32(w.list + 4);
    if place == 1 || head == 0 {
        e.mem.set_u32(x + 8, head);
        e.mem.set_u32(w.list + 4, x);
    } else {
        let mut last = head;
        while e.mem.u32(last + 8) != 0 {
            last = e.mem.u32(last + 8);
        }
        e.mem.set_u32(last + 8, x);
    }
    x
}

/// A block of `n` bytes, each different (`salt` and its index), so a copy shows.
fn xd_block(e: &mut Engine, n: u32, salt: u32) -> u32 {
    let x = e.mem.alloc(n);
    for i in 0..n {
        e.mem.set_u8(x + i, (i.wrapping_mul(7) ^ salt) as u8);
    }
    x
}

/// A sound handle: 0 one with a sound, 1 an empty one with its other fields set, 2 one whose id is
/// 0, 3 none (a null pointer).
fn xd_handle(e: &mut Engine, which: u32) -> u32 {
    if which == 3 {
        return 0;
    }
    let h = xd_block(e, 0x10, which);
    e.mem.set_u32(h, [7, u32::MAX, 0][which as usize]);
    e.mem.set_u32(h + 8, which + 1);
    h
}

/// An extra data a copy reads (0x40 bytes): its payload a node (item, next) of a list whose items
/// are records (a word and a byte; the node's +8 a second node and +0x10 a byte, as a room's data
/// has them), a handle at +0x10, words at +0x14 to +0x1c, a node of its own at +0x20.
fn xd_source(e: &mut Engine, d: &mut Draw, kind: u32) -> u32 {
    let vt = e.mem.u32(XD_VT);
    let record = xd_block(e, 0x10, kind);
    e.mem.set_u32(record, XD_WORDS[d.next(6) as usize]);
    let tail = xd_block(e, 0x40, kind ^ 1);
    e.mem.set_u32(tail, [record, 0][d.next(2) as usize]);
    e.mem.set_u32(tail + 4, 0);
    let node = xd_block(e, 0x40, kind ^ 2);
    e.mem.set_u32(node, [record, 0, record][d.next(3) as usize]);
    e.mem.set_u32(node + 4, [tail, 0][d.next(2) as usize]);
    e.mem.set_u32(node + 8, [record, 0][d.next(2) as usize]);
    e.mem.set_u32(node + 0xc, 0);
    let x = xd_block(e, 0x40, kind ^ 3);
    let payload = if d.next(6) == 0 { 0 } else { node };
    for (k, word) in [vt, kind, 0, payload, record].into_iter().enumerate() {
        e.mem.set_u32(x + 4 * k as u32, word);
    }
    for offset in [0x14, 0x18, 0x1c] {
        let word = XD_WORDS[d.next(6) as usize];
        e.mem.set_u32(x + offset, word);
    }
    e.mem.set_u32(x + 0x20, [record, 0][d.next(2) as usize]);
    e.mem.set_u32(x + 0x24, 0);
    x
}

/// The types a copy's source list is drawn from: every list copy's copied types, a script, a
/// count, types a copy leaves out (0x01, 0x02, 0x04, 0x15, 0x58), JIP's keyed data, a hotkey and a
/// few with arms of their own.
const XD_SOURCE_KINDS: &[u32] = &[
    0x01, 0x02, 0x04, 0x0d, 0x15, 0x16, 0x17, 0x1c, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
    0x2b, 0x2f, 0x30, 0x3f, 0x47, 0x4a, 0x58, 0x65, 0x8d, 0x92,
];

/// A list of 0 to 6 extra data drawn from `kinds` ([`xd_source`] each), its bitmap as its chain.
fn xd_list(e: &mut Engine, seed: u32, kinds: &[u32]) -> u32 {
    let mut d = Draw(seed.wrapping_mul(0x5851_f42d) ^ 0x0041_1ec0);
    let n = [0, 1, 2, 3, 4, 6][d.next(6) as usize];
    let list = e.mem.alloc(0x20);
    e.mem.set_u32(list, 0x0101_43e8);
    let mut last = 0;
    for _ in 0..n {
        let kind = kinds[d.next(kinds.len() as u32) as usize];
        let x = xd_source(e, &mut d, kind);
        if last == 0 {
            e.mem.set_u32(list + 4, x);
        } else {
            e.mem.set_u32(last + 8, x);
        }
        last = x;
        if kind >> 3 < 0x15 {
            let flags = e.mem.u8(list + 8 + (kind >> 3)) | (1 << (kind & 7));
            e.mem.set_u8(list + 8 + (kind >> 3), flags);
        }
    }
    list
}

/// A list copy's source as the copy Diffs make it: [`xd_list`]'s, with a time-left extra data
/// (type 0x27, JIP's keyed copy) at its head in the seeds with JIP's handler at its sites, so its
/// copy site is asked from both list copies, with both flags.
fn xd_copy_source(e: &mut Engine, seed: u32) -> u32 {
    let list = xd_list(e, seed, XD_SOURCE_KINDS);
    if seed % 4 == 2 {
        let mut d = Draw(seed ^ 0x0041_1978);
        let x = xd_source(e, &mut d, 0x27);
        e.mem.set_u32(x + 8, e.mem.u32(list + 4));
        e.mem.set_u32(list + 4, x);
        let flags = e.mem.u8(list + 8 + 4) | 0x80;
        e.mem.set_u8(list + 8 + 4, flags);
    }
    list
}

/// Another list to compare the world's with: empty, the same types with the same payloads (each
/// `Compare`s equal), the same less its last, or one of the copies' types; but for the same list,
/// an owner (type 0x21) at its head in the seeds [`xd_owner_first`] picks, so the container compare
/// walks one (and JIP's owner site) under each JIP state.
fn xd_other(e: &mut Engine, w: &ExtraWorld, seed: u32) -> u32 {
    let list = xd_other_list(e, w, seed);
    if xd_owner_first(seed) {
        let vt = e.mem.u32(XD_VT);
        let owner = extra_data(e, vt, 0x21, 0x21);
        e.mem.set_u32(owner + 8, e.mem.u32(list + 4));
        e.mem.set_u32(list + 4, owner);
        let flags = e.mem.u8(list + 8 + 4) | 2;
        e.mem.set_u8(list + 8 + 4, flags);
    }
    list
}

/// Whether [`xd_other`] puts an owner first: half the seeds (each JIP state among them), not with
/// the same list.
fn xd_owner_first(seed: u32) -> bool {
    (seed / 4) % 2 == 0 && Draw(seed ^ 0x0041_27e0).next(4) != 1
}

/// [`xd_other`]'s list before the owner.
fn xd_other_list(e: &mut Engine, w: &ExtraWorld, seed: u32) -> u32 {
    let form = Draw(seed ^ 0x0041_27e0).next(4);
    if form == 3 {
        return xd_list(e, seed, XD_SOURCE_KINDS);
    }
    let list = e.mem.alloc(0x20);
    e.mem.set_u32(list, 0x0101_43e8);
    let keep = match form {
        0 => 0,
        1 => w.chain.len(),
        _ => w.chain.len().saturating_sub(1),
    };
    let vt = e.mem.u32(XD_VT);
    let mut last = 0;
    for &x in &w.chain[..keep] {
        let kind = e.mem.u32(x + 4);
        let copy = extra_data(e, vt, kind, 0);
        e.mem.set_u32(copy + 0xc, e.mem.u32(x + 0xc));
        if last == 0 {
            e.mem.set_u32(list + 4, copy);
        } else {
            e.mem.set_u32(last + 8, copy);
        }
        last = copy;
        if kind >> 3 < 0x15 {
            let flags = e.mem.u8(list + 8 + (kind >> 3)) | (1 << (kind & 7));
            e.mem.set_u8(list + 8 + (kind >> 3), flags);
        }
    }
    list
}

/// The getters of one type: the world's list, with an extra data of `kind` placed or not.
trait XdGetter {
    fn xd_getter(self, kind: u32) -> Diff;
    fn xd_sound_getter(self, kind: u32) -> Diff;
    fn xd_setter(self, kind: u32, values: &'static [u32]) -> Diff;
    fn xd_sound_setter(self, kind: u32) -> Diff;
}

impl XdGetter for Diff {
    fn xd_getter(self, kind: u32) -> Diff {
        self.extra_data_case()
            .inputs(&[Input::Of(EXTRA_SEEDS)])
            .setup(move |e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), kind);
                vec![w.list]
            })
    }

    fn xd_sound_getter(self, kind: u32) -> Diff {
        self.extra_data_case()
            .inputs(&[Input::Of(EXTRA_SEEDS)])
            .setup(move |e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), kind);
                vec![w.list, junk(e, 0x10)]
            })
    }

    fn xd_setter(self, kind: u32, values: &'static [u32]) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * values.len())
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(values)])
            .setup(move |e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), kind);
                vec![w.list, case.word(1)]
            })
    }

    fn xd_sound_setter(self, kind: u32) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * 4)
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(&[0, 1, 2, 3])])
            .setup(move |e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), kind);
                vec![w.list, xd_handle(e, case.word(1))]
            })
    }
}

/// The family's other test shapes: a block of different bytes, two of them, a scalar deleting
/// destructor with each flag, and a pair of sound handles in the world.
trait XdShapes {
    fn xd_on_pattern(self, n: u32, own: u32) -> Diff;
    fn xd_copies(self, own: u32) -> Diff;
    fn xd_data_delete(self) -> Diff;
    fn xd_sound_compare(self, own: u32) -> Diff;
}

impl XdShapes for Diff {
    fn xd_on_pattern(self, n: u32, own: u32) -> Diff {
        self.extra_data_case_but(own).setup(move |e, _| {
            xd_stand_ins(e);
            vec![xd_block(e, n, 0x5a)]
        })
    }

    fn xd_copies(self, own: u32) -> Diff {
        self.extra_data_case_but(own)
            .setup(|e, _| vec![xd_block(e, 0x110, 0x11), xd_block(e, 0x110, 0x22)])
    }

    fn xd_data_delete(self) -> Diff {
        self.extra_data_case()
            .inputs(&[Input::Of(&[0]), Input::Of(&[0, 1, 2, 3])])
            .setup(|e, case| {
                xd_stand_ins(e);
                e.register(0x0040_1030, nothing);
                vec![junk(e, 0x10), case.word(1)]
            })
    }

    fn xd_sound_compare(self, own: u32) -> Diff {
        self.extra_data_case_but(own)
            .max_cases(4 * 16)
            .inputs(&[
                Input::Of(&[0, 1, 2, 3]),
                Input::Of(&[0, 1, 2, 3]),
                Input::Of(&[0, 1, 2, 3]),
            ])
            .setup(|e, case| {
                xd_world(e, case.word(0));
                vec![xd_handle(e, case.word(1)), xd_handle(e, case.word(2))]
            })
    }
}

/// Words a setter is given: none, one, all ones, a pointer-like word, the sign bit.
const XD_VALUES: &[u32] = &[0, 1, u32::MAX, 0x1234_5678, 0x8000_0000];
/// Floats a setter is given: 1.0, 0.0, a signalling NaN, -1.0, the quiet NaN, the float after 1.0.
const XD_FLOATS: &[u32] = &[
    0x3f80_0000,
    0,
    0x7fa0_0001,
    0xbf80_0000,
    0x7fc0_0000,
    0x3f80_0001,
];

#[test]
fn xd_anim_save() {
    Diff::new(entry!(0x0041_8200, extra_data::access::anim_save(u32) -> u32))
        .xd_getter(0x2d)
        .run()
        .assert_same();
}

#[test]
fn xd_animation() {
    Diff::new(entry!(0x0041_8220, extra_data::access::animation(u32) -> u32))
        .xd_getter(0x10)
        .run()
        .assert_same();
}

#[test]
fn xd_light() {
    Diff::new(entry!(0x0041_8250, extra_data::access::light(u32) -> u32))
        .xd_getter(0x29)
        .run()
        .assert_same();
}

#[test]
fn xd_spell_effect_light() {
    Diff::new(entry!(0x0041_8280, extra_data::access::spell_effect_light(u32) -> u32))
        .xd_getter(0x40)
        .run()
        .assert_same();
}

#[test]
fn xd_lock_data() {
    Diff::new(entry!(0x0041_82b0, extra_data::access::lock_data(u32) -> u32))
        .xd_getter(0x2a)
        .run()
        .assert_same();
}

#[test]
fn xd_radio_data() {
    Diff::new(entry!(0x0041_82e0, extra_data::access::radio_data(u32) -> u32))
        .xd_getter(0x68)
        .run()
        .assert_same();
}

#[test]
fn xd_teleport() {
    Diff::new(entry!(0x0041_8460, extra_data::access::teleport(u32) -> u32))
        .xd_getter(0x2b)
        .run()
        .assert_same();
}

#[test]
fn xd_map_marker() {
    Diff::new(entry!(0x0041_8490, extra_data::access::map_marker(u32) -> u32))
        .xd_getter(0x2c)
        .run()
        .assert_same();
}

#[test]
fn xd_audio_marker() {
    Diff::new(entry!(0x0041_84c0, extra_data::access::audio_marker(u32) -> u32))
        .xd_getter(0x90)
        .run()
        .assert_same();
}

#[test]
fn xd_audio_buoy_marker() {
    Diff::new(entry!(0x0041_84f0, extra_data::access::audio_buoy_marker(u32) -> u32))
        .xd_getter(0x91)
        .run()
        .assert_same();
}

#[test]
fn xd_container_changes() {
    Diff::new(entry!(0x0041_8520, extra_data::access::container_changes(u32) -> u32))
        .xd_getter(0x15)
        .run()
        .assert_same();
}

#[test]
fn xd_original_reference() {
    Diff::new(entry!(0x0041_8630, extra_data::access::original_reference(u32) -> u32))
        .xd_getter(0x20)
        .run()
        .assert_same();
}

#[test]
fn xd_ownership() {
    Diff::new(entry!(0x0041_8660, extra_data::access::ownership(u32) -> u32))
        .xd_getter(0x21)
        .run()
        .assert_same();
}

#[test]
fn xd_global() {
    Diff::new(entry!(0x0041_8690, extra_data::access::global(u32) -> u32))
        .xd_getter(0x22)
        .run()
        .assert_same();
}

#[test]
fn xd_rank() {
    Diff::new(entry!(0x0041_86c0, extra_data::access::rank(u32) -> i32))
        .xd_getter(0x23)
        .run()
        .assert_same();
}

#[test]
fn xd_health() {
    Diff::new(entry!(0x0041_86f0, extra_data::access::health(u32) -> f32))
        .xd_getter(0x25)
        .run()
        .assert_same();
}

#[test]
fn xd_leveled_item() {
    Diff::new(entry!(0x0041_8720, extra_data::access::leveled_item(u32) -> u32))
        .xd_getter(0x2f)
        .run()
        .assert_same();
}

#[test]
fn xd_has_leveled_item() {
    Diff::new(entry!(0x0041_8750, extra_data::access::has_leveled_item(u32) -> bool))
        .xd_getter(0x2f)
        .run()
        .assert_same();
}

#[test]
fn xd_count() {
    Diff::new(entry!(0x0041_8770, extra_data::access::count(u32) -> u16))
        .xd_getter(0x24)
        .run()
        .assert_same();
}

#[test]
fn xd_charge() {
    Diff::new(entry!(0x0041_87a0, extra_data::access::charge(u32) -> f32))
        .xd_getter(0x28)
        .run()
        .assert_same();
}

#[test]
fn xd_poison() {
    Diff::new(entry!(0x0041_87d0, extra_data::access::poison(u32) -> u32))
        .xd_getter(0x3f)
        .run()
        .assert_same();
}

#[test]
fn xd_script() {
    Diff::new(entry!(0x0041_8800, extra_data::access::script(u32) -> u32))
        .xd_getter(0x0d)
        .run()
        .assert_same();
}

#[test]
fn xd_script_locals() {
    Diff::new(entry!(0x0041_8830, extra_data::access::script_locals(u32) -> u32))
        .xd_getter(0x0d)
        .run()
        .assert_same();
}

#[test]
fn xd_scale() {
    Diff::new(entry!(0x0041_8860, extra_data::access::scale(u32) -> f32))
        .xd_getter(0x30)
        .run()
        .assert_same();
}

#[test]
fn xd_is_ghost() {
    Diff::new(entry!(0x0041_8a80, extra_data::access::is_ghost(u32) -> bool))
        .xd_getter(0x1f)
        .run()
        .assert_same();
}

#[test]
fn xd_seed() {
    Diff::new(entry!(0x0041_8b40, extra_data::access::seed(u32) -> u8))
        .xd_getter(0x31)
        .run()
        .assert_same();
}

#[test]
fn xd_package_start_location() {
    Diff::new(entry!(0x0041_8b70, extra_data::access::package_start_location(u32) -> u32))
        .xd_getter(0x18)
        .run()
        .assert_same();
}

#[test]
fn xd_has_weapon_mods() {
    Diff::new(entry!(0x0041_8ba0, extra_data::access::has_weapon_mods(u32) -> bool))
        .xd_getter(0x8d)
        .run()
        .assert_same();
}

#[test]
fn xd_action() {
    Diff::new(entry!(0x0041_95b0, extra_data::access::action(u32) -> u32))
        .xd_getter(0x0e)
        .run()
        .assert_same();
}

#[test]
fn xd_remove_original_reference() {
    Diff::new(entry!(
        0x0041_8600,
        extra_data::access::remove_original_reference(u32)
    ))
    .xd_getter(0x20)
    .run()
    .assert_same();
}

#[test]
fn xd_sound() {
    Diff::new(entry!(0x0041_8890, extra_data::access::sound(u32, u32) -> u32))
        .xd_sound_getter(0x4f)
        .run()
        .assert_same();
}

#[test]
fn xd_creature_awake_sound() {
    Diff::new(entry!(0x0041_8940, extra_data::access::creature_awake_sound(u32, u32) -> u32))
        .xd_sound_getter(0x7d)
        .run()
        .assert_same();
}

#[test]
fn xd_creature_movement_sound() {
    Diff::new(entry!(0x0041_8980, extra_data::access::creature_movement_sound(u32, u32) -> u32))
        .xd_sound_getter(0x8a)
        .run()
        .assert_same();
}

#[test]
fn xd_weapon_idle_sound() {
    Diff::new(entry!(0x0041_89c0, extra_data::access::weapon_idle_sound(u32, u32) -> u32))
        .xd_sound_getter(0x83)
        .run()
        .assert_same();
}

#[test]
fn xd_weapon_attack_sound() {
    Diff::new(entry!(0x0041_8a00, extra_data::access::weapon_attack_sound(u32, u32) -> u32))
        .xd_sound_getter(0x86)
        .run()
        .assert_same();
}

#[test]
fn xd_activate_loop_sound() {
    Diff::new(entry!(0x0041_8a40, extra_data::access::activate_loop_sound(u32, u32) -> u32))
        .xd_sound_getter(0x87)
        .run()
        .assert_same();
}

#[test]
fn xd_set_original_reference() {
    Diff::new(entry!(
        0x0041_8550,
        extra_data::access::set_original_reference(u32, u32)
    ))
    .xd_setter(0x20, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xd_set_anim_ptr() {
    Diff::new(entry!(0x0041_8c40, extra_data::access::set_anim_ptr(u32, u32) -> u32))
        .xd_setter(0x10, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xd_starting_position() {
    Diff::new(entry!(0x0041_8d50, extra_data::access::starting_position(u32, u32) -> u32))
        .xd_setter(0x0f, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xd_set_lock_ptr() {
    Diff::new(entry!(0x0041_9050, extra_data::access::set_lock_ptr(u32, u32) -> u32))
        .xd_setter(0x2a, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xd_set_teleport() {
    Diff::new(entry!(0x0041_9120, extra_data::access::set_teleport(u32, u32) -> u32))
        .xd_setter(0x2b, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xd_set_map_marker() {
    Diff::new(entry!(0x0041_9250, extra_data::access::set_map_marker(u32, u32) -> u32))
        .xd_setter(0x2c, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xd_set_audio_marker() {
    Diff::new(entry!(0x0041_9380, extra_data::access::set_audio_marker(u32, u32) -> u32))
        .xd_setter(0x90, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xd_set_audio_buoy_marker() {
    Diff::new(entry!(0x0041_94b0, extra_data::access::set_audio_buoy_marker(u32, u32) -> u32))
        .xd_setter(0x91, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xd_set_container_changes() {
    Diff::new(entry!(
        0x0041_9650,
        extra_data::access::set_container_changes(u32, u32)
    ))
    .xd_setter(0x15, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xd_set_ownership() {
    Diff::new(entry!(
        0x0041_9700,
        extra_data::access::set_ownership(u32, u32)
    ))
    .xd_setter(0x21, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xd_set_global() {
    Diff::new(entry!(
        0x0041_97d0,
        extra_data::access::set_global(u32, u32)
    ))
    .xd_setter(0x22, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xd_set_rank() {
    Diff::new(entry!(0x0041_98a0, extra_data::access::set_rank(u32, i32)))
        .xd_setter(0x23, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xd_set_health() {
    Diff::new(entry!(
        0x0041_9970,
        extra_data::access::set_health(u32, f32)
    ))
    .xd_setter(0x25, XD_FLOATS)
    .run()
    .assert_same();
}

#[test]
fn xd_set_uses() {
    Diff::new(entry!(0x0041_9a20, extra_data::access::set_uses(u32, u8)))
        .xd_setter(0x26, &[0, 1, 0xff, 0x100, 0x1ff])
        .run()
        .assert_same();
}

#[test]
fn xd_set_count() {
    Diff::new(entry!(0x0041_9ad0, extra_data::access::set_count(u32, u16)))
        .xd_setter(0x24, &[0, 1, 2, 0x7fff, 0x8000, 0xffff, 0x1_0001])
        .run()
        .assert_same();
}

#[test]
fn xd_set_time_left() {
    Diff::new(entry!(
        0x0041_9bb0,
        extra_data::access::set_time_left(u32, f32)
    ))
    .xd_setter(0x27, XD_FLOATS)
    .run()
    .assert_same();
}

#[test]
fn xd_set_charge() {
    Diff::new(entry!(
        0x0041_9c60,
        extra_data::access::set_charge(u32, f32)
    ))
    .xd_setter(0x28, XD_FLOATS)
    .run()
    .assert_same();
}

#[test]
fn xd_set_poison() {
    Diff::new(entry!(
        0x0041_9d10,
        extra_data::access::set_poison(u32, u32)
    ))
    .xd_setter(0x3f, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xd_set_item_dropper() {
    Diff::new(entry!(
        0x0041_9dc0,
        extra_data::access::set_item_dropper(u32, u32)
    ))
    .xd_setter(0x46, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xd_set_script() {
    Diff::new(entry!(
        0x0041_9ed0,
        extra_data::access::set_script(u32, u32)
    ))
    .xd_setter(0x0d, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xd_set_script_locals() {
    Diff::new(entry!(
        0x0041_9f80,
        extra_data::access::set_script_locals(u32, u32)
    ))
    .xd_setter(0x0d, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xd_set_scale() {
    Diff::new(entry!(0x0041_9fb0, extra_data::access::set_scale(u32, f32)))
        .xd_setter(0x30, XD_FLOATS)
        .run()
        .assert_same();
}

#[test]
fn xd_worn() {
    Diff::new(entry!(0x0041_8ab0, extra_data::access::worn(u32, bool) -> bool))
        .xd_setter(0x16, &[0, 1, 0x100])
        .run()
        .assert_same();
}

#[test]
fn xd_weapon_mod_slot_active() {
    Diff::new(entry!(0x0041_8c00, extra_data::access::weapon_mod_slot_active(u32, u8) -> bool))
        .xd_setter(0x8d, &[0, 1, 2, 4, 7, 0x100])
        .run()
        .assert_same();
}

/// `GetWorn` with the left-hand type placed instead.
#[test]
fn xd_worn_left() {
    Diff::new(entry!(0x0041_8ab0, extra_data::access::worn(u32, bool) -> bool))
        .xd_setter(0x17, &[0, 1])
        .run()
        .assert_same();
}

#[test]
fn xd_set_creature_awake_sound() {
    Diff::new(entry!(
        0x0041_a090,
        extra_data::access::set_creature_awake_sound(u32, u32)
    ))
    .xd_sound_setter(0x7d)
    .run()
    .assert_same();
}

#[test]
fn xd_set_creature_movement_sound() {
    Diff::new(entry!(
        0x0041_a280,
        extra_data::access::set_creature_movement_sound(u32, u32)
    ))
    .xd_sound_setter(0x8a)
    .run()
    .assert_same();
}

#[test]
fn xd_set_weapon_idle_sound() {
    Diff::new(entry!(
        0x0041_a3e0,
        extra_data::access::set_weapon_idle_sound(u32, u32)
    ))
    .xd_sound_setter(0x83)
    .run()
    .assert_same();
}

#[test]
fn xd_set_weapon_attack_sound() {
    Diff::new(entry!(
        0x0041_a540,
        extra_data::access::set_weapon_attack_sound(u32, u32)
    ))
    .xd_sound_setter(0x86)
    .run()
    .assert_same();
}

/// `set_radio_data`: no record (the type removed), or a record copied into the one there or a new
/// one.
#[test]
fn xd_set_radio_data() {
    Diff::new(entry!(
        0x0041_8310,
        extra_data::access::set_radio_data(u32, u32)
    ))
    .extra_data_case()
    .max_cases(EXTRA_SEEDS.len() * 2)
    .inputs(&[Input::Of(EXTRA_SEEDS), Input::Flag])
    .setup(|e, case| {
        let w = xd_world(e, case.word(0));
        place_kind(e, &w, case.word(0), 0x68);
        let data = if case.word(1) == 0 {
            0
        } else {
            xd_block(e, 0x10, 0x68)
        };
        vec![w.list, data]
    })
    .run()
    .assert_same();
}

/// The constructors and the small bodies of one block of junk.
trait XdOnBlock {
    fn xd_on_block(self, n: u32, own: u32) -> Diff;
}

impl XdOnBlock for Diff {
    fn xd_on_block(self, n: u32, own: u32) -> Diff {
        self.extra_data_case_but(own).setup(move |e, _| {
            xd_stand_ins(e);
            vec![junk(e, n)]
        })
    }
}

#[test]
fn xd_radio_extra_new() {
    Diff::new(entry!(0x0041_83e0, extra_data::access::radio_extra_new(u32) -> u32))
        .xd_on_block(0x20, 0x0041_83e0)
        .run()
        .assert_same();
}

#[test]
fn xd_sound_handle_new() {
    Diff::new(entry!(0x0041_88d0, extra_data::access::sound_handle_new(u32) -> u32))
        .xd_on_block(0x10, 0x0041_88d0)
        .run()
        .assert_same();
}

#[test]
fn xd_sound_handle_init() {
    Diff::new(entry!(0x0041_a250, extra_data::access::sound_handle_init(u32) -> u32))
        .xd_on_block(0x10, 0x0041_a250)
        .run()
        .assert_same();
}

#[test]
fn xd_empty_node_new() {
    Diff::new(entry!(0x0041_4010, extra_data::records::empty_node_new(u32) -> u32))
        .xd_on_block(0x10, 0x0041_4010)
        .run()
        .assert_same();
}

#[test]
fn xd_portal_ref_new() {
    Diff::new(entry!(0x0041_43c0, extra_data::records::portal_ref_new(u32) -> u32))
        .xd_on_block(0x10, 0x0041_43c0)
        .run()
        .assert_same();
}

#[test]
fn xd_primitive_new() {
    Diff::new(entry!(0x0041_43f0, extra_data::records::primitive_new(u32) -> u32))
        .xd_on_block(0x20, 0x0041_43f0)
        .run()
        .assert_same();
}

#[test]
fn xd_plane_new() {
    Diff::new(entry!(0x0041_4470, extra_data::records::plane_new(u32) -> u32))
        .xd_on_block(0x30, 0x0041_4470)
        .run()
        .assert_same();
}

#[test]
fn xd_portal_record_new() {
    Diff::new(entry!(0x0041_6ab0, extra_data::records::portal_record_new(u32) -> u32))
        .xd_on_block(0x10, 0x0041_6ab0)
        .run()
        .assert_same();
}

#[test]
fn xd_radio_data_new() {
    Diff::new(entry!(0x0041_6b40, extra_data::records::radio_data_new(u32) -> u32))
        .xd_on_block(0x10, 0x0041_6b40)
        .run()
        .assert_same();
}

#[test]
fn xd_room_links_new() {
    Diff::new(entry!(0x0041_6b80, extra_data::records::room_links_new(u32) -> u32))
        .xd_on_block(0x14, 0x0041_6b80)
        .run()
        .assert_same();
}

#[test]
fn xd_swap_decal() {
    Diff::new(entry!(0x0041_4030, extra_data::records::swap_decal(u32)))
        .xd_on_pattern(0x1c, 0x0041_4030)
        .run()
        .assert_same();
}

#[test]
fn xd_swap_plane() {
    Diff::new(entry!(0x0041_40f0, extra_data::records::swap_plane(u32)))
        .xd_on_pattern(0x24, 0x0041_40f0)
        .run()
        .assert_same();
}

#[test]
fn xd_swap_word_short() {
    Diff::new(entry!(
        0x0041_41e0,
        extra_data::records::swap_word_short(u32)
    ))
    .xd_on_pattern(0x8, 0x0041_41e0)
    .run()
    .assert_same();
}

#[test]
fn xd_swap_four_words() {
    Diff::new(entry!(
        0x0041_4220,
        extra_data::records::swap_four_words(u32)
    ))
    .xd_on_pattern(0x10, 0x0041_4220)
    .run()
    .assert_same();
}

#[test]
fn xd_swap_primitive() {
    Diff::new(entry!(
        0x0041_4290,
        extra_data::records::swap_primitive(u32)
    ))
    .xd_on_pattern(0x20, 0x0041_4290)
    .run()
    .assert_same();
}

#[test]
fn xd_swap_plane_refs() {
    Diff::new(entry!(
        0x0041_4370,
        extra_data::records::swap_plane_refs(u32)
    ))
    .xd_on_pattern(0x10, 0x0041_4370)
    .run()
    .assert_same();
}

#[test]
fn xd_word_1d8() {
    Diff::new(entry!(0x0041_69d0, extra_data::records::word_1d8(u32) -> u32))
        .xd_on_pattern(0x1dc, 0x0041_69d0)
        .run()
        .assert_same();
}

#[test]
fn xd_save_form() {
    Diff::new(entry!(0x0041_81e0, extra_data::access::save_form(u32) -> u32))
        .xd_on_pattern(0x24, 0x0041_81e0)
        .run()
        .assert_same();
}

#[test]
fn xd_weapon_mods_any() {
    Diff::new(entry!(0x0041_8be0, extra_data::access::weapon_mods_any(u32) -> bool))
        .xd_on_pattern(0x10, 0x0041_8be0)
        .run()
        .assert_same();
}

/// The address of the plane's centre: no memory.
#[test]
fn xd_plane_center() {
    Diff::new(entry!(0x0041_3f40, extra_data::records::plane_center(u32) -> u32))
        .family(&EXTRA_DATA)
        .inputs(&[Input::Word])
        .run()
        .assert_same();
}

#[test]
fn xd_plane_rotation() {
    Diff::new(entry!(0x0041_3f60, extra_data::records::plane_rotation(u32, u32) -> u32))
        .xd_copies(0x0041_3f60)
        .run()
        .assert_same();
}

#[test]
fn xd_primitive_color() {
    Diff::new(entry!(0x0041_3f90, extra_data::records::primitive_color(u32, u32) -> u32))
        .xd_copies(0x0041_3f90)
        .run()
        .assert_same();
}

#[test]
fn xd_primitive_radii() {
    Diff::new(entry!(0x0041_3fc0, extra_data::records::primitive_radii(u32, u32) -> u32))
        .xd_copies(0x0041_3fc0)
        .run()
        .assert_same();
}

#[test]
fn xd_plane_set_center() {
    Diff::new(entry!(
        0x0041_69f0,
        extra_data::records::plane_set_center(u32, u32)
    ))
    .xd_copies(0x0041_69f0)
    .run()
    .assert_same();
}

#[test]
fn xd_plane_set_size() {
    Diff::new(entry!(
        0x0041_6a30,
        extra_data::records::plane_set_size(u32, u32)
    ))
    .xd_copies(0x0041_6a30)
    .run()
    .assert_same();
}

#[test]
fn xd_plane_set_rotation() {
    Diff::new(entry!(
        0x0041_6a70,
        extra_data::records::plane_set_rotation(u32, u32)
    ))
    .xd_copies(0x0041_6a70)
    .run()
    .assert_same();
}

#[test]
fn xd_sound_handle_copy() {
    Diff::new(entry!(0x0041_8900, extra_data::access::sound_handle_copy(u32, u32) -> u32))
        .xd_copies(0x0041_8900)
        .run()
        .assert_same();
}

/// A `float` copied through the x87: ordinary, signalling and quiet NaNs, a denormal.
#[test]
fn xd_float_copy() {
    Diff::new(entry!(0x0041_3ff0, extra_data::records::float_copy(u32, u32) -> u32))
        .family(&EXTRA_DATA)
        .inputs(&[Input::Of(&[
            0x3f80_0000,
            0x7fa0_0001,
            0xffa0_0000,
            0x7fc0_0000,
            1,
        ])])
        .setup(|e, case| {
            let source = e.mem.alloc(4);
            e.mem.set_u32(source, case.word(0));
            vec![junk(e, 4), source]
        })
        .run()
        .assert_same();
}

/// `NiColorA`'s constructor over the float table.
#[test]
fn xd_color_new() {
    Diff::new(entry!(
        0x0041_4430,
        extra_data::records::color_new(u32, f32, f32, f32, f32) -> u32
    ))
    .family(&EXTRA_DATA)
    .inputs(&[
        Input::Of(&[0]),
        Input::F32,
        Input::F32,
        Input::F32,
        Input::F32,
    ])
    .setup(|e, case| {
        let mut words = case.words();
        words[0] = junk(e, 0x10);
        words
    })
    .run()
    .assert_same();
}

/// The rotation matrix over the float table, and swept: every float at once with all its bits.
#[test]
fn xd_rotation_matrix() {
    Diff::new(entry!(
        0x0041_68a0,
        extra_data::records::rotation_matrix(u32, f32, f32, f32, f32)
    ))
    .extra_data_case()
    .inputs(&[
        Input::Of(&[0]),
        Input::F32,
        Input::F32,
        Input::F32,
        Input::F32,
    ])
    .float_sweep(256)
    .setup(|e, case| {
        let mut words = case.words();
        words[0] = junk(e, 0x24);
        words
    })
    .run()
    .assert_same();
}

/// `sincos` over the float table (past 2^63 too), the two outputs apart or the same word.
#[test]
fn xd_sincos() {
    Diff::new(entry!(
        0x0041_69a0,
        extra_data::records::sincos(f32, u32, u32)
    ))
    .family(&EXTRA_DATA)
    .inputs(&[
        Input::F32,
        Input::Of(&[0, 1]),
        Input::Of(&[0x5f00_0000, 0xdf80_0000, 0x4049_0fdb]),
    ])
    .setup(|e, case| {
        let angle = if case.word(1) == 0 {
            case.f32(0).to_bits()
        } else {
            case.word(2)
        };
        let sine = junk(e, 4);
        let cosine = if case.word(1) == 0 { junk(e, 4) } else { sine };
        vec![angle, sine, cosine]
    })
    .run()
    .assert_same();
}

/// The navmesh portal extra data's constructor: the record's two words.
#[test]
fn xd_portal_extra_new() {
    Diff::new(entry!(
        0x0041_6ad0,
        extra_data::records::portal_extra_new(u32, u32, u32) -> u32
    ))
    .extra_data_case()
    .inputs(&[Input::Of(&[0]), Input::Word, Input::Word])
    .setup(|e, case| vec![junk(e, 0x18), case.word(1), case.word(2)])
    .run()
    .assert_same();
}

/// A plane's linked plane `index` set.
#[test]
fn xd_plane_set_linked() {
    Diff::new(entry!(
        0x0041_81c0,
        extra_data::records::plane_set_linked(u32, u32, u32)
    ))
    .family(&EXTRA_DATA)
    .inputs(&[Input::Of(&[0]), Input::Of(&[0, 1, 3, 7]), Input::Word])
    .setup(|e, case| vec![xd_block(e, 0x110, 0x33), case.word(1), case.word(2)])
    .run()
    .assert_same();
}

#[test]
fn xd_teleport_data_delete() {
    Diff::new(entry!(0x0041_9220, extra_data::access::teleport_data_delete(u32, u32) -> u32))
        .xd_data_delete()
        .run()
        .assert_same();
}

#[test]
fn xd_map_marker_data_delete() {
    Diff::new(entry!(0x0041_9350, extra_data::access::map_marker_data_delete(u32, u32) -> u32))
        .xd_data_delete()
        .run()
        .assert_same();
}

#[test]
fn xd_audio_marker_data_delete() {
    Diff::new(entry!(0x0041_9480, extra_data::access::audio_marker_data_delete(u32, u32) -> u32))
        .xd_data_delete()
        .run()
        .assert_same();
}

#[test]
fn xd_same_sound_id() {
    Diff::new(entry!(0x0041_a1f0, extra_data::access::same_sound_id(u32, u32) -> bool))
        .xd_sound_compare(0x0041_a1f0)
        .run()
        .assert_same();
}

#[test]
fn xd_other_sound_id() {
    Diff::new(entry!(0x0041_a220, extra_data::access::other_sound_id(u32, u32) -> bool))
        .xd_sound_compare(0x0041_a220)
        .run()
        .assert_same();
}

/// Every type `AddExtraCopy` has a case for, and some it has none for (0x05, 0x93, 0xa0, 0xff).
const XD_COPY_KINDS: &[u32] = &[
    0x01, 0x02, 0x03, 0x04, 0x05, 0x07, 0x08, 0x0a, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x13, 0x14,
    0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25,
    0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2f, 0x30, 0x31, 0x34, 0x37, 0x3b, 0x3c, 0x3e, 0x3f,
    0x40, 0x44, 0x46, 0x48, 0x4a, 0x4b, 0x4c, 0x51, 0x52, 0x53, 0x54, 0x55, 0x57, 0x58, 0x59, 0x5a,
    0x5c, 0x5d, 0x5e, 0x61, 0x62, 0x63, 0x65, 0x66, 0x67, 0x68, 0x6b, 0x6e, 0x6f, 0x71, 0x72, 0x73,
    0x74, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b, 0x80, 0x81, 0x84, 0x85, 0x8b, 0x8c, 0x8d, 0x90, 0x91,
    0x92, 0x93, 0xa0, 0xff,
];

/// `AddExtraCopy` of every type (and of none), into the world's list.
#[test]
fn xd_add_extra_copy() {
    Diff::new(entry!(
        0x0041_03d0,
        extra_data::copy::add_extra_copy(u32, u32)
    ))
    .extra_data_case()
    .max_cases(1600)
    .inputs(&[Input::Of(XD_COPY_KINDS), Input::Of(EXTRA_SEEDS)])
    .setup(|e, case| {
        let seed = case.word(1);
        let w = xd_world(e, seed);
        let mut d = Draw(seed ^ case.word(0).wrapping_mul(0x27d4_eb2d));
        let extra = if d.next(16) == 0 {
            0
        } else {
            xd_source(e, &mut d, case.word(0))
        };
        if case.word(0) == 0x18 && extra != 0 {
            // `__RTDynamicCast` stands in; its object none, as the world has no RTTI.
            e.mem.set_u32(extra + 0xc, 0);
        }
        vec![w.list, extra]
    })
    .run()
    .assert_same();
}

/// `IsCopyableExtra` of every type and of none.
#[test]
fn xd_is_copyable_extra() {
    Diff::new(entry!(0x0041_1f20, extra_data::copy::is_copyable_extra(u32, u32) -> bool))
        .extra_data_case()
        .inputs(&[Input::Of(XD_COPY_KINDS), Input::Flag])
        .setup(|e, case| {
            let w = xd_world(e, 0);
            let vt = e.mem.u32(XD_VT);
            let extra = if case.word(1) == 0 {
                0
            } else {
                extra_data(e, vt, case.word(0), 1)
            };
            vec![w.list, extra]
        })
        .run()
        .assert_same();
}

/// The world's list and a flag (`RemoveAllCopyableExtra`'s destroy).
#[test]
fn xd_remove_all_copyable_extra() {
    Diff::new(entry!(
        0x0041_1fd0,
        extra_data::copy::remove_all_copyable_extra(u32, bool)
    ))
    .extra_data_case()
    .inputs(&[Input::Of(EXTRA_SEEDS), Input::Flag])
    .setup(|e, case| vec![xd_world(e, case.word(0)).list, case.word(1)])
    .run()
    .assert_same();
}

/// The types `RemoveNonPersistentCellData` keeps (1, 2 by the thread's flag, 4) among others.
const XD_PERSIST_KINDS: &[u32] = &[0x00, 0x01, 0x02, 0x04, 0x0d, 0x21, 0x2b];

#[test]
fn xd_remove_non_persistent_cell_data() {
    Diff::new(entry!(
        0x0041_20b0,
        extra_data::copy::remove_non_persistent_cell_data(u32)
    ))
    .extra_data_case()
    .inputs(&[Input::Of(EXTRA_SEEDS)])
    .setup(|e, case| {
        xd_world(e, case.word(0));
        vec![xd_list(e, case.word(0), XD_PERSIST_KINDS)]
    })
    .run()
    .assert_same();
}

#[test]
fn xd_thread_flag_4() {
    Diff::new(entry!(0x0041_21b0, extra_data::copy::thread_flag_4(u32) -> bool))
        .extra_data_case_but(0x0041_21b0)
        .inputs(&[Input::Of(EXTRA_SEEDS)])
        .setup(|e, case| {
            xd_world(e, case.word(0));
            vec![0x011d_df38]
        })
        .run()
        .assert_same();
}

/// The list copies: the world's list and a source list of the copies' types (none for some).
trait XdCopyLists {
    fn xd_copy_lists(self, flags: bool) -> Diff;
}

impl XdCopyLists for Diff {
    fn xd_copy_lists(self, flags: bool) -> Diff {
        let inputs: &[Input] = if flags {
            &[Input::Of(EXTRA_SEEDS), Input::Flag]
        } else {
            &[Input::Of(EXTRA_SEEDS)]
        };
        self.extra_data_case().inputs(inputs).setup(move |e, case| {
            let w = xd_world(e, case.word(0));
            let source = xd_copy_source(e, case.word(0));
            let mut words = vec![w.list, source];
            if flags {
                words.push(case.word(1));
            }
            words
        })
    }
}

#[test]
fn xd_copy_list() {
    Diff::new(entry!(0x0041_1ec0, extra_data::copy::copy_list(u32, u32)))
        .extra_data_case()
        .inputs(&[Input::Of(EXTRA_SEEDS), Input::Flag])
        .setup(|e, case| {
            let w = xd_world(e, case.word(0));
            let source = if case.word(1) == 0 {
                0
            } else {
                xd_list(e, case.word(0), XD_SOURCE_KINDS)
            };
            vec![w.list, source]
        })
        .run()
        .assert_same();
}

#[test]
fn xd_copy_list_for_container() {
    Diff::new(entry!(
        0x0041_21e0,
        extra_data::copy::copy_list_for_container(u32, u32, bool)
    ))
    .xd_copy_lists(true)
    .run()
    .assert_same();
}

#[test]
fn xd_duplicate_for_container() {
    Diff::new(entry!(
        0x0041_2380,
        extra_data::copy::duplicate_for_container(u32, u32)
    ))
    .xd_copy_lists(false)
    .run()
    .assert_same();
}

#[test]
fn xd_copy_list_for_reference() {
    Diff::new(entry!(
        0x0041_2490,
        extra_data::copy::copy_list_for_reference(u32, u32, bool)
    ))
    .xd_copy_lists(true)
    .run()
    .assert_same();
}

#[test]
fn xd_compare_list_for_container() {
    Diff::new(entry!(
        0x0041_26c0,
        extra_data::copy::compare_list_for_container(u32, u32, bool, bool) -> bool
    ))
    .extra_data_case()
    .inputs(&[Input::Of(EXTRA_SEEDS), Input::Flag, Input::Flag])
    .setup(|e, case| {
        let w = xd_world(e, case.word(0));
        let other = xd_other(e, &w, case.word(0));
        vec![w.list, other, case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn xd_compare_list() {
    Diff::new(entry!(0x0041_27e0, extra_data::copy::compare_list(u32, u32) -> bool))
        .extra_data_case()
        .inputs(&[Input::Of(EXTRA_SEEDS)])
        .setup(|e, case| {
            let w = xd_world(e, case.word(0));
            let other = xd_other(e, &w, case.word(0));
            vec![w.list, other]
        })
        .run()
        .assert_same();
}

/// The family's worlds reach what the functions branch on: each placement of a getter's type, a
/// failing `operator new`, the thread flag both ways, JIP's sites with JIP on, every form of the
/// compares' other list and both answers of each compare and lookup (run as the translations).
#[test]
fn the_extra_data_worlds_reach_every_branch() {
    let mut reached = std::collections::BTreeSet::<String>::new();
    for &seed in EXTRA_SEEDS {
        for kind in [0x10, 0x2b, 0x4f, 0x8d] {
            reached.insert(format!("placed {}", xd_place(seed, kind)));
        }
        reached.insert(format!("other form {}", Draw(seed ^ 0x0041_27e0).next(4)));
        let mut e = Engine::new_untyped_for_test();
        let w = xd_world(&mut e, seed);
        reached.insert(format!("new fails {}", e.mem.u32(XD_NEW_FAILS)));
        let tls = e.tls();
        reached.insert(format!("thread flag {}", e.mem.u32(tls + 0x294) & 4 != 0));
        reached.insert(format!("JIP {}", w.jip));
        place_kind(&mut e, &w, seed, 0x4f);
        let found = e.call(0x0041_0220, &args![w.list, 0x4fu32]).u32() != 0;
        reached.insert(format!("found {found}"));
        let other = xd_other(&mut e, &w, seed);
        let differs = e.call(0x0041_27e0, &args![w.list, other]).bool();
        reached.insert(format!("lists differ {differs}"));
        let differs = e
            .call(0x0041_26c0, &args![w.list, other, 1u32, 0u32])
            .bool();
        reached.insert(format!("container lists differ {differs}"));
        // Without `allow` the container compare walks the owner first, where there is one, under
        // each JIP state; with JIP's fixes and its handler its owner site is asked.
        let head = e.mem.u32(other + 4);
        if head != 0 && e.mem.u32(head + 4) == 0x21 {
            e.call(0x0041_26c0, &args![w.list, other, 0u32, 0u32]);
            let asked = xd_site_rows(&e).iter().any(|r| r[0] == 0x0041_271e);
            reached.insert(format!(
                "an owner walked under JIP {}, site asked {asked}",
                w.jip
            ));
        }
        // The list copies, as their Diffs run them: the words JIP's copy and hotkey sites are
        // handed.
        for function in [0x0041_21e0u32, 0x0041_2490] {
            for flag in [0u32, 1] {
                let mut e = Engine::new_untyped_for_test();
                let w = xd_world(&mut e, seed);
                let source = xd_copy_source(&mut e, seed);
                e.call(function, &args![w.list, source, flag]);
                for row in xd_site_rows(&e) {
                    reached.insert(match row[0] {
                        0x0041_1978 => format!("copy site from {:08x} flag {}", row[3], row[4]),
                        site => format!("site {site:08x} asked"),
                    });
                }
            }
        }
        let source = xd_list(&mut e, seed, XD_SOURCE_KINDS);
        let n = e.call(0x0040_fe20, &args![source]).i32();
        reached.insert(format!("a source of {}", n.min(3)));
    }
    let mut wanted: Vec<String> = (0..=3).map(|n| format!("placed {n}")).collect();
    wanted.extend((0..=3).map(|n| format!("other form {n}")));
    wanted.extend((0..=3).map(|n| format!("a source of {n}")));
    wanted.extend((0..=3).map(|n| format!("JIP {n}")));
    for flag in ["false", "true"] {
        for what in [
            "thread flag",
            "found",
            "lists differ",
            "container lists differ",
        ] {
            wanted.push(format!("{what} {flag}"));
        }
    }
    wanted.extend(["new fails 0", "new fails 1"].map(String::from));
    for jip in 0..=3 {
        wanted.push(format!(
            "an owner walked under JIP {jip}, site asked {}",
            jip == 2
        ));
    }
    for caller in [0x0041_2250, 0x0041_2523] {
        for flag in 0..=1 {
            wanted.push(format!("copy site from {caller:08x} flag {flag}"));
        }
    }
    wanted.push("site 00412675 asked".to_string());
    for name in wanted {
        assert!(
            reached.contains(&name),
            "no seed reaches {name}: {reached:?}"
        );
    }
}

/// The part 2 range's setups reach their thin branches (run as the translations, every miss listed
/// at once): the terminal state's delete on 0 and the action's delete on 1 as `xd_setter` builds
/// their worlds, and a run-once search whose answer the stop at an empty item decides, as
/// `xe_run_once` builds it.
#[test]
fn the_part_2_worlds_reach_every_branch() {
    let mut reached = std::collections::BTreeSet::<String>::new();
    for &seed in EXTRA_SEEDS {
        for (setter, kind, value) in [(0x0041_be10u32, 0x50u32, 0u32), (0x0041_b3d0, 0x0e, 1)] {
            let mut e = Engine::new_untyped_for_test();
            let w = xd_world(&mut e, seed);
            place_kind(&mut e, &w, seed, kind);
            let extra = e.call(0x0041_0220, &args![w.list, kind]).u32();
            if extra == 0 {
                continue;
            }
            let deletes = e.mem.u32(extra + EXTRA_DELETES);
            e.call(setter, &args![w.list, value]);
            let deleted = e.mem.u32(extra + EXTRA_DELETES) != deletes;
            reached.insert(format!("type {kind:#x} deleted {deleted}"));
        }
        for second in 0..5 {
            let mut e = Engine::new_untyped_for_test();
            let words = xe_run_once_world(&mut e, seed, second, false);
            let found = e.call(0x0041_d8c0, &args![words[0], words[1]]).bool();
            // The answer of a walk that passes over empty items.
            let extra = e.call(0x0041_0220, &args![words[0], 0x1bu32]).u32();
            let mut node = if extra == 0 {
                0
            } else {
                e.mem.u32(extra + 0xc)
            };
            let mut anywhere = false;
            while node != 0 {
                let entry = e.mem.u32(node);
                anywhere |= entry != 0 && e.mem.u32(entry) == words[1];
                node = e.mem.u32(node + 4);
            }
            reached.insert(format!(
                "run-once found {found}, past an empty item {anywhere}"
            ));
        }
    }
    let wanted = [
        "type 0x50 deleted true",
        "type 0x50 deleted false",
        "type 0xe deleted true",
        "type 0xe deleted false",
        "run-once found true, past an empty item true",
        "run-once found false, past an empty item false",
        "run-once found false, past an empty item true",
    ];
    let missed: Vec<&str> = wanted
        .into_iter()
        .filter(|name| !reached.contains(*name))
        .collect();
    assert!(missed.is_empty(), "no seed reaches {missed:?}: {reached:?}");
}

// ---- The extra data family's loader, InitItem and save writer ----------------------------------

/// What the loader, `InitItem` and the save writer do in place beside the family's own: the file's
/// chunk size and byte-swap flag, a form's type and id, a reference's extra list, a simple list's
/// emptiness and item store, the map marker's flags and reputation, the audio marker's controller
/// and its store, the big-endian flag, the distant data's point constructor.
const XF_IN_PLACE: &[u32] = &[
    0x0040_1660,
    0x0040_1680,
    0x0040_1170,
    0x0084_e3a0,
    0x005d_43c0,
    0x0082_56d0,
    0x0072_6c60,
    0x0043_8ed0,
    0x0043_8ef0,
    0x0044_de80,
    0x0044_edb0,
    0x0059_bb30,
    0x0070_37c0,
    0x0040_1500,
    0x0041_6870,
];

trait XfCase {
    fn xf_case(self) -> Diff;
}

/// The forms family's functions the three call as typed calls (the lookup by id, `AddCompileIndex`,
/// `TESForm::GetFile` and the six chunk adders): their stand-ins here, so by address, their direct
/// calls in the forms tests.
const XF_FORMS: &[u32] = &[
    0x0048_39c0,
    0x0048_5d50,
    0x0048_4e60,
    0x0048_56d0,
    0x0048_56f0,
    0x0048_5710,
    0x0048_58f0,
    0x0048_5910,
    0x0048_5990,
];

impl XfCase for Diff {
    fn xf_case(self) -> Diff {
        self.extra_data_case()
            .family(&engine_typed::pathing::PATHING)
            .ignore_calls(XF_IN_PLACE)
            .goes_by_address(XF_FORMS)
    }
}

/// The stand-ins' seed, the chunk the file stands at, their draw count, the file's next word, the
/// pool of blocks, and a log of what the messages, the chunk adds and the coins said.
const XF_SEED: u32 = CONTROL + 0x400;
const XF_CHUNK: u32 = CONTROL + 0x404;
const XF_DRAWS: u32 = CONTROL + 0x408;
const XF_POOL_AT: u32 = CONTROL + 0x410;
const XF_LOG_N: u32 = CONTROL + 0x440;
const XF_LOG: u32 = CONTROL + 0x444;
const XF_LOG_MAX: u32 = 0x60;
/// A reference's vtable slot 0x100 (whether it is an actor that lives) answers a coin.
const XF_COIN_FN: u32 = 0x00ff_c200;
/// The pool's blocks: each word 0, a small id (page 0), or a later block, so walks end.
const XF_POOL: u32 = 6;
const XF_SMALL: [u32; 3] = [0x40, 0x1e, 0x37];

/// A draw of the stand-ins: the case's seed and their count.
fn xf_draw(e: &mut Engine, n: u32) -> u32 {
    let k = e.mem.u32(XF_DRAWS).wrapping_add(1);
    e.mem.set_u32(XF_DRAWS, k);
    Draw(k.wrapping_mul(0x9e37_79b9) ^ e.mem.u32(XF_SEED)).next(n)
}

/// A line of the log.
fn xf_note(e: &mut Engine, word: u32) {
    let n = e.mem.u32(XF_LOG_N);
    if n < XF_LOG_MAX {
        e.mem.set_u32(XF_LOG + 4 * n, word);
        e.mem.set_u32(XF_LOG_N, n + 1);
    }
}

/// The log's lines.
fn xf_lines(e: &Engine) -> Vec<u32> {
    (0..e.mem.u32(XF_LOG_N))
        .map(|i| e.mem.u32(XF_LOG + 4 * i))
        .collect()
}

/// A stand-in answering 0 or 1 (a load's success, a loop check, a health, a string length).
fn xf_coin(e: &mut Engine, _: &[u32]) -> Ret {
    let coin = xf_draw(e, 2);
    xf_note(e, 0xc0_0000 | coin);
    ret(coin)
}

/// A stand-in answering its second word or 0 (a region found or not, an occlusion plane).
fn xf_maybe(e: &mut Engine, a: &[u32]) -> Ret {
    let pick = xf_draw(e, 3);
    ret(if pick == 0 {
        0
    } else {
        a.get(1).copied().unwrap_or(a[0])
    })
}

/// The log (`005b5e40`) and the chunk adds: their first word logged.
fn xf_logged(e: &mut Engine, a: &[u32]) -> Ret {
    xf_note(e, a[0]);
    Ret::default()
}

/// `TESFile::GetTESChunk`: the case's chunk.
fn xf_chunk(e: &mut Engine, _: &[u32]) -> Ret {
    ret(e.mem.u32(XF_CHUNK))
}

/// Words the file holds: none, one, a count of three with the master flag, a small id, a
/// signalling NaN, 1.0, a negative count of two.
const XF_FILE_WORDS: [u32; 7] = [
    0,
    1,
    0x0001_0003,
    0x40,
    0x7fa0_0001,
    0x3f80_0000,
    0x8000_0002,
];

/// `TESFile::GetChunkData(&word)`: the next word of the file into the local.
fn xf_chunk_word(e: &mut Engine, a: &[u32]) -> Ret {
    let word = XF_FILE_WORDS[xf_draw(e, 7) as usize];
    e.mem.set_u32(a[1], word);
    ret(1)
}

/// `TESFile::GetChunkData(buffer, size)`: the file's words into the buffer (its last bytes too).
fn xf_chunk_data(e: &mut Engine, a: &[u32]) -> Ret {
    for i in 0..a[2] {
        if i % 4 == 0 {
            let word = XF_FILE_WORDS[xf_draw(e, 7) as usize];
            for (k, byte) in word.to_le_bytes().into_iter().enumerate() {
                if i + (k as u32) < a[2] {
                    e.mem.set_u8(a[1] + i + k as u32, byte);
                }
            }
        }
    }
    ret(1)
}

/// `TESForm::AddCompileIndex(&id, file)`: the id's bit 0 flipped.
fn xf_compile_index(e: &mut Engine, a: &[u32]) -> Ret {
    let id = e.mem.u32(a[0]);
    e.mem.set_u32(a[0], id ^ 1);
    Ret::default()
}

/// The lookup by id: no form for some ids, else a block of the pool.
fn xf_lookup(e: &mut Engine, a: &[u32]) -> Ret {
    if a[0] % 3 == 0 {
        return ret(0);
    }
    let pool = e.mem.u32(XF_POOL_AT + 4 * (a[0] % XF_POOL));
    ret(pool)
}

/// `__RTDynamicCast`: the form, or none for some forms and targets.
fn xf_cast(e: &mut Engine, a: &[u32]) -> Ret {
    let none = xf_draw(e, 4) == 0;
    ret(if none { 0 } else { a[0] })
}

/// `BSSimpleList::Count`: the nodes of the list.
fn xf_list_count(e: &mut Engine, a: &[u32]) -> Ret {
    let (mut node, mut n) = (a[0], 0u32);
    while node != 0 && n < 16 {
        n += 1;
        node = e.mem.u32(node + 4);
    }
    ret(n)
}

/// What the loader, `InitItem` and the save writer call by address in other units and the list's
/// other part files: each a stand-in that counts and answers its first word.
const XF_NOTED: &[u32] = &[
    0x0047_26f0,
    0x0089_1170,
    0x0050_3210,
    0x0060_ce80,
    0x0046_2230,
    0x004f_6320,
    0x004f_6600,
    0x005a_e3d0,
    0x004d_9330,
    0x0043_a160,
    0x0043_8bb0,
    0x0058_9450,
    0x0068_0890,
    0x0055_a400,
    0x0043_8f50,
    0x0069_2870,
    0x00c3_35d0,
    0x0067_c690,
    0x0043_a4e0,
    0x0043_8ca0,
    0x0058_9600,
    0x0058_9890,
    0x0043_8fc0,
    0x0069_dfd0,
    0x0043_2f30,
    0x0067_dd20,
    0x0043_0a90,
    0x004a_4e90,
    0x0041_e160,
    0x0041_e250,
    0x0041_e440,
    0x0042_1430,
    0x0042_1b70,
    0x0042_1c60,
    0x0042_1d50,
    0x0042_1e40,
    0x0042_1540,
    0x0042_2220,
    0x0042_2350,
    0x0042_eb60,
    0x0041_dc70,
    0x0041_ece0,
    0x0041_f080,
    0x0042_1f30,
    0x0042_e2c0,
    0x0041_f1a0,
    0x0041_f330,
    0x0041_f940,
    0x0041_fa60,
    0x0042_2150,
    0x0041_fc10,
    0x0041_fe90,
    0x0041_fec0,
    0x0042_0210,
    0x0042_0440,
    0x0042_0fd0,
    0x0042_f200,
    0x0041_fd00,
    0x0048_4e60,
    0x0046_1630,
    0x0043_7730,
    0x0041_dcd0,
    0x0041_e530,
    0x0043_3db0,
    0x0043_3ed0,
    0x0043_4050,
    0x0067_c6c0,
    0x0042_0ce0,
    0x0058_f210,
    0x0041_edd0,
    0x0090_5330,
    0x0063_f7b0,
    0x004d_93c0,
    0x0043_a420,
    0x0043_8c10,
    0x0043_8f90,
    0x0058_9560,
    0x0067_dc80,
    0x007d_6bb0,
    0x00a5_8550,
    0x0041_ec80,
    0x0040_37d0,
];

/// The stand-ins of the extra data world and these three's: the file, the forms, the casts, the
/// log and chunk adds, the coins, the ragdoll extra data's constructor.
fn xf_stand_ins(e: &mut Engine) {
    for &addr in XF_NOTED {
        e.register(addr, noted);
    }
    e.register(0x0047_26b0, xf_chunk);
    e.register(0x0047_27f0, xf_chunk_word);
    e.register(0x0047_2890, xf_chunk_data);
    e.register(0x0048_5d50, xf_compile_index);
    e.register(0x0048_39c0, xf_lookup);
    e.register(0x00ec_43fb, xf_cast);
    e.register(0x005a_e380, xf_list_count);
    e.register(0x00aa_13e0, xd_new);
    e.register(0x0043_2cb0, built::<0x14>);
    for addr in [
        0x005b_5e40,
        0x0048_5910,
        0x0048_58f0,
        0x0048_56d0,
        0x0048_5990,
        0x0048_5710,
        0x0048_56f0,
    ] {
        e.register(addr, xf_logged);
    }
    for addr in [
        0x004d_94a0,
        0x0043_a590,
        0x0056_aac0,
        0x0057_5d10,
        0x005f_0b00,
        XF_COIN_FN,
    ] {
        e.register(addr, xf_coin);
    }
    // `BSString::Length` (`004048e0`): the string's 16-bit length at +4 (no 0xffff strings here).
    e.register(0x0040_48e0, |e, a| ret(u32::from(e.mem.u16(a[0] + 4))));
    for addr in [0x0042_eb40, 0x004f_66c0, 0x0042_2120] {
        e.register(addr, xf_maybe);
    }
}

/// The pool: `XF_POOL` blocks of 0x100 bytes, each word 0, a small id or a later block (so every
/// walk through them ends), their addresses at `XF_POOL_AT`.
fn xf_pool(e: &mut Engine, d: &mut Draw) -> Vec<u32> {
    let pool: Vec<u32> = (0..XF_POOL).map(|_| e.mem.alloc(0x100)).collect();
    for (i, &block) in pool.iter().enumerate() {
        e.mem.set_u32(XF_POOL_AT + 4 * i as u32, block);
        for offset in (0..0x100).step_by(4) {
            let later = &pool[i + 1..];
            let pick = d.next(4 + later.len() as u32) as usize;
            let word = match pick {
                0 => 0,
                1..=3 => XF_SMALL[pick - 1],
                _ => later[pick - 4],
            };
            e.mem.set_u32(block + offset, word);
        }
    }
    pool
}

/// A word of an extra data's fields: 0, a small id or a block of the pool.
fn xf_field(d: &mut Draw, pool: &[u32]) -> u32 {
    let pick = d.next(4 + pool.len() as u32) as usize;
    match pick {
        0 => 0,
        1..=3 => XF_SMALL[pick - 1],
        _ => pool[pick - 4],
    }
}

/// The extra data world (`xd_world`) with the page past page 0 mapped (the world chain's payloads
/// point there), the big-endian flag for some seeds, the stand-ins and the pool.
fn xf_world(e: &mut Engine, seed: u32) -> (ExtraWorld, Vec<u32>) {
    let w = xd_world(e, seed);
    e.map(0x1000, 0x1000);
    e.map(0x011c_5000, 0x1000);
    let mut d = Draw(seed.wrapping_mul(0x2c1b_3c6d) ^ 0x0041_2970);
    e.mem.set_u8(0x011c_54ba, u8::from(d.next(4) == 0));
    e.mem.set_u32(XF_SEED, seed);
    xf_stand_ins(e);
    let pool = xf_pool(e, &mut d);
    (w, pool)
}

/// `n` extra data of `kinds` added at the head of the world's list, their fields from the pool.
fn xf_chain(e: &mut Engine, w: &ExtraWorld, pool: &[u32], d: &mut Draw, kinds: &[u32], n: u32) {
    let vt = e.mem.u32(XD_VT);
    for _ in 0..n {
        let kind = kinds[d.next(kinds.len() as u32) as usize];
        let x = e.mem.alloc(0x40);
        let head = e.mem.u32(w.list + 4);
        for (k, word) in [vt, kind, head].into_iter().enumerate() {
            e.mem.set_u32(x + 4 * k as u32, word);
        }
        for offset in (0xc..0x40).step_by(4) {
            let word = xf_field(d, pool);
            e.mem.set_u32(x + offset, word);
        }
        e.mem.set_u32(w.list + 4, x);
        if kind >> 3 < 0x15 {
            let flags = e.mem.u8(w.list + 8 + (kind >> 3)) | (1 << (kind & 7));
            e.mem.set_u8(w.list + 8 + (kind >> 3), flags);
        }
    }
}

/// The chunks the loader knows, and one it does not.
const fn xf_tag(name: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*name)
}
const XF_CHUNKS: &[u32] = &[
    xf_tag(b"XACT"),
    xf_tag(b"XOWN"),
    xf_tag(b"XGLB"),
    xf_tag(b"XRNK"),
    xf_tag(b"XPSN"),
    xf_tag(b"XCMO"),
    xf_tag(b"XCAS"),
    xf_tag(b"XCCM"),
    xf_tag(b"XCIM"),
    xf_tag(b"XCWT"),
    xf_tag(b"XRTM"),
    xf_tag(b"XLKR"),
    xf_tag(b"XMRC"),
    xf_tag(b"XTRG"),
    xf_tag(b"XEZN"),
    xf_tag(b"XEMI"),
    xf_tag(b"XMBR"),
    xf_tag(b"XLCM"),
    xf_tag(b"XHLP"),
    xf_tag(b"XTIM"),
    xf_tag(b"XCHG"),
    xf_tag(b"XRDS"),
    xf_tag(b"XRAD"),
    xf_tag(b"XHLT"),
    xf_tag(b"XUSE"),
    xf_tag(b"XCNT"),
    xf_tag(b"XAMC"),
    xf_tag(b"XAMT"),
    xf_tag(b"XLOC"),
    xf_tag(b"XRDO"),
    xf_tag(b"XTEL"),
    xf_tag(b"XMRK"),
    xf_tag(b"MMRK"),
    xf_tag(b"AMRK"),
    xf_tag(b"XSED"),
    xf_tag(b"XCLR"),
    xf_tag(b"XPSL"),
    xf_tag(b"XRGD"),
    xf_tag(b"XRGB"),
    xf_tag(b"XLOD"),
    xf_tag(b"XESP"),
    xf_tag(b"XAPD"),
    xf_tag(b"XATO"),
    xf_tag(b"XAPR"),
    xf_tag(b"XACR"),
    xf_tag(b"XDCR"),
    xf_tag(b"XMBO"),
    xf_tag(b"XPCI"),
    xf_tag(b"XNVP"),
    xf_tag(b"XNDP"),
    xf_tag(b"XPWR"),
    xf_tag(b"XLTW"),
    xf_tag(b"XPRM"),
    xf_tag(b"XOCP"),
    xf_tag(b"XPTL"),
    xf_tag(b"XPRD"),
    xf_tag(b"XORD"),
    xf_tag(b"XPOD"),
    xf_tag(b"XRMR"),
    xf_tag(b"XTRI"),
    xf_tag(b"XPPA"),
    xf_tag(b"XIBS"),
    xf_tag(b"XSRF"),
    xf_tag(b"XSRD"),
    xf_tag(b"XCMT"),
    xf_tag(b"XCET"),
    xf_tag(b"ZZZZ"),
];
const XF_LOAD_SEEDS: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

/// The types whose extra data the loader looks for, placed in the list for some seeds with a
/// payload block of zeros or none.
const XF_LOAD_KINDS: [u32; 9] = [0x2a, 0x14, 0x53, 0x92, 0x2b, 0x2c, 0x90, 0x91, 0x68];

/// The loader's world: the list with some of [`XF_LOAD_KINDS`], the file (its chunk size 4, 1, 8,
/// 0x13 or 0x10 in turn over the seeds, the swap flag set for some seeds), the save buffer (its form none, of type 0x23 or
/// another). Answers the list, the file and the buffer.
fn xf_load_world(e: &mut Engine, seed: u32, chunk: u32) -> Vec<u32> {
    let (w, pool) = xf_world(e, seed);
    e.mem.set_u32(XF_CHUNK, chunk);
    let mut d = Draw(seed.wrapping_mul(0x7f4a_7c15) ^ chunk);
    let vt = e.mem.u32(XD_VT);
    for kind in XF_LOAD_KINDS {
        if d.next(2) == 0 {
            continue;
        }
        let x = e.mem.alloc(0x40);
        let payload = if d.next(4) == 0 { 0 } else { e.mem.alloc(0x40) };
        let head = e.mem.u32(w.list + 4);
        for (k, word) in [vt, kind, head, payload].into_iter().enumerate() {
            e.mem.set_u32(x + 4 * k as u32, word);
        }
        e.mem.set_u32(w.list + 4, x);
        let flags = e.mem.u8(w.list + 8 + (kind >> 3)) | (1 << (kind & 7));
        e.mem.set_u8(w.list + 8 + (kind >> 3), flags);
    }
    let file = e.mem.alloc(0x300);
    // Each chunk meets each size across the seeds (the draw is kept, so the rest is as it was).
    d.next(5);
    let size = [4, 1, 8, 0x13, 0x10][(seed.wrapping_add(chunk) % 5) as usize];
    e.mem.set_u32(file + 0x25c, size);
    e.mem.set_u8(file + 0x299, u8::from(d.next(3) == 0));
    let buffer = e.mem.alloc(0x40);
    let form = [0, pool[0], pool[1]][d.next(3) as usize];
    e.mem.set_u32(buffer + 0x20, form);
    if form == pool[0] {
        e.mem.set_u8(pool[0] + 4, 0x23);
    }
    vec![w.list, file, buffer]
}

#[test]
fn xf_load_chunk() {
    Diff::new(entry!(
        0x0041_44a0,
        extra_data::load::load_chunk(u32, u32, u32)
    ))
    .xf_case()
    .max_cases(XF_LOAD_SEEDS.len() * XF_CHUNKS.len())
    .inputs(&[Input::Of(XF_LOAD_SEEDS), Input::Of(XF_CHUNKS)])
    .setup(|e, case| xf_load_world(e, case.word(0), case.word(1)))
    .run()
    .assert_same();
}

/// The types `InitItem` and the save writer have arms for, and two they leave alone.
const XF_LIST_KINDS: &[u32] = &[
    0x03, 0x04, 0x07, 0x08, 0x0e, 0x13, 0x14, 0x18, 0x1e, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27,
    0x28, 0x2a, 0x2b, 0x2c, 0x31, 0x37, 0x3b, 0x3c, 0x3f, 0x44, 0x51, 0x53, 0x57, 0x59, 0x5a, 0x5c,
    0x5d, 0x61, 0x62, 0x63, 0x66, 0x67, 0x68, 0x6b, 0x6e, 0x6f, 0x71, 0x72, 0x74, 0x76, 0x77, 0x78,
    0x7a, 0x7b, 0x80, 0x81, 0x85, 0x8c, 0x8d, 0x90, 0x91, 0x92, 0x0d, 0xa0,
];
const XF_LIST_SEEDS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
];

/// A list for `InitItem` and the save writer: the world's with 2 to 7 extra data of
/// [`XF_LIST_KINDS`] at its head, and the reference (its vtable's slot 0x100 a coin, its words from
/// the pool). Answers the list and the reference.
fn xf_list_world(e: &mut Engine, seed: u32) -> Vec<u32> {
    let (w, pool) = xf_world(e, seed);
    let mut d = Draw(seed.wrapping_mul(0x6c07_8965) ^ 0x0041_6be0);
    let n = d.next(5);
    xf_chain(e, &w, &pool, &mut d, XF_LIST_KINDS, n);
    // Four types in turn at the head, so each type is in the lists of three seeds: with its fields
    // as drawn (seeds 0 to 14), all 0 (15 to 29: empty lists, no forms), or with its payload a block
    // of an id the lookup does not find (30 to 44).
    let count = XF_LIST_KINDS.len() as u32;
    for j in 0..4 {
        let kind = XF_LIST_KINDS[((seed * 4 + j) % count) as usize];
        xf_chain(e, &w, &pool, &mut d, &[kind], 1);
        let head = e.mem.u32(w.list + 4);
        match seed / 15 {
            1 => {
                for offset in (0xc..0x40).step_by(4) {
                    e.mem.set_u32(head + offset, 0);
                }
            }
            2 => {
                // The block's own address is an id the lookup does not find too (`xf_lookup`).
                let mut block = e.mem.alloc(0x40);
                while (block ^ 1) % 3 != 0 {
                    block = e.mem.alloc(0x40);
                }
                for offset in (0..0x40).step_by(4) {
                    e.mem.set_u32(block + offset, 0x37);
                }
                e.mem.set_u32(head + 0xc, block);
                for offset in (0x10..0x40).step_by(4) {
                    e.mem.set_u32(head + offset, 0x37);
                }
            }
            _ => {}
        }
    }
    let vtable = e.mem.alloc(0x200);
    e.mem.set_u32(vtable + 0x100, XF_COIN_FN);
    let reference = e.mem.alloc(0x60);
    for offset in (4..0x60).step_by(4) {
        let word = xf_field(&mut d, &pool);
        e.mem.set_u32(reference + offset, word);
    }
    e.mem.set_u32(reference, vtable);
    vec![w.list, reference]
}

#[test]
fn xf_init_item() {
    Diff::new(entry!(
        0x0041_6be0,
        extra_data::init_item::init_item(u32, u32)
    ))
    .xf_case()
    .inputs(&[Input::Of(XF_LIST_SEEDS)])
    .setup(|e, case| xf_list_world(e, case.word(0)))
    .run()
    .assert_same();
}

#[test]
fn xf_save_list() {
    Diff::new(entry!(0x0041_2970, extra_data::save::save_list(u32)))
        .xf_case()
        .inputs(&[Input::Of(XF_LIST_SEEDS)])
        .setup(|e, case| vec![xf_list_world(e, case.word(0))[0]])
        .run()
        .assert_same();
}

/// The loader, `InitItem` and the save writer, run as their translations over the tests' worlds,
/// reach every arm's branches: each message `InitItem` logs, each chunk the save writer adds, both
/// sides of the coins, the big-endian flag and the file's swap flag.
#[test]
fn the_list_load_worlds_reach_every_branch() {
    let mut lines = std::collections::BTreeSet::<u32>::new();
    let mut facts = std::collections::BTreeSet::<String>::new();
    for &seed in XF_LIST_SEEDS {
        for function in [0x0041_6be0u32, 0x0041_2970] {
            let mut e = Engine::new_untyped_for_test();
            let words = xf_list_world(&mut e, seed);
            facts.insert(format!("big-endian {}", e.mem.u8(0x011c_54ba)));
            if function == 0x0041_6be0 {
                e.call(function, &args![words[0], words[1]]);
            } else {
                e.call(function, &args![words[0]]);
            }
            lines.extend(xf_lines(&e));
        }
    }
    for &seed in XF_LOAD_SEEDS {
        for &chunk in &XF_CHUNKS[..XF_CHUNKS.len() - 1] {
            let mut e = Engine::new_untyped_for_test();
            let words = xf_load_world(&mut e, seed, chunk);
            facts.insert(format!("swaps {}", e.mem.u8(words[1] + 0x299)));
            let size = e.mem.u32(words[1] + 0x25c);
            facts.insert(format!("size {size}"));
            if chunk == xf_tag(b"XAPD") || chunk == xf_tag(b"XSED") {
                facts.insert(format!(
                    "{} size {size}",
                    String::from_utf8_lossy(&chunk.to_le_bytes())
                ));
            }
            e.call(0x0041_44a0, &args![words[0], words[1], words[2]]);
            lines.extend(xf_lines(&e));
        }
    }
    let messages: [u32; 37] = [
        0x0101_4d60,
        0x0101_4eb8,
        0x0101_4cf0,
        0x0101_4f50,
        0x0101_4830,
        0x0101_4f08,
        0x0101_47d8,
        0x0101_4e60,
        0x0101_4620,
        0x0101_4788,
        0x0101_4e00,
        0x0101_4984,
        0x0101_4db0,
        0x0101_4fa8,
        0x0101_4be0,
        0x0101_4b88,
        0x0101_5060,
        0x0101_4c90,
        0x0101_4c50,
        0x0101_4b28,
        0x0101_4890,
        0x0101_46b8,
        0x0101_4684,
        0x0101_4728,
        0x0101_50f8,
        0x0101_50b0,
        0x0101_4ab8,
        0x0101_4a80,
        0x0101_4a18,
        0x0101_49c0,
        0x0101_4920,
        0x0101_4658,
        0x0101_5000,
        0x0101_48ec,
        0x0101_48b8,
        0x0101_45ac,
        0x0101_45d8,
    ];
    let mut missing: Vec<String> = messages
        .iter()
        .filter(|message| !lines.contains(message))
        .map(|message| format!("message {message:08x}"))
        .collect();
    for name in [
        b"XCWT", b"XCLR", b"XACT", b"XLOD", b"XPSL", b"XCNT", b"XHLT", b"XLOC", b"XMRK", b"XSED",
        b"XESP", b"XRTM", b"XAPD", b"XAPR", b"XATO", b"XDCR", b"XNDP", b"XMBP", b"XPWR", b"XRDO",
        b"XPRM", b"XAMT", b"XPRD", b"XOCP", b"XPTL", b"XTRI", b"XORD", b"XPOD", b"XRMR", b"XLRM",
        b"XIBS", b"XLTW", b"MMRK", b"AMRK", b"XSRF", b"XSRD",
    ] {
        if !lines.contains(&xf_tag(name)) {
            missing.push(format!("chunk {}", String::from_utf8_lossy(name)));
        }
    }
    for coin in [0xc0_0000, 0xc0_0001] {
        if !lines.contains(&coin) {
            missing.push(format!("coin {coin:x}"));
        }
    }
    for fact in [
        "big-endian 0",
        "big-endian 1",
        "swaps 0",
        "swaps 1",
        "size 4",
        "size 1",
        "size 19",
        "XAPD size 4",
        "XAPD size 1",
        "XSED size 4",
        "XSED size 1",
    ] {
        if !facts.contains(fact) {
            missing.push(fact.to_string());
        }
    }
    assert!(missing.is_empty(), "no world reaches {missing:?}");
}

/// The loader's room count is the chunk word's low 16 bits (`& 0xffff`), a count with bit 15 set
/// too, in the translation and the native code alike: a plain run of each, since a Diff's call log
/// over 0x8001 rooms would take too long.
#[test]
fn xf_room_count_takes_the_low_sixteen_bits() {
    const ADDS: u32 = CONTROL + 0x480;
    for native in [false, true] {
        let mut e = Engine::new_untyped_for_test();
        let words = xf_load_world(&mut e, 0, xf_tag(b"XRMR"));
        // The first word read: 0x8001 rooms and the byte 1; every room's word 0.
        e.mem.set_u32(XF_DRAWS, 0);
        e.register(0x0047_27f0, |e, a| {
            let first = e.mem.u32(XF_DRAWS) == 0;
            e.mem.set_u32(XF_DRAWS, 1);
            e.mem.set_u32(a[1], if first { 0x0001_8001 } else { 0 });
            ret(1)
        });
        e.register(0x005a_e3d0, |e, _| {
            let n = e.mem.u32(ADDS);
            e.mem.set_u32(ADDS, n + 1);
            Ret::default()
        });
        if native {
            extra_data::load::load_chunk(&mut e, words[0], words[1], words[2]);
        } else {
            e.call(0x0041_44a0, &args![words[0], words[1], words[2]]);
        }
        assert_eq!(e.mem.u32(ADDS), 0x8001, "native {native}");
    }
}

// ---- The extra data family's part 2 range (`0041a6a0` to `0041da40`) ----------------------------

/// Where the part 2 range's stand-ins read what the setup chose: the package tests' flags (bit 0
/// created, bit 1 never runs) and the cell a reference's `TESChildCell` answers.
const XE_PACKAGE_FLAGS: u32 = CONTROL + 0x340;
const XE_CELL: u32 = CONTROL + 0x344;
/// An object's slots (a region list's delete, a package's slot 0x10, a reference's name) and a
/// reference's `TESChildCell` slot 0.
const XE_SLOT_FN: u32 = 0x00ff_c300;
const XE_CELL_FN: u32 = 0x00ff_c304;

/// What the part 2 range calls by address in other units: each a stand-in that counts (the
/// `NiPointer` assignment, the editor id's string, the base destructor's body, the reference's
/// persistence, the packages', ragdolls' and run-once packages' callees, the list adds and frees).
const XE_NOTED: &[u32] = &[
    0x0066_b0d0,
    0x0040_37b0,
    0x0040_37d0,
    0x0040_37f0,
    0x0043_2f30,
    0x0056_53d0,
    0x0067_4d70,
    0x004d_9800,
    0x004d_9670,
    0x004d_9330,
    0x0043_31c0,
    0x005a_e3d0,
    0x009e_ba00,
    0x0047_02f0,
    XE_SLOT_FN,
];

/// `TESObjectCELL::GetWorldSpace`: none for a cell with bit 4 set, else another word.
fn xe_world_space(_: &mut Engine, a: &[u32]) -> Ret {
    ret(if a[0] & 0x10 != 0 { 0 } else { a[0] ^ 0x100 })
}

/// `BSSimpleList::Remove(&item)`: the first node holding the item unlinked (the head's next moved
/// into the head), so a walk that starts again ends.
fn xe_list_remove(e: &mut Engine, a: &[u32]) -> Ret {
    noted(e, a);
    let want = e.mem.u32(a[1]);
    let (mut node, mut previous) = (a[0], 0);
    while node != 0 {
        if e.mem.u32(node) == want {
            let next = e.mem.u32(node + 4);
            if previous != 0 {
                e.mem.set_u32(previous + 4, next);
            } else if next != 0 {
                e.mem.set_u32(node, e.mem.u32(next));
                e.mem.set_u32(node + 4, e.mem.u32(next + 4));
            } else {
                e.mem.set_u32(node, 0);
            }
            break;
        }
        previous = node;
        node = e.mem.u32(node + 4);
    }
    ret(0)
}

/// The part 2 range's stand-ins (`xd_stand_ins` holds them): the counted callees, the world
/// space, the list remove, the package's two flags and the reference's cell from the setup's words.
fn xe_stand_ins(e: &mut Engine) {
    for &addr in XE_NOTED {
        e.register(addr, noted);
    }
    e.register(0x0054_ddd0, xe_world_space);
    e.register(0x0090_5330, xe_list_remove);
    e.register(0x0067_4d40, |e, _| ret(e.mem.u32(XE_PACKAGE_FLAGS) & 1));
    e.register(
        0x0067_4e40,
        |e, _| ret(e.mem.u32(XE_PACKAGE_FLAGS) >> 1 & 1),
    );
    e.register(XE_CELL_FN, |e, _| ret(e.mem.u32(XE_CELL)));
}

/// The extra data of `kind` the lookup can answer in the world besides the placed one: the entry the
/// thread's cache holds for the type (the world's decoy in some seeds), or 0.
fn xe_cached(e: &mut Engine, kind: u32) -> u32 {
    let tls = e.tls();
    e.mem.u32(tls + 0x10 + 4 * kind)
}

/// The payload word `held` stored in the placed extra data and in the one the cache holds for `kind`.
fn xe_hold(e: &mut Engine, placed: u32, kind: u32, held: u32) {
    let cached = xe_cached(e, kind);
    for x in [placed, cached] {
        if x != 0 {
            e.mem.set_u32(x + 0xc, held);
        }
    }
}

/// An object whose every vtable slot is [`XE_SLOT_FN`].
fn xe_object(e: &mut Engine) -> u32 {
    let vtable = e.mem.alloc(0x200);
    for slot in (0..0x200).step_by(4) {
        e.mem.set_u32(vtable + slot, XE_SLOT_FN);
    }
    let object = xd_block(e, 0x40, 0x3c);
    e.mem.set_u32(object, vtable);
    object
}

/// Bytes a byte setter is given: none, one, the top bit, all ones.
const XE_BYTES: &[u32] = &[0, 1, 0x80, 0xff];
/// The flag setters' flag.
const XE_FLAGS: &[u32] = &[0, 1];
/// The action flags' masks: none, the low bit, a byte's bits, a bit past the byte.
const XE_MASKS: &[u32] = &[0, 1, 0xfe, 0x100];
/// An action's flags: 1 (the default), 0, others, a word whose low byte is 1.
const XE_ACTIONS: &[u32] = &[1, 0, 2, 0xff, 0x101];

/// [`XeShapes::xe_run_once`]'s world: the run-once packages (type 0x1b) of up to five entries,
/// the third empty in some seeds (five with the third empty in a third of them); the second word a
/// flag, or the `second`th package of the list (the first one none of it, the fourth and fifth past
/// the empty item when there is one).
fn xe_run_once_world(e: &mut Engine, seed: u32, second: u32, flags: bool) -> Vec<u32> {
    let w = xd_world(e, seed);
    let x = place_kind(e, &w, seed, 0x1b);
    let mut d = Draw(seed.wrapping_mul(0x0041_d7b0) ^ 0x7e57);
    let mut packages = vec![0x4321];
    {
        let head = e.mem.alloc(8);
        let mut node = head;
        // A third of the seeds hold five entries, the third empty, so a search for a later package
        // meets the stop.
        let full = seed % 3 == 2;
        let n = d.next(5);
        for i in 0..if full { 5 } else { n } {
            let entry = if i == 2 && (full || d.next(3) == 0) {
                0
            } else {
                let entry = e.mem.alloc(8);
                let package = if d.next(4) == 0 {
                    0
                } else {
                    let package = xd_block(e, 0x40, i);
                    let day = [0u8, 0x10, 0x14, 0x15, 0xf0][d.next(5) as usize];
                    e.mem.set_u8(package + 0x38 + 3, day);
                    e.mem
                        .set_u32(package + 0x38 + 4, [0, 1, 5, u32::MAX][d.next(4) as usize]);
                    packages.push(package);
                    package
                };
                e.mem.set_u32(entry, package);
                e.mem.set_u8(entry + 4, [0u8, 1, 0xff][d.next(3) as usize]);
                entry
            };
            if i > 0 {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
            e.mem.set_u32(node, entry);
        }
        xe_hold(e, x, 0x1b, head);
    }
    let second = if flags {
        [0, 1, 0xff][second as usize % 3]
    } else {
        packages[second as usize % packages.len()]
    };
    vec![w.list, second]
}

/// The part 2 range's test shapes.
trait XeShapes {
    fn xe_pair(self, kinds: &'static [u32], first: &'static [u32], second: &'static [u32]) -> Diff;
    fn xe_owned(self, kind: u32, equal_too: bool) -> Diff;
    fn xe_start(self, setter: bool) -> Diff;
    fn xe_start_location(self) -> Diff;
    fn xe_for_ref(self) -> Diff;
    fn xe_canopy_set(self) -> Diff;
    fn xe_canopy_get(self) -> Diff;
    fn xe_package(self) -> Diff;
    fn xe_crimes(self) -> Diff;
    fn xe_scan(self, worn: &'static [u32]) -> Diff;
    fn xe_run_once(self, flags: bool) -> Diff;
    fn xe_with_block(self, kind: u32) -> Diff;
    fn xe_block_and(self, own: u32, values: &'static [u32]) -> Diff;
    fn xe_terminal_bytes(self) -> Diff;
}

impl XeShapes for Diff {
    /// (list, a, b) with an extra data of each of `kinds` placed or not.
    fn xe_pair(self, kinds: &'static [u32], first: &'static [u32], second: &'static [u32]) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * first.len() * second.len())
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(first), Input::Of(second)])
            .setup(move |e, case| {
                let w = xd_world(e, case.word(0));
                for &kind in kinds {
                    place_kind(e, &w, case.word(0), kind);
                }
                vec![w.list, case.word(1), case.word(2)]
            })
    }

    /// (list, value) for a setter that deletes the object the extra data holds: a placed one holds
    /// none, an object with a vtable, or (with `equal_too`) the value itself.
    fn xe_owned(self, kind: u32, equal_too: bool) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * XD_VALUES.len())
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(XD_VALUES)])
            .setup(move |e, case| {
                let w = xd_world(e, case.word(0));
                let x = place_kind(e, &w, case.word(0), kind);
                let held = match case.word(0) % 3 {
                    0 => 0,
                    2 if equal_too => case.word(1),
                    _ => xe_object(e),
                };
                xe_hold(e, x, kind, held);
                vec![w.list, case.word(1)]
            })
    }

    /// The starting position's rotation or location: (list, out, position) to read, with the three
    /// words to store for a setter.
    fn xe_start(self, setter: bool) -> Diff {
        let picks: &'static [u32] = if setter { &[0, 1, 2] } else { &[0] };
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * picks.len())
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(picks)])
            .setup(move |e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), 0x0f);
                let out = junk(e, 0x10);
                let position = xd_block(e, 0x10, 0x0f);
                let mut words = vec![w.list, out, position];
                if setter {
                    let k = case.word(1) as usize;
                    words.extend([XD_WORDS[k], XD_WORDS[k + 1], XD_WORDS[k + 3]]);
                }
                words
            })
    }

    /// `SetPackageStartLocation`: (list, location form or none, cell or none, a position, a float).
    fn xe_start_location(self) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * 4 * XD_FLOATS.len())
            .inputs(&[
                Input::Of(EXTRA_SEEDS),
                Input::Of(&[0, 1, 2, 3]),
                Input::Of(XD_FLOATS),
            ])
            .setup(|e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), 0x18);
                let pick = case.word(1);
                let form = if pick & 1 != 0 { 0x1234 } else { 0 };
                let cell = if pick & 2 != 0 { 0x5678 } else { 0 };
                let position = xd_block(e, 0x10, 0x18);
                vec![w.list, form, cell, position, case.word(2)]
            })
    }

    /// `SetStartingWorldOrCellForRef`: a reference whose `TESChildCell` answers no cell, a cell with
    /// a world space or one without, with an extra data of type 0x49 placed or not.
    fn xe_for_ref(self) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * 3)
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(&[0, 0x2000, 0x2010])])
            .setup(|e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), 0x49);
                e.mem.set_u32(XE_CELL, case.word(1));
                let reference = xe_object(e);
                let child_vtable = e.mem.alloc(0x10);
                e.mem.set_u32(child_vtable, XE_CELL_FN);
                e.mem.set_u32(reference + 0x18, child_vtable);
                vec![w.list, reference]
            })
    }

    /// `SetCanopyShadowMask`: (list, a state, a mask, where the rectangle goes).
    fn xe_canopy_set(self) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * 6)
            .inputs(&[
                Input::Of(EXTRA_SEEDS),
                Input::Of(&[0, 1, 0x8000_0000]),
                Input::Of(&[0, 0x1234]),
            ])
            .setup(|e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), 0x0a);
                let rect_out = junk(e, 4);
                vec![w.list, case.word(1), case.word(2), rect_out]
            })
    }

    /// The canopy shadow mask's reader: (list, where the mask goes, where the rectangle goes).
    fn xe_canopy_get(self) -> Diff {
        self.extra_data_case()
            .inputs(&[Input::Of(EXTRA_SEEDS)])
            .setup(|e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), 0x0a);
                vec![w.list, junk(e, 4), junk(e, 4)]
            })
    }

    /// `SetPackageExtra`: no package, or one created or not, never to run or not, of type 1, 0x17,
    /// 0x18 or another; the index, target and flags drawn.
    fn xe_package(self) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * 6)
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(&[0, 1, 2, 3, 4, 5])])
            .setup(|e, case| {
                let seed = case.word(0);
                let w = xd_world(e, seed);
                place_kind(e, &w, seed, 0x19);
                let mut d = Draw(seed.wrapping_mul(0x0041_c930) ^ case.word(1));
                let package = if case.word(1) == 0 {
                    0
                } else {
                    let package = xd_block(e, 0x40, 0x19);
                    let kind = [1u8, 0x17, 0x18, 0x42, 0x81][d.next(5) as usize];
                    e.mem.set_u8(package + 0x20, kind);
                    package
                };
                e.mem.set_u32(XE_PACKAGE_FLAGS, d.next(4));
                let index = [0, u32::MAX, 7][d.next(3) as usize];
                let target = XD_VALUES[d.next(5) as usize];
                let flags = [0, 1, 0xff].map(|_| [0, 1, 0x80][d.next(3) as usize]);
                vec![w.list, package, index, target, flags[0], flags[1], flags[2]]
            })
    }

    /// `RemovePlayerCrimeListExtra`: (list, word) with a crime list of none, or 1 to 3 nodes whose
    /// items are crimes or an empty one.
    fn xe_crimes(self) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * 2)
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(&[0, 0x77])])
            .setup(|e, case| {
                let seed = case.word(0);
                let w = xd_world(e, seed);
                let x = place_kind(e, &w, seed, 0x35);
                {
                    let mut d = Draw(seed ^ 0x0041_cf30);
                    let n = d.next(4);
                    let mut next = 0;
                    for i in 0..n {
                        let node = e.mem.alloc(8);
                        let crime = if i == 1 && d.next(2) == 0 {
                            0
                        } else {
                            xd_block(e, 0x20, i)
                        };
                        e.mem.set_u32(node, crime);
                        e.mem.set_u32(node + 4, next);
                        next = node;
                    }
                    xe_hold(e, x, 0x35, next);
                }
                vec![w.list, case.word(1)]
            })
    }

    /// The two default scans: a list of 0 to 5 extra data of the types they allow, an owner, worn,
    /// JIP's keyed data and one they do not; JIP's three sites with their handlers in the seeds
    /// with the head site's.
    fn xe_scan(self, worn: &'static [u32]) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * worn.len())
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(worn)])
            .setup(|e, case| {
                use engine_core::extend::{Owner, SiteSlot};
                let seed = case.word(0);
                let w = xd_world(e, seed);
                if w.jip >= 2 {
                    e.on_site(SiteSlot::new(Owner::Jip, 0x0041_d0f6), jip_owned_site);
                    e.on_site(SiteSlot::new(Owner::Jip, 0x0041_d212), jip_worn_site);
                    e.on_site(SiteSlot::new(Owner::Jip, 0x0041_d25a), jip_not_worn_site);
                }
                const KINDS: [u32; 13] = [
                    0x0d, 0x1c, 0x20, 0x24, 0x27, 0x2f, 0x30, 0x4a, 0x21, 0x16, 0x21, 0x27, 0x05,
                ];
                let mut d = Draw(seed.wrapping_mul(0x0041_d030) ^ 0x5ca7);
                let list = e.mem.alloc(0x20);
                e.mem.set_u32(list, 0x0101_43e8);
                let vt = e.mem.u32(XD_VT);
                let mut next = 0;
                for _ in 0..d.next(6) {
                    // The last one first: a type the scans end at in only a few seeds.
                    let kind = KINDS[d.next(if seed % 4 == 3 { 13 } else { 12 }) as usize];
                    let x = extra_data(e, vt, kind, 0);
                    e.mem.set_u32(x + 8, next);
                    next = x;
                }
                e.mem.set_u32(list + 4, next);
                vec![list, case.word(1)]
            })
    }

    /// The run-once packages: (list, flag or package) with the type 0x1b extra data holding a list
    /// of 0 to 4 entries (a package or none, a flag byte; the package's time stamp's day and hours
    /// drawn about the bound) or an empty item; the second word a flag, or a package of the list or
    /// another.
    fn xe_run_once(self, flags: bool) -> Diff {
        self.extra_data_case()
            .max_cases(EXTRA_SEEDS.len() * 5)
            .inputs(&[Input::Of(EXTRA_SEEDS), Input::Of(&[0, 1, 2, 3, 4])])
            .setup(move |e, case| xe_run_once_world(e, case.word(0), case.word(1), flags))
    }

    /// (list, the address of three words) with an extra data of `kind` placed or not.
    fn xe_with_block(self, kind: u32) -> Diff {
        self.extra_data_case()
            .inputs(&[Input::Of(EXTRA_SEEDS)])
            .setup(move |e, case| {
                let w = xd_world(e, case.word(0));
                place_kind(e, &w, case.word(0), kind);
                vec![w.list, xd_block(e, 0x10, kind)]
            })
    }

    /// A constructor's block of junk and a word.
    fn xe_block_and(self, own: u32, values: &'static [u32]) -> Diff {
        self.extra_data_case_but(own)
            .inputs(&[Input::Of(values)])
            .setup(|e, case| {
                xd_stand_ins(e);
                vec![junk(e, 0x20), case.word(0)]
            })
    }

    /// A terminal state with each pair of its two bytes.
    fn xe_terminal_bytes(self) -> Diff {
        self.extra_data_case()
            .max_cases(16)
            .inputs(&[Input::Of(XE_BYTES), Input::Of(XE_BYTES)])
            .setup(|e, case| {
                xd_stand_ins(e);
                let x = junk(e, 0x10);
                e.mem.set_u8(x + 0xc, case.word(0) as u8);
                e.mem.set_u8(x + 0xd, case.word(1) as u8);
                vec![x]
            })
    }
}

/// JIP's handlers at the scans' three sites: noted, the type ends the scan.
fn jip_owned_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    xd_site_note(e, 0x0041_d0f6, a);
    Some(1)
}

fn jip_worn_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    xd_site_note(e, 0x0041_d212, a);
    Some(1)
}

fn jip_not_worn_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    xd_site_note(e, 0x0041_d25a, a);
    Some(1)
}

#[test]
fn xe_set_activate_loop_sound() {
    Diff::new(entry!(
        0x0041_a6a0,
        extra_data::setters::set_activate_loop_sound(u32, u32)
    ))
    .xd_sound_setter(0x87)
    .run()
    .assert_same();
}

#[test]
fn xe_set_sound() {
    Diff::new(entry!(
        0x0041_a800,
        extra_data::setters::set_sound(u32, u32)
    ))
    .xd_sound_setter(0x4f)
    .run()
    .assert_same();
}

#[test]
fn xe_set_ghost() {
    Diff::new(entry!(
        0x0041_a960,
        extra_data::setters::set_ghost(u32, bool)
    ))
    .xd_setter(0x1f, XE_FLAGS)
    .run()
    .assert_same();
}

#[test]
fn xe_set_worn() {
    Diff::new(entry!(
        0x0041_aa20,
        extra_data::setters::set_worn(u32, bool, bool)
    ))
    .xe_pair(&[0x16, 0x17], XE_FLAGS, XE_FLAGS)
    .run()
    .assert_same();
}

#[test]
fn xe_set_can_not_wear() {
    Diff::new(entry!(
        0x0041_ab70,
        extra_data::setters::set_can_not_wear(u32, bool)
    ))
    .xd_setter(0x3e, XE_FLAGS)
    .run()
    .assert_same();
}

#[test]
fn xe_set_seed() {
    Diff::new(entry!(0x0041_ac30, extra_data::setters::set_seed(u32, u8)))
        .xd_setter(0x31, XE_BYTES)
        .run()
        .assert_same();
}

#[test]
fn xe_set_package_start_location() {
    Diff::new(entry!(
        0x0041_ad00,
        extra_data::setters::set_package_start_location(u32, u32, u32, u32, f32)
    ))
    .xe_start_location()
    .run()
    .assert_same();
}

#[test]
fn xe_remove_anim_ptr() {
    Diff::new(entry!(
        0x0041_adf0,
        extra_data::setters::remove_anim_ptr(u32)
    ))
    .xd_getter(0x10)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_anim_save() {
    Diff::new(entry!(
        0x0041_ae10,
        extra_data::setters::remove_anim_save(u32)
    ))
    .xd_getter(0x2d)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_light() {
    Diff::new(entry!(0x0041_ae30, extra_data::setters::remove_light(u32)))
        .xd_getter(0x29)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_spell_effect_light() {
    Diff::new(entry!(
        0x0041_ae50,
        extra_data::setters::remove_spell_effect_light(u32)
    ))
    .xd_getter(0x40)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_lock_ptr() {
    Diff::new(entry!(
        0x0041_ae70,
        extra_data::setters::remove_lock_ptr(u32)
    ))
    .xd_getter(0x2a)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_teleport_ptr() {
    Diff::new(entry!(
        0x0041_ae90,
        extra_data::setters::remove_teleport_ptr(u32)
    ))
    .xd_getter(0x2b)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_container_changes() {
    Diff::new(entry!(
        0x0041_aeb0,
        extra_data::setters::remove_container_changes(u32)
    ))
    .xd_getter(0x15)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_ownership() {
    Diff::new(entry!(
        0x0041_aed0,
        extra_data::setters::remove_ownership(u32)
    ))
    .xd_getter(0x21)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_health() {
    Diff::new(entry!(0x0041_aef0, extra_data::setters::remove_health(u32)))
        .xd_getter(0x25)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_count() {
    Diff::new(entry!(0x0041_af10, extra_data::setters::remove_count(u32)))
        .xd_getter(0x24)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_poison() {
    Diff::new(entry!(0x0041_af30, extra_data::setters::remove_poison(u32)))
        .xd_getter(0x3f)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_script() {
    Diff::new(entry!(0x0041_af50, extra_data::setters::remove_script(u32)))
        .xd_getter(0x0d)
        .run()
        .assert_same();
}

#[test]
fn xe_clear_script_locals() {
    Diff::new(entry!(
        0x0041_afa0,
        extra_data::setters::clear_script_locals(u32)
    ))
    .xd_getter(0x0d)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_scale() {
    Diff::new(entry!(0x0041_afd0, extra_data::setters::remove_scale(u32)))
        .xd_getter(0x30)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_ghost() {
    Diff::new(entry!(0x0041_aff0, extra_data::setters::remove_ghost(u32)))
        .xd_getter(0x1f)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_worn() {
    Diff::new(entry!(
        0x0041_b010,
        extra_data::setters::remove_worn(u32, bool)
    ))
    .xe_pair(&[0x16, 0x17], XE_FLAGS, &[0])
    .run()
    .assert_same();
}

#[test]
fn xe_remove_can_not_wear() {
    Diff::new(entry!(
        0x0041_b040,
        extra_data::setters::remove_can_not_wear(u32)
    ))
    .xd_getter(0x3e)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_package_start_location() {
    Diff::new(entry!(
        0x0041_b060,
        extra_data::setters::remove_package_start_location(u32)
    ))
    .xd_getter(0x18)
    .run()
    .assert_same();
}

#[test]
fn xe_starting_rotation() {
    Diff::new(entry!(0x0041_b080, extra_data::setters::starting_rotation(u32, u32, u32) -> u32))
        .xe_start(false)
        .run()
        .assert_same();
}

#[test]
fn xe_starting_location() {
    Diff::new(entry!(0x0041_b0d0, extra_data::setters::starting_location(u32, u32, u32) -> u32))
        .xe_start(false)
        .run()
        .assert_same();
}

#[test]
fn xe_set_starting_rotation() {
    Diff::new(entry!(
        0x0041_b120,
        extra_data::setters::set_starting_rotation(u32, u32, u32, u32, u32, u32) -> u32
    ))
    .xe_start(true)
    .run()
    .assert_same();
}

#[test]
fn xe_set_starting_location() {
    Diff::new(entry!(
        0x0041_b180,
        extra_data::setters::set_starting_location(u32, u32, u32, u32, u32, u32) -> u32
    ))
    .xe_start(true)
    .run()
    .assert_same();
}

#[test]
fn xe_set_starting_world_or_cell() {
    Diff::new(entry!(
        0x0041_b1e0,
        extra_data::setters::set_starting_world_or_cell(u32, u32)
    ))
    .xd_setter(0x49, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_set_starting_world_or_cell_for_ref() {
    Diff::new(entry!(
        0x0041_b2a0,
        extra_data::setters::set_starting_world_or_cell_for_ref(u32, u32)
    ))
    .xe_for_ref()
    .run()
    .assert_same();
}

#[test]
fn xe_starting_world_or_cell() {
    Diff::new(entry!(0x0041_b320, extra_data::setters::starting_world_or_cell(u32) -> u32))
        .xd_getter(0x49)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_starting_world_or_cell() {
    Diff::new(entry!(
        0x0041_b350,
        extra_data::setters::remove_starting_world_or_cell(u32)
    ))
    .xd_getter(0x49)
    .run()
    .assert_same();
}

#[test]
fn xe_action_flags() {
    Diff::new(entry!(0x0041_b370, extra_data::setters::action_flags(u32) -> u32))
        .xd_getter(0x0e)
        .run()
        .assert_same();
}

#[test]
fn xe_has_action_flag() {
    Diff::new(entry!(0x0041_b3a0, extra_data::setters::has_action_flag(u32, u32) -> bool))
        .xd_setter(0x0e, XE_MASKS)
        .run()
        .assert_same();
}

#[test]
fn xe_set_action_flags() {
    Diff::new(entry!(
        0x0041_b3d0,
        extra_data::setters::set_action_flags(u32, u32)
    ))
    .xd_setter(0x0e, XE_ACTIONS)
    .run()
    .assert_same();
}

#[test]
fn xe_add_action_flags() {
    Diff::new(entry!(
        0x0041_b440,
        extra_data::setters::add_action_flags(u32, u32)
    ))
    .xd_setter(0x0e, XE_MASKS)
    .run()
    .assert_same();
}

#[test]
fn xe_remove_action_flags() {
    Diff::new(entry!(
        0x0041_b470,
        extra_data::setters::remove_action_flags(u32, u32)
    ))
    .xd_setter(0x0e, XE_MASKS)
    .run()
    .assert_same();
}

#[test]
fn xe_set_action_ref() {
    Diff::new(entry!(
        0x0041_b4b0,
        extra_data::setters::set_action_ref(u32, u32)
    ))
    .xd_setter(0x0e, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_action_ref() {
    Diff::new(entry!(0x0041_b520, extra_data::setters::action_ref(u32) -> u32))
        .xd_getter(0x0e)
        .run()
        .assert_same();
}

#[test]
fn xe_health_perc() {
    Diff::new(entry!(0x0041_b550, extra_data::setters::health_perc(u32) -> f32))
        .xd_getter(0x7a)
        .run()
        .assert_same();
}

#[test]
fn xe_set_health_perc() {
    Diff::new(entry!(
        0x0041_b580,
        extra_data::setters::set_health_perc(u32, f32)
    ))
    .xd_setter(0x7a, XD_FLOATS)
    .run()
    .assert_same();
}

#[test]
fn xe_health_perc_new() {
    Diff::new(entry!(0x0041_b650, extra_data::setters::health_perc_new(u32) -> u32))
        .xd_on_block(0x20, 0x0041_b650)
        .run()
        .assert_same();
}

#[test]
fn xe_health_perc_delete() {
    Diff::new(entry!(0x0041_b680, extra_data::setters::health_perc_delete(u32, u32) -> u32))
        .xd_data_delete()
        .run()
        .assert_same();
}

#[test]
fn xe_object_health() {
    Diff::new(entry!(0x0041_b6b0, extra_data::setters::object_health(u32) -> f32))
        .xd_getter(0x56)
        .run()
        .assert_same();
}

#[test]
fn xe_set_object_health() {
    Diff::new(entry!(
        0x0041_b6e0,
        extra_data::setters::set_object_health(u32, f32)
    ))
    .xd_setter(0x56, XD_FLOATS)
    .run()
    .assert_same();
}

#[test]
fn xe_object_health_new() {
    Diff::new(entry!(0x0041_b790, extra_data::setters::object_health_new(u32, f32) -> u32))
        .xe_block_and(0x0041_b790, XD_FLOATS)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_object_health() {
    Diff::new(entry!(
        0x0041_b7c0,
        extra_data::setters::remove_object_health(u32)
    ))
    .xd_getter(0x56)
    .run()
    .assert_same();
}

#[test]
fn xe_set_cell_3d() {
    Diff::new(entry!(
        0x0041_b7e0,
        extra_data::cell_data::set_cell_3d(u32, u32)
    ))
    .xd_setter(0x02, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_set_havok_world() {
    Diff::new(entry!(
        0x0041_b8d0,
        extra_data::cell_data::set_havok_world(u32, u32)
    ))
    .xd_setter(0x01, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_havok_world() {
    Diff::new(entry!(0x0041_b9a0, extra_data::cell_data::havok_world(u32) -> u32))
        .xd_getter(0x01)
        .run()
        .assert_same();
}

#[test]
fn xe_set_cell_mopp() {
    Diff::new(entry!(
        0x0041_ba60,
        extra_data::cell_data::set_cell_mopp(u32, u32)
    ))
    .xd_setter(0x01, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_cell_mopp() {
    Diff::new(entry!(0x0041_bb10, extra_data::cell_data::cell_mopp(u32) -> u32))
        .xd_getter(0x01)
        .run()
        .assert_same();
}

#[test]
fn xe_set_region_list() {
    Diff::new(entry!(
        0x0041_bbd0,
        extra_data::cell_data::set_region_list(u32, u32)
    ))
    .xe_owned(0x04, true)
    .run()
    .assert_same();
}

#[test]
fn xe_region_list() {
    Diff::new(entry!(0x0041_bce0, extra_data::cell_data::region_list(u32) -> u32))
        .xd_getter(0x04)
        .run()
        .assert_same();
}

#[test]
fn xe_set_music_type() {
    Diff::new(entry!(
        0x0041_bd10,
        extra_data::cell_data::set_music_type(u32, u32)
    ))
    .xd_setter(0x07, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_music_type() {
    Diff::new(entry!(0x0041_bde0, extra_data::cell_data::music_type(u32) -> u32))
        .xd_getter(0x07)
        .run()
        .assert_same();
}

#[test]
fn xe_set_terminal_state() {
    Diff::new(entry!(
        0x0041_be10,
        extra_data::cell_data::set_terminal_state(u32, u8)
    ))
    .xd_setter(0x50, XE_BYTES)
    .run()
    .assert_same();
}

#[test]
fn xe_terminal_state_new() {
    Diff::new(entry!(0x0041_bef0, extra_data::cell_data::terminal_state_new(u32) -> u32))
        .xd_on_block(0x20, 0x0041_bef0)
        .run()
        .assert_same();
}

#[test]
fn xe_terminal_state_empty() {
    Diff::new(entry!(0x0041_bf20, extra_data::cell_data::terminal_state_empty(u32) -> bool))
        .xe_terminal_bytes()
        .run()
        .assert_same();
}

#[test]
fn xe_terminal_state() {
    Diff::new(entry!(0x0041_bf50, extra_data::cell_data::terminal_state(u32) -> u8))
        .xd_getter(0x50)
        .run()
        .assert_same();
}

#[test]
fn xe_set_lock_override() {
    Diff::new(entry!(
        0x0041_bf80,
        extra_data::cell_data::set_lock_override(u32, u8)
    ))
    .xd_setter(0x50, XE_BYTES)
    .run()
    .assert_same();
}

#[test]
fn xe_lock_override() {
    Diff::new(entry!(0x0041_c060, extra_data::cell_data::lock_override(u32) -> u8))
        .xd_getter(0x50)
        .run()
        .assert_same();
}

#[test]
fn xe_set_acoustic_space() {
    Diff::new(entry!(
        0x0041_c090,
        extra_data::cell_data::set_acoustic_space(u32, u32)
    ))
    .xd_setter(0x81, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_acoustic_space() {
    Diff::new(entry!(0x0041_c160, extra_data::cell_data::acoustic_space(u32) -> u32))
        .xd_getter(0x81)
        .run()
        .assert_same();
}

#[test]
fn xe_set_climate() {
    Diff::new(entry!(
        0x0041_c190,
        extra_data::cell_data::set_climate(u32, u32)
    ))
    .xd_setter(0x08, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_climate() {
    Diff::new(entry!(0x0041_c260, extra_data::cell_data::climate(u32) -> u32))
        .xd_getter(0x08)
        .run()
        .assert_same();
}

#[test]
fn xe_set_image_space() {
    Diff::new(entry!(
        0x0041_c290,
        extra_data::cell_data::set_image_space(u32, u32)
    ))
    .xd_setter(0x59, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_image_space() {
    Diff::new(entry!(0x0041_c360, extra_data::cell_data::image_space(u32) -> u32))
        .xd_getter(0x59)
        .run()
        .assert_same();
}

#[test]
fn xe_set_impact_swap() {
    Diff::new(entry!(
        0x0041_c390,
        extra_data::cell_data::set_impact_swap(u32, u32)
    ))
    .xd_setter(0x8c, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_impact_swap() {
    Diff::new(entry!(0x0041_c460, extra_data::cell_data::impact_swap(u32) -> u32))
        .xd_getter(0x8c)
        .run()
        .assert_same();
}

#[test]
fn xe_set_canopy_shadow_mask() {
    Diff::new(entry!(
        0x0041_c490,
        extra_data::cell_data::set_canopy_shadow_mask(u32, u32, u32, u32)
    ))
    .xe_canopy_set()
    .run()
    .assert_same();
}

#[test]
fn xe_canopy_shadow_mask() {
    Diff::new(entry!(0x0041_c580, extra_data::cell_data::canopy_shadow_mask(u32, u32, u32) -> u32))
        .xe_canopy_get()
        .run()
        .assert_same();
}

#[test]
fn xe_editor_id_new() {
    Diff::new(entry!(0x0041_c5e0, extra_data::cell_data::editor_id_new(u32, u32) -> u32))
        .xe_block_and(0x0041_c5e0, XD_VALUES)
        .run()
        .assert_same();
}

#[test]
fn xe_editor_id_delete() {
    Diff::new(entry!(0x0041_c660, extra_data::cell_data::editor_id_delete(u32, u32) -> u32))
        .xd_data_delete()
        .run()
        .assert_same();
}

#[test]
fn xe_editor_id_drop() {
    Diff::new(entry!(
        0x0041_c690,
        extra_data::cell_data::editor_id_drop(u32)
    ))
    .xd_on_block(0x20, 0x0041_c690)
    .run()
    .assert_same();
}

#[test]
fn xe_set_editor_id() {
    Diff::new(entry!(
        0x0041_c6f0,
        extra_data::cell_data::set_editor_id(u32, u32)
    ))
    .xd_setter(0x06, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_editor_id_text() {
    Diff::new(entry!(0x0041_c7c0, extra_data::cell_data::editor_id_text(u32) -> u32))
        .xd_getter(0x06)
        .run()
        .assert_same();
}

#[test]
fn xe_set_reference_pointer() {
    Diff::new(entry!(
        0x0041_c7f0,
        extra_data::cell_data::set_reference_pointer(u32, u32)
    ))
    .xd_setter(0x1c, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_reference_pointer() {
    Diff::new(entry!(0x0041_c8d0, extra_data::cell_data::reference_pointer(u32) -> u32))
        .xd_getter(0x1c)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_reference_pointer() {
    Diff::new(entry!(
        0x0041_c900,
        extra_data::cell_data::remove_reference_pointer(u32)
    ))
    .xd_getter(0x1c)
    .run()
    .assert_same();
}

#[test]
fn xe_set_package() {
    Diff::new(entry!(
        0x0041_c930,
        extra_data::packages::set_package(u32, u32, i32, u32, u8, u8, u8)
    ))
    .xe_package()
    .run()
    .assert_same();
}

#[test]
fn xe_set_package_target() {
    Diff::new(entry!(
        0x0041_cab0,
        extra_data::packages::set_package_target(u32, u32)
    ))
    .xd_setter(0x19, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_set_package_index() {
    Diff::new(entry!(
        0x0041_cae0,
        extra_data::packages::set_package_index(u32, i32)
    ))
    .xd_setter(0x19, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_package() {
    Diff::new(entry!(0x0041_cb10, extra_data::packages::package(u32) -> u32))
        .xd_getter(0x19)
        .run()
        .assert_same();
}

#[test]
fn xe_package_index() {
    Diff::new(entry!(0x0041_cb40, extra_data::packages::package_index(u32) -> i32))
        .xd_getter(0x19)
        .run()
        .assert_same();
}

#[test]
fn xe_package_action_complete() {
    Diff::new(entry!(0x0041_cba0, extra_data::packages::package_action_complete(u32) -> u8))
        .xd_getter(0x19)
        .run()
        .assert_same();
}

#[test]
fn xe_set_package_action_complete() {
    Diff::new(entry!(
        0x0041_cbd0,
        extra_data::packages::set_package_action_complete(u32, u8)
    ))
    .xd_setter(0x19, XE_BYTES)
    .run()
    .assert_same();
}

#[test]
fn xe_package_activated() {
    Diff::new(entry!(0x0041_cc00, extra_data::packages::package_activated(u32) -> u8))
        .xd_getter(0x19)
        .run()
        .assert_same();
}

#[test]
fn xe_package_done_once() {
    Diff::new(entry!(0x0041_cc30, extra_data::packages::package_done_once(u32) -> u8))
        .xd_getter(0x19)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_package() {
    Diff::new(entry!(
        0x0041_cc60,
        extra_data::packages::remove_package(u32)
    ))
    .xd_getter(0x19)
    .run()
    .assert_same();
}

#[test]
fn xe_set_trespass_package() {
    Diff::new(entry!(
        0x0041_cc90,
        extra_data::packages::set_trespass_package(u32, u32)
    ))
    .xe_owned(0x1a, false)
    .run()
    .assert_same();
}

#[test]
fn xe_trespass_package() {
    Diff::new(entry!(0x0041_cd70, extra_data::packages::trespass_package(u32) -> u32))
        .xd_getter(0x1a)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_trespass_package() {
    Diff::new(entry!(
        0x0041_cda0,
        extra_data::packages::remove_trespass_package(u32)
    ))
    .xd_getter(0x1a)
    .run()
    .assert_same();
}

#[test]
fn xe_forget_trespass_package() {
    Diff::new(entry!(
        0x0041_ce10,
        extra_data::packages::forget_trespass_package(u32, u32)
    ))
    .xd_setter(0x1a, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_add_to_player_crime_list() {
    Diff::new(entry!(
        0x0041_ce50,
        extra_data::packages::add_to_player_crime_list(u32, u32)
    ))
    .xd_setter(0x35, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_player_crime_list() {
    Diff::new(entry!(0x0041_cf00, extra_data::packages::player_crime_list(u32) -> u32))
        .xd_getter(0x35)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_player_crime_list() {
    Diff::new(entry!(
        0x0041_cf30,
        extra_data::packages::remove_player_crime_list(u32, u32)
    ))
    .xe_crimes()
    .run()
    .assert_same();
}

#[test]
fn xe_remove_leveled_item() {
    Diff::new(entry!(
        0x0041_d000,
        extra_data::packages::remove_leveled_item(u32)
    ))
    .xd_getter(0x2f)
    .run()
    .assert_same();
}

#[test]
fn xe_only_owner_defaults() {
    Diff::new(entry!(0x0041_d030, extra_data::packages::only_owner_defaults(u32) -> bool))
        .xe_scan(&[0])
        .run()
        .assert_same();
}

#[test]
fn xe_is_default_for_container() {
    Diff::new(entry!(0x0041_d120, extra_data::packages::is_default_for_container(u32, u8) -> bool))
        .xe_scan(&[0, 1, 2])
        .run()
        .assert_same();
}

#[test]
fn xe_set_leveled_item_index() {
    Diff::new(entry!(
        0x0041_d280,
        extra_data::packages::set_leveled_item_index(u32, i32)
    ))
    .xd_setter(0x2f, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_set_leveled_item_default() {
    Diff::new(entry!(
        0x0041_d330,
        extra_data::packages::set_leveled_item_default(u32, u8)
    ))
    .xd_setter(0x2f, XE_BYTES)
    .run()
    .assert_same();
}

#[test]
fn xe_leveled_item_index() {
    Diff::new(entry!(0x0041_d360, extra_data::packages::leveled_item_index(u32) -> i32))
        .xd_getter(0x2f)
        .run()
        .assert_same();
}

#[test]
fn xe_set_persistent_cell() {
    Diff::new(entry!(
        0x0041_d390,
        extra_data::packages::set_persistent_cell(u32, u32)
    ))
    .xd_setter(0x0c, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_persistent_cell() {
    Diff::new(entry!(0x0041_d460, extra_data::packages::persistent_cell(u32) -> u32))
        .xd_getter(0x0c)
        .run()
        .assert_same();
}

#[test]
fn xe_set_rag_doll_data() {
    Diff::new(entry!(
        0x0041_d490,
        extra_data::packages::set_rag_doll_data(u32, u32)
    ))
    .xd_setter(0x14, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_set_rag_doll_data_from_copy() {
    Diff::new(entry!(
        0x0041_d5b0,
        extra_data::packages::set_rag_doll_data_from_copy(u32, u32)
    ))
    .xd_setter(0x14, XD_VALUES)
    .run()
    .assert_same();
}

#[test]
fn xe_rag_doll_data() {
    Diff::new(entry!(0x0041_d6d0, extra_data::packages::rag_doll_data(u32) -> u32))
        .xd_getter(0x14)
        .run()
        .assert_same();
}

#[test]
fn xe_add_run_once_package() {
    Diff::new(entry!(
        0x0041_d700,
        extra_data::packages::add_run_once_package(u32, u32, u8)
    ))
    .xe_pair(&[0x1b], XD_VALUES, XE_BYTES)
    .run()
    .assert_same();
}

#[test]
fn xe_prune_run_once_packages() {
    Diff::new(entry!(
        0x0041_d7b0,
        extra_data::packages::prune_run_once_packages(u32, i8)
    ))
    .xe_run_once(true)
    .run()
    .assert_same();
}

#[test]
fn xe_package_time_stamp() {
    Diff::new(entry!(0x0041_d8a0, extra_data::packages::package_time_stamp(u32) -> u32))
        .xd_on_pattern(0x40, 0x0041_d8a0)
        .run()
        .assert_same();
}

#[test]
fn xe_is_in_run_once_packages() {
    Diff::new(entry!(0x0041_d8c0, extra_data::packages::is_in_run_once_packages(u32, u32) -> bool))
        .xe_run_once(false)
        .run()
        .assert_same();
}

#[test]
fn xe_remove_run_once_packages() {
    Diff::new(entry!(
        0x0041_d930,
        extra_data::packages::remove_run_once_packages(u32)
    ))
    .xd_getter(0x1b)
    .run()
    .assert_same();
}

#[test]
fn xe_set_land_normal() {
    Diff::new(entry!(
        0x0041_d950,
        extra_data::packages::set_land_normal(u32, u32)
    ))
    .xe_with_block(0x13)
    .run()
    .assert_same();
}

#[test]
fn xe_enable_state_parent() {
    Diff::new(entry!(0x0041_da10, extra_data::packages::enable_state_parent(u32) -> u32))
        .xd_getter(0x37)
        .run()
        .assert_same();
}

#[test]
fn xe_set_enable_state_parent() {
    Diff::new(entry!(
        0x0041_da40,
        extra_data::packages::set_enable_state_parent(u32, u32)
    ))
    .xd_setter(0x37, XD_VALUES)
    .run()
    .assert_same();
}
