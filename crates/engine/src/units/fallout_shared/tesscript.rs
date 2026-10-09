//! `fallout shared/tesscript.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit holds `ScriptLocals` (the variables and action flags of a
//! running script instance, the engine's event list), the `Script` form, and
//! the static lists of references whose enable/disable/delete is delayed.
//!
//! Progress: 40 of the 77 functions, `005a8bc0` to `005aa930`, are
//! translated. The next session continues at `005aaaf0`
//! (`Script::LoadPendingReferences`).
//!
//! Notes for whoever continues:
//! - The engine map names `005a8bc0` `ScriptLocals::ScriptLocals`, but the
//!   body is the destructor (it empties both lists and frees the effect
//!   data); `005aa1a0` is `Script`'s destructor although the decompiler calls
//!   it `CSettingsStore::~CSettingsStore` (a wrong library match).
//! - `BSSimpleList` nodes are used through the game's small accessors, called
//!   by address as the game does: `006815c0(node)` (the address of the node's
//!   item word), `00726070(node)` (next node), `008256d0(node)` (the list is
//!   empty), `0063f7b0(list)` (remove the first item), `005ae3d0(list,
//!   &item)` (add at the head), `00905820(list, &item)` (add at the tail),
//!   `00905330(list, &item)` (remove an item) and `005f65d0(list, &item)`
//!   (contains).
//! - The four static lists of pending references are `ReferencesToEnable`
//!   (`011caca8`), `ReferencesToDisable` (`011cac98`), `ReferencesToDelete`
//!   (`011cacb8`) and `ReferencesToFadeDisable` (`011cace8`), all guarded by
//!   the critical section `ScriptRefListCrit` (`011cacf8`). The roles follow
//!   the names of the `IsPending...` functions and what
//!   `RunDelayedScriptActionsOnReferences` does with each list; the saved
//!   order is enable, disable, delete, fade-disable.
//! - A pushed word that follows a call with no stack clean-up belongs to the
//!   next call (`005e24d0` takes none, `005e2730` takes the three pushed
//!   words).
//! - Not translated: the compiler's exception-unwinding frames.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSSimpleList;

// ---- Memory and list helpers of the game, called by address ---------------

/// `operator new(size)` and `operator delete(block)` (`cdecl`).
pub(crate) const OPERATOR_NEW: u32 = 0x0040_1000;
pub(crate) const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `memset(ptr, value, size)` (`cdecl`) and `memcpy(dest, source, size)`
/// (`cdecl`, the game's wrapper).
const MEMSET: u32 = 0x0040_3d30;
const MEMCPY: u32 = 0x0040_1460;
/// The scope guard the game keeps on its stack: `00404eb0(guard, kind, 1,
/// source file, line)` and `00404ee0(guard)`.
const SCOPE_GUARD_OPEN: u32 = 0x0040_4eb0;
const SCOPE_GUARD_CLOSE: u32 = 0x0040_4ee0;
/// `BSSimpleList` accessors (see the module notes).
const LIST_ITEM_SLOT: u32 = 0x0068_15c0;
const LIST_NEXT: u32 = 0x0072_6070;
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
const LIST_POP_FIRST: u32 = 0x0063_f7b0;
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
const LIST_ADD_TAIL: u32 = 0x0090_5820;
const LIST_REMOVE: u32 = 0x0090_5330;
const LIST_CONTAINS: u32 = 0x005f_65d0;
/// The list constructor (clears item and next), its destructor and the
/// deleting destructor `004702f0(list, flags)`.
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
const LIST_DELETE: u32 = 0x0047_02f0;
/// `0044ddc0(object)`: the word at +8 (the action list of a `ScriptLocals`)
/// and `0084e3a0(locals)`: the word at +0xC (the variable list).
const ACTION_LIST_OF: u32 = 0x0044_ddc0;
const LOCAL_LIST_OF: u32 = 0x0084_e3a0;
/// `005d43c0(script)` and `005d43e0(script)`: the addresses of a script's
/// two embedded lists, `listRefObjects` (+0x44) and `listVariables` (+0x4C).
const SCRIPT_REF_OBJECT_LIST: u32 = 0x005d_43c0;
const SCRIPT_VARIABLE_LIST: u32 = 0x005d_43e0;

// ---- Logging, saving and loading ------------------------------------------

/// `Error(format, ...)` (Xbox PDB, `cdecl`, varargs) and the log
/// `005b5e40(format, ...)` (`cdecl`, varargs).
const ERROR_LOG: u32 = 0x0040_fbe0;
const SCRIPT_LOG: u32 = 0x005b_5e40;
/// The script line number that `GetVariable`'s message reports.
const SCRIPT_LINE_GLOBAL: u32 = 0x011c_ae50;
/// The save/load game object's global and the object `00408d60` turns into
/// the byte that says "log the save sizes".
const SAVE_LOAD_GAME_GLOBAL: u32 = 0x011d_e45c;
const SAVE_SIZE_LOG_OBJECT: u32 = 0x011d_e4e8;
const FLAG_BYTE_ADDRESS: u32 = 0x0040_8d60;
/// `TESSaveLoadGame` methods on the global (`this` first): the save format
/// uses blocks (`UseSaveGameBlocks`), the file version byte, the current
/// stream position (a pointer into the buffer), the header of the form being
/// loaded (`004fd3c0`) and saved (`004fd3e0`, `TES::GetWorldSpace` in the
/// engine map: form id at +0, flags at +5, version byte at +9), the load
/// test `0047c850` (a stub that returns 0), and the reading and writing of
/// raw bytes and numeric ids.
const USE_SAVE_GAME_BLOCKS: u32 = 0x0086_2110;
const SAVE_FILE_VERSION: u32 = 0x008d_f040;
const STREAM_POSITION: u32 = 0x0082_5c00;
const LOADING_FORM_HEADER: u32 = 0x004f_d3c0;
const SAVING_FORM_HEADER: u32 = 0x004f_d3e0;
const IS_LOADING_STUB: u32 = 0x0047_c850;
const STREAM_WRITE: u32 = 0x0085_79b0;
const STREAM_SAVE_NUMERIC_ID: u32 = 0x0085_7a10;
const STREAM_READ: u32 = 0x0085_79e0;
const STREAM_LOAD_NUMERIC_ID: u32 = 0x0085_7aa0;
/// `LookupFormByID(id)` (`cdecl`).
const LOOKUP_FORM_BY_ID: u32 = 0x0048_39c0;
/// `BGSSaveGameBuffer` methods (`this` the buffer): `StartVariableSizedValue`,
/// `SaveVariableSizedValue_ov2(count, position)`, `SaveFormID_ov2(form, 0)`,
/// `SaveFormID(id, 0)` and `00865e50(data, size, 0)`.
const BUFFER_START_SIZED_VALUE: u32 = 0x0086_5f20;
const BUFFER_SAVE_SIZED_VALUE: u32 = 0x0086_5ff0;
const BUFFER_SAVE_FORM_ID_OV2: u32 = 0x0086_5df0;
const BUFFER_SAVE_FORM_ID: u32 = 0x0086_5db0;
const BUFFER_SAVE_BYTES: u32 = 0x0086_5e50;
/// `BGSLoadGameBuffer` methods: `LoadVariableSizedValue`, `LoadFormID_ov2(
/// &id)` and `00864980(data, size)`.
const BUFFER_LOAD_SIZED_VALUE: u32 = 0x0086_4a60;
const BUFFER_LOAD_FORM_ID_OV2: u32 = 0x0086_48e0;
const BUFFER_LOAD_BYTES: u32 = 0x0086_4980;
/// `Script::PutNumericIDInDouble(&id, &double)` (`005acc70`) and its
/// inverse (`005acc90(&id, &double)`), both `cdecl`.
const PUT_NUMERIC_ID_IN_DOUBLE: u32 = 0x005a_cc70;
const GET_NUMERIC_ID_FROM_DOUBLE: u32 = 0x005a_cc90;
/// `ScriptRunManager::Instance()` and `InitActionList(master, count, locals)`.
const SCRIPT_RUN_MANAGER_INSTANCE: u32 = 0x005e_24d0;
const SCRIPT_RUN_MANAGER_INIT_ACTION_LIST: u32 = 0x005e_2730;
/// `005ac020(script)`: builds the variable list of a script.
const SCRIPT_BUILD_LOCALS: u32 = 0x005a_c020;
/// `RtlEnterCriticalSection` wrapper `004538a0(cs, 0)` and its leave
/// `004538c0(cs)` (`RtlLeaveCriticalSection` in the engine map).
const CRITICAL_SECTION_ENTER: u32 = 0x0045_38a0;
const CRITICAL_SECTION_LEAVE: u32 = 0x0045_38c0;
/// `__RTDynamicCast(object, 0, source type, target type, 0)`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `TESFullName::GetFullName(form)` (`cdecl`) and the name of a reference's
/// base form (`0055d520(reference)`).
const FULL_NAME_OF_FORM: u32 = 0x0048_2720;
const FULL_NAME_OF_REFERENCE: u32 = 0x0055_d520;
/// The type descriptors the cast in `SetVariable` uses.
const RTTI_SOURCE_FORM: u32 = 0x0118_3028;
const RTTI_TARGET_REFERENCE: u32 = 0x0118_41cc;

/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\TESScript.cpp"`.
const SOURCE_FILE: u32 = 0x0103_6d58;
/// Formats of the messages (`Error` and `005b5e40`), by what they report.
const FORMAT_VARIABLE_NOT_FOUND: u32 = 0x0103_6d98;
const FORMAT_SET_VARIABLE_NO_SCRIPT: u32 = 0x0103_6e2c;
const FORMAT_SET_VARIABLE_FORM: u32 = 0x0103_6e68;
const FORMAT_SET_VARIABLE_NAMED: u32 = 0x0103_6f00;
const FORMAT_SET_VARIABLE_REF: u32 = 0x0103_6fa0;
const FORMAT_SET_VARIABLE_IN_SCRIPT: u32 = 0x0103_7040;
/// `"UNKNOWN"`.
const UNKNOWN_SCRIPT_NAME: u32 = 0x0101_5890;
/// `GetSaveSize()` messages: without and with the form being saved.
const FORMAT_SAVE_SIZE_PLAIN: u32 = 0x0101_2c78;
const FORMAT_SAVE_SIZE_FORM: u32 = 0x0101_2cb0;
/// `SaveGame()` messages: without and with the form being saved.
const FORMAT_SAVE_PLAIN: u32 = 0x0101_536c;
const FORMAT_SAVE_FORM: u32 = 0x0101_53a0;
/// The save block is larger than a short.
const FORMAT_BLOCK_TOO_LARGE: u32 = 0x0101_5318;
/// `LoadGame` block header errors: without and with the loading form.
const FORMAT_BLOCK_HEADER_PLAIN: u32 = 0x0101_56a8;
const FORMAT_BLOCK_HEADER_FORM: u32 = 0x0101_5718;
/// `LoadGame` buffer overrun/underrun messages, without and with the form.
const FORMAT_OVERRUN_PLAIN: u32 = 0x0101_54a0;
const FORMAT_UNDERRUN_PLAIN: u32 = 0x0101_5440;
const FORMAT_OVERRUN_FORM: u32 = 0x0101_5588;
const FORMAT_UNDERRUN_FORM: u32 = 0x0101_5500;
/// `"BLOK"` as a little-endian word: the marker that starts a save block.
const BLOCK_MARKER: u32 = 0x424c_4f4b;

/// The source line of the scope guard in `fn_005a8d20`.
const ACTION_SCOPE_LINE: u32 = 300;
/// Fields of the TLS block this unit uses: the last variable lookup of
/// `ScriptLocals` (the Xbox PDB's statics `pLastScriptLocals`,
/// `iLastVarSearchID` and `pLastVar`), `Script::pLastRefSearchScript` and the
/// byte `Script::m_bSystemOutput`.
const TLS_LAST_VARIABLE: u32 = 0x270;
const TLS_LAST_VARIABLE_ID: u32 = 0x274;
const TLS_LAST_LOCALS: u32 = 0x278;
const TLS_LAST_REF_SEARCH_SCRIPT: u32 = 0x288;
const TLS_SYSTEM_OUTPUT: u32 = 0x268;

/// The critical section that guards the pending reference lists.
const SCRIPT_REF_LIST_CRITICAL_SECTION: u32 = 0x011c_acf8;
/// The pending reference lists (module notes).
const REFERENCES_TO_ENABLE: u32 = 0x011c_aca8;
const REFERENCES_TO_DISABLE: u32 = 0x011c_ac98;
const REFERENCES_TO_DELETE: u32 = 0x011c_acb8;
const REFERENCES_TO_FADE_DISABLE: u32 = 0x011c_ace8;
/// `TESObjectREFR` methods used when the delayed actions run: enable
/// (`00573f40`) and disable (`TESObjectREFR::Disable`, `00574400`), and the
/// test of a flag in the word at +0x30 (`00456630(object, mask)`).
const REFERENCE_ENABLE: u32 = 0x0057_3f40;
const REFERENCE_DISABLE: u32 = 0x0057_4400;
const FLAGS_TEST: u32 = 0x0045_6630;
/// Byte that is non-zero while the fade-out of a disabled reference counts.
const FADE_ENABLED_BYTE: u32 = 0x011a_d7b4;
/// `0.0` as a double.
const ZERO_DOUBLE: u32 = 0x0101_2060;
/// `Script` helpers of the neighbouring functions that `CopyResultScript`
/// calls: `SetCompileData(data size, data)`, the copy of the referenced
/// objects and of the variables (`cdecl(source list, destination list,
/// script)`) and `SetText(text)`.
const SCRIPT_SET_COMPILE_DATA: u32 = 0x005a_ae70;
const SCRIPT_COPY_REF_OBJECTS: u32 = 0x005a_b7f0;
const SCRIPT_COPY_VARIABLES: u32 = 0x005a_b930;
const SCRIPT_SET_TEXT: u32 = 0x005a_be50;
/// Accessors of a `Script`: the header (`00500940`, +0x18), the compile data
/// (`00671d10`, +0x30), the text (`0055b980`, +0x2C) and the flag test
/// (`004013e0`, bit 3 of the form flags).
const SCRIPT_HEADER_OF: u32 = 0x0050_0940;
const SCRIPT_COMPILE_DATA_OF: u32 = 0x0067_1d10;
const SCRIPT_TEXT_OF: u32 = 0x0055_b980;
const FORM_FLAG_BIT_3: u32 = 0x0040_13e0;
/// `TESForm::TESForm` and `TESForm::~TESForm`, the form-type setter
/// (`004f15a0(form, type)`) and `00484ab0(script, flag)`.
const TES_FORM_CONSTRUCT: u32 = 0x0048_3370;
const TES_FORM_DESTRUCT: u32 = 0x0048_3630;
const SET_FORM_TYPE: u32 = 0x004f_15a0;
const SCRIPT_SET_QUEST_SCRIPT_FLAG: u32 = 0x0048_4ab0;
/// Deleting destructor `0061cb30(item, flags)` of the variable list's items,
/// and `007d9ec0(item)`, the body of the destructor of a
/// `SCRIPT_REFERENCED_OBJECT` (its string member).
const SCRIPT_VARIABLE_DELETE: u32 = 0x0061_cb30;
const SCRIPT_REFERENCED_OBJECT_DESTRUCT: u32 = 0x007d_9ec0;
/// `Script`'s vtable.
const SCRIPT_VTABLE: u32 = 0x0103_7094;

layout! {
    /// `ScriptLocals` (Xbox PDB), 0x14 bytes on both builds: the variables and
    /// the action flags of one running script instance.
    pub struct ScriptLocals: 0x14 {
        /// `m_pMasterScript` (Xbox PDB): the `Script` it belongs to.
        0x00 m_pMasterScript: Ptr,
        /// `m_cFlags` (Xbox PDB).
        0x04 m_cFlags: u8,
        /// `m_pActionList` (Xbox PDB): `BSSimpleList<ACTION_OBJECT *> *`.
        0x08 m_pActionList: Ptr,
        /// `m_pLocalList` (Xbox PDB): `BSSimpleList<SCRIPT_LOCAL *> *`.
        0x0C m_pLocalList: Ptr,
        /// `m_pScriptEffectData` (Xbox PDB): `SCRIPT_EFFECT_DATA *` (8 bytes).
        0x10 m_pScriptEffectData: Ptr,
    }

    /// `ACTION_OBJECT` (Xbox PDB), 8 bytes: a form and its action flags.
    pub struct ActionObject: 0x08 {
        /// `pForm` (Xbox PDB).
        0x00 pForm: Ptr,
        /// `iFlags` (Xbox PDB).
        0x04 iFlags: u32,
    }

    /// `SCRIPT_LOCAL` (Xbox PDB): one script variable.
    pub struct ScriptLocal: 0x18 {
        /// `uiID` (Xbox PDB).
        0x00 uiID: u32,
        /// `fValue` (Xbox PDB).
        0x08 fValue: f64,
        /// `bIsInteger` (Xbox PDB).
        0x10 bIsInteger: bool,
    }

    /// `SCRIPT_REFERENCED_OBJECT` (Xbox PDB), 0x10 bytes.
    pub struct ScriptReferencedObject: 0x10 {
        /// `uiVariableID` (Xbox PDB).
        0x0C uiVariableID: u32,
    }

    /// `SCRIPT_HEADER` (Xbox PDB), 0x14 bytes.
    pub struct ScriptHeader: 0x14 {
        /// `variableCount` (Xbox PDB).
        0x00 variableCount: u32,
        /// `refObjectCount` (Xbox PDB).
        0x04 refObjectCount: u32,
        /// `dataSize` (Xbox PDB).
        0x08 dataSize: u32,
        /// `m_uiLastID` (Xbox PDB).
        0x0C m_uiLastID: u32,
        /// `bIsQuestScript` (Xbox PDB).
        0x10 bIsQuestScript: bool,
        /// `bIsMagicEffectScript` (Xbox PDB).
        0x11 bIsMagicEffectScript: bool,
        /// `bIsCompiled` (Xbox PDB).
        0x12 bIsCompiled: bool,
    }

    /// `Script` (Xbox PDB), 0x54 bytes on PC (0x64 on Xbox, whose `TESForm`
    /// is 0x10 larger): the fields are the PDB's offsets minus 0x10.
    pub struct Script: 0x54 {
        /// `m_header` (Xbox PDB).
        0x18 m_header: Inline<ScriptHeader>,
        /// `m_text` (Xbox PDB).
        0x2C m_text: Ptr,
        /// `m_data` (Xbox PDB).
        0x30 m_data: Ptr,
        /// `fProfilerTimer` (Xbox PDB).
        0x34 fProfilerTimer: f32,
        /// `fQuestScriptDelay` (Xbox PDB).
        0x38 fQuestScriptDelay: f32,
        /// `fQuestScriptGetSecondsBuffer` (Xbox PDB).
        0x3C fQuestScriptGetSecondsBuffer: f32,
        /// `pOwnerQuest` (Xbox PDB).
        0x40 pOwnerQuest: Ptr,
        /// `listRefObjects` (Xbox PDB): a `BSSimpleList` embedded here.
        0x44 listRefObjects: Inline<BSSimpleList>,
        /// `listVariables` (Xbox PDB): a `BSSimpleList` embedded here.
        0x4C listVariables: Inline<BSSimpleList>,
    }
}

