//! Group GF's seams and wraps (step 3.9: forms and records). Only group GF's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GF". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G83 (a fix), fn_0048e730: the biped icon path call (00480ef0), JIP's GetIconPathForItemHook.
    Entry {
        addr: 0x0048_e761,
        owners: JIP,
        step: "3.9-GF",
    },
    // G20 (a fix), fn_00523150 (FireWeapon): the jump over the hand-held block with no process.
    Entry {
        addr: 0x0052_48ef,
        owners: JIP,
        step: "3.9-GF",
    },
    // G16 (a fix), ExpelShellCasing 00524db0: the player's first-person casing path jumped over.
    Entry {
        addr: 0x0052_51a3,
        owners: JIP,
        step: "3.9-GF",
    },
    // G07 (a fix), fn_0057d0a0: the grass grid's edge-cell tests jumped over.
    Entry {
        addr: 0x0057_d3b9,
        owners: JIP,
        step: "3.9-GF",
    },
    // G12 (a fix), BGSPerk fn_005eb6a0: a teammate perk's player entry applied once.
    Entry {
        addr: 0x005e_b79b,
        owners: JIP,
        step: "3.9-GF",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // G39 (JIP's own), TESNPC's TESModel::GetModel 00601c30 rewritten by JIP.
    Entry {
        addr: 0x0060_1c30,
        owners: JIP,
        step: "3.9-GF",
    },
];
