//! The differential test of every typed function of the AI queries family
//! (`engine_typed::ai_queries`) against its translation. The world: actors and processes with
//! vtables of their own, each slot either the exe function the game's classes hold there (so the
//! typed side's direct branch runs) or a stand-in that answers a word kept beside the vtable; the
//! packages and package locations they hold; the player global.
//!
//! What stands in: the slots this family does not type (0x280, 0x234, 0x42c, and the others when a
//! case gives a stand-in) and `CombatController::IsFleeing` (`00981990`).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_typed::ai_queries::{queries, AI_QUERIES};

/// Where the stand-in slots are registered: `VT_FN + slot`.
const VT_FN: u32 = 0x00ff_4000;
/// A vtable's stand-in answers sit this far past it, at the same slot offset.
const ANSWERS: u32 = 0x800;
/// The player, a global.
const PLAYER: u32 = 0x011d_ea3c;
/// `CombatController::IsFleeing`.
const COMBAT_CONTROLLER_IS_FLEEING: u32 = 0x0098_1990;
/// Where its stand-in finds its answer.
const FLEEING_ANSWER: u32 = 0x0300_0000;

/// The slots a stand-in may hold.
const STAND_IN_SLOTS: [u32; 9] = [
    0x22c, 0x27c, 0x128, 0x35c, 0x280, 0x234, 0x428, 0x42c, 0x2c8,
];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// A stand-in slot: the answer its object's vtable keeps for it.
fn stand_in<const SLOT: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let vt = e.mem.u32(a[0]);
    ret(e.mem.u32(vt + ANSWERS + SLOT))
}

fn stand_ins(e: &mut Engine) {
    e.register(VT_FN + 0x22c, stand_in::<0x22c>);
    e.register(VT_FN + 0x27c, stand_in::<0x27c>);
    e.register(VT_FN + 0x128, stand_in::<0x128>);
    e.register(VT_FN + 0x35c, stand_in::<0x35c>);
    e.register(VT_FN + 0x280, stand_in::<0x280>);
    e.register(VT_FN + 0x234, stand_in::<0x234>);
    e.register(VT_FN + 0x428, stand_in::<0x428>);
    e.register(VT_FN + 0x42c, stand_in::<0x42c>);
    e.register(VT_FN + 0x2c8, stand_in::<0x2c8>);
    e.register(COMBAT_CONTROLLER_IS_FLEEING, |e, _| {
        ret(e.mem.u32(FLEEING_ANSWER))
    });
    e.map(FLEEING_ANSWER, 0x1000);
    // The player global's page.
    e.map(PLAYER & !0xfff, 0x1000);
}

