//! `fallout/interface/menus/loadingmenu.cpp` (Xbox PDB source unit), subsystem `fallout/interface`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `LoadingMenu` is the menu shown while a save or a cell loads (menu class
//! `0x3ef`). It keeps a list of `TESLoadScreen` forms (`LoadScreens`), cross
//! fades two background tiles and a text tile between them, and owns a
//! background thread object (`LoadingMenuThread`, global `0x011da0c4`).
//!
//! Layout and helpers are at the top of this file. The unit has 80 functions;
//! this file holds the first 40 open ones (`00788730` to `0078b3f0`). The
//! next session continues at `0078b650`.
//!
//! x87 note: the game computes in extended precision and stores `float`
//! results. The translations compute in `f64` and round to `f32` where the
//! code stores a `float`.
//!
//! Not translated: the compiler's exception-unwinding frames and the stack
//! cookie checks.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleList, BSStringT, NiColorA, NiTPointerList};

// ----- layout -----

/// Vtable of `LoadingMenu` (the destructor stores it as its first act).
pub(crate) const LOADING_MENU_VTABLE: u32 = 0x0107_3ebc;
/// The menu class number `LoadingMenu::GetClass` returns.
pub(crate) const LOADING_MENU_CLASS: u32 = 0x3ef;
/// Class of the menu `_Create` closes first (`0x3f4`).
pub(crate) const REPLACED_MENU_CLASS: u32 = 0x3f4;

/// `pTiles` (Xbox PDB): the tile pointers by byte offset from the menu, 38
/// words from `+0x28`; `AttachTileByID` fills them. Slots used by the
/// translations (the roles are read from how the code uses them):
pub(crate) const TILES: u32 = 0x28;
/// Number of tile slots `AttachTileByID` accepts.
pub(crate) const TILE_COUNT: i32 = 0x26;
/// The tile with the 3D model that holds the two slide nodes.
pub(crate) const SLOT_MODEL: u32 = 0x28;
/// Text tile fed by `78aec0` from a setting.
pub(crate) const SLOT_STATUS_TEXT: u32 = 0x34;
/// The loading wheel (the code warns when it has no model).
pub(crate) const SLOT_WHEEL: u32 = 0x54;
/// Description text tile.
pub(crate) const SLOT_DESCRIPTION: u32 = 0x5c;
/// Text tiles fed by `78aec0` from settings.
pub(crate) const SLOT_TEXT_60: u32 = 0x60;
pub(crate) const SLOT_TEXT_64: u32 = 0x64;
/// Tile that `78aec0` sets the 0x1004/0x1005 values of.
pub(crate) const SLOT_SCALED: u32 = 0x50;
/// The two background tiles that fade in and out alternately.
pub(crate) const SLOT_FADE_A: u32 = 0xa8;
pub(crate) const SLOT_FADE_B: u32 = 0xac;
/// The two tip text tiles and the matte tiles `Update` copies them to.
pub(crate) const SLOT_TIP_A: u32 = 0xb0;
pub(crate) const SLOT_TIP_B: u32 = 0xb4;
pub(crate) const SLOT_MATTE_A: u32 = 0xb8;
pub(crate) const SLOT_MATTE_B: u32 = 0xbc;
/// `m_pHoldTextures` (Xbox PDB): 16 `NiPointer<NiTexture>` on the Xbox; the
/// PC code walks and wraps at 4 (one word each).
pub(crate) const HOLD_TEXTURES: u32 = 0x20c;
pub(crate) const HOLD_TEXTURE_COUNT: u32 = 4;
/// `InitialSlideSound` and `SlideSound` (Xbox PDB): `BSSoundHandle`, 12 bytes.
pub(crate) const INITIAL_SLIDE_SOUND: u32 = 0x1f4;
pub(crate) const SLIDE_SOUND: u32 = 0x200;

layout! {
    /// `LoadingMenu` (Xbox PDB), 0x5c0 bytes on the Xbox (the PC size was
    /// not confirmed; the last field used ends at 0x5c0). Offsets checked
    /// against the PC code.
    pub struct LoadingMenu: 0x5c0 {
        /// Vtable (slot `0x00` `~Menu`, `0x04` `AttachTileByID`, `0x34`
        /// `GetClass`).
        0x00 vtable: u32,
        /// `pMenu` (Xbox PDB): `TileMenu*`.
        0x04 pMenu: Ptr,
        /// `StatsTilesList` (Xbox PDB): `NiTPointerList<Tile*>`.
        0xD0 StatsTilesList: Inline<NiTPointerList>,
        /// `ObjectiveTilesList` (Xbox PDB).
        0xDC ObjectiveTilesList: Inline<NiTPointerList>,
        /// `XPProgressTilesList` (Xbox PDB).
        0xE8 XPProgressTilesList: Inline<NiTPointerList>,
        /// `TipTilesList` (Xbox PDB).
        0xF4 TipTilesList: Inline<NiTPointerList>,
        /// `TipTilesList02` (Xbox PDB).
        0x100 TipTilesList02: Inline<NiTPointerList>,
        /// `strBuffer` (Xbox PDB): `BSStringT<char>`.
        0x1B8 strBuffer: Inline<BSStringT>,
        /// `SaveBackgroundColor` (Xbox PDB): `NiColorA`.
        0x1C0 SaveBackgroundColor: Inline<NiColorA>,
        /// `LoadScreens` (Xbox PDB): `BSSimpleList<TESLoadScreen*>`.
        0x1D0 LoadScreens: Inline<BSSimpleList>,
        /// `pNextLoadScreen` (Xbox PDB): the list node after the current one.
        0x1D8 pNextLoadScreen: Ptr,
        /// `pCurrentLoadScreen` (Xbox PDB): `TESLoadScreen*`.
        0x1DC pCurrentLoadScreen: Ptr,
        /// `uiNextBackgroundSwap` (Xbox PDB): tick count of the next swap.
        0x1E0 uiNextBackgroundSwap: u32,
        /// `uiFadeInComplete` (Xbox PDB).
        0x1E4 uiFadeInComplete: u32,
        /// `pLocation` (Xbox PDB): `TESForm*` the screen list was built for.
        0x1E8 pLocation: Ptr,
        /// `pSlide1TSP` (Xbox PDB): `TileShaderProperty*`.
        0x1EC pSlide1TSP: Ptr,
        /// `pSlide2TSP` (Xbox PDB).
        0x1F0 pSlide2TSP: Ptr,
        /// `m_nHoldTextureIndex` (Xbox PDB).
        0x21C m_nHoldTextureIndex: u32,
        /// `usFlags` (Xbox PDB): bits are tested and set through `00458110`
        /// and `00458080` (bit number as argument). Bits seen: 0 the list has
        /// one screen only, 1 slide toggle, 2 audio made multi-threaded,
        /// 3 re-show, 4 fade state, 5 overlay mode, 6 tips enabled,
        /// 7 first screen pending, 8 single screen, 9 interior location.
        0x222 usFlags: u16,
        /// `m_TileCrossFadeStartTime` (Xbox PDB).
        0x5B4 m_TileCrossFadeStartTime: u32,
        /// `m_SlideCount` (Xbox PDB).
        0x5B8 m_SlideCount: u32,
        /// `m_DelayedTipMatteUpdate` (Xbox PDB): bits 0 and 1 ask `Update`
        /// to copy a tip tile's values to its matte tile.
        0x5BC m_DelayedTipMatteUpdate: u32,
    }
}

// ----- globals -----

/// `LoadingMenu::pMe` (Xbox PDB): the one loading menu.
pub(crate) const MENU_INSTANCE: u32 = 0x011d_a0c0;
/// `LoadingMenu::pLoadingThread` (Xbox PDB): the background thread object.
pub(crate) const LOADING_THREAD: u32 = 0x011d_a0c4;
/// Byte set while `Create` reads the menu XML.
const READING_MENU_FILE: u32 = 0x011d_a0c8;
/// Byte set once `Update` has started the loading music.
pub(crate) const MUSIC_STARTED: u32 = 0x011d_a0c9;
/// `LoadingMenu::bInitialLoad` (Xbox PDB): the game's first load.
pub(crate) const INITIAL_LOAD: u32 = 0x011a_0294;
/// `LoadingMenu::bDoTileUpdate` (Xbox PDB).
pub(crate) const DO_TILE_UPDATE: u32 = 0x011a_0295;
/// Byte cleared by the destructor and tested by `00788ab0` and `_Create`.
pub(crate) const MENU_REPLACED: u32 = 0x011d_8907;
/// The byte `00788a90` stores and `00788aa0` returns.
const SWITCH_BYTE: u32 = 0x011d_cfb2;
/// Byte returned by `0078a930`.
const LIST_LOCKED: u32 = 0x011c_abb8;
/// Word: pointer to the object whose `+0xe4c` holds the loading movie.
const MOVIE_OWNER: u32 = 0x011d_ea3c;
/// Word: pointer to the movie player.
const MOVIE_PLAYER: u32 = 0x0126_fac4;
/// Word: object tested with `00678ce0` (byte at `+2`) for overlay mode.
const OVERLAY_OWNER: u32 = 0x011d_ea0c;
/// Word: object tested with `0042ce10` (flag `0x2` at `+0x244`).
const SKIP_OWNER: u32 = 0x011d_df38;
/// Word: the object `004568c0` (texture creation) is called on.
const TEXTURE_OWNER: u32 = 0x011d_ea10;
/// Word: object whose `QMultiBoundRadius`-named getter (`00526ac0`, float in
/// ST0) `_Create` and `Update` compare with 0.
const VALUE_OWNER: u32 = 0x011c_3f3c;
/// Word: pointer to the object that holds the list of all load screens at
/// `+0x128`.
const LOAD_SCREEN_OWNER: u32 = 0x011c_3f2c;
/// Table of pointers to words holding setting pointers (`0078a260`).
const SETTING_TABLE: u32 = 0x011c_6d50;
/// Globals the tip tiles are set from (`0078aec0`).
const TIP_SCALE: u32 = 0x011d_a16c;
const TIP_VALUE: u32 = 0x011d_a170;
const TIP_TEXT_OWNER: u32 = 0x011d_a168;
/// Floats `Update` stores when it starts the music.
const MUSIC_LEVEL_A: u32 = 0x011d_d300;
const MUSIC_LEVEL_B: u32 = 0x011d_d338;

// ----- settings (the INI setting objects; `00403df0` reads a string) -----

const SETTING_SCALE: u32 = 0x011d_2ad8;
const SETTING_MUSIC_PATH: u32 = 0x011d_e740;
const SETTING_TITLE_MUSIC: u32 = 0x011d_a0f0;
const SETTING_FIRST_SCREEN: u32 = 0x011d_a1b0;
/// Three words per `SettingT` in the screen setting array at `0x011da1b0`.
const SETTING_STRIDE: u32 = 12;
const SETTING_MAX_SCREENS: u32 = 0x011d_a138;
const SETTING_MAX_SCREENS_OVERLAY: u32 = 0x011d_a124;
const SETTING_LOCATION_SCREENS: u32 = 0x011d_a108;
const SETTING_INTERVAL_XUI: u32 = 0x011d_a190;
const SETTING_INTERVAL_INITIAL: u32 = 0x011d_a158;
const SETTING_INTERVAL_DEFAULT: u32 = 0x011d_a1a0;
const SETTING_DESCRIPTION: u32 = 0x011d_3000;
const SETTING_TIP_FALLBACK: u32 = 0x011d_1f8c;
const SETTING_TEXT_60: u32 = 0x011d_40f0;
const SETTING_TEXT_64: u32 = 0x011d_2358;
const SETTING_MUSIC_VOLUME: u32 = 0x011d_ed5c;

// ----- constants in .rdata -----

/// Two `double`s the `0xfa7` value of the menu tile is compared with.
const MENU_MODE_A: u32 = 0x0107_04f0;
const MENU_MODE_B: u32 = 0x0107_04e8;
/// `0.0` (`double`).
const ZERO: u32 = 0x0101_2060;
/// `-1.0` (`float`).
const MINUS_ONE: u32 = 0x0101_2054;
/// `pi / 2` (`float`).
const HALF_PI: u32 = 0x0101_ff38;
/// `float` the slide model is scaled by.
const SLIDE_SCALE: u32 = 0x0107_3f20;
/// `double`s the tip matte tile values are moved by (15.0, 30.0).
const MATTE_OFFSET_DOWN: u32 = 0x0101_5a38;
const MATTE_OFFSET_UP: u32 = 0x0101_db88;
/// `double`: cross fade progress per millisecond.
const FADE_PER_MS: u32 = 0x0107_3fb8;
/// `double` 255.0.
const BYTE_SCALE: u32 = 0x0101_e568;
/// `double` 1000.0.
const MS_PER_SECOND: u32 = 0x0101_7b70;
/// `float` both music levels are set to.
const MUSIC_LEVEL: u32 = 0x0101_8180;
/// `float` 255.0: the alpha the second fade tile is set to.
const ALPHA_FULL: u32 = 0x0102_3cd8;

// Strings in .rdata.
const MENU_FILE_PATH: u32 = 0x0107_3f04;
const SLIDE_ONE: u32 = 0x0107_3f2c;
const SLIDE_TWO: u32 = 0x0107_3f24;
const WHEEL_WARNING: u32 = 0x0107_3f34;
const SEQUENCE_WARNING: u32 = 0x0107_3f68;
const MUSIC_PATH_FORMAT: u32 = 0x0107_3f9c;
const TEXTURE_PATH_FORMAT: u32 = 0x0101_dcc4;
const TEXTURES_DIRECTORY: u32 = 0x0101_7f88;
const SCREEN_PATH_FORMAT: u32 = 0x0107_4028;
const LOADING_SCREEN_PATH_FORMAT: u32 = 0x0107_400c;
const MAIN_DIRECTORY: u32 = 0x0105_c2b0;
const LOADING_DIRECTORY: u32 = 0x0107_403c;
/// Source file name the profiler guard in `0078a3e0` is given (with line
/// `0x423`).
const SOURCE_FILE_NAME: u32 = 0x0107_3fc0;
/// Sequence names `Update` plays, by flag bit 1, and the intro one.
const SEQUENCE_ODD: u32 = 0x0104_8458;
const SEQUENCE_EVEN: u32 = 0x0104_844c;
const SEQUENCE_INTRO: u32 = 0x0104_8444;
/// Address the destructor and `_Create` pass to the decal manager's vtable
/// slot `0xac`.
const DECAL_TARGET: u32 = 0x011a_9bd0;

// ----- callees outside this file, by exe address -----

/// Tests / sets bit `bit` of the 16-bit word at `this + 0x222`.
const FLAG_TEST: u32 = 0x0045_8110;
const FLAG_SET: u32 = 0x0045_8080;
/// Milliseconds (`timeGetTime` wrapper).
const TICK_COUNT: u32 = 0x0045_7fe0;
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `__ehvec_dtor` (`00ec5fce`): array, element size, count, destructor.
const VECTOR_DESTRUCT: u32 = 0x00ec_5fce;
/// `NiPointer` getter (the first word of `this`), assignment, destructor and
/// constructor taking a pointer.
const NI_POINTER_GET: u32 = 0x0055_9450;
const NI_POINTER_SET: u32 = 0x0066_b0d0;
const NI_POINTER_DESTRUCTOR: u32 = 0x0045_cec0;
const NI_POINTER_NEW: u32 = 0x0063_3c90;
/// `BSTexturePalette::RemoveTexture` (Xbox PDB).
const REMOVE_TEXTURE: u32 = 0x00a6_1f30;
/// Getters of the globals at `0x011d8a80`, `0x011f4748` and `0x011f6d98`.
const PLAYER_GETTER: u32 = 0x004b_7210;
const DECAL_MANAGER_GETTER: u32 = 0x0043_c4b0;
const AUDIO_GETTER: u32 = 0x0045_3a70;
/// Decal manager calls that bracket the tile changes.
const DECAL_BEGIN: u32 = 0x004a_0370;
const DECAL_END: u32 = 0x004a_03c0;
/// `BSSoundHandle` calls.
const SOUND_STOP: u32 = 0x00ad_88f0;
const SOUND_RELEASE: u32 = 0x00ad_8d10;
const SOUND_DESTRUCTOR: u32 = 0x0048_3710;
/// Destructors: `BSSimpleList`, `NiTPointerList`, `BSStringT`, and the
/// `Menu` base body.
const SIMPLE_LIST_DESTRUCTOR: u32 = 0x0046_ffb0;
const POINTER_LIST_DESTRUCTOR: u32 = 0x004a_1a30;
const STRING_DESTRUCTOR: u32 = 0x0040_37d0;
const MENU_BASE_DESTRUCTOR: u32 = 0x00a1_c520;
/// `NiTPointerList`: remove all, and add (the item is passed by address).
const POINTER_LIST_CLEAR: u32 = 0x004e_d900;
const POINTER_LIST_ADD: u32 = 0x004e_d8c0;
/// List iteration: the address of the item word of a `BSSimpleList` node,
/// the next node, and the `NiTPointerList` step that returns the address of
/// the element and advances the iterator whose address it is given.
const NODE_ITEM: u32 = 0x0068_15c0;
const NODE_NEXT: u32 = 0x0072_6070;
const POINTER_LIST_STEP: u32 = 0x0057_cbe0;
/// `BSSimpleList`: empty test, count, push at the head, append, free nodes.
const LIST_EMPTY: u32 = 0x0082_56d0;
const LIST_COUNT: u32 = 0x005a_e380;
const LIST_PUSH: u32 = 0x005a_e3d0;
const LIST_ADD: u32 = 0x0090_5820;
const LIST_FREE_NODES: u32 = 0x0047_0470;
/// Tile calls: value getter (`float` in ST0), value setter, string setter,
/// `IsTrue`, `IsVisible`, `UpdateAll`, `GetMaximumDepth`, `GetMenuByClass`,
/// `GetMenu`, `ReadFile`, `Menu::SetMenuTile`.
const TILE_GET_VALUE: u32 = 0x00a0_11b0;
const TILE_SET_VALUE: u32 = 0x00a0_12d0;
const TILE_SET_STRING: u32 = 0x00a0_1350;
const TILE_IS_TRUE: u32 = 0x00a0_1230;
const TILE_IS_VISIBLE: u32 = 0x00a0_40a0;
const TILE_UPDATE_ALL: u32 = 0x00a0_4200;
const TILE_MAXIMUM_DEPTH: u32 = 0x00a0_1530;
const TILE_GET_MENU_BY_CLASS: u32 = 0x00a0_9030;
const TILE_GET_MENU: u32 = 0x00a0_3c90;
const TILE_READ_FILE: u32 = 0x00a0_1b00;
const MENU_SET_MENU_TILE: u32 = 0x00a1_dc70;
/// Sets a tile value from an integer (`float(value)`, flag 1).
const TILE_SET_INT: u32 = 0x0070_0320;
/// `Tile3D` calls: play a sequence by name, show frame, and the update of a
/// model node; the tile's model (the word at `+0x2c`).
const TILE_PLAY_SEQUENCE: u32 = 0x00a2_0cc0;
const TILE_SHOW_FRAME: u32 = 0x00a2_0ed0;
const NODE_UPDATE: u32 = 0x00a5_9c60;
const TILE_MODEL: u32 = 0x0056_c7f0;
/// `NiUpdateData` constructor (time, two bytes), 12 bytes.
const UPDATE_DATA_NEW: u32 = 0x0043_d410;
/// Sets tile value `id` (second argument) from a word (interface manager
/// helper that forwards to `TILE_SET_VALUE`).
const TILE_SET_ID_VALUE: u32 = 0x0071_5c60;
/// Setting value getters: address of the value (`00403e20`), string
/// (`00403df0`), `float` in ST0 (`00450410`), address of an unsigned value
/// (`0043d4d0`).
const SETTING_VALUE_POINTER: u32 = 0x0040_3e20;
const SETTING_STRING: u32 = 0x0040_3df0;
const SETTING_FLOAT: u32 = 0x0045_0410;
const SETTING_INT_POINTER: u32 = 0x0043_d4d0;
/// String length, `memset`, `snprintf`, and the `BSStringT` formatter.
const STRING_LENGTH: u32 = 0x0044_a670;
const MEMSET: u32 = 0x0040_3d30;
const SNPRINTF: u32 = 0x0040_6d00;
const STRING_FORMAT: u32 = 0x0040_6f60;
/// CRT character conversion used by the name hash.
const CHAR_CONVERT: u32 = 0x00ec_67aa;
/// `_ftol2_sse`: truncates the `f64` in the first two words.
const FLOAT_TO_INT: u32 = 0x00ec_62c0;
/// Works on the `float` at the pointer; called with the bounds 0.0 and 1.0.
const CLAMP_FLOAT: u32 = 0x0053_30e0;
/// Form calls: type byte, name of the object at `+0x18`, name comparison,
/// set path, set temporary, flags word, "is temporary" test.
const FORM_TYPE: u32 = 0x0040_1170;
const FORM_NAME: u32 = 0x0040_8da0;
const NAME_COMPARE: u32 = 0x0040_8b20;
const SET_PATH: u32 = 0x0048_9100;
const SET_TEMPORARY: u32 = 0x0048_4490;
const WORD_AT_8: u32 = 0x0044_ddc0;
const IS_TEMPORARY: u32 = 0x0040_77c0;
/// Load screen calls: constructor, type, match for a location, generic
/// test, random index, minimum.
/// Bytes of a `TESLoadScreen` (`operator new` argument).
const LOAD_SCREEN_SIZE: u32 = 0x40;
const LOAD_SCREEN_NEW: u32 = 0x005a_66e0;
const LOAD_SCREEN_TYPE: u32 = 0x005a_7130;
const LOAD_SCREEN_FOR_LOCATION: u32 = 0x005a_6ec0;
/// True when the location list of the screen (`BSSimpleList` at `+0x2c`) is
/// empty (`005a7110` calls `008256d0` on it).
const LOAD_SCREEN_GENERIC: u32 = 0x005a_7110;
/// The word at `+0x34` of a load screen again (`005f36f0`: identical code
/// folded with `005a7130`).
const LOAD_SCREEN_TYPE_FOLDED: u32 = 0x005f_36f0;
const RANDOM_INDEX: u32 = 0x005a_00a0;
const MINIMUM: u32 = 0x0042_f5a0;
/// Candidate array (`BSSimpleArray`) calls: reset (`008454f0`, argument 0
/// keeps the buffer), element address, add, remove, and the head of the
/// global list of all load screens.
const CANDIDATES_RESET: u32 = 0x0084_54f0;
const CANDIDATE_AT: u32 = 0x0087_7a30;
const CANDIDATE_ADD: u32 = 0x007c_b2e0;
const CANDIDATE_REMOVE: u32 = 0x009a_4320;
const ALL_SCREENS_LIST: u32 = 0x0046_1070;
/// Temporary candidate `BSSimpleArray` (0x10 bytes, vtable `0107410c`):
/// constructor and destructor (in this unit, after
/// `0078b3f0`).
const CANDIDATE_LIST_NEW: u32 = 0x0078_d880;
const CANDIDATE_LIST_DESTRUCTOR: u32 = 0x0078_d8b0;
/// Misc state queries and actions.
const XUI_IS_UP: u32 = 0x0070_edf0;
const OVERLAY_TEST: u32 = 0x0067_8ce0;
const SKIP_TEST: u32 = 0x0042_ce10;
const MENU_CLASS_GETTER: u32 = 0x0070_ede0;
const FLAG_1A8_TEST: u32 = 0x004a_4080;
const SET_CURSOR_ALPHA: u32 = 0x0071_7740;
const SET_MENU_MODE: u32 = 0x0077_1700;
const SET_INFO_FOR_REF: u32 = 0x0077_5a00;
const SET_CROSSHAIR_TARGET_TYPE: u32 = 0x0070_3860;
const SET_CURRENT_SPELL: u32 = 0x0041_fd00;
const PLAYER_BYTE: u32 = 0x0042_4940;
const PLAYER_TILE: u32 = 0x0045_cd60;
const ROOT_TILE_GETTER: u32 = 0x0058_6150;
const PLAYING_SEQUENCE: u32 = 0x00ec_17c0;
const STOP_MOVIE: u32 = 0x00ec_25e0;
const START_SEQUENCE: u32 = 0x00ec_16d0;
const MOVIE_RELEASE: u32 = 0x0060_c9c0;
const MENU_SHOW_CHANGES: u32 = 0x0078_d080;
const LOCATION_FLAG: u32 = 0x0042_5fd0;
const REFR_GET_INTERIOR: u32 = 0x0057_5d10;
const TEXTURE_LOAD: u32 = 0x0045_68c0;
const TEXTURE_APPLY: u32 = 0x00bb_7a10;
const PROFILE_GUARD_NEW: u32 = 0x0040_4eb0;
const PROFILE_GUARD_DESTRUCTOR: u32 = 0x0040_4ee0;
/// Vtable slots: `GetObjectByName` and the node cast.
const OBJECT_BY_NAME_SLOT: u32 = 0x9c;
const AS_NODE_SLOT: u32 = 0x0c;
const NODE_CHILD: u32 = 0x0045_bc00;
const NODE_PROPERTY: u32 = 0x00a5_9d30;
const PROPERTY_ID: u32 = 0x0043_8220;
const FIXED_STRING_NEW: u32 = 0x0043_8170;
const FIXED_STRING_DESTRUCTOR: u32 = 0x0043_81b0;
const MODEL_SET_SCALE: u32 = 0x0044_0490;
const MODEL_SET_ROTATION: u32 = 0x0064_04b0;
const MODEL_SHOW: u32 = 0x0045_0f90;
const LOG_LINE: u32 = 0x005b_5e40;
const REAL_SCREEN_WIDTH: u32 = 0x0070_6e40;
const REAL_SCREEN_HEIGHT: u32 = 0x0070_6e10;
const AUDIO_IS_ACTIVE: u32 = 0x004f_1540;
const AUDIO_IS_MULTI_THREADED: u32 = 0x005b_b4d0;
const AUDIO_SET_MULTI_THREADED: u32 = 0x00ad_7230;
const AUDIO_FLUSH: u32 = 0x00ad_7740;
const AUDIO_MUTE_TYPE: u32 = 0x00ad_8480;
const AUDIO_FADE: u32 = 0x005d_1740;
const MUSIC_FADE_OUT: u32 = 0x0083_0680;
const MUSIC_PLAY: u32 = 0x0083_00c0;
const MUSIC_UPDATE: u32 = 0x0083_0660;
const MUSIC_REFRESH: u32 = 0x0082_fb70;
const CREDITS_SET_FLAG: u32 = 0x0076_1460;
const SLEEP_TICK: u32 = 0x0040_fca0;
/// `QMultiBoundRadius`-named getter (`00526ac0`, `float` in ST0).
const VALUE_GETTER: u32 = 0x0052_6ac0;
/// Name of the object at `0x011da168` (`005ec330`).
const TEXT_OWNER_NAME: u32 = 0x005e_c330;
/// Getters on the model tile: the word at `+0x3c`, the word at `+0x38`, and
/// the call that takes them.
const TILE_SEQUENCE: u32 = 0x0063_9b40;
const TILE_TARGET: u32 = 0x009e_32d0;
const TILE_SEQUENCE_CALL: u32 = 0x0047_b220;
const SEQUENCE_STATE: u32 = 0x0080_41a0;

// Calls into this unit's later functions (translated by a later session).
const HIDE_ALL_BUT: u32 = 0x0078_cbd0;
const FADE_STEP: u32 = 0x0078_cc50;
const SET_TILE_FADE: u32 = 0x0078_ce10;
const SUSPEND_BACKGROUND_THREAD: u32 = 0x0078_cfc0;
const RESUME_BACKGROUND_THREAD: u32 = 0x0078_d020;
const THREAD_NEW: u32 = 0x0078_d2f0;
const REFRESH_STATS: u32 = 0x0078_d670;
const INIT_ENTER: u32 = 0x0078_b810;
const GROUP_VISIBLE_0078B6A0: u32 = 0x0078_b6a0;
const GROUP_VISIBLE_0078B650: u32 = 0x0078_b650;
const GROUP_VISIBLE_0078B6F0: u32 = 0x0078_b6f0;
const GROUP_VISIBLE_0078B7C0: u32 = 0x0078_b7c0;
const CALC_FACES: u32 = 0x0078_bb40;
const ANNOUNCE_SCREEN: u32 = 0x0078_bb60;

