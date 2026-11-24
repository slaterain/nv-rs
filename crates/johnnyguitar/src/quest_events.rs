//! JohnnyGuitarNVSE 59049a0 callbacks using the same single-form filter:
//! `fn_event.cpp:67–75,132–151`, `JohnnyEvents.cpp:123–137,171–195,486–489,
//! 536–559,903–905,992–1058`, and `EventFramework.{h,cpp}`.
//! Registration retains raw filter identity and a cached form-ID expansion; mutations take
//! effect at Update. The engine notification seams and main-loop tick are wired separately.
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::functions::Caller;
use nvse::table::{self, PluginRecord, Returns};
use nvse::{handled, CommandArgs};
use std::collections::HashSet;
use std::sync::Mutex;

const COMPLETE: usize = 0;
const FAILED: usize = 1;
const ADD_PERK: usize = 2;
const START: usize = 3;
const STOP: usize = 4;
const REMOVE_PERK: usize = 5;
pub(crate) const DYING: usize = 6;
pub(crate) const SEEN_DATA: usize = 7;
const BUCKETS: usize = 8;
const SCRIPT_VTABLE: u32 = 0x0103_7094;
/// `TESForm::FormFlags` and `IsValidForm` in EventFramework.cpp.
const INVALID_FLAGS: u32 = (1 << 14) | (1 << 21) | (1 << 5);

#[derive(Clone)]
struct Callback {
    script: u32,
    raw_filter: u32,
    form_ids: HashSet<u32>,
    deleted: bool,
}

#[derive(Default)]
struct Lists {
    active: [Vec<Callback>; BUCKETS],
    queued: [Vec<Callback>; BUCKETS],
    dirty: bool,
    dispatches: usize,
    // As with render callbacks, a nested tick must not free an in-flight lambda snapshot.
    retired: Vec<u32>,
}

#[derive(Default)]
struct QuestEvents {
    lists: Mutex<Lists>,
}

pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<QuestEvents>();
    table::grow_to(e, 0x318f);
    let params = table::alloc(e, 48);
    for (i, (name, kind, optional)) in [
        ("setOrRemove", 1, false),
        ("Script", 0x3d, false),
        ("flags", 1, true),
        ("Form", 0x3d, true),
    ]
    .into_iter()
    .enumerate()
    {
        let slot = params + 12 * i as u32;
        let name = table::guest_str(e, name.as_bytes());
        e.mem.set_u32(slot, name);
        e.mem.set_u32(slot + 4, kind);
        e.mem.set_u32(slot + 8, u32::from(optional));
    }
    for (opcode, name, short, execute) in [
        (
            0x314c,
            "SetJohnnyOnDyingEventHandler",
            "SetOnDyingEventHandler",
            command::<DYING> as AbiFn,
        ),
        (
            0x314d,
            "SetJohnnyOnStartQuestEventHandler",
            "SetOnStartQuestEventHandler",
            command::<START> as AbiFn,
        ),
        (
            0x314e,
            "SetJohnnyOnStopQuestEventHandler",
            "SetOnStopQuestEventHandler",
            command::<STOP> as AbiFn,
        ),
        (
            0x3158,
            "SetJohnnySeenDataEventHandler",
            "SetSeenDataEventHandler",
            command::<SEEN_DATA> as AbiFn,
        ),
        (
            0x317c,
            "SetJohnnyOnCompleteQuestEventHandler",
            "SetOnCompleteQuestEventHandler",
            command::<COMPLETE> as AbiFn,
        ),
        (
            0x317d,
            "SetJohnnyOnFailQuestEventHandler",
            "SetOnFailQuestEventHandler",
            command::<FAILED> as AbiFn,
        ),
        (
            0x318d,
            "SetJohnnyOnAddPerkEventHandler",
            "SetOnAddPerkEventHandler",
            command::<ADD_PERK> as AbiFn,
        ),
        (
            0x318e,
            "SetJohnnyOnRemovePerkEventHandler",
            "SetOnRemovePerkEventHandler",
            command::<REMOVE_PERK> as AbiFn,
        ),
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
                param_count: 4,
                params,
                execute: address,
                eval: 0,
                flags: 0,
                expression: false,
                returns: Returns::Default,
            },
        );
    }
    nvse::plugins::listen_per_frame(e);
}

fn command<const BUCKET: usize>(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let read = Read::of(e, &a);
    if read.ok {
        let script = read.form(1);
        if script != 0 && e.mem.u32(script) == SCRIPT_VTABLE {
            // The uint32 set/remove test treats negative encoded values as set. Flags ignored.
            change(e, BUCKET, script, read.form(3), read.int(0, 0) != 0);
        }
    }
    // The source does not write the command result, including rejected registrations.
    handled(true)
}

