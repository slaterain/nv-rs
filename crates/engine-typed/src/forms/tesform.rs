//! `TESForm`'s own functions (`fallout shared/tesform.cpp`, Xbox PDB source unit), each as the exe
//! does it.
//!
//! A form is `TESForm`'s layout: the vtable at +0, the form type byte at +4, the flags word at +8,
//! the form id at +0x0c and the source file list (`BSSimpleList`, its first node embedded) at
//! +0x10. The unit's registries: the form map (form id to form, `011c54c0`), the editor id map
//! (`011c54c8`) and the form array (`011c54c4`, items at +4, count at +0x0c) of the forms that carry
//! flag 2. The record writer works on the buffer at `011c54cc` of the size at `011c54d0`: a 0x18
//! byte record header (type tag, data size, flags, form id, version-control word, the version as a
//! `u16` and one more `u16`), then chunks of a 4 byte tag, a `u16` size and the data.
//!
//! The small tests the exe calls (the form type `00401170`, temporary `004077c0`, flag 2
//! `00460340`, the big-endian target `00401500`, the list's first node `00460140`) stay calls by
//! address, as they are in the translation; the leaves' family answers the getters it holds.

use super::FAMILY;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

// ---- globals ----
/// Number of live forms (the registries exist while it is not 0).
const FORM_COUNT: u32 = 0x011c_54bc;
/// Form id to form map, editor id to form map, the form array (pointers).
const FORM_MAP: u32 = 0x011c_54c0;
const EDITOR_ID_MAP: u32 = 0x011c_54c8;
const FORM_LIST: u32 = 0x011c_54c4;
/// The record writer's buffer (pointer) and its size in bytes.
const SAVE_BUFFER: u32 = 0x011c_54cc;
const SAVE_BUFFER_SIZE: u32 = 0x011c_54d0;
/// `TESDataHandler`, `TESSaveLoadGame` and `BGSSaveLoadGame` (pointers).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const BGS_SAVE_LOAD_GAME: u32 = 0x011d_df38;
/// The form-type table: entries of 12 bytes, the type's packed string word at +8.
const FORM_ENUM_TABLE: u32 = 0x0118_7000;
const FORM_ENUM_STRIDE: u32 = 12;
/// The packed string of a group record's tag (the table's third entry).
const GROUP_TAG: u32 = 0x0118_7020;
/// `TESForm`'s vtable.
const TES_FORM_VTABLE: u32 = 0x0101_c794;

// ---- callees by address ----
const SIMPLE_LIST_DESTROY: u32 = 0x0047_0470;
const SIMPLE_LIST_DESTROY_WRAPPER: u32 = 0x0046_ffb0;
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const MEMCPY: u32 = 0x0040_1460;
const SWAP_WORD: u32 = 0x0040_1080;
const IS_BIG_ENDIAN: u32 = 0x0040_1500;
const FORM_ENDIAN: u32 = 0x0047_0650;
const FORM_TYPE: u32 = 0x0040_1170;
const IS_TEMPORARY: u32 = 0x0040_77c0;
const IS_FLAG_2: u32 = 0x0046_0340;
const FORM_LIST_REMOVE_AT: u32 = 0x0048_6b80;
const FORM_LIST_ADD: u32 = 0x0047_0140;
const FORM_LIST_CONSTRUCT: u32 = 0x0047_01d0;
const FORM_LIST_CLEAR: u32 = 0x0086_3db0;
const FORM_MAP_CONSTRUCT: u32 = 0x0048_6930;
const EDITOR_ID_MAP_CONSTRUCT: u32 = 0x0048_6ab0;
const MAP_CLEAR: u32 = 0x0043_8af0;
const MAP_LOOKUP: u32 = 0x0085_3130;
const MAP_REMOVE_AT: u32 = 0x0040_5430;
const MAP_SET_AT: u32 = 0x0084_4700;
const DATA_HANDLER_REMOVE_ID: u32 = 0x0046_96f0;
const SAVE_LOAD_REMOVE_CHANGES: u32 = 0x0085_6c70;
const SAVE_LOAD_REMOVE_FORM: u32 = 0x0085_a410;
const BGS_REMOVE_CHANGES: u32 = 0x0084_a7c0;
const BGS_ADD_CHANGE: u32 = 0x0084_a690;
const BGS_CHANGE_SIBLING: u32 = 0x0084_a780;
const STORE_WORD: u32 = 0x008c_71b0;
const FILE_START_FORM: u32 = 0x0047_2e60;
const FILE_ADD_FORM: u32 = 0x0047_2fe0;
const FILE_AFTER_START: u32 = 0x0047_3090;
const GET_FORM_TYPE_FROM_FORM_STRING: u32 = 0x0048_6890;
const COMPONENTS_CONSTRUCT: u32 = 0x0047_c050;
const COMPONENTS_INIT: u32 = 0x0047_c070;
const COMPONENTS_VISIT_FIRST: u32 = 0x0047_c560;
const COMPONENTS_VISIT_SECOND: u32 = 0x0047_c5b0;
const COMPONENTS_COPY: u32 = 0x0047_c600;
const COMPONENTS_COMPARE: u32 = 0x0047_c670;
/// A `FormComponentCollection`, built on the stack: 0x26 pointers.
const COMPONENT_COLLECTION_SIZE: u32 = 0x98;
const CLEAR_SUB_OBJECT: u32 = 0x0048_ce80;
/// The save/load stub that answers false (`0047c850`): a call by address, as the translation
/// makes it (its owner, the draw family, may be off).
const SAVE_LOAD_STUB: u32 = 0x0047_c850;
/// The game's printf-style log: YUP and xNVSE may replace it.
const LOG_MESSAGE: u32 = 0x005b_5e40;
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_BASE_FORM_COMPONENT: u32 = 0x0118_3040;
const TYPE_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const SAVE_LOAD_SAVE_DATA: u32 = 0x0085_79b0;
const SAVE_LOAD_LOAD_DATA: u32 = 0x0085_79e0;
const SAVE_LOAD_SAVE_NUMERIC_ID: u32 = 0x0085_7a10;
const SAVE_LOAD_LOAD_NUMERIC_ID: u32 = 0x0085_7aa0;
const BUFFER_SAVE_KIND: u32 = 0x0042_8110;
const WORD_HAS_BITS: u32 = 0x0042_80f0;
const SAVE_BUFFER_WRITE: u32 = 0x0086_5e50;
const LOAD_BUFFER_READ: u32 = 0x0086_4980;
const FORM_TYPE_NAME: u32 = 0x0044_0e30;
/// The form's name key (JohnnyGuitar NVSE may replace it).
const FORM_NAME_KEY: u32 = 0x0047_4cb0;
const STRING_COMPARE: u32 = 0x0040_4dc0;
const FILE_LOAD_FORM: u32 = 0x0047_2f60;
const FILE_MASTER_LIST: u32 = 0x0046_4df0;
const FILE_ROOT: u32 = 0x0047_3c70;
const FILE_IS_MASTER: u32 = 0x0047_1c20;
const LIST_FIRST_NODE: u32 = 0x0046_0140;
const LIST_POP_NODE: u32 = 0x0063_f7b0;
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
const LIST_APPEND: u32 = 0x0090_5820;
const LIST_PUSH_FRONT: u32 = 0x005a_e3d0;
const RECORD_VERSION: u32 = 0x0047_0bb0;
const SWAP_HALF: u32 = 0x0040_7a90;
const SWAP_CHUNK_HEADER: u32 = 0x0041_41e0;
const BUFFER_REALLOC: u32 = 0x0042_f5d0;
const COPY_MESSAGE: u32 = 0x0101_c930;
const COMPARE_MESSAGE: u32 = 0x0101_c978;
const FILE_GET_INDEX_FILE: u32 = 0x0047_1a10;
const FILE_COMPILE_INDEX: u32 = 0x0047_3250;
const SAVE_LOAD_REMAP_ID: u32 = 0x0085_7bf0;

/// The tag `"XXXX"` of the chunk written ahead of one larger than 0xffff.
const OVERSIZE_CHUNK_TAG: u32 = 0x5858_5858;
/// Flag bits: temporary, deleted; what `StartForm` keeps for a type other than 1; what the flags'
/// load takes for a reference and for any other form; what `differs_from` compares.
const FLAG_TEMPORARY: u32 = 0x4000;
const FLAG_DELETED: u32 = 0x20;
const START_FORM_FLAGS_MASK: u32 = 0x3003_2fe0;
const LOAD_GAME_REFERENCE_FLAGS: u32 = 0x0091_2860;
const LOAD_GAME_FORM_FLAGS: u32 = 0x4000_0c20;
const COMPARE_FLAGS_MASK: u32 = 0xffff_bff4;
/// The form's virtual slots: set flag 2, after `StartForm`, is a reference, the name.
const SLOT_SET_FLAG_2: u32 = 0xc8;
const SLOT_AFTER_START: u32 = 0xdc;
const SLOT_IS_REFERENCE: u32 = 0xf0;
const SLOT_NAME: u32 = 0x130;

/// The low half of a word (the exe's `u16` store of a size it has bounded or means to cut).
#[allow(clippy::cast_possible_truncation)]
fn low_half(word: u32) -> u16 {
    word as u16
}

/// Sets or clears `mask` in the form's flags (+8).
fn set_form_flag(e: &mut Engine, this: u32, mask: u32, on: bool) {
    let at = this.wrapping_add(8);
    let flags = e.mem.u32(at);
    e.mem
        .set_u32(at, if on { flags | mask } else { flags & !mask });
}

/// The form's slot 0x48 with 1: the "form changed" call after a saved flag changes.
fn notify_changed(e: &mut Engine, this: u32) {
    e.vcall(this, 0x48, &args![1u32]);
}

/// Sort key of a form type, as the exe inlines it: `0x67`, `0x6c`, `0x73` and `0x74` rank `0x137`
/// to `0x13a`, every other type `10 * type`.
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
fn form_rank(e: &mut Engine, form: u32) -> u32 {
    let form_type = e.call(FORM_TYPE, &args![form]).u32();
    form_type_rank(form_type)
}

/// Index of `form` in the form array, if the array exists and holds it.
fn find_in_form_list(e: &mut Engine, form: u32) -> Option<u32> {
    let mut index = 0u32;
    loop {
        let list = e.mem.u32(FORM_LIST);
        if list == 0 {
            return None;
        }
        let count = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, list));
        if index >= count {
            return None;
        }
        let item = typed_call!(
            leaf::FAMILY,
            0x0087_7a30,
            leaf::array_element(e, list, index)
        );
        if e.mem.u32(item) == form {
            return Some(index);
        }
        index = index.wrapping_add(1);
    }
}

/// The first node of the form's source file list (the node embedded at +0x10).
fn list_first(e: &mut Engine, form: u32) -> u32 {
    e.call(LIST_FIRST_NODE, &args![form]).u32()
}

/// The item (a file) of a list node.
fn list_item(e: &mut Engine, node: u32) -> u32 {
    let slot = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    e.mem.u32(slot)
}

