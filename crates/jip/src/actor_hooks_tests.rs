//! Tests of JIP's actor event hooks: nothing put in while JIP answers "not installed" (the
//! eleven commands' records as #502's table leaves them, no seam handler, the processes'
//! vtables untouched); installed, each seam's handler as JIP's hook decides, the commands through
//! their records with the hooks' user counts and the actors' and forms' flags, the hit-data
//! routines, and the pre-load housekeeping. Script calls are recorded ([`crate::script_call`]).
//! Layouts are written out at literal offsets, not the code's: a form's type `+4`, JIP's flag
//! byte `+6`, its id `+0xC`; a reference's base `+0x20`; an actor's process `+0x68`, JIP's flag
//! bytes `+0x105` and `+0x107`, its teammate byte `+0x18D`; a process's held weapon `+0x114`,
//! hit data `+0x240`, action `+0x2EC` (16 bits) and sequence `+0x2F0`; an item change's form
//! `+8`; an animation's actor `+4`; the player's first-person animation `+0x690`; a hit data's
//! source, target, location, weapon, flags and JIP's location at `+0`, `+4`, `+0x10`, `+0x30`,
//! `+0x58`, `+0x60`; a weapon's flags `+0x100` (byte) and `+0x12C`; a projectile base's type
//! `+0x62`, a projectile's position `+0x30` and impact list `+0x88`; a ragdoll controller's Bip01
//! node `+0x58`, a node's parent `+0x18`, a fade node's reference `+0xCC`; a list form's first
//! node `+0x18` (item, next); the vtable entry that answers true `0x008D0360` at `+0x100`
//! (an actor) and `+0xF0` (a reference); `BSFadeNode`'s vtable `0x010A8F90`; the processes'
//! hit-data slots `0x01087FD8`, `0x010897C0` (copy) and `0x01087FDC`, `0x010897C4` (clear).

use super::*;
use crate::script_call::calls;
use crate::test_support::{engine, form_arg, int_arg, line, script_ref, short_arg, RESULT};

const AREA: u32 = 0x0500_0000;
const SCRIPT_FORM: u32 = AREA;
const SCRIPT_TWO: u32 = AREA + 0x40;
const ACTOR_VT: u32 = AREA + 0x800;
const ACTOR: u32 = AREA + 0x1000;
const OTHER: u32 = AREA + 0x1400;
const NOT_ACTOR: u32 = AREA + 0x1800;
const PROCESS: u32 = AREA + 0x2000;
const WEAPON: u32 = AREA + 0x3000;
const ITEM: u32 = AREA + 0x3400;
const PROJ_BASE: u32 = AREA + 0x3800;
const LIST_FORM: u32 = AREA + 0x3c00;
const NODES: u32 = AREA + 0x3d00;
const HIT: u32 = AREA + 0x4000;
const PROJECTILE: u32 = AREA + 0x5000;
const IMPACT: u32 = AREA + 0x5400;
const HEAP: u32 = AREA + 0x8000;
const EXE_COPY: u32 = 0x0092_c400;
const EXE_CLEAR: u32 = 0x0092_c4a0;

