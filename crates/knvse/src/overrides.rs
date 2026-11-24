//! Actor/folder overrides from official kNVSE24 4640f0f:
//! SetOverrideAnimation, PickAnimation, GetActorAnimation and group polling.
//! Immutable snapshots keep callback mutation safe; lambda captures retired
//! during callbacks are released after the last active dispatch, not under
//! the registry lock. Only the target asset families are enabled: ordered and
//! partial-reload variants are explicitly diagnosed if encountered.
use super::{actors, loader, paths};
use engine_core::{
    extend::{Owner, SiteSlot},
    prelude::*,
};
use nvse::{
    commands::{arg_in_buffer, dynamic_cast, rtti, Read},
    expr::Tok,
    functions::Caller,
    handled, CommandArgs,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[derive(Clone)]
struct Set {
    paths: Vec<Vec<u8>>,
    condition: u32,
    poll: bool,
}
type Group = [Vec<Arc<Set>>; 8];
type Map = HashMap<(bool, u32, u16), Arc<Group>>;
#[derive(Clone, Copy, Default)]
struct Track {
    condition: u32,
    group: u16,
    real_group: u16,
    sequence: u32,
    actor: u32,
    first: bool,
}
type TrackKey = (u32, u32, bool);
#[derive(Default)]
struct State {
    forms: Map,
    mods: Map,
    dispatches: usize,
    retired: Vec<u32>,
    polled: BTreeMap<TrackKey, Track>,
    aim_variant: u32,
}
#[derive(Default)]
struct Registry(Arc<Mutex<State>>, AtomicBool);
fn state(e: &Engine) -> Arc<Mutex<State>> {
    Arc::clone(
        &e.ext_get::<Registry>()
            .expect("kNVSE overrides installed at startup")
            .0,
    )
}
const LOOKUPS: &[SiteSlot] = &[
    SiteSlot::new(Owner::Knvse, 0x00493115),
    SiteSlot::new(Owner::Knvse, 0x00493dc0),
    SiteSlot::new(Owner::Knvse, 0x0049431b),
    SiteSlot::new(Owner::Knvse, 0x0049472b),
    SiteSlot::new(Owner::Knvse, 0x004948e6),
    SiteSlot::new(Owner::Knvse, 0x00495630),
    SiteSlot::new(Owner::Knvse, 0x004956ce),
    SiteSlot::new(Owner::Knvse, 0x0049575b),
    SiteSlot::new(Owner::Knvse, 0x00495965),
    SiteSlot::new(Owner::Knvse, 0x004959b9),
    SiteSlot::new(Owner::Knvse, 0x00495a03),
    SiteSlot::new(Owner::Knvse, 0x00495dc6),
    SiteSlot::new(Owner::Knvse, 0x0049651b),
    SiteSlot::new(Owner::Knvse, 0x008b7985),
    SiteSlot::new(Owner::Knvse, 0x0097f36d),
    SiteSlot::new(Owner::Knvse, 0x0097fe09),
    SiteSlot::new(Owner::Knvse, 0x0097ff06),
    SiteSlot::new(Owner::Knvse, 0x009d0e80),
];
pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Registry>();
    for &site in LOOKUPS {
        e.on_site(site, |e, a| {
            if !e.ext_get::<Registry>().unwrap().1.load(Ordering::Acquire) {
                return None;
            }
            lookup(e, a[0], a[2] as u16)
        });
    }
    e.on_site(SiteSlot::new(Owner::Knvse, 0x004949d0), |e, a| {
        start(e, a[0], a[2] as u16, a[1])
    });
    e.on_site(SiteSlot::new(Owner::Knvse, 0x005f444f), |e, a| {
        super::timing24::is_custom_key(&nvse::table::guest_text(e, a[0])).then_some(1)
    });
}
fn retire(state: &mut State, script: u32) {
    if script != 0 {
        state.retired.push(script);
    }
}
fn reap(e: &mut Engine) {
    let state = state(e);
    let scripts = {
        let mut s = state.lock().unwrap();
        if s.dispatches != 0 {
            return;
        }
        std::mem::take(&mut s.retired)
    };
    for script in scripts {
        nvse::lambda::unsave_variables(e, script);
    }
}
fn dispatch<T>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> T) -> T {
    state(e).lock().unwrap().dispatches += 1;
    let result = catch_unwind(AssertUnwindSafe(|| f(e)));
    state(e).lock().unwrap().dispatches -= 1;
    reap(e);
    match result {
        Ok(v) => v,
        Err(stop) => resume_unwind(stop),
    }
}
fn custom(path: &[u8]) -> usize {
    for (index, name) in [
        (7, b"human".as_slice()),
        (1, b"male"),
        (2, b"female"),
        (6, b"hurt"),
        (3, b"mod1"),
        (4, b"mod2"),
        (5, b"mod3"),
    ] {
        let pattern = [b"\\".as_slice(), name, b"\\"].concat();
        if path
            .windows(pattern.len())
            .any(|p| p.eq_ignore_ascii_case(&pattern))
        {
            return index;
        }
    }
    0
}
#[allow(clippy::too_many_arguments)] // Mirrors the source registration identity and batch context.
fn register_file(
    e: &mut Engine,
    first: bool,
    form: u32,
    path: Vec<u8>,
    enable: bool,
    condition: u32,
    poll: bool,
    fill: &mut HashSet<u16>,
) -> Result<(), String> {
    let filename = path
        .rsplit(|&c| c == b'\\' || c == b'/')
        .next()
        .unwrap_or(&path);
    if [b"_order_".as_slice(), b"_partial"].iter().any(|suffix| {
        filename
            .windows(suffix.len())
            .any(|w| w.eq_ignore_ascii_case(suffix))
    }) {
        return Err(format!(
            "kNVSE24: ordered/partial variants unsupported: {}",
            String::from_utf8_lossy(&path)
        ));
    }
    let group = paths::group_id(e, &path)?;
    let kind = custom(&path);
    let id = if form == 0 {
        0xff
    } else {
        e.mem.u32(form + 0xc)
    };
    let shared = state(e);
    let mut saves = Vec::new();
    {
        let mut s = shared.lock().unwrap();
        let mut stack = {
            let map = if form == 0 { &mut s.mods } else { &mut s.forms };
            std::mem::take(&mut Arc::make_mut(map.entry((first, id, group)).or_default())[kind])
        };
        let found = stack
            .iter()
            .position(|set| set.paths.iter().any(|p| p.eq_ignore_ascii_case(&path)));
        if !enable {
            stack.retain(|set| {
                let remove = set.paths.iter().any(|p| p.eq_ignore_ascii_case(&path));
                if remove {
                    retire(&mut s, set.condition);
                }
                !remove
            });
        } else if let Some(index) = found {
            // Source rotates then updates the element at the old iterator
            // position. Keep that24 hot-reload order, including its edge case.
            stack[index..].rotate_left(1);
            if condition != 0 {
                let old = Arc::clone(&stack[index]);
                let mut updated = (*old).clone();
                updated.condition = condition;
                updated.poll = poll;
                saves.push(condition);
                retire(&mut s, old.condition);
                stack[index] = Arc::new(updated);
            }
        } else {
            let new_group = fill.insert(group);
            if new_group || stack.is_empty() {
                stack.push(Arc::new(Set {
                    paths: Vec::new(),
                    condition: 0,
                    poll: false,
                }));
            }
            let old = Arc::clone(stack.last().unwrap());
            let mut updated = (*old).clone();
            updated.paths.push(path);
            if condition != 0 {
                updated.condition = condition;
                updated.poll = poll;
                saves.push(condition);
                retire(&mut s, old.condition);
            }
            *stack.last_mut().unwrap() = Arc::new(updated);
        }
        let map = if form == 0 { &mut s.mods } else { &mut s.forms };
        Arc::make_mut(map.entry((first, id, group)).or_default())[kind] = stack;
    }
    for script in saves {
        nvse::lambda::save_variables(e, script);
    }
    if enable {
        e.ext_get::<Registry>()
            .unwrap()
            .1
            .store(true, Ordering::Release);
    }
    reap(e);
    Ok(())
}
pub(crate) fn register_batch(
    e: &mut Engine,
    first: bool,
    form: u32,
    files: Vec<Vec<u8>>,
    enable: bool,
    condition: u32,
    poll: bool,
) -> Result<(), String> {
    register_files(e, first, form, files, enable, condition, poll, Err)
}

