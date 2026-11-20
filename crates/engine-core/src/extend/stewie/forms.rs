//! Share 1b's rows (forms, cells, scripts, saves and sound: forms, perks, weapons, script
//! functions, extra data, inventory changes, references, cells, models, saves, audio media and the
//! radio). Only share 1b adds rows here (see `super::world` for the rule). A row is
//! `Row::share(FORMS_FIXES, FORMS_FIXES_END, n)` (or the tweaks' pair), which does not compile past
//! the share's last bit.

/// Share 1b's fix rows: bits `214..300`.
pub const FORMS_FIXES: u16 = super::world::WORLD_FIXES_END;
/// One past share 1b's last fix bit.
pub const FORMS_FIXES_END: u16 = 300;
/// Share 1b's tweak rows: bits `704..768`.
pub const FORMS_TWEAKS: u16 = super::world::WORLD_TWEAKS_END;
/// One past share 1b's last tweak bit.
pub const FORMS_TWEAKS_END: u16 = 768;

use super::Row;

const fn fix(n: u16) -> Row {
    Row::share(FORMS_FIXES, FORMS_FIXES_END, n)
}

const fn tweak(n: u16) -> Row {
    Row::share(FORMS_TWEAKS, FORMS_TWEAKS_END, n)
}

// ---- The script commands (group 1) ----

/// S0017 `patchPridDontDuplicateLine` (`Script::PickRefByIDFunction` `005bd410`, `0x5BD546`):
/// `prid` no longer draws its reference's name as debug text at a row the game works out wrongly
/// (the call of `DebugText::Print`, `00a0f8b0`, replaced by `add esp, 0x20`); the reference is still
/// picked.
pub const PRID_NO_DEBUG_TEXT: Row = fix(0);
/// S0056 `patchMuteSomeCommandsUnlessCalledFromConsole` (`GetVATSValue` `0x5952AB`,
/// `StopMagicShaderVisuals` `0x5D21B8`/`0x5D21C7`, `PlayerSpellBook` `0x5B5CA5`, `IsImageSpaceActive`
/// `0x5DE5C9`/`0x5DE5D8`): those commands print their console line only while a console line runs
/// (`IsConsoleMode`), not when a mod's script calls them.
pub const PRINTS_ONLY_FROM_CONSOLE: Row = fix(1);
/// S0161 `ToggleNavMeshFix` (`Script::ToggleNavMeshFunction` `005bcfe0`, `0x5BD030`): the navigation
/// mesh display's root node (`0x11D6E5C`) is made ("NavMeshRootNode") and put under the scene
/// graph when it is missing, so `ToggleNavMesh` draws the meshes.
pub const NAV_MESH_ROOT_NODE: Row = fix(2);
/// S0213 `LockCommandFixes` (`Script::LockFunction` `005cbf80`, `0x5CC08B`, `0x5CC10A`): `Lock 0`,
/// and a plain `Lock` with no level, set the lock level 0 (very easy) instead of keeping the old
/// level, and the console line names the level ("Locked %s with lock level %d (%s)").
pub const LOCK_COMMAND: Row = fix(3);
/// S0236 `PlaceOcclusionPlaneFix` (`005bb9e0`, `0x5BBBA6`, `0x5BBBB5`, `0x5BBBE4`): the plane
/// `PlaceOcclusionPlane` makes goes into the occluder list without the `NiPointer` that freed it at
/// once (a use after free, the game's crash).
pub const OCCLUSION_PLANE_KEPT: Row = fix(4);

/// S0099 `patchNoNeededCountForAddRemoveItemCommands` (the parameter table `0x118D3F0`'s optional
/// word; `005b4be0` `0x5B4C0F`, `005b4e90` `0x5B4EC6`): `AddItem` and `RemoveItem` compile without
/// a count and then add or remove one.
pub const ADD_REMOVE_ITEM_COUNT_OPTIONAL: Row = tweak(0);
/// S0101 `patchCenterOnCellAcceptFormIDs` (`Script::CenterOnCellFunction` `005bd5e0`, `0x5BD63D`):
/// `CenterOnCell` (`coc`) also takes a cell's form id in hex when no cell has that editor id.
pub const CENTER_ON_CELL_FORM_ID: Row = tweak(1);
/// S0194 `TestAllCellsEnablesGodmode` (`Script::TestAllCellsFunction` `005b71b0`, `0x5B7212`):
/// `TestAllCells` turns god mode on (`0x11E07BA`) once it has started the test (at the call asking
/// whether the test runs).
pub const TEST_ALL_CELLS_GODMODE: Row = tweak(2);

/// S0099's parameter: the `ParamInfo` `0x118D3F0` (AddItem's and RemoveItem's count, a type
/// string, a type and the optional word at `+8`).
pub const COUNT_PARAMETER_OPTIONAL: u32 = 0x0118_d3f8;
