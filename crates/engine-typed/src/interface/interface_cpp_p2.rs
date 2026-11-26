//! The interface unit from `00705a90` up to `00709b50` (the translation is engine-fallout-interface's
//! `interface_p2.rs`, which these functions replace at the same addresses). Native as
//! [`super::interface_cpp`] is: the exe's work in its order, the manager singleton and the getters,
//! setters and small helpers it calls done in place ([`IN_PLACE`]), the family's functions and the
//! other families' typed ones called directly, everything else by address (an `overridable.tsv`
//! address always by address: the form by editor id, the item's mod getters, the weapon's
//! projectiles and ammo, the armor rating).

use super::interface_cpp::in_place::{self, manager, ready_manager};
use super::interface_cpp::{fn_007027b0, fn_00703960, fn_00703fe0, fn_007058c0};
use crate::{crt, dispatch, multibound};
use engine_core::prelude::*;
use engine_core::typed_call;

/// What this file does in place besides [`in_place::ADDRESSES`]: plain reads and stores the exe
/// calls (the differential tests leave these out of the translation's calls).
pub const IN_PLACE: &[u32] = &[
    0x0047_ee30, // the manager's byte at +0x12
    0x0084_3150, // +0x14
    0x0098_2bc0, // +0x15
    0x004a_8f20, // the smaller of two integers
    0x0070_b8d0, // does nothing (its one store is to its own argument)
    0x0040_8d60, // a setting's byte (the rendered-menus flag on its constant setting)
    0x008d_f040, // the save object's byte at +0x80
    0x0078_24b0, // the inventory's byte (the object at 011d9ea4, +0x84)
    0x0097_ae90, // the manager's +0xbc
    0x004f_d400, // the manager's +0xa0
    0x004d_c200, // the renderer's height word
    0x004d_c1f0, // the renderer's width word
    0x0043_c4b0, // the object at 011f4748
    0x0045_cd60, // the manager's +0x28
    0x0040_1170, // a form's type byte
    0x0052_9ea0, // a global's kind byte (+0x20)
    0x0052_6ac0, // a global's float (+0x24)
    0x0068_3520, // the item list's step
    0x0068_15c0, // a word's own address
    0x0044_ddc0, // the word at +8 (an item's form)
    0x008d_8520, // the word at +0x68 (the player's saved acquire object)
    0x004d_1360, // the player's +0x7bc is 1
    0x004c_0bd0, // a flag byte's 0x80
    0x0051_4450, // a flag byte's 0x08
    0x004b_e180, // an armor's float at +0x17c
    0x0066_3b60, // a weapon's +0x19c
    0x0066_3b40, // an ammo's name characters (+0xc4)
    0x0040_8da0, // a name's characters or the empty text
    0x0040_ebd0, // the smaller of two floats
    0x0040_e210, // a spell item's flag
    0x0098_4f60, // a rendered menu's menu stored (+0x18)
];

/// `0057c730`: takes the address of a word holding an item and returns the
/// object it removes from the list (0 when there is none).
const LIST_REMOVE: u32 = 0x0057_c730;
/// `Tile::GetMenu` (Xbox PDB): the menu a tile belongs to.
const TILE_GET_MENU: u32 = 0x00a0_3c90;
/// Trait id (`0x1771`) of the menus root tile that is saved and restored
/// with the game.
const TRAIT_SAVED_VALUE: u32 = 0x1771;
/// `Tile::UpdateAll` (Xbox PDB), `cdecl`: one word.
const TILE_UPDATE_ALL: u32 = 0x00a0_4200;
/// `00401000`: `operator new`.
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `FORenderedMenu::FORenderedMenu` (Xbox PDB): constructs the menu in the
/// given block and returns it.
const RENDERED_MENU_CTOR: u32 = 0x0070_5fe0;
/// Size of a `FORenderedMenu`.
const RENDERED_MENU_SIZE: u32 = 0xf0;
/// `00705fc0` on the rendered menu: takes that node.
const RENDERED_MENU_SET_NODE: u32 = 0x0070_5fc0;
/// `InterfaceManager::EnterRenderedMenu` (Xbox PDB), on the manager.
const ENTER_RENDERED_MENU: u32 = 0x0071_81d0;
/// The object `00408d60` is called on.
const RENDERED_MENUS_FLAG_OWNER: u32 = 0x011d_b620;
/// `HackingMenu::Create` (Xbox PDB), `cdecl`: one word.
const HACKING_MENU_CREATE: u32 = 0x0076_5b80;
/// `ComputersMenu::Create` (Xbox PDB), `cdecl`: one word.
const COMPUTERS_MENU_CREATE: u32 = 0x0075_7b70;
/// `__RTDynamicCast` (`00ec43fb`): object, vfptr delta, source type,
/// target type, is-reference.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// Source type descriptor of the casts `CloseAllSupportMenus` makes.
const CAST_SOURCE_TYPE: u32 = 0x0119_f34c;
/// Target type descriptor of the cast for menu id `0x424` (the menu whose
/// close function is `SPECIALBookMenu::Close`).
const SPECIAL_BOOK_MENU_TYPE: u32 = 0x0119_f47c;
/// Target type descriptor of the cast for menu id `0x432`.
const MENU_432_TYPE: u32 = 0x0119_f45c;
/// `SPECIALBookMenu::Close` (Xbox PDB), no arguments.
const SPECIAL_BOOK_MENU_CLOSE: u32 = 0x007c_6120;
/// `007917a0`, no arguments: the close function for menu id `0x432`.
const MENU_432_CLOSE: u32 = 0x0079_17a0;
/// `004a4040`: returns a flag that decides whether menu `0x3f5` is closed.
const CLOSE_MENU_3F5_ALLOWED: u32 = 0x004a_4040;
/// Byte set to 1 while `CloseAllSupportMenus` runs.
const CLOSING_SUPPORT_MENUS: u32 = 0x011d_8909;
/// The loading menu object (0 when none).
const LOADING_MENU: u32 = 0x011d_a0c0;
/// `LoadingMenu::Create` (Xbox PDB), `cdecl`: three words.
const LOADING_MENU_CREATE: u32 = 0x0078_8cc0;
/// The pointer to the game's save object (`011de45c`).
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
/// Writes `(pointer, size)` bytes into the save game (`008579b0`).
const SAVE_GAME_WRITE: u32 = 0x0085_79b0;
/// Save game buffer write (`00865e50`): `(pointer, size, 0)`.
const SAVE_BUFFER_WRITE: u32 = 0x0086_5e50;
/// Load game buffer read (`00864980`): `(pointer, size)`.
const LOAD_BUFFER_READ: u32 = 0x0086_4980;
/// `BGSLoadGameBuffer::LoadString` (Xbox PDB): reads a string into the given
/// buffer.
const LOAD_BUFFER_LOAD_STRING: u32 = 0x0086_49a0;
/// The state getters written by `007066d0` (bytes, then floats) and the
/// setters `LoadGame` hands them to.
const STATS_PAGE_GET: u32 = 0x007d_fcd0;
const MAP_FLOAT_GET_A: u32 = 0x007a_17a0;
const MAP_FLOAT_GET_B: u32 = 0x007a_17e0;
const STATS_PAGE_SET: u32 = 0x007d_fd00;
const MAP_FLOAT_SET_A: u32 = 0x007a_1820;
const MAP_FLOAT_SET_B: u32 = 0x007a_1850;
/// The manager's sub-object at `+0x4d4`: `007187a0` writes a string into a
/// buffer, `00718890` takes one.
const MANAGER_SUBOBJECT: u32 = 0x4d4;
const SUBOBJECT_GET_STRING: u32 = 0x0071_87a0;
const SUBOBJECT_SET_STRING: u32 = 0x0071_8890;
/// Size of the string buffer saved and loaded with the menu state.
const STATE_STRING_SIZE: u32 = 100;
/// Pointer to the object whose `+0x80` the byte `00706830` reads.
const STATE_OBJECT: u32 = 0x011d_a368;
/// Pointer to the object `00706a40` updates, and the word holding the trait
/// id it sets.
const TRAIT_OBJECT: u32 = 0x011d_9ea4;
const TRAIT_OBJECT_TRAIT_ID: u32 = 0x011d_9eac;
/// Byte set by `00706ae0`.
const FINISH_FLAG: u32 = 0x011d_8a85;
/// The object `00953060` is called on, in `00706ae0`.
const FINISH_OBJECT: u32 = 0x011d_ea3c;
/// The `float` at `0106ec38` (an aspect ratio).
const ASPECT_RATIO_CONSTANT: u32 = 0x0106_ec38;
/// The table of `0x1c` pointers (`011d51d0`) `00707330` searches.
const TEXT_TABLE: u32 = 0x011d_51d0;
/// The object (`011dea3c`) the player-name, race and sex texts come from.
const TEXT_OWNER: u32 = 0x011d_ea3c;
/// `"%.2f"`.
const FLOAT_FORMAT: u32 = 0x0103_0018;
/// `"%i"`.
const INTEGER_FORMAT: u32 = 0x0102_0774;
/// `"PCName"`, `"PCRace"`, `"PCSex"`, `"PCSexPronoun"`, `"PCSexPossessive"`.
const PC_NAME_TEXT: u32 = 0x0106_ec6c;
const PC_RACE_TEXT: u32 = 0x0106_ec64;
const PC_SEX_TEXT: u32 = 0x0106_ec5c;
const PC_SEX_PRONOUN_TEXT: u32 = 0x0106_ec4c;
const PC_SEX_POSSESSIVE_TEXT: u32 = 0x0106_ec3c;
/// `":LANGUAGE"`.
const LANGUAGE_SUFFIX: u32 = 0x0106_ec84;
/// `"&true;"` and `"&false;"`.
const TRUE_TEXT: u32 = 0x0106_ec7c;
const FALSE_TEXT: u32 = 0x0106_ec74;
/// The model loader (`011c3b3c`), and the byte (`011d8908`) that makes
/// `00707660` load without it.
const MODEL_LOADER: u32 = 0x011c_3b3c;
const LOADER_DISABLED: u32 = 0x011d_8908;
/// The temporary model (`011d890c`) `00707660` builds and `00707820` frees.
const TEMP_MODEL: u32 = 0x011d_890c;
/// Bytes of the `BSStream` `00707660` keeps on its stack.
const BS_STREAM_SIZE: u32 = 0x5e0;
/// Bytes of the cloning process `00707870` keeps on its stack.
const CLONING_PROCESS_SIZE: u32 = 0x20;
/// The object (`011dea10`) whose deep copy `00707870` calls.
const DEEP_COPY_OWNER: u32 = 0x011d_ea10;

/// `_ftol2` (`00ec62c0`, the C runtime family's) of a double: its low word.
fn double_to_int(e: &mut Engine, value: f64) -> i32 {
    super::low_dword(typed_call!(
        crt::FAMILY,
        0x00ec_62c0,
        crt::math::ftol2_sse(e, value)
    )) as i32
}

/// `_ftol2` on a `float`.
fn float_to_int(e: &mut Engine, value: f32) -> i32 {
    double_to_int(e, f64::from(value))
}

/// `004a8f20`: the smaller of two signed words (the second when they are equal).
#[inline(always)]
fn smaller(a: i32, b: i32) -> i32 {
    if a >= b {
        b
    } else {
        a
    }
}

/// `0040ebd0`: the smaller of two floats as the exe's `FCOMPP` picks it: `b` unless it is above
/// `a` (so `b` for a NaN).
#[inline(always)]
fn smaller_float(a: f32, b: f32) -> f32 {
    if b > a {
        a
    } else {
        b
    }
}

/// `Tile::SetInt` (`00700320`, the dispatch family's).
fn tile_set_int(e: &mut Engine, tile: u32, trait_id: u32, value: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0070_0320,
        dispatch::tile::set_int(e, Ptr::new(tile), trait_id as i32, value as i32)
    );
}

// Typed from 00705a90
/// When the manager is ready: `007058c0` on it (0 otherwise).
pub fn fn_00705a90(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => typed_call!(super::FAMILY, 0x0070_58c0, fn_007058c0(e, manager)),
        None => 0,
    }
}

// Typed from 00705ad0
/// When the manager is ready: stores `value` in it (`00705b10`).
pub fn fn_00705ad0(e: &mut Engine, value: u32) {
    if let Some(current) = ready_manager(e) {
        typed_call!(super::FAMILY, 0x0070_5b10, fn_00705b10(e, current, value));
    }
}

// Typed from 00705b10
/// Stores `value` in the manager's word at `+0xc4`.
pub fn fn_00705b10(e: &mut Engine, this: u32, value: u32) {
    e.mem.set_u32(this.wrapping_add(0xc4), value);
}

// Typed from 00705b30
/// When the manager is ready: `00716010(manager, value)`.
pub fn fn_00705b30(e: &mut Engine, value: u32) {
    if let Some(current) = ready_manager(e) {
        e.call(0x0071_6010, &args![current, value]);
    }
}

// Typed from 00705b70
/// When the manager is ready: gives it three values, each the byte (as a
/// signed byte) raised to at least 0 (`00705c40`) and kept at most 4 (`004a8f20(4, ..)`), to
/// `0070b790`, `0070b810` and `0070b870` (the manager read again after each call, as the exe
/// does). The exe then hands the menu id to `0070b8d0`, whose one store is to its own argument: no
/// effect, so neither the call nor the id's choice is made here (`menu_id` is not read).
pub fn fn_00705b70(e: &mut Engine, first: u8, second: u8, third: u8, _menu_id: i32) {
    if ready_manager(e).is_none() {
        return;
    }
    for (setter, byte) in [
        (0x0070_b790, first),
        (0x0070_b810, second),
        (0x0070_b870, third),
    ] {
        let clamped = typed_call!(
            super::FAMILY,
            0x0070_5c40,
            fn_00705c40(e, 0, i8::from_le_bytes([byte]))
        );
        let value = smaller(4, clamped);
        let current = manager(e);
        e.call(setter, &args![current, value]);
    }
}

// Typed from 00705c40
/// The larger of `a` and the signed byte `b`.
pub fn fn_00705c40(_e: &mut Engine, a: i32, b: i8) -> i32 {
    if a > b as i32 {
        a
    } else {
        b as i32
    }
}

// Typed from 00705c70
/// When the manager is ready: its byte at `+0x12` (`0047ee30`; `0xff` otherwise).
pub fn fn_00705c70(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => e.mem.u8(manager.wrapping_add(0x12)),
        None => 0xff,
    }
}

// Typed from 00705cb0
/// When the manager is ready: the byte at its `+0x13` (`0xff` otherwise).
pub fn fn_00705cb0(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => typed_call!(super::FAMILY, 0x0070_5cf0, fn_00705cf0(e, manager)),
        None => 0xff,
    }
}

// Typed from 00705cf0
/// The byte at `+0x13` of the manager.
pub fn fn_00705cf0(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(this.wrapping_add(0x13))
}

// Typed from 00705d10
/// When the manager is ready: its byte at `+0x14` (`00843150`; `0xff` otherwise).
pub fn fn_00705d10(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => e.mem.u8(manager.wrapping_add(0x14)),
        None => 0xff,
    }
}

// Typed from 00705d50
/// When the manager is ready: its byte at `+0x15` (`00982bc0`; `0xff` otherwise).
pub fn fn_00705d50(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => e.mem.u8(manager.wrapping_add(0x15)),
        None => 0xff,
    }
}

// Typed from 00705d90
/// When the manager is ready: `00705dd0` on it (`-1` otherwise).
pub fn fn_00705d90(e: &mut Engine) -> i32 {
    match ready_manager(e) {
        Some(manager) => typed_call!(super::FAMILY, 0x0070_5dd0, fn_00705dd0(e, manager)),
        None => -1,
    }
}

// Typed from 00705dd0
/// Trait `0x1771` of the tile at `+0x9c` of the manager (`pMenusRoot`),
/// truncated to an integer.
pub fn fn_00705dd0(e: &mut Engine, this: u32) -> i32 {
    let root = in_place::menus_root(e, this);
    let value = tile_float(e, root, TRAIT_SAVED_VALUE);
    float_to_int(e, value)
}

// Typed from 00705e00
/// `Interface::CreateLoadingMenu` (Xbox PDB): sets the loading flag
/// (`00703fe0(1)`), then `LoadingMenu::Create(arg, 1, flag)`.
pub fn interface_create_loading_menu(e: &mut Engine, arg: u32, flag: u8) -> u32 {
    typed_call!(super::FAMILY, 0x0070_3fe0, fn_00703fe0(e, 1));
    e.call(LOADING_MENU_CREATE, &args![arg, 1u32, u32::from(flag)])
        .u32()
}

// Typed from 00705e30
/// `Interface::CloseLoadingMenu` (Xbox PDB): destroys the loading menu
/// object when there is one (its virtual slot 0 with the argument 1), then
/// clears the loading flag (`00703fe0(0)`).
pub fn interface_close_loading_menu(e: &mut Engine) {
    let menu = e.mem.u32(LOADING_MENU);
    if menu != 0 {
        e.vcall(menu, 0, &args![1u32]);
    }
    typed_call!(super::FAMILY, 0x0070_3fe0, fn_00703fe0(e, 0));
}

// Typed from 00705e80
/// `Interface::GetLoadingMenuVisible` (Xbox PDB): `007027b0(0x3ef)`.
pub fn interface_get_loading_menu_visible(e: &mut Engine) -> u8 {
    typed_call!(super::FAMILY, 0x0070_27b0, fn_007027b0(e, 0x3ef))
}

// Typed from 00705ea0
/// `Interface::IsInGameLoadingMenuOpen` (Xbox PDB): whether a loading menu
/// object exists.
pub fn interface_is_in_game_loading_menu_open(e: &mut Engine) -> bool {
    e.mem.u32(LOADING_MENU) != 0
}

/// The tail both rendered-menu creators share: wraps `menu` in a new
/// `FORenderedMenu` and enters it.
fn enter_rendered_menu(e: &mut Engine, menu: u32) {
    let block = e.call(OPERATOR_NEW, &args![RENDERED_MENU_SIZE]).u32();
    let rendered = if block != 0 {
        e.call(RENDERED_MENU_CTOR, &args![block]).u32()
    } else {
        0
    };
    // `00984f60`: the menu stored at the rendered menu's `+0x18`.
    e.mem.set_u32(rendered.wrapping_add(0x18), menu);
    let node = in_place::tile_node(e, menu);
    e.call(RENDERED_MENU_SET_NODE, &args![rendered, node]);
    let current = manager(e);
    e.call(ENTER_RENDERED_MENU, &args![current, rendered]);
}

