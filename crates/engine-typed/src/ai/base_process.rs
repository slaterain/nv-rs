//! `BaseProcess` (the Xbox PDB's source unit of that name, in `fallout/ai`): every function the unit
//! has in the exe but `0044edb0` (the facegen family's) and `008d0360` (the cell references
//! family's), each as its translation in `engine-fallout-ai` does it.
//!
//! The fields read here (Xbox PDB names, checked against the PC code): the embedded current package
//! (`ActorPackage`, +0x04), `fSystemTimeClock` (+0x1c), `fHourLastProcessed` (+0x20, a float),
//! `iDateLastProcessed` (+0x24) and `pCachedValues` (+0x2c; its `iFlags` at +0x44).
//!
//! The process classes' slots, as the exe's vtables hold them (`BaseProcess` `01085e2c`,
//! `LowProcess` `010886e4`, `MiddleLowProcess` `01089bcc`, `MiddleHighProcess` `0108904c`,
//! `HighProcess` `01087864`): 0x238, 0x23c and 0x240 are `008d03c0`, `008d03e0` and `008d0400`
//! on all five; 0x22c is `007fa950` (the AI queries family's) on all five; 0x280 and 0x284 are
//! `008d0450` and `008d0470` on the first three. A call through one of them is the typed call
//! when the slot holds that body, and a call through the slot otherwise. A form's slot 0x130 (its
//! name) is [`empty_name`] on the classes that keep the default.

use super::FAMILY;
use super::{actor_package, middle_low_process};
use crate::{ai_queries, dispatch, multibound, slot_target};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The embedded current package (`CurrentPackage`, an `ActorPackage`).
const CURRENT_PACKAGE: u32 = 0x04;
/// `fSystemTimeClock`.
const SYSTEM_TIME_CLOCK: u32 = 0x1c;
/// `fHourLastProcessed`, a float.
const HOUR_LAST_PROCESSED: u32 = 0x20;
/// `iDateLastProcessed`.
const DATE_LAST_PROCESSED: u32 = 0x24;
/// `ActorPackage::iCurrentProcedureIndex`, in the embedded package.
const PROCEDURE_INDEX: u32 = 0x0c;
/// `pCachedValues`.
const CACHED_VALUES: u32 = 0x2c;
/// `CachedValues::iFlags`.
const CACHED_FLAGS: u32 = 0x44;

/// `BaseProcess`'s vtable, stored by the destructor.
const BASE_PROCESS_VTABLE: u32 = 0x0108_5e2c;
/// A global three-word point (zero in the exe image): copied out by one stub, answered by address
/// by another.
const SHARED_POINT: u32 = 0x011f_426c;
/// The float `FLT_MAX`, whose negation the last-time-detected stub answers.
const LARGEST_FLOAT: u32 = 0x0108_5df0;
/// The double 0.0 that `Copy` compares a slot's result with.
pub(super) const ZERO_DOUBLE: u32 = 0x0101_2060;
/// The float -1.0 that `Revert` (the buffer one) stores as the hour.
pub(super) const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
/// The float 20.0 the breath timer answers.
const BREATH_TIMER_FLOAT: u32 = 0x0101_7868;
/// The empty string forms name themselves with.
const EMPTY_STRING: u32 = 0x0101_1584;
/// The player character, a global.
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;

/// The save/load game object every save function works through, a global.
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
/// The setting whose value byte turns the save size messages on.
const SAVE_LOG_SETTING: u32 = 0x011d_e4e8;
/// The block tag written before a process block and checked after it: `KOLB`.
pub(super) const BLOCK_TAG: u32 = 0x424c_4f4b;
/// The source file's path the messages name (the AI folder's `BaseProcess` source).
const SOURCE_FILE: u32 = 0x0108_65e0;
/// `GetSaveSize(): %-5i for form %08X %s with flags %08X ending at line %i in file %s`.
pub(super) const SAVE_SIZE_FORM_FORMAT: u32 = 0x0101_2cb0;
/// `GetSaveSize(): %-5i ending at line %i in file %s`.
pub(super) const SAVE_SIZE_FORMAT: u32 = 0x0101_2c78;
/// `SaveGame(): %-5i for form %08X %s with flags %08X ending at line %i in file %s`.
pub(super) const SAVE_GAME_FORM_FORMAT: u32 = 0x0101_53a0;
/// `SaveGame(): %-5i ending at line %i in file %s`.
pub(super) const SAVE_GAME_FORMAT: u32 = 0x0101_536c;
/// The warning of a block longer than a 16-bit length holds.
pub(super) const BLOCK_TOO_LARGE_FORMAT: u32 = 0x0101_5318;
/// The warnings of a block whose tag is wrong: with the form being loaded, and with the version.
pub(super) const BAD_HEADER_FORM_FORMAT: u32 = 0x0101_5718;
pub(super) const BAD_HEADER_FORMAT: u32 = 0x0101_56a8;
/// The warnings of a block read past its end or short of it: with the form being loaded, and with
/// the version.
pub(super) const OVERRUN_FORM_FORMAT: u32 = 0x0101_5588;
pub(super) const UNDERRUN_FORM_FORMAT: u32 = 0x0101_5500;
pub(super) const OVERRUN_FORMAT: u32 = 0x0101_54a0;
pub(super) const UNDERRUN_FORMAT: u32 = 0x0101_5440;

