//! `TESActorBase`'s functions (the Xbox PDB's `tesactorbase` source unit), each as the exe does it.
//!
//! `TESActorBase` is the base of `TESNPC` (form type 0x2a) and `TESCreature` (0x2b): the form,
//! then its components: the actor base data at +0x30 (its base flags at +0x34, change flags at
//! +0x58), the spell list at +0x7c, the AI form at +0x90 (its statistics bytes from +0x94), the
//! health form at +0xb0 (the health at +0xb4), the attributes at +0xb8, the full name at +0xd0
//! (its string at +0xd4) and the actor value owner at +0x100. The flag getters test the base flags
//! for one form type; the actor value getter and setter route a code to the component that holds
//! it; the save functions write and read the components in the exe's order.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds (checked
//! in the exe): the form type (`00401170`), the base flags' test (`00461580`), the form flags'
//! getter and setter (`0044ddc0`, `00403550`), the form id (`0084e3a0`), the created-id test
//! (`00469860`: `0xff000000` and up), the actor base data's change flags (`0047e340`, `0047e360`)
//! and speed multiplier (`008f21d0`), the AI form's statistics bytes (`004f1540`, `004f6de0`,
//! `005bb4d0`, `0047ef30`, `0062f2f0`, `0047ee30`), the attribute bytes (`00480160`), the health's
//! setter (`006ecd40`), the full name's text (`00408da0`: the string, or the empty one), a
//! reference's base object and process (`004181e0` and `007af430`, `008d8520`), the word after a
//! form's node (`00726070`), the save/load game's flag 2 (`0042ce10`), buffer position
//! (`00825c00`), current save and load entries (`004fd3e0`, `004fd3c0`) and version byte
//! (`008df040`), a save buffer's change flags (`00428110`, `004280f0`), a setting's value
//! (`00408d60`), the change flags' word (`008c71b0`) and the iterator's base constructor
//! (`005d8b10`). `TESForm`'s, `TESActorBaseData`'s and `TESAttributes`' functions are this
//! family's typed calls, `memset` and `_ftol2` the C runtime family's; a word the exe reads again
//! with nothing between that could change it is read once, and so is the save/load game
//! (`011de45c`), which the exe reads again after calls by address but which is set once at start-up
//! and stays the same through a save or a load. Every virtual call goes through its
//! slot, and the rest (the RTTI casts, the AI form's and spell list's functions, the save/load
//! game's change tests, the combat formulas, the logger) are calls by address.
//!
//! Floats: the AI and the loading threads ask the desirability too, so it is the exe's x87
//! arithmetic as the translation does it (`x87_add` under the running thread's control word, each
//! result stored as the `f32` the exe stores); the karma set from an actor value is the integer
//! loaded exactly and stored as an `f32`; the karma read as an actor value is made an integer by
//! `_ftol2`, the C runtime family's.

use super::{actor_base_data, attributes, tesform, FAMILY};
use crate::crt;
use engine_core::typed::leaf;
use engine_core::x87::{fistp_chop, x87_add, x87_quiet, x87_store32};
use engine_core::{args, typed_call, Engine, Ptr};

