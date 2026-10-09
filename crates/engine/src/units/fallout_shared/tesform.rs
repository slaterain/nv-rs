//! `fallout shared/tesform.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `TESForm` itself (vtable, `cFormType`, `iFormFlags`, `iFormID`, the source
//! file list) is [`crate::types::TESForm`]. The unit also owns the form
//! registries the constructor creates on first use and the destructor
//! destroys with the last form:
//!
//! - the form map (`FORM_MAP`, `NiTPointerMap<unsigned int, TESForm *>`):
//!   form ID to form;
//! - the editor ID map (`EDITOR_ID_MAP`): editor ID string to form;
//! - the form array (`FORM_LIST`, `BSSimpleArray`-shaped: items at +4, count
//!   at +0x0C) of the forms that carry flag 2 (`fn_00484730`).
//!
//! Done so far: the first 40 open functions of the unit, `00483370` to
//! `00484bc0`. The next session continues at `00484bf0`.
//!
//! The flag setters share two helpers, [`set_form_flag`] and
//! [`notify_changed`] (the form's virtual at +0x48 called with 1).
//!
//! Not translated: the compiler's exception-unwinding frames and SEH
//! registration in the functions that have them (`00483370`, `00483630`,
//! `00483720`, `00483870`, `00483a50`).

use crate::prelude::*;
use crate::types::TESForm;

// ---- globals of this unit ----
/// Number of live `TESForm` objects (the registries exist while it is not 0).
const FORM_COUNT: u32 = 0x011c_54bc;
/// Byte: the form-type string table has been checked and packed.
const FORM_TABLE_CHECKED: u32 = 0x011c_54b8;
/// Form ID to form map (pointer).
const FORM_MAP: u32 = 0x011c_54c0;
/// Editor ID to form map (pointer).
const EDITOR_ID_MAP: u32 = 0x011c_54c8;
/// Array of forms (pointer; items at +4, count at +0x0C).
const FORM_LIST: u32 = 0x011c_54c4;
/// The save writer's current record buffer (pointer) and its size in bytes.
const SAVE_BUFFER: u32 = 0x011c_54cc;
const SAVE_BUFFER_SIZE: u32 = 0x011c_54d0;
/// `TESDataHandler` instance (pointer).
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// `TESSaveLoadGame` instance (pointer).
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
/// `BGSSaveLoadGame` instance (pointer).
const BGS_SAVE_LOAD_GAME: u32 = 0x011d_df38;

/// The form-type table: 0x79 entries of 12 bytes, each a form type byte at
/// +0, the type's 4-character string (pointer) at +4 and that string packed
/// into a word at +8 (the constructor fills the packed word in).
const FORM_ENUM_TABLE: u32 = 0x0118_7000;
const FORM_ENUM_COUNT: u32 = 0x79;
const FORM_ENUM_STRIDE: u32 = 12;
/// The packed string of the table's third entry, the tag of a group record
/// (`fn_00484150` compares a record's first word against it).
const GROUP_TAG: u32 = 0x0118_7020;

/// `TESForm`'s vtable.
const TES_FORM_VTABLE: u32 = 0x0101_c794;
/// Messages: `"FORMS: formEnumString[ %d ].cFormID in TESForm.cpp is out of
/// order."`, the duplicate-string message, the fatal error text, the zlib
/// version `"1.2.1"` and the two zlib error messages.
const FORM_ENUM_ERROR_ORDER: u32 = 0x0101_c748;
const FORM_ENUM_ERROR_DUPLICATE: u32 = 0x0101_c6e0;
const FORM_ENUM_FIX_MESSAGE: u32 = 0x0101_c6a0;
const ZLIB_VERSION: u32 = 0x0101_a29c;
const ZLIB_INIT_ERROR: u32 = 0x0101_c8f4;
const ZLIB_DEFLATE_ERROR: u32 = 0x0101_c8cc;

// ---- callees outside this file ----
/// `BSSimpleList` constructor (empties the list).
const SIMPLE_LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// `BSSimpleList` destructor (frees every node).
const SIMPLE_LIST_DESTROY: u32 = 0x0047_0470;
/// Destructor wrapper of the same list (`0046ffb0` calls `00470470`).
const SIMPLE_LIST_DESTROY_WRAPPER: u32 = 0x0046_ffb0;
/// Base class constructor step: stores the base vtable (`00402d30`).
const BASE_CONSTRUCT: u32 = 0x0040_2d30;
/// `operator new(size)` and `operator delete(block)`.
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `memcpy(dst, src, size)` (the game's wrapper).
const MEMCPY: u32 = 0x0040_1460;
/// `Swap32(pointer, 0)`: byte-swaps the word at `pointer`.
const SWAP_WORD: u32 = 0x0040_1080;
/// The big-endian target flag (`MOV AL,[011c54ba]`).
const IS_BIG_ENDIAN: u32 = 0x0040_1500;
/// `FORM::Endian(header)`: swaps a record header in place.
const FORM_ENDIAN: u32 = 0x0047_0650;
/// `TESForm::cFormType` getter (`MOVZX EAX,byte [ECX+4]`).
const FORM_TYPE: u32 = 0x0040_1170;
/// Whether `iFormFlags & 0x4000` (temporary) is set.
const IS_TEMPORARY: u32 = 0x0040_77c0;
/// Whether `iFormFlags & 2` is set.
const IS_FLAG_2: u32 = 0x0046_0340;
/// Whether `iFormFlags & 0x20` (deleted) is set.
const IS_DELETED: u32 = 0x0044_0d80;
/// `MOV EAX,[ECX+0x0C]`: the form ID of a form, or the item count of the
/// form array.
const WORD_AT_0C: u32 = 0x0084_e3a0;
/// Address of the item at an index of the form array (`data + 4 * index`).
const LIST_ITEM_ADDRESS: u32 = 0x0087_7a30;
/// Form array: remove (clear) the item at an index.
const FORM_LIST_REMOVE_AT: u32 = 0x0048_6b80;
/// Form array: add an item (argument: address of the item pointer).
const FORM_LIST_ADD: u32 = 0x0047_0140;
/// Form array constructor (capacity, grow flag).
const FORM_LIST_CONSTRUCT: u32 = 0x0047_01d0;
/// Form array: clear (sets every item to null; `00863db0`).
const FORM_LIST_CLEAR: u32 = 0x0086_3db0;
/// Form map constructor (bucket count).
const FORM_MAP_CONSTRUCT: u32 = 0x0048_6930;
/// Editor ID map constructor (bucket count, copy-key flag).
const EDITOR_ID_MAP_CONSTRUCT: u32 = 0x0048_6ab0;
/// Map: remove every entry (`00438af0`).
const MAP_CLEAR: u32 = 0x0043_8af0;
/// Map: look up a key (`MAP_LOOKUP(map, key, &result)`, returns found).
const MAP_LOOKUP: u32 = 0x0085_3130;
/// Map: remove a key (`NiTMapBase::RemoveAt`).
const MAP_REMOVE_AT: u32 = 0x0040_5430;
/// Map: insert (`NiTMapBase::SetAt(key, value)`).
const MAP_SET_AT: u32 = 0x0084_4700;
/// `TESDataHandler::GetNextID`.
const DATA_HANDLER_GET_NEXT_ID: u32 = 0x0046_9800;
/// `TESDataHandler::RemoveIDFromDataHandler(id)`.
const DATA_HANDLER_REMOVE_ID: u32 = 0x0046_96f0;
/// Returns the byte at +0x620 of the data handler (`00482f20`).
const DATA_HANDLER_FLAG: u32 = 0x0048_2f20;
/// `TESSaveLoadGame::RemoveChanges(form, flags)` and `00856c70`'s partner.
const SAVE_LOAD_REMOVE_CHANGES: u32 = 0x0085_6c70;
const SAVE_LOAD_REMOVE_FORM: u32 = 0x0085_a410;
/// `BGSSaveLoadGame::RemoveChanges(form)`.
const BGS_REMOVE_CHANGES: u32 = 0x0084_a7c0;
/// `BGSSaveLoadGame` add change (form, change flags, force) and its sibling.
const BGS_ADD_CHANGE: u32 = 0x0084_a690;
const BGS_CHANGE_SIBLING: u32 = 0x0084_a780;
/// Stores its argument at `*this` (a one-word value's constructor).
const STORE_WORD: u32 = 0x008c_71b0;
/// `TESFile::StartForm(file, form)`, `TESFile::AddTESForm(file, form)` and
/// the call the file makes after a deleted form's start (`00473090`).
const FILE_START_FORM: u32 = 0x0047_2e60;
const FILE_ADD_FORM: u32 = 0x0047_2fe0;
const FILE_AFTER_START: u32 = 0x0047_3090;
/// `TESForm::SetFile(form, file)`, `TESForm::SetFormID(form, id, flag)` and
/// `TESForm::GetFormTypeFromFormString(string)` (all in this unit, later).
const SET_FILE: u32 = 0x0048_4f50;
const SET_FORM_ID: u32 = 0x0048_5c10;
const GET_FORM_TYPE_FROM_FORM_STRING: u32 = 0x0048_6890;
/// `FormComponentCollection` constructor and `InitFormComponentArray(form)`.
const COMPONENTS_CONSTRUCT: u32 = 0x0047_c050;
const COMPONENTS_INIT: u32 = 0x0047_c070;
/// Calls virtual slot 0 of every component, and virtual slot 1.
const COMPONENTS_VISIT_FIRST: u32 = 0x0047_c560;
const COMPONENTS_VISIT_SECOND: u32 = 0x0047_c5b0;
/// A `FormComponentCollection` is 0x26 pointers.
const COMPONENT_COLLECTION_SIZE: u32 = 0x98;
/// Clears a sub-object at +4 of the argument (`0048ce80`).
const CLEAR_SUB_OBJECT: u32 = 0x0048_ce80;
/// The save/load game stub that returns false (`0047c850`).
const SAVE_LOAD_STUB: u32 = 0x0047_c850;
/// Game messages: printf-style log (cdecl) and the fatal error.
const LOG_MESSAGE: u32 = 0x005b_5e40;
const FATAL_ERROR: u32 = 0x0040_fbe0;
const DISABLE_WARNING_COUNT: u32 = 0x0043_b2b0;
/// zlib 1.2.1: `deflateInit_(stream, level, version, stream_size)`,
/// `deflate(stream, flush)`, `deflateEnd(stream)`.
const DEFLATE_INIT: u32 = 0x00b4_6360;
const DEFLATE: u32 = 0x00b4_6b70;
const DEFLATE_END: u32 = 0x00b4_7370;

// `iFormFlags` bits set or cleared by the setters in this file that have a
// named role.
const FLAG_TEMPORARY: u32 = 0x4000;
const FLAG_DELETED: u32 = 0x20;

/// Sets or clears `mask` in the form's flags.
fn set_form_flag(e: &mut Engine, this: Ptr<TESForm>, mask: u32, on: bool) {
    let flags = e.get(this, TESForm::iFormFlags);
    let flags = if on { flags | mask } else { flags & !mask };
    e.set(this, TESForm::iFormFlags, flags);
}

/// The form's virtual at +0x48 with argument 1 (the "form changed" hook the
/// setters call after changing a flag that is saved).
fn notify_changed(e: &mut Engine, this: Ptr<TESForm>) {
    e.vcall(this.addr(), 0x48, &args![1u32]);
}

/// Sort key of a form type, as the compiler inlined it into several
/// functions: types `0x67`, `0x6c`, `0x73`, `0x74` rank `0x137` to `0x13a`,
/// every other type ranks `10 * type`.
fn form_type_rank(form_type: u32) -> u32 {
    match form_type {
        0x67 => 0x137,
        0x6c => 0x138,
        0x73 => 0x139,
        0x74 => 0x13a,
        other => other.wrapping_mul(10),
    }
}

/// The rank of the form's own type.
fn form_rank(e: &mut Engine, form: Ptr<TESForm>) -> u32 {
    let form_type = e.call(FORM_TYPE, &args![form]).u32();
    form_type_rank(form_type)
}

/// Index of `form` in the form array, if the array exists and holds it.
fn find_in_form_list(e: &mut Engine, form: Ptr<TESForm>) -> Option<u32> {
    let mut index = 0u32;
    loop {
        let list = e.global::<u32>(FORM_LIST);
        if list == 0 {
            return None;
        }
        let count = e.call(WORD_AT_0C, &args![list]).u32();
        if index >= count {
            return None;
        }
        let item = e.call(LIST_ITEM_ADDRESS, &args![list, index]).u32();
        if e.mem.u32(item) == form.addr() {
            return Some(index);
        }
        index += 1;
    }
}

// Translated from 00483370 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::TESForm` (Xbox PDB): sets up the base class and vtable, creates
/// the registries if this is the first form, checks and packs the form-type
/// string table once, gives the form flags 8 and, when the data handler
/// exists and its flag byte is clear, takes the next form ID (and the
/// handler's current file), then registers the form in the form map.
pub fn tes_form_tes_form(e: &mut Engine, this: Ptr<TESForm>) -> Ptr<TESForm> {
    e.call(BASE_CONSTRUCT, &args![this]);
    e.mem.set_u32(this.addr(), TES_FORM_VTABLE);
    e.call(SIMPLE_LIST_CONSTRUCT, &args![this.byte_add(0x10)]);

    if e.global::<u32>(FORM_COUNT) == 0 {
        fn_00483a50(e);
    }
    let count = e.global::<u32>(FORM_COUNT);
    e.set_global(FORM_COUNT, count.wrapping_add(1));

    if e.global::<u8>(FORM_TABLE_CHECKED) == 0 {
        check_form_enum_table(e);
        e.set_global(FORM_TABLE_CHECKED, 1u8);
    }

    e.set(this, TESForm::cFormType, 0);
    e.set(this, TESForm::iFormFlags, 8);
    e.set(this, TESForm::iFormID, 0);

    let handler = e.global::<u32>(DATA_HANDLER);
    if handler != 0 && !e.call(DATA_HANDLER_FLAG, &args![handler]).bool() {
        let id = e.call(DATA_HANDLER_GET_NEXT_ID, &args![handler]).u32();
        e.set(this, TESForm::iFormID, id);
        if fn_004835e0(e, handler) != 0 {
            let file = fn_004835e0(e, handler);
            e.call(SET_FILE, &args![this, file]);
        }
        if e.get(this, TESForm::iFormID) < 0x800 {
            e.call(SET_FORM_ID, &args![this, 0x800u32, 1u32]);
        }
    }

    let id = e.get(this, TESForm::iFormID);
    if id != 0 {
        let map = e.global::<u32>(FORM_MAP);
        e.call(MAP_SET_AT, &args![map, id, this]);
    }
    this
}

/// The constructor's one-time check of the form-type table: packs each type's
/// four characters (sign-extended bytes, first character lowest) into the
/// entry's word at +8, reports an entry whose type byte is not its index and
/// entries with equal packed strings, and stops the game if any was reported.
fn check_form_enum_table(e: &mut Engine) {
    let mut failed = false;
    e.call(DISABLE_WARNING_COUNT, &args![1u32]);
    for i in 0..FORM_ENUM_COUNT {
        let entry = FORM_ENUM_TABLE + i * FORM_ENUM_STRIDE;
        let text = e.mem.u32(entry + 4);
        let packed = (e.mem.i8(text) as i32)
            | (e.mem.i8(text + 1) as i32) << 8
            | (e.mem.i8(text + 2) as i32) << 16
            | (e.mem.i8(text + 3) as i32) << 24;
        e.mem.set_i32(entry + 8, packed);
        if e.mem.u8(entry) as u32 != i {
            e.call(LOG_MESSAGE, &args![FORM_ENUM_ERROR_ORDER, i]);
            failed = true;
        }
        for j in 0..FORM_ENUM_COUNT {
            if i != j
                && e.mem.u32(entry + 8) == e.mem.u32(FORM_ENUM_TABLE + j * FORM_ENUM_STRIDE + 8)
            {
                e.call(LOG_MESSAGE, &args![FORM_ENUM_ERROR_DUPLICATE, i, j, text]);
                failed = true;
            }
        }
    }
    e.call(DISABLE_WARNING_COUNT, &args![0u32]);
    if failed {
        e.call(FATAL_ERROR, &args![FORM_ENUM_FIX_MESSAGE]);
    }
}

// Translated from 004835e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A `TESDataHandler` getter: the word at +0x20C (the file the handler is
/// currently loading, passed to `SetFile` by the form constructor).
pub fn fn_004835e0(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this + 0x20c)
}

