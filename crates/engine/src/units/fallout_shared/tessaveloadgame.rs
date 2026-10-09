//! `fallout shared/tessaveloadgame.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit has 192 functions (`ledger queue "fallout shared/tessaveloadgame.cpp"`);
//! it is translated in address order, a session at a time. State of this
//! file: the first 40 functions, `00486a90` and `00666050` (two
//! `NiTPointerMap` instance methods the linker placed far from the unit) and
//! `00854e10` to `008572c0`. The next session continues at `008572f0`.
//!
//! What is here:
//!
//! - `ChangeData` and `ChangesMap` (the per-form record of what changed, and
//!   the hash map of those records keyed by form id): the accessors
//!   `00854e10` to `00855220`, the map's destructors and
//!   `ChangesMap::RemoveAllChanges`;
//! - the three smaller maps `InteriorCellNewReferencesMap`,
//!   `ExteriorCellNewReferencesMap` and `NumericIDBufferMap` (constructor,
//!   destructor and scalar deleting destructor of each);
//! - `SaveStats` (the statistics of a save, which `PrintStats` writes out as
//!   a text file), `SaveStats::Stats` and the small file-writing helper;
//! - `TESSaveLoadGame::RemoveChanges` and the save routine `00856ca0`.
//!
//! Layouts and constants are below. What the next session needs:
//!
//! - `TESSaveLoadGame` (0x1C8 bytes, the size `TES`'s constructor allocates),
//!   `ChangeData`, `SaveStats`, `Stats`, `ExtraStat`, `LoadFormHeader`,
//!   `SaveFormHeader` and `FormAndFlags` are declared here with the fields
//!   used so far;
//! - the pointer `011de45c` is the game's `TESSaveLoadGame`; the save code
//!   reads it through memory and passes it as `this` to several helpers that
//!   are members of the same class (`00857b50`, `008579e0`, `0085b320`,
//!   `0047c850`);
//! - the shared map and list helpers (`NiTMapBase::GetFirstPos` `004b9ba0`,
//!   `GetNext` `006b7f20` for a map keyed by `unsigned int`, `00863bc0` for
//!   `SaveStats`'s map keyed by a byte, `RemoveAll` `00438af0`, `RemoveAt`
//!   `00405430`, `GetAt` `00853130`, `SetAt` `00844700`; the `BSSimpleList`
//!   node item `006815c0`, next `00726070`, `RemoveAll` `00470470`, scalar
//!   deleting destructor `004702f0`) are called by address.
//!
//! Not translated: the compiler's exception-unwinding frames (the `FS:[0]`
//! chains and state variables) and the stack-cookie check of `00855ba0`.
//! The locals the game keeps on its stack and passes by address (the 4-byte
//! allocation-scope object, out parameters, the buffers and the file object
//! of `PrintStats`, the form header of `00856ca0`) are heap blocks here,
//! freed where the game's scope ends.
//!
//! The decompiler dropped or mis-attached several arguments in this unit
//! (the `this` of `0047c850`, `0085b320`, `00857b50`; the five stack
//! arguments of the "can't save" message, which belong to `007052f0`; the
//! base-class destructor calls of the map destructors), so the translations
//! follow the disassembly.

#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------------
// Layouts

layout! {
    /// `ChangeData` (Xbox PDB), 8 bytes: what is saved for one form.
    pub struct ChangeData: 0x08 {
        /// `iFlags` (Xbox PDB): the changed-parts bits.
        0x00 iFlags: u32,
        /// `pBuffer` (Xbox PDB): the form's pre-built save buffer, or null.
        0x04 pBuffer: Ptr,
    }

    /// `ChangesMap` (Xbox PDB): `NiTPointerMap<unsigned int, ChangeData *>`,
    /// 0x10 bytes. Vtable at +0 (slot 0x14 `NewItem`, 0x18 `DeleteItem`).
    pub struct ChangesMap: 0x10 {
        /// `m_uiHashSize` (Xbox PDB).
        0x04 m_uiHashSize: u32,
        /// `m_ppkHashTable` (Xbox PDB).
        0x08 m_ppkHashTable: Ptr,
        /// The allocator subobject (`AntiBloatAllocator`, Xbox PDB
        /// `m_kAllocator`), which holds the entry count (`NewItem` and
        /// `DeleteItem` pass its address).
        0x0C m_kAllocator: u32,
    }

    /// `InteriorCellNewReferencesMap` (Xbox PDB): a pointer map from a cell
    /// form id to a `BSSimpleList<unsigned int> *`, 0x10 bytes.
    pub struct InteriorCellNewReferencesMap: 0x10 {
        /// `m_uiHashSize` (Xbox PDB).
        0x04 m_uiHashSize: u32,
    }

    /// `ExteriorCellNewReferencesMap` (Xbox PDB): a pointer map to a
    /// `BSSimpleList<ExteriorCellReferenceData *> *`, 0x10 bytes.
    pub struct ExteriorCellNewReferencesMap: 0x10 {
        /// `m_uiHashSize` (Xbox PDB).
        0x04 m_uiHashSize: u32,
    }

    /// `NumericIDBufferMap` (Xbox PDB): a pointer map from an id to a buffer
    /// (`void *`), 0x10 bytes.
    pub struct NumericIDBufferMap: 0x10 {
        /// `m_uiHashSize` (Xbox PDB).
        0x04 m_uiHashSize: u32,
    }

    /// `SaveStats` (Xbox PDB), 8 bytes: the statistics of one save.
    pub struct SaveStats: 0x08 {
        /// `pStatsMap` (Xbox PDB): `NiTPointerMap<unsigned char,
        /// BSSimpleList<LoadFormHeader *> *> *`, from form type to the list
        /// of that type's headers, sorted by descending size.
        0x00 pStatsMap: Ptr,
        /// `pExtraStats` (Xbox PDB): `BSSimpleList<SaveStats::ExtraStat *> *`.
        0x04 pExtraStats: Ptr,
    }

    /// `SaveStats::Stats` (Xbox PDB), 0xC bytes: running count, total,
    /// minimum and maximum size.
    pub struct Stats: 0x0C {
        /// `iNum` (Xbox PDB).
        0x00 iNum: i32,
        /// `iTotalSize` (Xbox PDB).
        0x04 iTotalSize: i32,
        /// `iMinSize` (Xbox PDB).
        0x08 iMinSize: u16,
        /// `iMaxSize` (Xbox PDB).
        0x0A iMaxSize: u16,
    }

    /// `SaveStats::ExtraStat` (Xbox PDB), 8 bytes: a size and its description.
    pub struct ExtraStat: 0x08 {
        /// `iSize` (Xbox PDB).
        0x00 iSize: u32,
        /// `pDescription` (Xbox PDB): a heap copy of the text.
        0x04 pDescription: Ptr,
    }

    /// `LoadFormHeader` (Xbox PDB), 0xC bytes, packed (the flags word is
    /// unaligned).
    pub struct LoadFormHeader: 0x0C {
        /// `iFormID` (Xbox PDB).
        0x00 iFormID: u32,
        /// `cFormType` (Xbox PDB).
        0x04 cFormType: u8,
        /// `iFlags` (Xbox PDB), at the odd offset +5.
        0x05 iFlags: u32,
        /// `cVersion` (Xbox PDB).
        0x09 cVersion: u8,
        /// `iSize` (Xbox PDB).
        0x0A iSize: u16,
    }

    /// `SaveFormHeader` (Xbox PDB), 0xA bytes: the `LoadFormHeader` without
    /// the size.
    pub struct SaveFormHeader: 0x0A {
        /// `iFormID` (Xbox PDB).
        0x00 iFormID: u32,
        /// `cFormType` (Xbox PDB).
        0x04 cFormType: u8,
        /// `iFlags` (Xbox PDB), at the odd offset +5.
        0x05 iFlags: u32,
        /// `cVersion` (Xbox PDB).
        0x09 cVersion: u8,
    }

    /// `FormAndFlags` (Xbox PDB), 0x10 bytes.
    pub struct FormAndFlags: 0x10 {
        /// `pForm` (Xbox PDB).
        0x00 pForm: Ptr,
        /// `iFlags` (Xbox PDB).
        0x04 iFlags: u32,
        /// `iOldFlags` (Xbox PDB).
        0x08 iOldFlags: u32,
        /// `cVersion` (Xbox PDB).
        0x0C cVersion: u8,
    }

    /// `TESSaveLoadGame` (Xbox PDB), 0x1C8 bytes on the PC too (the size
    /// `TES` allocates); the fields this unit's first functions use, at the
    /// offsets the PC code uses (equal to the Xbox PDB's).
    pub struct TESSaveLoadGame: 0x1C8 {
        /// `m_pChanges` (Xbox PDB): the `ChangesMap`.
        0x00 m_pChanges: Ptr<ChangesMap>,
        /// `m_pBuffer` (Xbox PDB): the current form's pre-built buffer.
        0x14 m_pBuffer: Ptr,
        /// `m_pSaveLoadStats` (Xbox PDB): the `SaveStats`, or null.
        0x44 m_pSaveLoadStats: Ptr<SaveStats>,
        /// `m_pCurrentlySavingFormHeader` (Xbox PDB).
        0x88 m_pCurrentlySavingFormHeader: Ptr,
    }
}

// ---------------------------------------------------------------------------
// Constants: callees and data outside this file

/// `operator new(size)` (`00401000`).
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete(block)` (`00401030`).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// The dword at `this + 0x0C` (`MOV EAX,[ECX+0xC]`): for a form, its form id.
const FORM_ID: u32 = 0x0084_e3a0;
/// `MOV EAX,[ECX]`: the word at the address `this` (the flags of a
/// `ChangeData`; the first word of a file object).
const READ_WORD: u32 = 0x0055_9450;
/// The global that holds the pointer to the game's `TESSaveLoadGame`.
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
/// `0047c850`: takes a `TESSaveLoadGame` and returns false in this build
/// (`XOR AL,AL`); the save code asks it before every write.
const SAVE_LOAD_UNAVAILABLE: u32 = 0x0047_c850;
/// `TESSaveLoadGame::GetSavingAllowed` (Xbox PDB), `this` the game.
const GET_SAVING_ALLOWED: u32 = 0x0086_16f0;
/// `(form flags at +8) & 0x4000` (`SETNZ`): the form is deleted.
const FORM_IS_DELETED: u32 = 0x0040_77c0;

/// The allocation scope: `this` is a 4-byte object, then (0x11, 1, file,
/// line); `00404eb0` constructs it, `00404ee0` destroys it.
const SCOPE_ENTER: u32 = 0x0040_4eb0;
const SCOPE_LEAVE: u32 = 0x0040_4ee0;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\TESSaveLoadGame.cpp"`.
const SOURCE_FILE: u32 = 0x0108_04c8;

/// `NiTMapBase<unsigned int, X *>::GetAt(key, &value) -> bool`.
const MAP_GET_AT: u32 = 0x0085_3130;
/// `NiTMapBase::SetAt(key, value)` of `ChangesMap`.
const CHANGES_MAP_SET_AT: u32 = 0x0084_4700;
/// `NiTMapBase<unsigned int, X *>::RemoveAt(key) -> bool`.
const MAP_REMOVE_AT: u32 = 0x0040_5430;
/// `NiTMapBase::GetFirstPos`: the first used slot's entry, or 0.
const MAP_FIRST_POSITION: u32 = 0x004b_9ba0;
/// `NiTMapBase<unsigned int, X *>::GetNext(&pos, &key, &value)`.
const MAP_NEXT: u32 = 0x006b_7f20;
/// `SaveStats`'s map: `GetNext(&pos, &key (a byte), &value)`.
const BYTE_MAP_NEXT: u32 = 0x0086_3bc0;
/// `SaveStats`'s map: `GetAt(key (a byte), &value) -> bool`.
const BYTE_MAP_GET_AT: u32 = 0x0086_3b40;
/// `SaveStats`'s map: `SetAt(key (a byte), value)`.
const BYTE_MAP_SET_AT: u32 = 0x0086_3a60;
/// `NiTMapBase::RemoveAll`.
const MAP_REMOVE_ALL: u32 = 0x0043_8af0;
/// The map allocator subobject's `Allocate` (called with `map + 0x0C`) and
/// `Deallocate(item)`.
const MAP_ALLOCATOR_NEW_ITEM: u32 = 0x0043_a010;
const MAP_ALLOCATOR_DELETE_ITEM: u32 = 0x0045_cee0;

/// `BSSimpleList` constructor (`0096a2d0`: item and next set to null; the
/// linker folded `ChangeData`'s constructor into it).
const SIMPLE_LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// `BSSimpleList` node's item address (`006815c0`: returns `this`; the same
/// code is the empty constructor of the form headers).
const LIST_NODE_ITEM: u32 = 0x0068_15c0;
/// A node's next node (`00726070`: `[this + 4]`).
const LIST_NODE_NEXT: u32 = 0x0072_6070;
/// `BSSimpleList::RemoveAll`.
const LIST_REMOVE_ALL: u32 = 0x0047_0470;
/// `BSSimpleList` scalar deleting destructor: `this` and the delete flag.
const LIST_SCALAR_DELETE: u32 = 0x0047_02f0;
/// `BSSimpleList::AddHead(&item)`.
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
/// `BSSimpleList::Insert(item, comparator)`, keeping the list sorted by the
/// comparator (a `cdecl` function of two items).
const LIST_INSERT: u32 = 0x007a_7eb0;
/// The comparator `fn_00855a20` passes to it.
const STATS_COMPARATOR: u32 = 0x0085_5b60;

/// `strlen` through the game's wrapper.
const STRLEN: u32 = 0x0044_a670;
/// `strcpy_s(destination, size, source)` through the game's wrapper.
const STRING_COPY: u32 = 0x0040_6d30;
/// `strcat_s(destination, size, source)`.
const STRING_CAT: u32 = 0x0040_6d50;
/// `sprintf_s(buffer, size, format, ...)`.
const FORMAT: u32 = 0x0040_6d00;
/// `strcmp(a, b)`.
const STRING_COMPARE: u32 = 0x0040_8b20;
/// `__RTDynamicCast(object, vfDelta, sourceType, targetType, isReference)`.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;

/// The `NiTPointerMap` base constructors (`this`, hash size) and
/// destructors of the maps this unit owns.
const CHANGES_MAP_BASE_DESTRUCT: u32 = 0x0086_3640;
const INTERIOR_MAP_BASE_CONSTRUCT: u32 = 0x0086_3390;
const INTERIOR_MAP_BASE_DESTRUCT: u32 = 0x0086_3740;
const EXTERIOR_MAP_BASE_CONSTRUCT: u32 = 0x0086_33c0;
const EXTERIOR_MAP_BASE_DESTRUCT: u32 = 0x0086_3860;
const NUMERIC_ID_MAP_BASE_CONSTRUCT: u32 = 0x0086_33f0;
const NUMERIC_ID_MAP_BASE_DESTRUCT: u32 = 0x0086_3960;
/// `SaveStats`'s `NiTPointerMap<unsigned char, ...>` constructor (`this`,
/// hash size).
const STATS_MAP_CONSTRUCT: u32 = 0x0086_3420;
/// Hash size of the three reference maps and of `SaveStats`'s map.
const HASH_SIZE: u32 = 0x25;

/// Vtables of the maps this unit owns.
const CHANGES_MAP_VTABLE: u32 = 0x0108_04a8;
const INTERIOR_MAP_VTABLE: u32 = 0x0108_0514;
const EXTERIOR_MAP_VTABLE: u32 = 0x0108_0534;
const NUMERIC_ID_MAP_VTABLE: u32 = 0x0108_0554;

