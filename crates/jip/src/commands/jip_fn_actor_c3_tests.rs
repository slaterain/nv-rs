//! Lane C3's tests of `jip_fn_actor`: `GetRadiationLevelAlt` (command and condition). Actors and
//! the radiation sources at the offsets the module's doc gives (each confirmed in the exe), the
//! game functions and virtual functions as stand-ins. Expectations from JIP's source
//! (functions_jip/jip_fn_actor.h: `GetRadiationLevelAlt`; nvse/GameObjects.cpp:
//! `Actor::GetRadiationLevel`), never from the port's output; the distance is the one JIP's
//! `TESObjectREFR::GetDistance` gives (`ln_fn_math::distance_3d`, pinned by its own tests).

use super::*;
use crate::commands::test_support::*;

const ACTOR: u32 = FORMS + 0x1000;
const PLAYER_REF: u32 = FORMS + 0x1400;
const PLAIN: u32 = FORMS + 0x1800;
const PROC: u32 = FORMS + 0x2000;
const LOADED: u32 = FORMS + 0x2800;
const WATER_REF: u32 = FORMS + 0x2900;
const PWATER: u32 = FORMS + 0x2a00;
const WATER1: u32 = FORMS + 0x3000;
const WATER2: u32 = FORMS + 0x3400;
const WATER3: u32 = FORMS + 0x3800;
const CELL: u32 = FORMS + 0x4000;
const CELL2: u32 = FORMS + 0x4200;
const COLL: u32 = FORMS + 0x5000;
const BUCKETS: u32 = FORMS + 0x5200;
const ENTRIES: u32 = FORMS + 0x5300;
const EM1: u32 = FORMS + 0x6000;
const EM2: u32 = FORMS + 0x6200;
const EM3: u32 = FORMS + 0x6400;
const EM4: u32 = FORMS + 0x6600;
const EM5: u32 = FORMS + 0x6800;
const EM6: u32 = FORMS + 0x6a00;
const XS: u32 = FORMS + 0x7000;
const ACTOR_VT: u32 = FORMS + 0x8000;
const PLAIN_VT: u32 = FORMS + 0x8400;
const PROC_VT: u32 = FORMS + 0x8800;
const AVO_VT: u32 = FORMS + 0x8c00;
const ANSWERS: u32 = FORMS + 0xc000;
const A_RESIST: u32 = ANSWERS;
const A_RADS_SEC: u32 = ANSWERS + 4;
const A_CELL_WATER: u32 = ANSWERS + 8;
const F_GET_AV: u32 = 0x0090_0300;
const F_RADS_SEC: u32 = 0x0090_0304;
const CELL_WATER_FN: u32 = 0x0054_7770;
const RETURN_TRUE: u32 = 0x008d_0360;
const SWIM: u32 = 0x011d_0460;
const WADE: u32 = 0x011d_1464;
const COLLECTION_AT: u32 = 0x011c_95c8;

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

fn map_once(e: &mut Engine, at: u32) {
    if !e.mem.is_mapped(at) {
        e.map(at & !0xfff, 0x1000);
    }
}

