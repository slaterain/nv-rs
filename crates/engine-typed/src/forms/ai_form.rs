//! `TESAIForm`'s functions (the Xbox PDB's `tesaiform` source unit) and the two of the unit after
//! it (`run_0047f940`) that save and load its AI data into a save buffer, each as the exe does it.
//!
//! `TESAIForm` is a form component (at +0x90 of `TESNPC` and `TESCreature`, 0x20 bytes): its
//! vtable, then the `AIDATA` block (Xbox PDB) from +4: aggression (+4), confidence (+5), energy
//! (+6), responsibility (+7) and mood (+8) bytes, the service flags (+0xc), the training skill
//! (+0x10, an actor value less 0x20) and level (+0x11), the assistance (+0x12), the aggro radius
//! flag (+0x13) and radius (+0x14); then the package list (`AIPackList`, +0x18, a `BSSimpleList`
//! head). The setters mark the form that holds the component changed with change flag 8.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): the AI data's getters the compiler shares with other classes (`004f1540`
//! +4, `004f6de0` +5, `005bb4d0` +6, `0062f2f0` +8, `00705cf0` +0x13, `00825c00` +0x14,
//! `0084e3a0` +0xc, `00824060` +0x11) and this unit's own (`0047ee30` +0x12, `0047ef30` +7,
//! `0047f000` +0x10 plus 0x20, `0047efe0` the service flags' test, `0047f060` the training skill's
//! range), the package list's address (`00500940`, +0x18) and its constructor (`00680890`, both
//! words of the head zeroed), the base component's constructor (`00402d30`, whose vtable the
//! class's own replaces at once), the form type (`00401170`), the settings' value pointers
//! (`00403e20`, `00408d60`: a setting here is never null, so its value is at +4), a save
//! buffer's change flags (`00428110`, the word at +0x17, and `004280f0`, its bits) and a
//! reference's base form (`004181e0`, the word at +0x20). The compare reads the compared fields
//! once each, as words where the exe compares every byte of them. This unit's functions call each
//! other as typed calls, and so are the package list's operations ([`package_list`], this
//! family's). The vtable slot that marks the form changed is this family's typed call when it
//! holds `TESActorBase::AddChange` (`005f1630`) or `TESForm::AddChange` (`00484b60`), and a call
//! through the slot otherwise. What stays a call by address: the RTTI cast, the equip type, the
//! save/load game's and the save buffers' reads and writes.
//!
//! Floats are only moved: the five copied settings and the two package query floats pass through
//! `FLD`/`FSTP` in the exe, which quiets a signalling NaN, and the copied setting a getter answers
//! is loaded the same way ([`x87_quiet_bits`]).

use super::{actor_base, package_list, tesform, FAMILY};
use crate::slot_target;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// `__RTDynamicCast` (cdecl: object, vftable delta, source type, target type, is-reference).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_BASE_FORM_COMPONENT: u32 = 0x0118_3040;
const TYPE_TES_AI_FORM: u32 = 0x0118_6cd4;
const TES_AI_FORM_VTABLE: u32 = 0x0101_be98;
/// The component's offset in the actor base form that holds it.
const IN_ACTOR_BASE: u32 = 0x90;
/// A reference's base form (`004181e0`, in place).
const REFERENCE_BASE_FORM: u32 = 0x20;
/// A form's vtable slot that marks it changed (`AddChange`), and the AI data's change flag.
const SLOT_ADD_CHANGE: u32 = 0x48;
const CHANGE_AI_DATA: u32 = 8;

// The AI data's fields.
const AGGRESSION: u32 = 4;
const CONFIDENCE: u32 = 5;
const ENERGY: u32 = 6;
const RESPONSIBILITY: u32 = 7;
const MOOD: u32 = 8;
const SERVICE_FLAGS: u32 = 0xc;
const TRAINING_SKILL: u32 = 0x10;
const TRAINING_LEVEL: u32 = 0x11;
const ASSISTANCE: u32 = 0x12;
const AGGRO_RADIUS_FLAG: u32 = 0x13;
const AGGRO_RADIUS: u32 = 0x14;
const PACKAGE_LIST: u32 = 0x18;
/// The AI data block a save buffer holds: 0x14 bytes from +4.
const AI_DATA: u32 = 4;
const AI_DATA_SIZE: u32 = 0x14;
/// The first actor value of the skills, the training skill's base.
const FIRST_SKILL: u32 = 0x20;
/// The moods and confidences the record data keeps (below these).
const MOOD_COUNT: u8 = 8;
const CONFIDENCE_COUNT: u8 = 5;