fn world_as(installed: bool) -> Engine {
    calls();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    for slot in COPY_HIT_DATA_SLOTS {
        e.mem.set_u32(slot, EXE_COPY);
    }
    for slot in CLEAR_HIT_DATA_SLOTS {
        e.mem.set_u32(slot, EXE_CLEAR);
    }
    for (form, kind) in [
        (SCRIPT_FORM, 0x11),
        (SCRIPT_TWO, 0x11),
        (WEAPON, 0x28),
        (ITEM, 0x2f),
        (PROJ_BASE, 0x33),
        (LIST_FORM, 0x55),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form, AREA + 0x900);
    }
    e.mem.set_u32(SCRIPT_FORM + 0xc, 0x0100_0900);
    e.mem.set_u32(SCRIPT_TWO + 0xc, 0x0100_0901);
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(ACTOR_VT + 0xf0, 0x008d_0360);
    for actor in [ACTOR, OTHER] {
        e.mem.set_u32(actor, ACTOR_VT);
        e.mem.set_u8(actor + 4, 0x3c);
        e.mem.set_u32(actor + 0xc, 0x0100_2000 + actor - ACTOR);
        e.mem.set_u32(actor + 0x68, PROCESS);
    }
    e.mem.set_u32(NOT_ACTOR, AREA + 0x900);
    // A list form holding ACTOR, NOT_ACTOR and OTHER.
    e.mem.set_u32(LIST_FORM + 0x18, ACTOR);
    e.mem.set_u32(LIST_FORM + 0x1c, NODES);
    e.mem.set_u32(NODES, NOT_ACTOR);
    e.mem.set_u32(NODES + 4, NODES + 8);
    e.mem.set_u32(NODES + 8, OTHER);
    e.mem.set_u32(HEAP, HEAP + 0x10);
    e.register(0x00aa_3e40, |e, _| {
        let at = e.mem.u32(HEAP);
        e.mem.set_u32(HEAP, at + 0x80);
        Ret {
            eax: at,
            ..Ret::default()
        }
    });
    crate::install_as(&mut e, installed);
    for (i, form) in [
        SCRIPT_FORM,
        SCRIPT_TWO,
        ACTOR,
        OTHER,
        LIST_FORM,
        ITEM,
        PROJ_BASE,
        WEAPON,
        NOT_ACTOR,
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
const S1: u16 = 1;
const S2: u16 = 2;
const A1: u16 = 3;
const A2: u16 = 4;
const LIST: u16 = 5;
const AID: u16 = 6;
const PROJ: u16 = 7;
const WEAP: u16 = 8;

/// Runs `opcode` with `args` through its record (no calling reference).
fn run(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) {
    let words = line(e, opcode, args);
    e.mem.set_f64(RESULT, 0.0);
    let at = crate::test_support::execute_of(e, opcode);
    assert_eq!(e.call(at, &words).eax & 0xff, 1);
}

fn site(e: &mut Engine, addr: u32, args: &[u32]) -> Option<u32> {
    e.site(addr, args)
}

fn flags3(e: &Engine, actor: u32) -> u8 {
    e.mem.u8(actor + 0x107)
}

/// While JIP answers "not installed": each command's whole record (0x28 bytes and its parameter
/// list) is what #502's table leaves, no seam has a handler, no hook has a user, and the
/// processes' hit-data slots keep the exe's functions.
#[test]
fn not_installed_nothing_is_put_in() {
    let snapshot = |e: &Engine, op: u32| {
        let at = e.script_tables().record(op);
        let mut bytes = e.mem.bytes(at, 0x28);
        let (count, params) = (e.mem.u16(at + 0x12), e.mem.u32(at + 0x14));
        bytes.extend(e.mem.bytes(params, 12 * u32::from(count)));
        bytes
    };
    let mut table_only = engine();
    crate::table::install(&mut table_only, false);
    let off = world_as(false);
    for op in COMMANDS {
        assert_eq!(snapshot(&off, op), snapshot(&table_only, op), "{op:04x}");
        assert_eq!(off.mem.u32(off.script_tables().record(op) + 0x18), 0);
    }
    for addr in SITES {
        assert!(off.site_owners(addr).is_empty(), "{addr:08x}");
    }
    for slot in COPY_HIT_DATA_SLOTS {
        assert_eq!(off.mem.u32(slot), EXE_COPY);
    }
    for slot in CLEAR_HIT_DATA_SLOTS {
        assert_eq!(off.mem.u32(slot), EXE_CLEAR);
    }
    for (_, hook, _) in COMMAND_SITES {
        assert_eq!(off.hook_users(hook), 0);
    }
}

/// Installed: JIP's handler at each seam, its routines in the four slots, the commands built.
#[test]
fn installed_puts_in_the_handlers_routines_and_commands() {
    let e = world();
    for addr in SITES {
        assert_eq!(e.site_owners(addr), [Owner::Jip], "{addr:08x}");
    }
    for slot in COPY_HIT_DATA_SLOTS {
        assert_eq!(e.mem.u32(slot), COPY_HIT_DATA_ROUTINE);
    }
    for slot in CLEAR_HIT_DATA_SLOTS {
        assert_eq!(e.mem.u32(slot), CLEAR_HIT_DATA_ROUTINE);
    }
    for op in COMMANDS {
        assert_ne!(
            e.mem.u32(e.script_tables().record(op) + 0x18),
            0,
            "{op:04x}"
        );
    }
}

/// A command may add users to a hook only while every site JIP patches for it has a seam with
/// JIP's handler. And no command that adds MarkScriptEvent users (`SetScriptEventDisabled`
/// 0x27E6, `DisableScriptedActivate` 0x282B) may be built while that seam has no handler: its
/// hook also reads the caller's first stack word, which no seam passes yet.
#[test]
fn every_command_hook_has_all_its_seams() {
    let e = world();
    let jip_row = |addr: u32| {
        engine_core::extend::SITES
            .iter()
            .any(|s| s.addr == addr && s.owners.iter().any(|o| matches!(o, Owner::Jip)))
    };
    for (op, _, sites) in COMMAND_SITES {
        let built = e.mem.u32(e.script_tables().record(op) + 0x18) != 0;
        for &addr in sites {
            assert!(
                !built
                    || (jip_row(addr)
                        && (!SITES.contains(&addr) || e.site_owners(addr) == [Owner::Jip])),
                "{op:04x} is built but {addr:08x} has no JIP seam"
            );
        }
    }
    let mark_handled = !e.site_owners(0x005a_c763).is_empty();
    for op in [0x27e6u32, 0x282b] {
        let built = e.mem.u32(e.script_tables().record(op) + 0x18) != 0;
        assert!(
            mark_handled || !built,
            "{op:04x} adds MarkScriptEvent users"
        );
    }
}

/// The action events for every actor (through an empty list form) and for actors: a type's
/// first script is one user, an actor's first type one user with its flag; the seam stores the
/// sequence and a new action, then calls the scripts with the actor and the action; removals
/// give users back only for what was set, the last of an actor's types clearing its flag.
#[test]
fn anim_action_events() {
    let mut e = world();
    let empty = AREA + 0x6000;
    e.mem.set_u8(empty + 4, 0x55);
    script_ref(&mut e, 10, empty);
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S1), int_arg(1), form_arg(10), int_arg(2)],
    );
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S2), int_arg(1), form_arg(10), int_arg(2)],
    );
    assert_eq!(e.hook_users(Hook::SetAnimAction), 1, "the same type");
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S1), int_arg(1), form_arg(A1), int_arg(2)],
    );
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S1), int_arg(1), form_arg(A1), int_arg(5)],
    );
    assert_eq!(
        e.hook_users(Hook::SetAnimAction),
        2,
        "the actor's first type"
    );
    assert_eq!(flags3(&e, ACTOR), 1);
    calls();
    e.mem.set_u16(PROCESS + 0x2ec, 0xffff);
    assert_eq!(site(&mut e, 0x008a_7551, &[ACTOR, 2, 0x77]), Some(0));
    assert_eq!(e.mem.u32(PROCESS + 0x2f0), 0x77);
    assert_eq!(e.mem.u16(PROCESS + 0x2ec), 2);
    assert_eq!(
        calls(),
        [
            (SCRIPT_FORM, ACTOR, vec![2]),
            (SCRIPT_TWO, ACTOR, vec![2]),
            (SCRIPT_FORM, ACTOR, vec![2])
        ]
    );
    assert_eq!(site(&mut e, 0x008a_7551, &[ACTOR, 2, 0x78]), Some(0));
    assert_eq!(e.mem.u32(PROCESS + 0x2f0), 0x78, "the sequence always");
    assert!(calls().is_empty(), "the same action");
    set_animation_event(&mut e, SCRIPT_FORM, true, empty, u32::MAX, true);
    assert_eq!(site(&mut e, 0x008a_7551, &[OTHER, u32::MAX, 0]), Some(0));
    assert_eq!(e.mem.u16(PROCESS + 0x2ec), 0xffff, "stored");
    assert!(calls().is_empty(), "a negative action");
    set_animation_event(&mut e, SCRIPT_FORM, false, empty, u32::MAX, true);
    assert_eq!(site(&mut e, 0x008a_7551, &[OTHER, 5, 0]), Some(0));
    assert!(
        calls().is_empty(),
        "no events for type 5 without OTHER's flag"
    );
    // Removals: what is not set gives nothing back.
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S2), int_arg(0), form_arg(A1), int_arg(2)],
    );
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S1), int_arg(0), form_arg(A2), int_arg(2)],
    );
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S1), int_arg(0), form_arg(10), int_arg(9)],
    );
    assert_eq!(e.hook_users(Hook::SetAnimAction), 2);
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S1), int_arg(0), form_arg(A1), int_arg(2)],
    );
    assert_eq!(flags3(&e, ACTOR), 1, "type 5 stays");
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S1), int_arg(0), form_arg(A1), int_arg(5)],
    );
    assert_eq!(
        (flags3(&e, ACTOR), e.hook_users(Hook::SetAnimAction)),
        (0, 1)
    );
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S1), int_arg(0), form_arg(10), int_arg(2)],
    );
    assert_eq!(e.hook_users(Hook::SetAnimAction), 1, "S2 keeps type 2");
    run(
        &mut e,
        SET_ON_ANIM_ACTION,
        &[form_arg(S2), int_arg(0), form_arg(10), int_arg(2)],
    );
    assert_eq!(e.hook_users(Hook::SetAnimAction), 0);
    assert!(e.ext::<Jip>().actor_hooks.anim_action.is_empty());
}

