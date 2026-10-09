//! `fallout/interface/interface.cpp` (Xbox PDB source unit), subsystem `fallout/interface`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! This file holds the range `00000000`..`00705a90` of the unit. Almost
//! every function is a static `Interface::` wrapper over the interface
//! manager (`InterfaceManager`, Xbox PDB): `004b7210` returns the manager
//! object (the word at `011d8a80`) and `009373f0` reads its first byte
//! (`bFirstInit`); a wrapper does nothing (returns 0) unless the manager
//! exists and that byte is set. The wrappers read the manager once where
//! the code reads the global again for each call; the global does not
//! change in between.
//!
//! Offsets of the manager object used here (PC offsets match the Xbox PDB):
//! `+0x0c cMenuMode`, `+0x7c bDebugTextVisible`, `+0x9c pMenusRoot`,
//! `+0xdd bFullHelp`, `+0xf0 pPickRef`, `+0x110 bFuzzyActivatePick`,
//! `+0x148 bClickMultithreaded`.
//!
//! The compiler's exception-unwinding frames (`Init`, `007031e0`) are not
//! translated.

#[allow(unused_imports)]
use crate::prelude::*;

/// `004b7210`: the interface manager object (the word at `011d8a80`).
const GET_MANAGER: u32 = 0x004b_7210;
/// `009373f0`: the byte at the start of the manager (`bFirstInit`).
const MANAGER_READY: u32 = 0x0093_73f0;
/// `InterfaceManager::GetEnterStackTop` (Xbox PDB), on the manager.
const GET_ENTER_STACK_TOP: u32 = 0x0071_4f00;
/// `InterfaceManager::GetEnterStack` (Xbox PDB): the entry of the menu
/// stack at the given index.
const GET_ENTER_STACK: u32 = 0x0071_4f70;
/// `Tile::GetMenuByClass` (Xbox PDB), `cdecl`: the menu with the given id.
const GET_MENU_BY_CLASS: u32 = 0x00a0_9030;
/// `Tile::GetMenu` (Xbox PDB): the menu a tile belongs to.
const TILE_GET_MENU: u32 = 0x00a0_3c90;
/// Reads the word at `+0x24` of a menu (the engine map names `0059bb30`
/// `D3DTexture_LockRect` because identical code was folded; the body is a
/// plain getter). `1` means the menu is faded in.
const MENU_STATE: u32 = 0x0059_bb30;
/// The menu id on top of the stack that means "one of the five menus in
/// [`TOP_MENU_CANDIDATES`]".
const TOP_PLACEHOLDER: i32 = 1;
/// The menu ids that stand in for [`TOP_PLACEHOLDER`], in the order the code
/// tries them.
const TOP_MENU_CANDIDATES: [i32; 5] = [0x3eb, 0x3ea, 0x40b, 0x425, 0x3ff];
/// First and last menu id of the per-menu flag table at
/// [`MENU_FLAG_TABLE`].
const FIRST_MENU_ID: i32 = 0x3e9;
const LAST_MENU_ID: i32 = 0x43c;
/// Byte table indexed by menu id (`011f308f + id`).
const MENU_FLAG_TABLE: u32 = 0x011f_308f;
/// `00709c00`: true when the manager's `+0x4bc` word is 3.
const MANAGER_MODE_IS_THREE: u32 = 0x0070_9c00;
/// `MoviePlayer::GetPlayingSequence` (Xbox PDB), on the movie player whose
/// pointer is at `0126fac4`.
const MOVIE_PLAYER_PLAYING: u32 = 0x00ec_17c0;
/// The movie player pointer.
const MOVIE_PLAYER: u32 = 0x0126_fac4;

/// Tile trait setter (`00700320`): tile, trait id, value.
const TILE_SET_VALUE: u32 = 0x0070_0320;
/// Tile trait getter (`00a011b0`, value in ST0): tile, trait id.
const TILE_GET_VALUE: u32 = 0x00a0_11b0;
/// `Tile::GetChildByID` (Xbox PDB).
const TILE_GET_CHILD_BY_ID: u32 = 0x00a0_3eb0;
/// Trait id whose value (110.0, 111.0 or 112.0) tells the isolate code what
/// part a tile plays (`0xfaa`).
const TRAIT_ROLE: u32 = 0xfaa;
/// Trait id read as "enabled" (`0xfa3`).
const TRAIT_ENABLED: u32 = 0xfa3;
/// Trait id written as "visible" (`0x1779`).
const TRAIT_VISIBLE: u32 = 0x1779;
/// `110.0`, `111.0`, `112.0` and `0.0` (`double`s the compares read).
const DOUBLE_110: u32 = 0x0106_ebc8;
const DOUBLE_111: u32 = 0x0106_ebc0;
const DOUBLE_112: u32 = 0x0106_ebd0;
const DOUBLE_ZERO: u32 = 0x0101_2060;
/// `0056c7f0`: the node a tile keeps (the first word of its `+0x2c`).
const TILE_NODE: u32 = 0x0056_c7f0;
/// `00450f90`: sets a flag (byte argument) on a node.
const SET_NODE_FLAG: u32 = 0x0045_0f90;
/// `00559450`: the first word of the object (a list head, a string's
/// character pointer).
const FIRST_WORD: u32 = 0x0055_9450;
/// `00586150`: the manager's `pMenusRoot` (`+0x9c`).
const GET_MENUS_ROOT: u32 = 0x0058_6150;
/// `0057cbe0`: list iterator step. The first argument word is the list
/// object, the second the address of the cursor word; it returns the
/// address of the current item and moves the cursor to the next node.
const LIST_NEXT: u32 = 0x0057_cbe0;
/// `00905820` on the manager's sub-object at `+0x4c4`; argument: the
/// address of a tile pointer.
const SUBOBJECT_ADD_TILE: u32 = 0x0090_5820;
/// `00470470` on that sub-object, no arguments.
const SUBOBJECT_CLEAR: u32 = 0x0047_0470;
/// Offset of that sub-object in the manager.
const MANAGER_SUBOBJECT: u32 = 0x4c4;
/// `005e3fc0` / `004fd3c0`: getters on the manager for two nodes whose
/// flag the isolate code sets.
const MANAGER_NODE_A: u32 = 0x005e_3fc0;
const MANAGER_NODE_B: u32 = 0x004f_d3c0;

/// `InterfaceManager::Initialize` (Xbox PDB), `cdecl`: two words.
const MANAGER_INITIALIZE: u32 = 0x0070_9fd0;
/// Scope guard constructor (`00404eb0`, `thiscall`: 0xd, 1, file, line) and
/// destructor (`00404ee0`); the guard is a 4-byte local.
const SCOPE_GUARD_CTOR: u32 = 0x0040_4eb0;
const SCOPE_GUARD_DTOR: u32 = 0x0040_4ee0;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout\Interface\Interface.cpp"`.
const INTERFACE_CPP_PATH: u32 = 0x0106_eb78;

/// Reads the manager and applies the wrappers' guard: `Some(manager)` only
/// when it exists and `009373f0` says so.
fn ready_manager(e: &mut Engine) -> Option<u32> {
    if e.call(GET_MANAGER, &[]).u32() == 0 {
        return None;
    }
    let manager = e.call(GET_MANAGER, &[]).u32();
    if e.call(MANAGER_READY, &args![manager]).bool() {
        Some(manager)
    } else {
        None
    }
}

fn tile_value(e: &mut Engine, tile: u32, id: u32) -> f64 {
    e.call(TILE_GET_VALUE, &args![tile, id]).f32() as f64
}

fn set_tile_visible(e: &mut Engine, tile: u32, visible: u32) {
    e.call(TILE_SET_VALUE, &args![tile, TRAIT_VISIBLE, visible]);
}

/// `true` when the tile's "enabled" trait or its "visible" trait is zero:
/// the value the code gives the tile node's flag.
fn node_flag_for(e: &mut Engine, tile: u32) -> bool {
    let zero: f64 = e.global(DOUBLE_ZERO);
    tile_value(e, tile, TRAIT_ENABLED) == zero || tile_value(e, tile, TRAIT_VISIBLE) == zero
}