/// The node after `node` (0 at the end).
fn list_next(e: &mut Engine, node: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node))
}

/// Whether a node holds no item and no next node.
fn list_node_empty(e: &mut Engine, node: u32) -> bool {
    typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, node))
}

/// Runs `f` with the address of a stack word holding `value` (an item passed by address).
fn with_item<R>(e: &mut Engine, value: u32, f: impl FnOnce(&mut Engine, u32) -> R) -> R {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        f(e, slot.addr())
    })
}

/// Whether the target stores multi-byte values big-endian.
fn is_big_endian(e: &mut Engine) -> bool {
    e.call(IS_BIG_ENDIAN, &args![]).bool()
}

/// Whether the form is temporary (`004077c0`).
fn is_temporary(e: &mut Engine, form: u32) -> bool {
    e.call(IS_TEMPORARY, &args![form]).bool()
}

/// The packed string word of the form's type: what a record header carries as its tag.
fn form_type_tag(e: &mut Engine, form: u32) -> u32 {
    let form_type = e.call(FORM_TYPE, &args![form]).u32();
    e.mem.u32(
        FORM_ENUM_TABLE
            .wrapping_add(form_type.wrapping_mul(FORM_ENUM_STRIDE))
            .wrapping_add(8),
    )
}

/// The form inside a `BaseFormComponent` pointer (`__RTDynamicCast`), or 0.
fn component_to_form(e: &mut Engine, component: u32) -> u32 {
    e.call(
        RT_DYNAMIC_CAST,
        &args![
            component,
            0u32,
            TYPE_BASE_FORM_COMPONENT,
            TYPE_TES_FORM,
            0u32
        ],
    )
    .u32()
}

/// Whether bit 0 of a save buffer's save kind is set (`00428110` into a local, then `004280f0`).
fn buffer_saves_flags(e: &mut Engine, buffer: u32) -> bool {
    e.with_stack(4, |e, kind| {
        let kind = e.call(BUFFER_SAVE_KIND, &args![buffer, kind]).u32();
        e.call(WORD_HAS_BITS, &args![kind, 1u32]).bool()
    })
}

// Typed from 004835e0
/// A `TESDataHandler` getter: the word at +0x20c, the file the handler is loading.
#[inline]
pub fn handler_loading_file(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(0x20c))
}

// Typed from 00483600
/// `TESForm`'s scalar deleting destructor: the destructor, then the release when bit 0 of `flags`
/// is set. Returns the object.
pub fn scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0048_3630, destructor(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 00483630
/// `TESForm::~TESForm` (Xbox PDB): out of the registries and, unless temporary, out of both save
/// games' change tables; the source file list goes; with the last form, the registries.
pub fn destructor(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, TES_FORM_VTABLE);
    // The script extender forgets what it kept for the form (its seam at this call).
    e.site(0x0048_3669, &[this]);
    typed_call!(FAMILY, 0x0048_3c70, remove_from_data_structures(e, this));
    e.call(SIMPLE_LIST_DESTROY, &args![this.wrapping_add(0x10)]);
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    if !is_temporary(e, this) && save_load != 0 {
        e.call(SAVE_LOAD_REMOVE_CHANGES, &args![save_load, this, 0u32]);
        e.call(SAVE_LOAD_REMOVE_FORM, &args![save_load, this]);
    }
    let bgs = e.mem.u32(BGS_SAVE_LOAD_GAME);
    if !is_temporary(e, this) && bgs != 0 {
        e.call(BGS_REMOVE_CHANGES, &args![bgs, this]);
    }
    let count = e.mem.u32(FORM_COUNT).wrapping_sub(1);
    e.mem.set_u32(FORM_COUNT, count);
    if count == 0 {
        typed_call!(FAMILY, 0x0048_3b80, destroy_registries(e));
    }
    e.call(SIMPLE_LIST_DESTROY_WRAPPER, &args![this.wrapping_add(0x10)]);
}

// Typed from 00483710
/// An empty base-class destructor the exe still calls; the typed callers leave the call out.
#[inline]
pub fn empty_destructor(_e: &mut Engine, _this: u32) {}

/// Whether forms of this type have no components to visit.
fn has_no_components(form_type: u32) -> bool {
    matches!(form_type, 0x11 | 0x3a..=0x40 | 0x42 | 0x43 | 0x49 | 0x69)
}

// Typed from 00483720
/// Each component's slot 0, through the form's `FormComponentCollection` built on the stack
/// (`0047c560`); none for the types without components; type 0x39 clears its sub-object at +0x18.
pub fn each_component_slot_0(e: &mut Engine, this: u32) {
    let form_type = e.call(FORM_TYPE, &args![this]).u32();
    if has_no_components(form_type) {
        return;
    }
    if form_type == 0x39 {
        e.call(CLEAR_SUB_OBJECT, &args![this.wrapping_add(0x18)]);
        return;
    }
    e.with_stack(COMPONENT_COLLECTION_SIZE, |e, collection| {
        e.call(COMPONENTS_CONSTRUCT, &args![collection]);
        e.call(COMPONENTS_INIT, &args![collection, this]);
        e.call(COMPONENTS_VISIT_FIRST, &args![collection]);
    });
}

// Typed from 00483870
/// Each component's slot 1 (`0047c5b0`), as [`each_component_slot_0`]; type 0x39 does nothing.
pub fn each_component_slot_1(e: &mut Engine, this: u32) {
    let form_type = e.call(FORM_TYPE, &args![this]).u32();
    if has_no_components(form_type) || form_type == 0x39 {
        return;
    }
    e.with_stack(COMPONENT_COLLECTION_SIZE, |e, collection| {
        e.call(COMPONENTS_CONSTRUCT, &args![collection]);
        e.call(COMPONENTS_INIT, &args![collection, this]);
        e.call(COMPONENTS_VISIT_SECOND, &args![collection]);
    });
}

/// A map's lookup of `key` into a zeroed local: the value when found, else 0.
fn map_lookup(e: &mut Engine, map: u32, key: u32) -> u32 {
    e.with_stack(4, |e, result| {
        e.mem.set_u32(result.addr(), 0);
        if e.call(MAP_LOOKUP, &args![map, key, result]).bool() {
            e.mem.u32(result.addr())
        } else {
            0
        }
    })
}

// Typed from 004839c0
/// The form with this id in the form map; 0 without a map or a match.
pub fn form_by_id(e: &mut Engine, form_id: u32) -> u32 {
    let map = e.mem.u32(FORM_MAP);
    if map == 0 {
        return 0;
    }
    map_lookup(e, map, form_id)
}

/// `operator new(size)`, then `construct(block, words..)` when it gave a block: the new object or 0.
fn new_object(e: &mut Engine, size: u32, construct: u32, words: &[u32]) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        return 0;
    }
    let mut all = vec![block];
    all.extend_from_slice(words);
    e.call(construct, &all).u32()
}

// Typed from 00483a50
/// The form map (0x2008d buckets), the editor id map (0xfa1 buckets, keys copied) and the form
/// array (capacity 0x20), each made when it does not exist yet.
pub fn create_registries(e: &mut Engine) {
    if e.mem.u32(FORM_MAP) == 0 {
        let map = new_object(e, 0x10, FORM_MAP_CONSTRUCT, &[0x2008d]);
        e.mem.set_u32(FORM_MAP, map);
    }
    if e.mem.u32(EDITOR_ID_MAP) == 0 {
        let map = new_object(e, 0x14, EDITOR_ID_MAP_CONSTRUCT, &[0xfa1, 1]);
        e.mem.set_u32(EDITOR_ID_MAP, map);
    }
    if e.mem.u32(FORM_LIST) == 0 {
        let list = new_object(e, 0x18, FORM_LIST_CONSTRUCT, &[0x20, 1]);
        e.mem.set_u32(FORM_LIST, list);
    }
}

// Typed from 00483b80
/// The form map, the editor id map and the form array go: each cleared, then its scalar deleting
/// destructor (slot 0 with 1), and its global nulled.
pub fn destroy_registries(e: &mut Engine) {
    for (global, clear) in [
        (FORM_MAP, MAP_CLEAR),
        (EDITOR_ID_MAP, MAP_CLEAR),
        (FORM_LIST, FORM_LIST_CLEAR),
    ] {
        let object = e.mem.u32(global);
        if object != 0 {
            e.call(clear, &args![object]);
            let object = e.mem.u32(global);
            if object != 0 {
                e.vcall(object, 0, &args![1u32]);
            }
            e.mem.set_u32(global, 0);
        }
    }
}

// Typed from 00483c70
/// `TESForm::RemoveFromDataStructures` (Xbox PDB): unless temporary, the form's id leaves the form
/// map, its entry the form array and its id the data handler.
pub fn remove_from_data_structures(e: &mut Engine, this: u32) {
    if is_temporary(e, this) {
        return;
    }
    let map = e.mem.u32(FORM_MAP);
    if map != 0 {
        let id = e.mem.u32(this.wrapping_add(0xc));
        e.call(MAP_REMOVE_AT, &args![map, id]);
    }
    if let Some(index) = find_in_form_list(e, this) {
        let list = e.mem.u32(FORM_LIST);
        e.call(FORM_LIST_REMOVE_AT, &args![list, index]);
    }
    let handler = e.mem.u32(DATA_HANDLER);
    if handler != 0 {
        let id = e.mem.u32(this.wrapping_add(0xc));
        e.call(DATA_HANDLER_REMOVE_ID, &args![handler, id]);
    }
}

// Typed from 00483d20
/// `TESForm::Save` (Xbox PDB): false for a temporary form; else its slot 0x2c, then the file's
/// `AddTESForm`, true when that answers 0.
pub fn save(e: &mut Engine, this: u32, file: u32) -> bool {
    if is_temporary(e, this) {
        return false;
    }
    e.vcall(this, 0x2c, &[]);
    e.call(FILE_ADD_FORM, &args![file, this]).u32() == 0
}

// Typed from 00483fc0
/// The form written to `file`: 0 for a temporary form; a deleted form's record started in the file
/// (`TESFile::StartForm`, then `00473090`) and 1; else what its slot 0x28 answers for the file.
pub fn write_to_file(e: &mut Engine, this: u32, file: u32) -> u8 {
    if is_temporary(e, this) {
        return 0;
    }
    if typed_call!(leaf::FAMILY, 0x0044_0d80, leaf::form_flag_20(e, this)) {
        e.call(FILE_START_FORM, &args![file, this]);
        e.call(FILE_AFTER_START, &args![file]);
        return 1;
    }
    e.vcall(this, 0x28, &args![file]).u8()
}

// Typed from 00484020
/// Whether this form's type ranks before `other`'s in the file order (`other`'s asked first).
pub fn type_ranks_before(e: &mut Engine, this: u32, other: u32) -> bool {
    let other_rank = form_rank(e, other);
    let this_rank = form_rank(e, this);
    this_rank < other_rank
}

