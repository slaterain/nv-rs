//! Vtable slots JIP writes (`SafeWrite32(slot, routine)`), the way step 3.5a writes JIP's hit-data
//! routines into the processes' vtables (`actor_hooks::COPY_HIT_DATA_SLOTS`): a JIP routine is
//! registered at a code address of its own and that address is written into the guest's slot, so
//! only calls through that vtable reach it. Not a wrap of the function the slot held: most of
//! those are shared, linker-folded stubs (`0x401290`, `0x489100`, `0x8D0600`, ...) that hundreds of
//! other slots and direct calls use (`measure/jip/PLAN-3.9-verified.md` §3).
//!
//! Each group has its own 64 routine addresses ([`routine`]), so groups never pick the same one.

use engine_core::abi::AbiFn;
use engine_core::prelude::*;

/// The groups of step 3.9 (`measure/jip/PLAN-3.9-groups.md`), in the order of their routine blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// Step 3.9a itself.
    A39,
    GA,
    GB,
    GC,
    GD,
    GE,
    GP,
    GH1,
    GH2,
    GF,
    GS,
    GW,
    GL,
    GX,
}

/// The first routine address of the game patches: after JIP's identity, override, UI and actor
/// routines (`identity::ROUTINES` + 0 to 0x508).
pub const ROUTINES: u32 = crate::identity::ROUTINES + 0x4000;
/// Routine addresses per group (4 bytes apart).
pub const PER_GROUP: u32 = 64;

const _: () = assert!(
    ROUTINES + (Group::GX as u32 + 1) * PER_GROUP * 4 <= engine_core::mem::EXTENDER_CODE.end
);

/// Routine `index` (0 to 63) of `group`.
pub const fn routine(group: Group, index: u32) -> u32 {
    assert!(index < PER_GROUP, "64 routines a group");
    ROUTINES + (group as u32 * PER_GROUP + index) * 4
}

/// Registers `f` at `routine` and writes that address into the guest vtable slot at `slot` (as
/// JIP's `SafeWrite32`); answers the word the slot held (the function JIP's routine may call on).
/// Before the threads start, while JIP installs or the host's row is set.
pub fn write_slot(e: &mut Engine, slot: u32, routine: u32, f: AbiFn) -> u32 {
    e.register(routine, f);
    let held = e.mem.u32(slot);
    e.mem.set_u32(slot, routine);
    held
}
