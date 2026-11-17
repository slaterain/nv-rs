//! Group GA's seams and wraps (step 3.9: UI: HUD, messages, text, start-up INI). Only group GA's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GA". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GA",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G58 (fix row): InterfaceManager::Idle's console key query 0x70E5FE: JIP's
    // ToggleConsoleHook answers from the key bytes alone, so the console key works also while a
    // controller is connected (the exe's query answers 0 then: flag 0x11F35C8).
    row(0x0070_e5fe),
    // G35 (fix row): MenuConsole::Print 0071d0a0: JIP's jump over the loading menu test.
    row(0x0071_d0d2),
    // H21 (fix row): HUDMainMenu::DoIdle 00770430, before its call of 0x77DA60: JIP's
    // ClearHUDOrphanedTiles destroys the HUD's child tiles whose scene node has no parent.
    row(0x0077_04ae),
    // G10 (fix row): the HUD corner message record's layout (text 0x104 -> 0x204, record 0x310 ->
    // 0x410), one seam per function for JIP's 16 bytes: DoIdle 00770430 (0x7714DA), fn_00775380
    // (the queueing: 0x7753B8 and its ten neighbours to 0x775600), HideMessage 007756e0
    // (0x7757B1, 0x7757D3, 0x77582B).
    row(0x0077_14da),
    row(0x0077_53b8),
    row(0x0077_57b1),
    // G80 (fix row): HUDMainMenu::SetInfoForRef 00775a00, its weapon test 0x777123: JIP's
    // RolloverWeaponHook answers the same test and appends "+" to a modded weapon's name.
    row(0x0077_7123),
    // G21 (fix row): the activation choices 007aa720: a choice with no text is passed over.
    row(0x007a_a80f),
    // G37 (fix row): StartMenu::ConfirmNewGame 007d32b0: InvalidChargenQuestFixHook falls back
    // to the vanilla quest 0x102037.
    row(0x007d_333b),
    // G11 (fix row): TraitMenu::DoEnter 007e6ea0: the description's buffer 0x200 -> 0x400 (one
    // seam for JIP's four writes 0x7E6EA6, 0x7E6F04, 0x7E6F09, 0x7E6F2B).
    row(0x007e_6f04),
    // G121 (fix row): VATSMenu::DoIdle 007ec810, the target's damage threshold 0x7ECE32: JIP's
    // GetVATSTargetDTHook answers the threshold its hit damage last used.
    row(0x007e_ce32),
    // G17 (fix row): the TileMenu destructor 00a1eff0: JIP's jump over the menu's open byte clear.
    row(0x00a1_f035),
    // G08 (fix row): INISettingCollection::ReadSetting 00c44280: the text buffer 0x100 -> 0x400.
    row(0x00c4_4405),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
