//! JohnnyGuitar59049a0 fn_event.cpp191–202; JohnnyEvents.cpp388–402,
//! 494,808–817,958–959,1076–1078 and EventFramework.{h,cpp}.
//! Only callsites715CD5/718059 notify; captured frontmost menu precedes the
//! original focus change, while its ID and script callbacks follow it.
use engine_core::{
    extend::{Owner, SiteSlot},
    prelude::*,
};
use nvse::{
    commands::Read,
    functions::Caller,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
#[derive(Clone, Copy)]
struct Callback {
    script: u32,
    filter: u32,
    deleted: bool,
}
#[derive(Default)]
struct Lists {
    active: Vec<Callback>,
    queued: Vec<Callback>,
    retired: Vec<u32>,
    dispatches: usize,
}
#[derive(Default)]
struct Selection {
    lists: Mutex<Lists>,
    active: AtomicBool,
    dirty: AtomicBool,
}
const OP: u32 = 0x3203;
pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Selection>();
    table::grow_to(e, OP + 1);
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
    let execute = table::execute_address(OP);
    e.register(execute, command);
    table::add_plugin_record(
        e,
        OP,
        &PluginRecord {
            name: "SetJohnnyOnKeyboardControllerSelectionChangeEventHandler",
            short: "SetOnKBCTrlUIDeltaHandler",
            needs_ref: false,
            param_count: 4,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
    for address in [0x0071_5cd5, 0x0071_8059] {
        e.on_site(SiteSlot::new(Owner::JohnnyGuitar, address), site);
    }
    nvse::plugins::listen_per_frame(e);
}
fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    if r.ok {
        let script = r.form(1);
        if script != 0 && e.mem.u32(script) == 0x0103_7094 {
            change(e, script, r.int(3, 0), r.int(0, 0) != 0);
        }
    }
    handled(true)
}
fn change(e: &mut Engine, script: u32, filter: u32, add: bool) {
    if add {
        nvse::lambda::save_variables(e, script);
        let added = {
            let shared = e.ext_get::<Selection>().unwrap();
            let mut s = shared.lists.lock().unwrap();
            if s.active
                .iter()
                .chain(&s.queued)
                .any(|c| c.script == script && c.filter == filter)
            {
                false
            } else {
                s.queued.push(Callback {
                    script,
                    filter,
                    deleted: false,
                });
                shared.dirty.store(true, Ordering::Release);
                true
            }
        };
        if !added {
            nvse::lambda::unsave_variables(e, script);
        }
    } else {
        let shared = e.ext_get::<Selection>().unwrap();
        let mut s = shared.lists.lock().unwrap();
        for c in &mut s.active {
            if c.script == script && c.filter == filter {
                c.deleted = true;
                shared.dirty.store(true, Ordering::Release);
            }
        }
    }
}
pub(crate) fn tick(e: &mut Engine) {
    let released = {
        let shared = e.ext_get::<Selection>().unwrap();
        if !shared.dirty.swap(false, Ordering::AcqRel) {
            return;
        }
        let mut s = shared.lists.lock().unwrap();
        let queued = std::mem::take(&mut s.queued);
        s.active.extend(queued);
        let mut removed = Vec::new();
        s.active.retain(|c| {
            if c.deleted {
                removed.push(c.script);
            }
            !c.deleted
        });
        shared.active.store(!s.active.is_empty(), Ordering::Release);
        if s.dispatches != 0 {
            s.retired.extend(removed);
            Vec::new()
        } else {
            removed
        }
    };
    for script in released {
        nvse::lambda::unsave_variables(e, script);
    }
}
/// Phase0 payload[manager,target,0], returning Some(frontmost), including
/// Some(0), only for a change. Phase1 payload[manager,capturedmenu,1].
fn site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if a[2] == 0 {
        if !e
            .ext_get::<Selection>()
            .unwrap()
            .active
            .load(Ordering::Acquire)
        {
            return None;
        }
        let manager = e.call(0x0071_e290, &args![true]).u32();
        let menu = e.call(0x0072_0e60, &args![manager]).u32();
        (e.mem.u32(a[0] + 0xbc) != a[1]).then_some(menu)
    } else {
        dispatch(e, a[1]);
        None
    }
}
fn dispatch(e: &mut Engine, menu: u32) {
    let callbacks = {
        let shared = e.ext_get::<Selection>().unwrap();
        if !shared.active.load(Ordering::Acquire) {
            return;
        }
        let mut s = shared.lists.lock().unwrap();
        s.dispatches += 1;
        s.active.clone()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id = if menu == 0 {
            u32::MAX
        } else {
            e.vcall(menu, 0x34, &[]).u32()
        };
        for c in callbacks {
            // FilterInt does not insert -1, even when the captured menu ID is -1.
            if c.filter != u32::MAX && (c.filter == 0 || c.filter == id) {
                nvse::functions::call(
                    e,
                    Caller::Internal {
                        script: c.script,
                        this_obj: 0,
                        containing: 0,
                        args: vec![id],
                        alt: true,
                        allow_surplus: false,
                        event_types: None,
                    },
                );
            }
        }
    }));
    let retired = {
        let mut s = e.ext_get::<Selection>().unwrap().lists.lock().unwrap();
        s.dispatches -= 1;
        if s.dispatches == 0 {
            std::mem::take(&mut s.retired)
        } else {
            Vec::new()
        }
    };
    for script in retired {
        nvse::lambda::unsave_variables(e, script);
    }
    if let Err(stop) = result {
        std::panic::resume_unwind(stop);
    }
}
#[cfg(test)]
mod tests;
