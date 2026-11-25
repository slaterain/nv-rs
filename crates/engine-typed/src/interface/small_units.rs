//! Two small units of the interface code (the translations are engine-fallout-interface's
//! `run_0071b020.rs` and `run_00726bf0.rs`, which these functions replace at the same addresses): a
//! destructor of an object holding two string lists, and the helpers after the menu manager unit
//! (the word-sized elements' placement loop, a list box's item count, a word copy, an object with a
//! `BSString`, a flags byte and a tile value).
//!
//! **Native.** The placement loop's `006e6da0` (placement new: it answers its second argument, and
//! the loop drops the answer) is no work at all, so the loop does nothing; the count's
//! `005ae380` (the non-null items of a `BSSimpleList`, walked through its node accessors) is done
//! in place, as are `BSString`'s constructor and the string list destructor's forwarder's own
//! siblings. The rest stays by address: the string lists' destructor, a deleting destructor, the
//! free, the string's destructor and the setup after it, and `Tile::Value::SetFloat`
//! (`00a0a270`, an `overridable.tsv` row).

use super::interface_cpp::in_place;
use engine_core::prelude::*;
use engine_core::typed_call;

/// What this file does in place besides the interface unit's lists: plain reads the exe calls (the
/// differential tests leave these out of the translation's calls).
pub const IN_PLACE: &[u32] = &[
    0x006e_6da0, // placement new: its second argument
    0x005a_e380, // a list's count of non-null items
];

/// The string lists' destructor (thiscall).
const STRING_LIST_DESTRUCT: u32 = 0x0071_dec0;
/// A deleting destructor (`007b3fa0`, thiscall with the flags; an empty body, then the free).
const DELETE: u32 = 0x007b_3fa0;
/// The setup after the string (`006ffe10`, thiscall), the allocator's free (cdecl) and `BSString`'s
/// destructor (thiscall).
const SETUP: u32 = 0x006f_fe10;
const FREE: u32 = 0x0040_1030;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
/// `Tile::Value::SetFloat` (thiscall: the float, a flag; an `overridable.tsv` row).
const VALUE_SET_FLOAT: u32 = 0x00a0_a270;
/// The object's tile value (`+0xc`), freed block (`+0x24`) and flags byte (`+0x28`).
const TILE_VALUE: u32 = 0x0c;
const BLOCK: u32 = 0x24;
const FLAGS: u32 = 0x28;

// Typed from 0071b020
/// A forwarder: the string list at `this` destroyed (`0071dec0`).
pub fn fn_0071b020(e: &mut Engine, this: Ptr) {
    e.call(STRING_LIST_DESTRUCT, &args![this]);
}

// Typed from 0071b040
/// A destructor body: the object at `+0` deleted (`007b3fa0` with 1) when there is one, then the
/// string lists at `+0x10` and `+4` destroyed.
pub fn fn_0071b040(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let object = e.mem.u32(at);
    if object != 0 {
        e.call(DELETE, &args![object, 1u32]);
    }
    typed_call!(
        super::FAMILY,
        0x0071_b020,
        fn_0071b020(e, Ptr::new(at.wrapping_add(0x10)))
    );
    typed_call!(
        super::FAMILY,
        0x0071_b020,
        fn_0071b020(e, Ptr::new(at.wrapping_add(4)))
    );
}

// Typed from 00726bf0
/// The placement of `count` word-sized elements from `address` (thiscall, `RET 8`; `this` unread):
/// for each, `006e6da0(4, element)`, a placement new that only answers the element's address, which
/// the loop drops. Nothing is stored and nothing is called.
pub fn fn_00726bf0(_e: &mut Engine, _this: Ptr, _address: u32, _count: u32) {}

