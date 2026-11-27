//! The extra data objects unit's last 86 functions (`extradataobjects.rs`, `00432cb0` to
//! `00435bb0`): the constructors, destructors, `Compare`s and `Copy`s of the classes that hold a
//! reference, a list or a name (`ExtraRagDollData`, `ExtraEncounterZone`, the used and reserved
//! markers, `ExtraRunOncePacks`, `ExtraDistantData`, the enable state parent and children,
//! `ExtraRandomTeleportMarker`, the linked reference and its children, the ash pile reference,
//! `ExtraActivateRef` and its children, `ExtraDecalRefs`, the reflected, reflector, water light and
//! lit water references with the three `InitItem`s, the merchant container, the leveled creature
//! modifier, `ExtraPoison`, `ExtraLastFinishedSequence`, `ExtraXTarget`, and the saved animation's
//! and havok data's destructors).
//!
//! A constructor is `BSExtraData`'s (`0040ec80`) in place with the class's type, then its vtable
//! and its payload; a class with a list of its own makes it empty in place (`0096a2d0`). A scalar
//! deleting destructor is the class's destructor as a typed call, then `operator delete` for bit 0
//! of the flags. A destructor puts the class's vtable back, drops what the class owns and ends in
//! `BSExtraData`'s destructor (`0040ecb0`, the cell references family's, a typed call). A `Compare`
//! casts the other extra data to its class (`__RTDynamicCast`, called by address); the rag doll
//! data and the encounter zone ask `BSExtraData::Compare` (`0040f700`, a typed call) too, the
//! others do not. True when the two differ.
//!
//! **Native.** The exe's order of work. Done in place: `BSExtraData`'s constructor, a list node's
//! item (`006815c0`) and next (`00726070`), a list's emptiness (`008256d0`) and its test for an
//! item (`005f65d0`), the empty list's constructor (`0096a2d0`), `NiPoint3`'s constructor
//! (`00416870`, the x87's load and store: a signalling NaN comes out quiet), a reference's extra
//! list (`005d43c0`, +0x44), a string's characters (`00559450`, with JIP LN NVSE's null check as
//! the leaf has it) and the two record constructors the copies call: `REF_ACTIVATE_DATA`'s
//! (`00414010`, two null words) and `REF_DECAL_DATA`'s (`0055a400`, which stores nothing). Each
//! item and next is read once. Typed calls: this family's own functions (the searches, clears, adds
//! and removals the methods share, the destructors the scalar deleting ones call, the reflected
//! flag and water light setters `0041f4c0`, `0041f650`, `0041f840`), the cell references family's
//! `BSExtraData` destructor and `Compare`, the C runtime's `memcmp` and `memcpy` wrapper
//! (`00401460`), and the forms family's `strlen` wrapper (`0044a670`), a form's file (`00484e60`),
//! its compile index (`00485d50`) and the lookup by id (`004839c0`). Called by address: `operator
//! new` and `operator delete`, the simple list's clear (`00470470`), destructor body (`0046ffb0`),
//! delete (`004702f0`), head removal (`0063f7b0`), add at the head (`005ae3d0`), count (`005ae380`)
//! and removal after a node (`00905330`), the string's constructor, `Set`, clear, copy and length
//! (`004037b0`, `004037f0`, `004037d0`, `00438470`, `004048e0`), the `strcmp` and `strcpy_s`
//! wrappers (`00408b20`, `00406d30`), the rag doll data's destructor and compare (`004d9380`,
//! `004d9780`), a reference's persistence (`005653d0`), the master file log (`005b5e40`), the float
//! setting's value (`00403e20`) and the saved data's buffer release (`00435c10`). A call by address
//! the exe makes again (the string length) is made again; a word read again after one is read
//! again.
//!
//! **Extenders.** None of these addresses is wrapped, hooked or patched by xNVSE, JIP LN NVSE, UIO
//! or JohnnyGuitar; they call none of them by address either.

use super::classes::extra_with_vtable;
use super::links::{holds, is_empty};
use super::objects::{
    compare_start, compare_word, drop_plain, extra_drop, memcmp, new_actor_list, with_word,
    DYNAMIC_CAST, TYPE_BS_EXTRA_DATA,
};
use super::{
    free_when, next_node, read_word, rooms, simple_list_init, word_at, EXTRA_PAYLOAD, FAMILY,
    OPERATOR_DELETE, OPERATOR_NEW,
};
use crate::{crt, forms};
use engine_core::ptr::Ptr;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

// ---- The classes' types, vtables and RTTI descriptors ---------------------------------------

const KIND_RAG_DOLL_DATA: u8 = 0x14;
const KIND_ENCOUNTER_ZONE: u8 = 0x74;
const KIND_USED_MARKERS: u8 = 0x12;
const KIND_RESERVED_MARKERS: u8 = 0x82;
const KIND_RUN_ONCE_PACKS: u8 = 0x1b;
const KIND_DISTANT_DATA: u8 = 0x13;
const KIND_ENABLE_STATE_PARENT: u8 = 0x37;
const KIND_ENABLE_STATE_CHILDREN: u8 = 0x38;
const KIND_RANDOM_TELEPORT_MARKER: u8 = 0x3b;
const KIND_LINKED_REF_CHILDREN: u8 = 0x52;
const KIND_LINKED_REF: u8 = 0x51;
const KIND_ASH_PILE_REF: u8 = 0x89;
const KIND_ACTIVATE_REF_CHILDREN: u8 = 0x54;
const KIND_ACTIVATE_REF: u8 = 0x53;
const KIND_DECAL_REFS: u8 = 0x57;
const KIND_MERCHANT_CONTAINER: u8 = 0x3c;
const KIND_LEV_CREA_MODIFIER: u8 = 0x1e;
const KIND_POISON: u8 = 0x3f;
const KIND_LAST_FINISHED_SEQUENCE: u8 = 0x41;
const KIND_X_TARGET: u8 = 0x44;

const VTABLE_RAG_DOLL_DATA: u32 = 0x0101_5c48;
const VTABLE_ENCOUNTER_ZONE: u32 = 0x0101_5c54;
const VTABLE_USED_MARKERS: u32 = 0x0101_5c60;
const VTABLE_RESERVED_MARKERS: u32 = 0x0101_5c6c;
const VTABLE_RUN_ONCE_PACKS: u32 = 0x0101_5c78;
const VTABLE_DISTANT_DATA: u32 = 0x0101_5c84;
const VTABLE_ENABLE_STATE_PARENT: u32 = 0x0101_5c90;
const VTABLE_ENABLE_STATE_CHILDREN: u32 = 0x0101_5c9c;
const VTABLE_RANDOM_TELEPORT_MARKER: u32 = 0x0101_5ca8;
const VTABLE_LINKED_REF_CHILDREN: u32 = 0x0101_5cb4;
const VTABLE_LINKED_REF: u32 = 0x0101_5cc0;
const VTABLE_ASH_PILE_REF: u32 = 0x0101_5ccc;
const VTABLE_ACTIVATE_REF_CHILDREN: u32 = 0x0101_5cd8;
const VTABLE_ACTIVATE_REF: u32 = 0x0101_5ce4;
const VTABLE_DECAL_REFS: u32 = 0x0101_5cf0;
const VTABLE_REFLECTED_REFS: u32 = 0x0101_4428;
const VTABLE_REFLECTOR_REFS: u32 = 0x0101_4434;
const VTABLE_WATER_LIGHT_REFS: u32 = 0x0101_4440;
const VTABLE_LIT_WATER_REFS: u32 = 0x0101_444c;
const VTABLE_MERCHANT_CONTAINER: u32 = 0x0101_5e04;
const VTABLE_LEV_CREA_MODIFIER: u32 = 0x0101_5e10;
const VTABLE_POISON: u32 = 0x0101_595c;
const VTABLE_LAST_FINISHED_SEQUENCE: u32 = 0x0101_5e1c;
const VTABLE_X_TARGET: u32 = 0x0101_5e28;
const VTABLE_SAVED_ANIMATION: u32 = 0x0101_5ea0;
const VTABLE_SAVED_HAVOK_DATA: u32 = 0x0101_5eac;

