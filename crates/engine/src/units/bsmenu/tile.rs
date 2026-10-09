//! `BSMenu/tile.cpp` (Xbox PDB source unit), subsystem `BSMenu`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit is the menu system's `Tile` (a node of the interface tree), its
//! trait values (`Tile::Value`), the text table that maps XML words to
//! numbers (`Tile::MenuStringMap`) and the XML reader that builds a tile tree
//! from a menu file. It is a large unit, 164 functions in address order from
//! `009ff340`. Translated so far, in order:
//!
//! * session 1: `009ff340` to `00a03da0` (40 functions, up to and including
//!   `Tile::GetChildByName`); the next function to translate is
//!   `00a03eb0` (`Tile::GetChildByID`).
//!
//! Conventions of the whole unit:
//!
//! * Every tile lock (`Tile::Lock` `00a044f0`, `Tile::Unlock` `00a04500`)
//!   is a call by address; the Win32 critical section the trait code wraps
//!   is `011f3330`, entered and left through the import slots.
//! * Trait ids, tile types, update flags and template commands are the
//!   Xbox PDB enumerations (`Tile::enumTrait`, `enumType`, `enumbfUpdate`,
//!   `TEMPLATE_ID`); the PC build uses the same numbers.
//! * The compiler's exception-unwinding frames and stack cookies are not
//!   translated. Blocks the compiler folded away (`if (0)` compiled as
//!   `XOR ECX,ECX; JZ`) are left out and said so where they occur.
//! * Locals the game keeps on its stack and passes by address are
//!   [`Engine::with_stack`] blocks. The fixed-size character arrays of the
//!   XML reader are separate blocks here, not neighbours of the other
//!   locals as on the game's stack, so an overlong tag name overflows into
//!   its neighbour block instead of into the game's other locals.
//!
//! Layouts, constants and helpers a later session reuses are at the top of
//! this file.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleArray, BSSimpleList, BSStringT, NiTPointerList};
use crate::units::platform::MEMORY_MANAGER;
use crate::Inline;

// ---------------------------------------------------------------------------
// Layouts

layout! {
    /// `Tile` (Xbox PDB), 0x38 bytes on PC as on the Xbox. The offsets of
    /// the fields match the PDB.
    pub struct Tile: 0x38 {
        /// `xChildren` (Xbox PDB): `NiTPointerList<Tile *>`, head pointer at
        /// +4, item count at +0xC.
        0x04 xChildren: Inline<NiTPointerList>,
        /// `xTraits` (Xbox PDB): `BSSimpleArray<Tile::Value *, 8>`, kept
        /// sorted by trait id; buffer at +0x14, size at +0x18.
        0x10 xTraits: Inline<BSSimpleArray>,
        /// `xName` (Xbox PDB): `BSStringT<char>`.
        0x20 xName: Inline<BSStringT>,
        /// `pParent` (Xbox PDB).
        0x28 pParent: Ptr<Tile>,
        /// `spModel` (Xbox PDB): `NiPointer<NiNode>`.
        0x2C spModel: Ptr,
        /// `uiFlags` (Xbox PDB): the `enumbfUpdate` bits.
        0x30 uiFlags: u32,
        /// `bNeedsNiUpdate` (Xbox PDB).
        0x34 bNeedsNiUpdate: bool,
        /// `bSpeechChallengeFailure` (Xbox PDB).
        0x35 bSpeechChallengeFailure: bool,
    }

    /// `Tile::Value` (Xbox PDB), 0x14 bytes: one trait of a tile.
    pub struct TileValue: 0x14 {
        /// `eIndex` (Xbox PDB): the trait id.
        0x00 eIndex: i32,
        /// `pParent` (Xbox PDB): the owning tile.
        0x04 pParent: Ptr<Tile>,
        /// `fValue` (Xbox PDB).
        0x08 fValue: f32,
        /// `strValue` (Xbox PDB): `char *`.
        0x0C strValue: Ptr,
        /// `pActionListA` (Xbox PDB).
        0x10 pActionListA: Ptr,
    }

    /// `Tile::BuildStorage` (Xbox PDB), 0x14 bytes: what the XML reader
    /// fills.
    pub struct BuildStorage: 0x14 {
        /// `pTemplate` (Xbox PDB): the template being built.
        0x00 pTemplate: Ptr<TileTemplate>,
        /// `xSubTemplates` (Xbox PDB): `BSSimpleList<Tile::TileTemplate *>`,
        /// first node embedded (item at +4, next at +8).
        0x04 xSubTemplates: Inline<BSSimpleList>,
        /// `pCurrentTemplate` (Xbox PDB).
        0x0C pCurrentTemplate: Ptr<TileTemplate>,
        /// `bDeleteTemplates` (Xbox PDB).
        0x10 bDeleteTemplates: bool,
    }

    /// `Tile::TileTemplate` (Xbox PDB), 0x14 bytes.
    pub struct TileTemplate: 0x14 {
        /// `xName` (Xbox PDB): `NiFixedString`.
        0x00 xName: u32,
        /// `pParent` (Xbox PDB): the owning [`BuildStorage`].
        0x04 pParent: Ptr<BuildStorage>,
        /// `xList` (Xbox PDB): `NiTPointerList<Tile::TileTemplateItem *>`,
        /// head pointer at +8.
        0x08 xList: Inline<NiTPointerList>,
    }

    /// `Tile::TileTemplateItem` (Xbox PDB), 0x18 bytes: one parsed element.
    pub struct TileTemplateItem: 0x18 {
        /// `iCmd` (Xbox PDB): a `TEMPLATE_ID`.
        0x00 iCmd: i32,
        /// `fVal` (Xbox PDB).
        0x04 fVal: f32,
        /// `xStr` (Xbox PDB): `BSStringT<char>`, string at +8, length at +0xC.
        0x08 xStr: Inline<BSStringT>,
        /// `u` (Xbox PDB): a trait id, or (for tile items) the tile built
        /// from the item.
        0x10 u: u32,
        /// `iLine` (Xbox PDB).
        0x14 iLine: i32,
    }

    /// `XMLStorage` (Xbox PDB): a menu file read into memory.
    pub struct XmlStorage: 0x08 {
        /// `iFileSize` (Xbox PDB).
        0x00 iFileSize: u32,
        /// `pXMLData` (Xbox PDB).
        0x04 pXMLData: Ptr,
    }
}

// ---------------------------------------------------------------------------
// Constants

/// The win32 critical section the tile code enters around trait lists.
pub(crate) const TILE_CRITICAL_SECTION: u32 = 0x011f_3330;
/// `EnterCriticalSection` / `LeaveCriticalSection` import slots.
pub(crate) const ENTER_CRITICAL_SECTION: u32 = 0x00fd_f05c;
pub(crate) const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;
/// `InterlockedIncrement` / `InterlockedDecrement` import slots.
pub(crate) const INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
pub(crate) const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// `MemoryManager::Allocate(size)` and `Deallocate(block)` (methods of the
/// singleton at [`MEMORY_MANAGER`]).
const MEMORY_ALLOCATE: u32 = 0x00aa_3e40;
const MEMORY_DEALLOCATE: u32 = 0x00aa_4060;

/// `Tile::Lock()` and `Tile::Unlock()` (Xbox PDB): recursive lock around
/// the shared tile data (`cdecl`, no arguments).
pub(crate) const TILE_LOCK: u32 = 0x00a0_44f0;
pub(crate) const TILE_UNLOCK: u32 = 0x00a0_4500;
/// `Tile::SetParent(parent, flag)` (Xbox PDB).
const TILE_SET_PARENT: u32 = 0x00a0_87d0;
/// `Tile::RemoveFadeControl` (Xbox PDB): given `0x80000000` by `Release`.
const TILE_REMOVE_FADE_CONTROL: u32 = 0x00a0_7dc0;
/// `Tile::AddNeedsUpdate(flags)` (Xbox PDB), `thiscall`.
pub(crate) const TILE_ADD_NEEDS_UPDATE: u32 = 0x00a0_7530;
/// `Tile::UpdateChildren(bool)` (Xbox PDB), `thiscall`.
const TILE_UPDATE_CHILDREN: u32 = 0x00a0_4620;
/// `Tile::Value::~Value` (Xbox PDB): destroys a trait value in place.
const VALUE_DESTROY: u32 = 0x00a0_9330;
/// `Tile::Value::SetFloat(float, bool)` and `SetString(string, bool)`
/// (Xbox PDB).
const VALUE_SET_FLOAT: u32 = 0x00a0_a270;
const VALUE_SET_STRING: u32 = 0x00a0_a300;
/// `Tile::Value::AddAction(action, float)` (Xbox PDB), `thiscall`.
const VALUE_ADD_ACTION: u32 = 0x00a0_9080;
/// `Tile::Value::AddAction_ov2(action, tile, int)` (Xbox PDB).
const VALUE_ADD_ACTION_OV2: u32 = 0x00a0_9130;
/// `Tile::Value::CalculateValue(bool)` (Xbox PDB).
const VALUE_CALCULATE_VALUE: u32 = 0x00a0_9410;
/// `BSSimpleArray<Tile::Value *, 8>::SortedFind(&key, compare)` (Xbox PDB)
/// and its insert counterpart `(&value, compare)`; both are given
/// [`fn_00a01160`] as the comparison.
const VALUES_SORTED_FIND: u32 = 0x00a0_cb90;
const VALUES_SORTED_INSERT: u32 = 0x00a0_caf0;
/// `BSSimpleArray` clear `(this, free)` (no PDB name): empties the array
/// and, with the flag, frees its buffer; `Release` calls it with 1.
const VALUES_CLEAR: u32 = 0x0084_54f0;
/// Destructor of the `Tile::Value *` array member.
const VALUES_DESTROY: u32 = 0x0070_9eb0;
/// `NiTPointerList` pop from the head, returning the element.
const LIST_REMOVE_HEAD: u32 = 0x007b_5390;
/// `NiTPointerListBase<...>::RemoveAll` (Xbox PDB).
const LIST_REMOVE_ALL: u32 = 0x004e_d900;
/// `NiTPointerList::Remove(&item)` (no PDB name): unlinks the item whose
/// element equals the one passed by address and returns that element (the
/// word itself when it is not in the list). Used with the dirty-tile list
/// (`011f3318`) and the hibernating-tile list (`011f3324`).
const LIST_REMOVE_ITEM: u32 = 0x0057_c730;
const DIRTY_TILES_LIST: u32 = 0x011f_3318;
const HIBERNATING_TILES_LIST: u32 = 0x011f_3324;
/// `Tile::iHibernatingTileCount` (Xbox PDB).
const HIBERNATING_TILE_COUNT: u32 = 0x011f_32d8;
/// `Interface::TileIsBeingDeleted(tile)` (Xbox PDB, `cdecl`).
const INTERFACE_TILE_IS_BEING_DELETED: u32 = 0x0070_6c20;
/// `Interface::GetMenusRoot()` (Xbox PDB).
const INTERFACE_GET_MENUS_ROOT: u32 = 0x0070_6cd0;
/// `Interface::TestConstantForGameSettings(name, out)` (Xbox PDB, `cdecl`).
const INTERFACE_TEST_CONSTANT_FOR_GAME_SETTINGS: u32 = 0x0070_73d0;
/// The interface's "is widescreen" query used by the text table.
const INTERFACE_IS_WIDESCREEN: u32 = 0x0070_7b40;
/// `bHas360Controller` (Xbox PDB, static of the interface manager) and the
/// byte the text table tests next to it.
const HAS_360_CONTROLLER: u32 = 0x011d_8a84;
const CONSOLE_UI_BYTE: u32 = 0x011d_8c50;
/// `NiPointer<T>::operator=(ptr)`, called on the embedded pointer.
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `BSStringT<char>::Set(string, length)`: assigns (or, with `0, 0`, frees).
const STRING_SET: u32 = 0x0040_37f0;
/// `BSStringT<char>::operator_` (Xbox PDB): appends the string to the text.
const STRING_APPEND: u32 = 0x0040_4820;
/// `BSStringT<char>` assignment from another `BSStringT` (`(this, &other)`).
const STRING_ASSIGN: u32 = 0x0043_8470;
/// `sprintf`-like formatter into a `BSStringT` (`cdecl`, `(string, format,
/// ...)`).
const STRING_FORMAT: u32 = 0x0040_6f60;
/// Prints an error to the console and log (`cdecl`, format and arguments).
const PRINT_ERROR: u32 = 0x005b_5e40;
/// `NiObjectNET::GetExtraData(key)` (Xbox PDB).
const NI_OBJECT_GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
/// The extra-data key (an `NiFixedString` handle) the tile pointer is
/// stored under on a model.
const TILE_EXTRA_DATA_KEY: u32 = 0x011f_34cc;
/// `Menu::GetXML(path, flag)` (Xbox PDB, `cdecl`).
const MENU_GET_XML: u32 = 0x00a1_c9b0;
/// `Menu::GetMaxDepth()` (Xbox PDB).
const MENU_GET_MAX_DEPTH: u32 = 0x00a1_dfb0;
/// `(menu, template)` (no PDB name, `BSMenu/menu.cpp`): adds the template
/// to a list (`BSSimpleList::AddHead`); `ReadFile` calls it with each
/// sub-template of the parse storage.
const MENU_ADD_TEMPLATE: u32 = 0x00a1_dd80;
/// `XMLStorage::~XMLStorage` and its scalar deleting destructor
/// (`(this, 1)`, Xbox PDB).
const XML_STORAGE_DESTROY: u32 = 0x00a0_be70;
const XML_STORAGE_SCALAR_DELETING_DESTRUCTOR: u32 = 0x00a0_be10;
/// `Tile::BuildStorage::BuildStorage` / `~BuildStorage` (Xbox PDB).
const BUILD_STORAGE_CONSTRUCT: u32 = 0x00a0_ac80;
const BUILD_STORAGE_DESTROY: u32 = 0x00a0_ad40;
/// `Tile::TileTemplate::AddPair(command, text, line, flag)` (Xbox PDB).
const TEMPLATE_ADD_PAIR: u32 = 0x00a0_a410;
/// The tile path lookup `(tile, name)` (`cdecl`) used by action links.
const TILE_FIND_BY_PATH: u32 = 0x00a0_8b20;

/// The `Tile::MenuStringMap` text table (`xTextTable`, Xbox PDB static):
/// a `char *` to `int` hash map (bucket count at +4, buckets at +8).
const TEXT_TABLE: u32 = 0x011f_32f4;
const TEXT_TABLE_BUCKET_COUNT: u32 = 0x011f_32f8;
const TEXT_TABLE_BUCKETS: u32 = 0x011f_32fc;
/// The `int` to `int` pointer map `xTraitExtraData` (Xbox PDB static, matched
/// by its type and use): registration enters a user trait's id under the
/// number at the end of its name.
const TRAIT_EXTRA_DATA_MAP: u32 = 0x011f_3308;
/// `Tile::iTextTableUserNextID` (Xbox PDB).
const TEXT_TABLE_USER_NEXT_ID: u32 = 0x011a_6d74;
/// First id of a user-defined name.
const FIRST_USER_TEXT_ID: i32 = 10000;
/// `MenuStringMap` lookup `(this, name, &out)`: true and `*out` when found.
const TEXT_TABLE_FIND: u32 = 0x00a1_e380;
/// `MenuStringMap` insert `(this, name, id)`, the insert into
/// [`TRAIT_EXTRA_DATA_MAP`] `(this, id, number)` and the text table's
/// iterator step `(this, &node, &key, &value)`.
const TEXT_TABLE_ADD: u32 = 0x00a0_c900;
const TRAIT_EXTRA_DATA_ADD: u32 = 0x00a0_c6c0;
const TEXT_TABLE_NEXT: u32 = 0x00a0_c850;
/// `(name)` (`cdecl`, no PDB name): true for a name that starts with `_`
/// or with `&_` (the two bytes at `01093f80`), the spelling of the names
/// that users add.
const TEXT_IS_USER_NAME: u32 = 0x00a0_bb00;
/// Destroys a `Tile::TextureAtlasEntry`'s three strings (no PDB name).
const TEXTURE_ATLAS_ENTRY_DESTROY: u32 = 0x00a0_bb50;
/// Empties a hash map and a `BSSimpleList` (`this`; code shared with the audio
/// library by identical-code folding, no PDB names).
const MAP_REMOVE_ALL: u32 = 0x00ae_91f0;
const SIMPLE_LIST_REMOVE_ALL: u32 = 0x00ae_83e0;
/// Heads of the two static `BSSimpleList`s that [`tile_free_trait_list`]
/// empties: `TextureEntryList` (`BSSimpleList<Tile::TextureAtlasEntry *>`,
/// Xbox PDB static; its entries are destroyed with
/// [`TEXTURE_ATLAS_ENTRY_DESTROY`]) and a list whose items are only freed,
/// presumably `xFadeControls` (Xbox PDB static; matched by role: `Release`
/// removes fade controls).
const TEXTURE_ENTRY_LIST: u32 = 0x011f_3350;
const FADE_CONTROLS_LIST: u32 = 0x011f_3348;
/// The flag the XML reader sets when it hit an error.
const XML_ERROR_FLAG: u32 = 0x011f_32c8;
/// The pointer to the interface object whose lock (+0x80) the reader holds
/// while it updates the new tree.
const INTERFACE_LOCK_OWNER: u32 = 0x011f_4748;

/// Function-local statics of [`tile_get_value`] and
/// [`tile_get_or_create_value`]: a search key (a 0x14-byte `Tile::Value`),
/// its "initialized" guard word and the `atexit` destructor registered when
/// it is initialized.
const GET_VALUE_KEY: u32 = 0x011f_3374;
const GET_VALUE_KEY_GUARD: u32 = 0x011f_3388;
const GET_VALUE_KEY_DESTROY: u32 = 0x00fd_a290;
const GET_OR_CREATE_KEY: u32 = 0x011f_338c;
const GET_OR_CREATE_KEY_GUARD: u32 = 0x011f_33a0;
const GET_OR_CREATE_KEY_DESTROY: u32 = 0x00fd_a2a0;
/// `atexit(function)` (CRT).
const ATEXIT: u32 = 0x00ec_658f;

/// Vtables written by the constructors and destructor of the tile classes
/// (the first word of the object).
const VTABLE_TILE: u32 = 0x0106_ed9c;
const VTABLE_TILE_RECT: u32 = 0x0106_ed70;
const VTABLE_TILE_TEXT: u32 = 0x0109_4878;
const VTABLE_TILE_MENU: u32 = 0x0106_ed44;
/// Constructors used by [`tile_build_tile`]: the base-class constructor
/// `(this)`, `TileImage::TileImage` (Xbox PDB), the `Tile3D` constructor and
/// `RadialTile::RadialTile` (Xbox PDB).
const TILE_BASE_CONSTRUCT: u32 = 0x0070_9860;
const TILE_IMAGE_CONSTRUCT: u32 = 0x0070_b5d0;
const TILE_3D_CONSTRUCT: u32 = 0x00a0_bc80;
const RADIAL_TILE_CONSTRUCT: u32 = 0x00a2_1660;

/// The empty string a tile's name is initialized with.
const EMPTY_NAME: u32 = 0x0101_1584;
/// `"solid.dds"`, the texture file name of the hot-rect tile.
const SOLID_TEXTURE_NAME: u32 = 0x0109_4a70;
/// The word at `011f336c`: the default texture atlas name the hot-rect
/// tile gets.
const DEFAULT_ATLAS_NAME: u32 = 0x011f_336c;

/// The `%s` format of the XML errors and their message strings (all in
/// the exe's `.rdata`).
const XML_ERROR_FORMAT: u32 = 0x0109_4a08;
const MSG_BASE_TILE_NOT_RELEASED: u32 = 0x0109_3f44;
const MSG_UNABLE_TO_CREATE_TILE: u32 = 0x0109_4a88;
const MSG_EMPTY_TAG_NAME: u32 = 0x0109_4a3c;
const MSG_ATTRIBUTE_NO_VALUE: u32 = 0x0109_49f0;
const MSG_MISSING_ATTRIBUTE_NAME: u32 = 0x0109_49d8;
const MSG_UNEXPECTED_WORD: u32 = 0x0109_49b0;
const MSG_MISSING_ATTRIBUTE_VALUE: u32 = 0x0109_4970;
const MSG_CLOSE_TAG_MARKER: u32 = 0x0109_4934;
const MSG_UNBALANCED_CLOSE_PAIR: u32 = 0x0109_4904;
const MSG_UNBALANCED_END_OF_TAG: u32 = 0x0109_48dc;
const MSG_BUFFER_TOO_SMALL: u32 = 0x0109_498c;
const MSG_TRAIT_OUTSIDE_TILE: u32 = 0x0109_4b78;
const MSG_ACTION_OUTSIDE_TRAIT: u32 = 0x0109_4b4c;
const MSG_ACTION_LINK_OUTSIDE_TRAIT: u32 = 0x0109_4b18;
const MSG_ACTION_BEGUN_OUTSIDE_TRAIT: u32 = 0x0109_4aec;
const MSG_ACTION_ENDED_OUTSIDE_TRAIT: u32 = 0x0109_4ac0;

/// Doubles the code compares floats with: `0.0` (`01012060`), the
/// `Tile::enumTrait::eNone` marker `-2147483648.0` (`01094ba8`) and `906.0`
/// (`01094a80`, `eHotRect` as a number).
const ZERO: u32 = 0x0101_2060;
const NO_TRAIT_VALUE: u32 = 0x0109_4ba8;
const HOT_RECT_TYPE_VALUE: u32 = 0x0109_4a80;
/// Floats the hot-rect setup writes: `-1.0` (`01012054`) and `255.0`
/// (`01023cd8`).
const MINUS_ONE: u32 = 0x0101_2054;
const TWO_FIFTY_FIVE: u32 = 0x0102_3cd8;

/// The `"<include"` tag searched for, the `"` delimiter and the include
/// path format `"Data\Menus\Prefabs\%s"`.
const INCLUDE_TAG: u32 = 0x0109_4a64;
const QUOTE_DELIMITER: u32 = 0x0102_1184;
const PREFAB_PATH_FORMAT: u32 = 0x0109_4a4c;
/// Longest text the reader's string buffers hold before it complains.
const READ_BUFFER_LIMIT: i32 = 0x1000;
/// Size of the reader's tag-name buffer and of its text buffers.
const TAG_BUFFER_SIZE: u32 = 0x80;
const TEXT_BUFFER_SIZE: u32 = 0x1008;
/// Size of the buffer `fn_00a018d0` copies the part outside the brackets to.
const NAME_BUFFER_SIZE: u32 = 0x104;

/// CRT functions that have no translation yet (`strstr`, `strrchr`,
/// `strcmp`, `strtok_s`) and the translated ones the reader calls.
const STRSTR: u32 = 0x00ec_7750;
const STRRCHR: u32 = 0x00ec_6e30;
const STRCMP: u32 = 0x00ec_6da0;
const STRTOK_S: u32 = 0x00ec_c5f8;
const STRCHR: u32 = 0x00ec_7690;
const STRLEN: u32 = 0x00ec_6130;
const MEMSET: u32 = 0x00ec_61c0;
const STRCPY_S: u32 = 0x00ec_65a6;
const STRNCPY_S: u32 = 0x00ec_8d5f;
const ATOL: u32 = 0x00ec_a6d3;
/// `_ftol2_sse` (`00ec62c0`): truncates the float given as an `f64`.
const FTOL: u32 = 0x00ec_62c0;

/// Offset of the "current source line" word in the TLS block that
/// `Tile::Release` and `Tile::ReadFile` set to 13 for their duration.
const TLS_CURRENT_LINE: u32 = 0x2b4;

/// The names the text table registers whose value is computed at run time.
const XENON_NAME: u32 = 0x0106_f490;
const XBOX_NAME: u32 = 0x0106_f488;
const WIDESCREEN_NAME: u32 = 0x0109_47c8;

/// `Tile::enumType` (Xbox PDB): the tile types.
pub(crate) const TYPE_RECT: u32 = 0x385;
pub(crate) const TYPE_IMAGE: u32 = 0x386;
pub(crate) const TYPE_TEXT: u32 = 0x387;
pub(crate) const TYPE_3D: u32 = 0x388;
pub(crate) const TYPE_MENU: u32 = 0x389;
pub(crate) const TYPE_HOT_RECT: u32 = 0x38a;
pub(crate) const TYPE_RADIAL: u32 = 0x38c;

/// `Tile::enumbfUpdate` (Xbox PDB): the tile's update and state bits.
pub(crate) const UPDATE_POSITION: u32 = 0x1;
pub(crate) const UPDATE_CREATE: u32 = 0x2;
pub(crate) const UPDATE_VISIBILITY: u32 = 0x4;
pub(crate) const UPDATE_COLOR: u32 = 0x8;
pub(crate) const UPDATE_GEOMETRY: u32 = 0x10;
pub(crate) const UPDATE_TEXTURE: u32 = 0x20;
pub(crate) const UPDATE_NIF_FILE: u32 = 0x40;
pub(crate) const UPDATE_SCISSOR_WINDOW: u32 = 0x80;
pub(crate) const UPDATE_SCISSOR: u32 = 0x100;
pub(crate) const UPDATE_LOCUS: u32 = 0x200;
pub(crate) const FLAG_DIRTY: u32 = 0x400;
pub(crate) const FLAG_HIBERNATED: u32 = 0x800;
pub(crate) const FLAG_PROMOTED: u32 = 0x1000;
pub(crate) const FLAG_RELEASED: u32 = 0x2000;
pub(crate) const FLAG_MENU_DELETING: u32 = 0x4000;
pub(crate) const FLAG_TILE_LOADING: u32 = 0x10000;

/// `Tile::enumTrait` (Xbox PDB): the trait ids used so far.
pub(crate) const TRAIT_NONE: i32 = i32::MIN;
pub(crate) const TRAIT_X: i32 = 0xfa1;
pub(crate) const TRAIT_Y: i32 = 0xfa2;
pub(crate) const TRAIT_VISIBLE: i32 = 0xfa3;
pub(crate) const TRAIT_CLASS: i32 = 0xfa4;
pub(crate) const TRAIT_CLIP_WINDOW: i32 = 0xfa6;
pub(crate) const TRAIT_LOCUS: i32 = 0xfa8;
pub(crate) const TRAIT_ALPHA: i32 = 0xfa9;
pub(crate) const TRAIT_ID: i32 = 0xfaa;
pub(crate) const TRAIT_DEPTH: i32 = 0xfad;
pub(crate) const TRAIT_CLIPS: i32 = 0xfae;
pub(crate) const TRAIT_TARGET: i32 = 0xfaf;
pub(crate) const TRAIT_HEIGHT: i32 = 0xfb0;
pub(crate) const TRAIT_WIDTH: i32 = 0xfb1;
pub(crate) const TRAIT_RED: i32 = 0xfb2;
pub(crate) const TRAIT_GREEN: i32 = 0xfb3;
pub(crate) const TRAIT_BLUE: i32 = 0xfb4;
pub(crate) const TRAIT_JUSTIFY: i32 = 0xfb7;
pub(crate) const TRAIT_FONT: i32 = 0xfb9;
pub(crate) const TRAIT_WRAP_WIDTH: i32 = 0xfba;
pub(crate) const TRAIT_WRAP_LIMIT: i32 = 0xfbb;
pub(crate) const TRAIT_WRAP_LINES: i32 = 0xfbc;
pub(crate) const TRAIT_PAGE_NUM: i32 = 0xfbd;
pub(crate) const TRAIT_IS_HTML: i32 = 0xfbe;
pub(crate) const TRAIT_CROP_Y: i32 = 0xfbf;
pub(crate) const TRAIT_CROP_X: i32 = 0xfc0;
pub(crate) const TRAIT_STRING: i32 = 0xfc4;
pub(crate) const TRAIT_FILENAME: i32 = 0xfcc;
pub(crate) const TRAIT_SYSTEM_COLOR: i32 = 0xff4;
pub(crate) const TRAIT_BRIGHTNESS: i32 = 0xff5;
pub(crate) const TRAIT_LINE_GAP: i32 = 0xff7;
pub(crate) const TRAIT_TEX_ATLAS: i32 = 0xff9;
pub(crate) const TRAIT_ROTATE_ANGLE: i32 = 0xffa;
pub(crate) const TRAIT_ROTATE_AXIS_X: i32 = 0xffb;
pub(crate) const TRAIT_ROTATE_AXIS_Y: i32 = 0xffc;

