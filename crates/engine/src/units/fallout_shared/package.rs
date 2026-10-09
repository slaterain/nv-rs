//! `fallout shared/package.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds `TESPackage` (91 functions to translate). The first
//! session translated the first 40, `006707c0` to `00674d70`: the
//! constructor, `CreatePackage`, `SetPackType`, the flag accessors of
//! `PACKAGE_DATA`, the location, second-location and target setters, the
//! dialogue-data accessors, `InitItem` and `SetIsCreated`. The next session
//! continues with the first `open` function after `00674d70`
//! (`00674dd0`).
//!
//! ## Layout (PC build)
//!
//! `TESPackage` is 0x80 bytes on PC (the constructor's allocation); the
//! Xbox PDB says 0x90 and its fields after `TESForm` sit 0x10 lower here,
//! because the PC `TESForm` has no editor ID or version-control fields. The
//! `PACKAGE_DATA` member of the PDB (`iPackFlags` at PDB +0x2c) is
//! flattened into the `TESPackage` layout at its PC offsets (+0x1c).
//!
//! The parts of other classes are read at their offsets with the PDB name
//! in a comment, as their own units are not ours: `PackageLocation` (0xC),
//! `PackageTarget` (0x10), the per-procedure data object at `+0x28`
//! (`TESPackageData`, a vtable pointer and the fields of the subclass for
//! the package's procedure type).
//!
//! ## Helpers the translations call by address
//!
//! Nearly every callee is a one-line accessor of the original source (the
//! build did not optimise): `0041ca90` returns the package type
//! (`cPackType`, a sign-extended byte at +0x20), `0055b980` the location
//! (`pPackLoc`, +0x2c), `0087eaa0` the address of the conditions (+0x40),
//! `005f36f0` the idle collection (+0x34), `006733e0` and `00673400` get and
//! set the combat style (+0x48), `0084e3a0` the form ID (+0x0c), `004013e0`
//! tests form flag 0x8, `00484ab0` sets or clears it. The names the engine
//! map gives to some of these belong to other methods that the linker
//! folded onto the same body.
//!
//! `00401000` is `operator new`; objects are built the way the compiler
//! wrote it: allocate, and call the constructor only when the allocation
//! returned something. The scalar deleting destructors are called with
//! the flag 1.
//!
//! Not translated: the C++ exception-unwinding frames (`FS:[0]` chains) of
//! every function that has one.

#[allow(unused_imports)]
use crate::prelude::*;

/// `operator new(size)`.
const OPERATOR_NEW: u32 = 0x0040_1000;
/// The package type: a sign-extended byte at +0x20 (`PACKAGE_DATA::cPackType`).
const GET_PACK_TYPE: u32 = 0x0041_ca90;
/// `pPackLoc` getter (+0x2c).
const GET_LOCATION: u32 = 0x0055_b980;
/// Address of `packConditions` (+0x40).
const GET_CONDITIONS: u32 = 0x0087_eaa0;
/// `pIdleCollection` getter (+0x34).
const GET_IDLE_COLLECTION: u32 = 0x005f_36f0;
/// `pCombatStyle` getter (+0x48).
const GET_COMBAT_STYLE: u32 = 0x0067_33e0;
/// `pCombatStyle` setter (+0x48).
const SET_COMBAT_STYLE: u32 = 0x0067_3400;
/// `TESForm::iFormID` getter (+0x0c).
const GET_FORM_ID: u32 = 0x0084_e3a0;
/// Tests form flag 0x8 (`iFormFlags`, +0x08).
const GET_FORM_FLAG_8: u32 = 0x0040_13e0;
/// Sets (argument 1) or clears form flag 0x8.
const SET_FORM_FLAG_8: u32 = 0x0048_4ab0;
/// `TESForm::TESForm`.
const TES_FORM_CONSTRUCTOR: u32 = 0x0048_3370;
/// Sets `cFormType` (+0x04); the constructor passes 0x49.
const SET_FORM_TYPE: u32 = 0x004f_15a0;
/// `TESForm::GetFile(this, -1)`.
const TES_FORM_GET_FILE: u32 = 0x0048_4e60;
/// `TESForm::AddCompileIndex(&formId, file)` (cdecl).
const TES_FORM_ADD_COMPILE_INDEX: u32 = 0x0048_5d50;
/// Looks a form up by ID (cdecl, one argument): the pointer that goes
/// to `__RTDynamicCast`.
const LOOKUP_FORM: u32 = 0x0048_39c0;
/// `__RTDynamicCast(object, 0, from, to, 0)`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// RTTI type descriptors `InitItem` passes to `__RTDynamicCast`: from
/// `TESForm` to the class of the combat style.
const TYPE_DESCRIPTOR_TES_FORM: u32 = 0x0118_3028;
const TYPE_DESCRIPTOR_COMBAT_STYLE: u32 = 0x0118_627c;
/// Printf-style logger of the data loader (cdecl, variable arguments).
const LOG_FORM_WARNING: u32 = 0x005b_5e40;
/// `fr2Assert(file, line)`.
const ASSERT: u32 = 0x00aa_fb90;
/// The source file path string `fr2Assert` receives.
const PACKAGE_SOURCE_FILE: u32 = 0x0106_85dc;
/// `"FORMS: Could not find combat style (%08X) on package '%s' (%08X)."`.
const MESSAGE_COMBAT_STYLE_MISSING: u32 = 0x0106_8688;
/// `"MASTERFILE: Warnings were generated for package '%s' (%08). ..."`.
const MESSAGE_WARNINGS_GENERATED: u32 = 0x0106_8620;
/// The loader's warning counter (a thread-local field, read by a function).
const GET_WARNING_COUNT: u32 = 0x0046_e8a0;
/// The data handler singleton and its "is this form ID dynamic" test
/// (`formID >= 0xFF000000`).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const IS_DYNAMIC_FORM_ID: u32 = 0x0046_9860;
/// `0.0` as a `double`: the compare constant of `GetAcquireRadius`.
const ZERO: u32 = 0x0101_2060;

/// `TESPackage` vtable.
const TES_PACKAGE_VTABLE: u32 = 0x0106_847c;
/// Slot 0 of every package-data object: the scalar deleting destructor.
const SLOT_DELETE: u32 = 0x00;
/// Slot of `InitItem` in the package-data vtable (`TESPackageData`).
const SLOT_DATA_INIT_ITEM: u32 = 0x10;
/// `TESForm` slot 0x130: returns the name the loader prints for a form
/// (it takes no argument; the code still pushes the form ID right before
/// the call, which stays on the stack as the first variable argument of
/// the logger call that follows).
const SLOT_FORM_NAME: u32 = 0x130;

/// `PackageSchedule`, `TESCondition` and the three `PackageEventAction`
/// members: constructors, the setter of the event kind and `InitItem`.
const PACKAGE_SCHEDULE_CONSTRUCTOR: u32 = 0x0067_fcb0;
const TES_CONDITION_CONSTRUCTOR: u32 = 0x0068_0890;
const PACKAGE_EVENT_ACTION_CONSTRUCTOR: u32 = 0x0067_da40;
const PACKAGE_EVENT_ACTION_SET_KIND: u32 = 0x0041_fd00;
const PACKAGE_EVENT_ACTION_INIT_ITEM: u32 = 0x0067_e190;
const TES_CONDITION_INIT_ITEM: u32 = 0x0068_0a20;
const BGS_IDLE_COLLECTION_INIT_ITEM: u32 = 0x0047_9b00;

/// `PackageLocation` (0xC bytes): constructor, scalar deleting destructor,
/// `Copy(source, 0)`, `InitItem(package)`, `SetLocType`, `GetLocType`.
const LOCATION_SIZE: u32 = 0xc;
const LOCATION_CONSTRUCTOR: u32 = 0x0067_f030;
const LOCATION_DELETE: u32 = 0x0067_0b30;
const LOCATION_COPY: u32 = 0x0067_f4d0;
const LOCATION_INIT_ITEM: u32 = 0x0067_f760;
const LOCATION_SET_TYPE: u32 = 0x0067_f140;
const LOCATION_GET_TYPE: u32 = 0x0067_8ca0;
/// The location type `InitItem` sets for the types that decide their own
/// location at run time (a number; the exe does not name it here).
const LOCATION_TYPE_SIX: u32 = 6;
/// The `GetLocType` value for which `GetPackageSearchLocation` answers the
/// package's own location.
const LOCATION_TYPE_SEVEN: u32 = 7;

/// `PackageTarget` (0x10 bytes): constructor, scalar deleting destructor,
/// `Copy(source)`, `InitItem(package)`, `GetTargType`, `SetTargType`,
/// `iValue` getter (+0x08).
const TARGET_SIZE: u32 = 0x10;
const TARGET_CONSTRUCTOR: u32 = 0x0067_ff70;
const TARGET_DELETE: u32 = 0x007b_3fa0;
const TARGET_COPY: u32 = 0x0068_01a0;
const TARGET_INIT_ITEM: u32 = 0x0068_03b0;
const TARGET_GET_TYPE: u32 = 0x0051_9b00;
const TARGET_SET_TYPE: u32 = 0x0068_00b0;
const TARGET_GET_VALUE: u32 = 0x0044_ddc0;

/// `PACKAGE_PROCEDURE_TYPE` getter (+0x18).
const GET_PROCEDURE_TYPE: u32 = 0x0096_11e0;
/// A `float` getter at +0x0c of the target (`PackageTarget::fAcquireRadius`
/// is at +0x0c in the Xbox PDB), returned in `ST0`.
const GET_TARGET_ACQUIRE_RADIUS: u32 = 0x0084_d030;
/// Other functions of this unit, called by address because a later
/// session translates them.
const FN_00676280: u32 = 0x0067_6280;
const FN_00673180: u32 = 0x0067_3180;
const FN_00673890: u32 = 0x0067_3890;

/// Package types (`cPackType` values) the code compares with. The exe
/// does not name them in the code translated here, so they are numbers
/// except the two that select the dialogue data and the one that selects
/// the use-weapon data.
const TYPE_DIALOGUE: i32 = 0xf;
/// The second type that shares the dialogue-data object.
const TYPE_DIALOGUE_SECOND: i32 = 0x1c;
const TYPE_USE_WEAPON: i32 = 0x10;
/// "No type yet" (the reset stores 0xff).
const TYPE_NONE: u8 = 0xff;

/// Size and constructor of the per-procedure data objects.
const DIALOGUE_DATA_SIZE: u32 = 0x20;
const DIALOGUE_DATA_CONSTRUCTOR: u32 = 0x0067_b170;
const USE_WEAPON_DATA_SIZE: u32 = 0x24;
const USE_WEAPON_DATA_CONSTRUCTOR: u32 = 0x0067_caa0;

/// `TESDialoguePackageData` (Xbox PDB), 0x20 bytes: the offsets read here.
const DIALOGUE_FOV: u32 = 0x04;
const DIALOGUE_TOPIC: u32 = 0x08;
const DIALOGUE_TARGET_LOCATION: u32 = 0x0c;
const DIALOGUE_NO_HEADTRACKING: u32 = 0x10;
const DIALOGUE_DO_NOT_CONTROL_TARGET: u32 = 0x11;
const DIALOGUE_SAY_TO: u32 = 0x18;
/// `TESUseWeaponPackageData::pTargetLocation` (Xbox PDB).
const USE_WEAPON_TARGET_LOCATION: u32 = 0x08;
/// The other procedure data objects keep their second location at +4.
const SECOND_LOCATION_AT_4: u32 = 0x04;

/// `PACKAGE_DATA::iPackFlags` bits (roles unconfirmed except the two named).
const FLAG_BIT_40: u32 = 0x40;
const FLAG_BIT_10: u32 = 0x10;
const FLAG_BIT_80: u32 = 0x80;
const FLAG_BIT_20: u32 = 0x20;
const FLAG_BIT_100: u32 = 0x100;
const FLAG_ONCE_PER_DAY: u32 = 0x400;
const FLAG_CREATED: u32 = 0x800;
const FLAG_BIT_800000: u32 = 0x80_0000;
/// Set by `InitItem` when loading the package produced warnings (the
/// message says the package is disabled in game).
const FLAG_DISABLED: u32 = 0x8000;