/// `TESSaveLoadGame` members the save routine calls (`this` the game unless
/// noted).
const SAVE_HEADER: u32 = 0x0086_1130; // (file, name)
const SAVE_PLUGIN_LIST: u32 = 0x0085_b240; // (file)
const SAVE_GLOBAL_DATA: u32 = 0x0085_8030; // (file)
const SAVE_FINAL_DATA: u32 = 0x0085_8570; // (file)
const SAVE_NUMERIC_ID_ARRAYS: u32 = 0x0086_1d10; // (file)
const CHECK_FLAGS: u32 = 0x0085_91b0; // (form, flags) -> flags
const GET_INITIAL_DATA_SAVE_SIZE: u32 = 0x0085_a450; // (form, flags) -> u16
const SAVE_INITIAL_DATA: u32 = 0x0085_a520; // (form, flags)
const CREATE_BUFFER: u32 = 0x0085_8600; // (size) -> buffer
const WRITE_FILE: u32 = 0x0085_86a0; // (file, buffer, size)
const FREE_BUFFER: u32 = 0x0085_8700; // (buffer)
/// Writes `size` bytes at `data` to the file: (file, data, size).
const WRITE_BYTES: u32 = 0x0085_7b50;
/// Reads 4 bytes of the current buffer into `data`: (data, size).
const READ_BYTES: u32 = 0x0085_79e0;
/// The version number of the save format.
const CURRENT_VERSION: u32 = 0x008d_f040;
/// Opens the save file: (file or null, name, 0) -> the file.
const OPEN_SAVE_FILE: u32 = 0x0085_7370;
/// The save's preparation steps, each `this` only.
const SAVE_PREPARE_A: u32 = 0x0086_27b0;
const SAVE_PREPARE_B: u32 = 0x0086_20f0;
const SAVE_PREPARE_C: u32 = 0x0085_6850;
/// The save's closing steps: (file) and (file, 0).
const SAVE_CLOSE_A: u32 = 0x0086_2150;
const SAVE_CLOSE_B: u32 = 0x0085_78b0;
/// The current position of a file (`this` the file).
const FILE_POSITION: u32 = 0x0047_20a0;
/// The form type byte of a form (`MOVZX EAX,byte [ECX+4]`).
const FORM_TYPE: u32 = 0x0040_1170;
/// `LookupFormByID(id)`, `cdecl`; null for none.
const LOOKUP_FORM: u32 = 0x0048_39c0;
/// The global that holds the object whose `Enter` and `Leave` (`00c3e310`,
/// `00c3e340`) bracket a save.
const SAVE_LOCK: u32 = 0x0120_2d98;
const SAVE_LOCK_ENTER: u32 = 0x00c3_e310;
const SAVE_LOCK_LEAVE: u32 = 0x00c3_e340;
/// File object virtual slots: `0x14` seek(position, mode), `0x18` name.
const FILE_SEEK: u32 = 0x14;
const FILE_GET_NAME: u32 = 0x18;
/// The seek mode the save routine uses (a global dword).
const SEEK_MODE: u32 = 0x010a_2480;
/// Form virtual slots: `0x50` the size of the changed data, `0x58` its save,
/// `0x130` its description.
const FORM_GET_CHANGES_SIZE: u32 = 0x50;
const FORM_SAVE_CHANGES: u32 = 0x58;
const FORM_GET_DESCRIPTION: u32 = 0x130;

/// The "can't save" message: the message queue getter (`this` the object at
/// `011d2364`) and `007052f0(queue, 0, icon, 0, time, 0)`.
const MESSAGE_QUEUE_OBJECT: u32 = 0x011d_2364;
const GET_MESSAGE_QUEUE: u32 = 0x004c_69f0;
const SHOW_MESSAGE: u32 = 0x0070_52f0;
/// `"Interface\Icons\Message Icons\glow_message_vaultboy_sad.dds"`.
const SAD_ICON: u32 = 0x0102_08a0;
/// The message's display time (a `float` constant).
const MESSAGE_TIME: u32 = 0x0101_62c0;
/// `"autosave"`.
const AUTOSAVE_NAME: u32 = 0x0107_fae0;

/// `FileFinder::Exist(path, 0, 0, -1)` and `BSSystemFile::DeleteFileA(path)`.
const FILE_EXISTS: u32 = 0x0045_6a20;
const FILE_DELETE: u32 = 0x00af_f0b0;
/// The text file object's constructor (`this`, path, 1, 2, 0) and
/// destructor.
const FILE_OBJECT_CONSTRUCT: u32 = 0x00b0_0900;
const FILE_OBJECT_DESTRUCT: u32 = 0x00b0_0950;
/// `BSSystemFile::DoWrite(this = file, text, size, 0, &scratch) -> error`.
const SYSTEM_FILE_DO_WRITE: u32 = 0x0085_6350;
/// `00aa15a0(file)`: the call that ends a save's use of the file.
const FILE_FLUSH: u32 = 0x00aa_15a0;
/// `TESSaveLoadGame::BuildChangesString(buffer, form, flags, type, 0)`,
/// `this` the singleton.
const BUILD_CHANGES_STRING: u32 = 0x0085_b320;
/// The name of a reference (`this` the reference) and the location name of
/// a map marker (`""` when it has none).
const REFERENCE_GET_NAME: u32 = 0x0055_d520;
const MAP_MARKER_GET_LOCATION_NAME: u32 = 0x0040_8da0;
/// The table of the form type names: 12 bytes per type, the first word the
/// name's address.
const FORM_TYPE_NAME_TABLE: u32 = 0x0118_7004;
/// The initializer of the record embedded at +8 of `fn_008572c0`'s object
/// (the first function of the next session's range).
const EMBEDDED_RECORD_INIT: u32 = 0x0085_72f0;

// ---------------------------------------------------------------------------
// Helpers

/// The allocation scope object the game keeps on its stack (`00404eb0`);
/// returns the object. `line` is the source line of the scope.
fn scope_enter(e: &mut Engine, line: u32) -> Ptr {
    let scope = Ptr::new(e.mem.alloc(4));
    e.call(SCOPE_ENTER, &args![scope, 0x11u32, 1u32, SOURCE_FILE, line]);
    scope
}

/// Destroys the scope object (`00404ee0`) and frees its block.
fn scope_leave(e: &mut Engine, scope: Ptr) {
    e.call(SCOPE_LEAVE, &args![scope]);
    e.mem.free(scope.addr());
}

/// `operator delete(block)`.
fn delete(e: &mut Engine, block: u32) {
    e.call(OPERATOR_DELETE, &args![block]);
}

/// Whether the save/load singleton reports the operation unavailable
/// (`0047c850` on `*011de45c`); false in this build.
fn singleton_unavailable(e: &mut Engine) -> bool {
    let singleton: u32 = e.global(SAVE_LOAD_GAME);
    e.call(SAVE_LOAD_UNAVAILABLE, &args![singleton]).bool()
}

/// `0047c850` on the game the save routine was called on.
fn game_unavailable(e: &mut Engine, game: Ptr<TESSaveLoadGame>) -> bool {
    e.call(SAVE_LOAD_UNAVAILABLE, &args![game]).bool()
}

/// The key of a form: the dword at +0x0C.
fn form_key(e: &mut Engine, form: Ptr) -> u32 {
    e.call(FORM_ID, &args![form]).u32()
}

/// Deletes a `BSSimpleList` the maps own: `RemoveAll`, then the scalar
/// deleting destructor (flag 1) when the list exists.
fn destroy_list(e: &mut Engine, list: u32) {
    e.call(LIST_REMOVE_ALL, &args![list]);
    if list != 0 {
        e.call(LIST_SCALAR_DELETE, &args![list, 1u32]);
    }
}

/// Walks a map keyed by `unsigned int` with the game's iterator
/// (`GetFirstPos`, then `GetNext(&pos, &key, &value)` until the position is
/// 0), calling `visit(e, key, value)` for each entry. The position, key and
/// value cells are a 12-byte block, as in the game's stack frame.
fn for_each_entry(e: &mut Engine, map: Ptr, mut visit: impl FnMut(&mut Engine, u32, u32)) {
    let cells = e.mem.alloc(12);
    let first = e.call(MAP_FIRST_POSITION, &args![map]).u32();
    e.mem.set_u32(cells, first);
    while e.mem.u32(cells) != 0 {
        e.mem.set_u32(cells + 4, 0);
        e.mem.set_u32(cells + 8, 0);
        e.call(MAP_NEXT, &args![map, cells, cells + 4, cells + 8]);
        let (key, value) = (e.mem.u32(cells + 4), e.mem.u32(cells + 8));
        visit(e, key, value);
    }
    e.mem.free(cells);
}

// Translated from 00486a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTPointerMap<unsigned int, ChangeData *>::NewItem` (Xbox PDB): gets a
/// map entry from the allocator subobject at `this + 0x0C`.
pub fn ni_t_pointer_map_unsigned_int_change_data_p_new_item(
    e: &mut Engine,
    this: Ptr<ChangesMap>,
) -> Ptr {
    e.call(MAP_ALLOCATOR_NEW_ITEM, &args![this.byte_add(0x0C)])
        .ptr()
}

// Translated from 00666050 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTPointerMap<unsigned int, ChangeData *>::DeleteItem` (Xbox PDB):
/// clears the entry's value (`+8`) and gives the entry back to the allocator
/// subobject at `this + 0x0C`.
pub fn ni_t_pointer_map_unsigned_int_change_data_p_delete_item(
    e: &mut Engine,
    this: Ptr<ChangesMap>,
    item: Ptr,
) {
    e.mem.set_u32(item.addr() + 8, 0);
    e.call(MAP_ALLOCATOR_DELETE_ITEM, &args![this.byte_add(0x0C), item]);
}

// Translated from 00854e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ChangeData::~ChangeData`: frees the form's buffer when there is one.
pub fn fn_00854e10(e: &mut Engine, this: Ptr<ChangeData>) {
    let buffer = e.get(this, ChangeData::pBuffer);
    if !buffer.is_null() {
        delete(e, buffer.addr());
    }
}

// Translated from 00854e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ChangeData::AddFlags`: sets bits in `iFlags` unless the data already
/// carries a buffer.
pub fn fn_00854e40(e: &mut Engine, this: Ptr<ChangeData>, flags: u32) {
    if e.get(this, ChangeData::pBuffer).is_null() {
        let current = e.get(this, ChangeData::iFlags);
        e.set(this, ChangeData::iFlags, current | flags);
    }
}

// Translated from 00854e70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ChangeData::RemoveFlags`: clears bits in `iFlags` unless the data
/// already carries a buffer.
pub fn fn_00854e70(e: &mut Engine, this: Ptr<ChangeData>, flags: u32) {
    if e.get(this, ChangeData::pBuffer).is_null() {
        let current = e.get(this, ChangeData::iFlags);
        e.set(this, ChangeData::iFlags, !flags & current);
    }
}

// Translated from 00854ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ChangesMap::scalar deleting destructor` (Xbox PDB): destroys the map and
/// frees it when bit 0 of `flags` is set.
pub fn changes_map_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ChangesMap>,
    flags: u32,
) -> Ptr<ChangesMap> {
    fn_00854f00(e, this);
    if flags & 1 != 0 {
        delete(e, this.addr());
    }
    this
}

// Translated from 00854f00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ChangesMap::~ChangesMap`: sets the map's vtable, deletes every
/// `ChangeData`, then runs the `NiTPointerMap` base destructor (`00863640`).
pub fn fn_00854f00(e: &mut Engine, this: Ptr<ChangesMap>) {
    e.mem.set_u32(this.addr(), CHANGES_MAP_VTABLE);
    changes_map_remove_all_changes(e, this);
    e.call(CHANGES_MAP_BASE_DESTRUCT, &args![this]);
}

// Translated from 00854f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ChangesMap::RemoveAllChanges` (Xbox PDB): deletes every `ChangeData`
/// (the destructor with flag 1) and empties the map.
pub fn changes_map_remove_all_changes(e: &mut Engine, this: Ptr<ChangesMap>) {
    for_each_entry(e, this.cast(), |e, _key, value| {
        if value != 0 {
            fn_00854fe0(e, Ptr::new(value), 1);
        }
    });
    e.call(MAP_REMOVE_ALL, &args![this]);
}

// Translated from 00854fe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ChangeData` scalar deleting destructor: frees the buffer, and the
/// `ChangeData` itself when bit 0 of `flags` is set.
pub fn fn_00854fe0(e: &mut Engine, this: Ptr<ChangeData>, flags: u32) -> Ptr<ChangeData> {
    fn_00854e10(e, this);
    if flags & 1 != 0 {
        delete(e, this.addr());
    }
    this
}

// Translated from 00855010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `flags` to the `ChangeData` of `form`, creating it (flags 0, no
/// buffer) and putting it in the map when the form has none. Returns the
/// `ChangeData`.
pub fn fn_00855010(
    e: &mut Engine,
    this: Ptr<ChangesMap>,
    form: Ptr,
    flags: u32,
) -> Ptr<ChangeData> {
    let key = form_key(e, form);
    let cell = e.mem.alloc(4);
    let found = e.call(MAP_GET_AT, &args![this, key, cell]).bool();
    if !found {
        let scope = scope_enter(e, 0x104);
        let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
        let change_data = if block != 0 {
            e.call(SIMPLE_LIST_CONSTRUCT, &args![block]).u32()
        } else {
            0
        };
        e.mem.set_u32(cell, change_data);
        e.call(CHANGES_MAP_SET_AT, &args![this, key, change_data]);
        scope_leave(e, scope);
    }
    let change_data: Ptr<ChangeData> = Ptr::new(e.mem.u32(cell));
    e.mem.free(cell);
    fn_00854e40(e, change_data, flags);
    change_data
}

// Translated from 00855100 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `ChangeData` stored under `key`, or null.
pub fn fn_00855100(e: &mut Engine, this: Ptr<ChangesMap>, key: u32) -> Ptr<ChangeData> {
    let cell = e.mem.alloc(4);
    e.call(MAP_GET_AT, &args![this, key, cell]);
    let found = e.mem.u32(cell);
    e.mem.free(cell);
    Ptr::new(found)
}

// Translated from 00855130 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `ChangeData` of `form` (looked up by the form's key), or null.
pub fn fn_00855130(e: &mut Engine, this: Ptr<ChangesMap>, form: Ptr) -> Ptr<ChangeData> {
    let key = form_key(e, form);
    fn_00855100(e, this, key)
}

// Translated from 00855150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears `flags` from the `ChangeData` of `form`; when none are left it is
/// removed from the map and deleted. False when saving is unavailable or the
/// form has no `ChangeData`.
pub fn fn_00855150(e: &mut Engine, this: Ptr<ChangesMap>, form: Ptr, flags: u32) -> bool {
    if singleton_unavailable(e) {
        return false;
    }
    let change_data = fn_00855130(e, this, form);
    if change_data.is_null() {
        return false;
    }
    fn_00854e70(e, change_data, flags);
    if e.call(READ_WORD, &args![change_data]).u32() == 0 {
        let key = form_key(e, form);
        e.call(MAP_REMOVE_AT, &args![this, key]);
        if !change_data.is_null() {
            fn_00854fe0(e, change_data, 1);
        }
    }
    true
}

// Translated from 008551f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `fn_00855220` for the key of `form`.
pub fn fn_008551f0(e: &mut Engine, this: Ptr<ChangesMap>, form: Ptr, force: u8) -> bool {
    let key = form_key(e, form);
    fn_00855220(e, this, key, force)
}

// Translated from 00855220 (decompiled, FalloutNV.exe 1.4.0.525)
/// Drops the `ChangeData` stored under `key` when it has no buffer or when
/// `force` is set. False when saving is unavailable or there is none.
pub fn fn_00855220(e: &mut Engine, this: Ptr<ChangesMap>, key: u32, force: u8) -> bool {
    if singleton_unavailable(e) {
        return false;
    }
    let change_data = fn_00855100(e, this, key);
    if change_data.is_null() {
        return false;
    }
    if e.get(change_data, ChangeData::pBuffer).is_null() || force != 0 {
        e.call(MAP_REMOVE_AT, &args![this, key]);
        if !change_data.is_null() {
            fn_00854fe0(e, change_data, 1);
        }
    }
    true
}

// Translated from 008552b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InteriorCellNewReferencesMap::InteriorCellNewReferencesMap`: the pointer
/// map base with hash size 0x25, then this class's vtable.
pub fn fn_008552b0(
    e: &mut Engine,
    this: Ptr<InteriorCellNewReferencesMap>,
) -> Ptr<InteriorCellNewReferencesMap> {
    e.call(INTERIOR_MAP_BASE_CONSTRUCT, &args![this, HASH_SIZE]);
    e.mem.set_u32(this.addr(), INTERIOR_MAP_VTABLE);
    this
}

// Translated from 008552e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InteriorCellNewReferencesMap::scalar deleting destructor` (Xbox PDB).
pub fn interior_cell_new_references_map_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<InteriorCellNewReferencesMap>,
    flags: u32,
) -> Ptr<InteriorCellNewReferencesMap> {
    fn_00855310(e, this);
    if flags & 1 != 0 {
        delete(e, this.addr());
    }
    this
}

// Translated from 00855310 (decompiled, FalloutNV.exe 1.4.0.525)
/// `InteriorCellNewReferencesMap::~InteriorCellNewReferencesMap`: deletes
/// each cell's list of new references, empties the map, runs the base
/// destructor (`00863740`).
pub fn fn_00855310(e: &mut Engine, this: Ptr<InteriorCellNewReferencesMap>) {
    e.mem.set_u32(this.addr(), INTERIOR_MAP_VTABLE);
    for_each_entry(e, this.cast(), |e, _key, list| {
        if list != 0 {
            destroy_list(e, list);
        }
    });
    e.call(MAP_REMOVE_ALL, &args![this]);
    e.call(INTERIOR_MAP_BASE_DESTRUCT, &args![this]);
}