const NPC_TYPE: u8 = 0x2a;
const CREATURE_TYPE: u8 = 0x2b;
/// The components, from the actor base.
const ACTOR_BASE_DATA: u32 = 0x30;
const SPELL_LIST: u32 = 0x7c;
const AI_FORM: u32 = 0x90;
const HEALTH_FORM: u32 = 0xb0;
const ATTRIBUTES: u32 = 0xb8;
const FULL_NAME: u32 = 0xd0;
/// The actor value owner's distance from the actor base, and from it the AI form's, the
/// attributes' and the actor base data's.
const OWNER_TO_AI_FORM: u32 = 0x70;
const OWNER_TO_ATTRIBUTES: u32 = 0x48;
const OWNER_TO_DATA: u32 = 0xd0;
const OWNER_TO_BASE: u32 = 0x100;
/// The actor base data's base flags (+4), speed multiplier (+0x12) and change flags (+0x28).
const DATA_BASE_FLAGS: u32 = 4;
const DATA_SPEED_MULT: u32 = 0x12;
const DATA_CHANGE_FLAGS: u32 = 0x28;
/// The form's flags (+8) and type byte (+4).
const FORM_FLAGS: u32 = 8;
const FORM_TYPE: u32 = 4;
const STARTS_DEAD: u32 = 0x0008_0000;
/// The vtable slots: the health form's health, the actor base data's karma (`f32` in `ST0`) and
/// 16-bit statistic, a form's name, a reference's test, a process's set.
const SLOT_HEALTH: u32 = 0x10;
const SLOT_DATA_STATISTIC: u32 = 0x60;
const SLOT_DATA_KARMA: u32 = 0x64;
const SLOT_FORM_NAME: u32 = 0x130;
const SLOT_REFERENCE_TEST: u32 = 0x22c;
const SLOT_PROCESS_SET: u32 = 0x468;
/// A reference's base object (+0x20) and process (+0x68).
const REFERENCE_BASE_OBJECT: u32 = 0x20;
const REFERENCE_PROCESS: u32 = 0x68;
/// The AI form's statistics bytes: actor value codes 0 to 4 at +4 to +8, and code 0x39's.
const AI_FORM_BYTES: u32 = 4;
const AI_FORM_BYTE_39: u32 = 0x12;
const AI_FORM_SET_0: u32 = 0x0047_ee00;
const AI_FORM_SET_1: u32 = 0x0047_eed0;
const AI_FORM_SET_2: u32 = 0x0047_ef00;
const AI_FORM_SET_3: u32 = 0x0047_ef50;
const AI_FORM_SET_4: u32 = 0x0047_ef80;
const PLAYER: u32 = 0x011d_ea3c;
const BGS_SAVE_LOAD_GAME: u32 = 0x011d_df38;
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const CREATED_IDS: u32 = 0xff00_0000;
const SAVE_LOAD_IS_CHANGED: u32 = 0x0084_a620;
const SAVE_LOAD_IS_FORM_CHANGED: u32 = 0x0084_a740;
const SAVE_LOAD_GET_CHANGE: u32 = 0x0084_a6d0;
/// `BGSSaveLoadGame`'s flags (+0x244) and its bit 2.
const SAVE_LOAD_FLAGS: u32 = 0x244;
/// `TESSaveLoadGame`: its buffer position, current save and load entries and version byte.
const SAVE_LOAD_POSITION: u32 = 0x14;
const SAVE_LOAD_SAVE_ENTRY: u32 = 0x88;
const SAVE_LOAD_LOAD_ENTRY: u32 = 0x84;
const SAVE_LOAD_VERSION: u32 = 0x80;
const USE_SAVE_GAME_BLOCKS: u32 = 0x0086_2110;
const SAVE_BYTES: u32 = 0x0085_79b0;
const LOAD_BYTES: u32 = 0x0085_79e0;
/// The save version from which the form's own flags are saved.
const FORM_FLAGS_VERSION: u8 = 0x6d;
/// The boolean setting that turns on the save size reports (its value at +4).
const REPORT_SAVE_SIZES: u32 = 0x011d_e4e8;
/// A save/load buffer's change flags: the word at +0x17 (`00428110`); the name's flag.
const BUFFER_CHANGE_FLAGS: u32 = 0x17;
const CHANGE_NAME: u32 = 0x20;
const MESSAGE_LOG: u32 = 0x005b_5e40;
const SOURCE_FILE: u32 = 0x0104_7628;
const TAG_KOLB: u32 = 0x424c_4f4b;
const TEXT_SIZE_FORM: u32 = 0x0101_2cb0;
const TEXT_SIZE_PLAIN: u32 = 0x0101_2c78;
const TEXT_SAVE_FORM: u32 = 0x0101_53a0;
const TEXT_SAVE_PLAIN: u32 = 0x0101_536c;
const TEXT_BLOCK_TOO_BIG: u32 = 0x0101_5318;
const TEXT_TAG_FORM: u32 = 0x0101_5718;
const TEXT_TAG_PLAIN: u32 = 0x0101_56a8;
const TEXT_OVERRUN_FORM: u32 = 0x0101_5588;
const TEXT_UNDERRUN_FORM: u32 = 0x0101_5500;
const TEXT_OVERRUN_PLAIN: u32 = 0x0101_54a0;
const TEXT_UNDERRUN_PLAIN: u32 = 0x0101_5440;
const EMPTY_STRING: u32 = 0x0101_1584;
const STRING_LENGTH: u32 = 0x0044_a670;
const SET_NAME: u32 = 0x0048_9100;
const SAVE_STRING: u32 = 0x0086_5e70;
const LOAD_STRING: u32 = 0x0086_49a0;
const NAME_BUFFER_SIZE: u32 = 0x104;
/// The components' save functions the actor base's call by address.
const DATA_SAVE_TO_BUFFER: u32 = 0x0047_ea30;
const DATA_LOAD_FROM_BUFFER: u32 = 0x0047_ea70;
const SPELLS_SAVE_TO_BUFFER: u32 = 0x0048_e040;
const SPELLS_LOAD_FROM_BUFFER: u32 = 0x0048_e140;
const AI_SAVE_TO_BUFFER: u32 = 0x0047_f940;
const AI_LOAD_FROM_BUFFER: u32 = 0x0047_f980;
const SPELLS_SAVE_SIZE: u32 = 0x0048_d8f0;
const AI_SAVE_SIZE: u32 = 0x0047_f7e0;
const SPELLS_SAVE_GAME: u32 = 0x0048_da20;
const AI_SAVE_GAME: u32 = 0x0047_f840;
const SPELLS_LOAD_GAME: u32 = 0x0048_dca0;
const AI_LOAD_GAME: u32 = 0x0047_f8c0;
/// `GetDesirability`'s casts and callees.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_AMMO: u32 = 0x0118_40c4;
const TYPE_TES_OBJECT_WEAP: u32 = 0x0118_3998;
const TYPE_TES_OBJECT_CLOT: u32 = 0x0118_6fcc;
const TYPE_TES_OBJECT_ARMO: u32 = 0x0118_3a34;
const GET_FORM_HEALTH: u32 = 0x0048_73d0;
const CALC_WEAPON_DAMAGE: u32 = 0x0064_4ce0;
const CLOTHING_RATING: u32 = 0x0064_7cf0;
const CALC_ARMOR_RATING: u32 = 0x0064_6360;
const ARMOR_BASE_RATING: u32 = 0x004b_e060;
const ARMOR_EXTRA_RATING: u32 = 0x004b_e180;
const KEY_DESIRABILITY: u32 = 0x0101_6410;
/// The value the global picks over the default for the player (`005f0f80`).
const PLAYER_VALUE: u32 = 0x011c_aff0;
const DEFAULT_VALUE: u32 = 0x011c_afec;
/// `Reset3DMobIterator`: its vtable (slot 0 `005f2170`), the base's (`005d8b10`), the list it
/// walks and the walk.
const RESET_3D_ITERATOR_VTABLE: u32 = 0x0104_7670;
const ITERATED_LIST: u32 = 0x011e_0e80;
const ITERATE: u32 = 0x0096_bbc0;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

/// The low half of a word (the exe's `u16` store of a value it means to cut).
#[allow(clippy::cast_possible_truncation)]
fn low_half(word: u32) -> u16 {
    word as u16
}

/// The low byte of a word.
#[allow(clippy::cast_possible_truncation)]
fn low_byte(word: u32) -> u8 {
    word as u8
}

/// The form type byte (`00401170`).
fn form_type(e: &mut Engine, form: u32) -> u8 {
    e.mem.u8(at(form, FORM_TYPE))
}

/// The actor base data's base flags (`00461580` tests them).
fn base_flags(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(at(this, ACTOR_BASE_DATA + DATA_BASE_FLAGS))
}

/// Whether the form is of type `kind` and its base flags have one of `mask`'s bits.
fn flag_of_type(e: &mut Engine, this: u32, kind: u8, mask: u32) -> bool {
    form_type(e, this) == kind && base_flags(e, this) & mask != 0
}

