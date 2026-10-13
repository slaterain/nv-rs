//! `fallout/interface/menumanager.cpp` (Xbox PDB source unit), subsystem `fallout/interface`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds `MenuManager` (an empty class whose single instance sits
//! in the global `011d8d54`), its menu factory `CreateMenuByClass`, and the
//! constructors, destructors, `GetClass` and `AttachTileByID` methods of
//! the menus the factory makes that the compiler emitted here (tutorial,
//! stats, text edit, surgery, VATS, S.P.E.C.I.A.L. book, love tester and
//! quantity menus), followed by the `ListBox` template instances.
//!
//! Notes that apply to every function:
//! - The menu classes belong to other units (`statsmenu.cpp`, ...); their
//!   fields are written at the PC offsets the code uses, with the Xbox PDB
//!   field name in a comment where the offset matches the PDB. A class's
//!   identity comes from the RTTI type descriptor of the vtable the code
//!   stores (`.?AVStatsMenu@@`, ...).
//! - Calls to functions of this file that the queue lists after the ones
//!   translated so far (the `ListBox` constructors and destructors,
//!   `00721060` on) go through their exe address; the factory calls every
//!   menu constructor through its address, as one table.
//! - The compiler's exception-unwinding frames (`FS:[0]`) and the security
//!   cookie the prologues mix into them are not translated; the functions
//!   that have them carry a `Translation incomplete` marker.
//! - `float` stores of `FLDZ` are `0.0`.
//!
//! The functions from `00720a30` (`QuantityMenu::GetClass`) on are not
//! translated yet.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::units::fallout_interface::run_007e6320::TextEditMenu;
use crate::units::fallout_interface::tutorialmenu::TutorialMenu;

/// The allocator, cdecl (size): the block or null.
const ALLOCATE: u32 = 0x0040_1000;
/// The allocator's free, cdecl (block).
const FREE: u32 = 0x0040_1030;
/// `memset`, cdecl (destination, byte, count).
const MEMSET: u32 = 0x0040_3d30;
/// The compiler's array constructor helper, stdcall (array, element size,
/// count, element constructor, element destructor).
const VECTOR_CONSTRUCT: u32 = 0x00ec_782f;
/// The compiler's array destructor helper, stdcall (array, element size,
/// count, element destructor); engine map: `eh vector destructor iterator`.
const VECTOR_DESTRUCT: u32 = 0x00ec_5fce;
/// Debug message, cdecl (format, ...).
const DEBUG_MESSAGE: u32 = 0x005b_5e40;
/// `"MENUS: Unknown menu class!"`.
const UNKNOWN_MENU_CLASS_TEXT: u32 = 0x0106_ff64;
/// The `Menu` base constructor, thiscall.
const MENU_CONSTRUCT: u32 = 0x00a1_c4a0;
/// The `Menu` base destructor, thiscall.
const MENU_DESTRUCT: u32 = 0x00a1_c520;

/// `MenuManager`'s single instance (`MenuManager::pInstance`, Xbox PDB).
const MENU_MANAGER: u32 = 0x011d_8d54;
/// The object the `MenuManager` constructor body grows to `0x54` entries.
const MENU_TABLE: u32 = 0x011f_3508;
/// Thiscall on `MENU_TABLE` (count).
const MENU_TABLE_GROW: u32 = 0x0096_ad30;
/// The number the constructor body passes: `0x54`, as many as the Xbox
/// PDB's `Menu::pMenusVisible` (`bool[84]`) has entries.
const MENU_TABLE_COUNT: u32 = 0x54;

/// Vtables (class from the RTTI type descriptor before each).
const TUTORIAL_MENU_VTABLE: u32 = 0x0106_ff84;
const STATS_MENU_VTABLE: u32 = 0x0106_ffd4;
const NAMED_EFFECTS_LIST_VTABLE: u32 = 0x0107_0020;
const STATUS_EFFECT_LIST_VTABLE: u32 = 0x0107_0028;
const TEXT_EDIT_MENU_VTABLE: u32 = 0x0107_0034;
const SURGERY_MENU_VTABLE: u32 = 0x0107_0084;
const VATS_MENU_VTABLE: u32 = 0x0107_00d4;
const SPECIAL_BOOK_MENU_VTABLE: u32 = 0x0107_0124;
const LOVE_TESTER_MENU_VTABLE: u32 = 0x0107_0174;
const QUANTITY_MENU_VTABLE: u32 = 0x0107_01c4;

/// The tutorial menu global the tutorial menu destructor clears
/// (`TutorialMenu::pMe`, see `tutorialmenu.rs`).
const TUTORIAL_MENU_GLOBAL: u32 = 0x011d_b088;
/// The global the stats menu destructor clears.
const STATS_MENU_GLOBAL: u32 = 0x011d_ace0;
/// The object `fn_0071f8c0` deletes (six 8-byte lists from +4, as the Xbox
/// PDB's `StatsMenu::BodyPartController::listFlashingTiles`).
const BODY_PART_CONTROLLER_GLOBAL: u32 = 0x011d_ace4;
/// The string the text edit menu constructor empties (the text edit
/// menu's entered text, see `run_007e6320.rs`).
const ENTERED_TEXT: u32 = 0x011d_aed8;
/// The three smart pointer globals the surgery menu constructor and
/// destructor set to null, in the order the code does.
const SURGERY_STATIC_POINTERS: [u32; 3] = [0x011d_adb4, 0x011d_ade8, 0x011d_ae5c];

/// Thiscall (flags) on the item a healing option holds (+8).
const ITEM_DELETE: u32 = 0x0044_59e0;
/// List constructor, thiscall (the list at +4 of the effect lists).
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// List destructor, thiscall.
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
/// List clear, thiscall.
const LIST_CLEAR: u32 = 0x0047_0470;
/// Thiscall on a `StatsMenu::NamedEffectsList` (statsmenu.cpp): empties it.
const NAMED_EFFECTS_CLEAR: u32 = 0x007e_09b0;
/// Thiscall on a `StatsMenu::StatusEffectList` (statsmenu.cpp): empties it.
const STATUS_EFFECTS_CLEAR: u32 = 0x007e_07f0;
/// `StatsMenu::StatusDataList::Flush` (engine map), thiscall.
const STATUS_DATA_FLUSH: u32 = 0x0040_6de0;
/// Thiscall: constructs the text edit menu's editor (+0x34).
const TEXT_EDITOR_CONSTRUCT: u32 = 0x0071_6980;
/// Thiscall: destroys the text edit menu's editor.
const TEXT_EDITOR_DESTRUCT: u32 = 0x0071_6a10;
/// String set, thiscall (text, length).
const STRING_SET: u32 = 0x0040_37f0;
/// An empty constructor, thiscall: returns `this` and writes nothing.
const EMPTY_CONSTRUCTOR: u32 = 0x0068_15c0;
/// Sound handle constructor, thiscall.
const SOUND_HANDLE_CONSTRUCT: u32 = 0x0041_a250;
/// An empty function (`RET`) the surgery menu destructor calls on its two
/// sound handles.
const EMPTY_FUNCTION: u32 = 0x0048_3710;
/// Smart pointer default constructor, thiscall.
const POINTER_CONSTRUCT_DEFAULT: u32 = 0x0066_94e0;
/// Smart pointer constructor from a pointer, thiscall (pointer).
const POINTER_CONSTRUCT: u32 = 0x0063_3c90;
/// Smart pointer assignment, thiscall (pointer).
const POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// Smart pointer destructor, thiscall.
const POINTER_DESTRUCT: u32 = 0x0045_cec0;
/// Thiscall: constructs the VATS menu's action list box (+0xB0; the
/// `ListBox<unsigned int>` constructor of this unit, later in the queue).
const VATS_ACTION_LIST_CONSTRUCT: u32 = 0x0072_3750;
/// Thiscall: constructs the VATS menu's command at +0x11C.
const VATS_COMMAND_CONSTRUCT: u32 = 0x009c_a4a0;
/// The VATS menu destructor (fallout/interface), thiscall.
const VATS_MENU_DESTRUCT: u32 = 0x007e_9130;

/// The stats menu's list box constructors and destructors (this unit,
/// later in the queue), by the offset of the list box.
const STATS_LIST_BOX_CONSTRUCTORS: [(u32, u32); 6] = [
    (0x180, 0x0072_1060),
    (0x1b0, 0x0072_1060),
    (0x1e0, 0x0072_1850),
    (0x210, 0x0072_2000),
    (0x240, 0x0072_27b0),
    (0x270, 0x0072_2f60),
];
const STATS_LIST_BOX_DESTRUCTORS: [(u32, u32); 6] = [
    (0x270, 0x0072_3070),
    (0x240, 0x0072_2880),
    (0x210, 0x0072_20d0),
    (0x1e0, 0x0072_1920),
    (0x1b0, 0x0072_1150),
    (0x180, 0x0072_1150),
];

/// Constructors and destructors of the page arrays' `BSSimpleArray`
/// members (this unit, later in the queue), thiscall.
const BOOK_BUTTONS_CONSTRUCT: u32 = 0x0072_3f50;
const BOOK_TEXT_ITEMS_CONSTRUCT: u32 = 0x0072_3f00;
const BOOK_TEXT_ITEMS_DESTRUCT: u32 = 0x0072_3f30;
const BOOK_BUTTONS_DESTRUCT: u32 = 0x0072_3f80;
const LOVE_TESTER_BUTTONS_CONSTRUCT: u32 = 0x0072_4040;
const LOVE_TESTER_TEXT_ITEMS_CONSTRUCT: u32 = 0x0072_3fa0;
const LOVE_TESTER_TEXT_ITEMS_DESTRUCT: u32 = 0x0072_3fd0;
const LOVE_TESTER_BUTTONS_DESTRUCT: u32 = 0x0072_4070;
const LOVE_TESTER_GLOWS_CONSTRUCT: u32 = 0x0072_3ff0;
const LOVE_TESTER_GLOWS_DESTRUCT: u32 = 0x0072_4020;

/// `CreateMenuByClass`'s table, in class order: (menu class, allocation
/// size, constructor). Read from the jump tables at `0071f154` (targets)
/// and `0071f1ec` (class - 0x3e9 to target index, index 37 the default).
const MENU_CLASSES: [(u32, u32, u32); 37] = [
    (0x3e9, 0x94, 0x007a_8a10),
    (0x3ea, 0x124, 0x0077_fa00),
    (0x3eb, 0x2a4, 0x0071_f340),
    (0x3ec, 0x278, 0x0076_bb40),
    (0x3ef, 0x5c0, 0x0078_8440),
    (0x3f0, 0x10c, 0x0075_b0d0),
    (0x3f1, 0x13c, 0x0076_17a0),
    (0x3f4, 0x4c, 0x007b_fb30),
    (0x3f5, 0x1d4, 0x007c_b330),
    (0x3f6, 0xe4, 0x0078_d930),
    (0x3f8, 0x44, 0x0072_0a10),
    (0x3ff, 0x230, 0x0079_6740),
    (0x402, 0x3c, 0x0073_acb0),
    (0x403, 0xcc, 0x0078_4a40),
    (0x40b, 0x8c, 0x007b_5400),
    (0x40c, 0x1a4, 0x007a_c1f0),
    (0x417, 0x80, 0x0075_f070),
    (0x418, 0x88, 0x0075_3230),
    (0x41b, 0x5c, 0x0071_fa80),
    (0x41d, 0x120, 0x0072_ceb0),
    (0x41e, 0x160, 0x0071_fbf0),
    (0x41f, 0x1dc, 0x0076_54f0),
    (0x420, 0x144, 0x0072_0040),
    (0x421, 0xfc, 0x0075_77f0),
    (0x422, 0xa0, 0x007b_7400),
    (0x423, 0x54, 0x0071_f240),
    (0x424, 0x1e4, 0x0072_0170),
    (0x425, 0x90, 0x0078_3310),
    (0x432, 0x230, 0x0072_0550),
    (0x433, 0x88, 0x0075_4b40),
    (0x434, 0x88, 0x007e_7a80),
    (0x435, 0x104, 0x0072_6df0),
    (0x438, 0x158, 0x007c_06f0),
    (0x439, 0xe70, 0x0073_15e0),
    (0x43a, 0x4014, 0x007b_8d30),
    (0x43b, 0xe88, 0x0073_b7e0),
    (0x43c, 0x8c, 0x007e_6780),
];

