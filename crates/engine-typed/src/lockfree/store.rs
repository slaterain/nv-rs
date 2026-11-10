//! Where the family's code reads and writes the maps' body (stage 2 of the migration;
//! `engine_core::typed::moved`): through the store of the operation's root, the same code in every
//! form. Stage 1 (`NV_TYPED=lockfree`, the default) is the guest store; `lockfree:dual` keeps the
//! guest body the truth and mirrors it in the slab; `lockfree:moved` (K1) and `lockfree:freed` (K2)
//! keep the body in the slab.
//!
//! What moves: the body (buckets, node slots, interface manager and entries, interfaces, nodes,
//! the scans' copies) and four fields of the map object ([`MAP_STUB`]). What stays in the 0x40 map
//! block, which untyped code still reads: the vtable, the bucket count (+8), the count (+0x18) and
//! the lock (+0x20); +0x1c, which nothing reads, holds the moved record's handle.

use super::layout::LockFreeMap;
use super::FAMILY;
use engine_core::ptr::{Field, Ptr};
use engine_core::typed::moved::{self, Store, Word, K1};
use engine_core::{args, Engine};

/// The map object's K1 stub: the record is the map's layout in the slab, its moved fields the
/// node slots (+4), the buckets (+0xc), the delete batch size (+0x10) and the interface manager
/// (+0x14).
pub(crate) const MAP_STUB: K1 = K1 {
    size: 0x40,
    spare: 0x1c,
    moved: &[0x04, 0x0c, 0x10, 0x14],
};

/// The tags of the family's slab blocks, for the heap census.
pub(crate) mod tag {
    pub const RECORD: u16 = 1;
    pub const BUCKETS: u16 = 2;
    pub const SLOTS: u16 = 3;
    pub const MANAGER: u16 = 4;
    pub const ENTRIES: u16 = 5;
    pub const INTERFACE: u16 = 6;
    pub const NODE: u16 = 7;
    pub const SCAN_COPY: u16 = 8;
}

/// The family's store: stage 1's unless `NV_TYPED` puts the family in a stage-2 form. Reads and
/// writes through it reach a guest object or a moved one, whichever the address names.
#[inline(always)]
pub(crate) fn store(e: &Engine) -> Store {
    e.typed_store(FAMILY)
}

/// The store a new body object of the body object `root` (an interface or a manager) is made in:
/// the family's when `root` is itself moved (a handle, or mirrored in the dual run); stage 1's
/// otherwise (a queue's manager, which the queues' own code reads).
pub(crate) fn store_for(e: &Engine, root: u32) -> Store {
    let s = store(e);
    let moved = match s {
        Store::Guest => false,
        Store::Moved { .. } => moved::is_handle(root),
        Store::Dual { moved, .. } => moved.dual.mirror_of(root).is_some(),
    };
    if moved {
        s
    } else {
        Store::Guest
    }
}

/// The store a new body object of the map `map` is made in: the family's when the map has its
/// record ([`MAP_STUB`], attached by every map constructor in a stage-2 form), stage 1's
/// otherwise.
pub(crate) fn map_store(e: &Engine, map: u32) -> Store {
    let s = store(e);
    let moved = match s {
        Store::Guest => false,
        Store::Moved { .. } => MAP_STUB.record(e, map).is_some(),
        Store::Dual { moved, .. } => {
            let first = MAP_STUB.moved[0];
            moved.dual.mirror_of(map.wrapping_add(first)).is_some()
        }
    };
    if moved {
        s
    } else {
        Store::Guest
    }
}

/// Field `f` of map `map`: a moved field from its record, the others from the guest block.
#[inline(always)]
pub(crate) fn map_get<T: Word>(
    e: &Engine,
    s: Store,
    map: Ptr<LockFreeMap>,
    f: Field<LockFreeMap, T>,
) -> T {
    T::from_word(s.load(e, MAP_STUB.place(e, s, map.addr(), f.off)))
}

/// Sets field `f` of map `map` ([`map_get`]).
#[inline(always)]
pub(crate) fn map_set<T: Word>(
    e: &Engine,
    s: Store,
    map: Ptr<LockFreeMap>,
    f: Field<LockFreeMap, T>,
    v: T,
) {
    s.store(e, MAP_STUB.place(e, s, map.addr(), f.off), v.to_word())
}

/// A call by address of `addr` that clears the word at `at` of the family's own object (a link's
/// or a holder's constructor with null): by address on a guest word (the dual run then takes the
/// guest word into the mirror), in place on a moved one.
pub(crate) fn clear_word(e: &mut Engine, s: Store, addr: u32, at: u32, words: &[u32]) {
    if matches!(s, Store::Moved { .. }) && moved::is_handle(at) {
        s.store(e, at, 0);
    } else {
        e.call(addr, words);
        s.resync(e, at, 1);
    }
}

