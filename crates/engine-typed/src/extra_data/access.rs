//! The getters and setters of single extra data (`004181e0` to `0041a540`): each finds the first
//! extra data of its type with the dispatch family's lookup and reads it, or writes into it, deletes
//! it or builds and adds one (`operator new` and the type's constructor by address, then `AddExtra`).
//! With them the sound handle's constructors, copy and compares, done in place where the setters use
//! them, and the destructors of the data the setters replace.

use super::{
    cell_refs, extra_init, first_of_kind, free_when, has_kind, list_add, list_remove, new_object,
    read_word, records, word_at, zero_bytes, EXTRA_PAYLOAD, FAMILY, OPERATOR_DELETE, OPERATOR_NEW,
};
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// `-1.0f` (the health and charge getters' default) and the `double` 1.0 the scale setter compares
/// with, in the exe's read-only data.
pub(super) const MINUS_ONE: u32 = 0x0101_2054;
pub(super) const ONE_DOUBLE: u32 = 0x0101_2070;
/// `ExtraRadioData`'s vtable and the copy of a `RADIO_DATA` record (`thiscall(dst, src)`).
const VTABLE_EXTRA_RADIO_DATA: u32 = 0x0101_5138;
const RADIO_DATA_COPY: u32 = 0x0043_7240;
/// No sound: a sound handle's id when it holds none.
const NO_SOUND: u32 = 0xffff_ffff;

/// The extra data types (`cEtype`) the getters and setters ask for.
const KIND_SCRIPT: u8 = 0x0d;
const KIND_ACTION: u8 = 0x0e;
const KIND_STARTING_POSITION: u8 = 0x0f;
const KIND_ANIM: u8 = 0x10;
const KIND_CONTAINER_CHANGES: u8 = 0x15;
const KIND_WORN: u8 = 0x16;
const KIND_WORN_LEFT: u8 = 0x17;
const KIND_PACKAGE_START_LOCATION: u8 = 0x18;
const KIND_GHOST: u8 = 0x1f;
const KIND_ORIGINAL_REFERENCE: u8 = 0x20;
const KIND_OWNERSHIP: u8 = 0x21;
const KIND_GLOBAL: u8 = 0x22;
const KIND_RANK: u8 = 0x23;
const KIND_COUNT: u8 = 0x24;
const KIND_HEALTH: u8 = 0x25;
const KIND_USES: u8 = 0x26;
const KIND_TIME_LEFT: u8 = 0x27;
const KIND_CHARGE: u8 = 0x28;
const KIND_LIGHT: u8 = 0x29;
const KIND_LOCK: u8 = 0x2a;
const KIND_TELEPORT: u8 = 0x2b;
const KIND_MAP_MARKER: u8 = 0x2c;
const KIND_ANIM_SAVE: u8 = 0x2d;
const KIND_LEVELED_ITEM: u8 = 0x2f;
const KIND_SCALE: u8 = 0x30;
const KIND_SEED: u8 = 0x31;
const KIND_POISON: u8 = 0x3f;
const KIND_SPELL_EFFECT_LIGHT: u8 = 0x40;
const KIND_ITEM_DROPPER: u8 = 0x46;
const KIND_SOUND: u8 = 0x4f;
const KIND_RADIO_DATA: u8 = 0x68;
const KIND_CREATURE_AWAKE_SOUND: u8 = 0x7d;
const KIND_WEAPON_IDLE_SOUND: u8 = 0x83;
const KIND_WEAPON_ATTACK_SOUND: u8 = 0x86;
const KIND_ACTIVATE_LOOP_SOUND: u8 = 0x87;
const KIND_CREATURE_MOVEMENT_SOUND: u8 = 0x8a;
const KIND_WEAPON_MOD_SLOTS: u8 = 0x8d;
const KIND_AUDIO_MARKER: u8 = 0x90;
const KIND_AUDIO_BUOY_MARKER: u8 = 0x91;

