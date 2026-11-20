//! Group GX's seams and wraps (step 3.9: Havok, pad rumble, mouse). Only group GX's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GX". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // O14 (bSneakBoundingBoxFix), HUDMainMenu::DoIdle: the player's sneak test, the sneak shape's vertices swapped (SneakBoundingBoxFixHook).
    Entry {
        addr: 0x0077_0b0e,
        owners: JIP,
        step: "3.9-GX",
    },
    // G29 (a fix), MobileObject::InitHavok 00930c70: the immobile creature's no-fall flag (MarkCreatureNoFallHook).
    Entry {
        addr: 0x0093_13ec,
        owners: JIP,
        step: "3.9-GX",
    },
    // G49 (JIP's own), the PlayerCharacter constructor: bMenuModeButtonClicked and the three bytes after it cleared.
    Entry {
        addr: 0x0093_9258,
        owners: JIP,
        step: "3.9-GX",
    },
    // G32 (a fix), bhkCollisionObject::NitohkTransform 00c65b00: a fixed body follows its node when asked (0xC65B47, 0xC65B5B).
    Entry {
        addr: 0x00c6_5b5b,
        owners: JIP,
        step: "3.9-GX",
    },
    // O14 (bSneakBoundingBoxFix), fn_00c72410: the player's and teammates' own controller shape (InitControllerShapeHook).
    Entry {
        addr: 0x00c7_2ea3,
        owners: JIP,
        step: "3.9-GX",
    },
    // G29 (a fix), bhkCharacterStateInAir::Update 00cd3fb0: the no-fall flag counts as bit 8.
    Entry {
        addr: 0x00cd_3fdd,
        owners: JIP,
        step: "3.9-GX",
    },
    // G65 (a fix), bhkCharacterStateJumping::Update 00cd4280: the jump height from the controller's own value (JumpHeightFixHook).
    Entry {
        addr: 0x00cd_4468,
        owners: JIP,
        step: "3.9-GX",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // H20 (JIP's own, InitJIPHooks): the mouse axes ToggleMouseMovement turns off, the wheel of a disabled key (GetMouseMovementHook).
    Entry {
        addr: 0x00a2_39e0,
        owners: JIP,
        step: "3.9-GX",
    },
    // G59 (a fix): no pad rumble while bGamePadRumble is off (GamePadRumbleHook).
    Entry {
        addr: 0x00ee_820a,
        owners: JIP,
        step: "3.9-GX",
    },
];
