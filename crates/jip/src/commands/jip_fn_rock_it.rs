//! `InitRockItLauncher` (`functions_jip/jip_fn_miscellaneous.h`, JIP LN NVSE): a weapon that
//! fires the items it is loaded with (JIP CC&C's Rock-It Launcher) has no ammo and no ammo swap.
//! JIP keeps the weapon in `g_rockItLauncher` and jumps from four places of the game's code to
//! its own: the weapon's equipped ammo (`TESObjectWEAP::GetEquippedAmmo`, `00525980`) and its
//! projectile (`00525A90`), both whole functions reached only by address, so wraps here; and two
//! places in the player's code (the ammo swap's weapon entry at `0x946310`, the ammo switch key's
//! weapon test at `0x940998`), seams in engine-fallout-ai's `playercharacter.rs`. All four are
//! put in only while JIP answers "installed" ([`AREA`]), and do what the exe does until the
//! command has named the weapon. The wraps keep every caller: `00525980` has its row in
//! engine-core's `data/overridable.tsv`, so no typed code calls it directly. Run-wide,
//! lock-free; never saved (JIP's global lives until the game ends).

use super::Handlers;
use crate::game_patches::Area;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicU32, Ordering};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x28d7, init_rock_it_launcher),
];

pub const EQUIPPED_AMMO: WrapSlot = WrapSlot::new(Owner::Jip, 0x0052_5980);
pub const WEAPON_PROJECTILE: WrapSlot = WrapSlot::new(Owner::Jip, 0x0052_5a90);
pub const AMMO_SWAP: SiteSlot = SiteSlot::new(Owner::Jip, 0x0094_6310);
pub const AMMO_SWITCH: SiteSlot = SiteSlot::new(Owner::Jip, 0x0094_0998);

/// `g_rockItLauncher` (0 until InitRockItLauncher).
#[derive(Default)]
pub struct RockIt {
    launcher: AtomicU32,
}

/// JIP's Rock-It Launcher, 0 before InitRockItLauncher.
pub fn launcher(e: &mut Engine) -> u32 {
    e.ext_free::<RockIt>().launcher.load(Ordering::Relaxed)
}

/// The slot (made before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<RockIt>();
}

/// The two wraps and the two seams: in with JIP's `INSTALLED`, as JIP's other patches (with JIP
/// not installed the game's functions are not wrapped at all).
pub const AREA: Area = Area {
    name: "Rock-It Launcher",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[AMMO_SWAP.addr(), AMMO_SWITCH.addr()],
    jip_wraps: &[EQUIPPED_AMMO.addr(), WEAPON_PROJECTILE.addr()],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.wrap(EQUIPPED_AMMO, equipped_ammo);
    e.wrap(WEAPON_PROJECTILE, weapon_projectile);
    e.on_site(AMMO_SWAP, ammo_swap);
    e.on_site(AMMO_SWITCH, ammo_switch);
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// `TESObjectWEAP::GetEquippedAmmo` (thiscall: the weapon, the actor): JIP's
/// ([`crate::game_patches::gd::equipped_ammo`]: none for the launcher) once the launcher is
/// named, else the exe's.
fn equipped_ammo(e: &mut Engine, w: &[u32]) -> Ret {
    if launcher(e) == 0 {
        return e.call_next(EQUIPPED_AMMO.addr(), w);
    }
    eax(crate::game_patches::gd::equipped_ammo(e, w[0], w[1]))
}

/// `kFormType_TESAmmo`; an ammo's projectile (`+0xB4`); a weapon's (`+0x118`).
const TES_AMMO: u8 = 0x29;
const AMMO_PROJECTILE: u32 = 0xb4;
const WEAPON_OWN_PROJECTILE: u32 = 0x118;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// GetWeaponProjectileHook (asm).
/// The weapon's projectile for the actor (thiscall: the weapon, the actor), once the launcher is
/// named: the projectile of the ammo JIP's `GetEquippedAmmo` gives, when that is a `TESAmmo`
/// with one; else the weapon's own. Before, the exe's.
fn weapon_projectile(e: &mut Engine, w: &[u32]) -> Ret {
    if launcher(e) == 0 {
        return e.call_next(WEAPON_PROJECTILE.addr(), w);
    }
    let weapon = w[0];
    let ammo = crate::game_patches::gd::equipped_ammo(e, weapon, w[1]);
    if ammo != 0 && e.mem.u8(ammo.wrapping_add(4)) == TES_AMMO {
        let projectile = e.mem.u32(ammo.wrapping_add(AMMO_PROJECTILE));
        if projectile != 0 {
            return eax(projectile);
        }
    }
    eax(e.mem.u32(weapon.wrapping_add(WEAPON_OWN_PROJECTILE)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// RILAmmoFixHook (asm).
/// The ammo swap's weapon entry (`a`: the entry the first slot `0x148` call gave), once the
/// launcher is named: that entry, or 0 when its form (`+8`) is the launcher. Before, nothing
/// (the exe's second call).
fn ammo_swap(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let rock_it = launcher(e);
    if rock_it == 0 {
        return None;
    }
    let entry = a[0];
    Some(if e.mem.u32(entry.wrapping_add(8)) == rock_it {
        0
    } else {
        entry
    })
}

/// The HUD's weapon form (`*011D98D4`, which `PCAmmoSwitchHook` compares).
const HUD_WEAPON: u32 = 0x011d_98d4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// PCAmmoSwitchHook (asm).
/// The ammo switch key's weapon test, once the launcher is named: 1 (no switch) when the HUD's
/// weapon is the launcher, else 0 (on, without the exe's weapon entry test). Before, nothing.
///
/// JIP puts this jump in only while the byte at `0x781EE6` is not `0xEB` (another plugin's patch
/// of the HUD's ammo display); nothing in the port writes that byte, so it is always put in.
fn ammo_switch(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let rock_it = launcher(e);
    if rock_it == 0 {
        return None;
    }
    Some(u32::from(e.mem.u32(HUD_WEAPON) == rock_it))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_InitRockItLauncher_Execute.
/// `InitRockItLauncher weapon` (once: a second call does nothing): the weapon fires without ammo
/// and has no ammo swap or switch.
fn init_rock_it_launcher(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if launcher(e) != 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok {
        let form = read.form(0);
        e.ext_free::<RockIt>()
            .launcher
            .store(form, Ordering::Relaxed);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_rock_it_tests.rs"]
mod tests;
