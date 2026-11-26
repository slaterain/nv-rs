//! `MiddleLowProcess` (the Xbox PDB's source unit of that name, in `fallout/ai`): every function the
//! unit has in the exe, each as its translation in `engine-fallout-ai` does it.
//!
//! The fields read here (Xbox PDB names, checked against the PC code): `LowProcess::pTarget`
//! (+0x40), the list of objects the package made (+0x5c), the package's created object (+0x64),
//! the damage modifiers (+0x94), the process flags byte (+0x30), `iPackageEvaluateHour` (+0xb4)
//! and the temporary modifiers (+0xb8). An actor's process is its word at +0x68 (`008d8520`, a
//! folded body), a process's level its word at +0x28 (`0045cd60`; 2 is a middle-low process), a
//! reference's parent cell its word at +0x40 and its extra data
//! list at +0x44; a package's type is its word at +0x18, its flags at +0x1c, its procedure the
//! signed byte at +0x20 and its location at +0x2c (a location's kind the signed byte at +0, its reference
//! at +8).
//!
//! The process classes' slots, as the exe's vtables hold them (`MiddleLowProcess` `01089bcc`,
//! `MiddleHighProcess` `0108904c`, `HighProcess` `01087864`): 0x22c is `007fa950` on all three; 0x24
//! is [`checkfor_new_package`] on the first only; 0x280 and 0x284 are `008d0450` and `008d0470` on
//! the first only; 0x3a8 is [`damage_mod_actor_value_ov2`] on the first two; 0x1bc, 0x1c0, 0x1c8,
//! 0x5a4 and 0x5a8 are `LowProcess`'s flag getters and setters (`008d8060`, `008d8080`, `008d7ee0`,
//! `008d8000`, `008d8020`) on all three; 0x288 is `LowProcess::AddToProcedureIndexRunning`
//! (`009119b0`) on the first only. A call through one of them is the typed call, or the flag test
//! or store in place, when the slot holds that body, and a call through the slot otherwise.
//!
//! The callees whose bodies are one load, store or test are done in place: the actor value's flag
//! (`00406d70`), the buffers' change flags (`00428110`, `0042ce30`, `004280f0`), the calendar's hour
//! (`00867da0` with `00526ac0`), the settings' value addresses (`00403e20`, `00408d60`, `0043d4d0`),
//! the save/load object's version, position and records, the package and location getters and the
//! leaves. The values the exe works out and never reads (the hours `Update` stores, the seconds of
//! the travel's hours, the reach distance) are not worked out.

use super::base_process::{
    self, buffer_position, current_version, form_name, loading_record, low_half, record_words,
    runtime_cast, save_load_game, save_log_enabled, saving_record, use_save_game_blocks,
};
use super::{low_process, FAMILY};
use crate::native::{float_precision, to_float, Real};
use crate::{ai_queries, crt, interface, slot_target};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// `LowProcess::pTarget`.
const TARGET: u32 = 0x40;
/// The list of objects the package made (a `BSSimpleList`).
const MADE_OBJECTS: u32 = 0x5c;
/// The object the package created.
const CREATED_OBJECT: u32 = 0x64;
/// The damage modifier list.
const DAMAGE_MODIFIERS: u32 = 0x94;
/// The process flags byte (`LowProcess`'s getters and setters).
const PROCESS_FLAGS: u32 = 0x30;
/// `iPackageEvaluateHour`: the hour of the last package check.
const LAST_PACKAGE_HOUR: u32 = 0xb4;
/// The temporary modifier list.
const TEMPORARY_MODIFIERS: u32 = 0xb8;

/// The actor value infos, by actor value; an info's flags at +0x44.
const ACTOR_VALUE_INFOS: u32 = 0x011d_61c8;
/// The actor value flag of a value with no temporary modifiers, and that of a value whose damage
/// modifier is of the second kind.
const NO_TEMPORARY_MODIFIERS: u32 = 0x100;
const DAMAGE_KIND_FLAG: u32 = 0x200;
/// The change flag of the temporary modifiers.
const MODIFIERS_CHANGED: u32 = 0x0010_0000;

/// `LowProcess`'s functions the overrides call first (by address: not typed yet).
const LOW_GET_ACTOR_FLOAT_VALUE: u32 = 0x0090_7380;
const LOW_GET_ACTOR_VALUE: u32 = 0x0090_72d0;
const LOW_SAVE_GAME_BUFFER: u32 = 0x0091_0450;
const LOW_DESTRUCTOR: u32 = 0x0090_6fe0;
const LOW_CHECK_FOR_NEW_PACKAGE: u32 = 0x0090_a1a0;
const LOW_GET_SAVE_SIZE: u32 = 0x0090_f8b0;
const LOW_SAVE_GAME: u32 = 0x0090_fa70;
const LOW_LOAD_GAME: u32 = 0x0090_fd80;
const LOW_REVERT: u32 = 0x0091_0310;
const LOW_LOAD_GAME_BUFFER: u32 = 0x0091_0630;
const LOW_REVERT_BUFFER: u32 = 0x0091_0a60;
/// `ModifierList`'s functions on the modifier lists.
const MODIFIER_GET: u32 = 0x0093_7730;
const MODIFIER_ADJUST: u32 = 0x0093_7480;
const MODIFIER_SAVE_GAME_BUFFER: u32 = 0x0093_7b50;
const MODIFIER_COPY: u32 = 0x0093_7160;
const MODIFIER_DESTRUCTOR: u32 = 0x0093_7030;
const MODIFIER_GET_SAVE_SIZE: u32 = 0x0093_7ab0;
const MODIFIER_SAVE_GAME: u32 = 0x0093_78d0;
const MODIFIER_LOAD_GAME: u32 = 0x0093_79c0;
const MODIFIER_LOAD_GAME_BUFFER: u32 = 0x0093_7c60;
const MODIFIER_REVERT: u32 = 0x0093_7b30;
const MODIFIER_REVERT_BUFFER: u32 = 0x0093_7d50;
/// The save buffer's byte writer (with a last 0) and the load buffer's byte reader.
const BUFFER_WRITE: u32 = 0x0086_5e50;
const BUFFER_READ: u32 = 0x0086_4980;
/// `ActorValue::CheckClampDamageModifier` (`cdecl`: owner, value, float; ST0).
const CHECK_CLAMP_DAMAGE_MODIFIER: u32 = 0x0066_eea0;
/// The free.
const FREE: u32 = 0x0040_1030;
/// The player character, a global.
const PLAYER: u32 = 0x011d_ea3c;
/// `MiddleLowProcess`'s vtable.
const MIDDLE_LOW_PROCESS_VTABLE: u32 = 0x0108_9bcc;
/// The RTTI type descriptor of `TESObjectREFR` (the combat target's cast is from it).
const TYPE_REFERENCE: u32 = 0x0118_41cc;

