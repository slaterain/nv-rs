//! `TESPackage`'s first functions (Xbox PDB; the unit `fallout shared/package`, `006707c0` to
//! `00674e70`, and the type 9 data's constructor `0067acf0`): the constructor, the reset of the
//! package data, `CreatePackage`, `SetPackType`, the flag tests and the specific flags' setters,
//! the location, second location and target setters, the dialogue data's getters and setters,
//! `GetAcquireRadius`, `GetInitialTargetCount`, the use weapon data, the second and search
//! locations, `InitItem`, `GetIsCreated` and `SetIsCreated`, the script and never-to-run flags and
//! `SetNeverRun`; and the unit's other short functions: the flag 0x10000's setter (`00674f30`),
//! the type data getters (`006758f0` to `006759b0`), `GetEscortFollowDistance`,
//! `IsInterruptPackage`, `GetShouldReserveTarget`, `GetObjectTypeFromForm` and
//! `FormMatchesPackageObjectType` (the type 0x28 form's two tests `006450c0` and `004c0c30`, the
//! signed byte at +0xf4 from 0 to 2 and from 3 to 13, in place; the effect list's three tests and
//! a form's `+0x3c` member's flag slot are calls). The place functions are in
//! [`super::package_place`], the save game functions in [`super::package_save`].
//!
//! A package (0x80 bytes) has its procedure type at +0x18, its flags at +0x1c (a word), its type
//! at +0x20 (a signed byte, 0xff when none), its behaviour flags at +0x22 and specific flags at
//! +0x24 (half words), its type's data object at +0x28, its location (`PackageLocation`, 0xc
//! bytes) at +0x2c, its target (`PackageTarget`, 0x10 bytes) at +0x30, its idle collection at
//! +0x34, its schedule at +0x38, its conditions at +0x40, its combat style at +0x48, its three
//! event actions at +0x4c, +0x5c and +0x6c and a reference count at +0x7c. The dialogue types
//! (0xf, 0x1c) keep a 0x20-byte data object (`fFov` +4, the topic +8, the second location
//! +0xc, the bytes +0x10, +0x11 and +0x18), the use weapon type (0x10) a 0x24-byte one (the second
//! location +8), the types 1, 2, 3, 8 and 9 small ones with the second location at +4.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): the package's type (`0041ca90`, the sign-extended byte at +0x20), location
//! (`0055b980`, +0x2c), target (`00671d10`, +0x30) and procedure type (`009611e0`, +0x18), the
//! form's id (`0084e3a0`, +0xc), the created-id test (`00469860`: an id from 0xff00_0000 up), a
//! target's type (`00519b00`, its first byte) and value
//! (`0044ddc0`, +8) and constructor (`0067ff70`: the byte 2, two zero words, 0.0), a location's
//! type (`00678ca0`, its first byte, sign-extended) and constructor (`0067f030`: the byte 3, two
//! zero words) and reference (`00680020`: a target of type 0 holds it at +4), the form's flag 8
//! test (`004013e0`), the conditions' address (`0087eaa0`, +0x40), the idle collection
//! (`005f36f0`, +0x34), the combat style's getter and setter (`006733e0`, `00673400`, +0x48), an
//! event action's kind setter (`0041fd00`, its +0xc), `TESPackageData`'s constructor (`0067ad20`,
//! its vtable, which the type 9 data's constructor stores over), and this unit's own flag tests
//! and setters. Where the exe asks a getter twice with nothing between, it is read once. The
//! specific flags' setters `SetPackType` calls for type 0xc only clear bits of the word it has
//! just set to zero, so they are left out. The other functions of the family are typed calls:
//! `TESForm`'s constructor, the schedule's, the conditions' and the event actions'
//! constructors, the package data constructors, `TESForm`'s file, compile index, form map lookup
//! and flag 8 setter, the target's copy, type setter and `InitItem` (`package_target`), and this
//! unit's own functions that do more than one line. What stays a call by address: the allocator,
//! the subclasses' constructors, the location's and target's deleting destructors, the location's
//! copy, the use weapon data's constructor, the form type's setter (`004f15a0`, which JIP LN may
//! replace), the schedule's setter (`00673180`),
//! the assert, the count `GetAcquireRadius` takes for type 0xc (`00676280`), `InitItem`'s loader
//! warning count, its location's, conditions', idle collection's and event actions'
//! `InitItem`s, the RTTI cast, the warning logger (which a fix may replace), the location's type
//! setter and the type 0xc test `00673890`; the data object's deleting destructor and `InitItem`
//! and the form's name and change slots are called through their slots (each class has its own).
//! JIP LN's H16 seam in `CreatePackage` (`00670c30`, the combat controller's size while JIP LN
//! is installed) runs where it does.
//!
//! Floats: the dialogue data's `fFov` is moved through the exe's `FLD`/`FSTP` (a signalling NaN
//! quieted; 1.0 when there is none); the acquire radius is the target's float as `FLD` loads it,
//! compared with the exe's 0.0 as the x87 compare does (an unordered pair passes) and stored as
//! the `f32` the exe stores, or for type 0xc the unsigned count converted exactly and stored
//! rounded to an `f32`.

use super::package_flags::{pack_flag, set_pack_flag, PACK_FLAGS, PACK_TYPE};
use super::{conditions, package_data, package_place, package_target, schedule, tesform, FAMILY};
use engine_core::x87::{x87_load32, x87_quiet_bits, x87_store32};
use engine_core::{args, typed_call, Engine};

