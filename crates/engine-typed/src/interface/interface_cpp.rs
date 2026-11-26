//! The interface unit from its start to `00705a90` (the translation is engine-fallout-interface's
//! `interface.rs`, which these functions replace at the same addresses): the static `Interface::`
//! wrappers over the interface manager, the Pip-Boy's menus, the console, the message boxes and the
//! HUD forwards.
//!
//! **Native.** The exe's order of work, its by-address calls of functions no family types, and
//! its virtual calls. What the exe calls but are plain field reads and stores is done here with
//! no call ([`in_place`]): the manager singleton (`004b7210`) and its ready byte (`009373f0`), the
//! menu by class (`00a09030`), the top and entries of the enter stack (`00714f00`, `00714f70`),
//! the manager's getters and setters, a menu's state word, a tile's node, the list walks' step
//! (`0057cbe0`), the console's visible byte, the Pip-Boy's query, a setting's text, the empty
//! `BSString` and the like. The exe's back-to-back repeated reads of the same word with nothing
//! between are read once; a word read again after a call is read again. The family's functions
//! call one another through `typed_call!`, also where the translation went by address; the tile's
//! integer setter and float getter (`00700320`, `00a011b0`) and `_ftol2` (`00ec62c0`) are the
//! dispatch and C runtime families'.

use crate::{crt, dispatch};
use engine_core::prelude::*;
use engine_core::typed_call;

/// Functions the exe calls that are plain reads and stores of memory, done here with no call. The
/// differential tests leave these addresses out of the translation's calls.
pub mod in_place {
    use engine_core::Engine;

    /// `InterfaceManager::GetSingleton` (`004b7210`): the word at `011d8a80`.
    pub const SINGLETON: u32 = 0x011d_8a80;
    /// The manager's fields the getters and setters read and write.
    pub const MENU_MODE: u32 = 0x0c;
    pub const MOUSE_MOVE_MODE: u32 = 0x18;
    pub const DEBUG_TEXT_VISIBLE: u32 = 0x7c;
    pub const MENUS_ROOT: u32 = 0x9c;
    pub const NODE_A: u32 = 0xa4;
    pub const FULL_HELP: u32 = 0xdd;
    pub const MESSAGE_RESULT_BYTE: u32 = 0xe4;
    pub const PICK_REF: u32 = 0xf0;
    pub const ACTIVATE_REF: u32 = 0xf8;
    pub const TARGET_REF: u32 = 0xfc;
    pub const TELEKINESIS_REF: u32 = 0x100;
    pub const CROSSHAIR_POINT: u32 = 0x104;
    pub const ENTER_STACK: u32 = 0x114;
    pub const ENTER_STACK_SLOTS: u32 = 10;
    pub const PIPBOY: u32 = 0x174;
    pub const PIPBOY_FIELD: u32 = 0x178;
    pub const FIELD_1DC: u32 = 0x1dc;
    /// `NODE_B` is the word at `+0x84` of the manager (`004fd3c0`).
    pub const NODE_B: u32 = 0x84;
    /// The menus by class (`00a09030`): a `BSSimpleArray`'s data pointer and its count (a `u16`),
    /// one entry a menu id from `0x3e9`.
    pub const MENU_TABLE: u32 = 0x011f_350c;
    pub const MENU_COUNT: u32 = 0x011f_3512;
    pub const FIRST_MENU_ID: i32 = 0x3e9;
    pub const LAST_MENU_ID: i32 = 0x43c;
    /// A menu's state word (`0059bb30` reads it, `007037c0` writes it): `1` faded in, `4` hidden.
    pub const MENU_STATE: u32 = 0x24;
    /// A tile's node (`0056c7f0`: the first word of the tile's `+0x2c`).
    pub const TILE_NODE: u32 = 0x2c;
    /// The controls object (`00877720` answers `007fdf30`'s word at `011f35cc`, whatever its `this`).
    pub const CONTROLS: u32 = 0x011f_35cc;
    /// The console's visible byte (`004a4020`: signed, above 0) and the byte `005b6f70` answers.
    pub const CONSOLE_VISIBLE: u32 = 0x38;
    pub const CONSOLE_ACCEPTS_TEXT: u32 = 0x011f_158c;
    /// The Pip-Boy's byte `00974d90` answers.
    pub const PIPBOY_QUERY: u32 = 0x28;
    /// The HUD main menu (`011d96c0`) and its word `007732b0` answers.
    pub const HUD_MAIN_MENU: u32 = 0x011d_96c0;
    pub const HUD_WORD: u32 = 0x21c;
    /// A dialog menu's word `004fb070` answers.
    pub const DIALOG_WORD: u32 = 0x80;

    /// The interface manager (`004b7210`).
    #[inline(always)]
    pub fn manager(e: &Engine) -> u32 {
        e.mem.u32(SINGLETON)
    }

    /// The wrappers' guard: the manager when it exists and its `bFirstInit` byte (`009373f0`) is
    /// set. The exe asks `004b7210` twice with nothing between; read once.
    #[inline(always)]
    pub fn ready_manager(e: &Engine) -> Option<u32> {
        let manager = manager(e);
        (manager != 0 && e.mem.u8(manager) != 0).then_some(manager)
    }

    /// `Tile::GetMenuByClass` (`00a09030`, Xbox PDB): the menu of class `id`, 0 outside
    /// `0x3e9..=0x43c` (signed) or past the table's count.
    #[inline]
    pub fn menu_by_class(e: &Engine, id: i32) -> u32 {
        if !(FIRST_MENU_ID..=LAST_MENU_ID).contains(&id) {
            return 0;
        }
        let index = id.wrapping_sub(FIRST_MENU_ID) as u32;
        if u32::from(e.mem.u16(MENU_COUNT)) <= index {
            return 0;
        }
        e.mem.u32(e.mem.u32(MENU_TABLE).wrapping_add(index * 4))
    }

    /// `InterfaceManager::GetEnterStack` (`00714f70`, Xbox PDB): the enter stack's entry `index`.
    #[inline(always)]
    pub fn enter_stack(e: &Engine, manager: u32, index: u32) -> u32 {
        e.mem.u32(
            manager
                .wrapping_add(ENTER_STACK)
                .wrapping_add(index.wrapping_mul(4)),
        )
    }

    /// `InterfaceManager::GetEnterStackTop` (`00714f00`, Xbox PDB): the last of the stack's
    /// leading nonzero entries (the tenth when all ten are set), 0 for an empty stack.
    #[inline]
    pub fn enter_stack_top(e: &Engine, manager: u32) -> u32 {
        let mut depth = 0;
        while depth < ENTER_STACK_SLOTS && enter_stack(e, manager, depth) != 0 {
            depth += 1;
        }
        if depth == 0 {
            0
        } else {
            enter_stack(e, manager, depth - 1)
        }
    }

    /// The menus root tile (`00586150` on the manager).
    #[inline(always)]
    pub fn menus_root(e: &Engine, manager: u32) -> u32 {
        e.mem.u32(manager.wrapping_add(MENUS_ROOT))
    }

    /// The Pip-Boy object (`00704370` on the manager).
    #[inline(always)]
    pub fn pipboy(e: &Engine, manager: u32) -> u32 {
        e.mem.u32(manager.wrapping_add(PIPBOY))
    }

    /// A menu's state word (`0059bb30`).
    #[inline(always)]
    pub fn menu_state(e: &Engine, menu: u32) -> u32 {
        e.mem.u32(menu.wrapping_add(MENU_STATE))
    }

    /// A tile's node (`0056c7f0`: `00559450` on the tile's `+0x2c`, never null here).
    #[inline(always)]
    pub fn tile_node(e: &Engine, tile: u32) -> u32 {
        e.mem.u32(tile.wrapping_add(TILE_NODE))
    }

    /// The controls object (`00877720` on the global at `011dea0c`, which it doesn't read).
    #[inline(always)]
    pub fn controls(e: &Engine) -> u32 {
        e.mem.u32(CONTROLS)
    }

    /// `004a4020` on the console: its byte at `+0x38`, signed, is above 0.
    #[inline(always)]
    pub fn console_is_visible(e: &Engine, console: u32) -> bool {
        e.mem.i8(console.wrapping_add(CONSOLE_VISIBLE)) > 0
    }

    /// `00403df0`: a setting's text (its `+4`), 0 for a null setting.
    #[inline(always)]
    pub fn setting_text(e: &Engine, setting: u32) -> u32 {
        if setting == 0 {
            0
        } else {
            e.mem.u32(setting.wrapping_add(4))
        }
    }

    /// `BSString`'s constructor (`004037b0`: `00403970`, then the length and capacity set to 0 by
    /// `004039c0` and `00403a00`): the 8 bytes of an empty string.
    #[inline(always)]
    pub fn empty_string(e: &Engine, string: u32) {
        e.mem.set_u32(string, 0);
        e.mem.set_u16(string.wrapping_add(4), 0);
        e.mem.set_u16(string.wrapping_add(6), 0);
    }

    /// Every address done in place in this file and in `interface_cpp_p3`: what the differential
    /// tests leave out of the translation's calls.
    pub const ADDRESSES: &[u32] = &[
        0x004b_7210, // the manager
        0x0093_73f0, // its ready byte
        0x00a0_9030, // the menu by class
        0x0071_4f00, // the enter stack's top
        0x0071_4f70, // an enter stack entry
        0x0058_6150, // the menus root
        0x0070_4370, // the Pip-Boy
        0x0046_0fb0, // the manager's +0x178
        0x0060_2170, // the manager's +0x1dc
        0x0064_4640, // the debug text byte
        0x0087_4480, // the target reference
        0x009b_4440, // the telekinesis reference
        0x009b_7010, // the crosshair point's copy
        0x0071_4d70, // the pick reference stored
        0x0098_4f60, // the manager's +0x18 stored
        0x009b_df60, // the activate reference stored
        0x005e_3fc0, // node A
        0x004f_d3c0, // node B
        0x0059_bb30, // a menu's state
        0x0070_37c0, // a menu's state stored
        0x0056_c7f0, // a tile's node
        0x0055_9450, // the word at +0 (a list's head, a string's characters)
        0x0072_6070, // the word at +4 (a menu's tile)
        0x0084_e3a0, // the word at +0xc (a form's id)
        0x0057_cbe0, // the list walks' step
        0x0087_7720, // the controls object
        0x007f_df30, // (its body)
        0x004a_4020, // the console's visible byte
        0x005b_6f70, // the console's accepts-text byte
        0x0097_4d90, // the Pip-Boy's query
        0x0040_3df0, // a setting's text
        0x0043_d4d0, // a setting's value (constant objects)
        0x0040_3e20, // a float setting's value (constant objects)
        0x0040_37b0, // BSString's constructor
        0x0040_3970, // (its body)
        0x0040_39c0, // (its length)
        0x0040_3a00, // (its capacity)
        0x0040_3d30, // memset wrapper (the message pump's zeroed message)
        0x00ec_61c0, // (memset)
        0x0041_8900, // the subtitle value's copy
        0x0048_3710, // its empty destructor
        0x0077_32b0, // the HUD's word
        0x004f_b070, // a dialog menu's word
        0x004e_a970, // the rendering system object
        0x004d_e070, // the renderer flag byte
        0x004d_e0c0, // the renderer width
        0x004d_e0d0, // the renderer height
        0x00a7_1b70, // NiFrustum's constructor
        0x005b_d5b0, // 007031e0's unused line number: not read at all
        0x005b_d5c0, // (the same)
        0x005b_d580, // (the same)
    ];
}

use in_place::{manager, menu_by_class, ready_manager};

/// `Tile::GetMenu` (Xbox PDB): the menu a tile belongs to.
const TILE_GET_MENU: u32 = 0x00a0_3c90;
/// The menu id on top of the stack that means "one of the five menus in
/// [`TOP_MENU_CANDIDATES`]".
const TOP_PLACEHOLDER: i32 = 1;
/// The menu ids that stand in for [`TOP_PLACEHOLDER`], in the order the code
/// tries them.
const TOP_MENU_CANDIDATES: [i32; 5] = [0x3eb, 0x3ea, 0x40b, 0x425, 0x3ff];
/// Byte table indexed by menu id (`011f308f + id`).
const MENU_FLAG_TABLE: u32 = 0x011f_308f;
/// `MoviePlayer::GetPlayingSequence` (Xbox PDB), on the movie player whose
/// pointer is at `0126fac4`.
const MOVIE_PLAYER_PLAYING: u32 = 0x00ec_17c0;
/// The movie player pointer.
const MOVIE_PLAYER: u32 = 0x0126_fac4;

/// `Tile::GetChildByID` (Xbox PDB).
const TILE_GET_CHILD_BY_ID: u32 = 0x00a0_3eb0;
/// Trait id whose value (110.0, 111.0 or 112.0) tells the isolate code what
/// part a tile plays (`0xfaa`).
const TRAIT_ROLE: i32 = 0xfaa;
/// Trait id read as "enabled" (`0xfa3`).
const TRAIT_ENABLED: i32 = 0xfa3;
/// Trait id written as "visible" (`0x1779`).
const TRAIT_VISIBLE: i32 = 0x1779;
/// `110.0`, `111.0`, `112.0` and `0.0` (`double`s the compares read).
const DOUBLE_110: u32 = 0x0106_ebc8;
const DOUBLE_111: u32 = 0x0106_ebc0;
const DOUBLE_112: u32 = 0x0106_ebd0;
const DOUBLE_ZERO: u32 = 0x0101_2060;
/// `00450f90`: sets a flag (byte argument) on a node.
const SET_NODE_FLAG: u32 = 0x0045_0f90;
/// `00905820` on the manager's sub-object at `+0x4c4`; argument: the
/// address of a tile pointer.
const SUBOBJECT_ADD_TILE: u32 = 0x0090_5820;
/// `00470470` on that sub-object, no arguments.
const SUBOBJECT_CLEAR: u32 = 0x0047_0470;
/// Offset of that sub-object in the manager.
const MANAGER_SUBOBJECT: u32 = 0x4c4;

/// `InterfaceManager::Initialize` (Xbox PDB), `cdecl`: two words.
const MANAGER_INITIALIZE: u32 = 0x0070_9fd0;
/// Scope guard constructor (`00404eb0`, `thiscall`: 0xd, 1, file, line) and
/// destructor (`00404ee0`); the guard is a 4-byte local.
const SCOPE_GUARD_CTOR: u32 = 0x0040_4eb0;
const SCOPE_GUARD_DTOR: u32 = 0x0040_4ee0;
/// The interface's source file name, for the scope guard's report.
const INTERFACE_SOURCE_NAME: u32 = 0x0106_eb78;