// Typed from 00484150
/// Whether this form goes before the group record `group` (false for none or a record that is not
/// a group). By the group type at +0x0c: 0 compares with the rank of the type its label (+8) names;
/// 1, 4 and 5 take ranks below 0x28a, 2, 3 and 6 below 0x23a, 7 below 0x2b2; others false.
pub fn goes_before_group(e: &mut Engine, this: u32, group: u32) -> bool {
    if group == 0 || e.mem.u32(group) != e.mem.u32(GROUP_TAG) {
        return false;
    }
    match e.mem.u32(group.wrapping_add(0xc)) {
        0 => {
            let label = e.mem.u32(group.wrapping_add(8));
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

// Typed from 00484490
/// `TESForm::SetTemporary` (Xbox PDB): out of the registries and both save games' change tables,
/// then the temporary flag (0x4000).
pub fn set_temporary(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0048_3c70, remove_from_data_structures(e, this));
    // JohnnyGuitar NVSE's seam after that call: its names of the form leave its map.
    e.site(0x0048_449a, &[this]);
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    if save_load != 0 {
        e.call(SAVE_LOAD_REMOVE_CHANGES, &args![save_load, this, 0u32]);
    }
    let bgs = e.mem.u32(BGS_SAVE_LOAD_GAME);
    if bgs != 0 {
        e.call(BGS_REMOVE_CHANGES, &args![bgs, this]);
    }
    set_form_flag(e, this, FLAG_TEMPORARY, true);
}

// Typed from 004844f0
/// Flag 0x1 set or cleared.
pub fn set_flag_1(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x1, on);
}

// Typed from 00484530
/// `TESForm::SetDelete` (Xbox PDB): the deleted flag (0x20), then the changed call.
pub fn set_delete(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, FLAG_DELETED, on);
    notify_changed(e, this);
}

// Typed from 00484580
/// `TESForm::SetEmpty` (Xbox PDB): flag 0x2000; first, unless the save/load stub answers true, the
/// form's slot 0x48 (set) or 0x4c (clear) with 0x200000.
pub fn set_empty(e: &mut Engine, this: u32, on: bool) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    if !e.call(SAVE_LOAD_STUB, &args![save_load]).bool() {
        let slot = if on { 0x48 } else { 0x4c };
        e.vcall(this, slot, &args![0x20_0000u32]);
    }
    set_form_flag(e, this, 0x2000, on);
}

// Typed from 00484610
/// Flag 0x2000 set or cleared.
pub fn set_flag_2000(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x2000, on);
}

// Typed from 00484650
/// Flag 0x800000, then the changed call.
pub fn set_flag_800000(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x80_0000, on);
    notify_changed(e, this);
}

// Typed from 004846a0
/// `TESForm::SetDestructible` (Xbox PDB): flag 0x1000000.
pub fn set_destructible(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x100_0000, on);
}

// Typed from 004846e0
/// Flag 0x4000000, then the changed call.
pub fn set_flag_4000000(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x400_0000, on);
    notify_changed(e, this);
}

// Typed from 00484730
/// Flag 2, with the form array kept in step: clearing a set flag takes the form's entry out of the
/// array; setting a clear flag on a form that is not temporary adds it to an existing array that
/// does not list it yet.
pub fn set_listed_flag(e: &mut Engine, this: u32, on: bool) {
    if on {
        if !e.call(IS_FLAG_2, &args![this]).bool()
            && !is_temporary(e, this)
            && e.mem.u32(FORM_LIST) != 0
            && find_in_form_list(e, this).is_none()
        {
            let list = e.mem.u32(FORM_LIST);
            with_item(e, this, |e, slot| {
                e.call(FORM_LIST_ADD, &args![list, slot]);
            });
        }
        set_form_flag(e, this, 2, true);
    } else {
        if e.call(IS_FLAG_2, &args![this]).bool() && e.mem.u32(FORM_LIST) != 0 {
            if let Some(index) = find_in_form_list(e, this) {
                let list = e.mem.u32(FORM_LIST);
                e.call(FORM_LIST_REMOVE_AT, &args![list, index]);
            }
        }
        set_form_flag(e, this, 2, false);
    }
}

// Typed from 00484860
/// Flag 0x400, then the changed call.
pub fn set_flag_400(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x400, on);
    notify_changed(e, this);
}

// Typed from 004848b0
/// Flag 0x40, then the changed call.
pub fn set_flag_40(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x40, on);
    notify_changed(e, this);
}

// Typed from 00484900
/// Flag 0x10000, then the changed call.
pub fn set_flag_10000(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x1_0000, on);
    notify_changed(e, this);
}

// Typed from 00484950
/// Flag 0x20000 set or cleared.
pub fn set_flag_20000(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x2_0000, on);
}

// Typed from 00484990
/// Flag 0x100000 set or cleared.
pub fn set_flag_100000(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x10_0000, on);
}

// Typed from 004849d0
/// Flag 0x200 set or cleared.
pub fn set_flag_200(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x200, on);
}

// Typed from 00484a10
/// The exe's second copy of [`set_flag_40`]: flag 0x40, then the changed call.
pub fn set_flag_40_again(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x40, on);
    notify_changed(e, this);
}

// Typed from 00484a70
/// `TESForm::SetFireOff` (Xbox PDB): flag 0x80.
pub fn set_fire_off(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x80, on);
}

// Typed from 00484ab0
/// Flag 0x8 set or cleared.
pub fn set_flag_8(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x8, on);
}

// Typed from 00484af0
/// `TESForm::SetDisabled` (Xbox PDB): flag 0x800, then the changed call.
pub fn set_disabled(e: &mut Engine, this: u32, on: bool) {
    set_form_flag(e, this, 0x800, on);
    notify_changed(e, this);
}

// Typed from 00484b40
/// Whether a form id is in `1..=0x7ff` (cdecl).
#[inline]
pub fn id_in_low_range(_e: &mut Engine, form_id: u32) -> bool {
    form_id != 0 && form_id <= 0x7ff
}

/// The two words the change calls pass: `extra`, then `changes` stored by `008c71b0` in the word
/// below it.
fn change_words(e: &mut Engine, changes: u32, extra: u32) -> (u32, u32) {
    e.with_stack(8, |e, words| {
        e.mem.set_u32(words.addr().wrapping_add(4), extra);
        e.call(STORE_WORD, &args![words, changes]);
        (
            e.mem.u32(words.addr()),
            e.mem.u32(words.addr().wrapping_add(4)),
        )
    })
}

// Typed from 00484b60
/// A change of `changes` on the form, to `BGSSaveLoadGame`'s add-change with a last word 0.
pub fn add_change(e: &mut Engine, this: u32, changes: u32) {
    let (changes, extra) = change_words(e, changes, 0);
    let bgs = e.mem.u32(BGS_SAVE_LOAD_GAME);
    e.call(BGS_ADD_CHANGE, &args![bgs, this, changes, extra]);
}

// Typed from 00484b90
/// `TESForm::ForceChange` (Xbox PDB): [`add_change`] with a last word 1.
pub fn force_change(e: &mut Engine, this: u32, changes: u32) {
    let (changes, extra) = change_words(e, changes, 1);
    let bgs = e.mem.u32(BGS_SAVE_LOAD_GAME);
    e.call(BGS_ADD_CHANGE, &args![bgs, this, changes, extra]);
}

// Typed from 00484bc0
/// The form and `changes` to the add-change's sibling (`0084a780`).
pub fn change_sibling(e: &mut Engine, this: u32, changes: u32) {
    let value = e.with_stack(4, |e, word| {
        e.call(STORE_WORD, &args![word, changes]);
        e.mem.u32(word.addr())
    });
    let bgs = e.mem.u32(BGS_SAVE_LOAD_GAME);
    e.call(BGS_CHANGE_SIBLING, &args![bgs, this, value]);
}

// Typed from 00484bf0
/// The bytes the form saves under `flags`: 4 (its flags word) when bit 0 is set, else 0.
#[inline]
pub fn flags_save_size(_e: &mut Engine, _this: u32, flags: u32) -> u16 {
    if flags & 1 != 0 {
        4
    } else {
        0
    }
}

// Typed from 00484c20
/// The form's flags word (+8, 4 bytes) saved with `SaveGameDataOLD` when bit 0 of `flags` is set.
pub fn save_flags(e: &mut Engine, this: u32, flags: u32) {
    if flags & 1 != 0 {
        typed_call!(
            FAMILY,
            0x0048_4ce0,
            save_game_data_old(e, this, this.wrapping_add(8), 4)
        );
    }
}

// Typed from 00484c50
/// `TESForm::LoadGame` (Xbox PDB): with bit 0 of `flags`, 4 bytes read with `LoadGameDataOLD` and
/// merged into the flags: a form that casts to `TESObjectREFR` takes the bits `0x912860`, any other
/// the bits `0x40000c20`. The second word is not read.
pub fn load_game(e: &mut Engine, this: u32, flags: u32, _unused: u32) {
    if flags & 1 == 0 {
        return;
    }
    let saved = e.with_stack(4, |e, word| {
        typed_call!(
            FAMILY,
            0x0048_4d00,
            load_game_data_old(e, this, word.addr(), 4)
        );
        e.mem.u32(word.addr())
    });
    let reference = e
        .call(
            RT_DYNAMIC_CAST,
            &args![this, 0u32, TYPE_TES_FORM, TYPE_TES_OBJECT_REFR, 0u32],
        )
        .u32();
    let at = this.wrapping_add(8);
    let current = e.mem.u32(at);
    let mask = if reference != 0 {
        LOAD_GAME_REFERENCE_FLAGS
    } else {
        LOAD_GAME_FORM_FLAGS
    };
    e.mem.set_u32(at, current & !mask | saved & mask);
}

// Typed from 00484ce0
/// `TESForm::SaveGameDataOLD` (Xbox PDB): `(buffer, size)` to `TESSaveLoadGame`'s.
pub fn save_game_data_old(e: &mut Engine, _this: u32, buffer: u32, size: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    e.call(SAVE_LOAD_SAVE_DATA, &args![save_load, buffer, size]);
}

// Typed from 00484d00
/// `TESForm::LoadGameDataOLD` (Xbox PDB): `(buffer, size)` to `TESSaveLoadGame`'s.
pub fn load_game_data_old(e: &mut Engine, _this: u32, buffer: u32, size: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    e.call(SAVE_LOAD_LOAD_DATA, &args![save_load, buffer, size]);
}

// Typed from 00484d20
/// `TESForm::SaveNumericID` (Xbox PDB): its two words to `TESSaveLoadGame`'s.
pub fn save_numeric_id(e: &mut Engine, _this: u32, first: u32, second: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    e.call(SAVE_LOAD_SAVE_NUMERIC_ID, &args![save_load, first, second]);
}

// Typed from 00484d40
/// `TESForm::LoadNumericID` (Xbox PDB): its two words to `TESSaveLoadGame`'s.
pub fn load_numeric_id(e: &mut Engine, _this: u32, first: u32, second: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    e.call(SAVE_LOAD_LOAD_NUMERIC_ID, &args![save_load, first, second]);
}