/// The constructors the setters build their extra data with (`thiscall(block, value)`).
const EXTRA_ANIM_INIT: u32 = 0x0043_00c0;
const EXTRA_STARTING_POSITION_INIT: u32 = 0x0043_0780;
const EXTRA_LOCK_INIT: u32 = 0x0043_0c70;
const EXTRA_TELEPORT_INIT: u32 = 0x0043_1230;
const EXTRA_MAP_MARKER_INIT: u32 = 0x0043_1360;
const EXTRA_AUDIO_MARKER_INIT: u32 = 0x0043_14c0;
const EXTRA_AUDIO_BUOY_MARKER_INIT: u32 = 0x0043_1620;
const EXTRA_ACTION_INIT: u32 = 0x0043_1780;
const EXTRA_CONTAINER_CHANGES_INIT: u32 = 0x0043_1830;
const EXTRA_ORIGINAL_REFERENCE_INIT: u32 = 0x0043_1950;
const EXTRA_OWNERSHIP_INIT: u32 = 0x0043_1a40;
const EXTRA_GLOBAL_INIT: u32 = 0x0043_1ae0;
const EXTRA_RANK_INIT: u32 = 0x0043_1b80;
const EXTRA_COUNT_INIT: u32 = 0x0043_1c20;
const EXTRA_HEALTH_INIT: u32 = 0x0043_1d10;
const EXTRA_USES_INIT: u32 = 0x0043_1e20;
const EXTRA_TIME_LEFT_INIT: u32 = 0x0043_1ec0;
const EXTRA_CHARGE_INIT: u32 = 0x0043_1f60;
const EXTRA_SCRIPT_INIT: u32 = 0x0043_2000;
const EXTRA_SCALE_INIT: u32 = 0x0043_2220;
const EXTRA_POISON_INIT: u32 = 0x0043_5150;
const EXTRA_ITEM_DROPPER_INIT: u32 = 0x0043_5ec0;
const EXTRA_CREATURE_AWAKE_SOUND_INIT: u32 = 0x0043_61e0;
const EXTRA_CREATURE_MOVEMENT_SOUND_INIT: u32 = 0x0043_6300;
const EXTRA_WEAPON_IDLE_SOUND_INIT: u32 = 0x0043_6420;
const EXTRA_WEAPON_ATTACK_SOUND_INIT: u32 = 0x0043_6540;
/// What the setters run on the data they replace or delete: the animation's scalar deleting
/// destructor (`this`, flags), the teleport, map marker and audio marker data's destructors
/// (`this`), the audio buoy marker data's scalar deleting destructor (`this`, flags) and the
/// reference's `SetTargeted(bool)`.
const ANIM_DELETE: u32 = 0x0041_8d20;
const TELEPORT_DATA_DROP: u32 = 0x0066_65a0;
const MAP_MARKER_DATA_DROP: u32 = 0x0043_8bf0;
const AUDIO_MARKER_DATA_DROP: u32 = 0x0058_9500;
const AUDIO_BUOY_MARKER_DATA_DELETE: u32 = 0x007b_3fa0;
const REFERENCE_SET_TARGETED: u32 = 0x0056_4db0;

// ---- What the getters and setters share ------------------------------------------------------

/// The word at +0xc of the first extra data of `kind`, or `default`.
#[inline]
pub(super) fn word_or(e: &mut Engine, list: u32, kind: u8, default: u32) -> u32 {
    match first_of_kind(e, list, kind) {
        0 => default,
        extra => word_at(e, extra, EXTRA_PAYLOAD),
    }
}

/// The `float` at +0xc of the first extra data of `kind`, or the one of the bits `default`; through
/// the x87's load, so a signalling NaN comes out quiet.
#[inline]
pub(super) fn float_or(e: &mut Engine, list: u32, kind: u8, default: u32) -> f32 {
    f32::from_bits(x87_quiet_bits(word_or(e, list, kind, default)))
}

/// The address of the record at +0xc of the first extra data of `kind`, or 0.
#[inline]
fn record_or_null(e: &mut Engine, list: u32, kind: u8) -> u32 {
    match first_of_kind(e, list, kind) {
        0 => 0,
        extra => extra.wrapping_add(EXTRA_PAYLOAD),
    }
}

/// The empty sound handle stored at `at` (`004188d0`, `0041a250`): no sound, success not assumed,
/// state 0.
#[inline]
fn store_empty_handle(e: &mut Engine, at: u32) {
    e.mem.set_u32(at, NO_SOUND);
    e.mem.set_u8(at.wrapping_add(4), 0);
    e.mem.set_u32(at.wrapping_add(8), 0);
}

/// A sound handle copied from `source` to `at` (`00418900`): the id, the success byte, the state.
#[inline]
fn copy_handle(e: &mut Engine, at: u32, source: u32) {
    let id = e.mem.u32(source);
    e.mem.set_u32(at, id);
    let assume_success = e.mem.u8(source.wrapping_add(4));
    e.mem.set_u8(at.wrapping_add(4), assume_success);
    let state = e.mem.u32(source.wrapping_add(8));
    e.mem.set_u32(at.wrapping_add(8), state);
}

/// A sound getter: the handle at +0xc of the first extra data of `kind` copied to `out`, or the
/// empty handle stored. Answers `out`.
fn sound_or_empty(e: &mut Engine, list: u32, kind: u8, out: u32) -> u32 {
    match first_of_kind(e, list, kind) {
        0 => store_empty_handle(e, out),
        extra => copy_handle(e, out, extra.wrapping_add(EXTRA_PAYLOAD)),
    }
    out
}

/// A new extra data of `size` bytes built by the constructor at `construct` (with `value` when it
/// takes one) and added to the list, a null one too when the allocation failed. Answers it.
pub(super) fn add_new(
    e: &mut Engine,
    list: u32,
    size: u32,
    construct: u32,
    value: Option<u32>,
) -> u32 {
    let extra = new_object(e, size, |e, block| match value {
        Some(value) => e.call(construct, &args![block, value]).u32(),
        None => e.call(construct, &args![block]).u32(),
    });
    list_add(e, list, extra);
    extra
}

/// The plain setters: `value` stored by `store` in the first extra data of `kind`, or one of `size`
/// bytes built from it and added.
fn set_or_add(
    e: &mut Engine,
    list: u32,
    kind: u8,
    size: u32,
    construct: u32,
    value: u32,
    store: impl FnOnce(&mut Engine, u32),
) {
    match first_of_kind(e, list, kind) {
        0 => {
            add_new(e, list, size, construct, Some(value));
        }
        extra => store(e, extra),
    }
}

/// The word at +0xc set to `value` in the first extra data of `kind`, or one (0x10 bytes) added.
pub(super) fn set_word_or_add(e: &mut Engine, list: u32, kind: u8, construct: u32, value: u32) {
    set_or_add(e, list, kind, 0x10, construct, value, |e, extra| {
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), value);
    });
}