/// `BGSEquipType::GetEquipType` (cdecl: a form).
const GET_EQUIP_TYPE: u32 = 0x0047_9430;

/// The five copied settings: once (the guard byte), each setting's float into its copy.
const SETTINGS_COPIED: u32 = 0x011c_52c0;
const COPIED_SETTINGS: [(u32, u32); 5] = [
    (0x011c_533c, 0x011c_52a8),
    (0x011c_5320, 0x011c_52ac),
    (0x011c_52fc, 0x011c_52b0),
    (0x011c_530c, 0x011c_52b4),
    (0x011c_52e4, 0x011c_52b8),
];
const COPIED_SETTING_FLOATS: u32 = 0x011c_52a8;
/// The byte setting `InitItem` checks before it resolves the packages.
const INIT_ITEM_SETTING: u32 = 0x011c_5348;
/// A setting's value: at +4 (`00403e20`, `00408d60`).
const SETTING_VALUE: u32 = 4;

/// The save/load game and its byte write and read (`ECX` = it; data, size).
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const SAVE_LOAD_WRITE: u32 = 0x0085_79b0;
const SAVE_LOAD_READ: u32 = 0x0085_79e0;
/// A save buffer's change flags (+0x17, `00428110`), its write (data, size, 0) and a load
/// buffer's read (data, size).
const BUFFER_CHANGE_FLAGS: u32 = 0x17;
const SAVE_BUFFER_WRITE: u32 = 0x0086_5e50;
const LOAD_BUFFER_READ: u32 = 0x0086_4980;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

/// `other` as a `TESAIForm` (the RTTI cast from `BaseFormComponent`), or 0.
fn as_ai_form(e: &mut Engine, other: u32) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![
            other,
            0u32,
            TYPE_BASE_FORM_COMPONENT,
            TYPE_TES_AI_FORM,
            0u32
        ],
    )
    .u32()
}

/// A byte setter's body: the byte at `offset`, then, with `notify`, the form marked changed.
fn set_byte(e: &mut Engine, this: u32, offset: u32, value: u8, notify: u8) {
    e.mem.set_u8(at(this, offset), value);
    if notify != 0 {
        typed_call!(FAMILY, 0x0047_f790, mark_changed(e, this, CHANGE_AI_DATA));
    }
}

/// The training skill's range test (`0047f060`, done in place by its callers).
fn is_training_skill(value: u32) -> bool {
    (FIRST_SKILL..FIRST_SKILL + 14).contains(&value)
}

// Typed from 0047ede0
/// `TESAIForm::ClearDataComponent` (Xbox PDB): the package list cleared.
pub fn clear_data_component(e: &mut Engine, this: u32) {
    let list = at(this, PACKAGE_LIST);
    typed_call!(FAMILY, 0x0067_e400, package_list::free(e, list));
}

// Typed from 0047ee00
/// The aggression (+4) set; with `notify`, the form marked changed.
pub fn set_aggression(e: &mut Engine, this: u32, value: u8, notify: u8) {
    set_byte(e, this, AGGRESSION, value, notify);
}

// Typed from 0047eed0
/// The confidence (+5) set; with `notify`, the form marked changed.
pub fn set_confidence(e: &mut Engine, this: u32, value: u8, notify: u8) {
    set_byte(e, this, CONFIDENCE, value, notify);
}

// Typed from 0047ef00
/// The energy (+6) set; with `notify`, the form marked changed.
pub fn set_energy(e: &mut Engine, this: u32, value: u8, notify: u8) {
    set_byte(e, this, ENERGY, value, notify);
}