// ----- small helpers -----

fn now(e: &mut Engine) -> u32 {
    e.call(TICK_COUNT, &args![]).u32()
}

/// Whether bit `bit` of the state word is set (`00458110`).
fn flag(e: &mut Engine, this: Ptr<LoadingMenu>, bit: u32) -> bool {
    e.call(FLAG_TEST, &args![this, bit]).bool()
}

/// Sets or clears bit `bit` of the state word (`00458080`).
fn set_flag(e: &mut Engine, this: Ptr<LoadingMenu>, bit: u32, value: u8) {
    e.call(FLAG_SET, &args![this, bit, value]);
}

/// The tile pointer stored at byte offset `offset` of the menu.
fn tile(e: &Engine, this: Ptr<LoadingMenu>, offset: u32) -> u32 {
    e.mem.u32(this.addr() + offset)
}

fn load_screens(this: Ptr<LoadingMenu>) -> u32 {
    this.at(LoadingMenu::LoadScreens).addr()
}

fn string_buffer(this: Ptr<LoadingMenu>) -> u32 {
    this.at(LoadingMenu::strBuffer).addr()
}

fn tile_value(e: &mut Engine, tile: u32, id: u32) -> f32 {
    e.call(TILE_GET_VALUE, &args![tile, id]).f32()
}

fn set_tile_value(e: &mut Engine, tile: u32, id: u32, value: f32, flag: u32) {
    e.call(TILE_SET_VALUE, &args![tile, id, value, flag]);
}

fn set_tile_int(e: &mut Engine, tile: u32, id: u32, value: u32) {
    e.call(TILE_SET_INT, &args![tile, id, value]);
}

fn set_tile_string(e: &mut Engine, tile: u32, id: u32, text: u32, flag: u32) {
    e.call(TILE_SET_STRING, &args![tile, id, text, flag]);
}

/// The `TESLoadScreen` held by the list node `node` (`006815c0` returns the
/// address of the item word).
fn node_item(e: &mut Engine, node: u32) -> u32 {
    let slot = e.call(NODE_ITEM, &args![node]).u32();
    e.mem.u32(slot)
}

fn next_node(e: &mut Engine, node: u32) -> u32 {
    e.call(NODE_NEXT, &args![node]).u32()
}

fn setting_float(e: &mut Engine, setting: u32) -> f32 {
    e.call(SETTING_FLOAT, &args![setting]).f32()
}

fn setting_string(e: &mut Engine, setting: u32) -> u32 {
    e.call(SETTING_STRING, &args![setting]).u32()
}

/// An unsigned INI setting's value (`0043d4d0` returns its address).
fn setting_int(e: &mut Engine, setting: u32) -> u32 {
    let pointer = e.call(SETTING_INT_POINTER, &args![setting]).u32();
    e.mem.u32(pointer)
}

fn model_of(e: &mut Engine, tile: u32) -> u32 {
    e.call(TILE_MODEL, &args![tile]).u32()
}

fn xui_is_up(e: &mut Engine) -> bool {
    e.call(XUI_IS_UP, &args![]).bool()
}

fn overlay_active(e: &mut Engine) -> bool {
    let owner = e.mem.u32(OVERLAY_OWNER);
    e.call(OVERLAY_TEST, &args![owner]).bool()
}

fn audio_instance(e: &mut Engine) -> u32 {
    e.call(AUDIO_GETTER, &args![]).u32()
}

/// Updates the decal manager around a tile change: the pair of calls
/// `004a0370` / `004a03c0` on the manager the global getter returns.
fn decals_begin(e: &mut Engine) {
    let manager = e.call(DECAL_MANAGER_GETTER, &args![]).u32();
    e.call(DECAL_BEGIN, &args![manager]);
}

fn decals_end(e: &mut Engine) {
    let manager = e.call(DECAL_MANAGER_GETTER, &args![]).u32();
    e.call(DECAL_END, &args![manager]);
}

/// The rotation call both model setups make: `pi/2`, axis `(1, 0, 0)`.
fn rotate_half_pi(e: &mut Engine, model: u32) {
    let angle: f32 = e.global(HALF_PI);
    e.call(
        MODEL_SET_ROTATION,
        &args![model, angle, 1.0f32, 0.0f32, 0.0f32],
    );
}

/// A `float` clamped through `005330e0`, which works on the float at the
/// pointer it gets (called with the bounds 0.0 and 1.0).
fn clamp_unit(e: &mut Engine, value: f32) -> f32 {
    e.with_stack(4, |e, cell| {
        e.mem.set_f32(cell.addr(), value);
        e.call(CLAMP_FLOAT, &args![cell, 0.0f32, 1.0f32]);
        e.mem.f32(cell.addr())
    })
}

/// `_ftol2_sse` on a value the game holds in ST0.
fn float_to_int(e: &mut Engine, value: f64) -> u32 {
    e.call(FLOAT_TO_INT, &args![value]).u32()
}

// Translated from 00788730 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadingMenu::GetClass` (Xbox PDB), vtable slot `0x34`: the menu class.
pub fn loading_menu_get_class(_e: &mut Engine, _this: Ptr<LoadingMenu>) -> u32 {
    LOADING_MENU_CLASS
}

// Translated from 00788740 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadingMenu::AttachTileByID` (Xbox PDB), vtable slot `0x04`: stores a
/// non-null tile into `pTiles[id]` when `0 <= id < 0x26`.
pub fn loading_menu_attach_tile_by_id(e: &mut Engine, this: Ptr<LoadingMenu>, id: i32, tile: u32) {
    if tile != 0 && (0..TILE_COUNT).contains(&id) {
        e.mem.set_u32(this.addr() + TILES + id as u32 * 4, tile);
    }
}

// Translated from 00788770 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadingMenu::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor body and, when bit 0 of `flags` is set, `operator delete`.
pub fn loading_menu_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<LoadingMenu>,
    flags: u32,
) -> Ptr<LoadingMenu> {
    fn_007887a0(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 007887a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The destructor body of `LoadingMenu` (no Xbox PDB name on the map): stops
/// the background thread, releases the hold textures, stops the loading
/// music and sounds, clears `pMe` and the loading movie, destroys the
/// members and runs the `Menu` base destructor. The exception-unwinding
/// states are not translated.
pub fn fn_007887a0(e: &mut Engine, this: Ptr<LoadingMenu>) {
    e.set(this, LoadingMenu::vtable, LOADING_MENU_VTABLE);
    e.call(SUSPEND_BACKGROUND_THREAD, &args![]);
    for index in 0..HOLD_TEXTURE_COUNT {
        let pointer = this.addr() + HOLD_TEXTURES + index * 4;
        if e.call(NI_POINTER_GET, &args![pointer]).u32() != 0 {
            let texture = e.call(NI_POINTER_GET, &args![pointer]).u32();
            e.call(REMOVE_TEXTURE, &args![texture]);
            e.call(NI_POINTER_SET, &args![pointer, 0u32]);
        }
    }
    e.mem.set_u8(MENU_REPLACED, 0);
    let manager = e.call(DECAL_MANAGER_GETTER, &args![]).u32();
    e.vcall(
        manager,
        0xac,
        &args![this.at(LoadingMenu::SaveBackgroundColor)],
    );
    let mut stop_music = xui_is_up(e);
    if !stop_music && e.mem.u8(INITIAL_LOAD) != 0 {
        let volume = setting_string(e, SETTING_MUSIC_VOLUME);
        stop_music = e.mem.i8(volume) != 0;
    }
    if stop_music {
        let audio = audio_instance(e);
        if e.call(AUDIO_IS_ACTIVE, &args![audio]).bool() {
            e.call(MUSIC_FADE_OUT, &args![4000u32, 0u32]);
        }
    }
    if fn_00788aa0(e) != 0 {
        fn_00788a90(e, 0);
        e.call(CREDITS_SET_FLAG, &args![0u32]);
        e.call(MUSIC_FADE_OUT, &args![4000u32, 0u32]);
    }
    if flag(e, this, 2) {
        let audio = audio_instance(e);
        if e.call(AUDIO_IS_MULTI_THREADED, &args![audio]).bool() {
            let audio = audio_instance(e);
            e.call(AUDIO_SET_MULTI_THREADED, &args![audio, 0u32]);
        }
    }
    let audio = audio_instance(e);
    e.call(AUDIO_FADE, &args![audio, 1000u32]);
    e.mem.set_u8(INITIAL_LOAD, 0);
    fn_0078a940(e, this);
    let slide_sound = this.addr() + SLIDE_SOUND;
    let initial_sound = this.addr() + INITIAL_SLIDE_SOUND;
    e.call(SOUND_STOP, &args![slide_sound]);
    e.call(SOUND_RELEASE, &args![slide_sound]);
    e.call(SOUND_RELEASE, &args![initial_sound]);
    e.set(this, LoadingMenu::pLocation, Ptr::NULL);
    e.mem.set_u32(MENU_INSTANCE, 0);
    let owner = e.mem.u32(MOVIE_OWNER);
    if owner != 0 && e.mem.u32(owner + 0xe4c) != 0 {
        let movie = e.mem.u32(owner + 0xe4c);
        e.call(MOVIE_RELEASE, &args![movie, 1u32]);
        e.mem.set_u32(owner + 0xe4c, 0);
    }
    e.call(
        VECTOR_DESTRUCT,
        &args![
            this.addr() + HOLD_TEXTURES,
            4u32,
            HOLD_TEXTURE_COUNT,
            NI_POINTER_DESTRUCTOR
        ],
    );
    e.call(SOUND_DESTRUCTOR, &args![slide_sound]);
    e.call(SOUND_DESTRUCTOR, &args![initial_sound]);
    e.call(SIMPLE_LIST_DESTRUCTOR, &args![load_screens(this)]);
    e.call(STRING_DESTRUCTOR, &args![string_buffer(this)]);
    for list in [
        this.at(LoadingMenu::TipTilesList02),
        this.at(LoadingMenu::TipTilesList),
        this.at(LoadingMenu::XPProgressTilesList),
        this.at(LoadingMenu::ObjectiveTilesList),
        this.at(LoadingMenu::StatsTilesList),
    ] {
        e.call(POINTER_LIST_DESTRUCTOR, &args![list]);
    }
    e.call(MENU_BASE_DESTRUCTOR, &args![this]);
}

// Translated from 00788a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a byte in the global at `0x011dcfb2` (the destructor clears it
/// after stopping the music it guards).
pub fn fn_00788a90(e: &mut Engine, value: u8) {
    e.mem.set_u8(SWITCH_BYTE, value);
}

// Translated from 00788aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte stored by `00788a90`.
pub fn fn_00788aa0(e: &mut Engine) -> u8 {
    e.mem.u8(SWITCH_BYTE)
}

// Translated from 00788ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Re-shows an existing loading menu for `location` (called by `Create`
/// after it set state bit 3; does nothing when the bit is clear): suspends
/// the thread, restarts the cross fade clock 4 s in the past, closes the menu
/// of class `0x3f5` when `0x011d8907` or the skip test says so, rebuilds the
/// load-screen list and steps two slides, then shows the wheel and resumes
/// the thread.
pub fn fn_00788ab0(e: &mut Engine, this: Ptr<LoadingMenu>, location: u32) {
    if !flag(e, this, 3) {
        return;
    }
    e.call(SUSPEND_BACKGROUND_THREAD, &args![]);
    let start = now(e).wrapping_sub(4000);
    e.set(this, LoadingMenu::m_TileCrossFadeStartTime, start);
    let skip_owner = e.mem.u32(SKIP_OWNER);
    if e.mem.u8(MENU_REPLACED) != 0 || e.call(SKIP_TEST, &args![skip_owner]).bool() {
        let class = e.call(MENU_CLASS_GETTER, &args![]).u32();
        let found = e.call(TILE_GET_MENU_BY_CLASS, &args![class]).u32();
        let menu = e.call(TILE_GET_MENU, &args![found]).u32();
        if menu != 0 && !e.call(FLAG_1A8_TEST, &args![menu, 1u32]).bool() {
            e.vcall(menu, 0, &args![1u32]);
        }
    }
    if overlay_active(e) {
        fn_0078afa0(e, this, 0);
    } else {
        fn_0078afa0(e, this, 1);
        fn_0078aec0(e, this);
    }
    e.set(this, LoadingMenu::pLocation, Ptr::new(location));
    loading_menu_make_load_screen_list(e, this);
    e.set(this, LoadingMenu::pLocation, Ptr::new(location));
    let swap = now(e);
    e.set(this, LoadingMenu::uiNextBackgroundSwap, swap);
    for _ in 0..2 {
        let count = e.get(this, LoadingMenu::m_SlideCount);
        e.set(this, LoadingMenu::m_SlideCount, count.wrapping_add(1));
        fn_0078a280(e, this, 0);
        fn_0078a9f0(e, this);
        fn_0078a3e0(e, this, 0);
        fn_0078abd0(e, this);
    }
    set_flag(e, this, 3, 0);
    decals_begin(e);
    let wheel = tile(e, this, SLOT_WHEEL);
    if !e.call(TILE_IS_VISIBLE, &args![wheel]).bool() {
        set_tile_int(e, wheel, 0xfa3, 1);
        fn_0078a230(e, this);
    }
    let player = e.call(PLAYER_GETTER, &args![]).u32();
    let player_tile = e.call(PLAYER_TILE, &args![player]).u32();
    set_tile_int(e, player_tile, 0xfa3, 0);
    e.call(TILE_UPDATE_ALL, &args![1u32]);
    decals_end(e);
    e.mem.set_u8(DO_TILE_UPDATE, 1);
    e.call(RESUME_BACKGROUND_THREAD, &args![]);
}

// Translated from 00788cc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadingMenu::Create` (Xbox PDB, `cdecl`, three words): when a menu of
/// the loading menu's class exists it sets bit 3, re-shows it
/// (`00788ab0`) and stores `hidden_flag` in bit 10; otherwise it reads
/// `Data\Menus\loading_menu.xml`, and when the file's menu is a loading menu
/// builds it (tile lists, visibility, `_Create`, the background thread) and
/// returns the root tile. Returns 0 when the file holds no loading menu
/// (that menu is deleted if it has a tile).
pub fn loading_menu_create(
    e: &mut Engine,
    location: u32,
    resume_thread: u8,
    hidden_flag: u8,
) -> u32 {
    let class = fn_00788f20(e);
    let existing = e.call(TILE_GET_MENU_BY_CLASS, &args![class]).u32();
    if existing != 0 {
        let menu = Ptr::<LoadingMenu>::new(e.mem.u32(MENU_INSTANCE));
        set_flag(e, menu, 3, 1);
        fn_00788ab0(e, menu, location);
        let menu = Ptr::<LoadingMenu>::new(e.mem.u32(MENU_INSTANCE));
        set_flag(e, menu, 10, hidden_flag);
        return existing;
    }
    e.mem.set_u8(READING_MENU_FILE, 1);
    let player = e.call(PLAYER_GETTER, &args![]).u32();
    let root = e.call(ROOT_TILE_GETTER, &args![player]).u32();
    let tile_root = e.call(TILE_READ_FILE, &args![root, MENU_FILE_PATH]).u32();
    e.mem.set_u8(READING_MENU_FILE, 0);
    let menu = e.call(TILE_GET_MENU, &args![tile_root]).u32();
    if menu != 0 {
        let class = e.vcall(menu, 0x34, &args![]).u32();
        if class == fn_00788f20(e) {
            e.call(SET_CURSOR_ALPHA, &args![0u32]);
            e.call(MENU_SET_MENU_TILE, &args![menu, tile_root, 0u32]);
            e.mem.set_u32(MENU_INSTANCE, menu);
            let me = Ptr::<LoadingMenu>::new(menu);
            e.call(INIT_ENTER, &args![me]);
            fn_00788f30(e, me);
            fn_00788f80(e, me);
            fn_00789080(e, me);
            fn_00788fe0(e, me);
            e.call(GROUP_VISIBLE_0078B6A0, &args![me, 0u32]);
            e.call(GROUP_VISIBLE_0078B6F0, &args![me, 0u32]);
            e.call(GROUP_VISIBLE_0078B650, &args![me, 0u32]);
            e.call(GROUP_VISIBLE_0078B7C0, &args![me, 0u32]);
            loading_menu_create_impl(e, me, location);
            set_flag(e, me, 10, hidden_flag);
            if e.mem.u32(LOADING_THREAD) == 0 {
                let block = e.call(OPERATOR_NEW, &args![0x4cu32]).u32();
                let thread = if block != 0 {
                    e.call(THREAD_NEW, &args![block]).u32()
                } else {
                    0
                };
                e.mem.set_u32(LOADING_THREAD, thread);
            } else if resume_thread != 0 {
                e.call(RESUME_BACKGROUND_THREAD, &args![]);
            }
            let fade_a = tile(e, me, SLOT_FADE_A);
            set_tile_int(e, fade_a, 0xfa9, 0);
            let fade_b = tile(e, me, SLOT_FADE_B);
            set_tile_int(e, fade_b, 0xfa9, 0);
            return tile_root;
        }
    }
    if menu != 0 && next_node(e, menu) != 0 {
        e.vcall(menu, 0, &args![1u32]);
    }
    0
}

// Translated from 00788f20 (decompiled, FalloutNV.exe 1.4.0.525)
/// A static twin of `GetClass`: the loading menu's class, `0x3ef`.
pub fn fn_00788f20(_e: &mut Engine) -> u32 {
    LOADING_MENU_CLASS
}

/// Clears the tile list at `list` (offset in the menu) and adds the tile
/// slots at `slots` (offsets from the menu), each passed by address as
/// `NiTPointerList::AddItem` takes it.
fn fill_tile_list(e: &mut Engine, this: Ptr<LoadingMenu>, list: u32, slots: &[u32]) {
    e.call(POINTER_LIST_CLEAR, &args![this.addr() + list]);
    for slot in slots {
        e.call(
            POINTER_LIST_ADD,
            &args![this.addr() + list, this.addr() + slot],
        );
    }
}

// Translated from 00788f30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Fills `StatsTilesList` with the tile slots `+0x6c` and `+0x70`.
pub fn fn_00788f30(e: &mut Engine, this: Ptr<LoadingMenu>) {
    fill_tile_list(e, this, LoadingMenu::StatsTilesList.off, &[0x6c, 0x70]);
}

// Translated from 00788f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Fills `ObjectiveTilesList` with the slots `+0x94`, `+0x34` and `+0x60`.
pub fn fn_00788f80(e: &mut Engine, this: Ptr<LoadingMenu>) {
    fill_tile_list(
        e,
        this,
        LoadingMenu::ObjectiveTilesList.off,
        &[0x94, 0x34, 0x60],
    );
}

// Translated from 00788fe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Fills `XPProgressTilesList` with the slots `+0x68`, `+0x98`, `+0x9c`,
/// `+0x64`, `+0x50` and `+0xa0`.
pub fn fn_00788fe0(e: &mut Engine, this: Ptr<LoadingMenu>) {
    fill_tile_list(
        e,
        this,
        LoadingMenu::XPProgressTilesList.off,
        &[0x68, 0x98, 0x9c, 0x64, 0x50, 0xa0],
    );
}

// Translated from 00789080 (decompiled, FalloutNV.exe 1.4.0.525)
/// Fills `TipTilesList` with the slot `+0xb0` and `TipTilesList02` with
/// `+0xb4`.
pub fn fn_00789080(e: &mut Engine, this: Ptr<LoadingMenu>) {
    fill_tile_list(e, this, LoadingMenu::TipTilesList.off, &[SLOT_TIP_A]);
    fill_tile_list(e, this, LoadingMenu::TipTilesList02.off, &[SLOT_TIP_B]);
}

/// The object the virtual `GetObjectByName` (slot `0x9c`) of `model` finds
/// for `name`, which the game passes as a temporary `NiFixedString` on its
/// stack.
fn find_object(e: &mut Engine, model: u32, name: u32) -> u32 {
    e.with_stack(4, |e, fixed| {
        let fixed_string = e.call(FIXED_STRING_NEW, &args![fixed, name]).u32();
        let object = e
            .vcall(model, OBJECT_BY_NAME_SLOT, &args![fixed_string])
            .u32();
        e.call(FIXED_STRING_DESTRUCTOR, &args![fixed]);
        object
    })
}

/// The shader property (`NiAVObject::GetProperty`) of a slide object: of its
/// first child when the object is a node, else of the object itself.
fn slide_property(e: &mut Engine, object: u32, is_node: bool) -> u32 {
    let property = e.call(PROPERTY_ID, &args![]).u32();
    if is_node {
        let child = e.call(NODE_CHILD, &args![object, 0u32]).u32();
        e.call(NODE_PROPERTY, &args![child, property]).u32()
    } else {
        e.call(NODE_PROPERTY, &args![object, property]).u32()
    }
}

/// Switches the audio system to multi-threaded mode when it is not yet and
/// records that in state bit 2.
fn make_audio_multi_threaded(e: &mut Engine, this: Ptr<LoadingMenu>) {
    let instance = audio_instance(e);
    if !e.call(AUDIO_IS_MULTI_THREADED, &args![instance]).bool() {
        let instance = audio_instance(e);
        e.call(AUDIO_SET_MULTI_THREADED, &args![instance, 1u32]);
        set_flag(e, this, 2, 1);
    }
}

// Translated from 007890e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadingMenu::_Create` (Xbox PDB): second stage of building the menu.
/// Fixes the depth value of the menu tile, makes the player's current spell
/// `2` unless its byte is `3`, sets the HUD mode, closes the `0x3f4` menu,
/// then either hides everything (initial load or a nonzero value) or lays
/// out for an interior/exterior chosen from the location's form type, sets up
/// the wheel and the two slide models, builds the load-screen list and
/// starts the audio.
pub fn loading_menu_create_impl(e: &mut Engine, this: Ptr<LoadingMenu>, location: u32) {
    e.set(this, LoadingMenu::uiNextBackgroundSwap, 0);
    let menu_tile = e.get(this, LoadingMenu::pMenu).addr();
    let mode = tile_value(e, menu_tile, 0xfa7) as f64;
    if mode == e.global::<f64>(MENU_MODE_A) || mode == e.global::<f64>(MENU_MODE_B) {
        let depth = e.call(TILE_MAXIMUM_DEPTH, &args![]).f32();
        set_tile_value(e, menu_tile, 0xfad, depth, 1);
    }
    let player = e.call(PLAYER_GETTER, &args![]).u32();
    if e.call(PLAYER_BYTE, &args![player]).u8() as i8 != 3 {
        let player = e.call(PLAYER_GETTER, &args![]).u32();
        e.call(SET_CURRENT_SPELL, &args![player, 2u32]);
    }
    e.call(SET_MENU_MODE, &args![5u32]);
    e.call(SET_INFO_FOR_REF, &args![0u32, 0u32, 0u32]);
    e.call(SET_CROSSHAIR_TARGET_TYPE, &args![0u32]);
    if e.mem.u32(MOVIE_OWNER) != 0 {
        let player = e.mem.u32(MOVIE_PLAYER);
        if !e.call(PLAYING_SEQUENCE, &args![player]).bool() {
            let player = e.mem.u32(MOVIE_PLAYER);
            e.call(STOP_MOVIE, &args![player]);
        }
    }
    let replaced = e
        .call(TILE_GET_MENU_BY_CLASS, &args![REPLACED_MENU_CLASS])
        .u32();
    if replaced != 0 {
        e.vcall(replaced, 0, &args![1u32]);
    }
    e.set(this, LoadingMenu::pLocation, Ptr::new(location));
    let value_owner = e.mem.u32(VALUE_OWNER);
    let value = e.call(VALUE_GETTER, &args![value_owner]).f32() as f64;
    if e.mem.u8(INITIAL_LOAD) != 0 || value != e.global::<f64>(ZERO) {
        e.call(HIDE_ALL_BUT, &args![this, 0u32, 0u32]);
    } else if e.mem.u8(MENU_REPLACED) != 0 || overlay_active(e) {
        fn_0078afa0(e, this, 0);
        decals_begin(e);
        fn_0078a230(e, this);
        decals_end(e);
    } else {
        let location = e.get(this, LoadingMenu::pLocation).addr();
        let skip_owner = e.mem.u32(SKIP_OWNER);
        if location != 0 && !e.call(SKIP_TEST, &args![skip_owner]).bool() {
            let form_type = e.call(FORM_TYPE, &args![location]).u32();
            match form_type.wrapping_sub(0x39) {
                0 => {
                    let interior = e.call(LOCATION_FLAG, &args![location]).u8();
                    set_flag(e, this, 9, interior);
                }
                1..=7 | 0x30 => {
                    let interior = e.call(REFR_GET_INTERIOR, &args![location]).u8();
                    set_flag(e, this, 9, interior);
                }
                _ => {}
            }
        }
        if flag(e, this, 9) {
            e.call(HIDE_ALL_BUT, &args![this, 0xbu32, 0u32]);
            set_flag(e, this, 4, 0);
            let fade_in = now(e);
            e.set(this, LoadingMenu::uiFadeInComplete, fade_in);
            e.mem.set_u8(DO_TILE_UPDATE, 1);
        } else {
            fn_0078afa0(e, this, 1);
            fn_0078aec0(e, this);
        }
        decals_begin(e);
        fn_0078a230(e, this);
        decals_end(e);
    }
    let manager = e.call(DECAL_MANAGER_GETTER, &args![]).u32();
    e.vcall(manager, 0xac, &args![DECAL_TARGET]);
    let wheel_tile = tile(e, this, SLOT_WHEEL);
    if model_of(e, wheel_tile) != 0 {
        let scale = setting_float(e, SETTING_SCALE);
        let node = model_of(e, wheel_tile);
        e.call(MODEL_SET_SCALE, &args![node, scale]);
        let value_pointer = e.call(SETTING_VALUE_POINTER, &args![SETTING_SCALE]).u32();
        let value = e.mem.f32(value_pointer);
        set_tile_value(e, wheel_tile, 0x1004, value, 1);
        let node = model_of(e, wheel_tile);
        rotate_half_pi(e, node);
    } else {
        e.call(LOG_LINE, &args![0x12u32, WHEEL_WARNING]);
    }
    if !flag(e, this, 9) {
        let model_tile = tile(e, this, SLOT_MODEL);
        let model = model_of(e, model_tile);
        let slide_one = find_object(e, model, SLIDE_ONE);
        let as_node = e.vcall(slide_one, AS_NODE_SLOT, &args![]).u32();
        if as_node != 0 {
            let property = slide_property(e, as_node, true);
            e.set(this, LoadingMenu::pSlide1TSP, Ptr::new(property));
            let model = model_of(e, model_tile);
            let slide_two = find_object(e, model, SLIDE_TWO);
            let property = slide_property(e, slide_two, true);
            e.set(this, LoadingMenu::pSlide2TSP, Ptr::new(property));
        } else {
            let property = slide_property(e, slide_one, false);
            e.set(this, LoadingMenu::pSlide1TSP, Ptr::new(property));
            let model = model_of(e, model_tile);
            let slide_two = find_object(e, model, SLIDE_TWO);
            let property = slide_property(e, slide_two, false);
            e.set(this, LoadingMenu::pSlide2TSP, Ptr::new(property));
        }
        loading_menu_make_load_screen_list(e, this);
        if e.mem.u8(INITIAL_LOAD) == 0 {
            fn_0078a280(e, this, 0);
            fn_0078a9f0(e, this);
            fn_0078a3e0(e, this, 0);
            fn_0078abd0(e, this);
        }
        let slide_scale: f32 = e.global(SLIDE_SCALE);
        let node = model_of(e, model_tile);
        e.call(MODEL_SET_SCALE, &args![node, slide_scale]);
        let node = model_of(e, model_tile);
        rotate_half_pi(e, node);
        let width = e.call(REAL_SCREEN_WIDTH, &args![]).f32() as f64;
        let height = e.call(REAL_SCREEN_HEIGHT, &args![]).f32() as f64;
        set_tile_value(e, model_tile, 0x1004, (width / height) as f32, 1);
        e.call(TILE_PLAY_SEQUENCE, &args![model_tile, SEQUENCE_INTRO]);
        e.with_stack(12, |e, update_data| {
            e.call(UPDATE_DATA_NEW, &args![update_data, 0.0f32, 1u32, 0u32]);
            let node = model_of(e, model_tile);
            e.call(NODE_UPDATE, &args![node, update_data]);
        });
        e.call(FADE_STEP, &args![this]);
        e.set(this, LoadingMenu::uiFadeInComplete, 0);
    }
    e.call(MENU_SHOW_CHANGES, &args![]);
    let mut handled = false;
    if e.mem.u8(INITIAL_LOAD) != 0 || location == 0 {
        let instance = audio_instance(e);
        if e.call(AUDIO_IS_ACTIVE, &args![instance]).bool() {
            let volume = setting_string(e, SETTING_MUSIC_VOLUME);
            if e.mem.i8(volume) == 0 {
                make_audio_multi_threaded(e, this);
                handled = true;
            }
        }
    }
    if !handled && e.mem.u8(INITIAL_LOAD) == 0 {
        let instance = audio_instance(e);
        if e.call(AUDIO_IS_ACTIVE, &args![instance]).bool() {
            make_audio_multi_threaded(e, this);
            let instance = audio_instance(e);
            e.call(AUDIO_MUTE_TYPE, &args![instance, 2u32, 1000u32, 6000u32]);
        }
    }
    e.set(this, LoadingMenu::m_TileCrossFadeStartTime, 0);
    let model_tile = tile(e, this, SLOT_MODEL);
    let model = model_of(e, model_tile);
    e.call(MODEL_SHOW, &args![model, 1u32]);
}

/// Copies the tip tile `source` values `0xfa2` (minus 15.0) and `0xfb0`
/// (plus 30.0) to the matte tile `target`.
fn copy_tip_matte(e: &mut Engine, this: Ptr<LoadingMenu>, source: u32, target: u32) {
    let source = tile(e, this, source);
    let target = tile(e, this, target);
    let first = (tile_value(e, source, 0xfa2) as f64 - e.global::<f64>(MATTE_OFFSET_DOWN)) as f32;
    set_tile_value(e, target, 0xfa2, first, 1);
    let second = (tile_value(e, source, 0xfb0) as f64 + e.global::<f64>(MATTE_OFFSET_UP)) as f32;
    set_tile_value(e, target, 0xfb0, second, 1);
}

/// Starts the title music: formats `Data\Music\Special\%s.mp3` with the
/// title music setting and hands it to the music player.
fn start_title_music(e: &mut Engine) {
    e.with_stack(0x40, |e, buffer| {
        e.call(MEMSET, &args![buffer, 0u32, 0x40u32]);
        let name = setting_string(e, SETTING_TITLE_MUSIC);
        e.call(SNPRINTF, &args![buffer, 0x40u32, MUSIC_PATH_FORMAT, name]);
        e.call(CREDITS_SET_FLAG, &args![1u32]);
        e.call(
            MUSIC_PLAY,
            &args![8u32, buffer, 0u32, 1u32, 1u32, 0.0f32, 0u32],
        );
        e.call(MUSIC_UPDATE, &args![]);
        let level: f32 = e.global(MUSIC_LEVEL);
        e.set_global(MUSIC_LEVEL_A, level);
        e.set_global(MUSIC_LEVEL_B, level);
    });
}

// Translated from 00789820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadingMenu::Update` (Xbox PDB), no arguments: once per frame. Applies
/// the delayed tip matte copies, cross fades the two background tiles by the
/// time since `m_TileCrossFadeStartTime`, during the first 3 s starts the
/// title music on the initial load, afterwards swaps to the next load screen
/// when its time has come (or plays the next slide sequence), and finally
/// refreshes the wheel and the audio.
///
/// The wait for `0x011dea10` (while the initial load is shown and no swap
/// time is set) spins on `Sleep(5)`; it ends when the loading thread sets
/// the word, which a one-thread engine never does by itself.
pub fn loading_menu_update(e: &mut Engine, this: Ptr<LoadingMenu>) {
    e.mem.set_u8(DO_TILE_UPDATE, 1);
    let pending = e.get(this, LoadingMenu::m_DelayedTipMatteUpdate);
    if pending & 1 != 0 {
        e.set(this, LoadingMenu::m_DelayedTipMatteUpdate, pending & !1);
        copy_tip_matte(e, this, SLOT_TIP_A, SLOT_MATTE_A);
    }
    let pending = e.get(this, LoadingMenu::m_DelayedTipMatteUpdate);
    if pending & 2 != 0 {
        e.set(this, LoadingMenu::m_DelayedTipMatteUpdate, pending & !2);
        copy_tip_matte(e, this, SLOT_TIP_B, SLOT_MATTE_B);
    }
    let time = now(e);
    if !flag(e, this, 4) {
        let start = e.get(this, LoadingMenu::m_TileCrossFadeStartTime);
        let delta = time.wrapping_sub(start);
        let mut fade = (delta as f64 * e.global::<f64>(FADE_PER_MS)) as f32;
        if start == 0 {
            fade = 1.0;
        }
        fade = clamp_unit(e, fade);
        if e.get(this, LoadingMenu::m_SlideCount) % 2 == 0 {
            fade = (1.0 - fade as f64) as f32;
        }
        let incoming = clamp_unit(e, fade);
        let fade_a = tile(e, this, SLOT_FADE_A);
        if fade_a != 0 {
            let alpha = float_to_int(e, incoming as f64 * e.global::<f64>(BYTE_SCALE));
            set_tile_int(e, fade_a, 0xfa9, alpha);
        }
        let outgoing = clamp_unit(e, (1.0 - fade as f64) as f32);
        let fade_b = tile(e, this, SLOT_FADE_B);
        if fade_b != 0 {
            let alpha = float_to_int(e, outgoing as f64 * e.global::<f64>(BYTE_SCALE));
            set_tile_int(e, fade_b, 0xfa9, alpha);
        }
        let current = e.get(this, LoadingMenu::pCurrentLoadScreen).addr();
        fn_0078afc0(e, this, current, incoming, 1);
        let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
        let next_screen = node_item(e, next);
        fn_0078afc0(e, this, next_screen, outgoing, 2);
        e.mem.set_u8(DO_TILE_UPDATE, 1);
    }
    let elapsed = time.wrapping_sub(e.get(this, LoadingMenu::m_TileCrossFadeStartTime));
    if elapsed < 3000 {
        let wheel = tile(e, this, SLOT_WHEEL);
        // The title music starts once: with a music path set (first branch)
        // the movie player also starts it; with none (second branch) only the
        // music player does.
        let music_path_set = |e: &mut Engine| {
            let path = setting_string(e, SETTING_MUSIC_PATH);
            e.call(STRING_LENGTH, &args![path]).u32() != 0
        };
        if e.mem.u8(INITIAL_LOAD) != 0
            && !flag(e, this, 0)
            && !playing_sequence(e)
            && music_path_set(e)
            && e.mem.u8(MUSIC_STARTED) == 0
        {
            start_title_music(e);
            let model_tile = tile(e, this, SLOT_MODEL);
            let model = model_of(e, model_tile);
            e.call(MODEL_SHOW, &args![model, 1u32]);
            let fade_a = tile(e, this, SLOT_FADE_A);
            set_tile_value(e, fade_a, 0xfa9, 0.0, 1);
            let full: f32 = e.global(ALPHA_FULL);
            let fade_b = tile(e, this, SLOT_FADE_B);
            set_tile_value(e, fade_b, 0xfa9, full, 1);
            let path = setting_string(e, SETTING_MUSIC_PATH);
            let player = e.mem.u32(MOVIE_PLAYER);
            e.call(
                START_SEQUENCE,
                &args![player, path, 0u32, 0xffff_ffffu32, 0u32, 0u32, 0.0f32],
            );
            e.mem.set_u8(MUSIC_STARTED, 1);
            set_tile_int(e, wheel, 0xfa3, 1);
            e.mem.set_u8(DO_TILE_UPDATE, 1);
        } else if e.mem.u8(INITIAL_LOAD) != 0
            && !flag(e, this, 0)
            && !playing_sequence(e)
            && e.mem.u8(MUSIC_STARTED) == 0
            && !music_path_set(e)
        {
            start_title_music(e);
            e.mem.set_u8(MUSIC_STARTED, 1);
        }
    } else if e.get(this, LoadingMenu::uiNextBackgroundSwap) == 0 {
        if e.mem.u8(INITIAL_LOAD) != 0 {
            while e.mem.u32(TEXTURE_OWNER) == 0 {
                e.call(SLEEP_TICK, &args![5u32]);
            }
            let slide = e.get(this, LoadingMenu::pSlide1TSP).addr();
            fn_0078a3e0(e, this, slide);
        }
        if e.get(this, LoadingMenu::uiFadeInComplete) == 0 && flag(e, this, 4) {
            e.call(FADE_STEP, &args![this]);
        } else {
            let fade_in = now(e);
            e.set(this, LoadingMenu::uiFadeInComplete, fade_in);
        }
        let fade_in = e.get(this, LoadingMenu::uiFadeInComplete);
        e.set(this, LoadingMenu::uiNextBackgroundSwap, fade_in);
    } else if flag(e, this, 4) {
        e.call(FADE_STEP, &args![this]);
    } else if !flag(e, this, 9) && now(e) >= e.get(this, LoadingMenu::uiNextBackgroundSwap) {
        swap_load_screen(e, this);
    }
    let wheel = tile(e, this, SLOT_WHEEL);
    if e.call(TILE_IS_VISIBLE, &args![wheel]).bool() {
        fn_0078a230(e, this);
    }
    let instance = audio_instance(e);
    e.call(AUDIO_FLUSH, &args![instance, 1u32]);
    e.call(MUSIC_REFRESH, &args![]);
}

fn playing_sequence(e: &mut Engine) -> bool {
    let player = e.mem.u32(MOVIE_PLAYER);
    e.call(PLAYING_SEQUENCE, &args![player]).bool()
}

/// The part of `Update` that runs when the swap time has come: with the
/// model tile's sequence finished it moves on to the next screen, otherwise
/// it plays the next slide sequence.
fn swap_load_screen(e: &mut Engine, this: Ptr<LoadingMenu>) {
    let model_tile = tile(e, this, SLOT_MODEL);
    if fn_0078a1f0(e, model_tile) {
        if e.mem.u8(INITIAL_LOAD) == 0 {
            let value_owner = e.mem.u32(VALUE_OWNER);
            let value = e.call(VALUE_GETTER, &args![value_owner]).f32() as f64;
            if value == e.global::<f64>(ZERO) && !xui_is_up(e) {
                let tip = tile(e, this, SLOT_TIP_A);
                if !flag(e, this, 3) || e.call(TILE_IS_TRUE, &args![tip, 0xfa3u32]).bool() {
                    fn_0078a280(e, this, 0);
                }
            }
        }
        fn_0078a9f0(e, this);
        fn_0078a3e0(e, this, 0);
        let sequence = e.call(TILE_SEQUENCE, &args![model_tile]).u32();
        let target = e.call(TILE_TARGET, &args![model_tile]).u32();
        e.call(TILE_SEQUENCE_CALL, &args![target, sequence, 0.0f32]);
        fn_0078abd0(e, this);
        let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
        if e.mem.u8(INITIAL_LOAD) != 0 && next == load_screens(this) {
            set_flag(e, this, 0, 1);
        }
        return;
    }
    if e.mem.u8(INITIAL_LOAD) != 0 && flag(e, this, 0) {
        return;
    }
    if flag(e, this, 3) || flag(e, this, 8) {
        return;
    }
    let time = now(e);
    e.set(this, LoadingMenu::uiNextBackgroundSwap, time);
    fn_0078a1c0(e, this, 1);
    let sequence = if flag(e, this, 1) {
        SEQUENCE_ODD
    } else {
        SEQUENCE_EVEN
    };
    if e.call(TILE_PLAY_SEQUENCE, &args![model_tile, sequence])
        .bool()
    {
        let time = now(e);
        e.set(this, LoadingMenu::m_TileCrossFadeStartTime, time);
        let count = e.get(this, LoadingMenu::m_SlideCount);
        e.set(this, LoadingMenu::m_SlideCount, count.wrapping_add(1));
        if flag(e, this, 7) {
            let screens = e.call(LIST_COUNT, &args![load_screens(this)]).u32();
            set_flag(e, this, 8, (screens == 1) as u8);
            set_flag(e, this, 7, 0);
        }
        e.with_stack(12, |e, update_data| {
            e.call(UPDATE_DATA_NEW, &args![update_data, 0.0f32, 1u32, 0u32]);
            let node = model_of(e, model_tile);
            e.call(NODE_UPDATE, &args![node, update_data]);
        });
    } else {
        let sequence = if flag(e, this, 1) {
            SEQUENCE_ODD
        } else {
            SEQUENCE_EVEN
        };
        e.call(LOG_LINE, &args![SEQUENCE_WARNING, sequence]);
    }
}

// Translated from 0078a1c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Toggles bit `bit` of the state word (`usFlags`).
pub fn fn_0078a1c0(e: &mut Engine, this: Ptr<LoadingMenu>, bit: u32) {
    let flags = e.get(this, LoadingMenu::usFlags);
    let toggled = flags ^ (1u32 << (bit & 31)) as u16;
    e.set(this, LoadingMenu::usFlags, toggled);
}

// Translated from 0078a1f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the tile `model_tile` (the word at `+0x3c` is its current
/// sequence) has a sequence whose state word (`+0x44`, `008041a0`) is 1.
pub fn fn_0078a1f0(e: &mut Engine, model_tile: u32) -> bool {
    let sequence = e.mem.u32(model_tile + 0x3c);
    sequence != 0 && e.call(SEQUENCE_STATE, &args![sequence]).u32() == 1
}

// Translated from 0078a230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Shows frame `-1.0` of the wheel tile (`Tile3D` `a20ed0`).
pub fn fn_0078a230(e: &mut Engine, this: Ptr<LoadingMenu>) {
    let wheel = tile(e, this, SLOT_WHEEL);
    let frame: f32 = e.global(MINUS_ONE);
    e.call(TILE_SHOW_FRAME, &args![wheel, frame, 0u32]);
}

// Translated from 0078a260 (decompiled, FalloutNV.exe 1.4.0.525)
/// The string of the setting that the table at `0x011c6d50` lists at
/// `index` (`cdecl`, one word).
pub fn fn_0078a260(e: &mut Engine, index: u32) -> u32 {
    let entry = e.mem.u32(SETTING_TABLE + index * 4);
    let setting = e.mem.u32(entry);
    setting_string(e, setting)
}

// Translated from 0078a280 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the description text of the next load screen on one of the two tip
/// tiles, alternating by `m_SlideCount`, and asks `Update` (bit 0 or 1 of
/// `m_DelayedTipMatteUpdate`) to copy it to the matte; also sets the
/// description tile from a setting. The word argument is not used. The name
/// hash it computes from the screen's name (times 33 plus the converted
/// character, modulo 100000) is not used either, but the conversion calls
/// are made.
pub fn fn_0078a280(e: &mut Engine, this: Ptr<LoadingMenu>, _unused_0: u32) {
    let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
    let screen = node_item(e, next);
    let mut name = e.call(FORM_NAME, &args![screen + 0x18]).u32();
    let mut hash: u32 = 0;
    while e.mem.i8(name) != 0 {
        let character = e.mem.i8(name) as i32;
        let converted = e.call(CHAR_CONVERT, &args![character]).u32();
        hash = (hash << 5).wrapping_add(hash).wrapping_add(converted);
        name += 1;
    }
    let _name_hash = hash % 100_000;
    let (tip, bit) = if e.get(this, LoadingMenu::m_SlideCount) % 2 == 0 {
        (SLOT_TIP_A, 1)
    } else {
        (SLOT_TIP_B, 2)
    };
    let pending = e.get(this, LoadingMenu::m_DelayedTipMatteUpdate);
    e.set(this, LoadingMenu::m_DelayedTipMatteUpdate, pending | bit);
    let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
    let screen = node_item(e, next);
    // `TESDescription::GetText`-like: vtable slot 0x10 of the sub-object at
    // +0x24 with (0, 'DESC').
    let text = e
        .vcall(screen + 0x24, 0x10, &args![0u32, 0x4353_4544u32])
        .u32();
    let tip_tile = tile(e, this, tip);
    set_tile_string(e, tip_tile, 0xfc4, text, 1);
    let description = setting_string(e, SETTING_DESCRIPTION);
    let description_tile = tile(e, this, SLOT_DESCRIPTION);
    set_tile_string(e, description_tile, 0xfc4, description, 1);
    e.mem.set_u8(DO_TILE_UPDATE, 1);
}

// Translated from 0078a3e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Loads the texture `Textures\<name of the next load screen>` into the next
/// ring-buffer slot of `m_pHoldTextures` (releasing the texture that slot
/// held) and applies it to the fading tile that is next (`+0xa8` for even
/// `m_SlideCount`, else `+0xac`; virtual slot `0x20` of that tile gives the
/// shader property). The word argument is overwritten before it is read. The
/// profiler guard (`00404eb0` with line `0x423`) is created and destroyed
/// around it.
pub fn fn_0078a3e0(e: &mut Engine, this: Ptr<LoadingMenu>, _unused_0: u32) {
    e.with_stack(4, |e, guard| {
        e.call(
            PROFILE_GUARD_NEW,
            &args![guard, 0xdu32, 1u32, SOURCE_FILE_NAME, 0x423u32],
        );
        let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
        let screen = node_item(e, next);
        let name = e.call(FORM_NAME, &args![screen + 0x18]).u32();
        e.call(
            STRING_FORMAT,
            &args![
                string_buffer(this),
                TEXTURE_PATH_FORMAT,
                TEXTURES_DIRECTORY,
                name
            ],
        );
        e.with_stack(4, |e, texture| {
            e.call(NI_POINTER_NEW, &args![texture, 0u32]);
            let path = e.call(NI_POINTER_GET, &args![string_buffer(this)]).u32();
            let owner = e.mem.u32(TEXTURE_OWNER);
            e.call(TEXTURE_LOAD, &args![owner, path, texture, 0u32, 0u32]);
            let index = e.get(this, LoadingMenu::m_nHoldTextureIndex);
            let slot = this.addr() + HOLD_TEXTURES + index * 4;
            let next_index = index.wrapping_add(1);
            e.set(this, LoadingMenu::m_nHoldTextureIndex, next_index);
            if next_index >= HOLD_TEXTURE_COUNT {
                e.set(this, LoadingMenu::m_nHoldTextureIndex, 0);
            }
            if e.call(NI_POINTER_GET, &args![slot]).u32() != 0 {
                let held = e.call(NI_POINTER_GET, &args![slot]).u32();
                e.call(REMOVE_TEXTURE, &args![held]);
            }
            let loaded = e.call(NI_POINTER_GET, &args![texture]).u32();
            e.call(NI_POINTER_SET, &args![slot, loaded]);
            let fading = if e.get(this, LoadingMenu::m_SlideCount) % 2 == 0 {
                tile(e, this, SLOT_FADE_A)
            } else {
                tile(e, this, SLOT_FADE_B)
            };
            let property = e.vcall(fading, 0x20, &args![]).u32();
            if property != 0 {
                let loaded = e.call(NI_POINTER_GET, &args![texture]).u32();
                e.call(TEXTURE_APPLY, &args![property, loaded]);
            }
            e.call(NI_POINTER_DESTRUCTOR, &args![texture]);
        });
        e.call(PROFILE_GUARD_DESTRUCTOR, &args![guard]);
    });
}

/// A new temporary `TESLoadScreen` whose path is the text in `strBuffer`.
fn new_load_screen(e: &mut Engine, this: Ptr<LoadingMenu>) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![LOAD_SCREEN_SIZE]).u32();
    let screen = if block != 0 {
        e.call(LOAD_SCREEN_NEW, &args![block]).u32()
    } else {
        0
    };
    let path = e.call(NI_POINTER_GET, &args![string_buffer(this)]).u32();
    e.call(SET_PATH, &args![screen + 0x18, path]);
    e.call(SET_TEMPORARY, &args![screen]);
    screen
}

