//! Share 1b's unit tests (forms, cells, scripts, saves and sound). Only share 1b edits this file.

use engine_core::extend::stewie::{Row, FORMS_FIXES, FORMS_TWEAKS};

/// Share 1b's rows' plan numbers, off keys and bits (`rows::forms::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[
    ("S0017", "bPridDontDuplicateLine", Row::at(FORMS_FIXES, 0)),
    (
        "S0056",
        "bMuteSomeCommandsUnlessCalledFromConsole",
        Row::at(FORMS_FIXES, 1),
    ),
    (
        "S0093.01",
        "bPerkMenuDescriptionTypoFixNoDashes",
        Row::at(FORMS_FIXES, 5),
    ),
    (
        "S0093.02",
        "bPerkMenuDescriptionTypoFixNoEmptyComma",
        Row::at(FORMS_FIXES, 6),
    ),
    (
        "S0093.03",
        "bPerkMenuDescriptionTypoFixIndexByOne",
        Row::at(FORMS_FIXES, 7),
    ),
    (
        "S0093.04",
        "bPerkMenuDescriptionTypoFixSkipOther",
        Row::at(FORMS_FIXES, 8),
    ),
    (
        "S0093.05",
        "bPerkMenuDescriptionTypoFixCommas",
        Row::at(FORMS_FIXES, 9),
    ),
    (
        "S0093.06",
        "bPerkMenuDescriptionTypoFixPerkNotOr",
        Row::at(FORMS_FIXES, 10),
    ),
    (
        "S0099",
        "bNoNeededCountForAddRemoveItemCommands",
        Row::at(FORMS_TWEAKS, 0),
    ),
    (
        "S0101",
        "bCenterOnCellAcceptFormIDs",
        Row::at(FORMS_TWEAKS, 1),
    ),
    ("S0161", "bToggleNavMeshFix", Row::at(FORMS_FIXES, 2)),
    (
        "S0194",
        "bTestAllCellsEnablesGodmode",
        Row::at(FORMS_TWEAKS, 2),
    ),
    ("S0213", "bLockCommandFixes", Row::at(FORMS_FIXES, 3)),
    ("S0236", "bPlaceOcclusionPlaneFix", Row::at(FORMS_FIXES, 4)),
];

/// S0099's data write: with the row on, the install makes the count parameter optional; with it
/// off, the word is left as the exe has it.
#[test]
fn s0099_the_install_makes_the_count_optional() {
    use engine_core::extend::stewie;
    use engine_core::prelude::*;
    for on in [false, true] {
        let mut e = Engine::new();
        let at = stewie::COUNT_PARAMETER_OPTIONAL;
        e.map(at & !0xfff, 0x1000);
        e.mem.set_u32(at, 0);
        e.set_stewie(stewie::ADD_REMOVE_ITEM_COUNT_OPTIONAL, on);
        crate::rows::forms::write_data(&mut e);
        assert_eq!(e.mem.u32(at), u32::from(on), "row {on}");
    }
}