/// TESObjectREFR::IsReferenceFormType (TESObjectREFR.cpp:932), the canonical reference kinds.
fn reference_kind(kind: u8) -> bool {
    (0x3a..=0x40).contains(&kind) || kind == 0x69
}
fn valid_form(e: &Engine, form: u32) -> bool {
    if form == 0 || e.mem.u32(form + 0xc) == 0 {
        return false;
    }
    let flags = e.mem.u32(form + 8);
    flags & INVALID_FLAGS == 0 && (!reference_kind(e.mem.u8(form + 4)) || flags & (1 << 3) != 0)
}

/// FilterForm::InsertForm/InsertReference/InsertFormList. Expansion happens once at registration,
/// not at dispatch: later list or form-ID changes do not silently change this cached filter.
fn expand_filter(e: &Engine, form: u32, ids: &mut HashSet<u32>) {
    if !valid_form(e, form) {
        return;
    }
    let kind = e.mem.u8(form + 4);
    if reference_kind(kind) {
        let base = e.mem.u32(form + 0x20);
        if valid_form(e, base) && e.mem.u8(base + 4) != 0x20 {
            ids.insert(e.mem.u32(base + 0xc));
        }
    } else if kind == 0x55 {
        // BGSListForm::GetFormList (00500940) is the inline head at +18. IsEmpty is
        // item==null AND next==null; a null item with a next node does not end iteration.
        let mut node = form + 0x18;
        loop {
            let item = e.mem.u32(node);
            let next = e.mem.u32(node + 4);
            if item == 0 && next == 0 {
                break;
            }
            if item != 0 {
                expand_filter(e, item, ids);
            }
            if next == 0 {
                break;
            }
            node = next;
        }
    } else if kind != 0x20 {
        // FilterForm's only rejected ordinary type is TESObjectSTAT.
        ids.insert(e.mem.u32(form + 0xc));
    }
}

fn change(e: &mut Engine, bucket: usize, script: u32, raw_filter: u32, add: bool) {
    if add {
        let mut form_ids = HashSet::new();
        expand_filter(e, raw_filter, &mut form_ids);
        nvse::lambda::save_variables(e, script);
        let inserted = {
            let shared = e.ext_free::<QuestEvents>();
            let mut lists = shared.lists.lock().unwrap();
            if lists.active[bucket]
                .iter()
                .chain(lists.queued[bucket].iter())
                .any(|cb| cb.script == script && cb.raw_filter == raw_filter)
            {
                false
            } else {
                lists.queued[bucket].push(Callback {
                    script,
                    raw_filter,
                    form_ids,
                    deleted: false,
                });
                lists.dirty = true;
                true
            }
        };
        if !inserted {
            nvse::lambda::unsave_variables(e, script);
        }
    } else {
        let mut lists = e.ext_free::<QuestEvents>().lists.lock().unwrap();
        let mut changed = false;
        for cb in &mut lists.active[bucket] {
            if cb.script == script && cb.raw_filter == raw_filter {
                cb.deleted = true;
                changed = true;
            }
        }
        lists.dirty |= changed;
    }
}

/// JohnnyEvents::Update appends pending entries before erasing deleted active callbacks.
/// JohnnyEvents::Reset never flushes these quest/perk buckets.
pub(crate) fn tick(e: &mut Engine) {
    let released = {
        let mut lists = e.ext_free::<QuestEvents>().lists.lock().unwrap();
        if !lists.dirty {
            return;
        }
        lists.dirty = false;
        let mut removed = Vec::new();
        for bucket in 0..BUCKETS {
            let pending = std::mem::take(&mut lists.queued[bucket]);
            lists.active[bucket].extend(pending);
            lists.active[bucket].retain(|cb| {
                if cb.deleted {
                    removed.push(cb.script);
                }
                !cb.deleted
            });
        }
        if lists.dispatches == 0 {
            removed
        } else {
            lists.retired.extend(removed);
            Vec::new()
        }
    };
    for script in released {
        nvse::lambda::unsave_variables(e, script);
    }
}

/// JohnnyEvents::Reset flushes active dying callbacks. Seen-data handlers and
/// still-pending dying registrations survive FlushEventCallbacks.
pub(crate) fn reset_dying(e: &mut Engine) {
    let released = {
        let mut lists = e.ext_free::<QuestEvents>().lists.lock().unwrap();
        let removed: Vec<_> = std::mem::take(&mut lists.active[DYING])
            .into_iter()
            .map(|cb| cb.script)
            .collect();
        if lists.dispatches == 0 {
            removed
        } else {
            lists.retired.extend(removed);
            Vec::new()
        }
    };
    for script in released {
        nvse::lambda::unsave_variables(e, script);
    }
}