// ---- Small helpers ---------------------------------------------------------

/// The item word of a list node (`006815c0` gives its address).
fn list_item(e: &mut Engine, node: u32) -> u32 {
    let slot = e.call(LIST_ITEM_SLOT, &args![node]).u32();
    e.mem.u32(slot)
}

/// The next node of a list node.
fn list_next(e: &mut Engine, node: u32) -> u32 {
    e.call(LIST_NEXT, &args![node]).u32()
}

/// Whether the list starting at `node` has no item (`008256d0`).
fn list_is_empty(e: &mut Engine, node: u32) -> bool {
    e.call(LIST_IS_EMPTY, &args![node]).bool()
}

fn delete_block(e: &mut Engine, block: u32) {
    e.call(OPERATOR_DELETE, &args![block]);
}

fn new_block(e: &mut Engine, size: u32) -> u32 {
    e.call(OPERATOR_NEW, &args![size]).u32()
}

/// `new BSSimpleList` (8 bytes): the block is constructed unless the
/// allocation failed (a null block gives a null list).
fn new_list(e: &mut Engine) -> u32 {
    let block = new_block(e, 8);
    if block != 0 {
        e.call(LIST_CONSTRUCT, &args![block]).u32()
    } else {
        0
    }
}

/// `Error(...)`/`005b5e40(...)` with the words the game pushes.
fn error_log(e: &mut Engine, words: &[u32]) {
    e.call(ERROR_LOG, words);
}

fn script_log(e: &mut Engine, words: &[u32]) {
    e.call(SCRIPT_LOG, words);
}

fn save_load_game(e: &Engine) -> u32 {
    e.global::<u32>(SAVE_LOAD_GAME_GLOBAL)
}

fn use_save_game_blocks(e: &mut Engine) -> bool {
    let save_load = save_load_game(e);
    e.call(USE_SAVE_GAME_BLOCKS, &args![save_load]).bool()
}

fn file_version(e: &mut Engine) -> u8 {
    let save_load = save_load_game(e);
    e.call(SAVE_FILE_VERSION, &args![save_load]).u8()
}

fn stream_position(e: &mut Engine) -> u32 {
    let save_load = save_load_game(e);
    e.call(STREAM_POSITION, &args![save_load]).u32()
}

/// Whether the "log the save sizes" byte is set.
fn save_size_logging(e: &mut Engine) -> bool {
    let flag = e
        .call(FLAG_BYTE_ADDRESS, &args![SAVE_SIZE_LOG_OBJECT])
        .u32();
    e.mem.u8(flag) != 0
}

fn stream_write(e: &mut Engine, data: u32, size: u32) {
    let save_load = save_load_game(e);
    e.call(STREAM_WRITE, &args![save_load, data, size]);
}

fn stream_read(e: &mut Engine, data: u32, size: u32) {
    let save_load = save_load_game(e);
    e.call(STREAM_READ, &args![save_load, data, size]);
}

/// The type name of a form (virtual slot `+0x130`, no arguments).
fn form_type_name(e: &mut Engine, form: u32) -> u32 {
    e.vcall(form, 0x130, &args![]).u32()
}

/// Runs `body` between the scope guard `00404eb0(guard, 0x15, 1, file, line)`
/// and `00404ee0(guard)` (the guard is a local of the game).
fn with_scope_guard<R>(e: &mut Engine, line: u32, body: impl FnOnce(&mut Engine) -> R) -> R {
    e.with_stack(4, |e, guard| {
        e.call(
            SCOPE_GUARD_OPEN,
            &args![guard, 0x15u32, 1u32, SOURCE_FILE, line],
        );
        let result = body(e);
        e.call(SCOPE_GUARD_CLOSE, &args![guard]);
        result
    })
}

/// Runs `body` with the pending-reference critical section held
/// (`004538a0(cs, 0)` ... `004538c0(cs)`).
fn with_reference_lock<R>(e: &mut Engine, body: impl FnOnce(&mut Engine) -> R) -> R {
    e.call(
        CRITICAL_SECTION_ENTER,
        &args![SCRIPT_REF_LIST_CRITICAL_SECTION, 0u32],
    );
    let result = body(e);
    e.call(
        CRITICAL_SECTION_LEAVE,
        &args![SCRIPT_REF_LIST_CRITICAL_SECTION],
    );
    result
}

/// Empties and deletes one of the two lists a `ScriptLocals` owns: deletes
/// each item as it is taken off the front, deletes the list and clears the
/// field.
fn free_item_list(e: &mut Engine, this: Ptr<ScriptLocals>, field: Field<ScriptLocals, Ptr>) {
    if e.get(this, field).is_null() {
        return;
    }
    loop {
        let list = e.get(this, field).addr();
        let item = list_item(e, list);
        if item == 0 {
            break;
        }
        e.call(LIST_POP_FIRST, &args![list]);
        delete_block(e, item);
    }
    let list = e.get(this, field);
    if !list.is_null() {
        e.call(LIST_DELETE, &args![list, 1u32]);
    }
    e.set(this, field, Ptr::NULL);
}

// Translated from 005a8bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The destructor body of `ScriptLocals` (the engine map names it
/// `ScriptLocals::ScriptLocals`): empties the action list and the variable
/// list and frees the script effect data.
pub fn fn_005a8bc0(e: &mut Engine, this: Ptr<ScriptLocals>) {
    fn_005a8c00(e, this);
    fn_005a8c90(e, this);
    let effect_data = e.get(this, ScriptLocals::m_pScriptEffectData);
    if !effect_data.is_null() {
        delete_block(e, effect_data.addr());
    }
}

// Translated from 005a8c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Frees every `ACTION_OBJECT` and the action list.
pub fn fn_005a8c00(e: &mut Engine, this: Ptr<ScriptLocals>) {
    free_item_list(e, this, ScriptLocals::m_pActionList);
}

// Translated from 005a8c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Frees every `SCRIPT_LOCAL` and the variable list.
pub fn fn_005a8c90(e: &mut Engine, this: Ptr<ScriptLocals>) {
    free_item_list(e, this, ScriptLocals::m_pLocalList);
}

// Translated from 005a8d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Makes sure the action list has an entry for `form` (flags 0): creates the
/// list on first use and adds a new `ACTION_OBJECT` at its head when the
/// form has none. Runs inside a scope guard (kind `0x15`, line 300).
pub fn fn_005a8d20(e: &mut Engine, this: Ptr<ScriptLocals>, form: u32) {
    with_scope_guard(e, ACTION_SCOPE_LINE, |e| {
        if e.get(this, ScriptLocals::m_pActionList).is_null() {
            let list = new_list(e);
            e.set(this, ScriptLocals::m_pActionList, Ptr::new(list));
        }
        let action = fn_005a90b0(e, this, form, false);
        if action.is_null() {
            let block = new_block(e, 8);
            e.mem.set_u32(block, form);
            e.mem.set_u32(block + 4, 0);
            let list = e.get(this, ScriptLocals::m_pActionList);
            e.with_stack(4, |e, slot| {
                e.mem.set_u32(slot.addr(), block);
                e.call(LIST_ADD_HEAD, &args![list, slot]);
            });
        }
    });
}

// Translated from 005a8e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ScriptLocals::SetActionFlag` (Xbox PDB): ORs `flags` into the entry of
/// `form` and into the entry with no form (the "any form" entry), both
/// looked up with the create flag set. False when there is no action list
/// or neither entry exists.
pub fn script_locals_set_action_flag(
    e: &mut Engine,
    this: Ptr<ScriptLocals>,
    form: u32,
    flags: u32,
) -> bool {
    if e.get(this, ScriptLocals::m_pActionList).is_null() {
        return false;
    }
    let mut found = false;
    for target in [form, 0] {
        let action = fn_005a90b0(e, this, target, true).cast::<ActionObject>();
        if !action.is_null() {
            let old = e.get(action, ActionObject::iFlags);
            e.set(action, ActionObject::iFlags, old | flags);
            found = true;
        }
    }
    found
}

// Translated from 005a8ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the flags of every entry of the action list.
pub fn fn_005a8ea0(e: &mut Engine, this: Ptr<ScriptLocals>) {
    let mut node = e.get(this, ScriptLocals::m_pActionList).addr();
    while node != 0 {
        let item = list_item(e, node);
        if item == 0 {
            break;
        }
        node = list_next(e, node);
        e.set(Ptr::<ActionObject>::new(item), ActionObject::iFlags, 0);
    }
}

// Translated from 005a8ef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the entry of `form` has any of the flags in `mask`; a form with no
/// entry gets one (`fn_005a8d20`) and the answer is false.
pub fn fn_005a8ef0(e: &mut Engine, this: Ptr<ScriptLocals>, form: u32, mask: u32) -> bool {
    let action = fn_005a90b0(e, this, form, false).cast::<ActionObject>();
    if action.is_null() {
        fn_005a8d20(e, this, form);
        return false;
    }
    e.get(action, ActionObject::iFlags) & mask != 0
}

// Translated from 005a8f40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Replaces the action list with a copy of the action list of `source` (an
/// object whose +8 word is an action list): each `ACTION_OBJECT` is copied
/// (form and flags) and appended; the new list is created on the first item.
pub fn fn_005a8f40(e: &mut Engine, this: Ptr<ScriptLocals>, source: u32) {
    if !e.get(this, ScriptLocals::m_pActionList).is_null() {
        fn_005a8c00(e, this);
    }
    if source == 0 {
        return;
    }
    let mut node = e.call(ACTION_LIST_OF, &args![source]).u32();
    if node == 0 || list_is_empty(e, node) {
        return;
    }
    let mut destination = e.get(this, ScriptLocals::m_pActionList).addr();
    while node != 0 {
        let original = list_item(e, node);
        if original == 0 {
            break;
        }
        let copy = new_block(e, 8);
        let form = e.mem.u32(original);
        let flags = e.mem.u32(original + 4);
        e.mem.set_u32(copy, form);
        e.mem.set_u32(copy + 4, flags);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), copy);
            if destination == 0 {
                let list = new_list(e);
                e.set(this, ScriptLocals::m_pActionList, Ptr::new(list));
                e.call(LIST_ADD_HEAD, &args![list, slot]);
                destination = e.get(this, ScriptLocals::m_pActionList).addr();
            } else {
                e.call(LIST_ADD_TAIL, &args![destination, slot]);
                destination = list_next(e, destination);
            }
        });
        node = list_next(e, node);
    }
}

// Translated from 005a90b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds the `ACTION_OBJECT` of `form` in the action list. With `create`
/// set and no match, returns the last entry that has no form instead.
/// Null when there is no list.
pub fn fn_005a90b0(e: &mut Engine, this: Ptr<ScriptLocals>, form: u32, create: bool) -> Ptr {
    let mut node = e.get(this, ScriptLocals::m_pActionList).addr();
    if node == 0 {
        return Ptr::NULL;
    }
    let mut empty_entry = 0u32;
    while node != 0 {
        let item = list_item(e, node);
        if item == 0 {
            break;
        }
        let entry_form = e.mem.u32(item);
        if entry_form == form {
            return Ptr::new(item);
        }
        if entry_form == 0 {
            empty_entry = item;
        }
        node = list_next(e, node);
    }
    if create {
        Ptr::new(empty_entry)
    } else {
        Ptr::NULL
    }
}

// Translated from 005a9140 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ScriptLocals::GetVariable` (Xbox PDB): the value of the variable `id`.
/// The last lookup is cached in the TLS block (locals, id, variable);
/// otherwise the variable list is searched. A missing variable is logged
/// (with the name of `script`, "UNKNOWN" when null, and the line in
/// `011cae50`) and reads 0.
pub fn script_locals_get_variable(
    e: &mut Engine,
    this: Ptr<ScriptLocals>,
    id: u32,
    script: Ptr,
) -> f64 {
    let tls = e.tls();
    let mut variable = 0u32;
    if e.mem.u32(tls + TLS_LAST_LOCALS) == this.addr()
        && e.mem.u32(tls + TLS_LAST_VARIABLE_ID) == id
    {
        variable = e.mem.u32(tls + TLS_LAST_VARIABLE);
    }
    if variable == 0 {
        let mut node = e.call(LOCAL_LIST_OF, &args![this]).u32();
        while node != 0 && !list_is_empty(e, node) {
            let candidate = list_item(e, node);
            if e.mem.u32(candidate) == id {
                variable = candidate;
                break;
            }
            node = list_next(e, node);
        }
    }
    e.mem.set_u32(tls + TLS_LAST_LOCALS, this.addr());
    e.mem.set_u32(tls + TLS_LAST_VARIABLE_ID, id);
    e.mem.set_u32(tls + TLS_LAST_VARIABLE, variable);
    if variable != 0 {
        return e.mem.f64(variable + 8);
    }
    let name = if script.is_null() {
        UNKNOWN_SCRIPT_NAME
    } else {
        form_type_name(e, script.addr())
    };
    let line: u32 = e.global(SCRIPT_LINE_GLOBAL);
    script_log(e, &args![FORMAT_VARIABLE_NOT_FOUND, id, name, line]);
    0.0
}

// Translated from 005a9290 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ScriptLocals::SetVariable` (Xbox PDB): stores `value` in the variable
/// `id`. A missing variable is logged: with the master script's name when
/// there is one; otherwise, while loading (`0047c850` on the save/load game
/// object, a stub that returns 0), with the file name and the form being
/// loaded; otherwise with the id alone.
pub fn script_locals_set_variable(e: &mut Engine, this: Ptr<ScriptLocals>, id: u32, value: f64) {
    let mut node = e.call(LOCAL_LIST_OF, &args![this]).u32();
    while node != 0 {
        let variable = list_item(e, node);
        if variable == 0 {
            break;
        }
        node = list_next(e, node);
        if e.mem.u32(variable) == id {
            e.mem.set_f64(variable + 8, value);
            return;
        }
    }
    let mut log_generic = true;
    let master = e.get(this, ScriptLocals::m_pMasterScript);
    if !master.is_null() {
        let name = form_type_name(e, master.addr());
        script_log(e, &args![FORMAT_SET_VARIABLE_IN_SCRIPT, id, name]);
        log_generic = false;
    } else {
        let save_load = save_load_game(e);
        if e.call(IS_LOADING_STUB, &args![save_load]).bool() {
            let header = e.call(LOADING_FORM_HEADER, &args![save_load]).u32();
            if header != 0 {
                let header_id = e.mem.u32(header);
                let form = e.call(LOOKUP_FORM_BY_ID, &args![header_id]).u32();
                let reference = e
                    .call(
                        RT_DYNAMIC_CAST,
                        &args![form, 0u32, RTTI_SOURCE_FORM, RTTI_TARGET_REFERENCE, 0u32],
                    )
                    .u32();
                let name = if reference != 0 {
                    e.call(FULL_NAME_OF_REFERENCE, &args![reference]).u32()
                } else {
                    e.call(FULL_NAME_OF_FORM, &args![form]).u32()
                };
                let file_name = fn_005a9460(e, Ptr::new(save_load));
                if form != 0 && name != 0 {
                    let format = if reference != 0 {
                        FORMAT_SET_VARIABLE_REF
                    } else {
                        FORMAT_SET_VARIABLE_NAMED
                    };
                    script_log(e, &args![format, file_name, id, name, header_id]);
                } else {
                    script_log(
                        e,
                        &args![FORMAT_SET_VARIABLE_FORM, file_name, id, header_id],
                    );
                }
                log_generic = false;
            }
        }
    }
    if log_generic {
        script_log(e, &args![FORMAT_SET_VARIABLE_NO_SCRIPT, id]);
    }
}

// Translated from 005a9460 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at +0x1C4 of the save/load game object: the name of the file
/// being loaded.
pub fn fn_005a9460(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x1c4)
}

// Translated from 005a9480 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether one of the master script's referenced objects has the variable id
/// `variable_id` (such a variable holds a form reference, saved as a form id).
pub fn fn_005a9480(e: &mut Engine, this: Ptr<ScriptLocals>, variable_id: u32) -> bool {
    let master = e.get(this, ScriptLocals::m_pMasterScript);
    if master.is_null() {
        return false;
    }
    let mut node = e.call(SCRIPT_REF_OBJECT_LIST, &args![master]).u32();
    while node != 0 {
        let object = Ptr::<ScriptReferencedObject>::new(list_item(e, node));
        if object.is_null() {
            break;
        }
        if e.get(object, ScriptReferencedObject::uiVariableID) == variable_id {
            return true;
        }
        node = list_next(e, node);
    }
    false
}

