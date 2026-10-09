//! `fallout shared/tesobjectrefr.cpp` (Xbox PDB source unit), part 2: its functions from `00563d80` up to
//! (not including) `0056a860` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesobjectrefr`]; anything public there may be used here.
//!
//! # Where this file stops
//!
//! The first batch holds the 40 functions from `00563d80` to `00567400`: the
//! sequence save/load helpers of a reference's controller manager, the
//! reference factory, the form-flag accessors (persistent, targeted, ...),
//! `InitScript`, `RunScript`, `InitAnimation` and `BuildKFFileList`. The next
//! session continues with the next open function after `00567400` in address
//! order (`00567470`).
//!
//! # Conventions of the exe worth knowing
//!
//! - MSVC pushes a `float` argument as `PUSH ECX; FSTP [ESP]`: the `PUSH ECX`
//!   only reserves the four bytes, the value is the stored `float`.
//! - A word pushed before a call to a function that pops fewer words
//!   belongs to a later call; every call here was matched against the
//!   callee's `RET n` (so `GetModel` `005715d0`, `Get3D` (virtual `+0x1d0`)
//!   and the `NiControllerManager` getters take no stack words).
//! - Scope guards (`00404eb0`, 4 bytes on the stack) and C++ exception
//!   unwinding are modelled only as far as the guard's constructor and
//!   destructor calls; the unwinding states and the stack-protector
//!   cookies are not translated.

#[allow(unused_imports)]
use super::tesobjectrefr::*;
#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------------
// Callees outside this file, by exe address.

/// `TESObjectREFR::GetBaseObject` as the map names `007af430`: the
/// reference's base form (`pObjectReference`), 0 when it has none.
const GET_BASE_FORM: u32 = 0x007a_f430;
/// `TESForm::cFormType` (`00401170`, the byte at +4), `iFormID`
/// (`0084e3a0`, +0x0c), `iFormFlags` getter (`0044ddc0`, +8) and setter
/// (`00403550`).
const FORM_TYPE: u32 = 0x0040_1170;
const FORM_ID: u32 = 0x0084_e3a0;
const FORM_GET_FLAGS: u32 = 0x0044_ddc0;
const FORM_SET_FLAGS: u32 = 0x0040_3550;
/// `iFormFlags & 0x20` (deleted) and `iFormFlags & 0x800` (disabled).
const FORM_IS_DELETED: u32 = 0x0044_0d80;
const FORM_IS_DISABLED: u32 = 0x0044_0da0;
/// `this + 0x44`: the reference's embedded `ExtraDataList`.
const GET_EXTRA_LIST: u32 = 0x005d_43c0;
/// `operator new(size)` and the scope guard the allocation tag is set by.
const OPERATOR_NEW: u32 = 0x0040_1000;
const SCOPE_GUARD_CTOR: u32 = 0x0040_4eb0;
const SCOPE_GUARD_DTOR: u32 = 0x0040_4ee0;
const SCOPE_GUARD_SIZE: u32 = 4;
/// Path of `TESObjectREFR.cpp` as the allocation scope guard records it.
const SOURCE_FILE: u32 = 0x0102_fc20;
/// The error log (`005b5e40`, cdecl: format, then its arguments).
const LOG_MESSAGE: u32 = 0x005b_5e40;
/// `TESObjectREFR::GetModel` (`005715d0`): the base form's model path.
const GET_MODEL: u32 = 0x0057_15d0;
/// The reference's loaded 3D (`0043fcd0`), a 3D object's first controller
/// (`0043b230`), and the checked cast of a controller to a controller
/// manager (`00653270`, cdecl, with the record at `011f36ac`).
const GET_LOADED_3D: u32 = 0x0043_fcd0;
const GET_CONTROLLER: u32 = 0x0043_b230;
const CHECKED_CAST: u32 = 0x0065_3270;
const MANAGER_TYPE: u32 = 0x011f_36ac;
/// `NiControllerManager` methods: sequence count (`00495d00`), sequence at
/// index (`00495d20`), `DeactivateAll(time)` (`0048fef0`), the manager
/// switch (`0047aa40(flag)`), activate (`0047aab0`), "has sequences"
/// (`004f05a0`), lookup by `NiFixedString` (`0047a520`) and
/// `0047b220(sequence, time)`.
const MANAGER_SEQUENCE_COUNT: u32 = 0x0049_5d00;
const MANAGER_SEQUENCE_AT: u32 = 0x0049_5d20;
const MANAGER_DEACTIVATE_ALL: u32 = 0x0048_fef0;
const MANAGER_SET_FLAG: u32 = 0x0047_aa40;
const MANAGER_ACTIVATE: u32 = 0x0047_aab0;
const MANAGER_HAS_SEQUENCES: u32 = 0x004f_05a0;
const MANAGER_FIND_SEQUENCE: u32 = 0x0047_a520;
const MANAGER_FINISH_SEQUENCE: u32 = 0x0047_b220;
/// A sequence's methods: its name holder (`00413f40`, `this + 8`), the C
/// string of a name holder (`0043b1b0`), `LowProcess::GetGenericLocation` as
/// the map names `008041a0` (non-zero for the sequences that are saved),
/// the time offset (`00759450`, ST0), two more `float` getters (`00508100`
/// and `00639aa0`, ST0), the offset setter (`0098adb0`), and the save and
/// load through a `BGSSaveGameBuffer` (`004efc90`, `004efd10`).
const SEQUENCE_NAME_HOLDER: u32 = 0x0041_3f40;
const NAME_TEXT: u32 = 0x0043_b1b0;
const SEQUENCE_IS_GENERIC: u32 = 0x0080_41a0;
const SEQUENCE_OFFSET_TIME: u32 = 0x0075_9450;
const SEQUENCE_LIMIT_TIME: u32 = 0x0050_8100;
const SEQUENCE_DURATION: u32 = 0x0063_9aa0;
const SEQUENCE_SET_OFFSET: u32 = 0x0098_adb0;
const SEQUENCE_SAVE_TO_BUFFER: u32 = 0x004e_fc90;
const SEQUENCE_LOAD_FROM_BUFFER: u32 = 0x004e_fd10;
/// A `BSAnimGroupSequence` on the stack: its constructor (`0049baa0`) and
/// destructor (`004eeb00`); the object is 0x84 bytes.
const SEQUENCE_CONSTRUCT: u32 = 0x0049_baa0;
const SEQUENCE_DESTRUCT: u32 = 0x004e_eb00;
const SEQUENCE_OBJECT_SIZE: u32 = 0x84;
/// `bhkNiCollisionObject::ResetSim(node, 1)` (`00c6bd00`, cdecl), the 12-byte
/// velocity record maker (`0043d410(record, value, a, b)`) and the two
/// functions that give a 3D object such a record (set `00a59c60`, add
/// `00a59c90`).
const RESET_SIM: u32 = 0x00c6_bd00;
const MAKE_VELOCITY: u32 = 0x0043_d410;
const SET_3D_VELOCITY: u32 = 0x00a5_9c60;
const ADD_3D_VELOCITY: u32 = 0x00a5_9c90;
/// Whether a 3D object is an actor's (`004b5c80`, cdecl).
const IS_ACTOR_3D: u32 = 0x004b_5c80;
/// `strcmp` (`00ec6da0`, cdecl) and the exe's own string compare
/// (`00408b20`, cdecl; 0 when equal).
const STRCMP: u32 = 0x00ec_6da0;
const STRING_COMPARE: u32 = 0x0040_8b20;
/// `_strrchr` (`00ec6e30`) and the unit's wrapper over it (`0040ab30`),
/// `_strnicmp` (`00ec7ec0`), `strstr` (`00ec7750`), `sprintf` (`00ec623a`),
/// and the bounded copy and append (`00406d30(dest, size, source)`,
/// `00406d50`), all cdecl.
const STRRCHR: u32 = 0x00ec_6e30;
const STRRCHR_WRAPPER: u32 = 0x0040_ab30;
const STRNICMP: u32 = 0x00ec_7ec0;
const STRSTR: u32 = 0x00ec_7750;
const SPRINTF: u32 = 0x00ec_623a;
const BOUNDED_COPY: u32 = 0x0040_6d30;
const BOUNDED_APPEND: u32 = 0x0040_6d50;
/// `NiFixedString` constructor from a C string (`00438170`) and destructor
/// (`004381b0`); the C string pointers of the two idle sequence names and of
/// "Unequip" are kept in the exe's data.
const FIXED_STRING_CONSTRUCT: u32 = 0x0043_8170;
const FIXED_STRING_DESTRUCT: u32 = 0x0043_81b0;
const NAME_UNEQUIP: u32 = 0x0119_7b5c;
const NAME_IDLE_A: u32 = 0x0119_77d8;
const NAME_IDLE_B: u32 = 0x0119_7820;
/// `float` constants of the exe: `FLT_MAX` (`0102f510`), `-1.0f`
/// (`01012054`), and the `double`s `-1.0` (`0101a6b0`), `0.0` (`01012060`),
/// `20.0` (`0102fc70`) and `0.0166666` (`0102fc68`, with the `float` at
/// `0102fc64`); `011c3c08` is the default blend time.
const FLOAT_MAX: u32 = 0x0102_f510;
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
const MINUS_ONE_DOUBLE: u32 = 0x0101_a6b0;
const ZERO_DOUBLE: u32 = 0x0101_2060;
const STEP_DIVISOR: u32 = 0x0102_fc70;
const MIN_STEP_DOUBLE: u32 = 0x0102_fc68;
const MIN_STEP_FLOAT: u32 = 0x0102_fc64;
const DEFAULT_BLEND_TIME: u32 = 0x011c_3c08;
/// The `BGSSaveGameBuffer` / `BGSLoadGameBuffer` methods: start and end of a
/// variable sized value (`00865f20`, `00865ff0`; `00864a60` loads the
/// count), the save of a string (`00865e70(text, 0)`) and the load of one
/// (`008649a0(buffer)`).
const SAVE_START_SIZED: u32 = 0x0086_5f20;
const SAVE_END_SIZED: u32 = 0x0086_5ff0;
const SAVE_STRING: u32 = 0x0086_5e70;
const LOAD_START_SIZED: u32 = 0x0086_4a60;
const LOAD_STRING: u32 = 0x0086_49a0;
/// Globals: the data handler (`011c3f2c`), the object with the loading flags
/// (`011ddf38`), the save/load game object (`011de45c`) and the player
/// (`011dea3c`).
const GLOBAL_DATA_HANDLER: u32 = 0x011c_3f2c;
const GLOBAL_LOADING_OBJECT: u32 = 0x011d_df38;
const GLOBAL_SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const GLOBAL_PLAYER: u32 = 0x011d_ea3c;
/// The reference's pointer to its base object (`OBJ_REFR::pObjectReference`
/// inside `data`, +0x20) and its parent cell (+0x40) are read through the
/// layout; `TESChildCell` is the base subobject at +0x18.
const CHILD_CELL_OFFSET: u32 = 0x18;

// ---------------------------------------------------------------------------
// Small helpers.

/// The base form of `this` and its `cFormType`, `None` when it has no base
/// form (the exe asks for the base form twice, as `if (GetBase()) {
/// GetBase()->GetType() }`).
fn base_type(e: &mut Engine, this: u32) -> Option<u32> {
    if e.call(GET_BASE_FORM, &args![this]).u32() == 0 {
        return None;
    }
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    Some(e.call(FORM_TYPE, &args![base]).u32())
}

/// Whether `iFormFlags` has any of `mask`.
fn form_flag_set(e: &mut Engine, this: u32, mask: u32) -> bool {
    e.call(FORM_GET_FLAGS, &args![this]).u32() & mask != 0
}

/// Sets or clears `mask` in `iFormFlags`: read (`0044ddc0`), change, write
/// (`00403550`).
fn update_form_flags(e: &mut Engine, this: u32, mask: u32, set: bool) {
    let flags = e.call(FORM_GET_FLAGS, &args![this]).u32();
    let flags = if set { flags | mask } else { flags & !mask };
    e.call(FORM_SET_FLAGS, &args![this, flags]);
}

/// The C string of a sequence's name (`0043b1b0(00413f40(sequence))`).
fn sequence_name(e: &mut Engine, sequence: u32) -> u32 {
    let holder = e.call(SEQUENCE_NAME_HOLDER, &args![sequence]).u32();
    e.call(NAME_TEXT, &args![holder]).u32()
}

/// The sequence of `manager` whose name is the C string the pointer at
/// `name_global` points to: builds the `NiFixedString`, looks it up and
/// destroys it.
fn find_sequence(e: &mut Engine, manager: u32, name_global: u32) -> u32 {
    let name = e.global::<u32>(name_global);
    e.with_stack(4, |e, fixed| {
        let handle = e.call(FIXED_STRING_CONSTRUCT, &args![fixed, name]).u32();
        let sequence = e.call(MANAGER_FIND_SEQUENCE, &args![manager, handle]).u32();
        e.call(FIXED_STRING_DESTRUCT, &args![fixed]);
        sequence
    })
}

/// Gives the 3D object `node` a velocity record built from `value` and the
/// two flag words: `0043d410(record, value, a, b)`, then `00a59c60`.
fn set_velocity(e: &mut Engine, node: u32, value: f32, a: u32, b: u32) {
    e.with_stack(0xc, |e, record| {
        e.call(MAKE_VELOCITY, &args![record, value, a, b]);
        e.call(SET_3D_VELOCITY, &args![node, record]);
    });
}

/// `blend == -1.0` (the `double` at `0101a6b0`) stands for the default blend
/// time (`float` at `011c3c08`), as the three save/load helpers start.
fn resolve_blend(e: &mut Engine, blend: f32) -> f32 {
    let marker: f64 = e.global(MINUS_ONE_DOUBLE);
    if f64::from(blend) == marker {
        e.global(DEFAULT_BLEND_TIME)
    } else {
        blend
    }
}

/// Activates `sequence` on `manager` with the arguments every caller here
/// uses: `(sequence, 0, 0, 1.0, 0.0, 0)`.
fn activate_sequence(e: &mut Engine, manager: u32, sequence: u32) {
    e.call(
        MANAGER_ACTIVATE,
        &args![manager, sequence, 0u32, 0u32, 1.0f32, 0.0f32, 0u32],
    );
}

// Translated from 00563d80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Restarts the sequence of `manager` whose name is `name` and plays it
/// from `time`: returns false when the reference has no loaded 3D or no
/// manager is given, or no sequence has that name. A generic-location
/// sequence is not activated again. The sequence's offset is set from where
/// `time` falls against its offset time (`00759450`) and the other limit
/// (`00508100`), the 3D object's simulation is reset, the 3D gets a velocity
/// record of the first value, the sequence is ended (`005672c0`) and the
/// manager switched off. Not named in the Xbox PDB.
pub fn fn_00563d80(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    manager: Ptr,
    name: Ptr,
    time: f32,
) -> bool {
    let manager = manager.addr();
    let node = e.call(GET_LOADED_3D, &args![this]).u32();
    if manager == 0 || node == 0 {
        return false;
    }
    let count = e.call(MANAGER_SEQUENCE_COUNT, &args![manager]).u32();
    for index in 0..count {
        let sequence = e.call(MANAGER_SEQUENCE_AT, &args![manager, index]).u32();
        if sequence == 0 {
            continue;
        }
        let text = sequence_name(e, sequence);
        if e.call(STRCMP, &args![text, name]).i32() != 0 {
            continue;
        }
        e.call(MANAGER_DEACTIVATE_ALL, &args![manager, 0.0f32]);
        e.call(MANAGER_SET_FLAG, &args![manager, 1u32]);
        if e.call(SEQUENCE_IS_GENERIC, &args![sequence]).u32() == 0 {
            activate_sequence(e, manager, sequence);
        }
        let mut record_time = time;
        let mut offset = time;
        let offset_time = e.call(SEQUENCE_OFFSET_TIME, &args![sequence]).f32();
        if time < offset_time {
            record_time = e.call(SEQUENCE_OFFSET_TIME, &args![sequence]).f32();
            let most: f32 = e.global(FLOAT_MAX);
            offset = -most;
        } else {
            let limit = e.call(SEQUENCE_LIMIT_TIME, &args![sequence]).f32();
            if limit < time {
                record_time = e.call(SEQUENCE_LIMIT_TIME, &args![sequence]).f32();
                offset = e.call(SEQUENCE_LIMIT_TIME, &args![sequence]).f32();
            }
        }
        e.call(SEQUENCE_SET_OFFSET, &args![sequence, offset]);
        e.call(RESET_SIM, &args![node, 1u32]);
        set_velocity(e, node, record_time, 1, 0);
        tes_object_refr_end_sequence(e, this, Ptr::new(sequence));
        e.call(MANAGER_FINISH_SEQUENCE, &args![manager, sequence, 0.0f32]);
        e.call(MANAGER_SET_FLAG, &args![manager, 0u32]);
        return true;
    }
    false
}

// Translated from 00563f30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::SaveControllerManager` (Xbox PDB), a cdecl function of
/// three words (the reference is not used): writes the sequences of
/// `manager` that `008041a0` accepts into the save buffer as a variable
/// sized value holding their count, each as its name (`SaveString`) and its
/// own data (`BSAnimGroupSequence::SaveGame`, `004efc90(buffer, blend)`).
/// A `blend` of `-1.0` stands for the default at `011c3c08`.
pub fn tes_object_refr_save_controller_manager(
    e: &mut Engine,
    manager: Ptr,
    buffer: Ptr,
    blend: f32,
) {
    let manager = manager.addr();
    let buffer = buffer.addr();
    let blend = resolve_blend(e, blend);
    let mut saved = 0u32;
    let start = e.call(SAVE_START_SIZED, &args![buffer]).u32();
    if manager != 0 && e.call(MANAGER_HAS_SEQUENCES, &args![manager]).bool() {
        let mut index = 0u32;
        while index < e.call(MANAGER_SEQUENCE_COUNT, &args![manager]).u32() {
            let sequence = e.call(MANAGER_SEQUENCE_AT, &args![manager, index]).u32();
            if sequence != 0 && e.call(SEQUENCE_IS_GENERIC, &args![sequence]).u32() != 0 {
                let name = sequence_name(e, sequence);
                e.call(SAVE_STRING, &args![buffer, name, 0u32]);
                e.call(SEQUENCE_SAVE_TO_BUFFER, &args![sequence, buffer, blend]);
                saved += 1;
            }
            index += 1;
        }
    }
    e.call(SAVE_END_SIZED, &args![buffer, saved, start]);
}

// Translated from 00564010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Loads what [`tes_object_refr_save_controller_manager`] saved (cdecl,
/// `(manager, buffer, blend)`; the C++ exception state is not translated).
/// With a manager all sequences are deactivated and, when the buffer holds
/// entries, the manager is switched on. For each saved entry the name is
/// read; the sequence of that name is found in the manager, activated if it
/// is not a generic-location one, loaded with `blend`, and, when the buffer's
/// owner (virtual `+8`) has a loaded 3D that `004b5c80` accepts, the 3D gets
/// a run of velocity records stepping through the sequence's duration. An
/// entry without a sequence is loaded into a temporary `BSAnimGroupSequence`
/// (to consume its data). When any entry was found the 3D object's simulation
/// is reset and it gets one record of `blend`. Not named in the Xbox PDB.
pub fn fn_00564010(e: &mut Engine, manager: Ptr, buffer: Ptr, blend: f32) {
    let manager = manager.addr();
    let buffer = buffer.addr();
    let blend = resolve_blend(e, blend);
    let count = e.call(LOAD_START_SIZED, &args![buffer]).u32();
    if manager != 0 {
        e.call(MANAGER_DEACTIVATE_ALL, &args![manager, 0.0f32]);
        if count != 0 {
            e.call(MANAGER_SET_FLAG, &args![manager, 1u32]);
        }
    }
    let mut any = false;
    let owner = e.vcall(buffer, 8, &args![]).u32();
    let node = if owner != 0 {
        e.call(GET_LOADED_3D, &args![owner]).u32()
    } else {
        0
    };
    e.with_stack(0x104, |e, text| {
        for _ in 0..count {
            e.call(LOAD_STRING, &args![buffer, text]);
            let mut found = false;
            if manager != 0 {
                let sequences = e.call(MANAGER_SEQUENCE_COUNT, &args![manager]).u32();
                for position in 0..sequences {
                    let sequence = e.call(MANAGER_SEQUENCE_AT, &args![manager, position]).u32();
                    if sequence == 0 {
                        continue;
                    }
                    let name = sequence_name(e, sequence);
                    if e.call(STRING_COMPARE, &args![name, text]).u32() != 0 {
                        continue;
                    }
                    if e.call(SEQUENCE_IS_GENERIC, &args![sequence]).u32() == 0 {
                        activate_sequence(e, manager, sequence);
                    }
                    e.call(SEQUENCE_LOAD_FROM_BUFFER, &args![sequence, buffer, blend]);
                    if node != 0 && e.call(IS_ACTOR_3D, &args![node]).bool() {
                        add_velocity_run(e, node, sequence, blend);
                    }
                    found = true;
                    any = true;
                    break;
                }
            }
            if !found {
                e.with_stack(SEQUENCE_OBJECT_SIZE, |e, temporary| {
                    e.call(SEQUENCE_CONSTRUCT, &args![temporary]);
                    e.call(SEQUENCE_LOAD_FROM_BUFFER, &args![temporary, buffer, blend]);
                    e.call(SEQUENCE_DESTRUCT, &args![temporary]);
                });
            }
        }
    });
    if any && node != 0 {
        e.call(RESET_SIM, &args![node, 1u32]);
        set_velocity(e, node, blend, 1, 0);
    }
}

/// The run of velocity records `fn_00564010` gives an actor's 3D after
/// loading `sequence`: from `max(blend - total, 0)` to `blend` in steps of
/// `total / 20` (at least `0.0166666`), where `total` is the sequence's
/// duration plus `blend`; every record is added with `00a59c90`.
fn add_velocity_run(e: &mut Engine, node: u32, sequence: u32, blend: f32) {
    let duration = e.call(SEQUENCE_DURATION, &args![sequence]).f64();
    let total = (duration + f64::from(blend)) as f32;
    let mut start = (f64::from(blend) - f64::from(total)) as f32;
    let zero: f64 = e.global(ZERO_DOUBLE);
    if f64::from(start) < zero {
        start = 0.0;
    }
    let divisor: f64 = e.global(STEP_DIVISOR);
    let mut step = (f64::from(total) / divisor) as f32;
    let smallest: f64 = e.global(MIN_STEP_DOUBLE);
    if f64::from(step) < smallest {
        step = e.global(MIN_STEP_FLOAT);
    }
    e.with_stack(0xc, |e, record| {
        let mut time = start;
        while time < blend {
            e.call(MAKE_VELOCITY, &args![record, time, 0u32, 0u32]);
            e.call(ADD_3D_VELOCITY, &args![node, record]);
            time = (f64::from(time) + f64::from(step)) as f32;
        }
    });
}

// Translated from 00564360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Classifies the base form of the reference that owns the 3D object `node`
/// (`0056f930` finds the reference): 0 when there is none; otherwise by the
/// base form's type 1 (types 0x15, 0x1c, 0x20, 0x21), 2 (0x1b, 0x27), 3
/// (0x2a, 0x2b), 4 (0x18, 0x1a, 0x28, 0x29), 5 (0x1d, 0x1f, 0x2e, 0x32,
/// 0x67, 0x6c, 0x73, 0x74) or 6 (every other type). cdecl, one word. Not
/// named in the Xbox PDB.
pub fn fn_00564360(e: &mut Engine, node: Ptr) -> u32 {
    let node = node.addr();
    let mut class = 0;
    if node != 0 {
        let refr = e.call(0x0056_f930, &args![node]).u32();
        if refr != 0 && e.call(GET_BASE_FORM, &args![refr]).u32() != 0 {
            let base = e.call(GET_BASE_FORM, &args![refr]).u32();
            class = match e.call(FORM_TYPE, &args![base]).u32() {
                0x15 | 0x1c | 0x20 | 0x21 => 1,
                0x1b | 0x27 => 2,
                0x2a | 0x2b => 3,
                0x18 | 0x1a | 0x28 | 0x29 => 4,
                0x1d | 0x1f | 0x2e | 0x32 | 0x67 | 0x6c | 0x73 | 0x74 => 5,
                _ => 6,
            };
        }
    }
    class
}

// Translated from 00564480 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::CreateReference` (Xbox PDB), cdecl `(kind, flag)`: allocates
/// and constructs the reference object for the reference kind `kind` (the
/// values `fn_00564820` returns), inside an allocation scope guard (tag
/// 0x31). Kind 0x3a is a plain `TESObjectREFR` (0x68 bytes), 0x16 an object
/// of 0x88 bytes; kinds 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40 and 0x69 are
/// objects of 0x1c8, 0x1c0, 0x160, 0x154, 0x154, 0x158 and 0x158 bytes whose
/// constructors take `flag`. Any other kind gives null.
pub fn tes_object_refr_create_reference(e: &mut Engine, kind: u8, flag: u8) -> Ptr {
    let flag = u32::from(flag);
    e.with_stack(SCOPE_GUARD_SIZE, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, 0x31u32, 1u32, SOURCE_FILE, 0x1493u32],
        );
        // (size, constructor, whether the constructor takes `flag`)
        let spec: Option<(u32, u32, bool)> = match kind {
            0x16 => Some((0x88, 0x0092_eb50, false)),
            0x3a => Some((0x68, 0x0055_a2f0, false)),
            0x3b => Some((0x1c8, 0x008d_1f40, true)),
            0x3c => Some((0x1c0, 0x008d_4500, true)),
            0x3d => Some((0x160, 0x009b_7a50, true)),
            0x3e => Some((0x154, 0x009b_3fa0, true)),
            0x3f => Some((0x154, 0x0097_8fd0, true)),
            0x40 => Some((0x158, 0x009b_3160, true)),
            0x69 => Some((0x158, 0x009a_aff0, true)),
            _ => None,
        };
        let mut created = 0;
        if let Some((size, constructor, takes_flag)) = spec {
            let block = e.call(OPERATOR_NEW, &args![size]).u32();
            if block != 0 {
                created = if takes_flag {
                    e.call(constructor, &args![block, flag]).u32()
                } else {
                    e.call(constructor, &args![block]).u32()
                };
            }
        }
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
        Ptr::new(created)
    })
}

