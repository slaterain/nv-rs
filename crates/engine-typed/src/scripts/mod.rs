//! The scripts family: what every running script line goes through to get its values. The
//! expression evaluator (`CExpression`: its construction, the conversion of an infix text to
//! postfix, the tokenizer, `Eval` and its two stacks), the number tests its operands take, and the
//! operand reader (`005ac7a0`) with the two lookups it makes for each variable and referenced
//! object (`Script::GetReferencedObject`, `ScriptLocals::GetVariable`). Every mod's scripts run
//! through these, every frame: a HUD mod, the mod configuration menu and the UI organizer add
//! thousands of evaluated lines a frame.
//!
//! - [`expression`]: `CExpression`, `00593490` to `00594d40`;
//! - [`numbers`]: the text tests of an operand (`IsAnInteger`, `IsAFloat`);
//! - [`value`]: the operand reader and its two lookups;
//! - [`run`]: `Script::Run` and the run manager every script run goes through, a fresh runner, and
//!   the locals' variable set and action flags' clear a run makes.
//!
//! What stays by address: the tokenizer (`00593870`, which lStewieAl's Tweaks' script inlines
//! replace; a built-in of it reaches every caller there), the operand reader (`005ac7a0`, a
//! script or frame function of `overridable.tsv`), the CRT's `atol` and `atof`, every script
//! command's handler, the error reports, and the seams the extenders hook: xNVSE's command call
//! inside `Eval` (`005949d4`), JIP LN's double precision operators (`00593fbc`) and the seam before
//! a command's handler in the operand reader (`005acb9d`).
//!
//! Stage 1: the objects stay in guest memory where the game made them; each function stays
//! registered at its exe address. **Threads and floats.** Scripts run on the main thread and on the
//! AI threads (the package and combat scripts the middle-high process runs there), so the row says
//! [`Threads::Both`]. The values are doubles; an arithmetic step is native `f64` maths, rounded to
//! a float's 24 bits on the main thread (its x87 precision), as the exe's `FADD`, `FSUB`, `FMUL`
//! and `FDIV` round there (the exact x87 step only for a result below 2^-1000, where the x87's
//! wider exponent matters); logic, comparisons and integers are exact.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod expression;
pub mod numbers;
pub mod run;
pub mod value;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(11);

/// `NV_TYPED=scripts`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static SCRIPTS: Family = Family {
    name: "scripts",
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
        entry!(0x0059_3490, expression::construct(Ptr) -> Ptr),
        entry!(0x0059_4b60, expression::operator_stack_construct(Ptr) -> Ptr),
        entry!(0x0059_4c60, expression::operand_stack_construct(Ptr) -> Ptr),
        entry!(0x0059_34d0, expression::set_error(Ptr, u32)),
        entry!(0x0059_3500, expression::to_postfix(Ptr, Ptr, Ptr) -> Ptr),
        entry!(0x0059_3870, expression::next_token(Ptr, Ptr, Ptr, Ptr, bool, Ptr) -> u32),
        entry!(0x0059_3db0, expression::eval(Ptr, Ptr, u32, u32, u32, u32, i32, u8) -> f64),
        entry!(0x0059_4b80, expression::operator_push(Ptr, u32)),
        entry!(0x0059_4bc0, expression::operator_pop(Ptr) -> u32),
        entry!(0x0059_4c00, expression::operator_is_empty(Ptr) -> bool),
        entry!(0x0059_4c20, expression::operator_is_full(Ptr) -> bool),
        entry!(0x0059_4c40, expression::operator_clear(Ptr)),
        entry!(0x0059_4c80, expression::operand_push(Ptr, u32, u32, u32, u32)),
        entry!(0x0059_4cd0, expression::operand_pop(Ptr, Ptr) -> Ptr),
        entry!(0x0059_4d20, expression::operand_is_empty(Ptr) -> bool),
        entry!(0x0059_4d40, expression::operand_clear(Ptr)),
        entry!(0x004b_1230, numbers::is_an_integer(Ptr) -> bool),
        entry!(0x004b_12d0, numbers::is_a_float(Ptr) -> bool),
        entry!(0x005a_c7a0, value::read_operand(u32, u32, u32, u32, u32, u32, u32, u8) -> bool),
        entry!(0x005a_c4f0, value::referenced_object(Ptr, u32, u32) -> u32),
        entry!(0x005a_9140, value::get_variable(Ptr, u32, Ptr) -> f64),
        entry!(0x005a_c1e0, run::script_run(Ptr, u32, u32, u32, u8) -> bool),
        entry!(0x005a_c340, run::run_effect_start(Ptr, u32, u32)),
        entry!(0x005a_c380, run::run_effect_finish(Ptr, u32, u32)),
        entry!(0x005a_c3c0, run::run_effect_update(Ptr, u32, u32, f32)),
        entry!(0x005e_24d0, run::manager_instance() -> Ptr),
        entry!(
            0x005e_2590,
            run::manager_run(Ptr, Ptr, Ptr, Ptr, Ptr, u8, u8, u8, f32) -> bool
        ),
        entry!(0x005e_0c60, run::construct_runner(Ptr) -> Ptr),
        entry!(0x005e_2850, run::runner_is_busy(Ptr) -> bool),
        entry!(0x005e_2730, run::manager_init_action_list(Ptr, Ptr, Ptr, Ptr) -> bool),
        entry!(0x005a_e2e0, run::forget_referenced_objects(Ptr)),
        entry!(0x005a_8ea0, run::clear_action_flags(Ptr)),
        entry!(0x005a_9290, run::set_variable(Ptr, u32, f64)),
    ]
}
