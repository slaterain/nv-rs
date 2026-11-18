//! `GetAllPerks`, `GetPlayerPerks`, `RemoveAllPerks`, `GetExcludedCombatActions` and
//! `SetExcludedCombatActions`, called by their opcodes. Expectations from JIP's bodies
//! (functions_jip/jip_fn_actor.h: `GetAllPerks`, the commands; functions_jip/jip_fn_misc_ref.h:
//! `Cmd_GetPlayerPerks_Execute`; internal/patches_game.h: `RemovePerkNPCHook`), never from the
//! port's output.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::Data;

const PC: u32 = FORMS + 0x1000;
const NPC: u32 = FORMS + 0x1400;
const PLAIN: u32 = FORMS + 0x1800;
const ACTOR_VT: u32 = FORMS + 0x2000;
const PLAIN_VT: u32 = FORMS + 0x2400;
const PERKS: u32 = FORMS + 0x3000;
const RANKS: u32 = FORMS + 0x4000;
const NODES: u32 = FORMS + 0x4800;
const CONTROLLER: u32 = FORMS + 0x5000;
const REMOVE_PERK_FN: u32 = 0x0dea_4000;
const GET_CONTROLLER_FN: u32 = 0x0dea_4004;

/// Perk `i` (0x100 apart), hidden when `hidden`, its id 0x900 + i, no entries.
fn perk(e: &mut Engine, i: u32, hidden: bool) -> u32 {
    let at = PERKS + 0x100 * i;
    form(e, at, 0x56, 0x900 + i);
    e.mem.set_u8(at + 0x3c, u8::from(hidden));
    at
}

/// A `tList<PerkRank>` at `head` of `perks` (rank i + 1 each).
fn rank_list(e: &mut Engine, head: u32, perks: &[u32], tag: u32) {
    let mut node = head;
    for (i, &p) in perks.iter().enumerate() {
        let rank = RANKS + tag + 8 * i as u32;
        e.mem.set_u32(rank, p);
        e.mem.set_u8(rank + 4, i as u8 + 1);
        e.mem.set_u32(node, rank);
        let next = if i + 1 < perks.len() {
            NODES + tag + 8 * i as u32
        } else {
            0
        };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
}

fn setup() -> Engine {
    let mut e = engine();
    e.map(0x011d_e000, 0x1000);
    for (at, id, vt) in [
        (PC, 0x14, ACTOR_VT),
        (NPC, 0x7001, ACTOR_VT),
        (PLAIN, 0x7002, PLAIN_VT),
    ] {
        form(&mut e, at, 0x3b, id);
        e.mem.set_u32(at, vt);
    }
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(PLAIN_VT + 0x100, 0x0047_c850);
    e.mem.set_u32(ACTOR_VT + 0x49c, REMOVE_PERK_FN);
    e.mem.set_u32(ACTOR_VT + 0x428, GET_CONTROLLER_FN);
    e.register(REMOVE_PERK_FN, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((REMOVE_PERK_FN, w.to_vec())));
        // The game's removal from the rank list: a match in the head copies the second node into
        // the head (an emptied head when there is none); a later match is unlinked and its next
        // cleared.
        let head = w[0] + if w[2] != 0 { 0xad4 } else { 0x87c };
        let mut prev = 0;
        let mut node = head;
        while node != 0 {
            let rank = e.mem.u32(node);
            if rank != 0 && e.mem.u32(rank) == w[1] {
                let next = e.mem.u32(node + 4);
                if prev == 0 {
                    let (data, after) = if next == 0 {
                        (0, 0)
                    } else {
                        (e.mem.u32(next), e.mem.u32(next + 4))
                    };
                    e.mem.set_u32(head, data);
                    e.mem.set_u32(head + 4, after);
                } else {
                    e.mem.set_u32(prev + 4, next);
                    e.mem.set_u32(node + 4, 0);
                }
                break;
            }
            prev = node;
            node = e.mem.u32(node + 4);
        }
        Ret::default()
    });
    e.register(GET_CONTROLLER_FN, |e, w| Ret {
        eax: if w[0] == NPC {
            e.mem.u32(FORMS + 0x5f00)
        } else {
            0
        },
        ..Ret::default()
    });
    e.mem.set_u32(0x011d_ea3c, PC);
    let (a, b, c, d) = (
        perk(&mut e, 0, false),
        perk(&mut e, 1, true),
        perk(&mut e, 2, false),
        perk(&mut e, 3, false),
    );
    rank_list(&mut e, PC + 0x87c, &[a, b, c], 0);
    rank_list(&mut e, PC + 0xad4, &[d], 0x100);
    e
}

