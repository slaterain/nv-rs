//! `TESActorBaseData`'s functions (the Xbox PDB's `tesactorbasedata` source unit),
//! each as the exe does it.
//!
//! `TESActorBaseData` is a form component at +0x30 of `TESNPC` and `TESCreature` (so
//! `component - 0x30` is the owning form, whose slots 0x48 and 0x4c mark it changed): the base flags
//! at +4, the fatigue (+8), bartergold (+0xa), level (+0xc), calculated level range (+0xe, +0x10),
//! speed multiplier (+0x12), karma (`f32`, +0x14), base disposition (+0x18), template-use flags
//! (+0x1a), death item (+0x1c), voice type (+0x20), template form (+0x24), change flags (+0x28) and
//! the faction list (`BSSimpleList<FACTION_RANK *>`, its head node at +0x2c; a `FACTION_RANK` is
//! the faction at +0 and the rank byte at +4).
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds (checked
//! in the exe): the base flags' tests (`00461580`, `00461560` with 0x80), the form type
//! (`00401170`), the faction list's head (`005d8a70`, +0x2c), the template form (`0059bb30`,
//! +0x24), death item (`00441110`, +0x1c) and speed multiplier (`008f21d0`, +0x12) getters, the
//! death item's, voice type's and template form's setters (`0050f9a0`, `0050f9c0`, `007037c0`),
//! the addresses +4 and +0x44 (`00717e50`, `005d43c0`), the settings' value pointers (`00403e20`,
//! `0043d4d0`, `00408d60`: a setting here is never null, so its value is at +4) and string
//! (`00403df0`), the byte swaps (`00401080`, `00407a90`), the big-endian byte (`00401500`), the
//! base component's and the empty list's constructors (`00402d30`, `0096a2d0`), the store of a word
//! (`008c71b0`), the save/load game's buffer position (`00825c00`, +0x14), current save and load
//! entries (`004fd3e0`, `004fd3c0`) and version byte (`008df040`), and the container's empty-list
//! test (`0042cde0`). `TESForm`'s functions (`AddChunkData`, `AddChunk` of a word, `GetFile`,
//! `AddCompileIndex`, the form lookup) are this family's typed calls, `memcpy` the C runtime
//! family's, and the template walk's extra data getters (the container changes, `00418520`, and the
//! thread's flag 4, `004121b0`) the extra data family's; a word the exe reads twice with nothing
//! between that could change it is read once.
//!
//! Floats: the loading threads and the AI ask these too, so the exe's extended precision is kept
//! as the translation keeps it: `f64` maths, rounded to `f32` where the exe stores a float, and a
//! float made an integer through `x87::fistp_chop` (the exe's `FISTP` under the truncating control
//! word it sets around it). Comparisons are of the values the exe compares.

use super::{tesform, FAMILY};
use crate::{cell_refs, crt, extra_data};
use engine_core::typed::leaf;
use engine_core::x87::{fistp_chop, x87_div, x87_mul, x87_quiet};
use engine_core::{args, typed_call, Engine, Ptr};

const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const FLOATS_NEAR: u32 = 0x0040_eb10;
/// The game's printf-style log (YUP and xNVSE may replace it).
const MESSAGE_LOG: u32 = 0x005b_5e40;
const EXTRA_LIST_CLEAR_A: u32 = 0x0041_aeb0;
const EXTRA_LIST_CLEAR_B: u32 = 0x0041_af50;
const TLS_FLAG_ONE_SET: u32 = 0x0046_23f0;
const CREATE_FORM_OF_TYPE: u32 = 0x0046_5110;
const CHAINED_CONTAINER_DESTRUCT: u32 = 0x0046_ffb0;
const TES_CONTAINER_CONSTRUCT: u32 = 0x0048_1610;
const TES_CONTAINER_DESTRUCT: u32 = 0x0048_1680;
const TES_CONTAINER_SIZE: u32 = 0x0c;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const ATEXIT: u32 = 0x00ec_658f;
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
const LIST_COUNT_ITEMS: u32 = 0x005a_e380;
const LIST_TAKE_HEAD: u32 = 0x005a_e3d0;
const LIST_APPEND: u32 = 0x0090_5820;
const SAVE_LOAD_GAME_GLOBAL: u32 = 0x011d_e45c;
const USE_SAVE_GAME_BLOCKS: u32 = 0x0086_2110;
const SAVE_BYTES: u32 = 0x0085_79b0;
const SAVE_NUMERIC_ID: u32 = 0x0085_7a10;
const SAVE_DEBUG_SWITCH: u32 = 0x011d_e4e8;
const SOURCE_FILE_PATH: u32 = 0x0101_b210;
const LOAD_BYTES: u32 = 0x0085_79e0;
const LOAD_NUMERIC_ID: u32 = 0x0085_7aa0;
const TEXT_LOAD_HEADER_PLAIN: u32 = 0x0101_56a8;
const TEXT_LOAD_HEADER_FORM: u32 = 0x0101_5718;
const TEXT_LOAD_OVERRUN_PLAIN: u32 = 0x0101_54a0;
const TEXT_LOAD_OVERRUN_FORM: u32 = 0x0101_5588;
const TEXT_LOAD_UNDERRUN_PLAIN: u32 = 0x0101_5440;
const TEXT_LOAD_UNDERRUN_FORM: u32 = 0x0101_5500;
const LEVEL_SOURCE_OBJECT: u32 = 0x011d_ea3c;
const ACTOR_TEMPLATE_BLOCKED: u32 = 0x0056_afc0;
const BUILD_TEMPLATE_CONTAINER: u32 = 0x0048_8350;
const CLEAR_ACTOR_PROCESS_FLAGS: u32 = 0x0088_4f80;
const APPLY_TEMPLATE_FORM: u32 = 0x0056_b020;
const SET_OBJECT_REFERENCE: u32 = 0x0057_5690;
const GET_INVENTORY_CHANGES: u32 = 0x004b_f220;
const INVENTORY_CHANGES_STEP_A: u32 = 0x004d_1440;
const INVENTORY_CHANGES_STEP_B: u32 = 0x004d_1610;
const INVENTORY_CHANGES_STEP_C: u32 = 0x004d_1960;
const ACTOR_LEVEL: u32 = 0x0087_f9f0;
const GET_CHANGE: u32 = 0x0084_a6d0;
const KARMA_ACTOR_VALUE: u32 = 0x17;
const TES_ACTOR_BASE_DATA_VTABLE: u32 = 0x0101_b0dc;
/// What `BaseFormComponent`'s constructor (`00402d30`) stores as the vtable.
const BASE_COMPONENT_VTABLE: u32 = 0x0101_1d68;
/// The target's byte order: not 0 when it stores multi-byte values big-endian (`00401500`).
const BIG_ENDIAN: u32 = 0x011c_54ba;
/// `TESSaveLoadGame`: its buffer position (`00825c00`), current save and load entries (`004fd3e0`,
/// `004fd3c0`) and version byte (`008df040`).
const SAVE_LOAD_POSITION: u32 = 0x14;
const SAVE_LOAD_SAVE_ENTRY: u32 = 0x88;
const SAVE_LOAD_LOAD_ENTRY: u32 = 0x84;
const SAVE_LOAD_VERSION: u32 = 0x80;
const FATIGUE_DEFAULT_SETTING: u32 = 0x011c_4fa4;
const KARMA_DEFAULT_SETTING: u32 = 0x011c_5074;
const DISPOSITION_DEFAULT_SETTING: u32 = 0x011c_50d8;
/// The four karma alignment thresholds (settings) `GetAlignmentForKarma` compares with: at or below
/// the evil one evil (very evil at or below its own), at or above the good one good (very good at
/// or above its own), else neutral.
const KARMA_EVIL: u32 = 0x011c_5144;
const KARMA_VERY_EVIL: u32 = 0x011c_4f64;
const KARMA_GOOD: u32 = 0x011c_4e54;
const KARMA_VERY_GOOD: u32 = 0x011c_5090;
const KARMA_MAX_SETTING: u32 = 0x011c_5108;
const KARMA_MIN_SETTING: u32 = 0x011c_5160;
const DEFAULT_ALIGNMENT_VALUES: u32 = 0x0118_6d08;
const ALIGNMENT_TABLE_STATE: u32 = 0x0118_6f78;
/// `InitItem`'s gaps between the thresholds (floats it computes once): very good less good, good
/// less evil, very evil less evil.
const GAP_GOOD_TO_VERY_GOOD: u32 = 0x011c_52a4;
const GAP_EVIL_TO_GOOD: u32 = 0x011c_52a0;
const GAP_EVIL_TO_VERY_EVIL: u32 = 0x011c_529c;
const KARMA_TOLERANCE: u32 = 0x0101_7d00;
const HALF: u32 = 0x0101_1588;
const HUNDRED: u32 = 0x0101_7a40;
const THOUSAND: u32 = 0x0101_7b70;
const EMPTY_STRING: u32 = 0x0101_1584;
const KARMIC_TITLE_TABLE: u32 = 0x0118_6d20;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_ACTOR_BASE_DATA: u32 = 0x0118_6cb4;
const RTTI_BASE_FORM_COMPONENT_SOURCE: u32 = 0x0118_3040;
const RTTI_FACTION: u32 = 0x0118_4704;
const RTTI_VOICE_TYPE: u32 = 0x0118_6318;
const RTTI_LEVELED_ITEM: u32 = 0x0118_6f7c;
const TEXT_DEATH_ITEM_MISSING: u32 = 0x0101_b1c8;
const TEXT_TEMPLATE_MISSING: u32 = 0x0101_b178;
const TEXT_INVALID_KARMA: u32 = 0x0101_b148;
const TEXT_SAVE_SIZE_FORM: u32 = 0x0101_2cb0;
const TEXT_SAVE_SIZE_PLAIN: u32 = 0x0101_2c78;
const TEXT_SAVE_FORM: u32 = 0x0101_53a0;
const TEXT_SAVE_PLAIN: u32 = 0x0101_536c;
const TEXT_SAVE_BLOCK_TOO_BIG: u32 = 0x0101_5318;
const TAG_ACBS: u32 = 0x5342_4341;
const TAG_INAM: u32 = 0x4d41_4e49;
const TAG_VTCK: u32 = 0x4b43_5456;
const TAG_TPLT: u32 = 0x544c_5054;
const TAG_KOBL: u32 = 0x424c_4f4b;
const BGS_SAVE_LOAD_GAME: u32 = 0x011d_df38;
const PATH_MANAGER_GUARD: u32 = 0x011c_5298;
const PATH_MANAGER_OBJECT: u32 = 0x011c_5290;
const PATH_MANAGER_ATEXIT: u32 = 0x00fc_8480;
const PATH_MANAGER_CONSTRUCT: u32 = 0x006e_b820;
const PATH_MANAGER_CLEAR_INVENTORY_POINTERS: u32 = 0x006e_bd10;

// Field offsets.
const BASE_FLAGS: u32 = 0x04;
const FATIGUE: u32 = 0x08;
const BARTERGOLD: u32 = 0x0a;
const LEVEL: u32 = 0x0c;
const CALC_LEVEL_MIN: u32 = 0x0e;
const CALC_LEVEL_MAX: u32 = 0x10;
const SPEED_MULT: u32 = 0x12;
const KARMA: u32 = 0x14;
const BASE_DISPOSITION: u32 = 0x18;
const TEMPLATE_USE_FLAGS: u32 = 0x1a;
const DEATH_ITEM: u32 = 0x1c;
const VOICE_TYPE: u32 = 0x20;
const TEMPLATE_FORM: u32 = 0x24;
const CHANGE_FLAGS: u32 = 0x28;
const FACTION_LIST: u32 = 0x2c;

/// The low half of a word (the exe's `u16` store of a value it means to cut).
#[allow(clippy::cast_possible_truncation)]
fn low_half(word: u32) -> u16 {
    word as u16
}

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

fn get_u16(e: &mut Engine, this: u32, offset: u32) -> u16 {
    e.mem.u16(at(this, offset))
}

fn set_u16(e: &mut Engine, this: u32, offset: u32, value: u16) {
    e.mem.set_u16(at(this, offset), value);
}

/// The value of a float setting, at +4 of the setting (`00403e20`'s pointer; it answers a shared
/// scratch word only for a null setting, and these are the exe's own settings).
fn setting_float(e: &mut Engine, setting: u32) -> f32 {
    e.mem.f32(setting.wrapping_add(4))
}

/// The form's type, the byte at +4 (`00401170`).
fn form_type(e: &mut Engine, form: u32) -> u32 {
    u32::from(e.mem.u8(form.wrapping_add(4)))
}

/// The template form (`0059bb30`: the word at +0x24).
fn template_form(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(at(this, TEMPLATE_FORM))
}

/// Whether the target stores multi-byte values big-endian (`00401500`'s byte).
fn is_big_endian(e: &mut Engine) -> bool {
    e.mem.u8(BIG_ENDIAN) != 0
}

/// `memcpy(to, from, n)`, the game's wrapper (`00401460`).
fn copy_memory(e: &mut Engine, to: u32, from: u32, n: u32) {
    typed_call!(
        crt::FAMILY,
        0x0040_1460,
        crt::mem::memcpy_wrapper(e, Ptr::new(to), Ptr::new(from), n)
    );
}

/// The save/load game's buffer position (`00825c00`).
fn buffer_position(e: &mut Engine, save_load: u32) -> u32 {
    e.mem.u32(save_load.wrapping_add(SAVE_LOAD_POSITION))
}

/// The head node of the faction list (`005d8a70`, `this + 0x2c`).
fn faction_list_head(this: u32) -> u32 {
    at(this, FACTION_LIST)
}

/// The item of a list node.
fn node_item(e: &mut Engine, node: u32) -> u32 {
    let slot = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    e.mem.u32(slot)
}

/// The node after `node`.
fn node_next(e: &mut Engine, node: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node))
}

