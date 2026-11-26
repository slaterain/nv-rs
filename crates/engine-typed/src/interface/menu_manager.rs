//! The menu manager unit (the Xbox PDB's `fallout/interface/menumanager` source) from its start
//! up to `00721060` (the translation is engine-fallout-interface's `menumanager.rs`, which these
//! functions replace at the same addresses): `MenuManager` (its instance, constructor, destructors,
//! the menu factory, `ReloadMenu`, `DeleteMenu`, `GetFrontmostMenu`, `IsMenuOpen`) and
//! the constructors, destructors, `GetClass` and `AttachTileByID` of the menus the factory makes
//! that the compiler emitted here (tutorial, stats, text edit, surgery, VATS, S.P.E.C.I.A.L. book,
//! love tester and quantity menus).
//!
//! **Native.** The exe's order of work: its calls by address of functions no family types (the
//! `Menu` base's destructor, the lists' destructors and clears, the smart pointer assignment, the
//! timers, the other menus' creators), its virtual calls (a menu's deleting destructor and class,
//! whatever menu it is). What it calls that is plain stores or reads is done here with no call
//! ([`IN_PLACE`] and the interface unit's [`in_place::ADDRESSES`]): the `Menu` base's, the
//! 8-byte list's and the text edit menu's editor's constructors, `memset` of a menu's tile
//! pointers, the sound handle's and a null smart pointer's constructors (and the array
//! constructor over null smart pointers or over the healing options), the empty constructors
//! (`006815c0`, which answers its argument), the menus by class, the manager and its menus root, a list node's item and next, a menu's tile and
//! state, the state stored (`007037c0`), the start menu's class (`0070ede0`, 0x3f5) and the map
//! menu's (`00720df0`, 0x3ff). The family's functions call one another through `typed_call!`
//! (the stats and VATS menus' list boxes and the pages' simple arrays are
//! [`super::menu_manager_p2`]'s), as do the interface unit's menu creators; the tile's float
//! getter and test (`00a011b0`, `00a01230`) and the smart pointer destructor (`0045cec0`) are the
//! dispatch family's.

use super::interface_cpp::in_place::{self, manager, menu_by_class, menus_root};
use super::interface_cpp::{
    fn_00704140, fn_007048c0, fn_00704be0, interface_create_item_mod_menu,
    interface_create_repair_menu,
};
use super::menu_manager_p2::{
    fn_00721060, fn_00721150, fn_00721850, fn_00721920, fn_00722000, fn_007220d0, fn_007227b0,
    fn_00722880, fn_00722f60, fn_00723070, fn_00723750, fn_00723f00, fn_00723f30, fn_00723f50,
    fn_00723f80, fn_00723fa0, fn_00723fd0, fn_00723ff0, fn_00724020, fn_00724040, fn_00724070,
};
use crate::dispatch;
use engine_core::prelude::*;
use engine_core::typed_call;

/// What this file does in place besides [`in_place::ADDRESSES`]: plain reads and stores the exe
/// calls (the differential tests leave these out of the translation's calls).
/// (`006815c0`, which answers its argument, is in `interface_cpp_p2::IN_PLACE`.)
pub const IN_PLACE: &[u32] = &[
    0x0041_a250, // the sound handle's constructor: -1, a zero byte, 0
    0x0063_3c90, // a smart pointer's constructor, here always with null
    0x0066_94e0, // the default one (null), the array constructor's element
    0x0063_17a0, // a list node's item: the node's +8
    0x007b_52d0, // a list node's next: its first word
    0x00a1_c4a0, // the `Menu` base's constructor: plain stores
    0x0096_a2d0, // the 8-byte list's constructor: two zero words
    0x0070_ede0, // the start menu's class, 0x3f5
    0x004f_d3a0, // the word at +0x74 (the container menu's)
    0x0067_1d10, // the word at +0x30 (the book menu's reference)
    0x007a_f430, // the word at +0x20 (the reference's object)
    0x0071_6980, // the text edit menu's editor constructor: two empty strings and stores
];

/// The array constructor (`eh vector constructor iterator`): done in place where every element's
/// constructor is a plain store (a null smart pointer, a healing option), else by address.
pub const VECTOR_CONSTRUCT: u32 = 0x00ec_782f;

/// The allocator and its free (cdecl).
const ALLOCATE: u32 = 0x0040_1000;
const FREE: u32 = 0x0040_1030;
/// The array destructor (`eh vector destructor iterator`, stdcall: array, size, count, destructor).
const VECTOR_DESTRUCT: u32 = 0x00ec_5fce;
/// The debug message print (an `overridable.tsv` row) and the unknown class's text.
const DEBUG_MESSAGE: u32 = 0x005b_5e40;
const UNKNOWN_MENU_CLASS_TEXT: u32 = 0x0106_ff64;
/// The `Menu` base's vtable (its constructor `00a1c4a0` stores it, [`menu_base`]) and its destructor
/// (thiscall; JIP LN's menu state site is in it, so it stays a call).
const MENU_VTABLE: u32 = 0x0109_5484;
const MENU_DESTRUCT: u32 = 0x00a1_c520;

/// `MenuManager`'s single instance, and the table its constructor body grows to `0x54` entries
/// (`0096ad30`).
const MENU_MANAGER: u32 = 0x011d_8d54;
const MENU_TABLE: u32 = 0x011f_3508;
const MENU_TABLE_GROW: u32 = 0x0096_ad30;
const MENU_TABLE_COUNT: u32 = 0x54;

/// The vtables (the class from the RTTI type descriptor before each).
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

/// The globals the destructors clear: the tutorial menu, the stats menu, the body part controller
/// (`0071f8c0` deletes it) and the quantity menu; the entered text the text edit menu empties; the
/// surgery menu's three static smart pointers.
const TUTORIAL_MENU_GLOBAL: u32 = 0x011d_b088;
const STATS_MENU_GLOBAL: u32 = 0x011d_ace0;
const BODY_PART_CONTROLLER_GLOBAL: u32 = 0x011d_ace4;
const QUANTITY_MENU_GLOBAL: u32 = 0x011d_a618;
const ENTERED_TEXT: u32 = 0x011d_aed8;
const SURGERY_STATIC_POINTERS: [u32; 3] = [0x011d_adb4, 0x011d_ade8, 0x011d_ae5c];

/// A healing option's inventory item delete (`004459e0`, thiscall with a flag).
const ITEM_DELETE: u32 = 0x0044_59e0;
/// The 8-byte list's destructor and clear (thiscall; its constructor is [`empty_list`]).
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
const LIST_CLEAR: u32 = 0x0047_0470;
/// The text edit menu's editor destructor, and `BSString`'s set (string, text, length).
const TEXT_EDITOR_DESTRUCT: u32 = 0x0071_6a10;
const STRING_SET: u32 = 0x0040_37f0;
/// The smart pointer's assignment (`NiPointer::operator=`: the old object released, the new one
/// held) and destructor (the dispatch family's `0045cec0`), and the default constructor the array
/// constructor is given.
const POINTER_ASSIGN: u32 = 0x0066_b0d0;
const POINTER_DESTRUCT: u32 = 0x0045_cec0;
const POINTER_CONSTRUCT_DEFAULT: u32 = 0x0066_94e0;
/// The VATS menu's command and destructor.
const VATS_COMMAND_CONSTRUCT: u32 = 0x009c_a4a0;
const VATS_MENU_DESTRUCT: u32 = 0x007e_9130;

