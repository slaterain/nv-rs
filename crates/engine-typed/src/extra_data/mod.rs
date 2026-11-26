//! The extra data family (`NV_TYPED=extra_data`): what the game asks of and puts into a
//! reference's, a cell's or an inventory entry's list of extra data (`ExtraDataList`, Xbox PDB),
//! from the exe's extra data list unit: `AddExtraCopy` and the copies and compares of whole lists
//! ([`copy`]), the getters and setters of single extra data ([`access`]), the small records the
//! unit's loader and save writer build ([`records`]), the loader `ExtraDataList::Load` ([`load`]),
//! the fix-up of a loaded list `InitItem` ([`init_item`]), the list's save writer ([`save`]), and
//! from the unit's part 2 file the second run of setters, getters and removers ([`setters`]), the
//! cell's extra data ([`cell_data`]) and the package extra data with the rest of that range
//! ([`packages`]). The list's own operations (its constructors, `AddExtra`, `RemoveExtra`, the type
//! bitmap, the thread's cache) are the cell references family's (`cell_refs::extra_list`), the
//! lookup `GetExtraData` (`00410220`) the dispatch family's. Every caller reaches these functions
//! by address; the unit's translation that stays (`GetCanNotWear` `00418b10`) and the unit's other
//! part files call the native functions by address too. Three functions of the part 2 range are
//! other families': `ScriptLocals`' scalar deleting destructor `0041af70` (native in the script
//! lines family, the one that holds xNVSE's wraps), a package's type `0041ca90` (the AI queries',
//! read in place here) and `GetPackageExtraTarget` `0041cb70` (the dispatch family's).
//!
//! **Native.** The exe's order of work. The lookup (`00410220`), the lock (`0040fbf0`, `0040fba0`),
//! `AddExtra`, both `RemoveExtra`s, `ItemsInList` and the cache's clear are typed calls into their
//! families; a function of this family calls another as a typed call. The one-instruction leaves
//! the exe calls are done in place: an extra data's type (`004f1540`) and next (`0044ddc0`,
//! `00403550`), a list node's item and next (`006815c0`, `00726070`), a handle's word (`00559450`,
//! with JIP LN NVSE's null check as the leaf has it), a buffer's form (`007af430`), the type
//! bitmap's test (`0040fe80`), `BSExtraData`'s constructor (`0040ec80`), the sound handle's
//! constructors, compares and empty destructor, the audio buoy marker's empty copy (`004534f0`),
//! the small records' constructors, a form's id (`0084e3a0`), a package's type (`0041ca90`), a time
//! stamp's byte (`0086a460`), the save/load singleton's test that always answers false
//! (`0047c850`), an `NiPointer`'s compare with a pointer (`00822510`) and a getter's temporary
//! `NiPointer` made from null (`00633c90`, released by `0045cec0`: 0). Each type, next or word is
//! read once where the exe reads it again with nothing in between that could change it. A virtual
//! call (an extra data's slot 0 delete, its slot 4 `Compare`, the type 0x92 copy's slot 0xc) stays
//! a call through the slot: each class has its own body. The constructors, destructors and setters
//! of other units, `operator new` and `operator delete` are called by address.
//!
//! **Extenders.** JIP LN NVSE has seven sites here, each kept where the translation has it:
//! `AddExtraCopy`'s arm for its keyed data's type 0x27 (`0x411978`) and that type's entries in the
//! two default scans' tables (`0x41D0F6`, `0x41D212`, `0x41D25A`), while JIP is installed; the
//! hotkey copy of `CopyListForReference` (`0x412675`), the owner test of `CompareListForContainer`
//! (`0x41271E`) and the reference pointer setter's persistence test (`0x41C833`), with its fixes
//! on. JIP wraps `GetCanNotWear` (`00418b10`), which stays translated, and xNVSE `0041af70` (the
//! script lines family's): both stay out of this family and are called by address, as a wrapped
//! member would send the whole family by address. No other extender replaces or patches these
//! functions; xNVSE, JIP and JohnnyGuitar call the getters, the setters and `AddExtraCopy` by
//! address, which reaches them here.
//!
//! **Which thread.** Every thread that touches a list (main, the loaders, the AI threads), under
//! the extra data lock as the exe takes it: [`Threads::Both`]. The floats are moved, not computed
//! (each through the x87's load and store as the exe does, so a signalling NaN comes out quiet),
//! but for the rotation matrix's products, worked in `f64` and rounded to `float` where the exe
//! stores, and the sine and cosine, the x87's `FSINCOS` as `engine_core::x87` has it.