// Translated from 008553e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExteriorCellNewReferencesMap::ExteriorCellNewReferencesMap`.
pub fn fn_008553e0(
    e: &mut Engine,
    this: Ptr<ExteriorCellNewReferencesMap>,
) -> Ptr<ExteriorCellNewReferencesMap> {
    e.call(EXTERIOR_MAP_BASE_CONSTRUCT, &args![this, HASH_SIZE]);
    e.mem.set_u32(this.addr(), EXTERIOR_MAP_VTABLE);
    this
}

// Translated from 00855410 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExteriorCellNewReferencesMap::scalar deleting destructor` (Xbox PDB).
pub fn exterior_cell_new_references_map_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExteriorCellNewReferencesMap>,
    flags: u32,
) -> Ptr<ExteriorCellNewReferencesMap> {
    fn_00855440(e, this);
    if flags & 1 != 0 {
        delete(e, this.addr());
    }
    this
}

// Translated from 00855440 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExteriorCellNewReferencesMap::~ExteriorCellNewReferencesMap`: for each
/// cell, frees every `ExteriorCellReferenceData` in its list, then the list;
/// empties the map and runs the base destructor (`00863860`).
pub fn fn_00855440(e: &mut Engine, this: Ptr<ExteriorCellNewReferencesMap>) {
    e.mem.set_u32(this.addr(), EXTERIOR_MAP_VTABLE);
    for_each_entry(e, this.cast(), |e, _key, list| {
        if list != 0 {
            let mut node = list;
            while node != 0 {
                let item_slot = e.call(LIST_NODE_ITEM, &args![node]).u32();
                let item = e.mem.u32(item_slot);
                if item != 0 {
                    delete(e, item);
                }
                node = e.call(LIST_NODE_NEXT, &args![node]).u32();
            }
            destroy_list(e, list);
        }
    });
    e.call(MAP_REMOVE_ALL, &args![this]);
    e.call(EXTERIOR_MAP_BASE_DESTRUCT, &args![this]);
}

// Translated from 00855550 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NumericIDBufferMap::NumericIDBufferMap`.
pub fn fn_00855550(e: &mut Engine, this: Ptr<NumericIDBufferMap>) -> Ptr<NumericIDBufferMap> {
    e.call(NUMERIC_ID_MAP_BASE_CONSTRUCT, &args![this, HASH_SIZE]);
    e.mem.set_u32(this.addr(), NUMERIC_ID_MAP_VTABLE);
    this
}

// Translated from 00855580 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NumericIDBufferMap::scalar deleting destructor` (Xbox PDB).
pub fn numeric_id_buffer_map_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<NumericIDBufferMap>,
    flags: u32,
) -> Ptr<NumericIDBufferMap> {
    fn_008555b0(e, this);
    if flags & 1 != 0 {
        delete(e, this.addr());
    }
    this
}

// Translated from 008555b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NumericIDBufferMap::~NumericIDBufferMap`: frees every buffer, empties
/// the map and runs the base destructor (`00863960`).
pub fn fn_008555b0(e: &mut Engine, this: Ptr<NumericIDBufferMap>) {
    e.mem.set_u32(this.addr(), NUMERIC_ID_MAP_VTABLE);
    for_each_entry(e, this.cast(), |e, _key, buffer| {
        if buffer != 0 {
            delete(e, buffer);
        }
    });
    e.call(MAP_REMOVE_ALL, &args![this]);
    e.call(NUMERIC_ID_MAP_BASE_DESTRUCT, &args![this]);
}

// Translated from 00855660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SaveStats::SaveStats`: makes the per-type map (hash size 0x25) and the
/// empty list of extra stats.
pub fn fn_00855660(e: &mut Engine, this: Ptr<SaveStats>) -> Ptr<SaveStats> {
    let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
    let stats_map = if block != 0 {
        e.call(STATS_MAP_CONSTRUCT, &args![block, HASH_SIZE]).ptr()
    } else {
        Ptr::NULL
    };
    e.set(this, SaveStats::pStatsMap, stats_map);
    let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
    let extra_stats = if block != 0 {
        e.call(SIMPLE_LIST_CONSTRUCT, &args![block]).ptr()
    } else {
        Ptr::NULL
    };
    e.set(this, SaveStats::pExtraStats, extra_stats);
    this
}

// Translated from 00855730 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SaveStats::~SaveStats`: frees the headers of every type's list and the
/// lists, deletes the map through its destructor (vtable slot 0, flag 1),
/// then frees every extra stat (description and record) and its list.
pub fn fn_00855730(e: &mut Engine, this: Ptr<SaveStats>) {
    let stats_map = e.get(this, SaveStats::pStatsMap);
    // `00863bc0(&pos, &type, &list)`: three cells, the type a byte.
    let cells = e.mem.alloc(12);
    let first = e.call(MAP_FIRST_POSITION, &args![stats_map]).u32();
    e.mem.set_u32(cells, first);
    while e.mem.u32(cells) != 0 {
        e.call(
            BYTE_MAP_NEXT,
            &args![stats_map, cells, cells + 4, cells + 8],
        );
        let list = e.mem.u32(cells + 8);
        if list != 0 {
            let mut node = list;
            while node != 0 {
                let item_slot = e.call(LIST_NODE_ITEM, &args![node]).u32();
                let header = e.mem.u32(item_slot);
                node = e.call(LIST_NODE_NEXT, &args![node]).u32();
                if header != 0 {
                    delete(e, header);
                }
            }
            destroy_list(e, list);
        }
    }
    e.mem.free(cells);
    let stats_map = e.get(this, SaveStats::pStatsMap);
    if !stats_map.is_null() {
        e.vcall(stats_map.addr(), 0, &args![1u32]);
    }
    let extra_stats = e.get(this, SaveStats::pExtraStats);
    if !extra_stats.is_null() {
        let mut node = extra_stats.addr();
        while node != 0 {
            let item_slot = e.call(LIST_NODE_ITEM, &args![node]).u32();
            let stat: Ptr<ExtraStat> = Ptr::new(e.mem.u32(item_slot));
            if !stat.is_null() {
                let description = e.get(stat, ExtraStat::pDescription);
                delete(e, description.addr());
                delete(e, stat.addr());
            }
            node = e.call(LIST_NODE_NEXT, &args![node]).u32();
        }
        let extra_stats = e.get(this, SaveStats::pExtraStats);
        destroy_list(e, extra_stats.addr());
    }
}

// Translated from 008558a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SaveStats::AddExtraStat` (Xbox PDB): adds a record of `size` bytes with a
/// copy of `description` to the head of the extra stats list.
pub fn save_stats_add_extra_stat(
    e: &mut Engine,
    this: Ptr<SaveStats>,
    size: u32,
    description: Ptr,
) {
    let scope = scope_enter(e, 0x267);
    let stat: Ptr<ExtraStat> = e.call(OPERATOR_NEW, &args![8u32]).ptr();
    e.set(stat, ExtraStat::iSize, size);
    let length = e.call(STRLEN, &args![description]).u32().wrapping_add(1);
    let copy = e.call(OPERATOR_NEW, &args![length]).ptr();
    e.set(stat, ExtraStat::pDescription, copy);
    e.call(STRING_COPY, &args![copy, length, description]);
    let cell = e.mem.alloc(4);
    e.mem.set_u32(cell, stat.addr());
    let extra_stats = e.get(this, SaveStats::pExtraStats);
    e.call(LIST_ADD_HEAD, &args![extra_stats, cell]);
    e.mem.free(cell);
    scope_leave(e, scope);
}

// Translated from 00855970 (decompiled, FalloutNV.exe 1.4.0.525)
/// Records a saved form's header with its saved size: builds a
/// `LoadFormHeader` from the form id, type, flags and version of `header`
/// and `size`, and adds it to the stats (`fn_00855a20`).
pub fn fn_00855970(e: &mut Engine, this: Ptr<SaveStats>, header: Ptr<SaveFormHeader>, size: u16) {
    let scope = scope_enter(e, 0x276);
    let copy: Ptr<LoadFormHeader> = Ptr::new(e.mem.alloc(LoadFormHeader::SIZE));
    e.call(LIST_NODE_ITEM, &args![copy]);
    let flags = e.get(header, SaveFormHeader::iFlags);
    let form_id = e.get(header, SaveFormHeader::iFormID);
    let form_type = e.get(header, SaveFormHeader::cFormType);
    let version = e.get(header, SaveFormHeader::cVersion);
    e.set(copy, LoadFormHeader::iFlags, flags);
    e.set(copy, LoadFormHeader::iFormID, form_id);
    e.set(copy, LoadFormHeader::cFormType, form_type);
    e.set(copy, LoadFormHeader::cVersion, version);
    e.set(copy, LoadFormHeader::iSize, size);
    fn_00855a20(e, this, copy);
    e.mem.free(copy.addr());
    scope_leave(e, scope);
}

// Translated from 00855a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds a heap copy of `header` to the list of its form type in the stats
/// map (creating the list when the type has none), kept sorted by
/// `fn_00855b60` (largest first).
pub fn fn_00855a20(e: &mut Engine, this: Ptr<SaveStats>, header: Ptr<LoadFormHeader>) {
    let scope = scope_enter(e, 0x286);
    let block = e.call(OPERATOR_NEW, &args![0xCu32]).u32();
    let item: Ptr<LoadFormHeader> = if block != 0 {
        e.call(LIST_NODE_ITEM, &args![block]).ptr()
    } else {
        Ptr::NULL
    };
    for word in 0..3 {
        let value = e.mem.u32(header.addr() + 4 * word);
        e.mem.set_u32(item.addr() + 4 * word, value);
    }
    let form_type = e.get(item, LoadFormHeader::cFormType);
    let stats_map = e.get(this, SaveStats::pStatsMap);
    let list_cell = e.mem.alloc(4);
    let found = e
        .call(BYTE_MAP_GET_AT, &args![stats_map, form_type, list_cell])
        .bool();
    if !found {
        let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
        let list = if block != 0 {
            e.call(SIMPLE_LIST_CONSTRUCT, &args![block]).u32()
        } else {
            0
        };
        e.mem.set_u32(list_cell, list);
        e.call(BYTE_MAP_SET_AT, &args![stats_map, form_type, list]);
    }
    let list = e.mem.u32(list_cell);
    e.mem.free(list_cell);
    e.call(LIST_INSERT, &args![list, item, STATS_COMPARATOR]);
    scope_leave(e, scope);
}

// Translated from 00855b60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The comparison of two `LoadFormHeader`s the stats lists are sorted by:
/// -1 when `a` is larger than `b`, 1 when smaller, 0 when equal (so the list
/// runs from the largest to the smallest).
pub fn fn_00855b60(e: &mut Engine, a: Ptr<LoadFormHeader>, b: Ptr<LoadFormHeader>) -> i32 {
    let size_a = e.get(a, LoadFormHeader::iSize) as i32;
    let size_b = e.get(b, LoadFormHeader::iSize) as i32;
    if size_a > size_b {
        -1
    } else {
        (size_a < size_b) as i32
    }
}

// Translated from 00855ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SaveStats::PrintStats` (Xbox PDB): writes the statistics to the text
/// file `<path><extension>` (an existing file is deleted first): the table
/// header, then for each form type a section listing every saved form of
/// that type (form id, size, flags, version, name and the string of
/// changes) followed by that type's totals, then the extra stats and the
/// grand totals. A form that is not loaded prints "NOT LOADED"; otherwise
/// its name is the location name of a map marker or the name of a reference
/// when it has one, else its description (form vtable slot `0x130`).
/// Nothing is written when the file cannot be opened.
///
/// The `strlen` the game takes of the type heading is never used and is not
/// translated; the compiler's stack-cookie check and the exception frame are
/// not translated.
pub fn save_stats_print_stats(e: &mut Engine, this: Ptr<SaveStats>, path: Ptr) {
    const PATH: u32 = 0x000; // char[0x104]
    const FILE: u32 = 0x108; // the file object, 0x20 bytes
    const TOTAL: u32 = 0x128; // Stats
    const LINE: u32 = 0x138; // char[0x208]
    const CHANGES: u32 = 0x340; // char[0x1f8]
    const TYPE_NAME: u32 = 0x538; // char[10]
    const TYPE_STATS: u32 = 0x548; // Stats
    const CURSOR: u32 = 0x558; // position, type byte, list head
    const FRAME: u32 = 0x570;

    let frame = e.mem.alloc(FRAME);
    let (path_buffer, file, total) = (frame + PATH, frame + FILE, frame + TOTAL);
    let (line, changes, type_name) = (frame + LINE, frame + CHANGES, frame + TYPE_NAME);
    let (type_stats, cursor) = (frame + TYPE_STATS, frame + CURSOR);
    let singleton: u32 = e.global(SAVE_LOAD_GAME);

    e.call(STRING_COPY, &args![path_buffer, 0x104u32, path]);
    e.call(STRING_CAT, &args![path_buffer, 0x104u32, 0x0103_9788u32]);
    if e.call(FILE_EXISTS, &args![path_buffer, 0u32, 0u32, -1i32])
        .u32()
        != 0
    {
        e.call(FILE_DELETE, &args![path_buffer]);
    }
    e.call(
        FILE_OBJECT_CONSTRUCT,
        &args![file, path_buffer, 1u32, 2u32, 0u32],
    );
    if e.call(READ_WORD, &args![file]).u32() != 0 {
        e.call(FILE_OBJECT_DESTRUCT, &args![file]);
        e.mem.free(frame);
        return;
    }

    fn_008562c0(e, Ptr::new(total));
    e.call(
        FORMAT,
        &args![
            line,
            0x208u32,
            0x0108_0670u32,
            0x0108_0414u32,
            0x0108_041cu32,
            0x0104_4aecu32,
            0x0106_3d04u32,
            0x0105_ac4cu32,
            0x0108_0428u32
        ],
    );
    fn_00856300(e, this.cast(), Ptr::new(file), Ptr::new(line));

    let stats_map = e.get(this, SaveStats::pStatsMap);
    let first = e.call(MAP_FIRST_POSITION, &args![stats_map]).u32();
    e.mem.set_u32(cursor, first);
    while e.mem.u32(cursor) != 0 {
        e.call(
            BYTE_MAP_NEXT,
            &args![stats_map, cursor, cursor + 4, cursor + 8],
        );
        let form_type = e.mem.u8(cursor + 4);
        let list = e.mem.u32(cursor + 8);

        // The heading: "Form" for type 0, "Buffer" for 0x79, else the type's name.
        if form_type == 0 {
            e.call(FORMAT, &args![type_name, 10u32, 0x0104_469cu32]);
        } else if form_type == 0x79 {
            e.call(FORMAT, &args![type_name, 10u32, 0x0108_0668u32]);
        } else {
            let name = e.mem.u32(FORM_TYPE_NAME_TABLE + form_type as u32 * 12);
            e.call(FORMAT, &args![type_name, 10u32, 0x0101_9f08u32, name]);
        }
        e.call(FORMAT, &args![line, 0x208u32, 0x0108_03e4u32, type_name]);
        fn_00856300(e, this.cast(), Ptr::new(file), Ptr::new(line));
        fn_008562c0(e, Ptr::new(type_stats));

        let mut node = list;
        while node != 0 {
            let item_slot = e.call(LIST_NODE_ITEM, &args![node]).u32();
            let header: Ptr<LoadFormHeader> = Ptr::new(e.mem.u32(item_slot));
            if !header.is_null() {
                let form_id = e.get(header, LoadFormHeader::iFormID);
                let size = e.get(header, LoadFormHeader::iSize);
                let flags = e.get(header, LoadFormHeader::iFlags);
                let version = e.get(header, LoadFormHeader::cVersion);
                let header_type = e.get(header, LoadFormHeader::cFormType);
                let form = e.call(LOOKUP_FORM, &args![form_id]).u32();

                let stats: Ptr<Stats> = Ptr::new(type_stats);
                if size > e.get(stats, Stats::iMaxSize) {
                    e.set(stats, Stats::iMaxSize, size);
                }
                if size < e.get(stats, Stats::iMinSize) {
                    e.set(stats, Stats::iMinSize, size);
                }
                let total_size = e.get(stats, Stats::iTotalSize);
                e.set(
                    stats,
                    Stats::iTotalSize,
                    total_size.wrapping_add(size as i32),
                );
                let count = e.get(stats, Stats::iNum);
                e.set(stats, Stats::iNum, count.wrapping_add(1));

                e.call(
                    BUILD_CHANGES_STRING,
                    &[singleton, changes, form, flags, header_type as u32, 0],
                );
                let map_marker = e
                    .call(
                        DYNAMIC_CAST,
                        &args![form, 0i32, 0x0118_3028u32, 0x0118_3158u32, 0i32],
                    )
                    .u32();
                let reference = e
                    .call(
                        DYNAMIC_CAST,
                        &args![form, 0i32, 0x0118_3028u32, 0x0118_41ccu32, 0i32],
                    )
                    .u32();
                let text = if form != 0 {
                    let mut name = 0u32;
                    if reference != 0 {
                        name = e.call(REFERENCE_GET_NAME, &args![reference]).u32();
                    }
                    if map_marker != 0 && (name == 0 || is_empty_string(e, name)) {
                        name = e
                            .call(MAP_MARKER_GET_LOCATION_NAME, &args![map_marker])
                            .u32();
                    }
                    if name == 0 || is_empty_string(e, name) {
                        name = e.vcall(form, FORM_GET_DESCRIPTION, &args![]).u32();
                    }
                    name
                } else {
                    0x0108_063c // "NOT LOADED"
                };
                e.call(
                    FORMAT,
                    &args![
                        line,
                        0x208u32,
                        0x0108_0648u32,
                        form_id,
                        size as u32,
                        flags,
                        version as u32,
                        text,
                        changes
                    ],
                );
                fn_00856300(e, this.cast(), Ptr::new(file), Ptr::new(line));
            }
            node = e.call(LIST_NODE_NEXT, &args![node]).u32();
        }

        // This type's totals.
        let stats: Ptr<Stats> = Ptr::new(type_stats);
        let (count, total_size) = (e.get(stats, Stats::iNum), e.get(stats, Stats::iTotalSize));
        let (min_size, max_size) = (e.get(stats, Stats::iMinSize), e.get(stats, Stats::iMaxSize));
        let average = total_size as f64 / count as f64;
        e.call(
            FORMAT,
            &args![
                line,
                0x208u32,
                0x0108_05d8u32,
                type_name,
                count,
                type_name,
                total_size,
                type_name,
                min_size as u32,
                type_name,
                max_size as u32,
                type_name,
                average
            ],
        );
        fn_00856300(e, this.cast(), Ptr::new(file), Ptr::new(line));

        let grand: Ptr<Stats> = Ptr::new(total);
        if max_size > e.get(grand, Stats::iMaxSize) {
            e.set(grand, Stats::iMaxSize, max_size);
        }
        if min_size < e.get(grand, Stats::iMinSize) {
            e.set(grand, Stats::iMinSize, min_size);
        }
        let grand_size = e.get(grand, Stats::iTotalSize);
        e.set(
            grand,
            Stats::iTotalSize,
            grand_size.wrapping_add(total_size),
        );
        let grand_count = e.get(grand, Stats::iNum);
        e.set(grand, Stats::iNum, grand_count.wrapping_add(count));
    }

    fn_00856300(e, this.cast(), Ptr::new(file), Ptr::new(0x0108_05c4));
    let mut node = e.get(this, SaveStats::pExtraStats).addr();
    while node != 0 {
        let item_slot = e.call(LIST_NODE_ITEM, &args![node]).u32();
        let stat: Ptr<ExtraStat> = Ptr::new(e.mem.u32(item_slot));
        if !stat.is_null() {
            let size = e.get(stat, ExtraStat::iSize);
            let description = e.get(stat, ExtraStat::pDescription);
            e.call(
                FORMAT,
                &args![line, 0x208u32, 0x0108_0254u32, size, description],
            );
            fn_00856300(e, this.cast(), Ptr::new(file), Ptr::new(line));
            let grand: Ptr<Stats> = Ptr::new(total);
            let grand_size = e.get(grand, Stats::iTotalSize);
            e.set(
                grand,
                Stats::iTotalSize,
                grand_size.wrapping_add(size as i32),
            );
        }
        node = e.call(LIST_NODE_NEXT, &args![node]).u32();
    }

    let grand: Ptr<Stats> = Ptr::new(total);
    let (count, total_size) = (e.get(grand, Stats::iNum), e.get(grand, Stats::iTotalSize));
    let (min_size, max_size) = (e.get(grand, Stats::iMinSize), e.get(grand, Stats::iMaxSize));
    let average = total_size as f64 / count as f64;
    e.call(
        FORMAT,
        &args![
            line,
            0x208u32,
            0x0108_0570u32,
            count,
            total_size,
            min_size as u32,
            max_size as u32,
            average
        ],
    );
    fn_00856300(e, this.cast(), Ptr::new(file), Ptr::new(line));
    e.call(FILE_OBJECT_DESTRUCT, &args![file]);
    e.mem.free(frame);
}

