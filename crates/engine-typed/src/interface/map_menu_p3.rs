//! The map menu unit (the Xbox PDB's `fallout/interface/menus/mapmenu` source), its third part: the
//! idle (the note's sound paused and resumed, the local map's steps, the cursor over the maps, the
//! selected note read, the radio page), the player marker's space and callbacks, the fast travel
//! callbacks, the note's sounds stopped and cleared, the enter and leave of the data pages with the
//! objectives' target filter, the challenge's and note's loads, the marker under the cursor, the
//! note's playing sounds, the map and world coordinates, the wheel and the zoom, the world and
//! local maps' updates, the quest markers, the companions' markers updated, the player marker
//! created, set and deleted, the world map's tracking reference, the sound list's append, the
//! waypoint path and the local map's making step. The translation is engine-fallout-interface's
//! `mapmenu.rs`, which these functions replace at the same addresses. `DoClick` (JIP LN's sound
//! flag seam at `0x7974cc`), `DoGamepad`, `AddMapIcons` and `AddCompanionMarkers` (extender hooks
//! and patches) stay translated and are called by address.
//!
//! **Native.** The exe's order of work; `MapMenu::pMe` read again after each call where the exe
//! reads it again (a call may close the menu). The calls of functions no family types stay calls by
//! address (the sounds and the audio, the pathing locations' set-up, the map marker data, the world
//! spaces' map data and markers, the cells' seen values and textures, the menu's templates, the
//! values' actions and their calculation (overridable), the fonts, the text formats, the
//! controls' tests, the Pip-Boy's knob, the renderer's and the geometry's set-up). The tiles'
//! getters and setters, a value's actions cleared, a node's child and controller, an array's
//! construction and a cast are the dispatch family's; the pathing locations' constructor and
//! assignment, the points' length, sum and root, the teleport paths' arrays and the array's shrink
//! test the pathing family's; the larger float the drawing family's; a text's length and a form by
//! id the forms family's; the TES object's world space the multibound family's; the player's map
//! marker the HUD family's; a reference's seen data and a colour's constructor the extra data
//! family's; a node's update the scene family's; `_ftol2` and `_stricmp` the C runtime's; the
//! `Interface::` functions, the list boxes' and this file's own are this family's (all through
//! `typed_call!`). Done in place ([`IN_PLACE`] and the other files' lists): the interface manager,
//! its Pip-Boy and the query byte, the controls object, a tile's node and owner, the timer's time,
//! a setting's byte, value and float, a name's characters, the audio object, a sound handle's copy,
//! the two points' constructors, the address `+0x10`, a reference's cell (`+0x40`), the words at
//! `+0`, `+4`, `+8`, `+0xc`, `+0x10` and `+0x20`, a form's type and flag, the smaller float and
//! integer, an objective's and a quest's flags, a challenge's value, the font manager, the renderer
//! object, a word's own address, the list walks' step and the 8-byte list's constructor. A tile's
//! and a list box's virtual slots, a marker's position slot, the menus' click and gamepad slots and
//! the challenge's description slot stay virtual.
//!
//! **Floats.** The main thread's precision (the x87 control word of the thread that runs them):
//! each step goes through engine-core's x87 steps in the exe's order (a sum, difference, product or
//! quotient kept on the x87 stack keeps its wider exponent; a store to a float rounds it once); the
//! exe's double constants are read where it reads them; a float the exe moves through the x87 stack
//! is quieted.

use super::interface_cpp::{self, in_place};
use super::interface_cpp_p2;
use super::map_menu;
use super::map_menu_p2;
use super::menu_manager_p2::{child_by_name, ftol2, trait_by_name, ADD_USER_TRAIT};
use super::stats_menu;
use crate::{
    crt, dispatch, draw_f, extra_data, forms, hud, multibound, pathing, scene,
};
use engine_core::prelude::*;
use engine_core::typed_call;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub,
};

/// What this file does in place besides the other files' lists: plain reads and stores the exe
/// calls (the differential tests leave these out of the translation's calls).
pub const IN_PLACE: &[u32] = &[
    0x0046_0140, // the address +0x10 (the challenges' owner's map markers, a target's list)
    0x008d_6f30, // a reference's cell (+0x40)
    0x0045_2dc0, // `NiPoint2`'s constructor: two stores
    0x0041_6870, // `NiPoint3`'s constructor: three stores
    0x0071_7e50, // the address +4 (a quest target's condition)
];

/// `MapMenu::pMe`.
const SINGLETON: u32 = 0x011d_a368;
/// The members: `pTiles` (+0x28), the page (+0x80), the map tile (+0x84), the selected note
/// (+0x90), the note's enter time (+0x94), the sound list (+0x98), the subtitle array (+0xa8), the
/// current sound node (+0xb8), the sounds' loading byte (+0xbc), the sounds' length (+0xc0), the
/// play start time (+0xc4), the old radio volume (+0xc8), the displayed quest and its target
/// (+0xcc, +0xd0), the highlighted marker (+0xd4), the marker list (+0xd8), the door marker list
/// (+0xe0), the map rectangle (+0xe8 to +0xf4), the player marker's space and point (+0xf8,
/// +0xfc), the map space and the root space (+0x108, +0x10c), the root changed (+0x110), the map's
/// player reference (+0x114), the fast travel marker (+0x118), the north rotation cell (+0x11c),
/// the local map's next grid (+0x128), the fog of war (+0x12c); the list boxes (quests +0x130,
/// notes +0x160, radio stations +0x190, objectives +0x1c0, challenges +0x1f0).
const TILES: u32 = 0x28;
const PAGE: u32 = 0x80;
const MAP_TILE: u32 = 0x84;
const SELECTED_NOTE: u32 = 0x90;
const NOTE_ENTER_TIME: u32 = 0x94;
const SOUNDS: u32 = 0x98;
const SUBTITLES: u32 = 0xa8;
const CURRENT_SOUND: u32 = 0xb8;
const SOUNDS_LOADING: u32 = 0xbc;
const SOUNDS_LENGTH: u32 = 0xc0;
const PLAY_START: u32 = 0xc4;
const OLD_RADIO_VOLUME: u32 = 0xc8;
const DISPLAYED_QUEST: u32 = 0xcc;
const DISPLAYED_TARGET: u32 = 0xd0;
const HIGHLIGHTED_MARKER: u32 = 0xd4;
const MARKERS: u32 = 0xd8;
const DOOR_MARKERS: u32 = 0xe0;
const MAP_FROM_X: u32 = 0xe8;
const MAP_FROM_Y: u32 = 0xec;
const MAP_TO_X: u32 = 0xf0;
const MAP_TO_Y: u32 = 0xf4;
const MARKER_SPACE: u32 = 0xf8;
const MARKER_POINT: u32 = 0xfc;
const MAP_SPACE: u32 = 0x108;
const ROOT_SPACE: u32 = 0x10c;
const ROOT_CHANGED: u32 = 0x110;
const MAP_PLAYER_REF: u32 = 0x114;
const FAST_TRAVEL_MARKER: u32 = 0x118;
const NORTH_CELL: u32 = 0x11c;
const NEXT_LOCAL_GRID: u32 = 0x128;
const FOG_OF_WAR: u32 = 0x12c;
const QUESTS: u32 = 0x130;
const NOTES: u32 = 0x160;
const RADIO: u32 = 0x190;
const OBJECTIVES: u32 = 0x1c0;
const CHALLENGES: u32 = 0x1f0;

/// The pages: the local map (0x20) and the world map (0x21).
const LOCAL_PAGE: u32 = 0x20;
const WORLD_PAGE: u32 = 0x21;

/// The static trait ids (`MapMenu::MapMenu`'s): selected, text alpha, x, y, magnification.
const TRAIT_SELECTED: u32 = 0x011d_a348;
const TRAIT_TEXT_ALPHA: u32 = 0x011d_a344;
const TRAIT_X: u32 = 0x011d_a350;
const TRAIT_Y: u32 = 0x011d_a34c;
const TRAIT_MAGNIFICATION: u32 = 0x011d_a35c;

/// The statics: the challenge mode byte, the note sound's paused byte and pause time, the faded
/// byte, the knob's last index, the text scroll's time, the next sound's wait, the running marker
/// index.
const CHALLENGE_MODE: u32 = 0x011d_a36d;
const SOUND_PAUSED: u32 = 0x011d_a36f;
const PAUSE_TIME: u32 = 0x011d_a40c;
const FADED: u32 = 0x011d_a408;
const KNOB_INDEX: u32 = 0x011d_a400;
const SOUND_WAIT: u32 = 0x011d_a410;
const MARKER_INDEX: u32 = 0x011d_a370;
const TEXTURE_CLEARED: u32 = 0x011d_a36c;

/// The player, the timer (its time at +0x14, `00825c00` in place), the challenges' owner (its map
/// markers at +0x10), the start point of the player marker, the object `00877720` is asked on (not
/// read), the grid setting (`0043d4d0`'s value), the object `005f36f0` asks.
const PLAYER: u32 = 0x011d_ea3c;
const TIMER: u32 = 0x011f_6394;
const TIMER_TIME: u32 = 0x14;
const OWNER: u32 = 0x011c_3f2c;
const MARKER_START: u32 = 0x011f_426c;
const GRID_SETTING: u32 = 0x011c_63cc;
const INTERIOR_TEST_OBJECT: u32 = 0x011d_ea10;

/// The settings: the local map's step request (a byte), the note read delay, the knob's step and
/// time, the quest markers' switch (a byte), the zoom limits and the traits' ranges.
const LOCAL_STEP_SETTING: u32 = 0x011d_a3b8;
const NOTE_DELAY_SETTING: u32 = 0x011d_47bc;
const KNOB_STEP_SETTING: u32 = 0x011d_b2f0;
const KNOB_TIME_SETTING: u32 = 0x011d_b358;
const QUEST_MARKERS_SETTING: u32 = 0x011e_07f0;
const LOCAL_ZOOM_LIMITS: (u32, u32) = (0x011d_4c60, 0x011d_2f64);
const LOCAL_ZOOM_TRAITS: [(i32, u32, u32); 3] = [
    (0x1004, 0x011d_4564, 0x011d_42b8),
    (0x1005, 0x011d_29d0, 0x011d_26c4),
    (0x1006, 0x011d_32e8, 0x011d_38f4),
];
const WORLD_ZOOM_LIMITS: (u32, u32) = (0x011d_22f8, 0x011d_3d3c);
const WORLD_ZOOM_TRAITS: [(i32, u32, u32); 3] = [
    (0x1004, 0x011d_3ce8, 0x011d_3bbc),
    (0x1005, 0x011d_342c, 0x011d_3c58),
    (0x1006, 0x011d_4870, 0x011d_3df0),
];

/// The constants: the doubles 0.0, 1.0, 2.0, 1000.0, the map's width and height, 128.0 (the local
/// map's label offset), the marker kind 26.0, the fog divisor, the cell size 4096.0, the knob's
/// zoom divisor, the world map's scale and offset and their inverses, the world map's label
/// offset; the floats 255.0, 128.0, 2.0 (a depth), the largest float, the text width, the label's
/// width and height, the marker's depth, the zoom by the wheel; the empty text.
const ZERO_DOUBLE: u32 = 0x0101_2060;
const ONE_DOUBLE: u32 = 0x0101_2070;
const TWO_DOUBLE: u32 = 0x0101_1590;
const THOUSAND_DOUBLE: u32 = 0x0101_7b70;
const MAP_WIDTH_DOUBLE: u32 = 0x0107_4f68;
const MAP_HEIGHT_DOUBLE: u32 = 0x0103_01a8;
const HALF_ALPHA_DOUBLE: u32 = 0x0102_e430;
const MARKER_KIND_DOUBLE: u32 = 0x0107_4f60;
const FOG_DIVISOR_DOUBLE: u32 = 0x0101_db80;
const CELL_SIZE_DOUBLE: u32 = 0x0101_7a10;
const ZOOM_KNOB_DIVISOR_DOUBLE: u32 = 0x0107_5028;
const WORLD_SCALE_DOUBLE: u32 = 0x0107_5010;
const WORLD_OFFSET_DOUBLE: u32 = 0x0107_5008;
const WORLD_INVERSE_SCALE_DOUBLE: u32 = 0x0107_5020;
const WORLD_INVERSE_OFFSET_DOUBLE: u32 = 0x0107_5018;
const WORLD_LABEL_OFFSET_DOUBLE: u32 = 0x0101_ecb8;
const FULL_ALPHA_FLOAT: u32 = 0x0102_3cd8;
const HALF_ALPHA_FLOAT: u32 = 0x0101_e704;
const TWO_FLOAT: u32 = 0x0101_62c0;
const LARGEST_FLOAT: u32 = 0x0101_6970;
const TEXT_WIDTH_FLOAT: u32 = 0x0103_11a8;
const LABEL_WIDTH_FLOAT: u32 = 0x0102_226c;
const LABEL_HEIGHT_FLOAT: u32 = 0x0103_4294;
const MARKER_DEPTH_FLOAT: u32 = 0x0101_712c;
const WHEEL_ZOOM_FLOAT: u32 = 0x0105_6528;
const EMPTY_TEXT: u32 = 0x0101_1584;

/// The texts: `"test"`, `"_ItemType"`, `"MM_DataText"`, `"MM_TextScrollbar"`, `"_ScrollDelta"`,
/// the challenge's format, `"MM_DataImage"`, the audio note's time line, `"_ItemStringInfo"`,
/// `"_AudioFileCompletion"`, `"_number_of_items"`, `"_current_value"`, `"_SetInCode"`, `"title"`,
/// `"_LocationName"`, `"Companion"`, `" Marker"`, the play time formats, `"_MarkerIndex"`, the
/// templates and icon paths, `"%s\n(%s)"`, `"%s Marker"`.
const TEST_TEXT: u32 = 0x0101_7bfc;
const ITEM_TYPE_TEXT: u32 = 0x0107_39a0;
const DATA_TEXT_NAME: u32 = 0x0107_4edc;
const TEXT_SCROLLBAR_NAME: u32 = 0x0107_4ec8;
const SCROLL_DELTA_TEXT: u32 = 0x0107_4eb8;
const CHALLENGE_TEXT_FORMAT: u32 = 0x0107_4ea8;
const DATA_IMAGE_NAME: u32 = 0x0107_4f1c;
const AUDIO_TIME_FORMAT: u32 = 0x0107_4f10;
const ITEM_STRING_INFO_TEXT: u32 = 0x0107_4f00;
const AUDIO_COMPLETION_TEXT: u32 = 0x0107_4ee8;
const NUMBER_OF_ITEMS_TEXT: u32 = 0x0106_f8f8;
const CURRENT_VALUE_TEXT: u32 = 0x0107_02d8;
const SET_IN_CODE_TEXT: u32 = 0x0107_0250;
const TITLE_TEXT: u32 = 0x0106_ed20;
const LOCATION_NAME_TEXT: u32 = 0x0107_4e18;
const COMPANION_TEXT: u32 = 0x0107_4e28;
const MARKER_TEXT: u32 = 0x0107_4f54;
const PLAY_TIME_FORMAT: u32 = 0x0107_4f84;
const PLAY_DONE_FORMAT: u32 = 0x0107_4f78;
const MARKER_INDEX_TEXT: u32 = 0x0107_4e3c;
const MAP_MARKER_TEMPLATE: u32 = 0x0107_506c;
const MARKER_REPUTATION_FORMAT: u32 = 0x0107_5064;
const UNDISCOVERED_ICON_PATH: u32 = 0x0107_5030;
const DOOR_ICON_PATH: u32 = 0x0107_5334;
const WORLD_QUEST_MARKER_TEMPLATE: u32 = 0x0107_53a8;
const LOCAL_QUEST_MARKER_TEMPLATE: u32 = 0x0107_538c;
const QUEST_MARKER_ICON_PATH: u32 = 0x0107_5364;
const NAME_MARKER_FORMAT: u32 = 0x0107_53c4;
const PLAYER_MARKER_ICON_PATH: u32 = 0x0107_53f0;
const WAYPOINT_TEMPLATE: u32 = 0x0107_5454;
const PATH_LINE_TEMPLATE: u32 = 0x0107_5440;
const INTERIOR_PATH_TEXTURE: u32 = 0x0107_5410;
/// The sounds by name: the holotape's stop and the highlight.
const HOLOTAPE_STOP_SOUND: u32 = 0x0107_4e90;
const HIGHLIGHT_SOUND: u32 = 0x0107_4f40;
/// The audio note's label (a setting's text) and the marker icons by kind.
const AUDIO_LABEL: u32 = 0x011d_3960;
const MARKER_ICONS: u32 = 0x011a_0404;

/// The callbacks the code hands on: the fast travel confirmation (`00798a00`), the objectives'
/// target filter (`00799100`) and quest filter (`0079bc50`).
const FAST_TRAVEL_CONFIRMED: u32 = 0x0079_8a00;
const OBJECTIVE_TARGET_FILTER: u32 = 0x0079_9100;
const OBJECTIVE_QUEST_FILTER: u32 = 0x0079_bc50;

/// The virtual slots: a menu's gamepad (+0xc) and click (+0x10) and leave (+0x14) slots, a list
/// box's highlighted tile (+4), a reference's position (+0x1f4), a tile's deleting destructor (+0),
/// the challenge description's text (+0x10 of its +0x24), a note text object's (+0x10).
const MENU_GAMEPAD_SLOT: u32 = 0xc;
const MENU_CLICK_SLOT: u32 = 0x10;
const MENU_LEAVE_SLOT: u32 = 0x14;
const HIGHLIGHTED_SLOT: u32 = 4;
const POSITION_SLOT: u32 = 0x1f4;
const DELETE_SLOT: u32 = 0;
const TEXT_SLOT: u32 = 0x10;

/// The by-address callees.
const SOUND_PLAY: u32 = 0x00ad_8830;
const SOUND_PAUSE: u32 = 0x00ad_88b0;
const SOUND_STOP: u32 = 0x00ad_88f0;
const SOUND_IS_PLAYING: u32 = 0x00ad_8930;
const SOUND_IS_VALID: u32 = 0x00ad_8ce0;
const SOUND_GET_DURATION: u32 = 0x00ad_8b30;
const AUDIO_SOUND_BY_NAME: u32 = 0x00ad_7550;
const AUDIO_EXIT_DIALOGUE: u32 = 0x00ad_8650;
const AUDIO_SET_RADIO_VOLUME: u32 = 0x00ad_7c00;
const AUDIO: u32 = 0x011f_6d98;
const SOUND_LIST_CLEAR: u32 = 0x0076_b7a0;
const AFTER_NOTE_SOUND: u32 = 0x0076_24c0;
const SOUND_HANDLE_TEST: u32 = 0x0076_b780;
const SOUND_NODE_EMPTY: u32 = 0x0076_a3b0;
const SOUND_NODE_CONSTRUCT: u32 = 0x0076_b900;
const PIPBOY_READY: u32 = 0x007f_aef0;
const PIPBOY_MOVE_KNOB: u32 = 0x007f_8610;
const MANAGER_STATE_TEST: u32 = 0x004b_71d0;
const NOTE_SET_READ: u32 = 0x005e_9300;
const NOTE_TYPE: u32 = 0x005e_8d40;
const NOTE_TEXT_OBJECT: u32 = 0x005e_91d0;
const NOTE_IMAGE: u32 = 0x005e_9160;
const LIST_BOX_GET_TILE_FOR: u32 = 0x007a_22d0;
const LIST_BOX_FILTER_RANGE: u32 = 0x0073_14c0;
const CURSOR_X: u32 = 0x0076_97f0;
const CONTROLS_TEST_A: u32 = 0x00a2_3a50;
const CONTROLS_TEST_B: u32 = 0x00a2_4660;
const SET_MESSAGE_CALLBACK: u32 = 0x0070_f690;
const SET_PLAYER_MAP_MARKER: u32 = 0x0095_2e60;
const REMOVE_PLAYER_MAP_MARKER: u32 = 0x0095_2f90;
const REQUEST_POSITION_PLAYER: u32 = 0x0093_be30;
const POSITION_REQUEST_CONSTRUCT: u32 = 0x0045_6540;
const ALLOCATE: u32 = 0x0040_1000;
const BS_STRISTR: u32 = 0x004b_75a0;
const REFERENCE_SKIPPED: u32 = 0x0058_6210;
const ADJUST_MAP_MARKER_COORD: u32 = 0x0058_8e40;
const ADJUST_FOR_NORTH_ROTATION: u32 = 0x0055_5b10;
const PATHING_LOCATION_AT: u32 = 0x006d_cea0;
const PATHING_LOCATION_DESTRUCT: u32 = 0x004f_f7e0;
const PATHING_LOCATION_TEST: u32 = 0x006d_e250;
const FORM_TEXT: u32 = 0x0058_6440;
const TEXT_COMPARE_N: u32 = 0x0045_64f0;
const QUEST_TARGET_GET_REFERENCE: u32 = 0x0061_01b0;
const OBJECTIVE_TARGETS: u32 = 0x007d_6bb0;
const CONDITION_IS_TRUE: u32 = 0x0068_0c30;
const CHALLENGE_COUNT: u32 = 0x0062_86d0;
const FONT_CALCULATE_STRING_DIMENSIONS: u32 = 0x00a1_b020;
const ROUND_FLOAT: u32 = 0x004e_4430;
const BUFFER_FORMAT: u32 = 0x0040_6d00;
const TILE_NAME: u32 = 0x0070_0300;
const TEXT_COMPARE: u32 = 0x00ec_7750;
const POINT2_SCALE: u32 = 0x006a_d180;
const POINT2_SUBTRACT: u32 = 0x004e_8880;
const WHEEL_DELTA: u32 = 0x0070_ec70;
const CELL_INTERIOR_TEST: u32 = 0x005f_36f0;
const NODE_SET_SCALE: u32 = 0x0044_0490;
const NODE_SET_TRANSLATE: u32 = 0x004b_c1f0;
const NODE_GEOMETRY_DATA: u32 = 0x0054_95f0;
const GEOMETRY_SET_CONSISTENCY: u32 = 0x00a6_7050;
const MATRIX_SET: u32 = 0x0043_d410;
const WORLD_GET_WORLD_MAP_DATA: u32 = 0x0058_6400;
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const SOURCE_TYPE: u32 = 0x0118_3028;
const TARGET_TYPE: u32 = 0x0118_3fd0;
const TILE_SET_COUNT: u32 = 0x0071_5c60;
const TILE_GET_CHILD_BY_ID: u32 = 0x00a0_3eb0;
const WORLD_BUILD_MAP_MARKER_LIST: u32 = 0x0058_82a0;
const LIST_REMOVE_ALL: u32 = 0x0047_0470;
const NODE_DELETE: u32 = 0x0047_02f0;
const REFR_GET_WORLD_SPACE: u32 = 0x0057_5d70;
const WORLD_GET_PARENT_OFFSET: u32 = 0x70;
const REFR_GET_MAP_MARKER_DATA: u32 = 0x0056_9060;
const MARKER_GET_VISIBLE: u32 = 0x0043_8ed0;
const MARKER_GET_TRAVEL_LOC: u32 = 0x0043_8ef0;
const MARKER_NAME: u32 = 0x0043_8eb0;
const MARKER_ICON_KIND: u32 = 0x0043_8f30;
const REPUTATION_GET_TITLE: u32 = 0x0061_6710;
const MENU_RENDER_TEMPLATE: u32 = 0x00a1_ddb0;
const LIST_ADD_TAIL: u32 = 0x0090_5820;
const REFR_GET_INTERIOR: u32 = 0x0057_5d10;
const FLOAT_TO_INT: u32 = 0x0040_6d90;
const LOCAL_MAP_READY: u32 = 0x0076_b610;
const REFERENCE_SKIPPED_A: u32 = 0x0047_7ba0;
const REFERENCE_SKIPPED_B: u32 = 0x0057_b4b0;
const DOOR_SKIPPED: u32 = 0x0051_7fd0;
const CELL_GET_SEEN_VALUE: u32 = 0x0055_6870;
const REFERENCE_TELEPORT: u32 = 0x0056_8e50;
const TELEPORT_GET_CELL_MAP_NAME: u32 = 0x0043_a2e0;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
const CHILD_LIST_REMOVE: u32 = 0x0057_c730;
const TILE_RELEASE: u32 = 0x009f_f690;
const PLAYER_GET_CURRENT_TARGET_LIST: u32 = 0x0095_2ba0;
const TARGET_LOCAL_REFERENCE: u32 = 0x005c_bb70;
const VALUE_ADD_ACTION_TILE: u32 = 0x00a0_9130;
const VALUE_ADD_ACTION_FLOAT: u32 = 0x00a0_9080;
const VALUE_CALCULATE: u32 = 0x00a0_9410;
const PLAYER_COMPANIONS: u32 = 0x005e_b7e0;
const PLAYER_BUILD_PATH_TO_TARGET: u32 = 0x0095_2d60;
const TELEPORT_PATH_CONSTRUCT: u32 = 0x006f_48b0;
const TELEPORT_PATH_DESTRUCT: u32 = 0x006f_4930;
const STATION_NAME: u32 = 0x0055_d520;
const TEXT_COMPARE_NAMES: u32 = 0x0040_4dc0;
const SPRINTF: u32 = 0x00ec_623a;
const TILE_ADD_NEEDS_UPDATE: u32 = 0x00a0_7530;
const FLOAT_POWER: u32 = 0x0068_5d20;
const ATAN2: u32 = 0x004b_5550;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;

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

