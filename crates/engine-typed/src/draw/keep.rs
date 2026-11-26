//! Kept draw records drawn as items (`NV_DRAW_ITEMS`, off by default; `1` on, `check` both forms; the renderer
//! design's stage 1, `engine_core::draw_items`): the world draw's plain geometries (the batch renderer's `render_standard`, `00b98e80`) built once and kept. The first time a
//! geometry is drawn in a pass, its set-up and draw run as always while three logs note what they do: the state
//! cache's setters it calls (`super::super::shader::state`'s leaf setters, kept as calls: the cache decides again
//! what the device gets), the device calls it makes outside them (`engine_core::real_threads`' recorder, with the
//! bytes of each structure passed by address), and its writes of guest memory outside them (the store log, its own
//! stack locals left out), all in one order; and the reads it makes through the draw's accessors
//! ([`super::capture`]: everything the rest of the frame may change, by #574's census) with the bytes read. The
//! next time, when none of those inputs and none of the geometry's and its properties' own bytes changed, the record
//! is replayed: its writes, setter calls and device calls, in order, without the set-up's work.
//! The constant set-ups that follow the camera or the game time (world x view x projection, the eye position, the
//! lights' vectors, the constant maps' defined constants, the vertex entries 15, 16 and 21) are kept as calls too and
//! run again on every replay with the frame's camera, sun, lights and time; what they read is not an input.
//! So a kept draw breaks only when the geometry's own inputs change, not when the camera moves.
//!
//! The draw's own device calls (stream sources, indices, constant uploads, the draw: what the recorder notes) go to
//! the backend once, as a record it keeps (`RecordNext`, `d3d9_stream::Kept`); a replay then sends one `DrawItem`
//! of that record with the replay's registers in their place, when its own calls came together up to the draw and
//! are the record's again (the platform layer holds them: `engine_platform`'s `d3d9items`; anything else goes out
//! as it came). The setter calls stay calls, so the state cache and the device keep agreeing, and the device's state
//! after an item is the one after the draw.
//!
//! What a player sees is the same: a record is used only when every input it read is the same, and it does what the
//! set-up did. With `check`, every draw that could have been replayed is drawn as always and checked: first the item
//! it would have been is made from the record's replay (`DrawItem` with `check` 1), then the set-up runs as always
//! (its own commands, the old form, after a `RecordNext` with `check` 1), and what the set-up did is compared with
//! the record (the counters name the first difference). The backend draws the old form; one recording of the run
//! replayed as its items instead (`d3d9_stream::kept::Form`) gives the pictures and per-draw digests to compare.
//!
//! Not kept: pass types with their own set-ups (0xfc, 0xfd, 0x102, 0xca, 0xd1) and 4, 5, 0x250, 0x251, a shader
//! other than the shadow light shader's set-ups, a skinned geometry, and everything while the drawing thread's
//! packet is open (stage 1 of the records).

use super::capture;
use engine_core::draw_items::{self, Ask, Done};
use engine_core::mem::{STORE_LOG_MARK, STORE_LOG_QUIET};
use engine_core::real_threads::{self as rt, DeviceCall};
use engine_core::Engine;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// `NV_DRAW_ITEMS` ([`engine_core::draw_items::Mode`]), with the engineers' audit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    Check,
    /// As [`Mode::Check`] (`NV_DRAW_ITEMS=check` with `NV_DRAW_ITEMS_AUDIT=1`), and every 64th build also notes every
    /// guest read the set-up makes ([`engine_core::Mem`]'s read audit; stepped threads only, slow): reads that are none
    /// of the record's inputs, its own writes, its device calls' structures, its own bytes or its locals are counted by
    /// address (the counters' `uncovered`).
    Audit,
}

/// The run's mode: [`engine_core::draw_items::mode`] (`NV_DRAW_ITEMS`, or the host's choice from `sDrawItems`);
/// tests set their own ([`set_mode_for_tests`]).
#[inline]
pub fn mode() -> Mode {
    use engine_core::draw_items::Mode as Run;
    if let Some(m) = TEST_MODE.with(Cell::get) {
        return m;
    }
    match engine_core::draw_items::mode() {
        Run::Off => Mode::Off,
        Run::On => Mode::On,
        Run::Check => {
            static AUDIT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if *AUDIT.get_or_init(|| std::env::var("NV_DRAW_ITEMS_AUDIT").is_ok_and(|v| v == "1")) {
                Mode::Audit
            } else {
                Mode::Check
            }
        }
    }
}

thread_local! {
    static TEST_MODE: Cell<Option<Mode>> = const { Cell::new(None) };
}

/// Tests: this thread's mode, in place of `NV_DRAW_ITEMS`.
#[doc(hidden)]
pub fn set_mode_for_tests(mode: Option<Mode>) {
    TEST_MODE.with(|m| m.set(mode));
}