pub mod access;
pub mod cell_data;
pub mod copy;
pub mod init_item;
pub mod load;
pub mod packages;
pub mod records;
pub mod save;
pub mod setters;

use crate::{cell_refs, crt, dispatch};
use dispatch::layout::LightLock;
use engine_core::ptr::Ptr;
use engine_core::typed::{leaf, Family, FamilyId, Threads};
use engine_core::{args, entry, typed_call, AbiFn, Engine};

/// This family's bit: what each `typed_call!` of it names. Id 46: the next after the highest in use
/// when it was made (44 on main, 45 in an open pull request).
pub const FAMILY: FamilyId = FamilyId::new(46);

/// `NV_TYPED=extra_data`. A native conversion proved by its differential tests: on unless
/// `NV_TYPED` says otherwise (`NV_TYPED=off` or `extra_data:off`).
pub static EXTRA_DATA: Family = Family {
    name: "extra_data",
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
        entry!(0x0041_03d0, copy::add_extra_copy(u32, u32)),
        entry!(0x0041_1ec0, copy::copy_list(u32, u32)),
        entry!(0x0041_1f20, copy::is_copyable_extra(u32, u32) -> bool),
        entry!(0x0041_1fd0, copy::remove_all_copyable_extra(u32, bool)),
        entry!(0x0041_20b0, copy::remove_non_persistent_cell_data(u32)),
        entry!(0x0041_21b0, copy::thread_flag_4(u32) -> bool),
        entry!(0x0041_21e0, copy::copy_list_for_container(u32, u32, bool)),
        entry!(0x0041_2380, copy::duplicate_for_container(u32, u32)),
        entry!(0x0041_2490, copy::copy_list_for_reference(u32, u32, bool)),
        entry!(0x0041_26c0, copy::compare_list_for_container(u32, u32, bool, bool) -> bool),
        entry!(0x0041_27e0, copy::compare_list(u32, u32) -> bool),
        entry!(0x0041_2970, save::save_list(u32)),
        entry!(0x0041_3f40, records::plane_center(u32) -> u32),
        entry!(0x0041_3f60, records::plane_rotation(u32, u32) -> u32),
        entry!(0x0041_3f90, records::primitive_color(u32, u32) -> u32),
        entry!(0x0041_3fc0, records::primitive_radii(u32, u32) -> u32),
        entry!(0x0041_3ff0, records::float_copy(u32, u32) -> u32),
        entry!(0x0041_4010, records::empty_node_new(u32) -> u32),
        entry!(0x0041_4030, records::swap_decal(u32)),
        entry!(0x0041_40f0, records::swap_plane(u32)),
        entry!(0x0041_41e0, records::swap_word_short(u32)),
        entry!(0x0041_4220, records::swap_four_words(u32)),
        entry!(0x0041_4290, records::swap_primitive(u32)),
        entry!(0x0041_4370, records::swap_plane_refs(u32)),
        entry!(0x0041_43c0, records::portal_ref_new(u32) -> u32),
        entry!(0x0041_43f0, records::primitive_new(u32) -> u32),
        entry!(0x0041_4430, records::color_new(u32, f32, f32, f32, f32) -> u32),
        entry!(0x0041_4470, records::plane_new(u32) -> u32),
        entry!(0x0041_44a0, load::load_chunk(u32, u32, u32)),
        entry!(0x0041_68a0, records::rotation_matrix(u32, f32, f32, f32, f32)),
        entry!(0x0041_69a0, records::sincos(f32, u32, u32)),
        entry!(0x0041_69d0, records::word_1d8(u32) -> u32),
        entry!(0x0041_69f0, records::plane_set_center(u32, u32)),
        entry!(0x0041_6a30, records::plane_set_size(u32, u32)),
        entry!(0x0041_6a70, records::plane_set_rotation(u32, u32)),
        entry!(0x0041_6ab0, records::portal_record_new(u32) -> u32),
        entry!(0x0041_6ad0, records::portal_extra_new(u32, u32, u32) -> u32),
        entry!(0x0041_6b40, records::radio_data_new(u32) -> u32),
        entry!(0x0041_6b80, records::room_links_new(u32) -> u32),
        entry!(0x0041_6be0, init_item::init_item(u32, u32)),
        entry!(0x0041_81c0, records::plane_set_linked(u32, u32, u32)),
        entry!(0x0041_81e0, access::save_form(u32) -> u32),
        entry!(0x0041_8200, access::anim_save(u32) -> u32),
        entry!(0x0041_8220, access::animation(u32) -> u32),
        entry!(0x0041_8250, access::light(u32) -> u32),
        entry!(0x0041_8280, access::spell_effect_light(u32) -> u32),
        entry!(0x0041_82b0, access::lock_data(u32) -> u32),
        entry!(0x0041_82e0, access::radio_data(u32) -> u32),
        entry!(0x0041_8310, access::set_radio_data(u32, u32)),
        entry!(0x0041_83e0, access::radio_extra_new(u32) -> u32),
        entry!(0x0041_8460, access::teleport(u32) -> u32),
        entry!(0x0041_8490, access::map_marker(u32) -> u32),
        entry!(0x0041_84c0, access::audio_marker(u32) -> u32),
        entry!(0x0041_84f0, access::audio_buoy_marker(u32) -> u32),
        entry!(0x0041_8520, access::container_changes(u32) -> u32),
        entry!(0x0041_8550, access::set_original_reference(u32, u32)),
        entry!(0x0041_8600, access::remove_original_reference(u32)),
        entry!(0x0041_8630, access::original_reference(u32) -> u32),
        entry!(0x0041_8660, access::ownership(u32) -> u32),
        entry!(0x0041_8690, access::global(u32) -> u32),
        entry!(0x0041_86c0, access::rank(u32) -> i32),
        entry!(0x0041_86f0, access::health(u32) -> f32),
        entry!(0x0041_8720, access::leveled_item(u32) -> u32),
        entry!(0x0041_8750, access::has_leveled_item(u32) -> bool),
        entry!(0x0041_8770, access::count(u32) -> u16),
        entry!(0x0041_87a0, access::charge(u32) -> f32),
        entry!(0x0041_87d0, access::poison(u32) -> u32),
        entry!(0x0041_8800, access::script(u32) -> u32),
        entry!(0x0041_8830, access::script_locals(u32) -> u32),
        entry!(0x0041_8860, access::scale(u32) -> f32),
        entry!(0x0041_8890, access::sound(u32, u32) -> u32),
        entry!(0x0041_88d0, access::sound_handle_new(u32) -> u32),
        entry!(0x0041_8900, access::sound_handle_copy(u32, u32) -> u32),
        entry!(0x0041_8940, access::creature_awake_sound(u32, u32) -> u32),
        entry!(0x0041_8980, access::creature_movement_sound(u32, u32) -> u32),
        entry!(0x0041_89c0, access::weapon_idle_sound(u32, u32) -> u32),
        entry!(0x0041_8a00, access::weapon_attack_sound(u32, u32) -> u32),
        entry!(0x0041_8a40, access::activate_loop_sound(u32, u32) -> u32),
        entry!(0x0041_8a80, access::is_ghost(u32) -> bool),
        entry!(0x0041_8ab0, access::worn(u32, bool) -> bool),
        entry!(0x0041_8b40, access::seed(u32) -> u8),
        entry!(0x0041_8b70, access::package_start_location(u32) -> u32),
        entry!(0x0041_8ba0, access::has_weapon_mods(u32) -> bool),
        entry!(0x0041_8be0, access::weapon_mods_any(u32) -> bool),
        entry!(0x0041_8c00, access::weapon_mod_slot_active(u32, u8) -> bool),
        entry!(0x0041_8c40, access::set_anim_ptr(u32, u32) -> u32),
        entry!(0x0041_8d50, access::starting_position(u32, u32) -> u32),
        entry!(0x0041_9050, access::set_lock_ptr(u32, u32) -> u32),
        entry!(0x0041_9120, access::set_teleport(u32, u32) -> u32),
        entry!(0x0041_9220, access::teleport_data_delete(u32, u32) -> u32),
        entry!(0x0041_9250, access::set_map_marker(u32, u32) -> u32),
        entry!(0x0041_9350, access::map_marker_data_delete(u32, u32) -> u32),
        entry!(0x0041_9380, access::set_audio_marker(u32, u32) -> u32),
        entry!(0x0041_9480, access::audio_marker_data_delete(u32, u32) -> u32),
        entry!(0x0041_94b0, access::set_audio_buoy_marker(u32, u32) -> u32),
        entry!(0x0041_95b0, access::action(u32) -> u32),
        entry!(0x0041_9650, access::set_container_changes(u32, u32)),
        entry!(0x0041_9700, access::set_ownership(u32, u32)),
        entry!(0x0041_97d0, access::set_global(u32, u32)),
        entry!(0x0041_98a0, access::set_rank(u32, i32)),
        entry!(0x0041_9970, access::set_health(u32, f32)),
        entry!(0x0041_9a20, access::set_uses(u32, u8)),
        entry!(0x0041_9ad0, access::set_count(u32, u16)),
        entry!(0x0041_9bb0, access::set_time_left(u32, f32)),
        entry!(0x0041_9c60, access::set_charge(u32, f32)),
        entry!(0x0041_9d10, access::set_poison(u32, u32)),
        entry!(0x0041_9dc0, access::set_item_dropper(u32, u32)),
        entry!(0x0041_9ed0, access::set_script(u32, u32)),
        entry!(0x0041_9f80, access::set_script_locals(u32, u32)),
        entry!(0x0041_9fb0, access::set_scale(u32, f32)),
        entry!(0x0041_a090, access::set_creature_awake_sound(u32, u32)),
        entry!(0x0041_a1f0, access::same_sound_id(u32, u32) -> bool),
        entry!(0x0041_a220, access::other_sound_id(u32, u32) -> bool),
        entry!(0x0041_a250, access::sound_handle_init(u32) -> u32),
        entry!(0x0041_a280, access::set_creature_movement_sound(u32, u32)),
        entry!(0x0041_a3e0, access::set_weapon_idle_sound(u32, u32)),
        entry!(0x0041_a540, access::set_weapon_attack_sound(u32, u32)),
        entry!(0x0041_a6a0, setters::set_activate_loop_sound(u32, u32)),
        entry!(0x0041_a800, setters::set_sound(u32, u32)),
        entry!(0x0041_a960, setters::set_ghost(u32, bool)),
        entry!(0x0041_aa20, setters::set_worn(u32, bool, bool)),
        entry!(0x0041_ab70, setters::set_can_not_wear(u32, bool)),
        entry!(0x0041_ac30, setters::set_seed(u32, u8)),
        entry!(0x0041_ad00, setters::set_package_start_location(u32, u32, u32, u32, f32)),
        entry!(0x0041_adf0, setters::remove_anim_ptr(u32)),
        entry!(0x0041_ae10, setters::remove_anim_save(u32)),
        entry!(0x0041_ae30, setters::remove_light(u32)),
        entry!(0x0041_ae50, setters::remove_spell_effect_light(u32)),
        entry!(0x0041_ae70, setters::remove_lock_ptr(u32)),
        entry!(0x0041_ae90, setters::remove_teleport_ptr(u32)),
        entry!(0x0041_aeb0, setters::remove_container_changes(u32)),
        entry!(0x0041_aed0, setters::remove_ownership(u32)),
        entry!(0x0041_aef0, setters::remove_health(u32)),
        entry!(0x0041_af10, setters::remove_count(u32)),
        entry!(0x0041_af30, setters::remove_poison(u32)),
        entry!(0x0041_af50, setters::remove_script(u32)),
        entry!(0x0041_afa0, setters::clear_script_locals(u32)),
        entry!(0x0041_afd0, setters::remove_scale(u32)),
        entry!(0x0041_aff0, setters::remove_ghost(u32)),
        entry!(0x0041_b010, setters::remove_worn(u32, bool)),
        entry!(0x0041_b040, setters::remove_can_not_wear(u32)),
        entry!(0x0041_b060, setters::remove_package_start_location(u32)),
        entry!(0x0041_b080, setters::starting_rotation(u32, u32, u32) -> u32),
        entry!(0x0041_b0d0, setters::starting_location(u32, u32, u32) -> u32),
        entry!(0x0041_b120, setters::set_starting_rotation(u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x0041_b180, setters::set_starting_location(u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x0041_b1e0, setters::set_starting_world_or_cell(u32, u32)),
        entry!(0x0041_b2a0, setters::set_starting_world_or_cell_for_ref(u32, u32)),
        entry!(0x0041_b320, setters::starting_world_or_cell(u32) -> u32),
        entry!(0x0041_b350, setters::remove_starting_world_or_cell(u32)),
        entry!(0x0041_b370, setters::action_flags(u32) -> u32),
        entry!(0x0041_b3a0, setters::has_action_flag(u32, u32) -> bool),
        entry!(0x0041_b3d0, setters::set_action_flags(u32, u32)),
        entry!(0x0041_b440, setters::add_action_flags(u32, u32)),
        entry!(0x0041_b470, setters::remove_action_flags(u32, u32)),
        entry!(0x0041_b4b0, setters::set_action_ref(u32, u32)),
        entry!(0x0041_b520, setters::action_ref(u32) -> u32),
        entry!(0x0041_b550, setters::health_perc(u32) -> f32),
        entry!(0x0041_b580, setters::set_health_perc(u32, f32)),
        entry!(0x0041_b650, setters::health_perc_new(u32) -> u32),
        entry!(0x0041_b680, setters::health_perc_delete(u32, u32) -> u32),
        entry!(0x0041_b6b0, setters::object_health(u32) -> f32),
        entry!(0x0041_b6e0, setters::set_object_health(u32, f32)),
        entry!(0x0041_b790, setters::object_health_new(u32, f32) -> u32),
        entry!(0x0041_b7c0, setters::remove_object_health(u32)),
        entry!(0x0041_b7e0, cell_data::set_cell_3d(u32, u32)),
        entry!(0x0041_b8d0, cell_data::set_havok_world(u32, u32)),
        entry!(0x0041_b9a0, cell_data::havok_world(u32) -> u32),
        entry!(0x0041_ba60, cell_data::set_cell_mopp(u32, u32)),
        entry!(0x0041_bb10, cell_data::cell_mopp(u32) -> u32),
        entry!(0x0041_bbd0, cell_data::set_region_list(u32, u32)),
        entry!(0x0041_bce0, cell_data::region_list(u32) -> u32),
        entry!(0x0041_bd10, cell_data::set_music_type(u32, u32)),
        entry!(0x0041_bde0, cell_data::music_type(u32) -> u32),
        entry!(0x0041_be10, cell_data::set_terminal_state(u32, u8)),
        entry!(0x0041_bef0, cell_data::terminal_state_new(u32) -> u32),
        entry!(0x0041_bf20, cell_data::terminal_state_empty(u32) -> bool),
        entry!(0x0041_bf50, cell_data::terminal_state(u32) -> u8),
        entry!(0x0041_bf80, cell_data::set_lock_override(u32, u8)),
        entry!(0x0041_c060, cell_data::lock_override(u32) -> u8),
        entry!(0x0041_c090, cell_data::set_acoustic_space(u32, u32)),
        entry!(0x0041_c160, cell_data::acoustic_space(u32) -> u32),
        entry!(0x0041_c190, cell_data::set_climate(u32, u32)),
        entry!(0x0041_c260, cell_data::climate(u32) -> u32),
        entry!(0x0041_c290, cell_data::set_image_space(u32, u32)),
        entry!(0x0041_c360, cell_data::image_space(u32) -> u32),
        entry!(0x0041_c390, cell_data::set_impact_swap(u32, u32)),
        entry!(0x0041_c460, cell_data::impact_swap(u32) -> u32),
        entry!(0x0041_c490, cell_data::set_canopy_shadow_mask(u32, u32, u32, u32)),
        entry!(0x0041_c580, cell_data::canopy_shadow_mask(u32, u32, u32) -> u32),
        entry!(0x0041_c5e0, cell_data::editor_id_new(u32, u32) -> u32),
        entry!(0x0041_c660, cell_data::editor_id_delete(u32, u32) -> u32),
        entry!(0x0041_c690, cell_data::editor_id_drop(u32)),
        entry!(0x0041_c6f0, cell_data::set_editor_id(u32, u32)),
        entry!(0x0041_c7c0, cell_data::editor_id_text(u32) -> u32),
        entry!(0x0041_c7f0, cell_data::set_reference_pointer(u32, u32)),
        entry!(0x0041_c8d0, cell_data::reference_pointer(u32) -> u32),
        entry!(0x0041_c900, cell_data::remove_reference_pointer(u32)),
        entry!(0x0041_c930, packages::set_package(u32, u32, i32, u32, u8, u8, u8)),
        entry!(0x0041_cab0, packages::set_package_target(u32, u32)),
        entry!(0x0041_cae0, packages::set_package_index(u32, i32)),
        entry!(0x0041_cb10, packages::package(u32) -> u32),
        entry!(0x0041_cb40, packages::package_index(u32) -> i32),
        entry!(0x0041_cba0, packages::package_action_complete(u32) -> u8),
        entry!(0x0041_cbd0, packages::set_package_action_complete(u32, u8)),
        entry!(0x0041_cc00, packages::package_activated(u32) -> u8),
        entry!(0x0041_cc30, packages::package_done_once(u32) -> u8),
        entry!(0x0041_cc60, packages::remove_package(u32)),
        entry!(0x0041_cc90, packages::set_trespass_package(u32, u32)),
        entry!(0x0041_cd70, packages::trespass_package(u32) -> u32),
        entry!(0x0041_cda0, packages::remove_trespass_package(u32)),
        entry!(0x0041_ce10, packages::forget_trespass_package(u32, u32)),
        entry!(0x0041_ce50, packages::add_to_player_crime_list(u32, u32)),
        entry!(0x0041_cf00, packages::player_crime_list(u32) -> u32),
        entry!(0x0041_cf30, packages::remove_player_crime_list(u32, u32)),
        entry!(0x0041_d000, packages::remove_leveled_item(u32)),
        entry!(0x0041_d030, packages::only_owner_defaults(u32) -> bool),
        entry!(0x0041_d120, packages::is_default_for_container(u32, u8) -> bool),
        entry!(0x0041_d280, packages::set_leveled_item_index(u32, i32)),
        entry!(0x0041_d330, packages::set_leveled_item_default(u32, u8)),
        entry!(0x0041_d360, packages::leveled_item_index(u32) -> i32),
        entry!(0x0041_d390, packages::set_persistent_cell(u32, u32)),
        entry!(0x0041_d460, packages::persistent_cell(u32) -> u32),
        entry!(0x0041_d490, packages::set_rag_doll_data(u32, u32)),
        entry!(0x0041_d5b0, packages::set_rag_doll_data_from_copy(u32, u32)),
        entry!(0x0041_d6d0, packages::rag_doll_data(u32) -> u32),
        entry!(0x0041_d700, packages::add_run_once_package(u32, u32, u8)),
        entry!(0x0041_d7b0, packages::prune_run_once_packages(u32, i8)),
        entry!(0x0041_d8a0, packages::package_time_stamp(u32) -> u32),
        entry!(0x0041_d8c0, packages::is_in_run_once_packages(u32, u32) -> bool),
        entry!(0x0041_d930, packages::remove_run_once_packages(u32)),
        entry!(0x0041_d950, packages::set_land_normal(u32, u32)),
        entry!(0x0041_da10, packages::enable_state_parent(u32) -> u32),
        entry!(0x0041_da40, packages::set_enable_state_parent(u32, u32)),
    ]
}

// ---- What the family's files share ----------------------------------------------------------

/// The extra data lock (`BaseExtraList::ExtraCritSection`, Xbox PDB).
const EXTRA_LOCK: u32 = 0x011c_3920;
/// A list's head (+4) and type bitmap (+8, 21 bytes).
const LIST_HEAD: u32 = 4;
const LIST_FLAGS: u32 = 8;
const FLAGS_LEN: u32 = 0x15;
/// An extra data's type (+4), next (+8) and payload (+0xc).
const EXTRA_TYPE: u32 = 4;
const EXTRA_NEXT: u32 = 8;
const EXTRA_PAYLOAD: u32 = 0xc;
/// `BSExtraData`'s vtable, which its constructor sets.
const VTABLE_BS_EXTRA_DATA: u32 = 0x0101_4210;
/// `operator new` (`cdecl(size)`) and `operator delete` (`cdecl(block)`).
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// The constructor of an empty `BSSimpleList` (two null words, `thiscall(list)`).
const SIMPLE_LIST_NEW: u32 = 0x0096_a2d0;

/// The extra data lock taken (`0040fbf0`) under `name`.
#[inline]
fn take_lock(e: &mut Engine, name: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_fbf0,
        dispatch::refs::lock(e, Ptr::<LightLock>::new(EXTRA_LOCK), name)
    );
}

