//! Group GC, part 4: JIP's combat formula patches and the commands that set them, all JIP's own
//! (`InitGamePatches`, `internal/patches_game.h` (JIP LN NVSE): in once JIP answers
//! "installed", whatever the row `NV_JIP_GAME_PATCHES` says).
//!
//! - **G84** (`WriteRelJump(0x646D00, GetConditionDamagePenaltyHook)`): a weapon's condition
//!   factor on its damage, `1 - (0.75 - health) * s_condDmgPenalty` below three quarters of its
//!   health, clamped to 0..1, in single floats (the exe: the same at or below 0.75, with its
//!   constant 0.67f widened to a double, in x87, no clamp; at exactly 0.75 both give 1). `GetConditionDamagePenalty` / `SetConditionDamagePenalty` read and set
//!   the factor (0.67 by default, JIP's value of the exe's constant).
//! - **G85** (`WriteRelJump(0x646360, GetArmorConditionPenaltyHook)`):
//!   `CombatFormulas::CalcArmorRating`, an armour's DR or DT times
//!   `1 - (0.5 - health) * s_condDRDTPenalty` below half its health, clamped to 0..1 (the exe:
//!   `0x646D40`, factor 1 at or below 0.5, no clamp; at exactly 0.5 both give the rating). `SetArmorConditionPenalty` answers and sets the factor (1.0; O12 reads
//!   the same one).
//! - **G99** (`WriteRelJump(0x89D2FF, DoHitArmorDamageHook)`): the worn armour's damage from a
//!   hit (`Actor::HitMe`) is at most the armour's full health times
//!   `SetDamageToArmorMaxPercent`'s cap (1.0, kept in JIP's co-save), and goes through
//!   `PlayerCharacter::DamageEquipment` (`009508b0`, by address for every actor, as JIP calls it).
//! - **G126** (`WriteRelCall(0x8A04B2, GetControlFlagHook)`): `fn_008a03a0` leaves an actor's
//!   combat update with the player as its target undone while the player's movement controls are
//!   disabled (`+0x680` bit 0, `PlayerCharacter::GetControlFlag(1)`), unless
//!   `ToggleNoMovementCombat` is on (then the test answers 0 and the update runs).
//!
//! **Threads.** The wraps and seams run on the AI threads too; they read the run-wide lock-free
//! statics of [`super::damage::HitDamageState`] (made in [`install_jip`], before the threads) and
//! the cap's lock-free copy (`jip_fn_miscellaneous::armor_damage_cap`), never JIP's shared slot.

use super::damage;
use crate::commands::jip_fn_miscellaneous::{armor_damage_cap, publish_serialized_flags};
use crate::Jip;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

/// G84's wrap: a weapon's condition factor on its damage (`cdecl (float health)`, the factor in
/// `ST0`).
pub const CONDITION_DAMAGE: WrapSlot = WrapSlot::new(Owner::Jip, 0x0064_6d00);
/// G85's wrap: `CombatFormulas::CalcArmorRating` (`cdecl (UInt16 rating, float health)`).
pub const ARMOR_RATING: WrapSlot = WrapSlot::new(Owner::Jip, 0x0064_6360);
/// G99's seam: the worn armour's damage in `Actor::HitMe`.
pub const HIT_ARMOR_DAMAGE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0089_d2ff);
/// G126's seam: the player's control flag test in `fn_008a03a0`.
pub const CONTROL_FLAG: SiteSlot = SiteSlot::new(Owner::Jip, 0x008a_04b2);

/// `PlayerCharacter::DamageEquipment` (`thiscall (entry, float damage, bool)`), JIP's call.
const PLAYER_DAMAGE_EQUIPMENT: u32 = 0x0095_08b0;
/// The inventory entry's delete (`thiscall (flag)`), `0x4459E0`.
const ENTRY_DELETE: u32 = 0x0044_59e0;
/// An inventory entry's form (`+8`), an armour's health (`TESHealthForm`, `+0x6C`, an int), the
/// hit's armour damage (`+0x28`), the player's control flags (`+0x680`).
const ENTRY_FORM: u32 = 0x8;
const ARMOR_HEALTH: u32 = 0x6c;
const HIT_ARMOR_DAMAGE_FIELD: u32 = 0x28;
const CONTROL_FLAGS: u32 = 0x680;

/// The commands built with these patches (`functions_jip/jip_fn_miscellaneous.h`).
pub const GET_CONDITION_DAMAGE_PENALTY: u32 = 0x28c0;
pub const SET_CONDITION_DAMAGE_PENALTY: u32 = 0x28c1;
pub const TOGGLE_NO_MOVEMENT_COMBAT: u32 = 0x28ea;
pub const SET_ARMOR_CONDITION_PENALTY: u32 = 0x291d;
pub const SET_DAMAGE_TO_ARMOR_MAX_PERCENT: u32 = 0x2937;
#[rustfmt::skip]
pub static COMMANDS: crate::commands::Handlers = &[
    (GET_CONDITION_DAMAGE_PENALTY, get_condition_damage_penalty),
    (SET_CONDITION_DAMAGE_PENALTY, set_condition_damage_penalty),
    (TOGGLE_NO_MOVEMENT_COMBAT, toggle_no_movement_combat),
    (SET_ARMOR_CONDITION_PENALTY, set_armor_condition_penalty),
    (SET_DAMAGE_TO_ARMOR_MAX_PERCENT, set_damage_to_armor_max_percent),
];

pub(super) fn install_jip(e: &mut Engine) {
    // The run-wide statics, made before the threads start (`ext_free`'s rule).
    e.ext_free::<damage::HitDamageState>();
    e.wrap(CONDITION_DAMAGE, condition_damage);
    e.wrap(ARMOR_RATING, armor_rating);
    e.on_site(HIT_ARMOR_DAMAGE, hit_armor_damage);
    e.on_site(CONTROL_FLAG, control_flag);
}

