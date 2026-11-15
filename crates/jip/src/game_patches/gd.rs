//! Group GD's game patches (step 3.9: projectiles, explosions, combat AI, packages): its handlers, wraps and slot routines. Only group
//! GD's PR edits this file (and its own tests file beside it).
//!
//! **G117, a projectile's ammo, count and split beams** (`InitGamePatches`,
//! `internal/patches_game.h` (JIP LN NVSE): `WriteRelJump(0x9BC241, InitProjectileSetAmmoHook)`).
//! The launch constructor (`009bbef0`) divides the weapon's damage by the exe's projectile count
//! (`TESObjectWEAP::GetNumProjectiles` `00525b20`) and scales it by the double 1.3 at `01051678`
//! when the held weapon's mod effect 0xC (split beam) is active. JIP's hook replaces that block
//! (up to `0x9BC293`): it records the weapon's equipped ammo, the split-beam flag and its own
//! count in the projectile, scales by the float 1.3 at `01022004` first, then divides by its
//! count. A JIP fix (with the row `NV_JIP_GAME_PATCHES`): split-beam and multi-projectile damage
//! change for every player. Seam in `engine-fallout-ai` `projectile.rs` (`fn_009bbef0`).
//!
//! **The record** (`measure/jip/GROUP-GD-NOTES.md`, "G117 record for GC's O1"): kept where JIP keeps
//! it, in the projectile's own bytes ([`AMMO`], [`HAS_SPLIT_BEAMS`], [`NUM_PROJECTILES`]; the
//! exe never touches them). G117 is the only writer; GC's O1 reads it through [`projectile_ammo`]
//! and the two bytes, GC's G119 reads the [`AMMO`] word directly. A projectile G117 did not set up
//! (JIP not installed, the plain constructor of a loaded projectile, no weapon) reads all zero:
//! the port's guest heap zero-fills every block (on the game's heap JIP reads whatever the two
//! bytes held for a loaded projectile).
//!
//! Threads: the constructor runs wherever a weapon fires (the main thread, the AI threads with real
//! threads on). The handler writes only the new projectile and reads forms and the shooter's
//! process; it keeps no state of JIP's and takes no slot.

use super::Area;
use crate::commands::forms::TES_AMMO_VTBL;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// G117's seam: the launch constructor's count block (words: the projectile, the weapon
/// argument, the actor shooter or 0, the mod-0xC flag).
pub const PROJECTILE_SET_AMMO: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_c241);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GD",
    fix_sites: &[PROJECTILE_SET_AMMO.addr()],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

/// The record's three places in a projectile (JIP LN NVSE's `Projectile`, GameObjects.h): the
/// ammo word in the reference's extra data list (`extraDataList.ammo`, list `+0x44` + `0x1C`), JIP's
/// `hasSplitBeams` and `numProjectiles` bytes (padding after the exe's `bSoundFired` `+0x148`).
pub const AMMO: u32 = 0x60;
pub const HAS_SPLIT_BEAMS: u32 = 0x149;
pub const NUM_PROJECTILES: u32 = 0x14a;
/// The projectile's damage (`fDamage`, JIP's `hitDamage`).
const HIT_DAMAGE: u32 = 0xdc;

/// `TESObjectWEAP` (JIP LN NVSE, GameForms.h): its ammo form (`BGSAmmoForm::ammo`, a `TESAmmo` or a
/// form list), `ammoUse`, `numProjectiles`, the three mod effects and their first values.
const WEAPON_AMMO: u32 = 0xa8;
const WEAPON_AMMO_USE: u32 = 0x102;
const WEAPON_NUM_PROJECTILES: u32 = 0x11e;
const WEAPON_EFFECT_MODS: u32 = 0x180;
const WEAPON_VALUE1_MODS: u32 = 0x18c;
/// The split-beam mod effect.
const SPLIT_BEAM_EFFECT: u32 = 0xc;
/// `TESAmmo::projPerShot` (JIP LN NVSE), read as a signed byte.
const AMMO_PROJ_PER_SHOT: u32 = 0xb0;
/// `kFormType_BGSListForm`; a form list's first node (`+0x18`) and the count JIP walks (`+0x20`).
const LIST_FORM_TYPE: u8 = 0x55;
const LIST_HEAD: u32 = 0x18;
const LIST_WALK: u32 = 0x20;
/// The float JIP scales split-beam damage by (1.3, in the exe's constants).
pub const SPLIT_BEAM_SCALE: u32 = 0x0102_2004;
/// `Actor::baseProcess` and, in a middle or high process (level 0 or 1 at `+0x28`), the weapon
/// and ammo entries (`ContChangesEntry*`, their form at `+8`).
const BASE_PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const PROCESS_WEAPON_INFO: u32 = 0x114;
const PROCESS_AMMO_INFO: u32 = 0x118;
const ENTRY_FORM: u32 = 8;

fn install_fixes(e: &mut Engine) {
    e.on_site(PROJECTILE_SET_AMMO, projectile_set_ammo);
}