#[allow(clippy::too_many_arguments)] // Same registration context as register_file, with a folder-wide error policy.
fn register_files(
    e: &mut Engine,
    first: bool,
    form: u32,
    files: Vec<Vec<u8>>,
    enable: bool,
    condition: u32,
    poll: bool,
    mut on_error: impl FnMut(String) -> Result<(), String>,
) -> Result<(), String> {
    // Protect a fresh capture while the file batch publishes its per-set
    // references; no registry lock spans argument/file/compiler/game calls.
    if condition != 0 {
        nvse::lambda::save_variables(e, condition);
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut fill = HashSet::new();
        for path in files {
            if let Err(error) =
                register_file(e, first, form, path, enable, condition, poll, &mut fill)
            {
                on_error(error)?;
            }
        }
        Ok(())
    }));
    if condition != 0 {
        nvse::lambda::unsave_variables(e, condition);
    }
    match result {
        Ok(v) => v,
        Err(stop) => resume_unwind(stop),
    }
}
pub(crate) fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let first = read.int(0, 0) != 0;
    let enable = read.int(1, 0) != 0;
    let path = arg_in_buffer(e, &read, 2, 0x1000, "SetActorAnimationPath");
    let poll = read.int(3, 0) != 0;
    let mut condition = read.form(4);
    let actor = if a.this_obj != 0 {
        let actor = dynamic_cast(e, a.this_obj, rtti::TES_FORM, rtti::ACTOR);
        if actor == 0 {
            eprintln!("kNVSE24: SetActorAnimationPath received a non-actor reference");
            return handled(true);
        }
        actor
    } else if first {
        e.mem.u32(actors::PLAYER)
    } else {
        0
    };
    if condition != 0 && e.mem.u8(condition + 4) != 0x11 {
        eprintln!("kNVSE24: condition is not a Script");
        condition = 0;
    }
    let files = paths::command_files(e, &path);
    match register_batch(e, first, actor, files, enable, condition, poll) {
        Ok(()) => a.set_result(e, 1.0),
        Err(error) => eprintln!("{error}"),
    };
    handled(true)
}
fn condition(e: &mut Engine, script: u32, actor: u32) -> Option<f64> {
    nvse::functions::call(
        e,
        Caller::Internal {
            script,
            this_obj: actor,
            containing: 0,
            args: Vec::new(),
            alt: false,
            allow_surplus: false,
            event_types: None,
        },
    )
    .map(|token| match token {
        Tok::Number { v, .. } => v,
        Tok::Bool(b) => f64::from(u32::from(b)),
        _ => 0.0,
    })
}
fn track(e: &mut Engine, animation: u32, actor: u32, group: u16, set: &Set) -> TrackKey {
    let key = (
        e.mem.u32(actor + 0xc),
        actors::slot(e, group),
        actors::first(e, animation),
    );
    let real = actors::real_group(e, actor, group as u8);
    let old = {
        let shared = state(e);
        let mut s = shared.lock().unwrap();
        let t = s.polled.entry(key).or_default();
        let old = t.condition;
        t.condition = set.condition;
        t.group = group;
        t.real_group = real;
        t.actor = key.0;
        t.first = key.2;
        old
    };
    if old != set.condition {
        nvse::lambda::save_variables(e, set.condition);
        let shared = state(e);
        retire(&mut shared.lock().unwrap(), old);
    }
    key
}
fn pick(e: &mut Engine, mods: bool, id: u32, animation: u32, group: u16) -> Option<Arc<Set>> {
    let snapshot = {
        let shared = state(e);
        let s = shared.lock().unwrap();
        let map = if mods { &s.mods } else { &s.forms };
        map.get(&(actors::first(e, animation), id, group)).cloned()
    }?;
    let actor = e.mem.u32(animation + 4);
    let present = [
        !snapshot[3].is_empty(),
        !snapshot[4].is_empty(),
        !snapshot[5].is_empty(),
    ];
    for kind in actors::custom_order(e, animation, actor, group, present)
        .into_iter()
        .rev()
    {
        for set in snapshot[kind].iter().rev() {
            if set.condition != 0 {
                if set.poll {
                    track(e, animation, actor, group, set);
                }
                if condition(e, set.condition, actor).map_or(true, |v| v == 0.0) {
                    continue;
                }
            }
            if !set.paths.is_empty() {
                return Some(Arc::clone(set));
            }
        }
    }
    None
}
fn form_pick(
    e: &mut Engine,
    form: u32,
    animation: u32,
    group: u16,
    mod_result: &mut Option<Arc<Set>>,
) -> Option<Arc<Set>> {
    let id = e.mem.u32(form + 0xc);
    if let Some(result) = pick(e, false, id, animation, group) {
        return Some(result);
    }
    if mod_result.is_none() {
        *mod_result = pick(e, true, id >> 24, animation, group);
    }
    None
}
fn select(e: &mut Engine, animation: u32, group: u16) -> Option<Arc<Set>> {
    if animation == 0 {
        return None;
    }
    let actor = e.mem.u32(animation + 4);
    if actor == 0 {
        return None;
    }
    let mut mod_result = None;
    if let Some(result) = form_pick(e, actor, animation, group, &mut mod_result) {
        return Some(result);
    }
    let weapon = actors::weapon(e, actor);
    if weapon != 0 {
        if let Some(result) = form_pick(e, weapon, animation, group, &mut mod_result) {
            return Some(result);
        }
    }
    let base = e.mem.u32(actor + 0x20);
    if base != 0 {
        if let Some(result) = pick(e, false, e.mem.u32(base + 0xc), animation, group) {
            return Some(result);
        }
    }
    let npc = actors::npc(e, actor);
    if npc != 0 {
        let race = e.mem.u32(npc + 0x110);
        if race != 0 {
            if let Some(result) = form_pick(e, race, animation, group, &mut mod_result) {
                return Some(result);
            }
        }
    }
    let base = actors::actor_base(e, actor);
    if base != 0 && e.mem.u8(base + 4) == 0x2b {
        if let Some(result) = form_pick(e, base, animation, group, &mut mod_result) {
            return Some(result);
        }
    }
    mod_result.or_else(|| pick(e, true, 0xff, animation, group))
}
pub(crate) fn lookup(e: &mut Engine, animation: u32, group: u16) -> Option<u32> {
    dispatch(e, |e| {
        if let Some(set) = select(e, animation, group) {
            return Some(loader::load(e, animation, &set.paths[0]).base).filter(|&base| base != 0);
        }
        let actor = e.mem.u32(animation + 4);
        if actor == e.mem.u32(actors::PLAYER)
            && !actors::first(e, animation)
            && group != actors::real_group(e, actor, group as u8)
        {
            let first = e.mem.u32(actor + 0x690);
            select(e, first, group);
        }
        None
    })
}
fn variant(e: &mut Engine, set: &Set, group: u16) -> Vec<u8> {
    let count = set.paths.len() as u32;
    let low = group & 0xff;
    let triple = if (17..=22).contains(&low) {
        Some(17)
    } else if (26..=91).contains(&low) {
        Some(26)
    } else if (102..=167).contains(&low) {
        Some(102)
    } else {
        None
    };
    let index = if let Some(base) = triple {
        let last = state(e).lock().unwrap().aim_variant;
        if (low - base) % 3 == 0 || last >= count {
            let next = e.call(0x00aa_5230, &args![0x011c_4180u32, count]).u32();
            state(e).lock().unwrap().aim_variant = next;
            next
        } else {
            last
        }
    } else {
        e.call(0x00aa_5230, &args![0x011c_4180u32, count]).u32()
    };
    set.paths[index as usize].clone()
}
pub(crate) fn start(e: &mut Engine, animation: u32, group: u16, original: u32) -> Option<u32> {
    if animation == 0 || e.mem.u32(animation + 4) == 0 {
        return None;
    }
    if !e.ext_get::<Registry>().unwrap().1.load(Ordering::Acquire) {
        if original != 0 {
            if let Err(error) = super::timing24::on_override_play(e, animation, original) {
                eprintln!("{error}");
                return Some(0);
            }
        }
        return None;
    }
    dispatch(e, |e| {
        let Some(set) = select(e, animation, group) else {
            if original != 0 {
                if let Err(error) = super::timing24::on_override_play(e, animation, original) {
                    eprintln!("{error}");
                    return Some(0);
                }
            }
            return None;
        };
        let path = variant(e, &set, group);
        let loaded = loader::load(e, animation, &path);
        if loaded.sequence == 0 {
            return None;
        }
        if let Err(error) = super::timing24::on_override_play(e, animation, loaded.sequence) {
            eprintln!("{error}");
            return None;
        }
        if set.condition != 0 && set.poll {
            let actor = e.mem.u32(animation + 4);
            let key = (
                e.mem.u32(actor + 0xc),
                actors::slot(e, group),
                actors::first(e, animation),
            );
            if let Some(track) = state(e).lock().unwrap().polled.get_mut(&key) {
                track.sequence = loaded.sequence;
            }
        }
        Some(loaded.sequence)
    })
}
pub(crate) fn post_load(e: &mut Engine) {
    let shared = state(e);
    let mut s = shared.lock().unwrap();
    let tracks = std::mem::take(&mut s.polled);
    for t in tracks.values() {
        retire(&mut s, t.condition);
    }
    drop(s);
    reap(e);
}
pub(crate) fn sequence_destroyed(e: &mut Engine, sequence: u32) {
    let shared = state(e);
    let mut s = shared.lock().unwrap();
    let keys = s
        .polled
        .iter()
        .filter(|(_, t)| t.sequence == sequence)
        .map(|(k, _)| *k)
        .collect::<Vec<_>>();
    for key in keys {
        if let Some(t) = s.polled.remove(&key) {
            retire(&mut s, t.condition);
        }
    }
    drop(s);
    reap(e);
}
fn remove_track(e: &mut Engine, key: TrackKey) {
    let shared = state(e);
    let mut s = shared.lock().unwrap();
    if let Some(track) = s.polled.remove(&key) {
        retire(&mut s, track.condition);
    }
}
pub(crate) fn tick(e: &mut Engine) {
    dispatch(e, |e| {
        let snapshot = state(e).lock().unwrap().polled.clone();
        for (key, track) in snapshot {
            let form = e.call(0x0048_39c0, &args![track.actor]).u32();
            let actor = dynamic_cast(e, form, rtti::TES_FORM, rtti::ACTOR);
            if actor == 0 || super::timing24::should_erase(e, actor) {
                remove_track(e, key);
                continue;
            }
            let animation = if track.first {
                e.mem.u32(e.mem.u32(actors::PLAYER) + 0x690)
            } else {
                actors::third(e, actor)
            };
            if (track.sequence != 0
                && e.mem.u32(track.sequence + 0x44) == 0
                && e.mem.u32(track.sequence + 0x24) != 0)
                || track.condition == 0
                || animation == 0
            {
                remove_track(e, key);
                continue;
            }
            let current = e
                .mem
                .u32(animation + 0xe0 + actors::slot(e, track.group) * 4);
            if current == 0 || e.mem.u32(current + 0x74) == 0 {
                remove_track(e, key);
                continue;
            }
            let current_group = e.mem.u16(e.mem.u32(current + 0x74) + 0x10);
            let real = actors::real_group(e, actor, current_group as u8);
            if (track.group & 0xf00) >= (current_group & 0xf00)
                && (track.real_group == real || track.real_group == current_group)
            {
                if let Some(value) = condition(e, track.condition, actor) {
                    let custom = if track.sequence == 0 {
                        0
                    } else {
                        e.mem.u32(track.sequence + 0x44)
                    };
                    if (custom == 0 && value != 0.0 && current != track.sequence)
                        || (custom != 0 && value == 0.0 && current == track.sequence)
                    {
                        let fit = e
                            .call(
                                0x0089_7910,
                                &args![actor, track.group & 0xff, 0u32, 0u32, animation],
                            )
                            .u16();
                        e.call(
                            0x0049_4740,
                            &args![animation, fit, 1u32, u32::MAX, u32::MAX],
                        );
                    }
                }
            } else {
                remove_track(e, key);
            }
        }
    })
}
pub(crate) fn deferred(e: &mut Engine) {
    for entry in paths::read_json(e) {
        for (first, files) in paths::json_folder(e, &entry.folder) {
            register_json_folder(e, &entry, first, files);
        }
    }
}

fn register_json_folder(
    e: &mut Engine,
    entry: &paths::JsonEntry,
    first: bool,
    files: Vec<Vec<u8>>,
) {
    // kNVSE24 LoadPathsForType reports each bad KF and keeps the folder's
    // variant set for the remaining files. Script-command batches stop instead.
    let _ = register_files(
        e,
        first,
        entry.form,
        files,
        true,
        entry.condition,
        entry.poll,
        |error| {
            eprintln!("{error}");
            Ok(())
        },
    );
}

#[cfg(test)]
mod tests;