/// The game calendar: its hour global (+0xc) and day global (+8); a global's value is its float at
/// +0x24. Without the hour global the hour is the float at `01018f7c` (12.0), without the day
/// global the day is the float at `01018f8c`.
const CALENDAR: u32 = 0x011d_e7b8;
const CALENDAR_HOUR_GLOBAL: u32 = 0xc;
const CALENDAR_DAY_GLOBAL: u32 = 8;
const GLOBAL_VALUE: u32 = 0x24;
const DEFAULT_HOUR: u32 = 0x0101_8f7c;
const DEFAULT_DAY: u32 = 0x0101_8f8c;
/// The setting whose float gives the combat rounds, and the double the one-hour combat multiplies
/// it by (4.0). A setting's value is at +4.
const COMBAT_ROUNDS_SETTING: u32 = 0x011c_d6cc;
const ONE_HOUR_FACTOR: u32 = 0x0101_db80;
const SETTING_VALUE: u32 = 4;

/// The save/load object's numbers in the reports: the source file (the unit's path) and the
/// lines of `GetSaveSize`, `SaveGame` and `LoadGame`'s header and end.
const SOURCE_FILE: u32 = 0x0108_a3f0;
const GET_SAVE_SIZE_LINE: u32 = 0x263;
const SAVE_GAME_LINE: u32 = 0x27a;
const LOAD_HEADER_LINE: u32 = 0x287;
const LOAD_END_LINE: u32 = 0x290;

/// Whether the actor value `code` has `flag` (`00406d70`: the flags of its info, at +0x44).
pub(super) fn value_has_flag(e: &Engine, code: u32, flag: u32) -> bool {
    let info = e
        .mem
        .u32(ACTOR_VALUE_INFOS.wrapping_add(code.wrapping_mul(4)));
    e.mem.u32(info.wrapping_add(0x44)) & flag != 0
}

/// The temporary modifier list of a process.
#[inline]
fn temporary_modifiers(this: Ptr) -> u32 {
    this.addr().wrapping_add(TEMPORARY_MODIFIERS)
}

/// `_ftol2_sse` (the CRT family's): the value truncated, its low word.
fn truncate(e: &mut Engine, value: f64) -> u32 {
    let whole = typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, value));
    crt::low_word(whole)
}

/// An integer as the float the exe stores (`FILD`, `FSTP dword`): rounded once to the nearest.
#[inline]
fn int_to_float(value: i32) -> f32 {
    to_float(f64::from(value))
}

/// `Calendar::GetHour` (`00867da0`): the hour global's value, or the default hour without one, as
/// loaded (`FLD`, which quiets a signalling NaN).
pub(super) fn calendar_hour(e: &Engine) -> u32 {
    let global = e.mem.u32(CALENDAR + CALENDAR_HOUR_GLOBAL);
    let bits = if global != 0 {
        e.mem.u32(global.wrapping_add(GLOBAL_VALUE))
    } else {
        e.mem.u32(DEFAULT_HOUR)
    };
    x87_quiet_bits(bits)
}

// Typed from 00867d60
/// `Calendar::GetDay` (engine map; typed in this family, whose processes ask it): the day global's
/// value (`pGameDay`, +8; a global's value done in place, `00526ac0`), or the default day
/// (`01018f8c`) without one, stored as a float, loaded again (`FLD`, which quiets a signalling NaN)
/// and truncated (`_ftol2`).
pub fn calendar_day(e: &mut Engine, this: Ptr) -> u32 {
    let global = e.mem.u32(this.addr().wrapping_add(CALENDAR_DAY_GLOBAL));
    let bits = if global != 0 {
        e.mem.u32(global.wrapping_add(GLOBAL_VALUE))
    } else {
        e.mem.u32(DEFAULT_DAY)
    };
    truncate(e, loaded(bits))
}

/// A float bit pattern as the x87 stack holds it after `FLD dword`.
#[inline]
pub(super) fn loaded(bits: u32) -> f64 {
    f64::from(f32::from_bits(x87_quiet_bits(bits)))
}

// Typed from 0092ce00
/// `MiddleLowProcess::GetActorFloatValue` (Xbox PDB): `LowProcess::GetActorFloatValue`, plus the
/// temporary modifier of the value (`ModifierList::GetModifier`; the sum stored as a float) unless
/// the value has no temporary modifiers. Returns ST0.
pub fn get_actor_float_value(e: &mut Engine, this: Ptr, actor: u32, code: u32, extra: u32) -> f64 {
    let skips = value_has_flag(e, code, NO_TEMPORARY_MODIFIERS);
    let base = e
        .call(LOW_GET_ACTOR_FLOAT_VALUE, &args![this, actor, code, extra])
        .st0;
    if skips {
        return base;
    }
    let modifier = e
        .call(MODIFIER_GET, &args![temporary_modifiers(this), code & 0xff])
        .st0;
    // `FADD qword` of the stored base, then `FSTP dword`: one rounding to a float on either thread.
    f64::from(to_float(modifier + base))
}

// Typed from 0092cd90
/// `MiddleLowProcess::GetActorValue` (Xbox PDB): `LowProcess::GetActorValue`, plus the temporary
/// modifier truncated (`_ftol2`) unless the value has no temporary modifiers.
pub fn get_actor_value(e: &mut Engine, this: Ptr, actor: u32, code: u32, extra: u32) -> i32 {
    let skips = value_has_flag(e, code, NO_TEMPORARY_MODIFIERS);
    let base = e
        .call(LOW_GET_ACTOR_VALUE, &args![this, actor, code, extra])
        .i32();
    if skips {
        return base;
    }
    let modifier = e
        .call(MODIFIER_GET, &args![temporary_modifiers(this), code & 0xff])
        .st0;
    let whole = i32::from_ne_bytes(truncate(e, modifier).to_ne_bytes());
    whole.wrapping_add(base)
}

// Typed from 0092cec0
/// `MiddleLowProcess::TempModActorValue` of a float (Xbox PDB `TempModActorValue_ov2`): unless the
/// value has no temporary modifiers, its temporary modifier adjusted by `amount` (loaded and stored,
/// which quiets a signalling NaN; `ModifierList::AdjustModifier`, kind 2). The first word unread.
pub fn temp_mod_actor_value_float(e: &mut Engine, this: Ptr, _unused: u32, code: u32, amount: f32) {
    if value_has_flag(e, code, NO_TEMPORARY_MODIFIERS) {
        return;
    }
    let amount = x87_quiet_bits(amount.to_bits());
    e.call(
        MODIFIER_ADJUST,
        &args![temporary_modifiers(this), code & 0xff, amount, 2u32],
    );
}

// Typed from 0092cf10
/// `MiddleLowProcess::GetTemporaryModifier` (Xbox PDB): the temporary modifiers'
/// `ModifierList::GetModifier`; its ST0.
pub fn get_temporary_modifier(e: &mut Engine, this: Ptr, code: u8) -> f64 {
    e.call(
        MODIFIER_GET,
        &args![temporary_modifiers(this), u32::from(code)],
    )
    .st0
}

// Typed from 0092ea30
/// `MiddleLowProcess::SaveGame` of a buffer (Xbox PDB `SaveGame_ov2`): `LowProcess`'s, the last
/// package hour (4 bytes), then the temporary modifiers when the buffer's change flags (its word at
/// +0x17) have theirs.
pub fn save_game_buffer(e: &mut Engine, this: Ptr, buffer: Ptr) {
    e.call(LOW_SAVE_GAME_BUFFER, &args![this, buffer]);
    let hour = this.addr().wrapping_add(LAST_PACKAGE_HOUR);
    e.call(BUFFER_WRITE, &args![buffer, hour, 4u32, 0u32]);
    if e.mem.u32(buffer.addr().wrapping_add(0x17)) & MODIFIERS_CHANGED != 0 {
        e.call(
            MODIFIER_SAVE_GAME_BUFFER,
            &args![temporary_modifiers(this), buffer],
        );
    }
}