/// The package's fields.
const FORM_ID: u32 = 0xc;
const PROCEDURE: u32 = 0x18;
const BEHAVIOR_FLAGS: u32 = 0x22;
const SPECIFIC_FLAGS: u32 = 0x24;
const DATA: u32 = 0x28;
const LOCATION: u32 = 0x2c;
const TARGET: u32 = 0x30;
const IDLE_COLLECTION: u32 = 0x34;
const SCHEDULE: u32 = 0x38;
const CONDITIONS: u32 = 0x40;
const COMBAT_STYLE: u32 = 0x48;
const EVENT_ACTIONS: [u32; 3] = [0x4c, 0x5c, 0x6c];
const REF_COUNT: u32 = 0x7c;
/// The vtable the constructor stores, the form type it sets and the form type's setter (`ECX` = the
/// form: the type; JIP LN may replace it, so it is called by address).
const PACKAGE_VTABLE: u32 = 0x0106_847c;
const PACKAGE_FORM_TYPE: u8 = 0x49;
const SET_FORM_TYPE: u32 = 0x004f_15a0;
/// The flags' bits: once a day, created.
const FLAG_ONCE_PER_DAY: u32 = 0x400;
const FLAG_CREATED: u32 = 0x800;
const FLAG_800000: u32 = 0x0080_0000;
/// The package types the code tells apart.
const TYPE_DIALOGUE: i32 = 0xf;
const TYPE_DIALOGUE_SECOND: i32 = 0x1c;
const TYPE_USE_WEAPON: i32 = 0x10;
/// `operator new`, the assert (file, line) and the package unit's file name.
const OPERATOR_NEW: u32 = 0x0040_1000;
const ASSERT: u32 = 0x00aa_fb90;
const PACKAGE_SOURCE_FILE: u32 = 0x0106_85dc;
/// The schedule's setter (`ECX` = the package: a schedule or null).
const SET_SCHEDULE: u32 = 0x0067_3180;
/// `PackageLocation`: its size, deleting destructor (flags) and copy (source, 0).
const LOCATION_SIZE: u32 = 0xc;
const LOCATION_DELETE: u32 = 0x0067_0b30;
const LOCATION_COPY: u32 = 0x0067_f4d0;
/// The location type for which the search location is the package's own location.
const LOCATION_TYPE_SEVEN: i32 = 7;
/// `PackageTarget`: its size and deleting destructor (flags).
const TARGET_SIZE: u32 = 0x10;
const TARGET_DELETE: u32 = 0x007b_3fa0;
/// The data objects: the dialogue data's size and offsets, the use weapon data's size,
/// constructor (by address) and second location, the small data objects' second location.
const DIALOGUE_DATA_SIZE: u32 = 0x20;
const DIALOGUE_FOV: u32 = 4;
const DIALOGUE_TOPIC: u32 = 8;
const DIALOGUE_SECOND_LOCATION: u32 = 0xc;
const DIALOGUE_NO_HEADTRACKING: u32 = 0x10;
const DIALOGUE_DO_NOT_CONTROL_TARGET: u32 = 0x11;
const DIALOGUE_SAY_TO: u32 = 0x18;
const USE_WEAPON_DATA_SIZE: u32 = 0x24;
const USE_WEAPON_DATA_CONSTRUCT: u32 = 0x0067_caa0;
const USE_WEAPON_SECOND_LOCATION: u32 = 8;
const SMALL_SECOND_LOCATION: u32 = 4;
/// The type 9 data's vtable.
const TYPE_9_DATA_VTABLE: u32 = 0x0106_8860;
/// The data object's deleting destructor slot.
const SLOT_DELETE: u32 = 0;
/// The data handler's ids from here up are created ones.
const CREATED_IDS: u32 = 0xff00_0000;
/// The exe's 0.0 the acquire radius is compared with.
const ZERO: f64 = 0.0;
/// `InitItem`'s callees: the loader's warning count (a thread's field, read by `0046e8a0`), the
/// location's, target's, conditions' and idle collection's `InitItem` (each `ECX` = the object: the
/// package), the data object's `InitItem` slot, the RTTI cast and the types it casts between, the
/// warning logger (which a fix may replace), its two messages, the form's name slot, the event
/// action's `InitItem` (the package), the location's type setter, the type 0xc test `00673890`.
const WARNING_COUNT: u32 = 0x0046_e8a0;
const LOCATION_INIT_ITEM: u32 = 0x0067_f760;
const CONDITIONS_INIT_ITEM: u32 = 0x0068_0a20;
const IDLE_COLLECTION_INIT_ITEM: u32 = 0x0047_9b00;
const SLOT_DATA_INIT_ITEM: u32 = 0x10;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_COMBAT_STYLE: u32 = 0x0118_627c;
const WARNING: u32 = 0x005b_5e40;
const TEXT_NO_COMBAT_STYLE: u32 = 0x0106_8688;
const TEXT_WARNINGS_GENERATED: u32 = 0x0106_8620;
const SLOT_FORM_NAME: u32 = 0x130;
const EVENT_ACTION_INIT_ITEM: u32 = 0x0067_e190;
const LOCATION_SET_TYPE: u32 = 0x0067_f140;
const TYPE_C_DECIDES_LOCATION: u32 = 0x0067_3890;
/// The location type `InitItem` gives the types that decide their location as they run.
const LOCATION_TYPE_SIX: u32 = 6;
/// The form's flags (+8) and their bit 8 (initialised); the flags' never-to-run and script bits;
/// the form's slots told of a flag change (set, cleared) and the mask `SetNeverRun` tells them.
const FORM_FLAGS: u32 = 8;
const FORM_FLAG_INITIALISED: u32 = 8;
const FLAG_NEVER_TO_RUN: u32 = 0x8000;
const FLAG_SCRIPT_PACKAGE: u32 = 0x4000;
const SLOT_FORM_SET_CHANGED: u32 = 0x48;
const SLOT_FORM_CLEAR_CHANGED: u32 = 0x4c;
const NEVER_RUN_CHANGE: u32 = 0x8000_0000;
/// The flag 0x10000 and the mask its setter tells the form.
const FLAG_10000: u32 = 0x1_0000;
const FLAG_10000_CHANGE: u32 = 0x4000_0000;
/// A form's type, its `+0x3c` member and that member's flag slot, the type 0x28 form's kind byte, the
/// effect list's three tests (`ECX` = the list at +0x24).
const FORM_TYPE: u32 = 4;
const FORM_MEMBER_3C: u32 = 0x3c;
const SLOT_MEMBER_FLAG: u32 = 4;
const FORM_28_KIND: u32 = 0xf4;
const EFFECT_LIST_TARGET: u32 = 0x0040_5f30;
const EFFECT_LIST_TOUCH: u32 = 0x0040_5fe0;
const EFFECT_LIST_THIRD: u32 = 0x0040_6090;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

/// The package type (`0041ca90`).
fn pack_type(e: &Engine, this: u32) -> i32 {
    i32::from(e.mem.i8(at(this, PACK_TYPE)))
}

fn is_dialogue_type(e: &Engine, this: u32) -> bool {
    matches!(pack_type(e, this), TYPE_DIALOGUE | TYPE_DIALOGUE_SECOND)
}

/// `new` of a class the exe builds by address: `size` bytes, then `constructor` with `extra` when
/// the allocation gave a block. Null when it didn't.
fn allocate_and_construct(e: &mut Engine, size: u32, constructor: u32, extra: &[u32]) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        return 0;
    }
    let mut words = vec![block];
    words.extend_from_slice(extra);
    e.call(constructor, &words).u32()
}

/// A new location (`0067f030` in place: the byte 3 and two zero words).
fn new_location(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![LOCATION_SIZE]).u32();
    if block != 0 {
        e.mem.set_u8(block, 3);
        e.mem.set_u32(at(block, 4), 0);
        e.mem.set_u32(at(block, 8), 0);
    }
    block
}

/// A new target (`0067ff70` in place: the byte 2, two zero words and 0.0).
fn new_target(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![TARGET_SIZE]).u32();
    if block != 0 {
        e.mem.set_u8(block, 2);
        e.mem.set_u32(at(block, 4), 0);
        e.mem.set_u32(at(block, 8), 0);
        e.mem.set_u32(at(block, 0xc), 0);
    }
    block
}