// Translated from 00564820 (decompiled, FalloutNV.exe 1.4.0.525)
/// The reference kind to create for `this`'s base form (the argument of
/// [`tes_object_refr_create_reference`]): 0x3a without a base form or for
/// most types, 0x3b for type 0x2a, 0x3c for 0x2b, and for type 0x33 by the
/// base form's `004fd4f0` value: 0x10000 gives 0x3d, 0x20000 0x3e, 0x40000
/// 0x3f, 0x80000 0x40, 0x100000 0x69. Not named in the Xbox PDB.
pub fn fn_00564820(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    let me = this.addr();
    let mut kind = 0x3a;
    if e.call(GET_BASE_FORM, &args![me]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![me]).u32();
        match e.call(FORM_TYPE, &args![base]).u32() {
            0x2a => kind = 0x3b,
            0x2b => kind = 0x3c,
            0x33 => {
                let base = e.call(GET_BASE_FORM, &args![me]).u32();
                match e.call(0x004f_d4f0, &args![base]).i32() {
                    0x10000 => kind = 0x3d,
                    0x20000 => kind = 0x3e,
                    0x40000 => kind = 0x3f,
                    0x80000 => kind = 0x40,
                    0x100000 => kind = 0x69,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    kind
}

// Translated from 00564900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `kind` (a value of [`fn_00564820`]) is one of the kinds
/// 0x3a to 0x40 or 0x69. cdecl, one word. Not named in the Xbox PDB.
pub fn fn_00564900(_e: &mut Engine, kind: i32) -> bool {
    (0x3a..=0x40).contains(&kind) || kind == 0x69
}

// Translated from 00564930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::SetDelete` (Xbox PDB): when `deleted` is set, the data
/// handler is not in its state `004226e0` and knows this form's ID
/// (`00469860`), the form does not have flag 0x4000 (`004077c0`) and the
/// loading object (`011ddf38`) does not report `0042ce10`, the loading
/// object is told about this reference (`0084a7c0`). Then `00484530(this,
/// deleted)` does the form-level work.
pub fn tes_object_refr_set_delete(e: &mut Engine, this: Ptr<TESObjectREFR>, deleted: u8) {
    let handler = e.global::<u32>(GLOBAL_DATA_HANDLER);
    let loading = e.global::<u32>(GLOBAL_LOADING_OBJECT);
    if deleted != 0 && !e.call(0x0042_26e0, &args![handler]).bool() {
        let id = e.call(FORM_ID, &args![this]).u32();
        if e.call(0x0046_9860, &args![handler, id]).bool()
            && !e.call(0x0040_77c0, &args![this]).bool()
            && !e.call(0x0042_ce10, &args![loading]).bool()
        {
            e.call(0x0084_a7c0, &args![loading, this]);
        }
    }
    e.call(0x0048_4530, &args![this, u32::from(deleted)]);
}

// Translated from 005649b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the form flag 0x200 is set, for a reference whose base form is
/// absent or of type 0x1e; false for any other base form type. Not named in
/// the Xbox PDB.
pub fn fn_005649b0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    if let Some(kind) = base_type(e, this.addr()) {
        if kind != 0x1e {
            return false;
        }
    }
    form_flag_set(e, this.addr(), 0x200)
}

// Translated from 00564a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears the form flag 0x200 (persistent) on a reference whose base
/// form is absent or of type 0x1e, and keeps the object that
/// `005725f0` returns in step: with a non-null result, clearing the flag
/// first calls `00b4f9a0`, then `00b5c450(handle, flag)` is called on the
/// object `00450b80(0)` returns, where the handle is the first dword of
/// `005725f0`'s result. The flags are stored last (`00403550`). Not named in
/// the Xbox PDB.
pub fn fn_00564a00(e: &mut Engine, this: Ptr<TESObjectREFR>, persist: u8) {
    let me = this.addr();
    if let Some(kind) = base_type(e, me) {
        if kind != 0x1e {
            return;
        }
    }
    let mut flags = e.call(FORM_GET_FLAGS, &args![me]).u32();
    if persist != 0 {
        flags |= 0x200;
    } else {
        flags &= !0x200;
    }
    let tracked = e.call(0x0057_25f0, &args![me]).u32();
    if tracked != 0 {
        if persist == 0 {
            e.call(0x00b4_f9a0, &args![]);
        }
        let handle = e.call(0x0055_9450, &args![tracked]).u32();
        let manager = e.call(0x0045_0b80, &args![0u32]).u32();
        e.call(0x00b5_c450, &args![manager, handle, u32::from(persist)]);
    }
    e.call(FORM_SET_FLAGS, &args![me, flags]);
}

// Translated from 00564ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the form flag 0x200 is set, for a reference whose base form is
/// absent or of type 0x22; false for any other base form type. Not named in
/// the Xbox PDB.
pub fn fn_00564ab0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    if let Some(kind) = base_type(e, this.addr()) {
        if kind != 0x22 {
            return false;
        }
    }
    form_flag_set(e, this.addr(), 0x200)
}

// Translated from 00564b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears the form flag 0x200 on a reference whose base form is
/// absent or of type 0x22; does nothing for any other base form type. Not
/// named in the Xbox PDB.
pub fn fn_00564b00(e: &mut Engine, this: Ptr<TESObjectREFR>, set: u8) {
    if let Some(kind) = base_type(e, this.addr()) {
        if kind != 0x22 {
            return;
        }
    }
    update_form_flags(e, this.addr(), 0x200, set != 0);
}

// Translated from 00564b70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The base form's virtual at `+0xa8` (no stack words), false without a base
/// form. Not named in the Xbox PDB.
pub fn fn_00564b70(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        return e.vcall(base, 0xa8, &args![]).bool();
    }
    false
}

// Translated from 00564bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The base form's virtual at `+0xb0` (no stack words), false without a base
/// form. Not named in the Xbox PDB.
pub fn fn_00564bc0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        return e.vcall(base, 0xb0, &args![]).bool();
    }
    false
}

// Translated from 00564c10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the form flag 0x100 is set; false without a base form and for a
/// base form of type 0x1c. Not named in the Xbox PDB.
pub fn fn_00564c10(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    match base_type(e, this.addr()) {
        None | Some(0x1c) => false,
        Some(_) => form_flag_set(e, this.addr(), 0x100),
    }
}

// Translated from 00564c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears the form flag 0x100 on a reference that has a base form of
/// a type other than 0x1c. Not named in the Xbox PDB.
pub fn fn_00564c60(e: &mut Engine, this: Ptr<TESObjectREFR>, set: u8) {
    if let Some(kind) = base_type(e, this.addr()) {
        if kind != 0x1c {
            update_form_flags(e, this.addr(), 0x100, set != 0);
        }
    }
}

// Translated from 00564cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the form flag 0x100000 is set; false for an actor (virtual
/// `+0x100`, `IsActor`). Not named in the Xbox PDB.
pub fn fn_00564cd0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    if e.vcall(this.addr(), 0x100, &args![]).bool() {
        return false;
    }
    form_flag_set(e, this.addr(), 0x10_0000)
}

// Translated from 00564d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears the form flag 0x100000 on a reference that is not an actor
/// (virtual `+0x100`). Not named in the Xbox PDB.
pub fn fn_00564d20(e: &mut Engine, this: Ptr<TESObjectREFR>, set: u8) {
    if !e.vcall(this.addr(), 0x100, &args![]).bool() {
        update_form_flags(e, this.addr(), 0x10_0000, set != 0);
    }
}

// Translated from 00564d80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the form flag 0x40000 (targeted) is set. Not named in the Xbox
/// PDB.
pub fn fn_00564d80(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    form_flag_set(e, this.addr(), 0x4_0000)
}

// Translated from 00564db0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::SetTargeted` (Xbox PDB): sets or clears the form flag
/// 0x40000.
pub fn tes_object_refr_set_targeted(e: &mut Engine, this: Ptr<TESObjectREFR>, targeted: u8) {
    update_form_flags(e, this.addr(), 0x4_0000, targeted != 0);
}

// Translated from 00565050 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 4 is set in the `iFormID`-style dword (`0084e3a0`) of the
/// object `00527080(this, 0x92)` returns; false when that is null. Not named
/// in the Xbox PDB.
pub fn fn_00565050(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    let object = e.call(0x0052_7080, &args![this, 0x92u32]).u32();
    object != 0 && e.call(FORM_ID, &args![object]).u32() & 4 != 0
}

// Translated from 00565260 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::MustRefPersist` (Xbox PDB): whether the reference has to
/// stay persistent because of its base form. False without a base form. True
/// for the base forms with the form IDs 4, 5, 6, 0x10, 0x12, 0x15, 0x1f,
/// 0x23, 0x24, 0x34 and 0x3b, and by the base form's type: 4 and 0x23
/// always; 0x16 when `004fede0(base, 0)` holds; 0x1c when `005194d0(base)`
/// or `00568e50(this)` holds; 0x2a and 0x2b when `0055e200(base + 0x30)`
/// holds.
pub fn tes_object_refr_must_ref_persist(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    let me = this.addr();
    let base = e.call(GET_BASE_FORM, &args![me]).u32();
    if base == 0 {
        return false;
    }
    let id = e.call(FORM_ID, &args![base]).u32();
    if matches!(
        id,
        4 | 5 | 6 | 0x10 | 0x12 | 0x15 | 0x1f | 0x23 | 0x24 | 0x34 | 0x3b
    ) {
        return true;
    }
    match e.call(FORM_TYPE, &args![base]).u32() {
        4 | 0x23 => true,
        0x16 => e.call(0x004f_ede0, &args![base, 0u32]).bool(),
        0x1c => {
            e.call(0x0051_94d0, &args![base]).bool() || e.call(0x0056_8e50, &args![me]).u32() != 0
        }
        0x2a | 0x2b => e.call(0x0055_e200, &args![base + 0x30]).bool(),
        _ => false,
    }
}

// Translated from 005653d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetRefPersists` (Xbox PDB): true when `00484e60(this,
/// -1)` finds an object and neither the data handler (`004516b0`,
/// `004226e0`), the save/load game object (`0047c850`) nor that object
/// (`00471c20`) objects; otherwise whether the form flag 0x400 is set
/// ([`fn_00565450`]).
pub fn tes_object_refr_get_ref_persists(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    let found = e.call(0x0048_4e60, &args![this, u32::MAX]).u32();
    if found != 0 {
        let handler = e.global::<u32>(GLOBAL_DATA_HANDLER);
        let save_load = e.global::<u32>(GLOBAL_SAVE_LOAD_GAME);
        if !e.call(0x0045_16b0, &args![handler]).bool()
            && !e.call(0x0047_c850, &args![save_load]).bool()
            && !e.call(0x0042_26e0, &args![handler]).bool()
            && !e.call(0x0047_1c20, &args![found]).bool()
        {
            return true;
        }
    }
    fn_00565450(e, this)
}

// Translated from 00565450 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the form flag 0x400 (persistent reference) is set. Not named in
/// the Xbox PDB.
pub fn fn_00565450(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    form_flag_set(e, this.addr(), 0x400)
}

// Translated from 00565480 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::SetRefPersists` (Xbox PDB): sets or clears the form flag
/// 0x400. Setting it also calls the virtual `+0x48` with 1 and, when the
/// parent cell (the `TESChildCell` base's virtual 0) exists and neither
/// `00425fd0` nor `005516c0` holds for it, adds the reference to the cell's
/// persistent list (`00587ff0` on `0054ddd0(cell)`). Clearing it, when the
/// cell exists and `005516c0` holds, removes the reference from that list
/// (`00588030`) and calls the virtual `+0xc8` with 1.
pub fn tes_object_refr_set_ref_persists(e: &mut Engine, this: Ptr<TESObjectREFR>, persists: u8) {
    let me = this.addr();
    let cell = e.vcall(me + CHILD_CELL_OFFSET, 0, &args![]).u32();
    update_form_flags(e, me, 0x400, persists != 0);
    if persists != 0 {
        e.vcall(me, 0x48, &args![1u32]);
        if cell != 0
            && !e.call(0x0042_5fd0, &args![cell]).bool()
            && !e.call(0x0055_16c0, &args![cell]).bool()
        {
            let list = e.call(0x0054_ddd0, &args![cell]).u32();
            e.call(0x0058_7ff0, &args![list, me]);
        }
    } else if cell != 0 && e.call(0x0055_16c0, &args![cell]).bool() {
        let list = e.call(0x0054_ddd0, &args![cell]).u32();
        e.call(0x0058_8030, &args![list, me]);
        e.vcall(me, 0xc8, &args![1u32]);
    }
}

// Translated from 00565580 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::HasTimeControllers` (Xbox PDB), cdecl on a 3D object:
/// whether `node` or any of its descendants has a controller (`0043b230`).
/// The node's own list (`00430830`, walked with `0057cbe0`) is searched
/// first, then its children (the virtual `+0xc`, `0043b480` counts and
/// `0043b4a0` fetches them) recursively.
pub fn tes_object_refr_has_time_controllers(e: &mut Engine, node: Ptr) -> bool {
    let node = node.addr();
    let mut found = false;
    if node == 0 {
        return false;
    }
    if e.call(GET_CONTROLLER, &args![node]).u32() != 0 {
        found = true;
    }
    if !found {
        let list = e.call(0x0043_0830, &args![node]).u32();
        let first = e.call(0x0055_9450, &args![list]).u32();
        e.with_stack(4, |e, position| {
            e.mem.set_u32(position.addr(), first);
            while e.mem.u32(position.addr()) != 0 && !found {
                let list = e.call(0x0043_0830, &args![node]).u32();
                let slot = e.call(0x0057_cbe0, &args![list, position]).u32();
                let item = e.call(0x0055_9450, &args![slot]).u32();
                if item != 0 {
                    found = e.call(GET_CONTROLLER, &args![item]).u32() != 0;
                }
            }
        });
    }
    if !found {
        let children = e.vcall(node, 0xc, &args![]).u32();
        if children != 0 {
            let count = e.call(0x0043_b480, &args![children]).u32();
            for index in 0..count {
                let child = e.call(0x0043_b4a0, &args![children, index]).u32();
                if tes_object_refr_has_time_controllers(e, Ptr::new(child)) {
                    return true;
                }
            }
        }
    }
    found
}

// Translated from 00565670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes `key` from the map object at `011ca0e0` (`00405430`, a map
/// remove) and returns whether it was there. cdecl, one word. Not named in
/// the Xbox PDB.
pub fn fn_00565670(e: &mut Engine, key: u32) -> bool {
    e.call(0x0040_5430, &args![0x011c_a0e0u32, key]).bool()
}

// Translated from 00565690 (decompiled, FalloutNV.exe 1.4.0.525)
/// The base form's virtual at `+0x94` (no stack words), false without a base
/// form. Not named in the Xbox PDB.
pub fn fn_00565690(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    if e.call(GET_BASE_FORM, &args![this]).u32() != 0 {
        let base = e.call(GET_BASE_FORM, &args![this]).u32();
        return e.vcall(base, 0x94, &args![]).bool();
    }
    false
}

// Translated from 005656d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `004826d0` accepts the base form, or when the reference has a
/// container (`0055d310`) whose inventory changes (`004bf220`, cdecl) report
/// `004d0490`. Not named in the Xbox PDB.
pub fn fn_005656d0(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    if e.call(0x0048_26d0, &args![base]).u32() != 0 {
        return true;
    }
    if e.call(0x0055_d310, &args![this]).u32() != 0 {
        let changes = e.call(0x004b_f220, &args![this]).u32();
        return e.call(0x004d_0490, &args![changes]).bool();
    }
    false
}

// Translated from 00565730 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::InitScript` (Xbox PDB): for a reference that is not
/// deleted, inside an allocation scope guard (tag 0x15), and only when it has
/// no script extra data (type 0xd with a non-null +0xc), looks for the script
/// of the reference (virtual `+0x1a4`, else its base form), turns it into the
/// form `004826d0` returns and, when there is one, stores it and the
/// value `005abf60` derives from it in the reference's extra data
/// (`00419ed0`, `00419f80`). A reference with a container also has its
/// inventory changes (`004bf220`) initialised (`004d1960`).
pub fn tes_object_refr_init_script(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    if e.call(FORM_IS_DELETED, &args![me]).bool() {
        return;
    }
    e.with_stack(SCOPE_GUARD_SIZE, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, 0x15u32, 1u32, SOURCE_FILE, 0x196eu32],
        );
        let list = e.call(GET_EXTRA_LIST, &args![me]).u32();
        let script_extra = e.call(0x0041_0220, &args![list, 0xdu32]).u32();
        if script_extra != 0 && e.mem.u32(script_extra + 0xc) != 0 {
            e.call(SCOPE_GUARD_DTOR, &args![guard]);
            return;
        }
        let mut script = e.vcall(me, 0x1a4, &args![]).u32();
        if script == 0 {
            script = e.call(GET_BASE_FORM, &args![me]).u32();
        }
        let form = e.call(0x0048_26d0, &args![script]).u32();
        if form != 0 {
            let list = e.call(GET_EXTRA_LIST, &args![me]).u32();
            e.call(0x0041_9ed0, &args![list, form]);
            let derived = e.call(0x005a_bf60, &args![form]).u32();
            let list = e.call(GET_EXTRA_LIST, &args![me]).u32();
            e.call(0x0041_9f80, &args![list, derived]);
        }
        if e.call(0x0055_d310, &args![me]).u32() != 0 {
            let changes = e.call(0x004b_f220, &args![me]).u32();
            e.call(0x004d_1960, &args![changes]);
        }
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
    });
}

// Translated from 00565870 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::RunScript` (Xbox PDB): runs the scripts of the reference
/// and returns whether any ran. Nothing happens for a deleted reference or
/// one without a base form. For the base form types 0x2a and 0x2b a
/// reference whose process (`008d8520`) exists, is not `00931850` and has no
/// loaded 3D is skipped entirely (false). A reference with a container runs
/// the scripts of its inventory changes (`004d2480`). Then the reference's
/// own script (`00418800` on the extra list) runs with its variables
/// (`00418830`) through `005ac1e0`; when it did not run and there are
/// variables, the virtual `+0x48` is called with `0x80000000`. A "say to"
/// extra (`0042ede0`) on a reference that is not an actor-like (virtual
/// `+0xfc`) calls the virtual `+0x140`.
pub fn tes_object_refr_run_script(e: &mut Engine, this: Ptr<TESObjectREFR>) -> bool {
    let me = this.addr();
    let mut ran = false;
    if e.call(FORM_IS_DELETED, &args![me]).bool() || e.call(GET_BASE_FORM, &args![me]).u32() == 0 {
        return ran;
    }
    let base = e.call(GET_BASE_FORM, &args![me]).u32();
    let kind = e.call(FORM_TYPE, &args![base]).u32();
    let actor_like = (0x2a..=0x2b).contains(&kind);
    if actor_like
        && e.call(0x008d_8520, &args![me]).u32() != 0
        && e.call(0x0093_1850, &args![me]).u32() == 0
        && e.vcall(me, 0x1d0, &args![]).u32() == 0
    {
        return false;
    }
    if (kind == 0x1b || actor_like) && e.call(0x0055_d310, &args![me]).u32() != 0 {
        let changes = e.call(0x004b_f220, &args![me]).u32();
        if changes != 0 && e.call(0x004d_2480, &args![changes, me]).bool() {
            ran = true;
        }
    }
    let list = e.call(GET_EXTRA_LIST, &args![me]).u32();
    let script = e.call(0x0041_8800, &args![list]).u32();
    if script != 0 {
        let list = e.call(GET_EXTRA_LIST, &args![me]).u32();
        let variables = e.call(0x0041_8830, &args![list]).u32();
        if e.call(0x005a_c1e0, &args![script, me, variables, 0u32, 0u32])
            .bool()
        {
            ran = true;
        } else if variables != 0 {
            e.vcall(me, 0x48, &args![0x8000_0000u32]);
        }
        let list = e.call(GET_EXTRA_LIST, &args![me]).u32();
        let say_to = e.call(0x0042_ede0, &args![list]).u32();
        if say_to != 0 && !e.vcall(me, 0xfc, &args![]).bool() {
            e.vcall(me, 0x140, &args![]);
        }
    }
    ran
}

// ---------------------------------------------------------------------------
// InitAnimation and what it uses.

/// `TESObjectREFR::SetAnimation` (Xbox PDB, `00572e50(this, animation)`):
/// installs the animation object (0 clears it) and returns the current one.
const SET_ANIMATION: u32 = 0x0057_2e50;
/// `Animation::Update` (`00491180(animation, reference, delta, -1.0f)`).
const ANIMATION_UPDATE: u32 = 0x0049_1180;
/// The `Animation` constructor (`0048f810`, object of 0x13c bytes).
const ANIMATION_CONSTRUCT: u32 = 0x0048_f810;
const ANIMATION_SIZE: u32 = 0x13c;
/// The actor's process object (`008d8520`; the map names it
/// `MiddleHighProcess::GetSavedAcquireObject`, which does not describe it)
/// and its level (`0045cd60`): 0 or 1 for the processes the animation
/// calls below need.
const ACTOR_PROCESS: u32 = 0x008d_8520;
const PROCESS_LEVEL: u32 = 0x0045_cd60;
/// `Actor::IsWeaponDrawn` as `008a16d0` (no stack words), the actor's
/// animation getter `008b70d0` (no stack words) and the player's
/// `GetBiped(flag)` (`00950b00`) and `GetAnimation(flag)` (`00950a60`).
const ACTOR_WEAPON_DRAWN: u32 = 0x008a_16d0;
const ACTOR_ANIMATION: u32 = 0x008b_70d0;
const PLAYER_BIPED: u32 = 0x0095_0b00;
const PLAYER_ANIMATION: u32 = 0x0095_0a60;
/// `MiddleHighProcess::GetForceNextUpdate` (`00566950`), a byte test on the
/// actor's process data.
const FORCE_NEXT_UPDATE: u32 = 0x0056_6950;
/// The strings the animation set-up reports with.
const SKELETON_PREFIX: u32 = 0x0101_3458;
const SKELETON_MESSAGE: u32 = 0x0102_ff48;
const CUMULATIVE_MESSAGE: u32 = 0x0102_fee8;
const NOT_LOOPING_MESSAGE: u32 = 0x0102_feb4;
const MOVED_MESSAGE: u32 = 0x0102_fe74;
const NPC_ANIMATION_MESSAGE: u32 = 0x0102_fdd0;
const CREATURE_ANIMATION_MESSAGE: u32 = 0x0102_fe20;
const LILY_NAME: u32 = 0x0102_fdc8;
/// The table of type names (3 dwords per form type, the name pointer first)
/// at `01187004`.
const TYPE_NAME_TABLE: u32 = 0x0118_7004;
/// The `NiTimeController` record `00a5c570` looks for on the 3D object
/// (`011f36bc`).
const CONTROLLER_RECORD: u32 = 0x011f_36bc;

// Translated from 005659f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::InitAnimation` (Xbox PDB): builds the animation of a
/// reference from its loaded 3D. The C++ exception state and the stack
/// cookie are not translated. Outline:
///
/// - Clears the current animation (`SetAnimation(this, 0)`) and stops for a
///   reference without a 3D node (virtual `+0xc` of the loaded 3D) or a
///   deleted one.
/// - Base form types 0x2a and 0x2b: warns when the model path's file name
///   does not start with "Skeleton", and asks `008a1760` for the actor's
///   skeleton.
/// - Type 0x2a (with a biped, virtual `+0x1e8`) and 0x2b build a new
///   `Animation`, its KF file list ([`tes_object_refr_build_kf_file_list`])
///   and special animations ([`init_character_animation`]); type 0x28 (a
///   light, `004c0bf0`) sets its light. Every other type (and 0x28) sets
///   up the node's controller manager ([`init_controller_manager`]).
/// - Updates the new animation once (`Animation::Update(this, 0, -1.0)`),
///   then for actors ([`init_actor_state`]) refreshes the process's
///   weapon and animation state.
/// - Finally restores a saved animation time (the extra data `00418200`)
///   into the 3D and removes that extra data.
pub fn tes_object_refr_init_animation(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let data = this.at(TESObjectREFR::data);
    let loaded = e.call(GET_LOADED_3D, &args![this]).u32();
    let mut node = if loaded == 0 {
        0
    } else {
        e.vcall(loaded, 0xc, &args![]).u32()
    };
    let biped = e.vcall(me, 0x1e8, &args![]).u32();
    let mut animation = 0u32;
    e.call(SET_ANIMATION, &args![this, animation]);
    if node == 0 || e.call(FORM_IS_DELETED, &args![this]).bool() {
        return;
    }
    let base = e.get(data, OBJ_REFR::pObjectReference).addr();
    let actor = if e.vcall(me, 0x100, &args![]).bool() {
        me
    } else {
        0
    };
    let mut skeleton = 0u32;
    if e.call(FORM_TYPE, &args![base]).u32() == 0x2a
        || e.call(FORM_TYPE, &args![base]).u32() == 0x2b
    {
        let model = e.call(GET_MODEL, &args![this]).u32();
        let slash = e.call(STRRCHR, &args![model, 0x5cu32]).u32();
        if slash != 0
            && e.call(STRNICMP, &args![slash + 1, SKELETON_PREFIX, 8u32])
                .i32()
                != 0
        {
            let name = e.vcall(base, 0x130, &args![]).u32();
            e.call(LOG_MESSAGE, &args![SKELETON_MESSAGE, name, model]);
        }
        skeleton = e.call(0x008a_1760, &args![actor]).u32();
    }
    let kind = e.call(FORM_TYPE, &args![base]).u32();
    match kind {
        0x2a => {
            if biped != 0 {
                match init_character_animation(e, me, base, skeleton, biped, false) {
                    Some((created, created_node)) => {
                        animation = created;
                        node = created_node;
                    }
                    None => return,
                }
            }
        }
        0x2b => match init_character_animation(e, me, base, skeleton, 0, true) {
            Some((created, created_node)) => {
                animation = created;
                node = created_node;
            }
            None => return,
        },
        _ => {
            if kind == 0x28 && e.call(0x004c_0bf0, &args![base]).bool() {
                e.call(0x0057_26e0, &args![this, node, 0u32]);
            }
            init_controller_manager(e, this, base, node);
        }
    }
    if animation != 0 {
        let minus_one: f32 = e.global(MINUS_ONE_FLOAT);
        e.call(ANIMATION_UPDATE, &args![animation, me, 0.0f32, minus_one]);
    }
    if actor != 0 {
        init_actor_state(e, me, actor, biped, animation, node);
    }
    let list = e.call(GET_EXTRA_LIST, &args![me]).u32();
    let saved = e.call(0x0041_8200, &args![list]).u32();
    if saved != 0 && animation != 0 {
        // the saved animation time, a `float` at +0x10 of the extra data
        let time = e.mem.f32(saved + 0x10);
        e.with_stack(0xc, |e, record| {
            e.call(MAKE_VELOCITY, &args![record, time, 0u32, 0u32]);
            let current = e.call(GET_LOADED_3D, &args![me]).u32();
            e.call(SET_3D_VELOCITY, &args![current, record]);
        });
        let list = e.call(GET_EXTRA_LIST, &args![me]).u32();
        e.call(0x0041_ae10, &args![list]);
    }
}