// Typed from 0047ef50
/// The responsibility (+7) set; with `notify`, the form marked changed.
pub fn set_responsibility(e: &mut Engine, this: u32, value: u8, notify: u8) {
    set_byte(e, this, RESPONSIBILITY, value, notify);
}

// Typed from 0047f060
/// Whether an actor value is a skill the training skill can hold (`0x20..=0x2d`, signed).
pub fn is_skill(_e: &mut Engine, value: i32) -> bool {
    is_training_skill(u32::from_ne_bytes(value.to_ne_bytes()))
}

// Typed from 0047ee30
/// The assistance (+0x12).
pub fn assistance(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(at(this, ASSISTANCE))
}

// Typed from 0047ef30
/// The responsibility (+7).
pub fn responsibility(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(at(this, RESPONSIBILITY))
}

// Typed from 0047f790
/// The form that holds this component (`this - 0x90`; none for a null `this`) marked changed
/// with `flags` (its vtable slot 0x48: this family's `TESActorBase::AddChange`, which NPCs and
/// creatures keep, or `TESForm::AddChange` as typed calls when the slot holds one of them).
pub fn mark_changed(e: &mut Engine, this: u32, flags: u32) {
    let owner = if this == 0 {
        0
    } else {
        this.wrapping_sub(IN_ACTOR_BASE)
    };
    if owner == 0 {
        return;
    }
    match slot_target(e, owner, SLOT_ADD_CHANGE) {
        0x005f_1630 => typed_call!(FAMILY, 0x005f_1630, actor_base::add_change(e, owner, flags)),
        0x0048_4b60 => typed_call!(FAMILY, 0x0048_4b60, tesform::add_change(e, owner, flags)),
        _ => {
            e.vcall(owner, SLOT_ADD_CHANGE, &args![flags]);
        }
    }
}

// Typed from 0047ee50
/// The assistance (+0x12) set and the form marked changed.
pub fn set_assistance(e: &mut Engine, this: u32, value: u8) {
    set_byte(e, this, ASSISTANCE, value, 1);
}

// Typed from 0047ee70
/// The aggro radius flag (+0x13) set and the form marked changed.
pub fn set_aggro_radius_flag(e: &mut Engine, this: u32, value: u8) {
    set_byte(e, this, AGGRO_RADIUS_FLAG, value, 1);
}

// Typed from 0047ee90
/// The aggro radius (+0x14) set and the form marked changed.
pub fn set_aggro_radius(e: &mut Engine, this: u32, value: u32) {
    e.mem.set_u32(at(this, AGGRO_RADIUS), value);
    typed_call!(FAMILY, 0x0047_f790, mark_changed(e, this, CHANGE_AI_DATA));
}

// Typed from 0047ef80
/// The mood (+8) set and the form marked changed, for a mood below 8; a larger one changes nothing.
pub fn set_mood(e: &mut Engine, this: u32, value: u8) {
    if value < MOOD_COUNT {
        set_byte(e, this, MOOD, value, 1);
    }
}

// Typed from 0047efb0
/// The service flags (+0xc) set; with `notify`, the form marked changed.
pub fn set_service_flags(e: &mut Engine, this: u32, flags: u32, notify: u8) {
    e.mem.set_u32(at(this, SERVICE_FLAGS), flags);
    if notify != 0 {
        typed_call!(FAMILY, 0x0047_f790, mark_changed(e, this, CHANGE_AI_DATA));
    }
}

// Typed from 0047f000
/// The training skill as an actor value (+0x10, plus 0x20).
pub fn training_skill(e: &mut Engine, this: u32) -> u32 {
    u32::from(e.mem.u8(at(this, TRAINING_SKILL))) + FIRST_SKILL
}

// Typed from 0047f020
/// The training skill (+0x10) set from an actor value when it is a skill (`0047f060`, in place),
/// and the form marked changed either way.
pub fn set_training_skill(e: &mut Engine, this: u32, skill: u32) {
    if is_training_skill(skill) {
        e.mem.set_u8(
            at(this, TRAINING_SKILL),
            (skill - FIRST_SKILL).to_le_bytes()[0],
        );
    }
    typed_call!(FAMILY, 0x0047_f790, mark_changed(e, this, CHANGE_AI_DATA));
}