/// Writes the vtable pointer at `object + 0`.
fn set_vtable(e: &mut Engine, object: Ptr, vtable: u32) {
    e.mem.set_u32(object.addr(), vtable);
}

fn memset(e: &mut Engine, destination: u32, size: u32) {
    e.call(MEMSET, &args![destination, 0u32, size]);
}

/// `count` smart pointers of 4 bytes at `object + offset`, built with the
/// default constructor and destroyed with the smart pointer destructor.
fn construct_pointer_array(e: &mut Engine, object: Ptr, offset: u32, count: u32) {
    e.call(
        VECTOR_CONSTRUCT,
        &args![
            object.addr() + offset,
            4u32,
            count,
            POINTER_CONSTRUCT_DEFAULT,
            POINTER_DESTRUCT
        ],
    );
}

fn destruct_pointer_array(e: &mut Engine, object: Ptr, offset: u32, count: u32) {
    e.call(
        VECTOR_DESTRUCT,
        &args![object.addr() + offset, 4u32, count, POINTER_DESTRUCT],
    );
}

/// Frees `this` when bit 0 of `flags` is set (the deleting destructors'
/// tail).
fn free_if_flagged(e: &mut Engine, this: Ptr, flags: u32) {
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
}

// Translated from 0071e290 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MenuManager::Instance` (Xbox PDB), cdecl: with `create` set and no
/// instance yet, allocates one byte, constructs the manager in it
/// (`fn_0071e340`, skipped when the allocation fails), stores it in the
/// global and runs the constructor body (`fn_0071e400`) again on it.
/// Returns the global.
// Translation incomplete: 0071e290 - the C++ exception-unwinding frame (FS:[0]) that frees the allocation when the constructor throws is not translated
pub fn menu_manager_instance(e: &mut Engine, create: bool) -> u32 {
    if create && e.global::<u32>(MENU_MANAGER) == 0 {
        let block = e.call(ALLOCATE, &args![1u32]).u32();
        let manager = if block == 0 {
            0
        } else {
            fn_0071e340(e, Ptr::new(block)).addr()
        };
        e.set_global(MENU_MANAGER, manager);
        let manager = e.global::<u32>(MENU_MANAGER);
        fn_0071e400(e, Ptr::new(manager));
    }
    e.global(MENU_MANAGER)
}

// Translated from 0071e340 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `MenuManager` constructor: runs the constructor body
/// (`fn_0071e400`) and returns `this`.
pub fn fn_0071e340(e: &mut Engine, this: Ptr) -> Ptr {
    fn_0071e400(e, this);
    this
}

// Translated from 0071e360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the `MenuManager` instance, if any (deleting destructor with
/// flag 1), and clears the global.
pub fn fn_0071e360(e: &mut Engine) {
    let manager: u32 = e.global(MENU_MANAGER);
    if manager != 0 {
        fn_0071e3b0(e, Ptr::new(manager), 1);
        e.set_global(MENU_MANAGER, 0u32);
    }
}

// Translated from 0071e3b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `MenuManager` deleting destructor: runs the destructor
/// (`fn_0071e3e0`) and frees `this` when bit 0 of `flags` is set.
pub fn fn_0071e3b0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_0071e3e0(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 0071e3e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `MenuManager` destructor: clears the instance global
/// (`fn_0071e3f0`).
pub fn fn_0071e3e0(e: &mut Engine, _this: Ptr) {
    fn_0071e3f0(e);
}

// Translated from 0071e3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the `MenuManager` instance global. Takes no argument (it does
/// not read `ECX`).
pub fn fn_0071e3f0(e: &mut Engine) {
    e.set_global(MENU_MANAGER, 0u32);
}

// Translated from 0071e400 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `MenuManager` constructor body: calls `0096ad30` on the object at
/// `011f3508` with `0x54`. `this` is not read.
pub fn fn_0071e400(e: &mut Engine, _this: Ptr) {
    e.call(MENU_TABLE_GROW, &args![MENU_TABLE, MENU_TABLE_COUNT]);
}

// Translated from 0071e420 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MenuManager::CreateMenuByClass` (Xbox PDB): allocates and constructs
/// the menu of class `class` (`MENU_CLASSES`: allocation size and
/// constructor) and returns it; null when the allocation fails. An
/// unknown class (outside `0x3e9..=0x43c`, or a gap in the table) prints
/// `"MENUS: Unknown menu class!"` and returns null. `this` is not read.
// Translation incomplete: 0071e420 - the C++ exception-unwinding frame (FS:[0]) that frees the allocation when a menu constructor throws is not translated
pub fn menu_manager_create_menu_by_class(e: &mut Engine, _this: Ptr, class: u32) -> u32 {
    match MENU_CLASSES.iter().find(|(id, _, _)| *id == class) {
        Some(&(_, size, constructor)) => {
            let block = e.call(ALLOCATE, &args![size]).u32();
            if block == 0 {
                0
            } else {
                e.call(constructor, &args![block]).u32()
            }
        }
        None => {
            e.call(DEBUG_MESSAGE, &args![UNKNOWN_MENU_CLASS_TEXT]);
            0
        }
    }
}

// Translated from 0071f240 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `TutorialMenu` constructor: the `Menu` base, the vtable, the eight
/// tile pointers (+0x28) zeroed, and the page count, current page and help
/// manual cleared. Returns `this`.
// Translation incomplete: 0071f240 - the C++ exception-unwinding frame (FS:[0]) that destroys the Menu base when the constructor throws is not translated
pub fn fn_0071f240(e: &mut Engine, this: Ptr<TutorialMenu>) -> Ptr<TutorialMenu> {
    e.call(MENU_CONSTRUCT, &args![this]);
    set_vtable(e, this.cast(), TUTORIAL_MENU_VTABLE);
    memset(e, this.addr() + 0x28, 0x20);
    e.set(this, TutorialMenu::iNumPages, 0);
    e.set(this, TutorialMenu::iCurrentPage, 0);
    e.set(this, TutorialMenu::pHelpManual, 0);
    this
}

// Translated from 0071f2d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TutorialMenu::GetClass` (Xbox PDB): `0x423`.
pub fn tutorial_menu_get_class(_e: &mut Engine, _this: Ptr<TutorialMenu>) -> u32 {
    0x423
}

// Translated from 0071f2e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TutorialMenu::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor and frees `this` when bit 0 of `flags` is set.
pub fn tutorial_menu_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<TutorialMenu>,
    flags: u32,
) -> Ptr<TutorialMenu> {
    fn_0071f310(e, this);
    free_if_flagged(e, this.cast(), flags);
    this
}

// Translated from 0071f310 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `TutorialMenu` destructor: restores the vtable, clears the tutorial
/// menu global and destroys the `Menu` base.
pub fn fn_0071f310(e: &mut Engine, this: Ptr<TutorialMenu>) {
    set_vtable(e, this.cast(), TUTORIAL_MENU_VTABLE);
    e.set_global(TUTORIAL_MENU_GLOBAL, 0u32);
    e.call(MENU_DESTRUCT, &args![this]);
}

// Translated from 0071f340 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu` constructor: the `Menu` base and vtable, the four
/// healing options (`pHealingOptionsA`, +0x28, 0x10 bytes each, built by
/// `fn_0071f4a0`), the two effect lists (+0x68 named, +0x84 status), the
/// six list boxes (+0x180 to +0x270), the 60 tile pointers (`pTiles`,
/// +0x90) zeroed, the four status effect pointers (+0x74 to +0x80) and the
/// two flags (+0x2A1, +0x2A0) cleared. Returns `this`.
// Translation incomplete: 0071f340 - the C++ exception-unwinding frame (FS:[0]) that destroys the members built so far when a constructor throws is not translated
pub fn fn_0071f340(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.call(MENU_CONSTRUCT, &args![this]);
    set_vtable(e, this, STATS_MENU_VTABLE);
    e.call(
        VECTOR_CONSTRUCT,
        &args![base + 0x28, 0x10u32, 4u32, 0x0071_f4a0u32, 0x0071_f4e0u32],
    );
    fn_0071f540(e, Ptr::new(base + 0x68));
    fn_0071f5d0(e, Ptr::new(base + 0x84));
    for (offset, constructor) in STATS_LIST_BOX_CONSTRUCTORS {
        e.call(constructor, &args![base + offset]);
    }
    memset(e, base + 0x90, 0xf0);
    // pRadiationEffect, pDehydrationEffect, pSleepEffect, pHungerEffect
    // (Xbox PDB), in the code's order.
    for offset in [0x74, 0x78, 0x80, 0x7c] {
        e.mem.set_u32(base + offset, 0);
    }
    // bActiveEffectsUpdatedThisFrame, bLimbMode (Xbox PDB).
    e.mem.set_u8(base + 0x2a1, 0);
    e.mem.set_u8(base + 0x2a0, 0);
    this
}

// Translated from 0071f4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu::HealingOption` constructor: clears `pAlchemyItem`,
/// `pTile` and `pInventoryItem` and sets `pfTargetConditionCallback` to
/// `fn_0071f520` (Xbox PDB names). Returns `this`.
pub fn fn_0071f4a0(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.mem.set_u32(base, 0);
    e.mem.set_u32(base + 4, 0);
    e.mem.set_u32(base + 8, 0);
    e.mem.set_u32(base + 0xc, 0x0071_f520);
    this
}

// Translated from 0071f4e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu::HealingOption` destructor: calls `004459e0` with flag 1
/// on the inventory item (+8), if any.
pub fn fn_0071f4e0(e: &mut Engine, this: Ptr) {
    let item = e.mem.u32(this.addr() + 8);
    if item != 0 {
        e.call(ITEM_DELETE, &args![item, 1u32]);
    }
}

// Translated from 0071f520 (decompiled, FalloutNV.exe 1.4.0.525)
/// A healing option's target condition (cdecl, the option): whether it has
/// an inventory item (+8). The value is in `AL`; the rest of `EAX` is the
/// option's address, which callers of a `bool` do not read.
pub fn fn_0071f520(e: &mut Engine, option: Ptr) -> bool {
    e.mem.u32(option.addr() + 8) != 0
}

// Translated from 0071f540 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu::NamedEffectsList` constructor: the list at +4, then the
/// vtable. Returns `this`.
pub fn fn_0071f540(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(LIST_CONSTRUCT, &args![this.addr() + 4]);
    set_vtable(e, this, NAMED_EFFECTS_LIST_VTABLE);
    this
}

// Translated from 0071f570 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu::NamedEffectsList` destructor: restores the vtable,
/// empties the list (`007e09b0`) and destroys the list at +4.
// Translation incomplete: 0071f570 - the C++ exception-unwinding frame (FS:[0]) that destroys the list when emptying it throws is not translated
pub fn fn_0071f570(e: &mut Engine, this: Ptr) {
    set_vtable(e, this, NAMED_EFFECTS_LIST_VTABLE);
    e.call(NAMED_EFFECTS_CLEAR, &args![this]);
    e.call(LIST_DESTRUCT, &args![this.addr() + 4]);
}

// Translated from 0071f5d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu::StatusEffectList` constructor: the list at +4, then the
/// vtable. Returns `this`.
pub fn fn_0071f5d0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(LIST_CONSTRUCT, &args![this.addr() + 4]);
    set_vtable(e, this, STATUS_EFFECT_LIST_VTABLE);
    this
}

// Translated from 0071f600 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu::StatusEffectList` destructor: restores the vtable,
/// empties the list (`007e07f0`) and destroys the list at +4.
// Translation incomplete: 0071f600 - the C++ exception-unwinding frame (FS:[0]) that destroys the list when emptying it throws is not translated
pub fn fn_0071f600(e: &mut Engine, this: Ptr) {
    set_vtable(e, this, STATUS_EFFECT_LIST_VTABLE);
    e.call(STATUS_EFFECTS_CLEAR, &args![this]);
    e.call(LIST_DESTRUCT, &args![this.addr() + 4]);
}

// Translated from 0071f660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `StatsMenu::GetClass` (Xbox PDB): `0x3eb`.
pub fn stats_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x3eb
}

