//! The map itself (`LockFreeMap<K, V>`, Xbox PDB): its iteration, lookups and upkeep. The iterator's
//! flag helpers (`006ebde0`, `0044de20`, `0044de40`, `0044de80`) and the map's interface getter
//! (`00449f80`: `0044d5c0` on `pInterfaceManager`) are done in place here.

use super::layout::{
    BucketHead, HashListNode, Holder, InterfaceManager, LockFreeMap, LockFreeMapIterator,
    MapInterface, MapLock, Marked, NodeSlot,
};
use super::{
    field_ptr, iface, manager, FAMILY, OPERATOR_DELETE, OPERATOR_NEW, SCOPE_BEGIN, SCOPE_CONTEXT,
    SCOPE_END, SCOPE_MODE, SOURCE_FILE,
};
use engine_core::ptr::{Field, Layout, Ptr};
use engine_core::{args, typed_call, Engine};

/// The map lock's entry (`thiscall(lock, 0)`; it spins and yields to the other game threads) and
/// exit (`thiscall(lock)`).
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;

/// The iterator's flag bits.
const HAS_LAST_KEY: u8 = 1;
const AT_END: u8 = 2;

/// The map's lock, where the game keeps it.
#[inline(always)]
fn lock(this: Ptr<LockFreeMap>) -> Ptr<MapLock> {
    this.at::<MapLock>(LockFreeMap::lock)
}

// Typed from 00449f80
/// The current thread's interface to the map (`thiscall`): `0044d5c0` on its interface manager. A
/// one-line helper, typed so the walks reach the interface manager by a Rust call.
pub fn interface_of(e: &mut Engine, this: Ptr<LockFreeMap>) -> Ptr<MapInterface> {
    let manager = e.get(this, LockFreeMap::pInterfaceManager);
    typed_call!(
        FAMILY,
        0x0044_d5c0,
        manager::interface_for_thread(e, manager, this)
    )
}

/// The current thread's interface to `this`, through [`interface_of`].
#[inline(always)]
fn interface(e: &mut Engine, this: Ptr<LockFreeMap>) -> Ptr<MapInterface> {
    typed_call!(FAMILY, 0x0044_9f80, interface_of(e, this))
}

/// Sets or clears `bit` of the iterator's flags (`0044de40` bit 0, `0044de80` bit 1).
#[inline(always)]
fn set_flag(e: &mut Engine, it: Ptr<LockFreeMapIterator>, bit: u8, on: bool) {
    let flags = e.get(it, LockFreeMapIterator::cFlags);
    let flags = if on { flags | bit } else { flags & !bit };
    e.set(it, LockFreeMapIterator::cFlags, flags);
}

// Typed from 0044b8d0
/// The iteration step (`thiscall(iterator, key out, value holder out, keep going)`, bool), under the
/// map's lock: nothing at the iterator's end. Otherwise from the iterator's bucket, the first node
/// (`0044e550`, then the "has a last key" flag set) or the node after the last key (`0044e680`);
/// on a hit the last key goes to `*key_out`, on a miss the bucket moves on, and at the bucket count
/// the iterator is at its end; while `keep_going`, the next bucket starts afresh. A hit is then
/// removed from the map (`00528bb0` with the key), and the answer is whether that removal happened.
pub fn iterate(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    it: Ptr<LockFreeMapIterator>,
    key_out: Ptr<u32>,
    value_out: Ptr<Holder>,
    keep_going: u8,
) -> bool {
    e.call(LOCK_ENTER, &args![lock(this), 0u32]);
    let mut found = walk(
        e,
        this,
        it,
        key_out,
        keep_going,
        |e, face, bucket, last_key| {
            typed_call!(
                FAMILY,
                0x0044_e550,
                iface::first_in_bucket(e, face, bucket, last_key, value_out)
            )
        },
        |e, face, bucket, last_key| {
            typed_call!(
                FAMILY,
                0x0044_e680,
                iface::next_in_bucket(e, face, bucket, last_key, value_out)
            )
        },
    );
    if found {
        let key = e.get(key_out, KEY);
        let face = interface(e, this);
        found = typed_call!(FAMILY, 0x0052_8bb0, iface::remove(e, face, key));
    }
    e.call(LOCK_LEAVE, &args![lock(this)]);
    found
}

