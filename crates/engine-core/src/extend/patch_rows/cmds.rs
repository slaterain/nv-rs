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

/// The commands' seams.
pub(in super::super) const SITES: &[Entry] = &[
    // `ShowQuantityMenu`: `QuantityMenu::DoClick` 007abe40's `CALL dword ptr [0x011DA614]` (the
    // result callback). JIP's `QuantityMenuCallback` reads DoClick's button id from the caller's
    // frame (`[ebp+8]`): the seam hands it over with the callback and the count.
    row(0x007a_becc),
    // `InitRockItLauncher`: `PlayerCharacter::Update` 0093e860, the ammo switch key's weapon entry
    // test (0x940998; JIP's `PCAmmoSwitchHook`: no switch for its Rock-It Launcher).
    row(0x0094_0998),
    // `ToggleVanityWheel`: the player's look handling 009445b0, the zoom test (its `JNP` at
    // 0x945A28; JIP's `SafeWrite8(0x945A29, ..)`): the wheel's zoom skipped while it is off.
    row(0x0094_5a28),
    // `InitRockItLauncher`: `PlayerCharacter::AmmoSwapHelper` 009462c0, the second weapon entry
    // call (0x946310; JIP's `RILAmmoFixHook`: no ammo swap for its Rock-It Launcher).
    row(0x0094_6310),
    // `ToggleCameraCollision`: the player's camera distance 0094a0c0, the `JZ` after the camera
    // pick (JIP's `SafeWrite8(0x94A34E, ..)`): the pick's hit ignored while it is off.
    row(0x0094_a34e),
];

/// The commands' wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // `InitRockItLauncher`: `TESObjectWEAP::GetCurrentAmmo` 00525980 (reached by address; its own
    // file calls it by address too; JIP's `GetEquippedAmmo`: none for its Rock-It Launcher).
    row(0x0052_5980),
    // `InitRockItLauncher`: the weapon's projectile for an actor 00525a90 (reached only by
    // address; JIP's `GetWeaponProjectileHook`).
    row(0x0052_5a90),
];
