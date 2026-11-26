//! The part 3 file's first run (`0041db00` to `0041f050`): the extra data that link a reference to
//! others. The enable state parent's flags (type 0x37) and its children (0x38), the item dropper
//! (0x39) and the dropped item list (0x3a), the water type (3), the random teleport marker (0x3b),
//! the ash pile (0x89), the linked reference (0x51) and its children (0x52), the open/close activate
//! reference (0x6c), the activate parents (0x53) with their delays, flag and text, the activate
//! children (0x54) with their timer, and the decal list's getter (0x57).
//!
//! The reference lists are `BSSimpleList`s whose head node sits in the extra data at +0xc. The exe
//! asks `005f65d0` whether a list holds an item (a walk of the nodes, done in place here) and
//! `BSSimpleList::AddHead` (`005ae3d0`) and the item remover (`00905330`) by address; each is given
//! the address of a one-word local holding the item, as the translation has it.

use super::{
    access, extra_init, first_of_kind, list_add, list_remove, new_object, records, setters,
    word_at, EXTRA_PAYLOAD, FAMILY, OPERATOR_DELETE,
};
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The extra data types (`cEtype`, Xbox PDB `EXTRA_DATA_TYPE`) of this file.
const KIND_WATER_TYPE: u8 = 0x03;
const KIND_ENABLE_STATE_PARENT: u8 = 0x37;
const KIND_ENABLE_STATE_CHILDREN: u8 = 0x38;
const KIND_ITEM_DROPPER: u8 = 0x39;
const KIND_DROPPED_ITEM_LIST: u8 = 0x3a;
const KIND_TELEPORT_MARKER: u8 = 0x3b;
const KIND_LINKED_REF: u8 = 0x51;
const KIND_LINKED_REF_CHILDREN: u8 = 0x52;
const KIND_ACTIVATE_REF: u8 = 0x53;
const KIND_ACTIVATE_REF_CHILDREN: u8 = 0x54;
const KIND_DECAL_REFS: u8 = 0x57;
const KIND_OPEN_CLOSE_ACTIVATE_REF: u8 = 0x6c;
const KIND_ASH_PILE_REF: u8 = 0x89;

/// The constructors (other units', `thiscall(block)`) the setters build their extra data with.
const ENABLE_STATE_CHILDREN_INIT: u32 = 0x0043_33a0;
const ITEM_DROPPER_INIT: u32 = 0x0043_5920;
const DROPPED_ITEM_LIST_INIT: u32 = 0x0043_5950;
const WATER_TYPE_INIT: u32 = 0x0040_f370;
const TELEPORT_MARKER_INIT: u32 = 0x0043_34b0;
const ASH_PILE_REF_INIT: u32 = 0x0043_36c0;
const LINKED_REF_INIT: u32 = 0x0043_3640;
const LINKED_REF_CHILDREN_INIT: u32 = 0x0043_3530;
const ACTIVATE_REF_INIT: u32 = 0x0043_38b0;
const ACTIVATE_REF_CHILDREN_INIT: u32 = 0x0043_3790;
/// `ExtraOpenCloseActivateRef`'s vtable, which its constructor (`0041e750`) sets.
const VTABLE_EXTRA_OPEN_CLOSE_ACTIVATE_REF: u32 = 0x0101_51a8;

/// `BSSimpleList::AddHead(&item)` and the remover of the first node holding the item, both
/// `thiscall(head, &item)`; the remover of a list's head node (`thiscall(node)`).
pub(super) const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
pub(super) const LIST_REMOVE_ITEM: u32 = 0x0090_5330;
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
/// `ExtraActivateRef`'s search of its parents (`thiscall(extra, reference)`): the entry whose
/// reference it is, or null.
const ACTIVATE_REF_FIND: u32 = 0x0043_3a00;
/// The form type the dropped item search looks for.
const FORM_TYPE_DROPPED_MATCH: u8 = 0x28;
/// `BSStringT<char>`'s copy constructor (`thiscall(string, source)`), its constructor from a C
/// string (`thiscall(string, text)`) and its set from one (`thiscall(string, text, 0)`); the empty C
/// string in the read-only data.
const STRING_COPY_NEW: u32 = 0x0040_47f0;
const STRING_FROM_CHARS_NEW: u32 = 0x0040_c0e0;
const STRING_SET: u32 = 0x0040_37f0;
const EMPTY_TEXT: u32 = 0x0101_1584;
/// The -1.0 the children's timer getter answers without the extra data.
const NO_TIMER: u32 = 0x0101_2054;