/// `Tile::enumTag` (Xbox PDB): the tags for `<value>` text and for the
/// `name` attribute.
const TAG_VALUE: i32 = 0xbb9;
const TAG_NAME: i32 = 0xbba;

/// `Tile::VALUE_ACTION` (Xbox PDB): the left and right parenthesis actions.
const ACTION_LEFT_PAREN: i32 = 0x7e8;
const ACTION_RIGHT_PAREN: i32 = 0x7e9;

/// `Tile::TEMPLATE_ID` (Xbox PDB): the `iCmd` of a template item.
const TI_ACTION_START: i32 = 2;
const TI_ACTION_END: i32 = 3;
const TI_TRAIT_START: i32 = 4;
const TI_TRAIT_END: i32 = 5;
const TI_TILE_START: i32 = 6;
const TI_TILE_END: i32 = 7;
const TI_SIMPLE_TRAIT: i32 = 8;
const TI_SIMPLE_ACTION: i32 = 9;
const TI_TRAIT_LINK: i32 = 10;

// ---------------------------------------------------------------------------
// Helpers

fn lock(e: &mut Engine) {
    e.call(TILE_LOCK, &args![]);
}

fn unlock(e: &mut Engine) {
    e.call(TILE_UNLOCK, &args![]);
}

fn enter_critical_section(e: &mut Engine, section: u32) {
    e.call(ENTER_CRITICAL_SECTION, &args![section]);
}

fn leave_critical_section(e: &mut Engine, section: u32) {
    e.call(LEAVE_CRITICAL_SECTION, &args![section]);
}

/// `new`: a block from the memory manager singleton.
fn allocate(e: &mut Engine, size: u32) -> Ptr {
    e.call(MEMORY_ALLOCATE, &args![MEMORY_MANAGER, size]).ptr()
}

/// `delete`: gives a block back to the memory manager singleton.
fn deallocate(e: &mut Engine, block: u32) {
    e.call(MEMORY_DEALLOCATE, &args![MEMORY_MANAGER, block]);
}

/// True when a `float` differs from the `0.0` double in `.rdata` (or is not
/// a number), as the code's `FCOMP` / `TEST AH,44` idiom tests it.
fn differs_from_zero(e: &Engine, value: f32) -> bool {
    value as f64 != e.global::<f64>(ZERO)
}

/// `FISTP` of a `float` in the default rounding mode: round to nearest,
/// ties to even; out of range or not a number gives the "integer
/// indefinite" value `0x80000000`.
fn float_to_int_rounded(value: f32) -> i32 {
    let value = value as f64;
    let below = value.floor();
    let rounded = if value - below > 0.5 || (value - below == 0.5 && below % 2.0 != 0.0) {
        below + 1.0
    } else {
        below
    };
    if rounded.is_nan() || !(-2147483648.0..2147483648.0).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// `_ftol2_sse` on a `float`.
fn float_to_int(e: &mut Engine, value: f32) -> i32 {
    e.call(FTOL, &args![value as f64]).i32()
}

/// The tile's virtual `GetType` (slot +0xC, Xbox PDB `Tile::GetType`).
fn tile_type(e: &mut Engine, tile: Ptr<Tile>) -> u32 {
    e.vcall(tile.addr(), 0xc, &[]).u32()
}

/// The template a `BuildStorage` adds items to: the current sub-template,
/// else the main one.
fn storage_template(e: &Engine, storage: Ptr<BuildStorage>) -> Ptr<TileTemplate> {
    let current = e.get(storage, BuildStorage::pCurrentTemplate);
    if current.is_null() {
        e.get(storage, BuildStorage::pTemplate)
    } else {
        current
    }
}

/// `NiPointer::~NiPointer` on a raw pointer: drops one reference through
/// `InterlockedDecrement` and, at zero, calls the object's virtual
/// destructor (slot +4).
fn release_reference(e: &mut Engine, object: u32) {
    if object == 0 {
        return;
    }
    if e.call(INTERLOCKED_DECREMENT, &args![object + 4]).i32() == 0 {
        e.vcall(object, 4, &[]);
    }
}

/// The names `Tile::LoadTextTable` registers, as `(name address, value)`,
/// in the order the game registers them. Three values are computed at run
/// time and are 0 here (`&xenon;`, `&xbox;`, `&widescreen;`).
const TEXT_TABLE_ENTRIES: &[(u32, i32)] = &[
    (0x0109_4868, -0x1),
    (0x0106_ec74, 0x0),
    (0x0106_ec7c, 0x1),
    (0x0109_4860, 0x1),
    (0x0109_4854, 0x2),
    (0x0109_484c, 0x4),
    (0x0109_4844, 0x5),
    (0x0109_483c, 0x6),
    (0x0109_4834, -0x1),
    (0x0109_4824, 0x65),
    (0x0109_4814, 0x66),
    (0x0109_4800, 0x1778),
    (0x0109_47f0, 0x67),
    (XENON_NAME, 0),
    (0x0109_47e4, 0x0),
    (XBOX_NAME, 0),
    (0x0109_47d8, 0x1),
    (WIDESCREEN_NAME, 0),
    (0x0109_47b8, 0x6e),
    (0x0109_47a8, 0x6f),
    (0x0109_4798, 0x70),
    (0x0109_4788, -0x1),
    (0x0109_4778, 0x0),
    (0x0109_476c, -0x1),
    (0x0109_4758, 0x9),
    (0x0109_474c, 0x9),
    (0x0109_4740, 0xa),
    (0x0109_4734, 0xb),
    (0x0109_4728, 0xc),
    (0x0109_471c, 0xd),
    (0x0109_4710, 0xe),
    (0x0109_4704, 0xf),
    (0x0109_46f8, 0x10),
    (0x0107_7c78, 0x11),
    (0x0107_7c6c, 0x12),
    (0x0109_46ec, 0x11),
    (0x0109_46e0, 0x12),
    (0x0109_46d8, 0x386),
    (0x0109_46d0, 0x389),
    (0x0109_46cc, 0x388),
    (0x0109_46c8, 0x388),
    (0x0109_46c0, 0x385),
    (0x0109_46b8, 0x38a),
    (0x0109_46ac, 0x3e7),
    (0x0103_c83c, 0x387),
    (0x0109_46a4, 0x38b),
    (0x0109_469c, 0x38c),
    (0x0109_4690, 0x3f9),
    (0x0109_4684, 0x402),
    (0x0109_4674, 0x418),
    (0x0109_4664, 0x3f0),
    (0x0109_4654, 0x3f1),
    (0x0109_4644, 0x3fc),
    (0x0109_4634, 0x3ec),
    (0x0109_4624, 0x3ea),
    (0x0109_4614, 0x3ef),
    (0x0109_4604, 0x3f6),
    (0x0109_45f8, 0x3ff),
    (0x0109_45e8, 0x3e9),
    (0x0109_45d8, 0x423),
    (0x0109_45c8, 0x41b),
    (0x0109_45bc, 0x3f5),
    (0x0109_45ac, 0x3f8),
    (0x0109_459c, 0x40c),
    (0x0109_458c, 0x3f4),
    (0x0109_4580, 0x3eb),
    (0x0109_4574, 0x3fa),
    (0x0109_4564, 0x403),
    (0x0109_4554, 0x40b),
    (0x0109_4544, 0x425),
    (0x0109_452c, 0x422),
    (0x0109_451c, 0x417),
    (0x0109_450c, 0x41d),
    (0x0109_44fc, 0x41e),
    (0x0109_44ec, 0x41f),
    (0x0109_44dc, 0x421),
    (0x0109_44d0, 0x420),
    (0x0109_44bc, 0x424),
    (0x0109_44a4, 0x433),
    (0x0109_4490, 0x432),
    (0x0109_447c, 0x434),
    (0x0109_4468, 0x438),
    (0x0109_4458, 0x439),
    (0x0109_4448, 0x43a),
    (0x0109_4438, 0x435),
    (0x0109_4428, 0x43b),
    (0x0109_441c, 0x43c),
    (0x0109_4418, 0x7da),
    (0x0109_4414, 0x7d1),
    (0x0109_4410, 0x7e2),
    (0x0109_4408, 0x7d9),
    (0x0109_4400, 0x7d0),
    (0x0109_43fc, 0x7d4),
    (0x0109_43f8, 0x7de),
    (0x0109_43f0, 0x7d8),
    (0x0109_43ec, 0x7dc),
    (0x0109_43e8, 0x7dd),
    (0x0107_6f38, 0x7e0),
    (0x0109_43e4, 0x7e1),
    (0x0109_43e0, 0x7d6),
    (0x0109_43dc, 0x7d5),
    (0x0109_43d8, 0x7d7),
    (0x0109_43d4, 0x7d3),
    (0x0109_43cc, 0x7d3),
    (0x0109_43c8, 0x7df),
    (0x0109_43c4, 0x7e4),
    (0x0109_43bc, 0x7e5),
    (0x0109_43b0, 0x7e6),
    (0x0109_43ac, 0x7e3),
    (0x0104_4c0c, 0x7e7),
    (0x0109_43a4, 0x7db),
    (0x0109_43a0, 0x7d2),
    (0x0109_4398, 0xbba),
    (0x0109_4394, 0xbbb),
    (0x0109_438c, 0xbbc),
    (0x0109_4384, 0xbb9),
    (0x0103_a454, 0xfa9),
    (0x0109_4378, 0xfd2),
    (0x0109_4370, 0xfb4),
    (0x0109_4364, 0xff5),
    (0x0109_4358, 0xfb6),
    (0x0109_434c, 0xfb6),
    (0x0109_4344, 0xfa4),
    (0x0109_433c, 0xfc7),
    (0x0109_4330, 0xfcb),
    (0x0109_4320, 0xfe8),
    (0x0109_4314, 0xfab),
    (0x0109_4308, 0xfc0),
    (0x0109_42fc, 0xfbf),
    (0x0109_42f4, 0xfc0),
    (0x0109_42ec, 0xfbf),
    (0x0109_42e4, 0xfae),
    (0x0109_42d8, 0xfa6),
    (0x0103_c034, 0xfad),
    (0x0109_42cc, 0xfe9),
    (0x0109_42c0, 0xfee),
    (0x0109_42b4, 0xfef),
    (0x0109_42a8, 0xfea),
    (0x0109_429c, 0xfeb),
    (0x0109_4290, 0xfec),
    (0x0109_4284, 0xfed),
    (0x0109_427c, 0xff0),
    (0x0109_4274, 0xff1),
    (0x0109_4268, 0xfc2),
    (0x0109_425c, 0xfce),
    (0x0109_4250, 0xfcc),
    (0x0109_4244, 0xfcd),
    (0x0109_423c, 0xfb9),
    (0x0109_4234, 0xfb3),
    (0x0109_422c, 0xfb0),
    (0x0109_4228, 0xfaa),
    (0x0109_4220, 0xfbe),
    (0x0109_4218, 0xfb7),
    (0x0109_420c, 0xfd4),
    (0x0109_4204, 0xff7),
    (0x0109_41f8, 0xfac),
    (0x0109_41f0, 0xfa8),
    (0x0109_41e4, 0xfc1),
    (0x0109_41d8, 0xfc3),
    (0x0109_41cc, 0xfd5),
    (0x0109_41c4, 0xfbd),
    (0x0109_41c0, 0xfb2),
    (0x0109_41b0, 0xfcf),
    (0x0109_419c, 0xfd0),
    (0x0109_4190, 0xffa),
    (0x0109_4184, 0xffb),
    (0x0109_4178, 0xffc),
    (0x0109_4168, 0xfc5),
    (0x0109_4158, 0xfa7),
    (0x0109_4150, 0xfc4),
    (0x0109_4144, 0xff4),
    (0x0108_dff8, 0xfaf),
    (0x0109_4138, 0xff9),
    (0x0109_4130, 0xfb5),
    (0x0109_4128, 0x1004),
    (0x0109_4120, 0x1005),
    (0x0109_4118, 0x1006),
    (0x0109_4110, 0x1007),
    (0x0109_4108, 0x1008),
    (0x0109_4100, 0x1009),
    (0x0109_40f8, 0x100a),
    (0x0109_40f0, 0x100b),
    (0x0109_40e8, 0x100c),
    (0x0109_40e0, 0x100d),
    (0x0109_40d8, 0x100e),
    (0x0109_40d0, 0x100f),
    (0x0109_40c8, 0x1010),
    (0x0109_40c0, 0x1011),
    (0x0109_40b8, 0x1012),
    (0x0109_40b0, 0x1013),
    (0x0109_40a8, 0x1014),
    (0x0106_fc9c, 0xfa3),
    (0x0109_409c, 0xff2),
    (0x0109_4090, 0xff3),
    (0x0109_4088, 0xfb1),
    (0x0109_407c, 0xfbb),
    (0x0109_4070, 0xfbc),
    (0x0109_4064, 0xfba),
    (0x0109_4058, 0xfdd),
    (0x0109_404c, 0xfde),
    (0x0109_4040, 0xfdf),
    (0x0109_4034, 0xfe0),
    (0x0109_4028, 0xfe1),
    (0x0109_401c, 0xfe2),
    (0x0109_4010, 0xfe3),
    (0x0109_4004, 0xfe4),
    (0x0109_3ff4, 0xfe7),
    (0x0109_3fe8, 0xfd6),
    (0x0109_3fe4, 0xfd7),
    (0x0109_3fdc, 0xfd8),
    (0x0109_3fd4, 0xfd9),
    (0x0109_3fcc, 0xfda),
    (0x0103_45e4, 0xfa1),
    (0x0107_6f20, 0xfa2),
    (0x0109_3fc4, 0xfb8),
    (0x0109_3fb0, 0xff8),
    (0x0108_6f50, 0x138d),
    (0x0109_3fac, 0x138a),
    (0x0109_3fa8, 0x1390),
    (0x0109_3fa0, 0x1389),
    (0x0107_8bc0, 0x138e),
    (0x0109_3f98, 0x138c),
    (0x0109_3f8c, 0x1391),
    (0x0109_3f84, 0x138f),
];

/// `Tile::SortedFind` comparison of two trait values; the address is also
/// what [`tile_get_value`] hands to the sorted-array functions.
const COMPARE_VALUES: u32 = 0x00a0_1160;
/// Called with the tile's model while the critical section is held, from
/// the destructor (no Xbox PDB name; the `BSMain` range).
const MODEL_HOOK_00C45830: u32 = 0x00c4_5830;

/// `LIST_REMOVE_ITEM` on `list` with the tile pointer passed by address;
/// true when the answer is not null (it answers the element, which is the
/// tile itself whether or not it was in the list).
fn remove_from_list(e: &mut Engine, list: u32, tile: Ptr<Tile>) -> bool {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), tile.addr());
        e.call(LIST_REMOVE_ITEM, &args![list, slot]).u32() != 0
    })
}

// Translated from 009ff340 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::~Tile` (Xbox PDB): under the tile critical section, tells the
/// interface the tile is going away, releases it first if nobody did
/// (printing a complaint), takes it out of the dirty and hibernating lists,
/// then drops the model and destroys the name, traits and children.
///
/// The code the compiler folded away (setting `FLAG_DIRTY`, `FLAG_HIBERNATED`
/// and `FLAG_PROMOTED` instead of clearing them) is not translated.
pub fn tile_destructor(e: &mut Engine, this: Ptr<Tile>) {
    e.mem.set_u32(this.addr(), VTABLE_TILE);
    enter_critical_section(e, TILE_CRITICAL_SECTION);
    e.call(INTERFACE_TILE_IS_BEING_DELETED, &args![this]);
    if e.get(this, Tile::uiFlags) & FLAG_RELEASED == 0 {
        e.call(PRINT_ERROR, &args![MSG_BASE_TILE_NOT_RELEASED]);
        tile_release(e, this);
    }
    if e.get(this, Tile::uiFlags) & FLAG_DIRTY != 0 {
        remove_from_list(e, DIRTY_TILES_LIST, this);
    }
    let flags = e.get(this, Tile::uiFlags);
    e.set(this, Tile::uiFlags, flags & !FLAG_DIRTY);
    if remove_from_list(e, HIBERNATING_TILES_LIST, this)
        && e.get(this, Tile::uiFlags) & FLAG_HIBERNATED != 0
    {
        let count: u32 = e.global(HIBERNATING_TILE_COUNT);
        e.set_global(HIBERNATING_TILE_COUNT, count.wrapping_sub(1));
    }
    let flags = e.get(this, Tile::uiFlags);
    e.set(this, Tile::uiFlags, flags & !FLAG_HIBERNATED);
    let flags = e.get(this, Tile::uiFlags);
    e.set(this, Tile::uiFlags, flags & !FLAG_PROMOTED);
    let model = e.get(this, Tile::spModel);
    e.call(MODEL_HOOK_00C45830, &args![model]);
    leave_critical_section(e, TILE_CRITICAL_SECTION);
    release_reference(e, e.get(this, Tile::spModel).addr());
    e.call(STRING_SET, &args![this.at(Tile::xName), 0u32, 0u32]);
    e.call(VALUES_DESTROY, &args![this.at(Tile::xTraits)]);
    e.call(LIST_REMOVE_ALL, &args![this.at(Tile::xChildren)]);
}

// Translated from 009ff600 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::Init` (Xbox PDB): clears the parent, model, flags and update
/// flag, sets the name to the empty string, attaches the tile to `parent`
/// (when given, passing `set_parent_arg` through) and gives it `name` (when
/// given).
pub fn tile_init(
    e: &mut Engine,
    this: Ptr<Tile>,
    parent: Ptr<Tile>,
    name: Ptr,
    set_parent_arg: u32,
) {
    e.set(this, Tile::pParent, Ptr::NULL);
    e.call(NI_POINTER_ASSIGN, &args![this.byte_add(0x2c), 0u32]);
    e.call(STRING_SET, &args![this.at(Tile::xName), EMPTY_NAME, 0u32]);
    e.set(this, Tile::bSpeechChallengeFailure, false);
    e.set(this, Tile::uiFlags, 0);
    if !parent.is_null() {
        e.call(TILE_SET_PARENT, &args![this, parent, set_parent_arg]);
    }
    if !name.is_null() {
        e.call(STRING_SET, &args![this.at(Tile::xName), name, 0u32]);
    }
}

// Translated from 009ff690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::Release` (Xbox PDB): the tile's teardown, under `Tile::Lock` and
/// with the thread's current-line word set to 13: marks the tile and its
/// children as being deleted, tells the interface, sets `FLAG_RELEASED`,
/// severs the model's pointer back to the tile, detaches from the parent,
/// destroys and frees every trait value, empties the trait array, drops the
/// fade control, detaches the model from its parent node and deletes every
/// child.
pub fn tile_release(e: &mut Engine, this: Ptr<Tile>) {
    let tls = e.tls() + TLS_CURRENT_LINE;
    let saved_line = e.mem.u32(tls);
    e.mem.set_u32(tls, 0xd);
    lock(e);
    tile_set_menu_deleting(e, this);
    e.call(INTERFACE_TILE_IS_BEING_DELETED, &args![this]);
    let flags = e.get(this, Tile::uiFlags);
    e.set(this, Tile::uiFlags, flags | FLAG_RELEASED);
    tile_sever_extra_data(e, this);
    e.call(TILE_SET_PARENT, &args![this, 0u32, 0u32]);

    let traits = this.at(Tile::xTraits);
    let mut index = 0;
    while index < e.get(traits, BSSimpleArray::iSize) {
        let buffer = e.get(traits, BSSimpleArray::pBuffer);
        let value = e.mem.u32(buffer + index * 4);
        if value != 0 {
            e.call(VALUE_DESTROY, &args![value]);
            deallocate(e, value);
        }
        index += 1;
    }
    e.call(VALUES_CLEAR, &args![traits, 1u32]);
    e.call(TILE_REMOVE_FADE_CONTROL, &args![this, 0x8000_0000u32]);

    // The model's parent node removes the model (virtual slot +0xE8).
    let model = e.get(this, Tile::spModel);
    if !model.is_null() {
        let parent_node = e.mem.u32(model.addr() + 0x18);
        if parent_node != 0 {
            e.vcall(parent_node, 0xe8, &args![model]);
        }
    }

    let children = this.at(Tile::xChildren);
    while e.get(children, NiTPointerList::m_uiCount) != 0 {
        let child = e.call(LIST_REMOVE_HEAD, &args![children]).u32();
        if child != 0 {
            // Scalar deleting destructor, slot 0, with the "delete" flag.
            e.vcall(child, 0, &args![1u32]);
        }
    }
    unlock(e);
    e.mem.set_u32(tls, saved_line);
}

// Translated from 009ff8a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Registers `name` in the text table with `id` (nothing for an empty or
/// missing name). A user trait name (one `TEXT_IS_USER_NAME` accepts) with an
/// id from 10000 up also raises the next free user id past `id` and enters
/// the id in the by-id map under the number at the end of the name (after
/// the last `_`), when it has one.
pub fn fn_009ff8a0(e: &mut Engine, name: Ptr, id: i32) {
    if name.is_null() || e.mem.i8(name.addr()) == 0 {
        return;
    }
    lock(e);
    e.call(TEXT_TABLE_ADD, &args![TEXT_TABLE, name, id]);
    if e.call(TEXT_IS_USER_NAME, &args![name]).bool() && id >= FIRST_USER_TEXT_ID {
        if e.global::<i32>(TEXT_TABLE_USER_NEXT_ID) <= id {
            e.set_global(TEXT_TABLE_USER_NEXT_ID, id.wrapping_add(1));
        }
        let suffix = fn_00a01a20(e, name);
        if suffix != 0 {
            e.call(
                TRAIT_EXTRA_DATA_ADD,
                &args![TRAIT_EXTRA_DATA_MAP, id, suffix],
            );
        }
    }
    unlock(e);
}

// Translated from 009ff970 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::LoadTextTable` (Xbox PDB): fills the text table with the words
/// the menu XML uses (`&true;`, tile types, menu names, trait names,
/// keywords), 224 of them in a fixed order ([`TEXT_TABLE_ENTRIES`]). The
/// values of `&xenon;` and `&xbox;` (a 360 controller is present and the
/// console-UI byte is 0) and `&widescreen;` (the interface's query) are
/// computed when their turn comes.
pub fn tile_load_text_table(e: &mut Engine) {
    for &(name, value) in TEXT_TABLE_ENTRIES {
        let value = match name {
            XENON_NAME | XBOX_NAME => {
                (e.global::<u8>(HAS_360_CONTROLLER) != 0 && e.global::<u8>(CONSOLE_UI_BYTE) == 0)
                    as i32
            }
            WIDESCREEN_NAME => e.call(INTERFACE_IS_WIDESCREEN, &args![]).u8() as i32,
            _ => value,
        };
        fn_009ff8a0(e, Ptr::new(name), value);
    }
}

// Translated from 00a00940 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::AddUserTrait` (Xbox PDB): the id of `name` in the text table; a
/// new name gets `id` (the next free user id when `id` is -1) if that is a
/// valid user id (from 10000 up, and `TEXT_IS_USER_NAME` accepts the name),
/// else the result is `0x80000000` (`eNone`).
pub fn tile_add_user_trait(e: &mut Engine, name: Ptr, id: i32) -> i32 {
    lock(e);
    let (found, existing) = e.with_stack(4, |e, out| {
        let found = e
            .call(TEXT_TABLE_FIND, &args![TEXT_TABLE, name, out])
            .bool();
        (found, e.mem.i32(out.addr()))
    });
    if found {
        unlock(e);
        return existing;
    }
    let mut id = id;
    if id == -1 {
        id = e.global(TEXT_TABLE_USER_NEXT_ID);
        e.set_global(TEXT_TABLE_USER_NEXT_ID, id.wrapping_add(1));
    }
    let result = if id >= FIRST_USER_TEXT_ID && e.call(TEXT_IS_USER_NAME, &args![name]).bool() {
        fn_009ff8a0(e, name, id);
        id
    } else {
        TRAIT_NONE
    };
    unlock(e);
    result
}

// Translated from 00a00a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::FreeTraitList` (Xbox PDB): empties the text table and the
/// trait-extra-data map, destroys and frees every entry of the texture atlas
/// entry list and frees (without a destructor) every item of the second
/// static list, then empties both lists.
pub fn tile_free_trait_list(e: &mut Engine) {
    lock(e);
    e.call(MAP_REMOVE_ALL, &args![TEXT_TABLE]);
    e.call(MAP_REMOVE_ALL, &args![TRAIT_EXTRA_DATA_MAP]);
    let mut node = TEXTURE_ENTRY_LIST;
    while node != 0 && e.mem.u32(node) != 0 {
        let item = e.mem.u32(node);
        e.call(TEXTURE_ATLAS_ENTRY_DESTROY, &args![item]);
        deallocate(e, item);
        node = e.mem.u32(node + 4);
    }
    e.call(SIMPLE_LIST_REMOVE_ALL, &args![TEXTURE_ENTRY_LIST]);
    let mut node = FADE_CONTROLS_LIST;
    while node != 0 && e.mem.u32(node) != 0 {
        let item = e.mem.u32(node);
        deallocate(e, item);
        node = e.mem.u32(node + 4);
    }
    e.call(SIMPLE_LIST_REMOVE_ALL, &args![FADE_CONTROLS_LIST]);
    unlock(e);
}

// Translated from 00a00b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::SetMenuDeleting` (Xbox PDB): sets `FLAG_MENU_DELETING` on the
/// tile and, recursively, on its children, unless the tile has been
/// released or is already marked.
pub fn tile_set_menu_deleting(e: &mut Engine, this: Ptr<Tile>) {
    lock(e);
    let flags = e.get(this, Tile::uiFlags);
    if flags & FLAG_RELEASED == 0 && flags & FLAG_MENU_DELETING == 0 {
        e.set(this, Tile::uiFlags, flags | FLAG_MENU_DELETING);
        let mut node = e.get(this.at(Tile::xChildren), NiTPointerList::m_pkHead);
        while node != 0 {
            let child = e.mem.u32(node + 8);
            node = e.mem.u32(node);
            tile_set_menu_deleting(e, Ptr::new(child));
        }
    }
    unlock(e);
}

// Translated from 00a00c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::BuildTile` (Xbox PDB): allocates and constructs a tile of the
/// type `kind` (`TYPE_*`): the rect (0x3c bytes), the image and hot rect
/// (0x48), the text (0x4c, vtable set and a trailing byte cleared), the 3D
/// tile (0x50), the menu (0x40, last word cleared) and the radial tile
/// (0x48). Any other type (the window, template and unknown types) gives
/// null, as does a failed allocation.
///
/// The compiler folded away a call to `SetParent(0, 0)` in the text case.
pub fn tile_build_tile(e: &mut Engine, kind: u32) -> Ptr {
    let size = match kind {
        TYPE_RECT => 0x3c,
        TYPE_IMAGE | TYPE_HOT_RECT | TYPE_RADIAL => 0x48,
        TYPE_TEXT => 0x4c,
        TYPE_3D => 0x50,
        TYPE_MENU => 0x40,
        _ => return Ptr::NULL,
    };
    let block = allocate(e, size);
    if block.is_null() {
        return Ptr::NULL;
    }
    match kind {
        TYPE_RECT => {
            e.call(TILE_BASE_CONSTRUCT, &args![block]);
            e.mem.set_u32(block.addr(), VTABLE_TILE_RECT);
            block
        }
        TYPE_IMAGE | TYPE_HOT_RECT => e.call(TILE_IMAGE_CONSTRUCT, &args![block]).ptr(),
        TYPE_TEXT => {
            e.call(TILE_BASE_CONSTRUCT, &args![block]);
            e.mem.set_u32(block.addr(), VTABLE_TILE_TEXT);
            e.mem.set_u8(block.addr() + 0x48, 0);
            block
        }
        TYPE_3D => e.call(TILE_3D_CONSTRUCT, &args![block]).ptr(),
        TYPE_MENU => {
            e.call(TILE_BASE_CONSTRUCT, &args![block]);
            // The rect vtable is written first and replaced at once.
            e.mem.set_u32(block.addr(), VTABLE_TILE_RECT);
            e.mem.set_u32(block.addr(), VTABLE_TILE_MENU);
            e.mem.set_u32(block.addr() + 0x3c, 0);
            block
        }
        _ => e.call(RADIAL_TILE_CONSTRUCT, &args![block]).ptr(),
    }
}