/// The setters whose `none` means no extra data: the one there deleted for `none`, its word at
/// +0xc set otherwise; a new one (0x10 bytes) unless the value is `none`.
pub(super) fn set_or_remove(
    e: &mut Engine,
    list: u32,
    kind: u8,
    construct: u32,
    value: u32,
    none: u32,
) {
    match first_of_kind(e, list, kind) {
        0 => {
            if value != none {
                add_new(e, list, 0x10, construct, Some(value));
            }
        }
        extra if value == none => list_remove(e, list, extra, true),
        extra => e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), value),
    }
}

/// The setters of a pointer the extra data owns: a null `value` deletes the one there (answering it
/// when `keep_deleted`, else null), another replaces the pointer held (`destroy_old` run on a
/// non-null old one first) or builds the extra data. Answers the extra data.
fn set_owned_pointer(
    e: &mut Engine,
    list: u32,
    kind: u8,
    construct: u32,
    value: u32,
    keep_deleted: bool,
    destroy_old: impl FnOnce(&mut Engine, u32),
) -> u32 {
    let extra = first_of_kind(e, list, kind);
    if extra == 0 {
        return if value == 0 {
            0
        } else {
            add_new(e, list, 0x10, construct, Some(value))
        };
    }
    if value == 0 {
        list_remove(e, list, extra, true);
        return if keep_deleted { extra } else { 0 };
    }
    let old = word_at(e, extra, EXTRA_PAYLOAD);
    if old != 0 {
        destroy_old(e, old);
    }
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), value);
    extra
}

/// The body of the sound setters (four here, two in [`super::setters`]). With an extra data of `kind`: a handle whose id is an empty
/// handle's deletes it, any other is copied in. Without one, a handle with another id builds one of
/// 0x18 bytes with `construct`, the handle by value (its id, its success byte zero-extended as the
/// exe's zeroed stack copy holds it, its state), and adds it. The exe's empty handles on the stack
/// and their compares (`0041a250`, `0041a1f0`, `0041a220`, the empty destructor `00483710`) and the
/// handle's copy (`00418900`) are done in place.
pub(super) fn sound_setter(e: &mut Engine, list: u32, kind: u8, construct: u32, sound: u32) {
    let extra = first_of_kind(e, list, kind);
    let empty = read_word(e, sound) == NO_SOUND;
    if extra != 0 {
        if empty {
            list_remove(e, list, extra, true);
        } else {
            copy_handle(e, extra.wrapping_add(EXTRA_PAYLOAD), sound);
        }
        return;
    }
    if empty {
        return;
    }
    let block = e.call(OPERATOR_NEW, &args![0x18u32]).u32();
    let created = if block == 0 {
        0
    } else {
        let id = e.mem.u32(sound);
        let assume_success = u32::from(e.mem.u8(sound.wrapping_add(4)));
        let state = e.mem.u32(sound.wrapping_add(8));
        e.call(construct, &args![block, id, assume_success, state])
            .u32()
    };
    list_add(e, list, created);
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 004181e0
/// A reference's base form: `BGSSaveFormBuffer::GetForm`'s word at +0x20 (`007af430`), in place.
pub fn save_form(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(0x20))
}

// Typed from 00418200
/// `ExtraDataList::GetAnimSave` (Xbox PDB): the type 0x2d extra data, or null.
pub fn anim_save(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_ANIM_SAVE)
}

// Typed from 00418220
/// `ExtraDataList::GetAnimation` (Xbox PDB): the word at +0xc of the type 0x10 extra data, or 0.
pub fn animation(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_ANIM, 0)
}

// Typed from 00418250
/// The word at +0xc of the type 0x29 extra data (the light), or 0.
pub fn light(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_LIGHT, 0)
}

// Typed from 00418280
/// `ExtraDataList::GetSpellEffectLight` (Xbox PDB): the word at +0xc of the type 0x40 extra data,
/// or 0.
pub fn spell_effect_light(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_SPELL_EFFECT_LIGHT, 0)
}

// Typed from 004182b0
/// The word at +0xc of the type 0x2a extra data (the lock), or 0.
pub fn lock_data(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_LOCK, 0)
}

// Typed from 004182e0
/// The address of the `RADIO_DATA` record (+0xc) of the type 0x68 extra data, or 0.
pub fn radio_data(e: &mut Engine, this: u32) -> u32 {
    record_or_null(e, this, KIND_RADIO_DATA)
}

// Typed from 00418310
/// The list's radio data set from the record at `data`: a null one removes the type 0x68 extra
/// data; otherwise the record is copied (`00437240`) into the one there, or into a new
/// `ExtraRadioData` (0x1c bytes, [`radio_extra_new`]) that is added, a failed allocation's null one
/// too (the copy then goes to +0xc of null, as the exe's does).
pub fn set_radio_data(e: &mut Engine, this: u32, data: u32) {
    if data == 0 {
        typed_call!(
            cell_refs::FAMILY,
            0x0041_0140,
            cell_refs::extra_list::remove_extra_of_type(e, this, KIND_RADIO_DATA)
        );
        return;
    }
    let existing = first_of_kind(e, this, KIND_RADIO_DATA);
    if existing != 0 {
        e.call(
            RADIO_DATA_COPY,
            &args![existing.wrapping_add(EXTRA_PAYLOAD), data],
        );
        return;
    }
    let extra = new_object(e, 0x1c, |e, block| {
        typed_call!(FAMILY, 0x0041_83e0, radio_extra_new(e, block))
    });
    e.call(
        RADIO_DATA_COPY,
        &args![extra.wrapping_add(EXTRA_PAYLOAD), data],
    );
    list_add(e, this, extra);
}

