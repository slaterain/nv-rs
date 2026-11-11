//! The map itself (`LockFreeMap<K, V>`, Xbox PDB): its iteration, lookups and upkeep. The iterator's
//! flag helpers (`006ebde0`, `0044de20`, `0044de40`, `0044de80`) and the map's interface getter
//! (`00449f80`: `0044d5c0` on `pInterfaceManager`) are done in place here.

use super::layout::{
    BucketHead, HashListNode, Holder, InterfaceManager, LockFreeMap, LockFreeMapIterator,
    MapInterface, MapLock, Marked, NodeSlot,
};
use super::store::{
    alloc, construct_array, free, holder_set, map_get, map_set, map_store, store, tag, MAP_STUB,
};
use super::{
    field_ptr, iface, manager, node, FAMILY, SCOPE_BEGIN, SCOPE_CONTEXT, SCOPE_END, SCOPE_MODE,
    SOURCE_FILE,
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
    let s = store(e);
    let manager = map_get(e, s, this, LockFreeMap::pInterfaceManager);
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

// Typed from 00906570
/// [`iterate_words`]'s copy for `LockFreeMap<MobileObject*, NiPointer<LipTask>>`: the same body,
/// the first node through `00906af0` and the next through `00906c10` (the map's own copies of
/// `0044e550` and `0044e680`: the value is assigned to the holder `value_out`).
pub fn iterate_lip_task_map(
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
                0x0090_6af0,
                iface::first_in_bucket_copy(e, face, bucket, last_key, value_out.cast())
            )
        },
        |e, face, bucket, last_key| {
            typed_call!(
                FAMILY,
                0x0090_6c10,
                iface::next_in_bucket_copy(e, face, bucket, last_key, value_out.cast())
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
    let s = store(e);
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
            let buckets = map_get(e, s, this, LockFreeMap::iNumBuckets);
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

// Typed from 00449530
/// [`find`]'s copy, the virtual lookup of the maps of plain values (their slot +8): the same body,
/// the interface finding the key with `006ec830`.
pub fn find_word(e: &mut Engine, this: Ptr<LockFreeMap>, key: u32, value_out: Ptr<u32>) -> u8 {
    e.call(LOCK_ENTER, &args![lock(this), 0u32]);
    let face = interface(e, this);
    let found: u8 = typed_call!(
        FAMILY,
        0x006e_c830,
        iface::lookup_word(e, face, key, value_out)
    );
    e.call(LOCK_LEAVE, &args![lock(this)]);
    found
}

/// The line `0044c480` names in its scope.
const CLEAR_LINE: u32 = 0x42c;

// Typed from 0044c480
/// Empties the map (`thiscall(keep none)`), in a memory-context scope; a map with no interface
/// manager is left as it is. Under the lock: the interface manager is deleted (its thread count
/// kept), `iCount` cleared, and every bucket emptied, each node unlinked, its value word cleared and
/// deleted (`0044dde0`). Unless `keep_none`, a new interface manager for as many threads takes the
/// old one's place (`006c73b0`). The marked-pointer reads and clears (`0044dec0`, `006665a0`) are
/// done in place.
pub fn clear(e: &mut Engine, this: Ptr<LockFreeMap>, keep_none: u8) {
    let s = store(e);
    e.with_stack(4, |e, scope| {
        e.call(
            SCOPE_BEGIN,
            &args![scope, SCOPE_CONTEXT, SCOPE_MODE, SOURCE_FILE, CLEAR_LINE],
        );
        if map_get(e, s, this, LockFreeMap::pInterfaceManager).is_null() {
            e.call(SCOPE_END, &args![scope]);
            return;
        }
        e.call(LOCK_ENTER, &args![lock(this), 0u32]);
        let manager = map_get(e, s, this, LockFreeMap::pInterfaceManager);
        let threads = s.get(e, manager, InterfaceManager::max_threads);
        let manager = map_get(e, s, this, LockFreeMap::pInterfaceManager);
        if !manager.is_null() {
            let _: Ptr<InterfaceManager> =
                typed_call!(FAMILY, 0x0066_6570, manager::delete(e, manager, 1));
        }
        map_set(e, s, this, LockFreeMap::pInterfaceManager, Ptr::NULL);
        map_set(e, s, this, LockFreeMap::iCount, 0);
        let mut i = 0u32;
        while i < map_get(e, s, this, LockFreeMap::iNumBuckets) {
            let buckets = map_get(e, s, this, LockFreeMap::pBuckets);
            let mut node = s
                .get(e, LockFreeMap::bucket(buckets, i), BucketHead::head)
                .node();
            let buckets = map_get(e, s, this, LockFreeMap::pBuckets);
            s.set(
                e,
                LockFreeMap::bucket(buckets, i),
                BucketHead::head,
                Marked(0),
            );
            while !node.is_null() {
                let next = s.get(e, node, HashListNode::next).node();
                s.set(e, node, HashListNode::next, Marked(0));
                s.set(e, node, HashListNode::value, 0);
                typed_call!(FAMILY, 0x0044_dde0, node::delete_plain(e, this, node));
                node = next;
            }
            i = i.wrapping_add(1);
        }
        if keep_none == 0 {
            let into = map_store(e, this.addr());
            let block: Ptr<InterfaceManager> =
                Ptr::new(alloc(e, into, InterfaceManager::SIZE, tag::MANAGER));
            let made = if block.is_null() {
                Ptr::NULL
            } else {
                typed_call!(FAMILY, 0x006c_73b0, manager::construct(e, block, threads))
            };
            map_set(e, s, this, LockFreeMap::pInterfaceManager, made);
        }
        e.call(LOCK_LEAVE, &args![lock(this)]);
        e.call(SCOPE_END, &args![scope]);
    });
}

/// The line the map constructors name in their scope.
const CONSTRUCT_LINE: u32 = 0x390;
/// The constructor of a bucket head (cleared; `thiscall`).
const BUCKET_CONSTRUCT: u32 = 0x0044_dee0;
/// Referenced-node slots a thread.
const SLOTS_PER_THREAD: u32 = 3;
/// The vtable of `LockFreeMap<TESObjectREFR*, NiPointer<QueuedReference>>` (Xbox PDB).
const QUEUED_REFERENCE_MAP_VTABLE: u32 = 0x0101_747c;

/// The body every map constructor of this shape shares, each with its own vtable: what the
/// interfaced base's constructor (`0044d7f0`) writes at +0 is overwritten at once, and the lock is
/// cleared in place as `0096a2d0` clears it. In a stage-2 form the map's record is attached first
/// (its K1 stub, [`MAP_STUB`]), so every moved field written after it is the record's, and the
/// body is made in the family's store.
#[inline(always)]
fn construct_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    vtable: u32,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    let s = store(e);
    MAP_STUB.attach(e, s, this.addr(), tag::RECORD);
    map_set(e, s, this, LockFreeMap::vtable, vtable);
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
        map_set(e, s, this, LockFreeMap::iCount, 0);
        map_set(e, s, this, LockFreeMap::iNumBuckets, buckets);
        let count = map_get(e, s, this, LockFreeMap::iNumBuckets);
        let bytes = count.saturating_mul(BucketHead::SIZE);
        let block: Ptr<BucketHead> = Ptr::new(alloc(e, s, bytes, tag::BUCKETS));
        if !block.is_null() {
            construct_array(
                e,
                s,
                block.addr(),
                BucketHead::SIZE,
                count,
                BUCKET_CONSTRUCT,
            );
        }
        map_set(e, s, this, LockFreeMap::pBuckets, block);
        // The slot count wraps; only the byte count saturates.
        let slots = max_threads.wrapping_mul(SLOTS_PER_THREAD);
        let bytes = slots.saturating_mul(NodeSlot::SIZE);
        let referenced: Ptr<NodeSlot> = Ptr::new(alloc(e, s, bytes, tag::SLOTS));
        map_set(e, s, this, LockFreeMap::pReferencedNodes, referenced);
        map_set(e, s, this, LockFreeMap::iDeleteBatchSize, batch);
        let block: Ptr<InterfaceManager> =
            Ptr::new(alloc(e, s, InterfaceManager::SIZE, tag::MANAGER));
        let made = if block.is_null() {
            Ptr::NULL
        } else {
            typed_call!(
                FAMILY,
                0x006c_73b0,
                manager::construct(e, block, max_threads)
            )
        };
        map_set(e, s, this, LockFreeMap::pInterfaceManager, made);
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

// The other map types' constructors: the same body (the exe's code differs only in the vtable
// written at +0), each a copy at its own address.

// Typed from 0044ba10
/// The constructor of `LockFreeMap<AnimIdle*, NiPointer<QueuedAnimIdle>>` (RTTI name): [`construct`]'s body with
/// the vtable `010174cc`.
pub fn construct_anim_idle_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0101_74cc, max_threads, buckets, batch)
}