/// The play-group command through its record, its fourth argument a `kParamType_AnimationGroup`
/// as the compiler writes it (two bytes, no type byte): the script is filed under that group,
/// for one actor and for every actor; the seam calls it for that type only; removing with the
/// same group gives the hook users back.
#[test]
fn play_group_command_files_under_its_group() {
    let mut e = world();
    let empty = AREA + 0x6000;
    e.mem.set_u8(empty + 4, 0x55);
    script_ref(&mut e, 10, empty);
    run(
        &mut e,
        SET_ON_PLAY_GROUP,
        &[form_arg(S1), int_arg(1), form_arg(A1), short_arg(0xc3)],
    );
    run(
        &mut e,
        SET_ON_PLAY_GROUP,
        &[form_arg(S2), int_arg(1), form_arg(10), short_arg(0x11)],
    );
    assert_eq!(e.hook_users(Hook::SetAnimGroup), 2);
    assert_eq!(flags3(&e, ACTOR), 2);
    {
        let s = &e.ext::<Jip>().actor_hooks;
        assert_eq!(s.play_group.keys().copied().collect::<Vec<_>>(), [0x11]);
        assert_eq!(
            s.play_group_actors[&ACTOR]
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [0xc3]
        );
    }
    let anim = AREA + 0x6100;
    e.mem.set_u32(anim + 4, ACTOR);
    calls();
    site(&mut e, 0x0049_4e19, &[anim, 0x31, 0xc3, ACTOR]);
    assert_eq!(calls(), [(SCRIPT_FORM, ACTOR, vec![0xc3])]);
    site(&mut e, 0x0049_4e19, &[anim, 0x31, 0x11, ACTOR]);
    assert_eq!(calls(), [(SCRIPT_TWO, ACTOR, vec![0x11])]);
    for other in [0, 0x12, 0xc4] {
        site(&mut e, 0x0049_4e19, &[anim, 0x31, other, ACTOR]);
        assert!(calls().is_empty(), "type {other:#x}");
    }
    // Another group gives nothing back; the same group gives each user back.
    run(
        &mut e,
        SET_ON_PLAY_GROUP,
        &[form_arg(S1), int_arg(0), form_arg(A1), short_arg(0x11)],
    );
    assert_eq!(e.hook_users(Hook::SetAnimGroup), 2);
    run(
        &mut e,
        SET_ON_PLAY_GROUP,
        &[form_arg(S1), int_arg(0), form_arg(A1), short_arg(0xc3)],
    );
    assert_eq!(
        (e.hook_users(Hook::SetAnimGroup), flags3(&e, ACTOR)),
        (1, 0)
    );
    run(
        &mut e,
        SET_ON_PLAY_GROUP,
        &[form_arg(S2), int_arg(0), form_arg(10), short_arg(0x11)],
    );
    assert_eq!(e.hook_users(Hook::SetAnimGroup), 0);
    assert!(e.ext::<Jip>().actor_hooks.play_group.is_empty());
}

