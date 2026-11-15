//! Group GE's game patches (step 3.9: actors: movement, life, NPC perks): its handlers, wraps and slot routines. Only group
//! GE's PR edits this file (and its own tests file beside it).
//!
//! **G63, actor flags on load** (`InitGamePatches`, `internal/patches_game.h` (JIP LN NVSE):
//! `SAFE_WRITE_BUF(0x8AB6D9, ...)`, 26 bytes). The actor's load-game routine (`008ab3a0`, also
//! reached from `008d33f0` and the creature's `008d4ff0`) reads the flag word `+0x140` from the
//! save and then calls `008c1ac0(0)` and `008c1b50(0)`, which clear its bits 0x80000000 (the
//! runtime "essential" bit) and 0x40000000. JIP's bytes in place of the two calls:
//! `mov ecx, [ebp-0xAC]` (the actor); `mov word [ecx+0x105], 0` (JIP's flag bytes
//! `jipActorFlags1`/`jipActorFlags2`: combat disabled, forced detection values and the rest its
//! commands set); `and byte [ecx+0x143], 0xBF` (bit 0x40000000 only); `nop`. So the essential bit a
//! save holds survives the load, and JIP's per-actor flags start clear on every load.
//!
//! Two kinds in one patch: the reset of JIP's own bytes answers JIP's commands (in whenever JIP
//! answers "installed", every preset; nothing of the game reads those bytes), while keeping the
//! essential bit changes the game, so it is a fix (with the row `NV_JIP_GAME_PATCHES` on). With
//! the row off the handler clears JIP's bytes and lets the exe's two calls run.

use super::Area;
use crate::game::JipActorFlags;
use engine_core::extend::{jip_patches, Owner, SiteSlot};
use engine_core::layout;
use engine_core::prelude::*;

layout! {
    /// `Actor` (0x1B4 bytes on PC), the field G63 writes (kept here, in the group's own file).
    pub struct ActorFlags: 0x1B4 {
        /// The actor's flag word (JIP LN NVSE's `actorFlags`, `// 140`: "0x80000000 -
        /// IsEssential"); `008c1ac0`/`008c1b50` set and clear its bits 0x80000000 and 0x40000000.
        0x140 actorFlags: u32,
    }
}

/// G63's seam: the load routine's two flag calls (words: the actor).
pub const ACTOR_FLAGS_LOADED: SiteSlot = SiteSlot::new(Owner::Jip, 0x008a_b6d9);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GE",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: super::nothing,
    jip_sites: &[ACTOR_FLAGS_LOADED.addr()],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(ACTOR_FLAGS_LOADED, actor_flags_loaded);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x8AB6D9, "\x8B\x8D\x54\xFF\xFF\xFF\x66\xC7\x81\x05\x01...")).
/// The load routine's flag calls (`0x8AB6D9`: the actor, the routine's `this`, never 0): JIP's two
/// flag bytes cleared; with the fixes, bit 0x40000000 of the flag word cleared and the calls
/// replaced (answered), else the calls run (nothing answered).
fn actor_flags_loaded(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = Ptr::<JipActorFlags>::new(a[0]);
    e.set(actor, JipActorFlags::jipActorFlags1, 0);
    e.set(actor, JipActorFlags::jipActorFlags2, 0);
    if e.jip_patches() & jip_patches::FIXES == 0 {
        return None;
    }
    // JIP's `and byte [ecx+0x143], 0xBF`: the word's top byte, the other bytes as they are.
    let actor = Ptr::<ActorFlags>::new(a[0]);
    let flags = e.get(actor, ActorFlags::actorFlags);
    e.set(actor, ActorFlags::actorFlags, flags & !0x4000_0000);
    Some(0)
}

#[cfg(test)]
#[path = "ge_tests.rs"]
mod tests;
