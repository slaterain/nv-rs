//! `fallout/interface/interface.cpp` (Xbox PDB source unit), part 2: its functions from `00705a90` up to
//! (not including) `00709b50` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::interface`]; anything public there may be used here.
//!
//! This part covers `00705a90`..`00707870` so far. As in the first part, most
//! functions are static `Interface::` wrappers over the interface manager
//! (`InterfaceManager`, Xbox PDB): `004b7210` returns the manager and
//! `009373f0` reads its `bFirstInit` byte; a wrapper does nothing unless the
//! manager exists and that byte is set. Manager offsets used here (PC
//! offsets match the Xbox PDB): `+0x9c pMenusRoot`, `+0xbc` and `+0xc4`
//! words set by `00706cb0` and `00705b10`, `+0xcc`/`+0xd4` tile slots
//! cleared by `TileIsBeingDeleted`, `+0x4bc` a mode word and the sub-object
//! at `+0x4d4`.
//!
//! The compiler's exception-unwinding frames (`CreateHackingMenu`,
//! `CreateComputersMenu`) and stack-cookie checks are not translated.

#[allow(unused_imports)]
use super::interface::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// `004b7210`: the interface manager object.
const GET_MANAGER: u32 = 0x004b_7210;
/// `009373f0`: the byte at the start of the manager (`bFirstInit`).
const MANAGER_READY: u32 = 0x0093_73f0;
/// `00586150`: the manager's `pMenusRoot`.
const GET_MENUS_ROOT: u32 = 0x0058_6150;
/// `00559450`: the first word of the object (a list head).
const FIRST_WORD: u32 = 0x0055_9450;
/// `0057cbe0`: list iterator step (list, address of the cursor word); it
/// returns the address of the current item and advances the cursor.
const LIST_NEXT: u32 = 0x0057_cbe0;
/// `0057c730`: takes the address of a word holding an item and returns the
/// object it removes from the list (0 when there is none).
const LIST_REMOVE: u32 = 0x0057_c730;
/// `Tile::GetMenu` (Xbox PDB): the menu a tile belongs to.
const TILE_GET_MENU: u32 = 0x00a0_3c90;
/// Tile trait getter (`00a011b0`, value in ST0): tile, trait id.
const TILE_GET_VALUE: u32 = 0x00a0_11b0;
/// Tile trait setter (`00700320`): tile, trait id, value.
const TILE_SET_VALUE: u32 = 0x0070_0320;
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
/// `00984f60` on the rendered menu: takes the menu it renders.
const RENDERED_MENU_SET_MENU: u32 = 0x0098_4f60;
/// `0056c7f0`: the node a menu keeps.
const MENU_NODE: u32 = 0x0056_c7f0;
/// `00705fc0` on the rendered menu: takes that node.
const RENDERED_MENU_SET_NODE: u32 = 0x0070_5fc0;
/// `InterfaceManager::EnterRenderedMenu` (Xbox PDB), on the manager.
const ENTER_RENDERED_MENU: u32 = 0x0071_81d0;
/// `00408d60` called with `ECX = 011db620`: returns the address of a byte
/// flag (nonzero when rendered menus are used).
const RENDERED_MENUS_FLAG_GETTER: u32 = 0x0040_8d60;
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
/// `_ftol2_sse` (`00ec62c0`): the value is passed as an `f64`.
const FTOL: u32 = 0x00ec_62c0;
/// `004a8f20`, `cdecl`, two words (its body is not part of this unit).
const HELPER_4A8F20: u32 = 0x004a_8f20;
/// Byte set to 1 while `CloseAllSupportMenus` runs.
const CLOSING_SUPPORT_MENUS: u32 = 0x011d_8909;
/// The loading menu object (0 when none).
const LOADING_MENU: u32 = 0x011d_a0c0;
/// `007027b0` (`cdecl`, one menu id), translated in the first part.
const MENU_ID_FLAG: u32 = 0x0070_27b0;
/// `00703fe0`, `cdecl`, one flag word.
const SET_LOADING_FLAG: u32 = 0x0070_3fe0;
/// `LoadingMenu::Create` (Xbox PDB), `cdecl`: three words.
const LOADING_MENU_CREATE: u32 = 0x0078_8cc0;
/// The pointer to the game's save object (`011de45c`).
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
/// Writes `(pointer, size)` bytes into the save game (`008579b0`).
const SAVE_GAME_WRITE: u32 = 0x0085_79b0;
/// `008df040` on the object in `011de45c`: returns a byte.
const SAVE_OBJECT_BYTE: u32 = 0x008d_f040;
/// Save game buffer write (`00865e50`): `(pointer, size, 0)`.
const SAVE_BUFFER_WRITE: u32 = 0x0086_5e50;
/// Load game buffer read (`00864980`): `(pointer, size)`.
const LOAD_BUFFER_READ: u32 = 0x0086_4980;
/// `BGSLoadGameBuffer::LoadString` (Xbox PDB): reads a string into the given
/// buffer.
const LOAD_BUFFER_LOAD_STRING: u32 = 0x0086_49a0;
/// `memset` (`00403d30`): destination, value, length.
const MEMSET: u32 = 0x0040_3d30;
/// The state getters written by `007066d0` (bytes, then floats) and the
/// setters `LoadGame` hands them to.
const STATS_PAGE_GET: u32 = 0x007d_fcd0;
const INVENTORY_BYTE_GET: u32 = 0x0078_24b0;
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

/// The manager now (each wrapper asks for it again where the game does).
fn manager(e: &mut Engine) -> u32 {
    e.call(GET_MANAGER, &[]).u32()
}

/// The wrappers' guard: `Some(manager)` only when it exists and
/// `009373f0` says so.
fn ready_manager(e: &mut Engine) -> Option<u32> {
    if manager(e) == 0 {
        return None;
    }
    let current = manager(e);
    if e.call(MANAGER_READY, &args![current]).bool() {
        Some(current)
    } else {
        None
    }
}

/// `_ftol2_sse` on a `float`.
fn float_to_int(e: &mut Engine, value: f32) -> i32 {
    e.call(FTOL, &args![value as f64]).i32()
}

// Translated from 00705a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `007058c0` on it (0 otherwise).
pub fn fn_00705a90(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => e.call(0x0070_58c0, &args![manager]).u32(),
        None => 0,
    }
}

// Translated from 00705ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: stores `value` in it (`00705b10`).
pub fn fn_00705ad0(e: &mut Engine, value: u32) {
    if ready_manager(e).is_some() {
        let current = manager(e);
        fn_00705b10(e, current, value);
    }
}

// Translated from 00705b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the manager's word at `+0xc4`.
pub fn fn_00705b10(e: &mut Engine, this: u32, value: u32) {
    e.mem.set_u32(this + 0xc4, value);
}

// Translated from 00705b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `00716010(manager, value)`.
pub fn fn_00705b30(e: &mut Engine, value: u32) {
    if ready_manager(e).is_some() {
        let current = manager(e);
        e.call(0x0071_6010, &args![current, value]);
    }
}

// Translated from 00705b70 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: gives it three values, each the byte (as a
/// signed byte) raised to at least 0 (`00705c40`) and passed through
/// `004a8f20(4, ..)` (`0070b790`, `0070b810`, `0070b870`), then the menu id
/// (`0070b8d0`), which is `menu_id` when it is `0x3eb`, `0x3ea` or `0x3ff`
/// and `0x3eb` otherwise.
pub fn fn_00705b70(e: &mut Engine, first: u8, second: u8, third: u8, menu_id: i32) {
    if ready_manager(e).is_none() {
        return;
    }
    for (setter, byte) in [
        (0x0070_b790, first),
        (0x0070_b810, second),
        (0x0070_b870, third),
    ] {
        let clamped = fn_00705c40(e, 0, byte as i8);
        let value = e.call(HELPER_4A8F20, &args![4u32, clamped]).u32();
        let current = manager(e);
        e.call(setter, &args![current, value]);
    }
    let menu_id = if menu_id == 0x3eb || menu_id == 0x3ea || menu_id == 0x3ff {
        menu_id
    } else {
        0x3eb
    };
    let current = manager(e);
    e.call(0x0070_b8d0, &args![current, menu_id]);
}

// Translated from 00705c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// The larger of `a` and the signed byte `b`.
pub fn fn_00705c40(_e: &mut Engine, a: i32, b: i8) -> i32 {
    if a > b as i32 {
        a
    } else {
        b as i32
    }
}

// Translated from 00705c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `0047ee30` on it, a byte (`0xff` otherwise).
pub fn fn_00705c70(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => e.call(0x0047_ee30, &args![manager]).u8(),
        None => 0xff,
    }
}

// Translated from 00705cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: the byte at its `+0x13` (`0xff` otherwise).
pub fn fn_00705cb0(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => fn_00705cf0(e, manager),
        None => 0xff,
    }
}

// Translated from 00705cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0x13` of the manager.
pub fn fn_00705cf0(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(this + 0x13)
}

// Translated from 00705d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `00843150` on it, a byte (`0xff` otherwise).
pub fn fn_00705d10(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => e.call(0x0084_3150, &args![manager]).u8(),
        None => 0xff,
    }
}

// Translated from 00705d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `00982bc0` on it, a byte (`0xff` otherwise).
pub fn fn_00705d50(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => e.call(0x0098_2bc0, &args![manager]).u8(),
        None => 0xff,
    }
}

// Translated from 00705d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `00705dd0` on it (`-1` otherwise).
pub fn fn_00705d90(e: &mut Engine) -> i32 {
    match ready_manager(e) {
        Some(manager) => fn_00705dd0(e, manager),
        None => -1,
    }
}

// Translated from 00705dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Trait `0x1771` of the tile at `+0x9c` of the manager (`pMenusRoot`),
/// truncated to an integer.
pub fn fn_00705dd0(e: &mut Engine, this: u32) -> i32 {
    let root = e.mem.u32(this + 0x9c);
    let value = e
        .call(TILE_GET_VALUE, &args![root, TRAIT_SAVED_VALUE])
        .f32();
    float_to_int(e, value)
}

// Translated from 00705e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::CreateLoadingMenu` (Xbox PDB): sets the loading flag
/// (`00703fe0(1)`), then `LoadingMenu::Create(arg, 1, flag)`.
pub fn interface_create_loading_menu(e: &mut Engine, arg: u32, flag: u8) -> u32 {
    e.call(SET_LOADING_FLAG, &args![1u32]);
    e.call(LOADING_MENU_CREATE, &args![arg, 1u32, flag as u32])
        .u32()
}

// Translated from 00705e30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::CloseLoadingMenu` (Xbox PDB): destroys the loading menu
/// object when there is one (its virtual slot 0 with the argument 1), then
/// clears the loading flag (`00703fe0(0)`).
pub fn interface_close_loading_menu(e: &mut Engine) {
    let menu = e.mem.u32(LOADING_MENU);
    if menu != 0 {
        e.vcall(menu, 0, &args![1u32]);
    }
    e.call(SET_LOADING_FLAG, &args![0u32]);
}

// Translated from 00705e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetLoadingMenuVisible` (Xbox PDB): `007027b0(0x3ef)`.
pub fn interface_get_loading_menu_visible(e: &mut Engine) -> u8 {
    e.call(MENU_ID_FLAG, &args![0x3efu32]).u8()
}

// Translated from 00705ea0 (decompiled, FalloutNV.exe 1.4.0.525)
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
    e.call(RENDERED_MENU_SET_MENU, &args![rendered, menu]);
    let node = e.call(MENU_NODE, &args![menu]).u32();
    e.call(RENDERED_MENU_SET_NODE, &args![rendered, node]);
    let current = manager(e);
    e.call(ENTER_RENDERED_MENU, &args![current, rendered]);
}

/// Whether rendered menus are in use (the byte `00408d60` points at).
fn rendered_menus_in_use(e: &mut Engine) -> bool {
    let flag = e
        .call(
            RENDERED_MENUS_FLAG_GETTER,
            &args![RENDERED_MENUS_FLAG_OWNER],
        )
        .u32();
    e.mem.u8(flag) != 0
}

// Translated from 00705ec0 (decompiled, FalloutNV.exe 1.4.0.525)
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

// Translated from 00706130 (decompiled, FalloutNV.exe 1.4.0.525)
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

// Translated from 00706230 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `007e41f0(a, b)`.
pub fn fn_00706230(e: &mut Engine, a: u32, b: u32) {
    if ready_manager(e).is_some() {
        e.call(0x007e_41f0, &args![a, b]);
    }
}

// Translated from 00706270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::CreateLevelUpMenu` (Xbox PDB): when the manager is ready,
/// `LevelUpMenu::Create`.
pub fn interface_create_level_up_menu(e: &mut Engine) {
    if ready_manager(e).is_some() {
        e.call(0x0078_4c80, &[]);
    }
}

// Translated from 007062a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `CharGenMenu::Create(a, b, c, d)`.
pub fn fn_007062a0(e: &mut Engine, a: u32, b: u32, c: u8, d: u8) {
    if ready_manager(e).is_some() {
        e.call(0x0075_3420, &args![a, b, c as u32, d as u32]);
    }
}

// Translated from 007062e0 (decompiled, FalloutNV.exe 1.4.0.525)
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