/// The offsets this file reads: the enable state parent's flags, the activate reference's flag
/// byte and text, the activate children's timer, a `REF_ACTIVATE_DATA`'s delay.
const PARENT_FLAGS: u32 = 0x10;
const ACTIVATE_FLAGS: u32 = 0x14;
const ACTIVATE_TEXT: u32 = 0x18;
const CHILDREN_TIMER: u32 = 0x14;
const ACTIVATE_DELAY: u32 = 4;

// ---- What the file shares (with the part 3 rooms file) --------------------------------------

/// A new block of `size` bytes built by the constructor at `construct` (other units', by address),
/// null when the allocation failed.
#[inline]
pub(super) fn new_by(e: &mut Engine, size: u32, construct: u32) -> u32 {
    new_object(e, size, |e, block| e.call(construct, &args![block]).u32())
}

/// Whether the list whose head node is `node` holds `item`: `005f65d0` in place (the nodes walked
/// to the first whose item is `item`; a null head holds nothing).
#[inline]
pub(super) fn holds(e: &Engine, mut node: u32, item: u32) -> bool {
    while node != 0 && e.mem.u32(node) != item {
        node = e.mem.u32(node.wrapping_add(4));
    }
    node != 0
}

/// `BSSimpleList::IsEmpty` (`008256d0`) in place: a head node with no item and no next.
#[inline]
pub(super) fn is_empty(e: &Engine, node: u32) -> bool {
    e.mem.u32(node.wrapping_add(4)) == 0 && e.mem.u32(node) == 0
}

/// `item` put at the head of the list at `head` unless it holds it, through a one-word local.
pub(super) fn add_unless_held(e: &mut Engine, head: u32, item: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        if !holds(e, head, item) {
            e.call(LIST_ADD_HEAD, &args![head, slot]);
        }
    });
}

/// The first node holding `item` taken out of the list at `head` (`00905330`), through a one-word
/// local.
pub(super) fn remove_item(e: &mut Engine, head: u32, item: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), item);
        e.call(LIST_REMOVE_ITEM, &args![head, slot]);
    });
}

/// The setters whose null deletes by type: 0 deletes the first extra data of `kind`, another value
/// is stored at +0xc of the one there or of a new one (0x10 bytes, `construct`) added after.
fn set_or_remove_kind(e: &mut Engine, list: u32, kind: u8, construct: u32, value: u32) {
    if value == 0 {
        setters::remove_kind(e, list, kind);
        return;
    }
    match first_of_kind(e, list, kind) {
        0 => {
            let block = new_by(e, 0x10, construct);
            e.mem.set_u32(block.wrapping_add(EXTRA_PAYLOAD), value);
            list_add(e, list, block);
        }
        extra => e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), value),
    }
}

/// The adders to a reference list: a non-null `item` put at the head of the list at +0xc of the
/// extra data of `kind` (the one there, or a new one of 0x14 bytes from `construct`, added) unless
/// it holds it.
fn add_reference(e: &mut Engine, list: u32, kind: u8, construct: u32, item: u32) {
    if item == 0 {
        return;
    }
    let mut extra = first_of_kind(e, list, kind);
    if extra == 0 {
        extra = new_by(e, 0x14, construct);
        list_add(e, list, extra);
    }
    add_unless_held(e, extra.wrapping_add(EXTRA_PAYLOAD), item);
}

/// The removers from a reference list: a non-null `item` taken out of the list at +0xc of the
/// first extra data of `kind`, which is deleted when that leaves its list empty.
fn remove_reference(e: &mut Engine, list: u32, kind: u8, item: u32) {
    if item == 0 {
        return;
    }
    let extra = first_of_kind(e, list, kind);
    if extra == 0 {
        return;
    }
    let head = extra.wrapping_add(EXTRA_PAYLOAD);
    remove_item(e, head, item);
    if is_empty(e, head) {
        list_remove(e, list, extra, true);
    }
}