/// The word at `offset` of the menu at `menu`.
#[inline]
fn field(e: &Engine, menu: u32, offset: u32) -> u32 {
    e.mem.u32(menu.wrapping_add(offset))
}

/// The page of the menu at `menu`.
#[inline]
fn page(e: &Engine, menu: u32) -> u32 {
    field(e, menu, PAGE)
}

/// A static trait id (`MapMenu::MapMenu`'s statics).
#[inline]
fn static_id(e: &Engine, at: u32) -> i32 {
    e.mem.i32(at)
}

/// The timer's time (`00825c00` on the timer: its word at +0x14).
#[inline]
fn now(e: &Engine) -> u32 {
    e.mem.u32(TIMER + TIMER_TIME)
}

/// A setting's value address (`00403e20`, `00408d60` and `0043d4d0` on a constant setting object:
/// its `+4`).
#[inline]
fn setting_value(at: u32) -> u32 {
    at.wrapping_add(4)
}

/// The float a setting holds, loaded (a signalling NaN quieted).
#[inline]
fn setting_float(e: &Engine, setting: u32) -> u32 {
    x87_quiet_bits(e.mem.u32(setting_value(setting)))
}

/// A float constant as an argument (`FLD dword; FSTP dword`): its bits, a signalling NaN quieted.
#[inline]
fn float_constant(e: &Engine, at: u32) -> u32 {
    x87_quiet_bits(e.mem.u32(at))
}

/// A tile's owner (`0045cd60`: its word at +0x28).
#[inline]
fn owner(e: &Engine, tile: u32) -> u32 {
    e.mem.u32(tile.wrapping_add(0x28))
}

/// A list node's item (`00559450`, its first word) and next (`00726070`, its +4).
#[inline]
fn item(e: &Engine, node: u32) -> u32 {
    e.mem.u32(node)
}
#[inline]
fn next(e: &Engine, node: u32) -> u32 {
    e.mem.u32(node.wrapping_add(4))
}

/// A `BSSimpleArray`'s size (`0044ddc0`: its word at +8).
#[inline]
fn array_size(e: &Engine, array: u32) -> u32 {
    e.mem.u32(array.wrapping_add(8))
}

/// A form's type byte (`00401170`: `+4`).
#[inline]
fn form_type(e: &Engine, form: u32) -> u8 {
    e.mem.u8(form.wrapping_add(4))
}

/// A reference's base object (`007af430`: its word at +0x20).
#[inline]
fn base_object(e: &Engine, reference: u32) -> u32 {
    e.mem.u32(reference.wrapping_add(0x20))
}

/// A reference's cell (`008d6f30`: its word at +0x40).
#[inline]
fn cell_of(e: &Engine, reference: u32) -> u32 {
    e.mem.u32(reference.wrapping_add(0x40))
}

/// A world space's parent (`004fd380`: its word at +0x70).
#[inline]
fn world_parent(e: &Engine, space: u32) -> u32 {
    e.mem.u32(space.wrapping_add(WORLD_GET_PARENT_OFFSET))
}

/// The three words at `at`.
#[inline]
fn point_at(e: &Engine, at: u32) -> [u32; 3] {
    [
        e.mem.u32(at),
        e.mem.u32(at.wrapping_add(4)),
        e.mem.u32(at.wrapping_add(8)),
    ]
}

/// The three words stored at `at`.
#[inline]
fn set_point_at(e: &mut Engine, at: u32, point: [u32; 3]) {
    for (k, word) in point.into_iter().enumerate() {
        e.mem.set_u32(at.wrapping_add(4 * super::word(k)), word);
    }
}

/// A reference's position (its virtual slot +0x1f4), its three words.
fn position(e: &mut Engine, reference: u32) -> [u32; 3] {
    let at = e.vcall(reference, POSITION_SLOT, &args![]).u32();
    point_at(e, at)
}

/// `NiPoint2`'s constructor (`00452dc0`, in place): both words stored.
#[inline]
fn point2_new(e: &mut Engine, at: u32, x: u32, y: u32) {
    e.mem.set_u32(at, x);
    e.mem.set_u32(at.wrapping_add(4), y);
}

/// `InterfaceManager::IsTopMenuID` (this family's).
#[inline]
fn is_top(e: &mut Engine, id: i32) -> bool {
    typed_call!(
        super::FAMILY,
        0x0070_2450,
        interface_cpp::interface_is_top_menu_id(e, id)
    ) != 0
}

/// `Interface::GetPipboy` (this family's).
#[inline]
fn get_pipboy(e: &mut Engine) -> u32 {
    typed_call!(
        super::FAMILY,
        0x0070_5990,
        interface_cpp::interface_get_pipboy(e)
    )
}

/// The manager's state test (`004b71d0`, by address).
#[inline]
fn manager_busy(e: &mut Engine) -> bool {
    e.call(MANAGER_STATE_TEST, &args![]).bool()
}

/// A tile's float trait (`Tile::GetFloat`, the dispatch family's).
#[inline]
fn get_float(e: &mut Engine, tile: u32, id: i32) -> f32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_11b0,
        dispatch::tile::get_float(e, Ptr::new(tile), id)
    )
}

/// [`get_float`] as the double on the x87 stack.
#[inline]
fn get_float64(e: &mut Engine, tile: u32, id: i32) -> f64 {
    f64::from(get_float(e, tile, id))
}

/// [`get_float`] as the float an `FSTP dword` stores, its bits.
#[inline]
fn get_float_bits(e: &mut Engine, tile: u32, id: i32) -> u32 {
    x87_store32(get_float64(e, tile, id))
}

/// `Tile::SetFloat` with `flag` (the dispatch family's), the float's bits.
#[inline]
fn set_float_flag(e: &mut Engine, tile: u32, id: i32, bits: u32, flag: u8) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_12d0,
        dispatch::tile::set_float(e, Ptr::new(tile), id, f32::from_bits(bits), flag)
    );
}

/// [`set_float_flag`] with the flag 1.
#[inline]
fn set_float(e: &mut Engine, tile: u32, id: i32, bits: u32) {
    set_float_flag(e, tile, id, bits, 1);
}

/// `Tile::SetInt` (the dispatch family's).
#[inline]
fn set_int(e: &mut Engine, tile: u32, id: i32, value: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0070_0320,
        dispatch::tile::set_int(e, Ptr::new(tile), id, value as i32)
    );
}

/// `Tile::SetString` with the flag 1 (the dispatch family's).
#[inline]
fn set_string(e: &mut Engine, tile: u32, id: i32, text: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1350,
        dispatch::tile::set_string(e, Ptr::new(tile), id, Ptr::new(text), 1)
    );
}

/// `Tile::GetString` (the dispatch family's).
#[inline]
fn get_string(e: &mut Engine, tile: u32, id: i32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_11f0,
        dispatch::tile::get_string(e, Ptr::new(tile), id)
    )
    .addr()
}

/// `Tile::IsTrue` (the dispatch family's).
#[inline]
fn is_true(e: &mut Engine, tile: u32, id: i32) -> bool {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1230,
        dispatch::tile::is_true(e, Ptr::new(tile), id)
    )
}

/// `Tile::GetValue` (the dispatch family's).
#[inline]
fn get_value(e: &mut Engine, tile: u32, id: i32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_0f30,
        dispatch::tile::get_value(e, Ptr::new(tile), id)
    )
    .addr()
}

/// A value's actions cleared (`Tile::Value::ClearActions`, the dispatch family's).
#[inline]
fn clear_actions(e: &mut Engine, value: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_9200,
        dispatch::tile::clear_actions(e, Ptr::new(value))
    );
}

/// A value calculated with the flag 0 (`Tile::Value::Calculate`: overridable, by address).
#[inline]
fn calculate(e: &mut Engine, value: u32) {
    e.call(VALUE_CALCULATE, &args![value, 0u32]);
}

/// A tile's absolute x and y (`00a013d0`, `00a01440`, the dispatch family's).
#[inline]
fn absolute_x(e: &mut Engine, tile: u32) -> f64 {
    f64::from(typed_call!(
        dispatch::FAMILY,
        0x00a0_13d0,
        dispatch::tile::absolute_x(e, Ptr::new(tile))
    ))
}
#[inline]
fn absolute_y(e: &mut Engine, tile: u32) -> f64 {
    f64::from(typed_call!(
        dispatch::FAMILY,
        0x00a0_1440,
        dispatch::tile::absolute_y(e, Ptr::new(tile))
    ))
}

/// A user trait's id (`Tile::AddUserTrait(name, -1)`, by address).
#[inline]
fn user_trait(e: &mut Engine, name: u32) -> i32 {
    e.call(ADD_USER_TRAIT, &args![name, u32::MAX]).i32()
}

/// A sound by name played once: `BSAudio::GetSoundHandleByName(name, 0x121)` into a stack handle,
/// `Play(0)`; the handle's destructor (`00483710`) does nothing.
fn play_named_sound(e: &mut Engine, name: u32) {
    e.with_stack(0xc, |e, handle| {
        let audio = e.mem.u32(AUDIO);
        let sound = e
            .call(AUDIO_SOUND_BY_NAME, &args![audio, handle, name, 0x121u32])
            .u32();
        e.call(SOUND_PLAY, &args![sound, 0u32]);
    });
}

/// The Pip-Boy's knob moved by `step` over `time` (`FOPipboyManager::MoveKnob`, by address).
fn move_knob(e: &mut Engine, step: u32, time: u32) {
    let manager = in_place::manager(e);
    let pipboy = in_place::pipboy(e, manager);
    e.call(PIPBOY_MOVE_KNOB, &args![pipboy, 0u32, step, time, 1u32]);
}

/// `FCOMP; TEST AH,0x41; JNZ` not taken: `a` strictly above `b` (ordered).
#[inline]
fn above(a: f64, b: f64) -> bool {
    a.partial_cmp(&b) == Some(std::cmp::Ordering::Greater)
}

/// `FCOMP; TEST AH,5; JNP` taken: `a` below `b` (ordered).
#[inline]
fn below(a: f64, b: f64) -> bool {
    a.partial_cmp(&b) == Some(std::cmp::Ordering::Less)
}

/// `TEST AH,0x44; JP` not taken: `a` equal to `b` (ordered).
#[inline]
fn equal(a: f64, b: f64) -> bool {
    a.partial_cmp(&b) == Some(std::cmp::Ordering::Equal)
}

// ---- The idle ---------------------------------------------------------------------------------

// Typed from 0079a130
/// `MapMenu::DoIdle` (Xbox PDB): the note's sound paused while the start menu (0x3f5) is on top
/// (the time kept) and resumed once neither it nor 0x3e9 is (the paused time added to the start).
/// With the Pip-Boy up and the map menu on top, by page: the local map's 25 steps when asked, the
/// cursor over the maps; the selected note read once its delay has passed; the radio page's
/// waveform. With another menu on top the Pip-Boy tile fades in once on the first two pages. Then
/// the playing sounds while any load or play.
pub fn map_menu_do_idle(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let paused = is_top(e, 0x3f5) && field(e, at, CURRENT_SOUND) != 0 && {
        let handle = field(e, at, CURRENT_SOUND);
        e.call(SOUND_IS_PLAYING, &args![handle]).bool()
    };
    if paused {
        let handle = field(e, at, CURRENT_SOUND);
        e.call(SOUND_PAUSE, &args![handle]);
        e.mem.set_u8(SOUND_PAUSED, 1);
        let time = now(e);
        e.mem.set_u32(PAUSE_TIME, time);
    } else if !is_top(e, 0x3f5)
        && !is_top(e, 0x3e9)
        && field(e, at, CURRENT_SOUND) != 0
        && e.mem.u8(SOUND_PAUSED) != 0
    {
        let handle = field(e, at, CURRENT_SOUND);
        e.call(SOUND_PLAY, &args![handle, 0u32]);
        e.mem.set_u8(SOUND_PAUSED, 0);
        let start = now(e)
            .wrapping_sub(e.mem.u32(PAUSE_TIME))
            .wrapping_add(field(e, at, PLAY_START));
        e.mem.set_u32(at.wrapping_add(PLAY_START), start);
    }
    let pipboy = get_pipboy(e);
    if e.mem.u8(pipboy.wrapping_add(in_place::PIPBOY_QUERY)) == 0 {
        return;
    }
    if is_top(e, 0x3ff) {
        match page(e, singleton(e)).wrapping_sub(LOCAL_PAGE) {
            0 => {
                if e.mem.u8(setting_value(LOCAL_STEP_SETTING)) != 0 {
                    let pipboy = get_pipboy(e);
                    if e.call(PIPBOY_READY, &args![pipboy]).bool() {
                        let menu = singleton(e);
                        e.mem.set_u32(menu.wrapping_add(NEXT_LOCAL_GRID), 0);
                        for _ in 0..0x19 {
                            typed_call!(super::FAMILY, 0x0079_ffb0, fn_0079ffb0(e, 0));
                        }
                    }
                }
                idle_show_position(e, at);
            }
            1 => idle_show_position(e, at),
            3 => idle_read_note(e, at),
            4 => {
                if typed_call!(super::FAMILY, 0x0079_8440, map_menu_p2::fn_00798440(e)) != 0 {
                    typed_call!(
                        super::FAMILY,
                        0x0079_c0c0,
                        map_menu_p2::fn_0079c0c0(e, this)
                    );
                } else {
                    typed_call!(
                        super::FAMILY,
                        0x0079_c090,
                        map_menu_p2::fn_0079c090(e, this)
                    );
                }
            }
            _ => {}
        }
        e.mem.set_u8(FADED, 0);
    } else if e.mem.u8(FADED) == 0 && !manager_busy(e) && e.mem.i32(at.wrapping_add(PAGE)) <= 0x21 {
        let alpha = float_constant(e, FULL_ALPHA_FLOAT);
        let manager = in_place::manager(e);
        set_float(e, owner(e, manager), 0xfa9, alpha);
        e.mem.set_u8(FADED, 1);
    }
    if e.mem.u8(at.wrapping_add(SOUNDS_LOADING)) != 0 || field(e, at, CURRENT_SOUND) != 0 {
        typed_call!(
            super::FAMILY,
            0x0079_a660,
            map_menu_update_playing_sounds(e, this)
        );
    }
}

/// The notes page of `DoIdle`: once the note's enter time is older than the delay setting (in
/// seconds, times 1000.0; ordered), the selected note (the list's highlighted one while the manager
/// is busy or none is selected) dims its text and is marked read.
fn idle_read_note(e: &mut Engine, at: u32) {
    if field(e, at, NOTE_ENTER_TIME) == 0 {
        return;
    }
    let elapsed = f64::from(now(e).wrapping_sub(field(e, at, NOTE_ENTER_TIME)));
    let thousand: f64 = e.global(THOUSAND_DOUBLE);
    let wait = x87_mul(
        x87_load32(e.mem.u32(setting_value(NOTE_DELAY_SETTING))),
        thousand,
    );
    if !matches!(
        wait.partial_cmp(&elapsed),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    ) {
        return;
    }
    e.with_stack(4, |e, slot| {
        let slot = slot.addr();
        let selected = field(e, at, SELECTED_NOTE);
        e.mem.set_u32(slot, selected);
        if manager_busy(e) || field(e, at, SELECTED_NOTE) == 0 {
            let note = typed_call!(
                super::FAMILY,
                0x007a_1910,
                map_menu::fn_007a1910(e, Ptr::new(at.wrapping_add(NOTES)))
            );
            e.mem.set_u32(slot, note);
        }
        let tile = e
            .call(LIST_BOX_GET_TILE_FOR, &args![at.wrapping_add(NOTES), slot])
            .u32();
        if e.mem.u32(slot) != 0 && tile != 0 {
            let alpha = float_constant(e, HALF_ALPHA_FLOAT);
            let id = static_id(e, TRAIT_TEXT_ALPHA);
            set_float(e, tile, id, alpha);
            let note = e.mem.u32(slot);
            e.call(NOTE_SET_READ, &args![note, 1u32]);
            e.mem.set_u32(at.wrapping_add(NOTE_ENTER_TIME), 0);
        }
    });
}

/// The maps' pages of `DoIdle`, unless the manager is busy: tile 6 (the cursor) follows the mouse
/// over the map (relative to tile 6's owner, its half size taken off); off the 850 x 500 map the
/// Pip-Boy tile is opaque, on it clear, with the menu's gamepad slot sent 0xc when the controls say
/// so; then the marker under the cursor (`00799dc0`).
fn idle_show_position(e: &mut Engine, at: u32) {
    if manager_busy(e) {
        return;
    }
    let manager = in_place::manager(e);
    let cursor_x = e.call(CURSOR_X, &args![manager]).f64();
    let left = absolute_x(e, owner(e, map_tile(e, at, 6)));
    let mut x = x87_store32(x87_sub(cursor_x, left));
    let manager = in_place::manager(e);
    let cursor_y = f64::from(typed_call!(
        super::FAMILY,
        0x0079_83c0,
        map_menu_p2::fn_007983c0(e, Ptr::new(manager))
    ));
    let top = absolute_y(e, owner(e, map_tile(e, at, 6)));
    let mut y = x87_store32(x87_sub(cursor_y, top));
    let zero: f64 = e.global(ZERO_DOUBLE);
    let width: f64 = e.global(MAP_WIDTH_DOUBLE);
    let height: f64 = e.global(MAP_HEIGHT_DOUBLE);
    let outside = below(x87_load32(x), zero)
        || below(x87_load32(y), zero)
        || above(x87_load32(x), width)
        || above(x87_load32(y), height);
    let manager = in_place::manager(e);
    if outside {
        let alpha = float_constant(e, FULL_ALPHA_FLOAT);
        set_float(e, owner(e, manager), 0xfa9, alpha);
    } else {
        set_float(e, owner(e, manager), 0xfa9, 0);
        let controls = in_place::controls(e);
        if e.call(CONTROLS_TEST_A, &args![controls, 1u32, 1u32]).u32() != 0 {
            let controls = in_place::controls(e);
            if e.call(CONTROLS_TEST_B, &args![controls, 0xeu32, 1u32]).u32() == 0 {
                e.vcall(at, MENU_GAMEPAD_SLOT, &args![0xcu32, 0u32]);
            }
        }
    }
    let two: f64 = e.global(TWO_DOUBLE);
    let width = get_float64(e, map_tile(e, at, 6), 0xfb1);
    x = x87_store32(x87_sub(x87_m32(x), x87_div(width, two)));
    let height = get_float64(e, map_tile(e, at, 6), 0xfb0);
    y = x87_store32(x87_sub(x87_m32(y), x87_div(height, two)));
    set_float(e, map_tile(e, at, 6), 0xfa1, x87_quiet_bits(x));
    set_float(e, map_tile(e, at, 6), 0xfa2, x87_quiet_bits(y));
    typed_call!(super::FAMILY, 0x0079_9dc0, fn_00799dc0(e, Ptr::new(at)));
}

// ---- The player marker's space and the callbacks ----------------------------------------------

// Typed from 007984d0
/// `MapMenu::CalculatePlayerMarkerSpace` (Xbox PDB): the first map marker of the challenges'
/// owner's list (`+0x10`) that counts: a location name, not starting with `"test"`
/// (`BSstristr`), not skipped (`00586210`), the point (`x`, `y`, `z`, adjusted by
/// `TESWorldSpace::AdjustMapMarkerCoord` with 1) as a `PathingLocation` of the marker passing
/// `006de250`, a root map space, and both forms' texts at least 0x1f long agreeing in their three
/// characters from +0x1d. Else the list's first item. The exe's copy of the largest float into a
/// local it never reads is left out.
pub fn map_menu_calculate_player_marker_space(
    e: &mut Engine,
    this: Ptr,
    x: u32,
    y: u32,
    z: u32,
) -> u32 {
    let at = this.addr();
    let mut found = 0u32;
    let mut node = e.mem.u32(OWNER).wrapping_add(0x10);
    e.with_stack(0x5c, |e, frame| {
        // The locals: the point (+0), the location (+0xc) and the temporary location (+0x34).
        let point = frame.addr();
        let location = point.wrapping_add(0xc);
        let temporary = point.wrapping_add(0x34);
        while node != 0 && item(e, node) != 0 {
            let marker = item(e, node);
            let name = interface_cpp_p2::name_characters(e, marker.wrapping_add(0x18));
            set_point_at(e, point, [x, y, z]);
            typed_call!(
                pathing::FAMILY,
                0x006d_cbb0,
                pathing::location::new(e, Ptr::new(location))
            );
            let skip = name == 0
                || e.mem.u8(name) == 0
                || e.call(BS_STRISTR, &args![name, TEST_TEXT]).u32() == name
                || e.call(REFERENCE_SKIPPED, &args![marker]).bool();
            if !skip {
                e.call(ADJUST_MAP_MARKER_COORD, &args![marker, point, 1u32]);
                let made = e
                    .call(PATHING_LOCATION_AT, &args![temporary, point, marker])
                    .u32();
                typed_call!(
                    pathing::FAMILY,
                    0x006d_cce0,
                    pathing::location::assign(e, Ptr::new(location), Ptr::new(made))
                );
                e.call(PATHING_LOCATION_DESTRUCT, &args![temporary]);
                let root = field(e, at, ROOT_SPACE);
                if e.call(PATHING_LOCATION_TEST, &args![location]).bool()
                    && root != 0
                    && marker != 0
                    && texts_agree(e, at, marker)
                {
                    found = marker;
                    e.call(PATHING_LOCATION_DESTRUCT, &args![location]);
                    break;
                }
            }
            e.call(PATHING_LOCATION_DESTRUCT, &args![location]);
            node = next(e, node);
        }
    });
    if found == 0 {
        found = item(e, e.mem.u32(OWNER).wrapping_add(0x10));
    }
    found
}

