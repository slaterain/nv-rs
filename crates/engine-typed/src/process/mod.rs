//! The process family (the update half's fifth group, stage 1): the process lists' passes over the
//! low, middle-low and middle-high process actors that `ProcessLists::UpdateProcessLists` runs on the
//! main thread, and the process array they walk. In the crowded Goodsprings street the low-process
//! pass (`0096b050`) runs on about one frame in three and walks about 2,000 actors each time, and
//! `ProcessArray::RemoveActor` searches the array from a level's start for each actor that leaves a
//! level (about 14 a frame, over 1,000 elements each): by the profile of 2026-11-11 a spike of about
//! 1 ms (quiet machine) on the frames the pass runs.
//!
//! - [`array`]: the process array (`ProcessArray`): the levels' starts and ends, the element
//!   store, the search (a plain loop over the element block), the removal and
//!   the cursors' correction;
//! - [`levels`]: the three level passes (`0096b050`, `0096b470`, `0096b810`) with the hour test,
//!   the actor update and the removal of the actors that left a level.
//!
//! What stays by address: the actor's own update (its vtable slot `0x25c`), its script
//! (`TESObjectREFR::RunScript`), `LowProcess::ComputeLastTimeProcessed`, the process type,
//! `Calendar::GetHour`, the actor list container, the process objects' slots and the clock import.
//!
//! Stage 1: the objects stay in guest memory where the game made them; each function stays
//! registered at its exe address. **Floats.** Only hours, on the main thread
//! ([`Threads::Main`]: `UpdateProcessLists` runs from the main loop only): the hours read as
//! floats, their difference one `f32` subtraction (the main thread's 24-bit precision), compared
//! with the exe's double constants exactly.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod array;
pub mod levels;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(28);

/// `NV_TYPED=process`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static PROCESS: Family = Family {
    name: "process",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Main,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x005d_e0f0, array::level_head(Ptr, u32) -> u32),
        entry!(0x005b_e5c0, array::level_tail(Ptr, u32) -> u32),
        entry!(0x0096_ae90, array::set_element(Ptr, u32, Ptr)),
        entry!(0x0096_ab20, array::level_of(Ptr, u32) -> i32),
        entry!(0x0096_abc0, array::find(Ptr, u32, i32) -> u32),
        entry!(0x0096_aa40, array::remove_at(Ptr, u32)),
        entry!(0x0096_ac30, array::correct_cursors(Ptr)),
        entry!(0x0096_aaf0, array::remove_actor(Ptr, u32, i32)),
        entry!(0x007d_f1f0, levels::hour_last_processed(Ptr) -> f32),
        entry!(0x0096_b050, levels::update_low(Ptr, f32, u32)),
        entry!(0x0096_b470, levels::update_middle_low(Ptr, f32, u32)),
        entry!(0x0096_b810, levels::update_middle_high(Ptr, f32, u32)),
    ]
}