const TYPE_RAG_DOLL_DATA: u32 = 0x0118_4cd4;
const TYPE_ENCOUNTER_ZONE: u32 = 0x0118_4cf4;
const TYPE_ENABLE_STATE_PARENT: u32 = 0x0118_4d7c;
const TYPE_RANDOM_TELEPORT_MARKER: u32 = 0x0118_4dcc;
const TYPE_LINKED_REF: u32 = 0x0118_4e1c;
const TYPE_ACTIVATE_REF: u32 = 0x0118_4e84;
const TYPE_DECAL_REFS: u32 = 0x0118_4ea4;
const TYPE_REFLECTED_REFS: u32 = 0x0118_3fec;
const TYPE_REFLECTOR_REFS: u32 = 0x0118_4010;
const TYPE_WATER_LIGHT_REFS: u32 = 0x0118_4034;
const TYPE_LIT_WATER_REFS: u32 = 0x0118_4058;
const TYPE_MERCHANT_CONTAINER: u32 = 0x0118_4ec4;
const TYPE_LEV_CREA_MODIFIER: u32 = 0x0118_4eec;
const TYPE_POISON: u32 = 0x0118_461c;
const TYPE_LAST_FINISHED_SEQUENCE: u32 = 0x0118_4408;
/// What the `InitItem`s cast a looked-up form from and to (`TESForm`, `TESObjectREFR`).
const TYPE_FORM: u32 = 0x0118_3028;
const TYPE_REFERENCE: u32 = 0x0118_41cc;

// ---- What the functions call ----------------------------------------------------------------

/// The simple list's clear (`thiscall(list)`: the nodes after the head freed, the head emptied),
/// destructor body, scalar deleting destructor, head removal, add at the head (`thiscall(list,
/// &item)`), count of the nodes with an item, and removal of the item at the address given from the
/// nodes after `this` (`thiscall(node, &item)`).
const LIST_CLEAR: u32 = 0x0047_0470;
const LIST_DROP: u32 = 0x0046_ffb0;
const LIST_DELETE: u32 = 0x0047_02f0;
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
const LIST_COUNT: u32 = 0x005a_e380;
const LIST_REMOVE_AFTER: u32 = 0x0090_5330;
/// `BSStringT<char>`'s constructor, `Set(text, length)`, clear, `Set(other)` and length.
const STRING_NEW: u32 = 0x0040_37b0;
const STRING_SET: u32 = 0x0040_37f0;
const STRING_CLEAR: u32 = 0x0040_37d0;
const STRING_COPY: u32 = 0x0043_8470;
const STRING_LENGTH: u32 = 0x0040_48e0;
/// The `strcmp` and `strcpy_s` wrappers (`cdecl`).
const STRCMP: u32 = 0x0040_8b20;
const STRCPY_S: u32 = 0x0040_6d30;
/// The empty C string `ExtraActivateRef`'s constructor sets its text to.
const EMPTY_STRING: u32 = 0x0101_1584;
/// The rag doll data's destructor body and compare (`thiscall(data, other)`).
const RAG_DOLL_DATA_DROP: u32 = 0x004d_9380;
const RAG_DOLL_DATA_COMPARE: u32 = 0x004d_9780;
/// A reference's persistence (`thiscall(reference)`) and the master file log (`cdecl(format, id)`)
/// with its four messages.
const REFERENCE_PERSISTS: u32 = 0x0056_53d0;
const LOG_MESSAGE: u32 = 0x005b_5e40;
const MESSAGE_DECAL_NOT_FOUND: u32 = 0x0101_5cf8;
const MESSAGE_REFLECTOR_NOT_FOUND: u32 = 0x0101_5d8c;
const MESSAGE_NOT_PERSISTENT: u32 = 0x0101_5d30;
const MESSAGE_LIT_WATER_NOT_FOUND: u32 = 0x0101_5dc8;
/// A float setting's value (`thiscall(setting)`: the value's address) and the leveled creature
/// modifiers' table of settings (one pointer per modifier below "none").
const SETTING_FLOAT: u32 = 0x0040_3e20;
const LEV_CREA_SETTINGS: u32 = 0x0118_4ab0;
/// The leveled creature modifier "none" (`LCM_NONE`).
const LEV_CREA_NONE: u32 = 4;
/// The `float` the distant data's land normal has as its z.
const DISTANT_NORMAL_Z: u32 = 0x0101_45a8;
/// The saved animation's and havok data's buffer release (`thiscall(extra)`).
const SAVED_DELETE_BUFFER: u32 = 0x0043_5c10;
/// A reference's extra list (`005d43c0` in place).
const REFERENCE_EXTRA_LIST: u32 = 0x44;

// ---- What the file shares --------------------------------------------------------------------

/// The list at +0xc of the classes that have one.
const OWN_LIST: u32 = 0x0c;

/// A list node's item (`006815c0` in place: the node's first word).
#[inline]
fn item_of(e: &Engine, node: u32) -> u32 {
    e.mem.u32(node)
}

/// Whether a walk goes on at `node`: one there, with an item or a next (`008256d0` in place).
#[inline]
fn walks(e: &Engine, node: u32) -> bool {
    node != 0 && !is_empty(e, node)
}

/// `other` as the class with the RTTI descriptor `target` (`__RTDynamicCast` by address), null when
/// it is not one.
#[inline]
fn cast_to(e: &mut Engine, other: u32, target: u32) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![other, 0u32, TYPE_BS_EXTRA_DATA, target, 0u32],
    )
    .u32()
}

/// A `Compare` that only casts (no `BSExtraData::Compare`) and compares the word at +0xc.
#[inline]
fn compare_cast_word(e: &mut Engine, this: u32, other: u32, target: u32) -> bool {
    let cast = cast_to(e, other, target);
    cast == 0 || word_at(e, cast, EXTRA_PAYLOAD) != word_at(e, this, EXTRA_PAYLOAD)
}

/// A constructor of a class with an empty list of its own at +0xc.
#[inline]
fn with_list(e: &mut Engine, this: u32, kind: u8, vtable: u32) -> u32 {
    extra_with_vtable(e, this, kind, vtable);
    simple_list_init(e, this.wrapping_add(OWN_LIST));
    this
}

/// The destructor of a class whose list holds what it does not own: the vtable back, the list
/// cleared and its destructor body, then `BSExtraData`'s.
#[inline]
fn drop_list(e: &mut Engine, this: u32, vtable: u32) {
    e.mem.set_u32(this, vtable);
    let list = this.wrapping_add(OWN_LIST);
    e.call(LIST_CLEAR, &args![list]);
    e.call(LIST_DROP, &args![list]);
    extra_drop(e, this);
}

/// The destructor of a class that owns its list's entries (the reflected and reflector references):
/// the vtable back, every entry deleted, the list cleared and its destructor body, then
/// `BSExtraData`'s.
#[inline]
fn drop_owned_entries(e: &mut Engine, this: u32, vtable: u32) {
    e.mem.set_u32(this, vtable);
    let list = this.wrapping_add(OWN_LIST);
    let mut node = list;
    while walks(e, node) {
        let item = item_of(e, node);
        e.call(OPERATOR_DELETE, &args![item]);
        node = next_node(e, node);
    }
    e.call(LIST_CLEAR, &args![list]);
    e.call(LIST_DROP, &args![list]);
    extra_drop(e, this);
}