/// An NPC actor (`ACTOR`, id 0x200) and the player (`PLAYER_REF`, id 0x14), each with the level 0
/// process `PROC`, in `CELL` at the origin; the radiation resistance stand-in (`GetActorValue`
/// on +0xA4) answering the float at `A_RESIST`; the game settings swim 2.0, wade 0.5; the loaded
/// references with an empty emitter map of two buckets.
fn setup() -> Engine {
    CALLED.with(|c| c.borrow_mut().clear());
    let mut e = engine();
    extra_stand_ins(&mut e);
    for at in [SWIM, WADE, COLLECTION_AT] {
        map_once(&mut e, at);
    }
    for (a, id) in [(ACTOR, 0x200), (PLAYER_REF, 0x14)] {
        form(&mut e, a, 0x3b, id);
        e.mem.set_u32(a, ACTOR_VT);
        e.mem.set_u32(a + 0x68, PROC);
        e.mem.set_u32(a + 0x40, CELL);
        e.mem.set_u32(a + 0xa4, AVO_VT);
    }
    e.mem.set_u32(ACTOR_VT + 0x100, RETURN_TRUE);
    form(&mut e, PLAIN, 0x3a, 0x201);
    e.mem.set_u32(PLAIN, PLAIN_VT);
    e.mem.set_u32(PLAIN + 0x68, PROC);
    e.mem.set_u32(PLAIN_VT + 0x100, 0x0047_c850);
    e.mem.set_u32(PROC, PROC_VT);
    e.mem.set_u32(PROC_VT + 0x76c, F_RADS_SEC);
    e.mem.set_u32(AVO_VT + 0xc, F_GET_AV);
    e.register(F_GET_AV, |e, w| {
        record(F_GET_AV, w);
        Ret {
            st0: f64::from(e.mem.f32(A_RESIST)),
            ..Ret::default()
        }
    });
    e.register(F_RADS_SEC, |e, w| {
        record(F_RADS_SEC, w);
        Ret {
            st0: f64::from(e.mem.f32(A_RADS_SEC)),
            ..Ret::default()
        }
    });
    e.register(CELL_WATER_FN, |e, w| {
        record(CELL_WATER_FN, w);
        Ret {
            eax: e.mem.u32(A_CELL_WATER),
            ..Ret::default()
        }
    });
    e.mem.set_f32(SWIM + 4, 2.0);
    e.mem.set_f32(WADE + 4, 0.5);
    e.mem.set_u32(COLLECTION_AT, COLL);
    e.mem.set_u32(COLL + 0x94, 2);
    e.mem.set_u32(COLL + 0x98, BUCKETS);
    for (w, rads) in [(WATER1, 10u8), (WATER2, 4), (WATER3, 6)] {
        e.mem.set_u8(w + 0x188, rads);
        // JIP reads the byte: the word's next byte is not part of it.
        e.mem.set_u8(w + 0x189, 1);
    }
    e
}

fn level(e: &mut Engine, this: u32) -> f64 {
    call_on(e, 0x277c, this, &[]).1
}

fn eval(e: &mut Engine, this: u32) -> f64 {
    let at = e.mem.u32(nvse::table::record_of(e, 0x277c) + 0x20);
    e.mem.set_f64(RESULT, -1.0);
    e.call(at, &[this, 0, 0, RESULT]);
    e.mem.f64(RESULT)
}

#[test]
fn the_players_radiation_is_its_process_values() {
    let mut e = setup();
    e.mem.set_f32(PROC + 0x238, 1.5);
    e.mem.set_f32(PROC + 0x23c, 2.25);
    e.mem.set_f32(A_RADS_SEC, 0.125);
    // `waterRadsSec (+0x23C) + rads238 (+0x238) + GetRadsSec()` (vtable +0x76C).
    assert_eq!(level(&mut e, PLAYER_REF), 3.875);
    assert_eq!(eval(&mut e, PLAYER_REF), 3.875, "the condition");
    assert_eq!(called(F_RADS_SEC)[0], vec![PROC]);
    // Single-precision sums in JIP's order: (2^24 + 1) is lost, then 1 is added.
    e.mem.set_f32(PROC + 0x23c, 16_777_216.0);
    e.mem.set_f32(PROC + 0x238, 1.0);
    e.mem.set_f32(A_RADS_SEC, 1.0);
    assert_eq!(level(&mut e, PLAYER_REF), 16_777_216.0);
    assert!(called(F_GET_AV).is_empty(), "no resistance for the player");
    // Level 1: no answer, the condition's 0 written.
    e.mem.set_u32(PROC + 0x28, 1);
    assert_eq!(level(&mut e, PLAYER_REF), 0.0);
    assert_eq!(eval(&mut e, PLAYER_REF), 0.0, "the condition writes 0");
}

