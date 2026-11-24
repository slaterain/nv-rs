//! JohnnyGuitar 59049a0: fn_event.cpp168–177, JohnnyEvents.cpp339–354,
//! 492,679–705,933–955,992–1003,1068–1070; EventFramework.{h,cpp}.
//! Source quirks are retained: default integer -1 matches nothing; the
//! post-call level comes from the captured process, even when the actor
//! replaced it; raw integer varargs go through CallFunctionAlt's float-bit
//! ABI. These are not rewritten as an assumed NVSEPlus spawn-fix policy.
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::functions::Caller;
use nvse::{handled, table, CommandArgs};
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Mutex,
};

const OPCODE: u32 = 0x31e3;
const SCRIPT_VTABLE: u32 = 0x0103_7094;
const INVALID_FLAGS: u32 = (1 << 14) | (1 << 21) | (1 << 5);
const ROUTINES: u32 = crate::ROUTINES + 0x100;
// PlayerCharacter, Creature, Character, Actor: MoveToHigh/Low/MiddleLow/MiddleHigh.
const FIRST_SLOTS: [u32; 4] = [0x0108_ac7c, 0x0108_72ec, 0x0108_6cac, 0x0108_4494];
const WRAPPERS: [AbiFn; 16] = [
    wrapper::<0>,
    wrapper::<1>,
    wrapper::<2>,
    wrapper::<3>,
    wrapper::<4>,
    wrapper::<5>,
    wrapper::<6>,
    wrapper::<7>,
    wrapper::<8>,
    wrapper::<9>,
    wrapper::<10>,
    wrapper::<11>,
    wrapper::<12>,
    wrapper::<13>,
    wrapper::<14>,
    wrapper::<15>,
];
const _: () = assert!(ROUTINES + 16 * 4 <= engine_core::mem::EXTENDER_CODE.end);