// Typed from 0047f090
/// The training level (+0x11) set and the form marked changed.
pub fn set_training_level(e: &mut Engine, this: u32, value: u8) {
    set_byte(e, this, TRAINING_LEVEL, value, 1);
}

// Typed from 0047ed10
/// `TESAIForm::InitializeDataComponent` (Xbox PDB): the AI data's start values (all 0 but the
/// confidence, 4); the first time (the guard byte), the five settings' floats copied.
pub fn initialize_data_component(e: &mut Engine, this: u32) {
    for (offset, value) in [
        (AGGRESSION, 0),
        (CONFIDENCE, 4),
        (ENERGY, 0),
        (RESPONSIBILITY, 0),
        (MOOD, 0),
    ] {
        e.mem.set_u8(at(this, offset), value);
    }
    e.mem.set_u32(at(this, SERVICE_FLAGS), 0);
    for offset in [
        TRAINING_SKILL,
        TRAINING_LEVEL,
        ASSISTANCE,
        AGGRO_RADIUS_FLAG,
    ] {
        e.mem.set_u8(at(this, offset), 0);
    }
    e.mem.set_u32(at(this, AGGRO_RADIUS), 0);
    if e.mem.u8(SETTINGS_COPIED) != 0 {
        return;
    }
    for (setting, copy) in COPIED_SETTINGS {
        let bits = e.mem.u32(at(setting, SETTING_VALUE));
        e.mem.set_u32(copy, x87_quiet_bits(bits));
    }
    e.mem.set_u8(SETTINGS_COPIED, 1);
}

// Typed from 0047ec40
/// `TESAIForm::TESAIForm` (Xbox PDB): the vtable (over the base component's), an empty package
/// list, then `InitializeDataComponent`. Returns `this`.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, TES_AI_FORM_VTABLE);
    e.mem.set_u32(at(this, PACKAGE_LIST), 0);
    e.mem.set_u32(at(this, PACKAGE_LIST + 4), 0);
    typed_call!(FAMILY, 0x0047_ed10, initialize_data_component(e, this));
    this
}

// Typed from 0047ecb0
/// `TESAIForm`'s destructor: its vtable, `ClearDataComponent`, then the package list's destructor.
pub fn destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, TES_AI_FORM_VTABLE);
    typed_call!(FAMILY, 0x0047_ede0, clear_data_component(e, this));
    let list = at(this, PACKAGE_LIST);
    typed_call!(FAMILY, 0x0067_e3a0, package_list::destruct(e, list));
}

// Typed from 0047f0b0
/// An `AIDATA` record (`data`: the five bytes, the service flags at +8, the training skill and
/// level at +0xc/+0xd, the assistance +0xe, the aggro radius flag +0xf and radius +0x10) set
/// through the setters in the exe's order (those with a notify flag given 1), the training bytes
/// stored directly; a confidence of 5 or more becomes 0. Nothing for a null `data`.
pub fn load_data(e: &mut Engine, this: u32, data: u32) {
    if data == 0 {
        return;
    }
    let byte = |e: &Engine, offset: u32| e.mem.u8(at(data, offset));
    let value = byte(e, 0);
    typed_call!(FAMILY, 0x0047_ee00, set_aggression(e, this, value, 1u8));
    let value = byte(e, 0xe);
    typed_call!(FAMILY, 0x0047_ee50, set_assistance(e, this, value));
    let value = byte(e, 1);
    typed_call!(FAMILY, 0x0047_eed0, set_confidence(e, this, value, 1u8));
    let value = byte(e, 2);
    typed_call!(FAMILY, 0x0047_ef00, set_energy(e, this, value, 1u8));
    let value = byte(e, 3);
    typed_call!(FAMILY, 0x0047_ef50, set_responsibility(e, this, value, 1u8));
    let flags = e.mem.u32(at(data, 8));
    typed_call!(FAMILY, 0x0047_efb0, set_service_flags(e, this, flags, 1u8));
    let value = byte(e, 4);
    typed_call!(FAMILY, 0x0047_ef80, set_mood(e, this, value));
    let value = byte(e, 0xf);
    typed_call!(FAMILY, 0x0047_ee70, set_aggro_radius_flag(e, this, value));
    let radius = e.mem.u32(at(data, 0x10));
    typed_call!(FAMILY, 0x0047_ee90, set_aggro_radius(e, this, radius));
    let value = byte(e, 0xc);
    e.mem.set_u8(at(this, TRAINING_SKILL), value);
    let value = byte(e, 0xd);
    e.mem.set_u8(at(this, TRAINING_LEVEL), value);
    if e.mem.u8(at(this, CONFIDENCE)) >= CONFIDENCE_COUNT {
        e.mem.set_u8(at(this, CONFIDENCE), 0);
    }
}