/// Menu ids the visibility functions of this unit switch between.
const MENU_STATS: i32 = 0x3eb;
const MENU_INVENTORY: i32 = 0x3ea;
const MENU_REPAIR: i32 = 0x40b;
const MENU_ITEM_MOD: i32 = 0x425;
const MENU_MAP: i32 = 0x3ff;
/// `Menu::StartFadeOut` / `Menu::StartFadeIn` (Xbox PDB), on a menu.
const MENU_START_FADE_OUT: u32 = 0x00a1_d910;
const MENU_START_FADE_IN: u32 = 0x00a1_db20;
/// State the menus are put in when they are hidden.
const MENU_STATE_HIDDEN: u32 = 4;
/// Trait of the menus root tile that holds the id of the menu the pause
/// tabs last made visible (`0x1771`); the visibility functions write the id
/// they show.
const TRAIT_CURRENT_MENU: i32 = 0x1771;
const PIPBOY_REFRESH: u32 = 0x007f_8a80;
/// Takes the object and a tab number.
const PIPBOY_SELECT_TAB: u32 = 0x007f_a010;
/// `MenuConsole::Instance` (Xbox PDB), `cdecl`: `0` answers the instance
/// if it exists, `1` creates it when needed.
const CONSOLE_INSTANCE: u32 = 0x0071_b160;
/// `MenuConsole::ToggleVisible` (Xbox PDB), on the console.
const CONSOLE_TOGGLE_VISIBLE: u32 = 0x0071_d580;
/// `0071d0a0` on the console: format string pointer and a `va_list`.
const CONSOLE_PRINT: u32 = 0x0071_d0a0;
/// `BSString::dtor` (`004037d0`) of the 8-byte string local.
const STRING_DTOR: u32 = 0x0040_37d0;
/// Message creation of the message menu (`007a8e60`): fifteen words, the
/// seventh a `va_list`.
const MESSAGE_CREATE: u32 = 0x007a_8e60;
/// The byte the message-box loop of `00704010` waits on (`-1` until the
/// callback `00704130` stores the button).
const MESSAGE_RESULT: u32 = 0x0119_f349;
/// `FaderManager::RemoveFader` (Xbox PDB) and the global holding the fader
/// manager.
const REMOVE_FADER: u32 = 0x0070_10e0;
const FADER_MANAGER_GLOBAL: u32 = 0x011d_8804;
/// `StartMenu::Close` (Xbox PDB, `cdecl`), `InterfaceManager::
/// EnterRenderedMenu`, `PopFromEnterStack` and `AddToEnterStack` (Xbox PDB),
/// the last three on the manager.
const START_MENU_CLOSE: u32 = 0x007c_e7a0;
const ENTER_RENDERED_MENU: u32 = 0x0071_81d0;
const POP_FROM_ENTER_STACK: u32 = 0x0071_4fd0;
const ADD_TO_ENTER_STACK: u32 = 0x0071_4d90;
/// `004046f0(buffer, text)` copies a string (`cdecl`); `00438390` on a string local assigns it a
/// character pointer; `BSstristr` (`cdecl`) and `sprintf` (`cdecl`).
const STRING_COPY: u32 = 0x0040_46f0;
const STRING_ASSIGN: u32 = 0x0043_8390;
const STRING_STRISTR: u32 = 0x004b_75a0;
const SPRINTF: u32 = 0x00ec_623a;
/// `Controls::Poll` (Xbox PDB), on the controls object.
const CONTROLS_POLL: u32 = 0x00a2_3010;
/// Imports of the message pump: `PeekMessageA`, `TranslateMessage`,
/// `DispatchMessageA` (by their import slots).
const PEEK_MESSAGE: u32 = 0x00fd_f29c;
const TRANSLATE_MESSAGE: u32 = 0x00fd_f298;
const DISPATCH_MESSAGE: u32 = 0x00fd_f294;
/// The window message the pump fills (`MSG`, 0x1c bytes).
const MESSAGE_SIZE: u32 = 0x1c;
/// `StatsMenu::ToggleHealingMode`, `RepairMenu::ResetMenu`,
/// `ItemModMenu::ResetMenu`, `MapMenu::UpdateMenu`,
/// `InventoryMenu::UpdateInventoryMenu` and `InventoryMenu::SavePageState` (Xbox PDB).
const STATS_TOGGLE_HEALING_MODE: u32 = 0x0070_4390;
const REPAIR_RESET_MENU: u32 = 0x007b_6fb0;
const ITEM_MOD_RESET_MENU: u32 = 0x0078_46a0;
const MAP_MENU_UPDATE: u32 = 0x0079_aba0;
const INVENTORY_UPDATE_MENU: u32 = 0x0078_2a90;
const INVENTORY_SAVE_PAGE_STATE: u32 = 0x0077_ffd0;

/// A tile's trait value (`Tile::GetFloat`, `00a011b0`, the dispatch family's), its float widened.
fn tile_value(e: &mut Engine, tile: u32, id: i32) -> f64 {
    f64::from(typed_call!(
        dispatch::FAMILY,
        0x00a0_11b0,
        dispatch::tile::get_float(e, Ptr::new(tile), id)
    ))
}

/// `Tile::SetInt` (`00700320`, the dispatch family's): the trait set to the integer as a float.
#[inline]
fn set_tile_int(e: &mut Engine, tile: u32, id: i32, value: i32) {
    typed_call!(
        dispatch::FAMILY,
        0x0070_0320,
        dispatch::tile::set_int(e, Ptr::new(tile), id, value)
    );
}

fn set_tile_visible(e: &mut Engine, tile: u32, visible: i32) {
    set_tile_int(e, tile, TRAIT_VISIBLE, visible);
}

/// `true` when the tile's "enabled" trait or its "visible" trait is zero:
/// the value the code gives the tile node's flag.
fn node_flag_for(e: &mut Engine, tile: u32) -> bool {
    let zero: f64 = e.global(DOUBLE_ZERO);
    tile_value(e, tile, TRAIT_ENABLED) == zero || tile_value(e, tile, TRAIT_VISIBLE) == zero
}

/// Sets the flag of the tile's node (`00450f90` on its node).
fn set_tile_node_flag(e: &mut Engine, tile: u32, flag: bool) {
    let node = in_place::tile_node(e, tile);
    e.call(SET_NODE_FLAG, &args![node, flag]);
}

/// The walk the isolate and restore code does over a tile list from its head: each step
/// (`0057cbe0`) answers the node's item (its `+8`) and moves on to the node's first word.
fn for_each_list_item(e: &mut Engine, first: u32, mut body: impl FnMut(&mut Engine, u32)) {
    let mut node = first;
    while node != 0 {
        let item = node.wrapping_add(8);
        node = e.mem.u32(node);
        let value = e.mem.u32(item);
        body(e, value);
    }
}

// Typed from 00702250
/// `Interface::Init` (Xbox PDB): inside a scope guard (line 0xb0 of
/// the interface's file), initializes the interface manager with `(1, flag)`.
pub fn interface_init(e: &mut Engine, flag: u8) {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, 0xdu32, 1u32, INTERFACE_SOURCE_NAME, 0xb0u32],
        );
        e.call(MANAGER_INITIALIZE, &args![1u32, flag]);
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
    });
}

// Typed from 007022c0
/// As [`interface_init`] with line 0xbd and the arguments `(2, 1)`.
pub fn fn_007022c0(e: &mut Engine) {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, 0xdu32, 1u32, INTERFACE_SOURCE_NAME, 0xbdu32],
        );
        e.call(MANAGER_INITIALIZE, &args![2u32, 1u32]);
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
    });
}

// Typed from 00702330
/// When the manager is ready: calls `0070a0b0` on it.
pub fn fn_00702330(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0070_a0b0, &args![manager]);
    }
}

// Typed from 00702360
/// `Interface::IsInMenuMode` (Xbox PDB): the manager is ready and its menu
/// mode is not 1.
pub fn interface_is_in_menu_mode(e: &mut Engine) -> bool {
    match ready_manager(e) {
        Some(manager) => typed_call!(super::FAMILY, 0x0070_23a0, fn_007023a0(e, manager)),
        None => false,
    }
}

// Typed from 007023a0
/// `cMenuMode` of the manager (`+0x0c`) is not 1.
pub fn fn_007023a0(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(this.wrapping_add(in_place::MENU_MODE)) != 1
}

// Typed from 007023c0
/// `Interface::GetTopMenuID` (Xbox PDB): the enter stack's top, 0 when the manager is not ready.
pub fn interface_get_top_menu_id(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => in_place::enter_stack_top(e, manager),
        None => 0,
    }
}

// Typed from 00702400
/// The enter stack's entry `index`, 0 when the manager is not ready.
pub fn fn_00702400(e: &mut Engine, index: u32) -> u32 {
    match ready_manager(e) {
        Some(manager) => in_place::enter_stack(e, manager, index),
        None => 0,
    }
}

// Typed from 00702440
/// Returns the constant 10.
pub fn fn_00702440(_e: &mut Engine) -> u32 {
    10
}

// Typed from 00702450
/// `Interface::IsTopMenuID` (Xbox PDB): whether the top of the menu stack
/// is the menu `menu_id`. When the top is the placeholder id 1 and
/// `menu_id` is one of the five ids that stand in for it, the answer is
/// whether that menu is visible instead.
pub fn interface_is_top_menu_id(e: &mut Engine, menu_id: i32) -> u8 {
    let Some(manager) = ready_manager(e) else {
        return 0;
    };
    let top = in_place::enter_stack_top(e, manager) as i32;
    if top == TOP_PLACEHOLDER && TOP_MENU_CANDIDATES.contains(&menu_id) {
        return typed_call!(
            super::FAMILY,
            0x0070_2680,
            interface_is_menu_id_visible(e, menu_id, 0)
        );
    }
    u8::from(top == menu_id)
}

// Typed from 007024e0
/// `Interface::IsTopMenuFadedIn` (Xbox PDB): finds the top menu (for the
/// placeholder id 1, the first visible one of the five stand-in menus) and
/// returns whether its menu's state word is 1.
pub fn interface_is_top_menu_faded_in(e: &mut Engine) -> bool {
    let Some(manager) = ready_manager(e) else {
        return false;
    };
    let top = in_place::enter_stack_top(e, manager) as i32;
    let mut menu = 0;
    if top == TOP_PLACEHOLDER {
        for id in TOP_MENU_CANDIDATES {
            if typed_call!(
                super::FAMILY,
                0x0070_2680,
                interface_is_menu_id_visible(e, id, 0)
            ) != 0
            {
                menu = menu_by_class(e, id);
                break;
            }
        }
    } else {
        menu = menu_by_class(e, top);
    }
    if menu == 0 || e.call(TILE_GET_MENU, &args![menu]).u32() == 0 {
        return false;
    }
    let inner = e.call(TILE_GET_MENU, &args![menu]).u32();
    in_place::menu_state(e, inner) == 1
}

// Typed from 00702640
/// `Interface::GetMenuModeType` (Xbox PDB): the enter stack's first entry, 0 when the manager is
/// not ready.
pub fn interface_get_menu_mode_type(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => in_place::enter_stack(e, manager, 0),
        None => 0,
    }
}

// Typed from 00702680
/// `Interface::IsMenuIDVisible` (Xbox PDB). For the placeholder id 1:
/// `00709c00` must hold and one of the five stand-in menus must be visible
/// (with the same mask). Otherwise, when `mask` is nonzero and the menu
/// exists with a menu tile, the menu's state word must have a bit of
/// `mask`; then the answer is the per-id flag byte ([`fn_007027b0`]).
pub fn interface_is_menu_id_visible(e: &mut Engine, menu_id: i32, mask: u32) -> u8 {
    if ready_manager(e).is_none() {
        return 0;
    }
    if menu_id == TOP_PLACEHOLDER {
        if !typed_call!(
            super::FAMILY,
            0x0070_9c00,
            super::interface_cpp_p3::fn_00709c00(e)
        ) {
            return 0;
        }
        for id in TOP_MENU_CANDIDATES {
            if typed_call!(
                super::FAMILY,
                0x0070_2680,
                interface_is_menu_id_visible(e, id, mask)
            ) != 0
            {
                return 1;
            }
        }
        return 0;
    }
    if mask != 0 {
        let menu = menu_by_class(e, menu_id);
        if menu != 0 && e.call(TILE_GET_MENU, &args![menu]).u32() != 0 {
            let inner = e.call(TILE_GET_MENU, &args![menu]).u32();
            if in_place::menu_state(e, inner) & mask == 0 {
                return 0;
            }
        }
    }
    typed_call!(super::FAMILY, 0x0070_27b0, fn_007027b0(e, menu_id))
}

// Typed from 007027b0
/// The flag byte of menu `menu_id` in the table at `011f308f`, 0 for an id
/// outside `0x3e9..=0x43c`.
pub fn fn_007027b0(e: &mut Engine, menu_id: i32) -> u8 {
    if !(in_place::FIRST_MENU_ID..=in_place::LAST_MENU_ID).contains(&menu_id) {
        return 0;
    }
    e.mem.u8(MENU_FLAG_TABLE.wrapping_add(menu_id as u32))
}

// Typed from 007027e0
/// When the manager is ready: `InterfaceManager::PreIdleStuff` (Xbox PDB).
pub fn fn_007027e0(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0070_b8f0, &args![manager]);
    }
}

// Typed from 00702810
/// When the manager is ready: `InterfaceManager::Idle` (Xbox PDB).
pub fn fn_00702810(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0070_c4a0, &args![manager]);
    }
}

// Typed from 00702840
/// When the manager is ready: calls `00711ea0` on it.
pub fn fn_00702840(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0071_1ea0, &args![manager]);
    }
}

/// `MoviePlayer::GetPlayingSequence` of the global movie player.
fn movie_playing(e: &mut Engine) -> bool {
    let player = e.mem.u32(MOVIE_PLAYER);
    e.call(MOVIE_PLAYER_PLAYING, &args![player]).bool()
}

// Typed from 00702870
/// When the manager is ready and no movie sequence is playing: calls
/// `00713f00` on the manager.
pub fn fn_00702870(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        if !movie_playing(e) {
            e.call(0x0071_3f00, &args![manager]);
        }
    }
}

/// The rendering system object (`004ea970` answers its address).
const RENDERING_SYSTEM: u32 = 0x0120_0088;