fn ensure_location(e: &mut Engine, this: u32) {
    if e.mem.u32(at(this, LOCATION)) == 0 {
        let made = new_location(e);
        e.mem.set_u32(at(this, LOCATION), made);
    }
}

fn ensure_target(e: &mut Engine, this: u32) {
    if e.mem.u32(at(this, TARGET)) == 0 {
        let made = new_target(e);
        e.mem.set_u32(at(this, TARGET), made);
    }
}

/// A data object of `size` bytes built by its constructor at `0x0067_xxxx` (the family's typed
/// constructors as typed calls) and stored at +0x28.
fn build_data(e: &mut Engine, this: u32, size: u32, constructor: u32) {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    let made = if block == 0 {
        0
    } else {
        match constructor {
            0x0067_acf0 => typed_call!(FAMILY, 0x0067_acf0, type_9_data_construct(e, block)),
            0x0067_b170 => typed_call!(
                FAMILY,
                0x0067_b170,
                package_data::dialogue_construct(e, block)
            ),
            0x0067_b950 => typed_call!(FAMILY, 0x0067_b950, package_data::eat_construct(e, block)),
            0x0067_bbf0 => typed_call!(
                FAMILY,
                0x0067_bbf0,
                package_data::escort_construct(e, block)
            ),
            0x0067_bfd0 => typed_call!(
                FAMILY,
                0x0067_bfd0,
                package_data::follow_construct(e, block)
            ),
            0x0067_c3f0 => typed_call!(
                FAMILY,
                0x0067_c3f0,
                package_data::patrol_construct(e, block)
            ),
            0x0067_c770 => typed_call!(
                FAMILY,
                0x0067_c770,
                package_data::use_item_construct(e, block)
            ),
            _ => e.call(constructor, &args![block]).u32(),
        }
    };
    e.mem.set_u32(at(this, DATA), made);
}

/// The location held at `slot` set from `source` (made when missing), or, for a null `source`,
/// deleted and the slot cleared.
fn replace_location(e: &mut Engine, slot: u32, source: u32) {
    if source == 0 {
        let old = e.mem.u32(slot);
        if old != 0 {
            e.call(LOCATION_DELETE, &args![old, 1u32]);
        }
        e.mem.set_u32(slot, 0);
    } else {
        if e.mem.u32(slot) == 0 {
            let made = new_location(e);
            e.mem.set_u32(slot, made);
        }
        let held = e.mem.u32(slot);
        e.call(LOCATION_COPY, &args![held, source, 0u32]);
    }
}

/// The dialogue data object of a dialogue type package (0 when it has none yet), or `None`.
fn dialogue_data(e: &Engine, this: u32) -> Option<u32> {
    is_dialogue_type(e, this).then(|| e.mem.u32(at(this, DATA)))
}

/// The dialogue data object of a dialogue type package, built when missing, or `None`.
fn dialogue_data_or_create(e: &mut Engine, this: u32) -> Option<u32> {
    if !is_dialogue_type(e, this) {
        return None;
    }
    if e.mem.u32(at(this, DATA)) == 0 {
        build_data(e, this, DIALOGUE_DATA_SIZE, 0x0067_b170);
    }
    Some(e.mem.u32(at(this, DATA)))
}

fn specific_flag(e: &Engine, this: u32, bit: u16) -> bool {
    e.mem.u16(at(this, SPECIFIC_FLAGS)) & bit != 0
}

fn set_specific_flag(e: &mut Engine, this: u32, bit: u16, on: bool) {
    let flags = e.mem.u16(at(this, SPECIFIC_FLAGS));
    let flags = if on { flags | bit } else { flags & !bit };
    e.mem.set_u16(at(this, SPECIFIC_FLAGS), flags);
}

// Typed from 006707c0
/// `TESPackage::TESPackage` (Xbox PDB): the form, the vtable, the schedule, the conditions and the
/// three event actions constructed, the form type 0x49, the package data reset, no references.
/// Returns `this`.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    typed_call!(FAMILY, 0x0048_3370, tesform::construct(e, this));
    e.mem.set_u32(this, PACKAGE_VTABLE);
    typed_call!(
        FAMILY,
        0x0067_fcb0,
        schedule::construct(e, at(this, SCHEDULE))
    );
    typed_call!(
        FAMILY,
        0x0068_0890,
        conditions::construct(e, at(this, CONDITIONS))
    );
    for action in EVENT_ACTIONS {
        typed_call!(
            FAMILY,
            0x0067_da40,
            package_data::event_action_construct(e, at(this, action))
        );
    }
    e.call(SET_FORM_TYPE, &args![this, u32::from(PACKAGE_FORM_TYPE)]);
    typed_call!(FAMILY, 0x0067_09d0, reset(e, this));
    e.mem.set_u32(at(this, REF_COUNT), 0);
    this
}

// Typed from 006709d0
/// The package data reset: the three flag words, the location, target, idle collection, combat
/// style and data object cleared, no schedule, the procedure type -1, no type, not created.
pub fn reset(e: &mut Engine, this: u32) {
    e.mem.set_u32(at(this, PACK_FLAGS), 0);
    e.mem.set_u16(at(this, SPECIFIC_FLAGS), 0);
    e.mem.set_u16(at(this, BEHAVIOR_FLAGS), 0);
    for field in [LOCATION, TARGET, IDLE_COLLECTION, COMBAT_STYLE, DATA] {
        e.mem.set_u32(at(this, field), 0);
    }
    e.call(SET_SCHEDULE, &args![this, 0u32]);
    e.mem.set_u32(at(this, PROCEDURE), u32::MAX);
    e.mem.set_u8(at(this, PACK_TYPE), 0xff);
    typed_call!(FAMILY, 0x0067_4d70, set_is_created(e, this, 0));
}

