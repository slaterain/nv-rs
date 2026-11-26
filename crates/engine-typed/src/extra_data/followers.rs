//! The part 4 file's first run (`00421400` to `00422c20`): the getters and setters of single extra
//! data (the merchant container 0x3c, the leveled creature modifier 0x1e and the leveled creature
//! 0x2e, the no-rumors flag 0x4e, the cell's detach time 0x0b, the seen data 0x05, the north
//! rotation 0x43, the X target 0x44, the encounter zone 0x74, the emittance source 0x67, the
//! multibound reference, data and bound 0x63, 0x62 and 0x61, the occlusion plane 0x71, the radius
//! 0x5c and radiation 0x5d with their constructors), the followers 0x1d and friend hits 0x45, the
//! refraction property 0x48, the last finished sequence 0x41 and the saved animation and havok data
//! 0x42 and 0x3d.
//!
//! Every setter is the compiler's one pattern: the neutral value removes the extra data of its type
//! by type (`RemoveExtra`, the cell references family's); otherwise the one there is updated, or a
//! new one built (`operator new`, the class's constructor) is given the value and added. A failed
//! allocation's null extra data is stored to and added as the exe does it. The radius and radiation
//! constructors (`004222f0`, `00422420`) are done in place where their setters build one; the other
//! units' constructors, the leveled creature modifier's and friend hits' methods, the string copy
//! of the last finished sequence and the save buffers' loaders are called by address.

use super::links::{add_unless_held, holds, new_by, remove_item};
use super::setters::remove_kind;
use super::{
    access, cell_refs, extra_init, first_of_kind, has_kind, list_add, list_remove, new_object,
    read_word, word_at, EXTRA_PAYLOAD, FAMILY, OPERATOR_DELETE,
};
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The extra data types (`cEtype`, Xbox PDB `EXTRA_DATA_TYPE`) of this file.
const KIND_SEEN_DATA: u8 = 0x05;
const KIND_CELL_DETACH_TIME: u8 = 0x0b;
const KIND_FOLLOWER: u8 = 0x1d;
const KIND_LEV_CREA_MOD: u8 = 0x1e;
const KIND_LEVELED_CREATURE: u8 = 0x2e;
const KIND_MERCHANT_CONTAINER: u8 = 0x3c;
const KIND_SAVED_HAVOK_DATA: u8 = 0x3d;
const KIND_LAST_FINISHED_SEQUENCE: u8 = 0x41;
const KIND_SAVED_ANIMATION: u8 = 0x42;
const KIND_NORTH_ROTATION: u8 = 0x43;
const KIND_X_TARGET: u8 = 0x44;
const KIND_FRIEND_HITS: u8 = 0x45;
const KIND_REFRACTION_PROPERTY: u8 = 0x48;
const KIND_NO_RUMORS: u8 = 0x4e;
const KIND_RADIUS: u8 = 0x5c;
const KIND_RADIATION: u8 = 0x5d;
const KIND_MULTIBOUND: u8 = 0x61;
const KIND_MULTIBOUND_DATA: u8 = 0x62;
const KIND_MULTIBOUND_REF: u8 = 0x63;
const KIND_EMITTANCE_SOURCE: u8 = 0x67;
const KIND_OCCLUSION_PLANE: u8 = 0x71;
const KIND_ENCOUNTER_ZONE: u8 = 0x74;

/// Other units' constructors (`thiscall(block)`, or with the stated stack words): the merchant
/// container, the leveled creature modifier, the no-rumors flag (with its byte), the leveled
/// creature (0x14 bytes), the seen data, the X target, the encounter zone, the emittance source,
/// the multibound reference, data and bound, the occlusion plane, the follower list, the friend
/// hits (0x1c bytes), the refraction property (with its `float`), the last finished sequence (with
/// its name), the saved animation and havok data.
const MERCHANT_CONTAINER_INIT: u32 = 0x0043_4fc0;
const LEV_CREA_MOD_INIT: u32 = 0x0043_5040;
const NO_RUMORS_INIT: u32 = 0x0043_6090;
const LEVELED_CREATURE_INIT: u32 = 0x0043_11f0;
const SEEN_DATA_INIT: u32 = 0x0040_f590;
const X_TARGET_INIT: u32 = 0x0043_5370;
const ENCOUNTER_ZONE_INIT: u32 = 0x0043_2e60;
const EMITTANCE_SOURCE_INIT: u32 = 0x0043_5440;
const MULTIBOUND_REF_INIT: u32 = 0x0043_5510;
const MULTIBOUND_DATA_INIT: u32 = 0x0043_5590;
const MULTIBOUND_INIT: u32 = 0x0043_5690;
const OCCLUSION_PLANE_INIT: u32 = 0x0043_57a0;
const FOLLOWER_INIT: u32 = 0x0043_0dd0;
const FRIEND_HITS_INIT: u32 = 0x0043_5c40;
const REFRACTION_PROPERTY_INIT: u32 = 0x0043_5f20;
const LAST_FINISHED_SEQUENCE_INIT: u32 = 0x0043_51d0;
const SAVED_ANIMATION_INIT: u32 = 0x0043_5a60;
const SAVED_HAVOK_DATA_INIT: u32 = 0x0043_5b50;