// Typed from 0044bbe0
/// The constructor of `LockFreeMap<Animation*, NiPointer<QueuedReplacementKFList>>` (RTTI name): [`construct`]'s body with
/// the vtable `0101751c`.
pub fn construct_replacement_kf_list_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0101_751c, max_threads, buckets, batch)
}

// Typed from 0044be80
/// The constructor of `LockFreeMap<TESObjectREFR*, NiPointer<QueuedHelmet>>` (RTTI name): [`construct`]'s body with
/// the vtable `0101756c`.
pub fn construct_helmet_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0101_756c, max_threads, buckets, batch)
}

// Typed from 0044c040
/// The constructor of `LockFreeMap<const char*, Model*>` (RTTI name): [`construct`]'s body with
/// the vtable `010175bc`.
pub fn construct_model_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0101_75bc, max_threads, buckets, batch)
}

// Typed from 0044c270
/// The constructor of `LockFreeMap<const char*, KFModel*>` (RTTI name): [`construct`]'s body with
/// the vtable `0101760c`.
pub fn construct_kf_model_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0101_760c, max_threads, buckets, batch)
}

// Typed from 0044c760
/// The constructor of `LockFreeMap<BSFileEntry*, NiPointer<QueuedTexture>>` (RTTI name): [`construct`]'s body with
/// the vtable `0101765c`.
pub fn construct_texture_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0101_765c, max_threads, buckets, batch)
}