/// SSE `minss a, b`: `a` when it is less than `b`, else `b` (also when either is NaN).
fn minss(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}

/// SSE `maxss a, b`: `a` when it is greater than `b`, else `b` (also when either is NaN).
fn maxss(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

/// JIP's clamped condition factor: `1 - (threshold - health) * penalty`, at most 1 and at least
/// 0 (`minss` with 1, then `maxss` with 0), in single floats.
fn penalty_factor(threshold: f32, health: f32, penalty: f32) -> f32 {
    let cut = (threshold - health) * penalty;
    maxss(minss(1.0 - cut, 1.0), 0.0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h:
// GetConditionDamagePenaltyHook.
/// G84: 1.0 unless the health is below 0.75 (`comiss`/`jnb`: a NaN health goes on to the
/// formula), else [`penalty_factor`] with `s_condDmgPenalty`. Never hands the call on.
fn condition_damage(e: &mut Engine, w: &[u32]) -> Ret {
    let health = f32::from_bits(w[0]);
    let factor = if health >= 0.75 {
        1.0
    } else {
        penalty_factor(0.75, health, damage::damage_penalty(e))
    };
    f32::into_ret(factor)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h:
// GetArmorConditionPenaltyHook.
/// G85: the rating (`movzx` of the first word's low 16 bits, `cvtsi2ss`) as it is unless the
/// health is below 0.5, else times [`penalty_factor`] with `s_condDRDTPenalty`.
fn armor_rating(e: &mut Engine, w: &[u32]) -> Ret {
    let rating = f32::from(w[0] as u16);
    let health = f32::from_bits(w[1]);
    let value = if health >= 0.5 {
        rating
    } else {
        rating * penalty_factor(0.5, health, damage::condition_penalty(e))
    };
    f32::into_ret(value)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DoHitArmorDamageHook.
/// G99 (`a`: the actor hit, the worn armour's inventory entry, the hit): the damage is the
/// armour's health (`cvtsi2ss` of its int) times the cap, or the hit's armour damage when that is
/// not more (`minss`); `PlayerCharacter::DamageEquipment` (`009508b0`) on the actor with the
/// entry, the damage and 0, then the entry deleted (flag 1). Answers 0 (the exe's span skipped).
fn hit_armor_damage(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (actor, entry, hit) = (a[0], a[1], a[2]);
    let form = e.mem.u32(entry.wrapping_add(ENTRY_FORM));
    let most = e.mem.u32(form.wrapping_add(ARMOR_HEALTH)) as i32 as f32 * armor_damage_cap(e);
    let damage = minss(most, e.mem.f32(hit.wrapping_add(HIT_ARMOR_DAMAGE_FIELD)));
    e.call(
        PLAYER_DAMAGE_EQUIPMENT,
        &[actor, entry, damage.to_bits(), 0],
    );
    e.call(ENTRY_DELETE, &[entry, 1]);
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetControlFlagHook.
/// G126 (`a`: the player): 0 while `s_noMovementCombat` is on, else the player's control flags'
/// bit 0 (JIP tests bit 0 whatever flag the call pushed; the exe's call pushes 1).
fn control_flag(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if damage::no_movement_combat(e) {
        return Some(0);
    }
    Some(u32::from(e.mem.u8(a[0].wrapping_add(CONTROL_FLAGS)) & 1))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetConditionDamagePenalty_Execute.
/// `GetConditionDamagePenalty`: G84's factor.
fn get_condition_damage_penalty(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let value = damage::damage_penalty(e);
    a.set_result(e, f64::from(value));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetConditionDamagePenalty_Execute.
/// `SetConditionDamagePenalty value`: G84's factor becomes the value (any value; nothing when
/// the argument is not read).
fn set_condition_damage_penalty(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let value = read.float(0, 0.0);
        damage::set_damage_penalty(e, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_ToggleNoMovementCombat_Execute.
/// `ToggleNoMovementCombat [on]`: answers 1 while it is on; with a read argument it becomes on
/// for any value but 0.
fn toggle_no_movement_combat(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if damage::no_movement_combat(e) {
        a.set_result(e, 1.0);
    }
    if crate::commands::jip_fn_utility::num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        if read.ok {
            let toggle = read.int(0, 0);
            damage::set_no_movement_combat(e, toggle != 0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetArmorConditionPenalty_Execute.
/// `SetArmorConditionPenalty [value]`: answers the factor before; with an argument the factor
/// becomes what was read (JIP reads straight into it: an argument not read leaves it).
fn set_armor_condition_penalty(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let before = damage::condition_penalty(e);
    a.set_result(e, f64::from(before));
    if crate::commands::jip_fn_utility::num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        damage::set_condition_penalty(e, read.float(0, before));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetDamageToArmorMaxPercent_Execute.
/// `SetDamageToArmorMaxPercent [value]` (`DTAMP`): answers the cap before; with an argument from
/// 0 to 1 the cap becomes it (kept in the co-save; the lock-free copy G99 reads is written at
/// once). An argument not read sets nothing (JIP's local would be undefined).
fn set_damage_to_armor_max_percent(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let before = f32::from_bits(e.ext::<Jip>().saved.serialized.dmg_armor_max_percent);
    a.set_result(e, f64::from(before));
    if crate::commands::jip_fn_utility::num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        let value = read.float(0, f32::NAN);
        if (0.0..=1.0).contains(&value) {
            e.ext::<Jip>().saved.serialized.dmg_armor_max_percent = value.to_bits();
            publish_serialized_flags(e);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "gc_formulas_tests.rs"]
mod tests;
