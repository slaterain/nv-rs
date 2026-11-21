//! JohnnyGuitar 59049a0: `fn_event.cpp`, `JohnnyEvents.cpp` and `EventFramework.cpp`.
//! Render callbacks are queued until its main-loop message, retained once per event bucket,
//! and removed at that message too. They survive load/new-game messages, as JG's `Reset` leaves
//! these three buckets alone. Native JG plugin messages have no consumer in this port and are
//! not emitted here; the script events run at JG's exact three call sites.
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::{commands::Read, functions::Caller, handled, CommandArgs};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Mutex,
};

const PRE: usize = 0;
const GAME: usize = 1;
const MENU: usize = 2;
const SITES: [SiteSlot; 3] = [
    SiteSlot::new(Owner::JohnnyGuitar, 0x0094_3748),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0087_0244),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0087_02a9),
];

#[derive(Clone, Copy)]
struct Callback {
    script: u32,
    deleted: bool,
}

#[derive(Default)]
struct Lists {
    active: [Vec<Callback>; 3],
    queued: [Vec<u32>; 3],
    dispatches: usize,
    // A nested main-loop update must not release a lambda a dispatch snapshot still uses.
    retired: Vec<u32>,
}

#[derive(Default)]
struct Render {
    lists: Mutex<Lists>,
    active_mask: AtomicU32,
    dirty: AtomicBool,
}

pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Render>();
    for (slot, handler) in SITES.into_iter().zip([pre as _, game as _, menu as _]) {
        e.on_site(slot, handler);
    }
    nvse::plugins::listen_per_frame(e);
}

/// `Cmd_SetJohnnyOnRenderUpdateEventHandler_Execute`: third flags argument ignored, fourth
/// selects buckets; the original does not write `result`. Its unsigned set/remove test treats
/// negative values as nonzero too. `IS_TYPE(Script)` compares vtable 01037094
/// (JohnnyGuitar nvse/GameForms.h), rather than trusting the form type byte.
pub(crate) fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    if r.ok {
        let script = r.form(1);
        if script != 0 && e.mem.u32(script) == 0x0103_7094 {
            change(e, script, r.int(0, 0) != 0, r.int(3, 0));
        }
    }
    handled(true)
}

fn buckets(flags: u32) -> [bool; 3] {
    [
        flags & 2 == 0 && flags & 4 != 0,
        flags & 2 == 0 && flags & 4 == 0,
        flags & 1 == 0,
    ]
}

fn change(e: &mut Engine, script: u32, add: bool, flags: u32) {
    for (bucket, selected) in buckets(flags).into_iter().enumerate() {
        if !selected {
            continue;
        }
        if add {
            // Capture before publishing the pending callback, without holding our lock across
            // an xNVSE call. Undo the extra capture if another registration already exists.
            nvse::lambda::save_variables(e, script);
            let inserted = {
                let shared = e.ext_free::<Render>();
                let mut lists = shared.lists.lock().unwrap();
                if lists.active[bucket].iter().any(|c| c.script == script)
                    || lists.queued[bucket].contains(&script)
                {
                    false
                } else {
                    lists.queued[bucket].push(script);
                    shared.dirty.store(true, Ordering::Release);
                    true
                }
            };
            if !inserted {
                nvse::lambda::unsave_variables(e, script);
            }
        } else {
            let shared = e.ext_free::<Render>();
            let mut lists = shared.lists.lock().unwrap();
            // JG removes from the active vector only, including already-deleted callbacks.
            for cb in &mut lists.active[bucket] {
                if cb.script == script {
                    cb.deleted = true;
                    shared.dirty.store(true, Ordering::Release);
                }
            }
        }
    }
}

/// `JohnnyEvents::Update`: append all queued entries, then erase marked callbacks. No flush
/// on load: JG's render callback buckets are absent from `JohnnyEvents::Reset`.
pub(crate) fn tick(e: &mut Engine) {
    let released = {
        let shared = e.ext_free::<Render>();
        if !shared.dirty.swap(false, Ordering::AcqRel) {
            return;
        }
        let mut lists = shared.lists.lock().unwrap();
        let mut removed = Vec::new();
        let mut mask = 0;
        for bucket in 0..3 {
            let pending = std::mem::take(&mut lists.queued[bucket]);
            lists.active[bucket].extend(pending.into_iter().map(|script| Callback {
                script,
                deleted: false,
            }));
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
        shared.active_mask.store(mask, Ordering::Release);
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

fn dispatch(e: &mut Engine, bucket: usize) {
    let scripts = {
        let shared = e.ext_free::<Render>();
        if shared.active_mask.load(Ordering::Acquire) & (1 << bucket) == 0 {
            return;
        }
        let mut lists = shared.lists.lock().unwrap();
        lists.dispatches += 1;
        // JG's CallScript does not check deleted; deletion takes effect at the next Update.
        lists.active[bucket]
            .iter()
            .map(|cb| cb.script)
            .collect::<Vec<_>>()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for script in scripts {
            nvse::functions::call(
                e,
                Caller::Internal {
                    script,
                    this_obj: 0,
                    containing: 0,
                    args: Vec::new(),
                    alt: true,
                    allow_surplus: false,
                    event_types: None,
                },
            );
        }
    }));
    let released = {
        let mut lists = e.ext_free::<Render>().lists.lock().unwrap();
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

fn pre(e: &mut Engine, _: &[u32]) -> Option<u32> {
    dispatch(e, PRE);
    None
}
fn game(e: &mut Engine, _: &[u32]) -> Option<u32> {
    dispatch(e, GAME);
    None
}
fn menu(e: &mut Engine, _: &[u32]) -> Option<u32> {
    dispatch(e, MENU);
    None
}

#[cfg(test)]
mod tests;