/// Whether the form is not a creature, or a creature whose base flags have one of `mask`'s bits.
fn not_creature_or(e: &mut Engine, this: u32, mask: u32) -> bool {
    form_type(e, this) != CREATURE_TYPE || base_flags(e, this) & mask != 0
}

/// The attribute byte an actor value code names (`00480160`'s `AL`).
fn attribute_byte(e: &mut Engine, attributes: u32, code: u32) -> u32 {
    u32::from(e.mem.u8(attributes.wrapping_add(code).wrapping_sub(1)))
}

/// `00480160`'s whole `EAX`: the attribute byte under `attributes + code`'s upper three bytes.
fn attribute_word(e: &mut Engine, attributes: u32, code: u32) -> u32 {
    (attributes.wrapping_add(code) & 0xffff_ff00) | attribute_byte(e, attributes, code)
}

/// The player's base form, when there is a player reference (`004181e0`: its base object).
fn player_base_form(e: &mut Engine) -> u32 {
    let player = e.mem.u32(PLAYER);
    e.mem.u32(at(player, REFERENCE_BASE_OBJECT))
}

/// The full name's text (`00408da0`): its string, or the empty one.
fn name_text(e: &mut Engine, this: u32) -> u32 {
    let text = e.mem.u32(at(this, FULL_NAME + 4));
    if text != 0 {
        text
    } else {
        EMPTY_STRING
    }
}

/// Whether the save size reports are on (the setting's value byte).
fn reports_save_sizes(e: &mut Engine) -> bool {
    e.mem.u8(at(REPORT_SAVE_SIZES, 4)) != 0
}

/// Whether a save/load buffer's change flags have the name's flag.
fn buffer_has_name(e: &mut Engine, buffer: u32) -> bool {
    e.mem.u32(at(buffer, BUFFER_CHANGE_FLAGS)) & CHANGE_NAME != 0
}

/// Whether the save uses blocks (`TESSaveLoadGame::UseSaveGameBlocks`).
fn uses_blocks(e: &mut Engine, save_load: u32) -> bool {
    e.call(USE_SAVE_GAME_BLOCKS, &args![save_load]).u8() != 0
}

/// The form a save entry names (by its id at +0) and that form's name (slot 0x130).
fn entry_form_name(e: &mut Engine, entry: u32) -> (u32, u32) {
    let id = e.mem.u32(entry);
    let form = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id));
    (form, e.vcall(form, SLOT_FORM_NAME, &args![]).u32())
}

/// A save size report (`Error`): with a current save entry its form id, name and flags (+5).
fn report_size(e: &mut Engine, save_load: u32, amount: u32, line: u32, texts: (u32, u32)) {
    let entry = e.mem.u32(at(save_load, SAVE_LOAD_SAVE_ENTRY));
    if entry != 0 {
        let (_, name) = entry_form_name(e, entry);
        let id = e.mem.u32(entry);
        let flags = e.mem.u32(at(entry, 5));
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words texts.0, amount, id, name, flags, line, SOURCE_FILE);
    } else {
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words texts.1, amount, line, SOURCE_FILE);
    }
}

// Typed from 005f0c40
/// For a creature, base flag 0x20; false for any other form.
pub fn creature_flag_20(e: &mut Engine, this: u32) -> bool {
    flag_of_type(e, this, CREATURE_TYPE, 0x20)
}

// Typed from 005f0c80
/// `TESActorBase::IsImmobile` (Xbox PDB): for a creature, base flag 0x800000.
pub fn is_immobile(e: &mut Engine, this: u32) -> bool {
    flag_of_type(e, this, CREATURE_TYPE, 0x0080_0000)
}

// Typed from 005f0cc0
/// `TESActorBase::GetSex` (Xbox PDB): -1 for a form that is not an NPC; else base flag 1 (1 or 0).
pub fn get_sex(e: &mut Engine, this: u32) -> i32 {
    if form_type(e, this) != NPC_TYPE {
        return -1;
    }
    i32::from(base_flags(e, this) & 1 != 0)
}

// Typed from 005f11f0
/// The attributes' actor value 5 (`00480160` with 5: the byte, `EAX` as it leaves it).
pub fn attribute_5(e: &mut Engine, this: u32) -> u32 {
    attribute_word(e, this, 5)
}

// Typed from 005f1210
/// The attributes' actor value 6.
pub fn attribute_6(e: &mut Engine, this: u32) -> u32 {
    attribute_word(e, this, 6)
}

// Typed from 005f1230
/// The attributes' actor value 7.
pub fn attribute_7(e: &mut Engine, this: u32) -> u32 {
    attribute_word(e, this, 7)
}

// Typed from 005f1250
/// The attributes' actor value 8.
pub fn attribute_8(e: &mut Engine, this: u32) -> u32 {
    attribute_word(e, this, 8)
}

// Typed from 005f1270
/// The attributes' actor value 9.
pub fn attribute_9(e: &mut Engine, this: u32) -> u32 {
    attribute_word(e, this, 9)
}

// Typed from 005f1290
/// The attributes' actor value 0xa.
pub fn attribute_a(e: &mut Engine, this: u32) -> u32 {
    attribute_word(e, this, 0xa)
}

// Typed from 005f12b0
/// The attributes' actor value 0xb.
pub fn attribute_b(e: &mut Engine, this: u32) -> u32 {
    attribute_word(e, this, 0xb)
}

/// The attributes' actor value `code` set to `value`, the form marked changed
/// (`TESAttributes::SetAttributeValue` with 1).
fn set_attribute(e: &mut Engine, this: u32, code: u32, value: u8) {
    typed_call!(
        FAMILY,
        0x0048_0180,
        attributes::set_attribute_value(e, this, code, value, 1)
    );
}

// Typed from 005f14b0
/// The attributes' actor value 5 set (the form marked changed).
pub fn set_attribute_5(e: &mut Engine, this: u32, value: u8) {
    set_attribute(e, this, 5, value);
}

// Typed from 005f14d0
/// The attributes' actor value 6 set.
pub fn set_attribute_6(e: &mut Engine, this: u32, value: u8) {
    set_attribute(e, this, 6, value);
}

