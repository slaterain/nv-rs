//! Share 2's unit tests (AI, actors and the player). Only share 2 edits this file.

use engine_core::extend::stewie::{Row, AI_FIXES};

/// Share 2's rows' plan numbers, off keys and bits (`rows::ai::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[
    ("bMineDisarmedLightLoadingFix", Row::at(AI_FIXES, 8)),
    ("bNonHitscanProjectileSplashFix", Row::at(AI_FIXES, 4)),
    (
        "bFixCrippledLimbAnimsNotUpdatingIfLimbsFixed",
        Row::at(AI_FIXES, 0),
    ),
    ("bFixInfiniteNPCLoopingReloads", Row::at(AI_FIXES, 1)),
    "S0147",
    "S0159",
    "S0180",
    ("S0205", "bEnduranceHealthRestoreFix", Row::at(AI_FIXES, 2)),
    ("S0206", "bCreatureDRFix", Row::at(AI_FIXES, 3)),
    "S0207",
    ("S0217", "bRagdollControllerUpdateFix", Row::at(AI_FIXES, 5)),
    ("S0221", "bDeathAnimFix", Row::at(AI_FIXES, 6)),
    ("S0229", "bArmorUpdateFix", Row::at(AI_FIXES, 7)),
];
