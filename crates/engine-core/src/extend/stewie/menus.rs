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
/// S0022.07 `MenuFixes` (`fn_007d52a0`, the start menu's input, at `0x7D5365`): the controls
/// menu's remap waits one frame after it opens (Tweaks' debounce), so the key that opened it is
/// not taken as the new key.
pub const START_MENU_REMAP_DEBOUNCE: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 21);
/// S0022.11 `MenuFixes` (`LevelUpMenu::DoGamepad` `00785ed0` at `0x785F33` and `0x785FAB`,
/// `CharGenMenu::DoGamepad` `00754150` at `0x7541C8`, `StartMenu::DoGamepad` `007cefc0` at
/// `0x7CF387`): the Left and Right keys set the interface's keyboard-selected tile unless the
/// mouse moved.
pub const LEVEL_UP_ARROWS_SELECT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 22);
/// S0022.28 `MenuFixes` (`fn_00739090`, blackjack's draw, at `0x739135`): a card dealt with
/// neutral luck takes 1 off the shoe's count, not 2.
pub const BLACKJACK_ONE_CARD_A_DEAL: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 23);
/// S0022.29 `MenuFixes` (`BlackJackMenu::Shuffle` `007399d0` at `0x7399E3`,
/// `BlackJackMenu::InitCasinoData` `00731d70` at `0x732456`): the shoe's count starts at
/// 52 x decks - 1.
pub const BLACKJACK_SHOE_COUNT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 24);
/// S0035 `patchUnsetQuieterRadioWhenForceClosingDialogueMenu` (`fn_00761960` at `0x7619CB`,
/// `DialogMenu::ServiceStart` `00763ff0` at `0x764090`, `ServiceEnd` `007640a0` at `0x7641E5`):
/// the radio's "in dialogue" byte is cleared when the dialogue menu goes and while a menu opened
/// from it is up.
pub const DIALOG_RADIO_VOLUME_RESET: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 25);
/// S0088 `fixItemModMenuUnEquipBug` (`ItemModMenu::InitItemModMenu` `00784710` at `0x7847A8`,
/// `ItemModMenu::DoClick` `007838a0` at `0x783A7E` and `0x783A35`): the weapon mod menu no longer
/// unequips the worn weapon on opening and equips it again on leaving; fitting a mod to the worn
/// weapon unequips and equips it once.
pub const ITEM_MOD_MENU_KEEPS_EQUIPPED: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 26);
/// S0110 `patchFixStuckInAddSPECIALPointsIfAtMaxAlready` (`CharGenMenu::Create` `00753420` at
/// `0x7535FD`): the SPECIAL points to give are capped by what the stats can still take.
pub const CHARGEN_SPECIAL_POINTS_CAPPED: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 27);
/// S0154 `preventLoadingDlcPopup` (`fn_007d5830` at `0x7D59C7`, `fn_007d69f0` at `0x7D6AA3` and
/// `0x7D6AA8`): no "loading DLC" message box at start, and the start's messages stay open.
pub const NO_LOADING_DLC_POPUP: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 28);
/// S0208 `patchFixRadioVolumeResetWhenUsingSomeHolotapes` (`MapMenu::UpdatePlayingSounds`
/// `0079a660` at `0x79A750`): the map's sound update leaves the radio's volume alone.
pub const MAP_KEEPS_RADIO_VOLUME: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 29);
/// S0883 `MoreDescriptiveSettingsSaveError` (`fn_007d6d70` at `0x7D6E50`): the settings save
/// failure message names the ini file and why.
pub const SETTINGS_SAVE_ERROR_NAMES_FILE: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 30);
/// S0022.08 `MenuFixes` (`RecipeMenu`'s vtable slot `0x10704B4`, reached from
/// `InterfaceManager::DoWheelMove` `00718080`): the mouse wheel over the crafting menu's category
/// title changes the category.
pub const RECIPE_WHEEL_CHANGES_CATEGORY: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 31);
/// S0022.22 `MenuFixes` (`fn_007721c0`, the HUD's weapon part, at `0x77268F` and `0x7726AC`): the
/// low-condition label's flash runs by the frame's time, the same at any frame rate.
pub const HUD_CONDITION_FLASH_FRAME_RATE: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 32);
/// S0022.35 `MenuFixes` (`HUDMainMenu::UpdateCompass` `00779070` at `0x7792F9` and `0x779F1F`): the
/// compass's markers take the compass window's alpha.
pub const HUD_COMPASS_FOLLOWS_ITS_ALPHA: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 33);
/// S0022.42 `MenuFixes` (`HUDMainMenu::DoIdle` `00770430` at `0x770E78`): the enemy health bar no
/// longer waits for the action prompt to be hidden.
pub const HUD_HEALTH_BAR_NOT_TIED_TO_PROMPT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 34);
/// S0038 `fixRockItLauncherProjectileDisplay` (`HUDMainMenu::SetInfoForRef` `00775a00` at
/// `0x77647F` and `0x777D55`): a Rock-It Launcher projectile shows its item and the Take prompt.
pub const ROCK_IT_PROJECTILE_PROMPT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 35);
/// S0072 `patchFixHideHealthbarSetting` (`HUDMainMenu::SetTargetType` `00772d10` at `0x772D36`): the
/// health bar settings' value words hide the enemy health bar.
pub const HEALTH_BAR_SETTING_HIDES: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 36);
/// S0120 `patchFixCndArrowsIgnoringHUDFlags` (`fn_007721c0` at `0x7725D0`): the condition arrows
/// follow the HUD's action points flag on a weapon change.
pub const HUD_CONDITION_ARROWS_FLAGS: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 37);
/// S0197 `CrosshairProjectileNameFix` (`HUDMainMenu::SetInfoForRef` at `0x776EE2`): a projectile
/// under the crosshair is named after its weapon only for thrown weapons.
pub const CROSSHAIR_THROWN_WEAPON_NAME: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 38);
/// S0022.04 `MenuFixes` (`StartMenu::DoEnter` `007cf7e0` at `0x7CF85B`): the save/load list shows
/// the highlighted save's picture whether or not the mouse is over the list.
pub const SAVE_PICTURE_WITHOUT_HOVER: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 39);
/// S0022.23 `MenuFixes` (`VATSMenu::UpdateAmmo` `007f28d0` at `0x7F2E30`): the VATS condition bar
/// measures against the weapon's largest health with its condition mods.
pub const VATS_CONDITION_WITH_HEALTH_MODS: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 40);
/// S0022.24 `MenuFixes` (`VATSMenu::UpdateAmmo` at `0x7F2DCE`): the VATS condition and ammo labels
/// show for weapons without ammo too.
pub const VATS_CONDITION_WITHOUT_AMMO: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 41);
/// S0022.55 `MenuFixes` (`VATSMenu::DoIdle` `007ec810` at `0x7ED119`): the VATS health meter keeps
/// at least one tick.
pub const VATS_HEALTH_AT_LEAST_ONE_TICK: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 42);
/// S0020 `patchSaveINIChangesWhenClosingStartMenuAndSetXPBarQuestAndMessagesVisible`
/// (`StartMenu`'s deleting destructor `007cb510` at `0x7CB51A`): changed settings are saved when
/// the start menu closes, and the HUD's XP bar, quest and messages are shown again.
pub const START_MENU_CLOSE_SAVES_SETTINGS: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 43);
/// S0047 `patchFixVATSAutoWeaponMultiAmmoUseBug` (`VATSMenu::GetShotBurstCount` `007f5050` at
/// `0x7F5177`): an automatic weapon's VATS burst counts its ammo use per shot.
pub const VATS_BURST_TIMES_AMMO_USE: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 44);
/// S0066 `patchFixPipboyDisplayedChemDurationIfPerkModified` (`StatusDataList::Register`
/// `007e0b20` at `0x7E0C1F`): the Pip-Boy's effect durations count the player's chem duration
/// perks.
pub const PIPBOY_CHEM_DURATION_PERKS: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 45);
/// S0144 `patchFixVATSChargingReloadAPCostWithNoAmmo` (`VATSMenu::DoIdle` at `0x7EDB33`): no
/// reload action points are charged without ammo to reload with or in god mode.
pub const VATS_NO_RELOAD_COST_WITHOUT_AMMO: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 46);
/// S0160 `MessageMenuFontWidthFix` (`MessageMenu::LoadNextMessage` `007a92e0` at `0x7A95D2`,
/// `0x7A9850`, `0x7A9B74`): message boxes measure their texts in the texts' own fonts.
pub const MESSAGE_MENU_TEXT_FONT: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 47);
/// S0022.37 `MenuFixes` (`StartMenu::ChooseHelp` `007d0770` at `0x7D08C6` and `0x7D08D3`): the
/// pause menu's Help shows the caravan, roulette, blackjack and slots helps in those games.
pub const MINIGAME_HELP_IN_PAUSE_MENU: Row = Row::share(MENUS_FIXES, MENUS_FIXES_END, 48);