/// Whether rendered menus are in use (`00408d60` on the setting at `011db620`: its byte at `+4`).
fn rendered_menus_in_use(e: &mut Engine) -> bool {
    e.mem.u8(RENDERED_MENUS_FLAG_OWNER.wrapping_add(4)) != 0
}

// Typed from 00705ec0
/// `Interface::CreateHackingMenu` (Xbox PDB): when the manager is ready,
/// creates the hacking menu (`HackingMenu::Create`); when rendered menus
/// are in use, updates all tiles and enters the menu inside a new
/// `FORenderedMenu`. Returns whether the manager was ready. The
/// compiler's exception frame is not translated.
pub fn interface_create_hacking_menu(e: &mut Engine, arg: u32) -> bool {
    if ready_manager(e).is_none() {
        return false;
    }
    let menu = e.call(HACKING_MENU_CREATE, &args![arg]).u32();
    if rendered_menus_in_use(e) {
        e.call(TILE_UPDATE_ALL, &args![0u32]);
        enter_rendered_menu(e, menu);
    }
    true
}

// Typed from 00706130
/// `Interface::CreateComputersMenu` (Xbox PDB): as
/// [`interface_create_hacking_menu`] for `ComputersMenu::Create`, except that
/// all tiles are updated before the rendered-menus check, whatever its
/// result.
pub fn interface_create_computers_menu(e: &mut Engine, arg: u32) -> bool {
    if ready_manager(e).is_none() {
        return false;
    }
    let menu = e.call(COMPUTERS_MENU_CREATE, &args![arg]).u32();
    e.call(TILE_UPDATE_ALL, &args![0u32]);
    if rendered_menus_in_use(e) {
        enter_rendered_menu(e, menu);
    }
    true
}

// Typed from 00706230
/// When the manager is ready: `007e41f0(a, b)`.
pub fn fn_00706230(e: &mut Engine, a: u32, b: u32) {
    if ready_manager(e).is_some() {
        e.call(0x007e_41f0, &args![a, b]);
    }
}

// Typed from 00706270
/// `Interface::CreateLevelUpMenu` (Xbox PDB): when the manager is ready,
/// `LevelUpMenu::Create`.
pub fn interface_create_level_up_menu(e: &mut Engine) {
    if ready_manager(e).is_some() {
        e.call(0x0078_4c80, &[]);
    }
}

// Typed from 007062a0
/// When the manager is ready: `CharGenMenu::Create(a, b, c, d)`.
pub fn fn_007062a0(e: &mut Engine, a: u32, b: u32, c: u8, d: u8) {
    if ready_manager(e).is_some() {
        e.call(0x0075_3420, &args![a, b, c as u32, d as u32]);
    }
}

// Typed from 007062e0
/// `Interface::SetInfoForRef` (Xbox PDB): when the manager is ready,
/// `HUDMainMenu::SetInfoForRef(reference, b, c)` (a byte; 0 otherwise).
pub fn interface_set_info_for_ref(e: &mut Engine, reference: u32, b: u8, c: u8) -> u8 {
    if ready_manager(e).is_none() {
        return 0;
    }
    e.call(0x0077_5a00, &args![reference, b as u32, c as u32])
        .u8()
}

/// Removes the tile whose pointer is at `tile_slot` from `list` (`0057c730`)
/// and, if that returns an object, calls its virtual slot 0 with the
/// argument 1.
fn remove_tile_and_delete(e: &mut Engine, list: u32, tile_slot: u32) {
    let removed = e.call(LIST_REMOVE, &args![list, tile_slot]).u32();
    if removed != 0 {
        e.vcall(removed, 0, &args![1u32]);
    }
}

/// `__RTDynamicCast` of the tile's menu to `target_type` (0 when the tile is
/// null).
fn cast_tile_menu(e: &mut Engine, tile: u32, target_type: u32) -> u32 {
    if tile == 0 {
        return 0;
    }
    let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
    e.call(
        DYNAMIC_CAST,
        &args![menu, 0u32, CAST_SOURCE_TYPE, target_type, 0u32],
    )
    .u32()
}

// Typed from 00706320
/// `Interface::CloseAllSupportMenus` (Xbox PDB): with the byte at
/// `011d8909` set during the walk, goes over the children of the menus root
/// and, for each tile that belongs to a menu, acts on the menu id (virtual
/// slot `0x34` of the menu):
/// - ids `0x3ea 0x3eb 0x3ec 0x3ef 0x3ff 0x40b 0x425`: kept;
/// - `0x3f5`: removed and deleted only when `004a4040` says so;
/// - `0x424`: `SPECIALBookMenu::Close` when the menu casts to its class,
///   then removed and deleted;
/// - `0x432`: `007917a0` when the menu casts to its class, then removed and
///   deleted;
/// - any other id: removed and deleted.
///
/// The caller pushes one word the function never reads.
pub fn interface_close_all_support_menus(e: &mut Engine, _unused_0: u32) {
    e.mem.set_u8(CLOSING_SUPPORT_MENUS, 1);
    let current = manager(e);
    let list = in_place::menus_root(e, current).wrapping_add(4);
    let mut node = e.mem.u32(list);
    // The current tile is a word of the game's stack (the removal takes its address); the walk's
    // step (`0057cbe0`) is done here: the item is the node's `+8`, the next the node's first word.
    e.with_stack(4, |e, tile_slot| {
        let tile_slot = tile_slot.addr();
        while node != 0 {
            let item = node.wrapping_add(8);
            node = e.mem.u32(node);
            let tile = e.mem.u32(item);
            e.mem.set_u32(tile_slot, tile);
            if tile == 0 || e.call(TILE_GET_MENU, &args![tile]).u32() == 0 {
                continue;
            }
            let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
            let menu_id = e.vcall(menu, 0x34, &[]).u32();
            match menu_id {
                0x3ea | 0x3eb | 0x3ec | 0x3ef | 0x3ff | 0x40b | 0x425 => {}
                0x3f5 => {
                    if e.call(CLOSE_MENU_3F5_ALLOWED, &[]).bool() {
                        remove_tile_and_delete(e, list, tile_slot);
                    }
                }
                0x424 => {
                    if cast_tile_menu(e, tile, SPECIAL_BOOK_MENU_TYPE) != 0 {
                        e.call(SPECIAL_BOOK_MENU_CLOSE, &[]);
                    }
                    remove_tile_and_delete(e, list, tile_slot);
                }
                0x432 => {
                    if cast_tile_menu(e, tile, MENU_432_TYPE) != 0 {
                        e.call(MENU_432_CLOSE, &[]);
                    }
                    remove_tile_and_delete(e, list, tile_slot);
                }
                _ => remove_tile_and_delete(e, list, tile_slot),
            }
        }
    });
    e.mem.set_u8(CLOSING_SUPPORT_MENUS, 0);
}

// Typed from 007065c0
/// 8, or 9 when the byte at `+0x80` (`008df040`) of the object in `011de45c` is at least `0x5d`.
pub fn fn_007065c0(e: &mut Engine) -> u16 {
    let owner = e.mem.u32(SAVE_LOAD_GAME);
    let byte = e.mem.u8(owner.wrapping_add(0x80));
    if byte >= 0x5d {
        9
    } else {
        8
    }
}

// Typed from 00706610
/// `Interface::SaveGame` (Xbox PDB): writes four bytes (`00705c70`,
/// `00705cb0`, `00705d10`, `00705d50`, each raised to at least 1 as a
/// signed byte) and the word `00705d90` into the save game (`008579b0` on
/// the object in `011de45c`).
pub fn interface_save_game(e: &mut Engine) {
    // The game's locals: the word at +0, then the bytes in the order
    // first, fourth, second, third.
    e.with_stack(8, |e, frame| {
        let base = frame.addr();
        let (first, second, third, fourth) = (base + 4, base + 6, base + 7, base + 5);
        let value = typed_call!(super::FAMILY, 0x0070_5c70, fn_00705c70(e));
        e.mem.set_u8(first, value);
        let value = typed_call!(super::FAMILY, 0x0070_5cb0, fn_00705cb0(e));
        e.mem.set_u8(second, value);
        let value = typed_call!(super::FAMILY, 0x0070_5d10, fn_00705d10(e));
        e.mem.set_u8(third, value);
        let value = typed_call!(super::FAMILY, 0x0070_5d50, fn_00705d50(e));
        e.mem.set_u8(fourth, value);
        let word = typed_call!(super::FAMILY, 0x0070_5d90, fn_00705d90(e));
        e.mem.set_i32(base, word);
        for byte in [first, second, third, fourth] {
            if e.mem.i8(byte) < 1 {
                e.mem.set_i8(byte, 1);
            }
        }
        for byte in [first, second, third, fourth] {
            let owner = e.mem.u32(SAVE_LOAD_GAME);
            e.call(SAVE_GAME_WRITE, &args![owner, byte, 1u32]);
        }
        let owner = e.mem.u32(SAVE_LOAD_GAME);
        e.call(SAVE_GAME_WRITE, &args![owner, base, 4u32]);
    });
}

// Typed from 007066d0
/// Writes the menu state into the save game buffer `stream`: trait `0x1771`
/// of the menus root as an integer, the bytes of `007dfcd0`, `007824b0` and
/// `00706830`, the floats of `007a17a0` and `007a17e0`, then the word
/// `00706810` and the string of the manager's sub-object (`007187a0` into a
/// 100-byte buffer, written with that length). [`interface_load_game`] reads
/// it back.
pub fn fn_007066d0(e: &mut Engine, stream: u32) {
    // The game's frame, from its lowest local: the length word at +0, the
    // 100-byte buffer at +8, the bytes at +7, +0x7b and +0x83, the trait
    // value at +0x74, the floats at +0x84 and +0x7c.
    e.with_stack(0x88, |e, frame| {
        let base = frame.addr();
        let (length, text) = (base, base + 8);
        let (first_byte, second_byte, third_byte) = (base + 7, base + 0x7b, base + 0x83);
        let (trait_value, first_float, second_float) = (base + 0x74, base + 0x84, base + 0x7c);

        let current = manager(e);
        let root = in_place::menus_root(e, current);
        let value = tile_float(e, root, TRAIT_SAVED_VALUE);
        let truncated = float_to_int(e, value);
        e.mem.set_i32(trait_value, truncated);
        let byte = e.call(STATS_PAGE_GET, &[]).u8();
        e.mem.set_u8(first_byte, byte);
        // `007824b0`: the inventory's byte (the object at `011d9ea4`, its `+0x84`; 0 for none).
        let inventory = e.mem.u32(TRAIT_OBJECT);
        let byte = if inventory == 0 {
            0
        } else {
            e.mem.u8(inventory.wrapping_add(0x84))
        };
        e.mem.set_u8(second_byte, byte);
        let byte = typed_call!(super::FAMILY, 0x0070_6830, fn_00706830(e));
        e.mem.set_u8(third_byte, byte);
        for (data, size) in [
            (trait_value, 4u32),
            (first_byte, 1),
            (second_byte, 1),
            (third_byte, 1),
        ] {
            e.call(SAVE_BUFFER_WRITE, &args![stream, data, size, 0u32]);
        }
        let float = e.call(MAP_FLOAT_GET_A, &[]).f32();
        e.mem.set_f32(first_float, float);
        let float = e.call(MAP_FLOAT_GET_B, &[]).f32();
        e.mem.set_f32(second_float, float);
        e.call(SAVE_BUFFER_WRITE, &args![stream, first_float, 4u32, 0u32]);
        e.call(SAVE_BUFFER_WRITE, &args![stream, second_float, 4u32, 0u32]);

        zero(e, text, STATE_STRING_SIZE);
        let current = manager(e);
        let size = typed_call!(
            super::FAMILY,
            0x0070_6810,
            fn_00706810(e, current.wrapping_add(MANAGER_SUBOBJECT))
        );
        e.mem.set_i32(length, size);
        e.call(
            SUBOBJECT_GET_STRING,
            &args![current.wrapping_add(MANAGER_SUBOBJECT), text],
        );
        e.call(SAVE_BUFFER_WRITE, &args![stream, length, 4u32, 0u32]);
        e.call(SAVE_BUFFER_WRITE, &args![stream, text, size as u32, 0u32]);
    });
}

// Typed from 00706810
/// `004a8f20(1, 1) + 5`, the smaller of 1 and 1 plus 5: 6 (the object it is called on is not
/// used).
pub fn fn_00706810(_e: &mut Engine, _unused_0: u32) -> i32 {
    smaller(1, 1) + 5
}

/// `00403d30(at, 0, size)` (`memset`'s wrapper): the bytes zeroed.
fn zero(e: &mut Engine, at: u32, size: u32) {
    for i in 0..size {
        e.mem.set_u8(at.wrapping_add(i), 0);
    }
}

// Typed from 00706830
/// The low byte of `(word at +0x80 of the object in 011da368) - 0x20`, or 4
/// when there is no object.
pub fn fn_00706830(e: &mut Engine) -> u8 {
    let object = e.mem.u32(STATE_OBJECT);
    if object == 0 {
        4
    } else {
        e.mem.u32(object + 0x80).wrapping_sub(0x20).to_le_bytes()[0]
    }
}

// Typed from 00706860
/// `Interface::LoadGame` (Xbox PDB): reads from the load game buffer
/// `stream` the state [`fn_007066d0`] wrote (a word, three bytes, two
/// floats) and applies it: the word to trait `0x1771` of the menus root,
/// the bytes to `StatsMenu::SetCurrentStatsPage`, `00706a20` and `007069d0`,
/// the floats to `007a1820` and `007a1850`. Then, if the stream's version
/// byte (virtual slot 0) is at least 12, reads a length and that many bytes
/// of text into a 100-byte buffer for the manager's sub-object
/// (`00718890`); otherwise reads a string (`LoadString`) that nobody uses.
pub fn interface_load_game(e: &mut Engine, stream: u32) {
    // The game's frame, from its lowest local: the LoadString buffer at +0,
    // the length word at +0x10c, the 100-byte buffer at +0x110, the saved
    // word and bytes at +0x184, +0x183, +0x18b and +0x193, the floats at
    // +0x194 and +0x18c.
    e.with_stack(0x198, |e, frame| {
        let base = frame.addr();
        let (word, first_byte, second_byte, third_byte) =
            (base + 0x184, base + 0x183, base + 0x18b, base + 0x193);
        let (first_float, second_float) = (base + 0x194, base + 0x18c);
        e.mem.set_u32(word, 0);
        e.mem.set_u8(first_byte, 0);
        e.mem.set_u8(second_byte, 0);
        e.mem.set_u8(third_byte, 0);
        e.call(LOAD_BUFFER_READ, &args![stream, word, 4u32]);
        e.call(LOAD_BUFFER_READ, &args![stream, first_byte, 1u32]);
        e.call(LOAD_BUFFER_READ, &args![stream, second_byte, 1u32]);
        e.call(LOAD_BUFFER_READ, &args![stream, third_byte, 1u32]);
        let current = manager(e);
        let root = in_place::menus_root(e, current);
        let saved = e.mem.u32(word);
        tile_set_int(e, root, TRAIT_SAVED_VALUE, saved);
        let byte = e.mem.u8(first_byte);
        e.call(STATS_PAGE_SET, &args![u32::from(byte)]);
        let byte = e.mem.u8(second_byte);
        typed_call!(super::FAMILY, 0x0070_6a20, fn_00706a20(e, byte));
        let byte = e.mem.u8(third_byte);
        typed_call!(super::FAMILY, 0x0070_69d0, fn_007069d0(e, byte));
        e.mem.set_f32(first_float, 0.0);
        e.mem.set_f32(second_float, 0.0);
        e.call(LOAD_BUFFER_READ, &args![stream, first_float, 4u32]);
        e.call(LOAD_BUFFER_READ, &args![stream, second_float, 4u32]);
        let float = e.mem.f32(first_float);
        e.call(MAP_FLOAT_SET_A, &args![float]);
        let float = e.mem.f32(second_float);
        e.call(MAP_FLOAT_SET_B, &args![float]);
        let version = e.vcall(stream, 0, &[]).u8();
        if version >= 12 {
            // Under `SUB ESP,0x198` (00706863): the length word is EBP-0x8c
            // (00706968/00706974), the buffer EBP-0x88 (00706959, 0070698a,
            // 00706999).
            let (length, text) = (base + 0x10c, base + 0x110);
            zero(e, text, STATE_STRING_SIZE);
            e.mem.set_u32(length, 0);
            e.call(LOAD_BUFFER_READ, &args![stream, length, 4u32]);
            let size = e.mem.u32(length);
            e.call(LOAD_BUFFER_READ, &args![stream, text, size]);
            let current = manager(e);
            e.call(
                SUBOBJECT_SET_STRING,
                &args![current.wrapping_add(MANAGER_SUBOBJECT), text],
            );
        } else {
            e.call(LOAD_BUFFER_LOAD_STRING, &args![stream, base]);
        }
    });
}

// Typed from 007069d0
/// When the object in `011da368` exists: `007069f0(object, value)`.
pub fn fn_007069d0(e: &mut Engine, value: u8) {
    let object = e.mem.u32(STATE_OBJECT);
    if object != 0 {
        typed_call!(
            super::FAMILY,
            0x0070_69f0,
            fn_007069f0(e, object, value as u32)
        );
    }
}

// Typed from 007069f0
/// Virtual slot `0xc` of the object with `(value + 0x20, 0)`.
pub fn fn_007069f0(e: &mut Engine, this: u32, value: u32) {
    e.vcall(this, 0xc, &args![value.wrapping_add(0x20), 0u32]);
}

// Typed from 00706a20
/// `00706a40` with the byte.
pub fn fn_00706a20(e: &mut Engine, value: u8) {
    typed_call!(super::FAMILY, 0x0070_6a40, fn_00706a40(e, value as u32));
}

// Typed from 00706a40
/// When the object in `011d9ea4` exists: keeps `value` at most 4 (`004a8f20(value, 4)`, signed),
/// sets that trait (the id in `011d9eac`) on the object's tile (`+0x5c`) and stores it at `+0x84`.
pub fn fn_00706a40(e: &mut Engine, value: u32) {
    let object = e.mem.u32(TRAIT_OBJECT);
    if object == 0 {
        return;
    }
    let value = smaller(value as i32, 4) as u32;
    let trait_id = e.mem.u32(TRAIT_OBJECT_TRAIT_ID);
    let tile = e.mem.u32(object.wrapping_add(0x5c));
    tile_set_int(e, tile, trait_id, value);
    e.mem.set_u32(object.wrapping_add(0x84), value);
}

