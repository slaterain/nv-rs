//! The differential test of the after-draw stage's per-actor loops (`engine_typed::process::after_draw`)
//! and of an actor's postponed scene graph update (`engine_typed::animation::queued`) against their
//! translations. The world: the process lists with their process array (at +4), the objects of level 0
//! (projectiles, explosions, characters, the null slot, skipped forms), each with its own vtable whose
//! slots log and answer from the object's own words; the characters' processes with their detection
//! lists (real `BSSimpleList` nodes; the list's append and clear and the state's constructor and copy
//! run translated, `operator new` and `delete` are stand-ins over the test heap); the animations with
//! their roots, the controllers and the pending bit.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::process::{self, after_draw};
use engine_typed::{animation, dispatch, scene};

const LOG: u32 = 0x0300_0000;
/// Where the stand-in slot functions live, one per slot.
const SLOT_FN: u32 = 0x00ff_4000;
const PLAYER: u32 = 0x011d_ea3c;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    if n >= 0x700 {
        return;
    }
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

/// A slot stand-in: logs `this` and its words under the slot, answers the object's word at
/// `+0x100 + slot / 4 % 64 * 4` (each object says what each slot answers).
fn slot<const SLOT: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a[0];
    log(e, 0x1_0000 | SLOT, this);
    for w in &a[1..] {
        log(e, 0x2_0000 | SLOT, *w);
    }
    ret(e.mem.u32(this + 0x100 + (SLOT / 4 % 64) * 4))
}

/// The slots the loops call.
const SLOTS: [u32; 16] = [
    0x100, 0x104, 0x170, 0x1d0, 0x1d8, 0x220, 0x224, 0x22c, 0x314, 0x34c, 0x428, 0x500, 0x504,
    0xa4, 0x0, 0x10,
];

/// A slot +0x34c stand-in that also clears the actor's process word (+0x68).
const CLEARS_PROCESS: u32 = SLOT_FN + 0x800;

fn register_slots(e: &mut Engine) {
    macro_rules! each {
        ($($s:literal)*) => { $( e.register(SLOT_FN + $s, slot::<$s>); )* };
    }
    each!(0x100 0x104 0x170 0x1d0 0x1d8 0x220 0x224 0x22c 0x314 0x34c 0x428 0x500 0x504 0xa4 0x0 0x10);
    e.register(CLEARS_PROCESS, |e, a| {
        let r = slot::<0x34c>(e, a);
        e.mem.set_u32(a[0] + 0x68, 0);
        r
    });
}

/// An object (0x2c0 bytes, answers from +0x100) with its own vtable; `real` names slots that hold an
/// exe function instead of the stand-in.
fn object(e: &mut Engine, real: &[(u32, u32)]) -> u32 {
    let vt = e.mem.alloc(0x600);
    for s in SLOTS {
        e.mem.set_u32(vt + s, SLOT_FN + s);
    }
    for &(s, f) in real {
        e.mem.set_u32(vt + s, f);
    }
    let o = e.mem.alloc(0x2c0);
    e.mem.set_u32(o, vt);
    o
}

/// Sets what `slot` of `o` answers.
fn answer(e: &mut Engine, o: u32, slot: u32, value: u32) {
    e.mem.set_u32(o + 0x100 + (slot / 4 % 64) * 4, value);
}

fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, TAG | 0x1000_0000, *w);
    }
    ret(e.mem.u32(LOG + 0x3f00 + 4 * (TAG & 0x3f)))
}

fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [
        0x011d_e000,
        0x011c_3000,
        0x011e_0000,
        0x011d_d000,
        0x0108_7000,
        0x0106_7000,
        0x011f_4000,
    ] {
        e.map(page, 0x1000);
    }
    // `FLT_MAX` as the detection state's copy and constructor read it (`01087820`, `010673a0`).
    e.mem.set_u32(0x0108_7820, 0x7f7f_ffff);
    e.mem.set_u32(0x0106_73a0, 0x7f7f_ffff);
    register_slots(e);
    // `operator new` and `delete` over the test heap.
    e.register(0x0040_1000, |e, a| {
        log(e, 0x1000, a[0]);
        ret(e.mem.alloc(a[0].max(4)))
    });
    e.register(0x0040_1030, logged::<0x01>);
    // The reference counts' interlocked imports.
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
    for (addr, f) in [
        (
            0x009a_e6e0u32,
            logged::<0x02> as fn(&mut Engine, &[u32]) -> Ret,
        ),
        (0x008b_a600, logged::<0x03>),
        (0x0093_1ed0, logged::<0x04>),
        (0x004a_3a20, logged::<0x05>),
        (0x00ca_2ad0, logged::<0x06>),
        (0x00ca_1410, logged::<0x07>),
        (0x0062_c430, logged::<0x08>),
        (0x0094_df60, logged::<0x09>),
        (0x009b_17a0, logged::<0x0a>),
        (0x0097_8890, logged::<0x0b>),
        (0x0042_ce10, logged::<0x0c>),
    ] {
        e.register(addr, f);
    }
}

fn set_logged(e: &mut Engine, tag: u32, value: u32) {
    e.mem.set_u32(LOG + 0x3f00 + 4 * (tag & 0x3f), value);
}

/// The process lists with level 0 holding `objects` (and an object of level 1 after them).
fn lists(e: &mut Engine, objects: &[u32]) -> u32 {
    let other = object(e, &[]);
    let n = objects.len() as u32 + 1;
    let elements = e.mem.alloc(4 * n);
    for (i, &o) in objects.iter().enumerate() {
        e.mem.set_u32(elements + 4 * i as u32, o);
    }
    e.mem.set_u32(elements + 4 * (n - 1), other);
    let lists = e.mem.alloc(0x100);
    let array = lists + 4;
    e.mem.set_u32(array + 4, elements);
    e.mem.set_u16(array + 8, n as u16);
    e.mem.set_u16(array + 0xa, n as u16);
    e.mem.set_u16(array + 0xc, n as u16);
    let heads = [0, n - 1, n, n];
    let tails = [n - 1, n, n, n];
    for level in 0..4u32 {
        e.mem
            .set_u32(array + 0x10 + 4 * level, heads[level as usize]);
        e.mem
            .set_u32(array + 0x20 + 4 * level, tails[level as usize]);
    }
    lists
}

/// A level-0 object of `kind` (pattern `k`): 0 null; 1..3 form flags 0x200000, 0x20, 0x800;
/// otherwise an object whose slots answer from `kind`'s bits.
fn level0(e: &mut Engine, kind: u32) -> u32 {
    if kind == 0 {
        return 0;
    }
    let o = object(e, &[]);
    let flags = match kind {
        1 => 0x0020_0000,
        2 => 0x20,
        3 => 0x800,
        _ => 0,
    };
    e.mem.set_u32(o + 8, flags);
    let bits = kind.saturating_sub(4);
    answer(e, o, 0x224, bits & 1);
    answer(e, o, 0x220, (bits >> 1) & 1);
    answer(e, o, 0x100, (bits >> 2) & 1);
    answer(e, o, 0x22c, (bits >> 3) & 1);
    o
}

/// The families every test of this file runs with (a method, so that each test is one chain from
/// `Diff::new` to `.assert_same()`).
trait Common {
    fn common(self) -> Self;
    fn animation_common(self) -> Self;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&process::PROCESS)
            .family(&dispatch::DISPATCH)
            .family(&leaf::LEAVES)
            .max_calls(50_000)
    }

    fn animation_common(self) -> Diff {
        self.common()
            .family(&animation::ANIMATION)
            .family(&scene::SCENE)
            .family(&engine_typed::multibound::MULTIBOUND)
    }
}

#[test]
fn after_draw_projectiles_step() {
    Diff::new(entry!(
        0x0096_c240,
        after_draw::projectiles_step(Ptr<_>, u32)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2, 3])])
    .setup(|e, case: &Case| {
        stand_ins(e);
        let objects: Vec<u32> = (0..20)
            .map(|i| level0(e, (i * 3 + case.word(0)) % 12))
            .collect();
        vec![lists(e, &objects), 0x55]
    })
    .run()
    .assert_same();
}

