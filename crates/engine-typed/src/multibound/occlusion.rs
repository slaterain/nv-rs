//! The shadow scene node's test of one multibound node, which the visibility pass makes for each
//! object it adds: the node's bound shape against the culling process's frustum, then against each
//! active occlusion plane of the portal graph.

use crate::culling;
use crate::culling::layout::{BSMultiBound, BSMultiBoundNode, BSMultiBoundShape, NiFrustumPlanes};
use crate::shader::layout::ShadowSceneNode;
use crate::slot_target;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The culling process's mode word (`BSCullingProcess::kCullMode`, +0x90): modes 1 and 3 test
/// nothing. The visibility pass hands a 0x90-byte `NiCullingProcess`, so there this word is the next
/// local of the exe's frame, the camera position's x.
const PROCESS_MODE: u32 = 0x90;
/// The process's frustum planes (`NiCullingProcess::m_kPlanes`, +0x2c), what the shape is tested
/// against.
const PROCESS_PLANES: u32 = 0x2c;
/// The shape's `WithinFrustum` slot (+0x9c), `BSMultiBoundAABB`'s typed in the culling family.
const SLOT_WITHIN_FRUSTUM: u32 = 0x9c;
/// An occlusion plane's word +0x40 (1: active) and `BSMultiBoundNode::VisibleAgainstActiveOccluders`
/// (`00c47150`, `thiscall(node, x, y, z, process, plane)`, a byte).
const PLANE_ACTIVE: u32 = 0x40;
const VISIBLE_AGAINST_OCCLUDERS: u32 = 0x00c4_7150;

// Typed from 00b5bbe0
/// The shadow scene node's test of a multibound node (`thiscall(scene, node, x, y, z, process)`,
/// `RET 0x14`). Nothing in modes 1 and 3, when the node's bound shape (`spMultiBound`'s `spShape`)
/// has a cull result already, or when the node's word +0xa8 (`m_usESize`) is 0. Otherwise the shape's
/// `WithinFrustum` (slot +0x9c, the process's planes; a Rust call when the slot holds
/// `BSMultiBoundAABB`'s) sets the result: 1 inside, 2 outside. Inside, the node is tested against
/// each active plane of the portal graph's occluder list (`VisibleAgainstActiveOccluders`, by address,
/// with the camera position and the process) until one answers 0.
pub fn add_object(
    e: &mut Engine,
    this: Ptr<ShadowSceneNode>,
    node: u32,
    x: u32,
    y: u32,
    z: u32,
    process: u32,
) {
    let node = Ptr::<BSMultiBoundNode>::new(node);
    let shape = shape_of(e, node);
    let mode = e.mem.u32(process.wrapping_add(PROCESS_MODE));
    let result = e.get(shape, BSMultiBoundShape::kCullResult);
    if mode == 1 || mode == 3 || result != 0 {
        return;
    }
    // `CMP word [node + 0xa8], DX; JBE`: DX is the result's low word, 0 here.
    if e.mem.u16(node.addr().wrapping_add(0xa8)) == 0 {
        return;
    }
    let planes = process.wrapping_add(PROCESS_PLANES);
    let inside = if slot_target(e, shape.addr(), SLOT_WITHIN_FRUSTUM) == 0x00c3_87f0 {
        typed_call!(
            culling::FAMILY,
            0x00c3_87f0,
            culling::process::within_frustum(
                e,
                shape.cast(),
                Ptr::<NiFrustumPlanes>::new(planes)
            )
        )
    } else {
        e.vcall(shape.addr(), SLOT_WITHIN_FRUSTUM, &args![planes]).u8() != 0
    };
    let shape = shape_of(e, node);
    e.set(
        shape,
        BSMultiBoundShape::kCullResult,
        if inside { 1 } else { 2 },
    );
    if !inside {
        return;
    }
    let graph = e.get(this, ShadowSceneNode::pBSPortalGraph);
    if graph == 0 {
        return;
    }
    // `lOccluderList` (+8): items of (next, previous, plane).
    let mut item = e.mem.u32(graph.wrapping_add(8));
    while item != 0 {
        let plane = e.mem.u32(item.wrapping_add(8));
        item = e.mem.u32(item);
        if plane != 0 && e.mem.u32(plane.wrapping_add(PLANE_ACTIVE)) == 1 {
            let visible = e
                .call(
                    VISIBLE_AGAINST_OCCLUDERS,
                    &args![node, x, y, z, process, plane],
                )
                .u8();
            if visible == 0 {
                return;
            }
        }
    }
}

/// The node's bound shape: `spMultiBound` (+0xac), then its `spShape` (+0xc), read as the exe reads
/// them (no null test).
fn shape_of(e: &mut Engine, node: Ptr<BSMultiBoundNode>) -> Ptr<BSMultiBoundShape> {
    let bound: Ptr<BSMultiBound> = e.get(node, BSMultiBoundNode::spMultiBound);
    e.get(bound, BSMultiBound::spShape)
}
