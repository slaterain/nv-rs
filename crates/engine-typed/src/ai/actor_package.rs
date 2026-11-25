//! `ActorPackage` (the Xbox PDB's source unit of that name, in `fallout/ai`): the package an actor
//! runs, as a process holds it (the current package is embedded in every process at +0x04). Every
//! function of the unit, and the two the unit's code shares with others: the data's delete
//! (`008c5e70`, the destructor body) and the deleted reference handler (`008c5ec0`, folded with
//! `GuardActorPackageData::HandleDeletedReference`).
//!
//! The record (0x18 bytes; Xbox PDB names, checked against the PC code): `pPackage` (+0x00, a
//! `TESPackage`), `pData` (+0x04, its `ActorPackageData`), the target (+0x08, a reference, or its
//! form id while a load is under way), `iCurrentProcedureIndex` (+0x0c), `fPackageStartTime`
//! (+0x10, a float) and the flags (+0x14; bit 0: `pPackage` is still the id of a created package).
//!
//! In place of a call: the package's procedure (`009611e0`, its word at +0x18), the procedure
//! count (`0096a820`, a signed byte of the table at `011a3fbc` for 0 to 0x30, else -1), the
//! package's flag 0x800 (`00674d40`, its word at +0x1c), the data handler's created-form test
//! (`00469860`, an id of `0xff000000` and up), the save/load game's position step (`00857bd0`,
//! its word at +0x14) and the procedure index's store (`0041fd00`). `ActorPackage::Clear` asks the
//! save/load game `0047c850`, whose body answers false (`XOR AL,AL`), so the package is always
//! deleted through its slot 0x10 there. The blocks the exe guards with a word ANDed with 0 (its
//! folded `if (0 & word)`) never run, so they are left out: the save size is 8 or 12, and
//! `008c65a0` does nothing.

use super::FAMILY;
use crate::{ai_queries, dispatch, slot_target};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{x87_quiet_bits, x87_store32};
use engine_core::{args, typed_call, Engine};

/// `pPackage`.
const PACKAGE: u32 = 0x00;
/// `pData`.
const DATA: u32 = 0x04;
/// The target.
const TARGET: u32 = 0x08;
/// `iCurrentProcedureIndex`.
const PROCEDURE_INDEX: u32 = 0x0c;
/// `fPackageStartTime`.
const START_TIME: u32 = 0x10;
/// The flags.
const FLAGS: u32 = 0x14;

/// A `TESPackage`'s procedure, flags and reference count.
const PACKAGE_PROCEDURE: u32 = 0x18;
const PACKAGE_FLAGS: u32 = 0x1c;
const PACKAGE_REF_COUNT: u32 = 0x7c;

/// The player character, a global, and its package being run (+0x208).
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;
const PLAYER_PACKAGE: u32 = 0x208;
/// The float -1.0 an empty record's start time is.
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
/// The save/load game object, a global, and its buffer position (+0x14).
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const BUFFER_POSITION: u32 = 0x14;
/// The `Calendar` singleton.
const CALENDAR: u32 = 0x011d_e7b8;
/// The `BGSSaveLoadGame` object, a global, whose changed form ids a load maps through.
const CHANGED_FORMS: u32 = 0x011d_df38;
/// Each procedure's count, a signed byte (`0096a820`'s table, 0x31 of them).
const PROCEDURE_COUNTS: u32 = 0x011a_3fbc;
/// The package type names, one string pointer per type.
const PACKAGE_TYPE_NAMES: u32 = 0x0119_bcb0;

