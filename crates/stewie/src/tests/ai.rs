//! Share 2's unit tests (AI, actors and the player). Only share 2 edits this file.

use crate::{rows, settings::Settings};
use engine_core::extend::stewie::{Row, AI_FIXES};

/// S0206 follows Tweaks' own switch, `[INI] bFixCreatureDoubledDR` (`settings.h`, code default 1,
/// not in VNV's INI): on with no INI at all, off when the player's main Tweaks INI sets it to 0, on
/// again when it sets 1, and off by its own key too.
#[test]
fn s0206_obeys_tweaks_fix_creature_doubled_dr() {
    let row = rows::all().find(|r| r.plan == "S0206").unwrap();
    assert_eq!(row.tweaks, Some(("INI", "bFixCreatureDoubledDR")));
    let keys = rows::Keys::default();
    assert!(rows::on(row, &keys, &Settings::new(&[], false)), "no file");
    assert!(
        rows::on(row, &keys, &Settings::new(&[], true)),
        "Tweaks' code default"
    );
    let off = Settings::new(&[("p", b"[INI]\nbFixCreatureDoubledDR=0\n")], false);
    assert!(!rows::on(row, &keys, &off), "the main INI's 0");
    let on = Settings::new(&[("p", b"[INI]\nbFixCreatureDoubledDR=1\n")], false);
    assert!(rows::on(row, &keys, &on));
    // A main-only key (inside Tweaks' `if (isDefaultINI)`): an overlay file's 0 is ignored.
    let overlay = Settings::new(
        &[("p", b""), ("o", b"[INI]\nbFixCreatureDoubledDR=0\n")],
        false,
    );
    assert!(rows::on(row, &keys, &overlay), "an overlay's 0");
    let off_by_key = rows::Keys {
        rows: vec![("bCreatureDRFix", false)],
        ..Default::default()
    };
    assert!(!rows::on(row, &off_by_key, &Settings::new(&[], false)));
}

/// S0111 follows Tweaks' own `[INI] bFixLoopingReloadAmmoSwapInstantRefill` (code default 1, not
/// in VNV's INI): on with no file either way, off when the player's Tweaks INI sets 0.
#[test]
fn s0111_obeys_tweaks_fix_looping_reload_ammo_swap() {
    let row = rows::all().find(|r| r.plan == "S0111").unwrap();
    assert_eq!(
        row.tweaks,
        Some(("INI", "bFixLoopingReloadAmmoSwapInstantRefill"))
    );
    let keys = rows::Keys::default();
    assert!(rows::on(row, &keys, &Settings::new(&[], false)));
    assert!(rows::on(row, &keys, &Settings::new(&[], true)));
    let off = Settings::new(
        &[("p", b"[INI]\nbFixLoopingReloadAmmoSwapInstantRefill=0\n")],
        false,
    );
    assert!(!rows::on(row, &keys, &off));
}

/// S0660, S0664 and S0782 follow Tweaks' own `[Tweaks]` keys (code default 0, VNV's 1): on with
/// no file (the port's defaults are VNV's), off when the player's Tweaks INI sets the key to 0, off
/// with no file under `bMissingKeysAsTweaks` (Tweaks' code default), and off by their own keys.
#[test]
fn tweaks_keyed_rows_follow_their_tweaks_keys() {
    for (plan, key) in [
        ("S0660", "bExplosionKnockbackDirectionFix"),
        ("S0664", "bFixKillChallengeSourceWeapon"),
        ("S0782", "bHiddenFragsFix"),
    ] {
        let row = rows::all().find(|r| r.plan == plan).unwrap();
        assert_eq!(row.tweaks, Some(("Tweaks", key)), "{plan}");
        let keys = rows::Keys::default();
        assert!(
            rows::on(row, &keys, &Settings::new(&[], false)),
            "{plan} VNV's 1"
        );
        assert!(
            !rows::on(row, &keys, &Settings::new(&[], true)),
            "{plan} code default"
        );
        let text = format!("[Tweaks]\n{key}=0\n");
        let off = Settings::new(&[("p", text.as_bytes())], false);
        assert!(!rows::on(row, &keys, &off), "{plan} the INI's 0");
        let off_by_key = rows::Keys {
            rows: vec![(row.key, false)],
            ..Default::default()
        };
        assert!(
            !rows::on(row, &off_by_key, &Settings::new(&[], false)),
            "{plan}"
        );
    }
}

