//! Share 3's unit tests (menus and the HUD). Only share 3 edits this file.

use engine_core::extend::stewie::{self, Row};

/// Share 3's rows' plan numbers, off keys and bits (`rows::menus::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[
    (
        "S0020",
        "bSaveINIChangesWhenClosingStartMenuAndSetXPBarQuestAndMessagesVisible",
        bit(43),
    ),
    ("S0022.04", "bMenuFixesSavePictureWithoutHover", bit(39)),
    ("S0022.07", "bMenuFixesRemapDebounce", bit(21)),
    ("S0022.08", "bMenuFixesRecipeWheelCategory", bit(31)),
    ("S0022.09", "bMenuFixesQuestScroll", bit(0)),
    ("S0022.11", "bMenuFixesLevelUpArrowsSelect", bit(22)),
    ("S0022.12", "bMenuFixesRecipeConditionNotch", bit(12)),
    ("S0022.13", "bMenuFixesDebugLockPickHealth", bit(1)),
    ("S0022.19", "bMenuFixesMapDragUndiscovered", bit(2)),
    ("S0022.22", "bMenuFixesConditionFlashFrameRate", bit(32)),
    ("S0022.23", "bMenuFixesVatsConditionHealthMods", bit(40)),
    ("S0022.24", "bMenuFixesVatsConditionWithoutAmmo", bit(41)),
    ("S0022.25", "bMenuFixesMapPadMarker", bit(3)),
    ("S0022.26", "bMenuFixesCompanionLabelClips", bit(4)),
    ("S0022.27", "bMenuFixesRouletteZeroBetText", bit(5)),
    ("S0022.28", "bMenuFixesBlackjackOneCardADeal", bit(23)),
    ("S0022.29", "bMenuFixesBlackjackShoeCount", bit(24)),
    ("S0022.31", "bMenuFixesMapPadSelectionCentred", bit(6)),
    ("S0022.34", "bMenuFixesWeaponModMenuTarget", bit(13)),
    ("S0022.35", "bMenuFixesCompassAlpha", bit(33)),
    ("S0022.37", "bMenuFixesMinigameHelp", bit(48)),
    ("S0022.41", "bMenuFixesCompanionMarkersReset", bit(7)),
    ("S0022.42", "bMenuFixesHealthBarNotTiedToPrompt", bit(34)),
    ("S0022.46", "bMenuFixesCaravanPadConsole", bit(8)),
    ("S0022.47", "bMenuFixesPadBDebounce", bit(51)),
    ("S0022.51", "bMenuFixesCreaturePickpocket", bit(55)),
    ("S0022.52", "bMenuFixesRepairBrokenItemType", bit(14)),
    ("S0022.55", "bMenuFixesVatsHealthOneTick", bit(42)),
    (
        "S0033",
        "bRepairServicesMenuRemembersScrollPosition",
        bit(15),
    ),
    (
        "S0035",
        "bUnsetQuieterRadioWhenForceClosingDialogueMenu",
        bit(25),
    ),
    ("S0038", "bFixRockItLauncherProjectileDisplay", bit(35)),
    ("S0044", "bFixRecipeMenuArmorIcons", bit(16)),
    ("S0047", "bFixVATSAutoWeaponMultiAmmoUseBug", bit(44)),
    ("S0051", "bUpdateDRDTWhenUsingVendorRepair", bit(17)),
    (
        "S0066",
        "bFixPipboyDisplayedChemDurationIfPerkModified",
        bit(45),
    ),
    ("S0069", "bAllowHotkeyGRXAndBooks", bit(54)),
    ("S0070", "bFixBookAndAmmoHotkeyIcons", bit(18)),
    ("S0071", "bFixRecipeNegativeIngredientCount", bit(19)),
    ("S0072", "bFixHideHealthbarSetting", bit(36)),
    ("S0088", "bFixItemModMenuUnEquipBug", bit(26)),
    ("S0092", "bCaravanDoublePayoutFix", bit(11)),
    ("S0095", "bFixConsoleMainMenuVisibility", bit(50)),
    ("S0105", "bFixDrDTArmorDisplayAlphas", bit(53)),
    ("S0107.05", "bMemoryLeakFixesPipBoyLight", bit(49)),
    (
        "S0110",
        "bFixStuckInAddSPECIALPointsIfAtMaxAlready",
        bit(27),
    ),
    ("S0120", "bFixCndArrowsIgnoringHUDFlags", bit(37)),
    ("S0144", "bFixVATSChargingReloadAPCostWithNoAmmo", bit(46)),
    ("S0154", "bPreventLoadingDlcPopup", bit(28)),
    ("S0160", "bMessageMenuFontWidthFix", bit(47)),
    ("S0196.04", "bRouletteMenuFixesZeroWins", bit(9)),
    ("S0196.05", "bRouletteMenuFixesCutBetValue", bit(10)),
    ("S0197", "bCrosshairProjectileNameFix", bit(38)),
    ("S0200", "bRecipeMenuRefreshFix", bit(20)),
    (
        "S0208",
        "bFixRadioVolumeResetWhenUsingSomeHolotapes",
        bit(29),
    ),
    ("S0218", "bWorldMapNoExteriorDoorFix", bit(52)),
    ("S0883", "bMoreDescriptiveSettingsSaveError", bit(30)),
];

/// Share 3's fix bit `n` (`Row::share` over its range).
const fn bit(n: u16) -> Row {
    Row::share(stewie::MENUS_FIXES, stewie::MENUS_FIXES_END, n)
}
