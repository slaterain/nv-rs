//! Group GB's seams and wraps (step 3.9: UI: item lists, repair, pad dead zones). Only group GB's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GB". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GB",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // GB-3's fixes (JIP's `InitGamePatches`), in with the game-patch row:
    // G100 (`WriteRelCall(0x72880D, IsDisposableWeaponHook)`): the recipe menu's crafted weapon,
    // disposable by its type.
    row(0x0072_880d),
    // G13 (`SAFE_WRITE_BUF(0x728E10, ..)`, `SafeWrite32(0x729616, 0x1030018)`): the recipe menu's
    // item value without the condition scaling, its weight below 1 with two decimals.
    row(0x0072_8e10),
    // G68 (`WriteRelJump(0x7294DC, RecipeMenuAmmoEffectsHook)`): ammunition's effects text.
    row(0x0072_94dc),
    // G13's second place (the weight's format).
    row(0x0072_9616),
    // G56 (`WriteRelCall(0x72D661, InitBarterMenuHook)`): `BarterMenu::Create` titles the
    // vendor's tab before the first list fill.
    row(0x0072_d661),
    // O9 (`bQttSelectShortKeys`, `HOOK_INIT_LIST(QuantitySelect, ..)`): the barter menu's
    // quantity test, Ctrl takes one, Shift all, without the quantity menu.
    row(0x0072_d8b4),
    // G82 (`WritePushRetRelJump(0x72F33D, 0x72F37F, ConstructItemEntryNameHook)`, with
    // `SAFE_WRITE_BUF(0x72F337, ..)` before it): `BarterMenu::InitFunc`'s item name.
    row(0x0072_f33d),
    // G64 (`WriteRelCall(0x73005C, BarterSellFixHook)`): `BarterMenu::CompleteTransaction`
    // splits a vendor's caps over 32,767 into stacks before taking them.
    row(0x0073_005c),
    // D04, the pad dead zones (JIP's `DeferredInit`: `SafeWrite32` of its own doubles over the
    // operand of `FCOMP double ptr [0x010718F0]`): each seam answers the double to compare with.
    // `CaravanMenu::DoIdle` 00741500, the stick's x and y.
    row(0x0074_16d6),
    row(0x0074_1705),
    // O9: `ContainerMenu::DoClick`'s weightless-item test (`{0x75BF18, 0x63EB, 2, 0}`) and its
    // quantity test.
    row(0x0075_bf18),
    row(0x0075_bf97),
    // G82: `ContainerMenu::InitFunc`'s item name.
    row(0x0075_d1a5),
    // O9: `InventoryMenu::DoClick`'s quantity test; G82: `InventoryMenu::InitFunc`'s item name.
    row(0x0078_0b09),
    row(0x0078_2876),
    // `LockPickMenu::RotateLockpick` 007908f0.
    row(0x0079_0966),
    // `MapMenu::DoGamepad` 00799790, the zoom axis.
    row(0x0079_9861),
    // G87 (`RefreshMapMenuHook`, a jump at 0x79DF73): `MapMenu::AddMapIcons` 0079dbb0 skips
    // centring the map once when JIP's item-list refresh asked for it.
    row(0x0079_df73),
    // G69 (`WriteRelJump(0x7AC179, QttMenuEnableWheelHook)`): `QuantityMenu::DoIdle` takes the
    // mouse wheel (a fix).
    row(0x007a_c179),
    // `RouletteMenu::DoIdle` 007bd2a0, its three stick values.
    row(0x007b_d47d),
    row(0x007b_d4a0),
    row(0x007b_d4dd),
    // `PlayerCharacter::UpdateUFOCamera` 0094a8c0 (seams in group GP's file): the look axes
    // (right stick: JIP's `s_deadZoneRSg`), then the move axes (left stick).
    row(0x0094_a93c),
    row(0x0094_a95f),
    row(0x0094_ab59),
    row(0x0094_ab88),
    // G88 (`ProcessGradualSetFloatHook` in the two jump-table words JIP adds after
    // `Tile::UpdateFadeControls`' table, the type test widened to 6 at 0xA081A8): fade types 5
    // and 6.
    row(0x00a0_8718),
    row(0x00a0_871c),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
