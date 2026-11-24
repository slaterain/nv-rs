//! JohnnyGuitar59049a0: fn_ui.cpp329–335; fn_event.cpp16–23,205–214;
//! JohnnyEvents.cpp197–206,356–368,604–606,838–843,901,966;
//! EventFramework.cpp registration/queued update and FilterInt behavior.
//! Native DLL messages have no consumer here; script events retain exact call boundaries.
use engine_core::{
    extend::{Owner, SiteSlot},
    prelude::*,
};
use nvse::{
    commands::{console_print, double_words, Read},
    functions::Caller,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Mutex,
};
const SETTINGS: usize = 0;
const SLEEP: usize = 1;
const MENU: u32 = 0x011d_a920;
const IS_REST: u32 = 0x45;
const SCRIPT_VTABLE: u32 = 0x0103_7094;
const SLEEP_CLICK_VTABLE: u32 = 0x0107_63b8;
// +100..13c belongs to process_events. Keep combined virtual installations distinct.
const SLEEP_WRAPPER: u32 = crate::ROUTINES + 0x200;
const _: () = assert!(SLEEP_WRAPPER + 4 <= engine_core::mem::EXTENDER_CODE.end);
const SETTINGS_SITE: SiteSlot = SiteSlot::new(Owner::JohnnyGuitar, 0x007d_6d73);
#[derive(Clone, Copy)]
struct Callback {
    script: u32,
    filter: u32,
    deleted: bool,
}
#[derive(Default)]
struct Lists {
    active: [Vec<Callback>; 2],
    queued: [Vec<Callback>; 2],
    dispatches: usize,
    retired: Vec<u32>,
}
#[derive(Default)]
struct Events {
    lists: Mutex<Lists>,
    active: AtomicU32,
    dirty: AtomicBool,
    original_click: AtomicU32,
}
pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Events>();
    table::grow_to(e, 0x3237);
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
    for (opcode, name, short, count, execute) in [
        (
            0x317f,
            "SetJohnnyOnSettingsUpdateEventHandler",
            "SetOnSettingsUpdateEventHandler",
            3,
            settings_command as AbiFn,
        ),
        (
            0x3211,
            "SetJohnnyOnSleepWaitEventHandler",
            "SetONSleepWEventHandler",
            4,
            sleep_command as AbiFn,
        ),
        (0x3236, "GetSleepWaitMenuState", "", 0, get_state as AbiFn),
    ] {
        let address = table::execute_address(opcode);
        e.register(address, execute);
        table::add_plugin_record(
            e,
            opcode,
            &PluginRecord {
                name,
                short,
                needs_ref: false,
                param_count: count,
                params: if count == 0 { 0 } else { params },
                execute: address,
                eval: 0,
                flags: 0,
                expression: false,
                returns: Returns::Default,
            },
        );
    }
    e.on_site(SETTINGS_SITE, |e, _| {
        dispatch(e, SETTINGS, 0);
        None
    });
    // A virtual-slot wrapper, not a global DoClick function wrapper: direct
    // calls remain exactly the game's path. Installed before real threads.
    if e.mem.is_mapped(SLEEP_CLICK_VTABLE) {
        let original = e.mem.u32(SLEEP_CLICK_VTABLE);
        if original != SLEEP_WRAPPER {
            e.ext_get::<Events>()
                .unwrap()
                .original_click
                .store(original, Ordering::Release);
            e.register(SLEEP_WRAPPER, sleep_click);
            e.mem.set_u32(SLEEP_CLICK_VTABLE, SLEEP_WRAPPER);
        }
    }
    nvse::plugins::listen_per_frame(e);
}
fn settings_command(e: &mut Engine, a: &[u32]) -> Ret {
    command(e, a, SETTINGS)
}
fn sleep_command(e: &mut Engine, a: &[u32]) -> Ret {
    command(e, a, SLEEP)
}
fn command(e: &mut Engine, words: &[u32], bucket: usize) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    if r.ok {
        let script = r.form(1);
        if script != 0 && e.mem.u32(script) == SCRIPT_VTABLE {
            change(
                e,
                bucket,
                script,
                if bucket == SLEEP { r.int(3, 0) } else { 0 },
                r.int(0, 0) != 0,
            );
        }
    }
    // flags are parsed but source RegisterOn* does not forward them. Result unchanged.
    handled(true)
}
fn get_state(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let menu = e.mem.u32(MENU);
    if menu == 0 {
        return handled(true);
    }
    let result = f64::from(u32::from(e.mem.u8(menu + IS_REST)) + 1);
    a.set_result(e, result);
    if e.call(0x005e_23f0, &[]).bool() {
        console_print(e, "GetSleepWaitMenuState >> %.f", &double_words(result));
    }
    handled(true)
}
fn change(e: &mut Engine, bucket: usize, script: u32, filter: u32, add: bool) {
    if add {
        nvse::lambda::save_variables(e, script);
        let added = {
            let shared = e.ext_get::<Events>().unwrap();
            let mut lists = shared.lists.lock().unwrap();
            if lists.active[bucket]
                .iter()
                .chain(&lists.queued[bucket])
                .any(|c| c.script == script && c.filter == filter)
            {
                false
            } else {
                lists.queued[bucket].push(Callback {
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
        let shared = e.ext_get::<Events>().unwrap();
        let mut lists = shared.lists.lock().unwrap();
        for cb in &mut lists.active[bucket] {
            if cb.script == script && cb.filter == filter {
                cb.deleted = true;
                shared.dirty.store(true, Ordering::Release);
            }
        }
    }
}
pub(crate) fn tick(e: &mut Engine) {
    let released = {
        let shared = e.ext_get::<Events>().unwrap();
        if !shared.dirty.swap(false, Ordering::AcqRel) {
            return;
        }
        let mut lists = shared.lists.lock().unwrap();
        let mut removed = Vec::new();
        let mut mask = 0;
        for bucket in 0..2 {
            let pending = std::mem::take(&mut lists.queued[bucket]);
            lists.active[bucket].extend(pending);
            lists.active[bucket].retain(|cb| {
                if cb.deleted {
                    removed.push(cb.script);
                }
                !cb.deleted
            });
            if !lists.active[bucket].is_empty() {
                mask |= 1 << bucket;
            }
        }
        shared.active.store(mask, Ordering::Release);
        if lists.dispatches != 0 {
            lists.retired.extend(removed);
            Vec::new()
        } else {
            removed
        }
    };
    for script in released {
        nvse::lambda::unsave_variables(e, script);
    }
}
fn dispatch(e: &mut Engine, bucket: usize, menu: u32) {
    let callbacks = {
        let shared = e.ext_get::<Events>().unwrap();
        if shared.active.load(Ordering::Acquire) & (1 << bucket) == 0 {
            return;
        }
        let mut lists = shared.lists.lock().unwrap();
        lists.dispatches += 1;
        lists.active[bucket].clone()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for cb in callbacks {
            let args = if bucket == SLEEP {
                let mode = u32::from(e.mem.u8(menu + IS_REST)) + 1;
                if cb.filter != 0 && cb.filter != mode {
                    continue;
                }
                vec![mode]
            } else {
                Vec::new()
            };
            nvse::functions::call(
                e,
                Caller::Internal {
                    script: cb.script,
                    this_obj: 0,
                    containing: 0,
                    // Source CallScript forwards raw uint32 mode words to
                    // CallFunctionAlt; integer locals floor their float bits.
                    args,
                    alt: true,
                    allow_surplus: false,
                    event_types: None,
                },
            );
        }
    }));
    let released = {
        let mut lists = e.ext_get::<Events>().unwrap().lists.lock().unwrap();
        lists.dispatches -= 1;
        if lists.dispatches == 0 {
            std::mem::take(&mut lists.retired)
        } else {
            Vec::new()
        }
    };
    for script in released {
        nvse::lambda::unsave_variables(e, script);
    }
    if let Err(stop) = result {
        std::panic::resume_unwind(stop);
    }
}
fn sleep_click(e: &mut Engine, args: &[u32]) -> Ret {
    let original = e
        .ext_get::<Events>()
        .unwrap()
        .original_click
        .load(Ordering::Acquire);
    let result = e.call(original, args);
    if args[1] == 4 {
        dispatch(e, SLEEP, args[0]);
    }
    result
}
#[cfg(test)]
mod tests;