/// Appends `screen` to the menu's screen list with `add` (`LIST_ADD` or
/// `LIST_PUSH`, which take the address of a word holding the item).
fn add_load_screen(e: &mut Engine, this: Ptr<LoadingMenu>, add: u32, screen: u32) {
    e.with_stack(4, |e, cell| {
        e.mem.set_u32(cell.addr(), screen);
        e.call(add, &args![load_screens(this), cell]);
    });
}

// Translated from 0078a5a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadingMenu::MakeLoadScreenList` (Xbox PDB): rebuilds `LoadScreens`.
/// A non-empty list is cleared first. On the initial load (and no XUI) it
/// builds one screen per setting from `Interface\Loading\<setting>.dds`
/// (or `Main` for the last), starting at the third when a music path is
/// set; otherwise it collects the candidates for the location and picks
/// from them (`BuildCandidateList`, `0078adc0`, once more without a
/// location when too few were found). An empty list gets one default
/// screen. Then the first screen becomes current and is shown (`0078b3f0`).
pub fn loading_menu_make_load_screen_list(e: &mut Engine, this: Ptr<LoadingMenu>) {
    let mut cleared = false;
    if !e.call(LIST_EMPTY, &args![load_screens(this)]).bool() {
        fn_0078a940(e, this);
        cleared = true;
    }
    let overlay = xui_is_up(e) || overlay_active(e);
    set_flag(e, this, 5, overlay as u8);
    if e.mem.u8(INITIAL_LOAD) != 0 && !xui_is_up(e) {
        let music = setting_string(e, SETTING_MUSIC_PATH);
        let start = if e.call(STRING_LENGTH, &args![music]).u32() != 0 {
            3
        } else {
            0
        };
        e.with_stack(4, |e, index| {
            e.mem.set_i32(index.addr(), start);
            while e.mem.i32(index.addr()) < 5 {
                let position = e.mem.i32(index.addr());
                let directory = if position == 4 {
                    MAIN_DIRECTORY
                } else {
                    LOADING_DIRECTORY
                };
                let name =
                    setting_string(e, SETTING_FIRST_SCREEN + position as u32 * SETTING_STRIDE);
                e.call(
                    STRING_FORMAT,
                    &args![string_buffer(this), SCREEN_PATH_FORMAT, directory, name],
                );
                let screen = new_load_screen(e, this);
                add_load_screen(e, this, LIST_ADD, screen);
                fn_0078a910(e, index.addr(), 0);
            }
        });
    } else {
        e.with_stack(0x10, |e, candidates| {
            e.call(CANDIDATE_LIST_NEW, &args![candidates]);
            loading_menu_build_candidate_list(e, this, candidates.addr());
            fn_0078adc0(e, this, candidates.addr());
            if e.get(this, LoadingMenu::pLocation).addr() != 0 {
                let count = e.call(LIST_COUNT, &args![load_screens(this)]).u32();
                let limit = setting_int(e, SETTING_MAX_SCREENS);
                if count < limit && !fn_0078a930(e) {
                    e.set(this, LoadingMenu::pLocation, Ptr::NULL);
                    loading_menu_build_candidate_list(e, this, candidates.addr());
                    fn_0078adc0(e, this, candidates.addr());
                }
            }
            e.call(CANDIDATE_LIST_DESTRUCTOR, &args![candidates]);
        });
    }
    if e.call(LIST_EMPTY, &args![load_screens(this)]).bool() {
        let name = setting_string(e, SETTING_FIRST_SCREEN);
        e.call(
            STRING_FORMAT,
            &args![string_buffer(this), LOADING_SCREEN_PATH_FORMAT, name],
        );
        let screen = new_load_screen(e, this);
        add_load_screen(e, this, LIST_PUSH, screen);
    }
    if cleared {
        let model_tile = tile(e, this, SLOT_MODEL);
        if !fn_0078a1f0(e, model_tile) {
            fn_0078a3e0(e, this, 0);
            fn_0078abd0(e, this);
        }
    }
    set_flag(e, this, 7, 1);
    let first = node_item(e, load_screens(this));
    e.set(this, LoadingMenu::pCurrentLoadScreen, Ptr::new(first));
    e.call(CALC_FACES, &args![this]);
    let current = e.get(this, LoadingMenu::pCurrentLoadScreen).addr();
    fn_0078b3f0(e, this, current, 1);
}

// Translated from 0078a910 (decompiled, FalloutNV.exe 1.4.0.525)
/// Post-increment of the word at `counter` (`cdecl`, two words, the second
/// not used): returns the old value.
pub fn fn_0078a910(e: &mut Engine, counter: u32, _unused_1: u32) -> u32 {
    let old = e.mem.u32(counter);
    e.mem.set_u32(counter, old.wrapping_add(1));
    old
}

// Translated from 0078a930 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `0x011cabb8`.
pub fn fn_0078a930(e: &mut Engine) -> bool {
    e.mem.u8(LIST_LOCKED) != 0
}

// Translated from 0078a940 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the temporary load screens (flag `0x4000`, `004077c0`) of
/// `LoadScreens` through their virtual slot `0x10` (argument 1), frees the
/// list nodes and resets `pNextLoadScreen` to the list head.
pub fn fn_0078a940(e: &mut Engine, this: Ptr<LoadingMenu>) {
    let mut node = load_screens(this);
    while node != 0 {
        let screen = node_item(e, node);
        if screen == 0 {
            break;
        }
        if e.call(IS_TEMPORARY, &args![screen]).bool() {
            let screen = node_item(e, node);
            if screen != 0 {
                e.vcall(screen, 0x10, &args![1u32]);
            }
        }
        node = next_node(e, node);
    }
    e.call(LIST_FREE_NODES, &args![load_screens(this)]);
    e.set(
        this,
        LoadingMenu::pNextLoadScreen,
        Ptr::new(load_screens(this)),
    );
}

// Translated from 0078a9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Advances to the next load screen: the current one becomes the item of
/// `pNextLoadScreen`, which moves on (wrapping to the list head) and, when
/// its item is not null, `0078bb60` gets the former current screen.
pub fn fn_0078a9f0(e: &mut Engine, this: Ptr<LoadingMenu>) {
    let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
    let screen = node_item(e, next);
    e.set(this, LoadingMenu::pCurrentLoadScreen, Ptr::new(screen));
    let following = next_node(e, next);
    e.set(this, LoadingMenu::pNextLoadScreen, Ptr::new(following));
    if following == 0 {
        e.set(
            this,
            LoadingMenu::pNextLoadScreen,
            Ptr::new(load_screens(this)),
        );
    }
    let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
    if next != 0 {
        let item = node_item(e, next);
        if item != 0 {
            let current = e.get(this, LoadingMenu::pCurrentLoadScreen).addr();
            e.call(ANNOUNCE_SCREEN, &args![this, current]);
        }
    }
}

// Translated from 0078aa90 (decompiled, FalloutNV.exe 1.4.0.525)
/// On the initial load with state bit 0 clear and the model tile's sequence
/// not running: moves `pNextLoadScreen` to the last node, shows the screen
/// (`0078a3e0`), sets the swap time to now and returns true; false
/// otherwise.
pub fn fn_0078aa90(e: &mut Engine, this: Ptr<LoadingMenu>) -> bool {
    let model_tile = tile(e, this, SLOT_MODEL);
    if e.mem.u8(INITIAL_LOAD) != 0 && !flag(e, this, 0) && !fn_0078a1f0(e, model_tile) {
        loop {
            let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
            if next_node(e, next) == 0 {
                break;
            }
            let following = next_node(e, next);
            e.set(this, LoadingMenu::pNextLoadScreen, Ptr::new(following));
        }
        fn_0078a3e0(e, this, 0);
        let time = now(e);
        e.set(this, LoadingMenu::uiNextBackgroundSwap, time);
        return true;
    }
    false
}

// Translated from 0078ab20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `screen` is a form of type `0x4b` whose name differs from the
/// name of every screen already in `LoadScreens` (the walk stops at the
/// first equal name; `00408b20` is the name comparison, nonzero meaning
/// different).
pub fn fn_0078ab20(e: &mut Engine, this: Ptr<LoadingMenu>, screen: u32) -> bool {
    let mut result = screen != 0 && e.call(FORM_TYPE, &args![screen]).u32() == 0x4b;
    let mut node = load_screens(this);
    while result && node != 0 {
        let item = node_item(e, node);
        if item == 0 {
            break;
        }
        let listed = e.call(FORM_NAME, &args![item + 0x18]).u32();
        let wanted = e.call(FORM_NAME, &args![screen + 0x18]).u32();
        result = e.call(NAME_COMPARE, &args![wanted, listed]).u32() != 0;
        node = next_node(e, node);
    }
    result
}