/// Runs `f` with the address of a stack word holding `value`.
fn with_item<R>(e: &mut Engine, value: u32, f: impl FnOnce(&mut Engine, u32) -> R) -> R {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        f(e, slot.addr())
    })
}

/// A test of the base flags (`00461580` with `mask`: the word at +4).
fn base_flag(e: &mut Engine, this: u32, mask: u32) -> bool {
    e.mem.u32(at(this, BASE_FLAGS)) & mask != 0
}

/// `fistp_chop`'s integer's low half: a float made a `u16` field as the exe stores it.
fn chopped_half(value: f64) -> u16 {
    let whole = fistp_chop(value);
    low_half(u32::from_ne_bytes(whole.to_ne_bytes()))
}

// Typed from 0047c700
/// `TESActorBaseData::TESActorBaseData` (Xbox PDB): the component base, this class's vtable, the
/// faction list's head node and the defaults (`InitializeDataComponent`). Returns `this`.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, BASE_COMPONENT_VTABLE);
    e.mem.set_u32(this, TES_ACTOR_BASE_DATA_VTABLE);
    e.mem.set_u32(at(this, FACTION_LIST), 0);
    e.mem.set_u32(at(this, FACTION_LIST + 4), 0);
    typed_call!(FAMILY, 0x0047_c8e0, initialize_data_component(e, this));
    this
}

// Typed from 0047c770
/// `TESActorBaseData::GetNoVATSMelee` (Xbox PDB): base flag 0x100000.
pub fn no_vats_melee(e: &mut Engine, this: u32) -> bool {
    base_flag(e, this, 0x0010_0000)
}

// Typed from 0047c790
/// `TESActorBaseData::GetAllowPCDialogue` (Xbox PDB): base flag 0x200000.
pub fn allow_pc_dialogue(e: &mut Engine, this: u32) -> bool {
    base_flag(e, this, 0x0020_0000)
}

// Typed from 0047c7b0
/// `TESActorBaseData::GetAllowPickPocket` (Xbox PDB): base flag 0x10000000.
pub fn allow_pick_pocket(e: &mut Engine, this: u32) -> bool {
    base_flag(e, this, 0x1000_0000)
}

// Typed from 0047c7d0
/// `TESActorBaseData::GetIsGhost` (Xbox PDB): base flag 0x20000000.
pub fn is_ghost(e: &mut Engine, this: u32) -> bool {
    base_flag(e, this, 0x2000_0000)
}

// Typed from 0047c7f0
/// `TESActorBaseData::GetInvulnerable` (Xbox PDB): base flag 0x80000000.
pub fn invulnerable(e: &mut Engine, this: u32) -> bool {
    base_flag(e, this, 0x8000_0000)
}

// Typed from 0047c810
/// `TESActorBaseData::GetCantOpenDoors` (Xbox PDB): base flag 0x400000.
pub fn cant_open_doors(e: &mut Engine, this: u32) -> bool {
    base_flag(e, this, 0x0040_0000)
}

// Typed from 0047c830
/// `TESActorBaseData::GetAutoCalcServiceFlags` (Xbox PDB): base flag 0x800000.
pub fn auto_calc_service_flags(e: &mut Engine, this: u32) -> bool {
    base_flag(e, this, 0x0080_0000)
}

// Typed from 0047c860
/// `TESActorBaseData::GetKarma` (Xbox PDB): the karma, in `ST0`.
#[inline]
pub fn karma(e: &mut Engine, this: u32) -> f32 {
    e.mem.f32(at(this, KARMA))
}

// Typed from 0047c880
/// The destructor body: this class's vtable, `ClearDataComponent` (called directly), then the
/// faction list's chain destroyed.
pub fn destructor(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, TES_ACTOR_BASE_DATA_VTABLE);
    typed_call!(FAMILY, 0x0047_c9b0, clear_data_component(e, this));
    e.call(CHAINED_CONTAINER_DESTRUCT, &args![at(this, FACTION_LIST)]);
}