// Typed from 00670b90
/// `TESPackage::CreatePackage` (Xbox PDB, cdecl): the package object of a type (a subclass for
/// 0xf and 0x1c, 0x12, 0x15, 0x16, 0x17, 0x18 and 0x27; none for -1, 9, 0x11 and 0x28; a plain
/// package for every other type), its type set (0x11 counts as 0). Null when none was built.
pub fn create_package(e: &mut Engine, package_type: i32) -> u32 {
    let package = match package_type {
        -1 | 9 | 0x11 | 0x28 => 0,
        0xf | 0x1c => allocate_and_construct(e, 0xd0, 0x009e_dbc0, &[]),
        // 00670c30: JIP's H16 (its own) makes the combat controller 0x18C bytes (its excluded
        // actions word at +0x188); the seam answers the size (no words). One load while JIP is
        // not installed.
        0x12 => {
            let size = if e.jip_patches() & engine_core::extend::jip_patches::INSTALLED != 0 {
                e.site(0x0067_0c30, &[]).unwrap_or(0x188)
            } else {
                0x188
            };
            allocate_and_construct(e, size, 0x0097_d3a0, &[0, 0, 0])
        }
        0x15 => allocate_and_construct(e, 0x88, 0x009e_c5d0, &[]),
        0x16 => allocate_and_construct(e, 0xac, 0x009f_0e00, &[0, 0, 0]),
        0x17 => allocate_and_construct(e, 0x9c, 0x009f_91b0, &[]),
        0x18 => allocate_and_construct(e, 0xb4, 0x009f_72f0, &[]),
        0x27 => allocate_and_construct(e, 0x8c, 0x009e_d030, &[]),
        _ => {
            let block = e.call(OPERATOR_NEW, &args![0x80u32]).u32();
            if block == 0 {
                0
            } else {
                typed_call!(FAMILY, 0x0067_07c0, construct(e, block))
            }
        }
    };
    if package != 0 {
        typed_call!(FAMILY, 0x0067_0fc0, set_pack_type(e, package, package_type));
    }
    package
}

// Typed from 00670ed0
/// Whether the flag 0x40 is set.
pub fn pack_flag_40(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x40)
}

// Typed from 00670ef0
/// Whether the flag 0x10 is set.
pub fn pack_flag_10(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x10)
}

// Typed from 00670f10
/// Whether the flag 0x80 is set.
pub fn pack_flag_80(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x80)
}

// Typed from 00670f40
/// Whether the flag 0x20 is set.
pub fn pack_flag_20(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x20)
}

// Typed from 00670f60
/// Whether the flag 0x100 is set.
pub fn pack_flag_100(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x100)
}

// Typed from 00670f90
/// `TESPackage::GetOncePerDay` (Xbox PDB): whether the flag 0x400 is set.
pub fn once_per_day(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, FLAG_ONCE_PER_DAY)
}

// Typed from 00670fc0
/// `TESPackage::SetPackType` (Xbox PDB): for a new type (0x11 counts as 0), the old data object
/// deleted, the location, second location or target the new type doesn't keep cleared, the
/// location, target and data object it needs built, the specific flags cleared, the type stored,
/// and for type 9 a second location made when it has none.
pub fn set_pack_type(e: &mut Engine, this: u32, new_type: i32) {
    let new_type = if new_type == 0x11 { 0 } else { new_type };
    if new_type == pack_type(e, this) {
        return;
    }
    let old_data = e.mem.u32(at(this, DATA));
    if old_data != 0 {
        e.vcall(old_data, SLOT_DELETE, &args![1u32]);
    }
    e.mem.set_u32(at(this, DATA), 0);

    match new_type {
        0 | 1 | 2 | 8 | 9 | 10 | 11 | 14 | 16 => {
            typed_call!(FAMILY, 0x0067_1e10, set_second_location(e, this, 0));
        }
        4..=6 | 13 => typed_call!(FAMILY, 0x0067_2fc0, set_target(e, this, 0)),
        7 => typed_call!(FAMILY, 0x0067_1d30, set_location(e, this, 0)),
        12 => {
            typed_call!(FAMILY, 0x0067_2fc0, set_target(e, this, 0));
            typed_call!(FAMILY, 0x0067_1e10, set_second_location(e, this, 0));
        }
        _ => {}
    }

    match new_type {
        0 | 7 | 14 => ensure_target(e, this),
        1 => {
            build_data(e, this, 0xc, 0x0067_bfd0);
            ensure_target(e, this);
        }
        2 => {
            ensure_target(e, this);
            ensure_location(e, this);
            build_data(e, this, 0xc, 0x0067_bbf0);
        }
        3 => {
            ensure_target(e, this);
            let target = e.mem.u32(at(this, TARGET));
            if e.mem.u8(target) == 0 {
                typed_call!(
                    FAMILY,
                    0x0068_00b0,
                    package_target::set_targ_type(e, target, 2)
                );
            }
            build_data(e, this, 8, 0x0067_b950);
            ensure_location(e, this);
        }
        4..=6 | 12 => ensure_location(e, this),
        8 => {
            build_data(e, this, 8, 0x0067_c770);
            ensure_location(e, this);
            ensure_target(e, this);
        }
        9 => {
            ensure_location(e, this);
            build_data(e, this, 8, 0x0067_acf0);
        }
        13 => build_data(e, this, 8, 0x0067_c3f0),
        TYPE_DIALOGUE | TYPE_DIALOGUE_SECOND => {
            ensure_target(e, this);
            build_data(e, this, DIALOGUE_DATA_SIZE, 0x0067_b170);
        }
        TYPE_USE_WEAPON => {
            set_pack_flag(e, this, FLAG_800000, true);
            ensure_target(e, this);
            build_data(e, this, USE_WEAPON_DATA_SIZE, USE_WEAPON_DATA_CONSTRUCT);
        }
        _ => {}
    }

    // Type 0xc's specific flags setters (`00671ad0` to `00671cb0` with 1, `00671a70` with 0) only
    // clear bits of this zero word.
    e.mem.set_u16(at(this, SPECIFIC_FLAGS), 0);
    e.mem.set_u8(at(this, PACK_TYPE), new_type.to_le_bytes()[0]);
    if new_type == 9 && second_location(e, this) == 0 {
        let location = new_location(e);
        typed_call!(FAMILY, 0x0067_1e10, set_second_location(e, this, location));
        if location != 0 {
            e.call(LOCATION_DELETE, &args![location, 1u32]);
        }
    }
}

// Typed from 00671a20
/// The flag 0x0080_0000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_800000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, FLAG_800000, value != 0);
}

// Typed from 00671a70
/// The specific flag 0x40 set for a byte other than zero, else cleared.
pub fn set_specific_flag_40(e: &mut Engine, this: u32, value: u8) {
    set_specific_flag(e, this, 0x40, value != 0);
}

// Typed from 00671ad0
/// The specific flag 0x1 cleared for a byte other than zero, else set (stored inverted).
pub fn set_specific_flag_1_clear(e: &mut Engine, this: u32, value: u8) {
    set_specific_flag(e, this, 0x1, value == 0);
}

// Typed from 00671b30
/// The specific flag 0x2 cleared for a byte other than zero, else set (stored inverted).
pub fn set_specific_flag_2_clear(e: &mut Engine, this: u32, value: u8) {
    set_specific_flag(e, this, 0x2, value == 0);
}

// Typed from 00671b90
/// The specific flag 0x4 cleared for a byte other than zero, else set (stored inverted).
pub fn set_specific_flag_4_clear(e: &mut Engine, this: u32, value: u8) {
    set_specific_flag(e, this, 0x4, value == 0);
}

// Typed from 00671bf0
/// The specific flag 0x8 cleared for a byte other than zero, else set (stored inverted).
pub fn set_specific_flag_8_clear(e: &mut Engine, this: u32, value: u8) {
    set_specific_flag(e, this, 0x8, value == 0);
}