/// The part of `InitAnimation` for base form types 0x2a (`creature` false;
/// `biped` is the actor's biped, whose parts are removed first,
/// `004aae70`) and 0x2b (`creature` true): installs a new `Animation`
/// (allocated with `operator new` of 0x13c bytes and constructed with
/// `0048f810`), re-reads the 3D node (stopping with the "SetAnimation
/// cleared 3D" message when it is gone), builds the KF file list and has
/// the animation load it (`0048ffd0(list, node, this, 1)`, reporting the
/// missing 'Idle' animation when it fails). When the base form's value
/// `00568ad0` is positive and its animation set (embedded at +0xc4) has KF
/// files (`0047fd90`), the model's directory is cut off the path and the
/// special animations are added (`00490330`). A creature's base form is
/// finally given the 3D (`005fbdd0`). Returns the new animation and the new
/// node, `None` when the 3D is gone.
fn init_character_animation(
    e: &mut Engine,
    me: u32,
    base: u32,
    skeleton: u32,
    biped: u32,
    creature: bool,
) -> Option<(u32, u32)> {
    if !creature {
        e.call(0x004a_ae70, &args![biped]);
    }
    let block = e.call(OPERATOR_NEW, &args![ANIMATION_SIZE]).u32();
    let object = if block != 0 {
        e.call(ANIMATION_CONSTRUCT, &args![block]).u32()
    } else {
        0
    };
    let animation = e.call(SET_ANIMATION, &args![me, object]).u32();
    let loaded = e.call(GET_LOADED_3D, &args![me]).u32();
    let node = if loaded == 0 {
        0
    } else {
        e.vcall(loaded, 0xc, &args![]).u32()
    };
    if node == 0 {
        let id = e.call(FORM_ID, &args![base]).u32();
        let name = e.vcall(base, 0x130, &args![]).u32();
        e.call(LOG_MESSAGE, &args![MOVED_MESSAGE, name, id]);
        return None;
    }
    e.call(0x0057_10c0, &args![me]);
    let list = tes_object_refr_build_kf_file_list(e, Ptr::new(me), skeleton);
    if !e
        .call(0x0048_ffd0, &args![animation, list, node, me, 1u32])
        .bool()
    {
        let id = e.call(FORM_ID, &args![base]).u32();
        let name = e.vcall(base, 0x130, &args![]).u32();
        let message = if creature {
            CREATURE_ANIMATION_MESSAGE
        } else {
            NPC_ANIMATION_MESSAGE
        };
        e.call(LOG_MESSAGE, &args![message, name, id]);
    }
    let value = e.call(0x0056_8ad0, &args![me]).f64();
    if value > 0.0 && e.call(0x0047_fd90, &args![base + 0xc4]).bool() {
        let model = e.call(GET_MODEL, &args![me]).u32();
        e.with_stack(0x104, |e, path| {
            e.call(BOUNDED_COPY, &args![path, 0x104u32, model]);
            let slash = e.call(STRRCHR_WRAPPER, &args![path, 0x5cu32]).u32();
            if slash != 0 {
                e.mem.set_u8(slash, 0);
                let package = e.call(0x0071_7e50, &args![base + 0xc4]).u32();
                e.call(0x0049_0330, &args![animation, package, path]);
            }
        });
    }
    if creature {
        e.call(0x005f_bdd0, &args![base, loaded]);
    }
    Some((animation, node))
}

/// Gives the node an empty velocity record through its virtual `+0xa4`
/// (`(record, 0)`), as `InitAnimation` does when the node has no
/// controller manager.
fn clear_node_motion(e: &mut Engine, node: u32) {
    e.with_stack(0xc, |e, record| {
        e.call(MAKE_VELOCITY, &args![record, 0.0f32, 0u32, 0u32]);
        e.vcall(node, 0xa4, &args![record, 0u32]);
    });
}

/// The part of `InitAnimation` for every base form type but 0x2a and 0x2b:
/// finds the node's controller manager (the first controller cast to the
/// manager type) and, without one, clears the node's motion. With one: the
/// form is marked changed (`TESForm::ForceChange(0x10000000)`), a cumulative
/// manager (`00566910`) is reported, an open/close form has its open state
/// restored (`0047aec0`: the state is the default state, or, unless the
/// loading object is busy, the default state changed by the saved change
/// 0x800000), and the idle sequences (the two names kept in the exe's data)
/// are started: with none, and the open state not set, the first sequence
/// is started from its offset time; with them, an idle that is not looping
/// (`0059bb30`) is reported and the others started. The manager is then
/// switched off again when no idle was found.
fn init_controller_manager(e: &mut Engine, this: Ptr<TESObjectREFR>, base: u32, node: u32) {
    let me = this.addr();
    if e.call(GET_CONTROLLER, &args![node]).u32() == 0 {
        clear_node_motion(e, node);
        return;
    }
    let controller = e.call(GET_CONTROLLER, &args![node]).u32();
    let manager = e.call(CHECKED_CAST, &args![MANAGER_TYPE, controller]).u32();
    if manager == 0 {
        clear_node_motion(e, node);
        return;
    }
    e.call(0x0048_4b90, &args![this, 0x1000_0000u32]);
    if fn_00566910(e, Ptr::new(manager)) != 0 {
        let model = e.call(GET_MODEL, &args![this]).u32();
        let kind = e.call(FORM_TYPE, &args![base]).u32();
        let type_name = e.global::<u32>(TYPE_NAME_TABLE.wrapping_add(kind.wrapping_mul(12)));
        e.call(LOG_MESSAGE, &args![CUMULATIVE_MESSAGE, type_name, model]);
    }
    let mut open_state_set = false;
    let base_form = e.call(GET_BASE_FORM, &args![this]).u32();
    if e.call(0x0047_a490, &args![base_form]).bool() {
        let mut open = e.call(0x0057_2d30, &args![this, 8u32]).u8();
        let loading = e.global::<u32>(GLOBAL_LOADING_OBJECT);
        if !e.call(0x0042_ce10, &args![loading]).bool() {
            let default_open = e.call(0x0056_1d90, &args![this]).u8();
            let changed = e.with_stack(4, |e, flags| {
                e.call(0x008c_71b0, &args![flags, 0x80_0000u32]);
                let flags = e.mem.u32(flags.addr());
                e.call(0x0084_a6d0, &args![loading, me, flags]).bool()
            });
            open = u8::from(u32::from(default_open) != u32::from(changed));
        }
        open_state_set = e.call(0x0047_aec0, &args![me, u32::from(open), 1u32]).u8() != 0;
    }
    let idle_a = find_sequence(e, manager, NAME_IDLE_A);
    let idle_b = find_sequence(e, manager, NAME_IDLE_B);
    let most: f32 = e.global(FLOAT_MAX);
    if idle_a == 0 && idle_b == 0 {
        if !open_state_set {
            e.call(MANAGER_SET_FLAG, &args![manager, 1u32]);
            e.call(MANAGER_DEACTIVATE_ALL, &args![manager, 0.0f32]);
            let first = e.call(MANAGER_SEQUENCE_AT, &args![manager, 0u32]).u32();
            activate_sequence(e, manager, first);
            e.call(SEQUENCE_SET_OFFSET, &args![first, -most]);
            let time = e.call(SEQUENCE_OFFSET_TIME, &args![first]).f32();
            set_velocity(e, node, time, 1, 0);
        }
        e.call(MANAGER_DEACTIVATE_ALL, &args![manager, 0.0f32]);
        e.call(MANAGER_SET_FLAG, &args![manager, 0u32]);
    } else {
        let mut time = 0.0f32;
        if idle_a != 0 {
            if e.call(0x0059_bb30, &args![idle_a]).u32() == 0 {
                activate_sequence(e, manager, idle_a);
                e.call(SEQUENCE_SET_OFFSET, &args![idle_a, -most]);
                time = e.call(SEQUENCE_OFFSET_TIME, &args![idle_a]).f32();
            } else {
                let model = e.call(GET_MODEL, &args![this]).u32();
                e.call(LOG_MESSAGE, &args![NOT_LOOPING_MESSAGE, model]);
            }
        }
        if idle_b != 0 {
            activate_sequence(e, manager, idle_b);
            e.call(SEQUENCE_SET_OFFSET, &args![idle_b, -most]);
            time = e.call(SEQUENCE_OFFSET_TIME, &args![idle_b]).f32();
        }
        set_velocity(e, node, time, 1, 0);
    }
}

/// The last part of `InitAnimation` for an actor: refreshes what the actor's
/// process knows about its weapon and animation. `actor` is the reference
/// itself (it is an actor here), `biped` its biped (virtual `+0x1e8`),
/// `animation` the new animation (0 when there is none) and `node` its 3D
/// node. The process (`008d8520`) counts when its level (`0045cd60`) is zero or
/// one. A process of an actor that is not the player, while the loading
/// object is not busy, is given the worn item of slot 5 and the actor's 3D
/// (virtual `+0x160`), has the worn weapon attached (`0087b360`, or
/// `00571760` for the actor itself, unless the actor is named "Lily"), has its
/// animation state (`00496940`, virtual `+0x18c`) and its drawn-weapon state
/// (virtual `+0x1cc`, with the biped, the animation and the actor) refreshed.
/// The 3D's controller of type `011f36bc`, if any, is switched on
/// ([`fn_00566930`]).
fn init_actor_state(e: &mut Engine, me: u32, actor: u32, biped: u32, animation: u32, node: u32) {
    let loading = e.global::<u32>(GLOBAL_LOADING_OBJECT);
    let player = e.global::<u32>(GLOBAL_PLAYER);
    let mut acquire = 0u32;
    if e.call(ACTOR_PROCESS, &args![actor]).u32() != 0 {
        let process = e.call(ACTOR_PROCESS, &args![actor]).u32();
        let level = e.call(PROCESS_LEVEL, &args![process]).i32();
        if (0..=1).contains(&level) {
            acquire = e.call(ACTOR_PROCESS, &args![actor]).u32();
        }
    }
    if acquire != 0 && me != player && !e.call(0x0042_ce10, &args![loading]).bool() {
        let changes = e.call(0x004b_f220, &args![me]).u32();
        let worn = if changes != 0 {
            e.call(0x004c_8c10, &args![changes, 5u32, 0u32]).u32()
        } else {
            0
        };
        let worn_flags = if worn != 0 {
            e.call(FORM_GET_FLAGS, &args![worn]).u32()
        } else {
            0
        };
        let worn_again = if changes != 0 {
            e.call(0x004c_8c10, &args![changes, 5u32, 0u32]).u32()
        } else {
            0
        };
        let node_3d = e.vcall(me, 0x1d0, &args![]).u32();
        e.vcall(acquire, 0x160, &args![worn_again, node_3d, 0u32]);
        let actor_again = if e.vcall(me, 0x100, &args![]).bool() {
            me
        } else {
            0
        };
        let mut lily = false;
        if e.vcall(me, 0x100, &args![]).bool()
            && e.call(FORCE_NEXT_UPDATE, &args![actor_again]).bool()
        {
            let text = e.call(0x0055_d520, &args![actor_again]).u32();
            lily = e.call(STRSTR, &args![text, LILY_NAME]).u32() != 0;
        }
        if e.vcall(me, 0x100, &args![]).bool()
            && (e.vcall(me, 0x218, &args![]).bool() || lily)
            && worn_flags != 0
            && !e.vcall(acquire, 0x474, &args![1u32]).bool()
            && worn != 0
        {
            if e.call(0x008c_7aa0, &args![]).bool() && me != e.call(0x0047_b200, &args![]).u32() {
                let flags = e.call(FORM_GET_FLAGS, &args![worn]).u32();
                let manager = e.call(0x0045_37b0, &args![]).u32();
                e.call(0x0087_b360, &args![manager, me, flags]);
            } else if !lily {
                let flags = e.call(FORM_GET_FLAGS, &args![worn]).u32();
                e.call(0x0057_1760, &args![me, flags]);
            }
        }
        if animation != 0 && e.call(0x0055_85e0, &args![animation]).u32() != 0 {
            let process = e.call(ACTOR_PROCESS, &args![actor]).u32();
            let first = e.call(0x0055_85e0, &args![animation]).u32();
            let second = e.call(0x0049_6940, &args![animation]).u32();
            e.vcall(process, 0x18c, &args![second, first]);
        }
        if !e.call(0x0042_ce10, &args![loading]).bool() {
            e.vcall(acquire, 0x22c, &args![]);
            if e.call(0x008a_6970, &args![actor]).bool() {
                e.vcall(acquire, 0x458, &args![actor, 1u32]);
            }
            if me == player && biped == e.call(PLAYER_BIPED, &args![player, 1u32]).u32() {
                let process = e.call(ACTOR_PROCESS, &args![actor]).u32();
                let animation_of = e.call(PLAYER_ANIMATION, &args![player, 1u32]).u32();
                let drawn = e.call(ACTOR_WEAPON_DRAWN, &args![actor]).u8();
                e.vcall(
                    process,
                    0x1cc,
                    &args![u32::from(drawn), biped, animation_of, actor],
                );
            } else {
                let process = e.call(ACTOR_PROCESS, &args![actor]).u32();
                let animation_of = e.call(ACTOR_ANIMATION, &args![actor]).u32();
                let drawn = e.call(ACTOR_WEAPON_DRAWN, &args![actor]).u8();
                e.vcall(
                    process,
                    0x1cc,
                    &args![u32::from(drawn), biped, animation_of, actor],
                );
            }
        }
    }
    if e.vcall(me, 0x21c, &args![]).bool()
        && e.call(FORCE_NEXT_UPDATE, &args![actor]).bool()
        && e.call(0x0042_ce10, &args![loading]).bool()
    {
        let process = e.call(ACTOR_PROCESS, &args![actor]).u32();
        let animation_of = e.call(ACTOR_ANIMATION, &args![actor]).u32();
        let drawn = e.call(ACTOR_WEAPON_DRAWN, &args![actor]).u8();
        e.vcall(
            process,
            0x1cc,
            &args![u32::from(drawn), biped, animation_of, actor],
        );
    }
    let controller = e.call(0x00a5_c570, &args![node, CONTROLLER_RECORD]).u32();
    if controller != 0 {
        fn_00566930(e, Ptr::new(controller), 1);
    }
}

// Translated from 00566910 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0x68` of its object (called on a controller manager by
/// `InitAnimation`; the object is larger than a reference). Not named in
/// the Xbox PDB.
pub fn fn_00566910(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x68)
}

// Translated from 00566930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `0047aa60(this, flag, 0x40)`: the flag-setter of the object
/// `InitAnimation` calls it on (the controller of type `011f36bc`). Not
/// named in the Xbox PDB.
pub fn fn_00566930(e: &mut Engine, this: Ptr, flag: u8) {
    e.call(0x0047_aa60, &args![this, u32::from(flag), 0x40u32]);
}

// ---------------------------------------------------------------------------
// BuildKFFileList and what it uses.

/// The model loader object (`011c3b3c`) and its methods: `BuildKFFileList`
/// (`00447330(model, 1, 1, selector)`), `BuildFileList` (`00447300(path,
/// model, list)`) and `CopyFilenameList` (`00447850(list, into)`); the idle
/// manager (`011cb6a0`) and its `GetRootFilenameList` (`00600700(path,
/// 0)`).
const MODEL_LOADER: u32 = 0x011c_3b3c;
const BUILD_KF_FILE_LIST: u32 = 0x0044_7330;
const BUILD_FILE_LIST: u32 = 0x0044_7300;
const COPY_FILENAME_LIST: u32 = 0x0044_7850;
const IDLE_MANAGER: u32 = 0x011c_b6a0;
const ROOT_FILENAME_LIST: u32 = 0x0060_0700;
/// `Actor::GetCurrentWeapon` (`008a1710`), the weapon's index (`00446390`)
/// and the two tables it indexes: `0118a838` (dwords) and `011977a4` (C
/// string pointers, the file name prefix of the holster animation).
const ACTOR_CURRENT_WEAPON: u32 = 0x008a_1710;
const WEAPON_INDEX: u32 = 0x0044_6390;
const WEAPON_INDEX_TABLE: u32 = 0x0118_a838;
const HOLSTER_PREFIX_TABLE: u32 = 0x0119_77a4;
/// The strings the file lists are built from.
const PATH_DATA: u32 = 0x0101_6fb4; // "Data\"
const PATH_MESHES: u32 = 0x0101_dccc; // "Meshes"
const PATH_BACKSLASH: u32 = 0x0101_3444; // "\"
const PATH_MT_IDLE: u32 = 0x0101_6fa0; // "\MTIdle.KF"
const PATH_HOLSTER: u32 = 0x0101_6f90; // "\%sHolster.KF"
const PATH_POWER_ARMOR_HOLSTER: u32 = 0x0101_6f80; // "\PA%sHolster.KF"
const PATH_DEATH: u32 = 0x0101_6f74; // "\Death.KF"
const PATH_CHILD_IDLES: u32 = 0x0101_6f58; // "\Locomotion\Child\IdleAnims"
const PATH_FEMALE_IDLES: u32 = 0x0101_6f38; // "\Locomotion\Female\IdleAnims"
const PATH_MALE_IDLES: u32 = 0x0101_6f1c; // "\Locomotion\Male\IdleAnims"
const PATH_FULL_MALE_LOCOMOTION: u32 = 0x0102_ffe4; // "Data\Meshes\Characters\_Male\Locomotion\MTIdle.kf"
const PATH_MALE_LOCOMOTION: u32 = 0x0102_ffbc; // "Characters\_Male\Locomotion\MTIdle.kf"
const PATH_FULL_MALE_IDLE: u32 = 0x0102_ff94; // "Data\Meshes\Characters\_Male\MTIdle.kf"

/// `fn_00567050`'s globals: the object at `011f2250` (the map calls its
/// methods `VATS::...`; `0044ddc0` on it gives the mode, `009c8d60` the
/// target update multiplier), the frame time object at `011f6394` (`0084d030`
/// returns its `float` at +0xc) and the pointer at `011f21cc` the VATS mode
/// is compared against.
const VATS_OBJECT: u32 = 0x011f_2250;
const TARGET_UPDATE_MULT: u32 = 0x009c_8d60;
const FRAME_TIME_OBJECT: u32 = 0x011f_6394;
const FRAME_TIME: u32 = 0x0084_d030;
const REFERENCE_COMPARED_WITH_VATS: u32 = 0x011f_21cc;

/// What follows the idle list in both character paths: the holster and
/// power armor holster animations of the actor's current weapon (the
/// prefix from the two tables, when it is not empty) and the death
/// animation. `slash` is the last backslash of `path` (the file name is
/// rewritten from there); every list is added to `list`.
fn add_holster_and_death_files(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    path: u32,
    slash: u32,
    list: u32,
) {
    let me = this.addr();
    let loader = e.global::<u32>(MODEL_LOADER);
    let weapon = e.call(ACTOR_CURRENT_WEAPON, &args![this]).u32();
    let index = if weapon == 0 {
        0
    } else {
        e.call(WEAPON_INDEX, &args![weapon]).u32()
    };
    let slot = e.global::<u32>(WEAPON_INDEX_TABLE.wrapping_add(index.wrapping_mul(4)));
    let prefix = e.global::<u32>(HOLSTER_PREFIX_TABLE.wrapping_add(slot.wrapping_mul(4)));
    if prefix != 0 && e.mem.u8(prefix) != 0 {
        e.call(SPRINTF, &args![slash, PATH_HOLSTER, prefix]);
        let model = e.call(GET_MODEL, &args![me]).u32();
        e.call(BUILD_FILE_LIST, &args![loader, path, model, list]);
        e.call(SPRINTF, &args![slash, PATH_POWER_ARMOR_HOLSTER, prefix]);
        let model = e.call(GET_MODEL, &args![me]).u32();
        e.call(BUILD_FILE_LIST, &args![loader, path, model, list]);
    }
    let room = 0x104u32.wrapping_sub(slash.wrapping_sub(path));
    e.call(BOUNDED_COPY, &args![slash, room, PATH_DEATH]);
    let model = e.call(GET_MODEL, &args![me]).u32();
    e.call(BUILD_FILE_LIST, &args![loader, path, model, list]);
}

// Translated from 00566970 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::BuildKFFileList` (Xbox PDB): the list of KF animation
/// files to load for the reference, 0 when its base form has none of the
/// types below. The C++ exception state and the stack cookie are not
/// translated. `selector` is the word the model loader's list builder takes
/// (the callers pass the actor's skeleton).
///
/// - Base form type 0x2b: `selector` becomes the weapon table entry of the
///   actor's weapon (virtual `+0x148` of its process, then `0044ddc0` and
///   `00446390`), or 1, when the process level (`0045cd60`) is 0 or 1 and the
///   reference does not report the virtual `+0x22c(0)`. Without the virtual
///   `+0x22c(1)` the loader builds the list from the model (`00447330`).
///   With it, the list is built from `Data\Meshes\<model>` with
///   `\MTIdle.KF`, followed by the holster and death files
///   ([`add_holster_and_death_files`]).
/// - Base form type 0x2a: with form ID 7, or without the virtual
///   `+0x22c(1)`, the loader builds the list from the base form's model
///   (`+0xdc`, virtual `+0x14`) and the idle lists of the child (virtual
///   `+0x1a0(1)`), female (`005f0cc0` is 1) or male actors are copied into
///   it. Otherwise the male locomotion list is built and the holster and
///   death files are added.
pub fn tes_object_refr_build_kf_file_list(
    e: &mut Engine,
    this: Ptr<TESObjectREFR>,
    selector: u32,
) -> u32 {
    let me = this.addr();
    let mut selector = selector;
    let mut list = 0u32;
    if e.call(GET_BASE_FORM, &args![this]).u32() == 0 {
        return list;
    }
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    let kind = e.call(FORM_TYPE, &args![base]).u32();
    let loader = e.global::<u32>(MODEL_LOADER);
    if kind == 0x2b {
        if e.call(ACTOR_PROCESS, &args![me]).u32() != 0 && !e.vcall(me, 0x22c, &args![0u32]).bool()
        {
            let process = e.call(ACTOR_PROCESS, &args![me]).u32();
            let level = e.call(PROCESS_LEVEL, &args![process]).i32();
            if (0..=1).contains(&level) {
                let process = e.call(ACTOR_PROCESS, &args![me]).u32();
                if e.vcall(process, 0x148, &args![]).u32() != 0 {
                    let weapon = e.vcall(process, 0x148, &args![]).u32();
                    let flags = e.call(FORM_GET_FLAGS, &args![weapon]).u32();
                    let index = e.call(WEAPON_INDEX, &args![flags]).u32();
                    selector = e.global(WEAPON_INDEX_TABLE.wrapping_add(index.wrapping_mul(4)));
                } else if e.call(0x008a_6970, &args![me]).bool() {
                    selector = 1;
                }
            }
        }
        if !e.vcall(me, 0x22c, &args![1u32]).bool() {
            let model = e.call(GET_MODEL, &args![this]).u32();
            list = e
                .call(
                    BUILD_KF_FILE_LIST,
                    &args![loader, model, 1u32, 1u32, selector],
                )
                .u32();
        } else {
            e.with_stack(0x104, |e, path| {
                let path = path.addr();
                e.call(BOUNDED_COPY, &args![path, 0x104u32, PATH_DATA]);
                e.call(BOUNDED_APPEND, &args![path, 0x104u32, PATH_MESHES]);
                e.call(BOUNDED_APPEND, &args![path, 0x104u32, PATH_BACKSLASH]);
                let model = e.call(GET_MODEL, &args![this]).u32();
                e.call(BOUNDED_APPEND, &args![path, 0x104u32, model]);
                let slash = e.call(STRRCHR_WRAPPER, &args![path, 0x5cu32]).u32();
                let room = 0x104u32.wrapping_sub(slash.wrapping_sub(path));
                e.call(BOUNDED_COPY, &args![slash, room, PATH_MT_IDLE]);
                let model = e.call(GET_MODEL, &args![this]).u32();
                list = e
                    .call(BUILD_FILE_LIST, &args![loader, path, model, 0u32])
                    .u32();
                add_holster_and_death_files(e, this, path, slash, list);
            });
        }
    } else if kind == 0x2a {
        if e.call(FORM_ID, &args![base]).u32() == 7 || !e.vcall(me, 0x22c, &args![1u32]).bool() {
            let model = e.vcall(base + 0xdc, 0x14, &args![]).u32();
            list = e
                .call(
                    BUILD_KF_FILE_LIST,
                    &args![loader, model, 1u32, 1u32, selector],
                )
                .u32();
            let model = e.vcall(base + 0xdc, 0x14, &args![]).u32();
            e.with_stack(0x104, |e, path| {
                let path = path.addr();
                e.call(BOUNDED_COPY, &args![path, 0x104u32, model]);
                let slash = e.call(STRRCHR_WRAPPER, &args![path, 0x5cu32]).u32();
                let room = 0x104u32
                    .wrapping_sub(slash.wrapping_sub(path))
                    .wrapping_sub(1);
                let idles = if e.vcall(me, 0x1a0, &args![1u32]).bool() {
                    PATH_CHILD_IDLES
                } else if e.call(0x005f_0cc0, &args![base]).i32() == 1 {
                    PATH_FEMALE_IDLES
                } else {
                    PATH_MALE_IDLES
                };
                e.call(BOUNDED_COPY, &args![slash, room, idles]);
                let idle_manager = e.global::<u32>(IDLE_MANAGER);
                let idle_list = e
                    .call(ROOT_FILENAME_LIST, &args![idle_manager, path, 0u32])
                    .u32();
                e.call(COPY_FILENAME_LIST, &args![loader, idle_list, list]);
            });
        } else {
            e.with_stack(0x104, |e, full| {
                e.with_stack(0x104, |e, short| {
                    let full = full.addr();
                    e.call(
                        BOUNDED_COPY,
                        &args![full, 0x104u32, PATH_FULL_MALE_LOCOMOTION],
                    );
                    e.call(BOUNDED_COPY, &args![short, 0x104u32, PATH_MALE_LOCOMOTION]);
                    list = e
                        .call(BUILD_FILE_LIST, &args![loader, full, short, 0u32])
                        .u32();
                    e.call(BOUNDED_COPY, &args![full, 0x104u32, PATH_FULL_MALE_IDLE]);
                    let slash = e.call(STRRCHR_WRAPPER, &args![full, 0x5cu32]).u32();
                    add_holster_and_death_files(e, this, full, slash, list);
                });
            });
        }
    }
    list
}