// Typed from 0047c8e0
/// `TESActorBaseData::InitializeDataComponent` (Xbox PDB): the defaults: no flags, the fatigue a
/// float setting chopped to an integer, no bartergold, level 1, no calculated range, speed 100,
/// the karma and base disposition from settings, no death item, voice type or template, no
/// template-use or change flags.
pub fn initialize_data_component(e: &mut Engine, this: u32) {
    e.mem.set_u32(at(this, BASE_FLAGS), 0);
    let fatigue = setting_float(e, FATIGUE_DEFAULT_SETTING);
    set_u16(e, this, FATIGUE, chopped_half(f64::from(fatigue)));
    set_u16(e, this, BARTERGOLD, 0);
    set_u16(e, this, LEVEL, 1);
    set_u16(e, this, CALC_LEVEL_MIN, 0);
    set_u16(e, this, CALC_LEVEL_MAX, 0);
    set_u16(e, this, SPEED_MULT, 100);
    let karma = setting_float(e, KARMA_DEFAULT_SETTING);
    e.mem.set_f32(at(this, KARMA), karma);
    // The setting's value pointer (`0043d4d0`): +4.
    let disposition = e.mem.u16(DISPOSITION_DEFAULT_SETTING.wrapping_add(4));
    set_u16(e, this, BASE_DISPOSITION, disposition);
    e.mem.set_u32(at(this, VOICE_TYPE), 0);
    e.mem.set_u32(at(this, DEATH_ITEM), 0);
    e.mem.set_u32(at(this, TEMPLATE_FORM), 0);
    set_u16(e, this, TEMPLATE_USE_FLAGS, 0);
    e.mem.set_u32(at(this, CHANGE_FLAGS), 0);
}

// Typed from 0047c9b0
/// `TESActorBaseData::ClearDataComponent` (Xbox PDB): the faction list cleared.
pub fn clear_data_component(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0047_c9d0, clear_faction_list(e, this));
}

// Typed from 0047c9d0
/// `TESActorBaseData::ClearFactionList` (Xbox PDB): while the list is not empty, its first
/// `FACTION_RANK` freed and its first node removed.
pub fn clear_faction_list(e: &mut Engine, this: u32) {
    let head = faction_list_head(this);
    while head != 0 && !typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, head)) {
        let item = node_item(e, head);
        e.call(OPERATOR_DELETE, &args![item]);
        e.call(LIST_REMOVE_HEAD, &args![head]);
    }
}

// Typed from 0047ca20
/// `TESActorBaseData::CopyFactionList` (Xbox PDB): this list cleared, then an 8-byte copy of each
/// `FACTION_RANK` of `source`'s list that is not null appended.
pub fn copy_faction_list(e: &mut Engine, this: u32, source: u32) {
    typed_call!(FAMILY, 0x0047_c9d0, clear_faction_list(e, this));
    let mut tail = faction_list_head(this);
    let mut node = faction_list_head(source);
    while node != 0 {
        let rank = node_item(e, node);
        if rank != 0 {
            let copy = e.call(OPERATOR_NEW, &args![8u32]).u32();
            copy_memory(e, copy, rank, 8);
            with_item(e, copy, |e, slot| {
                e.call(LIST_APPEND, &args![tail, slot]);
            });
        }
        let next = node_next(e, tail);
        if next != 0 {
            tail = next;
        }
        node = node_next(e, node);
    }
}

// Typed from 0047cac0
/// `TESActorBaseData::Save` (Xbox PDB): the `ACBS` chunk (the 0x18 bytes of the data, swapped
/// around the write on a big-endian target), then the death item (`INAM`), voice type (`VTCK`)
/// and template form (`TPLT`) as form ids when set.
pub fn save(e: &mut Engine, this: u32) {
    let data = at(this, BASE_FLAGS);
    if is_big_endian(e) {
        typed_call!(FAMILY, 0x0047_cb90, swap_data(e, data));
    }
    typed_call!(
        FAMILY,
        0x0048_5990,
        tesform::add_chunk_data(e, TAG_ACBS, data, 0x18)
    );
    if is_big_endian(e) {
        typed_call!(FAMILY, 0x0047_cb90, swap_data(e, data));
    }
    for (tag, offset) in [
        (TAG_INAM, DEATH_ITEM),
        (TAG_VTCK, VOICE_TYPE),
        (TAG_TPLT, TEMPLATE_FORM),
    ] {
        let form = e.mem.u32(at(this, offset));
        if form != 0 {
            let id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, form));
            typed_call!(FAMILY, 0x0048_5910, tesform::add_chunk_word(e, tag, id));
        }
    }
}

// Typed from 0047cb90
/// The 0x18 bytes of the data at `data` swapped in place: the flags and the karma as words, the
/// statistics and the last two fields as halves.
pub fn swap_data(e: &mut Engine, data: u32) {
    for (offset, wide) in [
        (0x00u32, true),
        (0x04, false),
        (0x06, false),
        (0x08, false),
        (0x0a, false),
        (0x0c, false),
        (0x0e, false),
        (0x10, true),
        (0x14, false),
        (0x16, false),
    ] {
        // `00401080` and `00407a90`: the runtime's byte swaps, in place.
        let at = data.wrapping_add(offset);
        if wide {
            let word = e.mem.u32(at);
            e.mem.set_u32(at, word.swap_bytes());
        } else {
            let half = e.mem.u16(at);
            e.mem.set_u16(at, half.swap_bytes());
        }
    }
}

// Typed from 0047cca0
/// Whether template-use flag 15 is set.
pub fn template_use_flag_15(e: &mut Engine, this: u32) -> bool {
    typed_call!(FAMILY, 0x0047_cd40, template_use_flag(e, this, 0xf))
}

// Typed from 0047ccc0
/// Template-use flag 15 set or cleared.
pub fn set_template_use_flag_15(e: &mut Engine, this: u32, value: u8) {
    typed_call!(
        FAMILY,
        0x0047_cce0,
        set_template_use_flag(e, this, 0xf, value)
    );
}

// Typed from 0047cce0
/// Template-use flag `index` (0 to 15) set (`value` not 0) or cleared; another index does nothing.
pub fn set_template_use_flag(e: &mut Engine, this: u32, index: i32, value: u8) {
    if (0..0x10).contains(&index) {
        let mask = low_half(1u32 << index);
        let flags = get_u16(e, this, TEMPLATE_USE_FLAGS);
        let flags = if value != 0 {
            flags | mask
        } else {
            flags & !mask
        };
        set_u16(e, this, TEMPLATE_USE_FLAGS, flags);
    }
}

// Typed from 0047cd40
/// Whether template-use flag `index` (0 to 15) is set; true for another index.
pub fn template_use_flag(e: &mut Engine, this: u32, index: i32) -> bool {
    if (0..0x10).contains(&index) {
        let mask = low_half(1u32 << index);
        get_u16(e, this, TEMPLATE_USE_FLAGS) & mask != 0
    } else {
        true
    }
}

// Typed from 0047cd90
/// The template-use flags replaced.
pub fn set_template_use_flags(e: &mut Engine, this: u32, flags: u16) {
    set_u16(e, this, TEMPLATE_USE_FLAGS, flags);
}

// Typed from 0047cdb0
/// The exe's forwarding copy of [`has_actor_template`].
pub fn has_actor_template_again(e: &mut Engine, this: u32) -> bool {
    typed_call!(FAMILY, 0x0047_cdd0, has_actor_template(e, this))
}

// Typed from 0047cdd0
/// Whether the template form is set and of form type 0x2c or 0x2d.
pub fn has_actor_template(e: &mut Engine, this: u32) -> bool {
    let template = e.mem.u32(at(this, TEMPLATE_FORM));
    if template == 0 {
        return false;
    }
    (0x2c..=0x2d).contains(&form_type(e, template))
}

/// The template's own `TESActorBaseData` (+0x30) when the template is of type `kind`.
/// The exe reads the template twice; nothing between the reads writes it.
fn template_component_of_type(e: &mut Engine, this: u32, kind: u32) -> Option<u32> {
    let template = template_form(e, this);
    if form_type(e, template) != kind {
        return None;
    }
    Some(if template != 0 { template + 0x30 } else { 0 })
}

