//! `fallout shared/bgsdestructibleobjectform.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! A form that can be destroyed carries a `BGSDestructibleObjectForm`
//! component: a vtable and a pointer to its [`DestructibleObjectData`]
//! (total health, stage count, flags, an array of
//! [`DestructibleObjectStage`] pointers, and the preloaded replacement
//! models). Each stage names the health percentage at which it applies and
//! what happens there (a model-damage stage number, an explosion, debris, a
//! replacement model, flags). The component sits at a different offset in
//! each form type: `GetDestructionForm` (`00475400`) finds it.
//!
//! This is a debug-style build: every function keeps its locals in memory,
//! so the translations follow the stack-slot logic closely. x87 note: the
//! game computes in extended precision and stores `float` results; the
//! translations compute in `f64` and round to `f32` where the code stores a
//! `float`. The `FISTP` conversions (after the control word is switched to
//! round toward zero) give the "integer indefinite" value for an
//! out-of-range input: [`x87_truncate_i32`].
//!
//! The compiler's exception-unwinding frames (`FS:[0]` chains) are not
//! translated.
//!
//! This unit is translated over several sessions: the first 40 functions
//! (`004751d0` to `00477640`) are here; the next session continues at
//! `00477780`. The layouts, constants and helpers are `pub(crate)` for it.

#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------
// Layouts
// ---------------------------------------------------------------------

layout! {
    /// `BGSDestructibleObjectForm` (Xbox PDB), 8 bytes: the component's
    /// vtable and its data.
    pub struct BGSDestructibleObjectForm: 0x08 {
        /// `pData` (Xbox PDB).
        0x04 pData: Ptr<DestructibleObjectData>,
    }

    /// `DestructibleObjectData` (Xbox PDB), 0x14 bytes on the PC as well.
    pub struct DestructibleObjectData: 0x14 {
        /// `iHealth` (Xbox PDB): the object's total health.
        0x00 iHealth: u32,
        /// `cNumStages` (Xbox PDB).
        0x04 cNumStages: u8,
        /// `cFlags` (Xbox PDB).
        0x05 cFlags: u8,
        /// `pStagesArray` (Xbox PDB): `DestructibleObjectStage**`.
        0x08 pStagesArray: Ptr,
        /// `iReplacementModelRefCount` (Xbox PDB).
        0x0C iReplacementModelRefCount: i32,
        /// `spPreloadedReplacementModels` (Xbox PDB): `NiPointer<QueuedFile>`.
        0x10 spPreloadedReplacementModels: Ptr,
    }

    /// `DestructibleObjectStage` (Xbox PDB), 0x18 bytes on the PC as well.
    pub struct DestructibleObjectStage: 0x18 {
        /// `cModelDamageStage` (Xbox PDB).
        0x00 cModelDamageStage: u8,
        /// `cHealthPercentage` (Xbox PDB): the stage applies once the
        /// object's health has fallen to this percentage.
        0x01 cHealthPercentage: u8,
        /// `cFlags` (Xbox PDB): see [`STAGE_FLAG_CAP_DAMAGE`] and the two
        /// after it.
        0x02 cFlags: u8,
        /// `iSelfDamagePerSecond` (Xbox PDB).
        0x04 iSelfDamagePerSecond: u32,
        /// `pExplosion` (Xbox PDB): `BGSExplosion*`.
        0x08 pExplosion: Ptr,
        /// `pDebris` (Xbox PDB): `BGSDebris*`.
        0x0C pDebris: Ptr,
        /// `iDebrisCount` (Xbox PDB).
        0x10 iDebrisCount: u32,
        /// `pReplacementModel` (Xbox PDB): `TESModelTextureSwap*`.
        0x14 pReplacementModel: Ptr,
    }
}

/// `DestructibleObjectStage::FLAG_CAP_DAMAGE` (Xbox PDB).
pub(crate) const STAGE_FLAG_CAP_DAMAGE: u8 = 1;
/// `DestructibleObjectStage::FLAG_DISABLE_OBJECT` (Xbox PDB).
pub(crate) const STAGE_FLAG_DISABLE_OBJECT: u8 = 2;
/// `DestructibleObjectStage::FLAG_DESTROY_OBJECT` (Xbox PDB).
pub(crate) const STAGE_FLAG_DESTROY_OBJECT: u8 = 4;

// ---------------------------------------------------------------------
// The exe's data
// ---------------------------------------------------------------------

/// `double 0.0`.
pub(crate) const ZERO: u32 = 0x0101_2060;
/// `double 100.0`.
pub(crate) const HUNDRED: u32 = 0x0101_7a40;
/// `double -1.0`: the health `ExtraDataList::GetObjectHealth` reports for
/// an object that was never damaged.
pub(crate) const NO_HEALTH: u32 = 0x0101_a6b0;
/// `double 4294967296.0` (2^32), the divisor that turns a random 32-bit
/// word into a fraction.
pub(crate) const TWO_POW_32: u32 = 0x0101_a6b8;
/// `float -1.0`.
pub(crate) const MINUS_ONE: u32 = 0x0101_2054;
/// `float 1.0` is loaded with `FLD1`; `double 1.0` here.
pub(crate) const ONE: u32 = 0x0101_2070;

/// `BGSDestructibleObjectForm::DestructibleObjects` (Xbox PDB), the static
/// `NiTMap<TESObjectREFR *, unsigned int>` of the references that take
/// self damage; the value is the damage per second.
pub(crate) const DESTRUCTIBLE_OBJECTS: u32 = 0x011c_4104;
/// The `BSRandom` object `RandomFloat` draws from, constructed on first use
/// (bit 0 of the word at [`RANDOM_GENERATOR_INITIALIZED`] says it is).
pub(crate) const RANDOM_GENERATOR: u32 = 0x011c_4180;
pub(crate) const RANDOM_GENERATOR_INITIALIZED: u32 = 0x011c_4b4c;

// ---------------------------------------------------------------------
// Callees outside this file
// ---------------------------------------------------------------------

/// Returns the form type byte (+4) of its `ECX`.
const GET_FORM_TYPE: u32 = 0x0040_1170;
/// Returns the word at +4 of its `ECX`: the component's data pointer.
const GET_COMPONENT_DATA: u32 = 0x0072_6070;
/// `ECX` = reference: its `ExtraDataList`.
const GET_EXTRA_LIST: u32 = 0x005d_43c0;
/// `ExtraDataList::GetObjectHealth` (Xbox PDB), `ECX` = list: the health in
/// ST0, -1.0 for an object that was never damaged.
const GET_OBJECT_HEALTH: u32 = 0x0041_b6b0;
/// `ECX` = list, `float` argument: sets the object's health.
const SET_OBJECT_HEALTH: u32 = 0x0041_b6e0;
/// Truncates its `float` argument toward zero and returns it in ST0.
const TRUNCATE_FLOAT: u32 = 0x0040_4090;
/// Two tests of a reference (`ECX`) after which the damage routine does
/// nothing.
const REFERENCE_TEST_A: u32 = 0x0044_0da0;
const REFERENCE_TEST_B: u32 = 0x0044_0d80;
/// Returns the word its `ECX` points at (`NiPointer` get).
const POINTER_GET: u32 = 0x0055_9450;
/// `NiPointer<NiNode>` constructor (`ECX` = holder, the node as argument)
/// and destructor (`ECX` = holder).
const NODE_HOLDER_CONSTRUCT: u32 = 0x0063_3c90;
const NODE_HOLDER_DESTROY: u32 = 0x0045_cec0;
/// The transform (13 floats: rotation, translation, scale) of an
/// `NiAVObject` (`ECX`).
const GET_TRANSFORM: u32 = 0x0046_1130;
/// The translation (3 floats) of an `NiAVObject` (`ECX`).
const GET_TRANSLATION: u32 = 0x0045_bb80;
/// `NiAVObject::GetWorldBound` (Xbox PDB), `ECX` = object: 4 floats
/// (centre, radius).
const GET_WORLD_BOUND: u32 = 0x0043_d450;
/// Returns the word at +0x40 of its `ECX` (a reference).
const GET_REFERENCE_WORD: u32 = 0x008d_6f30;
/// `NiNode` child count (`ECX` = node) and child at an index.
const GET_CHILD_COUNT: u32 = 0x0043_b480;
const GET_CHILD_AT: u32 = 0x0043_b4a0;
/// Checked cast of a node against a type descriptor (cdecl: descriptor,
/// node): the result is the byte in `AL`.
const CHECKED_CAST: u32 = 0x0043_b300;
/// `BSRangeNode::SetCurrent` (Xbox PDB), `ECX` = node, byte argument: the
/// byte in `AL` is whether the node is now shown.
const RANGE_NODE_SET_CURRENT: u32 = 0x00c4_55a0;
/// `TESObjectREFR::GetOrientation` (Xbox PDB), `ECX` = reference: writes an
/// `NiMatrix3` (9 floats) to the argument.
const GET_ORIENTATION: u32 = 0x0056_fa00;
/// `TESObjectREFR::Update3DPosition` (Xbox PDB), `ECX` = reference.
const UPDATE_3D_POSITION: u32 = 0x0056_2020;
/// `TESObjectREFR::GetScale` (Xbox PDB), `ECX` = reference: ST0.
const GET_SCALE: u32 = 0x0056_7400;
/// `ECX` = debris form: spawns the debris (value of `008d6f30`, position
/// pointer, count, scale).
const SPAWN_DEBRIS: u32 = 0x004f_a6f0;
/// Creates an explosion (cdecl, 16 argument words).
const CREATE_EXPLOSION: u32 = 0x009a_c9c0;
/// `NiTMapBase::RemoveAt` and `SetAt` (Xbox PDB), `ECX` = map.
const MAP_REMOVE_AT: u32 = 0x0040_5430;
const MAP_SET_AT: u32 = 0x0084_4700;
/// `BSSimpleArray` of 16 bytes: constructor and destructor (`ECX`).
const ARRAY_CONSTRUCT: u32 = 0x0047_9110;
const ARRAY_DESTROY: u32 = 0x0047_9140;
/// `ECX` = array: the element count (`iSize`).
const ARRAY_SIZE: u32 = 0x0044_ddc0;
/// `ECX` = array, index argument: pointer to the element.
const ARRAY_ELEMENT: u32 = 0x006a_7ad0;
/// Reads the `float` a setting pointer points at (`ECX` = setting).
const SETTING_POINTER: u32 = 0x0040_3e20;

/// Maps a `float` to the integer `FISTP` stores after the control word is
/// switched to truncation: out of range and NaN give `0x80000000`.
pub(crate) fn x87_truncate_i32(value: f64) -> i32 {
    if value.is_nan() || value >= 2147483648.0 || value <= -2147483649.0 {
        i32::MIN
    } else {
        value as i32
    }
}

/// The same for a 64-bit store (`FISTP qword`): the indefinite value is
/// `0x8000000000000000`.
pub(crate) fn x87_truncate_i64(value: f64) -> i64 {
    if value.is_nan() || value >= 9.223_372_036_854_776e18 || value < -9.223_372_036_854_776e18 {
        i64::MIN
    } else {
        value as i64
    }
}

/// The percentage of `max` that `health` is, rounded up and truncated to a
/// byte: `health * 100 / max` stored as a `float`, then `00476b20`, then
/// `FISTP`, low byte. Every stage lookup does exactly this.
pub(crate) fn health_percentage(e: &mut Engine, health: f32, max: u32) -> u8 {
    let hundred: f64 = e.global(HUNDRED);
    let ratio = (health as f64 * hundred / max as f64) as f32;
    let rounded = fn_00476b20(e, ratio);
    x87_truncate_i32(rounded as f64) as u8
}

/// Reads `count` bytes at `from` and writes them at `to` (the game's
/// `REP MOVSD` of a structure).
fn copy_bytes(e: &mut Engine, from: u32, to: u32, count: u32) {
    let bytes = e.mem.bytes(from, count);
    e.mem.write(to, &bytes);
}

/// The stage pointer at `index` of the data's stage array (no range check).
fn stage_at(
    e: &Engine,
    data: Ptr<DestructibleObjectData>,
    index: u32,
) -> Ptr<DestructibleObjectStage> {
    let stages = e.get(data, DestructibleObjectData::pStagesArray);
    Ptr::new(e.mem.u32(stages.addr().wrapping_add(index.wrapping_mul(4))))
}

// ---------------------------------------------------------------------
// Translations
// ---------------------------------------------------------------------

// Translated from 004751d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `DestructibleObjectData` constructor: everything zero, and the
/// `NiPointer<QueuedFile>` at +0x10 constructed from null (`00528cb0`).
pub fn fn_004751d0(
    e: &mut Engine,
    this: Ptr<DestructibleObjectData>,
) -> Ptr<DestructibleObjectData> {
    e.set(this, DestructibleObjectData::iHealth, 0);
    e.set(this, DestructibleObjectData::cNumStages, 0);
    e.set(this, DestructibleObjectData::cFlags, 0);
    e.set(this, DestructibleObjectData::pStagesArray, Ptr::NULL);
    e.set(this, DestructibleObjectData::iReplacementModelRefCount, 0);
    let preloaded = this.addr() + 0x10;
    e.call(0x0052_8cb0, &args![preloaded, 0u32]);
    this
}