// Typed from 004183e0
/// `ExtraRadioData`'s constructor (type 0x68, 0x1c bytes): the base with the type, its vtable,
/// `RADIO_DATA`'s constructor on the record at +0xc and then the record's 16 bytes zeroed. Answers
/// `this`.
pub fn radio_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_init(e, this, KIND_RADIO_DATA);
    e.mem.set_u32(this, VTABLE_EXTRA_RADIO_DATA);
    let record = this.wrapping_add(EXTRA_PAYLOAD);
    typed_call!(FAMILY, 0x0041_6b40, records::radio_data_new(e, record));
    zero_bytes(e, record, 0x10);
    this
}

// Typed from 00418460
/// The word at +0xc of the type 0x2b extra data (the teleport), or 0.
pub fn teleport(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_TELEPORT, 0)
}

// Typed from 00418490
/// The word at +0xc of the type 0x2c extra data (the map marker), or 0.
pub fn map_marker(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_MAP_MARKER, 0)
}

// Typed from 004184c0
/// The word at +0xc of the type 0x90 extra data (the audio marker), or 0.
pub fn audio_marker(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_AUDIO_MARKER, 0)
}

// Typed from 004184f0
/// The word at +0xc of the type 0x91 extra data (the audio buoy marker), or 0.
pub fn audio_buoy_marker(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_AUDIO_BUOY_MARKER, 0)
}

// Typed from 00418520
/// `ExtraDataList::GetContainerChanges` (Xbox PDB): the word at +0xc of the type 0x15 extra data,
/// or 0.
pub fn container_changes(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_CONTAINER_CHANGES, 0)
}

// Typed from 00418550
/// The list's original reference set: stored in the type 0x20 extra data there, and then, in every
/// case, a new one (0x10 bytes, `00431950`) built and added.
pub fn set_original_reference(e: &mut Engine, this: u32, reference: u32) {
    let existing = first_of_kind(e, this, KIND_ORIGINAL_REFERENCE);
    if existing != 0 {
        e.mem
            .set_u32(existing.wrapping_add(EXTRA_PAYLOAD), reference);
    }
    add_new(
        e,
        this,
        0x10,
        EXTRA_ORIGINAL_REFERENCE_INIT,
        Some(reference),
    );
}

// Typed from 00418600
/// `ExtraDataList::RemoveOriginalReferenceExtra` (Xbox PDB): the type 0x20 extra data removed and
/// deleted, if there is one.
pub fn remove_original_reference(e: &mut Engine, this: u32) {
    let extra = first_of_kind(e, this, KIND_ORIGINAL_REFERENCE);
    if extra != 0 {
        list_remove(e, this, extra, true);
    }
}

// Typed from 00418630
/// `ExtraDataList::GetOriginalReference` (Xbox PDB): the word at +0xc of the type 0x20 extra data,
/// or 0.
pub fn original_reference(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_ORIGINAL_REFERENCE, 0)
}

// Typed from 00418660
/// The word at +0xc of the type 0x21 extra data (the owner), or 0.
pub fn ownership(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_OWNERSHIP, 0)
}

// Typed from 00418690
/// `ExtraDataList::GetGlobal` (Xbox PDB): the word at +0xc of the type 0x22 extra data, or 0.
pub fn global(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_GLOBAL, 0)
}

// Typed from 004186c0
/// `ExtraDataList::GetRank` (Xbox PDB): the rank of the type 0x23 extra data, or -1.
pub fn rank(e: &mut Engine, this: u32) -> i32 {
    word_or(e, this, KIND_RANK, u32::MAX) as i32
}

// Typed from 004186f0
/// `ExtraDataList::GetHealth` (Xbox PDB): the health of the type 0x25 extra data, or the -1.0 at
/// `01012054` (in `ST0`).
pub fn health(e: &mut Engine, this: u32) -> f32 {
    let default = e.mem.u32(MINUS_ONE);
    float_or(e, this, KIND_HEALTH, default)
}

// Typed from 00418720
/// `ExtraDataList::GetLeveledItem` (Xbox PDB): the type 0x2f extra data, or null.
pub fn leveled_item(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_LEVELED_ITEM)
}

// Typed from 00418750
/// `ExtraDataList::HasLeveledItem` (Xbox PDB): the type 0x2f bit (`0040fe80`, in place).
pub fn has_leveled_item(e: &mut Engine, this: u32) -> bool {
    has_kind(e, this, KIND_LEVELED_ITEM)
}

// Typed from 00418770
/// `ExtraDataList::GetCount` (Xbox PDB): the count (a `short`) of the type 0x24 extra data, or 1
/// (in `AX`).
pub fn count(e: &mut Engine, this: u32) -> u16 {
    crate::crt::low_half(word_or(e, this, KIND_COUNT, 1))
}

// Typed from 004187a0
/// The charge of the type 0x28 extra data, or the -1.0 at `01012054` (in `ST0`).
pub fn charge(e: &mut Engine, this: u32) -> f32 {
    let default = e.mem.u32(MINUS_ONE);
    float_or(e, this, KIND_CHARGE, default)
}

