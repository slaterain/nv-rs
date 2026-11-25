//! The task manager's map (`LockFreeMap<int64_t, NiPointer<BSTask<int64_t>>>`, Xbox PDB): the
//! lock-free map with 64-bit keys that `BSTaskManager` keeps its tasks in. Its interface, buckets
//! and marked links are laid out as the 32-bit maps' ([`super::layout::MapInterface`]); a node is
//! 0x10 bytes (the key's two words, the task's holder at +0x08, the next link at +0x0c).
//!
//! - [`search`] (`00c416a0`): the search of one bucket, which the map's find (`00c3fe80`), insert
//!   (`00c3ffb0`), remove (`00c403d0`) and its iterator's step (`00c42560`) all call by address.
//! - [`key_not_below`] (`00c3f6b0`): the map's slot +0x34, the search's compare of each node's key.
//!
//! On the crowded Goodsprings street the search's compare was the second most frequent call by
//! address on main (`NV_CALL_COUNT`, 2026-11-23): about 7,000 a frame, one per node passed. Here
//! the search calls the compare directly when the map's slot +0x34 holds `00c3f6b0` (each of the
//! exe's three vtables for these maps does); any other function there is called through the vtable
//! as the exe calls it.
//!
//! These objects are made by translated code (`00c41610` makes the interfaces), always in guest
//! memory, so they are read there directly, not through the family's store.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{args, sync_direct, typed_call, Engine};

/// The interface (its fields at the 32-bit maps' offsets): the map, the three referenced-node slots
/// (pointers to this thread's words in the map's `pReferencedNodes`: next, current, previous), the
/// link to the current node, the current and the next node word.
const MAP: u32 = 0x00;
const NEXT_SLOT: u32 = 0x04;
const CURRENT_SLOT: u32 = 0x08;
const PREVIOUS_SLOT: u32 = 0x0c;
const LINK: u32 = 0x10;
const CURRENT: u32 = 0x14;
const NEXT: u32 = 0x18;
/// The map's `pBuckets`.
const BUCKETS: u32 = 0x0c;
/// A node's next link; bit 0 of a link marks the node it leads from as deleted.
const NODE_NEXT: u32 = 0x0c;
const MARK: u32 = 1;
/// The map's virtual compares: +0x34 whether a node's key is not below the key looked for, +0x38
/// the answer for a node it is not below, both `thiscall(map, node key low, node key high, key low,
/// key high)`.
const KEY_NOT_BELOW_SLOT: u32 = 0x34;
const FOUND_SLOT: u32 = 0x38;
/// `00c41f40` (`thiscall(interface, node)`): puts a node this thread unlinked on the interface's
/// free list.
const RETIRE: u32 = 0x00c4_1f40;

// Typed from 00c3f6b0
/// `thiscall(map, a low, a high, b low, b high)`: whether the signed 64-bit `a` is not below `b`
/// (the high words compared signed, then the low words unsigned). `this` is not read.
pub fn key_not_below(
    _e: &mut Engine,
    _this: Ptr,
    a_low: u32,
    a_high: u32,
    b_low: u32,
    b_high: u32,
) -> bool {
    let a = ((u64::from(a_high) << 32) | u64::from(a_low)) as i64;
    let b = ((u64::from(b_high) << 32) | u64::from(b_low)) as i64;
    a >= b
}

/// The map's slot +0x34 on a node's key and the key looked for: [`key_not_below`] directly when the
/// slot holds it, else through the vtable.
#[inline(always)]
fn not_below(e: &mut Engine, map: u32, node_low: u32, node_high: u32, low: u32, high: u32) -> bool {
    if crate::slot_target(e, map, KEY_NOT_BELOW_SLOT) == 0x00c3_f6b0 {
        typed_call!(
            FAMILY,
            0x00c3_f6b0,
            key_not_below(e, Ptr::new(map), node_low, node_high, low, high)
        )
    } else {
        e.vcall(
            map,
            KEY_NOT_BELOW_SLOT,
            &args![node_low, node_high, low, high],
        )
        .u8()
            != 0
    }
}