/// `Tile::GetMenu` (Xbox PDB).
const TILE_GET_MENU: u32 = 0x00a0_3c90;
/// `__RTDynamicCast` (cdecl: object, offset, from type, to type, is a reference) and the type
/// descriptors `ReloadMenu` casts with.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TILE_TYPE: u32 = 0x0119_f4cc;
const CONTAINER_MENU_TYPE: u32 = 0x0119_f758;
const BOOK_MENU_TYPE: u32 = 0x0119_f400;
const BOUND_OBJECT_TYPE: u32 = 0x0118_3108;
const BOOK_TYPE: u32 = 0x0118_a650;
/// The flag `ReloadMenu` reads first (`004a4040`: the object at `011daac0` asked through `004a4080`).
const RELOAD_FLAG: u32 = 0x004a_4040;
/// `InterfaceManager::NewTimer` (cdecl: menu, float).
const NEW_TIMER: u32 = 0x0071_64c0;
/// The menus' creators `ReloadMenu` calls that no family types.
const HUD_MAIN_MENU_CREATE: u32 = 0x0076_bfe0;
const START_MENU_CREATE: u32 = 0x007c_b7d0;
const LOADING_MENU_CREATE: u32 = 0x0078_8cc0;
const LEVEL_UP_MENU_CREATE: u32 = 0x0078_4c80;
const CONTAINER_MENU_CREATE: u32 = 0x0075_b310;
const BOOK_MENU_CREATE: u32 = 0x0073_ad50;
/// The start menu's class (`0070ede0` answers it) and the map menu's (`00720df0`).
const START_MENU_CLASS: u32 = 0x3f5;
const MAP_MENU_CLASS: u32 = 0x3ff;
/// The trait ids: the timer's delay, the depth, the menu kind and the enabled trait.
const TRAIT_TIMER: i32 = 0xfc1;
const TRAIT_DEPTH: i32 = 0xfad;
const TRAIT_KIND: i32 = 0xfa7;
const TRAIT_ENABLED: i32 = 0xfa3;
/// `GetFrontmostMenu`'s starting depth (a float) and the kind it passes over (a double, 6008.0).
const FRONTMOST_START_VALUE: u32 = 0x0107_0218;
const FRONTMOST_EXCLUDED_VALUE: u32 = 0x0107_0210;
/// A menu's tile (`00726070`), its state word (`0059bb30`) and the states `GetFrontmostMenu` and
/// `IsMenuOpen` pass over (2, 4), and the state `ReloadMenu` stores (8, `007037c0`).
const MENU_TILE: u32 = 4;
const MENU_STATE: u32 = in_place::MENU_STATE;
const STATE_RELOADED: u32 = 8;

/// Frees `this` when bit 0 of `flags` is set (the deleting destructors' tail).
#[inline]
fn free_if_flagged(e: &mut Engine, this: Ptr, flags: u32) {
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
}

/// The `Menu` base constructor (`00a1c4a0`, plain stores): its vtable, the words at +4, +8, +0xc,
/// +0x10, +0x14 and +0x18 null, the state (+0x24) 4, the byte at +0x1d 1 and the one at +0x1c 0.
#[inline]
fn menu_base(e: &mut Engine, at: u32) {
    e.mem.set_u32(at, MENU_VTABLE);
    for offset in [8u32, 0xc, 4, 0x10, 0x14] {
        e.mem.set_u32(at.wrapping_add(offset), 0);
    }
    e.mem.set_u32(at.wrapping_add(MENU_STATE), 4);
    e.mem.set_u8(at.wrapping_add(0x1d), 1);
    e.mem.set_u32(at.wrapping_add(0x18), 0);
    e.mem.set_u8(at.wrapping_add(0x1c), 0);
}

/// The 8-byte list's constructor (`0096a2d0`, plain stores): its two words zero.
#[inline]
pub(super) fn empty_list(e: &mut Engine, at: u32) {
    e.mem.set_u32(at, 0);
    e.mem.set_u32(at.wrapping_add(4), 0);
}

/// `memset(at, 0, size)` (`00403d30`) of a menu's tile pointers: `size` is a whole number of words.
#[inline]
fn zero_words(e: &mut Engine, at: u32, size: u32) {
    for offset in (0..size).step_by(4) {
        e.mem.set_u32(at.wrapping_add(offset), 0);
    }
}

/// The array constructor over `count` null smart pointers at `at` (`006694e0`: `00633c90` with
/// null stores the null and, holding none, adds no reference): `count` zero words.
#[inline]
fn null_pointers(e: &mut Engine, at: u32, count: u32) {
    zero_words(e, at, 4 * count);
}

/// The array constructor over `count` null smart pointers by address, as the book and love tester
/// menus' constructors have it beside their pages' array (`eh vector constructor iterator` with the
/// default constructor and the dispatch family's destructor).
fn construct_pointer_array(e: &mut Engine, at: u32, count: u32) {
    e.call(
        VECTOR_CONSTRUCT,
        &args![at, 4u32, count, POINTER_CONSTRUCT_DEFAULT, POINTER_DESTRUCT],
    );
}

/// The array destructor over `count` smart pointers at `at`.
fn destruct_pointer_array(e: &mut Engine, at: u32, count: u32) {
    e.call(VECTOR_DESTRUCT, &args![at, 4u32, count, POINTER_DESTRUCT]);
}

/// The sound handle's constructor (`0041a250`) at `at`: -1, a zero byte, 0.
#[inline]
fn sound_handle(e: &mut Engine, at: u32) {
    e.mem.set_u32(at, u32::MAX);
    e.mem.set_u8(at.wrapping_add(4), 0);
    e.mem.set_u32(at.wrapping_add(8), 0);
}

/// A tile's trait value (`Tile::GetFloat`, the dispatch family's).
#[inline]
fn tile_float(e: &mut Engine, tile: u32, id: i32) -> f32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_11b0,
        dispatch::tile::get_float(e, Ptr::new(tile), id)
    )
}

/// The first node of the menus root's tile list (`004b7210`, `00586150`, its `+4`, `00559450`).
#[inline]
fn first_menu_node(e: &Engine) -> u32 {
    let root = menus_root(e, manager(e));
    e.mem.u32(root.wrapping_add(4))
}

// Typed from 0071e290
/// `MenuManager::Instance` (Xbox PDB): with `create` and no instance yet, a one-byte block
/// constructed (`0071e340`, skipped when the allocation fails) becomes the instance, and the
/// constructor body (`0071e400`) runs again on it. The instance.
pub fn menu_manager_instance(e: &mut Engine, create: bool) -> u32 {
    if create && e.mem.u32(MENU_MANAGER) == 0 {
        let block = e.call(ALLOCATE, &args![1u32]).u32();
        let manager = if block == 0 {
            0
        } else {
            typed_call!(super::FAMILY, 0x0071_e340, fn_0071e340(e, Ptr::new(block))).addr()
        };
        e.mem.set_u32(MENU_MANAGER, manager);
        typed_call!(
            super::FAMILY,
            0x0071_e400,
            fn_0071e400(e, Ptr::new(manager))
        );
    }
    e.mem.u32(MENU_MANAGER)
}

// Typed from 0071e340
/// The `MenuManager` constructor: the constructor body (`0071e400`); `this`.
pub fn fn_0071e340(e: &mut Engine, this: Ptr) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_e400, fn_0071e400(e, this));
    this
}

// Typed from 0071e360
/// Deletes the instance, if any (its deleting destructor with flag 1), and clears the global (the
/// exe reads it twice with nothing between: once).
pub fn fn_0071e360(e: &mut Engine) {
    let manager = e.mem.u32(MENU_MANAGER);
    if manager != 0 {
        typed_call!(
            super::FAMILY,
            0x0071_e3b0,
            fn_0071e3b0(e, Ptr::new(manager), 1)
        );
        e.mem.set_u32(MENU_MANAGER, 0);
    }
}

