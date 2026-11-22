//! The dispatch family (speed batch D1 of the migration, stage 1): the functions the town calls by
//! address most often, picked by calls by address a frame in a profiled Goodsprings run, each with
//! the callers whose call sites make those calls, so that the calls become Rust calls. A typed
//! callee reached by address from a translation still pays the call by address; a typed caller
//! calling it with `typed_call!` doesn't. The list with each function's calls a frame is
//! `measure/typed/hand/batchD1.tsv` (private).
//!
//! - [`math`]: the transforms' and matrices' arithmetic (compose, invert, transpose, matrix times
//!   matrix and vector), points scaled and normalised, the multi-bound box's corners, the 4x4
//!   inverse for the shaders, the Havok vector scale, float closeness;
//! - [`scene`]: the scene graph's walks and getters: an object's controllers, children and their
//!   updates, the collision object and its RTTI cast, a reference's 3D, the frustum test of a bound,
//!   the geometry's immediate render;
//! - [`refs`]: the counted references, the recursive spin lock, lists, arrays and the small getters
//!   the AI, the inventory, the HUD's compass and the process levels call on every actor; and the
//!   word map's lookup (`00853130`), which a quicksave makes about 230,000 times in its one frame
//!   (2026-11-10: about 30 ms of the 120 ms save frame by address), registered too at `00a1e380`,
//!   the exe's second copy of the same body (the menus' maps and the threads' scrap heaps).
//! - [`extra`]: the extra data lookup (`00410220`) and four getters over it, which the process lists
//!   ask for every actor when a reference goes away (Stewie's Engine Optimizations, measure/stewie-eo).
//! - [`saveload`]: the clean-up of expired cells' data that every save runs first (`0084ab20`), its
//!   expiry queue a native hash map (Stewie's Engine Optimizations, item 2).
//! - [`tile`]: the menus' tile values (the sorted trait array's find, with the C runtime's `bsearch`, and insert, `GetValue`,
//!   `GetOrCreateValue`), the reaction map's `AddReaction` and `ClearActions`, the pointer maps' `SetAt`
//!   and the menu string map's and integer maps' hash and compare, which the map lookup calls
//!   directly (Stewie's Engine Optimizations, item 7: menus open faster); and the tiles' trait
//!   getters and setters (`GetFloat`, `GetString`, `IsTrue`, `SetFloat`, `SetString`, `SetInt`,
//!   `Poke`, the absolute position sums, `Lock`/`Unlock`) and `ValueChangeEvent`, which the HUD's
//!   update calls on every frame (its compass alone about 1,700 calls a frame on the street). The
//!   value's own `SetFloat`, `SetString` and `CalculateValue` stay calls by address (overridable).
//!
//! Stage 1: the objects stay in guest memory where the game made them. Each function stays
//! registered at its exe address; a call inside the family, or into another family, is a
//! `typed_call!`.
//!
//! **Floats.** These run on the main thread and on others ([`Threads::Both`]; the thread census has a
//! quarter of the transforms' calls and most of the reference location's off the main thread), so
//! their maths is native by ADR-0014's float rule: `f64` locals rounded where the exe stores a float
//! ([`crate::anim_keys::wide`], [`crate::anim_keys::narrow`]), in the exe's order. On the other
//! threads' 53-bit precision that is the exe's arithmetic bit for bit. A single operation on two
//! floats stored at once (a product, a quotient, a difference) is plain `f32`: its `f64` form rounds
//! to the same float (a product of two floats is exact in an `f64`; a quotient or a difference rounded
//! to 53 bits and then to 24 rounds as once, since 53 >= 2 * 24 + 2), and on the main thread's 24-bit
//! precision it is the exe's float too while the result stays in the float's normal range. The matrix
//! and vector products' sums (`dot3`) are where the main thread's result may differ from the exe's in
//! its last bits: the exe rounds each product and each sum to 24 bits there, native code once. No x87
//! helper is left in the family; a float the exe copies through the x87 stack is copied as it is (a
//! signalling NaN keeps its payload).

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

use crate::culling::layout::{NiAVObject, NiMatrix3, NiNode, NiPoint3, NiTransform};

pub mod extra;
pub mod layout;
pub mod math;
pub mod refs;
pub mod saveload;
pub mod scene;
pub mod tile;

use layout::{LightLock, RefObject};

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(9);

