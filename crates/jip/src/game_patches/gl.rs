//! Group GL's game patches (step 3.9: lights and magic effects): its handlers, wraps and slot routines. Only group
//! GL's PR edits this file (and its own tests file beside it).
//!
//! **GL-1, the magic effect fixes of `InitGamePatches`** (`internal/patches_game.h` (JIP LN NVSE), in
//! with the game-patch row `NV_JIP_GAME_PATCHES`):
//!
//! - G66 (`SAFE_WRITE_BUF(0x80ECA8, ..)`): `LightEffect::AttachLight` attaches its light only when the
//!   actor has a node to attach it to ([`attach_light`]); the original calls through a null node.
//! - G86 (`WriteRelCall(0x406720, GetEffectHiddenHook)`): `EffectItemList::BuildMenuString` leaves
//!   out an effect with no setting or a hidden one (setting flag 8), as well as the usage monitor
//!   ([`effect_hidden`]).
//! - G67 (`WriteRelJump(0x4069D4, EffectSkillModifiers1Hook)`, `WriteRelJump(0x816519,
//!   EffectSkillModifiers2Hook)`): the Medicine and Survival multipliers (with the "recovered
//!   health" perk entry on Health) are applied one or the other, Medicine first, in the effect
//!   text ([`menu_skill_modifiers`]) and when the effect is cast ([`cast_skill_modifiers`]); a
//!   detrimental effect (text: a negative magnitude; cast: setting flag 4) is left as it is; the
//!   cast applies the perk entry only for the player.
//! - G107 (`WriteRelCall(0x7E0C1F, EffectItemGetModifiedDuration)`, `WriteRelJump(0x8236FD,
//!   ModPositiveChemDurationHook)`): the `ModifyPositiveChemDuration` perk entry lengthens a chem's
//!   (alchemy type 10 or 11) non-hostile effect in the effect text ([`modified_duration`]) and only
//!   a chem's when the effect is added ([`chem_duration`]; the original: every non-hostile alchemy
//!   effect). JIP reads the list whose text is built from `BuildMenuString`'s frame
//!   (`[[EBP]-0xD4]` inside `StatusDataList::Register`, whose only caller it is); the port passes
//!   it with a seam at that call ([`menu_list`]).
//!
//! None of these changes what the renderer receives for a light: G66 only skips the attach call
//! when there is no node, where the original would crash.
//!
//! Threads: the effect text (`BuildMenuString`, `Register`) runs on the main thread (menus, HUD);
//! the list word is a lock-free `ext_free` word set and taken there. `FindTargets` and
//! `CheckAddEffect` run wherever an actor casts or is hit (main and AI threads); their handlers keep
//! no state.

use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use engine_core::x87::{x87_m32, x87_mul, x87_store32};
use std::sync::atomic::{AtomicU32, Ordering};

