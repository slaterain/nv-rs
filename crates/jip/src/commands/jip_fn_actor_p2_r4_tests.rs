//! `FireWeaponEx`, its queued call `DoFireWeaponEx`, and JIP's queued command call seam
//! (`DoQueuedCmdCallHook` at the task queue's tag 0x2B). Expectations from JIP's bodies
//! (functions_jip/jip_fn_actor.h, internal/hooks.h, internal/jip_core.h `QueuedCmdCall`), never
//! from the port's output. Game functions are stand-ins that record.

use super::*;
use crate::commands::test_support::*;

const ACTOR: u32 = FORMS + 0x1000;
const PLAIN: u32 = FORMS + 0x1200;
const ACTOR_VT: u32 = FORMS + 0x1400;
const PLAIN_VT: u32 = FORMS + 0x1500;
const WEAP: u32 = FORMS + 0x2000;
const MISC: u32 = FORMS + 0x2400;
const PROCESS: u32 = FORMS + 0x3000;
const FLASH: u32 = FORMS + 0x3400;
const QUEUE_IFACE: u32 = FORMS + 0x3800;
const QUEUE: u32 = FORMS + 0x3900;
const LOADED: u32 = FORMS + 0x4000;
const ROOT: u32 = FORMS + 0x4100;
const GUN_NODE: u32 = FORMS + 0x4200;
const KIDS: u32 = FORMS + 0x4300;
const NAMES: u32 = FORMS + 0x4400;
const NODE_VT: u32 = FORMS + 0x4800;
const R_WEAP: u16 = 1;
const R_MISC: u16 = 2;

fn record(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn calls_of(addr: u32) -> Vec<Vec<u32>> {
    calls()
        .into_iter()
        .filter(|c| c.0 == addr)
        .map(|c| c.1)
        .collect()
}

fn setup() -> Engine {
    let mut e = engine();
    e.map(0x011d_f000, 0x1000);
    form(&mut e, ACTOR, 0x3b, 0x0000_7001);
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    form(&mut e, PLAIN, 0x3a, 0x0000_7002);
    e.mem.set_u32(PLAIN, PLAIN_VT);
    e.mem.set_u32(PLAIN_VT + 0x100, 0x0047_c850);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(PLAIN + 0x68, PROCESS);
    form(&mut e, WEAP, 0x28, 0x0000_7003);
    e.mem.set_u32(WEAP + 0x118, FORMS + 0x5000);
    form(&mut e, MISC, 0x1f, 0x0000_7004);
    set_ref(&mut e, R_WEAP, WEAP);
    set_ref(&mut e, R_MISC, MISC);
    e.mem.set_u32(0x011d_f1a8, QUEUE_IFACE);
    e.mem.set_u32(QUEUE_IFACE + 8, QUEUE);
    e.register(0x0090_54a0, |e, w| {
        let words: Vec<u32> = (0..8).map(|i| e.mem.u32(w[1] + 4 * i)).collect();
        record(0x0090_54a0, &[&[w[0]][..], &words[..]].concat());
        Ret::default()
    });
    // The actor's 3D: ROOT "Scene Root" > GUN_NODE "Gun".
    e.mem.set_u32(ACTOR + 0x64, LOADED);
    e.mem.set_u32(LOADED + 0x14, ROOT);
    e.mem.set_u32(NODE_VT + 0xc, 0x0068_15c0);
    for (n, (at, name)) in [(ROOT, &b"Scene Root"[..]), (GUN_NODE, b"Gun")]
        .into_iter()
        .enumerate()
    {
        let text = NAMES + 0x40 * n as u32;
        e.mem.write(text, name);
        e.mem.set_u32(at, NODE_VT);
        e.mem.set_u32(at + 8, text);
    }
    e.mem.set_u32(ROOT + 0xa0, KIDS);
    e.mem.set_u16(ROOT + 0xa6, 1);
    e.mem.set_u32(KIDS, GUN_NODE);
    assert_ne!(
        crate::test_support::execute_of(&e, 0x28e4),
        0,
        "FireWeaponEx is registered with H01"
    );
    e
}

#[test]
fn fire_weapon_ex_queues_the_shot() {
    let mut e = setup();
    assert_eq!(
        call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP)]),
        (1, 0.0),
        "no result"
    );
    assert_eq!(
        calls_of(0x0090_54a0),
        vec![vec![
            QUEUE,
            0x2b,
            DO_FIRE_WEAPON_EX,
            0x0000_7001,
            1,
            WEAP,
            0,
            0,
            0
        ]],
        "the queue, then the call: kind 0x2B, the function, the actor's id, one argument"
    );
    assert_eq!(
        e.mem.u32(PROCESS + 0x130),
        0,
        "no node named: the projectile node left"
    );
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP), s(b"Gun")]);
    assert_eq!(e.mem.u32(PROCESS + 0x130), GUN_NODE, "the named node");
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP), s(b"None")]);
    assert_eq!(e.mem.u32(PROCESS + 0x130), GUN_NODE, "a missing node: left");
    assert_eq!(calls_of(0x0090_54a0).len(), 3);
}

