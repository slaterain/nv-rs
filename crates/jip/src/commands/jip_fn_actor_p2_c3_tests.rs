//! Lane C3's tests of `jip_fn_actor_p2`: `GetArmourPenetrated`, `ToggleTeammateKillable` and
//! `PlayIdleEx`, each called by its opcode. Scenes at the offsets the module's doc gives (each
//! confirmed in the exe), game functions as stand-ins that record their words. Expectations from
//! JIP's command bodies and helpers (functions_jip/jip_fn_actor.h, nvse/GameObjects.cpp:
//! `Actor::GetHitDataValue`, nvse/GameForms.cpp: `TESIdleForm::FindIdle`, internal/jip_core.cpp:
//! `AnimData::GetPlayedIdle`, nvse/GameProcess.h: `AnimData::PlayIdle`), never from the port's
//! output.

use super::*;
use crate::commands::test_support::*;

const ACTOR: u32 = FORMS + 0x1000;
const PLAIN: u32 = FORMS + 0x1200;
const PROCESS: u32 = FORMS + 0x2000;
const HIT: u32 = FORMS + 0x2400;
const ANIM: u32 = FORMS + 0x2800;
const ANIM_P: u32 = FORMS + 0x2a00;
const ANIM_ACTOR: u32 = FORMS + 0x2c00;
const IDLE: u32 = FORMS + 0x3000;
const IDLE_A: u32 = FORMS + 0x3100;
const IDLE_B: u32 = FORMS + 0x3200;
const IDLE_C: u32 = FORMS + 0x3300;
const PICKED: u32 = FORMS + 0x3400;
const CHILDREN: u32 = FORMS + 0x4000;
const CHILDREN_DATA: u32 = FORMS + 0x4100;
const B_CHILDREN: u32 = FORMS + 0x4200;
const B_CHILDREN_DATA: u32 = FORMS + 0x4300;
const INTERACTED: u32 = FORMS + 0x5000;
const MANAGER: u32 = FORMS + 0x5400;
const ACTOR_VT: u32 = FORMS + 0x8000;
const PLAIN_VT: u32 = FORMS + 0x8800;
const ANSWERS: u32 = FORMS + 0xc000;
const A_ANIM: u32 = ANSWERS;
const A_DONE: u32 = ANSWERS + 4;
const A_PICK: u32 = ANSWERS + 8;
const A_ZERO: u32 = ANSWERS + 0xc;
const CALL_ANIM: u32 = 0x0090_0200;
const CALL_NO_ANIM: u32 = 0x0090_0210;
const RETURN_TRUE: u32 = 0x008d_0360;
const DONE_PLAYING_FN: u32 = 0x0049_85f0;
const GET_IDLE_TO_PLAY_FN: u32 = 0x0060_0950;
const PLAY_IDLE_FN: u32 = 0x0049_7f20;
const EVALUATE_FN: u32 = 0x0068_0c60;
const IDLE_MANAGER_AT: u32 = 0x011c_b6a0;