/// `strcmp(name, "") == 0` against the game's empty string (`01011584`).
fn is_empty_string(e: &mut Engine, name: u32) -> bool {
    e.call(STRING_COMPARE, &args![name, 0x0101_1584u32]).i32() == 0
}

// Translated from 008562c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SaveStats::Stats::Stats`: no entries, total 0, minimum 0xFFFF, maximum 0.
pub fn fn_008562c0(e: &mut Engine, this: Ptr<Stats>) -> Ptr<Stats> {
    e.set(this, Stats::iMaxSize, 0);
    e.set(this, Stats::iTotalSize, 0);
    e.set(this, Stats::iNum, 0);
    e.set(this, Stats::iMinSize, 0xFFFF);
    this
}

// Translated from 00856300 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the text (without its terminator) to `file` through
/// `BSSystemFile::DoWrite`; true when the write reported no error. `this` is
/// not used.
pub fn fn_00856300(e: &mut Engine, _this: Ptr, file: Ptr, text: Ptr) -> bool {
    let length = e.call(STRLEN, &args![text]).u32();
    let scratch = e.mem.alloc(16);
    let error = e
        .call(
            SYSTEM_FILE_DO_WRITE,
            &args![file, text, length, 0u32, scratch],
        )
        .u32();
    e.mem.free(scratch);
    error == 0
}

// Translated from 00856c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESSaveLoadGame::RemoveChanges` (Xbox PDB): unless the form is flagged
/// deleted (bit 0x4000 of its flags), drops its `ChangeData`
/// (`fn_008551f0`).
pub fn tes_save_load_game_remove_changes(
    e: &mut Engine,
    this: Ptr<TESSaveLoadGame>,
    form: Ptr,
    force: u8,
) {
    if !e.call(FORM_IS_DELETED, &args![form]).bool() {
        let changes = e.get(this, TESSaveLoadGame::m_pChanges);
        fn_008551f0(e, changes, form, force);
    }
}

// Translated from 00856ca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Saves the game (the routine behind "sCantSaveNow"): refuses, with the
/// sad-Vault-Boy message, unless saving is allowed or the save is an
/// autosave; otherwise takes the save lock, opens the file (`file` is an
/// existing stream or null, `name` the save's name), writes the header, the
/// plugin list and the global data, then for every `ChangeData` of the
/// changes map the 10-byte form header, the size and the changes (the
/// pre-built buffer when the data has one, else the form's own initial data
/// and changes), the final data and the numeric id arrays, patches the
/// positions written at the start, prints the statistics when
/// `collect_stats` asked for them, and closes the file. True when it saved.
pub fn fn_00856ca0(
    e: &mut Engine,
    this: Ptr<TESSaveLoadGame>,
    file: Ptr,
    name: Ptr,
    collect_stats: bool,
) -> bool {
    // The frame cells the game keeps on its stack: the form header, the size
    // words, the reserved word, the end position and the form count.
    const HEADER: u32 = 0x00;
    const BUFFER_SIZE: u32 = 0x10; // the 4 bytes `READ_BYTES` fills
    const FORM_SIZE: u32 = 0x14; // u16
    const RESERVED: u32 = 0x18;
    const END_POSITION: u32 = 0x1C;
    const COUNT: u32 = 0x20;
    const FRAME: u32 = 0x24;

    let scope = scope_enter(e, 0x466);
    let allowed = game_unavailable(e, this)
        || e.call(GET_SAVING_ALLOWED, &args![this]).bool()
        || (!name.is_null() && e.call(STRING_COMPARE, &args![name, AUTOSAVE_NAME]).i32() == 0);
    if !allowed {
        let queue = e
            .call(GET_MESSAGE_QUEUE, &args![MESSAGE_QUEUE_OBJECT])
            .u32();
        let time: f32 = e.global(MESSAGE_TIME);
        e.call(
            SHOW_MESSAGE,
            &args![queue, 0u32, SAD_ICON, 0u32, time, 0u32],
        );
        scope_leave(e, scope);
        return false;
    }

    let lock: u32 = e.global(SAVE_LOCK);
    e.call(SAVE_LOCK_ENTER, &args![lock]);
    e.call(SAVE_PREPARE_A, &args![this]);
    let mut stream = Ptr::NULL;
    if !game_unavailable(e, this) {
        stream = e.call(OPEN_SAVE_FILE, &args![this, file, name, 0u32]).ptr();
    }
    if collect_stats {
        let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
        let stats = if block != 0 {
            fn_00855660(e, Ptr::new(block))
        } else {
            Ptr::NULL
        };
        e.set(this, TESSaveLoadGame::m_pSaveLoadStats, stats);
    }
    e.call(SAVE_PREPARE_B, &args![this]);
    e.call(SAVE_PREPARE_C, &args![this]);
    e.call(SAVE_HEADER, &args![this, stream, name]);
    e.call(SAVE_PLUGIN_LIST, &args![this, stream]);

    let frame = e.mem.alloc(FRAME);
    let header: Ptr<SaveFormHeader> = Ptr::new(frame + HEADER);
    let mut start_position = 0u32;
    if !game_unavailable(e, this) {
        start_position = e.call(FILE_POSITION, &args![stream]).u32();
    }
    e.mem.set_u32(frame + RESERVED, 0);
    e.call(WRITE_BYTES, &args![this, stream, frame + RESERVED, 4u32]);
    e.call(WRITE_BYTES, &args![this, stream, frame + RESERVED, 4u32]);
    e.call(SAVE_GLOBAL_DATA, &args![this, stream]);
    e.mem.set_u32(frame + COUNT, 0);

    let singleton: u32 = e.global(SAVE_LOAD_GAME);
    let changes = e.get(this, TESSaveLoadGame::m_pChanges);
    for_each_entry(e, changes.cast(), |e, form_id, change_data| {
        if form_id == 0 || change_data == 0 {
            return;
        }
        let buffer = e.call(LIST_NODE_NEXT, &args![change_data]).u32();
        e.set(this, TESSaveLoadGame::m_pBuffer, Ptr::new(buffer));
        let flags = e.call(READ_WORD, &args![change_data]).u32();
        e.call(LIST_NODE_ITEM, &args![header]);
        e.set(header, SaveFormHeader::iFormID, form_id);
        e.set(header, SaveFormHeader::iFlags, flags);
        let version = e.call(CURRENT_VERSION, &args![this]).u8();
        e.set(header, SaveFormHeader::cVersion, version);

        if buffer != 0 {
            // The data has its own buffer: its first four bytes are the
            // size, the type and the version.
            e.call(READ_BYTES, &args![singleton, frame + BUFFER_SIZE, 4u32]);
            let form_type = e.mem.u8(frame + BUFFER_SIZE + 2);
            let version = e.mem.u8(frame + BUFFER_SIZE + 3);
            e.set(header, SaveFormHeader::cFormType, form_type);
            e.set(header, SaveFormHeader::cVersion, version);
            e.call(WRITE_BYTES, &args![singleton, stream, header, 10u32]);
            let count = e.mem.u32(frame + COUNT);
            e.mem.set_u32(frame + COUNT, count.wrapping_add(1));
            e.call(WRITE_BYTES, &args![this, stream, frame + BUFFER_SIZE, 2u32]);
            let size = e.mem.u16(frame + BUFFER_SIZE);
            if size != 0 {
                let saved = e.get(this, TESSaveLoadGame::m_pBuffer);
                e.call(WRITE_BYTES, &args![this, stream, saved, size as u32]);
            }
            let stats = e.get(this, TESSaveLoadGame::m_pSaveLoadStats);
            if !stats.is_null() {
                fn_00855970(e, stats, header, size);
            }
            e.set(this, TESSaveLoadGame::m_pBuffer, Ptr::NULL);
        } else {
            let form = e.call(LOOKUP_FORM, &args![form_id]).u32();
            if form == 0 {
                return;
            }
            let form_type = e.call(FORM_TYPE, &args![form]).u8();
            e.set(header, SaveFormHeader::cFormType, form_type);
            let flags = e.get(header, SaveFormHeader::iFlags);
            let flags = e.call(CHECK_FLAGS, &args![this, form, flags]).u32();
            e.set(header, SaveFormHeader::iFlags, flags);
            e.call(WRITE_BYTES, &args![singleton, stream, header, 10u32]);
            let count = e.mem.u32(frame + COUNT);
            e.mem.set_u32(frame + COUNT, count.wrapping_add(1));
            fn_00857230(e, this, header.cast());
            let mut size = e.vcall(form, FORM_GET_CHANGES_SIZE, &args![flags]).u16();
            let initial = e
                .call(GET_INITIAL_DATA_SAVE_SIZE, &args![this, form, flags])
                .u16();
            size = size.wrapping_add(initial);
            e.mem.set_u16(frame + FORM_SIZE, size);
            e.call(WRITE_BYTES, &args![this, stream, frame + FORM_SIZE, 2u32]);
            if size != 0 {
                let save_buffer = e.call(CREATE_BUFFER, &args![this, size as u32]).u32();
                e.call(SAVE_INITIAL_DATA, &args![this, form, flags]);
                e.vcall(form, FORM_SAVE_CHANGES, &args![flags]);
                e.call(WRITE_FILE, &args![this, stream, save_buffer, size as u32]);
                e.call(FREE_BUFFER, &args![this, save_buffer]);
            }
            fn_00857230(e, this, Ptr::NULL);
            let stats = e.get(this, TESSaveLoadGame::m_pSaveLoadStats);
            if !stats.is_null() {
                fn_00855970(e, stats, header, size);
            }
        }
    });

    e.call(SAVE_FINAL_DATA, &args![this, stream]);
    e.mem.set_u32(frame + END_POSITION, 0);
    if !game_unavailable(e, this) {
        let position = e.call(FILE_POSITION, &args![stream]).u32();
        e.mem.set_u32(frame + END_POSITION, position);
    }
    e.call(SAVE_NUMERIC_ID_ARRAYS, &args![this, stream]);
    if !game_unavailable(e, this) {
        let mode: u32 = e.global(SEEK_MODE);
        e.vcall(stream.addr(), FILE_SEEK, &args![start_position, mode]);
        e.call(
            WRITE_BYTES,
            &args![this, stream, frame + END_POSITION, 4u32],
        );
        e.call(WRITE_BYTES, &args![this, stream, frame + COUNT, 4u32]);
    }
    e.mem.free(frame);

    let stats = e.get(this, TESSaveLoadGame::m_pSaveLoadStats);
    if !stats.is_null() {
        let stream_name = e.vcall(stream.addr(), FILE_GET_NAME, &args![]).u32();
        save_stats_print_stats(e, stats, Ptr::new(stream_name));
        let stats = e.get(this, TESSaveLoadGame::m_pSaveLoadStats);
        if !stats.is_null() {
            fn_00857250(e, stats, 1);
        }
        e.set(this, TESSaveLoadGame::m_pSaveLoadStats, Ptr::NULL);
    }
    if !game_unavailable(e, this) {
        fn_00857210(e, stream);
        e.call(SAVE_CLOSE_A, &args![this, stream]);
        e.call(SAVE_CLOSE_B, &args![this, stream, 0u32]);
    }
    e.call(SAVE_LOCK_LEAVE, &args![lock]);
    scope_leave(e, scope);
    true
}

// Translated from 00857210 (decompiled, FalloutNV.exe 1.4.0.525)
/// Ends the save's use of the file through `00aa15a0`.
pub fn fn_00857210(e: &mut Engine, this: Ptr) {
    e.call(FILE_FLUSH, &args![this]);
}

// Translated from 00857230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `m_pCurrentlySavingFormHeader`.
pub fn fn_00857230(e: &mut Engine, this: Ptr<TESSaveLoadGame>, header: Ptr) {
    e.set(this, TESSaveLoadGame::m_pCurrentlySavingFormHeader, header);
}

// Translated from 00857250 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SaveStats` scalar deleting destructor: runs `fn_00855730`, frees the
/// object when bit 0 of `flags` is set.
pub fn fn_00857250(e: &mut Engine, this: Ptr<SaveStats>, flags: u32) -> Ptr<SaveStats> {
    fn_00855730(e, this);
    if flags & 1 != 0 {
        delete(e, this.addr());
    }
    this
}

