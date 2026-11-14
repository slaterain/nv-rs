//! Tests of `jip_fn_actor_p2`, expectations from JIP's source: actors (vtable +0x100 true) with a
//! process (level +0x28; `GetInterruptPackage` +0x20C, `GetCurrentPackage` +0x27C, `GetHitData`
//! +0x770 as stand-ins), last hit data, movers, animation data and bipeds at JIP's header offsets,
//! actor bases at the loaders' offsets, and the game functions called as stand-ins that record
//! their words.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::Data;

const NPC: u32 = FORMS + 0x1000;
const NPC2: u32 = FORMS + 0x1400;
const CREA: u32 = FORMS + 0x1800;
const CREA2: u32 = FORMS + 0x1c00;
const LVLC: u32 = FORMS + 0x2000;
const ACTOR: u32 = FORMS + 0x2200;
const ACTOR2: u32 = FORMS + 0x2400;
const PLAIN_REF: u32 = FORMS + 0x2600;
const FACTION: u32 = FORMS + 0x2700;
const FACTION2: u32 = FORMS + 0x2740;
const FACTION3: u32 = FORMS + 0x2780;
const LIST: u32 = FORMS + 0x2800;
const SPELL: u32 = FORMS + 0x2840;
const SPELL2: u32 = FORMS + 0x2880;
const IDLE: u32 = FORMS + 0x28c0;
const IDLE2: u32 = FORMS + 0x2900;
const WEAP: u32 = FORMS + 0x2940;
const ARMO: u32 = FORMS + 0x2980;
const ARMO2: u32 = FORMS + 0x29c0;
const MISC: u32 = FORMS + 0x2a00;
const PROJ: u32 = FORMS + 0x2a40;
const BPD: u32 = FORMS + 0x2b00;
const BODY_PART: u32 = FORMS + 0x2c00;
const PACKAGE: u32 = FORMS + 0x2d00;
const PROCESS: u32 = FORMS + 0x3000;
const HIT: u32 = FORMS + 0x3400;
const MOVER: u32 = FORMS + 0x3500;
const ANIM: u32 = FORMS + 0x3600;
const ANIM_Q: u32 = FORMS + 0x3700;
const ANIM_P: u32 = FORMS + 0x3740;
const BIPED: u32 = FORMS + 0x3800;
const ENTRY: u32 = FORMS + 0x3c00;
const ENTRY2: u32 = FORMS + 0x3c40;
const XDL: u32 = FORMS + 0x3c80;
const CHANGES: u32 = FORMS + 0x3d00;
const NODES: u32 = FORMS + 0x4000;
const X1: u32 = FORMS + 0x5000;
const X2: u32 = FORMS + 0x5100;
const X3: u32 = FORMS + 0x5200;
const ACTOR_VT: u32 = FORMS + 0x8000;
const PLAIN_VT: u32 = FORMS + 0x8800;
const PROC_VT: u32 = FORMS + 0x9000;
const MOVER_VT: u32 = FORMS + 0x9800;
const BASE_VT: u32 = FORMS + 0xa000;
const PLAYER: u32 = FORMS + 0xb000;
const CALL_ANIM: u32 = 0x0090_0200;
const CALL_NO_ANIM: u32 = 0x0090_0210;
const CALL_BIPED: u32 = 0x0090_0220;
const CALL_IS_CHAR: u32 = 0x0090_0230;
const CALL_NOT_CHAR: u32 = 0x0090_0240;
const CALL_DAMAGE: u32 = 0x0090_0250;
const CALL_INTERRUPT: u32 = 0x0090_0260;
const CALL_CURRENT: u32 = 0x0090_0270;
const CALL_HIT_DATA: u32 = 0x0090_0280;
const CALL_MOVE_FLAGS: u32 = 0x0090_0290;
const CALL_BPD: u32 = 0x0090_02a0;

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn node(e: &mut Engine, at: u32, data: u32, next: u32) {
    e.mem.set_u32(at, data);
    e.mem.set_u32(at + 4, next);
}

/// A stand-in at `addr` answering the word at `at` (so a test changes what it answers by
/// memory), its words recorded.
macro_rules! answer_word {
    ($e:expr, $addr:expr, $at:expr) => {
        $e.register($addr, |e, w| {
            CALLED.with(|c| c.borrow_mut().push(($addr, w.to_vec())));
            eax(e.mem.u32($at))
        })
    };
}