/// The leveled creature modifier's `float` (ST0) and word (`thiscall(extra)`), the friend hits'
/// `AddHit` and `GetHitCount` (`thiscall(extra)`).
const LEV_CREA_MOD_FLOAT: u32 = 0x0043_50c0;
const LEV_CREA_MOD_WORD: u32 = 0x0043_5100;
const FRIEND_HITS_ADD_HIT: u32 = 0x0043_5d40;
const FRIEND_HITS_HIT_COUNT: u32 = 0x0043_5e20;
/// `NiPointer<T>::operator=(T*)` (`thiscall(at, object)`).
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `SaveGameWarning` (`cdecl(message)`), its two messages, and `ExtraSavedAnimation::DeleteBuffer`
/// (`thiscall(extra)`, the saved havok data's too).
const SAVE_GAME_WARNING: u32 = 0x0084_5cc0;
const MESSAGE_SAVED_ANIMATION_EXISTS: u32 = 0x0101_5220;
const MESSAGE_SAVED_HAVOK_DATA_EXISTS: u32 = 0x0101_5278;
const DELETE_SAVED_BUFFER: u32 = 0x0043_5c10;
/// `BGSLoadGameSubBuffer::LoadAnimation` and `LoadHavokData` (`thiscall(buffer, destination)`).
const LOAD_ANIMATION: u32 = 0x0086_4b00;
const LOAD_HAVOK_DATA: u32 = 0x0086_4d80;

/// The player character (`PlayerCharacter *`), whom `AddFollower` never adds.
const PLAYER: u32 = 0x011d_ea3c;
/// The `double` 0.0 the `float` setters compare with (`FCOMP`).
const ZERO_DOUBLE: u32 = 0x0101_2060;
/// The vtables of `ExtraRadius` and `ExtraRadiation`.
const VTABLE_EXTRA_RADIUS: u32 = 0x0101_5208;
const VTABLE_EXTRA_RADIATION: u32 = 0x0101_5214;
/// `TESDataHandler::bClearingData` (Xbox PDB).
const CLEARING_DATA: u32 = 0x61d;

// ---- What the file shares --------------------------------------------------------------------

/// The `float` setters' neutral value: `value` loaded and compared with the `double` 0.0 (`FCOMP`,
/// `TEST AH,0x44`, `JP`): equal only, a NaN being unordered.
#[inline]
fn is_zero(e: &Engine, value: f32) -> bool {
    f64::from(value) == e.global::<f64>(ZERO_DOUBLE)
}

/// A `float` as the x87 stores it (`FLD`, `FSTP`): a signalling NaN comes out quiet.
#[inline]
fn stored(value: f32) -> u32 {
    x87_quiet_bits(value.to_bits())
}

/// The word setters: `value == none` deletes the extra data of `kind` by type; otherwise the one
/// there has `release_old` run on it (its old word) and then holds `value`, or a new one
/// (0x10 bytes, built by `build`) is given `value` and added.
fn set_word_unless(
    e: &mut Engine,
    list: u32,
    kind: u8,
    value: u32,
    none: u32,
    build: impl FnOnce(&mut Engine, u32) -> u32,
    release_old: impl FnOnce(&mut Engine, u32),
) {
    if value == none {
        remove_kind(e, list, kind);
        return;
    }
    match first_of_kind(e, list, kind) {
        0 => {
            let extra = new_object(e, 0x10, build);
            e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), value);
            list_add(e, list, extra);
        }
        extra => {
            release_old(e, extra);
            e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), value);
        }
    }
}

/// [`set_word_unless`] with 0 as the neutral value, another unit's constructor at `construct` and
/// nothing to release.
#[inline]
fn set_word(e: &mut Engine, list: u32, kind: u8, value: u32, construct: u32) {
    set_word_unless(e, list, kind, value, 0, by(construct), |_, _| {});
}

