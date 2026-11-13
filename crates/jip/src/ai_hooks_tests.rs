//! Tests of JIP's actor AI hooks: nothing put in while JIP answers "not installed"; installed,
//! each seam's handler as JIP's patched code decides, and the commands through their records with
//! the hooks' user counts, the actors' flag bytes and JIP's maps, and the pre-load housekeeping.
//! Layouts at literal offsets, not the code's: a form's type `+4`; a reference's extra list
//! `+0x44`, 3D `+0x64`, JIP's byte `+0x5F`; an actor's process `+0x68`, magic target `+0x88`,
//! value owner `+0xA4`, in-combat byte `+0x104`, JIP's flag bytes `+0x105`/`+0x106`, combat
//! targets `+0x12C` (data `+4`, count `+8`), teammate byte `+0x18D`; a process's level `+0x28`
//! and held weapon `+0x114`; an entry's extra lists `+0`, count `+4`, form `+8`; an extra list's
//! worn byte `+0xA`; a weapon's ammo `+0xA8` and type `+0xF4`; an armour's enchantment `+0x4C`;
//! a combat style's restrictions `+0x40`; a combat controller's stop byte `+0x125`; an extra's
//! data `+0xC` (combat style) and `+0x10` (script's event list); a script's data length `+0x20`,
//! quest byte `+0x28`, data `+0x30`; a controller's push byte `+0x417` and time `+0x524`;
//! `Character`'s vtable `0x01086A6C`; the console's open byte `0x011DEA2E`.

use super::*;
use crate::test_support::{engine, form_arg, int_arg, line, script_ref, RESULT};

const AREA: u32 = 0x0500_0000;
const ACTOR: u32 = AREA + 0x1000;
const OTHER: u32 = AREA + 0x1400;
const NOT_CHARACTER: u32 = AREA + 0x1800;
const OTHER_VT: u32 = AREA + 0x900;
const SCRIPT_ONE: u32 = AREA + 0x2000;
const SCRIPT_TWO: u32 = AREA + 0x2100;
const QUEST_SCRIPT: u32 = AREA + 0x2200;
const DATA: u32 = AREA + 0x2400;
const LIST_FORM: u32 = AREA + 0x2800;
const NODES: u32 = AREA + 0x2900;
const PROCESS: u32 = AREA + 0x3000;
const CONTROLLER: u32 = AREA + 0x3400;
const WEAPON: u32 = AREA + 0x4000;
const AMMO: u32 = AREA + 0x4100;
const ARMOR: u32 = AREA + 0x4200;
const ENTRY: u32 = AREA + 0x4400;
const WEAPONS: u32 = AREA + 0x4800;
const EXTRAS: u32 = AREA + 0x5000;
const TARGETS: u32 = AREA + 0x5800;
const CHARACTER_VT: u32 = 0x0108_6a6c;
const CONSOLE: u32 = 0x011d_ea2e;

