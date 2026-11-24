//! The HUD family (stage 1): the HUD main menu's compass, which `HUDMainMenu::DoIdle` (`00770430`)
//! updates on every frame of play: its heading strip, the map markers near the player (discovering a
//! location on the way), the quest targets, the player's own marker and the ticks of the actors in
//! range. By the street profile of 2026-11-16 the compass makes about 1,760 calls by address a frame,
//! most of them the tiles' trait setters and getters (now in [`crate::dispatch::tile`]), which its
//! typed code calls directly.
//!
//! - [`compass`]: `UpdateCompass` (`00779070`), `BlinkTile` (`00778c20`), `GetCompassAngle`
//!   (`00778b80`), the player's own marker (`0077a400`) and the heading strip's two floats
//!   (`0077a450`).
//! - [`idle`]: `HUDMainMenu::DoIdle` (`00770430`) itself, its sub-updates by address (JIP's H21, O14
//!   and G10 places kept).
//!
//! What stays by address: the player's and the markers' virtual slots, the map marker data's getters
//! and setters, the location's discovery (its message, the event, the stat, the experience), the
//! quest target list and its records, the tile pools' iterators, the VATS target list and its sort,
//! the trait name table (`Tile::AddUserTrait`), the menu's render test, `fabs`. The extenders' places
//! in the compass stay where they are: JIP's location name (`0077951e`) and location-discovered event
//! (`0077955d`) are asked at the same points, under the same conditions.
//!
//! **Floats.** The compass runs on the main thread only ([`Threads::Main`]), so its maths is plain
//! `f32` (ADR-0014), each operation rounded where the main thread's 24-bit precision rounds it; a
//! double constant the exe adds or multiplies by is used as a double and the result rounded to a
//! float, and the comparisons with integers (FILD) are exact.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod compass;
pub mod idle;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(13);

/// `NV_TYPED=hud`. Native code, so on by its differential tests and planted mutants (the native-code
/// rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static HUD: Family = Family {
    name: "hud",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Main,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0077_9070, compass::update_compass(Ptr, Ptr, Ptr, Ptr)),
        entry!(0x0077_8c20, compass::blink_tile(Ptr, f32, f32)),
        entry!(0x0077_8b80, compass::compass_angle(f32, f32, f32, f32, f32, f32) -> f32),
        entry!(0x0077_a400, compass::player_marker(Ptr) -> Ptr),
        entry!(0x0077_a450, compass::set_strip(Ptr, f32, f32)),
        entry!(0x0077_0430, idle::do_idle(Ptr)),
    ]
}