/// A form text's length (`0044a670`, the forms family's).
#[inline]
fn text_length(e: &mut Engine, text: u32) -> u32 {
    typed_call!(
        forms::FAMILY,
        0x0044_a670,
        forms::debug_string::text_length(e, text)
    )
}

/// `CalculatePlayerMarkerSpace`'s last test: the root space's text (`00586440`) and the marker's
/// at least 0x1f long, and their three characters from +0x1d the same (`004564f0`), each text
/// asked again as the exe does.
fn texts_agree(e: &mut Engine, at: u32, marker: u32) -> bool {
    let root_text = e.call(FORM_TEXT, &args![field(e, at, ROOT_SPACE)]).u32();
    let long_enough = text_length(e, root_text) >= 0x1f && {
        let text = e.call(FORM_TEXT, &args![marker]).u32();
        text_length(e, text) >= 0x1f
    };
    if !long_enough {
        return false;
    }
    let text = e.call(FORM_TEXT, &args![marker]).u32();
    let root_text = e.call(FORM_TEXT, &args![field(e, at, ROOT_SPACE)]).u32();
    e.call(
        TEXT_COMPARE_N,
        &args![root_text.wrapping_add(0x1d), text.wrapping_add(0x1d), 3u32],
    )
    .u32()
        == 0
}

/// The block `00798710` and `SetPlayerMarkerCallback` end with: unless the manager is busy, tile 6
/// centred on the cursor (relative to its owner, its half size taken off, each difference kept on
/// the x87 stack); then tile 6's trait `0xfa3` set to 1.
fn centre_cursor_tile(e: &mut Engine) {
    if !manager_busy(e) {
        let two: f64 = e.global(TWO_DOUBLE);
        let manager = in_place::manager(e);
        let cursor_x = e.call(CURSOR_X, &args![manager]).f64();
        let left = absolute_x(e, owner(e, map_tile(e, singleton(e), 6)));
        let x = x87_sub(cursor_x, left);
        let width = get_float64(e, map_tile(e, singleton(e), 6), 0xfb1);
        let x = x87_quiet_bits(x87_store32(x87_sub(x, x87_div(width, two))));
        set_float(e, map_tile(e, singleton(e), 6), 0xfa1, x);
        let manager = in_place::manager(e);
        let cursor_y = f64::from(typed_call!(
            super::FAMILY,
            0x0079_83c0,
            map_menu_p2::fn_007983c0(e, Ptr::new(manager))
        ));
        let top = absolute_y(e, owner(e, map_tile(e, singleton(e), 6)));
        let y = x87_sub(cursor_y, top);
        let height = get_float64(e, map_tile(e, singleton(e), 6), 0xfb0);
        let y = x87_quiet_bits(x87_store32(x87_sub(y, x87_div(height, two))));
        set_float(e, map_tile(e, singleton(e), 6), 0xfa2, y);
    }
    set_int(e, map_tile(e, singleton(e), 6), 0xfa3, 1);
}

/// `Interface::GetMessageMenuResult` (this family's).
#[inline]
fn message_result(e: &mut Engine) -> u8 {
    typed_call!(
        super::FAMILY,
        0x0070_3fa0,
        interface_cpp::interface_get_message_menu_result(e)
    )
}

// Typed from 00798710
/// The fast travel question's callback: on "no" (result 0) the manager's next callback is
/// `00798a00` (`0070f690`), otherwise the fast travel marker is cleared; then the cursor tile is
/// centred.
pub fn fn_00798710(e: &mut Engine) {
    if message_result(e) == 0 {
        let manager = in_place::manager(e);
        e.call(SET_MESSAGE_CALLBACK, &args![manager, FAST_TRAVEL_CONFIRMED]);
    } else {
        let menu = singleton(e);
        e.mem.set_u32(menu.wrapping_add(FAST_TRAVEL_MARKER), 0);
    }
    centre_cursor_tile(e);
}

// Typed from 00798840
/// `MapMenu::SetPlayerMarkerCallback` (Xbox PDB): by the message box's result: 0 or 3 set the
/// player's map marker at the menu's marker point and space
/// (`PlayerCharacter::SetPlayerMapMarker`), delete the old marker on 0 and create the new one; 1
/// removes the player's marker and deletes it. Then the marker point is reset from `011f426c`, the
/// marker space cleared and the cursor tile centred.
pub fn map_menu_set_player_marker_callback(e: &mut Engine) {
    let result = message_result(e);
    match result {
        0 | 3 => {
            let menu = singleton(e);
            let space = field(e, menu, MARKER_SPACE);
            let [x, y, z] = point_at(e, menu.wrapping_add(MARKER_POINT));
            let player = e.mem.u32(PLAYER);
            e.call(SET_PLAYER_MAP_MARKER, &args![player, x, y, z, space]);
            if result == 0 {
                let menu = singleton(e);
                typed_call!(
                    super::FAMILY,
                    0x0079_f6f0,
                    map_menu_delete_player_marker(e, Ptr::new(menu))
                );
            }
            let menu = singleton(e);
            typed_call!(
                super::FAMILY,
                0x0079_f360,
                map_menu_create_player_marker(e, Ptr::new(menu))
            );
        }
        1 => {
            let player = e.mem.u32(PLAYER);
            e.call(REMOVE_PLAYER_MAP_MARKER, &args![player]);
            let menu = singleton(e);
            typed_call!(
                super::FAMILY,
                0x0079_f6f0,
                map_menu_delete_player_marker(e, Ptr::new(menu))
            );
        }
        _ => {}
    }
    let menu = singleton(e);
    let start = point_at(e, MARKER_START);
    set_point_at(e, menu.wrapping_add(MARKER_POINT), start);
    let menu = singleton(e);
    e.mem.set_u32(menu.wrapping_add(MARKER_SPACE), 0);
    centre_cursor_tile(e);
}

// Typed from 00798a00
/// The fast travel confirmation: with an open map menu and a fast travel marker, a 0x34-byte
/// request (`00456540`) with the marker at its +0x30 goes to
/// `PlayerCharacter::RequestPositionPlayer` and the marker is cleared.
pub fn fn_00798a00(e: &mut Engine) {
    if singleton(e) == 0 || field(e, singleton(e), FAST_TRAVEL_MARKER) == 0 {
        return;
    }
    let memory = e.call(ALLOCATE, &args![0x34u32]).u32();
    let request = if memory != 0 {
        e.call(POSITION_REQUEST_CONSTRUCT, &args![memory]).u32()
    } else {
        0
    };
    let marker = field(e, singleton(e), FAST_TRAVEL_MARKER);
    e.mem.set_u32(request.wrapping_add(0x30), marker);
    let player = e.mem.u32(PLAYER);
    e.call(REQUEST_POSITION_PLAYER, &args![player, request]);
    let menu = singleton(e);
    e.mem.set_u32(menu.wrapping_add(FAST_TRAVEL_MARKER), 0);
}

// ---- The note's sounds ------------------------------------------------------------------------

/// The part `StopNoteSound` and `ClearAudioLogSubtites` share, on the menu `menu` gives (read again
/// at each step, as the exe does): the current sound stopped while it plays, the sound list
/// emptied, the subtitle array released, the current sound, the length, the start time and the
/// loading byte cleared.
fn stop_note_sounds(e: &mut Engine, menu: impl Fn(&Engine) -> u32) {
    let node = field(e, menu(e), CURRENT_SOUND);
    if node != 0 && e.call(SOUND_IS_PLAYING, &args![node]).bool() {
        let node = field(e, menu(e), CURRENT_SOUND);
        e.call(SOUND_STOP, &args![node]);
    }
    let sounds = menu(e).wrapping_add(SOUNDS);
    e.call(SOUND_LIST_CLEAR, &args![sounds]);
    let subtitles = menu(e).wrapping_add(SUBTITLES);
    typed_call!(
        super::FAMILY,
        0x007a_1c30,
        map_menu::fn_007a1c30(e, Ptr::new(subtitles), 1)
    );
    e.mem.set_u32(menu(e).wrapping_add(CURRENT_SOUND), 0);
    e.mem.set_u32(menu(e).wrapping_add(SOUNDS_LENGTH), 0);
    e.mem.set_u32(menu(e).wrapping_add(PLAY_START), 0);
    e.mem.set_u8(menu(e).wrapping_add(SOUNDS_LOADING), 0);
}

// Typed from 00798ad0
/// `MapMenu::StopNoteSound` (Xbox PDB): the note's sounds and subtitles stopped and cleared,
/// `"UIPipBoyHolotapeStop"` played, dialogue audio left (`BSAudio::ExitDialogue`), then `007624c0`.
pub fn map_menu_stop_note_sound(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    stop_note_sounds(e, |_| at);
    play_named_sound(e, HOLOTAPE_STOP_SOUND);
    let audio = e.mem.u32(AUDIO);
    e.call(AUDIO_EXIT_DIALOGUE, &args![audio]);
    e.call(AFTER_NOTE_SOUND, &args![]);
}

// Typed from 00798bf0
/// `MapMenu::ClearAudioLogSubtites` (Xbox PDB): with an open map menu, its note's sounds and
/// subtitles stopped and cleared.
pub fn map_menu_clear_audio_log_subtites(e: &mut Engine) {
    if singleton(e) == 0 {
        return;
    }
    stop_note_sounds(e, singleton);
}

// ---- The data pages' enter and leave, the loads -----------------------------------------------

/// The index (trait `0xfac`, through `_ftol2`) of the tile the list box at `list` has highlighted
/// (its virtual slot +4, asked twice as the exe does), or 0 without one.
fn highlighted_index(e: &mut Engine, list: u32) -> i32 {
    if e.vcall(list, HIGHLIGHTED_SLOT, &args![]).u32() == 0 {
        return 0;
    }
    let tile = e.vcall(list, HIGHLIGHTED_SLOT, &args![]).u32();
    let value = get_float64(e, tile, 0xfac);
    ftol2(e, value)
}

/// A list box's item of a tile (`007e1110`, this family's).
#[inline]
fn item_of(e: &mut Engine, list: u32, tile: u32) -> u32 {
    typed_call!(
        super::FAMILY,
        0x007e_1110,
        stats_menu::fn_007e1110(e, Ptr::new(list), tile)
    )
}

/// The notes', quests' and objectives' `SetEnabled` (this family's).
fn set_lists_enabled(e: &mut Engine, at: u32, notes: Option<u8>, quests: Option<(u8, u8)>) {
    if let Some(value) = notes {
        typed_call!(
            super::FAMILY,
            0x007a_30e0,
            map_menu::list_box_bgs_note_set_enabled(e, Ptr::new(at.wrapping_add(NOTES)), value)
        );
    }
    if let Some((quests, objectives)) = quests {
        typed_call!(
            super::FAMILY,
            0x007a_2350,
            map_menu::list_box_tes_quest_set_enabled(e, Ptr::new(at.wrapping_add(QUESTS)), quests)
        );
        typed_call!(
            super::FAMILY,
            0x007a_4c50,
            map_menu::list_box_bgs_quest_objective_set_enabled(
                e,
                Ptr::new(at.wrapping_add(OBJECTIVES)),
                objectives
            )
        );
    }
}

/// Whether `tile`'s owner is the objectives list's scroll bar (the list's word at +0x14,
/// `00825c00` in place).
fn on_objectives_scroll_bar(e: &Engine, at: u32, tile: u32) -> bool {
    owner(e, tile) == e.mem.u32(at.wrapping_add(OBJECTIVES + 0x14))
}

// Typed from 00798cb0
/// `MapMenu::DoEnter` (Xbox PDB): by id: -1 sends the click slot 0xf when `tile`'s owner is the
/// objectives' scroll bar; 14 disables the notes list; 15 disables the quests list and enables the
/// objectives'; 0x17 shows the quest of `tile` (its objectives filtered by quest and target, the
/// item type 4 on tile 13, the target and open flags on tiles 12 and 11); 0x18 loads the note (or
/// in challenge mode the challenge) of `tile`; 0x19 reads the radio list's index. For 0x17 to 0x19
/// the Pip-Boy's knob then turns one step up or down when the index differs from the last one.
pub fn map_menu_do_enter(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    let at = this.addr();
    let mut index = 0i32;
    match id.wrapping_add(1) as u32 {
        0 => {
            if tile != 0 && on_objectives_scroll_bar(e, at, tile) {
                e.vcall(at, MENU_CLICK_SLOT, &args![0xfu32, 0u32]);
            }
        }
        15 => set_lists_enabled(e, at, Some(0), None),
        16 => set_lists_enabled(e, at, None, Some((0, 1))),
        0x18 => {
            if tile != 0 {
                index = enter_quest(e, at, tile);
            }
        }
        0x19 => {
            if e.mem.u8(CHALLENGE_MODE) == 0 {
                enter_note(e, at, tile, &mut index);
            } else {
                let challenge = item_of(e, at.wrapping_add(CHALLENGES), tile);
                if manager_busy(e) || challenge != 0 {
                    typed_call!(
                        super::FAMILY,
                        0x0079_91b0,
                        map_menu_load_challenge(e, this, challenge)
                    );
                }
                set_int(e, map_tile(e, at, 13), 0xfa3, 1);
                typed_call!(
                    super::FAMILY,
                    0x0070_6f30,
                    interface_cpp_p2::interface_play_menu_sound(e, 4)
                );
            }
        }
        0x1a => index = highlighted_index(e, at.wrapping_add(RADIO)),
        _ => {}
    }
    let last = e.mem.i32(KNOB_INDEX);
    if last == index || !(0x17..=0x19).contains(&id) {
        return;
    }
    let step = setting_float(e, KNOB_STEP_SETTING);
    // `FCHS` flips the sign bit.
    let step = if last < index {
        step
    } else {
        step ^ 0x8000_0000
    };
    let time = setting_float(e, KNOB_TIME_SETTING);
    move_knob(e, step, time);
    e.mem.set_i32(KNOB_INDEX, index);
}

/// `DoEnter`'s quest (id 0x17): the quest of `tile` displayed, the objectives filtered by it
/// (`0079bc50`), the item type 4 and the data shown on tile 13, the target cleared and found again
/// by the range filter (`00799100`), tile 12 showing whether there is one and tile 11 whether the
/// quest is open (not done). The tile's index.
fn enter_quest(e: &mut Engine, at: u32, tile: u32) -> i32 {
    let quest = item_of(e, at.wrapping_add(QUESTS), tile);
    e.mem.set_u32(at.wrapping_add(DISPLAYED_QUEST), quest);
    typed_call!(
        super::FAMILY,
        0x007a_49e0,
        map_menu::list_box_bgs_quest_objective_filter(
            e,
            Ptr::new(at.wrapping_add(OBJECTIVES)),
            OBJECTIVE_QUEST_FILTER
        )
    );
    let value = get_float64(e, tile, 0xfac);
    let index = ftol2(e, value);
    let item_type = trait_by_name(e, ITEM_TYPE_TEXT);
    set_int(e, map_tile(e, at, 13), item_type, 4);
    set_int(e, map_tile(e, at, 13), 0xfa3, 1);
    e.mem.set_u32(at.wrapping_add(DISPLAYED_TARGET), 0);
    e.call(
        LIST_BOX_FILTER_RANGE,
        &args![
            at.wrapping_add(OBJECTIVES),
            OBJECTIVE_TARGET_FILTER,
            0u32,
            0x7fff_ffffu32
        ],
    );
    let has_target = u32::from(field(e, at, DISPLAYED_TARGET) != 0);
    set_int(e, map_tile(e, at, 12), 0xfaf, has_target);
    let quest = field(e, at, DISPLAYED_QUEST);
    // A quest's done flag (`0059e400`, in place): bit 1 of its byte at +0x3c.
    let open = quest != 0 && e.mem.u8(quest.wrapping_add(0x3c)) & 2 == 0;
    set_int(e, map_tile(e, at, 11), 0xfaf, u32::from(open));
    index
}

/// `DoEnter`'s note (id 0x18 outside challenge mode): the note of `tile` and the list's index; the
/// note loaded while the manager is busy or when none is selected; an audio note (type 0 or 3)
/// shows on tile 13 whether it is the selected one playing, another unread note starts its read
/// delay.
fn enter_note(e: &mut Engine, at: u32, tile: u32, index: &mut i32) {
    let note = item_of(e, at.wrapping_add(NOTES), tile);
    *index = highlighted_index(e, at.wrapping_add(NOTES));
    if manager_busy(e) || (note != 0 && field(e, at, SELECTED_NOTE) == 0) {
        typed_call!(
            super::FAMILY,
            0x0079_93d0,
            map_menu_load_note(e, Ptr::new(at), note)
        );
    }
    if e.call(NOTE_TYPE, &args![note]).u32() == 0 || e.call(NOTE_TYPE, &args![note]).u32() == 3 {
        let playing = note == field(e, at, SELECTED_NOTE) && field(e, at, CURRENT_SOUND) != 0;
        set_int(e, map_tile(e, at, 13), 0x1004, u32::from(playing));
    } else if typed_call!(
        super::FAMILY,
        0x0079_8420,
        map_menu_p2::fn_00798420(e, Ptr::new(note))
    ) == 0
    {
        let time = now(e);
        e.mem.set_u32(at.wrapping_add(NOTE_ENTER_TIME), time);
    }
}

// Typed from 00799100
/// The objectives' target filter (cdecl; `DoEnter`'s range filter callback): until the map menu has
/// a displayed target, the first target of `objective` (unless its state is above 1, `005a5dc0` in
/// place) whose condition (`00717e50`: the target's +4, in place) is true for its reference
/// (`TESQuestTarget::GetReference(0)`, `TESCondition::IsTrue(.., 0)`). The first word is not read.
pub fn fn_00799100(e: &mut Engine, _unused: u32, objective: u32) {
    if field(e, singleton(e), DISPLAYED_TARGET) != 0 || objective == 0 {
        return;
    }
    if e.mem.i32(objective.wrapping_add(0x20)) > 1 {
        return;
    }
    let mut node = e.call(OBJECTIVE_TARGETS, &args![objective]).u32();
    while field(e, singleton(e), DISPLAYED_TARGET) == 0 && node != 0 && item(e, node) != 0 {
        let target = item(e, node);
        let reference = e
            .call(QUEST_TARGET_GET_REFERENCE, &args![target, 0u32])
            .u32();
        let condition = target.wrapping_add(4);
        if e.call(CONDITION_IS_TRUE, &args![condition, reference, 0u32])
            .bool()
        {
            let menu = singleton(e);
            e.mem.set_u32(menu.wrapping_add(DISPLAYED_TARGET), target);
        }
        node = next(e, node);
    }
}

/// Tile 13's data text and text scroll bar children (`"MM_DataText"`, `"MM_TextScrollbar"`), the
/// scroll bar's `"_ScrollDelta"` (stored as a float) and the item type `kind` on tile 13.
fn data_text_children(e: &mut Engine, at: u32, kind: u32) -> (u32, u32, u32) {
    let data_text = child_by_name(e, map_tile(e, at, 13), DATA_TEXT_NAME);
    let scrollbar = child_by_name(e, map_tile(e, at, 13), TEXT_SCROLLBAR_NAME);
    let delta_id = trait_by_name(e, SCROLL_DELTA_TEXT);
    let delta = get_float_bits(e, scrollbar, delta_id);
    let item_type = trait_by_name(e, ITEM_TYPE_TEXT);
    set_int(e, map_tile(e, at, 13), item_type, kind);
    (data_text, scrollbar, delta)
}

// Typed from 007991b0
/// `MapMenu::LoadChallenge` (Xbox PDB): tile 13 shows item type 1 and, in its data text, the
/// challenge's text (`"%s\n%d\%d\n\n%s"`: its name, the smaller of its value and its count, its
/// count and its description) in a 0x200-byte buffer, with the text scroll bar set up.
pub fn map_menu_load_challenge(e: &mut Engine, this: Ptr, challenge: u32) {
    let at = this.addr();
    let (data_text, scrollbar, delta) = data_text_children(e, at, 1);
    let description = e
        .vcall(
            challenge.wrapping_add(0x24),
            TEXT_SLOT,
            &args![0u32, 0x4353_4544u32],
        )
        .u32();
    let count = e.call(CHALLENGE_COUNT, &args![challenge]).u32();
    let second = e.call(CHALLENGE_COUNT, &args![challenge]).u32();
    // A challenge's value (`005e3fa0`: +0x6c) and the smaller of the two (`004a8f20`, signed), in
    // place.
    let value = e.mem.i32(challenge.wrapping_add(0x6c));
    let shown = value.min(second as i32) as u32;
    let name = interface_cpp_p2::name_characters(e, challenge.wrapping_add(0x18));
    e.with_stack(0x200, |e, text| {
        e.call(
            BUFFER_FORMAT,
            &args![
                text,
                0x200u32,
                CHALLENGE_TEXT_FORMAT,
                name,
                shown,
                count,
                description
            ],
        );
        set_string(e, data_text, 0xfc4, text.addr());
        set_text_scrollbar(e, data_text, scrollbar, delta, text.addr());
    });
}

/// The text scroll bar of `LoadNote` and `LoadChallenge`: `"_number_of_items"` from the measured
/// height of `text` (the font manager's `CalculateStringDimensions`, wrap width `010311a8`) less
/// the data text's owner's height (at least 0.0), over `"_ScrollDelta"`, rounded, plus 1.0;
/// `"_current_value"` 0.0 and `"_SetInCode"` poked with 1.0.
fn set_text_scrollbar(e: &mut Engine, data_text: u32, scrollbar: u32, delta: u32, text: u32) {
    e.with_stack(0x10, |e, dimensions| {
        let width = float_constant(e, TEXT_WIDTH_FLOAT);
        // The font manager (`005bd5b0`, in place: the word at `011f33f8`).
        let font = e.mem.u32(0x011f_33f8);
        let result = e
            .call(
                FONT_CALCULATE_STRING_DIMENSIONS,
                &args![font, dimensions, text, 1u32, width, 0u32],
            )
            .u32();
        let height = x87_quiet_bits(e.mem.u32(result.wrapping_add(4)));
        let visible = get_float64(e, owner(e, data_text), 0xfb0);
        let overflow = x87_quiet_bits(x87_store32(x87_sub(x87_m32(height), visible)));
        let overflow = typed_call!(
            draw_f::FAMILY,
            0x0040_4010,
            draw_f::helpers::float_max(e, 0.0, f32::from_bits(overflow))
        );
        let overflow = x87_store32(f64::from(overflow));
        let steps = x87_quiet_bits(x87_store32(x87_div(
            x87_load32(overflow),
            x87_m32(delta),
        )));
        let rounded = e.call(ROUND_FLOAT, &args![steps]).f64();
        let one: f64 = e.global(ONE_DOUBLE);
        let items = x87_quiet_bits(x87_store32(x87_add(rounded, one)));
        let number_of_items = trait_by_name(e, NUMBER_OF_ITEMS_TEXT);
        set_float(e, scrollbar, number_of_items, items);
    });
    let current_value = trait_by_name(e, CURRENT_VALUE_TEXT);
    set_float_flag(e, scrollbar, current_value, 0, 0);
    let set_in_code = trait_by_name(e, SET_IN_CODE_TEXT);
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1290,
        dispatch::tile::poke(e, Ptr::new(scrollbar), set_in_code, 1.0)
    );
}

