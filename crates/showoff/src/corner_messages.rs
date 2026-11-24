//! ShowOff f45d069 ShowOffEvents.h:1149-1194,2338: OnQueueCornerMessage.
//! All three original queue mutations notify synchronously before insertion.
//! OnShowCornerMessage is a separate thread-safe event and is not implemented
//! by this module. The engine supplies pointers from its actual message layout
//! (the original buffer or JIP G10's expanded one); no fixed offsets are assumed.
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::events;

const EVENT: &[u8] = b"ShowOff:OnQueueCornerMessage";
const APPEND_FIRST: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0077_54fa);
const APPEND_QUEUED: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0077_5624);
const INSERT_URGENT: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0077_5610);

pub(crate) fn install(e: &mut Engine) {
    events::register_native(
        e,
        "ShowOff:OnQueueCornerMessage",
        &[
            events::param::STRING,
            events::param::STRING,
            events::param::STRING,
            events::param::FLOAT,
        ],
        0,
    );
    for slot in [APPEND_FIRST, APPEND_QUEUED, INSERT_URGENT] {
        e.on_site(slot, queued);
    }
}

/// `[unformattedTextPtr, iconPtr, soundPtr, displayTimeBits]`. This source
/// event is notification only: no script result can cancel queue insertion.
fn queued(e: &mut Engine, words: &[u32]) -> Option<u32> {
    events::dispatch(e, EVENT, 0, &words[..4]);
    None
}

#[cfg(test)]
mod tests;