// Typed from 0071e3b0
/// The `MenuManager` deleting destructor: the destructor, then `this` freed for bit 0 of `flags`.
pub fn fn_0071e3b0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_e3e0, fn_0071e3e0(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071e3e0
/// The `MenuManager` destructor: the instance global cleared (`0071e3f0`).
pub fn fn_0071e3e0(e: &mut Engine, _this: Ptr) {
    typed_call!(super::FAMILY, 0x0071_e3f0, fn_0071e3f0(e));
}

// Typed from 0071e3f0
/// Clears the instance global.
pub fn fn_0071e3f0(e: &mut Engine) {
    e.mem.set_u32(MENU_MANAGER, 0);
}

// Typed from 0071e400
/// The constructor body: the table at `011f3508` grown to `0x54` entries (`0096ad30`).
pub fn fn_0071e400(e: &mut Engine, _this: Ptr) {
    e.call(MENU_TABLE_GROW, &args![MENU_TABLE, MENU_TABLE_COUNT]);
}

/// `CreateMenuByClass`'s switch on the class (`0x3e9..=0x43c`): the block's size and the
/// constructor of each class that has one.
fn menu_class(class: u32) -> Option<(u32, u32)> {
    Some(match class {
        0x3e9 => (0x94, 0x007a_8a10),
        0x3ea => (0x124, 0x0077_fa00),
        0x3eb => (0x2a4, 0x0071_f340),
        0x3ec => (0x278, 0x0076_bb40),
        0x3ef => (0x5c0, 0x0078_8440),
        0x3f0 => (0x10c, 0x0075_b0d0),
        0x3f1 => (0x13c, 0x0076_17a0),
        0x3f4 => (0x4c, 0x007b_fb30),
        0x3f5 => (0x1d4, 0x007c_b330),
        0x3f6 => (0xe4, 0x0078_d930),
        0x3f8 => (0x44, 0x0072_0a10),
        0x3ff => (0x230, 0x0079_6740),
        0x402 => (0x3c, 0x0073_acb0),
        0x403 => (0xcc, 0x0078_4a40),
        0x40b => (0x8c, 0x007b_5400),
        0x40c => (0x1a4, 0x007a_c1f0),
        0x417 => (0x80, 0x0075_f070),
        0x418 => (0x88, 0x0075_3230),
        0x41b => (0x5c, 0x0071_fa80),
        0x41d => (0x120, 0x0072_ceb0),
        0x41e => (0x160, 0x0071_fbf0),
        0x41f => (0x1dc, 0x0076_54f0),
        0x420 => (0x144, 0x0072_0040),
        0x421 => (0xfc, 0x0075_77f0),
        0x422 => (0xa0, 0x007b_7400),
        0x423 => (0x54, 0x0071_f240),
        0x424 => (0x1e4, 0x0072_0170),
        0x425 => (0x90, 0x0078_3310),
        0x432 => (0x230, 0x0072_0550),
        0x433 => (0x88, 0x0075_4b40),
        0x434 => (0x88, 0x007e_7a80),
        0x435 => (0x104, 0x0072_6df0),
        0x438 => (0x158, 0x007c_06f0),
        0x439 => (0xe70, 0x0073_15e0),
        0x43a => (0x4014, 0x007b_8d30),
        0x43b => (0xe88, 0x0073_b7e0),
        0x43c => (0x8c, 0x007e_6780),
        _ => return None,
    })
}

/// A menu constructor on `block`: this file's directly, the other units' by address.
fn construct_menu(e: &mut Engine, constructor: u32, block: Ptr) -> u32 {
    let menu = match constructor {
        0x0071_f240 => typed_call!(super::FAMILY, 0x0071_f240, fn_0071f240(e, block)),
        0x0071_f340 => typed_call!(super::FAMILY, 0x0071_f340, fn_0071f340(e, block)),
        0x0071_fa80 => typed_call!(super::FAMILY, 0x0071_fa80, fn_0071fa80(e, block)),
        0x0071_fbf0 => typed_call!(super::FAMILY, 0x0071_fbf0, fn_0071fbf0(e, block)),
        0x0072_0040 => typed_call!(super::FAMILY, 0x0072_0040, fn_00720040(e, block)),
        0x0072_0170 => typed_call!(super::FAMILY, 0x0072_0170, fn_00720170(e, block)),
        0x0072_0550 => typed_call!(super::FAMILY, 0x0072_0550, fn_00720550(e, block)),
        0x0072_0a10 => typed_call!(super::FAMILY, 0x0072_0a10, fn_00720a10(e, block)),
        _ => return e.call(constructor, &args![block]).u32(),
    };
    menu.addr()
}

// Typed from 0071e420
/// `MenuManager::CreateMenuByClass` (Xbox PDB): the menu of `class` allocated and constructed, null
/// when the allocation fails; for a class with no menu, `"MENUS: Unknown menu class!"` printed and
/// null.
pub fn menu_manager_create_menu_by_class(e: &mut Engine, _this: Ptr, class: u32) -> u32 {
    let Some((size, constructor)) = menu_class(class) else {
        e.call(DEBUG_MESSAGE, &args![UNKNOWN_MENU_CLASS_TEXT]);
        return 0;
    };
    let block = e.call(ALLOCATE, &args![size]).u32();
    if block == 0 {
        return 0;
    }
    construct_menu(e, constructor, Ptr::new(block))
}

// Typed from 0071f240
/// The `TutorialMenu` constructor: the `Menu` base, the vtable, the eight tile pointers (+0x28)
/// zeroed, the current page, the page count and the help manual cleared. `this`.
pub fn fn_0071f240(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    menu_base(e, this.addr());
    e.mem.set_u32(base, TUTORIAL_MENU_VTABLE);
    zero_words(e, base.wrapping_add(0x28), 0x20);
    for offset in [0x48u32, 0x4c, 0x50] {
        e.mem.set_u32(base.wrapping_add(offset), 0);
    }
    this
}

// Typed from 0071f2d0
/// `TutorialMenu::GetClass` (Xbox PDB): `0x423`.
pub fn tutorial_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x423
}

// Typed from 0071f2e0
/// `TutorialMenu`'s deleting destructor: the destructor, then `this` freed for bit 0 of `flags`.
pub fn tutorial_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_f310, fn_0071f310(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071f310
/// The `TutorialMenu` destructor: the vtable, the tutorial menu global cleared, the `Menu` base.
pub fn fn_0071f310(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), TUTORIAL_MENU_VTABLE);
    e.mem.set_u32(TUTORIAL_MENU_GLOBAL, 0);
    e.call(MENU_DESTRUCT, &args![this]);
}

// Typed from 0071f340
/// The `StatsMenu` constructor: the `Menu` base and vtable, the four healing options (+0x28, 0x10
/// bytes each: `0071f4a0`'s stores, the array constructor done in place), the two effect lists
/// (+0x68, +0x84), the six list boxes, the 60 tile pointers (+0x90) zeroed, the four status effects
/// and the two flags cleared. `this`.
pub fn fn_0071f340(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    menu_base(e, this.addr());
    e.mem.set_u32(base, STATS_MENU_VTABLE);
    for option in 0..4u32 {
        healing_option(e, base.wrapping_add(0x28 + 0x10 * option));
    }
    typed_call!(
        super::FAMILY,
        0x0071_f540,
        fn_0071f540(e, Ptr::new(base.wrapping_add(0x68)))
    );
    typed_call!(
        super::FAMILY,
        0x0071_f5d0,
        fn_0071f5d0(e, Ptr::new(base.wrapping_add(0x84)))
    );
    // The six list boxes, in the exe's order: two of `ActorValue::Index`, then `PerkRankData *`,
    // `MiscStatID`, `StatusEffect *` and `TESReputation *`.
    let list_box = |offset: u32| Ptr::new(base.wrapping_add(offset));
    typed_call!(super::FAMILY, 0x0072_1060, fn_00721060(e, list_box(0x180)));
    typed_call!(super::FAMILY, 0x0072_1060, fn_00721060(e, list_box(0x1b0)));
    typed_call!(super::FAMILY, 0x0072_1850, fn_00721850(e, list_box(0x1e0)));
    typed_call!(super::FAMILY, 0x0072_2000, fn_00722000(e, list_box(0x210)));
    typed_call!(super::FAMILY, 0x0072_27b0, fn_007227b0(e, list_box(0x240)));
    typed_call!(super::FAMILY, 0x0072_2f60, fn_00722f60(e, list_box(0x270)));
    zero_words(e, base.wrapping_add(0x90), 0xf0);
    // The four status effects, then `bActiveEffectsUpdatedThisFrame` and `bLimbMode` (Xbox PDB).
    for offset in [0x74u32, 0x78, 0x80, 0x7c] {
        e.mem.set_u32(base.wrapping_add(offset), 0);
    }
    e.mem.set_u8(base.wrapping_add(0x2a1), 0);
    e.mem.set_u8(base.wrapping_add(0x2a0), 0);
    this
}

/// A healing option's constructor (`0071f4a0`) at `at`: the item, tile and inventory item null,
/// the target condition `0071f520`.
#[inline]
fn healing_option(e: &mut Engine, at: u32) {
    e.mem.set_u32(at, 0);
    e.mem.set_u32(at.wrapping_add(4), 0);
    e.mem.set_u32(at.wrapping_add(8), 0);
    e.mem.set_u32(at.wrapping_add(0xc), 0x0071_f520);
}

