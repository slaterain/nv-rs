//! `fallout shared/tesdatahandler.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `TESDataHandler` is the game's form database: the singleton at
//! `0x011c3f2c` (0x63C bytes on both builds) owns one `BSSimpleList` per kind
//! of form (`listScripts`, `listFactions`, ...), the loaded `TESFile`s and the
//! interior cell and add-on node arrays, and it loads and saves forms for
//! the plugin files.
//!
//! Session 1 of this unit (the first 40 functions in address order,
//! `0040fbe0` to `00461250`) holds the constructor and destructor
//! (`0045d270`, `0045d970`), `ClearData` (`0045dfe0`), `LoadForm`, `SaveForm`,
//! `AddFormToDataHandler` and the many one-line helpers the compiler
//! emitted next to them: the address of one of the form lists
//! (`this + offset`, which the linker folded with identical accessors of
//! other classes, so some of the addresses belong to other units), the
//! flag getters and the scalar deleting destructors of the objects the
//! handler owns. Session 1 stops before `00461270`.
//!
//! Session 2 (the next 40 functions, `00461270` to `00462ec0`) holds
//! `NewCell`, the lookups by form id and editor id (`GetSound`, `GetFaction`,
//! `GetGlobal`, `GetCellByEditorID`, ...), the add-on node array
//! (`GetAddonNode`, `AddAddonNode`), the interior cell array helpers,
//! `GetCellFromWorldCoord`, `GetExtCellDataFromFileByEditorID`,
//! `UnloadCell`, `004624b0` (the scan of the data directory for plugin
//! files, whose Windows file-find imports are called by their import
//! slots) and the small `TESFile`/stream helpers the compiler emitted next to
//! them. The next session continues at `00462ee0` (`CreateThreadSafeFiles`).
//!
//! Notes for session 2:
//! - The virtual at `0x130` of a `TESForm` is the editor id getter and takes
//!   no argument: the decompiler hangs the following call's pushed argument on
//!   it (`GetGlobal`, `GetCellByEditorID`, `AddAddonNode`). `0x134` sets the
//!   editor id.
//! - `0045dfc0`'s list is `listFiles`; a `TESFile` is 0x42C bytes and its
//!   offsets used here are the Xbox PDB fields (`m_Filename` `+0x20`,
//!   `m_Path` `+0x124`, `m_currentform` `+0x240`, `m_FileInfo` `+0x29c`,
//!   `cSummary` `+0x418`, `bCached` `+0x428`), read at their offsets since
//!   `tesfile.cpp` is another unit.
//! - The tag words of the form stream (`0x01187020`, `0x01187314`,
//!   `0x011872b4`) are initialised in the exe's data; the code compares them
//!   and the translation reads them from memory, so tests set them.
//!
//! Notes for the next session:
//! - A `BSSimpleList<T>` is 8 bytes: the head node (item at +0, next at +4)
//!   lives in the owner. `006815c0(list)` returns the address of the head
//!   node's item slot, `0063f7b0(list)` removes the head node, `00726070`
//!   steps to the next node and `005ae3d0(list, &item)` adds an item.
//! - Functions of this same unit that come later in the queue and are called
//!   by address from the ones here: `00464e50` (`CleanUpBadForms`) and
//!   `00464f30` (returns its argument); `00461820` (`AddAddonNode`) and
//!   `00461bc0` (`GetCellFromWorldCoord`) are translated in session 2. The
//!   engine map assigns no unit
//!   to the other list accessors `AddFormToDataHandler` reaches
//!   (`00460fb0`, `00461070`, `004610d0`, `004610f0`, `00461110`,
//!   `00461130`, `00461150`, `00461190`) nor to `004612b0` (the name of a
//!   form type); they are called by address too.
//! - Type numbers: `AddFormToDataHandler` dispatches on the form type byte
//!   (`TESForm +4`); its table is `LOCAL_LIST_TYPES`/`EXTERNAL_LIST_TYPES`
//!   in the tests (form type to list).
//! - The calling convention of the leak report in `ClearData` pushes a
//!   stray 0 before two of its calls (`TESForm::GetFile` and the virtual at
//!   `0x130`); the callees do not read it (the `ADD ESP, 8` after the
//!   identity call `00464f30` removes it), so it is not passed.
//! - The compiler's exception-unwinding frames (`FS:[0]` chains and the
//!   `__CxxFrameHandler` state words) of the constructor and destructor are
//!   not translated.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleList, NiTArray};

/// `operator new(size)` (cdecl, one stack argument).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete(pointer)` (cdecl, one stack argument).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `memset(destination, value, size)` (cdecl wrapper).
const MEMSET: u32 = 0x0040_3d30;
/// `__RTDynamicCast(object, vfDelta, sourceType, targetType, isReference)`.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;

// Callees on the form lists, which are `BSSimpleList`s: the head node is in
// the owner, so `this` of these calls is the address of the list.
/// `BSSimpleList::BSSimpleList`.
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// The list destructor body.
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
/// The address of the head node's item slot.
const LIST_HEAD_ITEM: u32 = 0x0068_15c0;
/// Removes the head node.
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
/// The node after `this` (a node is a list address too).
const LIST_NEXT: u32 = 0x0072_6070;
/// Adds an item: the argument is the address of a word holding the item.
const LIST_ADD: u32 = 0x005a_e3d0;

// Callees on the arrays and the bad-form list.
/// The constructor of `arrayInteriorCells` (`this`, 0, 1).
const CELL_ARRAY_CONSTRUCT: u32 = 0x0047_0040;
/// The constructor of `arrayAddonNodes` (`this`, 0, 1).
const ADDON_NODE_ARRAY_CONSTRUCT: u32 = 0x0047_00a0;
/// The constructor of `listBadForms`.
const BAD_FORM_LIST_CONSTRUCT: u32 = 0x0048_f200;
/// The destructor of `listBadForms`.
const BAD_FORM_LIST_DESTRUCT: u32 = 0x0054_da00;
/// The destructor body `0045d930` calls for `arrayInteriorCells`.
const CELL_ARRAY_DESTRUCT: u32 = 0x0046_ffd0;
/// The destructor body `0045d950` calls for `arrayAddonNodes`.
const ADDON_NODE_ARRAY_DESTRUCT: u32 = 0x0047_0070;
/// The element count of `arrayInteriorCells` (`this` is the array).
const ARRAY_SIZE: u32 = 0x0065_8930;
/// The address of element `index` of `arrayInteriorCells` (`this` is the
/// array, the argument the index).
const ARRAY_AT: u32 = 0x0087_7a30;
/// Called with the cell array and 0 once its cells are destroyed.
const ARRAY_RESET: u32 = 0x0096_ad30;
/// Stores an element at an index, growing the array (`this` is the array;
/// arguments: the index, the address of a word holding the element).
const ARRAY_SET_AT_GROW: u32 = 0x0047_0000;
/// Called on `arrayInteriorCells` with 100 by the constructor (stores a
/// 16-bit value at `+0xE`, the array's grow-by).
const CELL_ARRAY_SET_GROW_BY: u32 = 0x0055_9490;
/// Called by `ClearData` on `arrayAddonNodes`.
const ADDON_NODE_ARRAY_CLEAR: u32 = 0x005e_03d0;

// Owned objects.
/// The `TESObjectList` constructor (`this`, 1); the object is 0x10 bytes.
const OBJECT_LIST_CONSTRUCT: u32 = 0x0051_00d0;
/// The `TESObjectList` destructor body.
const OBJECT_LIST_DESTRUCT: u32 = 0x0051_0110;
/// Adds an object to the `TESObjectList` (`this` is the list).
const OBJECT_LIST_ADD: u32 = 0x0051_02b0;
/// Called by `ClearData` on the `TESObjectList`.
const OBJECT_LIST_CLEAR: u32 = 0x0051_02e0;
/// The `TESRegionList` constructor (`this`, 1); 0x10 bytes.
const REGION_LIST_CONSTRUCT: u32 = 0x004f_6320;
/// Called by `ClearData` on the `TESRegionList`.
const REGION_LIST_CLEAR: u32 = 0x004f_6640;
/// The constructor of the `TESRegionDataManager`; 8 bytes.
const REGION_DATA_MANAGER_CONSTRUCT: u32 = 0x004f_3660;
/// The `TESIdleManager` constructor (0x28 bytes) and destructor body.
const IDLE_MANAGER_CONSTRUCT: u32 = 0x005f_faf0;
const IDLE_MANAGER_DESTRUCT: u32 = 0x005f_fb60;
/// The `BGSCameraPathManager` constructor (0x28 bytes) and destructor body.
const CAMERA_PATH_MANAGER_CONSTRUCT: u32 = 0x0058_b770;
const CAMERA_PATH_MANAGER_DESTRUCT: u32 = 0x0058_b7d0;
/// `BGSCameraPathManager::DestroyRootPathArrayForms` (`this`).
const CAMERA_PATH_MANAGER_DESTROY_ROOT_PATHS: u32 = 0x0058_ba40;
/// `InventoryChanges`' scalar deleting destructor (`this`, flags).
const INVENTORY_CHANGES_DELETE: u32 = 0x0043_1920;
/// Called by `~TESDataHandler` with `TESForm::pAllForms` as `this`: empties
/// the map (frees the entries of each bucket).
const ALL_FORMS_CLEAR: u32 = 0x0043_8af0;
/// Called with the address of `listFiles` by `0045dfa0` (cdecl).
const FILES_LIST_DESTROY: u32 = 0x0047_3270;

// `TESFile`.
/// `TESFile`'s destructor body.
const FILE_DESTRUCT: u32 = 0x0047_09f0;
/// `TESFile::CloseTES` (`this` is the file).
const FILE_CLOSE: u32 = 0x0047_1130;
/// `TESFile::GetMaster` and `TESFile::GetActive` (answer in AL).
const FILE_GET_MASTER: u32 = 0x0047_1c20;
const FILE_GET_ACTIVE: u32 = 0x0047_1d60;
/// Called on the active file by `ClearData` (`this`, 0) when the handler is
/// not a master save.
const FILE_CLOSE_ACTIVE: u32 = 0x0047_1d90;
/// `this + 0x20` of a `TESFile` (its name).
const FILE_NAME: u32 = 0x0089_1170;

// `TESForm`.
/// `TESForm::GetFile(index)` (`this` is the form).
const FORM_GET_FILE: u32 = 0x0048_4e60;
/// The form type byte (`TESForm +4`), zero-extended.
const FORM_GET_TYPE: u32 = 0x0040_1170;
/// The form id (`TESForm +0xC`).
const FORM_GET_ID: u32 = 0x0084_e3a0;
/// The name of the form's type, from the table at `0x01187004`.
const FORM_GET_TYPE_NAME: u32 = 0x0044_0e30;
/// Sets or clears bit 0 of the form's flags (`this`, flag).
const FORM_SET_FLAG_BIT_0: u32 = 0x0048_44f0;
/// Bit 5 (`0x20`) of the form's flags.
const FORM_HAS_FLAG_BIT_5: u32 = 0x0044_0d80;
/// Called by the type-3 branch of the leak report on a form.
const FORM_LIST_CLEAR: u32 = 0x0047_0470;
/// Vtable slots (byte offsets) of `TESForm`: `~TESForm` (deleting, flag),
/// `Load(file)`, `Save(file)`, `SaveEdit(file)`, `SetAltered(flag)`, a slot
/// the leak report calls with (0, 1) on forms of type 3, and the one it calls
/// to get a text for the log (the base version returns an empty string).
const FORM_VTABLE_DELETE: u32 = 0x10;
const FORM_VTABLE_LOAD: u32 = 0x20;
const FORM_VTABLE_SAVE: u32 = 0x28;
const FORM_VTABLE_SAVE_EDIT: u32 = 0x34;
const FORM_VTABLE_SET_ALTERED: u32 = 0xc8;
const FORM_VTABLE_SLOT_128: u32 = 0x128;
const FORM_VTABLE_SLOT_130: u32 = 0x130;
/// Returns its argument (cdecl).
const IDENTITY: u32 = 0x0046_4f30;

// Logging and the rest of the game.
/// `MessageHandler::IncDisableWarningCount(count)` (cdecl).
const DISABLE_WARNING_COUNT: u32 = 0x0043_b2b0;
/// The logging `printf` (cdecl: format, then the arguments).
const LOG_MESSAGE: u32 = 0x005b_5e40;
/// `TESDataHandler::CleanUpBadForms` (`this`).
const CLEAN_UP_BAD_FORMS: u32 = 0x0046_4e50;
/// `GarbageCollector::ClearAll(flag)`.
const GARBAGE_COLLECTOR_CLEAR_ALL: u32 = 0x0086_8d70;

// The map of every form (`TESForm::pAllForms`).
/// `GetFirstPos`: the first position, or 0 (`this` is the map).
const MAP_FIRST_POSITION: u32 = 0x004b_9ba0;
/// `GetNext(&position, &key, &value)`.
const MAP_GET_NEXT: u32 = 0x006b_7f20;
/// `SetAt(key, value)`.
const MAP_SET_AT: u32 = 0x0084_4700;

// Singletons and globals (the exe addresses of the variables).
/// `TESForm::pAllForms` (pointer variable).
const ALL_FORMS_MAP: u32 = 0x011c_54c0;
/// An object whose `00863db0` `ClearData` calls first (pointer variable).
const OBJECT_011C54C4: u32 = 0x011c_54c4;
/// The `TES` singleton (pointer variable).
const TES_SINGLETON: u32 = 0x011d_ea10;
/// The `PlayerCharacter` singleton (pointer variable).
const PLAYER_SINGLETON: u32 = 0x011d_ea3c;
/// An object `ClearData` asks (`004f6de0`) which idle-manager reset to run
/// (pointer variable).
const OBJECT_011DEA0C: u32 = 0x011d_ea0c;
/// The `TESIdleManager` singleton (pointer variable).
const IDLE_MANAGER: u32 = 0x011c_b6a0;
/// The `BGSCameraPathManager` singleton (pointer variable).
const CAMERA_PATH_MANAGER: u32 = 0x011c_a700;
/// Words that `00460160`, `00460190` and `ClearData` clear.
const FLAG_011CA830: u32 = 0x011c_a830;
const FLAG_011C9520: u32 = 0x011c_9520;
const FLAG_011CB96C: u32 = 0x011c_b96c;
const FLAG_011CA53C: u32 = 0x011c_a53c;
/// The words `ClearData` clears at its end (`0x011ca220` to `0x011ca26c`,
/// without `0x011ca24c` and `0x011ca264`).
const CLEARED_WORDS: [u32; 18] = [
    0x011c_a220,
    0x011c_a224,
    0x011c_a228,
    0x011c_a22c,
    0x011c_a230,
    0x011c_a234,
    0x011c_a238,
    0x011c_a23c,
    0x011c_a240,
    0x011c_a244,
    0x011c_a248,
    0x011c_a250,
    0x011c_a254,
    0x011c_a258,
    0x011c_a25c,
    0x011c_a260,
    0x011c_a268,
    0x011c_a26c,
];

// RTTI type descriptors for the `dynamic_cast`s of `AddFormToDataHandler`.
/// `TESForm`, the source type of every cast.
const FORM_TYPE_DESCRIPTOR: u32 = 0x0118_3028;
/// `EffectSetting`.
const EFFECT_SETTING_TYPE_DESCRIPTOR: u32 = 0x0118_37c4;
/// `TESObjectREFR`.
const REFERENCE_TYPE_DESCRIPTOR: u32 = 0x0118_41cc;
/// `TESObject`.
const OBJECT_TYPE_DESCRIPTOR: u32 = 0x0118_3128;

// Strings.
/// `"UNKNOWN"`.
const UNKNOWN_FILE_NAME: u32 = 0x0101_5890;
/// `"FORMS: Form '%s' (%08X) of type %s in file '%s' was not freed."`.
const FORM_LEAKED_FORMAT: u32 = 0x0101_8630;
/// `"FORMS: Forms were leaked during ClearData. Check Warnings file for more info."`.
const FORMS_LEAKED_MESSAGE: u32 = 0x0101_85e0;
/// `"FORMS: Unknown form type '%s' encountered in AddFormToDataHandler."`.
const UNKNOWN_FORM_TYPE_FORMAT: u32 = 0x0101_8670;

/// Size in bytes of `pFileIndex` (`TESFile*[1020]`).
const FILE_INDEX_BYTES: u32 = 0x3fc;
/// The first and last form list (`listPackages` to
/// `listMediaLocationControllers`): 58 consecutive `BSSimpleList`s, 8 bytes
/// apart.
const FIRST_LIST: u32 = 0x008;
const LAST_LIST: u32 = 0x1d0;

/// Returns `this + 0x6c` of the `TES` object (an embedded object), which
/// `ClearData` hands to `004ee920`.
const TES_GET_EMBEDDED_OBJECT: u32 = 0x0043_b5d0;
/// Called on that embedded object by `ClearData` (frees the chain of nodes at
/// `+4`).
const EMBEDDED_OBJECT_RESET: u32 = 0x004e_e920;

// Session 2 (`00461270` to `00462ec0`): more callees and data.
/// The `TESDataHandler` singleton (pointer variable), which some functions
/// read instead of using their own `this`.
const DATA_HANDLER_SINGLETON: u32 = 0x011c_3f2c;
/// The `TESSaveLoadGame` singleton (pointer variable): `this` of
/// `GetCreatedExteriorCellFormID`.
const SAVE_LOAD_GAME_SINGLETON: u32 = 0x011d_e45c;
/// An object (pointer variable) `UnloadCell` asks `0042ce10` about and hands
/// to `004623f0`.
const OBJECT_011DDF38: u32 = 0x011d_df38;
/// The object (in the exe's data, not a pointer variable) whose `009740a0`
/// `UnloadCell` calls with the cell.
const OBJECT_011E0E80: u32 = 0x011e_0e80;
/// Offset in the TLS block of the word whose bit 0 `004623f0` and `00462480`
/// work on.
const TLS_FLAG_WORD: u32 = 0x294;

/// The allocation scope object the game keeps on its stack (4 bytes):
/// `00404eb0` constructs it with (kind, 1, source file, line), `00404ee0`
/// destroys it.
const SCOPE_ENTER: u32 = 0x0040_4eb0;
const SCOPE_LEAVE: u32 = 0x0040_4ee0;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\TESDataHandler.cpp"`.
const SOURCE_FILE: u32 = 0x0101_86b8;

/// `_stricmp(a, b)` (cdecl wrapper).
const STRING_COMPARE: u32 = 0x0040_4dc0;
/// `_strcpy_s(destination, size, source)` (cdecl wrapper).
const STRING_COPY_S: u32 = 0x0040_6d30;
/// `sprintf_s(destination, size, format, ...)` (cdecl wrapper).
const FORMAT_S: u32 = 0x0040_6d00;
/// Converts the float on the stack to an integer (`FISTP`).
const FLOAT_TO_INT: u32 = 0x0040_6d90;
/// `_splitpath_s`, the body of `00462d40` (nine words).
const SPLIT_PATH_S: u32 = 0x00ec_7f1e;

/// `TESForm` lookup by form id (cdecl, one argument); null when the form is
/// not in `TESForm::pAllForms`.
const FORM_BY_ID: u32 = 0x0048_39c0;
/// `TESForm::GetFormByEditorID(name)` (Xbox PDB, cdecl).
const FORM_BY_EDITOR_ID: u32 = 0x0048_3a00;
/// `TESForm::GetFormTypeFromFormString(type word)` (Xbox PDB, cdecl; answer
/// in AL).
const FORM_TYPE_FROM_STRING: u32 = 0x0048_6890;
/// Virtual slot of `TESForm` that returns the editor id (no arguments), and
/// the one that sets it.
const FORM_VTABLE_GET_EDITOR_ID: u32 = 0x130;
const FORM_VTABLE_SET_EDITOR_ID: u32 = 0x134;
/// Virtual slot `NewCell` calls on the cell it keeps (no arguments) and the
/// one it calls with (form id, 1).
const CELL_VTABLE_SLOT_88: u32 = 0x88;
const CELL_VTABLE_SET_FORM_ID: u32 = 0x128;

/// The list-is-empty test (`this` is a list node: item and next are null).
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
/// `this` is a list node, the argument the address of a word holding an
/// item: whether the chain from the node holds that item.
const LIST_CONTAINS: u32 = 0x005f_65d0;
/// The same arguments: removes the item from the chain.
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
/// The same arguments: appends the item at the end of the chain.
const LIST_APPEND: u32 = 0x0090_5820;
/// The next object of a `TESObject` list: the word at `+0x20`.
const OBJECT_NEXT: u32 = 0x007a_f430;
/// The element count (`+0xC`) of the cell array.
const CELL_ARRAY_COUNT: u32 = 0x0099_38b0;
/// Compacts the cell array (removes the holes).
const CELL_ARRAY_COMPACT: u32 = 0x0060_0bc0;
/// Stores an element at an index (`this` is the array; the index and the
/// address of a word holding the element) and keeps the array's counts.
const CELL_ARRAY_SET_AT: u32 = 0x0096_ae90;
/// The sort key the cell sort compares (`this` is a cell).
const CELL_SORT_KEY: u32 = 0x0045_1cb0;
/// Appends an element (`this` is the add-on node array, the argument the
/// address of a word holding the node); answers the index.
const ADDON_NODE_ARRAY_ADD: u32 = 0x0099_38d0;
/// Clears the entry at an index of the add-on node array.
const ADDON_NODE_ARRAY_REMOVE_AT: u32 = 0x009e_98d0;
/// The index of an add-on node (`+0x50`) and its setter (`this`, index).
const ADDON_NODE_GET_INDEX: u32 = 0x0068_a830;
const ADDON_NODE_SET_INDEX: u32 = 0x0058_e430;

/// `TESObjectCELL`: constructor (`this`; the object is 0xE0 bytes),
/// `SetInterior(flag)`, the setter of the byte at `+0x24` (the cell flags),
/// `CreateCellData()` and `SetDataCoord(x, y)` (Xbox PDB for the named).
const CELL_CONSTRUCT: u32 = 0x0054_15b0;
const CELL_SET_INTERIOR: u32 = 0x0054_4300;
const CELL_SET_FLAGS_BYTE: u32 = 0x0046_1310;
const CELL_CREATE_CELL_DATA: u32 = 0x0054_4630;
const CELL_SET_DATA_COORD: u32 = 0x0054_4c90;
/// `TESObjectCELL::Detach(flag)`.
const CELL_DETACH: u32 = 0x0055_2bd0;
/// Sets the byte at `+0x26` of the cell.
const CELL_SET_BYTE_26: u32 = 0x0045_12a0;
/// Called by `UnloadCell` on the cell, in this order.
const CELL_UNLOAD_STEP_1: u32 = 0x0055_08b0;
const CELL_UNLOAD_STEP_2: u32 = 0x0054_b750;
/// `GetLowestProcessMiddleLow()` and `SetLowestProcessToMiddleLow(flag)`.
const CELL_GET_LOWEST_PROCESS: u32 = 0x0055_1620;
const CELL_SET_LOWEST_PROCESS: u32 = 0x0055_1480;
/// `UnloadCell` calls it with the cell and `!sleeping`.
const CELL_SET_PROCESS_FLAG: u32 = 0x0054_af40;
/// Whether bit 0 of the byte at `+0x24` is set (an interior cell).
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
/// `TESObjectCELL::GetLand()` and `TESObjectLAND::UnLoadVertices()`.
const CELL_GET_LAND: u32 = 0x0054_6fb0;
const LAND_UNLOAD_VERTICES: u32 = 0x0053_6d80;
const CELL_UNLOAD_STEP_3: u32 = 0x0054_6970;
const CELL_UNLOAD_STEP_4: u32 = 0x0096_1f30;
/// `TESObjectCELL::GetWorldSpace()`.
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
/// `009740a0(cell)` with the object at [`OBJECT_011E0E80`] as `this`.
const OBJECT_011E0E80_STEP: u32 = 0x0097_40a0;
/// Stores its argument in the byte at `0x01202df0`.
const SET_BYTE_01202DF0: u32 = 0x0044_ada0;
/// Whether bit 1 of the word at `+0x244` of the object is set.
const OBJECT_011DDF38_TEST: u32 = 0x0042_ce10;
/// `bClearingData` of the handler it is called on (`+0x61D`).
const HANDLER_IS_CLEARING_DATA: u32 = 0x0042_26e0;
/// `bSaveLoad` of the handler it is called on (`+0x61A`).
const HANDLER_IS_SAVE_LOAD: u32 = 0x0045_16b0;
/// `PlayerCharacter::IsSleepingorResting()` (Xbox PDB).
const PLAYER_IS_SLEEPING_OR_RESTING: u32 = 0x0094_df60;

/// `TESWorldSpace`: `GetCellFromCellCoord(x, y)`, `AddCell(cell)`, the lookup
/// of a cell by editor id and `GetExtCellDataFromFileByEditorID(name, &x,
/// &y)` (Xbox PDB for the named).
const WORLD_GET_CELL: u32 = 0x0058_75a0;
const WORLD_ADD_CELL: u32 = 0x0058_7670;
const WORLD_FIND_CELL_BY_EDITOR_ID: u32 = 0x0058_8560;
const WORLD_GET_EXT_CELL_DATA: u32 = 0x0058_5940;
/// `TESWorldSpace::UnLoadCell(cell)` (Xbox PDB).
const WORLD_UNLOAD_CELL: u32 = 0x0058_5e00;
/// `TESSaveLoadGame::GetCreatedExteriorCellFormID(world id, x, y)` (Xbox PDB).
const GET_CREATED_EXTERIOR_CELL_FORM_ID: u32 = 0x0086_1640;
/// `TESNPC::InitValues(flag)` (Xbox PDB).
const NPC_INIT_VALUES: u32 = 0x0060_3be0;
/// Calls of the topic code.
const TOPIC_LIST_FUNCTION: u32 = 0x0061_a380;
const TOPIC_INFO_LIST_FUNCTION: u32 = 0x0061_f360;

// `TESFile` (offsets are the Xbox PDB fields, checked against the PC code).
/// `TESFile::OpenTES(this; 0, 0)`.
const FILE_OPEN: u32 = 0x0047_0c70;
/// `TESFile::TESFile(this; directory, file name, 0)`; the object is 0x42C
/// bytes.
const FILE_CONSTRUCT: u32 = 0x0047_0710;
/// `TESFile::NextForm(flag)`.
const FILE_NEXT_FORM: u32 = 0x0047_2150;
/// Skips the rest of the current group (answer in AL).
const FILE_SKIP_GROUP: u32 = 0x0047_3660;
/// `TESFile::GetOptimizedFile` (answer in AL).
const FILE_GET_OPTIMIZED: u32 = 0x0047_1ca0;
/// `TESFile::GetIndexFile(index)`.
const FILE_GET_INDEX_FILE: u32 = 0x0047_1a10;
/// The count the loop over `GetIndexFile` runs to.
const FILE_INDEX_COUNT: u32 = 0x0047_1ad0;
/// `TESFile::GenIndexTable(list, flag)`.
const FILE_GEN_INDEX_TABLE: u32 = 0x0047_1870;
/// `cCompileIndex` (`+0x40C`).
const FILE_COMPILE_INDEX: u32 = 0x0047_3250;
/// `TESFile::GetTESChunk()`: the id of the current chunk.
const FILE_GET_CHUNK_ID: u32 = 0x0047_26b0;
/// Moves to the next chunk of the form; false when there is none (no name in
/// the map).
const FILE_NEXT_CHUNK: u32 = 0x0047_26f0;
/// `m_actualChunkSize` (`+0x25C`).
const FILE_CHUNK_SIZE: u32 = 0x0040_1660;
/// `TESFile::GetChunkData(this; buffer, size)` (size 0: the whole chunk).
const FILE_GET_CHUNK_DATA: u32 = 0x0047_2890;
/// `bMustEndianConvert` (`+0x299`).
const FILE_MUST_ENDIAN_CONVERT: u32 = 0x0040_1680;
/// Sets bit 2 of the file's `m_Flags` (`+0x3E8`) when its argument is true
/// (and clears bits 2 and 3 otherwise).
const FILE_SET_FLAG_BIT_2: u32 = 0x0047_1d00;
/// Opens the file's header and answers a status: 0 when it worked, else a
/// code (2 when the file cannot be opened). No name in the map.
const FILE_OPEN_HEADER: u32 = 0x0047_1400;
/// `TESDataHandler::GetListFile(name)` (Xbox PDB).
const GET_LIST_FILE: u32 = 0x0046_2f40;
/// `TESDataHandler::IsDLCPackageName(name)` (Xbox PDB).
const IS_DLC_PACKAGE_NAME: u32 = 0x0046_feb0;
/// Initialises the 12-byte record read from a cell's `XCLC` chunk (two words
/// and a byte).
const COORDS_INIT: u32 = 0x0054_0720;
/// Byte-swaps a word in place (`this` is the word's address; the second
/// word is not read).
const SWAP_WORD: u32 = 0x0040_1080;
/// The record type words of the form stream, in the exe's data: the type of
/// a group header, of the records the code casts to `TESWorldSpace`, and of
/// the records with the editor id and `XCLC` coordinate chunks.
const GROUP_TYPE_WORD: u32 = 0x0118_7020;
const WORLD_TYPE_WORD: u32 = 0x0118_7314;
const CELL_TYPE_WORD: u32 = 0x0118_72b4;
/// Chunk ids: `EDID` and `XCLC` as little-endian words.
const CHUNK_EDID: u32 = 0x4449_4445;
const CHUNK_XCLC: u32 = 0x434c_4358;

// Files and archives.
/// `BSFile::BSFile(this; path, 0, 0x100, 1)`, `BSFile::Exist()` and
/// `BSFile::~BSFile()` (Xbox PDB). The object is 0x158 bytes or more; the
/// frame of `004624b0` leaves that much.
const BSFILE_CONSTRUCT: u32 = 0x00b0_0260;
const BSFILE_EXIST: u32 = 0x00af_fc60;
const BSFILE_DESTRUCT: u32 = 0x00af_f240;
/// The byte at `+0x2c` of the `BSFile` (answer in AL).
const BSFILE_FLAG_2C: u32 = 0x0089_05f0;
/// `FileFinder::Exist(name, buffer, 1, -1)` (Xbox PDB, cdecl).
const FILE_FINDER_EXIST: u32 = 0x0045_6a20;
/// `ArchiveManager::OpenArchive(name, 0, 0)` (Xbox PDB, cdecl).
const OPEN_ARCHIVE: u32 = 0x00af_4be0;
/// Windows imports (slots of the import table): `lstrcpyA`, `lstrcatA`,
/// `FindFirstFileA`, `CompareFileTime`, `FindNextFileA`, `FindClose`.
const LSTRCPY_A: u32 = 0x00fd_f078;
const LSTRCAT_A: u32 = 0x00fd_f074;
const FIND_FIRST_FILE_A: u32 = 0x00fd_f070;
const COMPARE_FILE_TIME: u32 = 0x00fd_f06c;
const FIND_NEXT_FILE_A: u32 = 0x00fd_f068;
const FIND_CLOSE: u32 = 0x00fd_f064;
/// `INVALID_HANDLE_VALUE`.
const INVALID_HANDLE: u32 = 0xffff_ffff;
/// `"Update.bsa"`, `"%s%s.NAM"`, `"*.esp"` and `"*.esm"`.
const UPDATE_BSA: u32 = 0x0101_8804;
const NAM_PATH_FORMAT: u32 = 0x0101_8810;
const PATTERN_ESP: u32 = 0x0101_881c;
const PATTERN_ESM: u32 = 0x0101_8824;
/// `"CELLS: Trying to get exterior cell for invalid cell coordinate. Values
/// must be between %i and %i."`.
const INVALID_CELL_COORD_FORMAT: u32 = 0x0101_87a0;
/// `"FORMS: Addon Node %08X \"%s\" is trying to use index %i, but Addon
/// Node %08X \"%s\" already uses that index.  Addon Node %08X \"%s\" will be
/// remapped to a new index."`.
const ADDON_INDEX_CLASH_FORMAT: u32 = 0x0101_8700;
/// The empty string `GetCellFromCellCoord` passes to `NewCell` as the cell's
/// editor id.
const EMPTY_STRING: u32 = 0x0101_1584;

// Offsets in a `WIN32_FIND_DATAA` (0x140 bytes).
const FIND_DATA_LAST_WRITE_TIME: u32 = 0x14;
const FIND_DATA_SIZE_HIGH: u32 = 0x1c;
const FIND_DATA_SIZE_LOW: u32 = 0x20;
const FIND_DATA_FILE_NAME: u32 = 0x2c;

// RTTI type descriptors for the casts of the session-2 functions.
const NPC_TYPE_DESCRIPTOR: u32 = 0x0118_3a1c;
const CLASS_TYPE_DESCRIPTOR: u32 = 0x0118_6424;
const SOUND_TYPE_DESCRIPTOR: u32 = 0x0118_32fc;
const FACTION_TYPE_DESCRIPTOR: u32 = 0x0118_4704;
const REPUTATION_TYPE_DESCRIPTOR: u32 = 0x0118_64e4;
const LOAD_SCREEN_TYPE_TYPE_DESCRIPTOR: u32 = 0x0118_6110;
const TOPIC_TYPE_DESCRIPTOR: u32 = 0x0118_4720;
const WORLD_SPACE_TYPE_DESCRIPTOR: u32 = 0x0118_3fd0;

/// Sets a `BSStringT` (`this`, the text); answers `this`.
const SUMMARY_SET: u32 = 0x0043_8390;