// Typed from 007993d0
/// `MapMenu::LoadNote` (Xbox PDB): by the note's type: 1 (text) shows item type 1 and the note's
/// text (its text object's slot +0x10 with the note and `'TNAM'`, asked twice) with the text scroll
/// bar set up; 2 (image) the image path in the data image's `0xfcc` and item type 2; 0 and 3
/// (audio) item type 3 and, without a playing sound, the time line `"--:--:- %s"` (a 0x40-byte
/// buffer) as `"_ItemStringInfo"` and 0.0 as `"_AudioFileCompletion"`. Then tile 13's `0xfa3` 1.
pub fn map_menu_load_note(e: &mut Engine, this: Ptr, note: u32) {
    let at = this.addr();
    match e.call(NOTE_TYPE, &args![note]).u32() {
        1 => {
            let (data_text, scrollbar, delta) = data_text_children(e, at, 1);
            let object = e.call(NOTE_TEXT_OBJECT, &args![note]).u32();
            let text = e
                .vcall(object, TEXT_SLOT, &args![note, 0x4d41_4e54u32])
                .u32();
            set_string(e, data_text, 0xfc4, text);
            let object = e.call(NOTE_TEXT_OBJECT, &args![note]).u32();
            let text = e
                .vcall(object, TEXT_SLOT, &args![note, 0x4d41_4e54u32])
                .u32();
            set_text_scrollbar(e, data_text, scrollbar, delta, text);
        }
        2 => {
            let image = e.call(NOTE_IMAGE, &args![note]).u32();
            let path = interface_cpp_p2::name_characters(e, image);
            let data_image = child_by_name(e, map_tile(e, at, 13), DATA_IMAGE_NAME);
            set_string(e, data_image, 0xfcc, path);
            let item_type = trait_by_name(e, ITEM_TYPE_TEXT);
            set_int(e, map_tile(e, at, 13), item_type, 2);
        }
        0 | 3 => {
            let item_type = trait_by_name(e, ITEM_TYPE_TEXT);
            set_int(e, map_tile(e, at, 13), item_type, 3);
            if field(e, at, CURRENT_SOUND) == 0 {
                let label = in_place::setting_text(e, AUDIO_LABEL);
                e.with_stack(0x40, |e, text| {
                    e.call(
                        BUFFER_FORMAT,
                        &args![text, 0x40u32, AUDIO_TIME_FORMAT, label],
                    );
                    let info = trait_by_name(e, ITEM_STRING_INFO_TEXT);
                    set_string(e, map_tile(e, at, 13), info, text.addr());
                });
                let completion = trait_by_name(e, AUDIO_COMPLETION_TEXT);
                set_float(e, map_tile(e, at, 13), completion, 0);
            }
        }
        _ => {}
    }
    set_int(e, map_tile(e, at, 13), 0xfa3, 1);
}

// Typed from 00799690
/// `MapMenu::DoLeave` (Xbox PDB): by id: -1 sends the leave slot 0xf when `tile`'s owner is the
/// objectives' scroll bar; 14 enables the notes list; 15 enables the quests list and disables the
/// objectives'; 0x18 clears the note's enter time while the manager is busy or no note is selected.
pub fn map_menu_do_leave(e: &mut Engine, this: Ptr, id: i32, tile: u32) {
    let at = this.addr();
    match id.wrapping_add(1) as u32 {
        0 => {
            if tile != 0 && on_objectives_scroll_bar(e, at, tile) {
                e.vcall(at, MENU_LEAVE_SLOT, &args![0xfu32, 0u32]);
            }
        }
        15 => set_lists_enabled(e, at, Some(1), None),
        16 => set_lists_enabled(e, at, None, Some((1, 0))),
        0x19 if manager_busy(e) || field(e, at, SELECTED_NOTE) == 0 => {
            e.mem.set_u32(at.wrapping_add(NOTE_ENTER_TIME), 0);
        }
        _ => {}
    }
}

// ---- The marker under the cursor and the playing sounds ---------------------------------------

// Typed from 00799dc0
/// The marker under the cursor (`MapMenu`): with a map tile, the centre of tile 6 (the cursor)
/// relative to the map tile; among the map tile's children the marker tiles (trait `0xfaa` 26.0,
/// name `" Marker"`) each lose their selected trait, and the nearest one closer than half of tile
/// 6's height (its position from the static x and y traits, scaled by the map tile's width) gets
/// it. A new one plays `"UIPipBoyHighlight"`, gives tile 6 its `"_LocationName"` as the title
/// unless it is `"Companion"`, becomes the highlighted marker and tile 11's `0xfaf` follows its
/// `0x1004`. Without one the title is emptied, the highlighted marker cleared and tile 11's `0xfaf`
/// set to 0.
pub fn fn_00799dc0(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    if field(e, at, MAP_TILE) == 0 {
        return;
    }
    let two: f64 = e.global(TWO_DOUBLE);
    let top = get_float64(e, map_tile(e, at, 6), 0xfa2);
    let height = get_float64(e, map_tile(e, at, 6), 0xfb0);
    let centre_y = x87_quiet_bits(x87_store32(x87_add(x87_div(height, two), top)));
    let left = get_float64(e, map_tile(e, at, 6), 0xfa1);
    let width = get_float64(e, map_tile(e, at, 6), 0xfb1);
    let centre_x = x87_quiet_bits(x87_store32(x87_add(x87_div(width, two), left)));
    let found = e.with_stack(0x1c, |e, frame| {
        // The locals: the cursor point (+0), the node (+8), the marker point (+0xc) and the
        // difference (+0x14).
        let cursor = frame.addr();
        let node = cursor.wrapping_add(8);
        let marker_point = cursor.wrapping_add(0xc);
        let difference = cursor.wrapping_add(0x14);
        point2_new(e, cursor, centre_x, centre_y);
        let first = item(e, field(e, at, MAP_TILE).wrapping_add(4));
        e.mem.set_u32(node, first);
        let mut best = float_constant(e, LARGEST_FLOAT);
        let mut found = 0u32;
        let map_x = get_float64(e, field(e, at, MAP_TILE), 0xfa1);
        let x = x87_store32(x87_sub(x87_m32(e.mem.u32(cursor)), map_x));
        e.mem.set_u32(cursor, x);
        let map_y = get_float64(e, field(e, at, MAP_TILE), 0xfa2);
        let y = x87_store32(x87_sub(
            x87_m32(e.mem.u32(cursor.wrapping_add(4))),
            map_y,
        ));
        e.mem.set_u32(cursor.wrapping_add(4), y);
        while e.mem.u32(node) != 0 {
            // The list walk's step (`0057cbe0`, in place): the node's +8, the next its first word.
            let step = e.mem.u32(node);
            e.mem.set_u32(node, e.mem.u32(step));
            let tile = e.mem.u32(step.wrapping_add(8));
            let kind = get_float64(e, tile, 0xfaa);
            let marker_kind: f64 = e.global(MARKER_KIND_DOUBLE);
            if !equal(kind, marker_kind) {
                continue;
            }
            let name = e.call(TILE_NAME, &args![tile]).u32();
            if e.call(TEXT_COMPARE, &args![name, MARKER_TEXT]).u32() != 0 {
                continue;
            }
            let id = static_id(e, TRAIT_Y);
            let marker_y = get_float_bits(e, tile, id);
            let id = static_id(e, TRAIT_X);
            let marker_x = get_float_bits(e, tile, id);
            point2_new(e, marker_point, marker_x, marker_y);
            let scale = get_float_bits(e, field(e, at, MAP_TILE), 0xfb1);
            e.call(POINT2_SCALE, &args![marker_point, scale]);
            let result = e
                .call(POINT2_SUBTRACT, &args![cursor, difference, marker_point])
                .u32();
            let distance = typed_call!(
                pathing::FAMILY,
                0x0058_9850,
                pathing::point::length_2d(e, Ptr::new(result))
            );
            let distance = x87_store32(f64::from(distance));
            if above(x87_load32(best), x87_load32(distance)) {
                let height = get_float64(e, map_tile(e, at, 6), 0xfb0);
                if below(x87_load32(distance), x87_div(height, two)) {
                    best = x87_quiet_bits(distance);
                    found = tile;
                }
            }
            let selected = static_id(e, TRAIT_SELECTED);
            set_int(e, tile, selected, 0);
        }
        found
    });
    if found == 0 {
        let title = user_trait(e, TITLE_TEXT);
        set_string(e, map_tile(e, at, 6), title, EMPTY_TEXT);
        e.mem.set_u32(at.wrapping_add(HIGHLIGHTED_MARKER), 0);
        set_int(e, map_tile(e, at, 11), 0xfaf, 0);
        return;
    }
    let selected = static_id(e, TRAIT_SELECTED);
    set_int(e, found, selected, 1);
    if found == field(e, at, HIGHLIGHTED_MARKER) {
        return;
    }
    play_named_sound(e, HIGHLIGHT_SOUND);
    let name_id = user_trait(e, LOCATION_NAME_TEXT);
    let name = get_string(e, found, name_id);
    let other = typed_call!(
        crt::FAMILY,
        0x00ec_68e4,
        crt::string::stricmp(e, name, COMPANION_TEXT)
    );
    if other != 0 {
        let name_id = user_trait(e, LOCATION_NAME_TEXT);
        let name = get_string(e, found, name_id);
        let title = user_trait(e, TITLE_TEXT);
        set_string(e, map_tile(e, at, 6), title, name);
    }
    e.mem.set_u32(at.wrapping_add(HIGHLIGHTED_MARKER), found);
    let companion = u32::from(is_true(e, found, 0x1004));
    set_int(e, map_tile(e, at, 11), 0xfaf, companion);
}

/// The subtitle of the note's current sound: `Interface::ShowSubtitle` with the first subtitle's
/// text, a copy of the sound handle (`00418900` in place: its word, byte and word, into a stack
/// slot, passed by value), the player's position (passed by value), the player and `flag`; then the
/// first subtitle removed (`007a1b10(0, 1)`).
fn show_note_subtitle(e: &mut Engine, at: u32, flag: u8) {
    let player = e.mem.u32(PLAYER);
    let [x, y, z] = position(e, player);
    e.with_stack(0xc, |e, copy| {
        let copy = copy.addr();
        let handle = field(e, at, CURRENT_SOUND);
        e.mem.set_u32(copy, e.mem.u32(handle));
        e.mem
            .set_u8(copy.wrapping_add(4), e.mem.u8(handle.wrapping_add(4)));
        e.mem
            .set_u32(copy.wrapping_add(8), e.mem.u32(handle.wrapping_add(8)));
        let element = typed_call!(
            pathing::FAMILY,
            0x0069_50d0,
            pathing::search::entry_8(e, Ptr::new(at.wrapping_add(SUBTITLES)), 0)
        );
        let text = item(e, element);
        let [h0, h1, h2] = point_at(e, copy);
        typed_call!(
            super::FAMILY,
            0x0070_5210,
            interface_cpp::interface_show_subtitle(e, text, h0, h1, h2, x, y, z, player, flag)
        );
    });
    typed_call!(
        super::FAMILY,
        0x007a_1b10,
        map_menu_p2::fn_007a1b10(e, Ptr::new(at.wrapping_add(SUBTITLES)), 0, 1)
    );
}

/// The play time text of an audio note into tile 13's `"_ItemStringInfo"` (a 0x40-byte buffer).
fn set_play_time_text(e: &mut Engine, at: u32, words: &[u32]) {
    e.with_stack(0x40, |e, text| {
        let mut call = vec![text.addr(), 0x40];
        call.extend_from_slice(words);
        e.call(BUFFER_FORMAT, &call);
        let info = trait_by_name(e, ITEM_STRING_INFO_TEXT);
        set_string(e, map_tile(e, at, 13), info, text.addr());
    });
}

// Typed from 0079a660
/// `MapMenu::UpdatePlayingSounds` (Xbox PDB): while the note's sounds load, the sound list walked:
/// no valid sound stops the note and restores the radio volume; all with a duration end the
/// loading, set the total length (the durations plus 500 per gap), play the first one, keep the
/// start time and show its subtitle. Otherwise, with a length, the remaining time (`"%02d:%02d:%01d
/// %s"`) and the completion shown. Then, when the current sound has ended: after more than 500 ms
/// the next one plays with its subtitle; at the last one the time reads `"00:00:0 %s"`, the
/// completion 1.0, tile 13's `0x1004` 0, and the note stops.
pub fn map_menu_update_playing_sounds(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    if e.mem.u8(at.wrapping_add(SOUNDS_LOADING)) != 0 {
        load_playing_sounds(e, this);
    } else {
        let zero: f64 = e.global(ZERO_DOUBLE);
        if above(x87_load32(field(e, at, SOUNDS_LENGTH)), zero) {
            show_play_time(e, at);
        }
    }
    let node = field(e, at, CURRENT_SOUND);
    if node == 0 || e.call(SOUND_IS_VALID, &args![node]).bool() {
        return;
    }
    if e.mem.u32(SOUND_WAIT) == 0 {
        let time = now(e);
        e.mem.set_u32(SOUND_WAIT, time);
    }
    // A sound node's next (`0084e3a0`, in place: its +0xc).
    let next_sound = |e: &Engine| e.mem.u32(field(e, at, CURRENT_SOUND).wrapping_add(0xc));
    if next_sound(e) != 0 && now(e).wrapping_sub(e.mem.u32(SOUND_WAIT)) > 0x1f4 {
        let next = next_sound(e);
        e.mem.set_u32(at.wrapping_add(CURRENT_SOUND), next);
        e.call(SOUND_PLAY, &args![next, 0u32]);
        e.mem.set_u32(SOUND_WAIT, 0);
        show_note_subtitle(e, at, 1);
        return;
    }
    if next_sound(e) != 0 {
        return;
    }
    let label = in_place::setting_text(e, AUDIO_LABEL);
    set_play_time_text(e, at, &[PLAY_DONE_FORMAT, label]);
    let completion = trait_by_name(e, AUDIO_COMPLETION_TEXT);
    set_float(e, map_tile(e, at, 13), completion, 1.0f32.to_bits());
    set_int(e, map_tile(e, at, 13), 0x1004, 0);
    typed_call!(
        super::FAMILY,
        0x0079_8ad0,
        map_menu_stop_note_sound(e, this)
    );
}

/// `UpdatePlayingSounds` while the sounds load (see there).
fn load_playing_sounds(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let mut all_ready = true;
    let mut none_valid = true;
    let mut total = 0u32;
    let mut count = 0u32;
    let mut node = at.wrapping_add(SOUNDS);
    while node != 0 {
        if e.call(SOUND_IS_VALID, &args![node]).bool() {
            none_valid = false;
            if field(e, at, CURRENT_SOUND) == 0 {
                e.mem.set_u32(at.wrapping_add(CURRENT_SOUND), node);
            }
            if e.call(SOUND_GET_DURATION, &args![node]).u32() == 0 {
                all_ready = false;
            } else {
                let duration = e.call(SOUND_GET_DURATION, &args![node]).u32();
                total = duration.wrapping_add(total);
                count = count.wrapping_add(1);
            }
        }
        node = e.mem.u32(node.wrapping_add(0xc));
    }
    if none_valid {
        typed_call!(
            super::FAMILY,
            0x0079_8ad0,
            map_menu_stop_note_sound(e, this)
        );
        let volume = x87_quiet_bits(field(e, at, OLD_RADIO_VOLUME));
        let audio = e.mem.u32(AUDIO);
        e.call(AUDIO_SET_RADIO_VOLUME, &args![audio, volume]);
    } else if all_ready {
        e.mem.set_u8(at.wrapping_add(SOUNDS_LOADING), 0);
        let length = count.wrapping_sub(1).wrapping_mul(0x1f4).wrapping_add(total);
        e.mem.set_u32(
            at.wrapping_add(SOUNDS_LENGTH),
            x87_store32(f64::from(length as i32)),
        );
        let node = field(e, at, CURRENT_SOUND);
        e.call(SOUND_PLAY, &args![node, 0u32]);
        let time = now(e);
        e.mem.set_u32(at.wrapping_add(PLAY_START), time);
        show_note_subtitle(e, at, 0);
    }
}

/// `UpdatePlayingSounds`' play time: the time played (at most the length, `0040ebd0` in place),
/// the remaining minutes, seconds and tenths with the label, and the played fraction as the
/// completion.
fn show_play_time(e: &mut Engine, at: u32) {
    let elapsed = x87_store32(f64::from(now(e).wrapping_sub(field(e, at, PLAY_START))));
    let length = x87_quiet_bits(field(e, at, SOUNDS_LENGTH));
    let (a, b) = (
        f32::from_bits(x87_quiet_bits(elapsed)),
        f32::from_bits(length),
    );
    let played = x87_quiet_bits(if b > a { a } else { b }.to_bits());
    let remaining = x87_sub(
        x87_load32(field(e, at, SOUNDS_LENGTH)),
        x87_m32(played),
    );
    let remaining = ftol2(e, remaining);
    let label = in_place::setting_text(e, AUDIO_LABEL);
    let tenths = (remaining % 1000) / 100;
    let seconds = (remaining % 60000) / 1000;
    let minutes = remaining / 60000;
    set_play_time_text(
        e,
        at,
        &[
            PLAY_TIME_FORMAT,
            minutes as u32,
            seconds as u32,
            tenths as u32,
            label,
        ],
    );
    let fraction = x87_div(x87_load32(played), x87_m32(field(e, at, SOUNDS_LENGTH)));
    let fraction = x87_quiet_bits(x87_store32(fraction));
    let completion = trait_by_name(e, AUDIO_COMPLETION_TEXT);
    set_float(e, map_tile(e, at, 13), completion, fraction);
}

// ---- The coordinates, the wheel and the zoom --------------------------------------------------

/// `value - from` over `to - from` (each a float, the differences kept on the x87 stack), stored as
/// a float.
fn map_fraction(value: u32, from: u32, to: u32) -> u32 {
    let numerator = x87_sub(x87_load32(value), x87_m32(from));
    let denominator = x87_sub(x87_load32(to), x87_m32(from));
    x87_store32(x87_div(numerator, denominator))
}

// Typed from 0079c380
/// `MapMenu::WorldToMapCoord` (Xbox PDB): on the world map with a world space the point is adjusted
/// first (`TESWorldSpace::AdjustMapMarkerCoord` with 0); the result is the point's fraction of the
/// map rectangle (x and y, z 0.0), on the world map scaled and offset by `01075010`/`01075008`.
/// Returns `result`.
pub fn map_menu_world_to_map_coord(
    e: &mut Engine,
    this: Ptr,
    result: u32,
    x: u32,
    y: u32,
    z: u32,
    space: u32,
) -> u32 {
    let at = this.addr();
    let (x, y) = e.with_stack(0xc, |e, point| {
        let point = point.addr();
        set_point_at(e, point, [x, y, z]);
        if space != 0 && page(e, at) == WORLD_PAGE {
            e.call(ADJUST_MAP_MARKER_COORD, &args![space, point, 0u32]);
        }
        (e.mem.u32(point), e.mem.u32(point.wrapping_add(4)))
    });
    let mut map_x = map_fraction(x, field(e, at, MAP_FROM_X), field(e, at, MAP_TO_X));
    let mut map_y = map_fraction(y, field(e, at, MAP_FROM_Y), field(e, at, MAP_TO_Y));
    if page(e, at) == WORLD_PAGE {
        let scale: f64 = e.global(WORLD_SCALE_DOUBLE);
        let offset: f64 = e.global(WORLD_OFFSET_DOUBLE);
        map_x = x87_store32(x87_add(x87_mul(x87_load32(map_x), scale), offset));
        map_y = x87_store32(x87_add(x87_mul(x87_load32(map_y), scale), offset));
    }
    set_point_at(e, result, [map_x, map_y, 0]);
    result
}

// Typed from 0079c450
/// `MapMenu::MapToWorldCoord` (Xbox PDB): the inverse of `WorldToMapCoord`: on the world map the
/// map point unscaled (`01075020`/`01075018`); the world point the map rectangle at that fraction
/// (z 0.0), on the world map with a world space adjusted (`AdjustMapMarkerCoord` with 1). The
/// fourth word is not read. Returns `result`.
pub fn map_menu_map_to_world_coord(
    e: &mut Engine,
    this: Ptr,
    result: u32,
    x: u32,
    y: u32,
    _z: u32,
    space: u32,
) -> u32 {
    let at = this.addr();
    let (mut x, mut y) = (x, y);
    if page(e, at) == WORLD_PAGE {
        let scale: f64 = e.global(WORLD_INVERSE_SCALE_DOUBLE);
        let offset: f64 = e.global(WORLD_INVERSE_OFFSET_DOUBLE);
        x = x87_store32(x87_sub(x87_mul(x87_load32(x), scale), offset));
        y = x87_store32(x87_sub(x87_mul(x87_load32(y), scale), offset));
    }
    let world = |from: u32, to: u32, fraction: u32| -> u32 {
        let span = x87_sub(x87_load32(to), x87_m32(from));
        x87_store32(x87_add(x87_mul(span, x87_m32(fraction)), x87_m32(from)))
    };
    let world_x = world(field(e, at, MAP_FROM_X), field(e, at, MAP_TO_X), x);
    let world_y = world(field(e, at, MAP_FROM_Y), field(e, at, MAP_TO_Y), y);
    let point = e.with_stack(0xc, |e, point| {
        let point = point.addr();
        set_point_at(e, point, [world_x, world_y, 0]);
        if space != 0 && page(e, at) == WORLD_PAGE {
            e.call(ADJUST_MAP_MARKER_COORD, &args![space, point, 1u32]);
        }
        point_at(e, point)
    });
    set_point_at(e, result, point);
    result
}

// Typed from 0079c530
/// `MapMenu::DoWheelMove` (Xbox PDB): on the maps, a zoom by `01056528`, in when the manager's
/// wheel delta (`0070ec70`) over 120 is above 0. The two words are not read.
pub fn map_menu_do_wheel_move(e: &mut Engine, this: Ptr, _x: u32, _y: u32) {
    let at = this.addr();
    if page(e, at) != WORLD_PAGE && page(e, at) != LOCAL_PAGE {
        return;
    }
    let manager = in_place::manager(e);
    let delta = e.call(WHEEL_DELTA, &args![manager]).i32();
    let zero: f64 = e.global(ZERO_DOUBLE);
    let zoom_in = above(f64::from(delta.wrapping_div(0x78)), zero);
    let factor = float_constant(e, WHEEL_ZOOM_FLOAT);
    typed_call!(
        super::FAMILY,
        0x0079_c5a0,
        map_menu_zoom_map(e, this, u8::from(zoom_in), f32::from_bits(factor))
    );
}

/// `ZoomMap`'s clamp of the magnification (at most the page's largest, `0040ebd0` in place, at
/// least its smallest, `00404010`) and the three traits that follow it: each `low + (high - low) *
/// fraction`, the fraction the magnification's of its range. Returns the clamped magnification.
fn zoom_traits(
    e: &mut Engine,
    map: u32,
    magnification: u32,
    limits: (u32, u32),
    traits: &[(i32, u32, u32); 3],
) -> u32 {
    let (min_setting, max_setting) = limits;
    let low = setting_float(e, min_setting);
    let high = setting_float(e, max_setting);
    let (a, b) = (
        f32::from_bits(x87_quiet_bits(magnification)),
        f32::from_bits(high),
    );
    let clamped = x87_quiet_bits(if b > a { a } else { b }.to_bits());
    let clamped = typed_call!(
        draw_f::FAMILY,
        0x0040_4010,
        draw_f::helpers::float_max(e, f32::from_bits(clamped), f32::from_bits(low))
    );
    let magnification = x87_store32(f64::from(clamped));
    let low = setting_float(e, min_setting);
    let above_low = x87_sub(x87_load32(magnification), x87_m32(low));
    let high = setting_float(e, max_setting);
    let span = x87_sub(x87_load32(high), x87_m32(low));
    let fraction = x87_store32(x87_div(above_low, span));
    for &(id, from, to) in traits {
        let start = x87_load32(setting_float(e, from));
        let end = x87_load32(setting_float(e, to));
        let value = x87_sub(end, x87_load32(setting_float(e, from)));
        let value = x87_add(x87_mul(value, x87_m32(fraction)), start);
        set_float(e, map, id, x87_quiet_bits(x87_store32(value)));
    }
    magnification
}

