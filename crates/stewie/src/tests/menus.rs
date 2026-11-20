//! Share 3's unit tests (menus and the HUD). Only share 3 edits this file.

use engine_core::extend::stewie::{self, Row};

/// Share 3's rows' plan numbers, off keys and bits (`rows::menus::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[
    ("S0022.09", "bMenuFixesQuestScroll", bit(0)),
    ("S0022.13", "bMenuFixesDebugLockPickHealth", bit(1)),
    ("S0022.19", "bMenuFixesMapDragUndiscovered", bit(2)),
    ("S0022.25", "bMenuFixesMapPadMarker", bit(3)),
    ("S0022.26", "bMenuFixesCompanionLabelClips", bit(4)),
    ("S0022.27", "bMenuFixesRouletteZeroBetText", bit(5)),
    ("S0022.31", "bMenuFixesMapPadSelectionCentred", bit(6)),
    ("S0022.41", "bMenuFixesCompanionMarkersReset", bit(7)),
    ("S0022.46", "bMenuFixesCaravanPadConsole", bit(8)),
    ("S0092", "bCaravanDoublePayoutFix", bit(11)),
    ("S0196.04", "bRouletteMenuFixesZeroWins", bit(9)),
    ("S0196.05", "bRouletteMenuFixesCutBetValue", bit(10)),
];

/// Share 3's fix bit `n` (`Row::share` over its range).
const fn bit(n: u16) -> Row {
    Row::share(stewie::MENUS_FIXES, stewie::MENUS_FIXES_END, n)
}