// Typed from 0047ce10
/// The template applied to a loaded actor: with a template of type 0x2c or 0x2d and an actor the
/// `0056afc0` test lets through, the template's own component builds a container for the actor,
/// the actor's inventory pointers leave the path manager's requests, a new template base form is
/// made for the actor's saved form, and its inventory is rebuilt through its container changes.
pub fn apply_template(e: &mut Engine, this: u32, actor: u32) {
    if actor == 0
        || !typed_call!(FAMILY, 0x0047_cdb0, has_actor_template_again(e, this))
        || e.call(ACTOR_TEMPLATE_BLOCKED, &args![actor]).bool()
    {
        return;
    }
    let component = match template_component_of_type(e, this, 0x2d) {
        Some(component) => component,
        None => template_component_of_type(e, this, 0x2c).unwrap_or(0),
    };
    if component == 0 {
        return;
    }
    e.with_stack(TES_CONTAINER_SIZE, |e, container| {
        e.call(TES_CONTAINER_CONSTRUCT, &args![container]);
        e.call(
            BUILD_TEMPLATE_CONTAINER,
            &args![component, actor, container],
        );
        let list = container.addr().wrapping_add(4);
        let first = node_item(e, list);
        let first_entry_word = e.mem.u32(first.wrapping_add(4));
        let handler = e.mem.u32(BGS_SAVE_LOAD_GAME);
        if handler == 0
            || !typed_call!(
                extra_data::FAMILY,
                0x0041_21b0,
                extra_data::copy::thread_flag_4(e, handler)
            )
        {
            if e.vcall(actor, 0xfc, &args![]).bool()
                && typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, actor)) != 0
            {
                e.vcall(actor, 0x244, &args![]);
            }
            e.vcall(actor, 0x1cc, &args![0u32, 1u32]);
        }
        let manager = typed_call!(FAMILY, 0x0047_d0b0, path_manager_instance(e));
        if typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, manager)) != 0 {
            let extra_list = actor.wrapping_add(0x44);
            let changes = typed_call!(
                extra_data::FAMILY,
                0x0041_8520,
                extra_data::access::container_changes(e, extra_list)
            );
            let manager = typed_call!(FAMILY, 0x0047_d0b0, path_manager_instance(e));
            e.call(
                PATH_MANAGER_CLEAR_INVENTORY_POINTERS,
                &args![manager, changes],
            );
        }
        let extra_list = actor.wrapping_add(0x44);
        e.call(EXTRA_LIST_CLEAR_A, &args![extra_list]);
        let extra_list = actor.wrapping_add(0x44);
        e.call(EXTRA_LIST_CLEAR_B, &args![extra_list]);
        if e.vcall(actor, 0x100, &args![]).bool() {
            e.call(CLEAR_ACTOR_PROCESS_FLAGS, &args![actor]);
        }
        let saved_form = typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, actor));
        let template_base = typed_call!(
            FAMILY,
            0x0047_d130,
            create_template_actor_base(e, saved_form, first_entry_word)
        );
        e.call(
            APPLY_TEMPLATE_FORM,
            &args![actor, saved_form, first_entry_word],
        );
        e.call(SET_OBJECT_REFERENCE, &args![actor, template_base]);
        typed_call!(
            cell_refs::FAMILY,
            0x0056_5730,
            cell_refs::script::init_script(e, actor)
        );
        let changes = e.call(GET_INVENTORY_CHANGES, &args![actor]).u32();
        e.call(INVENTORY_CHANGES_STEP_A, &args![changes]);
        e.call(INVENTORY_CHANGES_STEP_B, &args![changes]);
        e.call(INVENTORY_CHANGES_STEP_C, &args![changes]);
        // `0042cde0`: the list its first word points at holds no item and no next node.
        let list = e.mem.u32(changes);
        if e.mem.u32(list.wrapping_add(4)) == 0 && e.mem.u32(list) == 0 {
            let extra_list = actor.wrapping_add(0x44);
            e.call(EXTRA_LIST_CLEAR_A, &args![extra_list]);
        }
        e.call(TES_CONTAINER_DESTRUCT, &args![container]);
    });
}

// Typed from 0047d0b0
/// `PathManager::QInstance`: the function-local `PathManager` at `011c5290`, made on the first
/// call (guard bit 0 of `011c5298`) with its destructor given to `atexit`.
pub fn path_manager_instance(e: &mut Engine) -> u32 {
    let guard = e.mem.u32(PATH_MANAGER_GUARD);
    if guard & 1 == 0 {
        e.mem.set_u32(PATH_MANAGER_GUARD, guard | 1);
        e.call(PATH_MANAGER_CONSTRUCT, &args![PATH_MANAGER_OBJECT]);
        e.call(ATEXIT, &args![PATH_MANAGER_ATEXIT]);
    }
    PATH_MANAGER_OBJECT
}

// Typed from 0047d130
/// `TESActorBaseData::CreateTemplateActorBase` (Xbox PDB): with the handler's flag cleared, a form
/// of `source`'s type made, `source` copied into it (its slot 0x108), its component given the
/// template (the component's slot 0x10), the flag put back; the new form.
pub fn create_template_actor_base(e: &mut Engine, source: u32, template: u32) -> u32 {
    let handler = e.mem.u32(BGS_SAVE_LOAD_GAME);
    let saved = e.call(TLS_FLAG_ONE_SET, &args![handler, 0u32]).u8();
    let form_type = form_type(e, source);
    let form = e.call(CREATE_FORM_OF_TYPE, &args![form_type]).u32();
    e.vcall(form, 0x108, &args![source]);
    e.vcall(form.wrapping_add(0x30), 0x10, &args![template]);
    let handler = e.mem.u32(BGS_SAVE_LOAD_GAME);
    e.call(TLS_FLAG_ONE_SET, &args![handler, saved]);
    form
}

// Typed from 0047d1a0
/// The chain of template forms from `form`: while the form's slot 0xf8 says true, its half at
/// +0x4a ANDed into `*mask` (from 0xffff) and the walk moves to its template; the first form whose
/// slot says false, or 0 at the end.
pub fn template_chain_mask(e: &mut Engine, form: u32, mask: u32) -> u32 {
    let mut found = 0;
    e.mem.set_u16(mask, 0xffff);
    let mut current = form;
    while current != 0 {
        if e.vcall(current, 0xf8, &args![]).bool() {
            let bits = e.mem.u16(current.wrapping_add(0x4a));
            let combined = e.mem.u16(mask) & bits;
            e.mem.set_u16(mask, combined);
            current = template_form(e, current.wrapping_add(0x30));
        } else {
            found = current;
            current = 0;
        }
    }
    found
}

/// `__RTDynamicCast(component, 0, BaseFormComponent, TESActorBaseData, 0)`.
fn as_actor_base_data(e: &mut Engine, component: u32) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![
            component,
            0u32,
            RTTI_BASE_FORM_COMPONENT_SOURCE,
            RTTI_TES_ACTOR_BASE_DATA,
            0u32
        ],
    )
    .u32()
}

// Typed from 0047d220
/// `TESActorBaseData::CopyComponent` (Xbox PDB): when `source` casts to a `TESActorBaseData`, its
/// base flags, fatigue (slot 0x60), karma (slot 0x64), bartergold, level, calculated range, speed
/// and base disposition through this class's setters (each marks the form changed), then the
/// faction list, death item, voice type (slot 0x68), template form and template-use flags.
pub fn copy_component(e: &mut Engine, this: u32, source: u32) {
    let other = as_actor_base_data(e, source);
    if other == 0 {
        return;
    }
    let flags = e.mem.u32(at(other, BASE_FLAGS));
    typed_call!(FAMILY, 0x0047_dd30, set_base_flags(e, this, flags));
    let fatigue = e.vcall(other, 0x60, &args![]).u16();
    typed_call!(FAMILY, 0x0047_e010, set_fatigue(e, this, fatigue));
    let karma = e.vcall(other, 0x64, &args![]).f32();
    typed_call!(FAMILY, 0x0047_e1c0, set_karma(e, this, karma));
    let bartergold = typed_call!(FAMILY, 0x0047_d3f0, bartergold(e, other));
    typed_call!(FAMILY, 0x0047_e240, set_bartergold(e, this, bartergold));
    let level = typed_call!(FAMILY, 0x0047_d370, level(e, other));
    typed_call!(FAMILY, 0x0047_dfe0, set_level(e, this, level));
    let calc_min = typed_call!(FAMILY, 0x0047_d390, calc_level_min(e, other));
    typed_call!(FAMILY, 0x0047_de40, set_calc_level_min(e, this, calc_min));
    let calc_max = typed_call!(FAMILY, 0x0047_d3b0, calc_level_max(e, other));
    typed_call!(FAMILY, 0x0047_de70, set_calc_level_max(e, this, calc_max));
    let speed = get_u16(e, other, SPEED_MULT);
    typed_call!(FAMILY, 0x0047_e270, set_speed_mult(e, this, speed));
    let disposition = typed_call!(FAMILY, 0x0047_d3d0, base_disposition(e, other));
    typed_call!(
        FAMILY,
        0x0047_dea0,
        set_base_disposition(e, this, disposition)
    );
    typed_call!(FAMILY, 0x0047_ca20, copy_faction_list(e, this, other));
    let death_item = e.mem.u32(at(other, DEATH_ITEM));
    e.mem.set_u32(at(this, DEATH_ITEM), death_item);
    let voice = e.vcall(other, 0x68, &args![]).u32();
    e.mem.set_u32(at(this, VOICE_TYPE), voice);
    let template = template_form(e, other);
    e.mem.set_u32(at(this, TEMPLATE_FORM), template);
    let use_flags = get_u16(e, other, TEMPLATE_USE_FLAGS);
    set_u16(e, this, TEMPLATE_USE_FLAGS, use_flags);
}

// Typed from 0047d370
/// The level (+0xc).
#[inline]
pub fn level(e: &mut Engine, this: u32) -> u16 {
    get_u16(e, this, LEVEL)
}

// Typed from 0047d390
/// The calculated level's minimum (+0xe).
#[inline]
pub fn calc_level_min(e: &mut Engine, this: u32) -> u16 {
    get_u16(e, this, CALC_LEVEL_MIN)
}

