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
//! - [`interface_cpp_p2`]: the same unit from `00705a90` up to `00709b50`: more of the static
//!   wrappers (the loading, hacking, computers and level-up menus' creation, save and load, the roots,
//!   the screen sizes, timers, the enter stack, model files, rendered menus, the queued menu opens),
//!   the tab line's layout, the item card's stats text, and the `Tile`, `TileRect` and `TileMenu`
//!   type names, constructors and deleting destructors.
//! - [`menu_manager`]: the menu manager unit up to `00721060`: `MenuManager` (its instance, the menu
//!   factory, reloading, deleting, the frontmost menu, whether a menu is open) and the constructors,
//!   destructors, classes and tile attaching of the menus the factory makes that were emitted there.
//!
//! **Floats.** The menus run on the main thread ([`Threads::Main`]), at the float precision the
//! exe's main thread computes in once Direct3D set it: plain `f32` (the camera frustum's products;
//! a product by one of the exe's double factors is exact in an `f64` and rounded once to the float).
//! The item card's ratios, the screen ratio and the tab line's share take a double operand of the
//! exe's (a constant, an integer loaded whole): each such step is the `f64` result rounded once to the
//! float, the 24-bit step. The rest are stores of integers as floats, and the menu manager's
//! compares of tile traits (a float against a float or a double, exact either way).

use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::AbiFn;

pub mod interface_cpp;
pub mod interface_cpp_p2;
pub mod interface_cpp_p3;
pub mod menu_manager;

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
    all.extend(interface_cpp_p2::funcs());
    all.extend(interface_cpp_p3::funcs());
    all.extend(menu_manager::funcs());
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

/// The low word of a 64-bit integer store, read as a signed dword (what `FISTP qword` leaves there).
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn low_dword_signed(value: i64) -> i32 {
    value as i32
}

/// An integer's bits as the guest's unsigned word (what a trait or format argument takes).
#[allow(clippy::cast_sign_loss)]
pub(crate) fn bits(value: i32) -> u32 {
    value as u32
}