/// The save/load object's buffer position (its word at +0x14, `00825c00`'s body), current
/// version (its byte at +0x80, `008df040`'s) and record being loaded (its word at +0x84,
/// `004fd3c0`'s).
const BUFFER_POSITION: u32 = 0x14;
const CURRENT_VERSION: u32 = 0x80;
const LOADING_RECORD: u32 = 0x84;
/// Writes `size` bytes from an address to the save buffer.
pub(super) const WRITE_BYTES: u32 = 0x0085_79b0;
/// Reads `size` bytes from the save buffer into an address.
pub(super) const READ_BYTES: u32 = 0x0085_79e0;
/// The form for a record's id word (`cdecl`).
pub(super) const FORM_FOR_ID: u32 = 0x0048_39c0;
/// The SAVELOAD warnings (`cdecl`, variable arguments). A fix or built-in may replace it, so it is
/// called by address.
pub(super) const SAVELOAD_WARNING: u32 = 0x005b_5e40;

/// `__RTDynamicCast` (`cdecl`, five words).
pub(super) const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// The RTTI type descriptors of the casts.
const TYPE_BASE_PROCESS: u32 = 0x0118_a6b8;
const TYPE_HIGH_PROCESS: u32 = 0x0118_a6d4;
const TYPE_MIDDLE_HIGH_PROCESS: u32 = 0x0118_c64c;
const TYPE_MIDDLE_LOW_PROCESS: u32 = 0x011a_3668;
const TYPE_LOW_PROCESS: u32 = 0x011a_364c;
const TYPE_MOBILE_OBJECT: u32 = 0x0118_4920;
pub(super) const TYPE_ACTOR: u32 = 0x0118_46d4;

/// The embedded current package of a process.
#[inline]
fn package_of(this: Ptr) -> u32 {
    this.addr().wrapping_add(CURRENT_PACKAGE)
}

/// The embedded current package, as a pointer.
#[inline]
fn package_ptr(this: Ptr) -> Ptr {
    Ptr::new(package_of(this))
}

/// `__RTDynamicCast(object, 0, from, to, 0)`.
pub(super) fn runtime_cast(e: &mut Engine, object: u32, from: u32, to: u32) -> u32 {
    e.call(RT_DYNAMIC_CAST, &args![object, 0u32, from, to, 0u32])
        .u32()
}

pub(super) fn save_load_game(e: &Engine) -> u32 {
    e.mem.u32(SAVE_LOAD_GAME)
}

/// `TESSaveLoadGame::UseSaveGameBlocks` (Xbox PDB, `00862110`): the current version from 0x1f
/// to 0x59.
pub(super) fn use_save_game_blocks(e: &Engine) -> bool {
    (0x1f..0x5a).contains(&current_version(e))
}

/// The save/load object's current version (`008df040`).
pub(super) fn current_version(e: &Engine) -> u8 {
    e.mem.u8(save_load_game(e).wrapping_add(CURRENT_VERSION))
}

/// The save/load object's record being loaded (`004fd3c0`).
pub(super) fn loading_record(e: &Engine) -> u32 {
    e.mem.u32(save_load_game(e).wrapping_add(LOADING_RECORD))
}

pub(super) fn buffer_position(e: &Engine) -> u32 {
    e.mem.u32(save_load_game(e).wrapping_add(BUFFER_POSITION))
}

/// The save log setting's value byte (`00408d60` gives a setting's value at +4).
pub(super) fn save_log_enabled(e: &Engine) -> bool {
    e.mem.u8(SAVE_LOG_SETTING + 4) != 0
}

/// The record being saved: the save/load object's word at +0x88 (`TES::GetWorldSpace`'s body,
/// the multibound family's).
pub(super) fn saving_record(e: &mut Engine) -> u32 {
    let save_load = save_load_game(e);
    typed_call!(
        multibound::FAMILY,
        0x004f_d3e0,
        multibound::getters::world_space(e, Ptr::new(save_load))
    )
}

/// A form's name (its slot 0x130): [`empty_name`] when the slot holds it.
pub(super) fn form_name(e: &mut Engine, form: u32) -> u32 {
    if slot_target(e, form, 0x130) == 0x0040_1280 {
        typed_call!(FAMILY, 0x0040_1280, empty_name(e, Ptr::new(form)))
    } else {
        e.vcall(form, 0x130, &[]).u32()
    }
}

/// The 16-bit length a block's placeholder holds: the low half of the bytes written.
#[allow(clippy::cast_possible_truncation)] // A 16-bit store, as the exe's.
pub(super) fn low_half(word: u32) -> u16 {
    word as u16
}

/// The words a save message prints for a record, in the order the code reads them: flags (+5) and
/// version (+9) before the name slot, the id after it.
pub(super) struct RecordWords {
    pub(super) form_id: u32,
    pub(super) name: u32,
    pub(super) version: u32,
    pub(super) flags: u32,
}