// Typed from 00706a90
/// `Interface::FinishLoadGame` (Xbox PDB): calls, in order, `007dd710`,
/// `007df4e0`, `007df5d0`, `0079f640`, `00706ac0`, `007a1490` and
/// `007e5890`.
pub fn interface_finish_load_game(e: &mut Engine) {
    e.call(0x007d_d710, &[]);
    e.call(0x007d_f4e0, &[]);
    e.call(0x007d_f5d0, &[]);
    e.call(0x0079_f640, &[]);
    typed_call!(super::FAMILY, 0x0070_6ac0, fn_00706ac0(e));
    e.call(0x007a_1490, &[]);
    e.call(0x007e_5890, &[]);
}

// Typed from 00706ac0
/// Clears the word at `+0x90` of the object in `011da368`, if any.
pub fn fn_00706ac0(e: &mut Engine) {
    let object = e.mem.u32(STATE_OBJECT);
    if object != 0 {
        e.mem.set_u32(object + 0x90, 0);
    }
}

// Typed from 00706ae0
/// Sets the byte `011d8a85`, runs the functions `007053b0` and `00773050`,
/// and when `007050d0` says so sets a value (`00705190(0.0, 1)`) on the
/// object in `011dea3c` (`00953060`) and runs `0083e510`; then `00705780`,
/// `007f6920`, `CloseAllSupportMenus`, `007aa480`, `007dda60`, `00706ac0`,
/// `00798bf0`, `00706fd0(1, 1)`, `EnterRenderedMenu(0)`, clears the
/// manager's word at `+0x4bc` and runs `00706b90` on its sub-object.
pub fn fn_00706ae0(e: &mut Engine) {
    use super::interface_cpp as unit;
    e.mem.set_u8(FINISH_FLAG, 1);
    typed_call!(super::FAMILY, 0x0070_53b0, unit::fn_007053b0(e));
    e.call(0x0077_3050, &[]);
    if typed_call!(super::FAMILY, 0x0070_50d0, unit::interface_in_dialog(e)) != 0 {
        // 00706b08: `00705190` takes no word (plain RET, no stack read); the pushed 0.0
        // and 1 are the last two words of `00953060` (RET 0xc).
        let value = typed_call!(
            super::FAMILY,
            0x0070_5190,
            unit::interface_in_dialog_with(e)
        );
        let object = e.mem.u32(FINISH_OBJECT);
        e.call(0x0095_3060, &args![object, value, 0.0f32, 1u32]);
        e.call(0x0083_e510, &[]);
    }
    typed_call!(
        super::FAMILY,
        0x0070_5780,
        unit::interface_close_vats_menu(e)
    );
    e.call(0x007f_6920, &[]);
    typed_call!(
        super::FAMILY,
        0x0070_6320,
        interface_close_all_support_menus(e, 0)
    );
    e.call(0x007a_a480, &[]);
    e.call(0x007d_da60, &[]);
    typed_call!(super::FAMILY, 0x0070_6ac0, fn_00706ac0(e));
    e.call(0x0079_8bf0, &[]);
    typed_call!(
        super::FAMILY,
        0x0070_6fd0,
        interface_pop_from_enter_stack(e, 1, 1)
    );
    let current = manager(e);
    e.call(ENTER_RENDERED_MENU, &args![current, 0u32]);
    let current = manager(e);
    e.mem.set_u32(current.wrapping_add(0x4bc), 0);
    typed_call!(
        super::FAMILY,
        0x0070_6b90,
        fn_00706b90(e, current.wrapping_add(MANAGER_SUBOBJECT))
    );
}

// Typed from 00706b90
/// Resets the manager sub-object's table: clears bits 0 and 1 of each of
/// its first `0x29` words and keeps only their low byte, then sets `+0xa8`
/// to 0 and `+0xa4` to `0x29`.
pub fn fn_00706b90(e: &mut Engine, this: u32) {
    for index in 0..0x29u32 {
        let slot = this + 4 * index;
        let mut word = e.mem.u32(slot);
        word &= 0xffff_fffe;
        word &= 0xffff_fffd;
        word &= 0xff;
        e.mem.set_u32(slot, word);
    }
    e.mem.set_u32(this + 0xa8, 0);
    e.mem.set_u32(this + 0xa4, 0x29);
}

// Typed from 00706c20
/// `Interface::TileIsBeingDeleted` (Xbox PDB): forgets `tile` in the
/// manager: clears its slots at `+0xcc`/`+0xd0` and `+0xd4`/`+0xd8` when
/// they hold it, runs `00706cb0` when its `+0xbc` (`0097ae90`) holds it, and finally
/// `00713de0(manager, tile)`.
pub fn interface_tile_is_being_deleted(e: &mut Engine, tile: u32) {
    let current = manager(e);
    let at = |offset: u32| current.wrapping_add(offset);
    if e.mem.u32(at(0xcc)) == tile {
        e.mem.set_u32(at(0xcc), 0);
        e.mem.set_u32(at(0xd0), 0);
    }
    if e.mem.u32(at(0xd4)) == tile {
        e.mem.set_u32(at(0xd4), 0);
        e.mem.set_u32(at(0xd8), 0);
    }
    if e.mem.u32(at(0xbc)) == tile {
        typed_call!(super::FAMILY, 0x0070_6cb0, fn_00706cb0(e, current));
    }
    e.call(0x0071_3de0, &args![current, tile]);
}

// Typed from 00706cb0
/// Clears the manager's word at `+0xbc`.
pub fn fn_00706cb0(e: &mut Engine, this: u32) {
    e.mem.set_u32(this.wrapping_add(0xbc), 0);
}

// Typed from 00706cd0
/// `Interface::GetMenusRoot` (Xbox PDB): the manager's `+0x9c` (`00586150`).
pub fn interface_get_menus_root(e: &mut Engine) -> u32 {
    let current = manager(e);
    in_place::menus_root(e, current)
}

// Typed from 00706cf0
/// The manager's word at `+0xa0` (`004fd400`).
pub fn fn_00706cf0(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.mem.u32(current.wrapping_add(0xa0))
}

// Typed from 00706d10
/// `Interface::GetDebugTextRoot` (Xbox PDB): the manager's word at `+0xa4` (`005e3fc0`).
pub fn interface_get_debug_text_root(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.mem.u32(current.wrapping_add(in_place::NODE_A))
}

// Typed from 00706d30
/// `Interface::GetInterfaceRoot` (Xbox PDB): the manager's word at `+0x80` (`004fb070`).
pub fn interface_get_interface_root(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.mem.u32(current.wrapping_add(in_place::DIALOG_WORD))
}

// Typed from 00706d50
/// `Interface::AttachDefaultAlphaProperty` (Xbox PDB): takes the property
/// `006629f0` (the multibound family's) returns for the manager and attaches it to `node` with
/// `NiAVObject::AttachProperty` (`00439410`).
pub fn interface_attach_default_alpha_property(e: &mut Engine, node: u32) {
    let current = manager(e);
    let property = typed_call!(
        multibound::FAMILY,
        0x0066_29f0,
        multibound::getters::multibound_of(e, Ptr::new(current))
    );
    e.call(0x0043_9410, &args![node, property]);
}

// Typed from 00706d70
/// `Interface::GetFirstChanceTextureRelease` (Xbox PDB): the byte at
/// `+0xec` of the manager (see `00706d90`).
pub fn interface_get_first_chance_texture_release(e: &mut Engine) -> u8 {
    let current = manager(e);
    typed_call!(super::FAMILY, 0x0070_6d90, fn_00706d90(e, current))
}

// Typed from 00706d90
/// The byte at `+0xec` of `this` (the manager's first-chance texture
/// release flag, which `00703960` sets).
pub fn fn_00706d90(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(this.wrapping_add(0xec))
}

// Typed from 00706db0
/// `00703960(manager, value)`: sets the manager's byte at `+0xec`.
pub fn fn_00706db0(e: &mut Engine, value: u8) {
    let current = manager(e);
    typed_call!(super::FAMILY, 0x0070_3960, fn_00703960(e, current, value));
}

// Typed from 00706dd0
/// Stores the byte in the manager's byte at `+0xdc`.
pub fn fn_00706dd0(e: &mut Engine, value: u8) {
    let current = manager(e);
    e.mem.set_u8(current.wrapping_add(0xdc), value);
}

// Typed from 00706df0
/// `Interface::GetScreenHeight` (Xbox PDB): the `float` `00715da0` returns
/// in `ST0`.
pub fn interface_get_screen_height(e: &mut Engine) -> f32 {
    e.call(0x0071_5da0, &[]).f32()
}

// Typed from 00706e00
/// `Interface::GetScreenWidth` (Xbox PDB): the `float`
/// `InterfaceManager::GetScreenWidth` (`00715d40`) returns in `ST0`.
pub fn interface_get_screen_width(e: &mut Engine) -> f32 {
    e.call(0x0071_5d40, &[]).f32()
}

// Typed from 00706e10
/// `Interface::GetRealScreenHeight` (Xbox PDB): `00706e20`.
pub fn interface_get_real_screen_height(e: &mut Engine) -> f32 {
    typed_call!(super::FAMILY, 0x0070_6e20, fn_00706e20(e))
}

// Typed from 00706e20
/// The renderer's integer (`004dc200`: the word at `01189480`) as a float
/// in `ST0`.
pub fn fn_00706e20(e: &mut Engine) -> f32 {
    e.mem.i32(RENDERER_HEIGHT_WORD) as f32
}

/// The renderer's two integers `004dc200` and `004dc1f0` answer.
const RENDERER_HEIGHT_WORD: u32 = 0x0118_9480;
const RENDERER_WIDTH_WORD: u32 = 0x0118_947c;

// Typed from 00706e40
/// `Interface::GetRealScreenWidth` (Xbox PDB): `00706e50`.
pub fn interface_get_real_screen_width(e: &mut Engine) -> f32 {
    typed_call!(super::FAMILY, 0x0070_6e50, fn_00706e50(e))
}

// Typed from 00706e50
/// The renderer's integer (`004dc1f0`: the word at `0118947c`) as a float in `ST0`.
pub fn fn_00706e50(e: &mut Engine) -> f32 {
    e.mem.i32(RENDERER_WIDTH_WORD) as f32
}

// Typed from 00706e70
/// `Interface::GetScreenAspectRatio` (Xbox PDB): the manager's `float` at
/// `+0x4d0`.
pub fn interface_get_screen_aspect_ratio(e: &mut Engine) -> f32 {
    let current = manager(e);
    e.mem.f32(current.wrapping_add(0x4d0))
}

// Typed from 00706e80
/// The `float` constant at `0106ec38` (an aspect ratio).
pub fn fn_00706e80(e: &mut Engine) -> f32 {
    e.global::<f32>(ASPECT_RATIO_CONSTANT)
}

// Typed from 00706e90
/// A float from virtual calls on the object `0043c4b0` returns: the
/// constant of `00706e80` times (slot `0x90` of one result of that object's
/// slot `0xc8`, called with 0) divided by (slot `0x8c` of another result of
/// the same call, called with 0), the two counts read as unsigned.
pub fn fn_00706e90(e: &mut Engine) -> f32 {
    // `0043c4b0`: the word at `011f4748`.
    let object = e.mem.u32(0x011f_4748);
    let first = e.vcall(object, 0xc8, &[]).u32();
    let second = e.vcall(object, 0xc8, &[]).u32();
    let divisor = f64::from(e.vcall(second, 0x8c, &args![0u32]).u32());
    let dividend = f64::from(e.vcall(first, 0x90, &args![0u32]).u32());
    // The exe's two 24-bit steps: the `FDIV` rounded to a float, then the `FMUL` by the float
    // (exact in an `f64`) rounded once.
    let ratio = super::narrow(dividend / divisor);
    let base = typed_call!(super::FAMILY, 0x0070_6e80, fn_00706e80(e));
    super::narrow(f64::from(base) * f64::from(ratio))
}

// Typed from 00706f20
/// The constant `0x3ec`.
pub fn fn_00706f20(_e: &mut Engine) -> u32 {
    0x3ec
}

// Typed from 00706f30
/// `Interface::PlayMenuSound` (Xbox PDB): `InterfaceManager::PlayMenuSound`
/// (`00717280`, `cdecl`) with the sound id.
pub fn interface_play_menu_sound(e: &mut Engine, sound: u32) {
    e.call(0x0071_7280, &args![sound]);
}

// Typed from 00706f50
/// `Interface::NewTimer` (Xbox PDB): `InterfaceManager::NewTimer`
/// (`007164c0`, `cdecl`) with the word and the `float`.
pub fn interface_new_timer(e: &mut Engine, id: u32, seconds: f32) {
    e.call(0x0071_64c0, &args![id, seconds]);
}

// Typed from 00706f70
/// `Interface::UpdateAllTimers` (Xbox PDB): `InterfaceManager::
/// UpdateAllTimers` (`00716320`) on the manager.
pub fn interface_update_all_timers(e: &mut Engine) {
    let current = manager(e);
    e.call(0x0071_6320, &args![current]);
}

// Typed from 00706f90
/// `Interface::ClearTimer` (Xbox PDB): `InterfaceManager::ClearTimer`
/// (`007165d0`, `cdecl`, finds the manager itself) with the timer id. (The
/// game also reads the manager first and ignores it.)
pub fn interface_clear_timer(e: &mut Engine, id: u32) {
    e.call(0x0071_65d0, &args![id]);
}

// Typed from 00706fb0
/// `Interface::AddToEnterStack` (Xbox PDB): `InterfaceManager::
/// AddToEnterStack` (`00714d90`) on the manager.
pub fn interface_add_to_enter_stack(e: &mut Engine, menu_id: u32) -> u32 {
    let current = manager(e);
    e.call(0x0071_4d90, &args![current, menu_id]).u32()
}

// Typed from 00706fd0
/// `Interface::PopFromEnterStack` (Xbox PDB): `InterfaceManager::
/// PopFromEnterStack` (`00714fd0`) on the manager, with the menu id and a
/// byte flag.
pub fn interface_pop_from_enter_stack(e: &mut Engine, menu_id: u32, flag: u8) -> u32 {
    let current = manager(e);
    e.call(0x0071_4fd0, &args![current, menu_id, flag]).u32()
}

// Typed from 00706ff0
/// `Interface::SetCurrentFocusTarget` (Xbox PDB): `InterfaceManager::
/// SetCurrentFocusTarget` (`00715860`) on the manager with the tile, the
/// word `0xfc3` and the flag 1.
pub fn interface_set_current_focus_target(e: &mut Engine, tile: u32) {
    let current = manager(e);
    e.call(0x0071_5860, &args![current, tile, 0xfc3u32, 1u32]);
}

// Typed from 00707010
/// `Interface::RecursiveFade` (Xbox PDB): `InterfaceManager::RecursiveFade`
/// (`00712450`) on the manager with the tile and the two floats.
pub fn interface_recursive_fade(e: &mut Engine, tile: u32, from: f32, to: f32) {
    let current = manager(e);
    e.call(0x0071_2450, &args![current, tile, from, to]);
}

// Typed from 00707040
/// The manager's word at `+0xcc`.
pub fn fn_00707040(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.mem.u32(current.wrapping_add(0xcc))
}

// Typed from 00707050
/// Stores the word in the manager's `+0xcc`.
pub fn fn_00707050(e: &mut Engine, value: u32) {
    let current = manager(e);
    e.mem.set_u32(current.wrapping_add(0xcc), value);
}

// Typed from 00707070
/// The manager's word at `+0xd0`.
pub fn fn_00707070(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.mem.u32(current.wrapping_add(0xd0))
}

// Typed from 00707080
/// Stores the word in the manager's `+0xd0`.
pub fn fn_00707080(e: &mut Engine, value: u32) {
    let current = manager(e);
    e.mem.set_u32(current.wrapping_add(0xd0), value);
}

// Typed from 007070a0
/// The manager's word at `+0x28` (`0045cd60`).
pub fn fn_007070a0(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.mem.u32(current.wrapping_add(0x28))
}

/// The bounded string copy (`00406d30`: destination, size, source) the
/// text replacement uses.
fn copy_text(e: &mut Engine, dest: u32, size: u32, src: u32) {
    e.call(0x0040_6d30, &args![dest, size, src]);
}

/// Whether the constant `name` equals the string at `label` (`00404dc0`
/// returns 0 for equal).
fn name_is(e: &mut Engine, name: u32, label: u32) -> bool {
    e.call(0x0040_4dc0, &args![name, label]).i32() == 0
}

// Typed from 007070c0
/// `Interface::FindTextReplacementString` (Xbox PDB): writes into `dest`
/// (`size` bytes) the text that replaces the constant `name` and returns
/// whether it found one. In order, the first that applies wins:
///
/// 1. a name `00707330` knows (an index below `0x1c`):
///    `GetControlPushTextFor(index, dest, flag)`;
/// 2. a form with that editor ID (`00483a00`) whose type byte (`+4`) is 6:
///    its value, formatted as `"%.2f"` when its byte at `+0x20` is `'f'` and
///    as `"%i"` otherwise;
/// 3. `"PCName"` (`0055d520` on the object in `011dea3c`), `"PCRace"`
///    (`0087f6c0`), `"PCSex"`, `"PCSexPronoun"` and `"PCSexPossessive"`
///    (a string of the tables at `0119b354`, `0119b364`, `0119b35c`,
///    indexed by `0087f4c0`), each compared with `00404dc0`;
/// 4. `007073d0` (a game-setting lookup).
///
/// The result is false for a null or empty name, buffer or size.
pub fn interface_find_text_replacement_string(
    e: &mut Engine,
    name: u32,
    dest: u32,
    size: u32,
    flag: u8,
) -> bool {
    if name == 0 || e.mem.u8(name) == 0 || dest == 0 || size == 0 {
        return false;
    }
    let mut found = false;
    let index = typed_call!(super::FAMILY, 0x0070_7330, fn_00707330(e, name));
    if index < 0x1c {
        typed_call!(
            super::FAMILY,
            0x0070_39b0,
            super::interface_cpp::interface_get_control_push_text_for(e, index, dest, flag)
        );
        found = true;
    }
    if !found {
        // `00483a00` (the form by editor id) is overridable: by address.
        let form = e.call(0x0048_3a00, &args![name]).u32();
        if form != 0 && form_type(e, form) == 6 {
            // `00529ea0` and `00526ac0`: the global's kind byte (`+0x20`) and value (`+0x24`).
            let kind = e.mem.i8(form.wrapping_add(0x20));
            let value = e.mem.f32(form.wrapping_add(0x24));
            if kind == 0x66 {
                e.call(
                    0x0040_6d00,
                    &args![dest, size, FLOAT_FORMAT, f64::from(value)],
                );
            } else {
                let whole = float_to_int(e, value);
                e.call(0x0040_6d00, &args![dest, size, INTEGER_FORMAT, whole]);
            }
            found = true;
        }
    }
    if !found && name_is(e, name, PC_NAME_TEXT) {
        let owner = e.mem.u32(TEXT_OWNER);
        let text = e.call(0x0055_d520, &args![owner]).u32();
        copy_text(e, dest, size, text);
        found = true;
    }
    if !found && name_is(e, name, PC_RACE_TEXT) {
        let owner = e.mem.u32(TEXT_OWNER);
        let text = e.call(0x0087_f6c0, &args![owner]).u32();
        copy_text(e, dest, size, text);
        found = true;
    }
    for (label, table) in [
        (PC_SEX_TEXT, 0x0119_b354u32),
        (PC_SEX_PRONOUN_TEXT, 0x0119_b364),
        (PC_SEX_POSSESSIVE_TEXT, 0x0119_b35c),
    ] {
        if !found && name_is(e, name, label) {
            let owner = e.mem.u32(TEXT_OWNER);
            let which = e.call(0x0087_f4c0, &args![owner]).u32();
            let entry = e.mem.u32(table.wrapping_add(which.wrapping_mul(4)));
            let text = in_place::setting_text(e, entry);
            copy_text(e, dest, size, text);
            found = true;
        }
    }
    if !found {
        found = typed_call!(super::FAMILY, 0x0070_73d0, fn_007073d0(e, name, dest));
    }
    found
}