/// `ZoomMap`'s local map cells (the local map page): every cell of the map tile's node (a grid of
/// `n * n`, `n` the grid setting's value) scaled to the magnification and placed at its column and
/// row (cell size 0x400, 0x800 when `005f36f0` says so, times the magnification), with a zero
/// rotation, its geometry marked changed. The exe's quotient of the cell size it stores in a local
/// it never reads is left out.
fn zoom_cell_grid(e: &mut Engine, map: u32, magnification: u32) {
    let object = in_place::tile_node(e, map);
    let side = e.mem.u32(setting_value(GRID_SETTING));
    let two: f64 = e.global(TWO_DOUBLE);
    let mut index = 0u32;
    while object != 0 && index < side.wrapping_mul(side) {
        let cell = typed_call!(
            dispatch::FAMILY,
            0x0043_b4a0,
            dispatch::scene::child(e, Ptr::new(object), index)
        )
        .addr();
        if cell != 0 {
            let test = e.mem.u32(INTERIOR_TEST_OBJECT);
            let size: i32 = if e.call(CELL_INTERIOR_TEST, &args![test]).u32() != 0 {
                0x800
            } else {
                0x400
            };
            let cell_size = x87_store32(x87_mul(f64::from(size), x87_m32(magnification)));
            e.call(NODE_SET_SCALE, &args![cell, x87_quiet_bits(magnification)]);
            let (row, column) = (index / side, index % side);
            let half = x87_div(x87_load32(cell_size), two);
            let y = x87_add(x87_mul(f64::from(row), x87_m32(cell_size)), half);
            let y = x87_sub(y, x87_mul(f64::from(side), x87_m32(cell_size)));
            let y = x87_quiet_bits(x87_store32(y));
            let half = x87_div(x87_load32(cell_size), two);
            let x = x87_add(x87_mul(f64::from(column), x87_m32(cell_size)), half);
            let x = x87_quiet_bits(x87_store32(x));
            e.call(NODE_SET_TRANSLATE, &args![cell, x, 0u32, y]);
            e.with_stack(0x24, |e, matrix| {
                e.call(MATRIX_SET, &args![matrix, 0u32, 0u32, 0u32]);
                typed_call!(
                    scene::FAMILY,
                    0x00a5_9c60,
                    scene::walk::update(e, Ptr::new(cell), matrix.addr())
                );
            });
            let data = e.call(NODE_GEOMETRY_DATA, &args![cell]).u32();
            e.call(GEOMETRY_SET_CONSISTENCY, &args![data, 0x4000u32]);
        }
        index = index.wrapping_add(1);
    }
}

// Typed from 0079c5a0
/// `MapMenu::ZoomMap` (Xbox PDB): with a map tile (and on the local map only while the grid is not
/// being made, `007a1750`), the magnification multiplied by `factor` (in) or divided by it, clamped
/// to the page's limits with the traits `0x1004` to `0x1006` following; the point under the map's
/// centre kept in place (`0xfa1`/`0xfa2` from the owner's half size, the scaled point and
/// `0xfee`/`0xfef`); on the local map the cells follow. When the magnification changed the
/// Pip-Boy's knob turns by `(factor - 1.0) / 01075028` steps, the other way zooming in. Then
/// `UpdateCompanionMarkers`.
pub fn map_menu_zoom_map(e: &mut Engine, this: Ptr, zoom_in: u8, factor: f32) {
    let at = this.addr();
    if field(e, at, MAP_TILE) == 0 {
        return;
    }
    if page(e, at) == LOCAL_PAGE && typed_call!(super::FAMILY, 0x007a_1750, map_menu_p2::fn_007a1750(e))
    {
        return;
    }
    let factor = factor.to_bits();
    let scale = if zoom_in != 0 {
        x87_load32(factor)
    } else {
        x87_div(1.0, x87_load32(factor))
    };
    let id = static_id(e, TRAIT_MAGNIFICATION);
    let current = get_float64(e, field(e, at, MAP_TILE), id);
    let mut magnification = x87_store32(x87_mul(current, scale));
    let two: f64 = e.global(TWO_DOUBLE);
    let (changed, point_x, point_y) = e.with_stack(8, |e, point| {
        let point = point.addr();
        let height = get_float64(e, owner(e, field(e, at, MAP_TILE)), 0xfb0);
        let top = get_float64(e, field(e, at, MAP_TILE), 0xfa2);
        let y = x87_quiet_bits(x87_store32(x87_sub(x87_div(height, two), top)));
        let width = get_float64(e, owner(e, field(e, at, MAP_TILE)), 0xfb1);
        let left = get_float64(e, field(e, at, MAP_TILE), 0xfa1);
        let x = x87_quiet_bits(x87_store32(x87_sub(x87_div(width, two), left)));
        point2_new(e, point, x, y);
        let map = field(e, at, MAP_TILE);
        magnification = if page(e, at) == LOCAL_PAGE {
            zoom_traits(e, map, magnification, LOCAL_ZOOM_LIMITS, &LOCAL_ZOOM_TRAITS)
        } else {
            zoom_traits(e, map, magnification, WORLD_ZOOM_LIMITS, &WORLD_ZOOM_TRAITS)
        };
        let id = static_id(e, TRAIT_MAGNIFICATION);
        let current = get_float64(e, field(e, at, MAP_TILE), id);
        let changed = !equal(x87_load32(magnification), current);
        let size = get_float64(e, field(e, at, MAP_TILE), 0xfcd);
        let size = x87_mul(size, x87_m32(magnification));
        let width = get_float64(e, field(e, at, MAP_TILE), 0xfb1);
        let ratio = x87_quiet_bits(x87_store32(x87_div(size, width)));
        e.call(POINT2_SCALE, &args![point, ratio]);
        let id = static_id(e, TRAIT_MAGNIFICATION);
        set_float(
            e,
            field(e, at, MAP_TILE),
            id,
            x87_quiet_bits(magnification),
        );
        if page(e, at) == LOCAL_PAGE {
            let map = field(e, at, MAP_TILE);
            zoom_cell_grid(e, map, magnification);
        }
        (
            changed,
            e.mem.u32(point),
            e.mem.u32(point.wrapping_add(4)),
        )
    });
    for (size_id, along, offset_id, target) in [
        (0xfb1, point_x, 0xfee, 0xfa1),
        (0xfb0, point_y, 0xfef, 0xfa2),
    ] {
        let size = get_float64(e, owner(e, field(e, at, MAP_TILE)), size_id);
        let centre = x87_sub(x87_div(size, two), x87_m32(along));
        let offset = get_float64(e, field(e, at, MAP_TILE), offset_id);
        let value = x87_quiet_bits(x87_store32(x87_sub(centre, offset)));
        set_float_flag(e, field(e, at, MAP_TILE), target, value, 0);
    }
    if changed {
        let one: f64 = e.global(ONE_DOUBLE);
        let divisor: f64 = e.global(ZOOM_KNOB_DIVISOR_DOUBLE);
        let steps = x87_store32(x87_div(x87_sub(x87_load32(factor), one), divisor));
        let step = setting_float(e, KNOB_STEP_SETTING);
        // `FCHS` flips the sign bit.
        let step = if zoom_in != 0 {
            step ^ 0x8000_0000
        } else {
            step
        };
        let turn = x87_mul(x87_load32(step), x87_m32(steps));
        let time = x87_load32(setting_float(e, KNOB_TIME_SETTING));
        let time = x87_quiet_bits(x87_store32(x87_add(time, time)));
        let turn = x87_quiet_bits(x87_store32(turn));
        move_knob(e, turn, time);
    }
    typed_call!(
        super::FAMILY,
        0x0079_e470,
        map_menu_update_companion_markers(e, this)
    );
}

// ---- The world and local maps -----------------------------------------------------------------

/// The world map data of `space` (`TESWorldSpace::GetWorldMapData`), 0 without a space.
fn world_map_data(e: &mut Engine, space: u32) -> u32 {
    if space == 0 {
        return 0;
    }
    e.call(WORLD_GET_WORLD_MAP_DATA, &args![space]).u32()
}

/// Whether the root space and its world map data's usable size are set.
fn world_map_ready(e: &Engine, root: u32, data: u32) -> bool {
    root != 0 && data != 0 && e.mem.u32(data) != 0 && e.mem.u32(data.wrapping_add(4)) != 0
}

/// A map corner as an `NiPoint2` built in a stack slot (`00452dc0` in place) from two float bit
/// patterns: its two words.
fn map_corner_bits(e: &mut Engine, x: u32, y: u32) -> (u32, u32) {
    e.with_stack(8, |e, corner| {
        let corner = corner.addr();
        point2_new(e, corner, x, y);
        (e.mem.u32(corner), e.mem.u32(corner.wrapping_add(4)))
    })
}

/// [`map_corner_bits`] from two whole world coordinates.
fn map_corner(e: &mut Engine, x: i32, y: i32) -> (u32, u32) {
    map_corner_bits(e, x87_store32(f64::from(x)), x87_store32(f64::from(y)))
}

/// A 16-bit cell coordinate of the world map data (`MOVSX; SHL 0xC`).
fn map_cell(e: &Engine, data: u32, offset: u32) -> i32 {
    i32::from(e.mem.u16(data.wrapping_add(offset)) as i16) << 0xc
}

// Typed from 0079cdb0
/// `MapMenu::UpdateWorldMap` (Xbox PDB): without a usable root space map, form 0x3c cast to a
/// world space, then the first world space of the challenges' owner's list (each marking the root
/// changed); the root becomes the map space, the north rotation cell is cleared. With map data: a
/// cleared texture or a changed root puts the root's texture and usable size into tile 4 (`0xfcc`,
/// `0xfcd`, `0xfce`); the map zooms by 1.0; the map rectangle comes from the data's cells (4096
/// units each); unless tile 4 has a child 0x1a, every visible marker of the root gets a tile, and
/// the built list is emptied and deleted. Then `AddMapIcons` (by address: extenders hook it).
pub fn map_menu_update_world_map(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let mut data = world_map_data(e, field(e, at, ROOT_SPACE));
    if !world_map_ready(e, field(e, at, ROOT_SPACE), data) {
        let form = typed_call!(
            forms::FAMILY,
            0x0048_39c0,
            forms::tesform::form_by_id(e, 0x3c)
        );
        let space = e
            .call(
                RT_DYNAMIC_CAST,
                &args![form, 0u32, SOURCE_TYPE, TARGET_TYPE, 0u32],
            )
            .u32();
        e.mem.set_u32(at.wrapping_add(ROOT_SPACE), space);
        data = world_map_data(e, field(e, at, ROOT_SPACE));
        e.mem.set_u8(at.wrapping_add(ROOT_CHANGED), 1);
    }
    if !world_map_ready(e, field(e, at, ROOT_SPACE), data) {
        let first = item(e, e.mem.u32(OWNER).wrapping_add(0x10));
        e.mem.set_u32(at.wrapping_add(ROOT_SPACE), first);
        data = world_map_data(e, field(e, at, ROOT_SPACE));
        e.mem.set_u8(at.wrapping_add(ROOT_CHANGED), 1);
    }
    let root = field(e, at, ROOT_SPACE);
    e.mem.set_u32(at.wrapping_add(MAP_SPACE), root);
    e.mem.set_u32(at.wrapping_add(NORTH_CELL), 0);
    if field(e, at, ROOT_SPACE) == 0 || data == 0 {
        return;
    }
    if e.mem.u8(TEXTURE_CLEARED) != 0 || e.mem.u8(at.wrapping_add(ROOT_CHANGED)) != 0 {
        let texture = e.call(FORM_TEXT, &args![field(e, at, ROOT_SPACE)]).u32();
        set_string(e, map_tile(e, at, 4), 0xfcc, texture);
        let width = e.mem.u32(data);
        let tile = map_tile(e, at, 4);
        e.call(TILE_SET_COUNT, &args![tile, 0xfcdu32, width]);
        let height = e.mem.u32(data.wrapping_add(4));
        let tile = map_tile(e, at, 4);
        e.call(TILE_SET_COUNT, &args![tile, 0xfceu32, height]);
        e.mem.set_u8(TEXTURE_CLEARED, 0);
        e.mem.set_u8(at.wrapping_add(ROOT_CHANGED), 0);
    }
    typed_call!(
        super::FAMILY,
        0x0079_c5a0,
        map_menu_zoom_map(e, this, 1, 1.0)
    );
    let y = map_cell(e, data, 0xa).wrapping_add(0x1000);
    let (x, y) = map_corner(e, map_cell(e, data, 8), y);
    e.mem.set_u32(at.wrapping_add(MAP_FROM_X), x);
    e.mem.set_u32(at.wrapping_add(MAP_FROM_Y), y);
    let y = map_cell(e, data, 0xe);
    let (x, y) = map_corner(e, map_cell(e, data, 0xc).wrapping_add(0x1000), y);
    e.mem.set_u32(at.wrapping_add(MAP_TO_X), x);
    e.mem.set_u32(at.wrapping_add(MAP_TO_Y), y);
    let tile = map_tile(e, at, 4);
    if e.call(TILE_GET_CHILD_BY_ID, &args![tile, 0x1au32]).u32() == 0 {
        let root = field(e, at, ROOT_SPACE);
        let list = e
            .call(WORLD_BUILD_MAP_MARKER_LIST, &args![root, 0u32])
            .u32();
        e.mem.set_u32(MARKER_INDEX, 0);
        let mut node = list;
        while node != 0 && item(e, node) != 0 {
            add_world_map_marker(e, at, item(e, node));
            node = next(e, node);
        }
        e.call(LIST_REMOVE_ALL, &args![list]);
        if list != 0 {
            e.call(NODE_DELETE, &args![list, 1u32]);
        }
    }
    e.call(ADD_MAP_ICONS, &args![at]);
}

/// `MapMenu::AddMapIcons` (translated: extenders hook it).
const ADD_MAP_ICONS: u32 = 0x0079_dbb0;

/// A marker's map marker data (`TESObjectREFR::GetMapMarkerData`, by address).
#[inline]
fn marker_data(e: &mut Engine, marker: u32) -> u32 {
    e.call(REFR_GET_MAP_MARKER_DATA, &args![marker]).u32()
}

/// A marker data's name (`00438eb0`, by address).
#[inline]
fn marker_name(e: &mut Engine, marker: u32) -> u32 {
    let data = marker_data(e, marker);
    e.call(MARKER_NAME, &args![data]).u32()
}

/// One marker of `UpdateWorldMap`'s list: a visible marker (not of flag 0x800, `00440da0` in place)
/// whose top world space is the root gets a `"MapMarkerTemplate"` tile under tile 4 with its name
/// (with its reputation's title, `"%s\n(%s)"`, when it has a reputation) as `"_LocationName"`,
/// `00707e10` with the name, kind 0x1a, its map position, its icon (by kind from `011a0404`) or the
/// undiscovered icon with `0x1004` 0, the running index as `"_MarkerIndex"`, and joins the marker
/// list (+0xd8).
fn add_world_map_marker(e: &mut Engine, at: u32, marker: u32) {
    let [x, y, z] = position(e, marker);
    let mut space = e.call(REFR_GET_WORLD_SPACE, &args![marker]).u32();
    while space != 0 && world_parent(e, space) != 0 {
        space = world_parent(e, space);
    }
    let data = marker_data(e, marker);
    if !e.call(MARKER_GET_VISIBLE, &args![data]).bool() || e.mem.u32(marker.wrapping_add(8)) & 0x800 != 0
    {
        return;
    }
    if space != field(e, at, ROOT_SPACE) {
        return;
    }
    let menu = singleton(e);
    let tile = e
        .call(
            MENU_RENDER_TEMPLATE,
            &args![menu, map_tile(e, at, 4), MAP_MARKER_TEMPLATE, 0u32],
        )
        .u32();
    if tile == 0 {
        return;
    }
    let (map_x, map_y) = e.with_stack(0xc, |e, map_point| {
        let map_point = map_point.addr();
        let space = e.call(REFR_GET_WORLD_SPACE, &args![marker]).u32();
        typed_call!(
            super::FAMILY,
            0x0079_c380,
            map_menu_world_to_map_coord(e, Ptr::new(at), map_point, x, y, z, space)
        );
        (e.mem.u32(map_point), e.mem.u32(map_point.wrapping_add(4)))
    });
    // The marker data's reputation (`0044edb0`, in place: its +0x10).
    let data = marker_data(e, marker);
    let reputation = e.mem.u32(data.wrapping_add(0x10));
    if reputation != 0 {
        let title = e.call(REPUTATION_GET_TITLE, &args![reputation]).u32();
        let name = marker_name(e, marker);
        e.with_stack(0x100, |e, text| {
            e.call(
                BUFFER_FORMAT,
                &args![text, 0x100u32, MARKER_REPUTATION_FORMAT, name, title],
            );
            let name_id = user_trait(e, LOCATION_NAME_TEXT);
            set_string(e, tile, name_id, text.addr());
        });
    } else {
        let name = marker_name(e, marker);
        let name_id = user_trait(e, LOCATION_NAME_TEXT);
        set_string(e, tile, name_id, name);
    }
    let name = marker_name(e, marker);
    typed_call!(
        super::FAMILY,
        0x0070_7e10,
        interface_cpp_p2::fn_00707e10(e, tile, name)
    );
    set_int(e, tile, 0xfaa, 0x1a);
    let id = static_id(e, TRAIT_X);
    set_float(e, tile, id, x87_quiet_bits(map_x));
    let id = static_id(e, TRAIT_Y);
    set_float(e, tile, id, x87_quiet_bits(map_y));
    let data = marker_data(e, marker);
    if e.call(MARKER_GET_TRAVEL_LOC, &args![data]).bool() {
        let data = marker_data(e, marker);
        let kind = e.call(MARKER_ICON_KIND, &args![data]).u32();
        let icon = e
            .mem
            .u32(MARKER_ICONS.wrapping_add(kind.wrapping_mul(4)));
        set_string(e, tile, 0xfcc, icon);
    } else {
        set_string(e, tile, 0xfcc, UNDISCOVERED_ICON_PATH);
        set_int(e, tile, 0x1004, 0);
    }
    let index = e.mem.u32(MARKER_INDEX);
    let index_id = user_trait(e, MARKER_INDEX_TEXT);
    set_int(e, tile, index_id, index);
    list_append(e, at.wrapping_add(MARKERS), marker);
    let index = e.mem.u32(MARKER_INDEX).wrapping_add(1);
    e.mem.set_u32(MARKER_INDEX, index);
}

/// A word appended to a list (`00905820`) through a stack slot holding it.
fn list_append(e: &mut Engine, list: u32, word: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), word);
        e.call(LIST_ADD_TAIL, &args![list, slot]);
    });
}

/// A point (`00416870` in place: x, y and 0.0) from whole coordinates stored as floats, built in a
/// stack slot: its three words.
fn set_point(e: &mut Engine, x: i32, y: i32) -> [u32; 3] {
    e.with_stack(0xc, |e, point| {
        let point = point.addr();
        let (x, y) = (x87_store32(f64::from(x)), x87_store32(f64::from(y)));
        set_point_at(e, point, [x, y, 0]);
        point_at(e, point)
    })
}

/// The local map grid's side (the grid setting's value).
fn local_grid_side(e: &Engine) -> i32 {
    e.mem.i32(setting_value(GRID_SETTING))
}

/// The grid cell of a coordinate: `(00406d90(value) - offset) >> 12` less half the grid side
/// (arithmetic shifts).
fn local_grid_cell(e: &mut Engine, value: u32, offset: i32) -> i32 {
    let whole = e
        .call(FLOAT_TO_INT, &args![x87_quiet_bits(value)])
        .i32();
    (whole.wrapping_sub(offset) >> 0xc).wrapping_sub(local_grid_side(e) >> 1)
}

// Typed from 0079d410
/// `MapMenu::UpdateLocalMap` (Xbox PDB): around the player's position (in an interior the north
/// rotation cell is the player's cell, the position rotated to north and the cells offset by
/// 0x800), the map rectangle covers the grid of cells centred on the player's; the map zooms by
/// 1.0; the map space is the world space, else the cell. With a map space: when tile 2's map is
/// ready, every door marker of the list at +0xe0 visible through the fog and inside the map gets a
/// tile; then `AddMapIcons` (by address). Tiles 2 and 3 get `0xfa3` 1.
pub fn map_menu_update_local_map(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let space = e
        .call(REFR_GET_WORLD_SPACE, &args![e.mem.u32(PLAYER)])
        .u32();
    let cell = cell_of(e, e.mem.u32(PLAYER));
    let player = e.mem.u32(PLAYER);
    let position = position(e, player);
    let interior = e
        .call(REFR_GET_INTERIOR, &args![e.mem.u32(PLAYER)])
        .bool();
    e.mem
        .set_u32(at.wrapping_add(NORTH_CELL), if interior { cell } else { 0 });
    let one_cell: f64 = e.global(CELL_SIZE_DOUBLE);
    let (first, second) = if field(e, at, NORTH_CELL) != 0 {
        let rotated = e.with_stack(0x18, |e, points| {
            let original = points.addr();
            let rotated = original.wrapping_add(0xc);
            set_point_at(e, original, position);
            set_point_at(e, rotated, position);
            let north = field(e, at, NORTH_CELL);
            e.call(
                ADJUST_FOR_NORTH_ROTATION,
                &args![north, original, rotated, 1u32],
            );
            [e.mem.u32(rotated), e.mem.u32(rotated.wrapping_add(4))]
        });
        let column = local_grid_cell(e, rotated[0], 0x800);
        let row = local_grid_cell(e, rotated[1], 0x800);
        let side = local_grid_side(e);
        let top = set_point(
            e,
            (column << 0xc).wrapping_add(0x800),
            (row.wrapping_add(side).wrapping_sub(1) << 0xc).wrapping_add(0x800),
        );
        let side = local_grid_side(e);
        let bottom = set_point(
            e,
            (column.wrapping_add(side).wrapping_sub(1) << 0xc).wrapping_add(0x800),
            (row << 0xc).wrapping_add(0x800),
        );
        (top, bottom)
    } else {
        let column = local_grid_cell(e, position[0], 0);
        let row = local_grid_cell(e, position[1], 0);
        let side = local_grid_side(e);
        let top = set_point(e, column << 0xc, row.wrapping_add(side).wrapping_sub(1) << 0xc);
        let side = local_grid_side(e);
        let bottom = set_point(e, column.wrapping_add(side).wrapping_sub(1) << 0xc, row << 0xc);
        (top, bottom)
    };
    let y = x87_quiet_bits(x87_store32(x87_add(x87_load32(first[1]), one_cell)));
    let (x, y) = map_corner_bits(e, x87_quiet_bits(first[0]), y);
    e.mem.set_u32(at.wrapping_add(MAP_FROM_X), x);
    e.mem.set_u32(at.wrapping_add(MAP_FROM_Y), y);
    let x = x87_quiet_bits(x87_store32(x87_add(x87_load32(second[0]), one_cell)));
    let (x, y) = map_corner_bits(e, x, x87_quiet_bits(second[1]));
    e.mem.set_u32(at.wrapping_add(MAP_TO_X), x);
    e.mem.set_u32(at.wrapping_add(MAP_TO_Y), y);
    typed_call!(
        super::FAMILY,
        0x0079_c5a0,
        map_menu_zoom_map(e, this, 1, 1.0)
    );
    e.mem.set_u32(
        at.wrapping_add(MAP_SPACE),
        if space != 0 { space } else { cell },
    );
    if field(e, at, MAP_SPACE) != 0 {
        let tile = map_tile(e, at, 2);
        if e.call(LOCAL_MAP_READY, &args![tile.wrapping_add(4)]).bool() {
            let mut node = at.wrapping_add(DOOR_MARKERS);
            while node != 0 && item(e, node) != 0 {
                add_door_marker(e, at, item(e, node));
                node = next(e, node);
            }
        }
        e.call(ADD_MAP_ICONS, &args![at]);
    }
    set_int(e, map_tile(e, at, 2), 0xfa3, 1);
    set_int(e, map_tile(e, at, 3), 0xfa3, 1);
}