/// The enable state parent's flags byte (+0x10 of the type 0x37 extra data) tested for `bit`;
/// false without the extra data. The exe's flag getters (`0041db30`, `0041dc00`, `0041dc50`) are
/// done in place.
#[inline]
fn parent_flag(e: &mut Engine, list: u32, bit: u8) -> bool {
    match first_of_kind(e, list, KIND_ENABLE_STATE_PARENT) {
        0 => false,
        extra => e.mem.u8(extra.wrapping_add(PARENT_FLAGS)) & bit != 0,
    }
}

/// Bit 0 of the byte at `at` set or cleared (the flag setters' `OR 1` and `AND 0xfe`).
#[inline]
fn set_bit_0(e: &mut Engine, at: u32, value: bool) {
    let flags = e.mem.u8(at);
    e.mem
        .set_u8(at, if value { flags | 1 } else { flags & 0xfe });
}

/// `ExtraOpenCloseActivateRef`'s constructor's body (`0041e750`): `BSExtraData`'s (`0040ec80`) with
/// type 0x6c, the class's vtable, a null reference. Answers `this`.
#[inline]
fn open_close_init(e: &mut Engine, this: u32) -> u32 {
    extra_init(e, this, KIND_OPEN_CLOSE_ACTIVATE_REF);
    e.mem.set_u32(this, VTABLE_EXTRA_OPEN_CLOSE_ACTIVATE_REF);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
    this
}

/// A new `REF_ACTIVATE_DATA` (8 bytes, the empty node constructor `00414010`), null when the
/// allocation failed.
#[inline]
fn new_activate_data(e: &mut Engine) -> u32 {
    new_object(e, 8, |e, block| {
        typed_call!(FAMILY, 0x0041_4010, records::empty_node_new(e, block))
    })
}

/// The activate reference's entry for `reference` (`00433a00`, by address), or null.
#[inline]
fn activate_entry(e: &mut Engine, extra: u32, reference: u32) -> u32 {
    e.call(ACTIVATE_REF_FIND, &args![extra, reference]).u32()
}

/// A new activate reference extra data (0x20 bytes, `004338b0`), added to the list.
#[inline]
fn add_activate_ref(e: &mut Engine, list: u32) -> u32 {
    let extra = new_by(e, 0x20, ACTIVATE_REF_INIT);
    list_add(e, list, extra);
    extra
}

/// `entry` given `reference` and put at the head of the list at +0xc of `extra`, through a one-word
/// local holding the entry.
fn push_entry(e: &mut Engine, extra: u32, entry: u32, reference: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), entry);
        e.mem.set_u32(entry, reference);
        e.call(
            LIST_ADD_HEAD,
            &args![extra.wrapping_add(EXTRA_PAYLOAD), slot],
        );
    });
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 0041db00
/// Bit 0 of the enable state parent's flags (`0041db30`, in place); false without one.
pub fn parent_flag_0(e: &mut Engine, this: u32) -> bool {
    parent_flag(e, this, 1)
}

// Typed from 0041db30
/// Bit 0 of an enable state parent's flags (`cFlags`, +0x10).
pub fn parent_flags_bit_0(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this.wrapping_add(PARENT_FLAGS)) & 1 != 0
}

// Typed from 0041db50
/// Bit 0 of the enable state parent's flags set to `value` (`0041db80`, in place); nothing without
/// one.
pub fn set_parent_flag_0(e: &mut Engine, this: u32, value: bool) {
    let extra = first_of_kind(e, this, KIND_ENABLE_STATE_PARENT);
    if extra != 0 {
        set_bit_0(e, extra.wrapping_add(PARENT_FLAGS), value);
    }
}

// Typed from 0041db80
/// Bit 0 of an enable state parent's flags set or cleared.
pub fn set_parent_flags_bit_0(e: &mut Engine, this: u32, value: bool) {
    set_bit_0(e, this.wrapping_add(PARENT_FLAGS), value);
}