/// A call a record keeps as a call, made again on replay: a state cache setter's (`super::super::shader::state`'s
/// leaf setters: the cache decides again what the device gets), or a constant set-up that depends on the camera
/// (`super::super::shader::accumulator::set_world_view_constants`, `super::super::shader::geometry::eye_pos` and
/// `lights`: they run again with the camera, sun and lights of the frame, so a kept draw survives a moving camera).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leaf {
    RenderState {
        this: u32,
        state: u32,
        value: u32,
        unused: u32,
        lock: u8,
    },
    StageState {
        this: u32,
        stage: u32,
        state: u32,
        value: u32,
        unused: u32,
    },
    SamplerState {
        this: u32,
        stage: u32,
        state: u32,
        value: u32,
        unused: u32,
    },
    Texture {
        this: u32,
        stage: u32,
        texture: u32,
    },
    VertexShader {
        this: u32,
        shader: u32,
        keep: u8,
    },
    PixelShader {
        this: u32,
        shader: u32,
        keep: u8,
    },
    Declaration {
        this: u32,
        declaration: u32,
        keep: u8,
    },
    WorldView {
        this: u32,
        transform: u32,
    },
    EyePos {
        pass_type: u32,
        object: u32,
        matrix: [u32; 16],
    },
    Lights {
        property: u32,
        rendering: u32,
        matrix: [u32; 16],
        scale_object: u32,
        pass_type: u32,
        flag: u32,
    },
    /// The vertex entries that follow the game time or the camera (`super::super::shader::geometry`'s 15, 16 and 21).
    Entry15 {
        this: u32,
        object: u32,
    },
    Entry16 {
        this: u32,
        unused: u32,
    },
    Entry21 {
        this: u32,
        pass_type: u32,
        first: u32,
        second: u32,
    },
    /// A texture stage's bind (`super::super::render::stage::bind`): the stage's texture against the one last bound to
    /// its stage (`0126f680`, a cache other draws change) and the texture's device object, both of the moment.
    StageBind {
        this: u32,
        force: u8,
    },
    /// The tint (`super::super::shader::accumulator::set_tint`): it reads the fog property of the frame.
    Tint {
        this: u32,
        owner: u32,
        green: u32,
        source: u32,
    },
    /// The ambient and emittance set-up (`super::super::shader::geometry::ambient_emittance`): the lights' colours
    /// and the HDR scale of the frame (`011fa270`, `011fa0c0`).
    Ambient {
        this: u32,
        property: u32,
        rendering: u32,
        block: u32,
        word: u32,
    },
    /// A constant map's defined constant (`super::defined::set_defined_constant`): its twelve words.
    Defined([u32; 12]),
}

/// What a set-up did, in its order.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Op {
    Write(u32, Vec<u8>),
    Leaf(Leaf),
    Device(DeviceCall),
}

/// The store log's mark kinds: a device call (`engine_core`'s), a read (`capture`'s), a setter call.
const MARK_DEVICE: u32 = rt::DEVICE_CALL_MARK;
const MARK_READ: u32 = capture::KEEP_READ_MARK;
const MARK_LEAF: u32 = 2;

/// A geometry drawn in a pass: the geometry, the pass type, the shader, the property state, and the draw's order-dependent
/// choices (its flags, the pass's sort flag, whether a restore is owed: `render_standard`).
pub type Key = (u32, u32, u32, u32, u32);

/// A kept draw.
struct Record {
    /// The geometry's and its properties' own bytes as they were (a change of any breaks the record).
    statics: Vec<u8>,
    /// The inputs the set-up read through the accessors, with their bytes at the time (reads of what the set-up
    /// itself had written left out).
    inputs: Vec<(u32, Vec<u8>, u32)>,
    ops: Vec<Op>,
    /// False when the set-up reached the device outside the calls the recorder saw (a call by the vtable from
    /// translated code, a buffer's lock): the record would leave that out, so it is never replayed.
    replayable: bool,
    /// The draw count ([`Stats::seen`]) when it was last built or replayed: records unused for long are dropped.
    last_used: u64,
    /// The backend's record of its draw's own commands (`engine_core::draw_items`): its number and their shape's
    /// hash; none when they could not be sent as a record (a setter's send between them, or no draw).
    item: Option<(u32, u64)>,
}

/// The counters (`NV_DRAW_ITEMS` reports them every [`REPORT_EVERY`] draws it saw).
#[derive(Default)]
struct Stats {
    seen: u64,
    replayed: u64,
    built_new: u64,
    built_statics: u64,
    built_inputs: u64,
    built_unkeepable: u64,
    checked: u64,
    mismatched: u64,
    /// Which input address broke records most (the inputs' addresses, counted).
    /// By (input address, the exe function that read it): how often it broke a record, and how often it was the only
    /// input that changed (the records it alone kept from replaying).
    broke_at: HashMap<(u32, u32), (u64, u64)>,
    first_mismatch: Option<String>,
    /// Builds made, and those audited ([`Mode::Audit`]) with their uncovered reads by word.
    builds: u64,
    audited: u64,
    uncovered: HashMap<u32, u64>,
    /// Check mode: the differing ops by kind and destination.
    differed_at: HashMap<String, u64>,
    /// Items (`engine_core::draw_items`): sent in place of a replay's own commands; replays whose commands went out
    /// as they came (a setter's send between them) or were not the record's (another shape); records sent with a
    /// build, and builds whose commands could not be one; check items made beside their old form, and those dropped.
    items: u64,
    items_plain: u64,
    items_other: u64,
    records_sent: u64,
    records_plain: u64,
    check_items: u64,
    check_items_dropped: u64,
    /// Every record dropped because the backend's were gone (a device made or reset).
    epochs: u64,
}

const REPORT_EVERY: u64 = 1_000_000;

#[derive(Default)]
struct Keep {
    records: HashMap<Key, Record>,
    stats: Stats,
    /// The setter calls of the record being built, and how deep in setters it is.
    leaves: Option<Vec<Leaf>>,
    leaf_depth: u32,
    /// The next backend record's number (never 0), the backend's epoch the records are of, and the numbers of
    /// backend records no record of this side holds any more, not yet sent to be dropped.
    next_id: u32,
    epoch: u64,
    forgotten: Vec<u32>,
}

thread_local! {
    static KEEP: RefCell<Keep> = RefCell::new(Keep::default());
    /// True while this thread builds a record (the setters' one test).
    static BUILDING: Cell<bool> = const { Cell::new(false) };
}

/// Whether this thread is building a record: the leaf setters' one test.
#[inline(always)]
pub fn building() -> bool {
    BUILDING.with(Cell::get)
}