/// A record's message words: the form for its id, then the record's flags (and version when
/// `with_version`), then the form's name (slot 0x130), then its id again.
pub(super) fn record_words(e: &mut Engine, record: u32, with_version: bool) -> RecordWords {
    let id = e.mem.u32(record);
    let form = e.call(FORM_FOR_ID, &args![id]).u32();
    let flags = e.mem.u32(record.wrapping_add(5));
    let version = if with_version {
        u32::from(e.mem.u8(record.wrapping_add(9)))
    } else {
        0
    };
    let name = form_name(e, form);
    let form_id = e.mem.u32(record);
    RecordWords {
        form_id,
        name,
        version,
        flags,
    }
}

// Typed from 00401280
/// The name getter of a form that keeps the default (its slot 0x130): the empty string at
/// `01011584`, the receiver unread.
pub fn empty_name(_e: &mut Engine, _this: Ptr) -> u32 {
    EMPTY_STRING
}

// Typed from 008d02d0
/// `BaseProcess::GetLastTimeDetected` (Xbox PDB): the float at `01085df0` (`FLT_MAX`) loaded
/// (`FLD`, which quiets a signalling NaN) and negated (`FCHS`: the sign bit alone); both words
/// unread.
pub fn last_time_detected(e: &mut Engine, _this: Ptr, _unused_0: u32, _unused_1: u32) -> f32 {
    f32::from_bits(x87_quiet_bits(e.mem.u32(LARGEST_FLOAT)) ^ 0x8000_0000)
}

// Typed from 008d02f0
/// False; six words unread (the slot 0x44 default).
#[allow(clippy::too_many_arguments)] // The exe's argument words.
pub fn false_6(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
) -> bool {
    false
}

// Typed from 008d0300
/// False; two words unread (the default of many slots).
pub fn false_2(_e: &mut Engine, _this: Ptr, _unused_0: u32, _unused_1: u32) -> bool {
    false
}

// Typed from 008d0310
/// The three words of the global point at `011f426c` copied into `result`, which it answers; two
/// more words unread (the slots 0x1c and 0x98 defaults).
pub fn shared_point_copy(
    e: &mut Engine,
    _this: Ptr,
    result: Ptr,
    _unused_0: u32,
    _unused_1: u32,
) -> Ptr {
    for offset in [0, 4, 8] {
        let word = e.mem.u32(SHARED_POINT + offset);
        e.mem.set_u32(result.addr().wrapping_add(offset), word);
    }
    result
}

// Typed from 008d0340
/// 0; seven words unread (the slot 0xf0 default).
#[allow(clippy::too_many_arguments)] // The exe's argument words.
pub fn zero_7(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    _unused_6: u32,
) -> u32 {
    0
}

// Typed from 008d0350
/// Nothing; seven words unread (the slot 0xfc default).
#[allow(clippy::too_many_arguments)] // The exe's argument words.
pub fn nothing_7(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    _unused_6: u32,
) {
}

// Typed from 008d0370
/// False; one word unread (the default of many slots).
pub fn false_1(_e: &mut Engine, _this: Ptr, _unused_0: u32) -> bool {
    false
}

// Typed from 008d0380
/// Nothing; eleven words unread (the slot 0x178 default).
#[allow(clippy::too_many_arguments)] // The exe's argument words.
pub fn nothing_11(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    _unused_6: u32,
    _unused_7: u32,
    _unused_8: u32,
    _unused_9: u32,
    _unused_10: u32,
) {
}

// Typed from 008d0390
/// 0.0 (`FLDZ`); two words unread (the slot 0x208 default).
pub fn float_zero_2(_e: &mut Engine, _this: Ptr, _unused_0: u32, _unused_1: u32) -> f32 {
    0.0
}

// Typed from 008d03a0
/// `BaseProcess::ClearCurrentPackage` (Xbox PDB): `ActorPackage::Clear` (`008c5ef0`) on the
/// embedded current package.
pub fn clear_current_package(e: &mut Engine, this: Ptr) {
    let package = package_ptr(this);
    typed_call!(FAMILY, 0x008c_5ef0, actor_package::clear(e, package));
}

// Typed from 008d03c0
/// `BaseProcess::SetCurrentProcedureIndex` (Xbox PDB, the slot 0x238): the embedded current
/// package's procedure index (+0x0c) stored (`0041fd00`'s body).
pub fn set_current_procedure_index(e: &mut Engine, this: Ptr, index: u32) {
    e.mem
        .set_u32(package_of(this).wrapping_add(PROCEDURE_INDEX), index);
}

// Typed from 008d03e0
/// The slot 0x23c (`GetCurrentProcedureIndex`, Xbox PDB): the embedded current package's word at
/// +0x0c, read through `0084e3a0`.
pub fn current_procedure_index(e: &mut Engine, this: Ptr) -> u32 {
    let package = package_of(this);
    typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, package))
}