#[derive(Clone)]
struct Callback {
    script: u32,
    raw_form: u32,
    integer: i32,
    ids: HashSet<u32>,
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
struct ProcessEvents {
    lists: Mutex<Lists>,
    originals: [AtomicU32; 16],
    installed: AtomicBool,
}

pub(crate) fn install(e: &mut Engine) {
    if e.ext_free::<ProcessEvents>()
        .installed
        .swap(true, Ordering::AcqRel)
    {
        return;
    }
    // Only the original virtual class slots change; direct calls to their
    // four shared Actor routines keep their original behavior.
    for (i, handler) in WRAPPERS.into_iter().enumerate() {
        let slot = FIRST_SLOTS[i / 4] + (i % 4) as u32 * 4;
        if !e.mem.is_mapped(slot) {
            continue;
        }
        let original = e.mem.u32(slot);
        e.ext_free::<ProcessEvents>().originals[i].store(original, Ordering::Release);
        let routine = ROUTINES + i as u32 * 4;
        e.register(routine, handler);
        e.mem.set_u32(slot, routine);
    }
    table::grow_to(e, OPCODE + 1);
    let params = table::alloc(e, 60);
    for (i, (name, kind, optional)) in [
        ("setOrRemove", 1, false),
        ("Script", 0x3d, false),
        ("flags", 1, true),
        ("Form", 0x3d, true),
        ("Integer", 1, true),
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
    let execute = table::execute_address(OPCODE);
    e.register(execute, command);
    table::add_plugin_record(
        e,
        OPCODE,
        &table::PluginRecord {
            name: "SetOnProcessLevelChangeEventHandler",
            short: "",
            needs_ref: false,
            param_count: 5,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: table::Returns::Default,
        },
    );
    nvse::plugins::listen_per_frame(e);
}

fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let read = Read::of(e, &a);
    if read.ok {
        let script = read.form(1);
        if script != 0 && e.mem.u32(script) == SCRIPT_VTABLE {
            change(
                e,
                script,
                read.form(3),
                read.int(4, u32::MAX) as i32,
                read.int(0, 0) != 0,
            );
        }
    }
    // Source parses but ignores flags, and never writes the command result.
    handled(true)
}

fn reference_kind(kind: u8) -> bool {
    (0x3a..=0x40).contains(&kind) || kind == 0x69
}
fn valid(e: &Engine, form: u32) -> bool {
    if form == 0 || e.mem.u32(form + 0xc) == 0 {
        return false;
    }
    let flags = e.mem.u32(form + 8);
    flags & INVALID_FLAGS == 0 && (!reference_kind(e.mem.u8(form + 4)) || flags & (1 << 3) != 0)
}
/// FilterFormInt rejects xMarker by ID (not every static). Lists and reference
/// bases are expanded once; later mutations do not rewrite the cached IDs.
fn expand(e: &Engine, form: u32, ids: &mut HashSet<u32>) {
    if !valid(e, form) {
        return;
    }
    let kind = e.mem.u8(form + 4);
    if reference_kind(kind) {
        let base = e.mem.u32(form + 0x20);
        if valid(e, base) && e.mem.u32(base + 0xc) != 0x3b {
            ids.insert(e.mem.u32(base + 0xc));
        }
    } else if kind == 0x55 {
        let mut node = form + 0x18;
        while node != 0 {
            let item = e.mem.u32(node);
            let next = e.mem.u32(node + 4);
            if item == 0 && next == 0 {
                break;
            }
            if item != 0 {
                expand(e, item, ids);
            }
            node = next;
        }
    } else if e.mem.u32(form + 0xc) != 0x3b {
        ids.insert(e.mem.u32(form + 0xc));
    }
}
fn change(e: &mut Engine, script: u32, raw_form: u32, integer: i32, add: bool) {
    if add {
        let mut ids = HashSet::new();
        expand(e, raw_form, &mut ids);
        nvse::lambda::save_variables(e, script);
        let inserted =
            {
                let mut lists = e.ext_free::<ProcessEvents>().lists.lock().unwrap();
                if lists.active.iter().chain(lists.queued.iter()).any(|cb| {
                    cb.script == script && cb.raw_form == raw_form && cb.integer == integer
                }) {
                    false
                } else {
                    lists.queued.push(Callback {
                        script,
                        raw_form,
                        integer,
                        ids,
                        deleted: false,
                    });
                    true
                }
            };
        if !inserted {
            nvse::lambda::unsave_variables(e, script);
        }
    } else {
        let mut lists = e.ext_free::<ProcessEvents>().lists.lock().unwrap();
        for cb in &mut lists.active {
            if cb.script == script && cb.raw_form == raw_form && cb.integer == integer {
                cb.deleted = true;
            }
        }
    }
}

/// Update appends pending callbacks before deleting marked active callbacks.
pub(crate) fn tick(e: &mut Engine) {
    let released = {
        let mut lists = e.ext_free::<ProcessEvents>().lists.lock().unwrap();
        let pending = std::mem::take(&mut lists.queued);
        lists.active.extend(pending);
        let mut gone = Vec::new();
        lists.active.retain(|cb| {
            if cb.deleted {
                gone.push(cb.script);
            }
            !cb.deleted
        });
        retire(&mut lists, gone)
    };
    release(e, released);
}
/// JohnnyEvents::Reset flushes active process handlers only. Its pending add
/// queue survives, as EventInformation::FlushEventCallbacks does not touch it.
pub(crate) fn reset(e: &mut Engine) {
    let released = {
        let mut lists = e.ext_free::<ProcessEvents>().lists.lock().unwrap();
        let gone = std::mem::take(&mut lists.active)
            .into_iter()
            .map(|c| c.script)
            .collect();
        retire(&mut lists, gone)
    };
    release(e, released);
}
fn retire(lists: &mut Lists, scripts: Vec<u32>) -> Vec<u32> {
    if lists.dispatches == 0 {
        scripts
    } else {
        lists.retired.extend(scripts);
        Vec::new()
    }
}
fn release(e: &mut Engine, scripts: Vec<u32>) {
    for script in scripts {
        nvse::lambda::unsave_variables(e, script);
    }
}

fn wrapper<const INDEX: usize>(e: &mut Engine, words: &[u32]) -> Ret {
    let original = e.ext_free::<ProcessEvents>().originals[INDEX].load(Ordering::Acquire);
    let actor = words[0];
    let process = e.mem.u32(actor + 0x68);
    if process == 0 {
        return e.call(original, words);
    }
    let old = e.mem.u32(process + 0x28);
    let result = e.call(original, words);
    // Deliberately the captured process, as DoProcessChange does, NOT a fresh
    // actor+68 lookup. Source may have deleted/replaced this process meanwhile.
    let new = e.mem.u32(process + 0x28);
    if old != new {
        dispatch(e, actor, old, new);
    }
    result
}
fn base_matches(e: &mut Engine, ids: &HashSet<u32>, form: u32) -> bool {
    if !valid(e, form) {
        return false;
    }
    if ids.contains(&e.mem.u32(form + 0xc)) {
        return true;
    }
    // TESObject::GetWaterType is virtual +140 after TESForm's +134 setter.
    if e.mem.u8(form + 4) == 0x23 {
        let water = e.vcall(form, 0x140, &[]).u32();
        return valid(e, water) && ids.contains(&e.mem.u32(water + 0xc));
    }
    false
}
fn matches(e: &mut Engine, ids: &HashSet<u32>, actor: u32) -> bool {
    if !valid(e, actor) {
        return false;
    }
    if ids.is_empty() || ids.contains(&e.mem.u32(actor + 0xc)) {
        return true;
    }
    let base = e.mem.u32(actor + 0x20);
    if !valid(e, base) {
        return false;
    }
    if base_matches(e, ids, base) {
        return true;
    }
    if e.call(0x0056_af40, &args![actor]).bool() {
        let extra = e.call(0x0041_0220, &args![actor + 0x44, 0x2eu32]).u32();
        // The original assumes this extra exists after IsLeveledCreature.
        for offset in [0x10, 0xc] {
            let form = e.mem.u32(extra + offset);
            if base_matches(e, ids, form) {
                return true;
            }
        }
    }
    false
}
fn dispatch(e: &mut Engine, actor: u32, old: u32, new: u32) {
    let callbacks = {
        let mut lists = e.ext_free::<ProcessEvents>().lists.lock().unwrap();
        lists.dispatches += 1;
        lists.active.clone()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for cb in callbacks {
            // -1 creates an empty integer set; IsInFilter is contains(), not
            // an empty-set wildcard. Preserve unsigned new-level bit equality.
            if cb.integer == -1 || cb.integer as u32 != new || !matches(e, &cb.ids, actor) {
                continue;
            }
            nvse::functions::call(
                e,
                Caller::Internal {
                    script: cb.script,
                    this_obj: 0,
                    containing: 0,
                    // CallScript forwards these uint32 words unconverted to
                    // CallFunctionAlt. Its int locals floor the float-bit values.
                    args: vec![actor, old, new],
                    alt: true,
                    allow_surplus: false,
                    event_types: None,
                },
            );
        }
    }));
    let released = {
        let mut lists = e.ext_free::<ProcessEvents>().lists.lock().unwrap();
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
}

#[cfg(test)]
mod tests;