/// A leaf setter's call while a record is being built: noted (in the writes' order), then `body` (the setter's own
/// work) runs with the logs paused, so what it writes and sends is the setter's, made again on replay. A setter
/// called from inside another runs as it is.
pub fn leaf(e: &mut Engine, leaf: Leaf, body: impl FnOnce(&mut Engine)) {
    let outer = KEEP.with(|k| {
        let mut k = k.borrow_mut();
        k.leaf_depth += 1;
        if k.leaf_depth != 1 {
            return false;
        }
        match k.leaves.as_mut() {
            Some(leaves) => {
                e.mem.store_log_mark(MARK_LEAF, n32(leaves.len()));
                leaves.push(leaf);
                true
            }
            None => false,
        }
    });
    if outer {
        // The run of the draw's own commands held so far ends here (`engine_core::draw_items`), as at the replay.
        draw_items::interrupt(e);
        pause(e, true);
    }
    body(e);
    if outer {
        pause(e, false);
    }
    KEEP.with(|k| k.borrow_mut().leaf_depth -= 1);
}

/// Pauses or resumes the three logs.
fn pause(e: &Engine, on: bool) {
    e.mem.store_log_pause(on);
    e.mem.read_audit_pause(on);
    rt::device_record_pause(on);
    capture::keep_reads_pause(on);
}

/// The geometry's and its properties' own bytes a record keeps: the geometry (0xc0 bytes: its bound, world
/// transform, data and skin), the property state's five words, the first property, the shader property (0x100
/// bytes: flags, alpha and fade, textures, material values), the alpha property and the shader.
fn statics(e: &Engine, key: Key) -> Vec<u8> {
    let mut out = Vec::with_capacity(0x380);
    for (at, len) in statics_ranges(e, key) {
        let start = out.len();
        out.resize(start + len as usize, 0);
        if at == 0 || e.mem.try_read(at, &mut out[start..]).is_err() {
            out[start..].fill(0xa5);
        }
    }
    out
}

/// Where [`statics`] reads: (address, length).
fn statics_ranges(e: &Engine, key: Key) -> [(u32, u32); 9] {
    let (geometry, _, shader, state, _) = key;
    let word = |a: u32| {
        let mut b = [0u8; 4];
        e.mem
            .try_read(a, &mut b)
            .map_or(0, |()| u32::from_le_bytes(b))
    };
    [
        (geometry, 0xc0),
        (state, 0x14),
        (word(state), 0x20),
        // The shader property but its per-frame values (+0x24 to +0x38: the second flags, alpha, fade and the distance
        // from the camera), which the set-up reads through the accessors where it uses them.
        (word(state.wrapping_add(0xc)), 0x24),
        (word(state.wrapping_add(0xc)).wrapping_add(0x38), 0xc8),
        (word(state.wrapping_add(0x10)), 0x20),
        (shader, 0x80),
        // The geometry data (its dirty bits +0xe, counts) and its render data (the counts, base and kind the draw sends,
        // its buffers): a change of either is a vertex update or another draw.
        (word(geometry.wrapping_add(0xb8)), 0x40),
        (
            word(word(geometry.wrapping_add(0xb8)).wrapping_add(0x34)),
            0x50,
        ),
    ]
}

/// Draws one geometry of a pass: through its kept record when every input is unchanged, else `body` (the set-up
/// and draw) runs and the record is built again. Off, `body` alone.
pub fn draw(e: &mut Engine, key: Key, mut body: impl FnMut(&mut Engine)) {
    let mode = mode();
    // A test that set its own mode is alone on its thread: other tests' packets (open on other threads) do not
    // concern it.
    let packets = (capture::active() || capture::viewing(e)) && TEST_MODE.with(Cell::get).is_none();
    if mode == Mode::Off || packets || building() {
        body(e);
        return;
    }
    follow_epoch();
    let snapshot = statics(e, key);
    // What breaks a kept record, if anything: None reuses it.
    let verdict = KEEP.with(|k| {
        let mut k = k.borrow_mut();
        let k = &mut *k;
        k.stats.seen += 1;
        match k.records.get(&key) {
            None => Some(Break::New),
            Some(r) if !r.replayable => Some(Break::Unkeepable),
            Some(r) if r.statics != snapshot => Some(Break::Statics),
            Some(r) => r
                .inputs
                .iter()
                .find(|(at, bytes, _)| !same_bytes(e, *at, bytes))
                .map(|(at, _, reader)| {
                    let others = r
                        .inputs
                        .iter()
                        .any(|(a, b, _)| a != at && !same_bytes(e, *a, b));
                    Break::Input(*at, *reader, !others)
                }),
        }
    });
    let item = KEEP.with(|k| k.borrow().records.get(&key).and_then(|r| r.item));
    match verdict {
        None if mode == Mode::On => {
            // The record's replay; its own commands one item when the backend has them.
            match item {
                Some((id, hash)) => {
                    let ask = Ask::Item {
                        id,
                        hash,
                        check: false,
                    };
                    let done = with_held(e, ask, |e| replay(e, key));
                    count(|s| match done {
                        Done::Drawn => s.items += 1,
                        Done::Other => s.items_other += 1,
                        _ => s.items_plain += 1,
                    });
                }
                None => replay(e, key),
            }
            KEEP.with(|k| {
                let mut k = k.borrow_mut();
                let seen = k.stats.seen;
                if let Some(r) = k.records.get_mut(&key) {
                    r.last_used = seen;
                }
            });
            count(|s| s.replayed += 1);
        }
        None => {
            // Check: the item this draw would have been, made from the record's replay as `On` makes it; then the
            // set-up runs as always (its commands the old form beside the item) and what it did is compared with
            // the record.
            let beside = item.is_some_and(|(id, hash)| {
                let ask = Ask::Item {
                    id,
                    hash,
                    check: true,
                };
                let done = with_held(e, ask, |e| replay(e, key));
                count(|s| match done {
                    Done::Drawn => s.check_items += 1,
                    _ => s.check_items_dropped += 1,
                });
                done == Done::Drawn
            });
            let (built, sent) = build_sent(e, key, &mut body, item, beside);
            KEEP.with(|k| {
                let mut k = k.borrow_mut();
                let k = &mut *k;
                k.stats.checked += 1;
                if let Some(old) = k.records.get(&key) {
                    if let Some(diff) = first_difference(&old.ops, &built.ops) {
                        k.stats.mismatched += 1;
                        *k.stats
                            .differed_at
                            .entry(difference_kind(&old.ops, &built.ops))
                            .or_default() += 1;
                        if k.stats.first_mismatch.is_none() {
                            k.stats.first_mismatch = Some(format!("{key:08x?}: {diff}"));
                        }
                    }
                }
                k.records.insert(
                    key,
                    Record {
                        statics: snapshot,
                        last_used: k.stats.seen,
                        item: sent,
                        ..built
                    },
                );
            });
        }
        Some(why) => {
            let (built, sent) = build_sent(e, key, &mut body, item, false);
            KEEP.with(|k| {
                let mut k = k.borrow_mut();
                let k = &mut *k;
                match why {
                    Break::New => k.stats.built_new += 1,
                    Break::Statics => k.stats.built_statics += 1,
                    Break::Unkeepable => k.stats.built_unkeepable += 1,
                    Break::Input(at, reader, alone) => {
                        k.stats.built_inputs += 1;
                        let n = k.stats.broke_at.entry((at, reader)).or_default();
                        n.0 += 1;
                        n.1 += u64::from(alone);
                    }
                }
                k.records.insert(
                    key,
                    Record {
                        statics: snapshot,
                        last_used: k.stats.seen,
                        item: sent,
                        ..built
                    },
                );
            });
        }
    }
    evict();
    send_forgotten(e);
    report();
}

