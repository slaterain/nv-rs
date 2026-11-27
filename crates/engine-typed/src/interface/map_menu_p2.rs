//! The map menu unit (the Xbox PDB's `fallout/interface/menus/mapmenu` source), its second part:
//! the menu's life (constructor, destructor, deleting destructor, class, `Create`, the tile ids,
//! `UpdateMenu`), its small members (the tile flags, the down click and the controller switch, the
//! selected note, the magnifications, the root map space, the local map's grid test, the map
//! memory and the world map's texture cleared, the cursor's y), the data pages (the current page,
//! the notes and their filter, the challenge mode, the challenges with their filter and sort, the
//! quests with their sort and the objectives' quest filter, the radio stations with their filter
//! and sort, the radio waveform), the array's remove and the conversation's deleting destructor.
//! The translation is engine-fallout-interface's `mapmenu.rs`, which these functions replace at the
//! same addresses. `DoClick` (JIP LN's sound flag seam), `DoGamepad`, `AddMapIcons` and
//! `AddCompanionMarkers` (extender hooks) and the maps, sounds and markers stay translated.
//!
//! **Native.** The exe's order of work; `MapMenu::pMe` read again after each call, as the exe does
//! (a call may close the menu). The calls of functions no family types stay calls by address (the
//! trait ids by name, the file read, the menu's tile and XML archive, the tutorial, the list boxes'
//! set-up and tile look-up, the focus, the calendar, the strings' format and destructor, the
//! sounds' lists, the maps' updates, the note's load, the radio stations, the clock, the message
//! box's text, an objective's display test (overridable)). The tile's getters and setters, a
//! node's children and the waypoint array's clear are the dispatch family's, a node's child the
//! multibound family's, an object's property the cell family's, the strip offset the HUD family's,
//! the array's shrink test and free the pathing family's, `_ftol2` the C runtime's; the list boxes'
//! methods, the `Interface::` functions and this file's own are this family's (all through
//! `typed_call!`). Done in place ([`IN_PLACE`] and the other files' lists): the interface manager,
//! its Pip-Boy and the Pip-Boy's query byte, the menu by class, the menus root, the `Menu` base's
//! and the lists' constructors, a tile's node and a property's word, the property kind (3), a
//! world space's parent, a setting's text and value, a string's characters and constructor, the
//! player's active quest and notes, a note's quest list, the challenges' list, flags and values, an
//! objective's quest, text and state, a quest's done flag, a form's flag, a list's count and whether
//! it holds a word, a node's next and item, a smart pointer's null constructor. A list box's
//! `RemoveAll`, `Sort`, `GetTileAt` and `+0x10` slots, the menus' deletion, class and click slots
//! and the node's `+0xe8` slot stay virtual: the object's class is not known here.
//!
//! **Floats.** The main thread's: each x87 step the `f64` result rounded once to the float (the
//! 24-bit step); the exe's double constants read where it reads them; a float the exe moves
//! through the x87 stack quieted (`x87_quiet`).

use super::interface_cpp::{self, in_place};
use super::interface_cpp_p2::{self, name_characters};
use super::map_menu;
use super::menu_manager::{self, empty_list};
use super::menu_manager_p2::{
    child_by_name, ftol2, tile_float, tile_set_float, tile_set_int, trait_by_name, ADD_USER_TRAIT,
    ENABLED_TEXT, FREE,
};
use super::narrow;
use crate::{cell_refs, dispatch, hud, multibound, pathing};
use engine_core::prelude::*;
use engine_core::typed_call;
use engine_core::x87::x87_quiet;

/// What this file does in place besides the other files' lists: plain reads the exe calls (the
/// differential tests leave these out of the translation's calls).
pub const IN_PLACE: &[u32] = &[
    0x0043_8220, // the property kind `GetProperty` takes: 3
    0x009c_8640, // a property's word at +0x60
    0x004f_d380, // a world space's parent (+0x70)
    0x0044_edb0, // the word at +0x10 (an objective's quest)
    0x005c_bb50, // the player's active quest (+0x6b8)
    0x0096_6a50, // the player's count of notes (the list at +0x5e4)
    0x004f_adc0, // a note's quest list (+0x74)
    0x0046_1150, // the challenges' list (+0x98)
    0x005f_5930, // a challenge's flag (bit 0 of the word at +0x5c)
    0x005a_60d0, // a challenge's bit 0 of +0x70
    0x005a_6150, // a challenge's bit 1 of +0x70
    0x005d_ef70, // a challenge's bit 2 of +0x70
    0x005e_3fa0, // a challenge's value (+0x6c)
    0x006a_1b60, // a challenge's kind (+0x54)
    0x005a_5dc0, // an objective's state above 1 (+0x20, signed)
    0x005e_c330, // an objective's text (its +8, or the empty text)
    0x0059_e400, // a quest's done flag (bit 1 of the byte at +0x3c)
    0x0044_0da0, // a form's flag 0x800 (+8)
];

/// `MapMenu::pMe`, the map menu's vtable and its class (`0x3ff`).
const SINGLETON: u32 = 0x011d_a368;
const VTABLE: u32 = 0x0107_4d44;
const CLASS: u32 = 0x3ff;
/// `pTiles` (+0x28, 22 of them), the page (+0x80), the map tile (+0x84), the waveform's start time
/// (+0x88), "showing all notes" (+0x8c), the selected note (+0x90), the displayed quest (+0xcc),
/// the root map space and whether it changed (+0x10c, +0x110), the drag start (+0x120), the local
/// map's next grid (+0x128).
const TILES: u32 = 0x28;
const TILE_COUNT: u32 = 0x16;
const PAGE: u32 = 0x80;
const MAP_TILE: u32 = 0x84;
const WAVEFORM_START: u32 = 0x88;
const SHOWING_ALL_NOTES: u32 = 0x8c;
const SELECTED_NOTE: u32 = 0x90;
const DISPLAYED_QUEST: u32 = 0xcc;
const ROOT_MAP_SPACE: u32 = 0x10c;
const ROOT_SPACE_CHANGED: u32 = 0x110;
const DRAG_START: u32 = 0x120;
const NEXT_LOCAL_GRID: u32 = 0x128;
/// The members: the sound list (+0x98), the subtitle array (+0xa8), the two marker lists (+0xd8,
/// +0xe0), the five list boxes (quests +0x130, notes +0x160, radio stations +0x190, objectives
/// +0x1c0, challenges +0x1f0), the waypoint array (+0x220).
const SOUNDS: u32 = 0x98;
const SUBTITLES: u32 = 0xa8;
const MARKERS_A: u32 = 0xd8;
const MARKERS_B: u32 = 0xe0;
const QUESTS: u32 = 0x130;
const NOTES: u32 = 0x160;
const RADIO: u32 = 0x190;
const OBJECTIVES: u32 = 0x1c0;
const CHALLENGES: u32 = 0x1f0;
const WAYPOINTS: u32 = 0x220;

/// The statics: the trait ids `MapMenu::MapMenu` names (title, value, current tab, selected, text
/// alpha, x, y, magnification) and their names.
const TRAIT_TITLE: u32 = 0x011d_a354;
const TRAIT_SELECTED: u32 = 0x011d_a348;
const TRAIT_TEXT_ALPHA: u32 = 0x011d_a344;
const TRAIT_MAGNIFICATION: u32 = 0x011d_a35c;
const TRAIT_NAMES: [(u32, u32); 8] = [
    (TRAIT_TITLE, 0x0106_ed20),
    (0x011d_a358, 0x0106_ed18),
    (0x011d_a360, 0x0107_3994),
    (TRAIT_SELECTED, 0x0107_1b64),
    (TRAIT_TEXT_ALPHA, 0x0107_3cb4),
    (0x011d_a350, 0x0106_ec90),
    (0x011d_a34c, 0x0106_f8f4),
    (TRAIT_MAGNIFICATION, 0x0107_4d2c),
];
/// The byte "the world map's texture was cleared", the challenge mode, the challenge type shown
/// and the challenge types' names (settings).
const TEXTURE_CLEARED: u32 = 0x011d_a36c;
const CHALLENGE_MODE: u32 = 0x011d_a36d;
const CHALLENGE_TYPE: u32 = 0x011d_a36e;
const CHALLENGE_TYPE_NAMES: u32 = 0x011a_03e4;
/// The player, the calendar, the player marker's start point, the word `007a1480` answers, the
/// station `0079c070` answers, the stations' list, the challenges' owner, the byte `00798440`
/// answers.
const PLAYER: u32 = 0x011d_ea3c;
const CALENDAR: u32 = 0x011d_e7b8;
const MARKER_START: u32 = 0x011f_426c;
const VALUE_011DEA14: u32 = 0x011d_ea14;
const CURRENT_STATION: u32 = 0x011d_d42c;
const STATIONS: u32 = 0x011d_d59c;
const CHALLENGE_OWNER: u32 = 0x011c_3f2c;
const FLAG_011DD434: u32 = 0x011d_d434;
/// The grid setting (`011c63cc`) whose value `0043d4d0` answers (its `+4`, a constant object).
const GRID_SETTING: u32 = 0x011c_63cc;
/// The renderer's height word (`004dc200`'s).
const RENDERER_HEIGHT: u32 = 0x0118_9480;

/// The player's `+0x6b8` (the active quest), its notes list (`+0x5e4`); a note's name (`+0x48`)
/// and quest list (`+0x74`); a quest's name (`+0x30`) and flags byte (`+0x3c`); a challenge's
/// name (`+0x18`), kind (`+0x54`), flags (`+0x5c`), value (`+0x6c`) and state (`+0x70`); an
/// objective's quest (`+0x10`), text (`+8`) and state (`+0x20`); the challenges' list of the owner
/// (`+0x98`); a world space's parent (`+0x70`); a property's word (`+0x60`).
const ACTIVE_QUEST: u32 = 0x6b8;
const PLAYER_NOTES: u32 = 0x5e4;
const NOTE_NAME: u32 = 0x48;
const NOTE_QUESTS: u32 = 0x74;
const QUEST_NAME: u32 = 0x30;
const QUEST_FLAGS: u32 = 0x3c;
const CHALLENGE_NAME: u32 = 0x18;
const CHALLENGE_KIND: u32 = 0x54;
const CHALLENGE_FLAGS: u32 = 0x5c;
const CHALLENGE_VALUE: u32 = 0x6c;
const CHALLENGE_STATE: u32 = 0x70;
const OBJECTIVE_QUEST: u32 = 0x10;
const OBJECTIVE_TEXT: u32 = 8;
const OBJECTIVE_STATE: u32 = 0x20;
const OWNER_CHALLENGES: u32 = 0x98;
const WORLD_PARENT: u32 = 0x70;
const PROPERTY_WORD: u32 = 0x60;
/// The property kind `NiAVObject::GetProperty` gets (`00438220`: 3).
const PROPERTY_KIND: i32 = 3;
/// The empty text.
const EMPTY_TEXT: u32 = 0x0101_1584;