// Typed from 0092ca70
/// The middle-low part of `BaseProcess::Copy`: the last package hour and the temporary modifiers
/// (`00937160`) copied from `other`.
pub fn copy(e: &mut Engine, this: Ptr, other: Ptr) {
    let hour = e.mem.u32(other.addr().wrapping_add(LAST_PACKAGE_HOUR));
    e.mem
        .set_u32(this.addr().wrapping_add(LAST_PACKAGE_HOUR), hour);
    e.call(
        MODIFIER_COPY,
        &args![temporary_modifiers(this), temporary_modifiers(other)],
    );
}

// Typed from 0092ca00
/// The `MiddleLowProcess` destructor: its vtable stored, the temporary modifiers destroyed
/// (`00937030`), then `LowProcess`'s destructor (`00906fe0`).
pub fn destructor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), MIDDLE_LOW_PROCESS_VTABLE);
    e.call(MODIFIER_DESTRUCTOR, &args![temporary_modifiers(this)]);
    e.call(LOW_DESTRUCTOR, &args![this]);
}

// Typed from 0092c9d0
/// `MiddleLowProcess`'s scalar deleting destructor: [`destructor`], then `this` freed when bit 0 of
/// `flags` is set. Answers `this`.
pub fn scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x0092_ca00, destructor(e, this));
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// Typed from 0092cfd0
/// `MiddleLowProcess::CheckforNewPackage` (Xbox PDB): without `force`, a process with a package
/// whose last package hour is still the calendar's hour (truncated) answers false; otherwise
/// `LowProcess::CheckforNewPackage`, and the hour becomes the last package hour.
pub fn checkfor_new_package(e: &mut Engine, this: Ptr, actor: u32, force: u8) -> u8 {
    let hour = loaded(calendar_hour(e));
    if force == 0 && ai_queries::queries::process_slot_22c(e, this.addr()) != 0 {
        let whole = truncate(e, hour);
        if e.mem.u32(this.addr().wrapping_add(LAST_PACKAGE_HOUR)) == whole {
            return 0;
        }
    }
    let result = e
        .call(
            LOW_CHECK_FOR_NEW_PACKAGE,
            &args![this, actor, u32::from(force)],
        )
        .u8();
    let whole = truncate(e, hour);
    e.mem
        .set_u32(this.addr().wrapping_add(LAST_PACKAGE_HOUR), whole);
    result
}

// Typed from 0092e400
/// `MiddleLowProcess::Update` of a time step (Xbox PDB `Update_ov2`): `LowProcess`'s, the step
/// loaded and stored (which quiets a signalling NaN).
pub fn update_step(e: &mut Engine, this: Ptr, actor: u32, delta: f32) {
    let delta = x87_quiet_bits(delta.to_bits());
    typed_call!(
        FAMILY,
        0x0090_c360,
        low_process::update_buffer(e, this, Ptr::new(actor), f32::from_bits(delta))
    );
}

// Typed from 00907430
/// `MiddleLowProcess::DamageModActorValue` of an integer (Xbox PDB): the process's slot 0x3a8 with
/// the amount as a float ([`damage_mod_actor_value_ov2`] when the slot holds it).
pub fn damage_mod_actor_value(e: &mut Engine, this: Ptr, actor: u32, code: u32, amount: i32) {
    let amount = int_to_float(amount);
    if slot_target(e, this.addr(), 0x3a8) == 0x0092_cf30 {
        typed_call!(
            FAMILY,
            0x0092_cf30,
            damage_mod_actor_value_ov2(e, this, actor, code, amount)
        );
    } else {
        e.vcall(this.addr(), 0x3a8, &args![actor, code, amount.to_bits()]);
    }
}

// Typed from 0092ce70
/// `MiddleLowProcess::TempModActorValue` of an integer (Xbox PDB): unless the value has no temporary
/// modifiers, its temporary modifier adjusted by `amount` as a float (kind 2). The first word is
/// unread.
pub fn temp_mod_actor_value(e: &mut Engine, this: Ptr, _unused: u32, code: u32, amount: i32) {
    if value_has_flag(e, code, NO_TEMPORARY_MODIFIERS) {
        return;
    }
    let amount = int_to_float(amount).to_bits();
    e.call(
        MODIFIER_ADJUST,
        &args![temporary_modifiers(this), code & 0xff, amount, 2u32],
    );
}

// Typed from 0092cf30
/// `MiddleLowProcess::DamageModActorValue` of a float (Xbox PDB `DamageModActorValue_ov2`): unless
/// the value has no temporary modifiers, the damage modifiers adjusted by
/// `ActorValue::CheckClampDamageModifier` of the actor's value owner (`actor + 0xa4`, 0 for no
/// actor), the value and `amount` (stored as a float), of the second kind when the value has that
/// flag and of the first otherwise.
pub fn damage_mod_actor_value_ov2(e: &mut Engine, this: Ptr, actor: u32, code: u32, amount: f32) {
    if value_has_flag(e, code, NO_TEMPORARY_MODIFIERS) {
        return;
    }
    let owner = if actor != 0 {
        actor.wrapping_add(0xa4)
    } else {
        0
    };
    let amount = x87_quiet_bits(amount.to_bits());
    let clamped = e
        .call(CHECK_CLAMP_DAMAGE_MODIFIER, &args![owner, code, amount])
        .st0;
    let clamped = to_float(clamped).to_bits();
    let kind = u32::from(value_has_flag(e, code, DAMAGE_KIND_FLAG));
    let modifiers = this.addr().wrapping_add(DAMAGE_MODIFIERS);
    e.call(
        MODIFIER_ADJUST,
        &args![modifiers, code & 0xff, clamped, kind],
    );
}

/// The combat rounds: the rounds setting's float (`FLD`) combined by `combine`, truncated.
fn combat_rounds(e: &mut Engine, combine: impl FnOnce(&Engine, f64) -> f64) -> u32 {
    let value = loaded(e.mem.u32(COMBAT_ROUNDS_SETTING + SETTING_VALUE));
    let value = combine(e, value);
    truncate(e, value)
}

/// The combat target (`+0x40`) cast from `TESObjectREFR` to `Actor` (0 when it is not one).
fn target_actor(e: &mut Engine, this: Ptr) -> u32 {
    let target = e.mem.u32(this.addr().wrapping_add(TARGET));
    runtime_cast(e, target, TYPE_REFERENCE, base_process::TYPE_ACTOR)
}

/// An actor's slot 0x22c with 0: whether it is out of the fight.
fn actor_slot_22c(e: &mut Engine, actor: u32) -> bool {
    e.vcall(actor, 0x22c, &args![0u32]).bool()
}

/// An actor's process (its word at +0x68; `008d8520` in the exe).
#[inline]
fn actor_process(e: &Engine, actor: u32) -> u32 {
    e.mem.u32(actor.wrapping_add(0x68))
}

