//! JohnnyGuitar NVSE's seams and wraps: its editor-id restoration (`internal/JG/
//! EditorIDRestoration.cpp`, `InitHooks`), the first piece of JohnnyGuitar built in. Rows by
//! address, owner `super::JG`, step "jg-edid"; the handlers are `crates/johnnyguitar`'s, in with
//! its row ("JohnnyGuitar's editor ids"). The vtable slots it writes (its getter and setter on the
//! form classes' +0x130 / +0x134, the nine classes that store a name wrapped) need no row.

use super::super::Entry;
use super::JG;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JG,
        step: "jg-edid",
    }
}

/// The seams (by address).
pub(in super::super) const SITES: &[Entry] = &[
    // `ReplaceCall(0x451CBA, hk_GetFormEditorID)`: `TESObjectCELL::GetCellName` 00451cb0.
    row(0x0045_1cba),
    // `WriteRelJump(0x4679F8, 0x467A84)`: the load's own insertion into the editor id map skipped
    // (`fn_00467780`'s registration of a loaded form).
    row(0x0046_79f8),
    // `PatchMemoryNop(.., 5)`: the hand registrations of GameYear, GameMonth, GameDay, GameHour,
    // GameDaysPassed, TimeScale (`GenerateDefaultObjects` 0046a370) ...
    row(0x0046_ca2b),
    row(0x0046_cb0d),
    row(0x0046_cbef),
    row(0x0046_ccd1),
    row(0x0046_cdaf),
    row(0x0046_ce91),
    // ... and PlayCredits.
    row(0x0046_dc77),
    // `ReplaceCall(0x48449A, hk_DetachEditorIDs)`: `TESForm::SetTemporary` 00484490's call of
    // `RemoveFromDataStructures` (the seam right after it).
    row(0x0048_449a),
    // `ReplaceCall(.., hk_GetFormEditorID)`: `TESForm::GetFormDetailedString` 004868f0,
    // `TESObjectCELL::SaveRenderFailureData` 005586e0, the reference's +0x130 (0055d480), its
    // `GetFormDetailedString` (0055d540), `InitItem` 0055d760 (three).
    row(0x0048_6903),
    row(0x0055_8796),
    row(0x0055_d498),
    row(0x0055_d683),
    row(0x0055_df83),
    row(0x0055_e076),
    row(0x0055_e0cc),
    // `ReplaceCall(0x58D7BC, hk_SetFormEditorID)`: `BGSDefaultObjectManager`'s constructor.
    row(0x0058_d7bc),
    // `PatchMemoryNop(0x61A243, 5)`: the default topics' hand registration (0061a040).
    row(0x0061_a243),
    // `ReplaceCall(0x61DA92, hk_GetFormEditorID)`: `TESTopicInfo::GetFormDetailedString` 0061d950.
    row(0x0061_da92),
    // `ReplaceCall(0x66FF57, hk_SetFormEditorID)`: `ActorValueInfo`'s constructor (0066fe40).
    row(0x0066_ff57),
    // `ReplaceCall(.., GetNameForConsole)`: the console's selected reference
    // (`InterfaceManager::DisplayCurrentPickRef` 00710ad0, `MenuConsole::Idle` 0071b210).
    row(0x0071_0bfc),
    row(0x0071_b748),
];

/// The wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // `WriteRelJump(0x474CB0, hk_GetFormEditorIDLength)`.
    row(0x0047_4cb0),
    // `WriteRelJump(0x483A00, hk_GetFormByEditorID)`: the lookup by name under JG's lock.
    row(0x0048_3a00),
];