/// The play-group events (the shared code, with the group hook and flag): a list of actors
/// skips what is not an actor; the seam calls nothing for group 0xFF or the player's
/// first-person animation, else the type's scripts with the actor and the type.
#[test]
fn play_group_events() {
    let mut e = world();
    set_animation_event(&mut e, SCRIPT_FORM, true, LIST_FORM, 0x11, false);
    assert_eq!(e.hook_users(Hook::SetAnimGroup), 2, "two actors");
    assert_eq!((flags3(&e, ACTOR), flags3(&e, OTHER)), (2, 2));
    assert_eq!(e.mem.u8(NOT_ACTOR + 0x107), 0);
    let anim = AREA + 0x6100;
    e.mem.set_u32(anim + 4, ACTOR);
    calls();
    assert_eq!(site(&mut e, 0x0049_4e19, &[anim, 0x31, 0x11, ACTOR]), None);
    assert_eq!(calls(), [(SCRIPT_FORM, ACTOR, vec![0x11])]);
    e.mem.set_u8(ACTOR + 0x107, 0);
    site(&mut e, 0x0049_4e19, &[anim, 0x31, 0x11, ACTOR]);
    assert!(calls().is_empty(), "the actor's flag clear");
    e.mem.set_u8(ACTOR + 0x107, 2);
    assert_eq!(site(&mut e, 0x0049_4e19, &[anim, 0xff, 0x11, ACTOR]), None);
    assert!(calls().is_empty(), "group 0xFF");
    assert_eq!(site(&mut e, 0x0049_4e19, &[anim, 0x31, 0x12, ACTOR]), None);
    assert!(calls().is_empty(), "another type");
    e.mem.set_u32(ACTOR + 0xc, 0x14);
    assert_eq!(site(&mut e, 0x0049_4e19, &[anim, 0x31, 0x11, ACTOR]), None);
    assert_eq!(calls().len(), 1, "the player's third person");
    e.mem.set_u32(ACTOR + 0x690, anim);
    assert_eq!(site(&mut e, 0x0049_4e19, &[anim, 0x31, 0x11, ACTOR]), None);
    assert!(calls().is_empty(), "the player's first person");
    set_animation_event(&mut e, SCRIPT_TWO, false, LIST_FORM, 0x11, false);
    assert_eq!(e.hook_users(Hook::SetAnimGroup), 2, "not set: nothing back");
    set_animation_event(&mut e, SCRIPT_FORM, false, OTHER, 0x11, false);
    assert_eq!(
        (e.hook_users(Hook::SetAnimGroup), flags3(&e, OTHER)),
        (1, 0)
    );
}

/// The four actor events' commands: without a form (fire and hit) each script one user; with
/// an actor or list, an actor's first script one user and the flag set with every script; a
/// removal clears the flag, the last gives the user back; what is not set gives nothing back
/// while another holder stays.
#[test]
fn actor_event_commands() {
    let mut e = world();
    for (op, hook, flag) in [
        (SET_ON_FIRE_WEAPON, Hook::RemoveAmmo, 0x10),
        (SET_ON_HIT, Hook::OnHitEvent, 0x20),
    ] {
        run(&mut e, op, &[form_arg(S1), int_arg(1)]);
        run(&mut e, op, &[form_arg(S1), int_arg(1)]);
        assert_eq!(e.hook_users(hook), 1, "{op:04x}");
        run(&mut e, op, &[form_arg(S2), int_arg(1), form_arg(LIST)]);
        assert_eq!(e.hook_users(hook), 3);
        assert_eq!(
            (flags3(&e, ACTOR) & flag, flags3(&e, OTHER) & flag),
            (flag, flag)
        );
        run(&mut e, op, &[form_arg(S1), int_arg(0), form_arg(A1)]);
        run(&mut e, op, &[form_arg(S2), int_arg(0)]);
        assert_eq!(e.hook_users(hook), 3, "not set: nothing back");
        assert_eq!(flags3(&e, ACTOR) & flag, flag);
        run(&mut e, op, &[form_arg(S2), int_arg(0), form_arg(A1)]);
        assert_eq!((e.hook_users(hook), flags3(&e, ACTOR) & flag), (2, 0));
        run(&mut e, op, &[form_arg(S1), int_arg(0)]);
        run(&mut e, op, &[form_arg(S2), int_arg(0), form_arg(A2)]);
        assert_eq!(e.hook_users(hook), 0);
    }
    for (op, flag) in [(SET_ON_HEALTH_DAMAGE, 4), (SET_ON_CRIPPLED_LIMB, 8)] {
        run(&mut e, op, &[form_arg(S1), int_arg(1), form_arg(A1)]);
        run(&mut e, op, &[form_arg(S2), int_arg(1), form_arg(A1)]);
        assert_eq!(flags3(&e, ACTOR) & flag, flag);
    }
    assert_eq!(e.hook_users(Hook::DamageActorValue), 2, "one actor per map");
    // The flag goes with any script removed, as in JIP, the user with the last.
    run(
        &mut e,
        SET_ON_HEALTH_DAMAGE,
        &[form_arg(S1), int_arg(0), form_arg(A1)],
    );
    assert_eq!(flags3(&e, ACTOR) & 4, 0);
    assert_eq!(e.hook_users(Hook::DamageActorValue), 2);
    run(
        &mut e,
        SET_ON_HEALTH_DAMAGE,
        &[form_arg(S1), int_arg(0), form_arg(A1)],
    );
    run(
        &mut e,
        SET_ON_CRIPPLED_LIMB,
        &[form_arg(S1), int_arg(0), form_arg(A2)],
    );
    assert_eq!(e.hook_users(Hook::DamageActorValue), 2, "not set");
    run(
        &mut e,
        SET_ON_HEALTH_DAMAGE,
        &[form_arg(S2), int_arg(0), form_arg(A1)],
    );
    assert_eq!(e.hook_users(Hook::DamageActorValue), 1);
}