/// `item` added at the head of `list` (`005ae3d0` by address, handed a one-word local holding it).
#[inline]
fn add_head(e: &mut Engine, list: u32, item: u32) {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), item);
        e.call(LIST_ADD_HEAD, &args![list, local]);
    });
}

/// `n` bytes copied (the C runtime family's `memcpy` wrapper, `00401460`).
#[inline]
fn copy_bytes(e: &mut Engine, to: u32, from: u32, n: u32) {
    typed_call!(
        crt::FAMILY,
        0x0040_1460,
        crt::mem::memcpy_wrapper(e, Ptr::new(to), Ptr::new(from), n)
    );
}

/// An 8-byte record from `operator new` made an empty node in place (`00414010` and `0096a2d0`: two
/// null words), or null when the allocation failed.
#[inline]
fn new_empty_record(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![8u32]).u32();
    if block != 0 {
        simple_list_init(e, block);
    }
    block
}

/// Three words copied (a point moved as its bits).
#[inline]
fn copy_point(e: &mut Engine, from: u32, to: u32) {
    for offset in [0, 4, 8] {
        let word = e.mem.u32(from.wrapping_add(offset));
        e.mem.set_u32(to.wrapping_add(offset), word);
    }
}

// ---- ExtraRagDollData and ExtraEncounterZone ---------------------------------------------------

// Typed from 00432cb0
/// `ExtraRagDollData::ExtraRagDollData` (type 0x14): no rag doll data. Answers `this`.
pub fn rag_doll_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_RAG_DOLL_DATA, VTABLE_RAG_DOLL_DATA, 0)
}

// Typed from 00432ce0
/// `ExtraRagDollData`'s scalar deleting destructor. Answers `this`.
pub fn rag_doll_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_2d10, rag_doll_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00432d10
/// `ExtraRagDollData`'s destructor: the owned rag doll data deleted (`00432da0` with 1).
pub fn rag_doll_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_RAG_DOLL_DATA);
    let data = word_at(e, this, EXTRA_PAYLOAD);
    if data != 0 {
        typed_call!(FAMILY, 0x0043_2da0, rag_doll_data_delete(e, data, 1));
    }
    extra_drop(e, this);
}

// Typed from 00432da0
/// The rag doll data's scalar deleting destructor: its destructor body (`004d9380`). Answers
/// `this`.
pub fn rag_doll_data_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(RAG_DOLL_DATA_DROP, &args![this]);
    free_when(e, this, flags)
}

// Typed from 00432dd0
/// `ExtraRagDollData::Compare`: with no data here, whether the other has some; else the data's own
/// compare (`004d9780`).
pub fn rag_doll_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let Some(cast) = compare_start(e, this, other, TYPE_RAG_DOLL_DATA) else {
        return true;
    };
    let mine = word_at(e, this, EXTRA_PAYLOAD);
    let theirs = word_at(e, cast, EXTRA_PAYLOAD);
    if mine == 0 {
        return theirs != 0;
    }
    e.call(RAG_DOLL_DATA_COMPARE, &args![mine, theirs]).bool()
}

// Typed from 00432e60
/// `ExtraEncounterZone::ExtraEncounterZone` (type 0x74): no zone. Answers `this`.
pub fn encounter_zone_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_ENCOUNTER_ZONE, VTABLE_ENCOUNTER_ZONE, 0)
}

// Typed from 00432e90
/// `ExtraEncounterZone::Compare`: the zone.
pub fn encounter_zone_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_word(e, this, other, TYPE_ENCOUNTER_ZONE)
}

// ---- The markers, the run-once packages, the distant data ---------------------------------------

// Typed from 00432f00
/// `ExtraUsedMarkers`' constructor (type 0x12): no marker used. Answers `this`.
pub fn used_markers_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_USED_MARKERS, VTABLE_USED_MARKERS, 0)
}

// Typed from 00432f30
/// The destructor body that is only `BSExtraData`'s (the vtable is not put back).
pub fn base_only_drop(e: &mut Engine, this: u32) {
    extra_drop(e, this);
}

// Typed from 00432f50
/// `ExtraReservedMarkers`' constructor (type 0x82): no marker reserved. Answers `this`.
pub fn reserved_markers_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_RESERVED_MARKERS, VTABLE_RESERVED_MARKERS, 0)
}

// Typed from 00432f80
/// `ExtraUsedMarkers::GetMarkerUsed`: bit `index` of the used markers; false from 30 on.
pub fn marker_used(e: &mut Engine, this: u32, index: u32) -> bool {
    index < 0x1e && word_at(e, this, EXTRA_PAYLOAD) & (1u32 << (index & 31)) != 0
}

// Typed from 00432fc0
/// `ExtraUsedMarkers::SetMarkerUsed`: bit `index` set (`used` not 0) or cleared; nothing from 30
/// on.
pub fn set_marker_used(e: &mut Engine, this: u32, index: u32, used: u8) {
    if index >= 0x1e {
        return;
    }
    let bit = 1u32 << (index & 31);
    let markers = word_at(e, this, EXTRA_PAYLOAD);
    let markers = if used != 0 {
        markers | bit
    } else {
        markers & !bit
    };
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), markers);
}

// Typed from 00433010
/// `ExtraRunOncePacks`' constructor (type 0x1b): an empty list of its own (null when the allocation
/// failed). Answers `this`.
pub fn run_once_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_RUN_ONCE_PACKS, VTABLE_RUN_ONCE_PACKS);
    let list = new_actor_list(e);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), list);
    this
}

// Typed from 004330b0
/// `ExtraRunOncePacks`' scalar deleting destructor. Answers `this`.
pub fn run_once_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_30e0, run_once_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004330e0
/// `ExtraRunOncePacks`' destructor: the entries deleted up to the first node without one, the list
/// cleared (even a null one, as the exe does) and deleted (`004702f0` with 1).
pub fn run_once_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_RUN_ONCE_PACKS);
    let mut node = word_at(e, this, EXTRA_PAYLOAD);
    while node != 0 {
        let item = item_of(e, node);
        if item == 0 {
            break;
        }
        e.call(OPERATOR_DELETE, &args![item]);
        node = next_node(e, node);
    }
    let list = word_at(e, this, EXTRA_PAYLOAD);
    e.call(LIST_CLEAR, &args![list]);
    let list = word_at(e, this, EXTRA_PAYLOAD);
    if list != 0 {
        e.call(LIST_DELETE, &args![list, 1u32]);
    }
    extra_drop(e, this);
}

// Typed from 004331c0
/// The run-once flag of the package `key`: its 8-byte entry found (up to the first node without
/// one) or made and added at the head, then the key and the flag byte stored.
pub fn run_once_set(e: &mut Engine, this: u32, key: u32, flag: u8) {
    let mut node = word_at(e, this, EXTRA_PAYLOAD);
    let mut found = 0;
    while node != 0 {
        let item = item_of(e, node);
        if item == 0 {
            break;
        }
        if e.mem.u32(item) == key {
            found = item;
            break;
        }
        node = next_node(e, node);
    }
    if found == 0 {
        let entry = e.call(OPERATOR_NEW, &args![8u32]).u32();
        let list = word_at(e, this, EXTRA_PAYLOAD);
        // The exe adds the pointer in a stack local and reads the local again for the entry.
        found = e.with_stack(4, |e, local| {
            e.mem.set_u32(local.addr(), entry);
            e.call(LIST_ADD_HEAD, &args![list, local]);
            e.mem.u32(local.addr())
        });
    }
    e.mem.set_u32(found, key);
    e.mem.set_u8(found.wrapping_add(4), flag);
}