// Typed from 008d0400
/// `BaseProcess::AddToCurrentProcedureIndex` (Xbox PDB, the slot 0x240): `amount` added to the
/// embedded current package's procedure index (+0x0c; `0049bb50`'s body).
pub fn add_to_current_procedure_index(e: &mut Engine, this: Ptr, amount: u32) {
    let at = package_of(this).wrapping_add(PROCEDURE_INDEX);
    let index = e.mem.u32(at).wrapping_add(amount);
    e.mem.set_u32(at, index);
}

// Typed from 008d0420
/// Nothing; eight words unread (the slot 0x250 default).
#[allow(clippy::too_many_arguments)] // The exe's argument words.
pub fn nothing_8(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    _unused_6: u32,
    _unused_7: u32,
) {
}

// Typed from 008d0430
/// The slot 0x274: the embedded current package's word at +0x04, read through `00726070`.
pub fn running_package_word(e: &mut Engine, this: Ptr) -> u32 {
    let package = package_of(this);
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, package))
}

// Typed from 008d0450
/// `BaseProcess::GetProcedureIndexRunning` (Xbox PDB): the receiver's slot 0x23c.
pub fn procedure_index_running(e: &mut Engine, this: Ptr) -> u32 {
    if slot_target(e, this.addr(), 0x23c) == 0x008d_03e0 {
        typed_call!(FAMILY, 0x008d_03e0, current_procedure_index(e, this))
    } else {
        e.vcall(this.addr(), 0x23c, &[]).u32()
    }
}

// Typed from 008d0470
/// `BaseProcess::SetProcedureIndexRunning` (Xbox PDB): the receiver's slot 0x238 with `index`.
pub fn set_procedure_index_running(e: &mut Engine, this: Ptr, index: u32) {
    if slot_target(e, this.addr(), 0x238) == 0x008d_03c0 {
        typed_call!(
            FAMILY,
            0x008d_03c0,
            set_current_procedure_index(e, this, index)
        );
    } else {
        e.vcall(this.addr(), 0x238, &args![index]);
    }
}

// Typed from 008d04a0
/// `BaseProcess::AddToProcedureIndexRunning` (Xbox PDB): the receiver's slot 0x240 with the second
/// word; the first is unread.
pub fn add_to_procedure_index_running(e: &mut Engine, this: Ptr, _unused_0: u32, amount: u32) {
    if slot_target(e, this.addr(), 0x240) == 0x008d_0400 {
        typed_call!(
            FAMILY,
            0x008d_0400,
            add_to_current_procedure_index(e, this, amount)
        );
    } else {
        e.vcall(this.addr(), 0x240, &args![amount]);
    }
}

// Typed from 008d04d0
/// `BaseProcess::SetCharController` (Xbox PDB): only the release of the counted pointer it was
/// handed by value (`NiPointer::~NiPointer`, `0045cec0`, the dispatch family's, on the argument's
/// word).
pub fn set_char_controller(e: &mut Engine, _this: Ptr, controller: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), controller);
        typed_call!(
            dispatch::FAMILY,
            0x0045_cec0,
            dispatch::refs::release_held(e, slot)
        );
    });
}

// Typed from 008d04f0
/// Nothing; six words unread (the slot 0x2a4 default).
#[allow(clippy::too_many_arguments)] // The exe's argument words.
pub fn nothing_6(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
) {
}

// Typed from 008d0500
/// Nothing; five words unread (the slots 0x2bc and 0x418 default).
pub fn nothing_5(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
) {
}

// Typed from 008d0510
/// False; four words unread (the slot 0x2cc default).
pub fn false_4(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
) -> bool {
    false
}

// Typed from 008d0520
/// False; three words unread (the default of several slots).
pub fn false_3(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
) -> bool {
    false
}

// Typed from 008d0530
/// False; thirteen words unread (the slot 0x33c default).
#[allow(clippy::too_many_arguments)] // The exe's argument words.
pub fn false_13(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    _unused_6: u32,
    _unused_7: u32,
    _unused_8: u32,
    _unused_9: u32,
    _unused_10: u32,
    _unused_11: u32,
    _unused_12: u32,
) -> bool {
    false
}

// Typed from 008d0540
/// `BaseProcess::CalculateMoveMode` (Xbox PDB): 0x101; six words unread.
#[allow(clippy::too_many_arguments)] // The exe's argument words.
pub fn calculate_move_mode(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
) -> u32 {
    0x101
}

// Typed from 008d0560
/// 0; one word unread (the default of many slots).
pub fn zero_1(_e: &mut Engine, _this: Ptr, _unused_0: u32) -> u32 {
    0
}

// Typed from 008d0570
/// -1; three words unread (the slot 0x398 default).
pub fn minus_one_3(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
) -> i32 {
    -1
}

// Typed from 008d0580
/// 0.0 (`FLDZ`); three words unread (the slot 0x39c default).
pub fn float_zero_3(
    _e: &mut Engine,
    _this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    _unused_2: u32,
) -> f32 {
    0.0
}