// Typed from 007028b0
/// `Interface::Click` (Xbox PDB): when the manager is ready and no movie is
/// playing, hands the click `(x, y)` to `00713fb0`; otherwise sets the
/// render thread stage of both rendering threads to 0x17 (`00ba30f0` /
/// `00ba3130` on the rendering system). Always clears `bClickMultithreaded`
/// (`+0x148`) afterwards.
pub fn interface_click(e: &mut Engine, x: u32, y: u32) {
    let mut handled = false;
    if let Some(manager) = ready_manager(e) {
        if !movie_playing(e) {
            e.call(0x0071_3fb0, &args![manager, x, y]);
            handled = true;
        }
    }
    if !handled {
        e.call(0x00ba_30f0, &args![RENDERING_SYSTEM, 0u32, 0x17u32]);
        e.call(0x00ba_3130, &args![RENDERING_SYSTEM, 1u32, 0x17u32]);
    }
    let manager = manager(e);
    typed_call!(super::FAMILY, 0x0070_2940, fn_00702940(e, manager, 0));
}

// Typed from 00702940
/// Stores the byte `value` in `bClickMultithreaded` (`+0x148`) of the
/// manager.
pub fn fn_00702940(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(this.wrapping_add(0x148), value);
}

// Typed from 00702960
/// When the manager is ready: asks the movie player whether a sequence is
/// playing and drops the answer.
pub fn fn_00702960(e: &mut Engine) {
    if ready_manager(e).is_some() {
        movie_playing(e);
    }
}

// Typed from 00702990
/// Updates the visibility of one menu tile `tile` and of those children
/// whose role trait (`0xfaa`) is 110.0 or 111.0; `flag_a`, `flag_b` and
/// `flag_c` are the bytes `IsolateMenuElements` passes (`flag_c` is always 1
/// there). Does nothing for a null tile. (The exe also reads the menus root, as
/// its walk's unused list object: not read here.)
pub fn fn_00702990(e: &mut Engine, tile: u32, flag_a: u8, flag_b: u8, flag_c: u8) {
    if tile == 0 {
        return;
    }
    let manager = manager(e);
    let role = tile_value(e, tile, TRAIT_ROLE);
    let is_112 = role == e.global::<f64>(DOUBLE_112);
    let visible = i32::from(!((flag_b != 0 && !is_112) || (flag_b == 0 && is_112)));
    set_tile_visible(e, tile, visible);
    let first = e.mem.u32(tile.wrapping_add(4));
    for_each_list_item(e, first, |e, child| {
        let role = tile_value(e, child, TRAIT_ROLE);
        let tile_110 = if role == e.global::<f64>(DOUBLE_110) {
            child
        } else {
            0
        };
        let role = tile_value(e, child, TRAIT_ROLE);
        let tile_111 = if role == e.global::<f64>(DOUBLE_111) {
            child
        } else {
            0
        };
        if tile_110 != 0 {
            if flag_a == 0 {
                set_tile_visible(e, tile_110, 0);
            } else {
                set_tile_visible(e, tile_110, 1);
                if flag_c != 0 && flag_b == 0 && !is_112 {
                    e.with_stack(4, |e, slot| {
                        e.mem.set_u32(slot.addr(), tile_110);
                        e.call(
                            SUBOBJECT_ADD_TILE,
                            &args![manager.wrapping_add(MANAGER_SUBOBJECT), slot],
                        );
                    });
                }
            }
        }
        if tile_111 != 0 {
            set_tile_visible(e, tile_111, i32::from(flag_a == 0));
        }
        if tile_110 != 0 && in_place::tile_node(e, tile_110) != 0 {
            let flag = node_flag_for(e, tile_110);
            set_tile_node_flag(e, tile_110, flag);
        }
        if tile_111 != 0 && in_place::tile_node(e, tile_111) != 0 {
            let flag = flag_a != 0 || node_flag_for(e, tile_111);
            set_tile_node_flag(e, tile_111, flag);
        }
    });
    if in_place::tile_node(e, tile) != 0 {
        let flag = node_flag_for(e, tile);
        set_tile_node_flag(e, tile, flag);
    }
}

// Typed from 00702c80
/// `Interface::IsolateMenuElements` (Xbox PDB): when the manager is ready,
/// applies [`fn_00702990`] with `(tile, a, b, 1)` to every menu tile of the
/// menus root (when `a` is set and `b` is not, first clears the manager's
/// sub-object at `+0x4c4`), then sets the flag of the manager's two nodes
/// (`+0xa4`, `+0x84`) to 0 when both `a` and `b` are zero, else 1.
pub fn interface_isolate_menu_elements(e: &mut Engine, a: u8, b: u8) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    if a != 0 && b == 0 {
        e.call(
            SUBOBJECT_CLEAR,
            &args![manager.wrapping_add(MANAGER_SUBOBJECT)],
        );
    }
    let menus_root = in_place::menus_root(e, manager);
    let first = e.mem.u32(menus_root.wrapping_add(4));
    for_each_list_item(e, first, |e, menu| {
        typed_call!(super::FAMILY, 0x0070_2990, fn_00702990(e, menu, a, b, 1));
    });
    let flag = !(a == 0 && b == 0);
    for field in [in_place::NODE_A, in_place::NODE_B] {
        let node = e.mem.u32(manager.wrapping_add(field));
        if node != 0 {
            e.call(SET_NODE_FLAG, &args![node, flag]);
        }
    }
}

// Typed from 00702dd0
/// `Interface::RestoreMenuElements` (Xbox PDB): for every menu tile of the
/// menus root, makes it and its children `0x6e` and `0x6f` visible and
/// recomputes their node flags.
pub fn interface_restore_menu_elements(e: &mut Engine) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let menus_root = in_place::menus_root(e, manager);
    let first = e.mem.u32(menus_root.wrapping_add(4));
    for_each_list_item(e, first, |e, menu| {
        let child_6e = e.call(TILE_GET_CHILD_BY_ID, &args![menu, 0x6eu32]).u32();
        let child_6f = e.call(TILE_GET_CHILD_BY_ID, &args![menu, 0x6fu32]).u32();
        set_tile_visible(e, menu, 1);
        if child_6e != 0 {
            set_tile_visible(e, child_6e, 1);
        }
        if child_6f != 0 {
            set_tile_visible(e, child_6f, 1);
        }
        for tile in [menu, child_6e, child_6f] {
            if tile != 0 && in_place::tile_node(e, tile) != 0 {
                let flag = node_flag_for(e, tile);
                set_tile_node_flag(e, tile, flag);
            }
        }
    });
}

// Typed from 00702fc0
/// `Interface::IsDebugTextVisible` (Xbox PDB): `bDebugTextVisible` (`+0x7c`, `00644640`) when the
/// manager is ready.
pub fn interface_is_debug_text_visible(e: &mut Engine) -> bool {
    match ready_manager(e) {
        Some(manager) => e.mem.u8(manager.wrapping_add(in_place::DEBUG_TEXT_VISIBLE)) != 0,
        None => false,
    }
}

// Typed from 00703000
/// `Interface::ToggleDebugTextVisible` (Xbox PDB): when the manager is
/// ready, [`fn_00703030`] on it.
pub fn interface_toggle_debug_text_visible(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        typed_call!(super::FAMILY, 0x0070_3030, fn_00703030(e, manager));
    }
}

// Typed from 00703030
/// Flips `bDebugTextVisible` (`+0x7c`) through [`fn_00703060`].
pub fn fn_00703030(e: &mut Engine, this: u32) {
    let visible = e.mem.u8(this.wrapping_add(in_place::DEBUG_TEXT_VISIBLE)) != 0;
    typed_call!(
        super::FAMILY,
        0x0070_3060,
        fn_00703060(e, this, u8::from(!visible))
    );
}

// Typed from 00703060
/// Stores the byte `value` in `bDebugTextVisible` (`+0x7c`).
pub fn fn_00703060(e: &mut Engine, this: u32, value: u8) {
    e.mem
        .set_u8(this.wrapping_add(in_place::DEBUG_TEXT_VISIBLE), value);
}

// Typed from 00703080
/// When the manager is ready: `007151b0(manager, a, b)` with two floats.
pub fn fn_00703080(e: &mut Engine, a: f32, b: f32) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0071_51b0, &args![manager, a, b]);
    }
}

// Typed from 007030c0
/// When the manager is ready: `00715e00(manager, a, b)` with two words.
pub fn fn_007030c0(e: &mut Engine, a: u32, b: u32) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0071_5e00, &args![manager, a, b]);
    }
}

// Typed from 00703100
/// `Interface::ToggleFullHelp` (Xbox PDB): flips `bFullHelp` (`+0xdd`).
pub fn interface_toggle_full_help(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        let at = manager.wrapping_add(in_place::FULL_HELP);
        let full = e.mem.u8(at) != 0;
        e.mem.set_u8(at, u8::from(!full));
    }
}

// Typed from 00703150
/// `Interface::GetFullHelp` (Xbox PDB): `bFullHelp` (`+0xdd`), 0 when the
/// manager is not ready.
pub fn interface_get_full_help(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => e.mem.u8(manager.wrapping_add(in_place::FULL_HELP)),
        None => 0,
    }
}

// Typed from 00703180
/// `Interface::GetTargetREFR` (Xbox PDB): `pPickRef` through
/// [`fn_007031c0`], 0 when the manager is not ready.
pub fn interface_get_target_refr(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => typed_call!(super::FAMILY, 0x0070_31c0, fn_007031c0(e, manager)),
        None => 0,
    }
}

// Typed from 007031c0
/// `pPickRef` (`+0xf0`) of the manager.
pub fn fn_007031c0(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(in_place::PICK_REF))
}

/// The setting whose integer is the debug text's line (`0043d4d0` on `011debac`: its `+4`).
const DEBUG_TEXT_Y: u32 = 0x011d_ebb0;

// Typed from 007031e0
/// When the manager is ready: for a non-null reference prints
/// `"<name>" (<form id>)` as debug text (`DebugText::Print`), then stores the
/// reference as the manager's `pPickRef` (`00714d70`) in every case. The text
/// goes at x = 640.0 (`0103a1c4`), y = the integer the setting at `011debac`
/// holds, with the arguments `2, -1, -1.0, 0, 0`.
///
/// The exe also computes a line number from a chain of getters (`0043d4d0`,
/// `005bd5b0`, `005bd5c0`, `005bd580`, `_ftol2`) that it never uses: reads and
/// a conversion with no effect, left out here.
pub fn fn_007031e0(e: &mut Engine, reference: u32) {
    if ready_manager(e).is_none() {
        return;
    }
    e.with_stack(8, |e, text| {
        in_place::empty_string(e, text.addr());
        if reference != 0 {
            let form_id = e.mem.u32(reference.wrapping_add(0xc));
            let name = e.call(0x0055_d520, &args![reference]).u32();
            e.call(0x0040_6f60, &args![text, 0x0103_a1c8u32, name, form_id]);
            let y = e.mem.i32(DEBUG_TEXT_Y);
            let characters = e.mem.u32(text.addr());
            let x: f32 = e.global(0x0103_a1c4);
            let minus_one: f32 = e.global(0x0101_2054);
            let debug_text = e.call(0x00a0_d9e0, &args![1u32]).u32();
            e.call(
                0x00a0_f8b0,
                &args![debug_text, characters, x, y as f32, 2u32, -1i32, minus_one, 0u32, 0u32],
            );
        }
        // The exe asks 004b7210 for the manager again after the four calls.
        let manager = in_place::manager(e);
        e.mem
            .set_u32(manager.wrapping_add(in_place::PICK_REF), reference);
        e.call(STRING_DTOR, &args![text]);
    });
}

// Typed from 00703310
/// `Interface::SetPickREFR` (Xbox PDB): the reference stored at the manager's `+0xf8`
/// (`009bdf60`) when the manager is ready.
pub fn interface_set_pick_refr(e: &mut Engine, reference: u32) {
    if let Some(manager) = ready_manager(e) {
        e.mem
            .set_u32(manager.wrapping_add(in_place::ACTIVATE_REF), reference);
    }
}

// Typed from 00703350
/// The manager's word at `+0xfc` (`00874480`) when it is ready, else 0.
pub fn fn_00703350(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => e.mem.u32(manager.wrapping_add(in_place::TARGET_REF)),
        None => 0,
    }
}

// Typed from 00703390
/// `Interface::GetTelekinesisREFR` (Xbox PDB): the manager's word at `+0x100` (`009b4440`) when
/// it is ready, else 0.
pub fn interface_get_telekinesis_refr(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => e.mem.u32(manager.wrapping_add(in_place::TELEKINESIS_REF)),
        None => 0,
    }
}

// Typed from 007033d0
/// Writes a 3-float point at `out` and returns `out`: the manager's three words at `+0x104`
/// (`009b7010`) when it is ready, else the three words at `011f426c`.
pub fn fn_007033d0(e: &mut Engine, out: Ptr) -> Ptr {
    let from = match ready_manager(e) {
        Some(manager) => manager.wrapping_add(in_place::CROSSHAIR_POINT),
        None => 0x011f_426c,
    };
    for i in 0..3 {
        let word = e.mem.u32(from.wrapping_add(4 * i));
        e.mem.set_u32(out.addr().wrapping_add(4 * i), word);
    }
    out
}

// Typed from 00703430
/// [`fn_00703470`] on the manager when it is ready, else false.
pub fn fn_00703430(e: &mut Engine) -> bool {
    match ready_manager(e) {
        Some(manager) => typed_call!(super::FAMILY, 0x0070_3470, fn_00703470(e, manager)),
        None => false,
    }
}

// Typed from 00703470
/// `bFuzzyActivatePick` (`+0x110`) of the manager.
pub fn fn_00703470(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this.wrapping_add(0x110)) != 0
}

/// Runs `f` with the address of the given words in memory, the way a
/// `va_list` points at the caller's stack words.
fn with_va_list<R>(e: &mut Engine, words: &[u32], f: impl FnOnce(&mut Engine, u32) -> R) -> R {
    e.with_stack(4 * super::word(words.len().max(1)), |e, block| {
        for (i, word) in words.iter().enumerate() {
            e.mem
                .set_u32(block.addr().wrapping_add(4 * super::word(i)), *word);
        }
        f(e, block.addr())
    })
}

/// The same for the callers that pass `&va_list`: the address of a local
/// word that holds the address of the words.
fn with_va_list_cell<R>(e: &mut Engine, words: &[u32], f: impl FnOnce(&mut Engine, u32) -> R) -> R {
    with_va_list(e, words, |e, list| {
        e.with_stack(4, |e, cell| {
            e.mem.set_u32(cell.addr(), list);
            f(e, cell.addr())
        })
    })
}

/// Starts the fade-out of the menu of `tile` (`Tile::GetMenu`, then
/// `Menu::StartFadeOut`).
fn menu_fade_out(e: &mut Engine, tile: u32) {
    let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
    e.call(MENU_START_FADE_OUT, &args![menu]);
}

