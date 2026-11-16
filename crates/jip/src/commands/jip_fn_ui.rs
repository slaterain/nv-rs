//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE): the interface's tiles found by path (whether one is
//! loaded, its string, a click on it, its removal, a gradual float, an XML file read into it, a
//! template's tile added to it), the pointer and the menus (the cursor's place, the menu on top,
//! the active tile's name, path and id, the character creation menus, a menu's button pressed),
//! messages (disabled by script, their flags and display time), and the open menus' data (item
//! filters, the barter caps and price multipliers, the recipe menu's selection, category and
//! category count, the VATS targets, the font files, the level-up menu with skill points, the
//! barter offers and a menu's item list as inventory references, a menu's target and the top
//! menu's highlighted item, a terminal's own model), a reference's place on the world map, and
//! menu XML made into tiles through the built-in UI Organizer (`InjectUIComponent`); HUD messages
//! queued with an icon and sound and the queue cleared, the system colours, the Pip-Boy light,
//! a text's size in a font. 41 of the file's 64 commands; its four menu event commands are the
//! core track's (`ui_hooks`).
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
//! - a hook or a code patch of JIP's: `UnlockRecipeMenuQuantity`, `ToggleCraftingMessages`,
//!   `EnableImprovedRecipeMenu`, `SuppressQuestMessages`, `InitItemFilter`, `SetItemFilter` (code bytes and jumps patched into
//!   the exe), `ShowTextInputMenu`, `SetTextInputExtendedProps`, `SetTextInputString` (the text
//!   input hooks, `TextInputClose`), `ToggleHUDCursor` (the HUD cursor and its input hooks);
//! - a routine of JIP's the game calls back: `ShowQuantityMenu`, `MessageBoxExAlt` (their
//!   callbacks), and `CloseActiveMenu`, which clears what those two keep;
//! - files and fonts: `SetFontFile` (a font file read from the folder, JIP's font map);
//! - 3D and drawing: `AttachUIXML`, `AttachUIComponent` (a tile's node under a reference's node),
//!   `ProjectUITile` (a rendered texture), `SetCursorPos` (the cursor's scene node);
//! - `RefreshItemsList`: in the map menu JIP sets `s_mapMenuSkipSetXY`, read only by its map
//!   menu patch (G87, `0x79DF73`, not built), and in the recipe menu it runs JIP's own
//!   `RefreshRecipeMenu`.
//!
//! `GetMenuTargetRef` and `GetSelectedItemRef`: where JIP reads through a null pointer (the top
//! menu's object, a highlighted perk rank with no item, a barter menu without a merchant), the
//! command gives no reference here. `GetWorldMapPosMults` works the map's numbers out on every
//! call where JIP keeps them for the last worldspace and root map (`s_worldDimensionsA`): the
//! same numbers, since a worldspace's map data does not change in play; a chain of parent
//! worlds stops after `MAX_WALK` steps.
//!
//! `MessageExAlt` cuts its message to 0x103 bytes until JIP's corner message length patch
//! (plan row G10, `InitGamePatches`, 16 bytes from `0x7753B8` to `0x77582B`: the HUD queue's
//! message buffer 0x104 -> 0x204) is on; then to JIP's 0x203. Its icon path and sound are always
//! cut to 0x103 bytes (the game's queue copies each into 0x104 and a longer one ends the process,
//! with JIP too). It calls the game's `QueueUIMessage` with its own seconds: JIP's G09 hook
//! there changes only the exe's own callers' time (return addresses up to `0x96A262`).
//!
//! Where JIP reads through a null pointer (a menu's tile, a recipe menu without a category, a
//! trait path without a trait), the command does nothing here: listed in the lane's audit. Also
//! there, as deliberate differences: a tile's child list ends as the crate's other game list
//! walks do (`list_step`), a trait map bucket and a tile's parents after `MAX_WALK` (kept as this
//! module's own cap: "not there"); a name handed to the game is cut to JIP's buffer (`in_buffer`);
//! the active tile's path is `""` where a child list is broken (`ui_hooks::component_full_name`);
//! and with the fixes preset's size caps row `ClickMenuButton` stops once its menu is closed and
//! after 65,536 clicks (vanilla makes every click, as JIP).

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
use nvse::commands::array::{fix_on, SIZE_CAPS};
use nvse::commands::Read;
use nvse::vars::Key;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2608, is_component_loaded),
    (0x2609, inject_ui_xml),
    (0x260a, inject_ui_component),
    (0x2638, get_cursor_pos),
    (0x2639, get_ui_string),
    (0x2681, get_active_menu_mode),
    (0x2682, get_active_ui_component_name),
    (0x2683, get_active_ui_component_full_name),
    (0x2684, get_active_ui_component_id),
    (0x2685, get_menu_target_ref),
    (0x2686, get_menu_item_filter),
    (0x2687, click_menu_button),
    (0x270b, get_selected_item_ref),
    (0x270e, get_barter_items),
    (0x2711, get_barter_gold_alt),
    (0x2712, get_recipe_menu_selection),
    (0x2713, get_recipe_menu_category),
    (0x2715, get_recipe_menu_num_subcategories),
    (0x2730, get_world_map_pos_mults),
    (0x2738, message_ex_alt),
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
    (0x2872, clear_message_queue),
    (0x2881, set_system_color),
    (0x28a4, click_menu_tile),
    (0x28ac, toggle_pipboy_light_cmd),
    (0x28c6, add_tile_from_template),
    (0x28cb, set_ui_float_gradual),
    (0x28e1, show_level_up_menu_ex),
    (0x2927, get_string_ui_dimensions),
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
/// The most items a trait map bucket is walked through, and the most parents a tile is followed
/// up for its menu: past it the walk ends silently with "not there" (JIP would walk a broken
/// chain for ever). A tile's child list is a game list and ends as the crate's other list walks
/// do ([`super::list_step`]: with the list guard row, else the port stops).
const MAX_WALK: u32 = 1 << 16;
/// `Menu::pMenu` (+4, engine-bs `menu.rs`): the menu's `TileMenu`, the tile `Menu::~Menu`
/// (`00A1C520`) deletes after clearing the menu's shown byte (`0x011F308F + id`) and its
/// `g_tileMenuArray` entry (`00A1E130(id, 0)`: `00A1E680` on the array object `0x011F3508`,
/// whose data word is `0x011F350C`).
const MENU_TILE: u32 = 0x04;
/// The most clicks `ClickMenuButton` makes with the fixes preset's size caps row on.
const MAX_CLICKS: u32 = 1 << 16;
/// JIP's `tilePath[0x100]`, `xmlPath[0x80]`, `modelPath[0x80]`, `AddTileFromTemplate`'s
/// `buffer[0x100]` (`ExtractFormatStringArgs` into it).
const PATH_BUFFER: usize = 0x100;
const XML_BUFFER: usize = 0x80;
/// `kCommandInfo_AddTileFromTemplate.numParams`: xNVSE's 21 format parameters.
const FORMAT_STRING_PARAMS: u32 = 21;
/// `kCommandInfo_InjectUIComponent.numParams`: the path and xNVSE's 21 format parameters.
const INJECT_PARAMS: u32 = 22;
/// JIP's string argument buffer (`GetStrArgBuffer`), which takes `InjectUIComponent`'s text.
const STR_ARG_BUFFER: usize = 0x4_0000;
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