// Typed from 00484d60
/// The form's flags word (+8, 4 bytes) written to `buffer` when bit 0 of its save kind is set.
pub fn write_flags(e: &mut Engine, this: u32, buffer: u32) {
    if buffer_saves_flags(e, buffer) {
        e.call(
            SAVE_BUFFER_WRITE,
            &args![buffer, this.wrapping_add(8), 4u32, 0u32],
        );
    }
}

// Typed from 00484da0
/// The load side of [`write_flags`]: with bit 0 of the save kind, 4 bytes read and merged into the
/// flags with [`load_game`]'s masks, chosen by the form's slot 0xf0.
pub fn read_flags(e: &mut Engine, this: u32, buffer: u32) {
    if !buffer_saves_flags(e, buffer) {
        return;
    }
    let saved = e.with_stack(4, |e, word| {
        e.call(LOAD_BUFFER_READ, &args![buffer, word, 4u32]);
        e.mem.u32(word.addr())
    });
    let at = this.wrapping_add(8);
    let current = e.mem.u32(at);
    let merged = if e.vcall(this, SLOT_IS_REFERENCE, &[]).bool() {
        // JIP LN's G45 (a fix row): its handler answers the mask of the saved flags a reference
        // takes (JIP's adds 0x400000). Words: the exe's mask.
        let taken = if e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0 {
            e.site(0x0048_4e03, &[LOAD_GAME_REFERENCE_FLAGS])
                .unwrap_or(LOAD_GAME_REFERENCE_FLAGS)
        } else {
            LOAD_GAME_REFERENCE_FLAGS
        };
        current & !LOAD_GAME_REFERENCE_FLAGS | saved & taken
    } else {
        current & !LOAD_GAME_FORM_FLAGS | saved & LOAD_GAME_FORM_FLAGS
    };
    e.mem.set_u32(at, merged);
}

// Typed from 00484e40
/// Whether the form's id (+0x0c) is in `1..=0x7ff`.
pub fn has_low_id(e: &mut Engine, this: u32) -> bool {
    let id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, this));
    typed_call!(FAMILY, 0x0048_4b40, id_in_low_range(e, id))
}

// Typed from 00484e60
/// `TESForm::GetFile` (Xbox PDB): the file at position `index` among the form's files that are not
/// null (0 is the first); the last such file when there are fewer, or for `index` -1; 0 without any.
pub fn get_file(e: &mut Engine, this: u32, index: i32) -> u32 {
    let mut node = list_first(e, this);
    let mut position = 0i32;
    let mut found = 0u32;
    while node != 0 {
        let file = list_item(e, node);
        node = list_next(e, node);
        if file == 0 {
            continue;
        }
        found = file;
        if index == -1 {
            continue;
        }
        position = position.wrapping_add(1);
        if position > index {
            break;
        }
    }
    found
}

// Typed from 00484ee0
/// `TESForm::GetOwnerMaster` (Xbox PDB): the last file of the form's list that
/// `TESFile::GetMaster` accepts, up to the list's empty node; 0 for none.
pub fn get_owner_master(e: &mut Engine, this: u32) -> u32 {
    let mut node = list_first(e, this);
    let mut master = 0u32;
    while node != 0 && !list_node_empty(e, node) {
        let file = list_item(e, node);
        node = list_next(e, node);
        if e.call(FILE_IS_MASTER, &args![file]).bool() {
            master = file;
        }
    }
    master
}

// Typed from 00484f50
/// `TESForm::SetFile` (Xbox PDB): `file` (the end of its chain, `00473c70`, when that is not 0)
/// recorded in the form's source file list. No file: the last node holding one is popped (the first
/// node when none holds one). A master (`GetMaster`): the files that are not masters move out from
/// in front of it, then it is appended. Another file: nothing when the list holds it already, else
/// appended after the last node holding a file, or pushed at the front of a list with none.
pub fn set_file(e: &mut Engine, this: u32, file: u32) {
    let mut file = file;
    let root = if file == 0 {
        0
    } else {
        e.call(FILE_ROOT, &args![file]).u32()
    };
    if root != 0 {
        file = root;
    }
    if file == 0 {
        let mut node = list_first(e, this);
        let mut last = 0u32;
        while node != 0 {
            if list_item(e, node) != 0 {
                last = node;
            }
            node = list_next(e, node);
        }
        let pop = if last != 0 { last } else { list_first(e, this) };
        e.call(LIST_POP_NODE, &args![pop]);
    } else if e.call(FILE_IS_MASTER, &args![file]).bool() {
        let mut node = list_first(e, this);
        let mut last_master = 0u32;
        while node != 0 && !list_node_empty(e, node) {
            let current = list_item(e, node);
            if e.call(FILE_IS_MASTER, &args![current]).bool() {
                last_master = node;
                node = list_next(e, node);
            } else if last_master != 0 {
                with_item(e, current, |e, slot| {
                    e.call(LIST_REMOVE_ITEM, &args![last_master, slot]);
                });
                node = list_next(e, last_master);
            } else {
                e.call(LIST_POP_NODE, &args![node]);
            }
        }
        let first = list_first(e, this);
        with_item(e, file, |e, slot| {
            e.call(LIST_APPEND, &args![first, slot]);
        });
    } else {
        let mut node = list_first(e, this);
        let mut last = 0u32;
        while node != 0 {
            let current = list_item(e, node);
            if current != 0 {
                last = node;
                if current == file {
                    return;
                }
            }
            node = list_next(e, node);
        }
        if last != 0 {
            with_item(e, file, |e, slot| {
                e.call(LIST_APPEND, &args![last, slot]);
            });
        } else {
            let first = list_first(e, this);
            with_item(e, file, |e, slot| {
                e.call(LIST_PUSH_FRONT, &args![first, slot]);
            });
        }
    }
}

// Typed from 00485110
/// `TESForm::LoadForm` (Xbox PDB): `file.LoadForm(this)`.
pub fn load_form(e: &mut Engine, this: u32, file: u32) {
    e.call(FILE_LOAD_FORM, &args![file, this]);
}

/// The base `Copy`/`Compare`'s log line for `other`: its type's name, then its name (slot 0x130).
fn log_no_override(e: &mut Engine, message: u32, other: u32) {
    let type_name = e.call(FORM_TYPE_NAME, &args![other]).u32();
    let name = e.vcall(other, SLOT_NAME, &[]).u32();
    e.call(LOG_MESSAGE, &args![message, name, type_name]);
}

// Typed from 00485130
/// `TESForm::Copy` (Xbox PDB), the base version: logs that `other`'s type has no copy.
pub fn copy(e: &mut Engine, _this: u32, other: u32) {
    log_no_override(e, COPY_MESSAGE, other);
}

// Typed from 00485170
/// `TESForm::Compare` (Xbox PDB), the base version: logs that `other`'s type has no compare;
/// false.
pub fn compare(e: &mut Engine, _this: u32, other: u32) -> bool {
    log_no_override(e, COMPARE_MESSAGE, other);
    false
}

/// Both forms' `FormComponentCollection`s built on the stack (this form's first), then `f` on the
/// two collections.
fn with_both_collections<R>(
    e: &mut Engine,
    this: u32,
    other: u32,
    f: impl FnOnce(&mut Engine, u32, u32) -> R,
) -> R {
    e.with_stack(COMPONENT_COLLECTION_SIZE, |e, mine| {
        e.call(COMPONENTS_CONSTRUCT, &args![mine]);
        e.with_stack(COMPONENT_COLLECTION_SIZE, |e, theirs| {
            e.call(COMPONENTS_CONSTRUCT, &args![theirs]);
            e.call(COMPONENTS_INIT, &args![mine, this]);
            e.call(COMPONENTS_INIT, &args![theirs, other]);
            f(e, mine.addr(), theirs.addr())
        })
    })
}

// Typed from 004851b0
/// `TESForm::CopyAllComponents` (Xbox PDB): `collection(this).Copy(collection(other))`.
pub fn copy_all_components(e: &mut Engine, this: u32, other: u32) {
    with_both_collections(e, this, other, |e, mine, theirs| {
        e.call(COMPONENTS_COPY, &args![mine, theirs]);
    });
}

// Typed from 00485270
/// `TESForm::CompareAllComponents` (Xbox PDB): 1 for no `other`; else
/// `collection(this).Compare(collection(other))`.
pub fn compare_all_components(e: &mut Engine, this: u32, other: u32) -> u8 {
    if other == 0 {
        return 1;
    }
    with_both_collections(e, this, other, |e, mine, theirs| {
        e.call(COMPONENTS_COMPARE, &args![mine, theirs]).u8()
    })
}

// Typed from 00485340
/// The form state of `other` (a base component) taken over when it casts to a form: its flag 2
/// through slot 0xc8 when that differs from this form's, its type, and its flags but for 0x4000,
/// which stays this form's.
pub fn copy_from(e: &mut Engine, this: u32, other: u32) {
    let source = component_to_form(e, other);
    if source == 0 {
        return;
    }
    let mine = e.call(IS_FLAG_2, &args![this]).u8();
    let theirs = e.call(IS_FLAG_2, &args![source]).u8();
    if mine != theirs {
        let theirs = e.call(IS_FLAG_2, &args![source]).u8();
        e.vcall(this, SLOT_SET_FLAG_2, &args![u32::from(theirs)]);
    }
    let form_type = e.call(FORM_TYPE, &args![source]).u8();
    e.mem.set_u8(this.wrapping_add(4), form_type);
    let flags = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, source));
    let at = this.wrapping_add(8);
    let kept = e.mem.u32(at) & FLAG_TEMPORARY;
    e.mem.set_u32(at, flags & !FLAG_TEMPORARY | kept);
}

// Typed from 004853e0
/// Whether this form differs from `other` (a base component): true when it is not a form, the
/// types differ, either has a name key (`00474cb0`) and the keys or the names (slot 0x130) differ,
/// or the flags differ outside 0x4000 and 0xb.
pub fn differs_from(e: &mut Engine, this: u32, other: u32) -> bool {
    let source = component_to_form(e, other);
    if source == 0 {
        return true;
    }
    let form_type = u32::from(e.mem.u8(this.wrapping_add(4)));
    if form_type != e.call(FORM_TYPE, &args![source]).u32() {
        return true;
    }
    let mine_key = e.call(FORM_NAME_KEY, &args![this]).u32();
    if mine_key != 0 || e.call(FORM_NAME_KEY, &args![source]).u32() != 0 {
        let mine_key = e.call(FORM_NAME_KEY, &args![this]).u32();
        let theirs_key = e.call(FORM_NAME_KEY, &args![source]).u32();
        if mine_key != theirs_key {
            return true;
        }
        let theirs_name = e.vcall(source, SLOT_NAME, &[]).u32();
        let mine_name = e.vcall(this, SLOT_NAME, &[]).u32();
        if e.call(STRING_COMPARE, &args![mine_name, theirs_name]).u32() != 0 {
            return true;
        }
    }
    let mine =
        typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, this)) & COMPARE_FLAGS_MASK;
    let theirs =
        typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, source)) & COMPARE_FLAGS_MASK;
    mine != theirs
}