/// Sets the flag of the tile's node (`00450f90` on `0056c7f0`'s result).
fn set_tile_node_flag(e: &mut Engine, tile: u32, flag: bool) {
    let node = e.call(TILE_NODE, &args![tile]).u32();
    e.call(SET_NODE_FLAG, &args![node, flag]);
}

/// The walk the isolate and restore code does over a tile list: the cursor
/// is a word in memory that `0057cbe0` advances; each item is a pointer to
/// a tile.
fn for_each_list_item(
    e: &mut Engine,
    first: u32,
    list_owner: u32,
    mut body: impl FnMut(&mut Engine, u32),
) {
    e.with_stack(4, |e, cursor| {
        e.mem.set_u32(cursor.addr(), first);
        while e.mem.u32(cursor.addr()) != 0 {
            let item = e.call(LIST_NEXT, &args![list_owner, cursor]).u32();
            let value = e.mem.u32(item);
            body(e, value);
        }
    });
}

// Translated from 00702250 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::Init` (Xbox PDB): inside a scope guard (line 0xb0 of
/// `Interface.cpp`), initializes the interface manager with `(1, flag)`.
pub fn interface_init(e: &mut Engine, flag: u8) {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, 0xdu32, 1u32, INTERFACE_CPP_PATH, 0xb0u32],
        );
        e.call(MANAGER_INITIALIZE, &args![1u32, flag]);
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
    });
}

// Translated from 007022c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// As [`interface_init`] with line 0xbd and the arguments `(2, 1)`.
pub fn fn_007022c0(e: &mut Engine) {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_CTOR,
            &args![guard, 0xdu32, 1u32, INTERFACE_CPP_PATH, 0xbdu32],
        );
        e.call(MANAGER_INITIALIZE, &args![2u32, 1u32]);
        e.call(SCOPE_GUARD_DTOR, &args![guard]);
    });
}

// Translated from 00702330 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: calls `0070a0b0` on it.
pub fn fn_00702330(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0070_a0b0, &args![manager]);
    }
}

// Translated from 00702360 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::IsInMenuMode` (Xbox PDB): the manager is ready and its menu
/// mode is not 1.
pub fn interface_is_in_menu_mode(e: &mut Engine) -> bool {
    match ready_manager(e) {
        Some(manager) => fn_007023a0(e, manager),
        None => false,
    }
}

// Translated from 007023a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `cMenuMode` of the manager (`+0x0c`) is not 1.
pub fn fn_007023a0(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(this + 0x0c) != 1
}

// Translated from 007023c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetTopMenuID` (Xbox PDB): `GetEnterStackTop` of the manager,
/// 0 when it is not ready.
pub fn interface_get_top_menu_id(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => e.call(GET_ENTER_STACK_TOP, &args![manager]).u32(),
        None => 0,
    }
}

// Translated from 00702400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `GetEnterStack(index)` of the manager, 0 when it is not ready.
pub fn fn_00702400(e: &mut Engine, index: u32) -> u32 {
    match ready_manager(e) {
        Some(manager) => e.call(GET_ENTER_STACK, &args![manager, index]).u32(),
        None => 0,
    }
}

// Translated from 00702440 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the constant 10.
pub fn fn_00702440(_e: &mut Engine) -> u32 {
    10
}

// Translated from 00702450 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::IsTopMenuID` (Xbox PDB): whether the top of the menu stack
/// is the menu `menu_id`. When the top is the placeholder id 1 and
/// `menu_id` is one of the five ids that stand in for it, the answer is
/// whether that menu is visible instead.
pub fn interface_is_top_menu_id(e: &mut Engine, menu_id: i32) -> u8 {
    let Some(manager) = ready_manager(e) else {
        return 0;
    };
    let top = e.call(GET_ENTER_STACK_TOP, &args![manager]).i32();
    if top == TOP_PLACEHOLDER && TOP_MENU_CANDIDATES.contains(&menu_id) {
        return interface_is_menu_id_visible(e, menu_id, 0);
    }
    (top == menu_id) as u8
}

// Translated from 007024e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::IsTopMenuFadedIn` (Xbox PDB): finds the top menu (for the
/// placeholder id 1, the first visible one of the five stand-in menus) and
/// returns whether its state word (`0059bb30`) is 1.
pub fn interface_is_top_menu_faded_in(e: &mut Engine) -> bool {
    let Some(manager) = ready_manager(e) else {
        return false;
    };
    let top = e.call(GET_ENTER_STACK_TOP, &args![manager]).i32();
    let mut menu = 0;
    if top == TOP_PLACEHOLDER {
        for id in TOP_MENU_CANDIDATES {
            if interface_is_menu_id_visible(e, id, 0) != 0 {
                menu = e.call(GET_MENU_BY_CLASS, &args![id]).u32();
                break;
            }
        }
    } else {
        menu = e.call(GET_MENU_BY_CLASS, &args![top]).u32();
    }
    if menu == 0 {
        return false;
    }
    let inner = e.call(TILE_GET_MENU, &args![menu]).u32();
    if inner == 0 {
        return false;
    }
    let inner = e.call(TILE_GET_MENU, &args![menu]).u32();
    e.call(MENU_STATE, &args![inner]).u32() == 1
}

// Translated from 00702640 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetMenuModeType` (Xbox PDB): `GetEnterStack(0)` of the
/// manager, 0 when it is not ready.
pub fn interface_get_menu_mode_type(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => e.call(GET_ENTER_STACK, &args![manager, 0u32]).u32(),
        None => 0,
    }
}

// Translated from 00702680 (decompiled, FalloutNV.exe 1.4.0.525)
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
        if !e.call(MANAGER_MODE_IS_THREE, &[]).bool() {
            return 0;
        }
        for id in TOP_MENU_CANDIDATES {
            if interface_is_menu_id_visible(e, id, mask) != 0 {
                return 1;
            }
        }
        return 0;
    }
    if mask != 0 {
        let menu = e.call(GET_MENU_BY_CLASS, &args![menu_id]).u32();
        if menu != 0 {
            let inner = e.call(TILE_GET_MENU, &args![menu]).u32();
            if inner != 0 {
                let inner = e.call(TILE_GET_MENU, &args![menu]).u32();
                let state = e.call(MENU_STATE, &args![inner]).u32();
                if state & mask == 0 {
                    return 0;
                }
            }
        }
    }
    fn_007027b0(e, menu_id)
}

// Translated from 007027b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The flag byte of menu `menu_id` in the table at `011f308f`, 0 for an id
/// outside `0x3e9..=0x43c`.
pub fn fn_007027b0(e: &mut Engine, menu_id: i32) -> u8 {
    if !(FIRST_MENU_ID..=LAST_MENU_ID).contains(&menu_id) {
        return 0;
    }
    e.mem.u8(MENU_FLAG_TABLE.wrapping_add(menu_id as u32))
}

// Translated from 007027e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `InterfaceManager::PreIdleStuff` (Xbox PDB).
pub fn fn_007027e0(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0070_b8f0, &args![manager]);
    }
}

// Translated from 00702810 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `InterfaceManager::Idle` (Xbox PDB).
pub fn fn_00702810(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0070_c4a0, &args![manager]);
    }
}

// Translated from 00702840 (decompiled, FalloutNV.exe 1.4.0.525)
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

// Translated from 00702870 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready and no movie sequence is playing: calls
/// `00713f00` on the manager.
pub fn fn_00702870(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        if !movie_playing(e) {
            e.call(0x0071_3f00, &args![manager]);
        }
    }
}

// Translated from 007028b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::Click` (Xbox PDB): when the manager is ready and no movie is
/// playing, hands the click `(x, y)` to `00713fb0`; otherwise sets the
/// render thread stage of both rendering threads to 0x17 (`004ea970` returns
/// the rendering system, `00ba30f0` / `00ba3130` are its stage setters).
/// Always clears `bClickMultithreaded` (`+0x148`) afterwards.
pub fn interface_click(e: &mut Engine, x: u32, y: u32) {
    let mut handled = false;
    if let Some(manager) = ready_manager(e) {
        if !movie_playing(e) {
            e.call(0x0071_3fb0, &args![manager, x, y]);
            handled = true;
        }
    }
    if !handled {
        let system = e.call(0x004e_a970, &args![0u32, 0x17u32]).u32();
        e.call(0x00ba_30f0, &args![system]);
        let system = e.call(0x004e_a970, &args![1u32, 0x17u32]).u32();
        e.call(0x00ba_3130, &args![system]);
    }
    let manager = e.call(GET_MANAGER, &[]).u32();
    fn_00702940(e, manager, 0);
}