/// G86: `BuildMenuString`'s usage-monitor test (`CALL 0x404730`; words: the effect item; answered:
/// 1 to leave it out, 0 to list it).
pub const EFFECT_HIDDEN: SiteSlot = SiteSlot::new(Owner::Jip, 0x0040_6720);
/// G107: `BuildMenuString`'s call of `StatusDataList::Register` (`0x40686D`; words: the list whose
/// text is built). Port-side: JIP reads this frame from inside `Register`.
pub const MENU_LIST: SiteSlot = SiteSlot::new(Owner::Jip, 0x0040_686d);
/// G67: `BuildMenuString`'s Medicine and Survival scaling (words: the entry's own magnitude, the
/// magnitude so far, the actor value, the Medicine and Survival flags; answered: the magnitude).
pub const MENU_SKILL_MODIFIERS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0040_69d4);
/// G107: `StatusDataList::Register`'s `EffectItem::GetDuration` (words: the effect item; answered:
/// the duration).
pub const MODIFIED_DURATION: SiteSlot = SiteSlot::new(Owner::Jip, 0x007e_0c1f);
/// G66: `LightEffect::AttachLight`'s attach through the node's slot +0xDC (words: the node;
/// answered: skip the call).
pub const ATTACH_LIGHT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0080_eca8);
/// G67: `MagicCaster::FindTargets`' alchemy Medicine and Survival blocks (words: the alchemy item,
/// the active effect, the actor, the effectiveness's bits; answered: done, the blocks skipped).
pub const CAST_SKILL_MODIFIERS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0081_6519);
/// G107: `MagicTarget::CheckAddEffect`'s hostile test before the positive chem duration entry
/// (words: the alchemy item, the active effect; answered: 1 to skip the entry, 0 to apply it).
pub const CHEM_DURATION: SiteSlot = SiteSlot::new(Owner::Jip, 0x0082_36fd);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GL",
    fix_sites: &[
        EFFECT_HIDDEN.addr(),
        MENU_LIST.addr(),
        MENU_SKILL_MODIFIERS.addr(),
        MODIFIED_DURATION.addr(),
        ATTACH_LIGHT.addr(),
        CAST_SKILL_MODIFIERS.addr(),
        CHEM_DURATION.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

fn install_fixes(e: &mut Engine) {
    e.on_site(EFFECT_HIDDEN, effect_hidden);
    e.on_site(MENU_LIST, menu_list);
    e.on_site(MENU_SKILL_MODIFIERS, menu_skill_modifiers);
    e.on_site(MODIFIED_DURATION, modified_duration);
    e.on_site(ATTACH_LIGHT, attach_light);
    e.on_site(CAST_SKILL_MODIFIERS, cast_skill_modifiers);
    e.on_site(CHEM_DURATION, chem_duration);
}

/// An effect item's magnitude (+0), duration (+8), actor value (+0x10) and setting (+0x14); a
/// setting's flags byte (+0x58: 1 hostile, 4 detrimental, 8 hidden, 0x80 no duration); an active
/// effect's effect item (+0xC); an alchemy item's flags byte (+0xBC: 2 food, 4 medicine) and type
/// byte (+0xA0, read through its `EffectItemList` at +0x64 in the menu); an actor's form id (+0xC).
const ITEM_MAGNITUDE: u32 = 0;
const ITEM_DURATION: u32 = 8;
const ITEM_ACTOR_VALUE: u32 = 0x10;
const ITEM_SETTING: u32 = 0x14;
const SETTING_FLAGS: u32 = 0x58;
const ACTIVE_ITEM: u32 = 0xc;
const ALCHEMY_FLAGS: u32 = 0xbc;
const ALCHEMY_TYPE: u32 = 0xa0;
const LIST_ALCHEMY_TYPE: u32 = 0x64;
const FORM_ID: u32 = 0xc;
/// `EffectSettingCollection::GetUsageMonitorEffectSetting`'s global (`00408b50`).
const USAGE_MONITOR: u32 = 0x011c_33f4;
/// An `AlchemyItem`'s `EffectItemList` vtable (JIP's test in `EffectItemGetModifiedDuration`).
const ALCHEMY_LIST_VTABLE: u32 = 0x0101_1900;
/// `g_thePlayer`; the player's Medicine and Survival multipliers (`thiscall` on an actor, `ST0`);
/// `ApplyPerkModifiers` (`cdecl`: the entry point, the actor, the value's address); an active
/// effect's `AdjustEffectiveness` (`thiscall`, `RET 4`, a float).
const PLAYER: u32 = 0x011d_ea3c;
const MEDICINE_MULT: u32 = 0x008b_ebb0;
const SURVIVAL_MULT: u32 = 0x008b_ec80;
const APPLY_PERK_MODIFIERS: u32 = 0x005e_58f0;
const ADJUST_EFFECTIVENESS: u32 = 0x0080_4bc0;
/// `kAVCode_Health`, `kAVCode_RadiationRads`, the player's form id, and the perk entry points
/// `kPerkEntry_ModifyRecoveredHealth` and `kPerkEntry_ModifyPositiveChemDuration`.
const AV_HEALTH: u32 = 0x10;
const AV_RADIATION_RADS: u32 = 0x36;
const PLAYER_FORM_ID: u32 = 0x14;
const ENTRY_RECOVERED_HEALTH: u32 = 0xc;
const ENTRY_POSITIVE_CHEM_DURATION: u32 = 0x19;

/// SSE `cvttss2si`: toward zero; out of range or NaN gives `0x80000000`.
fn cvttss2si(value: f32) -> i32 {
    if value.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}

/// `ApplyPerkModifiers(entry, actor, &value)` on a float: the value after the actor's perks.
fn apply_perk(e: &mut Engine, entry: u32, actor: u32, value: u32) -> u32 {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(APPLY_PERK_MODIFIERS, &args![entry, actor, slot]);
        e.mem.u32(slot.addr())
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: the 26 bytes at 0x80ECA8.
/// G66 (words: the node): no node, the attach is skipped (1); a node, the exe's call (`None`, the
/// same call JIP makes).
fn attach_light(_: &mut Engine, words: &[u32]) -> Option<u32> {
    (*words.first()? == 0).then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetEffectHiddenHook.
/// G86 (words: the effect item): 1 (left out) for no setting, the usage monitor or a setting with
/// flag 8; else 0.
fn effect_hidden(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let item = *words.first()?;
    let setting = e.mem.u32(item + ITEM_SETTING);
    if setting == 0 || setting == e.mem.u32(USAGE_MONITOR) {
        return Some(1);
    }
    Some(u32::from(e.mem.u8(setting + SETTING_FLAGS) & 8 != 0))
}

/// G107's list word: the `EffectItemList` whose text `BuildMenuString` builds, set at its call of
/// `Register` and taken by `Register`'s duration.
#[derive(Default)]
pub struct MenuList(AtomicU32);

/// G107 (words: the list): kept for `Register`'s duration; answers nothing (the call follows).
fn menu_list(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let list = *words.first()?;
    e.ext_free::<MenuList>().0.store(list, Ordering::Relaxed);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: EffectItemGetModifiedDuration.
/// G107 (words: the effect item): 0 with the setting's flag 0x80; else the duration, for a
/// non-hostile effect of a chem (the list is an alchemy item's, type 10 or 11) after the player's
/// `ModifyPositiveChemDuration` perks (from a float, toward zero).
fn modified_duration(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let item = *words.first()?;
    let list = e.ext_free::<MenuList>().0.swap(0, Ordering::Relaxed);
    let setting = e.mem.u32(item + ITEM_SETTING);
    let flags = e.mem.u8(setting + SETTING_FLAGS);
    if flags & 0x80 != 0 {
        return Some(0);
    }
    let duration = e.mem.u32(item + ITEM_DURATION);
    if flags & 1 != 0 || list == 0 || e.mem.u32(list) != ALCHEMY_LIST_VTABLE {
        return Some(duration);
    }
    if e.mem.u8(list + LIST_ALCHEMY_TYPE).wrapping_sub(0xa) > 1 {
        return Some(duration);
    }
    let player = e.mem.u32(PLAYER);
    let value = (duration as i32 as f32).to_bits();
    let value = apply_perk(e, ENTRY_POSITIVE_CHEM_DURATION, player, value);
    Some(cvttss2si(f32::from_bits(value)) as u32)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ModPositiveChemDurationHook.
/// G107 (words: the alchemy item, the active effect): skip (1) unless the item's type is 10 or 11;
/// then the effect setting's hostile flag (skip a hostile one).
fn chem_duration(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (&alchemy, &active) = (words.first()?, words.get(1)?);
    if e.mem.u8(alchemy + ALCHEMY_TYPE).wrapping_sub(0xa) > 1 {
        return Some(1);
    }
    let item = e.mem.u32(active + ACTIVE_ITEM);
    let setting = e.mem.u32(item + ITEM_SETTING);
    Some(u32::from(e.mem.u8(setting + SETTING_FLAGS) & 1))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: EffectSkillModifiers1Hook.
/// G67 in the effect text (words: the entry's magnitude, the magnitude so far, the actor value, the
/// Medicine flag, the Survival flag): with neither flag, or a negative entry magnitude, the
/// magnitude unchanged; else the magnitude times the player's Medicine multiplier (or, without
/// Medicine, the Survival one, not for radiation), the "recovered health" perks on Health, toward
/// zero.
fn menu_skill_modifiers(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let [entry, magnitude, actor_value, medicine, survival] = *words else {
        return None;
    };
    let mode = (survival as u8)
        .wrapping_shl(1)
        .wrapping_add(medicine as u8);
    if mode == 0 || (entry as i32) < 0 {
        return Some(magnitude);
    }
    let player = e.mem.u32(PLAYER);
    let factor = if mode & 1 != 0 {
        e.call(MEDICINE_MULT, &args![player]).f64()
    } else if mode & 2 != 0 && actor_value != AV_RADIATION_RADS {
        e.call(SURVIVAL_MULT, &args![player]).f64()
    } else {
        return Some(magnitude);
    };
    let mut value = x87_store32(x87_mul(factor, f64::from(magnitude as i32)));
    if actor_value == AV_HEALTH {
        value = apply_perk(e, ENTRY_RECOVERED_HEALTH, player, value);
    }
    Some(cvttss2si(f32::from_bits(value)) as u32)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: EffectSkillModifiers2Hook.
/// G67 when cast (words: the alchemy item, the active effect, the actor, the effectiveness): for
/// a Medicine or Survival item whose effect is not detrimental (setting flag 4), the effect's
/// effectiveness becomes the effectiveness times the actor's Medicine multiplier (or, without
/// Medicine, the Survival one, not for radiation), the "recovered health" perks on Health for the
/// player; always answered (the exe's two blocks are skipped).
fn cast_skill_modifiers(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let [alchemy, active, actor, effectiveness] = *words else {
        return None;
    };
    if alchemy == 0 {
        return Some(1);
    }
    let flags = e.mem.u8(alchemy + ALCHEMY_FLAGS);
    if flags & 6 == 0 {
        return Some(1);
    }
    let item = e.mem.u32(active + ACTIVE_ITEM);
    let setting = e.mem.u32(item + ITEM_SETTING);
    if e.mem.u8(setting + SETTING_FLAGS) & 4 != 0 {
        return Some(1);
    }
    let actor_value = e.mem.u32(item + ITEM_ACTOR_VALUE);
    let factor = if flags & 4 != 0 {
        e.call(MEDICINE_MULT, &args![actor]).f64()
    } else if actor_value != AV_RADIATION_RADS {
        e.call(SURVIVAL_MULT, &args![actor]).f64()
    } else {
        return Some(1);
    };
    let mut value = x87_store32(x87_mul(factor, x87_m32(effectiveness)));
    if actor_value == AV_HEALTH && e.mem.u32(actor + FORM_ID) == PLAYER_FORM_ID {
        value = apply_perk(e, ENTRY_RECOVERED_HEALTH, actor, value);
    }
    e.call(ADJUST_EFFECTIVENESS, &args![active, value]);
    Some(1)
}

#[cfg(test)]
#[path = "gl_tests.rs"]
mod tests;