// Typed from 0047d3b0
/// The calculated level's maximum (+0x10).
#[inline]
pub fn calc_level_max(e: &mut Engine, this: u32) -> u16 {
    get_u16(e, this, CALC_LEVEL_MAX)
}

// Typed from 0047d3d0
/// The base disposition (+0x18).
#[inline]
pub fn base_disposition(e: &mut Engine, this: u32) -> u16 {
    get_u16(e, this, BASE_DISPOSITION)
}

// Typed from 0047d3f0
/// The bartergold (+0xa).
#[inline]
pub fn bartergold(e: &mut Engine, this: u32) -> u16 {
    get_u16(e, this, BARTERGOLD)
}

// Typed from 0047d410
/// `TESActorBaseData::CompareComponent` (Xbox PDB): true (different) unless `other` casts to a
/// `TESActorBaseData` with the same base flags, fatigue (slot 0x60), karma (slot 0x64, a NaN
/// differs), bartergold, level, calculated range, death item, speed, base disposition, the same
/// rank for every faction of this list (`GetFactionRank`; the other's factions are only counted),
/// voice type (slot 0x68), template form and template-use flags.
pub fn compare_component(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = as_actor_base_data(e, other);
    if cast == 0 {
        return true;
    }
    if e.mem.u32(at(this, BASE_FLAGS)) != e.mem.u32(at(cast, BASE_FLAGS)) {
        return true;
    }
    if e.vcall(this, 0x60, &args![]).u16() != e.vcall(cast, 0x60, &args![]).u16() {
        return true;
    }
    let my_karma = f64::from(e.vcall(this, 0x64, &args![]).f32());
    let their_karma = f64::from(e.vcall(cast, 0x64, &args![]).f32());
    if my_karma != their_karma {
        return true;
    }
    if typed_call!(FAMILY, 0x0047_d3f0, bartergold(e, this))
        != typed_call!(FAMILY, 0x0047_d3f0, bartergold(e, cast))
        || typed_call!(FAMILY, 0x0047_d370, level(e, this))
            != typed_call!(FAMILY, 0x0047_d370, level(e, cast))
        || typed_call!(FAMILY, 0x0047_d390, calc_level_min(e, this))
            != typed_call!(FAMILY, 0x0047_d390, calc_level_min(e, cast))
        || typed_call!(FAMILY, 0x0047_d3b0, calc_level_max(e, this))
            != typed_call!(FAMILY, 0x0047_d3b0, calc_level_max(e, cast))
    {
        return true;
    }
    if e.mem.u32(at(this, DEATH_ITEM)) != e.mem.u32(at(cast, DEATH_ITEM)) {
        return true;
    }
    if get_u16(e, this, SPEED_MULT) != get_u16(e, cast, SPEED_MULT) {
        return true;
    }
    if typed_call!(FAMILY, 0x0047_d3d0, base_disposition(e, this))
        != typed_call!(FAMILY, 0x0047_d3d0, base_disposition(e, cast))
    {
        return true;
    }
    let my_list = faction_list_head(this);
    let my_count = e.call(LIST_COUNT_ITEMS, &args![my_list]).u32();
    let their_list = faction_list_head(cast);
    let their_count = e.call(LIST_COUNT_ITEMS, &args![their_list]).u32();
    if my_count != their_count {
        return true;
    }
    let mut node = faction_list_head(this);
    while node != 0 {
        let entry = node_item(e, node);
        if entry != 0 {
            let faction = e.mem.u32(entry);
            let rank = typed_call!(FAMILY, 0x0047_d680, faction_rank(e, cast, faction, 0));
            let own_rank = i32::from(e.mem.i8(entry.wrapping_add(4)));
            if rank != own_rank {
                return true;
            }
        }
        node = node_next(e, node);
    }
    if e.vcall(this, 0x68, &args![]).u32() != e.vcall(cast, 0x68, &args![]).u32() {
        return true;
    }
    if template_form(e, this) != template_form(e, cast) {
        return true;
    }
    get_u16(e, this, TEMPLATE_USE_FLAGS) != get_u16(e, cast, TEMPLATE_USE_FLAGS)
}

// Typed from 0047d680
/// `TESActorBaseData::GetFactionRank` (Xbox PDB): the rank (a signed byte) of `faction` in the
/// list, or -1; the search ends at the first node without an item. With `check_faction_flag`,
/// factions whose flag bit 3 is set (`0047d720`) never match.
pub fn faction_rank(e: &mut Engine, this: u32, faction: u32, check_faction_flag: u8) -> i32 {
    let mut node = faction_list_head(this);
    loop {
        if node == 0 || node_item(e, node) == 0 {
            return -1;
        }
        let entry = node_item(e, node);
        let entry_faction = e.mem.u32(entry);
        let matches = if check_faction_flag != 0 {
            entry != 0
                && !typed_call!(FAMILY, 0x0047_d720, faction_flag_8(e, entry_faction))
                && entry_faction == faction
        } else {
            entry != 0 && entry_faction == faction
        };
        if matches {
            return i32::from(e.mem.i8(entry.wrapping_add(4)));
        }
        node = node_next(e, node);
    }
}

// Typed from 0047d720
/// Whether bit 3 of a faction's flags (+0x34) is set.
#[inline]
pub fn faction_flag_8(e: &mut Engine, faction: u32) -> bool {
    e.mem.u32(faction.wrapping_add(0x34)) & 8 != 0
}

// Typed from 0047d740
/// `TESActorBaseData::IsInEvilFactionsOnly` (Xbox PDB): true when the list is not empty and the
/// faction of every entry that is not null has flag bit 1 (`0047d7c0`).
pub fn in_evil_factions_only(e: &mut Engine, this: u32) -> bool {
    let mut result = true;
    let mut node = faction_list_head(this);
    if typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, node)) {
        result = false;
    }
    while node != 0 && result {
        let entry = node_item(e, node);
        if entry != 0 {
            let faction = e.mem.u32(entry);
            if !typed_call!(FAMILY, 0x0047_d7c0, faction_flag_2(e, faction)) {
                result = false;
            }
        }
        node = node_next(e, node);
    }
    result
}

// Typed from 0047d7c0
/// Whether bit 1 of a faction's flags (+0x34) is set.
#[inline]
pub fn faction_flag_2(e: &mut Engine, faction: u32) -> bool {
    e.mem.u32(faction.wrapping_add(0x34)) & 2 != 0
}

// Typed from 0047d7e0
/// Whether bit 2 of a faction's flags (+0x34) is set.
#[inline]
pub fn faction_flag_4(e: &mut Engine, faction: u32) -> bool {
    e.mem.u32(faction.wrapping_add(0x34)) & 4 != 0
}

// Typed from 0047d800
/// `TESActorBaseData::SetFactionRank` (Xbox PDB): nothing for no faction. A listed faction's rank
/// -1 removes its node (`0063f7b0`), any other is stored; an unlisted faction gets a new 8-byte
/// `FACTION_RANK` at the list's head (`005ae3d0`).
pub fn set_faction_rank(e: &mut Engine, this: u32, faction: u32, rank: i8) {
    if faction == 0 {
        return;
    }
    let mut node = faction_list_head(this);
    while node != 0 {
        let entry = node_item(e, node);
        if entry != 0 && e.mem.u32(entry) == faction {
            if rank == -1 {
                e.call(LIST_REMOVE_HEAD, &args![node]);
            } else {
                e.mem.set_i8(entry.wrapping_add(4), rank);
            }
            return;
        }
        node = node_next(e, node);
    }
    let entry = e.call(OPERATOR_NEW, &args![8u32]).u32();
    e.mem.set_u32(entry, faction);
    e.mem.set_i8(entry.wrapping_add(4), rank);
    with_item(e, entry, |e, slot| {
        let head = faction_list_head(this);
        e.call(LIST_TAKE_HEAD, &args![head, slot]);
    });
}

/// A stored form id remapped to the load order (`AddCompileIndex`), looked up and, with a type
/// descriptor, cast from `TESForm` to it: the remapped id and the result.
fn resolve_form_id(e: &mut Engine, id: u32, file: u32, cast_to: Option<u32>) -> (u32, u32) {
    with_item(e, id, |e, slot| {
        typed_call!(
            FAMILY,
            0x0048_5d50,
            tesform::add_compile_index(e, slot, file)
        );
        let remapped = e.mem.u32(slot);
        let found = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, remapped));
        let result = match cast_to {
            Some(target) => e
                .call(
                    DYNAMIC_CAST,
                    &args![found, 0u32, RTTI_TES_FORM, target, 0u32],
                )
                .u32(),
            None => found,
        };
        (remapped, result)
    })
}

/// `InitItem`'s "could not find" line: the message, the missing id, `form`'s id and its name
/// (slot 0x130).
fn log_missing_form(e: &mut Engine, text: u32, id: u32, form: u32) {
    let name = e.vcall(form, 0x130, &args![]).u32();
    let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, form));
    e.call(MESSAGE_LOG, &args![text, id, form_id, name]);
}