/// `NV_TYPED=dispatch`. Past its speed gate (2026-11-08, together with the other five families of this crate: real
/// threads, the fixes preset, the same exe off and on, BAAB): Goodsprings 55.0 to 60.1 fps (+9.3%), the crowded
/// street at 2560x1440 8x AA 52.3 to 57.7 fps (+10.3%), the sky above it 117 to 134 fps (+14%). On unless
/// `NV_TYPED` says otherwise (`NV_TYPED=off` or `dispatch:off`).
pub static DISPATCH: Family = Family {
    name: "dispatch",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

/// Copies `count` words from `from` to `to` (`MOVSD`/`REP MOVSD`, never through the x87 stack).
#[inline(always)]
pub(crate) fn copy_words(e: &mut engine_core::Engine, from: u32, to: u32, count: u32) {
    for i in 0..count {
        let word = e.mem.u32(from.wrapping_add(4 * i));
        e.mem.set_u32(to.wrapping_add(4 * i), word);
    }
}

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        // The arithmetic.
        entry!(0x0062_c250, math::compose(Ptr<NiTransform>, Ptr<NiTransform>, Ptr<NiTransform>) -> Ptr<NiTransform>),
        entry!(0x004b_4880, math::invert_non_uniform(Ptr<NiTransform>, Ptr<NiTransform>)),
        entry!(0x004b_4500, math::rotate(Ptr<NiMatrix3>, Ptr<NiPoint3>, Ptr<NiPoint3>) -> Ptr<NiPoint3>),
        entry!(0x0043_f8d0, math::multiply(Ptr<NiMatrix3>, Ptr<NiMatrix3>, Ptr<NiMatrix3>) -> Ptr<NiMatrix3>),
        entry!(0x0047_68c0, math::transpose(Ptr<NiMatrix3>, Ptr<NiMatrix3>) -> Ptr<NiMatrix3>),
        entry!(0x0040_1050, math::construct_array(u32, u32, i32, u32)),
        entry!(0x0047_6930, math::row(Ptr<NiMatrix3>, u32, Ptr<NiPoint3>)),
        entry!(0x0047_6980, math::set_columns(Ptr<NiMatrix3>, Ptr<NiPoint3>, Ptr<NiPoint3>, Ptr<NiPoint3>) -> Ptr<NiMatrix3>),
        entry!(0x0047_69c0, math::set_column(Ptr<NiMatrix3>, u32, Ptr<NiPoint3>)),
        entry!(0x0047_6a80, math::transform_new(Ptr<NiTransform>) -> Ptr<NiTransform>),
        entry!(0x0045_bb20, math::times(Ptr<NiPoint3>, Ptr<NiPoint3>, f32) -> Ptr<NiPoint3>),
        entry!(0x004a_0c10, math::normalize(Ptr<NiPoint3>)),
        entry!(0x00c3_9270, math::box_corners(Ptr, Ptr)),
        entry!(0x00c4_c2d0, math::inverse_4x4(Ptr<NiTransform>, u32, Ptr)),
        entry!(0x004a_3e00, math::havok_vector(Ptr, Ptr<NiPoint3>) -> Ptr),
        entry!(0x004a_3e90, math::havok_scale(f32) -> f32),
        entry!(0x0056_0d30, math::lane(Ptr, u32) -> Ptr),
        entry!(0x004a_3c90, math::copy_16(Ptr, Ptr) -> Ptr),
        entry!(0x0045_2dc0, math::point2_new(Ptr, f32, f32) -> Ptr),
        entry!(0x0049_e2f0, math::points_close(Ptr<NiPoint3>, Ptr<NiPoint3>, f32) -> bool),
        entry!(0x0049_e390, math::floats_close(f32, f32, f32) -> bool),
        // The scene graph.
        entry!(0x0043_b230, scene::first_controller(Ptr<NiAVObject>) -> Ptr),
        entry!(0x004a_8a90, scene::next_controller(Ptr) -> u32),
        entry!(0x004e_ff50, scene::update_object_controllers(Ptr<NiAVObject>, Ptr, u8)),
        entry!(0x004f_0000, scene::has_property_controllers(Ptr<NiAVObject>) -> bool),
        entry!(0x0043_b480, scene::child_count(Ptr<NiNode>) -> u16),
        entry!(0x0043_b4a0, scene::child(Ptr<NiNode>, u32) -> Ptr),
        entry!(0x0065_8930, scene::array_size(Ptr) -> u16),
        entry!(0x00a5_dd70, scene::update_downward(Ptr<NiNode>, u32, u32)),
        entry!(0x00a5_ded0, scene::update_selected_downward(Ptr<NiNode>, u32, u32)),
        entry!(0x00a5_e0b0, scene::update_transform_and_bounds(Ptr<NiNode>, u32)),
        entry!(0x00a8_0610, scene::geometry_render_immediate(Ptr, Ptr)),
        entry!(0x00a7_1d70, scene::tri_strips_render_immediate(Ptr, Ptr)),
        entry!(0x0043_fcd0, scene::reference_3d(Ptr) -> Ptr),
        entry!(0x0043_b4f0, scene::loaded_3d(Ptr, Ptr) -> Ptr),
        entry!(0x0043_b540, scene::handle_3d(Ptr) -> u32),
        entry!(0x0043_b560, scene::loaded_handle(Ptr) -> u32),
        entry!(0x0043_b5b0, scene::lock_loaded(Ptr)),
        entry!(0x0043_b5d0, scene::loaded_lock(Ptr) -> Ptr),
        entry!(0x0043_b5f0, scene::unlock_loaded(Ptr)),
        entry!(0x0043_b610, scene::collision_object(Ptr<NiAVObject>) -> Ptr),
        entry!(0x0068_38b0, scene::collision_word(Ptr) -> u32),
        entry!(0x0065_3270, scene::cast_if(u32, u32) -> u32),
        entry!(0x0065_3290, scene::cast(Ptr, u32) -> Ptr),
        entry!(0x0065_32c0, scene::is_kind_of(Ptr, u32) -> bool),
        entry!(0x0045_3470, scene::esize_of_children(Ptr) -> u32),
        entry!(0x0099_38b0, scene::esize(Ptr) -> u16),
        entry!(0x004b_5fc0, scene::bound_in_frustum(Ptr<NiAVObject>, Ptr) -> bool),
        entry!(0x004b_5ff0, scene::in_frustum(Ptr, Ptr) -> bool),
        entry!(0x004b_4cf0, scene::plane(Ptr, u32) -> Ptr),
        entry!(0x0062_0b80, scene::word_8(u32) -> u32),
        entry!(0x004a_e750, scene::character_proxy(Ptr) -> u32),
        entry!(0x0056_0e10, scene::body_vector(Ptr) -> Ptr),
        entry!(0x0056_0e50, scene::body_vector_of(Ptr) -> Ptr),
        entry!(0x009d_9f40, scene::part_e0(Ptr) -> Ptr),
        // References, locks, lists and getters.
        entry!(0x0040_b460, refs::increment(Ptr) -> i32),
        entry!(0x0040_19a0, refs::decrement(Ptr) -> i32),
        entry!(0x0040_f6e0, refs::add_reference(Ptr<RefObject>)),
        entry!(0x0040_1970, refs::release(Ptr<RefObject>)),
        entry!(0x0045_cec0, refs::release_held(Ptr)),
        entry!(0x0049_68b0, refs::ref_object_new(Ptr<RefObject>) -> Ptr<RefObject>),
        entry!(0x0049_6910, refs::ref_object_drop(Ptr<RefObject>)),
        entry!(0x0040_fc90, refs::current_thread() -> u32),
        entry!(0x0043_b460, refs::compare_exchange(Ptr, i32, i32) -> i32),
        entry!(0x0040_fbf0, refs::lock(Ptr<LightLock>, u32)),
        entry!(0x0040_fba0, refs::unlock(Ptr<LightLock>)),
        entry!(0x0045_34f0, refs::nothing_2(u32, u32)),
        entry!(0x0048_3710, refs::nothing_1(u32)),
        entry!(0x0046_0140, refs::at_10(Ptr) -> Ptr),
        entry!(0x008c_71b0, refs::store_word(Ptr, u32) -> Ptr),
        entry!(0x0048_c1b0, refs::reaction(Ptr, u32) -> u32),
        entry!(0x008f_6650, refs::detection_state(Ptr, u32, i32) -> Ptr),
        entry!(0x0096_8670, refs::array_item(Ptr, u32) -> u32),
        entry!(0x004c_8f30, refs::item_count(Ptr, u32) -> i32),
        entry!(0x004b_ffb0, refs::container_of(Ptr) -> u32),
        entry!(0x004b_fba0, refs::entry_for(Ptr, u32, u32, u32) -> u32),
        entry!(0x008a_e4c0, refs::location_on_reference(Ptr) -> u32),
        entry!(0x0043_6aa0, refs::position(Ptr) -> Ptr),
        entry!(0x005a_2030, refs::byte_14d(Ptr) -> u8),
        entry!(0x0088_44f0, refs::in_state(Ptr, bool) -> bool),
        entry!(0x004f_8960, refs::word_108(Ptr) -> u32),
        entry!(0x0090_7650, refs::compute_last_time_processed(Ptr)),
        entry!(0x0069_3d50, refs::set_hour(Ptr, f32)),
        entry!(0x0070_37c0, refs::set_date(Ptr, u32)),
        entry!(0x0077_b300, refs::compass_order(Ptr<u32>, Ptr<u32>) -> i32),
        entry!(0x0086_a050, refs::object_factor(u32) -> f64),
        entry!(0x009a_d610, refs::word_cc(Ptr) -> Ptr),
        entry!(0x0084_54f0, refs::array_clear(Ptr, u8)),
        entry!(0x0069_aee0, refs::order_array_new(Ptr) -> Ptr),
        entry!(0x0069_af10, refs::info_array_drop(Ptr) -> Ptr),
        entry!(0x004f_1540, refs::extra_kind(Ptr) -> u8),
        // The extra data lookup and the getters over it the reference clean-up asks for every actor.
        entry!(0x0041_0220, extra::get_extra_data(Ptr, u8) -> Ptr),
        entry!(0x0042_2700, extra::followers(Ptr) -> Ptr),
        entry!(0x0041_9ea0, extra::head_track_target(Ptr) -> u32),
        entry!(0x0042_12b0, extra::actor_package_data(Ptr) -> u32),
        entry!(0x0041_cb70, extra::package_target(Ptr) -> u32),
        entry!(0x0084_ab20, saveload::cleanup_expired_data(Ptr)),
        entry!(0x0084_b930, saveload::queue_hash(Ptr, u32, u32) -> u32),
        entry!(0x0084_b950, saveload::queue_keys_equal(Ptr, u32, u32, u32, u32) -> bool),
        entry!(0x0084_b7c0, saveload::queue_insert(Ptr, u32, u32, u32)),
        // The menus' tile values and their reaction map (Engine Optimizations, item 7).
        entry!(0x00a0_1160, tile::compare_values(Ptr, Ptr) -> i32),
        entry!(0x00a0_cb90, tile::sorted_find(Ptr, u32, u32) -> i32),
        entry!(0x00ec_716d, tile::bsearch(u32, u32, u32, u32, u32) -> u32),
        entry!(0x00a0_caf0, tile::sorted_insert(Ptr, u32, u32)),
        entry!(0x00a0_0e90, tile::get_value_q(Ptr, i32) -> Ptr),
        entry!(0x00a0_0f30, tile::get_value(Ptr, i32) -> Ptr),
        entry!(0x00a0_1000, tile::get_or_create_value(Ptr, i32) -> Ptr),
        entry!(0x00a0_d1b0, tile::string_hash(Ptr, Ptr) -> u32),
        entry!(0x00a0_d180, tile::string_keys_equal(Ptr, Ptr, Ptr) -> bool),
        entry!(0x00a0_d260, tile::int_hash(Ptr, u32) -> u32),
        entry!(0x00ae_8dc0, tile::int_keys_equal(u32, u32, u32) -> bool),
        entry!(0x00a0_c6c0, tile::map_set_at(Ptr, u32, u32)),
        entry!(0x00a0_a130, tile::add_reaction(Ptr, Ptr)),
        entry!(0x00a0_9200, tile::clear_actions(Ptr)),
        // The tiles' trait getters and setters the menus and the HUD call on every update.
        entry!(0x00a0_44f0, tile::lock()),
        entry!(0x00a0_4500, tile::unlock()),
        entry!(0x00a0_11b0, tile::get_float(Ptr, i32) -> f32),
        entry!(0x00a0_11f0, tile::get_string(Ptr, i32) -> Ptr),
        entry!(0x00a0_1230, tile::is_true(Ptr, i32) -> bool),
        entry!(0x00a0_12d0, tile::set_float(Ptr, i32, f32, u8)),
        entry!(0x00a0_1350, tile::set_string(Ptr, i32, Ptr, u8)),
        entry!(0x0070_0320, tile::set_int(Ptr, i32, i32)),
        entry!(0x00a0_1290, tile::poke(Ptr, i32, f32)),
        entry!(0x00a0_13d0, tile::absolute_x(Ptr) -> f32),
        entry!(0x00a0_1440, tile::absolute_y(Ptr) -> f32),
        entry!(0x00a0_a220, tile::value_change_event(Ptr)),
        // The action links' tile search a menu's build makes for every link (every menu's opening).
        entry!(0x00a0_8b20, tile::find_by_path(Ptr, Ptr) -> Ptr),
        entry!(0x00a0_8f20, tile::find_named_in_tree(Ptr, Ptr) -> Ptr),
        entry!(0x00a0_3da0, tile::child_by_name(Ptr, Ptr) -> Ptr),
        entry!(0x0049_c680, refs::list_find(Ptr, Ptr<u32>, u32) -> u32),
        entry!(0x0071_9b20, refs::array_find(Ptr, u32, u32, u32) -> i32),
        entry!(0x0085_3130, refs::map_lookup(Ptr, u32, Ptr<u32>) -> u8),
        // The same function at a second address: instruction for instruction the same body.
        entry!(0x00a1_e380, refs::map_lookup(Ptr, u32, Ptr<u32>) -> u8),
    ]
}