/// A process's level (its word at +0x28; `0045cd60` in the exe): 0 high, 1 middle-high, 2
/// middle-low.
#[inline]
fn process_level(e: &Engine, process: u32) -> u32 {
    e.mem.u32(process.wrapping_add(0x28))
}

// Typed from 0092cab0
/// `MiddleLowProcess::ProcessCombat` (Xbox PDB): the rounds are twice the rounds setting
/// (truncated); the target is the combat target cast to `Actor`. Without a combat target, or with
/// a target that is a deleted or disabled form or out of the fight (its slot 0x22c), the actor stops
/// combat (slot 0x434) and ends its interrupt package. With a target whose process is middle-low,
/// up to that many rounds run while the actor is in the fight: the actor's slot 0x438 and, for a
/// cast target still in the fight with a middle-low process, the target's; then a target that took
/// a round gets its process's slot 0x28. Any other target: the process's slot 0x298 with (actor,
/// 1, 0x101, 0).
pub fn process_combat(e: &mut Engine, this: Ptr, actor: u32) {
    let mut rounds = combat_rounds(e, |_, value| value + value);
    let mut target = target_actor(e, this);
    // The form flags (+8) `00440da0` (0x800) and `00440d80` (0x20) test, read once.
    let stops = e.mem.u32(this.addr().wrapping_add(TARGET)) == 0
        || e.mem.u32(target.wrapping_add(8)) & 0x820 != 0
        || actor_slot_22c(e, target);
    if stops {
        e.vcall(actor, 0x434, &args![0u32]);
        e.call(0x0088_1680, &args![actor, 0u32]);
        return;
    }
    let engaged = target != 0 && {
        let object = actor_process(e, target);
        object != 0 && process_level(e, object) == 2
    };
    if !engaged {
        e.vcall(this.addr(), 0x298, &args![actor, 1u32, 0x101u32, 0u32]);
        return;
    }
    let mut fought = false;
    while rounds != 0 && !actor_slot_22c(e, actor) {
        e.vcall(actor, 0x438, &[]);
        target = target_actor(e, this);
        if target != 0
            && !actor_slot_22c(e, target)
            && process_level(e, actor_process(e, target)) == 2
        {
            e.vcall(target, 0x438, &[]);
            fought = true;
        }
        rounds = rounds.wrapping_sub(1);
    }
    if fought && target != 0 {
        let object = actor_process(e, target);
        e.vcall(object, 0x28, &[]);
    }
}

// Typed from 0092cc90
/// `MiddleLowProcess::ProcessCombatOneHour` (Xbox PDB): the rounds are the rounds setting times the
/// double at `0101db80` (truncated). While rounds remain and the actor is in the fight: the actor's
/// slot 0x438, the combat target cast to `Actor` and, when it is set and in the fight, its slot
/// 0x438. Then a last target in the fight gets its process's slot 0x28. Answers false; the
/// second word unread.
pub fn process_combat_one_hour(e: &mut Engine, this: Ptr, actor: u32, _unused: u32) -> bool {
    // `FMUL qword` at the thread's precision, then `_ftol2`.
    let mut rounds = combat_rounds(e, |e, value| {
        let factor = e.mem.f64(ONE_HOUR_FACTOR);
        if float_precision() {
            f64::from(f32::narrow(value) * f32::narrow(factor))
        } else {
            value * factor
        }
    });
    let mut target = 0u32;
    while rounds != 0 && !actor_slot_22c(e, actor) {
        e.vcall(actor, 0x438, &[]);
        target = target_actor(e, this);
        if target != 0 && !actor_slot_22c(e, target) {
            e.vcall(target, 0x438, &[]);
        }
        rounds = rounds.wrapping_sub(1);
    }
    if target != 0 && !actor_slot_22c(e, target) {
        let object = actor_process(e, target);
        e.vcall(object, 0x28, &[]);
    }
    false
}

/// The part's size report (`Error`, line `line`): with the record being saved, its id, its form's
/// name and its word at +5 (`form_format`), else the size alone (`format`).
fn report_part(e: &mut Engine, part: u32, form_format: u32, format: u32, line: u32) {
    let record = saving_record(e);
    if record != 0 {
        let words = record_words(e, record, false);
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words form_format, part, words.form_id, words.name, words.flags, line, SOURCE_FILE);
    } else {
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words format, part, line, SOURCE_FILE);
    }
}

// Typed from 0092e420
/// `MiddleLowProcess::GetSaveSize` (Xbox PDB): `LowProcess::GetSaveSize`, 6 more with save game
/// blocks, 4 for the last package hour, the temporary modifiers' size with their change flag: a
/// 16-bit size. With the save log setting on, this part's size is reported (line 0x263).
pub fn get_save_size(e: &mut Engine, this: Ptr, flags: u32, form: u32) -> u16 {
    let base = e.call(LOW_GET_SAVE_SIZE, &args![this, flags, form]).u16();
    let mut size = base;
    if use_save_game_blocks(e) {
        size = size.wrapping_add(6);
    }
    size = size.wrapping_add(4);
    if flags & MODIFIERS_CHANGED != 0 {
        let modifiers = e
            .call(MODIFIER_GET_SAVE_SIZE, &args![temporary_modifiers(this)])
            .u16();
        size = size.wrapping_add(modifiers);
    }
    if save_log_enabled(e) {
        let part = u32::from(size).wrapping_sub(u32::from(base));
        report_part(
            e,
            part,
            base_process::SAVE_SIZE_FORM_FORMAT,
            base_process::SAVE_SIZE_FORMAT,
            GET_SAVE_SIZE_LINE,
        );
    }
    size
}

// Typed from 0092e560
/// `MiddleLowProcess::SaveGame` (Xbox PDB): `LowProcess::SaveGame`; with save game blocks the block
/// tag and a 16-bit zero length placeholder; the last package hour; the temporary modifiers with
/// their change flag. With the save log setting on, the bytes written since the start are reported
/// (line 0x27a). With blocks, the block's length (the position now less the placeholder's) stored
/// as 16 bits at the placeholder, after a warning when it is past 0xffff.
pub fn save_game(e: &mut Engine, this: Ptr, flags: u32, form: u32) {
    e.call(LOW_SAVE_GAME, &args![this, flags, form]);
    e.with_stack(8, |e, locals| {
        let length_at = locals.addr();
        let tag_at = locals.addr().wrapping_add(4);
        e.mem.set_u16(length_at, 0);
        let mut placeholder = 0u32;
        let start = buffer_position(e);
        if use_save_game_blocks(e) {
            e.mem.set_u32(tag_at, base_process::BLOCK_TAG);
            let save_load = save_load_game(e);
            e.call(base_process::WRITE_BYTES, &args![save_load, tag_at, 4u32]);
            placeholder = buffer_position(e);
            let save_load = save_load_game(e);
            e.call(
                base_process::WRITE_BYTES,
                &args![save_load, length_at, 2u32],
            );
        }
        let save_load = save_load_game(e);
        let hour = this.addr().wrapping_add(LAST_PACKAGE_HOUR);
        e.call(base_process::WRITE_BYTES, &args![save_load, hour, 4u32]);
        if flags & MODIFIERS_CHANGED != 0 {
            e.call(MODIFIER_SAVE_GAME, &args![temporary_modifiers(this)]);
        }
        if save_log_enabled(e) {
            let written = buffer_position(e).wrapping_sub(start);
            report_part(
                e,
                written,
                base_process::SAVE_GAME_FORM_FORMAT,
                base_process::SAVE_GAME_FORMAT,
                SAVE_GAME_LINE,
            );
        }
        if use_save_game_blocks(e) {
            let end = buffer_position(e);
            if end > placeholder.wrapping_add(0xffff) {
                e.call(
                    base_process::SAVELOAD_WARNING,
                    &args![
                        base_process::BLOCK_TOO_LARGE_FORMAT,
                        SOURCE_FILE,
                        SAVE_GAME_LINE
                    ],
                );
            }
            e.mem
                .set_u16(placeholder, low_half(end.wrapping_sub(placeholder)));
        }
    });
}