/// Whether `record` is a group record (its first word the group tag).
fn is_group(e: &mut Engine, record: u32) -> bool {
    record != 0 && e.mem.u32(record) == e.mem.u32(GROUP_TAG)
}

// Typed from 004854e0
/// Whether `record` is the top-level group (group type 0) whose label is this form's type string.
/// The two further words are ignored.
pub fn is_own_top_group(e: &mut Engine, this: u32, record: u32, _first: u32, _second: u32) -> bool {
    if !is_group(e, record) || e.mem.u32(record.wrapping_add(0xc)) != 0 {
        return false;
    }
    let tag = form_type_tag(e, this);
    e.mem.u32(record.wrapping_add(8)) == tag
}

// Typed from 00485530
/// `record` filled as the top-level group of this form's type (the group tag, the type string as
/// label, group type 0, the rest 0), unless `record` is 0 or `flag` is not.
pub fn fill_top_group(e: &mut Engine, this: u32, record: u32, flag: u32) {
    if record == 0 || flag != 0 {
        return;
    }
    let group_tag = e.mem.u32(GROUP_TAG);
    e.mem.set_u32(record, group_tag);
    e.mem.set_u32(record.wrapping_add(0xc), 0);
    let tag = form_type_tag(e, this);
    e.mem.set_u32(record.wrapping_add(8), tag);
    e.mem.set_u32(record.wrapping_add(4), 0);
    e.mem.set_u32(record.wrapping_add(0x10), 0);
    e.mem.set_u16(record.wrapping_add(0x14), 0);
    e.mem.set_u16(record.wrapping_add(0x16), 0);
}

// Typed from 004855a0
/// `TESForm::StartForm` (Xbox PDB): unless temporary, a new 0x18 byte record header in the save
/// buffer: the flags (only `0x30032fe0`'s bits for a type other than 1), the type string as tag,
/// the form id, size 0, the record version (`00470bb0`), no version-control data; then the form's
/// slot 0xdc.
pub fn start_form(e: &mut Engine, this: u32) {
    if is_temporary(e, this) {
        return;
    }
    e.mem.set_u32(SAVE_BUFFER_SIZE, 0x18);
    let size = e.mem.u32(SAVE_BUFFER_SIZE);
    let allocated = e.call(OPERATOR_NEW, &args![size]).u32();
    e.mem.set_u32(SAVE_BUFFER, allocated);
    let header = e.mem.u32(SAVE_BUFFER);
    let mut flags = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, this));
    e.mem.set_u32(header.wrapping_add(8), flags);
    if e.call(FORM_TYPE, &args![this]).u32() != 1 {
        flags &= START_FORM_FLAGS_MASK;
        e.mem.set_u32(header.wrapping_add(8), flags);
    }
    let tag = form_type_tag(e, this);
    e.mem.set_u32(header, tag);
    let id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, this));
    e.mem.set_u32(header.wrapping_add(0xc), id);
    e.mem.set_u32(header.wrapping_add(4), 0);
    let version = e.call(RECORD_VERSION, &args![]).u16();
    e.mem.set_u16(header.wrapping_add(0x14), version);
    e.mem.set_u32(header.wrapping_add(0x10), 0);
    e.mem.set_u16(header.wrapping_add(0x16), 0);
    e.vcall(this, SLOT_AFTER_START, &[]);
}

// Typed from 00485680
/// `TESForm::CloseForm` (Xbox PDB): unless temporary, the record's data size (the buffer's size
/// less the header) in the header, which a big-endian target then swaps (`FORM::Endian`).
pub fn close_form(e: &mut Engine, this: u32) {
    if is_temporary(e, this) {
        return;
    }
    let header = e.mem.u32(SAVE_BUFFER);
    let size = e.mem.u32(SAVE_BUFFER_SIZE);
    e.mem
        .set_u32(header.wrapping_add(4), size.wrapping_sub(0x18));
    if is_big_endian(e) {
        e.call(FORM_ENDIAN, &args![header]);
    }
}

// Typed from 004856d0
/// `TESForm::AddChunk` (Xbox PDB) without data: a chunk of this tag and size 0 (cdecl).
pub fn add_chunk(e: &mut Engine, tag: u32) {
    typed_call!(FAMILY, 0x0048_5990, add_chunk_data(e, tag, 0, 0));
}

// Typed from 004856f0
/// `TESForm::AddChunkArray` (Xbox PDB): a chunk of `size` bytes of `data` (cdecl).
pub fn add_chunk_array(e: &mut Engine, tag: u32, data: u32, size: u32) {
    typed_call!(FAMILY, 0x0048_5990, add_chunk_data(e, tag, data, size));
}

// Typed from 00485710
/// `TESForm::AddChunkArray_ov2` (Xbox PDB): a chunk of `count` 32-bit values (cdecl).
pub fn add_chunk_array_words(e: &mut Engine, tag: u32, data: u32, count: u32) {
    typed_call!(FAMILY, 0x0048_5750, add_chunk_array32(e, tag, data, count));
}

// Typed from 00485730
/// A chunk of `count` 16-bit values (cdecl), through [`add_chunk_array16`].
pub fn add_chunk_array_halves(e: &mut Engine, tag: u32, data: u32, count: u32) {
    typed_call!(FAMILY, 0x0048_5820, add_chunk_array16(e, tag, data, count));
}

/// A chunk of `count` values of `width` bytes: on a big-endian target the values are copied to a
/// new block and each swapped (`swap`) before the chunk is written, and the block freed after.
fn add_chunk_values(e: &mut Engine, tag: u32, data: u32, count: u32, width: u32, swap: u32) {
    let size = count.wrapping_mul(width);
    if !is_big_endian(e) {
        typed_call!(FAMILY, 0x0048_5990, add_chunk_data(e, tag, data, size));
        return;
    }
    let request = count.saturating_mul(width);
    let copy = e.call(OPERATOR_NEW, &args![request]).u32();
    e.call(MEMCPY, &args![copy, data, size]);
    for i in 0..count {
        e.call(swap, &args![copy.wrapping_add(i.wrapping_mul(width)), 0u32]);
    }
    typed_call!(FAMILY, 0x0048_5990, add_chunk_data(e, tag, copy, size));
    e.call(OPERATOR_DELETE, &args![copy]);
}

// Typed from 00485750
/// `TESForm::AddChunkArray32` (Xbox PDB): a chunk of `count` 32-bit values, swapped on a big-endian
/// target (cdecl).
pub fn add_chunk_array32(e: &mut Engine, tag: u32, data: u32, count: u32) {
    add_chunk_values(e, tag, data, count, 4, SWAP_WORD);
}

// Typed from 00485820
/// A chunk of `count` 16-bit values, swapped on a big-endian target (cdecl).
pub fn add_chunk_array16(e: &mut Engine, tag: u32, data: u32, count: u32) {
    add_chunk_values(e, tag, data, count, 2, SWAP_HALF);
}

// Typed from 004858f0
/// A one-byte chunk: the low byte of `value`, by the address of the stack word holding it (cdecl).
pub fn add_chunk_byte(e: &mut Engine, tag: u32, value: u32) {
    with_item(e, value, |e, slot| {
        typed_call!(FAMILY, 0x0048_5990, add_chunk_data(e, tag, slot, 1));
    });
}

// Typed from 00485910
/// A chunk of one 32-bit value, swapped first on a big-endian target (cdecl).
pub fn add_chunk_word(e: &mut Engine, tag: u32, value: u32) {
    with_item(e, value, |e, slot| {
        if is_big_endian(e) {
            e.call(SWAP_WORD, &args![slot, 0u32]);
        }
        typed_call!(FAMILY, 0x0048_5990, add_chunk_data(e, tag, slot, 4));
    });
}

// Typed from 00485950
/// `TESForm::AddChunk_ov2` (Xbox PDB): a chunk of one 16-bit value (the stack word's low half),
/// swapped first on a big-endian target (cdecl).
pub fn add_chunk_half(e: &mut Engine, tag: u32, value: u32) {
    with_item(e, value, |e, slot| {
        if is_big_endian(e) {
            e.call(SWAP_HALF, &args![slot, 0u32]);
        }
        typed_call!(FAMILY, 0x0048_5990, add_chunk_data(e, tag, slot, 2));
    });
}

// Typed from 00485990
/// `TESForm::__AddChunkData` (Xbox PDB): a chunk appended to the save buffer: the tag, a `u16`
/// size (0 for a chunk larger than 0xffff, after an `"XXXX"` chunk holding the real size) and
/// `size` bytes of `data`. The buffer grows by `size + 6` (`0042f5d0`); a big-endian target swaps
/// the chunk's header (cdecl).
pub fn add_chunk_data(e: &mut Engine, tag: u32, data: u32, size: u32) {
    let mut size16 = low_half(size);
    if size > 0xffff {
        typed_call!(
            FAMILY,
            0x0048_5910,
            add_chunk_word(e, OVERSIZE_CHUNK_TAG, size)
        );
        size16 = 0;
    }
    let old = e.mem.u32(SAVE_BUFFER_SIZE);
    e.mem
        .set_u32(SAVE_BUFFER_SIZE, old.wrapping_add(size).wrapping_add(6));
    let buffer = e.mem.u32(SAVE_BUFFER);
    let new_size = e.mem.u32(SAVE_BUFFER_SIZE);
    let grown = e.call(BUFFER_REALLOC, &args![buffer, new_size]).u32();
    e.mem.set_u32(SAVE_BUFFER, grown);
    let header = e.mem.u32(SAVE_BUFFER).wrapping_add(old);
    e.mem.set_u32(header, tag);
    e.mem.set_u16(header.wrapping_add(4), size16);
    if is_big_endian(e) {
        e.call(SWAP_CHUNK_HEADER, &args![header]);
    }
    e.call(MEMCPY, &args![header, header, 6u32]);
    let body = e.mem.u32(SAVE_BUFFER).wrapping_add(old).wrapping_add(6);
    e.call(MEMCPY, &args![body, data, size]);
}

// Typed from 00485a70
/// The size of the chunk header `chunk` grown by `amount`, and the save buffer with it
/// (`0042f5d0`). False, with nothing changed, when the size and `amount` are both 0 or the new size
/// would pass 0xffff. A big-endian target swaps the header around the change.
pub fn grow_chunk(e: &mut Engine, _this: u32, chunk: u32, amount: u16) -> bool {
    let swap = is_big_endian(e);
    if swap {
        e.call(SWAP_CHUNK_HEADER, &args![chunk]);
    }
    let current = u32::from(e.mem.u16(chunk.wrapping_add(4)));
    let amount = u32::from(amount);
    if (current == 0 && amount == 0) || current + amount > 0xffff {
        if swap {
            e.call(SWAP_CHUNK_HEADER, &args![chunk]);
        }
        return false;
    }
    e.mem
        .set_u16(chunk.wrapping_add(4), low_half(current + amount));
    if swap {
        e.call(SWAP_CHUNK_HEADER, &args![chunk]);
    }
    let size = e.mem.u32(SAVE_BUFFER_SIZE).wrapping_add(amount);
    e.mem.set_u32(SAVE_BUFFER_SIZE, size);
    let buffer = e.mem.u32(SAVE_BUFFER);
    let grown = e.call(BUFFER_REALLOC, &args![buffer, size]).u32();
    e.mem.set_u32(SAVE_BUFFER, grown);
    true
}