layout! {
    /// `TESPackage` (Xbox PDB), 0x80 bytes on PC (0x90 on the Xbox). The
    /// `PACKAGE_DATA` member is flattened in at its PC offsets.
    pub struct TESPackage: 0x80 {
        /// `ePROCEDURE_TYPE` (Xbox PDB, `PACKAGE_PROCEDURE_TYPE`); -1 until
        /// the package is set up.
        0x18 ePROCEDURE_TYPE: i32,
        /// `PackData.iPackFlags` (Xbox PDB).
        0x1C iPackFlags: u32,
        /// `PackData.cPackType` (Xbox PDB): the package type.
        0x20 cPackType: i8,
        /// `PackData.iFOBehaviorFlags` (Xbox PDB).
        0x22 iFOBehaviorFlags: u16,
        /// `PackData.iPackageSpecificFlags` (Xbox PDB).
        0x24 iPackageSpecificFlags: u16,
        /// `pPackData` (Xbox PDB): the `TESPackageData` subclass object of
        /// the package's type.
        0x28 pPackData: Ptr,
        /// `pPackLoc` (Xbox PDB): `PackageLocation*`.
        0x2C pPackLoc: Ptr,
        /// `pPackTarg` (Xbox PDB): `PackageTarget*`.
        0x30 pPackTarg: Ptr,
        /// `pIdleCollection` (Xbox PDB): `BGSIdleCollection*`.
        0x34 pIdleCollection: Ptr,
        /// `packSched` (Xbox PDB): a `PackageSchedule`, 8 bytes.
        0x38 packSched: u32,
        /// `packConditions` (Xbox PDB): a `TESCondition`.
        0x40 packConditions: u32,
        /// `pCombatStyle` (Xbox PDB): `TESCombatStyle*`.
        0x48 pCombatStyle: Ptr,
        /// `OnBegin` (Xbox PDB): a `PackageEventAction`, 0x10 bytes.
        0x4C OnBegin: u32,
        /// `OnEnd` (Xbox PDB).
        0x5C OnEnd: u32,
        /// `OnChange` (Xbox PDB).
        0x6C OnChange: u32,
        /// `uiRefCount` (Xbox PDB).
        0x7C uiRefCount: u32,
    }
}

/// A `PackageLocation*`, `PackageTarget*` or package-data pointer: the
/// classes belong to other units, so the pointer is untyped here.
type Object = Ptr;

/// The package type, as the sign-extended byte the getter returns.
fn pack_type(e: &mut Engine, this: Ptr<TESPackage>) -> i32 {
    e.call(GET_PACK_TYPE, &args![this]).i32()
}

/// The package's `pPackLoc` through its getter.
fn location_of(e: &mut Engine, this: Ptr<TESPackage>) -> Object {
    e.call(GET_LOCATION, &args![this]).ptr()
}

/// Allocates `size` bytes and runs `constructor` on them, as the compiler
/// wrote `new T(...)`: no constructor call when the allocation is null.
/// `extra` are the constructor's stack arguments.
fn construct(e: &mut Engine, size: u32, constructor: u32, extra: &[u32]) -> Object {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        return Ptr::NULL;
    }
    let mut words = vec![block];
    words.extend_from_slice(extra);
    e.call(constructor, &words).ptr()
}

fn new_location(e: &mut Engine) -> Object {
    construct(e, LOCATION_SIZE, LOCATION_CONSTRUCTOR, &[])
}

fn new_target(e: &mut Engine) -> Object {
    construct(e, TARGET_SIZE, TARGET_CONSTRUCTOR, &[])
}

/// Gives the package a location object if it has none (the block the type
/// setup repeats).
fn ensure_location(e: &mut Engine, this: Ptr<TESPackage>) {
    if location_of(e, this).is_null() {
        let made = new_location(e);
        e.set(this, TESPackage::pPackLoc, made);
    }
}

/// The same for the target.
fn ensure_target(e: &mut Engine, this: Ptr<TESPackage>) {
    if fn_00671d10(e, this).is_null() {
        let made = new_target(e);
        e.set(this, TESPackage::pPackTarg, made);
    }
}

/// Builds a per-procedure data object and stores it at +0x28.
fn build_data(e: &mut Engine, this: Ptr<TESPackage>, size: u32, constructor: u32) {
    let made = construct(e, size, constructor, &[]);
    e.set(this, TESPackage::pPackData, made);
}

/// Copies `source` into the location held in the slot at `slot` (a field
/// of the package or of its data object), creating the object when the slot
/// is empty; a null `source` deletes the object and clears the slot.
fn replace_location(e: &mut Engine, slot: u32, source: Object) {
    if source.is_null() {
        let old = e.mem.u32(slot);
        if old != 0 {
            e.call(LOCATION_DELETE, &args![old, 1u32]);
        }
        e.mem.set_u32(slot, 0);
    } else {
        if e.mem.u32(slot) == 0 {
            let made = new_location(e);
            e.mem.set_u32(slot, made.addr());
        }
        let held = e.mem.u32(slot);
        e.call(LOCATION_COPY, &args![held, source, 0u32]);
    }
}

/// Whether the package type is one of the two that carry the dialogue
/// data object. The second test runs only when the first failed, as in
/// the code.
fn is_dialogue_type(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    pack_type(e, this) == TYPE_DIALOGUE || pack_type(e, this) == TYPE_DIALOGUE_SECOND
}

/// The dialogue data object (null when the package has none yet), or
/// `None` when the type has no dialogue data.
fn dialogue_data(e: &mut Engine, this: Ptr<TESPackage>) -> Option<u32> {
    if !is_dialogue_type(e, this) {
        return None;
    }
    Some(e.get(this, TESPackage::pPackData).addr())
}

/// The dialogue data object of a dialogue-type package, built when missing;
/// `None` for another type (the setters then do nothing).
fn dialogue_data_or_create(e: &mut Engine, this: Ptr<TESPackage>) -> Option<u32> {
    if !is_dialogue_type(e, this) {
        return None;
    }
    if e.get(this, TESPackage::pPackData).is_null() {
        build_data(e, this, DIALOGUE_DATA_SIZE, DIALOGUE_DATA_CONSTRUCTOR);
    }
    Some(e.get(this, TESPackage::pPackData).addr())
}

// Translated from 006707c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::TESPackage` (Xbox PDB): constructs the form base, installs
/// the vtable, constructs the schedule, conditions and the three event
/// actions, sets the form type 0x49 and resets the package data.
pub fn tes_package_tes_package(e: &mut Engine, this: Ptr<TESPackage>) -> Ptr<TESPackage> {
    e.call(TES_FORM_CONSTRUCTOR, &args![this]);
    e.mem.set_u32(this.addr(), TES_PACKAGE_VTABLE);
    e.call(
        PACKAGE_SCHEDULE_CONSTRUCTOR,
        &args![this.addr() + TESPackage::packSched.off],
    );
    e.call(
        TES_CONDITION_CONSTRUCTOR,
        &args![this.addr() + TESPackage::packConditions.off],
    );
    for action in [TESPackage::OnBegin, TESPackage::OnEnd, TESPackage::OnChange] {
        e.call(
            PACKAGE_EVENT_ACTION_CONSTRUCTOR,
            &args![this.addr() + action.off],
        );
    }
    e.call(SET_FORM_TYPE, &args![this, 0x49u32]);
    fn_006709d0(e, this);
    e.set(this, TESPackage::uiRefCount, 0);
    this
}

// Translated from 006709d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Resets the package data: flags, behaviour flags, the object pointers,
/// the package type (0xff) and the procedure type (-1), and clears the
/// "created" flag.
pub fn fn_006709d0(e: &mut Engine, this: Ptr<TESPackage>) {
    e.set(this, TESPackage::iPackFlags, 0);
    e.set(this, TESPackage::iPackageSpecificFlags, 0);
    e.set(this, TESPackage::iFOBehaviorFlags, 0);
    e.set(this, TESPackage::pPackLoc, Ptr::NULL);
    e.set(this, TESPackage::pPackTarg, Ptr::NULL);
    e.set(this, TESPackage::pIdleCollection, Ptr::NULL);
    e.set(this, TESPackage::pCombatStyle, Ptr::NULL);
    e.set(this, TESPackage::pPackData, Ptr::NULL);
    e.call(FN_00673180, &args![this, 0u32]);
    e.set(this, TESPackage::ePROCEDURE_TYPE, -1);
    e.set(this, TESPackage::cPackType, TYPE_NONE as i8);
    tes_package_set_is_created(e, this, 0);
}

// Translated from 00670b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::CreatePackage` (Xbox PDB), a cdecl static: builds the
/// package object for a package type and sets its type. The size and
/// constructor depend on the type (the switch is a jump table over
/// `type + 1`): 0xf and 0x1c, 0x12, 0x15, 0x16, 0x17, 0x18 and 0x27 build
/// one of the subclasses; -1, 9, 0x11 and 0x28 build nothing (0x11 is
/// mapped to 0 first, but there is still no object); every other value
/// builds a plain `TESPackage`. Returns null when nothing was built.
pub fn tes_package_create_package(e: &mut Engine, package_type: i32) -> Ptr<TESPackage> {
    let mut package_type = package_type;
    let package: Object = match package_type {
        -1 | 9 | 0x28 => Ptr::NULL,
        0x11 => {
            package_type = 0;
            Ptr::NULL
        }
        0xf | 0x1c => construct(e, 0xd0, 0x009e_dbc0, &[]),
        0x12 => construct(e, 0x188, 0x0097_d3a0, &[0, 0, 0]),
        0x15 => construct(e, 0x88, 0x009e_c5d0, &[]),
        0x16 => construct(e, 0xac, 0x009f_0e00, &[0, 0, 0]),
        0x17 => construct(e, 0x9c, 0x009f_91b0, &[]),
        0x18 => construct(e, 0xb4, 0x009f_72f0, &[]),
        0x27 => construct(e, 0x8c, 0x009e_d030, &[]),
        _ => construct(e, 0x80, 0x0067_07c0, &[]),
    };
    let package: Ptr<TESPackage> = package.cast();
    if !package.is_null() {
        tes_package_set_pack_type(e, package, package_type);
    }
    package
}

// Translated from 00670ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests bit 0x40 of `iPackFlags`.
pub fn fn_00670ed0(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    e.get(this, TESPackage::iPackFlags) & FLAG_BIT_40 != 0
}

// Translated from 00670ef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests bit 0x10 of `iPackFlags`.
pub fn fn_00670ef0(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    e.get(this, TESPackage::iPackFlags) & FLAG_BIT_10 != 0
}

// Translated from 00670f10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests bit 0x80 of `iPackFlags`.
pub fn fn_00670f10(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    e.get(this, TESPackage::iPackFlags) & FLAG_BIT_80 != 0
}

// Translated from 00670f40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests bit 0x20 of `iPackFlags`.
pub fn fn_00670f40(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    e.get(this, TESPackage::iPackFlags) & FLAG_BIT_20 != 0
}

// Translated from 00670f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests bit 0x100 of `iPackFlags`.
pub fn fn_00670f60(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    e.get(this, TESPackage::iPackFlags) & FLAG_BIT_100 != 0
}

// Translated from 00670f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetOncePerDay` (Xbox PDB): bit 0x400 of `iPackFlags`.
pub fn tes_package_get_once_per_day(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    e.get(this, TESPackage::iPackFlags) & FLAG_ONCE_PER_DAY != 0
}

// Translated from 00670fc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::SetPackType` (Xbox PDB): changes the package type. A
/// change deletes the data object of the old type, clears the objects the
/// new type does not use, and builds the location, target and data objects
/// the new type needs (two jump tables over the type). Type 0x11 is mapped
/// to 0. For type 0xC the specific-flags bits 0x01 to 0x20 are set to the
/// "off" state and bit 0x40 cleared; for type 9 a missing second location
/// is created.
pub fn tes_package_set_pack_type(e: &mut Engine, this: Ptr<TESPackage>, new_type: i32) {
    let mut new_type = new_type;
    if new_type == 0x11 {
        new_type = 0;
    }
    if new_type == pack_type(e, this) {
        return;
    }
    // Delete the old data object (scalar deleting destructor, flag 1).
    let old_data = e.get(this, TESPackage::pPackData);
    if !old_data.is_null() {
        e.vcall(old_data.addr(), SLOT_DELETE, &args![1u32]);
    }
    e.set(this, TESPackage::pPackData, Ptr::NULL);

    // First table: clear what the new type does not keep.
    match new_type {
        0 | 1 | 2 | 8 | 9 | 10 | 11 | 14 | 16 => {
            tes_package_set_package_second_location(e, this, Ptr::NULL)
        }
        4..=6 | 13 => tes_package_set_package_target(e, this, Ptr::NULL),
        7 => tes_package_set_package_location(e, this, Ptr::NULL),
        12 => {
            tes_package_set_package_target(e, this, Ptr::NULL);
            tes_package_set_package_second_location(e, this, Ptr::NULL);
        }
        _ => {}
    }

    // Second table: build what the new type needs.
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
            let target = fn_00671d10(e, this);
            if e.call(TARGET_GET_TYPE, &args![target]).i32() == 0 {
                e.call(TARGET_SET_TYPE, &args![target, 2u32]);
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
        0xf | 0x1c => {
            ensure_target(e, this);
            build_data(e, this, DIALOGUE_DATA_SIZE, DIALOGUE_DATA_CONSTRUCTOR);
        }
        0x10 => {
            fn_00671a20(e, this, 1);
            ensure_target(e, this);
            build_data(e, this, USE_WEAPON_DATA_SIZE, USE_WEAPON_DATA_CONSTRUCTOR);
        }
        _ => {}
    }

    e.set(this, TESPackage::iPackageSpecificFlags, 0);
    if new_type == 0xc {
        fn_00671ad0(e, this, 1);
        fn_00671b30(e, this, 1);
        fn_00671b90(e, this, 1);
        fn_00671bf0(e, this, 1);
        fn_00671c50(e, this, 1);
        fn_00671cb0(e, this, 1);
        fn_00671a70(e, this, 0);
    }
    e.set(this, TESPackage::cPackType, new_type as i8);
    if new_type == 9 && fn_00672dd0(e, this).is_null() {
        let location = new_location(e);
        tes_package_set_package_second_location(e, this, location);
        if !location.is_null() {
            e.call(LOCATION_DELETE, &args![location, 1u32]);
        }
    }
}