// Typed from 00707330
/// `Interface::MatchTextReplaceConstantToGameSettings` (Xbox PDB): the
/// index in the table of `0x1c` pointers at `011d51d0` of the entry equal to
/// the game setting `name` (`004f8a30` on the object `00404a70` returns),
/// else of the first whose `00403df0` text matches `name` by `00469880`;
/// `0x1c` when none does.
///
/// `00404a70` and `004f8a30` are called as the game calls them: the first
/// leaves `name` on the stack for the second.
pub fn fn_00707330(e: &mut Engine, name: u32) -> i32 {
    let settings = e.call(0x0040_4a70, &[]).u32();
    let wanted = e.call(0x004f_8a30, &args![settings, name]).u32();
    let mut index = 0u32;
    while index < 0x1c && e.mem.u32(TEXT_TABLE + 4 * index) != wanted {
        index += 1;
    }
    if index == 0x1c {
        index = 0;
        while index < 0x1c {
            let entry = e.mem.u32(TEXT_TABLE + 4 * index);
            let text = in_place::setting_text(e, entry);
            if e.call(0x0046_9880, &args![text, name]).i32() == 0 {
                break;
            }
            index += 1;
        }
    }
    index as i32
}

// Typed from 007073d0
/// `Interface::TestConstantForGameSettings` (Xbox PDB): looks the constant
/// `name` up as a game setting and writes its value as text into `dest`;
/// false (and nothing written) when `name` is null or empty or no setting
/// is found.
///
/// A leading `&` and then a leading `-` are skipped, the rest is copied into
/// a `0x104`-byte buffer and a trailing `;` is cut. The setting is looked up
/// by `004f8a30` (on the object `00404a70` returns); when it is missing,
/// `":LANGUAGE"` is appended and `005e02b0` (on the object `0044f560`
/// returns) tries again. The first letter of the buffer gives the type: `s`
/// copies the setting's text, `i` formats its integer with `"%i"`, `f`
/// formats its float, also with `"%i"` (as the exe does), and `b` copies
/// `"&true;"` when its byte is 1 and `"&false;"` otherwise. The stack cookie
/// check is not translated.
pub fn fn_007073d0(e: &mut Engine, name: u32, dest: u32) -> bool {
    if name == 0 || e.mem.u8(name) == 0 {
        return false;
    }
    e.with_stack(0x104, |e, buffer| {
        let buffer = buffer.addr();
        let mut skip = 0u32;
        if e.mem.u8(name) == b'&' {
            skip += 1;
        }
        if e.mem.u8(name + skip) == b'-' {
            skip += 1;
        }
        e.call(0x0040_6d30, &args![buffer, 0x104u32, name + skip]);
        let length = e.call(0x0044_a670, &args![buffer]).u32();
        if e.mem.u8(buffer + length.wrapping_sub(1)) == b';' {
            let length = e.call(0x0044_a670, &args![buffer]).u32();
            e.mem.set_u8(buffer + length.wrapping_sub(1), 0);
        }
        let settings = e.call(0x0040_4a70, &[]).u32();
        let mut setting = e.call(0x004f_8a30, &args![settings, buffer]).u32();
        if setting == 0 {
            e.call(0x0040_6d50, &args![buffer, 0x104u32, LANGUAGE_SUFFIX]);
            let owner = e.call(0x0044_f560, &[]).u32();
            setting = e.call(0x005e_02b0, &args![owner, buffer]).u32();
        }
        if setting == 0 {
            return false;
        }
        // A found setting is never null, so its value (`0043d4d0`, `00403e20`, `00408d60`) is
        // at its `+4`.
        let value_at = setting.wrapping_add(4);
        match e.mem.u8(buffer) {
            b's' | b'S' => {
                let text = in_place::setting_text(e, setting);
                e.call(0x0040_46f0, &args![dest, text]);
            }
            b'i' | b'I' => {
                let value = e.mem.u32(value_at);
                e.call(0x00ec_623a, &args![buffer, INTEGER_FORMAT, value]);
                e.call(0x0040_46f0, &args![dest, buffer]);
            }
            b'f' | b'F' => {
                let value = f64::from(e.mem.f32(value_at));
                e.call(0x00ec_623a, &args![buffer, INTEGER_FORMAT, value]);
                e.call(0x0040_46f0, &args![dest, buffer]);
            }
            b'b' | b'B' => {
                let text = if e.mem.u8(value_at) == 1 {
                    TRUE_TEXT
                } else {
                    FALSE_TEXT
                };
                e.call(0x0040_46f0, &args![dest, text]);
            }
            _ => {}
        }
        true
    })
}

// Typed from 00707640
/// `Interface::ReleaseModelFile` (Xbox PDB): when the model loader
/// (`011c3b3c`) exists, `0045a5e0(loader, model)` (releases the model).
pub fn interface_release_model_file(e: &mut Engine, model: u32) {
    let loader = e.mem.u32(MODEL_LOADER);
    if loader != 0 {
        e.call(0x0045_a5e0, &args![loader, model]);
    }
}

// Typed from 00707660
/// `Interface::LoadModelFile` (Xbox PDB): loads the model file `path` and
/// returns the model (0 when there is none). The byte at `loaded_by_loader`
/// is set to 1 when the model loader (`011c3b3c`) did it.
///
/// With a loader and the byte `011d8908` clear, `ModelLoader::LoadFile`
/// (`00447080`) does everything. Otherwise, if the file exists
/// (`FileFinder::Exist`, `00456a20`) it is read into a `BSStream` on the
/// stack; the previous temporary model in `011d890c` is destroyed, a new
/// 0x10-byte `Model` (`0043aaf0`) is built from the stream, kept in
/// `011d890c`, and `0043b230` on it gives the result. The compiler's
/// exception-unwinding frame and stack cookie are not translated.
pub fn interface_load_model_file(e: &mut Engine, path: u32, loaded_by_loader: u32) -> u32 {
    e.mem.set_u8(loaded_by_loader, 0);
    let loader = e.mem.u32(MODEL_LOADER);
    if loader != 0 && e.mem.u8(LOADER_DISABLED) == 0 {
        e.mem.set_u8(loaded_by_loader, 1);
        return e
            .call(
                0x0044_7080,
                &args![loader, path, 0u32, 1u32, 0u32, 0u32, 0u32],
            )
            .u32();
    }
    if e.call(0x0045_6a20, &args![path, 0u32, 0u32, -1i32]).u32() == 0 {
        return 0;
    }
    e.with_stack(BS_STREAM_SIZE, |e, stream| {
        let stream = stream.addr();
        e.call(0x0043_cfd0, &args![stream]);
        let loaded = e.call(0x00c3_a8a0, &args![stream, path, 0u32]).bool();
        if !loaded {
            e.call(0x0043_d090, &args![stream]);
            return 0;
        }
        let previous = e.mem.u32(TEMP_MODEL);
        if previous != 0 {
            e.call(0x0044_31f0, &args![previous, 1u32]);
        }
        let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
        let model = if block != 0 {
            e.call(0x0043_aaf0, &args![block, path, stream, 1u32, 0u32])
                .u32()
        } else {
            0
        };
        e.mem.set_u32(TEMP_MODEL, model);
        let result = typed_call!(
            dispatch::FAMILY,
            0x0043_b230,
            dispatch::scene::first_controller(e, Ptr::new(model))
        )
        .addr();
        e.call(0x0043_d090, &args![stream]);
        result
    })
}

// Typed from 00707820
/// `Interface::ClearTempModel` (Xbox PDB): destroys the model in `011d890c`
/// (`004431f0(model, 1)`) if there is one and clears the pointer.
pub fn interface_clear_temp_model(e: &mut Engine) {
    let model = e.mem.u32(TEMP_MODEL);
    if model != 0 {
        e.call(0x0044_31f0, &args![model, 1u32]);
    }
    e.mem.set_u32(TEMP_MODEL, 0);
}

// Typed from 00707860
/// The address `0x101dccc` of a string in the exe (`"Meshes"`).
pub fn fn_00707860(_e: &mut Engine) -> u32 {
    0x0101_dccc
}

// Typed from 00707870
/// `Interface::CopyOrDeepCopyNode` (Xbox PDB): copies the node, with a
/// cloning-process object (`004ad050`, scale 1.0, destroyed by `004ad270`)
/// on the stack. A node `004b5bf0` accepts is copied by
/// `TES::CreateDeepCopySameTextures` (`00457ba0`, on the object in
/// `011dea10`); any other by `NiObject::Clone` (`00a5d2c0`). The compiler's
/// exception-unwinding frame is not translated.
pub fn interface_copy_or_deep_copy_node(e: &mut Engine, node: u32) -> u32 {
    e.with_stack(CLONING_PROCESS_SIZE, |e, process| {
        let process = process.addr();
        e.call(0x004a_d050, &args![process, 1.0f32]);
        let copy = if e.call(0x004b_5bf0, &args![node]).bool() {
            let owner = e.mem.u32(DEEP_COPY_OWNER);
            e.call(0x0045_7ba0, &args![owner, node, process]).u32()
        } else {
            e.call(0x00a5_d2c0, &args![node, process]).u32()
        };
        e.call(0x004a_d270, &args![process]);
        copy
    })
}

/// `InterfaceManager::IsInPipboyMenu` (Xbox PDB), static, returns a byte.
const MANAGER_IS_IN_PIPBOY_MENU: u32 = 0x0071_78a0;
/// `InterfaceManager::IsCurrentRenderedMenuTopmost` (Xbox PDB), static.
const MANAGER_IS_CURRENT_RENDERED_MENU_TOPMOST: u32 = 0x0071_7990;
/// `00726070`: returns the word at `+4` of its object (a list's first node,
/// an item's count, a menu's tile).
/// `BSAnimGroupSequence::PlaySounds` (Xbox PDB), `cdecl`.
const ANIM_GROUP_PLAY_SOUNDS: u32 = 0x004e_ef00;
/// Trait ids of a menu tile that `Interface::IsRenderedMenu` reads
/// (truncated to integers): the first is compared with `0x70`, the second
/// with `0x421`, `0x41f` and `0x40c`.
const TRAIT_0FAA: u32 = 0xfaa;
const TRAIT_0FA4: u32 = 0xfa4;

/// The static guard (`011d8a4c`, bit 0) and the cached `float`
/// (`011d8a48`) of `00707b60`.
const SCREEN_RATIO_GUARD: u32 = 0x011d_8a4c;
const SCREEN_RATIO_CACHE: u32 = 0x011d_8a48;
/// The byte at `011c70eb`, returned by `00707b50`.
const RETURNED_FLAG: u32 = 0x011c_70eb;

/// `BSStringT<char>` (8 bytes: characters pointer, length word, capacity
/// word): constructor, `Set(characters, 0)`, length, destructor.
const STRING_SET: u32 = 0x0040_37f0;
const STRING_LENGTH: u32 = 0x0040_48e0;
const STRING_DESTROY: u32 = 0x0040_37d0;
/// `00438390` on a string: `Set(characters)`.
const STRING_ASSIGN: u32 = 0x0043_8390;
/// `Tile::TextToTrait` (Xbox PDB), `cdecl`: the trait id of a name.
const TILE_TEXT_TO_TRAIT: u32 = 0x00a0_1860;
/// `Menu::RenderTemplate` (Xbox PDB), on the menu: parent tile, template
/// name, a word stored in the menu.
const MENU_RENDER_TEMPLATE: u32 = 0x00a1_ddb0;
/// Names (strings in the exe) `00707be0` looks up or turns into traits.
const TAB_BUTTON_TEMPLATE: u32 = 0x0106_ecb4;
const LEFT_LINE_LENGTH_NAME: u32 = 0x0106_eca4;
const BUTTON_COUNT_NAME: u32 = 0x0106_ec94;
const X_NAME: u32 = 0x0106_ec90;
/// Trait `0xfb1` of a tile (the size `00707be0` adds up).
const TRAIT_0FB1: u32 = 0xfb1;

/// `Tile::SetParent` (Xbox PDB): tile, parent, a word.
const TILE_SET_PARENT: u32 = 0x00a0_87d0;
/// `Menu::SetMenuTile` (Xbox PDB), on the menu: tile, a word.
const MENU_SET_MENU_TILE: u32 = 0x00a1_dc70;
/// `007ab690`: constructor of the menu object (vtable `010757fc`) that
/// `HandleQueuedMenuOpen` builds for queued menu kind 9 (the tile it is given
/// is named "Player Name Entry Menu").
const NAME_ENTRY_MENU_CONSTRUCT: u32 = 0x007a_b690;
/// `00409480` on the tile: takes that menu.
const NAME_ENTRY_MENU_FINISH: u32 = 0x0040_9480;
/// The tile name `HandleQueuedMenuOpen` gives queued menu kind 9.
const NAME_ENTRY_MENU_NAME: u32 = 0x0106_ed28;
/// `RaceSexMenu::Create` (Xbox PDB), `cdecl`: one word.
const RACE_SEX_MENU_CREATE: u32 = 0x007a_c730;
/// `StartMenu::Create` (Xbox PDB), `cdecl`: two words.
const START_MENU_CREATE: u32 = 0x007c_b7d0;
/// `HUDMainMenu::SetMenuMode` (Xbox PDB), `cdecl`: the mode.
const HUD_SET_MENU_MODE: u32 = 0x0077_1700;
/// `MessageMenu::Create` (Xbox PDB), no arguments.
const MESSAGE_MENU_CREATE: u32 = 0x007a_8ba0;
/// `CompanionWheelMenu::CreateCompanionWheelMenu` (Xbox PDB), `cdecl`.
const COMPANION_WHEEL_MENU_CREATE: u32 = 0x0075_4cf0;
/// Source and target type descriptors of the cast made for queued menu
/// kind 8.
const COMPANION_CAST_SOURCE: u32 = 0x0118_41cc;
const COMPANION_CAST_TARGET: u32 = 0x0118_46d4;
/// `008c7aa0`: returns a byte; `QueueMenuCreate` opens the menu at once only
/// when it is clear.
const QUEUE_MENU_BLOCKED: u32 = 0x008c_7aa0;
/// The lock (`011d89c0`) held while the queue is handled: `0040fbf0`
/// enter (one more word, 0), `0040fba0` leave.
const QUEUE_LOCK: u32 = 0x011d_89c0;
/// The first queued menu: kind (`011d8950`), then its four words at
/// `+4`, `+8`, `+0xc` and `+0x14` (`+0x10` is written elsewhere).
const QUEUE_KIND: u32 = 0x011d_8950;
const QUEUE_FIRST: u32 = 0x011d_8954;
const QUEUE_SECOND: u32 = 0x011d_8958;
const QUEUE_THIRD: u32 = 0x011d_895c;
const QUEUE_RACE_SEX: u32 = 0x011d_8960;
const QUEUE_FOURTH: u32 = 0x011d_8964;
/// The second queued menu (only kind 7, the message menu): `011d8998`, its
/// first word at `011d899c`, then `011d89a0`, `011d89a4` and `011d89ac`.
const MESSAGE_QUEUE_KIND: u32 = 0x011d_8998;
const MESSAGE_QUEUE_FIRST: u32 = 0x011d_899c;
const MESSAGE_QUEUE_SECOND: u32 = 0x011d_89a0;
const MESSAGE_QUEUE_THIRD: u32 = 0x011d_89a4;
const MESSAGE_QUEUE_FOURTH: u32 = 0x011d_89ac;

/// Vtables of `Tile` (`0106ed9c`), `TileRect` (`0106ed70`) and `TileMenu`
/// (`0106ed44`) (RTTI `.?AVTile@@`, `.?AVTileRect@@`, `.?AVTileMenu@@`).
const TILE_VTABLE: u32 = 0x0106_ed9c;
const TILE_RECT_VTABLE: u32 = 0x0106_ed70;
const TILE_MENU_VTABLE: u32 = 0x0106_ed44;
/// `Tile::~Tile` (Xbox PDB), `Tile::Release` (Xbox PDB), and
/// `00a1eff0` (the `TileMenu` destructor).
const TILE_DESTRUCTOR: u32 = 0x009f_f340;
const TILE_RELEASE: u32 = 0x009f_f690;
const TILE_MENU_DESTRUCTOR: u32 = 0x00a1_eff0;
/// `operator delete` (`00401030`), `cdecl`.
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// Constructors the `Tile` constructor runs on its members: `0048f200`
/// (at `+4`), `00709e80` (at `+0x10`), the string constructor (at
/// `+0x20`), and `00633c90` and `0066b0d0` (both at `+0x2c`, one word 0;
/// the first done in place: with null it stores the null and adds no
/// reference, the menu manager file's `IN_PLACE`).
const TILE_MEMBER_AT_4: u32 = 0x0048_f200;
const TILE_MEMBER_AT_2C_SECOND: u32 = 0x0066_b0d0;
/// The `float` at `01016264`, returned by `00709ac0`.
const RETURNED_SCALE: u32 = 0x0101_6264;
/// Static menu functions `00709ae0` .. `00709b40` forward to.
const SURGERY_MENU_FN: u32 = 0x007e_47a0;
const LOCKPICK_MENU_FN: u32 = 0x0078_e7c0;
const SLOT_MACHINE_MENU_FN: u32 = 0x007c_19c0;
const BLACKJACK_MENU_FN: u32 = 0x0073_3500;
const ROULETTE_MENU_FN: u32 = 0x007b_bcf0;
const CARAVAN_MENU_FN: u32 = 0x0074_0f30;
const HUD_MAIN_MENU_FN: u32 = 0x0077_f0d0;