// Translated from 00702940 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte `value` in `bClickMultithreaded` (`+0x148`) of the
/// manager.
pub fn fn_00702940(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(this + 0x148, value);
}

// Translated from 00702960 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: asks the movie player whether a sequence is
/// playing and drops the answer.
pub fn fn_00702960(e: &mut Engine) {
    if ready_manager(e).is_some() {
        movie_playing(e);
    }
}

// Translated from 00702990 (decompiled, FalloutNV.exe 1.4.0.525)
/// Updates the visibility of one menu tile `tile` and of those children
/// whose role trait (`0xfaa`) is 110.0 or 111.0; `flag_a`, `flag_b` and
/// `flag_c` are the bytes `IsolateMenuElements` passes (`flag_c` is always 1
/// there). Does nothing for a null tile.
pub fn fn_00702990(e: &mut Engine, tile: u32, flag_a: u8, flag_b: u8, flag_c: u8) {
    if tile == 0 {
        return;
    }
    let manager = e.call(GET_MANAGER, &[]).u32();
    let menus_root = e.call(GET_MENUS_ROOT, &args![manager]).u32();
    let manager = e.call(GET_MANAGER, &[]).u32();
    let role = tile_value(e, tile, TRAIT_ROLE);
    let is_112 = role == e.global::<f64>(DOUBLE_112);
    let visible = if (flag_b != 0 && !is_112) || (flag_b == 0 && is_112) {
        0
    } else {
        1
    };
    set_tile_visible(e, tile, visible);
    let first = e.call(FIRST_WORD, &args![tile + 4]).u32();
    for_each_list_item(e, first, menus_root + 4, |e, child| {
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
                            &args![manager + MANAGER_SUBOBJECT, slot],
                        );
                    });
                }
            }
        }
        if tile_111 != 0 {
            set_tile_visible(e, tile_111, (flag_a == 0) as u32);
        }
        if tile_110 != 0 && e.call(TILE_NODE, &args![tile_110]).u32() != 0 {
            let flag = node_flag_for(e, tile_110);
            set_tile_node_flag(e, tile_110, flag);
        }
        if tile_111 != 0 && e.call(TILE_NODE, &args![tile_111]).u32() != 0 {
            let flag = flag_a != 0 || node_flag_for(e, tile_111);
            set_tile_node_flag(e, tile_111, flag);
        }
    });
    if e.call(TILE_NODE, &args![tile]).u32() != 0 {
        let flag = node_flag_for(e, tile);
        set_tile_node_flag(e, tile, flag);
    }
}

// Translated from 00702c80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::IsolateMenuElements` (Xbox PDB): when the manager is ready,
/// applies [`fn_00702990`] with `(tile, a, b, 1)` to every menu tile of the
/// menus root (when `a` is set and `b` is not, first clears the manager's
/// sub-object at `+0x4c4`), then sets the flag of the manager's two nodes
/// (`005e3fc0`, `004fd3c0`) to 0 when both `a` and `b` are zero, else 1.
pub fn interface_isolate_menu_elements(e: &mut Engine, a: u8, b: u8) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    if a != 0 && b == 0 {
        e.call(SUBOBJECT_CLEAR, &args![manager + MANAGER_SUBOBJECT]);
    }
    let menus_root = e.call(GET_MENUS_ROOT, &args![manager]).u32();
    let first = e.call(FIRST_WORD, &args![menus_root + 4]).u32();
    for_each_list_item(e, first, menus_root + 4, |e, menu| {
        fn_00702990(e, menu, a, b, 1);
    });
    let flag = !(a == 0 && b == 0);
    for getter in [MANAGER_NODE_A, MANAGER_NODE_B] {
        if e.call(getter, &args![manager]).u32() != 0 {
            let node = e.call(getter, &args![manager]).u32();
            e.call(SET_NODE_FLAG, &args![node, flag]);
        }
    }
}

// Translated from 00702dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::RestoreMenuElements` (Xbox PDB): for every menu tile of the
/// menus root, makes it and its children `0x6e` and `0x6f` visible and
/// recomputes their node flags.
pub fn interface_restore_menu_elements(e: &mut Engine) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    let menus_root = e.call(GET_MENUS_ROOT, &args![manager]).u32();
    let first = e.call(FIRST_WORD, &args![menus_root + 4]).u32();
    for_each_list_item(e, first, menus_root + 4, |e, menu| {
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
            if tile != 0 && e.call(TILE_NODE, &args![tile]).u32() != 0 {
                let flag = node_flag_for(e, tile);
                set_tile_node_flag(e, tile, flag);
            }
        }
    });
}

// Translated from 00702fc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::IsDebugTextVisible` (Xbox PDB): `00644640` on the manager
/// when it is ready.
pub fn interface_is_debug_text_visible(e: &mut Engine) -> bool {
    match ready_manager(e) {
        Some(manager) => e.call(0x0064_4640, &args![manager]).bool(),
        None => false,
    }
}

// Translated from 00703000 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::ToggleDebugTextVisible` (Xbox PDB): when the manager is
/// ready, [`fn_00703030`] on it.
pub fn interface_toggle_debug_text_visible(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        fn_00703030(e, manager);
    }
}

// Translated from 00703030 (decompiled, FalloutNV.exe 1.4.0.525)
/// Flips `bDebugTextVisible` (`+0x7c`) through [`fn_00703060`].
pub fn fn_00703030(e: &mut Engine, this: u32) {
    let visible = e.mem.u8(this + 0x7c) != 0;
    fn_00703060(e, this, !visible as u8);
}

// Translated from 00703060 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte `value` in `bDebugTextVisible` (`+0x7c`).
pub fn fn_00703060(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(this + 0x7c, value);
}

// Translated from 00703080 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `007151b0(manager, a, b)` with two floats.
pub fn fn_00703080(e: &mut Engine, a: f32, b: f32) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0071_51b0, &args![manager, a, b]);
    }
}

// Translated from 007030c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: `00715e00(manager, a, b)` with two words.
pub fn fn_007030c0(e: &mut Engine, a: u32, b: u32) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x0071_5e00, &args![manager, a, b]);
    }
}

// Translated from 00703100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::ToggleFullHelp` (Xbox PDB): flips `bFullHelp` (`+0xdd`).
pub fn interface_toggle_full_help(e: &mut Engine) {
    if let Some(manager) = ready_manager(e) {
        let full = e.mem.u8(manager + 0xdd) != 0;
        e.mem.set_u8(manager + 0xdd, !full as u8);
    }
}

// Translated from 00703150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetFullHelp` (Xbox PDB): `bFullHelp` (`+0xdd`), 0 when the
/// manager is not ready.
pub fn interface_get_full_help(e: &mut Engine) -> u8 {
    match ready_manager(e) {
        Some(manager) => e.mem.u8(manager + 0xdd),
        None => 0,
    }
}

// Translated from 00703180 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetTargetREFR` (Xbox PDB): `pPickRef` through
/// [`fn_007031c0`], 0 when the manager is not ready.
pub fn interface_get_target_refr(e: &mut Engine) -> u32 {
    let manager = e.call(GET_MANAGER, &[]).u32();
    if manager != 0 && e.call(MANAGER_READY, &args![manager]).bool() {
        fn_007031c0(e, manager)
    } else {
        0
    }
}

// Translated from 007031c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `pPickRef` (`+0xf0`) of the manager.
pub fn fn_007031c0(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this + 0xf0)
}