// Typed from 0041dbd0
/// `ExtraDataList::ShouldPopInWhenEnabledByParent` (Xbox PDB): bit 1 of the enable state parent's
/// flags (`0041dc00`, in place); false without one.
pub fn should_pop_in_when_enabled_by_parent(e: &mut Engine, this: u32) -> bool {
    parent_flag(e, this, 2)
}

// Typed from 0041dc00
/// Bit 1 of an enable state parent's flags.
pub fn parent_flags_bit_1(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this.wrapping_add(PARENT_FLAGS)) & 2 != 0
}

// Typed from 0041dc20
/// Bit 2 of the enable state parent's flags (`0041dc50`, in place); false without one.
pub fn parent_flag_2(e: &mut Engine, this: u32) -> bool {
    parent_flag(e, this, 4)
}

// Typed from 0041dc50
/// Bit 2 of an enable state parent's flags.
pub fn parent_flags_bit_2(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this.wrapping_add(PARENT_FLAGS)) & 4 != 0
}

// Typed from 0041dc70
/// The enable state parent's whole flags byte set to `flags`; nothing without one.
pub fn set_parent_flags(e: &mut Engine, this: u32, flags: u8) {
    let extra = first_of_kind(e, this, KIND_ENABLE_STATE_PARENT);
    if extra != 0 {
        e.mem.set_u8(extra.wrapping_add(PARENT_FLAGS), flags);
    }
}

// Typed from 0041dca0
/// The enable state children's list (the head node at +0xc of the type 0x38 extra data), or null.
pub fn enable_children(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_ENABLE_STATE_CHILDREN)
}

// Typed from 0041dcd0
/// A non-null `child` added to the enable state children (a new extra data of 0x14 bytes, `004333a0`,
/// when the list has none) unless they hold it.
pub fn add_enable_child(e: &mut Engine, this: u32, child: u32) {
    add_reference(
        e,
        this,
        KIND_ENABLE_STATE_CHILDREN,
        ENABLE_STATE_CHILDREN_INIT,
        child,
    );
}

// Typed from 0041dda0
/// A non-null `child` taken out of the enable state children, which are deleted when none is left.
pub fn remove_enable_child(e: &mut Engine, this: u32, child: u32) {
    remove_reference(e, this, KIND_ENABLE_STATE_CHILDREN, child);
}

// Typed from 0041de00
/// `ExtraDataList::GetItemDropper` (Xbox PDB): the dropper (+0xc of the type 0x39 extra data), or 0.
pub fn item_dropper(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_ITEM_DROPPER, 0)
}

// Typed from 0041de40
/// `ExtraDataList::AddDroppedItem` (Xbox PDB; it sets the dropper): a null `dropper` deletes the
/// type 0x39 extra data by type; another is stored in the one there, or in a new one (0x10 bytes,
/// `00435920`) after it is added.
pub fn add_dropped_item(e: &mut Engine, this: u32, dropper: u32) {
    if dropper == 0 {
        setters::remove_kind(e, this, KIND_ITEM_DROPPER);
        return;
    }
    let mut extra = first_of_kind(e, this, KIND_ITEM_DROPPER);
    if extra == 0 {
        extra = new_by(e, 0x10, ITEM_DROPPER_INIT);
        list_add(e, this, extra);
    }
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), dropper);
}