// Typed from 0071f4a0
/// The `StatsMenu::HealingOption` constructor (Xbox PDB): its item, tile and inventory item null,
/// its target condition `0071f520`. `this`.
pub fn fn_0071f4a0(e: &mut Engine, this: Ptr) -> Ptr {
    healing_option(e, this.addr());
    this
}

// Typed from 0071f4e0
/// The `StatsMenu::HealingOption` destructor: its inventory item (+8), if any, deleted
/// (`004459e0` with flag 1).
pub fn fn_0071f4e0(e: &mut Engine, this: Ptr) {
    let item = e.mem.u32(this.addr().wrapping_add(8));
    if item != 0 {
        e.call(ITEM_DELETE, &args![item, 1u32]);
    }
}

// Typed from 0071f520
/// A healing option's target condition (cdecl): whether it has an inventory item (+8).
pub fn fn_0071f520(e: &mut Engine, option: Ptr) -> bool {
    e.mem.u32(option.addr().wrapping_add(8)) != 0
}

// Typed from 0071f540
/// The `StatsMenu::NamedEffectsList` constructor: the list at +4, then the vtable. `this`.
pub fn fn_0071f540(e: &mut Engine, this: Ptr) -> Ptr {
    empty_list(e, this.addr().wrapping_add(4));
    e.mem.set_u32(this.addr(), NAMED_EFFECTS_LIST_VTABLE);
    this
}

// Typed from 0071f570
/// The `StatsMenu::NamedEffectsList` destructor: the vtable, the list emptied (`007e09b0`) and
/// destroyed.
pub fn fn_0071f570(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), NAMED_EFFECTS_LIST_VTABLE);
    typed_call!(
        super::FAMILY,
        0x007e_09b0,
        super::stats_menu::fn_007e09b0(e, this)
    );
    e.call(LIST_DESTRUCT, &args![this.addr().wrapping_add(4)]);
}

// Typed from 0071f5d0
/// The `StatsMenu::StatusEffectList` constructor: the list at +4, then the vtable. `this`.
pub fn fn_0071f5d0(e: &mut Engine, this: Ptr) -> Ptr {
    empty_list(e, this.addr().wrapping_add(4));
    e.mem.set_u32(this.addr(), STATUS_EFFECT_LIST_VTABLE);
    this
}

// Typed from 0071f600
/// The `StatsMenu::StatusEffectList` destructor: the vtable, the list emptied (`007e07f0`) and
/// destroyed.
pub fn fn_0071f600(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), STATUS_EFFECT_LIST_VTABLE);
    typed_call!(
        super::FAMILY,
        0x007e_07f0,
        super::stats_menu::fn_007e07f0(e, this)
    );
    e.call(LIST_DESTRUCT, &args![this.addr().wrapping_add(4)]);
}

// Typed from 0071f660
/// `StatsMenu::GetClass` (Xbox PDB): `0x3eb`.
pub fn stats_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x3eb
}