// Translated from 0071f670 (decompiled, FalloutNV.exe 1.4.0.525)
/// `StatsMenu::NamedEffectsList::_scalar_deleting_destructor_` (Xbox PDB):
/// runs the destructor and frees `this` when bit 0 of `flags` is set.
pub fn stats_menu_named_effects_list_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    fn_0071f570(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 0071f6a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `StatsMenu::StatusEffectList::_scalar_deleting_destructor_` (Xbox PDB):
/// runs the destructor and frees `this` when bit 0 of `flags` is set.
pub fn stats_menu_status_effect_list_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    fn_0071f600(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 0071f6d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `StatsMenu::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor and frees `this` when bit 0 of `flags` is set.
pub fn stats_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_0071f700(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 0071f700 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu` destructor: restores the vtable, clears the stats menu
/// global, deletes the four status effects (+0x74, +0x80, +0x7C, +0x78 in
/// that order, `fn_0071f9c0` with flag 1), deletes the body part
/// controller (`fn_0071f8c0`), destroys the six list boxes, the two effect
/// lists and the four healing options, then the `Menu` base.
// Translation incomplete: 0071f700 - the C++ exception-unwinding frame (FS:[0]) that destroys the remaining members when a destructor throws is not translated
pub fn fn_0071f700(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    set_vtable(e, this, STATS_MENU_VTABLE);
    e.set_global(STATS_MENU_GLOBAL, 0u32);
    for offset in [0x74, 0x80, 0x7c, 0x78] {
        let effect = e.mem.u32(base + offset);
        if effect != 0 {
            fn_0071f9c0(e, Ptr::new(effect), 1);
        }
    }
    fn_0071f8c0(e);
    for (offset, destructor) in STATS_LIST_BOX_DESTRUCTORS {
        e.call(destructor, &args![base + offset]);
    }
    fn_0071f600(e, Ptr::new(base + 0x84));
    fn_0071f570(e, Ptr::new(base + 0x68));
    e.call(
        VECTOR_DESTRUCT,
        &args![base + 0x28, 0x10u32, 4u32, 0x0071_f4e0u32],
    );
    e.call(MENU_DESTRUCT, &args![this]);
}

// Translated from 0071f8c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the object in the global `011dace4`, if any (`fn_0071f900` with
/// flag 1), and clears the global.
pub fn fn_0071f8c0(e: &mut Engine) {
    let controller: u32 = e.global(BODY_PART_CONTROLLER_GLOBAL);
    if controller != 0 {
        fn_0071f900(e, Ptr::new(controller), 1);
    }
    e.set_global(BODY_PART_CONTROLLER_GLOBAL, 0u32);
}

// Translated from 0071f900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deleting destructor of the object `fn_0071f8c0` deletes: runs
/// `fn_0071f930` and frees `this` when bit 0 of `flags` is set.
pub fn fn_0071f900(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_0071f930(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 0071f930 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of the object `fn_0071f8c0` deletes: empties the six 8-byte
/// lists from +4 (`00470470`), then destroys them (`0046ffb0` through the
/// array destructor helper).
// Translation incomplete: 0071f930 - the C++ exception-unwinding frame (FS:[0]) that destroys the lists when emptying one throws is not translated
pub fn fn_0071f930(e: &mut Engine, this: Ptr) {
    for index in 0..6u32 {
        e.call(LIST_CLEAR, &args![this.addr() + index * 8 + 4]);
    }
    e.call(
        VECTOR_DESTRUCT,
        &args![this.addr() + 4, 8u32, 6u32, LIST_DESTRUCT],
    );
}

// Translated from 0071f9c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu::StatusEffect` deleting destructor: runs the destructor
/// (`fn_0071f9f0`) and frees `this` when bit 0 of `flags` is set.
pub fn fn_0071f9c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_0071f9f0(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 0071f9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `StatsMenu::StatusEffect` destructor: destroys `xNamedEffects`
/// (+0x10) and flushes `xStatsEffected` (+4) (Xbox PDB names).
// Translation incomplete: 0071f9f0 - the C++ exception-unwinding frame (FS:[0]) that flushes the data list when the effect list destructor throws is not translated
pub fn fn_0071f9f0(e: &mut Engine, this: Ptr) {
    fn_0071f570(e, Ptr::new(this.addr() + 0x10));
    e.call(STATUS_DATA_FLUSH, &args![this.addr() + 4]);
}

// Translated from 0071fa50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `StatsMenu::AttachTileByID` (Xbox PDB): stores `tile` in `pTiles`
/// (+0x90) for ids 0 to 59 (signed compare); other ids do nothing.
pub fn stats_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (0..0x3c).contains(&id) {
        e.mem.set_u32(this.addr() + 0x90 + 4 * id as u32, tile);
    }
}

// Translated from 0071fa80 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `TextEditMenu` constructor: the `Menu` base and vtable, the editor
/// (+0x34), the three tile pointers (+0x28) zeroed, the entered text
/// global emptied (`004037f0` with null and 0) and the text check (+0x58)
/// cleared. Returns `this`.
// Translation incomplete: 0071fa80 - the C++ exception-unwinding frame (FS:[0]) that destroys the members built so far when a constructor throws is not translated
pub fn fn_0071fa80(e: &mut Engine, this: Ptr<TextEditMenu>) -> Ptr<TextEditMenu> {
    e.call(MENU_CONSTRUCT, &args![this]);
    set_vtable(e, this.cast(), TEXT_EDIT_MENU_VTABLE);
    e.call(TEXT_EDITOR_CONSTRUCT, &args![this.addr() + 0x34]);
    memset(e, this.addr() + 0x28, 0xc);
    e.call(STRING_SET, &args![ENTERED_TEXT, 0u32, 0u32]);
    e.set(this, TextEditMenu::pTextCheck, 0);
    this
}

// Translated from 0071fb20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TextEditMenu`'s `GetClass` (vtable slot +0x34): `0x41b`.
pub fn fn_0071fb20(_e: &mut Engine, _this: Ptr<TextEditMenu>) -> u32 {
    0x41b
}

// Translated from 0071fb30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TextEditMenu`'s deleting destructor (vtable slot 0): runs the
/// destructor and frees `this` when bit 0 of `flags` is set.
pub fn fn_0071fb30(e: &mut Engine, this: Ptr<TextEditMenu>, flags: u32) -> Ptr<TextEditMenu> {
    fn_0071fb60(e, this);
    free_if_flagged(e, this.cast(), flags);
    this
}

// Translated from 0071fb60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `TextEditMenu` destructor: restores the vtable, destroys the editor
/// (+0x34) and the `Menu` base.
// Translation incomplete: 0071fb60 - the C++ exception-unwinding frame (FS:[0]) that destroys the Menu base when the editor destructor throws is not translated
pub fn fn_0071fb60(e: &mut Engine, this: Ptr<TextEditMenu>) {
    set_vtable(e, this.cast(), TEXT_EDIT_MENU_VTABLE);
    e.call(TEXT_EDITOR_DESTRUCT, &args![this.addr() + 0x34]);
    e.call(MENU_DESTRUCT, &args![this]);
}

// Translated from 0071fbc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TextEditMenu`'s `AttachTileByID` (vtable slot +4): stores `tile` at
/// +0x28 for ids 0 to 2 (signed compare); other ids do nothing.
pub fn fn_0071fbc0(e: &mut Engine, this: Ptr<TextEditMenu>, id: i32, tile: u32) {
    if (0..3).contains(&id) {
        e.mem.set_u32(this.addr() + 0x28 + 4 * id as u32, tile);
    }
}

// Translated from 0071fbf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `SurgeryMenu` constructor: the `Menu` base and vtable, the members
/// (empty constructors at +0x74, +0x7C, +0xF8, +0x104, +0x128, +0x150; sound handles
/// at +0x94 and +0xA0; six smart pointers at +0xC8; smart pointers at
/// +0xF0 and +0xF4), the 18 tile pointers (+0x2C) zeroed, the three static
/// smart pointers, +0xF0, +0xF4 and the six at +0xC8 set to null, then the
/// scalar fields (Xbox PDB names in the body). Returns `this`.
// Translation incomplete: 0071fbf0 - the C++ exception-unwinding frame (FS:[0]) that destroys the members built so far when a constructor throws is not translated
pub fn fn_0071fbf0(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.call(MENU_CONSTRUCT, &args![this]);
    set_vtable(e, this, SURGERY_MENU_VTABLE);
    e.call(EMPTY_CONSTRUCTOR, &args![base + 0x74]);
    e.call(EMPTY_CONSTRUCTOR, &args![base + 0x7c]);
    e.call(SOUND_HANDLE_CONSTRUCT, &args![base + 0x94]);
    e.call(SOUND_HANDLE_CONSTRUCT, &args![base + 0xa0]);
    construct_pointer_array(e, this, 0xc8, 6);
    e.call(POINTER_CONSTRUCT, &args![base + 0xf0, 0u32]);
    e.call(POINTER_CONSTRUCT, &args![base + 0xf4, 0u32]);
    for offset in [0xf8, 0x104, 0x128, 0x150] {
        e.call(EMPTY_CONSTRUCTOR, &args![base + offset]);
    }
    memset(e, base + 0x2c, 0x48);
    for global in SURGERY_STATIC_POINTERS {
        e.call(POINTER_ASSIGN, &args![global, 0u32]);
    }
    e.call(POINTER_ASSIGN, &args![base + 0xf0, 0u32]);
    e.call(POINTER_ASSIGN, &args![base + 0xf4, 0u32]);
    for index in 0..6u32 {
        e.call(POINTER_ASSIGN, &args![base + 0xc8 + 4 * index, 0u32]);
    }
    // pFrameGeometry, pSkinControllerManager, pStimPaks, pScar.
    for offset in [0xe0, 0xe4, 0xbc, 0xe8] {
        e.mem.set_u32(base + offset, 0);
    }
    // fFrameAnimTimer, fToolMovement, fPainPercent; +0x14C (a float).
    for offset in [0xec, 0x84, 0xac, 0x14c] {
        e.mem.set_f32(base + offset, 0.0);
    }
    // uiDamageHealthTime is set to 1000 and cleared again below.
    e.mem.set_u32(base + 0xb8, 0x3e8);
    // uiWoundBurnTime, uiWoundCloseTime, uiPainTimer, uiHitPointTimer,
    // uiDamageHealthTime.
    for offset in [0x88, 0x8c, 0xb0, 0xb4, 0xb8] {
        e.mem.set_u32(base + offset, 0);
    }
    // bBurning, bToolBuriedInFlesh; +0x15C, +0x15D.
    for offset in [0x90, 0x91, 0x15c, 0x15d] {
        e.mem.set_u8(base + offset, 0);
    }
    // Resistance, ToolLocation (NiPoint2, Xbox PDB).
    for offset in [0x7c, 0x80, 0x74, 0x78] {
        e.mem.set_f32(base + offset, 0.0);
    }
    // eCurrentDamagedPart, eCurrentDamageLevel, eAnimation.
    e.mem.set_u32(base + 0xc0, 0x1b);
    e.mem.set_u32(base + 0xc4, 1);
    e.mem.set_u32(base + 0x28, 0);
    this
}

// Translated from 0071feb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SurgeryMenu::GetClass` (Xbox PDB): `0x41e`.
pub fn surgery_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x41e
}

// Translated from 0071fec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SurgeryMenu::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor and frees `this` when bit 0 of `flags` is set.
pub fn surgery_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_0071fef0(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 0071fef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `SurgeryMenu` destructor: restores the vtable, sets the three
/// static smart pointers and +0xF0 to null, destroys the smart pointers at
/// +0xF4, +0xF0 and the six at +0xC8, calls the empty function `00483710`
/// on the sound handles (+0xA0, +0x94) and destroys the `Menu` base.
// Translation incomplete: 0071fef0 - the C++ exception-unwinding frame (FS:[0]) that destroys the remaining members when a destructor throws is not translated
pub fn fn_0071fef0(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    set_vtable(e, this, SURGERY_MENU_VTABLE);
    for global in SURGERY_STATIC_POINTERS {
        e.call(POINTER_ASSIGN, &args![global, 0u32]);
    }
    e.call(POINTER_ASSIGN, &args![base + 0xf0, 0u32]);
    e.call(POINTER_DESTRUCT, &args![base + 0xf4]);
    e.call(POINTER_DESTRUCT, &args![base + 0xf0]);
    destruct_pointer_array(e, this, 0xc8, 6);
    e.call(EMPTY_FUNCTION, &args![base + 0xa0]);
    e.call(EMPTY_FUNCTION, &args![base + 0x94]);
    e.call(MENU_DESTRUCT, &args![this]);
}

// Translated from 0071ffe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SurgeryMenu::AttachTileByID` (Xbox PDB): for ids 1 to 18 (signed
/// compare) stores `tile` in `pTiles` (+0x2C) at the index `fn_00720010`
/// gives (`id - 1`); other ids do nothing.
pub fn surgery_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (1..=0x12).contains(&id) {
        let index = fn_00720010(e, this, id);
        e.mem.set_u32(this.addr() + 0x2c + 4 * index as u32, tile);
    }
}

// Translated from 00720010 (decompiled, FalloutNV.exe 1.4.0.525)
/// The surgery menu's tile index of a tile id: `id - 1` for ids 1 to 18
/// (signed compare), 18 otherwise.
pub fn fn_00720010(_e: &mut Engine, _this: Ptr, id: i32) -> i32 {
    if (1..=0x12).contains(&id) {
        id - 1
    } else {
        0x12
    }
}

// Translated from 00720040 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `VATSMenu` constructor: the `Menu` base and vtable, the action list
/// box (`lbActions`, +0xB0), the command at +0x11C, the 32 tile pointers
/// (`pTiles`, +0x28) zeroed, `bHeadingReached` (+0xF8) and
/// `bConcentratedFirePerk` (+0x104) cleared and `pLastLimb` (+0x108) null
/// (Xbox PDB names). Returns `this`.
// Translation incomplete: 00720040 - the C++ exception-unwinding frame (FS:[0]) that destroys the members built so far when a constructor throws is not translated
pub fn fn_00720040(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.call(MENU_CONSTRUCT, &args![this]);
    set_vtable(e, this, VATS_MENU_VTABLE);
    e.call(VATS_ACTION_LIST_CONSTRUCT, &args![base + 0xb0]);
    e.call(VATS_COMMAND_CONSTRUCT, &args![base + 0x11c]);
    memset(e, base + 0x28, 0x80);
    e.mem.set_u8(base + 0xf8, 0);
    e.mem.set_u8(base + 0x104, 0);
    e.mem.set_u32(base + 0x108, 0);
    this
}

// Translated from 00720100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `VATSMenu::GetClass` (Xbox PDB): `0x420`.
pub fn vats_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x420
}

// Translated from 00720110 (decompiled, FalloutNV.exe 1.4.0.525)
/// `VATSMenu::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`007e9130`) and frees `this` when bit 0 of `flags` is set.
pub fn vats_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(VATS_MENU_DESTRUCT, &args![this]);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 00720140 (decompiled, FalloutNV.exe 1.4.0.525)
/// `VATSMenu::AttachTileByID` (Xbox PDB): stores `tile` at
/// `+0x28 + 4 * id` for ids 1 to 31 (signed compare); other ids, 0
/// included, do nothing.
pub fn vats_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (1..=0x1f).contains(&id) {
        e.mem.set_u32(this.addr() + 0x28 + 4 * id as u32, tile);
    }
}

// Translated from 00720170 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `SPECIALBookMenu` constructor: the `Menu` base and vtable, the nine
/// pages (`Pages`, +0x60, 0x20 bytes each: `fn_00720360` /
/// `fn_007203c0`), the smart pointer arrays (`spAVValues` 11 at +0x180,
/// `spPointsText` 4 at +0x1AC, five pairs from +0x1BC), the eight tile
/// pointers (+0x28) zeroed and the state fields set (Xbox PDB names in the
/// body). Returns `this`.
// Translation incomplete: 00720170 - the C++ exception-unwinding frame (FS:[0]) that destroys the members built so far when a constructor throws is not translated
pub fn fn_00720170(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.call(MENU_CONSTRUCT, &args![this]);
    set_vtable(e, this, SPECIAL_BOOK_MENU_VTABLE);
    e.call(
        VECTOR_CONSTRUCT,
        &args![base + 0x60, 0x20u32, 9u32, 0x0072_0360u32, 0x0072_03c0u32],
    );
    construct_pointer_array(e, this, 0x180, 0xb);
    construct_pointer_array(e, this, 0x1ac, 4);
    for offset in [0x1bc, 0x1c4, 0x1cc, 0x1d4, 0x1dc] {
        construct_pointer_array(e, this, offset, 2);
    }
    memset(e, base + 0x28, 0x20);
    // pBookGeometry, eState, eXBoxIndexState, fFrameAnimTimer,
    // bAnimFinished, iAvailablePoints.
    e.mem.set_u32(base + 0x5c, 0);
    e.mem.set_u32(base + 0x48, 0);
    e.mem.set_u32(base + 0x4c, 1);
    e.mem.set_f32(base + 0x50, 0.0);
    e.mem.set_u8(base + 0x54, 1);
    e.mem.set_u32(base + 0x58, 0);
    this
}

// Translated from 00720320 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SPECIALBookMenu::GetClass` (Xbox PDB): `0x424`.
pub fn special_book_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x424
}

