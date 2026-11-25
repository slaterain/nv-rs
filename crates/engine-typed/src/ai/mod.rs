//! The AI family: the game's AI code (the `fallout/ai` source units: the actors, their processes,
//! packages, combat and the player character), turned native file by file. Each module is one
//! source unit's functions, registered at their exe addresses over the translations in
//! `engine-fallout-ai`.
//!
//! - [`actor_package`]: `ActorPackage` (its Xbox PDB source unit): the package record a process
//!   holds (the current one is embedded in every process): its clear, its start from a package,
//!   its move, its reference count, its save and load (the save/load game's and a buffer's) and
//!   its flags; with the data's delete and the deleted reference handler its code shares.
//! - [`base_process`]: `BaseProcess` (its Xbox PDB source unit): the defaults every process
//!   class keeps in its vtable (the stubs that answer false, 0, -1 or 0.0 and those that do
//!   nothing), the embedded current package's getters and setters, the destructor, `Copy`, the
//!   save and load of the process block and the breath timer.
//! - [`middle_low_process`]: `MiddleLowProcess` (its Xbox PDB source unit): the temporary and
//!   damage modifiers on top of `LowProcess`'s actor values, the hourly package check, the combat
//!   rounds, `Update` (the package's procedure actions), the destructors, the copy and the save,
//!   load and revert of its part of the process.
//!
//! Reached: each stays registered at its exe address, so the translations' calls by address and
//! the vtable slots that hold them reach the typed functions; between them the calls are typed
//! calls, and a virtual call whose slot holds one of them (the process classes' own slots, read
//! from their vtables) is made as that typed call. A slot holding anything else is called through
//! the slot, as the exe calls it.
//!
//! Stage 1: the objects stay in guest memory where the game made them. **Threads and floats.** The
//! process code runs on main and on the AI threads, so the row is [`Threads::Both`] and the tests
//! compare under both control words. Most floats here are moved, not computed (a load that quiets a
//! signalling NaN, a sign flip, a zero, the calendar's hour stored as a float, an integer stored as
//! a float). The few computed ones follow the float rule (`crate::native`): a sum stored as a float
//! is one double sum rounded to a float, which is the exe's result at either precision for the
//! float-valued operands it gets; a product or quotient that is compared or truncated before any
//! store is a float operation on main (24 bits) and a double one on the AI threads (53 bits).

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod actor_package;
pub mod base_process;
pub mod middle_low_process;

/// This family's bit: what each `typed_call!` of it names (39).
pub const FAMILY: FamilyId = FamilyId::new(39);