layout! {
    /// `TESDataHandler` (Xbox PDB), 0x63C bytes on both builds. The lists
    /// are `BSSimpleList`s of pointers to the form class named in each doc.
    pub struct TESDataHandler: 0x63C {
        /// `cDLCFlags` (Xbox PDB): `char`.
        0x000 cDLCFlags: u8,
        /// `pObjectList` (Xbox PDB): `TESObjectList*`.
        0x004 pObjectList: Ptr,
        /// `listPackages` (Xbox PDB): `BSSimpleList<TESPackage *>`.
        0x008 listPackages: Inline<BSSimpleList>,
        /// `listWorldSpaces` (Xbox PDB): `BSSimpleList<TESWorldSpace *>`.
        0x010 listWorldSpaces: Inline<BSSimpleList>,
        /// `listClimates` (Xbox PDB): `BSSimpleList<TESClimate *>`.
        0x018 listClimates: Inline<BSSimpleList>,
        /// `listImageSpaces` (Xbox PDB): `BSSimpleList<TESImageSpace *>`.
        0x020 listImageSpaces: Inline<BSSimpleList>,
        /// `listImageSpaceModifiers` (Xbox PDB): `BSSimpleList<TESImageSpaceModifier *>`.
        0x028 listImageSpaceModifiers: Inline<BSSimpleList>,
        /// `listWeather` (Xbox PDB): `BSSimpleList<TESWeather *>`.
        0x030 listWeather: Inline<BSSimpleList>,
        /// `listEnchantmentItems` (Xbox PDB): `BSSimpleList<EnchantmentItem *>`.
        0x038 listEnchantmentItems: Inline<BSSimpleList>,
        /// `listSpellItems` (Xbox PDB): `BSSimpleList<SpellItem *>`.
        0x040 listSpellItems: Inline<BSSimpleList>,
        /// `listHeadParts` (Xbox PDB): `BSSimpleList<BGSHeadPart *>`.
        0x048 listHeadParts: Inline<BSSimpleList>,
        /// `listHair` (Xbox PDB): `BSSimpleList<TESHair *>`.
        0x050 listHair: Inline<BSSimpleList>,
        /// `listEyes` (Xbox PDB): `BSSimpleList<TESEyes *>`.
        0x058 listEyes: Inline<BSSimpleList>,
        /// `listRaces` (Xbox PDB): `BSSimpleList<TESRace *>`.
        0x060 listRaces: Inline<BSSimpleList>,
        /// `listZones` (Xbox PDB): `BSSimpleList<BGSEncounterZone *>`.
        0x068 listZones: Inline<BSSimpleList>,
        /// `listLandTexts` (Xbox PDB): `BSSimpleList<TESLandTexture *>`.
        0x070 listLandTexts: Inline<BSSimpleList>,
        /// `listCameraShots` (Xbox PDB): `BSSimpleList<BGSCameraShot *>`.
        0x078 listCameraShots: Inline<BSSimpleList>,
        /// `listClasses` (Xbox PDB): `BSSimpleList<TESClass *>`.
        0x080 listClasses: Inline<BSSimpleList>,
        /// `listFactions` (Xbox PDB): `BSSimpleList<TESFaction *>`.
        0x088 listFactions: Inline<BSSimpleList>,
        /// `listReputations` (Xbox PDB): `BSSimpleList<TESReputation *>`.
        0x090 listReputations: Inline<BSSimpleList>,
        /// `listChallenges` (Xbox PDB): `BSSimpleList<TESChallenge *>`.
        0x098 listChallenges: Inline<BSSimpleList>,
        /// `listRecipes` (Xbox PDB): `BSSimpleList<TESRecipe *>`.
        0x0A0 listRecipes: Inline<BSSimpleList>,
        /// `listRecipeCategories` (Xbox PDB): `BSSimpleList<TESRecipeCategory *>`.
        0x0A8 listRecipeCategories: Inline<BSSimpleList>,
        /// `listAmmoEffects` (Xbox PDB): `BSSimpleList<TESAmmoEffect *>`.
        0x0B0 listAmmoEffects: Inline<BSSimpleList>,
        /// `listCasinos` (Xbox PDB): `BSSimpleList<TESCasino *>`.
        0x0B8 listCasinos: Inline<BSSimpleList>,
        /// `listCaravanDecks` (Xbox PDB): `BSSimpleList<TESCaravanDeck *>`.
        0x0C0 listCaravanDecks: Inline<BSSimpleList>,
        /// `listScripts` (Xbox PDB): `BSSimpleList<Script *>`.
        0x0C8 listScripts: Inline<BSSimpleList>,
        /// `listSounds` (Xbox PDB): `BSSimpleList<TESSound *>`.
        0x0D0 listSounds: Inline<BSSimpleList>,
        /// `listAcousticSpaces` (Xbox PDB): `BSSimpleList<BGSAcousticSpace *>`.
        0x0D8 listAcousticSpaces: Inline<BSSimpleList>,
        /// `listRagdolls` (Xbox PDB): `BSSimpleList<BGSRagdoll *>`.
        0x0E0 listRagdolls: Inline<BSSimpleList>,
        /// `listGlobals` (Xbox PDB): `BSSimpleList<TESGlobal *>`.
        0x0E8 listGlobals: Inline<BSSimpleList>,
        /// `listVoiceTypes` (Xbox PDB): `BSSimpleList<BGSVoiceType *>`.
        0x0F0 listVoiceTypes: Inline<BSSimpleList>,
        /// `listImpactData` (Xbox PDB): `BSSimpleList<BGSImpactData *>`.
        0x0F8 listImpactData: Inline<BSSimpleList>,
        /// `listImpactDataSet` (Xbox PDB): `BSSimpleList<BGSImpactDataSet *>`.
        0x100 listImpactDataSet: Inline<BSSimpleList>,
        /// `listTopics` (Xbox PDB): `BSSimpleList<TESTopic *>`.
        0x108 listTopics: Inline<BSSimpleList>,
        /// `listTopicInfos` (Xbox PDB): `BSSimpleList<TESTopicInfo *>`.
        0x110 listTopicInfos: Inline<BSSimpleList>,
        /// `listQuests` (Xbox PDB): `BSSimpleList<TESQuest *>`.
        0x118 listQuests: Inline<BSSimpleList>,
        /// `listCombatStyles` (Xbox PDB): `BSSimpleList<TESCombatStyle *>`.
        0x120 listCombatStyles: Inline<BSSimpleList>,
        /// `listLoadScreens` (Xbox PDB): `BSSimpleList<TESLoadScreen *>`.
        0x128 listLoadScreens: Inline<BSSimpleList>,
        /// `listWater` (Xbox PDB): `BSSimpleList<TESWaterForm *>`.
        0x130 listWater: Inline<BSSimpleList>,
        /// `listEffectShaders` (Xbox PDB): `BSSimpleList<TESEffectShader *>`.
        0x138 listEffectShaders: Inline<BSSimpleList>,
        /// `listProjectiles` (Xbox PDB): `BSSimpleList<BGSProjectile *>`.
        0x140 listProjectiles: Inline<BSSimpleList>,
        /// `listExplosions` (Xbox PDB): `BSSimpleList<BGSExplosion *>`.
        0x148 listExplosions: Inline<BSSimpleList>,
        /// `listRadiation` (Xbox PDB): `BSSimpleList<BGSRadiationStage *>`.
        0x150 listRadiation: Inline<BSSimpleList>,
        /// `listDehydration` (Xbox PDB): `BSSimpleList<BGSDehydrationStage *>`.
        0x158 listDehydration: Inline<BSSimpleList>,
        /// `listHunger` (Xbox PDB): `BSSimpleList<BGSHungerStage *>`.
        0x160 listHunger: Inline<BSSimpleList>,
        /// `listSleepDeprevation` (Xbox PDB): `BSSimpleList<BGSSleepDeprevationStage *>`.
        0x168 listSleepDeprevation: Inline<BSSimpleList>,
        /// `listDebris` (Xbox PDB): `BSSimpleList<BGSDebris *>`.
        0x170 listDebris: Inline<BSSimpleList>,
        /// `listPerks` (Xbox PDB): `BSSimpleList<BGSPerk *>`.
        0x178 listPerks: Inline<BSSimpleList>,
        /// `listPartData` (Xbox PDB): `BSSimpleList<BGSBodyPartData *>`.
        0x180 listPartData: Inline<BSSimpleList>,
        /// `listNotes` (Xbox PDB): `BSSimpleList<BGSNote *>`.
        0x188 listNotes: Inline<BSSimpleList>,
        /// `listListForms` (Xbox PDB): `BSSimpleList<BGSListForm *>`.
        0x190 listListForms: Inline<BSSimpleList>,
        /// `listMenuIcons` (Xbox PDB): `BSSimpleList<BGSMenuIcon *>`.
        0x198 listMenuIcons: Inline<BSSimpleList>,
        /// `animObjects` (Xbox PDB): `BSSimpleList<TESObjectANIO *>`.
        0x1A0 animObjects: Inline<BSSimpleList>,
        /// `listMessages` (Xbox PDB): `BSSimpleList<BGSMessage *>`.
        0x1A8 listMessages: Inline<BSSimpleList>,
        /// `listLightingTemplates` (Xbox PDB): `BSSimpleList<BGSLightingTemplate *>`.
        0x1B0 listLightingTemplates: Inline<BSSimpleList>,
        /// `listMusicTypes` (Xbox PDB): `BSSimpleList<BGSMusicType *>`.
        0x1B8 listMusicTypes: Inline<BSSimpleList>,
        /// `listLoadScreenTypes` (Xbox PDB): `BSSimpleList<TESLoadScreenType *>`.
        0x1C0 listLoadScreenTypes: Inline<BSSimpleList>,
        /// `listMediaSets` (Xbox PDB): `BSSimpleList<MediaSet *>`.
        0x1C8 listMediaSets: Inline<BSSimpleList>,
        /// `listMediaLocationControllers` (Xbox PDB): `BSSimpleList<MediaLocationController *>`.
        0x1D0 listMediaLocationControllers: Inline<BSSimpleList>,
        /// `pRegionList` (Xbox PDB): `TESRegionList*`.
        0x1D8 pRegionList: Ptr,
        /// `arrayInteriorCells` (Xbox PDB): `NiTPrimitiveArray<TESObjectCELL *>`.
        0x1DC arrayInteriorCells: Inline<NiTArray>,
        /// `arrayAddonNodes` (Xbox PDB): `NiTPrimitiveArray<BGSAddonNode *>`.
        0x1EC arrayAddonNodes: Inline<NiTArray>,
        /// `listBadForms` (Xbox PDB): `NiTList<TESForm *>`.
        0x1FC listBadForms: Inline<()>,
        /// `iNextID` (Xbox PDB): `u32`.
        0x208 iNextID: u32,
        /// `pActiveFile` (Xbox PDB): `TESFile*`.
        0x20C pActiveFile: Ptr,
        /// `listFiles` (Xbox PDB): `BSSimpleList<TESFile *>`.
        0x210 listFiles: Inline<BSSimpleList>,
        /// `iNumCompile` (Xbox PDB): `u32`.
        0x218 iNumCompile: u32,
        /// `pFileIndex` (Xbox PDB): `TESFile*[1020]` (0x3FC bytes), the first slot.
        0x21C pFileIndex: Inline<()>,
        /// `bMasterSave` (Xbox PDB): `bool`.
        0x618 bMasterSave: bool,
        /// `bSaveLoadGame` (Xbox PDB): `bool`.
        0x619 bSaveLoadGame: bool,
        /// `bSaveLoad` (Xbox PDB): `bool`.
        0x61A bSaveLoad: bool,
        /// `bAutoSaving` (Xbox PDB): `bool`.
        0x61B bAutoSaving: bool,
        /// `bExportingPlugin` (Xbox PDB): `bool`.
        0x61C bExportingPlugin: bool,
        /// `bClearingData` (Xbox PDB): `bool`.
        0x61D bClearingData: bool,
        /// `bHasDesiredFiles` (Xbox PDB): `bool`.
        0x61E bHasDesiredFiles: bool,
        /// `bCheckingModels` (Xbox PDB): `bool`.
        0x61F bCheckingModels: bool,
        /// `bLoadingFiles` (Xbox PDB): `bool`.
        0x620 bLoadingFiles: bool,
        /// `bDontRemoveIDs` (Xbox PDB): `bool`.
        0x621 bDontRemoveIDs: bool,
        /// `ucGameSettingsLoadState` (Xbox PDB): `u8`.
        0x622 ucGameSettingsLoadState: u8,
        /// `pRegionDataManager` (Xbox PDB): `TESRegionDataManager*`.
        0x624 pRegionDataManager: Ptr,
        /// `pBarterContainer` (Xbox PDB): `InventoryChanges*`.
        0x628 pBarterContainer: Ptr,
        /// `pRecipeContainer` (Xbox PDB): `InventoryChanges*`.
        0x62C pRecipeContainer: Ptr,
        /// `pSpotterShader` (Xbox PDB): `TESEffectShader*`.
        0x630 pSpotterShader: Ptr,
        /// `pItemDetectedShader` (Xbox PDB): `TESEffectShader*`.
        0x634 pItemDetectedShader: Ptr,
        /// `pCateyeMobileShader` (Xbox PDB): `TESEffectShader*`.
        0x638 pCateyeMobileShader: Ptr,
    }
}

// Translated from 0040fbe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Error` (Xbox PDB): an empty function (the code only sets up and tears
/// down its stack frame).
pub fn error(_e: &mut Engine) {}

// Translated from 0045d270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::TESDataHandler` (Xbox PDB): constructs the 58 form lists
/// (`FIRST_LIST` to `LAST_LIST`), the two arrays, `listBadForms` and
/// `listFiles`, sets the flags and the next form id (`0x800`), clears the
/// file index, creates the object list, the region list, the region data
/// manager, the idle manager and the camera path manager, and returns
/// `this`. The cell array is given its size (100) after `pRegionList` is
/// stored.
pub fn tes_data_handler_tes_data_handler(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
) -> Ptr<TESDataHandler> {
    for offset in (FIRST_LIST..=LAST_LIST).step_by(8) {
        e.call(LIST_CONSTRUCT, &args![this.byte_add(offset)]);
    }
    let cells = this.at(TESDataHandler::arrayInteriorCells);
    e.call(CELL_ARRAY_CONSTRUCT, &args![cells, 0u32, 1u32]);
    let addon_nodes = this.at(TESDataHandler::arrayAddonNodes);
    e.call(ADDON_NODE_ARRAY_CONSTRUCT, &args![addon_nodes, 0u32, 1u32]);
    e.call(
        BAD_FORM_LIST_CONSTRUCT,
        &args![this.at(TESDataHandler::listBadForms)],
    );
    e.call(LIST_CONSTRUCT, &args![this.at(TESDataHandler::listFiles)]);

    e.set(this, TESDataHandler::iNextID, 0x800);
    e.set(this, TESDataHandler::iNumCompile, 0);
    e.set(this, TESDataHandler::pActiveFile, Ptr::NULL);
    e.set(this, TESDataHandler::bMasterSave, false);
    e.set(this, TESDataHandler::bSaveLoadGame, false);
    e.set(this, TESDataHandler::bSaveLoad, false);
    e.set(this, TESDataHandler::bAutoSaving, false);
    e.set(this, TESDataHandler::bExportingPlugin, false);
    e.set(this, TESDataHandler::bClearingData, false);
    e.set(this, TESDataHandler::bHasDesiredFiles, true);
    e.set(this, TESDataHandler::bDontRemoveIDs, false);
    e.set(this, TESDataHandler::cDLCFlags, 0);
    e.set(this, TESDataHandler::ucGameSettingsLoadState, 0);
    e.call(
        MEMSET,
        &args![this.at(TESDataHandler::pFileIndex), 0u32, FILE_INDEX_BYTES],
    );

    let object_list = new_object(e, 0x10, OBJECT_LIST_CONSTRUCT, Some(1));
    e.set(this, TESDataHandler::pObjectList, object_list);
    let region_list = new_object(e, 0x10, REGION_LIST_CONSTRUCT, Some(1));
    e.set(this, TESDataHandler::pRegionList, region_list);
    e.set(this, TESDataHandler::pBarterContainer, Ptr::NULL);
    e.set(this, TESDataHandler::pRecipeContainer, Ptr::NULL);
    e.call(CELL_ARRAY_SET_GROW_BY, &args![cells, 100u32]);
    let region_data_manager = new_object(e, 8, REGION_DATA_MANAGER_CONSTRUCT, None);
    e.set(
        this,
        TESDataHandler::pRegionDataManager,
        region_data_manager,
    );
    let idle_manager = new_object(e, 0x28, IDLE_MANAGER_CONSTRUCT, None);
    e.set_global(IDLE_MANAGER, idle_manager.addr());
    let camera_path_manager = new_object(e, 0x28, CAMERA_PATH_MANAGER_CONSTRUCT, None);
    e.set_global(CAMERA_PATH_MANAGER, camera_path_manager.addr());
    e.set(this, TESDataHandler::bCheckingModels, false);
    e.set(this, TESDataHandler::pCateyeMobileShader, Ptr::NULL);
    e.set(this, TESDataHandler::pSpotterShader, Ptr::NULL);
    e.set(this, TESDataHandler::pItemDetectedShader, Ptr::NULL);
    this
}

/// `new T(args)` as the constructor writes it: `operator new(size)`, then
/// the constructor with `this` set to the block (and the one argument some
/// of them take) only if the allocation succeeded, else a null pointer.
fn new_object(e: &mut Engine, size: u32, construct: u32, argument: Option<u32>) -> Ptr {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        return Ptr::NULL;
    }
    let object = match argument {
        Some(argument) => e.call(construct, &args![block, argument]),
        None => e.call(construct, &args![block]),
    };
    object.ptr()
}

// Translated from 0045d930 (decompiled, FalloutNV.exe 1.4.0.525)
/// The destructor body of `arrayInteriorCells` (the `NiTPrimitiveArray` at
/// `this`): calls `0046ffd0`.
pub fn fn_0045d930(e: &mut Engine, this: Ptr) {
    e.call(CELL_ARRAY_DESTRUCT, &args![this]);
}

// Translated from 0045d950 (decompiled, FalloutNV.exe 1.4.0.525)
/// The destructor body of `arrayAddonNodes`: calls `00470070`.
pub fn fn_0045d950(e: &mut Engine, this: Ptr) {
    e.call(ADDON_NODE_ARRAY_DESTRUCT, &args![this]);
}

// Translated from 0045d970 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::~TESDataHandler`: deletes the barter container, the
/// files (`0045dfa0`), the object list, the region list, the region data
/// manager, the idle manager (and clears its global), the camera path
/// manager (ditto) and the `pAllForms` map's `00438af0` object, then runs
/// the member destructors in reverse order of construction.
pub fn fn_0045d970(e: &mut Engine, this: Ptr<TESDataHandler>) {
    let barter_container = e.get(this, TESDataHandler::pBarterContainer);
    if !barter_container.is_null() {
        e.call(INVENTORY_CHANGES_DELETE, &args![barter_container, 1u32]);
    }
    fn_0045dfa0(e, this);
    let object_list = e.get(this, TESDataHandler::pObjectList);
    if !object_list.is_null() {
        fn_0045df10(e, object_list, 1);
    }
    let region_list = e.get(this, TESDataHandler::pRegionList);
    if !region_list.is_null() {
        e.vcall(region_list.addr(), 0, &args![1u32]);
    }
    let region_data_manager = e.get(this, TESDataHandler::pRegionDataManager);
    e.call(OPERATOR_DELETE, &args![region_data_manager]);
    let idle_manager: u32 = e.global(IDLE_MANAGER);
    if idle_manager != 0 {
        fn_0045df40(e, Ptr::new(idle_manager), 1);
    }
    e.set_global(IDLE_MANAGER, 0u32);
    let camera_path_manager: u32 = e.global(CAMERA_PATH_MANAGER);
    if camera_path_manager != 0 {
        fn_0045df70(e, Ptr::new(camera_path_manager), 1);
    }
    e.set_global(CAMERA_PATH_MANAGER, 0u32);
    let all_forms: u32 = e.global(ALL_FORMS_MAP);
    if all_forms != 0 {
        e.call(ALL_FORMS_CLEAR, &args![all_forms]);
    }
    e.call(LIST_DESTRUCT, &args![this.at(TESDataHandler::listFiles)]);
    e.call(
        BAD_FORM_LIST_DESTRUCT,
        &args![this.at(TESDataHandler::listBadForms)],
    );
    fn_0045d950(e, this.at(TESDataHandler::arrayAddonNodes).cast());
    fn_0045d930(e, this.at(TESDataHandler::arrayInteriorCells).cast());
    for offset in (FIRST_LIST..=LAST_LIST).rev().step_by(8) {
        e.call(LIST_DESTRUCT, &args![this.byte_add(offset)]);
    }
}

// Translated from 0045df10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of the `TESObjectList` (`this`): runs its
/// destructor body (`00510110`) and, when bit 0 of `flags` is set, frees the
/// block. Returns `this`.
pub fn fn_0045df10(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(OBJECT_LIST_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0045df40 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of the `TESIdleManager`: body `005ffb60`,
/// then free when bit 0 of `flags` is set. Returns `this`.
pub fn fn_0045df40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(IDLE_MANAGER_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0045df70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of the `BGSCameraPathManager`: body
/// `0058b7d0`, then free when bit 0 of `flags` is set. Returns `this`.
pub fn fn_0045df70(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(CAMERA_PATH_MANAGER_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0045dfa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00473270` (cdecl) with the address of `listFiles`.
pub fn fn_0045dfa0(e: &mut Engine, this: Ptr<TESDataHandler>) {
    let files = fn_0045dfc0(e, this);
    e.call(FILES_LIST_DESTROY, &args![files]);
}

// Translated from 0045dfc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listFiles` (`this + 0x210`).
pub fn fn_0045dfc0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listFiles)
}

/// Empties a form list the way `ClearData` does: while the list's first
/// item is not null, removes the first node and destroys the item through
/// its vtable (slot `0x10`, the scalar deleting destructor, with the flag 1).
fn drain_list(e: &mut Engine, list: Ptr<BSSimpleList>) {
    loop {
        let head_item = e.call(LIST_HEAD_ITEM, &args![list]).u32();
        let item = e.mem.u32(head_item);
        if item == 0 {
            break;
        }
        e.call(LIST_REMOVE_HEAD, &args![list]);
        e.vcall(item, FORM_VTABLE_DELETE, &args![1u32]);
    }
}

/// `drain_list` without destroying the items: removes every node of the list
/// (`listTopicInfos`, whose items the topics own).
fn pop_list(e: &mut Engine, list: Ptr<BSSimpleList>) {
    loop {
        let head_item = e.call(LIST_HEAD_ITEM, &args![list]).u32();
        if e.mem.u32(head_item) == 0 {
            break;
        }
        e.call(LIST_REMOVE_HEAD, &args![list]);
    }
}

// Translated from 0045dfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::ClearData` (Xbox PDB): destroys everything the data
/// handler loaded, in this order:
/// 1. sets `bClearingData`, runs `CleanUpBadForms` and a series of resets of
///    other subsystems (calls by address, in the code's order);
/// 2. empties the 58 form lists, destroying each form through its vtable
///    (slot `0x10`, flag 1), in the fixed order of the code (see the
///    test's `CLEAR_ORDER`); `listTopicInfos` is only emptied, the interior
///    cells of `arrayInteriorCells` are destroyed and the array reset, the
///    player is destroyed and its global cleared, and the singletons
///    (object list, add-on node array, region list, navmesh obstacles, idle
///    manager, camera path manager, ...) are reset in between;
/// 3. when `TESForm::pAllForms` is still there, reports the forms that were
///    not freed (`report_leaked_forms`);
/// 4. clears the file index and, unless a save is being loaded, closes
///    (unless `bMasterSave`) and destroys the active file and restores
///    `iNextID` to `0x800`;
/// 5. clears the words `0x011ca220`..`0x011ca26c`, `bClearingData` and
///    the three effect shader pointers.
///
/// Returns true.
pub fn tes_data_handler_clear_data(e: &mut Engine, this: Ptr<TESDataHandler>) -> bool {
    e.set(this, TESDataHandler::bClearingData, true);
    e.call(CLEAN_UP_BAD_FORMS, &args![this]);
    let object: u32 = e.global(OBJECT_011C54C4);
    e.call(0x0086_3db0, &args![object]);
    let tes: u32 = e.global(TES_SINGLETON);
    e.call(0x0045_39a0, &args![tes, 0u32, 0u32]);
    fn_00460170(e, Ptr::new(tes));
    let tes_object = e.call(TES_GET_EMBEDDED_OBJECT, &args![tes]).u32();
    e.call(EMBEDDED_OBJECT_RESET, &args![tes_object]);
    e.call(GARBAGE_COLLECTOR_CLEAR_ALL, &args![0u32]);
    e.call(0x004f_f190, &args![]);
    let player: u32 = e.global(PLAYER_SINGLETON);
    e.call(0x0095_2f90, &args![player]);
    e.call(0x004c_1c40, &args![]);
    e.call(0x0059_3210, &args![]);
    e.call(0x0058_d710, &args![]);
    fn_00460190(e);
    drain_list(e, this.at(TESDataHandler::listScripts));
    drain_list(e, this.at(TESDataHandler::listListForms));
    drain_list(e, this.at(TESDataHandler::listMenuIcons));
    drain_list(e, this.at(TESDataHandler::listHeadParts));
    drain_list(e, this.at(TESDataHandler::listHair));
    drain_list(e, this.at(TESDataHandler::listEyes));
    drain_list(e, this.at(TESDataHandler::listRaces));
    e.set_global(FLAG_011CB96C, 0u32);
    drain_list(e, this.at(TESDataHandler::listZones));
    drain_list(e, this.at(TESDataHandler::listClimates));
    drain_list(e, this.at(TESDataHandler::listWeather));
    drain_list(e, this.at(TESDataHandler::listClasses));
    drain_list(e, this.at(TESDataHandler::listFactions));
    drain_list(e, this.at(TESDataHandler::listChallenges));
    drain_list(e, this.at(TESDataHandler::listReputations));
    drain_list(e, this.at(TESDataHandler::listRecipes));
    drain_list(e, this.at(TESDataHandler::listCasinos));
    drain_list(e, this.at(TESDataHandler::listRecipeCategories));
    drain_list(e, this.at(TESDataHandler::listAmmoEffects));
    drain_list(e, this.at(TESDataHandler::listGlobals));
    drain_list(e, this.at(TESDataHandler::listVoiceTypes));
    drain_list(e, this.at(TESDataHandler::listImpactData));
    drain_list(e, this.at(TESDataHandler::listImpactDataSet));
    drain_list(e, this.at(TESDataHandler::listQuests));
    drain_list(e, this.at(TESDataHandler::listTopics));
    e.call(0x0061a270, &args![]);
    pop_list(e, this.at(TESDataHandler::listTopicInfos));
    let cell_count = e
        .call(
            ARRAY_SIZE,
            &args![this.at(TESDataHandler::arrayInteriorCells)],
        )
        .i32();
    for index in 0..cell_count {
        let slot = e
            .call(
                ARRAY_AT,
                &args![this.at(TESDataHandler::arrayInteriorCells), index],
            )
            .u32();
        let cell = e.mem.u32(slot);
        if cell != 0 {
            e.vcall(cell, 0x10, &args![1u32]);
        }
    }
    e.call(
        ARRAY_RESET,
        &args![this.at(TESDataHandler::arrayInteriorCells), 0u32],
    );
    drain_list(e, this.at(TESDataHandler::listWorldSpaces));
    e.call(GARBAGE_COLLECTOR_CLEAR_ALL, &args![0u32]);
    drain_list(e, this.at(TESDataHandler::listSounds));
    let list = fn_00460090(e, this);
    drain_list(e, list);
    let list = fn_004600b0(e, this);
    drain_list(e, list);
    let list = fn_004600d0(e, this);
    drain_list(e, list);
    drain_list(e, this.at(TESDataHandler::listAcousticSpaces));
    drain_list(e, this.at(TESDataHandler::listLandTexts));
    drain_list(e, this.at(TESDataHandler::listMessages));
    let player_character: u32 = e.global(PLAYER_SINGLETON);
    if player_character != 0 {
        e.vcall(player_character, 0x10, &args![1u32]);
    }
    e.set_global(PLAYER_SINGLETON, 0u32);
    drain_list(e, this.at(TESDataHandler::listImageSpaces));
    drain_list(e, this.at(TESDataHandler::listImageSpaceModifiers));
    let object_list = e.get(this, TESDataHandler::pObjectList);
    e.call(OBJECT_LIST_CLEAR, &args![object_list]);
    e.call(
        ADDON_NODE_ARRAY_CLEAR,
        &args![this.at(TESDataHandler::arrayAddonNodes)],
    );
    drain_list(e, this.at(TESDataHandler::listSpellItems));
    drain_list(e, this.at(TESDataHandler::listEnchantmentItems));
    drain_list(e, this.at(TESDataHandler::listPackages));
    drain_list(e, this.at(TESDataHandler::listCombatStyles));
    drain_list(e, this.at(TESDataHandler::listLoadScreens));
    drain_list(e, this.at(TESDataHandler::listLoadScreenTypes));
    drain_list(e, this.at(TESDataHandler::listWater));
    e.set_global(FLAG_011CA53C, 0u32);
    drain_list(e, this.at(TESDataHandler::animObjects));
    drain_list(e, this.at(TESDataHandler::listEffectShaders));
    drain_list(e, this.at(TESDataHandler::listProjectiles));
    drain_list(e, this.at(TESDataHandler::listExplosions));
    drain_list(e, this.at(TESDataHandler::listDebris));
    drain_list(e, this.at(TESDataHandler::listPerks));
    drain_list(e, this.at(TESDataHandler::listRadiation));
    drain_list(e, this.at(TESDataHandler::listDehydration));
    drain_list(e, this.at(TESDataHandler::listHunger));
    drain_list(e, this.at(TESDataHandler::listSleepDeprevation));
    drain_list(e, this.at(TESDataHandler::listPartData));
    // `TES` reset calls.
    let region_list = e.get(this, TESDataHandler::pRegionList);
    e.call(REGION_LIST_CLEAR, &args![region_list]);
    let navmesh_obstacles = e.call(0x006c_0720, &args![]).u32();
    e.call(0x006c_09f0, &args![navmesh_obstacles]);
    let tes: u32 = e.global(TES_SINGLETON);
    if e.call(0x0045_af00, &args![tes]).u32() != 0 {
        e.call(0x0045_afb0, &args![tes, 0u32]);
    }
    let object: u32 = e.global(OBJECT_011DEA0C);
    let idle_manager: u32 = e.global(IDLE_MANAGER);
    if object == 0 || !e.call(0x004f_6de0, &args![object]).bool() {
        e.call(0x005f_fd20, &args![idle_manager]);
    } else {
        e.call(0x0060_0030, &args![idle_manager]);
    }
    let camera_path_manager: u32 = e.global(CAMERA_PATH_MANAGER);
    e.call(
        CAMERA_PATH_MANAGER_DESTROY_ROOT_PATHS,
        &args![camera_path_manager],
    );
    drain_list(e, this.at(TESDataHandler::listCameraShots));
    drain_list(e, this.at(TESDataHandler::listLightingTemplates));
    e.call(0x0040_8de0, &args![]);
    e.call(0x0066_f110, &args![]);
    drain_list(e, this.at(TESDataHandler::listNotes));
    drain_list(e, this.at(TESDataHandler::listRagdolls));
    drain_list(e, this.at(TESDataHandler::listCaravanDecks));
    let all_forms: u32 = e.global(ALL_FORMS_MAP);
    if all_forms != 0 {
        report_leaked_forms(e, all_forms);
    }

    // The file index: forget every loaded file.
    let file_count = e.get(this, TESDataHandler::iNumCompile);
    for index in 0..file_count {
        e.mem
            .set_u32(this.at(TESDataHandler::pFileIndex).addr() + 4 * index, 0);
    }
    e.set(this, TESDataHandler::iNumCompile, 0);

    let active_file = e.get(this, TESDataHandler::pActiveFile);
    if !e.get(this, TESDataHandler::bSaveLoadGame) && !active_file.is_null() {
        if !fn_004600f0(e, this) {
            e.call(FILE_CLOSE_ACTIVE, &args![active_file, 0u32]);
        }
        let active_file = e.get(this, TESDataHandler::pActiveFile);
        if !active_file.is_null() {
            fn_004601a0(e, active_file, 1);
        }
        e.set(this, TESDataHandler::pActiveFile, Ptr::NULL);
        e.set(this, TESDataHandler::iNextID, 0x800);
    }
    fn_00460160(e);
    for word in CLEARED_WORDS {
        e.set_global(word, 0u32);
    }
    e.set(this, TESDataHandler::bClearingData, false);
    e.set(this, TESDataHandler::pItemDetectedShader, Ptr::NULL);
    e.set(this, TESDataHandler::pSpotterShader, Ptr::NULL);
    e.set(this, TESDataHandler::pCateyeMobileShader, Ptr::NULL);
    true
}

/// The leak report at the end of `ClearData`: walks `TESForm::pAllForms`
/// with warnings disabled around it. A form of type 3 is
/// released through `00460110` and its virtual `0x128` (arguments 0 and 1);
/// any other is logged (`FORMS: Form '%s' (%08X) of type %s in file '%s' was
/// not freed.`, with the file of the form's last source file or `UNKNOWN`)
/// and removed from the map with `SetAt(key, 0)`. If any was logged, a final
/// message says so.
fn report_leaked_forms(e: &mut Engine, all_forms: u32) {
    e.call(DISABLE_WARNING_COUNT, &args![1u32]);
    let mut leaked = false;
    e.with_stack(12, |e, locals| {
        // position, key, form: the three out parameters of `GetNext`.
        let position = locals.addr();
        let key = locals.addr() + 4;
        let form_slot = locals.addr() + 8;
        let first = e.call(MAP_FIRST_POSITION, &args![all_forms]).u32();
        e.mem.set_u32(position, first);
        while e.mem.u32(position) != 0 {
            e.call(MAP_GET_NEXT, &args![all_forms, position, key, form_slot]);
            let form = e.mem.u32(form_slot);
            if form == 0 {
                continue;
            }
            if e.call(FORM_GET_TYPE, &args![form]).u32() == 3 {
                fn_00460110(e, Ptr::new(form));
                e.vcall(form, FORM_VTABLE_SLOT_128, &args![0u32, 1u32]);
                continue;
            }
            let file = e.call(FORM_GET_FILE, &args![form, -1i32]).u32();
            let file_name = if file != 0 {
                let file = e.call(FORM_GET_FILE, &args![form, -1i32]).u32();
                let name = e.call(FILE_NAME, &args![file]).u32();
                e.call(IDENTITY, &args![name]).u32()
            } else {
                UNKNOWN_FILE_NAME
            };
            let type_name = e.call(FORM_GET_TYPE_NAME, &args![form]).u32();
            let form_id = e.call(FORM_GET_ID, &args![form]).u32();
            // The 0 the code pushes before this call is not an argument (the
            // callee does not pop it; see the module notes).
            let editor_id = e.vcall(form, FORM_VTABLE_SLOT_130, &args![]).u32();
            let editor_id = e.call(IDENTITY, &args![editor_id]).u32();
            e.call(
                LOG_MESSAGE,
                &args![FORM_LEAKED_FORMAT, editor_id, form_id, type_name, file_name],
            );
            let key_value = e.mem.u32(key);
            e.call(MAP_SET_AT, &args![all_forms, key_value, 0u32]);
            leaked = true;
        }
    });
    e.call(DISABLE_WARNING_COUNT, &args![0u32]);
    if leaked {
        e.call(LOG_MESSAGE, &args![FORMS_LEAKED_MESSAGE]);
    }
}

// Translated from 00460090 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMusicTypes` (`this + 0x1b8`).
pub fn fn_00460090(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMusicTypes)
}

// Translated from 004600b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMediaSets` (`this + 0x1c8`).
pub fn fn_004600b0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMediaSets)
}

// Translated from 004600d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMediaLocationControllers` (`this + 0x1d0`).
pub fn fn_004600d0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMediaLocationControllers)
}

// Translated from 004600f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bMasterSave` (Xbox PDB, `this + 0x618`).
pub fn fn_004600f0(e: &mut Engine, this: Ptr<TESDataHandler>) -> bool {
    e.get(this, TESDataHandler::bMasterSave)
}

