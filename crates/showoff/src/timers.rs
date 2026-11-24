//! Partial ShowOff f45d069 timers: SO_fn_AuxTimer.cpp, AuxTimers.h,
//! internal/AuxTimers.cpp, Events/{ShowOffEvents,CustomEventFilters,
//! JohnnyEventPredefinitions}.h. This slice implements temporary seconds timers
//! with flags13 (MCM) and14/526/1038 (Enhanced Movement), including its TimeLeft
//! query. Permanent timers and other modes report unsupported runtime errors;
//! no TAOS persistence or complete timer API is claimed.
//! Malformed event registration is rejected instead of following the source's
//! erroneous OR guard into null/non-Script callback dereferences.
use engine_core::prelude::*;
use nvse::{
    commands::{arg_in_buffer, Read},
    functions::Caller,
    handled,
    plugins::{self, Message},
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
use std::sync::Mutex;
const MENU_FLAGS: u32 = 13;
const GAME_FLAGS: u32 = 14;
const IGNORE_TURBO_FLAGS: u32 = 526;
const REAL_GAME_TIME_FLAGS: u32 = 1038;
const DEFAULT_FLAGS: u32 = 7;
const SCRIPT_VT: u32 = 0x0103_7094;
const LOOKUP: u32 = 0x0048_39c0;
const RETURN_TRUE: u32 = 0x008d_0360;
const PLAYER: u32 = 0x011d_ea3c;

fn text_hash(text: &[u8]) -> u32 {
    let mut h = 0xd39c_a509u32;
    for (i, b) in text.iter().take_while(|&&b| b != 0).enumerate() {
        h = h
            .wrapping_sub(u32::from(b.to_ascii_lowercase()) << [4, 12, 20, 0][i % 4])
            .wrapping_mul(31);
    }
    h
}
fn uint_hash(v: u32) -> u32 {
    v.wrapping_mul(13) ^ (v >> 15)
}
/// Minimal source UnorderedMap ordering: default8 buckets, head insertion,
/// growth when count>bucketcount (before lookup), rehash reversing each chain.
/// Timer expiry order is observable through callbacks, so do not sort by name.
struct Map<V> {
    entries: Vec<(u32, V)>,
    buckets: u32,
    names: bool,
}
impl<V> Map<V> {
    fn new(names: bool) -> Self {
        Self {
            entries: Vec::new(),
            buckets: 8,
            names,
        }
    }
    fn hash(&self, key: u32) -> u32 {
        if self.names {
            key
        } else {
            uint_hash(key)
        }
    }
    fn get(&self, key: u32) -> Option<&V> {
        self.entries.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }
    fn get_mut(&mut self, key: u32) -> Option<&mut V> {
        self.entries
            .iter_mut()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }
    fn insert(&mut self, key: u32, make: impl FnOnce() -> V) -> &mut V {
        if self.entries.len() > self.buckets as usize && self.buckets < 0x8000 {
            self.buckets *= 2;
            let names = self.names;
            let mask = self.buckets - 1;
            let mut entries = std::mem::take(&mut self.entries)
                .into_iter()
                .enumerate()
                .collect::<Vec<_>>();
            entries.sort_by_key(|(i, (k, _))| {
                (
                    (if names { *k } else { uint_hash(*k) }) & mask,
                    std::cmp::Reverse(*i),
                )
            });
            self.entries = entries.into_iter().map(|(_, entry)| entry).collect();
        }
        let at = if let Some(at) = self.entries.iter().position(|(k, _)| *k == key) {
            at
        } else {
            let bucket = self.hash(key) & (self.buckets - 1);
            let at = self
                .entries
                .iter()
                .position(|(k, _)| (self.hash(*k) & (self.buckets - 1)) >= bucket)
                .unwrap_or(self.entries.len());
            self.entries.insert(at, (key, make()));
            at
        };
        &mut self.entries[at].1
    }
    fn remove(&mut self, key: u32) {
        if let Some(i) = self.entries.iter().position(|(k, _)| *k == key) {
            self.entries.remove(i);
        }
    }
}
type Key = (u32, u32, u32); // overriding mod, owner FormID, name hash
#[derive(Clone)]
struct Timer {
    name: Vec<u8>,
    total: f64,
    left: f64,
    pending: bool,
    flags: u32,
}
#[derive(Clone)]
struct Pending {
    key: Key,
    name: Vec<u8>,
    time: f64,
    flags: u32,
}
#[derive(Clone)]
struct Callback {
    script: u32,
    owner: u32,
    name: Vec<u8>,
    deleted: bool,
}
struct State {
    timers: Map<Map<Map<Timer>>>,
    insertions: Vec<Pending>,
    removals: Vec<Key>,
    iterating: usize,
    active: Vec<Callback>,
    queued: Vec<Callback>,
    dispatches: usize,
    retired: Vec<u32>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            timers: Map::new(false),
            insertions: Vec::new(),
            removals: Vec::new(),
            iterating: 0,
            active: Vec::new(),
            queued: Vec::new(),
            dispatches: 0,
            retired: Vec::new(),
        }
    }
}
impl State {
    fn timer(&self, (modid, owner, name): Key) -> Option<&Timer> {
        self.timers.get(modid)?.get(owner)?.get(name)
    }
    fn timer_mut(&mut self, (modid, owner, name): Key) -> Option<&mut Timer> {
        self.timers.get_mut(modid)?.get_mut(owner)?.get_mut(name)
    }
    fn put(&mut self, pending: Pending) {
        let timer = self
            .timers
            .insert(pending.key.0, || Map::new(false))
            .insert(pending.key.1, || Map::new(true))
            .insert(pending.key.2, || Timer {
                name: pending.name,
                total: 0.0,
                left: 0.0,
                pending: false,
                flags: pending.flags,
            });
        timer.total = pending.time;
        timer.left = pending.time;
        timer.pending = false;
        timer.flags = pending.flags;
    }
    fn remove(&mut self, key: Key) {
        if let Some(owners) = self.timers.get_mut(key.0) {
            if let Some(names) = owners.get_mut(key.1) {
                names.remove(key.2);
                if names.entries.is_empty() {
                    owners.remove(key.1);
                }
            }
            if owners.entries.is_empty() {
                self.timers.remove(key.0);
            }
        }
    }
    fn cleanup(&mut self) {
        for key in std::mem::take(&mut self.removals) {
            if self.timer(key).is_some_and(|t| t.pending) {
                self.remove(key);
            }
        }
    }
    fn keys(&self) -> Vec<Key> {
        self.timers
            .entries
            .iter()
            .flat_map(|(m, owners)| {
                owners.entries.iter().flat_map(move |(o, names)| {
                    names.entries.iter().map(move |(n, _)| (*m, *o, *n))
                })
            })
            .collect()
    }
    fn retire(&mut self, scripts: Vec<u32>) -> Vec<u32> {
        if self.dispatches == 0 {
            scripts
        } else {
            self.retired.extend(scripts);
            Vec::new()
        }
    }
}
#[derive(Default)]
struct Timers(Mutex<State>);
fn state(e: &mut Engine) -> std::sync::MutexGuard<'_, State> {
    e.ext_free::<Timers>()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}
