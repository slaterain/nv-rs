//! The walk itself: an object handed to the process, and what a visible object does (a node culls
//! its children, a geometry hands itself to the process to be drawn).

use super::layout::{
    BSCullingProcess, BSMultiBound, BSMultiBoundNode, BSMultiBoundShape, NiAVObject, NiBound,
    NiCullingProcess, NiGeometry, NiNode,
};
use super::{radius_not_zero, FAMILY, FLAG_APP_CULLED};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The process's `Process`, virtual slot 0x44: `thiscall(process, object)`.
const SLOT_PROCESS: u32 = 0x44;
/// The process's `AppendVirtual`, virtual slot 0x4c: `thiscall(process, geometry)`.
const SLOT_APPEND_VIRTUAL: u32 = 0x4c;
/// A multi-bound shape's `WithinFrustum`, virtual slot 0x9c: `thiscall(shape, planes)`, a bool.
const SLOT_WITHIN_FRUSTUM: u32 = 0x9c;

/// `kCullMode` values the multi-bound node reads: every object visible, the planes only, the
/// shapes asked again whatever they said last.
const MODE_ALL_VISIBLE: u32 = 1;
const MODE_PLANES_ONLY: u32 = 3;
const MODE_ASK_SHAPES: u32 = 4;

/// `BSMultiBoundShape::kCullResult` (Xbox PDB `BSMBCullResult`): not known yet, inside, outside.
const RESULT_UNKNOWN: u32 = 0;
const RESULT_INSIDE: u32 = 1;
const RESULT_OUTSIDE: u32 = 2;

/// The shape's `WithinFrustum` (slot 0x9c) on the process's planes, as its answer's low byte:
/// directly when the slot holds `BSMultiBoundAABB`'s (`00c387f0`, typed), else through the vtable.
#[inline(always)]
fn within_frustum(e: &mut Engine, shape: Ptr<BSMultiBoundShape>, planes: u32) -> u8 {
    if crate::slot_target(e, shape.addr(), SLOT_WITHIN_FRUSTUM) == 0x00c3_87f0 {
        u8::from(typed_call!(
            FAMILY,
            0x00c3_87f0,
            super::process::within_frustum(e, shape.cast(), Ptr::new(planes))
        ))
    } else {
        e.vcall(shape.addr(), SLOT_WITHIN_FRUSTUM, &args![planes])
            .u8()
    }
}

/// Child `index` of a node (null for an empty slot).
#[inline(always)]
fn child(e: &Engine, node: Ptr<NiNode>, index: u32) -> Ptr<NiAVObject> {
    let base = e.get(node, NiNode::m_pBase);
    Ptr::new(e.mem.u32(base.index(index, 4).addr()))
}

/// The children read ahead at most (a node or a task with more reads the first ones ahead).
pub(crate) const READ_AHEAD_MAX: u32 = 256;