// Translated from 004753a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `DestructibleObjectData` scalar deleting destructor: runs the
/// destructor body (`00475220`) and frees the block when bit 0 of `flags`
/// is set. Returns `this`.
pub fn fn_004753a0(
    e: &mut Engine,
    this: Ptr<DestructibleObjectData>,
    flags: u32,
) -> Ptr<DestructibleObjectData> {
    e.call(0x0047_5220, &args![this]);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Translated from 004753d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::IsDestructible` (Xbox PDB), cdecl: whether
/// the form has a destruction component with at least one valid stage.
pub fn bgs_destructible_object_form_is_destructible(e: &mut Engine, form: Ptr) -> bool {
    let component = bgs_destructible_object_form_get_destruction_form(e, form);
    if component.is_null() {
        return false;
    }
    bgs_destructible_object_form_has_valid_stages(e, component.cast())
}

// Translated from 00475400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::GetDestructionForm` (Xbox PDB), cdecl: the
/// address of the form's `BGSDestructibleObjectForm` component, or 0.
///
/// The component sits at a different offset in each form type; the
/// compiler's switch maps the form type byte (`0x15` to `0x67`, other
/// values give 0) to the offsets below through a byte table (`004758c8`)
/// and a jump table (`00475870`). Two types reach it through a second base
/// class (the intermediate pointer is itself checked for null). The names
/// of the form types are not confirmed from the exe, so the cases are
/// numbered.
pub fn bgs_destructible_object_form_get_destruction_form(e: &mut Engine, form: Ptr) -> Ptr {
    if form.is_null() {
        return Ptr::NULL;
    }
    let form_type = e.call(GET_FORM_TYPE, &args![form]).u32();
    let base = form.addr();
    // Adds `offset` to a non-null pointer.
    let at = |base: u32, offset: i32| -> u32 {
        if base == 0 {
            0
        } else {
            base.wrapping_add(offset as u32)
        }
    };
    let component = match form_type {
        0x15 | 0x16 | 0x17 | 0x1c | 0x27 => at(base, 0x68),
        0x18 => at(base, 0x14c),
        0x19 => at(base, 0x9c),
        0x1a => at(base, 0x144),
        0x1b => at(base, 0x7c),
        0x1d | 0x1e => at(base, 0x94),
        0x1f | 0x2e | 0x32 | 0x67 => at(base, 0x84),
        0x22 => at(at(base, -0x14), 0xc),
        0x25 => at(base, 0x54),
        0x26 => at(at(base, -0xc), 0x74),
        0x28 => at(base, 0xb4),
        0x29 => at(base, 0x88),
        0x2a | 0x2b => at(base, 0x104),
        0x2f => at(base, 0xa4),
        0x33 => at(base, 0x58),
        _ => 0,
    };
    Ptr::new(component)
}

// Translated from 00475920 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::GetModelSwapIndex` (Xbox PDB), cdecl: the
/// index of the stage whose replacement model is `model`, or 0xffffffff.
pub fn bgs_destructible_object_form_get_model_swap_index(
    e: &mut Engine,
    form: Ptr,
    model: u32,
) -> u32 {
    let component = bgs_destructible_object_form_get_destruction_form(e, form);
    if component.is_null() {
        return u32::MAX;
    }
    let data: Ptr<DestructibleObjectData> = e.call(GET_COMPONENT_DATA, &args![component]).ptr();
    if data.is_null() {
        return u32::MAX;
    }
    let count = e.get(data, DestructibleObjectData::cNumStages) as u32;
    for index in 0..count {
        let stage = stage_at(e, data, index);
        if e.get(stage, DestructibleObjectStage::pReplacementModel)
            .addr()
            == model
        {
            return index;
        }
    }
    u32::MAX
}

// Translated from 004759a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Cdecl: the replacement model of stage `index` of the form's destruction
/// component, 0 when there is none or the index is -1 or past the last
/// stage. (The index is compared as a signed number: another negative
/// index reads outside the array, as in the game.)
pub fn fn_004759a0(e: &mut Engine, form: Ptr, index: i32) -> u32 {
    let component = bgs_destructible_object_form_get_destruction_form(e, form);
    if component.is_null() || index == -1 {
        return 0;
    }
    let data: Ptr<DestructibleObjectData> = e.call(GET_COMPONENT_DATA, &args![component]).ptr();
    if data.is_null() {
        return 0;
    }
    if index < e.get(data, DestructibleObjectData::cNumStages) as i32 {
        let stage = stage_at(e, data, index as u32);
        e.get(stage, DestructibleObjectStage::pReplacementModel)
            .addr()
    } else {
        0
    }
}

// Translated from 00475a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::HasValidStages` (Xbox PDB): whether some
/// stage is non-null and has a replacement model, an explosion, a debris
/// form or the disable flag.
pub fn bgs_destructible_object_form_has_valid_stages(
    e: &mut Engine,
    this: Ptr<BGSDestructibleObjectForm>,
) -> bool {
    let data = e.get(this, BGSDestructibleObjectForm::pData);
    if data.is_null() {
        return false;
    }
    let mut index = 0u32;
    while index < e.get(data, DestructibleObjectData::cNumStages) as u32 {
        let stage = stage_at(e, data, index);
        if !stage.is_null()
            && (!e
                .get(stage, DestructibleObjectStage::pReplacementModel)
                .is_null()
                || !e.get(stage, DestructibleObjectStage::pExplosion).is_null()
                || !e.get(stage, DestructibleObjectStage::pDebris).is_null()
                || fn_00475a90(e, stage))
        {
            return true;
        }
        index += 1;
    }
    false
}

// Translated from 00475a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the stage has [`STAGE_FLAG_DISABLE_OBJECT`].
pub fn fn_00475a90(e: &mut Engine, stage: Ptr<DestructibleObjectStage>) -> bool {
    e.get(stage, DestructibleObjectStage::cFlags) & STAGE_FLAG_DISABLE_OBJECT != 0
}

// Translated from 00475ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The reference's total health for the percentage arithmetic. When
/// virtual slot 0x100 of the reference answers true, the value of the
/// interface embedded at +0xa4 (its slot 4, argument 0x10, a `float`
/// result) truncated to an integer; otherwise the component's `iHealth`.
pub fn fn_00475ab0(e: &mut Engine, this: Ptr<BGSDestructibleObjectForm>, reference: Ptr) -> u32 {
    if e.vcall(reference.addr(), 0x100, &args![]).bool() {
        let interface = reference.addr() + 0xa4;
        let value = e.vcall(interface, 4, &args![0x10u32]).f64();
        x87_truncate_i64(value) as u32
    } else {
        let data = e.get(this, BGSDestructibleObjectForm::pData);
        e.get(data, DestructibleObjectData::iHealth)
    }
}

// Translated from 004768c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiMatrix3::Transpose` (Xbox PDB): writes the transpose of the matrix
/// `this` to `result` and returns `result`. The three rows are copied to a
/// stack array of `NiPoint3` (the array constructor `00401050` runs the
/// no-op `NiPoint3` constructor `006815c0` on each), then stored as the
/// columns of the result.
pub fn ni_matrix3_transpose(e: &mut Engine, this: Ptr, result: Ptr) -> Ptr {
    e.with_stack(0x24, |e, rows| {
        let row = |i: u32| rows.addr() + 12 * i;
        e.call(0x0040_1050, &args![rows, 12u32, 3i32, 0x0068_15c0u32]);
        fn_00476930(e, this, 0, Ptr::new(row(0)));
        fn_00476930(e, this, 1, Ptr::new(row(1)));
        fn_00476930(e, this, 2, Ptr::new(row(2)));
        fn_00476980(
            e,
            result,
            Ptr::new(row(0)),
            Ptr::new(row(1)),
            Ptr::new(row(2)),
        );
    });
    result
}

// Translated from 00476930 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies row `row` (3 floats) of the matrix `this` to `out`.
pub fn fn_00476930(e: &mut Engine, this: Ptr, row: u32, out: Ptr) {
    let from = this.addr().wrapping_add(row.wrapping_mul(12));
    for i in 0..3 {
        let value = e.mem.f32(from + 4 * i);
        e.mem.set_f32(out.addr() + 4 * i, value);
    }
}

// Translated from 00476980 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores three `NiPoint3` as the columns 0, 1, 2 of the matrix `this`.
/// Returns `this`.
pub fn fn_00476980(e: &mut Engine, this: Ptr, column0: Ptr, column1: Ptr, column2: Ptr) -> Ptr {
    fn_004769c0(e, this, 0, column0);
    fn_004769c0(e, this, 1, column1);
    fn_004769c0(e, this, 2, column2);
    this
}

// Translated from 004769c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the three floats at `source` in column `column` of the matrix
/// `this` (elements `column`, `column + 3`, `column + 6`).
pub fn fn_004769c0(e: &mut Engine, this: Ptr, column: u32, source: Ptr) {
    for i in 0..3 {
        let value = e.mem.f32(source.addr() + 4 * i);
        let to = this
            .addr()
            .wrapping_add(column.wrapping_mul(4))
            .wrapping_add(12 * i);
        e.mem.set_f32(to, value);
    }
}

// Translated from 00476a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the stage has [`STAGE_FLAG_CAP_DAMAGE`].
pub fn fn_00476a00(e: &mut Engine, stage: Ptr<DestructibleObjectStage>) -> bool {
    e.get(stage, DestructibleObjectStage::cFlags) & STAGE_FLAG_CAP_DAMAGE != 0
}

// Translated from 00476a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the stage has [`STAGE_FLAG_DESTROY_OBJECT`].
pub fn fn_00476a20(e: &mut Engine, stage: Ptr<DestructibleObjectStage>) -> bool {
    e.get(stage, DestructibleObjectStage::cFlags) & STAGE_FLAG_DESTROY_OBJECT != 0
}

// Translated from 00476a40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stage `index` of the component, null when there is no data or the index
/// is not below the stage count.
pub fn fn_00476a40(
    e: &mut Engine,
    this: Ptr<BGSDestructibleObjectForm>,
    index: u32,
) -> Ptr<DestructibleObjectStage> {
    let data = e.get(this, BGSDestructibleObjectForm::pData);
    if data.is_null() || index >= e.get(data, DestructibleObjectData::cNumStages) as u32 {
        return Ptr::NULL;
    }
    stage_at(e, data, index)
}

// Translated from 00476a80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs the (empty) constructors `006815c0` of an `NiMatrix3` at `this` and
/// an `NiPoint3` at `this + 0x24`: the first two members of an
/// `NiTransform`. Returns `this`.
pub fn fn_00476a80(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(0x0068_15c0, &args![this]);
    let translation = this.addr() + 0x24;
    e.call(0x0068_15c0, &args![translation]);
    this
}

// Translated from 00476ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the two `float`s at +0xb4 and +0xb8 of `this` to 1.0 and then runs
/// `00476ae0` with 1 (which sets a flag through `0043b370`). `this` is the
/// object the damage routine finds at slot 0x10 of the node's slot-0xc
/// result (a Havok collision object).
pub fn fn_00476ab0(e: &mut Engine, this: Ptr) {
    e.mem.set_f32(this.addr() + 0xb8, 1.0);
    e.mem.set_f32(this.addr() + 0xb4, 1.0);
    fn_00476ae0(e, this, 1);
}

// Translated from 00476ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards its byte argument to `00476b00`.
pub fn fn_00476ae0(e: &mut Engine, this: Ptr, flag: u8) {
    fn_00476b00(e, this, flag);
}

// Translated from 00476b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `0043b370` on `this` with the byte and the flag word 0x4000.
pub fn fn_00476b00(e: &mut Engine, this: Ptr, flag: u8) {
    e.call(0x0043_b370, &args![this, flag, 0x4000u32]);
}

// Translated from 00476b20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Rounds a `float` up: the truncation `00404090` of `value`, plus 1.0 when
/// that is below `value`.
pub fn fn_00476b20(e: &mut Engine, value: f32) -> f32 {
    let truncated = e.call(TRUNCATE_FLOAT, &args![value]).f32();
    let zero: f64 = e.global(ZERO);
    let result = if (truncated as f64 - value as f64) < zero {
        let one: f64 = e.global(ONE);
        truncated as f64 + one
    } else {
        truncated as f64
    };
    result as f32
}

// Translated from 00476b70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `RandomFloat` (Xbox PDB), cdecl: a random `float` in `[min, max]` drawn
/// from the static generator (`00476c00`).
pub fn random_float(e: &mut Engine, min: f32, max: f32) -> f32 {
    let generator = fn_00476c00(e);
    fn_00476b90(e, generator, min, max)
}

// Translated from 00476b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `min + (max - min) * r / 2^32` for a random 32-bit word `r` of the
/// generator `this`, computed in extended precision and stored as a
/// `float`.
pub fn fn_00476b90(e: &mut Engine, this: Ptr, min: f32, max: f32) -> f32 {
    let word = fn_00476be0(e, this);
    let divisor: f64 = e.global(TWO_POW_32);
    let span = max as f64 - min as f64;
    ((span * word as f64) / divisor + min as f64) as f32
}

// Translated from 00476be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSRandom::UnsignedInt` (`00aa5230`) with the largest bound,
/// 0xffffffff.
pub fn fn_00476be0(e: &mut Engine, this: Ptr) -> u32 {
    e.call(0x00aa_5230, &args![this, u32::MAX]).u32()
}

// Translated from 00476c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The static `BSRandom` generator, constructed (`BSRandom::BSRandom`,
/// `00aa5140`) on the first call.
pub fn fn_00476c00(e: &mut Engine) -> Ptr {
    let guard: u32 = e.global(RANDOM_GENERATOR_INITIALIZED);
    if guard & 1 == 0 {
        e.set_global(RANDOM_GENERATOR_INITIALIZED, guard | 1);
        e.call(0x00aa_5140, &args![RANDOM_GENERATOR]);
    }
    Ptr::new(RANDOM_GENERATOR)
}

// Translated from 00476c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the word at +0xe0 of `this` (`pWeaponSource` of an
/// `Explosion` in the Xbox PDB, 0xf0 there).
pub fn fn_00476c70(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0xe0, value);
}

// Translated from 00476c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at +0xf8 of `this`.
pub fn fn_00476c90(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0xf8)
}

// Translated from 00476cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks the node tree below `node` and creates the explosion form
/// `explosion` at every node that is a checked cast to the type at
/// `01202e7c` and that `BSRangeNode::SetCurrent` (byte `selector`) accepts.
/// The explosion is created with the node's transform and translation, the
/// word `008d6f30` reads from `reference`, and `reference` twice. Returns
/// the number of explosions created. `this` (the `ECX` the game passes) is
/// only handed on to the recursion.
#[allow(clippy::only_used_in_recursion)]
pub fn fn_00476cb0(
    e: &mut Engine,
    this: Ptr<BGSDestructibleObjectForm>,
    node: Ptr,
    explosion: Ptr,
    selector: u8,
    reference: Ptr,
) -> i32 {
    if node.is_null() {
        return 0;
    }
    let mut created = 0i32;
    if e.call(CHECKED_CAST, &args![0x0120_2e7cu32, node]).bool()
        && e.call(RANGE_NODE_SET_CURRENT, &args![node, selector])
            .bool()
    {
        let transform = e.call(GET_TRANSFORM, &args![node]).u32();
        let translation = e.call(GET_TRANSLATION, &args![node]).u32();
        let word = e.call(GET_REFERENCE_WORD, &args![reference]).u32();
        let mut words = vec![explosion.addr(), reference.addr(), reference.addr(), word];
        for i in 0..3 {
            words.push(e.mem.u32(translation + 4 * i));
        }
        for i in 0..9 {
            words.push(e.mem.u32(transform + 4 * i));
        }
        e.call(CREATE_EXPLOSION, &words);
        created += 1;
    }
    let children = e.vcall(node.addr(), 0xc, &args![]).u32();
    if children != 0 {
        let mut index = 0u32;
        while index < e.call(GET_CHILD_COUNT, &args![children]).u32() {
            let child = e.call(GET_CHILD_AT, &args![children, index]).ptr::<()>();
            created =
                created.wrapping_add(fn_00476cb0(e, this, child, explosion, selector, reference));
            index += 1;
        }
    }
    created
}

// Translated from 00476dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::SpawnDebrisAtDebrisNodes` (Xbox PDB): walks
/// the node tree below `node` and, at every node that is a checked cast to
/// the type at `01202e84` and that `BSRangeNode::SetCurrent` (byte
/// `selector`) accepts, spawns `count` of the debris form `debris` (with
/// the node's translation, the word `008d6f30` reads from `reference`, and
/// the reference's scale). Returns the number of nodes that got debris.
#[allow(clippy::only_used_in_recursion)]
pub fn bgs_destructible_object_form_spawn_debris_at_debris_nodes(
    e: &mut Engine,
    this: Ptr<BGSDestructibleObjectForm>,
    node: Ptr,
    debris: Ptr,
    selector: u8,
    reference: Ptr,
    count: u32,
) -> i32 {
    if node.is_null() {
        return 0;
    }
    let mut spawned = 0i32;
    if e.call(CHECKED_CAST, &args![0x0120_2e84u32, node]).bool()
        && e.call(RANGE_NODE_SET_CURRENT, &args![node, selector])
            .bool()
    {
        let scale = e.call(GET_SCALE, &args![reference]).f32();
        let translation = e.call(GET_TRANSLATION, &args![node]).u32();
        let word = e.call(GET_REFERENCE_WORD, &args![reference]).u32();
        e.call(
            SPAWN_DEBRIS,
            &args![debris, word, translation, count, scale],
        );
        spawned += 1;
    }
    let children = e.vcall(node.addr(), 0xc, &args![]).u32();
    if children != 0 {
        let mut index = 0u32;
        while index < e.call(GET_CHILD_COUNT, &args![children]).u32() {
            let child = e.call(GET_CHILD_AT, &args![children, index]).ptr::<()>();
            spawned =
                spawned.wrapping_add(bgs_destructible_object_form_spawn_debris_at_debris_nodes(
                    e, this, child, debris, selector, reference, count,
                ));
            index += 1;
        }
    }
    spawned
}

// Translated from 00476ec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::UpdateDamageStageNodes` (Xbox PDB), cdecl:
/// walks the node tree below `node` and calls `BSRangeNode::SetCurrent`
/// with the damage stage `stage` on every node that is a checked cast to
/// the type at `01202e8c`.
pub fn bgs_destructible_object_form_update_damage_stage_nodes(
    e: &mut Engine,
    node: Ptr,
    stage: u8,
) {
    if node.is_null() {
        return;
    }
    if e.call(CHECKED_CAST, &args![0x0120_2e8cu32, node]).bool() {
        e.call(RANGE_NODE_SET_CURRENT, &args![node, stage]);
    }
    let children = e.vcall(node.addr(), 0xc, &args![]).u32();
    if children != 0 {
        let mut index = 0u32;
        while index < e.call(GET_CHILD_COUNT, &args![children]).u32() {
            let child = e.call(GET_CHILD_AT, &args![children, index]).ptr::<()>();
            bgs_destructible_object_form_update_damage_stage_nodes(e, child, stage);
            index += 1;
        }
    }
}

// Translated from 00476f50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::UpdateCurrentDamageStage` (Xbox PDB): sets
/// the damage-stage nodes of the reference's 3D from its current health.
/// Returns false when the reference has no 3D (virtual slot 0x1d0 answers
/// 0), or when no damage stage applies and `force` is false.
///
/// The damage stage is that of the last stage (in order) whose percentage is
/// at least the health percentage and whose `cModelDamageStage` is not 0;
/// a health of -1.0 (never damaged) looks nothing up.
pub fn bgs_destructible_object_form_update_current_damage_stage(
    e: &mut Engine,
    this: Ptr<BGSDestructibleObjectForm>,
    reference: Ptr,
    force: bool,
) -> bool {
    if e.vcall(reference.addr(), 0x1d0, &args![]).u32() == 0 {
        return false;
    }
    let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
    let health = e.call(GET_OBJECT_HEALTH, &args![list]).f32();
    let mut damage_stage = 0u32;
    let no_health: f64 = e.global(NO_HEALTH);
    if health as f64 == no_health {
        if !force {
            return false;
        }
    } else {
        let data = e.get(this, BGSDestructibleObjectForm::pData);
        let max = e.get(data, DestructibleObjectData::iHealth);
        let percentage = health_percentage(e, health, max);
        let mut index = 0u32;
        while index < e.get(data, DestructibleObjectData::cNumStages) as u32 {
            let stage = fn_00476a40(e, this, index);
            if e.get(stage, DestructibleObjectStage::cHealthPercentage) < percentage {
                break;
            }
            let model_stage = e.get(stage, DestructibleObjectStage::cModelDamageStage);
            if model_stage != 0 {
                damage_stage = model_stage as u32;
            }
            index += 1;
        }
    }
    if damage_stage == 0 && !force {
        return false;
    }
    let node = e.vcall(reference.addr(), 0x1d0, &args![]).ptr::<()>();
    bgs_destructible_object_form_update_damage_stage_nodes(e, node, damage_stage as u8);
    true
}