fn forms_of(e: &Engine, id: f64) -> Vec<u32> {
    array_data(e, id)
        .into_iter()
        .map(|d| match d {
            Data::Form(f) => f,
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn the_players_perks_with_and_without_hidden() {
    let mut e = setup();
    let (a, b, c, d) = (PERKS, PERKS + 0x100, PERKS + 0x200, PERKS + 0x300);
    let ids = |v: &[u32]| {
        v.iter()
            .map(|&p| 0x900 + (p - PERKS) / 0x100)
            .collect::<Vec<u32>>()
    };
    let got = call_on(&mut e, 0x2929, PC, &[]).1;
    assert_eq!(
        forms_of(&e, got),
        ids(&[a, c]),
        "hidden left out, the list's order"
    );
    let got = call_on(&mut e, 0x2929, PC, &[n(0), n(1)]).1;
    assert_eq!(forms_of(&e, got), ids(&[a, b, c]));
    let got = call_on(&mut e, 0x2929, PC, &[n(1)]).1;
    assert_eq!(forms_of(&e, got), ids(&[d]), "the teammates' list");
    // GetPlayerPerks: the player's own, no hidden, whoever calls.
    let got = call_on(&mut e, 0x286b, NPC, &[]).1;
    assert_eq!(forms_of(&e, got), ids(&[a, c]));
    // An NPC that is not a teammate, NPC perks off: none (0).
    assert_eq!(call_on(&mut e, 0x2929, NPC, &[n(1)]).1, 0.0);
    // A teammate asked for the teammates' list: the player's teammates' perks.
    e.mem.set_u8(NPC + 0x18d, 1);
    let got = call_on(&mut e, 0x2929, NPC, &[n(1)]).1;
    assert_eq!(forms_of(&e, got), ids(&[d]));
    assert_eq!(
        call_on(&mut e, 0x2929, NPC, &[]).1,
        0.0,
        "not for its own list"
    );
    assert_eq!(call_on(&mut e, 0x2929, PLAIN, &[]).1, 0.0, "not an actor");
}

#[test]
fn an_npcs_own_perks_while_the_patch_is_on() {
    let mut e = setup();
    let (a, b, c) = (PERKS, PERKS + 0x100, PERKS + 0x200);
    crate::cosave::npc_perks(&mut e)
        .emplace(0x7001)
        .ranks
        .extend([(c, 1u8), (a, 2), (b, 1)]);
    assert_eq!(call_on(&mut e, 0x2929, NPC, &[]).1, 0.0, "the patch off");
    crate::cosave::set_npc_perks_patch(&mut e, true);
    let ids = |v: &[u32]| {
        v.iter()
            .map(|&p| 0x900 + (p - PERKS) / 0x100)
            .collect::<Vec<u32>>()
    };
    let got = call_on(&mut e, 0x2929, NPC, &[]).1;
    assert_eq!(
        forms_of(&e, got),
        ids(&[a, c]),
        "by the perk, hidden left out"
    );
    let got = call_on(&mut e, 0x2929, NPC, &[n(0), n(1)]).1;
    assert_eq!(forms_of(&e, got), ids(&[a, b, c]));
    // Removed all: each perk's NPC removal; the entry gone.
    call_on(&mut e, 0x2905, NPC, &[]);
    assert!(crate::cosave::npc_perks(&mut e).get(&0x7001).is_none());
    assert_eq!(call_on(&mut e, 0x2929, NPC, &[]).1, 0.0);
}

#[test]
fn remove_all_perks_for_the_player_calls_remove_perk_for_each() {
    let mut e = setup();
    call_on(&mut e, 0x2905, PC, &[]);
    assert_eq!(
        calls_to(REMOVE_PERK_FN),
        vec![
            vec![PC, PERKS, 0],
            vec![PC, PERKS + 0x100, 0],
            vec![PC, PERKS + 0x200, 0]
        ],
        "A, B and C each, hidden ones too, the own list"
    );
    assert_eq!(
        (e.mem.u32(PC + 0x87c), e.mem.u32(PC + 0x880)),
        (0, 0),
        "the list ends empty"
    );
    call_on(&mut e, 0x2905, PC, &[n(5)]);
    assert_eq!(
        calls_to(REMOVE_PERK_FN)[3],
        vec![PC, PERKS + 0x300, 1],
        "the teammates' list, true"
    );
    assert_eq!(e.mem.u32(PC + 0xad4), 0, "emptied too");
    let before = calls_to(REMOVE_PERK_FN).len();
    call_on(&mut e, 0x2905, PLAIN, &[]);
    call_on(&mut e, 0x2905, NPC, &[]);
    assert_eq!(
        calls_to(REMOVE_PERK_FN).len(),
        before,
        "nothing for a non-actor or a perkless NPC"
    );
}

#[test]
fn excluded_combat_actions_kept_and_answered() {
    let mut e = setup();
    e.mem.set_u32(FORMS + 0x5f00, CONTROLLER);
    assert_eq!(call_on(&mut e, 0x291b, NPC, &[]).1, 0.0);
    call_on(&mut e, 0x291c, NPC, &[n(-1)]);
    assert_eq!(
        call_on(&mut e, 0x291b, NPC, &[]).1,
        f64::from(0x07ff_ffff),
        "27 bits kept"
    );
    assert_eq!(
        e.mem.u32(CONTROLLER + 0x188),
        0x07ff_ffff,
        "the controller's word"
    );
    call_on(&mut e, 0x291c, NPC, &[n(0)]);
    assert_eq!(
        call_on(&mut e, 0x291b, NPC, &[]).1,
        0.0,
        "0: the entry gone"
    );
    assert_eq!(e.mem.u32(CONTROLLER + 0x188), 0);
    call_on(&mut e, 0x291c, PLAIN, &[n(5)]);
    assert_eq!(
        call_on(&mut e, 0x291b, PLAIN, &[]).1,
        0.0,
        "not an actor: nothing kept"
    );
}