/// `body` with the draw's own commands held for `ask` (`engine_core::draw_items`): what came of it.
fn with_held(e: &mut Engine, ask: Ask, body: impl FnOnce(&mut Engine)) -> Done {
    draw_items::begin(e, ask);
    body(e);
    draw_items::end(e)
}

/// [`build`] with its draw's own commands sent as a backend record (a new number; the record's `old` one dropped
/// with it): the record, and its backend record when its commands could be sent as one. `beside`: a check run's
/// item of the old record was sent just before, so these are its old form. An old backend record not dropped with
/// the new one's sending is dropped later ([`send_forgotten`]).
fn build_sent(
    e: &mut Engine,
    key: Key,
    body: &mut impl FnMut(&mut Engine),
    old: Option<(u32, u64)>,
    beside: bool,
) -> (Record, Option<(u32, u64)>) {
    let id = KEEP.with(|k| {
        let mut k = k.borrow_mut();
        let id = k.next_id.max(1);
        k.next_id = id.wrapping_add(1);
        id
    });
    let forget = old.map_or(0, |(old, _)| old);
    let ask = Ask::Record {
        id,
        forget,
        check: beside,
    };
    let mut built = None;
    let done = with_held(e, ask, |e| built = Some(build(e, key, body)));
    let built = built.expect("the build ran");
    let sent = match done {
        Done::Recorded(hash) => Some((id, hash)),
        _ => None,
    };
    KEEP.with(|k| {
        let mut k = k.borrow_mut();
        if sent.is_some() {
            k.stats.records_sent += 1;
        } else {
            k.stats.records_plain += 1;
            if forget != 0 {
                k.forgotten.push(forget);
            }
        }
    });
    (built, sent)
}

/// The backend's records are gone (a device made or reset: `engine_core::draw_items::epoch`): every record goes, so
/// each is built again and sent anew.
fn follow_epoch() {
    let now = draw_items::epoch();
    KEEP.with(|k| {
        let mut k = k.borrow_mut();
        if k.epoch != now {
            k.epoch = now;
            if !k.records.is_empty() {
                k.stats.epochs += 1;
            }
            k.records.clear();
            k.forgotten.clear();
        }
    });
}

/// The backend records no record holds any more, sent to be dropped (in batches of [`FORGET_BATCH`]).
fn send_forgotten(e: &mut Engine) {
    let ids = KEEP.with(|k| {
        let mut k = k.borrow_mut();
        (k.forgotten.len() >= FORGET_BATCH).then(|| std::mem::take(&mut k.forgotten))
    });
    if let Some(ids) = ids {
        draw_items::forget(e, &ids);
    }
}

const FORGET_BATCH: usize = 256;

/// Above [`MAX_RECORDS`] records, the ones not used in the last [`STALE_AFTER`] draws are dropped (geometry freed, a cell
/// unloaded, a game loaded: their keys are dead or about to be reused), their backend records with them.
fn evict() {
    KEEP.with(|k| {
        let mut k = k.borrow_mut();
        let k = &mut *k;
        if k.records.len() > MAX_RECORDS {
            let cutoff = k.stats.seen.saturating_sub(STALE_AFTER);
            let forgotten = &mut k.forgotten;
            k.records.retain(|_, r| {
                let keep = r.last_used >= cutoff;
                if !keep {
                    forgotten.extend(r.item.map(|(id, _)| id));
                }
                keep
            });
            if k.records.len() > MAX_RECORDS {
                forgotten.extend(k.records.values().filter_map(|r| r.item.map(|(id, _)| id)));
                k.records.clear();
            }
        }
    });
}

const MAX_RECORDS: usize = 40_000;
const STALE_AFTER: u64 = 2_000_000;

/// A length or an index as a guest word (they are far below 4 G here).
fn n32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Why a record was built again.
#[derive(Clone, Copy)]
enum Break {
    New,
    Statics,
    Unkeepable,
    /// The first input that changed, the function that read it, and whether it was the only one.
    Input(u32, u32, bool),
}

fn count(f: impl FnOnce(&mut Stats)) {
    KEEP.with(|k| f(&mut k.borrow_mut().stats));
}

/// Whether guest memory at `at` holds `bytes`.
fn same_bytes(e: &Engine, at: u32, bytes: &[u8]) -> bool {
    let mut now = [0u8; 64];
    if bytes.len() <= now.len() {
        let now = &mut now[..bytes.len()];
        return e.mem.try_read(at, now).is_ok() && now == bytes;
    }
    let mut now = vec![0u8; bytes.len()];
    e.mem.try_read(at, &mut now).is_ok() && now == bytes
}