/// The same with `Menu::StartFadeIn`.
fn menu_fade_in(e: &mut Engine, tile: u32) {
    let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
    e.call(MENU_START_FADE_IN, &args![menu]);
}

/// Writes the menu id into the menus root's [`TRAIT_CURRENT_MENU`].
fn set_current_menu(e: &mut Engine, manager: u32, menu_id: i32) {
    let root = in_place::menus_root(e, manager);
    set_tile_int(e, root, TRAIT_CURRENT_MENU, menu_id);
}

/// The Pipboy tab refresh the visibility functions end with (the Pip-Boy read again after the
/// refresh, as the exe does).
fn refresh_pipboy_tab(e: &mut Engine, manager: u32, tab: u32) {
    let pipboy = in_place::pipboy(e, manager);
    e.call(PIPBOY_REFRESH, &args![pipboy]);
    let pipboy = in_place::pipboy(e, manager);
    e.call(PIPBOY_SELECT_TAB, &args![pipboy, tab]);
}

/// Calls `callee(0)` when the Pipboy's byte at `+0x28` (`00974d90`) is set.
fn call_if_pipboy_query(e: &mut Engine, manager: u32, callee: u32) {
    let pipboy = in_place::pipboy(e, manager);
    if e.mem.u8(pipboy.wrapping_add(in_place::PIPBOY_QUERY)) != 0 {
        e.call(callee, &args![0u32]);
    }
}

// Typed from 00703490
/// When the manager is ready: `0070bc20` on it.
pub fn fn_00703490(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0070_bc20, &args![manager]);
    }
}

// Typed from 007034c0
/// When the manager is ready: `value` stored at its `+0x18` (`00984f60`).
pub fn fn_007034c0(e: &mut Engine, value: u32) {
    if let Some(manager) = ready_manager(e) {
        e.mem
            .set_u32(manager.wrapping_add(in_place::MOUSE_MOVE_MODE), value);
    }
}

// Typed from 00703500
/// `Interface::ShowMenus` (Xbox PDB): shows the menu the menus root's
/// [`TRAIT_CURRENT_MENU`] names (when it is 0, `00717600` runs on the
/// manager and the stats menu is shown): the stats menu for `0x3eb`, the
/// map for `0x3ff`, the inventory for `0x3ea`, `0x40b` and `0x425`; any
/// other id shows nothing.
pub fn interface_show_menus(e: &mut Engine) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let root = in_place::menus_root(e, manager);
    let current = tile_value(e, root, TRAIT_CURRENT_MENU);
    let mut menu_id = super::low_dword(typed_call!(
        crt::FAMILY,
        0x00ec_62c0,
        crt::math::ftol2_sse(e, current)
    )) as i32;
    if menu_id == 0 {
        // The exe asks 004b7210 for the manager again after the getter and `_ftol2`.
        e.call(0x0071_7600, &args![in_place::manager(e)]);
        menu_id = MENU_STATS;
    }
    match menu_id {
        MENU_INVENTORY | MENU_REPAIR | MENU_ITEM_MOD => {
            typed_call!(
                super::FAMILY,
                0x0070_48f0,
                interface_set_inventory_menu_visible(e, 1, 0, 1)
            )
        }
        MENU_STATS => typed_call!(
            super::FAMILY,
            0x0070_4c10,
            interface_set_stats_menu_visible(e, 1, 0)
        ),
        MENU_MAP => typed_call!(
            super::FAMILY,
            0x0070_4170,
            interface_set_map_menu_visible(e, 1, 0)
        ),
        _ => {}
    }
}

// Typed from 00703610
/// `Interface::HideMenus` (Xbox PDB): for each of the stats, inventory,
/// repair, item-mod and map menus that exist, puts its menu in state 4
/// and sets the "enabled" trait (`0xfa3`) of the menu's tile (its `+4`)
/// to 0. Then [`fn_00704f50`], closes the start menu (`StartMenu::Close(1)`)
/// when `004a4040` says it is open, and
/// `InterfaceManager::EnterRenderedMenu(0)`.
pub fn interface_hide_menus(e: &mut Engine) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let tiles = [
        MENU_STATS,
        MENU_INVENTORY,
        MENU_REPAIR,
        MENU_ITEM_MOD,
        MENU_MAP,
    ]
    .map(|id| menu_by_class(e, id));
    for tile in tiles {
        if tile != 0 {
            let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
            e.mem
                .set_u32(menu.wrapping_add(in_place::MENU_STATE), MENU_STATE_HIDDEN);
            let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
            let menu_tile = e.mem.u32(menu.wrapping_add(4));
            set_tile_int(e, menu_tile, TRAIT_ENABLED, 0);
        }
    }
    typed_call!(super::FAMILY, 0x0070_4f50, fn_00704f50(e));
    if e.call(0x004a_4040, &[]).bool() {
        e.call(START_MENU_CLOSE, &args![1u32]);
    }
    e.call(ENTER_RENDERED_MENU, &args![manager, 0u32]);
}

// Typed from 007037e0
/// `Interface::EmergencyCloseAllMenusAndBreakStuff` (Xbox PDB): the
/// manager's method of the same name, when the manager is ready.
pub fn interface_emergency_close_all_menus_and_break_stuff(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0071_4c60, &args![manager]);
    }
}

// Typed from 00703810
/// When the manager is ready: sets the flag of the node of the menus root
/// tile to `value == 0`.
pub fn fn_00703810(e: &mut Engine, value: u8) {
    if let Some(manager) = ready_manager(e) {
        let root = in_place::menus_root(e, manager);
        set_tile_node_flag(e, root, value == 0);
    }
}

// Typed from 00703860
/// `Interface::SetCrosshairTargetType` (Xbox PDB): when the object at
/// `011dea3c` exists and its slot `0x1d0` answers nonzero,
/// `HUDMainMenu::SetTargetType(target_type, 0)`.
pub fn interface_set_crosshair_target_type(e: &mut Engine, target_type: u32) {
    let object = e.global::<u32>(0x011d_ea3c);
    if object != 0 && e.vcall(object, 0x1d0, &[]).u32() != 0 {
        e.call(0x0077_2d10, &args![target_type, 0u32]);
    }
}

// Typed from 007038a0
/// The word at `011d96b4`.
pub fn fn_007038a0(e: &mut Engine) -> u32 {
    e.global::<u32>(0x011d_96b4)
}

// Typed from 007038b0
/// `Interface::SetEnemyActor` (Xbox PDB): `HUDMainMenu::SetEnemyActor`.
pub fn interface_set_enemy_actor(e: &mut Engine, actor: u32) {
    e.call(0x0077_3190, &args![actor]);
}

// Typed from 007038d0
/// The HUD main menu's word at `+0x21c` (`007732b0`), 0 when there is no HUD main menu.
pub fn fn_007038d0(e: &mut Engine) -> Ptr {
    let hud = e.mem.u32(in_place::HUD_MAIN_MENU);
    if hud == 0 {
        Ptr::new(0)
    } else {
        Ptr::new(e.mem.u32(hud.wrapping_add(in_place::HUD_WORD)))
    }
}

// Typed from 007038e0
/// `Interface::ToggleSafeZone` (Xbox PDB): `InterfaceManager::ToggleSafeZone`
/// with the argument, when the manager is ready.
pub fn interface_toggle_safe_zone(e: &mut Engine, value: u32) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0071_5ec0, &args![manager, value]);
    }
}

// Typed from 00703920
/// `Interface::SetFirstChanceTextureRelease` (Xbox PDB): when the manager
/// is ready, stores 1 in its byte at `+0xec` ([`fn_00703960`]) and runs
/// `Tile::UpdateAll(0)` (`cdecl`).
pub fn interface_set_first_chance_texture_release(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        typed_call!(super::FAMILY, 0x0070_3960, fn_00703960(e, manager, 1));
        e.call(0x00a0_4200, &args![0u32]);
    }
}

// Typed from 00703960
/// Stores the byte `value` at `+0xec` of the manager.
pub fn fn_00703960(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(this.wrapping_add(0xec), value);
}

// Typed from 00703980
/// `InterfaceManager::ForceTextureRelease` (Xbox PDB) on the manager when
/// it is ready.
pub fn fn_00703980(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0071_60b0, &args![manager]);
    }
}

/// The control tables of [`interface_get_control_push_text_for`]: by mode 3, 1, 0 and 2.
const CONTROL_TABLE_3: u32 = 0x011d_5268;
const CONTROL_TABLE_1: u32 = 0x011d_5240;
const CONTROL_TABLE_0: u32 = 0x011d_52f0;
const CONTROL_TABLE_2: u32 = 0x011d_51b0;
/// The settings whose text is the empty push text and the two labels.
const SETTING_NO_TEXT: u32 = 0x011d_37b0;
const SETTING_TRIGGER_LABEL: u32 = 0x011d_4a08;
const SETTING_BUTTON_LABEL: u32 = 0x011d_3fc4;

// Typed from 007039b0
/// `Interface::GetControlPushTextFor` (Xbox PDB), `cdecl`: writes the push
/// text of control `control` (`0..=0x1c`; any other value does nothing)
/// into the character buffer `buffer`.
///
/// The control's setting is chosen from [`fn_00703be0`] on the controls
/// object, by mode: when `004b71d0` answers true mode 3 and the table at
/// `011d5268`; otherwise mode 1 with the table at `011d5240` (index below 9),
/// else mode 0 with `011d52f0` (index below `0xee`), else mode 2 with
/// `011d51b0` (index below 8). Its text (`+4`) is copied to the buffer
/// (`004046f0`). When `with_label` is set the buffer then becomes
/// `"<label> <text>"` (`sprintf`) where the label is the setting at `011d4a08`
/// if the setting's name (`+8`) contains "trigger" (`BSstristr`), else the one
/// at `011d3fc4`, and the text is the buffer itself, or the setting at
/// `011d37b0` when the buffer is empty.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn interface_get_control_push_text_for(
    e: &mut Engine,
    control: i32,
    buffer: u32,
    with_label: u8,
) {
    if !(0..=0x1c).contains(&control) {
        return;
    }
    let mut setting = 0u32;
    e.mem.set_u8(buffer, 0);
    let alternate = e.call(0x004b_71d0, &[]).bool();
    let control_byte = |e: &mut Engine, mode: u32| -> u32 {
        // The exe asks 00877720 for the controls object before each 00703be0.
        let controls = in_place::controls(e);
        u32::from(typed_call!(
            super::FAMILY,
            0x0070_3be0,
            fn_00703be0(e, controls, control as u32, mode)
        ))
    };
    if alternate {
        let index = control_byte(e, 3);
        setting = e.mem.u32(CONTROL_TABLE_3.wrapping_add(4 * index));
    } else {
        let index = control_byte(e, 1);
        if index < 9 {
            setting = e.mem.u32(CONTROL_TABLE_1.wrapping_add(4 * index));
        } else {
            let index = control_byte(e, 0);
            if index < 0xee {
                setting = e.mem.u32(CONTROL_TABLE_0.wrapping_add(4 * index));
            } else {
                let index = control_byte(e, 2);
                if index < 8 {
                    setting = e.mem.u32(CONTROL_TABLE_2.wrapping_add(4 * index));
                }
            }
        }
    }
    let text = in_place::setting_text(e, setting);
    if text != 0 {
        e.call(STRING_COPY, &args![buffer, text]);
    }
    if with_label == 0 {
        return;
    }
    e.with_stack(8, |e, string| {
        in_place::empty_string(e, string.addr());
        let shown = if e.mem.u8(buffer) != 0 {
            buffer
        } else {
            in_place::setting_text(e, SETTING_NO_TEXT)
        };
        e.call(STRING_ASSIGN, &args![string, shown]);
        let name = if setting == 0 {
            0
        } else {
            e.mem.u32(setting.wrapping_add(8))
        };
        let is_trigger =
            name != 0 && e.call(STRING_STRISTR, &args![name, 0x0106_ebd8u32]).u32() != 0;
        let characters = e.mem.u32(string.addr());
        let label = in_place::setting_text(
            e,
            if is_trigger {
                SETTING_TRIGGER_LABEL
            } else {
                SETTING_BUTTON_LABEL
            },
        );
        e.call(SPRINTF, &args![buffer, 0x0101_2058u32, label, characters]);
        e.call(STRING_DTOR, &args![string]);
    });
}

// Typed from 00703be0
/// The byte at `+0x1b94 + mode * 0x1c + control` of the object.
pub fn fn_00703be0(e: &mut Engine, this: u32, control: u32, mode: u32) -> u8 {
    let offset = mode.wrapping_mul(0x1c).wrapping_add(control);
    e.mem.u8(this.wrapping_add(0x1b94).wrapping_add(offset))
}

/// The console exists and is visible (`MenuConsole::Instance(0)`, then
/// `Instance(1)` and its visible byte).
fn console_visible(e: &mut Engine) -> bool {
    if e.call(CONSOLE_INSTANCE, &args![0u32]).u32() == 0 {
        return false;
    }
    let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
    in_place::console_is_visible(e, console)
}

/// Whether the console takes text: it is visible, or the byte `005b6f70` answers says text
/// is accepted anyway.
fn console_takes_text(e: &mut Engine) -> bool {
    console_visible(e) || e.mem.u8(in_place::CONSOLE_ACCEPTS_TEXT) != 0
}

/// Prints a formatted line on the console (`cdecl`, varargs: the format
/// string pointer, then the words the format uses): when the manager is
/// ready and the console takes text, `0071d0a0(console, format, va_list)`.
pub fn print_formatted(e: &mut Engine, format: u32, va: &[u32]) {
    if ready_manager(e).is_none() {
        return;
    }
    if console_takes_text(e) {
        with_va_list(e, va, |e, list| {
            let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
            e.call(CONSOLE_PRINT, &args![console, format, list]);
        });
    }
}

// Typed from 00703c00
/// The registered form of `00703c00`: the format and the variable words.
pub fn fn_00703c00(e: &mut Engine, words: &[u32]) -> Ret {
    assert!(!words.is_empty(), "00703c00 needs the format argument word");
    print_formatted(e, words[0], &words[1..]);
    Ret::default()
}

/// `Interface::PrintLine` (Xbox PDB), `cdecl`, varargs: the first argument
/// is a `BSString` passed by value (its two words, `text_pointer` and
/// `text_length`; the function destroys it) and the variable words follow.
/// Like [`fn_00703c00`] it prints `0071d0a0(console, characters, va_list)`
/// with the string's characters as format.
///
/// The compiler's exception-unwinding frame is not translated.
pub fn print_line(e: &mut Engine, text_pointer: u32, text_length: u32, va: &[u32]) {
    e.with_stack(8, |e, text| {
        e.mem.set_u32(text.addr(), text_pointer);
        e.mem.set_u32(text.addr().wrapping_add(4), text_length);
        if ready_manager(e).is_some() && console_takes_text(e) {
            with_va_list(e, va, |e, list| {
                let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
                e.call(CONSOLE_PRINT, &args![console, text_pointer, list]);
            });
        }
        e.call(STRING_DTOR, &args![text]);
    });
}