// Translated from 0078abd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the next swap time to now plus an interval from the settings
/// (`0x011da190` while an XUI is up, `0x011da158` on the initial load, else
/// `0x011da1a0`), in seconds times 1000 truncated.
pub fn fn_0078abd0(e: &mut Engine, this: Ptr<LoadingMenu>) {
    let interval = if xui_is_up(e) {
        setting_float(e, SETTING_INTERVAL_XUI)
    } else if e.mem.u8(INITIAL_LOAD) != 0 {
        setting_float(e, SETTING_INTERVAL_INITIAL)
    } else {
        setting_float(e, SETTING_INTERVAL_DEFAULT)
    };
    let time = now(e);
    let milliseconds = (interval as f64 * e.global::<f64>(MS_PER_SECOND)) as i64;
    e.set(
        this,
        LoadingMenu::uiNextBackgroundSwap,
        time.wrapping_add(milliseconds as u32),
    );
}

// Translated from 0078ac60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadingMenu::BuildCandidateList` (Xbox PDB): resets `list` and adds
/// every load screen of the global list that suits the location (overlay
/// mode: screens with flag `0x400`; else those matching `pLocation`, or the
/// generic ones without a location). While an overlay or `0x011d8907` is
/// active, an added screen whose type is neither 0 nor 3 is removed again.
pub fn loading_menu_build_candidate_list(e: &mut Engine, this: Ptr<LoadingMenu>, list: u32) {
    e.call(CANDIDATES_RESET, &args![list, 0u32]);
    let owner = e.mem.u32(LOAD_SCREEN_OWNER);
    let mut node = e.call(ALL_SCREENS_LIST, &args![owner]).u32();
    while node != 0 {
        let screen = node_item(e, node);
        if screen == 0 {
            break;
        }
        let location = e.get(this, LoadingMenu::pLocation).addr();
        let suits = if flag(e, this, 5) {
            fn_0078ada0(e, screen)
        } else if location != 0 {
            e.call(LOAD_SCREEN_FOR_LOCATION, &args![screen, location])
                .bool()
        } else {
            e.call(LOAD_SCREEN_GENERIC, &args![screen]).bool()
        };
        if suits {
            let slot = e.call(NODE_ITEM, &args![node]).u32();
            let index = e.call(CANDIDATE_ADD, &args![list, slot]).u32();
            let kind = e.call(LOAD_SCREEN_TYPE, &args![screen]).u32();
            if (e.mem.u8(MENU_REPLACED) != 0 || overlay_active(e)) && kind != 3 && kind != 0 {
                e.call(CANDIDATE_REMOVE, &args![list, index, 1u32]);
            }
        }
        node = next_node(e, node);
    }
}

// Translated from 0078ada0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x400` of the load screen's flags word (`+8`) is set.
pub fn fn_0078ada0(e: &mut Engine, screen: u32) -> bool {
    e.call(WORD_AT_8, &args![screen]).u32() & 0x400 != 0
}

// Translated from 0078adc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Moves random candidates from `list` (the size is its word at `+8`) into
/// `LoadScreens` while there are fewer than the limit (the screen count
/// setting; the overlay setting in overlay mode; with a location and no list
/// lock, the smaller of it and the per-location setting). Each candidate is
/// removed from `list` whether it was taken or not; `0078ab20` decides.
pub fn fn_0078adc0(e: &mut Engine, this: Ptr<LoadingMenu>, list: u32) {
    let mut limit = setting_int(e, SETTING_MAX_SCREENS);
    if flag(e, this, 5) {
        limit = setting_int(e, SETTING_MAX_SCREENS_OVERLAY);
    } else if e.get(this, LoadingMenu::pLocation).addr() != 0 && !fn_0078a930(e) {
        let per_location = setting_int(e, SETTING_LOCATION_SCREENS);
        limit = e.call(MINIMUM, &args![limit, per_location]).u32();
    }
    let mut count = e.call(LIST_COUNT, &args![load_screens(this)]).u32();
    while count < limit {
        if e.call(WORD_AT_8, &args![list]).u32() == 0 {
            break;
        }
        let size = e.call(WORD_AT_8, &args![list]).u32();
        let index = e.call(RANDOM_INDEX, &args![size]).u32();
        let slot = e.call(CANDIDATE_AT, &args![list, index]).u32();
        let screen = e.mem.u32(slot);
        if fn_0078ab20(e, this, screen) {
            let slot = e.call(CANDIDATE_AT, &args![list, index]).u32();
            e.call(LIST_ADD, &args![load_screens(this), slot]);
            count += 1;
        }
        e.call(CANDIDATE_REMOVE, &args![list, index, 1u32]);
    }
}

// Translated from 0078aec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the status tiles: refreshes the stats (`0078d670`) unless the skip
/// test passes for `0x011ddf38`, then sets the scale/value of the `+0x50`
/// tile from `0x011da16c`/`0x011da170`, the `+0x34` tile text from the
/// `0x011da168` object's name (or a setting) and the `+0x60`/`+0x64` texts
/// from settings.
pub fn fn_0078aec0(e: &mut Engine, this: Ptr<LoadingMenu>) {
    let owner = e.mem.u32(SKIP_OWNER);
    if owner == 0 || !e.call(SKIP_TEST, &args![owner]).bool() {
        e.call(REFRESH_STATS, &args![]);
    }
    let scaled = tile(e, this, SLOT_SCALED);
    let scale: f32 = e.global(TIP_SCALE);
    set_tile_value(e, scaled, 0x1004, scale, 1);
    let value = e.mem.u32(TIP_VALUE);
    e.call(TILE_SET_ID_VALUE, &args![scaled, 0x1005u32, value]);
    let text_owner = e.mem.u32(TIP_TEXT_OWNER);
    let text = if text_owner != 0 {
        e.call(TEXT_OWNER_NAME, &args![text_owner]).u32()
    } else {
        setting_string(e, SETTING_TIP_FALLBACK)
    };
    let status = tile(e, this, SLOT_STATUS_TEXT);
    set_tile_string(e, status, 0xfc4, text, 1);
    let text = setting_string(e, SETTING_TEXT_60);
    let label = tile(e, this, SLOT_TEXT_60);
    set_tile_string(e, label, 0xfc4, text, 1);
    let text = setting_string(e, SETTING_TEXT_64);
    let label = tile(e, this, SLOT_TEXT_64);
    set_tile_string(e, label, 0xfc4, text, 1);
}

// Translated from 0078afa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears state bit 6 (tips enabled) from `value`.
pub fn fn_0078afa0(e: &mut Engine, this: Ptr<LoadingMenu>, value: u8) {
    set_flag(e, this, 6, value);
}

/// The load screen `item`, or when null the current next screen's (else the
/// first of the list); null when there is none.
fn screen_or_default(e: &mut Engine, this: Ptr<LoadingMenu>, item: u32) -> u32 {
    if item != 0 {
        return item;
    }
    let next = e.get(this, LoadingMenu::pNextLoadScreen).addr();
    if next != 0 {
        node_item(e, next)
    } else {
        node_item(e, load_screens(this))
    }
}

// Translated from 0078afc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// With tips enabled (bit 6), takes `item` (default: the next screen, else
/// the first) and, when `005f36f0` says it is present, passes `value` and
/// `mode` to the tile-list routine for the screen's type: 1 `0078b360`,
/// 2 `0078b190`, 3 `0078b220`, 4 `0078b100`.
pub fn fn_0078afc0(e: &mut Engine, this: Ptr<LoadingMenu>, item: u32, value: f32, mode: i32) {
    if !flag(e, this, 6) {
        return;
    }
    let item = screen_or_default(e, this, item);
    if item == 0 {
        return;
    }
    if e.call(LOAD_SCREEN_TYPE_FOLDED, &args![item]).u32() == 0 {
        return;
    }
    match e.call(LOAD_SCREEN_TYPE, &args![item]).u32() {
        1 => fn_0078b360(e, this, value, mode),
        2 => fn_0078b190(e, this, value, mode),
        3 => fn_0078b220(e, this, value, mode),
        4 => fn_0078b100(e, this, value, mode),
        _ => {}
    }
}

/// Walks the `NiTPointerList` at offset `list` of the menu; for each
/// non-null tile sets its `0xfa3` value to 1 when it is 0, then calls
/// `0078ce10(this, tile, value)`.
fn fade_tile_list(e: &mut Engine, this: Ptr<LoadingMenu>, list: u32, value: f32) {
    let list = this.addr() + list;
    e.with_stack(4, |e, iterator| {
        let head = e.call(NI_POINTER_GET, &args![list]).u32();
        e.mem.set_u32(iterator.addr(), head);
        while e.mem.u32(iterator.addr()) != 0 {
            let element = e.call(POINTER_LIST_STEP, &args![list, iterator]).u32();
            let tile = e.mem.u32(element);
            if tile != 0 {
                if tile_value(e, tile, 0xfa3) as f64 == e.global::<f64>(ZERO) {
                    set_tile_int(e, tile, 0xfa3, 1);
                }
                e.call(SET_TILE_FADE, &args![this, tile, value]);
            }
        }
    });
}

// Translated from 0078b100 (decompiled, FalloutNV.exe 1.4.0.525)
/// With `mode` 1: fades the tiles of `StatsTilesList` (`fade_tile_list`).
pub fn fn_0078b100(e: &mut Engine, this: Ptr<LoadingMenu>, value: f32, mode: i32) {
    if mode == 1 {
        fade_tile_list(e, this, LoadingMenu::StatsTilesList.off, value);
    }
}

// Translated from 0078b190 (decompiled, FalloutNV.exe 1.4.0.525)
/// With `mode` 1: fades the tiles of `ObjectiveTilesList`.
pub fn fn_0078b190(e: &mut Engine, this: Ptr<LoadingMenu>, value: f32, mode: i32) {
    if mode == 1 {
        fade_tile_list(e, this, LoadingMenu::ObjectiveTilesList.off, value);
    }
}

// Translated from 0078b220 (decompiled, FalloutNV.exe 1.4.0.525)
/// With `mode` 1: fades the tiles of `TipTilesList` and then the tile at
/// `+0xb8`; with `mode` 2 the tiles of `TipTilesList02` and then `+0xbc`.
pub fn fn_0078b220(e: &mut Engine, this: Ptr<LoadingMenu>, value: f32, mode: i32) {
    if mode == 1 {
        fade_tile_list(e, this, LoadingMenu::TipTilesList.off, value);
        let matte = tile(e, this, SLOT_MATTE_A);
        e.call(SET_TILE_FADE, &args![this, matte, value]);
    }
    if mode == 2 {
        fade_tile_list(e, this, LoadingMenu::TipTilesList02.off, value);
        let matte = tile(e, this, SLOT_MATTE_B);
        e.call(SET_TILE_FADE, &args![this, matte, value]);
    }
}

// Translated from 0078b360 (decompiled, FalloutNV.exe 1.4.0.525)
/// With `mode` 1: fades the tiles of `XPProgressTilesList`.
pub fn fn_0078b360(e: &mut Engine, this: Ptr<LoadingMenu>, value: f32, mode: i32) {
    if mode == 1 {
        fade_tile_list(e, this, LoadingMenu::XPProgressTilesList.off, value);
    }
}