// Translated from 00460110 (decompiled, FalloutNV.exe 1.4.0.525)
/// On a `TESForm`: when the address of the list at `this + 0x10`
/// (`00460140`) is not null, calls `00470470` with it (as `this`).
pub fn fn_00460110(e: &mut Engine, this: Ptr) {
    if !fn_00460140(e, this.cast()).is_null() {
        let list = fn_00460140(e, this.cast());
        e.call(FORM_LIST_CLEAR, &args![list]);
    }
}

// Translated from 00460140 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `this + 0x10`: the handler's `listWorldSpaces`, or, called on
/// a `TESForm` (the linker folded the two), the list at `+0x10` of the form
/// that `TESForm::GetFile` walks.
pub fn fn_00460140(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listWorldSpaces)
}

// Translated from 00460160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at `0x011ca830`.
pub fn fn_00460160(e: &mut Engine) {
    e.set_global(FLAG_011CA830, 0u32);
}

// Translated from 00460170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at `this + 0x88` (of the `TES` object).
pub fn fn_00460170(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr() + 0x88, 0);
}

// Translated from 00460190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the word at `0x011c9520`.
pub fn fn_00460190(e: &mut Engine) {
    e.set_global(FLAG_011C9520, 0u32);
}

// Translated from 004601a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The scalar deleting destructor of a `TESFile`: body `004709f0`, then free
/// when bit 0 of `flags` is set. Returns `this`.
pub fn fn_004601a0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(FILE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 004601d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::LoadForm` (Xbox PDB), a plain function of the form and
/// the file: loads the form from the file (virtual `0x20`), then sets the
/// form's master flag (bit 0 of its flags) to 1 if it already had it, or to
/// whether the file is a master (`TESFile::GetMaster`) if it did not, and
/// marks the form altered (virtual `0xc8`, argument 1) when the file is the
/// active one. Returns the load's result.
pub fn tes_data_handler_load_form(e: &mut Engine, form: Ptr, file: Ptr) -> bool {
    let had_master_flag = fn_00460250(e, form);
    let loaded = e.vcall(form.addr(), FORM_VTABLE_LOAD, &args![file]).bool();
    if had_master_flag {
        e.call(FORM_SET_FLAG_BIT_0, &args![form, 1u32]);
    } else {
        let is_master = e.call(FILE_GET_MASTER, &args![file]).bool();
        e.call(FORM_SET_FLAG_BIT_0, &args![form, is_master as u32]);
    }
    if e.call(FILE_GET_ACTIVE, &args![file]).bool() {
        e.vcall(form.addr(), FORM_VTABLE_SET_ALTERED, &args![1u32]);
    }
    loaded
}

// Translated from 00460250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 0 of the form's `iFormFlags` (`this + 8`).
pub fn fn_00460250(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 8) & 1 != 0
}

// Translated from 00460270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::SaveForm` (Xbox PDB): saves `form` to the active file.
/// False when there is no active file, or when the form's last file is not
/// the active file (or the active file is a master) and the form does not
/// have bit 1 of its flags. A form with bit 0 set is written with virtual
/// `0x34` (`SaveEdit`); otherwise it is skipped if `00440d80` (bit 5 of its
/// flags) says so, else written with virtual `0x28` (`Save`). The second
/// stack argument (the last file's master flag, computed by the one caller)
/// is not used.
pub fn tes_data_handler_save_form(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    form: Ptr,
    _is_master: u8,
) -> bool {
    let active_file = e.get(this, TESDataHandler::pActiveFile);
    if active_file.is_null() {
        return false;
    }
    let file = e.call(FORM_GET_FILE, &args![form, -1i32]).ptr::<()>();
    let in_active_file =
        file == active_file && !e.call(FILE_GET_MASTER, &args![active_file]).bool();
    if !in_active_file && !fn_00460340(e, form) {
        return false;
    }
    if fn_00460250(e, form) {
        return e
            .vcall(form.addr(), FORM_VTABLE_SAVE_EDIT, &args![active_file])
            .bool();
    }
    if e.call(FORM_HAS_FLAG_BIT_5, &args![form]).bool() {
        return false;
    }
    e.vcall(form.addr(), FORM_VTABLE_SAVE, &args![active_file])
        .bool()
}

// Translated from 00460340 (decompiled, FalloutNV.exe 1.4.0.525)
/// Bit 1 of the form's `iFormFlags` (`this + 8`).
pub fn fn_00460340(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 8) & 2 != 0
}

// Translated from 00460360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `TESFile::CloseTES` on every file in `listFiles`, stopping at the
/// first node whose item is null.
pub fn fn_00460360(e: &mut Engine, this: Ptr<TESDataHandler>) {
    let mut node = fn_0045dfc0(e, this).addr();
    while node != 0 {
        let head_item = e.call(LIST_HEAD_ITEM, &args![node]).u32();
        if e.mem.u32(head_item) == 0 {
            break;
        }
        let head_item = e.call(LIST_HEAD_ITEM, &args![node]).u32();
        let file = e.mem.u32(head_item);
        e.call(FILE_CLOSE, &args![file]);
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
}

/// Adds `form` to `list` through `005ae3d0`, which takes the address of a
/// word holding the item.
fn list_add(e: &mut Engine, list: Ptr, form: Ptr) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), form.addr());
        e.call(LIST_ADD, &args![list, slot]);
    });
}

/// The list `AddFormToDataHandler` appends a form of type `form_type` to,
/// for the 57 types that have a plain list; `None` for the types it treats
/// specially (`0x10`, `0x39`, `0x3a`, `0x3d` to `0x40`, `0x58`, `0x69`) and
/// for the types it does not know. The lists are reached through the
/// accessors (`this + offset`) the compiler emitted for each, so the
/// accessors of other units are called by address.
fn list_for_form_type(e: &mut Engine, this: Ptr<TESDataHandler>, form_type: u32) -> Option<Ptr> {
    let accessor = match form_type {
        0x05 => return Some(fn_00461210(e, this).cast()),
        0x06 => 0x0046_1190,
        0x07 => 0x0061_30e0,
        0x08 => 0x0087_1a30,
        0x09 => 0x0062_4700,
        0x0a => 0x0061_3790,
        0x0b => 0x0043_c490,
        0x0c => 0x004e_a950,
        0x0d => 0x0045_c650,
        0x0e => return Some(fn_00461170(e, this).cast()),
        0x11 => 0x0063_77e0,
        0x12 => 0x0046_10f0,
        0x13 => 0x0041_d8a0,
        0x14 => 0x0087_eaa0,
        0x31 => return Some(fn_004611f0(e, this).cast()),
        0x33 => return Some(fn_00461010(e, this).cast()),
        0x35 => 0x0043_6aa0,
        0x36 => 0x0050_0940,
        0x37 => {
            // The list sits 4 bytes into what `004169d0` returns.
            let base = e.call(0x0041_69d0, &args![this]).u32();
            return Some(Ptr::new(base + 4));
        }
        0x41 => return Some(fn_00460140(e, this).cast()),
        0x47 => 0x0045_5600,
        0x49 => 0x0041_3f40,
        0x4a => return Some(fn_004610b0(e, this).cast()),
        0x4b => 0x0046_1070,
        0x4d => return Some(fn_00461090(e, this).cast()),
        0x4e => 0x0045_a730,
        0x4f => 0x0087_4670,
        0x51 => 0x0046_0ff0,
        0x52 => return Some(fn_00460fd0(e, this).cast()),
        0x53 => 0x0089_1170,
        0x54 => 0x0046_10d0,
        0x55 => return Some(fn_00461230(e, this).cast()),
        0x56 => 0x0046_0fb0,
        0x57 => 0x0045_a330,
        0x5a => return Some(fn_00461030(e, this).cast()),
        0x5b => 0x0046_1110,
        0x5d => return Some(fn_004611b0(e, this).cast()),
        0x5e => return Some(fn_004611d0(e, this).cast()),
        0x5f => 0x004a_0d10,
        0x61 => 0x0046_1130,
        0x62 => return Some(fn_00461250(e, this).cast()),
        0x63 => 0x009d_9f40,
        0x65 => 0x0046_1270,
        0x66 => return Some(fn_00460090(e, this).cast()),
        0x68 => 0x009c_1a50,
        0x6a => 0x0040_77e0,
        0x6b => 0x0050_3650,
        0x6d => 0x0098_4250,
        0x6e => 0x0046_1290,
        0x6f => return Some(fn_004600b0(e, this).cast()),
        0x70 => return Some(fn_004600d0(e, this).cast()),
        0x71 => 0x0046_1150,
        0x72 => 0x0098_4230,
        0x75 => 0x0051_4f30,
        0x76 => 0x0050_6390,
        0x77 => 0x0062_d2f0,
        0x78 => return Some(fn_00461050(e, this).cast()),
        _ => return None,
    };
    Some(e.call(accessor, &args![this]).ptr())
}

// Translated from 004603b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::AddFormToDataHandler` (Xbox PDB): files a newly
/// created or loaded form in the handler. Returns false for a null form and
/// for a form whose type nothing handles. By the form's type byte
/// (`TESForm +4`):
/// - most types are appended to their own `BSSimpleList` (`list_for_form_type`);
/// - `0x39` (a cell) is appended to `arrayInteriorCells` when its flag
///   `00425fd0` says so;
/// - `0x10` is cast to `EffectSetting` and given to `00409060`;
/// - `0x3a`, `0x3d` to `0x40` and `0x69` are cast to `TESObjectREFR` and
///   added to the cell they are in (`TESObjectCELL::AddReference`); a
///   reference with no parent cell gets the one found at its position in the
///   world space; persistence is updated with `TESObjectREFR::SetRefPersists`
///   and `00564eb0`;
/// - `0x58` goes to `AddAddonNode` and then also takes the last case's path;
/// - any other type is cast to `TESObject` and added to the object list, and
///   if it is not one the message `FORMS: Unknown form type ...` is logged and
///   false is returned.
pub fn tes_data_handler_add_form_to_data_handler(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    form: Ptr,
) -> bool {
    if form.is_null() {
        return false;
    }
    let form_type = e.call(FORM_GET_TYPE, &args![form]).u32();
    if let Some(list) = list_for_form_type(e, this, form_type) {
        list_add(e, list, form);
        return true;
    }
    match form_type {
        0x10 => {
            let setting = e
                .call(
                    DYNAMIC_CAST,
                    &args![
                        form,
                        0u32,
                        FORM_TYPE_DESCRIPTOR,
                        EFFECT_SETTING_TYPE_DESCRIPTOR,
                        0u32
                    ],
                )
                .ptr::<()>();
            if !setting.is_null() {
                e.call(0x0040_9060, &args![setting]);
            }
            true
        }
        0x39 => {
            if e.call(0x0042_5fd0, &args![form]).bool() {
                let cells = this.at(TESDataHandler::arrayInteriorCells);
                let count = e.call(ARRAY_SIZE, &args![cells]).u32();
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), form.addr());
                    e.call(ARRAY_SET_AT_GROW, &args![cells, count, slot]);
                });
            }
            true
        }
        0x3a | 0x3d | 0x3e | 0x3f | 0x40 | 0x69 => {
            add_reference_to_cell(e, this, form);
            true
        }
        0x58 => {
            e.call(0x0046_1820, &args![this, form]);
            add_form_to_object_list(e, this, form)
        }
        _ => add_form_to_object_list(e, this, form),
    }
}

/// The reference cases of `AddFormToDataHandler`: finds the reference's cell
/// (its parent cell, else the preferred one of `TES`'s `005f36f0`, else the
/// cell at its position in the current world space, `GetCellFromWorldCoord`;
/// a parent cell that is exterior also gets that lookup) and, if there is
/// one, adds the reference to it and updates the reference's persistence.
fn add_reference_to_cell(e: &mut Engine, this: Ptr<TESDataHandler>, form: Ptr) {
    let reference = e
        .call(
            DYNAMIC_CAST,
            &args![
                form,
                0u32,
                FORM_TYPE_DESCRIPTOR,
                REFERENCE_TYPE_DESCRIPTOR,
                0u32
            ],
        )
        .ptr::<()>();
    if reference.is_null() {
        return;
    }
    let tes: u32 = e.global(TES_SINGLETON);
    let mut cell = e.call(0x008d_6f30, &args![reference]).u32();
    if cell == 0 {
        cell = e.call(0x005f_36f0, &args![tes]).u32();
        if cell == 0 {
            cell = cell_at_reference_position(e, this, reference);
        }
    } else if !e.call(0x0042_5fd0, &args![cell]).bool() {
        cell = cell_at_reference_position(e, this, reference);
    }
    if cell != 0 {
        e.call(0x0054_8230, &args![cell, reference, 0u32]);
        if e.call(0x0056_5260, &args![reference]).bool() {
            e.call(0x0056_5480, &args![reference, 1u32]);
        }
        if e.call(0x0056_4e00, &args![reference]).bool() {
            e.call(0x0056_4eb0, &args![reference, 1u32]);
        }
    }
}

/// `GetCellFromWorldCoord(x, y, worldSpace)` for the reference's position
/// (virtual `0x1f4` of the reference returns the address of its position;
/// x is the first float, y the second) in the world space of `TES`
/// (`004fd3e0(1)`).
fn cell_at_reference_position(e: &mut Engine, this: Ptr<TESDataHandler>, reference: Ptr) -> u32 {
    let tes: u32 = e.global(TES_SINGLETON);
    let world_space = e.call(0x004f_d3e0, &args![tes, 1u32]).u32();
    let position = e.vcall(reference.addr(), 0x1f4, &args![]).u32();
    let y = e.mem.f32(position + 4);
    let position = e.vcall(reference.addr(), 0x1f4, &args![]).u32();
    let x = e.mem.f32(position);
    e.call(0x0046_1bc0, &args![this, x, y, world_space]).u32()
}

/// The last case of `AddFormToDataHandler`: the form is cast to `TESObject`
/// and added to the handler's object list; when it is not one, the unknown
/// type is logged and false returned.
fn add_form_to_object_list(e: &mut Engine, this: Ptr<TESDataHandler>, form: Ptr) -> bool {
    let object = e
        .call(
            DYNAMIC_CAST,
            &args![
                form,
                0u32,
                FORM_TYPE_DESCRIPTOR,
                OBJECT_TYPE_DESCRIPTOR,
                0u32
            ],
        )
        .ptr::<()>();
    if object.is_null() {
        let form_type = e.call(FORM_GET_TYPE, &args![form]).u32();
        let type_name = e.call(0x0046_12b0, &args![form_type]).u32();
        e.call(LOG_MESSAGE, &args![UNKNOWN_FORM_TYPE_FORMAT, type_name]);
        return false;
    }
    let object_list = e.get(this, TESDataHandler::pObjectList);
    e.call(OBJECT_LIST_ADD, &args![object_list, object]);
    true
}

// Translated from 00460fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listDebris` (`this + 0x170`).
pub fn fn_00460fd0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listDebris)
}

// Translated from 00461010 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listProjectiles` (`this + 0x140`).
pub fn fn_00461010(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listProjectiles)
}

// Translated from 00461030 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listRadiation` (`this + 0x150`).
pub fn fn_00461030(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listRadiation)
}

// Translated from 00461050 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listSleepDeprevation` (`this + 0x168`).
pub fn fn_00461050(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listSleepDeprevation)
}

// Translated from 00461090 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `animObjects` (`this + 0x1a0`).
pub fn fn_00461090(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::animObjects)
}

// Translated from 004610b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listCombatStyles` (`this + 0x120`).
pub fn fn_004610b0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listCombatStyles)
}

// Translated from 00461170 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listAcousticSpaces` (`this + 0xd8`).
pub fn fn_00461170(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listAcousticSpaces)
}

// Translated from 004611b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listVoiceTypes` (`this + 0xf0`).
pub fn fn_004611b0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listVoiceTypes)
}

// Translated from 004611d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listImpactData` (`this + 0xf8`).
pub fn fn_004611d0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listImpactData)
}

// Translated from 004611f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listNotes` (`this + 0x188`).
pub fn fn_004611f0(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listNotes)
}

// Translated from 00461210 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMenuIcons` (`this + 0x198`).
pub fn fn_00461210(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMenuIcons)
}

// Translated from 00461230 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listListForms` (`this + 0x190`).
pub fn fn_00461230(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listListForms)
}

// Translated from 00461250 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listMessages` (`this + 0x1a8`).
pub fn fn_00461250(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listMessages)
}

// ---------------------------------------------------------------------------
// Session 2: `00461270` to `00462ec0`.
// ---------------------------------------------------------------------------

/// Enters the allocation scope of a function (`kind` is the first argument
/// of the scope constructor, `line` the source line of the scope): the
/// 4-byte scope object the game keeps on its stack.
fn scope_enter(e: &mut Engine, kind: u32, line: u32) -> Ptr {
    let scope = Ptr::new(e.mem.alloc(4));
    e.call(SCOPE_ENTER, &args![scope, kind, 1u32, SOURCE_FILE, line]);
    scope
}

/// Destroys the scope object and frees its block.
fn scope_leave(e: &mut Engine, scope: Ptr) {
    e.call(SCOPE_LEAVE, &args![scope]);
    e.mem.free(scope.addr());
}

/// The item of a list node: `*` of the item slot address `006815c0` returns.
fn list_item(e: &mut Engine, node: u32) -> u32 {
    let slot = e.call(LIST_HEAD_ITEM, &args![node]).u32();
    e.mem.u32(slot)
}

/// Looks the form with `form_id` up and casts it to the class whose RTTI
/// type descriptor is `target` (`__RTDynamicCast(form, 0, TESForm, target,
/// 0)`).
fn form_by_id_as(e: &mut Engine, form_id: u32, target: u32) -> Ptr {
    let form = e.call(FORM_BY_ID, &args![form_id]).u32();
    e.call(
        DYNAMIC_CAST,
        &args![form, 0u32, FORM_TYPE_DESCRIPTOR, target, 0u32],
    )
    .ptr()
}

// Translated from 00461270 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listLightingTemplates` (`this + 0x1b0`).
pub fn fn_00461270(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listLightingTemplates)
}

// Translated from 00461290 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of `listLoadScreenTypes` (`this + 0x1c0`).
pub fn fn_00461290(_e: &mut Engine, this: Ptr<TESDataHandler>) -> Ptr<BSSimpleList> {
    this.at(TESDataHandler::listLoadScreenTypes)
}

// Translated from 00461330 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::NewCell` (Xbox PDB): creates the exterior cell at (`x`,
/// `y`) of `world`. Null when `world` is null. Otherwise it allocates a
/// `TESObjectCELL` (0xE0 bytes) and constructs it, gives it the form id the
/// save game created for that exterior cell (virtual `0x128`, arguments the
/// id and 1, only if there is one) and the editor id `name` (virtual
/// `0x134`, only if not null), makes it an exterior cell, sets the byte at
/// `+0x24` to 2, creates its cell data and sets its coordinates, and adds
/// it to the world space. If `AddCell` refuses, the cell is destroyed
/// (virtual `0x10`, flag 1) and null returned; else virtual `0x88` is
/// called on it and it is returned. `this` is not read. The allocation
/// scope (kind `0x1a`, line `0x7dd`) is only entered when `world` is not
/// null.
pub fn tes_data_handler_new_cell(
    e: &mut Engine,
    _this: Ptr<TESDataHandler>,
    name: u32,
    x: i32,
    y: i32,
    world: Ptr,
) -> Ptr {
    if world.is_null() {
        return Ptr::NULL;
    }
    let scope = scope_enter(e, 0x1a, 0x7dd);
    let block = e.call(OPERATOR_NEW, &args![0xe0u32]).u32();
    let cell = if block != 0 {
        e.call(CELL_CONSTRUCT, &args![block]).u32()
    } else {
        0
    };
    let world_id = e.call(FORM_GET_ID, &args![world]).u32();
    let save_load_game: u32 = e.global(SAVE_LOAD_GAME_SINGLETON);
    let created_id = e
        .call(
            GET_CREATED_EXTERIOR_CELL_FORM_ID,
            &args![save_load_game, world_id, x, y],
        )
        .u32();
    if created_id != 0 {
        e.vcall(cell, CELL_VTABLE_SET_FORM_ID, &args![created_id, 1u32]);
    }
    if name != 0 {
        e.vcall(cell, FORM_VTABLE_SET_EDITOR_ID, &args![name]);
    }
    e.call(CELL_SET_INTERIOR, &args![cell, 0u32]);
    e.call(CELL_SET_FLAGS_BYTE, &args![cell, 2u32]);
    e.call(CELL_CREATE_CELL_DATA, &args![cell]);
    e.call(CELL_SET_DATA_COORD, &args![cell, x, y]);
    let added = e.call(WORLD_ADD_CELL, &args![world, cell]).bool();
    let result = if added {
        e.vcall(cell, CELL_VTABLE_SLOT_88, &args![]);
        Ptr::new(cell)
    } else {
        if cell != 0 {
            e.vcall(cell, FORM_VTABLE_DELETE, &args![1u32]);
        }
        Ptr::NULL
    };
    scope_leave(e, scope);
    result
}

// Translated from 004614e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Walks the handler's object list (`pObjectList`, then its first object,
/// then each object's next one) and, for every object that casts to
/// `TESNPC` and whose component at `+0x30` has bit `0x80` set in its word at
/// `+4` (`00461560`), calls `TESNPC::InitValues(0)` (Xbox PDB).
pub fn fn_004614e0(e: &mut Engine, this: Ptr<TESDataHandler>) {
    let object_list = e.call(LIST_NEXT, &args![this]).u32();
    let mut object = e.call(LIST_NEXT, &args![object_list]).u32();
    while object != 0 {
        let npc = e
            .call(
                DYNAMIC_CAST,
                &args![
                    object,
                    0u32,
                    OBJECT_TYPE_DESCRIPTOR,
                    NPC_TYPE_DESCRIPTOR,
                    0u32
                ],
            )
            .u32();
        if npc != 0 && fn_00461560(e, Ptr::new(npc + 0x30)) {
            e.call(NPC_INIT_VALUES, &args![npc, 0u32]);
        }
        object = e.call(OBJECT_NEXT, &args![object]).u32();
    }
}

// Translated from 00461560 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit `0x80` of the word at `this + 4` is set (`00461580` with
/// `0x80`).
pub fn fn_00461560(e: &mut Engine, this: Ptr) -> bool {
    fn_00461580(e, this, 0x80)
}

// Translated from 00461580 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the word at `this + 4` has any of the bits in `mask`.
pub fn fn_00461580(e: &mut Engine, this: Ptr, mask: u32) -> bool {
    e.mem.u32(this.addr() + 4) & mask != 0
}

// Translated from 004615a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The form with id `form_id`, cast to `TESClass` (null when there is none
/// or it is another class). `this` is not read.
pub fn fn_004615a0(e: &mut Engine, _this: Ptr<TESDataHandler>, form_id: u32) -> Ptr {
    form_by_id_as(e, form_id, CLASS_TYPE_DESCRIPTOR)
}

// Translated from 004615d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetSound` (Xbox PDB): the form with id `form_id`, cast
/// to `TESSound`.
pub fn tes_data_handler_get_sound(e: &mut Engine, _this: Ptr<TESDataHandler>, form_id: u32) -> Ptr {
    form_by_id_as(e, form_id, SOUND_TYPE_DESCRIPTOR)
}

// Translated from 00461600 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetFaction` (Xbox PDB): the form with id `form_id`, cast
/// to `TESFaction`.
pub fn tes_data_handler_get_faction(
    e: &mut Engine,
    _this: Ptr<TESDataHandler>,
    form_id: u32,
) -> Ptr {
    form_by_id_as(e, form_id, FACTION_TYPE_DESCRIPTOR)
}

// Translated from 00461630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetReputation` (Xbox PDB): the form with id `form_id`,
/// cast to `TESReputation`.
pub fn tes_data_handler_get_reputation(
    e: &mut Engine,
    _this: Ptr<TESDataHandler>,
    form_id: u32,
) -> Ptr {
    form_by_id_as(e, form_id, REPUTATION_TYPE_DESCRIPTOR)
}

// Translated from 00461660 (decompiled, FalloutNV.exe 1.4.0.525)
/// The form with id `form_id`, cast to `TESLoadScreenType`.
pub fn fn_00461660(e: &mut Engine, _this: Ptr<TESDataHandler>, form_id: u32) -> Ptr {
    form_by_id_as(e, form_id, LOAD_SCREEN_TYPE_TYPE_DESCRIPTOR)
}

// Translated from 00461690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetTopic` (Xbox PDB): the form with id `form_id`, cast
/// to `TESTopic`.
pub fn tes_data_handler_get_topic(e: &mut Engine, _this: Ptr<TESDataHandler>, form_id: u32) -> Ptr {
    form_by_id_as(e, form_id, TOPIC_TYPE_DESCRIPTOR)
}

// Translated from 004616c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetSound` (Xbox PDB, overload by editor id): the form
/// `TESForm::GetFormByEditorID` finds for `name`, if its type byte is `0xd`,
/// else null. `this` is not read.
pub fn tes_data_handler_get_sound_ov2(
    e: &mut Engine,
    _this: Ptr<TESDataHandler>,
    name: Ptr,
) -> Ptr {
    let form = e.call(FORM_BY_EDITOR_ID, &args![name]).u32();
    if form != 0 && e.call(FORM_GET_TYPE, &args![form]).u32() == 0xd {
        Ptr::new(form)
    } else {
        Ptr::NULL
    }
}

// Translated from 00461700 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetGlobal` (Xbox PDB): the global in `listGlobals`
/// whose editor id (virtual `0x130`, no arguments) equals `name` ignoring
/// case; null if the list is empty or none does.
pub fn tes_data_handler_get_global(e: &mut Engine, this: Ptr<TESDataHandler>, name: Ptr) -> Ptr {
    let list = this.at(TESDataHandler::listGlobals);
    if e.call(LIST_IS_EMPTY, &args![list]).bool() {
        return Ptr::NULL;
    }
    let mut node = list.addr();
    while node != 0 {
        let item = list_item(e, node);
        if item != 0 {
            let editor_id = e.vcall(item, FORM_VTABLE_GET_EDITOR_ID, &args![]).u32();
            if e.call(STRING_COMPARE, &args![editor_id, name]).i32() == 0 {
                return Ptr::new(item);
            }
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    Ptr::NULL
}

// Translated from 00461780 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `0061a380(argument, &listTopics, 1)` (cdecl) and returns its
/// result.
pub fn fn_00461780(e: &mut Engine, this: Ptr<TESDataHandler>, argument: u32) -> u32 {
    e.call(
        TOPIC_LIST_FUNCTION,
        &args![argument, this.at(TESDataHandler::listTopics), 1u32],
    )
    .u32()
}

// Translated from 004617b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `0061f360(argument, &listTopicInfos)` (cdecl) and returns its
/// result.
pub fn fn_004617b0(e: &mut Engine, this: Ptr<TESDataHandler>, argument: u32) -> u32 {
    e.call(
        TOPIC_INFO_LIST_FUNCTION,
        &args![argument, this.at(TESDataHandler::listTopicInfos)],
    )
    .u32()
}

// Translated from 004617e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetAddonNode` (Xbox PDB): the node at `index` of
/// `arrayAddonNodes`, or null when `index` is not below its size.
pub fn tes_data_handler_get_addon_node(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    index: u32,
) -> Ptr {
    let nodes = this.at(TESDataHandler::arrayAddonNodes);
    let size = e.call(ARRAY_SIZE, &args![nodes]).u32();
    if index < size {
        let slot = e.call(ARRAY_AT, &args![nodes, index]).u32();
        Ptr::new(e.mem.u32(slot))
    } else {
        Ptr::NULL
    }
}

// Translated from 00461820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::AddAddonNode` (Xbox PDB): files an add-on node by its
/// index (`+0x50`). Nothing for a null node. If the index is inside the
/// array and another node already holds it, logs the clash (`FORMS: Addon
/// Node ... will be remapped to a new index.`, with the form ids and editor
/// ids of both) and treats the node as indexless (`0xffffffff`). A node
/// without an index is appended to the array and given the index that
/// answers; otherwise it is stored at its index (`SetAtGrow`).
pub fn tes_data_handler_add_addon_node(e: &mut Engine, this: Ptr<TESDataHandler>, node: Ptr) {
    if node.is_null() {
        return;
    }
    let nodes = this.at(TESDataHandler::arrayAddonNodes);
    let mut index = e.call(ADDON_NODE_GET_INDEX, &args![node]).u32();
    let size = e.call(ARRAY_SIZE, &args![nodes]).u32();
    if index < size {
        let slot = e.call(ARRAY_AT, &args![nodes, index]).u32();
        let existing = e.mem.u32(slot);
        if existing != 0 && existing != node.addr() {
            let node_name = e
                .vcall(node.addr(), FORM_VTABLE_GET_EDITOR_ID, &args![])
                .u32();
            let node_id = e.call(FORM_GET_ID, &args![node]).u32();
            let existing_name = e.vcall(existing, FORM_VTABLE_GET_EDITOR_ID, &args![]).u32();
            let existing_id = e.call(FORM_GET_ID, &args![existing]).u32();
            let node_name_again = e
                .vcall(node.addr(), FORM_VTABLE_GET_EDITOR_ID, &args![])
                .u32();
            let node_id_again = e.call(FORM_GET_ID, &args![node]).u32();
            e.call(
                LOG_MESSAGE,
                &args![
                    ADDON_INDEX_CLASH_FORMAT,
                    node_id_again,
                    node_name_again,
                    index,
                    existing_id,
                    existing_name,
                    node_id,
                    node_name
                ],
            );
            index = 0xffff_ffff;
        }
    }
    // The code keeps `node` in its parameter slot and passes that slot's
    // address to the array functions.
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), node.addr());
        if index == 0xffff_ffff {
            let new_index = e.call(ADDON_NODE_ARRAY_ADD, &args![nodes, slot]).u32();
            e.call(ADDON_NODE_SET_INDEX, &args![node, new_index]);
        } else {
            let node_index = e.call(ADDON_NODE_GET_INDEX, &args![node]).u32();
            e.call(ARRAY_SET_AT_GROW, &args![nodes, node_index, slot]);
        }
    });
}

// Translated from 00461930 (decompiled, FalloutNV.exe 1.4.0.525)
/// For a node that is not null, calls `009e98d0(arrayAddonNodes, node's
/// index)`.
pub fn fn_00461930(e: &mut Engine, this: Ptr<TESDataHandler>, node: Ptr) {
    if node.is_null() {
        return;
    }
    let index = e.call(ADDON_NODE_GET_INDEX, &args![node]).u32();
    e.call(
        ADDON_NODE_ARRAY_REMOVE_AT,
        &args![this.at(TESDataHandler::arrayAddonNodes), index],
    );
}

// Translated from 00461960 (decompiled, FalloutNV.exe 1.4.0.525)
/// The element count (`+0xC`, as a word) of `arrayInteriorCells`.
pub fn fn_00461960(e: &mut Engine, this: Ptr<TESDataHandler>) -> u32 {
    e.call(
        CELL_ARRAY_COUNT,
        &args![this.at(TESDataHandler::arrayInteriorCells)],
    )
    .u32()
}

// Translated from 00461980 (decompiled, FalloutNV.exe 1.4.0.525)
/// Compacts `arrayInteriorCells` (`00600bc0`), then returns its element at
/// `index`.
pub fn fn_00461980(e: &mut Engine, this: Ptr<TESDataHandler>, index: u32) -> u32 {
    let cells = this.at(TESDataHandler::arrayInteriorCells);
    e.call(CELL_ARRAY_COMPACT, &args![cells]);
    let slot = e.call(ARRAY_AT, &args![cells, index]).u32();
    e.mem.u32(slot)
}

// Translated from 004619b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Compacts `arrayInteriorCells`, then sorts it by selection: for each index
/// it finds the smallest later (or equal) element by comparing the keys
/// `00451cb0` gives (`_stricmp`; a null element is replaced by any element),
/// and swaps it in with two `SetAt` calls.
pub fn fn_004619b0(e: &mut Engine, this: Ptr<TESDataHandler>) {
    let cells = this.at(TESDataHandler::arrayInteriorCells);
    e.call(CELL_ARRAY_COMPACT, &args![cells]);
    let count = e.call(CELL_ARRAY_COUNT, &args![cells]).i32();
    let mut i = 0i32;
    while i < count - 1 {
        let slot = e.call(ARRAY_AT, &args![cells, i]).u32();
        let original = e.mem.u32(slot);
        let mut best = original;
        let mut best_index = i;
        let mut j = i + 1;
        while j < count {
            let slot = e.call(ARRAY_AT, &args![cells, j]).u32();
            let candidate = e.mem.u32(slot);
            if best != 0 && candidate != 0 {
                let best_key = e.call(CELL_SORT_KEY, &args![best]).u32();
                let candidate_key = e.call(CELL_SORT_KEY, &args![candidate]).u32();
                if e.call(STRING_COMPARE, &args![candidate_key, best_key])
                    .i32()
                    < 0
                {
                    best = candidate;
                    best_index = j;
                }
            } else if best == 0 {
                best = candidate;
                best_index = j;
            }
            j += 1;
        }
        if best != 0 && best != original {
            e.with_stack(8, |e, words| {
                let best_slot = words.addr();
                let original_slot = words.addr() + 4;
                e.mem.set_u32(best_slot, best);
                e.mem.set_u32(original_slot, original);
                e.call(CELL_ARRAY_SET_AT, &args![cells, i, best_slot]);
                e.call(CELL_ARRAY_SET_AT, &args![cells, best_index, original_slot]);
            });
        }
        i += 1;
    }
}