/// Share 2's rows' plan numbers, off keys and bits (`rows::ai::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[
    (
        "S0019",
        "bAllowAPRegenModifiersWithNoEquippedWeapon",
        Row::at(AI_FIXES, 32),
    ),
    (
        "S0022.21",
        "bMenuFixesWorkbenchNoActivateSound",
        Row::at(AI_FIXES, 42),
    ),
    (
        "S0024",
        "bPreventFollowerNoAmmoTopicIfUsingRechargerWeapon",
        Row::at(AI_FIXES, 20),
    ),
    (
        "S0039",
        "bFixWeaponIdleSoundsHavingInfiniteRange",
        Row::at(AI_FIXES, 11),
    ),
    (
        "S0041",
        "bFixWeaponModGetDestructibleForm",
        Row::at(AI_FIXES, 21),
    ),
    (
        "S0052",
        "bPreventAimingFOVDuringKillcams",
        Row::at(AI_FIXES, 43),
    ),
    (
        "S0055",
        "bFixZeroSpreadZeroVATSChance",
        Row::at(AI_FIXES, 22),
    ),
    (
        "S0074",
        "bFixCombatMissileDepthGamesettings",
        Row::at(AI_FIXES, 19),
    ),
    (
        "S0089.01",
        "bRememberAmmoCountOnLoadSavedCount",
        Row::at(AI_FIXES, 25),
    ),
    (
        "S0089.02",
        "bRememberAmmoCountOnLoadNoReload",
        Row::at(AI_FIXES, 26),
    ),
    (
        "S0089.03",
        "bRememberAmmoCountOnLoadReloadNoNewEntry",
        Row::at(AI_FIXES, 27),
    ),
    (
        "S0089.06",
        "bRememberAmmoCountOnLoadPlayerWithout3D",
        Row::at(AI_FIXES, 28),
    ),
    (
        "S0089.07",
        "bRememberAmmoCountOnLoadSetupAmmo",
        Row::at(AI_FIXES, 29),
    ),
    (
        "S0089.08",
        "bRememberAmmoCountOnLoadHotkeyAmmoNeedsRounds",
        Row::at(AI_FIXES, 30),
    ),
    (
        "S0089.09",
        "bRememberAmmoCountOnLoadSwapSetsType",
        Row::at(AI_FIXES, 31),
    ),
    ("S0094", "bFixDisintegrationsStat", Row::at(AI_FIXES, 33)),
    (
        "S0111",
        "bFixLoopingReloadAmmoSwapInstantRefill",
        Row::at(AI_FIXES, 50),
    ),
    (
        "S0112",
        "bFixBloodyMessPreventingExplosionGibbing",
        Row::at(AI_FIXES, 34),
    ),
    (
        "S0121",
        "bPreventReloadingIfClipIsOverfilled",
        Row::at(AI_FIXES, 51),
    ),
    (
        "S0133",
        "bFixSpectatorThreatInfoUsingWrongWeaponType",
        Row::at(AI_FIXES, 16),
    ),
    (
        "S0137",
        "bPreventAgroRadiusBreachedNPCsHolsteringWeapons",
        Row::at(AI_FIXES, 9),
    ),
    ("S0146", "bNPCThrowingAnimFix", Row::at(AI_FIXES, 13)),
    (
        "S0147",
        "bFixCrippledLimbAnimsNotUpdatingIfLimbsFixed",
        Row::at(AI_FIXES, 0),
    ),
    ("S0149", "bVATSFatalCheckDifficulty", Row::at(AI_FIXES, 35)),
    (
        "S0153",
        "bFixWeaponNifRoundNotCullingInFirstPerson",
        Row::at(AI_FIXES, 44),
    ),
    (
        "S0159",
        "bFixInfiniteNPCLoopingReloads",
        Row::at(AI_FIXES, 1),
    ),
    (
        "S0164",
        "bPreventLevelupInTestAllCells",
        Row::at(AI_FIXES, 45),
    ),
    ("S0171", "bKnockDownHandGripFix", Row::at(AI_FIXES, 12)),
    (
        "S0180",
        "bMineDisarmedLightLoadingFix",
        Row::at(AI_FIXES, 8),
    ),
    (
        "S0188",
        "bShotProjectileResistanceFix",
        Row::at(AI_FIXES, 36),
    ),
    (
        "S0189.01",
        "bInventoryCalcWeaponDamageUseAmmoFixDisplay",
        Row::at(AI_FIXES, 23),
    ),
    (
        "S0189.02",
        "bInventoryCalcWeaponDamageUseAmmoFixProjectile",
        Row::at(AI_FIXES, 24),
    ),
    (
        "S0199.01",
        "bMiscCrashFixesDialogueNoPackage",
        Row::at(AI_FIXES, 10),
    ),
    (
        "S0204",
        "bKillCommandDuplicateRewardXpFix",
        Row::at(AI_FIXES, 37),
    ),
    ("S0205", "bEnduranceHealthRestoreFix", Row::at(AI_FIXES, 2)),
    ("S0206", "bCreatureDRFix", Row::at(AI_FIXES, 3)),
    (
        "S0207",
        "bNonHitscanProjectileSplashFix",
        Row::at(AI_FIXES, 4),
    ),
    (
        "S0209",
        "bPreventAutoVanityModeIfSpecialIdlesArePlaying",
        Row::at(AI_FIXES, 46),
    ),
    (
        "S0210",
        "bFixSetIsFiringAutomaticWeaponNotClearing",
        Row::at(AI_FIXES, 47),
    ),
    ("S0211", "bThrowableRotationFix", Row::at(AI_FIXES, 17)),
    (
        "S0212",
        "bPlayerWaitDoubleHealingFix",
        Row::at(AI_FIXES, 48),
    ),
    ("S0214", "bCombatItemCooldownFix", Row::at(AI_FIXES, 14)),
    ("S0217", "bRagdollControllerUpdateFix", Row::at(AI_FIXES, 5)),
    ("S0221", "bDeathAnimFix", Row::at(AI_FIXES, 6)),
    (
        "S0224",
        "bPreventJumpingWhenFastTraveling",
        Row::at(AI_FIXES, 49),
    ),
    ("S0228", "bPatrolPackageFix", Row::at(AI_FIXES, 15)),
    ("S0229", "bArmorUpdateFix", Row::at(AI_FIXES, 7)),
    ("S0230", "bDoubleSoundFix", Row::at(AI_FIXES, 18)),
    ("S0241", "bActorGetHeadingTargetFix", Row::at(AI_FIXES, 38)),
    ("S0660", "bFixExplosionPushDirection", Row::at(AI_FIXES, 39)),
    (
        "S0664",
        "bFixKillChallengeSourceWeapon",
        Row::at(AI_FIXES, 40),
    ),
    (
        "S0782",
        "bShootableNonPlayableExplosives",
        Row::at(AI_FIXES, 41),
    ),
];
