//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE): the interface's tiles found by path (whether one is
//! loaded, its string, a click on it, its removal, a gradual float, an XML file read into it, a
//! template's tile added to it), the pointer and the menus (the cursor's place, the menu on top,
//! the active tile's name, path and id, the character creation menus, a menu's button pressed),
//! messages (disabled by script, their flags and display time), and the open menus' data (item
//! filters, the barter caps and price multipliers, the recipe menu's selection, category and
//! category count, the VATS targets, the font files, the level-up menu with skill points, the
//! barter offers and a menu's item list as inventory references, a terminal's own model). 31 of
//! the file's 64 commands; its four menu event commands are the core track's (`ui_hooks`).
//!
//! What JIP's tile helpers do is translated here (`nvse/GameTiles.cpp`, `nvse/GameUI.cpp` (JIP
//! LN NVSE)): a path's first part names a menu (looked up as JIP's `s_menuNameToID` looks it up,
//! by `StrHashCI` alone: `ui_hooks::MENU_NAMES`), each later part a child (`name`, `name:n` for the
//! `n`-th of that name, `*` for any), found among the children in their order with JIP's case-blind
//! compare; a last part that names no child names a trait, whose id comes from the game's trait
//! map (`0x11F32F4`, walked as JIP's `NiTMap::Lookup` walks it: 0 when it is not there). xNVSE's
//! own path helpers (`nvse::commands::ui`) walk the children differently (the first, then the last
//! back), so they are not used.
//!
//! The rest of the file waits (each named in the lane's audit):
//! - a hook or a code patch of JIP's: `InjectUIComponent` (writes `jip_temp.xml`: file writes),
//!   `UnlockRecipeMenuQuantity`, `ToggleCraftingMessages`, `EnableImprovedRecipeMenu`,
//!   `SuppressQuestMessages`, `InitItemFilter`, `SetItemFilter` (code bytes and jumps patched into
//!   the exe), `ShowTextInputMenu`, `SetTextInputExtendedProps`, `SetTextInputString` (the text
//!   input hooks, `TextInputClose`), `ToggleHUDCursor` (the HUD cursor and its input hooks);
//! - a routine of JIP's the game calls back: `ShowQuantityMenu`, `MessageBoxExAlt` (their
//!   callbacks), and `CloseActiveMenu`, which clears what those two keep;
//! - files and fonts: `SetFontFile` (a font file read from the folder, JIP's font map);
//! - 3D and drawing: `AttachUIXML`, `AttachUIComponent` (a tile's node under a reference's node),
//!   `ProjectUITile` (a rendered texture), `SetCursorPos` (the cursor's scene node);
//! - not yet built here (plain, for the next batch): `GetMenuTargetRef`, `GetSelectedItemRef`,
//!   `RefreshItemsList`, `MessageExAlt`, `SetSystemColor`, `ClearMessageQueue`,
//!   `TogglePipBoyLight`, `GetWorldMapPosMults`, `GetStringUIDimensions`.
//!
//! Where JIP reads through a null pointer (a menu's tile, a recipe menu without a category, a
//! trait path without a trait), the command does nothing here: listed in the lane's audit.

use super::arrays::{create_array, create_map, set_element, Element, TempElements};
use super::{
    assign_string, buffer_arg, cvttss2si, formatted_in_buffer, heap_alloc, heap_free, list_items,
    set_form_result, string_set, type_of, with_guest_text, Conditions, Handlers,
};
use crate::game::ListNode;
use crate::Jip;
use engine_core::extend::Hook;
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::Read;
use nvse::vars::Key;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2608, is_component_loaded),
    (0x2609, inject_ui_xml),
    (0x2638, get_cursor_pos),
    (0x2639, get_ui_string),
    (0x2681, get_active_menu_mode),
    (0x2682, get_active_ui_component_name),
    (0x2683, get_active_ui_component_full_name),
    (0x2684, get_active_ui_component_id),
    (0x2686, get_menu_item_filter),
    (0x2687, click_menu_button),
    (0x270e, get_barter_items),
    (0x2711, get_barter_gold_alt),
    (0x2712, get_recipe_menu_selection),
    (0x2713, get_recipe_menu_category),
    (0x2715, get_recipe_menu_num_subcategories),
    (0x274b, get_font_file),
    (0x2784, set_message_disabled),
    (0x2785, get_message_disabled),
    (0x2787, get_message_flags),
    (0x2788, set_message_flags),
    (0x278f, set_message_display_time),
    (0x27fa, get_barter_price_mult),
    (0x27fb, set_barter_price_mult),
    (0x2822, set_terminal_ui_model),
    (0x2827, get_vats_targets),
    (0x286c, unload_ui_component),
    (0x28a4, click_menu_tile),
    (0x28c6, add_tile_from_template),
    (0x28cb, set_ui_float_gradual),
    (0x28e1, show_level_up_menu_ex),
    (0x2935, get_menu_item_list_refs),
];

/// `IsInCharGen`, a condition (`DEFINE_CMD_COND_PLUGIN`).
pub static CONDITIONS: Conditions = &[(0x282a, is_in_char_gen, is_in_char_gen_eval)];

// ---- the game's objects and functions ----