/// Item stats display (`00707e30`): the exe's strings and constants, and the
/// scratch frame the game keeps on the stack (the `float` the entry-point
/// calls fill, and the text buffers).
const TITLE_NAME: u32 = 0x0106_ed20;
const VALUE_NAME: u32 = 0x0106_ed18;
const CONDITION_ARROWS_NAME: u32 = 0x0106_ed0c;
const CONDITION_METER_NAME: u32 = 0x0106_ecfc;
const STATS_FRAME_SIZE: u32 = 0x400;
const FRAME_ENTRY_OUT: u32 = 0x004;
const FRAME_VALUE_TEXT: u32 = 0x010;
const FRAME_WEIGHT_TEXT: u32 = 0x020;
const FRAME_ARMOR_TEXT: u32 = 0x040;
const FRAME_WEAPON_TEXT: u32 = 0x0c0;
const FRAME_MOD_TEXT: u32 = 0x140;
const FRAME_AMMO_TEXT: u32 = 0x1d0;
const FRAME_EFFECT_TEXT: u32 = 0x2e0;
/// The player object (a pointer global).
const PLAYER: u32 = 0x011d_ea3c;
/// `ItemChange` (Xbox PDB name of the `004bd400` family; the entry of an
/// inventory list) getters: value, form (`+8`), health, mod slots, ownership,
/// a mod effect flag, the mod item in a slot (on the form).
const ITEM_GET_VALUE: u32 = 0x004b_d400;
const ITEM_GET_HEALTH: u32 = 0x004b_cdb0;
const ITEM_GET_MOD_SLOTS: u32 = 0x004b_d820;
const ITEM_GET_OWNERSHIP: u32 = 0x004b_d740;
const ITEM_HAS_MOD_EFFECT: u32 = 0x004b_da70;
const ITEM_FORM_GET_MOD: u32 = 0x004b_d570;
/// `TESWeightForm::GetFormWeight` (Xbox PDB), `cdecl`: form, flag.
const GET_FORM_WEIGHT: u32 = 0x0048_ebc0;
/// `00406d00`: `sprintf`-style (destination, size, format, ...), and the
/// strlen `0044a670`.
const FORMAT_TEXT: u32 = 0x0040_6d00;
const STRING_LENGTH_C: u32 = 0x0044_a670;
/// `004bd510(value, divisor)` (`cdecl`, two `float`s, result in `ST0`): with
/// `q = value / divisor` and `n` its truncation, `(n + 1 if q - n >= 0.5
/// else n) * divisor`.
const ROUND_DIVIDE: u32 = 0x004b_d510;
/// `00476b20` (`cdecl`): one `float` in, a `float` out (it calls `00404090`).
const ROUND_FLOAT: u32 = 0x0047_6b20;
/// `TESHealthForm::GetFormHealth` (Xbox PDB), `cdecl`.
const GET_FORM_HEALTH: u32 = 0x0048_73d0;
/// `CombatFormulas::CalcArmorRating` (Xbox PDB), `cdecl`: integer, `float`.
const CALC_ARMOR_RATING: u32 = 0x0064_6360;
/// `004be060` and `004be180` on an armor form: the two values (`ST0`) whose
/// truncation `CalcArmorRating` takes.
const ARMOR_RATING_INPUT_A: u32 = 0x004b_e060;
/// Source and target type descriptors of the armor cast, and the three
/// pointer globals to the game settings that title the armor cell.
const ARMOR_CAST_SOURCE: u32 = 0x0118_3108;
const ARMOR_CAST_TARGET: u32 = 0x0118_3a34;
const ARMOR_SETTING_A: u32 = 0x0118_a644;
const ARMOR_SETTING_B: u32 = 0x0118_a648;
const ARMOR_SETTING_C: u32 = 0x0118_a640;
/// Flag getters on the cast object's sub-object at `+0x70` (byte at `+8`,
/// tested with `0x80` and `0x08`).
/// `BGSEntryPoint::HandleEntryPoint` (Xbox PDB), `cdecl`: entry point id,
/// the player, then the form (some entry points) and the address of the
/// `float` it updates.
const HANDLE_ENTRY_POINT: u32 = 0x005e_58f0;
/// `TESObjectWEAP::GetFormWeight` (Xbox PDB), on the weapon: a flag.
const WEAPON_GET_WEIGHT: u32 = 0x004b_e380;
/// `00645380` (weapon damage, `cdecl`, 13 words) and
/// `CombatFormulas::CalcWeaponDamageForDisplay` (Xbox PDB, `cdecl`).
const CALC_WEAPON_DAMAGE: u32 = 0x0064_5380;
const CALC_WEAPON_DAMAGE_FOR_DISPLAY: u32 = 0x0064_50f0;
/// `TESObjectWEAP::GetNumProjectiles`, `GetCurrentAmmo`,
/// `GetFormClipRounds` and `GetAmmoRegenRate` (Xbox PDB), on the weapon.
const WEAPON_GET_NUM_PROJECTILES: u32 = 0x0052_5b20;
const WEAPON_GET_CURRENT_AMMO: u32 = 0x0052_5980;
const WEAPON_GET_CLIP_ROUNDS: u32 = 0x004f_e160;
const WEAPON_GET_AMMO_REGEN_RATE: u32 = 0x0070_9430;
/// `InventoryChanges::GetInventoryChanges` (Xbox PDB), `cdecl`: the player;
/// `InventoryChanges::GetObjectCount` (Xbox PDB), on it: a form.
const GET_INVENTORY_CHANGES: u32 = 0x004b_f220;
/// `TESAmmo::BuildMenuString` (Xbox PDB) and `EffectItemList::BuildMenuString`
/// (Xbox PDB): buffer, size.
const AMMO_BUILD_MENU_STRING: u32 = 0x0050_3a70;
const EFFECT_LIST_BUILD_MENU_STRING: u32 = 0x0040_6620;
/// `TESEnchantableForm::GetFormEnchanting` (Xbox PDB), `cdecl`, and the flag
/// test `0040e210` on the enchantment (one word).
const GET_FORM_ENCHANTING: u32 = 0x004b_e330;
/// The record tag `'DESC'` passed to the description getter of a form of
/// type `0x67` (slot `0x10` of the sub-object at `+0x74`).
const DESCRIPTION_TAG: u32 = 0x4353_4544;
/// Game setting objects and their strings used by the item stats display.
const SETTING_MODS_TITLE: u32 = 0x011d_4024;
const LANGUAGE_SETTING: u32 = 0x011c_3cb4;
const WEIGHT_LIMIT_SETTING: u32 = 0x011c_6478;
const WEIGHT_FACTOR_SETTING: u32 = 0x011c_64a8;
/// `00408b20(a, b)`: string comparison (0 when equal).
const COMPARE_TEXT: u32 = 0x0040_8b20;
const ENGLISH_TEXT: u32 = 0x0101_8348;
/// Constants in the exe: `100.0`, `10.0`, `41.0`, `1.0`, `2.0`, `20.0`
/// (doubles) and `255.0` (`float`).
const PERCENT_DIVISOR: u32 = 0x0101_7a40;
const HEAVY_WEIGHT: u32 = 0x0102_0758;
const MOD_ROW_HEIGHT: u32 = 0x0103_5808;
const ONE: u32 = 0x0101_2070;
const TWO: u32 = 0x0101_1590;
const TWENTY: u32 = 0x0102_fc70;
const CELL_10_TRAIT_FA9: u32 = 0x0102_3cd8;
/// Texts in the exe: "--", "%d", "%s", "%s (%i/%i)", "%.1fx%d", "%.0f",
/// "%.1f" and "Mod 1".."Mod 3".
const DASHES: u32 = 0x0104_7008;
const FORMAT_INTEGER: u32 = 0x0102_0764;
const FORMAT_STRING: u32 = 0x0101_9f08;
const FORMAT_AMMO: u32 = 0x0106_ece8;
const FORMAT_PROJECTILES: u32 = 0x0106_ecf4;
const FORMAT_NO_DECIMAL: u32 = 0x0106_ecc8;
const FORMAT_ONE_DECIMAL: u32 = 0x0102_0768;
const MOD_1_TEXT: u32 = 0x0106_ece0;
const MOD_2_TEXT: u32 = 0x0106_ecd8;
const MOD_3_TEXT: u32 = 0x0106_ecd0;
/// Trait ids of the item stats cells: visibility (`0xfa3`), two layout
/// traits and the one the display copies from its condition meter to the
/// condition arrows (`0xfb0`, `0xfa2`).
const TRAIT_0FA3: u32 = 0xfa3;
const TRAIT_0FA2: u32 = 0xfa2;
const TRAIT_0FB0: u32 = 0xfb0;
const TRAIT_0FA9: u32 = 0xfa9;
/// The game settings whose strings title the cells when the language is not
/// English, with the cell each goes to.
const LANGUAGE_TITLES: [(u32, usize); 9] = [
    (0x011d_20c4, 0),
    (0x011d_412c, 1),
    (0x011d_2c7c, 11),
    (0x011d_2aa8, 2),
    (0x011d_26a0, 3),
    (0x011d_4da4, 4),
    (0x011d_4edc, 6),
    (0x011d_4024, 7),
    (0x011d_330c, 10),
];
/// The `flags` bit that shows each cell, in the order the game sets the
/// visibility trait (cell 10 is handled separately).
const VISIBILITY_BITS: [(u32, usize); 12] = [
    (0x1, 0),
    (0x1000, 12),
    (0x2, 1),
    (0x800, 11),
    (0x4, 2),
    (0x8, 3),
    (0x10, 4),
    (0x20, 5),
    (0x40, 6),
    (0x80, 7),
    (0x100, 8),
    (0x200, 9),
];

/// A tile trait read as a `float` (`Tile::GetFloat`, `00a011b0`, the dispatch family's).
fn tile_float(e: &mut Engine, tile: u32, trait_id: u32) -> f32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_11b0,
        dispatch::tile::get_float(e, Ptr::new(tile), trait_id as i32)
    )
}

/// The tile the path `path` names from `tile` (`00a08b20`, the dispatch family's).
fn find_tile(e: &mut Engine, tile: u32, path: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_8b20,
        dispatch::tile::find_by_path(e, Ptr::new(tile), Ptr::new(path))
    )
    .addr()
}

/// The float trait setter (`00a012d0`, the dispatch family's), with the flag 1 every caller here
/// passes.
fn tile_set_float(e: &mut Engine, tile: u32, trait_id: u32, value: f32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_12d0,
        dispatch::tile::set_float(e, Ptr::new(tile), trait_id as i32, value, 1)
    );
}

/// The string trait setter (`00a01350`, the dispatch family's), with the flag 1 every caller here
/// passes.
fn tile_set_text(e: &mut Engine, tile: u32, trait_id: u32, text: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1350,
        dispatch::tile::set_string(e, Ptr::new(tile), trait_id as i32, Ptr::new(text), 1)
    );
}

// Typed from 00707930
/// `Interface::SetNeedToUpdate` (Xbox PDB): sets the manager's
/// `bNeedToUpdate` (`00707950`); it does not check that the manager is
/// ready.
pub fn interface_set_need_to_update(e: &mut Engine, value: u8) {
    let this = manager(e);
    typed_call!(super::FAMILY, 0x0070_7950, fn_00707950(e, this, value));
}

// Typed from 00707950
/// Stores `value` in the manager's byte at `+0xb0` (`bNeedToUpdate`,
/// Xbox PDB).
pub fn fn_00707950(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(this + 0xb0, value);
}

// Typed from 00707970
/// `cdecl`: `BSAnimGroupSequence::PlaySounds(sequence, 0)` (Xbox PDB name of
/// `004eef00`), whose answer it answers (the exe ends on that call and leaves
/// its `EAX`; the caller tests `AL`).
pub fn fn_00707970(e: &mut Engine, sequence: u32) -> bool {
    e.call(ANIM_GROUP_PLAY_SOUNDS, &args![sequence, 0u32])
        .bool()
}

// Typed from 00707990
/// `Interface::CreateMenuByClass` (Xbox PDB; a `Menu*` there), `cdecl`: asks
/// the menu manager (`MenuManager::Instance(1)`) to create the menu of class
/// `class_id` and answers what that gives (the exe ends on that call and
/// leaves its `EAX`, which the caller keeps).
pub fn interface_create_menu_by_class(e: &mut Engine, class_id: u32) -> u32 {
    let menu_manager = typed_call!(
        super::FAMILY,
        0x0071_e290,
        super::menu_manager::menu_manager_instance(e, true)
    );
    let manager = Ptr::new(menu_manager);
    typed_call!(
        super::FAMILY,
        0x0071_e420,
        super::menu_manager::menu_manager_create_menu_by_class(e, manager, class_id)
    )
}

// Typed from 007079b0
/// `Interface::IsInRenderedMenu` (Xbox PDB): when the Pipboy menu is open
/// and `00974d90` on the Pipboy manager says no, false; otherwise
/// [`fn_007079f0`] on the interface manager.
pub fn interface_is_in_rendered_menu(e: &mut Engine) -> bool {
    use super::interface_cpp as unit;
    if typed_call!(
        super::FAMILY,
        0x0070_5a00,
        unit::interface_is_in_pipboy_menu(e)
    ) != 0
    {
        let pipboy = typed_call!(super::FAMILY, 0x0070_5990, unit::interface_get_pipboy(e));
        // `00974d90`: the Pip-Boy's byte at `+0x28`.
        if e.mem.u8(pipboy.wrapping_add(in_place::PIPBOY_QUERY)) == 0 {
            return false;
        }
    }
    let this = manager(e);
    typed_call!(super::FAMILY, 0x0070_79f0, fn_007079f0(e, this))
}

// Typed from 007079f0
/// The manager's `bIsInRenderedMenu` (Xbox PDB, byte at `+0x168`) is set or
/// `InterfaceManager::IsInPipboyMenu` (`007178a0`) says so.
pub fn fn_007079f0(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this.wrapping_add(0x168)) != 0 || e.call(MANAGER_IS_IN_PIPBOY_MENU, &[]).u8() != 0
}

// Typed from 00707a30
/// `Interface::IsRenderedMenu` (Xbox PDB), `cdecl`: false for no menu or a
/// menu without a tile (the word at `+4`, `00726070`); otherwise true when
/// the tile's trait `0xfaa` (as an integer) is `0x70` or its trait `0xfa4`
/// is `0x421`, `0x41f` or `0x40c`.
pub fn interface_is_rendered_menu(e: &mut Engine, menu: u32) -> bool {
    let tile = if menu != 0 {
        e.mem.u32(menu.wrapping_add(4))
    } else {
        0
    };
    if tile == 0 {
        return false;
    }
    let first = tile_float(e, tile, TRAIT_0FAA);
    let first = float_to_int(e, first);
    let second = tile_float(e, tile, TRAIT_0FA4);
    let second = float_to_int(e, second);
    first == 0x70 || second == 0x421 || second == 0x41f || second == 0x40c
}

// Typed from 00707ad0
/// `Interface::GetCurrentRenderedMenu` (Xbox PDB): [`fn_00707af0`] on the
/// interface manager.
pub fn interface_get_current_rendered_menu(e: &mut Engine) -> u32 {
    let this = manager(e);
    typed_call!(super::FAMILY, 0x0070_7af0, fn_00707af0(e, this))
}

// Typed from 00707af0
/// The manager's `pPipboy` (Xbox PDB, `+0x174`) when
/// `InterfaceManager::IsInPipboyMenu` (`007178a0`) says so, otherwise its
/// `pCurrentRenderedMenu` (`+0x16c`).
pub fn fn_00707af0(e: &mut Engine, this: u32) -> u32 {
    if e.call(MANAGER_IS_IN_PIPBOY_MENU, &[]).u8() != 0 {
        e.mem.u32(this.wrapping_add(0x174))
    } else {
        e.mem.u32(this.wrapping_add(0x16c))
    }
}

// Typed from 00707b30
/// `Interface::IsCurrentRenderedMenuTopmost` (Xbox PDB): forwards to
/// `InterfaceManager::IsCurrentRenderedMenuTopmost` (`00717990`, static).
/// (The game first reads the manager and ignores it.)
pub fn interface_is_current_rendered_menu_topmost(e: &mut Engine) -> bool {
    e.call(MANAGER_IS_CURRENT_RENDERED_MENU_TOPMOST, &[]).u8() != 0
}

// Typed from 00707b40
/// Returns [`fn_00707b50`].
pub fn fn_00707b40(e: &mut Engine) -> u8 {
    typed_call!(super::FAMILY, 0x0070_7b50, fn_00707b50(e))
}

// Typed from 00707b50
/// The byte global at `011c70eb`.
pub fn fn_00707b50(e: &mut Engine) -> u8 {
    e.global::<u8>(RETURNED_FLAG)
}

// Typed from 00707b60
/// A function-local static `float`: the first call sets the guard bit
/// (`011d8a4c`) and stores `Interface::GetScreenHeight()` divided by the
/// integer `004dc200` returns (`011d8a48`); every call returns the stored
/// value. The compiler's exception-unwinding frame is not translated.
pub fn fn_00707b60(e: &mut Engine) -> f32 {
    if e.global::<u32>(SCREEN_RATIO_GUARD) & 1 == 0 {
        let guard = e.global::<u32>(SCREEN_RATIO_GUARD);
        e.set_global(SCREEN_RATIO_GUARD, guard | 1);
        let height = typed_call!(super::FAMILY, 0x0070_6df0, interface_get_screen_height(e));
        let divisor = e.mem.i32(RENDERER_HEIGHT_WORD);
        e.set_global(
            SCREEN_RATIO_CACHE,
            super::narrow(height as f64 / divisor as f64),
        );
    }
    e.global::<f32>(SCREEN_RATIO_CACHE)
}

