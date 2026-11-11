//! A map's nodes (`HashListNode`, Xbox PDB): made with their key and value, and deleted, with or
//! without first telling the map (its slot +0x28) and releasing the value they hold. Through the
//! store ([`super::store`]): a node is a guest block or a moved one.

use super::layout::{HashListNode, Holder, LockFreeMap};
use super::store::{clear_word, free, holder_release, store};
use super::{holder, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// `NiPointer<T>` made from an object (Xbox PDB name of the template; `thiscall(holder, object)`,
/// one body for every `T`), here with null.
const HOLDER_INIT: u32 = 0x0052_8cb0;
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
    let s = store(e);
    let held = HashListNode::value_holder(this).addr();
    clear_word(e, s, HOLDER_INIT, held, &args![held, 0u32]);
    let link = HashListNode::next_link(this).addr();
    clear_word(e, s, LINK_CONSTRUCT, link, &args![link]);
    s.set(e, this, HashListNode::key, key);
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
    let s = store(e);
    let link = HashListNode::next_link(this).addr();
    clear_word(e, s, LINK_CONSTRUCT, link, &args![link]);
    s.set(e, this, HashListNode::key, key);
    let word = e.mem.u32(value.addr());
    s.set(e, this, HashListNode::value, word);
    this
}

// Typed from 0044dd40
/// A counted node's destructor (`thiscall`): its holder's (`0044cbf0`), which releases the value.
pub fn destroy(e: &mut Engine, this: Ptr<HashListNode>) {
    let s = store(e);
    holder_release(e, s, HashListNode::value_holder(this).addr());
}

// Typed from 0044ed50
/// A counted node's deleting destructor (`thiscall(flags)`, answers `this`): [`destroy`], then the
/// node freed when bit 0 of `flags` is set.
pub fn delete(e: &mut Engine, this: Ptr<HashListNode>, flags: u32) -> Ptr<HashListNode> {
    typed_call!(FAMILY, 0x0044_dd40, destroy(e, this));
    if flags & 1 != 0 {
        let s = store(e);
        free(e, s, this.addr());
    }
    this
}

// Typed from 0044dde0
/// The map's deletion of a plain node (`thiscall(map, node)`): the node's key handed to the map's
/// slot +0x28, then the node freed.
pub fn delete_plain(e: &mut Engine, map: Ptr<LockFreeMap>, node: Ptr<HashListNode>) {
    let s = store(e);
    let key = s.get(e, node, HashListNode::key);
    e.vcall(map.addr(), RELEASE_KEY_SLOT, &args![key]);
    free(e, s, node.addr());
}

// Typed from 0044edd0
/// The map's deletion of a counted node (`thiscall(map, node)`): the node's key handed to the
/// map's slot +0x28 (read before the node is tested for null, as the exe does), then the node
/// deleted (`0044ed50`) unless null.
pub fn delete_counted(e: &mut Engine, map: Ptr<LockFreeMap>, node: Ptr<HashListNode>) {
    let s = store(e);
    let key = s.get(e, node, HashListNode::key);
    e.vcall(map.addr(), RELEASE_KEY_SLOT, &args![key]);
    if !node.is_null() {
        let _: Ptr<HashListNode> = typed_call!(FAMILY, 0x0044_ed50, delete(e, node, 1));
    }
}