thread_local! {
    /// The game functions' calls the tests' stand-ins saw: (function, words).
    static CALLS: std::cell::RefCell<Vec<(u32, Vec<u32>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn calls() -> Vec<(u32, Vec<u32>)> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn record(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

/// Extras found by `GetByType` (stand-in): (reference, type) -> extra.
fn set_extra(e: &mut Engine, reference: u32, kind: u32, extra: u32) {
    let at = EXTRAS + (reference - AREA) / 0x10 + kind * 4;
    e.mem.set_u32(at, extra);
}

fn world_as(installed: bool) -> Engine {
    calls();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    e.map(0x0108_6000, 0x1000);
    e.mem.set_u32(CHARACTER_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(OTHER_VT + 0x100, 0x008d_0360);
    for (actor, vt) in [
        (ACTOR, CHARACTER_VT),
        (OTHER, CHARACTER_VT),
        (NOT_CHARACTER, OTHER_VT),
    ] {
        e.mem.set_u32(actor, vt);
        e.mem.set_u8(actor + 4, 0x3c);
    }
    for script in [SCRIPT_ONE, SCRIPT_TWO, QUEST_SCRIPT] {
        e.mem.set_u8(script + 4, 0x11);
        e.mem.set_u32(script, AREA + 0xa00);
        e.mem.set_u32(script + 0x30, DATA);
    }
    e.mem.set_u8(QUEST_SCRIPT + 0x28, 1);
    // `scn` (0x1D), one block (0x10, lengths 4 and 8 at +2 and +6: 12 bytes), `End` (0x11).
    e.mem.set_u32(DATA, 0x1d);
    e.mem.set_u16(DATA + 4, 0x10);
    e.mem.set_u16(DATA + 6, 4);
    e.mem.set_u16(DATA + 10, 8);
    e.mem.set_u32(DATA + 16, 0x11);
    for script in [SCRIPT_ONE, SCRIPT_TWO, QUEST_SCRIPT] {
        e.mem.set_u32(script + 0x20, 20);
    }
    e.mem.set_u8(LIST_FORM + 4, 0x55);
    e.mem.set_u32(LIST_FORM + 0x18, SCRIPT_ONE);
    e.mem.set_u32(LIST_FORM + 0x1c, NODES);
    e.mem.set_u32(NODES, QUEST_SCRIPT);
    e.mem.set_u8(WEAPON + 4, 0x28);
    e.mem.set_u8(ARMOR + 4, 0x18);
    e.register(0x0041_0220, |e, w| Ret {
        eax: e.mem.u32(EXTRAS + (w[0] - 0x44 - AREA) / 0x10 + w[1] * 4),
        ..Ret::default()
    });
    for f in [0x0088_b740u32, 0x0088_c650, 0x0044_59e0] {
        e.register_double(f, move |_, w| {
            record(f, w);
            Ret::default()
        });
    }
    crate::install_as(&mut e, installed);
    for (i, form) in [
        ACTOR,
        OTHER,
        SCRIPT_ONE,
        SCRIPT_TWO,
        QUEST_SCRIPT,
        LIST_FORM,
        NOT_CHARACTER,
    ]
    .into_iter()
    .enumerate()
    {
        script_ref(&mut e, i as u16 + 1, form);
    }
    e
}

fn world() -> Engine {
    world_as(true)
}

/// Form argument indices of `world`.
const A2: u16 = 2;
const S1: u16 = 3;
const S2: u16 = 4;
const SQ: u16 = 5;
const LIST: u16 = 6;

/// Runs `opcode` with `args` through its record on `this`; the result.
fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let mut words = line(e, opcode, args);
    words[2] = this;
    let at = crate::test_support::execute_of(e, opcode);
    assert_eq!(e.call(at, &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}

fn float_arg(v: f64) -> Vec<u8> {
    let mut b = vec![b'z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

fn site(e: &mut Engine, addr: u32, args: &[u32]) -> Option<u32> {
    e.site(addr, args)
}

fn flags1(e: &Engine, actor: u32) -> u8 {
    e.mem.u8(actor + 0x105)
}

/// While JIP answers "not installed": no seam has a handler of its, the commands have no handler.
#[test]
fn not_installed_nothing_is_put_in() {
    let e = world_as(false);
    for addr in SITES {
        assert!(e.site_owners(addr).is_empty(), "{addr:08x}");
    }
    for op in COMMANDS {
        assert_eq!(crate::test_support::execute_of(&e, op), 0, "{op:04x}");
    }
}

/// A command adds users only while every site of its hooks has a JIP seam, and every handler
/// here is the only one at its seam.
#[test]
fn every_command_hook_has_all_its_seams() {
    let e = world();
    for addr in SITES {
        assert_eq!(e.site_owners(addr), [Owner::Jip], "{addr:08x}");
    }
    for (op, sites) in COMMAND_SITES {
        for &addr in sites {
            assert!(
                engine_core::extend::SITES
                    .iter()
                    .any(|s| s.addr == addr && s.owners.iter().any(|o| matches!(o, Owner::Jip))),
                "{op:04x}: {addr:08x} has no JIP seam"
            );
        }
    }
}

/// SetCombatDisabled and GetCombatDisabled, with the StartCombat and SetCombatTarget seams.
#[test]
fn combat_disabled_and_forced_targets() {
    let mut e = world();
    assert_eq!(run(&mut e, GET_COMBAT_DISABLED, ACTOR, &[]), 0.0);
    assert_eq!(site(&mut e, 0x0090_01c6, &[ACTOR]), None);
    // Disabled while in combat: the combat controller told to stop.
    let controller = AREA + 0x6000;
    e.mem.set_u8(ACTOR + 0x104, 1);
    e.mem.set_u32(CHARACTER_VT + 0x428, 0x00de_0428);
    e.register_double(0x00de_0428, move |_, _| Ret {
        eax: controller,
        ..Ret::default()
    });
    run(&mut e, SET_COMBAT_DISABLED, ACTOR, &[int_arg(1)]);
    assert_eq!(e.mem.u8(controller + 0x125), 1);
    assert_eq!((flags1(&e, ACTOR), e.hook_users(Hook::StartCombat)), (1, 1));
    assert_eq!(run(&mut e, GET_COMBAT_DISABLED, ACTOR, &[]), 1.0);
    assert_eq!(site(&mut e, 0x0090_01c6, &[ACTOR]), Some(0));
    assert_eq!(site(&mut e, 0x0090_01c6, &[OTHER]), None);
    run(&mut e, SET_COMBAT_DISABLED, ACTOR, &[int_arg(1)]);
    assert_eq!(e.hook_users(Hook::StartCombat), 1, "already disabled");
    // Not in combat: nothing to stop.
    e.mem.set_u8(controller + 0x125, 0);
    e.mem.set_u8(OTHER + 0x104, 0);
    run(&mut e, SET_COMBAT_DISABLED, OTHER, &[int_arg(1)]);
    assert_eq!(e.mem.u8(controller + 0x125), 0);
    assert_eq!(e.hook_users(Hook::StartCombat), 2);
    // Switched to a forced target: the user moves to the other hook.
    run(
        &mut e,
        SET_COMBAT_DISABLED,
        ACTOR,
        &[int_arg(1), form_arg(A2)],
    );
    assert_eq!(flags1(&e, ACTOR), 2);
    assert_eq!(
        (
            e.hook_users(Hook::StartCombat),
            e.hook_users(Hook::SetCombatTarget)
        ),
        (1, 1)
    );
    assert_eq!(state(&mut e).force_combat_target[&ACTOR], OTHER);
    // The target only while it is among the actor's combat targets.
    assert_eq!(site(&mut e, 0x0098_6c79, &[ACTOR]), None);
    e.mem.set_u32(ACTOR + 0x12c, TARGETS);
    e.mem.set_u32(TARGETS + 4, TARGETS + 0x10);
    e.mem.set_u32(TARGETS + 8, 2);
    e.mem.set_u32(TARGETS + 0x10, NOT_CHARACTER);
    assert_eq!(site(&mut e, 0x0098_6c79, &[ACTOR]), None);
    e.mem.set_u32(TARGETS + 0x14, OTHER);
    assert_eq!(site(&mut e, 0x0098_6c79, &[ACTOR]), Some(OTHER));
    e.mem.set_u32(TARGETS + 8, 1);
    assert_eq!(site(&mut e, 0x0098_6c79, &[ACTOR]), None, "past the count");
    e.mem.set_u32(TARGETS + 8, 2);
    e.mem.set_u8(ACTOR + 0x105, 0);
    assert_eq!(site(&mut e, 0x0098_6c79, &[ACTOR]), None, "the flag clear");
    e.mem.set_u8(ACTOR + 0x105, 2);
    // Back to disabled: the target goes.
    run(&mut e, SET_COMBAT_DISABLED, ACTOR, &[int_arg(1)]);
    assert!(state(&mut e).force_combat_target.is_empty());
    assert_eq!(
        (
            e.hook_users(Hook::StartCombat),
            e.hook_users(Hook::SetCombatTarget)
        ),
        (2, 0)
    );
    // Off: each user back once; off again gives nothing back.
    run(&mut e, SET_COMBAT_DISABLED, ACTOR, &[int_arg(0)]);
    run(&mut e, SET_COMBAT_DISABLED, ACTOR, &[int_arg(-1)]);
    assert_eq!((flags1(&e, ACTOR), e.hook_users(Hook::StartCombat)), (0, 1));
    run(
        &mut e,
        SET_COMBAT_DISABLED,
        OTHER,
        &[int_arg(1), form_arg(A2)],
    );
    run(&mut e, SET_COMBAT_DISABLED, OTHER, &[int_arg(0)]);
    assert_eq!(
        (
            e.hook_users(Hook::StartCombat),
            e.hook_users(Hook::SetCombatTarget)
        ),
        (0, 0)
    );
    assert!(state(&mut e).force_combat_target.is_empty());
    // A first forced target is one user of the target hook; no actor: nothing.
    run(
        &mut e,
        SET_COMBAT_DISABLED,
        ACTOR,
        &[int_arg(1), form_arg(A2)],
    );
    assert_eq!(e.hook_users(Hook::SetCombatTarget), 1);
    run(&mut e, SET_COMBAT_DISABLED, SCRIPT_ONE, &[int_arg(1)]);
    assert_eq!(e.hook_users(Hook::StartCombat), 0);
}

/// ToggleNoHealthReset and its two seams: the command's reset skipped for a teammate's own
/// script, and the wake-up's restore as JIP computes it.
#[test]
fn no_health_reset() {
    let mut e = world();
    run(
        &mut e,
        TOGGLE_NO_HEALTH_RESET,
        0,
        &[int_arg(1), float_arg(0.5)],
    );
    assert_eq!(e.hook_users(Hook::NoHealthReset), 1);
    assert_eq!(state(&mut e).health_restore_ratio, 0.5);
    run(
        &mut e,
        TOGGLE_NO_HEALTH_RESET,
        0,
        &[int_arg(1), float_arg(0.25)],
    );
    assert_eq!(e.hook_users(Hook::NoHealthReset), 1, "set, not counted");
    assert_eq!(state(&mut e).health_restore_ratio, 0.25);
    // The command's seam.
    let (list, extra, node, event) = (AREA + 0x6100, AREA + 0x6200, AREA + 0x6300, AREA + 0x6400);
    set_extra(&mut e, ACTOR, 0x0d, extra);
    e.mem.set_u32(extra + 0x10, list);
    e.mem.set_u32(list + 8, node);
    e.mem.set_u32(node, event);
    e.mem.set_u32(event, ACTOR);
    e.mem.set_u8(event + 4, 0x40);
    assert_eq!(
        site(&mut e, 0x005c_6b8b, &[ACTOR, list]),
        None,
        "not a teammate"
    );
    e.mem.set_u8(ACTOR + 0x18d, 1);
    assert_eq!(site(&mut e, 0x005c_6b8b, &[ACTOR, list]), Some(1));
    assert_eq!(site(&mut e, 0x005c_6b8b, &[ACTOR, 0]), None, "no list");
    assert_eq!(
        site(&mut e, 0x005c_6b8b, &[ACTOR, list + 4]),
        None,
        "another list"
    );
    e.mem.set_u8(CONSOLE, 1);
    assert_eq!(
        site(&mut e, 0x005c_6b8b, &[ACTOR, list]),
        None,
        "the console open"
    );
    e.mem.set_u8(CONSOLE, 0);
    e.mem.set_u32(event, OTHER);
    assert_eq!(
        site(&mut e, 0x005c_6b8b, &[ACTOR, list]),
        None,
        "another object"
    );
    e.mem.set_u32(event, 0);
    assert_eq!(
        site(&mut e, 0x005c_6b8b, &[ACTOR, list]),
        Some(1),
        "no object"
    );
    e.mem.set_u8(event + 4, 0x3f);
    assert_eq!(
        site(&mut e, 0x005c_6b8b, &[ACTOR, list]),
        None,
        "no 0x40 bit"
    );
    // A first event without the bit, a second with: the walk goes on.
    let (node2, event2) = (AREA + 0x6500, AREA + 0x6600);
    e.mem.set_u32(node + 4, node2);
    e.mem.set_u32(node2, event2);
    e.mem.set_u32(event2, ACTOR);
    e.mem.set_u8(event2 + 4, 0x40);
    assert_eq!(site(&mut e, 0x005c_6b8b, &[ACTOR, list]), Some(1));
    e.mem.set_u32(node, 0);
    assert_eq!(
        site(&mut e, 0x005c_6b8b, &[ACTOR, list]),
        None,
        "an empty node ends it"
    );
    set_extra(&mut e, ACTOR, 0x0d, 0);
    assert_eq!(site(&mut e, 0x005c_6b8b, &[ACTOR, list]), None, "no script");
    // The wake-up's seam: base 100 * 0.25 - 10 = 15 restored.
    let owner_vt = AREA + 0x6800;
    e.mem.set_u32(ACTOR + 0xa4, owner_vt);
    e.mem.set_u32(owner_vt + 4, 0x00de_0004);
    e.mem.set_u32(owner_vt + 0xc, 0x00de_000c);
    e.register_double(0x00de_0004, |_, w| {
        assert_eq!(w[1], 0x10);
        Ret {
            st0: 100.0,
            ..Ret::default()
        }
    });
    e.register_double(0x00de_000c, |_, _| Ret {
        st0: 10.0,
        ..Ret::default()
    });
    calls();
    assert_eq!(site(&mut e, 0x008a_0963, &[ACTOR]), Some(1));
    assert_eq!(
        calls(),
        [(0x0088_b740, vec![ACTOR, 0x10, 15.0f32.to_bits()])]
    );
    e.mem.set_u8(ACTOR + 0x18d, 0);
    assert_eq!(site(&mut e, 0x008a_0963, &[ACTOR]), None);
    assert!(calls().is_empty());
    run(
        &mut e,
        TOGGLE_NO_HEALTH_RESET,
        0,
        &[int_arg(0), float_arg(1.0)],
    );
    assert_eq!(e.hook_users(Hook::NoHealthReset), 0);
}

/// CCCSetFollowState and the teleport seam.
#[test]
fn follow_and_wait_orders() {
    let mut e = world();
    assert_eq!(
        site(&mut e, 0x008a_d471, &[ACTOR]),
        Some(0),
        "not a teammate"
    );
    e.mem.set_u8(ACTOR + 0x18d, 1);
    assert_eq!(site(&mut e, 0x008a_d471, &[ACTOR]), None);
    run(&mut e, CCC_SET_FOLLOW_STATE, ACTOR, &[int_arg(0)]);
    assert_eq!(
        (flags1(&e, ACTOR), e.hook_users(Hook::TeleportWithPC)),
        (0x20, 1)
    );
    assert_eq!(site(&mut e, 0x008a_d471, &[ACTOR]), Some(1), "wait");
    run(&mut e, CCC_SET_FOLLOW_STATE, ACTOR, &[int_arg(0)]);
    run(&mut e, CCC_SET_FOLLOW_STATE, ACTOR, &[int_arg(1)]);
    assert_eq!(
        (flags1(&e, ACTOR), e.hook_users(Hook::TeleportWithPC)),
        (0x40, 1)
    );
    assert_eq!(site(&mut e, 0x008a_d471, &[ACTOR]), Some(0), "follow");
    run(&mut e, CCC_SET_FOLLOW_STATE, OTHER, &[int_arg(1)]);
    assert_eq!(e.hook_users(Hook::TeleportWithPC), 2);
    run(&mut e, CCC_SET_FOLLOW_STATE, ACTOR, &[int_arg(-1)]);
    run(&mut e, CCC_SET_FOLLOW_STATE, ACTOR, &[int_arg(-1)]);
    assert_eq!(
        (flags1(&e, ACTOR), e.hook_users(Hook::TeleportWithPC)),
        (0, 1)
    );
    run(&mut e, CCC_SET_FOLLOW_STATE, SCRIPT_ONE, &[int_arg(1)]);
    assert_eq!(e.hook_users(Hook::TeleportWithPC), 1, "not an actor");
}

/// LockEquipment and the equipment seams.
#[test]
fn locked_equipment() {
    let mut e = world();
    run(&mut e, LOCK_EQUIPMENT, NOT_CHARACTER, &[int_arg(1)]);
    assert_eq!(e.hook_users(Hook::LockEquipment), 0, "not a character");
    run(&mut e, LOCK_EQUIPMENT, ACTOR, &[int_arg(1)]);
    run(&mut e, LOCK_EQUIPMENT, ACTOR, &[int_arg(1)]);
    assert_eq!(
        (flags1(&e, ACTOR), e.hook_users(Hook::LockEquipment)),
        (8, 1)
    );
    // EquipObject: a weapon is refused, unless JIP forces the call or ResolveEquipment marked it.
    assert_eq!(site(&mut e, 0x0088_c87a, &[WEAPON, ACTOR]), Some(0));
    assert_eq!(site(&mut e, 0x0088_c87a, &[ARMOR, ACTOR]), None);
    assert_eq!(site(&mut e, 0x0088_c87a, &[WEAPON, OTHER]), None);
    {
        let _mark = e.mark_caller(Hook::LockEquipment, 0x0091_559d);
        assert_eq!(site(&mut e, 0x0088_c87a, &[ARMOR, ACTOR]), None);
        assert!(!e.caller_marked(0x0091_559d), "taken first");
    }
    {
        let _mark = e.mark_caller(Hook::LockEquipment, 0x0091_559d);
        assert_eq!(site(&mut e, 0x0088_c87a, &[WEAPON, ACTOR]), None);
    }
    state(&mut e).force_equip_call = true;
    assert_eq!(site(&mut e, 0x0088_c87a, &[WEAPON, ACTOR]), None);
    state(&mut e).force_equip_call = false;
    // The switch procedure's unequip, and the preferred weapon.
    assert_eq!(site(&mut e, 0x009d_a8e3, &[ACTOR]), Some(0));
    assert_eq!(site(&mut e, 0x009d_a8e3, &[OTHER]), None);
    assert_eq!(site(&mut e, 0x0089_1c86, &[ACTOR, 6]), Some(0));
    assert_eq!(site(&mut e, 0x0089_1c86, &[OTHER, 6]), None);
    {
        let _mark = e.mark_caller(Hook::LockEquipment, JIP_PREFERED_WEAPON_CALL);
        assert_eq!(
            site(&mut e, 0x0089_1c86, &[ACTOR, 6]),
            None,
            "JIP's own call"
        );
    }
    // The weapon rating: only the acquired weapon, when its ammo is there.
    e.mem.set_u32(ENTRY + 8, WEAPON);
    e.mem.set_u32(WEAPONS + 4, WEAPONS + 0x10);
    e.mem.set_u32(WEAPONS + 8, 3);
    for (i, w) in [ARMOR, WEAPON, AMMO].into_iter().enumerate() {
        e.mem.set_u32(WEAPONS + 0x10 + i as u32 * 0x14, w);
    }
    assert_eq!(site(&mut e, 0x0099_9857, &[ACTOR, ENTRY, WEAPONS]), Some(1));
    assert_eq!(e.mem.u32(WEAPONS + 8), 2, "cut after it");
    e.mem.set_u32(WEAPONS + 8, 1);
    assert_eq!(
        site(&mut e, 0x0099_9857, &[ACTOR, ENTRY, WEAPONS]),
        None,
        "not listed"
    );
    assert_eq!(e.mem.u32(WEAPONS + 8), 1);
    e.mem.set_u32(WEAPONS + 8, 3);
    assert_eq!(
        site(&mut e, 0x0099_9857, &[ACTOR, 0, WEAPONS]),
        None,
        "none acquired"
    );
    assert_eq!(
        site(&mut e, 0x0099_9857, &[OTHER, ENTRY, WEAPONS]),
        None,
        "not locked"
    );
    e.mem.set_u32(WEAPON + 0xa8, AMMO);
    assert_eq!(
        site(&mut e, 0x0099_9857, &[ACTOR, ENTRY, WEAPONS]),
        None,
        "no ammo"
    );
    assert_eq!(e.mem.u32(WEAPONS + 8), 3);
    run(&mut e, LOCK_EQUIPMENT, ACTOR, &[int_arg(0)]);
    run(&mut e, LOCK_EQUIPMENT, ACTOR, &[int_arg(0)]);
    assert_eq!(
        (flags1(&e, ACTOR), e.hook_users(Hook::LockEquipment)),
        (0, 0)
    );
}

/// ReEquipAll for a locked actor: from SetCombatStyle the preferred weapon (asked under JIP's
/// own mark), unless the held weapon suits the style; else the worn armour's enchantments again
/// for an actor with no 3D.
#[test]
fn locked_re_equip_all() {
    let mut e = world();
    run(&mut e, LOCK_EQUIPMENT, ACTOR, &[int_arg(1)]);
    assert_eq!(site(&mut e, 0x0060_47c0, &[0, OTHER]), None, "not locked");
    // Not from SetCombatStyle: enchantments on the worn armour.
    let (extra_cc, data, node, entry, lists, worn) = (
        AREA + 0x7100,
        AREA + 0x7200,
        AREA + 0x7300,
        AREA + 0x7400,
        AREA + 0x7500,
        AREA + 0x7600,
    );
    set_extra(&mut e, ACTOR, 0x15, extra_cc);
    e.mem.set_u32(extra_cc + 0xc, data);
    e.mem.set_u32(data, node);
    e.mem.set_u32(node, entry);
    e.mem.set_u32(entry + 8, ARMOR);
    e.mem.set_u32(ARMOR + 0x4c, AREA + 0x7800);
    e.mem.set_u32(entry, lists);
    e.mem.set_u32(lists, worn);
    e.mem.set_u8(worn + 0xa, 0x40);
    let magic_vt = AREA + 0x7900;
    e.mem.set_u32(ACTOR + 0x88, magic_vt);
    e.mem.set_u32(magic_vt + 0x14, 0x00de_0014);
    e.register(0x00de_0014, |_, w| {
        record(0x00de_0014, w);
        Ret::default()
    });
    assert_eq!(site(&mut e, 0x0060_47c0, &[0, ACTOR]), Some(0));
    assert_eq!(
        calls(),
        [(0x00de_0014, vec![ACTOR + 0x88, AREA + 0x7818, ARMOR, 0])]
    );
    e.mem.set_u8(worn + 0xa, 0);
    site(&mut e, 0x0060_47c0, &[0, ACTOR]);
    assert!(calls().is_empty(), "not worn");
    e.mem.set_u8(worn + 0xa, 0x40);
    e.mem.set_u32(ACTOR + 0x64, 1);
    assert_eq!(site(&mut e, 0x0060_47c0, &[0, ACTOR]), Some(0));
    assert!(calls().is_empty(), "has 3D");
    e.mem.set_u32(ACTOR + 0x64, 0);
    e.mem.set_u8(ACTOR + 0x5f, 1);
    site(&mut e, 0x0060_47c0, &[0, ACTOR]);
    assert!(calls().is_empty(), "waiting for Update3D");
    e.mem.set_u8(ACTOR + 0x5f, 0);
    e.mem.set_u8(ARMOR + 4, 0x28);
    site(&mut e, 0x0060_47c0, &[0, ACTOR]);
    assert!(calls().is_empty(), "not armour");
    e.mem.set_u8(ARMOR + 4, 0x18);
    // From SetCombatStyle: the preferred weapon, asked with 6 under JIP's mark, equipped.
    let pref = AREA + 0x7a00;
    e.mem.set_u32(CHARACTER_VT + 0x3bc, 0x00de_03bc);
    e.register_double(0x00de_03bc, move |e, w| {
        record(
            0x00de_03bc,
            &[w[1], u32::from(e.caller_marked(JIP_PREFERED_WEAPON_CALL))],
        );
        Ret {
            eax: pref,
            ..Ret::default()
        }
    });
    e.mem.set_u32(pref + 8, WEAPON);
    e.mem.set_u32(pref + 4, 5);
    e.mem.set_u32(pref, lists);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u8(PROCESS + 0x28, 2);
    let from_style = |e: &mut Engine| {
        let _mark = e.mark_caller(Hook::LockEquipment, 0x0060_678f);
        site(e, 0x0060_47c0, &[0, ACTOR])
    };
    assert_eq!(from_style(&mut e), Some(0));
    assert_eq!(
        calls(),
        [
            (0x00de_03bc, vec![6, 1]),
            (0x0088_c650, vec![ACTOR, WEAPON, 1, worn, 1, 0, 1]),
            (0x0044_59e0, vec![pref, 1]),
        ]
    );
    // A thrown weapon type (above 9) keeps the entry's count; no lists: no extra.
    e.mem.set_u8(WEAPON + 0xf4, 10);
    e.mem.set_u32(pref, 0);
    from_style(&mut e);
    assert_eq!(
        calls()[1],
        (0x0088_c650, vec![ACTOR, WEAPON, 5, 0, 1, 0, 1])
    );
    e.mem.set_u32(pref + 4, 1);
    from_style(&mut e);
    assert_eq!(calls()[1].1[2], 1, "a count of 1");
    // A held weapon that suits the style (melee restriction 1, a melee type): nothing.
    let (held, cs_extra, style) = (AREA + 0x7b00, AREA + 0x7c00, AREA + 0x7d00);
    e.mem.set_u8(PROCESS + 0x28, 1);
    e.mem.set_u32(PROCESS + 0x114, held);
    e.mem.set_u32(held + 8, AMMO);
    e.mem.set_u8(AMMO + 0xf4, 2);
    assert_eq!(from_style(&mut e), Some(0));
    assert!(calls().is_empty(), "no combat style extra");
    set_extra(&mut e, ACTOR, 0x69, cs_extra);
    from_style(&mut e);
    assert!(calls().is_empty(), "no style");
    e.mem.set_u32(cs_extra + 0xc, style);
    from_style(&mut e);
    assert!(calls().is_empty(), "no restrictions");
    e.mem.set_u8(style + 0x40, 1);
    from_style(&mut e);
    assert!(calls().is_empty(), "melee only, melee held");
    e.mem.set_u8(AMMO + 0xf4, 3);
    from_style(&mut e);
    assert_eq!(calls().len(), 3, "melee only, a gun held");
    e.mem.set_u8(style + 0x40, 2);
    from_style(&mut e);
    assert!(calls().is_empty(), "guns only, a gun held");
    e.mem.set_u32(PROCESS + 0x114, 0);
    from_style(&mut e);
    assert_eq!(calls().len(), 3, "nothing held");
    e.mem.set_u32(ACTOR + 0x68, 0);
    assert_eq!(from_style(&mut e), Some(0));
    assert!(calls().is_empty(), "no process");
    assert!(!e.caller_marked(0x0060_678f));
}

/// ForceActorDetectionValue and the detection seam.
#[test]
fn forced_detection_values() {
    let mut e = world();
    assert_eq!(site(&mut e, 0x008a_152d, &[ACTOR]), None);
    run(
        &mut e,
        FORCE_ACTOR_DETECTION_VALUE,
        ACTOR,
        &[int_arg(-5), int_arg(1)],
    );
    run(
        &mut e,
        FORCE_ACTOR_DETECTION_VALUE,
        ACTOR,
        &[int_arg(-7), int_arg(1)],
    );
    assert_eq!(
        (
            e.mem.u8(ACTOR + 0x106),
            e.hook_users(Hook::GetDetectionValue)
        ),
        (1, 1)
    );
    assert_eq!(site(&mut e, 0x008a_152d, &[ACTOR]), Some(0x1_fff9));
    run(&mut e, FORCE_ACTOR_DETECTION_VALUE, ACTOR, &[int_arg(30)]);
    assert_eq!(site(&mut e, 0x008a_152d, &[ACTOR]), Some(30));
    state(&mut e).force_detection_value.clear();
    assert_eq!(site(&mut e, 0x008a_152d, &[ACTOR]), Some(0), "missing: 0");
    state(&mut e).force_detection_value.insert(ACTOR, 30);
    run(&mut e, FORCE_ACTOR_DETECTION_VALUE, ACTOR, &[int_arg(0)]);
    run(&mut e, FORCE_ACTOR_DETECTION_VALUE, ACTOR, &[int_arg(0)]);
    assert_eq!(
        (
            e.mem.u8(ACTOR + 0x106),
            e.hook_users(Hook::GetDetectionValue)
        ),
        (0, 0)
    );
    assert_eq!(site(&mut e, 0x008a_152d, &[ACTOR]), None);
}

/// Get/SetScriptDisabled and the event-mask seam.
#[test]
fn disabled_scripts() {
    let mut e = world();
    assert_eq!(run(&mut e, GET_SCRIPT_DISABLED, 0, &[form_arg(S1)]), 0.0);
    run(
        &mut e,
        SET_SCRIPT_DISABLED,
        0,
        &[form_arg(LIST), int_arg(1)],
    );
    assert_eq!(e.mem.u32(SCRIPT_ONE + 0x20), 0);
    assert_eq!(e.mem.u32(QUEST_SCRIPT + 0x20), 0);
    assert_eq!(
        e.hook_users(Hook::MergeEventMask),
        1,
        "not for the quest's script"
    );
    assert_eq!(run(&mut e, GET_SCRIPT_DISABLED, 0, &[form_arg(S1)]), 1.0);
    assert_eq!(run(&mut e, GET_SCRIPT_DISABLED, 0, &[form_arg(S2)]), 0.0);
    run(&mut e, SET_SCRIPT_DISABLED, 0, &[form_arg(S1), int_arg(1)]);
    assert_eq!(e.hook_users(Hook::MergeEventMask), 1, "already disabled");
    let locals = AREA + 0x8000;
    e.mem.set_u32(locals, SCRIPT_ONE);
    assert_eq!(site(&mut e, 0x005a_c782, &[locals, 0, 0x80]), Some(1));
    e.mem.set_u32(locals, SCRIPT_TWO);
    assert_eq!(site(&mut e, 0x005a_c782, &[locals, 0, 0x80]), None);
    run(&mut e, SET_SCRIPT_DISABLED, 0, &[form_arg(S1), int_arg(0)]);
    assert_eq!(e.mem.u32(SCRIPT_ONE + 0x20), 20, "counted from its data");
    assert_eq!(e.hook_users(Hook::MergeEventMask), 0);
    run(&mut e, SET_SCRIPT_DISABLED, 0, &[form_arg(SQ), int_arg(0)]);
    assert_eq!(e.mem.u32(QUEST_SCRIPT + 0x20), 20);
    assert_eq!(e.hook_users(Hook::MergeEventMask), 0);
    // Data that does not follow the shape: length 0 (and so it stays disabled).
    e.mem.set_u32(DATA + 16, 0x12);
    assert_eq!(data_length(&e, SCRIPT_TWO), 0);
    e.mem.set_u32(DATA + 16, 0x11);
    e.mem.set_u32(DATA, 0x1c);
    assert_eq!(data_length(&e, SCRIPT_TWO), 0);
    e.mem.set_u32(DATA, 0x1d);
    e.mem.set_u16(DATA + 4, 0x11);
    assert_eq!(data_length(&e, SCRIPT_TWO), 4, "no block");
}

/// The push seam: the bit with time left keeps the velocity; with none left it goes.
#[test]
fn pushed_actor_velocity() {
    let mut e = world();
    assert_eq!(site(&mut e, 0x00c6_d4e4, &[CONTROLLER]), None);
    e.mem.set_u8(CONTROLLER + 0x417, 0x81);
    e.mem.set_f32(CONTROLLER + 0x524, 0.5);
    assert_eq!(site(&mut e, 0x00c6_d4e4, &[CONTROLLER]), Some(1));
    e.mem.set_f32(CONTROLLER + 0x524, f32::NAN);
    assert_eq!(
        site(&mut e, 0x00c6_d4e4, &[CONTROLLER]),
        Some(1),
        "unordered"
    );
    assert_eq!(e.mem.u8(CONTROLLER + 0x417), 0x81);
    e.mem.set_f32(CONTROLLER + 0x524, 0.0);
    assert_eq!(site(&mut e, 0x00c6_d4e4, &[CONTROLLER]), None);
    assert_eq!(e.mem.u8(CONTROLLER + 0x417), 0x01);
}

/// Get/SetTeammateUsingAmmo: infinite ammo is one user of the ammo hook.
#[test]
fn teammate_ammo() {
    let mut e = world();
    assert_eq!(run(&mut e, GET_TEAMMATE_USING_AMMO, ACTOR, &[]), 1.0);
    run(&mut e, SET_TEAMMATE_USING_AMMO, ACTOR, &[int_arg(0)]);
    run(&mut e, SET_TEAMMATE_USING_AMMO, ACTOR, &[int_arg(0)]);
    assert_eq!((flags1(&e, ACTOR), e.hook_users(Hook::RemoveAmmo)), (4, 1));
    assert_eq!(run(&mut e, GET_TEAMMATE_USING_AMMO, ACTOR, &[]), 0.0);
    run(&mut e, SET_TEAMMATE_USING_AMMO, ACTOR, &[int_arg(1)]);
    run(&mut e, SET_TEAMMATE_USING_AMMO, ACTOR, &[int_arg(1)]);
    assert_eq!((flags1(&e, ACTOR), e.hook_users(Hook::RemoveAmmo)), (0, 0));
    assert_eq!(run(&mut e, GET_TEAMMATE_USING_AMMO, SCRIPT_ONE, &[]), 0.0);
}

/// Pre-load: the combat, teleport and equipment hooks off; the maps cleared with their hooks.
#[test]
fn pre_load_housekeeping() {
    let mut e = world();
    run(&mut e, SET_COMBAT_DISABLED, ACTOR, &[int_arg(1)]);
    run(
        &mut e,
        SET_COMBAT_DISABLED,
        OTHER,
        &[int_arg(1), form_arg(A2)],
    );
    run(&mut e, CCC_SET_FOLLOW_STATE, ACTOR, &[int_arg(1)]);
    run(&mut e, LOCK_EQUIPMENT, ACTOR, &[int_arg(1)]);
    run(&mut e, LOCK_EQUIPMENT, OTHER, &[int_arg(1)]);
    run(&mut e, FORCE_ACTOR_DETECTION_VALUE, ACTOR, &[int_arg(3)]);
    run(
        &mut e,
        TOGGLE_NO_HEALTH_RESET,
        0,
        &[int_arg(1), float_arg(1.0)],
    );
    pre_load(&mut e);
    for hook in [
        Hook::StartCombat,
        Hook::SetCombatTarget,
        Hook::TeleportWithPC,
        Hook::LockEquipment,
        Hook::GetDetectionValue,
    ] {
        assert_eq!(e.hook_users(hook), 0, "{hook:?}");
    }
    assert_eq!(e.hook_users(Hook::NoHealthReset), 1, "kept");
    assert!(state(&mut e).force_combat_target.is_empty());
    assert!(state(&mut e).force_detection_value.is_empty());
    assert_eq!(flags1(&e, ACTOR), 1 | 0x40 | 8, "the flags stay");
    // Empty maps: their hooks keep their users (JIP sets them only when it clears a map).
    let mut e = world();
    e.add_hook_user(Hook::SetCombatTarget, true);
    e.add_hook_user(Hook::GetDetectionValue, true);
    pre_load(&mut e);
    assert_eq!(
        (
            e.hook_users(Hook::SetCombatTarget),
            e.hook_users(Hook::GetDetectionValue)
        ),
        (1, 1)
    );
}