// Typed from 00433260
/// `ExtraDistantData`'s constructor (type 0x13): the land normal (0, 0, z) with `z` the exe's
/// constant, through the x87 (`00416870` in place). Answers `this`.
pub fn distant_data_new(e: &mut Engine, this: u32) -> u32 {
    extra_with_vtable(e, this, KIND_DISTANT_DATA, VTABLE_DISTANT_DATA);
    let z = x87_quiet_bits(e.mem.u32(DISTANT_NORMAL_Z));
    let normal = this.wrapping_add(EXTRA_PAYLOAD);
    e.mem.set_u32(normal, 0);
    e.mem.set_u32(normal.wrapping_add(4), 0);
    e.mem.set_u32(normal.wrapping_add(8), z);
    this
}

// ---- The enable state, the random teleport marker, the linked and ash pile references ----------

// Typed from 00433300
/// `ExtraEnableStateParent`'s constructor (type 0x37): no parent, no flags. Answers `this`.
pub fn enable_parent_new(e: &mut Engine, this: u32) -> u32 {
    with_word(
        e,
        this,
        KIND_ENABLE_STATE_PARENT,
        VTABLE_ENABLE_STATE_PARENT,
        0,
    );
    e.mem.set_u8(this.wrapping_add(0x10), 0);
    this
}

// Typed from 00433340
/// `ExtraEnableStateParent::Compare`: the parent and the flags byte.
pub fn enable_parent_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = cast_to(e, other, TYPE_ENABLE_STATE_PARENT);
    cast == 0
        || word_at(e, cast, EXTRA_PAYLOAD) != word_at(e, this, EXTRA_PAYLOAD)
        || e.mem.u8(cast.wrapping_add(0x10)) != e.mem.u8(this.wrapping_add(0x10))
}

// Typed from 004333a0
/// `ExtraEnableStateChildren`' constructor (type 0x38): an empty child list. Answers `this`.
pub fn enable_children_new(e: &mut Engine, this: u32) -> u32 {
    with_list(
        e,
        this,
        KIND_ENABLE_STATE_CHILDREN,
        VTABLE_ENABLE_STATE_CHILDREN,
    )
}

// Typed from 00433410
/// `ExtraEnableStateChildren`' scalar deleting destructor. Answers `this`.
pub fn enable_children_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_3440, enable_children_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00433440
/// `ExtraEnableStateChildren`' destructor: the child list cleared.
pub fn enable_children_drop(e: &mut Engine, this: u32) {
    drop_list(e, this, VTABLE_ENABLE_STATE_CHILDREN);
}

// Typed from 004334b0
/// `ExtraRandomTeleportMarker`'s constructor (type 0x3b): no marker. Answers `this`.
pub fn random_teleport_new(e: &mut Engine, this: u32) -> u32 {
    with_word(
        e,
        this,
        KIND_RANDOM_TELEPORT_MARKER,
        VTABLE_RANDOM_TELEPORT_MARKER,
        0,
    )
}

// Typed from 004334e0
/// `ExtraRandomTeleportMarker::Compare`: the marker.
pub fn random_teleport_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_cast_word(e, this, other, TYPE_RANDOM_TELEPORT_MARKER)
}

// Typed from 00433530
/// `ExtraLinkedRefChildren`' constructor (type 0x52): an empty child list. Answers `this`.
pub fn linked_children_new(e: &mut Engine, this: u32) -> u32 {
    with_list(
        e,
        this,
        KIND_LINKED_REF_CHILDREN,
        VTABLE_LINKED_REF_CHILDREN,
    )
}

// Typed from 004335a0
/// `ExtraLinkedRefChildren`' scalar deleting destructor. Answers `this`.
pub fn linked_children_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_35d0, linked_children_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004335d0
/// `ExtraLinkedRefChildren`' destructor: the child list cleared.
pub fn linked_children_drop(e: &mut Engine, this: u32) {
    drop_list(e, this, VTABLE_LINKED_REF_CHILDREN);
}

// Typed from 00433640
/// `ExtraLinkedRef`'s constructor (type 0x51): no linked reference. Answers `this`.
pub fn linked_ref_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_LINKED_REF, VTABLE_LINKED_REF, 0)
}

// Typed from 00433670
/// `ExtraLinkedRef::Compare`: the linked reference.
pub fn linked_ref_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_cast_word(e, this, other, TYPE_LINKED_REF)
}

// Typed from 004336c0
/// `ExtraAshPileRef`'s constructor (type 0x89): no reference. Answers `this`.
pub fn ash_pile_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_ASH_PILE_REF, VTABLE_ASH_PILE_REF, 0)
}

// ---- ExtraActivateRef and its children --------------------------------------------------------

// Typed from 004336f0
/// The first `REF_ACTIVATE_DATA` of the list whose reference is `reference` (`cdecl`; the walk ends
/// at an empty node), or null.
pub fn activate_data_find(e: &mut Engine, reference: u32, list: u32) -> u32 {
    let mut node = list;
    while walks(e, node) {
        let item = item_of(e, node);
        if e.mem.u32(item) == reference {
            return item;
        }
        node = next_node(e, node);
    }
    0
}

// Typed from 00433750
/// Every entry of the list deleted and its head removed (`0063f7b0`) until the list is empty
/// (`cdecl`).
pub fn delete_entries(e: &mut Engine, list: u32) {
    while walks(e, list) {
        let item = item_of(e, list);
        e.call(OPERATOR_DELETE, &args![item]);
        e.call(LIST_REMOVE_HEAD, &args![list]);
    }
}

// Typed from 00433790
/// `ExtraActivateRefChildren`' constructor (type 0x54): an empty child list, a timer of 0. Answers
/// `this`.
pub fn activate_children_new(e: &mut Engine, this: u32) -> u32 {
    with_list(
        e,
        this,
        KIND_ACTIVATE_REF_CHILDREN,
        VTABLE_ACTIVATE_REF_CHILDREN,
    );
    e.mem.set_u32(this.wrapping_add(0x14), 0);
    this
}

// Typed from 00433800
/// `ExtraActivateRefChildren`' scalar deleting destructor. Answers `this`.
pub fn activate_children_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_3830, activate_children_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00433830
/// `ExtraActivateRefChildren`' destructor: the child list's entries deleted (`00433750`).
pub fn activate_children_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_ACTIVATE_REF_CHILDREN);
    let list = this.wrapping_add(OWN_LIST);
    typed_call!(FAMILY, 0x0043_3750, delete_entries(e, list));
    e.call(LIST_DROP, &args![list]);
    extra_drop(e, this);
}

/// `ExtraActivateRef`'s flags byte and its activate text (a `BSStringT`).
const ACTIVATE_FLAGS: u32 = 0x14;
const ACTIVATE_TEXT: u32 = 0x18;

// Typed from 004338b0
/// `ExtraActivateRef`'s constructor (type 0x53): an empty parent list, no flags, the activate text
/// made and set to the empty string. Answers `this`.
pub fn activate_ref_new(e: &mut Engine, this: u32) -> u32 {
    with_list(e, this, KIND_ACTIVATE_REF, VTABLE_ACTIVATE_REF);
    let text = this.wrapping_add(ACTIVATE_TEXT);
    e.call(STRING_NEW, &args![text]);
    e.mem.set_u8(this.wrapping_add(ACTIVATE_FLAGS), 0);
    e.call(STRING_SET, &args![text, EMPTY_STRING, 0u32]);
    this
}