// Typed from 00671c50
/// The specific flag 0x10 cleared for a byte other than zero, else set (stored inverted).
pub fn set_specific_flag_10_clear(e: &mut Engine, this: u32, value: u8) {
    set_specific_flag(e, this, 0x10, value == 0);
}

// Typed from 00671cb0
/// The specific flag 0x20 cleared for a byte other than zero, else set (stored inverted).
pub fn set_specific_flag_20_clear(e: &mut Engine, this: u32, value: u8) {
    set_specific_flag(e, this, 0x20, value == 0);
}

// Typed from 00671d30
/// `TESPackage::SetPackageLocation` (Xbox PDB): the package's own location set from `location`
/// (made when missing); a null `location` deletes it.
pub fn set_location(e: &mut Engine, this: u32, location: u32) {
    replace_location(e, at(this, LOCATION), location);
}

// Typed from 00671e10
/// `TESPackage::SetPackageSecondLocation` (Xbox PDB): the second location in the data object of
/// the package's type (built when missing; the use weapon data through its getter) set from
/// `location`, or deleted for a null one; nothing for a type without one.
pub fn set_second_location(e: &mut Engine, this: u32, location: u32) {
    if is_dialogue_type(e, this) {
        if e.mem.u32(at(this, DATA)) == 0 {
            build_data(e, this, DIALOGUE_DATA_SIZE, 0x0067_b170);
        }
        let data = e.mem.u32(at(this, DATA));
        replace_location(e, at(data, DIALOGUE_SECOND_LOCATION), location);
        return;
    }
    let (size, constructor) = match pack_type(e, this) {
        3 => (8, 0x0067_b950),
        2 => (0xc, 0x0067_bbf0),
        9 => (8, 0x0067_acf0),
        1 => (0xc, 0x0067_bfd0),
        8 => (8, 0x0067_c770),
        TYPE_USE_WEAPON => {
            let data = typed_call!(FAMILY, 0x0067_2d10, use_weapon_data(e, this));
            replace_location(e, at(data, USE_WEAPON_SECOND_LOCATION), location);
            return;
        }
        _ => return,
    };
    if e.mem.u32(at(this, DATA)) == 0 {
        build_data(e, this, size, constructor);
    }
    let data = e.mem.u32(at(this, DATA));
    replace_location(e, at(data, SMALL_SECOND_LOCATION), location);
}

// Typed from 00672710
/// `TESPackage::GetDialogueSayToFlag` (Xbox PDB): the dialogue data's byte at +0x18, or 0.
pub fn dialogue_say_to(e: &mut Engine, this: u32) -> u8 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => e.mem.u8(at(data, DIALOGUE_SAY_TO)),
        _ => 0,
    }
}

// Typed from 00672760
/// `TESPackage::GetDialogueTopic` (Xbox PDB): the dialogue data's topic, or null.
pub fn dialogue_topic(e: &mut Engine, this: u32) -> u32 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => e.mem.u32(at(data, DIALOGUE_TOPIC)),
        _ => 0,
    }
}

// Typed from 006727b0
/// `TESPackage::GetDialogueDoNotControlTarget` (Xbox PDB): the dialogue data's byte at +0x11, or
/// 0.
pub fn dialogue_do_not_control_target(e: &mut Engine, this: u32) -> u8 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => e.mem.u8(at(data, DIALOGUE_DO_NOT_CONTROL_TARGET)),
        _ => 0,
    }
}

// Typed from 00672800
/// `TESPackage::GetDialogueNoHeadtrack` (Xbox PDB): the dialogue data's byte at +0x10, or 0.
pub fn dialogue_no_headtrack(e: &mut Engine, this: u32) -> u8 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => e.mem.u8(at(data, DIALOGUE_NO_HEADTRACKING)),
        _ => 0,
    }
}

// Typed from 00672850
/// The dialogue data's `fFov` (Xbox PDB, +4) as `FLD` loads it, or 1.0 when the package has none.
pub fn dialogue_fov(e: &mut Engine, this: u32) -> f32 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => {
            f32::from_bits(x87_quiet_bits(e.mem.u32(at(data, DIALOGUE_FOV))))
        }
        _ => 1.0,
    }
}

// Typed from 006728a0
/// `TESPackage::GetAcquireRadius` (Xbox PDB): the radius written at `radius` (0.0 first) and
/// whether the package has one: for type 0xc the count `00676280` gives (converted from an
/// unsigned word), else the target's float when there is a target and the float is not at or
/// below 0.0.
pub fn acquire_radius(e: &mut Engine, this: u32, argument: u32, radius: u32) -> bool {
    e.mem.set_f32(radius, 0.0);
    if pack_type(e, this) == 0xc {
        let count = typed_call!(
            FAMILY,
            0x0067_6280,
            package_place::location_radius_for(e, this, argument)
        );
        let count = u32::from_ne_bytes(count.to_ne_bytes());
        e.mem.set_u32(radius, x87_store32(f64::from(count)));
        return true;
    }
    let target = e.mem.u32(at(this, TARGET));
    if target == 0 {
        return false;
    }
    let value = x87_load32(e.mem.u32(at(target, 0xc)));
    if value <= ZERO {
        return false;
    }
    e.mem.set_u32(radius, x87_store32(value));
    true
}

// Typed from 00672930
/// `TESPackage::GetInitialTargetCount` (Xbox PDB): 0 without a target; 1 for a target of type 0
/// or 3; the target's value when it is above zero; else 1 for the procedure 0x1c, 0x7fff_ffff
/// for 2, 3 and 0x1a, 0 for the others.
pub fn initial_target_count(e: &mut Engine, this: u32) -> i32 {
    let target = e.mem.u32(at(this, TARGET));
    if target == 0 {
        return 0;
    }
    if matches!(e.mem.u8(target), 0 | 3) {
        return 1;
    }
    let value = e.mem.i32(at(target, 8));
    if value > 0 {
        return value;
    }
    match e.mem.u32(at(this, PROCEDURE)) {
        0x1c => 1,
        2 | 3 | 0x1a => 0x7fff_ffff,
        _ => 0,
    }
}

// Typed from 006729d0
/// The dialogue data's topic set (the data built when missing); nothing for another type.
pub fn set_dialogue_topic(e: &mut Engine, this: u32, topic: u32) {
    if let Some(data) = dialogue_data_or_create(e, this) {
        e.mem.set_u32(at(data, DIALOGUE_TOPIC), topic);
    }
}

// Typed from 00672aa0
/// The dialogue data's byte at +0x11 set (the data built when missing).
pub fn set_dialogue_do_not_control_target(e: &mut Engine, this: u32, value: u8) {
    if let Some(data) = dialogue_data_or_create(e, this) {
        e.mem
            .set_u8(at(data, DIALOGUE_DO_NOT_CONTROL_TARGET), value);
    }
}