/// `Interface::InitializeTabline` (Xbox PDB), `cdecl`, varargs: lays out a
/// tab line on `tile`. `first_name` and the words of `more_names` are the
/// names (character pointers) of the tab buttons; the list ends at a null
/// pointer (or at the end of `more_names`, where the game would read the
/// next stack word).
///
/// For each name a "TabButtonTemplate" is rendered into `tile` by its menu
/// (`Menu::RenderTemplate`), given the name as its trait `0xfc4` text and
/// as its tile name (`00707e10`), the trait `0xfac` = its index and trait
/// `0xfaa` = `base_index` + its index, then updated with
/// `Tile::UpdateAll(0)`; the sizes (trait `0xfb1`, truncated) are added up.
/// The line length left over, `(tile's 0xfb1 - sum) / (count + 1)`
/// truncated, is stored as `_LeftLineLength` and the count as `_ButtonCount`
/// on `tile`. A second pass sets `_x` of each child found by name: the first
/// at the left line length, each next at the previous plus its size plus the
/// left line length. The temporary `BSStringT` is built and destroyed on the
/// stack; the compiler's exception-unwinding frame is not translated.
pub fn initialize_tabline(
    e: &mut Engine,
    tile: u32,
    base_index: u32,
    first_name: u32,
    more_names: &[u32],
) {
    let name_at = |index: usize| -> u32 {
        if index == 0 {
            first_name
        } else {
            more_names.get(index - 1).copied().unwrap_or(0)
        }
    };
    e.with_stack(8, |e, string| {
        let string = string.addr();
        in_place::empty_string(e, string);
        e.call(STRING_SET, &args![string, first_name, 0u32]);
        let mut count: u32 = 0;
        let mut total: i32 = 0;
        while e.call(STRING_LENGTH, &args![string]).u32() != 0 {
            let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
            let button = e
                .call(
                    MENU_RENDER_TEMPLATE,
                    &args![menu, tile, TAB_BUTTON_TEMPLATE, 0u32],
                )
                .u32();
            let characters = e.mem.u32(string);
            tile_set_text(e, button, 0xfc4, characters);
            let characters = e.mem.u32(string);
            typed_call!(
                super::FAMILY,
                0x0070_7e10,
                fn_00707e10(e, button, characters)
            );
            tile_set_int(e, button, 0xfac, count);
            tile_set_int(e, button, 0xfaa, base_index.wrapping_add(count));
            e.call(TILE_UPDATE_ALL, &args![0u32]);
            let size = tile_float(e, button, TRAIT_0FB1);
            total = total.wrapping_add(float_to_int(e, size));
            count += 1;
            e.call(STRING_ASSIGN, &args![string, name_at(count as usize)]);
        }
        // The exe's `FISUB` and `FIDIV` (the count plus one as a signed word), each a 24-bit
        // step, then `_ftol2`.
        let line_size = tile_float(e, tile, TRAIT_0FB1);
        let difference = super::narrow(f64::from(line_size) - f64::from(total));
        let divisor = count.wrapping_add(1) as i32;
        let share = super::narrow(f64::from(difference) / f64::from(divisor));
        let left_line_length = double_to_int(e, f64::from(share));
        let trait_id = e
            .call(TILE_TEXT_TO_TRAIT, &args![LEFT_LINE_LENGTH_NAME])
            .u32();
        tile_set_int(e, tile, trait_id, super::bits(left_line_length));
        let trait_id = e.call(TILE_TEXT_TO_TRAIT, &args![BUTTON_COUNT_NAME]).u32();
        tile_set_int(e, tile, trait_id, count);

        e.call(STRING_SET, &args![string, first_name, 0u32]);
        let mut position = left_line_length;
        let mut index = 0usize;
        while e.call(STRING_LENGTH, &args![string]).u32() != 0 {
            let characters = e.mem.u32(string);
            // `Tile::GetChildByName` (`00a03da0`, the dispatch family's).
            let child = typed_call!(
                dispatch::FAMILY,
                0x00a0_3da0,
                dispatch::tile::child_by_name(e, Ptr::new(tile), Ptr::new(characters))
            )
            .addr();
            let trait_id = e.call(TILE_TEXT_TO_TRAIT, &args![X_NAME]).u32();
            tile_set_int(e, child, trait_id, super::bits(position));
            let size = tile_float(e, child, TRAIT_0FB1);
            position = float_to_int(e, size)
                .wrapping_add(left_line_length)
                .wrapping_add(position);
            index += 1;
            e.call(STRING_ASSIGN, &args![string, name_at(index)]);
        }
        e.call(STRING_DESTROY, &args![string]);
    });
}

// Typed from 00707be0
/// The registered form of `00707be0`: the tile, the base index, the first
/// name, then the further names.
pub fn interface_initialize_tabline(e: &mut Engine, words: &[u32]) -> Ret {
    assert!(
        words.len() >= 3,
        "00707be0 needs the tile, the base index and the first name"
    );
    initialize_tabline(e, words[0], words[1], words[2], &words[3..]);
    Ret::default()
}

// Typed from 00707e10
/// Assigns `name` to the string at `+0x20` of `this` (a tile's name):
/// `00438390`.
pub fn fn_00707e10(e: &mut Engine, this: u32, name: u32) {
    e.call(STRING_ASSIGN, &args![this + 0x20, name]);
}

// Typed from 00707e30
/// `Interface::PopulateItemStatsDisplay` (Xbox PDB), `cdecl`: fills the item
/// stats display tile `display` for the inventory entry `item` (an
/// `ItemChange`, Xbox PDB name of the `004bd400` family: `+4` its count, `+8`
/// its form). Does nothing for a null `display` (a null `item` is
/// dereferenced before that check).
///
/// The first 13 children of `display` (list at `+4`) are the cells,
/// `cells[0..13]`; `flags` collects which of them are shown (see
/// `VISIBILITY_BITS`). The form's type byte (`00401170`) selects what is
/// shown:
///
/// * `0x18` and `0x1a`: cell 4 gets trait `0x100a` = 0; the condition
///   `GetItemHealth(1) / 100` and the two armor values (`004be060`,
///   `004be180`, truncated) go through `CombatFormulas::CalcArmorRating`
///   and are shown in cells 0 and 12 (or "--" in cell 0 when both are 0);
///   the title of cell 5 is the string of one of three game settings chosen
///   by two flag bytes of the cast object's sub-object at `+0x70`; cell
///   4's trait `0x1009` gets `GetItemHealth(0)` over the form's health
///   (`TESHealthForm::GetFormHealth`, or 1), at most 1. `flags` = `0x3c`,
///   plus `1` and `0x1000` for the cells 0 and 12.
/// * `0x28` (weapon): cell 4 gets `0x100a` = 1; the damage
///   (`00645380`, cell 1), the damage per shot
///   (`CalcWeaponDamageForDisplay`, cell 11, as "%.1fx%d" when the weapon
///   fires several projectiles), the weapon's word at `+0x19c` less the
///   truncated entry point `0x35` (cell 10), the ammunition text with its
///   loaded and spare counts (cell 5) and the weight, with entry point
///   `0x49` for weapons of at least 10 (the constant at `01020758`).
///   `flags` = `0xc3e`.
/// * `0x29` (ammo): `TESAmmo::BuildMenuString` fills cell 6 when it is not
///   empty; `flags` = `0xc` (plus `0x40` when cell 6 is filled).
/// * anything else: `flags` = `0xc`.
///
/// Then the effect list of the form (types `0x1d`, `0x2f`, or an
/// enchantment the form has) gives the text of cell 6 (`0x40`), a form of
/// type `0x67` its description, the mod slots fill the cells 7..9 (`0x80`,
/// `0x100`, `0x200`) and move the heights of the cells below, and the value
/// (cell 3) and weight (cell 2) texts are set. When the language setting is
/// not "ENGLISH", nine cells get their title from game settings. At the end
/// every cell's trait `0xfa3` is set from `flags` and the heights (trait
/// `0xfa2`) of cells 6..9 from the display's trait `0xfb0`.
///
/// The compiler's stack-cookie check is not translated.
pub fn interface_populate_item_stats_display(e: &mut Engine, item: u32, display: u32) {
    e.with_stack(STATS_FRAME_SIZE, |e, frame| {
        populate_item_stats(e, frame.addr(), item, display);
    });
}

/// `ItemChange`'s form (`0044ddc0`: the word at `+8`).
#[inline(always)]
fn item_form(e: &Engine, item: u32) -> u32 {
    e.mem.u32(item.wrapping_add(8))
}

/// The form's type byte (`00401170`: its `+4`).
#[inline(always)]
fn form_type(e: &Engine, form: u32) -> u32 {
    u32::from(e.mem.u8(form.wrapping_add(4)))
}

/// A game setting's string (`00403df0` on the setting object).
#[inline(always)]
fn setting_text(e: &Engine, setting: u32) -> u32 {
    in_place::setting_text(e, setting)
}

/// `MiddleHighProcess::GetSavedAcquireObject` (Xbox PDB) on the player: its word at `+0x68`
/// (`008d8520`).
#[inline(always)]
fn saved_acquire_object(e: &Engine) -> u32 {
    let player = e.mem.u32(PLAYER);
    e.mem.u32(player.wrapping_add(0x68))
}

/// A form name's characters (`00408da0` on the name at `+0x30`: its first word, or the empty text
/// at `01011584` when that is null).
#[inline(always)]
pub(super) fn name_characters(e: &Engine, name: u32) -> u32 {
    match e.mem.u32(name.wrapping_add(4)) {
        0 => 0x0101_1584,
        characters => characters,
    }
}

/// The float setting's value (`00403e20` on a constant setting object: its `+4`).
#[inline(always)]
fn float_setting(e: &Engine, setting: u32) -> f32 {
    e.mem.f32(setting.wrapping_add(4))
}

/// `00406d00(dest, size, format, word)`.
fn format_word(e: &mut Engine, dest: u32, size: u32, format: u32, word: u32) {
    e.call(FORMAT_TEXT, &args![dest, size, format, word]);
}

/// `FISTP` to a 64-bit integer with truncation, low 32 bits (what
/// `FISTP qword` leaves when read as a dword): NaN and out-of-range values
/// give the "integer indefinite" `0x8000000000000000`.
fn truncate_low_dword(value: f32) -> i32 {
    super::low_dword_signed(engine_core::x87::fistp_chop64(f64::from(value)))
}