// Typed from 0071f670
/// `StatsMenu::NamedEffectsList`'s deleting destructor.
pub fn stats_menu_named_effects_list_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_f570, fn_0071f570(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071f6a0
/// `StatsMenu::StatusEffectList`'s deleting destructor.
pub fn stats_menu_status_effect_list_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_f600, fn_0071f600(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071f6d0
/// `StatsMenu`'s deleting destructor.
pub fn stats_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_f700, fn_0071f700(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071f700
/// The `StatsMenu` destructor: the vtable, the stats menu global cleared, the four status effects
/// (+0x74, +0x80, +0x7c, +0x78) deleted (`0071f9c0` with flag 1), the body part controller
/// (`0071f8c0`), the six list boxes, the two effect lists and the four healing options destroyed,
/// then the `Menu` base.
pub fn fn_0071f700(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    e.mem.set_u32(base, STATS_MENU_VTABLE);
    e.mem.set_u32(STATS_MENU_GLOBAL, 0);
    for offset in [0x74u32, 0x80, 0x7c, 0x78] {
        let effect = e.mem.u32(base.wrapping_add(offset));
        if effect != 0 {
            typed_call!(
                super::FAMILY,
                0x0071_f9c0,
                fn_0071f9c0(e, Ptr::new(effect), 1)
            );
        }
    }
    typed_call!(super::FAMILY, 0x0071_f8c0, fn_0071f8c0(e));
    // The six list boxes in reverse.
    let list_box = |offset: u32| Ptr::new(base.wrapping_add(offset));
    typed_call!(super::FAMILY, 0x0072_3070, fn_00723070(e, list_box(0x270)));
    typed_call!(super::FAMILY, 0x0072_2880, fn_00722880(e, list_box(0x240)));
    typed_call!(super::FAMILY, 0x0072_20d0, fn_007220d0(e, list_box(0x210)));
    typed_call!(super::FAMILY, 0x0072_1920, fn_00721920(e, list_box(0x1e0)));
    typed_call!(super::FAMILY, 0x0072_1150, fn_00721150(e, list_box(0x1b0)));
    typed_call!(super::FAMILY, 0x0072_1150, fn_00721150(e, list_box(0x180)));
    typed_call!(
        super::FAMILY,
        0x0071_f600,
        fn_0071f600(e, Ptr::new(base.wrapping_add(0x84)))
    );
    typed_call!(
        super::FAMILY,
        0x0071_f570,
        fn_0071f570(e, Ptr::new(base.wrapping_add(0x68)))
    );
    e.call(
        VECTOR_DESTRUCT,
        &args![base.wrapping_add(0x28), 0x10u32, 4u32, 0x0071_f4e0u32],
    );
    e.call(MENU_DESTRUCT, &args![this]);
}

// Typed from 0071f8c0
/// The body part controller (`011dace4`), if any, deleted (`0071f900` with flag 1); the global
/// cleared.
pub fn fn_0071f8c0(e: &mut Engine) {
    let controller = e.mem.u32(BODY_PART_CONTROLLER_GLOBAL);
    if controller != 0 {
        typed_call!(
            super::FAMILY,
            0x0071_f900,
            fn_0071f900(e, Ptr::new(controller), 1)
        );
    }
    e.mem.set_u32(BODY_PART_CONTROLLER_GLOBAL, 0);
}

// Typed from 0071f900
/// The body part controller's deleting destructor.
pub fn fn_0071f900(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_f930, fn_0071f930(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071f930
/// The body part controller's destructor: its six 8-byte lists from +4 emptied (`00470470`), then
/// destroyed (`0046ffb0` through the array destructor).
pub fn fn_0071f930(e: &mut Engine, this: Ptr) {
    let lists = this.addr().wrapping_add(4);
    for index in 0..6u32 {
        e.call(LIST_CLEAR, &args![lists.wrapping_add(8 * index)]);
    }
    e.call(VECTOR_DESTRUCT, &args![lists, 8u32, 6u32, LIST_DESTRUCT]);
}

// Typed from 0071f9c0
/// The `StatsMenu::StatusEffect` deleting destructor.
pub fn fn_0071f9c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_f9f0, fn_0071f9f0(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071f9f0
/// The `StatsMenu::StatusEffect` destructor: `xNamedEffects` (+0x10) destroyed, `xStatsEffected`
/// (+4) flushed (Xbox PDB names).
pub fn fn_0071f9f0(e: &mut Engine, this: Ptr) {
    typed_call!(
        super::FAMILY,
        0x0071_f570,
        fn_0071f570(e, Ptr::new(this.addr().wrapping_add(0x10)))
    );
    typed_call!(
        super::FAMILY,
        0x0040_6de0,
        super::stats_menu::fn_00406de0(e, Ptr::new(this.addr().wrapping_add(4)))
    );
}

/// `AttachTileByID`'s store: `tile` at `base + 4 * index`.
#[inline]
fn attach(e: &mut Engine, base: u32, index: i32, tile: u32) {
    e.mem
        .set_u32(base.wrapping_add(4u32.wrapping_mul(index as u32)), tile);
}

// Typed from 0071fa50
/// `StatsMenu::AttachTileByID` (Xbox PDB): `tile` into `pTiles` (+0x90) for ids 0 to 59 (signed).
pub fn stats_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (0..0x3c).contains(&id) {
        attach(e, this.addr().wrapping_add(0x90), id, tile);
    }
}

/// The text edit menu's editor constructor (`00716980`, in place): the `BSString`s at `+0` and `+8`
/// empty (`004037b0`), the bytes `+0x21` and `+0x20` and the word `+0x10` cleared, `+0x14` -1,
/// `+0x1c` 0, `+0x18` 1, in the exe's order.
fn text_editor(e: &mut Engine, at: u32) {
    in_place::empty_string(e, at);
    in_place::empty_string(e, at.wrapping_add(8));
    e.mem.set_u8(at.wrapping_add(0x21), 0);
    e.mem.set_u32(at.wrapping_add(0x10), 0);
    e.mem.set_u8(at.wrapping_add(0x20), 0);
    e.mem.set_u32(at.wrapping_add(0x14), u32::MAX);
    e.mem.set_u32(at.wrapping_add(0x1c), 0);
    e.mem.set_u32(at.wrapping_add(0x18), 1);
}

// Typed from 0071fa80
/// The `TextEditMenu` constructor: the `Menu` base and vtable, the editor (+0x34), the three tile
/// pointers (+0x28) zeroed, the entered text global emptied (`004037f0` with null and 0), the text
/// check (+0x58) cleared. `this`.
pub fn fn_0071fa80(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    menu_base(e, this.addr());
    e.mem.set_u32(base, TEXT_EDIT_MENU_VTABLE);
    text_editor(e, base.wrapping_add(0x34));
    zero_words(e, base.wrapping_add(0x28), 0xc);
    e.call(STRING_SET, &args![ENTERED_TEXT, 0u32, 0u32]);
    e.mem.set_u32(base.wrapping_add(0x58), 0);
    this
}

// Typed from 0071fb20
/// `TextEditMenu`'s class: `0x41b`.
pub fn fn_0071fb20(_e: &mut Engine, _this: Ptr) -> u32 {
    0x41b
}

// Typed from 0071fb30
/// `TextEditMenu`'s deleting destructor.
pub fn fn_0071fb30(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_fb60, fn_0071fb60(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071fb60
/// The `TextEditMenu` destructor: the vtable, the editor (+0x34), the `Menu` base.
pub fn fn_0071fb60(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), TEXT_EDIT_MENU_VTABLE);
    e.call(TEXT_EDITOR_DESTRUCT, &args![this.addr().wrapping_add(0x34)]);
    e.call(MENU_DESTRUCT, &args![this]);
}

// Typed from 0071fbc0
/// `TextEditMenu`'s `AttachTileByID`: `tile` at +0x28 for ids 0 to 2 (signed).
pub fn fn_0071fbc0(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (0..3).contains(&id) {
        attach(e, this.addr().wrapping_add(0x28), id, tile);
    }
}

// Typed from 0071fbf0
/// The `SurgeryMenu` constructor: the `Menu` base and vtable; its members (the empty
/// constructors, which do nothing; the sound handles at +0x94 and +0xa0; the six smart pointers at
/// +0xc8 and those at +0xf0 and +0xf4, null), the 18 tile pointers (+0x2c) zeroed, the three static
/// smart pointers, +0xf0, +0xf4 and the six at +0xc8 assigned null (`0066b0d0`, by address: it
/// releases what a static held), then the scalar fields. `this`.
pub fn fn_0071fbf0(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    menu_base(e, this.addr());
    e.mem.set_u32(base, SURGERY_MENU_VTABLE);
    sound_handle(e, base.wrapping_add(0x94));
    sound_handle(e, base.wrapping_add(0xa0));
    null_pointers(e, base.wrapping_add(0xc8), 6);
    e.mem.set_u32(base.wrapping_add(0xf0), 0);
    e.mem.set_u32(base.wrapping_add(0xf4), 0);
    zero_words(e, base.wrapping_add(0x2c), 0x48);
    for global in SURGERY_STATIC_POINTERS {
        e.call(POINTER_ASSIGN, &args![global, 0u32]);
    }
    e.call(POINTER_ASSIGN, &args![base.wrapping_add(0xf0), 0u32]);
    e.call(POINTER_ASSIGN, &args![base.wrapping_add(0xf4), 0u32]);
    for index in 0..6u32 {
        e.call(
            POINTER_ASSIGN,
            &args![base.wrapping_add(0xc8 + 4 * index), 0u32],
        );
    }
    // pFrameGeometry, pSkinControllerManager, pStimPaks, pScar; fFrameAnimTimer, fToolMovement,
    // fPainPercent, +0x14c (0.0); the five timers (uiDamageHealthTime is 1000 and then 0 with
    // nothing between: 0); bBurning, bToolBuriedInFlesh, +0x15c, +0x15d; Resistance and ToolLocation
    // (0.0); eAnimation 0 (Xbox PDB names).
    for offset in [
        0xe0u32, 0xe4, 0xbc, 0xe8, 0xec, 0x84, 0xac, 0x14c, 0x88, 0x8c, 0xb0, 0xb4, 0xb8, 0x7c,
        0x80, 0x74, 0x78, 0x28,
    ] {
        e.mem.set_u32(base.wrapping_add(offset), 0);
    }
    for offset in [0x90u32, 0x91, 0x15c, 0x15d] {
        e.mem.set_u8(base.wrapping_add(offset), 0);
    }
    // eCurrentDamagedPart, eCurrentDamageLevel.
    e.mem.set_u32(base.wrapping_add(0xc0), 0x1b);
    e.mem.set_u32(base.wrapping_add(0xc4), 1);
    this
}

// Typed from 0071feb0
/// `SurgeryMenu::GetClass` (Xbox PDB): `0x41e`.
pub fn surgery_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x41e
}

// Typed from 0071fec0
/// `SurgeryMenu`'s deleting destructor.
pub fn surgery_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0071_fef0, fn_0071fef0(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 0071fef0
/// The `SurgeryMenu` destructor: the vtable, the three static smart pointers and +0xf0 assigned
/// null, the smart pointers at +0xf4, +0xf0 and the six at +0xc8 destroyed, the sound handles'
/// empty destructors (`00483710`, nothing), the `Menu` base.
pub fn fn_0071fef0(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    e.mem.set_u32(base, SURGERY_MENU_VTABLE);
    for global in SURGERY_STATIC_POINTERS {
        e.call(POINTER_ASSIGN, &args![global, 0u32]);
    }
    e.call(POINTER_ASSIGN, &args![base.wrapping_add(0xf0), 0u32]);
    for offset in [0xf4u32, 0xf0] {
        typed_call!(
            dispatch::FAMILY,
            0x0045_cec0,
            dispatch::refs::release_held(e, Ptr::new(base.wrapping_add(offset)))
        );
    }
    destruct_pointer_array(e, base.wrapping_add(0xc8), 6);
    e.call(MENU_DESTRUCT, &args![this]);
}

// Typed from 0071ffe0
/// `SurgeryMenu::AttachTileByID` (Xbox PDB): `tile` into `pTiles` (+0x2c) at `id - 1`
/// (`00720010`) for ids 1 to 18 (signed).
pub fn surgery_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (1..=0x12).contains(&id) {
        let index = typed_call!(super::FAMILY, 0x0072_0010, fn_00720010(e, this, id));
        attach(e, this.addr().wrapping_add(0x2c), index, tile);
    }
}

// Typed from 00720010
/// The surgery menu's tile index of an id: `id - 1` for ids 1 to 18 (signed), 18 otherwise.
pub fn fn_00720010(_e: &mut Engine, _this: Ptr, id: i32) -> i32 {
    if (1..=0x12).contains(&id) {
        id - 1
    } else {
        0x12
    }
}

// Typed from 00720040
/// The `VATSMenu` constructor: the `Menu` base and vtable, the action list box (+0xb0), the
/// command (+0x11c), the 32 tile pointers (+0x28) zeroed, `bHeadingReached` (+0xf8) and
/// `bConcentratedFirePerk` (+0x104) cleared and `pLastLimb` (+0x108) null (Xbox PDB names). `this`.
pub fn fn_00720040(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    menu_base(e, this.addr());
    e.mem.set_u32(base, VATS_MENU_VTABLE);
    typed_call!(
        super::FAMILY,
        0x0072_3750,
        fn_00723750(e, Ptr::new(base.wrapping_add(0xb0)))
    );
    e.call(VATS_COMMAND_CONSTRUCT, &args![base.wrapping_add(0x11c)]);
    zero_words(e, base.wrapping_add(0x28), 0x80);
    e.mem.set_u8(base.wrapping_add(0xf8), 0);
    e.mem.set_u8(base.wrapping_add(0x104), 0);
    e.mem.set_u32(base.wrapping_add(0x108), 0);
    this
}

// Typed from 00720100
/// `VATSMenu::GetClass` (Xbox PDB): `0x420`.
pub fn vats_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x420
}

// Typed from 00720110
/// `VATSMenu`'s deleting destructor: its destructor (`007e9130`), then `this` freed for bit 0.
pub fn vats_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(VATS_MENU_DESTRUCT, &args![this]);
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00720140
/// `VATSMenu::AttachTileByID` (Xbox PDB): `tile` at `+0x28 + 4 * id` for ids 1 to 31 (signed).
pub fn vats_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (1..=0x1f).contains(&id) {
        attach(e, this.addr().wrapping_add(0x28), id, tile);
    }
}

/// The state fields the book and love tester menus' constructors set: `eState` 0,
/// `eXBoxIndexState` 1, `fFrameAnimTimer` 0.0, `bAnimFinished` 1, `iAvailablePoints` 0.
#[inline]
fn book_state(e: &mut Engine, base: u32) {
    e.mem.set_u32(base.wrapping_add(0x48), 0);
    e.mem.set_u32(base.wrapping_add(0x4c), 1);
    e.mem.set_u32(base.wrapping_add(0x50), 0);
    e.mem.set_u8(base.wrapping_add(0x54), 1);
    e.mem.set_u32(base.wrapping_add(0x58), 0);
}

// Typed from 00720170
/// The `SPECIALBookMenu` constructor: the `Menu` base and vtable, the nine pages (+0x60, 0x20
/// bytes each: `00720360`, `007203c0`) and the smart pointer arrays (11 at +0x180, 4 at +0x1ac,
/// five pairs from +0x1bc) through the array constructor by address, the eight tile pointers
/// (+0x28) zeroed, `pBookGeometry` null and the state fields set (Xbox PDB names). `this`.
pub fn fn_00720170(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    menu_base(e, this.addr());
    e.mem.set_u32(base, SPECIAL_BOOK_MENU_VTABLE);
    e.call(
        VECTOR_CONSTRUCT,
        &args![
            base.wrapping_add(0x60),
            0x20u32,
            9u32,
            0x0072_0360u32,
            0x0072_03c0u32
        ],
    );
    construct_pointer_array(e, base.wrapping_add(0x180), 0xb);
    construct_pointer_array(e, base.wrapping_add(0x1ac), 4);
    for offset in [0x1bcu32, 0x1c4, 0x1cc, 0x1d4, 0x1dc] {
        construct_pointer_array(e, base.wrapping_add(offset), 2);
    }
    zero_words(e, base.wrapping_add(0x28), 0x20);
    e.mem.set_u32(base.wrapping_add(0x5c), 0);
    book_state(e, base);
    this
}

// Typed from 00720320
/// `SPECIALBookMenu::GetClass` (Xbox PDB): `0x424`.
pub fn special_book_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x424
}

// Typed from 00720330
/// `SPECIALBookMenu`'s deleting destructor.
pub fn special_book_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_0420, fn_00720420(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00720360
/// A `SPECIALBookMenu` page's constructor: `Buttons` (+0) and `TextItems` (+0x10) (Xbox PDB).
/// `this`.
pub fn fn_00720360(e: &mut Engine, this: Ptr) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_3f50, fn_00723f50(e, this));
    let text_items = Ptr::new(this.addr().wrapping_add(0x10));
    typed_call!(super::FAMILY, 0x0072_3f00, fn_00723f00(e, text_items));
    this
}