// Typed from 00703c80
/// The registered form of `Interface::PrintLine`.
pub fn interface_print_line(e: &mut Engine, words: &[u32]) -> Ret {
    assert!(
        words.len() >= 2,
        "PrintLine needs the two words of its string"
    );
    print_line(e, words[0], words[1], &words[2..]);
    Ret::default()
}

// Typed from 00703d50
/// `Interface::IsConsoleVisible` (Xbox PDB): the console exists and is
/// visible, when the manager is ready.
pub fn interface_is_console_visible(e: &mut Engine) -> bool {
    ready_manager(e).is_some() && console_visible(e)
}

// Typed from 00703da0
/// `Interface::CloseConsole` (Xbox PDB): when the manager is ready and the
/// console exists and is visible, toggles it and pops entry 3 from the
/// enter stack (`InterfaceManager::PopFromEnterStack(3, 0)`).
pub fn interface_close_console(e: &mut Engine) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    if console_visible(e) {
        let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
        e.call(CONSOLE_TOGGLE_VISIBLE, &args![console]);
        e.call(POP_FROM_ENTER_STACK, &args![manager, 3u32, 0u32]);
    }
}

// Typed from 00703e10
/// `Interface::OpenConsole` (Xbox PDB): when the manager is ready and the
/// console exists but is not visible, toggles it and pushes entry 3 on the
/// enter stack (`InterfaceManager::AddToEnterStack(3)`).
pub fn interface_open_console(e: &mut Engine) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    if e.call(CONSOLE_INSTANCE, &args![0u32]).u32() == 0 {
        return;
    }
    let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
    if in_place::console_is_visible(e, console) {
        return;
    }
    let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
    e.call(CONSOLE_TOGGLE_VISIBLE, &args![console]);
    e.call(ADD_TO_ENTER_STACK, &args![manager, 3u32]);
}

/// Creates a message (`cdecl`, varargs): when the manager is ready,
/// `007a8e60(arg0, arg1, arg2, 0, arg3, arg4, arg5, arg8, &va_list, 1, -1,
/// -1, 0.0, arg6, arg7)` and its byte answer; false otherwise. `arg6` and
/// `arg7` are floats.
#[allow(clippy::too_many_arguments)]
pub fn show_message(
    e: &mut Engine,
    arg0: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    arg4: u32,
    arg5: u32,
    arg6: f32,
    arg7: f32,
    arg8: u32,
    va: &[u32],
) -> bool {
    if ready_manager(e).is_none() {
        return false;
    }
    with_va_list_cell(e, va, |e, list| {
        e.call(
            MESSAGE_CREATE,
            &args![
                arg0, arg1, arg2, 0u32, arg3, arg4, arg5, arg8, list, 1u32, -1i32, -1i32, 0.0f32,
                arg6, arg7
            ],
        )
        .bool()
    })
}

// Typed from 00703e80
/// The registered form of `00703e80`: nine fixed words and the variable ones.
pub fn fn_00703e80(e: &mut Engine, w: &[u32]) -> Ret {
    assert!(
        w.len() >= 9,
        "00703e80 needs 9 argument words, got {}",
        w.len()
    );
    let ok = show_message(
        e,
        w[0],
        w[1],
        w[2],
        w[3],
        w[4],
        w[5],
        f32::from_bits(w[6]),
        f32::from_bits(w[7]),
        w[8],
        &w[9..],
    );
    Ret {
        eax: u32::from(ok),
        ..Ret::default()
    }
}

/// The same as [`fn_00703e80`] with ten fixed words: `007a8e60(arg0, arg1,
/// arg2, arg3, arg4, arg5, arg6, arg9, &va_list, 1, -1, -1, 0.0, arg7,
/// arg8)`; `arg7` and `arg8` are floats.
#[allow(clippy::too_many_arguments)]
pub fn show_message_full(
    e: &mut Engine,
    arg0: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    arg4: u32,
    arg5: u32,
    arg6: u32,
    arg7: f32,
    arg8: f32,
    arg9: u32,
    va: &[u32],
) -> bool {
    if ready_manager(e).is_none() {
        return false;
    }
    with_va_list_cell(e, va, |e, list| {
        e.call(
            MESSAGE_CREATE,
            &args![
                arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg9, list, 1u32, -1i32, -1i32, 0.0f32,
                arg7, arg8
            ],
        )
        .bool()
    })
}

// Typed from 00703f10
/// The registered form of `00703f10`: ten fixed words and the variable ones.
pub fn fn_00703f10(e: &mut Engine, w: &[u32]) -> Ret {
    assert!(
        w.len() >= 10,
        "00703f10 needs 10 argument words, got {}",
        w.len()
    );
    let ok = show_message_full(
        e,
        w[0],
        w[1],
        w[2],
        w[3],
        w[4],
        w[5],
        w[6],
        f32::from_bits(w[7]),
        f32::from_bits(w[8]),
        w[9],
        &w[10..],
    );
    Ret {
        eax: u32::from(ok),
        ..Ret::default()
    }
}

// Typed from 00703fa0
/// `Interface::GetMessageMenuResult` (Xbox PDB): the byte at `+0xe4` of the
/// manager, which it resets to `0xff`. (It does not check that the manager
/// is ready.)
pub fn interface_get_message_menu_result(e: &mut Engine) -> u8 {
    let at = manager(e).wrapping_add(in_place::MESSAGE_RESULT_BYTE);
    let result = e.mem.u8(at);
    e.mem.set_u8(at, 0xff);
    result
}

// Typed from 00703fd0
/// `007aa5c0` (the message menu's), no arguments.
pub fn fn_00703fd0(e: &mut Engine) {
    e.call(0x007a_a5c0, &[]);
}

// Typed from 00703fe0
/// `MessageMenu::DisableMessageMenu` (Xbox PDB) with the byte.
pub fn fn_00703fe0(e: &mut Engine, value: u8) {
    e.call(0x007a_a6b0, &args![u32::from(value)]);
}

// Typed from 00704000
/// `Interface::CloseMessageMenu` (Xbox PDB):
/// `MessageMenu::ClearMessageQueueHead`.
pub fn interface_close_message_menu(e: &mut Engine) {
    e.call(0x007a_a530, &[]);
}

/// Shows a message and waits for its answer (`cdecl`, varargs: four fixed
/// words). It sets the result byte (`0119f349`) to `0xff`, removes the
/// fader 1 (`FaderManager::RemoveFader(1, 1)`) and, when the manager is
/// ready, creates the message (`007a8e60(arg0, 0, 0, 0, 00704130, arg1,
/// arg2, arg3, &va_list, 1, -1, -1, 0.0, 0.0, 0.0)`). If that failed it
/// answers 0; otherwise it runs the interface loop (pumps the window
/// messages, `Controls::Poll`, the per-frame interface calls and a click)
/// until [`fn_00704130`] stores the answer, and returns it.
pub fn show_message_and_wait(
    e: &mut Engine,
    arg0: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    va: &[u32],
) -> u8 {
    e.mem.set_u8(MESSAGE_RESULT, 0xff);
    let fader_manager = e.global::<u32>(FADER_MANAGER_GLOBAL);
    e.call(REMOVE_FADER, &args![fader_manager, 1u32, 1u32]);
    let created = ready_manager(e).is_some()
        && with_va_list_cell(e, va, |e, list| {
            e.call(
                MESSAGE_CREATE,
                &args![
                    arg0,
                    0u32,
                    0u32,
                    0u32,
                    0x0070_4130u32,
                    arg1,
                    arg2,
                    arg3,
                    list,
                    1u32,
                    -1i32,
                    -1i32,
                    0.0f32,
                    0.0f32,
                    0.0f32
                ],
            )
            .bool()
        });
    if !created {
        return 0;
    }
    while e.mem.u8(MESSAGE_RESULT) == 0xff {
        e.with_stack(MESSAGE_SIZE, |e, message| {
            // `00403d30(message, 0, 0x1c)`: the message zeroed, word by word.
            for at in (0..MESSAGE_SIZE).step_by(4) {
                e.mem.set_u32(message.addr().wrapping_add(at), 0);
            }
            while e
                .call(PEEK_MESSAGE, &args![message, 0u32, 0u32, 0u32, 1u32])
                .u32()
                != 0
            {
                e.call(TRANSLATE_MESSAGE, &args![message]);
                e.call(DISPATCH_MESSAGE, &args![message]);
            }
        });
        let controls = in_place::controls(e);
        e.call(CONTROLS_POLL, &args![controls]);
        typed_call!(super::FAMILY, 0x0070_27e0, fn_007027e0(e));
        typed_call!(super::FAMILY, 0x0070_2810, fn_00702810(e));
        typed_call!(super::FAMILY, 0x0070_2840, fn_00702840(e));
        typed_call!(super::FAMILY, 0x0070_28b0, interface_click(e, 0, 0));
        typed_call!(super::FAMILY, 0x0070_2960, fn_00702960(e));
    }
    e.mem.u8(MESSAGE_RESULT)
}

// Typed from 00704010
/// The registered form of `00704010`: four fixed words and the variable ones.
pub fn fn_00704010(e: &mut Engine, w: &[u32]) -> Ret {
    assert!(
        w.len() >= 4,
        "00704010 needs 4 argument words, got {}",
        w.len()
    );
    let answer = show_message_and_wait(e, w[0], w[1], w[2], w[3], &w[4..]);
    Ret {
        eax: u32::from(answer),
        ..Ret::default()
    }
}

// Typed from 00704130
/// The message callback of [`fn_00704010`]: stores
/// [`interface_get_message_menu_result`] in the result byte (`0119f349`).
pub fn fn_00704130(e: &mut Engine) {
    let result = typed_call!(
        super::FAMILY,
        0x0070_3fa0,
        interface_get_message_menu_result(e)
    );
    e.mem.set_u8(MESSAGE_RESULT, result);
}

// Typed from 00704140
/// `MapMenu::Create` (`00796b90`) when the manager is ready, else 0.
pub fn fn_00704140(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(_) => e.call(0x0079_6b90, &[]).u32(),
        None => 0,
    }
}

/// The five Pip-Boy menus by class: stats, inventory, repair, item mod, map.
fn pipboy_menus(e: &Engine) -> [u32; 5] {
    [
        MENU_STATS,
        MENU_INVENTORY,
        MENU_REPAIR,
        MENU_ITEM_MOD,
        MENU_MAP,
    ]
    .map(|id| menu_by_class(e, id))
}

/// The "already visible" test of the visibility functions: `check_state` looks at the menu's
/// state bits `0xb` too.
fn menu_visible(e: &mut Engine, menu_id: i32, check_state: u8) -> bool {
    let mask = if check_state != 0 { 0xb } else { 0 };
    typed_call!(
        super::FAMILY,
        0x0070_2680,
        interface_is_menu_id_visible(e, menu_id, mask)
    ) != 0
}

// Typed from 00704170
/// `Interface::SetMapMenuVisible` (Xbox PDB), `cdecl`: shows (`show` set)
/// or hides the map menu. `check_state` makes the "already visible" test
/// look at the menu's state bits `0xb` ([`interface_is_menu_id_visible`]).
///
/// To show it while it is not visible: the stats, repair, item-mod and
/// inventory menus fade out (the first three also reset: `StatsMenu::
/// ToggleHealingMode`, `RepairMenu::ResetMenu`, `ItemModMenu::ResetMenu`),
/// the map menu is created if it does not exist and fades in with its id in
/// the menus root's [`TRAIT_CURRENT_MENU`], then `MapMenu::UpdateMenu`. To
/// hide it while visible: it fades out and its slot `0x14` runs with
/// `(0, 0)`. Showing always ends with the Pipboy refresh (tab 3) and
/// `0079c340(0)` when the Pip-Boy's query says so.
pub fn interface_set_map_menu_visible(e: &mut Engine, show: u8, check_state: u8) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let [stats, inventory, repair, item_mod, mut map] = pipboy_menus(e);
    let visible = menu_visible(e, MENU_MAP, check_state);
    if show != 0 && !visible {
        if stats != 0 {
            menu_fade_out(e, stats);
            e.call(STATS_TOGGLE_HEALING_MODE, &[]);
        }
        if repair != 0 {
            menu_fade_out(e, repair);
            e.call(REPAIR_RESET_MENU, &[]);
        }
        if item_mod != 0 {
            menu_fade_out(e, item_mod);
            e.call(ITEM_MOD_RESET_MENU, &[]);
        }
        if inventory != 0 {
            menu_fade_out(e, inventory);
        }
        if map == 0 {
            map = typed_call!(super::FAMILY, 0x0070_4140, fn_00704140(e));
        }
        if map != 0 {
            set_current_menu(e, manager, MENU_MAP);
            menu_fade_in(e, map);
        }
        e.call(MAP_MENU_UPDATE, &[]);
    } else if show == 0 && visible && map != 0 {
        menu_fade_out(e, map);
        let menu = e.call(TILE_GET_MENU, &args![map]).u32();
        e.vcall(menu, 0x14, &args![0u32, 0u32]);
    }
    if show != 0 {
        refresh_pipboy_tab(e, manager, 3);
        call_if_pipboy_query(e, manager, 0x0079_c340);
    }
}

// Typed from 00704490
/// `Interface::CreateRepairMenu` (Xbox PDB): `RepairMenu::Create` when the
/// manager is ready, else 0.
pub fn interface_create_repair_menu(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(_) => e.call(0x007b_5520, &[]).u32(),
        None => 0,
    }
}