/// `Calendar::GetHour` (a float in ST0).
const CALENDAR_GET_HOUR: u32 = 0x0086_7da0;
/// `ActorPackageData::CreateActorPackageData` (`cdecl`, the package type).
const CREATE_ACTOR_PACKAGE_DATA: u32 = 0x008c_6ca0;
/// `TESPackage::CreatePackage` (`cdecl`, the type).
const CREATE_PACKAGE: u32 = 0x0067_0b90;
/// `TESPackage::CalculateProcedureType` (`thiscall`, one word).
const CALCULATE_PROCEDURE_TYPE: u32 = 0x0067_77b0;
/// `TESObjectREFR::SetTargeted` (`thiscall`, one word).
const SET_TARGETED: u32 = 0x0056_4db0;
/// The save/load game's `SaveNumericID`, byte writer, `LoadNumericID` (a byte answer) and byte
/// reader (`thiscall`, an address and a size).
const SAVE_NUMERIC_ID: u32 = 0x0085_7a10;
const WRITE_BYTES: u32 = 0x0085_79b0;
const LOAD_NUMERIC_ID: u32 = 0x0085_7aa0;
const READ_BYTES: u32 = 0x0085_79e0;
/// The save buffer's `SaveFormID_ov2` (`thiscall`, a form and a word; the save family's) and byte
/// writer (`thiscall`, an address, a size and a word).
const SAVE_FORM_ID: u32 = 0x0086_5df0;
const BUFFER_WRITE: u32 = 0x0086_5e50;
/// The load buffer's `LoadFormID_ov2` (`thiscall`, the id's address; a byte answer) and byte reader
/// (`thiscall`, an address and a size).
const LOAD_FORM_ID: u32 = 0x0086_48e0;
const BUFFER_READ: u32 = 0x0086_4980;
/// `BGSSaveLoadGame::AddChangedFormID` (`thiscall`, two ids) and the changed id's lookup
/// (`thiscall`, one id).
const ADD_CHANGED_FORM_ID: u32 = 0x0084_aa30;
const CHANGED_FORM_LOOKUP: u32 = 0x0084_aa90;
/// The form for an id (`cdecl`) and `__RTDynamicCast` (`cdecl`, five words) from `TESForm` to
/// `TESPackage` or `TESObjectREFR`.
const FORM_FOR_ID: u32 = 0x0048_39c0;
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_PACKAGE: u32 = 0x0118_46a0;
const TYPE_TES_OBJECT_REFR: u32 = 0x0118_41cc;
/// The AI log (`cdecl`, a format and its words; a fix may replace it, so it is called by address)
/// and `SaveGameWarning` (`cdecl`, likewise).
const AI_LOG: u32 = 0x005b_5e40;
const SAVE_GAME_WARNING: u32 = 0x0084_5cc0;
/// `AI: Couldn't find a plugin package while loading and will try to choose a new one.`,
/// `AI: Couldn't find package %08X while loading ...` and `Package '%s' (%08X) changed type from
/// %s to %s, so the actor package data was replaced`.
const FORMAT_NO_PLUGIN_PACKAGE: u32 = 0x0108_5458;
const FORMAT_NO_PACKAGE: u32 = 0x0108_54b0;
const FORMAT_TYPE_CHANGED: u32 = 0x0108_5500;

#[inline]
fn word(e: &Engine, this: Ptr, offset: u32) -> u32 {
    e.mem.u32(this.addr().wrapping_add(offset))
}

#[inline]
fn set_word(e: &mut Engine, this: Ptr, offset: u32, value: u32) {
    e.mem.set_u32(this.addr().wrapping_add(offset), value);
}

#[inline]
fn field(this: Ptr, offset: u32) -> u32 {
    this.addr().wrapping_add(offset)
}

/// The float at `01012054` as `FLD`/`FSTP` move it (a signalling NaN quieted).
#[inline]
fn empty_start_time(e: &Engine) -> u32 {
    x87_quiet_bits(e.mem.u32(MINUS_ONE_FLOAT))
}

/// A package's procedure type (`0041ca90`, the AI queries family's).
#[inline]
fn package_type(e: &mut Engine, package: u32) -> i32 {
    typed_call!(
        ai_queries::FAMILY,
        0x0041_ca90,
        ai_queries::queries::package_type(e, package)
    )
}

/// A package's form id (`0084e3a0`).
#[inline]
fn form_id(e: &mut Engine, package: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, package))
}

/// `TESDataHandler`'s created-form test (`00469860`): an id of `0xff000000` and up.
#[inline]
fn is_created_id(id: u32) -> bool {
    id >= 0xff00_0000
}