// Typed from 00c416a0
/// `thiscall(interface, bucket, key low, key high)`: the search of bucket `bucket` for the 64-bit
/// key. From the bucket's head it walks the chain, keeping the link it came from, the current node
/// and the next one in the interface and publishing them in its node slots, and starts again from
/// the head whenever a link it read again has changed. A node whose own link is marked deleted is
/// unlinked (a compare-exchange of the link it came from; a failed one starts again) and retired
/// (`00c41f40`). It stops at the first live node whose key is not below the key (slot +0x34),
/// answering slot +0x38's byte for it; at the end of the chain, false. The slots are left as they
/// are: the caller clears them.
///
/// The same reads and writes in the same order as the exe. Its two copies of a node word through a
/// local (the mark cleared again by `0044e010`) are the words themselves here; the exe's tests that
/// would mark those copies are constant and never do.
pub fn search(e: &mut Engine, this: Ptr, bucket: u32, key_low: u32, key_high: u32) -> bool {
    let this = this.addr();
    let field = |e: &Engine, offset: u32| e.mem.u32(this.wrapping_add(offset));
    let map = field(e, MAP);
    let head = e
        .mem
        .u32(map.wrapping_add(BUCKETS))
        .wrapping_add(bucket.wrapping_mul(4));
    'restart: loop {
        e.mem.set_u32(this.wrapping_add(LINK), head);
        let current = e.mem.u32(field(e, LINK));
        e.mem.set_u32(this.wrapping_add(CURRENT), current);
        e.mem.set_u32(field(e, CURRENT_SLOT), current & !MARK);
        let published = field(e, CURRENT) & !MARK;
        if e.mem.u32(field(e, LINK)) != published {
            continue 'restart;
        }
        loop {
            let node = field(e, CURRENT) & !MARK;
            if node == 0 {
                return false;
            }
            let next = e.mem.u32(node.wrapping_add(NODE_NEXT));
            e.mem.set_u32(this.wrapping_add(NEXT), next);
            e.mem.set_u32(field(e, NEXT_SLOT), field(e, NEXT) & !MARK);
            let node = field(e, CURRENT) & !MARK;
            if field(e, NEXT) != e.mem.u32(node.wrapping_add(NODE_NEXT)) {
                continue 'restart;
            }
            let node = field(e, CURRENT) & !MARK;
            let node_key_low = e.mem.u32(node);
            let node_key_high = e.mem.u32(node.wrapping_add(4));
            let expected = field(e, CURRENT) & !MARK;
            if e.mem.u32(field(e, LINK)) != expected {
                continue 'restart;
            }
            if field(e, NEXT) & MARK == 0 {
                let map = field(e, MAP);
                if not_below(e, map, node_key_low, node_key_high, key_low, key_high) {
                    let map = field(e, MAP);
                    return e
                        .vcall(
                            map,
                            FOUND_SLOT,
                            &args![node_key_low, node_key_high, key_low, key_high],
                        )
                        .u8()
                        != 0;
                }
                let link = (field(e, CURRENT) & !MARK).wrapping_add(NODE_NEXT);
                e.mem.set_u32(this.wrapping_add(LINK), link);
                e.mem
                    .set_u32(field(e, PREVIOUS_SLOT), field(e, CURRENT) & !MARK);
            } else {
                let unlinked = field(e, NEXT) & !MARK;
                let link = field(e, LINK);
                let old = sync_direct::interlocked_compare_exchange(
                    e,
                    link,
                    unlinked as i32,
                    expected as i32,
                ) as u32;
                if old != expected {
                    continue 'restart;
                }
                let node = field(e, CURRENT) & !MARK;
                e.call(RETIRE, &args![this, node]);
            }
            let next = field(e, NEXT);
            e.mem.set_u32(this.wrapping_add(CURRENT), next);
            e.mem
                .set_u32(field(e, CURRENT_SLOT), field(e, NEXT) & !MARK);
        }
    }
}