// Typed from 007203c0
/// A `SPECIALBookMenu` page's destructor: `TextItems` (+0x10), then `Buttons` (+0).
pub fn fn_007203c0(e: &mut Engine, this: Ptr) {
    let text_items = Ptr::new(this.addr().wrapping_add(0x10));
    typed_call!(super::FAMILY, 0x0072_3f30, fn_00723f30(e, text_items));
    typed_call!(super::FAMILY, 0x0072_3f80, fn_00723f80(e, this));
}

// Typed from 00720420
/// The `SPECIALBookMenu` destructor: the vtable, the smart pointer arrays (in reverse) and the
/// nine pages destroyed, the `Menu` base.
pub fn fn_00720420(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    e.mem.set_u32(base, SPECIAL_BOOK_MENU_VTABLE);
    for offset in [0x1dcu32, 0x1d4, 0x1cc, 0x1c4, 0x1bc] {
        destruct_pointer_array(e, base.wrapping_add(offset), 2);
    }
    destruct_pointer_array(e, base.wrapping_add(0x1ac), 4);
    destruct_pointer_array(e, base.wrapping_add(0x180), 0xb);
    e.call(
        VECTOR_DESTRUCT,
        &args![base.wrapping_add(0x60), 0x20u32, 9u32, 0x0072_03c0u32],
    );
    e.call(MENU_DESTRUCT, &args![this]);
}

// Typed from 00720550
/// The `LoveTesterMenu` constructor: the `Menu` base and vtable, `LightBulbs` (+0x64) and
/// `LightBulbGlows` (+0x74), the nine pages (+0x84: `007207a0`, `00720800`, by the array
/// constructor), `GlobalPage` (+0x1a4), the smart pointer arrays (11 at +0x1c4, 4 at +0x1f0, six
/// pairs from +0x200), the eight tile pointers (+0x28) zeroed, the two geometries null and the
/// state fields set (Xbox PDB names). `this`.
pub fn fn_00720550(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    menu_base(e, this.addr());
    e.mem.set_u32(base, LOVE_TESTER_MENU_VTABLE);
    let light_bulbs = Ptr::new(base.wrapping_add(0x64));
    typed_call!(super::FAMILY, 0x0072_3fa0, fn_00723fa0(e, light_bulbs));
    let glows = Ptr::new(base.wrapping_add(0x74));
    typed_call!(super::FAMILY, 0x0072_3ff0, fn_00723ff0(e, glows));
    e.call(
        VECTOR_CONSTRUCT,
        &args![
            base.wrapping_add(0x84),
            0x20u32,
            9u32,
            0x0072_07a0u32,
            0x0072_0800u32
        ],
    );
    typed_call!(
        super::FAMILY,
        0x0072_07a0,
        fn_007207a0(e, Ptr::new(base.wrapping_add(0x1a4)))
    );
    construct_pointer_array(e, base.wrapping_add(0x1c4), 0xb);
    construct_pointer_array(e, base.wrapping_add(0x1f0), 4);
    for offset in [0x200u32, 0x208, 0x210, 0x218, 0x220, 0x228] {
        construct_pointer_array(e, base.wrapping_add(offset), 2);
    }
    zero_words(e, base.wrapping_add(0x28), 0x20);
    e.mem.set_u32(base.wrapping_add(0x5c), 0);
    e.mem.set_u32(base.wrapping_add(0x60), 0);
    book_state(e, base);
    this
}

// Typed from 00720760
/// `LoveTesterMenu::GetClass` (Xbox PDB): `0x432`.
pub fn love_tester_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x432
}

// Typed from 00720770
/// `LoveTesterMenu`'s deleting destructor.
pub fn love_tester_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_0860, fn_00720860(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 007207a0
/// A `LoveTesterMenu` page's constructor: `Buttons` (+0) and `TextItems` (+0x10) (Xbox PDB).
/// `this`.
pub fn fn_007207a0(e: &mut Engine, this: Ptr) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_4040, fn_00724040(e, this));
    let text_items = Ptr::new(this.addr().wrapping_add(0x10));
    typed_call!(super::FAMILY, 0x0072_3fa0, fn_00723fa0(e, text_items));
    this
}

// Typed from 00720800
/// A `LoveTesterMenu` page's destructor: `TextItems` (+0x10), then `Buttons` (+0).
pub fn fn_00720800(e: &mut Engine, this: Ptr) {
    let text_items = Ptr::new(this.addr().wrapping_add(0x10));
    typed_call!(super::FAMILY, 0x0072_3fd0, fn_00723fd0(e, text_items));
    typed_call!(super::FAMILY, 0x0072_4070, fn_00724070(e, this));
}