/// Runs `body` with the three logs on and makes the record of what it did (statics filled in by the caller).
fn build(e: &mut Engine, key: Key, body: &mut impl FnMut(&mut Engine)) -> Record {
    // The stack for locals made first, so the set-up's locals lie in the range the store log leaves out.
    e.with_stack(4, |_, _| ());
    let (low, top) = e.locals_below();
    let audit = mode() == Mode::Audit
        && e.real.is_none()
        && KEEP.with(|k| {
            let mut k = k.borrow_mut();
            k.stats.builds += 1;
            k.stats.builds % AUDIT_EVERY == 0
        });
    if audit {
        e.mem.read_audit_begin();
    }
    e.mem.store_log_begin(low, top);
    rt::device_record_begin();
    capture::keep_reads_begin();
    KEEP.with(|k| k.borrow_mut().leaves = Some(Vec::new()));
    BUILDING.with(|b| b.set(true));
    body(e);
    BUILDING.with(|b| b.set(false));
    let audited = if audit {
        e.mem.read_audit_end()
    } else {
        Vec::new()
    };
    let leaves = KEEP
        .with(|k| k.borrow_mut().leaves.take())
        .unwrap_or_default();
    let reads = capture::keep_reads_end();
    let calls = rt::device_record_end();
    let log = e.mem.store_log_end();
    let (mut record, written) = assemble(log, &calls, &leaves, &reads);
    record.replayable = !rt::device_record_unseen();
    if audit {
        note_uncovered(e, key, &audited, &record, &written, &calls);
    }
    record
}

/// Every [`AUDIT_EVERY`]th build is audited ([`Mode::Audit`]).
const AUDIT_EVERY: u64 = 64;

/// The audited reads of a build that nothing in its record covers, counted by their 4-byte word ([`Mode::Audit`]):
/// covered are the inputs, the build's own writes, its device calls' structures, its own bytes ([`statics`]'s
/// ranges) and its stack locals.
fn note_uncovered(
    e: &Engine,
    key: Key,
    audited: &[(u32, u32)],
    record: &Record,
    written: &std::collections::HashSet<u32>,
    calls: &[DeviceCall],
) {
    let mut ranges: Vec<(u32, u32)> = record
        .inputs
        .iter()
        .map(|(at, bytes, _)| (*at, n32(bytes.len())))
        .collect();
    for call in calls {
        if let Some((i, len)) = rt::device_structure(call.slot, &call.args) {
            ranges.push((call.args[i], len));
        }
    }
    ranges.extend(statics_ranges(e, key));
    let covered = |b: u32| {
        written.contains(&b)
            || e.in_locals(b)
            || ranges
                .iter()
                .any(|&(at, len)| b >= at && u64::from(b) < u64::from(at) + u64::from(len))
    };
    let mut words = std::collections::BTreeSet::new();
    for &(at, len) in audited {
        for b in at..at.saturating_add(len) {
            if !covered(b) {
                words.insert(b & !3);
            }
        }
    }
    KEEP.with(|k| {
        let mut k = k.borrow_mut();
        k.stats.audited += 1;
        for w in words {
            *k.stats.uncovered.entry(w).or_default() += 1;
        }
    });
}

/// The record from the logs: the writes, setter calls and device calls in the store log's order; the reads of bytes
/// the set-up had not written itself before, first read of each address only.
fn assemble(
    log: Vec<(u32, Vec<u8>)>,
    calls: &[DeviceCall],
    leaves: &[Leaf],
    reads: &[(u32, Vec<u8>, u32)],
) -> (Record, std::collections::HashSet<u32>) {
    let mut ops = Vec::new();
    let mut inputs: Vec<(u32, Vec<u8>, u32)> = Vec::new();
    let mut written = std::collections::HashSet::new();
    let mut read_at = std::collections::HashSet::new();
    for (at, bytes) in log {
        let index = |b: &[u8]| u32::from_le_bytes(b.try_into().unwrap_or([0xff; 4])) as usize;
        if at >= STORE_LOG_MARK {
            match at - STORE_LOG_MARK {
                MARK_DEVICE => ops.extend(calls.get(index(&bytes)).cloned().map(Op::Device)),
                MARK_LEAF => ops.extend(leaves.get(index(&bytes)).copied().map(Op::Leaf)),
                MARK_READ => {
                    if let Some((addr, value, reader)) = reads.get(index(&bytes)) {
                        let own =
                            (0..n32(value.len())).all(|i| written.contains(&addr.wrapping_add(i)));
                        if !own && read_at.insert(*addr) {
                            inputs.push((*addr, value.clone(), *reader));
                        }
                    }
                }
                // A write inside a kept call: made again by the call, so no op; but what it wrote is the set-up's own.
                STORE_LOG_QUIET if bytes.len() == 8 => {
                    let addr = n32(index(&bytes[..4]));
                    for i in 0..n32(index(&bytes[4..])) {
                        written.insert(addr.wrapping_add(i));
                    }
                }
                _ => {}
            }
            continue;
        }
        for i in 0..n32(bytes.len()) {
            written.insert(at.wrapping_add(i));
        }
        ops.push(Op::Write(at, bytes));
    }
    (
        Record {
            statics: Vec::new(),
            inputs,
            ops,
            replayable: true,
            last_used: 0,
            item: None,
        },
        written,
    )
}

/// Replays the record of `key`: its writes, setter calls and device calls, in order.
fn replay(e: &mut Engine, key: Key) {
    // Taken out while replaying (nothing it calls looks at the records) and put back: no copy.
    let ops = KEEP.with(|k| {
        k.borrow_mut()
            .records
            .get_mut(&key)
            .map(|r| std::mem::take(&mut r.ops))
    });
    for op in ops.iter().flatten() {
        match op {
            Op::Write(at, bytes) => e.mem.write(*at, bytes),
            Op::Leaf(leaf) => replay_leaf(e, *leaf),
            Op::Device(call) => {
                rt::device_replay(e, call);
            }
        }
    }
    if let Some(ops) = ops {
        KEEP.with(|k| {
            if let Some(r) = k.borrow_mut().records.get_mut(&key) {
                r.ops = ops;
            }
        });
    }
}

