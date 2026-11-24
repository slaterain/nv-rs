//! ShowOff NVSE f45d069: inventory OnAdd callbacks, at the original hook boundaries.
//! `ShowOff:OnAdd` observes Script::SetActionFlag before inventory insertion.
use super::Entry;
use crate::extend::Owner;
const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: &[Owner::ShowOff],
        step: "showoff-on-add",
    }
}
pub(in super::super) const SITES: &[Entry] = &[
    // Fire notification precedes JIP/ammo/perk processing, after IsActor.
    row(0x0052_3191),
    // OnPreActivate, before the original xNVSE OnActivate hook.
    row(0x0057_3184),
    row(0x0057_4b19),
    row(0x0057_4c28),
    row(0x0057_4d00),
    row(0x0057_4f03),
    row(0x0057_5091),
    // Queue notifications, including urgent insertion before append.
    row(0x0077_54fa),
    row(0x0077_5610),
    row(0x0077_5624),
    // ShowPerkMenu scoped collection and the source's delayed perk-limit hooks.
    row(0x0078_4f26),
    row(0x0078_5087),
    row(0x0078_5acc),
    row(0x0078_5b0f),
    row(0x0078_5b2c),
    // Events/ShowOffEvents.h: after JIP impact handling and before the original return.
    // Every normal return of the POV toggle, forwarding its raw stack word.
    row(0x0095_20e1),
    row(0x009c_20c9),
    // Player character controller jump, before its original position query.
    row(0x00cd_4454),
];