// Typed from 005f14f0
/// The attributes' actor value 7 set.
pub fn set_attribute_7(e: &mut Engine, this: u32, value: u8) {
    set_attribute(e, this, 7, value);
}

// Typed from 005f1510
/// The attributes' actor value 8 set.
pub fn set_attribute_8(e: &mut Engine, this: u32, value: u8) {
    set_attribute(e, this, 8, value);
}

// Typed from 005f1530
/// The attributes' actor value 9 set.
pub fn set_attribute_9(e: &mut Engine, this: u32, value: u8) {
    set_attribute(e, this, 9, value);
}

// Typed from 005f1550
/// The attributes' actor value 0xa set.
pub fn set_attribute_a(e: &mut Engine, this: u32, value: u8) {
    set_attribute(e, this, 0xa, value);
}

// Typed from 005f1570
/// The attributes' actor value 0xb set.
pub fn set_attribute_b(e: &mut Engine, this: u32, value: u8) {
    set_attribute(e, this, 0xb, value);
}

// Typed from 005f0b00
/// `TESActorBase::GetHealth` (Xbox PDB): the health form's health (its slot 0x10).
pub fn get_health(e: &mut Engine, this: u32) -> u32 {
    e.vcall(at(this, HEALTH_FORM), SLOT_HEALTH, &args![]).u32()
}

// Typed from 005f0d00
/// For an NPC, base flag 0x10; false for any other form.
pub fn npc_flag_10(e: &mut Engine, this: u32) -> bool {
    flag_of_type(e, this, NPC_TYPE, 0x10)
}

// Typed from 005f0d30
/// For an NPC, base flag 0x10 set to `value` (`TESActorBaseData::SetFlagBit`, the form marked
/// changed); nothing for any other form.
pub fn set_npc_flag_10(e: &mut Engine, this: u32, value: u8) {
    if form_type(e, this) == NPC_TYPE {
        typed_call!(
            FAMILY,
            0x0047_dd50,
            actor_base_data::set_flag_bit(e, at(this, ACTOR_BASE_DATA), 0x10, value, 1)
        );
    }
}

// Typed from 005f0f80
/// The word at `011caff0` when it is not 0 and this is the player's base form, else the one at
/// `011cafec`.
pub fn player_or_default_value(e: &mut Engine, this: u32) -> u32 {
    let value = e.mem.u32(PLAYER_VALUE);
    if value != 0 && player_base_form(e) == this {
        return value;
    }
    e.mem.u32(DEFAULT_VALUE)
}

// Typed from 00565210
/// `TESActorBase::SetStartsDead` (Xbox PDB): the form flag 0x80000 set or cleared.
pub fn set_starts_dead(e: &mut Engine, this: u32, value: u8) {
    let flags = e.mem.u32(at(this, FORM_FLAGS));
    let flags = if value != 0 {
        flags | STARTS_DEAD
    } else {
        flags & !STARTS_DEAD
    };
    e.mem.set_u32(at(this, FORM_FLAGS), flags);
}

// Typed from 005f0b50
/// True for a form that is not a creature; for a creature, whether base flag 0x10 or 1 is set.
pub fn creature_flag_10_or_1(e: &mut Engine, this: u32) -> bool {
    not_creature_or(e, this, 0x11)
}

// Typed from 005f0bf0
/// True for a form that is not a creature; for a creature, whether base flag 0x40 or 1 is set.
pub fn creature_flag_40_or_1(e: &mut Engine, this: u32) -> bool {
    not_creature_or(e, this, 0x41)
}

// Typed from 005f1630
/// The change `flags` added: for a created form (id `0xff000000` and up), to the actor base data's
/// change flags when the save/load game says the form is changed (`0084a620`); for any other form,
/// as a change of the form (`TESForm::AddChange`).
pub fn add_change(e: &mut Engine, this: u32, flags: u32) {
    let id = e.mem.u32(at(this, 0xc));
    if id >= CREATED_IDS {
        let save_load = e.mem.u32(BGS_SAVE_LOAD_GAME);
        if e.call(SAVE_LOAD_IS_CHANGED, &args![save_load, this, 0u32])
            .bool()
        {
            let change = at(this, ACTOR_BASE_DATA + DATA_CHANGE_FLAGS);
            let value = e.mem.u32(change);
            e.mem.set_u32(change, value | flags);
        }
    } else {
        typed_call!(FAMILY, 0x0048_4b60, tesform::add_change(e, this, flags));
    }
}

// Typed from 005f1690
/// The change `flags` removed: for a created form the save/load game says is changed
/// (`0084a740`), from the actor base data's change flags; else from the form's
/// (`00484bc0`).
pub fn remove_change(e: &mut Engine, this: u32, flags: u32) {
    let id = e.mem.u32(at(this, 0xc));
    if id >= CREATED_IDS {
        let save_load = e.mem.u32(BGS_SAVE_LOAD_GAME);
        if e.call(SAVE_LOAD_IS_FORM_CHANGED, &args![save_load, this])
            .bool()
        {
            let change = at(this, ACTOR_BASE_DATA + DATA_CHANGE_FLAGS);
            let value = e.mem.u32(change);
            e.mem.set_u32(change, !flags & value);
            return;
        }
    }
    typed_call!(FAMILY, 0x0048_4bc0, tesform::change_sibling(e, this, flags));
}

// Typed from 005f0fb0
/// An actor value of the actor base, asked of its actor value owner (`this`, at +0x100): codes 0
/// to 4 and 0x39 the AI form's bytes, 5 to 0xb the attributes, 0x10 the health, 0x15 the speed
/// multiplier, 0x16 the actor base data's slot 0x60 (16 bits), 0x17 its karma (slot 0x64) made an
/// integer by `_ftol2`; any other code 0.
pub fn get_actor_value(e: &mut Engine, this: u32, code: u32) -> u32 {
    let ai = this.wrapping_sub(OWNER_TO_AI_FORM);
    let data = this.wrapping_sub(OWNER_TO_DATA);
    match code {
        0..=4 => u32::from(e.mem.u8(at(ai, AI_FORM_BYTES + code))),
        5..=0xb => attribute_byte(e, this.wrapping_sub(OWNER_TO_ATTRIBUTES), code),
        0x10 => typed_call!(
            FAMILY,
            0x005f_0b00,
            get_health(e, this.wrapping_sub(OWNER_TO_BASE))
        ),
        0x15 => u32::from(e.mem.u16(at(data, DATA_SPEED_MULT))),
        0x16 => u32::from(e.vcall(data, SLOT_DATA_STATISTIC, &args![]).u16()),
        0x17 => {
            let karma = e.vcall(data, SLOT_DATA_KARMA, &args![]).f64();
            let whole = typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, karma));
            crt::low_word(whole)
        }
        0x39 => u32::from(e.mem.u8(at(ai, AI_FORM_BYTE_39))),
        _ => 0,
    }
}