// Translated from 00567050 (decompiled, FalloutNV.exe 1.4.0.525)
/// A per-frame step of a reference's animation (the Xbox PDB has no name for
/// it): nothing for a deleted or disabled reference. For a base form of type
/// 0x1e the virtual `+0x178` is called. Unless the base form is of type 0x2a
/// or 0x2b, the sequences of the 3D's controller manager (virtual `+0x88` of
/// its first controller) that `008041a0` accepts are asked to play their
/// sounds (`004eef00`); one that is finished is ended
/// ([`tes_object_refr_end_sequence`]), finished in the manager
/// (`0047b220`), recorded as the last finished sequence (`00578a30` with
/// its name) and announced (`0047ac70`); when none is left unfinished the
/// manager is switched off. For types 0x2a and 0x2b the animation
/// (virtual `+0x1e4`) is updated by the frame time (`011f6394`, scaled by
/// the VATS target mult when VATS is in mode 4 for the player's own
/// reference) if the 3D can be updated (`009611e0`), or [`tes_object_refr_init_animation`]
/// is run when there is no animation yet but a 3D.
pub fn fn_00567050(e: &mut Engine, this: Ptr<TESObjectREFR>) {
    let me = this.addr();
    let data = this.at(TESObjectREFR::data);
    let disabled = e.call(FORM_IS_DISABLED, &args![this]).bool();
    if e.call(FORM_IS_DELETED, &args![this]).bool() || disabled {
        return;
    }
    let base = e.get(data, OBJ_REFR::pObjectReference).addr();
    let kind = e.call(FORM_TYPE, &args![base]).u32();
    let mut updates_animation = false;
    if kind == 0x1e {
        e.vcall(me, 0x178, &args![]);
    } else if (0x2a..=0x2b).contains(&kind) {
        updates_animation = true;
    }
    if !updates_animation {
        let node = e.call(GET_LOADED_3D, &args![this]).u32();
        if node != 0 {
            let controller = e.call(GET_CONTROLLER, &args![node]).u32();
            if controller != 0 {
                let manager = e.vcall(controller, 0x88, &args![]).u32();
                let mut unfinished = 0u32;
                if manager != 0 && e.call(MANAGER_HAS_SEQUENCES, &args![manager]).bool() {
                    let mut index = 0u32;
                    while index < e.call(MANAGER_SEQUENCE_COUNT, &args![manager]).u32() {
                        let sequence = e.call(MANAGER_SEQUENCE_AT, &args![manager, index]).u32();
                        if sequence != 0 && e.call(SEQUENCE_IS_GENERIC, &args![sequence]).u32() != 0
                        {
                            if e.call(0x004e_ef00, &args![sequence, me]).bool() {
                                tes_object_refr_end_sequence(e, this, Ptr::new(sequence));
                                e.call(MANAGER_FINISH_SEQUENCE, &args![manager, sequence, 0.0f32]);
                                let name = sequence_name(e, sequence);
                                e.call(0x0057_8a30, &args![me, name]);
                                e.call(0x0047_ac70, &args![me, sequence]);
                            } else {
                                unfinished += 1;
                            }
                        }
                        index += 1;
                    }
                    if unfinished == 0 {
                        e.call(MANAGER_SET_FLAG, &args![manager, 0u32]);
                    }
                }
            }
        }
    }
    if updates_animation {
        let animation = e.vcall(me, 0x1e4, &args![]).u32();
        let node = e.call(GET_LOADED_3D, &args![this]).u32();
        if animation == 0 {
            if node != 0 {
                tes_object_refr_init_animation(e, this);
            }
        } else if node != 0 && e.call(0x0096_11e0, &args![node]).u32() != 0 {
            let minus_one: f32 = e.global(MINUS_ONE_FLOAT);
            if e.call(FORM_GET_FLAGS, &args![VATS_OBJECT]).u32() == 4
                && me == e.global::<u32>(REFERENCE_COMPARED_WITH_VATS)
            {
                let frame_time = e.call(FRAME_TIME, &args![FRAME_TIME_OBJECT]).f64();
                let scale = e.call(TARGET_UPDATE_MULT, &args![VATS_OBJECT]).f64();
                let delta = (scale * frame_time) as f32;
                e.call(ANIMATION_UPDATE, &args![animation, me, delta, minus_one]);
            } else {
                let frame_time = e.call(FRAME_TIME, &args![FRAME_TIME_OBJECT]).f64();
                e.call(
                    ANIMATION_UPDATE,
                    &args![animation, me, frame_time as f32, minus_one],
                );
            }
        }
    }
}

// Translated from 005672c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::EndSequence` (Xbox PDB): for a base form of type 0x15 or
/// 0x1c and the sequence named "Unequip" (the pointer at `01197b5c`, compared
/// with `00404dc0`), when the reference's parent cell passes `004543c0`: the
/// 3D gets `00c6a270(3d, 1, 1, 1)`, then either (`008c7aa0`) the manager
/// object `004537b0` is told about it (`0087b1e0(3d, 1)`) or the 3D's motion
/// is set (`00c6a350(3d, 1, 1, 0, 1)`). The virtual `+0x48` is then called
/// with 4.
pub fn tes_object_refr_end_sequence(e: &mut Engine, this: Ptr<TESObjectREFR>, sequence: Ptr) {
    let me = this.addr();
    let data = this.at(TESObjectREFR::data);
    let base = e.get(data, OBJ_REFR::pObjectReference).addr();
    let kind = e.call(FORM_TYPE, &args![base]).u32();
    if kind != 0x15 && kind != 0x1c {
        return;
    }
    let unequip = e.global::<u32>(NAME_UNEQUIP);
    let name = sequence_name(e, sequence.addr());
    if e.call(0x0040_4dc0, &args![name, unequip]).u32() != 0 {
        return;
    }
    let cell = e.get(this, TESObjectREFR::pParentCell).addr();
    if cell == 0 || e.call(0x0045_43c0, &args![cell]).u32() == 0 {
        return;
    }
    let node = e.vcall(me, 0x1d0, &args![]).u32();
    e.call(0x00c6_a270, &args![node, 1u32, 1u32, 1u32]);
    if e.call(0x008c_7aa0, &args![]).bool() {
        let node = e.vcall(me, 0x1d0, &args![]).u32();
        let manager = e.call(0x0045_37b0, &args![]).u32();
        e.call(0x0087_b1e0, &args![manager, node, 1u32]);
    } else {
        e.call(0x0045_43c0, &args![cell]);
        let node = e.vcall(me, 0x1d0, &args![]).u32();
        e.call(0x00c6_a350, &args![node, 1u32, 1u32, 0u32, 1u32]);
    }
    e.vcall(me, 0x48, &args![4u32]);
}

// Translated from 005673c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetScriptPointer` (Xbox PDB): `ExtraDataList::GetScript`
/// (`00418800`) of the reference's extra list.
pub fn tes_object_refr_get_script_pointer(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    let list = e.call(GET_EXTRA_LIST, &args![this]).u32();
    e.call(0x0041_8800, &args![list]).u32()
}

// Translated from 005673e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetScriptVariables` (Xbox PDB): the script locals
/// (`00418830`) of the reference's extra list.
pub fn tes_object_refr_get_script_variables(e: &mut Engine, this: Ptr<TESObjectREFR>) -> u32 {
    let list = e.call(GET_EXTRA_LIST, &args![this]).u32();
    e.call(0x0041_8830, &args![list]).u32()
}