// Translated from 00706320 (decompiled, FalloutNV.exe 1.4.0.525)
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
    let root = e.call(GET_MENUS_ROOT, &args![current]).u32();
    let list = root + 4;
    let first = e.call(FIRST_WORD, &args![list]).u32();
    // Two words of the game's stack: the list cursor and the current tile.
    e.with_stack(8, |e, slots| {
        let cursor = slots.addr();
        let tile_slot = slots.addr() + 4;
        e.mem.set_u32(cursor, first);
        while e.mem.u32(cursor) != 0 {
            let item = e.call(LIST_NEXT, &args![list, cursor]).u32();
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

// Translated from 007065c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// 8, or 9 when the byte `008df040` returns for the object in `011de45c`
/// is at least `0x5d`.
pub fn fn_007065c0(e: &mut Engine) -> u16 {
    let owner = e.mem.u32(SAVE_LOAD_GAME);
    let byte = e.call(SAVE_OBJECT_BYTE, &args![owner]).u8();
    if byte >= 0x5d {
        9
    } else {
        8
    }
}

// Translated from 00706610 (decompiled, FalloutNV.exe 1.4.0.525)
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
        let value = fn_00705c70(e);
        e.mem.set_u8(first, value);
        let value = fn_00705cb0(e);
        e.mem.set_u8(second, value);
        let value = fn_00705d10(e);
        e.mem.set_u8(third, value);
        let value = fn_00705d50(e);
        e.mem.set_u8(fourth, value);
        let word = fn_00705d90(e);
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

// Translated from 007066d0 (decompiled, FalloutNV.exe 1.4.0.525)
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
        let root = e.call(GET_MENUS_ROOT, &args![current]).u32();
        let value = e
            .call(TILE_GET_VALUE, &args![root, TRAIT_SAVED_VALUE])
            .f32();
        let truncated = float_to_int(e, value);
        e.mem.set_i32(trait_value, truncated);
        let byte = e.call(STATS_PAGE_GET, &[]).u8();
        e.mem.set_u8(first_byte, byte);
        let byte = e.call(INVENTORY_BYTE_GET, &[]).u8();
        e.mem.set_u8(second_byte, byte);
        let byte = fn_00706830(e);
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

        e.call(MEMSET, &args![text, 0u32, STATE_STRING_SIZE]);
        let current = manager(e);
        let size = fn_00706810(e, current + MANAGER_SUBOBJECT);
        e.mem.set_i32(length, size);
        let current = manager(e);
        e.call(
            SUBOBJECT_GET_STRING,
            &args![current + MANAGER_SUBOBJECT, text],
        );
        e.call(SAVE_BUFFER_WRITE, &args![stream, length, 4u32, 0u32]);
        e.call(SAVE_BUFFER_WRITE, &args![stream, text, size as u32, 0u32]);
    });
}

// Translated from 00706810 (decompiled, FalloutNV.exe 1.4.0.525)
/// `004a8f20(1, 1) + 5` (the object it is called on is not used).
pub fn fn_00706810(e: &mut Engine, _unused_0: u32) -> i32 {
    e.call(HELPER_4A8F20, &args![1u32, 1u32])
        .i32()
        .wrapping_add(5)
}

// Translated from 00706830 (decompiled, FalloutNV.exe 1.4.0.525)
/// The low byte of `(word at +0x80 of the object in 011da368) - 0x20`, or 4
/// when there is no object.
pub fn fn_00706830(e: &mut Engine) -> u8 {
    let object = e.mem.u32(STATE_OBJECT);
    if object == 0 {
        4
    } else {
        e.mem.u32(object + 0x80).wrapping_sub(0x20) as u8
    }
}

// Translated from 00706860 (decompiled, FalloutNV.exe 1.4.0.525)
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
    // the length word at +0xc, the 100-byte buffer at +0x10, the saved
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
        let root = e.call(GET_MENUS_ROOT, &args![current]).u32();
        let saved = e.mem.u32(word);
        e.call(TILE_SET_VALUE, &args![root, TRAIT_SAVED_VALUE, saved]);
        let byte = e.mem.u8(first_byte);
        e.call(STATS_PAGE_SET, &args![byte as u32]);
        let byte = e.mem.u8(second_byte);
        fn_00706a20(e, byte);
        let byte = e.mem.u8(third_byte);
        fn_007069d0(e, byte);
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
            let (length, text) = (base + 0xc, base + 0x10);
            e.call(MEMSET, &args![text, 0u32, STATE_STRING_SIZE]);
            e.mem.set_u32(length, 0);
            e.call(LOAD_BUFFER_READ, &args![stream, length, 4u32]);
            let size = e.mem.u32(length);
            e.call(LOAD_BUFFER_READ, &args![stream, text, size]);
            let current = manager(e);
            e.call(
                SUBOBJECT_SET_STRING,
                &args![current + MANAGER_SUBOBJECT, text],
            );
        } else {
            e.call(LOAD_BUFFER_LOAD_STRING, &args![stream, base]);
        }
    });
}

// Translated from 007069d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the object in `011da368` exists: `007069f0(object, value)`.
pub fn fn_007069d0(e: &mut Engine, value: u8) {
    let object = e.mem.u32(STATE_OBJECT);
    if object != 0 {
        fn_007069f0(e, object, value as u32);
    }
}

// Translated from 007069f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Virtual slot `0xc` of the object with `(value + 0x20, 0)`.
pub fn fn_007069f0(e: &mut Engine, this: u32, value: u32) {
    e.vcall(this, 0xc, &args![value.wrapping_add(0x20), 0u32]);
}

// Translated from 00706a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `00706a40` with the byte.
pub fn fn_00706a20(e: &mut Engine, value: u8) {
    fn_00706a40(e, value as u32);
}

// Translated from 00706a40 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the object in `011d9ea4` exists: runs `value` through
/// `004a8f20(value, 4)`, sets that trait (the id in `011d9eac`) on the
/// object's tile (`+0x5c`) and stores it at `+0x84`.
pub fn fn_00706a40(e: &mut Engine, value: u32) {
    let object = e.mem.u32(TRAIT_OBJECT);
    if object == 0 {
        return;
    }
    let value = e.call(HELPER_4A8F20, &args![value, 4u32]).u32();
    let trait_id = e.mem.u32(TRAIT_OBJECT_TRAIT_ID);
    let tile = e.mem.u32(object + 0x5c);
    e.call(TILE_SET_VALUE, &args![tile, trait_id, value]);
    e.mem.set_u32(object + 0x84, value);
}

// Translated from 00706a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::FinishLoadGame` (Xbox PDB): calls, in order, `007dd710`,
/// `007df4e0`, `007df5d0`, `0079f640`, `00706ac0`, `007a1490` and
/// `007e5890`.
pub fn interface_finish_load_game(e: &mut Engine) {
    e.call(0x007d_d710, &[]);
    e.call(0x007d_f4e0, &[]);
    e.call(0x007d_f5d0, &[]);
    e.call(0x0079_f640, &[]);
    fn_00706ac0(e);
    e.call(0x007a_1490, &[]);
    e.call(0x007e_5890, &[]);
}

// Translated from 00706ac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at `+0x90` of the object in `011da368`, if any.
pub fn fn_00706ac0(e: &mut Engine) {
    let object = e.mem.u32(STATE_OBJECT);
    if object != 0 {
        e.mem.set_u32(object + 0x90, 0);
    }
}

// Translated from 00706ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte `011d8a85`, runs the functions `007053b0` and `00773050`,
/// and when `007050d0` says so sets a value (`00705190(0.0, 1)`) on the
/// object in `011dea3c` (`00953060`) and runs `0083e510`; then `00705780`,
/// `007f6920`, `CloseAllSupportMenus`, `007aa480`, `007dda60`, `00706ac0`,
/// `00798bf0`, `00706fd0(1, 1)`, `EnterRenderedMenu(0)`, clears the
/// manager's word at `+0x4bc` and runs `00706b90` on its sub-object.
pub fn fn_00706ae0(e: &mut Engine) {
    e.mem.set_u8(FINISH_FLAG, 1);
    e.call(0x0070_53b0, &[]);
    e.call(0x0077_3050, &[]);
    if e.call(0x0070_50d0, &[]).bool() {
        let value = e.call(0x0070_5190, &args![0.0f32, 1u32]).u32();
        let object = e.mem.u32(FINISH_OBJECT);
        e.call(0x0095_3060, &args![object, value]);
        e.call(0x0083_e510, &[]);
    }
    e.call(0x0070_5780, &[]);
    e.call(0x007f_6920, &[]);
    interface_close_all_support_menus(e, 0);
    e.call(0x007a_a480, &[]);
    e.call(0x007d_da60, &[]);
    fn_00706ac0(e);
    e.call(0x0079_8bf0, &[]);
    e.call(0x0070_6fd0, &args![1u32, 1u32]);
    let current = manager(e);
    e.call(ENTER_RENDERED_MENU, &args![current, 0u32]);
    let current = manager(e);
    e.mem.set_u32(current + 0x4bc, 0);
    let current = manager(e);
    fn_00706b90(e, current + MANAGER_SUBOBJECT);
}

// Translated from 00706b90 (decompiled, FalloutNV.exe 1.4.0.525)
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

// Translated from 00706c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::TileIsBeingDeleted` (Xbox PDB): forgets `tile` in the
/// manager: clears its slots at `+0xcc`/`+0xd0` and `+0xd4`/`+0xd8` when
/// they hold it, runs `00706cb0` when `0097ae90` returns it, and finally
/// `00713de0(manager, tile)`.
pub fn interface_tile_is_being_deleted(e: &mut Engine, tile: u32) {
    let current = manager(e);
    if e.mem.u32(current + 0xcc) == tile {
        e.mem.set_u32(current + 0xcc, 0);
        e.mem.set_u32(current + 0xd0, 0);
    }
    if e.mem.u32(current + 0xd4) == tile {
        e.mem.set_u32(current + 0xd4, 0);
        e.mem.set_u32(current + 0xd8, 0);
    }
    if e.call(0x0097_ae90, &args![current]).u32() == tile {
        fn_00706cb0(e, current);
    }
    e.call(0x0071_3de0, &args![current, tile]);
}

// Translated from 00706cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the manager's word at `+0xbc`.
pub fn fn_00706cb0(e: &mut Engine, this: u32) {
    e.mem.set_u32(this + 0xbc, 0);
}

// Translated from 00706cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetMenusRoot` (Xbox PDB): `00586150` on the manager.
pub fn interface_get_menus_root(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.call(0x0058_6150, &args![current]).u32()
}

// Translated from 00706cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `004fd400` on the manager (a getter; its body is not part of this unit).
pub fn fn_00706cf0(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.call(0x004f_d400, &args![current]).u32()
}

// Translated from 00706d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetDebugTextRoot` (Xbox PDB): `005e3fc0` on the manager.
pub fn interface_get_debug_text_root(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.call(0x005e_3fc0, &args![current]).u32()
}

// Translated from 00706d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetInterfaceRoot` (Xbox PDB): `004fb070` on the manager.
pub fn interface_get_interface_root(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.call(0x004f_b070, &args![current]).u32()
}

// Translated from 00706d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::AttachDefaultAlphaProperty` (Xbox PDB): takes the property
/// `006629f0` returns for the manager and attaches it to `node` with
/// `NiAVObject::AttachProperty` (`00439410`).
pub fn interface_attach_default_alpha_property(e: &mut Engine, node: u32) {
    let current = manager(e);
    let property = e.call(0x0066_29f0, &args![current]).u32();
    e.call(0x0043_9410, &args![node, property]);
}

// Translated from 00706d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetFirstChanceTextureRelease` (Xbox PDB): the byte at
/// `+0xec` of the manager (see `00706d90`).
pub fn interface_get_first_chance_texture_release(e: &mut Engine) -> u8 {
    let current = manager(e);
    fn_00706d90(e, current)
}

// Translated from 00706d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0xec` of `this` (the manager's first-chance texture
/// release flag, which `00703960` sets).
pub fn fn_00706d90(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(this + 0xec)
}

// Translated from 00706db0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `00703960(manager, value)`: sets the manager's byte at `+0xec`.
pub fn fn_00706db0(e: &mut Engine, value: u8) {
    let current = manager(e);
    e.call(0x0070_3960, &args![current, value]);
}

// Translated from 00706dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte in the manager's byte at `+0xdc`.
pub fn fn_00706dd0(e: &mut Engine, value: u8) {
    let current = manager(e);
    e.mem.set_u8(current + 0xdc, value);
}

// Translated from 00706df0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetScreenHeight` (Xbox PDB): the `float` `00715da0` returns
/// in `ST0`.
pub fn interface_get_screen_height(e: &mut Engine) -> f32 {
    e.call(0x0071_5da0, &[]).f32()
}

// Translated from 00706e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetScreenWidth` (Xbox PDB): the `float`
/// `InterfaceManager::GetScreenWidth` (`00715d40`) returns in `ST0`.
pub fn interface_get_screen_width(e: &mut Engine) -> f32 {
    e.call(0x0071_5d40, &[]).f32()
}

// Translated from 00706e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetRealScreenHeight` (Xbox PDB): `00706e20`.
pub fn interface_get_real_screen_height(e: &mut Engine) -> f32 {
    fn_00706e20(e)
}

// Translated from 00706e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The renderer's integer `004dc200` (the word at `01189480`) as a float
/// in `ST0`.
pub fn fn_00706e20(e: &mut Engine) -> f32 {
    e.call(0x004d_c200, &[]).i32() as f32
}

// Translated from 00706e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetRealScreenWidth` (Xbox PDB): `00706e50`.
pub fn interface_get_real_screen_width(e: &mut Engine) -> f32 {
    fn_00706e50(e)
}

// Translated from 00706e50 (decompiled, FalloutNV.exe 1.4.0.525)
/// The renderer's integer `004dc1f0` as a float in `ST0`.
pub fn fn_00706e50(e: &mut Engine) -> f32 {
    e.call(0x004d_c1f0, &[]).i32() as f32
}

// Translated from 00706e70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetScreenAspectRatio` (Xbox PDB): the manager's `float` at
/// `+0x4d0`.
pub fn interface_get_screen_aspect_ratio(e: &mut Engine) -> f32 {
    let current = manager(e);
    e.mem.f32(current + 0x4d0)
}

