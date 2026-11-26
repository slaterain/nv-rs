//! The part 4 file's small classes and accessors: the byte at +0xc of an extra data (`00424940`),
//! the save buffer's helpers (`00428070` to `00428130`), the constructors and destructors of the
//! lock, teleport, ownership, global, rank, count, health and uses extra data and of the types 0x27
//! to 0x8f (`0042c470` to `0042cd70`), the small accessors after them (`0042cde0` to `0042ce90`)
//! and `0042dd70`.
//!
//! A constructor is `BSExtraData`'s (`0040ec80`) in place with the class's type, then its vtable
//! and its empty payload; a `float` payload's 0.0 (`FLDZ`, `FSTP`) is the word 0. The destructors
//! and members of other units (`ExtraLock`'s, `ExtraTeleport`'s, `ExtraScript`'s,
//! `ExtraTresPassPackage`'s and `ExtraHeadingTarget`'s destructors, the string members, the actor
//! cause's handle, the flags record's member) are called by address; `BSExtraData`'s destructor
//! (`0040ecb0`) and the lock record's constructor (`00411b00`) are typed calls into the cell
//! references family.

use super::links::is_empty;
use super::{cell_refs, extra_init, free_when, new_object, word_at, EXTRA_PAYLOAD, FAMILY};
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The extra data types (`cEtype`) the constructors give.
const KIND_SCRIPT: u8 = 0x0d;
const KIND_TRESPASS_PACKAGE: u8 = 0x1a;
const KIND_REFERENCE_POINTER: u8 = 0x1c;
const KIND_OWNERSHIP: u8 = 0x21;
const KIND_GLOBAL: u8 = 0x22;
const KIND_RANK: u8 = 0x23;
const KIND_COUNT: u8 = 0x24;
const KIND_HEALTH: u8 = 0x25;
const KIND_USES: u8 = 0x26;
const KIND_TIME_LEFT: u8 = 0x27;
const KIND_CHARGE: u8 = 0x28;
const KIND_LOCK: u8 = 0x2a;
const KIND_TELEPORT: u8 = 0x2b;
const KIND_LEVELED_ITEM: u8 = 0x2f;
const KIND_SCALE: u8 = 0x30;
const KIND_POISON: u8 = 0x3f;
const KIND_HEAD_TRACK_TARGET: u8 = 0x46;
const KIND_HOT_KEY: u8 = 0x4a;
const KIND_NO_RUMORS: u8 = 0x4e;
const KIND_OBJECT_HEALTH: u8 = 0x56;
const KIND_MODEL_SWAP: u8 = 0x5b;
const KIND_RADIUS: u8 = 0x5c;
const KIND_RADIATION: u8 = 0x5d;
const KIND_ACTOR_CAUSE: u8 = 0x60;
const KIND_AMMO: u8 = 0x6e;
const KIND_PACKAGE_DATA: u8 = 0x70;
const KIND_WEAPON_MOD_SLOTS: u8 = 0x8d;
const KIND_SECURITRON_FACE: u8 = 0x8f;