thread_local! {
    static CALLED: std::cell::RefCell<Vec<(u32, Vec<u32>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
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

/// Words the stand-ins answer from.
const ANSWERS: u32 = FORMS + 0xc000;
const A_ANIM: u32 = ANSWERS;
const A_BIPED: u32 = ANSWERS + 4;
const A_INTERRUPT: u32 = ANSWERS + 8;
const A_CURRENT: u32 = ANSWERS + 0xc;
const A_HIT: u32 = ANSWERS + 0x10;
const A_MOVE: u32 = ANSWERS + 0x14;
const A_FLEE: u32 = ANSWERS + 0x18;
const A_DONE: u32 = ANSWERS + 0x1c;
const A_BPD: u32 = ANSWERS + 0x20;

fn setup() -> Engine {
    CALLED.with(|c| c.borrow_mut().clear());
    let mut e = engine();
    extra_stand_ins(&mut e);
    game_globals(&mut e);
    for (f, t, id) in [
        (NPC, 0x2a, 0x100),
        (NPC2, 0x2a, 0x101),
        (CREA, 0x2b, 0x102),
        (CREA2, 0x2b, 0x103),
        (LVLC, 0x2c, 0x104),
        (FACTION, 0x08, 0x105),
        (FACTION2, 0x08, 0x106),
        (FACTION3, 0x08, 0x107),
        (LIST, 0x55, 0x108),
        (SPELL, 0x14, 0x109),
        (SPELL2, 0x14, 0x10a),
        (IDLE, 0x48, 0x10b),
        (IDLE2, 0x48, 0x10c),
        (WEAP, 0x28, 0x10d),
        (ARMO, 0x18, 0x10e),
        (ARMO2, 0x18, 0x10f),
        (MISC, 0x1f, 0x110),
        (PROJ, 0x33, 0x111),
        (BPD, 0x57, 0x112),
    ] {
        form(&mut e, f, t, id);
    }
    for a in [ARMO, ARMO2] {
        e.mem.set_u32(a, TES_OBJECT_ARMO_VTBL);
    }
    for b in [NPC, NPC2, CREA, CREA2] {
        e.mem.set_u32(b, BASE_VT);
    }
    e.mem.set_u32(BASE_VT + 0x180, CALL_BPD);
    answer_word!(e, CALL_BPD, A_BPD);
    // Actors: vtable +0x100 true; their virtual functions as stand-ins.
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(ACTOR_VT + 0x1e4, CALL_ANIM);
    e.mem.set_u32(ACTOR_VT + 0x1e8, CALL_BIPED);
    e.mem.set_u32(ACTOR_VT + 0x218, CALL_IS_CHAR);
    e.mem.set_u32(ACTOR_VT + 0x3ac, CALL_DAMAGE);
    // GetEditorID (+0x130): an empty text (the console echo asks).
    e.mem.set_u32(ACTOR_VT + 0x130, CALL_NO_ANIM);
    e.mem.set_u32(PLAIN_VT + 0x1e4, CALL_NO_ANIM);
    e.mem.set_u32(PLAIN_VT + 0x218, CALL_NOT_CHAR);
    answer_word!(e, CALL_ANIM, A_ANIM);
    answer_word!(e, CALL_BIPED, A_BIPED);
    answer_word!(e, CALL_IS_CHAR, A_ANIM + 0x100);
    e.mem.set_u32(A_ANIM + 0x100, 1);
    answer_word!(e, CALL_NO_ANIM, A_ANIM + 0x104);
    answer_word!(e, CALL_NOT_CHAR, A_ANIM + 0x104);
    answer_word!(e, CALL_DAMAGE, A_ANIM + 0x104);
    for (r, base, id) in [(ACTOR, NPC, 0x200), (ACTOR2, CREA, 0x201)] {
        form(&mut e, r, 0x3b, id);
        e.mem.set_u32(r, ACTOR_VT);
        e.mem.set_u32(r + 0x20, base);
        e.mem.set_u32(r + 0x68, PROCESS);
    }
    e.mem.set_u8(ACTOR2 + 4, 0x3c);
    form(&mut e, PLAIN_REF, 0x3a, 0x202);
    e.mem.set_u32(PLAIN_REF, PLAIN_VT);
    e.mem.set_u32(PLAIN_REF + 0x20, MISC);
    // The process (level 0), its virtual functions.
    e.mem.set_u32(PROCESS, PROC_VT);
    e.mem.set_u32(PROC_VT + 0x20c, CALL_INTERRUPT);
    e.mem.set_u32(PROC_VT + 0x27c, CALL_CURRENT);
    e.mem.set_u32(PROC_VT + 0x770, CALL_HIT_DATA);
    answer_word!(e, CALL_INTERRUPT, A_INTERRUPT);
    answer_word!(e, CALL_CURRENT, A_CURRENT);
    answer_word!(e, CALL_HIT_DATA, A_HIT);
    e.mem.set_u32(MOVER, MOVER_VT);
    e.mem.set_u32(MOVER_VT + 0x20, CALL_MOVE_FLAGS);
    answer_word!(e, CALL_MOVE_FLAGS, A_MOVE);
    // Game functions at their addresses.
    answer_word!(e, 0x0098_1990, A_FLEE);
    answer_word!(e, 0x0049_85f0, A_DONE);
    for (i, f) in [
        NPC, CREA, FACTION, LIST, SPELL, SPELL2, IDLE, WEAP, ARMO, MISC, LVLC, CREA2, ACTOR, NPC2,
    ]
    .iter()
    .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

fn eval(e: &mut Engine, opcode: u32, this: u32, arg1: u32) -> f64 {
    let record = nvse::table::record_of(e, opcode);
    let at = e.mem.u32(record + 0x20);
    e.mem.set_f64(RESULT, 0.0);
    e.call(at, &[this, arg1, 0, RESULT]);
    e.mem.f64(RESULT)
}

fn hit_setup(e: &mut Engine) {
    e.mem.set_u32(PROCESS + 0x240, HIT);
    e.mem.set_u32(A_HIT, HIT);
    e.mem.set_u32(HIT, ACTOR2);
    e.mem.set_u32(HIT + 8, PROJ);
    e.mem.set_f32(HIT + 0x18, 12.5);
    e.mem.set_f32(HIT + 0x1c, 3.25);
    e.mem.set_f32(HIT + 0x20, 7.75);
    e.mem.set_u32(HIT + 0x30, WEAP);
}

#[test]
fn the_last_hit_values() {
    let mut e = setup();
    hit_setup(&mut e);
    assert_eq!(call_on(&mut e, 0x2803, ACTOR, &[]).1, 7.75);
    assert_eq!(call_on(&mut e, 0x290a, ACTOR, &[]).1, 12.5);
    assert_eq!(call_on(&mut e, 0x290b, ACTOR, &[]).1, 3.25);
    call_on(&mut e, 0x282c, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x201);
    call_on(&mut e, 0x282d, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x111);
    call_on(&mut e, 0x282e, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x10d);
    // A form left out leaves the result at 0.
    e.mem.set_u32(HIT + 0x30, 0);
    call_on(&mut e, 0x282e, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
    // Level 1 still reads; level 2 does not (a 32-bit compare).
    e.mem.set_u32(PROCESS + 0x28, 1);
    assert_eq!(call_on(&mut e, 0x2803, ACTOR, &[]).1, 7.75);
    e.mem.set_u32(PROCESS + 0x28, 0x100);
    assert_eq!(call_on(&mut e, 0x2803, ACTOR, &[]).1, 0.0, "32-bit level");
    e.mem.set_u32(PROCESS + 0x28, 0);
    e.mem.set_u32(PROCESS + 0x240, 0);
    assert_eq!(call_on(&mut e, 0x2803, ACTOR, &[]).1, 0.0, "no hit data");
    e.mem.set_u32(PROCESS + 0x240, HIT);
    assert_eq!(call_on(&mut e, 0x2803, PLAIN_REF, &[]).1, 0.0, "not an actor");
    assert_eq!(call_on(&mut e, 0x2803, 0, &[]).1, 0.0, "no reference");
    e.mem.set_u32(ACTOR + 0x68, 0);
    assert_eq!(call_on(&mut e, 0x2803, ACTOR, &[]).1, 0.0, "no process");
}

#[test]
fn hit_flags_by_the_table() {
    let mut e = setup();
    hit_setup(&mut e);
    let table = [1u32, 4, 0x10, 0x20, 0x40, 0x80, 0x300, 0x400, 0x2000];
    for (i, &f) in table.iter().enumerate() {
        e.mem.set_u32(HIT + 0x58, f);
        assert_eq!(call_on(&mut e, 0x28ee, ACTOR, &[n(i as i32)]).1, 1.0, "{i}");
        e.mem.set_u32(HIT + 0x58, !f);
        assert_eq!(call_on(&mut e, 0x28ee, ACTOR, &[n(i as i32)]).1, 0.0, "{i}");
    }
    e.mem.set_u32(HIT + 0x58, 0x200);
    assert_eq!(call_on(&mut e, 0x28ee, ACTOR, &[n(6)]).1, 1.0, "either break bit");
    e.mem.set_u32(HIT + 0x58, u32::MAX);
    assert_eq!(call_on(&mut e, 0x28ee, ACTOR, &[n(9)]).1, 0.0, "past the table");
    assert_eq!(call_on(&mut e, 0x28ee, ACTOR, &[n(-1)]).1, 0.0, "unsigned");
    // The virtual function's answer, not the field.
    e.mem.set_u32(A_HIT, 0);
    assert_eq!(call_on(&mut e, 0x28ee, ACTOR, &[n(0)]).1, 0.0);
    e.mem.set_u32(A_HIT, HIT);
    assert_eq!(call_on(&mut e, 0x28ee, PLAIN_REF, &[n(0)]).1, 0.0);
    e.mem.set_u32(ACTOR + 0x68, 0);
    assert_eq!(call_on(&mut e, 0x28ee, ACTOR, &[n(0)]).1, 0.0);
}

#[test]
fn hit_3d_data_by_type() {
    let mut e = setup();
    hit_setup(&mut e);
    for i in 0..6 {
        e.mem.set_f32(HIT + 0x38 + 4 * i, i as f32 + 1.0);
    }
    let nums = |v: &[f64]| v.iter().map(|&x| Data::Number(x)).collect::<Vec<_>>();
    let res = call_on(&mut e, 0x28a8, ACTOR, &[n(0)]).1;
    assert_eq!(array_data(&e, res), nums(&[1.0, 2.0, 3.0]));
    let res = call_on(&mut e, 0x28a8, ACTOR, &[n(1)]).1;
    assert_eq!(array_data(&e, res), nums(&[4.0, 5.0, 6.0]));
    let res = call_on(&mut e, 0x28a8, ACTOR, &[n(2)]).1;
    assert_eq!(array_data(&e, res), nums(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]));
    let res = call_on(&mut e, 0x28a8, ACTOR, &[n(-1)]).1;
    assert_eq!(array_data(&e, res).len(), 6, "unsigned: above 1");
    e.mem.set_u32(A_HIT, 0);
    assert_eq!(call_on(&mut e, 0x28a8, ACTOR, &[n(0)]).1, 0.0);
    e.mem.set_u32(A_HIT, HIT);
    assert_eq!(call_on(&mut e, 0x28a8, PLAIN_REF, &[n(0)]).1, 0.0);
}

#[test]
fn base_spell_list() {
    let mut e = setup();
    let list = NPC + 0x80;
    call(&mut e, 0x2823, &[r(5), r(1)]);
    assert_eq!(e.mem.u32(list), SPELL);
    call(&mut e, 0x2823, &[r(5), r(1)]);
    assert_eq!(e.mem.u32(list + 4), 0, "already there");
    // The calling actor's own base form.
    call_on(&mut e, 0x2823, ACTOR, &[r(6)]);
    assert_eq!(e.mem.u32(list), SPELL2, "prepended");
    assert_eq!(e.mem.u32(e.mem.u32(list + 4)), SPELL);
    call_on(&mut e, 0x2824, ACTOR, &[r(5)]);
    assert_eq!((e.mem.u32(list), e.mem.u32(list + 4)), (SPELL2, 0));
    call(&mut e, 0x2824, &[r(6), r(1)]);
    assert_eq!(e.mem.u32(list), 0);
    // No base and no actor: nothing.
    call_on(&mut e, 0x2823, PLAIN_REF, &[r(5)]);
    call(&mut e, 0x2823, &[r(5)]);
    assert_eq!(e.mem.u32(list), 0);
    // Own base, not GetActorBase: a leveled actor's own base.
    e.mem.set_u8(CREA + 0xf, 0xff);
    put_extra(&mut e, ACTOR2 + 0x44, X1, 0x2e, 0);
    e.mem.set_u32(X1 + 0x10, CREA2);
    call_on(&mut e, 0x2823, ACTOR2, &[r(5)]);
    assert_eq!((e.mem.u32(CREA + 0x80), e.mem.u32(CREA2 + 0x80)), (SPELL, 0));
}

#[test]
fn skeleton_models_and_templates() {
    let mut e = setup();
    model(&mut e, NPC + 0xdc, Some(b"skel\\a.nif"));
    model(&mut e, NPC2 + 0xdc, Some(b"skel\\b.nif"));
    model(&mut e, CREA + 0xdc, Some(b"skel\\c.nif"));
    let res = call(&mut e, 0x2845, &[r(1)]).1;
    assert_eq!(string_result(&e, res), b"skel\\a.nif");
    // A template with flag 0x40 gives its model; a leveled template stops the walk.
    e.mem.set_u32(NPC + 0x54, NPC2);
    let res = call(&mut e, 0x2845, &[r(1)]).1;
    assert_eq!(string_result(&e, res), b"skel\\a.nif", "no flag");
    e.mem.set_u16(NPC + 0x4a, 0x40);
    let res = call(&mut e, 0x2845, &[r(1)]).1;
    assert_eq!(string_result(&e, res), b"skel\\b.nif");
    e.mem.set_u32(NPC2 + 0x54, LVLC);
    e.mem.set_u16(NPC2 + 0x4a, 0x40);
    let res = call(&mut e, 0x2845, &[r(1)]).1;
    assert_eq!(string_result(&e, res), b"skel\\b.nif", "leveled template");
    e.mem.set_u8(LVLC + 4, 0x2d);
    let res = call(&mut e, 0x2845, &[r(1)]).1;
    assert_eq!(string_result(&e, res), b"skel\\b.nif", "leveled character");
    e.mem.set_u8(LVLC + 4, 0x2c);
    // A circular template chain stops (JIP would hang).
    e.mem.set_u32(NPC2 + 0x54, NPC);
    let res = call(&mut e, 0x2845, &[r(1)]).1;
    assert!(!string_result(&e, res).is_empty());
    e.mem.set_u32(NPC2 + 0x54, 0);
    // Set: an NPC's (through its template) and a creature's.
    call(&mut e, 0x2846, &[r(1), s(b"new.nif")]);
    assert_eq!(text_at(&e, NPC2 + 0xe0), b"new.nif");
    assert_eq!(text_at(&e, NPC + 0xe0), b"skel\\a.nif");
    call(&mut e, 0x2846, &[r(2), s(b"x.nif")]);
    assert_eq!(text_at(&e, CREA + 0xe0), b"skel\\c.nif", "not an NPC");
    call(&mut e, 0x2846, &[r(14)]);
    assert_eq!(e.mem.u16(NPC2 + 0xe4), 0, "left out: emptied");
    call(&mut e, 0x2847, &[r(2), s(b"c2.nif")]);
    assert_eq!(text_at(&e, CREA + 0xe0), b"c2.nif");
    call(&mut e, 0x2847, &[r(14), s(b"y.nif")]);
    assert_eq!(text_at(&e, NPC2 + 0xe0), b"", "not a creature");
}

#[test]
fn weapon_lists_death_items_and_leveled_lists() {
    let mut e = setup();
    e.mem.set_u32(CREA + 0x158, LIST);
    call(&mut e, 0x2857, &[r(2)]);
    assert_eq!(form_result(&e), 0x108);
    call_on(&mut e, 0x2857, ACTOR2, &[]);
    assert_eq!(form_result(&e), 0x108, "the actor's base");
    e.mem.set_u32(NPC + 0x158, LIST);
    call(&mut e, 0x2857, &[r(1)]);
    assert_eq!(form_result(&e), 0, "not a creature");
    call_on(&mut e, 0x2857, PLAIN_REF, &[]);
    assert_eq!(form_result(&e), 0);
    // GetActorBase: a base made in the game gives its leveled creature's.
    e.mem.set_u8(CREA + 0xf, 0xff);
    put_extra(&mut e, ACTOR2 + 0x44, X1, 0x2e, LVLC);
    e.mem.set_u32(X1 + 0x10, CREA2);
    call_on(&mut e, 0x2857, ACTOR2, &[]);
    assert_eq!(form_result(&e), 0, "CREA2 has none");
    // Death items.
    call(&mut e, 0x2869, &[r(10), r(1)]);
    assert_eq!(e.mem.u32(NPC + 0x4c), MISC);
    call(&mut e, 0x2868, &[r(1)]);
    assert_eq!(form_result(&e), 0x110);
    call_on(&mut e, 0x2869, ACTOR2, &[r(8)]);
    assert_eq!(e.mem.u32(CREA2 + 0x4c), WEAP, "GetActorBase");
    call_on(&mut e, 0x2868, ACTOR2, &[]);
    assert_eq!(form_result(&e), 0x10d);
    call_on(&mut e, 0x2869, PLAIN_REF, &[r(8)]);
    call(&mut e, 0x2868, &[r(14)]);
    assert_eq!(form_result(&e), 0);
    // The leveled list: the extra's original base (+0xC)'s template.
    e.mem.set_u32(LVLC + 0x54, 0);
    let lvl_base = NPC2;
    e.mem.set_u32(X1 + 0xc, lvl_base);
    call_on(&mut e, 0x286a, ACTOR2, &[]);
    assert_eq!(form_result(&e), 0, "no template");
    e.mem.set_u32(lvl_base + 0x54, LVLC);
    call_on(&mut e, 0x286a, ACTOR2, &[]);
    assert_eq!(form_result(&e), 0x104);
    e.mem.set_u8(LVLC + 4, 0x2d);
    call_on(&mut e, 0x286a, ACTOR2, &[]);
    assert_eq!(form_result(&e), 0x104);
    e.mem.set_u32(lvl_base + 0x54, NPC);
    call_on(&mut e, 0x286a, ACTOR2, &[]);
    assert_eq!(form_result(&e), 0, "not a leveled list");
    e.mem.set_u32(lvl_base + 0x54, LVLC);
    call_on(&mut e, 0x286a, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "no extra");
    call_on(&mut e, 0x286a, PLAIN_REF, &[]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn impact_materials_and_sound_templates() {
    let mut e = setup();
    e.mem.set_u32(NPC + 0x1e4, 4);
    e.mem.set_u32(CREA + 0x148, 7);
    assert_eq!(call(&mut e, 0x2884, &[r(1)]).1, 4.0);
    assert_eq!(call(&mut e, 0x2884, &[r(2)]).1, 7.0);
    assert_eq!(call_on(&mut e, 0x2884, ACTOR2, &[]).1, 7.0);
    assert_eq!(call_on(&mut e, 0x2884, PLAIN_REF, &[]).1, 0.0);
    call(&mut e, 0x2885, &[n(11), r(1)]);
    assert_eq!(e.mem.u32(NPC + 0x1e4), 11);
    call(&mut e, 0x2885, &[n(12), r(1)]);
    assert_eq!(e.mem.u32(NPC + 0x1e4), 11, "above 11");
    call(&mut e, 0x2885, &[n(-1), r(1)]);
    assert_eq!(e.mem.u32(NPC + 0x1e4), 11, "unsigned");
    call_on(&mut e, 0x2885, ACTOR2, &[n(0)]);
    assert_eq!((e.mem.u32(CREA + 0x148), e.mem.u32(CREA + 0x1e4)), (0, 0));
    // Sounds templates.
    call(&mut e, 0x28bc, &[r(2), r(12)]);
    assert_eq!(e.mem.u32(CREA + 0x128), CREA2);
    call(&mut e, 0x28bb, &[r(2)]);
    assert_eq!(form_result(&e), 0x103);
    call(&mut e, 0x28bc, &[r(2), r(1)]);
    assert_eq!(e.mem.u32(CREA + 0x128), CREA2, "an NPC is no template");
    call(&mut e, 0x28bc, &[r(1), r(12)]);
    assert_eq!(e.mem.u32(NPC + 0x128), 0, "an NPC has none");
    call(&mut e, 0x28bc, &[r(2)]);
    assert_eq!(e.mem.u32(CREA + 0x128), 0, "left out: none");
    call(&mut e, 0x28bb, &[r(2)]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(NPC + 0x128, CREA2);
    call(&mut e, 0x28bb, &[r(1)]);
    assert_eq!(form_result(&e), 0, "not a creature");
}

#[test]
fn body_part_vats_targets() {
    let mut e = setup();
    e.mem.set_u32(A_BPD, BPD);
    e.mem.set_u32(BPD + 0x34 + 4 * 14, BODY_PART);
    string(&mut e, BODY_PART + 0xc, Some(b"Head"));
    let res = call(&mut e, 0x28cf, &[n(14), r(1)]).1;
    assert_eq!(string_result(&e, res), b"Head");
    assert_eq!(called(CALL_BPD), vec![vec![NPC]]);
    let res = call_on(&mut e, 0x28cf, ACTOR2, &[n(14)]).1;
    assert_eq!(string_result(&e, res), b"Head", "the actor's base");
    let res = call(&mut e, 0x28cf, &[n(15), r(1)]).1;
    assert_eq!(string_result(&e, res), b"", "past 14");
    let res = call(&mut e, 0x28cf, &[n(13), r(1)]).1;
    assert_eq!(string_result(&e, res), b"", "no part");
    e.mem.set_u16(BODY_PART + 0x10, 0);
    let res = call(&mut e, 0x28cf, &[n(14), r(1)]).1;
    assert_eq!(string_result(&e, res), b"", "length 0");
    e.mem.set_u16(BODY_PART + 0x10, 4);
    e.mem.set_u32(A_BPD, 0);
    let res = call(&mut e, 0x28cf, &[n(14), r(1)]).1;
    assert_eq!(string_result(&e, res), b"", "no body part data");
    let res = call_on(&mut e, 0x28cf, PLAIN_REF, &[n(14)]).1;
    assert_eq!(string_result(&e, res), b"");
}

fn faction_data(e: &mut Engine, at: u32, faction: u32, rank: i8) -> u32 {
    e.mem.set_u32(at, faction);
    e.mem.set_u8(at + 4, rank as u8);
    at
}

#[test]
fn factions_and_faction_lists() {
    let mut e = setup();
    // Base: FACTION3, FACTION (ranks don't matter); changes: FACTION2 added, FACTION3 removed.
    let d1 = faction_data(&mut e, X3, FACTION3, -1);
    let d2 = faction_data(&mut e, X3 + 8, FACTION, 0);
    node(&mut e, NPC + 0x5c, d1, NODES);
    node(&mut e, NODES, d2, 0);
    let res = call(&mut e, 0x28a6, &[r(1)]).1;
    assert_eq!(
        array_data(&e, res),
        vec![Data::Form(0x105), Data::Form(0x107)],
        "address order"
    );
    let c1 = faction_data(&mut e, X3 + 0x10, FACTION2, 0);
    let c2 = faction_data(&mut e, X3 + 0x18, FACTION3, -1);
    node(&mut e, CHANGES, c1, NODES + 8);
    node(&mut e, NODES + 8, 0, NODES + 0x10);
    node(&mut e, NODES + 0x10, c2, 0);
    put_extra(&mut e, ACTOR + 0x44, X2, 0x5e, CHANGES);
    let res = call_on(&mut e, 0x28a6, ACTOR, &[]).1;
    assert_eq!(
        array_data(&e, res),
        vec![Data::Form(0x105), Data::Form(0x106)]
    );
    // A base given: its own list only.
    let res = call_on(&mut e, 0x28a6, ACTOR, &[r(1)]).1;
    assert_eq!(array_data(&e, res).len(), 2);
    assert_eq!(call(&mut e, 0x28a6, &[r(2)]).1, 0.0, "none: no array");
    assert_eq!(call_on(&mut e, 0x28a6, PLAIN_REF, &[]).1, 0.0);
    // In a list: the faction list's items against the set.
    node(&mut e, LIST + 0x18, MISC, NODES + 0x18);
    node(&mut e, NODES + 0x18, FACTION3, 0);
    assert_eq!(call_on(&mut e, 0x28d9, ACTOR, &[r(4)]).1, 0.0, "removed");
    assert_eq!(eval(&mut e, 0x28d9, ACTOR, LIST), 0.0);
    e.mem.set_u32(NODES + 0x18, FACTION2);
    assert_eq!(call_on(&mut e, 0x28d9, ACTOR, &[r(4)]).1, 1.0);
    assert_eq!(eval(&mut e, 0x28d9, ACTOR, LIST), 1.0);
    assert_eq!(eval(&mut e, 0x28d9, ACTOR, 0), 0.0, "no list");
    assert_eq!(eval(&mut e, 0x28d9, PLAIN_REF, LIST), 0.0);
    assert_eq!(call_on(&mut e, 0x28d9, ACTOR2, &[r(4)]).1, 0.0, "no factions");
}

#[test]
fn cripple_limb_damages_the_condition() {
    let mut e = setup();
    call_on(&mut e, 0x2804, ACTOR, &[n(6), r(13)]);
    call_on(&mut e, 0x2804, ACTOR, &[n(0)]);
    call_on(&mut e, 0x2804, ACTOR, &[n(7)]);
    call_on(&mut e, 0x2804, PLAIN_REF, &[n(1)]);
    let m100 = (-100f32).to_bits();
    assert_eq!(
        called(CALL_DAMAGE),
        vec![vec![ACTOR, 0x1f, m100, ACTOR], vec![ACTOR, 0x19, m100, 0]]
    );
}

#[test]
fn kill_xp_through_the_game() {
    let mut e = setup();
    e.map(0x011d_e000, 0x1000);
    e.mem.set_u32(0x011d_ea3c, PLAYER);
    e.mem.set_u32(PLAYER + 0x7b8, 3);
    e.register(0x0047_ded0, |_, w| {
        CALLED.with(|c| c.borrow_mut().push((0x47ded0, w.to_vec())));
        eax(0xffff_fffe)
    });
    e.register(0x0067_05b0, |_, w| {
        CALLED.with(|c| c.borrow_mut().push((0x6705b0, w.to_vec())));
        eax(25)
    });
    e.register(0x0064_8c80, |_, w| {
        CALLED.with(|c| c.borrow_mut().push((0x648c80, w.to_vec())));
        Ret {
            st0: 37.75,
            ..Ret::default()
        }
    });
    e.register(0x0040_6ce0, |_, w| {
        CALLED.with(|c| c.borrow_mut().push((0x406ce0, w.to_vec())));
        Ret {
            st0: f64::from(f32::from_bits(w[1]).floor()),
            ..Ret::default()
        }
    });
    assert_eq!(call_on(&mut e, 0x2809, ACTOR, &[]).1, 37.0);
    assert_eq!(called(0x47ded0), vec![vec![NPC + 0x30]]);
    // Level -2 sign-extended from 16 bits; 1 for a character (not a creature reference).
    assert_eq!(called(0x6705b0)[0], vec![1, (-2i32) as u32]);
    let xp = 25f32.to_bits();
    assert_eq!(called(0x648c80)[0], vec![xp, 3]);
    assert_eq!(called(0x406ce0)[0], vec![xp, 37.75f32.to_bits()]);
    call_on(&mut e, 0x2809, ACTOR2, &[]);
    assert_eq!(called(0x6705b0)[1][0], 0, "a creature reference");
    assert_eq!(call_on(&mut e, 0x2809, PLAIN_REF, &[]).1, 0.0);
    assert_eq!(called(0x47ded0).len(), 2);
    let (res, lines) = echoed(&mut e, 0x2809, ACTOR, &[]);
    assert_eq!((res, lines), (37.0, vec![b"GetKillXP >> 37".to_vec()]));
}

#[test]
fn the_killer_of_a_dead_actor() {
    let mut e = setup();
    e.mem.set_u32(ACTOR + 0xc0, ACTOR2);
    call_on(&mut e, 0x280b, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "alive");
    for state in [1, 2] {
        e.mem.set_u32(ACTOR + 0x108, state);
        call_on(&mut e, 0x280b, ACTOR, &[]);
        assert_eq!(form_result(&e), 0x201);
    }
    e.mem.set_u32(ACTOR + 0x108, 3);
    call_on(&mut e, 0x280b, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(ACTOR + 0x108, 0x101);
    call_on(&mut e, 0x280b, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "32-bit state");
    e.mem.set_u32(ACTOR + 0x108, 1);
    let (_, lines) = echoed(&mut e, 0x280b, ACTOR, &[]);
    assert_eq!(lines, vec![b"GetKiller >> 201".to_vec()]);
    e.mem.set_u32(ACTOR + 0xc0, 0);
    let (_, lines) = echoed(&mut e, 0x280b, ACTOR, &[]);
    assert_eq!(lines, vec![b"GetKiller >> 0".to_vec()]);
    call_on(&mut e, 0x280b, PLAIN_REF, &[]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn kill_actor_alt_is_the_games_command() {
    let mut e = setup();
    e.register(0x005b_e2a0, |_, w| {
        CALLED.with(|c| c.borrow_mut().push((0x5be2a0, w.to_vec())));
        eax(0x55)
    });
    let words = crate::test_support::line(&mut e, 0x280c, &[r(13), n(3)]);
    let mut w = words;
    w[2] = ACTOR;
    let ok = e.call(crate::test_support::execute_of(&e, 0x280c), &w).eax;
    assert_eq!(ok, 0x55, "the game's answer");
    assert_eq!(called(0x5be2a0), vec![w.to_vec()]);
}

#[test]
fn played_idles() {
    let mut e = setup();
    e.mem.set_u32(A_ANIM, ANIM);
    call_on(&mut e, 0x2816, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "none");
    e.mem.set_u32(ANIM + 0x124, ANIM_P);
    e.mem.set_u32(ANIM_P + 0x2c, IDLE2);
    call_on(&mut e, 0x2816, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x10c, "playing, not done");
    assert_eq!(called(0x4985f0), vec![vec![ANIM]]);
    e.mem.set_u32(A_DONE, 0x101);
    call_on(&mut e, 0x2816, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "done (al 1)");
    e.mem.set_u32(A_DONE, 0x100);
    call_on(&mut e, 0x2816, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x10c, "al 0");
    // A queued idle with a form comes first; one without falls through.
    e.mem.set_u32(ANIM + 0x128, ANIM_Q);
    call_on(&mut e, 0x2816, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x10c);
    e.mem.set_u32(ANIM_Q + 0x2c, IDLE);
    call_on(&mut e, 0x2816, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x10b);
    assert_eq!(call_on(&mut e, 0x2817, ACTOR, &[r(7)]).1, 1.0);
    assert_eq!(eval(&mut e, 0x2817, ACTOR, IDLE), 1.0);
    assert_eq!(eval(&mut e, 0x2817, ACTOR, IDLE2), 0.0);
    e.mem.set_u32(ANIM_Q + 0x2c, 0);
    e.mem.set_u32(ANIM_P + 0x2c, 0);
    call_on(&mut e, 0x2816, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "playing with no form");
    // No animation data: none, and not playing even the null idle.
    e.mem.set_u32(A_ANIM, 0);
    call_on(&mut e, 0x2816, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
    assert_eq!(eval(&mut e, 0x2817, ACTOR, 0), 0.0);
    assert_eq!(eval(&mut e, 0x2817, PLAIN_REF, 0), 0.0);
    assert_eq!(eval(&mut e, 0x2817, 0, 0), 0.0, "no reference");
}

#[test]
fn set_weapon_out_calls_the_game() {
    let mut e = setup();
    answer_word!(e, 0x008a_6840, A_ANIM + 0x104);
    call_on(&mut e, 0x2821, ACTOR, &[n(5)]);
    call_on(&mut e, 0x2821, ACTOR, &[n(0)]);
    call_on(&mut e, 0x2821, PLAIN_REF, &[n(1)]);
    assert_eq!(called(0x8a6840), vec![vec![ACTOR, 1], vec![ACTOR, 0]]);
}

#[test]
fn water_immersion() {
    let mut e = setup();
    e.register(0x0088_5560, |_, w| {
        CALLED.with(|c| c.borrow_mut().push((0x885560, w.to_vec())));
        Ret {
            st0: 0.625,
            ..Ret::default()
        }
    });
    e.mem.set_f32(ACTOR + 0x38, 12.0);
    e.mem.set_u32(ACTOR + 0x40, X3);
    assert_eq!(call_on(&mut e, 0x2830, ACTOR, &[]).1, 0.0, "byte 0x14C clear");
    e.mem.set_u8(ACTOR + 0x14c, 1);
    assert_eq!(call_on(&mut e, 0x2830, ACTOR, &[]).1, 0.625);
    assert_eq!(called(0x885560), vec![vec![ACTOR, 12f32.to_bits(), X3]]);
    assert_eq!(eval(&mut e, 0x2830, ACTOR, 0), 0.625);
    e.mem.set_u32(PROCESS + 0x28, 1);
    assert_eq!(eval(&mut e, 0x2830, ACTOR, 0), 0.0, "not high");
    e.mem.set_u32(PROCESS + 0x28, 0);
    e.mem.set_u32(ACTOR + 0x68, 0);
    assert_eq!(eval(&mut e, 0x2830, ACTOR, 0), 0.0, "no process");
    assert_eq!(eval(&mut e, 0x2830, PLAIN_REF, 0), 0.0);
    assert_eq!(eval(&mut e, 0x2830, 0, 0), 0.0);
}

#[test]
fn interrupt_package_type() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x272a, ACTOR, &[]).1, -1.0);
    e.mem.set_u32(A_INTERRUPT, PACKAGE);
    e.mem.set_u8(PACKAGE + 0x20, 0x1a);
    assert_eq!(call_on(&mut e, 0x272a, ACTOR, &[]).1, 26.0);
    assert_eq!(call_on(&mut e, 0x272a, PLAIN_REF, &[]).1, -1.0);
    e.mem.set_u32(ACTOR + 0x68, 0);
    assert_eq!(call_on(&mut e, 0x272a, ACTOR, &[]).1, -1.0);
}

#[test]
fn fleeing_by_package() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x283c, ACTOR, &[]).1, 0.0, "no package");
    e.mem.set_u32(A_CURRENT, PACKAGE);
    for (t, want) in [(0x0a, 1.0), (0x16, 1.0), (0x12, 0.0), (0x11, 0.0)] {
        e.mem.set_u8(PACKAGE + 0x20, t);
        assert_eq!(call_on(&mut e, 0x283c, ACTOR, &[]).1, want, "{t:x}");
        assert_eq!(eval(&mut e, 0x283c, ACTOR, 0), want, "{t:x}");
    }
    assert_eq!(called(0x981990).len(), 2);
    e.mem.set_u32(A_FLEE, 0x201);
    e.mem.set_u8(PACKAGE + 0x20, 0x12);
    assert_eq!(eval(&mut e, 0x283c, ACTOR, 0), 1.0, "the combat controller says so");
    assert_eq!(called(0x981990)[2], vec![PACKAGE]);
    e.mem.set_u32(A_FLEE, 0x200);
    assert_eq!(eval(&mut e, 0x283c, ACTOR, 0), 0.0, "al only");
    assert_eq!(eval(&mut e, 0x283c, PLAIN_REF, 0), 0.0);
    assert_eq!(eval(&mut e, 0x283c, 0, 0), 0.0);
}

#[test]
fn current_ammo_rounds() {
    let mut e = setup();
    e.mem.set_u32(PROCESS + 0x118, ENTRY);
    call_on(&mut e, 0x283e, ACTOR, &[n(30)]);
    assert_eq!(e.mem.u32(ENTRY + 4), 30);
    call_on(&mut e, 0x283e, ACTOR, &[n(-1)]);
    assert_eq!(e.mem.u32(ENTRY + 4), 30, "negative");
    call_on(&mut e, 0x283e, ACTOR, &[n(0)]);
    assert_eq!(e.mem.u32(ENTRY + 4), 0);
    e.mem.set_u32(PROCESS + 0x28, 0x0100);
    call_on(&mut e, 0x283e, ACTOR, &[n(9)]);
    assert_eq!(e.mem.u32(ENTRY + 4), 9, "a byte compare");
    e.mem.set_u32(PROCESS + 0x28, 2);
    call_on(&mut e, 0x283e, ACTOR, &[n(5)]);
    assert_eq!(e.mem.u32(ENTRY + 4), 9, "low process");
    e.mem.set_u32(PROCESS + 0x28, 1);
    call_on(&mut e, 0x283e, PLAIN_REF, &[n(5)]);
    assert_eq!(e.mem.u32(ENTRY + 4), 9);
    e.mem.set_u32(PROCESS + 0x118, 0);
    call_on(&mut e, 0x283e, ACTOR, &[n(5)]);
}

#[test]
fn unreachable_targets_and_movement_flags() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x2849, ACTOR, &[]).1, 0.0, "no mover");
    e.mem.set_u32(ACTOR + 0x190, MOVER);
    assert_eq!(eval(&mut e, 0x2849, ACTOR, 0), 0.0, "no request");
    e.mem.set_u32(MOVER + 0x1c, X1);
    assert_eq!(call_on(&mut e, 0x2849, ACTOR, &[]).1, 1.0);
    assert_eq!(eval(&mut e, 0x2849, ACTOR, 0), 1.0);
    e.mem.set_u32(MOVER + 0x20, X2);
    assert_eq!(eval(&mut e, 0x2849, ACTOR, 0), 0.0, "solved");
    assert_eq!(eval(&mut e, 0x2849, PLAIN_REF, 0), 0.0);
    e.mem.set_u32(MOVER + 0x20, 0);
    let (res, lines) = echoed(&mut e, 0x2849, ACTOR, &[]);
    assert_eq!((res, lines), (1.0, vec![b"GetTargetUnreachable >> 1".to_vec()]));
    // Movement flags.
    e.mem.set_u32(A_MOVE, 0x880);
    assert_eq!(call_on(&mut e, 0x2908, ACTOR, &[]).1, 2176.0);
    assert_eq!(called(CALL_MOVE_FLAGS), vec![vec![MOVER]]);
    e.mem.set_u32(A_MOVE, 0x8000_0000);
    assert_eq!(call_on(&mut e, 0x2908, ACTOR, &[]).1, -2_147_483_648.0, "(int)");
    let (_, lines) = echoed(&mut e, 0x2908, PLAIN_REF, &[]);
    assert_eq!(lines, vec![b"GetActorMovementFlags >> 0".to_vec()]);
    e.mem.set_u32(ACTOR + 0x190, 0);
    assert_eq!(call_on(&mut e, 0x2908, ACTOR, &[]).1, 0.0);
}

/// ACTOR's container changes: ENTRY for WEAP (worn), ENTRY2 for ARMO (not worn).
fn inventory(e: &mut Engine) {
    put_extra(e, ACTOR + 0x44, X1, 0x15, X2);
    e.mem.set_u32(X2, NODES + 0x40);
    node(e, NODES + 0x40, ENTRY2, NODES + 0x48);
    node(e, NODES + 0x48, ENTRY, 0);
    e.mem.set_u32(ENTRY + 8, WEAP);
    e.mem.set_u32(ENTRY2 + 8, ARMO);
    node(e, ENTRY, NODES + 0x50, 0);
    node(e, NODES + 0x50, XDL, 0);
    put_extra(e, XDL, X3, 0x16, 0);
    node(e, ENTRY2, NODES + 0x58, 0);
    node(e, NODES + 0x58, XDL + 0x40, 0);
}

#[test]
fn the_first_equipped_item_of_a_list() {
    let mut e = setup();
    node(&mut e, LIST + 0x18, 0, NODES);
    node(&mut e, NODES, ARMO, NODES + 8);
    node(&mut e, NODES + 8, WEAP, 0);
    call_on(&mut e, 0x2897, ACTOR, &[r(4)]);
    assert_eq!(form_result(&e), 0, "no inventory");
    inventory(&mut e);
    call_on(&mut e, 0x2897, ACTOR, &[r(4)]);
    assert_eq!(form_result(&e), 0x10d, "the armour is not worn");
    put_extra(&mut e, XDL + 0x40, X3 + 0x40, 0x16, 0);
    call_on(&mut e, 0x2897, ACTOR, &[r(4)]);
    assert_eq!(form_result(&e), 0x10e, "first in the list");
    e.mem.set_u32(NODES, MISC);
    call_on(&mut e, 0x2897, ACTOR, &[r(4)]);
    assert_eq!(form_result(&e), 0x10d, "MISC has no entry");
    call_on(&mut e, 0x2897, PLAIN_REF, &[r(4)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn equipped_slots_against_an_armour() {
    let mut e = setup();
    e.mem.set_u32(ARMO + 0x74, 0b0110);
    e.mem.set_u32(ARMO2 + 0x74, 0b1000);
    assert_eq!(call_on(&mut e, 0x2898, ACTOR, &[r(9)]).1, 0.0, "no biped");
    e.mem.set_u32(A_BIPED, BIPED);
    e.mem.set_u32(BIPED + 0x2c + 0x10 * 19, ARMO2);
    assert_eq!(call_on(&mut e, 0x2898, ACTOR, &[r(9)]).1, 0.0, "no shared bit");
    e.mem.set_u32(ARMO2 + 0x74, 0b1100);
    assert_eq!(call_on(&mut e, 0x2898, ACTOR, &[r(9)]).1, 1.0, "slot 19");
    e.mem.set_u32(BIPED + 0x2c + 0x10 * 19, 0);
    e.mem.set_u32(BIPED + 0x2c + 0x10 * 20, ARMO2);
    assert_eq!(call_on(&mut e, 0x2898, ACTOR, &[r(9)]).1, 0.0, "20 slots");
    e.mem.set_u32(BIPED + 0x2c, WEAP);
    e.mem.set_u32(WEAP + 0x74, u32::MAX);
    assert_eq!(call_on(&mut e, 0x2898, ACTOR, &[r(9)]).1, 0.0, "not an armour");
    assert_eq!(call_on(&mut e, 0x2898, ACTOR, &[r(8)]).1, 0.0, "given no armour");
    e.mem.set_u32(BIPED + 0x2c, ARMO);
    e.mem.set_u32(A_ANIM + 0x100, 0x100);
    assert_eq!(call_on(&mut e, 0x2898, ACTOR, &[r(9)]).1, 0.0, "not a character (al)");
    assert_eq!(call_on(&mut e, 0x2898, PLAIN_REF, &[r(9)]).1, 0.0);
    e.mem.set_u32(A_ANIM + 0x100, 1);
    assert_eq!(call_on(&mut e, 0x2898, ACTOR, &[r(9)]).1, 1.0);
}

#[test]
fn eaten_marks() {
    let mut e = setup();
    e.register(0x0043_0200, |e, w| {
        CALLED.with(|c| c.borrow_mut().push((0x430200, w.to_vec())));
        let x = w[0];
        e.mem.set_u8(x + 4, 0x5f);
        e.mem.set_u32(x + 8, 0);
        e.mem.set_u16(x + 0xc, 0);
        e.mem.set_u8(x + 0x1c, 0);
        eax(x)
    });
    call_on(&mut e, 0x2895, ACTOR, &[n(0)]);
    assert!(calls_to(ADD_CHANGE_FN).is_empty(), "nothing to clear");
    let block = heap_next(&e);
    call_on(&mut e, 0x2895, ACTOR, &[n(2)]);
    assert_eq!(allocations(), vec![0x30]);
    assert_eq!(called(0x430200), vec![vec![block]]);
    assert_eq!(e.mem.u8(block + 0x1c), 1);
    assert_eq!(calls_to(ADD_EXTRA_FN), vec![vec![ACTOR + 0x44, block]]);
    assert_eq!(calls_to(ADD_CHANGE_FN), vec![vec![SAVE_LOAD, ACTOR, 0x2_0000, 0]]);
    call_on(&mut e, 0x2895, ACTOR, &[n(1)]);
    assert_eq!(calls_to(ADD_CHANGE_FN).len(), 1, "already set");
    // Cleared with a limb gone: the mark only.
    e.mem.set_u16(block + 0xc, 4);
    call_on(&mut e, 0x2895, ACTOR, &[n(0)]);
    assert_eq!(e.mem.u8(block + 0x1c), 0);
    assert!(calls_to(REMOVE_BY_TYPE_FN).is_empty());
    assert_eq!(calls_to(ADD_CHANGE_FN).len(), 2);
    call_on(&mut e, 0x2895, ACTOR, &[n(1)]);
    assert_eq!(e.mem.u8(block + 0x1c), 1);
    // Cleared with no limb gone: the extra removed.
    e.mem.set_u16(block + 0xc, 0);
    call_on(&mut e, 0x2895, ACTOR, &[n(0)]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![ACTOR + 0x44, 0x5f]]);
    assert_eq!(calls_to(ADD_CHANGE_FN).len(), 4);
    call_on(&mut e, 0x2895, PLAIN_REF, &[n(1)]);
    assert_eq!(allocations().len(), 1);
}