// Translated from 00706e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` constant at `0106ec38` (an aspect ratio).
pub fn fn_00706e80(e: &mut Engine) -> f32 {
    e.global::<f32>(ASPECT_RATIO_CONSTANT)
}

// Translated from 00706e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// A float from virtual calls on the object `0043c4b0` returns: the
/// constant of `00706e80` times (slot `0x90` of one result of that object's
/// slot `0xc8`, called with 0) divided by (slot `0x8c` of another result of
/// the same call, called with 0), the two counts read as unsigned.
pub fn fn_00706e90(e: &mut Engine) -> f32 {
    let object = e.call(0x0043_c4b0, &[]).u32();
    let first = e.vcall(object, 0xc8, &[]).u32();
    let second = e.vcall(object, 0xc8, &[]).u32();
    let divisor = e.vcall(second, 0x8c, &args![0u32]).u32() as f64;
    let dividend = e.vcall(first, 0x90, &args![0u32]).u32() as f64;
    let ratio = dividend / divisor;
    let base = fn_00706e80(e) as f64;
    (base * ratio) as f32
}

// Translated from 00706f20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The constant `0x3ec`.
pub fn fn_00706f20(_e: &mut Engine) -> u32 {
    0x3ec
}

// Translated from 00706f30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::PlayMenuSound` (Xbox PDB): `InterfaceManager::PlayMenuSound`
/// (`00717280`, `cdecl`) with the sound id.
pub fn interface_play_menu_sound(e: &mut Engine, sound: u32) {
    e.call(0x0071_7280, &args![sound]);
}

// Translated from 00706f50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::NewTimer` (Xbox PDB): `InterfaceManager::NewTimer`
/// (`007164c0`, `cdecl`) with the word and the `float`.
pub fn interface_new_timer(e: &mut Engine, id: u32, seconds: f32) {
    e.call(0x0071_64c0, &args![id, seconds]);
}

// Translated from 00706f70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::UpdateAllTimers` (Xbox PDB): `InterfaceManager::
/// UpdateAllTimers` (`00716320`) on the manager.
pub fn interface_update_all_timers(e: &mut Engine) {
    let current = manager(e);
    e.call(0x0071_6320, &args![current]);
}

// Translated from 00706f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::ClearTimer` (Xbox PDB): `InterfaceManager::ClearTimer`
/// (`007165d0`, `cdecl`, finds the manager itself) with the timer id. The
/// game also calls `004b7210` first and ignores the result.
pub fn interface_clear_timer(e: &mut Engine, id: u32) {
    manager(e);
    e.call(0x0071_65d0, &args![id]);
}

// Translated from 00706fb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::AddToEnterStack` (Xbox PDB): `InterfaceManager::
/// AddToEnterStack` (`00714d90`) on the manager.
pub fn interface_add_to_enter_stack(e: &mut Engine, menu_id: u32) -> u32 {
    let current = manager(e);
    e.call(0x0071_4d90, &args![current, menu_id]).u32()
}

// Translated from 00706fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::PopFromEnterStack` (Xbox PDB): `InterfaceManager::
/// PopFromEnterStack` (`00714fd0`) on the manager, with the menu id and a
/// byte flag.
pub fn interface_pop_from_enter_stack(e: &mut Engine, menu_id: u32, flag: u8) -> u32 {
    let current = manager(e);
    e.call(0x0071_4fd0, &args![current, menu_id, flag]).u32()
}

// Translated from 00706ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::SetCurrentFocusTarget` (Xbox PDB): `InterfaceManager::
/// SetCurrentFocusTarget` (`00715860`) on the manager with the tile, the
/// word `0xfc3` and the flag 1.
pub fn interface_set_current_focus_target(e: &mut Engine, tile: u32) {
    let current = manager(e);
    e.call(0x0071_5860, &args![current, tile, 0xfc3u32, 1u32]);
}

// Translated from 00707010 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::RecursiveFade` (Xbox PDB): `InterfaceManager::RecursiveFade`
/// (`00712450`) on the manager with the tile and the two floats.
pub fn interface_recursive_fade(e: &mut Engine, tile: u32, from: f32, to: f32) {
    let current = manager(e);
    e.call(0x0071_2450, &args![current, tile, from, to]);
}

// Translated from 00707040 (decompiled, FalloutNV.exe 1.4.0.525)
/// The manager's word at `+0xcc`.
pub fn fn_00707040(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.mem.u32(current + 0xcc)
}

// Translated from 00707050 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the word in the manager's `+0xcc`.
pub fn fn_00707050(e: &mut Engine, value: u32) {
    let current = manager(e);
    e.mem.set_u32(current + 0xcc, value);
}

// Translated from 00707070 (decompiled, FalloutNV.exe 1.4.0.525)
/// The manager's word at `+0xd0`.
pub fn fn_00707070(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.mem.u32(current + 0xd0)
}

// Translated from 00707080 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the word in the manager's `+0xd0`.
pub fn fn_00707080(e: &mut Engine, value: u32) {
    let current = manager(e);
    e.mem.set_u32(current + 0xd0, value);
}

// Translated from 007070a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `0045cd60` on the manager (a getter; its body is not part of this unit).
pub fn fn_007070a0(e: &mut Engine) -> u32 {
    let current = manager(e);
    e.call(0x0045_cd60, &args![current]).u32()
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

// Translated from 007070c0 (decompiled, FalloutNV.exe 1.4.0.525)
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
    let index = fn_00707330(e, name);
    if index < 0x1c {
        e.call(0x0070_39b0, &args![index, dest, flag]);
        found = true;
    }
    if !found {
        let form = e.call(0x0048_3a00, &args![name]).u32();
        if form != 0 && e.call(0x0040_1170, &args![form]).u32() == 6 {
            let kind = e.call(0x0052_9ea0, &args![form]).u32() as u8 as i8;
            let value = e.call(0x0052_6ac0, &args![form]).f32();
            if kind == 0x66 {
                e.call(0x0040_6d00, &args![dest, size, FLOAT_FORMAT, value as f64]);
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
            let text = e.call(0x0040_3df0, &args![entry]).u32();
            copy_text(e, dest, size, text);
            found = true;
        }
    }
    if !found {
        found = fn_007073d0(e, name, dest);
    }
    found
}

// Translated from 00707330 (decompiled, FalloutNV.exe 1.4.0.525)
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
            let text = e.call(0x0040_3df0, &args![entry]).u32();
            if e.call(0x0046_9880, &args![text, name]).i32() == 0 {
                break;
            }
            index += 1;
        }
    }
    index as i32
}

// Translated from 007073d0 (decompiled, FalloutNV.exe 1.4.0.525)
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
        match e.mem.u8(buffer) {
            b's' | b'S' => {
                let text = e.call(0x0040_3df0, &args![setting]).u32();
                e.call(0x0040_46f0, &args![dest, text]);
            }
            b'i' | b'I' => {
                let slot = e.call(0x0043_d4d0, &args![setting]).u32();
                let value = e.mem.u32(slot);
                e.call(0x00ec_623a, &args![buffer, INTEGER_FORMAT, value]);
                e.call(0x0040_46f0, &args![dest, buffer]);
            }
            b'f' | b'F' => {
                let slot = e.call(0x0040_3e20, &args![setting]).u32();
                let value = e.mem.f32(slot) as f64;
                e.call(0x00ec_623a, &args![buffer, INTEGER_FORMAT, value]);
                e.call(0x0040_46f0, &args![dest, buffer]);
            }
            b'b' | b'B' => {
                let slot = e.call(0x0040_8d60, &args![setting]).u32();
                let text = if e.mem.u8(slot) == 1 {
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

// Translated from 00707640 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::ReleaseModelFile` (Xbox PDB): when the model loader
/// (`011c3b3c`) exists, `0045a5e0(loader, model)` (releases the model).
pub fn interface_release_model_file(e: &mut Engine, model: u32) {
    let loader = e.mem.u32(MODEL_LOADER);
    if loader != 0 {
        e.call(0x0045_a5e0, &args![loader, model]);
    }
}

// Translated from 00707660 (decompiled, FalloutNV.exe 1.4.0.525)
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
        let result = e.call(0x0043_b230, &args![model]).u32();
        e.call(0x0043_d090, &args![stream]);
        result
    })
}

// Translated from 00707820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::ClearTempModel` (Xbox PDB): destroys the model in `011d890c`
/// (`004431f0(model, 1)`) if there is one and clears the pointer.
pub fn interface_clear_temp_model(e: &mut Engine) {
    let model = e.mem.u32(TEMP_MODEL);
    if model != 0 {
        e.call(0x0044_31f0, &args![model, 1u32]);
    }
    e.mem.set_u32(TEMP_MODEL, 0);
}

// Translated from 00707860 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `0x101dccc` of a string in the exe (`"Meshes"`).
pub fn fn_00707860(_e: &mut Engine) -> u32 {
    0x0101_dccc
}

// Translated from 00707870 (decompiled, FalloutNV.exe 1.4.0.525)
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