// Typed from 0044c920
/// The constructor of `LockFreeMap<const char*, LoadedFile*>` (RTTI name): [`construct`]'s body with
/// the vtable `010176ac`.
pub fn construct_loaded_file_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0101_76ac, max_threads, buckets, batch)
}

// Typed from 005289e0
/// The constructor of `LockFreeMap<unsigned int, NiPointer<ExteriorCellLoaderTask>>` (RTTI name): [`construct`]'s body with
/// the vtable `0102d084`.
pub fn construct_cell_task_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0102_d084, max_threads, buckets, batch)
}

// Typed from 00665cb0
/// The constructor of `LockFreeMap<TESObjectREFR*, BSTreeNode*>` (RTTI name): [`construct`]'s body with
/// the vtable `01066454`.
pub fn construct_tree_node_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0106_6454, max_threads, buckets, batch)
}

// Typed from 006ec660
/// The constructor of `LockFreeMap<PathingRequest*, PathingTaskData*>` (RTTI name): [`construct`]'s body with
/// the vtable `0106d72c`.
pub fn construct_pathing_task_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0106_d72c, max_threads, buckets, batch)
}

// Typed from 00906160
/// The constructor of `LockFreeMap<MobileObject*, NiPointer<LipTask>>` (RTTI name): [`construct`]'s body with
/// the vtable `0108864c`.
pub fn construct_lip_task_map(
    e: &mut Engine,
    this: Ptr<LockFreeMap>,
    max_threads: u32,
    buckets: u32,
    batch: u32,
) -> Ptr<LockFreeMap> {
    construct_map(e, this, 0x0108_864c, max_threads, buckets, batch)
}

/// The vtables of `LockFreeMap<const char*, KFModel*>` and of the interfaced base (Xbox PDB).
const KF_MODEL_MAP_VTABLE: u32 = 0x0101_760c;
const INTERFACED_BASE_VTABLE: u32 = 0x0101_7458;