// Translated from 00483600 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::scalar deleting destructor`: runs the destructor and, when bit 0
/// of `flags` is set, frees the object.
pub fn tes_form_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<TESForm>,
    flags: u32,
) -> Ptr<TESForm> {
    tes_form_dtor(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 00483630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::~TESForm` (Xbox PDB): takes the form out of the registries and
/// the save/load change tables (unless it is temporary), destroys the source
/// file list and, with the last form, the registries.
pub fn tes_form_dtor(e: &mut Engine, this: Ptr<TESForm>) {
    e.mem.set_u32(this.addr(), TES_FORM_VTABLE);
    tes_form_remove_from_data_structures(e, this);
    e.call(SIMPLE_LIST_DESTROY, &args![this.byte_add(0x10)]);
    let save_load = e.global::<u32>(SAVE_LOAD_GAME);
    if !e.call(IS_TEMPORARY, &args![this]).bool() && save_load != 0 {
        e.call(SAVE_LOAD_REMOVE_CHANGES, &args![save_load, this, 0u32]);
        e.call(SAVE_LOAD_REMOVE_FORM, &args![save_load, this]);
    }
    let bgs = e.global::<u32>(BGS_SAVE_LOAD_GAME);
    if !e.call(IS_TEMPORARY, &args![this]).bool() && bgs != 0 {
        e.call(BGS_REMOVE_CHANGES, &args![bgs, this]);
    }
    let count = e.global::<u32>(FORM_COUNT).wrapping_sub(1);
    e.set_global(FORM_COUNT, count);
    if count == 0 {
        fn_00483b80(e);
    }
    e.call(SIMPLE_LIST_DESTROY_WRAPPER, &args![this.byte_add(0x10)]);
}

// Translated from 00483710 (decompiled, FalloutNV.exe 1.4.0.525)
/// Does nothing (an empty base-class destructor the compiler left a call to).
pub fn fn_00483710(_e: &mut Engine, _this: u32) {}

// Translated from 00483720 (decompiled, FalloutNV.exe 1.4.0.525)
/// Visits a form's components. Forms of types `0x11`, `0x3a` to `0x40`,
/// `0x42`, `0x43`, `0x49` and `0x69` have none; type `0x39` clears its
/// sub-object at +0x18; every other form builds its
/// `FormComponentCollection` on the stack and calls virtual slot 0 of each
/// component (`0047c560`). What the slot does is not named in the Xbox PDB.
pub fn fn_00483720(e: &mut Engine, this: Ptr<TESForm>) {
    let form_type = e.call(FORM_TYPE, &args![this]).u32();
    match form_type {
        0x11 | 0x3a..=0x40 | 0x42 | 0x43 | 0x49 | 0x69 => fn_00483710(e, this.addr()),
        0x39 => {
            e.call(CLEAR_SUB_OBJECT, &args![this.byte_add(0x18)]);
            fn_00483710(e, this.addr());
        }
        _ => e.with_stack(COMPONENT_COLLECTION_SIZE, |e, collection| {
            e.call(COMPONENTS_CONSTRUCT, &args![collection]);
            e.call(COMPONENTS_INIT, &args![collection, this]);
            e.call(COMPONENTS_VISIT_FIRST, &args![collection]);
            fn_00483710(e, collection.addr());
        }),
    }
}

// Translated from 00483870 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like [`fn_00483720`] but calls virtual slot 1 of each component
/// (`0047c5b0`), and for type `0x39` only calls the empty `00483710`.
pub fn fn_00483870(e: &mut Engine, this: Ptr<TESForm>) {
    let form_type = e.call(FORM_TYPE, &args![this]).u32();
    match form_type {
        0x11 | 0x3a..=0x40 | 0x42 | 0x43 | 0x49 | 0x69 => fn_00483710(e, this.addr()),
        0x39 => {
            fn_00483710(e, this.byte_add(0x18).addr());
            fn_00483710(e, this.addr());
        }
        _ => e.with_stack(COMPONENT_COLLECTION_SIZE, |e, collection| {
            e.call(COMPONENTS_CONSTRUCT, &args![collection]);
            e.call(COMPONENTS_INIT, &args![collection, this]);
            e.call(COMPONENTS_VISIT_SECOND, &args![collection]);
            fn_00483710(e, collection.addr());
        }),
    }
}

// Translated from 004839c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Looks a form up by form ID in the form map; null when there is no map or
/// no such form.
pub fn fn_004839c0(e: &mut Engine, form_id: u32) -> Ptr<TESForm> {
    let map = e.global::<u32>(FORM_MAP);
    if map == 0 {
        return Ptr::NULL;
    }
    e.with_stack(4, |e, result| {
        e.mem.set_u32(result.addr(), 0);
        if e.call(MAP_LOOKUP, &args![map, form_id, result]).bool() {
            Ptr::new(e.mem.u32(result.addr()))
        } else {
            Ptr::NULL
        }
    })
}

// Translated from 00483a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::GetFormByEditorID` (Xbox PDB): looks a form up by editor ID in
/// the editor ID map; null for a null or empty name, no map, or no match.
pub fn tes_form_get_form_by_editor_id(e: &mut Engine, editor_id: Ptr) -> Ptr<TESForm> {
    if editor_id.is_null() || e.mem.i8(editor_id.addr()) == 0 {
        return Ptr::NULL;
    }
    let map = e.global::<u32>(EDITOR_ID_MAP);
    if map == 0 {
        return Ptr::NULL;
    }
    e.with_stack(4, |e, result| {
        e.mem.set_u32(result.addr(), 0);
        if e.call(MAP_LOOKUP, &args![map, editor_id, result]).bool() {
            Ptr::new(e.mem.u32(result.addr()))
        } else {
            Ptr::NULL
        }
    })
}

// Translated from 00483a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Creates the form map (0x2008d buckets), the editor ID map (0xfa1 buckets)
/// and the form array (capacity 0x20), each only if it does not exist yet.
pub fn fn_00483a50(e: &mut Engine) {
    if e.global::<u32>(FORM_MAP) == 0 {
        let block = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
        let map = if block != 0 {
            e.call(FORM_MAP_CONSTRUCT, &args![block, 0x2008du32]).u32()
        } else {
            0
        };
        e.set_global(FORM_MAP, map);
    }
    if e.global::<u32>(EDITOR_ID_MAP) == 0 {
        let block = e.call(OPERATOR_NEW, &args![0x14u32]).u32();
        let map = if block != 0 {
            e.call(EDITOR_ID_MAP_CONSTRUCT, &args![block, 0xfa1u32, 1u32])
                .u32()
        } else {
            0
        };
        e.set_global(EDITOR_ID_MAP, map);
    }
    if e.global::<u32>(FORM_LIST) == 0 {
        let block = e.call(OPERATOR_NEW, &args![0x18u32]).u32();
        let list = if block != 0 {
            e.call(FORM_LIST_CONSTRUCT, &args![block, 0x20u32, 1u32])
                .u32()
        } else {
            0
        };
        e.set_global(FORM_LIST, list);
    }
}

// Translated from 00483b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys the form map, the editor ID map and the form array (clear, then
/// the object's scalar deleting destructor, virtual slot 0 with argument 1)
/// and nulls the globals.
pub fn fn_00483b80(e: &mut Engine) {
    for (global, clear) in [
        (FORM_MAP, MAP_CLEAR),
        (EDITOR_ID_MAP, MAP_CLEAR),
        (FORM_LIST, FORM_LIST_CLEAR),
    ] {
        let object = e.global::<u32>(global);
        if object != 0 {
            e.call(clear, &args![object]);
            let object = e.global::<u32>(global);
            if object != 0 {
                e.vcall(object, 0, &args![1u32]);
            }
            e.set_global(global, 0u32);
        }
    }
}

// Translated from 00483c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::RemoveFromDataStructures` (Xbox PDB): unless the form is
/// temporary, removes its ID from the form map, its entry from the form
/// array and its ID from the data handler.
pub fn tes_form_remove_from_data_structures(e: &mut Engine, this: Ptr<TESForm>) {
    if e.call(IS_TEMPORARY, &args![this]).bool() {
        return;
    }
    let map = e.global::<u32>(FORM_MAP);
    if map != 0 {
        let id = e.get(this, TESForm::iFormID);
        e.call(MAP_REMOVE_AT, &args![map, id]);
    }
    if let Some(index) = find_in_form_list(e, this) {
        let list = e.global::<u32>(FORM_LIST);
        e.call(FORM_LIST_REMOVE_AT, &args![list, index]);
    }
    let handler = e.global::<u32>(DATA_HANDLER);
    if handler != 0 {
        let id = e.get(this, TESForm::iFormID);
        e.call(DATA_HANDLER_REMOVE_ID, &args![handler, id]);
    }
}

// Translated from 00483d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::Save` (Xbox PDB): false for a temporary form; otherwise calls
/// virtual slot 0x2c and adds the form to the file, true when the file's
/// `AddTESForm` returns 0.
pub fn tes_form_save(e: &mut Engine, this: Ptr<TESForm>, file: Ptr) -> bool {
    if e.call(IS_TEMPORARY, &args![this]).bool() {
        return false;
    }
    e.vcall(this.addr(), 0x2c, &[]);
    e.call(FILE_ADD_FORM, &args![file, this]).u32() == 0
}

// Translated from 00483d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::CompressSaveBuffer` (Xbox PDB): compresses the save writer's
/// current record body with zlib. The buffer is a 0x18-byte record header
/// followed by the body; when it is larger than the header and not already
/// compressed (flag 0x40000 in the header's flags at +8), it is replaced by
/// a new buffer holding the header (flagged, size = compressed size + 4),
/// the uncompressed size as a word and the deflate stream. On a big-endian
/// target the header is swapped around the changes and the size word is
/// swapped. A zlib failure is logged and leaves the buffer as it was.
pub fn tes_form_compress_save_buffer(e: &mut Engine) {
    let buffer = e.global::<u32>(SAVE_BUFFER);
    let size = e.global::<u32>(SAVE_BUFFER_SIZE);
    if buffer == 0 || size <= 0x18 {
        return;
    }
    if e.call(IS_BIG_ENDIAN, &[]).bool() {
        e.call(FORM_ENDIAN, &args![buffer]);
    }
    if e.mem.u32(buffer + 8) & 0x40000 != 0 {
        if e.call(IS_BIG_ENDIAN, &[]).bool() {
            e.call(FORM_ENDIAN, &args![buffer]);
        }
        return;
    }
    let body_size = size - 0x18;
    let body = buffer + 0x18;
    e.with_stack(0x38, |e, stream| {
        let stream = stream.addr();
        // z_stream: next_in +0, avail_in +4, next_out +0x0c, avail_out +0x10,
        // state +0x1c, zalloc +0x20, zfree +0x24, opaque +0x28.
        e.mem.set_u32(stream + 0x20, 0);
        e.mem.set_u32(stream + 0x24, 0);
        e.mem.set_u32(stream + 0x28, 0);
        e.mem.set_u32(stream + 0x1c, 0);
        let status = e
            .call(
                DEFLATE_INIT,
                &args![stream, 0xffff_ffffu32, ZLIB_VERSION, 0x38u32],
            )
            .i32();
        if status != 0 {
            e.call(LOG_MESSAGE, &args![ZLIB_INIT_ERROR]);
            return;
        }
        let capacity = body_size << 1;
        let packed = e.call(OPERATOR_NEW, &args![capacity]).u32();
        e.mem.set_u32(stream + 4, body_size);
        e.mem.set_u32(stream, body);
        e.mem.set_u32(stream + 0x10, capacity);
        e.mem.set_u32(stream + 0x0c, packed);
        let status = e.call(DEFLATE, &args![stream, 4u32]).i32();
        if status == -2 {
            e.call(LOG_MESSAGE, &args![ZLIB_DEFLATE_ERROR]);
            e.call(OPERATOR_DELETE, &args![packed]);
            return;
        }
        let packed_size = capacity.wrapping_sub(e.mem.u32(stream + 0x10));
        let total = packed_size + 0x1c;
        let new_buffer = e.call(OPERATOR_NEW, &args![total]).u32();
        let flags = e.mem.u32(buffer + 8) | 0x40000;
        e.mem.set_u32(buffer + 8, flags);
        e.mem.set_u32(buffer + 4, total - 0x18);
        if e.call(IS_BIG_ENDIAN, &[]).bool() {
            e.call(FORM_ENDIAN, &args![buffer]);
        }
        e.call(MEMCPY, &args![new_buffer, buffer, 0x18u32]);
        let mut cursor = new_buffer + 0x18;
        e.with_stack(4, |e, original_size| {
            e.mem.set_u32(original_size.addr(), body_size);
            if e.call(IS_BIG_ENDIAN, &[]).bool() {
                e.call(SWAP_WORD, &args![original_size, 0u32]);
            }
            e.call(MEMCPY, &args![cursor, original_size, 4u32]);
        });
        cursor += 4;
        e.call(MEMCPY, &args![cursor, packed, packed_size]);
        let old_buffer = e.global::<u32>(SAVE_BUFFER);
        e.call(OPERATOR_DELETE, &args![old_buffer]);
        e.set_global(SAVE_BUFFER, new_buffer);
        e.set_global(SAVE_BUFFER_SIZE, total);
        e.call(DEFLATE_END, &args![stream]);
        e.call(OPERATOR_DELETE, &args![packed]);
    });
}

// Translated from 00483fc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the form to `file`: false for a temporary form; for a deleted form
/// starts the form record in the file (`TESFile::StartForm`), calls
/// `00473090` on the file and returns true; otherwise returns what virtual
/// slot 0x28 returns for `file`.
pub fn fn_00483fc0(e: &mut Engine, this: Ptr<TESForm>, file: Ptr) -> bool {
    if e.call(IS_TEMPORARY, &args![this]).bool() {
        return false;
    }
    if e.call(IS_DELETED, &args![this]).bool() {
        e.call(FILE_START_FORM, &args![file, this]);
        e.call(FILE_AFTER_START, &args![file]);
        return true;
    }
    e.vcall(this.addr(), 0x28, &args![file]).bool()
}

// Translated from 00484020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether this form's type ranks before `other`'s in the file order
/// ([`form_type_rank`]).
pub fn fn_00484020(e: &mut Engine, this: Ptr<TESForm>, other: Ptr<TESForm>) -> bool {
    let other_rank = form_rank(e, other);
    let this_rank = form_rank(e, this);
    this_rank < other_rank
}

// Translated from 00484150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether this form goes before the group record `group` (a null pointer or
/// a record whose first word is not the group tag gives false). The record's
/// word at +0x0C is the group type: 0 compares the form's rank with the rank
/// of the form type named by the label string at +8; types 1, 4 and 5 accept
/// ranks below 0x28a, types 2, 3 and 6 below 0x23a and type 7 below 0x2b2;
/// other group types give false.
pub fn fn_00484150(e: &mut Engine, this: Ptr<TESForm>, group: Ptr) -> bool {
    if group.is_null() || e.mem.u32(group.addr()) != e.global::<u32>(GROUP_TAG) {
        return false;
    }
    match e.mem.u32(group.addr() + 0xc) {
        0 => {
            let label = e.mem.u32(group.addr() + 8);
            let label_type = e.call(GET_FORM_TYPE_FROM_FORM_STRING, &args![label]).u32();
            let label_rank = form_type_rank(label_type);
            form_rank(e, this) < label_rank
        }
        1 | 4 | 5 => form_rank(e, this) < 0x28a,
        2 | 3 | 6 => form_rank(e, this) < 0x23a,
        7 => form_rank(e, this) < 0x2b2,
        _ => false,
    }
}

// Translated from 00484490 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::SetTemporary` (Xbox PDB): removes the form from the registries
/// and the save/load change tables, then sets the temporary flag (0x4000).
pub fn tes_form_set_temporary(e: &mut Engine, this: Ptr<TESForm>) {
    tes_form_remove_from_data_structures(e, this);
    let save_load = e.global::<u32>(SAVE_LOAD_GAME);
    if save_load != 0 {
        e.call(SAVE_LOAD_REMOVE_CHANGES, &args![save_load, this, 0u32]);
    }
    let bgs = e.global::<u32>(BGS_SAVE_LOAD_GAME);
    if bgs != 0 {
        e.call(BGS_REMOVE_CHANGES, &args![bgs, this]);
    }
    set_form_flag(e, this, FLAG_TEMPORARY, true);
}

// Translated from 004844f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x1 (no notification).
pub fn fn_004844f0(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x1, on);
}

// Translated from 00484530 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::SetDelete` (Xbox PDB): sets or clears the deleted flag (0x20)
/// and calls the changed hook.
pub fn tes_form_set_delete(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, FLAG_DELETED, on);
    notify_changed(e, this);
}

