//! Group GH2's seams and wraps (step 3.9: references, models, cells, ash piles). Only group GH2's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GH2". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GH2",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G90 (fix row): leaving an interior, 00454450: CloudsFixHook in place of the interior
    // cell's clear (0x454576).
    row(0x0045_4576),
    // G106 (fix row): the water forms' update 004e3d60, the update map's find (0x4E400E; the
    // flag's push at 0x4E4000 nop'd with it).
    row(0x004e_400e),
    // G103 (fix row): the cell's reset walk 0054df30: JIP jumps over a deleted actor's ash pile
    // deletion (0x54E421, SafeWrite16 EB 1E) ...
    row(0x0054_e421),
    // ... and replaces the type tests from 0x54E4E1 on with ClearAshPilesHook.
    row(0x0054_e4e1),
    // H24: the reference's destructor body 0055a430, its call of TESObjectREFR::IsFurniture:
    // JIP's DestroyRefrHook (InitJIPHooks) drops the reference's own name and model path and a
    // temporary reference's aux variables, then answers the furniture test.
    row(0x0055_a488),
    // G78 (fix row): SetScale 00567490: SetScaleHook's clamp in place of the text rounding
    // (0x5674D5; the block's release at 0x567709 jumped over with it).
    row(0x0056_74d5),
    // G06 (JIP's own): the exe's compares of a reference's base form with the two ash pile
    // forms, each replaced by JIP's test of the base's ash pile bit: 0056a290, Activate 00573170,
    // Enable 00573f40, Disable 00574400, 005768b0 ...
    row(0x0056_a30a),
    // G81 (fix row): RemoveWeapon 00571b50: ClearWeaponNodeHook in place of the effects
    // release of the held weapon node (0x571F65) and the "Weapon" child (0x571FDF).
    row(0x0057_1f65),
    row(0x0057_1fdf),
    // G06 (continued).
    row(0x0057_3194),
    row(0x0057_3fe8),
    row(0x0057_445b),
    row(0x0057_6af0),
    // G31 (fix row): 00579690, the end of its door branch: the door's answer stands.
    row(0x0057_9942),
    // G73 (JIP's own): the death ash pile command 005db870 (group GS's seam): AttachAshPileHook
    // picks AttachAshPileEx's form once, else the exe's.
    row(0x005d_bcfd),
    // G06: 00624720 (ef-unplaced-a), HUDMainMenu::SetInfoForRef (group GA's seam), 0095f6c0
    // (group GP's seam).
    row(0x0062_4958),
    row(0x0077_765a),
    row(0x0095_f7fe),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