// Typed from 007044c0
/// Shows or hides the repair menu (`cdecl`: `show`, `check_state` as in
/// [`interface_set_map_menu_visible`], `refresh_pipboy`). Showing creates
/// the menu if needed ([`interface_create_repair_menu`]); when it was not
/// visible the stats (with `StatsMenu::ToggleHealingMode`), map and item-mod
/// menus fade out, the repair menu fades in and the inventory fades out.
/// Its id goes into [`TRAIT_CURRENT_MENU`]; with `refresh_pipboy` the Pipboy
/// refresh runs (tab 2); and `00782470(0)` runs when the Pip-Boy's query says so.
/// Hiding fades the menu out when it is visible.
pub fn fn_007044c0(e: &mut Engine, show: u8, check_state: u8, refresh_pipboy: u8) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let [stats, inventory, mut repair, item_mod, map] = pipboy_menus(e);
    let visible = menu_visible(e, MENU_REPAIR, check_state);
    if show != 0 {
        if repair == 0 {
            repair = typed_call!(super::FAMILY, 0x0070_4490, interface_create_repair_menu(e));
        }
        if !visible {
            if stats != 0 {
                menu_fade_out(e, stats);
                e.call(STATS_TOGGLE_HEALING_MODE, &[]);
            }
            if map != 0 {
                menu_fade_out(e, map);
            }
            if item_mod != 0 {
                menu_fade_out(e, item_mod);
            }
            if repair != 0 {
                menu_fade_in(e, repair);
            }
            if inventory != 0 {
                menu_fade_out(e, inventory);
            }
        }
        if repair != 0 {
            set_current_menu(e, manager, MENU_REPAIR);
        }
        if refresh_pipboy != 0 {
            refresh_pipboy_tab(e, manager, 2);
        }
        call_if_pipboy_query(e, manager, 0x0078_2470);
    } else if visible && repair != 0 {
        menu_fade_out(e, repair);
    }
}

// Typed from 00704690
/// `RepairServicesMenu::Create` (`cdecl`) with the argument, when the
/// manager is ready.
pub fn fn_00704690(e: &mut Engine, value: u32) {
    if ready_manager(e).is_some() {
        e.call(0x007b_7570, &args![value]);
    }
}

// Typed from 007046c0
/// `Interface::CreateItemModMenu` (Xbox PDB): `ItemModMenu::Create` when
/// the manager is ready, else 0.
pub fn interface_create_item_mod_menu(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(_) => e.call(0x0078_3440, &[]).u32(),
        None => 0,
    }
}

// Typed from 007046f0
/// Shows or hides the item-mod menu: the same as [`fn_007044c0`] with the
/// item-mod menu (`0x425`, created by [`interface_create_item_mod_menu`]);
/// when it was not visible the stats (with `StatsMenu::ToggleHealingMode`),
/// map and repair menus fade out, the item-mod menu fades in and the
/// inventory fades out.
pub fn fn_007046f0(e: &mut Engine, show: u8, check_state: u8, refresh_pipboy: u8) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let [stats, inventory, repair, mut item_mod, map] = pipboy_menus(e);
    let visible = menu_visible(e, MENU_ITEM_MOD, check_state);
    if show != 0 {
        if item_mod == 0 {
            item_mod = typed_call!(
                super::FAMILY,
                0x0070_46c0,
                interface_create_item_mod_menu(e)
            );
        }
        if !visible {
            if stats != 0 {
                menu_fade_out(e, stats);
                e.call(STATS_TOGGLE_HEALING_MODE, &[]);
            }
            if map != 0 {
                menu_fade_out(e, map);
            }
            if repair != 0 {
                menu_fade_out(e, repair);
            }
            if item_mod != 0 {
                menu_fade_in(e, item_mod);
            }
            if inventory != 0 {
                menu_fade_out(e, inventory);
            }
        }
        if item_mod != 0 {
            set_current_menu(e, manager, MENU_ITEM_MOD);
        }
        if refresh_pipboy != 0 {
            refresh_pipboy_tab(e, manager, 2);
        }
        call_if_pipboy_query(e, manager, 0x0078_2470);
    } else if visible && item_mod != 0 {
        menu_fade_out(e, item_mod);
    }
}

// Typed from 007048c0
/// `InventoryMenu::Create` when the manager is ready, else 0.
pub fn fn_007048c0(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(_) => e.call(0x0077_fc10, &[]).u32(),
        None => 0,
    }
}

// Typed from 007048f0
/// `Interface::SetInventoryMenuVisible` (Xbox PDB), `cdecl`: shows or hides
/// the inventory menu, as [`fn_007044c0`] does the repair menu. Showing
/// creates it if needed ([`fn_007048c0`]); when it was not visible the
/// stats (with `StatsMenu::ToggleHealingMode`), map, repair (with
/// `RepairMenu::ResetMenu`) and item-mod (with `ItemModMenu::ResetMenu`)
/// menus fade out and the inventory fades in; then
/// `InventoryMenu::UpdateInventoryMenu` runs and the id goes into
/// [`TRAIT_CURRENT_MENU`]. Hiding a visible one fades it out and runs
/// `InventoryMenu::SavePageState`.
pub fn interface_set_inventory_menu_visible(
    e: &mut Engine,
    show: u8,
    check_state: u8,
    refresh_pipboy: u8,
) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let [stats, mut inventory, repair, item_mod, map] = pipboy_menus(e);
    let visible = menu_visible(e, MENU_INVENTORY, check_state);
    if show != 0 {
        if inventory == 0 {
            inventory = typed_call!(super::FAMILY, 0x0070_48c0, fn_007048c0(e));
        }
        if !visible {
            if stats != 0 {
                menu_fade_out(e, stats);
                e.call(STATS_TOGGLE_HEALING_MODE, &[]);
            }
            if map != 0 {
                menu_fade_out(e, map);
            }
            if repair != 0 {
                menu_fade_out(e, repair);
                e.call(REPAIR_RESET_MENU, &[]);
            }
            if item_mod != 0 {
                menu_fade_out(e, item_mod);
                e.call(ITEM_MOD_RESET_MENU, &[]);
            }
            if inventory != 0 {
                menu_fade_in(e, inventory);
            }
        }
        if inventory != 0 {
            e.call(INVENTORY_UPDATE_MENU, &[]);
            set_current_menu(e, manager, MENU_INVENTORY);
        }
        if refresh_pipboy != 0 {
            refresh_pipboy_tab(e, manager, 2);
        }
        call_if_pipboy_query(e, manager, 0x0078_2470);
    } else if visible && inventory != 0 {
        menu_fade_out(e, inventory);
        e.call(INVENTORY_SAVE_PAGE_STATE, &[]);
    }
}

// Typed from 00704ad0
/// `Interface::GetInventoryMenuVisible` (Xbox PDB): the flag byte of the
/// inventory menu id ([`fn_007027b0`]).
pub fn interface_get_inventory_menu_visible(e: &mut Engine) -> u8 {
    typed_call!(super::FAMILY, 0x0070_27b0, fn_007027b0(e, MENU_INVENTORY))
}

// Typed from 00704af0
/// When the manager is ready, refreshes whichever menu is showing: the
/// inventory (`InventoryMenu::UpdateInventoryMenu`) if it is visible, else
/// the stats menu (`StatsMenu::UpdateHealingOptions(menu, 4)`) when
/// [`fn_00704df0`] says so, else `00730690(1)` when menu `0x41d` is visible,
/// else `00728a30(1)` when `0x435` is, else [`fn_00704bc0`] when `0x3f0` is.
pub fn fn_00704af0(e: &mut Engine) {
    if ready_manager(e).is_none() {
        return;
    }
    let visible = |e: &mut Engine, id: i32| {
        typed_call!(
            super::FAMILY,
            0x0070_2680,
            interface_is_menu_id_visible(e, id, 0)
        ) != 0
    };
    if typed_call!(
        super::FAMILY,
        0x0070_4ad0,
        interface_get_inventory_menu_visible(e)
    ) != 0
    {
        e.call(INVENTORY_UPDATE_MENU, &[]);
    } else if typed_call!(super::FAMILY, 0x0070_4df0, fn_00704df0(e)) != 0 {
        let stats = menu_by_class(e, MENU_STATS);
        let menu = e.call(TILE_GET_MENU, &args![stats]).u32();
        typed_call!(
            super::FAMILY,
            0x007d_f230,
            super::stats_menu::stats_menu_update_healing_options(e, Ptr::new(menu), 4)
        );
    } else if visible(e, 0x41d) {
        e.call(0x0073_0690, &args![1u32]);
    } else if visible(e, 0x435) {
        e.call(0x0072_8a30, &args![1u32]);
    } else if visible(e, 0x3f0) {
        typed_call!(super::FAMILY, 0x0070_4bc0, fn_00704bc0(e));
    }
}

/// The type descriptors the menu casts pass to `__RTDynamicCast`
/// (`00ec43fb`, `cdecl`): the source type is always `Menu`, the targets
/// `StatsMenu`, `DialogMenu`, `SleepWaitMenu` and `VATSMenu`.
const MENU_TYPE: u32 = 0x0119_f34c;
const STATS_MENU_TYPE: u32 = 0x0119_f37c;
const DIALOG_MENU_TYPE: u32 = 0x0119_f394;
const SLEEP_WAIT_MENU_TYPE: u32 = 0x0119_f3cc;
const VATS_MENU_TYPE: u32 = 0x0119_f3e8;
/// `__RTDynamicCast(object, 0, source type, target type, 0)`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// Menu ids of the menus this part looks up.
const MENU_DIALOG: i32 = 0x3f1;
const MENU_SLEEP_WAIT: i32 = 0x3f4;
const MENU_VATS: i32 = 0x420;
/// Menu ids `00705000` and `00705020` test for existence.
const MENU_435: i32 = 0x435;
const MENU_41D: i32 = 0x41d;
/// `StatsMenu::Create` (Xbox PDB), `StatsMenu::UpdateAll` (on the menu),
/// `StatsMenu::Create`, `StatsMenu::UpdateAll` (on the menu) and
/// `StatsMenu::SetKnobPosition` (`cdecl`, one word).
const STATS_MENU_CREATE: u32 = 0x007d_a2c0;
const STATS_UPDATE_ALL: u32 = 0x007d_cc80;
const STATS_SET_KNOB_POSITION: u32 = 0x007d_fc90;
/// The actor value that updates the whole stats screen (`0x4d`).
const ACTOR_VALUE_ALL: u32 = 0x4d;
/// The global holding the container menu object (`011d93f8`) and
/// `ContainerMenu::UpdateLists` (Xbox PDB) on it.
const CONTAINER_MENU_GLOBAL: u32 = 0x011d_93f8;
const CONTAINER_UPDATE_LISTS: u32 = 0x0075_c280;
/// The word at `+0x80` of the object at [`CONTAINER_MENU_GLOBAL`].
const CONTAINER_MENU_FIELD: u32 = 0x80;
/// `ContainerMenu::Create` (`cdecl`) and `ContainerMenu::Close` (Xbox PDB).
const CONTAINER_MENU_CREATE: u32 = 0x0075_b310;
const CONTAINER_MENU_CLOSE: u32 = 0x0075_b750;
/// `TESObjectREFR::GetOwner` and `TESObjectREFR::IsAnOwner` (Xbox PDB) on a
/// reference; the latter takes the word at `011dea3c` and a flag.
const REFR_GET_OWNER: u32 = 0x0056_7790;
const REFR_IS_AN_OWNER: u32 = 0x0057_85e0;
const OWNER_GLOBAL: u32 = 0x011d_ea3c;
/// Slot `0x100` of a reference (a getter answering a byte).
const REFR_SLOT_100: u32 = 0x100;
/// `BarterMenu::Create` and `RecipeMenu::Create` (Xbox PDB), `cdecl`, two
/// words each.
const BARTER_MENU_CREATE: u32 = 0x0072_d250;
const RECIPE_MENU_CREATE: u32 = 0x0072_6ff0;
/// `BSTimer::SetGlobalTimeMultiplier` (Xbox PDB) on the timer at
/// `011f6394`: a float and a flag.
const SET_GLOBAL_TIME_MULTIPLIER: u32 = 0x00aa_4db0;
const GLOBAL_TIMER: u32 = 0x011f_6394;
/// `DialogMenu::Create` (`cdecl`, three words), `DialogMenu::SetPause` (on
/// the menu, one word) and `DialogMenu::ServiceStart`.
const DIALOG_MENU_CREATE: u32 = 0x0076_1a20;
const DIALOG_SET_PAUSE: u32 = 0x0076_4200;
const DIALOG_SERVICE_START: u32 = 0x0076_3ff0;
/// The byte set while a dialog is open (`011d9514`).
const IN_DIALOG_FLAG: u32 = 0x011d_9514;
/// `HUDMainMenu::ShowSubtitle` (Xbox PDB) and `00775920`, on the HUD main menu.
const HUD_SHOW_SUBTITLE: u32 = 0x0077_4fd0;
const HUD_HIDE_SUBTITLE: u32 = 0x0077_5920;
/// `HUDMainMenu::SetRegionText` (Xbox PDB, `cdecl`): a pointer to a
/// 0x10c-byte request (a word, a 260-byte text, a flag byte at `+0x108`).
const HUD_SET_REGION_TEXT: u32 = 0x0077_28f0;
/// `strcpy_s(buffer, size, text)` (`00406d30`, `cdecl`).
const STRING_COPY_BOUNDED: u32 = 0x0040_6d30;
/// `StartMenu::Create(first, second)` (Xbox PDB, `cdecl`).
const START_MENU_CREATE: u32 = 0x007c_b7d0;
/// `LockPickMenu::Create` (Xbox PDB, `cdecl`, three words).
const LOCK_PICK_MENU_CREATE: u32 = 0x0078_db00;
/// `SleepWaitMenu::Create` (`cdecl`, one word), `SleepWaitMenu::
/// UpdateSleeping` (on the menu) and `SleepWaitMenu::Close` (Xbox PDB).
const SLEEP_MENU_CREATE: u32 = 0x007b_fc30;
const SLEEP_UPDATE_SLEEPING: u32 = 0x007c_0580;
const SLEEP_MENU_CLOSE: u32 = 0x007c_01b0;
/// `VATSMenu::Create` and `VATSMenu::Close` (Xbox PDB).
const VATS_MENU_CREATE: u32 = 0x007e_9200;
const VATS_MENU_CLOSE: u32 = 0x007e_bd50;
/// `RaceSexMenu::Create` (Xbox PDB, `cdecl`) and the words the deferred
/// path writes: the pending menu kind (`011d8950`) and its argument
/// (`011d8960`).
const RACE_SEX_MENU_CREATE: u32 = 0x007a_c730;
const PENDING_MENU_KIND: u32 = 0x011d_8950;
const PENDING_MENU_ARGUMENT: u32 = 0x011d_8960;
/// The pending menu kind of the race/sex menu.
const PENDING_RACE_SEX: u32 = 5;
/// The counter `0043d4d0` finds on `011c3ea4`: its `+4`.
const MENU_COUNTER: u32 = 0x011c_3ea8;
/// Methods of the manager the thin wrappers forward to.
const MANAGER_LAST_MINUTE_UPDATE: u32 = 0x0071_3c70;
const MANAGER_CALL_A: u32 = 0x0071_33f0;
const MANAGER_CALL_B: u32 = 0x0071_7e30;

