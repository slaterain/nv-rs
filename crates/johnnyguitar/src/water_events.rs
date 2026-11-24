//! JohnnyGuitarNVSE 59049a0, fn_event.cpp:87–105 and JohnnyEvents.cpp:
//! 140–155, 208–219, 613–631, 734–742, 883–886 and 991–1038.
//! These seams dispatch script callbacks. DLL-to-DLL JohnnyGuitar messages
//! use the original plugin interface and are outside this native script slice.

use crate::quest_events::{dispatch, DYING, SEEN_DATA};
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

const PLAYER: u32 = 0x011d_ea3c;
const ACTOR_LIFE_STATE: u32 = 0x108;
const DYING_STATE: u32 = 1;
const REFERENCE_FULL_NAME: u32 = 0x0055_d520;

pub(crate) fn install(e: &mut Engine) {
    e.on_site(SiteSlot::new(Owner::JohnnyGuitar, 0x0089_f49f), |e, a| {
        on_dying(e, a[0]);
        None
    });
    for address in [0x0055_678a, 0x0055_7053] {
        e.on_site(SiteSlot::new(Owner::JohnnyGuitar, address), |e, a| {
            dispatch(e, SEEN_DATA, a[0], 0, &[a[0]]);
            None
        });
    }
}

fn on_dying(e: &mut Engine, actor: u32) {
    if e.mem.u32(actor + ACTOR_LIFE_STATE) != DYING_STATE {
        return;
    }
    if actor != e.mem.u32(PLAYER) {
        let name = e.call(REFERENCE_FULL_NAME, &args![actor]).u32();
        if e.mem.u8(name) == 0 {
            return;
        }
    }
    dispatch(e, DYING, actor, 0, &[actor]);
}
