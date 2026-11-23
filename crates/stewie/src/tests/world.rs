//! Share 1a's unit tests (the engines and the console). Only share 1a edits this file.

use engine_core::extend::stewie::{Row, WORLD_FIXES};

/// Share 1a's rows' plan numbers, off keys and bits (`rows::world::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
/// The bits were given in the order the rows were traced, not PLAN.md's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[
    (
        "S0016",
        "bAllowKeyboardPauseAndScreenshotWhileControllerConnected",
        Row::at(WORLD_FIXES, 26),
    ),
    (
        "S0021",
        "bSaveInventoryScrollPositionWhenClosingPipBoy",
        Row::at(WORLD_FIXES, 9),
    ),
    (
        "S0022.02",
        "bMenuFixesMenuShiftHeldNow",
        Row::at(WORLD_FIXES, 17),
    ),
    (
        "S0022.03",
        "bMenuFixesTextShiftHeldNow",
        Row::at(WORLD_FIXES, 10),
    ),
    (
        "S0022.05",
        "bMenuFixesArrowKeysOverMouse",
        Row::at(WORLD_FIXES, 11),
    ),
    (
        "S0022.10",
        "bMenuFixesBookCloseDebounce",
        Row::at(WORLD_FIXES, 18),
    ),
    (
        "S0022.18",
        "bMenuFixesPipboyHotkeysAfterText",
        Row::at(WORLD_FIXES, 19),
    ),
    (
        "S0022.32",
        "bMenuFixesGamepadRepeatCleared",
        Row::at(WORLD_FIXES, 12),
    ),
    (
        "S0022.38",
        "bMenuFixesThumbstickCursor",
        Row::at(WORLD_FIXES, 13),
    ),
    (
        "S0022.39",
        "bMenuFixesPadActivateClicks",
        Row::at(WORLD_FIXES, 20),
    ),
    (
        "S0022.40",
        "bMenuFixesConsolePadActivateRB",
        Row::at(WORLD_FIXES, 21),
    ),
    (
        "S0022.45",
        "bMenuFixesConsoleDpad",
        Row::at(WORLD_FIXES, 22),
    ),
    ("S0022.53", "bMenuFixesMenuL3R3", Row::at(WORLD_FIXES, 14)),
    (
        "S0022.54",
        "bMenuFixesBindBackslash",
        Row::at(WORLD_FIXES, 27),
    ),
    (
        "S0046",
        "bFixPipboyLightStayingOnWhenViewing",
        Row::at(WORLD_FIXES, 15),
    ),
    ("S0050", "bAddMissingLODWarning", Row::at(WORLD_FIXES, 1)),
    (
        "S0059",
        "bShowSelectedMeshPathInConsoleIfMissing",
        Row::at(WORLD_FIXES, 16),
    ),
    (
        "S0062",
        "bAllowNumbersInXMLButtons",
        Row::at(WORLD_FIXES, 23),
    ),
    (
        "S0065",
        "bFixMouseInBarterAndContainerMenusInsideTerminals",
        Row::at(WORLD_FIXES, 24),
    ),
    (
        "S0080",
        "bTestAllCellsNeedlessINIChecks",
        Row::at(WORLD_FIXES, 31),
    ),
    (
        "S0103",
        "bFixFullLODResettingWhenOpeningPipboy",
        Row::at(WORLD_FIXES, 30),
    ),
    (
        "S0128",
        "bPreventNoUnequipMessageForNPCs",
        Row::at(WORLD_FIXES, 28),
    ),
    (
        "S0131",
        "bFixMapMarkerReputationLookup",
        Row::at(WORLD_FIXES, 29),
    ),
    ("S0145", "bFixSunDirection", Row::at(WORLD_FIXES, 7)),
    ("S0193", "bNavMeshTeleportDoorFix", Row::at(WORLD_FIXES, 4)),
    ("S0216", "bStairMaterialFix", Row::at(WORLD_FIXES, 8)),
    ("S0235", "bNavMeshInfoResolveFix", Row::at(WORLD_FIXES, 2)),
    ("S0237", "bSwitchNodeBoundsFix", Row::at(WORLD_FIXES, 6)),
    (
        "S0238",
        "bSkinnedDecalSubtextureFix",
        Row::at(WORLD_FIXES, 5),
    ),
    (
        "S0242",
        "bNavMeshObstacleDoorTypeFix",
        Row::at(WORLD_FIXES, 3),
    ),
    ("S0984", "bMenuFixesDeferredInit", Row::at(WORLD_FIXES, 25)),
];