/// One reference of `UpdateLocalMap`'s door list: a door (its base of form type 0x1c) not skipped
/// (`00477ba0`, `0057b4b0`, its base's `00517fd0`), its position (north-rotated in a rotated cell)
/// inside the map and with an alpha above 0 (255.0, times the fog's seen value over `0101db80`
/// with the fog of war on), gets a `"MapMarkerTemplate"` tile under tile 2 (kind 0x1a, its
/// position, the alpha `0xfa9`, the door icon, and the teleport cell's map name as
/// `"_LocationName"`).
fn add_door_marker(e: &mut Engine, at: u32, reference: u32) {
    let object = base_object(e, reference);
    let door = if form_type(e, object) == 0x1c {
        base_object(e, reference)
    } else {
        0
    };
    let position = position(e, reference);
    if e.call(REFERENCE_SKIPPED_A, &args![reference]).bool()
        || e.call(REFERENCE_SKIPPED_B, &args![reference]).bool()
        || door == 0
        || e.call(DOOR_SKIPPED, &args![door]).bool()
    {
        return;
    }
    e.with_stack(0x18, |e, frame| {
        // The locals: the position (+0) and the map point (+0xc).
        let point = frame.addr();
        let map_point = point.wrapping_add(0xc);
        set_point_at(e, point, position);
        let north = field(e, at, NORTH_CELL);
        if north != 0 {
            e.call(ADJUST_FOR_NORTH_ROTATION, &args![north, point, point, 1u32]);
        }
        let mut alpha = float_constant(e, FULL_ALPHA_FLOAT);
        let [x, y, z] = point_at(e, point);
        typed_call!(
            super::FAMILY,
            0x0079_c380,
            map_menu_world_to_map_coord(e, Ptr::new(at), map_point, x, y, z, 0)
        );
        if e.mem.u8(at.wrapping_add(FOG_OF_WAR)) != 0 {
            let seen = e.call(CELL_GET_SEEN_VALUE, &args![point]).i32();
            let divisor: f64 = e.global(FOG_DIVISOR_DOUBLE);
            alpha = x87_store32(x87_mul(
                x87_div(f64::from(seen), divisor),
                x87_m32(alpha),
            ));
        }
        let zero: f64 = e.global(ZERO_DOUBLE);
        let one: f64 = e.global(ONE_DOUBLE);
        // `TEST AH,1; JNZ` skips below 0.0 (or unordered); `TEST AH,0x41; JP` skips above 1.0 (or
        // unordered).
        let inside = |value: f64| {
            !matches!(
                value.partial_cmp(&zero),
                Some(std::cmp::Ordering::Less) | None
            ) && !matches!(
                value.partial_cmp(&one),
                Some(std::cmp::Ordering::Greater) | None
            )
        };
        if !above(x87_load32(alpha), zero)
            || !inside(x87_load32(e.mem.u32(map_point)))
            || !inside(x87_load32(e.mem.u32(map_point.wrapping_add(4))))
        {
            return;
        }
        let menu = singleton(e);
        let tile = e
            .call(
                MENU_RENDER_TEMPLATE,
                &args![menu, map_tile(e, at, 2), MAP_MARKER_TEMPLATE, 0u32],
            )
            .u32();
        let teleport = e.call(REFERENCE_TELEPORT, &args![reference]).u32();
        set_int(e, tile, 0xfaa, 0x1a);
        let id = static_id(e, TRAIT_X);
        set_float(e, tile, id, x87_quiet_bits(e.mem.u32(map_point)));
        let id = static_id(e, TRAIT_Y);
        set_float(
            e,
            tile,
            id,
            x87_quiet_bits(e.mem.u32(map_point.wrapping_add(4))),
        );
        set_float(e, tile, 0xfa9, x87_quiet_bits(alpha));
        set_string(e, tile, 0xfcc, DOOR_ICON_PATH);
        if teleport != 0 {
            e.with_stack(8, |e, name| {
                let name = name.addr();
                in_place::empty_string(e, name);
                e.call(TELEPORT_GET_CELL_MAP_NAME, &args![teleport, name]);
                let text = e.mem.u32(name);
                let name_id = user_trait(e, LOCATION_NAME_TEXT);
                set_string(e, tile, name_id, text);
                e.call(STRING_DESTRUCT, &args![name]);
            });
        }
    });
}

// ---- The quest markers and the companions' markers --------------------------------------------

/// Every child with id `id` of the map tile removed (the child list's `0057c730`), released
/// (`Tile::Release`) and deleted (its deleting destructor with 1).
fn remove_map_children(e: &mut Engine, at: u32, id: u32) {
    e.with_stack(4, |e, slot| loop {
        let map = field(e, at, MAP_TILE);
        let child = e.call(TILE_GET_CHILD_BY_ID, &args![map, id]).u32();
        e.mem.set_u32(slot.addr(), child);
        if child == 0 {
            break;
        }
        remove_child(e, at, slot.addr());
    });
}

/// The map tile's child in `slot` removed, released and deleted.
fn remove_child(e: &mut Engine, at: u32, slot: u32) {
    let list = field(e, at, MAP_TILE).wrapping_add(4);
    e.call(CHILD_LIST_REMOVE, &args![list, slot]);
    let child = e.mem.u32(slot);
    e.call(TILE_RELEASE, &args![child]);
    let child = e.mem.u32(slot);
    if child != 0 {
        e.vcall(child, DELETE_SLOT, &args![1u32]);
    }
}

/// A tracked reference's map position: its position (north-rotated in a rotated cell) through
/// `WorldToMapCoord` with its world space; x and y.
fn tracked_map_position(e: &mut Engine, at: u32, reference: u32) -> (u32, u32) {
    let position = position(e, reference);
    e.with_stack(0x18, |e, frame| {
        let world = frame.addr();
        let result = world.wrapping_add(0xc);
        set_point_at(e, world, position);
        let north = field(e, at, NORTH_CELL);
        if north != 0 {
            e.call(ADJUST_FOR_NORTH_ROTATION, &args![north, world, world, 1u32]);
        }
        let space = e.call(REFR_GET_WORLD_SPACE, &args![reference]).u32();
        let [x, y, z] = point_at(e, world);
        let made = typed_call!(
            super::FAMILY,
            0x0079_c380,
            map_menu_world_to_map_coord(e, Ptr::new(at), result, x, y, z, space)
        );
        (e.mem.u32(made), e.mem.u32(made.wrapping_add(4)))
    })
}

/// A marker tile from the world or local quest marker template under the map tile, at a map
/// position.
fn quest_marker_tile(e: &mut Engine, at: u32, point: (u32, u32)) -> u32 {
    let template = if page(e, at) == WORLD_PAGE {
        WORLD_QUEST_MARKER_TEMPLATE
    } else {
        LOCAL_QUEST_MARKER_TEMPLATE
    };
    let menu = singleton(e);
    let tile = e
        .call(
            MENU_RENDER_TEMPLATE,
            &args![menu, field(e, at, MAP_TILE), template, 0u32],
        )
        .u32();
    let id = static_id(e, TRAIT_X);
    set_float(e, tile, id, x87_quiet_bits(point.0));
    let id = static_id(e, TRAIT_Y);
    set_float(e, tile, id, x87_quiet_bits(point.1));
    tile
}

/// The player icon of the page: tile 5 on the world map, tile 3 on the local map.
fn player_icon(e: &Engine, at: u32) -> u32 {
    if page(e, at) == WORLD_PAGE {
        map_tile(e, at, 5)
    } else {
        map_tile(e, at, 3)
    }
}

/// A tile's value `id` made to follow another tile's trait (action `0x7d0`), cleared first and
/// calculated after.
fn follow_trait(e: &mut Engine, tile: u32, id: i32, source: u32) {
    let value = get_value(e, tile, id);
    clear_actions(e, value);
    e.call(
        VALUE_ADD_ACTION_TILE,
        &args![value, 0x7d0u32, source, id as u32],
    );
    calculate(e, value);
}

/// `MapMenu::GetWorldMapTrackingRef` (this file's).
#[inline]
fn tracking_ref(e: &mut Engine, at: u32, reference: u32, path: u32) -> u32 {
    typed_call!(
        super::FAMILY,
        0x0079_f7e0,
        map_menu_get_world_map_tracking_ref(e, Ptr::new(at), reference, path)
    )
}

/// `MapMenu::CreateWaypointPath` (this file's).
#[inline]
fn waypoint_path(e: &mut Engine, at: u32, from: u32, to: u32, path: u32, id: u32) {
    typed_call!(
        super::FAMILY,
        0x0079_f8d0,
        map_menu_create_waypoint_path(e, Ptr::new(at), from, to, path, id)
    );
}

// Typed from 0079e0a0
/// `MapMenu::AddQuestMarkers` (Xbox PDB): the map tile's children 0x1b and 0x1d removed; then while
/// the quest markers setting is on, for each of the player's current targets: the tracked
/// reference is on the world map the target's `GetWorldMapTrackingRef` (and a found one gets a
/// waypoint path from the map's player reference, id 0x1d), else `005cbb70`; without one, the
/// target's reference on the world map outside interiors or on the local map in the player's
/// cell. A tracked reference gets a quest marker tile with the marker icon, kind 0x1b and its fade
/// (`0xff5`) following the player icon's.
pub fn map_menu_add_quest_markers(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    remove_map_children(e, at, 0x1b);
    remove_map_children(e, at, 0x1d);
    let player = e.mem.u32(PLAYER);
    let mut node = e
        .call(PLAYER_GET_CURRENT_TARGET_LIST, &args![player])
        .u32();
    while e.mem.u8(setting_value(QUEST_MARKERS_SETTING)) != 0 && node != 0 && item(e, node) != 0 {
        let target = item(e, node);
        let reference = e
            .call(QUEST_TARGET_GET_REFERENCE, &args![target, 1u32])
            .u32();
        let mut tracked = if page(e, at) == WORLD_PAGE {
            let list = item(e, node).wrapping_add(0x10);
            tracking_ref(e, at, reference, list)
        } else {
            let target = item(e, node);
            e.call(TARGET_LOCAL_REFERENCE, &args![target]).u32()
        };
        if tracked != 0 {
            if page(e, at) == WORLD_PAGE {
                let list = item(e, node).wrapping_add(0x10);
                let from = field(e, at, MAP_PLAYER_REF);
                waypoint_path(e, at, from, tracked, list, 0x1d);
            }
        } else if reference != 0 {
            let take = if page(e, at) == WORLD_PAGE {
                !e.call(REFR_GET_INTERIOR, &args![reference]).bool()
            } else {
                page(e, at) == LOCAL_PAGE && in_player_cell(e, reference)
            };
            if take {
                let target = item(e, node);
                tracked = e
                    .call(QUEST_TARGET_GET_REFERENCE, &args![target, 1u32])
                    .u32();
            }
        }
        if tracked != 0 {
            let point = tracked_map_position(e, at, tracked);
            let tile = quest_marker_tile(e, at, point);
            set_string(e, tile, 0xfcc, QUEST_MARKER_ICON_PATH);
            set_int(e, tile, 0xfaa, 0x1b);
            let value = get_value(e, tile, 0xff5);
            clear_actions(e, value);
            let icon = player_icon(e, at);
            e.call(
                VALUE_ADD_ACTION_TILE,
                &args![value, 0x7d0u32, icon, 0xff5u32],
            );
            calculate(e, value);
        }
        node = next(e, node);
    }
}

/// Whether `reference` shares the player's cell (`008d6f30` in place: each one's +0x40).
fn in_player_cell(e: &Engine, reference: u32) -> bool {
    cell_of(e, reference) == cell_of(e, e.mem.u32(PLAYER))
}

/// The companion's tracked reference: on the world map the path to it built
/// (`PlayerCharacter::BuildPathToTarget`) and its `GetWorldMapTrackingRef`, else the companion.
fn companion_tracked(e: &mut Engine, at: u32, companion: u32, path: u32) -> u32 {
    if page(e, at) == WORLD_PAGE {
        let player = e.mem.u32(PLAYER);
        e.call(
            PLAYER_BUILD_PATH_TO_TARGET,
            &args![player, companion, path, 1u32],
        );
    }
    if page(e, at) == WORLD_PAGE {
        tracking_ref(e, at, companion, path)
    } else {
        companion
    }
}

/// The label's map position of a tracked companion: raised by `0101ecb8` on the world map, by
/// 128.0 otherwise (the marker's own position, north-rotated, is asked first as the exe does);
/// x and y.
fn label_position(e: &mut Engine, at: u32, tracked: u32) -> (u32, u32) {
    let position = position(e, tracked);
    let offset: f64 = if page(e, at) == WORLD_PAGE {
        e.global(WORLD_LABEL_OFFSET_DOUBLE)
    } else {
        e.global(HALF_ALPHA_DOUBLE)
    };
    let label_y = x87_store32(x87_sub(x87_load32(position[1]), offset));
    e.with_stack(0x24, |e, frame| {
        let world = frame.addr();
        let marker = world.wrapping_add(0xc);
        let label = world.wrapping_add(0x18);
        set_point_at(e, world, position);
        let north = field(e, at, NORTH_CELL);
        if north != 0 {
            e.call(ADJUST_FOR_NORTH_ROTATION, &args![north, world, world, 1u32]);
        }
        let space = e.call(REFR_GET_WORLD_SPACE, &args![tracked]).u32();
        let [x, y, z] = point_at(e, world);
        typed_call!(
            super::FAMILY,
            0x0079_c380,
            map_menu_world_to_map_coord(e, Ptr::new(at), marker, x, y, z, space)
        );
        let space = e.call(REFR_GET_WORLD_SPACE, &args![tracked]).u32();
        let made = typed_call!(
            super::FAMILY,
            0x0079_c380,
            map_menu_world_to_map_coord(
                e,
                Ptr::new(at),
                label,
                position[0],
                label_y,
                position[2],
                space
            )
        );
        (e.mem.u32(made), e.mem.u32(made.wrapping_add(4)))
    })
}

/// The map tile's size along `id` (`0xfb1` or `0xfb0`) times `fraction`, each stored as a float.
fn map_extent(e: &mut Engine, at: u32, id: i32, fraction: u32) -> u32 {
    let size = get_float_bits(e, field(e, at, MAP_TILE), id);
    x87_store32(x87_mul(x87_load32(size), x87_m32(fraction)))
}

/// The companion label tile's traits: the position (`0xfa1`, `0xfa2`), the companion's name
/// (`0xfc4`), the width, kind 0x26, depth 2.0, shown, (when `refresh`) `0xfae` 1, the two words 2
/// (`0xfb7`, `0xfb9`) and the height.
fn set_label_traits(e: &mut Engine, label: u32, x: u32, y: u32, companion: u32) {
    set_float(e, label, 0xfa1, x87_quiet_bits(x));
    set_float(e, label, 0xfa2, x87_quiet_bits(y));
    let name = e.call(STATION_NAME, &args![companion]).u32();
    set_string(e, label, 0xfc4, name);
    let width = float_constant(e, LABEL_WIDTH_FLOAT);
    set_float(e, label, 0xfba, width);
    set_int(e, label, 0xfaa, 0x26);
    let depth = float_constant(e, TWO_FLOAT);
    set_float(e, label, 0xfad, depth);
    set_int(e, label, 0xfa3, 1);
    set_int(e, label, 0xfae, 1);
    set_int(e, label, 0xfb7, 2);
    set_int(e, label, 0xfb9, 2);
    let height = float_constant(e, LABEL_HEIGHT_FLOAT);
    set_float(e, label, 0xfb0, height);
}

/// The label's position values following the marker tile: both looked up first, then each
/// cleared, made to follow the marker's same trait (action `0x7d0`; the second also raised by
/// `01034294`, action `0x7d1`) and calculated.
fn follow_marker(e: &mut Engine, label: u32, marker: u32) {
    let along_x = get_value(e, label, 0xfa1);
    let along_y = get_value(e, label, 0xfa2);
    clear_actions(e, along_x);
    e.call(
        VALUE_ADD_ACTION_TILE,
        &args![along_x, 0x7d0u32, marker, 0xfa1u32],
    );
    calculate(e, along_x);
    clear_actions(e, along_y);
    e.call(
        VALUE_ADD_ACTION_TILE,
        &args![along_y, 0x7d0u32, marker, 0xfa2u32],
    );
    let height = float_constant(e, LABEL_HEIGHT_FLOAT);
    e.call(VALUE_ADD_ACTION_FLOAT, &args![along_y, 0x7d1u32, height]);
    calculate(e, along_y);
}

// Typed from 0079e470
/// `MapMenu::UpdateCompanionMarkers` (Xbox PDB): for each of the player's companions (a teleport
/// path built for each), the child of the map tile named after the tracked reference is updated
/// when its text (`0xfc4`) is the companion's name (`00404dc0`): the label's position and traits,
/// with a `"%s Marker"` child the label following that marker; then the label is queued for an
/// update (0x3ff).
pub fn map_menu_update_companion_markers(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let player = e.mem.u32(PLAYER);
    let mut node = e.call(PLAYER_COMPANIONS, &args![player]).u32();
    while node != 0 {
        let companion = item(e, node);
        e.with_stack(0x38, |e, path| {
            let path = path.addr();
            e.call(TELEPORT_PATH_CONSTRUCT, &args![path]);
            let tracked = companion_tracked(e, at, companion, path);
            let label = if tracked != 0 {
                let name = e.call(STATION_NAME, &args![tracked]).u32();
                child_by_name(e, field(e, at, MAP_TILE), name)
            } else {
                0
            };
            let current = label != 0 && get_string(e, label, 0xfc4) != 0 && {
                let text = get_string(e, label, 0xfc4);
                let name = e.call(STATION_NAME, &args![companion]).u32();
                e.call(TEXT_COMPARE_NAMES, &args![name, text]).u32() == 0
            };
            if current {
                update_companion_label(e, at, companion, tracked, label);
            }
            e.call(TELEPORT_PATH_DESTRUCT, &args![path]);
        });
        node = next(e, node);
    }
}

/// `UpdateCompanionMarkers`' work on one current label (see there).
fn update_companion_label(e: &mut Engine, at: u32, companion: u32, tracked: u32, label: u32) {
    let marker = e.with_stack(0x40, |e, text| {
        let name = if tracked != 0 {
            e.call(STATION_NAME, &args![tracked]).u32()
        } else {
            0
        };
        e.call(SPRINTF, &args![text, NAME_MARKER_FORMAT, name]);
        if tracked != 0 {
            child_by_name(e, field(e, at, MAP_TILE), text.addr())
        } else {
            0
        }
    });
    let (label_x, label_y) = label_position(e, at, tracked);
    let x = map_extent(e, at, 0xfb1, label_x);
    let y = map_extent(e, at, 0xfb0, label_y);
    let id = static_id(e, TRAIT_X);
    set_float(e, label, id, x87_quiet_bits(label_x));
    let id = static_id(e, TRAIT_Y);
    set_float(e, label, id, x87_quiet_bits(label_y));
    set_label_traits(e, label, x, y, companion);
    if marker != 0 {
        follow_marker(e, label, marker);
    }
    e.call(TILE_ADD_NEEDS_UPDATE, &args![label, 0x3ffu32]);
}

// ---- The player marker, the tracking reference, the sound list --------------------------------

/// The player's map marker (`00798400`, this family's: the player's word at +0x6f4).
#[inline]
fn player_map_marker(e: &mut Engine, player: u32) -> u32 {
    typed_call!(
        super::FAMILY,
        0x0079_8400,
        map_menu_p2::fn_00798400(e, Ptr::new(player))
    )
}

/// The player's teleport path (`0079f620`, this file's).
#[inline]
fn player_path(e: &mut Engine, player: u32) -> u32 {
    typed_call!(
        super::FAMILY,
        0x0079_f620,
        fn_0079f620(e, Ptr::new(player))
    )
}

// Typed from 0079f360
/// `MapMenu::CreatePlayerMarker` (Xbox PDB): the player's map marker tracked on the world map
/// through `GetWorldMapTrackingRef` with the player's path (a found one gets a waypoint path from
/// the map's player reference, id 0x1e), elsewhere the player's marker reference (`0077a400`).
/// Without one, the marker itself on the world map outside interiors or on the local map in the
/// player's cell. A tracked reference gets a marker tile (the quest marker template, the player
/// marker icon, kind 0x1c, depth `0101712c`, its fade following the player icon's).
pub fn map_menu_create_player_marker(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let mut tracked = if page(e, at) == WORLD_PAGE {
        let path = player_path(e, e.mem.u32(PLAYER));
        let marker = player_map_marker(e, e.mem.u32(PLAYER));
        tracking_ref(e, at, marker, path)
    } else {
        let player = e.mem.u32(PLAYER);
        typed_call!(
            hud::FAMILY,
            0x0077_a400,
            hud::compass::player_marker(e, Ptr::new(player))
        )
        .addr()
    };
    let player = e.mem.u32(PLAYER);
    if tracked != 0 {
        if page(e, at) == WORLD_PAGE {
            let path = player_path(e, player);
            let from = field(e, at, MAP_PLAYER_REF);
            waypoint_path(e, at, from, tracked, path, 0x1e);
        }
    } else if player_map_marker(e, player) != 0 {
        let take = if page(e, at) == WORLD_PAGE {
            let marker = player_map_marker(e, player);
            !e.call(REFR_GET_INTERIOR, &args![marker]).bool()
        } else {
            page(e, at) == LOCAL_PAGE && {
                let marker = player_map_marker(e, player);
                in_player_cell(e, marker)
            }
        };
        if take {
            tracked = player_map_marker(e, player);
        }
    }
    if tracked == 0 {
        return;
    }
    let point = tracked_map_position(e, at, tracked);
    let icon = player_icon(e, at);
    let tile = quest_marker_tile(e, at, point);
    set_string(e, tile, 0xfcc, PLAYER_MARKER_ICON_PATH);
    set_int(e, tile, 0xfaa, 0x1c);
    let depth = float_constant(e, MARKER_DEPTH_FLOAT);
    set_float(e, tile, 0xfad, depth);
    follow_trait(e, tile, 0xff5, icon);
}

// Typed from 0079f620
/// `this + 0x6f8` (on the player: its teleport path).
pub fn fn_0079f620(_e: &mut Engine, this: Ptr) -> u32 {
    this.addr().wrapping_add(0x6f8)
}