/// Health and limb damage: health lost (a change below 0) calls the actor's health scripts with
/// the change and the source actor (0 for none); a limb condition (perception to brain) that
/// was not 0 and is now at most 0 calls the limb scripts with the limb; nothing without the
/// actor's flag. Every answer is that the callback is made.
#[test]
fn damage_events() {
    let mut e = world();
    run(
        &mut e,
        SET_ON_HEALTH_DAMAGE,
        &[form_arg(S1), int_arg(1), form_arg(A1)],
    );
    run(
        &mut e,
        SET_ON_CRIPPLED_LIMB,
        &[form_arg(S2), int_arg(1), form_arg(A1)],
    );
    let owner = ACTOR + 0xa4;
    let (old, hurt, healed) = (50.0f32.to_bits(), (-5.0f32).to_bits(), 5.0f32.to_bits());
    calls();
    assert_eq!(
        site(&mut e, 0x0066_ee72, &[owner, 0x10, old, hurt, OTHER + 0xa4]),
        Some(0)
    );
    assert_eq!(calls(), [(SCRIPT_FORM, ACTOR, vec![hurt, OTHER])]);
    site(&mut e, 0x0066_ee72, &[owner, 0x10, old, hurt, 0]);
    assert_eq!(calls(), [(SCRIPT_FORM, ACTOR, vec![hurt, 0])]);
    site(&mut e, 0x0066_ee72, &[owner, 0x10, old, healed, 0]);
    assert!(calls().is_empty(), "healed");
    let (limb_old, to_zero) = (10.0f32.to_bits(), (-10.0f32).to_bits());
    site(&mut e, 0x0066_ee72, &[owner, 0x1b, limb_old, to_zero, 0]);
    assert_eq!(calls(), [(SCRIPT_TWO, ACTOR, vec![2])]);
    site(&mut e, 0x0066_ee72, &[owner, 0x1f, limb_old, hurt, 0]);
    assert!(calls().is_empty(), "still above 0");
    site(&mut e, 0x0066_ee72, &[owner, 0x1b, 0, to_zero, 0]);
    assert!(calls().is_empty(), "was 0");
    site(&mut e, 0x0066_ee72, &[owner, 0x20, limb_old, to_zero, 0]);
    site(&mut e, 0x0066_ee72, &[owner, 0x18, limb_old, to_zero, 0]);
    assert!(calls().is_empty(), "not a condition");
    assert_eq!(site(&mut e, 0x0066_ee72, &[0, 0x10, old, hurt, 0]), Some(0));
    e.mem.set_u8(ACTOR + 0x107, 0);
    site(&mut e, 0x0066_ee72, &[owner, 0x10, old, hurt, 0]);
    site(&mut e, 0x0066_ee72, &[owner, 0x1b, limb_old, to_zero, 0]);
    assert!(calls().is_empty(), "no flags");
}

/// The fire events and `ShouldUseAmmo`'s answer: the reload's call (marked at `0x8A86B0`) calls
/// no scripts, the next call does; the answer is true for the player, false with infinite ammo
/// or no weapon, true for flag 2 at `+0x12C`, false for bit 7 at `+0x100`, else the teammate
/// byte.
#[test]
fn fire_weapon_events_and_the_ammo_answer() {
    let mut e = world();
    run(&mut e, SET_ON_FIRE_WEAPON, &[form_arg(S1), int_arg(1)]);
    run(
        &mut e,
        SET_ON_FIRE_WEAPON,
        &[form_arg(S2), int_arg(1), form_arg(A1)],
    );
    calls();
    assert_eq!(site(&mut e, 0x008a_8dd0, &[ACTOR, WEAPON]), Some(0));
    assert_eq!(
        calls(),
        [
            (SCRIPT_FORM, ACTOR, vec![WEAPON]),
            (SCRIPT_TWO, ACTOR, vec![WEAPON])
        ]
    );
    let mark = e.mark_caller(Hook::RemoveAmmo, 0x008a_86b0);
    site(&mut e, 0x008a_8dd0, &[ACTOR, WEAPON]);
    assert!(!e.caller_marked(0x008a_86b0), "taken at the first check");
    drop(mark);
    assert!(calls().is_empty(), "the reload's call");
    site(&mut e, 0x008a_8dd0, &[OTHER, WEAPON]);
    assert_eq!(calls(), [(SCRIPT_FORM, OTHER, vec![WEAPON])], "unmarked");
    let answer = |e: &mut Engine, actor: u32, weapon: u32| site(e, 0x008a_8dd0, &[actor, weapon]);
    e.mem.set_u8(ACTOR + 0x18d, 1);
    assert_eq!(answer(&mut e, ACTOR, WEAPON), Some(1), "a teammate");
    e.mem.set_u8(WEAPON + 0x100, 0x80);
    assert_eq!(answer(&mut e, ACTOR, WEAPON), Some(0));
    e.mem.set_u32(WEAPON + 0x12c, 2);
    assert_eq!(answer(&mut e, ACTOR, WEAPON), Some(1));
    assert_eq!(answer(&mut e, ACTOR, 0), Some(0), "no weapon");
    e.mem.set_u8(ACTOR + 0x105, 4);
    assert_eq!(answer(&mut e, ACTOR, WEAPON), Some(0), "infinite ammo");
    e.mem.set_u32(ACTOR + 0xc, 0x14);
    assert_eq!(answer(&mut e, ACTOR, 0), Some(1), "the player");
    e.mem.set_u32(ACTOR + 0xc, 0x15);
    e.mem.set_u8(ACTOR + 0x105, 0);
    e.mem.set_u8(ACTOR + 0x18d, 0);
    e.mem.set_u32(WEAPON + 0x12c, 0);
    e.mem.set_u8(WEAPON + 0x100, 0);
    assert_eq!(answer(&mut e, ACTOR, WEAPON), Some(0), "not a teammate");
}