/// Another unit's constructor (`thiscall(block)`) as a build step.
#[inline]
fn by(construct: u32) -> impl FnOnce(&mut Engine, u32) -> u32 {
    move |e, block| e.call(construct, &args![block]).u32()
}

/// The handle setters: 0 deletes the extra data of `kind` by type; otherwise the `NiPointer` at
/// +0xc of the one there, or of a new one (0x10 bytes, `construct`) added after, is assigned
/// `value`.
fn set_handle(e: &mut Engine, list: u32, kind: u8, value: u32, construct: u32) {
    if value == 0 {
        remove_kind(e, list, kind);
        return;
    }
    match first_of_kind(e, list, kind) {
        0 => {
            let extra = new_by(e, 0x10, construct);
            e.call(
                NI_POINTER_ASSIGN,
                &args![extra.wrapping_add(EXTRA_PAYLOAD), value],
            );
            list_add(e, list, extra);
        }
        extra => {
            e.call(
                NI_POINTER_ASSIGN,
                &args![extra.wrapping_add(EXTRA_PAYLOAD), value],
            );
        }
    }
}

/// The handle getters: the word the `NiPointer` at +0xc of the first extra data of `kind` holds
/// (`00559450`, in place), or 0.
#[inline]
fn handle_word(e: &mut Engine, list: u32, kind: u8) -> u32 {
    match first_of_kind(e, list, kind) {
        0 => 0,
        extra => read_word(e, extra.wrapping_add(EXTRA_PAYLOAD)),
    }
}

/// The radius and radiation setters: 0.0 deletes the extra data of `kind` by type; otherwise the
/// `float` is stored in the one there, or a new one (0x10 bytes) is built with it (the class's
/// constructor in place: `BSExtraData`'s, `vtable`, the `float`) and added.
fn set_float_built(e: &mut Engine, list: u32, kind: u8, value: f32, vtable: u32) {
    if is_zero(e, value) {
        remove_kind(e, list, kind);
        return;
    }
    match first_of_kind(e, list, kind) {
        0 => {
            let extra = new_object(e, 0x10, |e, block| {
                float_extra(e, block, kind, vtable, value)
            });
            list_add(e, list, extra);
        }
        extra => e
            .mem
            .set_u32(extra.wrapping_add(EXTRA_PAYLOAD), stored(value)),
    }
}

/// The radius and radiation constructors' body: `BSExtraData`'s constructor (`0040ec80`, in place)
/// with `kind`, `vtable`, the `float` at +0xc. Answers `this`.
#[inline]
fn float_extra(e: &mut Engine, this: u32, kind: u8, vtable: u32, value: f32) -> u32 {
    extra_init(e, this, kind);
    e.mem.set_u32(this, vtable);
    e.mem
        .set_u32(this.wrapping_add(EXTRA_PAYLOAD), stored(value));
    this
}

/// The saved buffers' setters: 0 deletes the extra data of `kind` by type; otherwise the one there
/// has `warning` logged and its old buffer deleted, or a new one (0x10 bytes, `construct`) is
/// added; then it holds `buffer`.
fn set_saved_buffer(
    e: &mut Engine,
    list: u32,
    kind: u8,
    buffer: u32,
    warning: u32,
    construct: u32,
) {
    if buffer == 0 {
        remove_kind(e, list, kind);
        return;
    }
    let extra = match first_of_kind(e, list, kind) {
        0 => {
            let extra = new_by(e, 0x10, construct);
            list_add(e, list, extra);
            extra
        }
        extra => {
            e.call(SAVE_GAME_WARNING, &args![warning]);
            e.call(DELETE_SAVED_BUFFER, &args![extra]);
            extra
        }
    };
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), buffer);
}

/// The saved buffers' restorers: with an extra data of `kind`, its buffer is loaded into
/// `destination` by `load` (given a one-word local holding the buffer: `008c71b0`'s store, in
/// place), its buffer word cleared and the extra data deleted by type. False without one.
fn restore_saved_buffer(e: &mut Engine, list: u32, kind: u8, destination: u32, load: u32) -> bool {
    let extra = first_of_kind(e, list, kind);
    if extra == 0 {
        return false;
    }
    let buffer = word_at(e, extra, EXTRA_PAYLOAD);
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), buffer);
        e.call(load, &args![slot.addr(), destination]);
    });
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), 0);
    remove_kind(e, list, kind);
    true
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 00421400
/// `ExtraDataList::GetMerchantContainer` (Xbox PDB): the word at +0xc of the type 0x3c extra data,
/// or 0.
pub fn merchant_container(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_MERCHANT_CONTAINER, 0)
}