// Translated from 00a00e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetValueQ` (Xbox PDB): the trait value with id `trait_id`, or
/// null; a linear search of the sorted trait array under the critical
/// section.
pub fn tile_get_value_q(e: &mut Engine, this: Ptr<Tile>, trait_id: i32) -> Ptr {
    let mut result = Ptr::NULL;
    let mut index = 0;
    enter_critical_section(e, TILE_CRITICAL_SECTION);
    let traits = this.at(Tile::xTraits);
    let id_at = |e: &Engine, index: u32| {
        let buffer = e.get(traits, BSSimpleArray::pBuffer);
        let value = e.mem.u32(buffer + index * 4);
        e.mem.i32(value)
    };
    while index < e.get(traits, BSSimpleArray::iSize) && id_at(e, index) < trait_id {
        index += 1;
    }
    if index < e.get(traits, BSSimpleArray::iSize) && id_at(e, index) == trait_id {
        let buffer = e.get(traits, BSSimpleArray::pBuffer);
        result = Ptr::new(e.mem.u32(buffer + index * 4));
    }
    leave_critical_section(e, TILE_CRITICAL_SECTION);
    result
}

/// The search keys of `GetValue` and `GetOrCreateValue` are function-local
/// `static Tile::Value`s: on the first call the key is set to the unset
/// state (`eNone`, null tile, 0.0, no string, no actions) and its
/// destructor is registered with `atexit`.
fn init_search_key(e: &mut Engine, key: u32, guard: u32, destroy: u32) {
    let guard_word: u32 = e.global(guard);
    if guard_word & 1 == 0 {
        e.set_global(guard, guard_word | 1);
        e.set_global(key, 0x8000_0000u32);
        e.set_global(key + 4, 0u32);
        e.set_global(key + 8, 0.0f32);
        e.set_global(key + 0xc, 0u32);
        e.set_global(key + 0x10, 0u32);
        e.call(ATEXIT, &args![destroy]);
    }
}

/// `SortedFind` of the key at `key` in the tile's trait array: the index or
/// -1. The array function takes the key by address of a pointer to it.
fn find_value_index(e: &mut Engine, this: Ptr<Tile>, key: u32) -> i32 {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), key);
        e.call(
            VALUES_SORTED_FIND,
            &args![this.at(Tile::xTraits), slot, COMPARE_VALUES],
        )
        .i32()
    })
}

// Translated from 00a00f30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetValue` (Xbox PDB): the trait value with id `trait_id`, or
/// null, by a sorted search of the trait array with the shared search key,
/// under the critical section.
pub fn tile_get_value(e: &mut Engine, this: Ptr<Tile>, trait_id: i32) -> Ptr {
    let mut result = Ptr::NULL;
    enter_critical_section(e, TILE_CRITICAL_SECTION);
    init_search_key(e, GET_VALUE_KEY, GET_VALUE_KEY_GUARD, GET_VALUE_KEY_DESTROY);
    e.set_global(GET_VALUE_KEY, trait_id);
    let index = find_value_index(e, this, GET_VALUE_KEY);
    if index != -1 {
        let buffer = e.get(this.at(Tile::xTraits), BSSimpleArray::pBuffer);
        result = Ptr::new(e.mem.u32(buffer + (index as u32) * 4));
    }
    leave_critical_section(e, TILE_CRITICAL_SECTION);
    result
}

// Translated from 00a01000 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetOrCreateValue` (Xbox PDB): the trait value with id
/// `trait_id`; a new one (0x14 bytes, unset, owned by this tile) is
/// allocated and inserted into the sorted array when there is none.
pub fn tile_get_or_create_value(e: &mut Engine, this: Ptr<Tile>, trait_id: i32) -> Ptr {
    enter_critical_section(e, TILE_CRITICAL_SECTION);
    init_search_key(
        e,
        GET_OR_CREATE_KEY,
        GET_OR_CREATE_KEY_GUARD,
        GET_OR_CREATE_KEY_DESTROY,
    );
    e.set_global(GET_OR_CREATE_KEY, trait_id);
    let index = find_value_index(e, this, GET_OR_CREATE_KEY);
    let result = if index != -1 {
        let buffer = e.get(this.at(Tile::xTraits), BSSimpleArray::pBuffer);
        Ptr::new(e.mem.u32(buffer + (index as u32) * 4))
    } else {
        let value: Ptr<TileValue> = allocate(e, 0x14).cast();
        e.set(value, TileValue::eIndex, TRAIT_NONE);
        e.set(value, TileValue::pParent, Ptr::NULL);
        e.set(value, TileValue::fValue, 0.0);
        e.set(value, TileValue::strValue, Ptr::NULL);
        e.set(value, TileValue::pActionListA, Ptr::NULL);
        e.set(value, TileValue::eIndex, trait_id);
        e.set(value, TileValue::pParent, this);
        // The insert takes the address of a variable holding the value.
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), value.addr());
            e.call(
                VALUES_SORTED_INSERT,
                &args![this.at(Tile::xTraits), slot, COMPARE_VALUES],
            );
            Ptr::new(e.mem.u32(slot.addr()))
        })
    };
    leave_critical_section(e, TILE_CRITICAL_SECTION);
    result
}

// Translated from 00a01160 (decompiled, FalloutNV.exe 1.4.0.525)
/// The comparison `GetValue`, `GetOrCreateValue` and the sorted array
/// functions use: both arguments point to a pointer to a `Tile::Value`;
/// -1, 0 or 1 by the signed trait ids.
pub fn fn_00a01160(e: &mut Engine, first: Ptr, second: Ptr) -> i32 {
    let first_id = e.mem.i32(e.mem.u32(first.addr()));
    let second_id = e.mem.i32(e.mem.u32(second.addr()));
    if first_id < second_id {
        -1
    } else if first_id > second_id {
        1
    } else {
        0
    }
}

// Translated from 00a011b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The float of trait `trait_id` (`Tile::Value::fValue`), 0.0 when the tile
/// has no such trait.
pub fn fn_00a011b0(e: &mut Engine, this: Ptr<Tile>, trait_id: i32) -> f32 {
    let value: Ptr<TileValue> = tile_get_value(e, this, trait_id).cast();
    if value.is_null() {
        0.0
    } else {
        e.get(value, TileValue::fValue)
    }
}

// Translated from 00a011f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetString` (Xbox PDB): the string of trait `trait_id`
/// (`Tile::Value::strValue`), null when the tile has no such trait.
pub fn tile_get_string(e: &mut Engine, this: Ptr<Tile>, trait_id: i32) -> Ptr {
    let value: Ptr<TileValue> = tile_get_value(e, this, trait_id).cast();
    if value.is_null() {
        Ptr::NULL
    } else {
        e.get(value, TileValue::strValue)
    }
}

// Translated from 00a01230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::IsTrue` (Xbox PDB): the trait exists and its float is not 0.0.
pub fn tile_is_true(e: &mut Engine, this: Ptr<Tile>, trait_id: i32) -> bool {
    let value: Ptr<TileValue> = tile_get_value(e, this, trait_id).cast();
    if value.is_null() {
        return false;
    }
    let float = e.get(value, TileValue::fValue);
    differs_from_zero(e, float)
}

// Translated from 00a01290 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::Poke` (Xbox PDB): sets trait `trait_id` to `value` and at once
/// back to 0.0, so that reactions to the change fire (both with the flag
/// set).
pub fn tile_poke(e: &mut Engine, this: Ptr<Tile>, trait_id: i32, value: f32) {
    fn_00a012d0(e, this, trait_id, value, true);
    fn_00a012d0(e, this, trait_id, 0.0, true);
}

// Translated from 00a012d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the float of trait `trait_id` (creating the trait when needed)
/// through `Tile::Value::SetFloat(value, flag)`, under `Tile::Lock`.
pub fn fn_00a012d0(e: &mut Engine, this: Ptr<Tile>, trait_id: i32, value: f32, flag: bool) {
    let trait_value = tile_get_or_create_value(e, this, trait_id);
    if !trait_value.is_null() {
        lock(e);
        e.call(VALUE_SET_FLOAT, &args![trait_value, value, flag as u32]);
        unlock(e);
    }
}

// Translated from 00a01350 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the string of trait `trait_id` (creating the trait when needed)
/// through `Tile::Value::SetString(string, flag)`, under `Tile::Lock`.
pub fn fn_00a01350(e: &mut Engine, this: Ptr<Tile>, trait_id: i32, string: Ptr, flag: bool) {
    let trait_value = tile_get_or_create_value(e, this, trait_id);
    if !trait_value.is_null() {
        lock(e);
        e.call(VALUE_SET_STRING, &args![trait_value, string, flag as u32]);
        unlock(e);
    }
}

/// The sum of trait `trait_id` over the tile and those of its ancestors
/// whose locus trait is not 0.0 (`fn_00a013d0` for `x`, `fn_00a01440` for
/// `y`): each term is added in turn to a `float` running total.
fn sum_over_ancestors(e: &mut Engine, this: Ptr<Tile>, trait_id: i32) -> f32 {
    let mut sum = fn_00a011b0(e, this, trait_id);
    let mut ancestor = e.get(this, Tile::pParent);
    while !ancestor.is_null() {
        let locus = fn_00a011b0(e, ancestor, TRAIT_LOCUS);
        if differs_from_zero(e, locus) {
            let term = fn_00a011b0(e, ancestor, trait_id);
            sum = (term as f64 + sum as f64) as f32;
        }
        ancestor = e.get(ancestor, Tile::pParent);
    }
    sum
}

// Translated from 00a013d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The tile's x position in the root's frame: its own `x` plus the `x` of
/// every ancestor whose locus trait is not 0.0 (see [`sum_over_ancestors`]).
pub fn fn_00a013d0(e: &mut Engine, this: Ptr<Tile>) -> f32 {
    sum_over_ancestors(e, this, TRAIT_X)
}

// Translated from 00a01440 (decompiled, FalloutNV.exe 1.4.0.525)
/// The tile's y position in the root's frame (as [`fn_00a013d0`], with the
/// `y` trait).
pub fn fn_00a01440(e: &mut Engine, this: Ptr<Tile>) -> f32 {
    sum_over_ancestors(e, this, TRAIT_Y)
}

// Translated from 00a014b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The tile's depth: its own depth trait plus the depth of every ancestor
/// whose locus trait is not 0.0 or whose parent is the menus root.
pub fn fn_00a014b0(e: &mut Engine, this: Ptr<Tile>) -> f32 {
    let mut sum = fn_00a011b0(e, this, TRAIT_DEPTH);
    let mut ancestor = e.get(this, Tile::pParent);
    while !ancestor.is_null() {
        let locus = fn_00a011b0(e, ancestor, TRAIT_LOCUS);
        let counts = if differs_from_zero(e, locus) {
            true
        } else {
            let parent = e.get(ancestor, Tile::pParent);
            let root: Ptr<Tile> = e.call(INTERFACE_GET_MENUS_ROOT, &args![]).ptr();
            parent == root
        };
        if counts {
            let term = fn_00a011b0(e, ancestor, TRAIT_DEPTH);
            sum = (term as f64 + sum as f64) as f32;
        }
        ancestor = e.get(ancestor, Tile::pParent);
    }
    sum
}

// Translated from 00a01530 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetMaximumDepth` (Xbox PDB): `Menu::GetMaxDepth()`.
pub fn tile_get_maximum_depth(e: &mut Engine) -> f32 {
    e.call(MENU_GET_MAX_DEPTH, &args![]).f32()
}

// Translated from 00a01540 (decompiled, FalloutNV.exe 1.4.0.525)
/// The greatest depth in the subtree of `tile` (`cdecl`): its own depth
/// trait, and for each child the child's subtree depth (plus this tile's
/// depth when its locus trait is not 0.0), whichever is larger.
pub fn fn_00a01540(e: &mut Engine, tile: Ptr<Tile>) -> f32 {
    let mut deepest = fn_00a011b0(e, tile, TRAIT_DEPTH);
    let mut node = e.get(tile.at(Tile::xChildren), NiTPointerList::m_pkHead);
    while node != 0 {
        let child: Ptr<Tile> = Ptr::new(e.mem.u32(node + 8));
        node = e.mem.u32(node);
        let locus = fn_00a011b0(e, tile, TRAIT_LOCUS);
        let candidate = if differs_from_zero(e, locus) {
            let own_depth = fn_00a011b0(e, tile, TRAIT_DEPTH);
            let below = fn_00a01540(e, child);
            (below as f64 + own_depth as f64) as f32
        } else {
            fn_00a01540(e, child)
        };
        // `max`, but a candidate that is not a number replaces the maximum.
        deepest = if candidate < deepest {
            deepest
        } else {
            candidate
        };
    }
    deepest
}

// Translated from 00a01630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetTileFromNode` (Xbox PDB): the tile a scene-graph node belongs
/// to: the pointer stored in the node's extra data (key `011f34cc`, tile at
/// +0xC of the extra data), or else in its parent node's; null when there
/// is none. Under `Tile::Lock`; the extra data is held by a counted
/// reference while it is read.
pub fn tile_get_tile_from_node(e: &mut Engine, node: Ptr) -> Ptr<Tile> {
    lock(e);
    let mut result = 0;
    if !node.is_null() {
        let key = e.global::<u32>(TILE_EXTRA_DATA_KEY);
        let extra = e.call(NI_OBJECT_GET_EXTRA_DATA, &args![node, key]).u32();
        if extra != 0 {
            e.call(INTERLOCKED_INCREMENT, &args![extra + 4]);
        }
        e.with_stack(4, |e, held| {
            e.mem.set_u32(held.addr(), extra);
            let parent_node = e.mem.u32(node.addr() + 0x18);
            if extra == 0 && parent_node != 0 {
                let key = e.global::<u32>(TILE_EXTRA_DATA_KEY);
                let found = e
                    .call(NI_OBJECT_GET_EXTRA_DATA, &args![parent_node, key])
                    .u32();
                e.call(NI_POINTER_ASSIGN, &args![held, found]);
            }
            let held_now = e.mem.u32(held.addr());
            if held_now != 0 {
                result = e.mem.u32(held_now + 0xc);
            }
            release_reference(e, held_now);
        });
    }
    unlock(e);
    Ptr::new(result)
}

// Translated from 00a01750 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::SeverExtraData` (Xbox PDB): clears the tile pointer in the
/// model's extra data (so the model no longer finds its tile), when the
/// model has any extra data (its 16-bit count at +0x14 is not 0). Under
/// `Tile::Lock`; the extra data is held by a counted reference meanwhile.
pub fn tile_sever_extra_data(e: &mut Engine, this: Ptr<Tile>) {
    lock(e);
    let model = e.get(this, Tile::spModel);
    if !model.is_null() && e.mem.u16(model.addr() + 0x14) != 0 {
        let key = e.global::<u32>(TILE_EXTRA_DATA_KEY);
        let extra = e.call(NI_OBJECT_GET_EXTRA_DATA, &args![model, key]).u32();
        if extra != 0 {
            e.call(INTERLOCKED_INCREMENT, &args![extra + 4]);
            e.mem.set_u32(extra + 0xc, 0);
            release_reference(e, extra);
        }
    }
    unlock(e);
}

// Translated from 00a01860 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::TextToTrait` (Xbox PDB): the text table value of `name`, or
/// `0x80000000` (`eNone`) when it has none. Under `Tile::Lock`.
pub fn tile_text_to_trait(e: &mut Engine, name: Ptr) -> i32 {
    lock(e);
    let id = e.with_stack(4, |e, out| {
        e.mem.set_i32(out.addr(), TRAIT_NONE);
        e.call(TEXT_TABLE_FIND, &args![TEXT_TABLE, name, out]);
        e.mem.i32(out.addr())
    });
    unlock(e);
    id
}

// Translated from 00a018d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Splits `name` of the form `trait(argument)` into the part outside the
/// parentheses (looked up with [`tile_text_to_trait`], the result) and the
/// part inside, which is copied to `argument` (a buffer the caller
/// provides, 0x100 bytes at most). `0x80000000` when `name` or `argument`
/// is missing or `name` is empty. The scan stops at the first `)`, at a NUL
/// or after 254 characters.
pub fn fn_00a018d0(e: &mut Engine, name: Ptr, argument: Ptr) -> i32 {
    if name.is_null() || e.mem.i8(name.addr()) == 0 || argument.is_null() {
        return TRAIT_NONE;
    }
    e.with_stack(NAME_BUFFER_SIZE, |e, outside| {
        let mut inside_at: i32 = -1;
        e.mem.set_u8(argument.addr(), 0);
        e.mem.set_u8(outside.addr(), 0);
        let mut index: i32 = 0;
        while index < 0xff && e.mem.i8(name.addr() + index as u32) != 0 {
            e.mem.set_u8(argument.addr() + index as u32 + 1, 0);
            e.mem.set_u8(outside.addr() + index as u32 + 1, 0);
            let c = e.mem.u8(name.addr() + index as u32);
            if c == b'(' {
                inside_at = 0;
            } else if c == b')' {
                index = 0x100;
            } else if inside_at < 0 {
                e.mem.set_u8(outside.addr() + index as u32, c);
            } else {
                e.mem.set_u8(argument.addr() + inside_at as u32, c);
                inside_at += 1;
            }
            index += 1;
        }
        tile_text_to_trait(e, outside)
    })
}

// Translated from 00a01a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The number at the end of a name: 0 when the name has no `_`, -1 when
/// the last `_` is the last character, else `atol` of what follows it.
pub fn fn_00a01a20(e: &mut Engine, name: Ptr) -> i32 {
    let underscore: Ptr = e.call(STRRCHR, &args![name, 0x5fu32]).ptr();
    if underscore.is_null() {
        0
    } else if e.mem.i8(underscore.addr() + 1) == 0 {
        -1
    } else {
        e.call(ATOL, &args![underscore.byte_add(1)]).i32()
    }
}

// Translated from 00a01a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The name (key) of the text table entry whose value is `value`: scans the
/// table's buckets for the first non-empty one and steps through the
/// entries from there; null when no entry has the value.
pub fn fn_00a01a70(e: &mut Engine, value: i32) -> Ptr {
    let bucket_count: u32 = e.global(TEXT_TABLE_BUCKET_COUNT);
    let buckets: u32 = e.global(TEXT_TABLE_BUCKETS);
    let mut first = 0;
    for index in 0..bucket_count {
        let bucket = e.mem.u32(buckets + index * 4);
        if bucket != 0 {
            first = bucket;
            break;
        }
    }
    e.with_stack(12, |e, slots| {
        let (node, key, entry_value) = (slots.addr(), slots.addr() + 4, slots.addr() + 8);
        e.mem.set_u32(node, first);
        while e.mem.u32(node) != 0 {
            e.call(TEXT_TABLE_NEXT, &args![TEXT_TABLE, node, key, entry_value]);
            if e.mem.i32(entry_value) == value {
                return Ptr::new(e.mem.u32(key));
            }
        }
        Ptr::NULL
    })
}

/// What the XML reader does with one template element, `AddPair` on the
/// storage's current template (else the main one).
fn add_pair(
    e: &mut Engine,
    storage: Ptr<BuildStorage>,
    command: i32,
    text: u32,
    line: i32,
    flag: u32,
) {
    let template = storage_template(e, storage);
    e.call(
        TEMPLATE_ADD_PAIR,
        &args![template, command, text, line, flag],
    );
}

/// Prints an XML error (`"MENUS: (XML ERROR) %s -- in file '%s' on line
/// %i."`) and raises the reader's error flag.
fn xml_error(e: &mut Engine, path: Ptr, line: i32, message: u32) {
    e.call(PRINT_ERROR, &args![XML_ERROR_FORMAT, message, path, line]);
    e.set_global(XML_ERROR_FLAG, 1u8);
}

/// Appends `c` to the text buffer at `buffer` (index `length`), keeps it
/// terminated and complains once the length passes the buffer limit.
fn push_text_char(e: &mut Engine, buffer: u32, length: &mut u32, c: u8) {
    e.mem.set_u8(buffer + *length, c);
    *length += 1;
    e.mem.set_u8(buffer + *length, 0);
    if *length as i32 > READ_BUFFER_LIMIT {
        e.call(PRINT_ERROR, &args![MSG_BUFFER_TOO_SMALL]);
    }
}

/// The scan of [`tile_parse_file`]: a state machine over the characters of
/// the XML text at `data` (`length` bytes), which hands what it finds to
/// `add_pair`. `tag` is the tag-name buffer, `text` the attribute / text
/// buffer, `converted` the buffer a game-setting constant is expanded into.
/// False after an error.
///
/// States (the `Tile::enumXMLname` names): 0 outside a tag, 1 in a tag name,
/// 2 after the `/` of a close tag, 3 in tag data, 4 in a comment, 5 in an
/// attribute name, 6 in an attribute value.
fn parse_xml(
    e: &mut Engine,
    path: Ptr,
    (data, length): (u32, u32),
    storage: Ptr<BuildStorage>,
    (tag, text, converted): (u32, u32, u32),
) -> bool {
    let mut state = 0;
    let mut trait_id = 0;
    let mut index: u32 = 0;
    let mut used: u32 = 0;
    let mut line: i32 = 1;
    // A word ended at white space; the next word is a new one.
    let mut after_word = false;
    let mut in_quote = false;
    let mut closing = false;
    e.mem.set_u8(text, 0);
    while index < length {
        let c = e.mem.u8(data + index);
        let next = |e: &Engine, offset: u32| e.mem.u8(data + index.wrapping_add(offset));
        let ends_tag = |e: &Engine| {
            c == b'>' || (index.wrapping_add(1) < length && c == b'/' && next(e, 1) == b'>')
        };
        let starts_comment = |e: &Engine| {
            index.wrapping_add(3) < length
                && next(e, 1) == b'!'
                && next(e, 2) == b'-'
                && next(e, 3) == b'-'
        };
        match state {
            0 => {
                if c == b'<' {
                    if starts_comment(e) {
                        after_word = false;
                        state = 4;
                    } else {
                        state = 1;
                        e.mem.set_u8(tag, 0);
                        closing = false;
                        used = 0;
                    }
                }
            }
            1 => {
                let tag_empty = e.mem.i8(tag) == 0;
                if tag_empty && c == b'/' {
                    closing = true;
                } else if ends_tag(e) {
                    if !tag_empty {
                        add_pair(e, storage, closing as i32, tag, line, 0);
                    } else if !after_word {
                        xml_error(e, path, line, MSG_EMPTY_TAG_NAME);
                        return false;
                    }
                    used = 0;
                    state = if c == b'/' { 2 } else { 3 };
                    after_word = false;
                    line += 1;
                } else if c <= 0x20 {
                    if !tag_empty {
                        add_pair(e, storage, closing as i32, tag, line, 0);
                        used = 0;
                        after_word = true;
                        line += 1;
                    }
                } else if after_word {
                    index = index.wrapping_sub(1);
                    state = 5;
                    after_word = false;
                } else {
                    // No length check here, unlike the text buffers.
                    e.mem.set_u8(tag + used, c);
                    used += 1;
                    e.mem.set_u8(tag + used, 0);
                }
            }
            4 => {
                if index.wrapping_add(2) < length
                    && c == b'-'
                    && next(e, 1) == b'-'
                    && next(e, 2) == b'>'
                {
                    index += 2;
                    state = 3;
                    after_word = false;
                }
            }
            5 => {
                if ends_tag(e) {
                    xml_error(e, path, line, MSG_ATTRIBUTE_NO_VALUE);
                    return false;
                }
                if c == b'=' {
                    if e.mem.i8(text) == 0 {
                        xml_error(e, path, line, MSG_MISSING_ATTRIBUTE_NAME);
                        return false;
                    }
                    trait_id = tile_text_to_trait(e, Ptr::new(text));
                    e.mem.set_u8(text, 0);
                    used = 0;
                    state = 6;
                    after_word = false;
                } else if c <= 0x20 {
                    if e.mem.i8(text) != 0 {
                        trait_id = tile_text_to_trait(e, Ptr::new(text));
                        e.mem.set_u8(text, 0);
                        used = 0;
                        after_word = true;
                    }
                } else {
                    if after_word {
                        xml_error(e, path, line, MSG_UNEXPECTED_WORD);
                        return false;
                    }
                    push_text_char(e, text, &mut used, c);
                }
            }
            6 => {
                if c == b'"' {
                    in_quote = !in_quote;
                } else if in_quote {
                    push_text_char(e, text, &mut used, c);
                } else if ends_tag(e) {
                    if e.mem.i8(text) != 0 {
                        add_pair(e, storage, trait_id, text, line, 0);
                    } else if !after_word {
                        xml_error(e, path, line, MSG_MISSING_ATTRIBUTE_VALUE);
                        return false;
                    }
                    e.mem.set_u8(text, 0);
                    used = 0;
                    state = if c == b'/' { 2 } else { 3 };
                    after_word = false;
                } else if c <= 0x20 {
                    if e.mem.i8(text) != 0 {
                        add_pair(e, storage, trait_id, text, line, 0);
                        e.mem.set_u8(text, 0);
                        used = 0;
                        after_word = true;
                    }
                } else if after_word {
                    index = index.wrapping_sub(1);
                    state = 5;
                    after_word = false;
                } else {
                    push_text_char(e, text, &mut used, c);
                }
            }
            2 => {
                if c != b'>' {
                    xml_error(e, path, line, MSG_CLOSE_TAG_MARKER);
                    return false;
                }
                add_pair(e, storage, 1, tag, line, 0);
                state = 3;
                e.mem.set_u8(tag, 0);
            }
            _ => {
                // State 3: data between tags.
                if c == b'<' {
                    if starts_comment(e) {
                        state = 4;
                    } else {
                        if e.mem.i8(text) != 0 {
                            // Trim the white space at the end of the text.
                            let mut last = e.call(STRLEN, &args![text]).u32().wrapping_sub(1);
                            while e.mem.u8(text + last) <= 0x20 {
                                last = last.wrapping_sub(1);
                            }
                            e.mem.set_u8(text + last + 1, 0);
                            if e.mem.i8(text) == b'&' as i8 {
                                if e.mem.i8(text + 1) == b'-' as i8 {
                                    // "&-name;": a game-setting constant.
                                    let known = e
                                        .call(
                                            INTERFACE_TEST_CONSTANT_FOR_GAME_SETTINGS,
                                            &args![text, converted],
                                        )
                                        .bool();
                                    if known {
                                        add_pair(e, storage, TAG_VALUE, converted, line, 0);
                                    }
                                } else {
                                    add_pair(e, storage, TAG_VALUE, text, line, 0);
                                }
                            } else if e.mem.i8(text) != 0 {
                                add_pair(e, storage, TAG_VALUE, text, line, 1);
                            }
                        }
                        e.mem.set_u8(text, 0);
                        used = 0;
                        closing = false;
                        state = 1;
                        e.mem.set_u8(tag, 0);
                    }
                    after_word = false;
                } else {
                    if index.wrapping_add(1) < length && c == b'/' && next(e, 1) == b'>' {
                        xml_error(e, path, line, MSG_UNBALANCED_CLOSE_PAIR);
                        return false;
                    }
                    if c == b'>' {
                        xml_error(e, path, line, MSG_UNBALANCED_END_OF_TAG);
                        return false;
                    }
                    if c > 0x20 || after_word {
                        push_text_char(e, text, &mut used, c);
                        after_word = true;
                    }
                }
            }
        }
        index = index.wrapping_add(1);
    }
    true
}

