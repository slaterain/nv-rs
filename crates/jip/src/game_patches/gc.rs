//! Group GC's game patches (step 3.9: hit damage, detection, combat formulas): its handlers, wraps and slot routines. Only group
//! GC's PR edits this file (and its own tests file beside it).
//!
//! **G53, detection** (`InitGamePatches`, `internal/patches_game.h` (JIP LN NVSE):
//! `DetectionTeammateHook`, `DetectionGetLevelHook`): part of JIP once it answers "installed"
//! (it only reads the actor flags JIP's commands set, and with none set answers as the exe).
//! `Actor::GetDetectionLevelAgainstActor` (`008a0d10`) asks three times whether the detected actor
//! is the player's teammate (`0056_6950`: the byte `+0x18D`; a teammate takes the player's stealth
//! and sneak values, and one out of combat while the player moves is not detected at all). JIP
//! answers "no" for an actor whose detection it changes: `ForceActorDetectionValue`'s flag
//! (`+0x106` bit 0) or `ToggleDetectionFix`'s (`+0x105` bit 4); otherwise the byte, read directly.
//! The level word of the detected actor (its actor value owner's virtual `+0x28`, a formula input)
//! is read from the player's owner instead for an actor with the detection fix flag.
//! Seams in `engine-fallout-ai` `actor_p3.rs` (`detection_teammate`, `detection_level_source`).

use super::Area;
use crate::game::JipActorFlags;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

/// G53's seams: the three teammate tests and the level word.
pub const DETECTION_TEAMMATE: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x008a_0dab),
    SiteSlot::new(Owner::Jip, 0x008a_1291),
    SiteSlot::new(Owner::Jip, 0x008a_131d),
];
pub const DETECTION_LEVEL: SiteSlot = SiteSlot::new(Owner::Jip, 0x008a_147c);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GC",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: super::nothing,
    jip_sites: &[
        DETECTION_TEAMMATE[0].addr(),
        DETECTION_TEAMMATE[1].addr(),
        DETECTION_TEAMMATE[2].addr(),
        DETECTION_LEVEL.addr(),
    ],
    jip_wraps: &[],
    install_jip,
};

/// `kHookActorFlags_DetectionModified` (`internal/hooks.h` (JIP LN NVSE)), tested as JIP's
/// `test word ptr [ecx+0x105]`: `kHookActorFlag1_DetectionFix` (`+0x105` bit 4) and
/// `kHookActorFlag2_ForceDetectionVal` (`+0x106` bit 0).
pub const DETECTION_MODIFIED: u16 = 0x0110;
/// `kHookActorFlag1_DetectionFix`.
pub const DETECTION_FIX: u8 = 1 << 4;
/// `g_thePlayer`'s word in the exe, and `Actor::bIsTeammate` (the byte `0056_6950` answers).
const THE_PLAYER: u32 = 0x011d_ea3c;
const IS_TEAMMATE: u32 = 0x18d;
/// The actor value owner inside an actor (`Actor`'s `ActorValueOwner` base).
const AV_OWNER: u32 = 0xa4;

fn install_jip(e: &mut Engine) {
    for slot in DETECTION_TEAMMATE {
        e.on_site(slot, detection_teammate);
    }
    e.on_site(DETECTION_LEVEL, detection_level);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DetectionTeammateHook.
/// A teammate test (the detected actor): 0 when JIP changes the actor's detection, else the
/// actor's teammate byte (JIP reads it in place of the call).
fn detection_teammate(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    let flags = u16::from(e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
    )) | u16::from(e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags2,
    )) << 8;
    if flags & DETECTION_MODIFIED != 0 {
        return Some(0);
    }
    Some(u32::from(e.mem.u8(actor.wrapping_add(IS_TEAMMATE))))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DetectionGetLevelHook.
/// The level word's owner (the detected actor): the player's actor value owner when the actor
/// has the detection fix flag (the function still read from the actor's own table), else the
/// exe's call. JIP takes `g_thePlayer + 0xA4` with no test; with no player (never while actors
/// detect) the port keeps the exe's call.
fn detection_level(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    let flags = e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
    );
    let player = e.mem.u32(THE_PLAYER);
    (flags & DETECTION_FIX != 0 && player != 0).then(|| player.wrapping_add(AV_OWNER))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ToggleDetectionFix_Execute.
/// `ToggleDetectionFix enable` (opcode `0x275C`, listed in `commands::jip_fn_actor`): on an actor,
/// sets (any value but 0) or clears its detection fix flag when that differs; read only by G53.
pub fn toggle_detection_fix(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let actor = a.this_obj;
    if !crate::commands::is_actor(e, actor) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let enable = read.int(0, 0) != 0;
    let flags = e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
    );
    if enable != (flags & DETECTION_FIX != 0) {
        e.set(
            Ptr::<JipActorFlags>::new(actor),
            JipActorFlags::jipActorFlags1,
            flags ^ DETECTION_FIX,
        );
    }
    handled(true)
}

#[cfg(test)]
#[path = "gc_tests.rs"]
mod tests;
