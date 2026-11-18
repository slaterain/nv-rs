//! `GetCalculatedWeaponDamage`, called by its opcode. Expectations from JIP's bodies
//! (functions_jip/jip_fn_weapon.h: `Cmd_GetCalculatedWeaponDamage_Execute`; nvse/GameExtraData.cpp:
//! `ContChangesEntry::CalculateWeaponDamage`), never from the port's output. The game's three
//! functions are stand-ins: the weapon damage answers 10, the ammo effects double the value and
//! add the kind, the perk entry point triples the float it is given.

use super::*;
use crate::commands::test_support::*;

const WEAP: u32 = FORMS + 0x1000;
const AMMO: u32 = FORMS + 0x1400;
const AMMO2: u32 = FORMS + 0x1500;
const MISC: u32 = FORMS + 0x1600;
const PLAYER: u32 = FORMS + 0x2000;
const NPC: u32 = FORMS + 0x2400;
const WREF: u32 = FORMS + 0x2800;
const PROC: u32 = FORMS + 0x3000;
const NPC_PROC: u32 = FORMS + 0x3100;
const INFO: u32 = FORMS + 0x3200;
const AMMO_ENTRY: u32 = FORMS + 0x3300;
const STACK: u32 = FORMS + 0x3400;
const STACK_LISTS: u32 = FORMS + 0x3500;
const XDL: u32 = FORMS + 0x3600;
const X1: u32 = FORMS + 0x3700;
const TEMP_REF: u32 = FORMS + 0x3800;
const VTBL: u32 = FORMS + 0x4000;
const PLAIN_VTBL: u32 = FORMS + 0x4400;
const EFFECTS: u32 = FORMS + 0x4800;
/// What the weapon damage stand-in saw: the process's weapon entry at the call.
const INFO_SEEN: u32 = 0x0dea_5000;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    e.map(0x011d_e000, 0x1000);
    e.map(0x0102_2000, 0x1000);
    e.mem.set_f32(DAMAGE_MOD_MULT, 1.3);
    form(&mut e, WEAP, 0x28, 0x3ea9);
    form(&mut e, AMMO, 0x29, 0x3eb0);
    form(&mut e, AMMO2, 0x29, 0x3eb1);
    form(&mut e, MISC, 0x1f, 0x0999);
    form(&mut e, PLAYER, 0x3b, 0x14);
    form(&mut e, NPC, 0x3b, 0x7000);
    form(&mut e, WREF, 0x3a, 0x7100);
    e.mem.set_u32(WEAP, PLAIN_VTBL + 0x400);
    e.mem.set_u32(MISC, PLAIN_VTBL + 0x400);
    for a in [AMMO, AMMO2] {
        e.mem.set_u32(a, super::super::forms::TES_AMMO_VTBL);
        e.mem.set_u32(a + 0xd4, EFFECTS);
    }
    e.mem.set_u32(WEAP + 0xa8, AMMO);
    for a in [PLAYER, NPC] {
        e.mem.set_u32(a, VTBL);
    }
    e.mem.set_u32(VTBL + 0x100, 0x008d_0360);
    e.mem.set_u32(WREF, PLAIN_VTBL);
    e.mem.set_u32(PLAIN_VTBL + 0x100, 0x0047_c850);
    e.mem.set_u32(WREF + 0x20, WEAP);
    e.mem.set_u32(0x011d_ea3c, PLAYER);
    e.mem.set_u32(PLAYER + 0x68, PROC);
    e.mem.set_u32(PROC + 0x114, INFO);
    e.mem.set_u32(INFO + 8, MISC);
    e.mem.set_u32(NPC + 0x68, NPC_PROC);
    e.register(CALC_WEAPON_DAMAGE, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((CALC_WEAPON_DAMAGE, w.to_vec())));
        let process = e.mem.u32(e.mem.u32(w[0] - 0xa4 + 0x68) + 0x114);
        CALLS.with(|c| c.borrow_mut().push((INFO_SEEN, vec![process])));
        f32::into_ret(10.0)
    });
    e.register(APPLY_AMMO_EFFECTS, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((APPLY_AMMO_EFFECTS, w.to_vec())));
        f32::into_ret(f32::from_bits(w[2]) * 2.0 + w[0] as f32)
    });
    e.register(APPLY_PERK_MODIFIERS, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((APPLY_PERK_MODIFIERS, w.to_vec())));
        e.mem.set_f32(w[4], e.mem.f32(w[4]) * 3.0);
        Ret::default()
    });
    set_ref(&mut e, 1, WEAP);
    set_ref(&mut e, 2, MISC);
    e
}