// Translated from 00a01b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::ReadFile` (Xbox PDB): reads the menu file `path` into a tile tree
/// under `this`: parses it, builds and names the tiles, connects the
/// traits, registers the sub-templates with the tile's menu, frees the
/// parse storage and updates the new tree (under the interface lock).
/// Returns the first new tile, or null after an error (the error flag
/// `011f32c8` is set by the parser). The thread's current-line word is 13
/// for the duration.
pub fn tile_read_file(e: &mut Engine, this: Ptr<Tile>, path: Ptr) -> Ptr<Tile> {
    let line_word = e.tls() + TLS_CURRENT_LINE;
    let saved_line = e.mem.u32(line_word);
    e.mem.set_u32(line_word, 0xd);
    e.set_global(XML_ERROR_FLAG, 0u8);

    let storage = tile_parse_file(e, path, Ptr::NULL, 0);
    let failed = |e: &mut Engine| {
        let failed = e.global::<u8>(XML_ERROR_FLAG) != 0;
        if failed {
            e.mem.set_u32(line_word, saved_line);
        }
        failed
    };
    if failed(e) {
        return Ptr::NULL;
    }
    let template = storage_template(e, storage);
    let tile = tile_build_and_name_tree(e, this, template);
    if failed(e) {
        return Ptr::NULL;
    }
    let template = storage_template(e, storage);
    tile_connect_traits_to_tree(e, tile, template);
    if failed(e) {
        return Ptr::NULL;
    }
    let menu = tile_get_menu(e, tile);
    if !menu.is_null() {
        let mut node = storage.addr() + 4;
        while node != 0 && e.mem.u32(node) != 0 {
            let sub_template = e.mem.u32(node);
            e.call(MENU_ADD_TEMPLATE, &args![menu, sub_template]);
            // `bDeleteTemplates` of the menu is set; the storage's own flag
            // (a local that is never set in this function) is cleared.
            e.mem.set_u8(menu.addr() + 0x1d, 1);
            e.set(storage, BuildStorage::bDeleteTemplates, false);
            if failed(e) {
                return Ptr::NULL;
            }
            node = e.mem.u32(node + 4);
        }
    }
    // The parse storage is deleted (the "keep" flag is never set).
    if !storage.is_null() {
        e.call(BUILD_STORAGE_DESTROY, &args![storage]);
        deallocate(e, storage.addr());
    }
    let lock_owner: u32 = e.global(INTERFACE_LOCK_OWNER);
    enter_critical_section(e, lock_owner + 0x80);
    if !tile.is_null() {
        e.call(TILE_UPDATE_CHILDREN, &args![tile, 0u32]);
    }
    let lock_owner: u32 = e.global(INTERFACE_LOCK_OWNER);
    leave_critical_section(e, lock_owner + 0x80);
    e.set_global(XML_ERROR_FLAG, 0u8);
    e.mem.set_u32(line_word, saved_line);
    tile
}

// Translated from 00a01e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::ParseFile` (Xbox PDB, `cdecl`): parses menu XML into a new
/// `BuildStorage` and returns it. With no `buffer` the text is the file
/// `path` read by `Menu::GetXML`, after its `<include>` tags are expanded
/// ([`tile_insert_included_files`]); with a `buffer` it is `length` bytes of
/// text given by the caller. The scanner is [`parse_xml`]. Returns null after
/// an XML error (the error flag `011f32c8` is set and the storage and the
/// file text are not freed); the file text is freed after a complete scan.
pub fn tile_parse_file(e: &mut Engine, path: Ptr, buffer: Ptr, length: u32) -> Ptr<BuildStorage> {
    let block = allocate(e, 0x14);
    let storage: Ptr<BuildStorage> = if block.is_null() {
        Ptr::NULL
    } else {
        e.call(BUILD_STORAGE_CONSTRUCT, &args![block]).ptr()
    };
    let mut xml: Ptr<XmlStorage> = Ptr::NULL;
    let (data, size) = if buffer.is_null() {
        xml = e.call(MENU_GET_XML, &args![path, 0u32]).ptr();
        if xml.is_null() {
            (0, 0)
        } else {
            tile_insert_included_files(e, xml);
            (
                e.get(xml, XmlStorage::pXMLData).addr(),
                e.get(xml, XmlStorage::iFileSize),
            )
        }
    } else {
        (buffer.addr(), length)
    };
    let scratch = e.mem.alloc(TAG_BUFFER_SIZE + 2 * TEXT_BUFFER_SIZE);
    let (tag, text, converted) = (
        scratch,
        scratch + TAG_BUFFER_SIZE,
        scratch + TAG_BUFFER_SIZE + TEXT_BUFFER_SIZE,
    );
    let complete = parse_xml(e, path, (data, size), storage, (tag, text, converted));
    e.mem.free(scratch);
    if !complete {
        return Ptr::NULL;
    }
    if !xml.is_null() {
        e.call(XML_STORAGE_SCALAR_DELETING_DESTRUCTOR, &args![xml, 1u32]);
    }
    storage
}

// Translated from 00a02d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::InsertIncludedFiles` (Xbox PDB, `cdecl`): replaces every
/// `<include src="name"/>` tag in the text of `xml` by the text of the
/// prefab file `Data\Menus\Prefabs\name` (read with `Menu::GetXML`, which
/// is freed afterwards), then repeats on the result in case the prefabs
/// have includes of their own. True when there was at least one include.
///
/// The pieces are collected in a `BSStringT`; the original text is freed
/// and replaced by a copy of the result. `strtok_s` writes into the old
/// text, as in the game.
pub fn tile_insert_included_files(e: &mut Engine, xml: Ptr<XmlStorage>) -> bool {
    let old_text = e.get(xml, XmlStorage::pXMLData).addr();
    let mut include = e.call(STRSTR, &args![old_text, INCLUDE_TAG]).u32();
    if include == 0 {
        return false;
    }
    e.with_stack(8, |e, result| {
        let size = e.get(xml, XmlStorage::iFileSize) + 1;
        let buffer = allocate(e, size).addr();
        e.call(MEMSET, &args![buffer, 0u32, size]);
        let head = include - old_text;
        e.call(STRNCPY_S, &args![buffer, head + 1, old_text, head]);
        e.call(STRING_SET, &args![result, buffer, 0x4000u32]);
        let mut last = false;
        while !last {
            e.with_stack(8, |e, path_string| {
                let tag_length = e.call(STRCHR, &args![include, 0x3eu32]).u32() - include + 1;
                e.call(MEMSET, &args![buffer, 0u32, size]);
                let name = e.with_stack(4, |e, context| {
                    e.call(STRTOK_S, &args![include, QUOTE_DELIMITER, context]);
                    e.call(STRTOK_S, &args![0u32, QUOTE_DELIMITER, context])
                        .u32()
                });
                e.call(STRING_FORMAT, &args![path_string, PREFAB_PATH_FORMAT, name]);
                let path = e.mem.u32(path_string.addr());
                let prefab: Ptr<XmlStorage> = e.call(MENU_GET_XML, &args![path, 0u32]).ptr();
                let prefab_text = e.get(prefab, XmlStorage::pXMLData);
                e.call(STRING_APPEND, &args![result, prefab_text]);
                if !prefab.is_null() {
                    e.call(XML_STORAGE_DESTROY, &args![prefab]);
                    deallocate(e, prefab.addr());
                }
                let after_tag = include + tag_length;
                let next = e.call(STRSTR, &args![after_tag, INCLUDE_TAG]).u32();
                e.call(MEMSET, &args![buffer, 0u32, size]);
                if next != 0 {
                    let between = next - after_tag;
                    e.call(STRNCPY_S, &args![buffer, between + 1, after_tag, between]);
                    include = next;
                } else {
                    e.call(STRCPY_S, &args![buffer, size, after_tag]);
                    last = true;
                }
                e.call(STRING_APPEND, &args![result, buffer]);
                e.call(STRING_SET, &args![path_string, 0u32, 0u32]);
            });
        }
        deallocate(e, old_text);
        // The result's length, from its length field or, when that is
        // 0xffff (unknown), by counting.
        let result_length = |e: &mut Engine| {
            if e.mem.u16(result.addr() + 4) == 0xffff {
                let text = e.mem.u32(result.addr());
                e.call(STRLEN, &args![text]).u32()
            } else {
                e.mem.u16(result.addr() + 4) as u32
            }
        };
        let length = result_length(e);
        let new_text = allocate(e, length + 1);
        e.set(xml, XmlStorage::pXMLData, new_text);
        let length = result_length(e);
        let text = e.mem.u32(result.addr());
        e.call(STRCPY_S, &args![new_text, length + 1, text]);
        let length = result_length(e);
        e.set(xml, XmlStorage::iFileSize, length);
        deallocate(e, buffer);
        tile_insert_included_files(e, xml);
        e.call(STRING_SET, &args![result, 0u32, 0u32]);
    });
    true
}

// Translated from 00a031b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::BuildAndNameTree` (Xbox PDB): walks the items of `template`,
/// creating a tile for each tile-start item (`BuildTile` with the item's
/// type number), initializing it as a child of the current parent with the
/// item's name (virtual slot +4, `Init`) and remembering it in the item;
/// a tile-end item moves back to the parent. A hot-rect tile (type 906) is
/// given its fixed look: id -1, target 1.0, brightness -1.0, red, green and
/// blue 255.0, alpha 0.0, texture `solid.dds` and the default atlas, and
/// queued for a texture update. Returns the first tile built, or null
/// (after printing a message) when a tile cannot be created.
pub fn tile_build_and_name_tree(
    e: &mut Engine,
    this: Ptr<Tile>,
    template: Ptr<TileTemplate>,
) -> Ptr<Tile> {
    let mut first: Ptr<Tile> = Ptr::NULL;
    let mut parent = this;
    let mut node = e.get(template.at(TileTemplate::xList), NiTPointerList::m_pkHead);
    while node != 0 {
        let item: Ptr<TileTemplateItem> = Ptr::new(e.mem.u32(node + 8));
        node = e.mem.u32(node);
        match e.get(item, TileTemplateItem::iCmd) {
            TI_TILE_START => {
                let type_number = e.get(item, TileTemplateItem::fVal);
                let kind = float_to_int(e, type_number);
                let tile: Ptr<Tile> = tile_build_tile(e, kind as u32).cast();
                if tile.is_null() {
                    e.call(PRINT_ERROR, &args![MSG_UNABLE_TO_CREATE_TILE]);
                    return Ptr::NULL;
                }
                let name = e.mem.u32(item.addr() + 8);
                e.vcall(tile.addr(), 4, &args![parent, name, 0u32]);
                let is_hot_rect = type_number as f64 == e.global::<f64>(HOT_RECT_TYPE_VALUE);
                if is_hot_rect {
                    let minus_one: f32 = e.global(MINUS_ONE);
                    let full: f32 = e.global(TWO_FIFTY_FIVE);
                    fn_00a012d0(e, tile, TRAIT_ID, minus_one, true);
                    fn_00a012d0(e, tile, TRAIT_TARGET, 1.0, true);
                    fn_00a012d0(e, tile, TRAIT_BRIGHTNESS, minus_one, true);
                    fn_00a012d0(e, tile, TRAIT_RED, full, true);
                    fn_00a012d0(e, tile, TRAIT_GREEN, full, true);
                    fn_00a012d0(e, tile, TRAIT_BLUE, full, true);
                    fn_00a012d0(e, tile, TRAIT_ALPHA, 0.0, true);
                    fn_00a01350(e, tile, TRAIT_FILENAME, Ptr::new(SOLID_TEXTURE_NAME), true);
                    let atlas: u32 = e.global(DEFAULT_ATLAS_NAME);
                    fn_00a01350(e, tile, TRAIT_TEX_ATLAS, Ptr::new(atlas), true);
                    e.call(TILE_ADD_NEEDS_UPDATE, &args![tile, UPDATE_TEXTURE]);
                }
                if first.is_null() {
                    first = tile;
                }
                e.set(item, TileTemplateItem::u, tile.addr());
                parent = tile;
            }
            TI_TILE_END => {
                parent = e.get(parent, Tile::pParent);
            }
            _ => {}
        }
    }
    first
}

// Translated from 00a033b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::ConnectTraitsToTree` (Xbox PDB): second pass over the items of
/// `template`, with the tiles `BuildAndNameTree` made:
///
/// * tile start: that tile becomes current and is queued for an update,
///   with `FLAG_TILE_LOADING` set;
/// * tile end: clears `FLAG_TILE_LOADING` and calculates every value of the
///   tile except its class, under the critical section, then goes back to
///   the parent;
/// * trait start / end: the current trait (the item's number / none);
/// * simple trait: a float, or a string for string traits or a value with
///   text (the `eName` tag becomes the tile's name; a file name of an image
///   tile also queues a texture update), set on the current tile;
/// * simple action, action link, action start / end (parentheses): an
///   action added to the current trait's value.
///
/// Items out of place print a message. The first parameter is `this` of
/// the call, which is not read.
pub fn tile_connect_traits_to_tree(
    e: &mut Engine,
    _unused_0: Ptr<Tile>,
    template: Ptr<TileTemplate>,
) {
    let mut first: Ptr<Tile> = Ptr::NULL;
    let mut current: Ptr<Tile> = Ptr::NULL;
    let mut trait_id: i32 = 0;
    let mut node = e.get(template.at(TileTemplate::xList), NiTPointerList::m_pkHead);
    while node != 0 {
        let item: Ptr<TileTemplateItem> = Ptr::new(e.mem.u32(node + 8));
        node = e.mem.u32(node);
        let value = e.get(item, TileTemplateItem::fVal);
        let argument = e.get(item, TileTemplateItem::u);
        let text = e.mem.u32(item.addr() + 8);
        match e.get(item, TileTemplateItem::iCmd) {
            TI_TILE_START => {
                current = Ptr::new(argument);
                if first.is_null() {
                    first = current;
                }
                e.call(TILE_ADD_NEEDS_UPDATE, &args![current, FLAG_TILE_LOADING]);
            }
            TI_TILE_END => {
                let flags = e.get(current, Tile::uiFlags);
                if flags & FLAG_TILE_LOADING != 0 {
                    e.set(current, Tile::uiFlags, flags ^ FLAG_TILE_LOADING);
                }
                enter_critical_section(e, TILE_CRITICAL_SECTION);
                let traits = current.at(Tile::xTraits);
                let mut index = 0;
                while index < e.get(traits, BSSimpleArray::iSize) {
                    let buffer = e.get(traits, BSSimpleArray::pBuffer);
                    let trait_value = e.mem.u32(buffer + index * 4);
                    if e.mem.i32(trait_value) != TRAIT_CLASS {
                        e.call(VALUE_CALCULATE_VALUE, &args![trait_value, 1u32]);
                    }
                    index += 1;
                }
                leave_critical_section(e, TILE_CRITICAL_SECTION);
                current = e.get(current, Tile::pParent);
            }
            TI_SIMPLE_TRAIT => {
                if current.is_null() {
                    e.call(PRINT_ERROR, &args![MSG_TRAIT_OUTSIDE_TILE]);
                    continue;
                }
                let item_trait = argument as i32;
                let mut is_string = false;
                if value as f64 == e.global::<f64>(NO_TRAIT_VALUE) {
                    // No number was given: a value with text is a string.
                    let length = if e.mem.u16(item.addr() + 0xc) == 0xffff {
                        e.call(STRLEN, &args![text]).u32()
                    } else {
                        e.mem.u16(item.addr() + 0xc) as u32
                    };
                    is_string = length != 0;
                }
                if !is_string && (item_trait == TRAIT_STRING || item_trait == TRAIT_FILENAME) {
                    is_string = true;
                }
                if !is_string {
                    fn_00a012d0(e, current, item_trait, value, true);
                } else if item_trait == TAG_NAME {
                    e.call(
                        STRING_ASSIGN,
                        &args![current.at(Tile::xName), item.at(TileTemplateItem::xStr)],
                    );
                } else if item_trait == TRAIT_FILENAME && tile_type(e, current) == TYPE_IMAGE {
                    fn_00a01350(e, current, item_trait, Ptr::new(text), true);
                    e.call(TILE_ADD_NEEDS_UPDATE, &args![current, UPDATE_TEXTURE]);
                } else {
                    fn_00a01350(e, current, item_trait, Ptr::new(text), true);
                }
            }
            TI_TRAIT_START => {
                trait_id = float_to_int(e, value);
            }
            TI_TRAIT_END => {
                trait_id = 0;
            }
            TI_SIMPLE_ACTION => {
                if trait_id == 0 {
                    e.call(PRINT_ERROR, &args![MSG_ACTION_OUTSIDE_TRAIT]);
                } else {
                    let trait_value = tile_get_or_create_value(e, current, trait_id);
                    e.call(VALUE_ADD_ACTION, &args![trait_value, argument, value]);
                }
            }
            TI_TRAIT_LINK => {
                if trait_id == 0 {
                    e.call(PRINT_ERROR, &args![MSG_ACTION_LINK_OUTSIDE_TRAIT]);
                } else {
                    let link = e.call(TILE_FIND_BY_PATH, &args![current, text]).u32();
                    if link != 0 {
                        let number = float_to_int_rounded(value);
                        let trait_value = tile_get_or_create_value(e, current, trait_id);
                        e.call(
                            VALUE_ADD_ACTION_OV2,
                            &args![trait_value, argument, link, number],
                        );
                    }
                }
            }
            TI_ACTION_START | TI_ACTION_END => {
                let (action, message) = if e.get(item, TileTemplateItem::iCmd) == TI_ACTION_START {
                    (ACTION_LEFT_PAREN, MSG_ACTION_BEGUN_OUTSIDE_TRAIT)
                } else {
                    (ACTION_RIGHT_PAREN, MSG_ACTION_ENDED_OUTSIDE_TRAIT)
                };
                if trait_id == 0 {
                    e.call(PRINT_ERROR, &args![message]);
                } else {
                    let number = float_to_int_rounded(value) as f32;
                    let trait_value = tile_get_or_create_value(e, current, trait_id);
                    e.call(VALUE_ADD_ACTION, &args![trait_value, action, number]);
                }
            }
            _ => {}
        }
    }
}

// Translated from 00a037e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::FinalPostParse` (Xbox PDB): after trait `trait_id` of the tile
/// changed to `value`, queues the updates it needs and returns `this`, or
/// null when the trait needs none:
///
/// * x, y or depth: position update (and, for depth, the menu's depth is
///   raised to the summed depth of the tile and its ancestors with a locus
///   when that is more and the tile is not the menu tile); a tile with a
///   clip window also gets a scissor-window update;
/// * a text tile's wrap, justify, font, page and similar traits: create;
/// * width, height, crop of an image: geometry (and scissor window with a
///   clip window);
/// * clip window of an image or rect: scissor window; clips of an image or
///   text: scissor, and the children's clips are set to the tile's;
/// * visible: visibility; colour traits of an image or text: colour;
/// * file name of an image with a file: texture; of a 3D tile with a file:
///   NIF file; locus: locus; id: the menu is told the new id (virtual slot
///   +4); rotation: position.
///
/// The third stack word of the call is not read.
pub fn tile_final_post_parse(
    e: &mut Engine,
    this: Ptr<Tile>,
    trait_id: i32,
    value: f32,
    _unused_3: u32,
) -> Ptr<Tile> {
    let queue = |e: &mut Engine, flags: u32| {
        e.call(TILE_ADD_NEEDS_UPDATE, &args![this, flags]);
    };
    if matches!(trait_id, TRAIT_X | TRAIT_Y | TRAIT_DEPTH) {
        if trait_id == TRAIT_DEPTH {
            let mut total = value;
            let mut ancestor = this;
            loop {
                let parent = e.get(ancestor, Tile::pParent);
                if parent.is_null() || e.get(parent, Tile::pParent).is_null() {
                    break;
                }
                let locus = fn_00a011b0(e, ancestor, TRAIT_LOCUS);
                if differs_from_zero(e, locus) {
                    let own = fn_00a011b0(e, ancestor, TRAIT_DEPTH);
                    total = (own as f64 + total as f64) as f32;
                }
                ancestor = parent;
            }
            let menu = tile_get_menu(e, this);
            let menu_tile = tile_get_menu_tile(e, this);
            if !menu.is_null() && !menu_tile.is_null() && menu_tile != this {
                let menu_depth = e.mem.i32(menu.addr() + 0x18);
                if total as f64 > menu_depth as f64 {
                    let depth = float_to_int(e, total);
                    e.mem.set_i32(menu.addr() + 0x18, depth);
                }
            }
        }
        let clip_window = fn_00a011b0(e, this, TRAIT_CLIP_WINDOW);
        if differs_from_zero(e, clip_window) {
            queue(e, UPDATE_SCISSOR_WINDOW);
        }
        queue(e, UPDATE_POSITION);
        return this;
    }

    let kind = tile_type(e, this);
    if kind == TYPE_TEXT
        && matches!(
            trait_id,
            TRAIT_WRAP_WIDTH
                | TRAIT_WRAP_LIMIT
                | TRAIT_WRAP_LINES
                | TRAIT_LINE_GAP
                | TRAIT_PAGE_NUM
                | TRAIT_IS_HTML
                | TRAIT_FONT
                | TRAIT_JUSTIFY
        )
    {
        queue(e, UPDATE_CREATE);
        return this;
    }
    if matches!(
        trait_id,
        TRAIT_WIDTH | TRAIT_HEIGHT | TRAIT_CROP_X | TRAIT_CROP_Y
    ) && tile_type(e, this) == TYPE_IMAGE
    {
        let clip_window = fn_00a011b0(e, this, TRAIT_CLIP_WINDOW);
        if differs_from_zero(e, clip_window) {
            queue(e, UPDATE_SCISSOR_WINDOW);
        }
        queue(e, UPDATE_GEOMETRY);
        return this;
    }
    if trait_id == TRAIT_CLIP_WINDOW
        && (tile_type(e, this) == TYPE_IMAGE || tile_type(e, this) == TYPE_RECT)
    {
        queue(e, UPDATE_SCISSOR_WINDOW);
        return this;
    }
    if trait_id == TRAIT_CLIPS
        && (tile_type(e, this) == TYPE_IMAGE || tile_type(e, this) == TYPE_TEXT)
    {
        queue(e, UPDATE_SCISSOR);
        let mut node = e.get(this.at(Tile::xChildren), NiTPointerList::m_pkHead);
        while node != 0 {
            let child: Ptr<Tile> = Ptr::new(e.mem.u32(node + 8));
            node = e.mem.u32(node);
            let clips = fn_00a011b0(e, this, TRAIT_CLIPS);
            fn_00a012d0(e, child, TRAIT_CLIPS, clips, true);
        }
        return this;
    }
    if trait_id == TRAIT_VISIBLE {
        queue(e, UPDATE_VISIBILITY);
        return this;
    }
    if matches!(
        trait_id,
        TRAIT_ALPHA | TRAIT_RED | TRAIT_GREEN | TRAIT_BLUE | TRAIT_SYSTEM_COLOR | TRAIT_BRIGHTNESS
    ) {
        if tile_type(e, this) == TYPE_IMAGE || tile_type(e, this) == TYPE_TEXT {
            queue(e, UPDATE_COLOR);
        }
        return this;
    }
    if trait_id == TRAIT_FILENAME
        && tile_type(e, this) == TYPE_IMAGE
        && !tile_get_string(e, this, TRAIT_FILENAME).is_null()
    {
        queue(e, UPDATE_TEXTURE);
        return this;
    }
    if trait_id == TRAIT_FILENAME
        && tile_type(e, this) == TYPE_3D
        && !tile_get_string(e, this, TRAIT_FILENAME).is_null()
    {
        queue(e, UPDATE_NIF_FILE);
        return this;
    }
    if trait_id == TRAIT_LOCUS {
        queue(e, UPDATE_LOCUS);
        return this;
    }
    if trait_id == TRAIT_ID {
        let menu = tile_get_menu(e, this);
        if !menu.is_null() {
            let id = float_to_int(e, value);
            e.vcall(menu.addr(), 4, &args![id, this]);
        }
    }
    if matches!(
        trait_id,
        TRAIT_ROTATE_ANGLE | TRAIT_ROTATE_AXIS_X | TRAIT_ROTATE_AXIS_Y
    ) {
        queue(e, UPDATE_POSITION);
        return this;
    }
    Ptr::NULL
}

// Translated from 00a03c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetRenderedMenu` (Xbox PDB): `bRenderedMenu` of the tile's menu.
pub fn tile_get_rendered_menu(e: &mut Engine, this: Ptr<Tile>) -> bool {
    let menu = tile_get_menu(e, this);
    e.mem.u8(menu.addr() + 0x1c) != 0
}

// Translated from 00a03c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetMenu` (Xbox PDB): the `Menu` object of the tile's menu tile
/// (the word at +0x3C of a `TileMenu`), or null when the menu tile is not
/// a menu-type tile. Under `Tile::Lock`.
pub fn tile_get_menu(e: &mut Engine, this: Ptr<Tile>) -> Ptr {
    lock(e);
    let menu_tile = tile_get_menu_tile(e, this);
    let menu = if !menu_tile.is_null() && tile_type(e, menu_tile) == TYPE_MENU {
        Ptr::new(e.mem.u32(menu_tile.addr() + 0x3c))
    } else {
        Ptr::NULL
    };
    unlock(e);
    menu
}

// Translated from 00a03d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetMenuTile` (Xbox PDB): the ancestor (or the tile itself) that
/// is a child of the root: walks up while the parent has a parent. Under
/// `Tile::Lock`.
pub fn tile_get_menu_tile(e: &mut Engine, this: Ptr<Tile>) -> Ptr<Tile> {
    lock(e);
    let mut tile = this;
    while !tile.is_null() {
        let parent = e.get(tile, Tile::pParent);
        if parent.is_null() || e.get(parent, Tile::pParent).is_null() {
            break;
        }
        tile = parent;
    }
    unlock(e);
    tile
}