// Typed from 0047f1e0
/// `other`'s AI data set into this component through the setters, in the exe's order (those with
/// a notify flag given 1), each field of `other` read just before its setter runs.
pub fn copy_ai_data(e: &mut Engine, this: u32, other: u32) {
    let byte = |e: &Engine, offset: u32| e.mem.u8(at(other, offset));
    let value = byte(e, AGGRESSION);
    typed_call!(FAMILY, 0x0047_ee00, set_aggression(e, this, value, 1u8));
    let value = byte(e, ASSISTANCE);
    typed_call!(FAMILY, 0x0047_ee50, set_assistance(e, this, value));
    let value = byte(e, CONFIDENCE);
    typed_call!(FAMILY, 0x0047_eed0, set_confidence(e, this, value, 1u8));
    let value = byte(e, ENERGY);
    typed_call!(FAMILY, 0x0047_ef00, set_energy(e, this, value, 1u8));
    let value = byte(e, RESPONSIBILITY);
    typed_call!(FAMILY, 0x0047_ef50, set_responsibility(e, this, value, 1u8));
    let value = byte(e, MOOD);
    typed_call!(FAMILY, 0x0047_ef80, set_mood(e, this, value));
    let value = byte(e, AGGRO_RADIUS_FLAG);
    typed_call!(FAMILY, 0x0047_ee70, set_aggro_radius_flag(e, this, value));
    let radius = e.mem.u32(at(other, AGGRO_RADIUS));
    typed_call!(FAMILY, 0x0047_ee90, set_aggro_radius(e, this, radius));
    let flags = e.mem.u32(at(other, SERVICE_FLAGS));
    typed_call!(FAMILY, 0x0047_efb0, set_service_flags(e, this, flags, 1u8));
    let skill = u32::from(byte(e, TRAINING_SKILL)) + FIRST_SKILL;
    typed_call!(FAMILY, 0x0047_f020, set_training_skill(e, this, skill));
    let value = byte(e, TRAINING_LEVEL);
    typed_call!(FAMILY, 0x0047_f090, set_training_level(e, this, value));
}

// Typed from 0047f2d0
/// `other`'s package list copied into this component's.
pub fn copy_packages(e: &mut Engine, this: u32, other: u32) {
    let (list, other_list) = (at(this, PACKAGE_LIST), at(other, PACKAGE_LIST));
    typed_call!(FAMILY, 0x0067_e440, package_list::copy(e, list, other_list));
}

// Typed from 0047f190
/// `TESAIForm::CopyComponent` (Xbox PDB): when `other` is a `TESAIForm`, its AI data and its
/// packages copied.
pub fn copy_component(e: &mut Engine, this: u32, other: u32) {
    let other = as_ai_form(e, other);
    if other == 0 {
        return;
    }
    typed_call!(FAMILY, 0x0047_f1e0, copy_ai_data(e, this, other));
    typed_call!(FAMILY, 0x0047_f2d0, copy_packages(e, this, other));
}

// Typed from 0047f500
/// `package` added at the end of the package list.
pub fn add_package(e: &mut Engine, this: u32, package: u32) {
    let list = at(this, PACKAGE_LIST);
    typed_call!(
        FAMILY,
        0x0067_e420,
        package_list::add_tail(e, list, package)
    );
}