/// A form-id field resolved in place (0 stays 0), logging `missing` when it does not resolve.
fn resolve_field(
    e: &mut Engine,
    this: u32,
    offset: u32,
    file: u32,
    cast_to: Option<u32>,
    missing: Option<(u32, u32)>,
) {
    let id = e.mem.u32(at(this, offset));
    if id == 0 {
        return;
    }
    let (remapped, resolved) = resolve_form_id(e, id, file, cast_to);
    e.mem.set_u32(at(this, offset), resolved);
    if let Some((text, form)) = missing {
        if resolved == 0 {
            log_missing_form(e, text, remapped, form);
        }
    }
}

/// `a - b` of two float settings, stored as the exe stores it.
fn setting_difference(e: &mut Engine, a: u32, b: u32) -> f32 {
    let a = setting_float(e, a);
    let b = setting_float(e, b);
    round_to_f32(f64::from(a) - f64::from(b))
}

/// An `f64` rounded to the `f32` the exe stores.
#[allow(clippy::cast_possible_truncation)]
fn round_to_f32(value: f64) -> f32 {
    value as f32
}

// Typed from 0047d8b0
/// `TESActorBaseData::InitItem` (Xbox PDB): after a load, the stored form ids resolved: each
/// faction entry's to its faction (an entry that does not resolve leaves the list), the voice
/// type's, the death item's and the template form's (the last two logged when missing). Then,
/// unless template-use flag 0 is set on a template, a stale karma is repaired: the first time, the
/// five default alignment values are checked against `GetAlignmentForKarma` and the gaps between
/// the thresholds kept; with an inconsistent table, a karma within the tolerance of a default
/// value moves to the value the thresholds give, else `Invalid karma value found` is logged.
pub fn init_item(e: &mut Engine, this: u32, form: u32) {
    let mut node = faction_list_head(this);
    let file = if form == 0 {
        0
    } else {
        typed_call!(FAMILY, 0x0048_4e60, tesform::get_file(e, form, -1))
    };
    while node != 0 {
        let entry = node_item(e, node);
        if entry == 0 {
            node = node_next(e, node);
            continue;
        }
        if e.mem.u32(entry) != 0 {
            let id = e.mem.u32(entry);
            let (_, resolved) = resolve_form_id(e, id, file, Some(RTTI_FACTION));
            e.mem.set_u32(entry, resolved);
        }
        if e.mem.u32(entry) != 0 {
            node = node_next(e, node);
        } else {
            e.call(LIST_REMOVE_HEAD, &args![node]);
        }
    }
    resolve_field(e, this, VOICE_TYPE, file, Some(RTTI_VOICE_TYPE), None);
    resolve_field(
        e,
        this,
        DEATH_ITEM,
        file,
        Some(RTTI_LEVELED_ITEM),
        Some((TEXT_DEATH_ITEM_MISSING, form)),
    );
    resolve_field(
        e,
        this,
        TEMPLATE_FORM,
        file,
        None,
        Some((TEXT_TEMPLATE_MISSING, form)),
    );

    if e.mem.u32(at(this, TEMPLATE_FORM)) != 0
        && typed_call!(FAMILY, 0x0047_cd40, template_use_flag(e, this, 0))
    {
        return;
    }
    if e.mem.i32(ALIGNMENT_TABLE_STATE) == -1 {
        for index in 0..5u32 {
            let value = e.mem.f32(DEFAULT_ALIGNMENT_VALUES + 4 * index);
            if typed_call!(FAMILY, 0x0047_e040, alignment_for_karma(e, value)) != index as i32 {
                e.mem.set_i32(ALIGNMENT_TABLE_STATE, 1);
                let gap = setting_difference(e, KARMA_VERY_GOOD, KARMA_GOOD);
                e.mem.set_f32(GAP_GOOD_TO_VERY_GOOD, gap);
                let gap = setting_difference(e, KARMA_GOOD, KARMA_EVIL);
                e.mem.set_f32(GAP_EVIL_TO_GOOD, gap);
                let gap = setting_difference(e, KARMA_VERY_EVIL, KARMA_EVIL);
                e.mem.set_f32(GAP_EVIL_TO_VERY_EVIL, gap);
                break;
            }
        }
        if e.mem.i32(ALIGNMENT_TABLE_STATE) != 1 {
            e.mem.set_i32(ALIGNMENT_TABLE_STATE, 0);
        }
    }
    if e.mem.i32(ALIGNMENT_TABLE_STATE) != 1 {
        return;
    }
    let tolerance = e.mem.f32(KARMA_TOLERANCE);
    let half = e.mem.f64(HALF);
    let hundred = e.mem.f64(HUNDRED);
    let near_default = |e: &mut Engine, index: u32| -> bool {
        let karma = e.mem.f32(at(this, KARMA));
        let default = e.mem.f32(DEFAULT_ALIGNMENT_VALUES + 4 * index);
        e.call(FLOATS_NEAR, &args![karma, default, tolerance])
            .bool()
    };
    let new_karma = if near_default(e, 1) {
        let base = setting_float(e, KARMA_GOOD);
        let gap = e.mem.f32(GAP_EVIL_TO_GOOD);
        Some(f64::from(base) - f64::from(gap) * half)
    } else if near_default(e, 0) {
        let base = setting_float(e, KARMA_GOOD);
        let gap = e.mem.f32(GAP_GOOD_TO_VERY_GOOD);
        Some(f64::from(gap) * half + f64::from(base))
    } else if near_default(e, 2) {
        let base = setting_float(e, KARMA_EVIL);
        let gap = e.mem.f32(GAP_EVIL_TO_VERY_EVIL);
        Some(f64::from(gap) * half + f64::from(base))
    } else if near_default(e, 3) {
        let base = setting_float(e, KARMA_VERY_GOOD);
        Some(f64::from(base) + hundred)
    } else if near_default(e, 4) {
        let base = setting_float(e, KARMA_VERY_EVIL);
        Some(f64::from(base) - hundred)
    } else {
        e.call(MESSAGE_LOG, &args![TEXT_INVALID_KARMA]);
        None
    };
    if let Some(karma) = new_karma {
        e.mem.set_f32(at(this, KARMA), round_to_f32(karma));
    }
}

// Typed from 0047dd30
/// The base flags replaced, the form marked changed (2).
pub fn set_base_flags(e: &mut Engine, this: u32, flags: u32) {
    e.mem.set_u32(at(this, BASE_FLAGS), flags);
    typed_call!(FAMILY, 0x0047_e2a0, mark_changed(e, this, 2));
}

// Typed from 0047dd50
/// `TESActorBaseData::SetFlagBit` (Xbox PDB): the base flag bits `mask` set (`value` not 0) or
/// cleared. When that changes them: for mask 1 the component's form, when it casts to one, is
/// marked (`0047e2f0` with 0x1000000 when `BGSSaveLoadGame::GetChange` says it has that change,
/// else `0047e2a0`); for another mask the form is marked changed (2) when `mark` is not 0.
pub fn set_flag_bit(e: &mut Engine, this: u32, mask: u32, value: u8, mark: u8) {
    let flags = e.mem.u32(at(this, BASE_FLAGS));
    let current = u8::from(flags & mask != 0);
    if current != value {
        if mask == 1 {
            let form = e
                .call(
                    DYNAMIC_CAST,
                    &args![this, 0u32, RTTI_TES_ACTOR_BASE_DATA, RTTI_TES_FORM, 0u32],
                )
                .u32();
            if form != 0 {
                // The by-value change flags (the exe stores them with `008c71b0`).
                let change_flags = 0x0100_0000u32;
                let save_load = e.mem.u32(BGS_SAVE_LOAD_GAME);
                if !e
                    .call(GET_CHANGE, &args![save_load, form, change_flags])
                    .bool()
                {
                    typed_call!(FAMILY, 0x0047_e2a0, mark_changed(e, this, 0x0100_0000));
                } else {
                    typed_call!(FAMILY, 0x0047_e2f0, mark_unchanged(e, this, 0x0100_0000));
                }
            }
        } else if mark != 0 {
            typed_call!(FAMILY, 0x0047_e2a0, mark_changed(e, this, 2));
        }
    }
    let flags = e.mem.u32(at(this, BASE_FLAGS));
    let flags = if value != 0 {
        flags | mask
    } else {
        flags & !mask
    };
    e.mem.set_u32(at(this, BASE_FLAGS), flags);
}

/// A half field set, then the form marked changed (2).
fn set_marked(e: &mut Engine, this: u32, offset: u32, value: u16) {
    set_u16(e, this, offset, value);
    typed_call!(FAMILY, 0x0047_e2a0, mark_changed(e, this, 2));
}

// Typed from 0047de40
/// The calculated level's minimum set, the form marked changed.
pub fn set_calc_level_min(e: &mut Engine, this: u32, value: u16) {
    set_marked(e, this, CALC_LEVEL_MIN, value);
}

// Typed from 0047de70
/// The calculated level's maximum set, the form marked changed.
pub fn set_calc_level_max(e: &mut Engine, this: u32, value: u16) {
    set_marked(e, this, CALC_LEVEL_MAX, value);
}

