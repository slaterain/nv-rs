//! Group GL's seams and wraps (step 3.9: lights and magic effects). Only group GL's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GL". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GL",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G86 (`WriteRelCall(0x406720, GetEffectHiddenHook)`): `EffectItemList::BuildMenuString`
    // 00406620 leaves out effects with no setting or a hidden one.
    row(0x0040_6720),
    // G107, port-side: `BuildMenuString`'s call of `StatsMenu::StatusDataList::Register`
    // (`0x40686D`), the frame JIP's `EffectItemGetModifiedDuration` reads (`[[EBP]-0xD4]`): the
    // list is handed over here.
    row(0x0040_686d),
    // G67 (`WriteRelJump(0x4069D4, EffectSkillModifiers1Hook)`): `BuildMenuString`'s Medicine and
    // Survival scaling of the effect text.
    row(0x0040_69d4),
    // G107 (`WriteRelCall(0x7E0C1F, EffectItemGetModifiedDuration)`): `Register` 007e0b20's
    // duration, a chem's after `ModifyPositiveChemDuration` (seam in `statsmenu.rs`).
    row(0x007e_0c1f),
    // G66 (26 bytes at `0x80ECA8`): `LightEffect::AttachLight` 0080e970 attaches only to a node.
    row(0x0080_eca8),
    // G67 (`WriteRelJump(0x816519, EffectSkillModifiers2Hook)`): `MagicCaster::FindTargets`
    // 00815d00's alchemy Medicine and Survival scaling.
    row(0x0081_6519),
    // G107 (`WriteRelJump(0x8236FD, ModPositiveChemDurationHook)`): `MagicTarget::CheckAddEffect`
    // 00823210 applies `ModifyPositiveChemDuration` only to chems (types 10 and 11).
    row(0x0082_36fd),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
