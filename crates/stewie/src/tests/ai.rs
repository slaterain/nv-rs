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

/// Share 2's rows' plan numbers, off keys and bits (`rows::ai::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[
    (
        "S0147",
        "bFixCrippledLimbAnimsNotUpdatingIfLimbsFixed",
        Row::at(AI_FIXES, 0),
    ),
    (
        "S0159",
        "bFixInfiniteNPCLoopingReloads",
        Row::at(AI_FIXES, 1),
    ),
    (
        "S0180",
        "bMineDisarmedLightLoadingFix",
        Row::at(AI_FIXES, 8),
    ),
    ("S0205", "bEnduranceHealthRestoreFix", Row::at(AI_FIXES, 2)),
    ("S0206", "bCreatureDRFix", Row::at(AI_FIXES, 3)),
    (
        "S0207",
        "bNonHitscanProjectileSplashFix",
        Row::at(AI_FIXES, 4),
    ),
    ("S0217", "bRagdollControllerUpdateFix", Row::at(AI_FIXES, 5)),
    ("S0221", "bDeathAnimFix", Row::at(AI_FIXES, 6)),
    ("S0229", "bArmorUpdateFix", Row::at(AI_FIXES, 7)),
];
