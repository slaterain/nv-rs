//! `functions_jip/jip_fn_projectile.h` (JIP LN NVSE): a projectile's numbers, flags, explosion,
//! muzzle flash and light, and a fired projectile's source, weapon, life, distance, damage, speed,
//! detonation timer, mine state and what it hit. 25 of its 26 commands:
//! `SetOnProjectileImpactEventHandler` registers an event handler and needs JIP's projectile impact
//! hook (the core track's).
//!
//! The base form (`forms::BGSProjectile`, type 0x33) is checked in its loader; the fired
//! projectile (`forms::Projectile`) in its constructor. JIP tells a fired projectile by its vtable
//! answering true (`008D0360`) at +0x224 (`IS_PROJECTILE`); a grenade is form type 0x3E. The
//! commands that run on a reference do nothing without one.

use super::forms::{form_type_b7p, BGSProjectile, ImpactData, ListNode, Projectile};
use super::{
    assign_string, buffer_arg, cvttss2si, list_items, model_path, set_form_result, set_model_path,
    type_of, Handlers,
};
use crate::game::TESObjectREFR;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2615, get_projectile_trait_numeric),
    (0x2616, set_projectile_trait_numeric),
    (0x2617, get_projectile_flag),
    (0x2618, set_projectile_flag),
    (0x2619, get_projectile_explosion),
    (0x261a, set_projectile_explosion),
    (0x26dc, get_projectile_ref_source),
    (0x26dd, set_projectile_ref_source),
    (0x26de, get_projectile_ref_weapon),
    (0x26df, set_projectile_ref_weapon),
    (0x26e0, get_projectile_ref_life_time),
    (0x26e1, get_projectile_ref_distance_traveled),
    (0x26e2, get_projectile_ref_damage),
    (0x26e3, set_projectile_ref_damage),
    (0x26e4, get_projectile_ref_speed_mult),
    (0x26e5, set_projectile_ref_speed_mult),
    (0x2772, get_detonation_timer),
    (0x2773, set_detonation_timer),
    (0x2774, get_mine_armed),
    (0x2831, get_projectile_muzzle_flash),
    (0x2832, set_projectile_muzzle_flash),
    (0x28af, get_projectile_ref_impact_ref),
    (0x28be, get_projectile_ref_impact_material),
    (0x2932, get_projectile_light),
    (0x2933, set_projectile_light),
];

type B = BGSProjectile;
type P = Projectile;

const PLAYER: u32 = 0x011d_ea3c;
const RETURN_TRUE: u32 = 0x008d_0360;
/// `char path[0x80]` of `SetProjectileMuzzleFlash`.
const PATH_BUFFER: usize = 0x80;
/// `kMaterialConvert` (JIP LN NVSE, internal/jip_core.cpp): the impact's material (0-31) as the
/// script's material number (stone 0, dirt 1, grass 2, glass 3, metal 4, wood 5, organic 6,
/// cloth 7, water 8, hollow metal 9).
const MATERIAL_CONVERT: [u8; 32] = [
    0, 7, 1, 3, 2, 4, 6, 6, 8, 5, 0, 4, 5, 4, 4, 4, 9, 9, 1, 0, 4, 4, 9, 9, 3, 9, 4, 4, 9, 9, 5, 5,
];

/// `form` when it is a projectile (`IS_ID(projectile, BGSProjectile)`).
fn projectile(e: &Engine, form: u32) -> Option<Ptr<B>> {
    (form != 0 && type_of(e, form) == form_type_b7p::BGS_PROJECTILE).then(|| Ptr::new(form))
}

/// Argument 0 when the read succeeded and it is a projectile.
fn projectile_arg(e: &Engine, read: &Read) -> Option<Ptr<B>> {
    if read.ok {
        projectile(e, read.form(0))
    } else {
        None
    }
}

/// `IS_PROJECTILE(ref)`: the reference's vtable answers true at +0x224.
fn fired(e: &Engine, r: u32) -> Option<Ptr<P>> {
    if r == 0 {
        return None;
    }
    let vtbl = e.mem.u32(r);
    (e.mem.u32(vtbl + 0x224) == RETURN_TRUE).then(|| Ptr::new(r))
}

/// The byte offset of float trait `t` (1-13) in the projectile: `((float*)projectile)[n + t]`
/// with n 24 (1-3), 26 (4-6), 28 (7-9), 31 (10-13).
fn trait_offset(t: u32) -> Option<u32> {
    let n = match t {
        1..=3 => 24,
        4..=6 => 26,
        7..=9 => 28,
        10..=13 => 31,
        _ => return None,
    };
    Some(4 * (n + t))
}

