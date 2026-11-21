//! The save family: the work a save does in its one frame, made native. A quicksave is one frame of
//! about 100 ms in the port (2026-11-20, the hitch log: 102-105 ms every time); most of it is the
//! body's walk of the changed forms, each form id it writes turned into a save index through the
//! form id map, and the forms' own save code (measure/smooth-200/SAVE-DESIGN.md).
//!
//! - [`refs`]: the form id map (`BGSSaveLoadFormIDMap`): an id's index, got or added (`00846c90`)
//!   and resolved back (`00846d20`); the word maps' `SetAt` (`00844700`) with the item functions
//!   their vtable holds; the three-byte index (`BGSNumericIDIndex`: `0084eaf0`, `00853500`,
//!   `00853570`) and the save buffer's two `SaveFormID`s (`00865db0`, `00865df0`), which every form
//!   reference a save writes goes through. The map's lookup (`00853130`) is the dispatch family's.
//!
//! - [`buffer`]: the save buffer every form's save writes its fields into (`BGSSaveGameBuffer`: the
//!   positioned write `00865ce0`, the append `00865570`, the growth `008660b0`, `SaveData`,
//!   `SaveString`, the variable-sized values and the room they make, `Save` and `Save_ov2`), the
//!   save file's write (`BGSSaveLoadFile::Write`, `00846330`) and the sizing test (`00424940`);
//! - [`maps`]: the walk over a word-keyed map (`004b9ba0`, `006b7f20`), which the save's body
//!   takes once per changed form.
//!
//! What stays by address: the allocator (`00401000`, `00401030`), the real file's write
//! (`00473180`), the warnings, and any map whose vtable holds other functions than the word map's
//! (called through the vtable, as the exe calls them).
//!
//! Stage 1: the objects stay in guest memory where the game made them; each function stays
//! registered at its exe address. **Threads.** The save runs on the main thread, but `SetAt` is the
//! one body of every word map's insert (the combat maps the AI threads fill too), so the row says
//! [`Threads::Both`]. There are no floats here: every function is integer and pointer work, exact.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod buffer;
pub mod maps;
pub mod refs;

/// This family's bit: what each `typed_call!` of it names (assigned by the lead, 2026-11-21).
pub const FAMILY: FamilyId = FamilyId::new(16);

/// `NV_TYPED=save`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static SAVE: Family = Family {
    name: "save",
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
        entry!(0x0046_9860, refs::is_created_id(Ptr, u32) -> bool),
        entry!(0x0084_4700, refs::set_at(Ptr, u32, u32)),
        entry!(0x0084_4a70, refs::new_item(Ptr) -> Ptr),
        entry!(0x006f_99e0, refs::allocate_item(Ptr) -> Ptr),
        entry!(0x0086_3840, refs::set_value(Ptr, Ptr, u32, u32)),
        entry!(0x0084_6c90, refs::get_or_add_index(Ptr, u32) -> u32),
        entry!(0x0084_6d20, refs::resolve_index(Ptr, u32) -> u32),
        entry!(0x0084_eaf0, refs::index_new(Ptr) -> Ptr),
        entry!(0x0085_3500, refs::get_numeric_id(Ptr) -> u32),
        entry!(0x0085_3570, refs::set_numeric_id(Ptr, u32)),
        entry!(0x0086_5db0, refs::save_numeric_form_id(Ptr, u32, u32)),
        entry!(0x0086_5df0, refs::save_form_id(Ptr, Ptr, u32)),
        entry!(0x0042_4940, buffer::byte_c(Ptr) -> u8),
        entry!(0x0086_60b0, buffer::grow(Ptr)),
        entry!(0x0086_5ce0, buffer::write_at(Ptr, Ptr, u32, u32, u8)),
        entry!(0x0086_5570, buffer::append(Ptr, Ptr, u32, u8)),
        entry!(0x0086_5e50, buffer::save_bytes(Ptr, Ptr, u32, u32)),
        entry!(0x0086_5f60, buffer::save_variable_sized_value(Ptr, u32)),
        entry!(0x0086_5e70, buffer::save_string(Ptr, Ptr, u32)),
        entry!(0x0086_5f20, buffer::start_variable_value(Ptr) -> u32),
        entry!(0x0086_6140, buffer::insert_space(Ptr, u32, u32)),
        entry!(0x0086_5ff0, buffer::finish_variable_value(Ptr, u32, u32)),
        entry!(0x0086_5c80, buffer::save_part(Ptr, Ptr, u32)),
        entry!(0x0086_5c40, buffer::save(Ptr, Ptr)),
        entry!(0x0084_6330, buffer::file_write(Ptr, u32, u32) -> u32),
        entry!(0x004b_9ba0, maps::first_position(Ptr) -> u32),
        entry!(0x006b_7f20, maps::next(Ptr, Ptr, Ptr, Ptr)),
    ]
}