/// `Procedure::GetProcedureCount` (`0096a820`) of a package's procedure (`009611e0`): the signed
/// byte of the table for 0 to 0x30, -1 otherwise.
fn procedure_count(e: &Engine, package: u32) -> i32 {
    let procedure = e.mem.u32(package.wrapping_add(PACKAGE_PROCEDURE));
    if procedure < 0x31 {
        i32::from(e.mem.i8(PROCEDURE_COUNTS + procedure))
    } else {
        -1
    }
}

/// The procedure index lowered below the package's procedure count when it has reached it (signed).
fn clamp_procedure_index(e: &mut Engine, this: Ptr) {
    let package = word(e, this, PACKAGE);
    let count = procedure_count(e, package);
    let index = word(e, this, PROCEDURE_INDEX) as i32;
    if index >= count {
        set_word(e, this, PROCEDURE_INDEX, count.wrapping_sub(1) as u32);
    }
}

/// The form for `id` cast to `target` (`004839c0`, then `__RTDynamicCast` from `TESForm`).
fn cast_form(e: &mut Engine, id: u32, target: u32) -> u32 {
    let form = e.call(FORM_FOR_ID, &args![id]).u32();
    e.call(
        RT_DYNAMIC_CAST,
        &args![form, 0u32, TYPE_TES_FORM, target, 0u32],
    )
    .u32()
}

/// [`cast_form`], or null for id 0.
fn cast_id(e: &mut Engine, id: u32, target: u32) -> u32 {
    if id == 0 {
        0
    } else {
        cast_form(e, id, target)
    }
}

/// An object deleted through its slot `slot` (the deleting destructor, with 1), when there is one.
fn delete(e: &mut Engine, object: u32, slot: u32) {
    if object != 0 {
        e.vcall(object, slot, &args![1u32]);
    }
}

/// A package's name (its slot 0x130): the base process's empty name when the slot holds it.
fn package_name(e: &mut Engine, package: u32) -> u32 {
    if slot_target(e, package, 0x130) == 0x0040_1280 {
        typed_call!(
            FAMILY,
            0x0040_1280,
            super::base_process::empty_name(e, Ptr::new(package))
        )
    } else {
        e.vcall(package, 0x130, &[]).u32()
    }
}

fn save_load_game(e: &Engine) -> u32 {
    e.mem.u32(SAVE_LOAD_GAME)
}

// Typed from 008c5e70
/// The data deleted (its slot 0 with 1) when there is one; the word is left as it is. The
/// destructor body (no Xbox PDB name).
pub fn destroy_data(e: &mut Engine, this: Ptr) {
    let data = word(e, this, DATA);
    delete(e, data, 0);
}

// Typed from 008c5ec0
/// `GuardActorPackageData::HandleDeletedReference` (Xbox PDB; the body is shared): the word at +8
/// cleared when it is `deleted`.
pub fn handle_deleted_reference(e: &mut Engine, this: Ptr, deleted: u32) {
    if deleted == word(e, this, TARGET) {
        set_word(e, this, TARGET, 0);
    }
}

// Typed from 008c5ef0
/// `ActorPackage::Clear` (engine map name). The package, when there is one: the player's package
/// being run (+0x208) forgotten when it is this one; its count lowered (`008c6030`), and when that
/// leaves 0 and its flag 0x800 is set, the word cleared and the package deleted (its slot 0x10
/// with 1); otherwise the word cleared. The data deleted (its slot 0 with 1) and cleared. Then the
/// empty values: the start time the float at `01012054`, the procedure index -1, the flags and the
/// target 0.
pub fn clear(e: &mut Engine, this: Ptr) {
    let package = word(e, this, PACKAGE);
    if package != 0 {
        let player = e.mem.u32(PLAYER_CHARACTER);
        let running = player.wrapping_add(PLAYER_PACKAGE);
        if package == e.mem.u32(running) {
            e.mem.set_u32(running, 0);
        }
        let package = word(e, this, PACKAGE);
        let remaining = typed_call!(FAMILY, 0x008c_6030, dec_ref_count(e, Ptr::new(package)));
        let package = word(e, this, PACKAGE);
        let deleted = remaining == 0 && e.mem.u32(package.wrapping_add(PACKAGE_FLAGS)) & 0x800 != 0;
        set_word(e, this, PACKAGE, 0);
        if deleted {
            e.vcall(package, 0x10, &args![1u32]);
        }
    }
    let data = word(e, this, DATA);
    if data != 0 {
        e.vcall(data, 0, &args![1u32]);
        set_word(e, this, DATA, 0);
    }
    let start = empty_start_time(e);
    set_word(e, this, START_TIME, start);
    set_word(e, this, PROCEDURE_INDEX, u32::MAX);
    set_word(e, this, FLAGS, 0);
    set_word(e, this, TARGET, 0);
}