// Typed from 005f12d0
/// An actor value of the actor base set through its components (`this` the actor base): codes 0
/// to 4 the AI form's setters (the low byte; 0 to 3 marking the form changed), 5 to 0xb the
/// attributes, 0x10 the health, 0x15 the speed multiplier (16 bits), 0x17 the karma (the integer
/// as an `f32`); any other code nothing.
pub fn set_actor_value(e: &mut Engine, this: u32, code: u32, value: u32) {
    let ai = at(this, AI_FORM);
    let attributes = at(this, ATTRIBUTES);
    let byte = value & 0xff;
    let low = low_byte(value);
    match code {
        0 => {
            e.call(AI_FORM_SET_0, &args![ai, byte, 1u32]);
        }
        1 => {
            e.call(AI_FORM_SET_1, &args![ai, byte, 1u32]);
        }
        2 => {
            e.call(AI_FORM_SET_2, &args![ai, byte, 1u32]);
        }
        3 => {
            e.call(AI_FORM_SET_3, &args![ai, byte, 1u32]);
        }
        4 => {
            e.call(AI_FORM_SET_4, &args![ai, byte]);
        }
        5 => typed_call!(FAMILY, 0x005f_14b0, set_attribute_5(e, attributes, low)),
        6 => typed_call!(FAMILY, 0x005f_14d0, set_attribute_6(e, attributes, low)),
        7 => typed_call!(FAMILY, 0x005f_14f0, set_attribute_7(e, attributes, low)),
        8 => typed_call!(FAMILY, 0x005f_1510, set_attribute_8(e, attributes, low)),
        9 => typed_call!(FAMILY, 0x005f_1530, set_attribute_9(e, attributes, low)),
        0xa => typed_call!(FAMILY, 0x005f_1550, set_attribute_a(e, attributes, low)),
        0xb => typed_call!(FAMILY, 0x005f_1570, set_attribute_b(e, attributes, low)),
        0x10 => typed_call!(FAMILY, 0x005f_0b30, set_health(e, this, value)),
        0x15 => {
            let speed = low_half(value);
            typed_call!(
                FAMILY,
                0x0047_e270,
                actor_base_data::set_speed_mult(e, at(this, ACTOR_BASE_DATA), speed)
            );
        }
        0x17 => {
            let karma = f32::from_bits(x87_store32(f64::from(i32::from_ne_bytes(
                value.to_ne_bytes(),
            ))));
            typed_call!(
                FAMILY,
                0x0047_e1c0,
                actor_base_data::set_karma(e, at(this, ACTOR_BASE_DATA), karma)
            );
        }
        _ => {}
    }
}

/// `form` as `target` (the RTTI cast from `TESForm`), or 0.
fn cast_form(e: &mut Engine, form: u32, target: u32) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![form, 0u32, TYPE_TES_FORM, target, 0u32],
    )
    .u32()
}

/// A double result stored as the `f32` the exe stores.
fn stored(value: f64) -> f32 {
    f32::from_bits(x87_store32(value))
}

// Typed from 005f0d60
/// `TESActorBase::GetDesirability` (Xbox PDB): how much the actor base wants an item, by the item's
/// form type: an armour with health its rating (`CalcArmorRating` of its base rating's truncated
/// low 16 bits with 1.0, plus `004be180`); a clothing item `00647cf0` of its word at +0x5c; type
/// 0x1e the constant at `01016410`; a weapon with health `CalcWeaponDamage` for the actor value
/// owner (0 for no actor base), 1.0, 1.0 and four zeros; ammunition 1.0; anything else, a null
/// form or a failed cast 0.
pub fn get_desirability(e: &mut Engine, this: u32, form: u32) -> f32 {
    if form == 0 {
        return 0.0;
    }
    match form_type(e, form) {
        0x29 => {
            if cast_form(e, form, TYPE_TES_AMMO) != 0 {
                return 1.0;
            }
        }
        0x28 => {
            let weapon = cast_form(e, form, TYPE_TES_OBJECT_WEAP);
            if weapon != 0 && e.call(GET_FORM_HEALTH, &args![weapon]).u32() != 0 {
                let owner = if this == 0 {
                    0
                } else {
                    at(this, OWNER_TO_BASE)
                };
                let one = 1.0f32.to_bits();
                let damage = e
                    .call(
                        CALC_WEAPON_DAMAGE,
                        &args![owner, weapon, one, one, 0u32, 0u32, 0u32, 0u32],
                    )
                    .f64();
                return stored(damage);
            }
        }
        0x1a => {
            let clothing = cast_form(e, form, TYPE_TES_OBJECT_CLOT);
            if clothing != 0 {
                let value = e.mem.u32(at(clothing, 0x5c));
                return stored(e.call(CLOTHING_RATING, &args![value]).f64());
            }
        }
        0x18 => {
            let armor = cast_form(e, form, TYPE_TES_OBJECT_ARMO);
            if armor != 0 && e.call(GET_FORM_HEALTH, &args![armor]).u32() != 0 {
                let base = e.call(ARMOR_BASE_RATING, &args![armor]).f64();
                let truncated = u32::from_ne_bytes(fistp_chop(base).to_ne_bytes()) & 0xffff;
                let rating = e
                    .call(CALC_ARMOR_RATING, &args![truncated, 1.0f32.to_bits()])
                    .f64();
                let extra = e.call(ARMOR_EXTRA_RATING, &args![armor]).f64();
                return stored(x87_add(extra, rating));
            }
        }
        0x1e => return x87_quiet(f32::from_bits(e.mem.u32(KEY_DESIRABILITY))),
        _ => {}
    }
    0.0
}

