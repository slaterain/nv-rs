//! Group GE's seams and wraps (step 3.9: actors: movement, life, NPC perks). Only group GE's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GE". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // O13 (switch bVoiceModulationFix): fn_00578a80's sound flags (0x106 | ..; group GH2's
    // file, its inert seam): JIP's byte 8 adds 0x80000.
    Entry {
        addr: 0x0057_8e16,
        owners: JIP,
        step: "3.9-GE",
    },
    // G22 (AI Z fix, credits Stewie): PathSmootherPOV 006b0f40, the start direction's z set to
    // 0 (fldz; fstp): removed.
    Entry {
        addr: 0x006b_10a3,
        owners: JIP,
        step: "3.9-GE",
    },
    // O13: map_menu_do_click's sound flags (0x105; group GB's file, its inert seam): JIP's byte
    // 8 adds 0x80000.
    Entry {
        addr: 0x0079_74cc,
        owners: JIP,
        step: "3.9-GE",
    },
    // H18: Actor::GetEssential 0087f3d0, the player's test for a teammate: JIP's
    // IsActorEssentialHook answers 1 for a teammate made killable.
    Entry {
        addr: 0x0087_f427,
        owners: JIP,
        step: "3.9-GE",
    },
    // G22: 00881860, the move direction's z set to 0: removed.
    Entry {
        addr: 0x0088_1a9c,
        owners: JIP,
        step: "3.9-GE",
    },
    // G63: the actor's load-game routine 008ab3a0, its two calls that clear bits 0x80000000
    // (008c1ac0) and 0x40000000 (008c1b50) of the flag word +0x140 after it is read: JIP's 26
    // bytes clear its own flag bytes +0x105/+0x106 and only bit 0x40000000 instead.
    Entry {
        addr: 0x008a_b6d9,
        owners: JIP,
        step: "3.9-GE",
    },
    // G28: 008c17c0, the Agility read for the reload speed: JIP's call of
    // GetClampedActorFloatValue (0066ef50).
    Entry {
        addr: 0x008c_17d9,
        owners: JIP,
        step: "3.9-GE",
    },
    // G28: 008c1940, the same for the equip speed.
    Entry {
        addr: 0x008c_1959,
        owners: JIP,
        step: "3.9-GE",
    },
    // G04: the creature's load routine 008d4ff0, after 008ab3a0, its clear of the flag word
    // +0x140: JIP returns there.
    Entry {
        addr: 0x008d_5003,
        owners: JIP,
        step: "3.9-GE",
    },
    // G23: ForceLevel 008d5300, the level - 1 given to the required experience: JIP gives the
    // level.
    Entry {
        addr: 0x008d_535f,
        owners: JIP,
        step: "3.9-GE",
    },
    // H11: MobileObject's Havok init 00930c70, Actor::CreateRagdollInstance for the player
    // again on the same root node: JIP's InitRagdollControllerHook skips it.
    Entry {
        addr: 0x0093_1384,
        owners: JIP,
        step: "3.9-GE",
    },
    // O13: 009344d0, the dialogue sound's flags 0x104: JIP's VoiceModulationFixHook (0x80104
    // unless the base form's flag 0x2000).
    Entry {
        addr: 0x0093_4ac8,
        owners: JIP,
        step: "3.9-GE",
    },
    // G22: DetailedActorPathHandler 009e6900, the offset's z set to 0: removed.
    Entry {
        addr: 0x009e_6a82,
        owners: JIP,
        step: "3.9-GE",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
