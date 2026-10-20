//! `BSShader/bsbatchrenderer.cpp` (Xbox PDB source unit), subsystem `BSShader`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

layout! {
    /// `BSBatchRenderer` (Xbox PDB), 0x98 bytes (the size its users
    /// allocate); the offsets used below match the PC constructor.
    pub struct BSBatchRenderer: 0x98 {
        /// `pRenderPassA` (Xbox PDB): the pass groups (0x68 bytes each,
        /// after a count word).
        0x08 pRenderPassA: u32,
        /// `RenderPassMap` (Xbox PDB), an `NiTMap`: its vtable.
        0x0c RenderPassMap_vtable: u32,
        /// `RenderPassMap.m_uiHashSize` (Xbox PDB).
        0x10 RenderPassMap_m_uiHashSize: u32,
        /// `RenderPassMap.m_ppkHashTable` (Xbox PDB).
        0x14 RenderPassMap_m_ppkHashTable: u32,
        /// `RenderPassMap.m_kAllocator.m_uiCount` (Xbox PDB).
        0x18 RenderPassMap_m_uiCount: u32,
        /// `uiRenderPassCount` (Xbox PDB).
        0x1c uiRenderPassCount: u32,
        /// `pInstanceRenderer` (Xbox PDB).
        0x28 pInstanceRenderer: u32,
        /// `kActivePassIndexList` (Xbox PDB), a `BSSimpleList`: its item.
        0x2c kActivePassIndexList_item: u16,
        /// `kActivePassIndexList`'s next link.
        0x30 kActivePassIndexList_next: u32,
        /// `iGroupingAlphas` (Xbox PDB).
        0x34 iGroupingAlphas: u32,
    }
}

/// The base-class constructor (engine map unit `niobject.cpp`; thiscall,
/// no arguments).
const OBJECT_CONSTRUCTOR: u32 = 0x00a5_d3a0;
/// The class's vtable.
const BATCH_RENDERER_VTABLE: u32 = 0x010b_7dc8;
/// The vtable the inlined `NiTMap` constructor sets first.
const MAP_BASE_VTABLE: u32 = 0x0106_cf88;
/// The map vtable it sets last.
const MAP_VTABLE: u32 = 0x0106_cf68;
/// The allocator the inlined map constructor calls for its buckets (cdecl,
/// one word: the size).
const ALLOCATE_BUCKETS: u32 = 0x00aa_1070;
/// `memset` (CRT, cdecl).
const MEMSET: u32 = 0x00ec_61c0;
/// The compiler's `eh vector constructor iterator` (stdcall: array, element
/// size, count, constructor, destructor).
const VECTOR_CONSTRUCTOR_ITERATOR: u32 = 0x00ec_782f;
/// Constructor of one alpha-group list (0x0C bytes).
const ALPHA_LIST_CONSTRUCTOR: u32 = 0x00b9_98e0;
/// Destructor of one alpha-group list.
const ALPHA_LIST_DESTRUCTOR: u32 = 0x00b9_98f0;
/// Constructor of one pass group (0x68 bytes).
const PASS_GROUP_CONSTRUCTOR: u32 = 0x00b9_a530;
/// Destructor of one pass group.
const PASS_GROUP_DESTRUCTOR: u32 = 0x00b9_9bb0;
/// The memory manager object the code passes as `this` to its allocator.
const MEMORY_MANAGER: u32 = 0x011f_6238;
/// The memory manager's allocation method (thiscall, one word: the size).
const MEMORY_ALLOCATE: u32 = 0x00aa_3e40;
/// Constructor of the 4-byte instance renderer (thiscall, no arguments).
const INSTANCE_RENDERER_CONSTRUCTOR: u32 = 0x00bd_9120;
/// The renderer's method `00b99bc0` (this unit, not yet translated;
/// thiscall, one word), called with 0.
const RENDERER_METHOD_B99BC0: u32 = 0x00b9_9bc0;
/// Constructor of a 0x24-byte geometry group (this unit, not yet
/// translated; thiscall, one word).
const GEOMETRY_GROUP_CONSTRUCTOR: u32 = 0x00b9_a560;
/// `NiTPointerListBase<..>::RemoveAll` (engine map; thiscall).
const POINTER_LIST_REMOVE_ALL: u32 = 0x00b7_1300;
/// Word of the thread's TLS block the constructor sets to 0x26 and restores.
const TLS_WORD_2B4: u32 = 0x2b4;