// Translated from 007031e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// When the manager is ready: for a non-null reference prints
/// `"<name>" (<form id>)` as debug text (`DebugText::Print`), then calls
/// `00714d70(manager, reference)` in every case. The text goes at x =
/// 640.0 (`0103a1c4`), y = the integer the setting at `011debac` holds,
/// with the arguments `2, -1, -1.0, 0, 0`.
///
/// A chain of getters (`0043d4d0`, `005bd5b0`, `005bd5c0`, `005bd580`,
/// `_ftol2`) computes a line number that the code never uses; the calls
/// are kept.
pub fn fn_007031e0(e: &mut Engine, reference: u32) {
    let Some(manager) = ready_manager(e) else {
        return;
    };
    e.with_stack(8, |e, text| {
        e.call(0x0040_37b0, &args![text]);
        if reference != 0 {
            let form_id = e.call(0x0084_e3a0, &args![reference]).u32();
            let name = e.call(0x0055_d520, &args![reference]).u32();
            e.call(0x0040_6f60, &args![text, 0x0103_a1c8u32, name, form_id]);
            let setting = e.call(0x0043_d4d0, &args![0x011f_33c8u32]).u32();
            let value = e.mem.u32(setting).wrapping_sub(1);
            let a = e.call(0x005b_d5b0, &args![value]).u32();
            let b = e.call(0x005b_d5c0, &args![a]).u32();
            let height = e.call(0x005b_d580, &args![b]).f32();
            let _unused_line = e
                .call(0x00ec_62c0, &args![height as f64])
                .i32()
                .wrapping_add(3);
            let y_setting = e.call(0x0043_d4d0, &args![0x011d_ebacu32]).u32();
            let y = e.mem.i32(y_setting);
            let characters = e.call(FIRST_WORD, &args![text]).u32();
            let x: f32 = e.global(0x0103_a1c4);
            let minus_one: f32 = e.global(0x0101_2054);
            let debug_text = e.call(0x00a0_d9e0, &args![1u32]).u32();
            e.call(
                0x00a0_f8b0,
                &args![debug_text, characters, x, y as f32, 2u32, -1i32, minus_one, 0u32, 0u32],
            );
        }
        e.call(0x0071_4d70, &args![manager, reference]);
        e.call(0x0040_37d0, &args![text]);
    });
}

// Translated from 00703310 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::SetPickREFR` (Xbox PDB): `009bdf60(manager, reference)`
/// when the manager is ready.
pub fn interface_set_pick_refr(e: &mut Engine, reference: u32) {
    if let Some(manager) = ready_manager(e) {
        e.call(0x009b_df60, &args![manager, reference]);
    }
}

// Translated from 00703350 (decompiled, FalloutNV.exe 1.4.0.525)
/// `00874480` on the manager when it is ready, else 0.
pub fn fn_00703350(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => e.call(0x0087_4480, &args![manager]).u32(),
        None => 0,
    }
}

// Translated from 00703390 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Interface::GetTelekinesisREFR` (Xbox PDB): `009b4440` on the manager
/// when it is ready, else 0.
pub fn interface_get_telekinesis_refr(e: &mut Engine) -> u32 {
    match ready_manager(e) {
        Some(manager) => e.call(0x009b_4440, &args![manager]).u32(),
        None => 0,
    }
}

// Translated from 007033d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes a 3-float point at `out` and returns `out`: `009b7010(manager,
/// out)` fills it when the manager is ready, else it is copied from the
/// three words at `011f426c`.
pub fn fn_007033d0(e: &mut Engine, out: Ptr) -> Ptr {
    if let Some(manager) = ready_manager(e) {
        e.call(0x009b_7010, &args![manager, out]);
        return out;
    }
    for i in 0..3 {
        let word = e.mem.u32(0x011f_426c + 4 * i);
        e.mem.set_u32(out.addr() + 4 * i, word);
    }
    out
}

// Translated from 00703430 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_00703470`] on the manager when it is ready, else false.
pub fn fn_00703430(e: &mut Engine) -> bool {
    match ready_manager(e) {
        Some(manager) => fn_00703470(e, manager),
        None => false,
    }
}

// Translated from 00703470 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bFuzzyActivatePick` (`+0x110`) of the manager.
pub fn fn_00703470(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this + 0x110) != 0
}