// Typed from 004187d0
/// `ExtraDataList::GetPoison` (Xbox PDB): the word at +0xc of the type 0x3f extra data, or 0.
pub fn poison(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_POISON, 0)
}

// Typed from 00418800
/// `ExtraDataList::GetScript` (Xbox PDB): the word at +0xc of the type 0xd extra data, or 0.
pub fn script(e: &mut Engine, this: u32) -> u32 {
    word_or(e, this, KIND_SCRIPT, 0)
}

// Typed from 00418830
/// `ExtraDataList::GetScriptLocals` (Xbox PDB): the word at +0x10 of the type 0xd extra data, or 0.
pub fn script_locals(e: &mut Engine, this: u32) -> u32 {
    match first_of_kind(e, this, KIND_SCRIPT) {
        0 => 0,
        extra => word_at(e, extra, 0x10),
    }
}

// Typed from 00418860
/// The scale of the type 0x30 extra data, or 1.0 (in `ST0`).
pub fn scale(e: &mut Engine, this: u32) -> f32 {
    float_or(e, this, KIND_SCALE, 1.0f32.to_bits())
}

// Typed from 00418890
/// `ExtraDataList::GetSound` (Xbox PDB, `thiscall(list, out)`): the sound handle of the type 0x4f
/// extra data copied to `out`, or the empty handle. Answers `out`.
pub fn sound(e: &mut Engine, this: u32, out: u32) -> u32 {
    sound_or_empty(e, this, KIND_SOUND, out)
}

// Typed from 004188d0
/// `BSSoundHandle`'s default constructor: no sound (0xffffffff), success not assumed, state 0.
/// Answers `this`.
pub fn sound_handle_new(e: &mut Engine, this: u32) -> u32 {
    store_empty_handle(e, this);
    this
}

// Typed from 00418900
/// `BSSoundHandle`'s copy constructor: the id, the success byte and the state of `source`. Answers
/// `this`.
pub fn sound_handle_copy(e: &mut Engine, this: u32, source: u32) -> u32 {
    copy_handle(e, this, source);
    this
}

// Typed from 00418940
/// `ExtraDataList::GetCreatureAwakeSound` (Xbox PDB): as [`sound`] for type 0x7d.
pub fn creature_awake_sound(e: &mut Engine, this: u32, out: u32) -> u32 {
    sound_or_empty(e, this, KIND_CREATURE_AWAKE_SOUND, out)
}

// Typed from 00418980
/// As [`sound`] for type 0x8a (the creature's movement sound).
pub fn creature_movement_sound(e: &mut Engine, this: u32, out: u32) -> u32 {
    sound_or_empty(e, this, KIND_CREATURE_MOVEMENT_SOUND, out)
}

// Typed from 004189c0
/// `ExtraDataList::GetWeaponIdleSound` (Xbox PDB): as [`sound`] for type 0x83.
pub fn weapon_idle_sound(e: &mut Engine, this: u32, out: u32) -> u32 {
    sound_or_empty(e, this, KIND_WEAPON_IDLE_SOUND, out)
}

// Typed from 00418a00
/// `ExtraDataList::GetWeaponAttackSound` (Xbox PDB): as [`sound`] for type 0x86.
pub fn weapon_attack_sound(e: &mut Engine, this: u32, out: u32) -> u32 {
    sound_or_empty(e, this, KIND_WEAPON_ATTACK_SOUND, out)
}

// Typed from 00418a40
/// `ExtraDataList::GetActivateLoopSound` (Xbox PDB): as [`sound`] for type 0x87.
pub fn activate_loop_sound(e: &mut Engine, this: u32, out: u32) -> u32 {
    sound_or_empty(e, this, KIND_ACTIVATE_LOOP_SOUND, out)
}

// Typed from 00418a80
/// Whether the list has a type 0x1f extra data (the ghost): its bit, in place.
pub fn is_ghost(e: &mut Engine, this: u32) -> bool {
    has_kind(e, this, KIND_GHOST)
}

// Typed from 00418ab0
/// `ExtraDataList::GetWorn` (Xbox PDB, `thiscall(list, left_only)`): the type 0x17 bit with
/// `left_only`, else the 0x16 or the 0x17 bit.
pub fn worn(e: &mut Engine, this: u32, left_only: bool) -> bool {
    if left_only {
        return has_kind(e, this, KIND_WORN_LEFT);
    }
    has_kind(e, this, KIND_WORN) || has_kind(e, this, KIND_WORN_LEFT)
}

// Typed from 00418b40
/// The seed (a byte at +0xc) of the type 0x31 extra data, or 0xff.
pub fn seed(e: &mut Engine, this: u32) -> u8 {
    match first_of_kind(e, this, KIND_SEED) {
        0 => 0xff,
        extra => e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD)),
    }
}

// Typed from 00418b70
/// `ExtraDataList::GetPackageStartLocation` (Xbox PDB): the address of the record at +0xc of the
/// type 0x18 extra data, or 0.
pub fn package_start_location(e: &mut Engine, this: u32) -> u32 {
    record_or_null(e, this, KIND_PACKAGE_START_LOCATION)
}

// Typed from 00418ba0
/// `ExtraDataList::HasWeaponMods` (Xbox PDB): false without a type 0x8d extra data, else whether its
/// byte at +0xc is nonzero (`00418be0`, in place).
pub fn has_weapon_mods(e: &mut Engine, this: u32) -> bool {
    match first_of_kind(e, this, KIND_WEAPON_MOD_SLOTS) {
        0 => false,
        extra => e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD)) != 0,
    }
}