fn replay_leaf(e: &mut Engine, leaf: Leaf) {
    // As at the build ([`leaf`]): the run of the draw's own commands held so far ends here.
    draw_items::interrupt(e);
    use crate::shader::{state, FAMILY as SHADER};
    use engine_core::ptr::Ptr;
    use engine_core::typed_call;
    match leaf {
        Leaf::RenderState {
            this,
            state: s,
            value,
            unused,
            lock,
        } => typed_call!(
            SHADER,
            0x00e8_8780,
            state::set_render_state(e, Ptr::new(this), s, value, unused, lock)
        ),
        Leaf::StageState {
            this,
            stage,
            state: s,
            value,
            unused,
        } => typed_call!(
            SHADER,
            0x00e8_8930,
            state::set_stage_state(e, Ptr::new(this), stage, s, value, unused)
        ),
        Leaf::SamplerState {
            this,
            stage,
            state: s,
            value,
            unused,
        } => typed_call!(
            SHADER,
            0x00e9_10a0,
            state::set_sampler_state(e, Ptr::new(this), stage, s, value, unused)
        ),
        Leaf::Texture {
            this,
            stage,
            texture,
        } => typed_call!(
            SHADER,
            0x00e8_8a20,
            state::set_texture(e, Ptr::new(this), stage, texture)
        ),
        Leaf::VertexShader { this, shader, keep } => {
            typed_call!(
                SHADER,
                0x00e9_0e00,
                state::set_vertex_shader(e, Ptr::new(this), shader, keep)
            );
        }
        Leaf::PixelShader { this, shader, keep } => {
            typed_call!(
                SHADER,
                0x00e8_8870,
                state::set_pixel_shader(e, Ptr::new(this), shader, keep)
            );
        }
        Leaf::Declaration {
            this,
            declaration,
            keep,
        } => typed_call!(
            SHADER,
            0x00e9_0fc0,
            state::set_declaration(e, Ptr::new(this), declaration, keep)
        ),
        Leaf::WorldView { this, transform } => {
            typed_call!(
                SHADER,
                0x00b7_c750,
                crate::shader::accumulator::set_world_view_constants(e, this, 0u32, transform)
            );
        }
        Leaf::EyePos {
            pass_type,
            object,
            matrix,
        } => crate::shader::geometry::eye_pos(e, pass_type, Ptr::new(object), matrix),
        Leaf::Lights {
            property,
            rendering,
            matrix,
            scale_object,
            pass_type,
            flag,
        } => crate::shader::geometry::lights(
            e,
            Ptr::new(property),
            Ptr::new(rendering),
            matrix,
            Ptr::new(scale_object),
            pass_type,
            flag,
        ),
        Leaf::Entry15 { this, object } => typed_call!(
            SHADER,
            0x00b7_90d0,
            crate::shader::geometry::vertex_entry_15(e, Ptr::new(this), Ptr::new(object))
        ),
        Leaf::Entry16 { this, unused } => typed_call!(
            SHADER,
            0x00b7_9100,
            crate::shader::geometry::vertex_entry_16(e, Ptr::new(this), unused)
        ),
        Leaf::Entry21 {
            this,
            pass_type,
            first,
            second,
        } => typed_call!(
            SHADER,
            0x00b7_91f0,
            crate::shader::geometry::vertex_entry_21(
                e,
                Ptr::new(this),
                pass_type,
                Ptr::new(first),
                Ptr::new(second)
            )
        ),
        Leaf::Ambient {
            this,
            property,
            rendering,
            block,
            word,
        } => typed_call!(
            SHADER,
            0x00b7_ba70,
            crate::shader::geometry::ambient_emittance(
                e,
                Ptr::new(this),
                Ptr::new(property),
                Ptr::new(rendering),
                Ptr::new(block),
                word
            )
        ),
        Leaf::StageBind { this, force } => {
            let _: bool = typed_call!(
                crate::render::FAMILY,
                0x00e7_eb00,
                crate::render::stage::bind(e, Ptr::new(this), force)
            );
        }
        Leaf::Tint {
            this,
            owner,
            green,
            source,
        } => typed_call!(
            SHADER,
            0x00c0_2d90,
            crate::shader::accumulator::set_tint(e, this, Ptr::new(owner), green, source)
        ),
        Leaf::Defined(w) => {
            let _: u32 = typed_call!(
                super::FAMILY,
                0x00e8_3db0,
                super::defined::set_defined_constant(
                    e,
                    Ptr::new(w[0]),
                    w[1],
                    w[2],
                    w[3],
                    w[4],
                    w[5],
                    w[6],
                    w[7],
                    w[8],
                    w[9],
                    w[10],
                    w[11],
                )
            );
        }
    }
}

/// The first place two records' work differs, in words, for the check mode's report.
fn first_difference(old: &[Op], new: &[Op]) -> Option<String> {
    for (i, (a, b)) in old.iter().zip(new).enumerate() {
        if a != b {
            return Some(format!("op {i}: kept {a:x?}, the set-up {b:x?}"));
        }
    }
    (old.len() != new.len())
        .then(|| format!("{} ops kept, {} made by the set-up", old.len(), new.len()))
}

/// Where two records' work first differs, by kind and destination (the check mode's histogram): a write by its
/// address, a device call by its slot, a kept call by its kind, or the lengths.
fn difference_kind(old: &[Op], new: &[Op]) -> String {
    let kind = |op: &Op| match op {
        Op::Write(at, _) => format!("write {at:08x}"),
        Op::Device(call) => format!("device slot {:#x}", call.slot),
        Op::Leaf(leaf) => format!(
            "call {}",
            format!("{leaf:?}")
                .split([' ', '{', '('])
                .next()
                .unwrap_or("")
        ),
    };
    old.iter().zip(new).find(|(a, b)| a != b).map_or_else(
        || "length".to_string(),
        |(a, b)| {
            let (a, b) = (kind(a), kind(b));
            if a == b {
                a
            } else {
                format!("{a} / {b}")
            }
        },
    )
}