#[test]
fn water_radiation_by_the_water_reference_and_swimming() {
    let mut e = setup();
    e.mem.set_f32(A_RESIST, 25.0);
    // Not in water: nothing, and no resistance asked for a 0 result.
    e.mem.set_u32(ACTOR + 0x64, LOADED);
    e.mem.set_u32(LOADED, WATER_REF);
    e.mem.set_u32(WATER_REF + 0x20, PWATER);
    e.mem.set_u32(PWATER + 0x4c, WATER1);
    assert_eq!(level(&mut e, ACTOR), 0.0);
    assert!(called(F_GET_AV).is_empty());
    // Wading: 10 * 0.5, less 25% resistance.
    e.mem.set_u8(ACTOR + 0x14c, 1);
    assert_eq!(level(&mut e, ACTOR), 3.75);
    assert_eq!(
        called(F_GET_AV),
        vec![vec![ACTOR + 0xa4, 0x14]],
        "RadResist on +0xA4"
    );
    // Swimming (+0x14D): 10 * 2.
    e.mem.set_u8(ACTOR + 0x14d, 1);
    assert_eq!(level(&mut e, ACTOR), 15.0);
    assert_eq!(eval(&mut e, ACTOR), 15.0, "the condition");
    // The water form's other form (+0x80) is used when it has one.
    e.mem.set_u32(WATER1 + 0x80, WATER2);
    assert_eq!(level(&mut e, ACTOR), 6.0);
    // No radiation byte: nothing.
    e.mem.set_u8(WATER2 + 0x188, 0);
    assert_eq!(level(&mut e, ACTOR), 0.0);
    // A water reference whose base has no water form: none, and no fall back to the cell's.
    e.mem.set_u32(A_CELL_WATER, WATER3);
    e.mem.set_u32(PWATER + 0x4c, 0);
    assert_eq!(
        level(&mut e, ACTOR),
        0.0,
        "no water form on the reference's base"
    );
    assert!(
        called(CELL_WATER_FN).is_empty(),
        "the reference's water, not the cell's"
    );
}

#[test]
fn water_radiation_by_the_cell_without_a_water_reference() {
    let mut e = setup();
    e.mem.set_u8(ACTOR + 0x14c, 1);
    e.mem.set_u32(A_CELL_WATER, WATER3);
    // The cell's water is used as it is (its +0x80 not followed there).
    e.mem.set_u32(WATER3 + 0x80, WATER1);
    assert_eq!(level(&mut e, ACTOR), 3.0, "no loaded data: 6 * 0.5");
    assert_eq!(called(CELL_WATER_FN), vec![vec![CELL]]);
    e.mem.set_u32(ACTOR + 0x64, LOADED);
    assert_eq!(
        level(&mut e, ACTOR),
        3.0,
        "loaded data with no water reference"
    );
    e.mem.set_u32(A_CELL_WATER, 0);
    assert_eq!(level(&mut e, ACTOR), 0.0, "no water form");
}

/// An emitter at `at` in `cell` at `pos` with an `ExtraRadius` and an `ExtraRadiation` (`None`:
/// left out), its extras at `x`.
fn emitter(
    e: &mut Engine,
    at: u32,
    cell: u32,
    pos: [f32; 3],
    radius: f32,
    rads: Option<f32>,
    x: u32,
) {
    e.mem.set_u32(at + 0x40, cell);
    for (i, v) in pos.into_iter().enumerate() {
        e.mem.set_f32(at + 0x30 + 4 * i as u32, v);
    }
    put_extra(e, at + 0x44, x, 0x5c, radius.to_bits());
    if let Some(r) = rads {
        put_extra(e, at + 0x44, x + 0x10, 0x5d, r.to_bits());
    }
}

fn entry(e: &mut Engine, at: u32, data: u32, next: u32) {
    e.mem.set_u32(at, next);
    e.mem.set_u32(at + 4, 0x1234);
    e.mem.set_u32(at + 8, data);
}

