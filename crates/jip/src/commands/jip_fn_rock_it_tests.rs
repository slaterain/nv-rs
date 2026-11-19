//! `InitRockItLauncher`, called by its opcode, and the two wraps and two seams it turns on.
//! Expectations from JIP's bodies (`Cmd_InitRockItLauncher_Execute`, `GetWeaponProjectileHook`,
//! `RILAmmoFixHook`, `PCAmmoSwitchHook`, `TESObjectWEAP::GetEquippedAmmo`), the exe's two
//! functions as stand-ins answering from their two words (so a wrap that changes them shows).

use super::*;
use crate::commands::test_support::*;

const LAUNCHER: u32 = FORMS + 0x1000;
const RIFLE: u32 = FORMS + 0x1400;
const AMMO: u32 = FORMS + 0x1800;
const ENTRY: u32 = FORMS + 0x1c00;
const ACTOR: u32 = FORMS + 0x2000;

/// What the stand-ins of the exe's two functions answer for their words.
fn ammo_of(w: &[u32]) -> u32 {
    w[0] ^ w[1].rotate_left(8)
}
fn projectile_of(w: &[u32]) -> u32 {
    w[0] ^ w[1].rotate_left(16)
}

/// JIP installed over stand-ins of the two weapon functions, forms in memory, the HUD's page
/// mapped.
fn world() -> Engine {
    world_as(true)
}

/// [`world`] with JIP answering "installed" or not.
fn world_as(installed: bool) -> Engine {
    let mut e = crate::test_support::engine();
    e.register(EQUIPPED_AMMO.addr(), |_, w| eax(ammo_of(w)));
    e.register(WEAPON_PROJECTILE.addr(), |_, w| eax(projectile_of(w)));
    crate::install_as(&mut e, installed);
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

/// Before InitRockItLauncher both wraps hand the exe's functions the caller's two words and give
/// back their answer; the seams do not answer.
#[test]
fn before_the_command_everything_is_the_exes() {
    let mut e = world();
    assert_eq!(e.wraps_at(EQUIPPED_AMMO.addr()), vec![Owner::Jip]);
    assert_eq!(e.wraps_at(WEAPON_PROJECTILE.addr()), vec![Owner::Jip]);
    let words = [RIFLE, ACTOR];
    assert_eq!(e.call(EQUIPPED_AMMO.addr(), &words).eax, ammo_of(&words));
    assert_eq!(
        e.call(WEAPON_PROJECTILE.addr(), &words).eax,
        projectile_of(&words)
    );
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

/// With JIP answering "not installed" nothing is put in (the wraps, the two seams and the camera's
/// two), and the exe's functions answer directly.
#[test]
fn not_installed_nothing_is_put_in() {
    let mut e = world_as(false);
    for wrap in [EQUIPPED_AMMO, WEAPON_PROJECTILE] {
        assert!(e.wraps_at(wrap.addr()).is_empty(), "{:08x}", wrap.addr());
    }
    for site in [
        AMMO_SWAP,
        AMMO_SWITCH,
        super::super::jip_fn_camera::VANITY_WHEEL,
        super::super::jip_fn_camera::CAMERA_COLLISION,
    ] {
        assert!(e.site_owners(site.addr()).is_empty(), "{:08x}", site.addr());
        assert_eq!(e.site(site.addr(), &[ENTRY]), None);
    }
    let words = [RIFLE, ACTOR];
    assert_eq!(e.call(EQUIPPED_AMMO.addr(), &words).eax, ammo_of(&words));
    assert_eq!(
        e.call(WEAPON_PROJECTILE.addr(), &words).eax,
        projectile_of(&words)
    );
}