/// Whether the read-ahead runs (always, but in a test that compares the walk with and without it).
static READ_AHEAD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// For the tests that compare the walk with and without the read-ahead: turns it on or off.
#[doc(hidden)]
pub fn set_read_ahead_for_tests(on: bool) {
    READ_AHEAD.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Reads ahead what the walk of each child will read first, for every child before the first is
/// walked: its flags word, its world bound's radius and its vtable's `OnVisible` slot. A scene's
/// objects lie scattered over the heap, so the walk waited for one cache miss after another; read
/// together, the misses overlap. Nothing is kept or written: the walk reads the same words again
/// when it gets there, so it sees what it always did. Only memory [`engine_core::Mem::span`] finds
/// plainly mapped is read, so a dangling word (a child the walk never follows, a bound it never
/// reads) is skipped and never stops the run, and a page the touch census counts is left to the walk.
/// With real threads these are the same plain loads of the same words the walk makes a moment later.
#[inline]
pub(crate) fn read_ahead(e: &Engine, children: impl Iterator<Item = u32>) {
    if !READ_AHEAD.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let mut seen = 0u32;
    for child in children {
        let Some(object) = (child != 0).then(|| e.mem.span(child)).flatten() else {
            continue;
        };
        seen ^= object.u32(NiAVObject::m_uFlags.off);
        if let Some(bound) = e.mem.span(object.u32(NiAVObject::m_kWorldBound.off)) {
            seen ^= bound.u32(NiBound::m_fRadius.off);
        }
        if let Some(vtable) = e.mem.span(object.u32(NiAVObject::vtable.off)) {
            seen ^= vtable.u32(super::process::SLOT_ON_VISIBLE);
        }
    }
    // The loads are the point: keep them.
    std::hint::black_box(seen);
}

/// [`read_ahead`] of a node's children (the first [`READ_AHEAD_MAX`] slots in use), their array
/// and count read as [`read_ahead`] reads (never faulting, never counted).
#[inline]
fn read_children_ahead(e: &Engine, node: Ptr<NiNode>) {
    let Some(n) = e.mem.span(node.addr()) else {
        return;
    };
    // `m_usMaxSize` and `m_usSize` share the word at +0xa4 (little-endian: the size above).
    let count = n.u32(NiNode::m_usMaxSize.off) >> 16;
    if let Some(words) = e.mem.span(n.u32(NiNode::m_pBase.off)) {
        read_ahead(e, (0..count.min(READ_AHEAD_MAX)).map(|i| words.u32(4 * i)));
    }
}

// Typed from 00a59e00
/// `NiAVObject::Cull` (Xbox PDB, `thiscall(object, process)`): unless the application culled the
/// object, the process's `Process` (slot 0x44) gets it.
pub fn cull(e: &mut Engine, this: Ptr<NiAVObject>, process: Ptr<NiCullingProcess>) {
    if e.get(this, NiAVObject::m_uFlags) & FLAG_APP_CULLED == 0 {
        // `BSCullingProcess::Process` (`00c4ee90`) directly when the slot holds it.
        if crate::slot_target(e, process.addr(), SLOT_PROCESS) == 0x00c4_ee90 {
            typed_call!(
                FAMILY,
                0x00c4_ee90,
                super::process::bs_process(e, process.cast(), this)
            );
        } else {
            // Another process class: its walk is timed too (`NV_PAR_RENDER=time`).
            let _walk = super::probe::on()
                .then(|| super::probe::walk_begins(e, this.addr(), process.addr()));
            e.vcall(process.addr(), SLOT_PROCESS, &args![this]);
        }
    }
}

// Typed from 00a5dbe0
/// `NiNode::OnVisible` (Xbox PDB, `thiscall(node, process)`): only with a world bound whose radius
/// isn't zero (a NaN radius goes on), each child (the count read once, empty slots skipped) is
/// culled with the process.
pub fn node_on_visible(e: &mut Engine, this: Ptr<NiNode>, process: Ptr<NiCullingProcess>) {
    let object = this.cast::<NiAVObject>();
    let bound = e.get(object, NiAVObject::m_kWorldBound);
    if bound.is_null() || !radius_not_zero(e, bound) {
        return;
    }
    let count = u32::from(e.get(this, NiNode::m_usSize));
    // `NV_PAR_RENDER`: the children on several cores, registered in order (`par`).
    if super::par::wants_split(count) && super::par::split(e, this, process.addr(), count) {
        return;
    }
    read_children_ahead(e, this);
    for index in 0..count {
        let child = child(e, this, index);
        if !child.is_null() {
            typed_call!(FAMILY, 0x00a5_9e00, cull(e, child, process));
        }
    }
}

// Typed from 00a7fd90
/// `NiGeometry::OnVisible` (Xbox PDB, `thiscall(geometry, process)`): the process's `AppendVirtual`
/// (slot 0x4c) gets the geometry.
pub fn geometry_on_visible(e: &mut Engine, this: Ptr<NiGeometry>, process: Ptr<NiCullingProcess>) {
    // `BSCullingProcess::AppendVirtual` (`00c4f1d0`) directly when the slot holds it.
    if crate::slot_target(e, process.addr(), SLOT_APPEND_VIRTUAL) == 0x00c4_f1d0 {
        typed_call!(
            FAMILY,
            0x00c4_f1d0,
            super::process::append_virtual(e, process.cast(), this)
        );
    } else {
        e.vcall(process.addr(), SLOT_APPEND_VIRTUAL, &args![this]);
    }
}

// Typed from 00c46f60
/// `BSMultiBoundNode::OnVisible` (Xbox PDB, `thiscall(node, process)`): nothing without non-null
/// children (`m_usESize`). Unless the process's mode is 1 or 3, a multi-bound's shape decides: its
/// last verdict, or in mode 4 a new one (`WithinFrustum`, slot 0x9c, on the process's planes, a Rust
/// call when the slot holds the box's:
/// inside or outside). A shape with no verdict yet is asked, and keeps the answer. Inside is
/// visible, outside or anything else not. When visible, the process's mode becomes the node's
/// (unless it was 3), each child (the count read at each step) is culled, and the mode is put back.
pub fn multibound_node_on_visible(
    e: &mut Engine,
    this: Ptr<BSMultiBoundNode>,
    process: Ptr<BSCullingProcess>,
) {
    let node = this.at(BSMultiBoundNode::base);
    if e.get(node, NiNode::m_usESize) == 0 {
        return;
    }
    let mode = e.get(process, BSCullingProcess::kCullMode);
    let mut visible = true;
    if mode != MODE_PLANES_ONLY
        && mode != MODE_ALL_VISIBLE
        && !e.get(this, BSMultiBoundNode::spMultiBound).is_null()
    {
        let shape = |e: &Engine| {
            let bound = e.get(this, BSMultiBoundNode::spMultiBound);
            e.get(bound, BSMultiBound::spShape)
        };
        let planes = process.addr().wrapping_add(0x2c);
        let mut result = e.get(shape(e), BSMultiBoundShape::kCullResult);
        if e.get(process, BSCullingProcess::kCullMode) == MODE_ASK_SHAPES {
            let inside = within_frustum(e, shape(e), planes);
            result = if inside != 0 {
                RESULT_INSIDE
            } else {
                RESULT_OUTSIDE
            };
        }
        match result {
            RESULT_UNKNOWN => {
                let inside = within_frustum(e, shape(e), planes);
                visible = inside != 0;
                let verdict = if inside != 0 {
                    RESULT_INSIDE
                } else {
                    RESULT_OUTSIDE
                };
                e.set(shape(e), BSMultiBoundShape::kCullResult, verdict);
            }
            RESULT_INSIDE => visible = true,
            _ => visible = false,
        }
    }
    if !visible {
        return;
    }
    if mode != MODE_PLANES_ONLY {
        let own = e.get(this, BSMultiBoundNode::CullingMode);
        e.set(process, BSCullingProcess::kCullMode, own);
    }
    read_children_ahead(e, node);
    let mut index = 0;
    while index < u32::from(e.get(node, NiNode::m_usSize)) {
        let child = child(e, node, index);
        if !child.is_null() {
            typed_call!(
                FAMILY,
                0x00a5_9e00,
                cull(e, child, process.cast::<NiCullingProcess>())
            );
        }
        index += 1;
    }
    e.set(process, BSCullingProcess::kCullMode, mode);
}

#[cfg(test)]
mod read_ahead_tests {
    use super::*;

    /// Two mapped pages for the test's objects (the read-ahead reads only where a word's page
    /// and the next are mapped, so a lone page at the end of a heap would hide what it reads).
    const BASE: u32 = 0x0800_0000;
    const UNMAPPED: u32 = 0x0900_0040;

    fn engine() -> Engine {
        let mut e = Engine::new();
        e.mem.map(BASE, 0x4000);
        assert!(!e.mem.is_mapped(UNMAPPED));
        e
    }

    /// A child at `at`: its flags, bound and vtable words (the bound's radius and the vtable's
    /// `OnVisible` slot made nonzero where they are mapped).
    fn child(e: &mut Engine, at: u32, bound: u32, vtable: u32) {
        e.mem
            .set_u32(at + NiAVObject::m_uFlags.off, FLAG_APP_CULLED);
        e.mem.set_u32(at + NiAVObject::m_kWorldBound.off, bound);
        e.mem.set_u32(at + NiAVObject::vtable.off, vtable);
        if e.mem.is_mapped(bound) {
            e.mem.set_u32(bound + NiBound::m_fRadius.off, 0x3f80_0000);
        }
        if e.mem.is_mapped(vtable) {
            e.mem
                .set_u32(vtable + super::super::process::SLOT_ON_VISIBLE, 0x00a5_dbe0);
        }
    }

    /// A node at `BASE` with `count` slots in use in the storage at `storage`.
    fn node(e: &mut Engine, storage: u32, count: u16) -> Ptr<NiNode> {
        e.mem.set_u32(BASE + NiNode::m_pBase.off, storage);
        e.mem.set_u16(BASE + NiNode::m_usMaxSize.off, count);
        e.mem.set_u16(BASE + NiNode::m_usSize.off, count);
        Ptr::new(BASE)
    }

    /// No slot in use and the storage at unmapped memory: the read-ahead reads none of it (nor
    /// does the walk).
    #[test]
    fn no_slot_in_use_reads_no_storage() {
        let mut e = engine();
        let n = node(&mut e, UNMAPPED, 0);
        read_children_ahead(&e, n);
    }

    /// Children whose bound, vtable or both are dangling, an empty slot, and two whole children:
    /// the read-ahead stops on nothing and writes nothing anywhere (the test's pages, the shared
    /// empty bound).
    #[test]
    fn dangling_children_stop_nothing_and_nothing_is_written() {
        let mut e = engine();
        let null_bound = super::super::NULL_BOUND.addr() & !0xfff;
        e.mem.map(null_bound, 0x2000);
        let storage = BASE + 0x1000;
        let bounds = BASE + 0x1800;
        let vtables = BASE + 0x2000;
        let children = [
            (BASE + 0x200, bounds, vtables),
            (BASE + 0x300, UNMAPPED, vtables),
            (BASE + 0x400, bounds + 0x10, UNMAPPED),
            (BASE + 0x500, UNMAPPED, UNMAPPED),
            (BASE + 0x600, bounds + 0x20, vtables + 0x100),
        ];
        for (i, &(at, bound, vtable)) in (0u32..).zip(children.iter()) {
            child(&mut e, at, bound, vtable);
            e.mem.set_u32(storage + 4 * i, at);
        }
        const CHILDREN: u16 = 5;
        assert_eq!(children.len(), usize::from(CHILDREN));
        // An empty slot and one pointing at unmapped memory.
        e.mem.set_u32(storage + 4 * u32::from(CHILDREN), 0);
        e.mem
            .set_u32(storage + 4 * (u32::from(CHILDREN) + 1), UNMAPPED);
        let n = node(&mut e, storage, CHILDREN + 2);
        e.mem.watch(BASE, 0x4000, false);
        e.mem.watch(null_bound, 0x2000, false);
        read_children_ahead(&e, n);
        assert!(
            e.mem.take_watch_hits().is_empty(),
            "the read-ahead wrote to memory"
        );
    }
}
