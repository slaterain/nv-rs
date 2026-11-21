//! Share 3's rows (menus and the HUD: every menu but the interface manager's and the console's,
//! tiles, fonts and the controller's input). Only share 3 adds rows here (see `super::world` for
//! the rule). A row is `Row::share(MENUS_FIXES, MENUS_FIXES_END, n)` (or the tweaks' pair), which
//! does not compile past the share's last bit.

/// Share 3's fix rows: bits `470..640`.
pub const MENUS_FIXES: u16 = super::ai::AI_FIXES_END;
/// One past share 3's last fix bit.
pub const MENUS_FIXES_END: u16 = super::VNV_TWEAKS;
/// Share 3's tweak rows: bits `896..1024`.
pub const MENUS_TWEAKS: u16 = super::ai::AI_TWEAKS_END;
/// One past share 3's last tweak bit.
pub const MENUS_TWEAKS_END: u16 = super::OTHER_TWEAKS;

use super::Row;

/// S0022.09 `MenuFixes` (`MapMenu::UpdateQuests` `0079b8e0` at `0x79BBD0`): selecting a quest
/// keeps the list's scroll position while the mouse moved.
pub const MAP_QUEST_KEEPS_SCROLL: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 0);
/// S0022.13 `MenuFixes` (`LockPickMenu::Create` `0078db00` at `0x78DEC2`): the debug lock pick
/// menu's pick health caption shows the pick's health, not 100%.
pub const LOCKPICK_DEBUG_HEALTH: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 1);
/// S0022.19 `MenuFixes` (`MapMenu::DoClick` `00796fd0` at `0x797FCF` and `0x797FFE`): a click
/// dragged over an undiscovered place on the local map no longer shows
/// `sNoFastTravelUndiscovered`.
pub const MAP_DRAG_NO_UNDISCOVERED_MESSAGE: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 2);
/// S0022.25 `MenuFixes` (`MapMenu::DoGamepad` `00799790` at `0x799A14`): the pad's X places a
/// map marker while the stick moves the map.
pub const MAP_PAD_MARKER_WHILE_MOVING: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 3);
/// S0022.26 `MenuFixes` (`MapMenu::AddCompanionMarkers` `0079ea30` at `0x79F1D7`): the
/// companions' name labels on the map get their `clips` value when first made.
pub const MAP_COMPANION_LABEL_CLIPS: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 4);
/// S0022.27 `MenuFixes` (`RouletteMenu::PrepareBetTiles` `007b9c40` at `0x7BA10F`, `0x7BA208`
/// and `0x7BA2CB`): the three bets with 0 or 00 show their names and odds in order.
pub const ROULETTE_ZERO_BET_TEXT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 5);
/// S0022.31 `MenuFixes` (`MapMenu::DoControllerSwitch` `0079a610` at `0x79A636` and
/// `0x79A652`): switching to the pad centres the map's selection rectangle.
pub const MAP_PAD_SELECTION_CENTRED: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 6);
/// S0022.41 `MenuFixes` (`MapMenu::AddCompanionMarkers` at `0x79EA73`): the companions' marker
/// tiles are taken off the map before the markers are made again.
pub const MAP_COMPANION_MARKERS_RESET: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 7);
/// S0022.46 `MenuFixes` (`CaravanMenu::DoIdle` `00741500` at `0x741683`): the pad does not move
/// the caravan selection while the console is open.
pub const CARAVAN_PAD_NOT_WITH_CONSOLE: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 8);
/// S0196.04 `RouletteMenuFixes` (`RouletteMenu::DoIdle` `007bd2a0` at `0x7BDCE4`): a bet on a
/// tile that includes 0 wins when the ball lands on 0.
pub const ROULETTE_ZERO_WINS: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 9);
/// S0196.05 `RouletteMenuFixes` (`RouletteMenu::DoIdle` at `0x7BEB9B` and `0x7BEBBD`): a bet cut
/// to the chips left goes to the bet's value, not over the `sCurrentBet` caption.
pub const ROULETTE_CUT_BET_VALUE: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 10);
/// S0092 `CaravanDoublePayoutFix` (the caravan payout `0074cc70` at `0x74CDB2` and `0x74CE21`): a
/// won game pays the ante once (the exe moves the won items and caps to the player, then gives
/// them again from the barter container).
pub const CARAVAN_SINGLE_PAYOUT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 11);
/// S0022.12 `MenuFixes` (`fn_00728da0`, the recipe menu's item information, at `0x7296E0`): the
/// condition tile's notch sits in the armour place for armour (`user6` 0) and at the crafted
/// condition (`user5`), not always in the weapon's place.
pub const RECIPE_CONDITION_NOTCH: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 12);
/// S0022.34 `MenuFixes` (`InventoryMenu::DoGamepad` `00782190`, its switch slot `0x7823B0`): the
/// pad's and Page Up's mod menu button does nothing while the mod button cannot be used
/// (`target` 0).
pub const WEAPON_MOD_MENU_NEEDS_TARGET: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 13);
/// S0022.52 `MenuFixes` (`RepairMenu::DoEnter` `007b6120` at `0x7B6514`): the repair menu's stat
/// lines follow the broken item's type, not the type of the item repaired with.
pub const REPAIR_MENU_BROKEN_ITEM_TYPE: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 14);
/// S0033 `patchRepairServicesMenuRemembersScrollPosition` (`RepairServicesMenu::DoClick`
/// `007b7d80` at `0x7B7E86` and `0x7B7F4E`): the vendor's repair list keeps its scroll position
/// after Repair and Repair All.
pub const REPAIR_SERVICES_KEEPS_SCROLL: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 15);
/// S0044 `patchFixRecipeMenuArmorIcons` (`RecipeMenu::DoEnter` `00727b10` at `0x728378`): an
/// armour recipe shows the armour's inventory icon, not its message (glow) icon.
pub const RECIPE_ARMOUR_ICONS: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 16);
/// S0051 `patchUpdateDRDTWhenUsingVendorRepair` (`RepairServicesMenu::RepairItem` `007b7f70` at
/// `0x7B8083`): a vendor's repair recounts the player's DR and DT.
pub const VENDOR_REPAIR_RESETS_DR_DT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 17);
/// S0070 `patchFixBookAndAmmoHotkeyIcons` (`InventoryMenu` `00780ff0` at `0x78149C`,
/// `HotkeyWheel::UpdateHotkeyList` `007017b0` at `0x701861`): a hotkeyed book shows the chems
/// icon and hotkeyed ammo the big guns icon, not none.
pub const BOOK_AND_AMMO_HOTKEY_ICONS: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 18);
/// S0071 `patchFixRecipeNegativeIngredientCount` (`RecipeMenu::DoEnter` at `0x727F92`): an
/// ingredient count below 0 shows 0.
pub const RECIPE_NO_NEGATIVE_COUNT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 19);
/// S0200 `RecipeMenuRefreshFix` (`fn_00728a30` at `0x728A4E`, `fn_00727680` at `0x7276A6` and
/// `0x7278A9`): the recipe list refilled after crafting keeps the menu's category, its filter
/// and its scroll position.
pub const RECIPE_REFRESH_KEEPS_CATEGORY: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 20);