// Typed from 0041df00
/// The first reference of the dropped item list whose base form (+0x20, `007af430` in place) has
/// form type 0x28 (`00401170` in place); 0 without the list, at its first null item, or with no
/// match.
pub fn first_dropped_of_type_28(e: &mut Engine, this: u32) -> u32 {
    let extra = first_of_kind(e, this, KIND_DROPPED_ITEM_LIST);
    if extra == 0 {
        return 0;
    }
    let mut node = extra.wrapping_add(EXTRA_PAYLOAD);
    while node != 0 {
        let item = e.mem.u32(node);
        if item == 0 {
            return 0;
        }
        let form = word_at(e, item, 0x20);
        if e.mem.u8(form.wrapping_add(4)) == FORM_TYPE_DROPPED_MATCH {
            return item;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    0
}

// Typed from 0041df90
/// `ExtraDataList::GetDroppedItemList` (Xbox PDB): the head node at +0xc of the type 0x3a extra data,
/// or null.
pub fn dropped_item_list(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_DROPPED_ITEM_LIST)
}

// Typed from 0041dfd0
/// `ExtraDataList::RemoveDroppedItemList` (Xbox PDB): the type 0x3a extra data deleted by type when
/// the list has one.
pub fn remove_dropped_item_list(e: &mut Engine, this: u32) {
    if first_of_kind(e, this, KIND_DROPPED_ITEM_LIST) != 0 {
        setters::remove_kind(e, this, KIND_DROPPED_ITEM_LIST);
    }
}

// Typed from 0041e000
/// A non-null `item` added to the dropped item list (a new extra data of 0x14 bytes, `00435950`,
/// when the list has none) unless it holds it.
pub fn add_to_dropped_items(e: &mut Engine, this: u32, item: u32) {
    add_reference(
        e,
        this,
        KIND_DROPPED_ITEM_LIST,
        DROPPED_ITEM_LIST_INIT,
        item,
    );
}

// Typed from 0041e0d0
/// `ExtraDataList::RemoveDroppedItem` (Xbox PDB): a non-null `item` taken out of the dropped item
/// list, which is deleted when none is left.
pub fn remove_dropped_item(e: &mut Engine, this: u32, item: u32) {
    remove_reference(e, this, KIND_DROPPED_ITEM_LIST, item);
}

// Typed from 0041e130
/// `ExtraDataList::GetWaterType` (Xbox PDB): the water (+0xc of the type 3 extra data), or 0.
pub fn water_type(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_WATER_TYPE, 0)
}

// Typed from 0041e160
/// The water set: a null one deletes the type 3 extra data by type, another is stored in the one
/// there or in a new one (0x10 bytes, `0040f370`) added after.
pub fn set_water_type(e: &mut Engine, this: u32, water: u32) {
    set_or_remove_kind(e, this, KIND_WATER_TYPE, WATER_TYPE_INIT, water);
}

// Typed from 0041e220
/// The random teleport marker (+0xc of the type 0x3b extra data), or 0.
pub fn teleport_marker(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_TELEPORT_MARKER, 0)
}

// Typed from 0041e250
/// The random teleport marker set as the water is (the type 0x3b extra data, `004334b0`).
pub fn set_teleport_marker(e: &mut Engine, this: u32, marker: u32) {
    set_or_remove_kind(e, this, KIND_TELEPORT_MARKER, TELEPORT_MARKER_INIT, marker);
}

// Typed from 0041e310
/// `ExtraDataList::GetAshPileRef` (Xbox PDB): the ash pile (+0xc of the type 0x89 extra data), or 0.
pub fn ash_pile_ref(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_ASH_PILE_REF, 0)
}

// Typed from 0041e340
/// The ash pile set: the type 0x89 extra data looked up first; a null `reference` deletes the one
/// there, another is stored in it or in a new one (0x10 bytes, `004336c0`) after it is added.
pub fn set_ash_pile_ref(e: &mut Engine, this: u32, reference: u32) {
    let mut extra = first_of_kind(e, this, KIND_ASH_PILE_REF);
    if reference == 0 {
        if extra != 0 {
            list_remove(e, this, extra, true);
        }
        return;
    }
    if extra == 0 {
        extra = new_by(e, 0x10, ASH_PILE_REF_INIT);
        list_add(e, this, extra);
    }
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), reference);
}

// Typed from 0041e410
/// The linked reference (+0xc of the type 0x51 extra data), or 0.
pub fn linked_ref(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_LINKED_REF, 0)
}

// Typed from 0041e440
/// The linked reference set as the water is (the type 0x51 extra data, `00433640`).
pub fn set_linked_ref(e: &mut Engine, this: u32, reference: u32) {
    set_or_remove_kind(e, this, KIND_LINKED_REF, LINKED_REF_INIT, reference);
}

