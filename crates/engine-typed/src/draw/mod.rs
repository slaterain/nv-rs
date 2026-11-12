//! The draw family (family 18, stage 1): the main world draw's drivers, typed as the groundwork for
//! running that draw on its own thread. The world pass (`00873200`) calls the draw's two halves; each
//! goes through the accumulator's slots to the render mode's draw and finish, which draw the batch
//! renderers' pass lists by pass type range and register group, the decal pass and the alpha pass.
//! The secondary scenes (water reflections, menus, the world pass's extra scenes) reach the same
//! functions through their own draw ([`drivers::scene_draw`]), so these are typed once for every caller.
//!
//! - [`drivers`]: the halves, the slots, render mode 0's halves, the pass type range draw, the
//!   register group draw, the alpha sort, the active list clears;
//! - [`alpha`]: the alpha group draw with its distances and clears, a holder's draw, the shadow types'
//!   group-clearing draw;
//! - the geometry paths the batch renderer's draws reach (group D): [`shapes`] (the geometries'
//!   immediate draws, the segmented shape's merge, the geometry data getters), [`packed`] (the renderer's
//!   shape, partition and points draws, with the particles' quad builders and vertex declarations),
//!   [`skin`] (the skinned shader classes' bone set-up and the renderer's bone matrices) and [`packing`]
//!   (a skin partition's buffer, a shader declaration's stream pack and build, chips, the vertex lock);
//!   [`layout`] has the classes they read;
//! - the pass begin and end the batch renderer's draws reach through a shader's vtable (group B):
//!   [`passes`] (`BSShader`'s slots: the pass's render states set and put back, the alpha function and
//!   test of a geometry, the end's states, the begin's end; the render state put back by state, the
//!   masked stages, the fog property), [`setups`] (the shadow light shader's: the pass choice, the type
//!   groups' begins and per-geometry set-ups, the tail of the pass kinds 1 to 5, the pass constants, the
//!   land toggles, the lighting property's fade) and [`constants`] (the vertex shader's constant
//!   setters);
//! - the D3DX import thunks the typed families call (group E): [`thunks`] (each calls its import slot,
//!   which the platform layer answers; the typed callers reach them directly);
//! - [`packet`] and [`own`]: the accessors. Everything the draw reads that the rest of the frame
//!   changes goes through [`packet`]; everything it writes that is its own through [`own`], one section
//!   for each of the pipeline probe's conditions that live here (1: the pass lists the next frame's
//!   registration fills again; 2: blend, clip and depth state and the device's transform, streams and
//!   indices, and the shader state a pass begin leaves (the current pass, its stage textures, the
//!   constant switches, the fog and toggle constants, the device's vertex shader constants); 3: the
//!   changed flags and buffers of the geometry it packs and draws; 4: the blocks it frees; 5: the blocks
//!   it allocates; 7: the reference counts, with the shader's counted pointers the pass choice sets).
//!   Not routed yet: the packing's own objects (the buffer data's streams and chips, the shader
//!   declaration, the dynamic geometry group, the vertex manager's staging block), which [`packing`]
//!   and [`packed`] read and write in place; the shader package's tables, the passes' contents (stages,
//!   state lists, masks, programs), the shaders' own fields (their renderer, the counted pointers'
//!   sources), the shader property's texture source +0xac and land table bytes, the constant map entries'
//!   fields and the pass type tables, the default stage texture `011f951c`, the fade distances
//!   `011f9454`/`011f9458`, the card's alpha-to-coverage switch `011f9421` (read by [`lists`]), the element bits
//!   `011ae640`, the constant kinds `011f5fc4` and the vertex shader's
//!   renderer's device word (+0x10f8), all filled at load and read in place by [`passes`], [`setups`] and
//!   [`constants`] (group B's other reads and writes, its own writes read back included, go through
//!   [`packet`] and [`own`]); and the exe's constant data (the -FLT_MAX and 0.001 the alpha pass
//!   compares with, the colour scale, the bone scale bounds, the far distance with no fog distances, the
//!   1.0 at `01012070`).
//!   A later step can feed the draw from a copied packet and give a drawing thread its own state by
//!   changing those two files. Step 1 of that ([`detach`], `NV_DRAW_DETACH=1`, a check that is slower and
//!   not for play) takes the pass lists' words out of guest memory while the world draw runs; nothing else
//!   is copied, deferred or moved yet.
//!
//! The drawing itself (the batch renderer's lists, passes and state setters) is the shader family's,
//! reached by `typed_call!`s that go direct while both families are on.
//!
//! Stage 1: the objects stay in guest memory where the game made them. Each function stays registered
//! at its exe address; a call the exe makes through a slot is direct when the slot holds the typed
//! function ([`crate::slot_target`]), except where the exe jumps with its caller's stack words still in
//! place (the world pass's two halves), which stay calls through the slot.
//!
//! **Threads and floats.** The drivers and the alpha pass run on the main thread only by the census
//! (`t3/threads.tsv`); the geometry paths' packing, shape draw and getters also on two other game
//! threads (about one call in 300 to one in 3,000), and six of the pass begin and end's functions and the
//! D3DX thunks `00ee6de0` and `00ee6dfe` also on game thread 12. The family keeps the shader family's
//! exact x87 helpers (the alpha distances, the particles' quads, the bone matrices): [`Threads::Both`].

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod alpha;
pub mod capture;
pub mod constants;
pub mod cut;
pub mod detach;
pub mod drivers;
pub mod group_c;
pub mod layout;
pub mod lighting30;
pub mod lists;
pub mod no_lighting;
pub mod own;
pub mod packed;
pub mod packet;
pub mod packing;
pub mod passes;
pub mod setups;
pub mod shapes;
pub mod skin;
pub mod sky;
pub mod speedtree;
pub mod tall_grass;
pub mod thunks;
pub mod water;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(18);