// Typed from 0044c3f0
/// The destructor of `LockFreeMap<const char*, KFModel*>` (Xbox PDB; `thiscall`): its vtable, the
/// map emptied with no interface manager kept (`0044c480`), the buckets and the referenced-node
/// slots freed, and the interfaced base's vtable (what `0044b4b0` writes).
pub fn destroy(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0044_c480>(e, this, KF_MODEL_MAP_VTABLE);
}

/// [`destroy`]'s body, which every map destructor shares with its own vtable; `CLEAR` empties the
/// map: `0044c480` ([`clear`]) for maps of plain values, `00528e40` ([`clear_counted`]) for maps
/// of counted values. In a stage-2 form the map's record goes last ([`MAP_STUB`]).
#[inline(always)]
fn destroy_map<const CLEAR: u32>(e: &mut Engine, this: Ptr<LockFreeMap>, vtable: u32) {
    let s = store(e);
    map_set(e, s, this, LockFreeMap::vtable, vtable);
    if CLEAR == 0x0044_c480 {
        typed_call!(FAMILY, 0x0044_c480, clear(e, this, 1));
    } else {
        typed_call!(FAMILY, 0x0052_8e40, clear_counted(e, this, 1));
    }
    let buckets = map_get(e, s, this, LockFreeMap::pBuckets);
    free(e, s, buckets.addr());
    let referenced = map_get(e, s, this, LockFreeMap::pReferencedNodes);
    free(e, s, referenced.addr());
    map_set(e, s, this, LockFreeMap::vtable, INTERFACED_BASE_VTABLE);
    MAP_STUB.detach(e, s, this.addr());
}

// The other map types' destructors: the same body (the exe's code differs only in the vtable and,
// for maps of counted values, the call that empties the map), each a copy at its own address.

// Typed from 0044c1e0
/// The destructor of `LockFreeMap<const char*, Model*>` (RTTI name): [`destroy`]'s body with
/// the vtable `010175bc`, emptied by `0044c480`.
pub fn destroy_model_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0044_c480>(e, this, 0x0101_75bc);
}

// Typed from 0044cae0
/// The destructor of `LockFreeMap<const char*, LoadedFile*>` (RTTI name): [`destroy`]'s body with
/// the vtable `010176ac`, emptied by `0044c480`.
pub fn destroy_loaded_file_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0044_c480>(e, this, 0x0101_76ac);
}

// Typed from 006661e0
/// The destructor of `LockFreeMap<TESObjectREFR*, BSTreeNode*>` (RTTI name): [`destroy`]'s body with
/// the vtable `01066454`, emptied by `0044c480`.
pub fn destroy_tree_node_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0044_c480>(e, this, 0x0106_6454);
}

// Typed from 006ec870
/// The destructor of `LockFreeMap<PathingRequest*, PathingTaskData*>` (RTTI name): [`destroy`]'s body with
/// the vtable `0106d72c`, emptied by `0044c480`.
pub fn destroy_pathing_task_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0044_c480>(e, this, 0x0106_d72c);
}

// Typed from 0044d8f0
/// The destructor of `LockFreeMap<TESObjectREFR*, NiPointer<QueuedReference>>` (RTTI name): [`destroy`]'s body with
/// the vtable `0101747c`, emptied by `00528e40`.
pub fn destroy_queued_reference_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0052_8e40>(e, this, 0x0101_747c);
}

// Typed from 0044d9b0
/// The destructor of `LockFreeMap<AnimIdle*, NiPointer<QueuedAnimIdle>>` (RTTI name): [`destroy`]'s body with
/// the vtable `010174cc`, emptied by `00528e40`.
pub fn destroy_anim_idle_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0052_8e40>(e, this, 0x0101_74cc);
}

// Typed from 0044da70
/// The destructor of `LockFreeMap<Animation*, NiPointer<QueuedReplacementKFList>>` (RTTI name): [`destroy`]'s body with
/// the vtable `0101751c`, emptied by `00528e40`.
pub fn destroy_replacement_kf_list_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0052_8e40>(e, this, 0x0101_751c);
}

// Typed from 0044db30
/// The destructor of `LockFreeMap<TESObjectREFR*, NiPointer<QueuedHelmet>>` (RTTI name): [`destroy`]'s body with
/// the vtable `0101756c`, emptied by `00528e40`.
pub fn destroy_helmet_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0052_8e40>(e, this, 0x0101_756c);
}

