//! `TESAnimation`'s functions (in the unit after `TESAIForm`'s, `run_0047f940`), each as the exe
//! does it.
//!
//! `TESAnimation` is a form component (at +0xc4 of `TESNPC` and `TESCreature`, 0xc bytes): its
//! vtable (`0101bf50`), then the KF file names (`kfFilenames`, Xbox PDB), a
//! `BSSimpleList<char const *>` whose head node is inside the object at +4 (the name, then the
//! next node). The list is empty when both words of the head are zero.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): the list's empty test (`008256d0`, both words of the head zero), a node's
//! item slot (`006815c0`, the node itself) and next node (`00726070`, its word at +4), the list
//! head's constructor (`0096a2d0`, both words zeroed), the base component's constructor
//! (`00402d30`, whose vtable the class's own replaces at once) and a file's current chunk size
//! (`00401660`, its word at +0x25c). This unit's functions call each other as typed calls, and the
//! copy's call of `ClearDataComponent` through its vtable slot is this unit's typed call when the
//! slot holds it. What stays a call by address: the RTTI cast, the list's removal and append, its
//! destructor, the allocator, the string length, copy and case-blind compare, and the file's chunk
//! type and data.
//!
//! No floats.

use super::FAMILY;
use crate::slot_target;
use engine_core::{args, typed_call, Engine};

const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `__RTDynamicCast` (cdecl: object, vftable delta, source type, target type, is-reference).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_BASE_FORM_COMPONENT: u32 = 0x0118_3040;
const TYPE_TES_ANIMATION: u32 = 0x0118_584c;
const ANIMATION_VTABLE: u32 = 0x0101_bf50;
/// `ClearDataComponent`'s slot in the component vtable.
const SLOT_CLEAR_DATA_COMPONENT: u32 = 4;
/// The KF file names' list head.
const NAMES: u32 = 4;
/// The list's operations: its destructor, the first item's removal, the append of the item a
/// word holds (`ECX` = the list).
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
const LIST_POP_FRONT: u32 = 0x0063_f7b0;
const LIST_APPEND: u32 = 0x0090_5820;
/// The strings' length (cdecl), `strcpy_s` (cdecl: destination, size, source) and case-blind
/// compare (cdecl: zero when equal).
const STRING_LENGTH: u32 = 0x0044_a670;
const STRING_COPY: u32 = 0x0040_6d30;
const STRING_COMPARE_NO_CASE: u32 = 0x0040_4dc0;
/// A file's current chunk type (`TESFile::GetTESChunk`), its size (+0x25c, `00401660`) and its
/// data (`TESFile::GetChunkData`: buffer, maximum size, 0 for none).
const FILE_CHUNK_TYPE: u32 = 0x0047_26b0;
const FILE_CHUNK_SIZE: u32 = 0x25c;
const FILE_CHUNK_DATA: u32 = 0x0047_2890;
/// The chunk of the KF file names ("KFFZ").
const CHUNK_KF_FILE_NAMES: u32 = 0x5a46_464b;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

/// `other` as a `TESAnimation` (the RTTI cast from `BaseFormComponent`), or 0.
fn as_animation(e: &mut Engine, other: u32) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![
            other,
            0u32,
            TYPE_BASE_FORM_COMPONENT,
            TYPE_TES_ANIMATION,
            0u32
        ],
    )
    .u32()
}

/// A list node's name (its item slot, `006815c0`) and the next node (`00726070`).
fn name_of(e: &Engine, node: u32) -> u32 {
    e.mem.u32(node)
}

fn next_of(e: &Engine, node: u32) -> u32 {
    e.mem.u32(at(node, 4))
}

/// Whether the list at `list` is empty (`008256d0`: both words of the head zero).
fn is_empty(e: &Engine, list: u32) -> bool {
    e.mem.u32(at(list, 4)) == 0 && e.mem.u32(list) == 0
}

// Typed from 0047f9c0
/// `TESAnimation`'s constructor: its vtable (over the base component's) and an empty name list.
/// Returns `this`.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, ANIMATION_VTABLE);
    e.mem.set_u32(at(this, NAMES), 0);
    e.mem.set_u32(at(this, NAMES + 4), 0);
    this
}

// Typed from 0047f9f0
/// `TESAnimation`'s destructor: its vtable, `ClearDataComponent`, then the list's destructor.
pub fn destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, ANIMATION_VTABLE);
    typed_call!(FAMILY, 0x0047_fa50, clear_data_component(e, this));
    e.call(LIST_DESTRUCT, &args![at(this, NAMES)]);
}