// Typed from 00672b70
/// The dialogue data's byte at +0x10 set (the data built when missing).
pub fn set_dialogue_no_headtrack(e: &mut Engine, this: u32, value: u8) {
    if let Some(data) = dialogue_data_or_create(e, this) {
        e.mem.set_u8(at(data, DIALOGUE_NO_HEADTRACKING), value);
    }
}

// Typed from 00672c40
/// The dialogue data's `fFov` set through the exe's `FLD`/`FSTP` (the data built when missing).
pub fn set_dialogue_fov(e: &mut Engine, this: u32, value: f32) {
    if let Some(data) = dialogue_data_or_create(e, this) {
        e.mem
            .set_u32(at(data, DIALOGUE_FOV), x87_quiet_bits(value.to_bits()));
    }
}

// Typed from 00672d10
/// `TESPackage::GetOrCreateUseWeaponPackageData` (Xbox PDB): the assert for another type (it goes
/// on), the use weapon data built when missing, the data object.
pub fn use_weapon_data(e: &mut Engine, this: u32) -> u32 {
    if pack_type(e, this) != TYPE_USE_WEAPON {
        e.call(ASSERT, &args![PACKAGE_SOURCE_FILE, 0x607u32]);
    }
    if e.mem.u32(at(this, DATA)) == 0 {
        build_data(e, this, USE_WEAPON_DATA_SIZE, USE_WEAPON_DATA_CONSTRUCT);
    }
    e.mem.u32(at(this, DATA))
}

// Typed from 00672dd0
/// The second location in the data object of the package's type: +0xc for the dialogue types,
/// +4 for the types 1, 2, 3, 8 and 9, +8 for the use weapon type; null without a data object or
/// for another type.
pub fn second_location(e: &mut Engine, this: u32) -> u32 {
    let data = e.mem.u32(at(this, DATA));
    if data == 0 {
        return 0;
    }
    let offset = match pack_type(e, this) {
        TYPE_DIALOGUE | TYPE_DIALOGUE_SECOND => DIALOGUE_SECOND_LOCATION,
        1 | 2 | 3 | 8 | 9 => SMALL_SECOND_LOCATION,
        TYPE_USE_WEAPON => USE_WEAPON_SECOND_LOCATION,
        _ => return 0,
    };
    e.mem.u32(at(data, offset))
}

// Typed from 00672f20
/// `TESPackage::GetPackageSearchLocation` (Xbox PDB): the package's own location for the types 0,
/// 0xc, 0x11 and 0x1c; for 2, 3, 8 and 0x10 the second location, or the own location when the
/// second one is of location type 7; null for the others.
pub fn search_location(e: &mut Engine, this: u32) -> u32 {
    match pack_type(e, this) {
        0 | 0xc | 0x11 | 0x1c => e.mem.u32(at(this, LOCATION)),
        2 | 3 | 8 | TYPE_USE_WEAPON => {
            let second = second_location(e, this);
            if second != 0 && i32::from(e.mem.i8(second)) == LOCATION_TYPE_SEVEN {
                e.mem.u32(at(this, LOCATION))
            } else {
                second
            }
        }
        _ => 0,
    }
}

// Typed from 00672fc0
/// `TESPackage::SetPackageTarget` (Xbox PDB): the package's own target set from `target` (made
/// when missing); a null `target` deletes it.
pub fn set_target(e: &mut Engine, this: u32, target: u32) {
    if target == 0 {
        let old = e.mem.u32(at(this, TARGET));
        if old != 0 {
            e.call(TARGET_DELETE, &args![old, 1u32]);
        }
        e.mem.set_u32(at(this, TARGET), 0);
    } else {
        ensure_target(e, this);
        let held = e.mem.u32(at(this, TARGET));
        typed_call!(FAMILY, 0x0068_01a0, package_target::copy(e, held, target));
    }
}

// Typed from 00674930
/// `TESPackage::InitItem` (Xbox PDB): once per package (nothing when the form's flag 8 is set),
/// the location, second location, target, conditions, idle collection and data object
/// initialised, the combat style's id resolved (the file's compile index, the form map, the cast;
/// a warning when none is found), the event actions given their kinds 0, 1, 2 and initialised,
/// the package marked never to run when the loader's warnings grew, the types' own locations and
/// targets set up (9, 0xc, 0xd, 0xe), and the form's flag 8 set.
pub fn init_item(e: &mut Engine, this: u32) {
    if e.mem.u32(at(this, FORM_FLAGS)) & FORM_FLAG_INITIALISED != 0 {
        return;
    }
    let warnings_before = e.call(WARNING_COUNT, &[]).u32();
    let location = e.mem.u32(at(this, LOCATION));
    if location != 0 {
        e.call(LOCATION_INIT_ITEM, &args![location, this]);
    }
    let second = second_location(e, this);
    if second != 0 {
        e.call(LOCATION_INIT_ITEM, &args![second, this]);
    }
    let target = e.mem.u32(at(this, TARGET));
    if target != 0 {
        typed_call!(
            FAMILY,
            0x0068_03b0,
            package_target::init_item(e, target, this)
        );
    }
    let conditions = at(this, CONDITIONS);
    if conditions != 0 {
        e.call(CONDITIONS_INIT_ITEM, &args![conditions, this]);
    }
    let idles = e.mem.u32(at(this, IDLE_COLLECTION));
    if idles != 0 {
        e.call(IDLE_COLLECTION_INIT_ITEM, &args![idles, this]);
    }
    let data = e.mem.u32(at(this, DATA));
    if data != 0 {
        e.vcall(data, SLOT_DATA_INIT_ITEM, &args![this]);
    }

    // The combat style is held as a form id until now.
    let style_id = e.mem.u32(at(this, COMBAT_STYLE));
    if style_id != 0 {
        let (resolved, style) = e.with_stack(4, |e, slot| {
            let slot = slot.addr();
            e.mem.set_u32(slot, style_id);
            let file = typed_call!(FAMILY, 0x0048_4e60, tesform::get_file(e, this, -1));
            typed_call!(
                FAMILY,
                0x0048_5d50,
                tesform::add_compile_index(e, slot, file)
            );
            let resolved = e.mem.u32(slot);
            let form = typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, resolved));
            let style = e
                .call(
                    DYNAMIC_CAST,
                    &args![form, 0u32, TYPE_TES_FORM, TYPE_COMBAT_STYLE, 0u32],
                )
                .u32();
            (resolved, style)
        });
        e.mem.set_u32(at(this, COMBAT_STYLE), style);
        if style == 0 {
            let id = e.mem.u32(at(this, FORM_ID));
            let name = e.vcall(this, SLOT_FORM_NAME, &[]).u32();
            e.call(WARNING, &args![TEXT_NO_COMBAT_STYLE, resolved, name, id]);
        }
    }

    for (action, kind) in EVENT_ACTIONS.into_iter().zip(0u32..) {
        e.mem.set_u32(at(at(this, action), 0xc), kind);
    }
    for action in EVENT_ACTIONS {
        e.call(EVENT_ACTION_INIT_ITEM, &args![at(this, action), this]);
    }

    if e.call(WARNING_COUNT, &[]).u32() != warnings_before {
        let id = e.mem.u32(at(this, FORM_ID));
        let name = e.vcall(this, SLOT_FORM_NAME, &[]).u32();
        e.call(WARNING, &args![TEXT_WARNINGS_GENERATED, name, id]);
        set_pack_flag(e, this, FLAG_NEVER_TO_RUN, true);
    }

    // The types' own locations and targets.
    match pack_type(e, this) {
        9 => {
            if specific_flag(e, this, 0x2) {
                let location = e.mem.u32(at(this, LOCATION));
                e.call(LOCATION_SET_TYPE, &args![location, LOCATION_TYPE_SIX]);
            }
        }
        0xc => {
            if e.call(TYPE_C_DECIDES_LOCATION, &args![this]).bool() {
                let location = e.mem.u32(at(this, LOCATION));
                e.call(LOCATION_SET_TYPE, &args![location, LOCATION_TYPE_SIX]);
            }
        }
        0xd => {
            let data = e.mem.u32(at(this, DATA));
            if e.mem.u8(at(data, 5)) != 0 {
                let location = e.mem.u32(at(this, LOCATION));
                e.call(LOCATION_SET_TYPE, &args![location, LOCATION_TYPE_SIX]);
            }
            if e.mem.u32(at(this, LOCATION)) == 0 {
                let made = new_location(e);
                e.call(LOCATION_SET_TYPE, &args![made, LOCATION_TYPE_SIX]);
                typed_call!(FAMILY, 0x0067_1d30, set_location(e, this, made));
                if made != 0 {
                    e.call(LOCATION_DELETE, &args![made, 1u32]);
                }
            }
        }
        0xe if specific_flag(e, this, 0x2) => {
            if e.mem.u32(at(this, TARGET)) == 0 {
                let made = new_target(e);
                typed_call!(FAMILY, 0x0067_2fc0, set_target(e, this, made));
            }
            let target = e.mem.u32(at(this, TARGET));
            typed_call!(
                FAMILY,
                0x0068_00b0,
                package_target::set_targ_type(e, target, 3)
            );
        }
        _ => {}
    }
    typed_call!(FAMILY, 0x0048_4ab0, tesform::set_flag_8(e, this, true));
}