/// The extra data lock given back (`0040fba0`).
#[inline]
fn give_lock(e: &mut Engine) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_fba0,
        dispatch::refs::unlock(e, Ptr::<LightLock>::new(EXTRA_LOCK))
    );
}

/// An extra data's type, in place (`004f1540`'s `MOV AL,[ECX+4]`).
#[inline]
fn kind_of(e: &Engine, extra: u32) -> u8 {
    e.mem.u8(extra.wrapping_add(EXTRA_TYPE))
}

/// An extra data's next, in place (`0044ddc0`'s `MOV EAX,[ECX+8]`).
#[inline]
fn next_of(e: &Engine, extra: u32) -> u32 {
    e.mem.u32(extra.wrapping_add(EXTRA_NEXT))
}

/// An extra data's next set, in place (`00403550`'s `MOV [ECX+8],arg`).
#[inline]
fn set_next_of(e: &mut Engine, extra: u32, next: u32) {
    e.mem.set_u32(extra.wrapping_add(EXTRA_NEXT), next);
}

/// A list's head.
#[inline]
fn head_of(e: &Engine, list: u32) -> u32 {
    e.mem.u32(list.wrapping_add(LIST_HEAD))
}

/// The word at `offset` of an extra data or a record.
#[inline]
fn word_at(e: &Engine, extra: u32, offset: u32) -> u32 {
    e.mem.u32(extra.wrapping_add(offset))
}