// Translated from 00477090 (decompiled, FalloutNV.exe 1.4.0.525)
/// From the reference's health percentage, walks the stages: a stage whose
/// percentage is below it ends the walk if the previous stage had no
/// self-damage value (`iSelfDamagePerSecond` 0; the first stage counts as
/// having none), otherwise it records the stage's replacement model; every
/// stage visited becomes the "previous" one. When the last stage visited
/// has a percentage below the reference's, the health is set to that
/// percentage of the total (`0041b6e0`), the replacement model (if any) is
/// applied (virtual slot 0x1cc with 0 and 1, then `0042e150` with the model
/// and the word `007af430` gives), and the damage-stage nodes are refreshed
/// (`UpdateCurrentDamageStage` with `force` false). Does nothing for a
/// health of -1.0.
pub fn fn_00477090(e: &mut Engine, this: Ptr<BGSDestructibleObjectForm>, reference: Ptr) {
    let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
    let health = e.call(GET_OBJECT_HEALTH, &args![list]).f32();
    let no_health: f64 = e.global(NO_HEALTH);
    if health as f64 == no_health {
        return;
    }
    let data = e.get(this, BGSDestructibleObjectForm::pData);
    let max = e.get(data, DestructibleObjectData::iHealth);
    let percentage = health_percentage(e, health, max);
    let mut previous_rate = 0u32;
    let mut previous_percentage = 100u8;
    let mut model = 0u32;
    let mut index = 0u32;
    while index < e.get(data, DestructibleObjectData::cNumStages) as u32 {
        let stage = stage_at(e, data, index);
        if e.get(stage, DestructibleObjectStage::cHealthPercentage) < percentage {
            if previous_rate == 0 {
                break;
            }
            let replacement = e.get(stage, DestructibleObjectStage::pReplacementModel);
            if !replacement.is_null() {
                model = replacement.addr();
            }
        }
        previous_rate = e.get(stage, DestructibleObjectStage::iSelfDamagePerSecond);
        previous_percentage = e.get(stage, DestructibleObjectStage::cHealthPercentage);
        index += 1;
    }
    if previous_percentage < percentage {
        let hundred: f64 = e.global(HUNDRED);
        let new_health = (previous_percentage as f64 / hundred * max as f64) as f32;
        let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
        e.call(SET_OBJECT_HEALTH, &args![list, new_health]);
        if model != 0 {
            e.vcall(reference.addr(), 0x1cc, &args![0u32, 1u32]);
            let form = e.call(0x007a_f430, &args![reference]).u32();
            let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
            e.call(0x0042_e150, &args![list, model, form]);
        }
        bgs_destructible_object_form_update_current_damage_stage(e, this, reference, false);
    }
}

// Translated from 00477230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks the node tree below `node` and appends to the `BSSimpleArray` at
/// `array` the pointer of every node-as-container (`node`'s virtual slot
/// 0xc result) that passes `009a3830` against the word `004772e0` returns
/// (and whose `00413f40`/`0043b1b0` lookup is non-zero). The pointer is
/// passed to the array's add routine (`007cb2e0`) by address of a local.
#[allow(clippy::only_used_in_recursion)]
pub fn fn_00477230(e: &mut Engine, this: Ptr<BGSDestructibleObjectForm>, node: Ptr, array: Ptr) {
    if node.is_null() {
        return;
    }
    let children = e.vcall(node.addr(), 0xc, &args![]).u32();
    if children == 0 {
        return;
    }
    let lookup = e.call(0x0041_3f40, &args![children]).u32();
    if e.call(0x0043_b1b0, &args![lookup]).u32() != 0 {
        let filter = fn_004772e0(e);
        let lookup = e.call(0x0041_3f40, &args![children]).u32();
        if e.call(0x009a_3830, &args![lookup, filter]).bool() {
            let slot = e.mem.alloc(4);
            e.mem.set_u32(slot, children);
            e.call(0x007c_b2e0, &args![array, slot]);
            e.mem.free(slot);
        }
    }
    let mut index = 0u32;
    while index < e.call(GET_CHILD_COUNT, &args![children]).u32() {
        let child = e.call(GET_CHILD_AT, &args![children, index]).ptr::<()>();
        fn_00477230(e, this, child, array);
        index += 1;
    }
}

// Translated from 004772e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `011c61d8`.
pub fn fn_004772e0(e: &mut Engine) -> u32 {
    e.global(0x011c_61d8)
}

// Translated from 004772f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Applies self damage: for every `(reference, damage per second)` entry of
/// the destructible-objects map (`DESTRUCTIBLE_OBJECTS`) whose reference
/// passes both tests `00440da0` and `00440d80` (they must be false),
/// computes `rate * scale` (the scale is the `float` `0084d030` reads from
/// the object at `011f6394`) and hands it to the reference: when the word
/// `0043d4d0` returns for the object at `011c3ea4` is 1, to virtual slot
/// 0x338 (rate, 0.0, 0) for a reference whose slot 0x100 answers true and to
/// slot 0x144 (rate, 1) for the others; otherwise to `0087af50`, the
/// object `004537b0` returns taking (reference, rate, 1).
pub fn fn_004772f0(e: &mut Engine) {
    let scale = e.call(0x0084_d030, &args![0x011f_6394u32]).f32();
    let mut position = e.call(0x004b_9ba0, &args![DESTRUCTIBLE_OBJECTS]).u32();
    // The iterator position, the reference and the damage rate: locals
    // whose addresses `006b7f20` receives.
    let locals = e.mem.alloc(12);
    while position != 0 {
        e.mem.set_u32(locals, position);
        e.mem.set_u32(locals + 4, 0);
        e.mem.set_u32(locals + 8, 0);
        e.call(
            0x006b_7f20,
            &args![DESTRUCTIBLE_OBJECTS, locals, locals + 4, locals + 8],
        );
        position = e.mem.u32(locals);
        let reference = e.mem.u32(locals + 4);
        let rate = e.mem.u32(locals + 8);
        if reference != 0
            && !e.call(REFERENCE_TEST_A, &args![reference]).bool()
            && !e.call(REFERENCE_TEST_B, &args![reference]).bool()
        {
            let damage = (rate as f64 * scale as f64) as f32;
            let mode_pointer = e.call(0x0043_d4d0, &args![0x011c_3ea4u32]).u32();
            if e.mem.u32(mode_pointer) == 1 {
                if e.vcall(reference, 0x100, &args![]).bool() {
                    e.vcall(reference, 0x338, &args![damage, 0.0f32, 0u32]);
                } else {
                    e.vcall(reference, 0x144, &args![damage, 1u32]);
                }
            } else {
                let queue = e.call(0x0045_37b0, &args![]).u32();
                e.call(0x0087_af50, &args![queue, reference, damage, 1u32]);
            }
        }
    }
    e.mem.free(locals);
}

// Translated from 00477410 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes `reference` from the destructible-objects map
/// (`NiTMapBase::RemoveAt`, `00405430`).
pub fn fn_00477410(e: &mut Engine, reference: Ptr) {
    e.call(MAP_REMOVE_AT, &args![DESTRUCTIBLE_OBJECTS, reference]);
}

// Translated from 00477430 (decompiled, FalloutNV.exe 1.4.0.525)
/// Cdecl: the index of the first stage of `reference`'s destruction
/// component whose percentage is below the reference's health percentage,
/// or the stage count when there is none; 0xffffffff when `reference` is
/// null, has no destruction data or has never been damaged (-1.0).
pub fn fn_00477430(e: &mut Engine, reference: Ptr) -> u32 {
    if reference.is_null() {
        return u32::MAX;
    }
    let form = e.call(0x007a_f430, &args![reference]).u32();
    let component = bgs_destructible_object_form_get_destruction_form(e, Ptr::new(form));
    let data: Ptr<DestructibleObjectData> = if component.is_null() {
        Ptr::NULL
    } else {
        e.call(GET_COMPONENT_DATA, &args![component]).ptr()
    };
    if data.is_null() {
        return u32::MAX;
    }
    let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
    let health = e.call(GET_OBJECT_HEALTH, &args![list]).f32();
    let no_health: f64 = e.global(NO_HEALTH);
    if health as f64 == no_health {
        return u32::MAX;
    }
    let max = fn_00475ab0(e, component.cast(), reference);
    let percentage = health_percentage(e, health, max);
    let count = e.get(data, DestructibleObjectData::cNumStages) as u32;
    for index in 0..count {
        let stage = stage_at(e, data, index);
        if e.get(stage, DestructibleObjectStage::cHealthPercentage) < percentage {
            return index;
        }
    }
    count
}

// Translated from 00477560 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::HideSmallDebris` (Xbox PDB), cdecl: when the
/// node has a collision object (`0043b610`) whose pointer at +0x10 (via
/// `006fa820`) leads to a value with type 0x13 (`0043b4f0`/`0043b4d0`),
/// the node is hidden (`00450f90` with 1) and the result is true;
/// otherwise the same is tried on every child, and the result is true if
/// any child was hidden.
pub fn bgs_destructible_object_form_hide_small_debris(e: &mut Engine, node: Ptr) -> bool {
    let mut result = false;
    if node.is_null() {
        return result;
    }
    let mut hide = false;
    let collision = e.call(0x0043_b610, &args![node]).u32();
    if collision != 0 {
        let shape = e.call(0x006f_a820, &args![collision]).u32();
        if shape != 0 {
            let out = e.mem.alloc(8);
            let value = e.call(0x0043_b4f0, &args![shape, out]).u32();
            if e.call(0x0043_b4d0, &args![value]).u32() == 0x13 {
                hide = true;
            }
            e.mem.free(out);
        }
    }
    if hide {
        result = true;
        e.call(0x0045_0f90, &args![node, 1u32]);
    } else {
        let children = e.vcall(node.addr(), 0xc, &args![]).u32();
        if children != 0 {
            let count = e.call(GET_CHILD_COUNT, &args![children]).u32();
            for index in 0..count {
                let child = e.call(GET_CHILD_AT, &args![children, index]).ptr::<()>();
                if bgs_destructible_object_form_hide_small_debris(e, child) {
                    result = true;
                }
            }
        }
    }
    result
}

// Translated from 00477640 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BGSDestructibleObjectForm::QueueFiles` (Xbox PDB): for every stage
/// queues the replacement model (`00443d30` on the object at `011c3b3c`
/// with `arg2`, `arg3`, the LOD multiplier `TES::GetLODMult` (`0045c6b0`)
/// of `arg1`, 1, 0, 0), and calls virtual slot 0x10 (`arg2`, `arg3`) of
/// the `TESModel` embedded at +0x64 of the stage's explosion and at +0x18
/// of its debris form.
pub fn bgs_destructible_object_form_queue_files(
    e: &mut Engine,
    this: Ptr<BGSDestructibleObjectForm>,
    arg1: u32,
    arg2: u32,
    arg3: u32,
) {
    let data = e.get(this, BGSDestructibleObjectForm::pData);
    if data.is_null() {
        return;
    }
    let mut index = 0u32;
    while index < e.get(data, DestructibleObjectData::cNumStages) as u32 {
        let stage = stage_at(e, data, index);
        let model = e.get(stage, DestructibleObjectStage::pReplacementModel);
        if !model.is_null() {
            let lod = e.call(0x0045_c6b0, &args![arg1]).u32();
            let queue: u32 = e.global(0x011c_3b3c);
            e.call(
                0x0044_3d30,
                &args![queue, model, arg2, arg3, lod, 1u32, 0u32, 0u32],
            );
        }
        let stage = stage_at(e, data, index);
        let explosion = e.get(stage, DestructibleObjectStage::pExplosion);
        if !explosion.is_null() {
            e.vcall(explosion.addr() + 0x64, 0x10, &args![arg2, arg3]);
        }
        let stage = stage_at(e, data, index);
        let debris = e.get(stage, DestructibleObjectStage::pDebris);
        if !debris.is_null() {
            e.vcall(debris.addr() + 0x18, 0x10, &args![arg2, arg3]);
        }
        index += 1;
    }
}

// Translated from 00475b20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Applies `damage` to a destructible `reference` (the third stack word,
/// `_unused_3`, is never read).
///
/// Does nothing without data, without a reference, for a damage that is not
/// above 0, when either reference test (`00440da0`, `00440d80`) is true, or
/// when the reference has no 3D (virtual slot 0x1d0). The total health is
/// `00475ab0`; the current health is the extra data's (`GetObjectHealth`),
/// or the total when it reports -1.0, and nothing happens at 0. The new
/// health is the current minus the damage, replaced by the actor value
/// (slot 0x100 true: slot 8 of the interface at +0xa4 with 0x10) and
/// clamped to 0. The old and new health percentages (rounded up, bytes)
/// select the stages crossed:
///
/// - a stage with a percentage below the old and at least the new one is
///   crossed: its damage stage, replacement model and self-damage value
///   are remembered (`crossed_*`), and its flags act at once (disable:
///   virtual slot 0x224 true runs `009bc8f0`, otherwise
///   `Script::AddPendingDisabledReference` and `TESForm::SetDelete`;
///   destroy: `00484650`; cap damage: the new health becomes that stage's
///   percentage of the total, and the walk stops);
/// - a stage at or above the old percentage and above the new one is
///   remembered as passed earlier (`earlier_*`).
///
/// Then the new health is stored (`00484650` when it is 0 or less first),
/// virtual slot 0x48 is called with 0x80000000, and when some stage was
/// crossed (`crossed_end > earlier_end`): the replacement model is swapped
/// in (keeping the reference's orientation and position relative to the
/// new 3D, and activating the new 3D in the Havok world) or, without one,
/// the damage stage nodes are updated; the self-damage map is updated
/// (set when the crossed stage has a value and health remains, removed
/// otherwise). Then for each crossed stage the action flag `0x200000` is
/// set (`Script::SetActionFlag`), and debris and explosions are spawned at
/// the 3D's nodes, falling back to the reference's own position. At
/// exactly 0 health `004778a0` runs. Finally the combat threat map is told
/// of the object (`009a54a0`, or `009a51e0` when no value results).
///
/// The local `BSSimpleArray`s and `NiTransform`s the game keeps on its
/// stack are engine blocks here; the unwinding frame is not translated.
pub fn fn_00475b20(
    e: &mut Engine,
    this: Ptr<BGSDestructibleObjectForm>,
    reference: Ptr,
    damage: f32,
    _unused_3: u32,
) {
    let data = e.get(this, BGSDestructibleObjectForm::pData);
    let zero: f64 = e.global(ZERO);
    if data.is_null() || reference.is_null() || damage as f64 <= zero {
        return;
    }
    if e.call(REFERENCE_TEST_A, &args![reference]).bool()
        || e.call(REFERENCE_TEST_B, &args![reference]).bool()
    {
        return;
    }
    if e.vcall(reference.addr(), 0x1d0, &args![]).u32() == 0 {
        return;
    }
    let max_health = fn_00475ab0(e, this, reference);
    let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
    let mut health = e.call(GET_OBJECT_HEALTH, &args![list]).f32();
    let no_health: f64 = e.global(NO_HEALTH);
    if health as f64 == no_health {
        health = max_health as f32;
    }
    if health as f64 == zero {
        return;
    }
    let old_percentage = health_percentage(e, health, max_health);
    let mut new_health = (health as f64 - damage as f64) as f32;
    if e.vcall(reference.addr(), 0x100, &args![]).bool() {
        let value = e.vcall(reference.addr() + 0xa4, 8, &args![0x10u32]).i32();
        new_health = value as f32;
    }
    if new_health as f64 <= zero {
        new_health = 0.0;
    }
    let mut new_percentage = health_percentage(e, new_health, max_health);
    if new_percentage == 0 && new_health as f64 > zero {
        new_percentage = 1;
    }

    // `NiPointer<NiNode>` holding the reference's 3D for the debris and
    // explosion walks.
    let holder = e.mem.alloc(4);
    let root = e.vcall(reference.addr(), 0x1d0, &args![]).u32();
    e.call(NODE_HOLDER_CONSTRUCT, &args![holder, root]);

    let mut crossed_end = 0u32;
    let mut earlier_end = 0u32;
    let mut crossed_model = 0u32;
    let mut crossed_damage_stage = 0u32;
    let mut earlier_damage_stage = 0u32;
    let mut crossed_rate = 0u32;
    let mut remaining_rate = 0u32;
    let hundred: f64 = e.global(HUNDRED);

    let mut index = 0u32;
    while index < e.get(data, DestructibleObjectData::cNumStages) as u32 {
        let stage = fn_00476a40(e, this, index);
        let stage_percentage = e.get(stage, DestructibleObjectStage::cHealthPercentage);
        if stage_percentage < old_percentage && stage_percentage >= new_percentage {
            let model_stage = e.get(stage, DestructibleObjectStage::cModelDamageStage);
            if model_stage != 0 {
                crossed_damage_stage = model_stage as u32;
            }
            let replacement = e.get(stage, DestructibleObjectStage::pReplacementModel);
            if !replacement.is_null() {
                crossed_model = replacement.addr();
            }
            crossed_rate = e.get(stage, DestructibleObjectStage::iSelfDamagePerSecond);
            crossed_end = index + 1;
            if fn_00475a90(e, stage) {
                if e.vcall(reference.addr(), 0x224, &args![]).bool() {
                    e.call(0x009b_c8f0, &args![reference]);
                } else {
                    e.call(0x005a_a500, &args![reference, 0u32]);
                    e.call(0x0048_4530, &args![reference, 1u32]);
                }
            }
            if fn_00476a20(e, stage) {
                e.call(0x0048_4650, &args![reference, 1u32]);
            }
            if fn_00476a00(e, stage) {
                let product = stage_percentage as u32 * max_health;
                new_health = (product as f64 / hundred) as f32;
                new_percentage = stage_percentage;
                break;
            }
        } else if old_percentage <= stage_percentage && new_percentage < stage_percentage {
            earlier_end = index + 1;
            let model_stage = e.get(stage, DestructibleObjectStage::cModelDamageStage);
            if model_stage != 0 {
                earlier_damage_stage = model_stage as u32;
            }
            remaining_rate = e.get(stage, DestructibleObjectStage::iSelfDamagePerSecond);
        }
        index += 1;
    }

    if new_health as f64 <= zero {
        e.call(0x0048_4650, &args![reference, 1u32]);
    }
    let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
    e.call(SET_OBJECT_HEALTH, &args![list, new_health]);
    e.vcall(reference.addr(), 0x48, &args![0x8000_0000u32]);

    if crossed_end > earlier_end {
        if !e.call(REFERENCE_TEST_A, &args![reference]).bool() {
            if crossed_model != 0 {
                swap_in_replacement_model(e, this, reference, crossed_model);
            } else if crossed_damage_stage > earlier_damage_stage {
                let node = e.call(0x0043_fcd0, &args![reference]).ptr::<()>();
                bgs_destructible_object_form_update_damage_stage_nodes(
                    e,
                    node,
                    crossed_damage_stage as u8,
                );
                e.call(0x0057_a3c0, &args![reference, 0u32]);
            }
        }
        if crossed_rate != 0 && new_health as f64 > zero {
            e.call(
                MAP_SET_AT,
                &args![DESTRUCTIBLE_OBJECTS, reference, crossed_rate],
            );
            remaining_rate = crossed_rate;
        } else {
            e.call(MAP_REMOVE_AT, &args![DESTRUCTIBLE_OBJECTS, reference]);
        }
    }

    // Effects of the crossed stages: debris and explosions.
    let mut crossed = earlier_end;
    while crossed < crossed_end {
        let stage = fn_00476a40(e, this, crossed);
        let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
        e.call(0x005a_c750, &args![reference, list, 0x0020_0000u32]);
        let selector_word = if crossed_damage_stage != 0 {
            crossed_damage_stage
        } else if earlier_damage_stage != 0 {
            earlier_damage_stage
        } else {
            0
        };
        let selector = selector_word as u8;
        let debris = e.get(stage, DestructibleObjectStage::pDebris);
        let count = e.get(stage, DestructibleObjectStage::iDebrisCount);
        if !debris.is_null() && count != 0 {
            let node = e.call(POINTER_GET, &args![holder]).ptr::<()>();
            let spawned = bgs_destructible_object_form_spawn_debris_at_debris_nodes(
                e, this, node, debris, selector, reference, count,
            );
            if spawned == 0 {
                // No debris node: spawn at the reference's own position.
                let position = e.vcall(reference.addr(), 0x1f4, &args![]).u32();
                let copy = e.mem.alloc(12);
                copy_bytes(e, position, copy, 12);
                let scale = e.call(GET_SCALE, &args![reference]).f32();
                let word = e.call(GET_REFERENCE_WORD, &args![reference]).u32();
                e.call(SPAWN_DEBRIS, &args![debris, word, copy, count, scale]);
                e.mem.free(copy);
            }
        }
        let explosion = e.get(stage, DestructibleObjectStage::pExplosion);
        if !explosion.is_null() {
            let node = e.call(POINTER_GET, &args![holder]).ptr::<()>();
            let created = fn_00476cb0(e, this, node, explosion, selector, reference);
            if created == 0 {
                create_explosion_near_bound(e, reference, holder, explosion);
            }
        }
        crossed += 1;
    }

    if new_health as f64 == zero {
        e.call(0x0047_78a0, &args![this]);
    }

    // The combat threat map is told about the object (its `float` result,
    // -1.0 for "none").
    let threat_key = e.mem.alloc(4);
    let minus_one: f32 = e.global(MINUS_ONE);
    let mut result = minus_one;
    if remaining_rate != 0 {
        let setting_a = setting_value(e, 0x011c_e720);
        let setting_b = setting_value(e, 0x011c_f1b0);
        result = e
            .call(
                0x0047_7a50,
                &args![
                    this,
                    new_percentage as u32,
                    setting_b,
                    setting_a,
                    threat_key
                ],
            )
            .f32();
        let limit = setting_value(e, 0x011c_eabc);
        if (limit as f64) < result as f64 {
            result = minus_one;
        }
    }
    if result as f64 == no_health {
        let setting_a = setting_value(e, 0x011c_e720);
        let setting_b = setting_value(e, 0x011c_f1b0);
        let candidate = e
            .call(
                0x0047_7970,
                &args![
                    this,
                    new_percentage as u32,
                    setting_b,
                    setting_a,
                    threat_key
                ],
            )
            .f32();
        if candidate as f64 != no_health {
            let difference = new_health as f64 - candidate as f64;
            let margin = setting_value(e, 0x011c_e1b8);
            if margin as f64 > difference {
                result = setting_value(e, 0x011c_ef20);
            }
        }
    }
    let threat_source: u32 = e.global(0x011f_1958);
    let threat_map = e.call(0x0082_5c00, &args![threat_source]).u32();
    if result as f64 != no_health {
        let key = e.mem.u32(threat_key);
        e.call(0x009a_54a0, &args![threat_map, reference, key, result]);
    } else {
        e.call(0x009a_51e0, &args![threat_map, reference]);
    }
    e.mem.free(threat_key);
    e.call(NODE_HOLDER_DESTROY, &args![holder]);
    e.mem.free(holder);
}