// Translated from 00461ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetCellByEditorID` (Xbox PDB): null for a null `name`.
/// Looks through `arrayInteriorCells` for the cell whose editor id (virtual
/// `0x130`) equals `name` ignoring case, then asks each world space of
/// `listWorldSpaces` (`00588560`) in turn; null if none has it.
pub fn tes_data_handler_get_cell_by_editor_id(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    name: Ptr,
) -> Ptr {
    if name.is_null() {
        return Ptr::NULL;
    }
    let cells = this.at(TESDataHandler::arrayInteriorCells);
    let count = e.call(ARRAY_SIZE, &args![cells]).i32();
    for index in 0..count {
        let slot = e.call(ARRAY_AT, &args![cells, index]).u32();
        let cell = e.mem.u32(slot);
        let editor_id = e.vcall(cell, FORM_VTABLE_GET_EDITOR_ID, &args![]).u32();
        if e.call(STRING_COMPARE, &args![editor_id, name]).i32() == 0 {
            return Ptr::new(cell);
        }
    }
    let mut node = fn_00460140(e, this).addr();
    while node != 0 {
        let world = list_item(e, node);
        if world != 0 {
            let cell = e
                .call(WORLD_FIND_CELL_BY_EDITOR_ID, &args![world, name])
                .u32();
            if cell != 0 {
                return Ptr::new(cell);
            }
        }
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    Ptr::NULL
}

// Translated from 00461bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetCellFromWorldCoord` (Xbox PDB): converts the world
/// position (`x`, `y`) to an integer each (`00406d90`) and shifts it right
/// by 12 (the 4096-unit cell size) for the cell coordinates, then returns
/// `00461c20` of them.
pub fn tes_data_handler_get_cell_from_world_coord(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    x: f32,
    y: f32,
    world: Ptr,
    create: u8,
) -> Ptr {
    let cell_x = e.call(FLOAT_TO_INT, &args![x]).i32() >> 12;
    let cell_y = e.call(FLOAT_TO_INT, &args![y]).i32() >> 12;
    fn_00461c20(e, this, cell_x, cell_y, world, create)
}

// Translated from 00461c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The exterior cell at (`x`, `y`) of `world` (the first world space of
/// `listWorldSpaces` when null; null when there is none). Coordinates
/// outside -0x8000..0x7fff are logged (`CELLS: Trying to get exterior cell
/// for invalid cell coordinate...`) and give null. Otherwise it asks the
/// world space (`GetCellFromCellCoord`); if it has none, and `bSaveLoad` is
/// clear and `create` is not zero, makes it with `NewCell`.
pub fn fn_00461c20(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    x: i32,
    y: i32,
    world: Ptr,
    create: u8,
) -> Ptr {
    let mut world = world;
    if world.is_null() {
        let worlds = fn_00460140(e, this).addr();
        world = Ptr::new(list_item(e, worlds));
        if world.is_null() {
            return Ptr::NULL;
        }
    }
    if x > 0x7fff || y > 0x7fff || x < -0x8000 || y < -0x8000 {
        e.call(
            LOG_MESSAGE,
            &args![INVALID_CELL_COORD_FORMAT, 0xffff_8000u32, 0x7fffu32],
        );
        return Ptr::NULL;
    }
    let cell = e.call(WORLD_GET_CELL, &args![world, x, y]).u32();
    if cell != 0 {
        return Ptr::new(cell);
    }
    if !e.call(HANDLER_IS_SAVE_LOAD, &args![this]).bool() && create != 0 {
        tes_data_handler_new_cell(e, this, EMPTY_STRING, x, y, world)
    } else {
        Ptr::NULL
    }
}

// Translated from 00461cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::GetExtCellDataFromFileByEditorID` (Xbox PDB): finds the
/// exterior cell named `name` and stores its coordinates in `out_x` and
/// `out_y` (both zeroed first); returns the world space it belongs to, or
/// null. Null or empty `name`: null.
///
/// - If any loaded file is "optimized" (`TESFile::GetOptimizedFile`), asks
///   each world space of the singleton's `listWorldSpaces`
///   (`TESWorldSpace::GetExtCellDataFromFileByEditorID`) until one answers
///   true (the loop stops at an empty list node) and returns it.
/// - Otherwise opens each loaded file in turn and scans its forms: it moves
///   over the top groups until the group of the world records, then reads
///   the world records (casting the form named by each record's id, with the
///   file index of its master, to `TESWorldSpace`; the last one is the
///   result) and cell records: a cell record's `EDID` chunk is compared with
///   `name` and, when equal, its `XCLC` chunk gives the coordinates (the
///   two words are byte-swapped when the file needs it). The file is closed
///   after the scan; a match ends the search.
///
/// The scan is inside the allocation scope of kind `0x16`, line `0x1041`.
pub fn tes_data_handler_get_ext_cell_data_from_file_by_editor_id(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    name: Ptr,
    out_x: Ptr,
    out_y: Ptr,
) -> Ptr {
    let scope = scope_enter(e, 0x16, 0x1041);
    e.mem.set_u32(out_x.addr(), 0);
    e.mem.set_u32(out_y.addr(), 0);
    let mut result = 0;
    if !name.is_null() && e.mem.u8(name.addr()) != 0 {
        result = find_ext_cell_data(e, this, name, out_x, out_y);
    }
    scope_leave(e, scope);
    Ptr::new(result)
}

/// The search of `GetExtCellDataFromFileByEditorID`; the world space or 0.
fn find_ext_cell_data(
    e: &mut Engine,
    this: Ptr<TESDataHandler>,
    name: Ptr,
    out_x: Ptr,
    out_y: Ptr,
) -> u32 {
    let file_index = this.at(TESDataHandler::pFileIndex).addr();
    let mut optimized = false;
    let mut index = 0;
    while index < e.get(this, TESDataHandler::iNumCompile) {
        let file = e.mem.u32(file_index + 4 * index);
        if file != 0 && e.call(FILE_GET_OPTIMIZED, &args![file]).bool() {
            optimized = true;
            break;
        }
        index += 1;
    }
    if optimized {
        let handler: u32 = e.global(DATA_HANDLER_SINGLETON);
        let mut node = fn_00460140(e, Ptr::new(handler)).addr();
        loop {
            if node == 0 || e.call(LIST_IS_EMPTY, &args![node]).bool() {
                return 0;
            }
            let world = list_item(e, node);
            node = e.call(LIST_NEXT, &args![node]).u32();
            if e.call(WORLD_GET_EXT_CELL_DATA, &args![world, name, out_x, out_y])
                .bool()
            {
                return world;
            }
        }
    }
    let mut index = 0;
    while index < e.get(this, TESDataHandler::iNumCompile) {
        let file = e.mem.u32(file_index + 4 * index);
        index += 1;
        if file == 0 || !e.call(FILE_OPEN, &args![file, 0u32, 0u32]).bool() {
            continue;
        }
        let mut world = 0;
        let found = scan_file_for_cell(e, file, name, out_x, out_y, &mut world);
        e.call(FILE_CLOSE, &args![file]);
        if found {
            return world;
        }
    }
    0
}

/// Scans the forms of an opened file for the cell named `name` (see
/// `GetExtCellDataFromFileByEditorID`); true when the cell was found. The
/// last world space cast is left in `world`.
fn scan_file_for_cell(
    e: &mut Engine,
    file: u32,
    name: Ptr,
    out_x: Ptr,
    out_y: Ptr,
    world: &mut u32,
) -> bool {
    let file_ptr = Ptr::new(file);
    let group_type: u32 = e.global(GROUP_TYPE_WORD);
    let world_type: u32 = e.global(WORLD_TYPE_WORD);
    let cell_type: u32 = e.global(CELL_TYPE_WORD);

    // Move over the groups to the one of the world records.
    let mut form = fn_00462270(e, file_ptr).addr();
    let mut found_group = false;
    while form != 0 && !found_group {
        let advanced;
        if e.mem.u32(form) == group_type {
            if e.mem.u32(form + 0xc) == 0 && e.mem.u32(form + 8) == world_type {
                found_group = true;
                advanced = e.call(FILE_NEXT_FORM, &args![file, 1u32]).bool();
            } else {
                advanced = e.call(FILE_SKIP_GROUP, &args![file]).bool();
            }
        } else {
            advanced = e.call(FILE_NEXT_FORM, &args![file, 1u32]).bool();
        }
        form = if advanced {
            fn_00462270(e, file_ptr).addr()
        } else {
            0
        };
    }
    if !found_group {
        return false;
    }

    form = fn_00462270(e, file_ptr).addr();
    let mut found = false;
    while form != 0 && !found {
        let mut advanced = false;
        let record_type = e.mem.u32(form);
        if record_type == world_type {
            let mut form_id = e.mem.u32(form + 0xc);
            let owner_index = form_id >> 24;
            let mut owner = e
                .call(FILE_GET_INDEX_FILE, &args![file, owner_index + 1])
                .u32();
            if owner == 0 {
                owner = file;
            }
            form_id &= 0x00ff_ffff;
            let compile_index = e.call(FILE_COMPILE_INDEX, &args![owner]).u8() as u32;
            form_id |= compile_index << 24;
            *world = form_by_id_as(e, form_id, WORLD_SPACE_TYPE_DESCRIPTOR).addr();
            advanced = e.call(FILE_NEXT_FORM, &args![file, 1u32]).bool();
        } else if record_type == cell_type {
            let mut chunk = e.call(FILE_GET_CHUNK_ID, &args![file]).u32();
            if chunk == CHUNK_EDID {
                let size = e.call(FILE_CHUNK_SIZE, &args![file]).u32().wrapping_add(1);
                let buffer = e.call(OPERATOR_NEW, &args![size]).u32();
                e.call(MEMSET, &args![buffer, 0u32, size]);
                e.call(FILE_GET_CHUNK_DATA, &args![file, buffer, 0u32]);
                if e.call(STRING_COMPARE, &args![buffer, name]).i32() == 0 {
                    found = true;
                    while chunk != 0 {
                        if chunk == CHUNK_XCLC {
                            e.with_stack(12, |e, coordinates| {
                                e.call(COORDS_INIT, &args![coordinates]);
                                e.call(MEMSET, &args![coordinates, 0u32, 12u32]);
                                e.call(FILE_GET_CHUNK_DATA, &args![file, coordinates, 12u32]);
                                if e.call(FILE_MUST_ENDIAN_CONVERT, &args![file]).bool() {
                                    fn_00462230(e, coordinates);
                                }
                                let x = e.mem.u32(coordinates.addr());
                                let y = e.mem.u32(coordinates.addr() + 4);
                                e.mem.set_u32(out_x.addr(), x);
                                e.mem.set_u32(out_y.addr(), y);
                            });
                            chunk = 0;
                        } else if e.call(FILE_NEXT_CHUNK, &args![file]).bool() {
                            chunk = e.call(FILE_GET_CHUNK_ID, &args![file]).u32();
                        } else {
                            chunk = 0;
                        }
                    }
                }
                e.call(OPERATOR_DELETE, &args![buffer]);
            }
            advanced = e.call(FILE_NEXT_FORM, &args![file, 1u32]).bool();
        } else if record_type == group_type {
            match e.mem.u32(form + 0xc) {
                0 => {
                    if e.mem.u32(form + 8) == world_type {
                        advanced = e.call(FILE_NEXT_FORM, &args![file, 1u32]).bool();
                    } else {
                        // The answer only decides to stop, which `advanced`
                        // already does.
                        let label = e.mem.u32(form + 8);
                        e.call(FORM_TYPE_FROM_STRING, &args![label]);
                    }
                }
                1 | 4 | 5 => {
                    advanced = e.call(FILE_NEXT_FORM, &args![file, 1u32]).bool();
                }
                6 => {
                    advanced = e.call(FILE_SKIP_GROUP, &args![file]).bool();
                }
                _ => {}
            }
        }
        form = if advanced {
            fn_00462270(e, file_ptr).addr()
        } else {
            0
        };
    }
    found
}

// Translated from 00462230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Byte-swaps the two words at `this` and `this + 4` (`00401080` on each).
pub fn fn_00462230(e: &mut Engine, this: Ptr) {
    e.call(SWAP_WORD, &args![this, 0u32]);
    e.call(SWAP_WORD, &args![this.addr() + 4, 0u32]);
}

// Translated from 00462270 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `this + 0x240` of a `TESFile`: its `m_currentform` (Xbox
/// PDB).
pub fn fn_00462270(_e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(this.addr() + 0x240)
}

// Translated from 00462290 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESDataHandler::UnloadCell` (Xbox PDB): unloads `cell`, a no-op for a
/// null cell or one with bit 5 of its flags set. In order: sets the byte at
/// `0x01202df0`; if the object at `0x011ddf38` has bit 1 at `+0x244`, sets
/// the thread flag with `004623f0(0)` and remembers its answer; detaches the
/// cell (`TESObjectCELL::Detach(1)`); `009740a0`, the byte at `+0x26`, two
/// more steps; clears the cell's lowest-process state while it reports it;
/// unless the singleton handler is clearing data, tells the cell whether the
/// player is not sleeping or resting (`0054af40`); for an exterior cell
/// unloads the vertices of its land; `00546970`, `00961f30(0)`; for an
/// exterior cell, `TESWorldSpace::UnLoadCell`; restores the thread flag
/// from the remembered answer and clears the byte at `0x01202df0`.
pub fn tes_data_handler_unload_cell(e: &mut Engine, _this: Ptr<TESDataHandler>, cell: Ptr) {
    if cell.is_null() || e.call(FORM_HAS_FLAG_BIT_5, &args![cell]).bool() {
        return;
    }
    e.call(SET_BYTE_01202DF0, &args![1u32]);
    let mut remembered = 0u8;
    let object: u32 = e.global(OBJECT_011DDF38);
    if e.call(OBJECT_011DDF38_TEST, &args![object]).bool() {
        remembered = fn_004623f0(e, Ptr::new(object), 0);
    }
    e.call(CELL_DETACH, &args![cell, 1u32]);
    e.call(OBJECT_011E0E80_STEP, &args![OBJECT_011E0E80, cell]);
    e.call(CELL_SET_BYTE_26, &args![cell, 1u32]);
    e.call(CELL_UNLOAD_STEP_1, &args![cell]);
    e.call(CELL_UNLOAD_STEP_2, &args![cell]);
    while e.call(CELL_GET_LOWEST_PROCESS, &args![cell]).bool() {
        e.call(CELL_SET_LOWEST_PROCESS, &args![cell, 0u32]);
    }
    let handler: u32 = e.global(DATA_HANDLER_SINGLETON);
    if !e.call(HANDLER_IS_CLEARING_DATA, &args![handler]).bool() {
        let player: u32 = e.global(PLAYER_SINGLETON);
        let resting = e.call(PLAYER_IS_SLEEPING_OR_RESTING, &args![player]).bool();
        e.call(CELL_SET_PROCESS_FLAG, &args![cell, !resting]);
    }
    if !e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
        let land = e.call(CELL_GET_LAND, &args![cell]).u32();
        e.call(LAND_UNLOAD_VERTICES, &args![land]);
    }
    e.call(CELL_UNLOAD_STEP_3, &args![cell]);
    e.call(CELL_UNLOAD_STEP_4, &args![cell, 0u32]);
    if !e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
        let world = e.call(CELL_GET_WORLD_SPACE, &args![cell]).u32();
        e.call(WORLD_UNLOAD_CELL, &args![world, cell]);
    }
    let object: u32 = e.global(OBJECT_011DDF38);
    if e.call(OBJECT_011DDF38_TEST, &args![object]).bool() {
        fn_004623f0(e, Ptr::new(object), remembered);
    }
    e.call(SET_BYTE_01202DF0, &args![0u32]);
}

// Translated from 004623f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears bit 0 of the word at `+0x294` of the TLS block: set when
/// `flag` is 0, cleared otherwise. Returns what `00462480` said before the
/// change. `this` is only passed on to `00462480`, which does not read it.
pub fn fn_004623f0(e: &mut Engine, _this: Ptr, flag: u8) -> u8 {
    let before = fn_00462480(e);
    let word = e.tls() + TLS_FLAG_WORD;
    let value = e.mem.u32(word);
    let value = if flag == 0 { value | 1 } else { value & !1 };
    e.mem.set_u32(word, value);
    before as u8
}

// Translated from 00462480 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 0 of the word at `+0x294` of the TLS block is clear. (Its
/// `this` is saved but never read.)
pub fn fn_00462480(e: &mut Engine) -> bool {
    let word = e.tls() + TLS_FLAG_WORD;
    e.mem.u32(word) & 1 == 0
}

/// The locals of `004624b0` in one block (the game's stack frame); the
/// offsets are those of the frame, in the order of the code's buffers.
const SCAN_SEARCH_DIRECTORY: u32 = 0x000;
const SCAN_FIND_PATH: u32 = 0x104;
const SCAN_FIND_DATA: u32 = 0x208;
const SCAN_BASE_NAME: u32 = 0x348;
const SCAN_EXTENSION: u32 = 0x44c;
const SCAN_NAM_PATH: u32 = 0x45c;
const SCAN_SUMMARY: u32 = 0x560;
const SCAN_ARCHIVE_PATH: u32 = 0x664;
const SCAN_STREAM: u32 = 0x768;
const SCAN_FILE_SLOT: u32 = 0x8c0;
const SCAN_OTHER_SLOT: u32 = 0x8c4;
const SCAN_FRAME_SIZE: u32 = 0x8c8;

// Translated from 004624b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scans the directory `directory` (a path ending with a separator) for
/// plugin files and brings `listFiles` up to date; always returns true.
/// Inside the allocation scope of kind `0x16`, line `0x1195`:
/// 1. Each file already in `listFiles` is reopened for its header
///    (`00471400`); the ones that fail are removed from the list. Every
///    file is closed afterwards.
/// 2. For `*.esm` and then `*.esp`: every non-empty file found that
///    `GetListFile` does not know gets a new `TESFile` (closed again at
///    once). If a `<name>.NAM` file sits next to it, its first 0x100 bytes
///    become the file's summary (`+0x418`), bit 2 of its `m_Flags` is set
///    and `bCached` (`+0x428`) is set. If the file name is a DLC package
///    name, the matching bit of `cDLCFlags` is set. The file is then put in
///    `listFiles` before the first listed file that was written at the same
///    time or later (when both are masters or both are not), or before the
///    first non-master when it is a master; failing both it goes to the
///    end (`CompareFileTime` of the listed file's time with the new one's
///    is not negative). (`LIST_ADD` puts the item
///    before the node's current one.)
/// 3. Files of `listFiles` that are not on disk any more are removed from
///    the list and destroyed.
/// 4. `Update.bsa` is opened as an archive if `FileFinder::Exist` finds it.
/// 5. For each master file in the list, `GenIndexTable` is built and each of
///    its masters (`GetIndexFile`) that the chain from that node holds is
///    removed and added again at that node (the loop does not move on from
///    a node where one was moved). Then the index table of every file is
///    built and every file is closed (`00460360`).
pub fn fn_004624b0(e: &mut Engine, this: Ptr<TESDataHandler>, directory: Ptr) -> bool {
    let scope = scope_enter(e, 0x16, 0x1195);
    e.with_stack(SCAN_FRAME_SIZE, |e, frame| {
        scan_data_directory(e, this, directory, frame.addr());
    });
    scope_leave(e, scope);
    true
}

/// The body of `004624b0`; `frame` is the block of its locals.
fn scan_data_directory(e: &mut Engine, this: Ptr<TESDataHandler>, directory: Ptr, frame: u32) {
    let search_directory = frame + SCAN_SEARCH_DIRECTORY;
    let find_path = frame + SCAN_FIND_PATH;
    let find_data = frame + SCAN_FIND_DATA;
    let file_slot = frame + SCAN_FILE_SLOT;
    e.call(STRING_COPY_S, &args![search_directory, 0x104u32, directory]);

    // 1. Reopen the files already listed; drop the ones that fail.
    let mut node = fn_0045dfc0(e, this).addr();
    while node != 0 {
        let file = list_item(e, node);
        if file != 0 {
            if e.call(FILE_OPEN_HEADER, &args![file]).u32() != 0 {
                e.call(LIST_REMOVE_HEAD, &args![node]);
            } else {
                node = e.call(LIST_NEXT, &args![node]).u32();
            }
            e.call(FILE_CLOSE, &args![file]);
        } else {
            node = e.call(LIST_NEXT, &args![node]).u32();
        }
    }

    // 2. The plugin files in the directory: masters first, then the others.
    for pass in 0..2u32 {
        e.call(LSTRCPY_A, &args![find_path, search_directory]);
        let pattern = if pass == 0 { PATTERN_ESM } else { PATTERN_ESP };
        e.call(LSTRCAT_A, &args![find_path, pattern]);
        let handle = e
            .call(FIND_FIRST_FILE_A, &args![find_path, find_data])
            .u32();
        if handle == INVALID_HANDLE {
            continue;
        }
        loop {
            let size_high = e.mem.u32(find_data + FIND_DATA_SIZE_HIGH);
            let size_low = e.mem.u32(find_data + FIND_DATA_SIZE_LOW);
            if size_high != 0 || size_low != 0 {
                let known = e
                    .call(GET_LIST_FILE, &args![this, find_data + FIND_DATA_FILE_NAME])
                    .u32();
                if known == 0 {
                    add_plugin_file(e, this, directory, frame);
                }
            }
            if e.call(FIND_NEXT_FILE_A, &args![handle, find_data]).u32() == 0 {
                break;
            }
        }
        e.call(FIND_CLOSE, &args![handle]);
    }

    // 3. Drop the listed files that are not on disk any more.
    let mut node = fn_0045dfc0(e, this).addr();
    let mut last = 0;
    while node != 0 && list_item(e, node) != 0 {
        let file = list_item(e, node);
        e.mem.set_u32(file_slot, file);
        let path = fn_00462e80(e, Ptr::new(file));
        e.call(LSTRCPY_A, &args![find_path, path]);
        let name = e.call(FILE_NAME, &args![file]).u32();
        e.call(LSTRCAT_A, &args![find_path, name]);
        let handle = e
            .call(FIND_FIRST_FILE_A, &args![find_path, find_data])
            .u32();
        if handle == INVALID_HANDLE {
            if e.call(LIST_NEXT, &args![node]).u32() != 0 || last == 0 {
                e.call(LIST_REMOVE_HEAD, &args![node]);
            } else {
                e.call(LIST_REMOVE_ITEM, &args![last, file_slot]);
                node = e.call(LIST_NEXT, &args![last]).u32();
            }
            if file != 0 {
                fn_004601a0(e, Ptr::new(file), 1);
            }
        } else {
            last = node;
            node = e.call(LIST_NEXT, &args![node]).u32();
            e.call(FIND_CLOSE, &args![handle]);
        }
    }

    // 4. The update archive.
    let archive_path = frame + SCAN_ARCHIVE_PATH;
    if e.call(
        FILE_FINDER_EXIST,
        &args![UPDATE_BSA, archive_path, 1u32, 0xffff_ffffu32],
    )
    .u32()
        != 0
    {
        e.call(OPEN_ARCHIVE, &args![UPDATE_BSA, 0u32, 0u32]);
    }

    // 5. Masters first: move each master of a master file up, then build the
    // index tables and close every file.
    let other_slot = frame + SCAN_OTHER_SLOT;
    let mut node = fn_0045dfc0(e, this).addr();
    while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
        let mut moved = false;
        let file = list_item(e, node);
        if e.call(FILE_GET_MASTER, &args![file]).bool() {
            let files = fn_0045dfc0(e, this);
            e.call(FILE_GEN_INDEX_TABLE, &args![file, files, 0u32]);
            let mut index = 0;
            while index < e.call(FILE_INDEX_COUNT, &args![file]).u32() {
                let master = e.call(FILE_GET_INDEX_FILE, &args![file, index + 1]).u32();
                e.mem.set_u32(other_slot, master);
                if master != 0 && e.call(LIST_CONTAINS, &args![node, other_slot]).bool() {
                    e.call(LIST_REMOVE_ITEM, &args![node, other_slot]);
                    e.call(LIST_ADD, &args![node, other_slot]);
                    moved = true;
                }
                index += 1;
            }
        }
        if !moved {
            node = e.call(LIST_NEXT, &args![node]).u32();
        }
    }
    let mut node = fn_0045dfc0(e, this).addr();
    while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
        let file = list_item(e, node);
        let files = fn_0045dfc0(e, this);
        e.call(FILE_GEN_INDEX_TABLE, &args![file, files, 0u32]);
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    fn_00460360(e, this);
}

/// Part 2 of `004624b0` for one file found by `FindFirstFileA` /
/// `FindNextFileA` (its data is at `frame + SCAN_FIND_DATA`): creates the
/// `TESFile`, reads its `.NAM` summary, marks DLC packages and puts the
/// file in `listFiles`.
fn add_plugin_file(e: &mut Engine, this: Ptr<TESDataHandler>, directory: Ptr, frame: u32) {
    let find_data = frame + SCAN_FIND_DATA;
    let file_name = find_data + FIND_DATA_FILE_NAME;
    let base_name = frame + SCAN_BASE_NAME;
    let nam_path = frame + SCAN_NAM_PATH;
    let summary = frame + SCAN_SUMMARY;
    let stream = frame + SCAN_STREAM;
    let file_slot = frame + SCAN_FILE_SLOT;

    let block = e.call(OPERATOR_NEW, &args![0x42cu32]).u32();
    let file = if block != 0 {
        e.call(FILE_CONSTRUCT, &args![block, directory, file_name, 0u32])
            .u32()
    } else {
        0
    };
    e.mem.set_u32(file_slot, file);
    e.call(FILE_CLOSE, &args![file]);
    fn_00462d40(
        e,
        Ptr::new(file_name),
        Ptr::NULL,
        0,
        Ptr::NULL,
        0,
        Ptr::new(base_name),
        0x104,
        Ptr::new(frame + SCAN_EXTENSION),
        0x10,
    );
    e.call(
        FORMAT_S,
        &args![nam_path, 0x104u32, NAM_PATH_FORMAT, directory, base_name],
    );
    e.call(
        BSFILE_CONSTRUCT,
        &args![stream, nam_path, 0u32, 0x100u32, 1u32],
    );
    if e.call(BSFILE_EXIST, &args![stream]).bool() && e.call(BSFILE_FLAG_2C, &args![stream]).bool()
    {
        let length = fn_00462d80(e, Ptr::new(stream), Ptr::new(summary), 0x100);
        e.mem.set_u8(summary + length, 0);
        fn_00462ec0(e, Ptr::new(file), Ptr::new(summary));
        e.call(FILE_SET_FLAG_BIT_2, &args![file, 1u32]);
        fn_00462e60(e, Ptr::new(file), 1);
    }
    let package = e.call(IS_DLC_PACKAGE_NAME, &args![this, base_name]).u32();
    if package != 0 {
        let mask = 1u32.wrapping_shl(package.wrapping_sub(1));
        fn_00462e10(e, this, mask as u8, true);
    }

    // Put the file in the list: before the first listed file it is not
    // "after" (see `004624b0`), else at the end.
    let mut node = fn_0045dfc0(e, this).addr();
    let mut previous = 0;
    let mut inserted = false;
    while node != 0 && list_item(e, node) != 0 {
        let other = list_item(e, node);
        let compare = if e.call(FILE_GET_MASTER, &args![other]).bool()
            && e.call(FILE_GET_MASTER, &args![file]).bool()
        {
            true
        } else if e.call(FILE_GET_MASTER, &args![other]).bool() {
            false
        } else {
            !e.call(FILE_GET_MASTER, &args![file]).bool()
        };
        if compare {
            let other_info = fn_00462ea0(e, Ptr::new(other)).addr();
            let order = e
                .call(
                    COMPARE_FILE_TIME,
                    &args![
                        other_info + FIND_DATA_LAST_WRITE_TIME,
                        find_data + FIND_DATA_LAST_WRITE_TIME
                    ],
                )
                .i32();
            if order >= 0 {
                inserted = true;
                e.call(LIST_ADD, &args![node, file_slot]);
                break;
            }
        } else if e.call(FILE_GET_MASTER, &args![file]).bool() {
            inserted = true;
            e.call(LIST_ADD, &args![node, file_slot]);
            break;
        }
        previous = node;
        node = e.call(LIST_NEXT, &args![node]).u32();
    }
    if !inserted {
        if previous != 0 {
            e.call(LIST_APPEND, &args![previous, file_slot]);
        } else {
            let files = fn_0045dfc0(e, this);
            e.call(LIST_APPEND, &args![files, file_slot]);
        }
    }
    e.call(BSFILE_DESTRUCT, &args![stream]);
}

// Translated from 00462d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `_splitpath_s(path, drive, drive_size, dir, dir_size, name, name_size,
/// ext, ext_size)` (cdecl, nine words): passes them on (`NiFilename::Splitpath`
/// in the engine map) and returns its result.
#[allow(clippy::too_many_arguments)]
pub fn fn_00462d40(
    e: &mut Engine,
    path: Ptr,
    drive: Ptr,
    drive_size: u32,
    directory: Ptr,
    directory_size: u32,
    name: Ptr,
    name_size: u32,
    extension: Ptr,
    extension_size: u32,
) -> i32 {
    e.call(
        SPLIT_PATH_S,
        &args![
            path,
            drive,
            drive_size,
            directory,
            directory_size,
            name,
            name_size,
            extension,
            extension_size
        ],
    )
    .i32()
}

// Translated from 00462d80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads `size` bytes into `buffer` through `00462dc0` with one component of
/// size 1 (a local word holding 1, counted as 1 component). Returns the
/// number it answers.
pub fn fn_00462d80(e: &mut Engine, this: Ptr, buffer: Ptr, size: u32) -> u32 {
    e.with_stack(4, |e, component| {
        e.mem.set_u32(component.addr(), 1);
        fn_00462dc0(e, this, buffer, size, component, 1)
    })
}

// Translated from 00462dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The read of an `NiBinaryStream`: calls the function stored at `this + 8`
/// (`m_pfnRead`, Xbox PDB; cdecl, with `this` as its first argument) with
/// (`this`, `buffer`, `size`, `components`, `component_count`), adds the
/// result to `m_uiAbsoluteCurrentPos` (`this + 4`) and returns it.
pub fn fn_00462dc0(
    e: &mut Engine,
    this: Ptr,
    buffer: Ptr,
    size: u32,
    components: Ptr,
    component_count: u32,
) -> u32 {
    let read = e.mem.u32(this.addr() + 8);
    let count = e
        .call(
            read,
            &args![this, buffer, size, components, component_count],
        )
        .u32();
    let position = e.mem.u32(this.addr() + 4);
    e.mem.set_u32(this.addr() + 4, position.wrapping_add(count));
    count
}

// Translated from 00462e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set`) or clears the bits of `mask` in `cDLCFlags` (`this + 0`).
/// Returns the new byte (it is left in AL).
pub fn fn_00462e10(e: &mut Engine, this: Ptr<TESDataHandler>, mask: u8, set: bool) -> u8 {
    let flags = e.get(this, TESDataHandler::cDLCFlags);
    let flags = if set { flags | mask } else { flags & !mask };
    e.set(this, TESDataHandler::cDLCFlags, flags);
    flags
}

// Translated from 00462e60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte at `this + 0x428` of a `TESFile` (`bCached`, Xbox PDB).
pub fn fn_00462e60(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x428, value);
}

// Translated from 00462e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `this + 0x124` of a `TESFile`: its `m_Path` (Xbox PDB).
pub fn fn_00462e80(_e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(this.addr() + 0x124)
}

// Translated from 00462ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address `this + 0x29c` of a `TESFile`: its `m_FileInfo`, a
/// `WIN32_FIND_DATAA` (Xbox PDB).
pub fn fn_00462ea0(_e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(this.addr() + 0x29c)
}