// Typed from 00433940
/// `ExtraActivateRef`'s scalar deleting destructor. Answers `this`.
pub fn activate_ref_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_3970, activate_ref_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00433970
/// `ExtraActivateRef`'s destructor: the parent list's entries deleted (`00433750`), the text
/// cleared.
pub fn activate_ref_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_ACTIVATE_REF);
    let list = this.wrapping_add(OWN_LIST);
    typed_call!(FAMILY, 0x0043_3750, delete_entries(e, list));
    e.call(STRING_CLEAR, &args![this.wrapping_add(ACTIVATE_TEXT)]);
    e.call(LIST_DROP, &args![list]);
    extra_drop(e, this);
}

// Typed from 00433a00
/// The parent list's entry for `reference` (`004336f0`), or null.
pub fn activate_ref_find(e: &mut Engine, this: u32, reference: u32) -> u32 {
    let list = this.wrapping_add(OWN_LIST);
    typed_call!(FAMILY, 0x0043_36f0, activate_data_find(e, reference, list))
}

// Typed from 00433a20
/// `ExtraActivateRef::Compare`: as many entries (`005ae380`), each of this list with an entry for
/// its reference in the other of the same 8 bytes, the same flags, and the same text (two empty
/// ones alike, one empty one different, else `strcmp`).
pub fn activate_ref_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = cast_to(e, other, TYPE_ACTIVATE_REF);
    if cast == 0 {
        return true;
    }
    let mine = this.wrapping_add(OWN_LIST);
    let theirs = cast.wrapping_add(OWN_LIST);
    let my_count = e.call(LIST_COUNT, &args![mine]).u32();
    let their_count = e.call(LIST_COUNT, &args![theirs]).u32();
    if my_count != their_count {
        return true;
    }
    let mut node = mine;
    while walks(e, node) {
        let item = item_of(e, node);
        let reference = e.mem.u32(item);
        let counterpart = typed_call!(FAMILY, 0x0043_3a00, activate_ref_find(e, cast, reference));
        if counterpart == 0 || memcmp(e, counterpart, item, 8) != 0 {
            return true;
        }
        node = next_node(e, node);
    }
    if e.mem.u8(this.wrapping_add(ACTIVATE_FLAGS)) != e.mem.u8(cast.wrapping_add(ACTIVATE_FLAGS)) {
        return true;
    }
    let my_text = this.wrapping_add(ACTIVATE_TEXT);
    let their_text = cast.wrapping_add(ACTIVATE_TEXT);
    // The exe asks each length again for its second test.
    if text_empty(e, my_text) && text_empty(e, their_text) {
        return false;
    }
    if text_empty(e, my_text) || text_empty(e, their_text) {
        return true;
    }
    let their_chars = read_word(e, their_text);
    let my_chars = read_word(e, my_text);
    e.call(STRCMP, &args![my_chars, their_chars]).u32() != 0
}

/// Whether a string's length (`004048e0` by address) is 0.
#[inline]
fn text_empty(e: &mut Engine, text: u32) -> bool {
    e.call(STRING_LENGTH, &args![text]).u32() == 0
}

// Typed from 00433b70
/// `ExtraActivateRef::Copy`: from another `ExtraActivateRef` only, the parent list's entries
/// deleted (`00433750`), a copy of each of the other's 8-byte entries added at the head (the order
/// turned round), the flags and the text (`00438470`) copied.
pub fn activate_ref_copy(e: &mut Engine, this: u32, other: u32) {
    let cast = cast_to(e, other, TYPE_ACTIVATE_REF);
    if cast == 0 {
        return;
    }
    let list = this.wrapping_add(OWN_LIST);
    typed_call!(FAMILY, 0x0043_3750, delete_entries(e, list));
    let mut node = cast.wrapping_add(OWN_LIST);
    while walks(e, node) {
        let item = item_of(e, node);
        node = next_node(e, node);
        let copy = new_empty_record(e);
        copy_bytes(e, copy, item, 8);
        add_head(e, list, copy);
    }
    let flags = e.mem.u8(cast.wrapping_add(ACTIVATE_FLAGS));
    e.mem.set_u8(this.wrapping_add(ACTIVATE_FLAGS), flags);
    e.call(
        STRING_COPY,
        &args![
            this.wrapping_add(ACTIVATE_TEXT),
            cast.wrapping_add(ACTIVATE_TEXT)
        ],
    );
}

// ---- ExtraDecalRefs and the three InitItems ---------------------------------------------------

// Typed from 00433ca0
/// `ExtraDecalRefs`' constructor (type 0x57): an empty decal list. Answers `this`.
pub fn decal_new(e: &mut Engine, this: u32) -> u32 {
    with_list(e, this, KIND_DECAL_REFS, VTABLE_DECAL_REFS)
}

// Typed from 00433d10
/// `ExtraDecalRefs`' scalar deleting destructor. Answers `this`.
pub fn decal_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_3d40, decal_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00433d40
/// `ExtraDecalRefs`' destructor: the entries deleted and the list emptied (`004343c0`).
pub fn decal_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_DECAL_REFS);
    typed_call!(FAMILY, 0x0043_43c0, decal_clear(e, this));
    e.call(LIST_DROP, &args![this.wrapping_add(OWN_LIST)]);
    extra_drop(e, this);
}

/// The `InitItem`s' lookup: the form id in the local `id_slot` given the owner's file's compile
/// index (the forms family's file `00484e60` and compile index `00485d50`), looked up (`004839c0`)
/// and cast to a reference. The reference, or null.
fn resolve_reference(e: &mut Engine, owner: u32, id_slot: u32) -> u32 {
    let file = typed_call!(
        forms::FAMILY,
        0x0048_4e60,
        forms::tesform::get_file(e, owner, -1)
    );
    typed_call!(
        forms::FAMILY,
        0x0048_5d50,
        forms::tesform::add_compile_index(e, id_slot, file)
    );
    let id = e.mem.u32(id_slot);
    let form = typed_call!(
        forms::FAMILY,
        0x0048_39c0,
        forms::tesform::form_by_id(e, id)
    );
    e.call(
        DYNAMIC_CAST,
        &args![form, 0u32, TYPE_FORM, TYPE_REFERENCE, 0u32],
    )
    .u32()
}

/// An entry the `InitItem` could not resolve dropped: the head removed (`0063f7b0`) when there is
/// no previous node, else the entry in the local `item_slot` removed after `previous` (`00905330`)
/// and the walk going on after `previous`; then the entry deleted and the local cleared. The node
/// the walk goes on with.
fn drop_unresolved(e: &mut Engine, node: u32, previous: u32, item_slot: u32) -> u32 {
    let next = if previous == 0 {
        e.call(LIST_REMOVE_HEAD, &args![node]);
        node
    } else {
        e.call(LIST_REMOVE_AFTER, &args![previous, item_slot]);
        next_node(e, previous)
    };
    let item = e.mem.u32(item_slot);
    e.call(OPERATOR_DELETE, &args![item]);
    e.mem.set_u32(item_slot, 0);
    next
}

// Typed from 00433db0
/// `ExtraDecalRefs::InitItem(owner)`: each decal entry's form id resolved to its reference (stored
/// in its first word); an entry whose form is no reference logged ("Unable to find decal
/// reference"), removed and deleted.
pub fn decal_init_item(e: &mut Engine, this: u32, owner: u32) {
    e.with_stack(8, |e, frame| {
        let item_slot = frame.addr();
        let id_slot = item_slot.wrapping_add(4);
        let mut node = this.wrapping_add(OWN_LIST);
        let mut previous = 0;
        while walks(e, node) {
            let item = item_of(e, node);
            e.mem.set_u32(item_slot, item);
            let id = e.mem.u32(item);
            e.mem.set_u32(id_slot, id);
            let reference = resolve_reference(e, owner, id_slot);
            if reference == 0 {
                let id = e.mem.u32(id_slot);
                e.call(LOG_MESSAGE, &args![MESSAGE_DECAL_NOT_FOUND, id]);
                node = drop_unresolved(e, node, previous, item_slot);
            } else {
                e.mem.set_u32(item, reference);
                previous = node;
                node = next_node(e, node);
            }
        }
    });
}

