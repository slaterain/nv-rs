//! The interface family: the game's menus, its Pip-Boy, the HUD's menu side and the interface
//! manager's static wrappers (`fallout/interface`, the translations in `engine-fallout-interface`),
//! typed file by file. Each typed function does what its translation does, read for read, write for
//! write and call for call; the family's functions call one another through `typed_call!` (Rust calls
//! while the family is direct), and every other function by address as before (a function a fix or a
//! built-in may replace, `overridable.tsv`, always by address).
//!
//! - [`interface_cpp`] and [`interface_cpp_p3`]: the interface unit's functions from
//!   the start up to `00705a90` and from `00709b50` on: the static `Interface::` wrappers over the interface
//!   manager (menu ids on the stack, the Pip-Boy's state, the console, the faders, the HUD and map
//!   menu forwards), the camera frustum set-up and a `BSSimpleArray<Tile::Value *>`'s constructor
//!   and destructors.
//!
//! **Floats.** The menus run on the main thread ([`Threads::Main`]); the few float steps here are
//! the translations' own (stores of integers as floats, the camera frustum's products), kept as they
//! are.

use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::AbiFn;

pub mod interface_cpp;
pub mod interface_cpp_p3;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(44);

/// `NV_TYPED=interface`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static INTERFACE: Family = Family {
    name: "interface",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Main,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address: each file's list.
fn funcs() -> Vec<(u32, AbiFn)> {
    let mut all = interface_cpp::funcs();
    all.extend(interface_cpp_p3::funcs());
    all
}

/// A double result as the float the exe stores (the translations' `as f32`).
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn narrow(value: f64) -> f32 {
    value as f32
}

/// A count of words as the guest's 32-bit word (counts here are a few words).
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn word(count: usize) -> u32 {
    count as u32
}