// Typed from 0041e500
/// `ExtraDataList::GetLinkedRefChildren` (Xbox PDB): the head node at +0xc of the type 0x52 extra
/// data, or null.
pub fn linked_ref_children(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_LINKED_REF_CHILDREN)
}

// Typed from 0041e530
/// A non-null `child` added to the linked reference's children (a new extra data of 0x14 bytes,
/// `00433530`, when the list has none) unless they hold it.
pub fn add_linked_ref_child(e: &mut Engine, this: u32, child: u32) {
    add_reference(
        e,
        this,
        KIND_LINKED_REF_CHILDREN,
        LINKED_REF_CHILDREN_INIT,
        child,
    );
}

// Typed from 0041e600
/// A non-null `child` taken out of the linked reference's children, deleted when none is left.
pub fn remove_linked_ref_child(e: &mut Engine, this: u32, child: u32) {
    remove_reference(e, this, KIND_LINKED_REF_CHILDREN, child);
}

// Typed from 0041e660
/// The open/close activate reference (+0xc of the type 0x6c extra data), or 0.
pub fn open_close_activate_ref(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_OPEN_CLOSE_ACTIVATE_REF, 0)
}

// Typed from 0041e690
/// `ExtraDataList::SetOpenCloseActivateRef` (Xbox PDB): a null `reference` deletes the type 0x6c
/// extra data by type; another is stored in the one there, or in a new one (0x10 bytes, its
/// constructor `0041e750` in place) added after.
pub fn set_open_close_activate_ref(e: &mut Engine, this: u32, reference: u32) {
    if reference == 0 {
        setters::remove_kind(e, this, KIND_OPEN_CLOSE_ACTIVATE_REF);
        return;
    }
    match first_of_kind(e, this, KIND_OPEN_CLOSE_ACTIVATE_REF) {
        0 => {
            let block = new_object(e, 0x10, open_close_init);
            e.mem.set_u32(block.wrapping_add(EXTRA_PAYLOAD), reference);
            list_add(e, this, block);
        }
        extra => e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), reference),
    }
}

// Typed from 0041e750
/// `ExtraOpenCloseActivateRef`'s constructor: `BSExtraData`'s (`0040ec80`, in place) with type 0x6c,
/// its vtable, a null reference. Answers `this`.
pub fn open_close_activate_ref_new(e: &mut Engine, this: u32) -> u32 {
    open_close_init(e, this)
}

// Typed from 0041e780
/// The activate parents' list (the head node at +0xc of the type 0x53 extra data), or null.
pub fn activate_parents(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_ACTIVATE_REF)
}

// Typed from 0041e7b0
/// The activate parents' entry for `reference` (`00433a00`), or 0 without the extra data.
pub fn activate_parent_entry(e: &mut Engine, this: u32, reference: u32) -> u32 {
    match first_of_kind(e, this, KIND_ACTIVATE_REF) {
        0 => 0,
        extra => activate_entry(e, extra, reference),
    }
}

// Typed from 0041e7f0
/// A non-null `reference` added as an activate parent: a new entry (8 bytes, `00414010`) holding it
/// at the head of the type 0x53 extra data's list (a new extra data, added, when the list has
/// none), unless the one there has an entry for it.
pub fn add_activate_parent(e: &mut Engine, this: u32, reference: u32) {
    if reference == 0 {
        return;
    }
    let extra = match first_of_kind(e, this, KIND_ACTIVATE_REF) {
        0 => add_activate_ref(e, this),
        extra => {
            if activate_entry(e, extra, reference) != 0 {
                return;
            }
            extra
        }
    };
    let entry = new_activate_data(e);
    push_entry(e, extra, entry, reference);
}

// Typed from 0041e960
/// `ExtraDataList::RemoveActivateParent` (Xbox PDB): the entry for a non-null `reference` taken out
/// of the activate parents and freed; the type 0x53 extra data deleted by type when none is left.
pub fn remove_activate_parent(e: &mut Engine, this: u32, reference: u32) {
    if reference == 0 {
        return;
    }
    let extra = first_of_kind(e, this, KIND_ACTIVATE_REF);
    if extra == 0 {
        return;
    }
    let entry = activate_entry(e, extra, reference);
    if entry == 0 {
        return;
    }
    let head = extra.wrapping_add(EXTRA_PAYLOAD);
    remove_item(e, head, entry);
    e.call(OPERATOR_DELETE, &args![entry]);
    if is_empty(e, head) {
        setters::remove_kind(e, this, KIND_ACTIVATE_REF);
    }
}