fn run(e: &mut Engine, this: u32, args: &[Vec<u8>]) -> f64 {
    call_on(e, 0x27ed, this, args).1
}

/// An inventory reference of `WEAP` in `container`'s stack `STACK` (its lists `STACK_LISTS`,
/// holding `XDL`), as reference 3.
fn inventory_ref(e: &mut Engine, container: u32) -> u32 {
    e.register(0x0040_1000, |_, _| Ret {
        eax: TEMP_REF,
        ..Ret::default()
    });
    e.register(0x0055_a2f0, |e, w| {
        e.mem.set_u32(w[0] + 0xc, 0xff00_0900);
        Ret::default()
    });
    e.mem.set_u32(STACK, STACK_LISTS);
    e.mem.set_u32(STACK + 8, WEAP);
    e.mem.set_u32(STACK_LISTS, XDL);
    let refr = nvse::invref::create(
        e,
        container,
        nvse::invref::Data {
            item: WEAP,
            entry: STACK,
            xdata: XDL,
        },
        false,
    );
    e.mem.set_u32(refr, PLAIN_VTBL);
    e.mem.set_u32(refr + 0x20, WEAP);
    set_ref(e, 3, refr);
    refr
}

/// A weapon named: the player's, at full condition, the weapon's own ammo; the process's weapon
/// entry away during the damage and back after; the ammo effects (kind 0) then the player's perk
/// entry point (kind 0, the player, the weapon, the player, the value).
#[test]
fn a_named_weapon_for_the_player() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0, &[r(1)]), f64::from((10.0f32 * 2.0) * 3.0));
    let calc = calls_to(CALC_WEAPON_DAMAGE);
    let entry = calc[0][6];
    assert_eq!(
        calc[0],
        vec![
            PLAYER + 0xa4,
            WEAP,
            1.0f32.to_bits(),
            1.0f32.to_bits(),
            0,
            0,
            entry,
            1
        ]
    );
    assert_eq!(calls_to(INFO_SEEN), vec![vec![0]], "the weapon entry away");
    assert_eq!(e.mem.u32(PROC + 0x114), INFO, "and back");
    assert_eq!(
        calls_to(APPLY_AMMO_EFFECTS),
        vec![vec![0, AMMO + 0xd4, 10.0f32.to_bits()]]
    );
    let perk = calls_to(APPLY_PERK_MODIFIERS);
    assert_eq!(perk[0][..4], [0, PLAYER, WEAP, PLAYER]);
}

/// No ammo effects: no call; ammo that is not a `TESAmmo`: none (the flag 0); no ammo at all.
#[test]
fn the_ammo_must_be_ammo_with_effects() {
    let mut e = setup();
    e.mem.set_u32(AMMO + 0xd4, 0);
    assert_eq!(run(&mut e, 0, &[r(1)]), 30.0);
    assert!(calls_to(APPLY_AMMO_EFFECTS).is_empty());
    assert_eq!(calls_to(CALC_WEAPON_DAMAGE)[0][7], 1, "ammo still");
    e.mem.set_u32(WEAP + 0xa8, MISC);
    run(&mut e, 0, &[r(1)]);
    assert_eq!(calls_to(CALC_WEAPON_DAMAGE)[1][7], 0, "not a TESAmmo");
    e.mem.set_u32(WEAP + 0xa8, 0);
    run(&mut e, 0, &[r(1)]);
    assert_eq!(calls_to(CALC_WEAPON_DAMAGE)[2][7], 0);
}

/// Not a weapon, or no weapon and no reference, or a reference whose base is not a weapon: 0 (the
/// result as the command is given it) and no game call.
#[test]
fn not_a_weapon_answers_nothing() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0, &[r(2)]), 0.0);
    assert_eq!(run(&mut e, 0, &[]), 0.0);
    e.mem.set_u32(WREF + 0x20, MISC);
    assert_eq!(run(&mut e, WREF, &[]), 0.0);
    assert!(calls_to(CALC_WEAPON_DAMAGE).is_empty());
}