/// What of `text` JIP's `size`-byte buffer holds before its 0 (`size - 1` bytes at most): the
/// part of a path, file name or template name a game function is given. With the list guard row
/// [`buffer_arg`] reads a longer text whole (JIP's would run past its buffer); a text of about
/// 1 MiB would not fit on the guest stack ([`with_guest_text`]), and no trait, file or template
/// name is that long, so the lookup answers "not there" as it would with the whole text.
fn in_buffer(text: &[u8], size: usize) -> &[u8] {
    &text[..text.len().min(size - 1)]
}

/// `SlashPos` (JIP LN NVSE): the first `/` or `\`.
fn slash_pos(path: &[u8]) -> Option<usize> {
    path.iter().position(|&b| b == b'/' || b == b'\\')
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: GetNextToken (one delimiter).
/// The text up to the first `delim` (the token), and what follows the run of `delim`s after it
/// (empty when there is no `delim`).
pub(crate) fn next_token(text: &[u8], delim: u8) -> (&[u8], &[u8]) {
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
    let mut walked = 0usize;
    while node != 0 && super::list_step(e, &mut walked) {
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
/// it is not there. The name is a part of JIP's `tilePath[0x100]`: at most its 255 bytes go to
/// the game ([`in_buffer`]).
fn trait_id(e: &mut Engine, name: &[u8]) -> u32 {
    let vtable = e.mem.u32(TRAIT_MAP);
    if vtable == 0 {
        return 0;
    }
    with_guest_text(e, in_buffer(name, PATH_BUFFER), |e, key| {
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
/// runs past its array there); empty when a tile on the way is missing from its parent's child
/// list (JIP reads through the list's end).
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
            let made = with_guest_text(e, in_buffer(&xml, XML_BUFFER), |e, at| {
                e.call(TILE_READ_FILE, &args![tile, at]).u32()
            });
            if made != 0 {
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

/// The log's one line, once a run, when `InjectUIComponent` is called with the UI Organizer row
/// off (nv-rs's own: JIP would write `jip_temp.xml` into the game's folder instead).
pub const INJECT_WITHOUT_UIO_LINE: &str = "JIP LN NVSE: InjectUIComponent was called with the UI \
     Organizer row off (bUIO, NV_UIO): nothing is injected; turn it on for UI mods that use it";

/// [`INJECT_WITHOUT_UIO_LINE`] was written in this run.
static INJECT_WITHOUT_UIO_NOTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_InjectUIComponent_Execute and InjectUIComponent.
/// `Menu/tile...` and a formatted text of menu XML: the text made into tiles under the tile
/// (none when the path ends at a trait) by the built-in UI Organizer's `UIOInjectComponent`
/// ([`uio::inject_component`]); result 1 when a tile is made. Without UIO, JIP writes the text to
/// `jip_temp.xml` in the game's folder and reads that file into the tile; nv-rs writes nothing
/// into the game's folder, so without UIO (the row off) the command does nothing (a deliberate
/// difference, as nothing is written into the game's folder).
fn inject_ui_component(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 1, INJECT_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let path = nvse::commands::args_in_buffer(e, &f.leading, 0, PATH_BUFFER, "InjectUIComponent");
    let text = formatted_in_buffer(e, &f.text, STR_ARG_BUFFER, "InjectUIComponent");
    if !uio::on(e) && !INJECT_WITHOUT_UIO_NOTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        eprintln!("{INJECT_WITHOUT_UIO_LINE}");
    }
    if let Some((tile, _)) = target_component(e, &path, false) {
        if uio::on(e) && uio::inject_component(e, tile, &text) != 0 {
            a.set_result(e, 1.0);
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
    let made = with_guest_text(e, in_buffer(template, PATH_BUFFER), |e, at| {
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
/// vtable's, so JIP's click events run too); result 1. JIP makes all `times` clicks (a count of
/// -1 is 4,294,967,295 of them, and they go on through a menu a click closed); with the fixes
/// preset's size caps row on, the clicks also end once the menu is closed ([`menu_still_open`])
/// and after [`MAX_CLICKS`] (a deliberate difference, in the lane's audit).
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
        let caps = fix_on(e, SIZE_CAPS);
        // Read before the first click: a click may close the menu and free it.
        let (id, menu_tile) = (e.mem.u32(menu + MENU_ID), e.mem.u32(menu + MENU_TILE));
        let mut clicks = 0u32;
        while clicks < times {
            let handler = e.mem.u32(e.mem.u32(menu) + HANDLE_CLICK_SLOT);
            e.call(handler, &args![menu, tile_id as u32, tile]);
            clicks += 1;
            if caps && (clicks == MAX_CLICKS || !menu_still_open(e, id, menu_tile)) {
                break;
            }
        }
        a.set_result(e, 1.0);
    }
    handled(true)
}

/// Whether menu `id`, whose `TileMenu` was `menu_tile`, is still open: `g_tileMenuArray[id]` is
/// still that tile and its shown byte is set (`Menu::~Menu` clears both).
fn menu_still_open(e: &Engine, id: u32, menu_tile: u32) -> bool {
    menu_tile != 0
        && menu_tile_by_id(e, id) == menu_tile
        && e.mem.u8(MENU_SHOWN.wrapping_add(id)) != 0
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

// ---- package C2: the message queue, system colours, the Pip-Boy light, text sizes ----

/// `HUDMainMenu` (`*011D96C0`): the queued messages (`tList` +0x18C), the shown message's start
/// time (+0x194) and the message tiles (icon +0x8C, text +0x90, bracket +0x94): the menu's own
/// queue code (`hudmainmenu.rs` `00775920`, `0077xxxx`) reads them there; `kTileValue_alpha`.
const HUD_MAIN_MENU: u32 = 0x011d_96c0;
const HUD_QUEUED_MESSAGES: u32 = 0x18c;
const HUD_MESSAGE_START: u32 = 0x194;
const HUD_MESSAGE_TILES: [u32; 3] = [0x8c, 0x90, 0x94];
const TRAIT_ALPHA: u32 = 0xfa9;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameUI.h: HUDMainMenu::ClearMessageQueue,
// nvse/GameTypes.h: tList::DeleteAll, nvse/GameTiles.cpp: Tile::SetFloat.
/// When the HUD's message queue is not empty (its first item is set): every node after the first
/// freed with its message, then the first node's message, the queue left empty; the start time
/// 0; the three message tiles' alpha 0 (each tile that has the value). No result.
///
/// nv-rs's own: the walk ends as the crate's other list walks do (`list_step`), and a missing
/// tile is passed over (JIP reads through it).
fn clear_message_queue(e: &mut Engine, _: &[u32]) -> Ret {
    let hud = e.mem.u32(HUD_MAIN_MENU);
    if hud == 0 {
        return handled(true);
    }
    let head = hud + HUD_QUEUED_MESSAGES;
    if e.get(Ptr::<ListNode>::new(head), ListNode::data) == 0 {
        return handled(true);
    }
    let mut walked = 0usize;
    loop {
        let node = e.get(Ptr::<ListNode>::new(head), ListNode::next);
        if node == 0 || !super::list_step(e, &mut walked) {
            break;
        }
        let message = e.get(Ptr::<ListNode>::new(node), ListNode::data);
        heap_free(e, message);
        let next = e.get(Ptr::<ListNode>::new(node), ListNode::next);
        e.set(Ptr::<ListNode>::new(head), ListNode::next, next);
        heap_free(e, node);
    }
    let first = e.get(Ptr::<ListNode>::new(head), ListNode::data);
    heap_free(e, first);
    e.set(Ptr::<ListNode>::new(head), ListNode::data, 0);
    e.mem.set_u32(hud + HUD_MESSAGE_START, 0);
    for at in HUD_MESSAGE_TILES {
        let tile = e.mem.u32(hud + at);
        if tile != 0 {
            let value = value_of(e, tile, TRAIT_ALPHA);
            if value != 0 {
                value_set_float(e, value, 0.0);
            }
        }
    }
    handled(true)
}

/// `SystemColorManager` (`*011D8A88`): its colours (`DList`: first +0; a node's next +0, colour
/// +8), as the game's own `SystemColorManager::Find` (`00718FB0`) walks them; a colour's
/// `SetColor` (vtable +8, one word: the game's setter `00719120` calls it so). The interface
/// manager's menus root (+0x9C, `00586150`) and the colour refresh `00A0B350` (`thiscall (tile,
/// type, 0)`, `RET 8`: the start menu calls it so after a colour change).
const SYSTEM_COLOR_MANAGER: u32 = 0x011d_8a88;
const SET_COLOR_SLOT: u32 = 0x08;
const IM_MENUS_ROOT: u32 = 0x9c;
const REFRESH_COLOR: u32 = 0x00a0_b350;
/// JIP's colour types: 1 to 5.
const SYSTEM_COLOR_TYPES: u32 = 5;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetSystemColor_Execute, nvse/GameUI.cpp: SystemColor::SetColorRGB.
/// `SetSystemColor type r g b`: for a type of 1 to 5, the manager's colour number `type`
/// (counted from 1) set to `r g b` with alpha 255 (`(r << 24) | (g << 16) | (b << 8) | 0xFF`,
/// each the argument's low byte), then the menus' colours refreshed. No result.
fn set_system_color(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(0, 0);
    if !read.ok || kind == 0 || kind > SYSTEM_COLOR_TYPES {
        return handled(true);
    }
    let manager = e.mem.u32(SYSTEM_COLOR_MANAGER);
    if manager == 0 {
        return handled(true);
    }
    // `DList::GetNthItem(type - 1)`: from the first node, `next` taken type - 1 times, stopping
    // at a null one.
    let mut node = e.mem.u32(manager);
    for _ in 1..kind {
        if node == 0 {
            break;
        }
        node = e.mem.u32(node);
    }
    if node == 0 {
        return handled(true);
    }
    let color_obj = e.mem.u32(node + 8);
    if color_obj == 0 {
        return handled(true);
    }
    let [r, g, b] = [1, 2, 3].map(|i| read.int(i, 0) & 0xff);
    let color = (r << 24) | (g << 16) | (b << 8) | 0xff;
    e.vcall(color_obj, SET_COLOR_SLOT, &args![color]);
    let im = interface_manager(e);
    if im != 0 {
        let root = e.mem.u32(im + IM_MENUS_ROOT);
        e.call(REFRESH_COLOR, &args![root, kind, 0u32]);
    }
    handled(true)
}

/// The Pip-Boy light spell (`*011C358C`: the magic system's form 0x147), its `MagicItem` (+0x18);
/// the player's `MagicCaster` (+0x88, vtable slot 0 `CastAbility(spell, 0)`, `RET 8`: `0094C870`)
/// and `MagicTarget` (+0x94); `MagicTarget::IsSpellTarget` (`00822B90`, `(item, 1)`, `RET 8`) and
/// `MagicTarget::Dispel` (`00824400`, `(item, 0, 0)`, `RET 0xC`); the interface manager's
/// `FOPipboyManager` (+0x174) and its `ShowPipboyLightEffect` (`007FA310`, three bytes, `RET 0xC`).
const PIPBOY_LIGHT: u32 = 0x011c_358c;
const MAGIC_ITEM: u32 = 0x18;
const PC_MAGIC_CASTER: u32 = 0x88;
const PC_MAGIC_TARGET: u32 = 0x94;
const IS_SPELL_TARGET: u32 = 0x0082_2b90;
const DISPEL: u32 = 0x0082_4400;
const IM_PIPBOY_MANAGER: u32 = 0x174;
const SHOW_PIPBOY_LIGHT: u32 = 0x007f_a310;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: TogglePipBoyLight (asm).
/// On: the player casts the light (`CastAbility`); off: its effect dispelled from the player. Then
/// the Pip-Boy manager's light effect shown twice: `(1, on, 1)` and `(0, on, 1)`.
fn toggle_pipboy_light(e: &mut Engine, player: u32, spell: u32, on: u32) {
    if on != 0 {
        let caster = player + PC_MAGIC_CASTER;
        e.vcall(caster, 0, &args![spell, 0u32]);
    } else {
        e.call(
            DISPEL,
            &args![player + PC_MAGIC_TARGET, spell + MAGIC_ITEM, 0u32, 0u32],
        );
    }
    let im = interface_manager(e);
    let manager = if im == 0 {
        0
    } else {
        e.mem.u32(im + IM_PIPBOY_MANAGER)
    };
    // nv-rs's own: without the manager no effect is shown (JIP calls through a null one).
    if manager != 0 {
        e.call(SHOW_PIPBOY_LIGHT, &args![manager, 1u32, on, 1u32]);
        e.call(SHOW_PIPBOY_LIGHT, &args![manager, 0u32, on, 1u32]);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_TogglePipBoyLight_Execute.
/// `TogglePipBoyLight [on]`: 1 when the player has the Pip-Boy light's effect; with an argument
/// that differs from that (compared whole: 2 differs from 1), the light turned on or off.
///
/// nv-rs's own: without the player or the spell nothing is done (JIP reads through them).
fn toggle_pipboy_light_cmd(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let (player, spell) = (e.mem.u32(PLAYER), e.mem.u32(PIPBOY_LIGHT));
    if player == 0 || spell == 0 {
        return handled(true);
    }
    let state = e
        .call(
            IS_SPELL_TARGET,
            &args![player + PC_MAGIC_TARGET, spell + MAGIC_ITEM, 1u32],
        )
        .u32()
        & 0xff;
    let state = u32::from(state != 0);
    a.set_result(e, f64::from(state));
    if num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        let on = read.int(0, 0);
        if read.ok && on != state {
            toggle_pipboy_light(e, player, spell, on);
        }
    }
    handled(true)
}

/// `FontManager::CalculateStringDimensions` (`00A1B020`, `thiscall (out, text, font, wrap
/// width, start)`, `RET 0x14`: a `NiVector3` out) on [`FONT_MANAGER`].
const STRING_DIMENSIONS: u32 = 0x00a1_b020;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetStringUIDimensions_Execute.
/// `GetStringUIDimensions text font wrapWidth outW outH outLines`: the game's size of the text in
/// the font (a wrap width of 0 or less: `FLT_MAX`) into the three variables. No result.
fn get_string_ui_dimensions(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let text = buffer_arg(e, &read, 0, STR_ARG_BUFFER, "GetStringUIDimensions");
    let font = read.int(1, 0);
    let mut wrap = read.float(2, 0.0);
    let vars = [read.local(3), read.local(4), read.local(5)];
    if wrap <= 0.0 {
        wrap = f32::MAX;
    }
    let manager = e.mem.u32(FONT_MANAGER);
    if manager == 0 {
        return handled(true);
    }
    let dims = with_guest_text(e, &text, |e, text| {
        e.with_stack(0x10, |e, out| {
            let out = out.addr();
            for i in 0..4 {
                e.mem.set_u32(out + 4 * i, 0);
            }
            e.call(
                STRING_DIMENSIONS,
                &args![manager, out, text, font, wrap, 0u32],
            );
            [0, 4, 8].map(|o| e.mem.f32(out + o))
        })
    });
    for (var, v) in vars.into_iter().zip(dims) {
        if var != 0 {
            e.mem.set_f64(var + 8, f64::from(v));
        }
    }
    handled(true)
}

/// `QueueUIMessage` (`007052F0`, `cdecl (text, icon type, icon path, sound, seconds, byte)`).
const QUEUE_UI_MESSAGE: u32 = 0x0070_52f0;
/// JIP's `kMsgIconsPathAddr`: the exe's message icon paths `#0` to `#6`.
const MESSAGE_ICONS: [u32; 7] = [
    0x0102_08a0,
    0x0102_08e0,
    0x0102_5cdc,
    0x0103_0e78,
    0x0103_a830,
    0x0104_9638,
    0x0104_bfe8,
];
/// `buffer[0x203] = 0`: the most of the message JIP hands on.
const MESSAGE_MAX: usize = 0x203;
/// The HUD's queue (`00775380`) copies the message, the icon path and the sound path with
/// `strcpy_s` into 0x104 bytes each (a block of 0x310: +0, +0x104, +0x208); a longer text fails
/// the copy and the CRT's invalid parameter path ends the process. JIP's `InitGamePatches`
/// raises the message's to 0x204 (its "corner message max text length" bytes at `0x7753B8`,
/// `0x7753D2`, `0x7753EA`, `0x77540C`, `0x775443`, `0x775461`, `0x77547E`, `0x77549C`, `0x7754BC`,
/// `0x7754D0`, `0x77553C`, `0x775600`, `0x7714DA`, `0x7757B1`, `0x7757D3`, `0x77582B`), which the
/// port has not built yet (plan row G10): until it is on, the message is cut to 0x103 bytes
/// (0x203 once it is). The icon path and the sound stay 0x104 bytes even with that patch: cut to
/// 0x103 always (JIP would stop the game there).
const HUD_TEXT_MAX: usize = 0x103;
/// `kCommandInfo_MessageExAlt.numParams`: the seconds and xNVSE's 21 format parameters.
const MESSAGE_PARAMS: u32 = 22;

/// What `MessageExAlt` hands `QueueUIMessage`: the message, an icon (the exe's path or a token of
/// the text) and a sound.
#[derive(Debug, PartialEq)]
enum Icon {
    None,
    Exe(u32),
    Text(Vec<u8>),
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: Cmd_MessageExAlt_Execute.
/// The text split at `|` runs: while a token has text after it, a token `$name` is the sound,
/// `#n` (`n` 0 to 6) the exe's icon `n` (another `#` token changes nothing), any other the icon's
/// path; the last token is the message, cut to 0x203 bytes. nv-rs's own: the message, the icon
/// path and the sound then cut to [`HUD_TEXT_MAX`], what the game's queue copies safely.
fn split_message(text: &[u8]) -> (Vec<u8>, Icon, Option<Vec<u8>>) {
    let fit = |t: &[u8]| t[..t.len().min(HUD_TEXT_MAX)].to_vec();
    let (mut icon, mut sound) = (Icon::None, None);
    let mut rest = text;
    loop {
        let (token, after) = next_token(rest, b'|');
        if after.is_empty() {
            let message = &token[..token.len().min(MESSAGE_MAX)];
            return (fit(message), icon, sound);
        }
        match token.first() {
            Some(b'$') => sound = Some(fit(&token[1..])),
            Some(b'#') => {
                // `buffer[1] - '0'` as a signed char: 0 to 6 only.
                let index = token
                    .get(1)
                    .map_or(-(b'0' as i32), |&c| i32::from(c as i8) - i32::from(b'0'));
                if let Some(&path) = usize::try_from(index)
                    .ok()
                    .and_then(|i| MESSAGE_ICONS.get(i))
                {
                    icon = Icon::Exe(path);
                }
            }
            _ => icon = Icon::Text(fit(token)),
        }
        rest = after;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_MessageExAlt_Execute.
/// `MessageExAlt seconds format ...`: the formatted text ([`split_message`]) queued as a HUD
/// message for the seconds, with its icon and sound. No result.
fn message_ex_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 1, MESSAGE_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let seconds = f.leading.first().and_then(Arg::float).unwrap_or(0.0);
    let text = formatted_in_buffer(e, &f.text, STR_ARG_BUFFER, "MessageExAlt");
    let (message, icon, sound) = split_message(&text);
    // One block on the guest stack: the message, then the icon's and the sound's texts, each
    // with its 0.
    let mut block = message.clone();
    block.push(0);
    let icon_at = block.len() as u32;
    if let Icon::Text(t) = &icon {
        block.extend_from_slice(t);
        block.push(0);
    }
    let sound_at = block.len() as u32;
    if let Some(s) = &sound {
        block.extend_from_slice(s);
    }
    with_guest_text(e, &block, |e, at| {
        let icon_ptr = match icon {
            Icon::None => 0,
            Icon::Exe(path) => path,
            Icon::Text(_) => at + icon_at,
        };
        let sound_ptr = if sound.is_some() { at + sound_at } else { 0 };
        e.call(
            QUEUE_UI_MESSAGE,
            &args![at, 0u32, icon_ptr, sound_ptr, seconds, 0u32],
        );
    });
    handled(true)
}

// ---- a menu's target and its selected item ----

/// The menus' singletons not named above, the game's own selection and target globals, and the
/// fields read (engine-fallout-interface, Xbox PDB names): the inventory, container and barter
/// menus' current item (`pCurrItem`, `0x11D9EA8`, `0x11D93FC`, `0x11D8FA8`), the repair menu's
/// broken item (`0x11DA760`), the item mod menu's item (`0x11D9F58`), the VATS target
/// (`0x11F21CC`), the repair vendor (`0x11DA7F4`).
const STATS_MENU: u32 = 0x011d_ace0;
const MAP_MENU: u32 = 0x011d_a368;
const ITEM_MOD_MENU: u32 = 0x011d_9f54;
const INVENTORY_SELECTION: u32 = 0x011d_9ea8;
const CONTAINER_SELECTION: u32 = 0x011d_93fc;
const BARTER_SELECTION: u32 = 0x011d_8fa8;
const REPAIR_TARGET: u32 = 0x011d_a760;
const ITEM_MOD_TARGET: u32 = 0x011d_9f58;
const VATS_TARGET: u32 = 0x011f_21cc;
const REPAIR_SERVICES_TARGET: u32 = 0x011d_a7f4;
/// `DialogMenu::pSpeaker` +0x80, `LockPickMenu::pLockTarget` +0x6C, `MapMenu`'s highlighted
/// marker tile +0xD4, its marker list +0xD8 and its quest, note and challenge lists +0x130,
/// +0x160, +0x1F0, `LevelUpMenu`'s page +0x28 (1: perks) and perk list +0x94,
/// `CompanionWheelMenu`'s companion +0x68, `RecipeMenu::pCrafterRef` +0x60, `CaravanMenu`'s
/// opponent +0x100, `TraitMenu`'s perk list +0x54, `StatsMenu`'s perk rank list +0x1E0 (its
/// items' perk +0), `ItemModMenu`'s list +0x60, the container and barter menus' current list
/// (+0xF8, +0x108).
const DIALOG_PARTNER: u32 = 0x80;
const LOCK_PICK_TARGET: u32 = 0x6c;
const MAP_MARKER_TILE: u32 = 0xd4;
const MAP_MARKERS: u32 = 0xd8;
const MAP_QUESTS: u32 = 0x130;
const MAP_NOTES: u32 = 0x160;
const MAP_CHALLENGES: u32 = 0x1f0;
const LEVEL_UP_PAGE: u32 = 0x28;
const LEVEL_UP_PERKS: u32 = 0x94;
const COMPANION_REF: u32 = 0x68;
const RECIPE_SOURCE: u32 = 0x60;
const CARAVAN_OPPONENT: u32 = 0x100;
const TRAIT_PERKS: u32 = 0x54;
const STATS_PERK_RANKS: u32 = 0x1e0;
const ITEM_MOD_ITEMS: u32 = 0x60;
const CONTAINER_CURRENT: u32 = 0xf8;
const BARTER_CURRENT: u32 = 0x108;
/// A list box's container tile (+0xC) and highlighted tile (+0x10); a list item's tile (+0).
const LIST_BOX_PARENT: u32 = 0x0c;
const LIST_BOX_SELECTED: u32 = 0x10;
const LIST_ITEM_TILE: u32 = 0x00;
/// `ExtraTalkingActor` (0x55) and its actor (+0xC); `BGSTalkingActivator`'s form type; a
/// reference's base form (+0x20).
const EXTRA_TALKING_ACTOR: u32 = 0x55;
const TALKING_ACTOR: u32 = 0x0c;
const TALKING_ACTIVATOR: u8 = 0x16;
const REFR_BASE: u32 = 0x20;
/// The menus `GetMenuTargetRef` knows besides those above (JIP's `kMenuType_*`).
const DIALOG: u32 = 0x3f1;
const LOCK_PICK: u32 = 0x3f6;
const QUANTITY: u32 = 0x3f8;
const LEVEL_UP: u32 = 0x403;
const HACKING: u32 = 0x41f;
const VATS: u32 = 0x420;
const COMPUTERS: u32 = 0x421;
const COMPANION_WHEEL: u32 = 0x433;
const RECIPE: u32 = 0x435;
const CARAVAN: u32 = 0x43b;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameUI.h: ListBox::GetSelected.
/// The item of the list box's highlighted tile: the first list item whose tile it is; 0 when
/// nothing is highlighted or no item has it.
fn list_box_selected(e: &Engine, list_box: u32) -> u32 {
    let selected = e.mem.u32(list_box + LIST_BOX_SELECTED);
    if selected == 0 {
        return 0;
    }
    for item in list_items(e, Ptr::<ListNode>::new(list_box + LIST_BOX_LIST)) {
        if item != 0 && e.mem.u32(item + LIST_ITEM_TILE) == selected {
            return e.mem.u32(item + LIST_ITEM_OBJECT);
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameUI.h: ListBox::IsEnabled.
/// The list box has a container tile whose `_enabled` is not 0.
fn list_box_enabled(e: &mut Engine, list_box: u32) -> bool {
    let parent = e.mem.u32(list_box + LIST_BOX_PARENT);
    if parent == 0 {
        return false;
    }
    let id = trait_id(e, b"_enabled");
    value_float(e, parent, id) != 0.0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetHackingMenuRef.
/// The hacking menu's terminal once it has guess tiles (the low byte of their count, +0x17C),
/// else the interface manager's activate reference (+0xFC).
fn hacking_menu_ref(e: &Engine, menu: u32) -> u32 {
    let menu = Ptr::<crate::game::HackingMenu>::new(menu);
    if e.get(menu, crate::game::HackingMenu::guessTileCount) != 0 {
        e.get(menu, crate::game::HackingMenu::pTerminalRef)
    } else {
        let im = interface_manager(e);
        if im == 0 {
            0
        } else {
            e.get(
                Ptr::<crate::game::InterfaceManager>::new(im),
                crate::game::InterfaceManager::pActivateRef,
            )
        }
    }
}

/// The highlighted map marker: the `_MarkerIndex` of the marker tile picks the marker list's
/// item (`tList::GetNthItem`, the float truncated as C++'s conversion does).
fn map_marker_ref(e: &mut Engine, menu: u32) -> u32 {
    let tile = e.mem.u32(menu + MAP_MARKER_TILE);
    if tile == 0 {
        return 0;
    }
    let id = trait_id(e, b"_MarkerIndex");
    let value = value_of(e, tile, id);
    if value == 0 {
        return 0;
    }
    let index = cvttss2si(e.mem.f32(value + VALUE_NUM));
    super::nth_item(e, Ptr::<ListNode>::new(menu + MAP_MARKERS), index)
}

/// The dialogue partner, or for a talking activator its `ExtraTalkingActor`'s actor when it has
/// one.
fn dialog_partner(e: &mut Engine, menu: u32) -> u32 {
    let partner = e.mem.u32(menu + DIALOG_PARTNER);
    if partner == 0 {
        return 0;
    }
    let base = e.mem.u32(partner + REFR_BASE);
    if base == 0 || type_of(e, base) != TALKING_ACTIVATOR {
        return partner;
    }
    let extra = super::extra::refr_extra(e, partner, EXTRA_TALKING_ACTOR);
    if extra == 0 {
        partner
    } else {
        e.mem.u32(extra + TALKING_ACTOR)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetMenuTargetRef_Execute.
/// `GetMenuTargetRef menuID`: what the open menu is about. The container, lock, barter,
/// computers, companion wheel, recipe and caravan menus' own reference; the dialogue partner
/// (a talking activator's actor); the hacking menu's terminal; the VATS target and the repair
/// vendor (the game's globals); the highlighted map marker; the highlighted perk of the level-up
/// menu's perk page and of the trait menu. The quantity, repair and item mod menus' item, as an
/// inventory reference: the inventory's, else the container menu's (in the container when its
/// right list is the current one), else the barter menu's (in the merchant's container). No
/// result when the menu is not open or names nothing.
fn get_menu_target_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let id = read.int(0, 0);
    let tile = menu_tile_by_id(e, id);
    let menu = if tile == 0 {
        0
    } else {
        e.mem.u32(tile + TILE_MENU_MENU)
    };
    if menu == 0 {
        return handled(true);
    }
    let mut target = 0;
    let mut container = e.mem.u32(PLAYER);
    let mut entry = 0;
    match id {
        CONTAINER => target = e.mem.u32(menu + CONTAINER_REF),
        DIALOG => target = dialog_partner(e, menu),
        LOCK_PICK => target = e.mem.u32(menu + LOCK_PICK_TARGET),
        QUANTITY => {
            let (container_menu, barter_menu) = (e.mem.u32(CONTAINER_MENU), e.mem.u32(BARTER_MENU));
            if e.mem.u8(MENU_SHOWN + INVENTORY) != 0 {
                entry = e.mem.u32(INVENTORY_SELECTION);
            } else if container_menu != 0 {
                entry = e.mem.u32(CONTAINER_SELECTION);
                if e.mem.u32(container_menu + CONTAINER_CURRENT)
                    == container_menu + CONTAINER_RIGHT_ITEMS
                {
                    container = e.mem.u32(container_menu + CONTAINER_REF);
                }
            } else if barter_menu != 0 {
                entry = e.mem.u32(BARTER_SELECTION);
                if e.mem.u32(barter_menu + BARTER_CURRENT) == barter_menu + BARTER_RIGHT_ITEMS {
                    let merchant = e.mem.u32(barter_menu + BARTER_MERCHANT);
                    container = merchant_container(e, merchant);
                }
            }
        }
        MAP => target = map_marker_ref(e, menu),
        LEVEL_UP => {
            if e.mem.u32(menu + LEVEL_UP_PAGE) == 1 {
                target = list_box_selected(e, menu + LEVEL_UP_PERKS);
            }
        }
        REPAIR => entry = e.mem.u32(REPAIR_TARGET),
        BARTER => target = e.mem.u32(menu + BARTER_MERCHANT),
        HACKING => target = hacking_menu_ref(e, menu),
        VATS => target = e.mem.u32(VATS_TARGET),
        COMPUTERS => {
            target = e.get(
                Ptr::<crate::game::ComputersMenu>::new(menu),
                crate::game::ComputersMenu::pTerminalRef,
            );
        }
        REPAIR_SERVICES => target = e.mem.u32(REPAIR_SERVICES_TARGET),
        ITEM_MOD => entry = e.mem.u32(ITEM_MOD_TARGET),
        COMPANION_WHEEL => target = e.mem.u32(menu + COMPANION_REF),
        RECIPE => target = e.mem.u32(menu + RECIPE_SOURCE),
        CARAVAN => target = e.mem.u32(menu + CARAVAN_OPPONENT),
        TRAIT_MENU => target = list_box_selected(e, menu + TRAIT_PERKS),
        _ => {}
    }
    if container != 0 && entry != 0 {
        target = create_ref_for_stack(e, container, entry);
    }
    if target != 0 {
        set_form_result(e, &a, target);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetSelectedItemRef_Execute.
/// `GetSelectedItemRef`: the highlighted item of the menu on top. The inventory's, the container
/// menu's (in the container when its right list has the highlight), the barter menu's (in the
/// merchant's container likewise), the repair, repair services and item mod menus' item, each
/// as an inventory reference; the stats menu's highlighted perk; the map menu's highlighted
/// quest, else note, else challenge, each only from an enabled list. No result otherwise.
fn get_selected_item_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mut target = 0;
    let mut container = e.mem.u32(PLAYER);
    let mut entry = 0;
    let top = top_visible_menu_id(e);
    let singleton = match top {
        INVENTORY => INVENTORY_MENU,
        STATS => STATS_MENU,
        CONTAINER => CONTAINER_MENU,
        MAP => MAP_MENU,
        REPAIR => REPAIR_MENU,
        BARTER => BARTER_MENU,
        REPAIR_SERVICES => REPAIR_SERVICES_MENU,
        ITEM_MOD => ITEM_MOD_MENU,
        _ => 0,
    };
    let menu = if singleton == 0 {
        0
    } else {
        e.mem.u32(singleton)
    };
    if menu != 0 {
        match top {
            INVENTORY => {
                if e.mem.u32(menu + INVENTORY_ITEMS + LIST_BOX_SELECTED) != 0 {
                    entry = e.mem.u32(INVENTORY_SELECTION);
                }
            }
            STATS => {
                let rank = list_box_selected(e, menu + STATS_PERK_RANKS);
                // JIP reads `GetSelected()->perk` with no null test (it would crash on a
                // highlighted tile no rank has); here that gives none.
                if rank != 0 {
                    target = e.mem.u32(rank);
                }
            }
            CONTAINER | BARTER => {
                let (left, right, selection) = if top == CONTAINER {
                    (
                        CONTAINER_LEFT_ITEMS,
                        CONTAINER_RIGHT_ITEMS,
                        CONTAINER_SELECTION,
                    )
                } else {
                    (BARTER_LEFT_ITEMS, BARTER_RIGHT_ITEMS, BARTER_SELECTION)
                };
                let right_selected = e.mem.u32(menu + right + LIST_BOX_SELECTED) != 0;
                if e.mem.u32(menu + left + LIST_BOX_SELECTED) != 0 || right_selected {
                    entry = e.mem.u32(selection);
                    if right_selected {
                        container = if top == CONTAINER {
                            e.mem.u32(menu + CONTAINER_REF)
                        } else {
                            let merchant = e.mem.u32(menu + BARTER_MERCHANT);
                            merchant_container(e, merchant)
                        };
                    }
                }
            }
            MAP => {
                for list in [MAP_QUESTS, MAP_NOTES, MAP_CHALLENGES] {
                    if e.mem.u32(menu + list + LIST_BOX_SELECTED) != 0
                        && list_box_enabled(e, menu + list)
                    {
                        target = list_box_selected(e, menu + list);
                        break;
                    }
                }
            }
            REPAIR => entry = list_box_selected(e, menu + REPAIR_ITEMS),
            REPAIR_SERVICES => entry = list_box_selected(e, menu + REPAIR_SERVICES_ITEMS),
            ITEM_MOD => entry = list_box_selected(e, menu + ITEM_MOD_ITEMS),
            _ => {}
        }
    }
    if container != 0 && entry != 0 {
        target = create_ref_for_stack(e, container, entry);
    }
    if target != 0 {
        set_form_result(e, &a, target);
    }
    handled(true)
}

// ---- the world map ----

/// `TESObjectREFR`'s position (+0x30, x then y). `TESWorldSpace` (engine-fallout-cell
/// `tesworldspace.rs`, Xbox PDB): `sParentUseFlags` +0x4E (bit 2: the parent's map data),
/// `pParentWorld` +0x70, `WorldMapData`'s NW and SE cells +0x88 (four `i16`),
/// `WorldMapOffsetData` +0x90 (scale, offset x, y), `MinimumCoords` +0xA0 and `MaximumCoords`
/// +0xA8. A cell's worldspace +0xC0.
const REFR_POSITION: u32 = 0x30;
const WS_PARENT_USE_FLAGS: u32 = 0x4e;
const WS_PARENT: u32 = 0x70;
const WS_MAP_CELLS: u32 = 0x88;
const WS_MAP_SCALE: u32 = 0x90;
const WS_MAP_OFFSET: u32 = 0x94;
const WS_MIN_COORDS: u32 = 0xa0;
const CELL_WORLD: u32 = 0xc0;
/// JIP's `kSizeMults` float (`0x3F4C0000`) and `kPosMultMods` (`0x3DD00000`).
const MAP_SIZE_MULT: f32 = 0.796_875;
const MAP_POS_ADD: f32 = 0.101_562_5;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESWorldSpace::GetRootMapWorld
// (asm).
/// The world whose map `world` uses: its parents' while it takes its parent's map data.
fn root_map_world(e: &Engine, world: u32) -> u32 {
    let mut world = world;
    for _ in 0..MAX_WALK {
        let parent = e.mem.u32(world + WS_PARENT);
        if parent == 0 || e.mem.u8(world + WS_PARENT_USE_FLAGS) & 4 == 0 {
            return world;
        }
        world = parent;
    }
    world
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// WorldDimensions::InitDimensions, WorldDimensions::InitPosMods, GetWorldMapPosMults (asm).
/// Where `pos` lies on the root world's map, as fractions of the map texture: the position
/// scaled and shifted by its own world's map offset data, less the root map's north-west corner
/// (cells times 4096), times 0.796875 over the map's size (the processor's reciprocal estimate,
/// `rcpps`), plus 0.1015625.
fn world_map_pos_mults(e: &Engine, world: u32, root: u32, pos: [f32; 2]) -> [f32; 2] {
    let cell = |i: u32| e.mem.u16(root + WS_MAP_CELLS + 2 * i) as i16;
    // `paddw` adds 1 to the NW cell's y and the SE cell's x (16-bit lanes), then each is widened.
    let nw = [i32::from(cell(0)), i32::from(cell(1).wrapping_add(1))];
    let se = [i32::from(cell(2).wrapping_add(1)), i32::from(cell(3))];
    let min = [nw[0].wrapping_shl(12) as f32, nw[1].wrapping_shl(12) as f32];
    let size = |i: usize| {
        let span = se[i].wrapping_sub(nw[i]).wrapping_shl(12) as f32;
        super::havok_refs::rcp_estimate(span) * MAP_SIZE_MULT
    };
    let size = [size(0), size(1)];
    let offset = [
        e.mem.f32(world + WS_MAP_OFFSET),
        e.mem.f32(world + WS_MAP_OFFSET + 4),
    ];
    let shifted = if e.mem.u32(world + WS_MAP_SCALE) == 0x3f80_0000 {
        [pos[0] + offset[0], pos[1] + offset[1]]
    } else {
        // `InitPosMods`: (min + max) * (0.5 - scale * 0.5) + offset.
        let scale = e.mem.f32(world + WS_MAP_SCALE);
        let half = 0.5 - scale * 0.5;
        let c = |i: u32| e.mem.f32(world + WS_MIN_COORDS + 4 * i);
        let pos_mod = [
            (c(0) + c(2)) * half + offset[0],
            (c(1) + c(3)) * half + offset[1],
        ];
        [pos[0] * scale + pos_mod[0], pos[1] * scale + pos_mod[1]]
    };
    [
        (shifted[0] - min[0]) * size[0] + MAP_POS_ADD,
        (shifted[1] - min[1]) * size[1] + MAP_POS_ADD,
    ]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_GetWorldMapPosMults_Execute.
/// `ref.GetWorldMapPosMults x y`: where the reference stands on its world's map (see
/// [`world_map_pos_mults`]) into the two variables; nothing for a reference not in a
/// worldspace. No result.
fn get_world_map_pos_mults(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let (out_x, out_y) = (read.local(0), read.local(1));
    // JIP's `GetParentWorld` tests the persistent cell extra's cell for null too.
    let Some(cell) = super::ln_fn_math::cell_or_persistent(e, a.this_obj).filter(|&cell| cell != 0)
    else {
        return handled(true);
    };
    let world = e.mem.u32(cell + CELL_WORLD);
    if world == 0 {
        return handled(true);
    }
    let root = root_map_world(e, world);
    let pos = [
        e.mem.f32(a.this_obj + REFR_POSITION),
        e.mem.f32(a.this_obj + REFR_POSITION + 4),
    ];
    let [x, y] = world_map_pos_mults(e, world, root, pos);
    for (var, v) in [(out_x, x), (out_y, y)] {
        if var != 0 {
            e.mem.set_f64(var + 8, f64::from(v));
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ui_tests.rs"]
mod tests;