// Translated from 00720330 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SPECIALBookMenu::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor and frees `this` when bit 0 of `flags` is set.
pub fn special_book_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00720420(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 00720360 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `SPECIALBookMenu::SpecialBookPage` constructor: `Buttons` (+0) and
/// `TextItems` (+0x10) (Xbox PDB). Returns `this`.
// Translation incomplete: 00720360 - the C++ exception-unwinding frame (FS:[0]) that destroys the buttons when the text item constructor throws is not translated
pub fn fn_00720360(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(BOOK_BUTTONS_CONSTRUCT, &args![this]);
    e.call(BOOK_TEXT_ITEMS_CONSTRUCT, &args![this.addr() + 0x10]);
    this
}

// Translated from 007203c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `SPECIALBookMenu::SpecialBookPage` destructor: `TextItems` (+0x10),
/// then `Buttons` (+0).
// Translation incomplete: 007203c0 - the C++ exception-unwinding frame (FS:[0]) that destroys the buttons when the text item destructor throws is not translated
pub fn fn_007203c0(e: &mut Engine, this: Ptr) {
    e.call(BOOK_TEXT_ITEMS_DESTRUCT, &args![this.addr() + 0x10]);
    e.call(BOOK_BUTTONS_DESTRUCT, &args![this]);
}

// Translated from 00720420 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `SPECIALBookMenu` destructor: restores the vtable, destroys the
/// smart pointer arrays (in reverse order) and the nine pages, then the
/// `Menu` base.
// Translation incomplete: 00720420 - the C++ exception-unwinding frame (FS:[0]) that destroys the remaining members when a destructor throws is not translated
pub fn fn_00720420(e: &mut Engine, this: Ptr) {
    set_vtable(e, this, SPECIAL_BOOK_MENU_VTABLE);
    for offset in [0x1dc, 0x1d4, 0x1cc, 0x1c4, 0x1bc] {
        destruct_pointer_array(e, this, offset, 2);
    }
    destruct_pointer_array(e, this, 0x1ac, 4);
    destruct_pointer_array(e, this, 0x180, 0xb);
    e.call(
        VECTOR_DESTRUCT,
        &args![this.addr() + 0x60, 0x20u32, 9u32, 0x0072_03c0u32],
    );
    e.call(MENU_DESTRUCT, &args![this]);
}

// Translated from 00720550 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `LoveTesterMenu` constructor: the `Menu` base and vtable,
/// `LightBulbs` (+0x64) and `LightBulbGlows` (+0x74), the nine pages
/// (`Pages`, +0x84, `fn_007207a0` / `fn_00720800`), `GlobalPage` (+0x1A4),
/// the smart pointer arrays (11 at +0x1C4, 4 at +0x1F0, six pairs from
/// +0x200), the eight tile pointers (+0x28) zeroed and the state fields
/// set (Xbox PDB names in the body). Returns `this`.
// Translation incomplete: 00720550 - the C++ exception-unwinding frame (FS:[0]) that destroys the members built so far when a constructor throws is not translated
pub fn fn_00720550(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.call(MENU_CONSTRUCT, &args![this]);
    set_vtable(e, this, LOVE_TESTER_MENU_VTABLE);
    e.call(LOVE_TESTER_TEXT_ITEMS_CONSTRUCT, &args![base + 0x64]);
    e.call(LOVE_TESTER_GLOWS_CONSTRUCT, &args![base + 0x74]);
    e.call(
        VECTOR_CONSTRUCT,
        &args![base + 0x84, 0x20u32, 9u32, 0x0072_07a0u32, 0x0072_0800u32],
    );
    fn_007207a0(e, Ptr::new(base + 0x1a4));
    construct_pointer_array(e, this, 0x1c4, 0xb);
    construct_pointer_array(e, this, 0x1f0, 4);
    for offset in [0x200, 0x208, 0x210, 0x218, 0x220, 0x228] {
        construct_pointer_array(e, this, offset, 2);
    }
    memset(e, base + 0x28, 0x20);
    // pDynamicGeometry, pCabinetGeometry, eState, eXBoxIndexState,
    // fFrameAnimTimer, bAnimFinished, iAvailablePoints.
    e.mem.set_u32(base + 0x5c, 0);
    e.mem.set_u32(base + 0x60, 0);
    e.mem.set_u32(base + 0x48, 0);
    e.mem.set_u32(base + 0x4c, 1);
    e.mem.set_f32(base + 0x50, 0.0);
    e.mem.set_u8(base + 0x54, 1);
    e.mem.set_u32(base + 0x58, 0);
    this
}

// Translated from 00720760 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoveTesterMenu::GetClass` (Xbox PDB): `0x432`.
pub fn love_tester_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x432
}

// Translated from 00720770 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoveTesterMenu::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor and frees `this` when bit 0 of `flags` is set.
pub fn love_tester_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_00720860(e, this);
    free_if_flagged(e, this, flags);
    this
}

// Translated from 007207a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `LoveTesterMenu::LoveTesterPage` constructor: `Buttons` (+0) and
/// `TextItems` (+0x10) (Xbox PDB). Returns `this`.
// Translation incomplete: 007207a0 - the C++ exception-unwinding frame (FS:[0]) that destroys the buttons when the text item constructor throws is not translated
pub fn fn_007207a0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(LOVE_TESTER_BUTTONS_CONSTRUCT, &args![this]);
    e.call(LOVE_TESTER_TEXT_ITEMS_CONSTRUCT, &args![this.addr() + 0x10]);
    this
}

// Translated from 00720800 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `LoveTesterMenu::LoveTesterPage` destructor: `TextItems` (+0x10),
/// then `Buttons` (+0).
// Translation incomplete: 00720800 - the C++ exception-unwinding frame (FS:[0]) that destroys the buttons when the text item destructor throws is not translated
pub fn fn_00720800(e: &mut Engine, this: Ptr) {
    e.call(LOVE_TESTER_TEXT_ITEMS_DESTRUCT, &args![this.addr() + 0x10]);
    e.call(LOVE_TESTER_BUTTONS_DESTRUCT, &args![this]);
}

// Translated from 00720860 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `LoveTesterMenu` destructor: restores the vtable, destroys the
/// smart pointer arrays (in reverse order), `GlobalPage`, the nine pages,
/// `LightBulbGlows` and `LightBulbs`, then the `Menu` base.
// Translation incomplete: 00720860 - the C++ exception-unwinding frame (FS:[0]) that destroys the remaining members when a destructor throws is not translated
pub fn fn_00720860(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    set_vtable(e, this, LOVE_TESTER_MENU_VTABLE);
    for offset in [0x228, 0x220, 0x218, 0x210, 0x208, 0x200] {
        destruct_pointer_array(e, this, offset, 2);
    }
    destruct_pointer_array(e, this, 0x1f0, 4);
    destruct_pointer_array(e, this, 0x1c4, 0xb);
    fn_00720800(e, Ptr::new(base + 0x1a4));
    e.call(
        VECTOR_DESTRUCT,
        &args![base + 0x84, 0x20u32, 9u32, 0x0072_0800u32],
    );
    e.call(LOVE_TESTER_GLOWS_DESTRUCT, &args![base + 0x74]);
    e.call(LOVE_TESTER_TEXT_ITEMS_DESTRUCT, &args![base + 0x64]);
    e.call(MENU_DESTRUCT, &args![this]);
}