// Typed from 005f1590
/// `TESActorBase::ShouldRunInitDefaultWorn` (Xbox PDB): false when the save/load game holds a
/// change for the reference under the worn items' change flags (0x08000020 when the save/load
/// game's flag 2 is set and the reference's form flag 0x200000 is, 0x400 when the flag is clear
/// and the reference's slot 0x22c with 0 says yes); else true.
pub fn should_run_init_default_worn(e: &mut Engine, _this: u32, reference: u32) -> bool {
    let save_load = e.mem.u32(BGS_SAVE_LOAD_GAME);
    let flags = if e.mem.u32(at(save_load, SAVE_LOAD_FLAGS)) & 2 != 0 {
        if typed_call!(
            leaf::FAMILY,
            0x0057_6d30,
            leaf::form_flag_200000(e, reference)
        ) {
            0x0800_0020u32
        } else {
            return true;
        }
    } else if e.vcall(reference, SLOT_REFERENCE_TEST, &args![0u32]).bool() {
        0x400
    } else {
        return true;
    };
    let save_load = e.mem.u32(BGS_SAVE_LOAD_GAME);
    !e.call(SAVE_LOAD_GET_CHANGE, &args![save_load, reference, flags])
        .bool()
}

// Typed from 005f1f30
/// The actor base's save into a save buffer: the form's flags (`TESForm`'s), the actor base data,
/// the spell list, the attributes and the AI form, then with the buffer's name flag the full name's
/// text (`SaveString`).
pub fn save_to_buffer(e: &mut Engine, this: u32, buffer: u32) {
    typed_call!(FAMILY, 0x0048_4d60, tesform::write_flags(e, this, buffer));
    e.call(
        DATA_SAVE_TO_BUFFER,
        &args![at(this, ACTOR_BASE_DATA), buffer],
    );
    e.call(SPELLS_SAVE_TO_BUFFER, &args![at(this, SPELL_LIST), buffer]);
    typed_call!(
        FAMILY,
        0x0048_0270,
        attributes::save_to_buffer(e, at(this, ATTRIBUTES), buffer)
    );
    e.call(AI_SAVE_TO_BUFFER, &args![at(this, AI_FORM), buffer]);
    if buffer_has_name(e, buffer) {
        let text = name_text(e, this);
        e.call(SAVE_STRING, &args![buffer, text, 0u32]);
    }
}

// Typed from 005f0b30
/// The health set (the health form's word at +4).
pub fn set_health(e: &mut Engine, this: u32, health: u32) {
    e.mem.set_u32(at(this, HEALTH_FORM + 4), health);
}

// Typed from 005f0ba0
/// False for a creature whose base flags have neither 0x10 nor 1, or have 0x40000; else true.
pub fn creature_flags_allow(e: &mut Engine, this: u32) -> bool {
    if form_type(e, this) != CREATURE_TYPE {
        return true;
    }
    let flags = base_flags(e, this);
    flags & 0x11 != 0 && flags & 0x0004_0000 == 0
}

// Typed from 005f0f50
/// Whether the form flag 0x80000 is set (`SetStartsDead`'s).
pub fn starts_dead(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(at(this, FORM_FLAGS)) & STARTS_DEAD != 0
}

// Typed from 005f16f0
/// The actor base's save size for the change `flags`: 6 for the block header with save blocks,
/// the form's flags' (save version 0x6d and up), the attributes', the actor base data's, the spell
/// list's and the AI form's, and with flag 0x20 the name's length and its byte; reported when the
/// save size reports are on. (The exe's 4 bytes for the health are under a mask the compiler
/// folded to 0: never added.)
pub fn save_size(e: &mut Engine, this: u32, flags: u32) -> u16 {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    let mut size: u16 = 0;
    if uses_blocks(e, save_load) {
        size = 6;
    }
    if e.mem.u8(at(save_load, SAVE_LOAD_VERSION)) >= FORM_FLAGS_VERSION {
        size = size.wrapping_add(typed_call!(
            FAMILY,
            0x0048_4bf0,
            tesform::flags_save_size(e, this, flags)
        ));
    }
    size = size.wrapping_add(typed_call!(
        FAMILY,
        0x0048_01e0,
        attributes::save_size(e, at(this, ATTRIBUTES), flags)
    ));
    size = size.wrapping_add(typed_call!(
        FAMILY,
        0x0047_e380,
        actor_base_data::save_size(e, at(this, ACTOR_BASE_DATA), flags)
    ));
    size = size.wrapping_add(
        e.call(SPELLS_SAVE_SIZE, &args![at(this, SPELL_LIST), flags])
            .u16(),
    );
    size = size.wrapping_add(e.call(AI_SAVE_SIZE, &args![at(this, AI_FORM), flags]).u16());
    if flags & CHANGE_NAME != 0 {
        let text = name_text(e, this);
        let length = e.call(STRING_LENGTH, &args![text]).u16();
        size = size.wrapping_add(1).wrapping_add(length);
    }
    if reports_save_sizes(e) {
        let save_load = e.mem.u32(SAVE_LOAD_GAME);
        report_size(
            e,
            save_load,
            u32::from(size),
            0x226,
            (TEXT_SIZE_FORM, TEXT_SIZE_PLAIN),
        );
    }
    size
}