// Typed from 0041e9e0
/// The activate delay (+4) of the entry for `reference`, or 0.0 without the extra data or the
/// entry; through the x87's load, so a signalling NaN comes out quiet.
pub fn activate_delay(e: &mut Engine, this: u32, reference: u32) -> f32 {
    let extra = first_of_kind(e, this, KIND_ACTIVATE_REF);
    if extra != 0 {
        let entry = activate_entry(e, extra, reference);
        if entry != 0 {
            let delay = word_at(e, entry, ACTIVATE_DELAY);
            return f32::from_bits(x87_quiet_bits(delay));
        }
    }
    0.0
}

// Typed from 0041ea30
/// The activate delay of the entry for a non-null `reference` set: the type 0x53 extra data built
/// and added when the list has none; without an entry for it, a new one (8 bytes, `00414010`) goes
/// to the head of its list and is given the reference; then `delay` is stored (through the x87, so
/// a signalling NaN is stored quiet).
pub fn set_activate_delay(e: &mut Engine, this: u32, reference: u32, delay: f32) {
    if reference == 0 {
        return;
    }
    let (extra, mut entry) = match first_of_kind(e, this, KIND_ACTIVATE_REF) {
        0 => (add_activate_ref(e, this), 0),
        extra => (extra, activate_entry(e, extra, reference)),
    };
    if entry == 0 {
        entry = new_activate_data(e);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), entry);
            e.call(
                LIST_ADD_HEAD,
                &args![extra.wrapping_add(EXTRA_PAYLOAD), slot],
            );
        });
        e.mem.set_u32(entry, reference);
    }
    e.mem.set_u32(
        entry.wrapping_add(ACTIVATE_DELAY),
        x87_quiet_bits(delay.to_bits()),
    );
}

// Typed from 0041eb60
/// Bit 0 of the activate reference's flags (+0x14 of the type 0x53 extra data); false without one.
pub fn activate_flag(e: &mut Engine, this: u32) -> bool {
    match first_of_kind(e, this, KIND_ACTIVATE_REF) {
        0 => false,
        extra => e.mem.u8(extra.wrapping_add(ACTIVATE_FLAGS)) & 1 != 0,
    }
}

// Typed from 0041eba0
/// Bit 0 of the activate reference's flags set (`flag`, the extra data built and added when the
/// list has none) or cleared (nothing without one).
pub fn set_activate_flag(e: &mut Engine, this: u32, flag: bool) {
    let mut extra = first_of_kind(e, this, KIND_ACTIVATE_REF);
    if extra == 0 && flag {
        extra = add_activate_ref(e, this);
    }
    if extra != 0 {
        set_bit_0(e, extra.wrapping_add(ACTIVATE_FLAGS), flag);
    }
}

// Typed from 0041ec80
/// The activate text built in `out` (a `BSStringT<char>`): a copy of the activate reference's
/// (+0x18, `004047f0`), or an empty one (`0040c0e0`) without the extra data. Answers `out`.
pub fn activate_text(e: &mut Engine, this: u32, out: u32) -> u32 {
    match first_of_kind(e, this, KIND_ACTIVATE_REF) {
        0 => e.call(STRING_FROM_CHARS_NEW, &args![out, EMPTY_TEXT]),
        extra => e.call(
            STRING_COPY_NEW,
            &args![out, extra.wrapping_add(ACTIVATE_TEXT)],
        ),
    };
    out
}

// Typed from 0041ece0
/// The activate text set from the C string `text` (`004037f0`, second word 0): the extra data built
/// and added first for a non-null `text` when the list has none; a null `text` reaches the one
/// there as it is.
pub fn set_activate_text(e: &mut Engine, this: u32, text: u32) {
    let mut extra = first_of_kind(e, this, KIND_ACTIVATE_REF);
    if extra == 0 && text != 0 {
        extra = add_activate_ref(e, this);
    }
    if extra != 0 {
        e.call(
            STRING_SET,
            &args![extra.wrapping_add(ACTIVATE_TEXT), text, 0u32],
        );
    }
}