/// This part's translated functions, by exe address.
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// Registers a double that returns a fixed value in `eax`.
    fn stub(e: &mut Engine, addr: u32, value: u32) {
        e.register_double(addr, move |_, _| ret(value));
    }

    /// An engine whose manager exists and is ready, with the pages holding
    /// the globals the code reads mapped, and the call log on.
    fn ui_engine() -> (Engine, u32) {
        let mut e = Engine::new();
        for page in [
            0x011d_8000,
            0x011d_9000,
            0x011d_a000,
            0x011d_b000,
            0x011d_e000,
        ] {
            e.map(page, 0x1000);
        }
        let manager = e.mem.alloc(0x600);
        e.mem.set_u32(0x011d_8a80, manager);
        e.mem.set_u8(manager, 1);
        e.register(MANAGER_READY, |e, a| ret(e.mem.u8(a[0]) as u32));
        e.call_log = Some(vec![]);
        (e, manager)
    }

    fn logged(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    }

    /// The addresses called, in order, including the top-level call.
    fn order(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| *a != GET_MANAGER && *a != MANAGER_READY)
            .collect()
    }

    /// Checks a wrapper that calls `callee` with `expected(manager)` when
    /// the manager is ready (returning `ready_result`) and does nothing,
    /// returning `default`, when the manager is not ready or missing.
    fn check_forward(
        wrapper: u32,
        wrapper_args: &[u32],
        callee: u32,
        expected: impl Fn(u32) -> Vec<u32>,
        ready_result: u32,
        default: u32,
    ) {
        let (mut e, manager) = ui_engine();
        stub(&mut e, callee, ready_result);
        assert_eq!(e.call(wrapper, wrapper_args).u32(), ready_result);
        assert_eq!(logged(&e, callee), vec![expected(manager)]);

        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(wrapper, wrapper_args).u32(), default);
        assert!(logged(&e, callee).is_empty());

        e.mem.set_u32(0x011d_8a80, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(wrapper, wrapper_args).u32(), default);
        assert!(logged(&e, callee).is_empty());
    }

    #[test]
    fn fn_00705a90_forwards_to_the_manager() {
        check_forward(0x0070_5a90, &[], 0x0070_58c0, |m| vec![m], 7, 0);
    }

    #[test]
    fn fn_00705ad0_stores_the_word_when_ready() {
        let (mut e, manager) = ui_engine();
        e.call(0x0070_5ad0, &args![0x1234u32]);
        assert_eq!(e.mem.u32(manager + 0xc4), 0x1234);
        e.mem.set_u8(manager, 0);
        e.call(0x0070_5ad0, &args![0x5678u32]);
        assert_eq!(e.mem.u32(manager + 0xc4), 0x1234);
    }

    #[test]
    fn fn_00705b10_stores_the_word() {
        let (mut e, manager) = ui_engine();
        e.call(0x0070_5b10, &args![manager, 9u32]);
        assert_eq!(e.mem.u32(manager + 0xc4), 9);
    }

    #[test]
    fn fn_00705b30_forwards_the_value() {
        check_forward(
            0x0070_5b30,
            &args![0x42u32],
            0x0071_6010,
            |m| vec![m, 0x42],
            0,
            0,
        );
    }

    #[test]
    fn fn_00705b70_clamps_the_bytes_and_the_menu_id() {
        let (mut e, manager) = ui_engine();
        // 004a8f20(4, x) doubles x.
        e.register(HELPER_4A8F20, |_, a| ret(a[1] * 2));
        for setter in [0x0070_b790, 0x0070_b810, 0x0070_b870, 0x0070_b8d0] {
            stub(&mut e, setter, 0);
        }
        e.call(0x0070_5b70, &args![5u32, 0xffu32, 7u32, 0x999u32]);
        assert_eq!(logged(&e, 0x0070_b790), vec![vec![manager, 10]]);
        assert_eq!(logged(&e, 0x0070_b810), vec![vec![manager, 0]]);
        assert_eq!(logged(&e, 0x0070_b870), vec![vec![manager, 14]]);
        assert_eq!(logged(&e, 0x0070_b8d0), vec![vec![manager, 0x3eb]]);

        e.call_log = Some(vec![]);
        e.call(0x0070_5b70, &args![0u32, 0u32, 0u32, 0x3ffu32]);
        assert_eq!(logged(&e, 0x0070_b8d0), vec![vec![manager, 0x3ff]]);
        e.call_log = Some(vec![]);
        e.call(0x0070_5b70, &args![0u32, 0u32, 0u32, 0x3eau32]);
        assert_eq!(logged(&e, 0x0070_b8d0), vec![vec![manager, 0x3ea]]);

        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_5b70, &args![1u32, 1u32, 1u32, 0x3eau32]);
        assert!(logged(&e, 0x0070_b8d0).is_empty());
    }

    #[test]
    fn fn_00705c40_is_the_larger_with_a_signed_byte() {
        let (mut e, _) = ui_engine();
        assert_eq!(e.call(0x0070_5c40, &args![0u32, 5u32]).i32(), 5);
        assert_eq!(e.call(0x0070_5c40, &args![0u32, 0xffu32]).i32(), 0);
        assert_eq!(e.call(0x0070_5c40, &args![-3i32, 0xfdu32]).i32(), -3);
        assert_eq!(e.call(0x0070_5c40, &args![-3i32, 0xfeu32]).i32(), -2);
    }

    #[test]
    fn fn_00705c70_forwards_a_byte() {
        check_forward(0x0070_5c70, &[], 0x0047_ee30, |m| vec![m], 9, 0xff);
    }

    #[test]
    fn fn_00705cb0_reads_the_manager_byte() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager + 0x13, 6);
        assert_eq!(e.call(0x0070_5cb0, &[]).u32(), 6);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_5cb0, &[]).u32(), 0xff);
    }

    #[test]
    fn fn_00705cf0_reads_the_byte_at_13() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager + 0x13, 0x81);
        assert_eq!(e.call(0x0070_5cf0, &args![manager]).u32(), 0x81);
    }

    #[test]
    fn fn_00705d10_forwards_a_byte() {
        check_forward(0x0070_5d10, &[], 0x0084_3150, |m| vec![m], 3, 0xff);
    }

    #[test]
    fn fn_00705d50_forwards_a_byte() {
        check_forward(0x0070_5d50, &[], 0x0098_2bc0, |m| vec![m], 4, 0xff);
    }

    #[test]
    fn fn_00705d90_is_the_trait_value_or_minus_one() {
        let (mut e, manager) = ui_engine();
        let root = e.mem.alloc(8);
        e.mem.set_u32(manager + 0x9c, root);
        e.register_double(TILE_GET_VALUE, move |_, a| {
            assert_eq!(a, &[root, 0x1771]);
            Ret {
                st0: -2.5,
                ..Ret::default()
            }
        });
        assert_eq!(e.call(0x0070_5d90, &[]).i32(), -2);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_5d90, &[]).i32(), -1);
    }

    #[test]
    fn fn_00705dd0_truncates_the_root_trait() {
        let (mut e, manager) = ui_engine();
        let root = e.mem.alloc(8);
        e.mem.set_u32(manager + 0x9c, root);
        e.register(TILE_GET_VALUE, |_, _| Ret {
            st0: 7.9,
            ..Ret::default()
        });
        assert_eq!(e.call(0x0070_5dd0, &args![manager]).i32(), 7);
    }

    #[test]
    fn create_loading_menu_sets_the_flag_then_creates() {
        let (mut e, _) = ui_engine();
        stub(&mut e, SET_LOADING_FLAG, 0);
        stub(&mut e, LOADING_MENU_CREATE, 0x77);
        assert_eq!(e.call(0x0070_5e00, &args![0x10u32, 0x1ffu32]).u32(), 0x77);
        assert_eq!(
            order(&e),
            vec![0x0070_5e00, SET_LOADING_FLAG, LOADING_MENU_CREATE]
        );
        assert_eq!(logged(&e, SET_LOADING_FLAG), vec![vec![1]]);
        assert_eq!(logged(&e, LOADING_MENU_CREATE), vec![vec![0x10, 1, 0xff]]);
    }

    #[test]
    fn close_loading_menu_deletes_the_menu_then_clears_the_flag() {
        let (mut e, _) = ui_engine();
        stub(&mut e, SET_LOADING_FLAG, 0);
        stub(&mut e, 0x0200_0000, 0);
        e.put_vtable(0x0200_1000, &[0x0200_0000]);
        let menu = e.mem.alloc(8);
        e.mem.set_u32(menu, 0x0200_1000);
        e.mem.set_u32(LOADING_MENU, menu);
        e.call(0x0070_5e30, &[]);
        assert_eq!(order(&e), vec![0x0070_5e30, 0x0200_0000, SET_LOADING_FLAG]);
        assert_eq!(logged(&e, 0x0200_0000), vec![vec![menu, 1]]);
        assert_eq!(logged(&e, SET_LOADING_FLAG), vec![vec![0]]);

        e.mem.set_u32(LOADING_MENU, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_5e30, &[]);
        assert_eq!(order(&e), vec![0x0070_5e30, SET_LOADING_FLAG]);
    }

    #[test]
    fn get_loading_menu_visible_asks_for_menu_3ef() {
        let (mut e, _) = ui_engine();
        stub(&mut e, MENU_ID_FLAG, 1);
        assert_eq!(e.call(0x0070_5e80, &[]).u32(), 1);
        assert_eq!(logged(&e, MENU_ID_FLAG), vec![vec![0x3ef]]);
    }

    #[test]
    fn is_in_game_loading_menu_open_tests_the_object() {
        let (mut e, _) = ui_engine();
        assert!(!e.call(0x0070_5ea0, &[]).bool());
        e.mem.set_u32(LOADING_MENU, 0x1234);
        assert!(e.call(0x0070_5ea0, &[]).bool());
    }

    /// An engine for the rendered-menu creators: the creators' callees are
    /// doubles; the rendered-menus flag is `flag`.
    fn menu_engine(flag: bool) -> (Engine, u32) {
        let (mut e, manager) = ui_engine();
        stub(&mut e, HACKING_MENU_CREATE, 0x5000);
        stub(&mut e, COMPUTERS_MENU_CREATE, 0x5000);
        let flag_byte = e.mem.alloc(8);
        e.mem.set_u8(flag_byte, flag as u8);
        stub(&mut e, RENDERED_MENUS_FLAG_GETTER, flag_byte);
        stub(&mut e, TILE_UPDATE_ALL, 0);
        let block = e.mem.alloc(RENDERED_MENU_SIZE);
        stub(&mut e, OPERATOR_NEW, block);
        e.register(RENDERED_MENU_CTOR, |_, a| ret(a[0]));
        stub(&mut e, RENDERED_MENU_SET_MENU, 0);
        stub(&mut e, MENU_NODE, 0x6000);
        stub(&mut e, RENDERED_MENU_SET_NODE, 0);
        stub(&mut e, ENTER_RENDERED_MENU, 0);
        (e, manager)
    }

    #[test]
    fn create_hacking_menu_enters_a_rendered_menu_when_used() {
        let (mut e, manager) = menu_engine(true);
        assert!(e.call(0x0070_5ec0, &args![0x33u32]).bool());
        assert_eq!(
            order(&e),
            vec![
                0x0070_5ec0,
                HACKING_MENU_CREATE,
                RENDERED_MENUS_FLAG_GETTER,
                TILE_UPDATE_ALL,
                OPERATOR_NEW,
                RENDERED_MENU_CTOR,
                RENDERED_MENU_SET_MENU,
                MENU_NODE,
                RENDERED_MENU_SET_NODE,
                ENTER_RENDERED_MENU,
            ]
        );
        let block = logged(&e, RENDERED_MENU_CTOR)[0][0];
        assert_eq!(logged(&e, HACKING_MENU_CREATE), vec![vec![0x33]]);
        assert_eq!(logged(&e, OPERATOR_NEW), vec![vec![0xf0]]);
        assert_eq!(
            logged(&e, RENDERED_MENU_SET_MENU),
            vec![vec![block, 0x5000]]
        );
        assert_eq!(
            logged(&e, RENDERED_MENU_SET_NODE),
            vec![vec![block, 0x6000]]
        );
        assert_eq!(logged(&e, ENTER_RENDERED_MENU), vec![vec![manager, block]]);
    }

    #[test]
    fn create_hacking_menu_without_rendered_menus_only_creates() {
        let (mut e, _) = menu_engine(false);
        assert!(e.call(0x0070_5ec0, &args![0x33u32]).bool());
        assert_eq!(
            order(&e),
            vec![0x0070_5ec0, HACKING_MENU_CREATE, RENDERED_MENUS_FLAG_GETTER]
        );
    }

    #[test]
    fn create_hacking_menu_needs_a_ready_manager() {
        let (mut e, manager) = menu_engine(true);
        e.mem.set_u8(manager, 0);
        assert!(!e.call(0x0070_5ec0, &args![0x33u32]).bool());
        assert_eq!(order(&e), vec![0x0070_5ec0]);
    }

    #[test]
    fn create_hacking_menu_with_a_failed_allocation_passes_null() {
        let (mut e, _) = menu_engine(true);
        stub(&mut e, OPERATOR_NEW, 0);
        e.call(0x0070_5ec0, &args![0x33u32]);
        assert!(logged(&e, RENDERED_MENU_CTOR).is_empty());
        assert_eq!(logged(&e, RENDERED_MENU_SET_MENU), vec![vec![0, 0x5000]]);
    }

    #[test]
    fn create_computers_menu_updates_the_tiles_before_the_check() {
        let (mut e, manager) = menu_engine(true);
        assert!(e.call(0x0070_6130, &args![0x44u32]).bool());
        assert_eq!(
            order(&e),
            vec![
                0x0070_6130,
                COMPUTERS_MENU_CREATE,
                TILE_UPDATE_ALL,
                RENDERED_MENUS_FLAG_GETTER,
                OPERATOR_NEW,
                RENDERED_MENU_CTOR,
                RENDERED_MENU_SET_MENU,
                MENU_NODE,
                RENDERED_MENU_SET_NODE,
                ENTER_RENDERED_MENU,
            ]
        );
        assert_eq!(logged(&e, COMPUTERS_MENU_CREATE), vec![vec![0x44]]);
        assert_eq!(logged(&e, TILE_UPDATE_ALL), vec![vec![0]]);
        assert_eq!(logged(&e, ENTER_RENDERED_MENU)[0][0], manager);

        let (mut e, manager) = menu_engine(false);
        assert!(e.call(0x0070_6130, &args![0x44u32]).bool());
        assert_eq!(
            order(&e),
            vec![
                0x0070_6130,
                COMPUTERS_MENU_CREATE,
                TILE_UPDATE_ALL,
                RENDERED_MENUS_FLAG_GETTER
            ]
        );
        e.mem.set_u8(manager, 0);
        assert!(!e.call(0x0070_6130, &args![0x44u32]).bool());
    }

    #[test]
    fn fn_00706230_forwards_both_words() {
        check_forward(
            0x0070_6230,
            &args![3u32, 4u32],
            0x007e_41f0,
            |_| vec![3, 4],
            0,
            0,
        );
    }

    #[test]
    fn create_level_up_menu_runs_when_ready() {
        check_forward(0x0070_6270, &[], 0x0078_4c80, |_| vec![], 0, 0);
    }

    #[test]
    fn fn_007062a0_forwards_the_four_words() {
        check_forward(
            0x0070_62a0,
            &args![1u32, 2u32, 0x1ffu32, 0x2aau32],
            0x0075_3420,
            |_| vec![1, 2, 0xff, 0xaa],
            0,
            0,
        );
    }

    #[test]
    fn set_info_for_ref_forwards_and_returns_a_byte() {
        check_forward(
            0x0070_62e0,
            &args![0x99u32, 1u32, 0u32],
            0x0077_5a00,
            |_| vec![0x99, 1, 0],
            1,
            0,
        );
    }

    /// What the `CloseAllSupportMenus` scenario saw.
    struct Closing {
        e: Engine,
        tiles: Vec<u32>,
        removed: Vec<u32>,
        flag_when_removed: Vec<u8>,
    }

    const MENU_ID_SLOT_FUNCTION: u32 = 0x0200_0100;
    const DELETE_FUNCTION: u32 = 0x0200_0200;

    /// Runs `CloseAllSupportMenus` over tiles whose menus have the given
    /// ids (0 = a tile with no menu). `allow_3f5` is the answer of
    /// `004a4040`, `cast_ok` that of the dynamic casts.
    fn run_close(menu_ids: &[u32], allow_3f5: bool, cast_ok: bool) -> Closing {
        let (mut e, _) = ui_engine();
        let root = e.mem.alloc(16);
        stub(&mut e, GET_MENUS_ROOT, root);
        let mut menu_slots = [0u32; 14];
        menu_slots[13] = MENU_ID_SLOT_FUNCTION;
        e.put_vtable(0x0200_2000, &menu_slots);
        e.register(MENU_ID_SLOT_FUNCTION, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.put_vtable(0x0200_3000, &[DELETE_FUNCTION]);
        stub(&mut e, DELETE_FUNCTION, 0);
        let mut tiles = vec![];
        let mut menus = std::collections::HashMap::new();
        for &id in menu_ids {
            let tile = e.mem.alloc(8);
            tiles.push(tile);
            if id != 0 {
                let menu = e.mem.alloc(16);
                e.mem.set_u32(menu, 0x0200_2000);
                e.mem.set_u32(menu + 8, id);
                menus.insert(tile, menu);
            }
        }
        e.register_double(TILE_GET_MENU, move |_, a| {
            ret(menus.get(&a[0]).copied().unwrap_or(0))
        });
        // The list at root + 4: nodes [next, 0, tile].
        let mut next = 0;
        for &tile in tiles.iter().rev() {
            let node = e.mem.alloc(12);
            e.mem.set_u32(node, next);
            e.mem.set_u32(node + 8, tile);
            next = node;
        }
        e.mem.set_u32(root + 4, next);
        e.register(FIRST_WORD, |e, a| ret(e.mem.u32(a[0])));
        e.register(LIST_NEXT, |e, a| {
            let node = e.mem.u32(a[1]);
            let following = e.mem.u32(node);
            e.mem.set_u32(a[1], following);
            ret(node + 8)
        });
        let removed = Rc::new(RefCell::new(vec![]));
        let flags = Rc::new(RefCell::new(vec![]));
        let (removed_log, flag_log) = (removed.clone(), flags.clone());
        let victim = e.mem.alloc(8);
        e.mem.set_u32(victim, 0x0200_3000);
        e.register_double(LIST_REMOVE, move |e, a| {
            removed_log.borrow_mut().push(e.mem.u32(a[1]));
            flag_log.borrow_mut().push(e.mem.u8(CLOSING_SUPPORT_MENUS));
            ret(victim)
        });
        stub(&mut e, CLOSE_MENU_3F5_ALLOWED, allow_3f5 as u32);
        stub(&mut e, DYNAMIC_CAST, if cast_ok { 0x7777 } else { 0 });
        stub(&mut e, SPECIAL_BOOK_MENU_CLOSE, 0);
        stub(&mut e, MENU_432_CLOSE, 0);
        e.call(0x0070_6320, &args![0u32]);
        let removed = removed.borrow().clone();
        let flag_when_removed = flags.borrow().clone();
        Closing {
            e,
            tiles,
            removed,
            flag_when_removed,
        }
    }

    #[test]
    fn close_all_support_menus_keeps_the_listed_menus() {
        let run = run_close(
            &[0x3eb, 0x3ea, 0x3ec, 0x3ef, 0x3ff, 0x40b, 0x425],
            true,
            true,
        );
        assert!(run.removed.is_empty());
        assert!(logged(&run.e, DELETE_FUNCTION).is_empty());
        assert_eq!(run.e.mem.u8(CLOSING_SUPPORT_MENUS), 0);
    }

    #[test]
    fn close_all_support_menus_removes_other_menus_and_skips_menuless_tiles() {
        let run = run_close(&[0x100, 0, 0x3eb, 0x3f6], true, true);
        assert_eq!(run.removed, vec![run.tiles[0], run.tiles[3]]);
        assert_eq!(run.flag_when_removed, vec![1, 1]);
        assert_eq!(run.e.mem.u8(CLOSING_SUPPORT_MENUS), 0);
        assert_eq!(logged(&run.e, DELETE_FUNCTION).len(), 2);
        assert_eq!(logged(&run.e, DELETE_FUNCTION)[0][1], 1);
    }

    #[test]
    fn close_all_support_menus_removes_menu_3f5_only_when_allowed() {
        let run = run_close(&[0x3f5], false, true);
        assert!(run.removed.is_empty());
        let run = run_close(&[0x3f5], true, true);
        assert_eq!(run.removed, run.tiles);
        assert_eq!(logged(&run.e, DELETE_FUNCTION).len(), 1);
    }

    #[test]
    fn close_all_support_menus_closes_menu_424_when_the_cast_works() {
        let run = run_close(&[0x424], true, true);
        assert_eq!(logged(&run.e, SPECIAL_BOOK_MENU_CLOSE).len(), 1);
        assert_eq!(run.removed, run.tiles);
        let cast = &logged(&run.e, DYNAMIC_CAST)[0];
        assert_eq!(
            &cast[1..],
            &[0, CAST_SOURCE_TYPE, SPECIAL_BOOK_MENU_TYPE, 0]
        );

        let run = run_close(&[0x424], true, false);
        assert!(logged(&run.e, SPECIAL_BOOK_MENU_CLOSE).is_empty());
        assert_eq!(run.removed, run.tiles);
    }

    #[test]
    fn close_all_support_menus_closes_menu_432_when_the_cast_works() {
        let run = run_close(&[0x432], true, true);
        assert_eq!(logged(&run.e, MENU_432_CLOSE).len(), 1);
        assert_eq!(run.removed, run.tiles);
        assert_eq!(logged(&run.e, DYNAMIC_CAST)[0][3], MENU_432_TYPE);

        let run = run_close(&[0x432], true, false);
        assert!(logged(&run.e, MENU_432_CLOSE).is_empty());
        assert_eq!(run.removed, run.tiles);
    }

    #[test]
    fn fn_007065c0_is_nine_from_0x5d() {
        let (mut e, _) = ui_engine();
        let owner = e.mem.alloc(8);
        e.mem.set_u32(SAVE_LOAD_GAME, owner);
        stub(&mut e, SAVE_OBJECT_BYTE, 0x5c);
        assert_eq!(e.call(0x0070_65c0, &[]).u32(), 8);
        assert_eq!(logged(&e, SAVE_OBJECT_BYTE), vec![vec![owner]]);
        stub(&mut e, SAVE_OBJECT_BYTE, 0x5d);
        assert_eq!(e.call(0x0070_65c0, &[]).u32(), 9);
    }

    /// Registers a double on `addr` that records the bytes `(pointer, size)`
    /// found at argument words 1 and 2 (the save buffer writers).
    fn record_writes(e: &mut Engine, addr: u32) -> Rc<RefCell<Vec<Vec<u8>>>> {
        let log = Rc::new(RefCell::new(vec![]));
        let sink = log.clone();
        e.register_double(addr, move |e, a| {
            sink.borrow_mut().push(e.mem.bytes(a[1], a[2]));
            ret(0)
        });
        log
    }

    /// `memset` double: fills the destination with the value byte.
    fn stub_memset(e: &mut Engine) {
        e.register(MEMSET, |e, a| {
            for i in 0..a[2] {
                e.mem.set_u8(a[0] + i, a[1] as u8);
            }
            ret(a[0])
        });
    }

    #[test]
    fn save_game_writes_the_clamped_bytes_and_the_word() {
        let (mut e, manager) = ui_engine();
        let owner = e.mem.alloc(8);
        e.mem.set_u32(SAVE_LOAD_GAME, owner);
        e.mem.set_u8(manager + 0x13, 5);
        let root = e.mem.alloc(8);
        e.mem.set_u32(manager + 0x9c, root);
        stub(&mut e, 0x0047_ee30, 0);
        stub(&mut e, 0x0084_3150, 0x80);
        stub(&mut e, 0x0098_2bc0, 9);
        e.register(TILE_GET_VALUE, |_, _| Ret {
            st0: 300.0,
            ..Ret::default()
        });
        let writes = record_writes(&mut e, SAVE_GAME_WRITE);
        e.call(0x0070_6610, &[]);
        assert_eq!(
            *writes.borrow(),
            vec![
                vec![1],
                vec![5],
                vec![1],
                vec![9],
                300u32.to_le_bytes().to_vec()
            ]
        );
        assert_eq!(logged(&e, SAVE_GAME_WRITE)[0][0], owner);
    }

    #[test]
    fn save_game_without_a_manager_writes_the_defaults() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager, 0);
        let owner = e.mem.alloc(8);
        e.mem.set_u32(SAVE_LOAD_GAME, owner);
        let writes = record_writes(&mut e, SAVE_GAME_WRITE);
        e.call(0x0070_6610, &[]);
        // 0xff is -1 as a signed byte, raised to 1; the word is -1.
        assert_eq!(
            *writes.borrow(),
            vec![
                vec![1],
                vec![1],
                vec![1],
                vec![1],
                u32::MAX.to_le_bytes().to_vec()
            ]
        );
    }

    #[test]
    fn fn_007066d0_writes_the_menu_state() {
        let (mut e, manager) = ui_engine();
        let root = e.mem.alloc(8);
        stub(&mut e, GET_MENUS_ROOT, root);
        e.register(TILE_GET_VALUE, |_, _| Ret {
            st0: 300.0,
            ..Ret::default()
        });
        stub(&mut e, STATS_PAGE_GET, 0x11);
        stub(&mut e, INVENTORY_BYTE_GET, 0x22);
        let floats = Rc::new(RefCell::new(vec![1.5f64, 2.5]));
        e.register_double(MAP_FLOAT_GET_A, {
            let floats = floats.clone();
            move |_, _| Ret {
                st0: floats.borrow()[0],
                ..Ret::default()
            }
        });
        e.register_double(MAP_FLOAT_GET_B, move |_, _| Ret {
            st0: floats.borrow()[1],
            ..Ret::default()
        });
        stub_memset(&mut e);
        stub(&mut e, HELPER_4A8F20, 3);
        e.register(SUBOBJECT_GET_STRING, |e, a| {
            e.mem.set_cstr(a[1], b"hello");
            ret(0)
        });
        let stream = e.mem.alloc(8);
        let writes = record_writes(&mut e, SAVE_BUFFER_WRITE);
        e.call(0x0070_66d0, &args![stream]);
        let mut text = b"hello".to_vec();
        text.resize(8, 0);
        assert_eq!(
            *writes.borrow(),
            vec![
                300u32.to_le_bytes().to_vec(),
                vec![0x11],
                vec![0x22],
                vec![4],
                1.5f32.to_le_bytes().to_vec(),
                2.5f32.to_le_bytes().to_vec(),
                8u32.to_le_bytes().to_vec(),
                text,
            ]
        );
        assert_eq!(logged(&e, SAVE_BUFFER_WRITE)[0][0], stream);
        assert_eq!(logged(&e, SAVE_BUFFER_WRITE)[7][3], 0);
        assert_eq!(logged(&e, SUBOBJECT_GET_STRING)[0][0], manager + 0x4d4);
        assert_eq!(logged(&e, MEMSET)[0][1..], [0, 100]);
    }

    #[test]
    fn fn_00706810_is_five_more_than_the_helper() {
        let (mut e, _) = ui_engine();
        stub(&mut e, HELPER_4A8F20, 10);
        assert_eq!(e.call(0x0070_6810, &args![0x1234u32]).i32(), 15);
        assert_eq!(logged(&e, HELPER_4A8F20), vec![vec![1, 1]]);
    }

    #[test]
    fn fn_00706830_is_the_state_word_minus_0x20_or_4() {
        let (mut e, _) = ui_engine();
        assert_eq!(e.call(0x0070_6830, &[]).u32(), 4);
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(STATE_OBJECT, object);
        e.mem.set_u32(object + 0x80, 0x25);
        assert_eq!(e.call(0x0070_6830, &[]).u32(), 5);
        e.mem.set_u32(object + 0x80, 0x10);
        assert_eq!(e.call(0x0070_6830, &[]).u32(), 0xf0);
    }

    /// Doubles for `LoadGame`: the reads deliver `data` in order. Returns
    /// the stream (whose version byte is `version`) and the log of the
    /// float setters.
    fn load_engine(version: u32, data: Vec<Vec<u8>>) -> (Engine, u32, u32) {
        let (mut e, manager) = ui_engine();
        let root = e.mem.alloc(8);
        stub(&mut e, GET_MENUS_ROOT, root);
        let queue = Rc::new(RefCell::new(std::collections::VecDeque::from(data)));
        e.register_double(LOAD_BUFFER_READ, move |e, a| {
            let next = queue.borrow_mut().pop_front().expect("unexpected read");
            assert_eq!(a[2] as usize, next.len());
            e.mem.write(a[1], &next);
            ret(0)
        });
        stub(&mut e, TILE_SET_VALUE, 0);
        stub(&mut e, STATS_PAGE_SET, 0);
        stub(&mut e, MAP_FLOAT_SET_A, 0);
        stub(&mut e, MAP_FLOAT_SET_B, 0);
        stub_memset(&mut e);
        stub(&mut e, SUBOBJECT_SET_STRING, 0);
        stub(&mut e, LOAD_BUFFER_LOAD_STRING, 0);
        stub(&mut e, 0x0200_0000, version);
        e.put_vtable(0x0200_1000, &[0x0200_0000]);
        let stream = e.mem.alloc(8);
        e.mem.set_u32(stream, 0x0200_1000);
        (e, manager, stream)
    }

    fn load_data(tail: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        let mut data = vec![
            300u32.to_le_bytes().to_vec(),
            vec![0x33],
            vec![5],
            vec![7],
            1.5f32.to_le_bytes().to_vec(),
            2.5f32.to_le_bytes().to_vec(),
        ];
        data.extend(tail);
        data
    }

    #[test]
    fn load_game_applies_the_state_and_reads_the_text_from_version_12() {
        let data = load_data(vec![5u32.to_le_bytes().to_vec(), b"world".to_vec()]);
        let (mut e, manager, stream) = load_engine(12, data);
        let seen = Rc::new(RefCell::new(vec![]));
        let sink = seen.clone();
        e.register_double(SUBOBJECT_SET_STRING, move |e, a| {
            sink.borrow_mut().push((a[0], e.mem.cstr(a[1])));
            ret(0)
        });
        e.call(0x0070_6860, &args![stream]);
        let root = logged(&e, GET_MENUS_ROOT);
        assert_eq!(root.len(), 1);
        let set = &logged(&e, TILE_SET_VALUE)[0];
        assert_eq!(set[1..], [0x1771, 300]);
        assert_eq!(logged(&e, STATS_PAGE_SET), vec![vec![0x33]]);
        assert_eq!(logged(&e, MAP_FLOAT_SET_A), vec![vec![1.5f32.to_bits()]]);
        assert_eq!(logged(&e, MAP_FLOAT_SET_B), vec![vec![2.5f32.to_bits()]]);
        assert_eq!(*seen.borrow(), vec![(manager + 0x4d4, b"world".to_vec())]);
        assert!(logged(&e, LOAD_BUFFER_LOAD_STRING).is_empty());
    }

    #[test]
    fn load_game_before_version_12_reads_a_string_and_drops_it() {
        let (mut e, _, stream) = load_engine(11, load_data(vec![]));
        e.call(0x0070_6860, &args![stream]);
        assert_eq!(logged(&e, LOAD_BUFFER_LOAD_STRING).len(), 1);
        assert_eq!(logged(&e, LOAD_BUFFER_LOAD_STRING)[0][0], stream);
        assert!(logged(&e, SUBOBJECT_SET_STRING).is_empty());
        assert!(logged(&e, MEMSET).is_empty());
    }

    #[test]
    fn load_game_hands_the_bytes_to_the_state_object_and_trait_object() {
        let (mut e, _, stream) = load_engine(11, load_data(vec![]));
        let state = e.mem.alloc(0x100);
        e.mem.set_u32(STATE_OBJECT, state);
        e.put_vtable(0x0200_4000, &[0, 0, 0, 0x0200_0100]);
        e.mem.set_u32(state, 0x0200_4000);
        stub(&mut e, 0x0200_0100, 0);
        let trait_object = e.mem.alloc(0x100);
        e.mem.set_u32(TRAIT_OBJECT, trait_object);
        e.mem.set_u32(trait_object + 0x5c, 0x4444);
        e.mem.set_u32(TRAIT_OBJECT_TRAIT_ID, 0x55);
        stub(&mut e, HELPER_4A8F20, 0x66);
        e.call(0x0070_6860, &args![stream]);
        // The third saved byte (7) goes to the virtual slot, the second (5)
        // to the trait object.
        assert_eq!(logged(&e, 0x0200_0100), vec![vec![state, 0x27, 0]]);
        assert_eq!(logged(&e, HELPER_4A8F20), vec![vec![5, 4]]);
        assert_eq!(logged(&e, TILE_SET_VALUE)[1], vec![0x4444, 0x55, 0x66]);
    }

    #[test]
    fn fn_007069d0_calls_the_state_object() {
        let (mut e, _) = ui_engine();
        e.call(0x0070_69d0, &args![0x41u32]);
        let state = e.mem.alloc(0x100);
        e.mem.set_u32(STATE_OBJECT, state);
        e.put_vtable(0x0200_4000, &[0, 0, 0, 0x0200_0100]);
        e.mem.set_u32(state, 0x0200_4000);
        stub(&mut e, 0x0200_0100, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_69d0, &args![0x41u32]);
        assert_eq!(logged(&e, 0x0200_0100), vec![vec![state, 0x61, 0]]);
    }

    #[test]
    fn fn_007069f0_passes_the_value_plus_0x20_and_zero_to_slot_c() {
        let (mut e, _) = ui_engine();
        let object = e.mem.alloc(8);
        e.put_vtable(0x0200_4000, &[0, 0, 0, 0x0200_0100]);
        e.mem.set_u32(object, 0x0200_4000);
        stub(&mut e, 0x0200_0100, 0);
        e.call(0x0070_69f0, &args![object, 5u32]);
        assert_eq!(logged(&e, 0x0200_0100), vec![vec![object, 0x25, 0]]);
    }

    #[test]
    fn fn_00706a20_zero_extends_the_byte() {
        let (mut e, _) = ui_engine();
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(TRAIT_OBJECT, object);
        e.register(HELPER_4A8F20, |_, a| ret(a[0]));
        stub(&mut e, TILE_SET_VALUE, 0);
        e.call(0x0070_6a20, &args![0x1ffu32]);
        assert_eq!(logged(&e, HELPER_4A8F20), vec![vec![0xff, 4]]);
    }

    #[test]
    fn fn_00706a40_sets_the_trait_and_stores_the_value() {
        let (mut e, _) = ui_engine();
        stub(&mut e, TILE_SET_VALUE, 0);
        e.register(HELPER_4A8F20, |_, a| ret(a[0] + 100));
        e.call(0x0070_6a40, &args![5u32]);
        assert!(logged(&e, TILE_SET_VALUE).is_empty());

        let object = e.mem.alloc(0x100);
        e.mem.set_u32(TRAIT_OBJECT, object);
        e.mem.set_u32(object + 0x5c, 0x4444);
        e.mem.set_u32(TRAIT_OBJECT_TRAIT_ID, 0x55);
        e.call(0x0070_6a40, &args![5u32]);
        assert_eq!(logged(&e, TILE_SET_VALUE), vec![vec![0x4444, 0x55, 105]]);
        assert_eq!(e.mem.u32(object + 0x84), 105);
    }

    #[test]
    fn finish_load_game_runs_the_hooks_in_order() {
        let (mut e, _) = ui_engine();
        let hooks = [
            0x007d_d710,
            0x007d_f4e0,
            0x007d_f5d0,
            0x0079_f640,
            0x007a_1490,
            0x007e_5890,
        ];
        for hook in hooks {
            stub(&mut e, hook, 0);
        }
        let state = e.mem.alloc(0x100);
        e.mem.set_u32(state + 0x90, 7);
        e.mem.set_u32(STATE_OBJECT, state);
        e.call(0x0070_6a90, &[]);
        assert_eq!(order(&e), [&[0x0070_6a90][..], &hooks].concat());
        assert_eq!(e.mem.u32(state + 0x90), 0);
    }

    #[test]
    fn fn_00706ac0_clears_the_word_when_there_is_an_object() {
        let (mut e, _) = ui_engine();
        e.call(0x0070_6ac0, &[]);
        let state = e.mem.alloc(0x100);
        e.mem.set_u32(state + 0x90, 7);
        e.mem.set_u32(STATE_OBJECT, state);
        e.call(0x0070_6ac0, &[]);
        assert_eq!(e.mem.u32(state + 0x90), 0);
    }

    /// Doubles for every callee of `00706ae0`; `value_hook` is the answer of
    /// `007050d0`.
    fn finish_engine(value_hook: bool) -> (Engine, u32) {
        let (mut e, manager) = ui_engine();
        for hook in [
            0x0070_53b0,
            0x0077_3050,
            0x0070_5190,
            0x0095_3060,
            0x0083_e510,
            0x0070_5780,
            0x007f_6920,
            0x007a_a480,
            0x007d_da60,
            0x0079_8bf0,
            0x0070_6fd0,
            ENTER_RENDERED_MENU,
        ] {
            stub(&mut e, hook, 0);
        }
        stub(&mut e, 0x0070_50d0, value_hook as u32);
        stub(&mut e, 0x0070_5190, 0x99);
        let root = e.mem.alloc(16);
        stub(&mut e, GET_MENUS_ROOT, root);
        stub(&mut e, FIRST_WORD, 0);
        (e, manager)
    }

    #[test]
    fn fn_00706ae0_resets_the_interface_with_the_value_hook() {
        let (mut e, manager) = finish_engine(true);
        e.mem.set_u32(FINISH_OBJECT, 0x1234);
        e.mem.set_u32(manager + 0x4bc, 5);
        e.mem.set_u32(manager + 0x4d4, 0xffff);
        e.call(0x0070_6ae0, &[]);
        assert_eq!(
            order(&e),
            vec![
                0x0070_6ae0,
                0x0070_53b0,
                0x0077_3050,
                0x0070_50d0,
                0x0070_5190,
                0x0095_3060,
                0x0083_e510,
                0x0070_5780,
                0x007f_6920,
                GET_MENUS_ROOT,
                FIRST_WORD,
                0x007a_a480,
                0x007d_da60,
                0x0079_8bf0,
                0x0070_6fd0,
                ENTER_RENDERED_MENU,
            ]
        );
        assert_eq!(logged(&e, 0x0070_5190), vec![vec![0, 1]]);
        assert_eq!(logged(&e, 0x0095_3060), vec![vec![0x1234, 0x99]]);
        assert_eq!(logged(&e, 0x0070_6fd0), vec![vec![1, 1]]);
        assert_eq!(logged(&e, ENTER_RENDERED_MENU), vec![vec![manager, 0]]);
        assert_eq!(e.mem.u8(FINISH_FLAG), 1);
        assert_eq!(e.mem.u8(CLOSING_SUPPORT_MENUS), 0);
        assert_eq!(e.mem.u32(manager + 0x4bc), 0);
        assert_eq!(e.mem.u32(manager + 0x4d4), 0xfc);
        assert_eq!(e.mem.u32(manager + 0x4d4 + 0xa4), 0x29);
    }

    #[test]
    fn fn_00706ae0_skips_the_value_hook_when_it_says_no() {
        let (mut e, _) = finish_engine(false);
        e.call(0x0070_6ae0, &[]);
        assert!(logged(&e, 0x0070_5190).is_empty());
        assert!(logged(&e, 0x0095_3060).is_empty());
        assert!(logged(&e, 0x0083_e510).is_empty());
        assert_eq!(logged(&e, 0x0070_5780).len(), 1);
    }

    #[test]
    fn fn_00706b90_clears_bits_and_resets_the_counters() {
        let (mut e, _) = ui_engine();
        let table = e.mem.alloc(0xb0);
        for index in 0..0x2au32 {
            e.mem.set_u32(table + 4 * index, 0xffff_ffff);
        }
        e.mem.set_u32(table + 0xa8, 7);
        e.call(0x0070_6b90, &args![table]);
        for index in 0..0x29u32 {
            assert_eq!(e.mem.u32(table + 4 * index), 0xfc);
        }
        assert_eq!(e.mem.u32(table + 0xa4), 0x29);
        assert_eq!(e.mem.u32(table + 0xa8), 0);
    }

    #[test]
    fn tile_is_being_deleted_clears_the_matching_slots() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0097_ae90, 0x5555);
        stub(&mut e, 0x0071_3de0, 0);
        let tile = 0x4444;
        e.mem.set_u32(manager + 0xcc, tile);
        e.mem.set_u32(manager + 0xd0, 1);
        e.mem.set_u32(manager + 0xd4, tile);
        e.mem.set_u32(manager + 0xd8, 2);
        e.mem.set_u32(manager + 0xbc, 3);
        e.call(0x0070_6c20, &args![tile]);
        for offset in [0xcc, 0xd0, 0xd4, 0xd8] {
            assert_eq!(e.mem.u32(manager + offset), 0);
        }
        assert_eq!(e.mem.u32(manager + 0xbc), 3);
        assert_eq!(logged(&e, 0x0071_3de0), vec![vec![manager, tile]]);
    }

    #[test]
    fn tile_is_being_deleted_leaves_other_slots_and_clears_the_current_one() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0097_ae90, 0x4444);
        stub(&mut e, 0x0071_3de0, 0);
        e.mem.set_u32(manager + 0xcc, 0x1111);
        e.mem.set_u32(manager + 0xd0, 1);
        e.mem.set_u32(manager + 0xd4, 0x2222);
        e.mem.set_u32(manager + 0xd8, 2);
        e.mem.set_u32(manager + 0xbc, 3);
        e.call(0x0070_6c20, &args![0x4444u32]);
        assert_eq!(e.mem.u32(manager + 0xcc), 0x1111);
        assert_eq!(e.mem.u32(manager + 0xd0), 1);
        assert_eq!(e.mem.u32(manager + 0xd4), 0x2222);
        assert_eq!(e.mem.u32(manager + 0xd8), 2);
        assert_eq!(e.mem.u32(manager + 0xbc), 0);
    }

    #[test]
    fn fn_00706cb0_clears_the_word_at_bc() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u32(manager + 0xbc, 9);
        e.call(0x0070_6cb0, &args![manager]);
        assert_eq!(e.mem.u32(manager + 0xbc), 0);
    }

    // ---- 00706cd0 .. 00707870 ----

    fn float_ret(value: f64) -> Ret {
        Ret {
            st0: value,
            ..Ret::default()
        }
    }

    /// An engine as `ui_engine` with the pages the text and model code reads
    /// mapped too.
    fn text_engine() -> (Engine, u32) {
        let (mut e, manager) = ui_engine();
        for page in [
            0x0106_e000,
            0x0101_d000,
            0x0119_b000,
            0x011c_3000,
            0x011c_8000,
            0x011d_5000,
        ] {
            e.map(page, 0x1000);
        }
        (e, manager)
    }

    /// A zero-terminated string in fresh memory.
    fn put_text(e: &mut Engine, text: &str) -> u32 {
        let at = e.mem.alloc(text.len() as u32 + 1);
        for (i, byte) in text.bytes().enumerate() {
            e.mem.set_u8(at + i as u32, byte);
        }
        e.mem.set_u8(at + text.len() as u32, 0);
        at
    }

    /// The string at `at`.
    fn text_at(e: &Engine, at: u32) -> String {
        let mut out = String::new();
        let mut i = 0;
        while e.mem.u8(at + i) != 0 {
            out.push(e.mem.u8(at + i) as char);
            i += 1;
        }
        out
    }

    /// A getter that passes the manager to `callee` and returns its result.
    fn check_manager_getter(wrapper: u32, callee: u32) {
        let (mut e, manager) = ui_engine();
        stub(&mut e, callee, 0x55);
        assert_eq!(e.call(wrapper, &[]).u32(), 0x55);
        assert_eq!(logged(&e, callee), vec![vec![manager]]);
    }

    #[test]
    fn get_menus_root_asks_the_manager() {
        check_manager_getter(0x0070_6cd0, 0x0058_6150);
    }

    #[test]
    fn fn_00706cf0_asks_the_manager() {
        check_manager_getter(0x0070_6cf0, 0x004f_d400);
    }

    #[test]
    fn get_debug_text_root_asks_the_manager() {
        check_manager_getter(0x0070_6d10, 0x005e_3fc0);
    }

    #[test]
    fn get_interface_root_asks_the_manager() {
        check_manager_getter(0x0070_6d30, 0x004f_b070);
    }

    #[test]
    fn attach_default_alpha_property_attaches_the_managers_property() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0066_29f0, 0x77);
        stub(&mut e, 0x0043_9410, 0);
        e.call(0x0070_6d50, &args![0x1234u32]);
        assert_eq!(logged(&e, 0x0066_29f0), vec![vec![manager]]);
        assert_eq!(logged(&e, 0x0043_9410), vec![vec![0x1234, 0x77]]);
    }

    #[test]
    fn get_first_chance_texture_release_reads_the_managers_byte() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager + 0xec, 1);
        assert_eq!(e.call(0x0070_6d70, &[]).u8(), 1);
        e.mem.set_u8(manager + 0xec, 0);
        assert_eq!(e.call(0x0070_6d70, &[]).u8(), 0);
    }

    #[test]
    fn fn_00706d90_reads_the_byte_at_ec() {
        let (mut e, _) = ui_engine();
        let object = e.mem.alloc(0x100);
        e.mem.set_u8(object + 0xec, 9);
        assert_eq!(e.call(0x0070_6d90, &args![object]).u8(), 9);
    }

    #[test]
    fn fn_00706db0_passes_the_byte_to_the_setter() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0070_3960, 0);
        e.call(0x0070_6db0, &args![0xabu32]);
        assert_eq!(logged(&e, 0x0070_3960), vec![vec![manager, 0xab]]);
    }

    #[test]
    fn fn_00706dd0_stores_only_a_byte_without_the_ready_check() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager, 0);
        e.mem.set_u32(manager + 0xdc, 0x1122_3344);
        e.call(0x0070_6dd0, &args![0x5u32]);
        assert_eq!(e.mem.u32(manager + 0xdc), 0x1122_3305);
    }

    #[test]
    fn screen_height_and_width_return_the_managers_floats() {
        let (mut e, _) = ui_engine();
        e.register(0x0071_5da0, |_, _| float_ret(1080.0));
        e.register(0x0071_5d40, |_, _| float_ret(1920.0));
        assert_eq!(e.call(0x0070_6df0, &[]).f32(), 1080.0);
        assert_eq!(e.call(0x0070_6e00, &[]).f32(), 1920.0);
    }

    #[test]
    fn real_screen_height_is_the_renderers_integer_as_a_float() {
        let (mut e, _) = ui_engine();
        stub(&mut e, 0x004d_c200, 1050);
        assert_eq!(e.call(0x0070_6e20, &[]).f32(), 1050.0);
        assert_eq!(e.call(0x0070_6e10, &[]).f32(), 1050.0);
    }

    #[test]
    fn real_screen_width_is_the_renderers_integer_as_a_float() {
        let (mut e, _) = ui_engine();
        stub(&mut e, 0x004d_c1f0, 1680);
        assert_eq!(e.call(0x0070_6e50, &[]).f32(), 1680.0);
        assert_eq!(e.call(0x0070_6e40, &[]).f32(), 1680.0);
    }

    #[test]
    fn screen_aspect_ratio_reads_the_managers_float() {
        let (mut e, manager) = ui_engine();
        e.mem.set_f32(manager + 0x4d0, 1.75);
        assert_eq!(e.call(0x0070_6e70, &[]).f32(), 1.75);
    }

    #[test]
    fn fn_00706e80_reads_the_constant() {
        let (mut e, _) = text_engine();
        e.set_global(ASPECT_RATIO_CONSTANT, 1.5f32);
        assert_eq!(e.call(0x0070_6e80, &[]).f32(), 1.5);
    }

    #[test]
    fn fn_00706e90_scales_the_constant_by_the_two_counts() {
        let (mut e, _) = text_engine();
        e.set_global(ASPECT_RATIO_CONSTANT, 2.0f32);
        // Two objects with their own tables: the result of slot 0xc8 is
        // the object itself, slot 0x8c gives 400 and slot 0x90 gives 1000.
        let object = e.mem.alloc(8);
        e.mem.set_u32(object, 0x0200_2000);
        e.put_vtable(0x0200_2000, &[0u32; 0x40]);
        e.mem.set_u32(0x0200_2000 + 0xc8, 0x0200_0001);
        e.mem.set_u32(0x0200_2000 + 0x8c, 0x0200_0002);
        e.mem.set_u32(0x0200_2000 + 0x90, 0x0200_0003);
        stub(&mut e, 0x0043_c4b0, object);
        stub(&mut e, 0x0200_0001, object);
        stub(&mut e, 0x0200_0002, 400);
        stub(&mut e, 0x0200_0003, 1000);
        // 2.0 * (1000 / 400).
        assert_eq!(e.call(0x0070_6e90, &[]).f32(), 5.0);
        assert_eq!(logged(&e, 0x0200_0002), vec![vec![object, 0]]);
        assert_eq!(logged(&e, 0x0200_0003), vec![vec![object, 0]]);
    }

    #[test]
    fn fn_00706f20_returns_0x3ec() {
        let (mut e, _) = ui_engine();
        assert_eq!(e.call(0x0070_6f20, &[]).u32(), 0x3ec);
    }

    #[test]
    fn play_menu_sound_forwards_the_sound() {
        let (mut e, _) = ui_engine();
        stub(&mut e, 0x0071_7280, 0);
        e.call(0x0070_6f30, &args![0x42u32]);
        assert_eq!(logged(&e, 0x0071_7280), vec![vec![0x42]]);
    }

    #[test]
    fn new_timer_forwards_the_float() {
        let (mut e, _) = ui_engine();
        stub(&mut e, 0x0071_64c0, 0);
        e.call(0x0070_6f50, &args![3u32, 2.5f32]);
        assert_eq!(logged(&e, 0x0071_64c0), vec![vec![3, 2.5f32.to_bits()]]);
    }

    #[test]
    fn update_all_timers_runs_on_the_manager() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0071_6320, 0);
        e.call(0x0070_6f70, &[]);
        assert_eq!(logged(&e, 0x0071_6320), vec![vec![manager]]);
    }

    #[test]
    fn clear_timer_forwards_the_id_without_the_manager() {
        let (mut e, _) = ui_engine();
        stub(&mut e, 0x0071_65d0, 0);
        e.call(0x0070_6f90, &args![8u32]);
        assert_eq!(logged(&e, 0x0071_65d0), vec![vec![8]]);
    }

    #[test]
    fn add_to_enter_stack_returns_the_managers_result() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0071_4d90, 5);
        assert_eq!(e.call(0x0070_6fb0, &args![3u32]).u32(), 5);
        assert_eq!(logged(&e, 0x0071_4d90), vec![vec![manager, 3]]);
    }

    #[test]
    fn pop_from_enter_stack_passes_the_id_and_the_byte() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0071_4fd0, 1);
        assert_eq!(e.call(0x0070_6fd0, &args![0x3eau32, 1u32]).u32(), 1);
        assert_eq!(logged(&e, 0x0071_4fd0), vec![vec![manager, 0x3ea, 1]]);
    }

    #[test]
    fn set_current_focus_target_adds_the_constant_arguments() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0071_5860, 0);
        e.call(0x0070_6ff0, &args![0x99u32]);
        assert_eq!(logged(&e, 0x0071_5860), vec![vec![manager, 0x99, 0xfc3, 1]]);
    }

    #[test]
    fn recursive_fade_passes_the_tile_and_both_floats() {
        let (mut e, manager) = ui_engine();
        stub(&mut e, 0x0071_2450, 0);
        e.call(0x0070_7010, &args![0x99u32, 0.5f32, 1.0f32]);
        assert_eq!(
            logged(&e, 0x0071_2450),
            vec![vec![manager, 0x99, 0.5f32.to_bits(), 1.0f32.to_bits()]]
        );
    }

    #[test]
    fn manager_slot_words_are_read_and_written() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u32(manager + 0xcc, 11);
        e.mem.set_u32(manager + 0xd0, 22);
        assert_eq!(e.call(0x0070_7040, &[]).u32(), 11);
        assert_eq!(e.call(0x0070_7070, &[]).u32(), 22);
        e.call(0x0070_7050, &args![33u32]);
        e.call(0x0070_7080, &args![44u32]);
        assert_eq!(e.mem.u32(manager + 0xcc), 33);
        assert_eq!(e.mem.u32(manager + 0xd0), 44);
    }

    #[test]
    fn fn_007070a0_asks_the_manager() {
        check_manager_getter(0x0070_70a0, 0x0045_cd60);
    }

    /// Doubles for the lookups of `FindTextReplacementString` that every
    /// test needs: no known index, no form with that editor ID, names that
    /// never equal a label.
    fn find_text_engine() -> Engine {
        let (mut e, _) = text_engine();
        // `00707330`: the settings object, no setting, entries of the table
        // that match nothing.
        stub(&mut e, 0x0040_4a70, 0);
        stub(&mut e, 0x004f_8a30, 0xdead);
        stub(&mut e, 0x0040_3df0, 0);
        stub(&mut e, 0x0046_9880, 1);
        stub(&mut e, 0x0048_3a00, 0);
        stub(&mut e, 0x0040_4dc0, 1);
        e.set_global(TEXT_OWNER, 0x7000u32);
        e
    }

    #[test]
    fn find_text_replacement_string_rejects_empty_input() {
        let mut e = find_text_engine();
        let name = put_text(&mut e, "PCName");
        let empty = put_text(&mut e, "");
        let dest = e.mem.alloc(32);
        for (n, d, s) in [
            (0, dest, 32),
            (empty, dest, 32),
            (name, 0, 32),
            (name, dest, 0),
        ] {
            let words = args![n, d, s, 0u32];
            assert!(!e.call(0x0070_70c0, &words).bool());
        }
        // Only the top-level calls: nothing was looked up.
        assert!(e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .all(|(a, _)| *a == 0x0070_70c0));
    }

    #[test]
    fn find_text_replacement_string_uses_a_known_index() {
        let mut e = find_text_engine();
        // The first table entry equals the setting.
        e.set_global(TEXT_TABLE, 0x5000u32);
        stub(&mut e, 0x004f_8a30, 0x5000);
        stub(&mut e, 0x0070_39b0, 0);
        let name = put_text(&mut e, "Jump");
        let dest = e.mem.alloc(32);
        assert!(e.call(0x0070_70c0, &args![name, dest, 32u32, 1u32]).bool());
        assert_eq!(logged(&e, 0x0070_39b0), vec![vec![0, dest, 1]]);
        assert!(logged(&e, 0x0048_3a00).is_empty());
    }

    #[test]
    fn find_text_replacement_string_formats_a_global_value() {
        let mut e = find_text_engine();
        stub(&mut e, 0x0048_3a00, 0x6000);
        stub(&mut e, 0x0040_1170, 6);
        stub(&mut e, 0x0040_6d00, 0);
        e.register(0x0052_6ac0, |_, _| float_ret(3.75));
        // A float global ('f'), then an integer one.
        stub(&mut e, 0x0052_9ea0, 0x66);
        let name = put_text(&mut e, "Gravity");
        let dest = e.mem.alloc(32);
        assert!(e.call(0x0070_70c0, &args![name, dest, 32u32, 0u32]).bool());
        let mut expected = vec![dest, 32, FLOAT_FORMAT];
        expected.extend_from_slice(&args![3.75f64]);
        assert_eq!(logged(&e, 0x0040_6d00), vec![expected]);

        stub(&mut e, 0x0052_9ea0, 0x73);
        stub(&mut e, 0x00ec_62c0, 3);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0070_70c0, &args![name, dest, 32u32, 0u32]).bool());
        assert_eq!(
            logged(&e, 0x0040_6d00),
            vec![vec![dest, 32, INTEGER_FORMAT, 3]]
        );
    }

    #[test]
    fn find_text_replacement_string_skips_a_form_of_another_type() {
        let mut e = find_text_engine();
        stub(&mut e, 0x0048_3a00, 0x6000);
        stub(&mut e, 0x0040_1170, 5);
        // The game-setting lookup (`007073d0`) runs for real on its helpers.
        stub(&mut e, 0x0040_6d30, 0);
        stub(&mut e, 0x0044_a670, 5);
        let name = put_text(&mut e, "Thing");
        let dest = e.mem.alloc(32);
        // Falls through to the game-setting lookup, which finds a setting
        // (`004f8a30`) whose type letter is unknown: found, nothing written.
        assert!(e.call(0x0070_70c0, &args![name, dest, 32u32, 0u32]).bool());
        assert!(logged(&e, 0x0040_6d00).is_empty());
        assert_eq!(logged(&e, 0x004f_8a30).len(), 2);
    }

    #[test]
    fn find_text_replacement_string_copies_the_player_texts() {
        let mut e = find_text_engine();
        stub(&mut e, 0x0040_6d30, 0);
        stub(&mut e, 0x0055_d520, 0x111);
        stub(&mut e, 0x0087_f6c0, 0x222);
        stub(&mut e, 0x0087_f4c0, 2);
        e.set_global(0x0119_b354u32 + 8, 0xa000u32);
        e.set_global(0x0119_b364u32 + 8, 0xb000u32);
        e.set_global(0x0119_b35cu32 + 8, 0xc000u32);
        e.register(0x0040_3df0, |_, a| Ret {
            eax: a[0] + 1,
            ..Ret::default()
        });
        // `00404dc0` is 0 only for the label the test is checking.
        let dest = e.mem.alloc(32);
        let name = put_text(&mut e, "X");
        for (label, source) in [
            (PC_NAME_TEXT, 0x111),
            (PC_RACE_TEXT, 0x222),
            (PC_SEX_TEXT, 0xa001),
            (PC_SEX_PRONOUN_TEXT, 0xb001),
            (PC_SEX_POSSESSIVE_TEXT, 0xc001),
        ] {
            e.register_double(0x0040_4dc0, move |_, a| Ret {
                eax: (a[1] != label) as u32,
                ..Ret::default()
            });
            e.call_log = Some(vec![]);
            assert!(e.call(0x0070_70c0, &args![name, dest, 32u32, 0u32]).bool());
            assert_eq!(logged(&e, 0x0040_6d30), vec![vec![dest, 32, source]]);
        }
    }

    #[test]
    fn find_text_replacement_string_asks_the_owner_for_the_player_name() {
        let mut e = find_text_engine();
        stub(&mut e, 0x0040_6d30, 0);
        stub(&mut e, 0x0055_d520, 0x111);
        e.register_double(0x0040_4dc0, |_, a| Ret {
            eax: (a[1] != PC_NAME_TEXT) as u32,
            ..Ret::default()
        });
        let name = put_text(&mut e, "PCName");
        let dest = e.mem.alloc(32);
        assert!(e.call(0x0070_70c0, &args![name, dest, 32u32, 0u32]).bool());
        assert_eq!(logged(&e, 0x0055_d520), vec![vec![0x7000]]);
    }

    #[test]
    fn find_text_replacement_string_falls_back_to_the_game_setting_lookup() {
        let mut e = find_text_engine();
        let name = put_text(&mut e, "sTest");
        let dest = e.mem.alloc(32);
        for i in 0..0x1c {
            e.set_global(TEXT_TABLE + 4 * i, 0x5000 + i);
        }
        stub(&mut e, 0x004f_8a30, 0);
        stub(&mut e, 0x0040_6d30, 0);
        stub(&mut e, 0x0044_a670, 5);
        stub(&mut e, 0x0040_6d50, 0);
        stub(&mut e, 0x0044_f560, 0);
        stub(&mut e, 0x005e_02b0, 0);
        assert!(!e.call(0x0070_70c0, &args![name, dest, 32u32, 0u32]).bool());
    }

    #[test]
    fn match_text_replace_constant_finds_an_equal_setting_or_a_matching_text() {
        let (mut e, _) = text_engine();
        stub(&mut e, 0x0040_4a70, 0x300);
        e.register(0x004f_8a30, |_, a| Ret {
            eax: if a[1] == 0x77 { 0x5004 } else { 0 },
            ..Ret::default()
        });
        for i in 0..0x1c {
            e.set_global(TEXT_TABLE + 4 * i, 0x5000 + i);
        }
        // Found by the setting: entry 4 is `0x5004`.
        assert_eq!(e.call(0x0070_7330, &args![0x77u32]).i32(), 4);
        // Not found: the texts are compared; entry 6's text matches.
        e.register(0x0046_9880, |_, a| Ret {
            eax: (a[0] != 0x5006) as u32,
            ..Ret::default()
        });
        e.register(0x0040_3df0, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        assert_eq!(e.call(0x0070_7330, &args![0x78u32]).i32(), 6);
        // Nothing matches.
        stub(&mut e, 0x0046_9880, 1);
        assert_eq!(e.call(0x0070_7330, &args![0x78u32]).i32(), 0x1c);
    }

    /// Doubles for `007073d0`'s helpers: the copy and length functions work
    /// on real memory so the buffer is the game's.
    fn setting_engine() -> Engine {
        let (mut e, _) = text_engine();
        e.register(0x0040_6d30, |e, a| {
            let mut i = 0;
            loop {
                let byte = e.mem.u8(a[2] + i);
                e.mem.set_u8(a[0] + i, byte);
                if byte == 0 {
                    break;
                }
                i += 1;
            }
            Ret::default()
        });
        e.register(0x0044_a670, |e, a| {
            let mut i = 0;
            while e.mem.u8(a[0] + i) != 0 {
                i += 1;
            }
            Ret {
                eax: i,
                ..Ret::default()
            }
        });
        e.register(0x0040_46f0, |e, a| {
            let mut i = 0;
            loop {
                let byte = e.mem.u8(a[1] + i);
                e.mem.set_u8(a[0] + i, byte);
                if byte == 0 {
                    break;
                }
                i += 1;
            }
            Ret::default()
        });
        stub(&mut e, 0x0040_4a70, 0);
        stub(&mut e, 0x0040_6d50, 0);
        stub(&mut e, 0x0044_f560, 0);
        e
    }

    #[test]
    fn test_constant_for_game_settings_rejects_empty_names() {
        let mut e = setting_engine();
        let empty = put_text(&mut e, "");
        let dest = e.mem.alloc(32);
        assert!(!e.call(0x0070_73d0, &args![0u32, dest]).bool());
        assert!(!e.call(0x0070_73d0, &args![empty, dest]).bool());
    }

    #[test]
    fn test_constant_for_game_settings_writes_each_type() {
        let mut e = setting_engine();
        let setting = e.mem.alloc(16);
        let value_slot = e.mem.alloc(8);
        stub(&mut e, 0x004f_8a30, setting);
        let text = put_text(&mut e, "hello");
        e.mem.set_u32(value_slot, 42);
        stub(&mut e, 0x0040_3df0, text);
        stub(&mut e, 0x0043_d4d0, value_slot);
        stub(&mut e, 0x0040_3e20, value_slot);
        stub(&mut e, 0x0040_8d60, value_slot);
        e.register(0x00ec_623a, |e, a| {
            // Formats the first value as decimal text.
            let digits = format!("{}", a[2]);
            for (i, byte) in digits.bytes().enumerate() {
                e.mem.set_u8(a[0] + i as u32, byte);
            }
            e.mem.set_u8(a[0] + digits.len() as u32, 0);
            Ret::default()
        });
        let dest = e.mem.alloc(0x100);

        // `s`: the setting's text, after skipping `&-` and cutting `;`.
        let name = put_text(&mut e, "&-sName;");
        assert!(e.call(0x0070_73d0, &args![name, dest]).bool());
        assert_eq!(text_at(&e, dest), "hello");
        // `i`: formatted integer.
        let name = put_text(&mut e, "iCount");
        assert!(e.call(0x0070_73d0, &args![name, dest]).bool());
        assert_eq!(text_at(&e, dest), "42");
        // `f`: the float goes to the formatter as a double (and, as in the
        // exe, with the integer format).
        e.mem.set_f32(value_slot, 1.0);
        let name = put_text(&mut e, "fScale");
        assert!(e.call(0x0070_73d0, &args![name, dest]).bool());
        let formats = logged(&e, 0x00ec_623a);
        assert_eq!(formats.len(), 2);
        assert_eq!(formats[1][1..], [INTEGER_FORMAT, 0, 0x3ff0_0000]);
        // `b`: true when the byte is 1, false otherwise.
        e.mem.set_u32(value_slot, 1);
        let name = put_text(&mut e, "bFlag");
        let copy_log = |e: &mut Engine| {
            e.call_log = Some(vec![]);
            assert!(e.call(0x0070_73d0, &args![name, dest]).bool());
            logged(e, 0x0040_46f0)
        };
        assert_eq!(copy_log(&mut e), vec![vec![dest, TRUE_TEXT]]);
        e.mem.set_u32(value_slot, 0);
        assert_eq!(copy_log(&mut e), vec![vec![dest, FALSE_TEXT]]);
        // Any other letter: found, nothing written.
        let name = put_text(&mut e, "xOther");
        assert!(e.call(0x0070_73d0, &args![name, dest]).bool());
    }

    #[test]
    fn test_constant_for_game_settings_retries_with_the_language_suffix() {
        let mut e = setting_engine();
        let setting = e.mem.alloc(16);
        let text = put_text(&mut e, "translated");
        stub(&mut e, 0x0040_3df0, text);
        stub(&mut e, 0x004f_8a30, 0);
        stub(&mut e, 0x005e_02b0, setting);
        let dest = e.mem.alloc(0x100);
        let name = put_text(&mut e, "sGreeting");
        assert!(e.call(0x0070_73d0, &args![name, dest]).bool());
        assert_eq!(text_at(&e, dest), "translated");
        assert_eq!(logged(&e, 0x0040_6d50).len(), 1);
        assert_eq!(logged(&e, 0x0040_6d50)[0][2], LANGUAGE_SUFFIX);

        // Missing in both lookups: false, nothing written.
        stub(&mut e, 0x005e_02b0, 0);
        e.mem.set_u8(dest, b'z');
        assert!(!e.call(0x0070_73d0, &args![name, dest]).bool());
        assert_eq!(e.mem.u8(dest), b'z');
    }

    #[test]
    fn release_model_file_needs_the_loader() {
        let (mut e, _) = text_engine();
        stub(&mut e, 0x0045_a5e0, 0);
        e.call(0x0070_7640, &args![0x44u32]);
        assert!(logged(&e, 0x0045_a5e0).is_empty());
        e.set_global(MODEL_LOADER, 0x9000u32);
        e.call(0x0070_7640, &args![0x44u32]);
        assert_eq!(logged(&e, 0x0045_a5e0), vec![vec![0x9000, 0x44]]);
    }

    #[test]
    fn load_model_file_uses_the_loader_when_it_can() {
        let (mut e, _) = text_engine();
        e.set_global(MODEL_LOADER, 0x9000u32);
        stub(&mut e, 0x0044_7080, 0xaaa);
        let flag = e.mem.alloc(4);
        e.mem.set_u8(flag, 7);
        assert_eq!(e.call(0x0070_7660, &args![0x44u32, flag]).u32(), 0xaaa);
        assert_eq!(e.mem.u8(flag), 1);
        assert_eq!(
            logged(&e, 0x0044_7080),
            vec![vec![0x9000, 0x44, 0, 1, 0, 0, 0]]
        );
    }

    #[test]
    fn load_model_file_builds_a_temporary_model_from_a_stream() {
        let (mut e, _) = text_engine();
        let flag = e.mem.alloc(4);
        stub(&mut e, 0x0045_6a20, 1);
        stub(&mut e, 0x0043_cfd0, 0);
        stub(&mut e, 0x00c3_a8a0, 1);
        stub(&mut e, 0x0043_d090, 0);
        stub(&mut e, 0x0044_31f0, 0);
        stub(&mut e, OPERATOR_NEW, 0xb00);
        stub(&mut e, 0x0043_aaf0, 0xb00);
        stub(&mut e, 0x0043_b230, 0xc0de);
        // The old temporary model is destroyed first.
        e.set_global(TEMP_MODEL, 0x1111u32);
        e.mem.set_u8(flag, 5);
        assert_eq!(e.call(0x0070_7660, &args![0x44u32, flag]).u32(), 0xc0de);
        assert_eq!(e.mem.u8(flag), 0);
        assert_eq!(e.global::<u32>(TEMP_MODEL), 0xb00);
        assert_eq!(logged(&e, 0x0044_31f0), vec![vec![0x1111, 1]]);
        assert_eq!(logged(&e, 0x0045_6a20), vec![vec![0x44, 0, 0, u32::MAX]]);
        let build = &logged(&e, 0x0043_aaf0)[0];
        assert_eq!(build[0], 0xb00);
        assert_eq!(build[1], 0x44);
        assert_eq!((build[3], build[4]), (1, 0));
        assert_eq!(logged(&e, 0x0043_d090).len(), 1);
    }

    #[test]
    fn load_model_file_stops_without_the_file_or_when_the_stream_fails() {
        let (mut e, _) = text_engine();
        let flag = e.mem.alloc(4);
        // The loader exists but the byte forces the slow path; no file.
        e.set_global(MODEL_LOADER, 0x9000u32);
        e.mem.set_u8(LOADER_DISABLED, 1);
        stub(&mut e, 0x0045_6a20, 0);
        assert_eq!(e.call(0x0070_7660, &args![0x44u32, flag]).u32(), 0);
        assert_eq!(e.mem.u8(flag), 0);

        // The file exists but the stream cannot load it.
        stub(&mut e, 0x0045_6a20, 1);
        stub(&mut e, 0x0043_cfd0, 0);
        stub(&mut e, 0x00c3_a8a0, 0);
        stub(&mut e, 0x0043_d090, 0);
        e.set_global(TEMP_MODEL, 0x1111u32);
        assert_eq!(e.call(0x0070_7660, &args![0x44u32, flag]).u32(), 0);
        assert_eq!(e.global::<u32>(TEMP_MODEL), 0x1111);
        assert_eq!(logged(&e, 0x0043_d090).len(), 1);
    }

    #[test]
    fn clear_temp_model_destroys_the_model_and_clears_the_pointer() {
        let (mut e, _) = text_engine();
        stub(&mut e, 0x0044_31f0, 0);
        e.call(0x0070_7820, &[]);
        assert!(logged(&e, 0x0044_31f0).is_empty());
        e.set_global(TEMP_MODEL, 0x1111u32);
        e.call(0x0070_7820, &[]);
        assert_eq!(logged(&e, 0x0044_31f0), vec![vec![0x1111, 1]]);
        assert_eq!(e.global::<u32>(TEMP_MODEL), 0);
    }

    #[test]
    fn fn_00707860_returns_the_meshes_string_address() {
        let (mut e, _) = ui_engine();
        assert_eq!(e.call(0x0070_7860, &[]).u32(), 0x0101_dccc);
    }

    #[test]
    fn copy_or_deep_copy_node_picks_the_copy_by_node_kind() {
        let (mut e, _) = text_engine();
        stub(&mut e, 0x004a_d050, 0);
        stub(&mut e, 0x004a_d270, 0);
        stub(&mut e, 0x0045_7ba0, 0xdee9);
        stub(&mut e, 0x00a5_d2c0, 0xc10e);
        e.set_global(DEEP_COPY_OWNER, 0x9100u32);
        stub(&mut e, 0x004b_5bf0, 1);
        assert_eq!(e.call(0x0070_7870, &args![0x55u32]).u32(), 0xdee9);
        let deep = logged(&e, 0x0045_7ba0);
        assert_eq!(deep[0][0], 0x9100);
        assert_eq!(deep[0][1], 0x55);
        let process = deep[0][2];
        assert_eq!(
            logged(&e, 0x004a_d050),
            vec![vec![process, 1.0f32.to_bits()]]
        );
        assert_eq!(logged(&e, 0x004a_d270), vec![vec![process]]);

        stub(&mut e, 0x004b_5bf0, 0);
        assert_eq!(e.call(0x0070_7870, &args![0x55u32]).u32(), 0xc10e);
        assert_eq!(logged(&e, 0x00a5_d2c0)[0][0], 0x55);
    }
}