/// The menu of class `menu_id` cast to `target_type`: `GetMenuByClass`,
/// `Tile::GetMenu` and `__RTDynamicCast`. 0 when there is no such menu or
/// it is another class.
fn menu_cast(e: &mut Engine, menu_id: i32, target_type: u32) -> u32 {
    let tile = menu_by_class(e, menu_id);
    if tile == 0 {
        return 0;
    }
    let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
    e.call(
        RT_DYNAMIC_CAST,
        &args![menu, 0u32, MENU_TYPE, target_type, 0u32],
    )
    .u32()
}

// Typed from 00704bc0
/// When the container menu object (`011d93f8`) exists:
/// `ContainerMenu::UpdateLists(0)` on it.
pub fn fn_00704bc0(e: &mut Engine) {
    let menu = e.global::<u32>(CONTAINER_MENU_GLOBAL);
    if menu != 0 {
        e.call(CONTAINER_UPDATE_LISTS, &args![menu, 0u32]);
    }
}

// Typed from 00704be0
/// `StatsMenu::Create` when the manager is ready, else 0.
pub fn fn_00704be0(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(_) => e.call(STATS_MENU_CREATE, &[]).u32(),
        None => 0,
    }
}

// Typed from 00704c10
/// `Interface::SetStatsMenuVisible` (Xbox PDB), `cdecl`: shows (`show`
/// set) or hides the stats menu; `check_state` is as in
/// [`interface_set_map_menu_visible`].
///
/// To show it while it is not visible: the inventory, repair (with
/// `RepairMenu::ResetMenu`), item-mod (with `ItemModMenu::ResetMenu`) and
/// map menus fade out, the stats menu is created if it does not exist, its
/// id goes into [`TRAIT_CURRENT_MENU`], `StatsMenu::UpdateAll` runs and it
/// fades in. Hiding a visible one fades it out. Showing always ends with
/// the Pipboy refresh (tab 1) and `StatsMenu::SetKnobPosition(0)` when
/// the Pip-Boy's query says so.
pub fn interface_set_stats_menu_visible(e: &mut Engine, show: u8, check_state: u8) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let [mut stats, inventory, repair, item_mod, map] = pipboy_menus(e);
    let visible = menu_visible(e, MENU_STATS, check_state);
    if show != 0 && !visible {
        if inventory != 0 {
            menu_fade_out(e, inventory);
        }
        if repair != 0 {
            menu_fade_out(e, repair);
            e.call(REPAIR_RESET_MENU, &[]);
        }
        if item_mod != 0 {
            menu_fade_out(e, item_mod);
            e.call(ITEM_MOD_RESET_MENU, &[]);
        }
        if map != 0 {
            menu_fade_out(e, map);
        }
        if stats == 0 {
            stats = typed_call!(super::FAMILY, 0x0070_4be0, fn_00704be0(e));
        }
        if stats != 0 {
            set_current_menu(e, manager, MENU_STATS);
            let menu = e.call(TILE_GET_MENU, &args![stats]).u32();
            e.call(STATS_UPDATE_ALL, &args![menu]);
            menu_fade_in(e, stats);
        }
    } else if show == 0 && visible && stats != 0 {
        menu_fade_out(e, stats);
    }
    if show != 0 {
        refresh_pipboy_tab(e, manager, 1);
        call_if_pipboy_query(e, manager, STATS_SET_KNOB_POSITION);
    }
}

// Typed from 00704df0
/// The flag byte of the stats menu id ([`fn_007027b0`]).
pub fn fn_00704df0(e: &mut Engine) -> u8 {
    typed_call!(super::FAMILY, 0x0070_27b0, fn_007027b0(e, MENU_STATS))
}

// Typed from 00704e10
/// `Interface::UpdateActorValue` (Xbox PDB), `cdecl`: when the manager is
/// ready, the stats menu is showing (state bits `0xb`) and really is a
/// `StatsMenu`: `UpdateAllActorValues` for the value `0x4d`, else
/// `UpdateActorValue(value)`.
pub fn interface_update_actor_value(e: &mut Engine, actor_value: u32) {
    if ready_manager(e).is_none() {
        return;
    }
    if typed_call!(
        super::FAMILY,
        0x0070_2680,
        interface_is_menu_id_visible(e, MENU_STATS, 0xb)
    ) == 0
    {
        return;
    }
    let menu = menu_cast(e, MENU_STATS, STATS_MENU_TYPE);
    if menu == 0 {
        return;
    }
    if actor_value == ACTOR_VALUE_ALL {
        typed_call!(
            super::FAMILY,
            0x007d_cfc0,
            super::stats_menu::stats_menu_update_all_actor_values(e, Ptr::new(menu))
        );
    } else {
        typed_call!(
            super::FAMILY,
            0x007d_d090,
            super::stats_menu::stats_menu_update_actor_value(e, Ptr::new(menu), actor_value)
        );
    }
}

// Typed from 00704ed0
/// `Interface::CreateContainerMenu` (Xbox PDB), `cdecl(reference, mode)`:
/// when the manager is ready, `ContainerMenu::Create`. Mode 1 becomes 2
/// when the reference's slot `0x100` answers false, it has an owner
/// (`TESObjectREFR::GetOwner`) and `IsAnOwner(011dea3c, 1)` is false.
pub fn interface_create_container_menu(e: &mut Engine, reference: u32, mode: u32) -> u32 {
    if ready_manager(e).is_none() {
        return 0;
    }
    let mut mode = mode;
    if mode == 1
        && e.vcall(reference, REFR_SLOT_100, &[]).u8() == 0
        && e.call(REFR_GET_OWNER, &args![reference]).u32() != 0
    {
        let player = e.global::<u32>(OWNER_GLOBAL);
        if e.call(REFR_IS_AN_OWNER, &args![reference, player, 1u32])
            .u8()
            == 0
        {
            mode = 2;
        }
    }
    e.call(CONTAINER_MENU_CREATE, &args![reference, mode]).u32()
}

// Typed from 00704f50
/// `ContainerMenu::Close` when the manager is ready.
pub fn fn_00704f50(e: &mut Engine) {
    if ready_manager(e).is_some() {
        e.call(CONTAINER_MENU_CLOSE, &[]);
    }
}

// Typed from 00704f80
/// `BarterMenu::Create(a, b)` (`cdecl`) when the manager is ready.
pub fn fn_00704f80(e: &mut Engine, first: u32, second: u32) {
    if ready_manager(e).is_some() {
        e.call(BARTER_MENU_CREATE, &args![first, second]);
    }
}

// Typed from 00704fc0
/// `RecipeMenu::Create(a, b)` (`cdecl`) when the manager is ready.
pub fn fn_00704fc0(e: &mut Engine, first: u32, second: u32) {
    if ready_manager(e).is_some() {
        e.call(RECIPE_MENU_CREATE, &args![first, second]);
    }
}

// Typed from 00705000
/// Whether a menu of class `0x435` exists.
pub fn fn_00705000(e: &mut Engine) -> bool {
    menu_by_class(e, MENU_435) != 0
}

// Typed from 00705020
/// Whether a menu of class `0x41d` exists.
pub fn fn_00705020(e: &mut Engine) -> bool {
    menu_by_class(e, MENU_41D) != 0
}

// Typed from 00705040
/// Returns what [`fn_00705050`] returns (the code calls it and leaves `eax`).
pub fn fn_00705040(e: &mut Engine) -> u32 {
    typed_call!(super::FAMILY, 0x0070_5050, fn_00705050(e))
}

// Typed from 00705050
/// The word at `+0x80` of the object at `011d93f8`, 0 when there is none.
pub fn fn_00705050(e: &mut Engine) -> u32 {
    let menu = e.global::<u32>(CONTAINER_MENU_GLOBAL);
    if menu == 0 {
        0
    } else {
        e.mem.u32(menu.wrapping_add(CONTAINER_MENU_FIELD))
    }
}

// Typed from 00705070
/// `Interface::CreateDialogMenu` (Xbox PDB), `cdecl`: first
/// `BSTimer::SetGlobalTimeMultiplier(1.0, 1)` on the timer at `011f6394`,
/// then `DialogMenu::Create` (three words) when the manager is ready, else
/// 0.
pub fn interface_create_dialog_menu(e: &mut Engine, first: u32, second: u32, third: u32) -> u32 {
    e.call(
        SET_GLOBAL_TIME_MULTIPLIER,
        &args![GLOBAL_TIMER, 1.0f32.to_bits(), 1u32],
    );
    if ready_manager(e).is_none() {
        return 0;
    }
    e.call(DIALOG_MENU_CREATE, &args![first, second, third])
        .u32()
}

// Typed from 007050d0
/// `Interface::InDialog` (Xbox PDB): the dialog flag ([`fn_00705100`]) when
/// the manager is ready, else 0.
pub fn interface_in_dialog(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(_) => typed_call!(super::FAMILY, 0x0070_5100, fn_00705100(e)),
        None => 0,
    }
}

// Typed from 00705100
/// The dialog flag byte (`011d9514`).
pub fn fn_00705100(e: &mut Engine) -> u8 {
    e.global::<u8>(IN_DIALOG_FLAG)
}

// Typed from 00705110
/// `Interface::SetDialogPause` (Xbox PDB), `cdecl`: when the manager is
/// ready and the menu of class `0x3f1` is a `DialogMenu`,
/// `DialogMenu::SetPause(pause)` on it.
pub fn interface_set_dialog_pause(e: &mut Engine, pause: u8) {
    if ready_manager(e).is_none() {
        return;
    }
    let menu = menu_cast(e, MENU_DIALOG, DIALOG_MENU_TYPE);
    if menu != 0 {
        e.call(DIALOG_SET_PAUSE, &args![menu, pause]);
    }
}

// Typed from 00705190
/// `Interface::InDialogWith` (Xbox PDB): the dialog menu's word at `+0x80` (`004fb070`) when the
/// manager is ready and the menu of class `0x3f1` is a `DialogMenu`, else 0.
pub fn interface_in_dialog_with(e: &mut Engine) -> u32 {
    if ready_manager(e).is_none() {
        return 0;
    }
    let menu = menu_cast(e, MENU_DIALOG, DIALOG_MENU_TYPE);
    if menu == 0 {
        return 0;
    }
    e.mem.u32(menu.wrapping_add(in_place::DIALOG_WORD))
}

// Typed from 00705210
/// `Interface::ShowSubtitle` (Xbox PDB), `cdecl`, nine words: a word
/// (`speaker`), a 12-byte value by value (`text_a/b/c`: a word, a byte and
/// a word), three words (`position_*`), a word and a byte. Inside a scope
/// guard (line 0x8ec of the interface's file): when the manager is ready and the
/// HUD main menu object (`011d96c0`) exists, calls `HUDMainMenu::ShowSubtitle`
/// on the object with the speaker, a copy of the 12-byte value (`00418900`
/// copies its word, byte and word: the byte's word goes with its other three
/// bytes zero, where the exe leaves the stack's), the three words, the word and
/// the byte. The by-value argument's destructor (`00483710`) does nothing.
/// Always returns 0. The exception frame is not translated.
#[allow(clippy::too_many_arguments)]
pub fn interface_show_subtitle(
    e: &mut Engine,
    speaker: u32,
    text_a: u32,
    text_b: u32,
    text_c: u32,
    position_a: u32,
    position_b: u32,
    position_c: u32,
    word: u32,
    flag: u8,
) -> u8 {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, 0xdu32, 1u32, INTERFACE_SOURCE_NAME, 0x8ecu32],
        );
        let hud = e.global::<u32>(in_place::HUD_MAIN_MENU);
        if ready_manager(e).is_some() && hud != 0 {
            let copy_b = u32::from(text_b.to_le_bytes()[0]);
            e.call(
                HUD_SHOW_SUBTITLE,
                &args![
                    hud, speaker, text_a, copy_b, text_c, position_a, position_b, position_c, word,
                    flag
                ],
            );
        }
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
    });
    0
}

// Typed from 007053b0
/// When the manager is ready and the HUD main menu object (`011d96c0`)
/// exists: `00775920` on it.
pub fn fn_007053b0(e: &mut Engine) {
    if ready_manager(e).is_none() {
        return;
    }
    let hud = e.global::<u32>(in_place::HUD_MAIN_MENU);
    if hud != 0 {
        e.call(HUD_HIDE_SUBTITLE, &args![hud]);
    }
}

// Typed from 007053f0
/// `StartMenu::Create(0, 0)` when the manager is ready.
pub fn fn_007053f0(e: &mut Engine) {
    if ready_manager(e).is_some() {
        e.call(START_MENU_CREATE, &args![0u32, 0u32]);
    }
}

// Typed from 00705420
/// `cdecl(region, text, flag)` where `text` is an 8-byte string by value
/// (two words, the first its character pointer): when the manager is
/// ready, builds a request on the stack (the word, the text copied with
/// `strcpy_s(buffer, 0x104, text)` or an empty string, the flag byte at
/// `+0x108`) and passes it to `HUDMainMenu::SetRegionText`. The string
/// argument is destroyed at the end (`004037d0`). The stack-protector
/// cookie and the exception frame are not translated.
pub fn fn_00705420(e: &mut Engine, region: u32, text_a: u32, text_b: u32, flag: u8) {
    e.with_stack(8, |e, text| {
        e.mem.set_u32(text.addr(), text_a);
        e.mem.set_u32(text.addr().wrapping_add(4), text_b);
        if ready_manager(e).is_some() {
            e.with_stack(0x10c, |e, request| {
                e.mem.set_u8(request.addr().wrapping_add(0x108), flag);
                e.mem.set_u32(request.addr(), region);
                let buffer = request.addr().wrapping_add(4);
                if text_a == 0 {
                    e.mem.set_u8(buffer, 0);
                } else {
                    e.call(STRING_COPY_BOUNDED, &args![buffer, 0x104u32, text_a]);
                }
                e.call(HUD_SET_REGION_TEXT, &args![request]);
            });
        }
        e.call(STRING_DTOR, &args![text]);
    });
}

// Typed from 007054f0
/// `Interface::CreateSleepMenu` (Xbox PDB), `cdecl(flag)`: when the manager
/// is ready and [`fn_00709be0`](super::interface_cpp_p3::fn_00709be0) (its
/// `+0x4bc` word is 0) says so: an existing sleep/wait menu (class `0x3f4`)
/// is destroyed through slot 0 with 1, then `SleepWaitMenu::Create(flag)`.
pub fn interface_create_sleep_menu(e: &mut Engine, flag: u8) {
    if ready_manager(e).is_none()
        || !typed_call!(
            super::FAMILY,
            0x0070_9be0,
            super::interface_cpp_p3::fn_00709be0(e)
        )
    {
        return;
    }
    let menu = menu_cast(e, MENU_SLEEP_WAIT, SLEEP_WAIT_MENU_TYPE);
    if menu != 0 {
        e.vcall(menu, 0, &args![1u32]);
    }
    e.call(SLEEP_MENU_CREATE, &args![flag]);
}