/// The constants: `map_menu.xml`, the failure warning, the list boxes' template, the double
/// values of trait `0xfa7` that give the menu the top depth (6006.0, 102.0), the tab line's
/// names (settings), the float -1.0, the doubles 0.0, 1.0 and 60.0, the floats 255.0 and 128.0
/// and the doubles 255.0 and 128.0, the controller's tile position (floats), the scale 960.0, the
/// objective's width (a float), the date's format, the language setting and `"English"`, the
/// titles' settings and names, the headline's name, the tab line's map child's setting, the
/// headline's setting.
const MAP_MENU_XML: u32 = 0x0107_4df8;
const CREATION_FAILED_TEXT: u32 = 0x0107_4da8;
const LIST_TEMPLATE: u32 = 0x0107_4d8c;
const DEPTH_VALUE_A: u32 = 0x0107_04f0;
const DEPTH_VALUE_B: u32 = 0x0107_04e8;
const TABLINE_ENTRIES: [u32; 5] = [
    0x011d_4aec,
    0x011d_28a4,
    0x011d_5020,
    0x011d_204c,
    0x011d_38ac,
];
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
const ZERO_DOUBLE: u32 = 0x0101_2060;
const ONE_DOUBLE: u32 = 0x0101_2070;
const SIXTY_DOUBLE: u32 = 0x0101_2638;
const FULL_ALPHA_FLOAT: u32 = 0x0102_3cd8;
const HALF_ALPHA_FLOAT: u32 = 0x0101_e704;
const FULL_ALPHA_DOUBLE: u32 = 0x0101_e568;
const HALF_ALPHA_DOUBLE: u32 = 0x0102_e430;
const CONTROLLER_X: u32 = 0x0107_4f74;
const CONTROLLER_Y: u32 = 0x0107_4f70;
const CURSOR_SCALE: u32 = 0x0106_e7f8;
const OBJECTIVE_WIDTH: u32 = 0x0107_4ffc;
const DATE_TIME_FORMAT: u32 = 0x0107_4fdc;
const LANGUAGE_SETTING: u32 = 0x011c_3cb4;
const ENGLISH_TEXT: u32 = 0x0101_8348;
const TITLE_ROWS: [(u32, [u32; 5]); 2] = [
    (
        11,
        [
            0x011d_1da8,
            0x011d_3f64,
            0x011d_45f4,
            0x011d_3414,
            0x011d_2f10,
        ],
    ),
    (
        12,
        [
            0x011d_3180,
            0x011d_4cfc,
            0x011d_27a8,
            0x011d_3798,
            0x011d_3210,
        ],
    ),
];
const TITLE_NAMES: [u32; 5] = [
    0x0107_3d30,
    0x0107_4fd0,
    0x0107_4fc4,
    0x0107_4fb8,
    0x0107_4fac,
];
const HEADLINE_TITLE_TEXT: u32 = 0x0107_4f98;
const MAP_TAB_SETTING: u32 = 0x011d_4aec;
const HEADLINE_SETTING: u32 = 0x011d_21e4;
/// `"_ShowingAllNotes"`, `"_ShowEmptyMarker"`, `"MM_DataText"`; the notes' and challenges'
/// headings (settings); the waveform image's format (a setting) and the three waveform names.
const SHOWING_ALL_NOTES_TEXT: u32 = 0x0107_4e64;
const SHOW_EMPTY_MARKER_TEXT: u32 = 0x0107_4fe8;
const DATA_TEXT_NAME: u32 = 0x0107_4edc;
const NOTES_HEADING: u32 = 0x011d_2a78;
const CHALLENGES_HEADING: u32 = 0x011d_2d18;
const WAVEFORM_FORMAT: u32 = 0x011d_505c;
const WAVEFORM_NAMES: [u32; 3] = [0x0104_8458, 0x0104_844c, 0x0104_8444];
/// The waveform's offset (a float), its last step's time and the doubles of its step and frame.
const WAVEFORM_OFFSET: u32 = 0x011d_a414;
const WAVEFORM_STEP_TIME: u32 = 0x011d_a418;
const WAVEFORM_STEP: u32 = 0x0101_ffa0;
const WAVEFORM_FRAME: u32 = 0x0107_5000;

/// The callbacks the code hands the list boxes: the notes filter (`0079b390`), the challenges'
/// filter and sort (`0079b720`, `0079b820`), the quests' sort (`0079bc90`), the objectives' quest
/// filter (`0079bc50`), the radio list's filter and sort (`0079be30`, `0079bd70`).
const NOTES_FILTER: u32 = 0x0079_b390;
const CHALLENGES_FILTER: u32 = 0x0079_b720;
const CHALLENGES_SORT: u32 = 0x0079_b820;
const QUESTS_SORT: u32 = 0x0079_bc90;
const OBJECTIVES_FILTER: u32 = 0x0079_bc50;
const RADIO_FILTER: u32 = 0x0079_be30;
const RADIO_SORT: u32 = 0x0079_bd70;

/// The virtual slots: a menu's deleting destructor (+0) and class (+0x34) and the map menu's click
/// (+0x10); a list box's tile at an index (+0x14), its slot +0x10, `RemoveAll` (+0x1c) and `Sort`
/// (+0x20), and its highlighted tile (+4); a node's +0xe8 (an item handed back); an array's
/// `Allocate` (+4).
const DELETE_SLOT: u32 = 0;
const CLASS_SLOT: u32 = 0x34;
const CLICK_SLOT: u32 = 0x10;
const TILE_AT_SLOT: u32 = 0x14;
const LIST_SLOT_10: u32 = 0x10;
const REMOVE_ALL_SLOT: u32 = 0x1c;
const SORT_SLOT: u32 = 0x20;
const HIGHLIGHTED_SLOT: u32 = 4;
const NODE_ITEM_SLOT: u32 = 0xe8;
const ALLOCATE_SLOT: u32 = 4;

/// The by-address callees.
const TILE_READ_FILE: u32 = 0x00a0_1b00;
const TILE_GET_MENU: u32 = 0x00a0_3c90;
const TILE_GET_CHILD_BY_ID: u32 = 0x00a0_3eb0;
const TILE_DELETE_CHILDREN: u32 = 0x00a0_4150;
const TILE_REMOVE_FADE_CONTROL: u32 = 0x00a0_7dc0;
const TILE_GET_MAXIMUM_DEPTH: u32 = 0x00a0_1530;
const VALUE_CALCULATE: u32 = 0x00a0_9410;
const MENU_REMOVE_FROM_XML_ARCHIVE: u32 = 0x00a1_ca20;
const MENU_SET_MENU_TILE: u32 = 0x00a1_dc70;
const MENU_PREP_FOR_VISIBILITY: u32 = 0x00a1_dc20;
const MENU_DESTRUCT: u32 = 0x00a1_c520;
const CALENDAR_MARK_STALE: u32 = 0x0086_7a20;
const CALENDAR_GET_HOUR: u32 = 0x0086_7da0;
const CALENDAR_GET_DATE_STRING: u32 = 0x0086_7970;
const PRINT_WARNING: u32 = 0x005b_5e40;
const TUTORIAL_SHOW_MESSAGE: u32 = 0x0071_8630;
const INITIALIZE_TABLINE: u32 = 0x0070_7be0;
const LIST_BOX_INITIALIZE: u32 = 0x0075_a0d0;
const LIST_BOX_GET_TILE_FOR: u32 = 0x007a_22d0;
const PROPERTY_SET_007700D0: u32 = 0x0077_00d0;
const SET_TILE_TEXTURE: u32 = 0x00bb_7a10;
const SOUND_LIST_CONSTRUCT: u32 = 0x0076_b930;
const SOUND_LIST_DESTRUCT: u32 = 0x0076_b8a0;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
const LIST_REMOVE_ALL: u32 = 0x0047_0470;
const WAYPOINTS_CONSTRUCT: u32 = 0x0071_9b80;
const AUDIO: u32 = 0x011f_6d98;
const AUDIO_GET_RADIO_VOLUME: u32 = 0x00ad_7ae0;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
const STRING_FORMAT: u32 = 0x0040_6f60;
const BUFFER_FORMAT: u32 = 0x0040_6d00;
const BUFFER_COPY: u32 = 0x0040_6d30;
const COMPARE: u32 = 0x0040_8b20;
const PLAYER_MESSAGE: u32 = 0x0057_8870;
const SET_TAB_KNOB_POSITION: u32 = 0x007f_a0f0;
const SET_CURRENT_FOCUS_TARGET: u32 = 0x0071_5860;
const MANAGER_STATE_TEST: u32 = 0x004b_71d0;
const FORM_FLAG_004FEE60: u32 = 0x004f_ee60;
const CONVERSATION_DESTRUCT: u32 = 0x0083_b8d0;
const ARRAY_SHRUNK_SIZE: u32 = 0x0086_9600;
const ARRAY_MOVE: u32 = 0x0069_2480;
const UPDATE_LOCAL_MAP: u32 = 0x0079_d410;
const UPDATE_WORLD_MAP: u32 = 0x0079_cdb0;
const LOAD_NOTE: u32 = 0x0079_93d0;
const PLAYER_NOTE_AT: u32 = 0x0096_6bb0;
const OBJECTIVE_SHOWN: u32 = 0x005a_5e70;
const STATION_NAME: u32 = 0x0055_d520;
const GET_RADIO_STATIONS_IN_RANGE: u32 = 0x004f_f1a0;
const CLOCK: u32 = 0x0045_7fe0;
const RANDOM: u32 = 0x0048_7f50;

// ---- Helpers --------------------------------------------------------------------------------

/// `MapMenu::pMe`.
#[inline]
fn singleton(e: &Engine) -> u32 {
    e.mem.u32(SINGLETON)
}

/// The tile pointer `index` of the menu at `menu` (`pTiles`).
#[inline]
fn map_tile(e: &Engine, menu: u32, index: u32) -> u32 {
    e.mem.u32(menu.wrapping_add(TILES + 4 * index))
}

/// `Tile::SetString` with the flag 1 (the dispatch family's).
#[inline]
fn set_text(e: &mut Engine, tile: u32, id: i32, text: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1350,
        dispatch::tile::set_string(e, Ptr::new(tile), id, Ptr::new(text), 1)
    );
}

/// A static trait id (`MapMenu::MapMenu`'s statics).
#[inline]
fn static_id(e: &Engine, at: u32) -> i32 {
    e.mem.i32(at)
}

/// The tile shader property of a tile (`ClearMapMemory`, `ClearWorldMapTexture`, `Create` and the
/// waveform): the tile's node (`0056c7f0`, in place), its child 0 (`0045bc00`, the multibound
/// family's), that one's property of kind 3 (`NiAVObject::GetProperty`, the cell family's); 0
/// where a step gives 0.
fn tile_shader_property(e: &mut Engine, tile: u32) -> u32 {
    let object = in_place::tile_node(e, tile);
    if object == 0 {
        return 0;
    }
    let node = typed_call!(
        multibound::FAMILY,
        0x0045_bc00,
        multibound::getters::node_child(e, Ptr::new(object), 0)
    );
    if node == 0 {
        return 0;
    }
    typed_call!(
        cell_refs::FAMILY,
        0x00a5_9d30,
        cell_refs::scenegraph::get_property(e, node, PROPERTY_KIND)
    )
}

/// The text alpha (static trait) of a list item: the float 128.0 when `dim`, else 255.0.
fn set_item_alpha(e: &mut Engine, tile: u32, dim: bool) {
    let alpha = x87_quiet(e.mem.f32(if dim {
        HALF_ALPHA_FLOAT
    } else {
        FULL_ALPHA_FLOAT
    }));
    let id = static_id(e, TRAIT_TEXT_ALPHA);
    tile_set_float(e, tile, id, alpha);
}

/// The text alphas of two list items as the sorts compare them: 1 when the first is 128.0 and the
/// second 255.0, -1 the other way round (the exe's double constants).
fn alpha_order(e: &Engine, first: f32, second: f32) -> Option<i32> {
    let half: f64 = e.global(HALF_ALPHA_DOUBLE);
    let full: f64 = e.global(FULL_ALPHA_DOUBLE);
    let (first, second) = (f64::from(first), f64::from(second));
    if first == half && second == full {
        Some(1)
    } else if first == full && second == half {
        Some(-1)
    } else {
        None
    }
}

