//! ShowOff f45d069 ShowOffEvents.h:1192-1235,2117-2180,2340,2370-2371.
//! Native notifications only: firing before ammo/perk effects, the player's
//! jump before position extraction, and every normal exit of the POV switch.
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::events;

const PLAYER: u32 = 0x011d_ea3c;
const FIRE: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0052_3191);
const JUMP: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x00cd_4454);
const POV: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0095_20e1);

pub(crate) fn install(e: &mut Engine) {
    events::register_native(
        e,
        "ShowOff:OnFireWeapon",
        &[events::param::BASE_FORM],
        events::FLAG_FLUSH_ON_LOAD,
    );
    events::register_native(e, "ShowOff:OnPlayerJump", &[], 0);
    events::register_native(e, "ShowOff:OnPOVChange", &[events::param::INT], 0);
    e.on_site(FIRE, fire);
    e.on_site(JUMP, jump);
    e.on_site(POV, pov);
}
/// `[raw firing reference, weapon]`, after the original IsActor call; the
/// saved IsActor result must survive dispatch unchanged in the producer.
fn fire(e: &mut Engine, a: &[u32]) -> Option<u32> {
    events::dispatch(e, b"ShowOff:OnFireWeapon", a[0], &[a[1]]);
    None
}
/// The source assumes a live player/high process, and compares the actual
/// controller with that high process's charCtrl(+138), not the controller's
/// owner or an invented actor classification.
fn jump(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let player = e.mem.u32(PLAYER);
    let process = e.mem.u32(player + 0x68);
    if a[0] == e.mem.u32(process + 0x138) {
        events::dispatch(e, b"ShowOff:OnPlayerJump", player, &[]);
    }
    None
}
/// Full original stack argument, forwarded verbatim despite the source event
/// argument's misleading isFirstPerson name. No inversion or bool reduction.
fn pov(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let player = e.mem.u32(PLAYER);
    events::dispatch(e, b"ShowOff:OnPOVChange", player, &[a[0]]);
    None
}

#[cfg(test)]
mod tests;
