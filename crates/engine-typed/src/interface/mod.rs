//! The interface family: the game's menus, its Pip-Boy, the HUD's menu side and the interface
//! manager's static wrappers (`fallout/interface`, the translations in `engine-fallout-interface`),
//! native file by file. Each function does the exe's work in the exe's order: the calls it makes to
//! functions no family types stay calls by address (a function a fix or a built-in may replace,
//! `overridable.tsv`, always by address), the family's functions and the other families' typed ones
//! are called through `typed_call!`, and what the exe calls but is a plain read or store of memory
//! (a getter, a setter, the manager singleton, the menu by class) is done in place.
//!
//! - [`interface_cpp`] and [`interface_cpp_p3`]: the interface unit's functions from
//!   the start up to `00705a90` and from `00709b50` on: the static `Interface::` wrappers over the interface
//!   manager (menu ids on the stack, the Pip-Boy's state, the console, the faders, the HUD and map
//!   menu forwards), the camera frustum set-up and a `BSSimpleArray<Tile::Value *>`'s constructor
//!   and destructors.
//!
//! **Floats.** The menus run on the main thread ([`Threads::Main`]), at the float precision the
//! exe's main thread computes in once Direct3D set it: plain `f32` (the camera frustum's products;
//! a product by one of the exe's double factors is exact in an `f64` and rounded once to the float).
//! The rest are stores of integers as floats.

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

/// A double result as the float the exe stores.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn narrow(value: f64) -> f32 {
    value as f32
}

/// A count of words as the guest's 32-bit word (counts here are a few words).
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn word(count: usize) -> u32 {
    count as u32
}

/// The low word of a 64-bit result (`_ftol2`'s `EAX`).
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn low_dword(value: u64) -> u32 {
    value as u32
}