fn mod_index(e: &Engine, form: u32) -> u32 {
    if form == 0 {
        return 0xff;
    }
    let mut node = form + 0x10;
    while e.mem.u32(node + 4) != 0 {
        node = e.mem.u32(node + 4);
    }
    let file = e.mem.u32(node);
    if file == 0 {
        0xff
    } else {
        u32::from(e.mem.u8(file + 0x40c))
    }
}
fn scope(e: &Engine, script: u32, owner: u32, name: &[u8]) -> Key {
    (
        if name.get(1) == Some(&b'_') {
            0xff
        } else {
            mod_index(e, script)
        },
        owner,
        text_hash(name),
    )
}
fn subject(e: &Engine, form: u32, this: u32) -> u32 {
    if form != 0 {
        let form = if e.mem.u32(e.mem.u32(form) + 0xf0) == RETURN_TRUE {
            e.mem.u32(form + 0x20)
        } else {
            form
        };
        e.mem.u32(form + 0xc)
    } else if this != 0 {
        e.mem.u32(this + 0xc)
    } else {
        0
    }
}
fn default_this(e: &Engine, a: &CommandArgs) -> u32 {
    if a.this_obj == 0 {
        e.mem.u32(PLAYER)
    } else {
        a.this_obj
    }
}
fn unsupported(e: &mut Engine, script: u32, detail: &str) {
    nvse::errors::show(e,script,&format!("ShowOff AuxTimer native partial support: {detail}; only temporary seconds timers with flags13,14,526,1038 are implemented"));
}
fn supported_name(e: &mut Engine, script: u32, name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }
    if name[0] != b'*' {
        unsupported(
            e,
            script,
            "permanent timer storage/TAOS persistence is unsupported",
        );
        return false;
    }
    true
}
fn start(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let name = arg_in_buffer(e, &r, 0, 256, "AuxTimerStart");
    if !supported_name(e, a.script, &name) {
        return handled(true);
    }
    let owner = subject(e, r.form(3), default_this(e, &a));
    if owner == 0 {
        return handled(true);
    }
    let key = scope(e, a.script, owner, &name);
    let time = r
        .args
        .get(1)
        .and_then(nvse::args::Arg::double)
        .unwrap_or(-1.0);
    let flags = r.int(2, DEFAULT_FLAGS);
    let existing = state(e).timer(key).filter(|t| !t.pending).cloned();
    if existing.is_none() && time == -1.0 {
        return handled(true);
    }
    let retained_restart = time == -1.0 && existing.is_some() && flags == DEFAULT_FLAGS;
    if !matches!(
        flags,
        MENU_FLAGS | GAME_FLAGS | IGNORE_TURBO_FLAGS | REAL_GAME_TIME_FLAGS
    ) && !retained_restart
    {
        unsupported(e, a.script, "requested timer flags/mode is unsupported");
        return handled(true);
    }
    {
        let mut s = state(e);
        let existing = s.timer_mut(key).filter(|t| !t.pending);
        if let Some(timer) = existing {
            if time == -1.0 {
                timer.left = timer.total;
            } else {
                timer.total = time;
                timer.left = time;
            }
            if !retained_restart {
                timer.flags = flags;
            }
        } else {
            let pending = Pending {
                key,
                name,
                time,
                flags,
            };
            if s.iterating != 0 {
                s.insertions.push(pending);
            } else {
                s.put(pending);
            }
        }
    }
    a.set_result(e, 1.0);
    handled(true)
}
fn time_left(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, -1.0);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let name = arg_in_buffer(e, &r, 0, 256, "AuxTimerTimeLeft");
    if !supported_name(e, a.script, &name) {
        return handled(true);
    }
    let owner = subject(e, r.form(1), default_this(e, &a));
    if owner == 0 {
        return handled(true);
    }
    let key = scope(e, a.script, owner, &name);
    let left = state(e).timer(key).filter(|t| !t.pending).map(|t| t.left);
    if let Some(left) = left {
        a.set_result(e, left);
    }
    handled(true)
}
fn stop(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let name = arg_in_buffer(e, &r, 0, 256, "AuxTimerStop");
    if !supported_name(e, a.script, &name) {
        return handled(true);
    }
    let owner = subject(e, r.form(2), default_this(e, &a));
    if owner == 0 {
        return handled(true);
    }
    let key = scope(e, a.script, owner, &name);
    if !state(e).timer(key).is_some_and(|t| !t.pending) {
        return handled(true);
    }
    // Source dispatches before setting PendingRemoval; an explicit stop still
    // wins over a restart attempted inside that stop's callback.
    if r.int(1, 0) != 0 {
        dispatch(e, key, &name);
    }
    {
        let mut s = state(e);
        if let Some(timer) = s.timer_mut(key) {
            timer.pending = true;
        }
        s.removals.push(key);
    }
    a.set_result(e, 1.0);
    handled(true)
}
fn register_stop(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let script = r.form(1);
    if script == 0 || e.mem.u32(script) != SCRIPT_VT {
        return handled(true);
    }
    let name = arg_in_buffer(e, &r, 2, 256, "SetOnAuxTimerStopHandler");
    if !name.is_empty() && !supported_name(e, a.script, &name) {
        return handled(true);
    }
    let form = r.form(3);
    let owner = if form == 0 { default_this(e, &a) } else { form };
    let owner = e.mem.u32(owner + 0xc);
    let same = |cb: &Callback| {
        cb.script == script && cb.owner == owner && cb.name.eq_ignore_ascii_case(&name)
    };
    if r.int(0, 0) != 0 {
        nvse::lambda::save_variables(e, script);
        let inserted = {
            let mut s = state(e);
            if s.active.iter().chain(&s.queued).any(same) {
                false
            } else {
                s.queued.push(Callback {
                    script,
                    owner,
                    name,
                    deleted: false,
                });
                true
            }
        };
        if !inserted {
            nvse::lambda::unsave_variables(e, script);
        }
    } else {
        for cb in &mut state(e).active {
            if same(cb) {
                cb.deleted = true;
            }
        }
    }
    handled(true)
}
fn dispatch(e: &mut Engine, key: Key, name: &[u8]) {
    let callbacks = {
        let mut s = state(e);
        s.dispatches += 1;
        s.active.clone()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for cb in callbacks {
            if cb.owner != key.1
                || !cb.name.eq_ignore_ascii_case(name)
                || (key.0 != 0xff && mod_index(e, cb.script) != key.0)
            {
                continue;
            }
            let owner = e.call(LOOKUP, &args![key.1]).u32();
            nvse::events::with_guest_text(e, name, |e, text| {
                nvse::functions::call(
                    e,
                    Caller::Internal {
                        script: cb.script,
                        this_obj: 0,
                        containing: 0,
                        args: vec![text, owner],
                        alt: true,
                        allow_surplus: false,
                        event_types: None,
                    },
                );
            });
        }
    }));
    let release = {
        let mut s = state(e);
        s.dispatches -= 1;
        if s.dispatches == 0 {
            std::mem::take(&mut s.retired)
        } else {
            Vec::new()
        }
    };
    for script in release {
        nvse::lambda::unsave_variables(e, script);
    }
    if let Err(stop) = result {
        std::panic::resume_unwind(stop);
    }
}
fn update_handlers(e: &mut Engine) {
    let release = {
        let mut s = state(e);
        let queued = std::mem::take(&mut s.queued);
        s.active.extend(queued);
        let mut release = Vec::new();
        s.active.retain(|cb| {
            if cb.deleted {
                release.push(cb.script);
                false
            } else {
                true
            }
        });
        s.retire(release)
    };
    for script in release {
        nvse::lambda::unsave_variables(e, script);
    }
}
pub(crate) fn tick(e: &mut Engine) {
    update_handlers(e);
    // Source ShowOff MessageHandler obtains all three before DoCountdown.
    let global_mult = e.call(0x0071_6440, &[]).f64();
    let vats = e.call(0x009c_8cc0, &args![0x011f_2250u32]).f64();
    let menu = e.call(0x0070_2360, &[]).bool();
    let seconds = f64::from(e.mem.f32(0x011f_63a0));
    let with_mult = seconds * vats;
    let without_mult = seconds / global_mult;
    let keys = {
        let mut s = state(e);
        s.iterating += 1;
        s.keys()
    };
    // AuxTimers.cpp computes this once for each nonempty map; all supported
    // timers live in the temporary map. Source player field E21 and setting
    // value +4 (utility.cpp::GetFltGameSetting). Preserve IEEE division by0.
    let without_turbo = if !keys.is_empty() && e.mem.u8(e.mem.u32(PLAYER) + 0xe21) != 0 {
        with_mult / f64::from(e.mem.f32(0x011d_0fac))
    } else {
        with_mult
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for key in keys {
            if e.call(LOOKUP, &args![key.1]).u32() == 0 {
                state(e).remove(key);
                continue;
            }
            let expire = {
                let mut s = state(e);
                match s.timer_mut(key) {
                    Some(timer)
                        if !timer.pending && timer.flags & if menu { 1 } else { 2 } != 0 =>
                    {
                        let delta = if !menu && timer.flags & (1 << 10) != 0 {
                            without_mult
                        } else if timer.flags & (1 << 9) != 0 {
                            without_turbo
                        } else {
                            with_mult
                        } as f32;
                        timer.left -= f64::from(delta);
                        if timer.left <= 0.0 {
                            Some(timer.name.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            };
            if let Some(name) = expire {
                dispatch(e, key, &name);
                let mut s = state(e);
                if s.timer(key).is_some_and(|timer| {
                    !timer.pending
                        && timer.left.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
                }) {
                    s.remove(key);
                }
            }
        }
    }));
    {
        let mut s = state(e);
        s.iterating -= 1;
        if s.iterating == 0 {
            for pending in std::mem::take(&mut s.insertions) {
                s.put(pending);
            }
            s.cleanup();
        }
    }
    if let Err(stop) = result {
        std::panic::resume_unwind(stop);
    }
}
pub(crate) fn message(e: &mut Engine, message: &Message) {
    if message.kind == plugins::msg::MAIN_GAME_LOOP {
        tick(e);
    } else if matches!(
        message.kind,
        plugins::msg::PRE_LOAD_GAME | plugins::msg::EXIT_TO_MAIN_MENU
    ) {
        let release = {
            let mut s = state(e);
            s.cleanup();
            s.timers.entries.clear();
            let scripts = s.active.drain(..).map(|cb| cb.script).collect();
            s.retire(scripts)
        };
        // Source flushes active callbacks only; queued registrations survive.
        for script in release {
            nvse::lambda::unsave_variables(e, script);
        }
    }
}
pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Timers>();
    plugins::listen_per_frame(e);
    table::grow_to(e, 0x3d46);
    for (op, name, short, spec, handler) in [
        (
            0x3d3e,
            "AuxTimerStart",
            "",
            &[(0, false), (0x2c, true), (1, true), (0x3d, true)][..],
            start as AbiFn,
        ),
        (
            0x3d3f,
            "AuxTimerStop",
            "AuxTimerErase",
            &[(0, false), (1, true), (0x3d, true)][..],
            stop,
        ),
        (
            0x3d43,
            "AuxTimerTimeLeft",
            "",
            &[(0, false), (0x3d, true)][..],
            time_left,
        ),
        (
            0x3d45,
            "SetOnAuxTimerStopHandler",
            "SetOnTimerStopHandler",
            &[(1, false), (0x3d, false), (0, false), (0x3d, true)][..],
            register_stop,
        ),
    ] {
        let params = table::alloc(e, 12 * spec.len() as u32);
        for (i, &(kind, optional)) in spec.iter().enumerate() {
            let at = params + 12 * i as u32;
            let text = table::guest_str(
                e,
                match kind {
                    0 => b"string",
                    0x2c => b"double",
                    0x3d => b"form",
                    _ => b"int",
                },
            );
            e.mem.set_u32(at, text);
            e.mem.set_u32(at + 4, kind);
            e.mem.set_u32(at + 8, u32::from(optional));
        }
        let execute = table::execute_address(op);
        e.register(execute, handler);
        table::add_plugin_record(
            e,
            op,
            &PluginRecord {
                name,
                short,
                needs_ref: false,
                param_count: spec.len() as u16,
                params,
                execute,
                eval: 0,
                flags: 0,
                expression: false,
                returns: Returns::Default,
            },
        );
    }
}
#[cfg(test)]
mod tests;