// Typed from 0041eda0
/// `ExtraDataList::GetActivateRefChildren` (Xbox PDB): the head node at +0xc of the type 0x54 extra
/// data, or null.
pub fn activate_children(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_ACTIVATE_REF_CHILDREN)
}

// Typed from 0041edd0
/// `ExtraDataList::AddActivateRefChild` (Xbox PDB): a non-null `child` added as a new entry (8 bytes,
/// `00414010`) at the head of the activate children (a new extra data of 0x18 bytes, `00433790`,
/// added when the list has none) unless an entry before the first empty node names it.
pub fn add_activate_child(e: &mut Engine, this: u32, child: u32) {
    if child == 0 {
        return;
    }
    let mut extra = first_of_kind(e, this, KIND_ACTIVATE_REF_CHILDREN);
    if extra == 0 {
        extra = new_by(e, 0x18, ACTIVATE_REF_CHILDREN_INIT);
        list_add(e, this, extra);
    }
    let mut node = extra.wrapping_add(EXTRA_PAYLOAD);
    while node != 0 && !is_empty(e, node) {
        let entry = e.mem.u32(node);
        if e.mem.u32(entry) == child {
            return;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    let entry = new_activate_data(e);
    push_entry(e, extra, entry, child);
}

// Typed from 0041ef20
/// The entry naming a non-null `child` (before the first empty node) taken out of the activate
/// children with the head remover on its own node (`0063f7b0`: the exe's previous node is never set)
/// and freed; the extra data deleted when its list is then empty, whether or not one was found.
pub fn remove_activate_child(e: &mut Engine, this: u32, child: u32) {
    if child == 0 {
        return;
    }
    let extra = first_of_kind(e, this, KIND_ACTIVATE_REF_CHILDREN);
    if extra == 0 {
        return;
    }
    let head = extra.wrapping_add(EXTRA_PAYLOAD);
    let mut node = head;
    while node != 0 && !is_empty(e, node) {
        let entry = e.mem.u32(node);
        if e.mem.u32(entry) == child {
            e.call(LIST_REMOVE_HEAD, &args![node]);
            e.call(OPERATOR_DELETE, &args![entry]);
            break;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    if is_empty(e, head) {
        list_remove(e, this, extra, true);
    }
}

// Typed from 0041eff0
/// `ExtraDataList::SetActivateChildrenTimer` (Xbox PDB): the timer (+0x14 of the type 0x54 extra
/// data) set, through the x87 (a signalling NaN is stored quiet); nothing without one.
pub fn set_activate_children_timer(e: &mut Engine, this: u32, timer: f32) {
    let extra = first_of_kind(e, this, KIND_ACTIVATE_REF_CHILDREN);
    if extra != 0 {
        e.mem.set_u32(
            extra.wrapping_add(CHILDREN_TIMER),
            x87_quiet_bits(timer.to_bits()),
        );
    }
}

// Typed from 0041f020
/// `ExtraDataList::GetActivateChildrenTimer` (Xbox PDB): the timer, or the -1.0 at `01012054`
/// without the extra data; through the x87's load, so a signalling NaN comes out quiet.
pub fn activate_children_timer(e: &mut Engine, this: u32) -> f32 {
    let timer = match first_of_kind(e, this, KIND_ACTIVATE_REF_CHILDREN) {
        0 => e.mem.u32(NO_TIMER),
        extra => word_at(e, extra, CHILDREN_TIMER),
    };
    f32::from_bits(x87_quiet_bits(timer))
}

// Typed from 0041f050
/// `ExtraDataList::GetDecalRefs` (Xbox PDB): the head node at +0xc of the type 0x57 extra data, or
/// null.
pub fn decal_refs(e: &mut Engine, this: u32) -> u32 {
    access::record_or_null(e, this, KIND_DECAL_REFS)
}