#[test]
fn fire_weapon_ex_refused() {
    let mut e = setup();
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_MISC)]);
    call_on(&mut e, 0x28e4, PLAIN, &[r(R_WEAP)]);
    e.mem.set_u32(WEAP + 0x118, 0);
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP)]);
    e.mem.set_u32(WEAP + 0x118, FORMS + 0x5000);
    e.mem.set_u32(PROCESS + 0x28, 1);
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP)]);
    assert!(
        calls_of(0x0090_54a0).is_empty(),
        "not a weapon, not an actor, no projectile, not high"
    );
}

#[test]
fn do_fire_weapon_ex_fires_with_the_entries_cleared() {
    let mut e = setup();
    e.mem.set_u32(PROCESS + 0x114, 0xaaaa);
    e.mem.set_u32(PROCESS + 0x118, 0xbbbb);
    e.register(0x0052_3150, |e, w| {
        record(
            0x0052_3150,
            &[
                w[0],
                w[1],
                e.mem.u32(PROCESS + 0x114),
                e.mem.u32(PROCESS + 0x118),
            ],
        );
        Ret::default()
    });
    e.mem.set_u32(PROCESS + 0x3d4, FLASH);
    e.call(DO_FIRE_WEAPON_EX, &[ACTOR, 0, WEAP]);
    assert_eq!(calls_of(0x0052_3150), vec![vec![WEAP, ACTOR, 0, 0]]);
    assert_eq!(
        (e.mem.u32(PROCESS + 0x114), e.mem.u32(PROCESS + 0x118)),
        (0xaaaa, 0xbbbb)
    );
    assert_eq!(e.mem.u8(FLASH + 3), 1, "JIP's mark on the muzzle flash");
    e.mem.set_u32(PROCESS + 0x3d4, 0);
    e.call(DO_FIRE_WEAPON_EX, &[ACTOR, 0, WEAP]);
}

#[test]
fn the_queued_call_runs_with_the_reference() {
    let mut e = setup();
    game_globals(&mut e);
    in_form_map(&mut e, ACTOR, 0x0000_7001, 0);
    const FN: u32 = 0x0dea_3000;
    e.register(FN, |_, w| {
        record(FN, w);
        Ret::default()
    });
    let task = FORMS + 0x6000;
    for (i, word) in [0x2b, FN, 0x0000_7001, 2, 0x11, 0x22]
        .into_iter()
        .enumerate()
    {
        e.mem.set_u32(task + 4 * i as u32, word);
    }
    assert_eq!(queued_cmd_call(&mut e, &[task]), Some(0));
    assert_eq!(calls_of(FN), vec![vec![ACTOR, 0, 0x11, 0x22]]);
    // An id with no form: nothing called.
    e.mem.set_u32(task + 8, 0x0000_7999);
    queued_cmd_call(&mut e, &[task]);
    assert_eq!(calls_of(FN).len(), 1);
}

