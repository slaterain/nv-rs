//! The lights family (the update half's sixth group, stage 1): the walk each shadow scene light
//! makes over its objects to find the geometry it lights (`ShadowSceneLight::AddObject` and the two
//! parse walks under it, by the light's frustum or its radius), the two bound tests it makes at each
//! node, and the add of a lit geometry to the light's list. The light passes run on the main thread
//! every frame before the draw's cut (the shadow preparation `00871290` in the draw sequence, and
//! the scene node's light processing from `00b60040` just before the render), and on cell loads and
//! memory purges; by the street profile of 2026-11-11, about 0.2 ms a frame of main-thread time.
//!
//! - [`walk`]: `AddObject` (`00ba0110`), `ParseObjectAddFrustum` (`00b9f5b0`),
//!   `ParseObjectAddRadius` (`00b9f6a0`), `AddTriGeom` (`00b9f480`),
//!   `BoundWithinLightFrustum` (`00b9bbb0`) and `BoundWithinLightRadius` (`00b9dae0`).
//!
//! What stays by address: the objects' `IsNode` and `IsTriBasedGeom` slots, the light's RTTI slot,
//! the geometry and lighting property tests, the property's light add and the list moves and adds.
//!
//! Stage 1: the objects stay in guest memory; each function stays registered at its exe address.
//! **Floats.** The bound tests' distances, on the main thread only ([`Threads::Main`]): native `f32`,
//! each operation and store the exe's in its order (the main thread's 24-bit precision rounds every
//! step to a float, as plain `f32` does), the square root `FSQRT`-like `f32::sqrt` in place of the
//! CRT's `_CIsqrt` (its result is stored to a float at once: one correctly rounded root).

use crate::image;
use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod walk;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(30);

/// `NV_TYPED=lights`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static LIGHTS: Family = Family {
    name: "lights",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Main,
    stage2: false,
    funcs,
};

image! {
    /// `ShadowSceneLight` (Xbox PDB), as far as the walk reads it (0x250 bytes on PC).
    pub struct ShadowSceneLight / ShadowSceneLightImage: 0x1a8 {
        0x000 unknown_000: [u32; 56],
        /// `lGeomList` (Xbox PDB): an `NiTPointerList` of the lit geometries (head at +0).
        0x0e0 lGeomList: [u32; 3],
        0x0ec bCastShadow: u8,
        0x0ed unknown_0ed: [u8; 11],
        /// `spLight` (Xbox PDB): the `NiLight`.
        0x0f8 spLight: u32,
        0x0fc unknown_0fc: [u32; 11],
        /// `spShadowScene` (Xbox PDB).
        0x128 spShadowScene: u32,
        0x12c unknown_12c: [u32; 3],
        /// `kGeomListFence` (Xbox PDB): the list item the geometries found again are moved before.
        0x138 kGeomListFence: u32,
        0x13c unknown_13c: [u32; 2],
        /// `kShadowCameraPlanes` (Xbox PDB): six planes (normal, distance), then their active mask.
        0x144 planes: [u32; 24],
        0x1a4 planes_active: u32,
    }
}

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00ba_0110, walk::add_object(Ptr<ShadowSceneLight>, u32)),
        entry!(0x00b9_f5b0, walk::parse_frustum(Ptr<ShadowSceneLight>, u32)),
        entry!(0x00b9_f6a0, walk::parse_radius(Ptr<ShadowSceneLight>, u32)),
        entry!(0x00b9_f480, walk::add_tri_geom(Ptr<ShadowSceneLight>, u32)),
        entry!(0x00b9_bbb0, walk::bound_within_frustum(Ptr<ShadowSceneLight>, Ptr) -> u8),
        entry!(0x00b9_dae0, walk::bound_within_radius(Ptr<ShadowSceneLight>, Ptr, u32, f32) -> u8),
    ]
}