/// The group slots +0x3C..+0x74 (13 `pGeometryGroupsA`, 2 `pAlphaGroupA`,
/// Xbox PDB) and the word each group of the kind-99 renderer is built with.
const ALL_GROUPS: [(u32, u32); 15] = [
    (0x3c, 0x2f7),
    (0x40, 0x2f7),
    (0x44, 0x2f7),
    (0x48, 0x2f7),
    (0x4c, 0x2f7),
    (0x50, 0x2f7),
    (0x54, 6),
    (0x58, 1),
    (0x5c, 4),
    (0x60, 0x2f7),
    (0x64, 0x2f7),
    (0x68, 6),
    (0x6c, 4),
    (0x70, 0x2f7),
    (0x74, 0x2f7),
];

/// Allocates a 0x24-byte geometry group and constructs it with `word`
/// (null when the allocation fails).
fn new_geometry_group(e: &mut Engine, word: u32) -> u32 {
    let block = e.call(MEMORY_ALLOCATE, &[MEMORY_MANAGER, 0x24]).u32();
    if block != 0 {
        e.call(GEOMETRY_GROUP_CONSTRUCTOR, &[block, word]).u32()
    } else {
        0
    }
}

/// The tail the kinds 99 and 101 share: sets bit 0 of the first byte of the
/// two alpha groups (+0x70, +0x74) and empties the two alpha-group lists
/// (+0x78, +0x84).
fn finish_alpha_groups(e: &mut Engine, base: u32) {
    for slot in [0x70, 0x74] {
        let group = e.mem.u32(base + slot);
        let flags = e.mem.u8(group);
        e.mem.set_u8(group, flags | 1);
    }
    e.call(POINTER_LIST_REMOVE_ALL, &[base + 0x78]);
    e.call(POINTER_LIST_REMOVE_ALL, &[base + 0x84]);
}