// Translated from 007209e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TutorialMenu::AttachTileByID` (Xbox PDB): stores `tile` at
/// `+0x28 + 4 * id` for ids 0 to 7 (signed compare); other ids do nothing.
/// The linker shares the body: the `SPECIALBookMenu` and `LoveTesterMenu`
/// vtables have it in the same slot.
pub fn tutorial_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (0..8).contains(&id) {
        e.mem.set_u32(this.addr() + 0x28 + 4 * id as u32, tile);
    }
}

// Translated from 00720a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `QuantityMenu` constructor: the `Menu` base and the vtable. Returns
/// `this`.
pub fn fn_00720a10(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(MENU_CONSTRUCT, &args![this]);
    set_vtable(e, this, QUANTITY_MENU_VTABLE);
    this
}

// Translated from 00726070 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the word at +4. Folded callers include a list's next-node pointer
/// and an inventory entry's signed count; the accessor preserves all bits.
pub fn word_at_four(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(4))
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0071e290, menu_manager_instance(bool) -> u32),
        entry!(0x0071e340, fn_0071e340(Ptr) -> Ptr),
        entry!(0x0071e360, fn_0071e360()),
        entry!(0x0071e3b0, fn_0071e3b0(Ptr, u32) -> Ptr),
        entry!(0x0071e3e0, fn_0071e3e0(Ptr)),
        entry!(0x0071e3f0, fn_0071e3f0()),
        entry!(0x0071e400, fn_0071e400(Ptr)),
        entry!(
            0x0071e420,
            menu_manager_create_menu_by_class(Ptr, u32) -> u32
        ),
        entry!(
            0x0071f240,
            fn_0071f240(Ptr<TutorialMenu>) -> Ptr<TutorialMenu>
        ),
        entry!(
            0x0071f2d0,
            tutorial_menu_get_class(Ptr<TutorialMenu>) -> u32
        ),
        entry!(
            0x0071f2e0,
            tutorial_menu_scalar_deleting_destructor(Ptr<TutorialMenu>, u32) -> Ptr<TutorialMenu>
        ),
        entry!(0x0071f310, fn_0071f310(Ptr<TutorialMenu>)),
        entry!(0x0071f340, fn_0071f340(Ptr) -> Ptr),
        entry!(0x0071f4a0, fn_0071f4a0(Ptr) -> Ptr),
        entry!(0x0071f4e0, fn_0071f4e0(Ptr)),
        entry!(0x0071f520, fn_0071f520(Ptr) -> bool),
        entry!(0x0071f540, fn_0071f540(Ptr) -> Ptr),
        entry!(0x0071f570, fn_0071f570(Ptr)),
        entry!(0x0071f5d0, fn_0071f5d0(Ptr) -> Ptr),
        entry!(0x0071f600, fn_0071f600(Ptr)),
        entry!(0x0071f660, stats_menu_get_class(Ptr) -> u32),
        entry!(
            0x0071f670,
            stats_menu_named_effects_list_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(
            0x0071f6a0,
            stats_menu_status_effect_list_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(
            0x0071f6d0,
            stats_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x0071f700, fn_0071f700(Ptr)),
        entry!(0x0071f8c0, fn_0071f8c0()),
        entry!(0x0071f900, fn_0071f900(Ptr, u32) -> Ptr),
        entry!(0x0071f930, fn_0071f930(Ptr)),
        entry!(0x0071f9c0, fn_0071f9c0(Ptr, u32) -> Ptr),
        entry!(0x0071f9f0, fn_0071f9f0(Ptr)),
        entry!(0x0071fa50, stats_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(
            0x0071fa80,
            fn_0071fa80(Ptr<TextEditMenu>) -> Ptr<TextEditMenu>
        ),
        entry!(0x0071fb20, fn_0071fb20(Ptr<TextEditMenu>) -> u32),
        entry!(
            0x0071fb30,
            fn_0071fb30(Ptr<TextEditMenu>, u32) -> Ptr<TextEditMenu>
        ),
        entry!(0x0071fb60, fn_0071fb60(Ptr<TextEditMenu>)),
        entry!(0x0071fbc0, fn_0071fbc0(Ptr<TextEditMenu>, i32, u32)),
        entry!(0x0071fbf0, fn_0071fbf0(Ptr) -> Ptr),
        entry!(0x0071feb0, surgery_menu_get_class(Ptr) -> u32),
        entry!(
            0x0071fec0,
            surgery_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x0071fef0, fn_0071fef0(Ptr)),
        entry!(0x0071ffe0, surgery_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(0x00720010, fn_00720010(Ptr, i32) -> i32),
        entry!(0x00720040, fn_00720040(Ptr) -> Ptr),
        entry!(0x00720100, vats_menu_get_class(Ptr) -> u32),
        entry!(
            0x00720110,
            vats_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x00720140, vats_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(0x00720170, fn_00720170(Ptr) -> Ptr),
        entry!(0x00720320, special_book_menu_get_class(Ptr) -> u32),
        entry!(
            0x00720330,
            special_book_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x00720360, fn_00720360(Ptr) -> Ptr),
        entry!(0x007203c0, fn_007203c0(Ptr)),
        entry!(0x00720420, fn_00720420(Ptr)),
        entry!(0x00720550, fn_00720550(Ptr) -> Ptr),
        entry!(0x00720760, love_tester_menu_get_class(Ptr) -> u32),
        entry!(
            0x00720770,
            love_tester_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x007207a0, fn_007207a0(Ptr) -> Ptr),
        entry!(0x00720800, fn_00720800(Ptr)),
        entry!(0x00720860, fn_00720860(Ptr)),
        entry!(0x007209e0, tutorial_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(0x00720a10, fn_00720a10(Ptr) -> Ptr),
        entry!(0x00726070, word_at_four(Ptr) -> u32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    type Calls = Vec<Vec<u32>>;
    type Sequence = Vec<(u32, Vec<u32>)>;

    fn word(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// Every callee outside this file answers 0; the allocator hands out
    /// heap blocks; the globals the file uses are mapped.
    fn engine() -> Engine {
        let mut e = Engine::new();
        let mut externals = vec![
            FREE,
            MEMSET,
            VECTOR_CONSTRUCT,
            VECTOR_DESTRUCT,
            DEBUG_MESSAGE,
            MENU_CONSTRUCT,
            MENU_DESTRUCT,
            MENU_TABLE_GROW,
            ITEM_DELETE,
            LIST_CONSTRUCT,
            LIST_DESTRUCT,
            LIST_CLEAR,
            NAMED_EFFECTS_CLEAR,
            STATUS_EFFECTS_CLEAR,
            STATUS_DATA_FLUSH,
            TEXT_EDITOR_CONSTRUCT,
            TEXT_EDITOR_DESTRUCT,
            STRING_SET,
            EMPTY_CONSTRUCTOR,
            SOUND_HANDLE_CONSTRUCT,
            EMPTY_FUNCTION,
            POINTER_CONSTRUCT_DEFAULT,
            POINTER_CONSTRUCT,
            POINTER_ASSIGN,
            POINTER_DESTRUCT,
            VATS_ACTION_LIST_CONSTRUCT,
            VATS_COMMAND_CONSTRUCT,
            VATS_MENU_DESTRUCT,
            BOOK_BUTTONS_CONSTRUCT,
            BOOK_TEXT_ITEMS_CONSTRUCT,
            BOOK_TEXT_ITEMS_DESTRUCT,
            BOOK_BUTTONS_DESTRUCT,
            LOVE_TESTER_BUTTONS_CONSTRUCT,
            LOVE_TESTER_TEXT_ITEMS_CONSTRUCT,
            LOVE_TESTER_TEXT_ITEMS_DESTRUCT,
            LOVE_TESTER_BUTTONS_DESTRUCT,
            LOVE_TESTER_GLOWS_CONSTRUCT,
            LOVE_TESTER_GLOWS_DESTRUCT,
        ];
        externals.extend(STATS_LIST_BOX_CONSTRUCTORS.iter().map(|(_, a)| *a));
        externals.extend(STATS_LIST_BOX_DESTRUCTORS.iter().map(|(_, a)| *a));
        for addr in externals {
            e.register(addr, |_, _| Ret::default());
        }
        e.register(ALLOCATE, |e, a| word(e.mem.alloc(a[0])));
        for global in [
            MENU_MANAGER,
            TUTORIAL_MENU_GLOBAL,
            STATS_MENU_GLOBAL,
            BODY_PART_CONTROLLER_GLOBAL,
        ] {
            e.map(global, 4);
        }
        e.call_log = Some(vec![]);
        e
    }

    /// The argument words of every logged call to `addr`.
    fn calls(e: &Engine, addr: u32) -> Calls {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(called, _)| *called == addr)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// The logged calls to any of `addrs`, in order.
    fn sequence(e: &Engine, addrs: &[u32]) -> Sequence {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(called, _)| addrs.contains(called))
            .cloned()
            .collect()
    }

    /// A heap block of `size` bytes filled with `0xAA`.
    fn filled(e: &mut Engine, size: u32) -> u32 {
        let block = e.mem.alloc(size);
        e.mem.write(block, &vec![0xaa; size as usize]);
        block
    }

    fn grow_call() -> Vec<u32> {
        vec![MENU_TABLE, MENU_TABLE_COUNT]
    }

    /// Calls the deleting destructor at `addr` with flags 0, then 1: frees
    /// only the second time and returns `this` both times.
    fn check_deleting_destructor(e: &mut Engine, addr: u32, this: u32) {
        assert_eq!(e.call(addr, &args![this, 0u32]).u32(), this);
        assert!(calls(e, FREE).is_empty());
        assert_eq!(e.call(addr, &args![this, 1u32]).u32(), this);
        assert_eq!(calls(e, FREE), vec![vec![this]]);
    }

    #[test]
    fn instance_creates_once_and_only_when_asked() {
        let mut e = engine();
        assert_eq!(e.call(0x0071_e290, &args![false]).u32(), 0);
        assert!(calls(&e, ALLOCATE).is_empty());
        let manager = e.call(0x0071_e290, &args![true]).u32();
        assert_ne!(manager, 0);
        assert_eq!(e.global::<u32>(MENU_MANAGER), manager);
        assert_eq!(calls(&e, ALLOCATE), vec![vec![1]]);
        // The constructor and the instance function both run the body.
        assert_eq!(calls(&e, MENU_TABLE_GROW), vec![grow_call(), grow_call()]);
        assert_eq!(e.call(0x0071_e290, &args![true]).u32(), manager);
        assert_eq!(calls(&e, ALLOCATE).len(), 1);
    }

    #[test]
    fn instance_with_failed_allocation_stays_null() {
        let mut e = engine();
        e.register(ALLOCATE, |_, _| Ret::default());
        assert_eq!(e.call(0x0071_e290, &args![true]).u32(), 0);
        assert_eq!(e.global::<u32>(MENU_MANAGER), 0);
        assert_eq!(calls(&e, MENU_TABLE_GROW), vec![grow_call()]);
    }

    #[test]
    fn manager_constructor_runs_the_body() {
        let mut e = engine();
        let this = e.mem.alloc(4);
        assert_eq!(e.call(0x0071_e340, &args![this]).u32(), this);
        assert_eq!(calls(&e, MENU_TABLE_GROW), vec![grow_call()]);
    }

    #[test]
    fn manager_shutdown_deletes_the_instance() {
        let mut e = engine();
        e.call(0x0071_e360, &args![]);
        assert!(calls(&e, FREE).is_empty());
        let manager = e.mem.alloc(4);
        e.set_global(MENU_MANAGER, manager);
        e.call(0x0071_e360, &args![]);
        assert_eq!(calls(&e, FREE), vec![vec![manager]]);
        assert_eq!(e.global::<u32>(MENU_MANAGER), 0);
    }

    #[test]
    fn manager_deleting_destructor_frees_only_with_the_flag() {
        let mut e = engine();
        let this = e.mem.alloc(4);
        e.set_global(MENU_MANAGER, 0x1234u32);
        assert_eq!(e.call(0x0071_e3b0, &args![this, 2u32]).u32(), this);
        assert_eq!(e.global::<u32>(MENU_MANAGER), 0);
        assert!(calls(&e, FREE).is_empty());
        e.call(0x0071_e3b0, &args![this, 1u32]);
        assert_eq!(calls(&e, FREE), vec![vec![this]]);
    }

    #[test]
    fn manager_destructor_clears_the_global() {
        let mut e = engine();
        e.set_global(MENU_MANAGER, 0x1234u32);
        e.call(0x0071_e3e0, &args![0x5678u32]);
        assert_eq!(e.global::<u32>(MENU_MANAGER), 0);
    }

    #[test]
    fn clear_instance_global() {
        let mut e = engine();
        e.set_global(MENU_MANAGER, 0x1234u32);
        e.call(0x0071_e3f0, &args![]);
        assert_eq!(e.global::<u32>(MENU_MANAGER), 0);
    }

    #[test]
    fn manager_constructor_body_grows_the_table() {
        let mut e = engine();
        e.call(0x0071_e400, &args![0u32]);
        assert_eq!(calls(&e, MENU_TABLE_GROW), vec![grow_call()]);
    }

    #[test]
    fn create_menu_by_class_allocates_and_constructs() {
        let mut e = engine();
        for (_, _, constructor) in MENU_CLASSES {
            e.register(constructor, |_, a| word(a[0] + 1));
        }
        let menu = e.call(0x0071_e420, &args![0u32, 0x3e9u32]).u32();
        assert_eq!(calls(&e, ALLOCATE), vec![vec![0x94]]);
        assert_eq!(calls(&e, 0x007a_8a10), vec![vec![menu - 1]]);
        let menu = e.call(0x0071_e420, &args![0u32, 0x43au32]).u32();
        assert_eq!(calls(&e, ALLOCATE)[1], vec![0x4014]);
        assert_eq!(calls(&e, 0x007b_8d30), vec![vec![menu - 1]]);
        let menu = e.call(0x0071_e420, &args![0u32, 0x423u32]).u32();
        assert_eq!(calls(&e, ALLOCATE)[2], vec![0x54]);
        assert_eq!(calls(&e, 0x0071_f240), vec![vec![menu - 1]]);
        assert!(calls(&e, DEBUG_MESSAGE).is_empty());
    }

    #[test]
    fn create_menu_by_class_rejects_unknown_classes() {
        let mut e = engine();
        for class in [0x3e8u32, 0x3ed, 0x43d, 0, 0xffff_ffff] {
            assert_eq!(e.call(0x0071_e420, &args![0u32, class]).u32(), 0);
        }
        assert!(calls(&e, ALLOCATE).is_empty());
        assert_eq!(
            calls(&e, DEBUG_MESSAGE),
            vec![vec![UNKNOWN_MENU_CLASS_TEXT]; 5]
        );
    }

    #[test]
    fn create_menu_by_class_with_failed_allocation_returns_null() {
        let mut e = engine();
        e.register(ALLOCATE, |_, _| Ret::default());
        e.register(0x0075_f070, |_, _| word(0xdead));
        assert_eq!(e.call(0x0071_e420, &args![0u32, 0x417u32]).u32(), 0);
        assert_eq!(calls(&e, ALLOCATE), vec![vec![0x80]]);
        assert!(calls(&e, 0x0075_f070).is_empty());
    }

    #[test]
    fn tutorial_menu_constructor() {
        let mut e = engine();
        let this = filled(&mut e, 0x54);
        assert_eq!(e.call(0x0071_f240, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), TUTORIAL_MENU_VTABLE);
        assert_eq!(calls(&e, MENU_CONSTRUCT), vec![vec![this]]);
        assert_eq!(calls(&e, MEMSET), vec![vec![this + 0x28, 0, 0x20]]);
        for offset in [0x48, 0x4c, 0x50] {
            assert_eq!(e.mem.u32(this + offset), 0);
        }
    }

    #[test]
    fn tutorial_menu_get_class_is_0x423() {
        let mut e = engine();
        assert_eq!(e.call(0x0071_f2d0, &args![0u32]).u32(), 0x423);
    }

    #[test]
    fn tutorial_menu_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x54);
        check_deleting_destructor(&mut e, 0x0071_f2e0, this);
        assert_eq!(calls(&e, MENU_DESTRUCT).len(), 2);
    }

    #[test]
    fn tutorial_menu_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x54);
        e.set_global(TUTORIAL_MENU_GLOBAL, this);
        e.call(0x0071_f310, &args![this]);
        assert_eq!(e.mem.u32(this), TUTORIAL_MENU_VTABLE);
        assert_eq!(e.global::<u32>(TUTORIAL_MENU_GLOBAL), 0);
        assert_eq!(calls(&e, MENU_DESTRUCT), vec![vec![this]]);
    }

    #[test]
    fn stats_menu_constructor() {
        let mut e = engine();
        let this = filled(&mut e, 0x2a4);
        assert_eq!(e.call(0x0071_f340, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), STATS_MENU_VTABLE);
        assert_eq!(e.mem.u32(this + 0x68), NAMED_EFFECTS_LIST_VTABLE);
        assert_eq!(e.mem.u32(this + 0x84), STATUS_EFFECT_LIST_VTABLE);
        let mut expected: Sequence = vec![
            (MENU_CONSTRUCT, vec![this]),
            (
                VECTOR_CONSTRUCT,
                vec![this + 0x28, 0x10, 4, 0x0071_f4a0, 0x0071_f4e0],
            ),
            (LIST_CONSTRUCT, vec![this + 0x6c]),
            (LIST_CONSTRUCT, vec![this + 0x88]),
        ];
        for (offset, constructor) in STATS_LIST_BOX_CONSTRUCTORS {
            expected.push((constructor, vec![this + offset]));
        }
        expected.push((MEMSET, vec![this + 0x90, 0, 0xf0]));
        let mut watched = vec![MENU_CONSTRUCT, VECTOR_CONSTRUCT, LIST_CONSTRUCT, MEMSET];
        watched.extend(STATS_LIST_BOX_CONSTRUCTORS.iter().map(|(_, a)| *a));
        assert_eq!(sequence(&e, &watched), expected);
        for offset in [0x74, 0x78, 0x7c, 0x80] {
            assert_eq!(e.mem.u32(this + offset), 0);
        }
        assert_eq!(e.mem.u8(this + 0x2a0), 0);
        assert_eq!(e.mem.u8(this + 0x2a1), 0);
        assert_eq!(e.mem.u8(this + 0x2a2), 0xaa);
    }

    #[test]
    fn healing_option_constructor() {
        let mut e = engine();
        let this = filled(&mut e, 0x10);
        assert_eq!(e.call(0x0071_f4a0, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), 0);
        assert_eq!(e.mem.u32(this + 4), 0);
        assert_eq!(e.mem.u32(this + 8), 0);
        assert_eq!(e.mem.u32(this + 0xc), 0x0071_f520);
    }

    #[test]
    fn healing_option_destructor_deletes_the_item() {
        let mut e = engine();
        let this = e.mem.alloc(0x10);
        e.call(0x0071_f4e0, &args![this]);
        assert!(calls(&e, ITEM_DELETE).is_empty());
        e.mem.set_u32(this + 8, 0x4444);
        e.call(0x0071_f4e0, &args![this]);
        assert_eq!(calls(&e, ITEM_DELETE), vec![vec![0x4444, 1]]);
    }

    #[test]
    fn healing_option_condition_needs_an_item() {
        let mut e = engine();
        let option = e.mem.alloc(0x10);
        assert!(!e.call(0x0071_f520, &args![option]).bool());
        e.mem.set_u32(option + 8, 0x100);
        assert!(e.call(0x0071_f520, &args![option]).bool());
    }

    #[test]
    fn named_effects_list_constructor() {
        let mut e = engine();
        let this = e.mem.alloc(0xc);
        assert_eq!(e.call(0x0071_f540, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), NAMED_EFFECTS_LIST_VTABLE);
        assert_eq!(calls(&e, LIST_CONSTRUCT), vec![vec![this + 4]]);
    }

    #[test]
    fn named_effects_list_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0xc);
        e.call(0x0071_f570, &args![this]);
        assert_eq!(e.mem.u32(this), NAMED_EFFECTS_LIST_VTABLE);
        assert_eq!(
            sequence(&e, &[NAMED_EFFECTS_CLEAR, LIST_DESTRUCT]),
            vec![
                (NAMED_EFFECTS_CLEAR, vec![this]),
                (LIST_DESTRUCT, vec![this + 4])
            ]
        );
    }

    #[test]
    fn status_effect_list_constructor() {
        let mut e = engine();
        let this = e.mem.alloc(0xc);
        assert_eq!(e.call(0x0071_f5d0, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), STATUS_EFFECT_LIST_VTABLE);
        assert_eq!(calls(&e, LIST_CONSTRUCT), vec![vec![this + 4]]);
    }

    #[test]
    fn status_effect_list_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0xc);
        e.call(0x0071_f600, &args![this]);
        assert_eq!(e.mem.u32(this), STATUS_EFFECT_LIST_VTABLE);
        assert_eq!(
            sequence(&e, &[STATUS_EFFECTS_CLEAR, LIST_DESTRUCT]),
            vec![
                (STATUS_EFFECTS_CLEAR, vec![this]),
                (LIST_DESTRUCT, vec![this + 4])
            ]
        );
    }

    #[test]
    fn stats_menu_get_class_is_0x3eb() {
        let mut e = engine();
        assert_eq!(e.call(0x0071_f660, &args![0u32]).u32(), 0x3eb);
    }

    #[test]
    fn named_effects_list_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0xc);
        check_deleting_destructor(&mut e, 0x0071_f670, this);
        assert_eq!(calls(&e, NAMED_EFFECTS_CLEAR).len(), 2);
    }

    #[test]
    fn status_effect_list_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0xc);
        check_deleting_destructor(&mut e, 0x0071_f6a0, this);
        assert_eq!(calls(&e, STATUS_EFFECTS_CLEAR).len(), 2);
    }

    #[test]
    fn stats_menu_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x2a4);
        check_deleting_destructor(&mut e, 0x0071_f6d0, this);
        assert_eq!(calls(&e, MENU_DESTRUCT).len(), 2);
    }

    #[test]
    fn stats_menu_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x2a4);
        let first = e.mem.alloc(0x1c);
        let last = e.mem.alloc(0x1c);
        e.mem.set_u32(this + 0x74, first);
        e.mem.set_u32(this + 0x78, last);
        e.set_global(STATS_MENU_GLOBAL, this);
        e.call(0x0071_f700, &args![this]);
        assert_eq!(e.mem.u32(this), STATS_MENU_VTABLE);
        assert_eq!(e.global::<u32>(STATS_MENU_GLOBAL), 0);
        // The two effects, +0x74 first and +0x78 last, then the members.
        let mut expected: Sequence = vec![
            (NAMED_EFFECTS_CLEAR, vec![first + 0x10]),
            (STATUS_DATA_FLUSH, vec![first + 4]),
            (FREE, vec![first]),
            (NAMED_EFFECTS_CLEAR, vec![last + 0x10]),
            (STATUS_DATA_FLUSH, vec![last + 4]),
            (FREE, vec![last]),
        ];
        for (offset, destructor) in STATS_LIST_BOX_DESTRUCTORS {
            expected.push((destructor, vec![this + offset]));
        }
        expected.push((STATUS_EFFECTS_CLEAR, vec![this + 0x84]));
        expected.push((NAMED_EFFECTS_CLEAR, vec![this + 0x68]));
        expected.push((VECTOR_DESTRUCT, vec![this + 0x28, 0x10, 4, 0x0071_f4e0]));
        expected.push((MENU_DESTRUCT, vec![this]));
        let mut watched = vec![
            STATUS_DATA_FLUSH,
            FREE,
            STATUS_EFFECTS_CLEAR,
            NAMED_EFFECTS_CLEAR,
            VECTOR_DESTRUCT,
            MENU_DESTRUCT,
        ];
        watched.extend(STATS_LIST_BOX_DESTRUCTORS.iter().map(|(_, a)| *a));
        assert_eq!(sequence(&e, &watched), expected);
    }

    #[test]
    fn body_part_controller_global_is_deleted() {
        let mut e = engine();
        e.call(0x0071_f8c0, &args![]);
        assert!(calls(&e, FREE).is_empty());
        let controller = e.mem.alloc(0x4c);
        e.set_global(BODY_PART_CONTROLLER_GLOBAL, controller);
        e.call(0x0071_f8c0, &args![]);
        assert_eq!(calls(&e, FREE), vec![vec![controller]]);
        assert_eq!(calls(&e, LIST_CLEAR).len(), 6);
        assert_eq!(e.global::<u32>(BODY_PART_CONTROLLER_GLOBAL), 0);
    }

    #[test]
    fn body_part_controller_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x4c);
        check_deleting_destructor(&mut e, 0x0071_f900, this);
        assert_eq!(calls(&e, LIST_CLEAR).len(), 12);
    }

    #[test]
    fn body_part_controller_destructor_empties_six_lists() {
        let mut e = engine();
        let this = e.mem.alloc(0x4c);
        e.call(0x0071_f930, &args![this]);
        let cleared: Vec<u32> = calls(&e, LIST_CLEAR).iter().map(|c| c[0]).collect();
        let expected: Vec<u32> = (0..6).map(|i| this + 4 + 8 * i).collect();
        assert_eq!(cleared, expected);
        assert_eq!(
            calls(&e, VECTOR_DESTRUCT),
            vec![vec![this + 4, 8, 6, LIST_DESTRUCT]]
        );
    }

    #[test]
    fn status_effect_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x1c);
        check_deleting_destructor(&mut e, 0x0071_f9c0, this);
        assert_eq!(calls(&e, STATUS_DATA_FLUSH).len(), 2);
    }

    #[test]
    fn status_effect_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x1c);
        e.call(0x0071_f9f0, &args![this]);
        assert_eq!(e.mem.u32(this + 0x10), NAMED_EFFECTS_LIST_VTABLE);
        assert_eq!(
            sequence(&e, &[NAMED_EFFECTS_CLEAR, LIST_DESTRUCT, STATUS_DATA_FLUSH]),
            vec![
                (NAMED_EFFECTS_CLEAR, vec![this + 0x10]),
                (LIST_DESTRUCT, vec![this + 0x14]),
                (STATUS_DATA_FLUSH, vec![this + 4]),
            ]
        );
    }

    #[test]
    fn stats_menu_attach_tile_takes_ids_0_to_59() {
        let mut e = engine();
        let this = e.mem.alloc(0x2a4);
        e.call(0x0071_fa50, &args![this, 0i32, 0x11u32]);
        e.call(0x0071_fa50, &args![this, 59i32, 0x22u32]);
        e.call(0x0071_fa50, &args![this, 60i32, 0x33u32]);
        e.call(0x0071_fa50, &args![this, -1i32, 0x44u32]);
        assert_eq!(e.mem.u32(this + 0x90), 0x11);
        assert_eq!(e.mem.u32(this + 0x90 + 59 * 4), 0x22);
        assert_eq!(e.mem.u32(this + 0x180), 0);
        assert_eq!(e.mem.u32(this + 0x8c), 0);
    }

    #[test]
    fn text_edit_menu_constructor() {
        let mut e = engine();
        let this = filled(&mut e, 0x5c);
        assert_eq!(e.call(0x0071_fa80, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), TEXT_EDIT_MENU_VTABLE);
        assert_eq!(e.mem.u32(this + 0x58), 0);
        assert_eq!(
            sequence(
                &e,
                &[MENU_CONSTRUCT, TEXT_EDITOR_CONSTRUCT, MEMSET, STRING_SET]
            ),
            vec![
                (MENU_CONSTRUCT, vec![this]),
                (TEXT_EDITOR_CONSTRUCT, vec![this + 0x34]),
                (MEMSET, vec![this + 0x28, 0, 0xc]),
                (STRING_SET, vec![ENTERED_TEXT, 0, 0]),
            ]
        );
    }

    #[test]
    fn text_edit_menu_get_class_is_0x41b() {
        let mut e = engine();
        assert_eq!(e.call(0x0071_fb20, &args![0u32]).u32(), 0x41b);
    }

    #[test]
    fn text_edit_menu_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x5c);
        check_deleting_destructor(&mut e, 0x0071_fb30, this);
        assert_eq!(calls(&e, TEXT_EDITOR_DESTRUCT).len(), 2);
    }

    #[test]
    fn text_edit_menu_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x5c);
        e.call(0x0071_fb60, &args![this]);
        assert_eq!(e.mem.u32(this), TEXT_EDIT_MENU_VTABLE);
        assert_eq!(
            sequence(&e, &[TEXT_EDITOR_DESTRUCT, MENU_DESTRUCT]),
            vec![
                (TEXT_EDITOR_DESTRUCT, vec![this + 0x34]),
                (MENU_DESTRUCT, vec![this])
            ]
        );
    }

    #[test]
    fn text_edit_menu_attach_tile_takes_ids_0_to_2() {
        let mut e = engine();
        let this = e.mem.alloc(0x5c);
        e.call(0x0071_fbc0, &args![this, 2i32, 0x11u32]);
        e.call(0x0071_fbc0, &args![this, 3i32, 0x22u32]);
        e.call(0x0071_fbc0, &args![this, -1i32, 0x33u32]);
        assert_eq!(e.mem.u32(this + 0x30), 0x11);
        assert_eq!(e.mem.u32(this + 0x34), 0);
        assert_eq!(e.mem.u32(this + 0x24), 0);
    }

    #[test]
    fn surgery_menu_constructor() {
        let mut e = engine();
        let this = filled(&mut e, 0x160);
        assert_eq!(e.call(0x0071_fbf0, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), SURGERY_MENU_VTABLE);
        let empty: Vec<u32> = calls(&e, EMPTY_CONSTRUCTOR).iter().map(|c| c[0]).collect();
        let expected: Vec<u32> = [0x74, 0x7c, 0xf8, 0x104, 0x128, 0x150]
            .iter()
            .map(|o| this + o)
            .collect();
        assert_eq!(empty, expected);
        assert_eq!(
            calls(&e, SOUND_HANDLE_CONSTRUCT),
            vec![vec![this + 0x94], vec![this + 0xa0]]
        );
        assert_eq!(
            calls(&e, VECTOR_CONSTRUCT),
            vec![vec![
                this + 0xc8,
                4,
                6,
                POINTER_CONSTRUCT_DEFAULT,
                POINTER_DESTRUCT
            ]]
        );
        assert_eq!(
            calls(&e, POINTER_CONSTRUCT),
            vec![vec![this + 0xf0, 0], vec![this + 0xf4, 0]]
        );
        assert_eq!(calls(&e, MEMSET), vec![vec![this + 0x2c, 0, 0x48]]);
        let assigned: Vec<u32> = calls(&e, POINTER_ASSIGN).iter().map(|c| c[0]).collect();
        let mut expected = SURGERY_STATIC_POINTERS.to_vec();
        expected.extend([this + 0xf0, this + 0xf4]);
        expected.extend((0..6).map(|i| this + 0xc8 + 4 * i));
        assert_eq!(assigned, expected);
        for offset in [
            0xe0, 0xe4, 0xbc, 0xe8, 0xec, 0x84, 0xac, 0x14c, 0x88, 0x8c, 0xb0, 0xb4, 0xb8, 0x7c,
            0x80, 0x74, 0x78, 0x28,
        ] {
            assert_eq!(e.mem.u32(this + offset), 0, "+{offset:x}");
        }
        for offset in [0x90, 0x91, 0x15c, 0x15d] {
            assert_eq!(e.mem.u8(this + offset), 0);
        }
        assert_eq!(e.mem.u8(this + 0x92), 0xaa);
        assert_eq!(e.mem.u32(this + 0xc0), 0x1b);
        assert_eq!(e.mem.u32(this + 0xc4), 1);
    }

    #[test]
    fn surgery_menu_get_class_is_0x41e() {
        let mut e = engine();
        assert_eq!(e.call(0x0071_feb0, &args![0u32]).u32(), 0x41e);
    }

    #[test]
    fn surgery_menu_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x160);
        check_deleting_destructor(&mut e, 0x0071_fec0, this);
        assert_eq!(calls(&e, MENU_DESTRUCT).len(), 2);
    }

    #[test]
    fn surgery_menu_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x160);
        e.call(0x0071_fef0, &args![this]);
        assert_eq!(e.mem.u32(this), SURGERY_MENU_VTABLE);
        let mut expected: Sequence = SURGERY_STATIC_POINTERS
            .iter()
            .map(|g| (POINTER_ASSIGN, vec![*g, 0]))
            .collect();
        expected.extend([
            (POINTER_ASSIGN, vec![this + 0xf0, 0]),
            (POINTER_DESTRUCT, vec![this + 0xf4]),
            (POINTER_DESTRUCT, vec![this + 0xf0]),
            (VECTOR_DESTRUCT, vec![this + 0xc8, 4, 6, POINTER_DESTRUCT]),
            (EMPTY_FUNCTION, vec![this + 0xa0]),
            (EMPTY_FUNCTION, vec![this + 0x94]),
            (MENU_DESTRUCT, vec![this]),
        ]);
        let watched = [
            POINTER_ASSIGN,
            POINTER_DESTRUCT,
            VECTOR_DESTRUCT,
            EMPTY_FUNCTION,
            MENU_DESTRUCT,
        ];
        assert_eq!(sequence(&e, &watched), expected);
    }

    #[test]
    fn surgery_menu_attach_tile_takes_ids_1_to_18() {
        let mut e = engine();
        let this = e.mem.alloc(0x160);
        e.call(0x0071_ffe0, &args![this, 1i32, 0x11u32]);
        e.call(0x0071_ffe0, &args![this, 18i32, 0x22u32]);
        e.call(0x0071_ffe0, &args![this, 0i32, 0x33u32]);
        e.call(0x0071_ffe0, &args![this, 19i32, 0x44u32]);
        assert_eq!(e.mem.u32(this + 0x2c), 0x11);
        assert_eq!(e.mem.u32(this + 0x2c + 17 * 4), 0x22);
        assert_eq!(e.mem.u32(this + 0x28), 0);
        assert_eq!(e.mem.u32(this + 0x2c + 18 * 4), 0);
    }

    #[test]
    fn surgery_tile_index_maps_ids_to_slots() {
        let mut e = engine();
        assert_eq!(e.call(0x0072_0010, &args![0u32, 1i32]).i32(), 0);
        assert_eq!(e.call(0x0072_0010, &args![0u32, 18i32]).i32(), 17);
        assert_eq!(e.call(0x0072_0010, &args![0u32, 0i32]).i32(), 18);
        assert_eq!(e.call(0x0072_0010, &args![0u32, 19i32]).i32(), 18);
        assert_eq!(e.call(0x0072_0010, &args![0u32, -5i32]).i32(), 18);
    }

    #[test]
    fn vats_menu_constructor() {
        let mut e = engine();
        let this = filled(&mut e, 0x144);
        assert_eq!(e.call(0x0072_0040, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), VATS_MENU_VTABLE);
        let watched = [
            MENU_CONSTRUCT,
            VATS_ACTION_LIST_CONSTRUCT,
            VATS_COMMAND_CONSTRUCT,
            MEMSET,
        ];
        assert_eq!(
            sequence(&e, &watched),
            vec![
                (MENU_CONSTRUCT, vec![this]),
                (VATS_ACTION_LIST_CONSTRUCT, vec![this + 0xb0]),
                (VATS_COMMAND_CONSTRUCT, vec![this + 0x11c]),
                (MEMSET, vec![this + 0x28, 0, 0x80]),
            ]
        );
        assert_eq!(e.mem.u8(this + 0xf8), 0);
        assert_eq!(e.mem.u8(this + 0xf9), 0xaa);
        assert_eq!(e.mem.u8(this + 0x104), 0);
        assert_eq!(e.mem.u32(this + 0x108), 0);
    }

    #[test]
    fn vats_menu_get_class_is_0x420() {
        let mut e = engine();
        assert_eq!(e.call(0x0072_0100, &args![0u32]).u32(), 0x420);
    }

    #[test]
    fn vats_menu_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x144);
        check_deleting_destructor(&mut e, 0x0072_0110, this);
        assert_eq!(calls(&e, VATS_MENU_DESTRUCT), vec![vec![this], vec![this]]);
    }

    #[test]
    fn vats_menu_attach_tile_takes_ids_1_to_31() {
        let mut e = engine();
        let this = e.mem.alloc(0x144);
        e.call(0x0072_0140, &args![this, 1i32, 0x11u32]);
        e.call(0x0072_0140, &args![this, 31i32, 0x22u32]);
        e.call(0x0072_0140, &args![this, 0i32, 0x33u32]);
        e.call(0x0072_0140, &args![this, 32i32, 0x44u32]);
        assert_eq!(e.mem.u32(this + 0x2c), 0x11);
        assert_eq!(e.mem.u32(this + 0xa4), 0x22);
        assert_eq!(e.mem.u32(this + 0x28), 0);
        assert_eq!(e.mem.u32(this + 0xa8), 0);
    }

    #[test]
    fn special_book_menu_constructor() {
        let mut e = engine();
        let this = filled(&mut e, 0x1e4);
        assert_eq!(e.call(0x0072_0170, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), SPECIAL_BOOK_MENU_VTABLE);
        let built = calls(&e, VECTOR_CONSTRUCT);
        let arrays: Vec<(u32, u32)> = built.iter().map(|c| (c[0] - this, c[2])).collect();
        assert_eq!(
            arrays,
            vec![
                (0x60, 9),
                (0x180, 0xb),
                (0x1ac, 4),
                (0x1bc, 2),
                (0x1c4, 2),
                (0x1cc, 2),
                (0x1d4, 2),
                (0x1dc, 2)
            ]
        );
        assert_eq!(
            built[0],
            vec![this + 0x60, 0x20, 9, 0x0072_0360, 0x0072_03c0]
        );
        assert_eq!(
            built[1],
            vec![
                this + 0x180,
                4,
                0xb,
                POINTER_CONSTRUCT_DEFAULT,
                POINTER_DESTRUCT
            ]
        );
        assert_eq!(calls(&e, MEMSET), vec![vec![this + 0x28, 0, 0x20]]);
        assert_eq!(e.mem.u32(this + 0x5c), 0);
        assert_eq!(e.mem.u32(this + 0x48), 0);
        assert_eq!(e.mem.u32(this + 0x4c), 1);
        assert_eq!(e.mem.u32(this + 0x50), 0);
        assert_eq!(e.mem.u8(this + 0x54), 1);
        assert_eq!(e.mem.u8(this + 0x55), 0xaa);
        assert_eq!(e.mem.u32(this + 0x58), 0);
    }

    #[test]
    fn special_book_menu_get_class_is_0x424() {
        let mut e = engine();
        assert_eq!(e.call(0x0072_0320, &args![0u32]).u32(), 0x424);
    }

    #[test]
    fn special_book_menu_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x1e4);
        check_deleting_destructor(&mut e, 0x0072_0330, this);
        assert_eq!(calls(&e, MENU_DESTRUCT).len(), 2);
    }

    #[test]
    fn special_book_page_constructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x20);
        assert_eq!(e.call(0x0072_0360, &args![this]).u32(), this);
        assert_eq!(
            sequence(&e, &[BOOK_BUTTONS_CONSTRUCT, BOOK_TEXT_ITEMS_CONSTRUCT]),
            vec![
                (BOOK_BUTTONS_CONSTRUCT, vec![this]),
                (BOOK_TEXT_ITEMS_CONSTRUCT, vec![this + 0x10])
            ]
        );
    }

    #[test]
    fn special_book_page_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x20);
        e.call(0x0072_03c0, &args![this]);
        assert_eq!(
            sequence(&e, &[BOOK_TEXT_ITEMS_DESTRUCT, BOOK_BUTTONS_DESTRUCT]),
            vec![
                (BOOK_TEXT_ITEMS_DESTRUCT, vec![this + 0x10]),
                (BOOK_BUTTONS_DESTRUCT, vec![this])
            ]
        );
    }

    #[test]
    fn special_book_menu_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x1e4);
        e.call(0x0072_0420, &args![this]);
        assert_eq!(e.mem.u32(this), SPECIAL_BOOK_MENU_VTABLE);
        let destroyed = calls(&e, VECTOR_DESTRUCT);
        let arrays: Vec<(u32, u32)> = destroyed.iter().map(|c| (c[0] - this, c[2])).collect();
        assert_eq!(
            arrays,
            vec![
                (0x1dc, 2),
                (0x1d4, 2),
                (0x1cc, 2),
                (0x1c4, 2),
                (0x1bc, 2),
                (0x1ac, 4),
                (0x180, 0xb),
                (0x60, 9)
            ]
        );
        assert_eq!(destroyed[0], vec![this + 0x1dc, 4, 2, POINTER_DESTRUCT]);
        assert_eq!(destroyed[7], vec![this + 0x60, 0x20, 9, 0x0072_03c0]);
        assert_eq!(calls(&e, MENU_DESTRUCT), vec![vec![this]]);
    }

    #[test]
    fn love_tester_menu_constructor() {
        let mut e = engine();
        let this = filled(&mut e, 0x230);
        assert_eq!(e.call(0x0072_0550, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), LOVE_TESTER_MENU_VTABLE);
        assert_eq!(
            calls(&e, LOVE_TESTER_GLOWS_CONSTRUCT),
            vec![vec![this + 0x74]]
        );
        // LightBulbs (+0x64), then the global page's text items.
        assert_eq!(
            calls(&e, LOVE_TESTER_TEXT_ITEMS_CONSTRUCT),
            vec![vec![this + 0x64], vec![this + 0x1b4]]
        );
        assert_eq!(
            calls(&e, LOVE_TESTER_BUTTONS_CONSTRUCT),
            vec![vec![this + 0x1a4]]
        );
        let built = calls(&e, VECTOR_CONSTRUCT);
        let arrays: Vec<(u32, u32)> = built.iter().map(|c| (c[0] - this, c[2])).collect();
        assert_eq!(
            arrays,
            vec![
                (0x84, 9),
                (0x1c4, 0xb),
                (0x1f0, 4),
                (0x200, 2),
                (0x208, 2),
                (0x210, 2),
                (0x218, 2),
                (0x220, 2),
                (0x228, 2)
            ]
        );
        assert_eq!(
            built[0],
            vec![this + 0x84, 0x20, 9, 0x0072_07a0, 0x0072_0800]
        );
        assert_eq!(calls(&e, MEMSET), vec![vec![this + 0x28, 0, 0x20]]);
        for offset in [0x5c, 0x60, 0x48, 0x50, 0x58] {
            assert_eq!(e.mem.u32(this + offset), 0);
        }
        assert_eq!(e.mem.u32(this + 0x4c), 1);
        assert_eq!(e.mem.u8(this + 0x54), 1);
    }

    #[test]
    fn love_tester_menu_get_class_is_0x432() {
        let mut e = engine();
        assert_eq!(e.call(0x0072_0760, &args![0u32]).u32(), 0x432);
    }

    #[test]
    fn love_tester_menu_deleting_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x230);
        check_deleting_destructor(&mut e, 0x0072_0770, this);
        assert_eq!(calls(&e, MENU_DESTRUCT).len(), 2);
    }

    #[test]
    fn love_tester_page_constructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x20);
        assert_eq!(e.call(0x0072_07a0, &args![this]).u32(), this);
        let watched = [
            LOVE_TESTER_BUTTONS_CONSTRUCT,
            LOVE_TESTER_TEXT_ITEMS_CONSTRUCT,
        ];
        assert_eq!(
            sequence(&e, &watched),
            vec![
                (LOVE_TESTER_BUTTONS_CONSTRUCT, vec![this]),
                (LOVE_TESTER_TEXT_ITEMS_CONSTRUCT, vec![this + 0x10])
            ]
        );
    }

    #[test]
    fn love_tester_page_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x20);
        e.call(0x0072_0800, &args![this]);
        let watched = [
            LOVE_TESTER_TEXT_ITEMS_DESTRUCT,
            LOVE_TESTER_BUTTONS_DESTRUCT,
        ];
        assert_eq!(
            sequence(&e, &watched),
            vec![
                (LOVE_TESTER_TEXT_ITEMS_DESTRUCT, vec![this + 0x10]),
                (LOVE_TESTER_BUTTONS_DESTRUCT, vec![this])
            ]
        );
    }

    #[test]
    fn love_tester_menu_destructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x230);
        e.call(0x0072_0860, &args![this]);
        assert_eq!(e.mem.u32(this), LOVE_TESTER_MENU_VTABLE);
        let watched = [
            VECTOR_DESTRUCT,
            LOVE_TESTER_TEXT_ITEMS_DESTRUCT,
            LOVE_TESTER_BUTTONS_DESTRUCT,
            LOVE_TESTER_GLOWS_DESTRUCT,
            MENU_DESTRUCT,
        ];
        let seen: Vec<(u32, u32)> = sequence(&e, &watched)
            .iter()
            .map(|(addr, words)| (*addr, words[0] - this))
            .collect();
        assert_eq!(
            seen,
            vec![
                (VECTOR_DESTRUCT, 0x228),
                (VECTOR_DESTRUCT, 0x220),
                (VECTOR_DESTRUCT, 0x218),
                (VECTOR_DESTRUCT, 0x210),
                (VECTOR_DESTRUCT, 0x208),
                (VECTOR_DESTRUCT, 0x200),
                (VECTOR_DESTRUCT, 0x1f0),
                (VECTOR_DESTRUCT, 0x1c4),
                (LOVE_TESTER_TEXT_ITEMS_DESTRUCT, 0x1b4),
                (LOVE_TESTER_BUTTONS_DESTRUCT, 0x1a4),
                (VECTOR_DESTRUCT, 0x84),
                (LOVE_TESTER_GLOWS_DESTRUCT, 0x74),
                (LOVE_TESTER_TEXT_ITEMS_DESTRUCT, 0x64),
                (MENU_DESTRUCT, 0),
            ]
        );
        let destroyed = calls(&e, VECTOR_DESTRUCT);
        assert_eq!(destroyed[6], vec![this + 0x1f0, 4, 4, POINTER_DESTRUCT]);
        assert_eq!(destroyed[8], vec![this + 0x84, 0x20, 9, 0x0072_0800]);
    }

    #[test]
    fn eight_tile_attach_takes_ids_0_to_7() {
        let mut e = engine();
        let this = e.mem.alloc(0x54);
        e.call(0x0072_09e0, &args![this, 0i32, 0x11u32]);
        e.call(0x0072_09e0, &args![this, 7i32, 0x22u32]);
        e.call(0x0072_09e0, &args![this, 8i32, 0x33u32]);
        e.call(0x0072_09e0, &args![this, -1i32, 0x44u32]);
        assert_eq!(e.mem.u32(this + 0x28), 0x11);
        assert_eq!(e.mem.u32(this + 0x44), 0x22);
        assert_eq!(e.mem.u32(this + 0x48), 0);
        assert_eq!(e.mem.u32(this + 0x24), 0);
    }

    #[test]
    fn quantity_menu_constructor() {
        let mut e = engine();
        let this = e.mem.alloc(0x44);
        assert_eq!(e.call(0x0072_0a10, &args![this]).u32(), this);
        assert_eq!(e.mem.u32(this), QUANTITY_MENU_VTABLE);
        assert_eq!(calls(&e, MENU_CONSTRUCT), vec![vec![this]]);
    }

    #[test]
    // Three generated vectors match native 00726070 on the executable/CPU
    // configuration recorded in docs/ENGINE_STATE.md, including untouched guards.
    fn folded_word_getter_preserves_next_pointer_or_signed_count_bits() {
        let mut e = Engine::new();
        let object = e.mem.alloc(12);
        for word in [0u32, 0x1234_5678, (-2i32) as u32] {
            e.mem.set_u32(object, 0xaaaa_aaaa);
            e.mem.set_u32(object + 4, word);
            e.mem.set_u32(object + 8, 0xbbbb_bbbb);
            let before = e.mem.bytes(object, 12);
            assert_eq!(e.call(0x00726070, &args![object]).u32(), word);
            assert_eq!(e.mem.bytes(object, 12), before);
        }
    }
}