// Translated from 00857280 (decompiled, FalloutNV.exe 1.4.0.525)
/// `FormAndFlags::FormAndFlags(form, flags, oldFlags, version)`.
pub fn fn_00857280(
    e: &mut Engine,
    this: Ptr<FormAndFlags>,
    form: Ptr,
    flags: u32,
    old_flags: u32,
    version: u8,
) -> Ptr<FormAndFlags> {
    e.set(this, FormAndFlags::pForm, form);
    e.set(this, FormAndFlags::iFlags, flags);
    e.set(this, FormAndFlags::iOldFlags, old_flags);
    e.set(this, FormAndFlags::cVersion, version);
    this
}

// Translated from 008572c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializes a local the save/load code keeps on its stack: two null
/// words, then the record embedded at +8 (`008572f0`, the first function of
/// the next session's range).
pub fn fn_008572c0(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), 0);
    e.mem.set_u32(this.addr() + 4, 0);
    e.call(EMBEDDED_RECORD_INIT, &args![this.byte_add(8)]);
    this
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x00486a90,
            ni_t_pointer_map_unsigned_int_change_data_p_new_item(Ptr<ChangesMap>) -> Ptr
        ),
        entry!(
            0x00666050,
            ni_t_pointer_map_unsigned_int_change_data_p_delete_item(Ptr<ChangesMap>, Ptr)
        ),
        entry!(0x00854e10, fn_00854e10(Ptr<ChangeData>)),
        entry!(0x00854e40, fn_00854e40(Ptr<ChangeData>, u32)),
        entry!(0x00854e70, fn_00854e70(Ptr<ChangeData>, u32)),
        entry!(
            0x00854ed0,
            changes_map_scalar_deleting_destructor(Ptr<ChangesMap>, u32) -> Ptr<ChangesMap>
        ),
        entry!(0x00854f00, fn_00854f00(Ptr<ChangesMap>)),
        entry!(0x00854f60, changes_map_remove_all_changes(Ptr<ChangesMap>)),
        entry!(
            0x00854fe0,
            fn_00854fe0(Ptr<ChangeData>, u32) -> Ptr<ChangeData>
        ),
        entry!(
            0x00855010,
            fn_00855010(Ptr<ChangesMap>, Ptr, u32) -> Ptr<ChangeData>
        ),
        entry!(
            0x00855100,
            fn_00855100(Ptr<ChangesMap>, u32) -> Ptr<ChangeData>
        ),
        entry!(
            0x00855130,
            fn_00855130(Ptr<ChangesMap>, Ptr) -> Ptr<ChangeData>
        ),
        entry!(0x00855150, fn_00855150(Ptr<ChangesMap>, Ptr, u32) -> bool),
        entry!(0x008551f0, fn_008551f0(Ptr<ChangesMap>, Ptr, u8) -> bool),
        entry!(0x00855220, fn_00855220(Ptr<ChangesMap>, u32, u8) -> bool),
        entry!(
            0x008552b0,
            fn_008552b0(Ptr<InteriorCellNewReferencesMap>) -> Ptr<InteriorCellNewReferencesMap>
        ),
        entry!(
            0x008552e0,
            interior_cell_new_references_map_scalar_deleting_destructor(
                Ptr<InteriorCellNewReferencesMap>,
                u32,
            ) -> Ptr<
                InteriorCellNewReferencesMap,
            >
        ),
        entry!(0x00855310, fn_00855310(Ptr<InteriorCellNewReferencesMap>)),
        entry!(
            0x008553e0,
            fn_008553e0(Ptr<ExteriorCellNewReferencesMap>) -> Ptr<ExteriorCellNewReferencesMap>
        ),
        entry!(
            0x00855410,
            exterior_cell_new_references_map_scalar_deleting_destructor(
                Ptr<ExteriorCellNewReferencesMap>,
                u32,
            ) -> Ptr<
                ExteriorCellNewReferencesMap,
            >
        ),
        entry!(0x00855440, fn_00855440(Ptr<ExteriorCellNewReferencesMap>)),
        entry!(
            0x00855550,
            fn_00855550(Ptr<NumericIDBufferMap>) -> Ptr<NumericIDBufferMap>
        ),
        entry!(
            0x00855580,
            numeric_id_buffer_map_scalar_deleting_destructor(
                Ptr<NumericIDBufferMap>,
                u32,
            )
                -> Ptr<NumericIDBufferMap>
        ),
        entry!(0x008555b0, fn_008555b0(Ptr<NumericIDBufferMap>)),
        entry!(0x00855660, fn_00855660(Ptr<SaveStats>) -> Ptr<SaveStats>),
        entry!(0x00855730, fn_00855730(Ptr<SaveStats>)),
        entry!(
            0x008558a0,
            save_stats_add_extra_stat(Ptr<SaveStats>, u32, Ptr)
        ),
        entry!(
            0x00855970,
            fn_00855970(Ptr<SaveStats>, Ptr<SaveFormHeader>, u16)
        ),
        entry!(0x00855a20, fn_00855a20(Ptr<SaveStats>, Ptr<LoadFormHeader>)),
        entry!(
            0x00855b60,
            fn_00855b60(Ptr<LoadFormHeader>, Ptr<LoadFormHeader>) -> i32
        ),
        entry!(0x00855ba0, save_stats_print_stats(Ptr<SaveStats>, Ptr)),
        entry!(0x008562c0, fn_008562c0(Ptr<Stats>) -> Ptr<Stats>),
        entry!(0x00856300, fn_00856300(Ptr, Ptr, Ptr) -> bool),
        entry!(
            0x00856c70,
            tes_save_load_game_remove_changes(Ptr<TESSaveLoadGame>, Ptr, u8)
        ),
        entry!(
            0x00856ca0,
            fn_00856ca0(Ptr<TESSaveLoadGame>, Ptr, Ptr, bool) -> bool
        ),
        entry!(0x00857210, fn_00857210(Ptr)),
        entry!(0x00857230, fn_00857230(Ptr<TESSaveLoadGame>, Ptr)),
        entry!(
            0x00857250,
            fn_00857250(Ptr<SaveStats>, u32) -> Ptr<SaveStats>
        ),
        entry!(
            0x00857280,
            fn_00857280(Ptr<FormAndFlags>, Ptr, u32, u32, u8) -> Ptr<FormAndFlags>
        ),
        entry!(0x008572c0, fn_008572c0(Ptr) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    type Log = Vec<(u32, Vec<u32>)>;
    type Lines = Rc<RefCell<Vec<String>>>;
    type Calls = Rc<RefCell<Vec<Vec<u32>>>>;
    type FileWrites = Rc<RefCell<Vec<(u32, Vec<u8>)>>>;

    fn returns(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn stub(e: &mut Engine, address: u32) {
        e.register(address, |_, _| Ret::default());
    }

    /// A double that returns `value` whatever it is called with.
    fn constant(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| returns(value));
    }

    /// An engine with the globals the unit reads mapped and working doubles
    /// for the small callees nearly every function uses: the dword getters,
    /// the list node accessors and constructor, `strlen`/`strcpy_s`, and the
    /// allocation scope (logged only). `operator new` and `delete` are the
    /// crate's own. Calls are logged.
    fn game() -> Engine {
        let mut e = Engine::new();
        e.map(0x011d_e000, 0x1000); // the TESSaveLoadGame pointer
        e.map(0x0120_2000, 0x1000); // the save lock
        e.map(0x010a_2000, 0x1000); // the seek mode
        e.map(0x0101_6000, 0x1000); // the message time
        let singleton = e.new_object::<TESSaveLoadGame>();
        e.set_global(SAVE_LOAD_GAME, singleton.addr());
        e.set_global(MESSAGE_TIME, 2.0f32);
        e.register(FORM_ID, |e, a| returns(e.mem.u32(a[0] + 0xc)));
        e.register(READ_WORD, |e, a| returns(e.mem.u32(a[0])));
        e.register(LIST_NODE_ITEM, |_, a| returns(a[0]));
        e.register(LIST_NODE_NEXT, |e, a| returns(e.mem.u32(a[0] + 4)));
        e.register(SAVE_LOAD_UNAVAILABLE, |_, _| returns(0));
        e.register(SIMPLE_LIST_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
            returns(a[0])
        });
        e.register(STRLEN, |e, a| returns(e.mem.cstr(a[0]).len() as u32));
        e.register(STRING_COPY, |e, a| {
            let text = e.mem.cstr(a[2]);
            e.mem.set_cstr(a[0], &text);
            Ret::default()
        });
        e.register(STRING_COMPARE, |e, a| {
            returns(e.mem.cstr(a[0]).cmp(&e.mem.cstr(a[1])) as i32 as u32)
        });
        stub(&mut e, SCOPE_ENTER);
        stub(&mut e, SCOPE_LEAVE);
        e.call_log = Some(vec![]);
        e
    }

    /// The logged argument lists of the calls to `address`.
    fn calls_to(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        let log: &Log = e.call_log.as_ref().unwrap();
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn freed(e: &Engine, block: u32) -> bool {
        e.mem.block_size(block).is_none()
    }

    /// A singly linked `BSSimpleList` of the items; returns the first node
    /// (0 for none).
    fn list_of(e: &mut Engine, items: &[u32]) -> u32 {
        let mut next = 0;
        for &item in items.iter().rev() {
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, item);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        next
    }

    /// The map iteration as the game's `GetFirstPos` / `GetNext` give it,
    /// over `entries` (key, value).
    fn install_entries(e: &mut Engine, entries: &[(u32, u32)]) {
        let queue = Rc::new(RefCell::new(VecDeque::from(entries.to_vec())));
        let first = queue.clone();
        e.register_double(MAP_FIRST_POSITION, move |_, _| {
            returns(!first.borrow().is_empty() as u32)
        });
        e.register_double(MAP_NEXT, move |e, a| {
            let (key, value) = queue.borrow_mut().pop_front().unwrap();
            e.mem.set_u32(a[2], key);
            e.mem.set_u32(a[3], value);
            e.mem.set_u32(a[1], !queue.borrow().is_empty() as u32);
            Ret::default()
        });
    }

    /// `install_entries` for `SaveStats`'s map, keyed by a byte.
    fn install_byte_entries(e: &mut Engine, entries: &[(u8, u32)]) {
        let queue = Rc::new(RefCell::new(VecDeque::from(entries.to_vec())));
        let first = queue.clone();
        e.register_double(MAP_FIRST_POSITION, move |_, _| {
            returns(!first.borrow().is_empty() as u32)
        });
        e.register_double(BYTE_MAP_NEXT, move |e, a| {
            let (key, value) = queue.borrow_mut().pop_front().unwrap();
            e.mem.set_u8(a[2], key);
            e.mem.set_u32(a[3], value);
            e.mem.set_u32(a[1], !queue.borrow().is_empty() as u32);
            Ret::default()
        });
    }

    type Table = Rc<RefCell<Vec<(u32, u32)>>>;

    /// A key-value store behind `GetAt`, `SetAt` and `RemoveAt` of the
    /// given addresses.
    fn install_table(e: &mut Engine, get_at: u32, set_at: Option<u32>, remove_at: u32) -> Table {
        let table: Table = Rc::new(RefCell::new(vec![]));
        let t = table.clone();
        e.register_double(get_at, move |e, a| {
            let found = t.borrow().iter().find(|(k, _)| *k == a[1]).map(|kv| kv.1);
            if let Some(value) = found {
                e.mem.set_u32(a[2], value);
            }
            returns(found.is_some() as u32)
        });
        if let Some(set_at) = set_at {
            let t = table.clone();
            e.register_double(set_at, move |_, a| {
                t.borrow_mut().push((a[1], a[2]));
                Ret::default()
            });
        }
        let t = table.clone();
        e.register_double(remove_at, move |_, a| {
            let before = t.borrow().len();
            t.borrow_mut().retain(|(k, _)| *k != a[1]);
            returns((t.borrow().len() != before) as u32)
        });
        table
    }

    /// A form whose key is `key` (`+0x0C`).
    fn form_with_key(e: &mut Engine, key: u32) -> Ptr {
        let form = Ptr::new(e.mem.alloc(0x40));
        e.mem.set_u32(form.addr() + 0xc, key);
        form
    }

    fn change_data(e: &mut Engine, flags: u32, buffer: u32) -> Ptr<ChangeData> {
        let data: Ptr<ChangeData> = e.new_object();
        e.set(data, ChangeData::iFlags, flags);
        e.set(data, ChangeData::pBuffer, Ptr::new(buffer));
        data
    }

    #[test]
    fn new_item_asks_the_allocator_subobject() {
        let mut e = game();
        constant(&mut e, MAP_ALLOCATOR_NEW_ITEM, 0x1234);
        let map: Ptr<ChangesMap> = e.new_object();
        let item = ni_t_pointer_map_unsigned_int_change_data_p_new_item(&mut e, map);
        assert_eq!(item.addr(), 0x1234);
        assert_eq!(
            calls_to(&e, MAP_ALLOCATOR_NEW_ITEM),
            vec![vec![map.addr() + 0xc]]
        );
    }

    #[test]
    fn delete_item_clears_the_value_and_gives_the_entry_back() {
        let mut e = game();
        stub(&mut e, MAP_ALLOCATOR_DELETE_ITEM);
        let map: Ptr<ChangesMap> = e.new_object();
        let item = e.mem.alloc(12);
        e.mem.set_u32(item + 8, 0x77);
        ni_t_pointer_map_unsigned_int_change_data_p_delete_item(&mut e, map, Ptr::new(item));
        assert_eq!(e.mem.u32(item + 8), 0);
        assert_eq!(
            calls_to(&e, MAP_ALLOCATOR_DELETE_ITEM),
            vec![vec![map.addr() + 0xc, item]]
        );
    }

    #[test]
    fn change_data_destructor_frees_only_a_buffer() {
        let mut e = game();
        let buffer = e.mem.alloc(16);
        let with_buffer = change_data(&mut e, 1, buffer);
        fn_00854e10(&mut e, with_buffer);
        assert!(freed(&e, buffer));
        let without = change_data(&mut e, 1, 0);
        fn_00854e10(&mut e, without);
        assert_eq!(calls_to(&e, OPERATOR_DELETE), vec![vec![buffer]]);
    }

    #[test]
    fn add_flags_only_without_a_buffer() {
        let mut e = game();
        let plain = change_data(&mut e, 0b0101, 0);
        fn_00854e40(&mut e, plain, 0b0010);
        assert_eq!(e.get(plain, ChangeData::iFlags), 0b0111);
        let buffered = change_data(&mut e, 0b0101, 0x1000);
        fn_00854e40(&mut e, buffered, 0b0010);
        assert_eq!(e.get(buffered, ChangeData::iFlags), 0b0101);
    }

    #[test]
    fn remove_flags_only_without_a_buffer() {
        let mut e = game();
        let plain = change_data(&mut e, 0b0111, 0);
        fn_00854e70(&mut e, plain, 0b0010);
        assert_eq!(e.get(plain, ChangeData::iFlags), 0b0101);
        let buffered = change_data(&mut e, 0b0111, 0x1000);
        fn_00854e70(&mut e, buffered, 0b0010);
        assert_eq!(e.get(buffered, ChangeData::iFlags), 0b0111);
    }

    #[test]
    fn changes_map_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = game();
        install_entries(&mut e, &[]);
        stub(&mut e, MAP_REMOVE_ALL);
        stub(&mut e, CHANGES_MAP_BASE_DESTRUCT);
        let kept: Ptr<ChangesMap> = e.new_object();
        let result = changes_map_scalar_deleting_destructor(&mut e, kept, 0);
        assert_eq!(result, kept);
        assert!(!freed(&e, kept.addr()));
        assert_eq!(e.mem.u32(kept.addr()), CHANGES_MAP_VTABLE);
        let deleted: Ptr<ChangesMap> = e.new_object();
        changes_map_scalar_deleting_destructor(&mut e, deleted, 1);
        assert!(freed(&e, deleted.addr()));
        assert_eq!(calls_to(&e, CHANGES_MAP_BASE_DESTRUCT).len(), 2);
    }

    #[test]
    fn changes_map_destructor_removes_the_changes_then_the_base() {
        let mut e = game();
        let data = change_data(&mut e, 1, 0);
        install_entries(&mut e, &[(0x100, data.addr())]);
        stub(&mut e, MAP_REMOVE_ALL);
        stub(&mut e, CHANGES_MAP_BASE_DESTRUCT);
        let map: Ptr<ChangesMap> = e.new_object();
        fn_00854f00(&mut e, map);
        assert_eq!(e.mem.u32(map.addr()), CHANGES_MAP_VTABLE);
        assert!(freed(&e, data.addr()));
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(a, _)| *a)
            .filter(|a| *a == MAP_REMOVE_ALL || *a == CHANGES_MAP_BASE_DESTRUCT)
            .collect();
        assert_eq!(order, vec![MAP_REMOVE_ALL, CHANGES_MAP_BASE_DESTRUCT]);
    }

    #[test]
    fn remove_all_changes_deletes_each_change_data_and_its_buffer() {
        let mut e = game();
        let buffer = e.mem.alloc(8);
        let with_buffer = change_data(&mut e, 1, buffer);
        let plain = change_data(&mut e, 2, 0);
        install_entries(
            &mut e,
            &[
                (0x100, with_buffer.addr()),
                (0x200, 0),
                (0x300, plain.addr()),
            ],
        );
        stub(&mut e, MAP_REMOVE_ALL);
        let map: Ptr<ChangesMap> = e.new_object();
        changes_map_remove_all_changes(&mut e, map);
        assert!(freed(&e, buffer));
        assert!(freed(&e, with_buffer.addr()));
        assert!(freed(&e, plain.addr()));
        assert_eq!(calls_to(&e, MAP_REMOVE_ALL), vec![vec![map.addr()]]);
    }

    #[test]
    fn change_data_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = game();
        let buffer = e.mem.alloc(8);
        let data = change_data(&mut e, 1, buffer);
        assert_eq!(fn_00854fe0(&mut e, data, 0), data);
        assert!(freed(&e, buffer));
        assert!(!freed(&e, data.addr()));
        let other = change_data(&mut e, 1, 0);
        fn_00854fe0(&mut e, other, 1);
        assert!(freed(&e, other.addr()));
    }

    #[test]
    fn get_or_create_adds_flags_to_the_existing_change_data() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, Some(CHANGES_MAP_SET_AT), MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let form = form_with_key(&mut e, 0x77);
        let existing = change_data(&mut e, 0b01, 0);
        table.borrow_mut().push((0x77, existing.addr()));
        let result = fn_00855010(&mut e, map, form, 0b10);
        assert_eq!(result, existing);
        assert_eq!(e.get(existing, ChangeData::iFlags), 0b11);
        assert!(calls_to(&e, SCOPE_ENTER).is_empty());
    }

    #[test]
    fn get_or_create_makes_and_stores_a_new_change_data() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, Some(CHANGES_MAP_SET_AT), MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let form = form_with_key(&mut e, 0x78);
        let created = fn_00855010(&mut e, map, form, 0b100);
        assert!(!created.is_null());
        assert_eq!(e.get(created, ChangeData::iFlags), 0b100);
        assert_eq!(table.borrow().as_slice(), &[(0x78, created.addr())]);
        // The allocation scope: (0x11, 1, file, line 0x104).
        let scope = calls_to(&e, SCOPE_ENTER);
        assert_eq!(scope.len(), 1);
        assert_eq!(&scope[0][1..], &[0x11, 1, SOURCE_FILE, 0x104]);
        assert_eq!(calls_to(&e, SCOPE_LEAVE).len(), 1);
    }

    #[test]
    fn lookup_by_key_returns_the_stored_change_data_or_null() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, None, MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let data = change_data(&mut e, 1, 0);
        table.borrow_mut().push((5, data.addr()));
        assert_eq!(fn_00855100(&mut e, map, 5), data);
        assert!(fn_00855100(&mut e, map, 6).is_null());
    }

    #[test]
    fn lookup_by_form_uses_the_forms_key() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, None, MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let data = change_data(&mut e, 1, 0);
        table.borrow_mut().push((0x1234, data.addr()));
        let form = form_with_key(&mut e, 0x1234);
        assert_eq!(fn_00855130(&mut e, map, form), data);
        let other = form_with_key(&mut e, 0x4321);
        assert!(fn_00855130(&mut e, map, other).is_null());
    }

    #[test]
    fn clear_flags_keeps_the_change_data_while_flags_remain() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, None, MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let form = form_with_key(&mut e, 9);
        let data = change_data(&mut e, 0b11, 0);
        table.borrow_mut().push((9, data.addr()));
        assert!(fn_00855150(&mut e, map, form, 0b01));
        assert_eq!(e.get(data, ChangeData::iFlags), 0b10);
        assert_eq!(table.borrow().len(), 1);
        assert!(!freed(&e, data.addr()));
    }

    #[test]
    fn clear_flags_removes_the_change_data_once_empty() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, None, MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let form = form_with_key(&mut e, 9);
        let data = change_data(&mut e, 0b11, 0);
        table.borrow_mut().push((9, data.addr()));
        assert!(fn_00855150(&mut e, map, form, 0b11));
        assert!(table.borrow().is_empty());
        assert!(freed(&e, data.addr()));
        // No change data, or saving unavailable: false.
        assert!(!fn_00855150(&mut e, map, form, 1));
        constant(&mut e, SAVE_LOAD_UNAVAILABLE, 1);
        assert!(!fn_00855150(&mut e, map, form, 1));
    }

    #[test]
    fn drop_by_form_passes_the_forms_key_and_the_flag() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, None, MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let form = form_with_key(&mut e, 0x55);
        let buffer = e.mem.alloc(8);
        let data = change_data(&mut e, 1, buffer);
        table.borrow_mut().push((0x55, data.addr()));
        // A buffered change data stays unless forced.
        assert!(fn_008551f0(&mut e, map, form, 0));
        assert_eq!(table.borrow().len(), 1);
        assert!(!freed(&e, data.addr()));
        assert!(fn_008551f0(&mut e, map, form, 1));
        assert!(table.borrow().is_empty());
        assert!(freed(&e, data.addr()));
        assert!(freed(&e, buffer));
        assert_eq!(calls_to(&e, MAP_REMOVE_AT), vec![vec![map.addr(), 0x55]]);
    }

    #[test]
    fn drop_by_key_removes_unbuffered_data_and_false_when_missing() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, None, MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let data = change_data(&mut e, 1, 0);
        table.borrow_mut().push((3, data.addr()));
        assert!(fn_00855220(&mut e, map, 3, 0));
        assert!(table.borrow().is_empty());
        assert!(freed(&e, data.addr()));
        assert!(!fn_00855220(&mut e, map, 3, 0));
        constant(&mut e, SAVE_LOAD_UNAVAILABLE, 1);
        assert!(!fn_00855220(&mut e, map, 3, 1));
    }

    #[test]
    fn interior_map_constructor_sets_base_and_vtable() {
        let mut e = game();
        e.register(INTERIOR_MAP_BASE_CONSTRUCT, |_, a| returns(a[0]));
        let map: Ptr<InteriorCellNewReferencesMap> = e.new_object();
        assert_eq!(fn_008552b0(&mut e, map), map);
        assert_eq!(e.mem.u32(map.addr()), INTERIOR_MAP_VTABLE);
        assert_eq!(
            calls_to(&e, INTERIOR_MAP_BASE_CONSTRUCT),
            vec![vec![map.addr(), 0x25]]
        );
    }

    #[test]
    fn interior_map_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = game();
        install_entries(&mut e, &[]);
        stub(&mut e, MAP_REMOVE_ALL);
        stub(&mut e, INTERIOR_MAP_BASE_DESTRUCT);
        let map: Ptr<InteriorCellNewReferencesMap> = e.new_object();
        interior_cell_new_references_map_scalar_deleting_destructor(&mut e, map, 0);
        assert!(!freed(&e, map.addr()));
        interior_cell_new_references_map_scalar_deleting_destructor(&mut e, map, 1);
        assert!(freed(&e, map.addr()));
    }

    #[test]
    fn interior_map_destructor_deletes_each_list() {
        let mut e = game();
        let list = e.mem.alloc(8);
        install_entries(&mut e, &[(1, list), (2, 0)]);
        stub(&mut e, LIST_REMOVE_ALL);
        e.register(LIST_SCALAR_DELETE, |e, a| {
            e.mem.free(a[0]);
            returns(a[0])
        });
        stub(&mut e, MAP_REMOVE_ALL);
        stub(&mut e, INTERIOR_MAP_BASE_DESTRUCT);
        let map: Ptr<InteriorCellNewReferencesMap> = e.new_object();
        fn_00855310(&mut e, map);
        assert_eq!(e.mem.u32(map.addr()), INTERIOR_MAP_VTABLE);
        assert!(freed(&e, list));
        assert_eq!(calls_to(&e, LIST_REMOVE_ALL), vec![vec![list]]);
        assert_eq!(calls_to(&e, INTERIOR_MAP_BASE_DESTRUCT).len(), 1);
    }

    #[test]
    fn exterior_map_constructor_sets_base_and_vtable() {
        let mut e = game();
        e.register(EXTERIOR_MAP_BASE_CONSTRUCT, |_, a| returns(a[0]));
        let map: Ptr<ExteriorCellNewReferencesMap> = e.new_object();
        assert_eq!(fn_008553e0(&mut e, map), map);
        assert_eq!(e.mem.u32(map.addr()), EXTERIOR_MAP_VTABLE);
        assert_eq!(
            calls_to(&e, EXTERIOR_MAP_BASE_CONSTRUCT),
            vec![vec![map.addr(), 0x25]]
        );
    }

    #[test]
    fn exterior_map_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = game();
        install_entries(&mut e, &[]);
        stub(&mut e, MAP_REMOVE_ALL);
        stub(&mut e, EXTERIOR_MAP_BASE_DESTRUCT);
        let map: Ptr<ExteriorCellNewReferencesMap> = e.new_object();
        exterior_cell_new_references_map_scalar_deleting_destructor(&mut e, map, 0);
        assert!(!freed(&e, map.addr()));
        exterior_cell_new_references_map_scalar_deleting_destructor(&mut e, map, 1);
        assert!(freed(&e, map.addr()));
    }

    #[test]
    fn exterior_map_destructor_frees_the_items_of_each_list() {
        let mut e = game();
        let (first, second) = (e.mem.alloc(12), e.mem.alloc(12));
        let list = list_of(&mut e, &[first, 0, second]);
        install_entries(&mut e, &[(1, list), (2, 0)]);
        stub(&mut e, LIST_REMOVE_ALL);
        e.register(LIST_SCALAR_DELETE, |_, a| returns(a[0]));
        stub(&mut e, MAP_REMOVE_ALL);
        stub(&mut e, EXTERIOR_MAP_BASE_DESTRUCT);
        let map: Ptr<ExteriorCellNewReferencesMap> = e.new_object();
        fn_00855440(&mut e, map);
        assert!(freed(&e, first));
        assert!(freed(&e, second));
        assert_eq!(calls_to(&e, LIST_REMOVE_ALL), vec![vec![list]]);
        assert_eq!(calls_to(&e, LIST_SCALAR_DELETE), vec![vec![list, 1]]);
        assert_eq!(e.mem.u32(map.addr()), EXTERIOR_MAP_VTABLE);
    }

    #[test]
    fn numeric_id_map_constructor_sets_base_and_vtable() {
        let mut e = game();
        e.register(NUMERIC_ID_MAP_BASE_CONSTRUCT, |_, a| returns(a[0]));
        let map: Ptr<NumericIDBufferMap> = e.new_object();
        assert_eq!(fn_00855550(&mut e, map), map);
        assert_eq!(e.mem.u32(map.addr()), NUMERIC_ID_MAP_VTABLE);
        assert_eq!(
            calls_to(&e, NUMERIC_ID_MAP_BASE_CONSTRUCT),
            vec![vec![map.addr(), 0x25]]
        );
    }

    #[test]
    fn numeric_id_map_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = game();
        install_entries(&mut e, &[]);
        stub(&mut e, MAP_REMOVE_ALL);
        stub(&mut e, NUMERIC_ID_MAP_BASE_DESTRUCT);
        let map: Ptr<NumericIDBufferMap> = e.new_object();
        numeric_id_buffer_map_scalar_deleting_destructor(&mut e, map, 0);
        assert!(!freed(&e, map.addr()));
        numeric_id_buffer_map_scalar_deleting_destructor(&mut e, map, 1);
        assert!(freed(&e, map.addr()));
    }

    #[test]
    fn numeric_id_map_destructor_frees_the_buffers() {
        let mut e = game();
        let buffer = e.mem.alloc(8);
        install_entries(&mut e, &[(1, buffer), (2, 0)]);
        stub(&mut e, MAP_REMOVE_ALL);
        stub(&mut e, NUMERIC_ID_MAP_BASE_DESTRUCT);
        let map: Ptr<NumericIDBufferMap> = e.new_object();
        fn_008555b0(&mut e, map);
        assert!(freed(&e, buffer));
        assert_eq!(e.mem.u32(map.addr()), NUMERIC_ID_MAP_VTABLE);
        assert_eq!(calls_to(&e, NUMERIC_ID_MAP_BASE_DESTRUCT).len(), 1);
    }

    #[test]
    fn save_stats_constructor_makes_the_map_and_the_list() {
        let mut e = game();
        e.register(STATS_MAP_CONSTRUCT, |_, a| returns(a[0]));
        let stats: Ptr<SaveStats> = e.new_object();
        assert_eq!(fn_00855660(&mut e, stats), stats);
        let map = e.get(stats, SaveStats::pStatsMap);
        let list = e.get(stats, SaveStats::pExtraStats);
        assert!(!map.is_null() && !list.is_null());
        assert_eq!(e.mem.block_size(map.addr()), Some(0x10));
        assert_eq!(e.mem.block_size(list.addr()), Some(8));
        assert_eq!(
            calls_to(&e, STATS_MAP_CONSTRUCT),
            vec![vec![map.addr(), 0x25]]
        );
    }

    #[test]
    fn save_stats_destructor_frees_headers_extra_stats_and_lists() {
        let mut e = game();
        let (header_a, header_b) = (e.mem.alloc(12), e.mem.alloc(12));
        let type_list = list_of(&mut e, &[header_a, header_b]);
        install_byte_entries(&mut e, &[(5, type_list)]);
        stub(&mut e, LIST_REMOVE_ALL);
        e.register(LIST_SCALAR_DELETE, |_, a| returns(a[0]));
        let map_destructor = 0x0200_1000;
        let vtable = 0x0200_0000;
        e.put_vtable(vtable, &[map_destructor]);
        stub(&mut e, map_destructor);
        let stats_map = e.mem.alloc(0x10);
        e.mem.set_u32(stats_map, vtable);
        let description = e.mem.alloc(8);
        let stat = e.mem.alloc(8);
        e.mem.set_u32(stat + 4, description);
        let extra_list = list_of(&mut e, &[stat, 0]);
        let stats: Ptr<SaveStats> = e.new_object();
        e.set(stats, SaveStats::pStatsMap, Ptr::new(stats_map));
        e.set(stats, SaveStats::pExtraStats, Ptr::new(extra_list));
        fn_00855730(&mut e, stats);
        assert!(freed(&e, header_a));
        assert!(freed(&e, header_b));
        assert!(freed(&e, description));
        assert!(freed(&e, stat));
        assert_eq!(
            calls_to(&e, LIST_SCALAR_DELETE),
            vec![vec![type_list, 1], vec![extra_list, 1]]
        );
        // The map's own destructor, vtable slot 0, with flag 1.
        assert_eq!(calls_to(&e, map_destructor), vec![vec![stats_map, 1]]);
    }

    #[test]
    fn add_extra_stat_copies_the_description_to_the_list_head() {
        let mut e = game();
        // The record is read while the list head call runs: the cell that
        // holds its address is freed afterwards.
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        e.register_double(LIST_ADD_HEAD, move |e, a| {
            let record = e.mem.u32(a[1]);
            sink.borrow_mut().push((a[0], record));
            Ret::default()
        });
        let list = e.mem.alloc(8);
        let stats: Ptr<SaveStats> = e.new_object();
        e.set(stats, SaveStats::pExtraStats, Ptr::new(list));
        let text = e.mem.alloc(16);
        e.mem.set_cstr(text, b"Animations");
        save_stats_add_extra_stat(&mut e, stats, 1234, Ptr::new(text));
        let seen = seen.borrow();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, list);
        let record: Ptr<ExtraStat> = Ptr::new(seen[0].1);
        assert_eq!(e.get(record, ExtraStat::iSize), 1234);
        let copy = e.get(record, ExtraStat::pDescription);
        assert_ne!(copy.addr(), text);
        assert_eq!(e.mem.cstr(copy.addr()), b"Animations");
        // The copy has room for the terminator.
        assert_eq!(e.mem.block_size(copy.addr()), Some(16));
        assert_eq!(
            &calls_to(&e, SCOPE_ENTER)[0][1..],
            &[0x11, 1, SOURCE_FILE, 0x267]
        );
        assert_eq!(calls_to(&e, SCOPE_LEAVE).len(), 1);
    }

    #[test]
    fn stats_record_copies_the_header_and_adds_the_size() {
        let mut e = game();
        install_table(
            &mut e,
            BYTE_MAP_GET_AT,
            Some(BYTE_MAP_SET_AT),
            MAP_REMOVE_AT,
        );
        stub(&mut e, LIST_INSERT);
        let map = e.mem.alloc(0x10);
        let stats: Ptr<SaveStats> = e.new_object();
        e.set(stats, SaveStats::pStatsMap, Ptr::new(map));
        let header: Ptr<SaveFormHeader> = e.new_object();
        e.set(header, SaveFormHeader::iFormID, 0xAABBCCDD);
        e.set(header, SaveFormHeader::cFormType, 0x2A);
        e.set(header, SaveFormHeader::iFlags, 0x11223344);
        e.set(header, SaveFormHeader::cVersion, 0x0F);
        fn_00855970(&mut e, stats, header, 0x0123);
        // The list insert receives the heap copy of the 12-byte header.
        let insert = calls_to(&e, LIST_INSERT);
        assert_eq!(insert.len(), 1);
        assert_eq!(
            e.mem.bytes(insert[0][1], 12),
            vec![0xDD, 0xCC, 0xBB, 0xAA, 0x2A, 0x44, 0x33, 0x22, 0x11, 0x0F, 0x23, 0x01]
        );
        // Two scopes: the one of this function (line 0x276), then the one of
        // the insert (line 0x286).
        let scopes = calls_to(&e, SCOPE_ENTER);
        assert_eq!(scopes[0][4], 0x276);
        assert_eq!(scopes[1][4], 0x286);
    }

    #[test]
    fn stats_insert_creates_the_type_list_when_missing() {
        let mut e = game();
        let get = install_table(
            &mut e,
            BYTE_MAP_GET_AT,
            Some(BYTE_MAP_SET_AT),
            MAP_REMOVE_AT,
        );
        stub(&mut e, LIST_INSERT);
        let map = e.mem.alloc(0x10);
        let stats: Ptr<SaveStats> = e.new_object();
        e.set(stats, SaveStats::pStatsMap, Ptr::new(map));
        let header: Ptr<LoadFormHeader> = e.new_object();
        e.set(header, LoadFormHeader::iFormID, 0x42);
        e.set(header, LoadFormHeader::cFormType, 9);
        e.set(header, LoadFormHeader::iSize, 77);
        fn_00855a20(&mut e, stats, header);
        // A list was created and stored under the type.
        let stored = get.borrow().clone();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, 9);
        let insert = calls_to(&e, LIST_INSERT);
        assert_eq!(insert.len(), 1);
        assert_eq!(insert[0][0], stored[0].1);
        assert_eq!(insert[0][2], STATS_COMPARATOR);
        // The inserted item is a copy of the header.
        let copy = insert[0][1];
        assert_ne!(copy, header.addr());
        assert_eq!(e.mem.u32(copy), 0x42);
        assert_eq!(e.mem.u16(copy + 0xa), 77);
        assert_eq!(
            &calls_to(&e, SCOPE_ENTER)[0][1..],
            &[0x11, 1, SOURCE_FILE, 0x286]
        );
    }

    #[test]
    fn stats_insert_reuses_the_existing_type_list() {
        let mut e = game();
        let table = install_table(
            &mut e,
            BYTE_MAP_GET_AT,
            Some(BYTE_MAP_SET_AT),
            MAP_REMOVE_AT,
        );
        stub(&mut e, LIST_INSERT);
        let list = e.mem.alloc(8);
        table.borrow_mut().push((9, list));
        let map = e.mem.alloc(0x10);
        let stats: Ptr<SaveStats> = e.new_object();
        e.set(stats, SaveStats::pStatsMap, Ptr::new(map));
        let header: Ptr<LoadFormHeader> = e.new_object();
        e.set(header, LoadFormHeader::cFormType, 9);
        fn_00855a20(&mut e, stats, header);
        assert_eq!(table.borrow().len(), 1);
        assert_eq!(calls_to(&e, LIST_INSERT)[0][0], list);
    }

    #[test]
    fn stats_comparator_orders_largest_first() {
        let mut e = game();
        let small: Ptr<LoadFormHeader> = e.new_object();
        let large: Ptr<LoadFormHeader> = e.new_object();
        let twin: Ptr<LoadFormHeader> = e.new_object();
        e.set(small, LoadFormHeader::iSize, 10);
        e.set(large, LoadFormHeader::iSize, 0xFFF0);
        e.set(twin, LoadFormHeader::iSize, 10);
        assert_eq!(fn_00855b60(&mut e, large, small), -1);
        assert_eq!(fn_00855b60(&mut e, small, large), 1);
        assert_eq!(fn_00855b60(&mut e, small, twin), 0);
    }

    /// The doubles `PrintStats` needs; returns the lines written to the file
    /// and the `FORMAT` calls (all argument words).
    fn print_engine() -> (Engine, Lines, Calls) {
        let mut e = game();
        e.map(0x0118_7000, 0x1000); // the form type names
        e.map(0x0108_0000, 0x1000); // the format strings
        e.map(0x0101_1000, 0x1000); // the empty string
        e.mem.set_cstr(0x0108_05c4, b"Extra Stats:\r\n\r\n");
        stub(&mut e, STRING_CAT);
        e.register(FILE_EXISTS, |_, _| returns(0));
        stub(&mut e, FILE_DELETE);
        stub(&mut e, FILE_OBJECT_CONSTRUCT);
        stub(&mut e, FILE_OBJECT_DESTRUCT);
        let lines = Rc::new(RefCell::new(Vec::new()));
        let formats = Rc::new(RefCell::new(Vec::new()));
        let sink = formats.clone();
        e.register_double(FORMAT, move |e, a| {
            sink.borrow_mut().push(a.to_vec());
            // The "text" is the format's address, so the written lines can
            // be told apart.
            e.mem.set_cstr(a[0], format!("{:08x}", a[2]).as_bytes());
            Ret::default()
        });
        let written = lines.clone();
        e.register_double(SYSTEM_FILE_DO_WRITE, move |e, a| {
            let text = e.mem.cstr(a[1]);
            assert_eq!(text.len() as u32, a[2]);
            written.borrow_mut().push(String::from_utf8(text).unwrap());
            returns(0)
        });
        stub(&mut e, BUILD_CHANGES_STRING);
        e.register(DYNAMIC_CAST, |_, _| returns(0));
        (e, lines, formats)
    }

    fn load_header(e: &mut Engine, id: u32, form_type: u8, size: u16, flags: u32) -> u32 {
        let header: Ptr<LoadFormHeader> = e.new_object();
        e.set(header, LoadFormHeader::iFormID, id);
        e.set(header, LoadFormHeader::cFormType, form_type);
        e.set(header, LoadFormHeader::iFlags, flags);
        e.set(header, LoadFormHeader::cVersion, 15);
        e.set(header, LoadFormHeader::iSize, size);
        header.addr()
    }

    /// A form object whose description (vtable slot `0x130`) is `text`.
    fn form_describing(e: &mut Engine, vtable: u32, text: &[u8]) -> u32 {
        let describe = vtable + 0x200;
        e.mem.map(vtable, 0x300);
        e.mem.set_u32(vtable + 0x130, describe);
        let name = e.mem.alloc(16);
        e.mem.set_cstr(name, text);
        constant(e, describe, name);
        let form = e.mem.alloc(0x40);
        e.mem.set_u32(form, vtable);
        form
    }

    #[test]
    fn print_stats_writes_the_sections_and_the_totals() {
        let (mut e, lines, formats) = print_engine();
        // Type 0x79 with two forms: 0x100 not loaded (size 10), 0x200 loaded
        // (size 30); one extra stat of 5.
        let first = load_header(&mut e, 0x100, 0x79, 10, 0xA);
        let second = load_header(&mut e, 0x200, 0x79, 30, 0xB);
        let type_list = list_of(&mut e, &[second, first]);
        install_byte_entries(&mut e, &[(0x79, type_list)]);
        let form = form_describing(&mut e, 0x0200_0000, b"Boone");
        e.register_double(LOOKUP_FORM, move |_, a| {
            returns(if a[0] == 0x200 { form } else { 0 })
        });
        let description = e.mem.alloc(8);
        e.mem.set_cstr(description, b"Textures");
        let stat = e.mem.alloc(8);
        e.mem.set_u32(stat, 5);
        e.mem.set_u32(stat + 4, description);
        let extra_list = list_of(&mut e, &[stat]);
        let stats_map = e.mem.alloc(0x10);
        let stats: Ptr<SaveStats> = e.new_object();
        e.set(stats, SaveStats::pStatsMap, Ptr::new(stats_map));
        e.set(stats, SaveStats::pExtraStats, Ptr::new(extra_list));
        let path = e.mem.alloc(16);
        e.mem.set_cstr(path, b"Saves\\x");

        save_stats_print_stats(&mut e, stats, Ptr::new(path));

        // The file object is built for the path with the extension.
        let construct = calls_to(&e, FILE_OBJECT_CONSTRUCT);
        assert_eq!(construct.len(), 1);
        assert_eq!(&construct[0][2..], &[1, 2, 0]);
        assert_eq!(calls_to(&e, FILE_OBJECT_DESTRUCT).len(), 1);
        assert!(calls_to(&e, FILE_DELETE).is_empty());

        let formats = formats.borrow();
        let used: Vec<u32> = formats.iter().map(|f| f[2]).collect();
        assert_eq!(
            used,
            vec![
                0x0108_0670, // table header
                0x0108_0668, // "Buffer"
                0x0108_03e4, // section heading
                0x0108_0648, // form 0x200 (largest first)
                0x0108_0648, // form 0x100
                0x0108_05d8, // section totals
                0x0108_0254, // extra stat
                0x0108_0570, // grand totals
            ]
        );
        // Rows: the loaded form prints its description, the other NOT LOADED.
        let row_loaded = &formats[3];
        assert_eq!(&row_loaded[3..7], &[0x200, 30, 0xB, 15]);
        assert_eq!(e.mem.cstr(row_loaded[7]), b"Boone");
        let row_missing = &formats[4];
        assert_eq!(&row_missing[3..8], &[0x100, 10, 0xA, 15, 0x0108_063c]);
        // Section totals: count 2, total 40, minimum 10, maximum 30, mean 20.
        let totals = &formats[5];
        let mean = f64::from_bits(totals[12] as u64 | (totals[13] as u64) << 32);
        assert_eq!(
            (totals[4], totals[6], totals[8], totals[10]),
            (2, 40, 10, 30)
        );
        assert_eq!(mean, 20.0);
        // Grand totals add the extra stat: 2 forms, 45 bytes, mean 22.5.
        let grand = &formats[7];
        let mean = f64::from_bits(grand[7] as u64 | (grand[8] as u64) << 32);
        assert_eq!(&grand[3..7], &[2, 45, 10, 30]);
        assert_eq!(mean, 22.5);
        // One line per format call except the type heading, plus the
        // "Extra Stats:" line.
        let lines = lines.borrow();
        assert_eq!(lines.len(), 8);
        assert_eq!(lines[0], format!("{:08x}", 0x0108_0670u32));
        assert_eq!(lines[5], "Extra Stats:\r\n\r\n");
    }

    #[test]
    fn print_stats_names_a_form_from_its_reference_marker_or_description() {
        let (mut e, _lines, formats) = print_engine();
        // Three loaded forms of type 0: A has a reference with a name, B a
        // map marker with a location, C a reference and a marker whose
        // names are empty, so its description is used.
        let ids = [0xA_u32, 0xB, 0xC];
        let headers: Vec<u32> = ids
            .iter()
            .map(|&id| load_header(&mut e, id, 0, 4, 1))
            .collect();
        let list = list_of(&mut e, &headers);
        install_byte_entries(&mut e, &[(0, list)]);
        let mut forms = Vec::new();
        for (i, text) in [b"descA", b"descB", b"descC"].iter().enumerate() {
            forms.push(form_describing(
                &mut e,
                0x0200_0000 + 0x1000 * i as u32,
                *text,
            ));
        }
        let table = forms.clone();
        e.register_double(LOOKUP_FORM, move |_, a| {
            returns(table[(a[0] - 0xA) as usize])
        });
        let strings: Vec<u32> = [b"RefName".as_slice(), b"Place", b""]
            .iter()
            .map(|text| {
                let block = e.mem.alloc(16);
                e.mem.set_cstr(block, text);
                block
            })
            .collect();
        e.mem.set_cstr(0x0101_1584, b"");
        // The casts: target 0x11841cc is the reference, 0x1183158 the marker.
        let (reference_cast, marker_cast) = (0x0200_5000, 0x0200_5004);
        let by_form = forms.clone();
        e.register_double(DYNAMIC_CAST, move |_, a| {
            let index = by_form.iter().position(|&f| f == a[0]).unwrap();
            let target = a[3];
            returns(match (index, target) {
                (0, 0x0118_41cc) | (2, 0x0118_41cc) => reference_cast + index as u32,
                (1, 0x0118_3158) | (2, 0x0118_3158) => marker_cast + index as u32,
                _ => 0,
            })
        });
        let (name_a, name_empty) = (strings[0], strings[2]);
        e.register_double(REFERENCE_GET_NAME, move |_, a| {
            returns(if a[0] == reference_cast {
                name_a
            } else {
                name_empty
            })
        });
        let (place, marker_empty) = (strings[1], strings[2]);
        e.register_double(MAP_MARKER_GET_LOCATION_NAME, move |_, a| {
            returns(if a[0] == marker_cast + 1 {
                place
            } else {
                marker_empty
            })
        });
        let stats_map = e.mem.alloc(0x10);
        let stats: Ptr<SaveStats> = e.new_object();
        e.set(stats, SaveStats::pStatsMap, Ptr::new(stats_map));
        let path = e.mem.alloc(8);
        e.mem.set_cstr(path, b"x");

        save_stats_print_stats(&mut e, stats, Ptr::new(path));

        let formats = formats.borrow();
        let rows: Vec<&Vec<u32>> = formats.iter().filter(|f| f[2] == 0x0108_0648).collect();
        assert_eq!(rows.len(), 3);
        let names: Vec<Vec<u8>> = rows.iter().map(|row| e.mem.cstr(row[7])).collect();
        assert_eq!(
            names,
            vec![b"RefName".to_vec(), b"Place".to_vec(), b"descC".to_vec()]
        );
        // The heading of type 0 is the text at `0104469c` ("Form").
        assert!(formats.iter().any(|f| f[2] == 0x0104_469c));
    }

    #[test]
    fn print_stats_titles_a_section_with_the_type_name() {
        let (mut e, _lines, formats) = print_engine();
        e.mem.set_u32(0x0118_7004 + 12 * 3, 0x0200_0100);
        let header = load_header(&mut e, 0x10, 3, 8, 0);
        let list = list_of(&mut e, &[header]);
        install_byte_entries(&mut e, &[(3, list)]);
        e.register(LOOKUP_FORM, |_, _| returns(0));
        let stats_map = e.mem.alloc(0x10);
        let stats: Ptr<SaveStats> = e.new_object();
        e.set(stats, SaveStats::pStatsMap, Ptr::new(stats_map));
        let path = e.mem.alloc(8);
        e.mem.set_cstr(path, b"x");
        save_stats_print_stats(&mut e, stats, Ptr::new(path));
        let heading = formats
            .borrow()
            .iter()
            .find(|f| f[2] == 0x0101_9f08)
            .cloned()
            .unwrap();
        assert_eq!(heading[3], 0x0200_0100);
    }

    #[test]
    fn print_stats_writes_nothing_when_the_file_will_not_open() {
        let (mut e, lines, formats) = print_engine();
        e.register(FILE_EXISTS, |_, _| returns(1));
        // The file object's first word is non-zero after construction.
        e.register(FILE_OBJECT_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 1);
            Ret::default()
        });
        let stats: Ptr<SaveStats> = e.new_object();
        let path = e.mem.alloc(8);
        e.mem.set_cstr(path, b"x");
        save_stats_print_stats(&mut e, stats, Ptr::new(path));
        assert!(formats.borrow().is_empty());
        assert!(lines.borrow().is_empty());
        // An existing file was deleted first, and the object was destroyed.
        assert_eq!(calls_to(&e, FILE_DELETE).len(), 1);
        assert_eq!(calls_to(&e, FILE_OBJECT_DESTRUCT).len(), 1);
    }

    #[test]
    fn stats_constructor_resets_the_counters() {
        let mut e = game();
        let stats: Ptr<Stats> = e.new_object();
        e.mem.write(stats.addr(), &[0x55; 12]);
        assert_eq!(fn_008562c0(&mut e, stats), stats);
        assert_eq!(e.get(stats, Stats::iNum), 0);
        assert_eq!(e.get(stats, Stats::iTotalSize), 0);
        assert_eq!(e.get(stats, Stats::iMinSize), 0xFFFF);
        assert_eq!(e.get(stats, Stats::iMaxSize), 0);
    }

    #[test]
    fn write_text_reports_success_of_the_file_write() {
        let mut e = game();
        e.register(SYSTEM_FILE_DO_WRITE, |_, a| returns(a[2] & 1));
        let file = e.mem.alloc(0x20);
        let text = e.mem.alloc(8);
        e.mem.set_cstr(text, b"ab");
        assert!(fn_00856300(
            &mut e,
            Ptr::NULL,
            Ptr::new(file),
            Ptr::new(text)
        ));
        e.mem.set_cstr(text, b"abc");
        assert!(!fn_00856300(
            &mut e,
            Ptr::NULL,
            Ptr::new(file),
            Ptr::new(text)
        ));
        // (file, text, length, 0, scratch)
        let writes = calls_to(&e, SYSTEM_FILE_DO_WRITE);
        assert_eq!(&writes[0][..4], &[file, text, 2, 0]);
    }

    #[test]
    fn remove_changes_skips_deleted_forms() {
        let mut e = game();
        let table = install_table(&mut e, MAP_GET_AT, None, MAP_REMOVE_AT);
        let map: Ptr<ChangesMap> = e.new_object();
        let game_object: Ptr<TESSaveLoadGame> = e.new_object();
        e.set(game_object, TESSaveLoadGame::m_pChanges, map);
        let form = form_with_key(&mut e, 0x31);
        e.register(FORM_IS_DELETED, |e, a| {
            returns((e.mem.u32(a[0] + 8) & 0x4000 != 0) as u32)
        });
        let data = change_data(&mut e, 1, 0);
        table.borrow_mut().push((0x31, data.addr()));
        e.mem.set_u32(form.addr() + 8, 0x4000);
        tes_save_load_game_remove_changes(&mut e, game_object, form, 1);
        assert_eq!(table.borrow().len(), 1);
        e.mem.set_u32(form.addr() + 8, 0);
        tes_save_load_game_remove_changes(&mut e, game_object, form, 1);
        assert!(table.borrow().is_empty());
    }

    // ---- the save routine ------------------------------------------------

    #[test]
    fn save_refuses_with_the_sad_message_unless_allowed() {
        let mut e = game();
        e.map(0x0107_f000, 0x1000);
        e.mem.set_cstr(AUTOSAVE_NAME, b"autosave");
        e.register(GET_SAVING_ALLOWED, |_, _| returns(0));
        constant(&mut e, GET_MESSAGE_QUEUE, 0x5000);
        stub(&mut e, SHOW_MESSAGE);
        let name = e.mem.alloc(16);
        e.mem.set_cstr(name, b"Quicksave");
        let game_object: Ptr<TESSaveLoadGame> = e.new_object();
        // A named save that is not an autosave, and an unnamed one, refuse.
        for name in [Ptr::new(name), Ptr::NULL] {
            assert!(!fn_00856ca0(&mut e, game_object, Ptr::NULL, name, false));
        }
        let message = calls_to(&e, SHOW_MESSAGE);
        assert_eq!(message.len(), 2);
        assert_eq!(&message[0][..4], &[0x5000, 0, SAD_ICON, 0]);
        assert_eq!(f32::from_bits(message[0][4]), 2.0);
        assert_eq!(message[0][5], 0);
        assert!(calls_to(&e, SAVE_LOCK_ENTER).is_empty());
        assert_eq!(
            calls_to(&e, GET_MESSAGE_QUEUE)[0],
            vec![MESSAGE_QUEUE_OBJECT]
        );
        assert_eq!(
            &calls_to(&e, SCOPE_ENTER)[0][1..],
            &[0x11, 1, SOURCE_FILE, 0x466]
        );
        assert_eq!(calls_to(&e, SCOPE_LEAVE).len(), 2);
    }

    /// What the full save needs: everything it calls, with doubles that log
    /// what the file receives.
    struct SaveRig {
        e: Engine,
        game: Ptr<TESSaveLoadGame>,
        stream: u32,
        writes: FileWrites,
        seeks: Rc<RefCell<Vec<(u32, u32)>>>,
    }

    fn save_rig() -> SaveRig {
        let mut e = game();
        constant(&mut e, GET_SAVING_ALLOWED, 1);
        stub(&mut e, SAVE_LOCK_ENTER);
        stub(&mut e, SAVE_LOCK_LEAVE);
        for step in [
            SAVE_PREPARE_A,
            SAVE_PREPARE_B,
            SAVE_PREPARE_C,
            SAVE_HEADER,
            SAVE_PLUGIN_LIST,
            SAVE_GLOBAL_DATA,
            SAVE_FINAL_DATA,
            SAVE_NUMERIC_ID_ARRAYS,
            SAVE_CLOSE_A,
            SAVE_CLOSE_B,
            FILE_FLUSH,
        ] {
            stub(&mut e, step);
        }
        constant(&mut e, CURRENT_VERSION, 7);
        let positions = Rc::new(RefCell::new(VecDeque::from(vec![100u32, 500])));
        e.register_double(FILE_POSITION, move |_, _| {
            returns(positions.borrow_mut().pop_front().unwrap())
        });
        let writes = Rc::new(RefCell::new(Vec::new()));
        let sink = writes.clone();
        e.register_double(WRITE_BYTES, move |e, a| {
            sink.borrow_mut().push((a[1], e.mem.bytes(a[2], a[3])));
            Ret::default()
        });
        // The stream: seek is slot 0x14, name slot 0x18.
        let vtable = 0x0200_0000;
        e.mem.map(vtable, 0x100);
        e.mem.set_u32(vtable + 0x14, 0x0200_1000);
        e.mem.set_u32(vtable + 0x18, 0x0200_1004);
        let stream = e.mem.alloc(0x20);
        e.mem.set_u32(stream, vtable);
        let seeks = Rc::new(RefCell::new(Vec::new()));
        let sink = seeks.clone();
        e.register_double(0x0200_1000, move |_, a| {
            sink.borrow_mut().push((a[1], a[2]));
            Ret::default()
        });
        let name = e.mem.alloc(16);
        e.mem.set_cstr(name, b"Saves\\save");
        constant(&mut e, 0x0200_1004, name);
        constant(&mut e, OPEN_SAVE_FILE, stream);
        e.set_global(SEEK_MODE, 0u32);
        let game_object: Ptr<TESSaveLoadGame> = e.new_object();
        SaveRig {
            e,
            game: game_object,
            stream,
            writes,
            seeks,
        }
    }

    #[test]
    fn save_writes_headers_sizes_and_patches_the_positions() {
        let mut rig = save_rig();
        let e = &mut rig.e;
        // Two changed forms: 0x100 (no buffer, a real form) and 0x200 (with
        // a pre-built buffer of 2 bytes "AB").
        let plain = change_data(e, 0x3, 0);
        let buffer = e.mem.alloc(8);
        e.mem.write(buffer, b"AB");
        let buffered = change_data(e, 0x9, buffer);
        let map: Ptr<ChangesMap> = e.new_object();
        e.set(rig.game, TESSaveLoadGame::m_pChanges, map);
        install_entries(e, &[(0x100, plain.addr()), (0x200, buffered.addr())]);
        // The form: vtable slot 0x50 changes size, 0x58 saves the changes.
        let form_vtable = 0x0200_2000;
        e.mem.map(form_vtable, 0x100);
        e.mem.set_u32(form_vtable + 0x50, 0x0200_3000);
        e.mem.set_u32(form_vtable + 0x58, 0x0200_3004);
        constant(e, 0x0200_3000, 4);
        stub(e, 0x0200_3004);
        let form = e.mem.alloc(0x40);
        e.mem.set_u32(form, form_vtable);
        e.register_double(LOOKUP_FORM, move |_, a| {
            returns(if a[0] == 0x100 { form } else { 0 })
        });
        constant(e, FORM_TYPE, 0x2B);
        e.register(CHECK_FLAGS, |_, a| returns(a[2] | 0x100));
        constant(e, GET_INITIAL_DATA_SAVE_SIZE, 3);
        let save_buffer = e.mem.alloc(16);
        constant(e, CREATE_BUFFER, save_buffer);
        stub(e, SAVE_INITIAL_DATA);
        stub(e, WRITE_FILE);
        stub(e, FREE_BUFFER);
        // The buffer's first four bytes: size 2, type 0x33, version 5.
        e.register(READ_BYTES, |e, a| {
            e.mem.write(a[1], &[2, 0, 0x33, 5]);
            Ret::default()
        });

        let saved = fn_00856ca0(e, rig.game, Ptr::NULL, Ptr::NULL, false);
        assert!(saved);

        let writes = rig.writes.borrow();
        let stream = rig.stream;
        let sizes: Vec<usize> = writes.iter().map(|(_, bytes)| bytes.len()).collect();
        // Two zero words, then for form 0x100: header, size, and for 0x200:
        // header, size, data; then the patched end position and count.
        assert_eq!(sizes, vec![4, 4, 10, 2, 10, 2, 2, 4, 4]);
        assert!(writes.iter().all(|(file, _)| *file == stream));
        assert_eq!(writes[0].1, vec![0; 4]);
        // Header of 0x100: id, type 0x2B, flags 0x3 | 0x100 (CheckFlags),
        // version 7.
        assert_eq!(
            writes[2].1,
            vec![0x00, 0x01, 0x00, 0x00, 0x2B, 0x03, 0x01, 0x00, 0x00, 0x07]
        );
        assert_eq!(writes[3].1, vec![7, 0]); // 4 + 3
                                             // Header of the buffered form: type and version from the buffer.
        assert_eq!(
            writes[4].1,
            vec![0x00, 0x02, 0x00, 0x00, 0x33, 0x09, 0x00, 0x00, 0x00, 0x05]
        );
        assert_eq!(writes[5].1, vec![2, 0]);
        assert_eq!(writes[6].1, b"AB".to_vec());
        // The end position (500) and the form count (2), after seeking back
        // to the start position (100).
        assert_eq!(writes[7].1, 500u32.to_le_bytes().to_vec());
        assert_eq!(writes[8].1, 2u32.to_le_bytes().to_vec());
        assert_eq!(rig.seeks.borrow().as_slice(), &[(100, 0)]);
        // The first form's data went through the buffer helpers.
        assert_eq!(
            calls_to(e, WRITE_FILE),
            vec![vec![rig.game.addr(), stream, save_buffer, 7]]
        );
        assert_eq!(
            calls_to(e, FREE_BUFFER),
            vec![vec![rig.game.addr(), save_buffer]]
        );
        // The routine bracketed itself with the lock and closed the file.
        assert_eq!(calls_to(e, SAVE_LOCK_ENTER).len(), 1);
        assert_eq!(calls_to(e, SAVE_LOCK_LEAVE).len(), 1);
        assert_eq!(calls_to(e, FILE_FLUSH), vec![vec![stream]]);
        assert_eq!(
            calls_to(e, SAVE_CLOSE_B),
            vec![vec![rig.game.addr(), stream, 0]]
        );
        // The form header pointer is cleared again, and the buffer too.
        assert!(e
            .get(rig.game, TESSaveLoadGame::m_pCurrentlySavingFormHeader)
            .is_null());
        assert!(e.get(rig.game, TESSaveLoadGame::m_pBuffer).is_null());
    }

    #[test]
    fn save_allows_an_autosave_when_saving_is_not_allowed() {
        let mut rig = save_rig();
        let e = &mut rig.e;
        constant(e, GET_SAVING_ALLOWED, 0);
        e.map(0x0107_f000, 0x1000);
        e.mem.set_cstr(AUTOSAVE_NAME, b"autosave");
        install_entries(e, &[]);
        let map: Ptr<ChangesMap> = e.new_object();
        e.set(rig.game, TESSaveLoadGame::m_pChanges, map);
        let name = e.mem.alloc(16);
        e.mem.set_cstr(name, b"autosave");
        stub(e, SHOW_MESSAGE);
        assert!(fn_00856ca0(e, rig.game, Ptr::NULL, Ptr::new(name), false));
        assert!(calls_to(e, SHOW_MESSAGE).is_empty());
        // No forms: the two zero words and the patched end and count.
        let sizes: Vec<usize> = rig.writes.borrow().iter().map(|w| w.1.len()).collect();
        assert_eq!(sizes, vec![4, 4, 4, 4]);
    }

    #[test]
    fn save_prints_and_deletes_the_statistics_when_asked() {
        let mut rig = save_rig();
        let e = &mut rig.e;
        install_entries(e, &[]);
        let map: Ptr<ChangesMap> = e.new_object();
        e.set(rig.game, TESSaveLoadGame::m_pChanges, map);
        // `SaveStats`'s own map: constructed with a destructor in slot 0.
        let map_vtable = 0x0200_4000;
        e.put_vtable(map_vtable, &[0x0200_4100]);
        stub(e, 0x0200_4100);
        e.register_double(STATS_MAP_CONSTRUCT, move |e, a| {
            e.mem.set_u32(a[0], map_vtable);
            returns(a[0])
        });
        // `PrintStats` runs (it is this unit's own): it builds the text file
        // object for the stream's name, which fails to open here.
        let printed = Rc::new(RefCell::new(Vec::new()));
        let sink = printed.clone();
        e.register_double(FILE_OBJECT_CONSTRUCT, move |e, a| {
            sink.borrow_mut().push(e.mem.cstr(a[1]));
            e.mem.set_u32(a[0], 1);
            Ret::default()
        });
        e.register(FILE_EXISTS, |_, _| returns(0));
        stub(e, STRING_CAT);
        stub(e, FILE_OBJECT_DESTRUCT);
        stub(e, LIST_REMOVE_ALL);
        stub(e, LIST_SCALAR_DELETE);
        let saved = fn_00856ca0(e, rig.game, Ptr::NULL, Ptr::NULL, true);
        assert!(saved);
        assert_eq!(printed.borrow().len(), 1);
        assert_eq!(printed.borrow()[0], b"Saves\\save");
        // The statistics were deleted (the map's destructor, flag 1) and
        // the pointer cleared.
        assert!(e.get(rig.game, TESSaveLoadGame::m_pSaveLoadStats).is_null());
        assert_eq!(calls_to(e, 0x0200_4100).len(), 1);
    }

    #[test]
    fn flush_calls_the_file_helper() {
        let mut e = game();
        stub(&mut e, FILE_FLUSH);
        fn_00857210(&mut e, Ptr::new(0x1234));
        assert_eq!(calls_to(&e, FILE_FLUSH), vec![vec![0x1234]]);
    }

    #[test]
    fn set_saving_form_header_stores_the_pointer() {
        let mut e = game();
        let game_object: Ptr<TESSaveLoadGame> = e.new_object();
        fn_00857230(&mut e, game_object, Ptr::new(0x4444));
        assert_eq!(
            e.get(game_object, TESSaveLoadGame::m_pCurrentlySavingFormHeader)
                .addr(),
            0x4444
        );
    }

    #[test]
    fn save_stats_scalar_deleting_destructor_frees_on_bit_zero() {
        let mut e = game();
        install_byte_entries(&mut e, &[]);
        let stats: Ptr<SaveStats> = e.new_object();
        assert_eq!(fn_00857250(&mut e, stats, 0), stats);
        assert!(!freed(&e, stats.addr()));
        fn_00857250(&mut e, stats, 1);
        assert!(freed(&e, stats.addr()));
    }

    #[test]
    fn form_and_flags_constructor_stores_the_four_fields() {
        let mut e = game();
        let record: Ptr<FormAndFlags> = e.new_object();
        let result = fn_00857280(&mut e, record, Ptr::new(0x1111), 5, 6, 9);
        assert_eq!(result, record);
        assert_eq!(e.get(record, FormAndFlags::pForm).addr(), 0x1111);
        assert_eq!(e.get(record, FormAndFlags::iFlags), 5);
        assert_eq!(e.get(record, FormAndFlags::iOldFlags), 6);
        assert_eq!(e.get(record, FormAndFlags::cVersion), 9);
    }

    #[test]
    fn local_record_initializer_clears_the_words_and_runs_the_embedded_one() {
        let mut e = game();
        stub(&mut e, EMBEDDED_RECORD_INIT);
        let local = e.mem.alloc(0x40);
        e.mem.write(local, &[0xFF; 16]);
        assert_eq!(fn_008572c0(&mut e, Ptr::new(local)).addr(), local);
        assert_eq!(e.mem.u32(local), 0);
        assert_eq!(e.mem.u32(local + 4), 0);
        assert_eq!(calls_to(&e, EMBEDDED_RECORD_INIT), vec![vec![local + 8]]);
    }

    #[test]
    fn every_function_is_registered_once() {
        let list = funcs();
        assert_eq!(list.len(), 40);
        let mut addresses: Vec<u32> = list.iter().map(|(a, _)| *a).collect();
        addresses.sort_unstable();
        addresses.dedup();
        assert_eq!(addresses.len(), 40);
    }
}