// Typed from 00418be0
/// Whether the byte at +0xc of the extra data is nonzero.
pub fn weapon_mods_any(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this.wrapping_add(EXTRA_PAYLOAD)) != 0
}

// Typed from 00418c00
/// `ExtraDataList::GetWeaponModSlotActive` (Xbox PDB, `thiscall(list, slot)`): false without a type
/// 0x8d extra data, else whether its byte at +0xc has a bit of `slot` (`00411e20`, in place).
pub fn weapon_mod_slot_active(e: &mut Engine, this: u32, slot: u8) -> bool {
    match first_of_kind(e, this, KIND_WEAPON_MOD_SLOTS) {
        0 => false,
        extra => e.mem.u8(extra.wrapping_add(EXTRA_PAYLOAD)) & slot != 0,
    }
}

// Typed from 00418c40
/// `ExtraDataList::SetAnimPtr` (Xbox PDB): the type 0x10 extra data's animation set, the old one
/// deleted first (`00418d20` with 1) when there was one; or an `ExtraAnim` (0x10 bytes, `004300c0`)
/// added. Answers the extra data.
pub fn set_anim_ptr(e: &mut Engine, this: u32, animation: u32) -> u32 {
    let extra = first_of_kind(e, this, KIND_ANIM);
    if extra == 0 {
        return add_new(e, this, 0x10, EXTRA_ANIM_INIT, Some(animation));
    }
    let old = word_at(e, extra, EXTRA_PAYLOAD);
    if old != 0 {
        e.call(ANIM_DELETE, &args![old, 1u32]);
    }
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), animation);
    extra
}

// Typed from 00418d50
/// The type 0xf extra data (the starting position): the list's own, or one built from `position`
/// (0x24 bytes, `00430780`) and added.
pub fn starting_position(e: &mut Engine, this: u32, position: u32) -> u32 {
    match first_of_kind(e, this, KIND_STARTING_POSITION) {
        0 => add_new(e, this, 0x24, EXTRA_STARTING_POSITION_INIT, Some(position)),
        extra => extra,
    }
}

// Typed from 00419050
/// `ExtraDataList::SetLockPtr` (Xbox PDB): the type 0x2a extra data's lock record set, the old block
/// deleted first (`operator delete`, a null one too); or an `ExtraLock` (0x10 bytes, `00430c70`)
/// added. Answers the extra data.
pub fn set_lock_ptr(e: &mut Engine, this: u32, lock: u32) -> u32 {
    let extra = first_of_kind(e, this, KIND_LOCK);
    if extra == 0 {
        return add_new(e, this, 0x10, EXTRA_LOCK_INIT, Some(lock));
    }
    let old = word_at(e, extra, EXTRA_PAYLOAD);
    e.call(OPERATOR_DELETE, &args![old]);
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), lock);
    extra
}

// Typed from 00419120
/// The type 0x2b extra data's teleport data set (the old data deleted by [`teleport_data_delete`]);
/// a null `data` deletes the extra data and answers it, a deleted one.
pub fn set_teleport(e: &mut Engine, this: u32, data: u32) -> u32 {
    set_owned_pointer(
        e,
        this,
        KIND_TELEPORT,
        EXTRA_TELEPORT_INIT,
        data,
        true,
        |e, old| {
            typed_call!(FAMILY, 0x0041_9220, teleport_data_delete(e, old, 1));
        },
    )
}

// Typed from 00419220
/// The teleport data's scalar deleting destructor: `006665a0`, then `operator delete` with bit 0 of
/// `flags`. Answers `this`.
pub fn teleport_data_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(TELEPORT_DATA_DROP, &args![this]);
    free_when(e, this, flags)
}

// Typed from 00419250
/// The type 0x2c extra data's map marker data set (the old data deleted by
/// [`map_marker_data_delete`]); a null `data` deletes the extra data.
pub fn set_map_marker(e: &mut Engine, this: u32, data: u32) -> u32 {
    set_owned_pointer(
        e,
        this,
        KIND_MAP_MARKER,
        EXTRA_MAP_MARKER_INIT,
        data,
        false,
        |e, old| {
            typed_call!(FAMILY, 0x0041_9350, map_marker_data_delete(e, old, 1));
        },
    )
}

// Typed from 00419350
/// The map marker data's scalar deleting destructor: `00438bf0`, then `operator delete` with bit 0
/// of `flags`. Answers `this`.
pub fn map_marker_data_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(MAP_MARKER_DATA_DROP, &args![this]);
    free_when(e, this, flags)
}

// Typed from 00419380
/// The type 0x90 extra data's audio marker data set (the old data deleted by
/// [`audio_marker_data_delete`]); a null `data` deletes the extra data.
pub fn set_audio_marker(e: &mut Engine, this: u32, data: u32) -> u32 {
    set_owned_pointer(
        e,
        this,
        KIND_AUDIO_MARKER,
        EXTRA_AUDIO_MARKER_INIT,
        data,
        false,
        |e, old| {
            typed_call!(FAMILY, 0x0041_9480, audio_marker_data_delete(e, old, 1));
        },
    )
}