// Translated from 00567400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESObjectREFR::GetScale` (Xbox PDB): the reference's scale (`fRefScale`)
/// times the base form's own scale for the base form types 0x2a
/// (`00944300`) and 0x2b (`00567470`); the plain `fRefScale` otherwise.
pub fn tes_object_refr_get_scale(e: &mut Engine, this: Ptr<TESObjectREFR>) -> f32 {
    let mut scale = e.get(this, TESObjectREFR::fRefScale);
    let base = e.call(GET_BASE_FORM, &args![this]).u32();
    if base != 0 {
        let own = match e.call(FORM_TYPE, &args![base]).u32() {
            0x2a => Some(e.call(0x0094_4300, &args![base]).f64()),
            0x2b => Some(e.call(0x0056_7470, &args![base]).f64()),
            _ => None,
        };
        if let Some(own) = own {
            scale = (own * f64::from(scale)) as f32;
        }
    }
    scale
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x00563d80,
            fn_00563d80(Ptr<TESObjectREFR>, Ptr, Ptr, f32) -> bool
        ),
        entry!(
            0x00563f30,
            tes_object_refr_save_controller_manager(Ptr, Ptr, f32)
        ),
        entry!(0x00564010, fn_00564010(Ptr, Ptr, f32)),
        entry!(0x00564360, fn_00564360(Ptr) -> u32),
        entry!(0x00564480, tes_object_refr_create_reference(u8, u8) -> Ptr),
        entry!(0x00564820, fn_00564820(Ptr<TESObjectREFR>) -> u32),
        entry!(0x00564900, fn_00564900(i32) -> bool),
        entry!(
            0x00564930,
            tes_object_refr_set_delete(Ptr<TESObjectREFR>, u8)
        ),
        entry!(0x005649b0, fn_005649b0(Ptr<TESObjectREFR>) -> bool),
        entry!(0x00564a00, fn_00564a00(Ptr<TESObjectREFR>, u8)),
        entry!(0x00564ab0, fn_00564ab0(Ptr<TESObjectREFR>) -> bool),
        entry!(0x00564b00, fn_00564b00(Ptr<TESObjectREFR>, u8)),
        entry!(0x00564b70, fn_00564b70(Ptr<TESObjectREFR>) -> bool),
        entry!(0x00564bc0, fn_00564bc0(Ptr<TESObjectREFR>) -> bool),
        entry!(0x00564c10, fn_00564c10(Ptr<TESObjectREFR>) -> bool),
        entry!(0x00564c60, fn_00564c60(Ptr<TESObjectREFR>, u8)),
        entry!(0x00564cd0, fn_00564cd0(Ptr<TESObjectREFR>) -> bool),
        entry!(0x00564d20, fn_00564d20(Ptr<TESObjectREFR>, u8)),
        entry!(0x00564d80, fn_00564d80(Ptr<TESObjectREFR>) -> bool),
        entry!(
            0x00564db0,
            tes_object_refr_set_targeted(Ptr<TESObjectREFR>, u8)
        ),
        entry!(0x00565050, fn_00565050(Ptr<TESObjectREFR>) -> bool),
        entry!(
            0x00565260,
            tes_object_refr_must_ref_persist(Ptr<TESObjectREFR>) -> bool
        ),
        entry!(
            0x005653d0,
            tes_object_refr_get_ref_persists(Ptr<TESObjectREFR>) -> bool
        ),
        entry!(0x00565450, fn_00565450(Ptr<TESObjectREFR>) -> bool),
        entry!(
            0x00565480,
            tes_object_refr_set_ref_persists(Ptr<TESObjectREFR>, u8)
        ),
        entry!(
            0x00565580,
            tes_object_refr_has_time_controllers(Ptr) -> bool
        ),
        entry!(0x00565670, fn_00565670(u32) -> bool),
        entry!(0x00565690, fn_00565690(Ptr<TESObjectREFR>) -> bool),
        entry!(0x005656d0, fn_005656d0(Ptr<TESObjectREFR>) -> bool),
        entry!(0x00565730, tes_object_refr_init_script(Ptr<TESObjectREFR>)),
        entry!(
            0x00565870,
            tes_object_refr_run_script(Ptr<TESObjectREFR>) -> bool
        ),
        entry!(
            0x005659f0,
            tes_object_refr_init_animation(Ptr<TESObjectREFR>)
        ),
        entry!(0x00566910, fn_00566910(Ptr) -> u8),
        entry!(0x00566930, fn_00566930(Ptr, u8)),
        entry!(
            0x00566970,
            tes_object_refr_build_kf_file_list(Ptr<TESObjectREFR>, u32) -> u32
        ),
        entry!(0x00567050, fn_00567050(Ptr<TESObjectREFR>)),
        entry!(
            0x005672c0,
            tes_object_refr_end_sequence(Ptr<TESObjectREFR>, Ptr)
        ),
        entry!(
            0x005673c0,
            tes_object_refr_get_script_pointer(Ptr<TESObjectREFR>) -> u32
        ),
        entry!(
            0x005673e0,
            tes_object_refr_get_script_variables(Ptr<TESObjectREFR>) -> u32
        ),
        entry!(
            0x00567400,
            tes_object_refr_get_scale(Ptr<TESObjectREFR>) -> f32
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    type Log = Vec<(u32, Vec<u32>)>;

    fn calls_to(log: &Log, addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// Registers `addrs` as doubles that do nothing and return 0.
    fn stub(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// A double that returns `value` in `eax`.
    fn returns(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| Ret {
            eax: value,
            ..Ret::default()
        });
    }

    /// A double that returns `value` in `ST0`.
    fn returns_float(e: &mut Engine, addr: u32, value: f32) {
        e.register_double(addr, move |_, _| Ret {
            st0: f64::from(value),
            ..Ret::default()
        });
    }

    /// Runs `call` with the call log on and gives back the log.
    fn logged(e: &mut Engine, call: impl FnOnce(&mut Engine)) -> Log {
        e.call_log = Some(vec![]);
        call(e);
        e.call_log.take().unwrap()
    }

    /// An engine with the accessors every function here leans on registered
    /// with the bodies the exe gives them (a form is: type byte at +4, flags
    /// at +8, ID at +0xc; the base form of a reference is at +0x20), and the
    /// pages of the globals mapped.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_2000,
            0x0101_a000,
            0x0101_3000,
            0x0102_f000,
            0x0118_7000,
            0x0118_a000,
            0x0119_7000,
            0x011c_3000,
            0x011c_b000,
            0x011d_d000,
            0x011d_e000,
            0x011f_2000,
            0x011f_3000,
            0x011f_6000,
        ] {
            e.map(page, 0x1000);
        }
        e.register(GET_BASE_FORM, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(FORM_TYPE, |e, a| u32::from(e.mem.u8(a[0] + 4)).into_ret());
        e.register(FORM_ID, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        e.register(FORM_GET_FLAGS, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(FORM_SET_FLAGS, |e, a| {
            e.mem.set_u32(a[0] + 8, a[1]);
            Ret::default()
        });
        e.register(FORM_IS_DELETED, |e, a| {
            (e.mem.u32(a[0] + 8) & 0x20 != 0).into_ret()
        });
        e.register(FORM_IS_DISABLED, |e, a| {
            (e.mem.u32(a[0] + 8) & 0x800 != 0).into_ret()
        });
        e.register(GET_EXTRA_LIST, |_, a| (a[0] + 0x44).into_ret());
        e.set_global(MINUS_ONE_DOUBLE, -1.0f64);
        e.set_global(MINUS_ONE_FLOAT, -1.0f32);
        e.set_global(FLOAT_MAX, f32::MAX);
        e.set_global(ZERO_DOUBLE, 0.0f64);
        e.set_global(STEP_DIVISOR, 20.0f64);
        e.set_global(MIN_STEP_DOUBLE, 0.0166666f64);
        e.set_global(MIN_STEP_FLOAT, 0.0166666f32);
        e.set_global(DEFAULT_BLEND_TIME, 0.25f32);
        e
    }

    /// A fresh fake address with `f` registered at it, for vtable slots.
    fn fake_function(e: &mut Engine, f: AbiFn) -> u32 {
        static NEXT: AtomicU32 = AtomicU32::new(0x00fd_0000);
        let addr = NEXT.fetch_add(4, Ordering::Relaxed);
        e.register(addr, f);
        addr
    }

    /// An object whose vtable has the given (byte offset, function) slots.
    fn object_with_vtable(e: &mut Engine, size: u32, slots: &[(u32, AbiFn)]) -> u32 {
        let object = e.mem.alloc(size);
        let vtable = e.mem.alloc(0x500);
        for (offset, f) in slots {
            let target = fake_function(e, *f);
            e.mem.set_u32(vtable + offset, target);
        }
        e.mem.set_u32(object, vtable);
        object
    }

    /// A form with a type byte, ID and flags.
    fn form(e: &mut Engine, kind: u8, id: u32, flags: u32) -> u32 {
        let form = e.mem.alloc(0x100);
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 8, flags);
        e.mem.set_u32(form + 0xc, id);
        form
    }

    /// A reference (with a vtable made of `slots`) over the base form `base`.
    fn refr_with(
        e: &mut Engine,
        base: u32,
        flags: u32,
        slots: &[(u32, AbiFn)],
    ) -> Ptr<TESObjectREFR> {
        let object = object_with_vtable(e, 0x100, slots);
        e.mem.set_u32(object + 8, flags);
        e.mem.set_u32(object + 0x20, base);
        Ptr::new(object)
    }

    /// A reference whose base form has the type `kind`.
    fn refr_of_kind(e: &mut Engine, kind: u8) -> Ptr<TESObjectREFR> {
        let base = form(e, kind, 0x77, 0);
        refr_with(e, base, 0, &[])
    }

    fn cstr(e: &mut Engine, text: &str) -> u32 {
        let address = e.mem.alloc(text.len() as u32 + 1);
        e.mem.set_cstr(address, text.as_bytes());
        address
    }

    /// The doubles of the sequence and manager methods. A sequence object
    /// (see [`sequence`]) has +4 non-zero when generic, +8 the name's C
    /// string and +0x2c, +0x30, +0x34 its offset time, limit time and
    /// duration; a manager (see [`manager`]) +4 the count and the sequences
    /// from +8.
    fn sequence_api(e: &mut Engine) {
        e.register(MANAGER_SEQUENCE_COUNT, |e, a| {
            e.mem.u32(a[0] + 4).into_ret()
        });
        e.register(MANAGER_SEQUENCE_AT, |e, a| {
            e.mem.u32(a[0] + 8 + 4 * a[1]).into_ret()
        });
        e.register(MANAGER_HAS_SEQUENCES, |_, _| true.into_ret());
        e.register(SEQUENCE_NAME_HOLDER, |_, a| (a[0] + 8).into_ret());
        e.register(NAME_TEXT, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(SEQUENCE_IS_GENERIC, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(SEQUENCE_OFFSET_TIME, |e, a| {
            e.mem.f32(a[0] + 0x2c).into_ret()
        });
        e.register(SEQUENCE_LIMIT_TIME, |e, a| {
            e.mem.f32(a[0] + 0x30).into_ret()
        });
        e.register(SEQUENCE_DURATION, |e, a| e.mem.f32(a[0] + 0x34).into_ret());
        let compare: AbiFn = |e, a| {
            let first = e.mem.cstr(a[0]);
            let second = e.mem.cstr(a[1]);
            (first.cmp(&second) as i32).into_ret()
        };
        e.register(STRCMP, compare);
        e.register(STRING_COMPARE, compare);
        e.register(MAKE_VELOCITY, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
    }

    fn sequence(e: &mut Engine, name: &str, generic: bool) -> u32 {
        let sequence = e.mem.alloc(0x40);
        let text = cstr(e, name);
        e.mem.set_u32(sequence + 8, text);
        e.mem.set_u32(sequence + 4, u32::from(generic));
        sequence
    }

    fn manager(e: &mut Engine, sequences: &[u32]) -> u32 {
        let manager = e.mem.alloc(0x100);
        e.mem.set_u32(manager + 4, sequences.len() as u32);
        for (index, sequence) in sequences.iter().enumerate() {
            e.mem.set_u32(manager + 8 + 4 * index as u32, *sequence);
        }
        manager
    }

    // ---- 00563d80 ---------------------------------------------------------

    fn restart_fixture() -> (Engine, Ptr<TESObjectREFR>, u32, u32, u32, u32) {
        let mut e = engine();
        sequence_api(&mut e);
        stub(
            &mut e,
            &[
                MANAGER_DEACTIVATE_ALL,
                MANAGER_SET_FLAG,
                MANAGER_ACTIVATE,
                SEQUENCE_SET_OFFSET,
                RESET_SIM,
                SET_3D_VELOCITY,
                MANAGER_FINISH_SEQUENCE,
            ],
        );
        let node = e.mem.alloc(0x20);
        returns(&mut e, GET_LOADED_3D, node);
        let idle = sequence(&mut e, "Idle", false);
        let walk = sequence(&mut e, "Walk", true);
        let manager = manager(&mut e, &[idle, walk]);
        let refr = refr_of_kind(&mut e, 0);
        (e, refr, manager, idle, walk, node)
    }

    #[test]
    fn fn_00563d80_plays_a_sequence_from_a_time_before_its_offset() {
        let (mut e, refr, manager, idle, _, node) = restart_fixture();
        e.mem.set_f32(idle + 0x2c, 2.0);
        let name = cstr(&mut e, "Idle");
        let log = logged(&mut e, |e| {
            let played = e
                .call(0x0056_3d80, &args![refr, manager, name, 1.0f32])
                .bool();
            assert!(played);
        });
        // the sequence is not a generic one, so it is activated
        assert_eq!(
            calls_to(&log, MANAGER_ACTIVATE),
            vec![vec![
                manager,
                idle,
                0,
                0,
                1.0f32.to_bits(),
                0.0f32.to_bits(),
                0
            ]]
        );
        // offset -FLT_MAX, the record holds the offset time
        assert_eq!(
            calls_to(&log, SEQUENCE_SET_OFFSET),
            vec![vec![idle, (-f32::MAX).to_bits()]]
        );
        let made = calls_to(&log, MAKE_VELOCITY);
        assert_eq!(
            (made[0][1], made[0][2], made[0][3]),
            (2.0f32.to_bits(), 1, 0)
        );
        assert_eq!(calls_to(&log, RESET_SIM), vec![vec![node, 1]]);
        assert_eq!(
            calls_to(&log, MANAGER_FINISH_SEQUENCE),
            vec![vec![manager, idle, 0.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, MANAGER_SET_FLAG),
            vec![vec![manager, 1], vec![manager, 0]]
        );
    }

    #[test]
    fn fn_00563d80_clamps_to_the_limit_when_the_time_is_past_it() {
        let (mut e, refr, manager, idle, _, _) = restart_fixture();
        e.mem.set_f32(idle + 0x2c, 0.5);
        e.mem.set_f32(idle + 0x30, 0.75);
        let name = cstr(&mut e, "Idle");
        let log = logged(&mut e, |e| {
            e.call(0x0056_3d80, &args![refr, manager, name, 1.0f32]);
        });
        assert_eq!(
            calls_to(&log, SEQUENCE_SET_OFFSET),
            vec![vec![idle, 0.75f32.to_bits()]]
        );
        assert_eq!(calls_to(&log, MAKE_VELOCITY)[0][1], 0.75f32.to_bits());
    }

    #[test]
    fn fn_00563d80_keeps_the_time_inside_the_limits_and_skips_activating_generic() {
        let (mut e, refr, manager, _, walk, _) = restart_fixture();
        e.mem.set_f32(walk + 0x2c, 0.5);
        e.mem.set_f32(walk + 0x30, 5.0);
        let name = cstr(&mut e, "Walk");
        let log = logged(&mut e, |e| {
            e.call(0x0056_3d80, &args![refr, manager, name, 1.0f32]);
        });
        assert_eq!(
            calls_to(&log, SEQUENCE_SET_OFFSET),
            vec![vec![walk, 1.0f32.to_bits()]]
        );
        assert_eq!(calls_to(&log, MAKE_VELOCITY)[0][1], 1.0f32.to_bits());
        // a generic sequence is not activated again
        assert!(calls_to(&log, MANAGER_ACTIVATE).is_empty());
    }

    #[test]
    fn fn_00563d80_reports_false_without_a_match_a_manager_or_a_3d() {
        let (mut e, refr, manager, _, _, _) = restart_fixture();
        let missing = cstr(&mut e, "Missing");
        let log = logged(&mut e, |e| {
            assert!(!e
                .call(0x0056_3d80, &args![refr, manager, missing, 1.0f32])
                .bool());
            assert!(!e
                .call(0x0056_3d80, &args![refr, 0u32, missing, 1.0f32])
                .bool());
        });
        assert!(calls_to(&log, MANAGER_DEACTIVATE_ALL).is_empty());
        returns(&mut e, GET_LOADED_3D, 0);
        let name = cstr(&mut e, "Idle");
        assert!(!e
            .call(0x0056_3d80, &args![refr, manager, name, 1.0f32])
            .bool());
    }

    // ---- 00563f30 ---------------------------------------------------------

    #[test]
    fn save_controller_manager_writes_the_generic_sequences() {
        let mut e = engine();
        sequence_api(&mut e);
        returns(&mut e, SAVE_START_SIZED, 77);
        stub(
            &mut e,
            &[SAVE_STRING, SEQUENCE_SAVE_TO_BUFFER, SAVE_END_SIZED],
        );
        let idle = sequence(&mut e, "Idle", false);
        let walk = sequence(&mut e, "Walk", true);
        let run = sequence(&mut e, "Run", true);
        let manager = manager(&mut e, &[idle, walk, run]);
        let buffer = e.mem.alloc(0x20);
        let log = logged(&mut e, |e| {
            e.call(0x0056_3f30, &args![manager, buffer, 0.5f32]);
        });
        let walk_name = e.mem.u32(walk + 8);
        let run_name = e.mem.u32(run + 8);
        assert_eq!(
            calls_to(&log, SAVE_STRING),
            vec![vec![buffer, walk_name, 0], vec![buffer, run_name, 0]]
        );
        assert_eq!(
            calls_to(&log, SEQUENCE_SAVE_TO_BUFFER),
            vec![
                vec![walk, buffer, 0.5f32.to_bits()],
                vec![run, buffer, 0.5f32.to_bits()]
            ]
        );
        // the count and the start returned by the start call
        assert_eq!(calls_to(&log, SAVE_END_SIZED), vec![vec![buffer, 2, 77]]);
    }

    #[test]
    fn save_controller_manager_uses_the_default_blend_and_copes_without_a_manager() {
        let mut e = engine();
        sequence_api(&mut e);
        returns(&mut e, SAVE_START_SIZED, 5);
        stub(
            &mut e,
            &[SAVE_STRING, SEQUENCE_SAVE_TO_BUFFER, SAVE_END_SIZED],
        );
        let walk = sequence(&mut e, "Walk", true);
        let manager = manager(&mut e, &[walk]);
        let buffer = e.mem.alloc(0x20);
        let log = logged(&mut e, |e| {
            e.call(0x0056_3f30, &args![manager, buffer, -1.0f32]);
            e.call(0x0056_3f30, &args![0u32, buffer, 1.0f32]);
        });
        assert_eq!(
            calls_to(&log, SEQUENCE_SAVE_TO_BUFFER),
            vec![vec![walk, buffer, 0.25f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, SAVE_END_SIZED),
            vec![vec![buffer, 1, 5], vec![buffer, 0, 5]]
        );
    }

    // ---- 00564010 ---------------------------------------------------------

    /// A load of two entries: the first names "Walk" (in the manager), the
    /// second "Missing".
    fn load_fixture(actor_3d: bool) -> (Engine, u32, u32, u32, u32) {
        let mut e = engine();
        sequence_api(&mut e);
        stub(
            &mut e,
            &[
                MANAGER_DEACTIVATE_ALL,
                MANAGER_SET_FLAG,
                MANAGER_ACTIVATE,
                SEQUENCE_LOAD_FROM_BUFFER,
                SEQUENCE_CONSTRUCT,
                SEQUENCE_DESTRUCT,
                RESET_SIM,
                SET_3D_VELOCITY,
                ADD_3D_VELOCITY,
            ],
        );
        returns(&mut e, LOAD_START_SIZED, 2);
        let mut names = vec!["Walk", "Missing"].into_iter();
        e.register_double(LOAD_STRING, move |e, a| {
            let name = names.next().unwrap();
            e.mem.set_cstr(a[1], name.as_bytes());
            Ret::default()
        });
        let node = e.mem.alloc(0x20);
        returns(&mut e, GET_LOADED_3D, node);
        returns(&mut e, IS_ACTOR_3D, u32::from(actor_3d));
        let owner = e.mem.alloc(0x20);
        // the buffer's virtual +8 gives the owner stored at +4
        let buffer =
            object_with_vtable(&mut e, 0x20, &[(8, |e, a| e.mem.u32(a[0] + 4).into_ret())]);
        e.mem.set_u32(buffer + 4, owner);
        let walk = sequence(&mut e, "Walk", false);
        e.mem.set_f32(walk + 0x34, 2.0);
        let manager = manager(&mut e, &[walk]);
        (e, buffer, manager, walk, node)
    }

    #[test]
    fn fn_00564010_loads_known_sequences_and_consumes_unknown_ones() {
        let (mut e, buffer, manager, walk, node) = load_fixture(false);
        let log = logged(&mut e, |e| {
            e.call(0x0056_4010, &args![manager, buffer, 1.0f32]);
        });
        // the manager is cleared and switched on because entries follow
        assert_eq!(
            calls_to(&log, MANAGER_DEACTIVATE_ALL),
            vec![vec![manager, 0.0f32.to_bits()]]
        );
        assert_eq!(calls_to(&log, MANAGER_SET_FLAG), vec![vec![manager, 1]]);
        // the known sequence is activated and loaded
        assert_eq!(calls_to(&log, MANAGER_ACTIVATE).len(), 1);
        let loads = calls_to(&log, SEQUENCE_LOAD_FROM_BUFFER);
        assert_eq!(loads.len(), 2);
        assert_eq!(loads[0], vec![walk, buffer, 1.0f32.to_bits()]);
        // the unknown one goes into a temporary sequence, built and destroyed
        let built = calls_to(&log, SEQUENCE_CONSTRUCT);
        assert_eq!(built.len(), 1);
        assert_eq!(loads[1][0], built[0][0]);
        assert_eq!(calls_to(&log, SEQUENCE_DESTRUCT), built);
        // not an actor's 3D: only the final record, with the blend time
        assert!(calls_to(&log, ADD_3D_VELOCITY).is_empty());
        assert_eq!(calls_to(&log, RESET_SIM), vec![vec![node, 1]]);
        let made = calls_to(&log, MAKE_VELOCITY);
        assert_eq!(made.len(), 1);
        assert_eq!(
            (made[0][1], made[0][2], made[0][3]),
            (1.0f32.to_bits(), 1, 0)
        );
    }

    #[test]
    fn fn_00564010_steps_an_actors_3d_through_the_sequence() {
        let (mut e, buffer, manager, _, node) = load_fixture(true);
        let log = logged(&mut e, |e| {
            e.call(0x0056_4010, &args![manager, buffer, 1.0f32]);
        });
        // duration 2 + blend 1 = 3; from 0 in steps of 0.15 up to 1
        let made = calls_to(&log, MAKE_VELOCITY);
        let times: Vec<f32> = made.iter().map(|m| f32::from_bits(m[1])).collect();
        assert_eq!(calls_to(&log, ADD_3D_VELOCITY).len(), 7);
        assert_eq!(times.len(), 8);
        assert_eq!(times[0], 0.0);
        assert!((times[6] - 0.9).abs() < 1e-5);
        assert!(calls_to(&log, ADD_3D_VELOCITY)
            .iter()
            .all(|call| call[0] == node));
        // the last record is the blend time
        assert_eq!(times[7], 1.0);
    }

    #[test]
    fn fn_00564010_without_a_manager_only_consumes_the_data() {
        let (mut e, buffer, _, _, node) = load_fixture(true);
        let log = logged(&mut e, |e| {
            e.call(0x0056_4010, &args![0u32, buffer, -1.0f32]);
        });
        assert!(calls_to(&log, MANAGER_DEACTIVATE_ALL).is_empty());
        // both entries go to temporaries, nothing was found: no record
        assert_eq!(calls_to(&log, SEQUENCE_CONSTRUCT).len(), 2);
        assert!(calls_to(&log, RESET_SIM).is_empty());
        let _ = node;
        // the default blend stands for -1.0
        assert_eq!(
            calls_to(&log, SEQUENCE_LOAD_FROM_BUFFER)[0][2],
            0.25f32.to_bits()
        );
    }

    // ---- 00564360 ---------------------------------------------------------

    #[test]
    fn fn_00564360_classifies_the_base_form_of_the_owning_reference() {
        let mut e = engine();
        let base_kind = |e: &mut Engine, kind: u8| {
            let base = form(e, kind, 1, 0);
            refr_with(e, base, 0, &[]).addr()
        };
        // 0056f930 finds the reference owning a 3D node; here the node is
        // the reference itself
        e.register(0x0056_f930, |_, a| a[0].into_ret());
        for (kind, class) in [
            (0x15u8, 1u32),
            (0x1c, 1),
            (0x20, 1),
            (0x21, 1),
            (0x1b, 2),
            (0x27, 2),
            (0x2a, 3),
            (0x2b, 3),
            (0x18, 4),
            (0x1a, 4),
            (0x28, 4),
            (0x29, 4),
            (0x1d, 5),
            (0x74, 5),
            (0x67, 5),
            (0x16, 6),
            (0x75, 6),
            (0x01, 6),
        ] {
            let node = base_kind(&mut e, kind);
            assert_eq!(
                e.call(0x0056_4360, &args![node]).u32(),
                class,
                "type {kind:#x}"
            );
        }
        // no node, no reference or no base form
        assert_eq!(e.call(0x0056_4360, &args![0u32]).u32(), 0);
        let bare = refr_with(&mut e, 0, 0, &[]);
        assert_eq!(e.call(0x0056_4360, &args![bare]).u32(), 0);
        returns(&mut e, 0x0056_f930, 0);
        let node = base_kind(&mut e, 0x15);
        assert_eq!(e.call(0x0056_4360, &args![node]).u32(), 0);
    }

    // ---- 00564480 ---------------------------------------------------------

    #[test]
    fn create_reference_allocates_and_constructs_by_kind() {
        let mut e = engine();
        e.register(OPERATOR_NEW, |e, a| e.mem.alloc(a[0]).into_ret());
        // every constructor returns its this (+1, so a mix-up shows)
        for ctor in [
            0x0092_eb50u32,
            0x0055_a2f0,
            0x008d_1f40,
            0x008d_4500,
            0x009b_7a50,
            0x009b_3fa0,
            0x0097_8fd0,
            0x009b_3160,
            0x009a_aff0,
        ] {
            e.register(ctor, |_, a| (a[0] + 1).into_ret());
        }
        stub(&mut e, &[SCOPE_GUARD_CTOR, SCOPE_GUARD_DTOR]);
        for (kind, size, ctor, takes_flag) in [
            (0x16u8, 0x88u32, 0x0092_eb50u32, false),
            (0x3a, 0x68, 0x0055_a2f0, false),
            (0x3b, 0x1c8, 0x008d_1f40, true),
            (0x3c, 0x1c0, 0x008d_4500, true),
            (0x3d, 0x160, 0x009b_7a50, true),
            (0x3e, 0x154, 0x009b_3fa0, true),
            (0x3f, 0x154, 0x0097_8fd0, true),
            (0x40, 0x158, 0x009b_3160, true),
            (0x69, 0x158, 0x009a_aff0, true),
        ] {
            let log = logged(&mut e, |e| {
                let made = e.call(0x0056_4480, &args![u32::from(kind), 1u32]).u32();
                assert_ne!(made, 0);
                assert_eq!(e.mem.block_size(made - 1), Some(size.div_ceil(8) * 8));
            });
            let constructed = calls_to(&log, ctor);
            assert_eq!(constructed.len(), 1, "kind {kind:#x}");
            assert_eq!(constructed[0].len(), if takes_flag { 2 } else { 1 });
            if takes_flag {
                assert_eq!(constructed[0][1], 1);
            }
            // the scope guard (tag 0x31) surrounds the allocation
            let guard = calls_to(&log, SCOPE_GUARD_CTOR);
            assert_eq!(guard[0][1..], [0x31, 1, SOURCE_FILE, 0x1493]);
            assert_eq!(calls_to(&log, SCOPE_GUARD_DTOR).len(), 1);
        }
        // another kind gives null without allocating
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_4480, &args![0x17u32, 0u32]).u32(), 0);
        });
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        // a failed allocation gives null without constructing
        returns(&mut e, OPERATOR_NEW, 0);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_4480, &args![0x3bu32, 0u32]).u32(), 0);
        });
        assert!(calls_to(&log, 0x008d_1f40).is_empty());
    }

    // ---- 00564820 and 00564900 -------------------------------------------

    #[test]
    fn fn_00564820_picks_the_reference_kind() {
        let mut e = engine();
        e.register(0x004f_d4f0, |e, a| e.mem.u32(a[0] + 0x40).into_ret());
        for (kind, value, expected) in [
            (0x2au8, 0u32, 0x3bu32),
            (0x2b, 0, 0x3c),
            (0x33, 0x10000, 0x3d),
            (0x33, 0x20000, 0x3e),
            (0x33, 0x40000, 0x3f),
            (0x33, 0x80000, 0x40),
            (0x33, 0x100000, 0x69),
            (0x33, 0x1, 0x3a),
            (0x10, 0, 0x3a),
        ] {
            let base = form(&mut e, kind, 1, 0);
            e.mem.set_u32(base + 0x40, value);
            let refr = refr_with(&mut e, base, 0, &[]);
            assert_eq!(
                e.call(0x0056_4820, &args![refr]).u32(),
                expected,
                "type {kind:#x} value {value:#x}"
            );
        }
        let bare = refr_with(&mut e, 0, 0, &[]);
        assert_eq!(e.call(0x0056_4820, &args![bare]).u32(), 0x3a);
    }

    #[test]
    fn fn_00564900_accepts_the_reference_kinds() {
        let mut e = engine();
        for (kind, expected) in [
            (0x39i32, false),
            (0x3a, true),
            (0x40, true),
            (0x41, false),
            (0x69, true),
            (0x6a, false),
            (-1, false),
        ] {
            assert_eq!(
                e.call(0x0056_4900, &args![kind]).bool(),
                expected,
                "kind {kind:#x}"
            );
        }
    }

    // ---- 00564930 ---------------------------------------------------------

    fn delete_fixture() -> (Engine, Ptr<TESObjectREFR>, u32) {
        let mut e = engine();
        let handler = e.mem.alloc(0x20);
        let loading = e.mem.alloc(0x20);
        e.set_global(GLOBAL_DATA_HANDLER, handler);
        e.set_global(GLOBAL_LOADING_OBJECT, loading);
        returns(&mut e, 0x0042_26e0, 0);
        returns(&mut e, 0x0046_9860, 1);
        returns(&mut e, 0x0040_77c0, 0);
        returns(&mut e, 0x0042_ce10, 0);
        stub(&mut e, &[0x0084_a7c0, 0x0048_4530]);
        let refr = refr_of_kind(&mut e, 0);
        (e, refr, loading)
    }

    #[test]
    fn set_delete_tells_the_loading_object_when_every_check_passes() {
        let (mut e, refr, loading) = delete_fixture();
        let log = logged(&mut e, |e| {
            e.call(0x0056_4930, &args![refr, 1u32]);
        });
        assert_eq!(
            calls_to(&log, 0x0084_a7c0),
            vec![vec![loading, refr.addr()]]
        );
        assert_eq!(calls_to(&log, 0x0048_4530), vec![vec![refr.addr(), 1]]);
    }

    #[test]
    fn set_delete_skips_the_notification_when_a_check_fails() {
        for (blocker, value) in [
            (0x0042_26e0u32, 1u32),
            (0x0046_9860, 0),
            (0x0040_77c0, 1),
            (0x0042_ce10, 1),
        ] {
            let (mut e, refr, _) = delete_fixture();
            returns(&mut e, blocker, value);
            let log = logged(&mut e, |e| {
                e.call(0x0056_4930, &args![refr, 1u32]);
            });
            assert!(calls_to(&log, 0x0084_a7c0).is_empty(), "{blocker:#x}");
            assert_eq!(calls_to(&log, 0x0048_4530).len(), 1);
        }
        // an undelete never notifies
        let (mut e, refr, _) = delete_fixture();
        let log = logged(&mut e, |e| {
            e.call(0x0056_4930, &args![refr, 0u32]);
        });
        assert!(calls_to(&log, 0x0084_a7c0).is_empty());
        assert_eq!(calls_to(&log, 0x0048_4530), vec![vec![refr.addr(), 0]]);
    }

    // ---- the flag accessors ----------------------------------------------

    #[test]
    fn persistence_flag_getters_check_the_base_form_type() {
        let mut e = engine();
        // 005649b0 (base absent or type 0x1e) and 00564ab0 (type 0x22)
        for (addr, accepted) in [(0x0056_49b0u32, 0x1eu8), (0x0056_4ab0, 0x22)] {
            for (kind, flags, expected) in [
                (Some(accepted), 0x200u32, true),
                (Some(accepted), 0x1ff, false),
                (Some(0x40), 0x200, false),
                (None, 0x200, true),
                (None, 0, false),
            ] {
                let base = kind.map_or(0, |k| form(&mut e, k, 1, 0));
                let refr = refr_with(&mut e, base, flags, &[]);
                assert_eq!(
                    e.call(addr, &args![refr]).bool(),
                    expected,
                    "{addr:#x} {kind:?} {flags:#x}"
                );
            }
        }
    }

    #[test]
    fn fn_00564a00_sets_the_flag_and_keeps_the_tracked_object_in_step() {
        let mut e = engine();
        let tracked = e.mem.alloc(0x10);
        e.mem.set_u32(tracked, 0x1234);
        let manager = e.mem.alloc(0x10);
        returns(&mut e, 0x0057_25f0, tracked);
        returns(&mut e, 0x0045_0b80, manager);
        e.register(0x0055_9450, |e, a| e.mem.u32(a[0]).into_ret());
        stub(&mut e, &[0x00b4_f9a0, 0x00b5_c450]);
        let base = form(&mut e, 0x1e, 1, 0);
        let refr = refr_with(&mut e, base, 0x41, &[]);

        let log = logged(&mut e, |e| {
            e.call(0x0056_4a00, &args![refr, 1u32]);
        });
        assert_eq!(e.mem.u32(refr.addr() + 8), 0x241);
        assert!(calls_to(&log, 0x00b4_f9a0).is_empty());
        assert_eq!(calls_to(&log, 0x00b5_c450), vec![vec![manager, 0x1234, 1]]);
        assert_eq!(calls_to(&log, 0x0045_0b80), vec![vec![0]]);

        // clearing also calls 00b4f9a0 first
        let log = logged(&mut e, |e| {
            e.call(0x0056_4a00, &args![refr, 0u32]);
        });
        assert_eq!(e.mem.u32(refr.addr() + 8), 0x41);
        let order: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        let cleanup = order.iter().position(|a| *a == 0x00b4_f9a0).unwrap();
        let notify = order.iter().position(|a| *a == 0x00b5_c450).unwrap();
        assert!(cleanup < notify);
        assert_eq!(calls_to(&log, 0x00b5_c450), vec![vec![manager, 0x1234, 0]]);

        // nothing tracked: only the flags change
        returns(&mut e, 0x0057_25f0, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_4a00, &args![refr, 1u32]);
        });
        assert_eq!(e.mem.u32(refr.addr() + 8), 0x241);
        assert!(calls_to(&log, 0x00b5_c450).is_empty());

        // another base form type does nothing
        let other = form(&mut e, 0x40, 1, 0);
        let refr = refr_with(&mut e, other, 0x41, &[]);
        e.call(0x0056_4a00, &args![refr, 1u32]);
        assert_eq!(e.mem.u32(refr.addr() + 8), 0x41);
    }

    #[test]
    fn flag_setters_change_one_bit_under_their_conditions() {
        let mut e = engine();
        let actor_slot: [(u32, AbiFn); 1] = [(0x100, |_, _| true.into_ret())];
        let plain_slot: [(u32, AbiFn); 1] = [(0x100, |_, _| false.into_ret())];
        // 00564b00: bit 0x200 for a base form of type 0x22 (or none)
        for (kind, set, flags, expected) in [
            (Some(0x22u8), 1u32, 0u32, 0x200u32),
            (Some(0x22), 0, 0x3ff, 0x1ff),
            (None, 1, 0, 0x200),
            (Some(0x23), 1, 0, 0),
        ] {
            let base = kind.map_or(0, |k| form(&mut e, k, 1, 0));
            let refr = refr_with(&mut e, base, flags, &[]);
            e.call(0x0056_4b00, &args![refr, set]);
            assert_eq!(e.mem.u32(refr.addr() + 8), expected, "{kind:?}");
        }
        // 00564c60: bit 0x100 unless there is no base form or it is type 0x1c
        for (kind, set, flags, expected) in [
            (Some(0x20u8), 1u32, 0u32, 0x100u32),
            (Some(0x20), 0, 0x1ff, 0xff),
            (Some(0x1c), 1, 0, 0),
            (None, 1, 0, 0),
        ] {
            let base = kind.map_or(0, |k| form(&mut e, k, 1, 0));
            let refr = refr_with(&mut e, base, flags, &[]);
            e.call(0x0056_4c60, &args![refr, set]);
            assert_eq!(e.mem.u32(refr.addr() + 8), expected, "{kind:?}");
        }
        // 00564d20: bit 0x100000 for a reference that is not an actor
        let refr = refr_with(&mut e, 0, 0, &plain_slot);
        e.call(0x0056_4d20, &args![refr, 1u32]);
        assert_eq!(e.mem.u32(refr.addr() + 8), 0x10_0000);
        e.call(0x0056_4d20, &args![refr, 0u32]);
        assert_eq!(e.mem.u32(refr.addr() + 8), 0);
        let actor = refr_with(&mut e, 0, 0, &actor_slot);
        e.call(0x0056_4d20, &args![actor, 1u32]);
        assert_eq!(e.mem.u32(actor.addr() + 8), 0);
        // 00564db0: bit 0x40000, whatever the reference
        e.call(0x0056_4db0, &args![actor, 5u32]);
        assert_eq!(e.mem.u32(actor.addr() + 8), 0x4_0000);
        e.call(0x0056_4db0, &args![actor, 0u32]);
        assert_eq!(e.mem.u32(actor.addr() + 8), 0);
    }

    #[test]
    fn flag_getters_read_one_bit_under_their_conditions() {
        let mut e = engine();
        // 00564c10: bit 0x100 unless there is no base form or it is type 0x1c
        for (kind, flags, expected) in [
            (Some(0x20u8), 0x100u32, true),
            (Some(0x20), 0xff, false),
            (Some(0x1c), 0x100, false),
            (None, 0x100, false),
        ] {
            let base = kind.map_or(0, |k| form(&mut e, k, 1, 0));
            let refr = refr_with(&mut e, base, flags, &[]);
            assert_eq!(e.call(0x0056_4c10, &args![refr]).bool(), expected);
        }
        // 00564cd0: bit 0x100000, false for an actor
        let plain = refr_with(&mut e, 0, 0x10_0000, &[(0x100, |_, _| false.into_ret())]);
        let actor = refr_with(&mut e, 0, 0x10_0000, &[(0x100, |_, _| true.into_ret())]);
        assert!(e.call(0x0056_4cd0, &args![plain]).bool());
        assert!(!e.call(0x0056_4cd0, &args![actor]).bool());
        e.mem.set_u32(plain.addr() + 8, 0xf_ffff);
        assert!(!e.call(0x0056_4cd0, &args![plain]).bool());
        // 00564d80 (bit 0x40000) and 00565450 (bit 0x400)
        let refr = refr_with(&mut e, 0, 0x4_0400, &[]);
        assert!(e.call(0x0056_4d80, &args![refr]).bool());
        assert!(e.call(0x0056_5450, &args![refr]).bool());
        e.mem.set_u32(refr.addr() + 8, 0x3_fbff);
        assert!(!e.call(0x0056_4d80, &args![refr]).bool());
        assert!(!e.call(0x0056_5450, &args![refr]).bool());
    }

    #[test]
    fn base_form_virtuals_are_forwarded_and_default_to_false() {
        let mut e = engine();
        // the base form answers through its virtuals +0xa8, +0xb0 and +0x94
        let base = object_with_vtable(
            &mut e,
            0x100,
            &[
                (0xa8, |_, _| true.into_ret()),
                (0xb0, |_, _| false.into_ret()),
                (0x94, |_, _| true.into_ret()),
            ],
        );
        let refr = refr_with(&mut e, base, 0, &[]);
        let bare = refr_with(&mut e, 0, 0, &[]);
        assert!(e.call(0x0056_4b70, &args![refr]).bool());
        assert!(!e.call(0x0056_4bc0, &args![refr]).bool());
        assert!(e.call(0x0056_5690, &args![refr]).bool());
        for addr in [0x0056_4b70u32, 0x0056_4bc0, 0x0056_5690] {
            assert!(!e.call(addr, &args![bare]).bool());
        }
    }

    #[test]
    fn fn_00565050_tests_bit_4_of_the_object_it_finds() {
        let mut e = engine();
        e.register(0x0052_7080, |e, a| {
            assert_eq!(a[1], 0x92);
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        let refr = refr_with(&mut e, 0, 0, &[]);
        assert!(!e.call(0x0056_5050, &args![refr]).bool());
        let found = form(&mut e, 0, 5, 0);
        e.mem.set_u32(refr.addr() + 0x30, found);
        assert!(e.call(0x0056_5050, &args![refr]).bool());
        e.mem.set_u32(found + 0xc, 3);
        assert!(!e.call(0x0056_5050, &args![refr]).bool());
    }

    // ---- 00565260 ---------------------------------------------------------

    #[test]
    fn must_ref_persist_goes_by_the_base_form() {
        let mut e = engine();
        returns(&mut e, 0x004f_ede0, 0);
        returns(&mut e, 0x0051_94d0, 0);
        returns(&mut e, 0x0056_8e50, 0);
        returns(&mut e, 0x0055_e200, 0);
        let check = |e: &mut Engine, kind: u8, id: u32| {
            let base = form(e, kind, id, 0);
            let refr = refr_with(e, base, 0, &[]);
            e.call(0x0056_5260, &args![refr]).bool()
        };
        // the listed form IDs persist whatever the type
        for id in [4u32, 5, 6, 0x10, 0x12, 0x15, 0x1f, 0x23, 0x24, 0x34, 0x3b] {
            assert!(check(&mut e, 0x01, id), "id {id:#x}");
        }
        for id in [7u32, 0x0f, 0x11, 0x3c, 0x1000] {
            assert!(!check(&mut e, 0x01, id), "id {id:#x}");
        }
        // types 4 and 0x23 always persist
        assert!(check(&mut e, 4, 0x1000));
        assert!(check(&mut e, 0x23, 0x1000));
        // type 0x16 asks 004fede0(base, 0)
        assert!(!check(&mut e, 0x16, 0x1000));
        returns(&mut e, 0x004f_ede0, 1);
        assert!(check(&mut e, 0x16, 0x1000));
        // type 0x1c asks 005194d0(base), then 00568e50(reference)
        assert!(!check(&mut e, 0x1c, 0x1000));
        returns(&mut e, 0x0056_8e50, 1);
        assert!(check(&mut e, 0x1c, 0x1000));
        returns(&mut e, 0x0056_8e50, 0);
        returns(&mut e, 0x0051_94d0, 1);
        assert!(check(&mut e, 0x1c, 0x1000));
        // types 0x2a and 0x2b ask 0055e200(base + 0x30)
        assert!(!check(&mut e, 0x2a, 0x1000));
        returns(&mut e, 0x0055_e200, 1);
        let base = form(&mut e, 0x2b, 0x1000, 0);
        let refr = refr_with(&mut e, base, 0, &[]);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0056_5260, &args![refr]).bool());
        });
        assert_eq!(calls_to(&log, 0x0055_e200), vec![vec![base + 0x30]]);
        // no base form: false
        let bare = refr_with(&mut e, 0, 0, &[]);
        assert!(!e.call(0x0056_5260, &args![bare]).bool());
    }

    // ---- 005653d0 ---------------------------------------------------------

    #[test]
    fn get_ref_persists_trusts_the_loaded_state_unless_a_check_objects() {
        let mut e = engine();
        let handler = e.mem.alloc(0x20);
        let save_load = e.mem.alloc(0x20);
        e.set_global(GLOBAL_DATA_HANDLER, handler);
        e.set_global(GLOBAL_SAVE_LOAD_GAME, save_load);
        let found = e.mem.alloc(0x20);
        returns(&mut e, 0x0048_4e60, found);
        for addr in [0x0045_16b0u32, 0x0047_c850, 0x0042_26e0, 0x0047_1c20] {
            returns(&mut e, addr, 0);
        }
        let refr = refr_with(&mut e, 0, 0, &[]);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0056_53d0, &args![refr]).bool());
        });
        assert_eq!(
            calls_to(&log, 0x0048_4e60),
            vec![vec![refr.addr(), u32::MAX]]
        );
        assert_eq!(calls_to(&log, 0x0045_16b0), vec![vec![handler]]);
        assert_eq!(calls_to(&log, 0x0047_c850), vec![vec![save_load]]);
        assert_eq!(calls_to(&log, 0x0047_1c20), vec![vec![found]]);
        // any check that objects falls back to the flag (bit 0x400)
        for addr in [0x0045_16b0u32, 0x0047_c850, 0x0042_26e0, 0x0047_1c20] {
            returns(&mut e, addr, 1);
            assert!(!e.call(0x0056_53d0, &args![refr]).bool());
            e.mem.set_u32(refr.addr() + 8, 0x400);
            assert!(e.call(0x0056_53d0, &args![refr]).bool());
            e.mem.set_u32(refr.addr() + 8, 0);
            returns(&mut e, addr, 0);
        }
        // nothing found: the flag decides
        returns(&mut e, 0x0048_4e60, 0);
        assert!(!e.call(0x0056_53d0, &args![refr]).bool());
        e.mem.set_u32(refr.addr() + 8, 0x400);
        assert!(e.call(0x0056_53d0, &args![refr]).bool());
    }

    // ---- 00565480 ---------------------------------------------------------

    fn persist_fixture() -> (Engine, Ptr<TESObjectREFR>, u32, u32) {
        let mut e = engine();
        stub(&mut e, &[0x0058_7ff0, 0x0058_8030]);
        let cell = e.mem.alloc(0x40);
        let list = e.mem.alloc(0x40);
        returns(&mut e, 0x0042_5fd0, 0);
        returns(&mut e, 0x0055_16c0, 0);
        returns(&mut e, 0x0054_ddd0, list);
        let refr = refr_with(
            &mut e,
            0,
            0,
            &[(0x48, |_, _| Ret::default()), (0xc8, |_, _| Ret::default())],
        );
        // the TESChildCell base at +0x18 has its own vtable whose slot 0
        // gives the cell stored right after the vtable pointer
        let child = object_with_vtable(&mut e, 0x10, &[(0, |e, a| e.mem.u32(a[0] + 4).into_ret())]);
        let child_vtable = e.mem.u32(child);
        e.mem.set_u32(refr.addr() + 0x18, child_vtable);
        e.mem.set_u32(refr.addr() + 0x1c, cell);
        (e, refr, cell, list)
    }

    #[test]
    fn set_ref_persists_adds_the_reference_to_the_cells_persistent_list() {
        let (mut e, refr, cell, list) = persist_fixture();
        let log = logged(&mut e, |e| {
            e.call(0x0056_5480, &args![refr, 1u32]);
        });
        assert_eq!(e.mem.u32(refr.addr() + 8), 0x400);
        assert_eq!(calls_to(&log, 0x0042_5fd0), vec![vec![cell]]);
        assert_eq!(calls_to(&log, 0x0054_ddd0), vec![vec![cell]]);
        assert_eq!(calls_to(&log, 0x0058_7ff0), vec![vec![list, refr.addr()]]);
        assert!(calls_to(&log, 0x0058_8030).is_empty());
        // a cell that fails one of the checks is left alone
        returns(&mut e, 0x0042_5fd0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5480, &args![refr, 1u32]);
        });
        assert!(calls_to(&log, 0x0058_7ff0).is_empty());
    }

    #[test]
    fn set_ref_persists_removes_the_reference_when_cleared() {
        let (mut e, refr, _, list) = persist_fixture();
        e.mem.set_u32(refr.addr() + 8, 0x400);
        // 005516c0 false: nothing to remove, only the flag changes
        let log = logged(&mut e, |e| {
            e.call(0x0056_5480, &args![refr, 0u32]);
        });
        assert_eq!(e.mem.u32(refr.addr() + 8), 0);
        assert!(calls_to(&log, 0x0058_8030).is_empty());
        returns(&mut e, 0x0055_16c0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5480, &args![refr, 0u32]);
        });
        assert_eq!(calls_to(&log, 0x0058_8030), vec![vec![list, refr.addr()]]);
        assert!(calls_to(&log, 0x0058_7ff0).is_empty());
    }

    // ---- 00565580 ---------------------------------------------------------

    /// A 3D node: +0x10 its controller, +0x14 its children container (+0 the
    /// count, the children from +4), +0x20 the head of its linked list of
    /// attached objects (cells of `next, item`).
    fn node(e: &mut Engine, controller: u32, items: &[u32], children: &[u32]) -> u32 {
        let node = object_with_vtable(e, 0x40, &[(0xc, |e, a| e.mem.u32(a[0] + 0x14).into_ret())]);
        e.mem.set_u32(node + 0x10, controller);
        let mut head = 0;
        for item in items.iter().rev() {
            let cell = e.mem.alloc(8);
            e.mem.set_u32(cell, head);
            e.mem.set_u32(cell + 4, *item);
            head = cell;
        }
        e.mem.set_u32(node + 0x20, head);
        if !children.is_empty() {
            let container = e.mem.alloc(0x40);
            e.mem.set_u32(container, children.len() as u32);
            for (index, child) in children.iter().enumerate() {
                e.mem.set_u32(container + 4 + 4 * index as u32, *child);
            }
            e.mem.set_u32(node + 0x14, container);
        }
        node
    }

    fn controller_api(e: &mut Engine) {
        e.register(GET_CONTROLLER, |e, a| e.mem.u32(a[0] + 0x10).into_ret());
        e.register(0x0043_0830, |_, a| (a[0] + 0x20).into_ret());
        e.register(0x0055_9450, |e, a| e.mem.u32(a[0]).into_ret());
        // 0057cbe0(list, &position): the item slot at the position, which
        // moves on to the next cell
        e.register(0x0057_cbe0, |e, a| {
            let cell = e.mem.u32(a[1]);
            let next = e.mem.u32(cell);
            e.mem.set_u32(a[1], next);
            (cell + 4).into_ret()
        });
        e.register(0x0043_b480, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(0x0043_b4a0, |e, a| {
            e.mem.u32(a[0] + 4 + 4 * a[1]).into_ret()
        });
    }

    #[test]
    fn has_time_controllers_searches_the_node_its_list_and_its_children() {
        let mut e = engine();
        controller_api(&mut e);
        let with_controller = node(&mut e, 1, &[], &[]);
        let bare = node(&mut e, 0, &[], &[]);
        // the node's own controller
        assert!(e.call(0x0056_5580, &args![with_controller]).bool());
        // an attached object with a controller, found after one without
        let in_list = node(&mut e, 0, &[bare, with_controller], &[]);
        assert!(e.call(0x0056_5580, &args![in_list]).bool());
        // a deeper child
        let child = node(&mut e, 0, &[], &[bare, with_controller]);
        let parent = node(&mut e, 0, &[bare], &[bare, child]);
        assert!(e.call(0x0056_5580, &args![parent]).bool());
        // nothing anywhere
        let empty_child = node(&mut e, 0, &[bare], &[bare]);
        let nothing = node(&mut e, 0, &[bare, bare], &[bare, empty_child]);
        assert!(!e.call(0x0056_5580, &args![nothing]).bool());
        assert!(!e.call(0x0056_5580, &args![0u32]).bool());
    }

    #[test]
    fn has_time_controllers_stops_at_the_first_controller() {
        let mut e = engine();
        controller_api(&mut e);
        let with_controller = node(&mut e, 1, &[], &[]);
        let bare = node(&mut e, 0, &[], &[]);
        let parent = node(&mut e, 0, &[with_controller], &[bare]);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0056_5580, &args![parent]).bool());
        });
        // the list item decided: the children were never asked
        assert!(calls_to(&log, 0x0043_b480).is_empty());
    }

    // ---- 00565670, 005656d0 -----------------------------------------------

    #[test]
    fn fn_00565670_removes_the_key_from_the_map_at_011ca0e0() {
        let mut e = engine();
        e.register(0x0040_5430, |_, a| (a[1] == 7).into_ret());
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0056_5670, &args![7u32]).bool());
            assert!(!e.call(0x0056_5670, &args![8u32]).bool());
        });
        assert_eq!(
            calls_to(&log, 0x0040_5430),
            vec![vec![0x011c_a0e0, 7], vec![0x011c_a0e0, 8]]
        );
    }

    #[test]
    fn fn_005656d0_accepts_the_base_form_or_the_inventory_changes() {
        let mut e = engine();
        let accepted = form(&mut e, 0x40, 1, 0);
        let refr = refr_with(&mut e, accepted, 0, &[]);
        let changes = e.mem.alloc(0x10);
        // 004826d0 accepts an object when it is the one made above
        e.register_double(0x0048_26d0, move |_, a| Ret {
            eax: u32::from(a[0] == accepted),
            ..Ret::default()
        });
        returns(&mut e, 0x0055_d310, 1);
        returns(&mut e, 0x004b_f220, changes);
        returns(&mut e, 0x004d_0490, 1);
        // the base form is accepted: no need to look further
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0056_56d0, &args![refr]).bool());
        });
        assert!(calls_to(&log, 0x0055_d310).is_empty());
        // otherwise a container whose changes agree
        let other = form(&mut e, 0x40, 2, 0);
        e.mem.set_u32(refr.addr() + 0x20, other);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0056_56d0, &args![refr]).bool());
        });
        assert_eq!(calls_to(&log, 0x004b_f220), vec![vec![refr.addr()]]);
        assert_eq!(calls_to(&log, 0x004d_0490), vec![vec![changes]]);
        returns(&mut e, 0x004d_0490, 0);
        assert!(!e.call(0x0056_56d0, &args![refr]).bool());
        returns(&mut e, 0x0055_d310, 0);
        assert!(!e.call(0x0056_56d0, &args![refr]).bool());
    }

    // ---- a reference with the virtuals the rest of the tests steer ---------

    /// A virtual that answers with the dword at `OFFSET` of its object, so a
    /// test steers the answer through memory.
    fn slot_value<const OFFSET: u32>(e: &mut Engine, a: &[u32]) -> Ret {
        e.mem.u32(a[0] + OFFSET).into_ret()
    }

    /// The virtual +0x22c takes a flag: it answers with the dword at `0xa0`
    /// for 0 and at `0xa4` for 1.
    fn slot_22c(e: &mut Engine, a: &[u32]) -> Ret {
        e.mem.u32(a[0] + 0xa0 + 4 * a[1]).into_ret()
    }

    /// Where `full_refr` keeps the answers of its virtuals.
    const IS_ACTOR_ANSWER: u32 = 0x70;
    const GET_3D_ANSWER: u32 = 0x74;
    const BIPED_ANSWER: u32 = 0x78;
    const ANIMATION_ANSWER: u32 = 0x7c;
    const SLOT_21C_ANSWER: u32 = 0x80;
    const SLOT_218_ANSWER: u32 = 0x84;
    const SLOT_22C_ARG0_ANSWER: u32 = 0xa0;
    const SLOT_22C_ARG1_ANSWER: u32 = 0xa4;
    const SLOT_1A0_ANSWER: u32 = 0x8c;
    const SLOT_FC_ANSWER: u32 = 0x90;
    const SCRIPT_ANSWER: u32 = 0x94;

    /// A reference over `base` whose virtuals `IsActor` (+0x100), `Get3D`
    /// (+0x1d0), the biped (+0x1e8), the animation (+0x1e4), +0x21c, +0x218,
    /// +0x22c, +0x1a0, +0xfc and the script (+0x1a4) answer with the dwords
    /// the constants above name, and +0x48, +0x140, +0x178 do nothing.
    fn full_refr(e: &mut Engine, base: u32) -> Ptr<TESObjectREFR> {
        let slots: [(u32, AbiFn); 13] = [
            (0x100, slot_value::<IS_ACTOR_ANSWER>),
            (0x1d0, slot_value::<GET_3D_ANSWER>),
            (0x1e8, slot_value::<BIPED_ANSWER>),
            (0x1e4, slot_value::<ANIMATION_ANSWER>),
            (0x21c, slot_value::<SLOT_21C_ANSWER>),
            (0x218, slot_value::<SLOT_218_ANSWER>),
            (0x22c, slot_22c),
            (0x1a0, slot_value::<SLOT_1A0_ANSWER>),
            (0xfc, slot_value::<SLOT_FC_ANSWER>),
            (0x1a4, slot_value::<SCRIPT_ANSWER>),
            (0x48, |_, _| Ret::default()),
            (0x140, |_, _| Ret::default()),
            (0x178, |_, _| Ret::default()),
        ];
        refr_with(e, base, 0, &slots)
    }

    /// The fake address a vtable slot of `object` points to (for call logs).
    fn slot_address(e: &Engine, object: u32, offset: u32) -> u32 {
        let vtable = e.mem.u32(object);
        e.mem.u32(vtable + offset)
    }

    /// A base form that is an object with a vtable (name at +0x130).
    fn named_base(e: &mut Engine, kind: u8, id: u32, name: &str) -> u32 {
        let text = cstr(e, name);
        let base = object_with_vtable(e, 0x200, &[(0x130, slot_value::<0x100>)]);
        e.mem.set_u8(base + 4, kind);
        e.mem.set_u32(base + 0xc, id);
        e.mem.set_u32(base + 0x100, text);
        base
    }

    // ---- 00565730 ---------------------------------------------------------

    fn init_script_fixture() -> (Engine, Ptr<TESObjectREFR>) {
        let mut e = engine();
        stub(
            &mut e,
            &[
                SCOPE_GUARD_CTOR,
                SCOPE_GUARD_DTOR,
                0x0041_9ed0,
                0x0041_9f80,
                0x004d_1960,
            ],
        );
        // 004826d0: the script of a form; here the form itself
        e.register(0x0048_26d0, |_, a| a[0].into_ret());
        e.register(0x005a_bf60, |_, a| (a[0] + 1).into_ret());
        returns(&mut e, 0x0041_0220, 0);
        returns(&mut e, 0x0055_d310, 0);
        let base = form(&mut e, 0x40, 1, 0);
        let refr = full_refr(&mut e, base);
        (e, refr)
    }

    #[test]
    fn init_script_stores_the_script_found_for_the_reference() {
        let (mut e, refr) = init_script_fixture();
        let me = refr.addr();
        let base = e.mem.u32(me + 0x20);
        let script = form(&mut e, 0x11, 9, 0);
        // the virtual +0x1a4 gives the script
        e.mem.set_u32(me + SCRIPT_ANSWER, script);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5730, &args![refr]);
        });
        let list = me + 0x44;
        assert_eq!(calls_to(&log, 0x0041_9ed0), vec![vec![list, script]]);
        assert_eq!(calls_to(&log, 0x0041_9f80), vec![vec![list, script + 1]]);
        // the guard (tag 0x15) surrounds it
        let guard = calls_to(&log, SCOPE_GUARD_CTOR);
        assert_eq!(guard[0][1..], [0x15, 1, SOURCE_FILE, 0x196e]);
        assert_eq!(calls_to(&log, SCOPE_GUARD_DTOR).len(), 1);
        // without a script of its own the base form is used
        e.mem.set_u32(me + SCRIPT_ANSWER, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5730, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0041_9ed0), vec![vec![list, base]]);
    }

    #[test]
    fn init_script_does_nothing_when_deleted_or_already_scripted() {
        let (mut e, refr) = init_script_fixture();
        let me = refr.addr();
        e.mem.set_u32(me + SCRIPT_ANSWER, 0x1000);
        // a script extra with a script instance (+0xc) is left alone
        let extra = e.mem.alloc(0x20);
        e.mem.set_u32(extra + 0xc, 5);
        returns(&mut e, 0x0041_0220, extra);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5730, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0041_0220), vec![vec![me + 0x44, 0xd]]);
        assert!(calls_to(&log, 0x0041_9ed0).is_empty());
        assert_eq!(calls_to(&log, SCOPE_GUARD_DTOR).len(), 1);
        // one without an instance is not
        e.mem.set_u32(extra + 0xc, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5730, &args![refr]);
        });
        assert_eq!(calls_to(&log, 0x0041_9ed0).len(), 1);
        // a deleted reference does nothing, not even the guard
        e.mem.set_u32(me + 8, 0x20);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5730, &args![refr]);
        });
        assert!(calls_to(&log, SCOPE_GUARD_CTOR).is_empty());
    }

    #[test]
    fn init_script_initialises_the_inventory_changes_of_a_container() {
        let (mut e, refr) = init_script_fixture();
        let changes = e.mem.alloc(0x10);
        returns(&mut e, 0x0055_d310, 1);
        returns(&mut e, 0x004b_f220, changes);
        // no script at all
        e.register(0x0048_26d0, |_, _| Ret::default());
        let log = logged(&mut e, |e| {
            e.call(0x0056_5730, &args![refr]);
        });
        assert!(calls_to(&log, 0x0041_9ed0).is_empty());
        assert_eq!(calls_to(&log, 0x004d_1960), vec![vec![changes]]);
    }

    // ---- 00565870 ---------------------------------------------------------

    fn run_script_fixture(kind: u8) -> (Engine, Ptr<TESObjectREFR>) {
        let mut e = engine();
        let base = form(&mut e, kind, 1, 0);
        let refr = full_refr(&mut e, base);
        returns(&mut e, 0x008d_8520, 0);
        returns(&mut e, 0x0093_1850, 0);
        returns(&mut e, 0x0055_d310, 0);
        returns(&mut e, 0x004b_f220, 0);
        returns(&mut e, 0x004d_2480, 0);
        returns(&mut e, 0x0041_8800, 0);
        returns(&mut e, 0x0041_8830, 0);
        returns(&mut e, 0x005a_c1e0, 0);
        returns(&mut e, 0x0042_ede0, 0);
        (e, refr)
    }

    #[test]
    fn run_script_runs_the_reference_script_and_reports_it() {
        let (mut e, refr) = run_script_fixture(0x40);
        let me = refr.addr();
        let script = e.mem.alloc(0x10);
        let variables = e.mem.alloc(0x10);
        returns(&mut e, 0x0041_8800, script);
        returns(&mut e, 0x0041_8830, variables);
        returns(&mut e, 0x005a_c1e0, 1);
        let log = logged(&mut e, |e| {
            assert!(e.call(0x0056_5870, &args![refr]).bool());
        });
        assert_eq!(
            calls_to(&log, 0x005a_c1e0),
            vec![vec![script, me, variables, 0, 0]]
        );
        // a script that did not run, with variables, flags the reference
        returns(&mut e, 0x005a_c1e0, 0);
        let flagged = slot_address(&e, me, 0x48);
        let log = logged(&mut e, |e| {
            assert!(!e.call(0x0056_5870, &args![refr]).bool());
        });
        assert_eq!(calls_to(&log, flagged), vec![vec![me, 0x8000_0000]]);
        // ... without variables it does not
        returns(&mut e, 0x0041_8830, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5870, &args![refr]);
        });
        assert!(calls_to(&log, flagged).is_empty());
    }

    #[test]
    fn run_script_calls_the_say_to_handler_unless_the_virtual_fc_holds() {
        let (mut e, refr) = run_script_fixture(0x40);
        let me = refr.addr();
        let script = e.mem.alloc(0x10);
        returns(&mut e, 0x0041_8800, script);
        returns(&mut e, 0x0042_ede0, 5);
        let handler = slot_address(&e, me, 0x140);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5870, &args![refr]);
        });
        assert_eq!(calls_to(&log, handler), vec![vec![me]]);
        e.mem.set_u32(me + SLOT_FC_ANSWER, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5870, &args![refr]);
        });
        assert!(calls_to(&log, handler).is_empty());
        // no script: none of it
        returns(&mut e, 0x0041_8800, 0);
        e.mem.set_u32(me + SLOT_FC_ANSWER, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_5870, &args![refr]);
        });
        assert!(calls_to(&log, handler).is_empty());
    }

    #[test]
    fn run_script_runs_the_inventory_scripts_for_containers_of_some_types() {
        for (kind, expected_container_scan) in [(0x1bu8, true), (0x2a, true), (0x40, false)] {
            let (mut e, refr) = run_script_fixture(kind);
            let me = refr.addr();
            let changes = e.mem.alloc(0x10);
            returns(&mut e, 0x0055_d310, 1);
            returns(&mut e, 0x004b_f220, changes);
            returns(&mut e, 0x004d_2480, 1);
            // the process of an actor-like reference has a 3D (virtual +0x1d0)
            returns(&mut e, 0x008d_8520, 1);
            e.mem.set_u32(me + GET_3D_ANSWER, 0x55);
            let log = logged(&mut e, |e| {
                assert_eq!(
                    e.call(0x0056_5870, &args![refr]).bool(),
                    expected_container_scan,
                    "type {kind:#x}"
                );
            });
            assert_eq!(
                calls_to(&log, 0x004d_2480),
                if expected_container_scan {
                    vec![vec![changes, me]]
                } else {
                    vec![]
                }
            );
        }
    }

    #[test]
    fn run_script_gives_up_on_an_actor_like_reference_without_3d() {
        let (mut e, refr) = run_script_fixture(0x2b);
        let me = refr.addr();
        returns(&mut e, 0x008d_8520, 1);
        returns(&mut e, 0x0055_d310, 1);
        returns(&mut e, 0x004b_f220, 0x99);
        returns(&mut e, 0x004d_2480, 1);
        // no 3D (virtual +0x1d0 gives 0): nothing runs
        let log = logged(&mut e, |e| {
            assert!(!e.call(0x0056_5870, &args![refr]).bool());
        });
        assert!(calls_to(&log, 0x004d_2480).is_empty());
        assert!(calls_to(&log, 0x0041_8800).is_empty());
        // 00931850 true, or a 3D, lets it go on
        returns(&mut e, 0x0093_1850, 1);
        assert!(e.call(0x0056_5870, &args![refr]).bool());
        returns(&mut e, 0x0093_1850, 0);
        e.mem.set_u32(me + GET_3D_ANSWER, 5);
        assert!(e.call(0x0056_5870, &args![refr]).bool());
        // no process at all: it goes on too
        e.mem.set_u32(me + GET_3D_ANSWER, 0);
        returns(&mut e, 0x008d_8520, 0);
        assert!(e.call(0x0056_5870, &args![refr]).bool());
        // deleted, or without a base form: nothing
        e.mem.set_u32(me + 8, 0x20);
        assert!(!e.call(0x0056_5870, &args![refr]).bool());
        e.mem.set_u32(me + 8, 0);
        e.mem.set_u32(me + 0x20, 0);
        assert!(!e.call(0x0056_5870, &args![refr]).bool());
    }

    // ---- 00566910, 00566930 -----------------------------------------------

    #[test]
    fn fn_00566910_reads_the_byte_at_0x68() {
        let mut e = engine();
        let object = e.mem.alloc(0x80);
        assert_eq!(e.call(0x0056_6910, &args![object]).u8(), 0);
        e.mem.set_u8(object + 0x68, 0x5a);
        assert_eq!(e.call(0x0056_6910, &args![object]).u8(), 0x5a);
    }

    #[test]
    fn fn_00566930_passes_the_flag_and_0x40_on() {
        let mut e = engine();
        stub(&mut e, &[0x0047_aa60]);
        let object = e.mem.alloc(0x80);
        let log = logged(&mut e, |e| {
            e.call(0x0056_6930, &args![object, 1u32]);
        });
        assert_eq!(calls_to(&log, 0x0047_aa60), vec![vec![object, 1, 0x40]]);
    }

    // ---- 005672c0 ---------------------------------------------------------

    fn end_sequence_fixture(kind: u8) -> (Engine, Ptr<TESObjectREFR>, u32, u32) {
        let mut e = engine();
        sequence_api(&mut e);
        let unequip = cstr(&mut e, "Unequip");
        e.set_global(NAME_UNEQUIP, unequip);
        e.register(0x0040_4dc0, |e, a| {
            (e.mem.cstr(a[0]) != e.mem.cstr(a[1])).into_ret()
        });
        let cell = e.mem.alloc(0x20);
        returns(&mut e, 0x0045_43c0, 1);
        let node = e.mem.alloc(0x20);
        let manager = e.mem.alloc(0x20);
        returns(&mut e, 0x0045_37b0, manager);
        returns(&mut e, 0x008c_7aa0, 0);
        stub(&mut e, &[0x00c6_a270, 0x00c6_a350, 0x0087_b1e0]);
        let base = form(&mut e, kind, 1, 0);
        let refr = full_refr(&mut e, base);
        e.mem.set_u32(refr.addr() + 0x40, cell);
        e.mem.set_u32(refr.addr() + GET_3D_ANSWER, node);
        (e, refr, node, manager)
    }

    #[test]
    fn end_sequence_resets_the_3d_when_the_unequip_sequence_ends() {
        let (mut e, refr, node, _) = end_sequence_fixture(0x15);
        let sequence = sequence(&mut e, "Unequip", false);
        let notified = slot_address(&e, refr.addr(), 0x48);
        let log = logged(&mut e, |e| {
            e.call(0x0056_72c0, &args![refr, sequence]);
        });
        assert_eq!(calls_to(&log, 0x00c6_a270), vec![vec![node, 1, 1, 1]]);
        // without 008c7aa0 the 3D's motion is set
        assert_eq!(calls_to(&log, 0x00c6_a350), vec![vec![node, 1, 1, 0, 1]]);
        assert!(calls_to(&log, 0x0087_b1e0).is_empty());
        assert_eq!(calls_to(&log, notified), vec![vec![refr.addr(), 4]]);
    }

    #[test]
    fn end_sequence_tells_the_manager_object_when_008c7aa0_holds() {
        let (mut e, refr, node, manager) = end_sequence_fixture(0x1c);
        returns(&mut e, 0x008c_7aa0, 1);
        let sequence = sequence(&mut e, "Unequip", false);
        let log = logged(&mut e, |e| {
            e.call(0x0056_72c0, &args![refr, sequence]);
        });
        assert_eq!(calls_to(&log, 0x0087_b1e0), vec![vec![manager, node, 1]]);
        assert!(calls_to(&log, 0x00c6_a350).is_empty());
    }

    #[test]
    fn end_sequence_ignores_other_sequences_types_and_cells() {
        let (mut e, refr, _, _) = end_sequence_fixture(0x15);
        let other = sequence(&mut e, "Equip", false);
        let log = logged(&mut e, |e| {
            e.call(0x0056_72c0, &args![refr, other]);
        });
        assert!(calls_to(&log, 0x00c6_a270).is_empty());
        // another base form type
        let (mut e, refr, _, _) = end_sequence_fixture(0x16);
        let unequip = sequence(&mut e, "Unequip", false);
        let log = logged(&mut e, |e| {
            e.call(0x0056_72c0, &args![refr, unequip]);
        });
        assert!(calls_to(&log, 0x00c6_a270).is_empty());
        // a cell that 004543c0 refuses, or none
        let (mut e, refr, _, _) = end_sequence_fixture(0x15);
        let unequip = sequence(&mut e, "Unequip", false);
        returns(&mut e, 0x0045_43c0, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_72c0, &args![refr, unequip]);
        });
        assert!(calls_to(&log, 0x00c6_a270).is_empty());
        e.mem.set_u32(refr.addr() + 0x40, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_72c0, &args![refr, unequip]);
        });
        assert!(calls_to(&log, 0x0045_43c0).is_empty());
    }

    // ---- 005673c0, 005673e0, 00567400 ---------------------------------------

    #[test]
    fn script_getters_read_the_extra_list() {
        let mut e = engine();
        let refr = refr_of_kind(&mut e, 0x40);
        returns(&mut e, 0x0041_8800, 0xa1);
        returns(&mut e, 0x0041_8830, 0xb2);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_73c0, &args![refr]).u32(), 0xa1);
            assert_eq!(e.call(0x0056_73e0, &args![refr]).u32(), 0xb2);
        });
        assert_eq!(calls_to(&log, 0x0041_8800), vec![vec![refr.addr() + 0x44]]);
        assert_eq!(calls_to(&log, 0x0041_8830), vec![vec![refr.addr() + 0x44]]);
    }

    #[test]
    fn get_scale_multiplies_by_the_base_scale_of_actors() {
        let mut e = engine();
        returns_float(&mut e, 0x0094_4300, 1.5);
        returns_float(&mut e, 0x0056_7470, 0.5);
        for (kind, expected) in [(0x2au8, 3.0f32), (0x2b, 1.0), (0x40, 2.0)] {
            let refr = refr_of_kind(&mut e, kind);
            e.mem.set_f32(refr.addr() + 0x3c, 2.0);
            assert_eq!(e.call(0x0056_7400, &args![refr]).f32(), expected);
        }
        // no base form: the plain scale
        let bare = refr_with(&mut e, 0, 0, &[]);
        e.mem.set_f32(bare.addr() + 0x3c, 0.75);
        assert_eq!(e.call(0x0056_7400, &args![bare]).f32(), 0.75);
    }

    // ---- 00567050 ---------------------------------------------------------

    /// A reference over a base form of `kind` whose loaded 3D has a controller;
    /// the controller's virtual +0x88 gives the dword at +0x10 of the
    /// controller (the test stores its manager there). The 3D's own virtual
    /// +0xc (its node) gives 0.
    fn step_fixture(kind: u8) -> (Engine, Ptr<TESObjectREFR>, u32) {
        let mut e = engine();
        sequence_api(&mut e);
        let base = form(&mut e, kind, 1, 0);
        let refr = full_refr(&mut e, base);
        let loaded = object_with_vtable(&mut e, 0x40, &[(0xc, |_, _| Ret::default())]);
        let controller = object_with_vtable(&mut e, 0x20, &[(0x88, slot_value::<0x10>)]);
        returns(&mut e, GET_LOADED_3D, loaded);
        e.register(GET_CONTROLLER, |e, a| e.mem.u32(a[0] + 0x10).into_ret());
        e.mem.set_u32(loaded + 0x10, controller);
        stub(
            &mut e,
            &[
                MANAGER_FINISH_SEQUENCE,
                MANAGER_SET_FLAG,
                0x0057_8a30,
                0x0047_ac70,
                SET_ANIMATION,
                ANIMATION_UPDATE,
            ],
        );
        // 004eef00(sequence, reference): the dword at +0x38 says "finished"
        e.register(0x004e_ef00, |e, a| e.mem.u32(a[0] + 0x38).into_ret());
        (e, refr, controller)
    }

    #[test]
    fn fn_00567050_ends_the_finished_sequences_and_keeps_the_manager_on_for_the_rest() {
        let (mut e, refr, controller) = step_fixture(0x40);
        let me = refr.addr();
        let finished = sequence(&mut e, "Done", true);
        e.mem.set_u32(finished + 0x38, 1);
        let running = sequence(&mut e, "Running", true);
        let plain = sequence(&mut e, "Plain", false);
        let manager = manager(&mut e, &[finished, running, plain]);
        e.mem.set_u32(controller + 0x10, manager);
        let name = e.mem.u32(finished + 8);
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        // only the generic sequences are asked
        assert_eq!(
            calls_to(&log, 0x004e_ef00),
            vec![vec![finished, me], vec![running, me]]
        );
        assert_eq!(
            calls_to(&log, MANAGER_FINISH_SEQUENCE),
            vec![vec![manager, finished, 0.0f32.to_bits()]]
        );
        assert_eq!(calls_to(&log, 0x0057_8a30), vec![vec![me, name]]);
        assert_eq!(calls_to(&log, 0x0047_ac70), vec![vec![me, finished]]);
        // one is still running: the manager stays on
        assert!(calls_to(&log, MANAGER_SET_FLAG).is_empty());

        // all finished: the manager is switched off
        e.mem.set_u32(running + 0x38, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        assert_eq!(calls_to(&log, MANAGER_SET_FLAG), vec![vec![manager, 0]]);
        assert_eq!(calls_to(&log, 0x0047_ac70).len(), 2);
    }

    #[test]
    fn fn_00567050_calls_the_virtual_178_for_type_1e_and_skips_deleted_references() {
        let (mut e, refr, _) = step_fixture(0x1e);
        let me = refr.addr();
        let hook = slot_address(&e, me, 0x178);
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        assert_eq!(calls_to(&log, hook), vec![vec![me]]);
        for flags in [0x20u32, 0x800] {
            e.mem.set_u32(me + 8, flags);
            let log = logged(&mut e, |e| {
                e.call(0x0056_7050, &args![refr]);
            });
            assert!(calls_to(&log, GET_LOADED_3D).is_empty(), "{flags:#x}");
            assert!(calls_to(&log, hook).is_empty());
        }
    }

    #[test]
    fn fn_00567050_updates_the_animation_of_types_2a_and_2b() {
        let (mut e, refr, _) = step_fixture(0x2a);
        let me = refr.addr();
        returns(&mut e, 0x0096_11e0, 1);
        returns_float(&mut e, FRAME_TIME, 0.5);
        returns_float(&mut e, TARGET_UPDATE_MULT, 2.0);
        let animation = e.mem.alloc(0x20);
        e.mem.set_u32(me + ANIMATION_ANSWER, animation);
        let minus_one = (-1.0f32).to_bits();
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        // the frame time is passed on as it is
        assert_eq!(
            calls_to(&log, ANIMATION_UPDATE),
            vec![vec![animation, me, 0.5f32.to_bits(), minus_one]]
        );
        // the sequences of the controller manager are not looked at
        assert!(calls_to(&log, GET_CONTROLLER).is_empty());

        // VATS in mode 4 with this reference scales the step
        e.mem.set_u32(VATS_OBJECT + 8, 4);
        e.set_global(REFERENCE_COMPARED_WITH_VATS, me);
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        assert_eq!(
            calls_to(&log, ANIMATION_UPDATE),
            vec![vec![animation, me, 1.0f32.to_bits(), minus_one]]
        );
        // another reference is not scaled
        e.set_global(REFERENCE_COMPARED_WITH_VATS, me + 4);
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        assert_eq!(calls_to(&log, ANIMATION_UPDATE)[0][2], 0.5f32.to_bits());

        // a 3D that cannot be updated: nothing
        returns(&mut e, 0x0096_11e0, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        assert!(calls_to(&log, ANIMATION_UPDATE).is_empty());
    }

    #[test]
    fn fn_00567050_initialises_a_missing_animation() {
        let (mut e, refr, _) = step_fixture(0x2b);
        let me = refr.addr();
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        // InitAnimation clears the animation first, then finds no node
        assert_eq!(calls_to(&log, SET_ANIMATION), vec![vec![me, 0]]);
        assert!(calls_to(&log, ANIMATION_UPDATE).is_empty());
        // no 3D at all: no initialisation
        returns(&mut e, GET_LOADED_3D, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_7050, &args![refr]);
        });
        assert!(calls_to(&log, SET_ANIMATION).is_empty());
    }

    // ---- the file lists of 00566970 ---------------------------------------

    /// The doubles of the KF file list builders. The model of a reference is
    /// the C string at +0x98; every file list added is recorded as (path as
    /// it is at that moment, model, list) in the returned log.
    fn kf_doubles(e: &mut Engine) -> std::rc::Rc<std::cell::RefCell<Vec<(String, String, u32)>>> {
        let added = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        for page in [0x0101_6000, 0x0101_d000, 0x0103_0000] {
            e.map(page, 0x1000);
        }
        for (address, text) in [
            (PATH_DATA, "Data\\"),
            (PATH_MESHES, "Meshes"),
            (PATH_BACKSLASH, "\\"),
            (PATH_MT_IDLE, "\\MTIdle.KF"),
            (PATH_HOLSTER, "\\%sHolster.KF"),
            (PATH_POWER_ARMOR_HOLSTER, "\\PA%sHolster.KF"),
            (PATH_DEATH, "\\Death.KF"),
            (PATH_CHILD_IDLES, "\\Locomotion\\Child\\IdleAnims"),
            (PATH_FEMALE_IDLES, "\\Locomotion\\Female\\IdleAnims"),
            (PATH_MALE_IDLES, "\\Locomotion\\Male\\IdleAnims"),
            (
                PATH_FULL_MALE_LOCOMOTION,
                "Data\\Meshes\\Locomotion\\MTIdle.kf",
            ),
            (PATH_MALE_LOCOMOTION, "Locomotion\\MTIdle.kf"),
            (
                PATH_FULL_MALE_IDLE,
                "Data\\Meshes\\Characters\\_Male\\MTIdle.kf",
            ),
        ] {
            e.mem.set_cstr(address, text.as_bytes());
        }
        let loader = e.mem.alloc(0x20);
        e.set_global(MODEL_LOADER, loader);
        let idle_manager = e.mem.alloc(0x20);
        e.set_global(IDLE_MANAGER, idle_manager);
        e.register(GET_MODEL, |e, a| e.mem.u32(a[0] + 0x98).into_ret());
        e.register(BOUNDED_COPY, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(BOUNDED_APPEND, |e, a| {
            let mut text = e.mem.cstr(a[0]);
            text.extend(e.mem.cstr(a[2]));
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(STRRCHR_WRAPPER, |e, a| {
            let text = e.mem.cstr(a[0]);
            text.iter()
                .rposition(|c| *c == b'\\')
                .map_or(0, |at| a[0] + at as u32)
                .into_ret()
        });
        // sprintf with one %s
        e.register(SPRINTF, |e, a| {
            let format = String::from_utf8(e.mem.cstr(a[1])).unwrap();
            let argument = String::from_utf8(e.mem.cstr(a[2])).unwrap();
            e.mem
                .set_cstr(a[0], format.replace("%s", &argument).as_bytes());
            Ret::default()
        });
        returns(e, BUILD_KF_FILE_LIST, 0x1111);
        let log = added.clone();
        e.register_double(BUILD_FILE_LIST, move |e, a| {
            let path = String::from_utf8(e.mem.cstr(a[1])).unwrap();
            let model = String::from_utf8(e.mem.cstr(a[2])).unwrap_or_default();
            log.borrow_mut().push((path, model, a[3]));
            Ret {
                eax: 0x2222,
                ..Ret::default()
            }
        });
        returns(e, ROOT_FILENAME_LIST, 0x3333);
        stub(e, &[COPY_FILENAME_LIST]);
        returns(e, ACTOR_CURRENT_WEAPON, 0);
        e.register(WEAPON_INDEX, |_, a| a[0].into_ret());
        added
    }

    /// A base form that is an object with a vtable (name pointer at +0x100);
    /// its embedded model object at +0xdc has a vtable whose +0x14 gives the
    /// model path stored at +0xe4.
    fn kf_base(e: &mut Engine, kind: u8, id: u32, model: &str) -> u32 {
        let base = named_base(e, kind, id, "Base");
        let embedded = object_with_vtable(e, 0x10, &[(0x14, slot_value::<0x8>)]);
        let vtable = e.mem.u32(embedded);
        e.mem.set_u32(base + 0xdc, vtable);
        let text = cstr(e, model);
        e.mem.set_u32(base + 0xe4, text);
        base
    }

    fn set_model(e: &mut Engine, refr: Ptr<TESObjectREFR>, model: &str) {
        let text = cstr(e, model);
        e.mem.set_u32(refr.addr() + 0x98, text);
    }

    #[test]
    fn build_kf_file_list_for_type_2a_with_id_7_copies_the_idle_lists() {
        for (child, female, idles) in [
            (1u32, 0u32, "\\Locomotion\\Child\\IdleAnims"),
            (0, 1, "\\Locomotion\\Female\\IdleAnims"),
            (0, 0, "\\Locomotion\\Male\\IdleAnims"),
        ] {
            let mut e = engine();
            let _ = kf_doubles(&mut e);
            let base = kf_base(&mut e, 0x2a, 7, "Characters\\Actor\\skeleton.nif");
            let refr = full_refr(&mut e, base);
            e.mem.set_u32(refr.addr() + SLOT_1A0_ANSWER, child);
            returns(&mut e, 0x005f_0cc0, female);
            // the idle list builder sees the path as it is then
            let paths = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
            let seen = paths.clone();
            e.register_double(ROOT_FILENAME_LIST, move |e, a| {
                seen.borrow_mut()
                    .push(String::from_utf8(e.mem.cstr(a[1])).unwrap());
                Ret {
                    eax: 0x3333,
                    ..Ret::default()
                }
            });
            let log = logged(&mut e, |e| {
                let list = e.call(0x0056_6970, &args![refr, 5u32]).u32();
                assert_eq!(list, 0x1111);
            });
            let loader = e.global::<u32>(MODEL_LOADER);
            // the base form's model, with the selector the caller gave
            let model = e.mem.u32(base + 0xe4);
            assert_eq!(
                calls_to(&log, BUILD_KF_FILE_LIST),
                vec![vec![loader, model, 1, 1, 5]]
            );
            assert_eq!(*paths.borrow(), vec![format!("Characters\\Actor{idles}")]);
            let idle_manager = e.global::<u32>(IDLE_MANAGER);
            assert_eq!(calls_to(&log, ROOT_FILENAME_LIST)[0][0..1], [idle_manager]);
            assert_eq!(
                calls_to(&log, COPY_FILENAME_LIST),
                vec![vec![loader, 0x3333, 0x1111]]
            );
            // the room left in the buffer: 0x104 less the directory, less 1
            let copies = calls_to(&log, BOUNDED_COPY);
            assert_eq!(copies[1][1], 0x104 - "Characters\\Actor".len() as u32 - 1);
        }
    }

    #[test]
    fn build_kf_file_list_for_type_2a_takes_the_idle_path_without_the_virtual_22c() {
        let mut e = engine();
        let _ = kf_doubles(&mut e);
        let base = kf_base(&mut e, 0x2a, 9, "Characters\\Actor\\skeleton.nif");
        let refr = full_refr(&mut e, base);
        returns(&mut e, 0x005f_0cc0, 0);
        // the virtual +0x22c(1) says no: the same path as for form ID 7
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_6970, &args![refr, 3u32]).u32(), 0x1111);
        });
        assert_eq!(calls_to(&log, BUILD_KF_FILE_LIST).len(), 1);
        assert!(calls_to(&log, BUILD_FILE_LIST).is_empty());
        assert_eq!(calls_to(&log, COPY_FILENAME_LIST).len(), 1);
    }

    #[test]
    fn build_kf_file_list_for_type_2a_builds_the_male_locomotion_and_the_weapon_files() {
        let mut e = engine();
        let added = kf_doubles(&mut e);
        let base = kf_base(&mut e, 0x2a, 9, "x");
        let refr = full_refr(&mut e, base);
        set_model(&mut e, refr, "m");
        e.mem.set_u32(refr.addr() + SLOT_22C_ARG1_ANSWER, 1);
        // weapon 2: table index -> table entry 3 -> holster prefix "Rifle"
        let prefix = cstr(&mut e, "Rifle");
        e.mem.set_u32(WEAPON_INDEX_TABLE + 8, 3);
        e.mem.set_u32(HOLSTER_PREFIX_TABLE + 12, prefix);
        returns(&mut e, ACTOR_CURRENT_WEAPON, 2);
        let log = logged(&mut e, |e| {
            // the list the first call builds is the result
            assert_eq!(e.call(0x0056_6970, &args![refr, 0u32]).u32(), 0x2222);
        });
        let loader = e.global::<u32>(MODEL_LOADER);
        assert!(calls_to(&log, BUILD_KF_FILE_LIST).is_empty());
        let paths: Vec<(String, u32)> = added
            .borrow()
            .iter()
            .map(|(path, model, list)| (format!("{path}|{model}"), *list))
            .collect();
        // (path, second path) first; the weapon and death files are added to it
        assert_eq!(
            paths[0],
            (
                "Data\\Meshes\\Locomotion\\MTIdle.kf|Locomotion\\MTIdle.kf".to_string(),
                0
            )
        );
        let holsters: Vec<&str> = paths[1..].iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            holsters,
            vec![
                "Data\\Meshes\\Characters\\_Male\\RifleHolster.KF|m",
                "Data\\Meshes\\Characters\\_Male\\PARifleHolster.KF|m",
                "Data\\Meshes\\Characters\\_Male\\Death.KF|m",
            ]
        );
        assert!(paths[1..].iter().all(|(_, list)| *list == 0x2222));
        let _ = loader;
        // without a weapon prefix only the death file follows
        let mut e = engine();
        let added = kf_doubles(&mut e);
        let base = kf_base(&mut e, 0x2a, 9, "x");
        let refr = full_refr(&mut e, base);
        set_model(&mut e, refr, "m");
        e.mem.set_u32(refr.addr() + SLOT_22C_ARG1_ANSWER, 1);
        e.call(0x0056_6970, &args![refr, 0u32]);
        assert_eq!(added.borrow().len(), 2);
        assert!(added.borrow()[1].0.ends_with("Death.KF"));
    }

    /// A type 0x2b reference whose process (virtual +0x148 gives the dword
    /// at +0x10 of it, the weapon) is at `process`.
    fn kf_actor(e: &mut Engine) -> (Ptr<TESObjectREFR>, u32, u32) {
        let base = kf_base(e, 0x2b, 9, "Characters\\Dog\\skeleton.nif");
        let refr = full_refr(e, base);
        set_model(e, refr, "Characters\\Dog\\skeleton.nif");
        let process = object_with_vtable(e, 0x20, &[(0x148, slot_value::<0x10>)]);
        returns(e, ACTOR_PROCESS, process);
        returns(e, PROCESS_LEVEL, 0);
        returns(e, 0x008a_6970, 0);
        (refr, base, process)
    }

    #[test]
    fn build_kf_file_list_for_type_2b_picks_the_selector_from_the_weapon() {
        let mut e = engine();
        let _ = kf_doubles(&mut e);
        let (refr, _, process) = kf_actor(&mut e);
        // a weapon whose flags (+8) are 2: the table entry 0x66
        let weapon = form(&mut e, 0, 0, 2);
        e.mem.set_u32(process + 0x10, weapon);
        e.mem.set_u32(WEAPON_INDEX_TABLE + 8, 0x66);
        let model = e.mem.u32(refr.addr() + 0x98);
        let loader = e.global::<u32>(MODEL_LOADER);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_6970, &args![refr, 5u32]).u32(), 0x1111);
        });
        assert_eq!(
            calls_to(&log, BUILD_KF_FILE_LIST),
            vec![vec![loader, model, 1, 1, 0x66]]
        );
        // no weapon, but 008a6970: the selector 1
        e.mem.set_u32(process + 0x10, 0);
        returns(&mut e, 0x008a_6970, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_6970, &args![refr, 5u32]);
        });
        assert_eq!(calls_to(&log, BUILD_KF_FILE_LIST)[0][4], 1);
        // neither: the caller's selector; and with the virtual +0x22c(0) true,
        // or a process level out of range, it is never changed
        returns(&mut e, 0x008a_6970, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_6970, &args![refr, 5u32]);
        });
        assert_eq!(calls_to(&log, BUILD_KF_FILE_LIST)[0][4], 5);
        e.mem.set_u32(process + 0x10, weapon);
        e.mem.set_u32(refr.addr() + SLOT_22C_ARG0_ANSWER, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_6970, &args![refr, 5u32]);
        });
        assert_eq!(calls_to(&log, BUILD_KF_FILE_LIST)[0][4], 5);
        e.mem.set_u32(refr.addr() + SLOT_22C_ARG0_ANSWER, 0);
        returns(&mut e, PROCESS_LEVEL, 2);
        let log = logged(&mut e, |e| {
            e.call(0x0056_6970, &args![refr, 5u32]);
        });
        assert_eq!(calls_to(&log, BUILD_KF_FILE_LIST)[0][4], 5);
    }

    #[test]
    fn build_kf_file_list_for_type_2b_builds_the_mesh_path_when_the_virtual_22c_holds() {
        let mut e = engine();
        let added = kf_doubles(&mut e);
        let (refr, _, _) = kf_actor(&mut e);
        e.mem.set_u32(refr.addr() + SLOT_22C_ARG1_ANSWER, 1);
        let prefix = cstr(&mut e, "Pistol");
        e.mem.set_u32(WEAPON_INDEX_TABLE, 1);
        e.mem.set_u32(HOLSTER_PREFIX_TABLE + 4, prefix);
        let log = logged(&mut e, |e| {
            assert_eq!(e.call(0x0056_6970, &args![refr, 0u32]).u32(), 0x2222);
        });
        assert!(calls_to(&log, BUILD_KF_FILE_LIST).is_empty());
        let paths: Vec<(String, String, u32)> = added.borrow().clone();
        let model = "Characters\\Dog\\skeleton.nif".to_string();
        // Data\Meshes\<model> with the file name replaced by \MTIdle.KF
        assert_eq!(
            paths[0],
            (
                "Data\\Meshes\\Characters\\Dog\\MTIdle.KF".to_string(),
                model.clone(),
                0
            )
        );
        assert_eq!(
            paths[1].0,
            "Data\\Meshes\\Characters\\Dog\\PistolHolster.KF"
        );
        assert_eq!(
            paths[2].0,
            "Data\\Meshes\\Characters\\Dog\\PAPistolHolster.KF"
        );
        assert_eq!(paths[3].0, "Data\\Meshes\\Characters\\Dog\\Death.KF");
        assert!(paths[1..].iter().all(|p| p.1 == model && p.2 == 0x2222));
    }

    #[test]
    fn build_kf_file_list_is_empty_for_other_types_and_without_a_base_form() {
        let mut e = engine();
        let _ = kf_doubles(&mut e);
        let other = kf_base(&mut e, 0x40, 9, "x");
        let refr = full_refr(&mut e, other);
        assert_eq!(e.call(0x0056_6970, &args![refr, 0u32]).u32(), 0);
        let bare = full_refr(&mut e, 0);
        assert_eq!(e.call(0x0056_6970, &args![bare, 0u32]).u32(), 0);
    }

    // ---- 005659f0 ---------------------------------------------------------

    /// The 3D object the world's `0043fcd0` gives: its virtual +0xc gives the
    /// node stored at +0x14.
    struct AnimationWorld {
        e: Engine,
        refr: Ptr<TESObjectREFR>,
        base: u32,
        loaded: u32,
        node: u32,
    }

    /// A reference over a base form of `kind` (ID 0x1234, name "Base") with
    /// a loaded 3D and a node, every callee of `InitAnimation` registered
    /// as a harmless double. The node's controller is at +0x10 of the node
    /// (none); `0042ce10` (the loading object's busy test) says no.
    fn animation_world(kind: u8) -> AnimationWorld {
        let mut e = engine();
        sequence_api(&mut e);
        let _ = kf_doubles(&mut e);
        e.mem.set_cstr(SKELETON_PREFIX, b"Skeleton");
        let base = kf_base(&mut e, kind, 0x1234, "Characters\\Actor\\skeleton.nif");
        let refr = full_refr(&mut e, base);
        set_model(&mut e, refr, "Characters\\Actor\\skeleton.nif");
        let node = object_with_vtable(&mut e, 0x40, &[(0xa4, |_, _| Ret::default())]);
        let loaded = object_with_vtable(&mut e, 0x40, &[(0xc, slot_value::<0x14>)]);
        e.mem.set_u32(loaded + 0x14, node);
        returns(&mut e, GET_LOADED_3D, loaded);
        e.register(SET_ANIMATION, |_, a| a[1].into_ret());
        e.register(OPERATOR_NEW, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(ANIMATION_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(STRRCHR, |e, a| {
            let text = e.mem.cstr(a[0]);
            text.iter()
                .rposition(|c| *c == a[1] as u8)
                .map_or(0, |at| a[0] + at as u32)
                .into_ret()
        });
        e.register(STRNICMP, |e, a| {
            let first = e.mem.cstr(a[0]);
            let second = e.mem.cstr(a[1]);
            let count = a[2] as usize;
            let lower = |text: &[u8]| {
                text.iter()
                    .take(count)
                    .map(u8::to_ascii_lowercase)
                    .collect::<Vec<_>>()
            };
            u32::from(lower(&first) != lower(&second)).into_ret()
        });
        returns(&mut e, 0x008a_1760, 0x5151);
        returns(&mut e, 0x0048_ffd0, 1);
        returns_float(&mut e, 0x0056_8ad0, 0.0);
        returns(&mut e, 0x0047_fd90, 0);
        returns(&mut e, 0x0071_7e50, 0x7777);
        stub(
            &mut e,
            &[
                LOG_MESSAGE,
                0x004a_ae70,
                0x0057_10c0,
                0x0049_0330,
                0x005f_bdd0,
                ANIMATION_UPDATE,
                0x0048_4b90,
                0x0057_26e0,
                MANAGER_SET_FLAG,
                MANAGER_DEACTIVATE_ALL,
                MANAGER_ACTIVATE,
                SEQUENCE_SET_OFFSET,
                SET_3D_VELOCITY,
                FIXED_STRING_DESTRUCT,
                0x0041_ae10,
            ],
        );
        e.register(GET_CONTROLLER, |e, a| e.mem.u32(a[0] + 0x10).into_ret());
        e.register(CHECKED_CAST, |_, a| a[1].into_ret());
        e.register(FIXED_STRING_CONSTRUCT, |_, a| a[1].into_ret());
        // the sequence of the manager whose name is the handle
        e.register(MANAGER_FIND_SEQUENCE, |e, a| {
            let count = e.mem.u32(a[0] + 4);
            for index in 0..count {
                let sequence = e.mem.u32(a[0] + 8 + 4 * index);
                if e.mem.u32(sequence + 8) == a[1] {
                    return sequence.into_ret();
                }
            }
            Ret::default()
        });
        returns(&mut e, 0x0047_a490, 0);
        returns(&mut e, 0x004c_0bf0, 0);
        returns(&mut e, 0x0059_bb30, 0);
        returns(&mut e, 0x0041_8200, 0);
        returns(&mut e, 0x0042_ce10, 0);
        returns(&mut e, ACTOR_PROCESS, 0);
        let loading = e.mem.alloc(0x20);
        e.set_global(GLOBAL_LOADING_OBJECT, loading);
        AnimationWorld {
            e,
            refr,
            base,
            loaded,
            node,
        }
    }

    #[test]
    fn init_animation_clears_the_animation_and_stops_without_a_node_or_when_deleted() {
        let mut world = animation_world(0x2a);
        let (me, loaded) = (world.refr.addr(), world.loaded);
        let e = &mut world.e;
        // no loaded 3D
        returns(e, GET_LOADED_3D, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(calls_to(&log, SET_ANIMATION), vec![vec![me, 0]]);
        assert!(calls_to(&log, GET_MODEL).is_empty());
        // a 3D without a node
        returns(e, GET_LOADED_3D, loaded);
        e.mem.set_u32(loaded + 0x14, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(calls_to(&log, SET_ANIMATION), vec![vec![me, 0]]);
        // a deleted reference
        e.mem.set_u32(loaded + 0x14, world.node);
        e.mem.set_u32(me + 8, 0x20);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(calls_to(&log, SET_ANIMATION), vec![vec![me, 0]]);
        assert!(calls_to(&log, GET_MODEL).is_empty());
    }

    #[test]
    fn init_animation_builds_the_npc_animation_and_its_special_animations() {
        let mut world = animation_world(0x2a);
        let (me, node, base) = (world.refr.addr(), world.node, world.base);
        let e = &mut world.e;
        e.mem.set_u32(me + BIPED_ANSWER, 0xb1);
        // base form value > 0 and KF files: the special animations are added
        returns_float(e, 0x0056_8ad0, 1.0);
        returns(e, 0x0047_fd90, 1);
        returns(e, 0x005f_0cc0, 0);
        let paths = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let seen = paths.clone();
        e.register_double(0x0049_0330, move |e, a| {
            seen.borrow_mut()
                .push((a[1], String::from_utf8(e.mem.cstr(a[2])).unwrap()));
            Ret::default()
        });
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        let animation = calls_to(&log, ANIMATION_CONSTRUCT)[0][0];
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x13c]]);
        assert_eq!(
            calls_to(&log, SET_ANIMATION),
            vec![vec![me, 0], vec![me, animation]]
        );
        // the biped loses its parts first; the skeleton is asked of the
        // actor (here none: the reference is not an actor)
        assert_eq!(calls_to(&log, 0x004a_ae70), vec![vec![0xb1]]);
        assert_eq!(calls_to(&log, 0x008a_1760), vec![vec![0]]);
        // the KF files: the base form's model, the skeleton as selector
        let loader = e.global::<u32>(MODEL_LOADER);
        let model = e.mem.u32(base + 0xe4);
        assert_eq!(
            calls_to(&log, BUILD_KF_FILE_LIST),
            vec![vec![loader, model, 1, 1, 0x5151]]
        );
        assert_eq!(
            calls_to(&log, 0x0048_ffd0),
            vec![vec![animation, 0x1111, node, me, 1]]
        );
        // the model's directory cut from the path
        assert_eq!(
            *paths.borrow(),
            vec![(0x7777, "Characters\\Actor".to_string())]
        );
        assert_eq!(calls_to(&log, 0x0071_7e50), vec![vec![base + 0xc4]]);
        // the animation is updated once, with a zero step
        assert_eq!(
            calls_to(&log, ANIMATION_UPDATE),
            vec![vec![animation, me, 0.0f32.to_bits(), (-1.0f32).to_bits()]]
        );
        assert!(calls_to(&log, LOG_MESSAGE).is_empty());
    }

    #[test]
    fn init_animation_reports_a_misnamed_skeleton_and_a_missing_idle() {
        let mut world = animation_world(0x2a);
        let (me, base) = (world.refr.addr(), world.base);
        let e = &mut world.e;
        e.mem.set_u32(me + BIPED_ANSWER, 0xb1);
        returns(e, 0x005f_0cc0, 0);
        returns(e, 0x0048_ffd0, 0);
        set_model(e, world.refr, "Characters\\Actor\\body.nif");
        let model = e.mem.u32(me + 0x98);
        let name = e.mem.u32(base + 0x100);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(
            calls_to(&log, LOG_MESSAGE),
            vec![
                vec![SKELETON_MESSAGE, name, model],
                vec![NPC_ANIMATION_MESSAGE, name, 0x1234]
            ]
        );
        // an NPC without a biped builds nothing
        e.mem.set_u32(me + BIPED_ANSWER, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(calls_to(&log, SET_ANIMATION), vec![vec![me, 0]]);
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert!(calls_to(&log, ANIMATION_UPDATE).is_empty());
    }

    #[test]
    fn init_animation_builds_the_creature_animation_and_notices_a_lost_node() {
        let mut world = animation_world(0x2b);
        let (me, node, base, loaded) = (world.refr.addr(), world.node, world.base, world.loaded);
        let e = &mut world.e;
        returns_float(e, 0x0056_8ad0, 2.0);
        returns(e, 0x0047_fd90, 1);
        returns(e, 0x0048_ffd0, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        let animation = calls_to(&log, ANIMATION_CONSTRUCT)[0][0];
        // creatures have no biped parts to remove; the KF list comes from the
        // reference's own model
        assert!(calls_to(&log, 0x004a_ae70).is_empty());
        let model = e.mem.u32(me + 0x98);
        let loader = e.global::<u32>(MODEL_LOADER);
        assert_eq!(
            calls_to(&log, BUILD_KF_FILE_LIST),
            vec![vec![loader, model, 1, 1, 0x5151]]
        );
        assert_eq!(
            calls_to(&log, 0x0048_ffd0),
            vec![vec![animation, 0x1111, node, me, 1]]
        );
        let name = e.mem.u32(base + 0x100);
        assert_eq!(
            calls_to(&log, LOG_MESSAGE),
            vec![vec![CREATURE_ANIMATION_MESSAGE, name, 0x1234]]
        );
        // the base form gets the 3D at the end
        assert_eq!(calls_to(&log, 0x005f_bdd0), vec![vec![base, loaded]]);
        assert_eq!(calls_to(&log, ANIMATION_UPDATE).len(), 1);

        // the node vanishes when the animation is installed
        let mut calls = 0;
        e.register_double(GET_LOADED_3D, move |_, _| {
            calls += 1;
            Ret {
                eax: if calls == 1 { loaded } else { 0 },
                ..Ret::default()
            }
        });
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(
            calls_to(&log, LOG_MESSAGE),
            vec![vec![MOVED_MESSAGE, name, 0x1234]]
        );
        assert!(calls_to(&log, 0x005f_bdd0).is_empty());
        assert!(calls_to(&log, ANIMATION_UPDATE).is_empty());
    }

    #[test]
    fn init_animation_clears_the_motion_of_a_node_without_a_manager() {
        let mut world = animation_world(0x40);
        let (me, node) = (world.refr.addr(), world.node);
        let e = &mut world.e;
        let clear = slot_address(e, node, 0xa4);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        // an empty record is given to the node through its virtual +0xa4
        let record = calls_to(&log, MAKE_VELOCITY)[0][0];
        assert_eq!(
            calls_to(&log, MAKE_VELOCITY),
            vec![vec![record, 0.0f32.to_bits(), 0, 0]]
        );
        assert_eq!(calls_to(&log, clear), vec![vec![node, record, 0]]);
        assert!(calls_to(&log, 0x0048_4b90).is_empty());
        assert!(calls_to(&log, ANIMATION_UPDATE).is_empty());
        // a controller that is not a manager is the same
        e.mem.set_u32(node + 0x10, 0x99);
        e.register(CHECKED_CAST, |_, _| Ret::default());
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(calls_to(&log, clear).len(), 1);
        assert!(calls_to(&log, 0x0048_4b90).is_empty());
        // a light base form (type 0x28) accepted by 004c0bf0 sets its light
        let light = animation_world(0x28);
        let mut e = light.e;
        returns(&mut e, 0x004c_0bf0, 1);
        let log = logged(&mut e, |e| {
            e.call(0x0056_59f0, &args![light.refr]);
        });
        assert_eq!(
            calls_to(&log, 0x0057_26e0),
            vec![vec![light.refr.addr(), light.node, 0]]
        );
        returns(&mut e, 0x004c_0bf0, 0);
        let log = logged(&mut e, |e| {
            e.call(0x0056_59f0, &args![light.refr]);
        });
        assert!(calls_to(&log, 0x0057_26e0).is_empty());
        let _ = me;
    }

    /// A world whose node has a controller (which the checked cast turns
    /// into itself) that is a manager with the sequences `idle_a`, `idle_b`
    /// and `other`; the idle names in the exe's data are those of the first
    /// two. Returns the world, the manager and the three sequences.
    fn manager_world(kind: u8) -> (AnimationWorld, u32, [u32; 3]) {
        let mut world = animation_world(kind);
        let e = &mut world.e;
        let idle_a = sequence(e, "IdleA", false);
        let idle_b = sequence(e, "IdleB", false);
        let other = sequence(e, "Other", false);
        e.mem.set_f32(idle_a + 0x2c, 1.5);
        e.mem.set_f32(idle_b + 0x2c, 3.0);
        e.mem.set_f32(other + 0x2c, 7.0);
        let manager = manager(e, &[other, idle_a, idle_b]);
        let name_a = e.mem.u32(idle_a + 8);
        let name_b = e.mem.u32(idle_b + 8);
        e.set_global(NAME_IDLE_A, name_a);
        e.set_global(NAME_IDLE_B, name_b);
        // the node's controller is the manager itself
        e.mem.set_u32(world.node + 0x10, manager);
        (world, manager, [idle_a, idle_b, other])
    }

    #[test]
    fn init_animation_starts_the_idle_sequences() {
        let (mut world, manager, [idle_a, idle_b, _]) = manager_world(0x40);
        let (me, node) = (world.refr.addr(), world.node);
        let e = &mut world.e;
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        // the form is marked changed
        assert_eq!(calls_to(&log, 0x0048_4b90), vec![vec![me, 0x1000_0000]]);
        let activate = |sequence: u32| {
            vec![
                manager,
                sequence,
                0,
                0,
                1.0f32.to_bits(),
                0.0f32.to_bits(),
                0,
            ]
        };
        assert_eq!(
            calls_to(&log, MANAGER_ACTIVATE),
            vec![activate(idle_a), activate(idle_b)]
        );
        assert_eq!(
            calls_to(&log, SEQUENCE_SET_OFFSET),
            vec![
                vec![idle_a, (-f32::MAX).to_bits()],
                vec![idle_b, (-f32::MAX).to_bits()]
            ]
        );
        // the node gets the record of the last idle's offset time
        let record = calls_to(&log, MAKE_VELOCITY);
        assert_eq!(
            (record[0][1], record[0][2], record[0][3]),
            (3.0f32.to_bits(), 1, 0)
        );
        assert_eq!(
            calls_to(&log, SET_3D_VELOCITY),
            vec![vec![node, record[0][0]]]
        );
        // the idle names were turned into fixed strings and looked up
        assert_eq!(calls_to(&log, MANAGER_FIND_SEQUENCE).len(), 2);
        assert_eq!(calls_to(&log, FIXED_STRING_DESTRUCT).len(), 2);
        // the manager is not switched on or off in this case
        assert!(calls_to(&log, MANAGER_SET_FLAG).is_empty());
    }

    #[test]
    fn init_animation_reports_an_idle_that_is_not_looping() {
        let (mut world, manager, [idle_a, idle_b, _]) = manager_world(0x40);
        let me = world.refr.addr();
        let e = &mut world.e;
        e.register(0x0059_bb30, |_, _| 1u32.into_ret());
        let model = e.mem.u32(me + 0x98);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        // only the second idle is started; the first is reported
        assert_eq!(
            calls_to(&log, LOG_MESSAGE),
            vec![vec![NOT_LOOPING_MESSAGE, model]]
        );
        let started: Vec<u32> = calls_to(&log, MANAGER_ACTIVATE)
            .iter()
            .map(|call| call[1])
            .collect();
        assert_eq!(started, vec![idle_b]);
        assert_eq!(calls_to(&log, MAKE_VELOCITY)[0][1], 3.0f32.to_bits());
        // with only the first idle present (and reported), the record is zero
        e.set_global(NAME_IDLE_B, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(calls_to(&log, MAKE_VELOCITY)[0][1], 0.0f32.to_bits());
        let _ = (manager, idle_a);
    }

    #[test]
    fn init_animation_reports_a_cumulative_manager() {
        let (mut world, manager, _) = manager_world(0x40);
        let me = world.refr.addr();
        let e = &mut world.e;
        let type_name = cstr(e, "Static");
        e.mem.set_u32(TYPE_NAME_TABLE + 0x40 * 12, type_name);
        let model = e.mem.u32(me + 0x98);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, LOG_MESSAGE).is_empty());
        e.mem.set_u8(manager + 0x68, 1);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(
            calls_to(&log, LOG_MESSAGE),
            vec![vec![CUMULATIVE_MESSAGE, type_name, model]]
        );
    }

    #[test]
    fn init_animation_starts_the_first_sequence_when_there_is_no_idle() {
        let (mut world, manager, [_, _, other]) = manager_world(0x40);
        let node = world.node;
        let e = &mut world.e;
        e.set_global(NAME_IDLE_A, 0);
        e.set_global(NAME_IDLE_B, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        // switched on, cleared, the first sequence (the manager's index 0)
        // started from its offset time, then switched off again
        assert_eq!(
            calls_to(&log, MANAGER_SET_FLAG),
            vec![vec![manager, 1], vec![manager, 0]]
        );
        assert_eq!(
            calls_to(&log, MANAGER_DEACTIVATE_ALL),
            vec![
                vec![manager, 0.0f32.to_bits()],
                vec![manager, 0.0f32.to_bits()]
            ]
        );
        assert_eq!(calls_to(&log, MANAGER_ACTIVATE)[0][1], other);
        assert_eq!(
            calls_to(&log, SEQUENCE_SET_OFFSET),
            vec![vec![other, (-f32::MAX).to_bits()]]
        );
        assert_eq!(calls_to(&log, MAKE_VELOCITY)[0][1], 7.0f32.to_bits());
        assert_eq!(calls_to(&log, SET_3D_VELOCITY)[0][0], node);
    }

    #[test]
    fn init_animation_restores_the_open_state_of_an_open_close_form() {
        let (mut world, manager, _) = manager_world(0x40);
        let me = world.refr.addr();
        let e = &mut world.e;
        e.set_global(NAME_IDLE_A, 0);
        e.set_global(NAME_IDLE_B, 0);
        returns(e, 0x0047_a490, 1);
        // 00572d30(reference, 8): the extra flag; 00561d90: the default state
        returns(e, 0x0057_2d30, 0);
        returns(e, 0x0056_1d90, 1);
        e.register(0x008c_71b0, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            a[0].into_ret()
        });
        // 0084a6d0(loading, reference, flags): the saved change says "closed"
        returns(e, 0x0084_a6d0, 0);
        returns(e, 0x0047_aec0, 0);
        let loading = e.global::<u32>(GLOBAL_LOADING_OBJECT);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        // default open, saved change not set: the open state is 1
        assert_eq!(
            calls_to(&log, 0x0084_a6d0),
            vec![vec![loading, me, 0x80_0000]]
        );
        assert_eq!(calls_to(&log, 0x0047_aec0), vec![vec![me, 1, 1]]);
        // SetOpenState said no, so the first sequence is started as well
        assert_eq!(calls_to(&log, MANAGER_ACTIVATE).len(), 1);
        // when it says yes, the sequence start is skipped
        returns(e, 0x0047_aec0, 1);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, MANAGER_ACTIVATE).is_empty());
        assert_eq!(calls_to(&log, MANAGER_SET_FLAG), vec![vec![manager, 0]]);
        // a busy loading object keeps the extra flag as the state
        returns(e, 0x0042_ce10, 1);
        returns(e, 0x0057_2d30, 1);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, 0x0084_a6d0).is_empty());
        assert_eq!(calls_to(&log, 0x0047_aec0), vec![vec![me, 1, 1]]);
    }

    #[test]
    fn init_animation_restores_a_saved_animation_time() {
        let mut world = animation_world(0x2a);
        let (me, node, loaded) = (world.refr.addr(), world.node, world.loaded);
        let e = &mut world.e;
        e.mem.set_u32(me + BIPED_ANSWER, 0xb1);
        returns(e, 0x005f_0cc0, 0);
        let saved = e.mem.alloc(0x20);
        e.mem.set_f32(saved + 0x10, 2.5);
        returns(e, 0x0041_8200, saved);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        let made = calls_to(&log, MAKE_VELOCITY);
        assert_eq!(
            (made[0][1], made[0][2], made[0][3]),
            (2.5f32.to_bits(), 0, 0)
        );
        // given to the reference's loaded 3D, and the extra data removed
        assert_eq!(
            calls_to(&log, SET_3D_VELOCITY),
            vec![vec![loaded, made[0][0]]]
        );
        assert_eq!(calls_to(&log, 0x0041_ae10), vec![vec![me + 0x44]]);
        // without a new animation (no biped) nothing is restored
        e.mem.set_u32(me + BIPED_ANSWER, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, 0x0041_ae10).is_empty());
        let _ = node;
    }

    /// The world of the actor tests: an actor (virtual +0x100 true) whose
    /// process is an object with the virtuals `InitAnimation` calls, at
    /// level 0, with a worn item of flags 0x33 and no loading in progress.
    fn actor_world(kind: u8) -> (AnimationWorld, u32, u32) {
        let mut world = animation_world(kind);
        let me = world.refr.addr();
        let e = &mut world.e;
        e.mem.set_u32(me + IS_ACTOR_ANSWER, 1);
        e.mem.set_u32(me + GET_3D_ANSWER, 0x5555);
        e.mem.set_u32(me + BIPED_ANSWER, 0xb1);
        let noop: AbiFn = |_, _| Ret::default();
        let process = object_with_vtable(
            e,
            0x40,
            &[
                (0x160, noop),
                (0x18c, noop),
                (0x1cc, noop),
                (0x22c, noop),
                (0x458, noop),
                (0x474, noop),
            ],
        );
        returns(e, ACTOR_PROCESS, process);
        returns(e, PROCESS_LEVEL, 0);
        let worn = form(e, 0x30, 1, 0x33);
        returns(e, 0x004b_f220, 0x77);
        returns(e, 0x004c_8c10, worn);
        returns(e, FORCE_NEXT_UPDATE, 0);
        returns(e, 0x008c_7aa0, 0);
        returns(e, 0x008a_6970, 0);
        returns(e, ACTOR_ANIMATION, 0xa1);
        returns(e, ACTOR_WEAPON_DRAWN, 1);
        returns(e, 0x00a5_c570, 0);
        e.register(0x0055_d520, |e, a| e.mem.u32(a[0] + 0xb0).into_ret());
        e.register(STRSTR, |e, a| {
            let text = String::from_utf8(e.mem.cstr(a[0])).unwrap();
            let needle = String::from_utf8(e.mem.cstr(a[1])).unwrap();
            u32::from(text.contains(&needle)).into_ret()
        });
        e.mem.set_cstr(LILY_NAME, b"Lily");
        stub(e, &[0x0057_1760, 0x0087_b360, 0x0047_aa60]);
        let manager_object = e.mem.alloc(0x20);
        returns(e, 0x0045_37b0, manager_object);
        returns(e, 0x0047_b200, 0x4242);
        (world, process, worn)
    }

    #[test]
    fn init_animation_gives_an_actor_its_worn_weapon() {
        let (mut world, process, _) = actor_world(0x40);
        let me = world.refr.addr();
        let e = &mut world.e;
        // the virtual +0x218 holds
        e.mem.set_u32(me + SLOT_218_ANSWER, 1);
        let gives = slot_address(e, process, 0x160);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        let worn = e.call(0x004c_8c10, &args![0x77u32, 5u32, 0u32]).u32();
        // the worn item (slot 5 of the inventory changes) and the 3D
        assert_eq!(calls_to(&log, 0x004c_8c10)[0], vec![0x77, 5, 0]);
        assert_eq!(calls_to(&log, gives), vec![vec![process, worn, 0x5555, 0]]);
        // not the manager object's attach: the plain weapon attach with the
        // worn item's flags
        assert_eq!(calls_to(&log, 0x0057_1760), vec![vec![me, 0x33]]);
        assert!(calls_to(&log, 0x0087_b360).is_empty());
        // the process refreshes its drawn state with the biped, the actor's
        // animation and the actor
        let drawn = slot_address(e, process, 0x1cc);
        assert_eq!(
            calls_to(&log, drawn),
            vec![vec![process, 1, 0xb1, 0xa1, me]]
        );
        assert_eq!(
            calls_to(&log, slot_address(e, process, 0x22c)),
            vec![vec![process]]
        );
        assert!(calls_to(&log, slot_address(e, process, 0x458)).is_empty());
    }

    #[test]
    fn init_animation_queues_the_weapon_attach_when_008c7aa0_holds() {
        let (mut world, _, _) = actor_world(0x40);
        let me = world.refr.addr();
        let e = &mut world.e;
        e.mem.set_u32(me + SLOT_218_ANSWER, 1);
        returns(e, 0x008c_7aa0, 1);
        let manager_object = e.call(0x0045_37b0, &args![]).u32();
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(
            calls_to(&log, 0x0087_b360),
            vec![vec![manager_object, me, 0x33]]
        );
        assert!(calls_to(&log, 0x0057_1760).is_empty());
        // ... unless the reference is the one 0047b200 names
        returns(e, 0x0047_b200, me);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, 0x0087_b360).is_empty());
        assert_eq!(calls_to(&log, 0x0057_1760), vec![vec![me, 0x33]]);
    }

    #[test]
    fn init_animation_leaves_the_weapon_alone_for_lily_and_for_other_actors() {
        let (mut world, process, worn) = actor_world(0x40);
        let me = world.refr.addr();
        let e = &mut world.e;
        // not +0x218, but the force-update test and the name "Lily"
        returns(e, FORCE_NEXT_UPDATE, 1);
        let name = cstr(e, "Lily Belle");
        e.mem.set_u32(me + 0xb0, name);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(calls_to(&log, STRSTR), vec![vec![name, LILY_NAME]]);
        assert!(calls_to(&log, 0x0057_1760).is_empty());
        assert!(calls_to(&log, 0x0087_b360).is_empty());
        // another name: neither condition holds, nothing is attached
        let other = cstr(e, "Someone");
        e.mem.set_u32(me + 0xb0, other);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, 0x0057_1760).is_empty());
        // a worn item without flags, or none at all, is not attached either
        e.mem.set_u32(me + SLOT_218_ANSWER, 1);
        e.mem.set_u32(worn + 8, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, 0x0057_1760).is_empty());
        e.mem.set_u32(worn + 8, 0x33);
        returns(e, 0x004c_8c10, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, 0x0057_1760).is_empty());
        let _ = process;
    }

    #[test]
    fn init_animation_skips_the_process_updates_for_the_player_and_while_loading() {
        let (mut world, process, _) = actor_world(0x40);
        let me = world.refr.addr();
        let e = &mut world.e;
        e.mem.set_u32(me + SLOT_218_ANSWER, 1);
        let gives = slot_address(e, process, 0x160);
        // the player is not given a worn item
        e.set_global(GLOBAL_PLAYER, me);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, gives).is_empty());
        e.set_global(GLOBAL_PLAYER, 0);
        // a process outside the levels 0 and 1 counts as none
        returns(e, PROCESS_LEVEL, 2);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, gives).is_empty());
        returns(e, PROCESS_LEVEL, 1);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(calls_to(&log, gives).len(), 1);
        // while the loading object is busy only the +0x21c refresh remains
        returns(e, 0x0042_ce10, 1);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, gives).is_empty());
        assert!(calls_to(&log, slot_address(e, process, 0x1cc)).is_empty());
        e.mem.set_u32(me + SLOT_21C_ANSWER, 1);
        returns(e, FORCE_NEXT_UPDATE, 1);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(
            calls_to(&log, slot_address(e, process, 0x1cc)),
            vec![vec![process, 1, 0xb1, 0xa1, me]]
        );
    }

    #[test]
    fn init_animation_tells_the_process_about_a_new_animation_and_a_controller() {
        let (mut world, process, _) = actor_world(0x2a);
        let me = world.refr.addr();
        let e = &mut world.e;
        returns(e, 0x005f_0cc0, 0);
        returns(e, 0x0055_85e0, 0x61);
        returns(e, 0x0049_6940, 0x62);
        returns(e, 0x008a_6970, 1);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        // the animation's two words reach the process (virtual +0x18c)
        assert_eq!(
            calls_to(&log, slot_address(e, process, 0x18c)),
            vec![vec![process, 0x62, 0x61]]
        );
        assert_eq!(calls_to(&log, 0x0049_6940).len(), 1);
        // the actor flag of the process (virtual +0x458)
        assert_eq!(
            calls_to(&log, slot_address(e, process, 0x458)),
            vec![vec![process, me, 1]]
        );
        // no new animation or a null word: nothing
        returns(e, 0x0055_85e0, 0);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert!(calls_to(&log, slot_address(e, process, 0x18c)).is_empty());
        // the controller of the 3D is switched on
        returns(e, 0x00a5_c570, 0x66);
        let log = logged(e, |e| {
            e.call(0x0056_59f0, &args![world.refr]);
        });
        assert_eq!(
            calls_to(&log, 0x00a5_c570),
            vec![vec![world.node, CONTROLLER_RECORD]]
        );
        assert_eq!(calls_to(&log, 0x0047_aa60), vec![vec![0x66, 1, 0x40]]);
    }
}