/// The word a handle points at, in place (`00559450`): a null handle as the leaf answers it (0 with
/// JIP LN NVSE's null check in, else the game's read of address 0).
#[inline]
fn read_word(e: &mut Engine, at: u32) -> u32 {
    if at == 0 {
        leaf::word_at_0_of_null(e)
    } else {
        e.mem.u32(at)
    }
}

/// A list node's next, in place (`00726070`'s `MOV EAX,[ECX+4]`).
#[inline]
fn next_node(e: &Engine, node: u32) -> u32 {
    e.mem.u32(node.wrapping_add(4))
}

/// `BaseExtraList::HasExtra` (`0040fe80`) in place: the type's bit in the bitmap, false past it.
#[inline]
fn has_kind(e: &Engine, list: u32, kind: u8) -> bool {
    let byte = u32::from(kind >> 3);
    byte < FLAGS_LEN && e.mem.u8(list.wrapping_add(LIST_FLAGS + byte)) & (1 << (kind & 7)) != 0
}

/// `BSExtraData`'s constructor (`0040ec80`) in place: the base vtable, the type, a null next.
#[inline]
fn extra_init(e: &mut Engine, this: u32, kind: u8) {
    e.mem.set_u32(this, VTABLE_BS_EXTRA_DATA);
    e.mem.set_u8(this.wrapping_add(EXTRA_TYPE), kind);
    e.mem.set_u32(this.wrapping_add(EXTRA_NEXT), 0);
}