// Typed from 0047fa50
/// `TESAnimation::ClearDataComponent` (Xbox PDB): each name removed from the list's front and
/// freed, until the list is empty.
pub fn clear_data_component(e: &mut Engine, this: u32) {
    let list = at(this, NAMES);
    while !is_empty(e, list) {
        let name = name_of(e, list);
        e.call(LIST_POP_FRONT, &args![list]);
        e.call(OPERATOR_DELETE, &args![name]);
    }
}

// Typed from 0047faa0
/// The KF file names' chunk read: with both `component` (the caller's `this` again, or null) and
/// `file` set and the file's current chunk "KFFZ", the chunk (its size from the file) read into a
/// buffer of its own, each name in it up to an empty one added, and the buffer freed.
pub fn load_names(e: &mut Engine, this: u32, component: u32, file: u32) {
    if file == 0 || component == 0 {
        return;
    }
    if e.call(FILE_CHUNK_TYPE, &args![file]).u32() != CHUNK_KF_FILE_NAMES {
        return;
    }
    let size = e.mem.u32(at(file, FILE_CHUNK_SIZE));
    let buffer = e.call(OPERATOR_NEW, &args![size]).u32();
    e.call(FILE_CHUNK_DATA, &args![file, buffer, 0u32]);
    let mut name = buffer;
    while e.mem.u8(name) != 0 {
        typed_call!(FAMILY, 0x0047_fd20, add_name(e, this, name));
        let length = e.call(STRING_LENGTH, &args![name]).u32();
        name = name.wrapping_add(length).wrapping_add(1);
    }
    e.call(OPERATOR_DELETE, &args![buffer]);
}

// Typed from 0047fb50
/// `TESAnimation::CopyComponent` (Xbox PDB): when `other` is a `TESAnimation`, this one's names
/// cleared (its vtable slot 4) and each name of `other`'s list added.
pub fn copy_component(e: &mut Engine, this: u32, other: u32) {
    let other = as_animation(e, other);
    if other == 0 {
        return;
    }
    if slot_target(e, this, SLOT_CLEAR_DATA_COMPONENT) == 0x0047_fa50 {
        typed_call!(FAMILY, 0x0047_fa50, clear_data_component(e, this));
    } else {
        e.vcall(this, SLOT_CLEAR_DATA_COMPONENT, &args![]);
    }
    let mut node = at(other, NAMES);
    while node != 0 {
        let name = name_of(e, node);
        if name != 0 {
            typed_call!(FAMILY, 0x0047_fd20, add_name(e, this, name));
        }
        node = next_of(e, node);
    }
}

// Typed from 0047fbd0
/// `TESAnimation::CompareComponent` (Xbox PDB): true when `other` is not a `TESAnimation` or its
/// names differ: the two lists walked together, a name against none differs, two names compare
/// ignoring case, and both lists must end together.
pub fn compare_component(e: &mut Engine, this: u32, other: u32) -> bool {
    let other = as_animation(e, other);
    if other == 0 {
        return true;
    }
    let mut mine = at(this, NAMES);
    let mut theirs = at(other, NAMES);
    while mine != 0 && theirs != 0 {
        let name = name_of(e, mine);
        let other_name = name_of(e, theirs);
        if (name == 0) != (other_name == 0) {
            return true;
        }
        if name != 0
            && other_name != 0
            && e.call(STRING_COMPARE_NO_CASE, &args![name, other_name])
                .u32()
                != 0
        {
            return true;
        }
        mine = next_of(e, mine);
        theirs = next_of(e, theirs);
    }
    mine != 0 || theirs != 0
}

// Typed from 0047fcb0
/// Whether the name list holds `name` (compared ignoring case); false for an empty list.
pub fn has_name(e: &mut Engine, this: u32, name: u32) -> bool {
    let list = at(this, NAMES);
    if is_empty(e, list) {
        return false;
    }
    let mut node = list;
    while node != 0 {
        let item = name_of(e, node);
        if e.call(STRING_COMPARE_NO_CASE, &args![item, name]).u32() == 0 {
            return true;
        }
        node = next_of(e, node);
    }
    false
}

// Typed from 0047fd20
/// A KF file name added unless the list holds it: a copy of the string (its length plus one
/// bytes) appended, the list given the address of the word that holds the copy.
pub fn add_name(e: &mut Engine, this: u32, name: u32) {
    if typed_call!(FAMILY, 0x0047_fcb0, has_name(e, this, name)) {
        return;
    }
    let size = e.call(STRING_LENGTH, &args![name]).u32().wrapping_add(1);
    let copy = e.call(OPERATOR_NEW, &args![size]).u32();
    // The exe keeps the copy in a local and hands the list that local's address.
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), copy);
        e.call(STRING_COPY, &args![copy, size, name]);
        e.call(LIST_APPEND, &args![at(this, NAMES), slot]);
    });
}