/// The classes' vtables.
const VTABLE_EXTRA_LOCK: u32 = 0x0101_589c;
const VTABLE_EXTRA_TELEPORT: u32 = 0x0101_58a8;
const VTABLE_EXTRA_OWNERSHIP: u32 = 0x0101_58b4;
const VTABLE_EXTRA_GLOBAL: u32 = 0x0101_58c0;
const VTABLE_EXTRA_RANK: u32 = 0x0101_58cc;
const VTABLE_EXTRA_COUNT: u32 = 0x0101_58d8;
const VTABLE_EXTRA_HEALTH: u32 = 0x0101_58e4;
const VTABLE_EXTRA_USES: u32 = 0x0101_58f0;
const VTABLE_EXTRA_TIME_LEFT: u32 = 0x0101_58fc;
const VTABLE_EXTRA_CHARGE: u32 = 0x0101_5908;
const VTABLE_EXTRA_SCRIPT: u32 = 0x0101_5914;
const VTABLE_EXTRA_SCALE: u32 = 0x0101_5920;
const VTABLE_EXTRA_HOT_KEY: u32 = 0x0101_592c;
const VTABLE_EXTRA_REFERENCE_POINTER: u32 = 0x0101_5938;
const VTABLE_EXTRA_TRESPASS_PACKAGE: u32 = 0x0101_5944;
const VTABLE_EXTRA_LEVELED_ITEM: u32 = 0x0101_5950;
const VTABLE_EXTRA_POISON: u32 = 0x0101_595c;
const VTABLE_EXTRA_HEAD_TRACK_TARGET: u32 = 0x0101_5968;
const VTABLE_EXTRA_NO_RUMORS: u32 = 0x0101_5974;
const VTABLE_EXTRA_OBJECT_HEALTH: u32 = 0x0101_5184;
const VTABLE_EXTRA_MODEL_SWAP: u32 = 0x0101_5980;
const VTABLE_EXTRA_RADIUS: u32 = 0x0101_5208;
const VTABLE_EXTRA_RADIATION: u32 = 0x0101_5214;
const VTABLE_EXTRA_ACTOR_CAUSE: u32 = 0x0101_598c;
const VTABLE_EXTRA_AMMO: u32 = 0x0101_5998;
const VTABLE_EXTRA_PACKAGE_DATA: u32 = 0x0101_51fc;
const VTABLE_EXTRA_WEAPON_MOD_SLOTS: u32 = 0x0101_59a4;
const VTABLE_EXTRA_SECURITRON_FACE: u32 = 0x0101_59b0;

/// Other units' destructors (`thiscall(extra)`): `ExtraLock`'s, `ExtraTeleport`'s, `ExtraScript`'s,
/// `ExtraTresPassPackage`'s, `ExtraHeadingTarget`'s.
const EXTRA_LOCK_DROP: u32 = 0x0043_0ca0;
const EXTRA_TELEPORT_DROP: u32 = 0x0043_1260;
const EXTRA_SCRIPT_DROP: u32 = 0x0043_2040;
const EXTRA_TRESPASS_PACKAGE_DROP: u32 = 0x0043_2900;
const EXTRA_HEADING_TARGET_DROP: u32 = 0x0043_5ef0;
/// The flags record's member (`thiscall(member)`), the actor cause's handle: its constructor
/// (`thiscall(at, 0)`), assignment (`thiscall(at, value)`) and release (`thiscall(at)`).
const FLAGS_MEMBER_INIT: u32 = 0x0042_f570;
const ACTOR_CAUSE_HANDLE_INIT: u32 = 0x0042_f730;
const ACTOR_CAUSE_HANDLE_ASSIGN: u32 = 0x0042_f780;
const ACTOR_CAUSE_HANDLE_RELEASE: u32 = 0x0042_f760;
/// `BSString`'s constructor and destructor (`thiscall(string)`) and its assignment from another
/// (`thiscall(string, source)`).
const STRING_INIT: u32 = 0x0040_37b0;
const STRING_DROP: u32 = 0x0040_37d0;
const STRING_ASSIGN: u32 = 0x0040_47f0;
/// The `float` the three-word record starts with (-1.0), the word `0042ce00` answers.
const RECORD_FLOAT: u32 = 0x0101_2054;
const GIVEN_WORD: u32 = 0x011c_6444;

// ---- What the file shares --------------------------------------------------------------------

/// A constructor's start: `BSExtraData`'s (`0040ec80`, in place) with `kind`, then `vtable`.
#[inline]
fn extra_with_vtable(e: &mut Engine, this: u32, kind: u8, vtable: u32) {
    extra_init(e, this, kind);
    e.mem.set_u32(this, vtable);
}

/// The constructors of an extra data whose payload is one zero word at +0xc. Answers `this`.
#[inline]
fn zero_word_extra(e: &mut Engine, this: u32, kind: u8, vtable: u32) -> u32 {
    extra_with_vtable(e, this, kind, vtable);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
    this
}

/// The constructors of an extra data whose payload is two zero words at +0xc. Answers `this`.
#[inline]
fn two_zero_words_extra(e: &mut Engine, this: u32, kind: u8, vtable: u32) -> u32 {
    zero_word_extra(e, this, kind, vtable);
    e.mem.set_u32(this.wrapping_add(0x10), 0);
    this
}

