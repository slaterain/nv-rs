//! `functions_jip/jip_fn_combat_style.h` (JIP LN NVSE): a combat style's 64 numbers and its
//! flags. All 4 of its commands. The style is read as `kParamType_CombatStyle`, whose type check
//! xNVSE's reader makes (JIP checks nothing more).
//!
//! JIP addresses the numbers as arrays over the form (`((float*)cStyle)[6 + traitID]`,
//! `((UInt8*)cStyle)[0x49 + traitID]`, ...). Here each run of traits is a field at the run's
//! first trait and an index from it, so trait `t` lands where JIP's expression does:
//!
//! | traits | JIP | here (byte offset) |
//! | --- | --- | --- |
//! | 0-8 | `float[6 + t]` | `coverSearchRadius` +0x18, step 4 |
//! | 9 | `weaponRestrictions` | +0x40 |
//! | 10-14 | `float[7 + t]` | `rangedWeapRangeMultMax` +0x44 |
//! | 15-16 | `UInt8[0x49 + t]` | `dodgeChance` +0x58, step 1 |
//! | 17-24 | `float[6 + t]` | `dodgeLRTimerMin` +0x5C |
//! | 25-26 | `UInt8[0x63 + t]` | `blockChance` +0x7C |
//! | 27-29 | `float[5 + t]` | `staggerBonusToAttack` +0x80 |
//! | 30 | `powerAttackChance` | +0x8C |
//! | 31-32 | `float[5 + t]` | `staggerBonusToPower` +0x90 |
//! | 33-37 | `UInt8[0x77 + t]` | `powerAttackN` +0x98 |
//! | 38-39 | `float[2 + t]` | `holdTimerMin` +0xA0 |
//! | 40-41 | `UInt8[0x84 + t]` | `acrobaticDodgeChance` +0xAC |
//! | 42-63 | `float[2 + t]` | `rushAttackDistMult` +0xB0 |
//!
//! The combat style loader (`00505180`) reads `CSSD` (0x40 bytes) at +0x18, `CSTD` (0x5C) at
//! +0x58 and `CSAD` (0x54) at +0xB4, so every trait lies in the form's 0x108 bytes.
//!
//! Cast of the script's float, from JIP's source (MSVC, SSE2): `UInt8 bVal = (int)fVal`
//! (`cvttss2si`, its low byte: `INT_MIN`'s, 0, for NaN and past the range); the float traits
//! store `fVal` as it is.

use super::forms::TESCombatStyle;
use super::{cvttss2si, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2630, get_combat_style_trait_numeric),
    (0x2631, set_combat_style_trait_numeric),
    (0x2632, get_combat_style_flag),
    (0x2633, set_combat_style_flag),
];

/// Where trait `t` (0-63) lies: a float or a byte, at a field indexed from its run's first trait.
enum Slot {
    Float(Ptr<TESCombatStyle>, Field<TESCombatStyle, f32>),
    Byte(Ptr<TESCombatStyle>, Field<TESCombatStyle, u8>),
    /// Trait 9, the weapon restrictions byte (set only to 1 or 2).
    Restrictions,
}

fn slot(c: Ptr<TESCombatStyle>, t: u32) -> Slot {
    use TESCombatStyle as S;
    let f = |first: u32, field| Slot::Float(c.index(t - first, 4), field);
    let b = |first: u32, field| Slot::Byte(c.index(t - first, 1), field);
    match t {
        0..=8 => f(0, S::coverSearchRadius),
        9 => Slot::Restrictions,
        10..=14 => f(10, S::rangedWeapRangeMultMax),
        15 | 16 => b(15, S::dodgeChance),
        17..=24 => f(17, S::dodgeLRTimerMin),
        25 | 26 => b(25, S::blockChance),
        27..=29 => f(27, S::staggerBonusToAttack),
        30 => b(30, S::powerAttackChance),
        31 | 32 => f(31, S::staggerBonusToPower),
        33..=37 => b(33, S::powerAttackN),
        38 | 39 => f(38, S::holdTimerMin),
        40 | 41 => b(40, S::acrobaticDodgeChance),
        _ => f(42, S::rushAttackDistMult),
    }
}

/// The style and the trait, when the read succeeded and the trait is at most 63.
fn style_and_trait(read: &Read) -> Option<(Ptr<TESCombatStyle>, u32)> {
    let t = read.int(1, 0);
    (read.ok && t <= 63).then(|| (Ptr::new(read.form(0)), t))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_combat_style.h:
// Cmd_GetCombatStyleTraitNumeric_Execute.
fn get_combat_style_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((c, t)) = style_and_trait(&read) {
        let value = match slot(c, t) {
            Slot::Float(p, field) => f64::from(e.get(p, field)),
            Slot::Byte(p, field) => f64::from(e.get(p, field)),
            Slot::Restrictions => f64::from(e.get(c, TESCombatStyle::weaponRestrictions)),
        };
        a.set_result(e, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_combat_style.h:
// Cmd_SetCombatStyleTraitNumeric_Execute.
fn set_combat_style_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((c, t)) = style_and_trait(&read) {
        let f_val = read.float(2, 0.0);
        let b_val = cvttss2si(f_val) as u8;
        match slot(c, t) {
            Slot::Float(p, field) => e.set(p, field, f_val),
            Slot::Byte(p, field) => e.set(p, field, b_val),
            Slot::Restrictions => {
                if b_val != 0 && b_val <= 2 {
                    e.set(c, TESCombatStyle::weaponRestrictions, b_val);
                }
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_combat_style.h:
// Cmd_GetCombatStyleFlag_Execute.
/// Flags 0-15, the bits of `csFlags` (a `UInt16`).
fn get_combat_style_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if read.ok && flag <= 15 {
        let c = Ptr::<TESCombatStyle>::new(read.form(0));
        if e.get(c, TESCombatStyle::csFlags) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_combat_style.h:
// Cmd_SetCombatStyleFlag_Execute (with TESCombatStyle::SetFlag).
fn set_combat_style_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if read.ok && flag <= 15 {
        let c = Ptr::<TESCombatStyle>::new(read.form(0));
        let flags = e.get(c, TESCombatStyle::csFlags);
        let bit = 1u16 << flag;
        let flags = if read.int(2, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(c, TESCombatStyle::csFlags, flags);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_combat_style_tests.rs"]
mod tests;
