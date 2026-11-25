//! The archives family: the archive manager's question "which listed archive owns this file
//! entry?", asked for every file the game opens from a BSA (models, textures, terrain chunks,
//! sounds). The answer comes from a walk over every listed archive and, in each, over every
//! directory's block of 16-byte file entries until one holds the entry's address. On the
//! crowded Goodsprings street that walk made about 16,000 calls by address a frame on main
//! (`NV_CALL_COUNT`, 2026-11-23): one per directory asked.
//!
//! - [`lookup`]: the walk (`00af6910`), an archive's test of its directories (`00afdc00`) and a
//!   directory's test of its file entries (`00afdbc0`).
//!
//! Reached: `00af6910` by address from the two `GetFileIndex` virtuals (`0043bfe0`, `006fc440`),
//! the model loader and the distant terrain chunks, and from `00af6340` (`GetFile`'s step);
//! `00afdc00` from the walk only (the typed walk calls it directly); `00afdbc0` from `00afdc00`
//! only. Each stays registered at its exe address.
//!
//! Stage 1: the archives stay in guest memory where the game made them. **Integers only**, so the
//! row is [`Threads::Both`].

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod lookup;

/// This family's bit: what each `typed_call!` of it names (32, the first id of the widened flag
/// word; the ids below it are taken or held).
pub const FAMILY: FamilyId = FamilyId::new(32);

/// `NV_TYPED=archives`. A plain native conversion proved by its differential tests (real archive
/// directory tables included), so on unless `NV_TYPED` says otherwise.
pub static ARCHIVES: Family = Family {
    name: "archives",
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
        entry!(0x00af_dbc0, lookup::directory_holds(u32, u32) -> bool),
        entry!(0x00af_dc00, lookup::archive_holds(Ptr, u32) -> bool),
        entry!(0x00af_6910, lookup::owner_of(Ptr, u32) -> u32),
    ]
}
