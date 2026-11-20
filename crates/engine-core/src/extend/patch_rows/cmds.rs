//! The seams and wraps JIP LN NVSE's commands need (not its game patches): places where a routine
//! of JIP's that the game calls back reads the game's own state in a way the translation cannot
//! give it (a caller's stack frame, say), and the places where a command rewrites the game's code
//! (code bytes and jumps; list group 3a). Rows by address, owner `super::JIP`, step "jip-cmds"; the
//! handlers are the commands' (`jip::commands`), in with JIP's `INSTALLED`.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "jip-cmds",
    }
}

/// The commands' seams (by address).
pub(in super::super) const SITES: &[Entry] = &[
    // `InitMiniMap` (JIP MiniMap; answered once it has run): `TES::UpdateCurrentGridCell`
    // 00452580, the small-grid test (JIP's `SafeWrite16(0x452736, 0x7705)`: `cmp [eax], 5; ja` for
    // `cmp [eax], 3; jnz`).
    row(0x0045_2737),
    // `InitMiniMap`: the game's update of the cells' seen bits 00555c20 (JIP's
    // `SafeWrite8(0x555C20, 0xC3)`).
    row(0x0055_5c20),
    // `InitMiniMap`: the quest's targets 0060f110, before its first target (JIP's
    // `SetQuestTargetsHook` jump at 0x60F13A: a flag).
    row(0x0060_f13a),
    // `ToggleNoZPosReset`: a reference's reset onto the ground 00628f90, after its collidable
    // reference (0x629006): a flagged reference keeps its height.
    row(0x0062_9006),
    // `ToggleNoZPosReset`: `LoadedAreaBound::WarpActor` 006295e0, at its start (0x6295E9): a
    // flagged actor is not warped.
    row(0x0062_95e9),
    // `SetMoonTexture`: `Moon::Moon` 00634a70, its texture names (0x634B30; JIP's `InitMoonHook`
    // while its moon refresh runs).
    row(0x0063_4b30),
    // `EnableImprovedRecipeMenu`: `RecipeMenu::DoClick` 007274b0, its switch (the bound at
    // 0x7274E1, the byte map and the table; JIP's ids 7 and 15 to `RecipeMenuAcceptHook`).
    row(0x0072_74e1),
    // `UnlockRecipeMenuQuantity`, `EnableImprovedRecipeMenu`: the count the recipe menu offers
    // 00727920, the 100 stored first (0x727975).
    row(0x0072_7975),
    // `ToggleCraftingMessages`: the recipe menu's craft 007284f0, the item's message (0x728933).
    row(0x0072_8933),
    // `EnableImprovedRecipeMenu`: the recipe menu's craft 007284f0, the menu's close (0x728A0C;
    // JIP's `RecipeMenuCloseHook`: the menu stays open, its list refreshed).
    row(0x0072_8a0c),
    // `InitMiniMap`: `HUDMainMenu::UpdateCompass` 00779070, the found location's name (0x77951E;
    // JIP's `DiscoverLocationHook`).
    row(0x0077_951e),
    // `ShowQuantityMenu`: `QuantityMenu::DoClick` 007abe40's `CALL dword ptr [0x011DA614]` (the
    // result callback). JIP's `QuantityMenuCallback` reads DoClick's button id from the caller's
    // frame (`[ebp+8]`): the seam hands it over with the callback and the count.
    row(0x007a_becc),
    // `ShowTextInputMenu`: JIP's `TextInputClose`, a jump at the start of the text edit
    // menu's click function 007e66a0 while JIP's text input menu is open (its
    // `TextInputCloseHook` replaces the function). Asked only while the hook is on.
    row(0x007e_66a0),
    // `ToggleDepthClear` (G128, JIP's own): the frame's draw after the world 00875110, its call of
    // the renderer's depth clear `0x7148C0` at 0x8751C6 (JIP's `RendererClearBufferHook`: the call
    // skipped while the switch is 0).
    row(0x0087_51c6),
    // `InitRockItLauncher`: `PlayerCharacter::Update` 0093e860, the ammo switch key's weapon entry
    // test (0x940998; JIP's `PCAmmoSwitchHook`: no switch for its Rock-It Launcher).
    row(0x0094_0998),
    // `InitMiniMap`: `PlayerCharacter::Update` 0093e860, its call of 00555c20 (0x9438F6; JIP's
    // `UpdateCellsSeenBitsHook`).
    row(0x0094_38f6),
    // `ToggleVanityWheel`: the player's look handling 009445b0, the zoom test (its `JNP` at
    // 0x945A28; JIP's `SafeWrite8(0x945A29, ..)`): the wheel's zoom skipped while it is off.
    row(0x0094_5a28),
    // `InitRockItLauncher`: `PlayerCharacter::AmmoSwapHelper` 009462c0, the second weapon entry
    // call (0x946310; JIP's `RILAmmoFixHook`: no ammo swap for its Rock-It Launcher).
    row(0x0094_6310),
    // `ToggleCameraCollision`: the player's camera distance 0094a0c0, the `JZ` after the camera
    // pick (JIP's `SafeWrite8(0x94A34E, ..)`): the pick's hit ignored while it is off.
    row(0x0094_a34e),
    // `InitMiniMap`: `PlayerCharacter::CheckForQuestTargetUpdate` 00952c30 and
    // `SetPlayerMapMarker` 00952e60, their call of the path to the target (0x952C69, 0x952F6B;
    // JIP's `UpdatePlacedMarkerHook`).
    row(0x0095_2c69),
    row(0x0095_2f6b),
    // `InitMiniMap`: the renderer's device check 00e74c00, its reset (0xE74C3C; JIP's
    // `RendererRecreateHook`).
    row(0x00e7_4c3c),
];

/// The commands' wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // `InitRockItLauncher`: `TESObjectWEAP::GetCurrentAmmo` 00525980 (reached by address; its own
    // file calls it by address too; JIP's `GetEquippedAmmo`: none for its Rock-It Launcher).
    row(0x0052_5980),
    // `InitRockItLauncher`: the weapon's projectile for an actor 00525a90 (reached only by
    // address; JIP's `GetWeaponProjectileHook`).
    row(0x0052_5a90),
    // `InitItemFilter`: the recipe menu's filter 00728d60 (reached only by address, as the list
    // box's filter; JIP's `RecipeFilterHook` at its end): a recipe the filter keeps is still left
    // out when its name lacks SetItemFilter's text.
    row(0x0072_8d60),
    // `InitItemFilter`: `BarterMenu::FilterModeFunc` 007304b0 (reached only by address; JIP's
    // `ItemFilterHook`), as above for the item's form.
    row(0x0073_04b0),
    // `InitItemFilter`: `ContainerMenu::FilterModeFunc` 0075e650 (reached only by address): as
    // above.
    row(0x0075_e650),
    // `SuppressQuestMessages`: `HUDMainMenu::ShowQuestName` 0077a480 and `ShowQuestObjective`
    // 0077a5b0 (reached only by address; JIP's `QueueQuestMsgHook` in place of the first, a `ret`
    // at the second, for one frame).
    row(0x0077_a480),
    row(0x0077_a5b0),
    // `InitItemFilter`: `InventoryMenu::FilterFunc` 00782620 (reached only by address; JIP's
    // `ItemFilterHook`).
    row(0x0078_2620),
];