/// The hit events at `Script::SetActionFlag`'s end: only with the OnHit flag, on an actor's list
/// (its vtable at 0x0108xxxx with bit 3), not for the magic target's call (marked at
/// `0x8C49B7`); the scripts for every actor, then the actor's own.
#[test]
fn hit_events() {
    let mut e = world();
    let actor = AREA + 0x7000;
    e.mem.set_u32(actor, 0x0108_7808);
    run(&mut e, SET_ON_HIT, &[form_arg(S1), int_arg(1)]);
    let list = actor + 0x44;
    calls();
    assert_eq!(site(&mut e, 0x005a_c78a, &[0, list, 0x80, 1]), None);
    assert_eq!(calls(), [(SCRIPT_FORM, actor, vec![])]);
    site(&mut e, 0x005a_c78a, &[0, list, 0x40, 1]);
    site(&mut e, 0x005a_c78a, &[0, 0, 0x80, 1]);
    assert!(calls().is_empty());
    e.mem.set_u32(actor, 0x0108_7800);
    site(&mut e, 0x005a_c78a, &[0, list, 0x80, 1]);
    e.mem.set_u32(actor, 0x0109_7808);
    site(&mut e, 0x005a_c78a, &[0, list, 0x80, 1]);
    assert!(calls().is_empty(), "not an actor's vtable");
    e.mem.set_u32(actor, 0x0108_7808);
    let mark = e.mark_caller(Hook::OnHitEvent, 0x008c_49b7);
    site(&mut e, 0x005a_c78a, &[0, list, 0x80, 1]);
    assert!(!e.caller_marked(0x008c_49b7), "taken");
    drop(mark);
    assert!(calls().is_empty(), "the magic target's call");
    site(&mut e, 0x005a_c78a, &[0, list, 0x80, 1]);
    assert_eq!(calls().len(), 1, "unmarked again");
}

/// The reload and ragdoll lists: one user per script, removals only for what is set; a reload
/// (or reload loop) action that is new calls the reload scripts with the held weapon; the
/// ragdoll scripts run on the fade node's reference above the Bip01 node.
#[test]
fn reload_and_ragdoll_events() {
    let mut e = world();
    for (op, hook) in [
        (SET_ON_RELOAD_WEAPON, Hook::ReloadWeapon),
        (SET_ON_RAGDOLL, Hook::OnRagdoll),
    ] {
        run(&mut e, op, &[form_arg(S1), int_arg(1)]);
        run(&mut e, op, &[form_arg(S2), int_arg(1)]);
        run(&mut e, op, &[form_arg(S2), int_arg(1)]);
        assert_eq!(e.hook_users(hook), 2);
        run(&mut e, op, &[form_arg(S2), int_arg(0)]);
        run(&mut e, op, &[form_arg(S2), int_arg(0)]);
        assert_eq!(e.hook_users(hook), 1, "removed once");
    }
    let entry = AREA + 0x6200;
    e.mem.set_u32(entry + 8, WEAPON);
    e.mem.set_u32(PROCESS + 0x114, entry);
    e.mem.set_u16(PROCESS + 0x2ec, 3);
    calls();
    assert_eq!(site(&mut e, 0x008a_7549, &[ACTOR, 9, 0]), None);
    assert_eq!(calls(), [(SCRIPT_FORM, ACTOR, vec![WEAPON])]);
    site(&mut e, 0x008a_7549, &[ACTOR, 0x111, 0]);
    assert_eq!(calls().len(), 1, "a reload loop (its low byte)");
    e.mem.set_u16(PROCESS + 0x2ec, 9);
    site(&mut e, 0x008a_7549, &[ACTOR, 9, 0]);
    e.mem.set_u16(PROCESS + 0x2ec, 3);
    site(&mut e, 0x008a_7549, &[ACTOR, 3, 0]);
    site(&mut e, 0x008a_7549, &[ACTOR, 8, 0]);
    assert!(calls().is_empty(), "the same action, or no reload");
    e.mem.set_u32(PROCESS + 0x114, 0);
    site(&mut e, 0x008a_7549, &[ACTOR, 9, 0]);
    assert_eq!(calls(), [(SCRIPT_FORM, ACTOR, vec![0])]);
    let (controller, bip01, fade) = (AREA + 0x6300, AREA + 0x6400, AREA + 0x6500);
    assert_eq!(site(&mut e, 0x00c7_c151, &[controller]), None);
    e.mem.set_u32(controller + 0x58, bip01);
    e.mem.set_u32(bip01 + 0x18, fade);
    e.mem.set_u32(fade + 0xcc, ACTOR);
    site(&mut e, 0x00c7_c151, &[controller]);
    assert!(calls().is_empty(), "not a fade node");
    e.mem.set_u32(fade, 0x010a_8f90);
    site(&mut e, 0x00c7_c151, &[controller]);
    assert_eq!(calls(), [(SCRIPT_FORM, ACTOR, vec![])]);
}

