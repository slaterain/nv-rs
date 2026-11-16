//! Lane C3's tests of `ln_fn_actor`: `CopyFaceGenFrom`. NPCs at the offsets the module's doc gives
//! (each confirmed in the exe), the game functions as stand-ins that record their words (the flag
//! setter and `CopyAppearance` also change the NPC as the exe's do, so the undo has something to
//! put back). Expectations from JIP's source (functions_ln/ln_fn_actor.h:
//! `Cmd_CopyFaceGenFrom_Execute`; internal/jip_core.h: `AppearanceUndo`; nvse/GameForms.cpp:
//! `TESNPC::SetSex`, `TESNPC::SetRace`; nvse/GameTypes.h: `tList::RemoveAll`, `Prepend`), never
//! from the port's output.

use super::*;
use crate::commands::test_support::*;

const NPC1: u32 = FORMS + 0x1000;
const NPC2: u32 = FORMS + 0x1400;
const NPC3: u32 = FORMS + 0x1800;
const CREA: u32 = FORMS + 0x1c00;
const ACTOR: u32 = FORMS + 0x2000;
const PLAIN: u32 = FORMS + 0x2200;
const PLAYER_REF: u32 = FORMS + 0x2400;
const RACE1: u32 = FORMS + 0x2600;
const RACE2: u32 = FORMS + 0x2640;
const HAIR1: u32 = FORMS + 0x2680;
const HAIR2: u32 = FORMS + 0x26c0;
const EYES1: u32 = FORMS + 0x2700;
const EYES2: u32 = FORMS + 0x2740;
const PART1: u32 = FORMS + 0x2780;
const PART2: u32 = FORMS + 0x27c0;
const PART3: u32 = FORMS + 0x2800;
const NODES: u32 = FORMS + 0x2a00;
const VALUES: u32 = FORMS + 0x3000;
const ACTOR_VT: u32 = FORMS + 0x8000;
const PLAIN_VT: u32 = FORMS + 0x8400;
const CALL_IS_CHAR: u32 = 0x0090_0300;
const CALL_NOT_CHAR: u32 = 0x0090_0304;
const SET_FLAG_BIT_FN: u32 = 0x0047_dd50;
const PLAYER_SET_RACE_FN: u32 = 0x0060_b240;
const COPY_APPEARANCE_FN: u32 = 0x0060_3790;
const RESET_3D_FN: u32 = 0x008d_3fa0;
const EGT_AT: u32 = 0x011d_5ae0;
const PLAYER_AT: u32 = 0x011d_ea3c;
const COUNTS: [u32; 3] = [50, 30, 50];
/// What the stand-ins saw: the destination's race at `CopyAppearance`, the EGT byte at the reset.
const RACE_AT_COPY: u32 = FORMS + 0x7f00;
const EGT_AT_RESET: u32 = FORMS + 0x7f04;

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