/// `g_interfaceManager` (`*0x011D8A80`) and the fields read: `currentMode` (+0xC, its low byte
/// compared, engine-fallout-interface `cMenuMode`), the cursor's place (+0x38, +0x40: the drag
/// code reads them there, `interfacemanager.rs`), `activeTileAlt` (+0xBC, `pMouseOverTarget`),
/// `activeTile` (+0xCC, `pOverTileTarget`), `activeMenu` (+0xD0, `pOverTileMenu`), `menuStack`
/// (+0x114, `iEnterStack`; `InterfaceManager::AddToEnterStack` `00714D90` fills it).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const IM_CURRENT_MODE: u32 = 0x0c;
const IM_CURSOR_X: u32 = 0x38;
const IM_CURSOR_Y: u32 = 0x40;
const IM_ACTIVE_TILE_ALT: u32 = 0xbc;
const IM_ACTIVE_TILE: u32 = 0xcc;
const IM_ACTIVE_MENU: u32 = 0xd0;
const IM_MENU_STACK: u32 = 0x114;
/// The interface manager's size (`0x580`): the menu stack walk stays inside it.
const IM_SIZE: u32 = 0x580;
/// `g_tileMenuArray` (`*0x011F350C`, from `kMenuType_Min`) and the menus' shown bytes
/// (`0x011F308F + id`).
const TILE_MENU_ARRAY: u32 = 0x011f_350c;
const MENU_SHOWN: u32 = 0x011f_308f;
const MENU_MIN: u32 = 0x3e9;
const MENU_MAX: u32 = 0x43c;
/// The game's trait names map (`TraitNameMap`, `0x011F32F4`): vtable (+4 the hash, +8 the key
/// compare), bucket count +4, buckets +8; an item is next +0, key +4, id +8 (the game's own
/// lookup `00A1E380`, called by `Tile::TextToTrait` `00A01860` on this map).
const TRAIT_MAP: u32 = 0x011f_32f4;
/// A `Tile` (engine-bs `tile.rs`): children (`NiTPointerList`: head +4, a node next +0, tile +8),
/// trait values (`BSSimpleArray`: data +0x14, count +0x18, sorted by id), name (`BSStringT` +0x20:
/// text +0x20, length +0x24), parent +0x28. A value: id +0, number +8, string +0xC.
const TILE_CHILDREN: u32 = 0x04;
const TILE_VALUES: u32 = 0x14;
const TILE_VALUE_COUNT: u32 = 0x18;
const TILE_NAME: u32 = 0x20;
const TILE_PARENT: u32 = 0x28;
const VALUE_NUM: u32 = 0x08;
const VALUE_STR: u32 = 0x0c;
/// `TileMenu`'s vtable and its menu (+0x3C, engine-bs `tilemenu.rs` `pMenu`).
const TILE_MENU_VTABLE: u32 = 0x0106_ed44;
const TILE_MENU_MENU: u32 = 0x3c;
/// A menu's id (+0x20) and its click handler's vtable slot (`HandleClick`, +0xC).
const MENU_ID: u32 = 0x20;
const HANDLE_CLICK_SLOT: u32 = 0x0c;
/// `kTileValue_id`, `kTileValue_target`, `kTileValue_clicked`.
const TRAIT_ID: u32 = 0xfaa;
const TRAIT_TARGET: u32 = 0xfaf;
const TRAIT_CLICKED: u32 = 0xfc7;
/// `Tile::ReadFile` (`0x00A01B00`, `thiscall`: the tile, the path; the new tile), the value's
/// refresh (`Tile::Value::CalculateValue` `0x00A09410`, `thiscall`), `Tile::AddFadeControl`
/// (`0x00A07C60`: the tile, the id, start, end, seconds, the mode) and `RemoveFadeControl`
/// (`0x00A07DC0`: the tile, the id), `Menu::RenderTemplate` (`0x00A1DDB0`: the menu, the parent
/// tile, the template's name, 0).
const TILE_READ_FILE: u32 = 0x00a0_1b00;
const VALUE_REFRESH: u32 = 0x00a0_9410;
const FADE_ADD: u32 = 0x00a0_7c60;
const FADE_REMOVE: u32 = 0x00a0_7dc0;
const RENDER_TEMPLATE: u32 = 0x00a1_ddb0;
/// The menus' singletons (engine-fallout-interface: `bartermenu.rs`, `containermenu.rs`,
/// `inventorymenu.rs`, `recipemenu.rs`, `vatsmenu.rs`, `levelupmenu.rs`, `repairmenu.rs`,
/// `repairservicesmenu.rs`).
const BARTER_MENU: u32 = 0x011d_8fa4;
const CONTAINER_MENU: u32 = 0x011d_93f8;
const INVENTORY_MENU: u32 = 0x011d_9ea4;
const RECIPE_MENU: u32 = 0x011d_8e90;
const RECIPE_SELECTION: u32 = 0x011d_8e94;
const RECIPE_CATEGORIES: u32 = 0x011d_8f08;
const VATS_MENU: u32 = 0x011d_b0d4;
const VATS_TARGETS: u32 = 0x011d_b150;
const LEVEL_UP_MENU: u32 = 0x011d_9fdc;
const REPAIR_MENU: u32 = 0x011d_a75c;
const REPAIR_SERVICES_MENU: u32 = 0x011d_a7f0;
/// `BarterMenu`: the merchant +0x80, its caps +0x90, the buy and sell multipliers +0x94, +0x98,
/// the filters +0x9C, +0xA0, the item lists +0xA8, +0xD8, the offers +0x10C, +0x114.
const BARTER_MERCHANT: u32 = 0x80;
const BARTER_GOLD: u32 = 0x90;
const BARTER_BUY_MULT: u32 = 0x94;
const BARTER_SELL_MULT: u32 = 0x98;
const BARTER_LEFT_FILTER: u32 = 0x9c;
const BARTER_RIGHT_FILTER: u32 = 0xa0;
const BARTER_LEFT_ITEMS: u32 = 0xa8;
const BARTER_RIGHT_ITEMS: u32 = 0xd8;
const BARTER_LEFT_OFFER: u32 = 0x10c;
const BARTER_RIGHT_OFFER: u32 = 0x114;
/// `ContainerMenu`: the container +0x74, the filters +0x8C, +0x90, the item lists +0x98, +0xC8.
const CONTAINER_REF: u32 = 0x74;
const CONTAINER_LEFT_FILTER: u32 = 0x8c;
const CONTAINER_RIGHT_FILTER: u32 = 0x90;
const CONTAINER_LEFT_ITEMS: u32 = 0x98;
const CONTAINER_RIGHT_ITEMS: u32 = 0xc8;
/// `InventoryMenu`: the filter (`iCurrTab`) +0x84, the item list +0xB8; `RepairMenu`'s list
/// +0x5C, `RepairServicesMenu`'s +0x68; `RecipeMenu`'s category +0x64; `LevelUpMenu`'s skill
/// points to assign +0x5C (`iMaxPoints`).
const INVENTORY_FILTER: u32 = 0x84;
const INVENTORY_ITEMS: u32 = 0xb8;
const REPAIR_ITEMS: u32 = 0x5c;
const REPAIR_SERVICES_ITEMS: u32 = 0x68;
const RECIPE_CATEGORY: u32 = 0x64;
const LEVEL_UP_SKILL_POINTS: u32 = 0x5c;
/// A list box's items (`BSSimpleList` of `ListBoxItem *`, first node at +4); an item: tile +0,
/// object +4, filtered byte +8 (engine-fallout-interface `listbox.rs`).
const LIST_BOX_LIST: u32 = 0x04;
const LIST_ITEM_OBJECT: u32 = 0x04;
const LIST_ITEM_FILTERED: u32 = 0x08;
/// The barter refresh JIP calls after a price change (`0x00730690`, `cdecl`, 1).
const BARTER_REFRESH: u32 = 0x0073_0690;
/// `LevelUpMenu::Create` (`0x00784C80`) and `LevelUpMenu::SetCurrentPage` (`0x00785830`).
const LEVEL_UP_CREATE: u32 = 0x0078_4c80;
const LEVEL_UP_SET_PAGE: u32 = 0x0078_5830;
/// `g_fontManager` (`*0x011F33F8`): the fonts by number from +0 (font `n` at `4 * (n - 1)`, as
/// engine-bs `debugtext.rs` reads them); a font's file path +4 (`Font` `pFontFile`, written by
/// its constructor `00A12020`).
const FONT_MANAGER: u32 = 0x011f_33f8;
const FONT_FILE_PATH: u32 = 0x04;
/// `g_thePlayer`.
const PLAYER: u32 = 0x011d_ea3c;
/// `ExtraMerchantContainer` (type 0x3C, the container +0xC: `ExtraDataList::GetMerchantContainer`
/// `00421400`).
const EXTRA_MERCHANT_CONTAINER: u32 = 0x3c;
const MERCHANT_CONTAINER_REF: u32 = 0x0c;
/// A `ContChangesEntry`: its extra data lists +0, count +4, form +8.
const ENTRY_EXTEND_DATA: u32 = 0x00;
const ENTRY_COUNT: u32 = 0x04;
const ENTRY_FORM: u32 = 0x08;
/// `BGSMessage`: the flags (`DNAM`) +0x38, the display time (`TNAM`) +0x3C (the forms crate's
/// `bgsmessage.rs` loader).
const MESSAGE_FLAGS: u32 = 0x38;
const MESSAGE_DISPLAY_TIME: u32 = 0x3c;
/// `kHookFormFlag6_MessageDisabled`.
const MESSAGE_DISABLED: u8 = 1;
/// The terminal model's path (`GameGlobals::TerminalModelPtr`, `0x011A0BB0`) and
/// `FORenderedTerminal::ReleaseStaticGeometry` (JIP's `PurgeTerminalModel`, `0x007FFE00`).
const TERMINAL_MODEL_PATH: u32 = 0x011a_0bb0;
const PURGE_TERMINAL_MODEL: u32 = 0x007f_fe00;
/// Form types: `BGSTerminal`, `BGSListForm`, `BGSMessage`.
const TERMINAL: u8 = 0x17;
const LIST_FORM: u8 = 0x55;
const MESSAGE: u8 = 0x62;
/// `BGSListForm`'s list (first node inline at +0x18).
const LIST_FORM_LIST: u32 = 0x18;
/// Menu ids.
const INVENTORY: u32 = 0x3ea;
const STATS: u32 = 0x3eb;
const CONTAINER: u32 = 0x3f0;
const MAP: u32 = 0x3ff;
const REPAIR: u32 = 0x40b;
const CHAR_GEN: u32 = 0x418;
const BARTER: u32 = 0x41d;
const REPAIR_SERVICES: u32 = 0x422;
const SPECIAL_BOOK: u32 = 0x424;
const ITEM_MOD: u32 = 0x425;
const LOVE_TESTER: u32 = 0x432;
const TRAIT_MENU: u32 = 0x43c;
/// The most nodes a child list or a map bucket is walked through, and the most parents a tile
/// path is followed up (a broken list ends there).
const MAX_WALK: u32 = 1 << 16;
/// JIP's `tilePath[0x100]`, `xmlPath[0x80]`, `modelPath[0x80]`, `AddTileFromTemplate`'s
/// `buffer[0x100]` (`ExtractFormatStringArgs` into it).
const PATH_BUFFER: usize = 0x100;
const XML_BUFFER: usize = 0x80;
/// `kCommandInfo_AddTileFromTemplate.numParams`: xNVSE's 21 format parameters.
const FORMAT_STRING_PARAMS: u32 = 21;
/// `Tile::Parents[24]` in `GetComponentFullName`: deeper tiles run past JIP's array.
const FULL_NAME_DEPTH: u32 = 24;