// Translated from 00b9a5b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `BSBatchRenderer` constructor (unnamed in the engine map; class and
/// fields from the Xbox PDB): runs the `NiObject` base constructor, sets the
/// vtable, builds the render-pass map (37 buckets), clears the active pass
/// list, constructs the two alpha-group lists, and with word +0x2B4 of the
/// thread block set to 0x26 builds the instance renderer and `pass_count`
/// pass groups (0x68 bytes each after a count word; the size saturates),
/// sets the pass count, clears `iGroupingAlphas` and calls `00b99bc0(0)`.
/// Then by `kind`: 99 builds all 15 geometry groups; 100 clears them; 101
/// clears all but +0x60, +0x70 and +0x74, which it builds (0x2F7); 99 and
/// 101 then flag the two alpha groups and empty the alpha-group lists. Any
/// other kind leaves the groups as they are. The TLS word is restored on
/// every path. Returns `this`.
// Translation incomplete: 00b9a5b0 - C++ exception unwinding (SEH frame state) is not translated.
pub fn fn_00b9a5b0(
    e: &mut Engine,
    this: Ptr<BSBatchRenderer>,
    pass_count: u32,
    kind: i32,
) -> Ptr<BSBatchRenderer> {
    let base = this.addr();
    e.call(OBJECT_CONSTRUCTOR, &[base]);
    e.mem.set_u32(base, BATCH_RENDERER_VTABLE);
    // The inlined NiTMap constructor (37 buckets).
    e.set(this, BSBatchRenderer::RenderPassMap_vtable, MAP_BASE_VTABLE);
    e.set(this, BSBatchRenderer::RenderPassMap_m_uiHashSize, 0x25);
    e.set(this, BSBatchRenderer::RenderPassMap_m_uiCount, 0);
    let buckets = e.call(ALLOCATE_BUCKETS, &[0x94]).u32();
    let hash_size = e.get(this, BSBatchRenderer::RenderPassMap_m_uiHashSize);
    e.set(this, BSBatchRenderer::RenderPassMap_m_ppkHashTable, buckets);
    e.call(MEMSET, &[buckets, 0, hash_size.wrapping_mul(4)]);
    e.set(this, BSBatchRenderer::RenderPassMap_vtable, MAP_VTABLE);
    e.set(this, BSBatchRenderer::kActivePassIndexList_item, 0);
    e.set(this, BSBatchRenderer::kActivePassIndexList_next, 0);
    e.call(
        VECTOR_CONSTRUCTOR_ITERATOR,
        &[
            base + 0x78,
            0xc,
            2,
            ALPHA_LIST_CONSTRUCTOR,
            ALPHA_LIST_DESTRUCTOR,
        ],
    );
    let tls = e.tls();
    let saved = e.mem.u32(tls + TLS_WORD_2B4);
    e.mem.set_u32(tls + TLS_WORD_2B4, 0x26);
    let block = e.call(MEMORY_ALLOCATE, &[MEMORY_MANAGER, 4]).u32();
    let instance = if block != 0 {
        e.call(INSTANCE_RENDERER_CONSTRUCTOR, &[block]).u32()
    } else {
        0
    };
    e.set(this, BSBatchRenderer::pInstanceRenderer, instance);
    let groups_size = pass_count.checked_mul(0x68).unwrap_or(u32::MAX);
    let size = groups_size.checked_add(4).unwrap_or(u32::MAX);
    let block = e.call(MEMORY_ALLOCATE, &[MEMORY_MANAGER, size]).u32();
    let groups = if block != 0 {
        e.mem.set_u32(block, pass_count);
        e.call(
            VECTOR_CONSTRUCTOR_ITERATOR,
            &[
                block + 4,
                0x68,
                pass_count,
                PASS_GROUP_CONSTRUCTOR,
                PASS_GROUP_DESTRUCTOR,
            ],
        );
        block + 4
    } else {
        0
    };
    e.set(this, BSBatchRenderer::pRenderPassA, groups);
    e.set(this, BSBatchRenderer::uiRenderPassCount, pass_count);
    e.set(this, BSBatchRenderer::iGroupingAlphas, 0);
    e.call(RENDERER_METHOD_B99BC0, &[base, 0]);
    match kind {
        99 => {
            for (slot, word) in ALL_GROUPS {
                let group = new_geometry_group(e, word);
                e.mem.set_u32(base + slot, group);
            }
            finish_alpha_groups(e, base);
        }
        100 => {
            for (slot, _) in ALL_GROUPS {
                e.mem.set_u32(base + slot, 0);
            }
        }
        101 => {
            for (slot, _) in &ALL_GROUPS[..9] {
                e.mem.set_u32(base + slot, 0);
            }
            let group = new_geometry_group(e, 0x2f7);
            e.mem.set_u32(base + 0x60, group);
            for slot in [0x64, 0x68, 0x6c] {
                e.mem.set_u32(base + slot, 0);
            }
            let group = new_geometry_group(e, 0x2f7);
            e.mem.set_u32(base + 0x70, group);
            let group = new_geometry_group(e, 0x2f7);
            e.mem.set_u32(base + 0x74, group);
            finish_alpha_groups(e, base);
        }
        _ => {}
    }
    e.mem.set_u32(tls + TLS_WORD_2B4, saved);
    this
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(
        0x00b9_a5b0,
        fn_00b9a5b0(Ptr<BSBatchRenderer>, u32, i32) -> Ptr<BSBatchRenderer>
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calls_to(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// An engine with doubles for every callee: allocations succeed except
    /// for the size `fail`, constructors return their `this`; the TLS word
    /// starts at 7 and each double checks it is 0x26 while it runs.
    fn renderer_engine(fail: u32) -> Engine {
        let mut e = Engine::new();
        fn in_context(e: &mut Engine) {
            let tls = e.tls();
            assert_eq!(e.mem.u32(tls + TLS_WORD_2B4), 0x26);
        }
        e.register(OBJECT_CONSTRUCTOR, |_, a| a[0].into_ret());
        e.register(ALLOCATE_BUCKETS, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(MEMSET, |e, a| {
            e.mem.write(a[0], &vec![a[1] as u8; a[2] as usize]);
            a[0].into_ret()
        });
        e.register(VECTOR_CONSTRUCTOR_ITERATOR, |_, a| a[0].into_ret());
        e.register_double(MEMORY_ALLOCATE, move |e, a| {
            in_context(e);
            if a[1] == fail {
                0u32.into_ret()
            } else {
                e.mem.alloc(a[1].min(0x1000)).into_ret()
            }
        });
        e.register(INSTANCE_RENDERER_CONSTRUCTOR, |e, a| {
            in_context(e);
            a[0].into_ret()
        });
        e.register(RENDERER_METHOD_B99BC0, |e, _| {
            in_context(e);
            0u32.into_ret()
        });
        e.register(GEOMETRY_GROUP_CONSTRUCTOR, |e, a| {
            in_context(e);
            e.mem.set_u8(a[0], 0x10);
            a[0].into_ret()
        });
        e.register(POINTER_LIST_REMOVE_ALL, |_, _| 0u32.into_ret());
        let tls = e.tls();
        e.mem.set_u32(tls + TLS_WORD_2B4, 7);
        e
    }

    fn build(e: &mut Engine, pass_count: u32, kind: i32) -> u32 {
        let this = e.mem.alloc(0x98);
        for off in (4..0x98).step_by(4) {
            e.mem.set_u32(this + off, 0xdead_beef);
        }
        e.call_log = Some(vec![]);
        let result = e.call(0x00b9_a5b0, &[this, pass_count, kind as u32]).u32();
        assert_eq!(result, this);
        let tls = e.tls();
        assert_eq!(e.mem.u32(tls + TLS_WORD_2B4), 7);
        this
    }

    #[test]
    fn kind_99_builds_every_group_and_the_pass_groups() {
        let mut e = renderer_engine(u32::MAX);
        let this = build(&mut e, 3, 99);
        assert_eq!(e.mem.u32(this), BATCH_RENDERER_VTABLE);
        assert_eq!(calls_to(&e, OBJECT_CONSTRUCTOR), vec![vec![this]]);
        assert_eq!(e.mem.u32(this + 0xc), MAP_VTABLE);
        assert_eq!(e.mem.u32(this + 0x10), 0x25);
        assert_eq!(e.mem.u32(this + 0x18), 0);
        let buckets = e.mem.u32(this + 0x14);
        assert_eq!(calls_to(&e, MEMSET), vec![vec![buckets, 0, 0x94]]);
        assert_eq!(e.mem.u16(this + 0x2c), 0);
        assert_eq!(e.mem.u32(this + 0x30), 0);
        assert_eq!(e.mem.u32(this + 0x34), 0);
        assert_eq!(e.mem.u32(this + 0x1c), 3);
        let groups = e.mem.u32(this + 8);
        assert_eq!(e.mem.u32(groups - 4), 3);
        assert_eq!(
            calls_to(&e, VECTOR_CONSTRUCTOR_ITERATOR),
            vec![
                vec![
                    this + 0x78,
                    0xc,
                    2,
                    ALPHA_LIST_CONSTRUCTOR,
                    ALPHA_LIST_DESTRUCTOR
                ],
                vec![
                    groups,
                    0x68,
                    3,
                    PASS_GROUP_CONSTRUCTOR,
                    PASS_GROUP_DESTRUCTOR
                ],
            ]
        );
        assert_ne!(e.mem.u32(this + 0x28), 0);
        assert_eq!(calls_to(&e, RENDERER_METHOD_B99BC0), vec![vec![this, 0]]);
        let sizes: Vec<u32> = calls_to(&e, MEMORY_ALLOCATE).iter().map(|a| a[1]).collect();
        let mut expected = vec![4, 3 * 0x68 + 4];
        expected.extend([0x24; 15]);
        assert_eq!(sizes, expected);
        let words: Vec<u32> = calls_to(&e, GEOMETRY_GROUP_CONSTRUCTOR)
            .iter()
            .map(|a| a[1])
            .collect();
        assert_eq!(
            words,
            ALL_GROUPS.iter().map(|(_, word)| *word).collect::<Vec<_>>()
        );
        for (slot, _) in ALL_GROUPS {
            assert_ne!(e.mem.u32(this + slot), 0);
        }
        // Only the two alpha groups get bit 0.
        assert_eq!(e.mem.u8(e.mem.u32(this + 0x6c)), 0x10);
        assert_eq!(e.mem.u8(e.mem.u32(this + 0x70)), 0x11);
        assert_eq!(e.mem.u8(e.mem.u32(this + 0x74)), 0x11);
        assert_eq!(
            calls_to(&e, POINTER_LIST_REMOVE_ALL),
            vec![vec![this + 0x78], vec![this + 0x84]]
        );
    }

    #[test]
    fn kind_100_clears_the_groups_and_kind_101_builds_three() {
        let mut e = renderer_engine(u32::MAX);
        let this = build(&mut e, 1, 100);
        for (slot, _) in ALL_GROUPS {
            assert_eq!(e.mem.u32(this + slot), 0);
        }
        assert!(calls_to(&e, GEOMETRY_GROUP_CONSTRUCTOR).is_empty());
        assert!(calls_to(&e, POINTER_LIST_REMOVE_ALL).is_empty());

        let mut e = renderer_engine(u32::MAX);
        let this = build(&mut e, 1, 101);
        for (slot, _) in ALL_GROUPS {
            let built = matches!(slot, 0x60 | 0x70 | 0x74);
            assert_eq!(e.mem.u32(this + slot) != 0, built, "+{slot:#x}");
        }
        assert_eq!(
            calls_to(&e, GEOMETRY_GROUP_CONSTRUCTOR)
                .iter()
                .map(|a| a[1])
                .collect::<Vec<_>>(),
            vec![0x2f7; 3]
        );
        assert_eq!(e.mem.u8(e.mem.u32(this + 0x70)), 0x11);
        assert_eq!(calls_to(&e, POINTER_LIST_REMOVE_ALL).len(), 2);
    }

    #[test]
    fn other_kinds_leave_the_groups_and_failed_allocations_stay_null() {
        // 0x24-byte allocations never happen; the pass-group block fails.
        let mut e = renderer_engine(0x68 * 2 + 4);
        let this = build(&mut e, 2, 5);
        assert_eq!(e.mem.u32(this + 8), 0);
        assert_eq!(e.mem.u32(this + 0x1c), 2);
        assert_eq!(calls_to(&e, VECTOR_CONSTRUCTOR_ITERATOR).len(), 1);
        for (slot, _) in ALL_GROUPS {
            assert_eq!(e.mem.u32(this + slot), 0xdead_beef);
        }
        // A failed instance renderer stays null; a huge pass count saturates.
        let mut e = renderer_engine(4);
        let this = build(&mut e, 0x0400_0000, 100);
        assert_eq!(e.mem.u32(this + 0x28), 0);
        assert!(calls_to(&e, INSTANCE_RENDERER_CONSTRUCTOR).is_empty());
        let sizes: Vec<u32> = calls_to(&e, MEMORY_ALLOCATE).iter().map(|a| a[1]).collect();
        assert_eq!(sizes, vec![4, u32::MAX]);
        // Kind 99 with failing group allocations stores nulls.
        let mut e = renderer_engine(0x24);
        let this = e.mem.alloc(0x98);
        e.call_log = Some(vec![]);
        // Flagging the null alpha groups writes at address 0: map it.
        e.map(0, 0x1000);
        e.call(0x00b9_a5b0, &[this, 1, 99]);
        for (slot, _) in ALL_GROUPS {
            assert_eq!(e.mem.u32(this + slot), 0);
        }
        assert!(calls_to(&e, GEOMETRY_GROUP_CONSTRUCTOR).is_empty());
        assert_eq!(e.mem.u8(0), 1);
    }
}