// Typed from 0047f4c0
/// `TESAIForm::InitItem` (Xbox PDB): unless the setting's byte is set, the package list resolved.
pub fn init_item(e: &mut Engine, this: u32, form: u32) {
    if e.mem.u8(at(INIT_ITEM_SETTING, SETTING_VALUE)) != 0 {
        return;
    }
    let list = at(this, PACKAGE_LIST);
    typed_call!(FAMILY, 0x0067_e560, package_list::init_item(e, list, form));
}

/// The `TESAIForm` of `reference`'s base form (the word at +0x20, `004181e0`; the component at
/// +0x90 of it), or 0 for no base form.
fn reference_ai_form(e: &mut Engine, reference: u32) -> u32 {
    let base = e.mem.u32(at(reference, REFERENCE_BASE_FORM));
    if base == 0 {
        0
    } else {
        at(base, IN_ACTOR_BASE)
    }
}

// Typed from 0047f520
/// `TESAIForm::GetCurrentPackage` (Xbox PDB, cdecl): the package list's current package for a
/// reference, from its base form's `TESAIForm`; 0 for a null reference or base form.
pub fn current_package(e: &mut Engine, reference: u32) -> u32 {
    if reference == 0 {
        return 0;
    }
    let ai = reference_ai_form(e, reference);
    if ai == 0 {
        return 0;
    }
    let list = at(ai, PACKAGE_LIST);
    typed_call!(
        FAMILY,
        0x0067_e780,
        package_list::find_current_package(e, list, reference)
    )
}

// Typed from 0047f590
/// `TESAIForm::GetMissedPackages` (Xbox PDB, cdecl: reference, a word, two floats): the package
/// list's missed packages for a reference, from its base form's `TESAIForm`, with the word and the
/// two floats (each through `FLD`/`FSTP`). Nothing for a null reference or base form.
pub fn missed_packages(e: &mut Engine, reference: u32, value: u32, first: f32, second: f32) {
    if reference == 0 {
        return;
    }
    let ai = reference_ai_form(e, reference);
    if ai == 0 {
        return;
    }
    let list = at(ai, PACKAGE_LIST);
    let first = x87_quiet_bits(first.to_bits());
    let second = f32::from_bits(x87_quiet_bits(second.to_bits()));
    typed_call!(
        FAMILY,
        0x0067_ec40,
        package_list::missed_packages(e, list, reference, value, first, second)
    );
}

// Typed from 0047eeb0
/// The copied setting float number `index`, loaded as `FLD` loads it; 0.0 for an index of 5 or
/// more (cdecl, one byte).
pub fn copied_setting(e: &mut Engine, index: u8) -> f32 {
    if index >= 5 {
        return 0.0;
    }
    let bits = e.mem.u32(COPIED_SETTING_FLOATS + 4 * u32::from(index));
    f32::from_bits(x87_quiet_bits(bits))
}

// Typed from 0047efe0
/// Whether the service flags (+0xc) have any of `flags`.
pub fn has_service(e: &mut Engine, this: u32, flags: u32) -> bool {
    e.mem.u32(at(this, SERVICE_FLAGS)) & flags != 0
}

// Typed from 0047f300
/// `TESAIForm::CompareComponent` (Xbox PDB): true when `other` is not a `TESAIForm`, when any
/// field of the AI data differs (the five bytes from +4 and the words at +0xc, +0x10 and +0x14:
/// the exe's getters, read once each), or when the package lists differ.
pub fn compare_component(e: &mut Engine, this: u32, other: u32) -> bool {
    let other = as_ai_form(e, other);
    if other == 0 {
        return true;
    }
    let differs =
        |e: &Engine, offset: u32| e.mem.u32(at(this, offset)) != e.mem.u32(at(other, offset));
    if differs(e, AGGRESSION)
        || e.mem.u8(at(this, MOOD)) != e.mem.u8(at(other, MOOD))
        || differs(e, SERVICE_FLAGS)
        || differs(e, TRAINING_SKILL)
        || differs(e, AGGRO_RADIUS)
    {
        return true;
    }
    let (list, other_list) = (at(this, PACKAGE_LIST), at(other, PACKAGE_LIST));
    typed_call!(
        FAMILY,
        0x0067_e4a0,
        package_list::lists_differ(e, list, other_list)
    )
}

