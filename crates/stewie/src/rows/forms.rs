//! Share 1b's rows (forms, cells, scripts, saves and sound; their bits in
//! `engine_core::extend::stewie`'s `forms.rs`), in PLAN.md's order. Only share 1b adds rows here.

use super::Row;
use engine_core::extend::stewie;
use engine_core::prelude::*;

/// Share 1b's rows built.
pub const ROWS: &[Row] = &[
    Row {
        bit: stewie::PRID_NO_DEBUG_TEXT,
        function: "patchPridDontDuplicateLine",
        plan: "S0017",
        key: "bPridDontDuplicateLine",
        tweaks: None,
        summary: "the console's prid no longer also draws the reference's name as debug text on the screen",
    },
    Row {
        bit: stewie::PRINTS_ONLY_FROM_CONSOLE,
        function: "patchMuteSomeCommandsUnlessCalledFromConsole",
        plan: "S0056",
        key: "bMuteSomeCommandsUnlessCalledFromConsole",
        tweaks: None,
        summary: "GetVATSValue, StopMagicShaderVisuals, PlayerSpellBook and IsImageSpaceActive print their console line only when typed in the console, not when a mod's script calls them",
    },
    Row {
        bit: stewie::ADD_REMOVE_ITEM_COUNT_OPTIONAL,
        function: "patchNoNeededCountForAddRemoveItemCommands",
        plan: "S0099",
        key: "bNoNeededCountForAddRemoveItemCommands",
        tweaks: None,
        summary: "AddItem and RemoveItem may leave out the count (one item); a script line the game refused before now compiles",
    },
    Row {
        bit: stewie::CENTER_ON_CELL_FORM_ID,
        function: "patchCenterOnCellAcceptFormIDs",
        plan: "S0101",
        key: "bCenterOnCellAcceptFormIDs",
        tweaks: None,
        summary: "the console's CenterOnCell (coc) also takes a cell's form id in hex",
    },
    Row {
        bit: stewie::NAV_MESH_ROOT_NODE,
        function: "ToggleNavMeshFix",
        plan: "S0161",
        key: "bToggleNavMeshFix",
        tweaks: None,
        summary: "the console's ToggleNavMesh shows the navigation meshes (their display node is made when missing)",
    },
    Row {
        bit: stewie::TEST_ALL_CELLS_GODMODE,
        function: "TestAllCellsEnablesGodmode",
        plan: "S0194",
        key: "bTestAllCellsEnablesGodmode",
        tweaks: None,
        summary: "the console's TestAllCells also turns god mode on",
    },
    Row {
        bit: stewie::LOCK_COMMAND,
        function: "LockCommandFixes",
        plan: "S0213",
        key: "bLockCommandFixes",
        tweaks: None,
        summary: "Lock 0, and a plain Lock with no level, set the lock to very easy instead of keeping its level; the console line names the level",
    },
    Row {
        bit: stewie::OCCLUSION_PLANE_KEPT,
        function: "PlaceOcclusionPlaneFix",
        plan: "S0236",
        key: "bPlaceOcclusionPlaneFix",
        tweaks: None,
        summary: "the console's PlaceOcclusionPlane no longer frees the plane it just placed (a crash)",
    },
];

/// The data writes share 1b's rows make at the install (Tweaks writes them once, when its DLL
/// loads), after the bits are set: S0099 makes AddItem's and RemoveItem's count parameter
/// optional (`countParam->isOptional = 1`). The exe's data is always there in a game; an engine
/// made without it (the unit tests') gets no write.
pub fn write_data(e: &mut Engine) {
    let at = stewie::COUNT_PARAMETER_OPTIONAL;
    if e.stewie(stewie::ADD_REMOVE_ITEM_COUNT_OPTIONAL) && e.mem.is_mapped(at) {
        e.mem.set_u32(at, 1);
    }
}