// Translated from 0078b3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// With tips enabled (bit 6): shows (`show` nonzero) the tile groups of
/// the load screen `item` (default as in `0078afc0`) by calling the four
/// group routines `0078b6a0`, `0078b650`, `0078b6f0` and `0078b7c0`: the
/// group of the screen's type gets `show`, the others the opposite (screen
/// type 2: `b6a0`, 4: `b650`, 3: `b6f0`, 1: `b7c0`; any other type hides
/// all). With `show` 0 all four get `show`.
pub fn fn_0078b3f0(e: &mut Engine, this: Ptr<LoadingMenu>, item: u32, show: u8) {
    if !flag(e, this, 6) {
        return;
    }
    let item = screen_or_default(e, this, item);
    if item == 0 {
        return;
    }
    let other = (show == 0) as u8;
    let groups = if show == 0 {
        [show; 4]
    } else {
        match e.call(LOAD_SCREEN_TYPE, &args![item]).u32() {
            1 => [other, other, other, show],
            2 => [show, other, other, other],
            3 => [other, other, show, other],
            4 => [other, show, other, other],
            _ => [0; 4],
        }
    };
    e.call(GROUP_VISIBLE_0078B6A0, &args![this, groups[0]]);
    e.call(GROUP_VISIBLE_0078B650, &args![this, groups[1]]);
    e.call(GROUP_VISIBLE_0078B6F0, &args![this, groups[2]]);
    e.call(GROUP_VISIBLE_0078B7C0, &args![this, groups[3]]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00788730, loading_menu_get_class(Ptr<LoadingMenu>) -> u32),
        entry!(
            0x00788740,
            loading_menu_attach_tile_by_id(Ptr<LoadingMenu>, i32, u32)
        ),
        entry!(
            0x00788770,
            loading_menu_scalar_deleting_destructor(Ptr<LoadingMenu>, u32) -> Ptr<LoadingMenu>
        ),
        entry!(0x007887a0, fn_007887a0(Ptr<LoadingMenu>)),
        entry!(0x00788a90, fn_00788a90(u8)),
        entry!(0x00788aa0, fn_00788aa0() -> u8),
        entry!(0x00788ab0, fn_00788ab0(Ptr<LoadingMenu>, u32)),
        entry!(0x00788cc0, loading_menu_create(u32, u8, u8) -> u32),
        entry!(0x00788f20, fn_00788f20() -> u32),
        entry!(0x00788f30, fn_00788f30(Ptr<LoadingMenu>)),
        entry!(0x00788f80, fn_00788f80(Ptr<LoadingMenu>)),
        entry!(0x00788fe0, fn_00788fe0(Ptr<LoadingMenu>)),
        entry!(0x00789080, fn_00789080(Ptr<LoadingMenu>)),
        entry!(0x007890e0, loading_menu_create_impl(Ptr<LoadingMenu>, u32)),
        entry!(0x00789820, loading_menu_update(Ptr<LoadingMenu>)),
        entry!(0x0078a1c0, fn_0078a1c0(Ptr<LoadingMenu>, u32)),
        entry!(0x0078a1f0, fn_0078a1f0(u32) -> bool),
        entry!(0x0078a230, fn_0078a230(Ptr<LoadingMenu>)),
        entry!(0x0078a260, fn_0078a260(u32) -> u32),
        entry!(0x0078a280, fn_0078a280(Ptr<LoadingMenu>, u32)),
        entry!(0x0078a3e0, fn_0078a3e0(Ptr<LoadingMenu>, u32)),
        entry!(
            0x0078a5a0,
            loading_menu_make_load_screen_list(Ptr<LoadingMenu>)
        ),
        entry!(0x0078a910, fn_0078a910(u32, u32) -> u32),
        entry!(0x0078a930, fn_0078a930() -> bool),
        entry!(0x0078a940, fn_0078a940(Ptr<LoadingMenu>)),
        entry!(0x0078a9f0, fn_0078a9f0(Ptr<LoadingMenu>)),
        entry!(0x0078aa90, fn_0078aa90(Ptr<LoadingMenu>) -> bool),
        entry!(0x0078ab20, fn_0078ab20(Ptr<LoadingMenu>, u32) -> bool),
        entry!(0x0078abd0, fn_0078abd0(Ptr<LoadingMenu>)),
        entry!(
            0x0078ac60,
            loading_menu_build_candidate_list(Ptr<LoadingMenu>, u32)
        ),
        entry!(0x0078ada0, fn_0078ada0(u32) -> bool),
        entry!(0x0078adc0, fn_0078adc0(Ptr<LoadingMenu>, u32)),
        entry!(0x0078aec0, fn_0078aec0(Ptr<LoadingMenu>)),
        entry!(0x0078afa0, fn_0078afa0(Ptr<LoadingMenu>, u8)),
        entry!(0x0078afc0, fn_0078afc0(Ptr<LoadingMenu>, u32, f32, i32)),
        entry!(0x0078b100, fn_0078b100(Ptr<LoadingMenu>, f32, i32)),
        entry!(0x0078b190, fn_0078b190(Ptr<LoadingMenu>, f32, i32)),
        entry!(0x0078b220, fn_0078b220(Ptr<LoadingMenu>, f32, i32)),
        entry!(0x0078b360, fn_0078b360(Ptr<LoadingMenu>, f32, i32)),
        entry!(0x0078b3f0, fn_0078b3f0(Ptr<LoadingMenu>, u32, u8)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A vtable whose slot at byte offset `s` points to a double at
    /// `VTABLE_TARGETS + s` (every rig has them).
    const VTABLE: u32 = 0x6000;
    const VTABLE_TARGETS: u32 = 0x6800;

    type Calls = Rc<RefCell<Vec<(u32, Vec<u32>)>>>;

    /// Every callee outside this file: all are doubled in a rig.
    const EXTERNAL: &[u32] = &[
        FLAG_TEST,
        FLAG_SET,
        TICK_COUNT,
        OPERATOR_NEW,
        OPERATOR_DELETE,
        VECTOR_DESTRUCT,
        NI_POINTER_GET,
        NI_POINTER_SET,
        NI_POINTER_DESTRUCTOR,
        NI_POINTER_NEW,
        REMOVE_TEXTURE,
        PLAYER_GETTER,
        DECAL_MANAGER_GETTER,
        AUDIO_GETTER,
        DECAL_BEGIN,
        DECAL_END,
        SOUND_STOP,
        SOUND_RELEASE,
        SOUND_DESTRUCTOR,
        SIMPLE_LIST_DESTRUCTOR,
        POINTER_LIST_DESTRUCTOR,
        STRING_DESTRUCTOR,
        MENU_BASE_DESTRUCTOR,
        POINTER_LIST_CLEAR,
        POINTER_LIST_ADD,
        NODE_ITEM,
        NODE_NEXT,
        POINTER_LIST_STEP,
        LIST_EMPTY,
        LIST_COUNT,
        LIST_PUSH,
        LIST_ADD,
        LIST_FREE_NODES,
        TILE_GET_VALUE,
        TILE_SET_VALUE,
        TILE_SET_STRING,
        TILE_IS_TRUE,
        TILE_IS_VISIBLE,
        TILE_UPDATE_ALL,
        TILE_MAXIMUM_DEPTH,
        TILE_GET_MENU_BY_CLASS,
        TILE_GET_MENU,
        TILE_READ_FILE,
        MENU_SET_MENU_TILE,
        TILE_SET_INT,
        TILE_PLAY_SEQUENCE,
        TILE_SHOW_FRAME,
        NODE_UPDATE,
        TILE_MODEL,
        UPDATE_DATA_NEW,
        TILE_SET_ID_VALUE,
        SETTING_VALUE_POINTER,
        SETTING_STRING,
        SETTING_FLOAT,
        SETTING_INT_POINTER,
        STRING_LENGTH,
        MEMSET,
        SNPRINTF,
        STRING_FORMAT,
        CHAR_CONVERT,
        FLOAT_TO_INT,
        CLAMP_FLOAT,
        FORM_TYPE,
        FORM_NAME,
        NAME_COMPARE,
        SET_PATH,
        SET_TEMPORARY,
        WORD_AT_8,
        IS_TEMPORARY,
        LOAD_SCREEN_NEW,
        LOAD_SCREEN_TYPE,
        LOAD_SCREEN_FOR_LOCATION,
        LOAD_SCREEN_GENERIC,
        LOAD_SCREEN_TYPE_FOLDED,
        RANDOM_INDEX,
        MINIMUM,
        CANDIDATES_RESET,
        CANDIDATE_AT,
        CANDIDATE_ADD,
        CANDIDATE_REMOVE,
        ALL_SCREENS_LIST,
        CANDIDATE_LIST_NEW,
        CANDIDATE_LIST_DESTRUCTOR,
        XUI_IS_UP,
        OVERLAY_TEST,
        SKIP_TEST,
        MENU_CLASS_GETTER,
        FLAG_1A8_TEST,
        SET_CURSOR_ALPHA,
        SET_MENU_MODE,
        SET_INFO_FOR_REF,
        SET_CROSSHAIR_TARGET_TYPE,
        SET_CURRENT_SPELL,
        PLAYER_BYTE,
        PLAYER_TILE,
        ROOT_TILE_GETTER,
        PLAYING_SEQUENCE,
        STOP_MOVIE,
        START_SEQUENCE,
        MOVIE_RELEASE,
        MENU_SHOW_CHANGES,
        LOCATION_FLAG,
        REFR_GET_INTERIOR,
        TEXTURE_LOAD,
        TEXTURE_APPLY,
        PROFILE_GUARD_NEW,
        PROFILE_GUARD_DESTRUCTOR,
        NODE_CHILD,
        NODE_PROPERTY,
        PROPERTY_ID,
        FIXED_STRING_NEW,
        FIXED_STRING_DESTRUCTOR,
        MODEL_SET_SCALE,
        MODEL_SET_ROTATION,
        MODEL_SHOW,
        LOG_LINE,
        REAL_SCREEN_WIDTH,
        REAL_SCREEN_HEIGHT,
        AUDIO_IS_ACTIVE,
        AUDIO_IS_MULTI_THREADED,
        AUDIO_SET_MULTI_THREADED,
        AUDIO_FLUSH,
        AUDIO_MUTE_TYPE,
        AUDIO_FADE,
        MUSIC_FADE_OUT,
        MUSIC_PLAY,
        MUSIC_UPDATE,
        MUSIC_REFRESH,
        CREDITS_SET_FLAG,
        SLEEP_TICK,
        VALUE_GETTER,
        TEXT_OWNER_NAME,
        TILE_SEQUENCE,
        TILE_TARGET,
        TILE_SEQUENCE_CALL,
        SEQUENCE_STATE,
        HIDE_ALL_BUT,
        FADE_STEP,
        SET_TILE_FADE,
        SUSPEND_BACKGROUND_THREAD,
        RESUME_BACKGROUND_THREAD,
        THREAD_NEW,
        REFRESH_STATS,
        INIT_ENTER,
        GROUP_VISIBLE_0078B6A0,
        GROUP_VISIBLE_0078B650,
        GROUP_VISIBLE_0078B6F0,
        GROUP_VISIBLE_0078B7C0,
        CALC_FACES,
        ANNOUNCE_SCREEN,
    ];

    /// An engine with the data the unit reads mapped, and doubles that record
    /// every call they receive, in order.
    struct Rig {
        e: Engine,
        calls: Calls,
        /// The object `DECAL_MANAGER_GETTER` returns (it has the default vtable).
        manager: u32,
    }

    fn rig() -> Rig {
        let mut e = Engine::new();
        e.map(0x0100_0000, 0x0010_0000);
        e.map(0x011a_0000, 0x0008_0000);
        e.map(0x0126_f000, 0x1000);
        let mut rig = Rig {
            e,
            calls: Rc::new(RefCell::new(Vec::new())),
            manager: 0,
        };
        rig.stubs(EXTERNAL);
        rig.e.map(0x7000, 0x1000);
        rig.stub(FORM_NAME, 0x7000);
        rig.stub(SETTING_STRING, 0x7000);
        rig.stub(SETTING_INT_POINTER, 0x7010);
        rig.stub(SETTING_VALUE_POINTER, 0x7020);
        let slots: Vec<u32> = (0..64).map(|i| VTABLE_TARGETS + 4 * i).collect();
        rig.e.put_vtable(VTABLE, &slots);
        for slot in &slots {
            rig.stub(*slot, 0);
        }
        let model = rig.object(0x100);
        rig.stub(TILE_MODEL, model);
        rig.stub(VTABLE_TARGETS + 0x9c, model);
        rig.manager = rig.object(0x100);
        rig.stub(DECAL_MANAGER_GETTER, rig.manager);
        rig.e.mem.set_f32(MINUS_ONE, -1.0);
        rig.e.mem.set_f32(HALF_PI, std::f32::consts::FRAC_PI_2);
        rig
    }

    fn eax(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn st0(value: f32) -> Ret {
        Ret {
            st0: value as f64,
            ..Ret::default()
        }
    }

    impl Rig {
        /// A double at `addr` that records its call and returns `f(args)`.
        fn with(&mut self, addr: u32, mut f: impl FnMut(&mut Engine, &[u32]) -> Ret + 'static) {
            let calls = self.calls.clone();
            self.e.register_double(addr, move |e, a| {
                calls.borrow_mut().push((addr, a.to_vec()));
                f(e, a)
            });
        }
        /// A double returning `value` in EAX.
        fn stub(&mut self, addr: u32, value: u32) {
            self.with(addr, move |_, _| eax(value));
        }
        /// Doubles returning 0 at every address in `addrs`.
        fn stubs(&mut self, addrs: &[u32]) {
            for addr in addrs {
                self.stub(*addr, 0);
            }
        }
        /// A double returning the `float` `value` in ST0.
        fn stub_float(&mut self, addr: u32, value: f32) {
            self.with(addr, move |_, _| st0(value));
        }
        /// The argument words of every recorded call to `addr`.
        fn calls_to(&self, addr: u32) -> Vec<Vec<u32>> {
            self.calls
                .borrow()
                .iter()
                .filter(|(a, _)| *a == addr)
                .map(|(_, args)| args.clone())
                .collect()
        }
        /// The addresses called, in order.
        fn order(&self) -> Vec<u32> {
            self.calls.borrow().iter().map(|(a, _)| *a).collect()
        }
        fn clear_calls(&self) {
            self.calls.borrow_mut().clear();
        }
        /// A zeroed menu.
        fn menu(&mut self) -> Ptr<LoadingMenu> {
            self.e.new_object::<LoadingMenu>()
        }
        /// The state word's bit test and set, kept in the menu's `usFlags`
        /// like the game does.
        fn real_flags(&mut self) {
            self.with(FLAG_TEST, |e, a| {
                let flags = e.mem.u16(a[0] + 0x222) as u32;
                eax(flags >> (a[1] & 31) & 1)
            });
            self.with(FLAG_SET, |e, a| {
                let flags = e.mem.u16(a[0] + 0x222) as u32;
                let bit = 1u32 << (a[1] & 31);
                let new = if a[2] as u8 != 0 {
                    flags | bit
                } else {
                    flags & !bit
                };
                e.mem.set_u16(a[0] + 0x222, new as u16);
                eax(0)
            });
        }
        /// Tick count source.
        fn clock(&mut self, ms: u32) {
            self.stub(TICK_COUNT, ms);
        }
        /// List node accessors: the item word address is the node itself, the
        /// next node is the word at `+4` (the `BSSimpleList` layout).
        fn real_nodes(&mut self) {
            self.with(NODE_ITEM, |_, a| eax(a[0]));
            self.with(NODE_NEXT, |e, a| eax(e.mem.u32(a[0] + 4)));
        }
        /// A `BSSimpleList` node holding `item`, chained to `next`.
        fn node(&mut self, item: u32, next: u32) -> u32 {
            let node = self.e.mem.alloc(8);
            self.e.mem.set_u32(node, item);
            self.e.mem.set_u32(node + 4, next);
            node
        }
        /// A `float` setting at `addr`'s `+4`.
        fn put_f64(&mut self, addr: u32, value: f64) {
            self.e.mem.set_f64(addr, value);
        }
    }

    #[test]
    fn every_translation_is_registered() {
        let e = Engine::new();
        for (addr, _) in funcs() {
            assert!(e.is_translated(addr), "{addr:08x}");
        }
        assert_eq!(funcs().len(), 40);
    }

    // ----- 00788730, 00788f20, 00788740 -----

    #[test]
    fn the_class_is_0x3ef() {
        let mut r = rig();
        let menu = r.menu();
        assert_eq!(r.e.call(0x0078_8730, &args![menu]).u32(), 0x3ef);
        assert_eq!(r.e.call(0x0078_8f20, &args![]).u32(), 0x3ef);
    }

    #[test]
    fn attaching_a_tile_stores_it_in_range() {
        let mut r = rig();
        let menu = r.menu();
        r.e.call(0x0078_8740, &args![menu, 3i32, 0x1234u32]);
        assert_eq!(r.e.mem.u32(menu.addr() + 0x28 + 12), 0x1234);
        r.e.call(0x0078_8740, &args![menu, 0x25i32, 0x77u32]);
        assert_eq!(r.e.mem.u32(menu.addr() + 0x28 + 0x25 * 4), 0x77);
    }

    #[test]
    fn attaching_ignores_null_tiles_and_ids_out_of_range() {
        let mut r = rig();
        let menu = r.menu();
        r.e.call(0x0078_8740, &args![menu, 1i32, 0u32]);
        r.e.call(0x0078_8740, &args![menu, 0x26i32, 5u32]);
        r.e.call(0x0078_8740, &args![menu, -1i32, 5u32]);
        assert_eq!(r.e.mem.u32(menu.addr() + 0x2c), 0);
        assert_eq!(r.e.mem.u32(menu.addr() + 0x28 + 0x26 * 4), 0);
        assert_eq!(r.e.mem.u32(menu.addr() + 0x24), 0);
    }

    // ----- 00788a90, 00788aa0 -----

    #[test]
    fn the_switch_byte_round_trips() {
        let mut r = rig();
        r.e.call(0x0078_8a90, &args![1u8]);
        assert_eq!(r.e.mem.u8(0x011d_cfb2), 1);
        assert_eq!(r.e.call(0x0078_8aa0, &args![]).u8(), 1);
        r.e.call(0x0078_8a90, &args![0u8]);
        assert_eq!(r.e.call(0x0078_8aa0, &args![]).u8(), 0);
    }

    // ----- 007887a0, 00788770 -----

    /// Doubles for everything the destructor calls outside the unit.
    fn destructor_rig() -> (Rig, Ptr<LoadingMenu>) {
        let mut r = rig();
        let menu = r.menu();
        r.real_flags();
        r.stubs(&[
            SUSPEND_BACKGROUND_THREAD,
            REMOVE_TEXTURE,
            NI_POINTER_SET,
            XUI_IS_UP,
            AUDIO_IS_ACTIVE,
            MUSIC_FADE_OUT,
            CREDITS_SET_FLAG,
            AUDIO_IS_MULTI_THREADED,
            AUDIO_SET_MULTI_THREADED,
            AUDIO_FADE,
            SOUND_STOP,
            SOUND_RELEASE,
            MOVIE_RELEASE,
            VECTOR_DESTRUCT,
            SOUND_DESTRUCTOR,
            SIMPLE_LIST_DESTRUCTOR,
            STRING_DESTRUCTOR,
            POINTER_LIST_DESTRUCTOR,
            MENU_BASE_DESTRUCTOR,
            LIST_FREE_NODES,
            OPERATOR_DELETE,
        ]);
        r.with(NI_POINTER_GET, |e, a| eax(e.mem.u32(a[0])));
        r.real_nodes();
        r.stub(AUDIO_GETTER, 0x5000);
        let manager = r.e.mem.alloc(0x100);
        r.e.mem.set_u32(manager, VTABLE);
        r.stub(DECAL_MANAGER_GETTER, manager);
        (r, menu)
    }

    #[test]
    fn the_destructor_releases_everything_and_clears_the_instance() {
        let (mut r, menu) = destructor_rig();
        // Two hold textures; the loading movie of the movie owner; the global
        // instance and initial-load flags.
        r.e.mem.set_u32(menu.addr() + 0x20c, 0x111);
        r.e.mem.set_u32(menu.addr() + 0x20c + 8, 0x222);
        r.e.mem.set_u32(MENU_INSTANCE, menu.addr());
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        r.e.mem.set_u8(MENU_REPLACED, 1);
        r.e.mem.set_u32(menu.addr() + 0x1e8, 0x99);
        let owner = r.e.mem.alloc(0xe50);
        r.e.mem.set_u32(owner + 0xe4c, 0x4242);
        r.e.mem.set_u32(MOVIE_OWNER, owner);
        r.e.mem.set_u32(menu.addr() + 0x1d0, 0);
        fn_007887a0(&mut r.e, menu);
        assert_eq!(r.e.mem.u32(menu.addr()), LOADING_MENU_VTABLE);
        assert_eq!(r.calls_to(REMOVE_TEXTURE), vec![vec![0x111], vec![0x222]]);
        assert_eq!(r.calls_to(NI_POINTER_SET).len(), 2);
        assert_eq!(r.e.mem.u8(MENU_REPLACED), 0);
        assert_eq!(r.e.mem.u8(INITIAL_LOAD), 0);
        assert_eq!(r.e.mem.u32(MENU_INSTANCE), 0);
        assert_eq!(r.e.mem.u32(menu.addr() + 0x1e8), 0);
        // The movie is released with 1 and cleared.
        assert_eq!(r.calls_to(MOVIE_RELEASE), vec![vec![0x4242, 1]]);
        assert_eq!(r.e.mem.u32(owner + 0xe4c), 0);
        // Members are destroyed before the base, which comes last.
        let order = r.order();
        assert_eq!(order[0], SUSPEND_BACKGROUND_THREAD);
        assert_eq!(*order.last().unwrap(), MENU_BASE_DESTRUCTOR);
        assert_eq!(
            r.calls_to(POINTER_LIST_DESTRUCTOR)
                .iter()
                .map(|a| a[0] - menu.addr())
                .collect::<Vec<_>>(),
            vec![0x100, 0xf4, 0xe8, 0xdc, 0xd0]
        );
        assert_eq!(r.calls_to(SOUND_RELEASE).len(), 2);
        assert_eq!(
            r.calls_to(VECTOR_DESTRUCT),
            vec![vec![menu.addr() + 0x20c, 4, 4, NI_POINTER_DESTRUCTOR]]
        );
    }

    #[test]
    fn the_destructor_stops_the_music_only_when_it_is_playing() {
        // Nothing up and not the initial load: no music stop at 4000.
        let (mut r, menu) = destructor_rig();
        fn_007887a0(&mut r.e, menu);
        assert!(r.calls_to(MUSIC_FADE_OUT).is_empty());
        // XUI up and the audio active: it stops the music.
        let (mut r, menu) = destructor_rig();
        r.stub(XUI_IS_UP, 1);
        r.stub(AUDIO_IS_ACTIVE, 1);
        fn_007887a0(&mut r.e, menu);
        assert_eq!(r.calls_to(MUSIC_FADE_OUT), vec![vec![4000, 0]]);
        // Initial load with a nonzero volume character behaves the same.
        let (mut r, menu) = destructor_rig();
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        r.e.mem.set_u8(0x7000, b'1');
        r.stub(AUDIO_IS_ACTIVE, 1);
        fn_007887a0(&mut r.e, menu);
        assert_eq!(r.calls_to(MUSIC_FADE_OUT).len(), 1);
        // The switch byte stops it too and is cleared.
        let (mut r, menu) = destructor_rig();
        r.e.mem.set_u8(SWITCH_BYTE, 1);
        fn_007887a0(&mut r.e, menu);
        assert_eq!(r.calls_to(MUSIC_FADE_OUT), vec![vec![4000, 0]]);
        assert_eq!(r.calls_to(CREDITS_SET_FLAG), vec![vec![0]]);
        assert_eq!(r.e.mem.u8(SWITCH_BYTE), 0);
    }

    #[test]
    fn the_destructor_undoes_the_multi_threaded_audio_it_started() {
        let (mut r, menu) = destructor_rig();
        r.e.mem.set_u16(menu.addr() + 0x222, 1 << 2);
        r.stub(AUDIO_IS_MULTI_THREADED, 1);
        fn_007887a0(&mut r.e, menu);
        assert_eq!(r.calls_to(AUDIO_SET_MULTI_THREADED), vec![vec![0x5000, 0]]);
        let (mut r, menu) = destructor_rig();
        fn_007887a0(&mut r.e, menu);
        assert!(r.calls_to(AUDIO_SET_MULTI_THREADED).is_empty());
    }

    #[test]
    fn the_scalar_deleting_destructor_frees_only_when_asked() {
        let (mut r, menu) = destructor_rig();
        let result = r.e.call(0x0078_8770, &args![menu, 0u32]);
        assert_eq!(result.u32(), menu.addr());
        assert!(r.calls_to(OPERATOR_DELETE).is_empty());
        let result = r.e.call(0x0078_8770, &args![menu, 1u32]);
        assert_eq!(result.u32(), menu.addr());
        assert_eq!(r.calls_to(OPERATOR_DELETE), vec![vec![menu.addr()]]);
    }

    impl Rig {
        /// A zeroed block with the default vtable.
        fn object(&mut self, size: u32) -> u32 {
            let object = self.e.mem.alloc(size);
            self.e.mem.set_u32(object, VTABLE);
            object
        }
        /// A load screen: its sub-object at `+0x24` has the default vtable.
        fn screen(&mut self) -> u32 {
            let screen = self.e.mem.alloc(0x40);
            self.e.mem.set_u32(screen, VTABLE);
            self.e.mem.set_u32(screen + 0x24, VTABLE);
            screen
        }
        /// A menu with every tile slot filled by a tile object, a `LoadScreens`
        /// list of two screens (the head node is embedded at `+0x1d0`) and
        /// `pNextLoadScreen`/`pCurrentLoadScreen` on the first. Returns the
        /// menu; the screens are at the items of the list.
        fn full_menu(&mut self) -> Ptr<LoadingMenu> {
            let menu = self.menu();
            for offset in (0x28..0xc0).step_by(4) {
                let tile = self.object(0x100);
                self.e.mem.set_u32(menu.addr() + offset, tile);
            }
            let first = self.screen();
            let second = self.screen();
            let tail = self.node(second, 0);
            self.e.mem.set_u32(menu.addr() + 0x1d0, first);
            self.e.mem.set_u32(menu.addr() + 0x1d4, tail);
            self.e.mem.set_u32(menu.addr() + 0x1d8, menu.addr() + 0x1d0);
            self.e.mem.set_u32(menu.addr() + 0x1dc, first);
            self.real_nodes();
            menu
        }
        /// The tile object in slot `offset`.
        fn tile_at(&self, menu: Ptr<LoadingMenu>, offset: u32) -> u32 {
            self.e.mem.u32(menu.addr() + offset)
        }
    }

    fn world() -> (Rig, Ptr<LoadingMenu>) {
        let mut r = rig();
        r.real_flags();
        r.clock(10_000);
        let menu = r.full_menu();
        (r, menu)
    }

    fn bit(n: u32) -> u16 {
        1 << n
    }

    // ----- 00788ab0 -----

    #[test]
    fn re_showing_does_nothing_when_bit_3_is_clear() {
        let (mut r, menu) = world();
        fn_00788ab0(&mut r.e, menu, 0x55);
        assert!(r.calls_to(SUSPEND_BACKGROUND_THREAD).is_empty());
        assert!(r.calls_to(RESUME_BACKGROUND_THREAD).is_empty());
        assert_eq!(r.e.get(menu, LoadingMenu::pLocation).addr(), 0);
    }

    #[test]
    fn re_showing_restarts_the_fade_and_steps_two_slides() {
        let (mut r, menu) = world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(3));
        fn_00788ab0(&mut r.e, menu, 0x55);
        assert_eq!(r.e.get(menu, LoadingMenu::m_TileCrossFadeStartTime), 6000);
        assert_eq!(r.e.get(menu, LoadingMenu::pLocation).addr(), 0x55);
        assert_eq!(r.e.get(menu, LoadingMenu::m_SlideCount), 2);
        assert_eq!(r.e.get(menu, LoadingMenu::usFlags) & bit(3), 0);
        assert_eq!(r.e.mem.u8(DO_TILE_UPDATE), 1);
        let order = r.order();
        assert_eq!(order[0], FLAG_TEST);
        assert_eq!(order[1], SUSPEND_BACKGROUND_THREAD);
        assert_eq!(*order.last().unwrap(), RESUME_BACKGROUND_THREAD);
        assert_eq!(r.calls_to(TILE_UPDATE_ALL), vec![vec![1]]);
        // The hidden wheel is made visible (value 0xfa3 = 1); the player's
        // tile gets 0.
        let wheel = r.tile_at(menu, SLOT_WHEEL);
        let sets = r.calls_to(TILE_SET_INT);
        assert_eq!(sets[0], vec![wheel, 0xfa3, 1]);
        assert_eq!(sets.last().unwrap()[1..], [0xfa3, 0]);
        // Not in overlay mode: the tips are enabled and the status tiles set.
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(6), 0);
        assert_eq!(r.calls_to(REFRESH_STATS).len(), 1);
    }

    #[test]
    fn re_showing_in_overlay_mode_disables_tips() {
        let (mut r, menu) = world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(3) | bit(6));
        r.stub(OVERLAY_TEST, 1);
        fn_00788ab0(&mut r.e, menu, 0);
        assert!(r.calls_to(REFRESH_STATS).is_empty());
        // Bit 6 was cleared by 00788afa0(0) and is not set again, but the
        // list rebuild sets bit 5 (overlay).
        assert_eq!(r.e.get(menu, LoadingMenu::usFlags) & bit(6), 0);
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(5), 0);
    }

    #[test]
    fn re_showing_deletes_the_stats_menu_when_it_is_replaced() {
        let (mut r, menu) = world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(3));
        r.e.mem.set_u8(MENU_REPLACED, 1);
        r.stub(MENU_CLASS_GETTER, 0x3f5);
        r.stub(TILE_GET_MENU_BY_CLASS, 0x300);
        let stats = r.object(0x40);
        r.stub(TILE_GET_MENU, stats);
        fn_00788ab0(&mut r.e, menu, 0);
        assert_eq!(r.calls_to(TILE_GET_MENU_BY_CLASS), vec![vec![0x3f5]]);
        assert_eq!(r.calls_to(TILE_GET_MENU), vec![vec![0x300]]);
        // Virtual slot 0 (the destructor) with 1.
        assert_eq!(r.calls_to(VTABLE_TARGETS), vec![vec![stats, 1]]);
        // A menu with flag 1 set in +0x1a8 is left alone.
        let (mut r, menu) = world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(3));
        r.e.mem.set_u8(MENU_REPLACED, 1);
        let stats = r.object(0x40);
        r.stub(TILE_GET_MENU, stats);
        r.stub(FLAG_1A8_TEST, 1);
        fn_00788ab0(&mut r.e, menu, 0);
        assert!(r.calls_to(VTABLE_TARGETS).is_empty());
    }

    // ----- 00788cc0 -----

    #[test]
    fn creating_when_a_menu_exists_re_shows_it() {
        let (mut r, menu) = world();
        r.e.mem.set_u32(MENU_INSTANCE, menu.addr());
        r.stub(TILE_GET_MENU_BY_CLASS, 0x321);
        let result = loading_menu_create(&mut r.e, 0x77, 1, 1);
        assert_eq!(result, 0x321);
        assert_eq!(r.calls_to(TILE_GET_MENU_BY_CLASS)[0], vec![0x3ef]);
        // Bit 3 was set for the re-show (which clears it), bit 10 takes the
        // last argument; the location reached the menu.
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(10), 0);
        assert_eq!(r.e.get(menu, LoadingMenu::pLocation).addr(), 0x77);
        assert_eq!(r.calls_to(SUSPEND_BACKGROUND_THREAD).len(), 1);
        assert!(r.calls_to(TILE_READ_FILE).is_empty());
    }

    /// A rig for building the menu from the XML: the file read returns a root
    /// tile whose menu is a loading menu.
    fn fresh_world() -> (Rig, Ptr<LoadingMenu>, u32) {
        let (mut r, menu) = world();
        let root = r.object(0x40);
        r.stub(TILE_GET_MENU_BY_CLASS, 0);
        r.stub(PLAYER_GETTER, 0x9100);
        r.stub(ROOT_TILE_GETTER, 0x9200);
        r.stub(TILE_READ_FILE, root);
        r.stub(TILE_GET_MENU, menu.addr());
        r.e.mem.set_u32(menu.addr(), VTABLE);
        r.stub(VTABLE_TARGETS + 0x34, 0x3ef);
        (r, menu, root)
    }

    #[test]
    fn creating_reads_the_menu_file_and_builds_the_menu() {
        let (mut r, menu, root) = fresh_world();
        r.stub(OPERATOR_NEW, 0x9300);
        r.stub(THREAD_NEW, 0x9400);
        let result = loading_menu_create(&mut r.e, 0x77, 0, 1);
        assert_eq!(result, root);
        assert_eq!(
            r.calls_to(TILE_READ_FILE),
            vec![vec![0x9200, MENU_FILE_PATH]]
        );
        assert_eq!(r.e.mem.u8(READING_MENU_FILE), 0);
        assert_eq!(r.e.mem.u32(MENU_INSTANCE), menu.addr());
        assert_eq!(
            r.calls_to(MENU_SET_MENU_TILE),
            vec![vec![menu.addr(), root, 0]]
        );
        assert_eq!(r.calls_to(SET_CURSOR_ALPHA), vec![vec![0]]);
        // The four tile lists were filled, the four groups hidden, _Create
        // ran (the location is stored), and the thread was created.
        assert_eq!(r.calls_to(POINTER_LIST_CLEAR).len(), 5);
        assert_eq!(r.calls_to(GROUP_VISIBLE_0078B6A0)[0], vec![menu.addr(), 0]);
        assert_eq!(r.calls_to(GROUP_VISIBLE_0078B650)[0], vec![menu.addr(), 0]);
        assert_eq!(r.calls_to(GROUP_VISIBLE_0078B6F0)[0], vec![menu.addr(), 0]);
        assert_eq!(r.calls_to(GROUP_VISIBLE_0078B7C0)[0], vec![menu.addr(), 0]);
        assert_eq!(r.calls_to(OPERATOR_NEW), vec![vec![0x4c]]);
        assert_eq!(r.calls_to(THREAD_NEW), vec![vec![0x9300]]);
        assert_eq!(r.e.mem.u32(LOADING_THREAD), 0x9400);
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(10), 0);
        // Both fade tiles start at 0.
        let fade_a = r.tile_at(menu, SLOT_FADE_A);
        let fade_b = r.tile_at(menu, SLOT_FADE_B);
        let sets = r.calls_to(TILE_SET_INT);
        assert!(sets.contains(&vec![fade_a, 0xfa9, 0]));
        assert!(sets.contains(&vec![fade_b, 0xfa9, 0]));
        assert!(r.calls_to(RESUME_BACKGROUND_THREAD).is_empty());
    }

    #[test]
    fn creating_resumes_an_existing_thread_only_when_asked() {
        let (mut r, _menu, _root) = fresh_world();
        r.e.mem.set_u32(LOADING_THREAD, 0x9400);
        loading_menu_create(&mut r.e, 0, 0, 0);
        assert!(r.calls_to(RESUME_BACKGROUND_THREAD).is_empty());
        assert!(r.calls_to(THREAD_NEW).is_empty());
        let (mut r, _menu, _root) = fresh_world();
        r.e.mem.set_u32(LOADING_THREAD, 0x9400);
        loading_menu_create(&mut r.e, 0, 1, 0);
        assert_eq!(r.calls_to(RESUME_BACKGROUND_THREAD).len(), 1);
        assert_eq!(r.e.mem.u32(LOADING_THREAD), 0x9400);
    }

    #[test]
    fn a_failed_allocation_leaves_the_thread_null() {
        let (mut r, _menu, _root) = fresh_world();
        r.stub(OPERATOR_NEW, 0);
        loading_menu_create(&mut r.e, 0, 0, 0);
        assert!(r.calls_to(THREAD_NEW).is_empty());
        assert_eq!(r.e.mem.u32(LOADING_THREAD), 0);
    }

    #[test]
    fn a_file_with_another_menu_fails_and_deletes_it() {
        let (mut r, menu, _root) = fresh_world();
        r.stub(VTABLE_TARGETS + 0x34, 0x3f0);
        r.with(NODE_NEXT, |_, _| eax(0x77));
        let result = loading_menu_create(&mut r.e, 0, 0, 0);
        assert_eq!(result, 0);
        assert!(r.calls_to(MENU_SET_MENU_TILE).is_empty());
        // Slot 0 (destructor) with 1, because the menu has a tile (nonzero
        // `pMenu`-style word from 00726070).
        assert_eq!(r.calls_to(VTABLE_TARGETS), vec![vec![menu.addr(), 1]]);
        // Without a tile it is not deleted.
        let (mut r, _menu, _root) = fresh_world();
        r.stub(VTABLE_TARGETS + 0x34, 0x3f0);
        r.stub(NODE_NEXT, 0);
        assert_eq!(loading_menu_create(&mut r.e, 0, 0, 0), 0);
        assert!(r.calls_to(VTABLE_TARGETS).is_empty());
    }

    #[test]
    fn a_file_without_a_menu_fails() {
        let (mut r, _menu, _root) = fresh_world();
        r.stub(TILE_GET_MENU, 0);
        assert_eq!(loading_menu_create(&mut r.e, 0, 0, 0), 0);
        assert!(r.calls_to(MENU_SET_MENU_TILE).is_empty());
    }

    // ----- 00788f30, 00788f80, 00788fe0, 00789080 -----

    fn list_adds(r: &Rig, list: u32) -> Vec<u32> {
        r.calls_to(POINTER_LIST_ADD)
            .iter()
            .filter(|a| a[0] == list)
            .map(|a| a[1])
            .collect()
    }

    #[test]
    fn the_tile_lists_hold_the_slots_by_address() {
        let (mut r, menu) = world();
        let base = menu.addr();
        fn_00788f30(&mut r.e, menu);
        assert_eq!(list_adds(&r, base + 0xd0), vec![base + 0x6c, base + 0x70]);
        fn_00788f80(&mut r.e, menu);
        assert_eq!(
            list_adds(&r, base + 0xdc),
            vec![base + 0x94, base + 0x34, base + 0x60]
        );
        fn_00788fe0(&mut r.e, menu);
        assert_eq!(
            list_adds(&r, base + 0xe8),
            vec![
                base + 0x68,
                base + 0x98,
                base + 0x9c,
                base + 0x64,
                base + 0x50,
                base + 0xa0
            ]
        );
        fn_00789080(&mut r.e, menu);
        assert_eq!(list_adds(&r, base + 0xf4), vec![base + 0xb0]);
        assert_eq!(list_adds(&r, base + 0x100), vec![base + 0xb4]);
        // Each list is cleared first.
        let cleared: Vec<u32> = r
            .calls_to(POINTER_LIST_CLEAR)
            .iter()
            .map(|a| a[0] - base)
            .collect();
        assert_eq!(cleared, vec![0xd0, 0xdc, 0xe8, 0xf4, 0x100]);
        assert_eq!(r.order()[0], POINTER_LIST_CLEAR);
    }

    // ----- 007890e0 -----

    /// A world for `_Create`: the flag and node accessors are real, the game
    /// time is 10000 and two floats are set the way the settings give them.
    fn impl_world() -> (Rig, Ptr<LoadingMenu>) {
        let (mut r, menu) = world();
        r.put_f64(MENU_MODE_A, 6006.0);
        r.put_f64(MENU_MODE_B, 102.0);
        r.stub_float(SETTING_FLOAT, 2.5);
        r.stub_float(TILE_MAXIMUM_DEPTH, 7.5);
        r.e.mem.set_f32(0x7020, 3.25);
        (r, menu)
    }

    fn set_values(r: &mut Rig, values: Vec<(u32, f32)>) {
        r.with(TILE_GET_VALUE, move |_, a| {
            st0(values
                .iter()
                .find(|(id, _)| *id == a[1])
                .map_or(0.0, |v| v.1))
        });
    }

    #[test]
    fn create_fixes_the_depth_for_the_two_menu_modes() {
        for mode in [6006.0, 102.0] {
            let (mut r, menu) = impl_world();
            set_values(&mut r, vec![(0xfa7, mode)]);
            loading_menu_create_impl(&mut r.e, menu, 0);
            let pm = r.e.get(menu, LoadingMenu::pMenu).addr();
            assert!(r
                .calls_to(TILE_SET_VALUE)
                .contains(&vec![pm, 0xfad, 7.5f32.to_bits(), 1]));
        }
        let (mut r, menu) = impl_world();
        set_values(&mut r, vec![(0xfa7, 5.0)]);
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert!(r.calls_to(TILE_MAXIMUM_DEPTH).is_empty());
    }

    #[test]
    fn create_sets_the_spell_unless_the_player_byte_is_3() {
        let (mut r, menu) = impl_world();
        r.stub(PLAYER_GETTER, 0x9100);
        r.stub(PLAYER_BYTE, 3);
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert!(r.calls_to(SET_CURRENT_SPELL).is_empty());
        let (mut r, menu) = impl_world();
        r.stub(PLAYER_GETTER, 0x9100);
        r.stub(PLAYER_BYTE, 4);
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert_eq!(r.calls_to(SET_CURRENT_SPELL), vec![vec![0x9100, 2]]);
        assert_eq!(r.calls_to(SET_MENU_MODE), vec![vec![5]]);
        assert_eq!(r.calls_to(SET_INFO_FOR_REF), vec![vec![0, 0, 0]]);
        assert_eq!(r.calls_to(SET_CROSSHAIR_TARGET_TYPE), vec![vec![0]]);
    }

    #[test]
    fn create_stops_the_movie_unless_a_sequence_plays() {
        let (mut r, menu) = impl_world();
        r.e.mem.set_u32(MOVIE_OWNER, 0x9300);
        r.e.mem.set_u32(MOVIE_PLAYER, 0x9400);
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert_eq!(r.calls_to(STOP_MOVIE), vec![vec![0x9400]]);
        let (mut r, menu) = impl_world();
        r.e.mem.set_u32(MOVIE_OWNER, 0x9300);
        r.e.mem.set_u32(MOVIE_PLAYER, 0x9400);
        r.stub(PLAYING_SEQUENCE, 1);
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert!(r.calls_to(STOP_MOVIE).is_empty());
        // No movie owner: no query at all.
        let (mut r, menu) = impl_world();
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert!(r.calls_to(PLAYING_SEQUENCE).is_empty());
    }

    #[test]
    fn create_closes_the_replaced_menu() {
        let (mut r, menu) = impl_world();
        let other = r.object(0x40);
        r.with(TILE_GET_MENU_BY_CLASS, move |_, a| {
            eax(if a[0] == 0x3f4 { other } else { 0 })
        });
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert_eq!(r.calls_to(VTABLE_TARGETS), vec![vec![other, 1]]);
    }

    #[test]
    fn create_hides_everything_on_the_initial_load() {
        let (mut r, menu) = impl_world();
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert_eq!(r.calls_to(HIDE_ALL_BUT), vec![vec![menu.addr(), 0, 0]]);
        assert_eq!(r.e.get(menu, LoadingMenu::pLocation).addr(), 0x44);
        // No slide setup: the list was not rebuilt.
        assert!(r.calls_to(CANDIDATES_RESET).is_empty());
    }

    #[test]
    fn create_hides_everything_when_the_value_is_nonzero() {
        let (mut r, menu) = impl_world();
        r.stub_float(VALUE_GETTER, 1.0);
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert_eq!(r.calls_to(HIDE_ALL_BUT), vec![vec![menu.addr(), 0, 0]]);
    }

    #[test]
    fn create_with_a_replaced_menu_disables_tips_and_updates_decals() {
        let (mut r, menu) = impl_world();
        r.e.mem.set_u8(MENU_REPLACED, 1);
        r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert!(r.calls_to(HIDE_ALL_BUT).is_empty());
        assert_eq!(r.e.get(menu, LoadingMenu::usFlags) & bit(6), 0);
        assert_eq!(r.calls_to(DECAL_BEGIN), vec![vec![r.manager]]);
        assert_eq!(r.calls_to(DECAL_END), vec![vec![r.manager]]);
        // The wheel tile shows frame -1.
        let wheel = r.tile_at(menu, SLOT_WHEEL);
        assert!(r
            .calls_to(TILE_SHOW_FRAME)
            .contains(&vec![wheel, (-1.0f32).to_bits(), 0]));
    }

    fn location_world(form_type: u32, interior: u32) -> (Rig, Ptr<LoadingMenu>) {
        let (mut r, menu) = impl_world();
        r.stub(FORM_TYPE, form_type);
        r.stub(LOCATION_FLAG, interior);
        r.stub(REFR_GET_INTERIOR, interior);
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        (r, menu)
    }

    #[test]
    fn an_interior_cell_location_hides_all_but_the_interior_tiles() {
        let (r, menu) = location_world(0x39, 1);
        assert_eq!(r.calls_to(LOCATION_FLAG), vec![vec![0x44]]);
        assert!(r.calls_to(REFR_GET_INTERIOR).is_empty());
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(9), 0);
        assert_eq!(r.calls_to(HIDE_ALL_BUT), vec![vec![menu.addr(), 0xb, 0]]);
        assert_eq!(r.e.get(menu, LoadingMenu::usFlags) & bit(4), 0);
        assert_eq!(r.e.get(menu, LoadingMenu::uiFadeInComplete), 10_000);
        // The slide setup is skipped for interiors.
        assert!(r.calls_to(CANDIDATES_RESET).is_empty());
    }

    #[test]
    fn reference_form_types_ask_the_reference_for_its_interior() {
        for form_type in [0x3a, 0x3b, 0x40, 0x69] {
            let (r, menu) = location_world(form_type, 1);
            assert_eq!(
                r.calls_to(REFR_GET_INTERIOR),
                vec![vec![0x44]],
                "{form_type:x}"
            );
            assert!(r.calls_to(LOCATION_FLAG).is_empty());
            assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(9), 0);
        }
    }

    #[test]
    fn other_form_types_leave_the_layout_alone() {
        for form_type in [0x38, 0x41, 0x68, 0x6a, 0] {
            let (r, menu) = location_world(form_type, 1);
            assert!(r.calls_to(REFR_GET_INTERIOR).is_empty(), "{form_type:x}");
            assert!(r.calls_to(LOCATION_FLAG).is_empty());
            assert_eq!(r.e.get(menu, LoadingMenu::usFlags) & bit(9), 0);
            // Exterior layout: tips on, slides set up.
            assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(6), 0);
            assert_eq!(r.calls_to(REFRESH_STATS).len(), 1);
            assert!(!r.calls_to(CANDIDATES_RESET).is_empty());
        }
    }

    #[test]
    fn the_skip_test_ignores_the_location_form_type() {
        let (mut r, menu) = impl_world();
        r.stub(SKIP_TEST, 1);
        r.e.mem.set_u32(SKIP_OWNER, 0x9600);
        r.stub(FORM_TYPE, 0x39);
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert!(r.calls_to(FORM_TYPE).is_empty());
        assert_eq!(r.e.get(menu, LoadingMenu::usFlags) & bit(9), 0);
    }

    #[test]
    fn the_wheel_gets_its_scale_and_rotation() {
        let (mut r, menu) = impl_world();
        r.e.mem.set_f32(HALF_PI, 1.5);
        let model = r.object(0x100);
        r.stub(TILE_MODEL, model);
        r.stub(SETTING_VALUE_POINTER, 0x7020);
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        let wheel = r.tile_at(menu, SLOT_WHEEL);
        assert!(r
            .calls_to(MODEL_SET_SCALE)
            .contains(&vec![model, 2.5f32.to_bits()]));
        assert!(r
            .calls_to(TILE_SET_VALUE)
            .contains(&vec![wheel, 0x1004, 3.25f32.to_bits(), 1]));
        assert!(r.calls_to(MODEL_SET_ROTATION).contains(&vec![
            model,
            1.5f32.to_bits(),
            1.0f32.to_bits(),
            0,
            0
        ]));
        assert!(r.calls_to(LOG_LINE).is_empty());
    }

    #[test]
    fn a_wheel_without_a_model_is_reported() {
        let (mut r, menu) = impl_world();
        let model = r.object(0x100);
        let wheel = r.tile_at(menu, SLOT_WHEEL);
        r.with(TILE_MODEL, move |_, a| {
            eax(if a[0] == wheel { 0 } else { model })
        });
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert_eq!(r.calls_to(LOG_LINE), vec![vec![0x12, WHEEL_WARNING]]);
        // The wheel was not scaled.
        assert!(!r
            .calls_to(MODEL_SET_SCALE)
            .iter()
            .any(|a| a[1] == 2.5f32.to_bits()));
    }

    #[test]
    fn slides_that_are_nodes_take_the_property_of_their_first_child() {
        let (mut r, menu) = impl_world();
        let names: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
        let seen = names.clone();
        r.with(FIXED_STRING_NEW, move |_, a| {
            seen.borrow_mut().push(a[1]);
            eax(a[0])
        });
        let slide = r.object(0x40);
        r.stub(VTABLE_TARGETS + 0x9c, slide);
        // The slide casts to a node.
        r.stub(VTABLE_TARGETS + 0x0c, 0x9500);
        r.stub(PROPERTY_ID, 3);
        r.stub(NODE_CHILD, 0x9600);
        r.with(NODE_PROPERTY, |_, a| eax(a[0] + a[1]));
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert_eq!(*names.borrow(), vec![SLIDE_ONE, SLIDE_TWO]);
        assert_eq!(
            r.calls_to(NODE_CHILD),
            vec![vec![0x9500, 0], vec![slide, 0]]
        );
        assert_eq!(r.e.get(menu, LoadingMenu::pSlide1TSP).addr(), 0x9603);
        assert_eq!(r.e.get(menu, LoadingMenu::pSlide2TSP).addr(), 0x9603);
        assert_eq!(r.calls_to(FIXED_STRING_DESTRUCTOR).len(), 2);
    }

    #[test]
    fn slides_that_are_not_nodes_take_their_own_property() {
        let (mut r, menu) = impl_world();
        let first = r.object(0x40);
        let second = r.object(0x40);
        let answers = Rc::new(RefCell::new(vec![second, first]));
        r.with(VTABLE_TARGETS + 0x9c, move |_, _| {
            eax(answers.borrow_mut().pop().unwrap())
        });
        r.stub(VTABLE_TARGETS + 0x0c, 0);
        r.stub(PROPERTY_ID, 3);
        r.with(NODE_PROPERTY, |_, a| eax(a[0] + a[1]));
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert!(r.calls_to(NODE_CHILD).is_empty());
        assert_eq!(r.e.get(menu, LoadingMenu::pSlide1TSP).addr(), first + 3);
        assert_eq!(r.e.get(menu, LoadingMenu::pSlide2TSP).addr(), second + 3);
    }

    #[test]
    fn create_finishes_the_slide_setup() {
        let (mut r, menu) = impl_world();
        r.stub_float(REAL_SCREEN_WIDTH, 1280.0);
        r.stub_float(REAL_SCREEN_HEIGHT, 720.0);
        r.e.mem.set_f32(SLIDE_SCALE, 0.5);
        let model_tile = r.tile_at(menu, SLOT_MODEL);
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        let ratio = (1280.0f64 / 720.0) as f32;
        assert!(r
            .calls_to(TILE_SET_VALUE)
            .contains(&vec![model_tile, 0x1004, ratio.to_bits(), 1]));
        assert_eq!(
            r.calls_to(TILE_PLAY_SEQUENCE),
            vec![vec![model_tile, SEQUENCE_INTRO]]
        );
        assert_eq!(r.calls_to(UPDATE_DATA_NEW).len(), 1);
        assert_eq!(r.calls_to(UPDATE_DATA_NEW)[0][1..], [0, 1, 0]);
        assert_eq!(r.calls_to(NODE_UPDATE).len(), 1);
        assert_eq!(r.calls_to(FADE_STEP), vec![vec![menu.addr()]]);
        assert_eq!(r.e.get(menu, LoadingMenu::uiFadeInComplete), 0);
        assert_eq!(r.calls_to(MENU_SHOW_CHANGES).len(), 1);
        assert_eq!(r.e.get(menu, LoadingMenu::m_TileCrossFadeStartTime), 0);
        // The model is shown last.
        assert_eq!(*r.order().last().unwrap(), MODEL_SHOW);
        // Not the initial load: the list is built, then the first slide.
        assert!(!r.calls_to(CANDIDATES_RESET).is_empty());
        assert_eq!(r.e.get(menu, LoadingMenu::m_SlideCount), 0);
    }

    #[test]
    fn create_starts_multi_threaded_audio_on_the_initial_load() {
        let (mut r, menu) = impl_world();
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        r.stub(AUDIO_GETTER, 0x5000);
        r.stub(AUDIO_IS_ACTIVE, 1);
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert_eq!(r.calls_to(AUDIO_SET_MULTI_THREADED), vec![vec![0x5000, 1]]);
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(2), 0);
        // The initial-load path never mutes.
        assert!(r.calls_to(AUDIO_MUTE_TYPE).is_empty());
    }

    #[test]
    fn create_leaves_the_audio_alone_when_the_music_setting_is_set() {
        let (mut r, menu) = impl_world();
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        r.e.mem.set_u8(0x7000, b'x');
        r.stub(AUDIO_GETTER, 0x5000);
        r.stub(AUDIO_IS_ACTIVE, 1);
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert!(r.calls_to(AUDIO_SET_MULTI_THREADED).is_empty());
        assert!(r.calls_to(AUDIO_MUTE_TYPE).is_empty());
    }

    #[test]
    fn create_mutes_audio_type_2_for_a_location_load() {
        let (mut r, menu) = impl_world();
        r.stub(AUDIO_GETTER, 0x5000);
        r.stub(AUDIO_IS_ACTIVE, 1);
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert_eq!(r.calls_to(AUDIO_SET_MULTI_THREADED), vec![vec![0x5000, 1]]);
        assert_eq!(
            r.calls_to(AUDIO_MUTE_TYPE),
            vec![vec![0x5000, 2, 1000, 6000]]
        );
        // Already multi-threaded: it is not switched again, still muted.
        let (mut r, menu) = impl_world();
        r.stub(AUDIO_GETTER, 0x5000);
        r.stub(AUDIO_IS_ACTIVE, 1);
        r.stub(AUDIO_IS_MULTI_THREADED, 1);
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert!(r.calls_to(AUDIO_SET_MULTI_THREADED).is_empty());
        assert_eq!(r.calls_to(AUDIO_MUTE_TYPE).len(), 1);
        // Inactive audio: nothing.
        let (mut r, menu) = impl_world();
        loading_menu_create_impl(&mut r.e, menu, 0x44);
        assert!(r.calls_to(AUDIO_MUTE_TYPE).is_empty());
    }

    #[test]
    fn create_with_no_location_takes_the_first_audio_branch() {
        let (mut r, menu) = impl_world();
        r.stub(AUDIO_GETTER, 0x5000);
        r.stub(AUDIO_IS_ACTIVE, 1);
        loading_menu_create_impl(&mut r.e, menu, 0);
        assert_eq!(r.calls_to(AUDIO_SET_MULTI_THREADED), vec![vec![0x5000, 1]]);
        assert!(r.calls_to(AUDIO_MUTE_TYPE).is_empty());
    }

    // ----- 00789820 -----

    /// A world for `Update`: time 10000, real clamp and float truncation,
    /// and the fade constants (0.0005 per ms, 255, 15, 30, 1000).
    fn update_world() -> (Rig, Ptr<LoadingMenu>) {
        let (mut r, menu) = world();
        r.with(CLAMP_FLOAT, |e, a| {
            let value = e.mem.f32(a[0]);
            let clamped = value.clamp(f32::from_bits(a[1]), f32::from_bits(a[2]));
            e.mem.set_f32(a[0], clamped);
            eax(0)
        });
        r.with(FLOAT_TO_INT, |_, a| {
            let value = f64::from_bits(a[0] as u64 | (a[1] as u64) << 32);
            eax(value as i32 as u32)
        });
        r.put_f64(FADE_PER_MS, 0.0005);
        r.put_f64(BYTE_SCALE, 255.0);
        r.put_f64(MATTE_OFFSET_DOWN, 15.0);
        r.put_f64(MATTE_OFFSET_UP, 30.0);
        r.put_f64(MS_PER_SECOND, 1000.0);
        r.stub(AUDIO_GETTER, 0x5000);
        (r, menu)
    }

    /// Flags that keep `Update` out of the fade and swap code: bit 4 set, a
    /// swap time in the future.
    fn quiet(r: &mut Rig, menu: Ptr<LoadingMenu>) {
        r.e.mem.set_u16(menu.addr() + 0x222, bit(4));
        r.e.set(menu, LoadingMenu::uiNextBackgroundSwap, 50_000);
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 1);
    }

    #[test]
    fn update_copies_the_tip_values_to_the_matte_tiles() {
        let (mut r, menu) = update_world();
        quiet(&mut r, menu);
        r.with(TILE_GET_VALUE, |_, a| {
            st0(if a[1] == 0xfa2 { 100.0 } else { 10.0 })
        });
        r.e.set(menu, LoadingMenu::m_DelayedTipMatteUpdate, 3);
        loading_menu_update(&mut r.e, menu);
        let (tip_a, tip_b) = (r.tile_at(menu, 0xb0), r.tile_at(menu, 0xb4));
        let (matte_a, matte_b) = (r.tile_at(menu, 0xb8), r.tile_at(menu, 0xbc));
        let sets = r.calls_to(TILE_SET_VALUE);
        let bits = |v: f32| v.to_bits();
        assert_eq!(
            sets,
            vec![
                vec![matte_a, 0xfa2, bits(85.0), 1],
                vec![matte_a, 0xfb0, bits(40.0), 1],
                vec![matte_b, 0xfa2, bits(85.0), 1],
                vec![matte_b, 0xfb0, bits(40.0), 1],
            ]
        );
        assert_eq!(r.calls_to(TILE_GET_VALUE)[0][0], tip_a);
        assert_eq!(r.calls_to(TILE_GET_VALUE)[2][0], tip_b);
        assert_eq!(r.e.get(menu, LoadingMenu::m_DelayedTipMatteUpdate), 0);
    }

    #[test]
    fn update_copies_only_the_requested_matte() {
        let (mut r, menu) = update_world();
        quiet(&mut r, menu);
        r.e.set(menu, LoadingMenu::m_DelayedTipMatteUpdate, 2);
        loading_menu_update(&mut r.e, menu);
        let matte_b = r.tile_at(menu, 0xbc);
        let sets = r.calls_to(TILE_SET_VALUE);
        assert_eq!(sets.len(), 2);
        assert!(sets.iter().all(|a| a[0] == matte_b));
        assert_eq!(r.e.get(menu, LoadingMenu::m_DelayedTipMatteUpdate), 0);
    }

    #[test]
    fn update_cross_fades_by_the_elapsed_time() {
        let (mut r, menu) = update_world();
        // 500 ms at 0.0005 per ms: 0.25; an odd slide count does not flip.
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 9_500);
        r.e.set(menu, LoadingMenu::m_SlideCount, 1);
        r.e.mem.set_u8(DO_TILE_UPDATE, 0);
        loading_menu_update(&mut r.e, menu);
        let (fade_a, fade_b) = (r.tile_at(menu, SLOT_FADE_A), r.tile_at(menu, SLOT_FADE_B));
        assert_eq!(
            r.calls_to(TILE_SET_INT),
            vec![vec![fade_a, 0xfa9, 63], vec![fade_b, 0xfa9, 191]]
        );
        assert_eq!(r.e.mem.u8(DO_TILE_UPDATE), 1);
    }

    #[test]
    fn update_flips_the_fade_for_an_even_slide_count() {
        let (mut r, menu) = update_world();
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 9_500);
        r.e.set(menu, LoadingMenu::m_SlideCount, 2);
        loading_menu_update(&mut r.e, menu);
        let (fade_a, fade_b) = (r.tile_at(menu, SLOT_FADE_A), r.tile_at(menu, SLOT_FADE_B));
        assert_eq!(
            r.calls_to(TILE_SET_INT),
            vec![vec![fade_a, 0xfa9, 191], vec![fade_b, 0xfa9, 63]]
        );
    }

    #[test]
    fn update_clamps_the_fade_and_treats_a_zero_start_as_complete() {
        let (mut r, menu) = update_world();
        r.e.set(menu, LoadingMenu::m_SlideCount, 1);
        // Start 0: the fade is 1.0 whatever the clock says.
        loading_menu_update(&mut r.e, menu);
        let (fade_a, fade_b) = (r.tile_at(menu, SLOT_FADE_A), r.tile_at(menu, SLOT_FADE_B));
        assert_eq!(
            r.calls_to(TILE_SET_INT),
            vec![vec![fade_a, 0xfa9, 255], vec![fade_b, 0xfa9, 0]]
        );
        // Far in the past: clamped to 1.0 too.
        let (mut r, menu) = update_world();
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 1);
        r.e.set(menu, LoadingMenu::m_SlideCount, 1);
        loading_menu_update(&mut r.e, menu);
        let fade_a = r.tile_at(menu, SLOT_FADE_A);
        assert_eq!(r.calls_to(TILE_SET_INT)[0], vec![fade_a, 0xfa9, 255]);
    }

    #[test]
    fn update_skips_null_fade_tiles() {
        let (mut r, menu) = update_world();
        r.e.mem.set_u32(menu.addr() + SLOT_FADE_A, 0);
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 9_500);
        loading_menu_update(&mut r.e, menu);
        let fade_b = r.tile_at(menu, SLOT_FADE_B);
        let sets = r.calls_to(TILE_SET_INT);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0][0], fade_b);
    }

    #[test]
    fn update_gives_the_current_and_next_screens_to_the_tip_routine() {
        let (mut r, menu) = update_world();
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 9_500);
        r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
        r.e.set(menu, LoadingMenu::m_SlideCount, 1);
        // The next node is the list head here, so its item is the first
        // screen; the current screen is the first as well.
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        loading_menu_update(&mut r.e, menu);
        assert_eq!(
            r.calls_to(LOAD_SCREEN_TYPE_FOLDED),
            vec![vec![first], vec![first]]
        );
    }

    fn started_world() -> (Rig, Ptr<LoadingMenu>) {
        let (mut r, menu) = update_world();
        // 1 s into the fade, on the initial load.
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 9_000);
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        r.e.mem.set_u32(MOVIE_PLAYER, 0x9400);
        r.e.mem.set_f32(MUSIC_LEVEL, 0.5);
        r.e.mem.set_f32(ALPHA_FULL, 255.0);
        (r, menu)
    }

    #[test]
    fn update_starts_the_title_music_and_movie_on_the_initial_load() {
        let (mut r, menu) = started_world();
        r.stub(STRING_LENGTH, 5);
        loading_menu_update(&mut r.e, menu);
        assert_eq!(r.calls_to(MEMSET).len(), 1);
        let buffer = r.calls_to(MEMSET)[0][0];
        assert_eq!(r.calls_to(MEMSET)[0][1..], [0, 0x40]);
        assert_eq!(
            r.calls_to(SNPRINTF),
            vec![vec![buffer, 0x40, MUSIC_PATH_FORMAT, 0x7000]]
        );
        assert_eq!(r.calls_to(CREDITS_SET_FLAG), vec![vec![1]]);
        assert_eq!(r.calls_to(MUSIC_PLAY), vec![vec![8, buffer, 0, 1, 1, 0, 0]]);
        assert_eq!(r.calls_to(MUSIC_UPDATE).len(), 1);
        assert_eq!(r.e.mem.f32(MUSIC_LEVEL_A), 0.5);
        assert_eq!(r.e.mem.f32(MUSIC_LEVEL_B), 0.5);
        let (fade_a, fade_b) = (r.tile_at(menu, SLOT_FADE_A), r.tile_at(menu, SLOT_FADE_B));
        let sets = r.calls_to(TILE_SET_VALUE);
        assert!(sets.contains(&vec![fade_a, 0xfa9, 0, 1]));
        assert!(sets.contains(&vec![fade_b, 0xfa9, 255.0f32.to_bits(), 1]));
        assert_eq!(
            r.calls_to(START_SEQUENCE),
            vec![vec![0x9400, 0x7000, 0, 0xffff_ffff, 0, 0, 0]]
        );
        assert_eq!(r.e.mem.u8(MUSIC_STARTED), 1);
        let wheel = r.tile_at(menu, SLOT_WHEEL);
        assert!(r.calls_to(TILE_SET_INT).contains(&vec![wheel, 0xfa3, 1]));
        assert_eq!(r.calls_to(MODEL_SHOW).len(), 1);
    }

    #[test]
    fn update_starts_only_the_music_without_a_movie_path() {
        let (mut r, menu) = started_world();
        r.stub(STRING_LENGTH, 0);
        loading_menu_update(&mut r.e, menu);
        assert_eq!(r.calls_to(MUSIC_PLAY).len(), 1);
        assert!(r.calls_to(START_SEQUENCE).is_empty());
        assert_eq!(r.e.mem.u8(MUSIC_STARTED), 1);
        assert!(r.calls_to(MODEL_SHOW).is_empty());
    }

    #[test]
    fn update_starts_the_music_once_and_only_when_nothing_blocks_it() {
        // Already started.
        let (mut r, menu) = started_world();
        r.e.mem.set_u8(MUSIC_STARTED, 1);
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(MUSIC_PLAY).is_empty());
        // A sequence is playing.
        let (mut r, menu) = started_world();
        r.stub(PLAYING_SEQUENCE, 1);
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(MUSIC_PLAY).is_empty());
        // State bit 0 (single screen).
        let (mut r, menu) = started_world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(0));
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(MUSIC_PLAY).is_empty());
        // Not the initial load.
        let (mut r, menu) = started_world();
        r.e.mem.set_u8(INITIAL_LOAD, 0);
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(MUSIC_PLAY).is_empty());
    }

    #[test]
    fn update_waits_for_the_loading_thread_and_shows_the_first_slide() {
        let (mut r, menu) = update_world();
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 1);
        r.e.set(menu, LoadingMenu::pSlide1TSP, Ptr::new(0x8888));
        let turns = Rc::new(RefCell::new(0));
        let counter = turns.clone();
        r.with(SLEEP_TICK, move |e, _| {
            *counter.borrow_mut() += 1;
            if *counter.borrow() == 3 {
                e.mem.set_u32(TEXTURE_OWNER, 0x9900);
            }
            eax(0)
        });
        loading_menu_update(&mut r.e, menu);
        assert_eq!(*turns.borrow(), 3);
        assert_eq!(r.calls_to(SLEEP_TICK), vec![vec![5], vec![5], vec![5]]);
        assert_eq!(r.calls_to(PROFILE_GUARD_NEW).len(), 1);
        // The fade-in time and the swap time both become now.
        assert_eq!(r.e.get(menu, LoadingMenu::uiFadeInComplete), 10_000);
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 10_000);
    }

    #[test]
    fn update_sets_the_swap_time_without_waiting_when_not_the_initial_load() {
        let (mut r, menu) = update_world();
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 1);
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(PROFILE_GUARD_NEW).is_empty());
        assert_eq!(r.e.get(menu, LoadingMenu::uiFadeInComplete), 10_000);
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 10_000);
    }

    #[test]
    fn update_steps_the_fade_in_when_state_bit_4_is_set() {
        let (mut r, menu) = update_world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(4));
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 1);
        loading_menu_update(&mut r.e, menu);
        assert_eq!(r.calls_to(FADE_STEP), vec![vec![menu.addr()]]);
        // The fade-in time stays 0, and so does the swap time.
        assert_eq!(r.e.get(menu, LoadingMenu::uiFadeInComplete), 0);
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 0);
        // With a swap time set, bit 4 steps the fade-in as well.
        let (mut r, menu) = update_world();
        quiet(&mut r, menu);
        loading_menu_update(&mut r.e, menu);
        assert_eq!(r.calls_to(FADE_STEP).len(), 1);
    }

    fn swap_world(extra_flags: u16) -> (Rig, Ptr<LoadingMenu>) {
        let (mut r, menu) = update_world();
        r.e.set(menu, LoadingMenu::m_TileCrossFadeStartTime, 1);
        r.e.set(menu, LoadingMenu::uiNextBackgroundSwap, 9_000);
        r.e.mem.set_u16(menu.addr() + 0x222, extra_flags);
        (r, menu)
    }

    /// Makes the model tile's sequence report state 1 (finished).
    fn sequence_done(r: &mut Rig, menu: Ptr<LoadingMenu>) {
        let tile = r.tile_at(menu, SLOT_MODEL);
        let sequence = r.object(0x60);
        r.e.mem.set_u32(tile + 0x3c, sequence);
        r.stub(SEQUENCE_STATE, 1);
    }

    #[test]
    fn update_does_not_swap_before_the_swap_time_or_for_an_interior() {
        let (mut r, menu) = swap_world(0);
        r.e.set(menu, LoadingMenu::uiNextBackgroundSwap, 10_001);
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(TILE_PLAY_SEQUENCE).is_empty());
        let (mut r, menu) = swap_world(bit(9));
        sequence_done(&mut r, menu);
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(PROFILE_GUARD_NEW).is_empty());
    }

    #[test]
    fn update_moves_to_the_next_screen_when_the_sequence_has_finished() {
        let (mut r, menu) = swap_world(0);
        sequence_done(&mut r, menu);
        let model_tile = r.tile_at(menu, SLOT_MODEL);
        r.stub(TILE_SEQUENCE, 0x11);
        r.stub(TILE_TARGET, 0x22);
        loading_menu_update(&mut r.e, menu);
        // The description is set (0078a280), the screen advanced (the
        // announcement), the texture loaded, the sequence call made, and the
        // next swap time computed (now + interval).
        assert!(!r.calls_to(TILE_SET_STRING).is_empty());
        assert_eq!(r.calls_to(ANNOUNCE_SCREEN).len(), 1);
        assert_eq!(r.calls_to(PROFILE_GUARD_NEW).len(), 1);
        assert_eq!(r.calls_to(TILE_SEQUENCE_CALL), vec![vec![0x22, 0x11, 0]]);
        assert_eq!(r.calls_to(TILE_SEQUENCE), vec![vec![model_tile]]);
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 10_000);
    }

    #[test]
    fn update_skips_the_description_with_a_locked_value_or_xui() {
        for (value, xui) in [(1.0, 0), (0.0, 1)] {
            let (mut r, menu) = swap_world(0);
            sequence_done(&mut r, menu);
            r.stub_float(VALUE_GETTER, value);
            r.stub(XUI_IS_UP, xui);
            loading_menu_update(&mut r.e, menu);
            assert!(r.calls_to(TILE_SET_STRING).is_empty());
            assert_eq!(r.calls_to(PROFILE_GUARD_NEW).len(), 1);
        }
        // On the initial load it is skipped as well.
        let (mut r, menu) = swap_world(0);
        sequence_done(&mut r, menu);
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        r.e.set(
            menu,
            LoadingMenu::pNextLoadScreen,
            Ptr::new(menu.addr() + 0x1d0),
        );
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(TILE_SET_STRING).is_empty());
    }

    #[test]
    fn update_with_state_bit_3_sets_the_description_only_when_the_tip_tile_is_true() {
        for (tip_true, called) in [(0, false), (1, true)] {
            let (mut r, menu) = swap_world(bit(3));
            sequence_done(&mut r, menu);
            r.stub(TILE_IS_TRUE, tip_true);
            loading_menu_update(&mut r.e, menu);
            assert_eq!(!r.calls_to(TILE_SET_STRING).is_empty(), called);
            let tip = r.tile_at(menu, SLOT_TIP_A);
            assert_eq!(r.calls_to(TILE_IS_TRUE), vec![vec![tip, 0xfa3]]);
        }
    }

    #[test]
    fn update_sets_state_bit_0_on_the_last_screen_of_the_initial_load() {
        let (mut r, menu) = swap_world(0);
        sequence_done(&mut r, menu);
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        // Make the list a single node so that advancing wraps to the head.
        r.e.mem.set_u32(menu.addr() + 0x1d4, 0);
        loading_menu_update(&mut r.e, menu);
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(0), 0);
    }

    #[test]
    fn update_plays_the_next_slide_sequence() {
        let (mut r, menu) = swap_world(0);
        r.stub(TILE_PLAY_SEQUENCE, 1);
        let model_tile = r.tile_at(menu, SLOT_MODEL);
        r.e.mem.set_u16(menu.addr() + 0x222, bit(7));
        r.stub(LIST_COUNT, 1);
        loading_menu_update(&mut r.e, menu);
        // Bit 1 was toggled on, so the odd sequence plays.
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(1), 0);
        assert_eq!(
            r.calls_to(TILE_PLAY_SEQUENCE),
            vec![vec![model_tile, SEQUENCE_ODD]]
        );
        assert_eq!(r.e.get(menu, LoadingMenu::m_TileCrossFadeStartTime), 10_000);
        assert_eq!(r.e.get(menu, LoadingMenu::m_SlideCount), 1);
        // The first screen was pending (bit 7): with one screen, bit 8 is
        // set and bit 7 cleared.
        let flags = r.e.get(menu, LoadingMenu::usFlags);
        assert_ne!(flags & bit(8), 0);
        assert_eq!(flags & bit(7), 0);
        assert_eq!(r.calls_to(UPDATE_DATA_NEW).len(), 1);
        assert_eq!(r.calls_to(NODE_UPDATE).len(), 1);
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 10_000);
    }

    #[test]
    fn update_keeps_bit_8_clear_when_there_are_several_screens() {
        let (mut r, menu) = swap_world(bit(7));
        r.stub(TILE_PLAY_SEQUENCE, 1);
        r.stub(LIST_COUNT, 2);
        loading_menu_update(&mut r.e, menu);
        let flags = r.e.get(menu, LoadingMenu::usFlags);
        assert_eq!(flags & bit(8), 0);
        assert_eq!(flags & bit(7), 0);
    }

    #[test]
    fn update_reports_a_slide_sequence_that_does_not_play() {
        let (mut r, menu) = swap_world(bit(1));
        loading_menu_update(&mut r.e, menu);
        // Bit 1 toggles off, so the even sequence is tried and reported.
        assert_eq!(r.calls_to(TILE_PLAY_SEQUENCE)[0][1], SEQUENCE_EVEN);
        assert_eq!(
            r.calls_to(LOG_LINE),
            vec![vec![SEQUENCE_WARNING, SEQUENCE_EVEN]]
        );
        assert_eq!(r.e.get(menu, LoadingMenu::m_SlideCount), 0);
    }

    #[test]
    fn update_does_not_start_a_slide_in_the_blocked_states() {
        // Initial load with bit 0, bit 3 and bit 8 each stop it.
        let (mut r, menu) = swap_world(bit(0));
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(TILE_PLAY_SEQUENCE).is_empty());
        for blocking in [bit(3), bit(8)] {
            let (mut r, menu) = swap_world(blocking);
            loading_menu_update(&mut r.e, menu);
            assert!(r.calls_to(TILE_PLAY_SEQUENCE).is_empty());
            assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 9_000);
        }
    }

    #[test]
    fn update_ends_by_refreshing_the_wheel_and_the_audio() {
        let (mut r, menu) = update_world();
        quiet(&mut r, menu);
        r.stub(TILE_IS_VISIBLE, 1);
        loading_menu_update(&mut r.e, menu);
        let wheel = r.tile_at(menu, SLOT_WHEEL);
        assert_eq!(
            r.calls_to(TILE_SHOW_FRAME),
            vec![vec![wheel, (-1.0f32).to_bits(), 0]]
        );
        assert_eq!(r.calls_to(AUDIO_FLUSH), vec![vec![0x5000, 1]]);
        let order = r.order();
        assert_eq!(*order.last().unwrap(), MUSIC_REFRESH);
        // A hidden wheel is left alone.
        let (mut r, menu) = update_world();
        quiet(&mut r, menu);
        loading_menu_update(&mut r.e, menu);
        assert!(r.calls_to(TILE_SHOW_FRAME).is_empty());
    }

    // ----- 0078a1c0, 0078a1f0, 0078a230, 0078a260 -----

    #[test]
    fn toggling_a_state_bit_flips_only_that_bit() {
        let mut r = rig();
        let menu = r.menu();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(1));
        fn_0078a1c0(&mut r.e, menu, 3);
        assert_eq!(r.e.get(menu, LoadingMenu::usFlags), bit(1) | bit(3));
        r.e.call(0x0078_a1c0, &args![menu, 3u32]);
        assert_eq!(r.e.get(menu, LoadingMenu::usFlags), bit(1));
    }

    #[test]
    fn a_tile_is_playing_when_its_sequence_state_is_1() {
        let mut r = rig();
        let tile = r.e.mem.alloc(0x100);
        assert!(!fn_0078a1f0(&mut r.e, tile));
        assert!(r.calls_to(SEQUENCE_STATE).is_empty());
        r.e.mem.set_u32(tile + 0x3c, 0x4000);
        r.stub(SEQUENCE_STATE, 2);
        assert!(!fn_0078a1f0(&mut r.e, tile));
        r.stub(SEQUENCE_STATE, 1);
        assert!(r.e.call(0x0078_a1f0, &args![tile]).bool());
        assert_eq!(r.calls_to(SEQUENCE_STATE).last().unwrap(), &vec![0x4000]);
    }

    #[test]
    fn the_wheel_shows_frame_minus_one() {
        let (mut r, menu) = world();
        fn_0078a230(&mut r.e, menu);
        let wheel = r.tile_at(menu, SLOT_WHEEL);
        assert_eq!(
            r.calls_to(TILE_SHOW_FRAME),
            vec![vec![wheel, (-1.0f32).to_bits(), 0]]
        );
    }

    #[test]
    fn a_setting_string_is_found_through_the_table() {
        let mut r = rig();
        r.e.mem.set_u32(SETTING_TABLE + 8, 0x7100);
        r.e.mem.set_u32(0x7100, 0x7200);
        r.with(SETTING_STRING, |_, a| eax(a[0] + 1));
        assert_eq!(r.e.call(0x0078_a260, &args![2u32]).u32(), 0x7201);
        assert_eq!(r.calls_to(SETTING_STRING), vec![vec![0x7200]]);
    }

    // ----- 0078a280 -----

    #[test]
    fn the_description_goes_to_the_first_tip_tile_for_even_slides() {
        let (mut r, menu) = world();
        r.e.mem.set_u8(0x7100, b'a');
        r.e.mem.set_u8(0x7101, b'b');
        r.stub(FORM_NAME, 0x7100);
        r.stub(VTABLE_TARGETS + 0x10, 0x7300);
        fn_0078a280(&mut r.e, menu, 0);
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        assert_eq!(
            r.calls_to(VTABLE_TARGETS + 0x10),
            vec![vec![first + 0x24, 0, 0x4353_4544]]
        );
        let (tip, description) = (
            r.tile_at(menu, SLOT_TIP_A),
            r.tile_at(menu, SLOT_DESCRIPTION),
        );
        assert_eq!(
            r.calls_to(TILE_SET_STRING),
            vec![
                vec![tip, 0xfc4, 0x7300, 1],
                vec![description, 0xfc4, 0x7000, 1]
            ]
        );
        assert_eq!(r.e.get(menu, LoadingMenu::m_DelayedTipMatteUpdate), 1);
        assert_eq!(r.e.mem.u8(DO_TILE_UPDATE), 1);
        // The name's characters are converted one by one.
        assert_eq!(
            r.calls_to(CHAR_CONVERT),
            vec![vec![b'a' as u32], vec![b'b' as u32]]
        );
        assert_eq!(r.calls_to(FORM_NAME), vec![vec![first + 0x18]]);
    }

    #[test]
    fn the_description_goes_to_the_second_tip_tile_for_odd_slides() {
        let (mut r, menu) = world();
        r.e.set(menu, LoadingMenu::m_SlideCount, 3);
        r.e.set(menu, LoadingMenu::m_DelayedTipMatteUpdate, 1);
        r.stub(VTABLE_TARGETS + 0x10, 0x7300);
        fn_0078a280(&mut r.e, menu, 0);
        let tip = r.tile_at(menu, SLOT_TIP_B);
        assert_eq!(r.calls_to(TILE_SET_STRING)[0], vec![tip, 0xfc4, 0x7300, 1]);
        assert_eq!(r.e.get(menu, LoadingMenu::m_DelayedTipMatteUpdate), 3);
        // An empty name converts nothing.
        assert!(r.calls_to(CHAR_CONVERT).is_empty());
    }

    // ----- 0078a3e0 -----

    #[test]
    fn a_texture_goes_into_the_ring_and_onto_the_fading_tile() {
        let (mut r, menu) = world();
        r.with(NI_POINTER_GET, |e, a| eax(e.mem.u32(a[0])));
        r.with(TEXTURE_LOAD, |e, a| {
            e.mem.set_u32(a[2], 0x5555);
            eax(0)
        });
        r.stub(VTABLE_TARGETS + 0x20, 0x6666);
        r.e.set(menu, LoadingMenu::m_SlideCount, 1);
        r.e.set(menu, LoadingMenu::m_nHoldTextureIndex, 3);
        r.e.mem.set_u32(menu.addr() + 0x20c + 12, 0x4444);
        r.e.mem.set_u32(menu.addr() + 0x1b8, 0x7777);
        r.e.mem.set_u32(TEXTURE_OWNER, 0x9900);
        fn_0078a3e0(&mut r.e, menu, 0);
        let slot = menu.addr() + 0x20c + 12;
        assert_eq!(r.calls_to(REMOVE_TEXTURE), vec![vec![0x4444]]);
        assert_eq!(r.calls_to(NI_POINTER_SET), vec![vec![slot, 0x5555]]);
        assert_eq!(r.e.get(menu, LoadingMenu::m_nHoldTextureIndex), 0);
        // Odd slide count: the second fading tile gets the property.
        assert_eq!(
            r.calls_to(VTABLE_TARGETS + 0x20),
            vec![vec![r.tile_at(menu, SLOT_FADE_B)]]
        );
        assert_eq!(r.calls_to(TEXTURE_APPLY), vec![vec![0x6666, 0x5555]]);
        let load = &r.calls_to(TEXTURE_LOAD)[0];
        assert_eq!((load[0], load[1], load[3], load[4]), (0x9900, 0x7777, 0, 0));
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        assert_eq!(
            r.calls_to(STRING_FORMAT),
            vec![vec![
                menu.addr() + 0x1b8,
                TEXTURE_PATH_FORMAT,
                TEXTURES_DIRECTORY,
                0x7000
            ]]
        );
        assert_eq!(r.calls_to(FORM_NAME), vec![vec![first + 0x18]]);
        // The guard brackets the work; the temporary is destroyed.
        let order = r.order();
        assert_eq!(order[0], PROFILE_GUARD_NEW);
        assert_eq!(*order.last().unwrap(), PROFILE_GUARD_DESTRUCTOR);
        let guard = r.calls_to(PROFILE_GUARD_NEW)[0].clone();
        assert_eq!(guard[1..], [0xd, 1, SOURCE_FILE_NAME, 0x423]);
        assert_eq!(r.calls_to(NI_POINTER_DESTRUCTOR).len(), 1);
    }

    #[test]
    fn the_ring_starts_over_and_empty_slots_are_not_released() {
        let (mut r, menu) = world();
        r.with(NI_POINTER_GET, |e, a| eax(e.mem.u32(a[0])));
        r.e.set(menu, LoadingMenu::m_nHoldTextureIndex, 1);
        fn_0078a3e0(&mut r.e, menu, 0);
        assert_eq!(r.e.get(menu, LoadingMenu::m_nHoldTextureIndex), 2);
        assert!(r.calls_to(REMOVE_TEXTURE).is_empty());
        // Even slide count: the first fading tile is asked; a null property
        // is not applied.
        assert_eq!(
            r.calls_to(VTABLE_TARGETS + 0x20),
            vec![vec![r.tile_at(menu, SLOT_FADE_A)]]
        );
        assert!(r.calls_to(TEXTURE_APPLY).is_empty());
        assert_eq!(r.calls_to(NI_POINTER_SET)[0][0], menu.addr() + 0x20c + 4);
    }

    // ----- 0078a910, 0078a930 -----

    #[test]
    fn the_counter_helper_post_increments() {
        let mut r = rig();
        r.e.mem.set_u32(0x7100, 41);
        assert_eq!(r.e.call(0x0078_a910, &args![0x7100u32, 0u32]).u32(), 41);
        assert_eq!(r.e.mem.u32(0x7100), 42);
    }

    #[test]
    fn the_list_lock_byte_is_read() {
        let mut r = rig();
        assert!(!r.e.call(0x0078_a930, &args![]).bool());
        r.e.mem.set_u8(LIST_LOCKED, 1);
        assert!(r.e.call(0x0078_a930, &args![]).bool());
    }

    // ----- 0078a940, 0078a9f0, 0078aa90 -----

    #[test]
    fn clearing_the_list_deletes_only_temporary_screens() {
        let (mut r, menu) = world();
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        r.with(IS_TEMPORARY, move |_, a| eax((a[0] == first) as u32));
        r.e.mem.set_u32(menu.addr() + 0x1d8, 0x1234);
        fn_0078a940(&mut r.e, menu);
        // Virtual slot 0x10 with 1 on the first screen only.
        assert_eq!(r.calls_to(VTABLE_TARGETS + 0x10), vec![vec![first, 1]]);
        assert_eq!(r.calls_to(LIST_FREE_NODES), vec![vec![menu.addr() + 0x1d0]]);
        assert_eq!(
            r.e.get(menu, LoadingMenu::pNextLoadScreen).addr(),
            menu.addr() + 0x1d0
        );
        assert_eq!(r.calls_to(IS_TEMPORARY).len(), 2);
    }

    #[test]
    fn clearing_the_list_stops_at_an_empty_node() {
        let (mut r, menu) = world();
        r.e.mem.set_u32(menu.addr() + 0x1d0, 0);
        r.stub(IS_TEMPORARY, 1);
        fn_0078a940(&mut r.e, menu);
        assert!(r.calls_to(IS_TEMPORARY).is_empty());
        assert_eq!(r.calls_to(LIST_FREE_NODES).len(), 1);
    }

    #[test]
    fn advancing_walks_the_list_and_wraps_to_the_head() {
        let (mut r, menu) = world();
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        let tail = r.e.mem.u32(menu.addr() + 0x1d4);
        let second = r.e.mem.u32(tail);
        fn_0078a9f0(&mut r.e, menu);
        assert_eq!(r.e.get(menu, LoadingMenu::pCurrentLoadScreen).addr(), first);
        assert_eq!(r.e.get(menu, LoadingMenu::pNextLoadScreen).addr(), tail);
        assert_eq!(r.calls_to(ANNOUNCE_SCREEN), vec![vec![menu.addr(), first]]);
        fn_0078a9f0(&mut r.e, menu);
        assert_eq!(
            r.e.get(menu, LoadingMenu::pCurrentLoadScreen).addr(),
            second
        );
        assert_eq!(
            r.e.get(menu, LoadingMenu::pNextLoadScreen).addr(),
            menu.addr() + 0x1d0
        );
        assert_eq!(r.calls_to(ANNOUNCE_SCREEN).len(), 2);
        assert_eq!(r.calls_to(ANNOUNCE_SCREEN)[1], vec![menu.addr(), second]);
    }

    #[test]
    fn advancing_onto_an_empty_node_announces_nothing() {
        let (mut r, menu) = world();
        let tail = r.e.mem.u32(menu.addr() + 0x1d4);
        r.e.mem.set_u32(tail, 0);
        fn_0078a9f0(&mut r.e, menu);
        assert!(r.calls_to(ANNOUNCE_SCREEN).is_empty());
    }

    #[test]
    fn jumping_to_the_last_screen_needs_the_initial_load_and_a_stopped_sequence() {
        // Not the initial load.
        let (mut r, menu) = world();
        assert!(!fn_0078aa90(&mut r.e, menu));
        // Initial load but bit 0 set.
        let (mut r, menu) = world();
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        r.e.mem.set_u16(menu.addr() + 0x222, bit(0));
        assert!(!fn_0078aa90(&mut r.e, menu));
        // Initial load but the sequence is running.
        let (mut r, menu) = world();
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        let tile = r.tile_at(menu, SLOT_MODEL);
        r.e.mem.set_u32(tile + 0x3c, 0x4000);
        r.stub(SEQUENCE_STATE, 1);
        assert!(!fn_0078aa90(&mut r.e, menu));
        assert!(r.calls_to(PROFILE_GUARD_NEW).is_empty());
    }

    #[test]
    fn jumping_to_the_last_screen_shows_it() {
        let (mut r, menu) = world();
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        assert!(r.e.call(0x0078_aa90, &args![menu]).bool());
        let tail = r.e.mem.u32(menu.addr() + 0x1d4);
        assert_eq!(r.e.get(menu, LoadingMenu::pNextLoadScreen).addr(), tail);
        assert_eq!(r.calls_to(PROFILE_GUARD_NEW).len(), 1);
        // The interval is 0 here: the swap time is now.
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 10_000);
    }

    // ----- 0078ab20 -----

    /// Name pointers by screen: the screens in `LoadScreens` are named "one"
    /// and "two"; the double compares C strings like `strcmp`.
    fn naming_world() -> (Rig, Ptr<LoadingMenu>, u32, u32) {
        let (mut r, menu) = world();
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        let tail = r.e.mem.u32(menu.addr() + 0x1d4);
        let second = r.e.mem.u32(tail);
        for (screen, text, at) in [(first, b"one\0", 0x7400u32), (second, b"two\0", 0x7410)] {
            r.e.mem.write(at, text);
            r.e.mem.set_u32(screen + 0x18, at);
        }
        r.with(FORM_NAME, |e, a| eax(e.mem.u32(a[0])));
        r.with(NAME_COMPARE, |e, a| {
            eax((e.mem.cstr(a[0]) != e.mem.cstr(a[1])) as u32)
        });
        r.stub(FORM_TYPE, 0x4b);
        (r, menu, first, second)
    }

    fn candidate(r: &mut Rig, name: &[u8], at: u32) -> u32 {
        let screen = r.screen();
        r.e.mem.write(at, name);
        r.e.mem.set_u32(screen + 0x18, at);
        screen
    }

    #[test]
    fn a_new_screen_is_accepted_when_no_listed_screen_has_its_name() {
        let (mut r, menu, _, _) = naming_world();
        let screen = candidate(&mut r, b"three\0", 0x7420);
        assert!(fn_0078ab20(&mut r.e, menu, screen));
        assert_eq!(r.calls_to(NAME_COMPARE).len(), 2);
    }

    #[test]
    fn a_screen_with_a_listed_name_is_refused_at_the_first_match() {
        let (mut r, menu, _, _) = naming_world();
        let screen = candidate(&mut r, b"one\0", 0x7420);
        assert!(!fn_0078ab20(&mut r.e, menu, screen));
        assert_eq!(r.calls_to(NAME_COMPARE).len(), 1);
        // A match on the second one stops there.
        let screen = candidate(&mut r, b"two\0", 0x7430);
        r.clear_calls();
        assert!(!fn_0078ab20(&mut r.e, menu, screen));
        assert_eq!(r.calls_to(NAME_COMPARE).len(), 2);
    }

    #[test]
    fn only_forms_of_type_0x4b_are_accepted() {
        let (mut r, menu, _, _) = naming_world();
        let screen = candidate(&mut r, b"three\0", 0x7420);
        r.stub(FORM_TYPE, 0x4a);
        assert!(!fn_0078ab20(&mut r.e, menu, screen));
        assert!(r.calls_to(NAME_COMPARE).is_empty());
        assert!(!fn_0078ab20(&mut r.e, menu, 0));
        assert_eq!(r.calls_to(FORM_TYPE).len(), 1);
    }

    // ----- 0078abd0 -----

    fn interval_world() -> (Rig, Ptr<LoadingMenu>) {
        let (mut r, menu) = world();
        r.put_f64(MS_PER_SECOND, 1000.0);
        r.with(SETTING_FLOAT, |_, a| {
            st0(match a[0] {
                SETTING_INTERVAL_XUI => 1.5,
                SETTING_INTERVAL_INITIAL => 2.5,
                SETTING_INTERVAL_DEFAULT => 4.0,
                other => panic!("unexpected setting {other:x}"),
            })
        });
        (r, menu)
    }

    #[test]
    fn the_swap_interval_depends_on_the_situation() {
        let (mut r, menu) = interval_world();
        fn_0078abd0(&mut r.e, menu);
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 14_000);
        r.e.mem.set_u8(INITIAL_LOAD, 1);
        fn_0078abd0(&mut r.e, menu);
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 12_500);
        r.stub(XUI_IS_UP, 1);
        r.e.call(0x0078_abd0, &args![menu]);
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 11_500);
    }

    #[test]
    fn a_negative_interval_moves_the_swap_time_back() {
        let (mut r, menu) = world();
        r.put_f64(MS_PER_SECOND, 1000.0);
        r.stub_float(SETTING_FLOAT, -1.25);
        fn_0078abd0(&mut r.e, menu);
        // The product truncates toward zero.
        assert_eq!(r.e.get(menu, LoadingMenu::uiNextBackgroundSwap), 8_750);
    }

    // ----- 0078a5a0 -----

    struct ListRig {
        r: Rig,
        menu: Ptr<LoadingMenu>,
        /// The `TESLoadScreen`s `operator new` handed out.
        made: Rc<RefCell<Vec<u32>>>,
        /// The items added through `LIST_ADD` / `LIST_PUSH` (read from the
        /// word whose address is passed).
        added: Rc<RefCell<Vec<u32>>>,
    }

    /// A rig for `MakeLoadScreenList`: `LIST_EMPTY` answers from `empty` (in
    /// order, then 0), `operator new` allocates, the constructor returns its
    /// argument and the string buffer holds the path 0x7777.
    fn list_rig(empty: Vec<u32>) -> ListRig {
        let (mut r, menu) = world();
        let made = Rc::new(RefCell::new(Vec::new()));
        let added = Rc::new(RefCell::new(Vec::new()));
        let answers = Rc::new(RefCell::new(empty));
        r.with(LIST_EMPTY, move |_, _| {
            let mut answers = answers.borrow_mut();
            eax(if answers.is_empty() {
                0
            } else {
                answers.remove(0)
            })
        });
        let record = made.clone();
        r.with(OPERATOR_NEW, move |e, a| {
            let block = e.mem.alloc(a[0]);
            record.borrow_mut().push(block);
            eax(block)
        });
        r.with(LOAD_SCREEN_NEW, |_, a| eax(a[0]));
        for add in [LIST_ADD, LIST_PUSH] {
            let record = added.clone();
            r.with(add, move |e, a| {
                record.borrow_mut().push(e.mem.u32(a[1]));
                eax(0)
            });
        }
        r.with(NI_POINTER_GET, |e, a| eax(e.mem.u32(a[0])));
        r.with(SETTING_STRING, |_, a| eax(a[0]));
        r.e.mem.set_u32(menu.addr() + 0x1b8, 0x7777);
        ListRig {
            r,
            menu,
            made,
            added,
        }
    }

    #[test]
    fn the_initial_load_builds_one_screen_per_setting() {
        let mut l = list_rig(vec![1, 0]);
        l.r.e.mem.set_u8(INITIAL_LOAD, 1);
        l.r.e.mem.set_u16(l.menu.addr() + 0x222, bit(6));
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        let menu = l.menu.addr();
        let formats = l.r.calls_to(STRING_FORMAT);
        assert_eq!(formats.len(), 5);
        for (i, call) in formats.iter().enumerate() {
            let directory = if i == 4 {
                MAIN_DIRECTORY
            } else {
                LOADING_DIRECTORY
            };
            assert_eq!(
                call,
                &vec![
                    menu + 0x1b8,
                    SCREEN_PATH_FORMAT,
                    directory,
                    SETTING_FIRST_SCREEN + i as u32 * 12
                ]
            );
        }
        let made = l.made.borrow().clone();
        assert_eq!(made.len(), 5);
        assert_eq!(*l.added.borrow(), made);
        assert_eq!(l.r.calls_to(LIST_ADD).len(), 5);
        assert_eq!(l.r.calls_to(LOAD_SCREEN_NEW).len(), 5);
        for screen in &made {
            assert!(l
                .r
                .calls_to(SET_PATH)
                .contains(&vec![screen + 0x18, 0x7777]));
            assert!(l.r.calls_to(SET_TEMPORARY).contains(&vec![*screen]));
        }
        assert!(l.r.calls_to(CANDIDATES_RESET).is_empty());
        // Empty list: nothing was cleared and no default screen was added.
        assert!(l.r.calls_to(LIST_FREE_NODES).is_empty());
        assert!(l.r.calls_to(LIST_PUSH).is_empty());
        // Bookkeeping at the end: bit 7, the first screen is current, the
        // face calculation, and the tips shown for that screen.
        let flags = l.r.e.get(l.menu, LoadingMenu::usFlags);
        assert_ne!(flags & bit(7), 0);
        assert_eq!(flags & bit(5), 0);
        let first = l.r.e.mem.u32(menu + 0x1d0);
        assert_eq!(
            l.r.e.get(l.menu, LoadingMenu::pCurrentLoadScreen).addr(),
            first
        );
        assert_eq!(l.r.calls_to(CALC_FACES), vec![vec![menu]]);
        assert_eq!(l.r.calls_to(GROUP_VISIBLE_0078B6A0), vec![vec![menu, 0]]);
        assert_eq!(l.r.calls_to(LOAD_SCREEN_TYPE), vec![vec![first]]);
    }

    #[test]
    fn a_music_path_makes_the_initial_load_start_at_the_third_setting() {
        let mut l = list_rig(vec![1, 0]);
        l.r.e.mem.set_u8(INITIAL_LOAD, 1);
        l.r.stub(STRING_LENGTH, 3);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        let formats = l.r.calls_to(STRING_FORMAT);
        assert_eq!(formats.len(), 2);
        assert_eq!(
            formats[0][2..],
            [LOADING_DIRECTORY, SETTING_FIRST_SCREEN + 36]
        );
        assert_eq!(formats[1][2..], [MAIN_DIRECTORY, SETTING_FIRST_SCREEN + 48]);
    }

    #[test]
    fn a_non_empty_list_is_cleared_and_the_slide_reloaded() {
        let mut l = list_rig(vec![0, 0]);
        l.r.e.mem.set_u8(INITIAL_LOAD, 1);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        assert_eq!(l.r.calls_to(LIST_FREE_NODES).len(), 1);
        assert_eq!(l.r.calls_to(PROFILE_GUARD_NEW).len(), 1);
        // A running model sequence is not interrupted.
        let mut l = list_rig(vec![0, 0]);
        l.r.e.mem.set_u8(INITIAL_LOAD, 1);
        let tile = l.r.tile_at(l.menu, SLOT_MODEL);
        l.r.e.mem.set_u32(tile + 0x3c, 0x4000);
        l.r.stub(SEQUENCE_STATE, 1);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        assert_eq!(l.r.calls_to(LIST_FREE_NODES).len(), 1);
        assert!(l.r.calls_to(PROFILE_GUARD_NEW).is_empty());
    }

    #[test]
    fn overlay_mode_sets_bit_5_and_collects_candidates() {
        let mut l = list_rig(vec![1, 0]);
        l.r.e.mem.set_u8(INITIAL_LOAD, 1);
        l.r.stub(XUI_IS_UP, 1);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        assert_ne!(l.r.e.get(l.menu, LoadingMenu::usFlags) & bit(5), 0);
        assert_eq!(l.r.calls_to(CANDIDATES_RESET).len(), 1);
        assert!(l.r.calls_to(STRING_FORMAT).is_empty());
    }

    #[test]
    fn the_overlay_owner_also_sets_bit_5() {
        let mut l = list_rig(vec![1, 0]);
        l.r.stub(OVERLAY_TEST, 1);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        assert_ne!(l.r.e.get(l.menu, LoadingMenu::usFlags) & bit(5), 0);
    }

    #[test]
    fn candidates_are_collected_into_a_temporary_list() {
        let mut l = list_rig(vec![1, 0]);
        l.r.e.set(l.menu, LoadingMenu::pLocation, Ptr::new(0x55));
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        let created = l.r.calls_to(CANDIDATE_LIST_NEW);
        let destroyed = l.r.calls_to(CANDIDATE_LIST_DESTRUCTOR);
        assert_eq!(created.len(), 1);
        assert_eq!(created, destroyed);
        let list = created[0][0];
        assert_eq!(l.r.calls_to(CANDIDATES_RESET), vec![vec![list, 0]]);
        // The limit setting is 0, so there is no second pass.
        assert_eq!(l.r.e.get(l.menu, LoadingMenu::pLocation).addr(), 0x55);
    }

    #[test]
    fn too_few_screens_for_a_location_trigger_a_second_pass_without_it() {
        let mut l = list_rig(vec![1, 0]);
        l.r.e.set(l.menu, LoadingMenu::pLocation, Ptr::new(0x55));
        l.r.e.mem.set_u32(0x7010, 3);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        assert_eq!(l.r.calls_to(CANDIDATES_RESET).len(), 2);
        assert_eq!(l.r.e.get(l.menu, LoadingMenu::pLocation).addr(), 0);
        // A locked list is not rebuilt.
        let mut l = list_rig(vec![1, 0]);
        l.r.e.set(l.menu, LoadingMenu::pLocation, Ptr::new(0x55));
        l.r.e.mem.set_u32(0x7010, 3);
        l.r.e.mem.set_u8(LIST_LOCKED, 1);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        assert_eq!(l.r.calls_to(CANDIDATES_RESET).len(), 1);
        assert_eq!(l.r.e.get(l.menu, LoadingMenu::pLocation).addr(), 0x55);
        // Without a location there is nothing to drop.
        let mut l = list_rig(vec![1, 0]);
        l.r.e.mem.set_u32(0x7010, 3);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        assert_eq!(l.r.calls_to(CANDIDATES_RESET).len(), 1);
    }

    #[test]
    fn an_empty_list_gets_one_default_screen() {
        let mut l = list_rig(vec![1, 1]);
        loading_menu_make_load_screen_list(&mut l.r.e, l.menu);
        let menu = l.menu.addr();
        assert_eq!(
            l.r.calls_to(STRING_FORMAT),
            vec![vec![
                menu + 0x1b8,
                LOADING_SCREEN_PATH_FORMAT,
                SETTING_FIRST_SCREEN
            ]]
        );
        let made = l.made.borrow().clone();
        assert_eq!(made.len(), 1);
        assert_eq!(*l.added.borrow(), made);
        assert_eq!(l.r.calls_to(LIST_PUSH).len(), 1);
        assert_eq!(l.r.calls_to(LIST_PUSH)[0][0], menu + 0x1d0);
        assert!(l.r.calls_to(LIST_ADD).is_empty());
    }

    // ----- 0078ac60, 0078ada0 -----

    struct Candidates {
        r: Rig,
        menu: Ptr<LoadingMenu>,
        list: u32,
        screens: Vec<u32>,
        nodes: Vec<u32>,
    }

    /// The global list holds three screens; the candidate list is a block.
    fn candidates() -> Candidates {
        let (mut r, menu) = world();
        let screens: Vec<u32> = (0..3).map(|_| r.screen()).collect();
        let c = r.node(screens[2], 0);
        let b = r.node(screens[1], c);
        let a = r.node(screens[0], b);
        r.stub(ALL_SCREENS_LIST, a);
        r.e.mem.set_u32(LOAD_SCREEN_OWNER, 0x9800);
        let list = r.e.mem.alloc(0x10);
        Candidates {
            r,
            menu,
            list,
            screens,
            nodes: vec![a, b, c],
        }
    }

    fn adds(c: &Candidates) -> Vec<u32> {
        c.r.calls_to(CANDIDATE_ADD).iter().map(|a| a[1]).collect()
    }

    #[test]
    fn candidates_without_a_location_are_the_generic_screens() {
        let mut c = candidates();
        let wanted = [c.screens[0], c.screens[2]];
        c.r.with(LOAD_SCREEN_GENERIC, move |_, a| {
            eax(wanted.contains(&a[0]) as u32)
        });
        loading_menu_build_candidate_list(&mut c.r.e, c.menu, c.list);
        assert_eq!(c.r.calls_to(CANDIDATES_RESET), vec![vec![c.list, 0]]);
        assert_eq!(adds(&c), vec![c.nodes[0], c.nodes[2]]);
        assert_eq!(c.r.calls_to(ALL_SCREENS_LIST), vec![vec![0x9800]]);
        assert!(c.r.calls_to(LOAD_SCREEN_FOR_LOCATION).is_empty());
        assert!(c.r.calls_to(CANDIDATE_REMOVE).is_empty());
    }

    #[test]
    fn candidates_for_a_location_are_asked_about_it() {
        let mut c = candidates();
        c.r.e.set(c.menu, LoadingMenu::pLocation, Ptr::new(0x55));
        let wanted = c.screens[1];
        c.r.with(LOAD_SCREEN_FOR_LOCATION, move |_, a| {
            eax((a[0] == wanted) as u32)
        });
        loading_menu_build_candidate_list(&mut c.r.e, c.menu, c.list);
        assert_eq!(adds(&c), vec![c.nodes[1]]);
        assert_eq!(c.r.calls_to(LOAD_SCREEN_FOR_LOCATION)[0][1], 0x55);
        assert!(c.r.calls_to(LOAD_SCREEN_GENERIC).is_empty());
    }

    #[test]
    fn overlay_candidates_are_the_screens_with_flag_0x400() {
        let mut c = candidates();
        c.r.e.mem.set_u16(c.menu.addr() + 0x222, bit(5));
        let flagged = c.screens[1];
        c.r.with(WORD_AT_8, move |_, a| {
            eax(if a[0] == flagged { 0x400 } else { 0x3ff })
        });
        loading_menu_build_candidate_list(&mut c.r.e, c.menu, c.list);
        assert_eq!(adds(&c), vec![c.nodes[1]]);
        assert!(c.r.calls_to(LOAD_SCREEN_GENERIC).is_empty());
    }

    #[test]
    fn candidates_of_other_types_are_dropped_again_while_a_menu_is_replaced() {
        for replaced in [0, 1] {
            let mut c = candidates();
            if replaced == 1 {
                c.r.e.mem.set_u8(MENU_REPLACED, 1);
            } else {
                c.r.stub(OVERLAY_TEST, 1);
            }
            c.r.stub(LOAD_SCREEN_GENERIC, 1);
            c.r.stub(CANDIDATE_ADD, 7);
            let types = [(c.screens[0], 2), (c.screens[1], 3), (c.screens[2], 0)];
            c.r.with(LOAD_SCREEN_TYPE, move |_, a| {
                eax(types.iter().find(|t| t.0 == a[0]).unwrap().1)
            });
            loading_menu_build_candidate_list(&mut c.r.e, c.menu, c.list);
            assert_eq!(adds(&c).len(), 3);
            assert_eq!(c.r.calls_to(CANDIDATE_REMOVE), vec![vec![c.list, 7, 1]]);
        }
        // Without a replaced menu or overlay nothing is removed.
        let mut c = candidates();
        c.r.stub(LOAD_SCREEN_GENERIC, 1);
        c.r.stub(LOAD_SCREEN_TYPE, 2);
        loading_menu_build_candidate_list(&mut c.r.e, c.menu, c.list);
        assert!(c.r.calls_to(CANDIDATE_REMOVE).is_empty());
    }

    #[test]
    fn the_candidate_walk_stops_at_an_empty_node() {
        let mut c = candidates();
        c.r.stub(LOAD_SCREEN_GENERIC, 1);
        c.r.e.mem.set_u32(c.nodes[1], 0);
        loading_menu_build_candidate_list(&mut c.r.e, c.menu, c.list);
        assert_eq!(adds(&c), vec![c.nodes[0]]);
    }

    #[test]
    fn bit_0x400_of_the_flags_word_marks_an_overlay_screen() {
        let mut r = rig();
        r.stub(WORD_AT_8, 0x400);
        assert!(r.e.call(0x0078_ada0, &args![0x77u32]).bool());
        r.stub(WORD_AT_8, 0xbff);
        assert!(!r.e.call(0x0078_ada0, &args![0x77u32]).bool());
        assert_eq!(r.calls_to(WORD_AT_8)[0], vec![0x77]);
    }

    // ----- 0078adc0 -----

    struct Picking {
        r: Rig,
        menu: Ptr<LoadingMenu>,
        list: u32,
        /// Screens added to `LoadScreens`.
        added: Rc<RefCell<Vec<u32>>>,
    }

    /// A candidate list of `n` screens (the size at `+8`, the array in a
    /// block). Random choice always picks index 0; removal shifts the array
    /// and shrinks the size. Settings: 5 screens, 2 in overlay mode, 1 per
    /// location. Every screen has type `0x4b` and a name of its own.
    fn picking(n: u32) -> Picking {
        let (mut r, menu) = world();
        let list = r.e.mem.alloc(0x10);
        let array = r.e.mem.alloc(4 * n.max(1));
        r.e.mem.set_u32(list + 8, n);
        for i in 0..n {
            let screen = r.screen();
            r.e.mem.write(0x7500 + i * 8, b"s\0");
            r.e.mem.set_u8(0x7501 + i * 8, b'a' + i as u8);
            r.e.mem.set_u32(screen + 0x18, 0x7500 + i * 8);
            r.e.mem.set_u32(array + 4 * i, screen);
        }
        // The screens already listed need names too.
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        let tail = r.e.mem.u32(menu.addr() + 0x1d4);
        let second = r.e.mem.u32(tail);
        for (screen, at) in [(first, 0x7600u32), (second, 0x7610)] {
            r.e.mem.write(at, b"x\0");
            r.e.mem.set_u32(screen + 0x18, at);
        }
        r.with(WORD_AT_8, |e, a| eax(e.mem.u32(a[0] + 8)));
        r.stub(RANDOM_INDEX, 0);
        r.with(CANDIDATE_AT, move |_, a| eax(array + 4 * a[1]));
        r.with(CANDIDATE_REMOVE, move |e, a| {
            let size = e.mem.u32(a[0] + 8);
            for i in a[1]..size - 1 {
                let next = e.mem.u32(array + 4 * (i + 1));
                e.mem.set_u32(array + 4 * i, next);
            }
            e.mem.set_u32(a[0] + 8, size - 1);
            eax(0)
        });
        r.with(FORM_NAME, |e, a| eax(e.mem.u32(a[0])));
        r.with(NAME_COMPARE, |e, a| {
            eax((e.mem.cstr(a[0]) != e.mem.cstr(a[1])) as u32)
        });
        r.stub(FORM_TYPE, 0x4b);
        r.with(MINIMUM, |_, a| eax(a[0].min(a[1])));
        r.with(SETTING_INT_POINTER, |e, a| {
            let value = match a[0] {
                SETTING_MAX_SCREENS => 5,
                SETTING_MAX_SCREENS_OVERLAY => 2,
                SETTING_LOCATION_SCREENS => 1,
                other => panic!("unexpected setting {other:x}"),
            };
            e.mem.set_u32(0x7010, value);
            eax(0x7010)
        });
        let added = Rc::new(RefCell::new(Vec::new()));
        let record = added.clone();
        r.with(LIST_ADD, move |e, a| {
            record.borrow_mut().push(e.mem.u32(a[1]));
            eax(0)
        });
        let record = added.clone();
        r.with(LIST_COUNT, move |_, _| eax(record.borrow().len() as u32));
        Picking {
            r,
            menu,
            list,
            added,
        }
    }

    #[test]
    fn picking_moves_candidates_until_the_screen_limit() {
        let mut p = picking(8);
        fn_0078adc0(&mut p.r.e, p.menu, p.list);
        assert_eq!(p.added.borrow().len(), 5);
        // Every pick removed its candidate; the rest stay.
        assert_eq!(p.r.calls_to(CANDIDATE_REMOVE).len(), 5);
        assert_eq!(p.r.e.mem.u32(p.list + 8), 3);
    }

    #[test]
    fn picking_stops_when_the_candidates_run_out() {
        let mut p = picking(2);
        fn_0078adc0(&mut p.r.e, p.menu, p.list);
        assert_eq!(p.added.borrow().len(), 2);
        assert_eq!(p.r.e.mem.u32(p.list + 8), 0);
    }

    #[test]
    fn picking_uses_the_overlay_limit_in_overlay_mode() {
        let mut p = picking(8);
        p.r.e.mem.set_u16(p.menu.addr() + 0x222, bit(5));
        fn_0078adc0(&mut p.r.e, p.menu, p.list);
        assert_eq!(p.added.borrow().len(), 2);
    }

    #[test]
    fn picking_for_a_location_uses_the_smaller_limit_unless_locked() {
        let mut p = picking(8);
        p.r.e.set(p.menu, LoadingMenu::pLocation, Ptr::new(0x55));
        fn_0078adc0(&mut p.r.e, p.menu, p.list);
        assert_eq!(p.added.borrow().len(), 1);
        assert_eq!(p.r.calls_to(MINIMUM), vec![vec![5, 1]]);
        let mut p = picking(8);
        p.r.e.set(p.menu, LoadingMenu::pLocation, Ptr::new(0x55));
        p.r.e.mem.set_u8(LIST_LOCKED, 1);
        fn_0078adc0(&mut p.r.e, p.menu, p.list);
        assert_eq!(p.added.borrow().len(), 5);
        assert!(p.r.calls_to(MINIMUM).is_empty());
    }

    #[test]
    fn picking_drops_candidates_the_list_already_has() {
        let mut p = picking(3);
        // Candidates of another type are removed from the list but not added.
        p.r.stub(FORM_TYPE, 0x4a);
        fn_0078adc0(&mut p.r.e, p.menu, p.list);
        assert!(p.added.borrow().is_empty());
        assert_eq!(p.r.calls_to(CANDIDATE_REMOVE).len(), 3);
        assert_eq!(p.r.e.mem.u32(p.list + 8), 0);
    }

    #[test]
    fn picking_does_nothing_when_the_list_is_already_full() {
        let mut p = picking(3);
        p.r.stub(LIST_COUNT, 5);
        fn_0078adc0(&mut p.r.e, p.menu, p.list);
        assert!(p.r.calls_to(CANDIDATE_REMOVE).is_empty());
        assert!(p.added.borrow().is_empty());
    }

    // ----- 0078aec0 -----

    #[test]
    fn the_status_tiles_are_set_from_the_globals_and_settings() {
        let (mut r, menu) = world();
        r.e.mem.set_f32(TIP_SCALE, 0.75);
        r.e.mem.set_u32(TIP_VALUE, 0x1234);
        r.e.mem.set_u32(TIP_TEXT_OWNER, 0x9a00);
        r.stub(TEXT_OWNER_NAME, 0x7500);
        r.with(SETTING_STRING, |_, a| eax(a[0] + 1));
        fn_0078aec0(&mut r.e, menu);
        assert_eq!(r.calls_to(REFRESH_STATS).len(), 1);
        let scaled = r.tile_at(menu, SLOT_SCALED);
        assert_eq!(
            r.calls_to(TILE_SET_VALUE),
            vec![vec![scaled, 0x1004, 0.75f32.to_bits(), 1]]
        );
        assert_eq!(
            r.calls_to(TILE_SET_ID_VALUE),
            vec![vec![scaled, 0x1005, 0x1234]]
        );
        assert_eq!(r.calls_to(TEXT_OWNER_NAME), vec![vec![0x9a00]]);
        let (status, label60, label64) = (
            r.tile_at(menu, SLOT_STATUS_TEXT),
            r.tile_at(menu, SLOT_TEXT_60),
            r.tile_at(menu, SLOT_TEXT_64),
        );
        assert_eq!(
            r.calls_to(TILE_SET_STRING),
            vec![
                vec![status, 0xfc4, 0x7500, 1],
                vec![label60, 0xfc4, SETTING_TEXT_60 + 1, 1],
                vec![label64, 0xfc4, SETTING_TEXT_64 + 1, 1],
            ]
        );
    }

    #[test]
    fn the_status_text_falls_back_to_a_setting_without_an_owner() {
        let (mut r, menu) = world();
        r.with(SETTING_STRING, |_, a| eax(a[0] + 1));
        fn_0078aec0(&mut r.e, menu);
        let status = r.tile_at(menu, SLOT_STATUS_TEXT);
        assert_eq!(
            r.calls_to(TILE_SET_STRING)[0],
            vec![status, 0xfc4, SETTING_TIP_FALLBACK + 1, 1]
        );
        assert!(r.calls_to(TEXT_OWNER_NAME).is_empty());
    }

    #[test]
    fn the_stats_are_not_refreshed_when_the_skip_test_passes() {
        let (mut r, menu) = world();
        r.e.mem.set_u32(SKIP_OWNER, 0x9b00);
        r.stub(SKIP_TEST, 1);
        fn_0078aec0(&mut r.e, menu);
        assert!(r.calls_to(REFRESH_STATS).is_empty());
        assert_eq!(r.calls_to(SKIP_TEST), vec![vec![0x9b00]]);
        // A failing test refreshes.
        r.stub(SKIP_TEST, 0);
        fn_0078aec0(&mut r.e, menu);
        assert_eq!(r.calls_to(REFRESH_STATS).len(), 1);
    }

    // ----- 0078afa0 -----

    #[test]
    fn tips_are_switched_with_state_bit_6() {
        let (mut r, menu) = world();
        fn_0078afa0(&mut r.e, menu, 1);
        assert_ne!(r.e.get(menu, LoadingMenu::usFlags) & bit(6), 0);
        r.e.call(0x0078_afa0, &args![menu, 0u8]);
        assert_eq!(r.e.get(menu, LoadingMenu::usFlags) & bit(6), 0);
    }

    // ----- 0078afc0 and the four tile-list routines -----

    /// A menu whose five tile lists each hold one tile, with the `NiPointer`
    /// getter and the list step the way the game defines them.
    fn lists_world() -> (Rig, Ptr<LoadingMenu>, Vec<(u32, u32)>) {
        let (mut r, menu) = world();
        r.with(NI_POINTER_GET, |e, a| eax(e.mem.u32(a[0])));
        r.with(POINTER_LIST_STEP, |e, a| {
            let node = e.mem.u32(a[1]);
            let next = e.mem.u32(node);
            e.mem.set_u32(a[1], next);
            eax(node + 8)
        });
        let mut tiles = Vec::new();
        for list in [0xd0u32, 0xdc, 0xe8, 0xf4, 0x100] {
            let tile = r.object(0x40);
            let node = r.e.mem.alloc(12);
            r.e.mem.set_u32(node + 8, tile);
            r.e.mem.set_u32(menu.addr() + list, node);
            tiles.push((list, tile));
        }
        (r, menu, tiles)
    }

    /// The tiles `SET_TILE_FADE` was called for, with the value bits.
    fn faded(r: &Rig) -> Vec<(u32, u32)> {
        r.calls_to(SET_TILE_FADE)
            .iter()
            .map(|a| (a[1], a[2]))
            .collect()
    }

    fn tile_of(tiles: &[(u32, u32)], list: u32) -> u32 {
        tiles.iter().find(|t| t.0 == list).unwrap().1
    }

    #[test]
    fn each_screen_type_fades_its_own_tile_list() {
        let value = 0.5f32.to_bits();
        for (kind, list) in [(1u32, 0xe8u32), (2, 0xdc), (4, 0xd0)] {
            let (mut r, menu, tiles) = lists_world();
            r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
            r.stub(LOAD_SCREEN_TYPE_FOLDED, 1);
            r.stub(LOAD_SCREEN_TYPE, kind);
            fn_0078afc0(&mut r.e, menu, 0x8100, 0.5, 1);
            assert_eq!(
                faded(&r),
                vec![(tile_of(&tiles, list), value)],
                "type {kind}"
            );
            assert_eq!(r.calls_to(SET_TILE_FADE)[0][0], menu.addr());
            // The tile's 0xfa3 value was 0, so it was made 1 first.
            assert_eq!(
                r.calls_to(TILE_SET_INT),
                vec![vec![tile_of(&tiles, list), 0xfa3, 1]]
            );
        }
    }

    #[test]
    fn the_tip_type_fades_a_list_and_its_matte_by_mode() {
        let value = 0.25f32.to_bits();
        let (mut r, menu, tiles) = lists_world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
        r.stub(LOAD_SCREEN_TYPE_FOLDED, 1);
        r.stub(LOAD_SCREEN_TYPE, 3);
        fn_0078afc0(&mut r.e, menu, 0x8100, 0.25, 1);
        assert_eq!(
            faded(&r),
            vec![
                (tile_of(&tiles, 0xf4), value),
                (r.tile_at(menu, SLOT_MATTE_A), value)
            ]
        );
        r.clear_calls();
        fn_0078afc0(&mut r.e, menu, 0x8100, 0.25, 2);
        assert_eq!(
            faded(&r),
            vec![
                (tile_of(&tiles, 0x100), value),
                (r.tile_at(menu, SLOT_MATTE_B), value)
            ]
        );
        // Any other mode does nothing for it.
        r.clear_calls();
        fn_0078afc0(&mut r.e, menu, 0x8100, 0.25, 0);
        assert!(faded(&r).is_empty());
    }

    #[test]
    fn the_other_types_ignore_mode_2() {
        for kind in [1, 2, 4] {
            let (mut r, menu, _) = lists_world();
            r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
            r.stub(LOAD_SCREEN_TYPE_FOLDED, 1);
            r.stub(LOAD_SCREEN_TYPE, kind);
            fn_0078afc0(&mut r.e, menu, 0x8100, 0.5, 2);
            assert!(faded(&r).is_empty(), "type {kind}");
        }
    }

    #[test]
    fn unknown_screen_types_and_untyped_screens_fade_nothing() {
        let (mut r, menu, _) = lists_world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
        r.stub(LOAD_SCREEN_TYPE_FOLDED, 1);
        r.stub(LOAD_SCREEN_TYPE, 5);
        fn_0078afc0(&mut r.e, menu, 0x8100, 0.5, 1);
        r.stub(LOAD_SCREEN_TYPE_FOLDED, 0);
        r.stub(LOAD_SCREEN_TYPE, 1);
        fn_0078afc0(&mut r.e, menu, 0x8100, 0.5, 1);
        assert!(faded(&r).is_empty());
        assert_eq!(r.calls_to(LOAD_SCREEN_TYPE).len(), 1);
    }

    #[test]
    fn without_tips_nothing_is_looked_at() {
        let (mut r, menu, _) = lists_world();
        fn_0078afc0(&mut r.e, menu, 0x8100, 0.5, 1);
        assert!(r.calls_to(LOAD_SCREEN_TYPE_FOLDED).is_empty());
    }

    #[test]
    fn a_missing_screen_defaults_to_the_next_then_the_first() {
        let (mut r, menu, _) = lists_world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        let tail = r.e.mem.u32(menu.addr() + 0x1d4);
        let second = r.e.mem.u32(tail);
        r.e.set(menu, LoadingMenu::pNextLoadScreen, Ptr::new(tail));
        fn_0078afc0(&mut r.e, menu, 0, 0.5, 1);
        assert_eq!(r.calls_to(LOAD_SCREEN_TYPE_FOLDED), vec![vec![second]]);
        r.e.set(menu, LoadingMenu::pNextLoadScreen, Ptr::NULL);
        fn_0078afc0(&mut r.e, menu, 0, 0.5, 1);
        assert_eq!(r.calls_to(LOAD_SCREEN_TYPE_FOLDED)[1], vec![first]);
        // No screen anywhere: stop.
        r.e.mem.set_u32(menu.addr() + 0x1d0, 0);
        fn_0078afc0(&mut r.e, menu, 0, 0.5, 1);
        assert_eq!(r.calls_to(LOAD_SCREEN_TYPE_FOLDED).len(), 2);
    }

    #[test]
    fn a_tile_whose_value_is_set_is_not_forced_on() {
        let (mut r, menu, tiles) = lists_world();
        r.stub_float(TILE_GET_VALUE, 1.0);
        fn_0078b100(&mut r.e, menu, 0.5, 1);
        assert!(r.calls_to(TILE_SET_INT).is_empty());
        assert_eq!(faded(&r), vec![(tile_of(&tiles, 0xd0), 0.5f32.to_bits())]);
        assert_eq!(
            r.calls_to(TILE_GET_VALUE),
            vec![vec![tile_of(&tiles, 0xd0), 0xfa3]]
        );
    }

    #[test]
    fn empty_list_entries_are_skipped_and_every_tile_is_visited() {
        let (mut r, menu, tiles) = lists_world();
        // Chain a second node with a null element and a third with a tile.
        let head = r.e.mem.u32(menu.addr() + 0xdc);
        let extra = r.object(0x40);
        let third = r.e.mem.alloc(12);
        r.e.mem.set_u32(third + 8, extra);
        let second = r.e.mem.alloc(12);
        r.e.mem.set_u32(second, third);
        r.e.mem.set_u32(head, second);
        fn_0078b190(&mut r.e, menu, 1.0, 1);
        assert_eq!(
            faded(&r),
            vec![
                (tile_of(&tiles, 0xdc), 1.0f32.to_bits()),
                (extra, 1.0f32.to_bits())
            ]
        );
    }

    #[test]
    fn the_list_routines_need_mode_1_and_an_empty_list_is_fine() {
        let (mut r, menu, _) = lists_world();
        fn_0078b100(&mut r.e, menu, 0.5, 2);
        fn_0078b190(&mut r.e, menu, 0.5, 0);
        fn_0078b360(&mut r.e, menu, 0.5, 3);
        assert!(faded(&r).is_empty());
        // A list with no head visits nothing.
        r.e.mem.set_u32(menu.addr() + 0xe8, 0);
        fn_0078b360(&mut r.e, menu, 0.5, 1);
        assert!(faded(&r).is_empty());
        // The tip routine with an empty list still fades the matte.
        r.e.mem.set_u32(menu.addr() + 0xf4, 0);
        fn_0078b220(&mut r.e, menu, 0.5, 1);
        assert_eq!(
            faded(&r),
            vec![(r.tile_at(menu, SLOT_MATTE_A), 0.5f32.to_bits())]
        );
    }

    // ----- 0078b3f0 -----

    fn groups(r: &Rig) -> Vec<(u32, u32)> {
        r.calls
            .borrow()
            .iter()
            .filter(|(a, _)| {
                [
                    GROUP_VISIBLE_0078B6A0,
                    GROUP_VISIBLE_0078B650,
                    GROUP_VISIBLE_0078B6F0,
                    GROUP_VISIBLE_0078B7C0,
                ]
                .contains(a)
            })
            .map(|(a, args)| (*a, args[1]))
            .collect()
    }

    fn visibility(kind: u32, show: u8) -> Vec<(u32, u32)> {
        let (mut r, menu) = world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
        r.stub(LOAD_SCREEN_TYPE, kind);
        fn_0078b3f0(&mut r.e, menu, 0x8100, show);
        groups(&r)
    }

    #[test]
    fn the_group_of_the_screen_type_is_shown_and_the_others_hidden() {
        let (a, b, c, d) = (
            GROUP_VISIBLE_0078B6A0,
            GROUP_VISIBLE_0078B650,
            GROUP_VISIBLE_0078B6F0,
            GROUP_VISIBLE_0078B7C0,
        );
        assert_eq!(visibility(1, 1), vec![(a, 0), (b, 0), (c, 0), (d, 1)]);
        assert_eq!(visibility(2, 1), vec![(a, 1), (b, 0), (c, 0), (d, 0)]);
        assert_eq!(visibility(3, 1), vec![(a, 0), (b, 0), (c, 1), (d, 0)]);
        assert_eq!(visibility(4, 1), vec![(a, 0), (b, 1), (c, 0), (d, 0)]);
        // The byte is passed on as it is.
        assert_eq!(visibility(2, 5), vec![(a, 5), (b, 0), (c, 0), (d, 0)]);
    }

    #[test]
    fn unknown_types_hide_all_groups_and_hiding_passes_the_byte_to_all() {
        let (a, b, c, d) = (
            GROUP_VISIBLE_0078B6A0,
            GROUP_VISIBLE_0078B650,
            GROUP_VISIBLE_0078B6F0,
            GROUP_VISIBLE_0078B7C0,
        );
        assert_eq!(visibility(7, 1), vec![(a, 0), (b, 0), (c, 0), (d, 0)]);
        assert_eq!(visibility(3, 0), vec![(a, 0), (b, 0), (c, 0), (d, 0)]);
    }

    #[test]
    fn the_groups_are_left_alone_without_tips_or_a_screen() {
        let (mut r, menu) = world();
        fn_0078b3f0(&mut r.e, menu, 0x8100, 1);
        assert!(groups(&r).is_empty());
        r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
        r.e.set(menu, LoadingMenu::pNextLoadScreen, Ptr::NULL);
        r.e.mem.set_u32(menu.addr() + 0x1d0, 0);
        fn_0078b3f0(&mut r.e, menu, 0, 1);
        assert!(groups(&r).is_empty());
    }

    #[test]
    fn a_missing_screen_is_taken_from_the_list() {
        let (mut r, menu) = world();
        r.e.mem.set_u16(menu.addr() + 0x222, bit(6));
        r.stub(LOAD_SCREEN_TYPE, 4);
        let first = r.e.mem.u32(menu.addr() + 0x1d0);
        fn_0078b3f0(&mut r.e, menu, 0, 1);
        assert_eq!(r.calls_to(LOAD_SCREEN_TYPE), vec![vec![first]]);
        assert_eq!(groups(&r).len(), 4);
    }
}