/// An object of 0x200 bytes with a vtable of its own, every stand-in slot a stand-in answering 0.
fn object(e: &mut Engine) -> u32 {
    let vt = e.mem.alloc(2 * ANSWERS);
    for slot in STAND_IN_SLOTS {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    let o = e.mem.alloc(0x200);
    e.mem.set_u32(o, vt);
    o
}

/// `object`'s slot holds `target` (an exe function).
fn slot(e: &mut Engine, object: u32, slot: u32, target: u32) {
    let vt = e.mem.u32(object);
    e.mem.set_u32(vt + slot, target);
}

/// `object`'s stand-in slot answers `answer`.
fn answer(e: &mut Engine, object: u32, slot: u32, answer: u32) {
    let vt = e.mem.u32(object);
    e.mem.set_u32(vt + ANSWERS + slot, answer);
}

/// A package (an object) whose procedure type byte (+0x20) is `kind`.
fn package(e: &mut Engine, kind: u32) -> u32 {
    let p = object(e);
    e.mem.set_u8(p + 0x20, kind as u8);
    p
}

/// Words a field getter should hand back whole.
const FIELD_VALUES: &[u32] = &[0, 1, 0x80, 0xff, 0x7fff_ffff, 0x8000_0000, u32::MAX];
/// Bytes a byte getter should read alone: signs, and the procedure types the callers test.
const BYTES: &[u32] = &[0, 1, 0xa, 0x12, 0x13, 0x16, 0x7f, 0x80, 0xff];

/// An object with `value` at `offset` and other bytes around it.
fn field_object(e: &mut Engine, offset: u32, value: u32, width: u32) -> u32 {
    let o = e.mem.alloc(offset + 8);
    for i in 0..offset + 8 {
        e.mem.set_u8(o + i, 0xa5);
    }
    match width {
        1 => e.mem.set_u8(o + offset, value as u8),
        _ => e.mem.set_u32(o + offset, value),
    }
    o
}

#[test]
fn package_type() {
    Diff::new(entry!(0x0041_ca90, queries::package_type(u32) -> i32))
        .family(&AI_QUERIES)
        .inputs(&[Input::Of(BYTES)])
        .setup(|e, case| vec![field_object(e, 0x20, case.word(0), 1)])
        .run()
        .assert_same();
}

#[test]
fn byte_at_104() {
    Diff::new(entry!(0x0049_3bb0, queries::byte_at_104(u32) -> u8))
        .family(&AI_QUERIES)
        .inputs(&[Input::Of(BYTES)])
        .setup(|e, case| vec![field_object(e, 0x104, case.word(0), 1)])
        .run()
        .assert_same();
}

#[test]
fn word_at_2c() {
    Diff::new(entry!(0x0055_b980, queries::word_at_2c(u32) -> u32))
        .family(&AI_QUERIES)
        .inputs(&[Input::Of(FIELD_VALUES)])
        .setup(|e, case| vec![field_object(e, 0x2c, case.word(0), 4)])
        .run()
        .assert_same();
}

#[test]
fn word_at_40() {
    Diff::new(entry!(0x008d_6f30, queries::word_at_40(u32) -> u32))
        .family(&AI_QUERIES)
        .inputs(&[Input::Of(FIELD_VALUES)])
        .setup(|e, case| vec![field_object(e, 0x40, case.word(0), 4)])
        .run()
        .assert_same();
}

#[test]
fn location_reference() {
    Diff::new(entry!(0x0067_f390, queries::location_reference(u32) -> u32))
        .family(&AI_QUERIES)
        .inputs(&[Input::Of(&[0, 1, 0x80, 0xff]), Input::Of(FIELD_VALUES)])
        .setup(|e, case| {
            let o = field_object(e, 8, case.word(1), 4);
            e.mem.set_u8(o, case.word(0) as u8);
            vec![o]
        })
        .run()
        .assert_same();
}

#[test]
fn held_package() {
    Diff::new(entry!(0x007f_a950, queries::held_package(u32) -> u32))
        .family(&AI_QUERIES)
        .family(&leaf::LEAVES)
        .inputs(&[Input::Of(FIELD_VALUES)])
        .setup(|e, case| vec![field_object(e, 4, case.word(0), 4)])
        .run()
        .assert_same();
}

/// A process whose slot 0x22c is `007fa950` (`real`) or a stand-in answering `stand_in`, holding
/// `held` at +4.
fn process(e: &mut Engine, real: bool, held: u32, stand_in: u32) -> u32 {
    let p = object(e);
    if real {
        slot(e, p, 0x22c, 0x007f_a950);
    }
    answer(e, p, 0x22c, stand_in);
    e.mem.set_u32(p + 4, held);
    p
}

#[test]
fn package_that_is_running() {
    Diff::new(entry!(
        0x0085_9670,
        queries::package_that_is_running(u32) -> u32
    ))
    .family(&AI_QUERIES)
    .family(&leaf::LEAVES)
    .inputs(&[Input::Flag, Input::Of(&[0, 0x1234])])
    .setup(|e, case| {
        stand_ins(e);
        vec![process(e, case.word(0) != 0, case.word(1), 0x5678)]
    })
    .run()
    .assert_same();
}

/// An actor whose process (+0x68) is `process` (0: none), its byte at +0x104 `combat`.
fn actor(e: &mut Engine, process: u32, combat: u32) -> u32 {
    let a = object(e);
    e.mem.set_u32(a + 0x68, process);
    e.mem.set_u8(a + 0x104, combat as u8);
    a
}

#[test]
fn current_package() {
    // Inputs: a process or none; its slot 0x27c `00859670` or a stand-in; its slot 0x22c `007fa950`
    // or a stand-in; the package it holds (0 or one).
    Diff::new(entry!(0x0093_44a0, queries::current_package(u32) -> u32))
        .family(&AI_QUERIES)
        .family(&leaf::LEAVES)
        .inputs(&[Input::Flag, Input::Flag, Input::Flag, Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            let held = if case.word(3) != 0 {
                package(e, 0x16)
            } else {
                0
            };
            let p = process(e, case.word(2) != 0, held, 0x2468);
            if case.word(1) != 0 {
                slot(e, p, 0x27c, 0x0085_9670);
            }
            answer(e, p, 0x27c, 0x1357);
            let a = actor(e, if case.word(0) != 0 { p } else { 0 }, 0);
            vec![a]
        })
        .run()
        .assert_same();
}

#[test]
fn combat_controller() {
    // Inputs: a process or none; the byte at +0x104; the slot 0x22c real or a stand-in; what it
    // holds: nothing, or an object of procedure type 0x12, 0x16 or 0.
    Diff::new(entry!(0x008a_02d0, queries::combat_controller(u32) -> u32))
        .family(&AI_QUERIES)
        .family(&leaf::LEAVES)
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 1, 0x80]),
            Input::Flag,
            Input::Of(&[0, 0x12, 0x16, 0x100]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            let held = match case.word(3) {
                0 => 0,
                0x100 => package(e, 0),
                kind => package(e, kind),
            };
            let p = process(e, case.word(2) != 0, held, held);
            let a = actor(e, if case.word(0) != 0 { p } else { 0 }, case.word(1));
            vec![a]
        })
        .run()
        .assert_same();
}