// ---- the base form ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_GetProjectileTraitNumeric_Execute.
/// 0 the type, 1-13 the floats, 14 the sound level; another trait, nothing.
fn get_projectile_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = projectile_arg(e, &read) {
        let t = read.int(1, 0);
        let v = match t {
            0 => Some(f64::from(e.get(p, B::projType))),
            14 => Some(f64::from(e.get(p, B::soundLevel))),
            _ => trait_offset(t).map(|off| f64::from(e.mem.f32(p.addr() + off))),
        };
        if let Some(v) = v {
            a.set_result(e, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetProjectileTraitNumeric_Execute.
/// The value made a `UInt32` (`(int)fVal`, `cvttss2si`): 0 the type when it is 0 or a power of
/// two (its low 16 bits); 14 the sound level's byte for 1 or 2; 1-13 the float itself.
fn set_projectile_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = projectile_arg(e, &read) {
        let value = read.float(2, 0.0);
        let int = cvttss2si(value) as u32;
        match read.int(1, 0) {
            0 => {
                if int & int.wrapping_sub(1) == 0 {
                    e.set(p, B::projType, int as u16);
                }
            }
            14 => {
                if int != 0 && int < 3 {
                    e.set(p, B::soundLevel, int as u8);
                }
            }
            t => {
                if let Some(off) = trait_offset(t) {
                    e.mem.set_f32(p.addr() + off, value);
                }
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_GetProjectileFlag_Execute.
/// Flag 0-15 of the 16-bit flags.
fn get_projectile_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(p) = projectile_arg(e, &read).filter(|_| flag <= 15) {
        if e.get(p, B::projFlags) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetProjectileFlag_Execute.
/// Flag 0-15 set or cleared (`BGSProjectile::SetFlag`).
fn set_projectile_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(p) = projectile_arg(e, &read).filter(|_| flag <= 15) {
        let flags = e.get(p, B::projFlags);
        let bit = 1u16 << flag;
        let flags = if read.int(2, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(p, B::projFlags, flags);
    }
    handled(true)
}

/// A form field of the projectile as the result, when set.
fn get_form(e: &mut Engine, w: &[u32], field: Field<B, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = projectile_arg(e, &read) {
        let form = e.get(p, field);
        if form != 0 {
            set_form_result(e, &a, form);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_GetProjectileExplosion_Execute and Cmd_GetProjectileLight_Execute.
fn get_projectile_explosion(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, B::explosion)
}
fn get_projectile_light(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, B::lightProjectile)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetProjectileExplosion_Execute.
/// The explosion when the optional form is one (0x51), else none.
fn set_projectile_explosion(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = projectile_arg(e, &read) {
        let x = read.form(1);
        let x = if x != 0 && type_of(e, x) == form_type_b7p::BGS_EXPLOSION {
            x
        } else {
            0
        };
        e.set(p, B::explosion, x);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetProjectileLight_Execute.
/// The light (0x1E), or none when the optional form is left out; another form, nothing.
fn set_projectile_light(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let light = read.form(1);
    if let Some(p) = projectile_arg(e, &read)
        .filter(|_| light == 0 || type_of(e, light) == form_type_b7p::TES_OBJECT_LIGH)
    {
        e.set(p, B::lightProjectile, light);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_GetProjectileMuzzleFlash_Execute.
/// The muzzle flash model's path (`GetModelPath`).
fn get_projectile_muzzle_flash(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = match projectile_arg(e, &read) {
        Some(p) => model_path(e, p.at(B::muzzleFlash).addr()),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetProjectileMuzzleFlash_Execute.
/// The path (JIP's 0x80-byte buffer) set through the model's own `SetModelPath`.
fn set_projectile_muzzle_flash(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetProjectileMuzzleFlash");
    if let Some(p) = projectile_arg(e, &read) {
        set_model_path(e, p.at(B::muzzleFlash).addr(), &path);
    }
    handled(true)
}

// ---- a fired projectile ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Projectile::GetData.
/// What `GetData(kind)` answers for the calling reference when it is a fired projectile: 0 the
/// source and 1 the weapon (as forms, when set), 2 life, 3 distance, 4 damage, 5 speed; 6 the
/// first impact with a reference, 7 the first impact's material through `kMaterialConvert`
/// (-1 when there is none or it is above 31), both only once it has hit.
fn get_data(e: &mut Engine, w: &[u32], kind: u32) -> Ret {
    let a = CommandArgs::new(w);
    let Some(p) = fired(e, a.this_obj) else {
        return handled(true);
    };
    let float = |e: &Engine, f: Field<P, f32>| f64::from(e.get(p, f));
    match kind {
        0 | 1 => {
            let field = if kind == 0 {
                P::sourceRef
            } else {
                P::sourceWeap
            };
            let form = e.get(p, field);
            if form != 0 {
                set_form_result(e, &a, form);
            }
        }
        2 => a.set_result(e, float(e, P::lifeTime)),
        3 => a.set_result(e, float(e, P::distTravelled)),
        4 => a.set_result(e, float(e, P::hitDamage)),
        5 => a.set_result(e, float(e, P::speedMult)),
        6 => {
            if e.get(p, P::hasImpacted) != 0 {
                let hit = list_items(e, p.at(P::impactDataList))
                    .into_iter()
                    .filter(|&d| d != 0)
                    .map(|d| e.get(Ptr::<ImpactData>::new(d), ImpactData::refr))
                    .find(|&r| r != 0);
                if let Some(r) = hit {
                    set_form_result(e, &a, r);
                }
            }
        }
        _ => {
            let mut v = -1.0;
            if e.get(p, P::hasImpacted) != 0 {
                let first = e.get(p.at(P::impactDataList), ListNode::data);
                if first != 0 {
                    let m = e.get(Ptr::<ImpactData>::new(first), ImpactData::materialType);
                    if m <= 31 {
                        v = f64::from(MATERIAL_CONVERT[m as usize]);
                    }
                }
            }
            a.set_result(e, v);
        }
    }
    handled(true)
}

fn get_projectile_ref_source(e: &mut Engine, w: &[u32]) -> Ret {
    get_data(e, w, 0)
}
fn get_projectile_ref_weapon(e: &mut Engine, w: &[u32]) -> Ret {
    get_data(e, w, 1)
}
fn get_projectile_ref_life_time(e: &mut Engine, w: &[u32]) -> Ret {
    get_data(e, w, 2)
}
fn get_projectile_ref_distance_traveled(e: &mut Engine, w: &[u32]) -> Ret {
    get_data(e, w, 3)
}
fn get_projectile_ref_damage(e: &mut Engine, w: &[u32]) -> Ret {
    get_data(e, w, 4)
}
fn get_projectile_ref_speed_mult(e: &mut Engine, w: &[u32]) -> Ret {
    get_data(e, w, 5)
}
fn get_projectile_ref_impact_ref(e: &mut Engine, w: &[u32]) -> Ret {
    get_data(e, w, 6)
}
fn get_projectile_ref_impact_material(e: &mut Engine, w: &[u32]) -> Ret {
    get_data(e, w, 7)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetProjectileRefSource_Execute.
/// The source reference (none when it is left out).
fn set_projectile_ref_source(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(p) = fired(e, a.this_obj) {
            e.set(p, P::sourceRef, read.form(0));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetProjectileRefWeapon_Execute.
/// The weapon, when the form is one (0x28).
fn set_projectile_ref_weapon(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let weapon = read.form(0);
    if read.ok && weapon != 0 && type_of(e, weapon) == form_type_b7p::TES_OBJECT_WEAP {
        if let Some(p) = fired(e, a.this_obj) {
            e.set(p, P::sourceWeap, weapon);
        }
    }
    handled(true)
}

/// A float of the fired projectile set to argument 0.
fn set_float(e: &mut Engine, w: &[u32], field: Field<P, f32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(p) = fired(e, a.this_obj) {
            e.set(p, field, read.float(0, 0.0));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetProjectileRefDamage_Execute and Cmd_SetProjectileRefSpeedMult_Execute.
fn set_projectile_ref_damage(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, P::hitDamage)
}
fn set_projectile_ref_speed_mult(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, P::speedMult)
}

/// The base projectile's flags of reference `r`.
fn base_flags(e: &Engine, r: u32) -> u16 {
    let base = e.get(
        Ptr::<TESObjectREFR>::new(r),
        TESObjectREFR::pObjectReference,
    );
    e.get(Ptr::<B>::new(base), B::projFlags)
}

/// The calling reference when it is a grenade (form type 0x3E) whose base projectile has the
/// alt-trigger flag (4): `IS_ID(thisObj, GrenadeProjectile)` and the base's flags.
fn timed_grenade(e: &Engine, r: u32) -> Option<Ptr<P>> {
    if r == 0 || type_of(e, r) != form_type_b7p::GRENADE_PROJECTILE {
        return None;
    }
    (base_flags(e, r) & 4 != 0).then(|| Ptr::new(r))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_GetDetonationTimer_Execute.
/// The timer of a timed grenade; -1 otherwise.
fn get_detonation_timer(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let v = match timed_grenade(e, a.this_obj) {
        Some(p) => f64::from(e.get(p, P::detonationTime)),
        None => -1.0,
    };
    a.set_result(e, v);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_SetDetonationTimer_Execute.
fn set_detonation_timer(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(p) = timed_grenade(e, a.this_obj) {
            e.set(p, P::detonationTime, read.float(0, 0.0));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_projectile.h:
// Cmd_GetMineArmed_Execute.
/// 1 for a grenade without its own flag 0x200, not fired by the player, whose base projectile's
/// flags masked with 0x426 are 0x26 (explosion, alt trigger, can be disabled; not detonating).
fn get_mine_armed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let r = a.this_obj;
    if r == 0 || type_of(e, r) != form_type_b7p::GRENADE_PROJECTILE {
        return handled(true);
    }
    let p = Ptr::<P>::new(r);
    let player = e.mem.u32(PLAYER);
    if e.get(p, P::projFlags) & 0x200 == 0
        && e.get(p, P::sourceRef) != player
        && base_flags(e, r) & 0x426 == 0x26
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_projectile_tests.rs"]
mod tests;