/// Aid items and projectiles: forms of the type only (a list's other forms skipped), a form's
/// first script one user and its flag, the last both back; the aid seam calls the item's
/// scripts on the actor with the item; the impact seam moves a non-beam projectile to its
/// impact's position, then calls its base's scripts on it.
#[test]
fn aid_item_and_projectile_events() {
    let mut e = world();
    run(
        &mut e,
        SET_ON_USE_AID_ITEM,
        &[form_arg(S1), int_arg(1), form_arg(AID)],
    );
    run(
        &mut e,
        SET_ON_USE_AID_ITEM,
        &[form_arg(S1), int_arg(1), form_arg(WEAP)],
    );
    assert_eq!(e.hook_users(Hook::EquipAidItem), 1);
    assert_eq!(e.mem.u8(ITEM + 6), 1);
    calls();
    assert_eq!(site(&mut e, 0x0088_c74e, &[ITEM, ACTOR]), None);
    assert_eq!(calls(), [(SCRIPT_FORM, ACTOR, vec![ITEM])]);
    e.mem.set_u8(ITEM + 6, 0);
    site(&mut e, 0x0088_c74e, &[ITEM, ACTOR]);
    assert!(calls().is_empty(), "the item's flag clear");
    e.mem.set_u8(ITEM + 6, 1);
    run(
        &mut e,
        SET_ON_USE_AID_ITEM,
        &[form_arg(S2), int_arg(0), form_arg(AID)],
    );
    assert_eq!(
        (e.hook_users(Hook::EquipAidItem), e.mem.u8(ITEM + 6)),
        (1, 1)
    );
    run(
        &mut e,
        SET_ON_USE_AID_ITEM,
        &[form_arg(S1), int_arg(0), form_arg(AID)],
    );
    assert_eq!(
        (e.hook_users(Hook::EquipAidItem), e.mem.u8(ITEM + 6)),
        (0, 0)
    );
    site(&mut e, 0x0088_c74e, &[ITEM, ACTOR]);
    assert!(calls().is_empty());
    run(
        &mut e,
        SET_ON_PROJECTILE_IMPACT,
        &[form_arg(S2), int_arg(1), form_arg(PROJ)],
    );
    assert_eq!(e.hook_users(Hook::ProjectileImpact), 1);
    e.mem.set_u32(PROJECTILE + 0x20, PROJ_BASE);
    e.mem.set_u32(PROJECTILE + 0x88, IMPACT);
    for (i, v) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
        e.mem.set_f32(IMPACT + 4 + 4 * i as u32, v);
    }
    calls();
    assert_eq!(site(&mut e, 0x009c_20bf, &[PROJECTILE]), None);
    assert_eq!(calls(), [(SCRIPT_TWO, PROJECTILE, vec![])]);
    assert_eq!(e.mem.f32(PROJECTILE + 0x38), 3.0, "moved");
    e.mem.set_f32(PROJECTILE + 0x30, 9.0);
    e.mem.set_u16(PROJ_BASE + 0x62, 2);
    site(&mut e, 0x009c_20bf, &[PROJECTILE]);
    assert_eq!(e.mem.f32(PROJECTILE + 0x30), 9.0, "a beam stays");
    assert_eq!(calls().len(), 1);
    e.mem.set_u8(PROJ_BASE + 6, 0);
    site(&mut e, 0x009c_20bf, &[PROJECTILE]);
    assert!(calls().is_empty(), "no flag");
}

/// The critical-hit events: filters sorted out as JIP does; adding what an event covers does
/// nothing, adding removes what the new one covers first; the copy routine keeps the hit data
/// (made once from the game's heap) with JIP's location and calls the matching scripts on a
/// critical hit; the clear routine sets only the location to -1.
#[test]
fn critical_hits_and_the_hit_data() {
    let mut e = world();
    let crit = |e: &mut Engine, s: u16, add: i32, forms: &[u16]| {
        let mut args = vec![form_arg(s), int_arg(add)];
        args.extend(forms.iter().map(|&f| form_arg(f)));
        run(e, SET_ON_CRITICAL_HIT, &args);
    };
    crit(&mut e, S1, 1, &[A1, A2, WEAP]);
    crit(&mut e, S1, 1, &[A1, A2]);
    assert_eq!(e.ext::<Jip>().actor_hooks.critical_hit.len(), 1, "replaced");
    crit(&mut e, S1, 1, &[A1, A2, WEAP]);
    assert_eq!(e.ext::<Jip>().actor_hooks.critical_hit.len(), 1, "covered");
    crit(&mut e, S2, 1, &[]);
    crit(&mut e, S2, 1, &[LIST]);
    assert_eq!(
        e.ext::<Jip>().actor_hooks.critical_hit.len(),
        2,
        "not an actor, no source: nothing"
    );
    // A target that is not an actor with a source: any target; the source's filter covers the
    // first event (S1, ACTOR, OTHER), which goes.
    crit(&mut e, S1, 1, &[LIST, A2]);
    let s1_any_target = CriticalHit {
        target: 0,
        source: OTHER,
        weapon: 0,
        script: SCRIPT_FORM,
    };
    assert_eq!(
        e.ext::<Jip>().actor_hooks.critical_hit,
        [
            CriticalHit {
                script: SCRIPT_TWO,
                ..CriticalHit::default()
            },
            s1_any_target
        ]
    );
    assert_eq!(e.hook_users(Hook::OnHitEvent), 0, "no hook user");
    // The copy routine.
    for i in 0..0x18 {
        e.mem.set_u32(HIT + 4 * i, 0x100 + i);
    }
    e.mem.set_u32(HIT, OTHER);
    e.mem.set_u32(HIT + 4, ACTOR);
    e.mem.set_u32(HIT + 0x10, 3);
    e.mem.set_u32(HIT + 0x30, WEAPON);
    e.mem.set_u32(HIT + 0x58, 0);
    calls();
    e.call(COPY_HIT_DATA_ROUTINE, &[PROCESS, HIT]);
    let data = e.mem.u32(PROCESS + 0x240);
    assert_eq!(data, HEAP + 0x10);
    assert_eq!(e.mem.bytes(data, 0x60), e.mem.bytes(HIT, 0x60));
    assert_eq!(e.mem.u32(data + 0x60), 3);
    assert!(calls().is_empty(), "not critical");
    e.mem.set_u32(HIT + 0x58, 4);
    e.call(COPY_HIT_DATA_ROUTINE, &[PROCESS, HIT]);
    assert_eq!(e.mem.u32(PROCESS + 0x240), data, "made once");
    assert_eq!(
        calls(),
        [
            (SCRIPT_TWO, ACTOR, vec![OTHER, WEAPON]),
            (SCRIPT_FORM, ACTOR, vec![OTHER, WEAPON])
        ]
    );
    e.mem.set_u32(HIT, ACTOR);
    e.call(COPY_HIT_DATA_ROUTINE, &[PROCESS, HIT]);
    assert_eq!(
        calls(),
        [(SCRIPT_TWO, ACTOR, vec![ACTOR, WEAPON])],
        "another source filtered out"
    );
    e.call(CLEAR_HIT_DATA_ROUTINE, &[PROCESS]);
    assert_eq!(e.mem.u32(data + 0x10), u32::MAX);
    assert_eq!(e.mem.u32(data + 0x60), 3, "the rest kept");
    e.mem.set_u32(PROCESS + 0x240, 0);
    e.call(CLEAR_HIT_DATA_ROUTINE, &[PROCESS]);
    assert_eq!(e.mem.u32(PROCESS + 0x240), 0, "nothing made");
    // Removal of every event the filters cover.
    crit(&mut e, S1, 0, &[]);
    assert_eq!(e.ext::<Jip>().actor_hooks.critical_hit.len(), 1);
}

