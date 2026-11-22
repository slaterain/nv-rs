//! The script_lines family: the script runner's line loop and the line it runs, which xNVSE wraps
//! (`ScriptRunner::Run` `005e0d20`, `ParseLine` `005e13a0`, `EvaluateLine` `005e1550`, and the
//! action list's run `005e1160`). A wrap registered over any of them clears this family's direct
//! bit for good, so every call into them stays by address and reaches the wrap, whose call to the
//! function it wrapped reaches the typed body here. What the bodies call in the scripts family (the
//! expression's constructor and `Eval`, the referenced object lookup, a variable's set and the
//! action flags' clear) goes direct while that family is direct.
//!
//! - [`runner`]: the line loop, the action list's run, `ParseLine` and `EvaluateLine` with its
//!   `set`, `if` and command lines.
//!
//! What stays by address: `ParseLine` and `EvaluateLine` from the loops (xNVSE's wraps), the
//! operand reader and the parameter parser (`overridable.tsv`), every command's and block type's
//! handler, the logs and the console, the locals' making and delete, and the casts. The seams stay
//! at their points: JIP LN's script wait (`005e0ec9`, `005e1716`), its kept failed scripts
//! (`005e0fbe`, `005e1a23`, `005e1f0b`), and the seams before a block type's handler (`005e16c3`)
//! and a command's handler (`005e2329`).
//!
//! Stage 1: the objects stay in guest memory where the game made them. **Threads and floats.**
//! Scripts run on the main thread and the AI threads ([`Threads::Both`]). The only float maths: a
//! global's value stored as a float (one rounding, the same on every thread) and the profiler's
//! seconds.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod runner;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(12);

/// `NV_TYPED=script_lines`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static SCRIPT_LINES: Family = Family {
    name: "script_lines",
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
        entry!(0x005e_0d20, runner::run_lines(Ptr, Ptr, Ptr, Ptr, Ptr, bool, u8, u8, f32) -> bool),
        entry!(0x005e_1160, runner::build_action_list(Ptr, Ptr, Ptr, Ptr) -> bool),
        entry!(0x005e_13a0, runner::read_line(Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, bool) -> bool),
        entry!(
            0x005e_1550,
            runner::run_line(Ptr, Ptr, i32, Ptr, Ptr, u32, Ptr, u32, bool, bool) -> bool
        ),
    ]
}
