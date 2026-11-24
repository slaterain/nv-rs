//! JohnnyGuitar59049a0 player actor-value callbacks: fn_event.cpp36–46,
//! JohnnyEvents.cpp20–33,284–338,491,708–731,909–930,1064–1066,
//! EventFramework.{h,cpp}. NPC registration is deliberately absent.
//! Source integer filters match by membership (-1 is not a wildcard), and
//! callbacks persist over load/new-game. Registration changes apply at Update.
use engine_core::{
    extend::{Owner, SiteSlot},
    prelude::*,
};
use nvse::{commands::Read, functions::Caller, handled, table, CommandArgs};
use std::{cell::Cell, sync::Mutex};
const OPCODE: u32 = 0x31c4;
const SCRIPT_VTABLE: u32 = 0x0103_7094;
const PLAYER: u32 = 0x011d_ea3c;
const NOTIFY: SiteSlot = SiteSlot::new(Owner::JohnnyGuitar, 0x0066_ee58);
const SET: SiteSlot = SiteSlot::new(Owner::JohnnyGuitar, 0x0088_0842);
const OLD_READS: [SiteSlot; 16] = [
    SiteSlot::new(Owner::JohnnyGuitar, 0x0088_071a),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0088_09c6),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0088_0b53),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0088_0ce6),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0088_0e83),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0088_1026),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0088_11b3),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0088_1397),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0093_a90a),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0093_aa2a),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0093_aece),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0093_b001),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0093_b15d),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0093_b2b0),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0093_b47a),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0093_b87d),
];
thread_local! {static SET_MODE:Cell<bool>=const{Cell::new(false)};}
#[derive(Clone)]
struct Callback {
    script: u32,
    player: u32,
    index: i32,
    flags: u16,
    deleted: bool,
}
#[derive(Default)]
struct Lists {
    active: Vec<Callback>,
    queued: Vec<Callback>,
    dispatches: usize,
    retired: Vec<u32>,
}
#[derive(Default)]
struct Events(Mutex<Lists>);
pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Events>();
    table::grow_to(e, OPCODE + 1);
    let params = table::alloc(e, 48);
    for (i, (name, kind, optional)) in [
        ("setOrRemove", 1, false),
        ("Script", 0x3d, false),
        ("flags", 1, true),
        ("Integer", 1, true),
    ]
    .into_iter()
    .enumerate()
    {
        let at = params + i as u32 * 12;
        let name = table::guest_str(e, name.as_bytes());
        e.mem.set_u32(at, name);
        e.mem.set_u32(at + 4, kind);
        e.mem.set_u32(at + 8, u32::from(optional));
    }
    let execute = table::execute_address(OPCODE);
    e.register(execute, command);
    table::add_plugin_record(
        e,
        OPCODE,
        &table::PluginRecord {
            name: "SetOnActorValueChangeEventHandler",
            short: "SetJohnnyOnActorValueChangeEventHandler",
            needs_ref: false,
            param_count: 4,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: table::Returns::Default,
        },
    );
    e.on_site(NOTIFY, notify);
    e.on_site(SET, set_change);
    for slot in OLD_READS {
        e.on_site(slot, |_, _| Some(0));
    }
    nvse::plugins::listen_per_frame(e);
}
fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let read = Read::of(e, &a);
    if read.ok {
        let script = read.form(1);
        let index = read.int(3, u32::MAX) as i32;
        // Index includes NONE=-1, hence signed comparison with COUNT=77.
        if script != 0 && e.mem.u32(script) == SCRIPT_VTABLE && index < 77 {
            let player = e.mem.u32(PLAYER);
            change(
                e,
                script,
                player,
                index,
                read.int(2, 0) as u16,
                read.int(0, 0) != 0,
            );
        }
    }
    handled(true)
}
fn change(e: &mut Engine, script: u32, player: u32, index: i32, flags: u16, add: bool) {
    if add {
        nvse::lambda::save_variables(e, script);
        let inserted = {
            let mut lists = e.ext_free::<Events>().0.lock().unwrap();
            if lists
                .active
                .iter()
                .chain(lists.queued.iter())
                .any(|c| c.script == script && c.player == player && c.index == index)
            {
                false
            } else {
                lists.queued.push(Callback {
                    script,
                    player,
                    index,
                    flags,
                    deleted: false,
                });
                true
            }
        };
        if !inserted {
            nvse::lambda::unsave_variables(e, script);
        }
    } else {
        let mut lists = e.ext_free::<Events>().0.lock().unwrap();
        for c in &mut lists.active {
            if c.script == script && c.player == player && c.index == index {
                c.deleted = true;
            }
        }
    }
}
pub(crate) fn tick(e: &mut Engine) {
    let released = {
        let mut lists = e.ext_free::<Events>().0.lock().unwrap();
        let queued = std::mem::take(&mut lists.queued);
        lists.active.extend(queued);
        let mut gone = Vec::new();
        lists.active.retain(|c| {
            if c.deleted {
                gone.push(c.script);
            }
            !c.deleted
        });
        if lists.dispatches == 0 {
            gone
        } else {
            lists.retired.extend(gone);
            Vec::new()
        }
    };
    release(e, released);
}
fn release(e: &mut Engine, scripts: Vec<u32>) {
    for script in scripts {
        nvse::lambda::unsave_variables(e, script);
    }
}
fn set_change(e: &mut Engine, words: &[u32]) -> Option<u32> {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            SET_MODE.with(|v| v.set(self.0));
        }
    }
    let previous = SET_MODE.with(|v| v.replace(true));
    let _restore = Restore(previous);
    e.call(0x0066_ee50, words);
    Some(0)
}
fn floor_integer(value: f32) -> i32 {
    engine_core::x87::fistp_chop(f64::from(value.floor()))
}
/// After GetActorValueInfo, including when the vanilla info has no callback.
/// Payload owner/index/previous/change/source/info; source is left for vanilla.
fn notify(e: &mut Engine, words: &[u32]) -> Option<u32> {
    if words[5] == 0 {
        return None;
    }
    let owner = words[0];
    let index = words[1];
    let previous = f32::from_bits(words[2]);
    let change = f32::from_bits(words[3]);
    let new = if SET_MODE.with(Cell::get) {
        change
    } else {
        previous + change
    };
    // The original reads clamped AV and GetAsForm even with no registered users.
    let clamped = e.vcall(owner, 0xc, &args![index]).f32();
    let previous_floor = previous.floor();
    let new_floor = new.floor();
    let clamped_floor = clamped.floor();
    let form = e.vcall(owner, 0x24, &[]).u32();
    if form == 0 || floor_integer(new) == floor_integer(previous) || form != e.mem.u32(PLAYER) {
        return None;
    }
    let callbacks = {
        let mut lists = e.ext_free::<Events>().0.lock().unwrap();
        lists.dispatches += 1;
        lists.active.clone()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for cb in callbacks {
            if cb.index == -1 || cb.index as u32 != index {
                continue;
            }
            let no_floor = cb.flags & 1 != 0;
            let clamp = cb.flags & 2 != 0;
            if clamp && floor_integer(clamped) == floor_integer(previous) {
                continue;
            }
            let old = if no_floor { previous } else { previous_floor };
            let new = if clamp {
                if no_floor {
                    clamped
                } else {
                    clamped_floor
                }
            } else if no_floor {
                new
            } else {
                new_floor
            };
            nvse::functions::call(
                e,
                Caller::Internal {
                    script: cb.script,
                    this_obj: 0,
                    containing: 0,
                    args: vec![index, old.to_bits(), new.to_bits()],
                    alt: true,
                    allow_surplus: false,
                    event_types: None,
                },
            );
        }
    }));
    let released = {
        let mut lists = e.ext_free::<Events>().0.lock().unwrap();
        lists.dispatches -= 1;
        if lists.dispatches == 0 {
            std::mem::take(&mut lists.retired)
        } else {
            Vec::new()
        }
    };
    release(e, released);
    if let Err(stop) = result {
        std::panic::resume_unwind(stop);
    }
    None
}
#[cfg(test)]
mod tests;