// Typed from 008d0590
/// `BaseProcess::GetSoundHandle` (Xbox PDB): an empty sound handle made in `result` (`0041a250`'s
/// body: the id -1, the byte at +4 and the word at +8 0), which it answers; the second word unread.
pub fn sound_handle(e: &mut Engine, _this: Ptr, result: Ptr, _unused_0: u32) -> Ptr {
    let at = result.addr();
    e.mem.set_u32(at, u32::MAX);
    e.mem.set_u8(at.wrapping_add(4), 0);
    e.mem.set_u32(at.wrapping_add(8), 0);
    result
}

// Typed from 008d0610
/// `BaseProcess`'s scalar deleting destructor: the destructor body, then the object freed through
/// `00401030` when bit 0 of `flags` is set; answers `this`.
pub fn scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x008d_0640, destructor(e, this));
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Typed from 008d0640
/// The destructor body: `BaseProcess`'s vtable stored, the embedded current package destroyed
/// (`008c5e70`).
pub fn destructor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), BASE_PROCESS_VTABLE);
    let package = package_ptr(this);
    typed_call!(FAMILY, 0x008c_5e70, actor_package::destroy_data(e, package));
}

/// `Copy`'s step for one high process (none for 0): when its slot 0x1e0 answers more than the
/// double at `01012060` (`FCOMP; TEST AH,0x41; JNZ`: only an ordered "greater" goes on), its slot
/// 0x214, then its slot 0x12c with 0.
fn reset_high_process(e: &mut Engine, process: u32) {
    if process == 0 {
        return;
    }
    let value = e.vcall(process, 0x1e0, &[]).f64();
    let threshold = f64::from_bits(e.mem.u64(ZERO_DOUBLE));
    if value > threshold {
        e.vcall(process, 0x214, &[]);
        e.vcall(process, 0x12c, &args![0u32]);
    }
}

// Typed from 008d0660
/// `BaseProcess::Copy` (Xbox PDB): nothing for a null `other`. Otherwise `other`, then `this`, cast
/// to `HighProcess`, `MiddleHighProcess`, `MiddleLowProcess` and `LowProcess`; each high process
/// reset (`other`'s first); then for each level both are, that level's copy on `this` with `other`
/// (`008d9fa0` `HighProcess::HighCopy`, `00914c60` `MiddleHighProcess::MiddleHighCopy`, `0092ca70`
/// (the middle-low module's, a typed call), `00907510`); last, when both have cached values, `008675e0` on this one's with the other's.
pub fn copy(e: &mut Engine, this: Ptr, other: Ptr) {
    if other.is_null() {
        return;
    }
    let levels = [
        TYPE_HIGH_PROCESS,
        TYPE_MIDDLE_HIGH_PROCESS,
        TYPE_MIDDLE_LOW_PROCESS,
        TYPE_LOW_PROCESS,
    ];
    let mut other_as = [0u32; 4];
    for (cast, level) in other_as.iter_mut().zip(levels) {
        *cast = runtime_cast(e, other.addr(), TYPE_BASE_PROCESS, level);
    }
    let mut this_as = [0u32; 4];
    for (cast, level) in this_as.iter_mut().zip(levels) {
        *cast = runtime_cast(e, this.addr(), TYPE_BASE_PROCESS, level);
    }
    reset_high_process(e, other_as[0]);
    reset_high_process(e, this_as[0]);
    let level_copies = [0x008d_9fa0u32, 0x0091_4c60, 0x0092_ca70, 0x0090_7510];
    for ((target, source), level_copy) in this_as.into_iter().zip(other_as).zip(level_copies) {
        if target != 0 && source != 0 {
            if level_copy == 0x0092_ca70 {
                typed_call!(
                    FAMILY,
                    0x0092_ca70,
                    middle_low_process::copy(e, Ptr::new(target), Ptr::new(source))
                );
            } else {
                e.call(level_copy, &args![target, source]);
            }
        }
    }
    let this_cached = e.mem.u32(this.addr().wrapping_add(CACHED_VALUES));
    if this_cached != 0 {
        let other_cached = e.mem.u32(other.addr().wrapping_add(CACHED_VALUES));
        if other_cached != 0 {
            e.call(0x0086_75e0, &args![this_cached, other_cached]);
        }
    }
}

// Typed from 008d0870
/// `BaseProcess::HandleDeletedReference` (Xbox PDB): `reference` handed to the embedded package's
/// handler (`008c5ec0`), then to the slot 0x04 of the object [`running_package_word`] answers, when
/// there is one.
pub fn handle_deleted_reference(e: &mut Engine, this: Ptr, reference: u32) {
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_5ec0,
        actor_package::handle_deleted_reference(e, package, reference)
    );
    let running = typed_call!(FAMILY, 0x008d_0430, running_package_word(e, this));
    if running != 0 {
        e.vcall(running, 0x4, &args![reference]);
    }
}

// Typed from 008d08b0
/// `BaseProcess::SetCurrentPackage` (Xbox PDB): the package's first target for `actor`
/// (`TESPackage::ComputeInitialTarget`, `006759e0`), then the embedded package started from both
/// (`ActorPackage::InitFromPackage`, `008c6080`).
pub fn set_current_package(e: &mut Engine, this: Ptr, package: u32, actor: u32) {
    let target = e.call(0x0067_59e0, &args![package, actor]).u32();
    let embedded = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_6080,
        actor_package::init_from_package(e, embedded, Ptr::new(package), target)
    );
}