// Typed from 0092e730
/// `MiddleLowProcess::LoadGame` (Xbox PDB): `LowProcess::LoadGame`; with save game blocks, the block
/// tag read (a warning at line 0x287 when it is not `KOLB`, with the record being loaded or the
/// current version), then the block's start noted and its 16-bit length read; the last package
/// hour; the temporary modifiers with their change flag. With blocks, a position past the block's
/// end or short of it is warned of (line 0x290), with the record being loaded or the version.
pub fn load_game(e: &mut Engine, this: Ptr, flags: u32, argument_0: u32, argument_1: u32) {
    e.call(LOW_LOAD_GAME, &args![this, flags, argument_0, argument_1]);
    e.with_stack(8, |e, locals| {
        let length_at = locals.addr();
        let tag_at = locals.addr().wrapping_add(4);
        e.mem.set_u16(length_at, 0);
        let mut block_start = 0u32;
        if use_save_game_blocks(e) {
            let save_load = save_load_game(e);
            e.call(base_process::READ_BYTES, &args![save_load, tag_at, 4u32]);
            if e.mem.u32(tag_at) != base_process::BLOCK_TAG {
                bad_header(e);
            }
            block_start = buffer_position(e);
            let save_load = save_load_game(e);
            e.call(base_process::READ_BYTES, &args![save_load, length_at, 2u32]);
        }
        let save_load = save_load_game(e);
        let hour = this.addr().wrapping_add(LAST_PACKAGE_HOUR);
        e.call(base_process::READ_BYTES, &args![save_load, hour, 4u32]);
        if flags & MODIFIERS_CHANGED != 0 {
            e.call(MODIFIER_LOAD_GAME, &args![temporary_modifiers(this)]);
        }
        if !use_save_game_blocks(e) {
            return;
        }
        let end = buffer_position(e);
        let record = loading_record(e);
        let expected = u32::from(e.mem.u16(length_at)).wrapping_add(block_start);
        if record != 0 {
            let id = e.mem.u32(record);
            let form = e.call(base_process::FORM_FOR_ID, &args![id]).u32();
            let (format, bytes) = if end > expected {
                (
                    base_process::OVERRUN_FORM_FORMAT,
                    end.wrapping_sub(expected),
                )
            } else if end < expected {
                (
                    base_process::UNDERRUN_FORM_FORMAT,
                    expected.wrapping_sub(end),
                )
            } else {
                return;
            };
            let record_flags = e.mem.u32(record.wrapping_add(5));
            let version = u32::from(e.mem.u8(record.wrapping_add(9)));
            let name = form_name(e, form);
            let form_id = e.mem.u32(record);
            e.call(
                base_process::SAVELOAD_WARNING,
                &args![
                    format,
                    bytes,
                    SOURCE_FILE,
                    LOAD_END_LINE,
                    form_id,
                    name,
                    version,
                    record_flags
                ],
            );
        } else {
            let (format, bytes) = if end > expected {
                (base_process::OVERRUN_FORMAT, end.wrapping_sub(expected))
            } else if end < expected {
                (base_process::UNDERRUN_FORMAT, expected.wrapping_sub(end))
            } else {
                return;
            };
            let version = u32::from(current_version(e));
            e.call(
                base_process::SAVELOAD_WARNING,
                &args![format, bytes, SOURCE_FILE, LOAD_END_LINE, version],
            );
        }
    });
}

/// `LoadGame`'s tag check: the warning at line 0x287, with the record being loaded or with the
/// current version.
fn bad_header(e: &mut Engine) {
    let record = loading_record(e);
    if record != 0 {
        let words = record_words(e, record, true);
        e.call(
            base_process::SAVELOAD_WARNING,
            &args![
                base_process::BAD_HEADER_FORM_FORMAT,
                SOURCE_FILE,
                LOAD_HEADER_LINE,
                words.form_id,
                words.name,
                words.version,
                words.flags
            ],
        );
    } else {
        let version = u32::from(current_version(e));
        e.call(
            base_process::SAVELOAD_WARNING,
            &args![
                base_process::BAD_HEADER_FORMAT,
                SOURCE_FILE,
                LOAD_HEADER_LINE,
                version
            ],
        );
    }
}

// Typed from 0092e9f0
/// `MiddleLowProcess::Revert` (Xbox PDB): `LowProcess::Revert`, then with the change flag the
/// temporary modifiers' revert (`00937b30`).
pub fn revert(e: &mut Engine, this: Ptr, flags: u32, argument: u32) {
    e.call(LOW_REVERT, &args![this, flags, argument]);
    if flags & MODIFIERS_CHANGED != 0 {
        e.call(MODIFIER_REVERT, &args![temporary_modifiers(this)]);
    }
}

// Typed from 0092eaa0
/// `MiddleLowProcess::LoadGame` of a buffer (Xbox PDB `LoadGame_ov2`): `LowProcess`'s, the last
/// package hour (4 bytes), then the temporary modifiers (`ModifierList::LoadGame_ov2`) when the
/// buffer's change flags (its word at +0x17) have theirs.
pub fn load_game_buffer(e: &mut Engine, this: Ptr, buffer: u32) {
    e.call(LOW_LOAD_GAME_BUFFER, &args![this, buffer]);
    let hour = this.addr().wrapping_add(LAST_PACKAGE_HOUR);
    e.call(BUFFER_READ, &args![buffer, hour, 4u32]);
    if e.mem.u32(buffer.wrapping_add(0x17)) & MODIFIERS_CHANGED != 0 {
        e.call(
            MODIFIER_LOAD_GAME_BUFFER,
            &args![temporary_modifiers(this), buffer],
        );
    }
}

// Typed from 0092eb00
/// `MiddleLowProcess::Revert` of a buffer (Xbox PDB `Revert_ov2`): `LowProcess`'s, then `00937d50`
/// on the temporary modifiers when the buffer's revert flags (its word at +0x2c) have their change
/// flag.
pub fn revert_buffer(e: &mut Engine, this: Ptr, buffer: u32) {
    e.call(LOW_REVERT_BUFFER, &args![this, buffer]);
    if e.mem.u32(buffer.wrapping_add(0x2c)) & MODIFIERS_CHANGED != 0 {
        e.call(
            MODIFIER_REVERT_BUFFER,
            &args![temporary_modifiers(this), buffer],
        );
    }
}

// ---- Update ----

