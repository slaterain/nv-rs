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
//!
//! Reached: each function stays registered at its exe address, so the translations' calls by
//! address and the vtable slots that hold them (`TESForm`'s vtable `0101c794` and the form classes
//! that keep its slots) reach the typed functions; between them the calls are typed calls. The
//! translations that called one of them as a plain Rust function (all in `tesform.rs` itself)
//! call it by address now, so the game reaches the typed function from there too.
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
//! removal (`0048449a`) and at `GetFormDetailedString`'s name (`00486903`), and JIP LN's G45 mask
//! in the flags' load (`00484e03`, under its fix bit). A seam is a check inside the function, not a
//! registration over it, so it costs the family nothing; no extender or fix row registers over any
//! of this family's addresses (`GetFormByEditorID`, which JohnnyGuitar NVSE replaces, stays out),
//! so the family's calls stay direct in play.
//! Plugins' load order and form ids are untouched: the ids, the files' order in a form's list and
//! the compile index are computed exactly as the translation computes them.
//!
//! Stage 1: the objects stay in guest memory where the game made them. The loading threads read and
//! write forms too, so the row is [`Threads::Both`]. No float maths: the components' weight and
//! quality are only moved (a getter's `ST0` stored as the `f32` the exe stores, a quality byte
//! widened exactly), and the loaded quality becomes its byte through `x87::fistp_chop`, as the
//! exe's `FISTP` does under the truncating control word it sets there.

use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod tesform;

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
        for seam in [0x0048_3669, 0x0048_449a, 0x0048_4e03, 0x0048_6903] {
            assert!(engine_core::extend::all_sites().any(|row| row.addr == seam));
            assert!(!members.contains(&seam));
        }
    }
}
