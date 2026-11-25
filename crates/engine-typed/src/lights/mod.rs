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
//! - The light lists' slow loops, with the same answers in linear work: [`resort`]
//!   (`BSShaderLightingProperty::ResortLights`, `00b70390`: each light's luminance worked out once
//!   instead of again at every comparison), [`weighted`] (`ComputeWeightedShadowLight`, `00b9d150`)
//!   and [`dominant`] (the dominant light direction, `00b5bca0`), whose walks over the scene node's
//!   lights keep their place instead of starting from the list's head for each light (`00b5ac20`'s
//!   walk, called by the first and inlined in the second). These three keep the exact x87 helpers:
//!   the sort also runs on the loading thread (the render passes), the dominant direction is also
//!   asked by the blood splatter's set-up; their Diff tests run both control words.
//! - [`queue`]: the scene node's queue drain (`00b5fd60`, once a frame from `00b60040`) and its
//!   object half, which redoes the light lists of every object that moved or animated:
//!   `UpdateQueuedObjectLightingStart` (`00b5dac0`), the object walks (`00b5c490`, `00b5c5c0`) and
//!   the per-geometry updates (`00b5b260`, `00b5b410`), reaching the typed bound test and add
//!   directly.
//!
//! The objects' `IsNode` and `IsTriBasedGeom` slots and the lights' RTTI slot are answered in place
//! when they hold the exe's constant bodies (`walk::slot_word`), through the vtable otherwise.
//! What stays by address: the geometry and lighting property tests, the property's light add and
//! the list moves and adds.
//!
//! Stage 1: the objects stay in guest memory; each function stays registered at its exe address.
//! **Floats.** The bound tests' distances, on the main thread only ([`Threads::Main`]): native `f32`,
//! each operation and store the exe's in its order (the main thread's 24-bit precision rounds every
//! step to a float, as plain `f32` does), the square root `FSQRT`-like `f32::sqrt` in place of the
//! CRT's `_CIsqrt` (its result is stored to a float at once: one correctly rounded root). The
//! family is declared [`Threads::Main`] for them. The light sort is also reached on the loading
//! thread (the render passes): it and the other slow-loop members keep the exact x87 helpers, right
//! under either control word, and their Diff tests run both.

use crate::image;
use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod dead;
pub mod dominant;
mod list;
pub mod queue;
pub mod resort;
pub mod walk;
pub mod weighted;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(30);

/// `NV_TYPED=lights`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static LIGHTS: Family = Family {
    name: "lights",
    id: FAMILY,
    past_gate: true,
    // The walk's members (plain `f32`) run on the main thread only: the light passes and the
    // shadow preparation. The light sort (`resort`) is also reached from the render passes on the
    // loading thread, so it keeps the exact x87 helpers, which are right under either control
    // word, and its Diff test runs both; the weighted and dominant members keep them too.
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
        entry!(0x00b7_0390, resort::resort_lights(Ptr, u32)),
        entry!(0x00b9_d150, weighted::compute_weighted(Ptr<ShadowSceneLight>, u32, u32, u32, u32)),
        entry!(0x00b5_bca0, dominant::dominant_light_direction(Ptr, Ptr, Ptr, Ptr) -> u32),
        entry!(0x00b5_fd60, queue::drain(Ptr)),
        entry!(0x00b5_dac0, queue::update_start(u32, u32, u32, u32) -> u8),
        entry!(0x00b5_b260, queue::update_geometry(u32, u32, u32, u32, u32) -> u8),
        entry!(0x00b5_b410, queue::update_geometry_list(u32, u32, u32, u32) -> u8),
        entry!(0x00b5_c490, queue::update_tree(u32, u32, u32, u32, u32) -> u8),
        entry!(0x00b5_c5c0, queue::update_tree_list(u32, u32, u32, u32) -> u8),
    ]
}
