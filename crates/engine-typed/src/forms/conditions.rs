//! A condition list's constructor, destructor and clear (`run_00680890`, `run_00680fe0`), a
//! condition's comparison of two values and its function data's byte swap, each as the exe does
//! it.
//!
//! A condition list (`TESCondition`, Xbox PDB) is a `BSSimpleList<TESConditionItem *>` whose head
//! node is the object itself (an item word, then the next node; empty when both are zero).
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): the list head's constructor (`0096a2d0`, both words zeroed), a node's
//! item slot (`006815c0`, the node itself), the list's empty test (`008256d0`) and the byte swaps
//! (`00401080` and `00407a90`, over the runtime's `_byteswap_ulong` and `_byteswap_ushort`). The
//! clear is this family's typed call from the destructor. What stays a call by address: an item's
//! deleting destructor, the list's first node's removal and its destructor.
//!
//! Floats: the comparison only compares the two `f32`s (`FLD` both, then `FCOMPP`/`FUCOMPP`), so
//! it is done on the `f32`s, an unordered pair (a NaN) taking the branch the exe's status-word
//! tests give it.

use super::FAMILY;
use core::cmp::Ordering::{Equal, Greater, Less};
use engine_core::{args, typed_call, Engine};

/// An item's deleting destructor (`007b3fa0`: flags), the list's first node's removal
/// (`0063f7b0`) and the list's destructor (`0046ffb0`), each with `ECX` = the item or list.
const ITEM_DELETING_DESTRUCTOR: u32 = 0x007b_3fa0;
const LIST_POP_FRONT: u32 = 0x0063_f7b0;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

// Typed from 00680890
/// A condition list's constructor: an empty head. Returns `this`.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, 0);
    e.mem.set_u32(at(this, 4), 0);
    this
}

// Typed from 006808b0
/// A condition list's destructor (the quest stage item's destructor runs it on its list):
/// [`clear`], then the list's destructor.
pub fn destruct(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0068_0fe0, clear(e, this));
    e.call(LIST_DESTRUCT, &args![this]);
}

// Typed from 00680fe0
/// A condition list emptied: while the list is set and not empty, its first item deleted (with
/// flag 1) when there is one and its first node removed.
pub fn clear(e: &mut Engine, this: u32) {
    if this == 0 {
        return;
    }
    while e.mem.u32(at(this, 4)) != 0 || e.mem.u32(this) != 0 {
        let item = e.mem.u32(this);
        if item != 0 {
            e.call(ITEM_DELETING_DESTRUCTOR, &args![item, 1u32]);
        }
        e.call(LIST_POP_FRONT, &args![this]);
    }
}

// Typed from 00681050
/// A condition's comparison (cdecl: the operator, the value, the value compared with): 0 equal, 1
/// not equal, 2 greater, 3 greater or equal, 4 less, 5 less or equal, as the value stands to the
/// other; any other operator holds. An unordered pair fails 0 and holds for 1 to 5.
pub fn compare_values(_e: &mut Engine, operator: u32, value: f32, comparand: f32) -> bool {
    let order = value.partial_cmp(&comparand);
    match operator {
        0 => order == Some(Equal),
        1 => order != Some(Equal),
        2 => !matches!(order, Some(Less | Equal)),
        3 => order != Some(Less),
        4 => !matches!(order, Some(Greater | Equal)),
        5 => order != Some(Greater),
        _ => true,
    }
}

// Typed from 00681110
/// A condition item's function data byte-swapped (`Data.FunctionData`, Xbox PDB): the function
/// number (the half at +0), then its two parameters (the words at +4 and +8).
pub fn swap_function_data(e: &mut Engine, this: u32) {
    let half = e.mem.u16(this);
    e.mem.set_u16(this, half.swap_bytes());
    for offset in [4, 8] {
        let word = e.mem.u32(at(this, offset));
        e.mem.set_u32(at(this, offset), word.swap_bytes());
    }
}