// Typed from 008d08f0
/// `BaseProcess::GetWeaponLastPos` (Xbox PDB): the address of the global point at `011f426c`.
pub fn weapon_last_pos(_e: &mut Engine, _this: Ptr) -> u32 {
    SHARED_POINT
}

// Typed from 008d0900
/// `BaseProcess::GetSaveSize` (Xbox PDB): the 16-bit size of the process block: 6 for the block
/// header when save game blocks are on, the embedded package's size (`008c61a0` with `argument`),
/// and 4 for `fSystemTimeClock`. With the save log setting on, it is printed through `Error` (line
/// 0xe8), with the record being saved when there is one. The second word unread.
pub fn save_size(e: &mut Engine, this: Ptr, argument: u32, _unused_0: u32) -> u16 {
    let mut size: u16 = 0;
    if use_save_game_blocks(e) {
        size = size.wrapping_add(4);
        size = size.wrapping_add(2);
    }
    let package = package_ptr(this);
    let package_size = low_half(typed_call!(
        FAMILY,
        0x008c_61a0,
        actor_package::save_size(e, package, argument)
    ));
    size = size.wrapping_add(package_size);
    size = size.wrapping_add(4);
    if save_log_enabled(e) {
        let record = saving_record(e);
        if record != 0 {
            let words = record_words(e, record, false);
            typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words SAVE_SIZE_FORM_FORMAT, u32::from(size), words.form_id, words.name, words.flags, 0xe8u32, SOURCE_FILE);
        } else {
            typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words SAVE_SIZE_FORMAT, u32::from(size), 0xe8u32, SOURCE_FILE);
        }
    }
    size
}

// Typed from 008d0a10
/// `BaseProcess::SaveGame` (Xbox PDB): with save game blocks, the block tag `KOLB` and a 16-bit
/// zero length placeholder; then the embedded package (`008c6250`) and `fSystemTimeClock`. With the
/// save log setting on, the bytes written since the start printed (`Error`, line 0xf9). With
/// blocks, the block's length (the position now less the placeholder's) stored as 16 bits at the
/// placeholder, with a warning first when it is past 0xffff. The second word unread.
pub fn save_game(e: &mut Engine, this: Ptr, argument: u32, _unused_0: u32) {
    let mut length_at: u32 = 0;
    let mut start = buffer_position(e);
    if save_log_enabled(e) {
        start = buffer_position(e);
    }
    if use_save_game_blocks(e) {
        e.with_stack(4, |e, tag| {
            e.mem.set_u32(tag.addr(), BLOCK_TAG);
            let save_load = save_load_game(e);
            e.call(WRITE_BYTES, &args![save_load, tag, 4u32]);
        });
        length_at = buffer_position(e);
        e.with_stack(4, |e, length| {
            let save_load = save_load_game(e);
            e.call(WRITE_BYTES, &args![save_load, length, 2u32]);
        });
    }
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_6250,
        actor_package::save(e, package, argument)
    );
    let save_load = save_load_game(e);
    let clock = this.addr().wrapping_add(SYSTEM_TIME_CLOCK);
    e.call(WRITE_BYTES, &args![save_load, clock, 4u32]);
    if save_log_enabled(e) {
        let end = buffer_position(e);
        let record = saving_record(e);
        let written = end.wrapping_sub(start);
        if record != 0 {
            let words = record_words(e, record, false);
            typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words SAVE_GAME_FORM_FORMAT, written, words.form_id, words.name, words.flags, 0xf9u32, SOURCE_FILE);
        } else {
            typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words SAVE_GAME_FORMAT, written, 0xf9u32, SOURCE_FILE);
        }
    }
    if use_save_game_blocks(e) {
        let end = buffer_position(e);
        if end > length_at.wrapping_add(0xffff) {
            e.call(
                SAVELOAD_WARNING,
                &args![BLOCK_TOO_LARGE_FORMAT, SOURCE_FILE, 0xf9u32],
            );
        }
        e.mem
            .set_u16(length_at, low_half(end.wrapping_sub(length_at)));
    }
}

/// `LoadGame`'s tag check: the warning at line 0x104, with the record being loaded or with the
/// current version.
fn bad_header(e: &mut Engine) {
    let record = loading_record(e);
    if record != 0 {
        let words = record_words(e, record, true);
        e.call(
            SAVELOAD_WARNING,
            &args![
                BAD_HEADER_FORM_FORMAT,
                SOURCE_FILE,
                0x104u32,
                words.form_id,
                words.name,
                words.version,
                words.flags
            ],
        );
    } else {
        let current = u32::from(current_version(e));
        e.call(
            SAVELOAD_WARNING,
            &args![BAD_HEADER_FORMAT, SOURCE_FILE, 0x104u32, current],
        );
    }
}

