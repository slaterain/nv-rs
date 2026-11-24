//! lStewieAl's Tweaks' row S0097 (`patchRunAndGunDontLetAPBeNegative`, main.h's
//! `PlayerCharacterDamageActionPointsForAction`): Tweaks writes its own function into
//! `PlayerCharacter`'s vtable slot `0108AD7C` (`+0x340`), the exe's only reference to `0093ab30`.
//! So only the game's virtual calls reach it; a direct call of `0093ab30` (JIP LN NVSE's
//! `ToggleSneak` makes one) keeps the exe's code, as in a game running both.

use engine_core::extend::ROUTINE_BASE;
use engine_core::prelude::*;

/// The vtable slot Tweaks writes: `PlayerCharacter`'s `+0x340`, holding `0093ab30`.
pub const SLOT: u32 = 0x0108_ad7c;
/// Tweaks' function's address here: in Tweaks' own block of the extenders' routine range
/// (`ROUTINE_BASE + 0xA000`, after JIP LN NVSE's routines; the block's first 16 bytes are kept for
/// S0185's message box callback).
pub const ROUTINE: u32 = ROUTINE_BASE + 0xa010;

const _: () = assert!(ROUTINE + 0x10 <= engine_core::mem::EXTENDER_CODE.end);

/// The player's pointer.
const PLAYER: u32 = 0x011d_ea3c;
/// The VATS mode word (+8 of the object at `011f2250`): 0 outside VATS.
const VATS_MODE: u32 = 0x011f_2250 + 8;

/// With the row on: Tweaks' function is put at [`ROUTINE`] and the slot pointed at it (once, at
/// Tweaks' install, as Tweaks' own `SafeWrite32`). Off, or an image without the slot, nothing.
pub(crate) fn install(e: &mut Engine) {
    if !e.stewie(engine_core::extend::stewie::RUN_AND_GUN_AP) || !e.mem.is_mapped(SLOT) {
        return;
    }
    e.register(ROUTINE, action_points);
    e.mem.set_u32(SLOT, ROUTINE);
}

// Translated from lStewieAl's Tweaks 10.05 (main.h): PlayerCharacterDamageActionPointsForAction.
/// `thiscall(player, action)`: outside VATS, out of combat (+0xDF0) and with
/// `[Tweaks] bRunAndGunAPInCombatOnly` nothing is taken; else the action's cost (`0066dca0` with
/// the action, whether in VATS, 0.0 and 0) is held to the player's action points (`011dea3c`
/// +0xA4, `GetClampedActorFloatValue` `0066ef50` with 0xC: the smaller is kept) and, when above 0,
/// spent (slot `0x33c`).
pub(crate) fn action_points(e: &mut Engine, a: &[u32]) -> Ret {
    let (this, action) = (a[0], a[1]);
    let vats = e.mem.u32(VATS_MODE) != 0;
    if !vats && e.mem.u8(this + 0xdf0) == 0 && e.stewie_run_and_gun_combat_only() {
        return Ret::default();
    }
    let cost = e
        .call(0x0066_dca0, &args![action, u32::from(vats), 0.0f32, 0u32])
        .f32();
    let player = e.global::<u32>(PLAYER);
    let points = e.call(0x0066_ef50, &args![player + 0xa4, 0xcu32]).f32();
    // `min(actionCost, pcActionPoints)` (`(a < b) ? a : b`).
    let damage = if cost < points { cost } else { points };
    if damage > 0.0 {
        e.vcall(this, 0x33c, &args![damage]);
    }
    Ret::default()
}