/// `operator new(size)` for the family's own object, in the store `s`.
pub(crate) fn alloc(e: &mut Engine, s: Store, size: u32, tag: u16) -> u32 {
    s.alloc(e, size, tag, |e| {
        e.call(super::OPERATOR_NEW, &args![size]).u32()
    })
}

/// `operator delete(p)` for the family's own object (a guest or moved block, whichever `p` is).
pub(crate) fn free(e: &mut Engine, s: Store, p: u32) {
    s.free(e, p, |e, block| {
        e.call(super::OPERATOR_DELETE, &args![block]);
    })
}

/// The vector constructor iterator (`stdcall(block, element size, count, constructor)`) over a
/// block of the family's own, each element a cleared word or two: by address on a guest block
/// (the dual run then takes the block into the mirror), nothing on a moved one (the slab's blocks
/// start zero).
pub(crate) fn construct_array(
    e: &mut Engine,
    s: Store,
    block: u32,
    element: u32,
    count: u32,
    constructor: u32,
) {
    if matches!(s, Store::Moved { .. }) && moved::is_handle(block) {
        return;
    }
    e.call(
        super::VECTOR_CONSTRUCT,
        &args![block, element, count, constructor],
    );
    s.resync(e, block, element.wrapping_mul(count) / 4);
}

/// The link compare-and-swap (`004491c0`, `cdecl(link, new, expected)`): whether the link held
/// `expected` and now holds `new`.
pub(crate) fn swap_link(e: &mut Engine, s: Store, link: u32, new: u32, expected: u32) -> bool {
    s.cas(e, link, expected, new, |e| {
        e.call(SWAP_LINK, &args![link, new, expected]).bool()
    })
}

/// The link compare-and-swap (`cdecl(link, new, expected)`).
const SWAP_LINK: u32 = 0x0044_91c0;
/// `NiPointer<T>::operator=` from an object (`thiscall(holder, object)`), the object's release
/// (`NiRefObject::DecRefCount`, Xbox PDB; `thiscall`) and its count taken (`thiscall`), a holder's
/// release (`thiscall(holder)`) and a counted pointer local made from a holder (`thiscall(local,
/// holder)`).
const HOLDER_SET: u32 = 0x006f_74f0;
const RELEASE: u32 = 0x0044_dd60;
const ADD_REF: u32 = 0x0092_c870;
const HOLDER_RELEASE: u32 = 0x0044_cbf0;
const POINTER_FROM_HOLDER: u32 = 0x0044_e370;

/// Whether `holder` is a moved word, done in place rather than by address.
#[inline(always)]
fn in_place(s: Store, at: u32) -> bool {
    matches!(s, Store::Moved { .. }) && moved::is_handle(at)
}

/// `006f74f0` on a holder of the family's own (a node's value) or a caller's: by address on a
/// guest holder (the dual run then takes the word into the mirror); on a moved one in place, as it
/// does it: when the object differs, the held one released (`0044dd60`), the new one stored and
/// counted (`0092c870`).
pub(crate) fn holder_set(e: &mut Engine, s: Store, holder: u32, object: u32) {
    if in_place(s, holder) {
        let held = s.load(e, holder);
        if held != object {
            if held != 0 {
                e.call(RELEASE, &args![held]);
            }
            s.store(e, holder, object);
            if object != 0 {
                e.call(ADD_REF, &args![object]);
            }
        }
    } else {
        e.call(HOLDER_SET, &args![holder, object]);
        s.resync(e, holder, 1);
    }
}

/// A holder's release (`0044cbf0`): by address on a guest holder; on a moved one in place: the
/// held object, if any, released (`0044dd60`).
pub(crate) fn holder_release(e: &mut Engine, s: Store, holder: u32) {
    if in_place(s, holder) {
        let held = s.load(e, holder);
        if held != 0 {
            e.call(RELEASE, &args![held]);
        }
    } else {
        e.call(HOLDER_RELEASE, &args![holder]);
    }
}

/// A counted pointer local (guest) made from a node's holder (`0044e370`): by address on a guest
/// holder; from a moved one in place: the object copied into the local and, if any, counted
/// (`0092c870`).
pub(crate) fn pointer_from_holder(e: &mut Engine, s: Store, local: u32, holder: u32) {
    if in_place(s, holder) {
        let held = s.load(e, holder);
        e.mem.set_u32(local, held);
        if held != 0 {
            e.call(ADD_REF, &args![held]);
        }
    } else {
        e.call(POINTER_FROM_HOLDER, &args![local, holder]);
    }
}
