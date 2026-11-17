//! Group GW's seams and wraps (step 3.9: world, sky, sound, saves, start-up). Only group GW's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GW". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G74 (a fix), fn_0063c490: the current weather's wind speed (GetWindSpeedHook).
    Entry {
        addr: 0x0063_c4d6,
        owners: JIP,
        step: "3.9-GW",
    },
    // G74 (a fix), fn_0063c490: the last weather's wind speed (GetWindSpeedHook).
    Entry {
        addr: 0x0063_c52c,
        owners: JIP,
        step: "3.9-GW",
    },
    // H17 (JIP's own, InitJIPHooks), Sky::SetCurrentClimate 0063c8f0: the forced climate (SkyRefreshClimateHook).
    Entry {
        addr: 0x0063_c918,
        owners: JIP,
        step: "3.9-GW",
    },
    // G15 (a fix), fn_0063e2f0: the sun's glare from the climate's glare texture.
    Entry {
        addr: 0x0063_e705,
        owners: JIP,
        step: "3.9-GW",
    },
    // G19 (a fix), Sky::UpdateHDRValues 0063ef20: the morning's image-space forms swapped (high-noon fix).
    Entry {
        addr: 0x0063_f385,
        owners: JIP,
        step: "3.9-GW",
    },
    // G19 (a fix), Sky::UpdateHDRValues 0063ef20: the afternoon's image-space forms swapped (high-noon fix).
    Entry {
        addr: 0x0063_f3f7,
        owners: JIP,
        step: "3.9-GW",
    },
    // G104 (a fix), FalloutAudio::UpdateRegionSounds 0082d7c0: the pending acoustic space applied once (SetAcousticSpaceFixHook).
    Entry {
        addr: 0x0082_d8ca,
        owners: JIP,
        step: "3.9-GW",
    },
    // G94 (a fix), fn_0083ac30: a silenced weapon's attack sound for any actor (PlayAttackSoundHook).
    Entry {
        addr: 0x0083_ad97,
        owners: JIP,
        step: "3.9-GW",
    },
    // G89 (a fix), the IntSeenData constructor 0087a100: the word +0x26 cleared.
    Entry {
        addr: 0x0087_a12a,
        owners: JIP,
        step: "3.9-GW",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