/// `NV_TYPED=ai`. A plain native conversion proved by its differential tests, so on unless
/// `NV_TYPED` says otherwise.
pub static AI: Family = Family {
    name: "ai",
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
        entry!(0x0040_1280, base_process::empty_name(Ptr) -> u32),
        entry!(0x008c_5e70, actor_package::destroy_data(Ptr)),
        entry!(0x008c_5ec0, actor_package::handle_deleted_reference(Ptr, u32)),
        entry!(0x008c_5ef0, actor_package::clear(Ptr)),
        entry!(0x008c_6030, actor_package::dec_ref_count(Ptr) -> u32),
        entry!(0x008c_6060, actor_package::null_instance_data(Ptr)),
        entry!(0x008c_6080, actor_package::init_from_package(Ptr, Ptr, u32)),
        entry!(0x008c_6100, actor_package::inc_ref_count(Ptr) -> u32),
        entry!(0x008c_6130, actor_package::move_to(Ptr, Ptr)),
        entry!(0x008c_61a0, actor_package::save_size(Ptr, u32) -> u32),
        entry!(0x008c_6250, actor_package::save(Ptr, u32)),
        entry!(0x008c_6320, actor_package::load_game(Ptr, u32, u32)),
        entry!(0x008c_64d0, actor_package::init_load_game(Ptr, u32, u32)),
        entry!(0x008c_65a0, actor_package::revert_unused(Ptr, u32)),
        entry!(0x008c_65d0, actor_package::save_game(Ptr, Ptr)),
        entry!(0x008c_6730, actor_package::load_game_buffer(Ptr, Ptr)),
        entry!(0x008c_6920, actor_package::set_flag(Ptr, u32, u8)),
        entry!(0x008c_6970, actor_package::init_load_game_buffer(Ptr, Ptr)),
        entry!(0x008c_6b00, actor_package::flag(Ptr, u32) -> u32),
        entry!(0x008c_6b20, actor_package::finish_load_game(Ptr, Ptr)),
        entry!(0x008c_6c80, actor_package::revert(Ptr, u32)),
        entry!(0x008d_02d0, base_process::last_time_detected(Ptr, u32, u32) -> f32),
        entry!(0x008d_02f0, base_process::false_6(Ptr, u32, u32, u32, u32, u32, u32) -> bool),
        entry!(0x008d_0300, base_process::false_2(Ptr, u32, u32) -> bool),
        entry!(0x008d_0310, base_process::shared_point_copy(Ptr, Ptr, u32, u32) -> Ptr),
        entry!(0x008d_0340, base_process::zero_7(Ptr, u32, u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x008d_0350, base_process::nothing_7(Ptr, u32, u32, u32, u32, u32, u32, u32)),
        entry!(0x008d_0370, base_process::false_1(Ptr, u32) -> bool),
        entry!(
            0x008d_0380,
            base_process::nothing_11(Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32)
        ),
        entry!(0x008d_0390, base_process::float_zero_2(Ptr, u32, u32) -> f32),
        entry!(0x008d_03a0, base_process::clear_current_package(Ptr)),
        entry!(0x008d_03c0, base_process::set_current_procedure_index(Ptr, u32)),
        entry!(0x008d_03e0, base_process::current_procedure_index(Ptr) -> u32),
        entry!(0x008d_0400, base_process::add_to_current_procedure_index(Ptr, u32)),
        entry!(0x008d_0420, base_process::nothing_8(Ptr, u32, u32, u32, u32, u32, u32, u32, u32)),
        entry!(0x008d_0430, base_process::running_package_word(Ptr) -> u32),
        entry!(0x008d_0450, base_process::procedure_index_running(Ptr) -> u32),
        entry!(0x008d_0470, base_process::set_procedure_index_running(Ptr, u32)),
        entry!(0x008d_04a0, base_process::add_to_procedure_index_running(Ptr, u32, u32)),
        entry!(0x008d_04d0, base_process::set_char_controller(Ptr, u32)),
        entry!(0x008d_04f0, base_process::nothing_6(Ptr, u32, u32, u32, u32, u32, u32)),
        entry!(0x008d_0500, base_process::nothing_5(Ptr, u32, u32, u32, u32, u32)),
        entry!(0x008d_0510, base_process::false_4(Ptr, u32, u32, u32, u32) -> bool),
        entry!(0x008d_0520, base_process::false_3(Ptr, u32, u32, u32) -> bool),
        entry!(
            0x008d_0530,
            base_process::false_13(
                Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32
            ) -> bool
        ),
        entry!(
            0x008d_0540,
            base_process::calculate_move_mode(Ptr, u32, u32, u32, u32, u32, u32) -> u32
        ),
        entry!(0x008d_0560, base_process::zero_1(Ptr, u32) -> u32),
        entry!(0x008d_0570, base_process::minus_one_3(Ptr, u32, u32, u32) -> i32),
        entry!(0x008d_0580, base_process::float_zero_3(Ptr, u32, u32, u32) -> f32),
        entry!(0x008d_0590, base_process::sound_handle(Ptr, Ptr, u32) -> Ptr),
        entry!(0x008d_0610, base_process::scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x008d_0640, base_process::destructor(Ptr)),
        entry!(0x008d_0660, base_process::copy(Ptr, Ptr)),
        entry!(0x008d_0870, base_process::handle_deleted_reference(Ptr, u32)),
        entry!(0x008d_08b0, base_process::set_current_package(Ptr, u32, u32)),
        entry!(0x008d_08f0, base_process::weapon_last_pos(Ptr) -> u32),
        entry!(0x008d_0900, base_process::save_size(Ptr, u32, u32) -> u16),
        entry!(0x008d_0a10, base_process::save_game(Ptr, u32, u32)),
        entry!(0x008d_0bc0, base_process::load_game(Ptr, u32, u32, u32)),
        entry!(0x008d_0e60, base_process::init_load_game(Ptr, u32, u32, u32)),
        entry!(0x008d_0e80, base_process::finish_init_load_game(Ptr, u32, u32, u32)),
        entry!(0x008d_0f10, base_process::revert(Ptr, u32, u32)),
        entry!(0x008d_0f30, base_process::save_game_buffer(Ptr, u32)),
        entry!(0x008d_0f90, base_process::load_game_buffer(Ptr, u32)),
        entry!(0x008d_1000, base_process::init_load_game_buffer(Ptr, u32)),
        entry!(0x008d_1030, base_process::cached_values_out_of_date(Ptr)),
        entry!(0x008d_1050, base_process::finish_load_game(Ptr, u32)),
        entry!(0x008d_1070, base_process::revert_buffer(Ptr, u32)),
        entry!(0x008d_10c0, base_process::breath_timer(Ptr) -> f32),
        entry!(0x0090_7430, middle_low_process::damage_mod_actor_value(Ptr, u32, u32, i32)),
        entry!(0x0092_c9d0, middle_low_process::scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0092_ca00, middle_low_process::destructor(Ptr)),
        entry!(0x0092_ca70, middle_low_process::copy(Ptr, Ptr)),
        entry!(0x0092_cab0, middle_low_process::process_combat(Ptr, u32)),
        entry!(0x0092_cc90, middle_low_process::process_combat_one_hour(Ptr, u32, u32) -> bool),
        entry!(0x0092_cd90, middle_low_process::get_actor_value(Ptr, u32, u32, u32) -> i32),
        entry!(0x0092_ce00, middle_low_process::get_actor_float_value(Ptr, u32, u32, u32) -> f64),
        entry!(0x0092_ce70, middle_low_process::temp_mod_actor_value(Ptr, u32, u32, i32)),
        entry!(0x0092_cec0, middle_low_process::temp_mod_actor_value_float(Ptr, u32, u32, f32)),
        entry!(0x0092_cf10, middle_low_process::get_temporary_modifier(Ptr, u8) -> f64),
        entry!(0x0092_cf30, middle_low_process::damage_mod_actor_value_ov2(Ptr, u32, u32, f32)),
        entry!(0x0092_cfd0, middle_low_process::checkfor_new_package(Ptr, u32, u8) -> u8),
        entry!(0x0092_d050, middle_low_process::update(Ptr, u32)),
        entry!(0x0092_e400, middle_low_process::update_step(Ptr, u32, f32)),
        entry!(0x0092_e420, middle_low_process::get_save_size(Ptr, u32, u32) -> u16),
        entry!(0x0092_e560, middle_low_process::save_game(Ptr, u32, u32)),
        entry!(0x0092_e730, middle_low_process::load_game(Ptr, u32, u32, u32)),
        entry!(0x0092_e9f0, middle_low_process::revert(Ptr, u32, u32)),
        entry!(0x0092_ea30, middle_low_process::save_game_buffer(Ptr, Ptr)),
        entry!(0x0092_eaa0, middle_low_process::load_game_buffer(Ptr, u32)),
        entry!(0x0092_eb00, middle_low_process::revert_buffer(Ptr, u32)),
    ]
}
