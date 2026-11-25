//! The AI queries family: the questions the game asks about an actor's current package and its
//! fight, many times a frame on a crowded street: the current package and its procedure type, the
//! package location's reference, whether the actor flees, its combat controller and its current
//! target. The process lists' clean-up when a reference goes away (`0096f600`, `00976680`, the
//! process family) asks most of them for every actor, for every reference freed; the actors' own
//! virtual slots ask the rest. By `NV_CALL_COUNT` on the Goodsprings street (2026-11-23) they were
//! about 4,000 calls by address a frame on main, one per question and per step of each answer
//! (`Actor::IsFleeing` -> `MobileObject::GetCurrentPackage` -> the process's slot 0x27c -> its slot
//! 0x22c -> the word it holds).
//!
//! - [`queries`]: the field reads (the package's type at +0x20, an actor's byte at +0x104, the
//!   words at +0x2c and +0x40, a package location's reference, a process's package at +4),
//!   `MobileObject::GetCurrentPackage`, `BaseProcess::GetPackageThatIsRunning`,
//!   `Actor::GetCombatController`, `Actor::IsRunningRunOnce`, `Actor::IsFleeing` and the actors'
//!   slot 0x2c8 (`008815a0`). The false body a process's slot 0x35c holds (`0047c850`) is the draw
//!   family's.
//!
//! Reached: each stays registered at its exe address, so the translations' calls by address and
//! the virtual slots that hold them (the process classes' 0x22c, 0x27c, 0x128 and 0x35c, the actor
//! classes' 0x428 and 0x2c8) reach the typed functions; between them, and from the process
//! family's clean-up, the calls are typed calls, and a virtual call whose slot holds one of them
//! (or the leaf that answers 0) is made as that typed call.
//!
//! Stage 1: the objects stay in guest memory where the game made them. **Integers only**: the AI
//! threads ask these too, and there is no float maths here, so the row is [`Threads::Both`].

use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod queries;

/// This family's bit: what each `typed_call!` of it names (34; 32 is the archives' and 33 the
/// navmesh lane's).
pub const FAMILY: FamilyId = FamilyId::new(34);

/// `NV_TYPED=ai_queries`. A plain native conversion proved by its differential tests, so on unless
/// `NV_TYPED` says otherwise.
pub static AI_QUERIES: Family = Family {
    name: "ai_queries",
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
        entry!(0x0041_ca90, queries::package_type(u32) -> i32),
        entry!(0x0049_3bb0, queries::byte_at_104(u32) -> u8),
        entry!(0x0055_b980, queries::word_at_2c(u32) -> u32),
        entry!(0x008d_6f30, queries::word_at_40(u32) -> u32),
        entry!(0x0067_f390, queries::location_reference(u32) -> u32),
        entry!(0x007f_a950, queries::held_package(u32) -> u32),
        entry!(0x0085_9670, queries::package_that_is_running(u32) -> u32),
        entry!(0x0093_44a0, queries::current_package(u32) -> u32),
        entry!(0x008a_02d0, queries::combat_controller(u32) -> u32),
        entry!(0x0088_1570, queries::is_running_run_once(u32) -> u8),
        entry!(0x008a_6650, queries::is_fleeing(u32, u8) -> bool),
        entry!(0x0088_15a0, queries::current_target(u32) -> u32),
    ]
}
