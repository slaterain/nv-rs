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
//!   the per-geometry updates (`00b5b260`, `00b5b410`), reaching the typed bound test, add and
//!   luminance directly.
//! - [`scene_light`]: the light's own functions beside the walk: its constructor (`00b9fda0`),
//!   `SetLight` (`00b9dda0`), its luminances (`GetLuminanceToBound` `00b9dbe0`, at a point
//!   `00b9bc50`, at a point facing a direction `00b9bdf0`), its shadow flag (`00b9dcb0`), its
//!   geometry list's fence clear, head and tail removals and `RemoveTriGeom` (`00b9e770`,
//!   `00b9e7c0`, `00b9e850`, `00ba01a0`), an accumulator's `NiPointer` (`00b9cfd0`) and the cull of
//!   its geometries (`00b9d020`). The luminances also run on other threads (the light sort of the
//!   render passes, the AI's light queries): written once over `native::Real`, their Diff tests
//!   run both control words.
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
use engine_core::types::NiTPointerList;
use engine_core::{entry, AbiFn};

pub mod dead;
pub mod dominant;
mod list;
pub mod queue;
pub mod resort;
pub mod scene_light;
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
    /// `ShadowSceneLight` (Xbox PDB names; PC offsets as its constructor `00b9fda0` writes them).
    pub struct ShadowSceneLight / ShadowSceneLightImage: 0x250 {
        /// The `NiRefObject` part: its vtable and count.
        0x000 vtable: u32,
        0x004 m_uiRefCount: u32,
        0x008 unknown_008: u32,
        /// `fLuminance` (Xbox PDB): the last luminance asked of the light.
        0x00c fLuminance: f32,
        /// `kViewProjection`, `kShadowView` and `kShadowProj` (Xbox PDB): three 4x4 float matrices.
        0x010 kViewProjection: [u32; 16],
        0x050 kShadowView: [u32; 16],
        0x090 kShadowProj: [u32; 16],
        0x0d0 fLODDimmer: f32,
        0x0d4 fShadowLODDimmer: f32,
        0x0d8 fShadowLODDimmerTarget: f32,
        0x0dc fShadowLODDimmerElapsed: f32,
        /// `lGeomList` (Xbox PDB): an `NiTPointerList` of the lit geometries (head, tail, count).
        0x0e0 lGeomList: [u32; 3],
        0x0ec bCastShadow: u8,
        0x0ed bCubeShadowMap: u8,
        0x0ee unknown_0ee: [u8; 2],
        0x0f0 spSpecularAttenuationTexture: u32,
        0x0f4 bPointLight: u8,
        0x0f5 bAmbientLight: u8,
        0x0f6 unknown_0f6: [u8; 2],
        /// `spLight` (Xbox PDB): the `NiLight`.
        0x0f8 spLight: u32,
        0x0fc bDynamicLight: u8,
        0x0fd unknown_0fd: [u8; 3],
        /// `bPointPosition` (Xbox PDB): an `NiPoint3`.
        0x100 bPointPosition: [u32; 3],
        0x10c spDynamicShadowMap: u32,
        0x110 usFrustumCull: u16,
        0x112 unknown_112: u16,
        0x114 spFrustumVis: u32,
        0x118 bShadowSpotlight: u8,
        0x119 unknown_119: [u8; 3],
        0x11c fSpotlightFOV: f32,
        0x120 fSpotlightFalloff: f32,
        0x124 bShowDebugTexture: u8,
        0x125 unknown_125: [u8; 3],
        /// `spShadowScene` (Xbox PDB).
        0x128 spShadowScene: u32,
        /// `lRootList` (Xbox PDB): an `NiTPointerList`.
        0x12c lRootList: [u32; 3],
        /// `kGeomListFence` (Xbox PDB): the list item the geometries found again are moved before.
        0x138 kGeomListFence: u32,
        0x13c kGeomListFenceObject: u32,
        0x140 spShadowCamera: u32,
        /// `kShadowCameraPlanes` (Xbox PDB): six planes (normal, distance), then their active mask.
        0x144 planes: [u32; 24],
        0x1a4 planes_active: u32,
        /// `pfClipSpacePlanes` (Xbox PDB): 0x60 bytes of floats.
        0x1a8 pfClipSpacePlanes: [u32; 24],
        0x208 bClipSpacePlanesReady: u8,
        0x209 unknown_209: [u8; 3],
        0x20c spShadowAccumulator: u32,
        /// `kMultiboundRooms`, `kPortals` and `kProcessedNodes` (Xbox PDB): `BSSimpleArray`s (vtable,
        /// buffer, size, reserved size).
        0x210 kMultiboundRooms: [u32; 4],
        0x220 kPortals: [u32; 4],
        0x230 kProcessedNodes: [u32; 4],
        0x240 unknown_240: [u32; 4],
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
        entry!(0x00b9_dda0, scene_light::set_light(Ptr<ShadowSceneLight>, Ptr)),
        entry!(0x00b9_fda0, scene_light::construct(Ptr<ShadowSceneLight>) -> Ptr<ShadowSceneLight>),
        entry!(0x00b9_e7c0, scene_light::pop_head(Ptr<NiTPointerList>, Ptr) -> Ptr),
        entry!(0x00b9_e850, scene_light::pop_tail(Ptr<NiTPointerList>, Ptr) -> Ptr),
        entry!(0x00b9_cfd0, scene_light::set_held_1a0(Ptr, u32)),
        entry!(0x00b9_dbe0, scene_light::luminance_to_bound(Ptr<ShadowSceneLight>, Ptr) -> f32),
        entry!(0x00b9_bc50, scene_light::luminance_at(Ptr<ShadowSceneLight>, u32, u32, u32, u32) -> f32),
        entry!(0x00b9_bdf0, scene_light::facing_luminance(Ptr<ShadowSceneLight>, u32, u32, u32, u32, u32, u32, u32) -> f32),
        entry!(0x00b9_dcb0, scene_light::set_cast_shadow(Ptr<ShadowSceneLight>, u8)),
        entry!(0x00b9_e770, scene_light::clear_fence(Ptr<ShadowSceneLight>)),
        entry!(0x00ba_01a0, scene_light::remove_tri_geom(Ptr<ShadowSceneLight>, u32, u32)),
        entry!(0x00b9_d020, scene_light::cull_geometries(Ptr<ShadowSceneLight>, u32) -> bool),
    ]
}