// Typed from 005f18c0
/// The actor base's save (the change `flags`): with save blocks the `KOLB` tag and a 16-bit size
/// placeholder; the form's flags (save version 0x6d and up), the attributes, the actor base data,
/// the spell list and the AI form; with flag 0x20 the name as a length byte and its characters
/// (`TESForm::SaveGameDataOLD`). The bytes written reported when the save size reports are on; with
/// save blocks the block's size written into the placeholder (a warning past 0xffff). (The exe's
/// health is under a mask the compiler folded to 0: never saved.)
pub fn save_game(e: &mut Engine, this: u32, flags: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    // The exe's frame: its locals at their offsets from `EBP`.
    e.with_stack(0x34, |e, frame| {
        let ebp = frame.addr() + 0x34;
        let size_slot = ebp - 8;
        let tag_slot = ebp - 0x10;
        let length_slot = ebp - 0x19;
        e.mem.set_u16(size_slot, 0);
        let mut placeholder = 0u32;
        let start = e.mem.u32(at(save_load, SAVE_LOAD_POSITION));
        if uses_blocks(e, save_load) {
            e.mem.set_u32(tag_slot, TAG_KOLB);
            e.call(SAVE_BYTES, &args![save_load, tag_slot, 4u32]);
            placeholder = e.mem.u32(at(save_load, SAVE_LOAD_POSITION));
            e.call(SAVE_BYTES, &args![save_load, size_slot, 2u32]);
        }
        if e.mem.u8(at(save_load, SAVE_LOAD_VERSION)) >= FORM_FLAGS_VERSION {
            typed_call!(FAMILY, 0x0048_4c20, tesform::save_flags(e, this, flags));
        }
        typed_call!(
            FAMILY,
            0x0048_0210,
            attributes::save_game(e, at(this, ATTRIBUTES), flags)
        );
        typed_call!(
            FAMILY,
            0x0047_e4a0,
            actor_base_data::save_game(e, at(this, ACTOR_BASE_DATA), flags)
        );
        e.call(SPELLS_SAVE_GAME, &args![at(this, SPELL_LIST), flags]);
        e.call(AI_SAVE_GAME, &args![at(this, AI_FORM), flags]);
        if flags & CHANGE_NAME != 0 {
            let text = name_text(e, this);
            let length = e.call(STRING_LENGTH, &args![text]).u8();
            e.mem.set_u8(length_slot, length);
            typed_call!(
                FAMILY,
                0x0048_4ce0,
                tesform::save_game_data_old(e, this, length_slot, 1)
            );
            let length = u32::from(e.mem.u8(length_slot));
            if length != 0 {
                typed_call!(
                    FAMILY,
                    0x0048_4ce0,
                    tesform::save_game_data_old(e, this, text, length)
                );
            }
        }
        if reports_save_sizes(e) {
            let end = e.mem.u32(at(save_load, SAVE_LOAD_POSITION));
            report_size(
                e,
                save_load,
                end.wrapping_sub(start),
                0x24b,
                (TEXT_SAVE_FORM, TEXT_SAVE_PLAIN),
            );
        }
        if uses_blocks(e, save_load) {
            let end = e.mem.u32(at(save_load, SAVE_LOAD_POSITION));
            if end > placeholder.wrapping_add(0xffff) {
                e.call(
                    MESSAGE_LOG,
                    &args![TEXT_BLOCK_TOO_BIG, SOURCE_FILE, 0x24bu32],
                );
            }
            e.mem
                .set_u16(placeholder, low_half(end.wrapping_sub(placeholder)));
        }
    });
}

/// A load warning: with a current load `entry` (its form already looked up) the form's id, name,
/// the entry's version byte (+9) and flags (+5) after the source line; else the save's version
/// byte.
#[allow(clippy::too_many_arguments)]
fn warn_load(
    e: &mut Engine,
    save_load: u32,
    entry: u32,
    form: u32,
    texts: (u32, u32),
    amount: Option<u32>,
    line: u32,
) {
    let mut words = Vec::with_capacity(8);
    if entry != 0 {
        let name = e.vcall(form, SLOT_FORM_NAME, &args![]).u32();
        let id = e.mem.u32(entry);
        words.push(texts.0);
        words.extend(amount);
        words.extend([
            SOURCE_FILE,
            line,
            id,
            name,
            u32::from(e.mem.u8(at(entry, 9))),
            e.mem.u32(at(entry, 5)),
        ]);
    } else {
        words.push(texts.1);
        words.extend(amount);
        words.extend([
            SOURCE_FILE,
            line,
            u32::from(e.mem.u8(at(save_load, SAVE_LOAD_VERSION))),
        ]);
    }
    e.call(MESSAGE_LOG, &words);
}

// Typed from 005f1b30
/// The actor base's load (the change `flags`, and a word passed on): with save blocks the tag (a
/// warning when it is not `KOLB`) and the 16-bit block size; the form's flags (save version 0x6d
/// and up), the attributes, the actor base data, the spell list and the AI form; with flag 0x20 the
/// name, read as a length byte and its characters into a cleared 0x104-byte buffer and set as the
/// full name. With save blocks a warning when the position is past or short of the block's end.
/// (The exe's health is under a mask the compiler folded to 0: never loaded.)
pub fn load_game(e: &mut Engine, this: u32, flags: u32, second: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    // The exe's frame: its locals at their offsets from `EBP`.
    e.with_stack(0x13c, |e, frame| {
        let ebp = frame.addr() + 0x13c;
        let size_slot = ebp - 4;
        let tag_slot = ebp - 0xc;
        let name_buffer = ebp - 0x128;
        let length_slot = ebp - 0x129;
        e.mem.set_u16(size_slot, 0);
        let mut block_start = 0u32;
        if uses_blocks(e, save_load) {
            e.call(LOAD_BYTES, &args![save_load, tag_slot, 4u32]);
            if e.mem.u32(tag_slot) != TAG_KOLB {
                let (entry, form) = load_entry_form(e, save_load);
                let texts = (TEXT_TAG_FORM, TEXT_TAG_PLAIN);
                warn_load(e, save_load, entry, form, texts, None, 0x254);
            }
            block_start = e.mem.u32(at(save_load, SAVE_LOAD_POSITION));
            e.call(LOAD_BYTES, &args![save_load, size_slot, 2u32]);
        }
        if e.mem.u8(at(save_load, SAVE_LOAD_VERSION)) >= FORM_FLAGS_VERSION {
            typed_call!(
                FAMILY,
                0x0048_4c50,
                tesform::load_game(e, this, flags, second)
            );
        }
        typed_call!(
            FAMILY,
            0x0048_0240,
            attributes::load_game(e, at(this, ATTRIBUTES), flags, second)
        );
        typed_call!(
            FAMILY,
            0x0047_e6f0,
            actor_base_data::load_game(e, at(this, ACTOR_BASE_DATA), flags, second)
        );
        e.call(
            SPELLS_LOAD_GAME,
            &args![at(this, SPELL_LIST), flags, second],
        );
        e.call(AI_LOAD_GAME, &args![at(this, AI_FORM), flags, second]);
        if flags & CHANGE_NAME != 0 {
            typed_call!(
                crt::FAMILY,
                0x0040_3d30,
                crt::mem::memset_wrapper(e, Ptr::new(name_buffer), 0, NAME_BUFFER_SIZE)
            );
            typed_call!(
                FAMILY,
                0x0048_4d00,
                tesform::load_game_data_old(e, this, length_slot, 1)
            );
            let length = u32::from(e.mem.u8(length_slot));
            if length != 0 {
                typed_call!(
                    FAMILY,
                    0x0048_4d00,
                    tesform::load_game_data_old(e, this, name_buffer, length)
                );
            }
            e.call(SET_NAME, &args![at(this, FULL_NAME), name_buffer]);
        }
        if !uses_blocks(e, save_load) {
            return;
        }
        let position = e.mem.u32(at(save_load, SAVE_LOAD_POSITION));
        // The exe looks the entry's form up before it compares.
        let (entry, form) = load_entry_form(e, save_load);
        let end = u32::from(e.mem.u16(size_slot)).wrapping_add(block_start);
        let (texts, amount) = if position > end {
            (
                (TEXT_OVERRUN_FORM, TEXT_OVERRUN_PLAIN),
                position.wrapping_sub(end),
            )
        } else if position < end {
            (
                (TEXT_UNDERRUN_FORM, TEXT_UNDERRUN_PLAIN),
                end.wrapping_sub(position),
            )
        } else {
            return;
        };
        warn_load(e, save_load, entry, form, texts, Some(amount), 0x274);
    });
}