/// The constructors of an extra data whose payload is one zero byte at +0xc. Answers `this`.
#[inline]
fn zero_byte_extra(e: &mut Engine, this: u32, kind: u8, vtable: u32) -> u32 {
    extra_with_vtable(e, this, kind, vtable);
    e.mem.set_u8(this.wrapping_add(EXTRA_PAYLOAD), 0);
    this
}

/// The scalar deleting destructors of the classes whose destructor is another unit's: `drop` on
/// `this`, then `operator delete` for bit 0 of `flags`. Answers `this`.
#[inline]
fn delete_by(e: &mut Engine, this: u32, flags: u32, drop: u32) -> u32 {
    e.call(drop, &args![this]);
    free_when(e, this, flags)
}

/// `BSExtraData`'s destructor (`0040ecb0`, the cell references family's).
#[inline]
fn extra_drop(e: &mut Engine, this: u32) {
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ecb0,
        cell_refs::extra_list::extra_data_drop(e, this)
    );
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 00424940
/// The byte at +0xc of the extra data it is called on.
pub fn byte_at_c(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(this.wrapping_add(EXTRA_PAYLOAD))
}

// Typed from 00428070
/// The string at +0xc of the extra data it is called on assigned to the string `out` (`004047f0`).
/// Answers `out`.
pub fn first_string_to(e: &mut Engine, this: u32, out: u32) -> u32 {
    e.call(STRING_ASSIGN, &args![out, this.wrapping_add(0x0c)]);
    out
}

// Typed from 004280b0
/// As [`first_string_to`] for the string at +0x14.
pub fn second_string_to(e: &mut Engine, this: u32, out: u32) -> u32 {
    e.call(STRING_ASSIGN, &args![out, this.wrapping_add(0x14)]);
    out
}

// Typed from 004280f0
/// Whether the word at `this` has any bit of `mask`.
pub fn has_any(e: &mut Engine, this: u32, mask: u32) -> bool {
    e.mem.u32(this) & mask != 0
}

// Typed from 00428110
/// The save buffer's current save kind (its unaligned word at +0x17) stored at `out`. Answers
/// `out`.
pub fn save_kind_to(e: &mut Engine, this: u32, out: u32) -> u32 {
    let kind = e.mem.u32(this.wrapping_add(0x17));
    e.mem.set_u32(out, kind);
    out
}

// Typed from 00428130
/// The save buffer's current save kind (the word at +0x17) set.
pub fn set_save_kind(e: &mut Engine, this: u32, kind: u32) {
    e.mem.set_u32(this.wrapping_add(0x17), kind);
}

// Typed from 0042c470
/// A record of four zeroed flag bytes and the member at +4 (`0042f570`). Answers `this`.
pub fn flags_record_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, 0);
    e.call(FLAGS_MEMBER_INIT, &args![this.wrapping_add(4)]);
    this
}

// Typed from 0042c4b0
/// `ExtraLock`'s constructor (type 0x2a): the 0x14-byte lock record at +0xc (`00411b00`, null when
/// the allocation failed). Answers `this`.
pub fn lock_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_LOCK, VTABLE_EXTRA_LOCK);
    let record = new_object(e, 0x14, |e, block| {
        typed_call!(
            cell_refs::FAMILY,
            0x0041_1b00,
            cell_refs::extra_list::lock_data_new(e, block)
        )
    });
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), record);
    this
}

// Typed from 0042c550
/// `ExtraLock`'s scalar deleting destructor: its destructor (`00430ca0`), then `operator delete`
/// for bit 0 of `flags`. Answers `this`.
pub fn lock_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    delete_by(e, this, flags, EXTRA_LOCK_DROP)
}

// Typed from 0042c580
/// `ExtraTeleport`'s constructor (type 0x2b): no teleport data. Answers `this`.
pub fn teleport_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_TELEPORT, VTABLE_EXTRA_TELEPORT)
}

// Typed from 0042c5b0
/// `ExtraTeleport`'s scalar deleting destructor (its destructor `00431260`). Answers `this`.
pub fn teleport_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    delete_by(e, this, flags, EXTRA_TELEPORT_DROP)
}