/// Before a load: the fire hook keeps one user per scripted actor and per script for every
/// actor (infinite-ammo users go), or none; the ragdoll hook and scripts go; the rest stays.
#[test]
fn pre_load_housekeeping() {
    let mut e = world();
    run(&mut e, SET_ON_FIRE_WEAPON, &[form_arg(S1), int_arg(1)]);
    run(
        &mut e,
        SET_ON_FIRE_WEAPON,
        &[form_arg(S1), int_arg(1), form_arg(LIST)],
    );
    e.add_hook_user(Hook::RemoveAmmo, true);
    run(&mut e, SET_ON_RAGDOLL, &[form_arg(S1), int_arg(1)]);
    run(&mut e, SET_ON_RELOAD_WEAPON, &[form_arg(S1), int_arg(1)]);
    assert_eq!(e.hook_users(Hook::RemoveAmmo), 4);
    pre_load(&mut e);
    assert_eq!(e.hook_users(Hook::RemoveAmmo), 3);
    assert_eq!(e.hook_users(Hook::OnRagdoll), 0);
    assert!(e.ext::<Jip>().actor_hooks.ragdoll.is_empty());
    assert_eq!(e.hook_users(Hook::ReloadWeapon), 1, "kept");
    let mut e = world();
    e.add_hook_user(Hook::RemoveAmmo, true);
    pre_load(&mut e);
    assert_eq!(e.hook_users(Hook::RemoveAmmo), 0);
}

/// Handler scripts that are lambdas keep their variables while they are held (JIP's
/// `LambdaVarContext`): a first add keeps them once and a duplicate add not again, a removal gives
/// them up, a critical-hit event replaced by one whose filters cover it gives up the old hold and
/// takes the new one, and the pre-load clearing of the ragdoll scripts gives them up.
#[test]
fn lambda_handlers_keep_their_variables_while_held() {
    let mut e = world();
    let (list_one, list_two) = (AREA + 0x7000, AREA + 0x7100);
    nvse::lambda::insert_for_tests(&mut e, SCRIPT_FORM, list_one);
    nvse::lambda::insert_for_tests(&mut e, SCRIPT_TWO, list_two);
    let kept = |e: &Engine, script| nvse::lambda::saved_count(e, script);
    run(&mut e, SET_ON_RELOAD_WEAPON, &[form_arg(S1), int_arg(1)]);
    run(&mut e, SET_ON_RELOAD_WEAPON, &[form_arg(S1), int_arg(1)]);
    assert_eq!(kept(&e, SCRIPT_FORM), 1, "a first add and a duplicate add");
    run(&mut e, SET_ON_RELOAD_WEAPON, &[form_arg(S1), int_arg(0)]);
    assert_eq!(kept(&e, SCRIPT_FORM), 0, "removed");
    run(
        &mut e,
        SET_ON_CRITICAL_HIT,
        &[form_arg(S1), int_arg(1), form_arg(A1)],
    );
    run(&mut e, SET_ON_CRITICAL_HIT, &[form_arg(S1), int_arg(1)]);
    assert_eq!(e.ext::<Jip>().actor_hooks.critical_hit.len(), 1);
    assert_eq!(kept(&e, SCRIPT_FORM), 1, "replaced by a covering filter");
    run(&mut e, SET_ON_RAGDOLL, &[form_arg(S2), int_arg(1)]);
    assert_eq!(kept(&e, SCRIPT_TWO), 1);
    pre_load(&mut e);
    assert_eq!(kept(&e, SCRIPT_TWO), 0, "the ragdoll scripts cleared");
    assert_eq!(kept(&e, SCRIPT_FORM), 1, "the critical hit's hold stays");
}