#[test]
fn is_running_run_once() {
    // Inputs: the slot 0x35c `0047c850` or a stand-in; the stand-in's answer.
    Diff::new(entry!(0x0088_1570, queries::is_running_run_once(u32) -> u8))
        .family(&AI_QUERIES)
        // `0047c850`, the false body, is the draw family's: its direct branch runs in `typed_draw`.
        .goes_by_address(&[0x0047_c850])
        .inputs(&[Input::Flag, Input::Of(&[0, 1, 0x100, 0x0123_4501])])
        .setup(|e, case| {
            stand_ins(e);
            let p = object(e);
            if case.word(0) != 0 {
                slot(e, p, 0x35c, 0x0047_c850);
            }
            answer(e, p, 0x35c, case.word(1));
            vec![actor(e, p, 0)]
        })
        .run()
        .assert_same();
}

/// The world of `Actor::IsFleeing` and the slot 0x2c8. `chain`: 0 the process's slots 0x27c and
/// 0x22c the exe's functions, 1 a stand-in at 0x27c answering the package, 2 no package at all.
/// `kind`: the package's procedure type. `controller`: the actor's slot 0x428 `008a02d0` (with the
/// actor in combat and a combat package held: 1), `00acbb70` (2), or a stand-in answering a combat
/// controller (3) or 0 (0). `answers`: bit 0 the process's slot 0x280 (1 or 0), bit 1
/// `CombatController::IsFleeing`. `player`: whether the actor is the player.
struct FleeWorld {
    actor: u32,
    process: u32,
}

fn flee_world(
    e: &mut Engine,
    chain: u32,
    kind: u32,
    controller: u32,
    answers: u32,
    player: bool,
) -> FleeWorld {
    stand_ins(e);
    let held = if chain == 2 { 0 } else { package(e, kind) };
    let p = process(e, true, held, 0);
    if chain == 0 {
        slot(e, p, 0x27c, 0x0085_9670);
    }
    answer(e, p, 0x27c, held);
    answer(e, p, 0x280, answers & 1);
    let a = actor(e, p, u32::from(controller == 1));
    match controller {
        1 => {
            slot(e, a, 0x428, 0x008a_02d0);
            // The combat controller the process's slot 0x22c gives once the actor is in combat:
            // here the held package itself, so its type decides (0x12: a controller).
        }
        2 => slot(e, a, 0x428, 0x00ac_bb70),
        3 => {
            let c = package(e, 0x12);
            answer(e, a, 0x428, c);
        }
        _ => answer(e, a, 0x428, 0),
    }
    e.mem.set_u32(FLEEING_ANSWER, (answers >> 1) & 1);
    let other = object(e);
    e.mem.set_u32(PLAYER, if player { a } else { other });
    FleeWorld {
        actor: a,
        process: p,
    }
}

#[test]
fn is_fleeing() {
    Diff::new(entry!(0x008a_6650, queries::is_fleeing(u32, u8) -> bool))
        .family(&AI_QUERIES)
        .family(&leaf::LEAVES)
        .inputs(&[
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0x16, 0xa, 0x13, 0x12, 0]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Flag,
            Input::Flag,
        ])
        .max_cases(300)
        .setup(|e, case| {
            let w = flee_world(
                e,
                case.word(0),
                case.word(1),
                case.word(2),
                case.word(3),
                case.word(4) != 0,
            );
            vec![w.actor, case.word(5)]
        })
        .run()
        .assert_same();
}

#[test]
fn current_target() {
    // Inputs: a process or none; the byte at +0x104; the slot 0x35c `0047c850` or a stand-in
    // answering 1; the package's type (0x16: fleeing); the actor's slot 0x42c's answer; the
    // process's slot 0x128 `008d6f30`, `00acbb70` or a stand-in.
    Diff::new(entry!(0x0088_15a0, queries::current_target(u32) -> u32))
        .family(&AI_QUERIES)
        .family(&leaf::LEAVES)
        .goes_by_address(&[0x0047_c850])
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 1, 0x80]),
            Input::Flag,
            Input::Of(&[0x16, 0]),
            Input::Of(&[0, 0x4444]),
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            let w = flee_world(e, 0, case.word(3), 0, 0, false);
            let p = w.process;
            if case.word(2) != 0 {
                slot(e, p, 0x35c, 0x0047_c850);
            }
            answer(e, p, 0x35c, 1);
            match case.word(5) {
                0 => slot(e, p, 0x128, 0x008d_6f30),
                1 => slot(e, p, 0x128, 0x00ac_bb70),
                _ => answer(e, p, 0x128, 0x5555),
            }
            e.mem.set_u32(p + 0x40, 0x6666);
            let a = w.actor;
            e.mem.set_u8(a + 0x104, case.word(1) as u8);
            answer(e, a, 0x42c, case.word(4));
            if case.word(0) == 0 {
                e.mem.set_u32(a + 0x68, 0);
            }
            vec![a]
        })
        .run()
        .assert_same();
}