/// H01 with the command: before the first `FireWeaponEx` the muzzle flash seam does not answer and
/// the getter is the game's (a marked flash left as it is); after it, a marked flash that is off
/// is handed over once with the mark and the projectile node cleared, an enabled one is left, and
/// the constructor's seam answers (the mark cleared with the flags).
#[test]
fn h01_turns_on_with_fire_weapon_ex() {
    use crate::game_patches::gw::{GET_MUZZLE_FLASH, MUZZLE_FLAGS};
    let mut e = setup();
    let get = GET_MUZZLE_FLASH.addr();
    e.mem.set_u32(PROCESS + 0x3d4, FLASH);
    e.mem.set_u32(FLASH, 0x0100_0000);
    e.mem.set_u32(PROCESS + 0x130, GUN_NODE);
    assert_eq!(e.site(MUZZLE_FLAGS.addr(), &[FLASH]), None, "off at first");
    assert_eq!(e.call(get, &[PROCESS]).u32(), FLASH);
    assert_eq!(e.mem.u8(FLASH + 3), 1, "off: the mark left");
    assert_eq!(e.mem.u32(PROCESS + 0x130), GUN_NODE);

    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP)]);
    assert_eq!(e.site(MUZZLE_FLAGS.addr(), &[FLASH]), Some(1), "on");
    // Enabled (the first flag byte set): left.
    e.mem.set_u32(FLASH, 0x0100_0001);
    assert_eq!(e.call(get, &[PROCESS]).u32(), FLASH);
    assert_eq!(e.mem.u32(FLASH), 0x0100_0001);
    assert_eq!(e.mem.u32(PROCESS + 0x130), GUN_NODE);
    // The second flag byte set: left too (JIP tests the two bytes as one word).
    e.mem.set_u32(FLASH, 0x0100_0100);
    e.call(get, &[PROCESS]);
    assert_eq!(e.mem.u32(PROCESS + 0x130), GUN_NODE);
    // Off and marked: the mark and the node cleared, the flash answered.
    e.mem.set_u32(FLASH, 0x0100_0000);
    assert_eq!(e.call(get, &[PROCESS]).u32(), FLASH);
    assert_eq!(e.mem.u32(FLASH), 0);
    assert_eq!(e.mem.u32(PROCESS + 0x130), 0);
    // Once: unmarked now, the node set again is left.
    e.mem.set_u32(PROCESS + 0x130, GUN_NODE);
    e.call(get, &[PROCESS]);
    assert_eq!(e.mem.u32(PROCESS + 0x130), GUN_NODE);
    // No flash: none answered.
    e.mem.set_u32(PROCESS + 0x3d4, 0);
    assert_eq!(e.call(get, &[PROCESS]).u32(), 0);
}

/// A refused `FireWeaponEx` (not an actor) leaves H01 off, as JIP writes only past its tests.
#[test]
fn h01_stays_off_after_a_refused_call() {
    use crate::game_patches::gw::MUZZLE_FLAGS;
    let mut e = setup();
    call_on(&mut e, 0x28e4, PLAIN, &[r(R_WEAP)]);
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_MISC)]);
    // No projectile, not in high process, no process: each put back after.
    e.mem.set_u32(WEAP + 0x118, 0);
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP)]);
    e.mem.set_u32(WEAP + 0x118, FORMS + 0x5000);
    e.mem.set_u32(PROCESS + 0x28, 1);
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP)]);
    e.mem.set_u32(PROCESS + 0x28, 0);
    e.mem.set_u32(ACTOR + 0x68, 0);
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP)]);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    assert_eq!(e.site(MUZZLE_FLAGS.addr(), &[FLASH]), None);
    // The same call with every switch back: on (the refusals above were the only reason).
    call_on(&mut e, 0x28e4, ACTOR, &[r(R_WEAP)]);
    assert_eq!(e.site(MUZZLE_FLAGS.addr(), &[FLASH]), Some(1));
}