/// A list of `items` (each a pointer), as `BSSimpleList` nodes: the head node holds the first.
fn list(e: &mut Engine, items: &[u32]) -> u32 {
    let head = e.mem.alloc(8);
    let mut node = head;
    for (i, &item) in items.iter().enumerate() {
        e.mem.set_u32(node, item);
        if i + 1 < items.len() {
            let next = e.mem.alloc(8);
            e.mem.set_u32(node + 4, next);
            node = next;
        }
    }
    head
}

/// A detection state (0x24 bytes) for `actor`, its fields from the pattern.
fn state(e: &mut Engine, actor: u32, k: u32) -> u32 {
    let s = e.mem.alloc(0x24);
    e.mem.set_u32(s, actor);
    for w in 1..9 {
        e.mem.set_u32(s + 4 * w, 0x100 * k + w);
    }
    s
}

/// A character with its process: the process's detection lists from the pattern (some actors
/// already known, some new, some lists empty); the process's slot +0x504 the exe's lookup or a
/// stand-in.
fn character(e: &mut Engine, k: u32, others: &[u32]) -> u32 {
    let real = k % 3 != 2;
    let process = object(e, if real { &[(0x504, 0x008f_6650)] } else { &[] });
    let actor = object(e, &[]);
    answer(e, actor, 0x100, 1);
    e.mem.set_u32(actor + 0x68, process);
    let pick = |i: u32| others[((i * 5 + k) as usize) % others.len()];
    let known: Vec<u32> = (0..(k % 4)).map(|i| state(e, pick(i), k)).collect();
    let thread: Vec<u32> = (0..((k + 1) % 4))
        .map(|i| state(e, pick(i + 1), k + 7))
        .collect();
    let who: Vec<u32> = (0..(k % 3)).map(|i| state(e, pick(i + 2), k)).collect();
    let temp: Vec<u32> = (0..((k + 2) % 3))
        .map(|i| state(e, pick(i), k + 9))
        .collect();
    let lists = [(0x25c, known), (0x260, who), (0x268, thread), (0x26c, temp)];
    for (offset, items) in lists {
        let l = list(e, &items);
        e.mem.set_u32(process + offset, l);
    }
    // The package of type 0x12 (slot +0x428): none, or one (the flag store writes its +0xc5).
    let package = if k % 2 == 0 { 0 } else { e.mem.alloc(0x100) };
    answer(e, actor, 0x428, package);
    // The stand-in lookup answers the first known state (or none).
    if !real {
        let first = e.mem.u32(e.mem.u32(process + 0x25c));
        answer(e, process, 0x504, first);
    }
    actor
}

#[test]
fn after_draw_merge_detection_lists() {
    Diff::new(entry!(
        0x008f_63e0,
        after_draw::merge_detection_lists(Ptr<_>, Ptr<_>)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])])
    .setup(|e, case: &Case| {
        stand_ins(e);
        let others: Vec<u32> = (0..6).map(|_| object(e, &[])).collect();
        let actor = character(e, case.word(0), &others);
        vec![e.mem.u32(actor + 0x68), actor]
    })
    .run()
    .assert_same();
}

#[test]
fn after_draw_merge_detection() {
    Diff::new(entry!(0x0096_c710, after_draw::merge_detection(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4]), Input::Flag])
        .setup(|e, case: &Case| {
            stand_ins(e);
            let others: Vec<u32> = (0..6).map(|_| object(e, &[])).collect();
            let objects: Vec<u32> = (0..9)
                .map(|i| {
                    if i == 4 {
                        0
                    } else if (i + case.word(0)) % 4 == 1 {
                        // Not an actor (slot +0x100 answers 0) and no process at +0x68.
                        object(e, &[])
                    } else {
                        character(e, i + case.word(0), &others)
                    }
                })
                .collect();
            // The player: with or without a process.
            let player = if case.word(1) == 1 {
                character(e, 5 + case.word(0), &others)
            } else {
                object(e, &[])
            };
            e.mem.set_u32(PLAYER, player);
            vec![lists(e, &objects)]
        })
        .run()
        .assert_same();
}

