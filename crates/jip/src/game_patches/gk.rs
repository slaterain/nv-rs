//! Group GK's game patches (step 3.9: lights in combat): JIP LN NVSE's projectile lights (G95), muzzle flash light
//! (G96) and explosion light (G97), `InitGamePatches` (`internal/patches_game.h` (JIP LN NVSE)), fix rows: in with
//! the game-patch row `NV_JIP_GAME_PATCHES` once JIP answers "installed". Only group GK's PR edits this file (and
//! its tests file beside it).
//!
//! - G95 (`WriteRelCall(0x9BDC92, AddProjectileLightHook)`, `WriteRelCall(0x9BC6D9,
//!   RemoveProjectileLightHook)`): a projectile whose form has a light (`BGSProjectile` +0x70) gets one of JIP's
//!   point lights from that form, under its model's `AttachLight` node or its root ([`add_projectile_light`]),
//!   and the light is taken off its node when the projectile goes ([`remove_projectile_light`]); JIP's light
//!   list (group GL's H12, [`super::gl::lights`]) sets it from its form, puts it into the scene's lights and
//!   frees it once let go. The game's own call there makes its light with `TESObjectLIGH::GenDynamic`, whose
//!   radius for a reference is the reference's own radius (`TESObjectREFR::GetRadius` `00568cb0`: 0 for any
//!   reference that is not a light or one of the two marker forms), so a projectile's light lights nothing.
//!   This is what makes `SetProjectileLight` (and a plugin's projectile light) show in the game.
//! - G96 (`WriteRelJump(0x9BB906, SetMuzzleFlashFadeHook)`): `MuzzleFlash::UpdateLight` gives the flash's light
//!   the dimmer of its light form (`BGSProjectile` +0x74, the form's fade +0xB4) while the flash is on, 0 when
//!   off ([`muzzle_flash_fade`]); the game gives 1.0 (`0050dd20`), whatever the form's fade.
//! - G97 (`WriteRelCall(0x9AE68C, SetExplosionLightFadeHook)`): an explosion's light fades as the game's does
//!   (`1 - 2 * age / life`, held between 0 and 1) times its light form's fade (`BGSExplosion` +0x80, +0xB4)
//!   ([`explosion_light_fade`]); the game's is not multiplied by it.
//!
//! As JIP, the two fades are written to the light's dimmer (+0xC4) only: the game's setter (`0050dd20`) also
//! counts the light's change number (+0xA8, `004bc2c0`), which JIP's code does not.
//!
//! Guards of nv-rs's own, with the missing-data guard row (`NV_NVSE_NULL_GUARD`, the fixes preset; off, as JIP,
//! which reads through null and crashes): a muzzle flash whose projectile form or muzzle flash light is gone, and
//! an explosion whose form has no light (a script can take a form's light away while its flash or explosion is
//! alive), get the game's own fade ([`missing_guard`]).
//!
//! Threads: projectiles, flashes and explosions are updated where the game's frame runs them (the main thread
//! and the AI threads); the handlers keep no state, and the light list is group GL's (a mutex).