/// The counters, every [`REPORT_EVERY`] draws seen.
fn report() {
    KEEP.with(|k| {
        let k = k.borrow();
        let s = &k.stats;
        if s.seen % REPORT_EVERY != 0 {
            return;
        }
        eprintln!("{}", summary(s, k.records.len()));
    });
}

fn summary(s: &Stats, records: usize) -> String {
    let mut broke: Vec<_> = s.broke_at.iter().collect();
    broke.sort_by_key(|(at, n)| (std::cmp::Reverse(n.0), **at));
    let top = broke
        .iter()
        .take(4)
        .map(|((at, reader), (n, alone))| {
            format!("{at:08x} read by {reader:08x}: {n} ({alone} alone)")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut uncovered: Vec<_> = s.uncovered.iter().collect();
    uncovered.sort_by_key(|(at, n)| (std::cmp::Reverse(**n), **at));
    let audit = if s.audited == 0 {
        String::new()
    } else {
        let words = uncovered
            .iter()
            .take(16)
            .map(|(at, n)| format!("{at:08x}: {n}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "; {} builds audited, reads none of their records covers (by word, builds): [{words}]",
            s.audited
        )
    };
    let mut differed: Vec<_> = s.differed_at.iter().collect();
    differed.sort_by_key(|(at, n)| (std::cmp::Reverse(**n), (*at).clone()));
    let differed = differed
        .iter()
        .take(6)
        .map(|(at, n)| format!("{at}: {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "NV_DRAW_ITEMS: {} draws: {} replayed, {} built (new {}, own bytes changed {}, inputs changed {}, device reached outside the record {}); {} checked, {} differed{}; {records} records; inputs that broke records most: [{top}]; differed first at: [{differed}]{audit}; items: {} drawn, {} plain, {} not the record's; backend records: {} sent, {} plain; check items: {} beside their old form, {} dropped; records dropped for a new device: {}",
        s.seen,
        s.replayed,
        s.built_new + s.built_statics + s.built_inputs + s.built_unkeepable,
        s.built_new,
        s.built_statics,
        s.built_inputs,
        s.built_unkeepable,
        s.checked,
        s.mismatched,
        s.first_mismatch.as_deref().map_or_else(String::new, |m| format!(" (first: {m})")),
        s.items,
        s.items_plain,
        s.items_other,
        s.records_sent,
        s.records_plain,
        s.check_items,
        s.check_items_dropped,
        s.epochs,
    )
}

/// The counters now, as the periodic report prints them.
pub fn summary_now() -> String {
    KEEP.with(|k| {
        let k = k.borrow();
        summary(&k.stats, k.records.len())
    })
}

/// Tests: drops this thread's records and counters.
#[doc(hidden)]
pub fn reset_for_tests() {
    KEEP.with(|k| *k.borrow_mut() = Keep::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record from logs: writes and marks in order become the ops; a read of bytes written before it is not an
    /// input, a second read of an address neither; a read of untouched bytes is, with its bytes.
    #[test]
    fn a_record_keeps_the_logs_order_and_only_outside_reads() {
        let call = DeviceCall {
            this: 0x100,
            slot: 0xe4,
            args: vec![7, 1],
            data: Vec::new(),
            data_arg: None,
        };
        let leaf = Leaf::Texture {
            this: 0x200,
            stage: 1,
            texture: 0x300,
        };
        let m = |kind: u32, i: u32| (STORE_LOG_MARK + kind, i.to_le_bytes().to_vec());
        let log = vec![
            m(MARK_READ, 0),
            (0x5000, vec![1, 2, 3, 4]),
            m(MARK_READ, 1),
            m(MARK_DEVICE, 0),
            m(MARK_READ, 2),
            m(MARK_LEAF, 0),
            (0x6000, vec![9]),
            m(MARK_READ, 3),
            (
                STORE_LOG_MARK + STORE_LOG_QUIET,
                vec![0, 0x70, 0, 0, 4, 0, 0, 0],
            ),
            m(MARK_READ, 4),
        ];
        let reads = vec![
            (0x4000, vec![5, 5], 0x11),
            (0x5000, vec![1, 2, 3, 4], 0x12),
            (0x4000, vec![6, 6], 0x13),
            (0x5002, vec![3, 4, 0], 0x14),
            (0x7001, vec![1, 1], 0x15),
        ];
        let (r, _) = assemble(log, std::slice::from_ref(&call), &[leaf], &reads);
        assert_eq!(
            r.ops,
            [
                Op::Write(0x5000, vec![1, 2, 3, 4]),
                Op::Device(call),
                Op::Leaf(leaf),
                Op::Write(0x6000, vec![9]),
            ]
        );
        assert_eq!(
            r.inputs,
            [(0x4000, vec![5, 5], 0x11), (0x5002, vec![3, 4, 0], 0x14)]
        );
    }

    /// A draw kept and replayed: the first draw runs the body and keeps what it wrote and read; the next, with the
    /// same input, replays the write without the body; a changed input builds again; `check` runs the body on a
    /// hit and counts a body that now does something else.
    #[test]
    fn a_kept_draw_is_replayed_while_its_inputs_stay() {
        let mut e = Engine::new();
        let block = e.mem.alloc(0x100);
        let (input, out, scratch) = (block, block + 0x10, block + 0x20);
        e.mem.set_u32(input, 5);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let extra = Cell::new(0u32);
        let body = |e: &mut Engine| {
            runs.set(runs.get() + 1);
            let value = capture::word(e, input, 0);
            // The body's own write, then its read: not an input.
            e.mem.set_u32(scratch, 1);
            let _ = capture::word(e, scratch, 0);
            e.mem.set_u32(out, value + 1 + extra.get());
        };
        let key = (0x10, 1, 0x20, 0x30, 0);
        draw(&mut e, key, body);
        assert_eq!((runs.get(), e.mem.u32(out)), (1, 6));
        e.mem.set_u32(out, 0);
        e.mem.set_u32(scratch, 9);
        draw(&mut e, key, body);
        assert_eq!(runs.get(), 1, "replayed, not run: {}", summary_now());
        assert_eq!(
            (e.mem.u32(out), e.mem.u32(scratch)),
            (6, 1),
            "the writes replayed"
        );
        e.mem.set_u32(input, 7);
        draw(&mut e, key, body);
        assert_eq!(
            (runs.get(), e.mem.u32(out)),
            (2, 8),
            "an input changed: built again"
        );
        // Check: the body runs on a hit; a body that writes something else now is counted.
        set_mode_for_tests(Some(Mode::Check));
        extra.set(1);
        draw(&mut e, key, body);
        assert_eq!((runs.get(), e.mem.u32(out)), (3, 9));
        let s = summary_now();
        assert!(s.contains("1 replayed"), "{s}");
        assert!(s.contains("1 checked, 1 differed"), "{s}");
        assert!(s.contains("inputs changed 1"), "{s}");
        set_mode_for_tests(None);
        reset_for_tests();
    }

    thread_local! {
        /// The test hooks' log of what was asked, their answer to a record (`None`: plain), what they were told to
        /// drop, and their epoch.
        static ASKS: RefCell<Vec<Ask>> = const { RefCell::new(Vec::new()) };
        static RECORDED: Cell<Option<u64>> = const { Cell::new(Some(77)) };
        static DROPPED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
        static EPOCH: Cell<u64> = const { Cell::new(0) };
    }

    /// TEST DATA: the platform layer's side as a log (a record answered `RECORDED`, an item drawn); other threads'
    /// tests see their own empty log and plain answers, as without hooks.
    static TEST_HOOKS: draw_items::Hooks = draw_items::Hooks {
        begin: |_, ask| ASKS.with(|a| a.borrow_mut().push(ask)),
        end: |_| match ASKS.with(|a| a.borrow().last().copied()) {
            Some(Ask::Record { .. }) => {
                RECORDED.with(Cell::get).map_or(Done::Plain, Done::Recorded)
            }
            Some(Ask::Item { .. }) => Done::Drawn,
            None => Done::Plain,
        },
        interrupt: |_| (),
        forget: |_, ids| DROPPED.with(|d| d.borrow_mut().extend_from_slice(ids)),
        epoch: || EPOCH.with(Cell::get),
    };

    fn asks() -> Vec<Ask> {
        ASKS.with(|a| std::mem::take(&mut *a.borrow_mut()))
    }

    /// With items: a build's own commands go to the backend as a record of a new number (the old one dropped with
    /// it), a replay's as one item of it; a check run makes the item from the replay and then its old form beside
    /// it; a build whose commands could not be a record leaves its replays plain and its old number to be dropped;
    /// the backend's records gone (a new device) drop every record.
    #[test]
    fn a_replay_is_one_item_while_the_backend_keeps_its_record() {
        draw_items::install(&TEST_HOOKS);
        let mut e = Engine::new();
        let block = e.mem.alloc(0x100);
        let (input, out) = (block, block + 0x10);
        e.mem.set_u32(input, 5);
        reset_for_tests();
        let _ = asks();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let body = |e: &mut Engine| {
            runs.set(runs.get() + 1);
            let value = capture::word(e, input, 0);
            e.mem.set_u32(out, value + 1);
        };
        let key = (0x10, 1, 0x20, 0x30, 0);
        let record = |id, forget, check| Ask::Record { id, forget, check };
        let item = |id, check| Ask::Item {
            id,
            hash: 77,
            check,
        };
        draw(&mut e, key, body);
        assert_eq!(asks(), [record(1, 0, false)]);
        draw(&mut e, key, body);
        assert_eq!((asks(), runs.get()), (vec![item(1, false)], 1));
        e.mem.set_u32(input, 6);
        draw(&mut e, key, body);
        assert_eq!((asks(), runs.get()), (vec![record(2, 1, false)], 2));
        set_mode_for_tests(Some(Mode::Check));
        draw(&mut e, key, body);
        assert_eq!(asks(), [item(2, true), record(3, 2, true)]);
        assert_eq!((runs.get(), e.mem.u32(out)), (3, 7), "the old form drew");
        // A record that could not be sent: its replays plain, its number dropped later.
        set_mode_for_tests(Some(Mode::On));
        RECORDED.with(|r| r.set(None));
        e.mem.set_u32(input, 8);
        draw(&mut e, key, body);
        assert_eq!(asks(), [record(4, 3, false)]);
        draw(&mut e, key, body);
        assert_eq!(asks(), [], "replayed with its own commands");
        assert_eq!(runs.get(), 4);
        assert_eq!(KEEP.with(|k| k.borrow().forgotten.clone()), [3]);
        // A new device: every record built again.
        RECORDED.with(|r| r.set(Some(77)));
        EPOCH.with(|n| n.set(1));
        draw(&mut e, key, body);
        assert_eq!((asks(), runs.get()), (vec![record(5, 0, false)], 5));
        let s = summary_now();
        assert!(s.contains("items: 1 drawn, 0 plain"), "{s}");
        assert!(s.contains("backend records: 4 sent, 1 plain"), "{s}");
        assert!(
            s.contains("check items: 1 beside their old form, 0 dropped"),
            "{s}"
        );
        assert!(s.contains("records dropped for a new device: 1"), "{s}");
        EPOCH.with(|n| n.set(0));
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// The check mode's report names the first op that differs, or the lengths.
    #[test]
    fn the_first_difference_is_named() {
        let a = [Op::Write(1, vec![1]), Op::Write(2, vec![2])];
        let b = [Op::Write(1, vec![1]), Op::Write(2, vec![3])];
        assert!(first_difference(&a, &a).is_none());
        assert!(first_difference(&a, &b).unwrap().starts_with("op 1:"));
        assert!(first_difference(&a, &a[..1])
            .unwrap()
            .contains("2 ops kept, 1 made"));
    }
}