// Translated from 00671a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag != 0`) or clears bit 0x800000 of `iPackFlags`.
pub fn fn_00671a20(e: &mut Engine, this: Ptr<TESPackage>, flag: u8) {
    let flags = e.get(this, TESPackage::iPackFlags);
    let flags = if flag == 0 {
        flags & !FLAG_BIT_800000
    } else {
        flags | FLAG_BIT_800000
    };
    e.set(this, TESPackage::iPackFlags, flags);
}

/// Sets or clears `bit` of `iPackageSpecificFlags`; `set` says which.
fn specific_flag(e: &mut Engine, this: Ptr<TESPackage>, bit: u16, set: bool) {
    let flags = e.get(this, TESPackage::iPackageSpecificFlags);
    let flags = if set { flags | bit } else { flags & !bit };
    e.set(this, TESPackage::iPackageSpecificFlags, flags);
}

// Translated from 00671a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`flag != 0`) or clears bit 0x40 of `iPackageSpecificFlags`.
pub fn fn_00671a70(e: &mut Engine, this: Ptr<TESPackage>, flag: u8) {
    specific_flag(e, this, 0x40, flag != 0);
}

// Translated from 00671ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears (`flag != 0`) or sets (`flag == 0`) bit 0x01 of
/// `iPackageSpecificFlags`: the flag is stored inverted.
pub fn fn_00671ad0(e: &mut Engine, this: Ptr<TESPackage>, flag: u8) {
    specific_flag(e, this, 0x01, flag == 0);
}

// Translated from 00671b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Same for bit 0x02 (stored inverted).
pub fn fn_00671b30(e: &mut Engine, this: Ptr<TESPackage>, flag: u8) {
    specific_flag(e, this, 0x02, flag == 0);
}

// Translated from 00671b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Same for bit 0x04 (stored inverted).
pub fn fn_00671b90(e: &mut Engine, this: Ptr<TESPackage>, flag: u8) {
    specific_flag(e, this, 0x04, flag == 0);
}

// Translated from 00671bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Same for bit 0x08 (stored inverted).
pub fn fn_00671bf0(e: &mut Engine, this: Ptr<TESPackage>, flag: u8) {
    specific_flag(e, this, 0x08, flag == 0);
}

// Translated from 00671c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Same for bit 0x10 (stored inverted).
pub fn fn_00671c50(e: &mut Engine, this: Ptr<TESPackage>, flag: u8) {
    specific_flag(e, this, 0x10, flag == 0);
}

// Translated from 00671cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Same for bit 0x20 (stored inverted).
pub fn fn_00671cb0(e: &mut Engine, this: Ptr<TESPackage>, flag: u8) {
    specific_flag(e, this, 0x20, flag == 0);
}

// Translated from 00671d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The package's `pPackTarg` (`PackageTarget*`, +0x30).
pub fn fn_00671d10(e: &mut Engine, this: Ptr<TESPackage>) -> Object {
    e.get(this, TESPackage::pPackTarg)
}

// Translated from 00671d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::SetPackageLocation` (Xbox PDB): copies `location` into the
/// package's own `PackageLocation` (creating it when missing); a null
/// `location` deletes it.
pub fn tes_package_set_package_location(e: &mut Engine, this: Ptr<TESPackage>, location: Object) {
    replace_location(e, this.addr() + TESPackage::pPackLoc.off, location);
}

// Translated from 00671e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::SetPackageSecondLocation` (Xbox PDB): the second location
/// lives in the data object of the package's type, whose layout depends on
/// the type: types 0xF and 0x1C keep it at +0xC of the 0x20-byte dialogue
/// data, type 0x10 at +8 of the use-weapon data (built on demand), types
/// 3, 2, 9, 1 and 8 at +4 of their small data objects (built here when
/// missing, with their sizes and constructors). Other types do nothing.
/// `location` is copied into it; null deletes it.
pub fn tes_package_set_package_second_location(
    e: &mut Engine,
    this: Ptr<TESPackage>,
    location: Object,
) {
    if is_dialogue_type(e, this) {
        if e.get(this, TESPackage::pPackData).is_null() {
            build_data(e, this, DIALOGUE_DATA_SIZE, DIALOGUE_DATA_CONSTRUCTOR);
        }
        let data = e.get(this, TESPackage::pPackData).addr();
        replace_location(e, data + DIALOGUE_TARGET_LOCATION, location);
        return;
    }
    // (type, data size, data constructor), each keeping the location at +4.
    const SMALL: [(i32, u32, u32); 5] = [
        (3, 8, 0x0067_b950),
        (2, 0xc, 0x0067_bbf0),
        (9, 8, 0x0067_acf0),
        (1, 0xc, 0x0067_bfd0),
        (8, 8, 0x0067_c770),
    ];
    for (kind, size, constructor) in SMALL {
        if pack_type(e, this) == kind {
            if e.get(this, TESPackage::pPackData).is_null() {
                build_data(e, this, size, constructor);
            }
            let data = e.get(this, TESPackage::pPackData).addr();
            replace_location(e, data + SECOND_LOCATION_AT_4, location);
            return;
        }
    }
    if pack_type(e, this) == TYPE_USE_WEAPON {
        let data = tes_package_get_or_create_use_weapon_package_data(e, this);
        replace_location(e, data.addr() + USE_WEAPON_TARGET_LOCATION, location);
    }
}

// Translated from 00672710 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetDialogueSayToFlag` (Xbox PDB): `bSayTo` of the dialogue
/// data, or 0.
pub fn tes_package_get_dialogue_say_to_flag(e: &mut Engine, this: Ptr<TESPackage>) -> u8 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => e.mem.u8(data + DIALOGUE_SAY_TO),
        _ => 0,
    }
}

// Translated from 00672760 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetDialogueTopic` (Xbox PDB): `ptopic` of the dialogue
/// data (`TESTopic*`), or null.
pub fn tes_package_get_dialogue_topic(e: &mut Engine, this: Ptr<TESPackage>) -> Object {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => Ptr::new(e.mem.u32(data + DIALOGUE_TOPIC)),
        _ => Ptr::NULL,
    }
}

// Translated from 006727b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetDialogueDoNotControlTarget` (Xbox PDB):
/// `bDoNotControlTarget`, or 0.
pub fn tes_package_get_dialogue_do_not_control_target(e: &mut Engine, this: Ptr<TESPackage>) -> u8 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => e.mem.u8(data + DIALOGUE_DO_NOT_CONTROL_TARGET),
        _ => 0,
    }
}

// Translated from 00672800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetDialogueNoHeadtrack` (Xbox PDB): `bNOHeadtracking`, or 0.
pub fn tes_package_get_dialogue_no_headtrack(e: &mut Engine, this: Ptr<TESPackage>) -> u8 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => e.mem.u8(data + DIALOGUE_NO_HEADTRACKING),
        _ => 0,
    }
}

// Translated from 00672850 (decompiled, FalloutNV.exe 1.4.0.525)
/// `fFov` of the dialogue data (1.0 when the package has none).
pub fn fn_00672850(e: &mut Engine, this: Ptr<TESPackage>) -> f32 {
    match dialogue_data(e, this) {
        Some(data) if data != 0 => e.mem.f32(data + DIALOGUE_FOV),
        _ => 1.0,
    }
}

// Translated from 006728a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetAcquireRadius` (Xbox PDB): writes the radius at `radius`
/// (0 first) and says whether the package has one. Type 0xC takes it from
/// `00676280(argument)`, converted from an unsigned integer; otherwise it
/// is the target's float (`0084d030`) when the package has a target and
/// the value is not below or equal to zero.
pub fn tes_package_get_acquire_radius(
    e: &mut Engine,
    this: Ptr<TESPackage>,
    argument: u32,
    radius: Ptr,
) -> bool {
    e.mem.set_f32(radius.addr(), 0.0);
    if pack_type(e, this) == 0xc {
        let value = e.call(FN_00676280, &args![this, argument]).u32();
        e.mem.set_f32(radius.addr(), value as f64 as f32);
        return true;
    }
    let target = fn_00671d10(e, this);
    if target.is_null() {
        return false;
    }
    let value = e.call(GET_TARGET_ACQUIRE_RADIUS, &args![target]).f32();
    let zero: f64 = e.global(ZERO);
    // `FCOMP` then `TEST AH,0x41; JP`: the radius is written when the
    // value is greater than zero or unordered.
    if (value as f64) <= zero {
        return false;
    }
    e.mem.set_f32(radius.addr(), value);
    true
}

// Translated from 00672930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetInitialTargetCount` (Xbox PDB): 0 without a target; 1
/// when the target's type is 0 or 3; otherwise the target's `iValue` when it
/// is positive; otherwise from the procedure type: 1 for 0x1c, 0x7fffffff
/// for 2, 3 and 0x1a, else 0.
pub fn tes_package_get_initial_target_count(e: &mut Engine, this: Ptr<TESPackage>) -> i32 {
    let target = fn_00671d10(e, this);
    if target.is_null() {
        return 0;
    }
    if e.call(TARGET_GET_TYPE, &args![target]).i32() == 0
        || e.call(TARGET_GET_TYPE, &args![target]).i32() == 3
    {
        return 1;
    }
    let value = e.call(TARGET_GET_VALUE, &args![target]).i32();
    if value > 0 {
        return value;
    }
    if e.call(GET_PROCEDURE_TYPE, &args![this]).i32() == 0x1c {
        return 1;
    }
    if e.call(GET_PROCEDURE_TYPE, &args![this]).i32() == 2
        || e.call(GET_PROCEDURE_TYPE, &args![this]).i32() == 3
        || e.call(GET_PROCEDURE_TYPE, &args![this]).i32() == 0x1a
    {
        return 0x7fff_ffff;
    }
    0
}

// Translated from 006729d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `ptopic` of the dialogue data (built when missing); does nothing
/// for another package type.
pub fn fn_006729d0(e: &mut Engine, this: Ptr<TESPackage>, topic: u32) {
    if let Some(data) = dialogue_data_or_create(e, this) {
        e.mem.set_u32(data + DIALOGUE_TOPIC, topic);
    }
}

// Translated from 00672aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `bDoNotControlTarget` of the dialogue data (built when missing).
pub fn fn_00672aa0(e: &mut Engine, this: Ptr<TESPackage>, value: u8) {
    if let Some(data) = dialogue_data_or_create(e, this) {
        e.mem.set_u8(data + DIALOGUE_DO_NOT_CONTROL_TARGET, value);
    }
}

// Translated from 00672b70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `bNOHeadtracking` of the dialogue data (built when missing).
pub fn fn_00672b70(e: &mut Engine, this: Ptr<TESPackage>, value: u8) {
    if let Some(data) = dialogue_data_or_create(e, this) {
        e.mem.set_u8(data + DIALOGUE_NO_HEADTRACKING, value);
    }
}

// Translated from 00672c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `fFov` (+4) of the dialogue data (built when missing).
pub fn fn_00672c40(e: &mut Engine, this: Ptr<TESPackage>, value: f32) {
    if let Some(data) = dialogue_data_or_create(e, this) {
        e.mem.set_f32(data + DIALOGUE_FOV, value);
    }
}

