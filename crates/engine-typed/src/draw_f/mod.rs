//! The `draw_f` family (family 25): group F of the world draw's worklist, the small helpers the typed
//! families call by address most: the float maximum and minimum, the run-time type test with its null
//! check, and the list node pool's allocation ([`helpers`]). On by default since 2026-11-11, on its
//! differential tests and review (no comparison run for native code, AGENT-RULES.md);
//! `NV_TYPED=draw_f:off` turns it off. The helpers run on every game thread, so [`Threads::Both`]; they compare floats and
//! move words, nothing that rounds.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod helpers;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(25);

/// `NV_TYPED=draw_f:off` turns it off. On by default since 2026-11-11.
pub static DRAW_F: Family = Family {
    name: "draw_f",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0040_4010, helpers::float_max(f32, f32) -> f32),
        entry!(0x0040_ebd0, helpers::float_min(f32, f32) -> f32),
        entry!(0x0043_b300, helpers::is_kind_of(u32, Ptr<()>) -> bool),
        entry!(0x0043_a010, helpers::allocate_node(Ptr<()>) -> Ptr<()>),
    ]
}