// Typed from 008d0bc0
/// `BaseProcess::LoadGame` (Xbox PDB): with save game blocks, the block tag read (the warning at
/// line 0x104 when it is not `KOLB`) and the 16-bit block length after the position is noted. Then
/// the embedded package (`ActorPackage::LoadGame`, `008c6320`, with the first two words) and
/// `fSystemTimeClock`. With blocks, the position compared with where the block should end, and an
/// overrun or underrun warned of (line 0x109), with the record being loaded or with the current
/// version. The third word unread.
pub fn load_game(e: &mut Engine, this: Ptr, argument_0: u32, argument_1: u32, _unused_0: u32) {
    let mut length: u16 = 0;
    let mut block_start: u32 = 0;
    if use_save_game_blocks(e) {
        let tag = e.with_stack(4, |e, tag| {
            let save_load = save_load_game(e);
            e.call(READ_BYTES, &args![save_load, tag, 4u32]);
            e.mem.u32(tag.addr())
        });
        if tag != BLOCK_TAG {
            bad_header(e);
        }
        block_start = buffer_position(e);
        length = e.with_stack(4, |e, slot| {
            let save_load = save_load_game(e);
            e.call(READ_BYTES, &args![save_load, slot, 2u32]);
            e.mem.u16(slot.addr())
        });
    }
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_6320,
        actor_package::load_game(e, package, argument_0, argument_1)
    );
    let save_load = save_load_game(e);
    let clock = this.addr().wrapping_add(SYSTEM_TIME_CLOCK);
    e.call(READ_BYTES, &args![save_load, clock, 4u32]);
    if !use_save_game_blocks(e) {
        return;
    }
    let end = buffer_position(e);
    let record = loading_record(e);
    let expected = u32::from(length).wrapping_add(block_start);
    if record != 0 {
        let id = e.mem.u32(record);
        let form = e.call(FORM_FOR_ID, &args![id]).u32();
        let (format, bytes) = if end > expected {
            (OVERRUN_FORM_FORMAT, end.wrapping_sub(expected))
        } else if end < expected {
            (UNDERRUN_FORM_FORMAT, expected.wrapping_sub(end))
        } else {
            return;
        };
        let flags = e.mem.u32(record.wrapping_add(5));
        let record_version = u32::from(e.mem.u8(record.wrapping_add(9)));
        let name = form_name(e, form);
        let form_id = e.mem.u32(record);
        e.call(
            SAVELOAD_WARNING,
            &args![
                format,
                bytes,
                SOURCE_FILE,
                0x109u32,
                form_id,
                name,
                record_version,
                flags
            ],
        );
    } else {
        let (format, bytes) = if end > expected {
            (OVERRUN_FORMAT, end.wrapping_sub(expected))
        } else if end < expected {
            (UNDERRUN_FORMAT, expected.wrapping_sub(end))
        } else {
            return;
        };
        let current = u32::from(current_version(e));
        e.call(
            SAVELOAD_WARNING,
            &args![format, bytes, SOURCE_FILE, 0x109u32, current],
        );
    }
}

// Typed from 008d0e60
/// `BaseProcess::InitLoadGame` (Xbox PDB): `ActorPackage::InitLoadGame` (`008c64d0`) on the
/// embedded package with the first two words; the third unread.
pub fn init_load_game(e: &mut Engine, this: Ptr, argument_0: u32, argument_1: u32, _unused_0: u32) {
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_64d0,
        actor_package::init_load_game(e, package, argument_0, argument_1)
    );
}

// Typed from 008d0e80
/// `BaseProcess::FinishInitLoadGame` (Xbox PDB): when the third word casts from `MobileObject` to
/// an `Actor` that is not the player, the process's slot 0x22c answers 0 and its slot 0x280
/// answers -1: its slot 0x284 with 0, then `Actor::EvaluatePackage` (`008a6ce0`) on the actor with
/// (1, 1). The first two words unread.
pub fn finish_init_load_game(
    e: &mut Engine,
    this: Ptr,
    _unused_0: u32,
    _unused_1: u32,
    object: u32,
) {
    let actor = runtime_cast(e, object, TYPE_MOBILE_OBJECT, TYPE_ACTOR);
    if actor == 0 || actor == e.mem.u32(PLAYER_CHARACTER) {
        return;
    }
    let process = this.addr();
    if ai_queries::queries::process_slot_22c(e, process) != 0 {
        return;
    }
    let running = if slot_target(e, process, 0x280) == 0x008d_0450 {
        typed_call!(FAMILY, 0x008d_0450, procedure_index_running(e, this))
    } else {
        e.vcall(process, 0x280, &[]).u32()
    };
    if running != u32::MAX {
        return;
    }
    if slot_target(e, process, 0x284) == 0x008d_0470 {
        typed_call!(
            FAMILY,
            0x008d_0470,
            set_procedure_index_running(e, this, 0u32)
        );
    } else {
        e.vcall(process, 0x284, &args![0u32]);
    }
    e.call(0x008a_6ce0, &args![actor, 1u32, 1u32]);
}

// Typed from 008d0f10
/// `BaseProcess::Revert` (Xbox PDB): `008c65a0` on the embedded package with the first word; the
/// second unread.
pub fn revert(e: &mut Engine, this: Ptr, argument: u32, _unused_0: u32) {
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_65a0,
        actor_package::revert_unused(e, package, argument)
    );
}