pub(crate) fn dispatch_complete(e: &mut Engine, quest: u32) {
    dispatch(e, COMPLETE, quest, 0, &[quest]);
}
pub(crate) fn dispatch_failed(e: &mut Engine, quest: u32) {
    dispatch(e, FAILED, quest, 0, &[quest]);
}
/// `JohnnyEvents.cpp:123–137,477,536–543` and `EventFramework.h:212–213`:
/// before original AddPerk, CallScript forwards the integer arguments unchanged
/// to CallFunctionAlt. C varargs promote the byte rank to int, including signed
/// `rank-1` (zero -> FFFFFFFF). Alt then interprets those raw words as f32 bits:
/// positive ranks become subnormals (integer UDF parameters floor them to zero),
/// and old rank at zero becomes NaN. Preserve this source ABI mismatch rather
/// than converting to the logical old/new ranks described by GECK-wiki.
pub(crate) fn dispatch_add_perk(e: &mut Engine, actor: u32, perk: u32, rank: u8) {
    let old = (i32::from(rank) - 1) as u32;
    dispatch(e, ADD_PERK, perk, actor, &[perk, old, u32::from(rank)]);
}
/// JohnnyEvents.cpp:103–120,524–529: before the original script-command
/// RemovePerk vcall, reject a null perk or a zero AL rank for the same teammate
/// argument; invoke the callback on the actor with the perk as its sole argument.
pub(crate) fn dispatch_remove_perk(e: &mut Engine, actor: u32, perk: u32, teammate: bool) {
    if perk == 0
        || e.vcall(actor, 0x4a0, &args![perk, u32::from(teammate)])
            .u8()
            == 0
    {
        return;
    }
    dispatch(e, REMOVE_PERK, perk, actor, &[perk]);
}
/// JohnnyEvents.cpp:748–756 after original AddChange at 0060CA1F. The pinned
/// source converts reinterpret_cast<bool*>(parentEBP+8) itself to bool; it does
/// not dereference the flag. That nonnull stack address is always true, so even
/// SetEnabled(0) emits Start, never Stop. Preserve this upstream behavior rather
/// than silently correcting its ABI. The Stop registration and lifecycle remain
/// real, but this source hook never dispatches its bucket.
pub(crate) fn dispatch_source_start_stop(e: &mut Engine, quest: u32) {
    dispatch(e, START, quest, 0, &[quest]);
}
/// FilterForm::DoFilterRef. Leveled template and original base use ID
/// comparison only, unlike the reference's immediate base form.
fn matching_reference_base(e: &mut Engine, actor: u32, ids: &HashSet<u32>) -> bool {
    let base = e.mem.u32(actor + 0x20);
    if !valid_form(e, base) {
        return false;
    }
    if ids.contains(&e.mem.u32(base + 0xc)) {
        return true;
    }
    if e.mem.u8(base + 4) == 0x23 {
        let water = e.vcall(base, 0x140, &[]).u32();
        if valid_form(e, water) && ids.contains(&e.mem.u32(water + 0xc)) {
            return true;
        }
    }
    if !e.call(0x0056_af40, &args![actor]).bool() {
        return false;
    }
    let extra = e.call(0x0041_0220, &args![actor + 0x44, 0x2eu32]).u32();
    // The source dereferences this directly. The engine's leveled check
    // can also succeed from the base flags when the extra is absent.
    extra != 0
        && [0x10, 0xc].into_iter().any(|offset| {
            let form = e.mem.u32(extra + offset);
            valid_form(e, form) && ids.contains(&e.mem.u32(form + 0xc))
        })
}

pub(crate) fn dispatch(
    e: &mut Engine,
    bucket: usize,
    event_form: u32,
    this_obj: u32,
    args: &[u32],
) {
    if !valid_form(e, event_form) {
        return;
    }
    let callbacks = {
        let mut lists = e.ext_free::<QuestEvents>().lists.lock().unwrap();
        let callbacks = lists.active[bucket].clone();
        lists.dispatches += 1;
        callbacks
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for cb in callbacks {
            // IsFormInFilter runs for each callback, including its current validity/ID check.
            // Empty cached sets are wildcards; marked-deleted callbacks still run until Update.
            if !valid_form(e, event_form) {
                continue;
            }
            let id = e.mem.u32(event_form + 0xc);
            if !cb.form_ids.is_empty()
                && !cb.form_ids.contains(&id)
                && (bucket != DYING || !matching_reference_base(e, event_form, &cb.form_ids))
            {
                continue;
            }
            nvse::functions::call(
                e,
                Caller::Internal {
                    script: cb.script,
                    this_obj,
                    containing: 0,
                    args: args.to_vec(),
                    alt: true,
                    allow_surplus: false,
                    event_types: None,
                },
            );
        }
    }));
    let released = {
        let mut lists = e.ext_free::<QuestEvents>().lists.lock().unwrap();
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

#[cfg(test)]
mod tests;