// Typed from 00433ed0
/// `ExtraReflectorRefs::InitItem(owner)`: each entry's form id resolved to a persistent reference
/// (`005653d0`), whose extra list is told of the owner for each of the entry's effect bits 0 and 1
/// (`0041f4c0`, `0041f650`), stored in the entry's first word; an entry whose form is no reference
/// or not persistent logged, removed and deleted.
pub fn reflector_init_item(e: &mut Engine, this: u32, owner: u32) {
    e.with_stack(8, |e, frame| {
        let item_slot = frame.addr();
        let id_slot = item_slot.wrapping_add(4);
        let mut node = this.wrapping_add(OWN_LIST);
        let mut previous = 0;
        while walks(e, node) {
            let item = item_of(e, node);
            e.mem.set_u32(item_slot, item);
            let id = e.mem.u32(item);
            e.mem.set_u32(id_slot, id);
            let reference = resolve_reference(e, owner, id_slot);
            if reference == 0 || !e.call(REFERENCE_PERSISTS, &args![reference]).bool() {
                let id = e.mem.u32(id_slot);
                let message = if reference == 0 {
                    MESSAGE_REFLECTOR_NOT_FOUND
                } else {
                    MESSAGE_NOT_PERSISTENT
                };
                e.call(LOG_MESSAGE, &args![message, id]);
                node = drop_unresolved(e, node, previous, item_slot);
                continue;
            }
            let list = reference.wrapping_add(REFERENCE_EXTRA_LIST);
            // The effect flags are read again after the first setter, as the exe does.
            if e.mem.u32(item.wrapping_add(4)) & 1 != 0 {
                typed_call!(
                    FAMILY,
                    0x0041_f4c0,
                    rooms::set_reflected_flag_1(e, list, owner, true)
                );
            }
            if e.mem.u32(item.wrapping_add(4)) & 2 != 0 {
                typed_call!(
                    FAMILY,
                    0x0041_f650,
                    rooms::set_reflected_flag_2(e, list, owner, true)
                );
            }
            e.mem.set_u32(item, reference);
            previous = node;
            node = next_node(e, node);
        }
    });
}

// Typed from 00434050
/// `ExtraLitWaterRefs::InitItem(owner)`: each form id resolved to a persistent reference, whose
/// extra list is told of the owner (`0041f840`, adding) and which goes at the head of a temporary
/// list; one that is not logged and dropped. The list is then cleared and refilled from the
/// temporary one (the order kept), which is cleared and dropped.
pub fn lit_water_init_item(e: &mut Engine, this: u32, owner: u32) {
    e.with_stack(0x10, |e, frame| {
        let temporary = frame.addr();
        let id_slot = temporary.wrapping_add(8);
        let reference_slot = temporary.wrapping_add(0xc);
        simple_list_init(e, temporary);
        let list = this.wrapping_add(OWN_LIST);
        let mut node = list;
        while walks(e, node) {
            let id = item_of(e, node);
            e.mem.set_u32(id_slot, id);
            let reference = resolve_reference(e, owner, id_slot);
            e.mem.set_u32(reference_slot, reference);
            if reference == 0 || !e.call(REFERENCE_PERSISTS, &args![reference]).bool() {
                let id = e.mem.u32(id_slot);
                let message = if reference == 0 {
                    MESSAGE_LIT_WATER_NOT_FOUND
                } else {
                    MESSAGE_NOT_PERSISTENT
                };
                e.call(LOG_MESSAGE, &args![message, id]);
            } else {
                let extra_list = reference.wrapping_add(REFERENCE_EXTRA_LIST);
                typed_call!(
                    FAMILY,
                    0x0041_f840,
                    rooms::set_water_light_ref(e, extra_list, owner, true)
                );
                e.call(LIST_ADD_HEAD, &args![temporary, reference_slot]);
            }
            node = next_node(e, node);
        }
        e.call(LIST_CLEAR, &args![list]);
        let mut walk = temporary;
        while walk != 0 && item_of(e, walk) != 0 {
            // The node is its own item slot.
            e.call(LIST_ADD_HEAD, &args![list, walk]);
            walk = next_node(e, walk);
        }
        e.call(LIST_CLEAR, &args![temporary]);
        e.call(LIST_DROP, &args![temporary]);
    });
}

/// The decal, reflected and reflector `Compare`s after their cast: as many entries (`005ae380`),
/// and each of the other list's entries with an entry of the same reference here (`00434410`) of
/// the same first `size` bytes. True when they differ.
fn entries_differ(e: &mut Engine, this: u32, other_list: u32, size: u32) -> bool {
    let my_count = e
        .call(LIST_COUNT, &args![this.wrapping_add(OWN_LIST)])
        .u32();
    let their_count = e.call(LIST_COUNT, &args![other_list]).u32();
    if my_count != their_count {
        return true;
    }
    let mut node = other_list;
    while walks(e, node) {
        let item = item_of(e, node);
        let reference = e.mem.u32(item);
        let counterpart = typed_call!(FAMILY, 0x0043_4410, decal_find(e, this, reference));
        if counterpart == 0 || memcmp(e, counterpart, item, size) != 0 {
            return true;
        }
        node = next_node(e, node);
    }
    false
}

// Typed from 004341e0
/// `ExtraDecalRefs::Compare`: the entries, 0x1c bytes each.
pub fn decal_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = cast_to(e, other, TYPE_DECAL_REFS);
    cast == 0 || entries_differ(e, this, cast.wrapping_add(OWN_LIST), 0x1c)
}

// Typed from 004342b0
/// `ExtraDecalRefs::Copy`: from another `ExtraDecalRefs` only, the entries deleted (`004343c0`) and
/// a copy of each of the other's 0x1c-byte entries added at the head (`0055a400` in place).
pub fn decal_copy(e: &mut Engine, this: u32, other: u32) {
    let cast = cast_to(e, other, TYPE_DECAL_REFS);
    if cast == 0 {
        return;
    }
    typed_call!(FAMILY, 0x0043_43c0, decal_clear(e, this));
    let mut node = cast.wrapping_add(OWN_LIST);
    while walks(e, node) {
        let item = item_of(e, node);
        let copy = e.call(OPERATOR_NEW, &args![0x1cu32]).u32();
        copy_bytes(e, copy, item, 0x1c);
        add_head(e, this.wrapping_add(OWN_LIST), copy);
        node = next_node(e, node);
    }
}

// Typed from 004343c0
/// The decal entries deleted: the head removed (`0063f7b0`), then its entry deleted, until the list
/// is empty.
pub fn decal_clear(e: &mut Engine, this: u32) {
    let list = this.wrapping_add(OWN_LIST);
    while !is_empty(e, list) {
        let item = item_of(e, list);
        e.call(LIST_REMOVE_HEAD, &args![list]);
        e.call(OPERATOR_DELETE, &args![item]);
    }
}

// Typed from 00434410
/// `ExtraDecalRefs::GetRefDecalData` (the reflected and reflector classes share the body): the
/// first entry whose reference is `reference`, or null.
pub fn decal_find(e: &mut Engine, this: u32, reference: u32) -> u32 {
    let mut node = this.wrapping_add(OWN_LIST);
    while walks(e, node) {
        let item = item_of(e, node);
        node = next_node(e, node);
        if e.mem.u32(item) == reference {
            return item;
        }
    }
    0
}

