//! The forms family (`NV_TYPED=forms`): the game's forms native, `TESForm` and the record classes
//! built on it (`engine-fallout-forms`), file by file.
//!
//! - [`tesform`]: every function of `TESForm`'s own unit (the Xbox PDB's `tesform` source
//!   unit) but `GetFormByEditorID` (`00483a00`, which JohnnyGuitar NVSE replaces) and the empty
//!   base destructor `00483710` (the dispatch family's; the typed callers leave its call out): the
//!   constructor and destructors, the flag setters, the form registries and their maps, the source
//!   file list (`GetFile`, `GetOwnerMaster`, `SetFile`), the save-change calls, the form's save and
//!   load of its flags, the components' `DATA` chunk, the copy and compare of its components, the
//!   form copier, and the record writer (`StartForm`, `CloseForm`, the `AddChunk` family,
//!   `__AddChunkData`, `CompressSaveBuffer`).
//! - [`actor_base_data`]: every function of `TESActorBaseData`'s unit (the component at +0x30 of
//!   `TESNPC` and `TESCreature`) but the save/load stub `0047c850` (the draw family's): its
//!   constructor and destructor, the flag getters and setters, the statistics' getters and
//!   setters, the faction list, the template forms, the record and save game chunks, the karma.
//! - [`actor_base`]: every function of `TESActorBase`'s unit (the base of `TESNPC` and
//!   `TESCreature`): the form-type flag tests, the actor value getter and setter, the attributes'
//!   getters and setters, the health, the change routing for created forms, an item's
//!   desirability, the worn items' check, the actor base's save and load, `Reset3DMobIterator`.
//! - [`attributes`]: every function of `TESAttributes`' unit and of the unit after it: the seven
//!   attribute bytes' record data, initialisation, copy, compare, getter and setter, their save
//!   game functions, and `TESBipedModelForm`'s constructor and destructor with `TESModelRDT`'s
//!   constructor.
//! - [`ai_form`]: every function of `TESAIForm`'s unit (the component at +0x90 of `TESNPC` and
//!   `TESCreature`) and its save buffer functions in the unit after it: the constructor and
//!   destructor, the AI data's getters and setters, the record data, the copy and compare, the
//!   package list's calls (`InitItem`, `GetCurrentPackage`, `GetMissedPackages`), the services an
//!   actor sells and buys, the save game functions.
//! - [`animation`]: `TESAnimation`'s functions (the component at +0xc4 of the actor base forms, in
//!   the unit after `TESAIForm`'s): the constructor and destructor, the KF file names' list, its
//!   chunk, its copy and compare.
//! - [`debug_string`]: the debug-heap `std::string` members the linker placed in `TESObject`'s unit
//!   and the run after it: its constructors, destructor, `c_str`, the assigns, `erase`, `_Tidy`, the
//!   `strlen` wrapper, `_Eos`, `char_traits::assign`, `_Grow`, `_Myptr`, `max_size`, `_Copy` and
//!   `_Inside`.
//! - [`std_exceptions`]: `std::logic_error`'s, `std::length_error`'s and `std::bad_alloc`'s
//!   constructors, copy constructors, `what()`, destructor bodies and scalar deleting destructors.
//! - [`word_vector`]: the debug `std::vector<unsigned long>` before the string (constructor,
//!   destructor, `clear`, `begin`, `erase`, `_Buy`, `max_size`) and its word allocator's
//!   `allocate`, `max_size` and one-word copy.
//! - [`queued_maps`]: the lock-free maps' scalar deleting destructors in the run after the string,
//!   the queued task pointer's release and the map iterator's flag test.
//! - [`object`]: every function of `TESObject`'s unit but `TESObject::IsMarker` (`0050f9e0`, which
//!   JIP LN NVSE rewrites): `TESObject`'s and the bound objects' constructors and destructors, the
//!   `OBND` box (its points, size, save and load), `Clone3D`, the model registration and the scaled
//!   model name.
//! - [`package_list`]: the package list's functions (`PackageList`'s unit and the units beside it):
//!   the destructor, clear, append, copy and compare, `InitItem`, the schedule window,
//!   `FindCurrentPackage` and the missed packages' walk and sort, and the missable flag's test.
//! - [`schedule`]: a package's schedule (`PSDT`): its order, getters and setters, constructor,
//!   assignment and compare, its chunk, and the three-word swap of its unit.
//! - [`package_data`]: the package data classes' constructors, destructors and save game loads
//!   (`TESPackageData` and its dialogue, eat, escort, follow, patrol and use item data), their
//!   chunk swaps, and the package event action's constructor, clear and copy.
//! - [`conditions`]: a condition list's constructor, destructor and clear, a condition's compare of
//!   two values and its function data's swap.
//!
//! Reached: each function stays registered at its exe address, so the translations' calls by
//! address and the vtable slots that hold them (`TESForm`'s vtable `0101c794` and the form classes
//! that keep its slots) reach the typed functions; between them the calls are typed calls. The
//! translations that called one of them as a plain Rust function (in `tesform.rs`,
//! `tesactorbasedata.rs`, `tesactorbase.rs`, `tesattributes.rs`, `run_004801e0.rs`, `tesaiform.rs`,
//! `run_0047f940.rs`, `tesobject.rs`, `packagelist.rs`, `run_0067ad20.rs`, `run_0067c330.rs`,
//! `run_0067ec40.rs`, `run_0067fcb0.rs`, `run_0044a880.rs` and `run_0044a170.rs` themselves) call
//! it by address now, so the game reaches the typed function from there too.
//!
//! Native: the one-line bodies the exe calls (the form type, its flag tests, the byte swaps, the
//! files' and save buffers' fields, the components' accessors) are done in place, the leaves', the
//! C runtime's, dispatch's and the save family's functions are typed calls, and so are this unit's
//! own (see [`tesform`]'s module comment for the list).
//!
//! What stays as the exe has it: every call that does real work outside the typed families is a
//! call by address (the form map's removal and the form array's operations, the save/load game's
//! change tables, the files' list operations, the allocator, the logger that YUP and xNVSE may
//! replace, the one-byte components' setter that JIP LN may replace, the name key JohnnyGuitar NVSE
//! may replace); every virtual call is made through its slot, since the form classes override them.
//! The seams inside the unit run where they do, with the words the translation gives them: the
//! script extender's in the destructor (`00483669`), JohnnyGuitar NVSE's after `SetTemporary`'s
//! removal (`0048449a`) and at `GetFormDetailedString`'s name (`00486903`), JIP LN's G45 mask in the
//! flags' load (`00484e03`, under its fix bit) and its clone of a model with lights of its own in
//! `Clone3D` (`0050ef46`, while JIP LN is installed). A seam is a check inside the function, not a
//! registration over it, so it costs the family nothing; no extender or fix row registers over any
//! of this family's addresses (`GetFormByEditorID`, which JohnnyGuitar NVSE replaces, and
//! `TESObject::IsMarker`, which JIP LN rewrites, stay out),
//! so the family's calls stay direct in play.
//! Plugins' load order and form ids are untouched: the ids, the files' order in a form's list and
//! the compile index are computed exactly as the translation computes them.
//!
//! Stage 1: the objects stay in guest memory where the game made them. The loading threads read and
//! write forms too, so the row is [`Threads::Both`], and the float maths is the exe's x87 maths under
//! the running thread's control word, as the translation does it: the calculated level's
//! multiply and divide, an item's desirability's add and the missed packages' hour step (`x87_mul`,
//! `x87_div`, `x87_add`), each result stored as the `f32` the exe stores; the karma loaded from an
//! integer exactly and read
//! back through the C runtime's typed `_ftol2`. A float becomes an integer through
//! `x87::fistp_chop` where the exe's `FISTP` runs under the truncating control word it sets (the
//! loaded quality, the armour's base rating, the statistics' fields). The components' weight and
//! quality are only moved (a getter's `ST0` stored as the `f32` the exe stores, a quality byte
//! widened exactly), and so are the AI form's copied settings and package query floats (through
//! the exe's `FLD`/`FSTP`, which quiets a signalling NaN); each module comment says how its own
//! floats are done.

