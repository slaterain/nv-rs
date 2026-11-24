//! ShowOff f45d069 ShowOffEvents.h:257-386: cancellable OnPreActivate.
//! The engine's 00573184 seam precedes xNVSE's OnActivate, preserving the
//! distinct early false / early true / continue results. Call-site exclusions
//! are determined at the actual original callers, not guessed from a host stack.
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::events;

const EVENT: &[u8] = b"ShowOff:OnPreActivate";
const ACTIVATE: SiteSlot = SiteSlot::new(Owner::ShowOff, 0x0057_3184);
const PLAYER: u32 = 0x011d_ea3c;
const INTERFACE: u32 = 0x011d_8a80;

pub(crate) fn install(e: &mut Engine) {
    e.set_hook(engine_core::extend::Hook::PreActivate, true);
    events::register_native(
        e,
        "ShowOff:OnPreActivate",
        &[events::param::REFERENCE, events::param::INT_PTR],
        0,
    );
    e.on_site(ACTIVATE, activate);
}

/// `[activated, activator]`, only when the original immediate return address
/// was not one of ShowOff's eight exclusions. 1 means continue the original
/// function; 0/2 mean return false/true before NVSE's OnActivate.
fn activate(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (activated, activator) = (words[0], words[1]);
    for at in [0x011c_9350, 0x011c_92d0] {
        let double = e.mem.u32(at);
        if double != 0 && double == activated {
            return Some(1);
        }
    }
    let base = e.mem.u32(activated + 0x20);
    if base == e.mem.u32(0x011c_a27c) || base == e.mem.u32(0x011c_a280) {
        return Some(1);
    }
    let player = e.mem.u32(PLAYER);
    if activator == player {
        let interface = e.mem.u32(INTERFACE);
        if e.mem.u32(interface + 0x4bc) == 2 {
            return Some(2);
        }
        if e.mem.u8(player + 0x64a) != e.mem.u8(player + 0x64c) {
            return Some(0);
        }
        if e.vcall(player, 0x1a0, &args![0u32]).bool()
            && e.call(0x0088_59e0, &args![player, activated]).bool()
        {
            return Some(1);
        }
        if e.mem.u32(activated + 8) & 0x0080_0000 != 0 && !e.vcall(activated, 0x100, &[]).bool() {
            return Some(1);
        }
    }
    let result = e.with_stack(4, |e, should| {
        e.mem.set_u32(should.addr(), 1);
        events::dispatch_with_result(
            e,
            EVENT,
            activated,
            &[activator, should.addr()],
            &mut |e, result| {
                if e.mem.u32(should.addr()) != 0 {
                    if let Some(result) = result {
                        e.mem.set_u32(should.addr(), u32::from(result.boolean()));
                    }
                }
                true
            },
        );
        e.mem.u32(should.addr())
    });
    Some(result)
}

#[cfg(test)]
mod tests;