fn order() -> Vec<u32> {
    CALLED.with(|c| c.borrow().iter().map(|x| x.0).collect())
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn map_once(e: &mut Engine, at: u32) {
    if !e.mem.is_mapped(at) {
        e.map(at & !0xfff, 0x1000);
    }
}

/// Coordinate `i`'s values of the NPC at `npc` (each NPC's own block in `VALUES`).
fn values_of(npc: u32, i: u32) -> u32 {
    VALUES + (npc - NPC1) + i * 0x100
}

/// The character `ACTOR` (base `NPC1`, `IsCharacter` true) and the plain reference `PLAIN`
/// (false). `NPC1`: male, `RACE1`, face values `1000 * coordinate + index`, `HAIR1` of length
/// 0.5, `EYES1`, colour 0x112233, head parts `PART1`, an empty node, `PART2`, a face NPC
/// `NPC3`. `NPC2` (script reference 1): female, `RACE2`. `CREA` (reference 2) a creature.
/// The player's base is `NPC3`.
fn setup() -> Engine {
    CALLED.with(|c| c.borrow_mut().clear());
    let mut e = engine();
    for at in [EGT_AT, PLAYER_AT] {
        map_once(&mut e, at);
    }
    for (f, t, id) in [
        (NPC1, 0x2a, 0x100),
        (NPC2, 0x2a, 0x101),
        (NPC3, 0x2a, 0x102),
        (CREA, 0x2b, 0x103),
    ] {
        form(&mut e, f, t, id);
    }
    for (r, vt, id) in [
        (ACTOR, ACTOR_VT, 0x200),
        (PLAIN, PLAIN_VT, 0x201),
        (PLAYER_REF, ACTOR_VT, 0x14),
    ] {
        form(&mut e, r, 0x3b, id);
        e.mem.set_u32(r, vt);
    }
    e.mem.set_u32(ACTOR + 0x20, NPC1);
    e.mem.set_u32(PLAIN + 0x20, NPC1);
    e.mem.set_u32(PLAYER_REF + 0x20, NPC3);
    e.mem.set_u32(PLAYER_AT, PLAYER_REF);
    e.mem.set_u32(ACTOR_VT + 0x218, CALL_IS_CHAR);
    e.mem.set_u32(PLAIN_VT + 0x218, CALL_NOT_CHAR);
    e.register(CALL_IS_CHAR, |_, _| eax(0x101));
    e.register(CALL_NOT_CHAR, |_, _| eax(0x100));
    // `SetFlagBit(mask, set, 1)` on the base data: the flags word +4 of it changed.
    e.register(SET_FLAG_BIT_FN, |e, w| {
        record(SET_FLAG_BIT_FN, w);
        let f = e.mem.u32(w[0] + 4);
        e.mem.set_u32(
            w[0] + 4,
            if w[2] & 0xff != 0 {
                f | w[1]
            } else {
                f & !w[1]
            },
        );
        eax(0)
    });
    e.register(PLAYER_SET_RACE_FN, |_, w| {
        record(PLAYER_SET_RACE_FN, w);
        eax(0)
    });
    // `CopyAppearance`: the copied face, hair, eyes and head parts (as the exe writes them).
    e.register(COPY_APPEARANCE_FN, |e, w| {
        record(COPY_APPEARANCE_FN, w);
        let dest = w[0];
        // What the destination's race is when the copy runs.
        e.mem.set_u32(RACE_AT_COPY, e.mem.u32(dest + 0x110));
        for i in 0..3 {
            let v = e.mem.u32(dest + 0x134 + i * 0x20 + 0xc);
            e.mem.set_f32(v, -1.0);
            e.mem.set_f32(v + 4 * (COUNTS[i as usize] - 1), -2.0);
        }
        e.mem.set_u32(dest + 0x1b8, HAIR2);
        e.mem.set_f32(dest + 0x1bc, 0.75);
        e.mem.set_u32(dest + 0x1c0, EYES2);
        e.mem.set_u32(dest + 0x1d8, 0x0044_5566);
        e.mem.set_u32(dest + 0x1dc, PART3);
        e.mem.set_u32(dest + 0x1e0, 0);
        e.mem.set_u32(dest + 0x1f0, w[1]);
        eax(0)
    });
    e.register(RESET_3D_FN, |e, w| {
        record(RESET_3D_FN, w);
        // Whether the EGT setting is on when the reset runs.
        e.mem.set_u8(EGT_AT_RESET, e.mem.u8(EGT_AT));
        eax(0)
    });
    for npc in [NPC1, NPC2] {
        for i in 0..3 {
            e.mem
                .set_u32(npc + 0x134 + i * 0x20 + 0xc, values_of(npc, i));
        }
    }
    for i in 0..3 {
        for j in 0..COUNTS[i as usize] {
            e.mem
                .set_f32(values_of(NPC1, i) + 4 * j, (1000 * i + j) as f32);
        }
    }
    e.mem.set_u32(NPC1 + 0x34, 0x40);
    e.mem.set_u32(NPC1 + 0x110, RACE1);
    e.mem.set_u32(NPC1 + 0x1b8, HAIR1);
    e.mem.set_f32(NPC1 + 0x1bc, 0.5);
    e.mem.set_u32(NPC1 + 0x1c0, EYES1);
    e.mem.set_u32(NPC1 + 0x1d8, 0x0011_2233);
    e.mem.set_u32(NPC1 + 0x1dc, PART1);
    e.mem.set_u32(NPC1 + 0x1e0, NODES);
    e.mem.set_u32(NODES, 0);
    e.mem.set_u32(NODES + 4, NODES + 8);
    e.mem.set_u32(NODES + 8, PART2);
    e.mem.set_u32(NODES + 0xc, 0);
    e.mem.set_u32(NPC1 + 0x1f0, NPC3);
    e.mem.set_u32(NPC2 + 0x34, 0x41);
    e.mem.set_u32(NPC2 + 0x110, RACE2);
    set_ref(&mut e, 1, NPC2);
    set_ref(&mut e, 2, CREA);
    e
}

fn kept(e: &mut Engine) -> Option<crate::cosave::AppearanceUndo> {
    crate::cosave::saved(e).appearance_undo.get(&NPC1).cloned()
}

fn head_parts(e: &Engine, npc: u32) -> Vec<u32> {
    super::super::list_items(e, Ptr::new(npc + 0x1dc))
}

/// The 0x214 bytes JIP keeps: the 50, 30 and 50 face values, then flags, hair length, colour.
fn expected_head() -> Vec<u8> {
    let mut b = Vec::new();
    for i in 0..3u32 {
        for j in 0..COUNTS[i as usize] {
            b.extend_from_slice(&((1000 * i + j) as f32).to_le_bytes());
        }
    }
    b.extend_from_slice(&0x40u32.to_le_bytes());
    b.extend_from_slice(&0.5f32.to_le_bytes());
    b.extend_from_slice(&0x0011_2233u32.to_le_bytes());
    b
}

#[test]
fn copy_face_gen_from_keeps_the_old_face_once_and_copies() {
    let mut e = setup();
    call_on(&mut e, 0x22a1, ACTOR, &[r(1)]);
    let undo = kept(&mut e).expect("the old appearance kept");
    assert_eq!(undo.head.len(), 0x214);
    assert_eq!(undo.head, expected_head());
    assert_eq!(
        (undo.race, undo.hair, undo.eyes),
        (RACE1, HAIR1, EYES1),
        "race +0x110, hair +0x1B8, eyes +0x1C0"
    );
    assert_eq!(
        undo.head_parts,
        vec![PART1, PART2],
        "the empty node left out"
    );
    // `SetSex(src flags)`: bit 0 differs, so `SetFlagBit(1, 1, 1)` on the base data (+0x30).
    assert_eq!(called(SET_FLAG_BIT_FN), vec![vec![NPC1 + 0x30, 1, 1, 1]]);
    // `SetRace`: not the player's base, a plain store.
    assert_eq!(e.mem.u32(NPC1 + 0x110), RACE2);
    assert!(called(PLAYER_SET_RACE_FN).is_empty());
    assert_eq!(called(COPY_APPEARANCE_FN), vec![vec![NPC1, NPC2]]);
    assert_eq!(e.mem.u8(EGT_AT), 1, "bLoadFaceGenHeadEGTFiles set");
    assert_eq!(e.mem.u8(EGT_AT_RESET), 1, "set before the reset");
    assert_eq!(
        e.mem.u32(RACE_AT_COPY),
        RACE2,
        "the race set before CopyAppearance"
    );
    assert_eq!(called(RESET_3D_FN), vec![vec![ACTOR]]);
    assert_eq!(
        order(),
        vec![SET_FLAG_BIT_FN, COPY_APPEARANCE_FN, RESET_3D_FN],
        "sex, race, copy, then the reset"
    );
    // A second copy keeps the first appearance, not the copied one.
    call_on(&mut e, 0x22a1, ACTOR, &[r(1)]);
    assert_eq!(kept(&mut e).expect("still kept").head, expected_head());
    assert_eq!(called(COPY_APPEARANCE_FN).len(), 2);
    assert_eq!(called(SET_FLAG_BIT_FN).len(), 1, "the sex already matches");
}

#[test]
fn copy_face_gen_from_without_an_npc_puts_the_old_face_back() {
    let mut e = setup();
    call_on(&mut e, 0x22a1, ACTOR, &[r(1)]);
    CALLED.with(|c| c.borrow_mut().clear());
    e.mem.set_u8(EGT_AT, 0);
    call_on(&mut e, 0x22a1, ACTOR, &[]);
    assert!(kept(&mut e).is_none(), "taken out of the map");
    assert_eq!(e.mem.u32(NPC1 + 0x1f0), 0, "copyFrom cleared");
    assert_eq!(
        called(SET_FLAG_BIT_FN),
        vec![vec![NPC1 + 0x30, 1, 0, 1]],
        "male again"
    );
    assert_eq!(e.mem.u32(NPC1 + 0x110), RACE1);
    for i in 0..3 {
        let n = COUNTS[i as usize];
        let v = values_of(NPC1, i);
        assert_eq!(
            e.mem.f32(v),
            (1000 * i) as f32,
            "coordinate {i} first value"
        );
        assert_eq!(
            e.mem.f32(v + 4 * (n - 1)),
            (1000 * i + n - 1) as f32,
            "coordinate {i} last"
        );
    }
    assert_eq!(e.mem.f32(NPC1 + 0x1bc), 0.5);
    assert_eq!(e.mem.u32(NPC1 + 0x1d8), 0x0011_2233);
    assert_eq!(e.mem.u32(NPC1 + 0x1b8), HAIR1);
    assert_eq!(e.mem.u32(NPC1 + 0x1c0), EYES1);
    // `RemoveAll`, then each kept part prepended in turn: the reverse order.
    assert_eq!(head_parts(&e, NPC1), vec![PART2, PART1]);
    assert_eq!(e.mem.u8(EGT_AT), 1);
    assert_eq!(called(RESET_3D_FN), vec![vec![ACTOR]]);
    assert!(called(COPY_APPEARANCE_FN).is_empty());
    // Nothing kept any more: no undo, but the flag and the reset all the same.
    CALLED.with(|c| c.borrow_mut().clear());
    e.mem.set_u8(EGT_AT, 0);
    e.mem.set_u32(NPC1 + 0x110, RACE2);
    call_on(&mut e, 0x22a1, ACTOR, &[]);
    assert_eq!(e.mem.u32(NPC1 + 0x110), RACE2, "no undo");
    assert_eq!(order(), vec![RESET_3D_FN]);
    assert_eq!(e.mem.u8(EGT_AT), 1);
}

#[test]
fn copy_face_gen_from_on_the_players_base_changes_the_race_through_the_game() {
    let mut e = setup();
    e.mem.set_u32(PLAYER_REF + 0x20, NPC1);
    call_on(&mut e, 0x22a1, ACTOR, &[r(1)]);
    assert_eq!(called(PLAYER_SET_RACE_FN), vec![vec![NPC1, RACE2, 0]]);
    assert_eq!(
        order(),
        vec![
            SET_FLAG_BIT_FN,
            PLAYER_SET_RACE_FN,
            COPY_APPEARANCE_FN,
            RESET_3D_FN
        ],
        "the race changed before the copy"
    );
    assert_eq!(e.mem.u32(NPC1 + 0x110), RACE1, "no store of its own");
    // The same race: neither.
    CALLED.with(|c| c.borrow_mut().clear());
    e.mem.set_u32(NPC1 + 0x110, RACE2);
    call_on(&mut e, 0x22a1, ACTOR, &[r(1)]);
    assert!(called(PLAYER_SET_RACE_FN).is_empty());
    // No player at all: a plain store.
    e.mem.set_u32(PLAYER_AT, 0);
    e.mem.set_u32(NPC1 + 0x110, RACE1);
    call_on(&mut e, 0x22a1, ACTOR, &[r(1)]);
    assert_eq!(e.mem.u32(NPC1 + 0x110), RACE2);
}

#[test]
fn copy_face_gen_from_needs_a_character_and_an_npc() {
    let mut e = setup();
    // A creature as the source.
    call_on(&mut e, 0x22a1, ACTOR, &[r(2)]);
    // Not a character.
    call_on(&mut e, 0x22a1, PLAIN, &[r(1)]);
    call_on(&mut e, 0x22a1, PLAIN, &[]);
    // No reference.
    call_on(&mut e, 0x22a1, 0, &[r(1)]);
    assert!(order().is_empty(), "no game call");
    assert!(kept(&mut e).is_none());
    assert_eq!(e.mem.u8(EGT_AT), 0, "the flag untouched");
    assert_eq!(e.mem.u32(NPC1 + 0x110), RACE1);
    assert_eq!(call_on(&mut e, 0x22a1, ACTOR, &[r(1)]).1, 0.0, "no result");
}

/// The slot as JIP's `DEFINE_COMMAND_PLUGIN` line and `REG_CMD` make it.
#[test]
fn copy_face_gen_from_table_entry() {
    let e = engine();
    let slot = crate::slots::SLOTS
        .iter()
        .find(|s| s.opcode == 0x22a1)
        .expect("a slot");
    assert_eq!(
        (slot.name, slot.alias, slot.params.name, slot.needs_ref),
        ("CopyFaceGenFrom", "", "kParams_OneOptionalActorBase", true)
    );
    assert_eq!(slot.kind, crate::table::Kind::Command);
    let p = &slot.params.list;
    assert_eq!((p.len(), p[0].kind, p[0].optional), (1, 0x19, true));
    assert!(super::super::is_built(0x22a1));
    assert_ne!(e.mem.u32(nvse::table::record_of(&e, 0x22a1) + 0x18), 0);
}