/// The text alphas of the items whose tiles are the first words of `first` and `second` (the trait
/// id read again after the first `Tile::GetFloat`, as the exe does).
fn item_alphas(e: &mut Engine, first: u32, second: u32) -> (f32, f32) {
    let id = static_id(e, TRAIT_TEXT_ALPHA);
    let tile = e.mem.u32(first);
    let a = tile_float(e, tile, id);
    let id = static_id(e, TRAIT_TEXT_ALPHA);
    let tile = e.mem.u32(second);
    (a, tile_float(e, tile, id))
}

/// The number of a list's nodes with an item (`005ae380`, in place).
fn item_count(e: &Engine, list: u32) -> u32 {
    let mut count = 0u32;
    let mut node = list;
    while node != 0 {
        if e.mem.u32(node) != 0 {
            count = count.wrapping_add(1);
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    count
}

/// Whether a list holds `word` (`005f65d0`, in place).
fn list_holds(e: &Engine, list: u32, word: u32) -> bool {
    let mut node = list;
    while node != 0 {
        if e.mem.u32(node) == word {
            return true;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    false
}

/// `Interface::InitializeTabline` (`00707be0`, cdecl with a variable count): this family's, with
/// its words, as `typed_call!` would make the call.
fn initialize_tabline(e: &mut Engine, words: &[u32]) {
    if e.typed_direct(super::FAMILY, INITIALIZE_TABLINE) {
        if cfg!(debug_assertions) && e.typed_noting_direct() {
            e.typed_note_direct(INITIALIZE_TABLINE, words);
        }
        e.typed_slice_step(INITIALIZE_TABLINE);
        interface_cpp_p2::interface_initialize_tabline(e, words);
    } else {
        e.call(INITIALIZE_TABLINE, words);
    }
}

// ---- The menu's life --------------------------------------------------------------------------

// Typed from 0072d040
/// `MapMenu::AttachTileByID` (Xbox PDB): `tile` as `pTiles[id]` for an id in `0..0x16`.
pub fn map_menu_attach_tile_by_id(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    if let Ok(index @ 0..TILE_COUNT) = u32::try_from(id) {
        e.mem
            .set_u32(this.addr().wrapping_add(TILES + 4 * index), tile);
    }
}

// Typed from 00796740
/// `MapMenu::MapMenu` (Xbox PDB): the `Menu` base and the vtable, the members (sound list, subtitle
/// array, marker lists, the five list boxes, the waypoint array; the four points need no work), the
/// eight static trait ids by name, then the fields: page 0x24, all notes shown, fog of war on, the
/// player marker at `011f426c`'s point and the radio volume kept. `this`.
pub fn map_menu_map_menu(e: &mut Engine, this: Ptr) -> Ptr {
    let at = this.addr();
    menu_manager::menu_base(e, at);
    e.mem.set_u32(at, VTABLE);
    e.call(SOUND_LIST_CONSTRUCT, &args![at.wrapping_add(SOUNDS)]);
    typed_call!(
        super::FAMILY,
        0x007a_1a70,
        map_menu::fn_007a1a70(e, Ptr::new(at.wrapping_add(SUBTITLES)))
    );
    empty_list(e, at.wrapping_add(MARKERS_A));
    empty_list(e, at.wrapping_add(MARKERS_B));
    typed_call!(
        super::FAMILY,
        0x007a_1c90,
        map_menu::fn_007a1c90(e, Ptr::new(at.wrapping_add(QUESTS)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_27f0,
        map_menu::fn_007a27f0(e, Ptr::new(at.wrapping_add(NOTES)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_36e0,
        map_menu::fn_007a36e0(e, Ptr::new(at.wrapping_add(RADIO)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_43a0,
        map_menu::fn_007a43a0(e, Ptr::new(at.wrapping_add(OBJECTIVES)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_50f0,
        map_menu::fn_007a50f0(e, Ptr::new(at.wrapping_add(CHALLENGES)))
    );
    e.call(WAYPOINTS_CONSTRUCT, &args![at.wrapping_add(WAYPOINTS)]);
    for (cache, name) in TRAIT_NAMES {
        let id = trait_by_name(e, name);
        e.mem.set_i32(cache, id);
    }
    e.mem.set_f32(at.wrapping_add(WAVEFORM_START), 0.0);
    e.mem.set_u8(at.wrapping_add(SHOWING_ALL_NOTES), 1);
    e.mem.set_u32(at.wrapping_add(0xb8), 0);
    e.mem.set_u8(at.wrapping_add(0xbc), 0);
    e.mem.set_u32(at.wrapping_add(0x118), 0);
    e.mem.set_u32(at.wrapping_add(PAGE), 0x24);
    for word in 0..3 {
        let value = e.mem.u32(MARKER_START + 4 * word);
        e.mem.set_u32(at.wrapping_add(0xfc + 4 * word), value);
    }
    for offset in [
        0xf8u32,
        0x108,
        MAP_TILE,
        DISPLAYED_QUEST,
        SELECTED_NOTE,
        0x11c,
        0xd4,
        0x94,
    ] {
        e.mem.set_u32(at.wrapping_add(offset), 0);
    }
    let audio = e.mem.u32(AUDIO);
    let volume = narrow(e.call(AUDIO_GET_RADIO_VOLUME, &args![audio]).f64());
    e.mem.set_f32(at.wrapping_add(0xc8), volume);
    for offset in [0xd0u32, NEXT_LOCAL_GRID, 0x114] {
        e.mem.set_u32(at.wrapping_add(offset), 0);
    }
    e.mem.set_u8(at.wrapping_add(0x12c), 1);
    e.mem.set_u32(at.wrapping_add(ROOT_MAP_SPACE), 0);
    e.mem.set_u8(at.wrapping_add(ROOT_SPACE_CHANGED), 0);
    this
}

// Typed from 00796a40
/// `MapMenu::GetClass` (Xbox PDB): `0x3ff`.
pub fn map_menu_get_class(_e: &mut Engine, _this: Ptr) -> u32 {
    CLASS
}

// Typed from 00796a50
/// `MapMenu`'s scalar deleting destructor: the destructor body, then the object freed when bit 0
/// of `flags` is set. `this`.
pub fn map_menu_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x0079_6a80, fn_00796a80(e, this));
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// Typed from 00796a80
/// The `MapMenu` destructor body: the vtable, `MapMenu::pMe` cleared, the members destroyed in
/// reverse order (waypoint array, the five list boxes, the marker lists, the subtitle array, the
/// sound list), then the `Menu` base's destructor.
pub fn fn_00796a80(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    e.mem.set_u32(at, VTABLE);
    e.mem.set_u32(SINGLETON, 0);
    typed_call!(
        super::FAMILY,
        0x0071_9be0,
        super::item_list_box::fn_00719be0(e, Ptr::new(at.wrapping_add(WAYPOINTS)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_51c0,
        map_menu::fn_007a51c0(e, Ptr::new(at.wrapping_add(CHALLENGES)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_4470,
        map_menu::fn_007a4470(e, Ptr::new(at.wrapping_add(OBJECTIVES)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_37b0,
        map_menu::fn_007a37b0(e, Ptr::new(at.wrapping_add(RADIO)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_28c0,
        map_menu::fn_007a28c0(e, Ptr::new(at.wrapping_add(NOTES)))
    );
    typed_call!(
        super::FAMILY,
        0x007a_1d60,
        map_menu::fn_007a1d60(e, Ptr::new(at.wrapping_add(QUESTS)))
    );
    e.call(LIST_DESTRUCT, &args![at.wrapping_add(MARKERS_B)]);
    e.call(LIST_DESTRUCT, &args![at.wrapping_add(MARKERS_A)]);
    typed_call!(
        super::FAMILY,
        0x007a_1aa0,
        map_menu::fn_007a1aa0(e, Ptr::new(at.wrapping_add(SUBTITLES)))
    );
    e.call(SOUND_LIST_DESTRUCT, &args![at.wrapping_add(SOUNDS)]);
    e.call(MENU_DESTRUCT, &args![at]);
}

// Typed from 00796b90
/// `MapMenu::Create` (Xbox PDB): an open map menu deleted (its slot +0 with 1), `map_menu.xml`
/// read into the menus root, its menu taken as `MapMenu::pMe` and the file removed from the XML
/// archive. A menu that is not the map menu (class 0x3ff) is deleted again (when it has a tile) and
/// 0 answered. Otherwise the calendar's days are marked, the menu's tile set, and unless a tile is
/// missing (a warning, 0 answered) the tile's depth (trait `0xfad`) set to the top one when trait
/// `0xfa7` is 6006.0 or 102.0, the tutorial message 0x1a shown, the tab line and the five list
/// boxes set up, the map's property updated, the title emptied, tiles 5 and 3 given the flag
/// 0x8000, the map memory cleared and the menu prepared for visibility. The tile.
pub fn map_menu_create(e: &mut Engine) -> u32 {
    let manager = in_place::manager(e);
    let old = in_place::menu_by_class(e, 0x3ff);
    if old != 0 {
        e.vcall(old, DELETE_SLOT, &args![1u32]);
    }
    let root = in_place::menus_root(e, manager);
    let tile = e.call(TILE_READ_FILE, &args![root, MAP_MENU_XML]).u32();
    let menu = e.call(TILE_GET_MENU, &args![tile]).u32();
    e.mem.set_u32(SINGLETON, menu);
    e.call(MENU_REMOVE_FROM_XML_ARCHIVE, &args![MAP_MENU_XML, menu]);
    let menu = singleton(e);
    if menu == 0 || e.vcall(menu, CLASS_SLOT, &args![]).u32() != CLASS {
        let menu = singleton(e);
        if menu != 0 && e.mem.u32(menu.wrapping_add(4)) != 0 {
            e.vcall(menu, DELETE_SLOT, &args![1u32]);
        }
        return 0;
    }
    e.call(CALENDAR_MARK_STALE, &args![menu]);
    let menu = singleton(e);
    e.call(MENU_SET_MENU_TILE, &args![menu, tile, 0u32]);
    let menu = singleton(e);
    if !typed_call!(super::FAMILY, 0x0079_6f90, fn_00796f90(e, Ptr::new(menu))) {
        e.call(PRINT_WARNING, &args![CREATION_FAILED_TEXT]);
        return 0;
    }
    let first: f64 = e.global(DEPTH_VALUE_A);
    let top = f64::from(tile_float(e, tile, 0xfa7)) == first || {
        let second: f64 = e.global(DEPTH_VALUE_B);
        f64::from(tile_float(e, tile, 0xfa7)) == second
    };
    if top {
        let depth = narrow(e.call(TILE_GET_MAXIMUM_DEPTH, &args![]).f64());
        tile_set_float(e, tile, 0xfad, depth);
    }
    let menu_id = typed_call!(super::FAMILY, 0x0072_0df0, menu_manager::fn_00720df0(e));
    let manager = in_place::manager(e);
    e.call(
        TUTORIAL_SHOW_MESSAGE,
        &args![manager.wrapping_add(0x4d4), 0x1au32, menu_id, 0x200u32],
    );
    let names = TABLINE_ENTRIES.map(|entry| in_place::setting_text(e, entry));
    let tabline = map_tile(e, singleton(e), 17);
    initialize_tabline(
        e,
        &[
            tabline, 0x20, names[4], names[3], names[2], names[1], names[0], 0,
        ],
    );
    for (list, index, first_flag) in [
        (QUESTS, 7u32, true),
        (OBJECTIVES, 15, true),
        (NOTES, 8, true),
        (CHALLENGES, 19, false),
        (RADIO, 10, false),
    ] {
        let menu = singleton(e);
        let list_tile = map_tile(e, menu, index);
        e.call(
            LIST_BOX_INITIALIZE,
            &args![menu.wrapping_add(list), list_tile, LIST_TEMPLATE],
        );
        if first_flag {
            let list = singleton(e).wrapping_add(list);
            typed_call!(
                super::FAMILY,
                0x007a_1930,
                map_menu::fn_007a1930(e, Ptr::new(list), 0)
            );
        }
    }
    let map = map_tile(e, singleton(e), 16);
    let property = tile_shader_property(e, map);
    if property != 0 {
        e.call(PROPERTY_SET_007700D0, &args![property, 1u32]);
    }
    let title = e
        .call(ADD_USER_TRAIT, &args![0x0106_ed20u32, u32::MAX])
        .i32();
    let title_tile = map_tile(e, singleton(e), 6);
    set_text(e, title_tile, title, EMPTY_TEXT);
    for index in [5u32, 3] {
        let flagged = map_tile(e, singleton(e), index);
        typed_call!(
            super::FAMILY,
            0x0079_6f40,
            fn_00796f40(e, Ptr::new(flagged), 1)
        );
    }
    typed_call!(super::FAMILY, 0x007a_1490, map_menu_clear_map_memory(e));
    let menu = singleton(e);
    e.call(MENU_PREP_FOR_VISIBILITY, &args![menu, 1u32]);
    tile
}

// Typed from 00796f40
/// Bit 0x8000 of the word at +0x30 (a tile's flags) set (`flag` non-zero) or cleared.
pub fn fn_00796f40(e: &mut Engine, this: Ptr, flag: u8) {
    let at = this.addr().wrapping_add(0x30);
    let word = e.mem.u32(at);
    e.mem.set_u32(
        at,
        if flag != 0 {
            word | 0x8000
        } else {
            word & !0x8000
        },
    );
}

// Typed from 00796f90
/// Whether all 22 `pTiles` are set.
pub fn fn_00796f90(e: &mut Engine, this: Ptr) -> bool {
    (0..TILE_COUNT).all(|index| map_tile(e, this.addr(), index) != 0)
}

// Typed from 00798450
/// The `Conversation` deleting destructor: its destructor body, then the object freed when bit 0
/// of `flags` is set. `this`.
pub fn fn_00798450(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(CONVERSATION_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// Typed from 0079aba0
/// `MapMenu::UpdateMenu` (Xbox PDB): with a map menu and the Pip-Boy's query byte set, the date and
/// time (`"%s, %d:%02d"`-style format of the calendar's date string, the hour and the minutes) as
/// tile 1's and tile 0's text (and handed to the player between); outside English, the titles of
/// tiles 11 and 12 (`_title_1` to `_title_5` from settings, then their title values recalculated),
/// the tab line's child 0x24 and the headline from settings; then `UpdateCurrentPage`.
pub fn map_menu_update_menu(e: &mut Engine) {
    if singleton(e) == 0 {
        return;
    }
    let pipboy = typed_call!(
        super::FAMILY,
        0x0070_5990,
        interface_cpp::interface_get_pipboy(e)
    );
    if e.mem.u8(pipboy.wrapping_add(in_place::PIPBOY_QUERY)) == 0 {
        return;
    }
    e.with_stack(0x10, |e, strings| {
        let text = strings.addr();
        let date = text.wrapping_add(8);
        in_place::empty_string(e, text);
        let hour = narrow(e.call(CALENDAR_GET_HOUR, &args![CALENDAR]).f64());
        let whole = ftol2(e, f64::from(hour));
        let sixty: f64 = e.global(SIXTY_DOUBLE);
        let fraction = narrow(f64::from(hour) - f64::from(whole));
        let minutes = ftol2(e, f64::from(narrow(f64::from(fraction) * sixty)));
        let hours = ftol2(e, f64::from(hour));
        let date_string = e
            .call(CALENDAR_GET_DATE_STRING, &args![CALENDAR, date])
            .u32();
        let date_text = e.mem.u32(date_string);
        e.call(
            STRING_FORMAT,
            &args![text, DATE_TIME_FORMAT, date_text, hours, minutes],
        );
        e.call(STRING_DESTRUCT, &args![date]);
        let characters = e.mem.u32(text);
        let tile = map_tile(e, singleton(e), 1);
        set_text(e, tile, 0xfc4, characters);
        let player = e.mem.u32(PLAYER);
        e.call(PLAYER_MESSAGE, &args![player, text]);
        let characters = e.mem.u32(text);
        let tile = map_tile(e, singleton(e), 0);
        set_text(e, tile, 0xfc4, characters);
        let language = in_place::setting_text(e, LANGUAGE_SETTING);
        if e.call(COMPARE, &args![language, ENGLISH_TEXT]).u32() != 0 {
            for (index, entries) in TITLE_ROWS {
                for (entry, name) in entries.into_iter().zip(TITLE_NAMES) {
                    let title = in_place::setting_text(e, entry);
                    let id = trait_by_name(e, name);
                    let tile = map_tile(e, singleton(e), index);
                    set_text(e, tile, id, title);
                }
                let id = static_id(e, TRAIT_TITLE);
                let tile = map_tile(e, singleton(e), index);
                let value = typed_call!(
                    dispatch::FAMILY,
                    0x00a0_0f30,
                    dispatch::tile::get_value(e, Ptr::new(tile), id)
                );
                e.call(VALUE_CALCULATE, &args![value, 1u32]);
            }
            let name = in_place::setting_text(e, MAP_TAB_SETTING);
            let tabline = map_tile(e, singleton(e), 17);
            let child = e.call(TILE_GET_CHILD_BY_ID, &args![tabline, 0x24u32]).u32();
            set_text(e, child, 0xfc4, name);
            let headline = in_place::setting_text(e, HEADLINE_SETTING);
            let menu_tile = e.mem.u32(singleton(e).wrapping_add(4));
            let child = child_by_name(e, menu_tile, HEADLINE_TITLE_TEXT);
            set_text(e, child, 0xfc4, headline);
        }
        let menu = singleton(e);
        typed_call!(
            super::FAMILY,
            0x0079_af30,
            map_menu_update_current_page(e, Ptr::new(menu))
        );
        e.call(STRING_DESTRUCT, &args![text]);
    });
}

// ---- Small members ----------------------------------------------------------------------------

// Typed from 007983c0
/// The cursor's y (the code calls it on the interface manager): the float at +0x4b0 times 960.0,
/// over the renderer's height.
pub fn fn_007983c0(e: &mut Engine, this: Ptr) -> f32 {
    let value = x87_quiet(e.mem.f32(this.addr().wrapping_add(0x4b0)));
    let scale: f64 = e.global(CURSOR_SCALE);
    let product = narrow(f64::from(value) * scale);
    let height = e.mem.i32(RENDERER_HEIGHT);
    narrow(f64::from(product) / f64::from(height))
}

// Typed from 00798400
/// The word at +0x6f4 (the code calls it on the player).
pub fn fn_00798400(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(0x6f4))
}

// Typed from 00798420
/// The byte at +0x7d (a note's "read" byte).
pub fn fn_00798420(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr().wrapping_add(0x7d))
}

// Typed from 00798440
/// The byte at `011dd434`.
pub fn fn_00798440(e: &mut Engine) -> u8 {
    e.mem.u8(FLAG_011DD434)
}

// Typed from 00798480
/// `MapMenu::DoDownClick` (Xbox PDB): a press on id 4 (the world map) keeps tile 4's position
/// (traits `0xfa1`, `0xfa2`) as the drag's start. The tile is not read.
pub fn map_menu_do_down_click(e: &mut Engine, this: Ptr, id: i32, _tile: u32) {
    if id != 4 {
        return;
    }
    let at = this.addr();
    for (k, id) in [0xfa1, 0xfa2].into_iter().enumerate() {
        let tile = map_tile(e, at, 4);
        let value = tile_float(e, tile, id);
        e.mem
            .set_f32(at.wrapping_add(DRAG_START + 4 * super::word(k)), value);
    }
}

// Typed from 0079a610
/// `MapMenu::DoControllerSwitch` (Xbox PDB): with `on`, tile 6 moved to the floats at `01074f74`
/// (x, trait `0xfa1`) and `01074f70` (y, `0xfa2`).
pub fn map_menu_do_controller_switch(e: &mut Engine, this: Ptr, on: u8) {
    if on == 0 {
        return;
    }
    for (at, id) in [(CONTROLLER_X, 0xfa1), (CONTROLLER_Y, 0xfa2)] {
        let value = x87_quiet(e.mem.f32(at));
        let tile = map_tile(e, this.addr(), 6);
        tile_set_float(e, tile, id, value);
    }
}

// Typed from 0079b430
/// The open map menu's selected note cleared when it is `note` (cdecl).
pub fn fn_0079b430(e: &mut Engine, note: u32) {
    let menu = singleton(e);
    if menu != 0 && e.mem.u32(menu.wrapping_add(SELECTED_NOTE)) == note {
        e.mem.set_u32(menu.wrapping_add(SELECTED_NOTE), 0);
    }
}

// Typed from 0079b800
/// Whether bit 2 of the 16-bit word at +0x5c (a challenge's flags) is set.
pub fn fn_0079b800(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u16(this.addr().wrapping_add(CHALLENGE_FLAGS)) & 4 != 0
}

// Typed from 0079bc30
/// `this + 0x6bc` (the player's objectives).
pub fn fn_0079bc30(_e: &mut Engine, this: Ptr) -> u32 {
    this.addr().wrapping_add(0x6bc)
}

// Typed from 0079bc50
/// The objectives' quest filter (cdecl): 1 when `objective` is set and its quest is not the map
/// menu's displayed quest (`pMe` read without a check), else 0.
pub fn fn_0079bc50(e: &mut Engine, objective: u32) -> u8 {
    if objective == 0 {
        return 0;
    }
    let quest = e.mem.u32(objective.wrapping_add(OBJECTIVE_QUEST));
    u8::from(e.mem.u32(singleton(e).wrapping_add(DISPLAYED_QUEST)) != quest)
}

// Typed from 0079be30
/// The radio list's filter (cdecl): 1 for a null station, one with the form flag 0x800, or one
/// whose form (`+0x20`) `004fee60` (with 0) answers; else 0.
pub fn fn_0079be30(e: &mut Engine, station: u32) -> u8 {
    if station == 0 || e.mem.u32(station.wrapping_add(8)) & 0x800 != 0 {
        return 1;
    }
    let form = e.mem.u32(station.wrapping_add(0x20));
    u8::from(e.call(FORM_FLAG_004FEE60, &args![form, 0u32]).bool())
}

// Typed from 0079be80
/// The tile's selected trait (a static) set to 0 (cdecl).
pub fn fn_0079be80(e: &mut Engine, tile: u32) {
    let selected = static_id(e, TRAIT_SELECTED);
    tile_set_int(e, tile, selected, 0);
}

// Typed from 0079c070
/// The word at `011dd42c` (the current radio station's).
pub fn fn_0079c070(e: &mut Engine) -> u32 {
    e.mem.u32(CURRENT_STATION)
}

// Typed from 0079c080
/// The radio stations' list (`011dd59c`).
pub fn fn_0079c080(_e: &mut Engine) -> u32 {
    STATIONS
}

// Typed from 0079c090
/// The waveform's start time and tile 16's trait `0xfa3` cleared.
pub fn fn_0079c090(e: &mut Engine, this: Ptr) {
    e.mem.set_f32(this.addr().wrapping_add(WAVEFORM_START), 0.0);
    let tile = map_tile(e, this.addr(), 16);
    tile_set_int(e, tile, 0xfa3, 0);
}

// Typed from 0079c340
/// With an open map menu, the Pip-Boy's tab knob (`FOPipboyManager::SetTabKnobPosition`) moved to
/// the page less 0x20, with `flag` (cdecl).
pub fn fn_0079c340(e: &mut Engine, flag: u8) {
    let menu = singleton(e);
    if menu == 0 {
        return;
    }
    let page = e.mem.u32(menu.wrapping_add(PAGE)).wrapping_sub(0x20);
    let manager = in_place::manager(e);
    let pipboy = in_place::pipboy(e, manager);
    e.call(SET_TAB_KNOB_POSITION, &args![pipboy, page, u32::from(flag)]);
}

// Typed from 007a1400
/// The bits of `mask` cleared from the word at +0x30 when any is set (`XOR`, as the code does).
pub fn fn_007a1400(e: &mut Engine, this: Ptr, mask: u32) {
    let at = this.addr().wrapping_add(0x30);
    let word = e.mem.u32(at);
    if word & mask != 0 {
        e.mem.set_u32(at, word ^ mask);
    }
}

// Typed from 007a1430
/// `value` into the byte at +0x92.
pub fn fn_007a1430(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr().wrapping_add(0x92), value);
}

// Typed from 007a1450
/// The four words at `from` copied to +0x68.
pub fn fn_007a1450(e: &mut Engine, this: Ptr, from: u32) {
    for word in 0..4 {
        let value = e.mem.u32(from.wrapping_add(4 * word));
        e.mem
            .set_u32(this.addr().wrapping_add(0x68 + 4 * word), value);
    }
}

// Typed from 007a1480
/// The word at `011dea14`.
pub fn fn_007a1480(e: &mut Engine) -> u32 {
    e.mem.u32(VALUE_011DEA14)
}

// Typed from 007a1490
/// `MapMenu::ClearMapMemory` (Xbox PDB): with a map menu whose tile 2 has a node, each child of
/// the node handed to its slot +0xe8; tiles 2's and 4's children deleted, trait `0xfa3` of tiles 2
/// and 3 cleared, the fade controls `0xff5` of tiles 3 and 5 removed, both marker lists emptied,
/// the waypoints cleared, the local map's next grid and the map tile cleared; the map tile's
/// texture (tile 16's property) cleared with its text `0xfcc`, then `0079c090`.
pub fn map_menu_clear_map_memory(e: &mut Engine) {
    let menu = singleton(e);
    if menu == 0 {
        return;
    }
    let node = in_place::tile_node(e, map_tile(e, menu, 2));
    if node == 0 {
        return;
    }
    let mut index = 0u32;
    while index
        < u32::from(typed_call!(
            dispatch::FAMILY,
            0x0043_b480,
            dispatch::scene::child_count(e, Ptr::new(node))
        ))
    {
        let child = typed_call!(
            dispatch::FAMILY,
            0x0043_b4a0,
            dispatch::scene::child(e, Ptr::new(node), index)
        )
        .addr();
        if child != 0 {
            e.vcall(node, NODE_ITEM_SLOT, &args![child]);
        }
        index = index.wrapping_add(1);
    }
    let tile = map_tile(e, singleton(e), 2);
    e.call(TILE_DELETE_CHILDREN, &args![tile]);
    for index in [2u32, 3] {
        let tile = map_tile(e, singleton(e), index);
        tile_set_int(e, tile, 0xfa3, 0);
    }
    let tile = map_tile(e, singleton(e), 3);
    e.call(TILE_REMOVE_FADE_CONTROL, &args![tile, 0xff5u32]);
    let list = singleton(e).wrapping_add(MARKERS_B);
    e.call(LIST_REMOVE_ALL, &args![list]);
    let tile = map_tile(e, singleton(e), 4);
    e.call(TILE_DELETE_CHILDREN, &args![tile]);
    let tile = map_tile(e, singleton(e), 5);
    e.call(TILE_REMOVE_FADE_CONTROL, &args![tile, 0xff5u32]);
    let list = singleton(e).wrapping_add(MARKERS_A);
    e.call(LIST_REMOVE_ALL, &args![list]);
    let waypoints = singleton(e).wrapping_add(WAYPOINTS);
    typed_call!(
        dispatch::FAMILY,
        0x0084_54f0,
        dispatch::refs::array_clear(e, Ptr::new(waypoints), 1)
    );
    let menu = singleton(e);
    e.mem.set_u32(menu.wrapping_add(NEXT_LOCAL_GRID), 0);
    e.mem.set_u32(menu.wrapping_add(MAP_TILE), 0);
    let property = tile_shader_property(e, map_tile(e, menu, 16));
    if property != 0 && e.mem.u32(property.wrapping_add(PROPERTY_WORD)) != 0 {
        e.call(SET_TILE_TEXTURE, &args![property, 0u32]);
        let tile = map_tile(e, singleton(e), 16);
        set_text(e, tile, 0xfcc, EMPTY_TEXT);
    }
    let menu = singleton(e);
    typed_call!(super::FAMILY, 0x0079_c090, fn_0079c090(e, Ptr::new(menu)));
}

// Typed from 007a1670
/// `MapMenu::ClearWorldMapTexture` (Xbox PDB): unless the map menu is on top on the world map page
/// (0x21; `pMe` read without a check there), tile 4's property texture cleared (when it has one),
/// its text `0xfcc` emptied and the byte `011da36c` set.
pub fn map_menu_clear_world_map_texture(e: &mut Engine) {
    let on_top = typed_call!(
        super::FAMILY,
        0x0070_2450,
        interface_cpp::interface_is_top_menu_id(e, 0x3ff)
    ) != 0;
    if on_top && e.mem.u32(singleton(e).wrapping_add(PAGE)) == 0x21 {
        return;
    }
    let menu = singleton(e);
    if menu == 0 {
        return;
    }
    let property = tile_shader_property(e, map_tile(e, menu, 4));
    if property != 0 && e.mem.u32(property.wrapping_add(PROPERTY_WORD)) != 0 {
        e.call(SET_TILE_TEXTURE, &args![property, 0u32]);
        let tile = map_tile(e, singleton(e), 4);
        set_text(e, tile, 0xfcc, EMPTY_TEXT);
        e.mem.set_u8(TEXTURE_CLEARED, 1);
    }
}

/// `007a17a0` and `007a17e0`: the magnification (a static trait) of the map menu's tile `index`,
/// 1.0 without a map menu.
fn magnification(e: &mut Engine, index: u32) -> f32 {
    let menu = singleton(e);
    if menu == 0 {
        return 1.0;
    }
    let id = static_id(e, TRAIT_MAGNIFICATION);
    x87_quiet(tile_float(e, map_tile(e, menu, index), id))
}

// Typed from 007a17a0
/// The magnification of the map menu's tile 2 (the local map), 1.0 without a map menu.
pub fn fn_007a17a0(e: &mut Engine) -> f32 {
    magnification(e, 2)
}

// Typed from 007a17e0
/// The magnification of the map menu's tile 4 (the world map), 1.0 without a map menu.
pub fn fn_007a17e0(e: &mut Engine) -> f32 {
    magnification(e, 4)
}

// Typed from 007a1750
/// With an open map menu, whether the local map's next grid is below the square of the grid
/// setting's value (unsigned, the product wrapping).
pub fn fn_007a1750(e: &mut Engine) -> bool {
    let menu = singleton(e);
    if menu == 0 {
        return false;
    }
    let side = e.mem.u32(GRID_SETTING.wrapping_add(4));
    e.mem.u32(menu.wrapping_add(NEXT_LOCAL_GRID)) < side.wrapping_mul(side)
}

/// `007a1820` and `007a1850`: with an open map menu, the magnification (a static trait) of tile
/// `index` set to `value`.
fn set_magnification(e: &mut Engine, index: u32, value: f32) {
    let menu = singleton(e);
    if menu == 0 {
        return;
    }
    let id = static_id(e, TRAIT_MAGNIFICATION);
    tile_set_float(e, map_tile(e, menu, index), id, x87_quiet(value));
}

// Typed from 007a1820
/// The magnification of the map menu's tile 2 set (cdecl).
pub fn fn_007a1820(e: &mut Engine, value: f32) {
    set_magnification(e, 2, value);
}

// Typed from 007a1850
/// The magnification of the map menu's tile 4 set (cdecl).
pub fn fn_007a1850(e: &mut Engine, value: f32) {
    set_magnification(e, 4, value);
}

// Typed from 007a1880
/// `MapMenu::SetRootMapSpace` (Xbox PDB): with a map menu, `space` walked up to its top parent
/// world as the root map space; whether it differs from the old one kept.
pub fn map_menu_set_root_map_space(e: &mut Engine, space: u32) {
    let menu = singleton(e);
    if menu == 0 {
        return;
    }
    let root = menu.wrapping_add(ROOT_MAP_SPACE);
    let old = e.mem.u32(root);
    let mut current = space;
    while current != 0 {
        let parent = e.mem.u32(current.wrapping_add(WORLD_PARENT));
        if parent == 0 {
            break;
        }
        current = parent;
    }
    e.mem.set_u32(root, current);
    e.mem.set_u8(
        menu.wrapping_add(ROOT_SPACE_CHANGED),
        u8::from(old != current),
    );
}

// Typed from 007a1b10
/// `BSSimpleArray<BSStringT<char>,1024>`'s removal of element `index`: with `shrink` and the array
/// wanting to shrink, a smaller buffer allocated (its slot +4), the elements before moved, the
/// element destroyed, the `size - 1` elements after it moved (the exe's count), the old buffer freed
/// and the new one and its size kept; else the element destroyed and the ones after moved down.
/// Then one fewer.
pub fn fn_007a1b10(e: &mut Engine, this: Ptr, index: u32, shrink: u8) {
    let at = this.addr();
    let offset = index.wrapping_mul(8);
    let shrinks = shrink != 0
        && typed_call!(
            pathing::FAMILY,
            0x006f_3170,
            pathing::search::smoother_array_should_shrink(e, Ptr::new(at))
        );
    if shrinks {
        let reserve = e.call(ARRAY_SHRUNK_SIZE, &args![at]).u32();
        let buffer = e.vcall(at, ALLOCATE_SLOT, &args![reserve]).u32();
        let old = e.mem.u32(at.wrapping_add(4));
        e.call(ARRAY_MOVE, &args![at, buffer, old, index]);
        let old = e.mem.u32(at.wrapping_add(4));
        typed_call!(
            super::FAMILY,
            0x007a_63b0,
            map_menu::fn_007a63b0(e, this, old.wrapping_add(offset), 1)
        );
        let count = e.mem.u32(at.wrapping_add(8)).wrapping_sub(1);
        let old = e.mem.u32(at.wrapping_add(4));
        e.call(
            ARRAY_MOVE,
            &args![
                at,
                buffer.wrapping_add(offset),
                old.wrapping_add(offset).wrapping_add(8),
                count
            ],
        );
        typed_call!(
            pathing::FAMILY,
            0x006a_8500,
            pathing::search::free_data(e, Ptr::new(at))
        );
        e.mem.set_u32(at.wrapping_add(4), buffer);
        e.mem.set_u32(at.wrapping_add(0xc), reserve);
    } else {
        let old = e.mem.u32(at.wrapping_add(4));
        typed_call!(
            super::FAMILY,
            0x007a_63b0,
            map_menu::fn_007a63b0(e, this, old.wrapping_add(offset), 1)
        );
        let count = e
            .mem
            .u32(at.wrapping_add(8))
            .wrapping_sub(index)
            .wrapping_sub(1);
        let old = e.mem.u32(at.wrapping_add(4)).wrapping_add(offset);
        e.call(ARRAY_MOVE, &args![at, old, old.wrapping_add(8), count]);
    }
    let size = e.mem.u32(at.wrapping_add(8)).wrapping_sub(1);
    e.mem.set_u32(at.wrapping_add(8), size);
}

// ---- The data pages ---------------------------------------------------------------------------

// Typed from 0079af30
/// `MapMenu::UpdateCurrentPage` (Xbox PDB): the quest, objective, note and challenge lists disabled
/// (the note and challenge containers' trait `0xfa3` cleared), tiles 11, 12, 18 and 21 reset, then
/// by the page: 0x20 the local map (tile 2 the map tile; updated unless `007a1750`), 0x21 the world
/// map (tile 4, updated), 0x22 the quests, 0x23 the notes (tile 18 shown; outside challenge mode the
/// list's state saved, the notes updated, the selected note loaded and its tile selected, the state
/// restored with the focus), then the challenges and `ToggleNotesChallengeMode(0)`; 0x24 the radio
/// (tile 12 hidden, the highlighted index kept, -1.0 without one, the list updated, the tile at that
/// index focused and the list's slot +0x10 called).
pub fn map_menu_update_current_page(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let list = |offset: u32| Ptr::new(at.wrapping_add(offset));
    typed_call!(
        super::FAMILY,
        0x007a_2350,
        map_menu::list_box_tes_quest_set_enabled(e, list(QUESTS), 0)
    );
    typed_call!(
        super::FAMILY,
        0x007a_4c50,
        map_menu::list_box_bgs_quest_objective_set_enabled(e, list(OBJECTIVES), 0)
    );
    typed_call!(
        super::FAMILY,
        0x007a_30e0,
        map_menu::list_box_bgs_note_set_enabled(e, list(NOTES), 0)
    );
    typed_call!(
        super::FAMILY,
        0x007a_5a30,
        map_menu::fn_007a5a30(e, list(NOTES), 0)
    );
    typed_call!(
        super::FAMILY,
        0x007a_59a0,
        map_menu::list_box_tes_challenge_set_enabled(e, list(CHALLENGES), 0)
    );
    typed_call!(
        super::FAMILY,
        0x007a_5a30,
        map_menu::fn_007a5a30(e, list(CHALLENGES), 0)
    );
    for (index, id, value) in [
        (11u32, 0xfaf, 1),
        (12, 0xfa3, 1),
        (18, 0xfa3, 0),
        (21, 0xfa3, 0),
    ] {
        let tile = map_tile(e, singleton(e), index);
        tile_set_int(e, tile, id, value);
    }
    let page = e.mem.u32(singleton(e).wrapping_add(PAGE));
    match page.wrapping_sub(0x20) {
        0 => {
            let menu = singleton(e);
            e.mem
                .set_u32(menu.wrapping_add(MAP_TILE), map_tile(e, menu, 2));
            if !typed_call!(super::FAMILY, 0x007a_1750, fn_007a1750(e)) {
                let menu = singleton(e);
                e.call(UPDATE_LOCAL_MAP, &args![menu]);
            }
        }
        1 => {
            let menu = singleton(e);
            e.mem
                .set_u32(menu.wrapping_add(MAP_TILE), map_tile(e, menu, 4));
            e.call(UPDATE_WORLD_MAP, &args![menu]);
        }
        2 => {
            let menu = singleton(e);
            typed_call!(
                super::FAMILY,
                0x0079_b8e0,
                map_menu_update_quests(e, Ptr::new(menu))
            );
        }
        3 => {
            let tile = map_tile(e, singleton(e), 18);
            tile_set_int(e, tile, 0xfa3, 1);
            if e.mem.u8(CHALLENGE_MODE) == 0 {
                let notes = Ptr::new(singleton(e).wrapping_add(NOTES));
                typed_call!(
                    super::FAMILY,
                    0x0073_12e0,
                    map_menu::list_box_bgs_note_save_list_state(e, notes)
                );
                let menu = singleton(e);
                typed_call!(
                    super::FAMILY,
                    0x0079_b240,
                    map_menu_update_notes(e, Ptr::new(menu))
                );
                if e.mem.u32(at.wrapping_add(SELECTED_NOTE)) != 0 && e.mem.u8(CHALLENGE_MODE) == 0 {
                    let tile = e
                        .call(
                            LIST_BOX_GET_TILE_FOR,
                            &args![at.wrapping_add(NOTES), at.wrapping_add(SELECTED_NOTE)],
                        )
                        .u32();
                    let note = e.mem.u32(at.wrapping_add(SELECTED_NOTE));
                    e.call(LOAD_NOTE, &args![at, note]);
                    if tile != 0 {
                        let selected = static_id(e, TRAIT_SELECTED);
                        tile_set_int(e, tile, selected, 1);
                    }
                }
                let notes = Ptr::new(singleton(e).wrapping_add(NOTES));
                typed_call!(
                    super::FAMILY,
                    0x007a_3580,
                    map_menu::fn_007a3580(e, notes, 1)
                );
            }
            let menu = singleton(e);
            typed_call!(super::FAMILY, 0x0079_b600, fn_0079b600(e, Ptr::new(menu)));
            typed_call!(
                super::FAMILY,
                0x0079_b460,
                map_menu_toggle_notes_challenge_mode(e, this, 0)
            );
        }
        4 => {
            let tile = map_tile(e, singleton(e), 12);
            tile_set_int(e, tile, 0xfa3, 0);
            let radio = at.wrapping_add(RADIO);
            let value = if e.vcall(radio, HIGHLIGHTED_SLOT, &args![]).u32() != 0 {
                let tile = e.vcall(radio, HIGHLIGHTED_SLOT, &args![]).u32();
                tile_float(e, tile, 0xfac)
            } else {
                e.mem.f32(MINUS_ONE_FLOAT)
            };
            let index = ftol2(e, f64::from(value));
            let menu = singleton(e);
            typed_call!(
                super::FAMILY,
                0x0079_bea0,
                map_menu_update_radio_list(e, Ptr::new(menu))
            );
            let tile = e.vcall(radio, TILE_AT_SLOT, &args![index, 0u32]).u32();
            if tile != 0 {
                let manager = in_place::manager(e);
                e.call(
                    SET_CURRENT_FOCUS_TARGET,
                    &args![manager, tile, 0xfc3u32, 1u32],
                );
            }
            e.vcall(radio, LIST_SLOT_10, &args![]);
        }
        _ => {}
    }
}

// Typed from 0079b240
/// `MapMenu::UpdateNotes` (Xbox PDB): with an open map menu, the note list emptied and each of the
/// player's notes inserted (its name) with trait `0xfaa` 0x18 and its text alpha (dim when read);
/// the list enabled and filtered (`0079b390`); tile 12's `"_ShowingAllNotes"` and trait `0xfaf`
/// (whether the player has an active quest).
pub fn map_menu_update_notes(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    if singleton(e) == 0 {
        return;
    }
    let notes = at.wrapping_add(NOTES);
    e.vcall(notes, REMOVE_ALL_SLOT, &args![]);
    let mut index = 0u32;
    loop {
        let player = e.mem.u32(PLAYER);
        // The count is signed in the exe's compare.
        if index as i32 >= item_count(e, player.wrapping_add(PLAYER_NOTES)) as i32 {
            break;
        }
        let note = e.call(PLAYER_NOTE_AT, &args![player, index]).u32();
        let name = name_characters(e, note.wrapping_add(NOTE_NAME));
        let tile = typed_call!(
            super::FAMILY,
            0x007a_2930,
            map_menu::fn_007a2930(e, Ptr::new(notes), note, name, 0, 0)
        );
        tile_set_int(e, tile, 0xfaa, 0x18);
        let read = typed_call!(super::FAMILY, 0x0079_8420, fn_00798420(e, Ptr::new(note))) != 0;
        set_item_alpha(e, tile, read);
        index = index.wrapping_add(1);
    }
    typed_call!(
        super::FAMILY,
        0x007a_30e0,
        map_menu::list_box_bgs_note_set_enabled(e, Ptr::new(notes), 1)
    );
    typed_call!(
        super::FAMILY,
        0x007a_2e30,
        map_menu::list_box_bgs_note_filter(e, Ptr::new(notes), NOTES_FILTER)
    );
    let showing = i32::from(e.mem.u8(at.wrapping_add(SHOWING_ALL_NOTES)));
    let id = trait_by_name(e, SHOWING_ALL_NOTES_TEXT);
    tile_set_int(e, map_tile(e, at, 12), id, showing);
    let player = e.mem.u32(PLAYER);
    let quest = e.mem.u32(player.wrapping_add(ACTIVE_QUEST));
    tile_set_int(e, map_tile(e, at, 12), 0xfaf, i32::from(quest != 0));
}

// Typed from 0079b390
/// The notes filter (cdecl): with a map menu showing only some notes, 0 when the note's quest list
/// holds the player's active quest, else 1; 0 when it shows all; 1 without a map menu.
pub fn fn_0079b390(e: &mut Engine, note: u32) -> u8 {
    let menu = singleton(e);
    if menu == 0 {
        return 1;
    }
    if e.mem.u8(menu.wrapping_add(SHOWING_ALL_NOTES)) != 0 {
        return 0;
    }
    let mut filtered = 1u8;
    let mut node = note.wrapping_add(NOTE_QUESTS);
    while node != 0 && e.mem.u32(node) != 0 {
        let player = e.mem.u32(PLAYER);
        if e.mem.u32(node) == e.mem.u32(player.wrapping_add(ACTIVE_QUEST)) {
            filtered = 0;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    filtered
}

// Typed from 0079b460
/// `MapMenu::ToggleNotesChallengeMode` (Xbox PDB): with `toggle` the challenge mode flipped; the
/// challenge list and its container enabled in challenge mode, the note list otherwise; tile 13
/// shown (its `"MM_DataText"` emptied when toggling); the heading (tile 18) and tile 21 (the shown
/// challenge type's name) or tile 12's trait `0xfaf`; with `toggle`, `UpdateCurrentPage`.
pub fn map_menu_toggle_notes_challenge_mode(e: &mut Engine, this: Ptr, toggle: u8) {
    let at = this.addr();
    if toggle != 0 {
        let mode = e.mem.u8(CHALLENGE_MODE);
        e.mem.set_u8(CHALLENGE_MODE, u8::from(mode == 0));
    }
    let challenges = Ptr::new(at.wrapping_add(CHALLENGES));
    let notes = Ptr::new(at.wrapping_add(NOTES));
    let mode = e.mem.u8(CHALLENGE_MODE);
    typed_call!(
        super::FAMILY,
        0x007a_59a0,
        map_menu::list_box_tes_challenge_set_enabled(e, challenges, mode)
    );
    let mode = e.mem.u8(CHALLENGE_MODE);
    typed_call!(
        super::FAMILY,
        0x007a_5a30,
        map_menu::fn_007a5a30(e, challenges, mode)
    );
    let shown = u8::from(e.mem.u8(CHALLENGE_MODE) == 0);
    typed_call!(
        super::FAMILY,
        0x007a_30e0,
        map_menu::list_box_bgs_note_set_enabled(e, notes, shown)
    );
    let shown = u8::from(e.mem.u8(CHALLENGE_MODE) == 0);
    typed_call!(
        super::FAMILY,
        0x007a_5a30,
        map_menu::fn_007a5a30(e, notes, shown)
    );
    tile_set_int(e, map_tile(e, at, 13), 0xfa3, 1);
    if toggle != 0 {
        let data_text = child_by_name(e, map_tile(e, at, 13), DATA_TEXT_NAME);
        set_text(e, data_text, 0xfc4, EMPTY_TEXT);
    }
    if e.mem.u8(CHALLENGE_MODE) != 0 {
        let heading = in_place::setting_text(e, CHALLENGES_HEADING);
        set_text(e, map_tile(e, singleton(e), 18), 0xfc4, heading);
        tile_set_int(e, map_tile(e, singleton(e), 21), 0xfa3, 1);
        let kind = i32::from(e.mem.i8(CHALLENGE_TYPE)) as u32;
        let names = e
            .mem
            .u32(CHALLENGE_TYPE_NAMES.wrapping_add(kind.wrapping_mul(4)));
        let name = in_place::setting_text(e, names);
        set_text(e, map_tile(e, at, 21), 0xfc4, name);
    } else {
        let heading = in_place::setting_text(e, NOTES_HEADING);
        set_text(e, map_tile(e, singleton(e), 18), 0xfc4, heading);
        tile_set_int(e, map_tile(e, singleton(e), 12), 0xfaf, 1);
        tile_set_int(e, map_tile(e, singleton(e), 21), 0xfa3, 0);
    }
    if toggle != 0 {
        typed_call!(
            super::FAMILY,
            0x0079_af30,
            map_menu_update_current_page(e, this)
        );
    }
}

// Typed from 0079b600
/// The challenge list filled: emptied, then every challenge of `011c3f2c`'s list without its flag
/// bit 0 or with its state bit 0 inserted (its name) with trait `0xfaa` 0x18 and its text alpha (dim
/// with state bit 1); filtered (`0079b720`) and sorted (`0079b820`).
pub fn fn_0079b600(e: &mut Engine, this: Ptr) {
    let list = this.addr().wrapping_add(CHALLENGES);
    e.vcall(list, REMOVE_ALL_SLOT, &args![]);
    let owner = e.mem.u32(CHALLENGE_OWNER);
    let mut node = owner.wrapping_add(OWNER_CHALLENGES);
    while node != 0 {
        let challenge = e.mem.u32(node);
        if challenge != 0
            && (e.mem.u16(challenge.wrapping_add(CHALLENGE_FLAGS)) & 1 == 0
                || e.mem.u32(challenge.wrapping_add(CHALLENGE_STATE)) & 1 != 0)
        {
            let name = name_characters(e, challenge.wrapping_add(CHALLENGE_NAME));
            let tile = typed_call!(
                super::FAMILY,
                0x007a_5230,
                map_menu::fn_007a5230(e, Ptr::new(list), challenge, name, 0, 0)
            );
            tile_set_int(e, tile, 0xfaa, 0x18);
            let dim = e.mem.u32(challenge.wrapping_add(CHALLENGE_STATE)) & 2 != 0;
            set_item_alpha(e, tile, dim);
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    typed_call!(
        super::FAMILY,
        0x007a_5730,
        map_menu::list_box_tes_challenge_filter(e, Ptr::new(list), CHALLENGES_FILTER)
    );
    e.vcall(list, SORT_SLOT, &args![CHALLENGES_SORT]);
}

// Typed from 0079b720
/// The challenge filter (cdecl): 1 (filtered out) for a challenge with no value above 0, no flag
/// bit 2 and no state bit 2; otherwise 0 when its kind belongs to the shown type (0: kinds 0 to 2,
/// 1: 3 and 8, 2: 4 to 6 and 9 to 0xd), else 1. (The exe's read of the state bit 0 with an open map
/// menu leaves 1 either way.)
pub fn fn_0079b720(e: &mut Engine, challenge: u32) -> u8 {
    let kind = e.mem.u32(challenge.wrapping_add(CHALLENGE_KIND));
    if e.mem.i32(challenge.wrapping_add(CHALLENGE_VALUE)) <= 0
        && !typed_call!(
            super::FAMILY,
            0x0079_b800,
            fn_0079b800(e, Ptr::new(challenge))
        )
        && e.mem.u32(challenge.wrapping_add(CHALLENGE_STATE)) & 4 == 0
    {
        return 1;
    }
    match (e.mem.i8(CHALLENGE_TYPE), kind) {
        (0, 0..=2) | (1, 3 | 8) | (2, 4..=6 | 9..=0xd) => 0,
        _ => 1,
    }
}

// Typed from 0079b820
/// The challenge sort (cdecl, two list items): by the text alphas (a dim one after a full one),
/// else by their names (`00408b20`).
pub fn fn_0079b820(e: &mut Engine, first: u32, second: u32) -> i32 {
    let (a, b) = item_alphas(e, first, second);
    if let Some(order) = alpha_order(e, a, b) {
        return order;
    }
    let second_name = name_characters(
        e,
        e.mem
            .u32(second.wrapping_add(4))
            .wrapping_add(CHALLENGE_NAME),
    );
    let first_name = name_characters(
        e,
        e.mem
            .u32(first.wrapping_add(4))
            .wrapping_add(CHALLENGE_NAME),
    );
    e.call(COMPARE, &args![first_name, second_name]).i32()
}

// Typed from 0079b8e0
/// `MapMenu::UpdateQuests` (Xbox PDB): the quest and objective lists emptied; each shown objective
/// of the player's (until the end node) inserted with its text, its first child's trait `0xfba` the
/// objective width, `"_ShowEmptyMarker"` 1, `"_enabled"` 0 and its text alpha (dim and selected for
/// a state above 1, else dim when its quest is done); its quest added once (its name, trait `0xfaa`
/// 0x17, dim when done, selected when it is the player's active quest). Both lists sorted
/// (`0079bc90`) and the quest list enabled; with an active quest it is clicked (id 0x17) and, when
/// the manager's state test answers, focused and the list's slot +0x10 called; without one the
/// objectives filtered (`0079bc50`).
pub fn map_menu_update_quests(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let quests = at.wrapping_add(QUESTS);
    let objectives = at.wrapping_add(OBJECTIVES);
    e.vcall(quests, REMOVE_ALL_SLOT, &args![]);
    e.vcall(objectives, REMOVE_ALL_SLOT, &args![]);
    let player = e.mem.u32(PLAYER);
    let mut node = typed_call!(super::FAMILY, 0x0079_bc30, fn_0079bc30(e, Ptr::new(player)));
    e.with_stack(4, |e, quest| {
        let slot = quest.addr();
        while node != 0 && !(e.mem.u32(node.wrapping_add(4)) == 0 && e.mem.u32(node) == 0) {
            let objective = e.mem.u32(node);
            e.mem
                .set_u32(slot, e.mem.u32(objective.wrapping_add(OBJECTIVE_QUEST)));
            if e.call(OBJECTIVE_SHOWN, &args![objective]).bool() {
                add_objective(e, node, slot, quests, objectives);
            }
            node = e.mem.u32(node.wrapping_add(4));
        }
    });
    e.vcall(objectives, SORT_SLOT, &args![QUESTS_SORT]);
    e.vcall(quests, SORT_SLOT, &args![QUESTS_SORT]);
    typed_call!(
        super::FAMILY,
        0x007a_2350,
        map_menu::list_box_tes_quest_set_enabled(e, Ptr::new(quests), 1)
    );
    let player = e.mem.u32(PLAYER);
    let active = e.mem.u32(player.wrapping_add(ACTIVE_QUEST));
    if active == 0 {
        typed_call!(
            super::FAMILY,
            0x007a_49e0,
            map_menu::list_box_bgs_quest_objective_filter(
                e,
                Ptr::new(objectives),
                OBJECTIVES_FILTER
            )
        );
        return;
    }
    let tile = e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), active);
        e.call(LIST_BOX_GET_TILE_FOR, &args![quests, slot]).u32()
    });
    e.vcall(at, CLICK_SLOT, &args![0x17u32, tile]);
    if e.call(MANAGER_STATE_TEST, &args![]).bool() {
        let manager = in_place::manager(e);
        e.call(
            SET_CURRENT_FOCUS_TARGET,
            &args![manager, tile, 0xfc3u32, 1u32],
        );
        e.vcall(quests, LIST_SLOT_10, &args![]);
    }
}

/// `UpdateQuests`' work for a shown objective (the item of `node`; its quest in the stack word
/// `slot`).
fn add_objective(e: &mut Engine, node: u32, slot: u32, quests: u32, objectives: u32) {
    let objective = e.mem.u32(node);
    let text = match e.mem.u32(objective.wrapping_add(OBJECTIVE_TEXT)) {
        0 => EMPTY_TEXT,
        text => text,
    };
    let tile = typed_call!(
        super::FAMILY,
        0x007a_44e0,
        map_menu::fn_007a44e0(e, Ptr::new(objectives), objective, text, 0, 0)
    );
    let width = x87_quiet(e.mem.f32(OBJECTIVE_WIDTH));
    let first = e.mem.u32(tile.wrapping_add(8));
    let child = e.mem.u32(first.wrapping_add(8));
    tile_set_float(e, child, 0xfba, width);
    let empty_marker = trait_by_name(e, SHOW_EMPTY_MARKER_TEXT);
    tile_set_int(e, tile, empty_marker, 1);
    let enabled = trait_by_name(e, ENABLED_TEXT);
    tile_set_int(e, tile, enabled, 0);
    let objective = e.mem.u32(node);
    if e.mem.i32(objective.wrapping_add(OBJECTIVE_STATE)) > 1 {
        set_item_alpha(e, tile, true);
        let selected = static_id(e, TRAIT_SELECTED);
        tile_set_int(e, tile, selected, 1);
    } else {
        let done = quest_done(e, e.mem.u32(slot));
        set_item_alpha(e, tile, done);
    }
    if e.call(LIST_BOX_GET_TILE_FOR, &args![quests, slot]).u32() != 0 {
        return;
    }
    let quest = e.mem.u32(slot);
    let name = name_characters(e, quest.wrapping_add(QUEST_NAME));
    let quest_tile = typed_call!(
        super::FAMILY,
        0x007a_1dd0,
        map_menu::fn_007a1dd0(e, Ptr::new(quests), quest, name, 0, 0)
    );
    tile_set_int(e, quest_tile, 0xfaa, 0x17);
    let quest = e.mem.u32(slot);
    let done = quest_done(e, quest);
    set_item_alpha(e, quest_tile, done);
    let player = e.mem.u32(PLAYER);
    if e.mem.u32(slot) == e.mem.u32(player.wrapping_add(ACTIVE_QUEST)) {
        let selected = static_id(e, TRAIT_SELECTED);
        tile_set_int(e, quest_tile, selected, 1);
    }
}

/// A quest's done flag (`0059e400`, in place): bit 1 of its byte at +0x3c.
fn quest_done(e: &Engine, quest: u32) -> bool {
    e.mem.u8(quest.wrapping_add(QUEST_FLAGS)) & 2 != 0
}

// Typed from 0079bc90
/// `UpdateQuests`' sort (cdecl, two list items): by the text alphas, else by their indices (trait
/// `0xfac`): -1 when the second's is above the first's, 1 when below, else 0.
pub fn fn_0079bc90(e: &mut Engine, first: u32, second: u32) -> i32 {
    let (a, b) = item_alphas(e, first, second);
    if let Some(order) = alpha_order(e, a, b) {
        return order;
    }
    let tile = e.mem.u32(first);
    let first_index = f64::from(tile_float(e, tile, 0xfac));
    let tile = e.mem.u32(second);
    let second_index = f64::from(tile_float(e, tile, 0xfac));
    match second_index.partial_cmp(&first_index) {
        Some(std::cmp::Ordering::Greater) => -1,
        Some(std::cmp::Ordering::Less) => 1,
        _ => 0,
    }
}

// Typed from 0079bd70
/// `UpdateRadioList`'s sort (cdecl, two list items): the exe tests only the first item's text alpha
/// against 255.0, asking each time: -1 when it is equal and then not, 1 when it is not and then is;
/// otherwise the stations' names compared (`00408b20`).
pub fn fn_0079bd70(e: &mut Engine, first: u32, second: u32) -> i32 {
    let full: f64 = e.global(FULL_ALPHA_DOUBLE);
    let full_alpha = |e: &mut Engine| {
        let id = static_id(e, TRAIT_TEXT_ALPHA);
        let tile = e.mem.u32(first);
        f64::from(tile_float(e, tile, id)) == full
    };
    if full_alpha(e) && !full_alpha(e) {
        return -1;
    }
    if !full_alpha(e) && full_alpha(e) {
        return 1;
    }
    let station = e.mem.u32(second.wrapping_add(4));
    let second_name = e.call(STATION_NAME, &args![station]).u32();
    let station = e.mem.u32(first.wrapping_add(4));
    let first_name = e.call(STATION_NAME, &args![station]).u32();
    e.call(COMPARE, &args![first_name, second_name]).i32()
}

// Typed from 0079bea0
/// `MapMenu::UpdateRadioList` (Xbox PDB): the stations in range of the player gathered into a
/// local list, the radio list emptied and every station of `011dd59c` inserted (its name, trait
/// `0xfaa` 0x19, its tile named, selected when it is the current station, dim when out of range);
/// filtered (`0079be30`) and sorted (`0079bd70`); the local list emptied and destroyed.
pub fn map_menu_update_radio_list(e: &mut Engine, this: Ptr) {
    let radio = this.addr().wrapping_add(RADIO);
    e.with_stack(8, |e, in_range| {
        let in_range = in_range.addr();
        empty_list(e, in_range);
        let player = e.mem.u32(PLAYER);
        e.call(GET_RADIO_STATIONS_IN_RANGE, &args![player, in_range, 0u32]);
        e.vcall(radio, REMOVE_ALL_SLOT, &args![]);
        let mut node = typed_call!(super::FAMILY, 0x0079_c080, fn_0079c080(e));
        while node != 0 && e.mem.u32(node) != 0 {
            let station = e.mem.u32(node);
            let name = e.call(STATION_NAME, &args![station]).u32();
            let station = e.mem.u32(node);
            let tile = typed_call!(
                super::FAMILY,
                0x007a_3820,
                map_menu::fn_007a3820(e, Ptr::new(radio), station, name, 0, 0)
            );
            tile_set_int(e, tile, 0xfaa, 0x19);
            let station = e.mem.u32(node);
            let name = e.call(STATION_NAME, &args![station]).u32();
            typed_call!(
                super::FAMILY,
                0x0070_7e10,
                interface_cpp_p2::fn_00707e10(e, tile, name)
            );
            // The exe asks for the current station again before reading it.
            let selected = typed_call!(super::FAMILY, 0x0079_c070, fn_0079c070(e)) != 0 && {
                let current = typed_call!(super::FAMILY, 0x0079_c070, fn_0079c070(e));
                e.mem.u32(node) == e.mem.u32(current)
            };
            let id = static_id(e, TRAIT_SELECTED);
            tile_set_int(e, tile, id, i32::from(selected));
            let reachable = list_holds(e, in_range, e.mem.u32(node));
            set_item_alpha(e, tile, !reachable);
            node = e.mem.u32(node.wrapping_add(4));
        }
        typed_call!(
            super::FAMILY,
            0x007a_3d20,
            map_menu::list_box_tes_object_refr_filter(e, Ptr::new(radio), RADIO_FILTER)
        );
        e.vcall(radio, SORT_SLOT, &args![RADIO_SORT]);
        e.call(LIST_REMOVE_ALL, &args![in_range]);
        e.call(LIST_DESTRUCT, &args![in_range]);
    });
}

// Typed from 0079c0c0
/// The radio waveform (tile 16), with tile 16's property: started when its start time is 0.0 (the
/// image path from the format setting and trait `0xfd2` into `0xfcc`, the clock as the start time,
/// trait `0xfa3` 1); every 10 ms the property's strip moved by the step (wrapping above 1.0); after
/// the frame time `"Forward"`, `"Backward"` or `"Left"` (a random number mod 3) as trait `0xfd2` and
/// the start time cleared.
pub fn fn_0079c0c0(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let property = tile_shader_property(e, map_tile(e, singleton(e), 16));
    if property == 0 {
        return;
    }
    let start = at.wrapping_add(WAVEFORM_START);
    let zero: f64 = e.global(ZERO_DOUBLE);
    if f64::from(x87_quiet(e.mem.f32(start))) == zero {
        e.with_stack(0x44, |e, frame| {
            let local = frame.addr();
            let text = local.wrapping_add(4);
            // The smart pointer's constructor from null (`00633c90`): it holds none.
            e.mem.set_u32(local, 0);
            let name = typed_call!(
                dispatch::FAMILY,
                0x00a0_11f0,
                dispatch::tile::get_string(e, Ptr::new(map_tile(e, at, 16)), 0xfd2)
            )
            .addr();
            let format = in_place::setting_text(e, WAVEFORM_FORMAT);
            e.call(BUFFER_FORMAT, &args![text, 0x40u32, format, name]);
            set_text(e, map_tile(e, singleton(e), 16), 0xfcc, text);
            let now = e.call(CLOCK, &args![]).u32();
            e.mem.set_f32(start, narrow(f64::from(now)));
            tile_set_int(e, map_tile(e, at, 16), 0xfa3, 1);
            typed_call!(
                dispatch::FAMILY,
                0x0045_cec0,
                dispatch::refs::release_held(e, Ptr::new(local))
            );
        });
    }
    let now = e.call(CLOCK, &args![]).u32();
    if now.wrapping_sub(e.mem.u32(WAVEFORM_STEP_TIME)) >= 10 {
        let offset = x87_quiet(e.mem.f32(WAVEFORM_OFFSET));
        typed_call!(
            hud::FAMILY,
            0x0077_a450,
            hud::compass::set_strip(e, Ptr::new(property), offset, 0.0)
        );
        let step: f64 = e.global(WAVEFORM_STEP);
        let offset = narrow(f64::from(x87_quiet(e.mem.f32(WAVEFORM_OFFSET))) + step);
        e.mem.set_f32(WAVEFORM_OFFSET, offset);
        let one: f64 = e.global(ONE_DOUBLE);
        if f64::from(offset) > one {
            e.mem
                .set_f32(WAVEFORM_OFFSET, narrow(f64::from(offset) - one));
        }
        let now = e.call(CLOCK, &args![]).u32();
        e.mem.set_u32(WAVEFORM_STEP_TIME, now);
    }
    let now = e.call(CLOCK, &args![]).u32();
    let elapsed = narrow(f64::from(now) - f64::from(e.mem.f32(start)));
    let frame: f64 = e.global(WAVEFORM_FRAME);
    // On unless below the frame time or unordered.
    if matches!(
        f64::from(elapsed).partial_cmp(&frame),
        Some(std::cmp::Ordering::Less) | None
    ) {
        return;
    }
    let pick = e.call(RANDOM, &args![]).u32() % 3;
    let name = WAVEFORM_NAMES[pick as usize];
    e.with_stack(0x40, |e, text| {
        e.call(BUFFER_COPY, &args![text, 0x40u32, name]);
        set_text(e, map_tile(e, at, 16), 0xfd2, text.addr());
    });
    e.mem.set_f32(start, 0.0);
}

/// The functions above, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0072_d040, map_menu_attach_tile_by_id(Ptr, i32, u32)),
        entry!(0x0079_6740, map_menu_map_menu(Ptr) -> Ptr),
        entry!(0x0079_6a40, map_menu_get_class(Ptr) -> u32),
        entry!(0x0079_6a50, map_menu_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0079_6a80, fn_00796a80(Ptr)),
        entry!(0x0079_6b90, map_menu_create() -> u32),
        entry!(0x0079_6f40, fn_00796f40(Ptr, u8)),
        entry!(0x0079_6f90, fn_00796f90(Ptr) -> bool),
        entry!(0x0079_83c0, fn_007983c0(Ptr) -> f32),
        entry!(0x0079_8400, fn_00798400(Ptr) -> u32),
        entry!(0x0079_8420, fn_00798420(Ptr) -> u8),
        entry!(0x0079_8440, fn_00798440() -> u8),
        entry!(0x0079_8450, fn_00798450(Ptr, u32) -> Ptr),
        entry!(0x0079_8480, map_menu_do_down_click(Ptr, i32, u32)),
        entry!(0x0079_a610, map_menu_do_controller_switch(Ptr, u8)),
        entry!(0x0079_aba0, map_menu_update_menu()),
        entry!(0x0079_af30, map_menu_update_current_page(Ptr)),
        entry!(0x0079_b240, map_menu_update_notes(Ptr)),
        entry!(0x0079_b390, fn_0079b390(u32) -> u8),
        entry!(0x0079_b430, fn_0079b430(u32)),
        entry!(0x0079_b460, map_menu_toggle_notes_challenge_mode(Ptr, u8)),
        entry!(0x0079_b600, fn_0079b600(Ptr)),
        entry!(0x0079_b720, fn_0079b720(u32) -> u8),
        entry!(0x0079_b800, fn_0079b800(Ptr) -> bool),
        entry!(0x0079_b820, fn_0079b820(u32, u32) -> i32),
        entry!(0x0079_b8e0, map_menu_update_quests(Ptr)),
        entry!(0x0079_bc30, fn_0079bc30(Ptr) -> u32),
        entry!(0x0079_bc50, fn_0079bc50(u32) -> u8),
        entry!(0x0079_bc90, fn_0079bc90(u32, u32) -> i32),
        entry!(0x0079_bd70, fn_0079bd70(u32, u32) -> i32),
        entry!(0x0079_be30, fn_0079be30(u32) -> u8),
        entry!(0x0079_be80, fn_0079be80(u32)),
        entry!(0x0079_bea0, map_menu_update_radio_list(Ptr)),
        entry!(0x0079_c070, fn_0079c070() -> u32),
        entry!(0x0079_c080, fn_0079c080() -> u32),
        entry!(0x0079_c090, fn_0079c090(Ptr)),
        entry!(0x0079_c0c0, fn_0079c0c0(Ptr)),
        entry!(0x0079_c340, fn_0079c340(u8)),
        entry!(0x007a_1400, fn_007a1400(Ptr, u32)),
        entry!(0x007a_1430, fn_007a1430(Ptr, u8)),
        entry!(0x007a_1450, fn_007a1450(Ptr, u32)),
        entry!(0x007a_1480, fn_007a1480() -> u32),
        entry!(0x007a_1490, map_menu_clear_map_memory()),
        entry!(0x007a_1670, map_menu_clear_world_map_texture()),
        entry!(0x007a_1750, fn_007a1750() -> bool),
        entry!(0x007a_17a0, fn_007a17a0() -> f32),
        entry!(0x007a_17e0, fn_007a17e0() -> f32),
        entry!(0x007a_1820, fn_007a1820(f32)),
        entry!(0x007a_1850, fn_007a1850(f32)),
        entry!(0x007a_1880, map_menu_set_root_map_space(u32)),
        entry!(0x007a_1b10, fn_007a1b10(Ptr, u32, u8)),
    ]
}