/// The calling reference's weapon, not an inventory reference: the player's, the reference's own
/// extra data as the stack's (its weapon mod flags read there: a mod with effect 0xC multiplies by
/// the exe's 1.3), its condition (no health: 100%).
#[test]
fn a_reference_on_the_ground() {
    let mut e = setup();
    e.mem.set_u32(AMMO + 0xd4, 0);
    assert_eq!(run(&mut e, WREF, &[]), 30.0);
    let calc = calls_to(CALC_WEAPON_DAMAGE);
    assert_eq!(calc[0][2], 1.0f32.to_bits(), "full condition");
    assert_eq!(
        calls_to(APPLY_PERK_MODIFIERS)[0][..4],
        [0, PLAYER, WEAP, PLAYER]
    );
    // Mod slot 1 with effect 0xC, its flag on the reference's own list.
    e.mem.set_u32(WEAP + 0x180, 0xc);
    put_extra(&mut e, WREF + 0x44, X1, 0x8d, 1);
    let want = f64::from((f64::from(10.0f32) * f64::from(1.3f32)) as f32 * 3.0);
    assert_eq!(run(&mut e, WREF, &[]), want);
}

/// An inventory reference in an NPC's inventory: the NPC holds it; the loaded ammo when its
/// process's weapon entry is this weapon; no perk entry point for an NPC that is not a teammate
/// (unless JIP's NPC perks patch is installed), one for a teammate.
#[test]
fn an_inventory_reference_held_by_an_npc() {
    let mut e = setup();
    let refr = inventory_ref(&mut e, NPC);
    e.mem.set_u32(NPC_PROC + 0x114, STACK);
    e.mem.set_u32(NPC_PROC + 0x118, AMMO_ENTRY);
    e.mem.set_u32(AMMO_ENTRY + 8, AMMO2);
    assert_eq!(run(&mut e, refr, &[]), 20.0, "no perks for this NPC");
    assert_eq!(calls_to(CALC_WEAPON_DAMAGE)[0][0], NPC + 0xa4);
    assert_eq!(
        calls_to(APPLY_AMMO_EFFECTS)[0][1],
        AMMO2 + 0xd4,
        "the loaded ammo"
    );
    assert_eq!(e.mem.u32(NPC_PROC + 0x114), STACK, "put back");
    assert!(calls_to(APPLY_PERK_MODIFIERS).is_empty());
    // Another weapon equipped: the weapon's own ammo.
    e.mem.set_u32(NPC_PROC + 0x114, INFO);
    run(&mut e, refr, &[]);
    assert_eq!(calls_to(APPLY_AMMO_EFFECTS)[1][1], AMMO + 0xd4);
    // A teammate; then the patch installed.
    e.mem.set_u8(NPC + 0x18d, 1);
    assert_eq!(run(&mut e, refr, &[]), 60.0);
    e.mem.set_u8(NPC + 0x18d, 0);
    crate::cosave::set_npc_perks_patch(&mut e, true);
    assert_eq!(run(&mut e, refr, &[]), 60.0);
}

/// A low process (level above 1): its weapon entry is left alone.
#[test]
fn a_low_process_keeps_its_weapon_entry() {
    let mut e = setup();
    e.mem.set_u8(PROC + 0x28, 2);
    run(&mut e, 0, &[r(1)]);
    assert_eq!(calls_to(INFO_SEEN), vec![vec![INFO]]);
}

/// A named weapon the player has equipped with other ammo loaded: still the weapon's own ammo
/// (JIP takes the loaded ammo only for an inventory reference).
#[test]
fn the_loaded_ammo_only_for_an_inventory_reference() {
    let mut e = setup();
    e.mem.set_u32(INFO + 8, WEAP);
    e.mem.set_u32(PROC + 0x118, AMMO_ENTRY);
    e.mem.set_u32(AMMO_ENTRY + 8, AMMO2);
    run(&mut e, 0, &[r(1)]);
    assert_eq!(calls_to(APPLY_AMMO_EFFECTS)[0][1], AMMO + 0xd4);
}

/// No perk step (an NPC that is not a teammate, the NPC perks patch off) with the damage mod:
/// the value still ends as a float, as JIP's function returns one: 10 x 1.3 is exactly 13.0
/// (12.99999952... rounds to even), never the double 12.9999995.
#[test]
fn the_damage_mod_without_perks_ends_as_a_float() {
    let mut e = setup();
    let refr = inventory_ref(&mut e, NPC);
    e.mem.set_u32(AMMO + 0xd4, 0);
    e.mem.set_u32(WEAP + 0x180, 0xc);
    put_extra(&mut e, XDL, X1, 0x8d, 1);
    assert_eq!(run(&mut e, refr, &[]), 13.0);
    assert!(calls_to(APPLY_PERK_MODIFIERS).is_empty());
}