// Typed from 00726c40
/// The item count of the list box's embedded list (`005ae380` on `this + 4`, in place): its nodes
/// walked from the first (each node's item its first word, the next node its second) and the
/// non-null items counted.
pub fn fn_00726c40(e: &mut Engine, this: Ptr) -> u32 {
    let mut count = 0u32;
    let mut node = this.addr().wrapping_add(4);
    while node != 0 {
        if e.mem.u32(node) != 0 {
            count = count.wrapping_add(1);
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    count
}

// Typed from 00726c60
/// The word at `source` copied into `this` unless it is 0 (thiscall, `RET 4`).
pub fn fn_00726c60(e: &mut Engine, this: Ptr, source: u32) {
    let value = e.mem.u32(source);
    if value != 0 {
        e.mem.set_u32(this.addr(), value);
    }
}

// Typed from 00726c90
/// A constructor: the `BSString` at `+0` (`004037b0`, in place: the empty string), then the setup
/// (`006ffe10`). `this`.
pub fn fn_00726c90(e: &mut Engine, this: Ptr) -> Ptr {
    in_place::empty_string(e, this.addr());
    e.call(SETUP, &args![this]);
    this
}

// Typed from 00726cf0
/// The matching destructor: the block at `+0x24` freed, then the `BSString` at `+0` destroyed
/// (`004037d0`).
pub fn fn_00726cf0(e: &mut Engine, this: Ptr) {
    let block = e.mem.u32(this.addr().wrapping_add(BLOCK));
    e.call(FREE, &args![block]);
    e.call(STRING_DESTRUCT, &args![this]);
}

// Typed from 00726d50
/// Bit 2 of the flags byte (`+0x28`) set or cleared (`00726d70` with the mask 2).
pub fn fn_00726d50(e: &mut Engine, this: Ptr, set: u8) {
    typed_call!(super::FAMILY, 0x0072_6d70, fn_00726d70(e, this, 2, set));
}

// Typed from 00726d70
/// The mask's bits set in the flags byte (`+0x28`), or cleared when `set` is 0 (thiscall, `RET 8`).
pub fn fn_00726d70(e: &mut Engine, this: Ptr, mask: u32, set: u8) {
    let at = this.addr().wrapping_add(FLAGS);
    let flags = e.mem.u8(at);
    // `OR`/`AND` with the whole mask, then the low byte stored.
    let mask = mask.to_le_bytes()[0];
    e.mem.set_u8(
        at,
        if set != 0 {
            flags | mask
        } else {
            flags & !mask
        },
    );
}

// Typed from 00726db0
/// With a tile value at `+0xc`, that value set to 1.0 or 0.0 as `on` says (`Tile::Value::SetFloat`
/// with 1; by address, an `overridable.tsv` row).
pub fn fn_00726db0(e: &mut Engine, this: Ptr, on: u8) {
    let value = e.mem.u32(this.addr().wrapping_add(TILE_VALUE));
    if value == 0 {
        return;
    }
    let number: f32 = if on != 0 { 1.0 } else { 0.0 };
    e.call(VALUE_SET_FLOAT, &args![value, number, 1u32]);
}

/// The file's functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        engine_core::entry!(0x0071_b020, fn_0071b020(Ptr)),
        engine_core::entry!(0x0071_b040, fn_0071b040(Ptr)),
        engine_core::entry!(0x0072_6bf0, fn_00726bf0(Ptr, u32, u32)),
        engine_core::entry!(0x0072_6c40, fn_00726c40(Ptr) -> u32),
        engine_core::entry!(0x0072_6c60, fn_00726c60(Ptr, u32)),
        engine_core::entry!(0x0072_6c90, fn_00726c90(Ptr) -> Ptr),
        engine_core::entry!(0x0072_6cf0, fn_00726cf0(Ptr)),
        engine_core::entry!(0x0072_6d50, fn_00726d50(Ptr, u8)),
        engine_core::entry!(0x0072_6d70, fn_00726d70(Ptr, u32, u8)),
        engine_core::entry!(0x0072_6db0, fn_00726db0(Ptr, u8)),
    ]
}