// Typed from 0044c640
/// [`iterate`] for a map of plain words (`thiscall(iterator, key out, value out, keep going)`,
/// bool): the first node through `0044ea60`, the next through `0044eb80`, and a hit is answered as
/// it is, not removed.
pub fn iterate_words(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    it: Ptr<LockFreeMapIterator>,
    key_out: Ptr<u32>,
    value_out: Ptr<u32>,
    keep_going: u8,
) -> bool {
    e.call(LOCK_ENTER, &args![lock(this), 0u32]);
    let found = walk(
        e,
        this,
        it,
        key_out,
        keep_going,
        |e, face, bucket, last_key| {
            typed_call!(
                FAMILY,
                0x0044_ea60,
                iface::first_word_in_bucket(e, face, bucket, last_key, value_out)
            )
        },
        |e, face, bucket, last_key| {
            typed_call!(
                FAMILY,
                0x0044_eb80,
                iface::next_word_in_bucket(e, face, bucket, last_key, value_out)
            )
        },
    );
    e.call(LOCK_LEAVE, &args![lock(this)]);
    found
}

/// The bucket walk of [`iterate`] and [`iterate_words`], under the lock: `first` finds a bucket's
/// first node, `next` the node after the iterator's last key.
#[inline(always)]
fn walk(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    it: Ptr<LockFreeMapIterator>,
    key_out: Ptr<u32>,
    keep_going: u8,
    first: impl Fn(&mut Engine, Ptr<MapInterface>, u32, Ptr<u32>) -> bool,
    next: impl Fn(&mut Engine, Ptr<MapInterface>, u32, Ptr<u32>) -> bool,
) -> bool {
    let mut found = false;
    if e.get(it, LockFreeMapIterator::cFlags) & AT_END == 0 {
        loop {
            let bucket = e.get(it, LockFreeMapIterator::iBucket);
            let last_key = field_ptr(it, LockFreeMapIterator::LastKey);
            let face = interface(e, this);
            if e.get(it, LockFreeMapIterator::cFlags) & HAS_LAST_KEY == 0 {
                found = first(e, face, bucket, last_key);
                set_flag(e, it, HAS_LAST_KEY, true);
            } else {
                found = next(e, face, bucket, last_key);
            }
            if found {
                let key = e.get(it, LockFreeMapIterator::LastKey);
                e.set(key_out, KEY, key);
                break;
            }
            let bucket = e.get(it, LockFreeMapIterator::iBucket).wrapping_add(1);
            e.set(it, LockFreeMapIterator::iBucket, bucket);
            let buckets = e.get(this, LockFreeMap::iNumBuckets);
            if e.get(it, LockFreeMapIterator::iBucket) >= buckets {
                set_flag(e, it, AT_END, true);
                break;
            }
            if keep_going == 0 {
                break;
            }
            set_flag(e, it, HAS_LAST_KEY, false);
        }
    }
    found
}

/// A key word where the caller keeps it.
const KEY: Field<u32, u32> = Field::new(0);

// Typed from 0044bd60
/// The map's virtual lookup (slot +8; `thiscall(key, value holder out)`, the found byte): under the
/// map's lock, the current thread's interface finds the key (`0044c8e0`).
pub fn find(e: &mut Engine, this: Ptr<LockFreeMap>, key: u32, value_out: Ptr<Holder>) -> u8 {
    e.call(LOCK_ENTER, &args![lock(this), 0u32]);
    let face = interface(e, this);
    let found: u8 = typed_call!(FAMILY, 0x0044_c8e0, iface::find(e, face, key, value_out));
    e.call(LOCK_LEAVE, &args![lock(this)]);
    found
}