/// The body of `00707e30` with the scratch `frame` the game keeps on its
/// stack.
fn populate_item_stats(e: &mut Engine, frame: u32, item: u32, display: u32) {
    let entry_out = frame + FRAME_ENTRY_OUT;
    let armor_text = frame + FRAME_ARMOR_TEXT;
    let weapon_text = frame + FRAME_WEAPON_TEXT;
    let ammo_text = frame + FRAME_AMMO_TEXT;
    let effect_text = frame + FRAME_EFFECT_TEXT;
    let mod_text = frame + FRAME_MOD_TEXT;
    let value_text = frame + FRAME_VALUE_TEXT;
    let weight_text = frame + FRAME_WEIGHT_TEXT;

    let mut flags: u32;
    let title_trait = e.call(TILE_TEXT_TO_TRAIT, &args![TITLE_NAME]).u32();
    let value_trait = e.call(TILE_TEXT_TO_TRAIT, &args![VALUE_NAME]).u32();
    let meter_trait: u32 = 0x1009;
    let mut height_a: f32 = 0.0;
    let mut height_b: f32 = 0.0;
    let item_value = e.call(ITEM_GET_VALUE, &args![item]).f32();
    let player = e.global::<u32>(PLAYER);
    // `004d1360`: the player's word at `+0x7bc` is 1.
    let weight_flag = u8::from(e.mem.u32(player.wrapping_add(0x7bc)) == 1);
    let form = item_form(e, item);
    let mut weight = e.call(GET_FORM_WEIGHT, &args![form, weight_flag]).f32();
    if item == 0 || display == 0 {
        return;
    }

    // The cells: the first 13 children of the display tile.
    let list = display + 4;
    let first_node = e.mem.u32(list.wrapping_add(4));
    let mut cells = [0u32; 13];
    let mut filled = 0usize;
    // The walk's step (`00683520`): the item is the node's `+8`, the next its `+4`.
    let mut node = first_node;
    while node != 0 && filled < 13 {
        let slot = node.wrapping_add(8);
        node = e.mem.u32(node.wrapping_add(4));
        cells[filled] = e.mem.u32(slot);
        filled += 1;
    }

    // The tiles the paths name (`00a08b20`, the dispatch family's).
    let arrows = find_tile(e, display, CONDITION_ARROWS_NAME);
    let meter = find_tile(e, display, CONDITION_METER_NAME);
    let value = tile_float(e, meter, TRAIT_0FB0);
    tile_set_float(e, arrows, TRAIT_0FB0, value);
    let value = tile_float(e, meter, TRAIT_0FA2);
    tile_set_float(e, arrows, TRAIT_0FA2, value);
    let has_mods = e.call(ITEM_GET_MOD_SLOTS, &args![item]).u8() != 0;

    let form = item_form(e, item);
    let kind = form_type(e, form);
    match kind {
        0x18 | 0x1a => {
            tile_set_int(e, cells[4], 0x100a, 0);
            let condition = super::narrow(
                e.call(ITEM_GET_HEALTH, &args![item, 1u32]).f32() as f64
                    / e.global::<f64>(PERCENT_DIVISOR),
            );
            flags = 0x3c;
            copy_text(e, armor_text, 0x80, DASHES);
            let form = item_form(e, item);
            let cast = e
                .call(
                    DYNAMIC_CAST,
                    &args![form, 0u32, ARMOR_CAST_SOURCE, ARMOR_CAST_TARGET, 0u32],
                )
                .u32();
            if cast != 0 {
                let sub_object = cast + 0x70;
                let setting_pointer = if e.mem.u8(sub_object.wrapping_add(8)) & 0x80 != 0 {
                    ARMOR_SETTING_A
                } else if e.mem.u8(sub_object.wrapping_add(8)) & 0x08 != 0 {
                    ARMOR_SETTING_B
                } else {
                    ARMOR_SETTING_C
                };
                let setting = e.global::<u32>(setting_pointer);
                let text = setting_text(e, setting);
                copy_text(e, armor_text, 0x80, text);
            }
            tile_set_text(e, cells[5], title_trait, armor_text);

            let form = item_form(e, item);
            let input = e.call(ARMOR_RATING_INPUT_A, &args![form]).f32();
            let input = float_to_int(e, input);
            let rating_a = e.call(CALC_ARMOR_RATING, &args![input, condition]).f32();
            let form = item_form(e, item);
            let input = e.mem.f32(form.wrapping_add(0x17c));
            let input = float_to_int(e, input);
            let rating_b = e.call(CALC_ARMOR_RATING, &args![input, condition]).f32();
            if rating_a == 0.0 && rating_b == 0.0 {
                flags |= 1;
                copy_text(e, armor_text, 0x80, DASHES);
                tile_set_text(e, cells[0], value_trait, armor_text);
            } else {
                if rating_a != 0.0 {
                    let rounded = e.call(ROUND_FLOAT, &args![rating_a]).f32();
                    let rounded = float_to_int(e, rounded);
                    format_word(e, armor_text, 0x80, FORMAT_INTEGER, super::bits(rounded));
                    tile_set_text(e, cells[0], value_trait, armor_text);
                    flags |= 1;
                }
                if rating_b != 0.0 {
                    let rounded = e.call(ROUND_FLOAT, &args![rating_b]).f32();
                    let rounded = float_to_int(e, rounded);
                    format_word(e, armor_text, 0x80, FORMAT_INTEGER, super::bits(rounded));
                    tile_set_text(e, cells[12], value_trait, armor_text);
                    flags |= 0x1000;
                }
            }

            let form = item_form(e, item);
            let health = e.call(GET_FORM_HEALTH, &args![form]).u32();
            let health = if health != 0 {
                let form = item_form(e, item);
                e.call(GET_FORM_HEALTH, &args![form]).u32()
            } else {
                1
            };
            let current = e.call(ITEM_GET_HEALTH, &args![item, 0u32]).f32();
            let fraction = super::narrow(current as f64 / health as f64);
            let level = smaller_float(1.0f32, fraction);
            tile_set_float(e, cells[4], meter_trait, level);
        }
        0x28 => {
            let weapon = item_form(e, item);
            tile_set_int(e, cells[4], 0x100a, 1);
            let mut saved_ammo = 0u32;
            let mut saved_process = 0u32;
            if saved_acquire_object(e) != 0 {
                let held = saved_acquire_object(e);
                if e.vcall(held, 0x148, &[]).u32() != 0 {
                    let held = saved_acquire_object(e);
                    let held_item = e.vcall(held, 0x148, &[]).u32();
                    let held_form = item_form(e, held_item);
                    if weapon == held_form {
                        let held = saved_acquire_object(e);
                        if e.vcall(held, 0x14c, &[]).u32() != 0 {
                            let held = saved_acquire_object(e);
                            let ammo_item = e.vcall(held, 0x14c, &[]).u32();
                            saved_ammo = item_form(e, ammo_item);
                        } else {
                            saved_ammo = 0;
                        }
                        saved_process = e.global::<u32>(PLAYER);
                    }
                }
            }
            let with_effect = e.call(ITEM_HAS_MOD_EFFECT, &args![item, 4u32]).u8();
            weight = e
                .call(WEAPON_GET_WEIGHT, &args![weapon, with_effect as u32])
                .f32();
            if weight as f64 >= e.global::<f64>(HEAVY_WEIGHT) {
                e.mem.set_f32(entry_out, 1.0);
                let player = e.global::<u32>(PLAYER);
                e.call(HANDLE_ENTRY_POINT, &args![0x49u32, player, entry_out]);
                weight = super::narrow(weight as f64 * e.mem.f32(entry_out) as f64);
            }
            let condition = super::narrow(
                e.call(ITEM_GET_HEALTH, &args![item, 1u32]).f32() as f64
                    / e.global::<f64>(PERCENT_DIVISOR),
            );
            let mut field_19c = e.mem.i32(weapon.wrapping_add(0x19c));
            let player = e.global::<u32>(PLAYER);
            let actor_values = if player != 0 { player + 0xa4 } else { 0 };
            let damage = e
                .call(
                    CALC_WEAPON_DAMAGE,
                    &args![
                        actor_values,
                        weapon,
                        condition,
                        1u32,
                        item,
                        0u32,
                        0u32,
                        0xffff_ffffu32,
                        0.0f32,
                        0.0f32,
                        0u32,
                        0u32,
                        saved_ammo
                    ],
                )
                .f32();
            let damage_for_display = if condition > 0.0 || condition.is_nan() {
                e.call(
                    CALC_WEAPON_DAMAGE_FOR_DISPLAY,
                    &args![weapon, condition, item, saved_ammo],
                )
                .f32()
            } else {
                0.0
            };
            let rounded = e.call(ROUND_DIVIDE, &args![damage, 1.0f32]).f32();
            let rounded = float_to_int(e, rounded);
            format_word(e, weapon_text, 0x80, FORMAT_INTEGER, super::bits(rounded));
            tile_set_text(e, cells[1], value_trait, weapon_text);

            let with_effect = e.call(ITEM_HAS_MOD_EFFECT, &args![item, 0xcu32]).u8();
            let projectiles = e
                .call(
                    WEAPON_GET_NUM_PROJECTILES,
                    &args![weapon, with_effect as u32, 0u32, saved_process],
                )
                .u8() as u32;
            if projectiles > 1 {
                let each = super::narrow(damage_for_display as f64 / projectiles as f64);
                e.call(
                    FORMAT_TEXT,
                    &args![
                        weapon_text,
                        0x80u32,
                        FORMAT_PROJECTILES,
                        each as f64,
                        projectiles
                    ],
                );
            } else {
                let rounded = e
                    .call(ROUND_DIVIDE, &args![damage_for_display, 1.0f32])
                    .f32();
                let rounded = float_to_int(e, rounded);
                format_word(e, weapon_text, 0x80, FORMAT_INTEGER, super::bits(rounded));
            }
            tile_set_text(e, cells[11], value_trait, weapon_text);

            let level = smaller_float(condition, 1.0f32);
            tile_set_float(e, cells[4], meter_trait, level);
            e.mem.set_f32(entry_out, 0.0);
            let player = e.global::<u32>(PLAYER);
            e.call(
                HANDLE_ENTRY_POINT,
                &args![0x35u32, player, weapon, entry_out],
            );
            let reduction = truncate_low_dword(e.mem.f32(entry_out));
            field_19c = field_19c.wrapping_sub(reduction);
            if field_19c < 0 {
                field_19c = 0;
            }
            format_word(e, weapon_text, 0x80, FORMAT_INTEGER, field_19c as u32);
            if cells[10] != 0 {
                tile_set_text(e, cells[10], value_trait, weapon_text);
            }

            let ammo = e
                .call(WEAPON_GET_CURRENT_AMMO, &args![weapon, saved_process])
                .u32();
            if ammo != 0 {
                let held = saved_acquire_object(e);
                let held_item = e.vcall(held, 0x148, &[]).u32();
                let held = saved_acquire_object(e);
                let held_ammo_item = e.vcall(held, 0x14c, &[]).u32();
                let ammo = e
                    .call(WEAPON_GET_CURRENT_AMMO, &args![weapon, saved_process])
                    .u32();
                let mut name = e.mem.u32(ammo.wrapping_add(0xc4));
                let ammo = e
                    .call(WEAPON_GET_CURRENT_AMMO, &args![weapon, saved_process])
                    .u32();
                let player = e.global::<u32>(PLAYER);
                let changes = e.call(GET_INVENTORY_CHANGES, &args![player]).u32();
                // `InventoryChanges::GetObjectCount` (`004c8f30`, the dispatch family's).
                let total = super::bits(typed_call!(
                    dispatch::FAMILY,
                    0x004c_8f30,
                    dispatch::refs::item_count(e, Ptr::new(changes), ammo)
                ));
                let mut loaded: u32 = 0;
                if name == 0 || e.mem.u8(name) == 0 {
                    let ammo = e
                        .call(WEAPON_GET_CURRENT_AMMO, &args![weapon, saved_process])
                        .u32();
                    name = name_characters(e, ammo.wrapping_add(0x30));
                }
                let mut loaded_known = false;
                if held_item != 0 && e.mem.u32(held_item) != 0 && e.mem.u32(item) != 0 {
                    let held_first = e.mem.u32(held_item);
                    let held_slot = held_first;
                    let item_first = e.mem.u32(item);
                    let item_slot = item_first;
                    if e.mem.u32(held_slot) == e.mem.u32(item_slot) {
                        loaded = if held_ammo_item != 0 {
                            e.mem.u32(held_ammo_item.wrapping_add(4))
                        } else {
                            0
                        };
                        loaded_known = true;
                    }
                }
                if !loaded_known {
                    let with_effect = e.call(ITEM_HAS_MOD_EFFECT, &args![item, 2u32]).u8();
                    let rounds = e
                        .call(WEAPON_GET_CLIP_ROUNDS, &args![weapon, with_effect as u32])
                        .u32();
                    loaded = smaller(rounds as i32, total as i32) as u32;
                }
                let regeneration = e
                    .call(WEAPON_GET_AMMO_REGEN_RATE, &args![weapon, 0u32])
                    .f32();
                if regeneration != 0.0 {
                    format_word(e, weapon_text, 0x80, FORMAT_STRING, name);
                } else {
                    e.call(
                        FORMAT_TEXT,
                        &args![
                            weapon_text,
                            0x80u32,
                            FORMAT_AMMO,
                            name,
                            loaded,
                            total.wrapping_sub(loaded)
                        ],
                    );
                }
            } else {
                copy_text(e, weapon_text, 0x80, DASHES);
            }
            tile_set_text(e, cells[5], title_trait, weapon_text);
            flags = 0xc3e;
        }
        0x29 => {
            flags = 0xc;
            let ammo = item_form(e, item);
            e.call(AMMO_BUILD_MENU_STRING, &args![ammo, ammo_text, 0x100u32]);
            if e.call(STRING_LENGTH_C, &args![ammo_text]).u32() > 0 {
                tile_set_text(e, cells[6], value_trait, ammo_text);
                flags |= 0x40;
            } else {
                flags &= !0x40;
            }
        }
        _ => flags = 0xc,
    }

    // The effect list whose text goes to cell 6.
    let mut effects: u32 = 0;
    let form = item_form(e, item);
    if form_type(e, form) == 0x1d {
        let ownership = e.call(ITEM_GET_OWNERSHIP, &args![item]).u32();
        effects = if ownership != 0 { ownership + 0x3c } else { 0 };
    } else {
        let form = item_form(e, item);
        if form_type(e, form) == 0x2f {
            let form = item_form(e, item);
            effects = if form != 0 { form + 0x3c } else { 0 };
        } else {
            let form = item_form(e, item);
            let enchanting = e.call(GET_FORM_ENCHANTING, &args![form]).u32();
            if enchanting != 0 && e.mem.u8(enchanting.wrapping_add(0x40)) & 4 == 0 {
                effects = enchanting + 0x24;
            }
        }
    }
    let has_effects = effects != 0 && {
        let list = effects.wrapping_add(4);
        e.mem.u32(list) != 0
    };
    if has_effects {
        e.call(
            EFFECT_LIST_BUILD_MENU_STRING,
            &args![effects, effect_text, 0x100u32],
        );
        if e.call(STRING_LENGTH_C, &args![effect_text]).u32() > 0 {
            tile_set_text(e, cells[6], value_trait, effect_text);
            flags |= 0x40;
        } else {
            flags &= !0x40;
        }
    } else {
        let form = item_form(e, item);
        if form_type(e, form) != 0x29 {
            flags &= !0x40;
        }
    }

    let form = item_form(e, item);
    if form_type(e, form) == 0x67 {
        let form = item_form(e, item);
        let description = e
            .vcall(form + 0x74, 0x10, &args![0u32, DESCRIPTION_TAG])
            .u32();
        tile_set_text(e, cells[6], value_trait, description);
        flags |= 0x40;
    }

    // The mod slots.
    if !has_mods {
        flags &= !0x80;
    } else {
        let mod_form = item_form(e, item);
        let slots = e.call(ITEM_GET_MOD_SLOTS, &args![item]).u8();
        let mut titled = false;
        if slots & 1 != 0 {
            flags |= 0x80;
            let text = setting_text(e, SETTING_MODS_TITLE);
            tile_set_text(e, cells[7], title_trait, text);
            titled = true;
            height_a = super::narrow(height_a as f64 + e.global::<f64>(MOD_ROW_HEIGHT));
            height_b = super::narrow(height_b as f64 + e.global::<f64>(MOD_ROW_HEIGHT));
            let mod_item = e.call(ITEM_FORM_GET_MOD, &args![mod_form, 1u32]).u32();
            if mod_item != 0 {
                let name = name_characters(e, mod_item.wrapping_add(0x30));
                copy_text(e, mod_text, 0x80, name);
            } else {
                copy_text(e, mod_text, 0x80, MOD_1_TEXT);
            }
            tile_set_text(e, cells[7], value_trait, mod_text);
        }
        if slots & 2 != 0 {
            flags |= 0x100;
            let mod_item = e.call(ITEM_FORM_GET_MOD, &args![mod_form, 2u32]).u32();
            if mod_item != 0 {
                let name = name_characters(e, mod_item.wrapping_add(0x30));
                copy_text(e, mod_text, 0x80, name);
            } else {
                copy_text(e, mod_text, 0x10, MOD_2_TEXT);
            }
            tile_set_text(e, cells[8], value_trait, mod_text);
            if !titled {
                let text = setting_text(e, SETTING_MODS_TITLE);
                tile_set_text(e, cells[8], title_trait, text);
                titled = true;
            }
            height_b = super::narrow(height_b as f64 + e.global::<f64>(MOD_ROW_HEIGHT));
        }
        if slots & 4 != 0 {
            let mod_item = e.call(ITEM_FORM_GET_MOD, &args![mod_form, 4u32]).u32();
            if mod_item != 0 {
                let name = name_characters(e, mod_item.wrapping_add(0x30));
                copy_text(e, mod_text, 0x80, name);
            } else {
                copy_text(e, mod_text, 0x10, MOD_3_TEXT);
            }
            tile_set_text(e, cells[9], value_trait, mod_text);
            flags |= 0x200;
            if !titled {
                let text = setting_text(e, SETTING_MODS_TITLE);
                tile_set_text(e, cells[9], title_trait, text);
            }
        }
        flags &= !0x40;
    }

    // The value text.
    if item_value <= 0.0 || item_value.is_nan() {
        copy_text(e, value_text, 0x10, DASHES);
    } else {
        let count = e.mem.u32(item.wrapping_add(4)) as i32;
        // 708f6b..708f77, 708f9d..708fa9: FILD, FMUL float (rounded with the running thread's
        // control word), FSTP double.
        let product = x87_mul(count as f64, x87_m32(item_value.to_bits()));
        let format = if (item_value as f64) < e.global::<f64>(ONE) {
            FORMAT_ONE_DECIMAL
        } else {
            FORMAT_NO_DECIMAL
        };
        e.call(FORMAT_TEXT, &args![value_text, 0x10u32, format, product]);
    }
    tile_set_text(e, cells[3], value_trait, value_text);

    // The weight text.
    e.mem.set_f32(entry_out, 0.0);
    let player = e.global::<u32>(PLAYER);
    e.call(HANDLE_ENTRY_POINT, &args![0x37u32, player, entry_out]);
    if e.mem.f32(entry_out) > 0.0 && weight <= float_setting(e, WEIGHT_LIMIT_SETTING) {
        weight =
            super::narrow(f64::from(weight) * f64::from(float_setting(e, WEIGHT_FACTOR_SETTING)));
    }
    if weight <= 0.0 || weight.is_nan() {
        copy_text(e, weight_text, 0x20, DASHES);
    } else {
        let count = e.mem.u32(item.wrapping_add(4)) as i32;
        // 709061..70906d: as for the value.
        let product = x87_mul(count as f64, x87_m32(weight.to_bits()));
        e.call(
            FORMAT_TEXT,
            &args![weight_text, 0x20u32, FLOAT_FORMAT, product],
        );
    }
    tile_set_text(e, cells[2], value_trait, weight_text);

    // Titles for languages other than English.
    let language = setting_text(e, LANGUAGE_SETTING);
    if e.call(COMPARE_TEXT, &args![language, ENGLISH_TEXT]).i32() != 0 {
        for (setting, cell) in LANGUAGE_TITLES {
            let text = setting_text(e, setting);
            tile_set_text(e, cells[cell], title_trait, text);
        }
    }

    // Which cells are shown.
    for (bit, cell) in VISIBILITY_BITS {
        tile_set_int(e, cells[cell], TRAIT_0FA3, (flags & bit != 0) as u32);
    }
    if cells[10] != 0 {
        tile_set_int(e, cells[10], TRAIT_0FA3, (flags & 0x400 != 0) as u32);
        let constant = e.global::<f32>(CELL_10_TRAIT_FA9);
        tile_set_float(e, cells[10], TRAIT_0FA9, constant);
    }

    // The heights of the cells below.
    let extent = tile_float(e, display, TRAIT_0FB0);
    let mut base = super::narrow((extent as f64 / e.global::<f64>(TWO)) - e.global::<f64>(TWENTY));
    if flags & 0x10 != 0 || flags & 0x20 != 0 {
        base += base;
    }
    tile_set_float(e, cells[6], TRAIT_0FA2, base);
    tile_set_float(e, cells[7], TRAIT_0FA2, base);
    tile_set_float(
        e,
        cells[8],
        TRAIT_0FA2,
        super::narrow(base as f64 + height_a as f64),
    );
    tile_set_float(
        e,
        cells[9],
        TRAIT_0FA2,
        super::narrow(base as f64 + height_b as f64),
    );
}

// Typed from 00709470
/// `Interface::QueueMenuCreate` (Xbox PDB), `cdecl`: remembers the menu to
/// open (kind and four words). Kind 7 (the message menu) goes to its own
/// slot (`011d8998`, words `011d899c`, `011d89a0`, `011d89a4`, `011d89ac`);
/// any other kind to the first (`011d8950`, `011d8954`, `011d8958`,
/// `011d895c`, `011d8964`). When `008c7aa0` says nothing blocks, the queue
/// is handled at once ([`interface_handle_queued_menu_open`]) and then both
/// slots are cleared of their kind and first word ([`fn_00709a90`]).
pub fn interface_queue_menu_create(
    e: &mut Engine,
    kind: u32,
    first: u32,
    second: u32,
    third: u32,
    fourth: u32,
) {
    if kind == 7 {
        e.mem.set_u32(MESSAGE_QUEUE_KIND, kind);
        e.mem.set_u32(MESSAGE_QUEUE_FIRST, first);
        e.mem.set_u32(MESSAGE_QUEUE_SECOND, second);
        e.mem.set_u32(MESSAGE_QUEUE_THIRD, third);
        e.mem.set_u32(MESSAGE_QUEUE_FOURTH, fourth);
    } else {
        e.mem.set_u32(QUEUE_KIND, kind);
        e.mem.set_u32(QUEUE_FIRST, first);
        e.mem.set_u32(QUEUE_SECOND, second);
        e.mem.set_u32(QUEUE_THIRD, third);
        e.mem.set_u32(QUEUE_FOURTH, fourth);
    }
    if !e.call(QUEUE_MENU_BLOCKED, &[]).bool() {
        typed_call!(
            super::FAMILY,
            0x0070_94f0,
            interface_handle_queued_menu_open(e)
        );
        typed_call!(super::FAMILY, 0x0070_9a90, fn_00709a90(e));
    }
}

// Typed from 007094f0
/// `Interface::HandleQueuedMenuOpen` (Xbox PDB): with the queue lock
/// (`011d89c0`) held, opens the queued menus when the manager's mode word
/// (`+0x4bc`) is 0 or the kind is 6 or 7, and clears a slot once its menu
/// was created.
///
/// First slot, by kind: 1 container menu (`00704ed0(first, fourth)`), 2
/// hacking menu, 3 computers menu, 4 dialog menu (`00705070(first, second,
/// third)`, only when `007023a0` on the manager is clear), 5 race/sex menu
/// (`011d8960`), 6 start menu (`007cb7d0(1, 0)`, then `HUDMainMenu::
/// SetMenuMode(4)` when it was created), 7 message menu, 8 companion wheel
/// menu (the first word cast `0118_41cc -> 0118_46d4`), 9 a player name
/// entry menu built here: a `TileMenu` (`0x40` bytes, [`fn_00709810`]) put
/// under the menus root, named "Player Name Entry Menu", given to a new menu
/// object (`007ab690`, `0x2c` bytes) through `Menu::SetMenuTile`. A created
/// menu clears the slot's kind and first word. The second slot only holds
/// kind 7 (the message menu) and is cleared the same way.
///
/// The compiler's exception-unwinding frame (around the two `operator new`
/// calls) is not translated.
pub fn interface_handle_queued_menu_open(e: &mut Engine) {
    use super::interface_cpp as unit;
    use dispatch::layout::LightLock;
    typed_call!(
        dispatch::FAMILY,
        0x0040_fbf0,
        dispatch::refs::lock(e, Ptr::<LightLock>::new(QUEUE_LOCK), 0)
    );
    let kind = e.mem.u32(QUEUE_KIND);
    if kind != 0 {
        let current = manager(e);
        if e.mem.u32(current.wrapping_add(0x4bc)) == 0 || kind == 6 || kind == 7 {
            let mut created = false;
            match kind {
                1 => {
                    let first = e.mem.u32(QUEUE_FIRST);
                    let fourth = e.mem.u32(QUEUE_FOURTH);
                    created = typed_call!(
                        super::FAMILY,
                        0x0070_4ed0,
                        unit::interface_create_container_menu(e, first, fourth)
                    ) != 0;
                }
                2 => {
                    let first = e.mem.u32(QUEUE_FIRST);
                    created = typed_call!(
                        super::FAMILY,
                        0x0070_5ec0,
                        interface_create_hacking_menu(e, first)
                    );
                }
                3 => {
                    let first = e.mem.u32(QUEUE_FIRST);
                    created = typed_call!(
                        super::FAMILY,
                        0x0070_6130,
                        interface_create_computers_menu(e, first)
                    );
                }
                4 => {
                    let current = manager(e);
                    if !typed_call!(super::FAMILY, 0x0070_23a0, unit::fn_007023a0(e, current)) {
                        let first = e.mem.u32(QUEUE_FIRST);
                        let second = e.mem.u32(QUEUE_SECOND);
                        let third = e.mem.u32(QUEUE_THIRD);
                        created = typed_call!(
                            super::FAMILY,
                            0x0070_5070,
                            unit::interface_create_dialog_menu(e, first, second, third)
                        ) != 0;
                    }
                }
                5 => {
                    let race_sex = e.mem.u32(QUEUE_RACE_SEX);
                    created = e.call(RACE_SEX_MENU_CREATE, &args![race_sex]).u32() != 0;
                }
                6 => {
                    created = e.call(START_MENU_CREATE, &args![1u32, 0u32]).bool();
                    if created {
                        e.call(HUD_SET_MENU_MODE, &args![4u32]);
                    }
                }
                7 => created = e.call(MESSAGE_MENU_CREATE, &[]).bool(),
                8 => {
                    let first = e.mem.u32(QUEUE_FIRST);
                    let wheel = e
                        .call(
                            DYNAMIC_CAST,
                            &args![
                                first,
                                0u32,
                                COMPANION_CAST_SOURCE,
                                COMPANION_CAST_TARGET,
                                0u32
                            ],
                        )
                        .u32();
                    created = e.call(COMPANION_WHEEL_MENU_CREATE, &args![wheel]).bool();
                }
                9 => {
                    let block = e.call(OPERATOR_NEW, &args![0x40u32]).u32();
                    let menu_tile = if block != 0 {
                        typed_call!(super::FAMILY, 0x0070_9810, fn_00709810(e, block, 0, 0))
                    } else {
                        0
                    };
                    let block = e.call(OPERATOR_NEW, &args![0x2cu32]).u32();
                    let menu = if block != 0 {
                        e.call(NAME_ENTRY_MENU_CONSTRUCT, &args![block]).u32()
                    } else {
                        0
                    };
                    let current = manager(e);
                    let root = in_place::menus_root(e, current);
                    e.call(TILE_SET_PARENT, &args![menu_tile, root, 0u32]);
                    typed_call!(
                        super::FAMILY,
                        0x0070_7e10,
                        fn_00707e10(e, menu_tile, NAME_ENTRY_MENU_NAME)
                    );
                    e.call(MENU_SET_MENU_TILE, &args![menu, menu_tile, 1u32]);
                    e.call(NAME_ENTRY_MENU_FINISH, &args![menu_tile, menu]);
                    created = true;
                }
                _ => {}
            }
            if created {
                e.mem.set_u32(QUEUE_KIND, 0);
                e.mem.set_u32(QUEUE_FIRST, 0);
            }
        }
    }
    let message_kind = e.mem.u32(MESSAGE_QUEUE_KIND);
    if message_kind != 0 {
        let current = manager(e);
        if e.mem.u32(current.wrapping_add(0x4bc)) == 0 || message_kind == 6 || message_kind == 7 {
            let mut created = false;
            if message_kind == 7 {
                created = e.call(MESSAGE_MENU_CREATE, &[]).bool();
            }
            if created {
                e.mem.set_u32(MESSAGE_QUEUE_KIND, 0);
                e.mem.set_u32(MESSAGE_QUEUE_FIRST, 0);
            }
        }
    }
    typed_call!(
        dispatch::FAMILY,
        0x0040_fba0,
        dispatch::refs::unlock(e, Ptr::<LightLock>::new(QUEUE_LOCK))
    );
}