// Typed from 00720860
/// The `LoveTesterMenu` destructor: the vtable, the smart pointer arrays (in reverse),
/// `GlobalPage`, the nine pages, `LightBulbGlows` and `LightBulbs` destroyed, the `Menu` base.
pub fn fn_00720860(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    e.mem.set_u32(base, LOVE_TESTER_MENU_VTABLE);
    for offset in [0x228u32, 0x220, 0x218, 0x210, 0x208, 0x200] {
        destruct_pointer_array(e, base.wrapping_add(offset), 2);
    }
    destruct_pointer_array(e, base.wrapping_add(0x1f0), 4);
    destruct_pointer_array(e, base.wrapping_add(0x1c4), 0xb);
    typed_call!(
        super::FAMILY,
        0x0072_0800,
        fn_00720800(e, Ptr::new(base.wrapping_add(0x1a4)))
    );
    e.call(
        VECTOR_DESTRUCT,
        &args![base.wrapping_add(0x84), 0x20u32, 9u32, 0x0072_0800u32],
    );
    let glows = Ptr::new(base.wrapping_add(0x74));
    typed_call!(super::FAMILY, 0x0072_4020, fn_00724020(e, glows));
    let light_bulbs = Ptr::new(base.wrapping_add(0x64));
    typed_call!(super::FAMILY, 0x0072_3fd0, fn_00723fd0(e, light_bulbs));
    e.call(MENU_DESTRUCT, &args![this]);
}

// Typed from 007209e0
/// `TutorialMenu::AttachTileByID` (Xbox PDB; the book and love tester menus' vtables share it):
/// `tile` at `+0x28 + 4 * id` for ids 0 to 7 (signed).
pub fn tutorial_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if (0..8).contains(&id) {
        attach(e, this.addr().wrapping_add(0x28), id, tile);
    }
}

// Typed from 00720a10
/// The `QuantityMenu` constructor: the `Menu` base and the vtable. `this`.
pub fn fn_00720a10(e: &mut Engine, this: Ptr) -> Ptr {
    menu_base(e, this.addr());
    e.mem.set_u32(this.addr(), QUANTITY_MENU_VTABLE);
    this
}

// Typed from 00720a30
/// `QuantityMenu::GetClass` (Xbox PDB): `0x3f8`.
pub fn quantity_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    0x3f8
}

// Typed from 00720a40
/// `QuantityMenu`'s deleting destructor.
pub fn quantity_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_0a70, fn_00720a70(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00720a70
/// The `QuantityMenu` destructor: the vtable, the quantity menu global cleared, the `Menu` base.
pub fn fn_00720a70(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), QUANTITY_MENU_VTABLE);
    e.mem.set_u32(QUANTITY_MENU_GLOBAL, 0);
    e.call(MENU_DESTRUCT, &args![this]);
}

/// The steps after `ReloadMenu` creates one of the five menus with a timer: the created tile's
/// delay trait (`0xfc1`) goes with the tile's menu to `NewTimer`, and the tile's menu (asked again)
/// gets the state 8 (`007037c0`). The tile.
fn reload_with_timer(e: &mut Engine, tile: u32) -> u32 {
    let delay = tile_float(e, tile, TRAIT_TIMER);
    let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
    e.call(NEW_TIMER, &args![menu, delay.to_bits()]);
    let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
    e.mem.set_u32(menu.wrapping_add(MENU_STATE), STATE_RELOADED);
    tile
}

/// An open menu's deleting destructor with flag 1 (vtable slot 0: whatever menu it is).
#[inline]
fn delete_menu(e: &mut Engine, menu: u32) {
    e.vcall(menu, 0, &args![1u32]);
}

// Typed from 00720aa0
/// `MenuManager::ReloadMenu` (Xbox PDB): the flag (`004a4040`) read, the open menu of `class`
/// deleted, then the menu made again; what the creator answers (0 for a class it doesn't remake):
/// the map, repair, item mod, inventory and stats menus through the interface unit's creators and
/// the timer steps; the HUD; the start menu (its class, `0070ede0`, when `StartMenu::Create(flag,
/// 0)` succeeds: the menu of that class); the loading and level up menus; the container menu (the
/// open container menu's `+0x74` again, when the menu of class 0x3f0 is one); the book menu (the
/// menu of class 0x3f0 as a book menu: its `+0x30` and that one's `+0x20` as a book, the book menu
/// deleted, then made again).
pub fn menu_manager_reload_menu(e: &mut Engine, _this: Ptr, class: u32) -> u32 {
    let open = menu_by_class(e, class as i32);
    let flag = e.call(RELOAD_FLAG, &args![]).u8();
    if open != 0 {
        delete_menu(e, open);
    }
    match class {
        MAP_MENU_CLASS => {
            let tile = typed_call!(super::FAMILY, 0x0070_4140, fn_00704140(e));
            return reload_with_timer(e, tile);
        }
        0x40b => {
            let tile = typed_call!(super::FAMILY, 0x0070_4490, interface_create_repair_menu(e));
            return reload_with_timer(e, tile);
        }
        0x425 => {
            let tile = typed_call!(
                super::FAMILY,
                0x0070_46c0,
                interface_create_item_mod_menu(e)
            );
            return reload_with_timer(e, tile);
        }
        0x3ea => {
            let tile = typed_call!(super::FAMILY, 0x0070_48c0, fn_007048c0(e));
            return reload_with_timer(e, tile);
        }
        0x3eb => {
            let tile = typed_call!(super::FAMILY, 0x0070_4be0, fn_00704be0(e));
            return reload_with_timer(e, tile);
        }
        0x3ec => return e.call(HUD_MAIN_MENU_CREATE, &args![]).u32(),
        _ => {}
    }
    if class == START_MENU_CLASS
        && e.call(START_MENU_CREATE, &args![u32::from(flag), 0u32])
            .bool()
    {
        return menu_by_class(e, START_MENU_CLASS as i32);
    }
    match class {
        0x3ef => e.call(LOADING_MENU_CREATE, &args![0u32, 1u32, 0u32]).u32(),
        0x403 => e.call(LEVEL_UP_MENU_CREATE, &args![]).u32(),
        0x3f0 => {
            let open = menu_by_class(e, 0x3f0);
            let container = e
                .call(
                    DYNAMIC_CAST,
                    &args![open, 0u32, TILE_TYPE, CONTAINER_MENU_TYPE, 0u32],
                )
                .u32();
            if container == 0 {
                return 0;
            }
            let word = e.mem.u32(container.wrapping_add(0x74));
            e.call(CONTAINER_MENU_CREATE, &args![word, 1u32]).u32()
        }
        0x402 => {
            let open = menu_by_class(e, 0x3f0);
            let book_menu = e
                .call(
                    DYNAMIC_CAST,
                    &args![open, 0u32, TILE_TYPE, BOOK_MENU_TYPE, 0u32],
                )
                .u32();
            if book_menu == 0 {
                return 0;
            }
            let reference = e.mem.u32(book_menu.wrapping_add(0x30));
            let object = e.mem.u32(reference.wrapping_add(0x20));
            let book = e
                .call(
                    DYNAMIC_CAST,
                    &args![object, 0u32, BOUND_OBJECT_TYPE, BOOK_TYPE, 0u32],
                )
                .u32();
            delete_menu(e, book_menu);
            e.call(BOOK_MENU_CREATE, &args![book, reference]).u32()
        }
        _ => 0,
    }
}

// Typed from 00720df0
/// The map menu's class, `0x3ff`.
pub fn fn_00720df0(_e: &mut Engine) -> u32 {
    MAP_MENU_CLASS
}

// Typed from 00720e00
/// `MenuManager::DeleteMenu` (Xbox PDB): the open menu of `class` deleted, true; false when none
/// is open.
pub fn menu_manager_delete_menu(e: &mut Engine, _this: Ptr, class: u32) -> bool {
    let open = menu_by_class(e, class as i32);
    if open == 0 {
        return false;
    }
    delete_menu(e, open);
    true
}