/// Logs a save size message with the form being saved: `size`, the form id,
/// its type name, its flags (word at +5 of the header), the line and the
/// source file; without a header only `size`, line and file.
fn log_saved_size(e: &mut Engine, header: u32, size: u32, line: u32, formats: (u32, u32)) {
    if header != 0 {
        let header_id = e.mem.u32(header);
        let form = e.call(LOOKUP_FORM_BY_ID, &args![header_id]).u32();
        let flags = e.mem.u32(header + 5);
        let name = form_type_name(e, form);
        error_log(
            e,
            &args![formats.0, size, header_id, name, flags, line, SOURCE_FILE],
        );
    } else {
        error_log(e, &args![formats.1, size, line, SOURCE_FILE]);
    }
}

// Translated from 005a94f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The size in bytes (a 16-bit count) the old save format needs for these
/// locals: 2 for the variable count (plus 6 when blocks are used), 8 or 12
/// per non-zero variable (8 when it is a form reference), 1 for the
/// effect-data flag (plus 8 when there is effect data). Logs the size when
/// size logging is on.
pub fn fn_005a94f0(e: &mut Engine, this: Ptr<ScriptLocals>) -> u16 {
    let mut size: u16 = 0;
    if use_save_game_blocks(e) {
        size = size.wrapping_add(6);
    }
    size = size.wrapping_add(2);
    let mut node = e.get(this, ScriptLocals::m_pLocalList).addr();
    while node != 0 {
        let variable = list_item(e, node);
        if variable != 0 && e.mem.f64(variable + 8) != 0.0 {
            let id = e.mem.u32(variable);
            if fn_005a9480(e, this, id) {
                size = size.wrapping_add(8);
            } else {
                size = size.wrapping_add(0xc);
            }
        }
        node = list_next(e, node);
    }
    size = size.wrapping_add(1);
    if !e.get(this, ScriptLocals::m_pScriptEffectData).is_null() {
        size = size.wrapping_add(8);
    }
    if save_size_logging(e) {
        let save_load = save_load_game(e);
        let header = e.call(SAVING_FORM_HEADER, &args![save_load]).u32();
        log_saved_size(
            e,
            header,
            size as u32,
            0x27e,
            (FORMAT_SAVE_SIZE_FORM, FORMAT_SAVE_SIZE_PLAIN),
        );
    }
    size
}

// Translated from 005a9670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the locals in the old save format straight to the save/load game
/// stream: an optional block (marker `BLOK` and a 16-bit block size patched
/// at the end), a 16-bit variable count (patched), each non-zero variable
/// (id and value, or the id with its top bit set and the form id for a form
/// reference), a flag for the effect data and the effect data.
pub fn fn_005a9670(e: &mut Engine, this: Ptr<ScriptLocals>) {
    e.with_stack(0x20, |e, frame| old_save_body(e, this, frame.addr()));
}

fn old_save_body(e: &mut Engine, this: Ptr<ScriptLocals>, frame: u32) {
    let marker = frame;
    let block_size = frame + 4;
    let count = frame + 8;
    let id_word = frame + 0xc;
    let numeric = frame + 0x10;
    let flag = frame + 0x14;
    let mut block_position = 0u32;
    let mut start = stream_position(e);
    if save_size_logging(e) {
        start = stream_position(e);
    }
    if use_save_game_blocks(e) {
        e.mem.set_u32(marker, BLOCK_MARKER);
        stream_write(e, marker, 4);
        block_position = stream_position(e);
        stream_write(e, block_size, 2);
    }
    e.mem.set_u16(count, 0);
    let count_position = stream_position(e);
    stream_write(e, count, 2);
    let mut node = e.get(this, ScriptLocals::m_pLocalList).addr();
    while node != 0 {
        let variable = list_item(e, node);
        if variable != 0 && e.mem.f64(variable + 8) != 0.0 {
            let id = e.mem.u32(variable);
            if fn_005a9480(e, this, id) {
                e.mem.set_u32(numeric, 0);
                e.call(GET_NUMERIC_ID_FROM_DOUBLE, &args![numeric, variable + 8]);
                e.mem.set_u32(id_word, id | 0x8000_0000);
                stream_write(e, id_word, 4);
                let save_load = save_load_game(e);
                e.call(STREAM_SAVE_NUMERIC_ID, &args![save_load, numeric, 4u32]);
            } else {
                stream_write(e, variable, 4);
                stream_write(e, variable + 8, 8);
            }
            let saved = e.mem.u16(count);
            e.mem.set_u16(count, saved.wrapping_add(1));
        }
        node = list_next(e, node);
    }
    let saved = e.mem.u16(count);
    e.mem.set_u16(count_position, saved);
    let effect_data = e.get(this, ScriptLocals::m_pScriptEffectData).addr();
    e.mem.set_u8(flag, (effect_data != 0) as u8);
    stream_write(e, flag, 1);
    if effect_data != 0 {
        stream_write(e, effect_data, 8);
    }
    if save_size_logging(e) {
        let end = stream_position(e);
        let save_load = save_load_game(e);
        let header = e.call(SAVING_FORM_HEADER, &args![save_load]).u32();
        log_saved_size(
            e,
            header,
            end.wrapping_sub(start),
            0x2b6,
            (FORMAT_SAVE_FORM, FORMAT_SAVE_PLAIN),
        );
    }
    if use_save_game_blocks(e) {
        let end = stream_position(e);
        if end > block_position.wrapping_add(0xffff) {
            script_log(e, &args![FORMAT_BLOCK_TOO_LARGE, SOURCE_FILE, 0x2b6u32]);
        }
        e.mem
            .set_u16(block_position, end.wrapping_sub(block_position) as u16);
    }
}

/// Logs a load error with the form being loaded: the optional leading
/// numbers, file, line, form id, type name, version byte and flags; or,
/// without a header, the leading numbers, file, line and the file version.
fn log_loading_form(
    e: &mut Engine,
    header: u32,
    form: u32,
    leading: &[u32],
    line: u32,
    formats: (u32, u32),
) {
    let mut words: Vec<u32> = Vec::new();
    if header != 0 {
        let flags = e.mem.u32(header + 5);
        let version = e.mem.u8(header + 9) as u32;
        let header_id = e.mem.u32(header);
        let name = form_type_name(e, form);
        words.push(formats.0);
        words.extend_from_slice(leading);
        words.extend_from_slice(&[SOURCE_FILE, line, header_id, name, version, flags]);
    } else {
        let version = file_version(e) as u32;
        words.push(formats.1);
        words.extend_from_slice(leading);
        words.extend_from_slice(&[SOURCE_FILE, line, version]);
    }
    script_log(e, &words);
}

/// The header of the form being loaded and the form it names (0 for both
/// when nothing is being loaded).
fn loading_form(e: &mut Engine) -> (u32, u32) {
    let save_load = save_load_game(e);
    let header = e.call(LOADING_FORM_HEADER, &args![save_load]).u32();
    let form = if header != 0 {
        let header_id = e.mem.u32(header);
        e.call(LOOKUP_FORM_BY_ID, &args![header_id]).u32()
    } else {
        0
    };
    (header, form)
}

// Translated from 005a9950 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the locals in the old save format (the inverse of `fn_005a9670`),
/// calling `SetVariable` for each saved variable. Files older than version
/// 0x75 store the id and value in two plain reads and never use form
/// references. Reports a wrong block header, and a block read that over- or
/// underran its recorded size.
pub fn fn_005a9950(e: &mut Engine, this: Ptr<ScriptLocals>) {
    e.with_stack(0x20, |e, frame| old_load_body(e, this, frame.addr()));
}

fn old_load_body(e: &mut Engine, this: Ptr<ScriptLocals>, frame: u32) {
    let marker = frame;
    let block_size = frame + 4;
    let count = frame + 8;
    let id_word = frame + 0xc;
    let value = frame + 0x10;
    let numeric = frame + 0x18;
    let flag = frame + 0x1c;
    let mut block_start = 0u32;
    if use_save_game_blocks(e) {
        stream_read(e, marker, 4);
        if e.mem.u32(marker) != BLOCK_MARKER {
            let (header, form) = loading_form(e);
            log_loading_form(
                e,
                header,
                form,
                &[],
                0x2bc,
                (FORMAT_BLOCK_HEADER_FORM, FORMAT_BLOCK_HEADER_PLAIN),
            );
        }
        block_start = stream_position(e);
        stream_read(e, block_size, 2);
    }
    stream_read(e, count, 2);
    let total = e.mem.u16(count) as i32;
    let mut index = 0i32;
    while index < total {
        e.mem.set_u32(id_word, 0);
        e.mem.set_f64(value, 0.0);
        if file_version(e) >= 0x75 {
            stream_read(e, id_word, 4);
            if e.mem.u32(id_word) & 0x8000_0000 != 0 {
                let id = e.mem.u32(id_word) & 0x7fff_ffff;
                e.mem.set_u32(id_word, id);
                let save_load = save_load_game(e);
                e.call(STREAM_LOAD_NUMERIC_ID, &args![save_load, numeric, 4u32]);
                e.call(PUT_NUMERIC_ID_IN_DOUBLE, &args![numeric, value]);
            } else {
                stream_read(e, value, 8);
            }
        }
        if file_version(e) < 0x75 {
            stream_read(e, id_word, 4);
            stream_read(e, value, 8);
        }
        let id = e.mem.u32(id_word);
        let variable = e.mem.f64(value);
        script_locals_set_variable(e, this, id, variable);
        index += 1;
    }
    stream_read(e, flag, 1);
    if e.mem.u8(flag) != 0 {
        let effect_data = new_block(e, 8);
        e.set(
            this,
            ScriptLocals::m_pScriptEffectData,
            Ptr::new(effect_data),
        );
        stream_read(e, effect_data, 8);
    }
    if use_save_game_blocks(e) {
        let current = stream_position(e);
        let (header, form) = loading_form(e);
        let expected = (e.mem.u16(block_size) as u32).wrapping_add(block_start);
        if current > expected {
            log_loading_form(
                e,
                header,
                form,
                &[current - expected],
                0x2eb,
                (FORMAT_OVERRUN_FORM, FORMAT_OVERRUN_PLAIN),
            );
        } else if current < expected {
            log_loading_form(
                e,
                header,
                form,
                &[expected - current],
                0x2eb,
                (FORMAT_UNDERRUN_FORM, FORMAT_UNDERRUN_PLAIN),
            );
        }
    }
}

// Translated from 005a9d00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Resets the locals: frees the variable list, clears every action flag,
/// rebuilds the variable list from the master script (when there is one) and
/// frees the effect data.
pub fn fn_005a9d00(e: &mut Engine, this: Ptr<ScriptLocals>) {
    fn_005a8c90(e, this);
    fn_005a8ea0(e, this);
    let master = e.get(this, ScriptLocals::m_pMasterScript);
    if !master.is_null() {
        let list = e.call(SCRIPT_BUILD_LOCALS, &args![master]).ptr::<()>();
        e.set(this, ScriptLocals::m_pLocalList, list);
    }
    let effect_data = e.get(this, ScriptLocals::m_pScriptEffectData);
    if !effect_data.is_null() {
        delete_block(e, effect_data.addr());
    }
    e.set(this, ScriptLocals::m_pScriptEffectData, Ptr::NULL);
}

// Translated from 005a9d60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the cache of the last variable lookup (locals 0, id `0xFFFFFFFF`,
/// variable 0).
pub fn fn_005a9d60(e: &mut Engine) {
    let tls = e.tls();
    e.mem.set_u32(tls + TLS_LAST_LOCALS, 0);
    e.mem.set_u32(tls + TLS_LAST_VARIABLE_ID, 0xffff_ffff);
    e.mem.set_u32(tls + TLS_LAST_VARIABLE, 0);
}

// Translated from 005a9db0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ScriptLocals::SaveGame` (Xbox PDB): writes the locals into a
/// `BGSSaveGameBuffer`: a variable-sized count, then each non-zero variable
/// (id and value, or the id with its top bit set and the saved form id for a
/// form reference), the effect-data flag and data, and a byte saying whether
/// the "any form" action entry has flag `0x1000`.
pub fn script_locals_save_game(e: &mut Engine, this: Ptr<ScriptLocals>, buffer: Ptr) {
    e.with_stack(0x10, |e, frame| {
        let id_word = frame.addr();
        let numeric = frame.addr() + 4;
        let flag = frame.addr() + 8;
        let mut count = 0u32;
        let position = e.call(BUFFER_START_SIZED_VALUE, &args![buffer]).u32();
        let mut node = e.get(this, ScriptLocals::m_pLocalList).addr();
        while node != 0 {
            let variable = list_item(e, node);
            if variable != 0 && e.mem.f64(variable + 8) != 0.0 {
                let id = e.mem.u32(variable);
                if fn_005a9480(e, this, id) {
                    e.mem.set_u32(numeric, 0);
                    e.call(GET_NUMERIC_ID_FROM_DOUBLE, &args![numeric, variable + 8]);
                    e.mem.set_u32(id_word, id | 0x8000_0000);
                    e.call(BUFFER_SAVE_BYTES, &args![buffer, id_word, 4u32, 0u32]);
                    let form_id = e.mem.u32(numeric);
                    e.call(BUFFER_SAVE_FORM_ID, &args![buffer, form_id, 0u32]);
                } else {
                    e.call(BUFFER_SAVE_BYTES, &args![buffer, variable, 4u32, 0u32]);
                    e.call(BUFFER_SAVE_BYTES, &args![buffer, variable + 8, 8u32, 0u32]);
                }
                count += 1;
            }
            node = list_next(e, node);
        }
        e.call(BUFFER_SAVE_SIZED_VALUE, &args![buffer, count, position]);
        let effect_data = e.get(this, ScriptLocals::m_pScriptEffectData).addr();
        e.mem.set_u8(flag, (effect_data != 0) as u8);
        e.call(BUFFER_SAVE_BYTES, &args![buffer, flag, 1u32, 0u32]);
        if effect_data != 0 {
            e.call(BUFFER_SAVE_BYTES, &args![buffer, effect_data, 8u32, 0u32]);
        }
        let any_form_flag = fn_005a8ef0(e, this, 0, 0x1000);
        e.mem.set_u8(flag, any_form_flag as u8);
        e.call(BUFFER_SAVE_BYTES, &args![buffer, flag, 1u32, 0u32]);
    });
}

// Translated from 005a9f20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ScriptLocals::LoadGame` (Xbox PDB): reads what `script_locals_save_game`
/// wrote from a `BGSLoadGameBuffer`: the variables (set with `SetVariable`;
/// a form reference id is read as a form id and put into the double), the
/// effect data, and, from buffer version 0x15, the "any form" action flag
/// byte. With no action list it is built by `ScriptRunManager::InitActionList`
/// for the master script, otherwise all flags are cleared; the saved flag
/// then sets `0x1000` on the "any form" entry.
pub fn script_locals_load_game(e: &mut Engine, this: Ptr<ScriptLocals>, buffer: Ptr) {
    e.with_stack(0x20, |e, frame| {
        let id_word = frame.addr();
        let numeric = frame.addr() + 4;
        let value = frame.addr() + 8;
        let effect_flag = frame.addr() + 0x10;
        let action_flag = frame.addr() + 0x11;
        let count = e.call(BUFFER_LOAD_SIZED_VALUE, &args![buffer]).u32();
        let mut index = 0u32;
        while index < count {
            e.mem.set_f64(value, 0.0);
            e.mem.set_u32(id_word, 0);
            e.call(BUFFER_LOAD_BYTES, &args![buffer, id_word, 4u32]);
            if e.mem.u32(id_word) & 0x8000_0000 != 0 {
                let id = e.mem.u32(id_word) & 0x7fff_ffff;
                e.mem.set_u32(id_word, id);
                e.mem.set_u32(numeric, 0);
                e.call(BUFFER_LOAD_FORM_ID_OV2, &args![buffer, numeric]);
                e.call(PUT_NUMERIC_ID_IN_DOUBLE, &args![numeric, value]);
            } else {
                e.call(BUFFER_LOAD_BYTES, &args![buffer, value, 8u32]);
            }
            let id = e.mem.u32(id_word);
            let variable = e.mem.f64(value);
            script_locals_set_variable(e, this, id, variable);
            index += 1;
        }
        e.mem.set_u8(effect_flag, 0);
        e.call(BUFFER_LOAD_BYTES, &args![buffer, effect_flag, 1u32]);
        if e.mem.u8(effect_flag) != 0 {
            let effect_data = new_block(e, 8);
            e.set(
                this,
                ScriptLocals::m_pScriptEffectData,
                Ptr::new(effect_data),
            );
            e.call(BUFFER_LOAD_BYTES, &args![buffer, effect_data, 8u32]);
        }
        e.mem.set_u8(action_flag, 0);
        let version = e.vcall(buffer.addr(), 0, &args![]).u8();
        if version >= 0x15 {
            e.call(BUFFER_LOAD_BYTES, &args![buffer, action_flag, 1u32]);
        }
        if e.get(this, ScriptLocals::m_pActionList).is_null() {
            let master = e.get(this, ScriptLocals::m_pMasterScript);
            let second = e.vcall(buffer.addr(), 8, &args![]).u32();
            let manager = e.call(SCRIPT_RUN_MANAGER_INSTANCE, &args![]).u32();
            e.call(
                SCRIPT_RUN_MANAGER_INIT_ACTION_LIST,
                &args![manager, master, second, this],
            );
        } else {
            fn_005a8ea0(e, this);
        }
        if e.mem.u8(action_flag) != 0 {
            script_locals_set_action_flag(e, this, 0, 0x1000);
        }
    });
}

// Translated from 005aa090 (decompiled, FalloutNV.exe 1.4.0.525)
/// The same body as `fn_005a9d00` (reset the locals), for a method that
/// takes one stack word it never reads.
pub fn fn_005aa090(e: &mut Engine, this: Ptr<ScriptLocals>, _unused_0: u32) {
    fn_005a9d00(e, this);
}

