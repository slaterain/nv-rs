//! Share 3's unit tests (menus and the HUD). Only share 3 edits this file.

use engine_core::extend::stewie::{self, Row};

/// Share 3's rows' plan numbers, off keys and bits (`rows::menus::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[
    ("S0015", "bEasterEggs", bit(57)),
    (
        "S0020",
        "bSaveINIChangesWhenClosingStartMenuAndSetXPBarQuestAndMessagesVisible",
        bit(43),
    ),
    ("S0022.04", "bMenuFixesSavePictureWithoutHover", bit(39)),
    ("S0022.06", "bMenuFixesVatsKeysOverMouse", bit(60)),
    ("S0022.07", "bMenuFixesRemapDebounce", bit(21)),
    ("S0022.08", "bMenuFixesRecipeWheelCategory", bit(31)),
    ("S0022.09", "bMenuFixesQuestScroll", bit(0)),
    ("S0022.11", "bMenuFixesLevelUpArrowsSelect", bit(22)),
    ("S0022.12", "bMenuFixesRecipeConditionNotch", bit(12)),
    ("S0022.13", "bMenuFixesDebugLockPickHealth", bit(1)),
    ("S0022.19", "bMenuFixesMapDragUndiscovered", bit(2)),
    ("S0022.20", "bMenuFixesRecipeCategoryReset", bit(64)),
    ("S0022.22", "bMenuFixesConditionFlashFrameRate", bit(32)),
    ("S0022.23", "bMenuFixesVatsConditionHealthMods", bit(40)),
    ("S0022.24", "bMenuFixesVatsConditionWithoutAmmo", bit(41)),
    ("S0022.25", "bMenuFixesMapPadMarker", bit(3)),
    ("S0022.26", "bMenuFixesCompanionLabelClips", bit(4)),
    ("S0022.27", "bMenuFixesRouletteZeroBetText", bit(5)),
    ("S0022.28", "bMenuFixesBlackjackOneCardADeal", bit(23)),
    ("S0022.29", "bMenuFixesBlackjackShoeCount", bit(24)),
    ("S0022.31", "bMenuFixesMapPadSelectionCentred", bit(6)),
    ("S0022.33", "bMenuFixesConsoleQuickSaveAmmoTab", bit(61)),
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
    ("S0028", "bVATSHighlightBodyPartNoHideWeapon", bit(62)),
    (
        "S0033",
        "bRepairServicesMenuRemembersScrollPosition",
        bit(15),
    ),
    ("S0034", "bFixRepairMenuDuplicateItemsExploit", bit(65)),
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
    ("S0116", "bShowAlertIfChargenQuestMissing", bit(66)),
    ("S0120", "bFixCndArrowsIgnoringHUDFlags", bit(37)),
    ("S0144", "bFixVATSChargingReloadAPCostWithNoAmmo", bit(46)),
    ("S0154", "bPreventLoadingDlcPopup", bit(28)),
    ("S0160", "bMessageMenuFontWidthFix", bit(47)),
    (
        "S0176.01",
        "bResolveIncludesLongStringFixLongString",
        bit(58),
    ),
    ("S0176.02", "bResolveIncludesLongStringFixLength", bit(59)),
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
    ("S0227", "bTileTextureCacheFix", bit(56)),
    ("S0240", "bFontDimensionMismatchFix", bit(67)),
    ("S0276", "bSaveDelay", tweak(0)),
    ("S0278", "bFasterHackingToComputerTransition", tweak(1)),
    ("S0286", "bConsoleNumpadSupport", tweak(8)),
    ("S0340", "bDecreaseDialogueClickingDelay", tweak(9)),
    ("S0348", "bConsoleNoStoreDuplicates", tweak(11)),
    ("S0351", "bAutoSkipLoadingTiles", tweak(2)),
    ("S0377", "bHideCursorInDialog", tweak(14)),
    ("S0382", "bVATSIgnoreFriendlies", tweak(16)),
    ("S0407", "bNonSelectablePlayerInConsole", tweak(3)),
    ("S0467", "bSkipSkillMenuIfNoPointsToAssign", tweak(12)),
    ("S0482", "bDontAllowConsoleBeforeInitIsDone", tweak(4)),
    ("S0539", "bClickingShowsFullTerminalText", tweak(17)),
    (
        "S0550",
        "bHideRedCrosshairOnDistantInvisibleTargets",
        tweak(15),
    ),
    ("S0556", "bNoCharacterNameLimit", tweak(5)),
    ("S0592", "bFasterEnterLockPickMenu", tweak(6)),
    ("S0608", "bDeselectHotkeyedItems", tweak(13)),
    ("S0628", "bAllowTakeAllInCompanionContainers", tweak(10)),
    ("S0734", "bEnemyHealthbarShowBuffedHP", tweak(18)),
    ("S0763", "bCapitalizeRecipeCategories", tweak(7)),
    ("S0883", "bMoreDescriptiveSettingsSaveError", bit(30)),
    ("S0983", "bPipboyMapStoreRefIDs", bit(63)),
];