// Typed from 00434480
/// The decal data of `reference`: its entry found (`00434410`) or made (0x1c bytes, `0055a400` in
/// place) with the reference and added at the head, then its intersection and normal copied.
pub fn decal_set(e: &mut Engine, this: u32, reference: u32, intersect: u32, normal: u32) {
    let mut entry = typed_call!(FAMILY, 0x0043_4410, decal_find(e, this, reference));
    if entry == 0 {
        entry = e.call(OPERATOR_NEW, &args![0x1cu32]).u32();
        e.mem.set_u32(entry, reference);
        add_head(e, this.wrapping_add(OWN_LIST), entry);
    }
    copy_point(e, intersect, entry.wrapping_add(4));
    copy_point(e, normal, entry.wrapping_add(0x10));
}

// ---- The reflected, reflector, water light and lit water references ----------------------------

// Typed from 00434560
/// `ExtraReflectedRefs`' destructor: every entry deleted, the list cleared.
pub fn reflected_drop(e: &mut Engine, this: u32) {
    drop_owned_entries(e, this, VTABLE_REFLECTED_REFS);
}

// Typed from 00434620
/// `ExtraReflectedRefs::Compare`: the entries, 8 bytes each.
pub fn reflected_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = cast_to(e, other, TYPE_REFLECTED_REFS);
    cast == 0 || entries_differ(e, this, cast.wrapping_add(OWN_LIST), 8)
}

/// The reflected and reflector `Copy`s' loop: a copy of each 8-byte entry of `source` (made an
/// empty node in place, then copied over) added at the head of `list`.
fn copy_entries(e: &mut Engine, list: u32, source: u32) {
    let mut node = source;
    while walks(e, node) {
        let item = item_of(e, node);
        let copy = new_empty_record(e);
        copy_bytes(e, copy, item, 8);
        add_head(e, list, copy);
        node = next_node(e, node);
    }
}

// Typed from 004346f0
/// `ExtraReflectedRefs::Copy`: from another `ExtraReflectedRefs` only, the entries removed without
/// deleting them (`00434d90`) and a copy of each of the other's added at the head.
pub fn reflected_copy(e: &mut Engine, this: u32, other: u32) {
    let cast = cast_to(e, other, TYPE_REFLECTED_REFS);
    if cast == 0 {
        return;
    }
    typed_call!(FAMILY, 0x0043_4d90, clear_reference_data(e, this));
    copy_entries(e, this.wrapping_add(OWN_LIST), cast.wrapping_add(OWN_LIST));
}

// Typed from 00434800
/// The entry of `reference` removed (the head with `0063f7b0`, another after its previous node with
/// `00905330`) and deleted; nothing when there is none (the reflected and reflector classes share
/// the body).
pub fn reflected_remove(e: &mut Engine, this: u32, reference: u32) {
    e.with_stack(4, |e, local| {
        let item_slot = local.addr();
        let mut node = this.wrapping_add(OWN_LIST);
        let mut previous = 0;
        while walks(e, node) {
            let item = item_of(e, node);
            e.mem.set_u32(item_slot, item);
            if e.mem.u32(item) == reference {
                if previous == 0 {
                    e.call(LIST_REMOVE_HEAD, &args![node]);
                } else {
                    e.call(LIST_REMOVE_AFTER, &args![previous, item_slot]);
                }
                let removed = e.mem.u32(item_slot);
                e.call(OPERATOR_DELETE, &args![removed]);
                e.mem.set_u32(item_slot, 0);
                return;
            }
            previous = node;
            node = next_node(e, node);
        }
    });
}

// Typed from 004348a0
/// The effect flags of `reference`'s entry: found (`00434410`) or made (8 bytes, an empty node in
/// place) with the reference and added at the head, then the flags stored (the reflected and
/// reflector classes share the body).
pub fn reflected_set(e: &mut Engine, this: u32, reference: u32, flags: u32) {
    let mut entry = typed_call!(FAMILY, 0x0043_4410, decal_find(e, this, reference));
    if entry == 0 {
        entry = new_empty_record(e);
        e.mem.set_u32(entry, reference);
        add_head(e, this.wrapping_add(OWN_LIST), entry);
    }
    e.mem.set_u32(entry.wrapping_add(4), flags);
}

// Typed from 00434950
/// `ExtraWaterLightRefs`' destructor: the list cleared (the references are not owned).
pub fn water_light_drop(e: &mut Engine, this: u32) {
    drop_list(e, this, VTABLE_WATER_LIGHT_REFS);
}

/// The water light and lit water `Compare`s after their cast: as many entries (`005ae380`), and
/// each reference of the other list in this one (`005f65d0` in place). True when they differ.
fn references_differ(e: &mut Engine, mine: u32, theirs: u32) -> bool {
    let my_count = e.call(LIST_COUNT, &args![mine]).u32();
    let their_count = e.call(LIST_COUNT, &args![theirs]).u32();
    if my_count != their_count {
        return true;
    }
    let mut node = theirs;
    while walks(e, node) {
        if !holds(e, mine, item_of(e, node)) {
            return true;
        }
        node = next_node(e, node);
    }
    false
}

// Typed from 004349c0
/// `ExtraWaterLightRefs::Compare`: the references.
pub fn water_light_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = cast_to(e, other, TYPE_WATER_LIGHT_REFS);
    cast == 0 || references_differ(e, this.wrapping_add(OWN_LIST), cast.wrapping_add(OWN_LIST))
}

/// The water light and lit water `Copy`s: from the class with the descriptor `target` only, the
/// list cleared (`00434fa0`) and each reference of the other's added unless already there
/// (`00434f60`).
fn copy_references(e: &mut Engine, this: u32, other: u32, target: u32) {
    let cast = cast_to(e, other, target);
    if cast == 0 {
        return;
    }
    typed_call!(FAMILY, 0x0043_4fa0, water_clear(e, this));
    let mut node = cast.wrapping_add(OWN_LIST);
    while walks(e, node) {
        let reference = item_of(e, node);
        typed_call!(FAMILY, 0x0043_4f60, water_add(e, this, reference));
        node = next_node(e, node);
    }
}

// Typed from 00434a70
/// `ExtraWaterLightRefs::Copy`.
pub fn water_light_copy(e: &mut Engine, this: u32, other: u32) {
    copy_references(e, this, other, TYPE_WATER_LIGHT_REFS);
}

// Typed from 00434af0
/// `ExtraReflectorRefs`' destructor: every entry deleted, the list cleared.
pub fn reflector_drop(e: &mut Engine, this: u32) {
    drop_owned_entries(e, this, VTABLE_REFLECTOR_REFS);
}

// Typed from 00434bb0
/// `ExtraReflectorRefs::Compare`: the entries, 8 bytes each.
pub fn reflector_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = cast_to(e, other, TYPE_REFLECTOR_REFS);
    cast == 0 || entries_differ(e, this, cast.wrapping_add(OWN_LIST), 8)
}

// Typed from 00434c80
/// `ExtraReflectorRefs::Copy`: from another `ExtraReflectorRefs` only, the entries removed without
/// deleting them (`00434d90`) and a copy of each of the other's added at the head.
pub fn reflector_copy(e: &mut Engine, this: u32, other: u32) {
    let cast = cast_to(e, other, TYPE_REFLECTOR_REFS);
    if cast == 0 {
        return;
    }
    typed_call!(FAMILY, 0x0043_4d90, clear_reference_data(e, this));
    copy_entries(e, this.wrapping_add(OWN_LIST), cast.wrapping_add(OWN_LIST));
}