// Translated from 00484580 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::SetEmpty` (Xbox PDB): sets or clears flag 0x2000. Unless the
/// save/load stub (`0047c850`, which returns false) says otherwise, it first
/// calls virtual slot 0x48 (set) or 0x4c (clear) with 0x200000.
pub fn tes_form_set_empty(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    let save_load = e.global::<u32>(SAVE_LOAD_GAME);
    if !e.call(SAVE_LOAD_STUB, &args![save_load]).bool() {
        let slot = if on { 0x48 } else { 0x4c };
        e.vcall(this.addr(), slot, &args![0x20_0000u32]);
    }
    set_form_flag(e, this, 0x2000, on);
}

// Translated from 00484610 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x2000 (no notification).
pub fn fn_00484610(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x2000, on);
}

// Translated from 00484650 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x800000 and calls the changed hook.
pub fn fn_00484650(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x80_0000, on);
    notify_changed(e, this);
}

// Translated from 004846a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::SetDestructible` (Xbox PDB): sets or clears flag 0x1000000.
pub fn tes_form_set_destructible(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x100_0000, on);
}

// Translated from 004846e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x4000000 and calls the changed hook.
pub fn fn_004846e0(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x400_0000, on);
    notify_changed(e, this);
}

// Translated from 00484730 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 2 and keeps the form array in step. Clearing: if
/// the flag was set, the form's entry is removed from the form array. Setting:
/// if the flag was not set, the form is not temporary and the form array
/// exists, the form is added to it unless already listed.
pub fn fn_00484730(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    if on {
        if !e.call(IS_FLAG_2, &args![this]).bool()
            && !e.call(IS_TEMPORARY, &args![this]).bool()
            && e.global::<u32>(FORM_LIST) != 0
            && find_in_form_list(e, this).is_none()
        {
            let list = e.global::<u32>(FORM_LIST);
            e.with_stack(4, |e, slot| {
                e.mem.set_u32(slot.addr(), this.addr());
                e.call(FORM_LIST_ADD, &args![list, slot]);
            });
        }
        set_form_flag(e, this, 2, true);
    } else {
        if e.call(IS_FLAG_2, &args![this]).bool() && e.global::<u32>(FORM_LIST) != 0 {
            if let Some(index) = find_in_form_list(e, this) {
                let list = e.global::<u32>(FORM_LIST);
                e.call(FORM_LIST_REMOVE_AT, &args![list, index]);
            }
        }
        set_form_flag(e, this, 2, false);
    }
}