/// `NV_TYPED=draw`. Proved by its differential tests and the byte-identical recorded street run (on
/// against `NV_TYPED=draw:off`), so on unless `NV_TYPED` says otherwise.
pub static DRAW: Family = Family {
    name: "draw",
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
        // The halves, the secondary scenes' draw and the accumulator's slots.
        entry!(0x00b6_ba20, drivers::world_draw_first(u32, Ptr<_>, u32)),
        entry!(0x00b6_b930, drivers::world_draw_second(u32, Ptr<_>, u32)),
        entry!(0x00b6_c0d0, drivers::scene_draw(u32, Ptr<_>) -> u32),
        entry!(0x00b6_6520, drivers::accumulator_draw(Ptr<_>)),
        entry!(0x00b6_6570, drivers::accumulator_finish(Ptr<_>)),
        entry!(0x00d4_7a40, packet::set_camera(Ptr<_>, u32)),
        entry!(0x00aa_0fe0, packet::finish_accumulating(Ptr<_>)),
        entry!(0x00b5_4b10, own::hold_accumulator(u32)),
        // Render mode 0's halves and what they draw with.
        entry!(0x00b6_5ae0, drivers::first_half(Ptr<_>)),
        entry!(0x00b6_5c60, drivers::second_half(Ptr<_>)),
        entry!(0x00b6_3770, drivers::draw_types(Ptr<_>, u32, u32, u32, i32)),
        entry!(0x00b6_39e0, drivers::draw_group(Ptr<_>, u32, u32)),
        entry!(0x00b6_3a50, drivers::sort_alphas(Ptr<_>)),
        entry!(0x00b6_3120, drivers::has_listed_pass(Ptr<_>, u32, u32) -> bool),
        entry!(0x00b6_5580, drivers::clear_active_lists(Ptr<_>)),
        // The alpha pass and the batch renderer's group draws.
        entry!(0x00b6_50c0, alpha::draw_alpha_group(Ptr<_>, u32)),
        entry!(0x00b9_8c80, alpha::alpha_distance(Ptr<_>, u32, u32, u32) -> f64),
        entry!(0x00b6_4f20, alpha::clear_alpha_group(Ptr<_>, u32)),
        entry!(0x00b9_a3c0, alpha::draw_and_clear_type(Ptr<_>, u32, u32, u32, u32) -> bool),
        entry!(0x00b9_a4d0, alpha::draw_holder(u32, u32)),
        entry!(0x00b9_af60, alpha::render_next_alpha(Ptr<_>, u32, u32, u32, f32)),
        // The list draws the world draw made through translations (the drawing thread's still-live list).
        entry!(0x00b9_9990, lists::own_list_draw(Ptr<_>, u32)),
        entry!(0x00b9_9e20, lists::render_batches(Ptr<_>, u32, u32, u32)),
        // The decal pass and the fog-coloured ambient.
        entry!(0x00b6_4e30, drivers::decal_pass(Ptr<_>)),
        entry!(0x00bb_b840, own::fog_ambient()),
        // The camera handed to the scene renderer, with the camera getters it reads (game-wide).
        entry!(0x004e_9bb0, packet::hand_camera(u32, u32)),
        entry!(0x004e_9c90, packet::pass_camera_data(u32, u32, u32, u32, u32, u32, u32)),
        entry!(0x004a_0d10, packet::camera_frustum(u32) -> u32),
        entry!(0x0045_bbe0, packet::camera_port(u32) -> u32),
        entry!(0x0045_bb80, packet::world_translate(u32) -> u32),
        entry!(0x0045_bba0, packet::rotation_column_0(u32, u32) -> u32),
        entry!(0x004e_9c10, packet::rotation_column_1(u32, u32) -> u32),
        entry!(0x004e_9c50, packet::rotation_column_2(u32, u32) -> u32),
        // The geometries' immediate draws (their slot 0xe0) with the segment merge, and the data getters.
        entry!(0x00a7_4600, shapes::tri_shape_render_immediate(Ptr<_>, Ptr<_>)),
        entry!(0x00a8_7130, shapes::segmented_render_immediate(Ptr<_>, Ptr<_>)),
        entry!(0x00a8_7040, shapes::merge_enabled_segments(Ptr<_>)),
        entry!(0x00a9_b1f0, shapes::particles_render_immediate(Ptr<_>, Ptr<_>)),
        entry!(0x00a6_69e0, shapes::active_vertex_count(u32) -> u16),
        entry!(0x00a9_23a0, shapes::particles_active_vertex_count(u32) -> u16),
        entry!(0x00a7_5f20, shapes::active_triangle_count(u32) -> u16),
        entry!(0x00a9_23b0, shapes::strips_dynamic_active_triangle_count(u32) -> u16),
        entry!(0x00a7_5f10, shapes::strip_lists(u32) -> u32),
        entry!(0x00a7_5f30, shapes::strips_data_rtti() -> u32),
        entry!(0x00a9_2470, shapes::strips_dynamic_data_rtti() -> u32),
        entry!(0x0047_c850, shapes::answers_false(u32) -> bool),
        // The renderer's draws of a shape, a skin partition and points, with the quads and declarations.
        entry!(0x00e7_45a0, packed::draw_shape(Ptr<_>, Ptr<_>)),
        entry!(0x00e6_d310, packed::draw_partition(Ptr<_>, u32, u32, u32)),
        entry!(0x00e7_0140, packed::draw_points(Ptr<_>, Ptr<_>)),
        entry!(0x00e6_9ea0, packed::build_quads(Ptr<_>, u32, u32, u32, u32, u32, u32, u16)),
        entry!(0x00e6_ab40, packed::build_quads_with_byte(Ptr<_>, u32, u32, u32, u32, u32, u32, u32, u16)),
        entry!(0x00e6_96a0, packed::declaration(Ptr<_>) -> u32),
        entry!(0x00e6_9790, packed::declaration_with_byte(Ptr<_>) -> u32),
        // The skinned draw's bones.
        entry!(0x00b7_a2c0, skin::bone_setup(Ptr<_>, u32, u32, u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x00e6_fe30, skin::bone_matrices(Ptr<_>, Ptr<_>, Ptr<_>, u8, u32, u8)),
        // The packing: a skin partition's buffer, a declaration's stream and build, chips, the vertex lock.
        entry!(0x00e6_d780, packing::pack_skinned(Ptr<_>, Ptr<_>, Ptr<_>, u32, Ptr<_>, Ptr<_>, u8) -> bool),
        entry!(0x00e7_6770, packing::pack_stream(Ptr<_>, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x00e7_6a80, packing::build_declaration(Ptr<_>) -> u32),
        entry!(0x00e8_bfa0, packing::remake_chip(Ptr<_>, Ptr<_>, u32) -> bool),
        entry!(0x00e8_bff0, packing::lock_vertices(Ptr<_>, u32, u32, u32, u32) -> u32),
        entry!(0x00e8_c0d0, packing::unlock_vertices(Ptr<_>, u32) -> bool),
        entry!(0x00e9_5550, packing::create_chip(Ptr<_>, u32, u32) -> u32),
        // The pass begin and end (group B): the BSShader slots, the render state put back, the masked stages.
        entry!(0x00be_0eb0, passes::set_pass_states(u32, u32)),
        entry!(0x00be_0ee0, passes::restore_pass_states(u32, u32)),
        entry!(0x00b9_8610, passes::restore_state(u32, u32)),
        entry!(0x00be_20b0, passes::alpha_function_of(u32, u32)),
        entry!(0x00be_20e0, passes::alpha_test_of(u32, u32, bool)),
        entry!(0x00be_0e50, passes::end_states(u32, u32)),
        entry!(0x00be_0e90, passes::end_states_restored(u32)),
        entry!(0x00be_21b0, passes::bind_masked_stages()),
        entry!(0x00be_1f90, passes::begin_done(u32, u32)),
        entry!(0x00b5_5520, passes::fog_property(u32) -> u32),
        // The shadow light shader's pass set-ups (group B).
        entry!(0x00b7_dd50, setups::begin_types_fc(Ptr<_>)),
        entry!(0x00b7_deb0, setups::begin_types_ca(Ptr<_>)),
        entry!(0x00b7_ded0, setups::setup_type_ca(Ptr<_>, Ptr<_>)),
        entry!(0x00b7_dfe0, setups::setup_type_d1(Ptr<_>, Ptr<_>)),
        entry!(0x00b7_e150, setups::setup_type_102(Ptr<_>, Ptr<_>)),
        entry!(0x00b7_ac20, setups::tail_kinds_1_to_5(Ptr<_>, u32)),
        entry!(0x00b7_b450, setups::setup_pass_constants(Ptr<_>, u32)),
        entry!(0x00b7_af80, setups::select_pass(Ptr<_>, u32)),
        entry!(0x00c0_2c30, setups::land_toggles(Ptr<_>, u32, u32, u32, u32)),
        entry!(0x00b6_6b80, setups::property_fade(Ptr<_>) -> f32),
        // The vertex shader's constant setters (group B).
        entry!(0x00e8_9ba0, constants::set_constant(Ptr<_>, Ptr<_>, u32, u32) -> bool),
        entry!(0x00e8_9c70, constants::set_constant_array(Ptr<_>, Ptr<_>, u32, u32, u32, u32) -> bool),
        // The D3DX import thunks (group E).
        entry!(0x00ee_6dfe, thunks::matrix_multiply_transpose(Ptr<()>, Ptr<()>, Ptr<()>) -> Ptr<()>),
        entry!(0x00ee_6dec, thunks::vec3_normalize(Ptr<()>, Ptr<()>) -> Ptr<()>),
        entry!(0x00ee_6df2, thunks::vec3_transform_normal(Ptr<()>, Ptr<()>, Ptr<()>) -> Ptr<()>),
        entry!(0x00ee_6df8, thunks::vec3_transform_coord(Ptr<()>, Ptr<()>, Ptr<()>) -> Ptr<()>),
        entry!(0x00ee_6dd4, thunks::matrix_transpose(Ptr<()>, Ptr<()>) -> Ptr<()>),
        entry!(0x00ee_6dda, thunks::matrix_inverse(Ptr<()>, u32, Ptr<()>) -> Ptr<()>),
        entry!(0x00ee_6de0, thunks::matrix_multiply(Ptr<()>, Ptr<()>, Ptr<()>) -> Ptr<()>),
        entry!(0x00ee_6dce, thunks::plane_normalize(Ptr<()>, Ptr<()>) -> Ptr<()>),
        entry!(0x00ee_6dc8, thunks::plane_transform(Ptr<()>, Ptr<()>, Ptr<()>) -> Ptr<()>),
        // Group C: the render states' lock counters.
        entry!(0x0071_4a40, group_c::release_z_enable(u32)),
        entry!(0x0071_4a60, group_c::release_z_write(u32)),
        entry!(0x0071_4aa0, group_c::release_alpha_test(u32)),
        entry!(0x0071_4ae0, group_c::release_alpha_blend(u32)),
        entry!(0x004e_ced0, group_c::release_color_write(u32)),
        // Group C: the tall grass shader.
        entry!(0x00ba_ac10, tall_grass::setup_geometry(Ptr<()>, u32, u32, u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x00ba_c950, tall_grass::setup_textures(Ptr<()>, Ptr<()>)),
        entry!(0x00ba_af40, tall_grass::copy_instances(Ptr<()>, u32, Ptr<()>)),
        entry!(0x00ba_c9c0, tall_grass::setup_constants(Ptr<()>, Ptr<()>)),
        // Group C: the no-lighting shader.
        entry!(0x00bc_6970, no_lighting::select_pass(Ptr<()>, u32)),
        entry!(0x00bc_6de0, no_lighting::setup_fog(Ptr<()>, u32)),
        entry!(0x00bc_6fa0, no_lighting::copy_subtextures(Ptr<()>, Ptr<()>, u32)),
        entry!(0x00bc_6fe0, no_lighting::material_constants(Ptr<()>, Ptr<()>, Ptr<()>)),
        entry!(0x00bc_7100, no_lighting::light_fog(Ptr<()>)),
        entry!(0x00bc_8e00, no_lighting::texture_matrix(Ptr<()>, Ptr<()>)),
        entry!(0x00bc_8d40, no_lighting::setup_textures(Ptr<()>, Ptr<()>)),
        entry!(0x00bc_9a50, no_lighting::setup_constants(Ptr<()>, Ptr<()>)),
        // Group C: the sky shader.
        entry!(0x00b8_9cc0, sky::setup_textures(Ptr<()>, Ptr<()>)),
        entry!(0x00b8_9d80, sky::setup_constants(Ptr<()>, Ptr<()>)),
        // Group C: the water shader, the SpeedTree leaf shader, the pass flags, a material's colour.
        entry!(0x00b8_63a0, water::setup_textures(Ptr<()>, Ptr<()>)),
        entry!(0x00b8_64d0, water::setup_constants(Ptr<()>, Ptr<()>)),
        entry!(0x00bb_25d0, speedtree::setup_geometry(Ptr<()>, u32, u32, u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x00bb_2760, speedtree::setup_constants(Ptr<()>, Ptr<()>)),
        entry!(0x00bb_1b50, speedtree::setup_fog(Ptr<()>, u32)),
        entry!(0x00b8_15e0, group_c::pass_flags(u32)),
        entry!(0x00c0_3180, group_c::material_colour_constant(u32)),
        // Group C: the lighting 3.0 shader and the shared leftovers.
        entry!(0x00bb_e650, lighting30::setup_textures(Ptr<()>, Ptr<()>)),
        entry!(0x00bb_bda0, lighting30::emittance(Ptr<()>, Ptr<()>)),
        entry!(0x00bb_bdf0, lighting30::glow(Ptr<()>, Ptr<()>)),
        entry!(0x00bb_be30, lighting30::alpha(Ptr<()>)),
        entry!(0x00bb_bcf0, lighting30::eye_position(Ptr<()>, Ptr<()>)),
        entry!(0x00bb_ba00, lighting30::world_matrix(Ptr<()>, u32, Ptr<()>)),
        entry!(0x00bb_b890, lighting30::light_position(Ptr<()>, [u32; 16], Ptr<()>)),
        entry!(0x00bb_be80, lighting30::fog(Ptr<()>)),
        entry!(0x00bb_e9c0, lighting30::material(Ptr<()>, u32, Ptr<()>, Ptr<()>, u32)),
        entry!(0x00bb_ecd0, lighting30::colour(Ptr<()>, Ptr<()>, Ptr<()>)),
        entry!(0x00bb_e6b0, lighting30::view_matrix(Ptr<()>, Ptr<()>, u32, u32, Ptr<()>)),
        entry!(0x00bb_c050, lighting30::lights(Ptr<()>, Ptr<()>, Ptr<()>, u32, Ptr<()>, u32, u32, Ptr<()>)),
        entry!(0x00bb_ef90, lighting30::setup_constants(Ptr<()>, Ptr<()>)),
        entry!(0x00bb_e120, lighting30::setup_switches(Ptr<()>, u32)),
        entry!(0x00bd_02e0, lighting30::setup_stage_textures(Ptr<()>, Ptr<()>)),
        entry!(0x00bb_95e0, group_c::set_light_entry(u32, u32, u32, u32, u32, u32, u32, u32, u32)),
        entry!(0x00bb_96d0, group_c::pass_end_flags(Ptr<()>, u32)),
    ]
}