// Typed from 00434d90
/// `ExtraReflectedRefs::ClearReferenceData` (the reflector class shares the body): the head removed
/// until the list is empty, the entries not deleted.
pub fn clear_reference_data(e: &mut Engine, this: u32) {
    let list = this.wrapping_add(OWN_LIST);
    while !is_empty(e, list) {
        e.call(LIST_REMOVE_HEAD, &args![list]);
    }
}

// Typed from 00434dc0
/// `ExtraLitWaterRefs`' destructor: the list cleared (the references are not owned).
pub fn lit_water_drop(e: &mut Engine, this: u32) {
    drop_list(e, this, VTABLE_LIT_WATER_REFS);
}

// Typed from 00434e30
/// `ExtraLitWaterRefs::Compare`: the references.
pub fn lit_water_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = cast_to(e, other, TYPE_LIT_WATER_REFS);
    cast == 0 || references_differ(e, this.wrapping_add(OWN_LIST), cast.wrapping_add(OWN_LIST))
}

// Typed from 00434ee0
/// `ExtraLitWaterRefs::Copy`.
pub fn lit_water_copy(e: &mut Engine, this: u32, other: u32) {
    copy_references(e, this, other, TYPE_LIT_WATER_REFS);
}

// Typed from 00434f60
/// `reference` added at the head unless the list holds it (`005f65d0` in place; the water light and
/// lit water classes share the body).
pub fn water_add(e: &mut Engine, this: u32, reference: u32) {
    let list = this.wrapping_add(OWN_LIST);
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), reference);
        if !holds(e, list, reference) {
            e.call(LIST_ADD_HEAD, &args![list, local]);
        }
    });
}

// Typed from 00434fa0
/// The list cleared (`00470470`; the water light and lit water classes share the body).
pub fn water_clear(e: &mut Engine, this: u32) {
    e.call(LIST_CLEAR, &args![this.wrapping_add(OWN_LIST)]);
}

// ---- The merchant container, the modifier, the poison, the sequence, the target -----------------

// Typed from 00434fc0
/// `ExtraMerchantContainer`'s constructor (type 0x3c): no container. Answers `this`.
pub fn merchant_new(e: &mut Engine, this: u32) -> u32 {
    with_word(
        e,
        this,
        KIND_MERCHANT_CONTAINER,
        VTABLE_MERCHANT_CONTAINER,
        0,
    )
}

// Typed from 00434ff0
/// `ExtraMerchantContainer::Compare`: the container.
pub fn merchant_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_cast_word(e, this, other, TYPE_MERCHANT_CONTAINER)
}

// Typed from 00435040
/// `ExtraLevCreaModifier`'s constructor (type 0x1e): the modifier "none" (4). Answers `this`.
pub fn lev_crea_new(e: &mut Engine, this: u32) -> u32 {
    with_word(
        e,
        this,
        KIND_LEV_CREA_MODIFIER,
        VTABLE_LEV_CREA_MODIFIER,
        LEV_CREA_NONE,
    )
}

// Typed from 00435070
/// `ExtraLevCreaModifier::Compare`: the modifier.
pub fn lev_crea_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_cast_word(e, this, other, TYPE_LEV_CREA_MODIFIER)
}

// Typed from 004350c0
/// The modifier's multiplier: 1.0 for "none", else the value of the float setting the modifier
/// picks from the table (`00403e20`).
pub fn lev_crea_multiplier(e: &mut Engine, this: u32) -> f32 {
    let modifier = word_at(e, this, EXTRA_PAYLOAD);
    if modifier == LEV_CREA_NONE {
        return 1.0;
    }
    let setting = e
        .mem
        .u32(LEV_CREA_SETTINGS.wrapping_add(modifier.wrapping_mul(4)));
    let value = e.call(SETTING_FLOAT, &args![setting]).u32();
    e.mem.f32(value)
}

// Typed from 00435100
/// The modifier's sign: 1 for easy (0), 0 for "none", -1 for any other.
pub fn lev_crea_sign(e: &mut Engine, this: u32) -> i32 {
    match word_at(e, this, EXTRA_PAYLOAD) {
        0 => 1,
        LEV_CREA_NONE => 0,
        _ => -1,
    }
}

// Typed from 00435150
/// `ExtraPoison`'s constructor (type 0x3f) with its poison. Answers `this`.
pub fn poison_new(e: &mut Engine, this: u32, poison: u32) -> u32 {
    with_word(e, this, KIND_POISON, VTABLE_POISON, poison)
}

// Typed from 00435180
/// `ExtraPoison::Compare`: the poison.
pub fn poison_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    compare_cast_word(e, this, other, TYPE_POISON)
}

// Typed from 004351d0
/// `ExtraLastFinishedSequence`'s constructor (type 0x41): a copy of `name` in a new buffer of its
/// length and one (the forms family's `strlen` wrapper, then `strcpy_s` by address). Answers
/// `this`.
pub fn last_sequence_new(e: &mut Engine, this: u32, name: u32) -> u32 {
    extra_with_vtable(
        e,
        this,
        KIND_LAST_FINISHED_SEQUENCE,
        VTABLE_LAST_FINISHED_SEQUENCE,
    );
    let length = typed_call!(
        forms::FAMILY,
        0x0044_a670,
        forms::debug_string::text_length(e, name)
    );
    let size = length.wrapping_add(1);
    let buffer = e.call(OPERATOR_NEW, &args![size]).u32();
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), buffer);
    e.call(STRCPY_S, &args![buffer, size, name]);
    this
}

// Typed from 00435270
/// `ExtraLastFinishedSequence`'s scalar deleting destructor. Answers `this`.
pub fn last_sequence_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_52a0, last_sequence_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004352a0
/// `ExtraLastFinishedSequence`'s destructor: the name deleted (null or not).
pub fn last_sequence_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_LAST_FINISHED_SEQUENCE);
    let name = word_at(e, this, EXTRA_PAYLOAD);
    e.call(OPERATOR_DELETE, &args![name]);
    extra_drop(e, this);
}

// Typed from 00435310
/// `ExtraLastFinishedSequence::Compare`: `strcmp` of the two names.
pub fn last_sequence_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    let cast = cast_to(e, other, TYPE_LAST_FINISHED_SEQUENCE);
    if cast == 0 {
        return true;
    }
    let mine = word_at(e, this, EXTRA_PAYLOAD);
    let theirs = word_at(e, cast, EXTRA_PAYLOAD);
    e.call(STRCMP, &args![mine, theirs]).u32() != 0
}

// Typed from 00435370
/// `ExtraXTarget`'s constructor (type 0x44): no target. Answers `this`.
pub fn x_target_new(e: &mut Engine, this: u32) -> u32 {
    with_word(e, this, KIND_X_TARGET, VTABLE_X_TARGET, 0)
}

// Typed from 004353a0
/// `ExtraXTarget`'s scalar deleting destructor. Answers `this`.
pub fn x_target_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0043_53d0, x_target_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 004353d0
/// `ExtraXTarget`'s destructor (the target is not owned).
pub fn x_target_drop(e: &mut Engine, this: u32) {
    drop_plain(e, this, VTABLE_X_TARGET);
}

// Typed from 00435ac0
/// `ExtraSavedAnimation`'s destructor: its buffer released (`00435c10`).
pub fn saved_animation_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_SAVED_ANIMATION);
    e.call(SAVED_DELETE_BUFFER, &args![this]);
    extra_drop(e, this);
}

// Typed from 00435bb0
/// `ExtraSavedHavokData`'s destructor: its buffer released (`00435c10`).
pub fn saved_havok_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_SAVED_HAVOK_DATA);
    e.call(SAVED_DELETE_BUFFER, &args![this]);
    extra_drop(e, this);
}