/// A package's type (its word at +0x18, `009611e0`), procedure (the signed byte at +0x20,
/// `0041ca90`), flags (+0x1c) and location (+0x2c, `0055b980`).
#[inline]
pub(super) fn package_kind(e: &Engine, package: u32) -> u32 {
    e.mem.u32(package.wrapping_add(0x18))
}
#[inline]
pub(super) fn procedure(e: &Engine, package: u32) -> i32 {
    i32::from(e.mem.i8(package.wrapping_add(0x20)))
}
#[inline]
fn package_flag(e: &Engine, package: u32, flag: u32) -> bool {
    e.mem.u32(package.wrapping_add(0x1c)) & flag != 0
}
#[inline]
fn package_location(e: &Engine, package: u32) -> u32 {
    e.mem.u32(package.wrapping_add(0x2c))
}

/// The process's current package (its slot 0x22c).
pub(super) fn current_package(e: &mut Engine, process: u32) -> u32 {
    ai_queries::queries::process_slot_22c(e, process)
}

/// The process's slot 0x280 (the running procedure index).
pub(super) fn procedure_index_running(e: &mut Engine, process: u32) -> u32 {
    if slot_target(e, process, 0x280) == 0x008d_0450 {
        typed_call!(
            FAMILY,
            0x008d_0450,
            base_process::procedure_index_running(e, Ptr::new(process))
        )
    } else {
        e.vcall(process, 0x280, &[]).u32()
    }
}

/// The process's slot 0x284 (sets the running procedure index).
pub(super) fn set_procedure_index_running(e: &mut Engine, process: u32, index: u32) {
    if slot_target(e, process, 0x284) == 0x008d_0470 {
        typed_call!(
            FAMILY,
            0x008d_0470,
            base_process::set_procedure_index_running(e, Ptr::new(process), index)
        );
    } else {
        e.vcall(process, 0x284, &args![index]);
    }
}

/// A process slot holding one of `LowProcess`'s flag tests (`body`, of `flag`): the test in place,
/// or the call through the slot.
pub(super) fn process_flag(e: &mut Engine, process: u32, slot: u32, body: u32, flag: u8) -> bool {
    if slot_target(e, process, slot) == body {
        e.mem.u8(process.wrapping_add(PROCESS_FLAGS)) & flag != 0
    } else {
        e.vcall(process, slot, &[]).bool()
    }
}

/// A process slot holding one of `LowProcess`'s flag setters (`body`, of `flag`): the store in
/// place, or the call through the slot.
pub(super) fn set_process_flag(
    e: &mut Engine,
    process: u32,
    slot: u32,
    body: u32,
    flag: u8,
    on: u8,
) {
    if slot_target(e, process, slot) == body {
        let at = process.wrapping_add(PROCESS_FLAGS);
        let flags = e.mem.u8(at);
        e.mem
            .set_u8(at, if on != 0 { flags | flag } else { flags & !flag });
    } else {
        e.vcall(process, slot, &args![u32::from(on)]);
    }
}

/// The process's slot 0x24 (`CheckforNewPackage`).
fn check_for_new_package(e: &mut Engine, process: u32, actor: u32, force: u8) -> bool {
    if slot_target(e, process, 0x24) == 0x0092_cfd0 {
        typed_call!(
            FAMILY,
            0x0092_cfd0,
            checkfor_new_package(e, Ptr::new(process), actor, force)
        ) != 0
    } else {
        e.vcall(process, 0x24, &args![actor, u32::from(force)])
            .bool()
    }
}

/// The action the table gives for the current package's type and the running procedure index.
fn procedure_action(e: &mut Engine, process: u32) -> u32 {
    let package = current_package(e, process);
    let kind = package_kind(e, package);
    let index = procedure_index_running(e, process);
    let row = e.mem.u32(0x011a_3ff0u32.wrapping_add(kind.wrapping_mul(4)));
    e.mem.u32(row.wrapping_add(index.wrapping_mul(4)))
}

/// The actor's process's slot 0x28 (`Update`'s last step).
fn update_actor_process(e: &mut Engine, actor: u32) {
    let object = actor_process(e, actor);
    e.vcall(object, 0x28, &[]);
}

/// The current package's slot 0x13c with (actor, 0, -1.0, 0).
fn package_test_13c(e: &mut Engine, process: u32, actor: u32) -> bool {
    let package = current_package(e, process);
    let minus_one = x87_quiet_bits(e.mem.u32(base_process::MINUS_ONE_FLOAT));
    e.vcall(package, 0x13c, &args![actor, 0u32, minus_one, 0u32])
        .bool()
}

/// `TESDataHandler::EnumReferencesCloseToPoint` around the actor (its slot 0x1f4 twice, the radius
/// at `01030020` twice) with `callback`.
fn enum_close_references(e: &mut Engine, actor: u32, callback: u32) {
    const RADIUS: u32 = 0x0103_0020;
    let radius = x87_quiet_bits(e.mem.u32(RADIUS));
    let first = e.vcall(actor, 0x1f4, &[]).u32();
    let radius_again = x87_quiet_bits(e.mem.u32(RADIUS));
    let second = e.vcall(actor, 0x1f4, &[]).u32();
    let cell = e.mem.u32(actor.wrapping_add(0x40));
    let handler = e.mem.u32(0x011c_3f2c);
    e.call(
        0x0046_f280,
        &args![
            handler,
            cell,
            second,
            radius_again,
            first,
            radius,
            callback,
            actor
        ],
    );
}

/// "`format`" printed with the actor's name and the procedure's name when the actor is the
/// interface's target.
fn show_procedure_text(e: &mut Engine, process: u32, actor: u32, format: u32) {
    let target = typed_call!(
        interface::FAMILY,
        0x0070_3180,
        interface::interface_cpp::interface_get_target_refr(e)
    );
    if target != actor {
        return;
    }
    let package = current_package(e, process);
    let kind = u32::from_ne_bytes(procedure(e, package).to_ne_bytes());
    // The name through `00464f30`, which answers its argument.
    let procedure_name = e.mem.u32(0x0119_bcb0u32.wrapping_add(kind.wrapping_mul(4)));
    let actor_name = e.call(0x0055_d520, &args![actor]).u32();
    e.with_stack(0x130, |e, text| {
        e.call(
            0x00ec_623a,
            &args![text, format, actor_name, procedure_name],
        );
        e.call(0x0070_3c00, &args![text]);
    });
}