use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod actor_base;
pub mod actor_base_data;
pub mod ai_form;
pub mod animation;
pub mod attributes;
pub mod conditions;
pub mod debug_string;
pub mod object;
pub mod package_data;
pub mod package_list;
pub mod queued_maps;
pub mod schedule;
pub mod std_exceptions;
pub mod tesform;
pub mod word_vector;

/// This family's bit: what each `typed_call!` of it names. Id 45: past the families in main
/// (0-35) and the interface lane's 44 in progress.
pub const FAMILY: FamilyId = FamilyId::new(45);

/// `NV_TYPED=forms`. A plain native conversion proved by its differential tests, so on unless
/// `NV_TYPED` says otherwise (`NV_TYPED=off` or `forms:off`).
pub static FORMS: Family = Family {
    name: "forms",
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
        entry!(0x0048_3370, tesform::construct(u32) -> u32),
        entry!(0x0048_35e0, tesform::handler_loading_file(u32) -> u32),
        entry!(0x0048_3600, tesform::scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0048_3630, tesform::destructor(u32)),
        entry!(0x0048_3720, tesform::each_component_slot_0(u32)),
        entry!(0x0048_3870, tesform::each_component_slot_1(u32)),
        entry!(0x0048_39c0, tesform::form_by_id(u32) -> u32),
        entry!(0x0048_3a50, tesform::create_registries()),
        entry!(0x0048_3b80, tesform::destroy_registries()),
        entry!(0x0048_3c70, tesform::remove_from_data_structures(u32)),
        entry!(0x0048_3d20, tesform::save(u32, u32) -> bool),
        entry!(0x0048_3d70, tesform::compress_save_buffer()),
        entry!(0x0048_3fc0, tesform::write_to_file(u32, u32) -> u8),
        entry!(0x0048_4020, tesform::type_ranks_before(u32, u32) -> bool),
        entry!(0x0048_4150, tesform::goes_before_group(u32, u32) -> bool),
        entry!(0x0048_4490, tesform::set_temporary(u32)),
        entry!(0x0048_44f0, tesform::set_flag_1(u32, bool)),
        entry!(0x0048_4530, tesform::set_delete(u32, bool)),
        entry!(0x0048_4580, tesform::set_empty(u32, bool)),
        entry!(0x0048_4610, tesform::set_flag_2000(u32, bool)),
        entry!(0x0048_4650, tesform::set_flag_800000(u32, bool)),
        entry!(0x0048_46a0, tesform::set_destructible(u32, bool)),
        entry!(0x0048_46e0, tesform::set_flag_4000000(u32, bool)),
        entry!(0x0048_4730, tesform::set_listed_flag(u32, bool)),
        entry!(0x0048_4860, tesform::set_flag_400(u32, bool)),
        entry!(0x0048_48b0, tesform::set_flag_40(u32, bool)),
        entry!(0x0048_4900, tesform::set_flag_10000(u32, bool)),
        entry!(0x0048_4950, tesform::set_flag_20000(u32, bool)),
        entry!(0x0048_4990, tesform::set_flag_100000(u32, bool)),
        entry!(0x0048_49d0, tesform::set_flag_200(u32, bool)),
        entry!(0x0048_4a10, tesform::set_flag_40_again(u32, bool)),
        entry!(0x0048_4a70, tesform::set_fire_off(u32, bool)),
        entry!(0x0048_4ab0, tesform::set_flag_8(u32, bool)),
        entry!(0x0048_4af0, tesform::set_disabled(u32, bool)),
        entry!(0x0048_4b40, tesform::id_in_low_range(u32) -> bool),
        entry!(0x0048_4b60, tesform::add_change(u32, u32)),
        entry!(0x0048_4b90, tesform::force_change(u32, u32)),
        entry!(0x0048_4bc0, tesform::change_sibling(u32, u32)),
        entry!(0x0048_4bf0, tesform::flags_save_size(u32, u32) -> u16),
        entry!(0x0048_4c20, tesform::save_flags(u32, u32)),
        entry!(0x0048_4c50, tesform::load_game(u32, u32, u32)),
        entry!(0x0048_4ce0, tesform::save_game_data_old(u32, u32, u32)),
        entry!(0x0048_4d00, tesform::load_game_data_old(u32, u32, u32)),
        entry!(0x0048_4d20, tesform::save_numeric_id(u32, u32, u32)),
        entry!(0x0048_4d40, tesform::load_numeric_id(u32, u32, u32)),
        entry!(0x0048_4d60, tesform::write_flags(u32, u32)),
        entry!(0x0048_4da0, tesform::read_flags(u32, u32)),
        entry!(0x0048_4e40, tesform::has_low_id(u32) -> bool),
        entry!(0x0048_4e60, tesform::get_file(u32, i32) -> u32),
        entry!(0x0048_4ee0, tesform::get_owner_master(u32) -> u32),
        entry!(0x0048_4f50, tesform::set_file(u32, u32)),
        entry!(0x0048_5110, tesform::load_form(u32, u32)),
        entry!(0x0048_5130, tesform::copy(u32, u32)),
        entry!(0x0048_5170, tesform::compare(u32, u32) -> bool),
        entry!(0x0048_51b0, tesform::copy_all_components(u32, u32)),
        entry!(0x0048_5270, tesform::compare_all_components(u32, u32) -> u8),
        entry!(0x0048_5340, tesform::copy_from(u32, u32)),
        entry!(0x0048_53e0, tesform::differs_from(u32, u32) -> bool),
        entry!(0x0048_54e0, tesform::is_own_top_group(u32, u32, u32, u32) -> bool),
        entry!(0x0048_5530, tesform::fill_top_group(u32, u32, u32)),
        entry!(0x0048_55a0, tesform::start_form(u32)),
        entry!(0x0048_5680, tesform::close_form(u32)),
        entry!(0x0048_56d0, tesform::add_chunk(u32)),
        entry!(0x0048_56f0, tesform::add_chunk_array(u32, u32, u32)),
        entry!(0x0048_5710, tesform::add_chunk_array_words(u32, u32, u32)),
        entry!(0x0048_5730, tesform::add_chunk_array_halves(u32, u32, u32)),
        entry!(0x0048_5750, tesform::add_chunk_array32(u32, u32, u32)),
        entry!(0x0048_5820, tesform::add_chunk_array16(u32, u32, u32)),
        entry!(0x0048_58f0, tesform::add_chunk_byte(u32, u32)),
        entry!(0x0048_5910, tesform::add_chunk_word(u32, u32)),
        entry!(0x0048_5950, tesform::add_chunk_half(u32, u32)),
        entry!(0x0048_5990, tesform::add_chunk_data(u32, u32, u32)),
        entry!(0x0048_5a70, tesform::grow_chunk(u32, u32, u16) -> bool),
        entry!(0x0048_5b30, tesform::free_form_buffer(u32)),
        entry!(0x0048_5b60, tesform::packed_id(u32) -> u32),
        entry!(0x0048_5bc0, tesform::low_id(u32) -> u32),
        entry!(0x0048_5be0, tesform::same_low_id(u32, u32) -> bool),
        entry!(0x0048_5c10, tesform::set_form_id(u32, u32, bool)),
        entry!(0x0048_5d50, tesform::add_compile_index(u32, u32)),
        entry!(0x0048_5e00, tesform::save_data(u32)),
        entry!(0x0048_5e20, tesform::save_data_chunk(u32, u32, u32)),
        entry!(0x0048_61f0, tesform::load_data(u32, u32, u32, u16)),
        entry!(0x0048_65f0, tesform::smaller_half(u16, u16) -> u16),
        entry!(0x0048_6620, tesform::components_data_size(u32) -> u32),
        entry!(0x0048_67a0, tesform::copy_form(u32, u32, u32) -> u32),
        entry!(0x0048_6890, tesform::form_type_from_string(u32) -> u32),
        entry!(0x0048_68f0, tesform::detailed_string(u32, u32)),
        entry!(0x0048_6930, tesform::form_map_construct(u32, u32) -> u32),
        entry!(0x0048_6960, tesform::form_map_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0048_6990, tesform::map_base_construct(u32, u32) -> u32),
        entry!(0x0048_6a00, tesform::form_map_destructor(u32)),
        entry!(0x0048_6c00, tesform::map_base_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0048_6c30, tesform::string_map_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0048_6c60, tesform::string_map_destructor(u32)),
        entry!(0x0048_6e50, tesform::char_key_map_base_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0048_6e80, tesform::char_key_map_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0048_6eb0, tesform::string_template_map_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0047_c700, actor_base_data::construct(u32) -> u32),
        entry!(0x0047_c770, actor_base_data::no_vats_melee(u32) -> bool),
        entry!(0x0047_c790, actor_base_data::allow_pc_dialogue(u32) -> bool),
        entry!(0x0047_c7b0, actor_base_data::allow_pick_pocket(u32) -> bool),
        entry!(0x0047_c7d0, actor_base_data::is_ghost(u32) -> bool),
        entry!(0x0047_c7f0, actor_base_data::invulnerable(u32) -> bool),
        entry!(0x0047_c810, actor_base_data::cant_open_doors(u32) -> bool),
        entry!(0x0047_c830, actor_base_data::auto_calc_service_flags(u32) -> bool),
        entry!(0x0047_c860, actor_base_data::karma(u32) -> f32),
        entry!(0x0047_c880, actor_base_data::destructor(u32)),
        entry!(0x0047_c8e0, actor_base_data::initialize_data_component(u32)),
        entry!(0x0047_c9b0, actor_base_data::clear_data_component(u32)),
        entry!(0x0047_c9d0, actor_base_data::clear_faction_list(u32)),
        entry!(0x0047_ca20, actor_base_data::copy_faction_list(u32, u32)),
        entry!(0x0047_cac0, actor_base_data::save(u32)),
        entry!(0x0047_cb90, actor_base_data::swap_data(u32)),
        entry!(0x0047_cca0, actor_base_data::template_use_flag_15(u32) -> bool),
        entry!(0x0047_ccc0, actor_base_data::set_template_use_flag_15(u32, u8)),
        entry!(0x0047_cce0, actor_base_data::set_template_use_flag(u32, i32, u8)),
        entry!(0x0047_cd40, actor_base_data::template_use_flag(u32, i32) -> bool),
        entry!(0x0047_cd90, actor_base_data::set_template_use_flags(u32, u16)),
        entry!(0x0047_cdb0, actor_base_data::has_actor_template_again(u32) -> bool),
        entry!(0x0047_cdd0, actor_base_data::has_actor_template(u32) -> bool),
        entry!(0x0047_ce10, actor_base_data::apply_template(u32, u32)),
        entry!(0x0047_d0b0, actor_base_data::path_manager_instance() -> u32),
        entry!(0x0047_d130, actor_base_data::create_template_actor_base(u32, u32) -> u32),
        entry!(0x0047_d1a0, actor_base_data::template_chain_mask(u32, u32) -> u32),
        entry!(0x0047_d220, actor_base_data::copy_component(u32, u32)),
        entry!(0x0047_d370, actor_base_data::level(u32) -> u16),
        entry!(0x0047_d390, actor_base_data::calc_level_min(u32) -> u16),
        entry!(0x0047_d3b0, actor_base_data::calc_level_max(u32) -> u16),
        entry!(0x0047_d3d0, actor_base_data::base_disposition(u32) -> u16),
        entry!(0x0047_d3f0, actor_base_data::bartergold(u32) -> u16),
        entry!(0x0047_d410, actor_base_data::compare_component(u32, u32) -> bool),
        entry!(0x0047_d680, actor_base_data::faction_rank(u32, u32, u8) -> i32),
        entry!(0x0047_d720, actor_base_data::faction_flag_8(u32) -> bool),
        entry!(0x0047_d740, actor_base_data::in_evil_factions_only(u32) -> bool),
        entry!(0x0047_d7c0, actor_base_data::faction_flag_2(u32) -> bool),
        entry!(0x0047_d7e0, actor_base_data::faction_flag_4(u32) -> bool),
        entry!(0x0047_d800, actor_base_data::set_faction_rank(u32, u32, i8)),
        entry!(0x0047_d8b0, actor_base_data::init_item(u32, u32)),
        entry!(0x0047_dd30, actor_base_data::set_base_flags(u32, u32)),
        entry!(0x0047_dd50, actor_base_data::set_flag_bit(u32, u32, u8, u8)),
        entry!(0x0047_de40, actor_base_data::set_calc_level_min(u32, u16)),
        entry!(0x0047_de70, actor_base_data::set_calc_level_max(u32, u16)),
        entry!(0x0047_dea0, actor_base_data::set_base_disposition(u32, u16)),
        entry!(0x0047_ded0, actor_base_data::calculated_level(u32) -> u16),
        entry!(0x0047_dfe0, actor_base_data::set_level(u32, u16)),
        entry!(0x0047_e010, actor_base_data::set_fatigue(u32, u16)),
        entry!(0x0047_e040, actor_base_data::alignment_for_karma(f32) -> i32),
        entry!(0x0047_e0e0, actor_base_data::karmic_title(u32) -> u32),
        entry!(0x0047_e1c0, actor_base_data::set_karma(u32, f32)),
        entry!(0x0047_e240, actor_base_data::set_bartergold(u32, u16)),
        entry!(0x0047_e270, actor_base_data::set_speed_mult(u32, u16)),
        entry!(0x0047_e2a0, actor_base_data::mark_changed(u32, u32)),
        entry!(0x0047_e2f0, actor_base_data::mark_unchanged(u32, u32)),
        entry!(0x0047_e340, actor_base_data::set_change_flags(u32, u32)),
        entry!(0x0047_e360, actor_base_data::clear_change_flags(u32, u32)),
        entry!(0x0047_e380, actor_base_data::save_size(u32, u32) -> u16),
        entry!(0x0047_e4a0, actor_base_data::save_game(u32, u32)),
        entry!(0x0047_e6f0, actor_base_data::load_game(u32, u32, u32)),
        entry!(0x004a_8ae0, actor_base_data::fatigue(u32) -> u16),
        entry!(0x0047_ffc0, attributes::initialize_data_component(u32)),
        entry!(0x0048_0000, attributes::copy_component(u32, u32)),
        entry!(0x0048_0070, attributes::compare_component(u32, u32) -> bool),
        entry!(0x0048_0100, attributes::save_data(u32, u32) -> u32),
        entry!(0x0048_0130, attributes::load_data(u32, u32) -> u32),
        entry!(0x0048_0160, attributes::attribute(u32, u32) -> u32),
        entry!(0x0048_0180, attributes::set_attribute_value(u32, u32, u8, u8)),
        entry!(0x0048_01e0, attributes::save_size(u32, u32) -> u16),
        entry!(0x0048_0210, attributes::save_game(u32, u32)),
        entry!(0x0048_0240, attributes::load_game(u32, u32, u32)),
        entry!(0x0048_0270, attributes::save_to_buffer(u32, u32)),
        entry!(0x0048_02b0, attributes::load_from_buffer(u32, u32)),
        entry!(0x0048_02f0, attributes::construct_biped_model_form(u32) -> u32),
        entry!(0x0048_03e0, attributes::construct_model_rdt_at(u32) -> u32),
        entry!(0x0048_0400, attributes::destruct_biped_model_form(u32)),
        entry!(0x0056_5210, actor_base::set_starts_dead(u32, u8)),
        entry!(0x005f_0b00, actor_base::get_health(u32) -> u32),
        entry!(0x005f_0b30, actor_base::set_health(u32, u32)),
        entry!(0x005f_0b50, actor_base::creature_flag_10_or_1(u32) -> bool),
        entry!(0x005f_0ba0, actor_base::creature_flags_allow(u32) -> bool),
        entry!(0x005f_0bf0, actor_base::creature_flag_40_or_1(u32) -> bool),
        entry!(0x005f_0c40, actor_base::creature_flag_20(u32) -> bool),
        entry!(0x005f_0c80, actor_base::is_immobile(u32) -> bool),
        entry!(0x005f_0cc0, actor_base::get_sex(u32) -> i32),
        entry!(0x005f_0d00, actor_base::npc_flag_10(u32) -> bool),
        entry!(0x005f_0d30, actor_base::set_npc_flag_10(u32, u8)),
        entry!(0x005f_0d60, actor_base::get_desirability(u32, u32) -> f32),
        entry!(0x005f_0f50, actor_base::starts_dead(u32) -> bool),
        entry!(0x005f_0f80, actor_base::player_or_default_value(u32) -> u32),
        entry!(0x005f_0fb0, actor_base::get_actor_value(u32, u32) -> u32),
        entry!(0x005f_11f0, actor_base::attribute_5(u32) -> u32),
        entry!(0x005f_1210, actor_base::attribute_6(u32) -> u32),
        entry!(0x005f_1230, actor_base::attribute_7(u32) -> u32),
        entry!(0x005f_1250, actor_base::attribute_8(u32) -> u32),
        entry!(0x005f_1270, actor_base::attribute_9(u32) -> u32),
        entry!(0x005f_1290, actor_base::attribute_a(u32) -> u32),
        entry!(0x005f_12b0, actor_base::attribute_b(u32) -> u32),
        entry!(0x005f_12d0, actor_base::set_actor_value(u32, u32, u32)),
        entry!(0x005f_14b0, actor_base::set_attribute_5(u32, u8)),
        entry!(0x005f_14d0, actor_base::set_attribute_6(u32, u8)),
        entry!(0x005f_14f0, actor_base::set_attribute_7(u32, u8)),
        entry!(0x005f_1510, actor_base::set_attribute_8(u32, u8)),
        entry!(0x005f_1530, actor_base::set_attribute_9(u32, u8)),
        entry!(0x005f_1550, actor_base::set_attribute_a(u32, u8)),
        entry!(0x005f_1570, actor_base::set_attribute_b(u32, u8)),
        entry!(0x005f_1590, actor_base::should_run_init_default_worn(u32, u32) -> bool),
        entry!(0x005f_1630, actor_base::add_change(u32, u32)),
        entry!(0x005f_1690, actor_base::remove_change(u32, u32)),
        entry!(0x005f_16f0, actor_base::save_size(u32, u32) -> u16),
        entry!(0x005f_18c0, actor_base::save_game(u32, u32)),
        entry!(0x005f_1b30, actor_base::load_game(u32, u32, u32)),
        entry!(0x005f_1f30, actor_base::save_to_buffer(u32, u32)),
        entry!(0x005f_1fd0, actor_base::load_from_buffer(u32, u32)),
        entry!(0x005f_20a0, actor_base::reset_3d(u32, u8)),
        entry!(0x005f_2150, actor_base::construct_reset_3d_iterator(u32) -> u32),
        entry!(0x005f_2170, actor_base::reset_3d_operate(u32, u32)),
        entry!(0x0047_ede0, ai_form::clear_data_component(u32)),
        entry!(0x0047_ee00, ai_form::set_aggression(u32, u8, u8)),
        entry!(0x0047_eed0, ai_form::set_confidence(u32, u8, u8)),
        entry!(0x0047_ef00, ai_form::set_energy(u32, u8, u8)),
        entry!(0x0047_ef50, ai_form::set_responsibility(u32, u8, u8)),
        entry!(0x0047_f060, ai_form::is_skill(i32) -> bool),
        entry!(0x0047_ee30, ai_form::assistance(u32) -> u8),
        entry!(0x0047_ef30, ai_form::responsibility(u32) -> u8),
        entry!(0x0047_f790, ai_form::mark_changed(u32, u32)),
        entry!(0x0047_ee50, ai_form::set_assistance(u32, u8)),
        entry!(0x0047_ee70, ai_form::set_aggro_radius_flag(u32, u8)),
        entry!(0x0047_ee90, ai_form::set_aggro_radius(u32, u32)),
        entry!(0x0047_ef80, ai_form::set_mood(u32, u8)),
        entry!(0x0047_efb0, ai_form::set_service_flags(u32, u32, u8)),
        entry!(0x0047_f000, ai_form::training_skill(u32) -> u32),
        entry!(0x0047_f020, ai_form::set_training_skill(u32, u32)),
        entry!(0x0047_f090, ai_form::set_training_level(u32, u8)),
        entry!(0x0047_ed10, ai_form::initialize_data_component(u32)),
        entry!(0x0047_ec40, ai_form::construct(u32) -> u32),
        entry!(0x0047_ecb0, ai_form::destruct(u32)),
        entry!(0x0047_f0b0, ai_form::load_data(u32, u32)),
        entry!(0x0047_f1e0, ai_form::copy_ai_data(u32, u32)),
        entry!(0x0047_f2d0, ai_form::copy_packages(u32, u32)),
        entry!(0x0047_f190, ai_form::copy_component(u32, u32)),
        entry!(0x0047_f500, ai_form::add_package(u32, u32)),
        entry!(0x0047_f4c0, ai_form::init_item(u32, u32)),
        entry!(0x0047_f520, ai_form::current_package(u32) -> u32),
        entry!(0x0047_f590, ai_form::missed_packages(u32, u32, f32, f32)),
        entry!(0x0047_eeb0, ai_form::copied_setting(u8) -> f32),
        entry!(0x0047_efe0, ai_form::has_service(u32, u32) -> bool),
        entry!(0x0047_f300, ai_form::compare_component(u32, u32) -> bool),
        entry!(0x0047_f600, ai_form::sells_buys_item(u32, u32) -> bool),
        entry!(0x0047_f7e0, ai_form::save_size(u32, u32) -> u16),
        entry!(0x0047_f840, ai_form::save_game(u32, u32)),
        entry!(0x0047_f8c0, ai_form::load_game(u32, u32, u32)),
        entry!(0x0047_f940, ai_form::save_to_buffer(u32, u32)),
        entry!(0x0047_f980, ai_form::load_from_buffer(u32, u32)),
        entry!(0x0047_f9c0, animation::construct(u32) -> u32),
        entry!(0x0047_f9f0, animation::destruct(u32)),
        entry!(0x0047_fa50, animation::clear_data_component(u32)),
        entry!(0x0047_faa0, animation::load_names(u32, u32, u32)),
        entry!(0x0047_fb50, animation::copy_component(u32, u32)),
        entry!(0x0047_fbd0, animation::compare_component(u32, u32) -> bool),
        entry!(0x0047_fcb0, animation::has_name(u32, u32) -> bool),
        entry!(0x0047_fd20, animation::add_name(u32, u32)),
        entry!(0x0044_a4c0, debug_string::construct_from_text(u32, u32) -> u32),
        entry!(0x0044_a500, debug_string::copy_construct(u32, u32) -> u32),
        entry!(0x0044_a540, debug_string::destruct(u32)),
        entry!(0x0044_a560, debug_string::c_str(u32) -> u32),
        entry!(0x0044_a580, debug_string::assign_range(u32, u32, u32, u32) -> u32),
        entry!(0x0044_a640, debug_string::assign_text(u32, u32) -> u32),
        entry!(0x0044_a670, debug_string::text_length(u32) -> u32),
        entry!(0x0044_a690, debug_string::erase(u32, u32, u32) -> u32),
        entry!(0x0044_a730, debug_string::tidy(u32, bool, u32)),
        entry!(0x0044_a7b0, debug_string::construct_proxy(u32, u32) -> u32),
        entry!(0x0044_a7d0, debug_string::construct_proxy_from(u32, u32) -> u32),
        entry!(0x0044_a7f0, debug_string::assign_chars(u32, u32, u32) -> u32),
        entry!(0x0044_a880, debug_string::end_string_at(u32, u32)),
        entry!(0x0044_a8c0, debug_string::assign_char(u32, u32)),
        entry!(0x0044_a8d0, debug_string::grow_storage(u32, u32, bool) -> bool),
        entry!(0x0044_a970, debug_string::string_characters(u32) -> u32),
        entry!(0x0044_a9d0, debug_string::string_max_size(u32) -> u32),
        entry!(0x0044_aa10, debug_string::copy_to_new_storage(u32, u32, u32)),
        entry!(0x0044_ab90, debug_string::string_holds(u32, u32) -> bool),
        entry!(0x0044_a2e0, std_exceptions::length_error_construct(u32, u32) -> u32),
        entry!(0x0044_a310, std_exceptions::logic_error_construct(u32, u32) -> u32),
        entry!(0x0044_a380, std_exceptions::logic_error_what(u32) -> u32),
        entry!(0x0044_a3a0, std_exceptions::logic_error_deleting_destructor(u32, u32) -> u32),
        entry!(0x0044_a3d0, std_exceptions::logic_error_destruct(u32)),
        entry!(0x0044_a400, std_exceptions::length_error_destruct(u32)),
        entry!(0x0044_a420, std_exceptions::length_error_deleting_destructor(u32, u32) -> u32),
        entry!(0x0044_a450, std_exceptions::logic_error_copy(u32, u32) -> u32),
        entry!(0x0044_a9a0, std_exceptions::length_error_copy(u32, u32) -> u32),
        entry!(0x0044_dd10, std_exceptions::bad_alloc_copy(u32, u32) -> u32),
        entry!(0x0044_9fd0, word_vector::word_vector_construct(u32) -> u32),
        entry!(0x0044_a040, word_vector::word_vector_destruct(u32)),
        entry!(0x0044_a0a0, word_vector::word_vector_clear(u32) -> u32),
        entry!(0x0044_a170, word_vector::word_vector_begin(u32, u32) -> u32),
        entry!(0x0044_a1a0, word_vector::word_vector_erase(u32, u32, u32, u32, u32, u32) -> u32),
        entry!(0x0044_a240, word_vector::word_vector_buy(u32, u32) -> bool),
        entry!(0x0044_a2c0, word_vector::word_vector_max_size(u32) -> u32),
        entry!(0x0044_abd0, word_vector::word_allocator_allocate(u32, u32) -> u32),
        entry!(0x0044_abf0, word_vector::word_allocator_max_size(u32) -> u32),
        entry!(0x0044_ac20, word_vector::copy_one_word(u32, u32)),
        entry!(0x0044_d8c0, queued_maps::queued_reference_map_delete(u32, u32) -> u32),
        entry!(0x0044_d980, queued_maps::anim_idle_map_delete(u32, u32) -> u32),
        entry!(0x0044_da40, queued_maps::replacement_kf_list_map_delete(u32, u32) -> u32),
        entry!(0x0044_db00, queued_maps::helmet_map_delete(u32, u32) -> u32),
        entry!(0x0044_dbc0, queued_maps::model_map_delete(u32, u32) -> u32),
        entry!(0x0044_dbf0, queued_maps::kf_model_map_delete(u32, u32) -> u32),
        entry!(0x0044_dc20, queued_maps::texture_map_delete(u32, u32) -> u32),
        entry!(0x0044_dce0, queued_maps::loaded_file_map_delete(u32, u32) -> u32),
        entry!(0x0044_dd60, queued_maps::release_task(u32)),
        entry!(0x0044_de20, queued_maps::iterator_flag_set(u32) -> bool),
        entry!(0x0050_e820, object::bound_object_construct(u32) -> u32),
        entry!(0x0050_e8b0, object::copy_box_point(u32, u32) -> u32),
        entry!(0x0050_e8f0, object::default_bound() -> u32),
        entry!(0x0050_e960, object::set_point(u32, u32, u32, u32) -> u32),
        entry!(0x0050_e990, object::bound_object_deleting_destructor(u32, u32) -> u32),
        entry!(0x0050_e9c0, object::bound_data_construct(u32) -> u32),
        entry!(0x0050_e9f0, object::bound_object_destruct(u32)),
        entry!(0x0050_ea10, object::points_difference(u32, u32)),
        entry!(0x0050_ea50, object::points_sum(u32, u32)),
        entry!(0x0050_ea90, object::bound_extra_data_name() -> u32),
        entry!(0x0050_eaa0, object::copy_bound(u32, u32) -> u32),
        entry!(0x0050_ead0, object::bound_min_as_floats(u32, u32) -> u32),
        entry!(0x0050_eb30, object::point_x(u32) -> f32),
        entry!(0x0050_eb50, object::point_y(u32) -> f32),
        entry!(0x0050_eb70, object::point_z(u32) -> f32),
        entry!(0x0050_eb90, object::bound_max_as_floats(u32, u32) -> u32),
        entry!(0x0050_ebf0, object::bound_size(u32) -> f32),
        entry!(0x0050_ec30, object::point_difference(u32, u32, u32) -> u32),
        entry!(0x0050_ec80, object::point_length(u32) -> f64),
        entry!(0x0050_ece0, object::save_bound(u32)),
        entry!(0x0050_ed30, object::swap_bound(u32)),
        entry!(0x0050_edd0, object::load_bound(u32, u32)),
        entry!(0x0050_ee20, object::clone_3d_unflagged(u32, u32) -> u32),
        entry!(0x0050_ee50, object::bound_object_clone_3d(u32, u32, u8) -> u32),
        entry!(0x0050_f5a0, object::clear_parent_list(u32)),
        entry!(0x0050_f5c0, object::clone_controller_word() -> u32),
        entry!(0x0050_f5d0, object::model_path_slot(u32) -> u32),
        entry!(0x0050_f600, object::register_model(u32, u32) -> bool),
        entry!(0x0050_f670, object::cast_and_call(u32, u32, u32, u8, u32, u32) -> bool),
        entry!(0x0050_f6e0, object::file_finder() -> u32),
        entry!(0x0050_f6f0, object::bound_anim_object_construct(u32) -> u32),
        entry!(0x0050_f710, object::bound_anim_object_deleting_destructor(u32, u32) -> u32),
        entry!(0x0050_f740, object::bound_anim_object_destruct(u32)),
        entry!(0x0050_f760, object::register_model_again(u32, u32) -> bool),
        entry!(0x0050_f780, object::forward_to_empty(u32, u32, u8)),
        entry!(0x0050_f7a0, object::bound_tree_object_construct(u32) -> u32),
        entry!(0x0050_f7c0, object::bound_tree_object_deleting_destructor(u32, u32) -> u32),
        entry!(0x0050_f7f0, object::bound_tree_object_destruct(u32)),
        entry!(0x0050_f810, object::type_25_result(u32, u32, u32) -> u32),
        entry!(0x0050_f8c0, object::object_construct(u32) -> u32),
        entry!(0x0050_f900, object::object_deleting_destructor(u32, u32) -> u32),
        entry!(0x0050_f930, object::object_destruct(u32)),
        entry!(0x0050_f9a0, object::set_previous(u32, u32)),
        entry!(0x0050_f9c0, object::set_word_20(u32, u32)),
        entry!(0x0050_fb90, object::is_named_type_20(u32) -> bool),
        entry!(0x0050_fbe0, object::zero(u32, u32, u32) -> u32),
        entry!(0x0050_fbf0, object::register_form_model(u32, u32)),
        entry!(0x0050_fd90, object::model_path(u32, u32) -> u32),
        entry!(0x0050_fdf0, object::scaled_model_file_name(u32, u32, u32)),
        entry!(0x0067_e3a0, package_list::destruct(u32)),
        entry!(0x0067_e400, package_list::free(u32)),
        entry!(0x0067_e420, package_list::add_tail(u32, u32)),
        entry!(0x0067_e440, package_list::copy(u32, u32)),
        entry!(0x0067_e4a0, package_list::lists_differ(u32, u32) -> bool),
        entry!(0x0067_e560, package_list::init_item(u32, u32)),
        entry!(0x0067_e700, package_list::in_schedule_window(i32, i32, f32) -> bool),
        entry!(0x0067_e780, package_list::find_current_package(u32, u32) -> u32),
        entry!(0x0067_ec40, package_list::missed_packages(u32, u32, u32, u32, f32)),
        entry!(0x0067_efd0, package_list::is_missable(u32) -> bool),
        entry!(0x0067_8a80, schedule::compare(u32, u32) -> i32),
        entry!(0x0067_8cc0, schedule::day_of_week(u32) -> i8),
        entry!(0x0067_8ce0, schedule::date(u32) -> u8),
        entry!(0x0067_fcb0, schedule::construct(u32) -> u32),
        entry!(0x0067_fcf0, schedule::assign(u32, u32) -> u32),
        entry!(0x0067_fd40, schedule::set_from(u32, u32)),
        entry!(0x0067_fdb0, schedule::set_day_of_week(u32, u8)),
        entry!(0x0067_fdd0, schedule::set_date(u32, u8)),
        entry!(0x0067_fdf0, schedule::set_hour(u32, u8)),
        entry!(0x0067_fe10, schedule::differs(u32, u32) -> bool),
        entry!(0x0067_fec0, schedule::save(u32)),
        entry!(0x0067_ff10, schedule::swap_three_words(u32)),
        entry!(0x0067_ad20, package_data::construct(u32) -> u32),
        entry!(0x0067_ad40, package_data::scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0067_ad70, package_data::destruct(u32)),
        entry!(0x0067_b050, package_data::dialogue_load_game(u32, u32)),
        entry!(0x0067_b100, package_data::dialogue_swap_endian(u32)),
        entry!(0x0067_b170, package_data::dialogue_construct(u32) -> u32),
        entry!(0x0067_b950, package_data::eat_construct(u32) -> u32),
        entry!(0x0067_bbf0, package_data::escort_construct(u32) -> u32),
        entry!(0x0067_bfd0, package_data::follow_construct(u32) -> u32),
        entry!(0x0067_c330, package_data::patrol_load_game(u32, u32)),
        entry!(0x0067_c3f0, package_data::patrol_construct(u32) -> u32),
        entry!(0x0067_c420, package_data::patrol_scalar_deleting_destructor(u32, u32) -> u32),
        entry!(0x0067_c450, package_data::patrol_construct_from(u32, u32) -> u32),
        entry!(0x0067_c490, package_data::patrol_destruct(u32)),
        entry!(0x0067_c770, package_data::use_item_construct(u32) -> u32),
        entry!(0x0067_ca10, package_data::use_weapon_swap_endian(u32)),
        entry!(0x0067_da20, package_data::clear_word_4(u32, u32)),
        entry!(0x0067_da40, package_data::event_action_construct(u32) -> u32),
        entry!(0x0067_da80, package_data::event_action_clear(u32)),
        entry!(0x0067_dad0, package_data::event_action_copy(u32, u32)),
        entry!(0x0067_eff0, package_data::location_swap_endian(u32)),
        entry!(0x0068_0890, conditions::construct(u32) -> u32),
        entry!(0x0068_08b0, conditions::destruct(u32)),
        entry!(0x0068_0fe0, conditions::clear(u32)),
        entry!(0x0068_1050, conditions::compare_values(u32, f32, f32) -> bool),
        entry!(0x0068_1110, conditions::swap_function_data(u32)),
    ]
}

#[cfg(test)]
mod tests {
    /// No member of the family is an address an extender or a fix row wraps: a wrap there would
    /// turn the family's direct calls off for the whole run. The seams inside its functions are
    /// rows of the seam tables, which do not.
    #[test]
    fn no_member_is_wrapped() {
        let members: Vec<u32> = super::funcs().iter().map(|(addr, _)| *addr).collect();
        for row in engine_core::extend::all_wraps() {
            assert!(
                !members.contains(&row.addr),
                "{:08x} is wrapped: keep it out of the family",
                row.addr
            );
        }
        for seam in [
            0x0048_3669,
            0x0048_449a,
            0x0048_4e03,
            0x0048_6903,
            0x0050_ef46,
        ] {
            assert!(engine_core::extend::all_sites().any(|row| row.addr == seam));
            assert!(!members.contains(&seam));
        }
    }
}