/// The save/load game's current load entry and, when there is one, the form its id names.
fn load_entry_form(e: &mut Engine, save_load: u32) -> (u32, u32) {
    let entry = e.mem.u32(at(save_load, SAVE_LOAD_LOAD_ENTRY));
    if entry == 0 {
        return (0, 0);
    }
    let id = e.mem.u32(entry);
    (
        entry,
        typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id)),
    )
}

// Typed from 005f1fd0
/// The actor base's load from a load buffer: the form's flags (`TESForm`'s), the actor base data,
/// the spell list, the attributes and the AI form, then with the buffer's name flag a name read
/// into a stack buffer (`LoadString`) and set as the full name.
pub fn load_from_buffer(e: &mut Engine, this: u32, buffer: u32) {
    typed_call!(FAMILY, 0x0048_4da0, tesform::read_flags(e, this, buffer));
    e.call(
        DATA_LOAD_FROM_BUFFER,
        &args![at(this, ACTOR_BASE_DATA), buffer],
    );
    e.call(
        SPELLS_LOAD_FROM_BUFFER,
        &args![at(this, SPELL_LIST), buffer],
    );
    typed_call!(
        FAMILY,
        0x0048_02b0,
        attributes::load_from_buffer(e, at(this, ATTRIBUTES), buffer)
    );
    e.call(AI_LOAD_FROM_BUFFER, &args![at(this, AI_FORM), buffer]);
    // The exe's frame: the name buffer at its offset from `EBP`.
    e.with_stack(0x118, |e, frame| {
        if buffer_has_name(e, buffer) {
            let name = frame.addr() + 8;
            e.call(LOAD_STRING, &args![buffer, name]);
            e.call(SET_NAME, &args![at(this, FULL_NAME), name]);
        }
    });
}

/// A `Reset3DMobIterator` built at `this`: the base's empty word at +4, its vtable.
fn construct_iterator(e: &mut Engine, this: u32) {
    e.mem.set_u32(at(this, 4), 0);
    e.mem.set_u32(this, RESET_3D_ITERATOR_VTABLE);
}

/// `value` passed to the process of a reference whose base object is `base` (slot 0x468), when
/// it has one.
fn set_on_process(e: &mut Engine, reference: u32, base: u32, value: u8) {
    if e.mem.u32(at(reference, REFERENCE_BASE_OBJECT)) != base {
        return;
    }
    let process = e.mem.u32(at(reference, REFERENCE_PROCESS));
    if process != 0 {
        e.vcall(process, SLOT_PROCESS_SET, &args![u32::from(value)]);
    }
}

// Typed from 005f20a0
/// For a non-zero `value`: a `Reset3DMobIterator` for this actor base and `value` walked over the
/// list at `011e0e80` twice (its +4 word 0, then 1); then, when this is the player's base form and
/// the player has a process, `value` passed to the process (slot 0x468).
pub fn reset_3d(e: &mut Engine, this: u32, value: u8) {
    if value == 0 {
        return;
    }
    e.with_stack(0x10, |e, iterator| {
        let iterator = iterator.addr();
        construct_iterator(e, iterator);
        e.mem.set_u32(at(iterator, 8), this);
        e.mem.set_u8(at(iterator, 0xc), value);
        for pass in [0u32, 1] {
            e.mem.set_u32(at(iterator, 4), pass);
            e.call(ITERATE, &args![ITERATED_LIST, iterator]);
        }
    });
    let player = e.mem.u32(PLAYER);
    set_on_process(e, player, this, value);
}

// Typed from 005f2150
/// The `Reset3DMobIterator` constructor (its vtable's slot 0 is `005f2170`). Returns `this`.
pub fn construct_reset_3d_iterator(e: &mut Engine, this: u32) -> u32 {
    construct_iterator(e, this);
    this
}

// Typed from 005f2170
/// `Reset3DMobIterator::Operate` (Xbox PDB): when the reference's base object is the iterator's
/// actor base (+8) and the reference has a process, the iterator's byte (+0xc) passed to the
/// process (slot 0x468).
pub fn reset_3d_operate(e: &mut Engine, this: u32, reference: u32) {
    let base = e.mem.u32(at(this, 8));
    let value = e.mem.u8(at(this, 0xc));
    set_on_process(e, reference, base, value);
}
