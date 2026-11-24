//! ShowOff f45d069: ShowOffEvents.h:876–891,993–1030,2334 and
//! internal/decoding.{h:574–615,cpp:23–38}. The original final ProcessImpacts
//! hook runs after JIP's impact hook and dispatches only for a newly walked
//! impact. No native DLL or complete-plugin identity is advertised.
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::events;

const EVENT: &[u8] = b"ShowOff:OnProjectileImpact";
const IMPACT: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x009c_20c9);

pub(crate) fn install(e: &mut Engine) {
    events::register_native(
        e,
        "ShowOff:OnProjectileImpact",
        &[
            events::param::REFERENCE,
            events::param::BASE_FORM,
            events::param::REFERENCE,
        ],
        events::FLAG_FLUSH_ON_LOAD,
    );
    e.on_site(IMPACT, impact);
}

/// `[projectile, detectedNewImpact]` from the original function's final
/// return, after JIP's 009c20bf hook. This observer never replaces its result.
fn impact(e: &mut Engine, words: &[u32]) -> Option<u32> {
    if words[1] == 0 {
        return None;
    }
    let projectile = words[0];
    let base = e.mem.u32(projectile + 0x20);
    // UpdateProjectilePosition: the head DATA (not the first target), except
    // type 2 projectiles. Preserve each float's bits and do not mark a save change.
    if e.mem.u16(base + 0x62) != 2 {
        let data = e.mem.u32(projectile + 0x88);
        if data != 0 {
            for offset in [0, 4, 8] {
                let word = e.mem.u32(data + 4 + offset);
                e.mem.set_u32(projectile + 0x30 + offset, word);
            }
        }
    }
    let mut target = 0;
    if e.mem.u8(projectile + 0x90) != 0 {
        let mut node = projectile + 0x88;
        while node != 0 {
            let data = e.mem.u32(node);
            if data != 0 {
                let reference = e.mem.u32(data);
                if reference != 0 {
                    target = reference;
                    break;
                }
            }
            node = e.mem.u32(node + 4);
        }
    }
    let source = e.mem.u32(projectile + 0xfc);
    let weapon = e.mem.u32(projectile + 0xf8);
    events::dispatch(e, EVENT, projectile, &[source, weapon, target]);
    None
}

#[cfg(test)]
mod tests;
