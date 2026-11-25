//! The scene node family: the shadow scene node's (`ShadowSceneNode`, Xbox PDB) own functions that
//! no other family holds, as native code. The node keeps the world's lights: its lists of lights
//! and shadow lights, the queues of lights to add, update and remove that the loading and main
//! threads fill and the frame's queue drain (the lights family's `00b5fd60`) empties, and the
//! arrays its lights and portals share.
//!
//! - [`lists`]: the lights looked up by `NiLight` (`GetLight` `00b5b4a0`) and counted (`00b5ac60`),
//!   the shadow flag by `NiLight` (`00b5c450`), the lights added (`AddLight` `00b5c940`, a node's
//!   lights `AddAllLightsFromNode_Recurse` `00b5ca70`, `FindAndCullLight` `00b5f150` and its
//!   recursion `00b5f0c0`), made dynamic (`00b5cd00`) and followed as they move (`00b5d930`); the
//!   update queue (`00b5d270`, `00b5f080`, `00b5ddb0`) and the removal queue (`00b5cff0`, `00b5edf0`,
//!   `RemoveLight` `00b5eed0`, `00b5d180`); every shadow map dropped (`00b5b1f0`); an object's
//!   geometries taken off their lights (`RemoveGeometryFromPropertyLights` `00b5b0e0`,
//!   `RemoveObjectRecurse` `00b5b150`, `RemoveObject` `00b5b1c0`); the portal graph's add of a
//!   missing room (`00b5c6e0`); the light's point, shadow map and shadow scene setters (`00b5a9d0`,
//!   `00b5aee0`, `00b5af30`), the luminance sort comparison (`00b5aa70`), a node's indexed child
//!   (`00b5aa40`), `GetRTTI` (`00b5e590`); the list move after an item (`00b5a8d0`); the arrays'
//!   slot reservation (`00b5c360`), search (`00b5a930`), removals (`00b5ade0`, `00b5aea0`) and clear
//!   (`00b5af80`); and the counted list's `RemoveAll` (`00e74d40`), which every `NiPointer` list of
//!   the exe shares, and the arrays' growth (`00cb9ce0`).
//! - [`node`]: the sun light (`SetSunLight` `00b5aac0`), the occlusion planes' visibility
//!   (`00b5ac90`), a light listed (`00b5eca0`), the queued lights removed (`RemoveQueuedLight`
//!   `00b5d040` and `00b5ef90`), the light value at a point (`00b5b4d0`), the dynamic shadow maps
//!   (`UpdateDynamicShadows` `00b5b880`), the destructor (`00b5e5a0`) and its deleting form
//!   (`00b5f980`), and the jump to `RemoveAll` (`00b5c7c0`).
//!
//! The node's functions the other families already hold stay there: the queue drain and the object
//! lighting updates (lights), the shadow light walk and sort (set-up), the object queue (cell
//! references), the dominant light (lights), the multibound test (multibound), the light at an index
//! (shader).
//!
//! **Which threads.** The main thread (the frame's light passes, the cell's attach, the moved
//! lights) and the loading thread (a cell's lights added and removed under the queues' lock), so
//! [`Threads::Both`]. **Floats.** Only the light value at a point (`00b5b4d0`, which the actors'
//! light value reaches on the AI threads too) works one out: over `native::Real`, at the running
//! thread's precision. The rest only compare loaded floats or store a float as the exe's
//! `FLD`/`FSTP` stores it (see [`lists`]); the Diff tests run both control words.
//!
//! Stage 1: the objects stay in guest memory; each function stays registered at its exe address.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::types::NiTPointerList;
use engine_core::{entry, AbiFn};

pub mod lists;
pub mod node;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(47);