/// JIP's `cvttss2si`: truncated toward zero; NaN or out of range gives `0x80000000`.
fn cvttss2si(value: f32) -> i32 {
    if value.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESObjectWEAP::GetAmmo.
/// The weapon's ammo: its ammo form, or for a form list the item at the list's `+0x20` count
/// from the head when the list is that long, else the first item.
pub fn weapon_ammo(e: &Engine, weapon: u32) -> u32 {
    let form = e.mem.u32(weapon.wrapping_add(WEAPON_AMMO));
    if form == 0 || e.mem.u8(form.wrapping_add(4)) != LIST_FORM_TYPE {
        return form;
    }
    let mut node = form.wrapping_add(LIST_HEAD);
    let mut count = e.mem.u32(form.wrapping_add(LIST_WALK)) as i32;
    loop {
        count = count.wrapping_sub(1);
        if count < 0 {
            return e.mem.u32(node);
        }
        node = e.mem.u32(node.wrapping_add(4));
        if node == 0 {
            return e.mem.u32(form.wrapping_add(LIST_HEAD));
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESObjectWEAP::GetEquippedAmmo.
/// The ammo `actor` has loaded in `weapon` (its middle or high process's ammo entry, when its
/// weapon entry is this weapon and the entry's form is a `TESAmmo`), else [`weapon_ammo`]. JIP
/// first answers 0 for its Rock-It Launcher (`g_rockItLauncher`, set only by
/// `InitRockItLauncher`, not built: null in the port, so never this weapon).
pub fn equipped_ammo(e: &Engine, weapon: u32, actor: u32) -> u32 {
    if actor != 0 {
        let process = e.mem.u32(actor.wrapping_add(BASE_PROCESS));
        if process != 0 && e.mem.u8(process.wrapping_add(PROCESS_LEVEL)) <= 1 {
            let weapon_info = e.mem.u32(process.wrapping_add(PROCESS_WEAPON_INFO));
            if weapon_info != 0 && e.mem.u32(weapon_info.wrapping_add(ENTRY_FORM)) == weapon {
                let ammo_info = e.mem.u32(process.wrapping_add(PROCESS_AMMO_INFO));
                if ammo_info != 0 {
                    let ammo = e.mem.u32(ammo_info.wrapping_add(ENTRY_FORM));
                    if ammo != 0 && e.mem.u32(ammo) == TES_AMMO_VTBL {
                        return ammo;
                    }
                }
            }
        }
    }
    weapon_ammo(e, weapon)
}

/// JIP's `Projectile::GetAmmo` (GameObjects.h): the recorded ammo when the recorded count is not
/// 0, else 0. For GC's O1.
pub fn projectile_ammo(e: &Engine, projectile: u32) -> u32 {
    if e.mem.u8(projectile.wrapping_add(NUM_PROJECTILES)) != 0 {
        e.mem.u32(projectile.wrapping_add(AMMO))
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitProjectileSetAmmoHook.
/// The launch constructor's count block (`0x9BC241`; the projectile and the weapon never 0 here,
/// the block is inside the constructor's `weapon != 0` branch): records the equipped ammo and the
/// split-beam flag, counts the projectiles (the ammo's projectiles per shot times the weapon's
/// ammo use when that signed byte is above 0, else the weapon's count; plus the split-beam mod's
/// value), scales the damage by 1.3 with split beams and divides it by the count when the count's
/// low byte is above 1. Answers 0: the exe's block is skipped.
fn projectile_set_ammo(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (projectile, weapon, actor, split) = (a[0], a[1], a[2], a[3] & 0xff != 0);
    let ammo = equipped_ammo(e, weapon, actor);
    e.mem.set_u32(projectile.wrapping_add(AMMO), ammo);
    let mut damage = e.mem.f32(projectile.wrapping_add(HIT_DAMAGE));
    let mut count = u32::from(e.mem.u8(weapon.wrapping_add(WEAPON_NUM_PROJECTILES)));
    if ammo != 0 {
        let per_shot = i32::from(e.mem.u8(ammo.wrapping_add(AMMO_PROJ_PER_SHOT)) as i8);
        if per_shot > 0 {
            let ammo_use = i32::from(e.mem.u8(weapon.wrapping_add(WEAPON_AMMO_USE)));
            count = ammo_use.wrapping_mul(per_shot) as u32;
        }
    }
    e.mem
        .set_u8(projectile.wrapping_add(HAS_SPLIT_BEAMS), u8::from(split));
    if split {
        damage *= e.mem.f32(SPLIT_BEAM_SCALE);
        let effects = weapon.wrapping_add(WEAPON_EFFECT_MODS);
        if let Some(slot) =
            (0..3u32).find(|i| e.mem.u32(effects.wrapping_add(4 * i)) == SPLIT_BEAM_EFFECT)
        {
            let value = e
                .mem
                .f32(weapon.wrapping_add(WEAPON_VALUE1_MODS + 4 * slot));
            count = count.wrapping_add(cvttss2si(value) as u32);
        }
    }
    e.mem
        .set_u8(projectile.wrapping_add(NUM_PROJECTILES), count as u8);
    if count as u8 > 1 {
        damage /= count as i32 as f32;
    }
    e.mem.set_f32(projectile.wrapping_add(HIT_DAMAGE), damage);
    Some(0)
}

#[cfg(test)]
#[path = "gd_tests.rs"]
mod tests;