// Typed from 008c6030
/// `TESPackage::DecRefCount` (Xbox PDB): the count (+0x7c) down (`004019a0`, the dispatch
/// family's); the new count.
pub fn dec_ref_count(e: &mut Engine, this: Ptr) -> u32 {
    let count = typed_call!(
        dispatch::FAMILY,
        0x0040_19a0,
        dispatch::refs::decrement(e, this.byte_add(PACKAGE_REF_COUNT))
    );
    count as u32
}

// Typed from 008c6060
/// The word at +4 cleared (`ActorPackage::NullInstanceData` in the engine map; the body is shared by
/// several constructors' helpers).
pub fn null_instance_data(e: &mut Engine, this: Ptr) {
    set_word(e, this, DATA, 0);
}

// Typed from 008c6080
/// `ActorPackage::InitFromPackage` (Xbox PDB): nothing when `package` is already this one's.
/// Otherwise the record cleared, and for a package: a reference taken (`008c6100`), the
/// calendar's hour (`00867da0`) stored as the start time (`FSTP float`), the package, new data
/// for its type (`008c6ca0`), procedure index 0 and `target`.
pub fn init_from_package(e: &mut Engine, this: Ptr, package: Ptr, target: u32) {
    if package.addr() == word(e, this, PACKAGE) {
        return;
    }
    typed_call!(FAMILY, 0x008c_5ef0, clear(e, this));
    if package.is_null() {
        return;
    }
    typed_call!(FAMILY, 0x008c_6100, inc_ref_count(e, package));
    let hour = e.call(CALENDAR_GET_HOUR, &args![CALENDAR]).f64();
    set_word(e, this, START_TIME, x87_store32(hour));
    set_word(e, this, PACKAGE, package.addr());
    let kind = package_type(e, package.addr());
    let data = e.call(CREATE_ACTOR_PACKAGE_DATA, &args![kind]).u32();
    set_word(e, this, DATA, data);
    set_word(e, this, PROCEDURE_INDEX, 0);
    set_word(e, this, TARGET, target);
}

// Typed from 008c6100
/// `TESPackage::IncRefCount` (Xbox PDB): the count (+0x7c) up (`0040b460`, the dispatch family's);
/// the new count.
pub fn inc_ref_count(e: &mut Engine, this: Ptr) -> u32 {
    let count = typed_call!(
        dispatch::FAMILY,
        0x0040_b460,
        dispatch::refs::increment(e, this.byte_add(PACKAGE_REF_COUNT))
    );
    count as u32
}

// Typed from 008c6130
/// `ActorPackage::MoveTo` (Xbox PDB): the package, the data, the procedure index and the start
/// time (`FLD`/`FSTP`, a signalling NaN quieted) copied to `destination`; this one's package and
/// data cleared, its procedure index -1 and its start time the float at `01012054`. The target
/// and the flags are not touched.
pub fn move_to(e: &mut Engine, this: Ptr, destination: Ptr) {
    for offset in [PACKAGE, DATA, PROCEDURE_INDEX] {
        let value = word(e, this, offset);
        set_word(e, destination, offset, value);
    }
    let start = x87_quiet_bits(word(e, this, START_TIME));
    set_word(e, destination, START_TIME, start);
    set_word(e, this, PACKAGE, 0);
    set_word(e, this, DATA, 0);
    set_word(e, this, PROCEDURE_INDEX, u32::MAX);
    let empty = empty_start_time(e);
    set_word(e, this, START_TIME, empty);
}