// Typed from 00720e60
/// `MenuManager::GetFrontmostMenu` (Xbox PDB): over the menus root's tile list, the menu of the
/// tile whose menu's tile has the greatest depth trait (`0xfad`) above the start value, among those
/// whose kind trait (`0xfa7`) is not 6008.0 (a NaN passes) and whose state is neither 2 nor 4;
/// null when none. The exe's compares: `depth < value` ordered, `!=` with a NaN passing. The list's
/// owner is asked again for each step, but the step's reads (`006317a0`, `007b52d0`) don't read it:
/// the node alone.
pub fn menu_manager_get_frontmost_menu(e: &mut Engine, _this: Ptr) -> u32 {
    let mut frontmost = 0;
    let mut depth = e.mem.f32(FRONTMOST_START_VALUE);
    let mut node = first_menu_node(e);
    while node != 0 {
        let tile = e.mem.u32(node.wrapping_add(8));
        let menu = if tile == 0 {
            0
        } else {
            e.call(TILE_GET_MENU, &args![tile]).u32()
        };
        if menu != 0 {
            let menu_tile = e.mem.u32(menu.wrapping_add(MENU_TILE));
            if menu_tile != 0 && depth < tile_float(e, menu_tile, TRAIT_DEPTH) {
                let menu_tile = e.mem.u32(menu.wrapping_add(MENU_TILE));
                let kind = f64::from(tile_float(e, menu_tile, TRAIT_KIND));
                let state = e.mem.u32(menu.wrapping_add(MENU_STATE));
                if kind != e.mem.f64(FRONTMOST_EXCLUDED_VALUE) && state != 2 && state != 4 {
                    let menu_tile = e.mem.u32(menu.wrapping_add(MENU_TILE));
                    depth = tile_float(e, menu_tile, TRAIT_DEPTH);
                    frontmost = menu;
                }
            }
        }
        node = e.mem.u32(node);
    }
    frontmost
}

// Typed from 00720f90
/// `MenuManager::IsMenuOpen` (Xbox PDB): over the menus root's tile list (each step, `0057cbe0`,
/// the node's item and then its next), the first tile whose menu has a tile and whose class
/// (vtable slot +0x34) is `class`: true when that menu's tile has the enabled trait (`0xfa3`,
/// `Tile::IsTrue`) and its state is not 2. False when no menu matches.
pub fn menu_manager_is_menu_open(e: &mut Engine, _this: Ptr, class: u32) -> bool {
    let mut node = first_menu_node(e);
    while node != 0 {
        let item = node.wrapping_add(8);
        node = e.mem.u32(node);
        let tile = e.mem.u32(item);
        if tile == 0 {
            continue;
        }
        let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
        if menu == 0
            || e.mem.u32(menu.wrapping_add(MENU_TILE)) == 0
            || e.vcall(menu, 0x34, &args![]).u32() != class
        {
            continue;
        }
        let menu_tile = e.mem.u32(menu.wrapping_add(MENU_TILE));
        return typed_call!(
            dispatch::FAMILY,
            0x00a0_1230,
            dispatch::tile::is_true(e, Ptr::new(menu_tile), TRAIT_ENABLED)
        ) && e.mem.u32(menu.wrapping_add(MENU_STATE)) != 2;
    }
    false
}

/// This file's functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0071_e290, menu_manager_instance(bool) -> u32),
        entry!(0x0071_e340, fn_0071e340(Ptr) -> Ptr),
        entry!(0x0071_e360, fn_0071e360()),
        entry!(0x0071_e3b0, fn_0071e3b0(Ptr, u32) -> Ptr),
        entry!(0x0071_e3e0, fn_0071e3e0(Ptr)),
        entry!(0x0071_e3f0, fn_0071e3f0()),
        entry!(0x0071_e400, fn_0071e400(Ptr)),
        entry!(0x0071_e420, menu_manager_create_menu_by_class(Ptr, u32) -> u32),
        entry!(0x0071_f240, fn_0071f240(Ptr) -> Ptr),
        entry!(0x0071_f2d0, tutorial_menu_get_class(Ptr) -> u32),
        entry!(0x0071_f2e0, tutorial_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0071_f310, fn_0071f310(Ptr)),
        entry!(0x0071_f340, fn_0071f340(Ptr) -> Ptr),
        entry!(0x0071_f4a0, fn_0071f4a0(Ptr) -> Ptr),
        entry!(0x0071_f4e0, fn_0071f4e0(Ptr)),
        entry!(0x0071_f520, fn_0071f520(Ptr) -> bool),
        entry!(0x0071_f540, fn_0071f540(Ptr) -> Ptr),
        entry!(0x0071_f570, fn_0071f570(Ptr)),
        entry!(0x0071_f5d0, fn_0071f5d0(Ptr) -> Ptr),
        entry!(0x0071_f600, fn_0071f600(Ptr)),
        entry!(0x0071_f660, stats_menu_get_class(Ptr) -> u32),
        entry!(0x0071_f670, stats_menu_named_effects_list_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0071_f6a0, stats_menu_status_effect_list_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0071_f6d0, stats_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0071_f700, fn_0071f700(Ptr)),
        entry!(0x0071_f8c0, fn_0071f8c0()),
        entry!(0x0071_f900, fn_0071f900(Ptr, u32) -> Ptr),
        entry!(0x0071_f930, fn_0071f930(Ptr)),
        entry!(0x0071_f9c0, fn_0071f9c0(Ptr, u32) -> Ptr),
        entry!(0x0071_f9f0, fn_0071f9f0(Ptr)),
        entry!(0x0071_fa50, stats_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(0x0071_fa80, fn_0071fa80(Ptr) -> Ptr),
        entry!(0x0071_fb20, fn_0071fb20(Ptr) -> u32),
        entry!(0x0071_fb30, fn_0071fb30(Ptr, u32) -> Ptr),
        entry!(0x0071_fb60, fn_0071fb60(Ptr)),
        entry!(0x0071_fbc0, fn_0071fbc0(Ptr, i32, u32)),
        entry!(0x0071_fbf0, fn_0071fbf0(Ptr) -> Ptr),
        entry!(0x0071_feb0, surgery_menu_get_class(Ptr) -> u32),
        entry!(0x0071_fec0, surgery_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0071_fef0, fn_0071fef0(Ptr)),
        entry!(0x0071_ffe0, surgery_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(0x0072_0010, fn_00720010(Ptr, i32) -> i32),
        entry!(0x0072_0040, fn_00720040(Ptr) -> Ptr),
        entry!(0x0072_0100, vats_menu_get_class(Ptr) -> u32),
        entry!(0x0072_0110, vats_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_0140, vats_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(0x0072_0170, fn_00720170(Ptr) -> Ptr),
        entry!(0x0072_0320, special_book_menu_get_class(Ptr) -> u32),
        entry!(0x0072_0330, special_book_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_0360, fn_00720360(Ptr) -> Ptr),
        entry!(0x0072_03c0, fn_007203c0(Ptr)),
        entry!(0x0072_0420, fn_00720420(Ptr)),
        entry!(0x0072_0550, fn_00720550(Ptr) -> Ptr),
        entry!(0x0072_0760, love_tester_menu_get_class(Ptr) -> u32),
        entry!(0x0072_0770, love_tester_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_07a0, fn_007207a0(Ptr) -> Ptr),
        entry!(0x0072_0800, fn_00720800(Ptr)),
        entry!(0x0072_0860, fn_00720860(Ptr)),
        entry!(0x0072_09e0, tutorial_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(0x0072_0a10, fn_00720a10(Ptr) -> Ptr),
        entry!(0x0072_0a30, quantity_menu_get_class(Ptr) -> u32),
        entry!(0x0072_0a40, quantity_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_0a70, fn_00720a70(Ptr)),
        entry!(0x0072_0aa0, menu_manager_reload_menu(Ptr, u32) -> u32),
        entry!(0x0072_0df0, fn_00720df0() -> u32),
        entry!(0x0072_0e00, menu_manager_delete_menu(Ptr, u32) -> bool),
        entry!(0x0072_0e60, menu_manager_get_frontmost_menu(Ptr) -> u32),
        entry!(0x0072_0f90, menu_manager_is_menu_open(Ptr, u32) -> bool),
    ]
}