// Typed from 0047f600
/// `TESAIForm::SellBuysItem` (Xbox PDB): whether the service flags cover the item's form type:
/// type 0x18 needs flag 2, 0x19 flag 8, 0x1f, 0x32 and 0x67 flag 0x400, 0x28 and 0x29 flag 1; for
/// type 0x2f the flag follows the item's equip type (0xa flag 0x20, 0xb 0x40, 0xc 0x10, 0xd 4).
/// Any other type or equip type: false.
pub fn sells_buys_item(e: &mut Engine, this: u32, item: u32) -> bool {
    let kind = u32::from(e.mem.u8(at(item, 4)));
    let flag = match kind {
        0x18 => 2,
        0x19 => 8,
        0x1f | 0x32 | 0x67 => 0x400,
        0x28 | 0x29 => 1,
        0x2f => match e.call(GET_EQUIP_TYPE, &args![item]).u32() {
            0xa => 0x20,
            0xb => 0x40,
            0xc => 0x10,
            0xd => 4,
            _ => return false,
        },
        _ => return false,
    };
    has_service(e, this, flag)
}

// Typed from 0047f7e0
/// `TESAIForm::GetSaveSize` (Xbox PDB): 5 with the AI data's change flag (one per saved byte),
/// else 0.
pub fn save_size(_e: &mut Engine, _this: u32, flags: u32) -> u16 {
    if flags & CHANGE_AI_DATA != 0 {
        5
    } else {
        0
    }
}

// Typed from 0047f840
/// `TESAIForm::SaveGame` (Xbox PDB): with the AI data's change flag, the five bytes from +4
/// written one at a time to the save/load game.
pub fn save_game(e: &mut Engine, this: u32, flags: u32) {
    if flags & CHANGE_AI_DATA == 0 {
        return;
    }
    for offset in AGGRESSION..=MOOD {
        let game = e.mem.u32(SAVE_LOAD_GAME);
        e.call(SAVE_LOAD_WRITE, &args![game, at(this, offset), 1u32]);
    }
}

// Typed from 0047f8c0
/// `TESAIForm::LoadGame` (Xbox PDB; the change flags and a word it does not read): with the AI
/// data's change flag, the five bytes from +4 read one at a time from the save/load game.
pub fn load_game(e: &mut Engine, this: u32, flags: u32, _unused: u32) {
    if flags & CHANGE_AI_DATA == 0 {
        return;
    }
    for offset in AGGRESSION..=MOOD {
        let game = e.mem.u32(SAVE_LOAD_GAME);
        e.call(SAVE_LOAD_READ, &args![game, at(this, offset), 1u32]);
    }
}

/// Whether a save/load buffer's change flags have the AI data's flag (`00428110`, `004280f0`).
fn buffer_has_ai_data(e: &mut Engine, buffer: u32) -> bool {
    e.mem.u32(at(buffer, BUFFER_CHANGE_FLAGS)) & CHANGE_AI_DATA != 0
}

// Typed from 0047f940
/// The AI data saved into a save buffer: with the buffer's AI data change flag, the 0x14 bytes
/// from +4 written.
pub fn save_to_buffer(e: &mut Engine, this: u32, buffer: u32) {
    if buffer_has_ai_data(e, buffer) {
        e.call(
            SAVE_BUFFER_WRITE,
            &args![buffer, at(this, AI_DATA), AI_DATA_SIZE, 0u32],
        );
    }
}

// Typed from 0047f980
/// The AI data loaded from a load buffer: with the buffer's AI data change flag, the 0x14 bytes
/// from +4 read.
pub fn load_from_buffer(e: &mut Engine, this: u32, buffer: u32) {
    if buffer_has_ai_data(e, buffer) {
        e.call(
            LOAD_BUFFER_READ,
            &args![buffer, at(this, AI_DATA), AI_DATA_SIZE],
        );
    }
}