// Typed from 00421430
/// The merchant container's setter (type 0x3c, built by `00434fc0`): 0 removes it.
pub fn set_merchant_container(e: &mut Engine, this: u32, value: u32) {
    set_word(
        e,
        this,
        KIND_MERCHANT_CONTAINER,
        value,
        MERCHANT_CONTAINER_INIT,
    );
}

// Typed from 004214f0
/// The leveled creature modifier (type 0x1e): 1.0 at `out_float` and 0 at `out_word`, then, with
/// the extra data, its `float` (`004350c0`, ST0) and its word (`00435100`) there.
pub fn lev_crea_modifier(e: &mut Engine, this: u32, out_float: u32, out_word: u32) {
    e.mem.set_u32(out_float, 1.0f32.to_bits());
    e.mem.set_u32(out_word, 0);
    let extra = first_of_kind(e, this, KIND_LEV_CREA_MOD);
    if extra != 0 {
        let scale = e.call(LEV_CREA_MOD_FLOAT, &args![extra]).f32();
        e.mem.set_f32(out_float, scale);
        let word = e.call(LEV_CREA_MOD_WORD, &args![extra]).u32();
        e.mem.set_u32(out_word, word);
    }
}

// Typed from 00421540
/// The leveled creature modifier's setter (type 0x1e, built by `00435040`): 4 removes it.
pub fn set_lev_crea_modifier(e: &mut Engine, this: u32, value: u32) {
    set_word_unless(
        e,
        this,
        KIND_LEV_CREA_MOD,
        value,
        4,
        by(LEV_CREA_MOD_INIT),
        |_, _| {},
    );
}

// Typed from 00421600
/// The no-rumors flag's setter (type 0x4e): the byte at +0xc of the one there, or a new one (0x10
/// bytes, `00436090` given the byte zero-extended) added. The flag never removes it.
pub fn set_no_rumors(e: &mut Engine, this: u32, flag: u8) {
    match first_of_kind(e, this, KIND_NO_RUMORS) {
        0 => {
            let extra = new_object(e, 0x10, |e, block| {
                e.call(NO_RUMORS_INIT, &args![block, u32::from(flag)]).u32()
            });
            list_add(e, this, extra);
        }
        extra => e.mem.set_u8(extra.wrapping_add(EXTRA_PAYLOAD), flag),
    }
}

// Typed from 004216b0
/// The no-rumors flag (type 0x4e) removed.
pub fn remove_no_rumors(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_NO_RUMORS);
}

// Typed from 004216d0
/// Whether the list has a leveled creature (type 0x2e): its bit (`0040fe80`, in place).
pub fn has_leveled_creature(e: &mut Engine, this: u32) -> bool {
    has_kind(e, this, KIND_LEVELED_CREATURE)
}

// Typed from 004216f0
/// `ExtraDataList::GetLevCreaOriginalBase` (Xbox PDB): the word at +0xc of the type 0x2e extra
/// data, or 0.
pub fn lev_crea_original_base(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_LEVELED_CREATURE, 0)
}

// Typed from 00421720
/// The word at +0x10 of the type 0x2e extra data, or 0.
pub fn leveled_creature_word(e: &mut Engine, this: u32) -> u32 {
    match first_of_kind(e, this, KIND_LEVELED_CREATURE) {
        0 => 0,
        extra => word_at(e, extra, 0x10),
    }
}

// Typed from 00421750
/// The leveled creature's setter (type 0x2e): either word 0 removes it; otherwise both words go to
/// +0xc and +0x10 of the one there or of a new one (0x14 bytes, `004311f0`) added first.
pub fn set_leveled_creature(e: &mut Engine, this: u32, first: u32, second: u32) {
    if first == 0 || second == 0 {
        remove_kind(e, this, KIND_LEVELED_CREATURE);
        return;
    }
    let extra = match first_of_kind(e, this, KIND_LEVELED_CREATURE) {
        0 => {
            let extra = new_by(e, 0x14, LEVELED_CREATURE_INIT);
            list_add(e, this, extra);
            extra
        }
        extra => extra,
    };
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), first);
    e.mem.set_u32(extra.wrapping_add(0x10), second);
}

// Typed from 00421820
/// `ExtraDataList::GetDetachTime` (Xbox PDB): the word at +0xc of the type 0x0b extra data, or 0.
pub fn detach_time(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_CELL_DETACH_TIME, 0)
}