// Typed from 00674d20
/// Whether the specific flag 0x2 is set.
pub fn specific_flag_2(e: &mut Engine, this: u32) -> bool {
    specific_flag(e, this, 0x2)
}

// Typed from 00674d40
/// `TESPackage::GetIsCreated` (Xbox PDB): whether the flag 0x800 is set.
pub fn is_created(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, FLAG_CREATED)
}

// Typed from 00674d70
/// `TESPackage::SetIsCreated` (Xbox PDB): the flag 0x800 set or cleared, only for a package with a
/// created id (from 0xff00_0000 up).
pub fn set_is_created(e: &mut Engine, this: u32, created: u8) {
    if e.mem.u32(at(this, FORM_ID)) < CREATED_IDS {
        return;
    }
    set_pack_flag(e, this, FLAG_CREATED, created != 0);
}

// Typed from 00674dd0
/// `TESPackage::IsScriptPackage` (Xbox PDB): whether the flag 0x4000 is set.
pub fn is_script_package(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, FLAG_SCRIPT_PACKAGE)
}

// Typed from 00674e00
/// The flag 0x4000 (a script package) set for a byte other than zero, else cleared.
pub fn set_script_package(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, FLAG_SCRIPT_PACKAGE, value != 0);
}

// Typed from 00674e40
/// `TESPackage::IsNeverToRun` (Xbox PDB): whether the flag 0x8000 is set.
pub fn is_never_to_run(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, FLAG_NEVER_TO_RUN)
}

// Typed from 00674e70
/// `TESPackage::SetNeverRun` (Xbox PDB): when the package's target is the reference `reference`
/// (`00680020` in place: a target of type 0 holds its reference at +4, another none), the flag
/// 0x8000 set for a byte other than zero or cleared, and for a package without a created id the
/// form told through its slot 0x48 (set) or 0x4c (cleared) with the mask 0x8000_0000.
pub fn set_never_run(e: &mut Engine, this: u32, reference: u32, value: u8) {
    let target = e.mem.u32(at(this, TARGET));
    if target == 0 {
        return;
    }
    let held = if e.mem.u8(target) == 0 {
        e.mem.u32(at(target, 4))
    } else {
        0
    };
    if held != reference {
        return;
    }
    let set = value != 0;
    set_pack_flag(e, this, FLAG_NEVER_TO_RUN, set);
    if e.mem.u32(at(this, FORM_ID)) < CREATED_IDS {
        let slot = if set {
            SLOT_FORM_SET_CHANGED
        } else {
            SLOT_FORM_CLEAR_CHANGED
        };
        e.vcall(this, slot, &args![NEVER_RUN_CHANGE]);
    }
}

// Typed from 0067acf0
/// The type 9 data's constructor: its vtable (over `TESPackageData`'s) and no second location.
/// Returns `this`.
pub fn type_9_data_construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, TYPE_9_DATA_VTABLE);
    e.mem.set_u32(at(this, SMALL_SECOND_LOCATION), 0);
    this
}

// Typed from 00674f30
/// The flag 0x10000 set for a byte other than zero, else cleared, and for a package without a
/// created id the form told through its slot 0x48 (set) or 0x4c (cleared) with the mask
/// 0x4000_0000.
pub fn set_pack_flag_10000(e: &mut Engine, this: u32, value: u8) {
    let set = value != 0;
    set_pack_flag(e, this, FLAG_10000, set);
    if e.mem.u32(at(this, FORM_ID)) < CREATED_IDS {
        let slot = if set {
            SLOT_FORM_SET_CHANGED
        } else {
            SLOT_FORM_CLEAR_CHANGED
        };
        e.vcall(this, slot, &args![FLAG_10000_CHANGE]);
    }
}

/// The data object when the package type is `kind`, else 0.
fn data_of_type(e: &Engine, this: u32, kind: i32) -> u32 {
    if pack_type(e, this) == kind {
        e.mem.u32(at(this, DATA))
    } else {
        0
    }
}

// Typed from 006758f0
/// `TESPackage::GetUseWeaponPackageData` (Xbox PDB): the data object of a type 0x10 package, else 0.
pub fn use_weapon_data_held(e: &mut Engine, this: u32) -> u32 {
    data_of_type(e, this, TYPE_USE_WEAPON)
}