// Typed from 0042c5e0
/// `ExtraOwnership`'s constructor (type 0x21): no owner. Answers `this`.
pub fn ownership_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_OWNERSHIP, VTABLE_EXTRA_OWNERSHIP)
}

// Typed from 0042c610
/// `ExtraGlobal`'s constructor (type 0x22): no global. Answers `this`.
pub fn global_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_GLOBAL, VTABLE_EXTRA_GLOBAL)
}

// Typed from 0042c640
/// `ExtraRank`'s constructor (type 0x23): rank 0. Answers `this`.
pub fn rank_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_RANK, VTABLE_EXTRA_RANK)
}

// Typed from 0042c670
/// `ExtraCount`'s constructor (type 0x24): the 16-bit count 0. Answers `this`.
pub fn count_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_COUNT, VTABLE_EXTRA_COUNT);
    e.mem.set_u16(this.wrapping_add(EXTRA_PAYLOAD), 0);
    this
}

// Typed from 0042c6a0
/// `ExtraHealth`'s constructor (type 0x25): 0.0. Answers `this`.
pub fn health_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_HEALTH, VTABLE_EXTRA_HEALTH)
}

// Typed from 0042c6d0
/// `ExtraUses`'s constructor (type 0x26): the byte 0. Answers `this`.
pub fn uses_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_byte_extra(e, this, KIND_USES, VTABLE_EXTRA_USES)
}

// Typed from 0042c700
/// `ExtraTimeLeft`'s constructor (type 0x27): 0.0. Answers `this`.
pub fn time_left_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_TIME_LEFT, VTABLE_EXTRA_TIME_LEFT)
}

// Typed from 0042c730
/// `ExtraCharge`'s constructor (type 0x28): 0.0. Answers `this`.
pub fn charge_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_CHARGE, VTABLE_EXTRA_CHARGE)
}

// Typed from 0042c760
/// `ExtraScript`'s constructor (type 0x0d): no script and no locals. Answers `this`.
pub fn script_extra_new(e: &mut Engine, this: u32) -> u32 {
    two_zero_words_extra(e, this, KIND_SCRIPT, VTABLE_EXTRA_SCRIPT)
}

// Typed from 0042c7a0
/// `ExtraScript`'s scalar deleting destructor (its destructor `00432040`). Answers `this`.
pub fn script_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    delete_by(e, this, flags, EXTRA_SCRIPT_DROP)
}

// Typed from 0042c7d0
/// `ExtraScale`'s constructor (type 0x30): 0.0. Answers `this`.
pub fn scale_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_SCALE, VTABLE_EXTRA_SCALE)
}

// Typed from 0042c800
/// `ExtraHotkey`'s constructor (type 0x4a): the byte 0. Answers `this`.
pub fn hot_key_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_byte_extra(e, this, KIND_HOT_KEY, VTABLE_EXTRA_HOT_KEY)
}

// Typed from 0042c830
/// `ExtraReferencePointer`'s constructor (type 0x1c): no reference. Answers `this`.
pub fn reference_pointer_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(
        e,
        this,
        KIND_REFERENCE_POINTER,
        VTABLE_EXTRA_REFERENCE_POINTER,
    )
}

// Typed from 0042c860
/// `ExtraTresPassPackage`'s constructor (type 0x1a): no package. Answers `this`.
pub fn trespass_package_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(
        e,
        this,
        KIND_TRESPASS_PACKAGE,
        VTABLE_EXTRA_TRESPASS_PACKAGE,
    )
}

// Typed from 0042c890
/// `ExtraTresPassPackage`'s scalar deleting destructor (its destructor `00432900`). Answers `this`.
pub fn trespass_package_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    delete_by(e, this, flags, EXTRA_TRESPASS_PACKAGE_DROP)
}

// Typed from 0042c8c0
/// `ExtraLeveledItem`'s constructor (type 0x2f): the word at +0xc and the byte at +0x10 zero.
/// Answers `this`.
pub fn leveled_item_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_LEVELED_ITEM, VTABLE_EXTRA_LEVELED_ITEM);
    e.mem.set_u8(this.wrapping_add(0x10), 0);
    this
}