// Translated from 00672d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetOrCreateUseWeaponPackageData` (Xbox PDB): asserts the
/// type is 0x10 (the assert does not stop the function), builds the
/// `TESUseWeaponPackageData` (0x24 bytes) when missing, and returns the
/// data object.
pub fn tes_package_get_or_create_use_weapon_package_data(
    e: &mut Engine,
    this: Ptr<TESPackage>,
) -> Object {
    if pack_type(e, this) != TYPE_USE_WEAPON {
        e.call(ASSERT, &args![PACKAGE_SOURCE_FILE, 0x607u32]);
    }
    if e.get(this, TESPackage::pPackData).is_null() {
        build_data(e, this, USE_WEAPON_DATA_SIZE, USE_WEAPON_DATA_CONSTRUCTOR);
    }
    e.get(this, TESPackage::pPackData)
}

// Translated from 00672dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The package's second location (`PackageLocation*`), read from the data
/// object of its type: +0xC for types 0xF and 0x1C, +4 for types 3, 2, 9,
/// 8 and 1, +8 for type 0x10; null when there is no data object or another
/// type.
pub fn fn_00672dd0(e: &mut Engine, this: Ptr<TESPackage>) -> Object {
    let has_data = |e: &Engine| !e.get(this, TESPackage::pPackData).is_null();
    if has_data(e) && is_dialogue_type(e, this) {
        let data = e.get(this, TESPackage::pPackData).addr();
        return Ptr::new(e.mem.u32(data + DIALOGUE_TARGET_LOCATION));
    }
    for kind in [3, 2, 9, 8, 1] {
        if has_data(e) && pack_type(e, this) == kind {
            let data = e.get(this, TESPackage::pPackData).addr();
            return Ptr::new(e.mem.u32(data + SECOND_LOCATION_AT_4));
        }
    }
    if has_data(e) && pack_type(e, this) == TYPE_USE_WEAPON {
        let data = e.get(this, TESPackage::pPackData).addr();
        return Ptr::new(e.mem.u32(data + USE_WEAPON_TARGET_LOCATION));
    }
    Ptr::NULL
}

// Translated from 00672f20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetPackageSearchLocation` (Xbox PDB): the package's own
/// location for types 0, 0xC, 0x11 and 0x1C; for types 2, 3, 8 and 0x10 the
/// second location, replaced by the package's own location when the second
/// one exists and has location type 7 (`GetLocType`); null otherwise.
pub fn tes_package_get_package_search_location(e: &mut Engine, this: Ptr<TESPackage>) -> Object {
    match pack_type(e, this) {
        0 | 0xc | 0x11 | 0x1c => location_of(e, this),
        2 | 3 | 8 | 0x10 => {
            let second = fn_00672dd0(e, this);
            if !second.is_null()
                && e.call(LOCATION_GET_TYPE, &args![second]).u32() == LOCATION_TYPE_SEVEN
            {
                location_of(e, this)
            } else {
                fn_00672dd0(e, this)
            }
        }
        _ => Ptr::NULL,
    }
}

// Translated from 00672fc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::SetPackageTarget` (Xbox PDB): copies `target` into the
/// package's own `PackageTarget` (creating it when missing); a null
/// `target` deletes it.
pub fn tes_package_set_package_target(e: &mut Engine, this: Ptr<TESPackage>, target: Object) {
    if target.is_null() {
        let old = e.get(this, TESPackage::pPackTarg);
        if !old.is_null() {
            e.call(TARGET_DELETE, &args![old, 1u32]);
        }
        e.set(this, TESPackage::pPackTarg, Ptr::NULL);
    } else {
        ensure_target(e, this);
        let held = e.get(this, TESPackage::pPackTarg);
        e.call(TARGET_COPY, &args![held, target]);
    }
}

// Translated from 00674930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::InitItem` (Xbox PDB): resolves the package's references
/// after loading, once (it returns at once when form flag 0x8 is set).
/// Initialises the location, second location, target, conditions, idle
/// collection and the data object; resolves the combat style form ID
/// through `AddCompileIndex` and a dynamic cast (logging when it cannot be
/// found); initialises the three event actions; marks the package disabled
/// when the loader's warning count changed; then applies the per-type fix
/// ups (types 9, 0xC, 0xD, 0xE) and sets form flag 0x8.
pub fn tes_package_init_item(e: &mut Engine, this: Ptr<TESPackage>) {
    if e.call(GET_FORM_FLAG_8, &args![this]).bool() {
        return;
    }
    let warnings_before = e.call(GET_WARNING_COUNT, &[]).u32();

    if !location_of(e, this).is_null() {
        let location = location_of(e, this);
        e.call(LOCATION_INIT_ITEM, &args![location, this]);
    }
    if !fn_00672dd0(e, this).is_null() {
        let second = fn_00672dd0(e, this);
        e.call(LOCATION_INIT_ITEM, &args![second, this]);
    }
    if !fn_00671d10(e, this).is_null() {
        let target = fn_00671d10(e, this);
        e.call(TARGET_INIT_ITEM, &args![target, this]);
    }
    if e.call(GET_CONDITIONS, &args![this]).u32() != 0 {
        let conditions = e.call(GET_CONDITIONS, &args![this]).u32();
        e.call(TES_CONDITION_INIT_ITEM, &args![conditions, this]);
    }
    if e.call(GET_IDLE_COLLECTION, &args![this]).u32() != 0 {
        let idle = e.call(GET_IDLE_COLLECTION, &args![this]).u32();
        e.call(BGS_IDLE_COLLECTION_INIT_ITEM, &args![idle, this]);
    }
    let data = e.get(this, TESPackage::pPackData);
    if !data.is_null() {
        e.vcall(data.addr(), SLOT_DATA_INIT_ITEM, &args![this]);
    }

    // The combat style is held as a form ID until now.
    let combat_style_id = e.call(GET_COMBAT_STYLE, &args![this]).u32();
    if combat_style_id != 0 {
        let (resolved_id, style) = e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), combat_style_id);
            let file = e.call(TES_FORM_GET_FILE, &args![this, -1i32]).u32();
            e.call(TES_FORM_ADD_COMPILE_INDEX, &args![slot, file]);
            let resolved_id = e.mem.u32(slot.addr());
            let form = e.call(LOOKUP_FORM, &args![resolved_id]).u32();
            let style = e
                .call(
                    RT_DYNAMIC_CAST,
                    &args![
                        form,
                        0u32,
                        TYPE_DESCRIPTOR_TES_FORM,
                        TYPE_DESCRIPTOR_COMBAT_STYLE,
                        0u32
                    ],
                )
                .u32();
            (resolved_id, style)
        });
        e.call(SET_COMBAT_STYLE, &args![this, style]);
        if e.call(GET_COMBAT_STYLE, &args![this]).u32() == 0 {
            let id = e.call(GET_FORM_ID, &args![this]).u32();
            let name = e.vcall(this.addr(), SLOT_FORM_NAME, &[]).u32();
            e.call(
                LOG_FORM_WARNING,
                &args![MESSAGE_COMBAT_STYLE_MISSING, resolved_id, name, id],
            );
        }
    }

    for (action, kind) in [
        (TESPackage::OnBegin, 0u32),
        (TESPackage::OnEnd, 1),
        (TESPackage::OnChange, 2),
    ] {
        e.call(
            PACKAGE_EVENT_ACTION_SET_KIND,
            &args![this.addr() + action.off, kind],
        );
    }
    for action in [TESPackage::OnBegin, TESPackage::OnEnd, TESPackage::OnChange] {
        e.call(
            PACKAGE_EVENT_ACTION_INIT_ITEM,
            &args![this.addr() + action.off, this],
        );
    }

    if e.call(GET_WARNING_COUNT, &[]).u32() != warnings_before {
        let id = e.call(GET_FORM_ID, &args![this]).u32();
        let name = e.vcall(this.addr(), SLOT_FORM_NAME, &[]).u32();
        e.call(
            LOG_FORM_WARNING,
            &args![MESSAGE_WARNINGS_GENERATED, name, id],
        );
        let flags = e.get(this, TESPackage::iPackFlags);
        e.set(this, TESPackage::iPackFlags, flags | FLAG_DISABLED);
    }

    // Per-type fix-ups of the location and target.
    match pack_type(e, this) {
        9 => {
            if fn_00674d20(e, this) {
                let location = location_of(e, this);
                e.call(LOCATION_SET_TYPE, &args![location, LOCATION_TYPE_SIX]);
            }
        }
        0xc => {
            if e.call(FN_00673890, &args![this]).bool() {
                let location = location_of(e, this);
                e.call(LOCATION_SET_TYPE, &args![location, LOCATION_TYPE_SIX]);
            }
        }
        0xd => {
            // Byte at +5 of the data object (`pPackData`).
            let data = e.get(this, TESPackage::pPackData).addr();
            if e.mem.u8(data + 5) != 0 {
                let location = location_of(e, this);
                e.call(LOCATION_SET_TYPE, &args![location, LOCATION_TYPE_SIX]);
            }
            if location_of(e, this).is_null() {
                let made = new_location(e);
                e.call(LOCATION_SET_TYPE, &args![made, LOCATION_TYPE_SIX]);
                tes_package_set_package_location(e, this, made);
                if !made.is_null() {
                    e.call(LOCATION_DELETE, &args![made, 1u32]);
                }
            }
        }
        0xe if fn_00674d20(e, this) => {
            if fn_00671d10(e, this).is_null() {
                let made = new_target(e);
                tes_package_set_package_target(e, this, made);
            }
            let target = fn_00671d10(e, this);
            e.call(TARGET_SET_TYPE, &args![target, 3u32]);
        }
        _ => {}
    }
    e.call(SET_FORM_FLAG_8, &args![this, 1u32]);
}

// Translated from 00674d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests bit 0x02 of `iPackageSpecificFlags`.
pub fn fn_00674d20(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    e.get(this, TESPackage::iPackageSpecificFlags) & 2 != 0
}

// Translated from 00674d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::GetIsCreated` (Xbox PDB): bit 0x800 of `iPackFlags`.
pub fn tes_package_get_is_created(e: &mut Engine, this: Ptr<TESPackage>) -> bool {
    e.get(this, TESPackage::iPackFlags) & FLAG_CREATED != 0
}