// Typed from 0047dea0
/// The base disposition set, the form marked changed.
pub fn set_base_disposition(e: &mut Engine, this: u32, value: u16) {
    set_marked(e, this, BASE_DISPOSITION, value);
}

// Typed from 0047ded0
/// The actor's level: the level, which with base flag 0x80 (`00461560`) is `level / 1000.0` (an
/// `f32`) times the level of the component of the form at +0x20 of the object at `011dea3c`,
/// chopped, when there is one; then a calculated minimum above it is returned, and a calculated
/// maximum below it caps it.
pub fn calculated_level(e: &mut Engine, this: u32) -> u16 {
    let mut level = typed_call!(FAMILY, 0x0047_d370, level(e, this));
    if base_flag(e, this, 0x80) {
        let divisor = e.mem.f64(THOUSAND);
        let scale = round_to_f32(x87_div(f64::from(level), divisor));
        let source = e.mem.u32(LEVEL_SOURCE_OBJECT);
        let form = typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, source));
        let component = if form != 0 { form + 0x30 } else { 0 };
        if component != 0 {
            let other = typed_call!(FAMILY, 0x0047_d370, self::level(e, component));
            // The exe's FMUL feeds its FISTP with no store between: the product rounded to the
            // running thread's precision (24 bits on main) is what is chopped.
            level = chopped_half(x87_mul(f64::from(other), f64::from(scale)));
        }
        // Each bound is asked again for each use, as the exe calls its getter.
        if typed_call!(FAMILY, 0x0047_d390, calc_level_min(e, this)) != 0
            && level < typed_call!(FAMILY, 0x0047_d390, calc_level_min(e, this))
        {
            return typed_call!(FAMILY, 0x0047_d390, calc_level_min(e, this));
        }
        if typed_call!(FAMILY, 0x0047_d3b0, calc_level_max(e, this)) != 0
            && level > typed_call!(FAMILY, 0x0047_d3b0, calc_level_max(e, this))
        {
            level = typed_call!(FAMILY, 0x0047_d3b0, calc_level_max(e, this));
        }
    }
    level
}

// Typed from 0047dfe0
/// The level set, the form marked changed.
pub fn set_level(e: &mut Engine, this: u32, value: u16) {
    set_marked(e, this, LEVEL, value);
}

// Typed from 0047e010
/// The fatigue set, the form marked changed.
pub fn set_fatigue(e: &mut Engine, this: u32, value: u16) {
    set_marked(e, this, FATIGUE, value);
}

// Typed from 0047e040
/// `TESActorBaseData::GetAlignmentForKarma` (Xbox PDB): a karma up to the `011c5144` threshold is
/// 4 when also up to `011c4f64`, else 2; a higher (or NaN) karma is 1 when below `011c4e54` (or
/// NaN), 0 when below `011c5090` (or NaN), else 3.
pub fn alignment_for_karma(e: &mut Engine, karma: f32) -> i32 {
    let karma = f64::from(karma);
    let first = f64::from(setting_float(e, KARMA_EVIL));
    if karma <= first {
        let second = f64::from(setting_float(e, KARMA_VERY_EVIL));
        return if karma <= second { 4 } else { 2 };
    }
    // `threshold <= karma` is false for a NaN on either side, as the exe's compare.
    let third = f64::from(setting_float(e, KARMA_GOOD));
    if third <= karma {
        let fourth = f64::from(setting_float(e, KARMA_VERY_GOOD));
        if fourth <= karma {
            3
        } else {
            0
        }
    } else {
        1
    }
}

// Typed from 0047e0e0
/// `TESActorBaseData::GetKarmicTitle`: an actor's title string, by its level (`0087f9f0`, 1 to
/// 50) and the alignment of its karma (actor value 0x17 through the object at `actor + 0xa4`, its
/// slot 8; alignments 3 and 4 take the rows of 0 and 2); the empty string for none or no actor.
pub fn karmic_title(e: &mut Engine, actor: u32) -> u32 {
    let mut title = 0u32;
    if actor != 0 {
        let level = i32::from(e.call(ACTOR_LEVEL, &args![actor]).u16());
        let level_index = (level - 1).clamp(0, 0x31);
        let karma = e
            .vcall(actor.wrapping_add(0xa4), 8, &args![KARMA_ACTOR_VALUE])
            .i32();
        #[allow(clippy::cast_precision_loss)]
        let karma = karma as f32;
        let alignment = typed_call!(FAMILY, 0x0047_e040, alignment_for_karma(e, karma));
        let row = match alignment {
            0..=2 => alignment.unsigned_abs(),
            3 => 0,
            _ => 2,
        };
        let setting = e
            .mem
            .u32(KARMIC_TITLE_TABLE + row * 0xc8 + level_index as u32 * 4);
        if setting != 0 {
            // `00403df0`: the setting's string, the word at +4.
            title = e.mem.u32(setting.wrapping_add(4));
        }
    }
    if title == 0 {
        title = EMPTY_STRING;
    }
    title
}

// Typed from 0047e1c0
/// The karma stored and clamped to the upper setting (`011c5108`) or else the lower one
/// (`011c5160`); the form marked changed.
pub fn set_karma(e: &mut Engine, this: u32, karma: f32) {
    let field = at(this, KARMA);
    // The exe's FLD/FSTP of the argument quiets a signalling NaN.
    e.mem.set_f32(field, x87_quiet(karma));
    let upper = setting_float(e, KARMA_MAX_SETTING);
    let current = e.mem.f32(field);
    if upper < current {
        e.mem.set_f32(field, upper);
    } else {
        let lower = setting_float(e, KARMA_MIN_SETTING);
        let current = e.mem.f32(field);
        if lower > current {
            e.mem.set_f32(field, lower);
        }
    }
    typed_call!(FAMILY, 0x0047_e2a0, mark_changed(e, this, 2));
}

// Typed from 0047e240
/// The bartergold set, the form marked changed.
pub fn set_bartergold(e: &mut Engine, this: u32, value: u16) {
    set_marked(e, this, BARTERGOLD, value);
}

// Typed from 0047e270
/// The speed multiplier set, the form marked changed.
pub fn set_speed_mult(e: &mut Engine, this: u32, value: u16) {
    set_marked(e, this, SPEED_MULT, value);
}

/// The owning form (`this - 0x30`; 0 for no component).
fn owning_form(this: u32) -> u32 {
    if this == 0 {
        0
    } else {
        this.wrapping_sub(0x30)
    }
}

// Typed from 0047e2a0
/// The owning form marked changed with `flags`: its slot 0x48.
pub fn mark_changed(e: &mut Engine, this: u32, flags: u32) {
    let form = owning_form(this);
    if form != 0 {
        e.vcall(form, 0x48, &args![flags]);
    }
}

// Typed from 0047e2f0
/// The same through the owning form's slot 0x4c.
pub fn mark_unchanged(e: &mut Engine, this: u32, flags: u32) {
    let form = owning_form(this);
    if form != 0 {
        e.vcall(form, 0x4c, &args![flags]);
    }
}

// Typed from 0047e340
/// Bits set in the change flags.
pub fn set_change_flags(e: &mut Engine, this: u32, mask: u32) {
    let flags = e.mem.u32(at(this, CHANGE_FLAGS));
    e.mem.set_u32(at(this, CHANGE_FLAGS), flags | mask);
}

// Typed from 0047e360
/// Bits cleared in the change flags.
pub fn clear_change_flags(e: &mut Engine, this: u32, mask: u32) {
    let flags = e.mem.u32(at(this, CHANGE_FLAGS));
    e.mem.set_u32(at(this, CHANGE_FLAGS), flags & !mask);
}

/// Whether the debug switch byte (`011de4e8 + 4`, through `00408d60`'s value pointer) is set.
fn save_debug_enabled(e: &mut Engine) -> bool {
    e.mem.u8(SAVE_DEBUG_SWITCH.wrapping_add(4)) != 0
}

/// The debug line of the save size and the save: with a current save entry, its form id, its
/// form's name (slot 0x130) and its flags (the word at +5).
fn log_save_function(
    e: &mut Engine,
    save_load: u32,
    amount: u32,
    line: u32,
    text_with_form: u32,
    text_plain: u32,
) {
    let entry = e.mem.u32(save_load.wrapping_add(SAVE_LOAD_SAVE_ENTRY));
    if entry != 0 {
        let form_id = e.mem.u32(entry);
        let form = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, form_id));
        let name = e.vcall(form, 0x130, &args![]).u32();
        let entry_flags = e.mem.u32(entry.wrapping_add(5));
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words text_with_form, amount, form_id, name, entry_flags, line, SOURCE_FILE_PATH);
    } else {
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words text_plain, amount, line, SOURCE_FILE_PATH);
    }
}

