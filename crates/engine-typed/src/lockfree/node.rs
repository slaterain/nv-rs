//! A map's nodes (`HashListNode`, Xbox PDB): made with their key and value, and deleted, with or
//! without first telling the map (its slot +0x28) and releasing the value they hold.

use super::layout::{HashListNode, Holder, LockFreeMap};
use super::{holder, FAMILY, OPERATOR_DELETE};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// `NiPointer<T>` made from an object (Xbox PDB name of the template; `thiscall(holder, object)`,
/// one body for every `T`) and its destructor (`thiscall(holder)`: the object released when held).
const HOLDER_INIT: u32 = 0x0052_8cb0;
const HOLDER_RELEASE: u32 = 0x0044_cbf0;
/// The constructor of a link (cleared; `thiscall`), as for a bucket head.
const LINK_CONSTRUCT: u32 = 0x0044_dee0;
/// The map's virtual release of a node's key (`thiscall(map, key)`).
const RELEASE_KEY_SLOT: u32 = 0x28;

// Typed from 0044df40
/// A node of a map of counted values (`thiscall(key, value holder)`, answers `this`): its holder
/// made empty (`00528cb0` with null), its link cleared (`0044dee0`), the key stored, and the value
/// assigned to its holder (`0092c820`).
pub fn construct(
    e: &mut Engine,
    this: Ptr<HashListNode>,
    key: u32,
    value: Ptr<Holder>,
) -> Ptr<HashListNode> {
    e.call(HOLDER_INIT, &args![HashListNode::value_holder(this), 0u32]);
    e.call(LINK_CONSTRUCT, &args![HashListNode::next_link(this)]);
    e.set(this, HashListNode::key, key);
    typed_call!(
        FAMILY,
        0x0092_c820,
        holder::assign(e, HashListNode::value_holder(this), value)
    );
    this
}

// Typed from 006ecbd0
/// A node of a map of plain words (`thiscall(key, value)`, answers `this`): its link cleared
/// (`0044dee0`), the key stored and the word at `value` copied.
pub fn construct_word(
    e: &mut Engine,
    this: Ptr<HashListNode>,
    key: u32,
    value: Ptr<u32>,
) -> Ptr<HashListNode> {
    e.call(LINK_CONSTRUCT, &args![HashListNode::next_link(this)]);
    e.set(this, HashListNode::key, key);
    let word = e.mem.u32(value.addr());
    e.set(this, HashListNode::value, word);
    this
}

// Typed from 0044dd40
/// A counted node's destructor (`thiscall`): its holder's (`0044cbf0`), which releases the value.
pub fn destroy(e: &mut Engine, this: Ptr<HashListNode>) {
    e.call(HOLDER_RELEASE, &args![HashListNode::value_holder(this)]);
}

// Typed from 0044ed50
/// A counted node's deleting destructor (`thiscall(flags)`, answers `this`): [`destroy`], then the
/// node freed when bit 0 of `flags` is set.
pub fn delete(e: &mut Engine, this: Ptr<HashListNode>, flags: u32) -> Ptr<HashListNode> {
    typed_call!(FAMILY, 0x0044_dd40, destroy(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0044dde0
/// The map's deletion of a plain node (`thiscall(map, node)`): the node's key handed to the map's
/// slot +0x28, then the node freed.
pub fn delete_plain(e: &mut Engine, map: Ptr<LockFreeMap>, node: Ptr<HashListNode>) {
    let key = e.get(node, HashListNode::key);
    e.vcall(map.addr(), RELEASE_KEY_SLOT, &args![key]);
    e.call(OPERATOR_DELETE, &args![node]);
}

// Typed from 0044edd0
/// The map's deletion of a counted node (`thiscall(map, node)`): the node's key handed to the
/// map's slot +0x28 (read before the node is tested for null, as the exe does), then the node
/// deleted (`0044ed50`) unless null.
pub fn delete_counted(e: &mut Engine, map: Ptr<LockFreeMap>, node: Ptr<HashListNode>) {
    let key = e.get(node, HashListNode::key);
    e.vcall(map.addr(), RELEASE_KEY_SLOT, &args![key]);
    if !node.is_null() {
        let _: Ptr<HashListNode> = typed_call!(FAMILY, 0x0044_ed50, delete(e, node, 1));
    }
}