// Typed from 008c61a0
/// The size of what [`save`] writes: 8, or 12 with a package (its procedure index). The word is
/// unread (the exe's only use of it is the folded block).
pub fn save_size(e: &mut Engine, this: Ptr, _word: u32) -> u32 {
    if word(e, this, PACKAGE) != 0 {
        12
    } else {
        8
    }
}

// Typed from 008c6250
/// The record saved through the save/load game: the package's form id (0 without one) as a numeric
/// id (`00857a10`, 4 bytes), with an id the procedure index (4 bytes, `008579b0`), and the start
/// time (4 bytes). The word is unread.
pub fn save(e: &mut Engine, this: Ptr, _word: u32) {
    e.with_stack(8, |e, locals| {
        let id_at = locals.addr();
        let package = word(e, this, PACKAGE);
        let id = if package != 0 { form_id(e, package) } else { 0 };
        e.mem.set_u32(id_at, id);
        let save_load = save_load_game(e);
        e.call(SAVE_NUMERIC_ID, &args![save_load, id_at, 4u32]);
        if e.mem.u32(id_at) != 0 {
            let save_load = save_load_game(e);
            e.call(
                WRITE_BYTES,
                &args![save_load, field(this, PROCEDURE_INDEX), 4u32],
            );
        }
    });
    let save_load = save_load_game(e);
    e.call(
        WRITE_BYTES,
        &args![save_load, field(this, START_TIME), 4u32],
    );
}

/// The procedure index stored (`0041fd00`'s body).
#[inline]
fn set_procedure_index(e: &mut Engine, this: Ptr, index: u32) {
    set_word(e, this, PROCEDURE_INDEX, index);
}

// Typed from 008c6320
/// `ActorPackage::LoadGame` (Xbox PDB): the numeric id read (`00857aa0`, its byte answer kept) and
/// the record cleared. With an id: the package taken from the id's form as a `TESPackage`
/// ([`init_from_package`]), which works out its procedure type for the target (`006777b0`), or,
/// with none, a failed load; then the procedure index read. Without an id but with the answer
/// set: the position moved on by 4 (the save/load game's +0x14), the procedure index 0 and a
/// failed load. A failed load is logged (`005b5e40`, with the id when there is one) and sets the
/// procedure index to -1. The start time is read last. The words are unread.
pub fn load_game(e: &mut Engine, this: Ptr, _word: u32, _unused: u32) {
    e.with_stack(8, |e, locals| {
        let id_at = locals.addr();
        let save_load = save_load_game(e);
        let answer = e.call(LOAD_NUMERIC_ID, &args![save_load, id_at, 4u32]).u8();
        typed_call!(FAMILY, 0x008c_5ef0, clear(e, this));
        let id = e.mem.u32(id_at);
        let failed = if id != 0 {
            let package = cast_form(e, id, TYPE_TES_PACKAGE);
            typed_call!(
                FAMILY,
                0x008c_6080,
                init_from_package(e, this, Ptr::new(package), 0)
            );
            let package = word(e, this, PACKAGE);
            let failed = package == 0;
            if !failed {
                let target = word(e, this, TARGET);
                e.call(CALCULATE_PROCEDURE_TYPE, &args![package, target]);
            }
            let save_load = save_load_game(e);
            e.call(
                READ_BYTES,
                &args![save_load, field(this, PROCEDURE_INDEX), 4u32],
            );
            failed
        } else if answer != 0 {
            let position = save_load_game(e).wrapping_add(BUFFER_POSITION);
            let moved = e.mem.u32(position).wrapping_add(4);
            e.mem.set_u32(position, moved);
            set_procedure_index(e, this, 0);
            true
        } else {
            false
        };
        if failed {
            if id != 0 {
                e.call(AI_LOG, &args![FORMAT_NO_PACKAGE, id]);
            } else {
                e.call(AI_LOG, &args![FORMAT_NO_PLUGIN_PACKAGE]);
            }
            set_procedure_index(e, this, u32::MAX);
        }
    });
    let save_load = save_load_game(e);
    e.call(READ_BYTES, &args![save_load, field(this, START_TIME), 4u32]);
}