// Typed from 007055c0
/// `Interface::CloseSleepMenu` (Xbox PDB): when the manager is ready and
/// the menu of class `0x3f4` is a `SleepWaitMenu`: `SleepWaitMenu::Close`.
pub fn interface_close_sleep_menu(e: &mut Engine) {
    if ready_manager(e).is_none() {
        return;
    }
    if menu_cast(e, MENU_SLEEP_WAIT, SLEEP_WAIT_MENU_TYPE) != 0 {
        e.call(SLEEP_MENU_CLOSE, &[]);
    }
}

// Typed from 00705640
/// `Interface::CreateVATSMenu` (Xbox PDB): when the manager is ready, an
/// existing VATS menu (class `0x420`) is destroyed through slot 0 with 1,
/// then `VATSMenu::Create`.
pub fn interface_create_vats_menu(e: &mut Engine) {
    if ready_manager(e).is_none() {
        return;
    }
    let menu = menu_cast(e, MENU_VATS, VATS_MENU_TYPE);
    if menu != 0 {
        e.vcall(menu, 0, &args![1u32]);
    }
    e.call(VATS_MENU_CREATE, &[]);
}

// Typed from 007056f0
/// `Interface::UpdateSleeping` (Xbox PDB): looks up the menu of the
/// sleep/wait id ([`fn_00705770`]) first; then, when the manager is ready
/// and that menu exists, `SleepWaitMenu::UpdateSleeping` on it decides the
/// result (1 or 0).
pub fn interface_update_sleeping(e: &mut Engine) -> u8 {
    let id = typed_call!(super::FAMILY, 0x0070_5770, fn_00705770(e)) as i32;
    let tile = menu_by_class(e, id);
    let menu = if tile == 0 {
        0
    } else {
        e.call(TILE_GET_MENU, &args![tile]).u32()
    };
    u8::from(
        ready_manager(e).is_some()
            && menu != 0
            && e.call(SLEEP_UPDATE_SLEEPING, &args![menu]).bool(),
    )
}

// Typed from 00705770
/// The menu id of the sleep/wait menu, `0x3f4`.
pub fn fn_00705770(_e: &mut Engine) -> u32 {
    MENU_SLEEP_WAIT as u32
}

// Typed from 00705780
/// `Interface::CloseVATSMenu` (Xbox PDB): when the manager is ready and the
/// menu of class `0x420` is a `VATSMenu`: `VATSMenu::Close`.
pub fn interface_close_vats_menu(e: &mut Engine) {
    if ready_manager(e).is_none() {
        return;
    }
    if menu_cast(e, MENU_VATS, VATS_MENU_TYPE) != 0 {
        e.call(VATS_MENU_CLOSE, &[]);
    }
}

// Typed from 00705800
/// `StartMenu::Create(1, 1)` when the manager is ready.
pub fn fn_00705800(e: &mut Engine) {
    if ready_manager(e).is_some() {
        e.call(START_MENU_CREATE, &args![1u32, 1u32]);
    }
}

// Typed from 00705830
/// `LockPickMenu::Create(a, b, flag)` (`cdecl`) when the manager is ready.
pub fn fn_00705830(e: &mut Engine, first: u32, second: u32, flag: u8) {
    if ready_manager(e).is_some() {
        e.call(LOCK_PICK_MENU_CREATE, &args![first, second, flag]);
    }
}

// Typed from 00705870
/// `Interface::CreateRaceSexMenu` (Xbox PDB), `cdecl(argument)`: when the
/// dialog flag ([`fn_00705100`]) is set, `DialogMenu::ServiceStart` runs
/// first. Then, if the counter (`0043d4d0` on `011c3ea4`: its `+4`) is above
/// 1 (signed) and no menu is pending (`011d8950` is 0), the creation is
/// deferred: `011d8950` becomes 5 and `011d8960` the argument. Otherwise
/// `RaceSexMenu::Create(argument)` runs now.
pub fn interface_create_race_sex_menu(e: &mut Engine, argument: u32) {
    if typed_call!(super::FAMILY, 0x0070_5100, fn_00705100(e)) != 0 {
        e.call(DIALOG_SERVICE_START, &[]);
    }
    if e.mem.i32(MENU_COUNTER) > 1 && e.global::<u32>(PENDING_MENU_KIND) == 0 {
        e.set_global::<u32>(PENDING_MENU_KIND, PENDING_RACE_SEX);
        e.set_global::<u32>(PENDING_MENU_ARGUMENT, argument);
    } else {
        e.call(RACE_SEX_MENU_CREATE, &args![argument]);
    }
}

// Typed from 007058c0
/// `fastcall` getter: the word at `+0xc4` of `this`.
pub fn fn_007058c0(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(0xc4))
}

// Typed from 007058e0
/// `Interface::LastMinuteUpdate` (Xbox PDB): `00713c70` on the manager when
/// it is ready.
pub fn interface_last_minute_update(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(MANAGER_LAST_MINUTE_UPDATE, &args![manager]);
    }
}

// Typed from 00705910
/// The address of the manager's `+0x1dc` (`00602170`) when it is ready, else 0.
pub fn fn_00705910(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => manager.wrapping_add(in_place::FIELD_1DC),
        None => 0,
    }
}

// Typed from 00705950
/// The address of the manager's `+0x178` (`00460fb0`) when it is ready, else 0.
pub fn fn_00705950(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => manager.wrapping_add(in_place::PIPBOY_FIELD),
        None => 0,
    }
}

// Typed from 00705990
/// `Interface::GetPipboy` (Xbox PDB): the manager's Pipboy object (`+0x174`)
/// when it is ready, else 0.
pub fn interface_get_pipboy(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => in_place::pipboy(e, manager),
        None => 0,
    }
}

// Typed from 007059d0
/// `007133f0` on the manager when it is ready.
pub fn fn_007059d0(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(MANAGER_CALL_A, &args![manager]);
    }
}

// Typed from 00705a00
/// `Interface::IsInPipboyMenu` (Xbox PDB): when the manager is ready, true
/// if its `+0x4bc` word is 2 or 3 (`00709bc0`, `00709c00`).
pub fn interface_is_in_pipboy_menu(e: &mut Engine) -> u8 {
    if ready_manager(e).is_none() {
        return 0;
    }
    u8::from(
        typed_call!(
            super::FAMILY,
            0x0070_9bc0,
            super::interface_cpp_p3::interface_is_pipboy_opening(e)
        ) || typed_call!(
            super::FAMILY,
            0x0070_9c00,
            super::interface_cpp_p3::fn_00709c00(e)
        ),
    )
}

// Typed from 00705a60
/// `00717e30` on the manager when it is ready.
pub fn fn_00705a60(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(MANAGER_CALL_B, &args![manager]);
    }
}

/// This file's typed functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00702250, interface_init(u8)),
        entry!(0x007022c0, fn_007022c0()),
        entry!(0x00702330, fn_00702330()),
        entry!(0x00702360, interface_is_in_menu_mode() -> bool),
        entry!(0x007023a0, fn_007023a0(u32) -> bool),
        entry!(0x007023c0, interface_get_top_menu_id() -> u32),
        entry!(0x00702400, fn_00702400(u32) -> u32),
        entry!(0x00702440, fn_00702440() -> u32),
        entry!(0x00702450, interface_is_top_menu_id(i32) -> u8),
        entry!(0x007024e0, interface_is_top_menu_faded_in() -> bool),
        entry!(0x00702640, interface_get_menu_mode_type() -> u32),
        entry!(0x00702680, interface_is_menu_id_visible(i32, u32) -> u8),
        entry!(0x007027b0, fn_007027b0(i32) -> u8),
        entry!(0x007027e0, fn_007027e0()),
        entry!(0x00702810, fn_00702810()),
        entry!(0x00702840, fn_00702840()),
        entry!(0x00702870, fn_00702870()),
        entry!(0x007028b0, interface_click(u32, u32)),
        entry!(0x00702940, fn_00702940(u32, u8)),
        entry!(0x00702960, fn_00702960()),
        entry!(0x00702990, fn_00702990(u32, u8, u8, u8)),
        entry!(0x00702c80, interface_isolate_menu_elements(u8, u8)),
        entry!(0x00702dd0, interface_restore_menu_elements()),
        entry!(0x00702fc0, interface_is_debug_text_visible() -> bool),
        entry!(0x00703000, interface_toggle_debug_text_visible()),
        entry!(0x00703030, fn_00703030(u32)),
        entry!(0x00703060, fn_00703060(u32, u8)),
        entry!(0x00703080, fn_00703080(f32, f32)),
        entry!(0x007030c0, fn_007030c0(u32, u32)),
        entry!(0x00703100, interface_toggle_full_help()),
        entry!(0x00703150, interface_get_full_help() -> u8),
        entry!(0x00703180, interface_get_target_refr() -> u32),
        entry!(0x007031c0, fn_007031c0(u32) -> u32),
        entry!(0x007031e0, fn_007031e0(u32)),
        entry!(0x00703310, interface_set_pick_refr(u32)),
        entry!(0x00703350, fn_00703350() -> u32),
        entry!(0x00703390, interface_get_telekinesis_refr() -> u32),
        entry!(0x007033d0, fn_007033d0(Ptr) -> Ptr),
        entry!(0x00703430, fn_00703430() -> bool),
        entry!(0x00703470, fn_00703470(u32) -> bool),
        entry!(0x00703490, fn_00703490()),
        entry!(0x007034c0, fn_007034c0(u32)),
        entry!(0x00703500, interface_show_menus()),
        entry!(0x00703610, interface_hide_menus()),
        entry!(
            0x007037e0,
            interface_emergency_close_all_menus_and_break_stuff()
        ),
        entry!(0x00703810, fn_00703810(u8)),
        entry!(0x00703860, interface_set_crosshair_target_type(u32)),
        entry!(0x007038a0, fn_007038a0() -> u32),
        entry!(0x007038b0, interface_set_enemy_actor(u32)),
        entry!(0x007038d0, fn_007038d0() -> Ptr),
        entry!(0x007038e0, interface_toggle_safe_zone(u32)),
        entry!(0x00703920, interface_set_first_chance_texture_release()),
        entry!(0x00703960, fn_00703960(u32, u8)),
        entry!(0x00703980, fn_00703980()),
        entry!(
            0x007039b0,
            interface_get_control_push_text_for(i32, u32, u8)
        ),
        entry!(0x00703be0, fn_00703be0(u32, u32, u32) -> u8),
        (0x00703c00, fn_00703c00 as AbiFn),
        (0x00703c80, interface_print_line as AbiFn),
        entry!(0x00703d50, interface_is_console_visible() -> bool),
        entry!(0x00703da0, interface_close_console()),
        entry!(0x00703e10, interface_open_console()),
        (0x00703e80, fn_00703e80 as AbiFn),
        (0x00703f10, fn_00703f10 as AbiFn),
        entry!(0x00703fa0, interface_get_message_menu_result() -> u8),
        entry!(0x00703fd0, fn_00703fd0()),
        entry!(0x00703fe0, fn_00703fe0(u8)),
        entry!(0x00704000, interface_close_message_menu()),
        (0x00704010, fn_00704010 as AbiFn),
        entry!(0x00704130, fn_00704130()),
        entry!(0x00704140, fn_00704140() -> u32),
        entry!(0x00704170, interface_set_map_menu_visible(u8, u8)),
        entry!(0x00704490, interface_create_repair_menu() -> u32),
        entry!(0x007044c0, fn_007044c0(u8, u8, u8)),
        entry!(0x00704690, fn_00704690(u32)),
        entry!(0x007046c0, interface_create_item_mod_menu() -> u32),
        entry!(0x007046f0, fn_007046f0(u8, u8, u8)),
        entry!(0x007048c0, fn_007048c0() -> u32),
        entry!(0x007048f0, interface_set_inventory_menu_visible(u8, u8, u8)),
        entry!(0x00704ad0, interface_get_inventory_menu_visible() -> u8),
        entry!(0x00704af0, fn_00704af0()),
        entry!(0x00704bc0, fn_00704bc0()),
        entry!(0x00704be0, fn_00704be0() -> u32),
        entry!(0x00704c10, interface_set_stats_menu_visible(u8, u8)),
        entry!(0x00704df0, fn_00704df0() -> u8),
        entry!(0x00704e10, interface_update_actor_value(u32)),
        entry!(0x00704ed0, interface_create_container_menu(u32, u32) -> u32),
        entry!(0x00704f50, fn_00704f50()),
        entry!(0x00704f80, fn_00704f80(u32, u32)),
        entry!(0x00704fc0, fn_00704fc0(u32, u32)),
        entry!(0x00705000, fn_00705000() -> bool),
        entry!(0x00705020, fn_00705020() -> bool),
        entry!(0x00705040, fn_00705040() -> u32),
        entry!(0x00705050, fn_00705050() -> u32),
        entry!(0x00705070, interface_create_dialog_menu(u32, u32, u32) -> u32),
        entry!(0x007050d0, interface_in_dialog() -> u8),
        entry!(0x00705100, fn_00705100() -> u8),
        entry!(0x00705110, interface_set_dialog_pause(u8)),
        entry!(0x00705190, interface_in_dialog_with() -> u32),
        entry!(
            0x00705210,
            interface_show_subtitle(u32, u32, u32, u32, u32, u32, u32, u32, u8) -> u8
        ),
        entry!(0x007053b0, fn_007053b0()),
        entry!(0x007053f0, fn_007053f0()),
        entry!(0x00705420, fn_00705420(u32, u32, u32, u8)),
        entry!(0x007054f0, interface_create_sleep_menu(u8)),
        entry!(0x007055c0, interface_close_sleep_menu()),
        entry!(0x00705640, interface_create_vats_menu()),
        entry!(0x007056f0, interface_update_sleeping() -> u8),
        entry!(0x00705770, fn_00705770() -> u32),
        entry!(0x00705780, interface_close_vats_menu()),
        entry!(0x00705800, fn_00705800()),
        entry!(0x00705830, fn_00705830(u32, u32, u8)),
        entry!(0x00705870, interface_create_race_sex_menu(u32)),
        entry!(0x007058c0, fn_007058c0(u32) -> u32),
        entry!(0x007058e0, interface_last_minute_update()),
        entry!(0x00705910, fn_00705910() -> u32),
        entry!(0x00705950, fn_00705950() -> u32),
        entry!(0x00705990, interface_get_pipboy() -> u32),
        entry!(0x007059d0, fn_007059d0()),
        entry!(0x00705a00, interface_is_in_pipboy_menu() -> u8),
        entry!(0x00705a60, fn_00705a60()),
    ]
}