/// The `float` a setting pointer (`00403e20`, `ECX` = setting) points at.
fn setting_value(e: &mut Engine, setting: u32) -> f32 {
    let pointer = e.call(SETTING_POINTER, &args![setting]).u32();
    e.mem.f32(pointer)
}

/// The part of the damage routine (`00475b20`) that replaces the
/// reference's model: gathers the 3D's `BSSimpleArray` of nodes
/// (`00477230`), swaps the model in through the extra data and the routines
/// at `011ddf38`/`011dea10`, re-gathers the nodes, and carries the old
/// transform over (rotation `old * transpose(new) * orientation`, position
/// moved by the difference of the translations), then activates the new 3D
/// in its Havok world (`bhkWorld::Activate`, `00c6a270`).
fn swap_in_replacement_model(
    e: &mut Engine,
    this: Ptr<BGSDestructibleObjectForm>,
    reference: Ptr,
    model: u32,
) {
    let data = e.get(this, BGSDestructibleObjectForm::pData);
    let nodes = e.mem.alloc(0x10);
    e.call(ARRAY_CONSTRUCT, &args![nodes]);
    // `NiTransform` of the first gathered node before the swap.
    let old_transform = e.mem.alloc(0x34);
    fn_00476a80(e, Ptr::new(old_transform));
    let root = e.vcall(reference.addr(), 0x1d0, &args![]).ptr::<()>();
    fn_00477230(e, this, root, Ptr::new(nodes));
    if e.call(ARRAY_SIZE, &args![nodes]).u32() != 0 {
        let element = e.call(ARRAY_ELEMENT, &args![nodes, 0u32]).u32();
        let first = e.mem.u32(element);
        let transform = e.call(GET_TRANSFORM, &args![first]).u32();
        copy_bytes(e, transform, old_transform, 0x34);
    }
    let mut had_preloaded_model = false;
    let preloaded = e.call(POINTER_GET, &args![data.addr() + 0x10]).u32();
    if preloaded != 0 {
        e.call(0x0040_b460, &args![data.addr() + 0xc]);
        had_preloaded_model = true;
    }
    let flag_b = e.call(0x0057_b200, &args![reference]).u8();
    let swap_flags: u32 = e.global(0x011d_df38);
    let flag_c = e.call(0x0046_23f0, &args![swap_flags, 0u32]).u8();
    e.vcall(reference.addr(), 0x1cc, &args![0u32, 1u32]);
    if flag_b != 0 {
        e.call(0x0057_b240, &args![reference, 1u32]);
    }
    e.call(0x0046_23f0, &args![swap_flags, flag_c as u32]);
    let form = e.call(0x007a_f430, &args![reference]).u32();
    let list = e.call(GET_EXTRA_LIST, &args![reference]).u32();
    e.call(0x0042_e150, &args![list, model, form]);
    e.vcall(reference.addr(), 0x48, &args![0x8000_0000u32]);
    let swap_object: u32 = e.global(0x011d_ea10);
    let word = e.call(GET_REFERENCE_WORD, &args![reference]).u32();
    e.call(
        0x0045_1ef0,
        &args![swap_object, reference, word, 0u32, 0u32],
    );

    if e.vcall(reference.addr(), 0x1d0, &args![]).u32() != 0 {
        let new_root = e.vcall(reference.addr(), 0x1d0, &args![]).u32();
        let container = if new_root != 0 {
            e.vcall(new_root, 0xc, &args![]).u32()
        } else {
            0
        };
        let collision = if container != 0 {
            e.vcall(container, 0x10, &args![]).u32()
        } else {
            0
        };
        if collision != 0 {
            fn_00476ab0(e, Ptr::new(collision));
        }
        if e.call(ARRAY_SIZE, &args![nodes]).u32() != 0 {
            let new_nodes = e.mem.alloc(0x10);
            e.call(ARRAY_CONSTRUCT, &args![new_nodes]);
            fn_00477230(e, this, Ptr::new(new_root), Ptr::new(new_nodes));
            if e.call(ARRAY_SIZE, &args![new_nodes]).u32() != 0 {
                carry_over_transform(e, reference, old_transform, new_nodes);
            }
            e.call(ARRAY_DESTROY, &args![new_nodes]);
            e.mem.free(new_nodes);
        }
        e.call(0x00c6_a270, &args![new_root, 1u32, 1u32, 0u32]);
    }
    if had_preloaded_model {
        e.call(0x0047_78a0, &args![this]);
    }
    e.call(ARRAY_DESTROY, &args![nodes]);
    e.mem.free(nodes);
    e.mem.free(old_transform);
}

/// Puts the reference's orientation and position back relative to the new
/// 3D: the rotation of the old transform times the transpose of the new
/// node's, times the reference's orientation, is set as the orientation;
/// the position is the old translation minus the new one, plus the
/// reference's position.
fn carry_over_transform(e: &mut Engine, reference: Ptr, old_transform: u32, new_nodes: u32) {
    let new_transform = e.mem.alloc(0x34);
    let element = e.call(ARRAY_ELEMENT, &args![new_nodes, 0u32]).u32();
    let first = e.mem.u32(element);
    let source = e.call(GET_TRANSFORM, &args![first]).u32();
    copy_bytes(e, source, new_transform, 0x34);
    let transposed = e.mem.alloc(0x24);
    ni_matrix3_transpose(e, Ptr::new(new_transform), Ptr::new(transposed));
    let old_rotation = e.mem.alloc(0x24);
    copy_bytes(e, old_transform, old_rotation, 0x24);
    let orientation = e.mem.alloc(0x24);
    e.call(GET_ORIENTATION, &args![reference, orientation]);
    let product = e.mem.alloc(0x24);
    let combined = e.mem.alloc(0x24);
    let first_product = e
        .call(0x0043_f8d0, &args![old_rotation, product, transposed])
        .u32();
    e.call(0x0043_f8d0, &args![first_product, combined, orientation]);
    let mut words = vec![reference.addr()];
    for i in 0..9 {
        words.push(e.mem.u32(combined + 4 * i));
    }
    e.call(0x0056_fa90, &words);
    e.call(UPDATE_3D_POSITION, &args![reference]);

    // The second look at the new node's transform, as the game does.
    let element = e.call(ARRAY_ELEMENT, &args![new_nodes, 0u32]).u32();
    let first = e.mem.u32(element);
    let source = e.call(GET_TRANSFORM, &args![first]).u32();
    copy_bytes(e, source, new_transform, 0x34);
    let difference = e.mem.alloc(12);
    e.call(
        0x0043_9ef0,
        &args![old_transform + 0x24, difference, new_transform + 0x24],
    );
    let position = e.vcall(reference.addr(), 0x1f4, &args![]).u32();
    e.call(0x0063_c8a0, &args![difference, position]);
    e.call(0x0057_5830, &args![reference, difference]);
    e.call(UPDATE_3D_POSITION, &args![reference]);
    for block in [
        new_transform,
        transposed,
        old_rotation,
        orientation,
        product,
        combined,
        difference,
    ] {
        e.mem.free(block);
    }
}