// Translated from 005aa0f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::Script` (Xbox PDB): the form constructor, the `Script` vtable,
/// the two embedded lists, then the field initialization of `fn_005aa220`.
pub fn script_script(e: &mut Engine, this: Ptr<Script>) -> Ptr<Script> {
    e.call(TES_FORM_CONSTRUCT, &args![this]);
    e.mem.set_u32(this.addr(), SCRIPT_VTABLE);
    let ref_objects = this.at(Script::listRefObjects);
    e.call(LIST_CONSTRUCT, &args![ref_objects]);
    let variables = this.at(Script::listVariables);
    e.call(LIST_CONSTRUCT, &args![variables]);
    fn_005aa220(e, this);
    this
}

// Translated from 005aa170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of `Script`: destroys it and, with bit 0 of
/// `flags`, frees the block.
pub fn fn_005aa170(e: &mut Engine, this: Ptr<Script>, flags: u32) -> Ptr<Script> {
    fn_005aa1a0(e, this);
    if flags & 1 != 0 {
        delete_block(e, this.addr());
    }
    this
}

// Translated from 005aa1a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The destructor body of `Script` (the decompiler calls it
/// `CSettingsStore::~CSettingsStore`, a wrong library match): sets the
/// vtable, releases the text, data and lists (`fn_005aa2a0`), destroys the
/// two embedded lists and the `TESForm` base.
pub fn fn_005aa1a0(e: &mut Engine, this: Ptr<Script>) {
    e.mem.set_u32(this.addr(), SCRIPT_VTABLE);
    fn_005aa2a0(e, this);
    let variables = this.at(Script::listVariables);
    e.call(LIST_DESTRUCT, &args![variables]);
    let ref_objects = this.at(Script::listRefObjects);
    e.call(LIST_DESTRUCT, &args![ref_objects]);
    e.call(TES_FORM_DESTRUCT, &args![this]);
}

// Translated from 005aa220 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializes the plain fields of a `Script`: the 0x14-byte header zeroed,
/// text and data null, the system-output byte cleared, the owner quest null,
/// the three float timers 0.0, and the form type set to 0x11.
pub fn fn_005aa220(e: &mut Engine, this: Ptr<Script>) {
    let header = this.at(Script::m_header);
    e.call(MEMSET, &args![header, 0u32, 0x14u32]);
    e.set(this, Script::m_data, Ptr::NULL);
    e.set(this, Script::m_text, Ptr::NULL);
    let tls = e.tls();
    e.mem.set_u8(tls + TLS_SYSTEM_OUTPUT, 0);
    e.set(this, Script::pOwnerQuest, Ptr::NULL);
    e.set(this, Script::fProfilerTimer, 0.0);
    e.set(this, Script::fQuestScriptDelay, 0.0);
    e.set(this, Script::fQuestScriptGetSecondsBuffer, 0.0);
    e.call(SET_FORM_TYPE, &args![this, 0x11u32]);
}

// Translated from 005aa2a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Frees the data and text of a `Script` and empties its variable and
/// referenced-object lists.
pub fn fn_005aa2a0(e: &mut Engine, this: Ptr<Script>) {
    let data = e.get(this, Script::m_data);
    delete_block(e, data.addr());
    let text = e.get(this, Script::m_text);
    delete_block(e, text.addr());
    fn_005aa2e0(e, this);
    fn_005aa350(e, this);
}

// Translated from 005aa2e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes every item of the script's variable list (`listVariables`) and
/// removes it from the list, until the list is empty.
pub fn fn_005aa2e0(e: &mut Engine, this: Ptr<Script>) {
    let list = e.call(SCRIPT_VARIABLE_LIST, &args![this]).u32();
    while list != 0 && !list_is_empty(e, list) {
        let item = list_item(e, list);
        if item != 0 {
            e.call(SCRIPT_VARIABLE_DELETE, &args![item, 1u32]);
        }
        e.call(LIST_POP_FIRST, &args![list]);
    }
}

// Translated from 005aa350 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forgets this script as the last reference-search script (TLS +0x288),
/// then deletes every item of the referenced-object list (`listRefObjects`)
/// and removes it from the list, until the list is empty.
pub fn fn_005aa350(e: &mut Engine, this: Ptr<Script>) {
    let tls = e.tls();
    if e.mem.u32(tls + TLS_LAST_REF_SEARCH_SCRIPT) == this.addr() {
        e.mem.set_u32(tls + TLS_LAST_REF_SEARCH_SCRIPT, 0);
    }
    let list = e.call(SCRIPT_REF_OBJECT_LIST, &args![this]).u32();
    while list != 0 && !list_is_empty(e, list) {
        let item = list_item(e, list);
        if item != 0 {
            fn_005aa3f0(e, Ptr::new(item), 1);
        }
        e.call(LIST_POP_FIRST, &args![list]);
    }
}

// Translated from 005aa3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of `SCRIPT_REFERENCED_OBJECT` (the body
/// `007d9ec0` destroys its editor ID string): frees the block with bit 0 of
/// `flags`.
pub fn fn_005aa3f0(e: &mut Engine, this: Ptr<ScriptReferencedObject>, flags: u32) -> Ptr {
    e.call(SCRIPT_REFERENCED_OBJECT_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        delete_block(e, this.addr());
    }
    this.cast()
}

// Translated from 005aa420 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::CopyResultScript` (Xbox PDB): makes this script a copy of
/// `other` (nothing for null): copies the 0x14-byte header and clears its
/// first word, copies the compile data (`SetCompileData(data size, data)`),
/// the referenced objects and the variables (each after emptying its list),
/// the text (`SetText`, null when `other` has none), and the quest-script
/// flag (bit 3 of the form flags).
pub fn script_copy_result_script(e: &mut Engine, this: Ptr<Script>, other: Ptr<Script>) {
    if other.is_null() {
        return;
    }
    let other_header = e.call(SCRIPT_HEADER_OF, &args![other]).u32();
    let header = this.at(Script::m_header);
    e.call(MEMCPY, &args![header, other_header, 0x14u32]);
    e.set(header, ScriptHeader::variableCount, 0);
    let compile_data = e.call(SCRIPT_COMPILE_DATA_OF, &args![other]).u32();
    let data_size = e.get(header, ScriptHeader::dataSize);
    e.call(
        SCRIPT_SET_COMPILE_DATA,
        &args![this, data_size, compile_data],
    );
    fn_005aa350(e, this);
    let own_refs = e.call(SCRIPT_REF_OBJECT_LIST, &args![this]).u32();
    let other_refs = e.call(SCRIPT_REF_OBJECT_LIST, &args![other]).u32();
    e.call(SCRIPT_COPY_REF_OBJECTS, &args![other_refs, own_refs, this]);
    fn_005aa2e0(e, this);
    let own_vars = e.call(SCRIPT_VARIABLE_LIST, &args![this]).u32();
    let other_vars = e.call(SCRIPT_VARIABLE_LIST, &args![other]).u32();
    e.call(SCRIPT_COPY_VARIABLES, &args![other_vars, own_vars, this]);
    if e.call(SCRIPT_TEXT_OF, &args![other]).u32() != 0 {
        let text = e.call(SCRIPT_TEXT_OF, &args![other]).u32();
        e.call(SCRIPT_SET_TEXT, &args![this, text]);
    } else {
        e.call(SCRIPT_SET_TEXT, &args![this, 0u32]);
    }
    let flag = e.call(FORM_FLAG_BIT_3, &args![other]).u8() as u32;
    e.call(SCRIPT_SET_QUEST_SCRIPT_FLAG, &args![this, flag]);
}

// Translated from 005aa500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::AddPendingDisabledReference` (Xbox PDB): under the lock, takes
/// `reference` off the to-enable list, then (unless null) adds it to the
/// to-disable list, or with `fade` to the fade-disable list, when it is not
/// already pending there. A null reference is only locked around.
pub fn script_add_pending_disabled_reference(e: &mut Engine, reference: u32, fade: u8) {
    with_reference_lock(e, |e| {
        if reference == 0 {
            return;
        }
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), reference);
            e.call(LIST_REMOVE, &args![REFERENCES_TO_ENABLE, slot]);
            if fade == 0 {
                if !script_is_pending_disabled_reference(e, reference) {
                    e.call(LIST_ADD_HEAD, &args![REFERENCES_TO_DISABLE, slot]);
                }
            } else if !script_is_pending_fade_disabled_reference(e, reference) {
                e.call(LIST_ADD_HEAD, &args![REFERENCES_TO_FADE_DISABLE, slot]);
            }
        });
    });
}

// Translated from 005aa580 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::AddPendingEnabledReference` (Xbox PDB): under the lock, takes
/// `reference` off the to-disable list and adds it to the to-enable list
/// unless it is already pending there. Null does nothing.
pub fn script_add_pending_enabled_reference(e: &mut Engine, reference: u32) {
    with_reference_lock(e, |e| {
        if reference == 0 {
            return;
        }
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), reference);
            e.call(LIST_REMOVE, &args![REFERENCES_TO_DISABLE, slot]);
            if !script_is_pending_enabled_reference(e, reference) {
                e.call(LIST_ADD_HEAD, &args![REFERENCES_TO_ENABLE, slot]);
            }
        });
    });
}

// Translated from 005aa5d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::RemoveDelayedScriptActionReference` (Xbox PDB): under the lock,
/// takes `reference` off the to-enable, to-disable, fade-disable and
/// to-delete lists. Null does nothing (and does not lock).
pub fn script_remove_delayed_script_action_reference(e: &mut Engine, reference: u32) {
    if reference == 0 {
        return;
    }
    with_reference_lock(e, |e| {
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), reference);
            for list in [
                REFERENCES_TO_ENABLE,
                REFERENCES_TO_DISABLE,
                REFERENCES_TO_FADE_DISABLE,
                REFERENCES_TO_DELETE,
            ] {
                e.call(LIST_REMOVE, &args![list, slot]);
            }
        });
    });
}

/// Whether `reference` (non-null) is in `list`, under the lock.
fn is_pending_in(e: &mut Engine, list: u32, reference: u32) -> bool {
    with_reference_lock(e, |e| {
        let mut found = false;
        if reference != 0 {
            e.with_stack(4, |e, slot| {
                e.mem.set_u32(slot.addr(), reference);
                if e.call(LIST_CONTAINS, &args![list, slot]).bool() {
                    found = true;
                }
            });
        }
        found
    })
}

// Translated from 005aa630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsPendingDisabledReference` (Xbox PDB): under the lock, whether
/// the to-disable list contains `reference`.
pub fn script_is_pending_disabled_reference(e: &mut Engine, reference: u32) -> bool {
    is_pending_in(e, REFERENCES_TO_DISABLE, reference)
}

// Translated from 005aa680 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsPendingEnabledReference` (Xbox PDB): under the lock, whether
/// the to-enable list contains `reference`.
pub fn script_is_pending_enabled_reference(e: &mut Engine, reference: u32) -> bool {
    is_pending_in(e, REFERENCES_TO_ENABLE, reference)
}

// Translated from 005aa6d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsPendingFadeDisabledReference` (Xbox PDB): under the lock,
/// whether the fade-disable list contains `reference`.
pub fn script_is_pending_fade_disabled_reference(e: &mut Engine, reference: u32) -> bool {
    is_pending_in(e, REFERENCES_TO_FADE_DISABLE, reference)
}

// Translated from 005aa720 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::RunDelayedScriptActionsOnReferences` (Xbox PDB): under the lock
/// enables every reference of the to-enable list and disables every one of
/// the to-disable list (each taken off the front first); then walks the
/// fade-disable list: a reference whose virtual `+0x1D0` gives an object
/// whose virtual `+0x10` gives a target that `fn_005aa8c0` accepts stays in
/// the list, any other is disabled and removed; finally deletes (virtual
/// `+0x10` with 1) every reference of the to-delete list.
pub fn script_run_delayed_script_actions_on_references(e: &mut Engine) {
    with_reference_lock(e, |e| {
        while !list_is_empty(e, REFERENCES_TO_ENABLE) {
            let reference = list_item(e, REFERENCES_TO_ENABLE);
            e.call(LIST_POP_FIRST, &args![REFERENCES_TO_ENABLE]);
            e.call(REFERENCE_ENABLE, &args![reference]);
        }
        while !list_is_empty(e, REFERENCES_TO_DISABLE) {
            let reference = list_item(e, REFERENCES_TO_DISABLE);
            e.call(LIST_POP_FIRST, &args![REFERENCES_TO_DISABLE]);
            e.call(REFERENCE_DISABLE, &args![reference]);
        }
        let mut node = REFERENCES_TO_FADE_DISABLE;
        while node != 0 && !list_is_empty(e, node) {
            let mut following = list_next(e, node);
            let reference = list_item(e, node);
            let object = e.vcall(reference, 0x1d0, &args![]).u32();
            let target = if object != 0 {
                e.vcall(object, 0x10, &args![]).u32()
            } else {
                0
            };
            if target == 0 || !fn_005aa8c0(e, Ptr::new(target)) {
                e.call(REFERENCE_DISABLE, &args![reference]);
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), reference);
                    e.call(LIST_REMOVE, &args![REFERENCES_TO_FADE_DISABLE, slot]);
                });
                if node == REFERENCES_TO_FADE_DISABLE {
                    following = REFERENCES_TO_FADE_DISABLE;
                }
            }
            node = following;
        }
        while !list_is_empty(e, REFERENCES_TO_DELETE) {
            let reference = list_item(e, REFERENCES_TO_DELETE);
            e.call(LIST_POP_FIRST, &args![REFERENCES_TO_DELETE]);
            if reference != 0 {
                e.vcall(reference, 0x10, &args![1u32]);
            }
        }
    });
}

// Translated from 005aa8c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// False when the object does not have flag `0x8000` (`fn_005aa910`) and the
/// fade byte (`011ad7b4`) is set and the float at +0xB8 is not greater than
/// zero (an unordered value counts as not greater); true otherwise.
pub fn fn_005aa8c0(e: &mut Engine, this: Ptr) -> bool {
    if !fn_005aa910(e, this) && e.global::<u8>(FADE_ENABLED_BYTE) != 0 {
        let value = e.mem.f32(this.addr() + 0xb8) as f64;
        let zero: f64 = e.global(ZERO_DOUBLE);
        if value.partial_cmp(&zero) != Some(std::cmp::Ordering::Greater) {
            return false;
        }
    }
    true
}

// Translated from 005aa910 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the object has flag `0x8000` in the word at +0x30 (`00456630`).
pub fn fn_005aa910(e: &mut Engine, this: Ptr) -> bool {
    e.call(FLAGS_TEST, &args![this, 0x8000u32]).bool()
}