fn interface_manager(e: &Engine) -> u32 {
    e.mem.u32(INTERFACE_MANAGER)
}

/// `NUM_ARGS` (JIP LN NVSE): the argument count byte at the start of the arguments.
fn num_args(e: &Engine, a: &CommandArgs) -> u8 {
    e.mem.u8(a.data.wrapping_add(e.mem.u32(a.opcode_offset)))
}

// ---- tiles and paths ----

/// `SlashPos` (JIP LN NVSE): the first `/` or `\`.
fn slash_pos(path: &[u8]) -> Option<usize> {
    path.iter().position(|&b| b == b'/' || b == b'\\')
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: GetNextToken (one delimiter).
/// The text up to the first `delim` (the token), and what follows the run of `delim`s after it
/// (empty when there is no `delim`).
fn next_token(text: &[u8], delim: u8) -> (&[u8], &[u8]) {
    match text.iter().position(|&b| b == delim) {
        None => (text, &[]),
        Some(at) => {
            let mut rest = at;
            while rest < text.len() && text[rest] == delim {
                rest += 1;
            }
            (&text[..at], &text[rest..])
        }
    }
}

/// The menu `id`'s tile (`g_tileMenuArray[id]`), 0 for none.
pub(crate) fn menu_tile_by_id(e: &Engine, id: u32) -> u32 {
    let array = e.mem.u32(TILE_MENU_ARRAY);
    if array == 0 || !(MENU_MIN..=MENU_MAX).contains(&id) {
        return 0;
    }
    e.mem.u32(array + 4 * (id - MENU_MIN))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameUI.cpp: GetMenuTile.
/// The tile of the menu named `name` (`s_menuNameToID`, by `StrHashCI` alone), 0 for none.
fn menu_tile(e: &Engine, name: &[u8]) -> u32 {
    let hash = crate::containers::str_hash_ci(name);
    crate::ui_hooks::MENU_NAMES
        .iter()
        .find(|(n, _)| crate::containers::str_hash_ci(n.as_bytes()) == hash)
        .map_or(0, |&(_, id)| menu_tile_by_id(e, id))
}

/// A tile's name text (up to its 0; empty without a buffer).
fn tile_name(e: &Engine, tile: u32) -> Option<Vec<u8>> {
    let data = e.mem.u32(tile + TILE_NAME);
    (data != 0).then(|| nvse::table::guest_text(e, data))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::GetChild.
/// The child named `child` (`*` for any, `name:n` for the `n`-th such), the children in their
/// order; 0 for none (and for a name that starts with the colon).
fn child(e: &Engine, tile: u32, child: &[u8]) -> u32 {
    let (name, mut index) = match child.iter().position(|&b| b == b':') {
        Some(0) => return 0,
        Some(at) => (&child[..at], crate::utility::str_to_int(&child[at + 1..])),
        None => (child, 0),
    };
    let any = name.first() == Some(&b'*');
    let mut node = e.mem.u32(tile + TILE_CHILDREN);
    for _ in 0..MAX_WALK {
        if node == 0 {
            break;
        }
        let candidate = e.mem.u32(node + 8);
        if candidate != 0 {
            // StrCompareCI with a null name answers "less": no match.
            let matches = any
                || tile_name(e, candidate)
                    .is_some_and(|n| crate::utility::str_compare_ci(&n, name).0 == 0);
            if matches {
                if index == 0 {
                    return candidate;
                }
                index = index.wrapping_sub(1);
            }
        }
        node = e.mem.u32(node);
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::GetComponent.
/// The tile at `path` below `tile` and, when its last part names no child, that part as a
/// trait name; `None` when a tile on the way is not there.
fn component<'a>(e: &Engine, tile: u32, path: &'a [u8]) -> Option<(u32, Option<&'a [u8]>)> {
    let mut parent = tile;
    let mut rest = path;
    while let Some(slash) = slash_pos(rest) {
        parent = child(e, parent, &rest[..slash]);
        if parent == 0 {
            return None;
        }
        rest = &rest[slash + 1..];
    }
    if !rest.is_empty() {
        let found = child(e, parent, rest);
        if found != 0 {
            return Some((found, None));
        }
        return Some((parent, Some(rest)));
    }
    Some((parent, None))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.h: NiTMap::Lookup (on the
// game's trait names map, nvse/GameTiles.cpp: Tile::TraitNameToID).
/// The trait id named `name`: the map's hash picks the bucket, its compare finds the key; 0 when
/// it is not there.
fn trait_id(e: &mut Engine, name: &[u8]) -> u32 {
    let vtable = e.mem.u32(TRAIT_MAP);
    if vtable == 0 {
        return 0;
    }
    with_guest_text(e, name, |e, key| {
        let (hash, equal) = (e.mem.u32(vtable + 4), e.mem.u32(vtable + 8));
        let bucket = e.call(hash, &args![TRAIT_MAP, key]).u32();
        let buckets = e.mem.u32(TRAIT_MAP + 8);
        let mut item = e.mem.u32(buckets.wrapping_add(bucket.wrapping_mul(4)));
        for _ in 0..MAX_WALK {
            if item == 0 {
                return 0;
            }
            let item_key = e.mem.u32(item + 4);
            if e.call(equal, &args![TRAIT_MAP, key, item_key]).u32() & 0xff != 0 {
                return e.mem.u32(item + 8);
            }
            item = e.mem.u32(item);
        }
        0
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::GetValue (assembly).
/// The tile's value of trait `id` (a binary search of the sorted values), 0 for none.
fn value_of(e: &Engine, tile: u32, id: u32) -> u32 {
    let data = e.mem.u32(tile + TILE_VALUES);
    let (mut lo, mut hi) = (0u32, e.mem.u32(tile + TILE_VALUE_COUNT));
    while lo != hi {
        let mid = lo.wrapping_add(hi) >> 1;
        let value = e.mem.u32(data.wrapping_add(4 * mid));
        let at = e.mem.u32(value);
        if at == id {
            return value;
        }
        if at < id {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    0
}

/// The value's number (0 for no value: `Tile::GetValueFloat`).
fn value_float(e: &Engine, tile: u32, id: u32) -> f32 {
    let value = value_of(e, tile, id);
    if value == 0 {
        0.0
    } else {
        e.mem.f32(value + VALUE_NUM)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: GetTargetComponent.
/// The tile at `path` (`Menu/tile/...`; a path without a slash is the menu's own tile). With
/// `want_value`, the path must end at a trait the tile has, and its value comes too; without, it
/// must end at a tile. A path without a slash gives the menu's tile and no value either way.
fn target_component(e: &mut Engine, path: &[u8], want_value: bool) -> Option<(u32, u32)> {
    let Some(slash) = slash_pos(path) else {
        let tile = menu_tile(e, path);
        return (tile != 0).then_some((tile, 0));
    };
    let menu = menu_tile(e, &path[..slash]);
    if menu == 0 {
        return None;
    }
    let (tile, trait_name) = component(e, menu, &path[slash + 1..])?;
    match trait_name {
        Some(name) => {
            if !want_value {
                return None;
            }
            let id = trait_id(e, name);
            let value = value_of(e, tile, id);
            (value != 0).then_some((tile, value))
        }
        None if want_value => None,
        None => Some((tile, 0)),
    }
}

/// Whether `tile` is a `TileMenu` (its vtable).
fn is_tile_menu(e: &Engine, tile: u32) -> bool {
    e.mem.u32(tile) == TILE_MENU_VTABLE
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::GetParentMenu.
/// The menu of the first `TileMenu` from `tile` up, 0 for none.
fn parent_menu(e: &Engine, tile: u32) -> u32 {
    let mut t = tile;
    for _ in 0..MAX_WALK {
        if t == 0 {
            break;
        }
        if is_tile_menu(e, t) {
            return e.mem.u32(t + TILE_MENU_MENU);
        }
        t = e.mem.u32(t + TILE_PARENT);
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::Value::SetFloat.
/// The value's number set (`comiss`: nothing when it is equal, or when either is a NaN) and
/// the value refreshed.
fn value_set_float(e: &mut Engine, value: u32, num: f32) {
    let old = e.mem.f32(value + VALUE_NUM);
    if num.is_nan() || old.is_nan() || num == old {
        return;
    }
    e.mem.set_f32(value + VALUE_NUM, num);
    e.call(VALUE_REFRESH, &args![value, 1u32]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::Value::SetBool.
/// The value's number set to 1.0 or 0 (by its bits) and the value refreshed, unless it has it.
fn value_set_bool(e: &mut Engine, value: u32, on: bool) {
    let bits = if on { 0x3f80_0000 } else { 0 };
    if e.mem.u32(value + VALUE_NUM) == bits {
        return;
    }
    e.mem.set_u32(value + VALUE_NUM, bits);
    e.call(VALUE_REFRESH, &args![value, 1u32]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::PokeValue.
/// The tile's value of `id` set true then false (a click), when the tile has it.
fn poke_value(e: &mut Engine, tile: u32, id: u32) {
    let value = value_of(e, tile, id);
    if value != 0 {
        value_set_bool(e, value, true);
        value_set_bool(e, value, false);
    }
}

/// The interface manager's active tile (`GetActiveTile`: `activeTile`, else `activeTileAlt`).
fn active_tile(e: &Engine) -> u32 {
    let im = interface_manager(e);
    if im == 0 {
        return 0;
    }
    let tile = e.mem.u32(im + IM_ACTIVE_TILE);
    if tile != 0 {
        tile
    } else {
        e.mem.u32(im + IM_ACTIVE_TILE_ALT)
    }
}

/// The active tile's path (`Tile::GetComponentFullName`, `ui_hooks::component_full_name`); `None`
/// without an active tile, or when the tile is not under a menu tile within JIP's 24 levels (JIP
/// runs past its array there).
fn active_full_name(e: &Engine) -> Option<Vec<u8>> {
    let tile = active_tile(e);
    if tile == 0 {
        return None;
    }
    // Each tile's parent's children are walked too (the menu tile's: the interface's root):
    // every tile on the way must have one.
    let mut t = tile;
    for _ in 0..FULL_NAME_DEPTH {
        let parent = e.mem.u32(t + TILE_PARENT);
        if parent == 0 {
            return None;
        }
        if is_tile_menu(e, t) {
            return Some(crate::ui_hooks::component_full_name(e, tile));
        }
        t = parent;
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameUI.cpp:
// InterfaceManager::GetTopVisibleMenuID.
/// Not in menu mode (the mode's low byte below 2): 0. Else the menu under the pointer's id, or
/// the top of the menu stack; the Pip-Boy (1) answers the shown one of its inventory, map, stats,
/// repair and item mod menus, else 0.
fn top_visible_menu_id(e: &Engine) -> u32 {
    let im = interface_manager(e);
    if im == 0 || e.mem.u8(im + IM_CURRENT_MODE) < 2 {
        return 0;
    }
    let menu = e.mem.u32(im + IM_ACTIVE_MENU);
    if menu != 0 {
        return e.mem.u32(menu + MENU_ID);
    }
    // From the second entry to the first 0; the entry before it is the top. JIP reads on past the
    // ten entries when all are set: here the walk stays inside the manager.
    let mut at = im + IM_MENU_STACK + 4;
    while at < im + IM_SIZE - 4 && e.mem.u32(at) != 0 {
        at += 4;
    }
    let top = e.mem.u32(at - 4);
    if top != 1 {
        return top;
    }
    for id in [INVENTORY, MAP, STATS, REPAIR, ITEM_MOD] {
        if e.mem.u8(MENU_SHOWN + id) != 0 {
            return id;
        }
    }
    0
}

/// `GetMenuMode` (JIP LN NVSE): menu mode (`currentMode > 1`) or JIP's HUD cursor mode, which only
/// `ToggleHUDCursor` (not built) turns on: off here.
fn menu_mode(e: &Engine) -> bool {
    let im = interface_manager(e);
    im != 0 && e.mem.u32(im + IM_CURRENT_MODE) > 1
}

/// The path argument as JIP's `tilePath[0x100]` holds it.
fn path_arg(e: &Engine, read: &Read, i: usize, command: &str) -> Vec<u8> {
    buffer_arg(e, read, i, PATH_BUFFER, command)
}

// ---- inventory references ----

/// A reference's merchant container (`GetMerchantContainer`), 0 for none.
fn merchant_container(e: &mut Engine, reference: u32) -> u32 {
    if reference == 0 {
        return 0;
    }
    let extra = super::extra::refr_extra(e, reference, EXTRA_MERCHANT_CONTAINER);
    if extra == 0 {
        0
    } else {
        e.mem.u32(extra + MERCHANT_CONTAINER_REF)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: CreateRefForStack.
/// An inventory reference for the menu entry's stack in `container` (xNVSE's
/// `InventoryRefCreateEntry`: the entry's form and count, its first extra data list).
fn create_ref_for_stack(e: &mut Engine, container: u32, entry: u32) -> u32 {
    let form = e.mem.u32(entry + ENTRY_FORM);
    let count = e.mem.u32(entry + ENTRY_COUNT) as i32;
    let lists = e.mem.u32(entry + ENTRY_EXTEND_DATA);
    let xdata = if lists == 0 { 0 } else { e.mem.u32(lists) };
    nvse::invref::create_entry(e, container, form, count, xdata)
}

// ---- tile commands ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_IsComponentLoaded_Execute.
/// 1 when the path names a tile.
fn is_component_loaded(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let path = path_arg(e, &read, 0, "IsComponentLoaded");
        if target_component(e, &path, false).is_some() {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_InjectUIXML_Execute.
/// The XML file read into the tile at the path (`Tile::ReadFile`); 1 when it made a tile.
fn inject_ui_xml(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let path = path_arg(e, &read, 0, "InjectUIXML");
        let xml = buffer_arg(e, &read, 1, XML_BUFFER, "InjectUIXML");
        if let Some((tile, _)) = target_component(e, &path, false) {
            let made = with_guest_text(e, &xml, |e, at| {
                e.call(TILE_READ_FILE, &args![tile, at]).u32()
            });
            if made != 0 {
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetUIString_Execute.
/// The string of the trait at the path (`""` for none).
fn get_ui_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut text = None;
    if read.ok {
        let path = path_arg(e, &read, 0, "GetUIString");
        if let Some((_, value)) = target_component(e, &path, true) {
            if value != 0 {
                let s = e.mem.u32(value + VALUE_STR);
                if s != 0 {
                    text = Some(nvse::table::guest_text(e, s));
                }
            }
        }
    }
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_ClickMenuTile_Execute.
/// In menu mode, a tile at the path whose `target` is above 0 gets a click (`clicked` poked);
/// result 1.
fn click_menu_tile(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let path = path_arg(e, &read, 0, "ClickMenuTile");
    if menu_mode(e) {
        if let Some((tile, _)) = target_component(e, &path, false) {
            if value_float(e, tile, TRAIT_TARGET) > 0.0 {
                poke_value(e, tile, TRAIT_CLICKED);
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_UnloadUIComponent_Execute.
/// The tile at the path destroyed (its deleting destructor, vtable slot 0, with 1).
fn unload_ui_component(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let path = path_arg(e, &read, 0, "UnloadUIComponent");
        if let Some((tile, _)) = target_component(e, &path, false) {
            let destroy = e.mem.u32(e.mem.u32(tile));
            e.call(destroy, &args![tile, 1u32]);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetUIFloatGradual_Execute.
/// With four arguments or more, the trait at the path fades from the start to the end value
/// over the time, in the mode (0-4 as the game's 0, 4, 1, 5, 6; above 4 as 0). With fewer, its
/// fade ends, and with two or three it is set to the start value.
fn set_ui_float_gradual(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let count = num_args(e, &a);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let path = path_arg(e, &read, 0, "SetUIFloatGradual");
    let Some((tile, value)) = target_component(e, &path, true) else {
        return handled(true);
    };
    // A path without a slash gives a menu's tile and no value: JIP reads the id through the null
    // value there; nothing here.
    if value == 0 {
        return handled(true);
    }
    let id = e.mem.u32(value);
    let start = read.float(1, 0.0);
    if count >= 4 {
        let (end, timer) = (read.float(2, 0.0), read.float(3, 0.0));
        let mode = match read.int(4, 0) {
            m @ 0..=4 => [0u32, 4, 1, 5, 6][m as usize],
            _ => 0,
        };
        e.call(
            FADE_ADD,
            &args![
                tile,
                id,
                start.to_bits(),
                end.to_bits(),
                timer.to_bits(),
                mode
            ],
        );
    } else {
        e.call(FADE_REMOVE, &args![tile, id]);
        if count >= 2 {
            value_set_float(e, value, start);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_AddTileFromTemplate_Execute.
/// `Menu/tile...|template|name`: the template's tile added under the tile (the menu's own for a
/// path without a slash; none when the path ends at a trait), named `name` when one is given;
/// result 1 when the menu made it.
fn add_tile_from_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 0, FORMAT_STRING_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let buffer = formatted_in_buffer(e, &f.text, PATH_BUFFER, "AddTileFromTemplate");
    let (path, rest) = next_token(&buffer, b'|');
    if rest.is_empty() {
        return handled(true);
    }
    let (template, alt_name) = next_token(rest, b'|');
    let (menu, tile) = match slash_pos(path) {
        Some(slash) => {
            let menu = menu_tile(e, &path[..slash]);
            if menu == 0 {
                return handled(true);
            }
            match component(e, menu, &path[slash + 1..]) {
                Some((_, Some(_))) => return handled(true),
                Some((tile, None)) => (menu, tile),
                None => (menu, 0),
            }
        }
        None => {
            let menu = menu_tile(e, path);
            (menu, menu)
        }
    };
    if tile == 0 {
        return handled(true);
    }
    // JIP calls the menu tile's menu as it is: a menu tile without its menu is passed over here.
    let owner = e.mem.u32(menu + TILE_MENU_MENU);
    if owner == 0 {
        return handled(true);
    }
    let made = with_guest_text(e, template, |e, at| {
        e.call(RENDER_TEMPLATE, &args![owner, tile, at, 0u32]).u32()
    });
    if made != 0 {
        a.set_result(e, 1.0);
        if !alt_name.is_empty() {
            string_set(e, made + TILE_NAME, alt_name);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_ClickMenuButton_Execute.
/// In menu mode, `times` clicks (0: none) on a menu's tile: `Menu#id` clicks tile id `id` (not
/// below 0) of the menu named before the `#`; else the tile at the path, its own `id` trait's
/// number (-1 without one), through its menu. Each click is the menu's click handler (its
/// vtable's, so JIP's click events run too); result 1.
fn click_menu_button(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let path = path_arg(e, &read, 0, "ClickMenuButton");
    let times = read.int(1, 1);
    if !menu_mode(e) || times == 0 {
        return handled(true);
    }
    let (mut tile, mut menu, mut tile_id) = (0u32, 0u32, -1i32);
    if let Some(hash) = path.iter().position(|&b| b == b'#') {
        tile_id = crate::utility::str_to_int(&path[hash + 1..]);
        if tile_id >= 0 {
            let menu_tile = menu_tile(e, &path[..hash]);
            if menu_tile != 0 {
                menu = e.mem.u32(menu_tile + TILE_MENU_MENU);
            }
        }
    } else {
        let Some((found, _)) = target_component(e, &path, false) else {
            return handled(true);
        };
        tile = found;
        menu = parent_menu(e, tile);
        let value = value_of(e, tile, TRAIT_ID);
        if value != 0 {
            tile_id = cvttss2si(e.mem.f32(value + VALUE_NUM));
        }
    }
    if menu != 0 {
        for _ in 0..times {
            let handler = e.mem.u32(e.mem.u32(menu) + HANDLE_CLICK_SLOT);
            e.call(handler, &args![menu, tile_id as u32, tile]);
        }
        a.set_result(e, 1.0);
    }
    handled(true)
}

// ---- the interface manager ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetCursorPos_Execute.
/// The cursor's x for axis `X`, else its y.
fn get_cursor_pos(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let im = interface_manager(e);
    if read.ok && im != 0 {
        let axis = match read.args.first() {
            Some(Arg::Byte(b)) => *b,
            _ => 0,
        };
        let at = if axis == b'X' {
            IM_CURSOR_X
        } else {
            IM_CURSOR_Y
        };
        a.set_result(e, f64::from(e.mem.f32(im + at)));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetActiveMenuMode_Execute.
/// The top visible menu's id (0 outside menu mode).
fn get_active_menu_mode(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let id = top_visible_menu_id(e) as i32;
    a.set_result(e, f64::from(id));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetActiveUIComponentName_Execute.
/// The active tile's path after its last `/` (`""` without one, or for a menu's own tile).
fn get_active_ui_component_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let name = active_full_name(e).and_then(|path| {
        let slash = path.iter().rposition(|&b| b == b'/')?;
        Some(path[slash + 1..].to_vec())
    });
    assign_string(e, &a, name);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetActiveUIComponentFullName_Execute.
/// The active tile's whole path (`""` without one).
fn get_active_ui_component_full_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let path = active_full_name(e).unwrap_or_default();
    assign_string(e, &a, Some(path));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: GetActiveTileID,
// Cmd_GetActiveUIComponentID_Execute.
/// The active tile's `id` trait (its number as `SInt32`, truncated), -1 without one.
fn get_active_ui_component_id(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let tile = active_tile(e);
    let mut id = -1;
    if tile != 0 {
        let value = value_of(e, tile, TRAIT_ID);
        if value != 0 {
            id = cvttss2si(e.mem.f32(value + VALUE_NUM));
        }
    }
    a.set_result(e, f64::from(id));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: InCharGen.
/// Whether the character creation, SPECIAL book, love tester or trait menu has its tile.
fn in_char_gen(e: &Engine) -> bool {
    [CHAR_GEN, SPECIAL_BOOK, LOVE_TESTER, TRAIT_MENU]
        .into_iter()
        .any(|id| menu_tile_by_id(e, id) != 0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_IsInCharGen_Execute.
fn is_in_char_gen(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if in_char_gen(e) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: Cmd_IsInCharGen_Eval.
fn is_in_char_gen_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if in_char_gen(e) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// ---- messages ----

/// A form list's forms, or the form alone (`tList<TESForm> tempList(form)`).
fn form_or_list(e: &Engine, form: u32) -> Vec<u32> {
    if form != 0 && type_of(e, form) == LIST_FORM {
        list_items(e, Ptr::<ListNode>::new(form + LIST_FORM_LIST))
    } else {
        vec![form]
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetMessageDisabled_Execute.
/// The message (or each message of a form list) disabled or enabled (`SetJIPFlag`); JIP's
/// message hook then has its user.
fn set_message_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let disable = read.int(1, 0) != 0;
    for form in form_or_list(e, read.form(0)) {
        if form != 0 && type_of(e, form) == MESSAGE {
            crate::world_hooks::set_jip_flag(e, form, MESSAGE_DISABLED, disable);
        }
    }
    e.set_hook(Hook::MessageDisabled, true);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetMessageDisabled_Execute.
fn get_message_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let flags = e.get(
            Ptr::<crate::game::JipFormFlags>::new(read.form(0)),
            crate::game::JipFormFlags::jipFormFlags6,
        );
        if flags & MESSAGE_DISABLED != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetMessageFlags_Execute.
/// The message's flag byte (1 a message box, 2 auto-display).
fn get_message_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let flags = e.mem.u8(read.form(0) + MESSAGE_FLAGS);
        a.set_result(e, f64::from(flags));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetMessageFlags_Execute.
/// The flag byte set, for flags 0 to 3.
fn set_message_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flags = read.int(1, 0);
    if read.ok && flags <= 3 {
        e.mem.set_u8(read.form(0) + MESSAGE_FLAGS, flags as u8);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetMessageDisplayTime_Execute.
fn set_message_display_time(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        e.mem
            .set_u32(read.form(0) + MESSAGE_DISPLAY_TIME, read.int(1, 0));
    }
    handled(true)
}

// ---- the menus' data ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetMenuItemFilter_Execute.
/// -1, or for an open inventory menu its filter plus 1, for the container and barter menus the
/// player's filter (or with the second argument the other side's).
fn get_menu_item_filter(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let right = read.int(1, 0) != 0;
    let answer = match read.int(0, 0) {
        INVENTORY => {
            let menu = e.mem.u32(INVENTORY_MENU);
            (menu != 0).then(|| e.mem.u32(menu + INVENTORY_FILTER).wrapping_add(1))
        }
        CONTAINER => {
            let menu = e.mem.u32(CONTAINER_MENU);
            let at = if right {
                CONTAINER_RIGHT_FILTER
            } else {
                CONTAINER_LEFT_FILTER
            };
            (menu != 0).then(|| e.mem.u32(menu + at))
        }
        BARTER => {
            let menu = e.mem.u32(BARTER_MENU);
            let at = if right {
                BARTER_RIGHT_FILTER
            } else {
                BARTER_LEFT_FILTER
            };
            (menu != 0).then(|| e.mem.u32(menu + at))
        }
        _ => None,
    };
    if let Some(v) = answer {
        a.set_result(e, f64::from(v as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetBarterGoldAlt_Execute.
/// The merchant's caps in the open barter menu.
fn get_barter_gold_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let menu = e.mem.u32(BARTER_MENU);
    if menu != 0 {
        let gold = e.mem.u32(menu + BARTER_GOLD) as i32;
        a.set_result(e, f64::from(gold));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetBarterPriceMult_Execute.
/// The open barter menu's sell (argument not 0) or buy multiplier.
fn get_barter_price_mult(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let menu = e.mem.u32(BARTER_MENU);
    if menu == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok {
        let at = if read.int(0, 0) != 0 {
            BARTER_SELL_MULT
        } else {
            BARTER_BUY_MULT
        };
        a.set_result(e, f64::from(e.mem.f32(menu + at)));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetBarterPriceMult_Execute.
/// The sell or buy multiplier set, then the menu's prices worked out again (`0x730690(1)`).
fn set_barter_price_mult(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let menu = e.mem.u32(BARTER_MENU);
    if menu == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok {
        let at = if read.int(0, 0) != 0 {
            BARTER_SELL_MULT
        } else {
            BARTER_BUY_MULT
        };
        e.mem.set_f32(menu + at, read.float(1, 0.0));
        e.call(BARTER_REFRESH, &args![1u32]);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetRecipeMenuSelection_Execute.
/// The recipe selected in the open recipe menu.
fn get_recipe_menu_selection(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let selection = e.mem.u32(RECIPE_SELECTION);
    if e.mem.u32(RECIPE_MENU) != 0 && selection != 0 {
        set_form_result(e, &a, selection);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetRecipeMenuCategory_Execute.
/// The open recipe menu's category (JIP reads through a null category; nothing here).
fn get_recipe_menu_category(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let menu = e.mem.u32(RECIPE_MENU);
    if menu != 0 {
        let category = e.mem.u32(menu + RECIPE_CATEGORY);
        if category != 0 {
            set_form_result(e, &a, category);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetRecipeMenuNumSubcategories_Execute.
/// The size of the recipe menu's category array (`0x011D8F08`).
fn get_recipe_menu_num_subcategories(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let size = e.get(
        Ptr::<engine_core::types::BSSimpleArray>::new(RECIPE_CATEGORIES),
        engine_core::types::BSSimpleArray::iSize,
    ) as i32;
    a.set_result(e, f64::from(size));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetFontFile_Execute.
/// Font `n`'s file path (1 to 89, not 9: the byte after the eight first fonts); `""` for none.
fn get_font_file(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let id = read.int(0, 0);
    let manager = e.mem.u32(FONT_MANAGER);
    let mut text = None;
    if read.ok && (1..=89).contains(&id) && id != 9 && manager != 0 {
        let font = e.mem.u32(manager + 4 * (id - 1));
        if font != 0 {
            let path = e.mem.u32(font + FONT_FILE_PATH);
            if path != 0 {
                text = Some(nvse::table::guest_text(e, path));
            }
        }
    }
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetVATSTargets_Execute.
/// With the VATS menu open, an array of the VATS targets' references; none when there are none.
fn get_vats_targets(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if e.mem.u32(VATS_MENU) == 0 {
        return handled(true);
    }
    let mut tmp = TempElements::new();
    for target in list_items(e, Ptr::<ListNode>::new(VATS_TARGETS)) {
        if target != 0 {
            let reference = e.mem.u32(target);
            if reference != 0 {
                tmp.append(Element::Form(reference));
            }
        }
    }
    if !tmp.is_empty() {
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_ShowLevelUpMenuEx_Execute.
/// The level-up menu opened (`LevelUpMenu::Create`); when it is there, its skill points set and
/// its first page shown.
fn show_level_up_menu_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        e.call(LEVEL_UP_CREATE, &[]);
        let menu = e.mem.u32(LEVEL_UP_MENU);
        if menu != 0 {
            e.mem.set_u32(menu + LEVEL_UP_SKILL_POINTS, read.int(0, 0));
            e.call(LEVEL_UP_SET_PAGE, &args![menu, 0u32]);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetBarterItems_Execute.
/// With a barter menu and its merchant, an array of inventory references for the player's offer
/// (argument 0, in the player) or the merchant's (in the merchant's container); none when empty.
fn get_barter_items(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let menu = e.mem.u32(BARTER_MENU);
    if !read.ok || menu == 0 {
        return handled(true);
    }
    let merchant = e.mem.u32(menu + BARTER_MERCHANT);
    if merchant == 0 {
        return handled(true);
    }
    let sold = read.int(0, 0) != 0;
    let (target, offer) = if sold {
        (merchant_container(e, merchant), BARTER_RIGHT_OFFER)
    } else {
        (e.mem.u32(PLAYER), BARTER_LEFT_OFFER)
    };
    let mut tmp = TempElements::new();
    for entry in list_items(e, Ptr::<ListNode>::new(menu + offer)) {
        if entry != 0 {
            let reference = create_ref_for_stack(e, target, entry);
            tmp.append(Element::Form(reference));
        }
    }
    if !tmp.is_empty() {
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetMenuItemListRefs_Execute.
/// For the top visible inventory, container, repair, barter or repair services menu: a map of
/// inventory references by the item's place among the list's items (filtered ones counted; left
/// out unless asked for), from the player's list or, asked for, the other side's. JIP reads
/// through a menu that is not there (or a barter menu without its merchant): nothing here.
fn get_menu_item_list_refs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (filtered_too, right) = (read.int(0, 0) != 0, read.int(1, 0) != 0);
    let mut container = e.mem.u32(PLAYER);
    let (singleton, list) = match top_visible_menu_id(e) {
        INVENTORY => (INVENTORY_MENU, INVENTORY_ITEMS),
        CONTAINER if right => (CONTAINER_MENU, CONTAINER_RIGHT_ITEMS),
        CONTAINER => (CONTAINER_MENU, CONTAINER_LEFT_ITEMS),
        REPAIR => (REPAIR_MENU, REPAIR_ITEMS),
        BARTER if right => (BARTER_MENU, BARTER_RIGHT_ITEMS),
        BARTER => (BARTER_MENU, BARTER_LEFT_ITEMS),
        REPAIR_SERVICES => (REPAIR_SERVICES_MENU, REPAIR_SERVICES_ITEMS),
        _ => return handled(true),
    };
    let menu = e.mem.u32(singleton);
    if menu == 0 {
        return handled(true);
    }
    if right && singleton == CONTAINER_MENU {
        container = e.mem.u32(menu + CONTAINER_REF);
    } else if right && singleton == BARTER_MENU {
        let merchant = e.mem.u32(menu + BARTER_MERCHANT);
        container = merchant_container(e, merchant);
    }
    if container == 0 {
        return handled(true);
    }
    let map = create_map(e, a.script, &[], &[]);
    let mut index = 0i32;
    for item in list_items(e, Ptr::<ListNode>::new(menu + list + LIST_BOX_LIST)) {
        if item == 0 {
            continue;
        }
        let entry = e.mem.u32(item + LIST_ITEM_OBJECT);
        if entry == 0 {
            continue;
        }
        if filtered_too || e.mem.u8(item + LIST_ITEM_FILTERED) == 0 {
            let reference = create_ref_for_stack(e, container, entry);
            set_element(
                e,
                map,
                &Key::Number(f64::from(index)),
                &Element::Form(reference),
            );
        }
        index += 1;
    }
    a.set_result(e, f64::from(map));
    handled(true)
}

// ---- a terminal's own model ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: DoPurgePath.
/// A path JIP made for a terminal let go: when it is the terminal model in use, the model is
/// released and the game's own path put back; the text freed.
fn purge_path(e: &mut Engine, path: u32) {
    if path == 0 {
        return;
    }
    if e.mem.u32(TERMINAL_MODEL_PATH) == path {
        e.call(PURGE_TERMINAL_MODEL, &[]);
        let default = e.ext::<Jip>().ui_hooks.terminal_model_default;
        e.mem.set_u32(TERMINAL_MODEL_PATH, default);
    }
    heap_free(e, path);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetTerminalUIModel_Execute.
/// The terminal (or each terminal of a form list) given its own model path, or with no path its
/// own one removed; JIP's terminal model hook has a user while any terminal has one.
fn set_terminal_ui_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let path = buffer_arg(e, &read, 1, XML_BUFFER, "SetTerminalUIModel");
    for terminal in form_or_list(e, read.form(0)) {
        if terminal == 0 || type_of(e, terminal) != TERMINAL {
            continue;
        }
        if path.is_empty() {
            let old = e.ext::<Jip>().ui_hooks.terminal_models.remove(&terminal);
            purge_path(e, old.unwrap_or(0));
        } else {
            if let Some(old) = e
                .ext::<Jip>()
                .ui_hooks
                .terminal_models
                .get(&terminal)
                .copied()
            {
                purge_path(e, old);
            }
            let copy = heap_alloc(e, path.len() as u32 + 1);
            e.mem.write(copy, &path);
            e.mem.set_u8(copy + path.len() as u32, 0);
            e.ext::<Jip>()
                .ui_hooks
                .terminal_models
                .insert(terminal, copy);
        }
    }
    let any = !e.ext::<Jip>().ui_hooks.terminal_models.is_empty();
    e.set_hook(Hook::SetTerminalModel, any);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ui_tests.rs"]
mod tests;