/// The scalar deleting destructors' end: `operator delete` when bit 0 of `flags` is set. Answers
/// `this`.
#[inline]
fn free_when(e: &mut Engine, this: u32, flags: u32) -> u32 {
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

/// The first extra data of `kind` in the list: the dispatch family's first_of_kind (`00410220`).
#[inline]
fn first_of_kind(e: &mut Engine, list: u32, kind: u8) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0041_0220,
        dispatch::extra::get_extra_data(e, Ptr::new(list), kind)
    )
    .addr()
}

/// `BaseExtraList::AddExtra` (`0040ff60`, the cell references family's).
#[inline]
fn list_add(e: &mut Engine, list: u32, extra: u32) {
    typed_call!(
        cell_refs::FAMILY,
        0x0040_ff60,
        cell_refs::extra_list::add_extra(e, list, extra)
    );
}

/// `BaseExtraList::RemoveExtra` (`00410020`, the cell references family's).
#[inline]
fn list_remove(e: &mut Engine, list: u32, extra: u32, destroy: bool) {
    typed_call!(
        cell_refs::FAMILY,
        0x0041_0020,
        cell_refs::extra_list::remove_extra(e, list, extra, destroy)
    );
}

/// `ClearLastExtraAll` (`0040f860`, the cell references family's).
#[inline]
fn forget_cached_all(e: &mut Engine, list: u32) {
    typed_call!(
        cell_refs::FAMILY,
        0x0040_f860,
        cell_refs::extra_list::clear_last_extra_all(e, list)
    );
}

