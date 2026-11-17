//! Group GF's seams and wraps (step 3.9: forms and records). Only group GF's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GF". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G130 (a fix, only with `Data\*.override` files), fn_00463070's archive open (00af4be0).
    Entry {
        addr: 0x0046_3855,
        owners: JIP,
        step: "3.9-GF",
    },
    // G47 (JIP's own), the TESFile constructor 00470710: the word at +0x29A cleared (JIP's
    // hasModLog).
    Entry {
        addr: 0x0047_079d,
        owners: JIP,
        step: "3.9-GF",
    },
    // G47 (JIP's own), TESFile::OpenTES_ov2 00470ca0: the name's hash kept before its copy
    // (InitModInfoNameHook).
    Entry {
        addr: 0x0047_0d56,
        owners: JIP,
        step: "3.9-GF",
    },
    // G45 (a fix), 00484da0: the mask of the saved flags a reference takes (0x400000 added).
    Entry {
        addr: 0x0048_4e03,
        owners: JIP,
        step: "3.9-GF",
    },
    // G83 (a fix), fn_0048e730: the biped icon path call (00480ef0), JIP's GetIconPathForItemHook.
    Entry {
        addr: 0x0048_e761,
        owners: JIP,
        step: "3.9-GF",
    },
    // G57 (a fix), TESObjectARMO::InitItem 00512e60: the TNAM sound template's lookup (004839c0).
    Entry {
        addr: 0x0051_2f3d,
        owners: JIP,
        step: "3.9-GF",
    },
    // G57 (a fix), the same function: each SNAM sound entry's lookup (004839c0).
    Entry {
        addr: 0x0051_2faf,
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
    // G110 (a fix), fn_00595600: the faction reaction call (005fd3b0), JIP's GetFactionReactionHook.
    Entry {
        addr: 0x0059_56bc,
        owners: JIP,
        step: "3.9-GF",
    },
    // G52 (a fix), TESGlobal's record load 005a64b0: a global with form flag 0x40 is not read
    // (TESGlobalLoadFormHook).
    Entry {
        addr: 0x005a_64d7,
        owners: JIP,
        step: "3.9-GF",
    },
    // G12 (a fix), BGSPerk fn_005eb6a0: a teammate perk's player entry applied once.
    Entry {
        addr: 0x005e_b79b,
        owners: JIP,
        step: "3.9-GF",
    },
    // G45 (a fix), 00845550 (group GW's seam): the form id map's bucket count 0x7D63.
    Entry {
        addr: 0x0084_5558,
        owners: JIP,
        step: "3.9-GF",
    },
    // G45 (a fix), the BGSSaveLoadGame constructor (group GW's seam): the same count.
    Entry {
        addr: 0x0084_6ffb,
        owners: JIP,
        step: "3.9-GF",
    },
    // G45 (a fix), UnloadForm (group GW's seam): GetShouldSaveUnloadedFormHook in place of
    // 00849a90.
    Entry {
        addr: 0x0084_97cd,
        owners: JIP,
        step: "3.9-GF",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // G48 (JIP's own), TESForm::SetFormType 004f15a0 rewritten by JIP (clears its flag bytes).
    Entry {
        addr: 0x004f_15a0,
        owners: JIP,
        step: "3.9-GF",
    },
    // H22 (JIP's own), TESObject::IsMarker 0050f9e0 rewritten by JIP
    // (GetIsInternalMarkerHook).
    Entry {
        addr: 0x0050_f9e0,
        owners: JIP,
        step: "3.9-GF",
    },
    // G39 (JIP's own), TESNPC's TESModel::GetModel 00601c30 rewritten by JIP.
    Entry {
        addr: 0x0060_1c30,
        owners: JIP,
        step: "3.9-GF",
    },
];
