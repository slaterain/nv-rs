//! The differential test of every typed function of the cell references family
//! (`engine_typed::cell_refs`) against its translation.
//!
//! The world of `0054bcf0`: a cell whose list of references is built from a seed: none to seven
//! references (now and then a null item), each with a base form (none, one of the two marker forms,
//! or an ordinary one of type 0x23, 0x1e or 0x15 with or without its flag `0x40000000`), form flags
//! `0x800` / `0x20` or none, an extra list whose `ScriptLocals` are absent, fresh or already built, a
//! 3D or none, and one of two vtables: `TESObjectREFR`'s own slots (the exe functions at 0x100, 0x224
//! and 0x1d0, so the typed side's direct branches run) or stand-ins that answer words kept in the
//! reference. The seed also sets the game loader's flag, whether `TES` accepts the cell and whether
//! the cell has a 3D.
//!
//! What stands in: the work the pass hands on (`Load3D`, the cell's scripts and the steps after them,
//! the lock, `TES`'s test and priority, `SetActionFlag`, the cast, the process lists, the model
//! loader's queue, the attach, the cell's state) and `Script::InitActionList`'s callees.

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
const DOUBLES: [u32; 20] = [
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
    0x0044_4850, // ModelLoader::QueueReference
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

/// The doubles that count their calls: double `K` counts at `COUNTS + 4 * K` and keeps the last word
/// it was handed (its first past `this` when it has one) at `COUNTS + 0x40 + 4 * K`, so what it was
/// handed shows in memory as well as in the call log.
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
    e.register(0x0044_4850, queue_reference);
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
            let kind = [0x23, 0x23, 0x1e, 0x15][d.next(4) as usize];
            base_form(e, kind, [0, 0x4000_0000][d.next(2) as usize])
        }
    };
    e.mem.set_u32(r + 0x20, base);
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
        .set_u32(loader + 0x244, [0, 0, 1, 2, 3][d.next(5) as usize]);
    e.set_global(GAME_LOADER_POINTER, loader);
    e.set_global(TES_POINTER, 0x7000u32);
    e.set_global(SAVE_GAME_POINTER, 0x7100u32);
    e.set_global(MODEL_LOADER_POINTER, 0x7200u32);
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

/// The worlds the seeds make reach every branch the pass has, checked on the translation (so a change
/// to the generator that loses one shows here, not as a weaker comparison): a cell `TES` accepts,
/// references queued, attached, given their action list and flag, actors' and the other kind's steps,
/// and a list whose last reference is brought up in both walks.
#[test]
fn the_seeds_reach_every_branch() {
    let mut reached = [0u32; 6];
    let (mut accepted, mut twice) = (0, 0);
    for &seed in SEEDS {
        let mut e = Engine::new_untyped_for_test();
        let cell = world(&mut e, seed);
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
    }
    assert!(accepted > 0, "no seed has TES accept the cell");
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