// Typed from 00419480
/// The audio marker data's scalar deleting destructor: `00589500`, then `operator delete` with bit 0
/// of `flags`. Answers `this`.
pub fn audio_marker_data_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(AUDIO_MARKER_DATA_DROP, &args![this]);
    free_when(e, this, flags)
}

// Typed from 004194b0
/// The type 0x91 extra data's audio buoy marker data set (the old data deleted by its scalar
/// deleting destructor `007b3fa0` with 1); a null `data` deletes the extra data.
pub fn set_audio_buoy_marker(e: &mut Engine, this: u32, data: u32) -> u32 {
    set_owned_pointer(
        e,
        this,
        KIND_AUDIO_BUOY_MARKER,
        EXTRA_AUDIO_BUOY_MARKER_INIT,
        data,
        false,
        |e, old| {
            e.call(AUDIO_BUOY_MARKER_DATA_DELETE, &args![old, 1u32]);
        },
    )
}

// Typed from 004195b0
/// The type 0xe extra data (the action): the list's own, or one built (0x14 bytes, `00431780`, no
/// argument) and added.
pub fn action(e: &mut Engine, this: u32) -> u32 {
    match first_of_kind(e, this, KIND_ACTION) {
        0 => add_new(e, this, 0x14, EXTRA_ACTION_INIT, None),
        extra => extra,
    }
}

// Typed from 00419650
/// `ExtraDataList::SetContainerChanges` (Xbox PDB): nothing for null `changes`; else stored in the
/// type 0x15 extra data, or one (0x10 bytes, `00431830`) added.
pub fn set_container_changes(e: &mut Engine, this: u32, changes: u32) {
    if changes != 0 {
        set_word_or_add(
            e,
            this,
            KIND_CONTAINER_CHANGES,
            EXTRA_CONTAINER_CHANGES_INIT,
            changes,
        );
    }
}

// Typed from 00419700
/// The type 0x21 extra data's owner set; a null owner deletes it.
pub fn set_ownership(e: &mut Engine, this: u32, owner: u32) {
    set_or_remove(e, this, KIND_OWNERSHIP, EXTRA_OWNERSHIP_INIT, owner, 0);
}

// Typed from 004197d0
/// The type 0x22 extra data's global set; a null global deletes it.
pub fn set_global(e: &mut Engine, this: u32, global: u32) {
    set_or_remove(e, this, KIND_GLOBAL, EXTRA_GLOBAL_INIT, global, 0);
}

// Typed from 004198a0
/// The type 0x23 extra data's rank set; -1 deletes it.
pub fn set_rank(e: &mut Engine, this: u32, rank: i32) {
    set_or_remove(e, this, KIND_RANK, EXTRA_RANK_INIT, rank as u32, u32::MAX);
}

// Typed from 00419970
/// `ExtraDataList::SetHealth` (Xbox PDB): stored in the type 0x25 extra data, or one (0x10 bytes,
/// `00431d10`) added; the `float` through the x87 (a signalling NaN comes out quiet).
pub fn set_health(e: &mut Engine, this: u32, health: f32) {
    let bits = x87_quiet_bits(health.to_bits());
    set_word_or_add(e, this, KIND_HEALTH, EXTRA_HEALTH_INIT, bits);
}

// Typed from 00419a20
/// The uses (a byte at +0xc) stored in the type 0x26 extra data, or one (0x10 bytes, `00431e20`)
/// added.
pub fn set_uses(e: &mut Engine, this: u32, uses: u8) {
    set_or_add(
        e,
        this,
        KIND_USES,
        0x10,
        EXTRA_USES_INIT,
        u32::from(uses),
        |e, extra| e.mem.set_u8(extra.wrapping_add(EXTRA_PAYLOAD), uses),
    );
}

// Typed from 00419ad0
/// `ExtraDataList::SetCount` (Xbox PDB, the count a `short`): 0 or 1 deletes the type 0x24 extra
/// data; any other count is stored at +0xc, or one (0x10 bytes, `00431c20`, given the count
/// zero-extended) added.
pub fn set_count(e: &mut Engine, this: u32, count: u16) {
    let extra = first_of_kind(e, this, KIND_COUNT);
    if count <= 1 {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
    } else if extra == 0 {
        add_new(e, this, 0x10, EXTRA_COUNT_INIT, Some(u32::from(count)));
    } else {
        e.mem.set_u16(extra.wrapping_add(EXTRA_PAYLOAD), count);
    }
}

// Typed from 00419bb0
/// The time left stored in the type 0x27 extra data, or one (0x10 bytes, `00431ec0`) added; the
/// `float` through the x87.
pub fn set_time_left(e: &mut Engine, this: u32, time_left: f32) {
    let bits = x87_quiet_bits(time_left.to_bits());
    set_word_or_add(e, this, KIND_TIME_LEFT, EXTRA_TIME_LEFT_INIT, bits);
}

// Typed from 00419c60
/// The charge stored in the type 0x28 extra data, or one (0x10 bytes, `00431f60`) added; the
/// `float` through the x87.
pub fn set_charge(e: &mut Engine, this: u32, charge: f32) {
    let bits = x87_quiet_bits(charge.to_bits());
    set_word_or_add(e, this, KIND_CHARGE, EXTRA_CHARGE_INIT, bits);
}

// Typed from 00419d10
/// The poison stored in the type 0x3f extra data, or one (0x10 bytes, `00435150`) added.
pub fn set_poison(e: &mut Engine, this: u32, poison: u32) {
    set_word_or_add(e, this, KIND_POISON, EXTRA_POISON_INIT, poison);
}