// Typed from 00675920
/// `TESPackage::GetPatrolPackageData` (Xbox PDB): the data object of a type 0xd package, else 0.
pub fn patrol_data(e: &mut Engine, this: u32) -> u32 {
    data_of_type(e, this, 0xd)
}

// Typed from 00675950
/// `TESPackage::GetFollowPackageData` (Xbox PDB): the data object of a type 1 package, else 0.
pub fn follow_data(e: &mut Engine, this: u32) -> u32 {
    data_of_type(e, this, 1)
}

// Typed from 00675980
/// The data object of a type 0xe package, else 0.
pub fn type_e_data(e: &mut Engine, this: u32) -> u32 {
    data_of_type(e, this, 0xe)
}

// Typed from 006759b0
/// `TESPackage::GetEscortPackageData` (Xbox PDB): the data object of a type 2 package, else 0.
pub fn escort_data(e: &mut Engine, this: u32) -> u32 {
    data_of_type(e, this, 2)
}

// Typed from 006784c0
/// `TESPackage::GetEscortFollowDistance` (Xbox PDB): an escort package's (type 2) follow distance,
/// the word at +8 of its data object; else 0x100.
pub fn escort_follow_distance(e: &mut Engine, this: u32) -> u32 {
    if pack_type(e, this) == 2 {
        let data = e.mem.u32(at(this, DATA));
        e.mem.u32(at(data, 8))
    } else {
        0x100
    }
}

// Typed from 00678610
/// `TESPackage::IsInterruptPackage` (Xbox PDB): the package types 0x12 to 0x19, 0x1b to 0x1d and
/// 0x1f to 0x24.
pub fn is_interrupt_package(e: &mut Engine, this: u32) -> bool {
    matches!(pack_type(e, this), 0x12..=0x19 | 0x1b..=0x1d | 0x1f..=0x24)
}

// Typed from 00678d00
/// `TESPackage::GetShouldReserveTarget` (Xbox PDB): the procedure types 2 to 5 and 0x1a
/// (`009611e0` in place).
pub fn should_reserve_target(e: &mut Engine, this: u32) -> bool {
    let procedure = e.mem.i32(at(this, PROCEDURE));
    (2..=5).contains(&procedure) || procedure == 0x1a
}

/// A form's `+0x3c` member's flag (its slot 4).
fn member_flag(e: &mut Engine, form: u32) -> bool {
    e.vcall(at(form, FORM_MEMBER_3C), SLOT_MEMBER_FLAG, &args![])
        .bool()
}

/// The two tests on a form of type 0x28 (`006450c0`, `004c0c30` in place): the signed byte at
/// +0xf4 from 0 to 2, and from 3 to 13.
fn form_28_kind(e: &Engine, form: u32) -> i8 {
    e.mem.i8(at(form, FORM_28_KIND))
}

// Typed from 00679ba0
/// `TESPackage::GetObjectTypeFromForm` (Xbox PDB): a form's package object type (0 for none) from
/// its form type: 0x14 0x18, 0x15 1, 0x18 to 0x1c 2 to 6, 0x1d 7 (0x12 when its member's flag
/// holds), 0x1e 8, 0x1f, 0x32, 0x67, 0x6c, 0x73 and 0x74 9, 0x26 10, 0x27 11, 0x28 0xc (0x16 or
/// 0x17 by its two tests), 0x29 to 0x2b 0xd to 0xf, 0x2e 0x10, 0x2f 0x11 (0x12 by its member).
pub fn object_type_from_form(e: &mut Engine, form: u32) -> i32 {
    if form == 0 {
        return 0;
    }
    match e.mem.u8(at(form, FORM_TYPE)) {
        0x14 => 0x18,
        0x15 => 1,
        kind @ 0x18..=0x1c => i32::from(kind) - 0x16,
        0x1d => {
            if member_flag(e, form) {
                0x12
            } else {
                7
            }
        }
        0x1e => 8,
        0x1f | 0x32 | 0x67 | 0x6c | 0x73 | 0x74 => 9,
        0x26 => 10,
        0x27 => 0xb,
        0x28 => match form_28_kind(e, form) {
            0..=2 => 0x16,
            3..=13 => 0x17,
            _ => 0xc,
        },
        kind @ 0x29..=0x2b => i32::from(kind) - 0x1c,
        0x2e => 0x10,
        0x2f => {
            if member_flag(e, form) {
                0x12
            } else {
                0x11
            }
        }
        _ => 0,
    }
}

// Typed from 00679e00
/// `TESPackage::FormMatchesPackageObjectType` (Xbox PDB): whether a form satisfies a package
/// object type (neither 0): by form type, 0x14 matches 0x18 and 0x19 to 0x1b by its effect list's
/// three tests; the plain types match their own object type ([`object_type_from_form`]); 0x18 and
/// 0x29 also 0x13 and 0x14, 0x1a also 0x14, 0x2a and 0x2b also 0x1c, 0x28 also 0x13 and 0x14 and
/// 0x16 or 0x17 by its tests, 0x1d and 0x2f 0x12 by their member's flag.
pub fn form_matches_object_type(e: &mut Engine, form: u32, object_type: i32) -> bool {
    if form == 0 || object_type == 0 {
        return false;
    }
    let either = matches!(object_type, 0x13 | 0x14);
    match e.mem.u8(at(form, FORM_TYPE)) {
        0x14 => match object_type {
            0x18 => true,
            0x19 => e.call(EFFECT_LIST_TARGET, &args![at(form, 0x24)]).bool(),
            0x1a => e.call(EFFECT_LIST_TOUCH, &args![at(form, 0x24)]).bool(),
            0x1b => e.call(EFFECT_LIST_THIRD, &args![at(form, 0x24)]).bool(),
            _ => false,
        },
        0x15 => object_type == 1,
        0x18 => object_type == 2 || either,
        0x19 => object_type == 3,
        0x1a => object_type == 4 || object_type == 0x14,
        0x1b => object_type == 5,
        0x1c => object_type == 6,
        0x1d => object_type == 7 || (object_type == 0x12 && member_flag(e, form)),
        0x1e => object_type == 8,
        0x1f | 0x32 | 0x67 | 0x6c | 0x73 | 0x74 => object_type == 9,
        0x26 => object_type == 10,
        0x27 => object_type == 0xb,
        0x28 => match object_type {
            0xc | 0x13 | 0x14 => true,
            0x16 => (0..=2).contains(&form_28_kind(e, form)),
            0x17 => (3..=13).contains(&form_28_kind(e, form)),
            _ => false,
        },
        0x29 => object_type == 0xd || either,
        0x2a => object_type == 0xe || object_type == 0x1c,
        0x2b => object_type == 0xf || object_type == 0x1c,
        0x2e => object_type == 0x10,
        0x2f => object_type == 0x11 || (object_type == 0x12 && member_flag(e, form)),
        _ => false,
    }
}