// Translated from 00674d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESPackage::SetIsCreated` (Xbox PDB): sets or clears bit 0x800 of
/// `iPackFlags`, but only for a package whose form ID is a dynamic one
/// (the data handler's test, `formID >= 0xFF000000`).
pub fn tes_package_set_is_created(e: &mut Engine, this: Ptr<TESPackage>, created: u8) {
    let form_id = e.call(GET_FORM_ID, &args![this]).u32();
    let handler: u32 = e.global(DATA_HANDLER);
    if !e.call(IS_DYNAMIC_FORM_ID, &args![handler, form_id]).bool() {
        return;
    }
    let flags = e.get(this, TESPackage::iPackFlags);
    let flags = if created == 0 {
        flags & !FLAG_CREATED
    } else {
        flags | FLAG_CREATED
    };
    e.set(this, TESPackage::iPackFlags, flags);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x006707c0,
            tes_package_tes_package(Ptr<TESPackage>) -> Ptr<TESPackage>
        ),
        entry!(0x006709d0, fn_006709d0(Ptr<TESPackage>)),
        entry!(0x00670b90, tes_package_create_package(i32) -> Ptr<TESPackage>),
        entry!(0x00670ed0, fn_00670ed0(Ptr<TESPackage>) -> bool),
        entry!(0x00670ef0, fn_00670ef0(Ptr<TESPackage>) -> bool),
        entry!(0x00670f10, fn_00670f10(Ptr<TESPackage>) -> bool),
        entry!(0x00670f40, fn_00670f40(Ptr<TESPackage>) -> bool),
        entry!(0x00670f60, fn_00670f60(Ptr<TESPackage>) -> bool),
        entry!(
            0x00670f90,
            tes_package_get_once_per_day(Ptr<TESPackage>) -> bool
        ),
        entry!(0x00670fc0, tes_package_set_pack_type(Ptr<TESPackage>, i32)),
        entry!(0x00671a20, fn_00671a20(Ptr<TESPackage>, u8)),
        entry!(0x00671a70, fn_00671a70(Ptr<TESPackage>, u8)),
        entry!(0x00671ad0, fn_00671ad0(Ptr<TESPackage>, u8)),
        entry!(0x00671b30, fn_00671b30(Ptr<TESPackage>, u8)),
        entry!(0x00671b90, fn_00671b90(Ptr<TESPackage>, u8)),
        entry!(0x00671bf0, fn_00671bf0(Ptr<TESPackage>, u8)),
        entry!(0x00671c50, fn_00671c50(Ptr<TESPackage>, u8)),
        entry!(0x00671cb0, fn_00671cb0(Ptr<TESPackage>, u8)),
        entry!(0x00671d10, fn_00671d10(Ptr<TESPackage>) -> Ptr),
        entry!(
            0x00671d30,
            tes_package_set_package_location(Ptr<TESPackage>, Ptr)
        ),
        entry!(
            0x00671e10,
            tes_package_set_package_second_location(Ptr<TESPackage>, Ptr)
        ),
        entry!(
            0x00672710,
            tes_package_get_dialogue_say_to_flag(Ptr<TESPackage>) -> u8
        ),
        entry!(
            0x00672760,
            tes_package_get_dialogue_topic(Ptr<TESPackage>) -> Ptr
        ),
        entry!(
            0x006727b0,
            tes_package_get_dialogue_do_not_control_target(Ptr<TESPackage>) -> u8
        ),
        entry!(
            0x00672800,
            tes_package_get_dialogue_no_headtrack(Ptr<TESPackage>) -> u8
        ),
        entry!(0x00672850, fn_00672850(Ptr<TESPackage>) -> f32),
        entry!(
            0x006728a0,
            tes_package_get_acquire_radius(Ptr<TESPackage>, u32, Ptr) -> bool
        ),
        entry!(
            0x00672930,
            tes_package_get_initial_target_count(Ptr<TESPackage>) -> i32
        ),
        entry!(0x006729d0, fn_006729d0(Ptr<TESPackage>, u32)),
        entry!(0x00672aa0, fn_00672aa0(Ptr<TESPackage>, u8)),
        entry!(0x00672b70, fn_00672b70(Ptr<TESPackage>, u8)),
        entry!(0x00672c40, fn_00672c40(Ptr<TESPackage>, f32)),
        entry!(
            0x00672d10,
            tes_package_get_or_create_use_weapon_package_data(Ptr<TESPackage>) -> Ptr
        ),
        entry!(0x00672dd0, fn_00672dd0(Ptr<TESPackage>) -> Ptr),
        entry!(
            0x00672f20,
            tes_package_get_package_search_location(Ptr<TESPackage>) -> Ptr
        ),
        entry!(
            0x00672fc0,
            tes_package_set_package_target(Ptr<TESPackage>, Ptr)
        ),
        entry!(0x00674930, tes_package_init_item(Ptr<TESPackage>)),
        entry!(0x00674d20, fn_00674d20(Ptr<TESPackage>) -> bool),
        entry!(
            0x00674d40,
            tes_package_get_is_created(Ptr<TESPackage>) -> bool
        ),
        entry!(0x00674d70, tes_package_set_is_created(Ptr<TESPackage>, u8)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    /// Vtable of the fake package-data objects: slot 0 (`FAKE_DELETE`) and
    /// slot 0x10 (`FAKE_INIT_ITEM`).
    const DATA_VTABLE: u32 = 0x0130_0000;
    const FAKE_DELETE: u32 = 0x7000_0000;
    const FAKE_INIT_ITEM: u32 = 0x7000_0010;
    /// Vtable of the test package: slot 0x130 gives the form name.
    const PACKAGE_VTABLE: u32 = 0x0130_1000;
    const FAKE_NAME: u32 = 0x7000_0130;
    const NAME_POINTER: u32 = 0x0aa0_0001;

    /// Constructors and the small data objects' constructors: they return
    /// their `this`.
    const CONSTRUCTORS: [u32; 19] = [
        LOCATION_CONSTRUCTOR,
        TARGET_CONSTRUCTOR,
        DIALOGUE_DATA_CONSTRUCTOR,
        USE_WEAPON_DATA_CONSTRUCTOR,
        0x0067_bfd0,
        0x0067_bbf0,
        0x0067_b950,
        0x0067_acf0,
        0x0067_c770,
        0x0067_c3f0,
        0x009e_dbc0,
        0x0097_d3a0,
        0x009e_c5d0,
        0x009f_0e00,
        0x009f_91b0,
        0x009f_72f0,
        0x009e_d030,
        TES_FORM_CONSTRUCTOR,
        PACKAGE_SCHEDULE_CONSTRUCTOR,
    ];

    /// Callees that do nothing visible to the tests (the log shows the call).
    const QUIET: [u32; 25] = [
        TES_CONDITION_CONSTRUCTOR,
        PACKAGE_EVENT_ACTION_CONSTRUCTOR,
        SET_FORM_TYPE,
        FN_00673180,
        LOCATION_DELETE,
        LOCATION_COPY,
        LOCATION_INIT_ITEM,
        LOCATION_SET_TYPE,
        TARGET_DELETE,
        TARGET_COPY,
        TARGET_INIT_ITEM,
        TARGET_SET_TYPE,
        TES_CONDITION_INIT_ITEM,
        BGS_IDLE_COLLECTION_INIT_ITEM,
        PACKAGE_EVENT_ACTION_SET_KIND,
        PACKAGE_EVENT_ACTION_INIT_ITEM,
        SET_FORM_FLAG_8,
        LOG_FORM_WARNING,
        ASSERT,
        IS_DYNAMIC_FORM_ID,
        GET_FORM_FLAG_8,
        GET_WARNING_COUNT,
        FAKE_DELETE,
        FAKE_INIT_ITEM,
        FN_00676280,
    ];

    /// An engine whose accessor callees behave as the exe's one-line
    /// accessors do (they read the field the code says), whose constructors
    /// return `this`, and whose other callees do nothing.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x0101_2000, 0x1000);
        e.map(0x011c_3000, 0x1000);
        for address in CONSTRUCTORS {
            e.register(address, |_, a| a[0].into_ret());
        }
        for address in QUIET {
            e.register(address, |_, _| Ret::default());
        }
        e.register(OPERATOR_NEW, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(GET_PACK_TYPE, |e, a| {
            (e.mem.i8(a[0] + 0x20) as i32).into_ret()
        });
        e.register(GET_LOCATION, |e, a| e.mem.u32(a[0] + 0x2c).into_ret());
        e.register(GET_FORM_ID, |e, a| e.mem.u32(a[0] + 0x0c).into_ret());
        e.register(GET_COMBAT_STYLE, |e, a| e.mem.u32(a[0] + 0x48).into_ret());
        e.register(SET_COMBAT_STYLE, |e, a| {
            e.mem.set_u32(a[0] + 0x48, a[1]);
            Ret::default()
        });
        e.register(GET_IDLE_COLLECTION, |e, a| {
            e.mem.u32(a[0] + 0x34).into_ret()
        });
        e.register(GET_CONDITIONS, |_, a| (a[0] + 0x40).into_ret());
        e.register(GET_PROCEDURE_TYPE, |e, a| e.mem.u32(a[0] + 0x18).into_ret());
        e.register(TARGET_GET_TYPE, |e, a| e.mem.u8(a[0]).into_ret());
        e.register(TARGET_GET_VALUE, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(LOCATION_GET_TYPE, |e, a| (e.mem.i8(a[0]) as i32).into_ret());
        e.register(FN_00673890, |e, a| {
            (e.mem.u16(a[0] + 0x24) & 0x40 != 0).into_ret()
        });
        e.register(GET_TARGET_ACQUIRE_RADIUS, |e, a| {
            e.mem.f32(a[0] + 0x0c).into_ret()
        });
        e.register(FAKE_NAME, |_, _| NAME_POINTER.into_ret());
        e.put_vtable(DATA_VTABLE, &[FAKE_DELETE, 0, 0, 0, FAKE_INIT_ITEM]);
        let mut slots = vec![0; 0x130 / 4 + 1];
        slots[0x130 / 4] = FAKE_NAME;
        e.put_vtable(PACKAGE_VTABLE, &slots);
        e
    }

    /// A zeroed package of the given type (as the getter sees it).
    fn package(e: &mut Engine, kind: i8) -> Ptr<TESPackage> {
        let p = e.new_object::<TESPackage>();
        e.set(p, TESPackage::cPackType, kind);
        e.mem.set_u32(p.addr(), PACKAGE_VTABLE);
        p
    }

    /// A data object with the fake vtable.
    fn data_object(e: &mut Engine, size: u32) -> u32 {
        let d = e.mem.alloc(size);
        e.mem.set_u32(d, DATA_VTABLE);
        d
    }

    fn log_on(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The argument lists of every call to `address` since `log_on`.
    fn calls_to(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The addresses called, in order, without the call under test and the
    /// type getter.
    fn sequence(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .skip(1)
            .map(|(a, _)| *a)
            .filter(|a| *a != GET_PACK_TYPE)
            .collect()
    }

    #[test]
    fn constructor_builds_base_and_members_then_resets() {
        let mut e = engine();
        let p = e.new_object::<TESPackage>();
        e.set(p, TESPackage::uiRefCount, 7);
        log_on(&mut e);
        assert_eq!(e.call(0x006707c0, &args![p]).ptr::<TESPackage>(), p);
        assert_eq!(e.mem.u32(p.addr()), TES_PACKAGE_VTABLE);
        assert_eq!(
            sequence(&e),
            [
                TES_FORM_CONSTRUCTOR,
                PACKAGE_SCHEDULE_CONSTRUCTOR,
                TES_CONDITION_CONSTRUCTOR,
                PACKAGE_EVENT_ACTION_CONSTRUCTOR,
                PACKAGE_EVENT_ACTION_CONSTRUCTOR,
                PACKAGE_EVENT_ACTION_CONSTRUCTOR,
                SET_FORM_TYPE,
                FN_00673180,
                GET_FORM_ID,
                IS_DYNAMIC_FORM_ID,
            ]
        );
        assert_eq!(
            calls_to(&e, PACKAGE_SCHEDULE_CONSTRUCTOR),
            [[p.addr() + 0x38]]
        );
        assert_eq!(calls_to(&e, TES_CONDITION_CONSTRUCTOR), [[p.addr() + 0x40]]);
        assert_eq!(
            calls_to(&e, PACKAGE_EVENT_ACTION_CONSTRUCTOR),
            [[p.addr() + 0x4c], [p.addr() + 0x5c], [p.addr() + 0x6c]]
        );
        assert_eq!(calls_to(&e, SET_FORM_TYPE), [[p.addr(), 0x49]]);
        assert_eq!(e.get(p, TESPackage::uiRefCount), 0);
        assert_eq!(e.get(p, TESPackage::cPackType), -1);
    }

    #[test]
    fn reset_clears_fields_and_sets_none_types() {
        let mut e = engine();
        let p = package(&mut e, 5);
        e.set(p, TESPackage::iPackFlags, 0xffff_ffff);
        e.set(p, TESPackage::iPackageSpecificFlags, 0xff);
        e.set(p, TESPackage::iFOBehaviorFlags, 0xff);
        for field in [
            TESPackage::pPackData,
            TESPackage::pPackLoc,
            TESPackage::pPackTarg,
            TESPackage::pIdleCollection,
            TESPackage::pCombatStyle,
        ] {
            e.set(p, field, Ptr::new(0x1234));
        }
        log_on(&mut e);
        e.call(0x006709d0, &args![p]);
        assert_eq!(e.get(p, TESPackage::iPackFlags), 0);
        assert_eq!(e.get(p, TESPackage::iPackageSpecificFlags), 0);
        assert_eq!(e.get(p, TESPackage::iFOBehaviorFlags), 0);
        assert!(e.get(p, TESPackage::pPackData).is_null());
        assert!(e.get(p, TESPackage::pPackLoc).is_null());
        assert!(e.get(p, TESPackage::pPackTarg).is_null());
        assert!(e.get(p, TESPackage::pIdleCollection).is_null());
        assert!(e.get(p, TESPackage::pCombatStyle).is_null());
        assert_eq!(e.get(p, TESPackage::ePROCEDURE_TYPE), -1);
        assert_eq!(e.get(p, TESPackage::cPackType), -1);
        assert_eq!(calls_to(&e, FN_00673180), [[p.addr(), 0]]);
    }

    #[test]
    fn create_package_builds_by_type() {
        let mut e = engine();
        // Types that build nothing.
        for kind in [-1, 9, 0x11, 0x28] {
            log_on(&mut e);
            assert!(e
                .call(0x00670b90, &args![kind])
                .ptr::<TESPackage>()
                .is_null());
            assert!(calls_to(&e, OPERATOR_NEW).is_empty(), "type {kind}");
        }
        // A subclass: size 0xd0, its constructor, then the type is set.
        log_on(&mut e);
        let p = e.call(0x00670b90, &args![0xfi32]).ptr::<TESPackage>();
        assert_eq!(calls_to(&e, OPERATOR_NEW)[0], [0xd0]);
        assert_eq!(calls_to(&e, 0x009e_dbc0), [[p.addr()]]);
        assert_eq!(e.get(p, TESPackage::cPackType), 0xf);
        // The constructors with three stack arguments.
        log_on(&mut e);
        let p = e.call(0x00670b90, &args![0x12i32]).ptr::<TESPackage>();
        assert_eq!(calls_to(&e, OPERATOR_NEW)[0], [0x188]);
        assert_eq!(calls_to(&e, 0x0097_d3a0), [[p.addr(), 0, 0, 0]]);
        log_on(&mut e);
        e.call(0x00670b90, &args![0x16i32]);
        assert_eq!(calls_to(&e, OPERATOR_NEW)[0], [0xac]);
        // The other subclass sizes.
        for (kind, size, ctor) in [
            (0x15, 0x88, 0x009e_c5d0),
            (0x17, 0x9c, 0x009f_91b0),
            (0x18, 0xb4, 0x009f_72f0),
            (0x27, 0x8c, 0x009e_d030),
            (0x1c, 0xd0, 0x009e_dbc0),
        ] {
            log_on(&mut e);
            e.call(0x00670b90, &args![kind]);
            assert_eq!(calls_to(&e, OPERATOR_NEW)[0], [size], "type {kind}");
            assert_eq!(calls_to(&e, ctor).len(), 1, "type {kind}");
        }
        // Any other type: a plain package (0x80 bytes, its own constructor).
        log_on(&mut e);
        let p = e.call(0x00670b90, &args![5i32]).ptr::<TESPackage>();
        assert_eq!(calls_to(&e, OPERATOR_NEW)[0], [0x80]);
        assert_eq!(e.mem.u32(p.addr()), TES_PACKAGE_VTABLE);
        assert_eq!(e.get(p, TESPackage::cPackType), 5);
        // Out of range types are plain packages too.
        log_on(&mut e);
        e.call(0x00670b90, &args![0x30i32]);
        assert_eq!(calls_to(&e, OPERATOR_NEW)[0], [0x80]);
        // A failed allocation builds nothing and calls no constructor.
        e.register(OPERATOR_NEW, |_, _| Ret::default());
        log_on(&mut e);
        assert!(e
            .call(0x00670b90, &args![5i32])
            .ptr::<TESPackage>()
            .is_null());
        assert!(calls_to(&e, 0x0067_07c0).is_empty());
    }

    /// A flag getter test: false without the bit, true with it, other bits
    /// ignored.
    fn check_flag_getter(address: u32, bit: u32) {
        let mut e = engine();
        let p = package(&mut e, 0);
        e.set(p, TESPackage::iPackFlags, !bit);
        assert!(!e.call(address, &args![p]).bool());
        e.set(p, TESPackage::iPackFlags, bit);
        assert!(e.call(address, &args![p]).bool());
    }

    #[test]
    fn flag_getter_00670ed0() {
        check_flag_getter(0x00670ed0, 0x40);
    }

    #[test]
    fn flag_getter_00670ef0() {
        check_flag_getter(0x00670ef0, 0x10);
    }

    #[test]
    fn flag_getter_00670f10() {
        check_flag_getter(0x00670f10, 0x80);
    }

    #[test]
    fn flag_getter_00670f40() {
        check_flag_getter(0x00670f40, 0x20);
    }

    #[test]
    fn flag_getter_00670f60() {
        check_flag_getter(0x00670f60, 0x100);
    }

    #[test]
    fn get_once_per_day_tests_bit_0x400() {
        check_flag_getter(0x00670f90, 0x400);
    }

    #[test]
    fn set_pack_type_same_type_does_nothing() {
        let mut e = engine();
        let p = package(&mut e, 3);
        let data = data_object(&mut e, 8);
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        log_on(&mut e);
        e.call(0x00670fc0, &args![p, 3i32]);
        assert!(sequence(&e).is_empty());
        assert_eq!(e.get(p, TESPackage::pPackData).addr(), data);
        // Type 0x11 counts as 0.
        let q = package(&mut e, 0);
        log_on(&mut e);
        e.call(0x00670fc0, &args![q, 0x11i32]);
        assert!(sequence(&e).is_empty());
    }

    #[test]
    fn set_pack_type_builds_for_type_3_and_deletes_old_data() {
        let mut e = engine();
        let p = package(&mut e, -1);
        let old = data_object(&mut e, 8);
        e.set(p, TESPackage::pPackData, Ptr::new(old));
        e.set(p, TESPackage::iPackageSpecificFlags, 0x55);
        log_on(&mut e);
        e.call(0x00670fc0, &args![p, 3i32]);
        // The old data object is deleted through its vtable, flag 1.
        assert_eq!(calls_to(&e, FAKE_DELETE), [[old, 1]]);
        // A target (type 0 so far -> set to 2), a location and the 8 byte data.
        let target = e.get(p, TESPackage::pPackTarg);
        assert!(!target.is_null());
        assert_eq!(calls_to(&e, TARGET_SET_TYPE), [[target.addr(), 2]]);
        assert!(!e.get(p, TESPackage::pPackLoc).is_null());
        let data = e.get(p, TESPackage::pPackData);
        assert!(!data.is_null() && data.addr() != old);
        assert_eq!(e.mem.block_size(data.addr()), Some(8));
        assert_eq!(calls_to(&e, 0x0067_b950), [[data.addr()]]);
        assert_eq!(e.get(p, TESPackage::cPackType), 3);
        assert_eq!(e.get(p, TESPackage::iPackageSpecificFlags), 0);
    }

    #[test]
    fn set_pack_type_clears_by_old_table_and_builds_other_types() {
        let mut e = engine();
        // Type 12: target removed; the specific flags are reset; location
        // created.
        let p = package(&mut e, -1);
        let target = e.mem.alloc(0x10);
        e.set(p, TESPackage::pPackTarg, Ptr::new(target));
        e.set(p, TESPackage::iPackageSpecificFlags, 0x7f);
        log_on(&mut e);
        e.call(0x00670fc0, &args![p, 12i32]);
        assert_eq!(calls_to(&e, TARGET_DELETE), [[target, 1]]);
        assert!(e.get(p, TESPackage::pPackTarg).is_null());
        assert!(!e.get(p, TESPackage::pPackLoc).is_null());
        assert_eq!(e.get(p, TESPackage::iPackageSpecificFlags), 0);
        assert!(e.get(p, TESPackage::pPackData).is_null());
        // Type 7: its location removed, target created.
        let q = package(&mut e, -1);
        let location = e.mem.alloc(0xc);
        e.set(q, TESPackage::pPackLoc, Ptr::new(location));
        log_on(&mut e);
        e.call(0x00670fc0, &args![q, 7i32]);
        assert_eq!(calls_to(&e, LOCATION_DELETE), [[location, 1]]);
        assert!(e.get(q, TESPackage::pPackLoc).is_null());
        assert!(!e.get(q, TESPackage::pPackTarg).is_null());
        // Type 0x10: bit 0x800000 set, 0x24 byte data, target.
        let r = package(&mut e, -1);
        e.call(0x00670fc0, &args![r, 0x10i32]);
        assert_eq!(e.get(r, TESPackage::iPackFlags) & 0x80_0000, 0x80_0000);
        assert_eq!(
            e.mem.block_size(e.get(r, TESPackage::pPackData).addr()),
            Some(0x28)
        );
        assert!(!e.get(r, TESPackage::pPackTarg).is_null());
        // Type 8: 8 byte data, location, target; type 13: only the data.
        let s = package(&mut e, -1);
        e.call(0x00670fc0, &args![s, 8i32]);
        assert!(!e.get(s, TESPackage::pPackLoc).is_null());
        assert!(!e.get(s, TESPackage::pPackTarg).is_null());
        assert!(!e.get(s, TESPackage::pPackData).is_null());
        let t = package(&mut e, -1);
        e.call(0x00670fc0, &args![t, 13i32]);
        assert!(e.get(t, TESPackage::pPackLoc).is_null());
        assert!(!e.get(t, TESPackage::pPackData).is_null());
        // Dialogue types: target and 0x20 byte data.
        let u = package(&mut e, -1);
        log_on(&mut e);
        e.call(0x00670fc0, &args![u, 0x1ci32]);
        assert_eq!(calls_to(&e, OPERATOR_NEW), [[0x10], [0x20]]);
        assert_eq!(e.get(u, TESPackage::cPackType), 0x1c);
    }

    #[test]
    fn set_pack_type_9_creates_the_second_location() {
        let mut e = engine();
        let p = package(&mut e, -1);
        log_on(&mut e);
        e.call(0x00670fc0, &args![p, 9i32]);
        let data = e.get(p, TESPackage::pPackData).addr();
        assert_ne!(data, 0);
        // The temporary location is copied into a location of its own and
        // then deleted.
        let held = e.mem.u32(data + 4);
        assert_ne!(held, 0);
        let copies = calls_to(&e, LOCATION_COPY);
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0][0], held);
        let deletes = calls_to(&e, LOCATION_DELETE);
        assert_eq!(deletes, [[copies[0][1], 1]]);
        assert_ne!(copies[0][1], held);
        assert!(!e.get(p, TESPackage::pPackLoc).is_null());
    }

    #[test]
    fn specific_flag_setters() {
        // (address, bit, stored inverted)
        for (address, bit, inverted) in [
            (0x00671a70u32, 0x40u16, false),
            (0x00671ad0, 0x01, true),
            (0x00671b30, 0x02, true),
            (0x00671b90, 0x04, true),
            (0x00671bf0, 0x08, true),
            (0x00671c50, 0x10, true),
            (0x00671cb0, 0x20, true),
        ] {
            let mut e = engine();
            let p = package(&mut e, 0);
            e.set(p, TESPackage::iPackageSpecificFlags, 0x0100);
            e.call(address, &args![p, 1u32]);
            let on = e.get(p, TESPackage::iPackageSpecificFlags);
            e.call(address, &args![p, 0u32]);
            let off = e.get(p, TESPackage::iPackageSpecificFlags);
            let (set, clear) = if inverted { (off, on) } else { (on, off) };
            assert_eq!(set, 0x0100 | bit, "{address:08x}");
            assert_eq!(clear, 0x0100, "{address:08x}");
        }
    }

    #[test]
    fn flag_setter_00671a20_sets_bit_0x800000() {
        let mut e = engine();
        let p = package(&mut e, 0);
        e.set(p, TESPackage::iPackFlags, 1);
        e.call(0x00671a20, &args![p, 1u32]);
        assert_eq!(e.get(p, TESPackage::iPackFlags), 0x80_0001);
        e.call(0x00671a20, &args![p, 0u32]);
        assert_eq!(e.get(p, TESPackage::iPackFlags), 1);
    }

    #[test]
    fn target_getter_00671d10() {
        let mut e = engine();
        let p = package(&mut e, 0);
        assert!(e.call(0x00671d10, &args![p]).ptr::<()>().is_null());
        e.set(p, TESPackage::pPackTarg, Ptr::new(0x4321));
        assert_eq!(e.call(0x00671d10, &args![p]).u32(), 0x4321);
    }

    #[test]
    fn set_package_location_creates_copies_and_deletes() {
        let mut e = engine();
        let p = package(&mut e, 0);
        let source = e.mem.alloc(0xc);
        log_on(&mut e);
        e.call(0x00671d30, &args![p, source]);
        let made = e.get(p, TESPackage::pPackLoc).addr();
        assert_ne!(made, 0);
        assert_eq!(calls_to(&e, LOCATION_COPY), [[made, source, 0]]);
        // A second call reuses the object.
        log_on(&mut e);
        e.call(0x00671d30, &args![p, source]);
        assert_eq!(e.get(p, TESPackage::pPackLoc).addr(), made);
        assert!(calls_to(&e, OPERATOR_NEW).is_empty());
        // Null deletes it and clears the field.
        log_on(&mut e);
        e.call(0x00671d30, &args![p, 0u32]);
        assert_eq!(calls_to(&e, LOCATION_DELETE), [[made, 1]]);
        assert!(e.get(p, TESPackage::pPackLoc).is_null());
        // Null on an empty package deletes nothing.
        log_on(&mut e);
        e.call(0x00671d30, &args![p, 0u32]);
        assert!(calls_to(&e, LOCATION_DELETE).is_empty());
    }

    #[test]
    fn set_second_location_by_type() {
        let mut e = engine();
        let source = e.mem.alloc(0xc);
        // Dialogue types: the 0x20 byte data object is built; slot +0xc.
        let p = package(&mut e, 0xf);
        log_on(&mut e);
        e.call(0x00671e10, &args![p, source]);
        let data = e.get(p, TESPackage::pPackData).addr();
        assert_eq!(e.mem.block_size(data), Some(0x20));
        let made = e.mem.u32(data + 0xc);
        assert_ne!(made, 0);
        assert_eq!(calls_to(&e, LOCATION_COPY), [[made, source, 0]]);
        // The other dialogue type reuses the object.
        let p = package(&mut e, 0x1c);
        let data = e.mem.alloc(0x20);
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        let held = e.mem.alloc(0xc);
        e.mem.set_u32(data + 0xc, held);
        log_on(&mut e);
        e.call(0x00671e10, &args![p, source]);
        assert_eq!(calls_to(&e, LOCATION_COPY), [[held, source, 0]]);
        // Small data objects keep it at +4: size and constructor by type.
        for (kind, size, ctor) in [
            (3, 8, 0x0067_b950),
            (2, 0xc, 0x0067_bbf0),
            (9, 8, 0x0067_acf0),
            (1, 0xc, 0x0067_bfd0),
            (8, 8, 0x0067_c770),
        ] {
            let p = package(&mut e, kind);
            log_on(&mut e);
            e.call(0x00671e10, &args![p, source]);
            let data = e.get(p, TESPackage::pPackData).addr();
            assert_eq!(
                e.mem.block_size(data),
                Some(u32::div_ceil(size, 8) * 8),
                "type {kind}"
            );
            assert_eq!(calls_to(&e, ctor), [[data]], "type {kind}");
            assert_ne!(e.mem.u32(data + 4), 0, "type {kind}");
        }
        // Use weapon: slot +8 of the 0x24 byte data object.
        let p = package(&mut e, 0x10);
        e.call(0x00671e10, &args![p, source]);
        let data = e.get(p, TESPackage::pPackData).addr();
        assert_eq!(e.mem.block_size(data), Some(0x28));
        assert_ne!(e.mem.u32(data + 8), 0);
        // A null source deletes the held location.
        let p = package(&mut e, 3);
        let data = e.mem.alloc(8);
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        let held = e.mem.alloc(0xc);
        e.mem.set_u32(data + 4, held);
        log_on(&mut e);
        e.call(0x00671e10, &args![p, 0u32]);
        assert_eq!(calls_to(&e, LOCATION_DELETE), [[held, 1]]);
        assert_eq!(e.mem.u32(data + 4), 0);
        // Another type does nothing.
        let p = package(&mut e, 5);
        log_on(&mut e);
        e.call(0x00671e10, &args![p, source]);
        assert!(sequence(&e).is_empty());
        assert!(e.get(p, TESPackage::pPackData).is_null());
    }

    /// A package of `kind` with a dialogue data object holding known values.
    fn dialogue_package(e: &mut Engine, kind: i8) -> (Ptr<TESPackage>, u32) {
        let p = package(e, kind);
        let data = e.mem.alloc(0x20);
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        e.mem.set_f32(data + 4, 2.5);
        e.mem.set_u32(data + 8, 0x7007);
        e.mem.set_u8(data + 0x10, 3);
        e.mem.set_u8(data + 0x11, 4);
        e.mem.set_u8(data + 0x18, 5);
        (p, data)
    }

    #[test]
    fn dialogue_say_to_flag() {
        let mut e = engine();
        for kind in [0xf, 0x1c] {
            let (p, _) = dialogue_package(&mut e, kind);
            assert_eq!(e.call(0x00672710, &args![p]).u8(), 5);
        }
        let (p, _) = dialogue_package(&mut e, 5);
        assert_eq!(e.call(0x00672710, &args![p]).u8(), 0);
        let p = package(&mut e, 0xf);
        assert_eq!(e.call(0x00672710, &args![p]).u8(), 0);
    }

    #[test]
    fn dialogue_topic() {
        let mut e = engine();
        let (p, _) = dialogue_package(&mut e, 0xf);
        assert_eq!(e.call(0x00672760, &args![p]).u32(), 0x7007);
        let (p, _) = dialogue_package(&mut e, 0x1c);
        assert_eq!(e.call(0x00672760, &args![p]).u32(), 0x7007);
        let (p, _) = dialogue_package(&mut e, 4);
        assert_eq!(e.call(0x00672760, &args![p]).u32(), 0);
        let p = package(&mut e, 0xf);
        assert_eq!(e.call(0x00672760, &args![p]).u32(), 0);
    }

    #[test]
    fn dialogue_do_not_control_target() {
        let mut e = engine();
        let (p, _) = dialogue_package(&mut e, 0xf);
        assert_eq!(e.call(0x006727b0, &args![p]).u8(), 4);
        let (p, _) = dialogue_package(&mut e, 0x1c);
        assert_eq!(e.call(0x006727b0, &args![p]).u8(), 4);
        let (p, _) = dialogue_package(&mut e, 4);
        assert_eq!(e.call(0x006727b0, &args![p]).u8(), 0);
    }

    #[test]
    fn dialogue_no_headtrack() {
        let mut e = engine();
        let (p, _) = dialogue_package(&mut e, 0xf);
        assert_eq!(e.call(0x00672800, &args![p]).u8(), 3);
        let (p, _) = dialogue_package(&mut e, 4);
        assert_eq!(e.call(0x00672800, &args![p]).u8(), 0);
        let p = package(&mut e, 0xf);
        assert_eq!(e.call(0x00672800, &args![p]).u8(), 0);
    }

    #[test]
    fn dialogue_fov_defaults_to_one() {
        let mut e = engine();
        let (p, _) = dialogue_package(&mut e, 0x1c);
        assert_eq!(e.call(0x00672850, &args![p]).f32(), 2.5);
        let (p, _) = dialogue_package(&mut e, 4);
        assert_eq!(e.call(0x00672850, &args![p]).f32(), 1.0);
        let p = package(&mut e, 0xf);
        assert_eq!(e.call(0x00672850, &args![p]).f32(), 1.0);
    }

    #[test]
    fn acquire_radius() {
        let mut e = engine();
        let out = e.mem.alloc(4);
        e.mem.set_f32(out, 9.0);
        // Type 0xC: from 00676280(argument), unsigned.
        e.register(FN_00676280, |_, a| (a[1] + 1).into_ret());
        let p = package(&mut e, 0xc);
        assert!(e.call(0x006728a0, &args![p, 0xffff_fffeu32, out]).bool());
        assert_eq!(e.mem.f32(out), 4_294_967_295.0f64 as f32);
        // No target: false, the radius left at 0.
        let p = package(&mut e, 4);
        e.mem.set_f32(out, 9.0);
        assert!(!e.call(0x006728a0, &args![p, 0u32, out]).bool());
        assert_eq!(e.mem.f32(out), 0.0);
        // A target with a positive radius.
        let target = e.mem.alloc(0x10);
        e.mem.set_f32(target + 0xc, 5.5);
        e.set(p, TESPackage::pPackTarg, Ptr::new(target));
        assert!(e.call(0x006728a0, &args![p, 0u32, out]).bool());
        assert_eq!(e.mem.f32(out), 5.5);
        // Zero and negative radii are rejected; NaN is accepted.
        for (value, accepted) in [(0.0f32, false), (-1.0, false), (f32::NAN, true)] {
            e.mem.set_f32(target + 0xc, value);
            e.mem.set_f32(out, 9.0);
            assert_eq!(e.call(0x006728a0, &args![p, 0u32, out]).bool(), accepted);
            if !accepted {
                assert_eq!(e.mem.f32(out), 0.0);
            }
        }
    }

    #[test]
    fn initial_target_count() {
        let mut e = engine();
        let p = package(&mut e, 4);
        assert_eq!(e.call(0x00672930, &args![p]).i32(), 0);
        let target = e.mem.alloc(0x10);
        e.set(p, TESPackage::pPackTarg, Ptr::new(target));
        // Target types 0 and 3: one.
        assert_eq!(e.call(0x00672930, &args![p]).i32(), 1);
        e.mem.set_u8(target, 3);
        assert_eq!(e.call(0x00672930, &args![p]).i32(), 1);
        // Another type: its value when positive.
        e.mem.set_u8(target, 1);
        e.mem.set_u32(target + 8, 6);
        assert_eq!(e.call(0x00672930, &args![p]).i32(), 6);
        // Otherwise by the procedure type.
        e.mem.set_u32(target + 8, 0);
        for (procedure, expected) in [
            (0x1c, 1),
            (2, 0x7fff_ffff),
            (3, 0x7fff_ffff),
            (0x1a, 0x7fff_ffff),
            (5, 0),
        ] {
            e.set(p, TESPackage::ePROCEDURE_TYPE, procedure);
            assert_eq!(e.call(0x00672930, &args![p]).i32(), expected);
        }
        // A negative value does not count.
        e.mem.set_u32(target + 8, 0xffff_ffff);
        e.set(p, TESPackage::ePROCEDURE_TYPE, 2);
        assert_eq!(e.call(0x00672930, &args![p]).i32(), 0x7fff_ffff);
    }

    #[test]
    fn dialogue_topic_setter() {
        let mut e = engine();
        // Built when missing.
        let p = package(&mut e, 0xf);
        e.call(0x006729d0, &args![p, 0x7007u32]);
        let data = e.get(p, TESPackage::pPackData).addr();
        assert_eq!(e.mem.block_size(data), Some(0x20));
        assert_eq!(e.mem.u32(data + 8), 0x7007);
        // Reused, and not for another type.
        e.call(0x006729d0, &args![p, 1u32]);
        assert_eq!(e.get(p, TESPackage::pPackData).addr(), data);
        assert_eq!(e.mem.u32(data + 8), 1);
        let q = package(&mut e, 4);
        e.call(0x006729d0, &args![q, 1u32]);
        assert!(e.get(q, TESPackage::pPackData).is_null());
    }

    #[test]
    fn dialogue_do_not_control_target_setter() {
        let mut e = engine();
        let p = package(&mut e, 0x1c);
        e.call(0x00672aa0, &args![p, 1u32]);
        let data = e.get(p, TESPackage::pPackData).addr();
        assert_eq!(e.mem.u8(data + 0x11), 1);
        let q = package(&mut e, 4);
        e.call(0x00672aa0, &args![q, 1u32]);
        assert!(e.get(q, TESPackage::pPackData).is_null());
    }

    #[test]
    fn dialogue_no_headtrack_setter() {
        let mut e = engine();
        let p = package(&mut e, 0xf);
        e.call(0x00672b70, &args![p, 1u32]);
        let data = e.get(p, TESPackage::pPackData).addr();
        assert_eq!(e.mem.u8(data + 0x10), 1);
        assert_eq!(e.mem.u8(data + 0x11), 0);
        let q = package(&mut e, 4);
        e.call(0x00672b70, &args![q, 1u32]);
        assert!(e.get(q, TESPackage::pPackData).is_null());
    }

    #[test]
    fn dialogue_fov_setter() {
        let mut e = engine();
        let p = package(&mut e, 0xf);
        e.call(0x00672c40, &args![p, 1.75f32]);
        let data = e.get(p, TESPackage::pPackData).addr();
        assert_eq!(e.mem.f32(data + 4), 1.75);
        let q = package(&mut e, 4);
        e.call(0x00672c40, &args![q, 1.75f32]);
        assert!(e.get(q, TESPackage::pPackData).is_null());
    }

    #[test]
    fn get_or_create_use_weapon_data() {
        let mut e = engine();
        let p = package(&mut e, 0x10);
        log_on(&mut e);
        let data = e.call(0x00672d10, &args![p]).u32();
        assert_eq!(e.mem.block_size(data), Some(0x28));
        assert_eq!(calls_to(&e, USE_WEAPON_DATA_CONSTRUCTOR), [[data]]);
        assert!(calls_to(&e, ASSERT).is_empty());
        // The second call returns the same object.
        assert_eq!(e.call(0x00672d10, &args![p]).u32(), data);
        // Another type asserts (line 0x607) but still builds.
        let q = package(&mut e, 4);
        log_on(&mut e);
        let data = e.call(0x00672d10, &args![q]).u32();
        assert_ne!(data, 0);
        assert_eq!(calls_to(&e, ASSERT), [[PACKAGE_SOURCE_FILE, 0x607]]);
    }

    #[test]
    fn second_location_getter() {
        let mut e = engine();
        // No data object: null.
        let p = package(&mut e, 3);
        assert_eq!(e.call(0x00672dd0, &args![p]).u32(), 0);
        for (kind, offset) in [
            (0xf, 0xc),
            (0x1c, 0xc),
            (3, 4),
            (2, 4),
            (9, 4),
            (8, 4),
            (1, 4),
            (0x10, 8),
        ] {
            let p = package(&mut e, kind);
            let data = e.mem.alloc(0x24);
            e.set(p, TESPackage::pPackData, Ptr::new(data));
            e.mem.set_u32(data + offset, 0x8800 + kind as u32);
            assert_eq!(e.call(0x00672dd0, &args![p]).u32(), 0x8800 + kind as u32);
        }
        // Another type with a data object: null.
        let p = package(&mut e, 5);
        let data = e.mem.alloc(0x24);
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        assert_eq!(e.call(0x00672dd0, &args![p]).u32(), 0);
    }

    #[test]
    fn package_search_location() {
        let mut e = engine();
        // Own location for types 0, 0xC, 0x11 and 0x1C.
        for kind in [0, 0xc, 0x11, 0x1c] {
            let p = package(&mut e, kind);
            e.set(p, TESPackage::pPackLoc, Ptr::new(0x1110));
            assert_eq!(e.call(0x00672f20, &args![p]).u32(), 0x1110, "type {kind}");
        }
        // Types 2, 3, 8 and 0x10: the second location, unless it is of
        // location type 7, when the own location replaces it.
        let second = e.mem.alloc(0xc);
        for kind in [2, 3, 8] {
            let p = package(&mut e, kind);
            let data = e.mem.alloc(0x24);
            e.set(p, TESPackage::pPackData, Ptr::new(data));
            e.mem.set_u32(data + 4, second);
            e.set(p, TESPackage::pPackLoc, Ptr::new(0x1110));
            e.mem.set_u8(second, 1);
            assert_eq!(e.call(0x00672f20, &args![p]).u32(), second, "type {kind}");
            e.mem.set_u8(second, 7);
            assert_eq!(e.call(0x00672f20, &args![p]).u32(), 0x1110, "type {kind}");
        }
        let p = package(&mut e, 0x10);
        let data = e.mem.alloc(0x24);
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        e.mem.set_u8(second, 2);
        e.mem.set_u32(data + 8, second);
        assert_eq!(e.call(0x00672f20, &args![p]).u32(), second);
        // No second location: null.
        let p = package(&mut e, 2);
        assert_eq!(e.call(0x00672f20, &args![p]).u32(), 0);
        // Other types: null.
        let p = package(&mut e, 1);
        e.set(p, TESPackage::pPackLoc, Ptr::new(0x1110));
        assert_eq!(e.call(0x00672f20, &args![p]).u32(), 0);
    }

    #[test]
    fn set_package_target_creates_copies_and_deletes() {
        let mut e = engine();
        let p = package(&mut e, 0);
        let source = e.mem.alloc(0x10);
        log_on(&mut e);
        e.call(0x00672fc0, &args![p, source]);
        let made = e.get(p, TESPackage::pPackTarg).addr();
        assert_ne!(made, 0);
        assert_eq!(calls_to(&e, TARGET_COPY), [[made, source]]);
        log_on(&mut e);
        e.call(0x00672fc0, &args![p, source]);
        assert!(calls_to(&e, OPERATOR_NEW).is_empty());
        log_on(&mut e);
        e.call(0x00672fc0, &args![p, 0u32]);
        assert_eq!(calls_to(&e, TARGET_DELETE), [[made, 1]]);
        assert!(e.get(p, TESPackage::pPackTarg).is_null());
        log_on(&mut e);
        e.call(0x00672fc0, &args![p, 0u32]);
        assert!(calls_to(&e, TARGET_DELETE).is_empty());
    }

    /// Registers the combat-style resolution doubles: the compile index
    /// turns the stored ID into `0x0100_0000 | id`, the lookup gives the
    /// form `form`, the cast `style`.
    fn resolve_combat_style(e: &mut Engine, form: u32, style: u32) {
        e.register(TES_FORM_GET_FILE, |_, _| 0xf11e.into_ret());
        e.register(TES_FORM_ADD_COMPILE_INDEX, |e, a| {
            let id = e.mem.u32(a[0]);
            e.mem.set_u32(a[0], 0x0100_0000 | id);
            Ret::default()
        });
        e.register_double(LOOKUP_FORM, move |_, _| form.into_ret());
        e.register_double(RT_DYNAMIC_CAST, move |_, _| style.into_ret());
    }

    #[test]
    fn init_item_does_nothing_when_flag_8_is_set() {
        let mut e = engine();
        e.register(GET_FORM_FLAG_8, |_, _| true.into_ret());
        let p = package(&mut e, 9);
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        assert_eq!(sequence(&e), [GET_FORM_FLAG_8]);
    }

    #[test]
    fn init_item_initialises_members_and_resolves_the_combat_style() {
        let mut e = engine();
        let p = package(&mut e, 4);
        let location = e.mem.alloc(0xc);
        let target = e.mem.alloc(0x10);
        let data = data_object(&mut e, 8);
        e.set(p, TESPackage::pPackLoc, Ptr::new(location));
        e.set(p, TESPackage::pPackTarg, Ptr::new(target));
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        e.set(p, TESPackage::pIdleCollection, Ptr::new(0x5550));
        e.set(p, TESPackage::pCombatStyle, Ptr::new(0x1234));
        e.mem.set_u32(p.addr() + 0x0c, 0xabcd);
        resolve_combat_style(&mut e, 0xf0, 0x5ea1);
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        assert_eq!(calls_to(&e, LOCATION_INIT_ITEM), [[location, p.addr()]]);
        assert_eq!(calls_to(&e, TARGET_INIT_ITEM), [[target, p.addr()]]);
        assert_eq!(
            calls_to(&e, TES_CONDITION_INIT_ITEM),
            [[p.addr() + 0x40, p.addr()]]
        );
        assert_eq!(
            calls_to(&e, BGS_IDLE_COLLECTION_INIT_ITEM),
            [[0x5550, p.addr()]]
        );
        assert_eq!(calls_to(&e, FAKE_INIT_ITEM), [[data, p.addr()]]);
        // The combat style: file of the package (-1), compile index of the
        // slot, lookup of the resolved ID, dynamic cast to the style class.
        assert_eq!(calls_to(&e, TES_FORM_GET_FILE), [[p.addr(), 0xffff_ffff]]);
        assert_eq!(calls_to(&e, LOOKUP_FORM), [[0x0100_1234]]);
        assert_eq!(
            calls_to(&e, RT_DYNAMIC_CAST),
            [[
                0xf0,
                0,
                TYPE_DESCRIPTOR_TES_FORM,
                TYPE_DESCRIPTOR_COMBAT_STYLE,
                0
            ]]
        );
        assert_eq!(e.get(p, TESPackage::pCombatStyle).addr(), 0x5ea1);
        assert!(calls_to(&e, LOG_FORM_WARNING).is_empty());
        // Event actions: kinds 0, 1, 2, then their InitItem with the package.
        assert_eq!(
            calls_to(&e, PACKAGE_EVENT_ACTION_SET_KIND),
            [
                [p.addr() + 0x4c, 0],
                [p.addr() + 0x5c, 1],
                [p.addr() + 0x6c, 2]
            ]
        );
        assert_eq!(
            calls_to(&e, PACKAGE_EVENT_ACTION_INIT_ITEM),
            [
                [p.addr() + 0x4c, p.addr()],
                [p.addr() + 0x5c, p.addr()],
                [p.addr() + 0x6c, p.addr()]
            ]
        );
        // Finally the form flag.
        assert_eq!(calls_to(&e, SET_FORM_FLAG_8), [[p.addr(), 1]]);
        assert_eq!(e.get(p, TESPackage::iPackFlags) & FLAG_DISABLED, 0);
    }

    #[test]
    fn init_item_logs_a_missing_combat_style_and_new_warnings() {
        let mut e = engine();
        let p = package(&mut e, 4);
        e.set(p, TESPackage::pCombatStyle, Ptr::new(0x1234));
        e.mem.set_u32(p.addr() + 0x0c, 0xabcd);
        resolve_combat_style(&mut e, 0xf0, 0);
        // The warning counter reads 0 first and 1 afterwards.
        let reads = Rc::new(Cell::new(0u32));
        let counter = reads.clone();
        e.register_double(GET_WARNING_COUNT, move |_, _| {
            let n = counter.get();
            counter.set(n + 1);
            u32::from(n >= 1).into_ret()
        });
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        assert!(e.get(p, TESPackage::pCombatStyle).is_null());
        assert_eq!(
            calls_to(&e, LOG_FORM_WARNING),
            vec![
                vec![
                    MESSAGE_COMBAT_STYLE_MISSING,
                    0x0100_1234,
                    NAME_POINTER,
                    0xabcd
                ],
                vec![MESSAGE_WARNINGS_GENERATED, NAME_POINTER, 0xabcd]
            ]
        );
        assert_eq!(
            e.get(p, TESPackage::iPackFlags) & FLAG_DISABLED,
            FLAG_DISABLED
        );
    }

    #[test]
    fn init_item_type_fixups() {
        // Type 9 and 0xC: the location type is set to 6 under the flag tests.
        let mut e = engine();
        let location = e.mem.alloc(0xc);
        let p = package(&mut e, 9);
        e.set(p, TESPackage::pPackLoc, Ptr::new(location));
        e.set(p, TESPackage::iPackageSpecificFlags, 2);
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        assert_eq!(calls_to(&e, LOCATION_SET_TYPE), [[location, 6]]);
        let p = package(&mut e, 9);
        e.set(p, TESPackage::pPackLoc, Ptr::new(location));
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        assert!(calls_to(&e, LOCATION_SET_TYPE).is_empty());
        let p = package(&mut e, 0xc);
        e.set(p, TESPackage::pPackLoc, Ptr::new(location));
        e.set(p, TESPackage::iPackageSpecificFlags, 0x40);
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        assert_eq!(calls_to(&e, LOCATION_SET_TYPE), [[location, 6]]);
        // Type 0xD without a location: one is made, typed 6, stored by copy
        // and the temporary deleted.
        let p = package(&mut e, 0xd);
        let data = data_object(&mut e, 8);
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        let made = calls_to(&e, LOCATION_SET_TYPE);
        assert_eq!(made.len(), 1);
        assert_eq!(made[0][1], 6);
        assert_eq!(calls_to(&e, LOCATION_COPY).len(), 1);
        assert_eq!(calls_to(&e, LOCATION_DELETE), [[made[0][0], 1]]);
        assert!(!e.get(p, TESPackage::pPackLoc).is_null());
        // Type 0xD with the data flag at +5 and a location: that one is typed.
        let p = package(&mut e, 0xd);
        let data = data_object(&mut e, 8);
        e.mem.set_u8(data + 5, 1);
        e.set(p, TESPackage::pPackData, Ptr::new(data));
        e.set(p, TESPackage::pPackLoc, Ptr::new(location));
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        assert_eq!(calls_to(&e, LOCATION_SET_TYPE), [[location, 6]]);
        // Type 0xE under flag 2 without a target: one is made and typed 3.
        let p = package(&mut e, 0xe);
        e.set(p, TESPackage::iPackageSpecificFlags, 2);
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        let target = e.get(p, TESPackage::pPackTarg).addr();
        assert_ne!(target, 0);
        assert_eq!(calls_to(&e, TARGET_SET_TYPE), [[target, 3]]);
        // Without the flag nothing happens.
        let p = package(&mut e, 0xe);
        log_on(&mut e);
        e.call(0x00674930, &args![p]);
        assert!(calls_to(&e, TARGET_SET_TYPE).is_empty());
        assert!(e.get(p, TESPackage::pPackTarg).is_null());
    }

    #[test]
    fn specific_flag_bit_2() {
        let mut e = engine();
        let p = package(&mut e, 0);
        assert!(!e.call(0x00674d20, &args![p]).bool());
        e.set(p, TESPackage::iPackageSpecificFlags, 2);
        assert!(e.call(0x00674d20, &args![p]).bool());
        e.set(p, TESPackage::iPackageSpecificFlags, 0xfffd);
        assert!(!e.call(0x00674d20, &args![p]).bool());
    }

    #[test]
    fn get_is_created_tests_bit_0x800() {
        check_flag_getter(0x00674d40, 0x800);
    }

    #[test]
    fn set_is_created_only_for_dynamic_form_ids() {
        let mut e = engine();
        e.set_global(DATA_HANDLER, 0x7777u32);
        let p = package(&mut e, 0);
        e.mem.set_u32(p.addr() + 0x0c, 0xff00_0001);
        // Not dynamic: nothing changes.
        log_on(&mut e);
        e.call(0x00674d70, &args![p, 1u32]);
        assert_eq!(calls_to(&e, IS_DYNAMIC_FORM_ID), [[0x7777, 0xff00_0001]]);
        assert_eq!(e.get(p, TESPackage::iPackFlags), 0);
        // Dynamic: set and clear.
        e.register(IS_DYNAMIC_FORM_ID, |_, _| true.into_ret());
        e.set(p, TESPackage::iPackFlags, 1);
        e.call(0x00674d70, &args![p, 1u32]);
        assert_eq!(e.get(p, TESPackage::iPackFlags), 0x801);
        e.call(0x00674d70, &args![p, 0u32]);
        assert_eq!(e.get(p, TESPackage::iPackFlags), 1);
    }
}