/// `NV_TYPED=scene_node`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static SCENE_NODE: Family = Family {
    name: "scene_node",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00b5_c360, lists::reserve_slot(Ptr) -> u32),
        entry!(0x00b5_a930, lists::find(Ptr, u32, u32, u32) -> u32),
        entry!(0x00b5_ade0, lists::remove_at(Ptr, u32, u8)),
        entry!(0x00b5_aea0, lists::remove_run(Ptr, u32, u32)),
        entry!(0x00b5_af80, lists::clear(Ptr, u8)),
        entry!(0x00b5_a8d0, lists::move_after(Ptr<NiTPointerList>, u32, u32)),
        entry!(0x00b5_a9d0, lists::set_point_position(Ptr, u32, u32, u32)),
        entry!(0x00b5_aee0, lists::set_shadow_map(Ptr, u32)),
        entry!(0x00b5_af30, lists::set_shadow_scene(Ptr, u32)),
        entry!(0x00b5_aa70, lists::compare_luminance(Ptr, Ptr) -> i32),
        entry!(0x00b5_aa40, lists::indexed_child(Ptr) -> u32),
        entry!(0x00b5_e590, lists::get_rtti() -> Ptr),
        entry!(0x00b5_ac60, lists::count_frustum_lights(Ptr) -> u32),
        entry!(0x00b5_b4a0, lists::get_light(Ptr, u32) -> u32),
        entry!(0x00b5_c450, lists::set_light_cast_shadow(Ptr, u32, u32)),
        entry!(0x00b5_d270, lists::queue_update(Ptr, u32)),
        entry!(0x00b5_f080, lists::queue_update_of(Ptr, u32)),
        entry!(0x00b5_cff0, lists::queue_removal(Ptr, Ptr)),
        entry!(0x00b5_edf0, lists::queue_removal_of_scene(Ptr, u32)),
        entry!(0x00b5_eed0, lists::remove_light(Ptr, u32)),
        entry!(0x00b5_d180, lists::queue_all_removals(Ptr)),
        entry!(0x00b5_ddb0, lists::queue_all_updates(Ptr, u8, u8)),
        entry!(0x00b5_c940, lists::add_light(Ptr, u32, u8) -> u32),
        entry!(0x00b5_ca70, lists::add_all_lights_from_node(Ptr, u32, f32) -> u32),
        entry!(0x00b5_f0c0, lists::find_and_cull_light_recurse(Ptr, u32, u32)),
        entry!(0x00b5_f150, lists::find_and_cull_light(Ptr, u32, u32)),
        entry!(0x00b5_cd00, lists::make_dynamic(Ptr, u32) -> u32),
        entry!(0x00b5_d930, lists::follow_moved_light(Ptr, u32)),
        entry!(0x00b5_b1f0, lists::drop_all_shadow_maps(Ptr)),
        entry!(0x00b5_b0e0, lists::remove_geometry_from_property_lights(u32)),
        entry!(0x00b5_b150, lists::remove_object_recurse(Ptr, u32)),
        entry!(0x00b5_b1c0, lists::remove_object(Ptr, u32)),
        entry!(0x00b5_c6e0, lists::add_unique(Ptr, u32)),
        entry!(0x00e7_4d40, lists::remove_all(Ptr<NiTPointerList>)),
        entry!(0x00cb_9ce0, lists::grow(Ptr, u32, u32)),
        entry!(0x00b5_c7c0, node::remove_all_jump(Ptr<NiTPointerList>)),
        entry!(0x00b5_aac0, node::set_sun_light(Ptr, u32)),
        entry!(0x00b5_ac90, node::update_occlusion_plane_visibility(Ptr, u32)),
        entry!(0x00b5_eca0, node::add_listed_light(Ptr, u32) -> u32),
        entry!(0x00b5_d040, node::remove_queued_light(Ptr, Ptr, u32)),
        entry!(0x00b5_ef90, node::remove_queued_light_of(Ptr, Ptr)),
        entry!(0x00b5_b4d0, node::light_value_at(Ptr, u32, u32, u32, Ptr, Ptr, Ptr, u32) -> f32),
        entry!(0x00b5_b880, node::update_dynamic_shadows(Ptr, u32)),
        entry!(0x00b5_e5a0, node::destruct(Ptr)),
        entry!(0x00b5_f980, node::delete(Ptr, u32) -> Ptr),
    ]
}