/// `Update`'s action 0: travel to the package location (when the actor's path is done) and look
/// around when there.
fn update_travel(e: &mut Engine, process: u32, actor: u32, location: u32) {
    let mut target = 0u32;
    if location != 0 {
        target = typed_call!(
            ai_queries::FAMILY,
            0x0067_f390,
            ai_queries::queries::location_reference(e, location)
        );
    }
    if target == 0 {
        let own = actor_process(e, actor);
        if own != 0 {
            target = e.vcall(own, 0x514, &[]).u32();
        }
    }
    if !package_test_13c(e, process, actor) && e.call(0x008b_3bb0, &args![actor]).bool() {
        // The exe works out how many seconds the hours since the process's hour take, and never
        // reads them: left out.
        let package = current_package(e, process);
        let radius = e.call(0x0067_8670, &args![package, actor, 0u32]).st0;
        let mut radius = to_float(radius).to_bits();
        let package = current_package(e, process);
        if procedure(e, package) == 5 {
            radius = 0;
        }
        let package = current_package(e, process);
        let world = e.call(0x0067_5a50, &args![package, actor]).u32();
        let package = current_package(e, process);
        let cell = e.call(0x0067_5c20, &args![package, actor]).u32();
        e.with_stack(0xc, |e, point| {
            let package = current_package(e, process);
            let coord = e.call(0x0067_5de0, &args![package, point, actor]).u32();
            e.call(0x008b_3690, &args![actor, coord, cell, world, radius, 0u32]);
        });
    }
    if process_level(e, actor_process(e, actor)) != 2 {
        return;
    }
    if !package_test_13c(e, process, actor) {
        return;
    }
    if !process_flag(e, process, 0x1bc, 0x008d_8060, 0x80) {
        let package = current_package(e, process);
        if package_flag(e, package, 0x20) {
            enum_close_references(e, actor, 0x0090_d480);
        }
        set_process_flag(e, process, 0x1c0, 0x008d_8080, 0x80, 1);
    }
    let package = current_package(e, process);
    if package_flag(e, package, 0x100) {
        enum_close_references(e, actor, 0x0090_d570);
    }
    let face = (target != 0 && e.mem.u32(target.wrapping_add(0x20)) == e.mem.u32(0x011c_a248)) || {
        let package = current_package(e, process);
        package_location(e, package) != 0 && {
            let package = current_package(e, process);
            let location = package_location(e, package);
            e.mem.i8(location) == 3
        }
    };
    if face {
        let heading = if target != 0 {
            let point = target.wrapping_add(0x24);
            x87_quiet_bits(e.mem.u32(point.wrapping_add(8)))
        } else {
            e.with_stack(0xc, |e, out| {
                let point = e.vcall(actor, 0x16c, &args![out]).u32();
                x87_quiet_bits(e.mem.u32(point.wrapping_add(8)))
            })
        };
        e.vcall(actor, 0x2c4, &args![heading]);
    }
    low_process::add_to_procedure_index_running_slot(e, process, actor, 1u32);
}

/// `Update`'s action 0x31: use the target (+0x40) when it is an actor (its slot 0x100), when the
/// package's `00672dd0` gives nothing or its slot 0x140 agrees.
fn update_use_object(e: &mut Engine, process: u32, actor: u32) {
    let held = e.mem.u32(process.wrapping_add(TARGET));
    if held == 0 || !e.vcall(held, 0x100, &[]).bool() {
        return;
    }
    let object = e.mem.u32(process.wrapping_add(TARGET));
    if object == 0 {
        return;
    }
    let package = current_package(e, process);
    if e.call(0x0067_2dd0, &args![package]).u32() != 0 {
        let package = current_package(e, process);
        let minus_one = x87_quiet_bits(e.mem.u32(base_process::MINUS_ONE_FLOAT));
        if !e
            .vcall(package, 0x140, &args![object, actor, 0u32, minus_one, 1u32])
            .bool()
        {
            return;
        }
    }
    e.vcall(process, 0x7c8, &args![actor, 1u32]);
}

/// `Update`'s action 0x11: follow the target (+0x40, found through slot 0x8c when missing) unless it
/// is the player.
fn update_follow(e: &mut Engine, process: u32, actor: u32, package: u32) {
    if e.mem.u32(process.wrapping_add(TARGET)) == 0 {
        e.vcall(process, 0x8c, &args![actor]);
    }
    let object = e.mem.u32(process.wrapping_add(TARGET));
    if object != 0 && object != e.mem.u32(PLAYER) {
        e.vcall(process, 0xc8, &args![actor, package, 1u32]);
    }
    low_process::add_to_procedure_index_running_slot(e, process, actor, 1u32);
}

/// The process's slot 0x288 with (actor, 1): the procedure done.
fn procedure_done(e: &mut Engine, process: u32, actor: u32) {
    low_process::add_to_procedure_index_running_slot(e, process, actor, 1u32);
}

/// `Update`'s action 0xD: reach the target (+0x40).
fn update_reach_object(e: &mut Engine, process: u32, actor: u32, package: u32) {
    if e.mem.u32(process.wrapping_add(TARGET)) == 0 {
        e.vcall(process, 0x8c, &args![actor]);
    }
    let object = e.mem.u32(process.wrapping_add(TARGET));
    if object == 0 || e.mem.u32(object.wrapping_add(8)) & 0x20 != 0 {
        procedure_done(e, process, actor);
        return;
    }
    if e.mem.u32(package.wrapping_add(0x1c)) & 0x1_0000 != 0 {
        let player_character = e.mem.u32(PLAYER);
        if !e.call(0x0096_2720, &args![player_character, actor]).bool() {
            let player_character = e.mem.u32(PLAYER);
            let count = e.call(0x0096_2620, &args![player_character]).i32();
            // The setting at `011cdad0`'s value (`0043d4d0`: +4, the setting is not null).
            if count < e.mem.i32(0x011c_dad4) {
                e.call(0x0067_4f30, &args![package, 0u32]);
            }
        }
        return;
    }
    if procedure(e, package) == 0x1c {
        procedure_done(e, process, actor);
        return;
    }
    if procedure(e, package) == 1 {
        let data = e.mem.u32(package.wrapping_add(0x30));
        let reach = e.mem.i32(data.wrapping_add(8));
        let reach = int_to_float(reach);
        let object = e.mem.u32(process.wrapping_add(TARGET));
        let distance = e.call(0x0057_23b0, &args![actor, object, 0u32, 0u32]).st0;
        // `FCOMPP`: no call when the reach is below the distance or they are unordered.
        if distance <= f64::from(reach) {
            procedure_done(e, process, actor);
        }
        return;
    }
    // The package's own distance (200 when not above 0) is worked out and never read: left out,
    // but the current package is still asked for (its slot may be any body).
    current_package(e, process);
    let zero = e.mem.f64(base_process::ZERO_DOUBLE);
    let mut reach = 0u32;
    // The setting at `011cdbac` (an interior's) or `011cd82c` (any other cell's): +4, through
    // `00403e20`.
    if 0.0 == zero {
        let cell = e.mem.u32(actor.wrapping_add(0x40));
        if cell != 0 && e.mem.u8(cell.wrapping_add(0x24)) & 1 != 0 {
            reach = x87_quiet_bits(e.mem.u32(0x011c_dbb0));
        } else {
            reach = x87_quiet_bits(e.mem.u32(0x011c_d830));
        }
    }
    let object = e.mem.u32(process.wrapping_add(TARGET));
    let distance = e.call(0x0057_23b0, &args![actor, object, 0u32, 0u32]).st0;
    let two = e.mem.f64(0x0101_1590);
    // `FDIV qword` at the thread's precision.
    let half = if float_precision() {
        f64::from(f32::load(reach) / f32::narrow(two))
    } else {
        f64::load(reach) / two
    };
    if distance <= half {
        procedure_done(e, process, actor);
    }
}

