//! `InitRockItLauncher`, called by its opcode, and the two wraps and two seams it turns on.
//! Expectations from JIP's bodies (`Cmd_InitRockItLauncher_Execute`, `GetWeaponProjectileHook`,
//! `RILAmmoFixHook`, `PCAmmoSwitchHook`, `TESObjectWEAP::GetEquippedAmmo`), the exe's two
//! functions as stand-ins answering 0x5A5A.

use super::*;
use crate::commands::test_support::*;

const LAUNCHER: u32 = FORMS + 0x1000;
const RIFLE: u32 = FORMS + 0x1400;
const AMMO: u32 = FORMS + 0x1800;
const ENTRY: u32 = FORMS + 0x1c00;

/// JIP installed over stand-ins of the two weapon functions, forms in memory, the HUD's page
/// mapped.
fn world() -> Engine {
    let mut e = crate::test_support::engine();
    e.register(EQUIPPED_AMMO.addr(), |_, _| eax(0x5a5a));
    e.register(WEAPON_PROJECTILE.addr(), |_, _| eax(0x5a5a));
    crate::install_as(&mut e, true);
    e.map(FORMS, 0x10_0000);
    if !e.mem.is_mapped(0x011d_9000) {
        e.map(0x011d_9000, 0x1000);
    }
    form(&mut e, LAUNCHER, 0x28, 0x0700_1000);
    form(&mut e, RIFLE, 0x28, 0x0000_1400);
    form(&mut e, AMMO, TES_AMMO, 0x0000_1800);
    // The rifle's ammo (`+0xA8`) and projectile (`+0x118`); the ammo's projectile (`+0xB4`).
    e.mem.set_u32(RIFLE + 0xa8, AMMO);
    // The launcher has ammo of its own too: JIP's GetEquippedAmmo still answers none for it.
    e.mem.set_u32(LAUNCHER + 0xa8, AMMO);
    e.mem.set_u32(RIFLE + 0x118, 0x0bad_0118);
    e.mem.set_u32(LAUNCHER + 0x118, 0x0bad_1118);
    e.mem.set_u32(AMMO + AMMO_PROJECTILE, 0x0bad_00b4);
    set_ref(&mut e, 1, LAUNCHER);
    set_ref(&mut e, 2, RIFLE);
    e
}

#[test]
fn before_the_command_everything_is_the_exes() {
    let mut e = world();
    assert_eq!(e.call(EQUIPPED_AMMO.addr(), &[RIFLE, 0]).eax, 0x5a5a);
    assert_eq!(e.call(WEAPON_PROJECTILE.addr(), &[RIFLE, 0]).eax, 0x5a5a);
    e.mem.set_u32(ENTRY + 8, LAUNCHER);
    assert_eq!(e.site(AMMO_SWAP.addr(), &[ENTRY]), None);
    assert_eq!(e.site(AMMO_SWITCH.addr(), &[]), None);
}

/// After InitRockItLauncher (once: a second weapon is not taken): JIP's equipped ammo (none for
/// the launcher, the weapon's own ammo for another without an actor), the projectile of that
/// ammo or the weapon's own, no ammo swap and no ammo switch for the launcher.
#[test]
fn the_launcher_has_no_ammo() {
    let mut e = world();
    call(&mut e, 0x28d7, &[r(1)]);
    call(&mut e, 0x28d7, &[r(2)]);
    assert_eq!(launcher(&mut e), LAUNCHER);
    assert_eq!(e.call(EQUIPPED_AMMO.addr(), &[LAUNCHER, 0]).eax, 0);
    assert_eq!(e.call(EQUIPPED_AMMO.addr(), &[RIFLE, 0]).eax, AMMO);
    assert_eq!(
        e.call(WEAPON_PROJECTILE.addr(), &[RIFLE, 0]).eax,
        0x0bad_00b4,
        "the ammo's"
    );
    assert_eq!(
        e.call(WEAPON_PROJECTILE.addr(), &[LAUNCHER, 0]).eax,
        0x0bad_1118,
        "no ammo: the weapon's own"
    );
    e.mem.set_u32(AMMO + AMMO_PROJECTILE, 0);
    assert_eq!(
        e.call(WEAPON_PROJECTILE.addr(), &[RIFLE, 0]).eax,
        0x0bad_0118
    );
    e.mem.set_u32(AMMO + AMMO_PROJECTILE, 0x0bad_00b4);
    e.mem.set_u8(AMMO + 4, 0x55);
    assert_eq!(
        e.call(WEAPON_PROJECTILE.addr(), &[RIFLE, 0]).eax,
        0x0bad_0118,
        "not a TESAmmo"
    );
    e.mem.set_u32(ENTRY + 8, LAUNCHER);
    assert_eq!(e.site(AMMO_SWAP.addr(), &[ENTRY]), Some(0));
    e.mem.set_u32(ENTRY + 8, RIFLE);
    assert_eq!(e.site(AMMO_SWAP.addr(), &[ENTRY]), Some(ENTRY));
    e.mem.set_u32(HUD_WEAPON, LAUNCHER);
    assert_eq!(e.site(AMMO_SWITCH.addr(), &[]), Some(1));
    e.mem.set_u32(HUD_WEAPON, RIFLE);
    assert_eq!(e.site(AMMO_SWITCH.addr(), &[]), Some(0));
    e.mem.set_u32(HUD_WEAPON, 0);
    assert_eq!(e.site(AMMO_SWITCH.addr(), &[]), Some(0), "no weapon: on");
}