// Typed from 0042c900
/// `ExtraPoison`'s constructor (type 0x3f): no poison. Answers `this`.
pub fn poison_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_POISON, VTABLE_EXTRA_POISON)
}

// Typed from 0042c930
/// `ExtraHeadingTarget`'s constructor (type 0x46): no target. Answers `this`.
pub fn head_track_target_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(
        e,
        this,
        KIND_HEAD_TRACK_TARGET,
        VTABLE_EXTRA_HEAD_TRACK_TARGET,
    )
}

// Typed from 0042c960
/// `ExtraHeadingTarget`'s scalar deleting destructor (its destructor `00435ef0`). Answers `this`.
pub fn head_track_target_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    delete_by(e, this, flags, EXTRA_HEADING_TARGET_DROP)
}

// Typed from 0042c990
/// `ExtraNoRumors`'s constructor (type 0x4e): the byte 0. Answers `this`.
pub fn no_rumors_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_byte_extra(e, this, KIND_NO_RUMORS, VTABLE_EXTRA_NO_RUMORS)
}

// Typed from 0042c9c0
/// `ExtraObjectHealth`'s constructor (type 0x56): 0.0. Answers `this`.
pub fn object_health_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_OBJECT_HEALTH, VTABLE_EXTRA_OBJECT_HEALTH)
}

// Typed from 0042c9f0
/// `ExtraModelSwap`'s constructor (type 0x5b): the two words zero. Answers `this`.
pub fn model_swap_extra_new(e: &mut Engine, this: u32) -> u32 {
    two_zero_words_extra(e, this, KIND_MODEL_SWAP, VTABLE_EXTRA_MODEL_SWAP)
}

// Typed from 0042ca30
/// `ExtraRadius`'s default constructor (type 0x5c): 0.0. Answers `this`.
pub fn radius_extra_default(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_RADIUS, VTABLE_EXTRA_RADIUS)
}

// Typed from 0042ca60
/// `ExtraRadiation`'s default constructor (type 0x5d): 0.0. Answers `this`.
pub fn radiation_extra_default(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_RADIATION, VTABLE_EXTRA_RADIATION)
}

// Typed from 0042ca90
/// `ExtraActorCause`'s constructor (type 0x60): its handle at +0xc built (`0042f730` given 0).
/// Answers `this`.
pub fn actor_cause_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_ACTOR_CAUSE, VTABLE_EXTRA_ACTOR_CAUSE);
    e.call(
        ACTOR_CAUSE_HANDLE_INIT,
        &args![this.wrapping_add(EXTRA_PAYLOAD), 0u32],
    );
    this
}

// Typed from 0042cb00
/// `ExtraActorCause`'s scalar deleting destructor: [`actor_cause_extra_drop`], then `operator
/// delete` for bit 0 of `flags`. Answers `this`.
pub fn actor_cause_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_cb30, actor_cause_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0042cb30
/// `ExtraActorCause`'s destructor: its vtable back, its handle assigned 0 (`0042f780`) and released
/// (`0042f760`), then `BSExtraData`'s destructor.
pub fn actor_cause_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_ACTOR_CAUSE);
    let handle = this.wrapping_add(EXTRA_PAYLOAD);
    e.call(ACTOR_CAUSE_HANDLE_ASSIGN, &args![handle, 0u32]);
    e.call(ACTOR_CAUSE_HANDLE_RELEASE, &args![handle]);
    extra_drop(e, this);
}

// Typed from 0042cba0
/// `ExtraAmmo`'s constructor (type 0x6e): no ammo, count 0. Answers `this`.
pub fn ammo_extra_new(e: &mut Engine, this: u32) -> u32 {
    two_zero_words_extra(e, this, KIND_AMMO, VTABLE_EXTRA_AMMO)
}

// Typed from 0042cbe0
/// `ExtraPackageData`'s constructor (type 0x70): no data. Answers `this`.
pub fn package_data_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_word_extra(e, this, KIND_PACKAGE_DATA, VTABLE_EXTRA_PACKAGE_DATA)
}

// Typed from 0042cc10
/// A three-word record: 0, -1, and the `float` at `01012054` moved through the x87 (a signalling
/// NaN would come out quiet). Answers `this`.
pub fn record_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, 0);
    e.mem.set_u32(this.wrapping_add(4), u32::MAX);
    let value = x87_quiet_bits(e.mem.u32(RECORD_FLOAT));
    e.mem.set_u32(this.wrapping_add(8), value);
    this
}