/// The line `0044c480` names in its scope.
const CLEAR_LINE: u32 = 0x42c;
/// The interface manager's scalar deleting destructor (`thiscall(manager, flags)`).
const MANAGER_DELETE: u32 = 0x0066_6570;
/// The map's deletion of a node (`thiscall(map, node)`).
const DELETE_NODE: u32 = 0x0044_dde0;

// Typed from 0044c480
/// Empties the map (`thiscall(keep none)`), in a memory-context scope; a map with no interface
/// manager is left as it is. Under the lock: the interface manager is deleted (its thread count
/// kept), `iCount` cleared, and every bucket emptied, each node unlinked, its value word cleared and
/// deleted (`0044dde0`). Unless `keep_none`, a new interface manager for as many threads takes the
/// old one's place (`006c73b0`). The marked-pointer reads and clears (`0044dec0`, `006665a0`) are
/// done in place.
pub fn clear(e: &mut Engine, this: Ptr<LockFreeMap>, keep_none: u8) {
    e.with_stack(4, |e, scope| {
        e.call(
            SCOPE_BEGIN,
            &args![scope, SCOPE_CONTEXT, SCOPE_MODE, SOURCE_FILE, CLEAR_LINE],
        );
        if e.get(this, LockFreeMap::pInterfaceManager).is_null() {
            e.call(SCOPE_END, &args![scope]);
            return;
        }
        e.call(LOCK_ENTER, &args![lock(this), 0u32]);
        let manager = e.get(this, LockFreeMap::pInterfaceManager);
        let threads = e.get(manager, InterfaceManager::max_threads);
        let manager = e.get(this, LockFreeMap::pInterfaceManager);
        if !manager.is_null() {
            e.call(MANAGER_DELETE, &args![manager, 1u32]);
        }
        e.set(this, LockFreeMap::pInterfaceManager, Ptr::NULL);
        e.set(this, LockFreeMap::iCount, 0);
        let mut i = 0u32;
        while i < e.get(this, LockFreeMap::iNumBuckets) {
            let buckets = e.get(this, LockFreeMap::pBuckets);
            let mut node = e
                .get(LockFreeMap::bucket(buckets, i), BucketHead::head)
                .node();
            let buckets = e.get(this, LockFreeMap::pBuckets);
            e.set(LockFreeMap::bucket(buckets, i), BucketHead::head, Marked(0));
            while !node.is_null() {
                let next = e.get(node, HashListNode::next).node();
                e.set(node, HashListNode::next, Marked(0));
                e.set(node, HashListNode::value, 0);
                e.call(DELETE_NODE, &args![this, node]);
                node = next;
            }
            i = i.wrapping_add(1);
        }
        if keep_none == 0 {
            let block: Ptr<InterfaceManager> =
                Ptr::new(e.call(OPERATOR_NEW, &args![InterfaceManager::SIZE]).u32());
            let made = if block.is_null() {
                Ptr::NULL
            } else {
                typed_call!(FAMILY, 0x006c_73b0, manager::construct(e, block, threads))
            };
            e.set(this, LockFreeMap::pInterfaceManager, made);
        }
        e.call(LOCK_LEAVE, &args![lock(this)]);
        e.call(SCOPE_END, &args![scope]);
    });
}

/// The line the map constructors name in their scope.
const CONSTRUCT_LINE: u32 = 0x390;
/// The vector constructor iterator (`stdcall(block, element size, count, constructor)`), and the
/// constructor of a bucket head (cleared; `thiscall`).
const VECTOR_CONSTRUCT: u32 = 0x0040_1050;
const BUCKET_CONSTRUCT: u32 = 0x0044_dee0;
/// Referenced-node slots a thread.
const SLOTS_PER_THREAD: u32 = 3;
/// The vtable of `LockFreeMap<TESObjectREFR*, NiPointer<QueuedReference>>` (Xbox PDB).
const QUEUED_REFERENCE_MAP_VTABLE: u32 = 0x0101_747c;