// Translated from 00484860 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x400 and calls the changed hook.
pub fn fn_00484860(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x400, on);
    notify_changed(e, this);
}

// Translated from 004848b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x40 and calls the changed hook.
pub fn fn_004848b0(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x40, on);
    notify_changed(e, this);
}

// Translated from 00484900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x10000 and calls the changed hook.
pub fn fn_00484900(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x1_0000, on);
    notify_changed(e, this);
}

// Translated from 00484950 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x20000 (no notification).
pub fn fn_00484950(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x2_0000, on);
}

// Translated from 00484990 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x100000 (no notification).
pub fn fn_00484990(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x10_0000, on);
}

// Translated from 004849d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x200 (no notification).
pub fn fn_004849d0(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x200, on);
}

// Translated from 00484a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Same as [`fn_004848b0`] (flag 0x40, changed hook); the compiler kept a
/// copy of the new flags in a local that the hook call does not use.
pub fn fn_00484a10(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x40, on);
    notify_changed(e, this);
}

// Translated from 00484a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::SetFireOff` (Xbox PDB): sets or clears flag 0x80.
pub fn tes_form_set_fire_off(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x80, on);
}

// Translated from 00484ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears form flag 0x8 (no notification).
pub fn fn_00484ab0(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x8, on);
}

// Translated from 00484af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::SetDisabled` (Xbox PDB): sets or clears flag 0x800 and calls the
/// changed hook.
pub fn tes_form_set_disabled(e: &mut Engine, this: Ptr<TESForm>, on: bool) {
    set_form_flag(e, this, 0x800, on);
    notify_changed(e, this);
}

// Translated from 00484b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether a form ID is in the range `1..=0x7ff` (cdecl, one argument).
pub fn fn_00484b40(_e: &mut Engine, form_id: u32) -> bool {
    form_id != 0 && form_id <= 0x7ff
}

/// The two words the change functions build on the stack: `00484b60` pushes
/// `extra`, makes room for a word, and has `008c71b0` store `changes` there.
fn change_words(e: &mut Engine, changes: u32, extra: u32) -> (u32, u32) {
    e.with_stack(8, |e, words| {
        e.mem.set_u32(words.addr() + 4, extra);
        e.call(STORE_WORD, &args![words, changes]);
        (e.mem.u32(words.addr()), e.mem.u32(words.addr() + 4))
    })
}

// Translated from 00484b60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Records a change of `changes` (change flags) on the form with the
/// save/load game's add-change function, with its last argument 0.
pub fn fn_00484b60(e: &mut Engine, this: Ptr<TESForm>, changes: u32) {
    let (changes, extra) = change_words(e, changes, 0);
    let bgs = e.global::<u32>(BGS_SAVE_LOAD_GAME);
    e.call(BGS_ADD_CHANGE, &args![bgs, this, changes, extra]);
}

// Translated from 00484b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESForm::ForceChange` (Xbox PDB): like [`fn_00484b60`] with the last
/// argument 1.
pub fn tes_form_force_change(e: &mut Engine, this: Ptr<TESForm>, changes: u32) {
    let (changes, extra) = change_words(e, changes, 1);
    let bgs = e.global::<u32>(BGS_SAVE_LOAD_GAME);
    e.call(BGS_ADD_CHANGE, &args![bgs, this, changes, extra]);
}