thread_local! {
    static CALLED: std::cell::RefCell<Vec<(u32, Vec<u32>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn record(addr: u32, w: &[u32]) {
    CALLED.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn called(addr: u32) -> Vec<Vec<u32>> {
    CALLED.with(|c| {
        c.borrow()
            .iter()
            .filter(|x| x.0 == addr)
            .map(|x| x.1.clone())
            .collect()
    })
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// A stand-in at `addr` recording its words and answering the word at `answer`.
macro_rules! answer_word {
    ($e:expr, $addr:expr, $answer:expr) => {
        $e.register($addr, |e, w| {
            record($addr, w);
            eax(e.mem.u32($answer))
        })
    };
}

/// An actor (`ACTOR`, vtable +0x100 `ReturnTrue`) with a level 0 process; a plain reference;
/// idles of type 0x48 (script references 1 to 4: `IDLE`, `IDLE_A`, `IDLE_B`, `PICKED`); the
/// game functions as stand-ins. `ConditionList::Evaluate` answers the first byte of the list it
/// is given (an idle's +0x30), so a test makes an idle's conditions hold with that byte.
fn setup() -> Engine {
    CALLED.with(|c| c.borrow_mut().clear());
    let mut e = engine();
    if !e.mem.is_mapped(IDLE_MANAGER_AT) {
        e.map(IDLE_MANAGER_AT & !0xfff, 0x1000);
    }
    e.mem.set_u32(IDLE_MANAGER_AT, MANAGER);
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u8(ACTOR + 4, 0x3b);
    e.mem.set_u32(ACTOR + 0xc, 0x200);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(ACTOR_VT + 0x100, RETURN_TRUE);
    e.mem.set_u32(ACTOR_VT + 0x1e4, CALL_ANIM);
    e.mem.set_u32(PLAIN, PLAIN_VT);
    e.mem.set_u8(PLAIN + 4, 0x3a);
    e.mem.set_u32(PLAIN + 0x68, PROCESS);
    e.mem.set_u32(PLAIN_VT + 0x100, 0x0047_c850);
    e.mem.set_u32(PLAIN_VT + 0x1e4, CALL_ANIM);
    answer_word!(e, CALL_ANIM, A_ANIM);
    answer_word!(e, CALL_NO_ANIM, A_ZERO);
    answer_word!(e, DONE_PLAYING_FN, A_DONE);
    answer_word!(e, GET_IDLE_TO_PLAY_FN, A_PICK);
    answer_word!(e, PLAY_IDLE_FN, A_ZERO);
    e.register(EVALUATE_FN, |e, w| {
        record(EVALUATE_FN, w);
        // A `bool` in `al`: the bits above it are whatever the exe left there.
        eax(0x1234_5600 | u32::from(e.mem.u8(w[0])))
    });
    for (i, (f, id)) in [
        (IDLE, 0x300),
        (IDLE_A, 0x301),
        (IDLE_B, 0x302),
        (PICKED, 0x303),
    ]
    .into_iter()
    .enumerate()
    {
        form(&mut e, f, 0x48, id);
        set_ref(&mut e, i as u16 + 1, f);
    }
    form(&mut e, IDLE_C, 0x48, 0x304);
    e.mem.set_u32(A_ANIM, ANIM);
    e.mem.set_u32(ANIM + 4, ANIM_ACTOR);
    e.mem.set_u32(PROCESS + 0x40, INTERACTED);
    e
}

// ---- GetArmourPenetrated ----

#[test]
fn armour_penetrated_is_bit_31_of_the_last_hits_flags() {
    let mut e = setup();
    e.mem.set_u32(PROCESS + 0x240, HIT);
    e.mem.set_u32(HIT + 0x58, 0x8000_0000);
    assert_eq!(call_on(&mut e, 0x2883, ACTOR, &[]).1, 1.0, "bit 31 set");
    e.mem.set_u32(HIT + 0x58, 0x7fff_ffff);
    assert_eq!(
        call_on(&mut e, 0x2883, ACTOR, &[]).1,
        0.0,
        "every other bit"
    );
    // The flags word is +0x58: bit 31 of the words beside it is not it.
    e.mem.set_u32(HIT + 0x54, 0x8000_0000);
    e.mem.set_u32(HIT + 0x5c, 0x8000_0000);
    e.mem.set_u32(HIT + 0x58, 0);
    assert_eq!(
        call_on(&mut e, 0x2883, ACTOR, &[]).1,
        0.0,
        "+0x54 and +0x5C"
    );
    e.mem.set_u32(HIT + 0x58, 0x8000_0001);
    // Level 1 still reads (`processLevel <= 1`); level 2 does not.
    e.mem.set_u32(PROCESS + 0x28, 1);
    assert_eq!(call_on(&mut e, 0x2883, ACTOR, &[]).1, 1.0, "level 1");
    e.mem.set_u32(PROCESS + 0x28, 2);
    assert_eq!(call_on(&mut e, 0x2883, ACTOR, &[]).1, 0.0, "level 2");
    e.mem.set_u32(PROCESS + 0x28, 0);
    e.mem.set_u32(PROCESS + 0x240, 0);
    assert_eq!(call_on(&mut e, 0x2883, ACTOR, &[]).1, 0.0, "no hit data");
    e.mem.set_u32(PROCESS + 0x240, HIT);
    assert_eq!(call_on(&mut e, 0x2883, PLAIN, &[]).1, 0.0, "not an actor");
    assert_eq!(call_on(&mut e, 0x2883, 0, &[]).1, 0.0, "no reference");
    e.mem.set_u32(ACTOR + 0x68, 0);
    assert_eq!(call_on(&mut e, 0x2883, ACTOR, &[]).1, 0.0, "no process");
}

// ---- ToggleTeammateKillable ----

#[test]
fn teammate_killable_answers_and_flips_bit_4_of_jip_flags_2() {
    let mut e = setup();
    assert_eq!(
        crate::game_patches::ge::TEAMMATE_KILLABLE,
        4,
        "the bit H18's seam reads"
    );
    e.mem.set_u8(ACTOR + 0x105, 0xaa);
    e.mem.set_u8(ACTOR + 0x106, 0x03);
    e.mem.set_u8(ACTOR + 0x107, 0x55);
    let flags2 = |e: &Engine| e.mem.u8(ACTOR + 0x106);
    assert_eq!(call_on(&mut e, 0x28eb, ACTOR, &[]).1, 0.0, "clear");
    assert_eq!(flags2(&e), 0x03, "no argument: no change");
    assert_eq!(
        call_on(&mut e, 0x28eb, ACTOR, &[n(1)]).1,
        0.0,
        "the state before"
    );
    assert_eq!(flags2(&e), 0x07, "set, the other bits kept");
    assert_eq!(call_on(&mut e, 0x28eb, ACTOR, &[n(1)]).1, 1.0);
    assert_eq!(flags2(&e), 0x07, "already set: stays");
    assert_eq!(call_on(&mut e, 0x28eb, ACTOR, &[]).1, 1.0);
    assert_eq!(flags2(&e), 0x07);
    assert_eq!(call_on(&mut e, 0x28eb, ACTOR, &[n(0)]).1, 1.0);
    assert_eq!(flags2(&e), 0x03, "cleared");
    assert_eq!(call_on(&mut e, 0x28eb, ACTOR, &[n(0)]).1, 0.0);
    assert_eq!(flags2(&e), 0x03, "already clear: stays");
    assert_eq!(call_on(&mut e, 0x28eb, ACTOR, &[n(-7)]).1, 0.0);
    assert_eq!(flags2(&e), 0x07, "any non-zero sets");
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0xaa, "jipActorFlags1 untouched");
    assert_eq!(e.mem.u8(ACTOR + 0x107), 0x55, "jipActorFlags3 untouched");
    // Not an actor: no answer, nothing written.
    e.mem.set_u8(PLAIN + 0x106, 0x04);
    assert_eq!(call_on(&mut e, 0x28eb, PLAIN, &[n(0)]).1, 0.0);
    assert_eq!(e.mem.u8(PLAIN + 0x106), 0x04);
    assert_eq!(call_on(&mut e, 0x28eb, 0, &[n(1)]).1, 0.0, "no reference");
}

// ---- PlayIdleEx ----

fn played(_: &Engine) -> Vec<Vec<u32>> {
    called(PLAY_IDLE_FN)
}

#[test]
fn play_idle_ex_plays_a_given_idle_with_its_group() {
    let mut e = setup();
    e.mem.set_u8(IDLE_A + 0x38, 0xc5);
    call_on(&mut e, 0x2807, ACTOR, &[r(2)]);
    // `PlayIdle(idle, animData->actor, groupFlags & 0x3F, 3)` on the animation data.
    assert_eq!(played(&e), vec![vec![ANIM, IDLE_A, ANIM_ACTOR, 5, 3]]);
    assert!(
        called(EVALUATE_FN).is_empty(),
        "no children: its conditions not asked"
    );
    assert!(called(GET_IDLE_TO_PLAY_FN).is_empty());
    // The same idle playing (`GetPlayedIdle`, not done): not played again.
    e.mem.set_u32(ANIM + 0x124, ANIM_P);
    e.mem.set_u32(ANIM_P + 0x2c, IDLE_A);
    call_on(&mut e, 0x2807, ACTOR, &[r(2)]);
    assert_eq!(played(&e).len(), 1, "already playing");
    // Another idle playing: played.
    call_on(&mut e, 0x2807, ACTOR, &[r(3)]);
    assert_eq!(played(&e)[1], vec![ANIM, IDLE_B, ANIM_ACTOR, 0, 3]);
    // The played idle done (al 1): it is no longer the played one.
    e.mem.set_u32(A_DONE, 1);
    call_on(&mut e, 0x2807, ACTOR, &[r(2)]);
    assert_eq!(played(&e).len(), 3);
}

#[test]
fn play_idle_ex_without_an_idle_plays_the_managers_pick() {
    let mut e = setup();
    e.mem.set_u32(A_PICK, PICKED);
    e.mem.set_u8(PICKED + 0x38, 0x14);
    call_on(&mut e, 0x2807, ACTOR, &[]);
    // `GetIdleToPlay(IdleAnimsDirectoryMap, actor, process->interactedRef)`.
    assert_eq!(
        called(GET_IDLE_TO_PLAY_FN),
        vec![vec![MANAGER, ACTOR, INTERACTED]]
    );
    assert_eq!(played(&e), vec![vec![ANIM, PICKED, ANIM_ACTOR, 0x14, 3]]);
    e.mem.set_u32(A_PICK, 0);
    call_on(&mut e, 0x2807, ACTOR, &[]);
    assert_eq!(played(&e).len(), 1, "no pick: nothing played");
}

/// `IDLE` with children `IDLE_A`, `IDLE_B` and a zeroed slot (its +8 word 3, its +0xC count 2);
/// `IDLE_B` with the child `IDLE_C`.
fn idle_tree(e: &mut Engine) {
    e.mem.set_u32(IDLE + 0x40, CHILDREN);
    e.mem.set_u32(CHILDREN + 4, CHILDREN_DATA);
    e.mem.set_u32(CHILDREN + 8, 3);
    e.mem.set_u32(CHILDREN + 0xc, 2);
    e.mem.set_u32(CHILDREN_DATA, IDLE_A);
    e.mem.set_u32(CHILDREN_DATA + 4, IDLE_B);
    e.mem.set_u32(CHILDREN_DATA + 8, 0);
    e.mem.set_u32(IDLE_B + 0x40, B_CHILDREN);
    e.mem.set_u32(B_CHILDREN + 4, B_CHILDREN_DATA);
    e.mem.set_u32(B_CHILDREN + 8, 1);
    e.mem.set_u32(B_CHILDREN + 0xc, 1);
    e.mem.set_u32(B_CHILDREN_DATA, IDLE_C);
}

fn holds(e: &mut Engine, idle: u32, yes: bool) {
    e.mem.set_u8(idle + 0x30, u8::from(yes));
}

fn asked(_: &Engine) -> Vec<u32> {
    called(EVALUATE_FN).into_iter().map(|w| w[0]).collect()
}

#[test]
fn play_idle_ex_picks_the_first_child_whose_conditions_hold() {
    let mut e = setup();
    idle_tree(&mut e);
    holds(&mut e, IDLE, true);
    holds(&mut e, IDLE_A, true);
    call_on(&mut e, 0x2807, ACTOR, &[r(1)]);
    assert_eq!(played(&e), vec![vec![ANIM, IDLE_A, ANIM_ACTOR, 0, 3]]);
    // `conditions.Evaluate(animActor, 0, &eval, 0)` on each idle's +0x30.
    let w = &called(EVALUATE_FN)[0];
    assert_eq!((w[0], w[1], w[2], w[4]), (IDLE + 0x30, ACTOR, 0, 0));
    assert_eq!(asked(&e), vec![IDLE + 0x30, IDLE_A + 0x30]);
    // IDLE_A fails: IDLE_B holds but has a child, which must hold too.
    CALLED.with(|c| c.borrow_mut().clear());
    holds(&mut e, IDLE_A, false);
    holds(&mut e, IDLE_B, true);
    holds(&mut e, IDLE_C, true);
    call_on(&mut e, 0x2807, ACTOR, &[r(1)]);
    assert_eq!(played(&e), vec![vec![ANIM, IDLE_C, ANIM_ACTOR, 0, 3]]);
    assert_eq!(
        asked(&e),
        vec![IDLE + 0x30, IDLE_A + 0x30, IDLE_B + 0x30, IDLE_C + 0x30]
    );
}

#[test]
fn play_idle_ex_plays_nothing_when_no_child_holds() {
    let mut e = setup();
    idle_tree(&mut e);
    // The parent's own conditions fail: none of its children is asked.
    call_on(&mut e, 0x2807, ACTOR, &[r(1)]);
    assert!(played(&e).is_empty());
    assert_eq!(asked(&e), vec![IDLE + 0x30]);
    // The parent holds, no child does: JIP's walk reaches the zeroed third slot (where JIP
    // itself would call `FindIdle` on null); skipped, nothing played.
    CALLED.with(|c| c.borrow_mut().clear());
    holds(&mut e, IDLE, true);
    holds(&mut e, IDLE_B, true);
    call_on(&mut e, 0x2807, ACTOR, &[r(1)]);
    assert!(
        played(&e).is_empty(),
        "IDLE_C fails, so IDLE_B answers none"
    );
    assert_eq!(
        asked(&e),
        vec![IDLE + 0x30, IDLE_A + 0x30, IDLE_B + 0x30, IDLE_C + 0x30]
    );
}

#[test]
fn play_idle_ex_counts_children_by_the_word_at_8() {
    let mut e = setup();
    idle_tree(&mut e);
    holds(&mut e, IDLE, true);
    holds(&mut e, IDLE_B, true);
    holds(&mut e, IDLE_C, true);
    // JIP's `children->size` is the word +8: with +8 at 1 only IDLE_A is asked (+0xC says 2).
    e.mem.set_u32(CHILDREN + 8, 1);
    call_on(&mut e, 0x2807, ACTOR, &[r(1)]);
    assert!(played(&e).is_empty());
    assert_eq!(asked(&e), vec![IDLE + 0x30, IDLE_A + 0x30]);
    // With +8 at 2 and +0xC at 1, IDLE_B is reached and its child played.
    CALLED.with(|c| c.borrow_mut().clear());
    e.mem.set_u32(CHILDREN + 8, 2);
    e.mem.set_u32(CHILDREN + 0xc, 1);
    call_on(&mut e, 0x2807, ACTOR, &[r(1)]);
    assert_eq!(played(&e), vec![vec![ANIM, IDLE_C, ANIM_ACTOR, 0, 3]]);
}

#[test]
fn play_idle_ex_needs_an_actor_with_a_high_process_and_animation_data() {
    let mut e = setup();
    e.mem.set_u32(A_PICK, PICKED);
    for level in [1, 3, 0x100] {
        e.mem.set_u32(PROCESS + 0x28, level);
        call_on(&mut e, 0x2807, ACTOR, &[r(2)]);
        call_on(&mut e, 0x2807, ACTOR, &[]);
    }
    e.mem.set_u32(PROCESS + 0x28, 0);
    call_on(&mut e, 0x2807, PLAIN, &[r(2)]);
    call_on(&mut e, 0x2807, 0, &[r(2)]);
    e.mem.set_u32(A_ANIM, 0);
    call_on(&mut e, 0x2807, ACTOR, &[r(2)]);
    call_on(&mut e, 0x2807, ACTOR, &[]);
    e.mem.set_u32(A_ANIM, ANIM);
    e.mem.set_u32(ACTOR + 0x68, 0);
    call_on(&mut e, 0x2807, ACTOR, &[r(2)]);
    assert!(played(&e).is_empty(), "nothing played");
    assert!(called(GET_IDLE_TO_PLAY_FN).is_empty(), "no pick asked");
    // The answer stays 0 throughout.
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    assert_eq!(call_on(&mut e, 0x2807, ACTOR, &[r(2)]).1, 0.0);
    assert_eq!(played(&e).len(), 1);
}

// ---- the table ----

/// The three slots as JIP's `DEFINE_*` lines and `REG_CMD` make them, each with a handler.
#[test]
fn c3_actor_p2_table_entries() {
    let e = engine();
    for (op, name, alias, params) in [
        (0x2883, "GetArmourPenetrated", "", "nullptr"),
        (
            0x28eb,
            "ToggleTeammateKillable",
            "SetTeammateKillable",
            "kParams_OneOptionalInt",
        ),
        (0x2807, "PlayIdleEx", "", "kParams_OneOptionalIdleForm"),
    ] {
        let slot = crate::slots::SLOTS
            .iter()
            .find(|s| s.opcode == op)
            .expect("a slot");
        assert_eq!(
            (slot.name, slot.alias, slot.params.name, slot.needs_ref),
            (name, alias, params, true),
            "{op:04x}"
        );
        assert_eq!(slot.kind, crate::table::Kind::Command, "{op:04x}");
        assert!(super::super::is_built(op), "{op:04x} built");
        let record = nvse::table::record_of(&e, op);
        assert_ne!(e.mem.u32(record + 0x18), 0, "{op:04x} has an execute");
    }
    let idle = &crate::slots::SLOTS
        .iter()
        .find(|s| s.opcode == 0x2807)
        .expect("PlayIdleEx")
        .params
        .list;
    assert_eq!(
        (idle.len(), idle[0].kind, idle[0].optional),
        (1, 0x30, true)
    );
}