/// This unit's translated functions, by exe address.
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Registers a double that returns a fixed value in `eax`.
    macro_rules! stub {
        ($e:expr, $addr:expr, $value:expr) => {
            $e.register($addr, |_, _| Ret {
                eax: $value,
                ..Ret::default()
            })
        };
    }

    /// An engine whose manager exists and is ready; the pages holding the
    /// globals the code reads are mapped.
    fn ui_engine() -> (Engine, u32) {
        let mut e = Engine::new();
        for page in [
            0x0101_2000,
            0x0103_a000,
            0x0106_e000,
            0x011d_8000,
            0x011d_e000,
            0x011f_3000,
            0x011f_4000,
            0x0126_f000,
        ] {
            e.map(page, 0x1000);
        }
        let manager = e.mem.alloc(0x500);
        e.mem.set_u32(0x011d_8a80, manager);
        e.mem.set_u8(manager, 1);
        e.register(MANAGER_READY, |e, a| Ret {
            eax: e.mem.u8(a[0]) as u32,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        (e, manager)
    }

    /// The same, with no manager object.
    fn engine_without_manager() -> Engine {
        let (mut e, _) = ui_engine();
        e.mem.set_u32(0x011d_8a80, 0);
        e
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

    /// Checks a wrapper that calls `callee(manager)` when the manager is
    /// ready and nothing otherwise.
    fn check_forwards_to_manager(wrapper: u32, callee: u32) {
        let (mut e, manager) = ui_engine();
        e.register(callee, |_, _| Ret::default());
        e.call(wrapper, &[]);
        assert_eq!(logged(&e, callee), vec![vec![manager]]);

        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        e.call(wrapper, &[]);
        assert!(logged(&e, callee).is_empty());

        let mut e = engine_without_manager();
        e.register(callee, |_, _| Ret::default());
        e.call(wrapper, &[]);
        assert!(logged(&e, callee).is_empty());
    }

    /// A list node `[next, 0, item]`, where the item is a tile pointer.
    fn list_node(e: &mut Engine, next: u32, tile: u32) -> u32 {
        let node = e.mem.alloc(12);
        e.mem.set_u32(node, next);
        e.mem.set_u32(node + 8, tile);
        node
    }

    /// Doubles for the list helpers: the first word of an object and the
    /// iterator step (returns the address of the node's item, moves the
    /// cursor to the next node).
    fn list_doubles(e: &mut Engine) {
        e.register(FIRST_WORD, |e, a| Ret {
            eax: e.mem.u32(a[0]),
            ..Ret::default()
        });
        e.register(LIST_NEXT, |e, a| {
            let node = e.mem.u32(a[1]);
            let next = e.mem.u32(node);
            e.mem.set_u32(a[1], next);
            Ret {
                eax: node + 8,
                ..Ret::default()
            }
        });
    }

    /// A tile trait double: values by `(tile, trait)`, 0.0 otherwise.
    fn tile_values(e: &mut Engine, values: &[((u32, u32), f32)]) {
        let table: HashMap<(u32, u32), f32> = values.iter().copied().collect();
        e.register_double(TILE_GET_VALUE, move |_, a| Ret {
            st0: table.get(&(a[0], a[1])).copied().unwrap_or(0.0) as f64,
            ..Ret::default()
        });
    }

    #[test]
    fn init_runs_the_manager_initializer_inside_the_guard() {
        let (mut e, _) = ui_engine();
        e.register(SCOPE_GUARD_CTOR, |_, _| Ret::default());
        e.register(SCOPE_GUARD_DTOR, |_, _| Ret::default());
        e.register(MANAGER_INITIALIZE, |_, _| Ret::default());
        e.call(0x0070_2250, &args![5u32]);
        let log = e.call_log.take().unwrap();
        let order: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            order,
            vec![
                0x0070_2250,
                SCOPE_GUARD_CTOR,
                MANAGER_INITIALIZE,
                SCOPE_GUARD_DTOR
            ]
        );
        let guard = log[1].1[0];
        assert_eq!(&log[1].1[1..], &[0xd, 1, INTERFACE_CPP_PATH, 0xb0]);
        assert_eq!(log[2].1, vec![1, 5]);
        assert_eq!(log[3].1, vec![guard]);
    }

    #[test]
    fn fn_007022c0_initializes_with_two_and_one() {
        let (mut e, _) = ui_engine();
        e.register(SCOPE_GUARD_CTOR, |_, _| Ret::default());
        e.register(SCOPE_GUARD_DTOR, |_, _| Ret::default());
        e.register(MANAGER_INITIALIZE, |_, _| Ret::default());
        e.call(0x0070_22c0, &[]);
        assert_eq!(logged(&e, MANAGER_INITIALIZE), vec![vec![2, 1]]);
        assert_eq!(logged(&e, SCOPE_GUARD_CTOR)[0][4], 0xbd);
    }

    #[test]
    fn fn_00702330_calls_on_the_ready_manager() {
        check_forwards_to_manager(0x0070_2330, 0x0070_a0b0);
    }

    #[test]
    fn is_in_menu_mode_is_menu_mode_other_than_one() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u32(manager + 0x0c, 1);
        assert!(!e.call(0x0070_2360, &[]).bool());
        e.mem.set_u32(manager + 0x0c, 2);
        assert!(e.call(0x0070_2360, &[]).bool());
        e.mem.set_u8(manager, 0);
        assert!(!e.call(0x0070_2360, &[]).bool());
    }

    #[test]
    fn fn_007023a0_compares_the_menu_mode_with_one() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u32(manager + 0x0c, 1);
        assert!(!e.call(0x0070_23a0, &args![manager]).bool());
        e.mem.set_u32(manager + 0x0c, 0);
        assert!(e.call(0x0070_23a0, &args![manager]).bool());
    }

    #[test]
    fn get_top_menu_id_asks_the_enter_stack() {
        let (mut e, manager) = ui_engine();
        stub!(e, GET_ENTER_STACK_TOP, 7);
        assert_eq!(e.call(0x0070_23c0, &[]).u32(), 7);
        assert_eq!(logged(&e, GET_ENTER_STACK_TOP), vec![vec![manager]]);
        let mut e = engine_without_manager();
        assert_eq!(e.call(0x0070_23c0, &[]).u32(), 0);
    }

    #[test]
    fn fn_00702400_asks_the_enter_stack_at_an_index() {
        let (mut e, manager) = ui_engine();
        e.register(GET_ENTER_STACK, |_, a| Ret {
            eax: a[1] + 100,
            ..Ret::default()
        });
        assert_eq!(e.call(0x0070_2400, &args![3u32]).u32(), 103);
        assert_eq!(logged(&e, GET_ENTER_STACK), vec![vec![manager, 3]]);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_2400, &args![3u32]).u32(), 0);
    }

    #[test]
    fn fn_00702440_returns_ten() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x0070_2440, &[]).u32(), 10);
    }

    #[test]
    fn is_top_menu_id_compares_or_checks_visibility_for_the_placeholder() {
        let (mut e, manager) = ui_engine();
        stub!(e, GET_ENTER_STACK_TOP, 5);
        assert_eq!(e.call(0x0070_2450, &args![5u32]).u8(), 1);
        assert_eq!(e.call(0x0070_2450, &args![6u32]).u8(), 0);
        // The placeholder (1) on top: the stand-in menus use the flag table.
        stub!(e, GET_ENTER_STACK_TOP, 1);
        e.mem.set_u8(MENU_FLAG_TABLE + 0x3eb, 1);
        assert_eq!(e.call(0x0070_2450, &args![0x3ebu32]).u8(), 1);
        assert_eq!(e.call(0x0070_2450, &args![0x3eau32]).u8(), 0);
        // Another id is compared with the top.
        assert_eq!(e.call(0x0070_2450, &args![7u32]).u8(), 0);
        assert_eq!(e.call(0x0070_2450, &args![1u32]).u8(), 1);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_2450, &args![1u32]).u8(), 0);
    }

    #[test]
    fn is_top_menu_faded_in_reads_the_menu_state() {
        let (mut e, _) = ui_engine();
        stub!(e, GET_ENTER_STACK_TOP, 7);
        stub!(e, GET_MENU_BY_CLASS, 0x1234);
        stub!(e, TILE_GET_MENU, 0x2345);
        stub!(e, MENU_STATE, 1);
        assert!(e.call(0x0070_24e0, &[]).bool());
        assert_eq!(logged(&e, GET_MENU_BY_CLASS), vec![vec![7]]);
        assert_eq!(logged(&e, MENU_STATE), vec![vec![0x2345]]);
        stub!(e, MENU_STATE, 2);
        assert!(!e.call(0x0070_24e0, &[]).bool());
        stub!(e, TILE_GET_MENU, 0);
        assert!(!e.call(0x0070_24e0, &[]).bool());
        stub!(e, GET_MENU_BY_CLASS, 0);
        assert!(!e.call(0x0070_24e0, &[]).bool());
    }

    #[test]
    fn is_top_menu_faded_in_picks_the_first_visible_stand_in() {
        let (mut e, _) = ui_engine();
        stub!(e, GET_ENTER_STACK_TOP, 1);
        stub!(e, GET_MENU_BY_CLASS, 0x1234);
        stub!(e, TILE_GET_MENU, 0x2345);
        stub!(e, MENU_STATE, 1);
        e.mem.set_u8(MENU_FLAG_TABLE + 0x40b, 1);
        e.mem.set_u8(MENU_FLAG_TABLE + 0x3ff, 1);
        assert!(e.call(0x0070_24e0, &[]).bool());
        assert_eq!(logged(&e, GET_MENU_BY_CLASS), vec![vec![0x40b]]);
        // None visible: no menu, not faded in.
        e.mem.set_u8(MENU_FLAG_TABLE + 0x40b, 0);
        e.mem.set_u8(MENU_FLAG_TABLE + 0x3ff, 0);
        assert!(!e.call(0x0070_24e0, &[]).bool());
        let mut e = engine_without_manager();
        assert!(!e.call(0x0070_24e0, &[]).bool());
    }

    #[test]
    fn get_menu_mode_type_asks_for_stack_entry_zero() {
        let (mut e, manager) = ui_engine();
        stub!(e, GET_ENTER_STACK, 9);
        assert_eq!(e.call(0x0070_2640, &[]).u32(), 9);
        assert_eq!(logged(&e, GET_ENTER_STACK), vec![vec![manager, 0]]);
        let mut e = engine_without_manager();
        assert_eq!(e.call(0x0070_2640, &[]).u32(), 0);
    }

    #[test]
    fn is_menu_id_visible_handles_the_placeholder_and_the_mask() {
        let (mut e, manager) = ui_engine();
        stub!(e, MANAGER_MODE_IS_THREE, 0);
        e.mem.set_u8(MENU_FLAG_TABLE + 0x3ea, 1);
        // Placeholder: needs 00709c00 and a visible stand-in.
        assert_eq!(e.call(0x0070_2680, &args![1u32, 0u32]).u8(), 0);
        stub!(e, MANAGER_MODE_IS_THREE, 1);
        assert_eq!(e.call(0x0070_2680, &args![1u32, 0u32]).u8(), 1);
        e.mem.set_u8(MENU_FLAG_TABLE + 0x3ea, 0);
        assert_eq!(e.call(0x0070_2680, &args![1u32, 0u32]).u8(), 0);
        // A plain id: the flag table; a mask tests the menu's state word.
        e.mem.set_u8(MENU_FLAG_TABLE + 0x3ee, 1);
        assert_eq!(e.call(0x0070_2680, &args![0x3eeu32, 0u32]).u8(), 1);
        stub!(e, GET_MENU_BY_CLASS, 0x1234);
        stub!(e, TILE_GET_MENU, 0x2345);
        stub!(e, MENU_STATE, 2);
        assert_eq!(e.call(0x0070_2680, &args![0x3eeu32, 4u32]).u8(), 0);
        stub!(e, MENU_STATE, 6);
        assert_eq!(e.call(0x0070_2680, &args![0x3eeu32, 4u32]).u8(), 1);
        // No menu tile for the id: the mask is not tested.
        stub!(e, GET_MENU_BY_CLASS, 0);
        stub!(e, MENU_STATE, 2);
        assert_eq!(e.call(0x0070_2680, &args![0x3eeu32, 4u32]).u8(), 1);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_2680, &args![0x3eeu32, 0u32]).u8(), 0);
    }

    #[test]
    fn fn_007027b0_reads_the_flag_table_inside_the_id_range() {
        let (mut e, _) = ui_engine();
        e.mem.set_u8(MENU_FLAG_TABLE + 0x3e9, 1);
        e.mem.set_u8(MENU_FLAG_TABLE + 0x43c, 1);
        e.mem.set_u8(MENU_FLAG_TABLE + 0x43d, 1);
        assert_eq!(e.call(0x0070_27b0, &args![0x3e9u32]).u8(), 1);
        assert_eq!(e.call(0x0070_27b0, &args![0x43cu32]).u8(), 1);
        assert_eq!(e.call(0x0070_27b0, &args![0x43du32]).u8(), 0);
        assert_eq!(e.call(0x0070_27b0, &args![0x3e8u32]).u8(), 0);
        assert_eq!(e.call(0x0070_27b0, &args![0x3eau32]).u8(), 0);
    }

    #[test]
    fn fn_007027e0_runs_pre_idle() {
        check_forwards_to_manager(0x0070_27e0, 0x0070_b8f0);
    }

    #[test]
    fn fn_00702810_runs_idle() {
        check_forwards_to_manager(0x0070_2810, 0x0070_c4a0);
    }

    #[test]
    fn fn_00702840_calls_on_the_ready_manager() {
        check_forwards_to_manager(0x0070_2840, 0x0071_1ea0);
    }

    #[test]
    fn fn_00702870_skips_while_a_movie_plays() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u32(MOVIE_PLAYER, 0x77);
        e.register(0x0071_3f00, |_, _| Ret::default());
        stub!(e, MOVIE_PLAYER_PLAYING, 1);
        e.call(0x0070_2870, &[]);
        assert!(logged(&e, 0x0071_3f00).is_empty());
        assert_eq!(logged(&e, MOVIE_PLAYER_PLAYING), vec![vec![0x77]]);
        stub!(e, MOVIE_PLAYER_PLAYING, 0);
        e.call(0x0070_2870, &[]);
        assert_eq!(logged(&e, 0x0071_3f00), vec![vec![manager]]);
    }

    #[test]
    fn click_forwards_the_click_or_sets_the_thread_stages() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager + 0x148, 1);
        stub!(e, MOVIE_PLAYER_PLAYING, 0);
        stub!(e, 0x0071_3fb0, 0);
        stub!(e, 0x004e_a970, 0x500);
        e.register(0x00ba_30f0, |_, _| Ret::default());
        e.register(0x00ba_3130, |_, _| Ret::default());
        e.call(0x0070_28b0, &args![10u32, 20u32]);
        assert_eq!(logged(&e, 0x0071_3fb0), vec![vec![manager, 10, 20]]);
        assert!(logged(&e, 0x004e_a970).is_empty());
        assert_eq!(e.mem.u8(manager + 0x148), 0);

        // A movie plays: the stages are set instead.
        e.mem.set_u8(manager + 0x148, 1);
        stub!(e, MOVIE_PLAYER_PLAYING, 1);
        e.call_log = Some(vec![]);
        e.call(0x0070_28b0, &args![10u32, 20u32]);
        assert!(logged(&e, 0x0071_3fb0).is_empty());
        assert_eq!(logged(&e, 0x004e_a970), vec![vec![0, 0x17], vec![1, 0x17]]);
        assert_eq!(logged(&e, 0x00ba_30f0), vec![vec![0x500]]);
        assert_eq!(logged(&e, 0x00ba_3130), vec![vec![0x500]]);
        assert_eq!(e.mem.u8(manager + 0x148), 0);
    }

    #[test]
    fn click_without_a_manager_sets_the_stages() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager, 0);
        stub!(e, 0x004e_a970, 0x500);
        e.register(0x00ba_30f0, |_, _| Ret::default());
        e.register(0x00ba_3130, |_, _| Ret::default());
        e.call(0x0070_28b0, &args![1u32, 2u32]);
        assert_eq!(logged(&e, 0x004e_a970).len(), 2);
    }

    #[test]
    fn fn_00702940_stores_the_click_flag() {
        let (mut e, manager) = ui_engine();
        e.call(0x0070_2940, &args![manager, 1u8]);
        assert_eq!(e.mem.u8(manager + 0x148), 1);
        e.call(0x0070_2940, &args![manager, 0u8]);
        assert_eq!(e.mem.u8(manager + 0x148), 0);
    }

    #[test]
    fn fn_00702960_polls_the_movie_player_when_ready() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u32(MOVIE_PLAYER, 0x77);
        stub!(e, MOVIE_PLAYER_PLAYING, 0);
        e.call(0x0070_2960, &[]);
        assert_eq!(logged(&e, MOVIE_PLAYER_PLAYING), vec![vec![0x77]]);
        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_2960, &[]);
        assert!(logged(&e, MOVIE_PLAYER_PLAYING).is_empty());
    }

    /// A menu tile whose child list (at `+4`) holds the given children.
    fn tile_with_children(e: &mut Engine, children: &[u32]) -> u32 {
        let tile = e.mem.alloc(16);
        let mut next = 0;
        for child in children.iter().rev() {
            next = list_node(e, next, *child);
        }
        e.mem.set_u32(tile + 4, next);
        tile
    }

    fn visible_calls(e: &Engine) -> Vec<(u32, u32)> {
        logged(e, TILE_SET_VALUE)
            .into_iter()
            .map(|w| {
                assert_eq!(w[1], TRAIT_VISIBLE);
                (w[0], w[2])
            })
            .collect()
    }

    #[test]
    fn fn_00702990_sets_the_visibility_of_the_menu_and_its_role_children() {
        let (mut e, manager) = ui_engine();
        list_doubles(&mut e);
        e.mem.set_f64(DOUBLE_110, 110.0);
        e.mem.set_f64(DOUBLE_111, 111.0);
        e.mem.set_f64(DOUBLE_112, 112.0);
        stub!(e, GET_MENUS_ROOT, 0x6000);
        e.register(TILE_SET_VALUE, |_, _| Ret::default());
        e.register(SUBOBJECT_ADD_TILE, |_, _| Ret::default());
        e.register(SET_NODE_FLAG, |_, _| Ret::default());
        e.register(TILE_NODE, |_, a| Ret {
            eax: a[0] + 1,
            ..Ret::default()
        });
        // Children: a role-110 tile, a role-111 tile and one with another role.
        let (child_a, child_b, child_c) = (0x1000, 0x2000, 0x3000);
        let tile = tile_with_children(&mut e, &[child_a, child_b, child_c]);
        tile_values(
            &mut e,
            &[
                ((child_a, TRAIT_ROLE), 110.0),
                ((child_b, TRAIT_ROLE), 111.0),
                ((child_c, TRAIT_ROLE), 5.0),
                // The enabled trait is set and the visible trait is zero for
                // the menu tile, nonzero for the others.
                ((tile, TRAIT_ENABLED), 1.0),
                ((child_a, TRAIT_ENABLED), 1.0),
                ((child_a, TRAIT_VISIBLE), 1.0),
                ((child_b, TRAIT_ENABLED), 1.0),
                ((child_b, TRAIT_VISIBLE), 1.0),
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x0070_2990, &args![tile, 1u8, 0u8, 1u8]);
        // The menu is made visible (b = 0 and not role 112), the 110 tile
        // visible, the 111 tile hidden.
        assert_eq!(
            visible_calls(&e),
            vec![(tile, 1), (child_a, 1), (child_b, 0)]
        );
        // The 110 tile is added to the manager's sub-object by address.
        let added = logged(&e, SUBOBJECT_ADD_TILE);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0][0], manager + MANAGER_SUBOBJECT);
        // Node flags: tile A (both traits set: false), tile B (a != 0: true),
        // the menu (visible trait zero: true).
        let flags: Vec<(u32, u32)> = logged(&e, SET_NODE_FLAG)
            .into_iter()
            .map(|w| (w[0], w[1]))
            .collect();
        assert_eq!(
            flags,
            vec![(child_a + 1, 0), (child_b + 1, 1), (tile + 1, 1)]
        );
    }

    #[test]
    fn fn_00702990_without_a_and_for_role_112() {
        let (mut e, _) = ui_engine();
        list_doubles(&mut e);
        e.mem.set_f64(DOUBLE_110, 110.0);
        e.mem.set_f64(DOUBLE_111, 111.0);
        e.mem.set_f64(DOUBLE_112, 112.0);
        stub!(e, GET_MENUS_ROOT, 0x6000);
        e.register(TILE_SET_VALUE, |_, _| Ret::default());
        e.register(SUBOBJECT_ADD_TILE, |_, _| Ret::default());
        e.register(SET_NODE_FLAG, |_, _| Ret::default());
        stub!(e, TILE_NODE, 0);
        let (child_a, child_b) = (0x1000, 0x2000);
        let tile = tile_with_children(&mut e, &[child_a, child_b]);
        tile_values(
            &mut e,
            &[
                ((tile, TRAIT_ROLE), 112.0),
                ((child_a, TRAIT_ROLE), 110.0),
                ((child_b, TRAIT_ROLE), 111.0),
            ],
        );
        e.call_log = Some(vec![]);
        // a = 0, b = 0, the menu has role 112: hidden; 110 hidden, 111 shown.
        e.call(0x0070_2990, &args![tile, 0u8, 0u8, 1u8]);
        assert_eq!(
            visible_calls(&e),
            vec![(tile, 0), (child_a, 0), (child_b, 1)]
        );
        assert!(logged(&e, SUBOBJECT_ADD_TILE).is_empty());
        // A null tile does nothing.
        e.call_log = Some(vec![]);
        e.call(0x0070_2990, &args![0u32, 0u8, 0u8, 1u8]);
        assert!(e.call_log.as_ref().unwrap().len() == 1);
    }

    /// A menus root with the given menu tiles in the list at `+4`.
    fn menus_root_with(e: &mut Engine, menus: &[u32]) -> u32 {
        tile_with_children(e, menus)
    }

    #[test]
    fn isolate_menu_elements_applies_the_menu_update_and_flags_the_nodes() {
        let (mut e, manager) = ui_engine();
        list_doubles(&mut e);
        e.mem.set_f64(DOUBLE_112, 112.0);
        e.mem.set_f64(DOUBLE_110, 110.0);
        e.mem.set_f64(DOUBLE_111, 111.0);
        let menu = tile_with_children(&mut e, &[]);
        let root = menus_root_with(&mut e, &[menu]);
        e.register(GET_MENUS_ROOT, |_, _| Ret::default());
        e.register_double(GET_MENUS_ROOT, move |_, _| Ret {
            eax: root,
            ..Ret::default()
        });
        e.register(TILE_SET_VALUE, |_, _| Ret::default());
        e.register(SUBOBJECT_CLEAR, |_, _| Ret::default());
        e.register(SET_NODE_FLAG, |_, _| Ret::default());
        stub!(e, TILE_NODE, 0);
        stub!(e, MANAGER_NODE_A, 0x900);
        stub!(e, MANAGER_NODE_B, 0);
        tile_values(&mut e, &[]);
        e.call_log = Some(vec![]);
        e.call(0x0070_2c80, &args![1u8, 0u8]);
        assert_eq!(logged(&e, SUBOBJECT_CLEAR), vec![vec![manager + 0x4c4]]);
        // The menu is visible (b = 0, role is not 112).
        assert_eq!(visible_calls(&e), vec![(menu, 1)]);
        // Only the first node getter returns a node; the flag is 1.
        assert_eq!(logged(&e, SET_NODE_FLAG), vec![vec![0x900, 1]]);

        // a = 0, b = 0: no clear, the node flag is 0.
        e.call_log = Some(vec![]);
        e.call(0x0070_2c80, &args![0u8, 0u8]);
        assert!(logged(&e, SUBOBJECT_CLEAR).is_empty());
        assert_eq!(logged(&e, SET_NODE_FLAG), vec![vec![0x900, 0]]);

        // Not ready: nothing.
        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_2c80, &args![1u8, 1u8]);
        assert!(logged(&e, SET_NODE_FLAG).is_empty());
    }

    #[test]
    fn restore_menu_elements_shows_the_menu_and_two_children() {
        let (mut e, manager) = ui_engine();
        list_doubles(&mut e);
        let menu = 0x4000;
        let root = menus_root_with(&mut e, &[menu]);
        e.register_double(GET_MENUS_ROOT, move |_, _| Ret {
            eax: root,
            ..Ret::default()
        });
        e.register(TILE_GET_CHILD_BY_ID, |_, a| Ret {
            eax: if a[1] == 0x6e { 0x5000 } else { 0 },
            ..Ret::default()
        });
        e.register(TILE_SET_VALUE, |_, _| Ret::default());
        e.register(SET_NODE_FLAG, |_, _| Ret::default());
        e.register(TILE_NODE, |_, a| Ret {
            eax: a[0] + 1,
            ..Ret::default()
        });
        // Menu: enabled and visible (flag false); child 0x6e: not enabled.
        tile_values(
            &mut e,
            &[
                ((menu, TRAIT_ENABLED), 1.0),
                ((menu, TRAIT_VISIBLE), 1.0),
                ((0x5000, TRAIT_ENABLED), 0.0),
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x0070_2dd0, &[]);
        assert_eq!(visible_calls(&e), vec![(menu, 1), (0x5000, 1)]);
        let flags: Vec<(u32, u32)> = logged(&e, SET_NODE_FLAG)
            .into_iter()
            .map(|w| (w[0], w[1]))
            .collect();
        assert_eq!(flags, vec![(menu + 1, 0), (0x5001, 1)]);
        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_2dd0, &[]);
        assert!(visible_calls(&e).is_empty());
    }

    #[test]
    fn is_debug_text_visible_forwards_the_answer() {
        let (mut e, manager) = ui_engine();
        stub!(e, 0x0064_4640, 1);
        assert!(e.call(0x0070_2fc0, &[]).bool());
        assert_eq!(logged(&e, 0x0064_4640), vec![vec![manager]]);
        stub!(e, 0x0064_4640, 0);
        assert!(!e.call(0x0070_2fc0, &[]).bool());
        let mut e = engine_without_manager();
        assert!(!e.call(0x0070_2fc0, &[]).bool());
    }

    #[test]
    fn toggle_debug_text_visible_flips_the_flag_when_ready() {
        let (mut e, manager) = ui_engine();
        e.call(0x0070_3000, &[]);
        assert_eq!(e.mem.u8(manager + 0x7c), 1);
        e.call(0x0070_3000, &[]);
        assert_eq!(e.mem.u8(manager + 0x7c), 0);
        e.mem.set_u8(manager, 0);
        e.call(0x0070_3000, &[]);
        assert_eq!(e.mem.u8(manager + 0x7c), 0);
    }

    #[test]
    fn fn_00703030_flips_the_debug_text_flag() {
        let (mut e, manager) = ui_engine();
        e.call(0x0070_3030, &args![manager]);
        assert_eq!(e.mem.u8(manager + 0x7c), 1);
        e.call(0x0070_3030, &args![manager]);
        assert_eq!(e.mem.u8(manager + 0x7c), 0);
    }

    #[test]
    fn fn_00703060_stores_the_debug_text_flag() {
        let (mut e, manager) = ui_engine();
        e.call(0x0070_3060, &args![manager, 1u8]);
        assert_eq!(e.mem.u8(manager + 0x7c), 1);
        e.call(0x0070_3060, &args![manager, 0u8]);
        assert_eq!(e.mem.u8(manager + 0x7c), 0);
    }

    #[test]
    fn fn_00703080_passes_two_floats() {
        let (mut e, manager) = ui_engine();
        e.register(0x0071_51b0, |_, _| Ret::default());
        e.call(0x0070_3080, &args![1.5f32, 2.5f32]);
        assert_eq!(
            logged(&e, 0x0071_51b0),
            vec![vec![manager, 1.5f32.to_bits(), 2.5f32.to_bits()]]
        );
        let mut e = engine_without_manager();
        e.register(0x0071_51b0, |_, _| Ret::default());
        e.call(0x0070_3080, &args![1.5f32, 2.5f32]);
        assert!(logged(&e, 0x0071_51b0).is_empty());
    }

    #[test]
    fn fn_007030c0_passes_two_words() {
        let (mut e, manager) = ui_engine();
        e.register(0x0071_5e00, |_, _| Ret::default());
        e.call(0x0070_30c0, &args![11u32, 22u32]);
        assert_eq!(logged(&e, 0x0071_5e00), vec![vec![manager, 11, 22]]);
        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_30c0, &args![11u32, 22u32]);
        assert!(logged(&e, 0x0071_5e00).is_empty());
    }

    #[test]
    fn toggle_full_help_flips_the_flag() {
        let (mut e, manager) = ui_engine();
        e.call(0x0070_3100, &[]);
        assert_eq!(e.mem.u8(manager + 0xdd), 1);
        e.mem.set_u8(manager + 0xdd, 7);
        e.call(0x0070_3100, &[]);
        assert_eq!(e.mem.u8(manager + 0xdd), 0);
        e.mem.set_u8(manager, 0);
        e.call(0x0070_3100, &[]);
        assert_eq!(e.mem.u8(manager + 0xdd), 0);
    }

    #[test]
    fn get_full_help_reads_the_flag() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager + 0xdd, 1);
        assert_eq!(e.call(0x0070_3150, &[]).u8(), 1);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_3150, &[]).u8(), 0);
    }

    #[test]
    fn get_target_refr_reads_the_pick_reference() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u32(manager + 0xf0, 0x1234);
        assert_eq!(e.call(0x0070_3180, &[]).u32(), 0x1234);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_3180, &[]).u32(), 0);
        let mut e = engine_without_manager();
        assert_eq!(e.call(0x0070_3180, &[]).u32(), 0);
    }

    #[test]
    fn fn_007031c0_reads_the_pick_reference() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u32(manager + 0xf0, 0x4321);
        assert_eq!(e.call(0x0070_31c0, &args![manager]).u32(), 0x4321);
    }

    #[test]
    fn fn_007031e0_prints_the_reference_as_debug_text() {
        let (mut e, manager) = ui_engine();
        let setting = e.mem.alloc(4);
        e.mem.set_u32(setting, 100);
        e.mem.set_f32(0x0103_a1c4, 640.0);
        e.mem.set_f32(0x0101_2054, -1.0);
        e.register(0x0040_37b0, |_, _| Ret::default());
        e.register(0x0040_37d0, |_, _| Ret::default());
        stub!(e, 0x0084_e3a0, 0xf00d);
        stub!(e, 0x0055_d520, 0x7000);
        // The formatter stores a character pointer in the first word.
        e.register(0x0040_6f60, |e, a| {
            e.mem.set_u32(a[0], 0xc0de);
            Ret::default()
        });
        e.register_double(0x0043_d4d0, move |_, _| Ret {
            eax: setting,
            ..Ret::default()
        });
        e.register(0x005b_d5b0, |_, _| Ret::default());
        e.register(0x005b_d5c0, |_, _| Ret::default());
        e.register(0x005b_d580, |_, _| Ret::default());
        stub!(e, 0x00ec_62c0, 0);
        stub!(e, 0x00a0_d9e0, 0xd00d);
        e.register(0x00a0_f8b0, |_, _| Ret::default());
        e.register(0x0071_4d70, |_, _| Ret::default());
        e.register(FIRST_WORD, |e, a| Ret {
            eax: e.mem.u32(a[0]),
            ..Ret::default()
        });
        e.call(0x0070_31e0, &args![0x9000u32]);
        let format = &logged(&e, 0x0040_6f60)[0];
        assert_eq!(&format[1..], &[0x0103_a1c8, 0x7000, 0xf00d]);
        assert_eq!(logged(&e, 0x005b_d5b0), vec![vec![99]]);
        assert_eq!(
            logged(&e, 0x00a0_f8b0),
            vec![vec![
                0xd00d,
                0xc0de,
                640.0f32.to_bits(),
                100.0f32.to_bits(),
                2,
                0xffff_ffff,
                (-1.0f32).to_bits(),
                0,
                0
            ]]
        );
        assert_eq!(logged(&e, 0x0071_4d70), vec![vec![manager, 0x9000]]);

        // A null reference prints nothing but still calls 00714d70.
        e.call_log = Some(vec![]);
        e.call(0x0070_31e0, &args![0u32]);
        assert!(logged(&e, 0x00a0_f8b0).is_empty());
        assert_eq!(logged(&e, 0x0071_4d70), vec![vec![manager, 0]]);

        // Not ready: nothing at all.
        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_31e0, &args![0x9000u32]);
        assert!(logged(&e, 0x0071_4d70).is_empty());
    }

    #[test]
    fn set_pick_refr_forwards_the_reference() {
        let (mut e, manager) = ui_engine();
        e.register(0x009b_df60, |_, _| Ret::default());
        e.call(0x0070_3310, &args![0x55u32]);
        assert_eq!(logged(&e, 0x009b_df60), vec![vec![manager, 0x55]]);
        e.mem.set_u8(manager, 0);
        e.call_log = Some(vec![]);
        e.call(0x0070_3310, &args![0x55u32]);
        assert!(logged(&e, 0x009b_df60).is_empty());
    }

    #[test]
    fn fn_00703350_forwards_the_answer() {
        let (mut e, manager) = ui_engine();
        stub!(e, 0x0087_4480, 0x66);
        assert_eq!(e.call(0x0070_3350, &[]).u32(), 0x66);
        assert_eq!(logged(&e, 0x0087_4480), vec![vec![manager]]);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_3350, &[]).u32(), 0);
    }

    #[test]
    fn get_telekinesis_refr_forwards_the_answer() {
        let (mut e, manager) = ui_engine();
        stub!(e, 0x009b_4440, 0x77);
        assert_eq!(e.call(0x0070_3390, &[]).u32(), 0x77);
        assert_eq!(logged(&e, 0x009b_4440), vec![vec![manager]]);
        e.mem.set_u8(manager, 0);
        assert_eq!(e.call(0x0070_3390, &[]).u32(), 0);
    }

    #[test]
    fn fn_007033d0_fills_the_point_or_copies_the_default() {
        let (mut e, manager) = ui_engine();
        let out = e.mem.alloc(12);
        e.register(0x009b_7010, |e, a| {
            e.mem.set_u32(a[1], 9);
            Ret::default()
        });
        let back = e.call(0x0070_33d0, &args![Ptr::<()>::new(out)]).u32();
        assert_eq!(back, out);
        assert_eq!(e.mem.u32(out), 9);
        assert_eq!(logged(&e, 0x009b_7010), vec![vec![manager, out]]);

        e.mem.set_u8(manager, 0);
        for (i, word) in [1u32, 2, 3].into_iter().enumerate() {
            e.mem.set_u32(0x011f_426c + 4 * i as u32, word);
        }
        let back = e.call(0x0070_33d0, &args![Ptr::<()>::new(out)]).u32();
        assert_eq!(back, out);
        assert_eq!(
            [e.mem.u32(out), e.mem.u32(out + 4), e.mem.u32(out + 8)],
            [1, 2, 3]
        );
    }

    #[test]
    fn fn_00703430_reads_the_fuzzy_pick_flag() {
        let (mut e, manager) = ui_engine();
        e.mem.set_u8(manager + 0x110, 1);
        assert!(e.call(0x0070_3430, &[]).bool());
        e.mem.set_u8(manager, 0);
        assert!(!e.call(0x0070_3430, &[]).bool());
    }

    #[test]
    fn fn_00703470_reads_the_fuzzy_pick_flag() {
        let (mut e, manager) = ui_engine();
        assert!(!e.call(0x0070_3470, &args![manager]).bool());
        e.mem.set_u8(manager + 0x110, 1);
        assert!(e.call(0x0070_3470, &args![manager]).bool());
    }
}