// Translated from 00484bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Passes the form and `changes` to the save/load game's sibling of the
/// add-change function (`0084a780`).
pub fn fn_00484bc0(e: &mut Engine, this: Ptr<TESForm>, changes: u32) {
    let value = e.with_stack(4, |e, word| {
        e.call(STORE_WORD, &args![word, changes]);
        e.mem.u32(word.addr())
    });
    let bgs = e.global::<u32>(BGS_SAVE_LOAD_GAME);
    e.call(BGS_CHANGE_SIBLING, &args![bgs, this, value]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00483370, tes_form_tes_form(Ptr<TESForm>) -> Ptr<TESForm>),
        entry!(0x004835e0, fn_004835e0(u32) -> u32),
        entry!(
            0x00483600,
            tes_form_scalar_deleting_destructor(Ptr<TESForm>, u32) -> Ptr<TESForm>
        ),
        entry!(0x00483630, tes_form_dtor(Ptr<TESForm>)),
        entry!(0x00483710, fn_00483710(u32)),
        entry!(0x00483720, fn_00483720(Ptr<TESForm>)),
        entry!(0x00483870, fn_00483870(Ptr<TESForm>)),
        entry!(0x004839c0, fn_004839c0(u32) -> Ptr<TESForm>),
        entry!(
            0x00483a00,
            tes_form_get_form_by_editor_id(Ptr) -> Ptr<TESForm>
        ),
        entry!(0x00483a50, fn_00483a50()),
        entry!(0x00483b80, fn_00483b80()),
        entry!(
            0x00483c70,
            tes_form_remove_from_data_structures(Ptr<TESForm>)
        ),
        entry!(0x00483d20, tes_form_save(Ptr<TESForm>, Ptr) -> bool),
        entry!(0x00483d70, tes_form_compress_save_buffer()),
        entry!(0x00483fc0, fn_00483fc0(Ptr<TESForm>, Ptr) -> bool),
        entry!(0x00484020, fn_00484020(Ptr<TESForm>, Ptr<TESForm>) -> bool),
        entry!(0x00484150, fn_00484150(Ptr<TESForm>, Ptr) -> bool),
        entry!(0x00484490, tes_form_set_temporary(Ptr<TESForm>)),
        entry!(0x004844f0, fn_004844f0(Ptr<TESForm>, bool)),
        entry!(0x00484530, tes_form_set_delete(Ptr<TESForm>, bool)),
        entry!(0x00484580, tes_form_set_empty(Ptr<TESForm>, bool)),
        entry!(0x00484610, fn_00484610(Ptr<TESForm>, bool)),
        entry!(0x00484650, fn_00484650(Ptr<TESForm>, bool)),
        entry!(0x004846a0, tes_form_set_destructible(Ptr<TESForm>, bool)),
        entry!(0x004846e0, fn_004846e0(Ptr<TESForm>, bool)),
        entry!(0x00484730, fn_00484730(Ptr<TESForm>, bool)),
        entry!(0x00484860, fn_00484860(Ptr<TESForm>, bool)),
        entry!(0x004848b0, fn_004848b0(Ptr<TESForm>, bool)),
        entry!(0x00484900, fn_00484900(Ptr<TESForm>, bool)),
        entry!(0x00484950, fn_00484950(Ptr<TESForm>, bool)),
        entry!(0x00484990, fn_00484990(Ptr<TESForm>, bool)),
        entry!(0x004849d0, fn_004849d0(Ptr<TESForm>, bool)),
        entry!(0x00484a10, fn_00484a10(Ptr<TESForm>, bool)),
        entry!(0x00484a70, tes_form_set_fire_off(Ptr<TESForm>, bool)),
        entry!(0x00484ab0, fn_00484ab0(Ptr<TESForm>, bool)),
        entry!(0x00484af0, tes_form_set_disabled(Ptr<TESForm>, bool)),
        entry!(0x00484b40, fn_00484b40(u32) -> bool),
        entry!(0x00484b60, fn_00484b60(Ptr<TESForm>, u32)),
        entry!(0x00484b90, tes_form_force_change(Ptr<TESForm>, u32)),
        entry!(0x00484bc0, fn_00484bc0(Ptr<TESForm>, u32)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test vtable for forms, and doubles at the addresses of its slots.
    const VTABLE: u32 = 0x0200_0000;
    const SLOT_DESTRUCTOR: u32 = 0x0f00_0000;
    const SLOT_0X28: u32 = 0x0f00_0028;
    const SLOT_0X2C: u32 = 0x0f00_002c;
    const SLOT_0X48: u32 = 0x0f00_0048;
    const SLOT_0X4C: u32 = 0x0f00_004c;
    const BIG_ENDIAN_FLAG: u32 = 0x011c_54ba;
    const GROUP_TAG_VALUE: u32 = 0x5055_5247;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// An engine with the globals' pages mapped and doubles for the trivial
    /// getters and the form's virtual slots.
    fn form_engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x011c_5000,
            0x011c_3000,
            0x011d_d000,
            0x011d_e000,
            0x0118_7000,
        ] {
            e.map(page, 0x1000);
        }
        let mut slots = vec![0u32; 20];
        slots[0] = SLOT_DESTRUCTOR;
        slots[0x28 / 4] = SLOT_0X28;
        slots[0x2c / 4] = SLOT_0X2C;
        slots[0x48 / 4] = SLOT_0X48;
        slots[0x4c / 4] = SLOT_0X4C;
        e.put_vtable(VTABLE, &slots);
        e.register(IS_TEMPORARY, |e, a| {
            ret((e.mem.u32(a[0] + 8) & 0x4000 != 0) as u32)
        });
        e.register(IS_FLAG_2, |e, a| ret((e.mem.u32(a[0] + 8) & 2 != 0) as u32));
        e.register(IS_DELETED, |e, a| {
            ret((e.mem.u32(a[0] + 8) & 0x20 != 0) as u32)
        });
        e.register(FORM_TYPE, |e, a| ret(e.mem.u8(a[0] + 4) as u32));
        e.register(WORD_AT_0C, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e.register(LIST_ITEM_ADDRESS, |e, a| {
            ret(e.mem.u32(a[0] + 4).wrapping_add(4 * a[1]))
        });
        e.register(IS_BIG_ENDIAN, |e, _| ret(e.mem.u8(BIG_ENDIAN_FLAG) as u32));
        for slot in [SLOT_DESTRUCTOR, SLOT_0X28, SLOT_0X2C, SLOT_0X48, SLOT_0X4C] {
            e.register(slot, |_, _| Ret::default());
        }
        e
    }

    /// A form with the test vtable, the given flags and type.
    fn form(e: &mut Engine, flags: u32, form_type: u8) -> Ptr<TESForm> {
        let form: Ptr<TESForm> = e.new_object();
        e.mem.set_u32(form.addr(), VTABLE);
        e.set(form, TESForm::iFormFlags, flags);
        e.set(form, TESForm::cFormType, form_type);
        e.set(form, TESForm::iFormID, 0x0100_0abc);
        form
    }

    /// The recorded calls to `addr`, with their argument words.
    fn calls_to(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// A form array holding the given forms (items at +4, count at +0x0C).
    fn form_array(e: &mut Engine, forms: &[Ptr<TESForm>]) -> u32 {
        let list = e.mem.alloc(0x18);
        let data = e.mem.alloc(4 * 8);
        e.mem.set_u32(list + 4, data);
        e.mem.set_u32(list + 0xc, forms.len() as u32);
        for (i, f) in forms.iter().enumerate() {
            e.mem.set_u32(data + 4 * i as u32, f.addr());
        }
        list
    }

    /// An object whose first word is the test vtable (slot 0 is the
    /// destructor double).
    fn object_with_vtable(e: &mut Engine) -> u32 {
        let object = e.mem.alloc(0x20);
        e.mem.set_u32(object, VTABLE);
        object
    }

    /// `form_engine` plus doubles for what the constructor calls.
    fn constructor_engine() -> Engine {
        let mut e = form_engine();
        e.register(BASE_CONSTRUCT, |_, _| Ret::default());
        e.register(SIMPLE_LIST_CONSTRUCT, |_, _| Ret::default());
        e.register(FORM_MAP_CONSTRUCT, |_, a| ret(a[0]));
        e.register(EDITOR_ID_MAP_CONSTRUCT, |_, a| ret(a[0]));
        e.register(FORM_LIST_CONSTRUCT, |_, a| ret(a[0]));
        e.register(MAP_SET_AT, |_, _| Ret::default());
        e.register(SET_FILE, |_, _| Ret::default());
        e.register(SET_FORM_ID, |_, _| Ret::default());
        e.register(DISABLE_WARNING_COUNT, |_, _| Ret::default());
        e.register(LOG_MESSAGE, |_, _| Ret::default());
        e.register(FATAL_ERROR, |_, _| Ret::default());
        e.register(DATA_HANDLER_FLAG, |e, a| ret(e.mem.u8(a[0] + 0x620) as u32));
        e.register(DATA_HANDLER_GET_NEXT_ID, |_, _| ret(0x10));
        e
    }

    #[test]
    fn constructor_creates_the_registries_with_the_first_form() {
        let mut e = constructor_engine();
        e.set_global(FORM_TABLE_CHECKED, 1u8);
        let first: Ptr<TESForm> = e.new_object();
        e.call_log = Some(vec![]);
        let back = e.call(0x0048_3370, &args![first]).ptr::<TESForm>();
        let log = e.call_log.take().unwrap();
        assert_eq!(back, first);
        assert_eq!(e.mem.u32(first.addr()), TES_FORM_VTABLE);
        assert_eq!(e.get(first, TESForm::iFormFlags), 8);
        assert_eq!(e.get(first, TESForm::iFormID), 0);
        assert_eq!(e.global::<u32>(FORM_COUNT), 1);
        assert_ne!(e.global::<u32>(FORM_MAP), 0);
        assert_ne!(e.global::<u32>(EDITOR_ID_MAP), 0);
        assert_ne!(e.global::<u32>(FORM_LIST), 0);
        // Without a form ID the form is not put in the map.
        assert!(calls_to(&log, MAP_SET_AT).is_empty());
        // A second form finds the registries and only counts itself.
        let map = e.global::<u32>(FORM_MAP);
        let second: Ptr<TESForm> = e.new_object();
        e.call(0x0048_3370, &args![second]);
        assert_eq!(e.global::<u32>(FORM_MAP), map);
        assert_eq!(e.global::<u32>(FORM_COUNT), 2);
    }

    #[test]
    fn constructor_takes_an_id_and_file_from_the_data_handler() {
        let mut e = constructor_engine();
        e.set_global(FORM_TABLE_CHECKED, 1u8);
        e.set_global(FORM_COUNT, 1u32);
        e.set_global(FORM_MAP, 0x7700_0000u32);
        let handler = e.mem.alloc(0x640);
        e.mem.set_u32(handler + 0x20c, 0x5555);
        e.set_global(DATA_HANDLER, handler);
        let form: Ptr<TESForm> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0048_3370, &args![form]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(form, TESForm::iFormID), 0x10);
        assert_eq!(calls_to(&log, SET_FILE), vec![vec![form.addr(), 0x5555]]);
        // The ID is below 0x800: the form is given the first free ID.
        assert_eq!(
            calls_to(&log, SET_FORM_ID),
            vec![vec![form.addr(), 0x800, 1]]
        );
        assert_eq!(
            calls_to(&log, MAP_SET_AT),
            vec![vec![0x7700_0000, 0x10, form.addr()]]
        );

        // A handler whose flag byte is set leaves the ID alone.
        e.mem.set_u8(handler + 0x620, 1);
        let other: Ptr<TESForm> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0048_3370, &args![other]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(other, TESForm::iFormID), 0);
        assert!(calls_to(&log, SET_FILE).is_empty());
        assert!(calls_to(&log, MAP_SET_AT).is_empty());
    }

    #[test]
    fn constructor_skips_set_file_without_a_current_file_and_keeps_high_ids() {
        let mut e = constructor_engine();
        e.set_global(FORM_TABLE_CHECKED, 1u8);
        e.set_global(FORM_COUNT, 1u32);
        e.set_global(FORM_MAP, 0x7700_0000u32);
        e.register(DATA_HANDLER_GET_NEXT_ID, |_, _| ret(0x0100_0000));
        let handler = e.mem.alloc(0x640);
        e.set_global(DATA_HANDLER, handler);
        let form: Ptr<TESForm> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0048_3370, &args![form]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, SET_FILE).is_empty());
        assert!(calls_to(&log, SET_FORM_ID).is_empty());
        assert_eq!(calls_to(&log, MAP_SET_AT).len(), 1);
    }

    /// Fills the form-type table with `0x79` entries: type byte = index and a
    /// distinct four-character string each.
    fn fill_form_enum_table(e: &mut Engine) {
        for i in 0..FORM_ENUM_COUNT {
            let entry = FORM_ENUM_TABLE + i * FORM_ENUM_STRIDE;
            let text = 0x0118_7800 + 4 * i;
            e.mem.write(
                text,
                &[b'T', b'A' + (i / 26) as u8, b'A' + (i % 26) as u8, b'_'],
            );
            e.mem.set_u8(entry, i as u8);
            e.mem.set_u32(entry + 4, text);
        }
    }

    #[test]
    fn constructor_packs_the_form_type_table_once() {
        let mut e = constructor_engine();
        fill_form_enum_table(&mut e);
        // A string with a byte above 0x7f packs sign-extended.
        e.mem.set_u8(0x0118_7800 + 4 * 10, 0xe9);
        let form: Ptr<TESForm> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0048_3370, &args![form]);
        let log = e.call_log.take().unwrap();
        let packed = |e: &Engine, i: u32| e.mem.u32(FORM_ENUM_TABLE + i * FORM_ENUM_STRIDE + 8);
        assert_eq!(packed(&e, 0), u32::from_le_bytes(*b"TAA_"));
        let expected = (-23i32) | (b'A' as i32) << 8 | (b'K' as i32) << 16 | (b'_' as i32) << 24;
        assert_eq!(packed(&e, 10), expected as u32);
        assert!(calls_to(&log, LOG_MESSAGE).is_empty());
        assert!(calls_to(&log, FATAL_ERROR).is_empty());
        assert_eq!(
            calls_to(&log, DISABLE_WARNING_COUNT),
            vec![vec![1], vec![0]]
        );
        assert_eq!(e.global::<u8>(FORM_TABLE_CHECKED), 1);

        // Already checked: the table is not looked at again.
        let again: Ptr<TESForm> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0048_3370, &args![again]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, DISABLE_WARNING_COUNT).is_empty());
    }

    #[test]
    fn constructor_reports_a_misnumbered_and_a_duplicated_form_type() {
        let mut e = constructor_engine();
        fill_form_enum_table(&mut e);
        // Entry 5 has the wrong type byte; entry 4 shares entry 3's string.
        e.mem.set_u8(FORM_ENUM_TABLE + 5 * FORM_ENUM_STRIDE, 7);
        let shared = e.mem.u32(FORM_ENUM_TABLE + 3 * FORM_ENUM_STRIDE + 4);
        e.mem
            .set_u32(FORM_ENUM_TABLE + 4 * FORM_ENUM_STRIDE + 4, shared);
        let form: Ptr<TESForm> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0048_3370, &args![form]);
        let log = e.call_log.take().unwrap();
        let messages = calls_to(&log, LOG_MESSAGE);
        assert!(messages.contains(&vec![FORM_ENUM_ERROR_ORDER, 5]));
        // Entry 3 is packed before entry 4, so the duplicate shows when entry
        // 4 is reached and compared with the already packed entry 3.
        assert!(messages.contains(&vec![FORM_ENUM_ERROR_DUPLICATE, 4, 3, shared]));
        assert_eq!(
            calls_to(&log, FATAL_ERROR),
            vec![vec![FORM_ENUM_FIX_MESSAGE]]
        );
        assert_eq!(e.global::<u8>(FORM_TABLE_CHECKED), 1);
    }

    #[test]
    fn data_handler_getter_reads_the_current_file() {
        let mut e = Engine::new();
        let handler = e.mem.alloc(0x640);
        e.mem.set_u32(handler + 0x20c, 0xabcd);
        assert_eq!(e.call(0x0048_35e0, &args![handler]).u32(), 0xabcd);
    }

    #[test]
    fn scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = form_engine();
        e.set_global(FORM_COUNT, 5u32);
        e.register(SIMPLE_LIST_DESTROY, |_, _| Ret::default());
        e.register(SIMPLE_LIST_DESTROY_WRAPPER, |_, _| Ret::default());
        let kept = form(&mut e, 8, 0x20);
        let back = e.call(0x0048_3600, &args![kept, 0u32]).ptr::<TESForm>();
        assert_eq!(back, kept);
        assert!(e.mem.block_size(kept.addr()).is_some());
        let freed = form(&mut e, 8, 0x20);
        let back = e.call(0x0048_3600, &args![freed, 1u32]).ptr::<TESForm>();
        assert_eq!(back, freed);
        assert_eq!(e.mem.block_size(freed.addr()), None);
        assert_eq!(e.global::<u32>(FORM_COUNT), 3);
    }

    #[test]
    fn destructor_unregisters_a_form_and_the_last_one_destroys_the_registries() {
        let mut e = form_engine();
        e.register(SIMPLE_LIST_DESTROY, |_, _| Ret::default());
        e.register(SIMPLE_LIST_DESTROY_WRAPPER, |_, _| Ret::default());
        e.register(SAVE_LOAD_REMOVE_CHANGES, |_, _| Ret::default());
        e.register(SAVE_LOAD_REMOVE_FORM, |_, _| Ret::default());
        e.register(BGS_REMOVE_CHANGES, |_, _| Ret::default());
        e.register(MAP_REMOVE_AT, |_, _| Ret::default());
        e.register(MAP_CLEAR, |_, _| Ret::default());
        e.register(FORM_LIST_CLEAR, |_, _| Ret::default());
        e.set_global(SAVE_LOAD_GAME, 0x6600_0000u32);
        e.set_global(BGS_SAVE_LOAD_GAME, 0x6700_0000u32);
        let map = object_with_vtable(&mut e);
        e.set_global(FORM_MAP, map);
        e.set_global(FORM_COUNT, 1u32);
        let victim = form(&mut e, 8, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_3630, &args![victim]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(victim.addr()), TES_FORM_VTABLE);
        assert_eq!(
            calls_to(&log, SAVE_LOAD_REMOVE_CHANGES),
            vec![vec![0x6600_0000, victim.addr(), 0]]
        );
        assert_eq!(
            calls_to(&log, SAVE_LOAD_REMOVE_FORM),
            vec![vec![0x6600_0000, victim.addr()]]
        );
        assert_eq!(
            calls_to(&log, BGS_REMOVE_CHANGES),
            vec![vec![0x6700_0000, victim.addr()]]
        );
        assert_eq!(calls_to(&log, MAP_REMOVE_AT).len(), 1);
        assert_eq!(calls_to(&log, SIMPLE_LIST_DESTROY).len(), 1);
        assert_eq!(calls_to(&log, SIMPLE_LIST_DESTROY_WRAPPER).len(), 1);
        // The last form: the registries go (the map's destructor got 1).
        assert_eq!(e.global::<u32>(FORM_COUNT), 0);
        assert_eq!(e.global::<u32>(FORM_MAP), 0);
        assert_eq!(calls_to(&log, SLOT_DESTRUCTOR), vec![vec![map, 1]]);
    }

    #[test]
    fn destructor_leaves_a_temporary_form_out_of_the_change_tables() {
        let mut e = form_engine();
        e.register(SIMPLE_LIST_DESTROY, |_, _| Ret::default());
        e.register(SIMPLE_LIST_DESTROY_WRAPPER, |_, _| Ret::default());
        e.set_global(SAVE_LOAD_GAME, 0x6600_0000u32);
        e.set_global(BGS_SAVE_LOAD_GAME, 0x6700_0000u32);
        e.set_global(FORM_COUNT, 3u32);
        let temporary = form(&mut e, 0x4000, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_3630, &args![temporary]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, SAVE_LOAD_REMOVE_CHANGES).is_empty());
        assert!(calls_to(&log, BGS_REMOVE_CHANGES).is_empty());
        assert_eq!(e.global::<u32>(FORM_COUNT), 2);
    }

    #[test]
    fn empty_destructor_does_nothing() {
        let mut e = Engine::new();
        e.call_log = Some(vec![]);
        e.call(0x0048_3710, &args![0x1234u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    /// Doubles for the component collection calls.
    fn component_engine() -> Engine {
        let mut e = form_engine();
        for addr in [
            COMPONENTS_CONSTRUCT,
            COMPONENTS_INIT,
            COMPONENTS_VISIT_FIRST,
            COMPONENTS_VISIT_SECOND,
            CLEAR_SUB_OBJECT,
        ] {
            e.register(addr, |_, _| Ret::default());
        }
        e
    }

    #[test]
    fn component_visit_depends_on_the_form_type() {
        let mut e = component_engine();
        for form_type in [0x11u8, 0x3a, 0x40, 0x42, 0x43, 0x49, 0x69] {
            let f = form(&mut e, 8, form_type);
            e.call_log = Some(vec![]);
            e.call(0x0048_3720, &args![f]);
            assert_eq!(e.call_log.take().unwrap().len(), 2, "type {form_type:#x}");
        }
        let special = form(&mut e, 8, 0x39);
        e.call_log = Some(vec![]);
        e.call(0x0048_3720, &args![special]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, CLEAR_SUB_OBJECT),
            vec![vec![special.addr() + 0x18]]
        );
        let ordinary = form(&mut e, 8, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_3720, &args![ordinary]);
        let log = e.call_log.take().unwrap();
        let built = calls_to(&log, COMPONENTS_CONSTRUCT);
        assert_eq!(built.len(), 1);
        let collection = built[0][0];
        assert_eq!(
            calls_to(&log, COMPONENTS_INIT),
            vec![vec![collection, ordinary.addr()]]
        );
        assert_eq!(
            calls_to(&log, COMPONENTS_VISIT_FIRST),
            vec![vec![collection]]
        );
        assert!(calls_to(&log, COMPONENTS_VISIT_SECOND).is_empty());
        // Types just outside the groups take the ordinary path.
        for form_type in [0x10u8, 0x41, 0x6a] {
            let f = form(&mut e, 8, form_type);
            e.call_log = Some(vec![]);
            e.call(0x0048_3720, &args![f]);
            let log = e.call_log.take().unwrap();
            assert_eq!(calls_to(&log, COMPONENTS_CONSTRUCT).len(), 1);
        }
    }

    #[test]
    fn second_component_visit_uses_the_second_slot_and_clears_nothing() {
        let mut e = component_engine();
        let skipped = form(&mut e, 8, 0x3c);
        e.call_log = Some(vec![]);
        e.call(0x0048_3870, &args![skipped]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
        let special = form(&mut e, 8, 0x39);
        e.call_log = Some(vec![]);
        e.call(0x0048_3870, &args![special]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, CLEAR_SUB_OBJECT).is_empty());
        assert_eq!(log.len(), 2);
        let ordinary = form(&mut e, 8, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_3870, &args![ordinary]);
        let log = e.call_log.take().unwrap();
        let collection = calls_to(&log, COMPONENTS_CONSTRUCT)[0][0];
        assert_eq!(
            calls_to(&log, COMPONENTS_VISIT_SECOND),
            vec![vec![collection]]
        );
        assert!(calls_to(&log, COMPONENTS_VISIT_FIRST).is_empty());
    }

    #[test]
    fn form_lookup_by_id() {
        let mut e = form_engine();
        // No map: null without a lookup.
        e.call_log = Some(vec![]);
        assert!(e
            .call(0x0048_39c0, &args![0x1234u32])
            .ptr::<TESForm>()
            .is_null());
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        // A hit stores the form through the out pointer.
        e.set_global(FORM_MAP, 0x7700_0000u32);
        e.register(MAP_LOOKUP, |e, a| {
            e.mem.set_u32(a[2], 0xf0f0);
            ret(1)
        });
        e.call_log = Some(vec![]);
        let hit = e.call(0x0048_39c0, &args![0x1234u32]).ptr::<TESForm>();
        let log = e.call_log.take().unwrap();
        assert_eq!(hit.addr(), 0xf0f0);
        let lookups = calls_to(&log, MAP_LOOKUP);
        assert_eq!((lookups[0][0], lookups[0][1]), (0x7700_0000, 0x1234));
        // A miss gives null even when the lookup scribbled on the out word.
        e.register(MAP_LOOKUP, |e, a| {
            e.mem.set_u32(a[2], 0xf0f0);
            ret(0)
        });
        assert!(e
            .call(0x0048_39c0, &args![0x1234u32])
            .ptr::<TESForm>()
            .is_null());
    }

    #[test]
    fn form_lookup_by_editor_id() {
        let mut e = form_engine();
        let name = Ptr::<()>::new(e.mem.alloc(16));
        e.mem.set_cstr(name.addr(), b"MyForm");
        let empty = Ptr::<()>::new(e.mem.alloc(16));
        e.register(MAP_LOOKUP, |e, a| {
            e.mem.set_u32(a[2], 0xabc0);
            ret(1)
        });
        // No map yet.
        assert!(e.call(0x0048_3a00, &args![name]).ptr::<TESForm>().is_null());
        e.set_global(EDITOR_ID_MAP, 0x7800_0000u32);
        // Null and empty names never reach the map.
        e.call_log = Some(vec![]);
        assert!(e
            .call(0x0048_3a00, &args![Ptr::<()>::NULL])
            .ptr::<TESForm>()
            .is_null());
        assert!(e
            .call(0x0048_3a00, &args![empty])
            .ptr::<TESForm>()
            .is_null());
        assert!(calls_to(&e.call_log.take().unwrap(), MAP_LOOKUP).is_empty());
        // A hit.
        e.call_log = Some(vec![]);
        let hit = e.call(0x0048_3a00, &args![name]).ptr::<TESForm>();
        assert_eq!(hit.addr(), 0xabc0);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, MAP_LOOKUP)[0][..2],
            [0x7800_0000, name.addr()]
        );
        // A miss.
        e.register(MAP_LOOKUP, |_, _| ret(0));
        assert!(e.call(0x0048_3a00, &args![name]).ptr::<TESForm>().is_null());
    }

    #[test]
    fn registries_are_created_once() {
        let mut e = form_engine();
        e.register(FORM_MAP_CONSTRUCT, |_, a| ret(a[0]));
        e.register(EDITOR_ID_MAP_CONSTRUCT, |_, a| ret(a[0]));
        e.register(FORM_LIST_CONSTRUCT, |_, a| ret(a[0]));
        e.call_log = Some(vec![]);
        e.call(0x0048_3a50, &[]);
        let log = e.call_log.take().unwrap();
        let map = e.global::<u32>(FORM_MAP);
        let editor_ids = e.global::<u32>(EDITOR_ID_MAP);
        let list = e.global::<u32>(FORM_LIST);
        assert!(map != 0 && editor_ids != 0 && list != 0);
        assert_eq!(calls_to(&log, FORM_MAP_CONSTRUCT), vec![vec![map, 0x2008d]]);
        assert_eq!(
            calls_to(&log, EDITOR_ID_MAP_CONSTRUCT),
            vec![vec![editor_ids, 0xfa1, 1]]
        );
        assert_eq!(
            calls_to(&log, FORM_LIST_CONSTRUCT),
            vec![vec![list, 0x20, 1]]
        );
        // The three blocks are 0x10, 0x14 and 0x18 bytes.
        assert_eq!(e.mem.block_size(map), Some(0x10));
        assert!(e.mem.block_size(editor_ids).unwrap() >= 0x14);
        assert_eq!(e.mem.block_size(list), Some(0x18));
        // A second call changes nothing.
        e.call_log = Some(vec![]);
        e.call(0x0048_3a50, &[]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        assert_eq!(e.global::<u32>(FORM_MAP), map);
    }

    #[test]
    fn registries_are_destroyed_and_cleared_from_the_globals() {
        let mut e = form_engine();
        e.register(MAP_CLEAR, |_, _| Ret::default());
        e.register(FORM_LIST_CLEAR, |_, _| Ret::default());
        // Nothing to do without registries.
        e.call_log = Some(vec![]);
        e.call(0x0048_3b80, &[]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        let map = object_with_vtable(&mut e);
        let editor_ids = object_with_vtable(&mut e);
        let list = object_with_vtable(&mut e);
        e.set_global(FORM_MAP, map);
        e.set_global(EDITOR_ID_MAP, editor_ids);
        e.set_global(FORM_LIST, list);
        e.call_log = Some(vec![]);
        e.call(0x0048_3b80, &[]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, MAP_CLEAR), vec![vec![map], vec![editor_ids]]);
        assert_eq!(calls_to(&log, FORM_LIST_CLEAR), vec![vec![list]]);
        assert_eq!(
            calls_to(&log, SLOT_DESTRUCTOR),
            vec![vec![map, 1], vec![editor_ids, 1], vec![list, 1]]
        );
        assert_eq!(e.global::<u32>(FORM_MAP), 0);
        assert_eq!(e.global::<u32>(EDITOR_ID_MAP), 0);
        assert_eq!(e.global::<u32>(FORM_LIST), 0);
    }

    #[test]
    fn remove_from_data_structures_skips_temporary_forms() {
        let mut e = form_engine();
        e.set_global(FORM_MAP, 0x7700_0000u32);
        e.set_global(DATA_HANDLER, 0x7900_0000u32);
        let temporary = form(&mut e, 0x4000, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_3c70, &args![temporary]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
    }

    #[test]
    fn remove_from_data_structures_clears_map_list_and_handler() {
        let mut e = form_engine();
        e.register(MAP_REMOVE_AT, |_, _| Ret::default());
        e.register(FORM_LIST_REMOVE_AT, |_, _| Ret::default());
        e.register(DATA_HANDLER_REMOVE_ID, |_, _| Ret::default());
        let other = form(&mut e, 8, 0x20);
        let target = form(&mut e, 8, 0x20);
        let list = form_array(&mut e, &[other, target]);
        e.set_global(FORM_MAP, 0x7700_0000u32);
        e.set_global(FORM_LIST, list);
        e.set_global(DATA_HANDLER, 0x7900_0000u32);
        e.call_log = Some(vec![]);
        e.call(0x0048_3c70, &args![target]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, MAP_REMOVE_AT),
            vec![vec![0x7700_0000, 0x0100_0abc]]
        );
        assert_eq!(calls_to(&log, FORM_LIST_REMOVE_AT), vec![vec![list, 1]]);
        assert_eq!(
            calls_to(&log, DATA_HANDLER_REMOVE_ID),
            vec![vec![0x7900_0000, 0x0100_0abc]]
        );

        // Not in the array, and no map or handler: nothing is called.
        let stranger = form(&mut e, 8, 0x20);
        e.set_global(FORM_MAP, 0u32);
        e.set_global(DATA_HANDLER, 0u32);
        e.call_log = Some(vec![]);
        e.call(0x0048_3c70, &args![stranger]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, FORM_LIST_REMOVE_AT).is_empty());
        assert!(calls_to(&log, MAP_REMOVE_AT).is_empty());
    }

    #[test]
    fn save_adds_the_form_to_the_file_unless_temporary() {
        let mut e = form_engine();
        e.register(FILE_ADD_FORM, |_, _| ret(0));
        let file = Ptr::<()>::new(0x5100_0000);
        let normal = form(&mut e, 8, 0x20);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0048_3d20, &args![normal, file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, SLOT_0X2C), vec![vec![normal.addr()]]);
        assert_eq!(
            calls_to(&log, FILE_ADD_FORM),
            vec![vec![file.addr(), normal.addr()]]
        );
        // A nonzero result from the file is a failure.
        e.register(FILE_ADD_FORM, |_, _| ret(3));
        assert!(!e.call(0x0048_3d20, &args![normal, file]).bool());
        // Temporary forms are not saved.
        let temporary = form(&mut e, 0x4000, 0x20);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0048_3d20, &args![temporary, file]).bool());
        assert_eq!(e.call_log.take().unwrap().len(), 2);
    }

    /// Doubles for zlib and the byte-swapping helpers; the deflate double
    /// checks its input and writes `ABCDE`.
    fn compress_engine() -> Engine {
        let mut e = form_engine();
        e.register(DEFLATE_INIT, |_, a| {
            assert_eq!(a[1..], [0xffff_ffff, ZLIB_VERSION, 0x38]);
            ret(0)
        });
        e.register(DEFLATE, |e, a| {
            let stream = a[0];
            assert_eq!(a[1], 4);
            // avail_in = 10, avail_out = twice that.
            assert_eq!(e.mem.u32(stream + 4), 10);
            assert_eq!(e.mem.u32(stream + 0x10), 20);
            let out = e.mem.u32(stream + 0x0c);
            e.mem.write(out, b"ABCDE");
            e.mem.set_u32(stream + 0x10, 15);
            ret(1)
        });
        e.register(DEFLATE_END, |_, _| Ret::default());
        e.register(FORM_ENDIAN, |_, _| Ret::default());
        e.register(SWAP_WORD, |e, a| {
            let value = e.mem.u32(a[0]);
            e.mem.set_u32(a[0], value.swap_bytes());
            Ret::default()
        });
        e.register(LOG_MESSAGE, |_, _| Ret::default());
        e
    }

    /// A save buffer of a 0x18-byte header and ten body bytes.
    fn install_save_buffer(e: &mut Engine, flags: u32) -> u32 {
        let buffer = e.mem.alloc(0x22);
        e.mem.set_u32(buffer, 0x4d41_4e59);
        e.mem.set_u32(buffer + 4, 10);
        e.mem.set_u32(buffer + 8, flags);
        e.mem.write(buffer + 0x18, b"0123456789");
        e.set_global(SAVE_BUFFER, buffer);
        e.set_global(SAVE_BUFFER_SIZE, 0x22u32);
        buffer
    }

    #[test]
    fn compress_replaces_the_buffer_with_header_size_and_stream() {
        let mut e = compress_engine();
        let old = install_save_buffer(&mut e, 0x10);
        e.call_log = Some(vec![]);
        e.call(0x0048_3d70, &[]);
        let log = e.call_log.take().unwrap();
        let new = e.global::<u32>(SAVE_BUFFER);
        assert_ne!(new, old);
        // 5 compressed bytes + header (0x18) + the size word (4).
        assert_eq!(e.global::<u32>(SAVE_BUFFER_SIZE), 0x21);
        assert_eq!(e.mem.u32(new), 0x4d41_4e59);
        assert_eq!(e.mem.u32(new + 4), 9);
        assert_eq!(e.mem.u32(new + 8), 0x10 | 0x40000);
        assert_eq!(e.mem.u32(new + 0x18), 10);
        assert_eq!(e.mem.bytes(new + 0x1c, 5), b"ABCDE");
        assert_eq!(e.mem.block_size(old), None);
        assert!(calls_to(&log, FORM_ENDIAN).is_empty());
        assert_eq!(calls_to(&log, DEFLATE_END).len(), 1);
    }

    #[test]
    fn compress_swaps_the_header_and_size_on_a_big_endian_target() {
        let mut e = compress_engine();
        e.set_global(BIG_ENDIAN_FLAG, 1u8);
        install_save_buffer(&mut e, 0);
        e.call_log = Some(vec![]);
        e.call(0x0048_3d70, &[]);
        let log = e.call_log.take().unwrap();
        let new = e.global::<u32>(SAVE_BUFFER);
        assert_eq!(calls_to(&log, FORM_ENDIAN).len(), 2);
        assert_eq!(e.mem.u32(new + 0x18), 10u32.swap_bytes());
    }

    #[test]
    fn compress_leaves_small_missing_and_compressed_buffers_alone() {
        let mut e = compress_engine();
        // No buffer.
        e.call_log = Some(vec![]);
        e.call(0x0048_3d70, &[]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        // A header only.
        let buffer = install_save_buffer(&mut e, 0);
        e.set_global(SAVE_BUFFER_SIZE, 0x18u32);
        e.call_log = Some(vec![]);
        e.call(0x0048_3d70, &[]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        // Already compressed: only the endian swap of the header, twice on a
        // big-endian target.
        e.set_global(SAVE_BUFFER_SIZE, 0x22u32);
        e.mem.set_u32(buffer + 8, 0x40000);
        e.set_global(BIG_ENDIAN_FLAG, 1u8);
        e.call_log = Some(vec![]);
        e.call(0x0048_3d70, &[]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, FORM_ENDIAN).len(), 2);
        assert!(calls_to(&log, DEFLATE_INIT).is_empty());
        assert_eq!(e.global::<u32>(SAVE_BUFFER), buffer);
    }

    #[test]
    fn compress_logs_zlib_failures_and_keeps_the_buffer() {
        let mut e = compress_engine();
        let buffer = install_save_buffer(&mut e, 0);
        e.register(DEFLATE_INIT, |_, _| ret(1));
        e.call_log = Some(vec![]);
        e.call(0x0048_3d70, &[]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, LOG_MESSAGE), vec![vec![ZLIB_INIT_ERROR]]);
        assert_eq!(e.global::<u32>(SAVE_BUFFER), buffer);
        assert_eq!(e.mem.u32(buffer + 8), 0);

        e.register(DEFLATE_INIT, |_, _| ret(0));
        e.register(DEFLATE, |_, _| ret(-2i32 as u32));
        e.call_log = Some(vec![]);
        e.call(0x0048_3d70, &[]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, LOG_MESSAGE), vec![vec![ZLIB_DEFLATE_ERROR]]);
        assert!(calls_to(&log, DEFLATE_END).is_empty());
        assert_eq!(e.global::<u32>(SAVE_BUFFER), buffer);
        assert_eq!(e.mem.u32(buffer + 8), 0);
    }

    #[test]
    fn write_form_starts_deleted_forms_and_defers_others_to_the_virtual() {
        let mut e = form_engine();
        e.register(FILE_START_FORM, |_, _| Ret::default());
        e.register(FILE_AFTER_START, |_, _| Ret::default());
        let file = Ptr::<()>::new(0x5100_0000);
        let temporary = form(&mut e, 0x4000, 0x20);
        assert!(!e.call(0x0048_3fc0, &args![temporary, file]).bool());
        let deleted = form(&mut e, 0x20, 0x20);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0048_3fc0, &args![deleted, file]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, FILE_START_FORM),
            vec![vec![file.addr(), deleted.addr()]]
        );
        assert_eq!(calls_to(&log, FILE_AFTER_START), vec![vec![file.addr()]]);
        assert!(calls_to(&log, SLOT_0X28).is_empty());
        // Otherwise the virtual's result decides.
        let normal = form(&mut e, 8, 0x20);
        e.register(SLOT_0X28, |_, _| ret(1));
        e.call_log = Some(vec![]);
        assert!(e.call(0x0048_3fc0, &args![normal, file]).bool());
        assert_eq!(
            calls_to(&e.call_log.take().unwrap(), SLOT_0X28),
            vec![vec![normal.addr(), file.addr()]]
        );
        e.register(SLOT_0X28, |_, _| ret(0x100));
        assert!(!e.call(0x0048_3fc0, &args![normal, file]).bool());
    }

    #[test]
    fn type_order_ranks_the_special_types_by_table() {
        let mut e = form_engine();
        let low = form(&mut e, 8, 0x20); // rank 320
        let special = form(&mut e, 8, 0x67); // rank 0x137 = 311
        let other_special = form(&mut e, 8, 0x74); // rank 0x13a = 314
        assert!(e.call(0x0048_4020, &args![special, low]).bool());
        assert!(!e.call(0x0048_4020, &args![low, special]).bool());
        assert!(e.call(0x0048_4020, &args![special, other_special]).bool());
        assert!(!e.call(0x0048_4020, &args![low, low]).bool());
        let t6c = form(&mut e, 8, 0x6c);
        let t73 = form(&mut e, 8, 0x73);
        assert!(e.call(0x0048_4020, &args![t6c, t73]).bool());
        assert!(e.call(0x0048_4020, &args![other_special, low]).bool());
    }

    #[test]
    fn group_order_depends_on_the_group_type() {
        let mut e = form_engine();
        e.set_global(GROUP_TAG, GROUP_TAG_VALUE);
        e.register(GET_FORM_TYPE_FROM_FORM_STRING, |_, _| ret(5));
        let group = e.mem.alloc(0x14);
        e.mem.set_u32(group, GROUP_TAG_VALUE);
        let ask = |e: &mut Engine, group_type: u32, form_type: u8| {
            e.mem.set_u32(group + 0xc, group_type);
            let f = form(e, 8, form_type);
            e.call(0x0048_4150, &args![f, Ptr::<()>::new(group)]).bool()
        };
        // Group type 0: rank of the label's form type (5 -> 50).
        assert!(ask(&mut e, 0, 4));
        assert!(!ask(&mut e, 0, 5));
        // Types 1, 4, 5: below 0x28a (650).
        for group_type in [1, 4, 5] {
            assert!(ask(&mut e, group_type, 64));
            assert!(!ask(&mut e, group_type, 65));
        }
        // Types 2, 3, 6: below 0x23a (570).
        for group_type in [2, 3, 6] {
            assert!(ask(&mut e, group_type, 56));
            assert!(!ask(&mut e, group_type, 57));
        }
        // Type 7: below 0x2b2 (690).
        assert!(ask(&mut e, 7, 68));
        assert!(!ask(&mut e, 7, 69));
        // The special ranks count too (0x67 ranks 311).
        assert!(ask(&mut e, 2, 0x67));
        // Unknown group types, a null group and a wrong tag are false.
        assert!(!ask(&mut e, 8, 1));
        let f = form(&mut e, 8, 1);
        assert!(!e.call(0x0048_4150, &args![f, Ptr::<()>::NULL]).bool());
        e.mem.set_u32(group, 0x1234);
        e.mem.set_u32(group + 0xc, 1);
        assert!(!e.call(0x0048_4150, &args![f, Ptr::<()>::new(group)]).bool());
    }

    #[test]
    fn set_temporary_removes_the_form_from_the_tables_then_flags_it() {
        let mut e = form_engine();
        e.register(SAVE_LOAD_REMOVE_CHANGES, |_, _| Ret::default());
        e.register(BGS_REMOVE_CHANGES, |_, _| Ret::default());
        let f = form(&mut e, 8, 0x20);
        // No save/load objects: only the flag changes.
        e.call_log = Some(vec![]);
        e.call(0x0048_4490, &args![f]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, SAVE_LOAD_REMOVE_CHANGES).is_empty());
        assert!(calls_to(&log, BGS_REMOVE_CHANGES).is_empty());
        assert_eq!(e.get(f, TESForm::iFormFlags), 8 | 0x4000);
        // With them.
        e.set_global(SAVE_LOAD_GAME, 0x6600_0000u32);
        e.set_global(BGS_SAVE_LOAD_GAME, 0x6700_0000u32);
        let g = form(&mut e, 8, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_4490, &args![g]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, SAVE_LOAD_REMOVE_CHANGES),
            vec![vec![0x6600_0000, g.addr(), 0]]
        );
        assert_eq!(
            calls_to(&log, BGS_REMOVE_CHANGES),
            vec![vec![0x6700_0000, g.addr()]]
        );
    }

    /// Checks a flag setter: it sets and clears exactly `mask` and calls the
    /// changed hook (virtual +0x48 with 1) when `notifies`.
    fn check_flag_setter(addr: u32, mask: u32, notifies: bool) {
        let mut e = form_engine();
        let f = form(&mut e, 0xa000_0000, 0x20);
        e.call_log = Some(vec![]);
        e.call(addr, &args![f, true]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(f, TESForm::iFormFlags), 0xa000_0000 | mask);
        let hooks = calls_to(&log, SLOT_0X48);
        if notifies {
            assert_eq!(hooks, vec![vec![f.addr(), 1]]);
        } else {
            assert!(hooks.is_empty());
        }
        e.set(f, TESForm::iFormFlags, 0xffff_ffff);
        e.call_log = Some(vec![]);
        e.call(addr, &args![f, false]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(f, TESForm::iFormFlags), !mask);
        assert_eq!(calls_to(&log, SLOT_0X48).len(), notifies as usize);
    }

    #[test]
    fn flag_0x1_setter() {
        check_flag_setter(0x0048_44f0, 0x1, false);
    }

    #[test]
    fn set_delete_sets_0x20_and_notifies() {
        check_flag_setter(0x0048_4530, 0x20, true);
    }

    #[test]
    fn flag_0x2000_setter() {
        check_flag_setter(0x0048_4610, 0x2000, false);
    }

    #[test]
    fn flag_0x800000_setter() {
        check_flag_setter(0x0048_4650, 0x80_0000, true);
    }

    #[test]
    fn set_destructible_sets_0x1000000() {
        check_flag_setter(0x0048_46a0, 0x100_0000, false);
    }

    #[test]
    fn flag_0x4000000_setter() {
        check_flag_setter(0x0048_46e0, 0x400_0000, true);
    }

    #[test]
    fn flag_0x400_setter() {
        check_flag_setter(0x0048_4860, 0x400, true);
    }

    #[test]
    fn flag_0x40_setter() {
        check_flag_setter(0x0048_48b0, 0x40, true);
    }

    #[test]
    fn flag_0x10000_setter() {
        check_flag_setter(0x0048_4900, 0x1_0000, true);
    }

    #[test]
    fn flag_0x20000_setter() {
        check_flag_setter(0x0048_4950, 0x2_0000, false);
    }

    #[test]
    fn flag_0x100000_setter() {
        check_flag_setter(0x0048_4990, 0x10_0000, false);
    }

    #[test]
    fn flag_0x200_setter() {
        check_flag_setter(0x0048_49d0, 0x200, false);
    }

    #[test]
    fn second_flag_0x40_setter() {
        check_flag_setter(0x0048_4a10, 0x40, true);
    }

    #[test]
    fn set_fire_off_sets_0x80() {
        check_flag_setter(0x0048_4a70, 0x80, false);
    }

    #[test]
    fn flag_0x8_setter() {
        check_flag_setter(0x0048_4ab0, 0x8, false);
    }

    #[test]
    fn set_disabled_sets_0x800_and_notifies() {
        check_flag_setter(0x0048_4af0, 0x800, true);
    }

    #[test]
    fn set_empty_calls_the_slot_unless_the_stub_says_otherwise() {
        let mut e = form_engine();
        e.register(SAVE_LOAD_STUB, |_, _| ret(0));
        e.set_global(SAVE_LOAD_GAME, 0x6600_0000u32);
        let f = form(&mut e, 0, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_4580, &args![f, true]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, SAVE_LOAD_STUB), vec![vec![0x6600_0000]]);
        assert_eq!(calls_to(&log, SLOT_0X48), vec![vec![f.addr(), 0x20_0000]]);
        assert!(calls_to(&log, SLOT_0X4C).is_empty());
        assert_eq!(e.get(f, TESForm::iFormFlags), 0x2000);
        e.call_log = Some(vec![]);
        e.call(0x0048_4580, &args![f, false]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, SLOT_0X4C), vec![vec![f.addr(), 0x20_0000]]);
        assert!(calls_to(&log, SLOT_0X48).is_empty());
        assert_eq!(e.get(f, TESForm::iFormFlags), 0);
        // The stub returning true suppresses the slot call.
        e.register(SAVE_LOAD_STUB, |_, _| ret(1));
        e.call_log = Some(vec![]);
        e.call(0x0048_4580, &args![f, true]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, SLOT_0X48).is_empty());
        assert_eq!(e.get(f, TESForm::iFormFlags), 0x2000);
    }

    #[test]
    fn flag_2_set_adds_the_form_to_the_array_once() {
        let mut e = form_engine();
        e.register(FORM_LIST_ADD, |_, _| ret(0));
        let f = form(&mut e, 0, 0x20);
        // No array: only the flag.
        e.call_log = Some(vec![]);
        e.call(0x0048_4730, &args![f, true]);
        assert!(calls_to(&e.call_log.take().unwrap(), FORM_LIST_ADD).is_empty());
        assert_eq!(e.get(f, TESForm::iFormFlags), 2);
        // With an array that lacks the form: added through a pointer slot.
        let g = form(&mut e, 0, 0x20);
        let list = form_array(&mut e, &[]);
        e.set_global(FORM_LIST, list);
        e.call_log = Some(vec![]);
        e.call(0x0048_4730, &args![g, true]);
        let log = e.call_log.take().unwrap();
        let adds = calls_to(&log, FORM_LIST_ADD);
        assert_eq!(adds.len(), 1);
        assert_eq!(adds[0][0], list);
        assert_eq!(e.get(g, TESForm::iFormFlags), 2);
        // Already listed: not added again.
        let listed = form(&mut e, 0, 0x20);
        let list = form_array(&mut e, &[listed]);
        e.set_global(FORM_LIST, list);
        e.call_log = Some(vec![]);
        e.call(0x0048_4730, &args![listed, true]);
        assert!(calls_to(&e.call_log.take().unwrap(), FORM_LIST_ADD).is_empty());
        // Flag already set, or temporary: not added either.
        let flagged = form(&mut e, 2, 0x20);
        let temporary = form(&mut e, 0x4000, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_4730, &args![flagged, true]);
        e.call(0x0048_4730, &args![temporary, true]);
        assert!(calls_to(&e.call_log.take().unwrap(), FORM_LIST_ADD).is_empty());
        assert_eq!(e.get(temporary, TESForm::iFormFlags), 0x4002);
    }

    #[test]
    fn flag_2_clear_removes_the_form_from_the_array() {
        let mut e = form_engine();
        e.register(FORM_LIST_REMOVE_AT, |_, _| Ret::default());
        let other = form(&mut e, 2, 0x20);
        let f = form(&mut e, 0x12, 0x20);
        let list = form_array(&mut e, &[other, f]);
        e.set_global(FORM_LIST, list);
        e.call_log = Some(vec![]);
        e.call(0x0048_4730, &args![f, false]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, FORM_LIST_REMOVE_AT), vec![vec![list, 1]]);
        assert_eq!(e.get(f, TESForm::iFormFlags), 0x10);
        // A form without the flag is not searched for.
        let plain = form(&mut e, 0x10, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_4730, &args![plain, false]);
        assert!(calls_to(&e.call_log.take().unwrap(), FORM_LIST_REMOVE_AT).is_empty());
        assert_eq!(e.get(plain, TESForm::iFormFlags), 0x10);
    }

    #[test]
    fn form_id_range_check() {
        let mut e = Engine::new();
        for (id, expected) in [(0u32, false), (1, true), (0x7ff, true), (0x800, false)] {
            assert_eq!(e.call(0x0048_4b40, &args![id]).bool(), expected, "{id:#x}");
        }
    }

    #[test]
    fn change_recorders_pass_the_form_flags_and_force_word() {
        let mut e = form_engine();
        e.register(STORE_WORD, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(BGS_ADD_CHANGE, |_, _| Ret::default());
        e.register(BGS_CHANGE_SIBLING, |_, _| Ret::default());
        e.set_global(BGS_SAVE_LOAD_GAME, 0x6700_0000u32);
        let f = form(&mut e, 8, 0x20);
        e.call_log = Some(vec![]);
        e.call(0x0048_4b60, &args![f, 0x0300_0001u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, BGS_ADD_CHANGE),
            vec![vec![0x6700_0000, f.addr(), 0x0300_0001, 0]]
        );
        e.call_log = Some(vec![]);
        e.call(0x0048_4b90, &args![f, 0x20u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, BGS_ADD_CHANGE),
            vec![vec![0x6700_0000, f.addr(), 0x20, 1]]
        );
        e.call_log = Some(vec![]);
        e.call(0x0048_4bc0, &args![f, 0x44u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, BGS_CHANGE_SIBLING),
            vec![vec![0x6700_0000, f.addr(), 0x44]]
        );
    }
}