#[test]
fn loaded_emitters_add_by_how_far_inside_their_radius() {
    let mut e = setup();
    e.mem.set_f32(A_RESIST, 10.0);
    // Bucket 0: EM1, EM5, then EM2 (radius only); bucket 1: a null reference, then EM6, EM3
    // (outside its radius) and EM4 (another cell, no worldspace: `FLT_MAX` away).
    emitter(&mut e, EM1, CELL, [30.0, 0.0, 0.0], 100.0, Some(8.0), XS);
    emitter(&mut e, EM2, CELL, [0.0, 10.0, 0.0], 50.0, None, XS + 0x40);
    emitter(
        &mut e,
        EM3,
        CELL,
        [0.0, 0.0, 70.0],
        60.0,
        Some(5.0),
        XS + 0x80,
    );
    emitter(
        &mut e,
        EM4,
        CELL2,
        [0.0, 0.0, 0.0],
        60.0,
        Some(5.0),
        XS + 0xc0,
    );
    // EM5 (second in bucket 0's chain) and EM6 (bucket 1, after the null entry) radiate too.
    emitter(
        &mut e,
        EM5,
        CELL,
        [0.0, 20.0, 0.0],
        40.0,
        Some(3.0),
        XS + 0x100,
    );
    emitter(
        &mut e,
        EM6,
        CELL,
        [0.0, 0.0, -50.0],
        80.0,
        Some(6.0),
        XS + 0x140,
    );
    e.mem.set_u32(BUCKETS, ENTRIES);
    entry(&mut e, ENTRIES, EM1, ENTRIES + 0x10);
    entry(&mut e, ENTRIES + 0x10, EM5, ENTRIES + 0x50);
    entry(&mut e, ENTRIES + 0x50, EM2, 0);
    e.mem.set_u32(BUCKETS + 4, ENTRIES + 0x20);
    entry(&mut e, ENTRIES + 0x20, 0, ENTRIES + 0x60);
    entry(&mut e, ENTRIES + 0x60, EM6, ENTRIES + 0x30);
    entry(&mut e, ENTRIES + 0x30, EM3, ENTRIES + 0x40);
    entry(&mut e, ENTRIES + 0x40, EM4, 0);
    let part = |e: &mut Engine, em: u32, rads: f32, radius: f32| {
        let d = super::super::ln_fn_math::distance_3d(e, ACTOR, em);
        rads * (radius - d) / radius
    };
    let in1 = part(&mut e, EM1, 8.0, 100.0);
    let in5 = part(&mut e, EM5, 3.0, 40.0);
    let in6 = part(&mut e, EM6, 6.0, 80.0);
    assert!(
        in1 > 0.0 && in5 > 0.0 && in6 > 0.0,
        "all three inside their radii"
    );
    // JIP's order: bucket 0's chain (EM1, EM5), then bucket 1 past its null entry (EM6).
    let emitters = 0.0f32 + in1 + in5 + in6;
    let expect = emitters * (1.0 - 10.0f32 * 0.01);
    assert_eq!(
        level(&mut e, ACTOR),
        f64::from(expect),
        "the three emitters"
    );
    assert_eq!(eval(&mut e, ACTOR), f64::from(expect), "the condition");
    // With water: the water's part first, then the emitters', then the resistance.
    e.mem.set_u8(ACTOR + 0x14c, 1);
    e.mem.set_u32(A_CELL_WATER, WATER1);
    let both = (10.0f32 * 0.5 + in1 + in5 + in6) * (1.0 - 10.0f32 * 0.01);
    assert_eq!(level(&mut e, ACTOR), f64::from(both));
    // No loaded references (before the game makes them): the water's part only.
    e.mem.set_u32(COLLECTION_AT, 0);
    assert_eq!(level(&mut e, ACTOR), f64::from(5.0f32 * 0.9));
}

#[test]
fn radiation_alt_needs_an_actor_with_a_high_process() {
    let mut e = setup();
    e.mem.set_u8(ACTOR + 0x14c, 1);
    e.mem.set_u32(A_CELL_WATER, WATER1);
    assert_eq!(level(&mut e, ACTOR), 5.0, "the scene answers");
    e.mem.set_u32(PROC + 0x28, 1);
    assert_eq!(level(&mut e, ACTOR), 0.0, "middle-high process");
    assert_eq!(eval(&mut e, ACTOR), 0.0);
    e.mem.set_u32(PROC + 0x28, 0);
    assert_eq!(level(&mut e, PLAIN), 0.0, "not an actor");
    assert_eq!(eval(&mut e, PLAIN), 0.0);
    assert_eq!(level(&mut e, 0), 0.0, "no reference");
    assert_eq!(eval(&mut e, 0), 0.0);
    e.mem.set_u32(ACTOR + 0x68, 0);
    assert_eq!(level(&mut e, ACTOR), 0.0, "no process");
}

/// The slot as JIP's `DEFINE_CMD_COND_PLUGIN` line and `REG_CMD` make it: a condition with an
/// execute and an eval, no parameters.
#[test]
fn radiation_alt_table_entry() {
    let e = engine();
    let slot = crate::slots::SLOTS
        .iter()
        .find(|s| s.opcode == 0x277c)
        .expect("a slot");
    assert_eq!(
        (slot.name, slot.alias, slot.params.name, slot.needs_ref),
        ("GetRadiationLevelAlt", "", "nullptr", true)
    );
    assert_eq!(slot.kind, crate::table::Kind::Condition);
    assert!(super::super::is_built(0x277c));
    let record = nvse::table::record_of(&e, 0x277c);
    assert_ne!(e.mem.u32(record + 0x18), 0, "an execute");
    assert_ne!(e.mem.u32(record + 0x20), 0, "an eval");
}