// Translated from 005aa930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SavePendingReferences` (Xbox PDB): under the lock, saves into
/// the buffer, for each of the to-enable, to-disable, to-delete and
/// fade-disable lists in that order, a variable-sized count followed by the
/// form id of every non-null reference.
pub fn script_save_pending_references(e: &mut Engine, buffer: Ptr) {
    with_reference_lock(e, |e| {
        for list in [
            REFERENCES_TO_ENABLE,
            REFERENCES_TO_DISABLE,
            REFERENCES_TO_DELETE,
            REFERENCES_TO_FADE_DISABLE,
        ] {
            let mut count = 0u32;
            let position = e.call(BUFFER_START_SIZED_VALUE, &args![buffer]).u32();
            let mut node = list;
            while node != 0 {
                let reference = list_item(e, node);
                if reference != 0 {
                    e.call(BUFFER_SAVE_FORM_ID_OV2, &args![buffer, reference, 0u32]);
                    count += 1;
                }
                node = list_next(e, node);
            }
            e.call(BUFFER_SAVE_SIZED_VALUE, &args![buffer, count, position]);
        }
    });
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x005a8bc0, fn_005a8bc0(Ptr<ScriptLocals>)),
        entry!(0x005a8c00, fn_005a8c00(Ptr<ScriptLocals>)),
        entry!(0x005a8c90, fn_005a8c90(Ptr<ScriptLocals>)),
        entry!(0x005a8d20, fn_005a8d20(Ptr<ScriptLocals>, u32)),
        entry!(
            0x005a8e20,
            script_locals_set_action_flag(Ptr<ScriptLocals>, u32, u32) -> bool
        ),
        entry!(0x005a8ea0, fn_005a8ea0(Ptr<ScriptLocals>)),
        entry!(0x005a8ef0, fn_005a8ef0(Ptr<ScriptLocals>, u32, u32) -> bool),
        entry!(0x005a8f40, fn_005a8f40(Ptr<ScriptLocals>, u32)),
        entry!(0x005a90b0, fn_005a90b0(Ptr<ScriptLocals>, u32, bool) -> Ptr),
        entry!(
            0x005a9140,
            script_locals_get_variable(Ptr<ScriptLocals>, u32, Ptr) -> f64
        ),
        entry!(
            0x005a9290,
            script_locals_set_variable(Ptr<ScriptLocals>, u32, f64)
        ),
        entry!(0x005a9460, fn_005a9460(Ptr) -> u32),
        entry!(0x005a9480, fn_005a9480(Ptr<ScriptLocals>, u32) -> bool),
        entry!(0x005a94f0, fn_005a94f0(Ptr<ScriptLocals>) -> u16),
        entry!(0x005a9670, fn_005a9670(Ptr<ScriptLocals>)),
        entry!(0x005a9950, fn_005a9950(Ptr<ScriptLocals>)),
        entry!(0x005a9d00, fn_005a9d00(Ptr<ScriptLocals>)),
        entry!(0x005a9d60, fn_005a9d60()),
        entry!(0x005a9db0, script_locals_save_game(Ptr<ScriptLocals>, Ptr)),
        entry!(0x005a9f20, script_locals_load_game(Ptr<ScriptLocals>, Ptr)),
        entry!(0x005aa090, fn_005aa090(Ptr<ScriptLocals>, u32)),
        entry!(0x005aa0f0, script_script(Ptr<Script>) -> Ptr<Script>),
        entry!(0x005aa170, fn_005aa170(Ptr<Script>, u32) -> Ptr<Script>),
        entry!(0x005aa1a0, fn_005aa1a0(Ptr<Script>)),
        entry!(0x005aa220, fn_005aa220(Ptr<Script>)),
        entry!(0x005aa2a0, fn_005aa2a0(Ptr<Script>)),
        entry!(0x005aa2e0, fn_005aa2e0(Ptr<Script>)),
        entry!(0x005aa350, fn_005aa350(Ptr<Script>)),
        entry!(
            0x005aa3f0,
            fn_005aa3f0(Ptr<ScriptReferencedObject>, u32) -> Ptr
        ),
        entry!(
            0x005aa420,
            script_copy_result_script(Ptr<Script>, Ptr<Script>)
        ),
        entry!(0x005aa500, script_add_pending_disabled_reference(u32, u8)),
        entry!(0x005aa580, script_add_pending_enabled_reference(u32)),
        entry!(
            0x005aa5d0,
            script_remove_delayed_script_action_reference(u32)
        ),
        entry!(
            0x005aa630,
            script_is_pending_disabled_reference(u32) -> bool
        ),
        entry!(
            0x005aa680,
            script_is_pending_enabled_reference(u32) -> bool
        ),
        entry!(
            0x005aa6d0,
            script_is_pending_fade_disabled_reference(u32) -> bool
        ),
        entry!(
            0x005aa720,
            script_run_delayed_script_actions_on_references()
        ),
        entry!(0x005aa8c0, fn_005aa8c0(Ptr) -> bool),
        entry!(0x005aa910, fn_005aa910(Ptr) -> bool),
        entry!(0x005aa930, script_save_pending_references(Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// The pages holding the globals this unit reads (the pending lists and
    /// their critical section, the save/load game pointer, the fade byte and
    /// the zero constant).
    const PAGES: [u32; 4] = [0x0101_2000, 0x011a_d000, 0x011c_a000, 0x011d_e000];

    /// An engine with test doubles for the callees outside this file: the
    /// allocator, the `BSSimpleList` accessors (with the behaviour the
    /// decompiled bodies have), the scope guard, the critical section, the
    /// logs and the form helpers. Doubles that only record are the ones whose
    /// behaviour the unit does not depend on (`00905330`, the logs); their
    /// calls are read from the call log.
    fn script_engine() -> Engine {
        let mut e = Engine::new();
        for page in PAGES {
            e.map(page, 0x1000);
        }
        e.register(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(OPERATOR_DELETE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(LIST_ITEM_SLOT, |_, a| ret(a[0]));
        e.register(LIST_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(LIST_IS_EMPTY, |e, a| {
            ret((e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32)
        });
        e.register(LIST_POP_FIRST, |e, a| {
            let next = e.mem.u32(a[0] + 4);
            if next == 0 {
                e.mem.set_u32(a[0], 0);
            } else {
                let item = e.mem.u32(next);
                let after = e.mem.u32(next + 4);
                e.mem.set_u32(a[0] + 4, after);
                e.mem.set_u32(a[0], item);
                e.mem.set_u32(next + 4, 0);
            }
            Ret::default()
        });
        e.register(LIST_ADD_HEAD, |e, a| {
            let item = e.mem.u32(a[1]);
            if item == 0 {
                return Ret::default();
            }
            if e.mem.u32(a[0]) == 0 {
                e.mem.set_u32(a[0], item);
            } else {
                let node = e.mem.alloc(8);
                let old = e.mem.u32(a[0]);
                let next = e.mem.u32(a[0] + 4);
                e.mem.set_u32(node, old);
                e.mem.set_u32(node + 4, next);
                e.mem.set_u32(a[0] + 4, node);
                e.mem.set_u32(a[0], item);
            }
            Ret::default()
        });
        e.register(LIST_ADD_TAIL, |e, a| {
            let item = e.mem.u32(a[1]);
            if item == 0 {
                return Ret::default();
            }
            let mut last = a[0];
            while e.mem.u32(last + 4) != 0 {
                last = e.mem.u32(last + 4);
            }
            if e.mem.u32(last) == 0 {
                e.mem.set_u32(last, item);
            } else {
                let node = e.mem.alloc(8);
                e.mem.set_u32(node, item);
                e.mem.set_u32(last + 4, node);
            }
            Ret::default()
        });
        // Removal of the item the slot holds (the node is unlinked; the head
        // node takes the place of its successor), as the lists need in the
        // tests that walk them while removing.
        e.register(LIST_REMOVE, |e, a| {
            let wanted = e.mem.u32(a[1]);
            let mut previous = 0u32;
            let mut node = a[0];
            while node != 0 {
                let next = e.mem.u32(node + 4);
                if e.mem.u32(node) == wanted {
                    if previous != 0 {
                        e.mem.set_u32(previous + 4, next);
                    } else if next == 0 {
                        e.mem.set_u32(node, 0);
                    } else {
                        let following = e.mem.u32(next);
                        let after = e.mem.u32(next + 4);
                        e.mem.set_u32(node, following);
                        e.mem.set_u32(node + 4, after);
                    }
                    break;
                }
                previous = node;
                node = next;
            }
            Ret::default()
        });
        e.register(LIST_CONTAINS, |e, a| {
            let wanted = e.mem.u32(a[1]);
            ret(list_items(e, a[0]).contains(&wanted) as u32)
        });
        e.register(LIST_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
            ret(a[0])
        });
        e.register(LIST_DESTRUCT, |_, _| Ret::default());
        e.register(LIST_DELETE, |_, _| Ret::default());
        e.register(ACTION_LIST_OF, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(LOCAL_LIST_OF, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e.register(SCRIPT_REF_OBJECT_LIST, |_, a| ret(a[0] + 0x44));
        e.register(SCRIPT_VARIABLE_LIST, |_, a| ret(a[0] + 0x4c));
        e.register(SCOPE_GUARD_OPEN, |_, _| Ret::default());
        e.register(SCOPE_GUARD_CLOSE, |_, _| Ret::default());
        e.register(CRITICAL_SECTION_ENTER, |_, _| Ret::default());
        e.register(CRITICAL_SECTION_LEAVE, |_, _| Ret::default());
        e.register(ERROR_LOG, |_, _| Ret::default());
        e.register(SCRIPT_LOG, |_, _| Ret::default());
        e
    }

    /// The items of the list starting at `head` (an empty head node has no
    /// items).
    fn list_items(e: &Engine, head: u32) -> Vec<u32> {
        let mut items = Vec::new();
        let mut node = head;
        while node != 0 {
            let item = e.mem.u32(node);
            if item != 0 {
                items.push(item);
            }
            node = e.mem.u32(node + 4);
        }
        items
    }

    /// A list of the items: a chain of 8-byte nodes (item, next); an empty
    /// slice gives an empty head node.
    fn make_list(e: &mut Engine, items: &[u32]) -> u32 {
        let head = e.mem.alloc(8);
        let mut node = head;
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
            e.mem.set_u32(node, *item);
        }
        head
    }

    /// Fills the list node at `list` (an embedded or static list) with the
    /// items.
    fn fill_list(e: &mut Engine, list: u32, items: &[u32]) {
        let built = make_list(e, items);
        let item = e.mem.u32(built);
        let next = e.mem.u32(built + 4);
        e.mem.set_u32(list, item);
        e.mem.set_u32(list + 4, next);
    }

    fn action(e: &mut Engine, form: u32, flags: u32) -> u32 {
        let block = e.mem.alloc(8);
        e.mem.set_u32(block, form);
        e.mem.set_u32(block + 4, flags);
        block
    }

    fn variable(e: &mut Engine, id: u32, value: f64) -> u32 {
        let block = e.mem.alloc(0x18);
        e.mem.set_u32(block, id);
        e.mem.set_f64(block + 8, value);
        block
    }

    fn locals(e: &mut Engine) -> Ptr<ScriptLocals> {
        e.new_object::<ScriptLocals>()
    }

    fn set_actions(e: &mut Engine, this: Ptr<ScriptLocals>, entries: &[(u32, u32)]) -> Vec<u32> {
        let blocks: Vec<u32> = entries.iter().map(|(f, fl)| action(e, *f, *fl)).collect();
        let list = make_list(e, &blocks);
        e.set(this, ScriptLocals::m_pActionList, Ptr::new(list));
        blocks
    }

    fn set_variables(e: &mut Engine, this: Ptr<ScriptLocals>, entries: &[(u32, f64)]) -> Vec<u32> {
        let blocks: Vec<u32> = entries.iter().map(|(id, v)| variable(e, *id, *v)).collect();
        let list = make_list(e, &blocks);
        e.set(this, ScriptLocals::m_pLocalList, Ptr::new(list));
        blocks
    }

    fn action_list(e: &Engine, this: Ptr<ScriptLocals>) -> Vec<(u32, u32)> {
        let head = e.get(this, ScriptLocals::m_pActionList).addr();
        list_items(e, head)
            .iter()
            .map(|a| (e.mem.u32(*a), e.mem.u32(*a + 4)))
            .collect()
    }

    /// A master script whose referenced-object list holds objects with the
    /// given variable ids.
    fn script_with_references(e: &mut Engine, ids: &[u32]) -> Ptr<Script> {
        let script = e.new_object::<Script>();
        let objects: Vec<u32> = ids
            .iter()
            .map(|id| {
                let object = e.mem.alloc(0x10);
                e.mem.set_u32(object + 0xc, *id);
                object
            })
            .collect();
        fill_list(e, script.addr() + 0x44, &objects);
        script
    }

    /// Registers `f` at a fresh fake code address and returns it.
    fn fake_code(e: &mut Engine, f: impl FnMut(&mut Engine, &[u32]) -> Ret + 'static) -> u32 {
        let address = e.mem.alloc(4);
        e.register_double(address, f);
        address
    }

    /// An object whose virtual slots (byte offsets) call the given fake
    /// code addresses.
    fn fake_object(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let object = e.mem.alloc(0x20);
        let table = e.mem.alloc(0x200);
        for (slot, target) in slots {
            e.mem.set_u32(table + slot, *target);
        }
        e.mem.set_u32(object, table);
        object
    }

    /// A form whose type-name slot (`+0x130`) returns `name`.
    fn named_form(e: &mut Engine, name: u32) -> u32 {
        let target = fake_code(e, move |_, _| ret(name));
        fake_object(e, &[(0x130, target)])
    }

    /// The argument lists of the calls to `addr` in the call log.
    fn calls_to(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The blocks passed to `operator delete`, in order.
    fn deleted(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        calls_to(log, OPERATOR_DELETE)
            .iter()
            .map(|a| a[0])
            .collect()
    }

    // ---- save/load game doubles --------------------------------------------

    /// The save/load game object (0x200 bytes at the global) with: the
    /// "use blocks" byte at +0x100, the file version at +0x101, the header
    /// of the form being loaded at +0x108, of the form being saved at
    /// +0x10C, the form `LookupFormByID` returns at +0x110, and the file name
    /// word at +0x1C4. The numeric id doubles keep the id in the low word of
    /// the double (a test convention: the real encoding is `005acc70`'s).
    fn save_load_game_object(e: &mut Engine) -> u32 {
        let object = e.mem.alloc(0x200);
        e.set_global(SAVE_LOAD_GAME_GLOBAL, object);
        e.register(USE_SAVE_GAME_BLOCKS, |e, a| {
            ret(e.mem.u8(a[0] + 0x100) as u32)
        });
        e.register(SAVE_FILE_VERSION, |e, a| ret(e.mem.u8(a[0] + 0x101) as u32));
        e.register(LOADING_FORM_HEADER, |e, a| ret(e.mem.u32(a[0] + 0x108)));
        e.register(SAVING_FORM_HEADER, |e, a| ret(e.mem.u32(a[0] + 0x10c)));
        e.register(LOOKUP_FORM_BY_ID, |e, _| {
            let save_load: u32 = e.global(SAVE_LOAD_GAME_GLOBAL);
            ret(e.mem.u32(save_load + 0x110))
        });
        e.register(IS_LOADING_STUB, |_, _| ret(0));
        e.register(FLAG_BYTE_ADDRESS, |_, a| ret(a[0] + 4));
        e.register(GET_NUMERIC_ID_FROM_DOUBLE, |e, a| {
            let id = e.mem.f64(a[1]) as u32;
            e.mem.set_u32(a[0], id);
            Ret::default()
        });
        e.register(PUT_NUMERIC_ID_IN_DOUBLE, |e, a| {
            let id = e.mem.u32(a[0]);
            e.mem.set_f64(a[1], id as f64);
            Ret::default()
        });
        object
    }

    /// A byte stream the old save format writes and reads: the position
    /// double returns an address in a buffer, the write and read doubles
    /// copy bytes and advance. Returns the buffer address and the cursor.
    fn install_stream(e: &mut Engine) -> (u32, Rc<Cell<u32>>) {
        let base = e.mem.alloc(0x400);
        let cursor = Rc::new(Cell::new(0u32));
        let c = cursor.clone();
        e.register_double(STREAM_POSITION, move |_, _| ret(base + c.get()));
        for write in [STREAM_WRITE, STREAM_SAVE_NUMERIC_ID] {
            let c = cursor.clone();
            e.register_double(write, move |e, a| {
                let bytes = e.mem.bytes(a[1], a[2]);
                e.mem.write(base + c.get(), &bytes);
                c.set(c.get() + a[2]);
                Ret::default()
            });
        }
        for read in [STREAM_READ, STREAM_LOAD_NUMERIC_ID] {
            let c = cursor.clone();
            e.register_double(read, move |e, a| {
                let bytes = e.mem.bytes(base + c.get(), a[2]);
                e.mem.write(a[1], &bytes);
                c.set(c.get() + a[2]);
                Ret::default()
            });
        }
        (base, cursor)
    }

    // ---- ScriptLocals lists --------------------------------------------------

    #[test]
    fn destructor_body_frees_both_lists_and_the_effect_data() {
        let mut e = script_engine();
        let this = locals(&mut e);
        let actions = set_actions(&mut e, this, &[(0x100, 1)]);
        let variables = set_variables(&mut e, this, &[(1, 2.0)]);
        let effect = e.mem.alloc(8);
        e.set(this, ScriptLocals::m_pScriptEffectData, Ptr::new(effect));
        e.call_log = Some(vec![]);
        e.call(0x005a_8bc0, &args![this]);
        let log = e.call_log.take().unwrap();
        assert_eq!(deleted(&log), vec![actions[0], variables[0], effect]);
        assert!(e.get(this, ScriptLocals::m_pActionList).is_null());
        assert!(e.get(this, ScriptLocals::m_pLocalList).is_null());
        assert_eq!(calls_to(&log, LIST_DELETE).len(), 2);
    }

    #[test]
    fn destructor_body_with_nothing_to_free_does_nothing() {
        let mut e = script_engine();
        let this = locals(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x005a_8bc0, &args![this]);
        let log = e.call_log.take().unwrap();
        assert!(deleted(&log).is_empty());
    }

    #[test]
    fn clear_action_list_leaves_the_variables() {
        let mut e = script_engine();
        let this = locals(&mut e);
        let actions = set_actions(&mut e, this, &[(1, 0), (2, 0)]);
        set_variables(&mut e, this, &[(1, 2.0)]);
        e.call_log = Some(vec![]);
        e.call(0x005a_8c00, &args![this]);
        let log = e.call_log.take().unwrap();
        assert_eq!(deleted(&log), actions);
        assert!(e.get(this, ScriptLocals::m_pActionList).is_null());
        assert!(!e.get(this, ScriptLocals::m_pLocalList).is_null());
    }

    #[test]
    fn clear_variable_list_leaves_the_actions() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_actions(&mut e, this, &[(1, 0)]);
        let variables = set_variables(&mut e, this, &[(1, 2.0), (2, 3.0)]);
        e.call_log = Some(vec![]);
        e.call(0x005a_8c90, &args![this]);
        let log = e.call_log.take().unwrap();
        assert_eq!(deleted(&log), variables);
        assert!(e.get(this, ScriptLocals::m_pLocalList).is_null());
        assert!(!e.get(this, ScriptLocals::m_pActionList).is_null());
    }

    #[test]
    fn action_entry_is_created_once_inside_the_scope_guard() {
        let mut e = script_engine();
        let this = locals(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x005a_8d20, &args![this, 0x1234u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(action_list(&e, this), vec![(0x1234, 0)]);
        // The guard opens first (kind 0x15, line 300) and closes last.
        assert_eq!(log[1].0, SCOPE_GUARD_OPEN);
        assert_eq!(&log[1].1[1..], &[0x15, 1, SOURCE_FILE, 300]);
        assert_eq!(log.last().unwrap().0, SCOPE_GUARD_CLOSE);
        // A second call for the same form adds nothing; another form does.
        e.call(0x005a_8d20, &args![this, 0x1234u32]);
        assert_eq!(action_list(&e, this).len(), 1);
        e.call(0x005a_8d20, &args![this, 0x5678u32]);
        assert_eq!(action_list(&e, this), vec![(0x5678, 0), (0x1234, 0)]);
    }

    #[test]
    fn set_action_flag_ors_into_the_form_and_the_any_form_entries() {
        let mut e = script_engine();
        let this = locals(&mut e);
        // No action list: nothing happens.
        assert!(!e.call(0x005a_8e20, &args![this, 0x10u32, 0x1000u32]).bool());
        set_actions(&mut e, this, &[(0x10, 1), (0, 2), (0x20, 4)]);
        assert!(e.call(0x005a_8e20, &args![this, 0x10u32, 0x1000u32]).bool());
        assert_eq!(
            action_list(&e, this),
            vec![(0x10, 0x1001), (0, 0x1002), (0x20, 4)]
        );
        // A form with no entry still reaches the any-form entry.
        assert!(e.call(0x005a_8e20, &args![this, 0x99u32, 0x8u32]).bool());
        assert_eq!(action_list(&e, this)[1], (0, 0x100a));
    }

    #[test]
    fn set_action_flag_reports_false_when_no_entry_exists() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_actions(&mut e, this, &[(0x20, 4)]);
        assert!(!e.call(0x005a_8e20, &args![this, 0x10u32, 0x1000u32]).bool());
        assert_eq!(action_list(&e, this), vec![(0x20, 4)]);
    }

    #[test]
    fn clear_action_flags_zeroes_every_entry() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_actions(&mut e, this, &[(0x10, 7), (0x20, 0x1000)]);
        e.call(0x005a_8ea0, &args![this]);
        assert_eq!(action_list(&e, this), vec![(0x10, 0), (0x20, 0)]);
        // No list is fine too.
        let bare = locals(&mut e);
        e.call(0x005a_8ea0, &args![bare]);
    }

    #[test]
    fn action_flag_test_creates_a_missing_entry() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_actions(&mut e, this, &[(0x10, 0x1000)]);
        assert!(e.call(0x005a_8ef0, &args![this, 0x10u32, 0x1000u32]).bool());
        assert!(!e.call(0x005a_8ef0, &args![this, 0x10u32, 0x0004u32]).bool());
        // Unknown form: false, and an entry with no flags appears.
        assert!(!e.call(0x005a_8ef0, &args![this, 0x77u32, 0x1000u32]).bool());
        assert_eq!(action_list(&e, this), vec![(0x77, 0), (0x10, 0x1000)]);
    }

    #[test]
    fn action_list_copy_duplicates_every_entry() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_actions(&mut e, this, &[(0x99, 5)]);
        let source = locals(&mut e);
        let originals = set_actions(&mut e, source, &[(0x10, 1), (0x20, 2), (0x30, 3)]);
        e.call(0x005a_8f40, &args![this, source.addr()]);
        assert_eq!(action_list(&e, this), vec![(0x10, 1), (0x20, 2), (0x30, 3)]);
        // The entries are copies, not the source's blocks.
        let head = e.get(this, ScriptLocals::m_pActionList).addr();
        for copy in list_items(&e, head) {
            assert!(!originals.contains(&copy));
        }
    }

    #[test]
    fn action_list_copy_from_nothing_only_clears() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_actions(&mut e, this, &[(0x99, 5)]);
        e.call(0x005a_8f40, &args![this, 0u32]);
        assert!(e.get(this, ScriptLocals::m_pActionList).is_null());
        // A source with no list, or an empty one, gives no list either.
        let source = locals(&mut e);
        e.call(0x005a_8f40, &args![this, source.addr()]);
        assert!(e.get(this, ScriptLocals::m_pActionList).is_null());
        set_actions(&mut e, source, &[]);
        e.call(0x005a_8f40, &args![this, source.addr()]);
        assert!(e.get(this, ScriptLocals::m_pActionList).is_null());
    }

    #[test]
    fn find_action_by_form_or_the_last_empty_entry() {
        let mut e = script_engine();
        let this = locals(&mut e);
        assert_eq!(e.call(0x005a_90b0, &args![this, 1u32, true]).u32(), 0);
        let blocks = set_actions(&mut e, this, &[(0x10, 0), (0, 0), (0x20, 0), (0, 0)]);
        let found = e.call(0x005a_90b0, &args![this, 0x20u32, false]).u32();
        assert_eq!(found, blocks[2]);
        // No match: null, or with the create flag the last entry with no form.
        assert_eq!(e.call(0x005a_90b0, &args![this, 0x30u32, false]).u32(), 0);
        assert_eq!(
            e.call(0x005a_90b0, &args![this, 0x30u32, true]).u32(),
            blocks[3]
        );
        // A form that matches wins even with the create flag.
        assert_eq!(
            e.call(0x005a_90b0, &args![this, 0x10u32, true]).u32(),
            blocks[0]
        );
    }

    // ---- variables -------------------------------------------------------------

    #[test]
    fn get_variable_searches_and_caches_the_last_lookup() {
        let mut e = script_engine();
        let this = locals(&mut e);
        let vars = set_variables(&mut e, this, &[(5, 2.5), (9, 7.0)]);
        let value = e.call(0x005a_9140, &args![this, 9u32, 0u32]).f64();
        assert_eq!(value, 7.0);
        let tls = e.tls();
        assert_eq!(e.mem.u32(tls + TLS_LAST_LOCALS), this.addr());
        assert_eq!(e.mem.u32(tls + TLS_LAST_VARIABLE_ID), 9);
        assert_eq!(e.mem.u32(tls + TLS_LAST_VARIABLE), vars[1]);
        // The cache answers the same lookup even if the list is gone.
        e.mem.set_f64(vars[1] + 8, 8.0);
        e.set(this, ScriptLocals::m_pLocalList, Ptr::NULL);
        assert_eq!(e.call(0x005a_9140, &args![this, 9u32, 0u32]).f64(), 8.0);
    }

    #[test]
    fn get_variable_missing_logs_and_reads_zero() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_variables(&mut e, this, &[(5, 2.5)]);
        e.set_global(SCRIPT_LINE_GLOBAL, 42u32);
        e.call_log = Some(vec![]);
        let value = e.call(0x005a_9140, &args![this, 6u32, 0u32]).f64();
        assert_eq!(value, 0.0);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![FORMAT_VARIABLE_NOT_FOUND, 6, UNKNOWN_SCRIPT_NAME, 42]]
        );
        // A named script is reported by its name.
        let script = named_form(&mut e, 0xabc0);
        e.call_log = Some(vec![]);
        e.call(0x005a_9140, &args![this, 7u32, script]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![FORMAT_VARIABLE_NOT_FOUND, 7, 0xabc0, 42]]
        );
    }

    #[test]
    fn set_variable_stores_the_value() {
        let mut e = script_engine();
        let this = locals(&mut e);
        let vars = set_variables(&mut e, this, &[(5, 2.5), (9, 7.0)]);
        e.call(0x005a_9290, &args![this, 9u32, 3.25f64]);
        assert_eq!(e.mem.f64(vars[1] + 8), 3.25);
        assert_eq!(e.mem.f64(vars[0] + 8), 2.5);
    }

    #[test]
    fn set_variable_missing_in_a_script_names_the_script() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_variables(&mut e, this, &[(5, 2.5)]);
        let master = named_form(&mut e, 0xabc0);
        e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(master));
        e.call_log = Some(vec![]);
        e.call(0x005a_9290, &args![this, 8u32, 1.0f64]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![FORMAT_SET_VARIABLE_IN_SCRIPT, 8, 0xabc0]]
        );
    }

    #[test]
    fn set_variable_missing_without_a_script_logs_by_load_state() {
        let mut e = script_engine();
        let save_load = save_load_game_object(&mut e);
        let this = locals(&mut e);
        // Not loading (the stub answers false): the plain message.
        e.call_log = Some(vec![]);
        e.call(0x005a_9290, &args![this, 8u32, 1.0f64]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![FORMAT_SET_VARIABLE_NO_SCRIPT, 8]]
        );

        // Loading a form that is a reference: the file name, id, name and
        // the form id are reported.
        e.register(IS_LOADING_STUB, |_, _| ret(1));
        let header = e.mem.alloc(0x10);
        e.mem.set_u32(header, 0x0001_0203);
        e.mem.set_u32(save_load + 0x108, header);
        e.mem.set_u32(save_load + 0x110, 0x7000);
        e.mem.set_u32(save_load + 0x1c4, 0xf11e);
        e.register(RT_DYNAMIC_CAST, |_, a| ret(a[0]));
        e.register(FULL_NAME_OF_REFERENCE, |_, _| ret(0x4e01));
        e.register(FULL_NAME_OF_FORM, |_, _| ret(0x4e02));
        e.call_log = Some(vec![]);
        e.call(0x005a_9290, &args![this, 8u32, 1.0f64]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![
                FORMAT_SET_VARIABLE_REF,
                0xf11e,
                8,
                0x4e01,
                0x0001_0203
            ]]
        );

        // The cast fails: the plain form name.
        e.register(RT_DYNAMIC_CAST, |_, _| ret(0));
        e.call_log = Some(vec![]);
        e.call(0x005a_9290, &args![this, 8u32, 1.0f64]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![
                FORMAT_SET_VARIABLE_NAMED,
                0xf11e,
                8,
                0x4e02,
                0x0001_0203
            ]]
        );

        // No name: the message without one.
        e.register(FULL_NAME_OF_FORM, |_, _| ret(0));
        e.call_log = Some(vec![]);
        e.call(0x005a_9290, &args![this, 8u32, 1.0f64]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![FORMAT_SET_VARIABLE_FORM, 0xf11e, 8, 0x0001_0203]]
        );

        // Loading with no form header: the plain message again.
        e.mem.set_u32(save_load + 0x108, 0);
        e.call_log = Some(vec![]);
        e.call(0x005a_9290, &args![this, 8u32, 1.0f64]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![FORMAT_SET_VARIABLE_NO_SCRIPT, 8]]
        );
    }

    #[test]
    fn file_name_is_read_from_the_save_load_object() {
        let mut e = script_engine();
        let object = e.mem.alloc(0x200);
        e.mem.set_u32(object + 0x1c4, 0xfeed);
        assert_eq!(e.call(0x005a_9460, &args![object]).u32(), 0xfeed);
    }

    #[test]
    fn form_reference_variables_are_found_in_the_master_script() {
        let mut e = script_engine();
        let this = locals(&mut e);
        // No master script: no variable is a reference.
        assert!(!e.call(0x005a_9480, &args![this, 3u32]).bool());
        let master = script_with_references(&mut e, &[2, 3]);
        e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(master.addr()));
        assert!(e.call(0x005a_9480, &args![this, 3u32]).bool());
        assert!(e.call(0x005a_9480, &args![this, 2u32]).bool());
        assert!(!e.call(0x005a_9480, &args![this, 4u32]).bool());
        // A master with no referenced objects has none.
        let empty = script_with_references(&mut e, &[]);
        e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(empty.addr()));
        assert!(!e.call(0x005a_9480, &args![this, 3u32]).bool());
    }

    // ---- saving ------------------------------------------------------------------

    #[test]
    fn save_size_counts_the_variables_the_old_format_writes() {
        let mut e = script_engine();
        let save_load = save_load_game_object(&mut e);
        let this = locals(&mut e);
        // A zero value is not saved; a form reference takes 8 bytes, any
        // other 12; the effect-data flag takes 1.
        set_variables(&mut e, this, &[(1, 5.0), (2, 0.0), (3, 6.0)]);
        let master = script_with_references(&mut e, &[3]);
        e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(master.addr()));
        assert_eq!(e.call(0x005a_94f0, &args![this]).u16(), 2 + 12 + 8 + 1);
        // Blocks add 6; effect data adds 8.
        e.mem.set_u8(save_load + 0x100, 1);
        let effect = e.mem.alloc(8);
        e.set(this, ScriptLocals::m_pScriptEffectData, Ptr::new(effect));
        assert_eq!(
            e.call(0x005a_94f0, &args![this]).u16(),
            6 + 2 + 12 + 8 + 1 + 8
        );
    }

    #[test]
    fn save_size_is_logged_when_logging_is_on() {
        let mut e = script_engine();
        let save_load = save_load_game_object(&mut e);
        let this = locals(&mut e);
        set_variables(&mut e, this, &[(1, 5.0)]);
        e.mem.set_u8(SAVE_SIZE_LOG_OBJECT + 4, 1);
        // Without a form being saved: size, line and file.
        e.call_log = Some(vec![]);
        e.call(0x005a_94f0, &args![this]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, ERROR_LOG),
            vec![vec![FORMAT_SAVE_SIZE_PLAIN, 15, 0x27e, SOURCE_FILE]]
        );
        // With one: also its id, type name and flags.
        let header = e.mem.alloc(0x10);
        e.mem.set_u32(header, 0x00aa_bbcc);
        e.mem.set_u32(header + 5, 0x40);
        e.mem.set_u32(save_load + 0x10c, header);
        let form = named_form(&mut e, 0xabc0);
        e.mem.set_u32(save_load + 0x110, form);
        e.call_log = Some(vec![]);
        e.call(0x005a_94f0, &args![this]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, ERROR_LOG),
            vec![vec![
                FORMAT_SAVE_SIZE_FORM,
                15,
                0x00aa_bbcc,
                0xabc0,
                0x40,
                0x27e,
                SOURCE_FILE
            ]]
        );
    }

    #[test]
    fn old_save_writes_variables_count_and_block_size() {
        let mut e = script_engine();
        let save_load = save_load_game_object(&mut e);
        e.mem.set_u8(save_load + 0x100, 1);
        let (base, cursor) = install_stream(&mut e);
        let this = locals(&mut e);
        set_variables(&mut e, this, &[(1, 5.0), (2, 0.0), (3, 6.0)]);
        let master = script_with_references(&mut e, &[3]);
        e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(master.addr()));
        e.call(0x005a_9670, &args![this]);
        // BLOK, block size, count, variable 1 (id, value), variable 3 as a
        // form reference (id with the top bit, the numeric id), no effect data.
        assert_eq!(e.mem.u32(base), BLOCK_MARKER);
        assert_eq!(e.mem.u16(base + 6), 2);
        assert_eq!(e.mem.u32(base + 8), 1);
        assert_eq!(e.mem.f64(base + 12), 5.0);
        assert_eq!(e.mem.u32(base + 20), 3 | 0x8000_0000);
        assert_eq!(e.mem.u32(base + 24), 6);
        assert_eq!(e.mem.u8(base + 28), 0);
        assert_eq!(cursor.get(), 29);
        // The block size counts from its own position to the end.
        assert_eq!(e.mem.u16(base + 4), 29 - 4);
    }

    #[test]
    fn old_save_without_blocks_writes_the_effect_data() {
        let mut e = script_engine();
        save_load_game_object(&mut e);
        let (base, cursor) = install_stream(&mut e);
        let this = locals(&mut e);
        let effect = e.mem.alloc(8);
        e.mem.set_u32(effect, 0x0101_0101);
        e.set(this, ScriptLocals::m_pScriptEffectData, Ptr::new(effect));
        e.call(0x005a_9670, &args![this]);
        assert_eq!(e.mem.u16(base), 0);
        assert_eq!(e.mem.u8(base + 2), 1);
        assert_eq!(e.mem.u32(base + 3), 0x0101_0101);
        assert_eq!(cursor.get(), 11);
    }

    #[test]
    fn old_save_logs_sizes_and_oversized_blocks() {
        let mut e = script_engine();
        let save_load = save_load_game_object(&mut e);
        e.mem.set_u8(save_load + 0x100, 1);
        e.mem.set_u8(SAVE_SIZE_LOG_OBJECT + 4, 1);
        let (_, cursor) = install_stream(&mut e);
        let this = locals(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x005a_9670, &args![this]);
        let log = e.call_log.take().unwrap();
        // marker 4 + block size 2 + count 2 + flag 1.
        assert_eq!(
            calls_to(&log, ERROR_LOG),
            vec![vec![FORMAT_SAVE_PLAIN, 9, 0x2b6, SOURCE_FILE]]
        );
        assert_eq!(cursor.get(), 9);
        // A block larger than a short is reported: the position double
        // jumps far ahead once the content has been written.
        let far = e.mem.alloc(0x400);
        // The fourth position (the end of the block) is far past the start.
        let asked = Rc::new(Cell::new(0u32));
        e.register_double(STREAM_POSITION, move |_, _| {
            asked.set(asked.get() + 1);
            ret(far + if asked.get() >= 4 { 0x2_0000 } else { 0 })
        });
        e.register(STREAM_WRITE, |_, _| Ret::default());
        cursor.set(0);
        e.call_log = Some(vec![]);
        e.mem.set_u8(SAVE_SIZE_LOG_OBJECT + 4, 0);
        e.call(0x005a_9670, &args![this]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![FORMAT_BLOCK_TOO_LARGE, SOURCE_FILE, 0x2b6]]
        );
    }

    #[test]
    fn old_save_and_old_load_round_trip() {
        for blocks in [0u8, 1] {
            let mut e = script_engine();
            let save_load = save_load_game_object(&mut e);
            e.mem.set_u8(save_load + 0x100, blocks);
            e.mem.set_u8(save_load + 0x101, 0x80);
            let (_, cursor) = install_stream(&mut e);
            let this = locals(&mut e);
            set_variables(&mut e, this, &[(1, 5.0), (3, 6.0), (4, -2.0)]);
            let master = script_with_references(&mut e, &[3]);
            e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(master.addr()));
            let effect = e.mem.alloc(8);
            e.mem.set_u32(effect + 4, 0x2222);
            e.set(this, ScriptLocals::m_pScriptEffectData, Ptr::new(effect));
            e.call(0x005a_9670, &args![this]);
            let written = cursor.get();

            // Load into locals that have the variables with other values.
            cursor.set(0);
            let other = locals(&mut e);
            let vars = set_variables(&mut e, other, &[(1, 0.5), (3, 0.5), (4, 0.5)]);
            e.call_log = Some(vec![]);
            e.call(0x005a_9950, &args![other]);
            let log = e.call_log.take().unwrap();
            assert_eq!(cursor.get(), written);
            assert_eq!(e.mem.f64(vars[0] + 8), 5.0);
            assert_eq!(e.mem.f64(vars[1] + 8), 6.0);
            assert_eq!(e.mem.f64(vars[2] + 8), -2.0);
            let loaded = e.get(other, ScriptLocals::m_pScriptEffectData);
            assert!(!loaded.is_null());
            assert_eq!(e.mem.u32(loaded.addr() + 4), 0x2222);
            // Nothing is reported for a correct stream.
            assert!(calls_to(&log, SCRIPT_LOG).is_empty());
        }
    }

    #[test]
    fn old_load_of_an_old_file_reads_id_and_value_plainly() {
        let mut e = script_engine();
        let save_load = save_load_game_object(&mut e);
        e.mem.set_u8(save_load + 0x101, 0x70);
        let (base, cursor) = install_stream(&mut e);
        // One variable: count 1, id 7, value 4.5, no effect data.
        e.mem.set_u16(base, 1);
        e.mem.set_u32(base + 2, 7);
        e.mem.set_f64(base + 6, 4.5);
        e.mem.set_u8(base + 14, 0);
        let this = locals(&mut e);
        let vars = set_variables(&mut e, this, &[(7, 0.0)]);
        e.call(0x005a_9950, &args![this]);
        assert_eq!(e.mem.f64(vars[0] + 8), 4.5);
        assert_eq!(cursor.get(), 15);
    }

    #[test]
    fn old_load_reports_a_bad_block_header_and_size_mismatches() {
        let mut e = script_engine();
        let save_load = save_load_game_object(&mut e);
        e.mem.set_u8(save_load + 0x100, 1);
        e.mem.set_u8(save_load + 0x101, 0x80);
        let (base, cursor) = install_stream(&mut e);
        // A wrong marker; the block claims 3 bytes.
        e.mem.set_u32(base, 0x1111_1111);
        e.mem.set_u16(base + 4, 3);
        e.mem.set_u16(base + 6, 0);
        e.mem.set_u8(base + 8, 0);
        let this = locals(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x005a_9950, &args![this]);
        let log = e.call_log.take().unwrap();
        let messages = calls_to(&log, SCRIPT_LOG);
        assert_eq!(
            messages[0],
            vec![FORMAT_BLOCK_HEADER_PLAIN, SOURCE_FILE, 0x2bc, 0x80]
        );
        // The block starts after the marker (4), claims 3 bytes: expected
        // end 7, the stream is at 9: an overrun of 2.
        assert_eq!(
            messages[1],
            vec![FORMAT_OVERRUN_PLAIN, 2, SOURCE_FILE, 0x2eb, 0x80]
        );
        assert_eq!(messages.len(), 2);

        // With a form being loaded, its header is reported, and a block
        // that claims too much is an underrun.
        let header = e.mem.alloc(0x10);
        e.mem.set_u32(header, 0x0000_0abc);
        e.mem.set_u32(header + 5, 0x22);
        e.mem.set_u8(header + 9, 0x33);
        e.mem.set_u32(save_load + 0x108, header);
        let form = named_form(&mut e, 0xabc0);
        e.mem.set_u32(save_load + 0x110, form);
        e.mem.set_u32(base, BLOCK_MARKER);
        e.mem.set_u16(base + 4, 100);
        cursor.set(0);
        e.call_log = Some(vec![]);
        let again = locals(&mut e);
        e.call(0x005a_9950, &args![again]);
        let log = e.call_log.take().unwrap();
        // Read to 9 bytes; the block ends at 4 + 100: an underrun of 95.
        assert_eq!(
            calls_to(&log, SCRIPT_LOG),
            vec![vec![
                FORMAT_UNDERRUN_FORM,
                95,
                SOURCE_FILE,
                0x2eb,
                0xabc,
                0xabc0,
                0x33,
                0x22
            ]]
        );
    }

    #[test]
    fn reset_rebuilds_the_variables_and_frees_the_effect_data() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_actions(&mut e, this, &[(1, 0x1000)]);
        set_variables(&mut e, this, &[(1, 2.0)]);
        let effect = e.mem.alloc(8);
        e.set(this, ScriptLocals::m_pScriptEffectData, Ptr::new(effect));
        let master = e.mem.alloc(0x60);
        e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(master));
        let rebuilt = e.mem.alloc(8);
        e.register_double(SCRIPT_BUILD_LOCALS, move |_, a| {
            assert_eq!(a[0], master);
            ret(rebuilt)
        });
        e.call_log = Some(vec![]);
        e.call(0x005a_9d00, &args![this]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(this, ScriptLocals::m_pLocalList).addr(), rebuilt);
        assert_eq!(action_list(&e, this), vec![(1, 0)]);
        assert!(e.get(this, ScriptLocals::m_pScriptEffectData).is_null());
        assert!(deleted(&log).contains(&effect));
        // Without a master script the list is only emptied.
        let bare = locals(&mut e);
        set_variables(&mut e, bare, &[(1, 2.0)]);
        e.call(0x005a_9d00, &args![bare]);
        assert!(e.get(bare, ScriptLocals::m_pLocalList).is_null());
    }

    #[test]
    fn the_second_reset_entry_does_the_same() {
        let mut e = script_engine();
        let this = locals(&mut e);
        set_actions(&mut e, this, &[(1, 0x1000)]);
        set_variables(&mut e, this, &[(1, 2.0)]);
        let effect = e.mem.alloc(8);
        e.set(this, ScriptLocals::m_pScriptEffectData, Ptr::new(effect));
        e.call(0x005a_a090, &args![this, 0xdead_beefu32]);
        assert_eq!(action_list(&e, this), vec![(1, 0)]);
        assert!(e.get(this, ScriptLocals::m_pLocalList).is_null());
        assert!(e.get(this, ScriptLocals::m_pScriptEffectData).is_null());
    }

    #[test]
    fn last_lookup_cache_is_cleared() {
        let mut e = script_engine();
        let tls = e.tls();
        e.mem.set_u32(tls + TLS_LAST_LOCALS, 0x1234);
        e.mem.set_u32(tls + TLS_LAST_VARIABLE_ID, 5);
        e.mem.set_u32(tls + TLS_LAST_VARIABLE, 0x5678);
        e.call(0x005a_9d60, &args![]);
        assert_eq!(e.mem.u32(tls + TLS_LAST_LOCALS), 0);
        assert_eq!(e.mem.u32(tls + TLS_LAST_VARIABLE_ID), 0xffff_ffff);
        assert_eq!(e.mem.u32(tls + TLS_LAST_VARIABLE), 0);
    }

    /// Doubles for the `BGSSaveGameBuffer` methods: they record their
    /// arguments (through the call log) and `StartVariableSizedValue` gives
    /// `0x7000`.
    fn install_save_buffer(e: &mut Engine) -> Ptr {
        e.register(BUFFER_START_SIZED_VALUE, |_, _| ret(0x7000));
        for addr in [
            BUFFER_SAVE_SIZED_VALUE,
            BUFFER_SAVE_FORM_ID_OV2,
            BUFFER_SAVE_FORM_ID,
            BUFFER_SAVE_BYTES,
        ] {
            e.register(addr, |_, _| Ret::default());
        }
        e.register(GET_NUMERIC_ID_FROM_DOUBLE, |e, a| {
            let id = e.mem.f64(a[1]) as u32;
            e.mem.set_u32(a[0], id);
            Ret::default()
        });
        Ptr::new(e.mem.alloc(0x20))
    }

    #[test]
    fn save_game_writes_count_variables_and_flags() {
        let mut e = script_engine();
        let buffer = install_save_buffer(&mut e);
        let this = locals(&mut e);
        let vars = set_variables(&mut e, this, &[(1, 5.0), (2, 0.0), (3, 6.0)]);
        let master = script_with_references(&mut e, &[3]);
        e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(master.addr()));
        let effect = e.mem.alloc(8);
        e.set(this, ScriptLocals::m_pScriptEffectData, Ptr::new(effect));
        set_actions(&mut e, this, &[(0, 0x1000)]);
        e.call_log = Some(vec![]);
        e.call(0x005a_9db0, &args![this, buffer]);
        let log = e.call_log.take().unwrap();
        let buffer_calls: Vec<(u32, Vec<u32>)> = log
            .iter()
            .filter(|(a, _)| {
                [
                    BUFFER_START_SIZED_VALUE,
                    BUFFER_SAVE_SIZED_VALUE,
                    BUFFER_SAVE_FORM_ID,
                    BUFFER_SAVE_BYTES,
                ]
                .contains(a)
            })
            .cloned()
            .collect();
        let b = buffer.addr();
        // Start; variable 1 (id and value); variable 3 as a form reference
        // (id with the top bit, then the form id 6); the count 2 at the
        // position; the effect data flag and data; the action flag.
        assert_eq!(buffer_calls[0], (BUFFER_START_SIZED_VALUE, vec![b]));
        assert_eq!(buffer_calls[1], (BUFFER_SAVE_BYTES, vec![b, vars[0], 4, 0]));
        assert_eq!(
            buffer_calls[2],
            (BUFFER_SAVE_BYTES, vec![b, vars[0] + 8, 8, 0])
        );
        assert_eq!(buffer_calls[3].0, BUFFER_SAVE_BYTES);
        assert_eq!(buffer_calls[3].1[2..], [4, 0]);
        assert_eq!(buffer_calls[4], (BUFFER_SAVE_FORM_ID, vec![b, 6, 0]));
        assert_eq!(
            buffer_calls[5],
            (BUFFER_SAVE_SIZED_VALUE, vec![b, 2, 0x7000])
        );
        assert_eq!(buffer_calls[6].1[2..], [1, 0]);
        assert_eq!(buffer_calls[7], (BUFFER_SAVE_BYTES, vec![b, effect, 8, 0]));
        assert_eq!(buffer_calls[8].1[2..], [1, 0]);
        assert_eq!(buffer_calls.len(), 9);
    }

    /// Doubles for the `BGSLoadGameBuffer`: the sized value count is
    /// `count`, bytes come from `data` in order, the version slot gives
    /// `version` and the second slot `0x5ec0`.
    fn install_load_buffer(e: &mut Engine, count: u32, data: Vec<u8>, version: u32) -> Ptr {
        e.register_double(BUFFER_LOAD_SIZED_VALUE, move |_, _| ret(count));
        let position = Rc::new(Cell::new(0usize));
        e.register_double(BUFFER_LOAD_BYTES, move |e, a| {
            let start = position.get();
            let size = a[2] as usize;
            e.mem.write(a[1], &data[start..start + size]);
            position.set(start + size);
            Ret::default()
        });
        e.register(BUFFER_LOAD_FORM_ID_OV2, |e, a| {
            e.mem.set_u32(a[1], 0xf0);
            Ret::default()
        });
        let first = fake_code(e, move |_, _| ret(version));
        let second = fake_code(e, |_, _| ret(0x5ec0));
        Ptr::new(fake_object(e, &[(0, first), (8, second)]))
    }

    #[test]
    fn load_game_reads_variables_effect_data_and_the_action_flag() {
        let mut e = script_engine();
        e.register(PUT_NUMERIC_ID_IN_DOUBLE, |e, a| {
            let id = e.mem.u32(a[0]);
            e.mem.set_f64(a[1], id as f64);
            Ret::default()
        });
        let this = locals(&mut e);
        let vars = set_variables(&mut e, this, &[(1, 0.0), (3, 0.0)]);
        let actions = set_actions(&mut e, this, &[(0, 0)]);
        // Two variables: id 1 with a plain value 2.5, id 3 with the top bit
        // (a form reference); an effect data flag with 8 bytes; the action
        // flag.
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&2.5f64.to_le_bytes());
        data.extend_from_slice(&(3u32 | 0x8000_0000).to_le_bytes());
        data.push(1);
        data.extend_from_slice(&[9, 9, 9, 9, 8, 8, 8, 8]);
        data.push(1);
        let buffer = install_load_buffer(&mut e, 2, data, 0x20);
        e.call(0x005a_9f20, &args![this, buffer]);
        assert_eq!(e.mem.f64(vars[0] + 8), 2.5);
        // The form id the buffer gave (0xf0) is put into the double.
        assert_eq!(e.mem.f64(vars[1] + 8), 0xf0 as f64);
        let effect = e.get(this, ScriptLocals::m_pScriptEffectData);
        assert!(!effect.is_null());
        assert_eq!(e.mem.u32(effect.addr()), 0x0909_0909);
        // An action list exists: flags are cleared, then the any-form entry
        // gets the saved flag.
        assert_eq!(e.mem.u32(actions[0] + 4), 0x1000);
    }

    #[test]
    fn load_game_builds_the_action_list_through_the_run_manager() {
        let mut e = script_engine();
        let this = locals(&mut e);
        let master = e.mem.alloc(0x60);
        e.set(this, ScriptLocals::m_pMasterScript, Ptr::new(master));
        // An old buffer version has no action flag byte.
        let buffer = install_load_buffer(&mut e, 0, vec![0], 0x14);
        e.register(SCRIPT_RUN_MANAGER_INSTANCE, |_, _| ret(0x6a6a));
        e.register(SCRIPT_RUN_MANAGER_INIT_ACTION_LIST, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x005a_9f20, &args![this, buffer]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SCRIPT_RUN_MANAGER_INIT_ACTION_LIST),
            vec![vec![0x6a6a, master, 0x5ec0, this.addr()]]
        );
        // Only the effect flag was read: one byte.
        assert_eq!(calls_to(&log, BUFFER_LOAD_BYTES).len(), 1);
        assert!(e.get(this, ScriptLocals::m_pScriptEffectData).is_null());
    }

    // ---- Script ------------------------------------------------------------------

    fn script_blank_engine() -> Engine {
        let mut e = script_engine();
        e.register(TES_FORM_CONSTRUCT, |_, _| Ret::default());
        e.register(TES_FORM_DESTRUCT, |_, _| Ret::default());
        e.register(MEMSET, |e, a| {
            for i in 0..a[2] {
                e.mem.set_u8(a[0] + i, a[1] as u8);
            }
            ret(a[0])
        });
        e.register(SET_FORM_TYPE, |e, a| {
            e.mem.set_u8(a[0] + 4, a[1] as u8);
            Ret::default()
        });
        e
    }

    #[test]
    fn script_constructor_sets_the_vtable_and_clears_the_fields() {
        let mut e = script_blank_engine();
        let script = e.new_object::<Script>();
        // Fill the fields with junk the constructor must reset.
        for off in (0x18..0x54).step_by(4) {
            e.mem.set_u32(script.addr() + off, 0xdddd_dddd);
        }
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_SYSTEM_OUTPUT, 1);
        e.call_log = Some(vec![]);
        let result = e.call(0x005a_a0f0, &args![script]).ptr::<Script>();
        let log = e.call_log.take().unwrap();
        assert_eq!(result, script);
        assert_eq!(e.mem.u32(script.addr()), SCRIPT_VTABLE);
        // The form is constructed first, then the two embedded lists.
        assert_eq!(log[1].0, TES_FORM_CONSTRUCT);
        assert_eq!(
            calls_to(&log, LIST_CONSTRUCT),
            vec![vec![script.addr() + 0x44], vec![script.addr() + 0x4c]]
        );
        assert_eq!(e.mem.bytes(script.addr() + 0x18, 0x14), vec![0; 0x14]);
        assert_eq!(e.get(script, Script::m_text), Ptr::NULL);
        assert_eq!(e.get(script, Script::m_data), Ptr::NULL);
        assert_eq!(e.get(script, Script::pOwnerQuest), Ptr::NULL);
        assert_eq!(e.get(script, Script::fProfilerTimer), 0.0);
        assert_eq!(e.get(script, Script::fQuestScriptDelay), 0.0);
        assert_eq!(e.get(script, Script::fQuestScriptGetSecondsBuffer), 0.0);
        assert_eq!(e.mem.u8(tls + TLS_SYSTEM_OUTPUT), 0);
        assert_eq!(e.mem.u8(script.addr() + 4), 0x11);
    }

    #[test]
    fn script_field_initializer_zeroes_the_header_and_timers() {
        let mut e = script_blank_engine();
        let script = e.new_object::<Script>();
        for off in (0x18..0x54).step_by(4) {
            e.mem.set_u32(script.addr() + off, 0x4040_4040);
        }
        e.call_log = Some(vec![]);
        e.call(0x005a_a220, &args![script]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, MEMSET),
            vec![vec![script.addr() + 0x18, 0, 0x14]]
        );
        assert_eq!(
            calls_to(&log, SET_FORM_TYPE),
            vec![vec![script.addr(), 0x11]]
        );
        assert_eq!(e.mem.u32(script.addr() + 0x2c), 0);
        assert_eq!(e.mem.u32(script.addr() + 0x40), 0);
        assert_eq!(e.mem.u32(script.addr() + 0x34), 0);
        // The lists are left alone.
        assert_eq!(e.mem.u32(script.addr() + 0x44), 0x4040_4040);
    }

    #[test]
    fn script_destructor_body_releases_everything() {
        let mut e = script_blank_engine();
        let script = e.new_object::<Script>();
        let text = e.mem.alloc(8);
        let data = e.mem.alloc(8);
        e.set(script, Script::m_text, Ptr::new(text));
        e.set(script, Script::m_data, Ptr::new(data));
        e.call_log = Some(vec![]);
        e.call(0x005a_a1a0, &args![script]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(script.addr()), SCRIPT_VTABLE);
        // Data first, then the text.
        assert_eq!(deleted(&log), vec![data, text]);
        // The variable list is destroyed first, then the object list, then
        // the form base.
        assert_eq!(
            calls_to(&log, LIST_DESTRUCT),
            vec![vec![script.addr() + 0x4c], vec![script.addr() + 0x44]]
        );
        assert_eq!(log.last().unwrap().0, TES_FORM_DESTRUCT);
    }

    #[test]
    fn script_deleting_destructor_frees_with_the_flag() {
        let mut e = script_blank_engine();
        let script = e.new_object::<Script>();
        e.call_log = Some(vec![]);
        let back = e.call(0x005a_a170, &args![script, 0u32]).ptr::<Script>();
        let log = e.call_log.take().unwrap();
        assert_eq!(back, script);
        assert!(!deleted(&log).contains(&script.addr()));
        e.call_log = Some(vec![]);
        e.call(0x005a_a170, &args![script, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(deleted(&log).last(), Some(&script.addr()));
    }

    #[test]
    fn script_releases_data_and_text_then_the_lists() {
        let mut e = script_blank_engine();
        let script = e.new_object::<Script>();
        let text = e.mem.alloc(8);
        let data = e.mem.alloc(8);
        e.set(script, Script::m_text, Ptr::new(text));
        e.set(script, Script::m_data, Ptr::new(data));
        let tls = e.tls();
        e.mem
            .set_u32(tls + TLS_LAST_REF_SEARCH_SCRIPT, script.addr());
        e.call(0x005a_a2a0, &args![script]);
        assert_eq!(e.mem.u32(tls + TLS_LAST_REF_SEARCH_SCRIPT), 0);
        // The text and data blocks were freed (a second free would panic).
        assert_eq!(e.mem.block_size(text), None);
        assert_eq!(e.mem.block_size(data), None);
    }

    #[test]
    fn script_variable_list_is_emptied_item_by_item() {
        let mut e = script_blank_engine();
        let script = e.new_object::<Script>();
        let items: Vec<u32> = (0..3).map(|_| e.mem.alloc(0x20)).collect();
        fill_list(&mut e, script.addr() + 0x4c, &items);
        e.register(SCRIPT_VARIABLE_DELETE, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x005a_a2e0, &args![script]);
        let log = e.call_log.take().unwrap();
        let removed = calls_to(&log, SCRIPT_VARIABLE_DELETE);
        assert_eq!(
            removed,
            items.iter().map(|i| vec![*i, 1]).collect::<Vec<_>>()
        );
        assert!(list_items(&e, script.addr() + 0x4c).is_empty());
    }

    #[test]
    fn script_object_list_is_emptied_and_the_search_cache_forgotten() {
        let mut e = script_blank_engine();
        let script = e.new_object::<Script>();
        let items: Vec<u32> = (0..2).map(|_| e.mem.alloc(0x10)).collect();
        fill_list(&mut e, script.addr() + 0x44, &items);
        e.register(SCRIPT_REFERENCED_OBJECT_DESTRUCT, |_, _| Ret::default());
        let tls = e.tls();
        e.mem
            .set_u32(tls + TLS_LAST_REF_SEARCH_SCRIPT, script.addr());
        e.call_log = Some(vec![]);
        e.call(0x005a_a350, &args![script]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(tls + TLS_LAST_REF_SEARCH_SCRIPT), 0);
        assert_eq!(deleted(&log), items);
        assert!(list_items(&e, script.addr() + 0x44).is_empty());
        // Another script in the cache is left alone.
        e.mem.set_u32(tls + TLS_LAST_REF_SEARCH_SCRIPT, 0x1234);
        e.call(0x005a_a350, &args![script]);
        assert_eq!(e.mem.u32(tls + TLS_LAST_REF_SEARCH_SCRIPT), 0x1234);
    }

    #[test]
    fn referenced_object_destructor_frees_only_with_the_flag() {
        let mut e = script_engine();
        e.register(SCRIPT_REFERENCED_OBJECT_DESTRUCT, |_, _| Ret::default());
        let object = e.mem.alloc(0x10);
        e.call_log = Some(vec![]);
        let back = e.call(0x005a_a3f0, &args![object, 0u32]).u32();
        let log = e.call_log.take().unwrap();
        assert_eq!(back, object);
        assert!(deleted(&log).is_empty());
        e.call_log = Some(vec![]);
        e.call(0x005a_a3f0, &args![object, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(deleted(&log), vec![object]);
    }

    fn copy_script_engine(text: u32, quest_flag: u32) -> Engine {
        let mut e = script_blank_engine();
        e.register(SCRIPT_HEADER_OF, |_, a| ret(a[0] + 0x18));
        e.register(MEMCPY, |e, a| {
            let bytes = e.mem.bytes(a[1], a[2]);
            e.mem.write(a[0], &bytes);
            ret(a[0])
        });
        e.register(SCRIPT_COMPILE_DATA_OF, |_, _| ret(0xda7a));
        e.register(SCRIPT_SET_COMPILE_DATA, |_, _| Ret::default());
        e.register(SCRIPT_COPY_REF_OBJECTS, |_, _| Ret::default());
        e.register(SCRIPT_COPY_VARIABLES, |_, _| Ret::default());
        e.register_double(SCRIPT_TEXT_OF, move |_, _| ret(text));
        e.register(SCRIPT_SET_TEXT, |_, _| Ret::default());
        e.register_double(FORM_FLAG_BIT_3, move |_, _| ret(quest_flag));
        e.register(SCRIPT_SET_QUEST_SCRIPT_FLAG, |_, _| Ret::default());
        e
    }

    #[test]
    fn copy_result_script_copies_header_data_lists_and_text() {
        let mut e = copy_script_engine(0x7e47, 1);
        let this = e.new_object::<Script>();
        let other = e.new_object::<Script>();
        // The other script's header (+0x18) has a data size at +8.
        for i in 0..5u32 {
            e.mem.set_u32(other.addr() + 0x18 + 4 * i, 0x10 + i);
        }
        e.call_log = Some(vec![]);
        e.call(0x005a_a420, &args![this, other]);
        let log = e.call_log.take().unwrap();
        let (t, o) = (this.addr(), other.addr());
        // The header is copied except the variable count.
        assert_eq!(e.mem.u32(t + 0x18), 0);
        assert_eq!(e.mem.u32(t + 0x1c), 0x11);
        assert_eq!(
            calls_to(&log, SCRIPT_SET_COMPILE_DATA),
            vec![vec![t, 0x12, 0xda7a]]
        );
        assert_eq!(
            calls_to(&log, SCRIPT_COPY_REF_OBJECTS),
            vec![vec![o + 0x44, t + 0x44, t]]
        );
        assert_eq!(
            calls_to(&log, SCRIPT_COPY_VARIABLES),
            vec![vec![o + 0x4c, t + 0x4c, t]]
        );
        assert_eq!(calls_to(&log, SCRIPT_SET_TEXT), vec![vec![t, 0x7e47]]);
        assert_eq!(
            calls_to(&log, SCRIPT_SET_QUEST_SCRIPT_FLAG),
            vec![vec![t, 1]]
        );
    }

    #[test]
    fn copy_result_script_without_text_sets_null_text_and_ignores_null_source() {
        let mut e = copy_script_engine(0, 0);
        let this = e.new_object::<Script>();
        let other = e.new_object::<Script>();
        e.call_log = Some(vec![]);
        e.call(0x005a_a420, &args![this, other]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, SCRIPT_SET_TEXT), vec![vec![this.addr(), 0]]);
        assert_eq!(
            calls_to(&log, SCRIPT_SET_QUEST_SCRIPT_FLAG),
            vec![vec![this.addr(), 0]]
        );
        // A null source does nothing at all (only the call itself is logged).
        e.call_log = Some(vec![]);
        e.call(0x005a_a420, &args![this, 0u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    // ---- pending references ------------------------------------------------------

    #[test]
    fn pending_checks_look_in_their_own_list_under_the_lock() {
        let mut e = script_engine();
        fill_list(&mut e, REFERENCES_TO_ENABLE, &[0x11]);
        fill_list(&mut e, REFERENCES_TO_DISABLE, &[0x22]);
        fill_list(&mut e, REFERENCES_TO_FADE_DISABLE, &[0x33]);
        for (addr, yes, no) in [
            (0x005a_a680u32, 0x11u32, 0x22u32),
            (0x005a_a630, 0x22, 0x33),
            (0x005a_a6d0, 0x33, 0x11),
        ] {
            e.call_log = Some(vec![]);
            assert!(e.call(addr, &args![yes]).bool());
            let log = e.call_log.take().unwrap();
            assert_eq!(log[1].0, CRITICAL_SECTION_ENTER);
            assert_eq!(&log[1].1, &[SCRIPT_REF_LIST_CRITICAL_SECTION, 0]);
            assert_eq!(log.last().unwrap().0, CRITICAL_SECTION_LEAVE);
            assert!(!e.call(addr, &args![no]).bool());
            // A null reference is not looked up.
            e.call_log = Some(vec![]);
            assert!(!e.call(addr, &args![0u32]).bool());
            let log = e.call_log.take().unwrap();
            assert!(calls_to(&log, LIST_CONTAINS).is_empty());
        }
    }

    #[test]
    fn add_pending_disabled_moves_the_reference_to_the_disable_list() {
        let mut e = script_engine();
        e.call_log = Some(vec![]);
        e.call(0x005a_a500, &args![0x4444u32, 0u32]);
        let log = e.call_log.take().unwrap();
        let removed = calls_to(&log, LIST_REMOVE);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0][0], REFERENCES_TO_ENABLE);
        let added = calls_to(&log, LIST_ADD_HEAD);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0][0], REFERENCES_TO_DISABLE);
        assert_eq!(list_items(&e, REFERENCES_TO_DISABLE), vec![0x4444]);
        // Already pending: not added again.
        e.call_log = Some(vec![]);
        e.call(0x005a_a500, &args![0x4444u32, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
    }

    #[test]
    fn add_pending_disabled_with_fade_uses_the_fade_list() {
        let mut e = script_engine();
        e.call(0x005a_a500, &args![0x5555u32, 1u32]);
        assert_eq!(list_items(&e, REFERENCES_TO_FADE_DISABLE), vec![0x5555]);
        assert!(list_items(&e, REFERENCES_TO_DISABLE).is_empty());
        // A null reference only takes the lock: the call, enter, leave.
        e.call_log = Some(vec![]);
        e.call(0x005a_a500, &args![0u32, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, LIST_REMOVE).is_empty());
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn add_pending_enabled_moves_the_reference_to_the_enable_list() {
        let mut e = script_engine();
        e.call_log = Some(vec![]);
        e.call(0x005a_a580, &args![0x6666u32]);
        let log = e.call_log.take().unwrap();
        let removed = calls_to(&log, LIST_REMOVE);
        assert_eq!(removed[0][0], REFERENCES_TO_DISABLE);
        assert_eq!(list_items(&e, REFERENCES_TO_ENABLE), vec![0x6666]);
        // Not added twice.
        e.call_log = Some(vec![]);
        e.call(0x005a_a580, &args![0x6666u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, LIST_ADD_HEAD).is_empty());
        // Null: lock only.
        e.call_log = Some(vec![]);
        e.call(0x005a_a580, &args![0u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 3);
    }

    #[test]
    fn remove_delayed_reference_leaves_all_four_lists() {
        let mut e = script_engine();
        e.call_log = Some(vec![]);
        e.call(0x005a_a5d0, &args![0x7777u32]);
        let log = e.call_log.take().unwrap();
        let lists: Vec<u32> = calls_to(&log, LIST_REMOVE).iter().map(|a| a[0]).collect();
        assert_eq!(
            lists,
            vec![
                REFERENCES_TO_ENABLE,
                REFERENCES_TO_DISABLE,
                REFERENCES_TO_FADE_DISABLE,
                REFERENCES_TO_DELETE
            ]
        );
        // Null does nothing, not even lock.
        e.call_log = Some(vec![]);
        e.call(0x005a_a5d0, &args![0u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    #[test]
    fn delayed_actions_enable_disable_and_delete() {
        let mut e = script_engine();
        let order = Rc::new(RefCell::new(Vec::new()));
        let seen = order.clone();
        e.register_double(REFERENCE_ENABLE, move |_, a| {
            seen.borrow_mut().push(("enable", a[0]));
            Ret::default()
        });
        let seen = order.clone();
        e.register_double(REFERENCE_DISABLE, move |_, a| {
            seen.borrow_mut().push(("disable", a[0]));
            Ret::default()
        });
        let seen = order.clone();
        let destroy = fake_code(&mut e, move |_, a| {
            seen.borrow_mut().push(("delete", a[0]));
            assert_eq!(a[1], 1);
            Ret::default()
        });
        let to_delete = fake_object(&mut e, &[(0x10, destroy)]);
        fill_list(&mut e, REFERENCES_TO_ENABLE, &[0x101, 0x102]);
        fill_list(&mut e, REFERENCES_TO_DISABLE, &[0x201]);
        fill_list(&mut e, REFERENCES_TO_DELETE, &[to_delete]);
        e.call(0x005a_a720, &args![]);
        assert_eq!(
            *order.borrow(),
            vec![
                ("enable", 0x101),
                ("enable", 0x102),
                ("disable", 0x201),
                ("delete", to_delete)
            ]
        );
        for list in [
            REFERENCES_TO_ENABLE,
            REFERENCES_TO_DISABLE,
            REFERENCES_TO_DELETE,
        ] {
            assert!(list_items(&e, list).is_empty());
        }
    }

    #[test]
    fn delayed_actions_keep_fading_references_that_are_still_visible() {
        let mut e = script_engine();
        let disabled = Rc::new(RefCell::new(Vec::new()));
        let seen = disabled.clone();
        e.register_double(REFERENCE_DISABLE, move |_, a| {
            seen.borrow_mut().push(a[0]);
            Ret::default()
        });
        // Reference A: its +0x1D0 object has no target. B: the target is
        // accepted. C: no object at all.
        let no_target = fake_code(&mut e, |_, _| ret(0));
        let object_without_target = fake_object(&mut e, &[(0x10, no_target)]);
        let give_object = fake_code(&mut e, move |_, _| ret(object_without_target));
        let a = fake_object(&mut e, &[(0x1d0, give_object)]);
        let visible_target = e.mem.alloc(0x100);
        let give_target = fake_code(&mut e, move |_, _| ret(visible_target));
        let object_with_target = fake_object(&mut e, &[(0x10, give_target)]);
        let give_object_b = fake_code(&mut e, move |_, _| ret(object_with_target));
        let b = fake_object(&mut e, &[(0x1d0, give_object_b)]);
        let give_nothing = fake_code(&mut e, |_, _| ret(0));
        let c = fake_object(&mut e, &[(0x1d0, give_nothing)]);
        // The target is accepted: flag 0x8000 set.
        e.register(FLAGS_TEST, |_, _| ret(1));
        fill_list(&mut e, REFERENCES_TO_FADE_DISABLE, &[a, b, c]);
        e.call(0x005a_a720, &args![]);
        assert_eq!(*disabled.borrow(), vec![a, c]);
    }

    #[test]
    fn fade_check_follows_flag_fade_byte_and_the_float() {
        let mut e = script_engine();
        let object = e.mem.alloc(0xc0);
        // Flag 0x8000 set: accepted whatever the rest.
        e.register(FLAGS_TEST, |_, _| ret(1));
        assert!(e.call(0x005a_a8c0, &args![object]).bool());
        // Flag clear and the fade byte clear: accepted.
        e.register(FLAGS_TEST, |_, _| ret(0));
        assert!(e.call(0x005a_a8c0, &args![object]).bool());
        // Flag clear, fade byte set: accepted only if the float is positive.
        e.set_global(FADE_ENABLED_BYTE, 1u8);
        e.mem.set_f32(object + 0xb8, 0.5);
        assert!(e.call(0x005a_a8c0, &args![object]).bool());
        e.mem.set_f32(object + 0xb8, 0.0);
        assert!(!e.call(0x005a_a8c0, &args![object]).bool());
        e.mem.set_f32(object + 0xb8, -1.0);
        assert!(!e.call(0x005a_a8c0, &args![object]).bool());
        e.mem.set_f32(object + 0xb8, f32::NAN);
        assert!(!e.call(0x005a_a8c0, &args![object]).bool());
    }

    #[test]
    fn flag_test_asks_for_bit_0x8000() {
        let mut e = script_engine();
        e.register(FLAGS_TEST, |_, a| ret((a[1] == 0x8000) as u32));
        let object = e.mem.alloc(0x40);
        assert!(e.call(0x005a_a910, &args![object]).bool());
        e.register(FLAGS_TEST, |_, _| ret(0));
        assert!(!e.call(0x005a_a910, &args![object]).bool());
    }

    #[test]
    fn pending_references_are_saved_list_by_list() {
        let mut e = script_engine();
        let buffer = install_save_buffer(&mut e);
        fill_list(&mut e, REFERENCES_TO_ENABLE, &[0x11, 0x12]);
        fill_list(&mut e, REFERENCES_TO_DELETE, &[0x31]);
        fill_list(&mut e, REFERENCES_TO_FADE_DISABLE, &[0x41]);
        e.call_log = Some(vec![]);
        e.call(0x005a_a930, &args![buffer]);
        let log = e.call_log.take().unwrap();
        let b = buffer.addr();
        let saved: Vec<(u32, Vec<u32>)> = log
            .iter()
            .filter(|(a, _)| {
                [
                    BUFFER_START_SIZED_VALUE,
                    BUFFER_SAVE_FORM_ID_OV2,
                    BUFFER_SAVE_SIZED_VALUE,
                ]
                .contains(a)
            })
            .cloned()
            .collect();
        assert_eq!(
            saved,
            vec![
                (BUFFER_START_SIZED_VALUE, vec![b]),
                (BUFFER_SAVE_FORM_ID_OV2, vec![b, 0x11, 0]),
                (BUFFER_SAVE_FORM_ID_OV2, vec![b, 0x12, 0]),
                (BUFFER_SAVE_SIZED_VALUE, vec![b, 2, 0x7000]),
                // The disable list is empty: a count of 0.
                (BUFFER_START_SIZED_VALUE, vec![b]),
                (BUFFER_SAVE_SIZED_VALUE, vec![b, 0, 0x7000]),
                (BUFFER_START_SIZED_VALUE, vec![b]),
                (BUFFER_SAVE_FORM_ID_OV2, vec![b, 0x31, 0]),
                (BUFFER_SAVE_SIZED_VALUE, vec![b, 1, 0x7000]),
                (BUFFER_START_SIZED_VALUE, vec![b]),
                (BUFFER_SAVE_FORM_ID_OV2, vec![b, 0x41, 0]),
                (BUFFER_SAVE_SIZED_VALUE, vec![b, 1, 0x7000]),
            ]
        );
        assert_eq!(log[1].0, CRITICAL_SECTION_ENTER);
        assert_eq!(log.last().unwrap().0, CRITICAL_SECTION_LEAVE);
    }
}