use super::gl::lights::create_point_light;
use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// G95: `Projectile`'s start in the world (`009bda10`), its call of the light maker `009c3d90` (`0x9BDC92`;
/// words: the projectile; answered: the call made).
pub const ADD_PROJECTILE_LIGHT: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_dc92);
/// G95: `Projectile`'s destructor (`009bc470`), its call of the light remover `009c3e10` (`0x9BC6D9`; words: the
/// projectile; answered: the call made).
pub const REMOVE_PROJECTILE_LIGHT: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_c6d9);
/// G96: `MuzzleFlash::UpdateLight` (`009bb8a0`) at its dimmer (`0x9BB906`; words: the flash, its light;
/// answered: the dimmer set).
pub const MUZZLE_FLASH_FADE: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_b906);
/// G97: the explosion's update (`009ae5d0`), its call of the light fade `009b1320` (`0x9AE68C`; words: the
/// explosion; answered: the call made).
pub const EXPLOSION_LIGHT_FADE: SiteSlot = SiteSlot::new(Owner::Jip, 0x009a_e68c);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GK",
    fix_sites: &[
        EXPLOSION_LIGHT_FADE.addr(),
        MUZZLE_FLASH_FADE.addr(),
        REMOVE_PROJECTILE_LIGHT.addr(),
        ADD_PROJECTILE_LIGHT.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

fn install_fixes(e: &mut Engine) {
    e.on_site(ADD_PROJECTILE_LIGHT, add_projectile_light);
    e.on_site(REMOVE_PROJECTILE_LIGHT, remove_projectile_light);
    e.on_site(MUZZLE_FLASH_FADE, muzzle_flash_fade);
    e.on_site(EXPLOSION_LIGHT_FADE, explosion_light_fade);
}

/// A form's flags (+8: 0x20 and 0x800, JIP's `test dword ptr [ecx+8], 0x820`), a reference's base form (+0x20)
/// and loaded data (+0x64, its root node +0x14); a projectile's light (+0x114, `spLight`).
const FORM_FLAGS: u32 = 8;
const NO_LIGHT_FLAGS: u32 = 0x820;
pub(crate) const REF_BASE: u32 = 0x20;
const REF_LOADED: u32 = 0x64;
const LOADED_ROOT: u32 = 0x14;
pub(crate) const PROJECTILE_LIGHT: u32 = 0x114;
/// `BGSProjectile`'s light (+0x70) and muzzle flash light (+0x74); `BGSExplosion`'s light (+0x80); a light
/// form's fade (+0xB4).
pub(crate) const PROJECTILE_FORM_LIGHT: u32 = 0x70;
const PROJECTILE_FORM_FLASH_LIGHT: u32 = 0x74;
const EXPLOSION_FORM_LIGHT: u32 = 0x80;
const LIGHT_FORM_FADE: u32 = 0xb4;
/// The game's `AttachLight` name: the word at `011C620C` holds the fixed string's holder (`0050de10` answers
/// it; JIP reads `**(const char***)0x11C620C` at its deferred start).
const ATTACH_LIGHT_NAME: u32 = 0x011c_620c;
/// `NiAVObject`'s parent (+0x18) and `NiNode::RemoveObject` (vtable +0xE8, the child); the shared `return
/// this` (`ADDR_ReturnThis`) a node's vtable slot +0xC holds.
const PARENT: u32 = 0x18;
const REMOVE_OBJECT_SLOT: u32 = 0xe8;
const RETURN_THIS: u32 = 0x0068_15c0;
/// A `MuzzleFlash`'s enabled byte (+0), light (+0x10, `spLight`) and projectile form (+0x14); an explosion's age
/// (+0x88), life (+0x8C) and light (+0xC4); a point light's dimmer (+0xC4).
const FLASH_ENABLED: u32 = 0;
const FLASH_PROJECTILE: u32 = 0x14;
const EXPLOSION_AGE: u32 = 0x88;
const EXPLOSION_LIFE: u32 = 0x8c;
const EXPLOSION_LIGHT: u32 = 0xc4;
const DIMMER: u32 = 0xc4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: AddProjectileLightHook (asm).
/// G95 at `0x9BDC92`, in place of the game's `009c3d90`: unless the projectile's flags have 0x20 or 0x800, with
/// its form's light (+0x70) and its 3D's root, one of JIP's point lights from that light ([`create_point_light`])
/// under the root's first block named `AttachLight` when that block is a node, else under the root, kept in the
/// projectile's light word (+0x114). Answers 0 (the call made).
fn add_projectile_light(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let projectile = w[0];
    if e.mem.u32(projectile + FORM_FLAGS) & NO_LIGHT_FLAGS != 0 {
        return Some(0);
    }
    let base = e.mem.u32(projectile + REF_BASE);
    let form = e.mem.u32(base + PROJECTILE_FORM_LIGHT);
    if form == 0 {
        return Some(0);
    }
    let loaded = e.mem.u32(projectile + REF_LOADED);
    if loaded == 0 {
        return Some(0);
    }
    let root = e.mem.u32(loaded + LOADED_ROOT);
    if root == 0 {
        return Some(0);
    }
    let node = match attach_light_name(e) {
        0 => root,
        name => match super::gh2::block_by_name_ptr(e, root, name, 0) {
            0 => root,
            block if e.mem.u32(e.mem.u32(block) + 0xc) == RETURN_THIS => block,
            _ => root,
        },
    };
    let light = create_point_light(e, form, node);
    e.mem.set_u32(projectile + PROJECTILE_LIGHT, light);
    Some(0)
}

/// The `AttachLight` fixed string (JIP's `g_attachLightString`), 0 before the game has made it (JIP reads it at
/// its deferred start, after the game made it at its own; with no holder the port looks for no block).
fn attach_light_name(e: &Engine) -> u32 {
    match e.mem.u32(ATTACH_LIGHT_NAME) {
        0 => 0,
        holder => e.mem.u32(holder),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RemoveProjectileLightHook (asm).
/// G95 at `0x9BC6D9`, in place of the game's `009c3e10`: the projectile's light word (+0x114) cleared and the
/// light taken off its parent (`RemoveObject`, vtable +0xE8), which lets JIP's list free it. Answers 0 (the call
/// made).
fn remove_projectile_light(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let projectile = w[0];
    let light = e.mem.u32(projectile + PROJECTILE_LIGHT);
    if light == 0 {
        return Some(0);
    }
    e.mem.set_u32(projectile + PROJECTILE_LIGHT, 0);
    let parent = e.mem.u32(light + PARENT);
    if parent != 0 {
        e.vcall(parent, REMOVE_OBJECT_SLOT, &args![light]);
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetMuzzleFlashFadeHook (asm).
/// G96 at `0x9BB906` (words: the flash, its light): the light's dimmer (+0xC4) the fade (+0xB4) of the flash's
/// projectile form's muzzle flash light (+0x74) while the flash is on (its byte +0, as JIP's `neg` and `and`),
/// 0 when off; the game's `0050dd20` is not called. Answers 0 (done). A flash without its projectile form or
/// that form without its light: with the missing-data guard row `None`, the game's 1.0 or 0 (nv-rs's guard);
/// without it read through null, as JIP.
fn muzzle_flash_fade(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let (flash, light) = (w[0], w[1]);
    let projectile = e.mem.u32(flash + FLASH_PROJECTILE);
    if projectile == 0 && missing_guard(e) {
        return None;
    }
    let form = e.mem.u32(projectile + PROJECTILE_FORM_FLASH_LIGHT);
    if form == 0 && missing_guard(e) {
        return None;
    }
    let fade = e.mem.u32(form + LIGHT_FORM_FADE);
    let on = u32::from(e.mem.u8(flash + FLASH_ENABLED)).wrapping_neg();
    e.mem.set_u32(light + DIMMER, fade & on);
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetExplosionLightFadeHook (asm).
/// G97 at `0x9AE68C`, in place of the game's `009b1320`: with a light (+0xC4), its dimmer `1 - (age + age) /
/// life` held at most 1 (`minss`: an unordered value gives 1) and at least 0 (`maxss`: unordered gives 0), times
/// the fade (+0xB4) of the explosion form's light (+0x80), in `float`s as JIP's SSE code; the game's `0050dd20`
/// is not called. Answers 0 (the call made). A form without a light: with the missing-data guard row `None`, the
/// game's fade (nv-rs's guard); without it read through null, as JIP.
fn explosion_light_fade(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let explosion = w[0];
    let light = e.mem.u32(explosion + EXPLOSION_LIGHT);
    if light == 0 {
        return Some(0);
    }
    let base = e.mem.u32(explosion + REF_BASE);
    let form = e.mem.u32(base + EXPLOSION_FORM_LIGHT);
    if form == 0 && missing_guard(e) {
        return None;
    }
    let age = e.mem.f32(explosion + EXPLOSION_AGE);
    let life = e.mem.f32(explosion + EXPLOSION_LIFE);
    let left = 1.0f32 - (age + age) / life;
    let left = if left < 1.0 { left } else { 1.0 };
    let left = if left > 0.0 { left } else { 0.0 };
    let dimmer = left * e.mem.f32(form + LIGHT_FORM_FADE);
    e.mem.set_u32(light + DIMMER, dimmer.to_bits());
    Some(0)
}

/// The missing-data guard row (`NV_NVSE_NULL_GUARD`, on in the fixes preset, off in vanilla), which holds nv-rs's
/// own guards where JIP reads through null.
fn missing_guard(e: &Engine) -> bool {
    nvse::commands::array::fix_on(e, nvse::commands::array::NULL_GUARD)
}

#[cfg(test)]
#[path = "gk_tests.rs"]
mod tests;