// Typed from 00421850
/// The cell's detach time setter (type 0x0b, built by `0040f6b0`, the cell references family's): 0
/// removes it.
pub fn set_detach_time(e: &mut Engine, this: u32, time: u32) {
    set_word_unless(
        e,
        this,
        KIND_CELL_DETACH_TIME,
        time,
        0,
        |e, block| {
            typed_call!(
                cell_refs::FAMILY,
                0x0040_f6b0,
                cell_refs::extra_list::detach_time_new(e, block)
            )
        },
        |_, _| {},
    );
}

// Typed from 00421910
/// The seen data (the word at +0xc of the type 0x05 extra data), or 0.
pub fn seen_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_SEEN_DATA, 0)
}

// Typed from 00421940
/// `ExtraDataList::SetSeenData` (Xbox PDB), type 0x05 (built by `0040f590`): 0 removes it; the one
/// there has the object it held deleted first (its slot 0 with 1).
pub fn set_seen_data(e: &mut Engine, this: u32, data: u32) {
    set_word_unless(
        e,
        this,
        KIND_SEEN_DATA,
        data,
        0,
        by(SEEN_DATA_INIT),
        |e, extra| {
            let old = word_at(e, extra, EXTRA_PAYLOAD);
            if old != 0 {
                e.vcall(old, 0, &args![1u32]);
            }
        },
    );
}

// Typed from 00421a40
/// The north rotation (the `float` at +0xc of the type 0x43 extra data), or 0.0.
pub fn north_rotation(e: &mut Engine, this: u32) -> f32 {
    access::float_or(e, this, KIND_NORTH_ROTATION, 0)
}

// Typed from 00421a70
/// `ExtraDataList::SetNorthRotation` (Xbox PDB), type 0x43: 0.0 removes it; otherwise the angle is
/// stored in the one there or in a new one (`0040f680`, the cell references family's) added after.
pub fn set_north_rotation(e: &mut Engine, this: u32, angle: f32) {
    if is_zero(e, angle) {
        remove_kind(e, this, KIND_NORTH_ROTATION);
        return;
    }
    match first_of_kind(e, this, KIND_NORTH_ROTATION) {
        0 => {
            let extra = new_object(e, 0x10, |e, block| {
                typed_call!(
                    cell_refs::FAMILY,
                    0x0040_f680,
                    cell_refs::extra_list::north_rotation_new(e, block)
                )
            });
            e.mem
                .set_u32(extra.wrapping_add(EXTRA_PAYLOAD), stored(angle));
            list_add(e, this, extra);
        }
        extra => e
            .mem
            .set_u32(extra.wrapping_add(EXTRA_PAYLOAD), stored(angle)),
    }
}

// Typed from 00421b40
/// `ExtraDataList::GetXTarget` (Xbox PDB): the word at +0xc of the type 0x44 extra data, or 0.
pub fn x_target(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_X_TARGET, 0)
}

// Typed from 00421b70
/// The X target's setter (type 0x44, built by `00435370`): 0 removes it.
pub fn set_x_target(e: &mut Engine, this: u32, target: u32) {
    set_word(e, this, KIND_X_TARGET, target, X_TARGET_INIT);
}

// Typed from 00421c30
/// The encounter zone (the word at +0xc of the type 0x74 extra data), or 0.
pub fn encounter_zone(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_ENCOUNTER_ZONE, 0)
}

// Typed from 00421c60
/// `ExtraDataList::SetEncounterZone` (Xbox PDB), type 0x74 (built by `00432e60`): 0 removes it.
pub fn set_encounter_zone(e: &mut Engine, this: u32, zone: u32) {
    set_word(e, this, KIND_ENCOUNTER_ZONE, zone, ENCOUNTER_ZONE_INIT);
}

// Typed from 00421d20
/// The emittance source (the word at +0xc of the type 0x67 extra data), or 0.
pub fn emittance_source(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_EMITTANCE_SOURCE, 0)
}

// Typed from 00421d50
/// The emittance source's setter (type 0x67, built by `00435440`): 0 removes it.
pub fn set_emittance_source(e: &mut Engine, this: u32, source: u32) {
    set_word(
        e,
        this,
        KIND_EMITTANCE_SOURCE,
        source,
        EMITTANCE_SOURCE_INIT,
    );
}

// Typed from 00421e10
/// The multibound reference (the word at +0xc of the type 0x63 extra data), or 0.
pub fn multibound_ref(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_MULTIBOUND_REF, 0)
}

// Typed from 00421e40
/// The multibound reference's setter (type 0x63, built by `00435510`): 0 removes it.
pub fn set_multibound_ref(e: &mut Engine, this: u32, reference: u32) {
    set_word(e, this, KIND_MULTIBOUND_REF, reference, MULTIBOUND_REF_INIT);
}

