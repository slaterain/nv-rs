//! The walk over a word-keyed map (`NiTMapBase`: the vtable, the bucket count at `+4`, the table at
//! `+8`; a node is the next node, its key and its value): its first position (`004b9ba0`) and the
//! step that hands out a node's key and value and moves on (`006b7f20`). The save's body walks its
//! changes map with these, one step per changed form, and the clean-up before it walks the map
//! once more; the changes map's hash is the word map's own (`006c6a60`).

use crate::pathing;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The word map's hash, by the function its vtable slot holds.
const WORD_MAP_HASH: u32 = 0x006c_6a60;

// Typed from 004b9ba0
/// The first position of a map (`thiscall`): the first bucket's head that is not empty, else 0.
/// The count and the table are read once, as the exe does.
pub fn first_position(e: &mut Engine, this: Ptr) -> u32 {
    let size = e.mem.u32(this.addr().wrapping_add(4));
    let table = e.mem.u32(this.addr().wrapping_add(8));
    (0..size)
        .map(|index| e.mem.u32(table.wrapping_add(index.wrapping_mul(4))))
        .find(|&head| head != 0)
        .unwrap_or(0)
}

// Typed from 006b7f20
/// The walk's step (`thiscall(map, position, key, value)`, `RET 0xc`): the node at `*position`
/// hands out its key and value; the position moves to the node's next, or at the end of its chain
/// to the head of the first later bucket that is not empty (the node's key read again and hashed
/// through vtable `+4`, the word map's own directly), or to 0 after the last bucket. The count and
/// the table are read again at each bucket, as in the exe.
pub fn next(e: &mut Engine, this: Ptr, position: Ptr, key: Ptr, value: Ptr) {
    let node = e.mem.u32(position.addr());
    let node_key = e.mem.u32(node.wrapping_add(4));
    e.mem.set_u32(key.addr(), node_key);
    let node_value = e.mem.u32(node.wrapping_add(8));
    e.mem.set_u32(value.addr(), node_value);
    let following = e.mem.u32(node);
    if following != 0 {
        e.mem.set_u32(position.addr(), following);
        return;
    }
    let key_again = e.mem.u32(node.wrapping_add(4));
    let t = this.addr();
    let bucket = match crate::slot_target(e, t, 4) {
        WORD_MAP_HASH => typed_call!(
            pathing::FAMILY,
            0x006c_6a60,
            pathing::search::hash(e, this.cast(), key_again)
        ),
        _ => e.vcall(t, 4, &args![key_again]).u32(),
    };
    let mut index = bucket.wrapping_add(1);
    loop {
        if index >= e.mem.u32(t.wrapping_add(4)) {
            e.mem.set_u32(position.addr(), 0);
            return;
        }
        let table = e.mem.u32(t.wrapping_add(8));
        let head = e.mem.u32(table.wrapping_add(index.wrapping_mul(4)));
        if head != 0 {
            e.mem.set_u32(position.addr(), head);
            return;
        }
        index = index.wrapping_add(1);
    }
}