// Typed from 008c64d0
/// `ActorPackage::InitLoadGame` (Xbox PDB): with a package, the procedure index lowered to the
/// package's procedure count less one when it has reached the count (signed). The words are
/// unread.
pub fn init_load_game(e: &mut Engine, this: Ptr, _word: u32, _unused: u32) {
    if word(e, this, PACKAGE) != 0 {
        clamp_procedure_index(e, this);
    }
}

// Typed from 008c65a0
/// Nothing: the exe's only code is the folded block. The word is unread.
pub fn revert_unused(_e: &mut Engine, _this: Ptr, _word: u32) {}

// Typed from 008c65d0
/// `ActorPackage::SaveGame` (Xbox PDB): the buffer's version asked (its slot 8), the package's form
/// id saved (`00865df0`). With a package: when its id is a created form's, its type byte (0xff when
/// its slot 0x15c refuses the version) written and, unless 0xff, the package's own save (its slot
/// 0x54); the data's type byte (its slot 8; 0xff without data) written and, unless 0xff, the data's
/// own save (its slot 0x0c); the procedure index, the start time and the flags (4 bytes each,
/// `00865e50`), and the target's form id.
pub fn save_game(e: &mut Engine, this: Ptr, buffer: Ptr) {
    let version = e.vcall(buffer.addr(), 8, &[]).u32();
    let package = word(e, this, PACKAGE);
    e.call(SAVE_FORM_ID, &args![buffer, package, 0u32]);
    let package = word(e, this, PACKAGE);
    if package == 0 {
        return;
    }
    e.with_stack(2, |e, bytes| {
        let package_type_at = bytes.addr();
        let data_type_at = bytes.addr() + 1;
        if is_created_id(form_id(e, package)) {
            let package = word(e, this, PACKAGE);
            #[allow(clippy::cast_possible_truncation)] // The type's low byte, as the exe stores AL.
            let kind = package_type(e, package) as u8;
            e.mem.set_u8(package_type_at, kind);
            let package = word(e, this, PACKAGE);
            if !e.vcall(package, 0x15c, &args![version]).bool() {
                e.mem.set_u8(package_type_at, 0xff);
            }
            e.call(BUFFER_WRITE, &args![buffer, package_type_at, 1u32, 0u32]);
            if e.mem.u8(package_type_at) != 0xff {
                let package = word(e, this, PACKAGE);
                e.vcall(package, 0x54, &args![buffer]);
            }
        }
        let data = word(e, this, DATA);
        let kind = if data != 0 {
            e.vcall(data, 8, &[]).u8()
        } else {
            0xff
        };
        e.mem.set_u8(data_type_at, kind);
        e.call(BUFFER_WRITE, &args![buffer, data_type_at, 1u32, 0u32]);
        if e.mem.u8(data_type_at) != 0xff {
            let data = word(e, this, DATA);
            e.vcall(data, 0xc, &args![buffer]);
        }
    });
    for offset in [PROCEDURE_INDEX, START_TIME, FLAGS] {
        e.call(
            BUFFER_WRITE,
            &args![buffer, field(this, offset), 4u32, 0u32],
        );
    }
    let target = word(e, this, TARGET);
    e.call(SAVE_FORM_ID, &args![buffer, target, 0u32]);
}