// Typed from 0079f640
/// `MapMenu::SetPlayerMarker` (Xbox PDB): with an open map menu and a player map marker, the marker
/// point becomes the marker's position and the marker space its world space, else its cell.
pub fn map_menu_set_player_marker(e: &mut Engine) {
    if singleton(e) == 0 {
        return;
    }
    let player = e.mem.u32(PLAYER);
    if player_map_marker(e, player) == 0 {
        return;
    }
    let marker = player_map_marker(e, player);
    let position = e.vcall(marker, POSITION_SLOT, &args![]).u32();
    let point = point_at(e, position);
    set_point_at(e, singleton(e).wrapping_add(MARKER_POINT), point);
    let marker = player_map_marker(e, player);
    let space = e.call(REFR_GET_WORLD_SPACE, &args![marker]).u32();
    e.mem
        .set_u32(singleton(e).wrapping_add(MARKER_SPACE), space);
    if field(e, singleton(e), MARKER_SPACE) == 0 {
        let marker = player_map_marker(e, player);
        let cell = cell_of(e, marker);
        e.mem.set_u32(singleton(e).wrapping_add(MARKER_SPACE), cell);
    }
}

// Typed from 0079f6f0
/// `MapMenu::DeletePlayerMarker` (Xbox PDB): the map tile's first child 0x1c (once) and every child
/// 0x1e removed, released and deleted.
pub fn map_menu_delete_player_marker(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    e.with_stack(4, |e, slot| {
        let map = field(e, at, MAP_TILE);
        let child = e.call(TILE_GET_CHILD_BY_ID, &args![map, 0x1cu32]).u32();
        e.mem.set_u32(slot.addr(), child);
        if child != 0 {
            remove_child(e, at, slot.addr());
        }
    });
    remove_map_children(e, at, 0x1e);
}

// Typed from 0079f7e0
/// `MapMenu::GetWorldMapTrackingRef` (Xbox PDB): 0 without a path or with both its arrays empty;
/// `reference` when it is null or not in an interior; otherwise the door of the last space (from
/// the end) whose flag byte is set and that has a door at that index, else the map's player
/// reference.
pub fn map_menu_get_world_map_tracking_ref(
    e: &mut Engine,
    this: Ptr,
    reference: u32,
    path: u32,
) -> u32 {
    if path == 0 {
        return 0;
    }
    let doors = path.wrapping_add(0x10);
    if array_size(e, path) == 0 && array_size(e, doors) == 0 {
        return 0;
    }
    if reference == 0 || !e.call(REFR_GET_INTERIOR, &args![reference]).bool() {
        return reference;
    }
    let mut index = array_size(e, path).wrapping_sub(1) as i32;
    while index >= 0 {
        let flag = typed_call!(
            pathing::FAMILY,
            0x006a_1440,
            pathing::navmesh::entry_12(e, Ptr::new(path), index as u32)
        );
        if e.mem.u8(flag) != 0 && (index as u32) < array_size(e, doors) {
            let door = typed_call!(
                pathing::FAMILY,
                0x0055_8dd0,
                pathing::navmesh::triangle_at(e, Ptr::new(doors), index as u32)
            );
            return e.mem.u32(door.addr());
        }
        index = index.wrapping_sub(1);
    }
    field(e, this.addr(), MAP_PLAYER_REF)
}

// Typed from 007a19a0
/// `BSSimpleList<BSSoundHandle>::AddTail` (Xbox PDB): unless `0076b780(0)` says so of the handle,
/// the last node (next at +0xc) gets a new 0x10-byte node (`0076b900` with the handle) linked after
/// it when `0076a3b0(0)` says it is in use, otherwise the handle copied in (`00418900` in place:
/// its word, byte and word).
pub fn fn_007a19a0(e: &mut Engine, this: Ptr, handle: u32) {
    if e.call(SOUND_HANDLE_TEST, &args![handle, 0u32]).bool() {
        return;
    }
    let mut node = this.addr();
    while e.mem.u32(node.wrapping_add(0xc)) != 0 {
        node = e.mem.u32(node.wrapping_add(0xc));
    }
    if e.call(SOUND_NODE_EMPTY, &args![node, 0u32]).bool() {
        let memory = e.call(ALLOCATE, &args![0x10u32]).u32();
        let made = if memory != 0 {
            e.call(SOUND_NODE_CONSTRUCT, &args![memory, handle]).u32()
        } else {
            0
        };
        e.mem.set_u32(node.wrapping_add(0xc), made);
    } else {
        e.mem.set_u32(node, e.mem.u32(handle));
        e.mem
            .set_u8(node.wrapping_add(4), e.mem.u8(handle.wrapping_add(4)));
        e.mem
            .set_u32(node.wrapping_add(8), e.mem.u32(handle.wrapping_add(8)));
    }
}

// ---- The waypoint path ------------------------------------------------------------------------

/// The map point of `reference` (its world space asked first, then its position) through
/// `WorldToMapCoord`: x and y.
fn reference_map_point(e: &mut Engine, at: u32, reference: u32) -> (u32, u32) {
    let space = e.call(REFR_GET_WORLD_SPACE, &args![reference]).u32();
    let [x, y, z] = position(e, reference);
    e.with_stack(0xc, |e, result| {
        let made = typed_call!(
            super::FAMILY,
            0x0079_c380,
            map_menu_world_to_map_coord(e, Ptr::new(at), result.addr(), x, y, z, space)
        );
        (e.mem.u32(made), e.mem.u32(made.wrapping_add(4)))
    })
}

/// A `"WaypointTemplate"` tile under the map tile at a map point, kind `id`.
fn new_waypoint(e: &mut Engine, at: u32, point: (u32, u32), id: u32) -> u32 {
    let menu = singleton(e);
    let tile = e
        .call(
            MENU_RENDER_TEMPLATE,
            &args![menu, field(e, at, MAP_TILE), WAYPOINT_TEMPLATE, 0u32],
        )
        .u32();
    let x_id = static_id(e, TRAIT_X);
    set_float(e, tile, x_id, x87_quiet_bits(point.0));
    let y_id = static_id(e, TRAIT_Y);
    set_float(e, tile, y_id, x87_quiet_bits(point.1));
    set_int(e, tile, 0xfaa, id);
    tile
}

/// The teleport reference of a door (`00568e50`, its first word).
fn door_destination(e: &mut Engine, door: u32) -> u32 {
    let teleport = e.call(REFERENCE_TELEPORT, &args![door]).u32();
    item(e, teleport)
}

/// The door at `index` of a teleport path's doors (+0x10; `00558dd0`, the pathing family's).
fn path_door(e: &mut Engine, path: u32, index: u32) -> u32 {
    let door = typed_call!(
        pathing::FAMILY,
        0x0055_8dd0,
        pathing::navmesh::triangle_at(e, Ptr::new(path.wrapping_add(0x10)), index)
    );
    e.mem.u32(door.addr())
}

/// Whether a reference is in an interior (`TESObjectREFR::GetInterior`, by address).
#[inline]
fn interior(e: &mut Engine, reference: u32) -> bool {
    e.call(REFR_GET_INTERIOR, &args![reference]).bool()
}

/// The two float traits (`TRAIT_X`, `TRAIT_Y`, the y asked first) of the tile of `node` (`next`
/// read again for each trait: the node after it), as an `NiPoint2`.
fn node_point(e: &mut Engine, node: u32, of_next: bool) -> (u32, u32) {
    let tile_of = |e: &Engine| {
        if of_next {
            item(e, next(e, node))
        } else {
            item(e, node)
        }
    };
    let id = static_id(e, TRAIT_Y);
    let y = get_float_bits(e, tile_of(e), id);
    let id = static_id(e, TRAIT_X);
    let x = get_float_bits(e, tile_of(e), id);
    map_corner_bits(e, x, y)
}

// Typed from 0079f8d0
/// `MapMenu::CreateWaypointPath` (Xbox PDB): with `from`, `to` and a teleport path, a
/// `"WaypointTemplate"` tile (kind `id`) at `from` (alpha `0xfa9` 0.0, `0x1004` 1 when it is a door
/// into an interior), one at each exterior door between them (the doors from just after `from` up
/// to just before `to`; the door's own reference, else its teleport reference; `0x1004` 1 when the
/// teleport leads into an interior) and one at `to` (alpha 0.0); then consecutive tiles joined by
/// `"PathLineTemplate"` tiles (kind `id`) whose length (`0x1004`, scaled by the map's
/// `0xfcd`/`0xfce`) and angle (`0xffa`, `-atan2`) span them, with the interior texture when the
/// line's owner has `0x1004`.
pub fn map_menu_create_waypoint_path(
    e: &mut Engine,
    this: Ptr,
    from: u32,
    to: u32,
    path: u32,
    id: u32,
) {
    if from == 0 || to == 0 || path == 0 {
        return;
    }
    let at = this.addr();
    e.with_stack(8, |e, list| {
        let list = list.addr();
        // The 8-byte list's constructor (`0096a2d0`, in place): two zero words.
        e.mem.set_u32(list, 0);
        e.mem.set_u32(list.wrapping_add(4), 0);
        let from_point = reference_map_point(e, at, from);
        let to_point = reference_map_point(e, at, to);
        let first = new_waypoint(e, at, from_point, id);
        set_float(e, first, 0xfa9, 0);
        if form_type(e, base_object(e, from)) == 0x1c {
            let destination = door_destination(e, from);
            if interior(e, destination) {
                set_int(e, first, 0x1004, 1);
            }
        }
        list_append(e, list, first);
        let doors = path.wrapping_add(0x10);
        let mut start = 0i32;
        let mut end = array_size(e, doors).wrapping_sub(1) as i32;
        let mut index = 0u32;
        while index < array_size(e, doors) {
            if path_door(e, path, index) == from || {
                let door = path_door(e, path, index);
                door_destination(e, door) == from
            } {
                start = index.wrapping_add(1) as i32;
            }
            if path_door(e, path, index) == to || {
                let door = path_door(e, path, index);
                door_destination(e, door) == to
            } {
                end = index.wrapping_sub(1) as i32;
            }
            index = index.wrapping_add(1);
        }
        let mut step = start;
        while step <= end {
            let door = path_door(e, path, step as u32);
            let mut reference = if interior(e, door) {
                0
            } else {
                path_door(e, path, step as u32)
            };
            if reference == 0 {
                let door = path_door(e, path, step as u32);
                reference = door_destination(e, door);
            }
            if reference != 0 && !interior(e, reference) {
                let menu = singleton(e);
                let tile = e
                    .call(
                        MENU_RENDER_TEMPLATE,
                        &args![menu, field(e, at, MAP_TILE), WAYPOINT_TEMPLATE, 0u32],
                    )
                    .u32();
                let point = reference_map_point(e, at, reference);
                let x_id = static_id(e, TRAIT_X);
                set_float(e, tile, x_id, x87_quiet_bits(point.0));
                let y_id = static_id(e, TRAIT_Y);
                set_float(e, tile, y_id, x87_quiet_bits(point.1));
                set_int(e, tile, 0xfaa, id);
                let door = path_door(e, path, step as u32);
                let destination = door_destination(e, door);
                if interior(e, destination) {
                    set_int(e, tile, 0x1004, 1);
                }
                list_append(e, list, tile);
            }
            step = step.wrapping_add(1);
        }
        let last = new_waypoint(e, at, to_point, id);
        set_float(e, last, 0xfa9, 0);
        list_append(e, list, last);
        let scale = |e: &mut Engine, id: i32| -> u32 {
            if field(e, at, MAP_TILE) == 0 {
                return 0;
            }
            get_float_bits(e, field(e, at, MAP_TILE), id)
        };
        let scale_x = scale(e, 0xfcd);
        let scale_y = scale(e, 0xfce);
        join_waypoints(e, list, id, (scale_x, scale_y));
        e.call(LIST_DESTRUCT, &args![list]);
    });
}

/// `CreateWaypointPath`'s lines between consecutive tiles of `list` (see there).
fn join_waypoints(e: &mut Engine, list: u32, id: u32, scale: (u32, u32)) {
    let mut node = list;
    while node != 0 && item(e, node) != 0 && next(e, node) != 0 {
        let menu = singleton(e);
        let line = e
            .call(
                MENU_RENDER_TEMPLATE,
                &args![menu, item(e, node), PATH_LINE_TEMPLATE, 0u32],
            )
            .u32();
        let (ax, ay) = node_point(e, node, false);
        let (bx, by) = node_point(e, node, true);
        set_int(e, line, 0xfaa, id);
        let dx = x87_store32(x87_mul(
            x87_sub(x87_load32(bx), x87_m32(ax)),
            x87_m32(scale.0),
        ));
        let square_x = e
            .call(FLOAT_POWER, &args![x87_quiet_bits(dx), 2u32])
            .f64();
        let dy = x87_store32(x87_mul(
            x87_sub(x87_load32(by), x87_m32(ay)),
            x87_m32(scale.1),
        ));
        let square_y = e
            .call(FLOAT_POWER, &args![x87_quiet_bits(dy), 2u32])
            .f64();
        let sum = x87_quiet_bits(x87_store32(x87_add(square_y, square_x)));
        let length = typed_call!(
            pathing::FAMILY,
            0x0045_79e0,
            pathing::point::root_of(e, f32::from_bits(sum))
        );
        set_float(e, line, 0x1004, x87_store32(f64::from(length)));
        let across = x87_quiet_bits(x87_store32(x87_sub(x87_load32(bx), x87_m32(ax))));
        let down = x87_quiet_bits(x87_store32(x87_sub(x87_load32(by), x87_m32(ay))));
        let angle = e.call(ATAN2, &args![down, across]).f64();
        set_float(e, line, 0xffa, x87_store32(-angle));
        if is_true(e, owner(e, line), 0x1004) {
            set_string(e, line, 0xfcc, INTERIOR_PATH_TEXTURE);
        }
        node = next(e, node);
    }
}

// ---- The local map's making -------------------------------------------------------------------

/// The local map's step: the height added to the player's position (`01020758`), the render state
/// words saved, replaced and restored (`011ff3a8` from `011a9bd0`) with the byte `011ff103` set
/// meanwhile, the texture coordinate scale (`00879d90`), the system colour manager, the terrain
/// manager and its object's flag, the cells' seen sections and textures and the cell's markers, the
/// renderer object (`0043c4b0` in place: the word at `011f4748`) bracketing the texture's making,
/// the geometry's allocators, the scissor shape, the tile shader and alpha properties, the shader
/// manager's preparation, the saved maps' directories and file names and the texture's save.
const HEIGHT_OFFSET_DOUBLE: u32 = 0x0102_0758;
const LOCAL_MAP_UV_SCALE: u32 = 0x0087_9d90;
const RENDER_STATE: u32 = 0x011f_f3a8;
const RENDER_STATE_SOURCE: u32 = 0x011a_9bd0;
const RENDER_FLAG: u32 = 0x011f_f103;
const SYSTEM_COLOR_MANAGER_GET_INSTANCE: u32 = 0x0071_8b60;
const SYSTEM_COLOR_MANAGER_GET_COLOR: u32 = 0x0071_9060;
const WORLD_GET_TERRAIN_MANAGER: u32 = 0x0058_6170;
const OBJECT_FLAG: u32 = 0x0045_6610;
const OBJECT_SET_FLAG: u32 = 0x0045_0f90;
const CELL_GET_INT_SEEN_SECTION: u32 = 0x0055_6ef0;
const CELL_GET_INTERIOR_LOCAL_MAP_TEXTURE: u32 = 0x0054_e750;
const CELL_GET_EXTERIOR_TEXTURE: u32 = 0x0054_e640;
const CELL_ADD_MARKERS: u32 = 0x0054_dcf0;
const RENDERER: u32 = 0x011f_4748;
const RENDERER_BEGIN: u32 = 0x004a_0370;
const RENDERER_END: u32 = 0x004a_03c0;
const WORLD_GET_CELL_FROM_CELL_COORD: u32 = 0x0058_75a0;
const CREATE_DIRECTORY_IMPORT: u32 = 0x00fd_f0b8;
const LOCAL_MAPS_DIRECTORIES: [u32; 3] = [0x0107_5520, 0x0107_5508, 0x0107_54f0];
const INTERIOR_DIRECTORY_FORMAT: u32 = 0x0107_54d4;
const INTERIOR_FILE_FORMAT: u32 = 0x0107_54ac;
const EXTERIOR_DIRECTORY_FORMAT: u32 = 0x0107_5490;
const EXTERIOR_FILE_FORMAT: u32 = 0x0107_5468;
const TEXTURE_TABLE: u32 = 0x011f_4418;
const SAVE_TEXTURE_JUMP: u32 = 0x00ee_6dc2;
const SELF_CONSTRUCTOR: u32 = 0x0068_15c0;
const COLOR_CONSTRUCTOR: u32 = 0x004a_7800;
const ALLOCATE_GEOMETRY: u32 = 0x00aa_1070;
const ALLOCATE_OBJECT: u32 = 0x00aa_13e0;
const FOG_ALPHA_FLOAT: u32 = 0x0101_e570;
const FULL_ALPHA_DOUBLE: u32 = 0x0101_e568;
const SCREEN_WIDTH: u32 = 0x0071_5d40;
const SCREEN_HEIGHT: u32 = 0x0071_5da0;
const SCISSOR_TRI_SHAPE_CONSTRUCT: u32 = 0x00c5_cf30;
const TILE_SHADER_PROPERTY_CONSTRUCT: u32 = 0x00bb_7c30;
const SET_TILE_TEXTURE: u32 = 0x00bb_7a10;
const PROPERTY_SET: u32 = 0x008c_c6d0;
const TEXTURE_POINTER_SET: u32 = 0x0066_b0d0;
const AV_OBJECT_ATTACH_PROPERTY: u32 = 0x0043_9410;
const AV_OBJECT_UPDATE_PROPERTIES: u32 = 0x00a5_a040;
const ALPHA_PROPERTY_CONSTRUCT: u32 = 0x0043_91c0;
const ALPHA_PROPERTY_SETS: [(u32, u32); 3] = [(0x0043_9340, 6), (0x0043_9390, 0), (0x0049_ed90, 1)];
const SHADER_MANAGER_PREPARE_OBJECT: u32 = 0x00b5_7e30;
/// A node's virtual slot that attaches a child (+0xdc), an object's name slot (+0x130), a texture
/// object's surface slot (+0x9c).
const ATTACH_CHILD_SLOT: u32 = 0xdc;
const NAME_SLOT: u32 = 0x130;
const SURFACE_SLOT: u32 = 0x9c;

/// An allocation of `count` elements of `size` bytes (all ones when the product overflows), then
/// the vector constructor iterator over it (`00401050`, the dispatch family's) with `constructor`
/// when it is not null.
fn new_array(e: &mut Engine, count: u32, size: u32, constructor: u32) -> u32 {
    let (bytes, overflow) = count.overflowing_mul(size);
    let array = e
        .call(ALLOCATE, &args![if overflow { u32::MAX } else { bytes }])
        .u32();
    if array == 0 {
        return 0;
    }
    typed_call!(
        dispatch::FAMILY,
        0x0040_1050,
        dispatch::math::construct_array(e, array, size, count as i32, constructor)
    );
    array
}

/// A point (`00416870` in place) built in a stack slot from three float bit patterns: its words.
fn point_words(e: &mut Engine, x: u32, y: u32, z: u32) -> [u32; 3] {
    e.with_stack(0xc, |e, point| {
        let point = point.addr();
        set_point_at(e, point, [x, y, z]);
        point_at(e, point)
    })
}

/// The player's world space (`TESObjectREFR::GetWorldSpace`, by address).
#[inline]
fn player_world_space(e: &mut Engine) -> u32 {
    let player = e.mem.u32(PLAYER);
    e.call(REFR_GET_WORLD_SPACE, &args![player]).u32()
}

/// The player's world space's terrain manager's first controller (`00586170`, then `0043b230`,
/// the dispatch family's).
fn terrain_object(e: &mut Engine) -> u32 {
    let space = player_world_space(e);
    let manager = e.call(WORLD_GET_TERRAIN_MANAGER, &args![space]).u32();
    typed_call!(
        dispatch::FAMILY,
        0x0043_b230,
        dispatch::scene::first_controller(e, Ptr::new(manager))
    )
    .addr()
}

/// The texture made by the renderer's bracket (`004a0370`, `004a03c0` on the renderer object).
fn with_renderer(e: &mut Engine, make: impl FnOnce(&mut Engine)) {
    let renderer = e.mem.u32(RENDERER);
    e.call(RENDERER_BEGIN, &args![renderer]);
    make(e);
    let renderer = e.mem.u32(RENDERER);
    e.call(RENDERER_END, &args![renderer]);
}

/// `00406d90` of a float's bits (quieted): the whole number.
#[inline]
fn float_to_int(e: &mut Engine, bits: u32) -> i32 {
    e.call(FLOAT_TO_INT, &args![x87_quiet_bits(bits)]).i32()
}

// Typed from 0079ffb0
/// One step of the local map's making (`MapMenu`, cdecl): with an open map menu whose tile 2 has a
/// node and while the grid is being made (`007a1750`), the next grid cell (the menu's next grid,
/// column and row of the grid's side) around the player gets its texture (an interior through the
/// player's cell, rotated to north, `GetIntSeenSection` and `GetInteriorLocalMapTexture`; an
/// exterior through its world space's cell at that coordinate, `GetSeenData` and `0054e640`), the
/// first cell's markers going to the door list (+0xe0). With `save` the texture is written to
/// `(LocalMaps)\...\%s.%i.%i.png` instead. Otherwise a 17 x 17 vertex `BSScissorTriShape` (cells
/// of 0x400, 0x800 in interiors, over 16; alternating diagonals; the colours' alpha from the fog of
/// war) with a `TileShaderProperty` of the texture and an alpha property is placed at the cell and
/// attached to tile 2's node; tile 2 gets the map size (`0xfcd`, `0xfce`) and the system colour 4
/// (`0xfb2` to `0xfb4`), a zero rotation, and the local map is updated when the grid is complete
/// and tile 2 is the map tile. The render state words, the byte `011ff103` and the terrain flags
/// are restored.
pub fn fn_0079ffb0(e: &mut Engine, save: u8) {
    if singleton(e) == 0 {
        return;
    }
    if in_place::tile_node(e, map_tile(e, singleton(e), 2)) == 0
        || !typed_call!(super::FAMILY, 0x007a_1750, map_menu_p2::fn_007a1750(e))
    {
        return;
    }
    e.with_stack(0x60, |e, frame| {
        // The locals: the player's position (+0), its north-rotated copy (+0xc), the map corner
        // (+0x18), the colour (+0x24), the texture pointer (+0x34) and a point (+0x38).
        let position = frame.addr();
        local_map_step(e, save, position);
    });
}