/// Creates the explosion `explosion` near the 3D's bound when no node took
/// it: the reference's orientation, and a position that is the centre of
/// the 3D's world bound plus a random vector (x and y in [-1, 1], z in
/// [-1, 0], scaled by the radius).
fn create_explosion_near_bound(e: &mut Engine, reference: Ptr, holder: u32, explosion: Ptr) {
    let orientation = e.mem.alloc(0x24);
    e.call(GET_ORIENTATION, &args![reference, orientation]);
    let node = e.call(POINTER_GET, &args![holder]).u32();
    let bound_source = e.call(GET_WORLD_BOUND, &args![node]).u32();
    let bound = e.mem.alloc(16);
    copy_bytes(e, bound_source, bound, 16);
    let vector = e.mem.alloc(12);
    e.call(0x0068_15c0, &args![vector]);
    let minus_one: f32 = e.global(MINUS_ONE);
    let x = random_float(e, minus_one, 1.0);
    e.mem.set_f32(vector, x);
    let y = random_float(e, minus_one, 1.0);
    e.mem.set_f32(vector + 4, y);
    let z = random_float(e, minus_one, 0.0);
    e.mem.set_f32(vector + 8, z);
    let radius = e.call(0x0084_d030, &args![bound]).f32();
    e.call(0x0043_9180, &args![vector, radius]);
    let position = e.mem.alloc(12);
    let bound_centre = e.call(0x0068_15c0, &args![bound]).u32();
    e.call(0x0043_9e90, &args![bound_centre, position, vector]);

    let mut words = vec![explosion.addr()];
    let word = e.call(GET_REFERENCE_WORD, &args![reference]).u32();
    let is_projectile_like = e.vcall(reference.addr(), 0x224, &args![]).bool();
    if is_projectile_like {
        let extra = e.call(0x0087_4480, &args![reference]).u32();
        words.extend_from_slice(&[extra, 0, word]);
    } else {
        words.extend_from_slice(&[reference.addr(), reference.addr(), word]);
    }
    for i in 0..3 {
        words.push(e.mem.u32(position + 4 * i));
    }
    for i in 0..9 {
        words.push(e.mem.u32(orientation + 4 * i));
    }
    let created = e.call(CREATE_EXPLOSION, &words).u32();
    if is_projectile_like && created != 0 && fn_00476c90(e, reference) != 0 {
        let value = fn_00476c90(e, reference);
        fn_00476c70(e, Ptr::new(created), value);
    }
    for block in [orientation, bound, vector, position] {
        e.mem.free(block);
    }
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x004751d0,
            fn_004751d0(Ptr<DestructibleObjectData>) -> Ptr<DestructibleObjectData>
        ),
        entry!(
            0x004753a0,
            fn_004753a0(Ptr<DestructibleObjectData>, u32) -> Ptr<DestructibleObjectData>
        ),
        entry!(
            0x004753d0,
            bgs_destructible_object_form_is_destructible(Ptr) -> bool
        ),
        entry!(
            0x00475400,
            bgs_destructible_object_form_get_destruction_form(Ptr) -> Ptr
        ),
        entry!(
            0x00475920,
            bgs_destructible_object_form_get_model_swap_index(Ptr, u32) -> u32
        ),
        entry!(0x004759a0, fn_004759a0(Ptr, i32) -> u32),
        entry!(
            0x00475a00,
            bgs_destructible_object_form_has_valid_stages(Ptr<BGSDestructibleObjectForm>) -> bool
        ),
        entry!(
            0x00475a90,
            fn_00475a90(Ptr<DestructibleObjectStage>) -> bool
        ),
        entry!(
            0x00475ab0,
            fn_00475ab0(Ptr<BGSDestructibleObjectForm>, Ptr) -> u32
        ),
        entry!(
            0x00475b20,
            fn_00475b20(Ptr<BGSDestructibleObjectForm>, Ptr, f32, u32)
        ),
        entry!(0x004768c0, ni_matrix3_transpose(Ptr, Ptr) -> Ptr),
        entry!(0x00476930, fn_00476930(Ptr, u32, Ptr)),
        entry!(0x00476980, fn_00476980(Ptr, Ptr, Ptr, Ptr) -> Ptr),
        entry!(0x004769c0, fn_004769c0(Ptr, u32, Ptr)),
        entry!(
            0x00476a00,
            fn_00476a00(Ptr<DestructibleObjectStage>) -> bool
        ),
        entry!(
            0x00476a20,
            fn_00476a20(Ptr<DestructibleObjectStage>) -> bool
        ),
        entry!(
            0x00476a40,
            fn_00476a40(Ptr<BGSDestructibleObjectForm>, u32) -> Ptr<DestructibleObjectStage>
        ),
        entry!(0x00476a80, fn_00476a80(Ptr) -> Ptr),
        entry!(0x00476ab0, fn_00476ab0(Ptr)),
        entry!(0x00476ae0, fn_00476ae0(Ptr, u8)),
        entry!(0x00476b00, fn_00476b00(Ptr, u8)),
        entry!(0x00476b20, fn_00476b20(f32) -> f32),
        entry!(0x00476b70, random_float(f32, f32) -> f32),
        entry!(0x00476b90, fn_00476b90(Ptr, f32, f32) -> f32),
        entry!(0x00476be0, fn_00476be0(Ptr) -> u32),
        entry!(0x00476c00, fn_00476c00() -> Ptr),
        entry!(0x00476c70, fn_00476c70(Ptr, u32)),
        entry!(0x00476c90, fn_00476c90(Ptr) -> u32),
        entry!(
            0x00476cb0,
            fn_00476cb0(Ptr<BGSDestructibleObjectForm>, Ptr, Ptr, u8, Ptr) -> i32
        ),
        entry!(
            0x00476dd0,
            bgs_destructible_object_form_spawn_debris_at_debris_nodes(
                Ptr<BGSDestructibleObjectForm>,
                Ptr,
                Ptr,
                u8,
                Ptr,
                u32,
            ) -> i32
        ),
        entry!(
            0x00476ec0,
            bgs_destructible_object_form_update_damage_stage_nodes(Ptr, u8)
        ),
        entry!(
            0x00476f50,
            bgs_destructible_object_form_update_current_damage_stage(
                Ptr<BGSDestructibleObjectForm>,
                Ptr,
                bool,
            ) -> bool
        ),
        entry!(0x00477090, fn_00477090(Ptr<BGSDestructibleObjectForm>, Ptr)),
        entry!(
            0x00477230,
            fn_00477230(Ptr<BGSDestructibleObjectForm>, Ptr, Ptr)
        ),
        entry!(0x004772e0, fn_004772e0() -> u32),
        entry!(0x004772f0, fn_004772f0()),
        entry!(0x00477410, fn_00477410(Ptr)),
        entry!(0x00477430, fn_00477430(Ptr) -> u32),
        entry!(
            0x00477560,
            bgs_destructible_object_form_hide_small_debris(Ptr) -> bool
        ),
        entry!(
            0x00477640,
            bgs_destructible_object_form_queue_files(Ptr<BGSDestructibleObjectForm>, u32, u32, u32)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn noop(e: &mut Engine, addresses: &[u32]) {
        for &address in addresses {
            e.register(address, |_, _| Ret::default());
        }
    }

    /// An engine with the exe's constants the code reads mapped and set.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_2000u32,
            0x0101_7000,
            0x0101_a000,
            0x011c_4000,
            0x011c_6000,
            0x011c_3000,
            0x011c_e000,
            0x011c_f000,
            0x011f_6000,
            0x011f_1000,
            0x011d_d000,
            0x011d_e000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(ZERO, 0.0f64);
        e.set_global(ONE, 1.0f64);
        e.set_global(HUNDRED, 100.0f64);
        e.set_global(NO_HEALTH, -1.0f64);
        e.set_global(TWO_POW_32, 4294967296.0f64);
        e.set_global(MINUS_ONE, -1.0f32);
        // `00404090`: truncate toward zero.
        e.register(TRUNCATE_FLOAT, |_, a| {
            f32::from_bits(a[0]).trunc().into_ret()
        });
        e
    }

    /// An object of `size` bytes whose vtable has the given slots (byte
    /// offset, function address).
    fn object_with_vtable(e: &mut Engine, size: u32, slots: &[(u32, u32)]) -> u32 {
        let object = e.mem.alloc(size);
        let table = e.mem.alloc(0x400);
        e.mem.set_u32(object, table);
        for &(offset, function) in slots {
            e.mem.set_u32(table + offset, function);
        }
        object
    }

    #[derive(Clone, Copy, Default)]
    struct StageSpec {
        damage_stage: u8,
        percentage: u8,
        flags: u8,
        rate: u32,
        explosion: u32,
        debris: u32,
        count: u32,
        model: u32,
    }

    /// A component with data of total health `health` and the stages.
    fn make_component(
        e: &mut Engine,
        health: u32,
        stages: &[StageSpec],
    ) -> (Ptr<BGSDestructibleObjectForm>, Ptr<DestructibleObjectData>) {
        let component: Ptr<BGSDestructibleObjectForm> = e.new_object();
        let data: Ptr<DestructibleObjectData> = e.new_object();
        e.set(component, BGSDestructibleObjectForm::pData, data);
        e.set(data, DestructibleObjectData::iHealth, health);
        e.set(data, DestructibleObjectData::cNumStages, stages.len() as u8);
        let array = e.mem.alloc(4 * stages.len() as u32 + 4);
        e.set(data, DestructibleObjectData::pStagesArray, Ptr::new(array));
        for (i, spec) in stages.iter().enumerate() {
            let stage: Ptr<DestructibleObjectStage> = e.new_object();
            e.set(
                stage,
                DestructibleObjectStage::cModelDamageStage,
                spec.damage_stage,
            );
            e.set(
                stage,
                DestructibleObjectStage::cHealthPercentage,
                spec.percentage,
            );
            e.set(stage, DestructibleObjectStage::cFlags, spec.flags);
            e.set(
                stage,
                DestructibleObjectStage::iSelfDamagePerSecond,
                spec.rate,
            );
            e.set(
                stage,
                DestructibleObjectStage::pExplosion,
                Ptr::new(spec.explosion),
            );
            e.set(
                stage,
                DestructibleObjectStage::pDebris,
                Ptr::new(spec.debris),
            );
            e.set(stage, DestructibleObjectStage::iDebrisCount, spec.count);
            e.set(
                stage,
                DestructibleObjectStage::pReplacementModel,
                Ptr::new(spec.model),
            );
            e.mem.set_u32(array + 4 * i as u32, stage.addr());
        }
        (component, data)
    }

    fn stage_pointer(
        e: &Engine,
        data: Ptr<DestructibleObjectData>,
        i: u32,
    ) -> Ptr<DestructibleObjectStage> {
        stage_at(e, data, i)
    }

    #[test]
    fn data_constructor_zeroes_the_fields_and_constructs_the_preloaded_pointer() {
        let mut e = engine();
        e.register(0x0052_8cb0, |_, _| Ret::default());
        let data: Ptr<DestructibleObjectData> = e.new_object();
        e.mem.write(data.addr(), &[0xff; 0x14]);
        e.call_log = Some(vec![]);
        let back = e
            .call(0x0047_51d0, &args![data])
            .ptr::<DestructibleObjectData>();
        assert_eq!(back, data);
        assert_eq!(e.get(data, DestructibleObjectData::iHealth), 0);
        assert_eq!(e.get(data, DestructibleObjectData::cNumStages), 0);
        assert_eq!(e.get(data, DestructibleObjectData::cFlags), 0);
        assert!(e.get(data, DestructibleObjectData::pStagesArray).is_null());
        assert_eq!(
            e.get(data, DestructibleObjectData::iReplacementModelRefCount),
            0
        );
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x0052_8cb0, vec![data.addr() + 0x10, 0])));
    }

    #[test]
    fn data_destructor_frees_only_when_asked() {
        let mut e = engine();
        noop(&mut e, &[0x0047_5220, 0x0040_1030]);
        let data: Ptr<DestructibleObjectData> = e.new_object();
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0047_53a0, &args![data, 0u32]).u32(), data.addr());
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x0047_5220, vec![data.addr()])));
        assert!(!log.iter().any(|(address, _)| *address == 0x0040_1030));
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0047_53a0, &args![data, 1u32]).u32(), data.addr());
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x0040_1030, vec![data.addr()])));
    }

    /// `00401170` reads the form type byte at +4.
    fn form_type_double(e: &mut Engine) {
        e.register(GET_FORM_TYPE, |e, a| ret(e.mem.u8(a[0] + 4) as u32));
        e.register(GET_COMPONENT_DATA, |e, a| ret(e.mem.u32(a[0] + 4)));
    }

    fn form_of_type(e: &mut Engine, form_type: u8) -> u32 {
        let form = e.mem.alloc(0x200);
        e.mem.set_u8(form + 4, form_type);
        form
    }

    #[test]
    fn destruction_form_offset_depends_on_the_form_type() {
        let mut e = engine();
        form_type_double(&mut e);
        // (form type, offset of the component from the form)
        let table: [(u8, i32); 22] = [
            (0x15, 0x68),
            (0x16, 0x68),
            (0x17, 0x68),
            (0x18, 0x14c),
            (0x19, 0x9c),
            (0x1a, 0x144),
            (0x1b, 0x7c),
            (0x1c, 0x68),
            (0x1d, 0x94),
            (0x1e, 0x94),
            (0x1f, 0x84),
            (0x22, -0x14 + 0xc),
            (0x25, 0x54),
            (0x26, -0xc + 0x74),
            (0x27, 0x68),
            (0x28, 0xb4),
            (0x29, 0x88),
            (0x2a, 0x104),
            (0x2b, 0x104),
            (0x2e, 0x84),
            (0x2f, 0xa4),
            (0x33, 0x58),
        ];
        for (form_type, offset) in table {
            let form = form_of_type(&mut e, form_type);
            let got = e.call(0x0047_5400, &args![form]).u32();
            assert_eq!(got, form.wrapping_add(offset as u32), "type {form_type:#x}");
        }
        for form_type in [0x32u8, 0x67] {
            let form = form_of_type(&mut e, form_type);
            assert_eq!(e.call(0x0047_5400, &args![form]).u32(), form + 0x84);
        }
        // Types outside the table and a null form have no component.
        for form_type in [0x14u8, 0x20, 0x30, 0x68, 0xff] {
            let form = form_of_type(&mut e, form_type);
            assert_eq!(e.call(0x0047_5400, &args![form]).u32(), 0);
        }
        assert_eq!(e.call(0x0047_5400, &args![0u32]).u32(), 0);
    }

    #[test]
    fn is_destructible_needs_a_component_with_a_valid_stage() {
        let mut e = engine();
        form_type_double(&mut e);
        let form = form_of_type(&mut e, 0x15);
        // No data at all.
        assert!(!e.call(0x0047_53d0, &args![form]).bool());
        let (component, _) = make_component(
            &mut e,
            100,
            &[StageSpec {
                model: 0x1000,
                ..StageSpec::default()
            }],
        );
        let data = e.get(component, BGSDestructibleObjectForm::pData);
        e.mem.set_u32(form + 0x68 + 4, data.addr());
        assert!(e.call(0x0047_53d0, &args![form]).bool());
        // A type without a component, and a null form.
        let other = form_of_type(&mut e, 0x14);
        assert!(!e.call(0x0047_53d0, &args![other]).bool());
        assert!(!e.call(0x0047_53d0, &args![0u32]).bool());
    }

    #[test]
    fn model_swap_index_finds_the_stage_with_the_model() {
        let mut e = engine();
        form_type_double(&mut e);
        let form = form_of_type(&mut e, 0x15);
        let (component, data) = make_component(
            &mut e,
            100,
            &[
                StageSpec {
                    model: 0x1000,
                    ..StageSpec::default()
                },
                StageSpec {
                    model: 0x2000,
                    ..StageSpec::default()
                },
            ],
        );
        let _ = component;
        e.mem.set_u32(form + 0x68 + 4, data.addr());
        assert_eq!(e.call(0x0047_5920, &args![form, 0x2000u32]).u32(), 1);
        assert_eq!(e.call(0x0047_5920, &args![form, 0x1000u32]).u32(), 0);
        assert_eq!(e.call(0x0047_5920, &args![form, 0x3000u32]).u32(), u32::MAX);
        // No component for the type, no data for the component.
        let other = form_of_type(&mut e, 0x14);
        assert_eq!(
            e.call(0x0047_5920, &args![other, 0x1000u32]).u32(),
            u32::MAX
        );
        let empty = form_of_type(&mut e, 0x16);
        assert_eq!(
            e.call(0x0047_5920, &args![empty, 0x1000u32]).u32(),
            u32::MAX
        );
    }

    #[test]
    fn model_at_stage_checks_the_index() {
        let mut e = engine();
        form_type_double(&mut e);
        let form = form_of_type(&mut e, 0x15);
        let (_, data) = make_component(
            &mut e,
            100,
            &[
                StageSpec {
                    model: 0x1000,
                    ..StageSpec::default()
                },
                StageSpec {
                    model: 0x2000,
                    ..StageSpec::default()
                },
            ],
        );
        e.mem.set_u32(form + 0x68 + 4, data.addr());
        assert_eq!(e.call(0x0047_59a0, &args![form, 1i32]).u32(), 0x2000);
        assert_eq!(e.call(0x0047_59a0, &args![form, 0i32]).u32(), 0x1000);
        // Past the last stage, and -1.
        assert_eq!(e.call(0x0047_59a0, &args![form, 2i32]).u32(), 0);
        assert_eq!(e.call(0x0047_59a0, &args![form, -1i32]).u32(), 0);
        // No component, no data.
        let other = form_of_type(&mut e, 0x14);
        assert_eq!(e.call(0x0047_59a0, &args![other, 0i32]).u32(), 0);
        let empty = form_of_type(&mut e, 0x16);
        assert_eq!(e.call(0x0047_59a0, &args![empty, 0i32]).u32(), 0);
    }

    #[test]
    fn has_valid_stages_looks_for_something_to_do() {
        let mut e = engine();
        let empty: Ptr<BGSDestructibleObjectForm> = e.new_object();
        assert!(!e.call(0x0047_5a00, &args![empty]).bool());
        let nothing = StageSpec::default();
        let (component, _) = make_component(&mut e, 100, &[nothing, nothing]);
        assert!(!e.call(0x0047_5a00, &args![component]).bool());
        for spec in [
            StageSpec {
                model: 1,
                ..nothing
            },
            StageSpec {
                explosion: 1,
                ..nothing
            },
            StageSpec {
                debris: 1,
                ..nothing
            },
            StageSpec {
                flags: STAGE_FLAG_DISABLE_OBJECT,
                ..nothing
            },
        ] {
            let (component, _) = make_component(&mut e, 100, &[nothing, spec]);
            assert!(e.call(0x0047_5a00, &args![component]).bool());
        }
        // The other two flags do not make a stage valid, and a null entry
        // is skipped.
        let (component, data) = make_component(
            &mut e,
            100,
            &[
                StageSpec {
                    flags: STAGE_FLAG_CAP_DAMAGE | STAGE_FLAG_DESTROY_OBJECT,
                    ..nothing
                },
                nothing,
            ],
        );
        let first = stage_pointer(&e, data, 0);
        let array = e.get(data, DestructibleObjectData::pStagesArray).addr();
        e.mem.set_u32(array + 4, 0);
        assert!(!e.call(0x0047_5a00, &args![component]).bool());
        let _ = first;
    }

    #[test]
    fn stage_flag_tests() {
        let mut e = engine();
        for (flag, address) in [
            (STAGE_FLAG_DISABLE_OBJECT, 0x0047_5a90u32),
            (STAGE_FLAG_CAP_DAMAGE, 0x0047_6a00),
            (STAGE_FLAG_DESTROY_OBJECT, 0x0047_6a20),
        ] {
            let with: Ptr<DestructibleObjectStage> = e.new_object();
            e.set(with, DestructibleObjectStage::cFlags, flag | 0x80);
            assert!(e.call(address, &args![with]).bool());
            let without: Ptr<DestructibleObjectStage> = e.new_object();
            e.set(without, DestructibleObjectStage::cFlags, 7 & !flag);
            assert!(!e.call(address, &args![without]).bool());
        }
    }

    #[test]
    fn total_health_comes_from_the_actor_value_or_the_data() {
        let mut e = engine();
        let (component, _) = make_component(&mut e, 123, &[]);
        let actor_slot = 0x0f00_0001u32;
        let value_slot = 0x0f00_0002u32;
        e.register(actor_slot, |e, a| ret(e.mem.u32(a[0] + 0x150)));
        e.register(value_slot, |_, a| {
            assert_eq!(a[1], 0x10);
            250.7f32.into_ret()
        });
        let reference = object_with_vtable(&mut e, 0x200, &[(0x100, actor_slot)]);
        let interface_table = e.mem.alloc(0x40);
        e.mem.set_u32(interface_table + 4, value_slot);
        e.mem.set_u32(reference + 0xa4, interface_table);
        // Not an actor: the data's health.
        assert_eq!(e.call(0x0047_5ab0, &args![component, reference]).u32(), 123);
        // An actor: the interface's value, truncated.
        e.mem.set_u32(reference + 0x150, 1);
        assert_eq!(e.call(0x0047_5ab0, &args![component, reference]).u32(), 250);
    }

    #[test]
    fn transpose_swaps_rows_and_columns() {
        let mut e = engine();
        e.register(0x0040_1050, |_, _| Ret::default());
        let matrix = e.mem.alloc(0x24);
        for i in 0..9u32 {
            e.mem.set_f32(matrix + 4 * i, (i + 1) as f32);
        }
        let result = e.mem.alloc(0x24);
        e.call_log = Some(vec![]);
        let back = e.call(0x0047_68c0, &args![matrix, result]).u32();
        assert_eq!(back, result);
        let log = e.call_log.take().unwrap();
        let constructor = log
            .iter()
            .find(|(address, _)| *address == 0x0040_1050)
            .unwrap();
        assert_eq!(&constructor.1[1..], &[12, 3, 0x0068_15c0]);
        let got: Vec<f32> = (0..9).map(|i| e.mem.f32(result + 4 * i)).collect();
        assert_eq!(got, [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]);
    }

    #[test]
    fn row_copy_and_column_store() {
        let mut e = engine();
        let matrix = e.mem.alloc(0x24);
        for i in 0..9u32 {
            e.mem.set_f32(matrix + 4 * i, (i + 1) as f32);
        }
        let out = e.mem.alloc(12);
        e.call(0x0047_6930, &args![matrix, 2u32, out]);
        assert_eq!(
            [e.mem.f32(out), e.mem.f32(out + 4), e.mem.f32(out + 8)],
            [7.0, 8.0, 9.0]
        );
        // Column 1 of a fresh matrix.
        let target = e.mem.alloc(0x24);
        e.call(0x0047_69c0, &args![target, 1u32, out]);
        assert_eq!(
            [
                e.mem.f32(target + 4),
                e.mem.f32(target + 16),
                e.mem.f32(target + 28)
            ],
            [7.0, 8.0, 9.0]
        );
        // Three columns.
        let a = e.mem.alloc(12);
        let b = e.mem.alloc(12);
        let c = e.mem.alloc(12);
        for i in 0..3u32 {
            e.mem.set_f32(a + 4 * i, 1.0 + i as f32);
            e.mem.set_f32(b + 4 * i, 4.0 + i as f32);
            e.mem.set_f32(c + 4 * i, 7.0 + i as f32);
        }
        let all = e.mem.alloc(0x24);
        assert_eq!(e.call(0x0047_6980, &args![all, a, b, c]).u32(), all);
        let got: Vec<f32> = (0..9).map(|i| e.mem.f32(all + 4 * i)).collect();
        assert_eq!(got, [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]);
    }

    #[test]
    fn stage_lookup_is_bounded_by_the_count() {
        let mut e = engine();
        let (component, data) = make_component(&mut e, 100, &[StageSpec::default(); 2]);
        let expected = stage_pointer(&e, data, 1);
        assert_eq!(
            e.call(0x0047_6a40, &args![component, 1u32]).u32(),
            expected.addr()
        );
        assert_eq!(e.call(0x0047_6a40, &args![component, 2u32]).u32(), 0);
        let empty: Ptr<BGSDestructibleObjectForm> = e.new_object();
        assert_eq!(e.call(0x0047_6a40, &args![empty, 0u32]).u32(), 0);
    }

    #[test]
    fn transform_constructors_run_on_both_members() {
        let mut e = engine();
        e.register(0x0068_15c0, |_, a| ret(a[0]));
        let transform = e.mem.alloc(0x34);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0047_6a80, &args![transform]).u32(), transform);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x0068_15c0, vec![transform])));
        assert!(log.contains(&(0x0068_15c0, vec![transform + 0x24])));
    }

    #[test]
    fn flag_setters_forward_to_the_model_routine() {
        let mut e = engine();
        e.register(0x0043_b370, |_, _| Ret::default());
        let object = e.mem.alloc(0x100);
        e.call_log = Some(vec![]);
        e.call(0x0047_6ab0, &args![object]);
        assert_eq!(e.mem.f32(object + 0xb4), 1.0);
        assert_eq!(e.mem.f32(object + 0xb8), 1.0);
        e.call(0x0047_6ae0, &args![object, 0u8]);
        e.call(0x0047_6b00, &args![object, 1u8]);
        let log = e.call_log.take().unwrap();
        let calls: Vec<_> = log
            .iter()
            .filter(|(address, _)| *address == 0x0043_b370)
            .map(|(_, a)| a.clone())
            .collect();
        assert_eq!(
            calls,
            vec![
                vec![object, 1, 0x4000],
                vec![object, 0, 0x4000],
                vec![object, 1, 0x4000]
            ]
        );
    }

    #[test]
    fn round_up_adds_one_below_the_value() {
        let mut e = engine();
        for (value, expected) in [
            (2.5f32, 3.0f32),
            (3.0, 3.0),
            (-2.5, -2.0),
            (0.0001, 1.0),
            (-0.5, 0.0),
            (0.0, 0.0),
        ] {
            let got = e.call(0x0047_6b20, &args![value]).f32();
            assert_eq!(got, expected, "ceil of {value}");
        }
    }

    #[test]
    fn random_generator_is_constructed_once() {
        let mut e = engine();
        e.register_double(0x00aa_5140, {
            let mut count = 0u32;
            move |_, a| {
                assert_eq!(a[0], RANDOM_GENERATOR);
                count += 1;
                ret(count)
            }
        });
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0047_6c00, &args![]).u32(), RANDOM_GENERATOR);
        assert_eq!(e.call(0x0047_6c00, &args![]).u32(), RANDOM_GENERATOR);
        let log = e.call_log.take().unwrap();
        let constructions = log
            .iter()
            .filter(|(address, _)| *address == 0x00aa_5140)
            .count();
        assert_eq!(constructions, 1);
        assert_eq!(e.global::<u32>(RANDOM_GENERATOR_INITIALIZED) & 1, 1);
    }

    #[test]
    fn random_word_uses_the_full_range() {
        let mut e = engine();
        e.register(0x00aa_5230, |_, a| {
            assert_eq!(a, &[0x1234, u32::MAX]);
            ret(0xc000_0000)
        });
        assert_eq!(e.call(0x0047_6be0, &args![0x1234u32]).u32(), 0xc000_0000);
    }

    #[test]
    fn random_float_scales_the_word_into_the_range() {
        let mut e = engine();
        e.register(0x00aa_5140, |_, _| Ret::default());
        e.register(0x00aa_5230, |_, _| ret(0xc000_0000));
        // The method form on an explicit generator.
        let value = e
            .call(0x0047_6b90, &args![RANDOM_GENERATOR, 10.0f32, 20.0f32])
            .f32();
        assert_eq!(value, 17.5);
        // The cdecl form draws from the static generator.
        let value = e.call(0x0047_6b70, &args![-1.0f32, 1.0f32]).f32();
        assert_eq!(value, 0.5);
        assert_eq!(e.global::<u32>(RANDOM_GENERATOR_INITIALIZED) & 1, 1);
    }

    #[test]
    fn explosion_word_accessors() {
        let mut e = engine();
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(object + 0xf8, 0xabcd);
        assert_eq!(e.call(0x0047_6c90, &args![object]).u32(), 0xabcd);
        e.call(0x0047_6c70, &args![object, 0x77u32]);
        assert_eq!(e.mem.u32(object + 0xe0), 0x77);
    }

    // ---- node trees --------------------------------------------------

    /// Virtual slot 0xc of a node: its container (+8).
    const NODE_CONTAINER: u32 = 0x0f00_0010;

    /// Doubles for the node-walking helpers. A node keeps a bit mask at
    /// +0x20 of the types it is a checked cast to (1: `01202e7c`, 2:
    /// `01202e84`, 4: `01202e8c`), a flag at +0x24 for `SetCurrent`, its
    /// transform at +0x30 and its translation at +0x80. A container keeps
    /// the child count at +0 and the children from +4.
    fn node_doubles(e: &mut Engine) {
        e.register(NODE_CONTAINER, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(GET_CHILD_COUNT, |e, a| ret(e.mem.u32(a[0])));
        e.register(GET_CHILD_AT, |e, a| ret(e.mem.u32(a[0] + 4 + 4 * a[1])));
        e.register(CHECKED_CAST, |e, a| {
            let bit = match a[0] {
                0x0120_2e7c => 1,
                0x0120_2e84 => 2,
                _ => 4,
            };
            ret((e.mem.u32(a[1] + 0x20) & bit != 0) as u32)
        });
        e.register(RANGE_NODE_SET_CURRENT, |e, a| ret(e.mem.u32(a[0] + 0x24)));
        e.register(GET_TRANSFORM, |_, a| ret(a[0] + 0x30));
        e.register(GET_TRANSLATION, |_, a| ret(a[0] + 0x80));
        e.register(GET_REFERENCE_WORD, |_, _| ret(0x1234));
        e.register(GET_SCALE, |_, _| 1.5f32.into_ret());
    }

    fn make_node(e: &mut Engine, cast_mask: u32, accepts: bool, children: &[u32]) -> u32 {
        let node = object_with_vtable(e, 0x100, &[(0xc, NODE_CONTAINER)]);
        e.mem.set_u32(node + 0x20, cast_mask);
        e.mem.set_u32(node + 0x24, accepts as u32);
        for i in 0..3u32 {
            e.mem.set_f32(node + 0x80 + 4 * i, 1.0 + i as f32);
        }
        for i in 0..13u32 {
            e.mem.set_u32(node + 0x30 + 4 * i, 100 + i);
        }
        if !children.is_empty() {
            let container = e.mem.alloc(0x80);
            e.mem.set_u32(container, children.len() as u32);
            for (i, child) in children.iter().enumerate() {
                e.mem.set_u32(container + 4 + 4 * i as u32, *child);
            }
            e.mem.set_u32(node + 8, container);
        }
        node
    }

    #[test]
    fn explosions_are_created_at_the_accepting_nodes() {
        let mut e = engine();
        node_doubles(&mut e);
        noop(&mut e, &[CREATE_EXPLOSION]);
        let hit = make_node(&mut e, 1, true, &[]);
        let rejected = make_node(&mut e, 1, false, &[]);
        let other_type = make_node(&mut e, 2, true, &[]);
        let root = make_node(&mut e, 1, true, &[rejected, hit, other_type]);
        let this: Ptr<BGSDestructibleObjectForm> = e.new_object();
        let reference = e.mem.alloc(0x100);
        e.call_log = Some(vec![]);
        let created = e
            .call(0x0047_6cb0, &args![this, root, 0x7000u32, 3u32, reference])
            .i32();
        assert_eq!(created, 2);
        let log = e.call_log.take().unwrap();
        let calls: Vec<_> = log.iter().filter(|(a, _)| *a == CREATE_EXPLOSION).collect();
        assert_eq!(calls.len(), 2);
        // The explosion, the reference twice, the reference's word, the
        // node's translation (3 words) and transform (9 words).
        let args_of_root = &calls[0].1;
        assert_eq!(args_of_root.len(), 16);
        assert_eq!(&args_of_root[..4], &[0x7000, reference, reference, 0x1234]);
        assert_eq!(
            &args_of_root[4..7],
            &[1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits()]
        );
        assert_eq!(
            &args_of_root[7..],
            &[100, 101, 102, 103, 104, 105, 106, 107, 108]
        );
        // The selector byte reached `SetCurrent` for every cast node.
        let selectors: Vec<_> = log
            .iter()
            .filter(|(a, _)| *a == RANGE_NODE_SET_CURRENT)
            .map(|(_, a)| (a[0], a[1]))
            .collect();
        assert_eq!(selectors, vec![(root, 3), (rejected, 3), (hit, 3)]);
        // No node, no explosions.
        assert_eq!(
            e.call(0x0047_6cb0, &args![this, 0u32, 0x7000u32, 3u32, reference])
                .i32(),
            0
        );
    }

    #[test]
    fn debris_is_spawned_at_the_debris_nodes() {
        let mut e = engine();
        node_doubles(&mut e);
        noop(&mut e, &[SPAWN_DEBRIS]);
        let hit = make_node(&mut e, 2, true, &[]);
        let rejected = make_node(&mut e, 2, false, &[]);
        let root = make_node(&mut e, 0, true, &[rejected, hit]);
        let this: Ptr<BGSDestructibleObjectForm> = e.new_object();
        let reference = e.mem.alloc(0x100);
        e.call_log = Some(vec![]);
        let spawned = e
            .call(
                0x0047_6dd0,
                &args![this, root, 0x7100u32, 5u32, reference, 4u32],
            )
            .i32();
        assert_eq!(spawned, 1);
        let log = e.call_log.take().unwrap();
        let calls: Vec<_> = log.iter().filter(|(a, _)| *a == SPAWN_DEBRIS).collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].1,
            vec![0x7100, 0x1234, hit + 0x80, 4, 1.5f32.to_bits()]
        );
        assert_eq!(
            e.call(
                0x0047_6dd0,
                &args![this, 0u32, 0x7100u32, 5u32, reference, 4u32]
            )
            .i32(),
            0
        );
    }

    #[test]
    fn damage_stage_nodes_get_the_stage() {
        let mut e = engine();
        node_doubles(&mut e);
        let leaf = make_node(&mut e, 4, true, &[]);
        let skipped = make_node(&mut e, 1, true, &[]);
        let root = make_node(&mut e, 4, true, &[skipped, leaf]);
        e.call_log = Some(vec![]);
        e.call(0x0047_6ec0, &args![root, 3u8]);
        e.call(0x0047_6ec0, &args![0u32, 3u8]);
        let log = e.call_log.take().unwrap();
        let set: Vec<_> = log
            .iter()
            .filter(|(a, _)| *a == RANGE_NODE_SET_CURRENT)
            .map(|(_, a)| a.clone())
            .collect();
        assert_eq!(set, vec![vec![root, 3], vec![leaf, 3]]);
    }

    // ---- the reference the damage routines work on ------------------

    const REFERENCE_GET_3D: u32 = 0x0f00_0001;
    const REFERENCE_IS_ACTOR: u32 = 0x0f00_0002;
    const REFERENCE_SLOT_224: u32 = 0x0f00_0003;
    const REFERENCE_POSITION: u32 = 0x0f00_0004;
    const REFERENCE_NOOP: u32 = 0x0f00_0005;
    const REFERENCE_GET_VALUE: u32 = 0x0f00_0006;
    const REFERENCE_SLOT_338: u32 = 0x0f00_0007;
    const REFERENCE_SLOT_144: u32 = 0x0f00_0008;

    /// The doubles of the reference's virtual slots and of the extra data:
    /// +0x150 the 3D, +0x154 the actor flag, +0x158 the answer of slot
    /// 0x224, +0x160 the position, +0x180 the extra data list holding the
    /// health as a `float`.
    fn reference_doubles(e: &mut Engine) {
        e.register(REFERENCE_GET_3D, |e, a| ret(e.mem.u32(a[0] + 0x150)));
        e.register(REFERENCE_IS_ACTOR, |e, a| ret(e.mem.u32(a[0] + 0x154)));
        e.register(REFERENCE_SLOT_224, |e, a| ret(e.mem.u32(a[0] + 0x158)));
        e.register(REFERENCE_POSITION, |_, a| ret(a[0] + 0x160));
        e.register(REFERENCE_NOOP, |_, _| Ret::default());
        e.register(REFERENCE_GET_VALUE, |e, a| ret(e.mem.u32(a[0] + 0x15c)));
        e.register(GET_EXTRA_LIST, |_, a| ret(a[0] + 0x180));
        e.register(GET_OBJECT_HEALTH, |e, a| e.mem.f32(a[0]).into_ret());
        e.register(SET_OBJECT_HEALTH, |e, a| {
            e.mem.set_f32(a[0], f32::from_bits(a[1]));
            Ret::default()
        });
    }

    fn make_reference(e: &mut Engine, three_d: u32, health: f32) -> u32 {
        let reference = object_with_vtable(
            e,
            0x200,
            &[
                (0x1d0, REFERENCE_GET_3D),
                (0x100, REFERENCE_IS_ACTOR),
                (0x224, REFERENCE_SLOT_224),
                (0x1f4, REFERENCE_POSITION),
                (0x48, REFERENCE_NOOP),
                (0x1cc, REFERENCE_NOOP),
                (0x338, REFERENCE_SLOT_338),
                (0x144, REFERENCE_SLOT_144),
            ],
        );
        e.mem.set_u32(reference + 0x150, three_d);
        for i in 0..3u32 {
            e.mem
                .set_f32(reference + 0x160 + 4 * i, 10.0 * (i + 1) as f32);
        }
        e.mem.set_f32(reference + 0x180, health);
        reference
    }

    #[test]
    fn current_damage_stage_follows_the_health() {
        let mut e = engine();
        node_doubles(&mut e);
        reference_doubles(&mut e);
        let stages = [
            StageSpec {
                damage_stage: 1,
                percentage: 75,
                ..StageSpec::default()
            },
            StageSpec {
                damage_stage: 0,
                percentage: 60,
                ..StageSpec::default()
            },
            StageSpec {
                damage_stage: 2,
                percentage: 50,
                ..StageSpec::default()
            },
            StageSpec {
                damage_stage: 3,
                percentage: 25,
                ..StageSpec::default()
            },
        ];
        let (this, _) = make_component(&mut e, 100, &stages);
        let node = make_node(&mut e, 4, true, &[]);
        let reference = make_reference(&mut e, node, 45.0);
        e.call_log = Some(vec![]);
        // Health 45 %: the stages at 75, 60 and 50 count (the one at 25 ends
        // the walk); the last damage stage among them is 2.
        assert!(e.call(0x0047_6f50, &args![this, reference, false]).bool());
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(RANGE_NODE_SET_CURRENT, vec![node, 2])));
        // Health 20 %: all four count.
        e.mem.set_f32(reference + 0x180, 20.0);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_6f50, &args![this, reference, false]).bool());
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(RANGE_NODE_SET_CURRENT, vec![node, 3])));
        // Health above every stage: no damage stage, so nothing unless forced.
        // A stage below the health ends the walk before any damage stage.
        let first_only = [StageSpec {
            damage_stage: 1,
            percentage: 50,
            ..StageSpec::default()
        }];
        let (low, _) = make_component(&mut e, 100, &first_only);
        e.mem.set_f32(reference + 0x180, 60.0);
        assert!(!e.call(0x0047_6f50, &args![low, reference, false]).bool());
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_6f50, &args![low, reference, true]).bool());
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(RANGE_NODE_SET_CURRENT, vec![node, 0])));
    }

    #[test]
    fn current_damage_stage_handles_undamaged_and_unseen_references() {
        let mut e = engine();
        node_doubles(&mut e);
        reference_doubles(&mut e);
        let (this, _) = make_component(
            &mut e,
            100,
            &[StageSpec {
                damage_stage: 1,
                percentage: 50,
                ..StageSpec::default()
            }],
        );
        let node = make_node(&mut e, 4, true, &[]);
        // Never damaged (-1.0): false unless forced.
        let reference = make_reference(&mut e, node, -1.0);
        assert!(!e.call(0x0047_6f50, &args![this, reference, false]).bool());
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_6f50, &args![this, reference, true]).bool());
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(RANGE_NODE_SET_CURRENT, vec![node, 0])));
        // No 3D at all.
        let hidden = make_reference(&mut e, 0, 50.0);
        assert!(!e.call(0x0047_6f50, &args![this, hidden, true]).bool());
    }

    #[test]
    fn repair_lowers_the_health_to_the_stage_reached() {
        let mut e = engine();
        node_doubles(&mut e);
        reference_doubles(&mut e);
        noop(&mut e, &[0x0042_e150]);
        e.register(0x007a_f430, |_, _| ret(0x4242));
        let stages = [
            StageSpec {
                percentage: 80,
                rate: 5,
                model: 0x100,
                ..StageSpec::default()
            },
            StageSpec {
                percentage: 50,
                rate: 7,
                model: 0x200,
                ..StageSpec::default()
            },
            StageSpec {
                percentage: 20,
                rate: 9,
                ..StageSpec::default()
            },
        ];
        let (this, _) = make_component(&mut e, 100, &stages);
        // No 3D, so the damage-stage refresh at the end stops at once.
        let reference = make_reference(&mut e, 0, 60.0);
        e.call_log = Some(vec![]);
        e.call(0x0047_7090, &args![this, reference]);
        let log = e.call_log.take().unwrap();
        // The walk ends on the last stage (20 %): health 20, model 0x200.
        assert_eq!(e.mem.f32(reference + 0x180), 20.0);
        assert!(log.contains(&(0x0042_e150, vec![reference + 0x180, 0x200, 0x4242])));
        assert!(log.contains(&(REFERENCE_NOOP, vec![reference, 0, 1])));
        // The last stage reached is above the health: nothing changes.
        let (high, _) = make_component(
            &mut e,
            100,
            &[StageSpec {
                percentage: 90,
                rate: 3,
                ..StageSpec::default()
            }],
        );
        e.mem.set_f32(reference + 0x180, 60.0);
        e.call(0x0047_7090, &args![high, reference]);
        assert_eq!(e.mem.f32(reference + 0x180), 60.0);
        // A first stage below the health without a previous rate stops the walk.
        let (stop, _) = make_component(
            &mut e,
            100,
            &[StageSpec {
                percentage: 40,
                rate: 0,
                ..StageSpec::default()
            }],
        );
        e.call(0x0047_7090, &args![stop, reference]);
        assert_eq!(e.mem.f32(reference + 0x180), 60.0);
        // Undamaged: nothing.
        e.mem.set_f32(reference + 0x180, -1.0);
        e.call(0x0047_7090, &args![this, reference]);
        assert_eq!(e.mem.f32(reference + 0x180), -1.0);
    }

    #[test]
    fn nodes_are_collected_into_the_array() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let mut e = engine();
        node_doubles(&mut e);
        e.register(0x0041_3f40, |_, a| ret(a[0]));
        // `0043b1b0` and `009a3830` read flags at +0x40 and +0x44 of the container.
        e.register(0x0043_b1b0, |e, a| ret(e.mem.u32(a[0] + 0x40)));
        e.register(0x009a_3830, |e, a| {
            assert_eq!(a[1], 0xf11);
            ret(e.mem.u32(a[0] + 0x44))
        });
        e.set_global(0x011c_61d8, 0xf11u32);
        let added = Rc::new(RefCell::new(Vec::new()));
        let sink = added.clone();
        e.register_double(0x007c_b2e0, move |e, a| {
            sink.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        let leaf = make_node(&mut e, 0, true, &[]);
        let inner = make_node(&mut e, 0, true, &[leaf]);
        let root = make_node(&mut e, 0, true, &[inner]);
        let root_container = e.mem.u32(root + 8);
        let inner_container = e.mem.u32(inner + 8);
        // The root's container passes both checks; the inner one only the first.
        e.mem.set_u32(root_container + 0x40, 1);
        e.mem.set_u32(root_container + 0x44, 1);
        e.mem.set_u32(inner_container + 0x40, 1);
        let this: Ptr<BGSDestructibleObjectForm> = e.new_object();
        let array = e.mem.alloc(0x10);
        e.call(0x0047_7230, &args![this, root, array]);
        assert_eq!(*added.borrow(), vec![(array, root_container)]);
        // A null node is ignored.
        e.call(0x0047_7230, &args![this, 0u32, array]);
        assert_eq!(added.borrow().len(), 1);
        assert_eq!(e.call(0x0047_72e0, &args![]).u32(), 0xf11);
    }

    #[test]
    fn self_damage_goes_to_every_reference_in_the_map() {
        use std::cell::Cell;
        use std::rc::Rc;
        let mut e = engine();
        reference_doubles(&mut e);
        e.register(0x0084_d030, |_, a| {
            assert_eq!(a[0], 0x011f_6394);
            2.0f32.into_ret()
        });
        e.register(0x004b_9ba0, |_, a| {
            assert_eq!(a[0], DESTRUCTIBLE_OBJECTS);
            ret(1)
        });
        let actor = make_reference(&mut e, 0, 50.0);
        e.mem.set_u32(actor + 0x154, 1);
        let plain = make_reference(&mut e, 0, 50.0);
        let skipped = make_reference(&mut e, 0, 50.0);
        e.mem.set_u32(skipped + 0x15c, 1);
        let entries = [
            (actor, 10u32, 2u32),
            (plain, 4, 3),
            (skipped, 6, 0),
            (0, 0, 0),
        ];
        let step = Rc::new(Cell::new(0usize));
        let counter = step.clone();
        e.register_double(0x006b_7f20, move |e, a| {
            assert_eq!(a[0], DESTRUCTIBLE_OBJECTS);
            let (key, value, next) = entries[counter.get()];
            counter.set(counter.get() + 1);
            e.mem.set_u32(a[1], next);
            e.mem.set_u32(a[2], key);
            e.mem.set_u32(a[3], value);
            Ret::default()
        });
        // The skipped reference fails the first test.
        e.register(REFERENCE_TEST_A, |e, a| ret(e.mem.u32(a[0] + 0x15c)));
        e.register(REFERENCE_TEST_B, |_, _| Ret::default());
        e.register(0x0043_d4d0, |_, a| {
            assert_eq!(a[0], 0x011c_3ea4);
            ret(0x011c_3ea8)
        });
        e.set_global(0x011c_3ea8, 1u32);
        noop(&mut e, &[REFERENCE_SLOT_338, REFERENCE_SLOT_144]);
        e.call_log = Some(vec![]);
        e.call(0x0047_72f0, &args![]);
        let log = e.call_log.take().unwrap();
        let slots: Vec<_> = log
            .iter()
            .filter(|(a, _)| *a == REFERENCE_SLOT_338 || *a == REFERENCE_SLOT_144)
            .cloned()
            .collect();
        assert_eq!(
            slots,
            vec![
                (REFERENCE_SLOT_338, vec![actor, 20.0f32.to_bits(), 0, 0]),
                (REFERENCE_SLOT_144, vec![plain, 8.0f32.to_bits(), 1]),
            ]
        );
        // Any other mode hands the damage to the queue object.
        e.set_global(0x011c_3ea8, 0u32);
        step.set(0);
        e.register(0x0045_37b0, |_, _| ret(0x5000));
        noop(&mut e, &[0x0087_af50]);
        e.call_log = Some(vec![]);
        e.call(0x0047_72f0, &args![]);
        let log = e.call_log.take().unwrap();
        let queued: Vec<_> = log
            .iter()
            .filter(|(a, _)| *a == 0x0087_af50)
            .cloned()
            .collect();
        assert_eq!(
            queued,
            vec![
                (0x0087_af50, vec![0x5000, actor, 20.0f32.to_bits(), 1]),
                (0x0087_af50, vec![0x5000, plain, 8.0f32.to_bits(), 1]),
            ]
        );
    }

    #[test]
    fn removing_from_the_map_forwards_the_reference() {
        let mut e = engine();
        noop(&mut e, &[MAP_REMOVE_AT]);
        e.call_log = Some(vec![]);
        e.call(0x0047_7410, &args![0x1234u32]);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(MAP_REMOVE_AT, vec![DESTRUCTIBLE_OBJECTS, 0x1234])));
    }

    #[test]
    fn stage_index_reached_by_a_reference() {
        let mut e = engine();
        form_type_double(&mut e);
        reference_doubles(&mut e);
        let form = form_of_type(&mut e, 0x15);
        let stages = [
            StageSpec {
                percentage: 75,
                ..StageSpec::default()
            },
            StageSpec {
                percentage: 50,
                ..StageSpec::default()
            },
            StageSpec {
                percentage: 25,
                ..StageSpec::default()
            },
        ];
        let (_, data) = make_component(&mut e, 100, &stages);
        e.mem.set_u32(form + 0x68 + 4, data.addr());
        e.register(0x007a_f430, |e, a| ret(e.mem.u32(a[0] + 0x1a0)));
        let reference = make_reference(&mut e, 0, 60.0);
        e.mem.set_u32(reference + 0x1a0, form);
        // 60 %: the first stage below it is stage 1 (50).
        assert_eq!(e.call(0x0047_7430, &args![reference]).u32(), 1);
        e.mem.set_f32(reference + 0x180, 10.0);
        assert_eq!(e.call(0x0047_7430, &args![reference]).u32(), 3);
        e.mem.set_f32(reference + 0x180, 100.0);
        assert_eq!(e.call(0x0047_7430, &args![reference]).u32(), 0);
        // Undamaged, no component, null reference.
        e.mem.set_f32(reference + 0x180, -1.0);
        assert_eq!(e.call(0x0047_7430, &args![reference]).u32(), u32::MAX);
        let other = form_of_type(&mut e, 0x14);
        let unseen = make_reference(&mut e, 0, 60.0);
        e.mem.set_u32(unseen + 0x1a0, other);
        assert_eq!(e.call(0x0047_7430, &args![unseen]).u32(), u32::MAX);
        assert_eq!(e.call(0x0047_7430, &args![0u32]).u32(), u32::MAX);
    }

    #[test]
    fn small_debris_is_hidden() {
        let mut e = engine();
        node_doubles(&mut e);
        noop(&mut e, &[0x0045_0f90]);
        // A node keeps its collision object at +0x28; the collision object
        // its shape at +0x2c; the shape its type word at +0x30.
        e.register(0x0043_b610, |e, a| ret(e.mem.u32(a[0] + 0x28)));
        e.register(0x006f_a820, |e, a| ret(e.mem.u32(a[0] + 0x2c)));
        e.register(0x0043_b4f0, |e, a| {
            let kind = e.mem.u32(a[0] + 0x30);
            e.mem.set_u32(a[1], kind);
            ret(a[1])
        });
        e.register(0x0043_b4d0, |e, a| ret(e.mem.u32(a[0]) & 0x7f));
        let with_shape = |e: &mut Engine, node: u32, kind: u32| {
            let shape = e.mem.alloc(0x40);
            e.mem.set_u32(shape + 0x30, kind);
            let collision = e.mem.alloc(0x40);
            e.mem.set_u32(collision + 0x2c, shape);
            e.mem.set_u32(node + 0x28, collision);
        };
        let small = make_node(&mut e, 0, true, &[]);
        with_shape(&mut e, small, 0x13);
        let big = make_node(&mut e, 0, true, &[]);
        with_shape(&mut e, big, 0x07);
        let plain = make_node(&mut e, 0, true, &[]);
        let parent = make_node(&mut e, 0, true, &[big, plain, small]);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0047_7560, &args![parent]).bool());
        let log = e.call_log.take().unwrap();
        let hidden: Vec<_> = log
            .iter()
            .filter(|(a, _)| *a == 0x0045_0f90)
            .cloned()
            .collect();
        assert_eq!(hidden, vec![(0x0045_0f90, vec![small, 1])]);
        // Nothing small below a node: false and nothing hidden; a null node
        // is false too.
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0047_7560, &args![big]).bool());
        assert!(!e.call(0x0047_7560, &args![0u32]).bool());
        let log = e.call_log.take().unwrap();
        assert!(!log.iter().any(|(a, _)| *a == 0x0045_0f90));
    }

    #[test]
    fn files_of_every_stage_are_queued() {
        let mut e = engine();
        e.register(0x0045_c6b0, |_, a| ret(a[0] + 100));
        noop(&mut e, &[0x0044_3d30, REFERENCE_NOOP]);
        e.set_global(0x011c_3b3c, 0x9000u32);
        // Explosion and debris forms with an embedded model whose vtable
        // has slot 0x10.
        let explosion = e.mem.alloc(0x100);
        let explosion_table = e.mem.alloc(0x40);
        e.mem.set_u32(explosion_table + 0x10, REFERENCE_NOOP);
        e.mem.set_u32(explosion + 0x64, explosion_table);
        let debris = e.mem.alloc(0x100);
        let debris_table = e.mem.alloc(0x40);
        e.mem.set_u32(debris_table + 0x10, REFERENCE_NOOP);
        e.mem.set_u32(debris + 0x18, debris_table);
        let stages = [
            StageSpec {
                model: 0x3000,
                ..StageSpec::default()
            },
            StageSpec {
                explosion,
                debris,
                ..StageSpec::default()
            },
            StageSpec::default(),
        ];
        let (this, _) = make_component(&mut e, 100, &stages);
        e.call_log = Some(vec![]);
        e.call(0x0047_7640, &args![this, 7u32, 8u32, 9u32]);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x0044_3d30, vec![0x9000, 0x3000, 8, 9, 107, 1, 0, 0])));
        assert!(log.contains(&(REFERENCE_NOOP, vec![explosion + 0x64, 8, 9])));
        assert!(log.contains(&(REFERENCE_NOOP, vec![debris + 0x18, 8, 9])));
        // A component without data does nothing.
        let empty: Ptr<BGSDestructibleObjectForm> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0047_7640, &args![empty, 7u32, 8u32, 9u32]);
        assert!(e.call_log.take().unwrap().len() == 1);
    }

    // ---- the damage routine -------------------------------------------

    /// Doubles for everything the damage routine calls. Settings (`float`s
    /// read through `00403e20`): A 1.0, B 2.0, limit 100.0, margin 5.0,
    /// fallback 9.0.
    fn damage_engine() -> Engine {
        let mut e = engine();
        node_doubles(&mut e);
        reference_doubles(&mut e);
        noop(
            &mut e,
            &[
                REFERENCE_TEST_A,
                REFERENCE_TEST_B,
                NODE_HOLDER_DESTROY,
                0x005a_a500,
                0x0048_4530,
                0x0048_4650,
                0x009b_c8f0,
                0x005a_c750,
                MAP_SET_AT,
                MAP_REMOVE_AT,
                SPAWN_DEBRIS,
                CREATE_EXPLOSION,
                0x0057_a3c0,
                0x0047_78a0,
                0x009a_54a0,
                0x009a_51e0,
            ],
        );
        e.register(NODE_HOLDER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
        e.register(0x0043_fcd0, |e, a| ret(e.mem.u32(a[0] + 0x150)));
        e.register(0x0047_7a50, |_, _| (-1.0f32).into_ret());
        e.register(0x0047_7970, |_, _| (-1.0f32).into_ret());
        e.register(SETTING_POINTER, |_, a| ret(a[0]));
        e.register(0x0082_5c00, |_, _| ret(0x7777));
        e.set_global(0x011c_e720, 1.0f32);
        e.set_global(0x011c_f1b0, 2.0f32);
        e.set_global(0x011c_eabc, 100.0f32);
        e.set_global(0x011c_e1b8, 5.0f32);
        e.set_global(0x011c_ef20, 9.0f32);
        e
    }

    fn damage(e: &mut Engine, this: Ptr<BGSDestructibleObjectForm>, reference: u32, amount: f32) {
        e.call(0x0047_5b20, &args![this, reference, amount, 0u32]);
    }

    fn health_of(e: &Engine, reference: u32) -> f32 {
        e.mem.f32(reference + 0x180)
    }

    fn calls_to(log: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    #[test]
    fn damage_is_ignored_when_it_does_not_apply() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            damage_stage: 1,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        // Not above 0.
        damage(&mut e, this, reference, 0.0);
        damage(&mut e, this, reference, -5.0);
        // No reference, no data.
        e.call(0x0047_5b20, &args![this, 0u32, 10.0f32, 0u32]);
        let (no_data, _) = make_component(&mut e, 100, &[]);
        let data = e.get(no_data, BGSDestructibleObjectForm::pData);
        e.set(no_data, BGSDestructibleObjectForm::pData, Ptr::NULL);
        let _ = data;
        damage(&mut e, no_data, reference, 10.0);
        // No 3D.
        let hidden = make_reference(&mut e, 0, 100.0);
        damage(&mut e, this, hidden, 10.0);
        assert_eq!(health_of(&e, reference), 100.0);
        assert_eq!(health_of(&e, hidden), 100.0);
        // Either reference test.
        e.register(REFERENCE_TEST_A, |_, _| ret(1));
        damage(&mut e, this, reference, 10.0);
        assert_eq!(health_of(&e, reference), 100.0);
        e.register(REFERENCE_TEST_A, |_, _| Ret::default());
        e.register(REFERENCE_TEST_B, |_, _| ret(1));
        damage(&mut e, this, reference, 10.0);
        assert_eq!(health_of(&e, reference), 100.0);
        // An exhausted object (0 health) is left alone as well.
        e.register(REFERENCE_TEST_B, |_, _| Ret::default());
        e.mem.set_f32(reference + 0x180, 0.0);
        damage(&mut e, this, reference, 10.0);
        assert_eq!(health_of(&e, reference), 0.0);
    }

    #[test]
    fn damage_crossing_a_stage_updates_nodes_map_debris_and_threats() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            damage_stage: 1,
            rate: 5,
            debris: 0x2000,
            count: 2,
            ..StageSpec::default()
        };
        let (this, data) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        let _ = data;
        assert_eq!(health_of(&e, reference), 70.0);
        // Virtual slot 0x48 with 0x80000000.
        assert!(log.contains(&(REFERENCE_NOOP, vec![reference, 0x8000_0000])));
        // The damage-stage nodes and sounds.
        assert!(log.contains(&(0x0057_a3c0, vec![reference, 0])));
        // The self-damage map.
        assert!(log.contains(&(MAP_SET_AT, vec![DESTRUCTIBLE_OBJECTS, reference, 5])));
        assert!(calls_to(&log, MAP_REMOVE_AT).is_empty());
        // The crossed stage's action flag, and debris at the reference's own
        // position (no debris node).
        assert!(log.contains(&(0x005a_c750, vec![reference, reference + 0x180, 0x0020_0000])));
        let spawned = calls_to(&log, SPAWN_DEBRIS);
        assert_eq!(spawned.len(), 1);
        assert_eq!(spawned[0][0], 0x2000);
        assert_eq!(spawned[0][1], 0x1234);
        assert_eq!(spawned[0][3..], [2, 1.5f32.to_bits()]);
        // The threat routines get the new percentage and the settings.
        let threat = calls_to(&log, 0x0047_7a50);
        assert_eq!(threat.len(), 1);
        assert_eq!(
            threat[0][..4],
            [this.addr(), 70, 2.0f32.to_bits(), 1.0f32.to_bits()]
        );
        // Nothing came back: the object is removed from the threat map.
        assert!(log.contains(&(0x009a_51e0, vec![0x7777, reference])));
        assert!(calls_to(&log, 0x009a_54a0).is_empty());
        // Alive: no death routine.
        assert!(calls_to(&log, 0x0047_78a0).is_empty());
    }

    #[test]
    fn damage_without_a_self_damage_rate_leaves_the_map() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            damage_stage: 1,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(MAP_REMOVE_AT, vec![DESTRUCTIBLE_OBJECTS, reference])));
        assert!(calls_to(&log, MAP_SET_AT).is_empty());
        // No rate: the first threat routine is not asked.
        assert!(calls_to(&log, 0x0047_7a50).is_empty());
        assert_eq!(calls_to(&log, 0x0047_7970).len(), 1);
    }

    #[test]
    fn damage_below_the_old_stage_does_nothing_more() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            damage_stage: 1,
            rate: 5,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        // 70 % already; 70 -> 60 crosses nothing, and the stage at 80 is
        // an earlier one ("passed" before).
        let reference = make_reference(&mut e, root, 70.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 10.0);
        let log = e.call_log.take().unwrap();
        assert_eq!(health_of(&e, reference), 60.0);
        assert!(calls_to(&log, 0x0057_a3c0).is_empty());
        assert!(calls_to(&log, MAP_SET_AT).is_empty());
        assert!(calls_to(&log, MAP_REMOVE_AT).is_empty());
        assert!(calls_to(&log, 0x005a_c750).is_empty());
        // The earlier stage's rate (5) is still handed to the threat routine.
        assert_eq!(calls_to(&log, 0x0047_7a50).len(), 1);
    }

    #[test]
    fn a_stage_can_cap_the_damage() {
        let mut e = damage_engine();
        let stages = [
            StageSpec {
                percentage: 80,
                flags: STAGE_FLAG_CAP_DAMAGE,
                rate: 3,
                ..StageSpec::default()
            },
            StageSpec {
                percentage: 60,
                rate: 4,
                ..StageSpec::default()
            },
        ];
        let (this, _) = make_component(&mut e, 100, &stages);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 50.0);
        let log = e.call_log.take().unwrap();
        // The health stops at the stage's 80 % and the walk ends there.
        assert_eq!(health_of(&e, reference), 80.0);
        assert!(log.contains(&(MAP_SET_AT, vec![DESTRUCTIBLE_OBJECTS, reference, 3])));
        // New percentage 80 for the threat routine.
        assert_eq!(calls_to(&log, 0x0047_7a50)[0][1], 80);
    }

    #[test]
    fn stage_flags_disable_and_destroy() {
        // Disable: the plain way (slot 0x224 false) adds the pending
        // disabled reference and deletes the form.
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            flags: STAGE_FLAG_DISABLE_OBJECT,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x005a_a500, vec![reference, 0])));
        assert!(log.contains(&(0x0048_4530, vec![reference, 1])));
        assert!(calls_to(&log, 0x009b_c8f0).is_empty());
        // Slot 0x224 true: the other routine.
        e.mem.set_f32(reference + 0x180, 100.0);
        e.mem.set_u32(reference + 0x158, 1);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x009b_c8f0, vec![reference])));
        assert!(calls_to(&log, 0x005a_a500).is_empty());
        // Destroy.
        let stage = StageSpec {
            percentage: 80,
            flags: STAGE_FLAG_DESTROY_OBJECT,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        e.mem.set_f32(reference + 0x180, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x0048_4650, vec![reference, 1])));
    }

    #[test]
    fn lethal_damage_runs_the_death_routine() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 25,
            rate: 5,
            explosion: 0,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 150.0);
        let log = e.call_log.take().unwrap();
        assert_eq!(health_of(&e, reference), 0.0);
        // At 0 health the form is flagged (`00484650`) before the health is
        // stored, the object leaves the self-damage map, and `004778a0` runs.
        assert!(log.contains(&(0x0048_4650, vec![reference, 1])));
        assert!(log.contains(&(MAP_REMOVE_AT, vec![DESTRUCTIBLE_OBJECTS, reference])));
        assert!(log.contains(&(0x0047_78a0, vec![this.addr()])));
        assert!(calls_to(&log, MAP_SET_AT).is_empty());
    }

    #[test]
    fn an_actors_health_is_its_actor_value() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 50,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.mem.set_u32(reference + 0x154, 1);
        // Interface at +0xa4: slot 4 gives the total (200.0), slot 8 the
        // current value after the damage (80).
        e.register(0x0f00_0020, |_, _| 200.0f32.into_ret());
        e.register(0x0f00_0021, |_, a| {
            assert_eq!(a[1], 0x10);
            ret(80)
        });
        let table = e.mem.alloc(0x40);
        e.mem.set_u32(table + 4, 0x0f00_0020);
        e.mem.set_u32(table + 8, 0x0f00_0021);
        e.mem.set_u32(reference + 0xa4, table);
        damage(&mut e, this, reference, 30.0);
        assert_eq!(health_of(&e, reference), 80.0);
    }

    /// Doubles for the model swap.
    fn swap_doubles(e: &mut Engine, node: u32) {
        e.set_global(0x011c_4800, node);
        e.set_global(0x011d_df38, 0x1111u32);
        e.set_global(0x011d_ea10, 0x2222u32);
        e.register(ARRAY_CONSTRUCT, |e, a| {
            let buffer = e.mem.alloc(4);
            let node: u32 = e.global(0x011c_4800);
            e.mem.set_u32(buffer, node);
            e.mem.set_u32(a[0] + 4, buffer);
            e.mem.set_u32(a[0] + 8, 1);
            ret(a[0])
        });
        e.register(ARRAY_SIZE, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(ARRAY_ELEMENT, |e, a| ret(e.mem.u32(a[0] + 4) + 4 * a[1]));
        noop(
            e,
            &[
                ARRAY_DESTROY,
                0x0040_b460,
                0x0057_b240,
                0x0042_e150,
                0x0045_1ef0,
                0x00c6_a270,
                GET_ORIENTATION,
                0x0056_fa90,
                UPDATE_3D_POSITION,
                0x0043_9ef0,
                0x0063_c8a0,
                0x0057_5830,
                0x0040_1050,
            ],
        );
        e.register(0x0068_15c0, |_, a| ret(a[0]));
        e.register(0x0043_f8d0, |_, a| ret(a[1]));
        e.register(0x0057_b200, |_, _| ret(1));
        e.register(0x0046_23f0, |_, _| ret(0x33));
        e.register(0x007a_f430, |_, _| ret(0x4242));
    }

    #[test]
    fn a_replacement_model_is_swapped_in_with_the_old_transform() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            model: 0x6000,
            ..StageSpec::default()
        };
        let (this, data) = make_component(&mut e, 100, &[stage]);
        e.mem.set_u32(data.addr() + 0x10, 0x5000);
        let root = make_node(&mut e, 0, true, &[]);
        let gathered = make_node(&mut e, 0, true, &[]);
        swap_doubles(&mut e, gathered);
        let reference = make_reference(&mut e, root, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        // No damage-stage update: the model path was taken.
        assert!(calls_to(&log, 0x0057_a3c0).is_empty());
        // The preloaded model is counted and released at the end.
        assert!(log.contains(&(0x0040_b460, vec![data.addr() + 0xc])));
        assert!(log.contains(&(0x0047_78a0, vec![this.addr()])));
        // The model goes into the extra data with the form `007af430` gave.
        assert!(log.contains(&(0x0042_e150, vec![reference + 0x180, 0x6000, 0x4242])));
        assert!(log.contains(&(0x0045_1ef0, vec![0x2222, reference, 0x1234, 0, 0])));
        // The flags around the swap.
        let flags = calls_to(&log, 0x0046_23f0);
        assert_eq!(flags, vec![vec![0x1111, 0], vec![0x1111, 0x33]]);
        assert!(log.contains(&(0x0057_b240, vec![reference, 1])));
        // Both node arrays were gathered, the new 3D activated.
        assert_eq!(calls_to(&log, ARRAY_CONSTRUCT).len(), 2);
        assert_eq!(calls_to(&log, ARRAY_DESTROY).len(), 2);
        assert!(log.contains(&(0x00c6_a270, vec![root, 1, 1, 0])));
        // The orientation and the position are carried over.
        let orientation = calls_to(&log, 0x0056_fa90);
        assert_eq!(orientation.len(), 1);
        assert_eq!(orientation[0].len(), 10);
        assert_eq!(orientation[0][0], reference);
        assert_eq!(calls_to(&log, 0x0057_5830).len(), 1);
        assert_eq!(calls_to(&log, UPDATE_3D_POSITION).len(), 2);
        // The translation difference is `old translation - new translation`
        // with the nodes' translations (+0x24 of the copied transforms).
        let difference = calls_to(&log, 0x0043_9ef0);
        assert_eq!(difference.len(), 1);
        assert_eq!(difference[0].len(), 3);
        // The matrix product runs twice (old * transpose(new) * orientation).
        assert_eq!(calls_to(&log, 0x0043_f8d0).len(), 2);
    }

    #[test]
    fn a_replacement_model_without_nodes_skips_the_transform() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            model: 0x6000,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        swap_doubles(&mut e, root);
        // Arrays that stay empty.
        e.register(ARRAY_CONSTRUCT, |_, a| ret(a[0]));
        let reference = make_reference(&mut e, root, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x0042_e150, vec![reference + 0x180, 0x6000, 0x4242])));
        assert!(calls_to(&log, 0x0056_fa90).is_empty());
        assert!(calls_to(&log, 0x0057_5830).is_empty());
        assert_eq!(calls_to(&log, ARRAY_CONSTRUCT).len(), 1);
        assert!(log.contains(&(0x00c6_a270, vec![root, 1, 1, 0])));
        // No preloaded model: no release.
        assert!(calls_to(&log, 0x0047_78a0).is_empty());
    }

    #[test]
    fn an_explosion_is_created_at_a_random_point_of_the_bound() {
        use std::cell::Cell;
        use std::rc::Rc;
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            explosion: 0x7000,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        for (i, v) in [5.0f32, 6.0, 7.0, 2.0].iter().enumerate() {
            e.mem.set_f32(root + 0x90 + 4 * i as u32, *v);
        }
        e.register(GET_WORLD_BOUND, |_, a| ret(a[0] + 0x90));
        e.register(0x0068_15c0, |_, a| ret(a[0]));
        e.register(0x0084_d030, |e, a| e.mem.f32(a[0] + 0xc).into_ret());
        e.register(0x0043_9180, |e, a| {
            // Keep the vector and the radius for the test.
            for i in 0..3 {
                let v = e.mem.u32(a[0] + 4 * i);
                e.mem.set_u32(0x011c_4900 + 4 * i, v);
            }
            e.mem.set_u32(0x011c_4910, a[1]);
            Ret::default()
        });
        noop(&mut e, &[0x0043_9e90, 0x00aa_5140, GET_ORIENTATION]);
        e.register(0x00aa_5230, |_, _| ret(0x8000_0000));
        let reference = make_reference(&mut e, root, 100.0);
        e.mem.set_u32(reference + 0xf8, 0xcafe);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        // x and y are in [-1, 1], z in [-1, 0]; the word is exactly half.
        assert_eq!(e.mem.f32(0x011c_4900), 0.0);
        assert_eq!(e.mem.f32(0x011c_4904), 0.0);
        assert_eq!(e.mem.f32(0x011c_4908), -0.5);
        assert_eq!(e.mem.f32(0x011c_4910), 2.0);
        let created = calls_to(&log, CREATE_EXPLOSION);
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].len(), 16);
        assert_eq!(created[0][..4], [0x7000, reference, reference, 0x1234]);

        // Slot 0x224 true: the other arguments, and the new explosion
        // copies the reference's word at +0xf8 into its +0xe0.
        e.mem.set_f32(reference + 0x180, 100.0);
        e.mem.set_u32(reference + 0x158, 1);
        e.register(0x0087_4480, |_, _| ret(0x9999));
        let spawned = Rc::new(Cell::new(0u32));
        let slot = spawned.clone();
        e.register_double(CREATE_EXPLOSION, move |e, _| {
            let block = e.mem.alloc(0x100);
            slot.set(block);
            ret(block)
        });
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        let created = calls_to(&log, CREATE_EXPLOSION);
        assert_eq!(created.len(), 1);
        assert_eq!(created[0][..4], [0x7000, 0x9999, 0, 0x1234]);
        assert_eq!(e.mem.u32(spawned.get() + 0xe0), 0xcafe);
    }

    #[test]
    fn explosions_prefer_the_nodes_of_the_3d() {
        let mut e = damage_engine();
        let stage = StageSpec {
            percentage: 80,
            explosion: 0x7000,
            debris: 0x2000,
            count: 3,
            damage_stage: 4,
            ..StageSpec::default()
        };
        let (this, _) = make_component(&mut e, 100, &[stage]);
        // A node that is both an explosion and a debris node.
        let hit = make_node(&mut e, 1 | 2, true, &[]);
        let root = make_node(&mut e, 0, true, &[hit]);
        let reference = make_reference(&mut e, root, 100.0);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        let created = calls_to(&log, CREATE_EXPLOSION);
        assert_eq!(created.len(), 1);
        assert_eq!(created[0][..4], [0x7000, reference, reference, 0x1234]);
        // The debris came from the node's translation, not the reference's.
        let spawned = calls_to(&log, SPAWN_DEBRIS);
        assert_eq!(spawned.len(), 1);
        assert_eq!(
            spawned[0],
            vec![0x2000, 0x1234, hit + 0x80, 3, 1.5f32.to_bits()]
        );
        // The damage stage reached `SetCurrent` as the selector.
        assert!(log.contains(&(RANGE_NODE_SET_CURRENT, vec![hit, 4])));
    }

    #[test]
    fn the_threat_map_gets_the_result_of_the_routines() {
        let stage = StageSpec {
            percentage: 80,
            rate: 5,
            ..StageSpec::default()
        };
        // The first routine answers 3.0 (and a key): the object is added.
        let mut e = damage_engine();
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.register(0x0047_7a50, |e, a| {
            e.mem.set_u32(a[4], 0x55);
            3.0f32.into_ret()
        });
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x009a_54a0, vec![0x7777, reference, 0x55, 3.0f32.to_bits()])));
        assert!(calls_to(&log, 0x0047_7970).is_empty());
        assert!(calls_to(&log, 0x009a_51e0).is_empty());

        // The answer is above the limit setting (100.0 here set to 1.0):
        // discarded, the second routine is asked, and a margin above the
        // health difference selects the fallback value.
        let mut e = damage_engine();
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.register(0x0047_7a50, |_, _| 3.0f32.into_ret());
        e.register(0x0047_7970, |_, _| 4.0f32.into_ret());
        e.set_global(0x011c_eabc, 1.0f32);
        e.set_global(0x011c_e1b8, 100.0f32);
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, 0x0047_7970).len(), 1);
        assert!(log.contains(&(0x009a_54a0, vec![0x7777, reference, 0, 9.0f32.to_bits()])));

        // A margin below the difference keeps "no result".
        let mut e = damage_engine();
        let (this, _) = make_component(&mut e, 100, &[stage]);
        let root = make_node(&mut e, 0, true, &[]);
        let reference = make_reference(&mut e, root, 100.0);
        e.register(0x0047_7970, |_, _| 4.0f32.into_ret());
        e.call_log = Some(vec![]);
        damage(&mut e, this, reference, 30.0);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(0x009a_51e0, vec![0x7777, reference])));
    }
}