// Typed from 00421f00
/// The multibound data (the word at +0xc of the type 0x62 extra data), or 0.
pub fn multibound_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_MULTIBOUND_DATA, 0)
}

// Typed from 00421f30
/// The multibound data's setter (type 0x62, built by `00435590`): 0 removes it; the one there has
/// the block it held freed first (`operator delete`, when not null).
pub fn set_multibound_data(e: &mut Engine, this: u32, data: u32) {
    set_word_unless(
        e,
        this,
        KIND_MULTIBOUND_DATA,
        data,
        0,
        by(MULTIBOUND_DATA_INIT),
        |e, extra| {
            let old = word_at(e, extra, EXTRA_PAYLOAD);
            if old != 0 {
                e.call(OPERATOR_DELETE, &args![old]);
            }
        },
    );
}

// Typed from 00422020
/// `ExtraDataList::GetMultiBound` (Xbox PDB): the word the `NiPointer` at +0xc of the type 0x61
/// extra data holds, or 0.
pub fn multi_bound(e: &mut Engine, this: u32) -> u32 {
    handle_word(e, this, KIND_MULTIBOUND)
}

// Typed from 00422050
/// `ExtraDataList::SetMultiBound` (Xbox PDB), type 0x61 (built by `00435690`): 0 removes it;
/// otherwise its `NiPointer` is assigned (`0066b0d0`), a new one's before it is added.
pub fn set_multi_bound(e: &mut Engine, this: u32, value: u32) {
    set_handle(e, this, KIND_MULTIBOUND, value, MULTIBOUND_INIT);
}

// Typed from 00422120
/// The occlusion plane: the word the `NiPointer` at +0xc of the type 0x71 extra data holds, or 0.
pub fn occlusion_plane(e: &mut Engine, this: u32) -> u32 {
    handle_word(e, this, KIND_OCCLUSION_PLANE)
}

// Typed from 00422150
/// `ExtraDataList::SetOcclusionPlane` (Xbox PDB), type 0x71 (built by `004357a0`): as
/// [`set_multi_bound`].
pub fn set_occlusion_plane(e: &mut Engine, this: u32, value: u32) {
    set_handle(e, this, KIND_OCCLUSION_PLANE, value, OCCLUSION_PLANE_INIT);
}

// Typed from 00422220
/// `ExtraDataList::SetRadius` (Xbox PDB), type 0x5c: 0.0 removes it; otherwise the radius is stored
/// in the one there or a new one built with it ([`radius_extra_new`], in place) is added.
pub fn set_radius(e: &mut Engine, this: u32, radius: f32) {
    set_float_built(e, this, KIND_RADIUS, radius, VTABLE_EXTRA_RADIUS);
}

// Typed from 004222f0
/// `ExtraRadius`'s constructor (`thiscall(block, radius)`): `BSExtraData`'s with type 0x5c, its
/// vtable, the radius at +0xc. Answers `this`.
pub fn radius_extra_new(e: &mut Engine, this: u32, radius: f32) -> u32 {
    float_extra(e, this, KIND_RADIUS, VTABLE_EXTRA_RADIUS, radius)
}

// Typed from 00422320
/// `ExtraDataList::GetRadius` (Xbox PDB): the `float` at +0xc of the type 0x5c extra data, or 0.0.
pub fn radius(e: &mut Engine, this: u32) -> f32 {
    access::float_or(e, this, KIND_RADIUS, 0)
}

// Typed from 00422350
/// The radiation's setter (type 0x5d): as [`set_radius`], built by [`radiation_extra_new`].
pub fn set_radiation(e: &mut Engine, this: u32, radiation: f32) {
    set_float_built(e, this, KIND_RADIATION, radiation, VTABLE_EXTRA_RADIATION);
}

// Typed from 00422420
/// `ExtraRadiation`'s constructor (`thiscall(block, radiation)`): `BSExtraData`'s with type 0x5d,
/// its vtable, the value at +0xc. Answers `this`.
pub fn radiation_extra_new(e: &mut Engine, this: u32, radiation: f32) -> u32 {
    float_extra(e, this, KIND_RADIATION, VTABLE_EXTRA_RADIATION, radiation)
}

// Typed from 00422450
/// The radiation (the `float` at +0xc of the type 0x5d extra data), or 0.0.
pub fn radiation(e: &mut Engine, this: u32) -> f32 {
    access::float_or(e, this, KIND_RADIATION, 0)
}