/// The body every map constructor of this shape shares, each with its own vtable: what the
/// interfaced base's constructor (`0044d7f0`) writes at +0 is overwritten at once, and the lock is
/// cleared in place as `0096a2d0` clears it.
#[inline(always)]
fn construct_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    vtable: u32,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    e.set(this, LockFreeMap::vtable, vtable);
    let lock = lock(this);
    e.set(lock, MapLock::owner, 0);
    e.set(lock, MapLock::count, 0);
    e.with_stack(4, |e, scope| {
        e.call(
            SCOPE_BEGIN,
            &args![
                scope,
                SCOPE_CONTEXT,
                SCOPE_MODE,
                SOURCE_FILE,
                CONSTRUCT_LINE
            ],
        );
        e.set(this, LockFreeMap::iCount, 0);
        e.set(this, LockFreeMap::iNumBuckets, buckets);
        let count = e.get(this, LockFreeMap::iNumBuckets);
        let block: Ptr<BucketHead> = Ptr::new(
            e.call(OPERATOR_NEW, &args![count.saturating_mul(BucketHead::SIZE)])
                .u32(),
        );
        if !block.is_null() {
            e.call(
                VECTOR_CONSTRUCT,
                &args![block, BucketHead::SIZE, count, BUCKET_CONSTRUCT],
            );
        }
        e.set(this, LockFreeMap::pBuckets, block);
        // The slot count wraps; only the byte count saturates.
        let slots = max_threads.wrapping_mul(SLOTS_PER_THREAD);
        let referenced: Ptr<NodeSlot> = Ptr::new(
            e.call(OPERATOR_NEW, &args![slots.saturating_mul(NodeSlot::SIZE)])
                .u32(),
        );
        e.set(this, LockFreeMap::pReferencedNodes, referenced);
        e.set(this, LockFreeMap::iDeleteBatchSize, batch);
        let block: Ptr<InterfaceManager> =
            Ptr::new(e.call(OPERATOR_NEW, &args![InterfaceManager::SIZE]).u32());
        let made = if block.is_null() {
            Ptr::NULL
        } else {
            typed_call!(
                FAMILY,
                0x006c_73b0,
                manager::construct(e, block, max_threads)
            )
        };
        e.set(this, LockFreeMap::pInterfaceManager, made);
        e.call(SCOPE_END, &args![scope]);
    });
    this
}

// Typed from 0044b750
/// The constructor of `LockFreeMap<TESObjectREFR*, NiPointer<QueuedReference>>` (Xbox PDB;
/// `thiscall(max threads, buckets, delete batch size)`, answers `this`), in a memory-context scope:
/// `iCount` 0, the buckets (cleared by the vector constructor iterator, or null when the allocation
/// failed), three referenced-node slots a thread, the batch size, and an interface manager for as
/// many threads (`006c73b0`, or null when its allocation failed). The byte counts are all ones when
/// they overflow.
pub fn construct(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(
        e,
        this,
        QUEUED_REFERENCE_MAP_VTABLE,
        max_threads,
        buckets,
        batch,
    )
}

/// The vtables of `LockFreeMap<const char*, KFModel*>` and of the interfaced base (Xbox PDB).
const KF_MODEL_MAP_VTABLE: u32 = 0x0101_760c;
const INTERFACED_BASE_VTABLE: u32 = 0x0101_7458;

// Typed from 0044c3f0
/// The destructor of `LockFreeMap<const char*, KFModel*>` (Xbox PDB; `thiscall`): its vtable, the
/// map emptied with no interface manager kept (`0044c480`), the buckets and the referenced-node
/// slots freed, and the interfaced base's vtable (what `0044b4b0` writes).
pub fn destroy(e: &mut Engine, this: Ptr<LockFreeMap>) {
    e.set(this, LockFreeMap::vtable, KF_MODEL_MAP_VTABLE);
    typed_call!(FAMILY, 0x0044_c480, clear(e, this, 1));
    let buckets = e.get(this, LockFreeMap::pBuckets);
    e.call(OPERATOR_DELETE, &args![buckets]);
    let referenced = e.get(this, LockFreeMap::pReferencedNodes);
    e.call(OPERATOR_DELETE, &args![referenced]);
    e.set(this, LockFreeMap::vtable, INTERFACED_BASE_VTABLE);
}