// Typed from 00709810
/// Constructor of `TileMenu` (vtable `0106ed44`, RTTI `.?AVTileMenu@@`),
/// `RET 8`: the `TileRect` constructor ([`fn_00709840`]), the vtable, and
/// the word at `+0x3c` cleared. Neither parameter is read. Returns `this`.
pub fn fn_00709810(e: &mut Engine, this: u32, _unused_1: u32, _unused_2: u32) -> u32 {
    typed_call!(super::FAMILY, 0x0070_9840, fn_00709840(e, this));
    e.mem.set_u32(this, TILE_MENU_VTABLE);
    e.mem.set_u32(this + 0x3c, 0);
    this
}

// Typed from 00709840
/// Constructor of `TileRect` (vtable `0106ed70`, RTTI `.?AVTileRect@@`): the
/// `Tile` constructor ([`fn_00709860`]) and the vtable. Returns `this`.
pub fn fn_00709840(e: &mut Engine, this: u32) -> u32 {
    typed_call!(super::FAMILY, 0x0070_9860, fn_00709860(e, this));
    e.mem.set_u32(this, TILE_RECT_VTABLE);
    this
}

// Typed from 00709860
/// Constructor of `Tile` (vtable `0106ed9c`, RTTI `.?AVTile@@`): constructs
/// the members at `+4` (`0048f200`), `+0x10` (`00709e80(0x10, 0)`), `+0x20`
/// (a `BSStringT`) and `+0x2c` (`00633c90(0)`, in place: the null stored;
/// then `0066b0d0(0)`), and
/// clears the words at `+0x28` and `+0x30` and the byte at `+0x34`.
/// Returns `this`. The compiler's exception-unwinding frame is not
/// translated.
pub fn fn_00709860(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, TILE_VTABLE);
    e.call(TILE_MEMBER_AT_4, &args![this.wrapping_add(4)]);
    typed_call!(
        super::FAMILY,
        0x0070_9e80,
        super::interface_cpp_p3::fn_00709e80(e, this.wrapping_add(0x10), 0x10, 0)
    );
    in_place::empty_string(e, this.wrapping_add(0x20));
    e.mem.set_u32(this + 0x2c, 0);
    e.call(TILE_MEMBER_AT_2C_SECOND, &args![this + 0x2c, 0u32]);
    e.mem.set_u32(this + 0x28, 0);
    e.mem.set_u32(this + 0x30, 0);
    e.mem.set_u8(this + 0x34, 0);
    this
}

// Typed from 00709920
/// `Tile::GetTypeName` (Xbox PDB): the address of the string "TILE"
/// (`0106edc4`).
pub fn tile_get_type_name(_e: &mut Engine) -> u32 {
    0x0106_edc4
}

// Typed from 00709930
/// `Tile::_scalar_deleting_destructor_` (Xbox PDB): `Tile::~Tile`
/// (`009ff340`), then `operator delete` (`00401030`) when bit 0 of `flags` is
/// set. Returns `this`.
pub fn tile_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(TILE_DESTRUCTOR, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 00709960
/// `TileRect::GetType` (Xbox PDB): `0x385`.
pub fn tile_rect_get_type(_e: &mut Engine) -> u32 {
    0x385
}

// Typed from 00709970
/// `TileRect::GetTypeName` (Xbox PDB): the address of the string "RECT"
/// (`0106edcc`).
pub fn tile_rect_get_type_name(_e: &mut Engine) -> u32 {
    0x0106_edcc
}

// Typed from 00709980
/// `TileRect::_scalar_deleting_destructor_` (Xbox PDB): the `TileRect`
/// destructor ([`fn_007099b0`]), then `operator delete` (`00401030`) when
/// bit 0 of `flags` is set. Returns `this`.
pub fn tile_rect_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(super::FAMILY, 0x0070_99b0, fn_007099b0(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 007099b0
/// Destructor of `TileRect`: sets the `TileRect` vtable, runs
/// `Tile::Release` (`009ff690`) unless [`fn_00709a20`] says the tile has the
/// flag `0x2000`, then `Tile::~Tile` (`009ff340`). The compiler's
/// exception-unwinding frame is not translated.
pub fn fn_007099b0(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, TILE_RECT_VTABLE);
    if !typed_call!(super::FAMILY, 0x0070_9a20, fn_00709a20(e, this)) {
        e.call(TILE_RELEASE, &args![this]);
    }
    e.call(TILE_DESTRUCTOR, &args![this]);
}

// Typed from 00709a20
/// Whether the tile's flag word at `+0x30` has bit `0x2000`.
pub fn fn_00709a20(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(this + 0x30) & 0x2000 != 0
}

// Typed from 00709a40
/// `TileMenu::GetType` (Xbox PDB): `0x389`.
pub fn tile_menu_get_type(_e: &mut Engine) -> u32 {
    0x389
}

// Typed from 00709a50
/// `TileMenu::GetTypeName` (Xbox PDB): the address of the string "MENU"
/// (`0106edd4`).
pub fn tile_menu_get_type_name(_e: &mut Engine) -> u32 {
    0x0106_edd4
}

// Typed from 00709a60
/// `TileMenu::_scalar_deleting_destructor_` (Xbox PDB): the `TileMenu`
/// destructor (`00a1eff0`), then `operator delete` (`00401030`) when bit 0
/// of `flags` is set. Returns `this`.
pub fn tile_menu_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(TILE_MENU_DESTRUCTOR, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 00709a90
/// Clears the kind and the first word of both queued menus (`011d8950`,
/// `011d8954`, `011d8998`, `011d899c`).
pub fn fn_00709a90(e: &mut Engine) {
    e.mem.set_u32(QUEUE_KIND, 0);
    e.mem.set_u32(QUEUE_FIRST, 0);
    e.mem.set_u32(MESSAGE_QUEUE_KIND, 0);
    e.mem.set_u32(MESSAGE_QUEUE_FIRST, 0);
}

// Typed from 00709ac0
/// The `float` constant at `01016264` (0.75 in the installed exe).
pub fn fn_00709ac0(e: &mut Engine) -> f32 {
    e.global::<f32>(RETURNED_SCALE)
}

// Typed from 00709ae0
/// Static wrapper: calls `007e47a0` (the surgery menu).
pub fn fn_00709ae0(e: &mut Engine) {
    e.call(SURGERY_MENU_FN, &[]);
}

// Typed from 00709af0
/// Static wrapper: calls `0078e7c0` (the lockpick menu).
pub fn fn_00709af0(e: &mut Engine) {
    e.call(LOCKPICK_MENU_FN, &[]);
}

// Typed from 00709b00
/// Static wrapper: calls `007c19c0` (the slot machine menu).
pub fn fn_00709b00(e: &mut Engine) {
    e.call(SLOT_MACHINE_MENU_FN, &[]);
}

// Typed from 00709b10
/// Static wrapper: calls `00733500` (the blackjack menu).
pub fn fn_00709b10(e: &mut Engine) {
    e.call(BLACKJACK_MENU_FN, &[]);
}

// Typed from 00709b20
/// Static wrapper: calls `007bbcf0` (the roulette menu).
pub fn fn_00709b20(e: &mut Engine) {
    e.call(ROULETTE_MENU_FN, &[]);
}

// Typed from 00709b30
/// Static wrapper: calls `00740f30` (the Caravan menu).
pub fn fn_00709b30(e: &mut Engine) {
    e.call(CARAVAN_MENU_FN, &[]);
}

// Typed from 00709b40
/// Static wrapper: calls `0077f0d0` (the HUD menu).
pub fn fn_00709b40(e: &mut Engine) {
    e.call(HUD_MAIN_MENU_FN, &[]);
}

/// This file's typed functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00705a90, fn_00705a90() -> u32),
        entry!(0x00705ad0, fn_00705ad0(u32)),
        entry!(0x00705b10, fn_00705b10(u32, u32)),
        entry!(0x00705b30, fn_00705b30(u32)),
        entry!(0x00705b70, fn_00705b70(u8, u8, u8, i32)),
        entry!(0x00705c40, fn_00705c40(i32, i8) -> i32),
        entry!(0x00705c70, fn_00705c70() -> u8),
        entry!(0x00705cb0, fn_00705cb0() -> u8),
        entry!(0x00705cf0, fn_00705cf0(u32) -> u8),
        entry!(0x00705d10, fn_00705d10() -> u8),
        entry!(0x00705d50, fn_00705d50() -> u8),
        entry!(0x00705d90, fn_00705d90() -> i32),
        entry!(0x00705dd0, fn_00705dd0(u32) -> i32),
        entry!(0x00705e00, interface_create_loading_menu(u32, u8) -> u32),
        entry!(0x00705e30, interface_close_loading_menu()),
        entry!(0x00705e80, interface_get_loading_menu_visible() -> u8),
        entry!(0x00705ea0, interface_is_in_game_loading_menu_open() -> bool),
        entry!(0x00705ec0, interface_create_hacking_menu(u32) -> bool),
        entry!(0x00706130, interface_create_computers_menu(u32) -> bool),
        entry!(0x00706230, fn_00706230(u32, u32)),
        entry!(0x00706270, interface_create_level_up_menu()),
        entry!(0x007062a0, fn_007062a0(u32, u32, u8, u8)),
        entry!(0x007062e0, interface_set_info_for_ref(u32, u8, u8) -> u8),
        entry!(0x00706320, interface_close_all_support_menus(u32)),
        entry!(0x007065c0, fn_007065c0() -> u16),
        entry!(0x00706610, interface_save_game()),
        entry!(0x007066d0, fn_007066d0(u32)),
        entry!(0x00706810, fn_00706810(u32) -> i32),
        entry!(0x00706830, fn_00706830() -> u8),
        entry!(0x00706860, interface_load_game(u32)),
        entry!(0x007069d0, fn_007069d0(u8)),
        entry!(0x007069f0, fn_007069f0(u32, u32)),
        entry!(0x00706a20, fn_00706a20(u8)),
        entry!(0x00706a40, fn_00706a40(u32)),
        entry!(0x00706a90, interface_finish_load_game()),
        entry!(0x00706ac0, fn_00706ac0()),
        entry!(0x00706ae0, fn_00706ae0()),
        entry!(0x00706b90, fn_00706b90(u32)),
        entry!(0x00706c20, interface_tile_is_being_deleted(u32)),
        entry!(0x00706cb0, fn_00706cb0(u32)),
        entry!(0x00706cd0, interface_get_menus_root() -> u32),
        entry!(0x00706cf0, fn_00706cf0() -> u32),
        entry!(0x00706d10, interface_get_debug_text_root() -> u32),
        entry!(0x00706d30, interface_get_interface_root() -> u32),
        entry!(0x00706d50, interface_attach_default_alpha_property(u32)),
        entry!(0x00706d70, interface_get_first_chance_texture_release() -> u8),
        entry!(0x00706d90, fn_00706d90(u32) -> u8),
        entry!(0x00706db0, fn_00706db0(u8)),
        entry!(0x00706dd0, fn_00706dd0(u8)),
        entry!(0x00706df0, interface_get_screen_height() -> f32),
        entry!(0x00706e00, interface_get_screen_width() -> f32),
        entry!(0x00706e10, interface_get_real_screen_height() -> f32),
        entry!(0x00706e20, fn_00706e20() -> f32),
        entry!(0x00706e40, interface_get_real_screen_width() -> f32),
        entry!(0x00706e50, fn_00706e50() -> f32),
        entry!(0x00706e70, interface_get_screen_aspect_ratio() -> f32),
        entry!(0x00706e80, fn_00706e80() -> f32),
        entry!(0x00706e90, fn_00706e90() -> f32),
        entry!(0x00706f20, fn_00706f20() -> u32),
        entry!(0x00706f30, interface_play_menu_sound(u32)),
        entry!(0x00706f50, interface_new_timer(u32, f32)),
        entry!(0x00706f70, interface_update_all_timers()),
        entry!(0x00706f90, interface_clear_timer(u32)),
        entry!(0x00706fb0, interface_add_to_enter_stack(u32) -> u32),
        entry!(0x00706fd0, interface_pop_from_enter_stack(u32, u8) -> u32),
        entry!(0x00706ff0, interface_set_current_focus_target(u32)),
        entry!(0x00707010, interface_recursive_fade(u32, f32, f32)),
        entry!(0x00707040, fn_00707040() -> u32),
        entry!(0x00707050, fn_00707050(u32)),
        entry!(0x00707070, fn_00707070() -> u32),
        entry!(0x00707080, fn_00707080(u32)),
        entry!(0x007070a0, fn_007070a0() -> u32),
        entry!(
            0x007070c0,
            interface_find_text_replacement_string(u32, u32, u32, u8) -> bool
        ),
        entry!(0x00707330, fn_00707330(u32) -> i32),
        entry!(0x007073d0, fn_007073d0(u32, u32) -> bool),
        entry!(0x00707640, interface_release_model_file(u32)),
        entry!(0x00707660, interface_load_model_file(u32, u32) -> u32),
        entry!(0x00707820, interface_clear_temp_model()),
        entry!(0x00707860, fn_00707860() -> u32),
        entry!(0x00707870, interface_copy_or_deep_copy_node(u32) -> u32),
        entry!(0x00707930, interface_set_need_to_update(u8)),
        entry!(0x00707950, fn_00707950(u32, u8)),
        entry!(0x00707970, fn_00707970(u32) -> bool),
        entry!(0x00707990, interface_create_menu_by_class(u32) -> u32),
        entry!(0x007079b0, interface_is_in_rendered_menu() -> bool),
        entry!(0x007079f0, fn_007079f0(u32) -> bool),
        entry!(0x00707a30, interface_is_rendered_menu(u32) -> bool),
        entry!(0x00707ad0, interface_get_current_rendered_menu() -> u32),
        entry!(0x00707af0, fn_00707af0(u32) -> u32),
        entry!(0x00707b30, interface_is_current_rendered_menu_topmost() -> bool),
        entry!(0x00707b40, fn_00707b40() -> u8),
        entry!(0x00707b50, fn_00707b50() -> u8),
        entry!(0x00707b60, fn_00707b60() -> f32),
        (0x00707be0, interface_initialize_tabline as AbiFn),
        entry!(0x00707e10, fn_00707e10(u32, u32)),
        entry!(0x00707e30, interface_populate_item_stats_display(u32, u32)),
        entry!(
            0x00709470,
            interface_queue_menu_create(u32, u32, u32, u32, u32)
        ),
        entry!(0x007094f0, interface_handle_queued_menu_open()),
        entry!(0x00709810, fn_00709810(u32, u32, u32) -> u32),
        entry!(0x00709840, fn_00709840(u32) -> u32),
        entry!(0x00709860, fn_00709860(u32) -> u32),
        entry!(0x00709920, tile_get_type_name() -> u32),
        entry!(0x00709930, tile_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x00709960, tile_rect_get_type() -> u32),
        entry!(0x00709970, tile_rect_get_type_name() -> u32),
        entry!(0x00709980, tile_rect_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x007099b0, fn_007099b0(u32)),
        entry!(0x00709a20, fn_00709a20(u32) -> bool),
        entry!(0x00709a40, tile_menu_get_type() -> u32),
        entry!(0x00709a50, tile_menu_get_type_name() -> u32),
        entry!(0x00709a60, tile_menu_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x00709a90, fn_00709a90()),
        entry!(0x00709ac0, fn_00709ac0() -> f32),
        entry!(0x00709ae0, fn_00709ae0()),
        entry!(0x00709af0, fn_00709af0()),
        entry!(0x00709b00, fn_00709b00()),
        entry!(0x00709b10, fn_00709b10()),
        entry!(0x00709b20, fn_00709b20()),
        entry!(0x00709b30, fn_00709b30()),
        entry!(0x00709b40, fn_00709b40()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `00706e90` at the main thread's 24-bit precision: the quotient is a float before the
    /// product, so 1280 * (768 / 1366) is 719.64856, where one rounding of the product kept in a
    /// double gives 719.648621.
    #[test]
    fn the_screen_ratio_rounds_the_quotient_then_the_product() {
        let mut e = Engine::new();
        e.map(0x0106_e000, 0x1000);
        e.map(0x011f_4000, 0x1000);
        e.mem.set_f32(0x0106_ec38, 1280.0);
        let object = e.mem.alloc(8);
        e.mem.set_u32(object, 0x0200_2000);
        e.put_vtable(0x0200_2000, &[0u32; 0x40]);
        e.mem.set_u32(0x0200_2000 + 0xc8, 0x0200_0001);
        e.mem.set_u32(0x0200_2000 + 0x8c, 0x0200_0002);
        e.mem.set_u32(0x0200_2000 + 0x90, 0x0200_0003);
        e.mem.set_u32(0x011f_4748, object);
        for (at, answer) in [
            (0x0200_0001, object),
            (0x0200_0002, 1366),
            (0x0200_0003, 768),
        ] {
            e.register_double(at, move |_, _| Ret {
                eax: answer,
                ..Ret::default()
            });
        }
        let value = fn_00706e90(&mut e);
        assert_eq!(value.to_bits(), 0x4433_e982, "{value}");
        let once = super::super::narrow(1280.0 * (768.0 / 1366.0));
        assert_ne!(once, value, "the case tells the two roundings apart");
    }
}