// Typed from 0042cc40
/// `ExtraWeaponModFlags`' constructor (type 0x8d): the byte 0. Answers `this`.
pub fn weapon_mod_slots_extra_new(e: &mut Engine, this: u32) -> u32 {
    zero_byte_extra(
        e,
        this,
        KIND_WEAPON_MOD_SLOTS,
        VTABLE_EXTRA_WEAPON_MOD_SLOTS,
    )
}

// Typed from 0042cc70
/// The type 0x8d extra data's scalar deleting destructor: [`weapon_mod_slots_extra_drop`], then
/// `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn weapon_mod_slots_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_cca0, weapon_mod_slots_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0042cca0
/// The type 0x8d extra data's destructor: its vtable back, then `BSExtraData`'s destructor.
pub fn weapon_mod_slots_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_WEAPON_MOD_SLOTS);
    extra_drop(e, this);
}

// Typed from 0042ccc0
/// `ExtraSecuritronFace`'s constructor (type 0x8f): its two strings at +0xc and +0x14 built
/// (`004037b0`). Answers `this`.
pub fn securitron_face_extra_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_SECURITRON_FACE, VTABLE_EXTRA_SECURITRON_FACE);
    e.call(STRING_INIT, &args![this.wrapping_add(0x0c)]);
    e.call(STRING_INIT, &args![this.wrapping_add(0x14)]);
    this
}

// Typed from 0042cd40
/// `ExtraSecuritronFace`'s scalar deleting destructor: [`securitron_face_extra_drop`], then
/// `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn securitron_face_extra_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_cd70, securitron_face_extra_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0042cd70
/// `ExtraSecuritronFace`'s destructor: its vtable back, its strings at +0x14 and +0xc destroyed
/// (`004037d0`), then `BSExtraData`'s destructor.
pub fn securitron_face_extra_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_SECURITRON_FACE);
    e.call(STRING_DROP, &args![this.wrapping_add(0x14)]);
    e.call(STRING_DROP, &args![this.wrapping_add(0x0c)]);
    extra_drop(e, this);
}

// Typed from 0042cde0
/// Whether the list the word at `this` points at is empty (`008256d0`, in place).
pub fn list_at_is_empty(e: &mut Engine, this: u32) -> bool {
    let node = word_at(e, this, 0);
    is_empty(e, node)
}

// Typed from 0042ce00
/// The word at `011c6444`.
pub fn given_word(e: &mut Engine) -> u32 {
    e.mem.u32(GIVEN_WORD)
}

// Typed from 0042ce10
/// Whether bit 1 of the word at +0x244 of the object it is called on is set.
pub fn flag_2_of_244(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(this.wrapping_add(0x244)) & 2 != 0
}

// Typed from 0042ce30
/// The save buffer's word of flags (+0x2c) stored at `out`. Answers `out`.
pub fn buffer_flags_to(e: &mut Engine, this: u32, out: u32) -> u32 {
    let flags = e.mem.u32(this.wrapping_add(0x2c));
    e.mem.set_u32(out, flags);
    out
}

// Typed from 0042ce50
/// Bit 3 of the word at +0x28 of the object it is called on set (`flag` non-zero) or cleared.
pub fn set_flag_8_of_28(e: &mut Engine, this: u32, flag: u8) {
    let at = this.wrapping_add(0x28);
    let word = e.mem.u32(at);
    e.mem
        .set_u32(at, if flag != 0 { word | 8 } else { word & !8 });
}

// Typed from 0042ce90
/// Whether bit 3 of the word at +0x28 of the object it is called on is set.
pub fn flag_8_of_28(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(this.wrapping_add(0x28)) & 8 != 0
}

// Typed from 0042dd70
/// The byte at +0xd of the object it is called on copied to +0xc.
pub fn copy_byte_d_to_c(e: &mut Engine, this: u32) {
    let byte = e.mem.u8(this.wrapping_add(0x0d));
    e.mem.set_u8(this.wrapping_add(0x0c), byte);
}