// Typed from 0047e380
/// The component's save size: 6 for the block header with save-game blocks in use, 0x18 with bit
/// 1 of `flags`, 2 for the faction count and 5 per faction entry; logged with the debug switch.
pub fn save_size(e: &mut Engine, this: u32, flags: u32) -> u16 {
    let mut size: u16 = 0;
    let save_load = e.mem.u32(SAVE_LOAD_GAME_GLOBAL);
    if e.call(USE_SAVE_GAME_BLOCKS, &args![save_load]).bool() {
        size = size.wrapping_add(6);
    }
    if flags & 2 != 0 {
        size = size.wrapping_add(0x18);
    }
    size = size.wrapping_add(2);
    let count = e
        .call(LIST_COUNT_ITEMS, &args![at(this, FACTION_LIST)])
        .u32();
    size = size.wrapping_add(low_half(count.wrapping_mul(5)));
    if save_debug_enabled(e) {
        log_save_function(
            e,
            save_load,
            u32::from(size),
            0xc4e,
            TEXT_SAVE_SIZE_FORM,
            TEXT_SAVE_SIZE_PLAIN,
        );
    }
    size
}

/// `bytes` bytes of the stack word holding `value` written to the save buffer.
fn save_word(e: &mut Engine, save_load: u32, value: u32, bytes: u32) {
    with_item(e, value, |e, slot| {
        e.call(SAVE_BYTES, &args![save_load, slot, bytes]);
    });
}

// Typed from 0047e4a0
/// The component's save into the save buffer: a `KOBL` block header and a 16-bit size placeholder
/// with save-game blocks in use, the 0x18 bytes of the data with bit 1 of `flags`, the faction
/// count (16-bit) and per `FACTION_RANK` its faction's numeric id (4 bytes) and rank (1 byte); the
/// count and the block size patched in at the end. The debug switch logs the bytes written; a
/// block over 64 KiB is reported.
pub fn save_game(e: &mut Engine, this: u32, flags: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME_GLOBAL);
    let mut block_size_slot = 0u32;
    let mut start = buffer_position(e, save_load);
    if save_debug_enabled(e) {
        start = buffer_position(e, save_load);
    }
    if e.call(USE_SAVE_GAME_BLOCKS, &args![save_load]).bool() {
        save_word(e, save_load, TAG_KOBL, 4);
        block_size_slot = buffer_position(e, save_load);
        e.with_stack(4, |e, size| {
            e.call(SAVE_BYTES, &args![save_load, size, 2u32]);
        });
    }
    if flags & 2 != 0 {
        e.call(SAVE_BYTES, &args![save_load, at(this, BASE_FLAGS), 0x18u32]);
    }
    let mut count = 0u16;
    let count_slot = buffer_position(e, save_load);
    e.with_stack(4, |e, zero| {
        e.call(SAVE_BYTES, &args![save_load, zero, 2u32]);
    });
    let mut node = faction_list_head(this);
    while node != 0 {
        let entry = node_item(e, node);
        if entry != 0 {
            let faction = e.mem.u32(entry);
            let numeric_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, faction));
            let rank = e.mem.u8(entry.wrapping_add(4));
            with_item(e, numeric_id, |e, id_slot| {
                e.call(SAVE_NUMERIC_ID, &args![save_load, id_slot, 4u32]);
            });
            e.with_stack(4, |e, rank_slot| {
                e.mem.set_u8(rank_slot.addr(), rank);
                e.call(SAVE_BYTES, &args![save_load, rank_slot, 1u32]);
            });
            count = count.wrapping_add(1);
        }
        node = node_next(e, node);
    }
    e.mem.set_u16(count_slot, count);
    if save_debug_enabled(e) {
        let end = buffer_position(e, save_load);
        log_save_function(
            e,
            save_load,
            end.wrapping_sub(start),
            0xc77,
            TEXT_SAVE_FORM,
            TEXT_SAVE_PLAIN,
        );
    }
    if e.call(USE_SAVE_GAME_BLOCKS, &args![save_load]).bool() {
        let end = buffer_position(e, save_load);
        if end > block_size_slot.wrapping_add(0xffff) {
            e.call(
                MESSAGE_LOG,
                &args![TEXT_SAVE_BLOCK_TOO_BIG, SOURCE_FILE_PATH, 0xc77u32],
            );
        }
        e.mem
            .set_u16(block_size_slot, low_half(end.wrapping_sub(block_size_slot)));
    }
}

/// One of the load's buffer messages: with a current load entry, naming its form (slot 0x130)
/// and ending with the entry's version byte (+9) and flags (+5); else ending with the save's
/// version byte. `amount` is the overrun or underrun.
fn log_load_message(
    e: &mut Engine,
    save_load: u32,
    entry: u32,
    texts: (u32, u32),
    amount: Option<u32>,
    line: u32,
) {
    let mut words = Vec::with_capacity(9);
    if entry != 0 {
        let form_id = e.mem.u32(entry);
        let form = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, form_id));
        let name = e.vcall(form, 0x130, &args![]).u32();
        words.push(texts.1);
        words.extend(amount);
        words.extend([SOURCE_FILE_PATH, line, form_id, name]);
        words.push(u32::from(e.mem.u8(entry.wrapping_add(9))));
        words.push(e.mem.u32(entry.wrapping_add(5)));
    } else {
        let version = u32::from(e.mem.u8(save_load.wrapping_add(SAVE_LOAD_VERSION)));
        words.push(texts.0);
        words.extend(amount);
        words.extend([SOURCE_FILE_PATH, line, version]);
    }
    e.call(MESSAGE_LOG, &words);
}

// Typed from 0047e6f0
/// `TESActorBaseData::LoadGame` (Xbox PDB): from the save buffer, the `KOBL` block header (logged
/// when it differs) and the 16-bit block size with save-game blocks in use, the 0x18 bytes of the
/// data with bit 1 of `flags`, then the faction list replaced by the stored entries (a 16-bit
/// count; per entry a numeric form id and a rank byte; an entry whose form does not resolve is
/// dropped); with blocks, a buffer overrun or underrun against the block size logged. The second
/// word is not read.
pub fn load_game(e: &mut Engine, this: u32, flags: u32, _unused: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME_GLOBAL);
    let mut block_size = 0u16;
    let mut block_start = 0u32;
    if e.call(USE_SAVE_GAME_BLOCKS, &args![save_load]).bool() {
        let tag = e.with_stack(4, |e, tag| {
            e.call(LOAD_BYTES, &args![save_load, tag, 4u32]);
            e.mem.u32(tag.addr())
        });
        if tag != TAG_KOBL {
            let entry = e.mem.u32(save_load.wrapping_add(SAVE_LOAD_LOAD_ENTRY));
            log_load_message(
                e,
                save_load,
                entry,
                (TEXT_LOAD_HEADER_PLAIN, TEXT_LOAD_HEADER_FORM),
                None,
                0xc80,
            );
        }
        block_start = buffer_position(e, save_load);
        block_size = e.with_stack(4, |e, size| {
            e.mem.set_u16(size.addr(), 0);
            e.call(LOAD_BYTES, &args![save_load, size, 2u32]);
            e.mem.u16(size.addr())
        });
    }
    if flags & 2 != 0 {
        e.call(LOAD_BYTES, &args![save_load, at(this, BASE_FLAGS), 0x18u32]);
    }
    typed_call!(FAMILY, 0x0047_c9d0, clear_faction_list(e, this));
    let count = e.with_stack(4, |e, count| {
        e.call(LOAD_BYTES, &args![save_load, count, 2u32]);
        e.mem.u16(count.addr())
    });
    for _ in 0..count {
        let (id, rank) = e.with_stack(8, |e, slot| {
            let slot = slot.addr();
            e.call(LOAD_NUMERIC_ID, &args![save_load, slot, 4u32]);
            e.call(LOAD_BYTES, &args![save_load, slot + 4, 1u32]);
            (e.mem.u32(slot), e.mem.u8(slot + 4))
        });
        let faction = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id));
        if faction != 0 {
            let entry = e.call(OPERATOR_NEW, &args![8u32]).u32();
            e.mem.set_u32(entry, faction);
            e.mem.set_u8(entry.wrapping_add(4), rank);
            with_item(e, entry, |e, slot| {
                let head = faction_list_head(this);
                e.call(LIST_TAKE_HEAD, &args![head, slot]);
            });
        }
    }
    if e.call(USE_SAVE_GAME_BLOCKS, &args![save_load]).bool() {
        let position = buffer_position(e, save_load);
        let entry = e.mem.u32(save_load.wrapping_add(SAVE_LOAD_LOAD_ENTRY));
        let expected = u32::from(block_size).wrapping_add(block_start);
        if position > expected {
            log_load_message(
                e,
                save_load,
                entry,
                (TEXT_LOAD_OVERRUN_PLAIN, TEXT_LOAD_OVERRUN_FORM),
                Some(position - expected),
                0xca2,
            );
        } else if position < expected {
            log_load_message(
                e,
                save_load,
                entry,
                (TEXT_LOAD_UNDERRUN_PLAIN, TEXT_LOAD_UNDERRUN_FORM),
                Some(expected - position),
                0xca2,
            );
        }
    }
}

// Typed from 004a8ae0
/// `TESActorBaseData::GetFatigue` (Xbox PDB): the fatigue (+8).
#[inline]
pub fn fatigue(e: &mut Engine, this: u32) -> u16 {
    get_u16(e, this, FATIGUE)
}