#[test]
fn after_draw_ragdoll_step() {
    Diff::new(entry!(0x0097_81d0, after_draw::ragdoll_step(Ptr<_>, f32)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3]), Input::Flag, Input::Flag])
        .setup(|e, case: &Case| {
            stand_ins(e);
            let objects: Vec<u32> = (0..16)
                .map(|i| {
                    let o = level0(e, (i * 5 + case.word(0)) % 16);
                    if o != 0 {
                        // A ragdoll object at +0xb0 whose flag byte +0x18 says whether it updates;
                        // a process at +0x68 or none.
                        let ragdoll = e.mem.alloc(0x40);
                        e.mem.set_u8(ragdoll + 0x18, (i % 2) as u8);
                        e.mem.set_u32(o + 0xb0, ragdoll);
                        let p = object(e, &[]);
                        e.mem.set_u32(o + 0x68, p);
                        // Some actors' frame step (slot +0x34c) clears their process word, so the
                        // second process read finds none.
                        if i % 3 == 0 {
                            let vt = e.mem.u32(o);
                            e.mem.set_u32(vt + 0x34c, CLEARS_PROCESS);
                        }
                    }
                    o
                })
                .collect();
            let player = object(e, &[]);
            if case.word(1) == 1 {
                let p = object(e, &[]);
                e.mem.set_u32(player + 0x68, p);
            }
            e.mem.set_u32(PLAYER, player);
            set_logged(e, 0x09, case.word(2));
            vec![lists(e, &objects), 0x3e80_0000]
        })
        .run()
        .assert_same();
}

/// An animation with its roots (stand-in nodes whose downward slot logs), the pending bit, the limit
/// setting, the forcing byte and the flag object's answer from the case.
fn animation_case(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let root = object(e, &[]);
    let accum = object(e, &[]);
    // Controllers on the roots (or none): plain objects with a reference count.
    for (node, has) in [(root, case.word(1) & 1), (accum, case.word(1) >> 1 & 1)] {
        if has != 0 {
            let c = object(e, &[]);
            e.mem.set_u32(c + 4, 3);
            e.mem.set_u32(node + 0xc, c);
        }
    }
    let animation = e.mem.alloc(0x100);
    let (root_word, accum_word) = match case.word(0) {
        0 => (0, accum),
        1 => (root, 0),
        _ => (root, accum),
    };
    e.mem.set_u32(animation + 8, root_word);
    e.mem.set_u32(animation + 0xc, accum_word);
    e.mem.set_f32(animation + 0xd0, 0.75);
    e.mem.set_u8(animation, case.word(2) as u8);
    // The limit setting (the thread count) at +4 of its object, the forcing byte.
    e.mem.set_u32(0x011c_3ea8, case.word(3));
    e.mem.set_u8(0x011e_07a8, case.word(4) as u8);
    e.mem.set_u32(0x011d_df38, 0x0300_0100);
    // The flag object's answer (`0042ce10`) on its own input, so all four pairs with the forcing byte
    // are generated.
    set_logged(e, 0x0c, case.word(5));
    vec![animation]
}

fn animation_inputs() -> Vec<Input> {
    vec![
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 2, 3, 6]),
        Input::Of(&[1, 2, 4]),
        Input::Flag,
        Input::Flag,
    ]
}

#[test]
fn animation_update_queued_scenegraph() {
    Diff::new(entry!(
        0x0049_3930,
        animation::queued::update_queued_scenegraph(Ptr<_>) -> bool
    ))
    .animation_common()
    .inputs(&animation_inputs())
    .setup(animation_case)
    .run()
    .assert_same();
}

#[test]
fn animation_update_scene_graph_no_controller() {
    Diff::new(entry!(
        0x0049_3bd0,
        animation::queued::update_scene_graph_no_controller(Ptr<_>)
    ))
    .animation_common()
    .inputs(&animation_inputs())
    .setup(animation_case)
    .run()
    .assert_same();
}
