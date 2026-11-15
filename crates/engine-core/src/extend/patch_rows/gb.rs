//! Group GB's seams and wraps (step 3.9: UI: item lists, repair, pad dead zones). Only group GB's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GB". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GB",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // D04, the pad dead zones (JIP's `DeferredInit`: `SafeWrite32` of its own doubles over the
    // operand of `FCOMP double ptr [0x010718F0]`): each seam answers the double to compare with.
    // `CaravanMenu::DoIdle` 00741500, the stick's x and y.
    row(0x0074_16d6),
    row(0x0074_1705),
    // `LockPickMenu::RotateLockpick` 007908f0.
    row(0x0079_0966),
    // `MapMenu::DoGamepad` 00799790, the zoom axis.
    row(0x0079_9861),
    // `RouletteMenu::DoIdle` 007bd2a0, its three stick values.
    row(0x007b_d47d),
    row(0x007b_d4a0),
    row(0x007b_d4dd),
    // `PlayerCharacter::UpdateUFOCamera` 0094a8c0 (seams in group GP's file): the look axes
    // (right stick: JIP's `s_deadZoneRSg`), then the move axes (left stick).
    row(0x0094_a93c),
    row(0x0094_a95f),
    row(0x0094_ab59),
    row(0x0094_ab88),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