// Typed from 00485b30
/// `TESForm::FreeFormBuffer` (Xbox PDB): the save buffer freed and its pointer cleared.
pub fn free_form_buffer(e: &mut Engine, _this: u32) {
    let buffer = e.mem.u32(SAVE_BUFFER);
    e.call(OPERATOR_DELETE, &args![buffer]);
    e.mem.set_u32(SAVE_BUFFER, 0);
}

// Typed from 00485b60
/// The low 24 bits of the form id, with bit 24 set when the master list (file +0x3ec) of the form's
/// first file is not empty (a form without a file reads the "file" at 0).
pub fn packed_id(e: &mut Engine, this: u32) -> u32 {
    let id = typed_call!(FAMILY, 0x0048_5bc0, low_id(e, this));
    let file = typed_call!(FAMILY, 0x0048_4e60, get_file(e, this, 0i32));
    let masters = e.call(FILE_MASTER_LIST, &args![file]).u32();
    let empty = list_node_empty(e, masters);
    u32::from(!empty) << 24 | id
}

// Typed from 00485bc0
/// The low 24 bits of the form id (+0x0c).
#[inline]
pub fn low_id(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(0xc)) & 0x00ff_ffff
}

// Typed from 00485be0
/// Whether the low 24 bits of `form_id` are those of the form's id.
pub fn same_low_id(e: &mut Engine, this: u32, form_id: u32) -> bool {
    form_id & 0x00ff_ffff == typed_call!(FAMILY, 0x0048_5bc0, low_id(e, this))
}

// Typed from 00485c10
/// `TESForm::SetFormID` (Xbox PDB): a new id that differs, on a form that is not temporary, takes
/// the old id out of the form map (and the data handler when `remove_from_handler` is set) and
/// maps the new one (if not 0) to the form; the id is stored in every case but an unchanged one.
pub fn set_form_id(e: &mut Engine, this: u32, new_id: u32, remove_from_handler: bool) {
    let id_at = this.wrapping_add(0xc);
    let old_id = e.mem.u32(id_at);
    if new_id == old_id {
        return;
    }
    if !is_temporary(e, this) {
        if old_id != 0 {
            let map = e.mem.u32(FORM_MAP);
            e.call(MAP_REMOVE_AT, &args![map, old_id]);
        }
        if remove_from_handler && e.mem.u32(id_at) != 0 {
            let id = e.mem.u32(id_at);
            let handler = e.mem.u32(DATA_HANDLER);
            e.call(DATA_HANDLER_REMOVE_ID, &args![handler, id]);
        }
        if new_id != 0 {
            let map = e.mem.u32(FORM_MAP);
            e.call(MAP_SET_AT, &args![map, new_id, this]);
        }
    }
    e.mem.set_u32(id_at, new_id);
}

// Typed from 00485d50
/// `TESForm::AddCompileIndex` (Xbox PDB), cdecl `(id pointer, file)`: while the save/load stub is
/// true the id is remapped by `TESSaveLoadGame` (`00857bf0`); otherwise, unless the id is in the
/// low range or there is no file, its top byte becomes the compile index of the file its top byte
/// plus one names in `file` (`GetIndexFile`), or of `file` itself when it names none.
pub fn add_compile_index(e: &mut Engine, id: u32, file: u32) {
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    if e.call(SAVE_LOAD_STUB, &args![save_load]).bool() {
        let old = e.mem.u32(id);
        let remapped = e.call(SAVE_LOAD_REMAP_ID, &args![save_load, old]).u32();
        e.mem.set_u32(id, remapped);
        return;
    }
    let old = e.mem.u32(id);
    if typed_call!(FAMILY, 0x0048_4b40, id_in_low_range(e, old)) || file == 0 {
        return;
    }
    let indexed = e
        .call(
            FILE_GET_INDEX_FILE,
            &args![file, (old >> 24).wrapping_add(1)],
        )
        .u32();
    let source = if indexed != 0 { indexed } else { file };
    let index = u32::from(e.call(FILE_COMPILE_INDEX, &args![source]).u8());
    let current = e.mem.u32(id);
    e.mem.set_u32(id, index << 24 | current & 0x00ff_ffff);
}

// ---- the constructor, the record compression, the component data, the copier, the maps ----
/// Byte: the form-type table has been checked and packed.
const FORM_TABLE_CHECKED: u32 = 0x011c_54b8;
const FORM_ENUM_COUNT: u32 = 0x79;
const BASE_CONSTRUCT: u32 = 0x0040_2d30;
const SIMPLE_LIST_CONSTRUCT: u32 = 0x0096_a2d0;
const DATA_HANDLER_FLAG: u32 = 0x0048_2f20;
const DATA_HANDLER_GET_NEXT_ID: u32 = 0x0046_9800;
const DISABLE_WARNING_COUNT: u32 = 0x0043_b2b0;
const FORM_ENUM_ERROR_ORDER: u32 = 0x0101_c748;
const FORM_ENUM_ERROR_DUPLICATE: u32 = 0x0101_c6e0;
const FORM_ENUM_FIX_MESSAGE: u32 = 0x0101_c6a0;
const ZLIB_VERSION: u32 = 0x0101_a29c;
const ZLIB_INIT_ERROR: u32 = 0x0101_c8f4;
const ZLIB_DEFLATE_ERROR: u32 = 0x0101_c8cc;
const DEFLATE_INIT: u32 = 0x00b4_6360;
const DEFLATE: u32 = 0x00b4_6b70;
const DEFLATE_END: u32 = 0x00b4_7370;
/// The form components the component data casts a form to (RTTI type descriptors), in the order
/// the code lists them: uses, value, health, weight, quality, attack damage, attributes, clip
/// rounds.
const COMPONENT_TYPES: [u32; 8] = [
    0x0118_6b88,
    0x0118_6b6c,
    0x0118_6c3c,
    0x0118_3274,
    0x0118_6bdc,
    0x0118_6c90,
    0x0118_6c74,
    0x0118_68d4,
];
/// The chunk tag `"DATA"`.
const DATA_CHUNK_TAG: u32 = 0x4154_4144;
const SAVE_DATA_TOO_LARGE_MESSAGE: u32 = 0x0101_ca2c;
const DETAILED_STRING_FORMAT: u32 = 0x0101_ca60;
const SAVE_BUFFER_SIZE_GETTER: u32 = 0x0047_3080;
const SAVE_BUFFER_GETTER: u32 = 0x0047_3070;
/// The one-byte component getter (the same body as the form type's) and its setter (JIP LN may
/// replace the setter).
const COMPONENT_BYTE_GET: u32 = FORM_TYPE;
const COMPONENT_BYTE_SET: u32 = 0x004f_15a0;
const VALUE_SET: u32 = 0x0048_e960;
const HEALTH_SET: u32 = 0x006e_cd40;
const WEIGHT_GET: u32 = 0x006b_9130;
const WEIGHT_SET: u32 = 0x004f_5d90;
const QUALITY_GET: u32 = 0x004f_1540;
const ATTACK_DAMAGE_SET: u32 = 0x0047_ff50;
const SLOT_ATTACK_DAMAGE_GET: u32 = 0x10;
const ATTRIBUTES_SIZE: u32 = 0x009d_2510;
const ATTRIBUTES_SAVE: u32 = 0x0048_0100;
const ATTRIBUTES_LOAD: u32 = 0x0048_0130;
const FILE_CHUNK_SIZE: u32 = 0x0040_1660;
const FILE_READ_CHUNK: u32 = 0x0047_2890;
const FILE_IS_BIG_ENDIAN: u32 = 0x0040_1680;
const CREATE_FORM_OF_TYPE: u32 = 0x0046_5110;
const FORM_NAME_GETTER: u32 = 0x0040_1280;
const DEFAULT_NAME: u32 = 0x0101_1584;
const SLOT_TAKE_NAME: u32 = 0x134;
const SLOT_COPY_FROM: u32 = 0x108;
const STRING_CONSTRUCT: u32 = 0x0040_c0e0;
const STRING_DESTROY: u32 = 0x0040_37d0;
const FORMAT_STRING: u32 = 0x0040_6f60;
const MEMSET: u32 = 0x0040_3d30;
const ALLOCATE_BUCKETS: u32 = 0x00aa_1070;
const MAP_BASE_DESTROY: u32 = 0x0048_6a60;
const STRING_MAP_BASE_DESTROY: u32 = 0x0048_6c80;
const CHAR_KEY_MAP_BASE_DESTROY: u32 = 0x0048_6dc0;
const NI_T_MAP_DESTROY: u32 = 0x0048_6d60;
const STRING_TEMPLATE_MAP_DESTROY: u32 = 0x0048_6ca0;
const MAP_BASE_VTABLE: u32 = 0x0101_ca98;
const POINTER_MAP_VTABLE: u32 = 0x0101_ca78;
const STRING_MAP_VTABLE: u32 = 0x0101_cab8;
/// `GetFormTypeFromFormString`'s cache: the last result and the last string.
const FORM_TYPE_CACHE_RESULT: u32 = 0x011c_54f0;
const FORM_TYPE_CACHE_KEY: u32 = 0x011c_54f4;

// Typed from 00483370
/// `TESForm::TESForm` (Xbox PDB): the base class and vtable, the source file list, the registries
/// with the first form, the form-type table checked and packed once, type 0, flags 8, id 0; while
/// the data handler exists and its flag byte is clear, the handler's next id (and its current
/// file), an id below 0x800 raised to 0x800; then the form in the form map under its id.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    e.call(BASE_CONSTRUCT, &args![this]);
    e.mem.set_u32(this, TES_FORM_VTABLE);
    e.call(SIMPLE_LIST_CONSTRUCT, &args![this.wrapping_add(0x10)]);
    if e.mem.u32(FORM_COUNT) == 0 {
        typed_call!(FAMILY, 0x0048_3a50, create_registries(e));
    }
    let count = e.mem.u32(FORM_COUNT);
    e.mem.set_u32(FORM_COUNT, count.wrapping_add(1));
    if e.mem.u8(FORM_TABLE_CHECKED) == 0 {
        check_form_enum_table(e);
        e.mem.set_u8(FORM_TABLE_CHECKED, 1);
    }
    e.mem.set_u8(this.wrapping_add(4), 0);
    e.mem.set_u32(this.wrapping_add(8), 8);
    let id_at = this.wrapping_add(0xc);
    e.mem.set_u32(id_at, 0);
    let handler = e.mem.u32(DATA_HANDLER);
    if handler != 0 && !e.call(DATA_HANDLER_FLAG, &args![handler]).bool() {
        // The exe reloads the handler after each call.
        let handler = e.mem.u32(DATA_HANDLER);
        let id = e.call(DATA_HANDLER_GET_NEXT_ID, &args![handler]).u32();
        e.mem.set_u32(id_at, id);
        let handler = e.mem.u32(DATA_HANDLER);
        if typed_call!(FAMILY, 0x0048_35e0, handler_loading_file(e, handler)) != 0 {
            let handler = e.mem.u32(DATA_HANDLER);
            let file = typed_call!(FAMILY, 0x0048_35e0, handler_loading_file(e, handler));
            typed_call!(FAMILY, 0x0048_4f50, set_file(e, this, file));
        }
        if e.mem.u32(id_at) < 0x800 {
            typed_call!(FAMILY, 0x0048_5c10, set_form_id(e, this, 0x800, true));
        }
    }
    let id = e.mem.u32(id_at);
    if id != 0 {
        let map = e.mem.u32(FORM_MAP);
        e.call(MAP_SET_AT, &args![map, id, this]);
    }
    this
}