// Translated from 00462ec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00438390(this + 0x418, text)`, the setter of the `BSStringT`
/// `cSummary` (Xbox PDB) of a `TESFile`, and returns its result.
pub fn fn_00462ec0(e: &mut Engine, this: Ptr, text: Ptr) -> Ptr {
    e.call(SUMMARY_SET, &args![this.addr() + 0x418, text]).ptr()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0040fbe0, error()),
        entry!(
            0x0045d270,
            tes_data_handler_tes_data_handler(Ptr<TESDataHandler>) -> Ptr<TESDataHandler>
        ),
        entry!(0x0045d930, fn_0045d930(Ptr)),
        entry!(0x0045d950, fn_0045d950(Ptr)),
        entry!(0x0045d970, fn_0045d970(Ptr<TESDataHandler>)),
        entry!(0x0045df10, fn_0045df10(Ptr, u32) -> Ptr),
        entry!(0x0045df40, fn_0045df40(Ptr, u32) -> Ptr),
        entry!(0x0045df70, fn_0045df70(Ptr, u32) -> Ptr),
        entry!(0x0045dfa0, fn_0045dfa0(Ptr<TESDataHandler>)),
        entry!(
            0x0045dfc0,
            fn_0045dfc0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x0045dfe0,
            tes_data_handler_clear_data(Ptr<TESDataHandler>) -> bool
        ),
        entry!(
            0x00460090,
            fn_00460090(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004600b0,
            fn_004600b0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004600d0,
            fn_004600d0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(0x004600f0, fn_004600f0(Ptr<TESDataHandler>) -> bool),
        entry!(0x00460110, fn_00460110(Ptr)),
        entry!(
            0x00460140,
            fn_00460140(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(0x00460160, fn_00460160()),
        entry!(0x00460170, fn_00460170(Ptr)),
        entry!(0x00460190, fn_00460190()),
        entry!(0x004601a0, fn_004601a0(Ptr, u32) -> Ptr),
        entry!(0x004601d0, tes_data_handler_load_form(Ptr, Ptr) -> bool),
        entry!(0x00460250, fn_00460250(Ptr) -> bool),
        entry!(
            0x00460270,
            tes_data_handler_save_form(Ptr<TESDataHandler>, Ptr, u8) -> bool
        ),
        entry!(0x00460340, fn_00460340(Ptr) -> bool),
        entry!(0x00460360, fn_00460360(Ptr<TESDataHandler>)),
        entry!(
            0x004603b0,
            tes_data_handler_add_form_to_data_handler(Ptr<TESDataHandler>, Ptr) -> bool
        ),
        entry!(
            0x00460fd0,
            fn_00460fd0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461010,
            fn_00461010(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461030,
            fn_00461030(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461050,
            fn_00461050(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461090,
            fn_00461090(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004610b0,
            fn_004610b0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461170,
            fn_00461170(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004611b0,
            fn_004611b0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004611d0,
            fn_004611d0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x004611f0,
            fn_004611f0(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461210,
            fn_00461210(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461230,
            fn_00461230(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461250,
            fn_00461250(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461270,
            fn_00461270(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461290,
            fn_00461290(Ptr<TESDataHandler>) -> Ptr<BSSimpleList>
        ),
        entry!(
            0x00461330,
            tes_data_handler_new_cell(Ptr<TESDataHandler>, u32, i32, i32, Ptr) -> Ptr
        ),
        entry!(0x004614e0, fn_004614e0(Ptr<TESDataHandler>)),
        entry!(0x00461560, fn_00461560(Ptr) -> bool),
        entry!(0x00461580, fn_00461580(Ptr, u32) -> bool),
        entry!(0x004615a0, fn_004615a0(Ptr<TESDataHandler>, u32) -> Ptr),
        entry!(
            0x004615d0,
            tes_data_handler_get_sound(Ptr<TESDataHandler>, u32) -> Ptr
        ),
        entry!(
            0x00461600,
            tes_data_handler_get_faction(Ptr<TESDataHandler>, u32) -> Ptr
        ),
        entry!(
            0x00461630,
            tes_data_handler_get_reputation(Ptr<TESDataHandler>, u32) -> Ptr
        ),
        entry!(0x00461660, fn_00461660(Ptr<TESDataHandler>, u32) -> Ptr),
        entry!(
            0x00461690,
            tes_data_handler_get_topic(Ptr<TESDataHandler>, u32) -> Ptr
        ),
        entry!(
            0x004616c0,
            tes_data_handler_get_sound_ov2(Ptr<TESDataHandler>, Ptr) -> Ptr
        ),
        entry!(
            0x00461700,
            tes_data_handler_get_global(Ptr<TESDataHandler>, Ptr) -> Ptr
        ),
        entry!(0x00461780, fn_00461780(Ptr<TESDataHandler>, u32) -> u32),
        entry!(0x004617b0, fn_004617b0(Ptr<TESDataHandler>, u32) -> u32),
        entry!(
            0x004617e0,
            tes_data_handler_get_addon_node(Ptr<TESDataHandler>, u32) -> Ptr
        ),
        entry!(
            0x00461820,
            tes_data_handler_add_addon_node(Ptr<TESDataHandler>, Ptr)
        ),
        entry!(0x00461930, fn_00461930(Ptr<TESDataHandler>, Ptr)),
        entry!(0x00461960, fn_00461960(Ptr<TESDataHandler>) -> u32),
        entry!(0x00461980, fn_00461980(Ptr<TESDataHandler>, u32) -> u32),
        entry!(0x004619b0, fn_004619b0(Ptr<TESDataHandler>)),
        entry!(
            0x00461ae0,
            tes_data_handler_get_cell_by_editor_id(Ptr<TESDataHandler>, Ptr) -> Ptr
        ),
        entry!(
            0x00461bc0,
            tes_data_handler_get_cell_from_world_coord(
                Ptr<TESDataHandler>,
                f32,
                f32,
                Ptr,
                u8,
            ) -> Ptr
        ),
        entry!(
            0x00461c20,
            fn_00461c20(Ptr<TESDataHandler>, i32, i32, Ptr, u8) -> Ptr
        ),
        entry!(
            0x00461cf0,
            tes_data_handler_get_ext_cell_data_from_file_by_editor_id(
                Ptr<TESDataHandler>,
                Ptr,
                Ptr,
                Ptr,
            ) -> Ptr
        ),
        entry!(0x00462230, fn_00462230(Ptr)),
        entry!(0x00462270, fn_00462270(Ptr) -> Ptr),
        entry!(
            0x00462290,
            tes_data_handler_unload_cell(Ptr<TESDataHandler>, Ptr)
        ),
        entry!(0x004623f0, fn_004623f0(Ptr, u8) -> u8),
        entry!(0x00462480, fn_00462480() -> bool),
        entry!(0x004624b0, fn_004624b0(Ptr<TESDataHandler>, Ptr) -> bool),
        entry!(
            0x00462d40,
            fn_00462d40(Ptr, Ptr, u32, Ptr, u32, Ptr, u32, Ptr, u32) -> i32
        ),
        entry!(0x00462d80, fn_00462d80(Ptr, Ptr, u32) -> u32),
        entry!(0x00462dc0, fn_00462dc0(Ptr, Ptr, u32, Ptr, u32) -> u32),
        entry!(0x00462e10, fn_00462e10(Ptr<TESDataHandler>, u8, bool) -> u8),
        entry!(0x00462e60, fn_00462e60(Ptr, u8)),
        entry!(0x00462e80, fn_00462e80(Ptr) -> Ptr),
        entry!(0x00462ea0, fn_00462ea0(Ptr) -> Ptr),
        entry!(0x00462ec0, fn_00462ec0(Ptr, Ptr) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// An engine with the pages holding the exe globals this unit reads and
    /// writes, and a zeroed `TESDataHandler`.
    fn engine() -> (Engine, Ptr<TESDataHandler>) {
        let mut e = Engine::new();
        for page in [
            0x011c_5000,
            0x011c_9000,
            0x011c_a000,
            0x011c_b000,
            0x011d_e000,
        ] {
            e.map(page, 0x1000);
        }
        let this = e.new_object::<TESDataHandler>();
        (e, this)
    }

    /// Registers a double that does nothing and returns 0 for each address.
    fn do_nothing(e: &mut Engine, addrs: &[u32]) {
        for &addr in addrs {
            e.register(addr, |_, _| Ret::default());
        }
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The argument words of every logged call to `addr`.
    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(called, _)| *called == addr)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// The logged calls (address, argument words) after the first (the call
    /// under test), without those to the addresses in `except`.
    fn log_without(e: &Engine, except: &[u32]) -> Vec<(u32, Vec<u32>)> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .skip(1)
            .filter(|(addr, _)| !except.contains(addr))
            .cloned()
            .collect()
    }

    /// A vtable at `vtable` with the given (byte offset, target) slots; every
    /// other slot points at an address nobody registered, so a call through
    /// it stops the test. Returns an object (a zeroed 0x80-byte block)
    /// using it.
    fn object_with(e: &mut Engine, vtable: u32, slots: &[(u32, u32)]) -> u32 {
        let mut table = vec![0x7fff_0000u32; 0x80];
        for &(offset, target) in slots {
            table[offset as usize / 4] = target;
        }
        e.put_vtable(vtable, &table);
        let object = e.mem.alloc(0x80);
        e.mem.set_u32(object, vtable);
        object
    }

    /// Registers a double at `target` that records (this, first argument)
    /// of each call in the returned log.
    fn recording_double(e: &mut Engine, target: u32) -> Rc<RefCell<Vec<(u32, u32)>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let shared = log.clone();
        e.register_double(target, move |_, a| {
            shared
                .borrow_mut()
                .push((a[0], a.get(1).copied().unwrap_or(0)));
            Ret::default()
        });
        log
    }

    // ---------------------------------------------------------------
    // The small functions.
    // ---------------------------------------------------------------

    #[test]
    fn error_does_nothing() {
        let mut e = Engine::new();
        start_log(&mut e);
        e.call(0x0040_fbe0, &args![]);
        assert_eq!(e.call_log.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn array_destructor_bodies_call_their_callees() {
        let (mut e, _) = engine();
        do_nothing(&mut e, &[CELL_ARRAY_DESTRUCT, ADDON_NODE_ARRAY_DESTRUCT]);
        start_log(&mut e);
        e.call(0x0045_d930, &args![0x1234u32]);
        e.call(0x0045_d950, &args![0x5678u32]);
        assert_eq!(calls(&e, CELL_ARRAY_DESTRUCT), vec![vec![0x1234]]);
        assert_eq!(calls(&e, ADDON_NODE_ARRAY_DESTRUCT), vec![vec![0x5678]]);
    }

    /// Checks a scalar deleting destructor: the body always runs, the block
    /// is freed only when bit 0 of the flags is set, `this` is returned.
    fn check_deleting_destructor(function: u32, body: u32) {
        let (mut e, _) = engine();
        do_nothing(&mut e, &[body, OPERATOR_DELETE]);
        start_log(&mut e);
        let result = e.call(function, &args![0x4000u32, 1u32]);
        assert_eq!(result.u32(), 0x4000);
        assert_eq!(calls(&e, body), vec![vec![0x4000]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0x4000]]);
        let result = e.call(function, &args![0x5000u32, 2u32]);
        assert_eq!(result.u32(), 0x5000);
        assert_eq!(calls(&e, body).len(), 2);
        assert_eq!(calls(&e, OPERATOR_DELETE).len(), 1, "bit 0 clear: no free");
    }

    #[test]
    fn object_list_deleting_destructor() {
        check_deleting_destructor(0x0045_df10, OBJECT_LIST_DESTRUCT);
    }

    #[test]
    fn idle_manager_deleting_destructor() {
        check_deleting_destructor(0x0045_df40, IDLE_MANAGER_DESTRUCT);
    }

    #[test]
    fn camera_path_manager_deleting_destructor() {
        check_deleting_destructor(0x0045_df70, CAMERA_PATH_MANAGER_DESTRUCT);
    }

    #[test]
    fn file_deleting_destructor() {
        check_deleting_destructor(0x0046_01a0, FILE_DESTRUCT);
    }

    #[test]
    fn files_list_destroy_gets_the_list_address() {
        let (mut e, this) = engine();
        do_nothing(&mut e, &[FILES_LIST_DESTROY]);
        start_log(&mut e);
        e.call(0x0045_dfa0, &args![this]);
        assert_eq!(
            calls(&e, FILES_LIST_DESTROY),
            vec![vec![this.addr() + 0x210]]
        );
    }

    /// Checks one accessor: it returns `this + offset`.
    fn check_accessor(addr: u32, offset: u32) {
        let (mut e, this) = engine();
        assert_eq!(e.call(addr, &args![this]).u32(), this.addr() + offset);
    }

    #[test]
    fn accessor_0045dfc0() {
        check_accessor(0x0045_dfc0, 0x210);
    }
    #[test]
    fn accessor_00460090() {
        check_accessor(0x0046_0090, 0x1b8);
    }
    #[test]
    fn accessor_004600b0() {
        check_accessor(0x0046_00b0, 0x1c8);
    }
    #[test]
    fn accessor_004600d0() {
        check_accessor(0x0046_00d0, 0x1d0);
    }
    #[test]
    fn accessor_00460140() {
        check_accessor(0x0046_0140, 0x10);
    }
    #[test]
    fn accessor_00460fd0() {
        check_accessor(0x0046_0fd0, 0x170);
    }
    #[test]
    fn accessor_00461010() {
        check_accessor(0x0046_1010, 0x140);
    }
    #[test]
    fn accessor_00461030() {
        check_accessor(0x0046_1030, 0x150);
    }
    #[test]
    fn accessor_00461050() {
        check_accessor(0x0046_1050, 0x168);
    }
    #[test]
    fn accessor_00461090() {
        check_accessor(0x0046_1090, 0x1a0);
    }
    #[test]
    fn accessor_004610b0() {
        check_accessor(0x0046_10b0, 0x120);
    }
    #[test]
    fn accessor_00461170() {
        check_accessor(0x0046_1170, 0xd8);
    }
    #[test]
    fn accessor_004611b0() {
        check_accessor(0x0046_11b0, 0xf0);
    }
    #[test]
    fn accessor_004611d0() {
        check_accessor(0x0046_11d0, 0xf8);
    }
    #[test]
    fn accessor_004611f0() {
        check_accessor(0x0046_11f0, 0x188);
    }
    #[test]
    fn accessor_00461210() {
        check_accessor(0x0046_1210, 0x198);
    }
    #[test]
    fn accessor_00461230() {
        check_accessor(0x0046_1230, 0x190);
    }
    #[test]
    fn accessor_00461250() {
        check_accessor(0x0046_1250, 0x1a8);
    }

    #[test]
    fn master_save_flag_is_read_from_0x618() {
        let (mut e, this) = engine();
        assert!(!e.call(0x0046_00f0, &args![this]).bool());
        e.mem.set_u8(this.addr() + 0x618, 1);
        assert!(e.call(0x0046_00f0, &args![this]).bool());
    }

    #[test]
    fn form_list_clear_gets_the_address_of_the_list_at_0x10() {
        let (mut e, _) = engine();
        do_nothing(&mut e, &[FORM_LIST_CLEAR]);
        start_log(&mut e);
        e.call(0x0046_0110, &args![0x3000u32]);
        assert_eq!(calls(&e, FORM_LIST_CLEAR), vec![vec![0x3010]]);
    }

    #[test]
    fn one_line_functions_clear_their_words() {
        let (mut e, _) = engine();
        e.set_global(FLAG_011CA830, 7u32);
        e.set_global(FLAG_011C9520, 9u32);
        e.call(0x0046_0160, &args![]);
        assert_eq!(e.global::<u32>(FLAG_011CA830), 0);
        assert_eq!(e.global::<u32>(FLAG_011C9520), 9);
        e.call(0x0046_0190, &args![]);
        assert_eq!(e.global::<u32>(FLAG_011C9520), 0);
        let tes = Ptr::<()>::new(e.mem.alloc(0x100));
        e.mem.set_u32(tes.addr() + 0x88, 5);
        e.mem.set_u32(tes.addr() + 0x8c, 6);
        e.call(0x0046_0170, &args![tes]);
        assert_eq!(e.mem.u32(tes.addr() + 0x88), 0);
        assert_eq!(e.mem.u32(tes.addr() + 0x8c), 6);
    }

    #[test]
    fn form_flag_getters_read_bits_0_and_1() {
        let (mut e, _) = engine();
        let form = Ptr::<()>::new(e.mem.alloc(0x100));
        for (flags, bit_0, bit_1) in [
            (0u32, false, false),
            (1, true, false),
            (2, false, true),
            (3, true, true),
            (0x20, false, false),
        ] {
            e.mem.set_u32(form.addr() + 8, flags);
            assert_eq!(e.call(0x0046_0250, &args![form]).bool(), bit_0);
            assert_eq!(e.call(0x0046_0340, &args![form]).bool(), bit_1);
        }
    }

    #[test]
    fn close_tes_runs_for_each_file_until_a_null_item() {
        let (mut e, this) = engine();
        // `listFiles` is the node at `this + 0x210`: file 0xa1, then a node
        // at 0x9100 with file 0xb2, then one at 0x9200 whose item is null.
        e.map(0x9000, 0x1000);
        let head = this.addr() + 0x210;
        let nodes: HashMap<u32, (u32, u32)> = HashMap::from([
            (head, (0xa1, 0x9100)),
            (0x9100, (0xb2, 0x9200)),
            (0x9200, (0, 0)),
        ]);
        let slot = e.mem.alloc(4);
        let table = nodes.clone();
        e.register_double(LIST_HEAD_ITEM, move |e, a| {
            e.mem.set_u32(slot, table[&a[0]].0);
            ret(slot)
        });
        e.register_double(LIST_NEXT, move |_, a| ret(nodes[&a[0]].1));
        do_nothing(&mut e, &[FILE_CLOSE]);
        start_log(&mut e);
        e.call(0x0046_0360, &args![this]);
        assert_eq!(calls(&e, FILE_CLOSE), vec![vec![0xa1], vec![0xb2]]);
    }

    // ---------------------------------------------------------------
    // Constructor and destructor.
    // ---------------------------------------------------------------

    /// The constructors of the owned objects: each returns its `this`.
    fn register_owned_constructors(e: &mut Engine) {
        for addr in [
            OBJECT_LIST_CONSTRUCT,
            REGION_LIST_CONSTRUCT,
            REGION_DATA_MANAGER_CONSTRUCT,
            IDLE_MANAGER_CONSTRUCT,
            CAMERA_PATH_MANAGER_CONSTRUCT,
        ] {
            e.register(addr, |_, a| ret(a[0]));
        }
    }

    #[test]
    fn constructor_builds_the_lists_and_the_owned_objects() {
        let (mut e, this) = engine();
        do_nothing(
            &mut e,
            &[
                LIST_CONSTRUCT,
                CELL_ARRAY_CONSTRUCT,
                ADDON_NODE_ARRAY_CONSTRUCT,
                BAD_FORM_LIST_CONSTRUCT,
                CELL_ARRAY_SET_GROW_BY,
            ],
        );
        register_owned_constructors(&mut e);
        // Garbage that the constructor must overwrite.
        e.mem.write(this.addr() + 0x21c, &[0xaa; 0x3fc]);
        for offset in [0x618, 0x619, 0x61a, 0x61b, 0x61c, 0x61d, 0x621, 0x622, 0] {
            e.mem.set_u8(this.addr() + offset, 0x55);
        }
        e.mem.set_u32(this.addr() + 0x62c, 0x1234);
        start_log(&mut e);

        let result = e.call(0x0045_d270, &args![this]);
        assert_eq!(result.u32(), this.addr());

        // 58 form lists in order, then the files list.
        let mut expected: Vec<Vec<u32>> = (0x08..=0x1d0)
            .step_by(8)
            .map(|offset| vec![this.addr() + offset])
            .collect();
        expected.push(vec![this.addr() + 0x210]);
        assert_eq!(expected.len(), 59);
        assert_eq!(calls(&e, LIST_CONSTRUCT), expected);
        assert_eq!(
            calls(&e, CELL_ARRAY_CONSTRUCT),
            vec![vec![this.addr() + 0x1dc, 0, 1]]
        );
        assert_eq!(
            calls(&e, ADDON_NODE_ARRAY_CONSTRUCT),
            vec![vec![this.addr() + 0x1ec, 0, 1]]
        );
        assert_eq!(
            calls(&e, BAD_FORM_LIST_CONSTRUCT),
            vec![vec![this.addr() + 0x1fc]]
        );
        assert_eq!(
            calls(&e, CELL_ARRAY_SET_GROW_BY),
            vec![vec![this.addr() + 0x1dc, 100]]
        );

        // The fields.
        assert_eq!(e.get(this, TESDataHandler::iNextID), 0x800);
        assert_eq!(e.get(this, TESDataHandler::iNumCompile), 0);
        assert!(e.get(this, TESDataHandler::pActiveFile).is_null());
        assert!(e.get(this, TESDataHandler::bHasDesiredFiles));
        for flag in [
            TESDataHandler::bMasterSave,
            TESDataHandler::bSaveLoadGame,
            TESDataHandler::bSaveLoad,
            TESDataHandler::bAutoSaving,
            TESDataHandler::bExportingPlugin,
            TESDataHandler::bClearingData,
            TESDataHandler::bDontRemoveIDs,
            TESDataHandler::bCheckingModels,
            TESDataHandler::bLoadingFiles,
        ] {
            assert!(!e.get(this, flag));
        }
        assert_eq!(e.get(this, TESDataHandler::cDLCFlags), 0);
        assert_eq!(e.get(this, TESDataHandler::ucGameSettingsLoadState), 0);
        assert!(e
            .mem
            .bytes(this.addr() + 0x21c, 0x3fc)
            .iter()
            .all(|&byte| byte == 0));
        assert!(e.get(this, TESDataHandler::pBarterContainer).is_null());
        assert!(e.get(this, TESDataHandler::pRecipeContainer).is_null());
        assert!(e.get(this, TESDataHandler::pSpotterShader).is_null());
        assert!(e.get(this, TESDataHandler::pItemDetectedShader).is_null());
        assert!(e.get(this, TESDataHandler::pCateyeMobileShader).is_null());

        // The owned objects, constructed in their fresh blocks.
        let object_list = e.get(this, TESDataHandler::pObjectList);
        let region_list = e.get(this, TESDataHandler::pRegionList);
        let region_data = e.get(this, TESDataHandler::pRegionDataManager);
        assert_eq!(
            calls(&e, OBJECT_LIST_CONSTRUCT),
            vec![vec![object_list.addr(), 1]]
        );
        assert_eq!(
            calls(&e, REGION_LIST_CONSTRUCT),
            vec![vec![region_list.addr(), 1]]
        );
        assert_eq!(
            calls(&e, REGION_DATA_MANAGER_CONSTRUCT),
            vec![vec![region_data.addr()]]
        );
        assert_eq!(e.mem.block_size(object_list.addr()), Some(0x10));
        assert_eq!(e.mem.block_size(region_list.addr()), Some(0x10));
        assert_eq!(e.mem.block_size(region_data.addr()), Some(8));
        let idle = e.global::<u32>(IDLE_MANAGER);
        let camera = e.global::<u32>(CAMERA_PATH_MANAGER);
        assert_eq!(calls(&e, IDLE_MANAGER_CONSTRUCT), vec![vec![idle]]);
        assert_eq!(calls(&e, CAMERA_PATH_MANAGER_CONSTRUCT), vec![vec![camera]]);
        assert_eq!(e.mem.block_size(idle), Some(0x28));
        assert_eq!(e.mem.block_size(camera), Some(0x28));
    }

    #[test]
    fn constructor_stores_null_when_an_allocation_fails() {
        let (mut e, this) = engine();
        do_nothing(
            &mut e,
            &[
                LIST_CONSTRUCT,
                CELL_ARRAY_CONSTRUCT,
                ADDON_NODE_ARRAY_CONSTRUCT,
                BAD_FORM_LIST_CONSTRUCT,
                CELL_ARRAY_SET_GROW_BY,
            ],
        );
        register_owned_constructors(&mut e);
        e.register(OPERATOR_NEW, |_, _| Ret::default());
        e.set_global(IDLE_MANAGER, 0x77u32);
        start_log(&mut e);
        e.call(0x0045_d270, &args![this]);
        assert!(e.get(this, TESDataHandler::pObjectList).is_null());
        assert!(e.get(this, TESDataHandler::pRegionList).is_null());
        assert!(e.get(this, TESDataHandler::pRegionDataManager).is_null());
        assert_eq!(e.global::<u32>(IDLE_MANAGER), 0);
        assert_eq!(e.global::<u32>(CAMERA_PATH_MANAGER), 0);
        for constructor in [
            OBJECT_LIST_CONSTRUCT,
            REGION_LIST_CONSTRUCT,
            REGION_DATA_MANAGER_CONSTRUCT,
            IDLE_MANAGER_CONSTRUCT,
            CAMERA_PATH_MANAGER_CONSTRUCT,
        ] {
            assert!(calls(&e, constructor).is_empty());
        }
    }

    /// Everything `~TESDataHandler` calls.
    const DESTRUCTOR_CALLEES: [u32; 11] = [
        INVENTORY_CHANGES_DELETE,
        FILES_LIST_DESTROY,
        OBJECT_LIST_DESTRUCT,
        IDLE_MANAGER_DESTRUCT,
        CAMERA_PATH_MANAGER_DESTRUCT,
        OPERATOR_DELETE,
        ALL_FORMS_CLEAR,
        LIST_DESTRUCT,
        BAD_FORM_LIST_DESTRUCT,
        CELL_ARRAY_DESTRUCT,
        ADDON_NODE_ARRAY_DESTRUCT,
    ];

    #[test]
    fn destructor_deletes_the_owned_objects_then_the_members() {
        let (mut e, this) = engine();
        do_nothing(&mut e, &DESTRUCTOR_CALLEES);
        let region_destroy = 0x7000_0000;
        let region_log = recording_double(&mut e, region_destroy);
        let region_list = object_with(&mut e, 0x7100_0000, &[(0, region_destroy)]);
        e.set(this, TESDataHandler::pBarterContainer, Ptr::new(0xb0b));
        e.set(this, TESDataHandler::pObjectList, Ptr::new(0x0b1));
        e.set(this, TESDataHandler::pRegionList, Ptr::new(region_list));
        e.set(this, TESDataHandler::pRegionDataManager, Ptr::new(0xd0d));
        e.set_global(IDLE_MANAGER, 0x1d1eu32);
        e.set_global(CAMERA_PATH_MANAGER, 0xca4u32);
        e.set_global(ALL_FORMS_MAP, 0xa11u32);
        start_log(&mut e);

        e.call(0x0045_d970, &args![this]);

        let mut expected: Vec<(u32, Vec<u32>)> = vec![
            (INVENTORY_CHANGES_DELETE, vec![0xb0b, 1]),
            (FILES_LIST_DESTROY, vec![this.addr() + 0x210]),
            (OBJECT_LIST_DESTRUCT, vec![0x0b1]),
            (OPERATOR_DELETE, vec![0x0b1]),
            (region_destroy, vec![region_list, 1]),
            (OPERATOR_DELETE, vec![0xd0d]),
            (IDLE_MANAGER_DESTRUCT, vec![0x1d1e]),
            (OPERATOR_DELETE, vec![0x1d1e]),
            (CAMERA_PATH_MANAGER_DESTRUCT, vec![0xca4]),
            (OPERATOR_DELETE, vec![0xca4]),
            (ALL_FORMS_CLEAR, vec![0xa11]),
            (LIST_DESTRUCT, vec![this.addr() + 0x210]),
            (BAD_FORM_LIST_DESTRUCT, vec![this.addr() + 0x1fc]),
            (ADDON_NODE_ARRAY_DESTRUCT, vec![this.addr() + 0x1ec]),
            (CELL_ARRAY_DESTRUCT, vec![this.addr() + 0x1dc]),
        ];
        for offset in (0x08..=0x1d0).rev().step_by(8) {
            expected.push((LIST_DESTRUCT, vec![this.addr() + offset]));
        }
        assert_eq!(e.call_log.clone().unwrap()[1..], expected[..]);
        assert_eq!(region_log.borrow().as_slice(), &[(region_list, 1)]);
        assert_eq!(e.global::<u32>(IDLE_MANAGER), 0);
        assert_eq!(e.global::<u32>(CAMERA_PATH_MANAGER), 0);
    }

    #[test]
    fn destructor_skips_the_objects_that_are_not_there() {
        let (mut e, this) = engine();
        do_nothing(&mut e, &DESTRUCTOR_CALLEES);
        start_log(&mut e);
        e.call(0x0045_d970, &args![this]);
        let log = e.call_log.clone().unwrap()[1..].to_vec();
        // Only the unconditional calls remain: the files list, the delete of
        // the (null) region data manager, and the member destructors.
        assert_eq!(log[0], (FILES_LIST_DESTROY, vec![this.addr() + 0x210]));
        assert_eq!(log[1], (OPERATOR_DELETE, vec![0]));
        assert_eq!(log[2], (LIST_DESTRUCT, vec![this.addr() + 0x210]));
        assert_eq!(log.len(), 64, "6 calls and the 58 list destructors");
        for skipped in [
            INVENTORY_CHANGES_DELETE,
            OBJECT_LIST_DESTRUCT,
            IDLE_MANAGER_DESTRUCT,
            CAMERA_PATH_MANAGER_DESTRUCT,
            ALL_FORMS_CLEAR,
        ] {
            assert!(calls(&e, skipped).is_empty());
        }
    }

    // ---------------------------------------------------------------
    // LoadForm and SaveForm.
    // ---------------------------------------------------------------

    /// A form with the flags `flags` whose vtable slots `Load`, `Save`,
    /// `SaveEdit` and `SetAltered` are doubles at `0x7000_0020`, `0x7000_0028`,
    /// `0x7000_0034` and `0x7000_00c8` that answer true (their calls show
    /// in the call log).
    fn form_with_slots(e: &mut Engine, flags: u32) -> u32 {
        let slots = [
            (FORM_VTABLE_LOAD, 0x7000_0020),
            (FORM_VTABLE_SAVE, 0x7000_0028),
            (FORM_VTABLE_SAVE_EDIT, 0x7000_0034),
            (FORM_VTABLE_SET_ALTERED, 0x7000_00c8),
        ];
        let form = object_with(e, 0x7100_1000, &slots);
        e.mem.set_u32(form + 8, flags);
        for (_, target) in slots {
            e.register(target, |_, _| ret(1));
        }
        form
    }

    #[test]
    fn load_form_sets_the_master_flag_and_marks_the_active_file() {
        let (mut e, _) = engine();
        e.register(FILE_GET_MASTER, |_, a| ret((a[0] == 0xf11e) as u32));
        e.register(FILE_GET_ACTIVE, |_, a| ret((a[0] == 0xac71) as u32));
        e.register(FORM_SET_FLAG_BIT_0, |_, _| Ret::default());

        // Flag bit 0 already set: the load result is returned, the flag is
        // set to 1 whatever the file.
        let form = form_with_slots(&mut e, 1);
        start_log(&mut e);
        assert!(e.call(0x0046_01d0, &args![form, 0x0f11u32]).bool());
        assert_eq!(calls(&e, 0x7000_0020), vec![vec![form, 0x0f11]]);
        assert_eq!(calls(&e, FORM_SET_FLAG_BIT_0), vec![vec![form, 1]]);
        assert!(calls(&e, FILE_GET_MASTER).is_empty());
        assert!(calls(&e, 0x7000_00c8).is_empty());

        // Flag clear and a master file: the flag becomes 1; active file:
        // `SetAltered(1)`.
        let form = form_with_slots(&mut e, 0);
        start_log(&mut e);
        e.call(0x0046_01d0, &args![form, 0xf11eu32]);
        assert_eq!(calls(&e, FORM_SET_FLAG_BIT_0), vec![vec![form, 1]]);
        assert!(calls(&e, 0x7000_00c8).is_empty());

        // Flag clear and a plain file that is the active one: flag 0, altered.
        start_log(&mut e);
        e.call(0x0046_01d0, &args![form, 0xac71u32]);
        assert_eq!(calls(&e, FORM_SET_FLAG_BIT_0), vec![vec![form, 0]]);
        assert_eq!(calls(&e, 0x7000_00c8), vec![vec![form, 1]]);
    }

    #[test]
    fn save_form_chooses_the_virtual_by_the_form_flags() {
        let (mut e, this) = engine();
        let active_file = 0xac71;
        e.register(FILE_GET_MASTER, |_, _| ret(0));
        e.register(
            FORM_HAS_FLAG_BIT_5,
            |e, a| ret(e.mem.u32(a[0] + 8) >> 5 & 1),
        );
        // The form's last file is stored at +0x30 of the test form.
        e.register(FORM_GET_FILE, |e, a| {
            assert_eq!(a[1] as i32, -1);
            ret(e.mem.u32(a[0] + 0x30))
        });

        // No active file: false, nothing called.
        let form = form_with_slots(&mut e, 0);
        e.mem.set_u32(form + 0x30, active_file);
        start_log(&mut e);
        assert!(!e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert!(calls(&e, 0x7000_0028).is_empty());

        e.set(this, TESDataHandler::pActiveFile, Ptr::new(active_file));
        // A plain form of the active file: `Save`.
        assert!(e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0028), vec![vec![form, active_file]]);
        // Bit 0: `SaveEdit`.
        e.mem.set_u32(form + 8, 1);
        assert!(e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0034), vec![vec![form, active_file]]);
        // Bit 5 (deleted) and not bit 0: skipped.
        e.mem.set_u32(form + 8, 0x20);
        assert!(!e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0028).len(), 1);
        // A form of another file is refused unless bit 1 is set.
        e.mem.set_u32(form + 8, 0);
        e.mem.set_u32(form + 0x30, 0x0f11);
        assert!(!e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        e.mem.set_u32(form + 8, 2);
        assert!(e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0028).len(), 2);
    }

    #[test]
    fn save_form_refuses_forms_of_a_master_active_file_without_bit_1() {
        let (mut e, this) = engine();
        e.register(FILE_GET_MASTER, |_, _| ret(1));
        e.register(FORM_HAS_FLAG_BIT_5, |_, _| ret(0));
        e.register(FORM_GET_FILE, |_, _| ret(0xac71));
        let form = form_with_slots(&mut e, 0);
        e.set(this, TESDataHandler::pActiveFile, Ptr::new(0xac71));
        start_log(&mut e);
        assert!(!e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        e.mem.set_u32(form + 8, 2);
        assert!(e.call(0x0046_0270, &args![this, form, 0u32]).bool());
        assert_eq!(calls(&e, 0x7000_0028), vec![vec![form, 0xac71]]);
    }

    // ---------------------------------------------------------------
    // ClearData.
    // ---------------------------------------------------------------

    /// The offsets of the 58 lists in the order `ClearData` empties them.
    const CLEAR_ORDER: [u32; 58] = [
        0xc8, 0x190, 0x198, 0x48, 0x50, 0x58, 0x60, 0x68, 0x18, 0x30, 0x80, 0x88, 0x98, 0x90, 0xa0,
        0xb8, 0xa8, 0xb0, 0xe8, 0xf0, 0xf8, 0x100, 0x118, 0x108, 0x110, 0x10, 0xd0, 0x1b8, 0x1c8,
        0x1d0, 0xd8, 0x70, 0x1a8, 0x20, 0x28, 0x40, 0x38, 0x8, 0x120, 0x128, 0x1c0, 0x130, 0x1a0,
        0x138, 0x140, 0x148, 0x170, 0x178, 0x150, 0x158, 0x160, 0x168, 0x180, 0x78, 0x1b0, 0x188,
        0xe0, 0xc0,
    ];

    /// The callees of `ClearData` that the tests stand in for with doubles
    /// that do nothing.
    const CLEAR_CALLEES: [u32; 33] = [
        CLEAN_UP_BAD_FORMS,
        0x0086_3db0,
        0x0045_39a0,
        EMBEDDED_OBJECT_RESET,
        GARBAGE_COLLECTOR_CLEAR_ALL,
        0x004f_f190,
        0x0095_2f90,
        0x004c_1c40,
        0x0059_3210,
        0x0058_d710,
        0x0061_a270,
        ARRAY_RESET,
        OBJECT_LIST_CLEAR,
        ADDON_NODE_ARRAY_CLEAR,
        REGION_LIST_CLEAR,
        0x006c_09f0,
        0x0045_af00,
        0x0045_afb0,
        0x004f_6de0,
        0x005f_fd20,
        0x0060_0030,
        CAMERA_PATH_MANAGER_DESTROY_ROOT_PATHS,
        0x0040_8de0,
        0x0066_f110,
        DISABLE_WARNING_COUNT,
        LOG_MESSAGE,
        FILE_CLOSE_ACTIVE,
        FILE_DESTRUCT,
        OPERATOR_DELETE,
        FORM_LIST_CLEAR,
        MAP_SET_AT,
        0x7000_0128,
        0x7000_0130,
    ];

    /// The address the test forms' scalar deleting destructor has.
    const FORM_DESTROY: u32 = 0x7000_0010;

    /// What the list doubles keep: the items of each list (the head first)
    /// and the lists asked for their head item, in order.
    #[derive(Default)]
    struct ListModel {
        items: HashMap<u32, Vec<u32>>,
        visited: Vec<u32>,
    }

    /// Doubles for `GetHead` and `RemoveHead` over a [`ListModel`].
    fn install_list_model(e: &mut Engine) -> Rc<RefCell<ListModel>> {
        let model = Rc::new(RefCell::new(ListModel::default()));
        let slot = e.mem.alloc(4);
        let shared = model.clone();
        e.register_double(LIST_HEAD_ITEM, move |e, a| {
            let mut model = shared.borrow_mut();
            model.visited.push(a[0]);
            let head = model
                .items
                .get(&a[0])
                .and_then(|items| items.first().copied())
                .unwrap_or(0);
            e.mem.set_u32(slot, head);
            ret(slot)
        });
        let shared = model.clone();
        e.register_double(LIST_REMOVE_HEAD, move |_, a| {
            if let Some(items) = shared.borrow_mut().items.get_mut(&a[0]) {
                items.remove(0);
            }
            Ret::default()
        });
        model
    }

    /// An object whose destructor is [`FORM_DESTROY`].
    fn destroyable(e: &mut Engine) -> u32 {
        object_with(e, 0x7100_2000, &[(0x10, FORM_DESTROY)])
    }

    struct ClearSetup {
        e: Engine,
        this: Ptr<TESDataHandler>,
        model: Rc<RefCell<ListModel>>,
        /// (this, flag) of every call to a form's destructor.
        destroyed: Rc<RefCell<Vec<(u32, u32)>>>,
        tes: u32,
        player: u32,
    }

    /// An engine where `ClearData` can run: doubles for what it calls, the
    /// list model, the singletons it uses, and `TES::pAllForms` null.
    fn clear_setup() -> ClearSetup {
        let (mut e, this) = engine();
        do_nothing(&mut e, &CLEAR_CALLEES);
        e.register(TES_GET_EMBEDDED_OBJECT, |_, _| ret(0x1e1e));
        e.register(0x006c_0720, |_, _| ret(0x4e4));
        let model = install_list_model(&mut e);
        let destroyed = recording_double(&mut e, FORM_DESTROY);
        let tes = e.mem.alloc(0x100);
        e.mem.set_u32(tes + 0x88, 5);
        e.set_global(TES_SINGLETON, tes);
        let player = destroyable(&mut e);
        e.set_global(PLAYER_SINGLETON, player);
        e.set_global(OBJECT_011C54C4, 0x0c54c4u32);
        e.set_global(IDLE_MANAGER, 0x1d1eu32);
        e.set_global(CAMERA_PATH_MANAGER, 0xca4u32);
        e.set(this, TESDataHandler::pObjectList, Ptr::new(0x0b1));
        e.set(this, TESDataHandler::pRegionList, Ptr::new(0x0e6));
        ClearSetup {
            e,
            this,
            model,
            destroyed,
            tes,
            player,
        }
    }

    #[test]
    fn clear_data_empties_the_lists_in_the_games_order() {
        let ClearSetup {
            mut e,
            this,
            model,
            destroyed,
            tes,
            player,
        } = clear_setup();
        let cells = this.addr() + 0x1dc;
        let scripts = [destroyable(&mut e), destroyable(&mut e)];
        let cell_objects = [destroyable(&mut e), destroyable(&mut e)];
        let music = destroyable(&mut e);
        let media_set = destroyable(&mut e);
        let media_location = destroyable(&mut e);
        let anim = destroyable(&mut e);
        let topic_info = destroyable(&mut e);
        {
            let mut model = model.borrow_mut();
            let at = |offset: u32| this.addr() + offset;
            model.items.insert(at(0xc8), scripts.to_vec());
            model.items.insert(at(0x1b8), vec![music]);
            model.items.insert(at(0x1c8), vec![media_set]);
            model.items.insert(at(0x1d0), vec![media_location]);
            model.items.insert(at(0x1a0), vec![anim]);
            model.items.insert(at(0x110), vec![topic_info]);
        }
        // The cell array: two cells.
        let slots = e.mem.alloc(8);
        e.mem.set_u32(slots, cell_objects[0]);
        e.mem.set_u32(slots + 4, cell_objects[1]);
        e.register(ARRAY_SIZE, |_, _| ret(2));
        e.register_double(ARRAY_AT, move |_, a| ret(slots + 4 * a[1]));
        for flag in [FLAG_011CB96C, FLAG_011CA53C, 0x011c_a24c, 0x011c_a264] {
            e.set_global(flag, 7u32);
        }
        for word in CLEARED_WORDS {
            e.set_global(word, 7u32);
        }
        e.set_global(FLAG_011CA830, 7u32);
        e.set(this, TESDataHandler::bClearingData, false);
        for shader in [
            TESDataHandler::pSpotterShader,
            TESDataHandler::pItemDetectedShader,
            TESDataHandler::pCateyeMobileShader,
        ] {
            e.set(this, shader, Ptr::new(0x5a));
        }
        start_log(&mut e);

        assert!(e.call(0x0045_dfe0, &args![this]).bool());

        // Forms destroyed, in the order of the lists they were in.
        let expected_destroyed: Vec<(u32, u32)> = [
            scripts[0],
            scripts[1],
            cell_objects[0],
            cell_objects[1],
            music,
            media_set,
            media_location,
            player,
            anim,
        ]
        .iter()
        .map(|&form| (form, 1))
        .collect();
        assert_eq!(destroyed.borrow().as_slice(), expected_destroyed.as_slice());
        assert_eq!(
            e.global::<u32>(PLAYER_SINGLETON),
            0,
            "the player is released"
        );

        // The lists were visited in the game's order, each until empty;
        // `listTopicInfos` was emptied without destroying its item.
        let mut visited = model.borrow().visited.clone();
        visited.dedup();
        let expected_order: Vec<u32> = CLEAR_ORDER.iter().map(|o| this.addr() + o).collect();
        assert_eq!(visited, expected_order);
        assert!(model.borrow().items.values().all(Vec::is_empty));

        // Every other call, in order.
        let idle = 0x1d1e;
        let camera = 0xca4;
        let other_calls = log_without(&e, &[LIST_HEAD_ITEM, LIST_REMOVE_HEAD, FORM_DESTROY]);
        let expected: Vec<(u32, Vec<u32>)> = vec![
            (CLEAN_UP_BAD_FORMS, vec![this.addr()]),
            (0x0086_3db0, vec![0x0c54c4]),
            (0x0045_39a0, vec![tes, 0, 0]),
            (TES_GET_EMBEDDED_OBJECT, vec![tes]),
            (EMBEDDED_OBJECT_RESET, vec![0x1e1e]),
            (GARBAGE_COLLECTOR_CLEAR_ALL, vec![0]),
            (0x004f_f190, vec![]),
            (0x0095_2f90, vec![player]),
            (0x004c_1c40, vec![]),
            (0x0059_3210, vec![]),
            (0x0058_d710, vec![]),
            (0x0061_a270, vec![]),
            (ARRAY_SIZE, vec![cells]),
            (ARRAY_AT, vec![cells, 0]),
            (ARRAY_AT, vec![cells, 1]),
            (ARRAY_RESET, vec![cells, 0]),
            (GARBAGE_COLLECTOR_CLEAR_ALL, vec![0]),
            (OBJECT_LIST_CLEAR, vec![0x0b1]),
            (ADDON_NODE_ARRAY_CLEAR, vec![this.addr() + 0x1ec]),
            (REGION_LIST_CLEAR, vec![0x0e6]),
            (0x006c_0720, vec![]),
            (0x006c_09f0, vec![0x4e4]),
            (0x0045_af00, vec![tes]),
            (0x005f_fd20, vec![idle]),
            (CAMERA_PATH_MANAGER_DESTROY_ROOT_PATHS, vec![camera]),
            (0x0040_8de0, vec![]),
            (0x0066_f110, vec![]),
        ];
        assert_eq!(other_calls, expected);

        // State.
        assert_eq!(e.mem.u32(tes + 0x88), 0);
        assert!(!e.get(this, TESDataHandler::bClearingData));
        for shader in [
            TESDataHandler::pSpotterShader,
            TESDataHandler::pItemDetectedShader,
            TESDataHandler::pCateyeMobileShader,
        ] {
            assert!(e.get(this, shader).is_null());
        }
        for word in CLEARED_WORDS {
            assert_eq!(e.global::<u32>(word), 0, "{word:08x}");
        }
        for flag in [FLAG_011CB96C, FLAG_011CA53C, FLAG_011CA830] {
            assert_eq!(e.global::<u32>(flag), 0, "{flag:08x}");
        }
        // Not touched.
        assert_eq!(e.global::<u32>(0x011c_a24c), 7);
        assert_eq!(e.global::<u32>(0x011c_a264), 7);
    }

    #[test]
    fn clear_data_picks_the_idle_manager_reset_and_the_navmesh_map() {
        let ClearSetup { mut e, this, .. } = clear_setup();
        e.register(ARRAY_SIZE, |_, _| ret(0));
        // A navmesh info map is set: it is cleared with `SetNavMeshInfoMap(0)`.
        e.register(0x0045_af00, |_, _| ret(0x9));
        // An object that answers yes: the other idle-manager reset.
        e.set_global(OBJECT_011DEA0C, 0x0b0bu32);
        e.register(0x004f_6de0, |_, _| ret(1));
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        let tes = e.global::<u32>(TES_SINGLETON);
        assert_eq!(calls(&e, 0x0045_afb0), vec![vec![tes, 0]]);
        assert_eq!(calls(&e, 0x004f_6de0), vec![vec![0x0b0b]]);
        assert_eq!(calls(&e, 0x0060_0030), vec![vec![0x1d1e]]);
        assert!(calls(&e, 0x005f_fd20).is_empty());

        // The object answers no: the first reset.
        e.register(0x004f_6de0, |_, _| ret(0));
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert_eq!(calls(&e, 0x005f_fd20), vec![vec![0x1d1e]]);
        assert!(calls(&e, 0x0060_0030).is_empty());
    }

    /// Doubles for the callees of the leak report, with the forms in
    /// `entries` (key, form) in `TESForm::pAllForms`.
    fn install_all_forms(e: &mut Engine, entries: Vec<(u32, u32)>) {
        e.set_global(ALL_FORMS_MAP, 0xa11u32);
        e.register(MAP_FIRST_POSITION, |_, _| ret(1));
        e.register_double(MAP_GET_NEXT, move |e, a| {
            let position = e.mem.u32(a[1]) as usize;
            let (key, form) = entries[position - 1];
            e.mem.set_u32(a[2], key);
            e.mem.set_u32(a[3], form);
            e.mem.set_u32(
                a[1],
                if position < entries.len() {
                    position as u32 + 1
                } else {
                    0
                },
            );
            Ret::default()
        });
        // The form type is the byte at +4, its id the word at +0xC, its
        // last file the word at +0x24.
        e.register(FORM_GET_TYPE, |e, a| ret(e.mem.u8(a[0] + 4) as u32));
        e.register(FORM_GET_ID, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e.register(FORM_GET_FILE, |e, a| {
            assert_eq!(a[1] as i32, -1);
            ret(e.mem.u32(a[0] + 0x24))
        });
        e.register(FILE_NAME, |_, a| ret(a[0] + 0x20));
        e.register(IDENTITY, |_, a| ret(a[0]));
        e.register(FORM_GET_TYPE_NAME, |_, _| ret(0x7e57));
        e.register(0x7000_0130, |_, _| ret(0xed17));
    }

    /// A form of `form_type` with `id`, last in `file`.
    fn leak_form(e: &mut Engine, form_type: u8, id: u32, file: u32) -> u32 {
        let form = object_with(
            e,
            0x7100_3000,
            &[
                (FORM_VTABLE_SLOT_128, 0x7000_0128),
                (FORM_VTABLE_SLOT_130, 0x7000_0130),
            ],
        );
        e.mem.set_u8(form + 4, form_type);
        e.mem.set_u32(form + 0xc, id);
        e.mem.set_u32(form + 0x24, file);
        form
    }

    #[test]
    fn clear_data_reports_the_forms_that_were_not_freed() {
        let ClearSetup { mut e, this, .. } = clear_setup();
        e.register(ARRAY_SIZE, |_, _| ret(0));
        let file = e.mem.alloc(0x40);
        let game_setting = leak_form(&mut e, 3, 0x1111, 0);
        let leaked = leak_form(&mut e, 5, 0x2222, file);
        let leaked_without_file = leak_form(&mut e, 6, 0x3333, 0);
        install_all_forms(
            &mut e,
            vec![
                (0x100, game_setting),
                (0x101, leaked),
                (0x102, 0),
                (0x103, leaked_without_file),
            ],
        );
        start_log(&mut e);

        e.call(0x0045_dfe0, &args![this]);

        // Warnings are switched off around the walk.
        assert_eq!(calls(&e, DISABLE_WARNING_COUNT), vec![vec![1], vec![0]]);
        // The type 3 form is released, not reported.
        assert_eq!(calls(&e, FORM_LIST_CLEAR), vec![vec![game_setting + 0x10]]);
        assert_eq!(calls(&e, 0x7000_0128), vec![vec![game_setting, 0, 1]]);
        // The others are reported and dropped from the map.
        assert_eq!(
            calls(&e, LOG_MESSAGE),
            vec![
                vec![FORM_LEAKED_FORMAT, 0xed17, 0x2222, 0x7e57, file + 0x20],
                vec![
                    FORM_LEAKED_FORMAT,
                    0xed17,
                    0x3333,
                    0x7e57,
                    UNKNOWN_FILE_NAME
                ],
                vec![FORMS_LEAKED_MESSAGE],
            ]
        );
        assert_eq!(
            calls(&e, MAP_SET_AT),
            vec![vec![0xa11, 0x101, 0], vec![0xa11, 0x103, 0]]
        );
    }

    #[test]
    fn clear_data_is_quiet_when_only_game_settings_remain() {
        let ClearSetup { mut e, this, .. } = clear_setup();
        e.register(ARRAY_SIZE, |_, _| ret(0));
        let game_setting = leak_form(&mut e, 3, 0x1111, 0);
        install_all_forms(&mut e, vec![(0x100, game_setting)]);
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert!(calls(&e, LOG_MESSAGE).is_empty());
        assert!(calls(&e, MAP_SET_AT).is_empty());
        assert_eq!(calls(&e, DISABLE_WARNING_COUNT), vec![vec![1], vec![0]]);
    }

    #[test]
    fn clear_data_forgets_the_files_and_destroys_the_active_file() {
        let ClearSetup { mut e, this, .. } = clear_setup();
        e.register(ARRAY_SIZE, |_, _| ret(0));
        let index = this.addr() + 0x21c;
        let prepare = |e: &mut Engine| {
            e.set(this, TESDataHandler::iNumCompile, 3);
            for slot in 0..4 {
                e.mem.set_u32(index + 4 * slot, 9);
            }
            e.set(this, TESDataHandler::pActiveFile, Ptr::new(0xf11e));
            e.set(this, TESDataHandler::iNextID, 0x1234);
        };

        // A normal session: the active file is closed and destroyed.
        prepare(&mut e);
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert_eq!(e.get(this, TESDataHandler::iNumCompile), 0);
        for slot in 0..3 {
            assert_eq!(e.mem.u32(index + 4 * slot), 0);
        }
        assert_eq!(
            e.mem.u32(index + 12),
            9,
            "only iNumCompile slots are cleared"
        );
        assert_eq!(calls(&e, FILE_CLOSE_ACTIVE), vec![vec![0xf11e, 0]]);
        assert_eq!(calls(&e, FILE_DESTRUCT), vec![vec![0xf11e]]);
        assert_eq!(calls(&e, OPERATOR_DELETE), vec![vec![0xf11e]]);
        assert!(e.get(this, TESDataHandler::pActiveFile).is_null());
        assert_eq!(e.get(this, TESDataHandler::iNextID), 0x800);

        // A master save is not closed first.
        prepare(&mut e);
        e.set(this, TESDataHandler::bMasterSave, true);
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert!(calls(&e, FILE_CLOSE_ACTIVE).is_empty());
        assert_eq!(calls(&e, FILE_DESTRUCT), vec![vec![0xf11e]]);

        // While loading a save the active file is left alone.
        prepare(&mut e);
        e.set(this, TESDataHandler::bSaveLoadGame, true);
        start_log(&mut e);
        e.call(0x0045_dfe0, &args![this]);
        assert!(calls(&e, FILE_DESTRUCT).is_empty());
        assert_eq!(e.get(this, TESDataHandler::pActiveFile).addr(), 0xf11e);
        assert_eq!(e.get(this, TESDataHandler::iNextID), 0x1234);
    }

    // ---------------------------------------------------------------
    // AddFormToDataHandler.
    // ---------------------------------------------------------------

    /// (form type, offset of its list) for the types whose list accessor is
    /// in this unit.
    const LOCAL_LIST_TYPES: [(u32, u32); 17] = [
        (0x05, 0x198),
        (0x0e, 0x0d8),
        (0x31, 0x188),
        (0x33, 0x140),
        (0x41, 0x010),
        (0x4a, 0x120),
        (0x4d, 0x1a0),
        (0x52, 0x170),
        (0x55, 0x190),
        (0x5a, 0x150),
        (0x5d, 0x0f0),
        (0x5e, 0x0f8),
        (0x62, 0x1a8),
        (0x66, 0x1b8),
        (0x6f, 0x1c8),
        (0x70, 0x1d0),
        (0x78, 0x168),
    ];

    /// (form type, accessor address) for the types whose list accessor is in
    /// another unit.
    const EXTERNAL_LIST_TYPES: [(u32, u32); 39] = [
        (0x06, 0x00461190),
        (0x07, 0x006130e0),
        (0x08, 0x00871a30),
        (0x09, 0x00624700),
        (0x0a, 0x00613790),
        (0x0b, 0x0043c490),
        (0x0c, 0x004ea950),
        (0x0d, 0x0045c650),
        (0x11, 0x006377e0),
        (0x12, 0x004610f0),
        (0x13, 0x0041d8a0),
        (0x14, 0x0087eaa0),
        (0x35, 0x00436aa0),
        (0x36, 0x00500940),
        (0x47, 0x00455600),
        (0x49, 0x00413f40),
        (0x4b, 0x00461070),
        (0x4e, 0x0045a730),
        (0x4f, 0x00874670),
        (0x51, 0x00460ff0),
        (0x53, 0x00891170),
        (0x54, 0x004610d0),
        (0x56, 0x00460fb0),
        (0x57, 0x0045a330),
        (0x5b, 0x00461110),
        (0x5f, 0x004a0d10),
        (0x61, 0x00461130),
        (0x63, 0x009d9f40),
        (0x65, 0x00461270),
        (0x68, 0x009c1a50),
        (0x6a, 0x004077e0),
        (0x6b, 0x00503650),
        (0x6d, 0x00984250),
        (0x6e, 0x00461290),
        (0x71, 0x00461150),
        (0x72, 0x00984230),
        (0x75, 0x00514f30),
        (0x76, 0x00506390),
        (0x77, 0x0062d2f0),
    ];

    /// What the list append recorded: the list and the item at the address
    /// it was given.
    type Appended = Rc<RefCell<Vec<(u32, u32)>>>;

    /// An engine with the form type reader and the list append stand-ins.
    fn add_engine() -> (Engine, Ptr<TESDataHandler>, Appended) {
        let (mut e, this) = engine();
        e.register(FORM_GET_TYPE, |e, a| ret(e.mem.u8(a[0] + 4) as u32));
        let appended: Appended = Rc::default();
        let shared = appended.clone();
        e.register_double(LIST_ADD, move |e, a| {
            shared.borrow_mut().push((a[0], e.mem.u32(a[1])));
            Ret::default()
        });
        (e, this, appended)
    }

    /// A form of `form_type` (a zeroed block with the type byte at +4).
    fn form_of_type(e: &mut Engine, form_type: u32) -> Ptr {
        let form = Ptr::<()>::new(e.mem.alloc(0x100));
        e.mem.set_u8(form.addr() + 4, form_type as u8);
        form
    }

    #[test]
    fn add_form_ignores_a_null_form() {
        let (mut e, this, appended) = add_engine();
        start_log(&mut e);
        assert!(!e.call(0x0046_03b0, &args![this, 0u32]).bool());
        assert!(e.call_log.as_ref().unwrap().len() == 1);
        assert!(appended.borrow().is_empty());
    }

    #[test]
    fn add_form_appends_to_the_list_of_its_type() {
        let (mut e, this, appended) = add_engine();
        for (form_type, offset) in LOCAL_LIST_TYPES {
            let form = form_of_type(&mut e, form_type);
            assert!(e.call(0x0046_03b0, &args![this, form]).bool());
            assert_eq!(
                appended.borrow().last().copied(),
                Some((this.addr() + offset, form.addr())),
                "type {form_type:#x}"
            );
        }
        for (form_type, accessor) in EXTERNAL_LIST_TYPES {
            let list = 0x5000_0000 + form_type * 8;
            e.register_double(accessor, move |_, a| {
                assert_ne!(a[0], 0);
                ret(list)
            });
            let form = form_of_type(&mut e, form_type);
            assert!(e.call(0x0046_03b0, &args![this, form]).bool());
            assert_eq!(
                appended.borrow().last().copied(),
                Some((list, form.addr())),
                "type {form_type:#x}"
            );
        }
        assert_eq!(appended.borrow().len(), 17 + 39);
    }

    #[test]
    fn add_form_of_type_0x37_uses_the_list_4_bytes_into_its_accessor_result() {
        let (mut e, this, appended) = add_engine();
        e.register(0x0041_69d0, |_, a| ret(a[0] + 0x300));
        let form = form_of_type(&mut e, 0x37);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(
            appended.borrow().as_slice(),
            &[(this.addr() + 0x304, form.addr())]
        );
    }

    #[test]
    fn add_form_of_type_0x39_goes_to_the_interior_cells_only_when_flagged() {
        let (mut e, this, _) = add_engine();
        let cells = this.addr() + 0x1dc;
        e.register(ARRAY_SIZE, |_, _| ret(3));
        e.register(0x0042_5fd0, |e, a| ret(e.mem.u8(a[0] + 0x24) as u32 & 1));
        let added: Appended = Rc::default();
        let shared = added.clone();
        e.register_double(ARRAY_SET_AT_GROW, move |e, a| {
            shared.borrow_mut().push((a[0], a[1]));
            shared.borrow_mut().push((a[0], e.mem.u32(a[2])));
            Ret::default()
        });
        let form = form_of_type(&mut e, 0x39);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert!(added.borrow().is_empty());
        e.mem.set_u8(form.addr() + 0x24, 1);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(
            added.borrow().as_slice(),
            &[(cells, 3), (cells, form.addr())]
        );
    }

    #[test]
    fn add_form_of_type_0x10_hands_effect_settings_to_their_registry() {
        let (mut e, this, appended) = add_engine();
        let cast_result = Rc::new(RefCell::new(0u32));
        let shared = cast_result.clone();
        e.register_double(DYNAMIC_CAST, move |_, _| ret(*shared.borrow()));
        do_nothing(&mut e, &[0x0040_9060]);
        let form = form_of_type(&mut e, 0x10);
        start_log(&mut e);
        // Not an `EffectSetting`: nothing more happens.
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert!(calls(&e, 0x0040_9060).is_empty());
        *cast_result.borrow_mut() = 0xe5;
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, 0x0040_9060), vec![vec![0xe5]]);
        assert_eq!(
            calls(&e, DYNAMIC_CAST).last().unwrap(),
            &vec![
                form.addr(),
                0,
                FORM_TYPE_DESCRIPTOR,
                EFFECT_SETTING_TYPE_DESCRIPTOR,
                0
            ]
        );
        assert!(appended.borrow().is_empty());
    }

    /// What the reference cases see and do.
    struct ReferenceWorld {
        cast_result: Rc<RefCell<u32>>,
        parent_cell: Rc<RefCell<u32>>,
        preferred_cell: Rc<RefCell<u32>>,
        found_cell: Rc<RefCell<u32>>,
        must_persist: Rc<RefCell<bool>>,
        save_flag: Rc<RefCell<bool>>,
        reference: u32,
    }

    /// Doubles for everything the reference cases call. Cells are blocks
    /// whose byte at +0x24 is the interior flag.
    fn reference_world(e: &mut Engine) -> ReferenceWorld {
        let mut world = ReferenceWorld {
            cast_result: Rc::default(),
            parent_cell: Rc::default(),
            preferred_cell: Rc::default(),
            found_cell: Rc::default(),
            must_persist: Rc::default(),
            save_flag: Rc::default(),
            reference: 0,
        };
        let position = e.mem.alloc(12);
        e.mem.set_f32(position, 1.5);
        e.mem.set_f32(position + 4, -2.5);
        e.register(0x0042_5fd0, |e, a| ret(e.mem.u8(a[0] + 0x24) as u32 & 1));
        let shared = world.cast_result.clone();
        e.register_double(DYNAMIC_CAST, move |_, _| ret(*shared.borrow()));
        let shared = world.parent_cell.clone();
        e.register_double(0x008d_6f30, move |_, _| ret(*shared.borrow()));
        let shared = world.preferred_cell.clone();
        e.register_double(0x005f_36f0, move |_, _| ret(*shared.borrow()));
        e.register(0x004f_d3e0, |_, a| {
            assert_eq!(a[1], 1);
            ret(0x77)
        });
        let shared = world.found_cell.clone();
        e.register_double(0x0046_1bc0, move |_, _| ret(*shared.borrow()));
        let shared = world.must_persist.clone();
        e.register_double(0x0056_5260, move |_, _| ret(*shared.borrow() as u32));
        let shared = world.save_flag.clone();
        e.register_double(0x0056_4e00, move |_, _| ret(*shared.borrow() as u32));
        do_nothing(e, &[0x0054_8230, 0x0056_5480, 0x0056_4eb0]);
        world.reference = object_with(e, 0x7100_4000, &[(0x1f4, 0x7000_01f4)]);
        e.register_double(0x7000_01f4, move |_, _| ret(position));
        world
    }

    fn cell(e: &mut Engine, interior: bool) -> u32 {
        let cell = e.mem.alloc(0x40);
        e.mem.set_u8(cell + 0x24, interior as u8);
        cell
    }

    #[test]
    fn add_reference_to_the_cell_it_is_in() {
        let (mut e, this, _) = add_engine();
        let world = reference_world(&mut e);
        let form = form_of_type(&mut e, 0x3a);
        *world.cast_result.borrow_mut() = world.reference;
        let parent = cell(&mut e, true);
        *world.parent_cell.borrow_mut() = parent;
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![
                form.addr(),
                0,
                FORM_TYPE_DESCRIPTOR,
                REFERENCE_TYPE_DESCRIPTOR,
                0
            ]]
        );
        assert_eq!(
            calls(&e, 0x0054_8230),
            vec![vec![parent, world.reference, 0]]
        );
        // Interior parent cell: no lookup by position.
        assert!(calls(&e, 0x0046_1bc0).is_empty());
        assert!(calls(&e, 0x005f_36f0).is_empty());
        // Neither flag set: persistence untouched.
        assert!(calls(&e, 0x0056_5480).is_empty());
        assert!(calls(&e, 0x0056_4eb0).is_empty());
    }

    #[test]
    fn add_reference_to_an_exterior_cell_looks_the_cell_up_by_position() {
        let (mut e, this, _) = add_engine();
        let world = reference_world(&mut e);
        // Every form type of the group.
        for form_type in [0x3a, 0x3d, 0x3e, 0x3f, 0x40, 0x69] {
            let form = form_of_type(&mut e, form_type);
            *world.cast_result.borrow_mut() = world.reference;
            let parent = cell(&mut e, false);
            *world.parent_cell.borrow_mut() = parent;
            let found = cell(&mut e, false);
            *world.found_cell.borrow_mut() = found;
            start_log(&mut e);
            assert!(e.call(0x0046_03b0, &args![this, form]).bool());
            // `GetCellFromWorldCoord(x, y, world space)`, x and y being the
            // first two floats of the position.
            assert_eq!(
                calls(&e, 0x0046_1bc0),
                vec![vec![
                    this.addr(),
                    1.5f32.to_bits(),
                    (-2.5f32).to_bits(),
                    0x77
                ]]
            );
            assert_eq!(
                calls(&e, 0x0054_8230),
                vec![vec![found, world.reference, 0]]
            );
        }
    }

    #[test]
    fn add_reference_without_a_parent_cell_tries_the_preferred_cell_then_the_position() {
        let (mut e, this, _) = add_engine();
        let world = reference_world(&mut e);
        let form = form_of_type(&mut e, 0x3a);
        *world.cast_result.borrow_mut() = world.reference;
        let preferred = cell(&mut e, true);
        *world.preferred_cell.borrow_mut() = preferred;
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(
            calls(&e, 0x0054_8230),
            vec![vec![preferred, world.reference, 0]]
        );
        assert!(calls(&e, 0x0046_1bc0).is_empty());

        // No preferred cell either: the lookup by position decides.
        *world.preferred_cell.borrow_mut() = 0;
        *world.found_cell.borrow_mut() = 0;
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, 0x0046_1bc0).len(), 1);
        // No cell at all: the reference is not added and its persistence
        // is left alone.
        assert!(calls(&e, 0x0054_8230).is_empty());
        assert!(calls(&e, 0x0056_5480).is_empty());
        assert!(calls(&e, 0x0056_4eb0).is_empty());
    }

    #[test]
    fn add_reference_updates_the_persistence_flags() {
        let (mut e, this, _) = add_engine();
        let world = reference_world(&mut e);
        let form = form_of_type(&mut e, 0x3a);
        *world.cast_result.borrow_mut() = world.reference;
        let parent = cell(&mut e, true);
        *world.parent_cell.borrow_mut() = parent;
        *world.must_persist.borrow_mut() = true;
        *world.save_flag.borrow_mut() = true;
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, 0x0056_5480), vec![vec![world.reference, 1]]);
        assert_eq!(calls(&e, 0x0056_4eb0), vec![vec![world.reference, 1]]);
    }

    #[test]
    fn add_reference_that_is_not_a_reference_does_nothing() {
        let (mut e, this, _) = add_engine();
        reference_world(&mut e);
        let form = form_of_type(&mut e, 0x40);
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, DYNAMIC_CAST).len(), 1);
        assert!(calls(&e, 0x008d_6f30).is_empty());
    }

    #[test]
    fn add_form_of_other_types_goes_to_the_object_list() {
        let (mut e, this, _) = add_engine();
        let cast_result = Rc::new(RefCell::new(0xb0u32));
        let shared = cast_result.clone();
        e.register_double(DYNAMIC_CAST, move |_, _| ret(*shared.borrow()));
        e.register(0x0046_12b0, |_, a| ret(0x4a3e_0000 + a[0]));
        do_nothing(&mut e, &[OBJECT_LIST_ADD, LOG_MESSAGE]);
        e.set(this, TESDataHandler::pObjectList, Ptr::new(0x0b1));
        // 0x03 has no case of its own.
        let form = form_of_type(&mut e, 0x03);
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        assert_eq!(calls(&e, OBJECT_LIST_ADD), vec![vec![0x0b1, 0xb0]]);
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![
                form.addr(),
                0,
                FORM_TYPE_DESCRIPTOR,
                OBJECT_TYPE_DESCRIPTOR,
                0
            ]]
        );
        // Not an object: logged and refused.
        *cast_result.borrow_mut() = 0;
        start_log(&mut e);
        assert!(!e.call(0x0046_03b0, &args![this, form]).bool());
        assert!(calls(&e, OBJECT_LIST_ADD).is_empty());
        assert_eq!(
            calls(&e, LOG_MESSAGE),
            vec![vec![UNKNOWN_FORM_TYPE_FORMAT, 0x4a3e_0003]]
        );
        // Types above the table end take the same path.
        let form = form_of_type(&mut e, 0xf0);
        assert!(!e.call(0x0046_03b0, &args![this, form]).bool());
    }

    #[test]
    fn add_form_of_type_0x58_adds_the_addon_node_first() {
        let (mut e, this, _) = add_engine();
        e.register(DYNAMIC_CAST, |_, a| ret(a[0] + 0x1000));
        do_nothing(&mut e, &[0x0046_1820, OBJECT_LIST_ADD]);
        e.set(this, TESDataHandler::pObjectList, Ptr::new(0x0b1));
        let form = form_of_type(&mut e, 0x58);
        start_log(&mut e);
        assert!(e.call(0x0046_03b0, &args![this, form]).bool());
        let log = e.call_log.clone().unwrap();
        let addon = log
            .iter()
            .position(|(addr, _)| *addr == 0x0046_1820)
            .unwrap();
        let add = log
            .iter()
            .position(|(addr, _)| *addr == OBJECT_LIST_ADD)
            .unwrap();
        assert!(addon < add, "the add-on node comes first");
        assert_eq!(log[addon].1, vec![this.addr(), form.addr()]);
        assert_eq!(log[add].1, vec![0x0b1, form.addr() + 0x1000]);
    }

    // ---------------------------------------------------------------
    // Session 2 (`00461270` to `00462ec0`).
    // ---------------------------------------------------------------

    /// `engine()` plus the pages of the globals session 2 reads; the handler
    /// singleton points at `this`.
    fn engine2() -> (Engine, Ptr<TESDataHandler>) {
        let (mut e, this) = engine();
        for page in [0x011c_3000, 0x011d_d000, 0x0118_7000] {
            e.map(page, 0x1000);
        }
        e.set_global(DATA_HANDLER_SINGLETON, this.addr());
        (e, this)
    }

    /// The accessors of the list nodes as the game has them: the item slot
    /// of a node is the node, the next node is the word at +4, a node is
    /// empty when both of its words are 0.
    fn real_lists(e: &mut Engine) {
        e.register(LIST_HEAD_ITEM, |_, a| ret(a[0]));
        e.register(LIST_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(LIST_IS_EMPTY, |e, a| {
            ret((e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32)
        });
    }

    /// Builds a list with its first node at `head` (inside the owner) and one
    /// allocated node per further item; returns the node addresses.
    fn build_list(e: &mut Engine, head: u32, items: &[u32]) -> Vec<u32> {
        let mut nodes = vec![head];
        for _ in 1..items.len() {
            nodes.push(e.mem.alloc(8));
        }
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(nodes[i], *item);
            let next = nodes.get(i + 1).copied().unwrap_or(0);
            e.mem.set_u32(nodes[i] + 4, next);
        }
        nodes
    }

    /// `_stricmp` as the game has it (the sign of the comparison).
    fn real_string_compare(e: &mut Engine) {
        e.register(STRING_COMPARE, |e, a| {
            let x = e.mem.cstr(a[0]).to_ascii_lowercase();
            let y = e.mem.cstr(a[1]).to_ascii_lowercase();
            ret(x.cmp(&y) as i32 as u32)
        });
    }

    /// A C string in memory.
    fn c_string(e: &mut Engine, text: &str) -> u32 {
        let at = e.mem.alloc(text.len() as u32 + 1);
        e.mem.set_cstr(at, text.as_bytes());
        at
    }

    const NAMED_VTABLE: u32 = 0x0200_0000;
    const NAMED_EDITOR_ID: u32 = 0x0200_0130;

    /// A form-like object: the form id at +0xC, the editor id's address at
    /// +0x10, which the virtual `0x130` returns.
    fn named_form(e: &mut Engine, name: &str, id: u32) -> u32 {
        if !e.mem.is_mapped(NAMED_VTABLE) {
            let mut table = vec![0x7fff_0000u32; 0x80];
            table[0x130 / 4] = NAMED_EDITOR_ID;
            e.put_vtable(NAMED_VTABLE, &table);
            e.register(NAMED_EDITOR_ID, |e, a| ret(e.mem.u32(a[0] + 0x10)));
        }
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(object, NAMED_VTABLE);
        e.mem.set_u32(object + 0xc, id);
        let text = c_string(e, name);
        e.mem.set_u32(object + 0x10, text);
        object
    }

    /// `ARRAY_SIZE` and `ARRAY_AT` over a table of words; returns the table.
    fn word_array(e: &mut Engine, words: &[u32]) -> u32 {
        let table = e.mem.alloc(4 * words.len().max(1) as u32);
        for (i, word) in words.iter().enumerate() {
            e.mem.set_u32(table + 4 * i as u32, *word);
        }
        let count = words.len() as u32;
        e.register_double(ARRAY_SIZE, move |_, _| ret(count));
        e.register_double(ARRAY_AT, move |_, a| ret(table + 4 * a[1]));
        table
    }

    /// The addresses of the logged calls after the first (the one under
    /// test).
    fn call_order(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .skip(1)
            .map(|(addr, _)| *addr)
            .collect()
    }

    #[test]
    fn accessor_00461270() {
        check_accessor(0x0046_1270, 0x1b0);
    }

    #[test]
    fn accessor_00461290() {
        check_accessor(0x0046_1290, 0x1c0);
    }

    // `NewCell`.

    /// The callees of `NewCell` with a cell whose virtual slots are doubles;
    /// returns the cell.
    fn new_cell_world(e: &mut Engine, add_cell_result: bool, created_id: u32) -> u32 {
        let cell = object_with(
            e,
            0x0210_0000,
            &[
                (0x10, 0x0210_0010),
                (0x88, 0x0210_0088),
                (0x128, 0x0210_0128),
                (0x134, 0x0210_0134),
            ],
        );
        do_nothing(
            e,
            &[
                0x0210_0010,
                0x0210_0088,
                0x0210_0128,
                0x0210_0134,
                SCOPE_ENTER,
                SCOPE_LEAVE,
                CELL_SET_INTERIOR,
                CELL_SET_FLAGS_BYTE,
                CELL_CREATE_CELL_DATA,
                CELL_SET_DATA_COORD,
            ],
        );
        e.register(OPERATOR_NEW, |_, _| ret(0x4400));
        e.register_double(CELL_CONSTRUCT, move |_, _| ret(cell));
        e.register(FORM_GET_ID, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e.register_double(GET_CREATED_EXTERIOR_CELL_FORM_ID, move |_, _| {
            ret(created_id)
        });
        e.register_double(WORLD_ADD_CELL, move |_, _| ret(add_cell_result as u32));
        e.set_global(SAVE_LOAD_GAME_SINGLETON, 0x5a5a);
        cell
    }

    #[test]
    fn new_cell_makes_and_adds_an_exterior_cell() {
        let (mut e, this) = engine2();
        let cell = new_cell_world(&mut e, true, 0x00ab_cdef);
        let world = e.mem.alloc(0x100);
        e.mem.set_u32(world + 0xc, 0x3c);
        start_log(&mut e);
        let result = e.call(0x0046_1330, &args![this, 0x1234u32, 5i32, -3i32, world]);
        assert_eq!(result.u32(), cell);
        assert_eq!(
            call_order(&e),
            vec![
                SCOPE_ENTER,
                OPERATOR_NEW,
                CELL_CONSTRUCT,
                FORM_GET_ID,
                GET_CREATED_EXTERIOR_CELL_FORM_ID,
                0x0210_0128,
                0x0210_0134,
                CELL_SET_INTERIOR,
                CELL_SET_FLAGS_BYTE,
                CELL_CREATE_CELL_DATA,
                CELL_SET_DATA_COORD,
                WORLD_ADD_CELL,
                0x0210_0088,
                SCOPE_LEAVE,
            ]
        );
        assert_eq!(
            &calls(&e, SCOPE_ENTER)[0][1..],
            &[0x1a, 1, SOURCE_FILE, 0x7dd]
        );
        assert_eq!(calls(&e, OPERATOR_NEW), vec![vec![0xe0]]);
        assert_eq!(calls(&e, CELL_CONSTRUCT), vec![vec![0x4400]]);
        assert_eq!(
            calls(&e, GET_CREATED_EXTERIOR_CELL_FORM_ID),
            vec![vec![0x5a5a, 0x3c, 5, -3i32 as u32]]
        );
        assert_eq!(calls(&e, 0x0210_0128), vec![vec![cell, 0x00ab_cdef, 1]]);
        assert_eq!(calls(&e, 0x0210_0134), vec![vec![cell, 0x1234]]);
        assert_eq!(calls(&e, CELL_SET_INTERIOR), vec![vec![cell, 0]]);
        assert_eq!(calls(&e, CELL_SET_FLAGS_BYTE), vec![vec![cell, 2]]);
        assert_eq!(
            calls(&e, CELL_SET_DATA_COORD),
            vec![vec![cell, 5, -3i32 as u32]]
        );
        assert_eq!(calls(&e, WORLD_ADD_CELL), vec![vec![world, cell]]);
    }

    #[test]
    fn new_cell_destroys_the_cell_the_world_refuses() {
        let (mut e, this) = engine2();
        let cell = new_cell_world(&mut e, false, 0);
        let world = e.mem.alloc(0x100);
        start_log(&mut e);
        // No saved form id and no editor id: neither virtual is called.
        let result = e.call(0x0046_1330, &args![this, 0u32, 1i32, 2i32, world]);
        assert_eq!(result.u32(), 0);
        assert!(calls(&e, 0x0210_0128).is_empty());
        assert!(calls(&e, 0x0210_0134).is_empty());
        assert!(calls(&e, 0x0210_0088).is_empty());
        assert_eq!(calls(&e, 0x0210_0010), vec![vec![cell, 1]]);
        assert_eq!(call_order(&e).last(), Some(&SCOPE_LEAVE));
    }

    #[test]
    fn new_cell_without_a_world_does_nothing() {
        let (mut e, this) = engine2();
        new_cell_world(&mut e, true, 7);
        start_log(&mut e);
        let result = e.call(0x0046_1330, &args![this, 0u32, 1i32, 2i32, 0u32]);
        assert_eq!(result.u32(), 0);
        assert!(call_order(&e).is_empty(), "no scope, no allocation");
    }

    // The object list walk.

    #[test]
    fn init_values_runs_for_npcs_with_the_flag() {
        let (mut e, this) = engine2();
        real_lists(&mut e);
        e.register(OBJECT_NEXT, |e, a| ret(e.mem.u32(a[0] + 0x20)));
        // this + 4 -> list object; list object + 4 -> first object; each
        // object's next at +0x20.
        let list_object = e.mem.alloc(0x10);
        e.set(this, TESDataHandler::pObjectList, Ptr::new(list_object));
        let objects: Vec<u32> = (0..3).map(|_| e.mem.alloc(0x100)).collect();
        e.mem.set_u32(list_object + 4, objects[0]);
        e.mem.set_u32(objects[0] + 0x20, objects[1]);
        e.mem.set_u32(objects[1] + 0x20, objects[2]);
        // The cast: object 0 is no NPC, 1 is one with the flag, 2 one without.
        let npcs: Vec<u32> = (0..2).map(|_| e.mem.alloc(0x100)).collect();
        e.mem.set_u32(npcs[0] + 0x30 + 4, 0x80);
        e.mem.set_u32(npcs[1] + 0x30 + 4, 0x7f);
        let table = HashMap::from([(objects[1], npcs[0]), (objects[2], npcs[1])]);
        e.register_double(DYNAMIC_CAST, move |_, a| {
            ret(table.get(&a[0]).copied().unwrap_or(0))
        });
        do_nothing(&mut e, &[NPC_INIT_VALUES]);
        start_log(&mut e);
        e.call(0x0046_14e0, &args![this]);
        assert_eq!(calls(&e, NPC_INIT_VALUES), vec![vec![npcs[0], 0]]);
        assert_eq!(
            calls(&e, DYNAMIC_CAST)[0],
            vec![
                objects[0],
                0,
                OBJECT_TYPE_DESCRIPTOR,
                NPC_TYPE_DESCRIPTOR,
                0
            ]
        );
        assert_eq!(calls(&e, DYNAMIC_CAST).len(), 3);
    }

    #[test]
    fn mask_test_reads_the_word_at_plus_4() {
        let mut e = Engine::new();
        let object = e.mem.alloc(0x10);
        e.mem.set_u32(object + 4, 0x0000_0180);
        assert!(e.call(0x0046_1580, &args![object, 0x100u32]).bool());
        assert!(e.call(0x0046_1580, &args![object, 0x180u32]).bool());
        assert!(!e.call(0x0046_1580, &args![object, 0x7fu32]).bool());
    }

    #[test]
    fn flag_0x80_test_uses_the_mask_0x80() {
        let mut e = Engine::new();
        let object = e.mem.alloc(0x10);
        e.mem.set_u32(object + 4, 0x0000_0180);
        assert!(e.call(0x0046_1560, &args![object]).bool());
        e.mem.set_u32(object + 4, 0x7f);
        assert!(!e.call(0x0046_1560, &args![object]).bool());
    }

    // The lookups by form id.

    /// A lookup by id: the form is fetched by id and cast to the class of
    /// `descriptor`.
    fn check_cast_lookup(function: u32, descriptor: u32) {
        let (mut e, this) = engine2();
        e.register_double(FORM_BY_ID, |_, _| ret(0x5000));
        e.register_double(DYNAMIC_CAST, |_, _| ret(0x5008));
        start_log(&mut e);
        let result = e.call(function, &args![this, 0x1234u32]);
        assert_eq!(result.u32(), 0x5008);
        assert_eq!(calls(&e, FORM_BY_ID), vec![vec![0x1234]]);
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![0x5000, 0, FORM_TYPE_DESCRIPTOR, descriptor, 0]]
        );
    }

    #[test]
    fn get_class_casts_to_tes_class() {
        check_cast_lookup(0x0046_15a0, CLASS_TYPE_DESCRIPTOR);
    }

    #[test]
    fn get_sound_casts_to_tes_sound() {
        check_cast_lookup(0x0046_15d0, SOUND_TYPE_DESCRIPTOR);
    }

    #[test]
    fn get_faction_casts_to_tes_faction() {
        check_cast_lookup(0x0046_1600, FACTION_TYPE_DESCRIPTOR);
    }

    #[test]
    fn get_reputation_casts_to_tes_reputation() {
        check_cast_lookup(0x0046_1630, REPUTATION_TYPE_DESCRIPTOR);
    }

    #[test]
    fn get_load_screen_type_casts_to_the_load_screen_type() {
        check_cast_lookup(0x0046_1660, LOAD_SCREEN_TYPE_TYPE_DESCRIPTOR);
    }

    #[test]
    fn get_topic_casts_to_tes_topic() {
        check_cast_lookup(0x0046_1690, TOPIC_TYPE_DESCRIPTOR);
    }

    #[test]
    fn get_sound_by_editor_id_wants_form_type_0xd() {
        let (mut e, this) = engine2();
        let name = c_string(&mut e, "snd");
        e.register_double(FORM_BY_EDITOR_ID, |_, _| ret(0x6000));
        e.register_double(FORM_GET_TYPE, |_, _| ret(0xd));
        start_log(&mut e);
        assert_eq!(e.call(0x0046_16c0, &args![this, name]).u32(), 0x6000);
        assert_eq!(calls(&e, FORM_BY_EDITOR_ID), vec![vec![name]]);
        assert_eq!(calls(&e, FORM_GET_TYPE), vec![vec![0x6000]]);
        // Another form type.
        e.register_double(FORM_GET_TYPE, |_, _| ret(0xe));
        assert_eq!(e.call(0x0046_16c0, &args![this, name]).u32(), 0);
        // No form at all: the type is not asked.
        e.register_double(FORM_BY_EDITOR_ID, |_, _| ret(0));
        start_log(&mut e);
        assert_eq!(e.call(0x0046_16c0, &args![this, name]).u32(), 0);
        assert!(calls(&e, FORM_GET_TYPE).is_empty());
    }

    // `GetGlobal`.

    #[test]
    fn get_global_finds_the_global_by_editor_id() {
        let (mut e, this) = engine2();
        real_lists(&mut e);
        real_string_compare(&mut e);
        let globals = named_form(&mut e, "GameYear", 1);
        let timescale = named_form(&mut e, "TimeScale", 2);
        build_list(
            &mut e,
            this.at(TESDataHandler::listGlobals).addr(),
            &[0, globals, timescale],
        );
        // The first node has a null item and is skipped.
        let name = c_string(&mut e, "timescale");
        assert_eq!(e.call(0x0046_1700, &args![this, name]).u32(), timescale);
        let name = c_string(&mut e, "Missing");
        assert_eq!(e.call(0x0046_1700, &args![this, name]).u32(), 0);
    }

    #[test]
    fn get_global_on_an_empty_list_is_null() {
        let (mut e, this) = engine2();
        real_lists(&mut e);
        start_log(&mut e);
        let name = c_string(&mut e, "x");
        assert_eq!(e.call(0x0046_1700, &args![this, name]).u32(), 0);
        assert_eq!(
            calls(&e, LIST_IS_EMPTY),
            vec![vec![this.at(TESDataHandler::listGlobals).addr()]]
        );
        assert!(calls(&e, LIST_NEXT).is_empty());
    }

    // The two topic list calls.

    #[test]
    fn topic_list_call_passes_the_topic_list() {
        let (mut e, this) = engine2();
        e.register_double(TOPIC_LIST_FUNCTION, |_, _| ret(0x11));
        start_log(&mut e);
        assert_eq!(e.call(0x0046_1780, &args![this, 0x77u32]).u32(), 0x11);
        assert_eq!(
            calls(&e, TOPIC_LIST_FUNCTION),
            vec![vec![0x77, this.addr() + 0x108, 1]]
        );
    }

    #[test]
    fn topic_info_list_call_passes_the_topic_info_list() {
        let (mut e, this) = engine2();
        e.register_double(TOPIC_INFO_LIST_FUNCTION, |_, _| ret(0x22));
        start_log(&mut e);
        assert_eq!(e.call(0x0046_17b0, &args![this, 0x88u32]).u32(), 0x22);
        assert_eq!(
            calls(&e, TOPIC_INFO_LIST_FUNCTION),
            vec![vec![0x88, this.addr() + 0x110]]
        );
    }

    // The add-on nodes.

    #[test]
    fn get_addon_node_is_bounded_by_the_array_size() {
        let (mut e, this) = engine2();
        word_array(&mut e, &[0xa1, 0xa2]);
        assert_eq!(e.call(0x0046_17e0, &args![this, 1u32]).u32(), 0xa2);
        assert_eq!(e.call(0x0046_17e0, &args![this, 2u32]).u32(), 0);
        assert_eq!(e.call(0x0046_17e0, &args![this, 0xffff_ffffu32]).u32(), 0);
    }

    /// The callees of `AddAddonNode`: node indexes at +0x50, form ids at
    /// +0xC, the array over `existing`, and recording doubles.
    fn addon_engine(existing: &[u32]) -> (Engine, Ptr<TESDataHandler>) {
        let (mut e, this) = engine2();
        e.register(ADDON_NODE_GET_INDEX, |e, a| ret(e.mem.u32(a[0] + 0x50)));
        e.register(FORM_GET_ID, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        word_array(&mut e, existing);
        do_nothing(
            &mut e,
            &[LOG_MESSAGE, ARRAY_SET_AT_GROW, ADDON_NODE_SET_INDEX],
        );
        e.register(ADDON_NODE_ARRAY_ADD, |_, _| ret(9));
        (e, this)
    }

    #[test]
    fn add_addon_node_stores_a_node_at_its_free_index() {
        let (mut e, this) = addon_engine(&[0, 0, 0]);
        let node = named_form(&mut e, "Node", 0x44);
        e.mem.set_u32(node + 0x50, 2);
        // The word the call gets holds the node.
        let stored = Rc::new(RefCell::new(vec![]));
        let shared = stored.clone();
        e.register_double(ARRAY_SET_AT_GROW, move |e, a| {
            shared.borrow_mut().push((a[0], a[1], e.mem.u32(a[2])));
            Ret::default()
        });
        start_log(&mut e);
        e.call(0x0046_1820, &args![this, node]);
        let nodes = this.at(TESDataHandler::arrayAddonNodes).addr();
        assert_eq!(*stored.borrow(), vec![(nodes, 2, node)]);
        assert!(calls(&e, LOG_MESSAGE).is_empty());
        assert!(calls(&e, ADDON_NODE_ARRAY_ADD).is_empty());
    }

    #[test]
    fn add_addon_node_remaps_a_node_whose_index_is_taken() {
        let (mut e, this) = addon_engine(&[0, 0]);
        let other = named_form(&mut e, "Other", 0x33);
        let node = named_form(&mut e, "Node", 0x44);
        // Index 1 of the array holds `other`.
        word_array(&mut e, &[0, other]);
        e.mem.set_u32(node + 0x50, 1);
        start_log(&mut e);
        e.call(0x0046_1820, &args![this, node]);
        let log = calls(&e, LOG_MESSAGE);
        assert_eq!(log.len(), 1);
        let node_name = e.mem.u32(node + 0x10);
        let other_name = e.mem.u32(other + 0x10);
        assert_eq!(
            log[0],
            vec![
                ADDON_INDEX_CLASH_FORMAT,
                0x44,
                node_name,
                1,
                0x33,
                other_name,
                0x44,
                node_name
            ]
        );
        // Remapped: appended, then given the index that answers.
        assert!(calls(&e, ARRAY_SET_AT_GROW).is_empty());
        let nodes = this.at(TESDataHandler::arrayAddonNodes).addr();
        let add = calls(&e, ADDON_NODE_ARRAY_ADD);
        assert_eq!(add.len(), 1);
        assert_eq!(add[0][0], nodes);
        assert_eq!(calls(&e, ADDON_NODE_SET_INDEX), vec![vec![node, 9]]);
    }

    #[test]
    fn add_addon_node_appends_a_node_without_an_index() {
        let (mut e, this) = addon_engine(&[0, 0]);
        let node = named_form(&mut e, "Node", 0x44);
        e.mem.set_u32(node + 0x50, 0xffff_ffff);
        start_log(&mut e);
        e.call(0x0046_1820, &args![this, node]);
        assert!(calls(&e, LOG_MESSAGE).is_empty());
        assert!(calls(&e, ARRAY_SET_AT_GROW).is_empty());
        assert_eq!(calls(&e, ADDON_NODE_SET_INDEX), vec![vec![node, 9]]);
    }

    #[test]
    fn add_addon_node_ignores_a_null_node() {
        let (mut e, this) = addon_engine(&[]);
        start_log(&mut e);
        e.call(0x0046_1820, &args![this, 0u32]);
        assert!(call_order(&e).is_empty());
    }

    #[test]
    fn remove_addon_node_clears_its_index() {
        let (mut e, this) = engine2();
        e.register(ADDON_NODE_GET_INDEX, |e, a| ret(e.mem.u32(a[0] + 0x50)));
        do_nothing(&mut e, &[ADDON_NODE_ARRAY_REMOVE_AT]);
        let node = e.mem.alloc(0x100);
        e.mem.set_u32(node + 0x50, 4);
        start_log(&mut e);
        e.call(0x0046_1930, &args![this, node]);
        assert_eq!(
            calls(&e, ADDON_NODE_ARRAY_REMOVE_AT),
            vec![vec![this.addr() + 0x1ec, 4]]
        );
        start_log(&mut e);
        e.call(0x0046_1930, &args![this, 0u32]);
        assert!(call_order(&e).is_empty());
    }

    // The interior cell array.

    #[test]
    fn cell_count_asks_the_cell_array() {
        let (mut e, this) = engine2();
        e.register_double(CELL_ARRAY_COUNT, |_, _| ret(7));
        start_log(&mut e);
        assert_eq!(e.call(0x0046_1960, &args![this]).u32(), 7);
        assert_eq!(calls(&e, CELL_ARRAY_COUNT), vec![vec![this.addr() + 0x1dc]]);
    }

    #[test]
    fn cell_at_compacts_the_array_first() {
        let (mut e, this) = engine2();
        word_array(&mut e, &[0xc1, 0xc2]);
        do_nothing(&mut e, &[CELL_ARRAY_COMPACT]);
        start_log(&mut e);
        assert_eq!(e.call(0x0046_1980, &args![this, 1u32]).u32(), 0xc2);
        assert_eq!(call_order(&e), vec![CELL_ARRAY_COMPACT, ARRAY_AT]);
        assert_eq!(calls(&e, ARRAY_AT)[0][0], this.addr() + 0x1dc);
    }

    /// Sorts the cells with keys `keys` (0 for a null cell) and returns the
    /// order the array ends in.
    fn sorted_cells(keys: &[&str]) -> Vec<u32> {
        let (mut e, this) = engine2();
        real_string_compare(&mut e);
        let cells: Vec<u32> = keys
            .iter()
            .map(|key| {
                if key.is_empty() {
                    0
                } else {
                    let cell = e.mem.alloc(0x20);
                    let text = c_string(&mut e, key);
                    e.mem.set_u32(cell + 0x10, text);
                    cell
                }
            })
            .collect();
        let table = word_array(&mut e, &cells);
        let count = cells.len() as u32;
        e.register_double(CELL_ARRAY_COUNT, move |_, _| ret(count));
        do_nothing(&mut e, &[CELL_ARRAY_COMPACT]);
        e.register(CELL_SORT_KEY, |e, a| ret(e.mem.u32(a[0] + 0x10)));
        // `SetAt(index, &value)` stores the word.
        e.register_double(CELL_ARRAY_SET_AT, move |e, a| {
            let value = e.mem.u32(a[2]);
            e.mem.set_u32(table + 4 * a[1], value);
            Ret::default()
        });
        e.call(0x0046_19b0, &args![this]);
        let by_address: HashMap<u32, usize> =
            cells.iter().enumerate().map(|(i, c)| (*c, i)).collect();
        (0..cells.len() as u32)
            .map(|i| by_address[&e.mem.u32(table + 4 * i)] as u32)
            .collect()
    }

    #[test]
    fn cell_sort_orders_the_cells_by_key() {
        // Indexes into the input, in the order the array ends in.
        assert_eq!(sorted_cells(&["b", "a", "c"]), vec![1, 0, 2]);
        assert_eq!(sorted_cells(&["d", "c", "b", "a"]), vec![3, 2, 1, 0]);
        assert_eq!(sorted_cells(&["a", "b"]), vec![0, 1]);
    }

    #[test]
    fn cell_sort_puts_a_cell_in_front_of_a_null_slot() {
        assert_eq!(sorted_cells(&["", "b", "a"]), vec![2, 1, 0]);
    }

    // Cells by editor id and by coordinates.

    #[test]
    fn get_cell_by_editor_id_looks_in_the_interior_cells_then_the_worlds() {
        let (mut e, this) = engine2();
        real_lists(&mut e);
        real_string_compare(&mut e);
        let interior_a = named_form(&mut e, "VaultA", 1);
        let interior_b = named_form(&mut e, "VaultB", 2);
        word_array(&mut e, &[interior_a, interior_b]);
        // Two world spaces: 0xd1 has no such cell, 0xd2 has.
        build_list(
            &mut e,
            this.at(TESDataHandler::listWorldSpaces).addr(),
            &[0xd1, 0xd2],
        );
        e.register_double(WORLD_FIND_CELL_BY_EDITOR_ID, |e, a| {
            let name = e.mem.cstr(a[1]);
            ret(if a[0] == 0xd2 && name == b"Outside" {
                0xcc
            } else {
                0
            })
        });
        let name = c_string(&mut e, "vaultb");
        start_log(&mut e);
        assert_eq!(e.call(0x0046_1ae0, &args![this, name]).u32(), interior_b);
        assert!(calls(&e, WORLD_FIND_CELL_BY_EDITOR_ID).is_empty());
        let name = c_string(&mut e, "Outside");
        assert_eq!(e.call(0x0046_1ae0, &args![this, name]).u32(), 0xcc);
        assert_eq!(
            calls(&e, WORLD_FIND_CELL_BY_EDITOR_ID),
            vec![vec![0xd1, name], vec![0xd2, name]]
        );
        let name = c_string(&mut e, "Nowhere");
        assert_eq!(e.call(0x0046_1ae0, &args![this, name]).u32(), 0);
    }

    #[test]
    fn get_cell_by_editor_id_of_a_null_name_is_null() {
        let (mut e, this) = engine2();
        start_log(&mut e);
        assert_eq!(e.call(0x0046_1ae0, &args![this, 0u32]).u32(), 0);
        assert!(call_order(&e).is_empty());
    }

    #[test]
    fn world_coordinates_become_cell_coordinates() {
        let (mut e, this) = engine2();
        e.register(FLOAT_TO_INT, |_, a| ret(f32::from_bits(a[0]) as i32 as u32));
        e.register_double(WORLD_GET_CELL, |_, _| ret(0xce11));
        e.register(HANDLER_IS_SAVE_LOAD, |_, _| ret(0));
        start_log(&mut e);
        let result = e.call(
            0x0046_1bc0,
            &args![this, 8192.0f32, -4097.0f32, 0x70u32, 0u8],
        );
        assert_eq!(result.u32(), 0xce11);
        // 8192 >> 12 = 2; -4097 >> 12 = -2 (arithmetic shift).
        assert_eq!(calls(&e, WORLD_GET_CELL), vec![vec![0x70, 2, -2i32 as u32]]);
        assert_eq!(
            calls(&e, FLOAT_TO_INT),
            vec![vec![8192.0f32.to_bits()], vec![(-4097.0f32).to_bits()]]
        );
    }

    #[test]
    fn exterior_cell_lookup_uses_the_first_world_when_none_is_given() {
        let (mut e, this) = engine2();
        real_lists(&mut e);
        e.register_double(WORLD_GET_CELL, |_, _| ret(0xce11));
        // No world space at all.
        assert_eq!(
            e.call(0x0046_1c20, &args![this, 1i32, 2i32, 0u32, 0u8])
                .u32(),
            0
        );
        build_list(
            &mut e,
            this.at(TESDataHandler::listWorldSpaces).addr(),
            &[0xd1],
        );
        start_log(&mut e);
        assert_eq!(
            e.call(0x0046_1c20, &args![this, 1i32, 2i32, 0u32, 0u8])
                .u32(),
            0xce11
        );
        assert_eq!(calls(&e, WORLD_GET_CELL), vec![vec![0xd1, 1, 2]]);
    }

    #[test]
    fn exterior_cell_lookup_rejects_coordinates_out_of_range() {
        let (mut e, this) = engine2();
        e.register_double(WORLD_GET_CELL, |_, _| ret(0xce11));
        do_nothing(&mut e, &[LOG_MESSAGE]);
        for (x, y) in [(0x8000, 0), (0, 0x8000), (-0x8001, 0), (0, -0x8001)] {
            start_log(&mut e);
            let result = e.call(0x0046_1c20, &args![this, x, y, 0x70u32, 1u8]);
            assert_eq!(result.u32(), 0, "({x}, {y})");
            assert_eq!(
                calls(&e, LOG_MESSAGE),
                vec![vec![INVALID_CELL_COORD_FORMAT, 0xffff_8000, 0x7fff]]
            );
            assert!(calls(&e, WORLD_GET_CELL).is_empty());
        }
        // The limits themselves are valid.
        start_log(&mut e);
        e.call(
            0x0046_1c20,
            &args![this, 0x7fffi32, -0x8000i32, 0x70u32, 0u8],
        );
        assert_eq!(calls(&e, WORLD_GET_CELL).len(), 1);
        assert!(calls(&e, LOG_MESSAGE).is_empty());
    }

    #[test]
    fn exterior_cell_lookup_creates_a_missing_cell_when_asked() {
        let (mut e, this) = engine2();
        e.register(HANDLER_IS_SAVE_LOAD, |e, a| {
            ret(e.mem.u8(a[0] + 0x61a) as u32)
        });
        e.register_double(WORLD_GET_CELL, |_, _| ret(0));
        let cell = new_cell_world(&mut e, true, 0);
        let world = e.mem.alloc(0x100);
        // `create` clear: null, and no cell is made.
        start_log(&mut e);
        assert_eq!(
            e.call(0x0046_1c20, &args![this, 3i32, 4i32, world, 0u8])
                .u32(),
            0
        );
        assert!(calls(&e, CELL_CONSTRUCT).is_empty());
        // `create` set: `NewCell` with the empty editor id.
        start_log(&mut e);
        assert_eq!(
            e.call(0x0046_1c20, &args![this, 3i32, 4i32, world, 1u8])
                .u32(),
            cell
        );
        assert_eq!(calls(&e, 0x0210_0134), vec![vec![cell, EMPTY_STRING]]);
        assert_eq!(calls(&e, CELL_SET_DATA_COORD), vec![vec![cell, 3, 4]]);
        // Loading a save game: never created.
        e.set(this, TESDataHandler::bSaveLoad, true);
        start_log(&mut e);
        assert_eq!(
            e.call(0x0046_1c20, &args![this, 3i32, 4i32, world, 1u8])
                .u32(),
            0
        );
        assert!(calls(&e, CELL_CONSTRUCT).is_empty());
    }

    // `GetExtCellDataFromFileByEditorID`.

    #[test]
    fn ext_cell_data_of_an_empty_name_is_null_and_zeroes_the_outputs() {
        let (mut e, this) = engine2();
        do_nothing(&mut e, &[SCOPE_ENTER, SCOPE_LEAVE]);
        let out = e.mem.alloc(8);
        e.mem.set_u32(out, 5);
        e.mem.set_u32(out + 4, 6);
        let empty = c_string(&mut e, "");
        start_log(&mut e);
        let result = e.call(0x0046_1cf0, &args![this, empty, out, out + 4]);
        assert_eq!(result.u32(), 0);
        assert_eq!((e.mem.u32(out), e.mem.u32(out + 4)), (0, 0));
        assert_eq!(
            &calls(&e, SCOPE_ENTER)[0][1..],
            &[0x16, 1, SOURCE_FILE, 0x1041]
        );
        assert_eq!(calls(&e, SCOPE_LEAVE).len(), 1);
        let result = e.call(0x0046_1cf0, &args![this, 0u32, out, out + 4]);
        assert_eq!(result.u32(), 0);
    }

    #[test]
    fn ext_cell_data_from_optimized_files_asks_the_world_spaces() {
        let (mut e, this) = engine2();
        real_lists(&mut e);
        do_nothing(&mut e, &[SCOPE_ENTER, SCOPE_LEAVE]);
        e.set(this, TESDataHandler::iNumCompile, 2);
        let index = this.at(TESDataHandler::pFileIndex).addr();
        e.mem.set_u32(index, 0);
        e.mem.set_u32(index + 4, 0xf1);
        e.register(FILE_GET_OPTIMIZED, |_, a| ret((a[0] == 0xf1) as u32));
        build_list(
            &mut e,
            this.at(TESDataHandler::listWorldSpaces).addr(),
            &[0xd1, 0xd2, 0xd3],
        );
        // World 0xd2 knows the cell and writes its coordinates.
        e.register_double(WORLD_GET_EXT_CELL_DATA, |e, a| {
            if a[0] == 0xd2 {
                e.mem.set_u32(a[2], 11);
                e.mem.set_u32(a[3], 12);
                ret(1)
            } else {
                ret(0)
            }
        });
        let name = c_string(&mut e, "Goodsprings");
        let out = e.mem.alloc(8);
        start_log(&mut e);
        let result = e.call(0x0046_1cf0, &args![this, name, out, out + 4]);
        assert_eq!(result.u32(), 0xd2);
        assert_eq!((e.mem.u32(out), e.mem.u32(out + 4)), (11, 12));
        assert_eq!(
            calls(&e, WORLD_GET_EXT_CELL_DATA),
            vec![
                vec![0xd1, name, out, out + 4],
                vec![0xd2, name, out, out + 4]
            ]
        );
        // No file is opened.
        assert!(calls(&e, FILE_OPEN).is_empty());
        // Nobody knows it: the list ends.
        let name = c_string(&mut e, "Nowhere");
        e.register_double(WORLD_GET_EXT_CELL_DATA, |_, _| ret(0));
        assert_eq!(
            e.call(0x0046_1cf0, &args![this, name, out, out + 4]).u32(),
            0
        );
    }

    /// One form of the scripted file: (type word, word at +8, word at +0xC).
    type ScriptedForm = (u32, u32, u32);

    /// The state of a scripted `TESFile`: `forms[current]` is what sits in
    /// `m_currentform` (+0x240); `NextForm` and the group skip step it.
    struct ScriptedFile {
        forms: Vec<ScriptedForm>,
        current: usize,
        /// The chunk ids of the current cell record, in order.
        chunks: Vec<u32>,
        chunk: usize,
    }

    fn publish_current_form(e: &mut Engine, file: u32, script: &ScriptedFile) {
        let (kind, label, extra) = script
            .forms
            .get(script.current)
            .copied()
            .unwrap_or((0, 0, 0));
        e.mem.set_u32(file + 0x240, kind);
        e.mem.set_u32(file + 0x248, label);
        e.mem.set_u32(file + 0x24c, extra);
    }

    /// An engine with one opened file `0xf1`'s stand-in (a real block),
    /// scripted to hold `forms`, whose cell records have the chunks
    /// `chunks`. The record types are the exe's words 1 (group), 2 (world),
    /// 3 (cell). Returns (engine, handler, file, shared script).
    fn scan_engine(
        forms: Vec<ScriptedForm>,
        chunks: Vec<u32>,
    ) -> (Engine, Ptr<TESDataHandler>, u32, Rc<RefCell<ScriptedFile>>) {
        let (mut e, this) = engine2();
        e.set_global(GROUP_TYPE_WORD, 1u32);
        e.set_global(WORLD_TYPE_WORD, 2u32);
        e.set_global(CELL_TYPE_WORD, 3u32);
        do_nothing(&mut e, &[SCOPE_ENTER, SCOPE_LEAVE, FILE_CLOSE]);
        e.set(this, TESDataHandler::iNumCompile, 1);
        let file = e.mem.alloc(0x400);
        e.mem
            .set_u32(this.at(TESDataHandler::pFileIndex).addr(), file);
        e.register(FILE_GET_OPTIMIZED, |_, _| ret(0));
        e.register(FILE_OPEN, |_, _| ret(1));
        real_string_compare(&mut e);
        let script = Rc::new(RefCell::new(ScriptedFile {
            forms,
            current: 0,
            chunks,
            chunk: 0,
        }));
        publish_current_form(&mut e, file, &script.borrow());
        // NextForm and the group skip step to the next form; true while there
        // is one.
        for addr in [FILE_NEXT_FORM, FILE_SKIP_GROUP] {
            let shared = script.clone();
            e.register_double(addr, move |e, a| {
                let mut script = shared.borrow_mut();
                script.current += 1;
                script.chunk = 0;
                publish_current_form(e, a[0], &script);
                ret((script.current < script.forms.len()) as u32)
            });
        }
        let shared = script.clone();
        e.register_double(FILE_GET_CHUNK_ID, move |_, _| {
            let script = shared.borrow();
            ret(script.chunks.get(script.chunk).copied().unwrap_or(0))
        });
        let shared = script.clone();
        e.register_double(FILE_NEXT_CHUNK, move |_, _| {
            let mut script = shared.borrow_mut();
            script.chunk += 1;
            ret((script.chunk < script.chunks.len()) as u32)
        });
        e.register(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(OPERATOR_DELETE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(MEMSET, |e, a| {
            for i in 0..a[2] {
                e.mem.set_u8(a[0] + i, a[1] as u8);
            }
            Ret::default()
        });
        e.register(FILE_CHUNK_SIZE, |_, _| ret(4));
        e.register(FILE_MUST_ENDIAN_CONVERT, |_, _| ret(0));
        e.register(COORDS_INIT, |_, a| ret(a[0]));
        // The owner's file index word 0 sits in the file named 0x777.
        e.register(FILE_GET_INDEX_FILE, |_, _| ret(0x777));
        e.register(FILE_COMPILE_INDEX, |_, a| {
            ret(if a[0] == 0x777 { 5 } else { 0 })
        });
        e.register(FORM_BY_ID, |_, a| ret(a[0] + 0x1000_0000));
        e.register(DYNAMIC_CAST, |_, a| ret(a[0] ^ 0x00ff_0000));
        // The chunk data: EDID gives the name "Name", XCLC two words.
        e.register(FILE_GET_CHUNK_DATA, |e, a| {
            if a[2] == 0 {
                e.mem.set_cstr(a[1], b"Name");
            } else {
                e.mem.set_u32(a[1], 5);
                e.mem.set_u32(a[1] + 4, -3i32 as u32);
            }
            Ret::default()
        });
        (e, this, file, script)
    }

    /// The forms of a world: a group that is not the world one, the world
    /// group, a world record, then a cell record.
    fn world_forms() -> Vec<ScriptedForm> {
        vec![
            (1, 0x1111, 0),
            (1, 2, 0),
            (2, 0, 0x0100_0aaa),
            (3, 0, 0x0100_0bbb),
        ]
    }

    #[test]
    fn ext_cell_data_is_read_from_the_file() {
        let (mut e, this, file, _) =
            scan_engine(world_forms(), vec![CHUNK_EDID, 0x4141, CHUNK_XCLC]);
        let name = c_string(&mut e, "NAME");
        let out = e.mem.alloc(8);
        start_log(&mut e);
        let result = e.call(0x0046_1cf0, &args![this, name, out, out + 4]);
        // The world record's id: file index 1 + 1 -> file 0x777, whose
        // compile index is 5: 0x05000aaa; the cast result is what
        // `__RTDynamicCast` answers for the form `TESForm lookup` found.
        assert_eq!(result.u32(), (0x0500_0aaa + 0x1000_0000) ^ 0x00ff_0000);
        assert_eq!(
            calls(&e, FILE_GET_INDEX_FILE),
            vec![vec![file, 2]],
            "the owner index is the id's high byte plus 1"
        );
        assert_eq!(calls(&e, FORM_BY_ID), vec![vec![0x0500_0aaa]]);
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![
                0x0500_0aaa + 0x1000_0000,
                0,
                FORM_TYPE_DESCRIPTOR,
                WORLD_SPACE_TYPE_DESCRIPTOR,
                0
            ]]
        );
        assert_eq!((e.mem.u32(out), e.mem.u32(out + 4)), (5, -3i32 as u32));
        // The file was opened with (0, 0) and closed.
        assert_eq!(calls(&e, FILE_OPEN), vec![vec![file, 0, 0]]);
        assert_eq!(calls(&e, FILE_CLOSE), vec![vec![file]]);
        // The chunk buffer was one byte longer than the chunk, and freed.
        assert_eq!(calls(&e, OPERATOR_NEW), vec![vec![5]]);
        assert_eq!(calls(&e, OPERATOR_DELETE).len(), 1);
        assert_eq!(calls(&e, FILE_GET_CHUNK_DATA).len(), 2);
        assert_eq!(calls(&e, FILE_GET_CHUNK_DATA)[1][2], 12);
    }

    #[test]
    fn ext_cell_data_swaps_the_coordinates_of_a_file_that_needs_it() {
        let (mut e, this, _, _) = scan_engine(world_forms(), vec![CHUNK_EDID, CHUNK_XCLC]);
        e.register(FILE_MUST_ENDIAN_CONVERT, |_, _| ret(1));
        e.register(SWAP_WORD, |e, a| {
            let word = e.mem.u32(a[0]);
            e.mem.set_u32(a[0], word.swap_bytes());
            Ret::default()
        });
        let name = c_string(&mut e, "name");
        let out = e.mem.alloc(8);
        e.call(0x0046_1cf0, &args![this, name, out, out + 4]);
        assert_eq!(
            (e.mem.u32(out), e.mem.u32(out + 4)),
            (5u32.swap_bytes(), (-3i32 as u32).swap_bytes())
        );
    }

    #[test]
    fn ext_cell_data_of_a_cell_with_another_name_is_null() {
        let (mut e, this, file, _) = scan_engine(world_forms(), vec![CHUNK_EDID, CHUNK_XCLC]);
        let name = c_string(&mut e, "Other");
        let out = e.mem.alloc(8);
        start_log(&mut e);
        let result = e.call(0x0046_1cf0, &args![this, name, out, out + 4]);
        assert_eq!(result.u32(), 0);
        assert_eq!((e.mem.u32(out), e.mem.u32(out + 4)), (0, 0));
        // The file was scanned to the end and closed; only the name chunk
        // was read.
        assert_eq!(calls(&e, FILE_CLOSE), vec![vec![file]]);
        assert_eq!(calls(&e, FILE_GET_CHUNK_DATA).len(), 1);
    }

    #[test]
    fn ext_cell_data_skips_files_that_do_not_open_or_have_no_world_group() {
        // No world group: the scan walks over the group and stops.
        let (mut e, this, file, _) = scan_engine(vec![(1, 0x1111, 0), (1, 0x2222, 0)], vec![]);
        let name = c_string(&mut e, "Name");
        let out = e.mem.alloc(8);
        start_log(&mut e);
        assert_eq!(
            e.call(0x0046_1cf0, &args![this, name, out, out + 4]).u32(),
            0
        );
        assert_eq!(
            calls(&e, FILE_SKIP_GROUP).len(),
            2,
            "both groups, then the end"
        );
        assert_eq!(calls(&e, FILE_CLOSE), vec![vec![file]]);
        // A file that does not open is passed over.
        e.register(FILE_OPEN, |_, _| ret(0));
        start_log(&mut e);
        assert_eq!(
            e.call(0x0046_1cf0, &args![this, name, out, out + 4]).u32(),
            0
        );
        assert!(calls(&e, FILE_CLOSE).is_empty());
    }

    #[test]
    fn ext_cell_data_walks_the_groups_inside_the_world_group() {
        // After the world group: a type-4 group is stepped into, a type-0
        // group of another label ends the scan.
        let forms = vec![(1, 2, 0), (1, 0x77, 4), (1, 0x99, 0), (3, 0, 0x0100_0bbb)];
        let (mut e, this, _, script) = scan_engine(forms, vec![CHUNK_EDID, CHUNK_XCLC]);
        e.register(FORM_TYPE_FROM_STRING, |_, _| ret(0x50));
        let name = c_string(&mut e, "Name");
        let out = e.mem.alloc(8);
        start_log(&mut e);
        assert_eq!(
            e.call(0x0046_1cf0, &args![this, name, out, out + 4]).u32(),
            0
        );
        // Form 1 (type 4) steps on; form 2 (type 0, other label) asks the
        // form type of its label and stops without stepping.
        assert_eq!(calls(&e, FORM_TYPE_FROM_STRING), vec![vec![0x99]]);
        assert_eq!(script.borrow().current, 2);
    }

    #[test]
    fn byte_swap_swaps_two_words() {
        let mut e = Engine::new();
        e.register(SWAP_WORD, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x0046_2230, &args![0x4000u32]);
        assert_eq!(
            e.call_log.unwrap()[1..],
            [(SWAP_WORD, vec![0x4000, 0]), (SWAP_WORD, vec![0x4004, 0])]
        );
    }

    #[test]
    fn current_form_is_at_plus_0x240() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x0046_2270, &args![0x1000u32]).u32(), 0x1240);
    }

    // `UnloadCell` and the thread flag.

    fn unload_engine(
        interior: bool,
        clearing: bool,
        object_test: bool,
    ) -> (Engine, Ptr<TESDataHandler>, u32) {
        let (mut e, this) = engine2();
        e.set_global(OBJECT_011DDF38, 0x3300u32);
        e.set_global(PLAYER_SINGLETON, 0x4400u32);
        do_nothing(
            &mut e,
            &[
                SET_BYTE_01202DF0,
                CELL_DETACH,
                OBJECT_011E0E80_STEP,
                CELL_SET_BYTE_26,
                CELL_UNLOAD_STEP_1,
                CELL_UNLOAD_STEP_2,
                CELL_SET_LOWEST_PROCESS,
                CELL_SET_PROCESS_FLAG,
                LAND_UNLOAD_VERTICES,
                CELL_UNLOAD_STEP_3,
                CELL_UNLOAD_STEP_4,
                WORLD_UNLOAD_CELL,
            ],
        );
        e.register(FORM_HAS_FLAG_BIT_5, |_, _| ret(0));
        e.register_double(OBJECT_011DDF38_TEST, move |_, _| ret(object_test as u32));
        // The cell reports its lowest-process state twice, then not.
        let mut remaining = 2;
        e.register_double(CELL_GET_LOWEST_PROCESS, move |_, _| {
            let reported = remaining > 0;
            if reported {
                remaining -= 1;
            }
            ret(reported as u32)
        });
        e.register_double(HANDLER_IS_CLEARING_DATA, move |_, _| ret(clearing as u32));
        e.register(PLAYER_IS_SLEEPING_OR_RESTING, |_, _| ret(0));
        e.register_double(CELL_IS_INTERIOR, move |_, _| ret(interior as u32));
        e.register(CELL_GET_LAND, |_, _| ret(0x5500));
        e.register(CELL_GET_WORLD_SPACE, |_, _| ret(0x6600));
        let cell = e.mem.alloc(0x100);
        (e, this, cell)
    }

    #[test]
    fn unload_cell_runs_its_steps_in_the_games_order() {
        let (mut e, this, cell) = unload_engine(false, false, false);
        start_log(&mut e);
        e.call(0x0046_2290, &args![this, cell]);
        assert_eq!(
            call_order(&e),
            vec![
                FORM_HAS_FLAG_BIT_5,
                SET_BYTE_01202DF0,
                OBJECT_011DDF38_TEST,
                CELL_DETACH,
                OBJECT_011E0E80_STEP,
                CELL_SET_BYTE_26,
                CELL_UNLOAD_STEP_1,
                CELL_UNLOAD_STEP_2,
                CELL_GET_LOWEST_PROCESS,
                CELL_SET_LOWEST_PROCESS,
                CELL_GET_LOWEST_PROCESS,
                CELL_SET_LOWEST_PROCESS,
                CELL_GET_LOWEST_PROCESS,
                HANDLER_IS_CLEARING_DATA,
                PLAYER_IS_SLEEPING_OR_RESTING,
                CELL_SET_PROCESS_FLAG,
                CELL_IS_INTERIOR,
                CELL_GET_LAND,
                LAND_UNLOAD_VERTICES,
                CELL_UNLOAD_STEP_3,
                CELL_UNLOAD_STEP_4,
                CELL_IS_INTERIOR,
                CELL_GET_WORLD_SPACE,
                WORLD_UNLOAD_CELL,
                OBJECT_011DDF38_TEST,
                SET_BYTE_01202DF0,
            ]
        );
        assert_eq!(calls(&e, SET_BYTE_01202DF0), vec![vec![1], vec![0]]);
        assert_eq!(calls(&e, CELL_DETACH), vec![vec![cell, 1]]);
        assert_eq!(
            calls(&e, OBJECT_011E0E80_STEP),
            vec![vec![OBJECT_011E0E80, cell]]
        );
        assert_eq!(calls(&e, CELL_SET_BYTE_26), vec![vec![cell, 1]]);
        assert_eq!(calls(&e, CELL_SET_LOWEST_PROCESS), vec![vec![cell, 0]; 2]);
        assert_eq!(calls(&e, PLAYER_IS_SLEEPING_OR_RESTING), vec![vec![0x4400]]);
        // The player is not sleeping or resting: the flag is 1.
        assert_eq!(calls(&e, CELL_SET_PROCESS_FLAG), vec![vec![cell, 1]]);
        assert_eq!(calls(&e, LAND_UNLOAD_VERTICES), vec![vec![0x5500]]);
        assert_eq!(calls(&e, CELL_UNLOAD_STEP_4), vec![vec![cell, 0]]);
        assert_eq!(calls(&e, WORLD_UNLOAD_CELL), vec![vec![0x6600, cell]]);
        assert_eq!(
            calls(&e, HANDLER_IS_CLEARING_DATA),
            vec![vec![this.addr()]],
            "the singleton, which is `this` here"
        );
    }

    #[test]
    fn unload_cell_of_an_interior_cell_leaves_the_land_and_world_alone() {
        let (mut e, this, cell) = unload_engine(true, true, false);
        start_log(&mut e);
        e.call(0x0046_2290, &args![this, cell]);
        let order = call_order(&e);
        // Clearing data: nobody asks the player.
        for skipped in [
            PLAYER_IS_SLEEPING_OR_RESTING,
            CELL_SET_PROCESS_FLAG,
            CELL_GET_LAND,
            LAND_UNLOAD_VERTICES,
            CELL_GET_WORLD_SPACE,
            WORLD_UNLOAD_CELL,
        ] {
            assert!(!order.contains(&skipped), "{skipped:08x}");
        }
        assert!(order.contains(&CELL_UNLOAD_STEP_3));
    }

    #[test]
    fn unload_cell_sets_the_thread_flag_around_the_work() {
        let (mut e, this, cell) = unload_engine(true, false, true);
        let word = e.tls() + TLS_FLAG_WORD;
        let seen = Rc::new(RefCell::new(None));
        let shared = seen.clone();
        e.register_double(CELL_DETACH, move |e, _| {
            *shared.borrow_mut() = Some(e.mem.u32(word) & 1);
            Ret::default()
        });
        e.call(0x0046_2290, &args![this, cell]);
        assert_eq!(*seen.borrow(), Some(1), "set while the cell is detached");
        assert_eq!(e.mem.u32(word) & 1, 0, "restored at the end");
    }

    #[test]
    fn unload_cell_ignores_null_and_flagged_cells() {
        let (mut e, this, cell) = unload_engine(false, false, false);
        start_log(&mut e);
        e.call(0x0046_2290, &args![this, 0u32]);
        assert!(call_order(&e).is_empty());
        e.register(FORM_HAS_FLAG_BIT_5, |_, _| ret(1));
        start_log(&mut e);
        e.call(0x0046_2290, &args![this, cell]);
        assert_eq!(call_order(&e), vec![FORM_HAS_FLAG_BIT_5]);
    }

    #[test]
    fn thread_flag_setter_sets_and_clears_bit_0() {
        let mut e = Engine::new();
        let word = e.tls() + TLS_FLAG_WORD;
        // Flag 0 sets the bit and answers what the test said before.
        assert_eq!(e.call(0x0046_23f0, &args![0x1000u32, 0u8]).u8(), 1);
        assert_eq!(e.mem.u32(word), 1);
        assert_eq!(e.call(0x0046_23f0, &args![0x1000u32, 0u8]).u8(), 0);
        e.mem.set_u32(word, 0xf1);
        assert_eq!(e.call(0x0046_23f0, &args![0x1000u32, 1u8]).u8(), 0);
        assert_eq!(e.mem.u32(word), 0xf0);
        assert_eq!(e.call(0x0046_23f0, &args![0x1000u32, 1u8]).u8(), 1);
    }

    #[test]
    fn thread_flag_test_says_whether_bit_0_is_clear() {
        let mut e = Engine::new();
        let word = e.tls() + TLS_FLAG_WORD;
        assert!(e.call(0x0046_2480, &args![]).bool());
        e.mem.set_u32(word, 1);
        assert!(!e.call(0x0046_2480, &args![]).bool());
        e.mem.set_u32(word, 0xfe);
        assert!(e.call(0x0046_2480, &args![]).bool());
    }

    // The small accessors and wrappers.

    #[test]
    fn file_path_is_at_plus_0x124() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x0046_2e80, &args![0x1000u32]).u32(), 0x1124);
    }

    #[test]
    fn file_info_is_at_plus_0x29c() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x0046_2ea0, &args![0x1000u32]).u32(), 0x129c);
    }

    #[test]
    fn file_cached_flag_is_the_byte_at_plus_0x428() {
        let mut e = Engine::new();
        let file = e.mem.alloc(0x430);
        e.call(0x0046_2e60, &args![file, 1u8]);
        assert_eq!(e.mem.u8(file + 0x428), 1);
        e.call(0x0046_2e60, &args![file, 0u8]);
        assert_eq!(e.mem.u8(file + 0x428), 0);
    }

    #[test]
    fn summary_setter_passes_the_string_at_plus_0x418() {
        let mut e = Engine::new();
        e.register_double(SUMMARY_SET, |_, a| ret(a[0]));
        start_log(&mut e);
        let result = e.call(0x0046_2ec0, &args![0x1000u32, 0x2000u32]);
        assert_eq!(result.u32(), 0x1418);
        assert_eq!(calls(&e, SUMMARY_SET), vec![vec![0x1418, 0x2000]]);
    }

    #[test]
    fn dlc_flags_are_set_and_cleared_by_mask() {
        let (mut e, this) = engine2();
        assert_eq!(e.call(0x0046_2e10, &args![this, 0x02u8, true]).u8(), 0x02);
        assert_eq!(e.call(0x0046_2e10, &args![this, 0x08u8, true]).u8(), 0x0a);
        assert_eq!(e.get(this, TESDataHandler::cDLCFlags), 0x0a);
        assert_eq!(e.call(0x0046_2e10, &args![this, 0x02u8, false]).u8(), 0x08);
        assert_eq!(e.get(this, TESDataHandler::cDLCFlags), 0x08);
    }

    #[test]
    fn splitpath_wrapper_passes_all_nine_words() {
        let mut e = Engine::new();
        e.register_double(SPLIT_PATH_S, |_, _| ret(22));
        start_log(&mut e);
        let result = e.call(
            0x0046_2d40,
            &args![1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32, 8u32, 9u32],
        );
        assert_eq!(result.i32(), 22);
        assert_eq!(
            calls(&e, SPLIT_PATH_S),
            vec![vec![1, 2, 3, 4, 5, 6, 7, 8, 9]]
        );
    }

    #[test]
    fn stream_read_calls_the_function_pointer_and_advances_the_position() {
        let mut e = Engine::new();
        let stream = e.mem.alloc(0x20);
        e.mem.set_u32(stream + 4, 100);
        e.mem.set_u32(stream + 8, 0x0300_0000);
        e.register(0x0300_0000, |_, _| ret(12));
        let buffer = e.mem.alloc(0x20);
        let components = e.mem.alloc(4);
        start_log(&mut e);
        let count = e.call(
            0x0046_2dc0,
            &args![stream, buffer, 0x40u32, components, 1u32],
        );
        assert_eq!(count.u32(), 12);
        assert_eq!(e.mem.u32(stream + 4), 112);
        assert_eq!(
            calls(&e, 0x0300_0000),
            vec![vec![stream, buffer, 0x40, components, 1]]
        );
    }

    #[test]
    fn stream_read_of_a_buffer_gives_one_component_of_size_one() {
        let mut e = Engine::new();
        let stream = e.mem.alloc(0x20);
        e.mem.set_u32(stream + 8, 0x0300_0000);
        let seen = Rc::new(RefCell::new(vec![]));
        let shared = seen.clone();
        e.register_double(0x0300_0000, move |e, a| {
            // The component word is only valid during the call.
            shared.borrow_mut().push((a.to_vec(), e.mem.u32(a[3])));
            ret(7)
        });
        let buffer = e.mem.alloc(0x20);
        assert_eq!(
            e.call(0x0046_2d80, &args![stream, buffer, 0x100u32]).u32(),
            7
        );
        let seen = seen.borrow();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0[..3], [stream, buffer, 0x100]);
        assert_eq!(seen[0].0[4], 1);
        assert_eq!(seen[0].1, 1);
        assert_eq!(e.mem.u32(stream + 4), 7);
    }

    // `004624b0`, the scan of the data directory.

    const READ_STREAM: u32 = 0x0300_0100;
    const DIRECTORY: &str = "Data\\";

    /// What the doubles of `scan_env` recorded.
    #[derive(Default)]
    struct ScanLog {
        /// (node, item) of each `LIST_ADD` and `LIST_APPEND`.
        added: Vec<(u32, u32)>,
        appended: Vec<(u32, u32)>,
        removed_items: Vec<(u32, u32)>,
        summary: Option<(u32, Vec<u8>)>,
        nam_path: Option<Vec<u8>>,
        created: Vec<u32>,
    }

    struct ScanEnv {
        e: Engine,
        this: Ptr<TESDataHandler>,
        old_file: u32,
        directory: u32,
        log: Rc<RefCell<ScanLog>>,
        file_time_order: Rc<RefCell<i32>>,
        old_file_present: Rc<RefCell<bool>>,
        header_status: Rc<RefCell<u32>>,
    }

    fn copy_c_string(e: &mut Engine, destination: u32, text: &[u8]) {
        e.mem.set_cstr(destination, text);
    }

    /// Doubles for the callees of `004624b0` with one listed master file
    /// `Old.esm`, and `New.esm` on disk (no `.esp`). The new file is a
    /// master too (the byte at +0x3f0 of a file).
    fn scan_env() -> ScanEnv {
        let (mut e, this) = engine2();
        e.map(0x0101_8000, 0x1000);
        e.mem.set_cstr(PATTERN_ESM, b"*.esm");
        e.mem.set_cstr(PATTERN_ESP, b"*.esp");
        e.mem.set_cstr(UPDATE_BSA, b"Update.bsa");
        e.mem.set_cstr(NAM_PATH_FORMAT, b"%s%s.NAM");
        real_lists(&mut e);
        let log = Rc::new(RefCell::new(ScanLog::default()));
        let file_time_order = Rc::new(RefCell::new(-1));
        let old_file_present = Rc::new(RefCell::new(true));
        let header_status = Rc::new(RefCell::new(0u32));
        do_nothing(
            &mut e,
            &[
                SCOPE_ENTER,
                SCOPE_LEAVE,
                FILE_CLOSE,
                FIND_CLOSE,
                BSFILE_DESTRUCT,
                FILE_SET_FLAG_BIT_2,
                OPEN_ARCHIVE,
                FILE_GEN_INDEX_TABLE,
                BSFILE_CONSTRUCT,
                FILE_DESTRUCT,
                OPERATOR_DELETE,
            ],
        );
        e.register(STRING_COPY_S, |e, a| {
            let text = e.mem.cstr(a[2]);
            copy_c_string(e, a[0], &text);
            Ret::default()
        });
        e.register(LSTRCPY_A, |e, a| {
            let text = e.mem.cstr(a[1]);
            copy_c_string(e, a[0], &text);
            ret(a[0])
        });
        e.register(LSTRCAT_A, |e, a| {
            let mut text = e.mem.cstr(a[0]);
            text.extend(e.mem.cstr(a[1]));
            copy_c_string(e, a[0], &text);
            ret(a[0])
        });
        let present = old_file_present.clone();
        e.register_double(FIND_FIRST_FILE_A, move |e, a| {
            let path = e.mem.cstr(a[0]);
            if path.ends_with(b"*.esm") {
                e.mem.set_u32(a[1] + FIND_DATA_SIZE_LOW, 100);
                e.mem.set_u32(a[1] + FIND_DATA_SIZE_HIGH, 0);
                copy_c_string(e, a[1] + FIND_DATA_FILE_NAME, b"New.esm");
                ret(0x41)
            } else if path.ends_with(b"*.esp") || (path.ends_with(b"Old.esm") && !*present.borrow())
            {
                ret(INVALID_HANDLE)
            } else {
                ret(0x42)
            }
        });
        e.register(FILE_NAME, |_, a| ret(a[0] + 0x20));
        e.register(FIND_NEXT_FILE_A, |_, _| ret(0));
        e.register(GET_LIST_FILE, |_, _| ret(0));
        let status = header_status.clone();
        e.register_double(FILE_OPEN_HEADER, move |_, _| ret(*status.borrow()));
        e.register(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        let shared = log.clone();
        e.register_double(FILE_CONSTRUCT, move |e, a| {
            let name = e.mem.cstr(a[2]);
            let path = e.mem.cstr(a[1]);
            copy_c_string(e, a[0] + 0x20, &name);
            copy_c_string(e, a[0] + 0x124, &path);
            e.mem.set_u8(a[0] + 0x3f0, name.ends_with(b".esm") as u8);
            shared.borrow_mut().created.push(a[0]);
            ret(a[0])
        });
        e.register(SPLIT_PATH_S, |e, a| {
            let name = e.mem.cstr(a[0]);
            let dot = name.iter().rposition(|c| *c == b'.').unwrap();
            copy_c_string(e, a[5], &name[..dot]);
            copy_c_string(e, a[7], &name[dot..]);
            ret(0)
        });
        let shared = log.clone();
        e.register_double(FORMAT_S, move |e, a| {
            let mut text = e.mem.cstr(a[3]);
            text.extend(e.mem.cstr(a[4]));
            text.extend_from_slice(b".NAM");
            copy_c_string(e, a[0], &text);
            shared.borrow_mut().nam_path = Some(text);
            ret(0)
        });
        e.register(BSFILE_EXIST, |_, _| ret(1));
        e.register(BSFILE_FLAG_2C, |_, _| ret(1));
        // The stream's read function, after the constructor double sets it.
        e.register_double(BSFILE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0] + 8, READ_STREAM);
            ret(a[0])
        });
        e.register(READ_STREAM, |e, a| {
            copy_c_string(e, a[1], b"Summary");
            ret(7)
        });
        let shared = log.clone();
        e.register_double(SUMMARY_SET, move |e, a| {
            shared.borrow_mut().summary = Some((a[0], e.mem.cstr(a[1])));
            ret(a[0])
        });
        e.register(IS_DLC_PACKAGE_NAME, |_, _| ret(2));
        e.register(FILE_GET_MASTER, |e, a| ret(e.mem.u8(a[0] + 0x3f0) as u32));
        let order = file_time_order.clone();
        e.register_double(COMPARE_FILE_TIME, move |_, _| ret(*order.borrow() as u32));
        let shared = log.clone();
        e.register_double(LIST_ADD, move |e, a| {
            let item = e.mem.u32(a[1]);
            shared.borrow_mut().added.push((a[0], item));
            Ret::default()
        });
        let shared = log.clone();
        e.register_double(LIST_APPEND, move |e, a| {
            let item = e.mem.u32(a[1]);
            shared.borrow_mut().appended.push((a[0], item));
            Ret::default()
        });
        let shared = log.clone();
        e.register_double(LIST_REMOVE_ITEM, move |e, a| {
            let item = e.mem.u32(a[1]);
            shared.borrow_mut().removed_items.push((a[0], item));
            Ret::default()
        });
        // Removing the head node pulls the next one into it.
        e.register(LIST_REMOVE_HEAD, |e, a| {
            let next = e.mem.u32(a[0] + 4);
            if next != 0 {
                let (item, after) = (e.mem.u32(next), e.mem.u32(next + 4));
                e.mem.set_u32(a[0], item);
                e.mem.set_u32(a[0] + 4, after);
            } else {
                e.mem.set_u32(a[0], 0);
            }
            Ret::default()
        });
        e.register(FILE_FINDER_EXIST, |_, _| ret(1));
        e.register(FILE_INDEX_COUNT, |_, _| ret(0));
        e.register(FILE_GET_INDEX_FILE, |_, _| ret(0));
        let old_file = e.mem.alloc(0x430);
        copy_c_string(&mut e, old_file + 0x20, b"Old.esm");
        copy_c_string(&mut e, old_file + 0x124, DIRECTORY.as_bytes());
        e.mem.set_u8(old_file + 0x3f0, 1);
        build_list(
            &mut e,
            this.at(TESDataHandler::listFiles).addr(),
            &[old_file],
        );
        let directory = c_string(&mut e, DIRECTORY);
        ScanEnv {
            e,
            this,
            old_file,
            directory,
            log,
            file_time_order,
            old_file_present,
            header_status,
        }
    }

    #[test]
    fn scan_adds_a_new_master_file_at_the_end() {
        let mut env = scan_env();
        let (this, old_file, directory) = (env.this, env.old_file, env.directory);
        let head = this.at(TESDataHandler::listFiles).addr();
        start_log(&mut env.e);
        assert!(env.e.call(0x0046_24b0, &args![this, directory]).bool());
        let log = env.log.borrow();
        let new_file = log.created[0];
        assert_eq!(log.created.len(), 1, "only New.esm is unknown");
        // The old file is older: the new one goes after it.
        assert_eq!(log.appended, vec![(head, new_file)]);
        assert!(log.added.is_empty());
        assert_eq!(log.nam_path.as_deref(), Some(&b"Data\\New.NAM"[..]));
        assert_eq!(
            log.summary,
            Some((new_file + 0x418, b"Summary".to_vec())),
            "the summary read from the .NAM file"
        );
        let e = &env.e;
        assert_eq!(e.mem.u8(new_file + 0x428), 1, "bCached");
        assert_eq!(
            calls(e, FILE_SET_FLAG_BIT_2),
            vec![vec![new_file, 1]],
            "bit 2 of m_Flags"
        );
        // DLC package 2: bit 1 of cDLCFlags.
        assert_eq!(e.get(this, TESDataHandler::cDLCFlags), 2);
        let constructed = calls(e, FILE_CONSTRUCT);
        assert_eq!(constructed.len(), 1);
        assert_eq!(&constructed[0][..2], &[new_file, directory]);
        assert_eq!(e.mem.cstr(constructed[0][2]), b"New.esm");
        assert_eq!(constructed[0][3], 0);
        assert_eq!(
            &calls(e, SCOPE_ENTER)[0][1..],
            &[0x16, 1, SOURCE_FILE, 0x1195]
        );
        assert_eq!(calls(e, OPEN_ARCHIVE), vec![vec![UPDATE_BSA, 0, 0]]);
        assert_eq!(
            calls(e, FILE_FINDER_EXIST)[0][0],
            UPDATE_BSA,
            "Update.bsa is looked for"
        );
        // Both passes: *.esm first, then *.esp.
        let searches: Vec<Vec<u8>> = calls(e, LSTRCAT_A)
            .iter()
            .take(2)
            .map(|call| e.mem.cstr(call[1]))
            .collect();
        assert_eq!(searches, vec![b"*.esm".to_vec(), b"*.esp".to_vec()]);
        // The old file is a master: its index table is built in both loops.
        assert_eq!(
            calls(e, FILE_GEN_INDEX_TABLE),
            vec![vec![old_file, head, 0]; 2]
        );
        // Every listed file is closed at the end (and once at the start).
        assert!(calls(e, FILE_CLOSE).contains(&vec![old_file]));
    }

    #[test]
    fn scan_puts_a_new_file_before_a_listed_one_that_is_not_older() {
        let mut env = scan_env();
        *env.file_time_order.borrow_mut() = 1;
        let (this, directory) = (env.this, env.directory);
        let head = this.at(TESDataHandler::listFiles).addr();
        env.e.call(0x0046_24b0, &args![this, directory]);
        let log = env.log.borrow();
        let new_file = log.created[0];
        assert_eq!(log.added, vec![(head, new_file)]);
        assert!(log.appended.is_empty());
    }

    #[test]
    fn scan_compares_the_write_times_of_the_listed_and_the_new_file() {
        let mut env = scan_env();
        env.e.call_log = Some(vec![]);
        let (this, old_file, directory) = (env.this, env.old_file, env.directory);
        env.e.call(0x0046_24b0, &args![this, directory]);
        let find_data = calls(&env.e, FIND_FIRST_FILE_A)[0][1];
        assert_eq!(
            calls(&env.e, COMPARE_FILE_TIME),
            vec![vec![old_file + 0x29c + 0x14, find_data + 0x14]]
        );
    }

    #[test]
    fn scan_puts_a_new_master_before_a_listed_plugin() {
        let mut env = scan_env();
        // The listed file is not a master: the new master goes in front of
        // it without comparing times.
        env.e.mem.set_u8(env.old_file + 0x3f0, 0);
        env.e.call_log = Some(vec![]);
        let (this, directory) = (env.this, env.directory);
        let head = this.at(TESDataHandler::listFiles).addr();
        env.e.call(0x0046_24b0, &args![this, directory]);
        let log = env.log.borrow();
        assert_eq!(log.added, vec![(head, log.created[0])]);
        assert!(calls(&env.e, COMPARE_FILE_TIME).is_empty());
    }

    #[test]
    fn scan_removes_listed_files_that_fail_to_open() {
        let mut env = scan_env();
        *env.header_status.borrow_mut() = 2;
        env.e.call_log = Some(vec![]);
        let (this, old_file, directory) = (env.this, env.old_file, env.directory);
        let head = this.at(TESDataHandler::listFiles).addr();
        env.e.call(0x0046_24b0, &args![this, directory]);
        assert_eq!(calls(&env.e, FILE_OPEN_HEADER), vec![vec![old_file]]);
        assert_eq!(calls(&env.e, LIST_REMOVE_HEAD)[0], vec![head]);
        // The list is empty, so the new file is simply appended to it.
        let log = env.log.borrow();
        assert_eq!(log.appended, vec![(head, log.created[0])]);
    }

    #[test]
    fn scan_destroys_listed_files_that_are_gone_from_the_disk() {
        let mut env = scan_env();
        *env.old_file_present.borrow_mut() = false;
        env.e.call_log = Some(vec![]);
        let (this, old_file, directory) = (env.this, env.old_file, env.directory);
        let head = this.at(TESDataHandler::listFiles).addr();
        env.e.call(0x0046_24b0, &args![this, directory]);
        // The lookup was for the file's path plus name.
        let lookups: Vec<Vec<u8>> = calls(&env.e, FIND_FIRST_FILE_A)
            .iter()
            .map(|call| env.e.mem.cstr(call[0]))
            .collect();
        assert!(lookups.contains(&b"Data\\Old.esm".to_vec()));
        // It was the only node and nothing came before: head removal.
        assert_eq!(calls(&env.e, LIST_REMOVE_HEAD), vec![vec![head]]);
        assert_eq!(calls(&env.e, FILE_DESTRUCT), vec![vec![old_file]]);
        assert_eq!(calls(&env.e, OPERATOR_DELETE), vec![vec![old_file]]);
        // Nothing is left to index.
        assert!(calls(&env.e, FILE_GEN_INDEX_TABLE).is_empty());
    }

    #[test]
    fn scan_moves_the_masters_of_a_master_up_the_list() {
        let mut env = scan_env();
        // Old.esm has one master, 0x888, that the chain holds. The count
        // answers 1 once, then 0.
        let asked = Rc::new(RefCell::new(0u32));
        let shared = asked.clone();
        env.e.register_double(FILE_INDEX_COUNT, move |_, _| {
            *shared.borrow_mut() += 1;
            ret((*shared.borrow() == 1) as u32)
        });
        env.e.register(FILE_GET_INDEX_FILE, |_, _| ret(0x888));
        env.e.register(LIST_CONTAINS, |_, _| ret(1));
        env.e.call_log = Some(vec![]);
        let (this, old_file, directory) = (env.this, env.old_file, env.directory);
        let head = this.at(TESDataHandler::listFiles).addr();
        env.e.call(0x0046_24b0, &args![this, directory]);
        let log = env.log.borrow();
        // Removed and added again at the node, with the master file's address
        // in the word the call gets.
        assert_eq!(log.removed_items, vec![(head, 0x888)]);
        assert_eq!(log.added, vec![(head, 0x888)]);
        assert_eq!(
            calls(&env.e, FILE_GET_INDEX_FILE).first(),
            Some(&vec![old_file, 1])
        );
        // The node was not advanced after a move: the file is visited again.
        assert_eq!(calls(&env.e, FILE_GEN_INDEX_TABLE).len(), 3);
    }
}