// Typed from 00422480
/// `ExtraDataList::AddFollower` (Xbox PDB): unless `follower` is the player, the follower list
/// (type 0x1d; a new one is built by `00430dd0` and added) whose list +0xc points at gets
/// `follower` at its head unless it holds it (`005f65d0`, in place; `005ae3d0` given a one-word
/// local holding it).
pub fn add_follower(e: &mut Engine, this: u32, follower: u32) {
    if follower == e.mem.u32(PLAYER) {
        return;
    }
    let extra = match first_of_kind(e, this, KIND_FOLLOWER) {
        0 => {
            let extra = new_by(e, 0x10, FOLLOWER_INIT);
            list_add(e, this, extra);
            extra
        }
        extra => extra,
    };
    let followers = word_at(e, extra, EXTRA_PAYLOAD);
    add_unless_held(e, followers, follower);
}

// Typed from 00422550
/// Whether the follower list (type 0x1d) holds `follower` (`005f65d0`, in place); false without
/// one.
pub fn has_follower(e: &mut Engine, this: u32, follower: u32) -> bool {
    match first_of_kind(e, this, KIND_FOLLOWER) {
        0 => false,
        extra => {
            let followers = word_at(e, extra, EXTRA_PAYLOAD);
            holds(e, followers, follower)
        }
    }
}

// Typed from 00422590
/// `ExtraDataList::AddFriendHit` (Xbox PDB): the friend hits (type 0x45; a new one of 0x1c bytes is
/// built by `00435c40` and added) given a hit (`00435d40`); answers their count (`00435e20`), the
/// exe's last call.
pub fn add_friend_hit(e: &mut Engine, this: u32) -> u32 {
    let extra = match first_of_kind(e, this, KIND_FRIEND_HITS) {
        0 => {
            let extra = new_by(e, 0x1c, FRIEND_HITS_INIT);
            list_add(e, this, extra);
            extra
        }
        extra => extra,
    };
    e.call(FRIEND_HITS_ADD_HIT, &args![extra]);
    e.call(FRIEND_HITS_HIT_COUNT, &args![extra]).u32()
}

// Typed from 00422640
/// `ExtraDataList::GetFriendHitCount` (Xbox PDB): the friend hits' count (`00435e20`), or 0.
pub fn friend_hit_count(e: &mut Engine, this: u32) -> u32 {
    match first_of_kind(e, this, KIND_FRIEND_HITS) {
        0 => 0,
        extra => e.call(FRIEND_HITS_HIT_COUNT, &args![extra]).u32(),
    }
}

// Typed from 00422670
/// The friend hits (type 0x45) removed.
pub fn remove_friend_hits(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_FRIEND_HITS);
}

// Typed from 00422690
/// `ExtraDataList::RemoveFollower` (Xbox PDB): with a follower list (type 0x1d), `follower` taken
/// out of the list +0xc points at (`00905330`, given a one-word local holding it). The exe then
/// asks the save/load singleton (`0047c850`, false in this build) and reads the data handler's
/// `bClearingData` ([`clearing_data`]), using neither answer: nothing to do here.
pub fn remove_follower(e: &mut Engine, this: u32, follower: u32) {
    let extra = first_of_kind(e, this, KIND_FOLLOWER);
    if extra != 0 {
        let followers = word_at(e, extra, EXTRA_PAYLOAD);
        remove_item(e, followers, follower);
    }
}

// Typed from 004226e0
/// `TESDataHandler::bClearingData` (Xbox PDB, the byte at +0x61d) of the handler it is called on.
pub fn clearing_data(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(this.wrapping_add(CLEARING_DATA))
}

// Typed from 00422720
/// `ExtraDataList::RemoveFollowerExtra` (Xbox PDB): the follower list (type 0x1d) removed when the
/// list has one.
pub fn remove_follower_extra(e: &mut Engine, this: u32) {
    if first_of_kind(e, this, KIND_FOLLOWER) != 0 {
        remove_kind(e, this, KIND_FOLLOWER);
    }
}

// Typed from 00422750
/// The refraction property's setter (type 0x48): a zero `enable` deletes the one there
/// (`RemoveExtra(extra, true)`); otherwise the `float` is stored at +0xc of the one there or of a
/// new one (0x10 bytes, `00435f20` given the `float` as the x87 pushes it) added first.
pub fn set_refraction_property(e: &mut Engine, this: u32, enable: u8, value: f32) {
    let extra = first_of_kind(e, this, KIND_REFRACTION_PROPERTY);
    if enable == 0 {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
        return;
    }
    let extra = if extra == 0 {
        let made = new_object(e, 0x10, |e, block| {
            e.call(REFRACTION_PROPERTY_INIT, &args![block, stored(value)])
                .u32()
        });
        list_add(e, this, made);
        made
    } else {
        extra
    };
    e.mem
        .set_u32(extra.wrapping_add(EXTRA_PAYLOAD), stored(value));
}