/// The constructor's one-time check of the form-type table: each type's four characters packed
/// (sign-extended bytes, the first lowest) into the entry's word at +8; an entry whose type byte is
/// not its index, and entries with equal packed strings, reported; the game stopped if any was.
fn check_form_enum_table(e: &mut Engine) {
    let mut failed = false;
    e.call(DISABLE_WARNING_COUNT, &args![1u32]);
    for i in 0..FORM_ENUM_COUNT {
        let entry = FORM_ENUM_TABLE + i * FORM_ENUM_STRIDE;
        let text = e.mem.u32(entry + 4);
        let packed = i32::from(e.mem.i8(text))
            | i32::from(e.mem.i8(text.wrapping_add(1))) << 8
            | i32::from(e.mem.i8(text.wrapping_add(2))) << 16
            | i32::from(e.mem.i8(text.wrapping_add(3))) << 24;
        e.mem.set_i32(entry + 8, packed);
        if u32::from(e.mem.u8(entry)) != i {
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
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words FORM_ENUM_FIX_MESSAGE);
    }
}

// Typed from 00483d70
/// `TESForm::CompressSaveBuffer` (Xbox PDB): the record body after the 0x18 byte header deflated
/// with zlib when it has one and is not compressed yet (flag 0x40000 at +8): a new buffer of the
/// header (flagged, its size the packed size + 4), the body's size and the stream. A big-endian
/// target swaps the header around the change and the size word. A zlib failure is logged and
/// leaves the buffer as it was.
pub fn compress_save_buffer(e: &mut Engine) {
    let buffer = e.mem.u32(SAVE_BUFFER);
    let size = e.mem.u32(SAVE_BUFFER_SIZE);
    if buffer == 0 || size <= 0x18 {
        return;
    }
    if is_big_endian(e) {
        e.call(FORM_ENDIAN, &args![buffer]);
    }
    if e.mem.u32(buffer.wrapping_add(8)) & 0x40000 != 0 {
        if is_big_endian(e) {
            e.call(FORM_ENDIAN, &args![buffer]);
        }
        return;
    }
    let body_size = size - 0x18;
    let body = buffer.wrapping_add(0x18);
    e.with_stack(0x38, |e, stream| {
        let stream = stream.addr();
        // z_stream: next_in +0, avail_in +4, next_out +0x0c, avail_out +0x10, state +0x1c,
        // zalloc +0x20, zfree +0x24, opaque +0x28.
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
        if e.call(DEFLATE, &args![stream, 4u32]).i32() == -2 {
            e.call(LOG_MESSAGE, &args![ZLIB_DEFLATE_ERROR]);
            e.call(OPERATOR_DELETE, &args![packed]);
            return;
        }
        let packed_size = capacity.wrapping_sub(e.mem.u32(stream + 0x10));
        let total = packed_size.wrapping_add(0x1c);
        let new_buffer = e.call(OPERATOR_NEW, &args![total]).u32();
        let flags = e.mem.u32(buffer.wrapping_add(8)) | 0x40000;
        e.mem.set_u32(buffer.wrapping_add(8), flags);
        e.mem
            .set_u32(buffer.wrapping_add(4), total.wrapping_sub(0x18));
        if is_big_endian(e) {
            e.call(FORM_ENDIAN, &args![buffer]);
        }
        e.call(MEMCPY, &args![new_buffer, buffer, 0x18u32]);
        let cursor = new_buffer.wrapping_add(0x18);
        with_item(e, body_size, |e, original_size| {
            if is_big_endian(e) {
                e.call(SWAP_WORD, &args![original_size, 0u32]);
            }
            e.call(MEMCPY, &args![cursor, original_size, 4u32]);
        });
        e.call(MEMCPY, &args![cursor.wrapping_add(4), packed, packed_size]);
        let old_buffer = e.mem.u32(SAVE_BUFFER);
        e.call(OPERATOR_DELETE, &args![old_buffer]);
        e.mem.set_u32(SAVE_BUFFER, new_buffer);
        e.mem.set_u32(SAVE_BUFFER_SIZE, total);
        e.call(DEFLATE_END, &args![stream]);
        e.call(OPERATOR_DELETE, &args![packed]);
    });
}

/// The eight components the component data casts a form to (0 where the form has none), in
/// [`COMPONENT_TYPES`]' order.
fn cast_components(e: &mut Engine, form: u32) -> [u32; 8] {
    let mut parts = [0u32; 8];
    for (part, target) in parts.iter_mut().zip(COMPONENT_TYPES) {
        *part = e
            .call(
                RT_DYNAMIC_CAST,
                &args![form, 0u32, TYPE_TES_FORM, target, 0u32],
            )
            .u32();
    }
    parts
}

/// The components' bytes in the `DATA` chunk: 1 for the uses and clip rounds, 4 each for value,
/// health, weight and quality, 2 for the attack damage, the attributes' own size (`009d2510`).
fn components_size(e: &mut Engine, parts: &[u32; 8]) -> u32 {
    let [uses, value, health, weight, quality, attack_damage, attributes, clip_rounds] = *parts;
    let mut size = u32::from(uses != 0);
    for part in [value, health, weight, quality] {
        if part != 0 {
            size = size.wrapping_add(4);
        }
    }
    if attack_damage != 0 {
        size = size.wrapping_add(2);
    }
    if attributes != 0 {
        size = size.wrapping_add(e.call(ATTRIBUTES_SIZE, &args![attributes]).u32());
    }
    if clip_rounds != 0 {
        size = size.wrapping_add(1);
    }
    size
}

/// `value` stored as a float at `out` through a stack local and `memcpy`, swapped on a big-endian
/// target; the address after it.
fn write_float(e: &mut Engine, out: u32, value: f32) -> u32 {
    e.with_stack(4, |e, local| {
        e.mem.set_f32(local.addr(), value);
        if is_big_endian(e) {
            e.call(SWAP_WORD, &args![local.addr(), 0u32]);
        }
        e.call(MEMCPY, &args![out, local.addr(), 4u32]);
    });
    out.wrapping_add(4)
}

// Typed from 00485e00
/// `TESForm::SaveData` (Xbox PDB): `__SaveData` with no data.
pub fn save_data(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0048_5e20, save_data_chunk(e, this, 0, 0));
}

// Typed from 00485e20
/// `TESForm::__SaveData` (Xbox PDB): the `DATA` chunk of `size` bytes of `data` (only a log line
/// for a size over 0xffff), grown by what the components add, which follow in this order: the uses
/// byte, the value and health words, the weight and the quality as floats, the attack damage
/// `u16`, the attributes' own data and the clip rounds byte, swapped where the target is
/// big-endian.
pub fn save_data_chunk(e: &mut Engine, this: u32, data: u32, size: u32) {
    if size > 0xffff {
        e.call(LOG_MESSAGE, &args![SAVE_DATA_TOO_LARGE_MESSAGE]);
        return;
    }
    let size_before = e.call(SAVE_BUFFER_SIZE_GETTER, &args![this]).u32();
    typed_call!(
        FAMILY,
        0x0048_5990,
        add_chunk_data(e, DATA_CHUNK_TAG, data, size)
    );
    let chunk = e
        .call(SAVE_BUFFER_GETTER, &args![this])
        .u32()
        .wrapping_add(size_before);
    let parts = cast_components(e, this);
    let [uses, value, health, weight, quality, attack_damage, attributes, clip_rounds] = parts;
    let extra = low_half(components_size(e, &parts));
    if extra == 0 {
        return;
    }
    let size_before = e.call(SAVE_BUFFER_SIZE_GETTER, &args![this]).u32();
    if !typed_call!(FAMILY, 0x0048_5a70, grow_chunk(e, this, chunk, extra)) {
        return;
    }
    let mut out = e
        .call(SAVE_BUFFER_GETTER, &args![this])
        .u32()
        .wrapping_add(size_before);
    if uses != 0 {
        let byte = e.call(COMPONENT_BYTE_GET, &args![uses]).u8();
        e.mem.set_u8(out, byte);
        out = out.wrapping_add(1);
    }
    for part in [value, health] {
        if part != 0 {
            let word = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, part));
            e.mem.set_u32(out, word);
            if is_big_endian(e) {
                e.call(SWAP_WORD, &args![out, 0u32]);
            }
            out = out.wrapping_add(4);
        }
    }
    if weight != 0 {
        let weight = e.call(WEIGHT_GET, &args![weight]).f32();
        out = write_float(e, out, weight);
    }
    if quality != 0 {
        let quality = e.call(QUALITY_GET, &args![quality]).u8();
        out = write_float(e, out, f32::from(quality));
    }
    if attack_damage != 0 {
        let damage = e
            .vcall(attack_damage, SLOT_ATTACK_DAMAGE_GET, &args![])
            .u16();
        e.mem.set_u16(out, damage);
        if is_big_endian(e) {
            e.call(SWAP_HALF, &args![out, 0u32]);
        }
        out = out.wrapping_add(2);
    }
    if attributes != 0 {
        let used = e.call(ATTRIBUTES_SAVE, &args![attributes, out]).u32();
        out = out.wrapping_add(used);
    }
    if clip_rounds != 0 {
        let byte = e.call(COMPONENT_BYTE_GET, &args![clip_rounds]).u8();
        e.mem.set_u8(out, byte);
    }
}

/// A word swapped through a stack local (`Swap32`).
fn byte_swapped_word(e: &mut Engine, value: u32) -> u32 {
    with_item(e, value, |e, local| {
        e.call(SWAP_WORD, &args![local, 0u32]);
        e.mem.u32(local)
    })
}

