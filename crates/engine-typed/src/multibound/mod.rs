//! The multibound family (the update half's fourth group, stage 1): the two passes main runs over the
//! loaded world once a frame, after the frame is shown, that keep the multibound culling current. The
//! reset pass takes every object's multibound shape back to "not tested"; the visibility pass tests
//! each again against the camera's frustum (the culling family's `BSMultiBoundAABB::WithinFrustum`,
//! called directly) and the portal graph's occlusion planes. About 170 objects a frame in the crowded
//! Goodsprings street (25 loaded cells, their multibound nodes, land and water), by the profile of
//! 2026-11-10.
//!
//! - [`pass`]: the visibility pass (`0045b070`) and the reset pass (`0045bc80`), with the portal
//!   graph's held and queued objects;
//! - [`occlusion`]: the shadow scene node's test of one multibound node (`00b5bbe0`);
//! - [`getters`]: the small functions they call for each object, cell and water entry.
//!
//! What stays by address: the culling process's set-up, the terrain manager's calls (each once a
//! frame), `BSMultiBoundNode::VisibleAgainstActiveOccluders`, the shadow scene node's candidate
//! (`00b5f170`), the portal graph's lists and the queued objects' calls, and the objects' own slots.
//!
//! Stage 1: the objects stay in guest memory where the game made them; each function stays
//! registered at its exe address. **Integers only** (the near distance moves once through the x87
//! stack, which quiets a signalling NaN under any control word), so the row is [`Threads::Both`]:
//! nothing here depends on the control word.

use crate::image;
use crate::shader::layout::ShadowSceneNode;
use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod getters;
pub mod occlusion;
pub mod pass;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(22);

/// `NV_TYPED=multibound`. Proved by its differential tests (each planted mutant caught) and the
/// byte-identical recorded street run in both presets (on against `NV_TYPED=multibound:off`), so on
/// unless `NV_TYPED` says otherwise.
pub static MULTIBOUND: Family = Family {
    name: "multibound",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

image! {
    /// `TES` (Xbox PDB), as far as the passes read it (0xc4 bytes on PC).
    pub struct TES / TESImage: 0x8c {
        0x00 vtable: u32,
        0x04 unknown_04: u32,
        /// `pGridCellA` (Xbox PDB): the loaded exterior cells.
        0x08 pGridCellA: u32,
        /// `pObjRoot` (Xbox PDB) to `iCurrentQueuedY`: not read here.
        0x0c unknown_0c: [u32; 10],
        /// `pInteriorCell` (Xbox PDB): the loaded interior cell, null outdoors.
        0x34 pInteriorCell: u32,
        /// `pInteriorBuffer` (Xbox PDB) to `bShowLANDborders`: not read here.
        0x38 unknown_38: [u32; 11],
        /// `pWaterSystem` (Xbox PDB).
        0x64 pWaterSystem: u32,
        /// `pSky` (Xbox PDB) to `fCell_delta_y`: not read here.
        0x68 unknown_68: [u32; 8],
        /// `pWorldSpace` (Xbox PDB).
        0x88 pWorldSpace: u32,
    }

    /// `GridCellArray` (Xbox PDB), as far as read (0x28 bytes).
    pub struct GridCellArray / GridCellArrayImage: 0x14 {
        0x00 vtable: u32,
        /// `iCenterX` (Xbox PDB).
        0x04 iCenterX: i32,
        /// `iCenterY` (Xbox PDB).
        0x08 iCenterY: i32,
        /// `iDimension` (Xbox PDB): the side of the square of loaded cells.
        0x0c iDimension: u32,
        /// `pGridCellA` (Xbox PDB): the cell slots, row by row.
        0x10 pGridCellA: u32,
    }
}

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0045_b070, pass::add(Ptr<TES>, u32)),
        entry!(0x0045_bc80, pass::remove(Ptr<TES>, u8, u8)),
        entry!(0x00b5_bbe0, occlusion::add_object(Ptr<ShadowSceneNode>, u32, u32, u32, u32, u32)),
        entry!(0x0066_29f0, getters::multibound_of(Ptr) -> u32),
        entry!(0x0052_8820, getters::clear_cull_result(Ptr)),
        entry!(0x0054_5cb0, getters::cell_3d(Ptr) -> u32),
        entry!(0x0045_6fc0, getters::cell_child(Ptr, u32) -> u32),
        entry!(0x004b_a490, getters::grid_cell(Ptr<GridCellArray>, u32, u32) -> u32),
        entry!(0x0043_d4d0, getters::setting_value(Ptr) -> u32),
        entry!(0x0045_0b80, getters::shadow_scene_node(u32) -> u32),
        entry!(0x0045_4b30, getters::portal_graph(Ptr) -> u32),
        entry!(0x0045_bc00, getters::node_child(Ptr, u32) -> u32),
        entry!(0x004f_d3e0, getters::world_space(Ptr<TES>) -> u32),
        entry!(0x0070_ec90, getters::word_at_64(Ptr) -> u32),
        entry!(0x005a_8080, getters::address_3c(Ptr) -> u32),
        entry!(0x0067_33e0, getters::word_at_48(Ptr) -> u32),
        entry!(0x007b_52d0, getters::next_position(Ptr, Ptr) -> u32),
        entry!(0x0063_17a0, getters::item_address(Ptr, Ptr) -> u32),
        entry!(0x0045_bad0, getters::is_exactly(u32, Ptr) -> bool),
        entry!(0x0045_baf0, getters::rtti_is(Ptr, u32) -> bool),
    ]
}