// Typed from 00422820
/// `ExtraDataList::GetRefractionPropertyExtra` (Xbox PDB): the type 0x48 extra data, or null.
pub fn refraction_property_extra(e: &mut Engine, this: u32) -> u32 {
    first_of_kind(e, this, KIND_REFRACTION_PROPERTY)
}

// Typed from 00422850
/// The last finished sequence's setter (type 0x41): the old one removed
/// ([`remove_last_finished_sequence`]), then for a non-null `name` a new one (0x10 bytes,
/// `004351d0`, which copies the name) added.
pub fn set_last_finished_sequence(e: &mut Engine, this: u32, name: u32) {
    typed_call!(FAMILY, 0x0042_2920, remove_last_finished_sequence(e, this));
    if name != 0 {
        let extra = new_object(e, 0x10, |e, block| {
            e.call(LAST_FINISHED_SEQUENCE_INIT, &args![block, name])
                .u32()
        });
        list_add(e, this, extra);
    }
}

// Typed from 004228f0
/// `ExtraDataList::GetLastFinishedSequence` (Xbox PDB): the word at +0xc of the type 0x41 extra
/// data, or 0.
pub fn last_finished_sequence(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_LAST_FINISHED_SEQUENCE, 0)
}

// Typed from 00422920
/// `ExtraDataList::RemoveLastFinishedSequence` (Xbox PDB): the type 0x41 extra data removed.
pub fn remove_last_finished_sequence(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_LAST_FINISHED_SEQUENCE);
}

// Typed from 00422940
/// `ExtraDataList::SetSavedAnimation` (Xbox PDB), type 0x42 (built by `00435a60`): 0 removes it;
/// one there is warned about and has its buffer deleted before it holds the new one.
pub fn set_saved_animation(e: &mut Engine, this: u32, buffer: u32) {
    set_saved_buffer(
        e,
        this,
        KIND_SAVED_ANIMATION,
        buffer,
        MESSAGE_SAVED_ANIMATION_EXISTS,
        SAVED_ANIMATION_INIT,
    );
}

// Typed from 00422a10
/// `ExtraDataList::GetSavedAnimation` (Xbox PDB): the word at +0xc of the type 0x42 extra data, or
/// 0.
pub fn saved_animation(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_SAVED_ANIMATION, 0)
}

// Typed from 00422a40
/// `ExtraDataList::RestoreSavedAnimation` (Xbox PDB): the saved animation loaded into `destination`
/// (`00864b00`), forgotten and removed; false without one.
pub fn restore_saved_animation(e: &mut Engine, this: u32, destination: u32) -> bool {
    restore_saved_buffer(e, this, KIND_SAVED_ANIMATION, destination, LOAD_ANIMATION)
}

// Typed from 00422aa0
/// `ExtraDataList::RemoveSavedAnimation` (Xbox PDB): the type 0x42 extra data removed.
pub fn remove_saved_animation(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_SAVED_ANIMATION);
}

// Typed from 00422ac0
/// `ExtraDataList::SetSavedHavokData` (Xbox PDB), type 0x3d (built by `00435b50`): as
/// [`set_saved_animation`], with its own warning.
pub fn set_saved_havok_data(e: &mut Engine, this: u32, buffer: u32) {
    set_saved_buffer(
        e,
        this,
        KIND_SAVED_HAVOK_DATA,
        buffer,
        MESSAGE_SAVED_HAVOK_DATA_EXISTS,
        SAVED_HAVOK_DATA_INIT,
    );
}

// Typed from 00422b90
/// `ExtraDataList::GetSavedHavokData` (Xbox PDB): the word at +0xc of the type 0x3d extra data, or
/// 0.
pub fn saved_havok_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_SAVED_HAVOK_DATA, 0)
}

// Typed from 00422bc0
/// `ExtraDataList::RestoreSavedHavokData` (Xbox PDB): as [`restore_saved_animation`] with
/// `00864d80`.
pub fn restore_saved_havok_data(e: &mut Engine, this: u32, destination: u32) -> bool {
    restore_saved_buffer(e, this, KIND_SAVED_HAVOK_DATA, destination, LOAD_HAVOK_DATA)
}

// Typed from 00422c20
/// `ExtraDataList::RemoveSavedHavokData` (Xbox PDB): the type 0x3d extra data removed.
pub fn remove_saved_havok_data(e: &mut Engine, this: u32) {
    remove_kind(e, this, KIND_SAVED_HAVOK_DATA);
}