/// `Update`'s switch on the procedure action (the jump tables at `0092e3c8` and `0092e34c`).
fn update_action(
    e: &mut Engine,
    process: u32,
    actor: u32,
    package: u32,
    location: u32,
    action: u32,
) {
    let one = |e: &mut Engine, slot: u32| {
        e.vcall(process, slot, &args![actor]);
    };
    match action {
        0 => update_travel(e, process, actor, location),
        1 => {
            let current = current_package(e, process);
            if package_kind(e, current) == 0x1a {
                low_process::add_to_procedure_index_running_slot(e, process, actor, 0xffff_ffffu32);
            }
        }
        2 => {
            e.vcall(process, 0x7c8, &args![actor, 1u32]);
        }
        3 => one(e, 0x7d0),
        4 => show_procedure_text(e, process, actor, 0x0108_8f2c),
        5 => show_procedure_text(e, process, actor, 0x0108_8f14),
        6 => {
            e.vcall(process, 0x298, &args![actor, 1u32, 0x101u32, 0u32]);
        }
        7 => one(e, 0x7b4),
        8 => one(e, 0x7c4),
        0xa => one(e, 0x2a0),
        0xc => {
            e.call(0x0088_1680, &args![actor, 0u32]);
        }
        0xd => update_reach_object(e, process, actor, package),
        0xe => {
            e.vcall(process, 0x7c8, &args![actor, 0u32]);
        }
        0xf => {
            e.vcall(process, 0x29c, &args![actor, 0u32, 1u32, 0x101u32]);
        }
        0x11 => update_follow(e, process, actor, package),
        0x1a => one(e, 0x808),
        0x1b => one(e, 0x7cc),
        0x1c => one(e, 0x7d8),
        0x1e => one(e, 0x7d4),
        0x21 => one(e, 0x7e0),
        0x22 => one(e, 0x7e4),
        0x26 => one(e, 0x800),
        0x27 => one(e, 0x7e8),
        0x29 => {
            low_process::add_to_procedure_index_running_slot(e, process, actor, 2u32);
        }
        0x2a => one(e, 0x7ec),
        0x2c => one(e, 0x7fc),
        0x2f => one(e, 0x7f4),
        0x31 => update_use_object(e, process, actor),
        0x32 => one(e, 0x7dc),
        0x33 => one(e, 0x7f0),
        _ => {}
    }
}

/// `Update` after the action, for a middle-low process (level 2) whose action is 0x36 (the package is done).
/// Returns false when `Update` stops right there (the package's slot 0x13c turned the actor away).
fn update_finish_package(e: &mut Engine, process: u32, actor: u32, package: &mut u32) -> bool {
    if !process_flag(e, process, 0x5a4, 0x008d_8000, 0x20) {
        let extra = actor.wrapping_add(0x44);
        let current = current_package(e, process);
        e.call(0x005a_c750, &args![current, extra, 0x400u32]);
        let current = current_package(e, process);
        low_process::package_done_slot(e, process, actor, current);
        set_process_flag(e, process, 0x5a8, 0x008d_8020, 0x20, 1);
    }
    set_process_flag(e, process, 0x1c8, 0x008d_7ee0, 0x04, 0);
    if current_package(e, process) != 0 {
        let current = current_package(e, process);
        if package_kind(e, current) == 0 && !package_test_13c(e, process, actor) {
            low_process::add_to_procedure_index_running_slot(e, process, actor, 0xffff_ffffu32);
            return false;
        }
    }
    if current_package(e, process) != 0 {
        let current = current_package(e, process);
        if package_flag(e, current, 0x400) {
            let day = typed_call!(FAMILY, 0x0086_7d60, calendar_day(e, Ptr::new(CALENDAR))) & 0xff;
            let current = current_package(e, process);
            e.vcall(actor, 0x28c, &args![current, day]);
        }
    }
    if current_package(e, process) != 0 {
        let current = current_package(e, process);
        if package_flag(e, current, 0x800) {
            e.call(0x0088_1680, &args![actor, 0u32]);
        }
    }
    show_procedure_text(e, process, actor, 0x0108_8f00);
    let created = e.mem.u32(process.wrapping_add(CREATED_OBJECT));
    if created != 0 {
        e.call(0x007b_3fa0, &args![created, 1u32]);
    }
    e.mem.set_u32(process.wrapping_add(CREATED_OBJECT), 0);
    let list = process.wrapping_add(MADE_OBJECTS);
    while !(e.mem.u32(list.wrapping_add(4)) == 0 && e.mem.u32(list) == 0) {
        let item = e.mem.u32(list);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), item);
            if item != 0 {
                e.call(0x007b_3fa0, &args![item, 1u32]);
            }
            e.call(0x0090_5330, &args![list, slot]);
        });
    }
    if current_package(e, process) != 0 {
        let current = current_package(e, process);
        let data = current.wrapping_add(0x38);
        if e.mem.u32(data.wrapping_add(4)) == 0 {
            let old = current_package(e, process);
            check_for_new_package(e, process, actor, 1);
            *package = current_package(e, process);
            if old != current_package(e, process) {
                set_procedure_index_running(e, process, 0);
            }
        }
    }
    true
}

// Typed from 0092d050
/// `MiddleLowProcess::Update` (Xbox PDB): for an actor (its slot 0x100), a new package asked for
/// (slot 0x24) and re-evaluated when there is one; with a current package that has a type and isn't
/// waiting, the target reset and the action the table at `011a3ff0` gives for the package's type and
/// the running procedure index done (travel, sleep, eat, follow, use an object, the many single
/// slots); a middle-low process (level 2) whose action is 0x36 finishes the package. Unless the actor waits on a
/// path, the actor's process's slot 0x28 runs last (also for a non-actor).
pub fn update(e: &mut Engine, this: Ptr, actor: u32) {
    let process = this.addr();
    // `00483710` on `011e0528` first: an empty body. The calendar's hour stored and never read.
    if actor == 0 {
        return;
    }
    if !e.vcall(actor, 0x100, &[]).bool() {
        update_actor_process(e, actor);
        return;
    }
    if check_for_new_package(e, process, actor, 0) {
        e.vcall(process, 0x524, &args![actor]);
        e.call(0x008a_8f60, &args![actor]);
        e.vcall(process, 0x90, &args![actor, 0u32]);
    }
    let mut package = current_package(e, process);
    let runs = current_package(e, process) != 0
        && {
            let current = current_package(e, process);
            package_kind(e, current) != 0xffff_ffff
        }
        && !package_flag(e, package, 0x1_0000);
    if runs {
        typed_call!(FAMILY, 0x0090_a070, low_process::reset_target(e, this));
        let current = current_package(e, process);
        let location = package_location(e, current);
        let action = procedure_action(e, process);
        if action <= 0x33 {
            update_action(e, process, actor, package, location, action);
        }
        let process_type = {
            // `MobileObject::GetCurrentProcessType` (`00931850`): the level of the actor's process,
            // -1 without one.
            let own = actor_process(e, actor);
            if own != 0 {
                process_level(e, own)
            } else {
                0xffff_ffff
            }
        };
        if process_type == 2
            && current_package(e, process) != 0
            && procedure_action(e, process) == 0x36
            && !update_finish_package(e, process, actor, &mut package)
        {
            return;
        }
    }
    if !e.call(0x008b_3bf0, &args![actor]).bool() {
        update_actor_process(e, actor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_integer_becomes_the_nearest_float() {
        assert_eq!(int_to_float(16_777_217), 16_777_216.0);
        assert_eq!(int_to_float(-3), -3.0);
    }
}
