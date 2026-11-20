//! Share 3's rows (menus and the HUD; their bits in `engine_core::extend::stewie::menus`), in
//! PLAN.md's order. Only share 3 adds rows here.

use super::Row;
use engine_core::extend::stewie;

/// Share 3's rows built.
pub const ROWS: &[Row] = &[
    Row {
        bit: stewie::MAP_QUEST_KEEPS_SCROLL,
        function: "MenuFixes",
        plan: "S0022.09",
        key: "bMenuFixesQuestScroll",
        tweaks: None,
        summary: "selecting a quest on the Pip-Boy map keeps the quest list where it is when the mouse has moved",
    },
    Row {
        bit: stewie::LOCKPICK_DEBUG_HEALTH,
        function: "MenuFixes",
        plan: "S0022.13",
        key: "bMenuFixesDebugLockPickHealth",
        tweaks: None,
        summary: "the debug lock pick menu shows the pick's real health, not always 100%",
    },
    Row {
        bit: stewie::MAP_DRAG_NO_UNDISCOVERED_MESSAGE,
        function: "MenuFixes",
        plan: "S0022.19",
        key: "bMenuFixesMapDragUndiscovered",
        tweaks: None,
        summary: "dragging the local map over an undiscovered place no longer says you cannot fast travel there",
    },
    Row {
        bit: stewie::MAP_PAD_MARKER_WHILE_MOVING,
        function: "MenuFixes",
        plan: "S0022.25",
        key: "bMenuFixesMapPadMarker",
        tweaks: None,
        summary: "the pad's X places a map marker while the stick moves the map",
    },
    Row {
        bit: stewie::MAP_COMPANION_LABEL_CLIPS,
        function: "MenuFixes",
        plan: "S0022.26",
        key: "bMenuFixesCompanionLabelClips",
        tweaks: None,
        summary: "companion names on the world map are clipped to the map from the start",
    },
    Row {
        bit: stewie::ROULETTE_ZERO_BET_TEXT,
        function: "MenuFixes",
        plan: "S0022.27",
        key: "bMenuFixesRouletteZeroBetText",
        tweaks: None,
        summary: "the roulette bets that include 0 or 00 show their caption and numbers in order",
    },
    Row {
        bit: stewie::MAP_PAD_SELECTION_CENTRED,
        function: "MenuFixes",
        plan: "S0022.31",
        key: "bMenuFixesMapPadSelectionCentred",
        tweaks: None,
        summary: "switching to the pad centres the map's marker selection box",
    },
    Row {
        bit: stewie::MAP_COMPANION_MARKERS_RESET,
        function: "MenuFixes",
        plan: "S0022.41",
        key: "bMenuFixesCompanionMarkersReset",
        tweaks: None,
        summary: "stale companion markers are cleared when the map refreshes",
    },
    Row {
        bit: stewie::CARAVAN_PAD_NOT_WITH_CONSOLE,
        function: "MenuFixes",
        plan: "S0022.46",
        key: "bMenuFixesCaravanPadConsole",
        tweaks: None,
        summary: "the pad does not move the caravan selection while the console is open",
    },
    Row {
        bit: stewie::CARAVAN_SINGLE_PAYOUT,
        function: "CaravanDoublePayoutFix",
        plan: "S0092",
        key: "bCaravanDoublePayoutFix",
        tweaks: None,
        summary: "changes payouts: a won caravan game pays the ante once (the game paid it twice), with a message for each payment",
    },
    Row {
        bit: stewie::ROULETTE_ZERO_WINS,
        function: "RouletteMenuFixes",
        plan: "S0196.04",
        key: "bRouletteMenuFixesZeroWins",
        tweaks: None,
        summary: "changes payouts: a roulette bet on a tile that includes 0 wins when the ball lands on 0",
    },
    Row {
        bit: stewie::ROULETTE_CUT_BET_VALUE,
        function: "RouletteMenuFixes",
        plan: "S0196.05",
        key: "bRouletteMenuFixesCutBetValue",
        tweaks: None,
        summary: "a roulette bet cut to the chips left updates the bet's number, not its caption",
    },
];