// Typed from 00419dc0
/// `ExtraDataList::SetItemDropper` (Xbox PDB, type 0x46): a null dropper deletes the one there and
/// does nothing more; otherwise (a null one with none there too) the reference is stored, or one
/// (0x10 bytes, `00435ec0`) added; a non-null dropper is then targeted (`00564db0` with true).
pub fn set_item_dropper(e: &mut Engine, this: u32, dropper: u32) {
    let extra = first_of_kind(e, this, KIND_ITEM_DROPPER);
    if dropper == 0 && extra != 0 {
        list_remove(e, this, extra, true);
        return;
    }
    if extra == 0 {
        add_new(e, this, 0x10, EXTRA_ITEM_DROPPER_INIT, Some(dropper));
    } else {
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), dropper);
    }
    if dropper != 0 {
        e.call(REFERENCE_SET_TARGETED, &args![dropper, 1u32]);
    }
}

// Typed from 00419ed0
/// `ExtraDataList::SetScript` (Xbox PDB): stored in the type 0xd extra data, or one (0x14 bytes,
/// `00432000`) added.
pub fn set_script(e: &mut Engine, this: u32, script: u32) {
    set_or_add(
        e,
        this,
        KIND_SCRIPT,
        0x14,
        EXTRA_SCRIPT_INIT,
        script,
        |e, extra| e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), script),
    );
}

// Typed from 00419f80
/// The script's locals (+0x10) stored in the type 0xd extra data when there is one.
pub fn set_script_locals(e: &mut Engine, this: u32, locals: u32) {
    let extra = first_of_kind(e, this, KIND_SCRIPT);
    if extra != 0 {
        e.mem.set_u32(extra.wrapping_add(0x10), locals);
    }
}

// Typed from 00419fb0
/// `ExtraDataList::SetScale` (Xbox PDB): a scale equal to the `double` 1.0 at `01012070` deletes the
/// type 0x30 extra data; any other (a NaN too) is stored, or one (0x10 bytes, `00432220`) added; the
/// `float` through the x87.
pub fn set_scale(e: &mut Engine, this: u32, scale: f32) {
    let extra = first_of_kind(e, this, KIND_SCALE);
    let is_one = f64::from(scale) == e.mem.f64(ONE_DOUBLE);
    let bits = x87_quiet_bits(scale.to_bits());
    if extra == 0 {
        if !is_one {
            add_new(e, this, 0x10, EXTRA_SCALE_INIT, Some(bits));
        }
    } else if is_one {
        list_remove(e, this, extra, true);
    } else {
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), bits);
    }
}

// Typed from 0041a090
/// `ExtraDataList::SetCreatureAwakeSound` (Xbox PDB, `thiscall(list, handle)`): the type 0x7d extra
/// data's sound handle set (`sound_setter`'s cases; built by `004361e0`).
pub fn set_creature_awake_sound(e: &mut Engine, this: u32, sound: u32) {
    sound_setter(
        e,
        this,
        KIND_CREATURE_AWAKE_SOUND,
        EXTRA_CREATURE_AWAKE_SOUND_INIT,
        sound,
    );
}

// Typed from 0041a1f0
/// Whether two sound handles have the same id (each read as `00559450` reads it, `other` first).
pub fn same_sound_id(e: &mut Engine, this: u32, other: u32) -> bool {
    let other_id = read_word(e, other);
    read_word(e, this) == other_id
}

// Typed from 0041a220
/// Whether two sound handles have different ids (each read as `00559450` reads it, `other` first).
pub fn other_sound_id(e: &mut Engine, this: u32, other: u32) -> bool {
    let other_id = read_word(e, other);
    read_word(e, this) != other_id
}

// Typed from 0041a250
/// `BSSoundHandle`'s default constructor (as [`sound_handle_new`]). Answers `this`.
pub fn sound_handle_init(e: &mut Engine, this: u32) -> u32 {
    store_empty_handle(e, this);
    this
}

// Typed from 0041a280
/// As [`set_creature_awake_sound`] for type 0x8a (the creature's movement sound, built by
/// `00436300`).
pub fn set_creature_movement_sound(e: &mut Engine, this: u32, sound: u32) {
    sound_setter(
        e,
        this,
        KIND_CREATURE_MOVEMENT_SOUND,
        EXTRA_CREATURE_MOVEMENT_SOUND_INIT,
        sound,
    );
}

// Typed from 0041a3e0
/// `ExtraDataList::SetWeaponIdleSound` (Xbox PDB): as [`set_creature_awake_sound`] for type 0x83
/// (built by `00436420`).
pub fn set_weapon_idle_sound(e: &mut Engine, this: u32, sound: u32) {
    sound_setter(
        e,
        this,
        KIND_WEAPON_IDLE_SOUND,
        EXTRA_WEAPON_IDLE_SOUND_INIT,
        sound,
    );
}

// Typed from 0041a540
/// `ExtraDataList::SetWeaponAttackSound` (Xbox PDB): as [`set_creature_awake_sound`] for type 0x86
/// (built by `00436540`).
pub fn set_weapon_attack_sound(e: &mut Engine, this: u32, sound: u32) {
    sound_setter(
        e,
        this,
        KIND_WEAPON_ATTACK_SOUND,
        EXTRA_WEAPON_ATTACK_SOUND_INIT,
        sound,
    );
}