/// The float at `buffer + offset` (`memcpy` into a local, copied into a second that a big-endian
/// file swaps) and the offset after it.
fn read_float(e: &mut Engine, file: u32, buffer: u32, offset: u16) -> (f32, u16) {
    let value = e.with_stack(8, |e, locals| {
        let copy = locals.addr();
        let work = copy.wrapping_add(4);
        e.call(
            MEMCPY,
            &args![copy, buffer.wrapping_add(u32::from(offset)), 4u32],
        );
        let value = e.mem.f32(copy);
        e.mem.set_f32(work, value);
        if e.call(FILE_IS_BIG_ENDIAN, &args![file]).bool() {
            e.call(SWAP_WORD, &args![work, 0u32]);
        }
        e.mem.f32(work)
    });
    (value, offset.wrapping_add(4))
}

/// The quality byte from its float: chopped to an integer as the exe's `FISTP` does (the integer
/// indefinite out of range), its low byte.
fn quality_byte(value: f32) -> u32 {
    let chopped = engine_core::x87::fistp_chop(f64::from(value));
    chopped as u32 & 0xff
}

// Typed from 004861f0
/// `TESForm::LoadData` (Xbox PDB), `(file, destination, start)`: the file's current chunk read
/// into a local buffer, its first `min(chunk size, start)` bytes copied to `destination`, then
/// the components handed their values from offset `start` on, in `__SaveData`'s order (each only
/// while the offset is inside the chunk, the attributes always); the quality is chopped to an
/// integer and set as a byte. Words are swapped when the file is big-endian.
pub fn load_data(e: &mut Engine, this: u32, file: u32, destination: u32, start: u16) {
    let size = e.call(FILE_CHUNK_SIZE, &args![file]).u16();
    e.with_stack(u32::from(size).max(1), |e, buffer| {
        let buffer = buffer.addr();
        let at = |offset: u16| buffer.wrapping_add(u32::from(offset));
        e.call(FILE_READ_CHUNK, &args![file, buffer, u32::from(size)]);
        let copied = typed_call!(FAMILY, 0x0048_65f0, smaller_half(e, size, start));
        e.call(MEMCPY, &args![destination, buffer, u32::from(copied)]);
        let [uses, value, health, weight, quality, attack_damage, attributes, clip_rounds] =
            cast_components(e, this);
        let mut offset = start;
        let swapped = |e: &mut Engine| e.call(FILE_IS_BIG_ENDIAN, &args![file]).bool();
        if uses != 0 && offset < size {
            let byte = u32::from(e.mem.u8(at(offset)));
            e.call(COMPONENT_BYTE_SET, &args![uses, byte]);
            offset = offset.wrapping_add(1);
        }
        for (part, setter) in [(value, VALUE_SET), (health, HEALTH_SET)] {
            if part != 0 && offset < size {
                let mut word = e.mem.u32(at(offset));
                offset = offset.wrapping_add(4);
                if swapped(e) {
                    word = byte_swapped_word(e, word);
                }
                e.call(setter, &args![part, word]);
            }
        }
        if weight != 0 && offset < size {
            let (value, next) = read_float(e, file, buffer, offset);
            offset = next;
            e.call(WEIGHT_SET, &args![weight, value]);
        }
        if quality != 0 && offset < size {
            let (value, next) = read_float(e, file, buffer, offset);
            offset = next;
            e.call(COMPONENT_BYTE_SET, &args![quality, quality_byte(value)]);
        }
        if attack_damage != 0 && offset < size {
            let mut half = e.mem.u16(at(offset));
            offset = offset.wrapping_add(2);
            if swapped(e) {
                half = e.with_stack(2, |e, local| {
                    e.mem.set_u16(local.addr(), half);
                    e.call(SWAP_HALF, &args![local.addr(), 0u32]);
                    e.mem.u16(local.addr())
                });
            }
            e.call(ATTACK_DAMAGE_SET, &args![attack_damage, u32::from(half)]);
        }
        if attributes != 0 {
            let used = e
                .call(ATTRIBUTES_LOAD, &args![attributes, at(offset)])
                .u32();
            offset = low_half(u32::from(offset).wrapping_add(used));
        }
        if clip_rounds != 0 && offset < size {
            let byte = u32::from(e.mem.u8(at(offset)));
            e.call(COMPONENT_BYTE_SET, &args![clip_rounds, byte]);
        }
    });
}

// Typed from 004865f0
/// The smaller of two `u16` values (cdecl).
#[inline]
pub fn smaller_half(_e: &mut Engine, first: u16, second: u16) -> u16 {
    first.min(second)
}

// Typed from 00486620
/// The bytes the form's components add to its `DATA` chunk (fastcall).
pub fn components_data_size(e: &mut Engine, this: u32) -> u32 {
    let parts = cast_components(e, this);
    components_size(e, &parts)
}

// Typed from 004867a0
/// A form copier `(form, unused, map)`: a form of the same type (`CreateFormOfType`); when there is
/// one, the original's name (its getter `00401280`) kept in a string local while the original's
/// slot 0x134 takes the constant default name and the new form's slot 0x108 copies the original,
/// then handed back to the original when it holds anything; with a `map`, the new form set in it
/// under the original. The new form or 0.
pub fn copy_form(e: &mut Engine, this: u32, _unused: u32, map: u32) -> u32 {
    let form_type = e.call(FORM_TYPE, &args![this]).u32();
    let created = e.call(CREATE_FORM_OF_TYPE, &args![form_type]).u32();
    if created != 0 {
        let name = e.call(FORM_NAME_GETTER, &args![this]).u32();
        e.with_stack(8, |e, saved| {
            let saved = saved.addr();
            e.call(STRING_CONSTRUCT, &args![saved, name]);
            e.vcall(this, SLOT_TAKE_NAME, &args![DEFAULT_NAME]);
            e.vcall(created, SLOT_COPY_FROM, &args![this]);
            if typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, saved)) != 0 {
                let text = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, saved));
                e.vcall(this, SLOT_TAKE_NAME, &args![text]);
            }
            e.call(STRING_DESTROY, &args![saved]);
        });
        if map != 0 {
            e.call(MAP_SET_AT, &args![map, this, created]);
        }
    }
    created
}

// Typed from 00486890
/// `TESForm::GetFormTypeFromFormString` (Xbox PDB): the index of the form-type table's entry whose
/// packed string is `packed` (0 for none), the last string found and its index kept.
pub fn form_type_from_string(e: &mut Engine, packed: u32) -> u32 {
    if packed == e.mem.u32(FORM_TYPE_CACHE_KEY) {
        return e.mem.u32(FORM_TYPE_CACHE_RESULT);
    }
    for index in 0..FORM_ENUM_COUNT {
        if e.mem.u32(FORM_ENUM_TABLE + index * FORM_ENUM_STRIDE + 8) == packed {
            e.mem.set_u32(FORM_TYPE_CACHE_RESULT, index);
            e.mem.set_u32(FORM_TYPE_CACHE_KEY, packed);
            return index;
        }
    }
    0
}

// Typed from 004868f0
/// `TESForm::GetFormDetailedString` (Xbox PDB): `"<type> Form '<name>' (<form id>)"` printed into
/// `buffer`, the type the form-type table's string and the name what `00401280` answers.
pub fn detailed_string(e: &mut Engine, this: u32, buffer: u32) {
    let id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, this));
    // JohnnyGuitar NVSE's seam at the name's call answers the name.
    let name = match e.site(0x0048_6903, &[this]) {
        Some(name) => name,
        None => e.call(FORM_NAME_GETTER, &args![this]).u32(),
    };
    let type_name = e.call(FORM_TYPE_NAME, &args![this]).u32();
    e.call(
        FORMAT_STRING,
        &args![buffer, DETAILED_STRING_FORMAT, type_name, name, id],
    );
}

/// `destroy(this)`, then the release when bit 0 of `flags` is set; the object.
fn scalar_deleting(e: &mut Engine, this: u32, flags: u32, destroy: u32) -> u32 {
    e.call(destroy, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 00486930
/// The form map's constructor `(buckets)`: the map base's, then the `NiTPointerMap` vtable.
pub fn form_map_construct(e: &mut Engine, this: u32, buckets: u32) -> u32 {
    typed_call!(FAMILY, 0x0048_6990, map_base_construct(e, this, buckets));
    e.mem.set_u32(this, POINTER_MAP_VTABLE);
    this
}

// Typed from 00486960
/// `NiTPointerMap<unsigned int, TESForm *>`'s scalar deleting destructor.
pub fn form_map_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0048_6a00, form_map_destructor(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 00486990
/// The map base's constructor `(buckets)`: its vtable, the bucket count (+4), no items (+0x0c) and
/// a zeroed array of `buckets` words (+8, the count read again after the allocation).
pub fn map_base_construct(e: &mut Engine, this: u32, buckets: u32) -> u32 {
    e.mem.set_u32(this, MAP_BASE_VTABLE);
    e.mem.set_u32(this.wrapping_add(4), buckets);
    e.mem.set_u32(this.wrapping_add(0xc), 0);
    let array = e.call(ALLOCATE_BUCKETS, &args![buckets << 2]).u32();
    e.mem.set_u32(this.wrapping_add(8), array);
    let bytes = e.mem.u32(this.wrapping_add(4)).wrapping_shl(2);
    e.call(MEMSET, &args![array, 0u32, bytes]);
    this
}

// Typed from 00486a00
/// The form map's destructor: the `NiTPointerMap` vtable, every entry removed (`00438af0`), the
/// map base's destruction (`00486a60`).
pub fn form_map_destructor(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, POINTER_MAP_VTABLE);
    e.call(MAP_CLEAR, &args![this]);
    e.call(MAP_BASE_DESTROY, &args![this]);
}

// Typed from 00486c00
/// `NiTMapBase<.., unsigned int, TESForm *>`'s scalar deleting destructor (`00486a60`).
pub fn map_base_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    scalar_deleting(e, this, flags, MAP_BASE_DESTROY)
}

// Typed from 00486c30
/// `BSTCaseInsensitiveStringMap<TESForm *>`'s scalar deleting destructor.
pub fn string_map_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0048_6c60, string_map_destructor(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 00486c60
/// The case-insensitive string map's destructor: its vtable, then the base's (`00486c80`).
pub fn string_map_destructor(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, STRING_MAP_VTABLE);
    e.call(STRING_MAP_BASE_DESTROY, &args![this]);
}

// Typed from 00486e50
/// `NiTMapBase<.., const char *, TESForm *>`'s scalar deleting destructor (`00486dc0`).
pub fn char_key_map_base_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    scalar_deleting(e, this, flags, CHAR_KEY_MAP_BASE_DESTROY)
}

// Typed from 00486e80
/// `NiTMap<const char *, TESForm *>`'s scalar deleting destructor (`00486d60`).
pub fn char_key_map_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    scalar_deleting(e, this, flags, NI_T_MAP_DESTROY)
}

// Typed from 00486eb0
/// `NiTStringTemplateMap<.., TESForm *>`'s scalar deleting destructor (`00486ca0`).
pub fn string_template_map_scalar_deleting_destructor(
    e: &mut Engine,
    this: u32,
    flags: u32,
) -> u32 {
    scalar_deleting(e, this, flags, STRING_TEMPLATE_MAP_DESTROY)
}