// Typed from 0044dc50
/// The destructor of `LockFreeMap<BSFileEntry*, NiPointer<QueuedTexture>>` (RTTI name): [`destroy`]'s body with
/// the vtable `0101765c`, emptied by `00528e40`.
pub fn destroy_texture_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0052_8e40>(e, this, 0x0101_765c);
}

// Typed from 00528be0
/// The destructor of `LockFreeMap<unsigned int, NiPointer<ExteriorCellLoaderTask>>` (RTTI name): [`destroy`]'s body with
/// the vtable `0102d084`, emptied by `00528e40`.
pub fn destroy_cell_task_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0052_8e40>(e, this, 0x0102_d084);
}

// Typed from 009064e0
/// The destructor of `LockFreeMap<MobileObject*, NiPointer<LipTask>>` (RTTI name): [`destroy`]'s body with
/// the vtable `0108864c`, emptied by `00528e40`.
pub fn destroy_lip_task_map(e: &mut Engine, this: Ptr<LockFreeMap>) {
    destroy_map::<0x0052_8e40>(e, this, 0x0108_864c);
}

// Typed from 00528e40
/// [`clear`] for the maps of counted values: the same steps, the interface manager deleted with
/// `00529300`, each node's value released (`006f74f0` with null) and the node deleted with
/// `0044edd0` (which first hands its key to the map's slot +0x28).
pub fn clear_counted(e: &mut Engine, this: Ptr<LockFreeMap>, keep_none: u8) {
    let s = store(e);
    e.with_stack(4, |e, scope| {
        e.call(
            SCOPE_BEGIN,
            &args![scope, SCOPE_CONTEXT, SCOPE_MODE, SOURCE_FILE, CLEAR_LINE],
        );
        if map_get(e, s, this, LockFreeMap::pInterfaceManager).is_null() {
            e.call(SCOPE_END, &args![scope]);
            return;
        }
        e.call(LOCK_ENTER, &args![lock(this), 0u32]);
        let manager = map_get(e, s, this, LockFreeMap::pInterfaceManager);
        let threads = s.get(e, manager, InterfaceManager::max_threads);
        let manager = map_get(e, s, this, LockFreeMap::pInterfaceManager);
        if !manager.is_null() {
            let _: Ptr<InterfaceManager> =
                typed_call!(FAMILY, 0x0052_9300, manager::delete_counted(e, manager, 1));
        }
        map_set(e, s, this, LockFreeMap::pInterfaceManager, Ptr::NULL);
        map_set(e, s, this, LockFreeMap::iCount, 0);
        let mut i = 0u32;
        while i < map_get(e, s, this, LockFreeMap::iNumBuckets) {
            let buckets = map_get(e, s, this, LockFreeMap::pBuckets);
            let mut node = s
                .get(e, LockFreeMap::bucket(buckets, i), BucketHead::head)
                .node();
            let buckets = map_get(e, s, this, LockFreeMap::pBuckets);
            s.set(
                e,
                LockFreeMap::bucket(buckets, i),
                BucketHead::head,
                Marked(0),
            );
            while !node.is_null() {
                let next = s.get(e, node, HashListNode::next).node();
                s.set(e, node, HashListNode::next, Marked(0));
                holder_set(e, s, HashListNode::value_holder(node).addr(), 0);
                typed_call!(FAMILY, 0x0044_edd0, node::delete_counted(e, this, node));
                node = next;
            }
            i = i.wrapping_add(1);
        }
        if keep_none == 0 {
            let into = map_store(e, this.addr());
            let block: Ptr<InterfaceManager> =
                Ptr::new(alloc(e, into, InterfaceManager::SIZE, tag::MANAGER));
            let made = if block.is_null() {
                Ptr::NULL
            } else {
                typed_call!(FAMILY, 0x006c_73b0, manager::construct(e, block, threads))
            };
            map_set(e, s, this, LockFreeMap::pInterfaceManager, made);
        }
        e.call(LOCK_LEAVE, &args![lock(this)]);
        e.call(SCOPE_END, &args![scope]);
    });
}