/// Share 3's fix bit `n` (`Row::share` over its range).
const fn bit(n: u16) -> Row {
    Row::share(stewie::MENUS_FIXES, stewie::MENUS_FIXES_END, n)
}

/// Share 3's tweak bit `n`.
const fn tweak(n: u16) -> Row {
    Row::share(stewie::MENUS_TWEAKS, stewie::MENUS_TWEAKS_END, n)
}

/// Row S0351 (Tweaks' `bSkipIntroVideo`): Tweaks' settings turn it on with `bFasterTitleMenu` too
/// (settings.h, after reading: `if (g_bFasterTitleMenu) g_bSkipIntroVideo = 1;`); the row's own
/// key off still turns it off.
#[test]
fn s0351_the_faster_title_menu_skips_the_intro_too() {
    use crate::rows;
    use crate::settings::Settings;
    let row = rows::all().find(|r| r.plan == "S0351").unwrap();
    let keys = |text: &'static str| {
        rows::Keys::read(move |key: &str| {
            text.lines()
                .filter_map(|l| l.split_once('='))
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v.to_string())
        })
    };
    let settings = |text: &'static str| Settings::new(&[("t.ini", text.as_bytes())], false);
    for (file, on) in [
        ("[Tweaks]\nbSkipIntroVideo=0\nbFasterTitleMenu=0\n", false),
        ("[Tweaks]\nbSkipIntroVideo=0\nbFasterTitleMenu=1\n", true),
        ("[Tweaks]\nbSkipIntroVideo=1\nbFasterTitleMenu=0\n", true),
    ] {
        let settings = settings(file);
        assert_eq!(rows::on(row, &keys(""), &settings), on, "{file}");
        assert!(
            !rows::on(row, &keys("bAutoSkipLoadingTiles=0"), &settings),
            "{file}"
        );
    }
}

/// Rows S0382 and S0377 at the install: the VATS target list's new Stewie seam (`0x7F5420`) is
/// answered once Tweaks is installed (xNVSE's raw shift states, here both clear: 0; the handler is
/// S0022.03's text seam's), and the two sub-settings read there take VNV's values with no INI
/// (`bOnlyWhenNPCSpeaks` 1, `bHideFriendliesByDefault` 0) and follow an INI that says otherwise.
#[test]
fn s0382_s0377_the_vats_seam_and_the_two_sub_settings() {
    let mut e = super::engine(None);
    assert_eq!(e.site(0x007f_5420, &[]), None, "not before the install");
    crate::set_on(&mut e, true);
    crate::install(&mut e);
    assert_eq!(e.site(0x007f_5420, &[]), Some(0));
    assert!(e.stewie_cursor_only_speaking());
    assert!(!e.stewie_vats_friendlies_hidden());

    let ini =
        b"[Dialog Hide Mouse]\r\nbOnlyWhenNPCSpeaks=0\r\n[VATS]\r\nbHideFriendliesByDefault=1\r\n";
    let mut e = super::engine(Some(&ini[..]));
    crate::set_on(&mut e, true);
    crate::install(&mut e);
    assert!(!e.stewie_cursor_only_speaking());
    assert!(e.stewie_vats_friendlies_hidden());
}