// Translated from 00a03da0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Tile::GetChildByName` (Xbox PDB): the first tile named `name` among the
/// descendants of the tile, depth first (each child is compared before its
/// own children are searched); null when there is none or `name` is null.
/// Under `Tile::Lock`.
pub fn tile_get_child_by_name(e: &mut Engine, this: Ptr<Tile>, name: Ptr) -> Ptr<Tile> {
    lock(e);
    let mut result = Ptr::NULL;
    let mut node = e.get(this.at(Tile::xChildren), NiTPointerList::m_pkHead);
    while node != 0 && !name.is_null() {
        let child: Ptr<Tile> = Ptr::new(e.mem.u32(node + 8));
        node = e.mem.u32(node);
        let child_name = e.mem.u32(child.addr() + 0x20);
        if child_name != 0 && e.call(STRCMP, &args![child_name, name]).i32() == 0 {
            result = child;
            break;
        }
        let found = tile_get_child_by_name(e, child, name);
        if !found.is_null() {
            result = found;
            break;
        }
    }
    unlock(e);
    result
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x009ff340, tile_destructor(Ptr<Tile>)),
        entry!(0x009ff600, tile_init(Ptr<Tile>, Ptr<Tile>, Ptr, u32)),
        entry!(0x009ff690, tile_release(Ptr<Tile>)),
        entry!(0x009ff8a0, fn_009ff8a0(Ptr, i32)),
        entry!(0x009ff970, tile_load_text_table()),
        entry!(0x00a00940, tile_add_user_trait(Ptr, i32) -> i32),
        entry!(0x00a00a30, tile_free_trait_list()),
        entry!(0x00a00b50, tile_set_menu_deleting(Ptr<Tile>)),
        entry!(0x00a00c20, tile_build_tile(u32) -> Ptr),
        entry!(0x00a00e90, tile_get_value_q(Ptr<Tile>, i32) -> Ptr),
        entry!(0x00a00f30, tile_get_value(Ptr<Tile>, i32) -> Ptr),
        entry!(0x00a01000, tile_get_or_create_value(Ptr<Tile>, i32) -> Ptr),
        entry!(0x00a01160, fn_00a01160(Ptr, Ptr) -> i32),
        entry!(0x00a011b0, fn_00a011b0(Ptr<Tile>, i32) -> f32),
        entry!(0x00a011f0, tile_get_string(Ptr<Tile>, i32) -> Ptr),
        entry!(0x00a01230, tile_is_true(Ptr<Tile>, i32) -> bool),
        entry!(0x00a01290, tile_poke(Ptr<Tile>, i32, f32)),
        entry!(0x00a012d0, fn_00a012d0(Ptr<Tile>, i32, f32, bool)),
        entry!(0x00a01350, fn_00a01350(Ptr<Tile>, i32, Ptr, bool)),
        entry!(0x00a013d0, fn_00a013d0(Ptr<Tile>) -> f32),
        entry!(0x00a01440, fn_00a01440(Ptr<Tile>) -> f32),
        entry!(0x00a014b0, fn_00a014b0(Ptr<Tile>) -> f32),
        entry!(0x00a01530, tile_get_maximum_depth() -> f32),
        entry!(0x00a01540, fn_00a01540(Ptr<Tile>) -> f32),
        entry!(0x00a01630, tile_get_tile_from_node(Ptr) -> Ptr<Tile>),
        entry!(0x00a01750, tile_sever_extra_data(Ptr<Tile>)),
        entry!(0x00a01860, tile_text_to_trait(Ptr) -> i32),
        entry!(0x00a018d0, fn_00a018d0(Ptr, Ptr) -> i32),
        entry!(0x00a01a20, fn_00a01a20(Ptr) -> i32),
        entry!(0x00a01a70, fn_00a01a70(i32) -> Ptr),
        entry!(0x00a01b00, tile_read_file(Ptr<Tile>, Ptr) -> Ptr<Tile>),
        entry!(0x00a01e20, tile_parse_file(Ptr, Ptr, u32) -> Ptr<BuildStorage>),
        entry!(
            0x00a02d40,
            tile_insert_included_files(Ptr<XmlStorage>) -> bool
        ),
        entry!(
            0x00a031b0,
            tile_build_and_name_tree(Ptr<Tile>, Ptr<TileTemplate>) -> Ptr<Tile>
        ),
        entry!(
            0x00a033b0,
            tile_connect_traits_to_tree(Ptr<Tile>, Ptr<TileTemplate>)
        ),
        entry!(
            0x00a037e0,
            tile_final_post_parse(Ptr<Tile>, i32, f32, u32) -> Ptr<Tile>
        ),
        entry!(0x00a03c60, tile_get_rendered_menu(Ptr<Tile>) -> bool),
        entry!(0x00a03c90, tile_get_menu(Ptr<Tile>) -> Ptr),
        entry!(0x00a03d40, tile_get_menu_tile(Ptr<Tile>) -> Ptr<Tile>),
        entry!(
            0x00a03da0,
            tile_get_child_by_name(Ptr<Tile>, Ptr) -> Ptr<Tile>
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// Every callee outside this file, given a do-nothing double by
    /// [`tile_engine`] (tests replace the ones they care about).
    const OUTSIDE_CALLEES: &[u32] = &[
        ATEXIT,
        ENTER_CRITICAL_SECTION,
        LEAVE_CRITICAL_SECTION,
        TILE_LOCK,
        TILE_UNLOCK,
        TILE_SET_PARENT,
        TILE_REMOVE_FADE_CONTROL,
        TILE_ADD_NEEDS_UPDATE,
        TILE_UPDATE_CHILDREN,
        VALUE_DESTROY,
        VALUE_SET_FLOAT,
        VALUE_SET_STRING,
        VALUE_ADD_ACTION,
        VALUE_ADD_ACTION_OV2,
        VALUE_CALCULATE_VALUE,
        VALUES_CLEAR,
        VALUES_DESTROY,
        LIST_REMOVE_HEAD,
        LIST_REMOVE_ALL,
        LIST_REMOVE_ITEM,
        INTERFACE_TILE_IS_BEING_DELETED,
        INTERFACE_GET_MENUS_ROOT,
        INTERFACE_TEST_CONSTANT_FOR_GAME_SETTINGS,
        INTERFACE_IS_WIDESCREEN,
        NI_POINTER_ASSIGN,
        STRING_SET,
        STRING_APPEND,
        STRING_ASSIGN,
        STRING_FORMAT,
        PRINT_ERROR,
        NI_OBJECT_GET_EXTRA_DATA,
        MENU_GET_XML,
        MENU_GET_MAX_DEPTH,
        MENU_ADD_TEMPLATE,
        XML_STORAGE_DESTROY,
        XML_STORAGE_SCALAR_DELETING_DESTRUCTOR,
        BUILD_STORAGE_DESTROY,
        TEMPLATE_ADD_PAIR,
        TILE_FIND_BY_PATH,
        TEXT_TABLE_FIND,
        TEXT_TABLE_ADD,
        TRAIT_EXTRA_DATA_ADD,
        TEXT_TABLE_NEXT,
        TEXT_IS_USER_NAME,
        TEXTURE_ATLAS_ENTRY_DESTROY,
        MAP_REMOVE_ALL,
        SIMPLE_LIST_REMOVE_ALL,
        MODEL_HOOK_00C45830,
        STRSTR,
        STRRCHR,
        STRCMP,
        STRTOK_S,
    ];

    /// An engine with do-nothing doubles for everything outside this file,
    /// constructors that return `this`, interlocked operations on memory,
    /// and the pages of the globals the code reads mapped (zeroed).
    fn tile_engine() -> Engine {
        let mut e = Engine::new();
        for &addr in OUTSIDE_CALLEES {
            e.register(addr, |_, _| Ret::default());
        }
        for addr in [
            TILE_BASE_CONSTRUCT,
            TILE_IMAGE_CONSTRUCT,
            TILE_3D_CONSTRUCT,
            RADIAL_TILE_CONSTRUCT,
            BUILD_STORAGE_CONSTRUCT,
        ] {
            e.register(addr, |_, a| a[0].into_ret());
        }
        e.register(INTERLOCKED_INCREMENT, |e, a| {
            let count = e.mem.i32(a[0]).wrapping_add(1);
            e.mem.set_i32(a[0], count);
            count.into_ret()
        });
        e.register(INTERLOCKED_DECREMENT, |e, a| {
            let count = e.mem.i32(a[0]).wrapping_sub(1);
            e.mem.set_i32(a[0], count);
            count.into_ret()
        });
        for page in [
            0x011a_6000,
            0x011d_8000,
            0x011f_3000,
            0x011f_4000,
            0x0101_2000,
            0x0102_3000,
            0x0109_4000,
        ] {
            e.map(page, 0x1000);
        }
        e
    }

    /// The `Tile::Value *` array doubles: a sorted search by trait id and a
    /// sorted insert (the arrays the game keeps for a tile's traits).
    fn install_value_array_doubles(e: &mut Engine) {
        e.register(VALUES_SORTED_FIND, |e, a| {
            let key = e.mem.i32(e.mem.u32(a[1]));
            let (buffer, size) = (e.mem.u32(a[0] + 4), e.mem.u32(a[0] + 8));
            for index in 0..size {
                if e.mem.i32(e.mem.u32(buffer + 4 * index)) == key {
                    return (index as i32).into_ret();
                }
            }
            (-1i32).into_ret()
        });
        e.register(VALUES_SORTED_INSERT, |e, a| {
            let value = e.mem.u32(a[1]);
            insert_value(e, a[0], value);
            Ret::default()
        });
    }

    /// Inserts `value` into the sorted `Tile::Value *` array at `array`.
    fn insert_value(e: &mut Engine, array: u32, value: u32) {
        let id = e.mem.i32(value);
        let mut buffer = e.mem.u32(array + 4);
        if buffer == 0 {
            buffer = e.mem.alloc(0x80);
            e.mem.set_u32(array + 4, buffer);
            e.mem.set_u32(array + 0xc, 32);
        }
        let size = e.mem.u32(array + 8);
        let mut at = 0;
        while at < size && e.mem.i32(e.mem.u32(buffer + 4 * at)) < id {
            at += 1;
        }
        for index in (at..size).rev() {
            let moved = e.mem.u32(buffer + 4 * index);
            e.mem.set_u32(buffer + 4 * (index + 1), moved);
        }
        e.mem.set_u32(buffer + 4 * at, value);
        e.mem.set_u32(array + 8, size + 1);
    }

    /// Gives the tile a float trait.
    fn give_trait(e: &mut Engine, tile: Ptr<Tile>, id: i32, value: f32) -> Ptr<TileValue> {
        let trait_value: Ptr<TileValue> = Ptr::new(e.mem.alloc(0x14));
        e.set(trait_value, TileValue::eIndex, id);
        e.set(trait_value, TileValue::pParent, tile);
        e.set(trait_value, TileValue::fValue, value);
        insert_value(e, tile.addr() + 0x10, trait_value.addr());
        trait_value
    }

    /// A NUL-terminated string in game memory.
    fn cstring(e: &mut Engine, text: &str) -> u32 {
        let block = e.mem.alloc(text.len() as u32 + 1);
        e.mem.set_cstr(block, text.as_bytes());
        block
    }

    fn string_at(e: &Engine, address: u32) -> String {
        String::from_utf8(e.mem.cstr(address)).unwrap()
    }

    /// The arguments of every logged call to `addr`.
    fn calls_to(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(called, _)| *called == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// Appends `child` to the children list of `parent`.
    fn add_child(e: &mut Engine, parent: Ptr<Tile>, child: Ptr<Tile>) {
        let list = parent.at(Tile::xChildren);
        let node = e.mem.alloc(12);
        let tail = e.get(list, NiTPointerList::m_pkTail);
        e.mem.set_u32(node + 4, tail);
        e.mem.set_u32(node + 8, child.addr());
        if tail == 0 {
            e.set(list, NiTPointerList::m_pkHead, node);
        } else {
            e.mem.set_u32(tail, node);
        }
        e.set(list, NiTPointerList::m_pkTail, node);
        let count = e.get(list, NiTPointerList::m_uiCount);
        e.set(list, NiTPointerList::m_uiCount, count + 1);
        e.set(child, Tile::pParent, parent);
    }

    /// A tile whose virtual `GetType` (slot +0xC) answers `kind`.
    fn typed_tile(e: &mut Engine, kind: u32) -> Ptr<Tile> {
        let vtable = 0x0400_0000 + kind * 0x20;
        let function = 0x0500_0000 + kind;
        e.put_vtable(vtable, &[0, 0, 0, function]);
        e.register_double(function, move |_, _| kind.into_ret());
        let tile: Ptr<Tile> = e.new_object();
        e.mem.set_u32(tile.addr(), vtable);
        tile
    }

    /// The `Menu` object of a menu-type tile; `depth` is its +0x18 word.
    fn make_menu_tile(e: &mut Engine, depth: i32) -> (Ptr<Tile>, Ptr) {
        let tile = typed_tile(e, TYPE_MENU);
        let menu: Ptr = Ptr::new(e.mem.alloc(0x28));
        e.mem.set_i32(menu.addr() + 0x18, depth);
        e.mem.set_u32(tile.addr() + 0x3c, menu.addr());
        (tile, menu)
    }

    /// `Lock` and `Unlock` calls balance.
    fn assert_lock_balanced(e: &Engine) {
        assert_eq!(calls_to(e, TILE_LOCK).len(), calls_to(e, TILE_UNLOCK).len());
    }

    #[test]
    fn destructor_clears_the_lists_flags_and_releases_the_model() {
        let mut e = tile_engine();
        let tile = typed_tile(&mut e, TYPE_RECT);
        e.set(
            tile,
            Tile::uiFlags,
            FLAG_RELEASED | FLAG_DIRTY | FLAG_HIBERNATED | FLAG_PROMOTED,
        );
        // A model with one reference and a destructor in vtable slot 1.
        let model = e.mem.alloc(0x20);
        e.mem.set_u32(model + 4, 1);
        e.put_vtable(0x0600_0000, &[0, 0x0600_1000]);
        e.mem.set_u32(model, 0x0600_0000);
        let destroyed = Rc::new(Cell::new(0));
        let seen = destroyed.clone();
        e.register_double(0x0600_1000, move |_, a| {
            seen.set(a[0]);
            Ret::default()
        });
        e.set(tile, Tile::spModel, Ptr::new(model));
        e.set_global(HIBERNATING_TILE_COUNT, 5u32);
        e.register(LIST_REMOVE_ITEM, |_, _| 1u32.into_ret());
        e.call_log = Some(vec![]);

        e.call(0x009f_f340, &args![tile]);

        assert_eq!(e.mem.u32(tile.addr()), VTABLE_TILE);
        assert_eq!(e.get(tile, Tile::uiFlags), FLAG_RELEASED);
        assert_eq!(e.global::<u32>(HIBERNATING_TILE_COUNT), 4);
        assert_eq!(destroyed.get(), model);
        assert_eq!(
            calls_to(&e, INTERFACE_TILE_IS_BEING_DELETED),
            vec![vec![tile.addr()]]
        );
        let removals: Vec<u32> = calls_to(&e, LIST_REMOVE_ITEM)
            .iter()
            .map(|args| args[0])
            .collect();
        assert_eq!(removals, vec![DIRTY_TILES_LIST, HIBERNATING_TILES_LIST]);
        assert_eq!(calls_to(&e, MODEL_HOOK_00C45830), vec![vec![model]]);
        assert_eq!(
            calls_to(&e, STRING_SET),
            vec![vec![tile.addr() + 0x20, 0, 0]]
        );
        assert_eq!(calls_to(&e, VALUES_DESTROY), vec![vec![tile.addr() + 0x10]]);
        assert_eq!(calls_to(&e, LIST_REMOVE_ALL), vec![vec![tile.addr() + 4]]);
        // The critical section is left again.
        assert_eq!(
            calls_to(&e, ENTER_CRITICAL_SECTION),
            calls_to(&e, LEAVE_CRITICAL_SECTION)
        );
    }

    #[test]
    fn destructor_keeps_the_hibernating_count_when_the_removal_answers_null() {
        let mut e = tile_engine();
        let tile = typed_tile(&mut e, TYPE_RECT);
        e.set(tile, Tile::uiFlags, FLAG_RELEASED | FLAG_HIBERNATED);
        e.set_global(HIBERNATING_TILE_COUNT, 5u32);
        e.register(LIST_REMOVE_ITEM, |_, _| 0u32.into_ret());
        e.call(0x009f_f340, &args![tile]);
        assert_eq!(e.global::<u32>(HIBERNATING_TILE_COUNT), 5);
        assert_eq!(e.get(tile, Tile::uiFlags), FLAG_RELEASED);
    }

    #[test]
    fn destructor_complains_about_and_releases_a_tile_nobody_released() {
        let mut e = tile_engine();
        let tile = typed_tile(&mut e, TYPE_RECT);
        e.call_log = Some(vec![]);
        e.call(0x009f_f340, &args![tile]);
        assert_eq!(
            calls_to(&e, PRINT_ERROR),
            vec![vec![MSG_BASE_TILE_NOT_RELEASED]]
        );
        assert_eq!(e.get(tile, Tile::uiFlags) & FLAG_RELEASED, FLAG_RELEASED);
        // `Release` detached the tile from its parent.
        assert_eq!(calls_to(&e, TILE_SET_PARENT), vec![vec![tile.addr(), 0, 0]]);
    }

    #[test]
    fn init_resets_the_tile_and_attaches_it_when_given_a_parent() {
        let mut e = tile_engine();
        let tile: Ptr<Tile> = e.new_object();
        e.set(tile, Tile::pParent, Ptr::new(0x1234));
        e.set(tile, Tile::uiFlags, 0xffff);
        e.set(tile, Tile::bSpeechChallengeFailure, true);
        let name = cstring(&mut e, "Name");
        e.call_log = Some(vec![]);
        e.call(
            0x009f_f600,
            &args![tile, Ptr::<Tile>::new(0x4000), Ptr::<()>::new(name), 7u32],
        );
        assert_eq!(e.get(tile, Tile::pParent), Ptr::NULL);
        assert_eq!(e.get(tile, Tile::uiFlags), 0);
        assert!(!e.get(tile, Tile::bSpeechChallengeFailure));
        assert_eq!(
            calls_to(&e, NI_POINTER_ASSIGN),
            vec![vec![tile.addr() + 0x2c, 0]]
        );
        assert_eq!(
            calls_to(&e, STRING_SET),
            vec![
                vec![tile.addr() + 0x20, EMPTY_NAME, 0],
                vec![tile.addr() + 0x20, name, 0]
            ]
        );
        assert_eq!(
            calls_to(&e, TILE_SET_PARENT),
            vec![vec![tile.addr(), 0x4000, 7]]
        );
    }

    #[test]
    fn init_without_parent_and_name_only_resets() {
        let mut e = tile_engine();
        let tile: Ptr<Tile> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(
            0x009f_f600,
            &args![tile, Ptr::<Tile>::NULL, Ptr::<()>::NULL, 0u32],
        );
        assert!(calls_to(&e, TILE_SET_PARENT).is_empty());
        assert_eq!(calls_to(&e, STRING_SET).len(), 1);
    }

    #[test]
    fn release_tears_the_tile_down() {
        let mut e = tile_engine();
        let tile = typed_tile(&mut e, TYPE_RECT);
        // Two trait values, one child, a model with a parent node.
        let first = give_trait(&mut e, tile, TRAIT_X, 1.0);
        let second = give_trait(&mut e, tile, TRAIT_Y, 2.0);
        let child = typed_tile(&mut e, TYPE_RECT);
        add_child(&mut e, tile, child);
        let model = e.mem.alloc(0x20);
        let parent_node = e.mem.alloc(0x20);
        e.mem.set_u32(model + 0x18, parent_node);
        e.put_vtable(0x0600_0000, &[0; 0x3c]);
        e.mem.set_u32(0x0600_0000 + 0xe8, 0x0600_1000);
        e.mem.set_u32(parent_node, 0x0600_0000);
        let removed = Rc::new(RefCell::new(vec![]));
        let seen = removed.clone();
        e.register_double(0x0600_1000, move |_, a| {
            seen.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        e.set(tile, Tile::spModel, Ptr::new(model));
        // The list pop hands out the child once.
        let pending = Rc::new(RefCell::new(vec![child.addr()]));
        let queue = pending.clone();
        e.register_double(LIST_REMOVE_HEAD, move |e, a| {
            let list = a[0];
            let child = queue.borrow_mut().pop().unwrap_or(0);
            let count = e.mem.u32(list + 8);
            e.mem.set_u32(list + 8, count - 1);
            child.into_ret()
        });
        // The child's scalar deleting destructor (slot 0) is recorded.
        let deleted = Rc::new(RefCell::new(vec![]));
        let seen = deleted.clone();
        e.register_double(0x0600_2000, move |_, a| {
            seen.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        e.put_vtable(
            0x0400_0000 + TYPE_RECT * 0x20,
            &[0x0600_2000, 0, 0, 0x0500_0385],
        );
        // The thread's current-line word is 13 while the tile is released.
        let line_word = e.tls() + TLS_CURRENT_LINE;
        e.mem.set_u32(line_word, 0x77);
        let during = Rc::new(Cell::new(0));
        let seen = during.clone();
        e.register_double(VALUES_CLEAR, move |e, _| {
            let line_word = e.tls() + TLS_CURRENT_LINE;
            seen.set(e.mem.u32(line_word));
            Ret::default()
        });
        e.call_log = Some(vec![]);

        e.call(0x009f_f690, &args![tile]);

        assert_eq!(during.get(), 13);
        assert_eq!(e.mem.u32(line_word), 0x77);
        assert_eq!(e.get(tile, Tile::uiFlags) & FLAG_RELEASED, FLAG_RELEASED);
        assert_eq!(
            calls_to(&e, VALUE_DESTROY),
            vec![vec![first.addr()], vec![second.addr()]]
        );
        // Both values are given back to the allocator.
        assert_eq!(e.mem.block_size(first.addr()), None);
        assert_eq!(e.mem.block_size(second.addr()), None);
        assert_eq!(
            calls_to(&e, VALUES_CLEAR),
            vec![vec![tile.addr() + 0x10, 1]]
        );
        assert_eq!(
            calls_to(&e, TILE_REMOVE_FADE_CONTROL),
            vec![vec![tile.addr(), 0x8000_0000]]
        );
        assert_eq!(removed.borrow().clone(), vec![vec![parent_node, model]]);
        assert_eq!(deleted.borrow().clone(), vec![vec![child.addr(), 1]]);
        assert_eq!(
            calls_to(&e, INTERFACE_TILE_IS_BEING_DELETED),
            vec![vec![tile.addr()]]
        );
        assert_lock_balanced(&e);
    }

    #[test]
    fn registering_a_user_name_raises_the_next_id_and_records_its_number() {
        let mut e = tile_engine();
        e.register(TEXT_IS_USER_NAME, |_, _| 1u32.into_ret());
        e.register(STRRCHR, |e, a| {
            let text = e.mem.cstr(a[0]);
            text.iter()
                .rposition(|&c| c == a[1] as u8)
                .map_or(0, |at| a[0] + at as u32)
                .into_ret()
        });
        e.set_global(TEXT_TABLE_USER_NEXT_ID, 10000i32);
        let name = cstring(&mut e, "user_12");
        e.call_log = Some(vec![]);
        e.call(0x009f_f8a0, &args![Ptr::<()>::new(name), 10005i32]);
        assert_eq!(e.global::<i32>(TEXT_TABLE_USER_NEXT_ID), 10006);
        assert_eq!(
            calls_to(&e, TEXT_TABLE_ADD),
            vec![vec![TEXT_TABLE, name, 10005]]
        );
        assert_eq!(
            calls_to(&e, TRAIT_EXTRA_DATA_ADD),
            vec![vec![TRAIT_EXTRA_DATA_MAP, 10005, 12]]
        );
        assert_lock_balanced(&e);
    }

    #[test]
    fn registering_a_small_id_only_enters_the_text_table() {
        let mut e = tile_engine();
        e.register(TEXT_IS_USER_NAME, |_, _| 1u32.into_ret());
        e.set_global(TEXT_TABLE_USER_NEXT_ID, 10000i32);
        let name = cstring(&mut e, "x_12");
        e.call_log = Some(vec![]);
        e.call(0x009f_f8a0, &args![Ptr::<()>::new(name), 9999i32]);
        assert_eq!(calls_to(&e, TEXT_TABLE_ADD).len(), 1);
        assert!(calls_to(&e, TRAIT_EXTRA_DATA_ADD).is_empty());
        assert_eq!(e.global::<i32>(TEXT_TABLE_USER_NEXT_ID), 10000);
    }

    #[test]
    fn registering_a_name_without_a_number_skips_the_id_map() {
        let mut e = tile_engine();
        e.register(TEXT_IS_USER_NAME, |_, _| 1u32.into_ret());
        e.register(STRRCHR, |_, _| 0u32.into_ret());
        e.set_global(TEXT_TABLE_USER_NEXT_ID, 20000i32);
        let name = cstring(&mut e, "plain");
        e.call_log = Some(vec![]);
        e.call(0x009f_f8a0, &args![Ptr::<()>::new(name), 10005i32]);
        // The next id was already past this one.
        assert_eq!(e.global::<i32>(TEXT_TABLE_USER_NEXT_ID), 20000);
        assert!(calls_to(&e, TRAIT_EXTRA_DATA_ADD).is_empty());
    }

    #[test]
    fn registering_an_empty_or_missing_name_does_nothing() {
        let mut e = tile_engine();
        let empty = cstring(&mut e, "");
        e.call_log = Some(vec![]);
        e.call(0x009f_f8a0, &args![Ptr::<()>::new(empty), 5i32]);
        e.call(0x009f_f8a0, &args![Ptr::<()>::NULL, 5i32]);
        assert!(e.call_log.as_ref().unwrap().len() == 2);
    }

    /// Makes every word of the text table non-empty in memory (the game's
    /// strings are in the exe's data, which tests do not map).
    fn provide_text_table_words(e: &mut Engine) {
        for &(name, _) in TEXT_TABLE_ENTRIES {
            e.map(name, 1);
            e.mem.set_u8(name, b'x');
        }
    }

    #[test]
    fn the_text_table_gets_all_its_words_in_order() {
        let mut e = tile_engine();
        provide_text_table_words(&mut e);
        e.set_global(HAS_360_CONTROLLER, 1u8);
        e.register(INTERFACE_IS_WIDESCREEN, |_, _| 1u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x009f_f970, &args![]);
        let adds = calls_to(&e, TEXT_TABLE_ADD);
        assert_eq!(adds.len(), 224);
        assert_eq!(adds[0], vec![TEXT_TABLE, 0x0109_4868, 0xffff_ffff]);
        assert_eq!(adds[223][0], TEXT_TABLE);
        let value_of = |name: u32| adds.iter().find(|add| add[1] == name).unwrap()[2];
        // A 360 controller without the console-UI byte: xenon and xbox are 1.
        assert_eq!(value_of(XENON_NAME), 1);
        assert_eq!(value_of(XBOX_NAME), 1);
        assert_eq!(value_of(WIDESCREEN_NAME), 1);
        // The list is in the game's order: the third word is `&true;`.
        assert_eq!(adds[2][1..], [0x0106_ec7c, 1]);
        // Tile types and user traits further down.
        assert_eq!(adds[37][2], 0x386);
        assert_eq!(adds[223][2], 0x138f);
    }

    #[test]
    fn xenon_and_xbox_are_zero_without_a_controller_or_with_the_console_byte() {
        let mut e = tile_engine();
        provide_text_table_words(&mut e);
        e.register(INTERFACE_IS_WIDESCREEN, |_, _| 0u32.into_ret());
        for (controller, console) in [(0u8, 0u8), (1, 1)] {
            e.set_global(HAS_360_CONTROLLER, controller);
            e.set_global(CONSOLE_UI_BYTE, console);
            e.call_log = Some(vec![]);
            e.call(0x009f_f970, &args![]);
            let adds = calls_to(&e, TEXT_TABLE_ADD);
            let value_of = |name: u32| adds.iter().find(|add| add[1] == name).unwrap()[2];
            assert_eq!(value_of(XENON_NAME), 0);
            assert_eq!(value_of(XBOX_NAME), 0);
            assert_eq!(value_of(WIDESCREEN_NAME), 0);
        }
    }

    #[test]
    fn add_user_trait_returns_an_existing_names_id() {
        let mut e = tile_engine();
        e.register(TEXT_TABLE_FIND, |e, a| {
            e.mem.set_i32(a[2], 42);
            1u32.into_ret()
        });
        let name = cstring(&mut e, "known");
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x00a0_0940, &args![Ptr::<()>::new(name), -1i32])
                .i32(),
            42
        );
        assert!(calls_to(&e, TEXT_TABLE_ADD).is_empty());
        assert_lock_balanced(&e);
    }

    #[test]
    fn add_user_trait_takes_the_next_free_id_for_a_new_name() {
        let mut e = tile_engine();
        e.register(TEXT_IS_USER_NAME, |_, _| 1u32.into_ret());
        e.set_global(TEXT_TABLE_USER_NEXT_ID, 10010i32);
        let name = cstring(&mut e, "fresh");
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x00a0_0940, &args![Ptr::<()>::new(name), -1i32])
                .i32(),
            10010
        );
        // `AddUserTrait` took 10010; registering it raised the next id.
        assert_eq!(e.global::<i32>(TEXT_TABLE_USER_NEXT_ID), 10011);
        assert_eq!(
            calls_to(&e, TEXT_TABLE_ADD),
            vec![vec![TEXT_TABLE, name, 10010]]
        );
        assert_lock_balanced(&e);
    }

    #[test]
    fn add_user_trait_refuses_small_ids_and_unacceptable_names() {
        let mut e = tile_engine();
        e.register(TEXT_IS_USER_NAME, |_, _| 1u32.into_ret());
        let name = cstring(&mut e, "small");
        assert_eq!(
            e.call(0x00a0_0940, &args![Ptr::<()>::new(name), 500i32])
                .i32(),
            i32::MIN
        );
        e.register(TEXT_IS_USER_NAME, |_, _| 0u32.into_ret());
        assert_eq!(
            e.call(0x00a0_0940, &args![Ptr::<()>::new(name), 20000i32])
                .i32(),
            i32::MIN
        );
    }

    #[test]
    fn free_trait_list_empties_the_maps_and_frees_both_lists() {
        let mut e = tile_engine();
        // Each static list: an embedded head node and one more node, both
        // with an item (a heap block).
        let mut items = vec![];
        for head in [TEXTURE_ENTRY_LIST, FADE_CONTROLS_LIST] {
            let second = e.mem.alloc(8);
            let (first_item, second_item) = (e.mem.alloc(8), e.mem.alloc(8));
            e.mem.set_u32(head, first_item);
            e.mem.set_u32(head + 4, second);
            e.mem.set_u32(second, second_item);
            items.push((first_item, second_item));
        }
        e.call_log = Some(vec![]);
        e.call(0x00a0_0a30, &args![]);
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(addr, _)| *addr)
            .filter(|addr| {
                [
                    MAP_REMOVE_ALL,
                    SIMPLE_LIST_REMOVE_ALL,
                    TEXTURE_ATLAS_ENTRY_DESTROY,
                ]
                .contains(addr)
            })
            .collect();
        assert_eq!(
            order,
            vec![
                MAP_REMOVE_ALL,
                MAP_REMOVE_ALL,
                TEXTURE_ATLAS_ENTRY_DESTROY,
                TEXTURE_ATLAS_ENTRY_DESTROY,
                SIMPLE_LIST_REMOVE_ALL,
                SIMPLE_LIST_REMOVE_ALL
            ]
        );
        // Only the texture entries are destroyed before being freed.
        assert_eq!(
            calls_to(&e, TEXTURE_ATLAS_ENTRY_DESTROY),
            vec![vec![items[0].0], vec![items[0].1]]
        );
        for (first, second) in items {
            assert_eq!(e.mem.block_size(first), None);
            assert_eq!(e.mem.block_size(second), None);
        }
        assert_eq!(
            calls_to(&e, SIMPLE_LIST_REMOVE_ALL),
            vec![vec![TEXTURE_ENTRY_LIST], vec![FADE_CONTROLS_LIST]]
        );
        assert_lock_balanced(&e);
    }

    #[test]
    fn set_menu_deleting_marks_the_tile_and_its_children() {
        let mut e = tile_engine();
        let tile = typed_tile(&mut e, TYPE_RECT);
        let child = typed_tile(&mut e, TYPE_RECT);
        let grandchild = typed_tile(&mut e, TYPE_RECT);
        let released = typed_tile(&mut e, TYPE_RECT);
        let marked = typed_tile(&mut e, TYPE_RECT);
        add_child(&mut e, tile, child);
        add_child(&mut e, child, grandchild);
        add_child(&mut e, tile, released);
        add_child(&mut e, released, marked);
        e.set(released, Tile::uiFlags, FLAG_RELEASED);
        e.set(marked, Tile::uiFlags, FLAG_MENU_DELETING);
        e.call_log = Some(vec![]);
        e.call(0x00a0_0b50, &args![tile]);
        assert_eq!(e.get(tile, Tile::uiFlags), FLAG_MENU_DELETING);
        assert_eq!(e.get(child, Tile::uiFlags), FLAG_MENU_DELETING);
        assert_eq!(e.get(grandchild, Tile::uiFlags), FLAG_MENU_DELETING);
        // A released tile is left alone, with its children.
        assert_eq!(e.get(released, Tile::uiFlags), FLAG_RELEASED);
        assert_eq!(e.get(marked, Tile::uiFlags), FLAG_MENU_DELETING);
        assert_lock_balanced(&e);
    }

    #[test]
    fn build_tile_constructs_each_kind() {
        let mut e = tile_engine();
        // The constructors that return `this`; `TileImage`'s is told apart
        // by writing a word.
        e.register(TILE_IMAGE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0] + 4, 0x1111);
            a[0].into_ret()
        });
        e.call_log = Some(vec![]);
        let rect = e.call(0x00a0_0c20, &args![TYPE_RECT]).ptr::<()>();
        assert_eq!(e.mem.u32(rect.addr()), VTABLE_TILE_RECT);
        assert_eq!(calls_to(&e, TILE_BASE_CONSTRUCT), vec![vec![rect.addr()]]);

        let image = e.call(0x00a0_0c20, &args![TYPE_IMAGE]).ptr::<()>();
        let hot_rect = e.call(0x00a0_0c20, &args![TYPE_HOT_RECT]).ptr::<()>();
        assert_eq!(e.mem.u32(image.addr() + 4), 0x1111);
        assert_eq!(e.mem.u32(hot_rect.addr() + 4), 0x1111);
        assert_eq!(e.mem.block_size(image.addr()), Some(0x48));

        let text = e.call(0x00a0_0c20, &args![TYPE_TEXT]).ptr::<()>();
        assert_eq!(e.mem.u32(text.addr()), VTABLE_TILE_TEXT);
        assert_eq!(e.mem.u8(text.addr() + 0x48), 0);
        assert_eq!(e.mem.block_size(text.addr()), Some(0x50));

        let three_d = e.call(0x00a0_0c20, &args![TYPE_3D]).ptr::<()>();
        assert_eq!(calls_to(&e, TILE_3D_CONSTRUCT), vec![vec![three_d.addr()]]);
        assert_eq!(e.mem.block_size(three_d.addr()), Some(0x50));

        let menu = e.call(0x00a0_0c20, &args![TYPE_MENU]).ptr::<()>();
        assert_eq!(e.mem.u32(menu.addr()), VTABLE_TILE_MENU);
        assert_eq!(e.mem.u32(menu.addr() + 0x3c), 0);

        let radial = e.call(0x00a0_0c20, &args![TYPE_RADIAL]).ptr::<()>();
        assert_eq!(
            calls_to(&e, RADIAL_TILE_CONSTRUCT),
            vec![vec![radial.addr()]]
        );
        assert_eq!(e.mem.block_size(radial.addr()), Some(0x48));
    }

    #[test]
    fn build_tile_makes_nothing_for_other_types() {
        let mut e = tile_engine();
        e.call_log = Some(vec![]);
        // The window type, the template type, and numbers outside the table.
        for kind in [0x38bu32, 999, 0, 0x384, 0x38d, 0xffff_ffff] {
            assert_eq!(e.call(0x00a0_0c20, &args![kind]).u32(), 0);
        }
        assert!(calls_to(&e, MEMORY_ALLOCATE).is_empty());
    }

    #[test]
    fn get_value_q_scans_the_sorted_traits() {
        let mut e = tile_engine();
        let tile = typed_tile(&mut e, TYPE_RECT);
        let _first = give_trait(&mut e, tile, -5, 0.5);
        let second = give_trait(&mut e, tile, 5, 1.5);
        let third = give_trait(&mut e, tile, 9, 2.5);
        e.call_log = Some(vec![]);
        let find = |e: &mut Engine, id: i32| e.call(0x00a0_0e90, &args![tile, id]).u32();
        assert_eq!(find(&mut e, 5), second.addr());
        assert_eq!(find(&mut e, 9), third.addr());
        // Between two ids, below all of them, above all of them.
        assert_eq!(find(&mut e, 6), 0);
        assert_eq!(find(&mut e, -6), 0);
        assert_eq!(find(&mut e, 10), 0);
        let bare = typed_tile(&mut e, TYPE_RECT);
        assert_eq!(e.call(0x00a0_0e90, &args![bare, 5i32]).u32(), 0);
        assert_eq!(
            calls_to(&e, ENTER_CRITICAL_SECTION).len(),
            calls_to(&e, LEAVE_CRITICAL_SECTION).len()
        );
        assert_eq!(
            calls_to(&e, ENTER_CRITICAL_SECTION)[0],
            vec![TILE_CRITICAL_SECTION]
        );
    }

    #[test]
    fn get_value_searches_with_the_shared_key() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        let _low = give_trait(&mut e, tile, 3, 1.0);
        let high = give_trait(&mut e, tile, 7, 2.0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x00a0_0f30, &args![tile, 7i32]).u32(), high.addr());
        assert_eq!(e.call(0x00a0_0f30, &args![tile, 4i32]).u32(), 0);
        // The key was set up on the first call (unset state, guard bit,
        // `atexit` registration, once) and holds the last id searched.
        assert_eq!(e.global::<u32>(GET_VALUE_KEY_GUARD) & 1, 1);
        assert_eq!(e.global::<i32>(GET_VALUE_KEY), 4);
        assert_eq!(e.global::<u32>(GET_VALUE_KEY + 4), 0);
        assert_eq!(calls_to(&e, ATEXIT), vec![vec![GET_VALUE_KEY_DESTROY]]);
        let finds = calls_to(&e, VALUES_SORTED_FIND);
        assert_eq!(finds.len(), 2);
        assert_eq!(finds[0][0], tile.addr() + 0x10);
        assert_eq!(finds[0][2], COMPARE_VALUES);
        assert_eq!(
            calls_to(&e, ENTER_CRITICAL_SECTION).len(),
            calls_to(&e, LEAVE_CRITICAL_SECTION).len()
        );
    }

    #[test]
    fn get_or_create_value_returns_or_inserts() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        let existing = give_trait(&mut e, tile, 3, 1.0);
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x00a0_1000, &args![tile, 3i32]).u32(),
            existing.addr()
        );
        assert!(calls_to(&e, VALUES_SORTED_INSERT).is_empty());

        let created: Ptr<TileValue> = e.call(0x00a0_1000, &args![tile, 1i32]).ptr();
        assert!(!created.is_null());
        assert_eq!(e.get(created, TileValue::eIndex), 1);
        assert_eq!(e.get(created, TileValue::pParent), tile);
        assert_eq!(e.get(created, TileValue::fValue), 0.0);
        assert!(e.get(created, TileValue::strValue).is_null());
        assert!(e.get(created, TileValue::pActionListA).is_null());
        assert_eq!(e.mem.block_size(created.addr()), Some(0x18));
        // The new value went in before the existing one.
        let buffer = e.mem.u32(tile.addr() + 0x14);
        assert_eq!(e.mem.u32(buffer), created.addr());
        assert_eq!(e.mem.u32(buffer + 4), existing.addr());
        assert_eq!(e.mem.u32(tile.addr() + 0x18), 2);
        let inserts = calls_to(&e, VALUES_SORTED_INSERT);
        assert_eq!(inserts.len(), 1);
        assert_eq!(inserts[0][0], tile.addr() + 0x10);
        assert_eq!(inserts[0][2], COMPARE_VALUES);
        assert_eq!(calls_to(&e, ATEXIT), vec![vec![GET_OR_CREATE_KEY_DESTROY]]);
    }

    #[test]
    fn the_value_comparison_orders_by_signed_id() {
        let mut e = tile_engine();
        let slot_for = |e: &mut Engine, id: i32| {
            let value = e.mem.alloc(0x14);
            e.mem.set_i32(value, id);
            let slot = e.mem.alloc(4);
            e.mem.set_u32(slot, value);
            slot
        };
        let (low, high, same) = (
            slot_for(&mut e, -3),
            slot_for(&mut e, 4),
            slot_for(&mut e, 4),
        );
        assert_eq!(
            e.call(
                0x00a0_1160,
                &args![Ptr::<()>::new(low), Ptr::<()>::new(high)]
            )
            .i32(),
            -1
        );
        assert_eq!(
            e.call(
                0x00a0_1160,
                &args![Ptr::<()>::new(high), Ptr::<()>::new(low)]
            )
            .i32(),
            1
        );
        assert_eq!(
            e.call(
                0x00a0_1160,
                &args![Ptr::<()>::new(high), Ptr::<()>::new(same)]
            )
            .i32(),
            0
        );
    }

    #[test]
    fn trait_getters_read_the_value_or_give_zero() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        let value = give_trait(&mut e, tile, TRAIT_X, 2.5);
        e.set(value, TileValue::strValue, Ptr::new(0x7000));
        assert_eq!(e.call(0x00a0_11b0, &args![tile, TRAIT_X]).f32(), 2.5);
        assert_eq!(e.call(0x00a0_11b0, &args![tile, TRAIT_Y]).f32(), 0.0);
        assert_eq!(e.call(0x00a0_11f0, &args![tile, TRAIT_X]).u32(), 0x7000);
        assert_eq!(e.call(0x00a0_11f0, &args![tile, TRAIT_Y]).u32(), 0);
    }

    #[test]
    fn is_true_tests_the_float_against_zero() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        give_trait(&mut e, tile, 1, 0.0);
        give_trait(&mut e, tile, 2, 2.5);
        give_trait(&mut e, tile, 3, -0.5);
        give_trait(&mut e, tile, 4, f32::NAN);
        let is_true = |e: &mut Engine, id: i32| e.call(0x00a0_1230, &args![tile, id]).bool();
        assert!(!is_true(&mut e, 1));
        assert!(is_true(&mut e, 2));
        assert!(is_true(&mut e, 3));
        // Not a number differs from zero.
        assert!(is_true(&mut e, 4));
        // No such trait.
        assert!(!is_true(&mut e, 5));
    }

    #[test]
    fn poke_sets_the_value_and_then_zero() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        e.call_log = Some(vec![]);
        e.call(0x00a0_1290, &args![tile, TRAIT_VISIBLE, 1.5f32]);
        let value = e.mem.u32(e.mem.u32(tile.addr() + 0x14));
        assert_eq!(
            calls_to(&e, VALUE_SET_FLOAT),
            vec![
                vec![value, 1.5f32.to_bits(), 1],
                vec![value, 0.0f32.to_bits(), 1]
            ]
        );
        // The value was created once.
        assert_eq!(e.mem.u32(tile.addr() + 0x18), 1);
        assert_lock_balanced(&e);
    }

    #[test]
    fn setting_a_float_trait_creates_it_and_locks() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        e.call_log = Some(vec![]);
        e.call(0x00a0_12d0, &args![tile, TRAIT_ALPHA, 0.25f32, true]);
        let value = e.mem.u32(e.mem.u32(tile.addr() + 0x14));
        assert_eq!(e.mem.i32(value), TRAIT_ALPHA);
        let log = e.call_log.as_ref().unwrap();
        let positions: Vec<usize> = [TILE_LOCK, VALUE_SET_FLOAT, TILE_UNLOCK]
            .iter()
            .map(|addr| log.iter().position(|(called, _)| called == addr).unwrap())
            .collect();
        assert!(positions[0] < positions[1] && positions[1] < positions[2]);
        assert_eq!(
            calls_to(&e, VALUE_SET_FLOAT),
            vec![vec![value, 0.25f32.to_bits(), 1]]
        );
    }

    #[test]
    fn setting_a_string_trait_creates_it_and_locks() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        e.call_log = Some(vec![]);
        e.call(
            0x00a0_1350,
            &args![tile, TRAIT_FILENAME, Ptr::<()>::new(0x7100), false],
        );
        let value = e.mem.u32(e.mem.u32(tile.addr() + 0x14));
        assert_eq!(calls_to(&e, VALUE_SET_STRING), vec![vec![value, 0x7100, 0]]);
        assert_lock_balanced(&e);
        assert_eq!(calls_to(&e, TILE_LOCK).len(), 1);
    }

    /// root <- middle <- leaf, each with the given x, y, depth and locus.
    fn chain(e: &mut Engine, traits: [[f32; 4]; 3]) -> [Ptr<Tile>; 3] {
        install_value_array_doubles(e);
        let tiles = [
            typed_tile(e, TYPE_RECT),
            typed_tile(e, TYPE_RECT),
            typed_tile(e, TYPE_RECT),
        ];
        add_child(e, tiles[0], tiles[1]);
        add_child(e, tiles[1], tiles[2]);
        for (tile, values) in tiles.iter().zip(traits) {
            for (id, value) in [TRAIT_X, TRAIT_Y, TRAIT_DEPTH, TRAIT_LOCUS]
                .into_iter()
                .zip(values)
            {
                give_trait(e, *tile, id, value);
            }
        }
        tiles
    }

    #[test]
    fn position_sums_add_the_ancestors_with_a_locus() {
        let mut e = tile_engine();
        // [x, y, depth, locus]: the middle tile has a locus, the root none.
        let [_, _, leaf] = chain(
            &mut e,
            [
                [100.0, 200.0, 10.0, 0.0],
                [10.0, 20.0, 1.0, 1.0],
                [1.0, 2.0, 0.5, 0.0],
            ],
        );
        assert_eq!(e.call(0x00a0_13d0, &args![leaf]).f32(), 11.0);
        assert_eq!(e.call(0x00a0_1440, &args![leaf]).f32(), 22.0);
        // A tile without a parent is just its own value.
        let lone = typed_tile(&mut e, TYPE_RECT);
        give_trait(&mut e, lone, TRAIT_X, 3.5);
        assert_eq!(e.call(0x00a0_13d0, &args![lone]).f32(), 3.5);
    }

    #[test]
    fn depth_sum_adds_ancestors_with_a_locus_or_under_the_menus_root() {
        let mut e = tile_engine();
        let [root, middle, leaf] = chain(
            &mut e,
            [
                [0.0, 0.0, 10.0, 0.0],
                [0.0, 0.0, 2.0, 0.0],
                [0.0, 0.0, 0.5, 0.0],
            ],
        );
        e.register_double(INTERFACE_GET_MENUS_ROOT, move |_, _| root.addr().into_ret());
        // The middle tile has no locus, but its parent is the menus root.
        assert_eq!(e.call(0x00a0_14b0, &args![leaf]).f32(), 2.5);
        // With a locus on the middle tile the root is still not counted.
        give_trait_value(&mut e, middle, TRAIT_LOCUS, 1.0);
        assert_eq!(e.call(0x00a0_14b0, &args![leaf]).f32(), 2.5);
        // With a locus on the root it is counted as well.
        give_trait_value(&mut e, root, TRAIT_LOCUS, 1.0);
        assert_eq!(e.call(0x00a0_14b0, &args![leaf]).f32(), 12.5);
    }

    /// Changes the value of an existing trait.
    fn give_trait_value(e: &mut Engine, tile: Ptr<Tile>, id: i32, value: f32) {
        let found: Ptr<TileValue> = tile_get_value(e, tile, id).cast();
        e.set(found, TileValue::fValue, value);
    }

    #[test]
    fn maximum_depth_is_the_menus_answer() {
        let mut e = tile_engine();
        e.register(MENU_GET_MAX_DEPTH, |_, _| 3.5f32.into_ret());
        assert_eq!(e.call(0x00a0_1530, &args![]).f32(), 3.5);
    }

    #[test]
    fn subtree_depth_takes_the_deepest_child() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let top = typed_tile(&mut e, TYPE_RECT);
        let (first, second, grandchild) = (
            typed_tile(&mut e, TYPE_RECT),
            typed_tile(&mut e, TYPE_RECT),
            typed_tile(&mut e, TYPE_RECT),
        );
        add_child(&mut e, top, first);
        add_child(&mut e, top, second);
        add_child(&mut e, second, grandchild);
        for (tile, depth) in [(top, 1.0), (first, 5.0), (second, 3.0), (grandchild, 4.0)] {
            give_trait(&mut e, tile, TRAIT_DEPTH, depth);
        }
        // No locus anywhere: the deepest of 1, 5, 3 and 4.
        assert_eq!(e.call(0x00a0_1540, &args![top]).f32(), 5.0);
        // A locus on the top tile adds its depth to what is below it.
        give_trait(&mut e, top, TRAIT_LOCUS, 1.0);
        assert_eq!(e.call(0x00a0_1540, &args![top]).f32(), 6.0);
        // A tile without children gives its own depth.
        assert_eq!(e.call(0x00a0_1540, &args![grandchild]).f32(), 4.0);
    }

    /// A node with an extra data object (reference count `count`, tile
    /// pointer `tile`); `GetExtraData` answers for the nodes in `map`.
    fn extra_data(e: &mut Engine, count: i32, tile: u32) -> u32 {
        let extra = e.mem.alloc(0x20);
        e.mem.set_i32(extra + 4, count);
        e.mem.set_u32(extra + 0xc, tile);
        e.put_vtable(0x0600_3000, &[0, 0x0600_3100]);
        e.mem.set_u32(extra, 0x0600_3000);
        extra
    }

    fn install_extra_data(e: &mut Engine, map: Vec<(u32, u32)>) {
        e.register_double(NI_OBJECT_GET_EXTRA_DATA, move |e, a| {
            assert_eq!(a[1], e.global::<u32>(TILE_EXTRA_DATA_KEY));
            map.iter()
                .find(|(node, _)| *node == a[0])
                .map_or(0, |(_, extra)| *extra)
                .into_ret()
        });
        // `NiPointer::operator=`: takes a reference to the new object.
        e.register(NI_POINTER_ASSIGN, |e, a| {
            if a[1] != 0 {
                let count = e.mem.i32(a[1] + 4);
                e.mem.set_i32(a[1] + 4, count + 1);
            }
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
    }

    #[test]
    fn tile_from_node_reads_the_nodes_own_extra_data() {
        let mut e = tile_engine();
        e.set_global(TILE_EXTRA_DATA_KEY, 0x1234u32);
        let (node, parent_node) = (e.mem.alloc(0x20), e.mem.alloc(0x20));
        e.mem.set_u32(node + 0x18, parent_node);
        let extra = extra_data(&mut e, 1, 0xa000);
        let parent_extra = extra_data(&mut e, 1, 0xb000);
        install_extra_data(&mut e, vec![(node, extra), (parent_node, parent_extra)]);
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x00a0_1630, &args![Ptr::<()>::new(node)]).u32(),
            0xa000
        );
        // The reference taken while reading is dropped again.
        assert_eq!(e.mem.i32(extra + 4), 1);
        assert_eq!(calls_to(&e, NI_OBJECT_GET_EXTRA_DATA).len(), 1);
        assert_lock_balanced(&e);
    }

    #[test]
    fn tile_from_node_falls_back_to_the_parent_node() {
        let mut e = tile_engine();
        e.set_global(TILE_EXTRA_DATA_KEY, 0x1234u32);
        let (node, parent_node) = (e.mem.alloc(0x20), e.mem.alloc(0x20));
        e.mem.set_u32(node + 0x18, parent_node);
        let parent_extra = extra_data(&mut e, 1, 0xb000);
        install_extra_data(&mut e, vec![(parent_node, parent_extra)]);
        assert_eq!(
            e.call(0x00a0_1630, &args![Ptr::<()>::new(node)]).u32(),
            0xb000
        );
        assert_eq!(e.mem.i32(parent_extra + 4), 1);
        // Neither node has extra data: no tile.
        install_extra_data(&mut e, vec![]);
        assert_eq!(e.call(0x00a0_1630, &args![Ptr::<()>::new(node)]).u32(), 0);
        // No node at all: nothing is looked up.
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x00a0_1630, &args![Ptr::<()>::NULL]).u32(), 0);
        assert!(calls_to(&e, NI_OBJECT_GET_EXTRA_DATA).is_empty());
        assert_lock_balanced(&e);
    }

    #[test]
    fn tile_from_node_destroys_extra_data_nobody_else_holds() {
        let mut e = tile_engine();
        let node = e.mem.alloc(0x20);
        // Reference count 0: taking and dropping the reference frees it.
        let extra = extra_data(&mut e, 0, 0xa000);
        install_extra_data(&mut e, vec![(node, extra)]);
        let destroyed = Rc::new(Cell::new(0));
        let seen = destroyed.clone();
        e.register_double(0x0600_3100, move |_, a| {
            seen.set(a[0]);
            Ret::default()
        });
        assert_eq!(
            e.call(0x00a0_1630, &args![Ptr::<()>::new(node)]).u32(),
            0xa000
        );
        assert_eq!(destroyed.get(), extra);
    }

    #[test]
    fn sever_extra_data_clears_the_tile_pointer_in_the_models_extra_data() {
        let mut e = tile_engine();
        let tile = typed_tile(&mut e, TYPE_RECT);
        let model = e.mem.alloc(0x20);
        e.mem.set_u16(model + 0x14, 1);
        e.set(tile, Tile::spModel, Ptr::new(model));
        let extra = extra_data(&mut e, 1, 0xa000);
        install_extra_data(&mut e, vec![(model, extra)]);
        e.call_log = Some(vec![]);
        e.call(0x00a0_1750, &args![tile]);
        assert_eq!(e.mem.u32(extra + 0xc), 0);
        assert_eq!(e.mem.i32(extra + 4), 1);
        assert_lock_balanced(&e);

        // A model without extra data (count 0) is not asked.
        e.mem.set_u16(model + 0x14, 0);
        e.mem.set_u32(extra + 0xc, 0xa000);
        e.call_log = Some(vec![]);
        e.call(0x00a0_1750, &args![tile]);
        assert!(calls_to(&e, NI_OBJECT_GET_EXTRA_DATA).is_empty());
        assert_eq!(e.mem.u32(extra + 0xc), 0xa000);

        // No model at all.
        e.set(tile, Tile::spModel, Ptr::NULL);
        e.call(0x00a0_1750, &args![tile]);
        assert!(calls_to(&e, NI_OBJECT_GET_EXTRA_DATA).is_empty());
    }

    #[test]
    fn text_to_trait_looks_the_name_up_in_the_text_table() {
        let mut e = tile_engine();
        e.register(TEXT_TABLE_FIND, |e, a| {
            if e.mem.cstr(a[1]) == b"alpha" {
                e.mem.set_i32(a[2], TRAIT_ALPHA);
                1u32.into_ret()
            } else {
                0u32.into_ret()
            }
        });
        let known = cstring(&mut e, "alpha");
        let unknown = cstring(&mut e, "nonsense");
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x00a0_1860, &args![Ptr::<()>::new(known)]).i32(),
            TRAIT_ALPHA
        );
        // A name the table does not know gives `eNone`.
        assert_eq!(
            e.call(0x00a0_1860, &args![Ptr::<()>::new(unknown)]).i32(),
            TRAIT_NONE
        );
        assert_eq!(calls_to(&e, TEXT_TABLE_FIND)[0][0], TEXT_TABLE);
        assert_lock_balanced(&e);
    }

    #[test]
    fn splitting_a_name_from_its_bracketed_argument() {
        let mut e = tile_engine();
        let seen = Rc::new(RefCell::new(vec![]));
        let names = seen.clone();
        e.register_double(TEXT_TABLE_FIND, move |e, a| {
            names.borrow_mut().push(string_at(e, a[1]));
            e.mem.set_i32(a[2], 77);
            1u32.into_ret()
        });
        let argument = e.mem.alloc(0x100);
        let split = |e: &mut Engine, text: &str| {
            let name = cstring(e, text);
            let result = e
                .call(
                    0x00a0_18d0,
                    &args![Ptr::<()>::new(name), Ptr::<()>::new(argument)],
                )
                .i32();
            (result, string_at(e, argument))
        };
        assert_eq!(split(&mut e, "trait(arg)"), (77, "arg".to_string()));
        assert_eq!(split(&mut e, "plain"), (77, String::new()));
        // Nothing after the closing bracket is looked at.
        assert_eq!(split(&mut e, "a(b)c"), (77, "b".to_string()));
        assert_eq!(split(&mut e, "(x)"), (77, "x".to_string()));
        assert_eq!(seen.borrow().clone(), vec!["trait", "plain", "a", ""]);
    }

    #[test]
    fn splitting_needs_a_name_and_a_buffer() {
        let mut e = tile_engine();
        e.call_log = Some(vec![]);
        let name = cstring(&mut e, "a(b)");
        let empty = cstring(&mut e, "");
        let argument = e.mem.alloc(0x100);
        for (name, argument) in [(0, argument), (empty, argument), (name, 0)] {
            assert_eq!(
                e.call(
                    0x00a0_18d0,
                    &args![Ptr::<()>::new(name), Ptr::<()>::new(argument)]
                )
                .i32(),
                i32::MIN
            );
        }
        assert!(calls_to(&e, TEXT_TABLE_FIND).is_empty());
    }

    #[test]
    fn the_number_at_the_end_of_a_name() {
        let mut e = tile_engine();
        e.register(STRRCHR, |e, a| {
            let text = e.mem.cstr(a[0]);
            text.iter()
                .rposition(|&c| c == a[1] as u8)
                .map_or(0, |at| a[0] + at as u32)
                .into_ret()
        });
        let number = |e: &mut Engine, text: &str| {
            let name = cstring(e, text);
            e.call(0x00a0_1a20, &args![Ptr::<()>::new(name)]).i32()
        };
        assert_eq!(number(&mut e, "name_12"), 12);
        assert_eq!(number(&mut e, "a_b_7"), 7);
        assert_eq!(number(&mut e, "name"), 0);
        assert_eq!(number(&mut e, "name_"), -1);
        assert_eq!(number(&mut e, "x_abc"), 0);
    }

    /// A text-table node: next, key, value.
    fn table_node(e: &mut Engine, next: u32, key: u32, value: i32) -> u32 {
        let node = e.mem.alloc(12);
        e.mem.set_u32(node, next);
        e.mem.set_u32(node + 4, key);
        e.mem.set_i32(node + 8, value);
        node
    }

    #[test]
    fn the_name_of_a_text_table_value() {
        let mut e = tile_engine();
        // The iterator step: writes the node's key and value to the two out
        // parameters and advances the node.
        e.register(TEXT_TABLE_NEXT, |e, a| {
            let node = e.mem.u32(a[1]);
            let (key, value, next) = (e.mem.u32(node + 4), e.mem.u32(node + 8), e.mem.u32(node));
            e.mem.set_u32(a[2], key);
            e.mem.set_u32(a[3], value);
            e.mem.set_u32(a[1], next);
            Ret::default()
        });
        let third = table_node(&mut e, 0, 0x9003, 33);
        let second = table_node(&mut e, third, 0x9002, 22);
        let first = table_node(&mut e, second, 0x9001, 11);
        let buckets = e.mem.alloc(16);
        e.mem.set_u32(buckets + 4, first);
        e.set_global(TEXT_TABLE_BUCKET_COUNT, 4u32);
        e.set_global(TEXT_TABLE_BUCKETS, buckets);
        assert_eq!(e.call(0x00a0_1a70, &args![22i32]).u32(), 0x9002);
        assert_eq!(e.call(0x00a0_1a70, &args![11i32]).u32(), 0x9001);
        assert_eq!(e.call(0x00a0_1a70, &args![99i32]).u32(), 0);
        // An empty table never steps.
        e.mem.set_u32(buckets + 4, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x00a0_1a70, &args![11i32]).u32(), 0);
        assert!(calls_to(&e, TEXT_TABLE_NEXT).is_empty());
    }

    /// One `AddPair` call: command, text (read when it was made), line, flag.
    type Pair = (i32, String, i32, u32);

    /// Doubles for what the XML reader hands its results to: `AddPair`
    /// records its arguments, and the text table knows a few words.
    fn install_reader_doubles(e: &mut Engine) -> Rc<RefCell<Vec<Pair>>> {
        let pairs = Rc::new(RefCell::new(vec![]));
        let record = pairs.clone();
        e.register_double(TEMPLATE_ADD_PAIR, move |e, a| {
            record
                .borrow_mut()
                .push((a[1] as i32, string_at(e, a[2]), a[3] as i32, a[4]));
            Ret::default()
        });
        e.register(TEXT_TABLE_FIND, |e, a| {
            let id = match e.mem.cstr(a[1]).as_slice() {
                b"x" => TRAIT_X,
                b"y" => TRAIT_Y,
                b"width" => TRAIT_WIDTH,
                _ => return 0u32.into_ret(),
            };
            e.mem.set_i32(a[2], id);
            1u32.into_ret()
        });
        e.set_global(XML_ERROR_FLAG, 0u8);
        pairs
    }

    /// Runs the reader over `text`, returning what it produced.
    fn parse(e: &mut Engine, text: &str) -> (Vec<Pair>, Ptr<BuildStorage>) {
        let pairs = install_reader_doubles(e);
        let buffer = cstring(e, text);
        let path = cstring(e, "menu.xml");
        let storage: Ptr<BuildStorage> = e
            .call(
                0x00a0_1e20,
                &args![
                    Ptr::<()>::new(path),
                    Ptr::<()>::new(buffer),
                    text.len() as u32
                ],
            )
            .ptr();
        let pairs = pairs.borrow().clone();
        (pairs, storage)
    }

    fn pair(command: i32, text: &str, line: i32, flag: u32) -> Pair {
        (command, text.to_string(), line, flag)
    }

    #[test]
    fn parsing_a_tag_with_an_attribute_and_text() {
        let mut e = tile_engine();
        let (pairs, storage) = parse(&mut e, r#"<image x="5">hi</image>"#);
        assert!(!storage.is_null());
        assert_eq!(
            pairs,
            vec![
                // The tag name when it ends at white space.
                pair(0, "image", 1, 0),
                // The attribute: trait id of the name, the value.
                pair(TRAIT_X, "5", 2, 0),
                // The text between the tags, flagged as plain text.
                pair(TAG_VALUE, "hi", 2, 1),
                // The close tag.
                pair(1, "image", 2, 0),
            ]
        );
        assert_eq!(e.global::<u8>(XML_ERROR_FLAG), 0);
    }

    #[test]
    fn parsing_a_self_closing_tag_opens_and_closes_it() {
        let mut e = tile_engine();
        let (pairs, _) = parse(&mut e, "<a/>");
        assert_eq!(pairs, vec![pair(0, "a", 1, 0), pair(1, "a", 2, 0)]);
    }

    #[test]
    fn parsing_attribute_values_keeps_spaces_inside_quotes() {
        let mut e = tile_engine();
        let (pairs, _) = parse(&mut e, "<a x=\"1 2\" y=7 width=\"3\"/>");
        assert_eq!(
            pairs,
            vec![
                pair(0, "a", 1, 0),
                pair(TRAIT_X, "1 2", 2, 0),
                pair(TRAIT_Y, "7", 2, 0),
                pair(TRAIT_WIDTH, "3", 2, 0),
                pair(1, "a", 2, 0),
            ]
        );
    }

    #[test]
    fn parsing_skips_comments() {
        let mut e = tile_engine();
        let (pairs, _) = parse(&mut e, "<!-- <skip x=\"1\"> --><a/>");
        assert_eq!(pairs, vec![pair(0, "a", 1, 0), pair(1, "a", 2, 0)]);
        // A comment between a tag's end and its text too.
        let (pairs, _) = parse(&mut e, "<a><!-- c -->t</a>");
        assert_eq!(
            pairs,
            vec![
                pair(0, "a", 1, 0),
                pair(TAG_VALUE, "t", 2, 1),
                pair(1, "a", 2, 0)
            ]
        );
    }

    #[test]
    fn parsing_text_trims_the_end_but_not_the_middle() {
        let mut e = tile_engine();
        let (pairs, _) = parse(&mut e, "<a>  text  with space  </a>");
        assert_eq!(pairs[1], pair(TAG_VALUE, "text  with space", 2, 1));
    }

    #[test]
    fn parsing_a_constant_expands_game_settings() {
        let mut e = tile_engine();
        e.register(INTERFACE_TEST_CONSTANT_FOR_GAME_SETTINGS, |e, a| {
            if e.mem.cstr(a[0]) == b"&-known;" {
                e.mem.set_cstr(a[1], b"7");
                1u32.into_ret()
            } else {
                0u32.into_ret()
            }
        });
        let (pairs, _) = parse(&mut e, "<a>&-known;</a>");
        assert_eq!(pairs[1], pair(TAG_VALUE, "7", 2, 0));
        // A game setting the interface does not know adds nothing.
        let (pairs, _) = parse(&mut e, "<a>&-other;</a>");
        assert_eq!(pairs.len(), 2);
        // A plain constant is entered as it stands, without the flag.
        let (pairs, _) = parse(&mut e, "<a>&true;</a>");
        assert_eq!(pairs[1], pair(TAG_VALUE, "&true;", 2, 0));
    }

    /// Parses `text`, expecting the reader to stop with `message`.
    fn assert_parse_error(text: &str, message: u32, line: u32) {
        let mut e = tile_engine();
        e.call_log = Some(vec![]);
        let (_, storage) = parse(&mut e, text);
        assert!(storage.is_null(), "{text}");
        assert_eq!(e.global::<u8>(XML_ERROR_FLAG), 1, "{text}");
        let prints = calls_to(&e, PRINT_ERROR);
        assert_eq!(prints.len(), 1, "{text}");
        assert_eq!(prints[0][0], XML_ERROR_FORMAT);
        assert_eq!(prints[0][1], message, "{text}");
        assert_eq!(prints[0][3], line, "{text}");
        // The path given to the reader is the second argument.
        assert_eq!(string_at(&e, prints[0][2]), "menu.xml");
    }

    #[test]
    fn parsing_reports_malformed_xml() {
        assert_parse_error("<>", MSG_EMPTY_TAG_NAME, 1);
        assert_parse_error("<a x>", MSG_ATTRIBUTE_NO_VALUE, 2);
        assert_parse_error("<a =1>", MSG_MISSING_ATTRIBUTE_NAME, 2);
        assert_parse_error("<a x y=1>", MSG_UNEXPECTED_WORD, 2);
        assert_parse_error("<a x=>", MSG_MISSING_ATTRIBUTE_VALUE, 2);
        assert_parse_error("<a>/></a>", MSG_UNBALANCED_CLOSE_PAIR, 2);
        assert_parse_error("<a>></a>", MSG_UNBALANCED_END_OF_TAG, 2);
    }

    #[test]
    fn parsing_warns_when_the_text_buffer_overflows() {
        let mut e = tile_engine();
        e.call_log = Some(vec![]);
        let text = format!("<a>{}</a>", "b".repeat(0x1001));
        let (pairs, storage) = parse(&mut e, &text);
        assert!(!storage.is_null());
        assert_eq!(pairs[1].1.len(), 0x1001);
        assert_eq!(calls_to(&e, PRINT_ERROR), vec![vec![MSG_BUFFER_TOO_SMALL]]);
    }

    #[test]
    fn parsing_adds_to_the_current_template_of_the_storage() {
        let mut e = tile_engine();
        install_reader_doubles(&mut e);
        // The storage constructor leaves the template pointers to the test.
        let (main, current) = (e.mem.alloc(0x14), e.mem.alloc(0x14));
        let use_current = Rc::new(Cell::new(false));
        let flag = use_current.clone();
        e.register_double(BUILD_STORAGE_CONSTRUCT, move |e, a| {
            e.mem.set_u32(a[0], main);
            if flag.get() {
                e.mem.set_u32(a[0] + 0xc, current);
            }
            a[0].into_ret()
        });
        let used = Rc::new(RefCell::new(vec![]));
        let seen = used.clone();
        e.register_double(TEMPLATE_ADD_PAIR, move |_, a| {
            seen.borrow_mut().push(a[0]);
            Ret::default()
        });
        let buffer = cstring(&mut e, "<a/>");
        let path = cstring(&mut e, "m.xml");
        for (current_set, expected) in [(false, main), (true, current)] {
            use_current.set(current_set);
            used.borrow_mut().clear();
            e.call(
                0x00a0_1e20,
                &args![Ptr::<()>::new(path), Ptr::<()>::new(buffer), 4u32],
            );
            assert_eq!(used.borrow().clone(), vec![expected, expected]);
        }
    }

    /// Makes `Menu::GetXML` answer for `files` (path, text) and records the
    /// paths asked for.
    fn install_xml_files(
        e: &mut Engine,
        files: Vec<(&'static str, &'static str)>,
    ) -> Rc<RefCell<Vec<String>>> {
        let asked = Rc::new(RefCell::new(vec![]));
        let record = asked.clone();
        e.register_double(MENU_GET_XML, move |e, a| {
            let path = string_at(e, a[0]);
            record.borrow_mut().push(path.clone());
            let Some((_, text)) = files.iter().find(|(name, _)| *name == path) else {
                return 0u32.into_ret();
            };
            xml_storage(e, text).into_ret()
        });
        asked
    }

    /// An `XMLStorage` (size, text) holding `text`.
    fn xml_storage(e: &mut Engine, text: &str) -> u32 {
        let xml = e.mem.alloc(8);
        let data = cstring(e, text);
        e.mem.set_u32(xml, text.len() as u32);
        e.mem.set_u32(xml + 4, data);
        xml
    }

    #[test]
    fn parsing_a_file_reads_it_expands_includes_and_frees_the_text() {
        let mut e = tile_engine();
        let asked = install_xml_files(&mut e, vec![("menu.xml", "<a/>")]);
        let pairs = install_reader_doubles(&mut e);
        e.call_log = Some(vec![]);
        let path = cstring(&mut e, "menu.xml");
        let storage = e
            .call(
                0x00a0_1e20,
                &args![Ptr::<()>::new(path), Ptr::<()>::NULL, 0u32],
            )
            .ptr::<BuildStorage>();
        assert!(!storage.is_null());
        assert_eq!(pairs.borrow().len(), 2);
        assert_eq!(asked.borrow().clone(), vec!["menu.xml"]);
        let freed = calls_to(&e, XML_STORAGE_SCALAR_DELETING_DESTRUCTOR);
        assert_eq!(freed.len(), 1);
        assert_eq!(freed[0][1], 1);
        assert_eq!(calls_to(&e, MENU_GET_XML), vec![vec![path, 0]]);
    }

    #[test]
    fn parsing_a_missing_file_gives_an_empty_storage() {
        let mut e = tile_engine();
        install_xml_files(&mut e, vec![]);
        let pairs = install_reader_doubles(&mut e);
        e.call_log = Some(vec![]);
        let path = cstring(&mut e, "nothing.xml");
        let storage = e
            .call(
                0x00a0_1e20,
                &args![Ptr::<()>::new(path), Ptr::<()>::NULL, 0u32],
            )
            .ptr::<BuildStorage>();
        assert!(!storage.is_null());
        assert!(pairs.borrow().is_empty());
        assert!(calls_to(&e, XML_STORAGE_SCALAR_DELETING_DESTRUCTOR).is_empty());
    }

    /// Text and string doubles: `strstr` and `strtok_s` of the C library,
    /// and a small model of `BSStringT` (pointer and 16-bit length;
    /// `length_unknown` makes the length 0xffff, as a string whose length
    /// was never computed).
    fn install_string_doubles(e: &mut Engine, length_unknown: bool) {
        // The constants of the exe's data the include expansion uses.
        e.map(QUOTE_DELIMITER, 4);
        e.mem.set_cstr(QUOTE_DELIMITER, b"\"");
        e.mem.set_cstr(INCLUDE_TAG, b"<include");
        e.mem
            .set_cstr(PREFAB_PATH_FORMAT, br"Data\Menus\Prefabs\%s");
        e.register(STRSTR, |e, a| {
            let haystack = e.mem.cstr(a[0]);
            let needle = e.mem.cstr(a[1]);
            haystack
                .windows(needle.len())
                .position(|window| window == needle.as_slice())
                .map_or(0, |at| a[0] + at as u32)
                .into_ret()
        });
        e.register(STRTOK_S, |e, a| {
            let delimiters = e.mem.cstr(a[1]);
            let mut at = if a[0] != 0 { a[0] } else { e.mem.u32(a[2]) };
            while e.mem.u8(at) != 0 && delimiters.contains(&e.mem.u8(at)) {
                at += 1;
            }
            if e.mem.u8(at) == 0 {
                e.mem.set_u32(a[2], at);
                return 0u32.into_ret();
            }
            let start = at;
            while e.mem.u8(at) != 0 && !delimiters.contains(&e.mem.u8(at)) {
                at += 1;
            }
            if e.mem.u8(at) != 0 {
                e.mem.set_u8(at, 0);
                at += 1;
            }
            e.mem.set_u32(a[2], at);
            start.into_ret()
        });
        e.register_double(STRING_SET, move |e, a| {
            set_string(
                e,
                a[0],
                &if a[1] == 0 { vec![] } else { e.mem.cstr(a[1]) },
                length_unknown,
            );
            Ret::default()
        });
        e.register_double(STRING_APPEND, move |e, a| {
            if a[1] != 0 {
                let mut text = if e.mem.u32(a[0]) == 0 {
                    vec![]
                } else {
                    e.mem.cstr(e.mem.u32(a[0]))
                };
                text.extend(e.mem.cstr(a[1]));
                set_string(e, a[0], &text, length_unknown);
            }
            Ret::default()
        });
        e.register(STRING_FORMAT, |e, a| {
            assert_eq!(a[1], PREFAB_PATH_FORMAT);
            let mut text = b"Data\\Menus\\Prefabs\\".to_vec();
            text.extend(e.mem.cstr(a[2]));
            set_string(e, a[0], &text, false);
            Ret::default()
        });
    }

    /// Replaces the text of the model `BSStringT` at `string`.
    fn set_string(e: &mut Engine, string: u32, text: &[u8], length_unknown: bool) {
        let old = e.mem.u32(string);
        if old != 0 {
            e.mem.free(old);
        }
        if text.is_empty() {
            e.mem.set_u32(string, 0);
            e.mem.set_u16(string + 4, 0);
            return;
        }
        let block = e.mem.alloc(text.len() as u32 + 1);
        e.mem.set_cstr(block, text);
        e.mem.set_u32(string, block);
        let length = if length_unknown {
            0xffff
        } else {
            text.len() as u16
        };
        e.mem.set_u16(string + 4, length);
    }

    /// Runs `InsertIncludedFiles` over `text`; the text and size after, and
    /// the result.
    fn insert_includes(e: &mut Engine, text: &str) -> (bool, String, u32) {
        let xml = xml_storage(e, text);
        let result = e.call(0x00a0_2d40, &args![Ptr::<()>::new(xml)]).bool();
        (result, string_at(e, e.mem.u32(xml + 4)), e.mem.u32(xml))
    }

    #[test]
    fn includes_are_replaced_by_the_prefab_text() {
        for length_unknown in [false, true] {
            let mut e = tile_engine();
            install_string_doubles(&mut e, length_unknown);
            let asked = install_xml_files(&mut e, vec![("Data\\Menus\\Prefabs\\p.xml", "[X]")]);
            let (changed, text, size) = insert_includes(&mut e, "AA<include src=\"p.xml\"/>BB");
            assert!(changed);
            assert_eq!(text, "AA[X]BB");
            assert_eq!(size, 7);
            assert_eq!(asked.borrow().clone(), vec!["Data\\Menus\\Prefabs\\p.xml"]);
        }
    }

    #[test]
    fn several_includes_and_nested_includes_are_all_expanded() {
        let mut e = tile_engine();
        install_string_doubles(&mut e, false);
        install_xml_files(
            &mut e,
            vec![
                ("Data\\Menus\\Prefabs\\p", "1"),
                ("Data\\Menus\\Prefabs\\q", "2"),
                ("Data\\Menus\\Prefabs\\n", "[<include src=\"q\"/>]"),
            ],
        );
        let (changed, text, size) =
            insert_includes(&mut e, "A<include src=\"p\"/>B<include src=\"q\"/>C");
        assert!(changed);
        assert_eq!(text, "A1B2C");
        assert_eq!(size, 5);
        let (_, text, _) = insert_includes(&mut e, "A<include src=\"n\"/>B");
        assert_eq!(text, "A[2]B");
    }

    #[test]
    fn text_without_includes_is_left_alone() {
        let mut e = tile_engine();
        install_string_doubles(&mut e, false);
        let xml = xml_storage(&mut e, "<a/>");
        let before = e.mem.u32(xml + 4);
        assert!(!e.call(0x00a0_2d40, &args![Ptr::<()>::new(xml)]).bool());
        assert_eq!(e.mem.u32(xml + 4), before);
        assert_eq!(e.mem.u32(xml), 4);
    }

    #[test]
    fn included_files_have_their_prefab_storage_freed() {
        let mut e = tile_engine();
        install_string_doubles(&mut e, false);
        install_xml_files(&mut e, vec![("Data\\Menus\\Prefabs\\p", "1")]);
        e.call_log = Some(vec![]);
        insert_includes(&mut e, "<include src=\"p\"/>");
        // The prefab's storage object is destroyed in place and freed.
        let destroyed = calls_to(&e, XML_STORAGE_DESTROY);
        assert_eq!(destroyed.len(), 1);
        assert_eq!(e.mem.block_size(destroyed[0][0]), None);
    }

    /// Appends `element` to the `NiTPointerList` at `list`.
    fn push_list(e: &mut Engine, list: Ptr<NiTPointerList>, element: u32) {
        let node = e.mem.alloc(12);
        let tail = e.get(list, NiTPointerList::m_pkTail);
        e.mem.set_u32(node + 4, tail);
        e.mem.set_u32(node + 8, element);
        if tail == 0 {
            e.set(list, NiTPointerList::m_pkHead, node);
        } else {
            e.mem.set_u32(tail, node);
        }
        e.set(list, NiTPointerList::m_pkTail, node);
        let count = e.get(list, NiTPointerList::m_uiCount);
        e.set(list, NiTPointerList::m_uiCount, count + 1);
    }

    fn make_template(e: &mut Engine) -> Ptr<TileTemplate> {
        Ptr::new(e.mem.alloc(0x14))
    }

    /// Appends an item (`command`, number, text, argument) to a template.
    fn add_item(
        e: &mut Engine,
        template: Ptr<TileTemplate>,
        command: i32,
        value: f32,
        text: &str,
        argument: u32,
    ) -> Ptr<TileTemplateItem> {
        let item: Ptr<TileTemplateItem> = Ptr::new(e.mem.alloc(0x18));
        e.set(item, TileTemplateItem::iCmd, command);
        e.set(item, TileTemplateItem::fVal, value);
        if !text.is_empty() {
            let string = cstring(e, text);
            e.mem.set_u32(item.addr() + 8, string);
            e.mem.set_u16(item.addr() + 0xc, text.len() as u16);
        }
        e.set(item, TileTemplateItem::u, argument);
        push_list(e, template.at(TileTemplate::xList), item.addr());
        item
    }

    /// Records the calls to `addr` as (first word of the first argument's
    /// `eIndex`, second argument, third argument): the trait id of the
    /// value a setter was called on.
    fn record_value_calls(e: &mut Engine, addr: u32) -> Rc<RefCell<Vec<(i32, u32, u32)>>> {
        let calls = Rc::new(RefCell::new(vec![]));
        let record = calls.clone();
        e.register_double(addr, move |e, a| {
            record.borrow_mut().push((e.mem.i32(a[0]), a[1], a[2]));
            Ret::default()
        });
        calls
    }

    /// Records the (tile, flags) of every `AddNeedsUpdate`.
    fn record_updates(e: &mut Engine) -> Rc<RefCell<Vec<(u32, u32)>>> {
        let updates = Rc::new(RefCell::new(vec![]));
        let record = updates.clone();
        e.register_double(TILE_ADD_NEEDS_UPDATE, move |_, a| {
            record.borrow_mut().push((a[0], a[1]));
            Ret::default()
        });
        updates
    }

    #[test]
    fn building_the_tree_creates_names_and_nests_the_tiles() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        e.set_global(HOT_RECT_TYPE_VALUE, 906.0f64);
        e.set_global(MINUS_ONE, -1.0f32);
        e.set_global(TWO_FIFTY_FIVE, 255.0f32);
        e.set_global(DEFAULT_ATLAS_NAME, 0x7777u32);
        // `Init` (slot +4) of the rect and of the hot-rect (image) class.
        let inits = Rc::new(RefCell::new(vec![]));
        let record = inits.clone();
        e.register_double(0x0600_4000, move |e, a| {
            record.borrow_mut().push(a.to_vec());
            e.mem.set_u32(a[0] + 0x28, a[1]);
            Ret::default()
        });
        e.put_vtable(VTABLE_TILE_RECT, &[0, 0x0600_4000]);
        e.put_vtable(0x0600_4100, &[0, 0x0600_4000]);
        e.register(TILE_IMAGE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0x0600_4100);
            a[0].into_ret()
        });
        let floats = record_value_calls(&mut e, VALUE_SET_FLOAT);
        let strings = record_value_calls(&mut e, VALUE_SET_STRING);
        let updates = record_updates(&mut e);

        let top = typed_tile(&mut e, TYPE_RECT);
        let template = make_template(&mut e);
        let (root_name, hot_name, other_name) = (
            cstring(&mut e, "root"),
            cstring(&mut e, "hot"),
            cstring(&mut e, "other"),
        );
        let mut items = vec![];
        for (command, number, name) in [
            (TI_TILE_START, 901.0, root_name),
            (TI_TILE_START, 906.0, hot_name),
            (TI_TILE_END, 0.0, 0),
            (TI_TILE_START, 901.0, other_name),
            (TI_TILE_END, 0.0, 0),
            (TI_TILE_END, 0.0, 0),
        ] {
            let item = add_item(&mut e, template, command, number, "", 0);
            e.mem.set_u32(item.addr() + 8, name);
            items.push(item);
        }

        let first = e.call(0x00a0_31b0, &args![top, template]).ptr::<Tile>();

        let tile_of =
            |e: &Engine, index: usize| Ptr::<Tile>::new(e.get(items[index], TileTemplateItem::u));
        let (root, hot, other) = (tile_of(&e, 0), tile_of(&e, 1), tile_of(&e, 3));
        assert_eq!(first, root);
        assert!(!root.is_null() && !hot.is_null() && !other.is_null());
        // Each tile is initialized under its parent with its name.
        assert_eq!(
            inits.borrow().clone(),
            vec![
                vec![root.addr(), top.addr(), root_name, 0],
                vec![hot.addr(), root.addr(), hot_name, 0],
                vec![other.addr(), root.addr(), other_name, 0],
            ]
        );
        // The hot rect (type 906) gets its fixed look.
        let bits = |value: f32| value.to_bits();
        assert_eq!(
            floats.borrow().clone(),
            vec![
                (TRAIT_ID, bits(-1.0), 1),
                (TRAIT_TARGET, bits(1.0), 1),
                (TRAIT_BRIGHTNESS, bits(-1.0), 1),
                (TRAIT_RED, bits(255.0), 1),
                (TRAIT_GREEN, bits(255.0), 1),
                (TRAIT_BLUE, bits(255.0), 1),
                (TRAIT_ALPHA, bits(0.0), 1),
            ]
        );
        assert_eq!(
            strings.borrow().clone(),
            vec![
                (TRAIT_FILENAME, SOLID_TEXTURE_NAME, 1),
                (TRAIT_TEX_ATLAS, 0x7777, 1)
            ]
        );
        assert_eq!(updates.borrow().clone(), vec![(hot.addr(), UPDATE_TEXTURE)]);
    }

    #[test]
    fn building_the_tree_stops_when_a_tile_cannot_be_made() {
        let mut e = tile_engine();
        let top = typed_tile(&mut e, TYPE_RECT);
        let template = make_template(&mut e);
        // 907 is the window type, which `BuildTile` does not make.
        add_item(&mut e, template, TI_TILE_START, 907.0, "", 0);
        e.call_log = Some(vec![]);
        let result = e.call(0x00a0_31b0, &args![top, template]).u32();
        assert_eq!(result, 0);
        assert_eq!(
            calls_to(&e, PRINT_ERROR),
            vec![vec![MSG_UNABLE_TO_CREATE_TILE]]
        );
    }

    #[test]
    fn building_an_empty_template_gives_nothing() {
        let mut e = tile_engine();
        let top = typed_tile(&mut e, TYPE_RECT);
        let template = make_template(&mut e);
        assert_eq!(e.call(0x00a0_31b0, &args![top, template]).u32(), 0);
    }

    /// Items for `ConnectTraitsToTree` tests: a template whose first item
    /// starts `tile`.
    fn connect_template(e: &mut Engine, tile: Ptr<Tile>) -> Ptr<TileTemplate> {
        let template = make_template(e);
        add_item(e, template, TI_TILE_START, 0.0, "", tile.addr());
        template
    }

    fn connect(e: &mut Engine, template: Ptr<TileTemplate>) {
        let unused = typed_tile(e, TYPE_RECT);
        e.call(0x00a0_33b0, &args![unused, template]);
    }

    #[test]
    fn connecting_simple_traits_sets_floats_and_strings() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        e.set_global(NO_TRAIT_VALUE, -2147483648.0f64);
        let tile = typed_tile(&mut e, TYPE_RECT);
        let template = connect_template(&mut e, tile);
        let none = -2147483648.0f32;
        add_item(&mut e, template, TI_SIMPLE_TRAIT, 5.0, "", TRAIT_X as u32);
        // No number but a text: a string, whose length may be unknown.
        let with_text = add_item(
            &mut e,
            template,
            TI_SIMPLE_TRAIT,
            none,
            "w",
            TRAIT_WIDTH as u32,
        );
        e.mem.set_u16(with_text.addr() + 0xc, 0xffff);
        // No number and no text: the number itself is set.
        add_item(
            &mut e,
            template,
            TI_SIMPLE_TRAIT,
            none,
            "",
            TRAIT_HEIGHT as u32,
        );
        // String traits are strings whatever the number.
        add_item(
            &mut e,
            template,
            TI_SIMPLE_TRAIT,
            1.0,
            "s",
            TRAIT_STRING as u32,
        );
        add_item(
            &mut e,
            template,
            TI_SIMPLE_TRAIT,
            1.0,
            "f.dds",
            TRAIT_FILENAME as u32,
        );
        let name = add_item(
            &mut e,
            template,
            TI_SIMPLE_TRAIT,
            none,
            "nm",
            TAG_NAME as u32,
        );
        let floats = record_value_calls(&mut e, VALUE_SET_FLOAT);
        let strings = record_value_calls(&mut e, VALUE_SET_STRING);
        let updates = record_updates(&mut e);
        e.call_log = Some(vec![]);

        connect(&mut e, template);

        assert_eq!(
            floats.borrow().clone(),
            vec![
                (TRAIT_X, 5.0f32.to_bits(), 1),
                (TRAIT_HEIGHT, none.to_bits(), 1)
            ]
        );
        let text_of = |e: &Engine, item: usize| {
            let items = e.get(template.at(TileTemplate::xList), NiTPointerList::m_pkHead);
            let mut node = items;
            for _ in 0..item {
                node = e.mem.u32(node);
            }
            e.mem.u32(e.mem.u32(node + 8) + 8)
        };
        assert_eq!(
            strings.borrow().clone(),
            vec![
                (TRAIT_WIDTH, text_of(&e, 2), 1),
                (TRAIT_STRING, text_of(&e, 4), 1),
                (TRAIT_FILENAME, text_of(&e, 5), 1),
            ]
        );
        // The name tag becomes the tile's name.
        assert_eq!(
            calls_to(&e, STRING_ASSIGN),
            vec![vec![tile.addr() + 0x20, name.addr() + 8]]
        );
        // A rect tile's file name does not need a texture update.
        assert_eq!(
            updates.borrow().clone(),
            vec![(tile.addr(), FLAG_TILE_LOADING)]
        );
    }

    #[test]
    fn connecting_an_image_file_name_queues_a_texture_update() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        e.set_global(NO_TRAIT_VALUE, -2147483648.0f64);
        let tile = typed_tile(&mut e, TYPE_IMAGE);
        let template = connect_template(&mut e, tile);
        add_item(
            &mut e,
            template,
            TI_SIMPLE_TRAIT,
            0.0,
            "a.dds",
            TRAIT_FILENAME as u32,
        );
        let strings = record_value_calls(&mut e, VALUE_SET_STRING);
        let updates = record_updates(&mut e);
        connect(&mut e, template);
        assert_eq!(strings.borrow().len(), 1);
        assert_eq!(
            updates.borrow().clone(),
            vec![
                (tile.addr(), FLAG_TILE_LOADING),
                (tile.addr(), UPDATE_TEXTURE)
            ]
        );
    }

    #[test]
    fn connecting_actions_adds_them_to_the_current_trait() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        let template = connect_template(&mut e, tile);
        add_item(&mut e, template, TI_TRAIT_START, TRAIT_X as f32, "", 0);
        add_item(&mut e, template, TI_SIMPLE_ACTION, 2.0, "", 0x7d1);
        add_item(&mut e, template, TI_TRAIT_LINK, 2.5, "link", 0x7d2);
        add_item(&mut e, template, TI_ACTION_START, 2.6, "", 0);
        add_item(&mut e, template, TI_ACTION_END, 3.5, "", 0);
        add_item(&mut e, template, TI_TRAIT_END, 0.0, "", 0);
        // Out of any trait: a message.
        add_item(&mut e, template, TI_SIMPLE_ACTION, 2.0, "", 0x7d1);
        e.register(TILE_FIND_BY_PATH, |e, a| {
            assert_eq!(e.mem.cstr(a[1]), b"link");
            0x5555u32.into_ret()
        });
        let actions = record_value_calls(&mut e, VALUE_ADD_ACTION);
        let links = Rc::new(RefCell::new(vec![]));
        let record = links.clone();
        e.register_double(VALUE_ADD_ACTION_OV2, move |e, a| {
            record
                .borrow_mut()
                .push((e.mem.i32(a[0]), a[1], a[2], a[3]));
            Ret::default()
        });
        e.call_log = Some(vec![]);

        connect(&mut e, template);

        let bits = |value: f32| value.to_bits();
        assert_eq!(
            actions.borrow().clone(),
            vec![
                (TRAIT_X, 0x7d1, bits(2.0)),
                // 2.6 rounds to 3, 3.5 to 4 (ties to even).
                (TRAIT_X, ACTION_LEFT_PAREN as u32, bits(3.0)),
                (TRAIT_X, ACTION_RIGHT_PAREN as u32, bits(4.0)),
            ]
        );
        // 2.5 rounds to 2 (ties to even).
        assert_eq!(links.borrow().clone(), vec![(TRAIT_X, 0x7d2, 0x5555, 2)]);
        assert_eq!(
            calls_to(&e, PRINT_ERROR),
            vec![vec![MSG_ACTION_OUTSIDE_TRAIT]]
        );
    }

    #[test]
    fn an_action_link_to_nothing_adds_nothing() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, TYPE_RECT);
        let template = connect_template(&mut e, tile);
        add_item(&mut e, template, TI_TRAIT_START, TRAIT_X as f32, "", 0);
        add_item(&mut e, template, TI_TRAIT_LINK, 1.0, "gone", 0x7d2);
        e.call_log = Some(vec![]);
        connect(&mut e, template);
        assert!(calls_to(&e, VALUE_ADD_ACTION_OV2).is_empty());
    }

    #[test]
    fn items_out_of_place_print_messages() {
        let mut e = tile_engine();
        let template = make_template(&mut e);
        add_item(&mut e, template, TI_SIMPLE_TRAIT, 1.0, "", TRAIT_X as u32);
        add_item(&mut e, template, TI_TRAIT_LINK, 1.0, "l", 0x7d2);
        add_item(&mut e, template, TI_ACTION_START, 1.0, "", 0);
        add_item(&mut e, template, TI_ACTION_END, 1.0, "", 0);
        e.call_log = Some(vec![]);
        connect(&mut e, template);
        assert_eq!(
            calls_to(&e, PRINT_ERROR),
            vec![
                vec![MSG_TRAIT_OUTSIDE_TILE],
                vec![MSG_ACTION_LINK_OUTSIDE_TRAIT],
                vec![MSG_ACTION_BEGUN_OUTSIDE_TRAIT],
                vec![MSG_ACTION_ENDED_OUTSIDE_TRAIT],
            ]
        );
    }

    #[test]
    fn ending_a_tile_calculates_its_values_and_returns_to_the_parent() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let parent = typed_tile(&mut e, TYPE_RECT);
        let child = typed_tile(&mut e, TYPE_RECT);
        e.set(child, Tile::pParent, parent);
        let x = give_trait(&mut e, child, TRAIT_X, 1.0);
        let class = give_trait(&mut e, child, TRAIT_CLASS, 2.0);
        let _ = class;
        let y = give_trait(&mut e, child, TRAIT_Y, 3.0);
        e.set(child, Tile::uiFlags, FLAG_TILE_LOADING | UPDATE_COLOR);
        let template = make_template(&mut e);
        add_item(&mut e, template, TI_TILE_START, 0.0, "", parent.addr());
        add_item(&mut e, template, TI_TILE_START, 0.0, "", child.addr());
        add_item(&mut e, template, TI_TILE_END, 0.0, "", 0);
        // Back on the parent tile.
        add_item(
            &mut e,
            template,
            TI_SIMPLE_TRAIT,
            4.0,
            "",
            TRAIT_WIDTH as u32,
        );
        let floats = record_value_calls(&mut e, VALUE_SET_FLOAT);
        e.call_log = Some(vec![]);

        connect(&mut e, template);

        // The loading flag is cleared, the other bits stay.
        assert_eq!(e.get(child, Tile::uiFlags), UPDATE_COLOR);
        // Every value but the class is calculated (with the flag 1).
        assert_eq!(
            calls_to(&e, VALUE_CALCULATE_VALUE),
            vec![vec![x.addr(), 1], vec![y.addr(), 1]]
        );
        assert_eq!(
            calls_to(&e, ENTER_CRITICAL_SECTION).len(),
            calls_to(&e, LEAVE_CRITICAL_SECTION).len()
        );
        // The width went to the parent tile.
        let width = e.mem.u32(e.mem.u32(parent.addr() + 0x14));
        assert_eq!(e.mem.i32(width), TRAIT_WIDTH);
        assert_eq!(floats.borrow().len(), 1);
        assert_eq!(e.get(child, Tile::pParent), parent);
    }

    /// A `Menu` object, and the log of what is queued for tiles.
    fn menu_with_virtual_slot(e: &mut Engine) -> (Ptr, Rc<RefCell<Vec<Vec<u32>>>>) {
        let menu = Ptr::new(e.mem.alloc(0x28));
        e.put_vtable(0x0600_5000, &[0, 0x0600_5100]);
        e.mem.set_u32(menu.addr(), 0x0600_5000);
        let calls = Rc::new(RefCell::new(vec![]));
        let record = calls.clone();
        e.register_double(0x0600_5100, move |_, a| {
            record.borrow_mut().push(a.to_vec());
            Ret::default()
        });
        (menu, calls)
    }

    /// What `FinalPostParse` queues for `trait_id` on a tile of `kind`,
    /// and whether it answers the tile.
    fn post_parse(
        kind: u32,
        trait_id: i32,
        setup: impl FnOnce(&mut Engine, Ptr<Tile>),
    ) -> (Vec<u32>, bool) {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let tile = typed_tile(&mut e, kind);
        setup(&mut e, tile);
        let updates = record_updates(&mut e);
        let result = e
            .call(0x00a0_37e0, &args![tile, trait_id, 1.0f32, 0u32])
            .u32();
        let flags = updates.borrow().iter().map(|(_, flags)| *flags).collect();
        (flags, result == tile.addr())
    }

    fn no_setup(_: &mut Engine, _: Ptr<Tile>) {}

    fn with_clip_window(e: &mut Engine, tile: Ptr<Tile>) {
        give_trait(e, tile, TRAIT_CLIP_WINDOW, 1.0);
    }

    #[test]
    fn post_parse_of_a_position_trait_queues_a_position_update() {
        for trait_id in [TRAIT_X, TRAIT_Y] {
            assert_eq!(
                post_parse(TYPE_RECT, trait_id, no_setup),
                (vec![UPDATE_POSITION], true)
            );
            assert_eq!(
                post_parse(TYPE_RECT, trait_id, with_clip_window),
                (vec![UPDATE_SCISSOR_WINDOW, UPDATE_POSITION], true)
            );
        }
    }

    #[test]
    fn post_parse_of_the_depth_raises_the_menus_depth() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        // root <- menu tile <- tile; the tile has a locus and a depth of 2.
        let root = typed_tile(&mut e, TYPE_RECT);
        let (menu_tile, menu) = make_menu_tile(&mut e, 1);
        let tile = typed_tile(&mut e, TYPE_RECT);
        add_child(&mut e, root, menu_tile);
        add_child(&mut e, menu_tile, tile);
        give_trait(&mut e, tile, TRAIT_LOCUS, 1.0);
        give_trait(&mut e, tile, TRAIT_DEPTH, 2.0);
        let updates = record_updates(&mut e);
        // 3.0 plus the tile's own 2.0 is more than the menu's 1.
        let result = e.call(0x00a0_37e0, &args![tile, TRAIT_DEPTH, 3.0f32, 0u32]);
        assert_eq!(result.u32(), tile.addr());
        assert_eq!(e.mem.i32(menu.addr() + 0x18), 5);
        assert_eq!(
            updates
                .borrow()
                .iter()
                .map(|(_, flags)| *flags)
                .collect::<Vec<_>>(),
            vec![UPDATE_POSITION]
        );
        // A smaller total leaves the menu's depth alone.
        e.mem.set_i32(menu.addr() + 0x18, 9);
        e.call(0x00a0_37e0, &args![tile, TRAIT_DEPTH, 3.0f32, 0u32]);
        assert_eq!(e.mem.i32(menu.addr() + 0x18), 9);
        // The menu tile itself never changes its own menu's depth.
        e.mem.set_i32(menu.addr() + 0x18, 0);
        give_trait(&mut e, menu_tile, TRAIT_DEPTH, 1.0);
        e.call(0x00a0_37e0, &args![menu_tile, TRAIT_DEPTH, 6.0f32, 0u32]);
        assert_eq!(e.mem.i32(menu.addr() + 0x18), 0);
    }

    #[test]
    fn post_parse_of_text_traits_queues_a_create_update() {
        for trait_id in [
            TRAIT_WRAP_WIDTH,
            TRAIT_WRAP_LIMIT,
            TRAIT_WRAP_LINES,
            TRAIT_LINE_GAP,
            TRAIT_PAGE_NUM,
            TRAIT_IS_HTML,
            TRAIT_FONT,
            TRAIT_JUSTIFY,
        ] {
            assert_eq!(
                post_parse(TYPE_TEXT, trait_id, no_setup),
                (vec![UPDATE_CREATE], true),
                "{trait_id:#x}"
            );
            // Not for other kinds of tile.
            assert_eq!(post_parse(TYPE_IMAGE, trait_id, no_setup), (vec![], false));
        }
    }

    #[test]
    fn post_parse_of_the_size_of_an_image_queues_a_geometry_update() {
        for trait_id in [TRAIT_WIDTH, TRAIT_HEIGHT, TRAIT_CROP_X, TRAIT_CROP_Y] {
            assert_eq!(
                post_parse(TYPE_IMAGE, trait_id, no_setup),
                (vec![UPDATE_GEOMETRY], true)
            );
            assert_eq!(
                post_parse(TYPE_IMAGE, trait_id, with_clip_window),
                (vec![UPDATE_SCISSOR_WINDOW, UPDATE_GEOMETRY], true)
            );
            assert_eq!(post_parse(TYPE_TEXT, trait_id, no_setup), (vec![], false));
        }
    }

    #[test]
    fn post_parse_of_the_clip_window() {
        assert_eq!(
            post_parse(TYPE_IMAGE, TRAIT_CLIP_WINDOW, no_setup),
            (vec![UPDATE_SCISSOR_WINDOW], true)
        );
        assert_eq!(
            post_parse(TYPE_RECT, TRAIT_CLIP_WINDOW, no_setup),
            (vec![UPDATE_SCISSOR_WINDOW], true)
        );
        assert_eq!(
            post_parse(TYPE_TEXT, TRAIT_CLIP_WINDOW, no_setup),
            (vec![], false)
        );
    }

    #[test]
    fn post_parse_of_clips_passes_the_value_to_the_children() {
        assert_eq!(
            post_parse(TYPE_RECT, TRAIT_CLIPS, no_setup),
            (vec![], false)
        );
        for kind in [TYPE_IMAGE, TYPE_TEXT] {
            let mut e = tile_engine();
            install_value_array_doubles(&mut e);
            let tile = typed_tile(&mut e, kind);
            let (first, second) = (typed_tile(&mut e, TYPE_RECT), typed_tile(&mut e, TYPE_RECT));
            add_child(&mut e, tile, first);
            add_child(&mut e, tile, second);
            give_trait(&mut e, tile, TRAIT_CLIPS, 1.0);
            let updates = record_updates(&mut e);
            let floats = record_value_calls(&mut e, VALUE_SET_FLOAT);
            let result = e.call(0x00a0_37e0, &args![tile, TRAIT_CLIPS, 1.0f32, 0u32]);
            assert_eq!(result.u32(), tile.addr());
            assert_eq!(
                updates.borrow().clone(),
                vec![(tile.addr(), UPDATE_SCISSOR)]
            );
            // Each child's clips trait is set to the tile's.
            assert_eq!(
                floats.borrow().clone(),
                vec![
                    (TRAIT_CLIPS, 1.0f32.to_bits(), 1),
                    (TRAIT_CLIPS, 1.0f32.to_bits(), 1)
                ]
            );
        }
    }

    #[test]
    fn post_parse_of_visibility_color_and_locus() {
        assert_eq!(
            post_parse(TYPE_RECT, TRAIT_VISIBLE, no_setup),
            (vec![UPDATE_VISIBILITY], true)
        );
        for trait_id in [
            TRAIT_ALPHA,
            TRAIT_RED,
            TRAIT_GREEN,
            TRAIT_BLUE,
            TRAIT_SYSTEM_COLOR,
            TRAIT_BRIGHTNESS,
        ] {
            assert_eq!(
                post_parse(TYPE_IMAGE, trait_id, no_setup),
                (vec![UPDATE_COLOR], true)
            );
            assert_eq!(
                post_parse(TYPE_TEXT, trait_id, no_setup),
                (vec![UPDATE_COLOR], true)
            );
            // A tile of another kind has no colour to update, but the trait
            // is still handled.
            assert_eq!(post_parse(TYPE_RECT, trait_id, no_setup), (vec![], true));
        }
        assert_eq!(
            post_parse(TYPE_RECT, TRAIT_LOCUS, no_setup),
            (vec![UPDATE_LOCUS], true)
        );
    }

    #[test]
    fn post_parse_of_a_file_name_needs_a_file() {
        let with_file = |e: &mut Engine, tile: Ptr<Tile>| {
            let value = give_trait(e, tile, TRAIT_FILENAME, 0.0);
            e.set(value, TileValue::strValue, Ptr::new(0x7000));
        };
        assert_eq!(
            post_parse(TYPE_IMAGE, TRAIT_FILENAME, with_file),
            (vec![UPDATE_TEXTURE], true)
        );
        assert_eq!(
            post_parse(TYPE_3D, TRAIT_FILENAME, with_file),
            (vec![UPDATE_NIF_FILE], true)
        );
        // Without a file, or on a tile that shows none: nothing.
        assert_eq!(
            post_parse(TYPE_IMAGE, TRAIT_FILENAME, no_setup),
            (vec![], false)
        );
        assert_eq!(
            post_parse(TYPE_3D, TRAIT_FILENAME, no_setup),
            (vec![], false)
        );
        assert_eq!(
            post_parse(TYPE_RECT, TRAIT_FILENAME, with_file),
            (vec![], false)
        );
    }

    #[test]
    fn post_parse_of_rotation_and_unknown_traits() {
        for trait_id in [TRAIT_ROTATE_ANGLE, TRAIT_ROTATE_AXIS_X, TRAIT_ROTATE_AXIS_Y] {
            assert_eq!(
                post_parse(TYPE_RECT, trait_id, no_setup),
                (vec![UPDATE_POSITION], true)
            );
        }
        assert_eq!(post_parse(TYPE_RECT, 0xfa5, no_setup), (vec![], false));
        assert_eq!(post_parse(TYPE_RECT, TRAIT_NONE, no_setup), (vec![], false));
    }

    #[test]
    fn post_parse_of_the_id_tells_the_menu() {
        let mut e = tile_engine();
        install_value_array_doubles(&mut e);
        let (menu_tile, menu) = make_menu_tile(&mut e, 0);
        let (hidden, calls) = menu_with_virtual_slot(&mut e);
        // The menu object of the menu tile answers a virtual call.
        e.mem.set_u32(menu_tile.addr() + 0x3c, hidden.addr());
        let _ = menu;
        let updates = record_updates(&mut e);
        let result = e.call(0x00a0_37e0, &args![menu_tile, TRAIT_ID, 42.9f32, 0u32]);
        // The id is truncated; the menu is called with it and the tile; the
        // trait itself queues nothing and is not answered.
        assert_eq!(
            calls.borrow().clone(),
            vec![vec![hidden.addr(), 42, menu_tile.addr()]]
        );
        assert_eq!(result.u32(), 0);
        assert!(updates.borrow().is_empty());
    }

    #[test]
    fn the_menu_tile_is_the_child_of_the_root() {
        let mut e = tile_engine();
        let root = typed_tile(&mut e, TYPE_RECT);
        let (a, b, c) = (
            typed_tile(&mut e, TYPE_MENU),
            typed_tile(&mut e, TYPE_RECT),
            typed_tile(&mut e, TYPE_RECT),
        );
        add_child(&mut e, root, a);
        add_child(&mut e, a, b);
        add_child(&mut e, b, c);
        e.call_log = Some(vec![]);
        for (from, expected) in [(c, a), (b, a), (a, a), (root, root)] {
            assert_eq!(e.call(0x00a0_3d40, &args![from]).u32(), expected.addr());
        }
        assert_eq!(e.call(0x00a0_3d40, &args![Ptr::<Tile>::NULL]).u32(), 0);
        assert_lock_balanced(&e);
    }

    #[test]
    fn the_menu_comes_from_a_menu_type_menu_tile() {
        let mut e = tile_engine();
        let root = typed_tile(&mut e, TYPE_RECT);
        let (menu_tile, menu) = make_menu_tile(&mut e, 0);
        let leaf = typed_tile(&mut e, TYPE_RECT);
        add_child(&mut e, root, menu_tile);
        add_child(&mut e, menu_tile, leaf);
        e.mem.set_u8(menu.addr() + 0x1c, 1);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x00a0_3c90, &args![leaf]).u32(), menu.addr());
        assert_eq!(e.call(0x00a0_3c90, &args![menu_tile]).u32(), menu.addr());
        assert_lock_balanced(&e);
        assert!(e.call(0x00a0_3c60, &args![leaf]).bool());
        e.mem.set_u8(menu.addr() + 0x1c, 0);
        assert!(!e.call(0x00a0_3c60, &args![leaf]).bool());

        // A menu tile of another type has no menu.
        let other_root = typed_tile(&mut e, TYPE_RECT);
        let other = typed_tile(&mut e, TYPE_IMAGE);
        add_child(&mut e, other_root, other);
        assert_eq!(e.call(0x00a0_3c90, &args![other]).u32(), 0);
        assert_eq!(e.call(0x00a0_3c90, &args![Ptr::<Tile>::NULL]).u32(), 0);
    }

    #[test]
    fn child_names_are_searched_depth_first() {
        let mut e = tile_engine();
        e.register(STRCMP, |e, a| {
            let (first, second) = (e.mem.cstr(a[0]), e.mem.cstr(a[1]));
            (first.cmp(&second) as i32).into_ret()
        });
        let root = typed_tile(&mut e, TYPE_RECT);
        let names = ["a", "target", "b", "target", "c"];
        let tiles: Vec<Ptr<Tile>> = names
            .iter()
            .map(|name| {
                let tile = typed_tile(&mut e, TYPE_RECT);
                let string = cstring(&mut e, name);
                e.mem.set_u32(tile.addr() + 0x20, string);
                tile
            })
            .collect();
        // root -> a -> [target (inner)], root -> b -> [target (second)],
        // and a nameless tile, and "c" last.
        add_child(&mut e, root, tiles[0]);
        add_child(&mut e, tiles[0], tiles[1]);
        let nameless = typed_tile(&mut e, TYPE_RECT);
        add_child(&mut e, root, nameless);
        add_child(&mut e, root, tiles[2]);
        add_child(&mut e, tiles[2], tiles[3]);
        add_child(&mut e, root, tiles[4]);
        e.call_log = Some(vec![]);
        let target = cstring(&mut e, "target");
        let find = |e: &mut Engine, name: u32| {
            e.call(0x00a0_3da0, &args![root, Ptr::<()>::new(name)])
                .u32()
        };
        // Depth first: the inner tile under "a" comes before the one under "b".
        assert_eq!(find(&mut e, target), tiles[1].addr());
        let c = cstring(&mut e, "c");
        assert_eq!(find(&mut e, c), tiles[4].addr());
        let b = cstring(&mut e, "b");
        assert_eq!(find(&mut e, b), tiles[2].addr());
        let missing = cstring(&mut e, "missing");
        assert_eq!(find(&mut e, missing), 0);
        assert_eq!(find(&mut e, 0), 0);
        assert_lock_balanced(&e);
    }

    /// The tiles `ReadFile` works on: a parent tile and a template with one
    /// tile and a sub-template, where the tile's class is a menu.
    struct ReadSetup {
        parent: Ptr<Tile>,
        path: u32,
        menu: u32,
        storage: Rc<Cell<u32>>,
    }

    fn read_setup(e: &mut Engine, xml_text: &'static str) -> ReadSetup {
        install_value_array_doubles(e);
        install_xml_files(e, vec![("menu.xml", xml_text)]);
        install_reader_doubles(e);
        let parent = typed_tile(e, TYPE_RECT);
        let menu = e.mem.alloc(0x28);
        // The rect constructor's tile is a menu-type tile with that menu.
        e.put_vtable(VTABLE_TILE_RECT, &[0, 0x0600_6000, 0, 0x0600_6100]);
        e.register(0x0600_6000, |_, _| Ret::default());
        e.register(0x0600_6100, |_, _| TYPE_MENU.into_ret());
        e.register_double(TILE_BASE_CONSTRUCT, move |e, a| {
            e.mem.set_u32(a[0] + 0x3c, menu);
            a[0].into_ret()
        });
        // The storage: its template holds one tile start and end; one
        // sub-template is registered.
        let storage = Rc::new(Cell::new(0));
        let made = storage.clone();
        e.register_double(BUILD_STORAGE_CONSTRUCT, move |e, a| {
            let template = make_template(e);
            add_item(e, template, TI_TILE_START, 901.0, "", 0);
            add_item(e, template, TI_TILE_END, 0.0, "", 0);
            e.mem.set_u32(a[0], template.addr());
            e.mem.set_u32(a[0] + 4, 0xcafe);
            made.set(a[0]);
            a[0].into_ret()
        });
        let path = cstring(e, "menu.xml");
        ReadSetup {
            parent,
            path,
            menu,
            storage,
        }
    }

    #[test]
    fn reading_a_file_builds_the_tree_and_updates_it() {
        let mut e = tile_engine();
        let setup = read_setup(&mut e, "");
        e.set_global(INTERFACE_LOCK_OWNER, 0x5000_0000u32);
        let line_word = e.tls() + TLS_CURRENT_LINE;
        e.mem.set_u32(line_word, 0x77);
        e.set_global(XML_ERROR_FLAG, 1u8);
        e.call_log = Some(vec![]);

        let tile = e
            .call(
                0x00a0_1b00,
                &args![setup.parent, Ptr::<()>::new(setup.path)],
            )
            .ptr::<Tile>();

        assert!(!tile.is_null());
        assert_eq!(e.mem.u32(tile.addr() + 0x3c), setup.menu);
        // The sub-template is registered with the menu, which is marked as
        // deleting its templates.
        assert_eq!(
            calls_to(&e, MENU_ADD_TEMPLATE),
            vec![vec![setup.menu, 0xcafe]]
        );
        assert_eq!(e.mem.u8(setup.menu + 0x1d), 1);
        // The parse storage is destroyed and freed.
        assert_eq!(
            calls_to(&e, BUILD_STORAGE_DESTROY),
            vec![vec![setup.storage.get()]]
        );
        assert_eq!(e.mem.block_size(setup.storage.get()), None);
        // The new tree is updated under the interface lock.
        assert_eq!(
            calls_to(&e, TILE_UPDATE_CHILDREN),
            vec![vec![tile.addr(), 0]]
        );
        let log = e.call_log.as_ref().unwrap();
        let position = |addr: u32, args: Vec<u32>| {
            log.iter()
                .position(|(called, logged)| *called == addr && *logged == args)
                .unwrap()
        };
        let update = position(TILE_UPDATE_CHILDREN, vec![tile.addr(), 0]);
        assert!(position(ENTER_CRITICAL_SECTION, vec![0x5000_0080]) < update);
        assert!(update < position(LEAVE_CRITICAL_SECTION, vec![0x5000_0080]));
        // The error flag was cleared before and after; the line word is back.
        assert_eq!(e.global::<u8>(XML_ERROR_FLAG), 0);
        assert_eq!(e.mem.u32(line_word), 0x77);
    }

    #[test]
    fn reading_a_broken_file_gives_nothing() {
        let mut e = tile_engine();
        let setup = read_setup(&mut e, "<>");
        let line_word = e.tls() + TLS_CURRENT_LINE;
        e.mem.set_u32(line_word, 0x77);
        e.call_log = Some(vec![]);
        let tile = e.call(
            0x00a0_1b00,
            &args![setup.parent, Ptr::<()>::new(setup.path)],
        );
        assert_eq!(tile.u32(), 0);
        assert_eq!(e.global::<u8>(XML_ERROR_FLAG), 1);
        assert_eq!(e.mem.u32(line_word), 0x77);
        assert!(calls_to(&e, TILE_UPDATE_CHILDREN).is_empty());
        assert!(calls_to(&e, BUILD_STORAGE_DESTROY).is_empty());
    }

    #[test]
    fn reading_stops_when_a_template_cannot_be_registered() {
        let mut e = tile_engine();
        let setup = read_setup(&mut e, "");
        let line_word = e.tls() + TLS_CURRENT_LINE;
        e.mem.set_u32(line_word, 0x77);
        e.register(MENU_ADD_TEMPLATE, |e, _| {
            e.set_global(XML_ERROR_FLAG, 1u8);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        let tile = e.call(
            0x00a0_1b00,
            &args![setup.parent, Ptr::<()>::new(setup.path)],
        );
        assert_eq!(tile.u32(), 0);
        assert_eq!(e.mem.u32(line_word), 0x77);
        assert!(calls_to(&e, TILE_UPDATE_CHILDREN).is_empty());
        // The storage is left as it was.
        assert!(calls_to(&e, BUILD_STORAGE_DESTROY).is_empty());
    }

    #[test]
    fn float_to_int_rounded_rounds_to_even_and_flags_overflow() {
        assert_eq!(float_to_int_rounded(2.5), 2);
        assert_eq!(float_to_int_rounded(3.5), 4);
        assert_eq!(float_to_int_rounded(-2.5), -2);
        assert_eq!(float_to_int_rounded(-3.5), -4);
        assert_eq!(float_to_int_rounded(2.6), 3);
        assert_eq!(float_to_int_rounded(-0.4), 0);
        assert_eq!(float_to_int_rounded(f32::NAN), i32::MIN);
        assert_eq!(float_to_int_rounded(3.0e9), i32::MIN);
        assert_eq!(float_to_int_rounded(-3.0e9), i32::MIN);
    }
}