// Typed from 008c6730
/// `ActorPackage::LoadGame_ov2` (Xbox PDB): the package's form id read (`008648e0`, its byte
/// answer kept); nothing more with neither. For a created form's id a type byte follows: 0xff
/// keeps the id as the package, to be marked; another type makes a package (`00670b90`), records
/// the id's change (`0084aa30` with the new id) and loads it (its slot 0x5c). Any other id is
/// the id's form as a `TESPackage` (null for 0). Then the data's type byte: 0xff deletes the data
/// (its slot 0 with 1) and clears it, another makes new data (`008c6ca0`) that loads itself (its
/// slot 0x10). Then the procedure index, the start time and the flags (4 bytes each) and the
/// target's form id. A marked package sets flag 0 ([`set_flag`]); the answer set clears the record
/// at the end.
pub fn load_game_buffer(e: &mut Engine, this: Ptr, buffer: Ptr) {
    e.with_stack(8, |e, locals| {
        let id_at = locals.addr();
        let kind_at = locals.addr() + 4;
        let data_kind_at = locals.addr() + 5;
        e.mem.set_u32(id_at, 0);
        let answer = e.call(LOAD_FORM_ID, &args![buffer, id_at]).u8();
        let id = e.mem.u32(id_at);
        if id == 0 && answer == 0 {
            return;
        }
        let mut marked = false;
        if is_created_id(id) {
            e.mem.set_u8(kind_at, 0);
            e.call(BUFFER_READ, &args![buffer, kind_at, 1u32]);
            let kind = e.mem.i8(kind_at);
            if kind != -1 {
                let package = e.call(CREATE_PACKAGE, &args![i32::from(kind)]).u32();
                set_word(e, this, PACKAGE, package);
                let new_id = form_id(e, package);
                let changed = e.mem.u32(CHANGED_FORMS);
                let id = e.mem.u32(id_at);
                e.call(ADD_CHANGED_FORM_ID, &args![changed, id, new_id]);
                let package = word(e, this, PACKAGE);
                e.vcall(package, 0x5c, &args![buffer]);
            } else {
                let id = e.mem.u32(id_at);
                set_word(e, this, PACKAGE, id);
                marked = true;
            }
        } else {
            let package = cast_id(e, id, TYPE_TES_PACKAGE);
            set_word(e, this, PACKAGE, package);
        }
        e.mem.set_u8(data_kind_at, 0xff);
        e.call(BUFFER_READ, &args![buffer, data_kind_at, 1u32]);
        let data_kind = e.mem.i8(data_kind_at);
        if data_kind != -1 {
            let data = e
                .call(CREATE_ACTOR_PACKAGE_DATA, &args![i32::from(data_kind)])
                .u32();
            set_word(e, this, DATA, data);
            e.vcall(data, 0x10, &args![buffer]);
        } else {
            let data = word(e, this, DATA);
            delete(e, data, 0);
            set_word(e, this, DATA, 0);
        }
        for offset in [PROCEDURE_INDEX, START_TIME, FLAGS] {
            e.call(BUFFER_READ, &args![buffer, field(this, offset), 4u32]);
        }
        e.call(LOAD_FORM_ID, &args![buffer, field(this, TARGET)]);
        if marked {
            typed_call!(FAMILY, 0x008c_6920, set_flag(e, this, 0, 1));
        }
        if answer != 0 {
            typed_call!(FAMILY, 0x008c_5ef0, clear(e, this));
        }
    });
}

// Typed from 008c6920
/// The flag `bit` (its low 5 bits, as `SHL` counts) of the word at +0x14 set (`set` not 0) or
/// cleared.
pub fn set_flag(e: &mut Engine, this: Ptr, bit: u32, set: u8) {
    let mask = 1u32 << (bit & 0x1f);
    let flags = word(e, this, FLAGS);
    let flags = if set != 0 {
        flags | mask
    } else {
        flags & !mask
    };
    set_word(e, this, FLAGS, flags);
}

// Typed from 008c6970
/// `ActorPackage::InitLoadGame_ov2` (Xbox PDB): a package still marked an id (flag 0) looked up
/// again (`0084aa90`, then as a `TESPackage`; null for 0); an unmarked one with a created form's
/// id has its own `InitLoadGame` (its slot 0x64). With a package: its procedure type worked out
/// when it has none (-1; `006777b0` with 0), a reference taken (`008c6100`), the procedure index
/// lowered below its procedure count. The data's slot 0x14; the target (+8) resolved to a
/// `TESObjectREFR` (null for 0) and, when there is one, marked as targeted (`00564db0` with 1).
pub fn init_load_game_buffer(e: &mut Engine, this: Ptr, buffer: Ptr) {
    if word(e, this, FLAGS) & 1 != 0 {
        let changed = e.mem.u32(CHANGED_FORMS);
        let package = word(e, this, PACKAGE);
        let id = e.call(CHANGED_FORM_LOOKUP, &args![changed, package]).u32();
        let package = cast_id(e, id, TYPE_TES_PACKAGE);
        set_word(e, this, PACKAGE, package);
    } else {
        let package = word(e, this, PACKAGE);
        if package != 0 && is_created_id(form_id(e, package)) {
            e.vcall(package, 0x64, &args![buffer]);
        }
    }
    let package = word(e, this, PACKAGE);
    if package != 0 {
        if e.mem.u32(package.wrapping_add(PACKAGE_PROCEDURE)) == u32::MAX {
            e.call(CALCULATE_PROCEDURE_TYPE, &args![package, 0u32]);
        }
        let package = word(e, this, PACKAGE);
        typed_call!(FAMILY, 0x008c_6100, inc_ref_count(e, Ptr::new(package)));
        clamp_procedure_index(e, this);
    }
    let data = word(e, this, DATA);
    if data != 0 {
        e.vcall(data, 0x14, &args![buffer]);
    }
    let target = word(e, this, TARGET);
    let target = cast_id(e, target, TYPE_TES_OBJECT_REFR);
    set_word(e, this, TARGET, target);
    if target != 0 {
        e.call(SET_TARGETED, &args![target, 1u32]);
    }
}