// Typed from 008d0f30
/// `BaseProcess::SaveGame` of a buffer (Xbox PDB `SaveGame_ov2`): `fSystemTimeClock`,
/// `fHourLastProcessed` and `iDateLastProcessed` written (4 bytes each, through `00865e50` with a
/// last 0), then `ActorPackage::SaveGame` (`008c65d0`) on the embedded package.
pub fn save_game_buffer(e: &mut Engine, this: Ptr, buffer: u32) {
    for field in [SYSTEM_TIME_CLOCK, HOUR_LAST_PROCESSED, DATE_LAST_PROCESSED] {
        let at = this.addr().wrapping_add(field);
        e.call(0x0086_5e50, &args![buffer, at, 4u32, 0u32]);
    }
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_65d0,
        actor_package::save_game(e, package, Ptr::new(buffer))
    );
}

// Typed from 008d0f90
/// `BaseProcess::LoadGame` of a buffer (Xbox PDB `LoadGame_ov2`): `fSystemTimeClock` read
/// (`00864980`, 4 bytes); when the buffer's slot 0x00 answers a byte of at least 6 (`MOVZX`), also
/// `fHourLastProcessed` and `iDateLastProcessed`; then `ActorPackage::LoadGame_ov2` (`008c6730`) on
/// the embedded package.
pub fn load_game_buffer(e: &mut Engine, this: Ptr, buffer: u32) {
    let clock = this.addr().wrapping_add(SYSTEM_TIME_CLOCK);
    e.call(0x0086_4980, &args![buffer, clock, 4u32]);
    let buffer_version = i32::from(e.vcall(buffer, 0x0, &[]).u8());
    if buffer_version >= 6 {
        for field in [HOUR_LAST_PROCESSED, DATE_LAST_PROCESSED] {
            let at = this.addr().wrapping_add(field);
            e.call(0x0086_4980, &args![buffer, at, 4u32]);
        }
    }
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_6730,
        actor_package::load_game_buffer(e, package, Ptr::new(buffer))
    );
}

/// The cached values' flags cleared ([`cached_values_out_of_date`]) when the process has cached
/// values.
fn cached_out_of_date(e: &mut Engine, this: Ptr) {
    let cached = e.mem.u32(this.addr().wrapping_add(CACHED_VALUES));
    if cached != 0 {
        typed_call!(
            FAMILY,
            0x008d_1030,
            cached_values_out_of_date(e, Ptr::new(cached))
        );
    }
}

// Typed from 008d1000
/// `BaseProcess::InitLoadGame` of a buffer (Xbox PDB `InitLoadGame_ov2`):
/// `ActorPackage::InitLoadGame_ov2` (`008c6970`) on the embedded package, then the cached values'
/// flags cleared when there are cached values.
pub fn init_load_game_buffer(e: &mut Engine, this: Ptr, buffer: u32) {
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_6970,
        actor_package::init_load_game_buffer(e, package, Ptr::new(buffer))
    );
    cached_out_of_date(e, this);
}

// Typed from 008d1030
/// `CachedValues::iFlags` (+0x44) cleared: every cached value out of date.
pub fn cached_values_out_of_date(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr().wrapping_add(CACHED_FLAGS), 0);
}

// Typed from 008d1050
/// `BaseProcess::FinishLoadGame` (Xbox PDB): `ActorPackage::FinishLoadGame` (`008c6b20`) on the
/// embedded package.
pub fn finish_load_game(e: &mut Engine, this: Ptr, buffer: u32) {
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_6b20,
        actor_package::finish_load_game(e, package, Ptr::new(buffer))
    );
}

// Typed from 008d1070
/// `BaseProcess::Revert` of a buffer (Xbox PDB `Revert_ov2`): `fHourLastProcessed` set to the float
/// at `01012054` (-1.0; moved through the x87 stack, which quiets a signalling NaN) and
/// `iDateLastProcessed` to 0, the embedded package reverted (`ActorPackage::Revert`, `008c6c80`),
/// and the cached values' flags cleared when there are cached values.
pub fn revert_buffer(e: &mut Engine, this: Ptr, buffer: u32) {
    let hour = x87_quiet_bits(e.mem.u32(MINUS_ONE_FLOAT));
    e.mem
        .set_u32(this.addr().wrapping_add(HOUR_LAST_PROCESSED), hour);
    e.mem
        .set_u32(this.addr().wrapping_add(DATE_LAST_PROCESSED), 0);
    let package = package_ptr(this);
    typed_call!(
        FAMILY,
        0x008c_6c80,
        actor_package::revert(e, package, buffer)
    );
    cached_out_of_date(e, this);
}

// Typed from 008d10c0
/// `BaseProcess::GetBreathTimer` (Xbox PDB): the float at `01017868` (20.0), loaded (`FLD`, which
/// quiets a signalling NaN).
pub fn breath_timer(e: &mut Engine, _this: Ptr) -> f32 {
    f32::from_bits(x87_quiet_bits(e.mem.u32(BREATH_TIMER_FLOAT)))
}