/// An extra data deleted through its slot 0 (the scalar deleting destructor with 1); none for null.
#[inline]
fn delete_extra_data(e: &mut Engine, extra: u32) {
    if extra != 0 {
        e.vcall(extra, 0, &args![1u32]);
    }
}

/// `n` bytes at `at` cleared (`memset` through the C runtime family's wrapper, `00403d30`).
#[inline]
fn zero_bytes(e: &mut Engine, at: u32, n: u32) {
    typed_call!(
        crt::FAMILY,
        0x0040_3d30,
        crt::mem::memset_wrapper(e, Ptr::new(at), 0, n)
    );
}

/// `new` and construct: `size` bytes from `operator new`, `build` run on the block; null when the
/// allocation failed.
#[inline]
fn new_object(e: &mut Engine, size: u32, build: impl FnOnce(&mut Engine, u32) -> u32) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        0
    } else {
        build(e, block)
    }
}

/// The words `[from, to)` (byte offsets) of `source` copied to the same offsets of `target`.
#[inline]
fn copy_words(e: &mut Engine, source: u32, target: u32, from: u32, to: u32) {
    for offset in (from..to).step_by(4) {
        let word = e.mem.u32(source.wrapping_add(offset));
        e.mem.set_u32(target.wrapping_add(offset), word);
    }
}