// Typed from 008c6b00
/// Whether the flag `bit` (its low 5 bits) of the word at +0x14 is set, in `AL` (`SETNZ`); the
/// upper bytes of `EAX` keep the masked word's.
pub fn flag(e: &mut Engine, this: Ptr, bit: u32) -> u32 {
    let masked = (1u32 << (bit & 0x1f)) & word(e, this, FLAGS);
    (masked & 0xffff_ff00) | u32::from(masked != 0)
}

// Typed from 008c6b20
/// `ActorPackage::FinishLoadGame` (Xbox PDB): a package still marked an id has the mark cleared;
/// an unmarked one with a created form's id has its own `FinishLoadGame` (its slot 0x84). With
/// data and a package whose type differs from the data's (its slot 8), a warning (`00845cc0`,
/// with the package's name, its form id and the data's and the package's type names), the data
/// deleted (its slot 0 with 1) and replaced with new data for the package's type (`008c6ca0`).
pub fn finish_load_game(e: &mut Engine, this: Ptr, buffer: Ptr) {
    let flags = word(e, this, FLAGS);
    if flags & 1 != 0 {
        set_word(e, this, FLAGS, flags & !1);
    } else {
        let package = word(e, this, PACKAGE);
        if package != 0 && is_created_id(form_id(e, package)) {
            e.vcall(package, 0x84, &args![buffer]);
        }
    }
    if word(e, this, DATA) == 0 {
        return;
    }
    let package = word(e, this, PACKAGE);
    if package == 0 {
        return;
    }
    let package_kind = package_type(e, package);
    let data = word(e, this, DATA);
    let data_kind = e.vcall(data, 8, &[]).u32() as i32;
    if package_kind == data_kind {
        return;
    }
    let package = word(e, this, PACKAGE);
    let kind = package_type(e, package);
    let new_name = type_name(e, kind);
    let data = word(e, this, DATA);
    let old_kind = e.vcall(data, 8, &[]).u32() as i32;
    let old_name = type_name(e, old_kind);
    let package = word(e, this, PACKAGE);
    let id = form_id(e, package);
    let package = word(e, this, PACKAGE);
    let name = package_name(e, package);
    e.call(
        SAVE_GAME_WARNING,
        &args![FORMAT_TYPE_CHANGED, name, id, old_name, new_name],
    );
    let data = word(e, this, DATA);
    delete(e, data, 0);
    let package = word(e, this, PACKAGE);
    let kind = package_type(e, package);
    let data = e.call(CREATE_ACTOR_PACKAGE_DATA, &args![kind]).u32();
    set_word(e, this, DATA, data);
}

/// The name of package type `kind` (`[kind * 4 + 0119bcb0]`).
fn type_name(e: &Engine, kind: i32) -> u32 {
    let index = kind as u32;
    e.mem
        .u32(PACKAGE_TYPE_NAMES.wrapping_add(index.wrapping_mul(4)))
}

// Typed from 008c6c80
/// `ActorPackage::Revert` (Xbox PDB): [`clear`]. The word is unread.
pub fn revert(e: &mut Engine, this: Ptr, _word: u32) {
    typed_call!(FAMILY, 0x008c_5ef0, clear(e, this));
}