/// The body of [`fn_0079ffb0`] on its frame at `position` (see there).
fn local_map_step(e: &mut Engine, save: u8, position: u32) {
    let corner = position.wrapping_add(0x18);
    let color = position.wrapping_add(0x24);
    let texture = position.wrapping_add(0x34);
    let player = e.mem.u32(PLAYER);
    let at = e.vcall(player, POSITION_SLOT, &args![]).u32();
    let point = point_at(e, at);
    set_point_at(e, position, point);
    let lift: f64 = e.global(HEIGHT_OFFSET_DOUBLE);
    let z = x87_store32(x87_add(
        x87_load32(e.mem.u32(position.wrapping_add(8))),
        lift,
    ));
    e.mem.set_u32(position.wrapping_add(8), z);
    let player = e.mem.u32(PLAYER);
    let cell_size: i32 = if interior(e, player) { 0x800 } else { 0x400 };
    let side = local_grid_side(e);
    let uv_scale = x87_store32(e.call(LOCAL_MAP_UV_SCALE, &args![]).f64());
    let saved = [0u32, 4, 8, 0xc].map(|k| e.mem.u32(RENDER_STATE + k));
    for k in [0u32, 4, 8, 0xc] {
        let value = e.mem.u32(RENDER_STATE_SOURCE + k);
        e.mem.set_u32(RENDER_STATE + k, value);
    }
    let saved_flag = e.mem.u8(RENDER_FLAG);
    e.mem.set_u8(RENDER_FLAG, 1);
    typed_call!(
        extra_data::FAMILY,
        0x0041_4430,
        extra_data::records::color_new(e, color, 0.0, 0.0, 0.0, 0.0)
    );
    let manager = e.call(SYSTEM_COLOR_MANAGER_GET_INSTANCE, &args![]).u32();
    e.call(SYSTEM_COLOR_MANAGER_GET_COLOR, &args![manager, 4u32, color]);
    let mut terrain_flag = 1u8;
    if player_world_space(e) != 0 && terrain_object(e) != 0 {
        let object = terrain_object(e);
        terrain_flag = e.call(OBJECT_FLAG, &args![object]).u8();
        let object = terrain_object(e);
        e.call(OBJECT_SET_FLAG, &args![object, 1u32]);
    }
    let other = typed_call!(super::FAMILY, 0x007a_1480, map_menu_p2::fn_007a1480(e));
    let other_flag = e.call(OBJECT_FLAG, &args![other]).u8();
    if other_flag == 0 {
        e.call(OBJECT_SET_FLAG, &args![other, 1u32]);
    }
    let column_offset = field(e, singleton(e), NEXT_LOCAL_GRID) % local_grid_side(e) as u32;
    let row_offset = field(e, singleton(e), NEXT_LOCAL_GRID) / local_grid_side(e) as u32;
    // The texture pointer's constructor with null (`00633c90`, in place).
    e.mem.set_u32(texture, 0);
    let start = point_at(e, MARKER_START);
    set_point_at(e, corner, start);
    let offsets = (column_offset as i32, row_offset as i32);
    if interior(e, e.mem.u32(PLAYER)) {
        let cell = cell_of(e, e.mem.u32(PLAYER));
        if cell != 0 {
            interior_texture(e, position, cell, side, offsets);
        }
    } else {
        exterior_texture(e, position, side, offsets);
    }
    if save != 0 {
        save_local_map_texture(e, position, texture, column_offset, row_offset);
        advance_grid(e);
        typed_call!(
            dispatch::FAMILY,
            0x0045_cec0,
            dispatch::refs::release_held(e, Ptr::new(texture))
        );
        return;
    }
    let shape = make_shape(e, position, cell_size, uv_scale);
    make_properties(e, shape, texture, color);
    let two: f64 = e.global(TWO_DOUBLE);
    let span = cell_size.wrapping_mul(side);
    let half = x87_div(f64::from(cell_size), two);
    let along = x87_add(f64::from(cell_size.wrapping_mul(offsets.1)), half);
    let along = x87_quiet_bits(x87_store32(x87_sub(along, f64::from(span))));
    let half = x87_div(f64::from(cell_size), two);
    let across = x87_add(f64::from(cell_size.wrapping_mul(offsets.0)), half);
    let across = x87_quiet_bits(x87_store32(across));
    e.call(NODE_SET_TRANSLATE, &args![shape, across, 0u32, along]);
    e.call(AV_OBJECT_UPDATE_PROPERTIES, &args![shape]);
    let node = in_place::tile_node(e, map_tile(e, singleton(e), 2));
    e.vcall(node, ATTACH_CHILD_SLOT, &args![shape, 1u32]);
    let tile = map_tile(e, singleton(e), 2);
    typed_call!(
        super::FAMILY,
        0x007a_1400,
        map_menu_p2::fn_007a1400(e, Ptr::new(tile), 0x20)
    );
    for (k, word) in [0u32, 4, 8, 0xc].into_iter().zip(saved) {
        e.mem.set_u32(RENDER_STATE + k, word);
    }
    e.mem.set_u8(RENDER_FLAG, saved_flag);
    if terrain_flag == 0 && player_world_space(e) != 0 {
        let object = terrain_object(e);
        e.call(OBJECT_SET_FLAG, &args![object, u32::from(terrain_flag)]);
    }
    if other_flag == 0 {
        e.call(OBJECT_SET_FLAG, &args![other, 0u32]);
    }
    let tile = map_tile(e, singleton(e), 2);
    e.call(TILE_ADD_NEEDS_UPDATE, &args![tile, 0x108u32]);
    for id in [0xfcd, 0xfce] {
        set_int(e, map_tile(e, singleton(e), 2), id, span as u32);
    }
    let full: f64 = e.global(FULL_ALPHA_DOUBLE);
    for (k, id) in [0xfb2, 0xfb3, 0xfb4].into_iter().enumerate() {
        let channel = e.mem.u32(color.wrapping_add(4 * super::word(k)));
        let value = x87_quiet_bits(x87_store32(x87_mul(x87_load32(channel), full)));
        set_float(e, map_tile(e, singleton(e), 2), id, value);
    }
    e.with_stack(0x24, |e, matrix| {
        e.call(MATRIX_SET, &args![matrix, 0u32, 0u32, 0u32]);
        let node = in_place::tile_node(e, map_tile(e, singleton(e), 2));
        typed_call!(
            scene::FAMILY,
            0x00a5_9c60,
            scene::walk::update(e, Ptr::new(node), matrix.addr())
        );
    });
    advance_grid(e);
    if !typed_call!(super::FAMILY, 0x007a_1750, map_menu_p2::fn_007a1750(e)) {
        let menu = singleton(e);
        if field(e, menu, MAP_TILE) == map_tile(e, menu, 2) {
            typed_call!(
                super::FAMILY,
                0x0079_d410,
                map_menu_update_local_map(e, Ptr::new(menu))
            );
        }
    }
    typed_call!(
        dispatch::FAMILY,
        0x0045_cec0,
        dispatch::refs::release_held(e, Ptr::new(texture))
    );
}

/// The menu's next grid moved on by one.
fn advance_grid(e: &mut Engine) {
    let next = field(e, singleton(e), NEXT_LOCAL_GRID).wrapping_add(1);
    e.mem
        .set_u32(singleton(e).wrapping_add(NEXT_LOCAL_GRID), next);
}

/// The interior cell's texture of the local map's step: the player's position rotated to north,
/// the grid cell (offset by 0x800 and the step's offsets), the map corner at its centre, its seen
/// section and texture; the first cell's markers into the door list.
fn interior_texture(e: &mut Engine, position: u32, cell: u32, side: i32, offsets: (i32, i32)) {
    let rotated = position.wrapping_add(0xc);
    let corner = position.wrapping_add(0x18);
    let texture = position.wrapping_add(0x34);
    let point = point_at(e, position);
    set_point_at(e, rotated, point);
    e.call(
        ADJUST_FOR_NORTH_ROTATION,
        &args![cell, position, rotated, 1u32],
    );
    let x = float_to_int(e, e.mem.u32(rotated));
    let cell_x = x.wrapping_sub(0x800) >> 0xc;
    let y = float_to_int(e, e.mem.u32(rotated.wrapping_add(4)));
    let cell_y = y.wrapping_sub(0x800) >> 0xc;
    let column = cell_x.wrapping_add(offsets.0).wrapping_sub(side >> 1);
    let row = cell_y.wrapping_add(offsets.1).wrapping_sub(side >> 1);
    let point = point_words(
        e,
        x87_store32(f64::from((column << 0xc).wrapping_add(0x800))),
        x87_store32(f64::from((row << 0xc).wrapping_add(0x800))),
        0,
    );
    set_point_at(e, corner, point);
    e.call(CELL_GET_INT_SEEN_SECTION, &args![cell, column, row, 0u32]);
    with_renderer(e, |e| {
        e.call(
            CELL_GET_INTERIOR_LOCAL_MAP_TEXTURE,
            &args![cell, column, row, texture],
        );
    });
    if offsets == (0, 0) {
        let markers = singleton(e).wrapping_add(DOOR_MARKERS);
        e.call(CELL_ADD_MARKERS, &args![cell, markers]);
    }
}

/// The exterior cell's texture of the local map's step: the grid cell of the player's position
/// (and the step's offsets), the map corner at it, the world space's cell there with its seen data
/// and texture, and its markers into the door list.
fn exterior_texture(e: &mut Engine, position: u32, side: i32, offsets: (i32, i32)) {
    let corner = position.wrapping_add(0x18);
    let texture = position.wrapping_add(0x34);
    let space = player_world_space(e);
    let cell_x = float_to_int(e, e.mem.u32(position)) >> 0xc;
    let cell_y = float_to_int(e, e.mem.u32(position.wrapping_add(4))) >> 0xc;
    let column = cell_x.wrapping_add(offsets.0).wrapping_sub(side >> 1);
    let row = cell_y.wrapping_add(offsets.1).wrapping_sub(side >> 1);
    let point = point_words(
        e,
        x87_store32(f64::from(column << 0xc)),
        x87_store32(f64::from(row << 0xc)),
        0,
    );
    set_point_at(e, corner, point);
    let cell = if space != 0 {
        e.call(WORLD_GET_CELL_FROM_CELL_COORD, &args![space, column, row])
            .u32()
    } else {
        0
    };
    if cell == 0 {
        return;
    }
    typed_call!(
        extra_data::FAMILY,
        0x0055_5bc0,
        extra_data::more_setters::seen_data_at_28(e, cell)
    );
    with_renderer(e, |e| {
        e.call(CELL_GET_EXTERIOR_TEXTURE, &args![cell, texture]);
    });
    let markers = singleton(e).wrapping_add(DOOR_MARKERS);
    e.call(CELL_ADD_MARKERS, &args![cell, markers]);
}

/// The local map step's geometry: 17 x 17 positions, normals, texture coordinates and colours (the
/// alpha from the fog of war's seen value of each vertex's world point over `0101db80`), the
/// indices of the 16 x 16 quads with alternating diagonals, and the `BSScissorTriShape` over them
/// (the screen's size through `_ftol2`). The shape.
fn make_shape(e: &mut Engine, position: u32, cell_size: i32, uv_scale: u32) -> u32 {
    let corner = position.wrapping_add(0x18);
    let world = position.wrapping_add(0x38);
    const SIDE: i32 = 0x11;
    const QUADS: i32 = 0x10;
    let vertices = (SIDE * SIDE) as u32;
    let step = cell_size / QUADS;
    let positions = new_array(e, vertices, 0xc, SELF_CONSTRUCTOR);
    let normals = new_array(e, vertices, 0xc, SELF_CONSTRUCTOR);
    let uvs = new_array(e, vertices, 8, SELF_CONSTRUCTOR);
    let colors = new_array(e, vertices, 0x10, COLOR_CONSTRUCTOR);
    let indices = e
        .call(
            ALLOCATE_GEOMETRY,
            &args![((QUADS * QUADS) << 1).wrapping_mul(3) << 1],
        )
        .u32();
    let two: f64 = e.global(TWO_DOUBLE);
    let mut index = 0u32;
    for across in 0..SIDE {
        for down in 0..SIDE {
            let half = x87_div(f64::from(cell_size), two);
            let x = x87_store32(x87_sub(f64::from(across.wrapping_mul(step)), half));
            let half = x87_div(f64::from(cell_size), two);
            let y = x87_store32(x87_sub(f64::from(down.wrapping_mul(step)), half));
            let point = point_words(e, y, 0, x);
            set_point_at(e, positions.wrapping_add(index * 0xc), point);
            index = index.wrapping_add(1);
        }
    }
    let one = 1.0f32.to_bits();
    let mut index = 0u32;
    for across in 0..SIDE {
        for down in 0..SIDE {
            let normal = point_words(e, 0, one, 0);
            set_point_at(e, normals.wrapping_add(index * 0xc), normal);
            let u = x87_store32(x87_sub(
                1.0,
                x87_div(f64::from(across), f64::from(SIDE)),
            ));
            let v = x87_store32(x87_div(f64::from(down), f64::from(SIDE)));
            let (first, second) = map_corner_bits(e, v, u);
            e.mem.set_u32(uvs.wrapping_add(index * 8), first);
            e.mem.set_u32(uvs.wrapping_add(index * 8 + 4), second);
            let world_x = x87_store32(x87_mul(f64::from(down), x87_m32(uv_scale)));
            let world_y = x87_store32(x87_mul(f64::from(across), x87_m32(uv_scale)));
            set_point_at(e, world, [world_x, world_y, 0]);
            typed_call!(
                pathing::FAMILY,
                0x0063_c8a0,
                pathing::point::add_to(e, Ptr::new(world), Ptr::new(corner))
            );
            let mut alpha = float_constant(e, FOG_ALPHA_FLOAT);
            if e.mem.u8(singleton(e).wrapping_add(FOG_OF_WAR)) != 0 {
                let seen = e.call(CELL_GET_SEEN_VALUE, &args![world]).i32();
                alpha = x87_store32(f64::from(seen));
            }
            let divisor: f64 = e.global(FOG_DIVISOR_DOUBLE);
            let alpha = x87_quiet_bits(x87_store32(x87_div(x87_load32(alpha), divisor)));
            let made = e.with_stack(0x10, |e, value| {
                let made = typed_call!(
                    extra_data::FAMILY,
                    0x0041_4430,
                    extra_data::records::color_new(
                        e,
                        value.addr(),
                        1.0,
                        1.0,
                        1.0,
                        f32::from_bits(alpha)
                    )
                );
                [0u32, 4, 8, 0xc].map(|k| e.mem.u32(made.wrapping_add(k)))
            });
            for (k, word) in made.into_iter().enumerate() {
                e.mem.set_u32(
                    colors.wrapping_add(index * 0x10 + 4 * super::word(k)),
                    word,
                );
            }
            index = index.wrapping_add(1);
        }
    }
    let mut index = 0u32;
    for across in 0..QUADS {
        for down in 0..QUADS {
            let row_start = across.wrapping_mul(SIDE).wrapping_add(down);
            let next_row = across.wrapping_add(1).wrapping_mul(SIDE).wrapping_add(down);
            // `AND 0x80000001` with the sign fix-up: the remainder by 2.
            let corners = if (across % 2) ^ (down % 2) != 0 {
                [
                    next_row,
                    row_start,
                    row_start + 1,
                    row_start + 1,
                    next_row + 1,
                    next_row,
                ]
            } else {
                [
                    next_row + 1,
                    next_row,
                    row_start,
                    row_start,
                    row_start + 1,
                    next_row + 1,
                ]
            };
            for value in corners {
                // The index's low 16 bits.
                let [low, high, ..] = value.to_le_bytes();
                e.mem
                    .set_u16(indices.wrapping_add(index * 2), u16::from_le_bytes([low, high]));
                index = index.wrapping_add(1);
            }
        }
    }
    let memory = e.call(ALLOCATE_OBJECT, &args![0xd4u32]).u32();
    if memory == 0 {
        return 0;
    }
    let height = e.call(SCREEN_HEIGHT, &args![]).f64();
    let height = ftol2(e, height) as u32;
    let width = e.call(SCREEN_WIDTH, &args![]).f64();
    let width = ftol2(e, width) as u32;
    e.call(
        SCISSOR_TRI_SHAPE_CONSTRUCT,
        &args![
            memory,
            vertices & 0xffff,
            positions,
            normals,
            colors,
            uvs,
            1u32,
            0u32,
            ((QUADS * QUADS) << 1) as u32,
            indices,
            0u32,
            0u32,
            width,
            height
        ],
    )
    .u32()
}

/// The local map step's properties: a `TileShaderProperty` of the texture (its texture, the
/// colour, the flags), the texture pointer cleared, and an alpha property (6, 0, 1), both attached
/// to the shape, which the shader manager prepares.
fn make_properties(e: &mut Engine, shape: u32, texture: u32, color: u32) {
    let memory = e.call(ALLOCATE_OBJECT, &args![0xb0u32]).u32();
    let property = if memory != 0 {
        e.call(TILE_SHADER_PROPERTY_CONSTRUCT, &args![memory, 0u32])
            .u32()
    } else {
        0
    };
    let picture = item(e, texture);
    e.call(SET_TILE_TEXTURE, &args![property, picture]);
    e.call(PROPERTY_SET, &args![property, 0u32]);
    typed_call!(
        super::FAMILY,
        0x007a_1450,
        map_menu_p2::fn_007a1450(e, Ptr::new(property), color)
    );
    typed_call!(
        super::FAMILY,
        0x007a_1430,
        map_menu_p2::fn_007a1430(e, Ptr::new(property), 1)
    );
    e.call(TEXTURE_POINTER_SET, &args![texture, 0u32]);
    e.call(AV_OBJECT_ATTACH_PROPERTY, &args![shape, property]);
    let memory = e.call(ALLOCATE_OBJECT, &args![0x1cu32]).u32();
    let alpha = if memory != 0 {
        e.call(ALPHA_PROPERTY_CONSTRUCT, &args![memory]).u32()
    } else {
        0
    };
    for (set, value) in ALPHA_PROPERTY_SETS {
        e.call(set, &args![alpha, value]);
    }
    e.call(AV_OBJECT_ATTACH_PROPERTY, &args![shape, alpha]);
    e.call(SHADER_MANAGER_PREPARE_OBJECT, &args![shape, 0u32, 0u32]);
}

/// The TES object's world space (`004fd3e0`, the multibound family's).
#[inline]
fn tes_world_space(e: &mut Engine) -> u32 {
    let tes = e.mem.u32(INTERIOR_TEST_OBJECT);
    typed_call!(
        multibound::FAMILY,
        0x004f_d3e0,
        multibound::getters::world_space(e, Ptr::new(tes))
    )
}

/// The TES object's interior cell (`005f36f0`, by address).
#[inline]
fn tes_interior(e: &mut Engine) -> u32 {
    let tes = e.mem.u32(INTERIOR_TEST_OBJECT);
    e.call(CELL_INTERIOR_TEST, &args![tes]).u32()
}

/// The local map step's saving branch: `(LocalMaps)`, its `Interiors` and `Exteriors` directories
/// and the current cell's (an interior's, else the world space's; names through slot +0x130)
/// created, then the texture (its surface through `00653270` of its `+0x24`, `0059bb30` in place,
/// and slot +0x9c) saved as `...\%s\%s.%i.%i.png` with the grid offsets (an interior) or the
/// player's cell plus the offsets (an exterior), format 3, palette 0.
fn save_local_map_texture(
    e: &mut Engine,
    position: u32,
    texture: u32,
    column_offset: u32,
    row_offset: u32,
) {
    for directory in LOCAL_MAPS_DIRECTORIES {
        e.call(CREATE_DIRECTORY_IMPORT, &args![directory, 0u32]);
    }
    e.with_stack(0x208, |e, buffers| {
        let directory = buffers.addr();
        let file = directory.wrapping_add(0x104);
        if tes_interior(e) != 0 {
            let cell = tes_interior(e);
            let name = e.vcall(cell, NAME_SLOT, &args![]).u32();
            e.call(
                BUFFER_FORMAT,
                &args![directory, 0x104u32, INTERIOR_DIRECTORY_FORMAT, name],
            );
            e.call(CREATE_DIRECTORY_IMPORT, &args![directory, 0u32]);
            let first = tes_interior(e);
            let second = tes_interior(e);
            let first_name = e.vcall(first, NAME_SLOT, &args![]).u32();
            let second_name = e.vcall(second, NAME_SLOT, &args![]).u32();
            e.call(
                BUFFER_FORMAT,
                &args![
                    file,
                    0x104u32,
                    INTERIOR_FILE_FORMAT,
                    second_name,
                    first_name,
                    column_offset,
                    row_offset
                ],
            );
        } else {
            let space = tes_world_space(e);
            let name = e.vcall(space, NAME_SLOT, &args![]).u32();
            e.call(
                BUFFER_FORMAT,
                &args![directory, 0x104u32, EXTERIOR_DIRECTORY_FORMAT, name],
            );
            e.call(CREATE_DIRECTORY_IMPORT, &args![directory, 0u32]);
            let first = tes_world_space(e);
            let second = tes_world_space(e);
            let y = float_to_int(e, e.mem.u32(position.wrapping_add(4)));
            let row = (y >> 0xc).wrapping_add(row_offset as i32);
            let x = float_to_int(e, e.mem.u32(position));
            let column = (x >> 0xc).wrapping_add(column_offset as i32);
            let first_name = e.vcall(first, NAME_SLOT, &args![]).u32();
            let second_name = e.vcall(second, NAME_SLOT, &args![]).u32();
            e.call(
                BUFFER_FORMAT,
                &args![
                    file,
                    0x104u32,
                    EXTERIOR_FILE_FORMAT,
                    second_name,
                    first_name,
                    column,
                    row
                ],
            );
        }
        let picture = item(e, texture);
        // A texture's surface word (`D3DTexture_LockRect`, `0059bb30` in place: its +0x24).
        let surface = e.mem.u32(picture.wrapping_add(0x24));
        let object = typed_call!(
            dispatch::FAMILY,
            0x0065_3270,
            dispatch::scene::cast_if(e, TEXTURE_TABLE, surface)
        );
        // The slot (+0x9c) reads no stack word; the import takes four words.
        let target = e.vcall(object, SURFACE_SLOT, &args![]).u32();
        e.call(SAVE_TEXTURE_JUMP, &args![file, 3u32, target, 0u32]);
    });
}

/// The functions above, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0079_84d0, map_menu_calculate_player_marker_space(Ptr, u32, u32, u32) -> u32),
        entry!(0x0079_8710, fn_00798710()),
        entry!(0x0079_8840, map_menu_set_player_marker_callback()),
        entry!(0x0079_8a00, fn_00798a00()),
        entry!(0x0079_8ad0, map_menu_stop_note_sound(Ptr)),
        entry!(0x0079_8bf0, map_menu_clear_audio_log_subtites()),
        entry!(0x0079_8cb0, map_menu_do_enter(Ptr, i32, u32)),
        entry!(0x0079_9100, fn_00799100(u32, u32)),
        entry!(0x0079_91b0, map_menu_load_challenge(Ptr, u32)),
        entry!(0x0079_93d0, map_menu_load_note(Ptr, u32)),
        entry!(0x0079_9690, map_menu_do_leave(Ptr, i32, u32)),
        entry!(0x0079_9dc0, fn_00799dc0(Ptr)),
        entry!(0x0079_a130, map_menu_do_idle(Ptr)),
        entry!(0x0079_a660, map_menu_update_playing_sounds(Ptr)),
        entry!(0x0079_c380, map_menu_world_to_map_coord(Ptr, u32, u32, u32, u32, u32) -> u32),
        entry!(0x0079_c450, map_menu_map_to_world_coord(Ptr, u32, u32, u32, u32, u32) -> u32),
        entry!(0x0079_c530, map_menu_do_wheel_move(Ptr, u32, u32)),
        entry!(0x0079_c5a0, map_menu_zoom_map(Ptr, u8, f32)),
        entry!(0x0079_cdb0, map_menu_update_world_map(Ptr)),
        entry!(0x0079_d410, map_menu_update_local_map(Ptr)),
        entry!(0x0079_e0a0, map_menu_add_quest_markers(Ptr)),
        entry!(0x0079_e470, map_menu_update_companion_markers(Ptr)),
        entry!(0x0079_f360, map_menu_create_player_marker(Ptr)),
        entry!(0x0079_f620, fn_0079f620(Ptr) -> u32),
        entry!(0x0079_f640, map_menu_set_player_marker()),
        entry!(0x0079_f6f0, map_menu_delete_player_marker(Ptr)),
        entry!(0x0079_f7e0, map_menu_get_world_map_tracking_ref(Ptr, u32, u32) -> u32),
        entry!(0x0079_f8d0, map_menu_create_waypoint_path(Ptr, u32, u32, u32, u32)),
        entry!(0x0079_ffb0, fn_0079ffb0(u8)),
        entry!(0x007a_19a0, fn_007a19a0(Ptr, u32)),
    ]
}
