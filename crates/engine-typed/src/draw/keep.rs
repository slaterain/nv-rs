//! Kept draw records (`NV_DRAW_KEEP`, off by default; `1` on, `check` on with every reuse checked): the world
//! draw's plain geometries (the batch renderer's `render_standard`, `00b98e80`) built once and kept. The first time a
//! geometry is drawn in a pass, its set-up and draw run as always while three logs note what they do: the state
//! cache's setters it calls (`super::super::shader::state`'s leaf setters, kept as calls: the cache decides again
//! what the device gets), the device calls it makes outside them (`engine_core::real_threads`' recorder, with the
//! bytes of each structure passed by address), and its writes of guest memory outside them (the store log, its own
//! stack locals left out), all in one order; and the reads it makes through the draw's accessors
//! ([`super::capture`]: everything the rest of the frame may change, by #574's census) with the bytes read. The
//! next time, when none of those inputs and none of the geometry's and its properties' own bytes changed, the record
//! is replayed: its writes, setter calls and device calls, in order, without the set-up's work.
//!
//! What a player sees is the same: a record is used only when every input it read is the same, and it does what the
//! set-up did. With `check`, every replay is also checked: the set-up runs as always and what it did is compared
//! with the record (the counters name the first difference); the set-up's own result is what the frame gets.
//!
//! Not kept: pass types with their own set-ups (0xfc, 0xfd, 0x102, 0xca, 0xd1) and 4, 5, 0x250, 0x251, a shader
//! other than the shadow light shader's set-ups, a skinned geometry, and everything while the drawing thread's
//! packet is open (stage 1 of the records).

use super::capture;
use engine_core::mem::STORE_LOG_MARK;
use engine_core::real_threads::{self as rt, DeviceCall};
use engine_core::Engine;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// `NV_DRAW_KEEP`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    Check,
}

/// The process's mode: `NV_DRAW_KEEP` (`1` or `on`, `check`; anything else off), read once; tests set it
/// ([`set_mode_for_tests`]).
pub fn mode() -> Mode {
    static MODE: std::sync::OnceLock<Mode> = std::sync::OnceLock::new();
    if let Some(m) = TEST_MODE.with(Cell::get) {
        return m;
    }
    *MODE.get_or_init(|| match std::env::var("NV_DRAW_KEEP").as_deref() {
        Ok("1" | "on") => Mode::On,
        Ok("check") => Mode::Check,
        _ => Mode::Off,
    })
}

thread_local! {
    static TEST_MODE: Cell<Option<Mode>> = const { Cell::new(None) };
}

/// Tests: this thread's mode, in place of `NV_DRAW_KEEP`.
#[doc(hidden)]
pub fn set_mode_for_tests(mode: Option<Mode>) {
    TEST_MODE.with(|m| m.set(mode));
}

/// A state cache setter's call, as a record keeps it (`super::super::shader::state`'s leaf setters).
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

/// A geometry drawn in a pass: the geometry, the pass type, the shader, the property state.
pub type Key = (u32, u32, u32, u32);

/// A kept draw.
struct Record {
    /// The geometry's and its properties' own bytes as they were (a change of any breaks the record).
    statics: Vec<u8>,
    /// The inputs the set-up read through the accessors, with their bytes at the time (reads of what the set-up
    /// itself had written left out).
    inputs: Vec<(u32, Vec<u8>)>,
    ops: Vec<Op>,
}

/// The counters (`NV_DRAW_KEEP` reports them every [`REPORT_EVERY`] draws it saw).
#[derive(Default)]
struct Stats {
    seen: u64,
    replayed: u64,
    built_new: u64,
    built_statics: u64,
    built_inputs: u64,
    checked: u64,
    mismatched: u64,
    /// Which input address broke records most (the inputs' addresses, counted).
    broke_at: HashMap<u32, u64>,
    first_mismatch: Option<String>,
}

const REPORT_EVERY: u64 = 1_000_000;

#[derive(Default)]
struct Keep {
    records: HashMap<Key, Record>,
    stats: Stats,
    /// The setter calls of the record being built, and how deep in setters it is.
    leaves: Option<Vec<Leaf>>,
    leaf_depth: u32,
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
                e.mem.store_log_mark(MARK_LEAF, leaves.len() as u32);
                leaves.push(leaf);
                true
            }
            None => false,
        }
    });
    if outer {
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
    rt::device_record_pause(on);
    capture::keep_reads_pause(on);
}

/// The geometry's and its properties' own bytes a record keeps: the geometry (0xc0 bytes: its bound, world
/// transform, data and skin), the property state's five words, the first property, the shader property (0x100
/// bytes: flags, alpha and fade, textures, material values), the alpha property and the shader.
fn statics(e: &Engine, key: Key) -> Vec<u8> {
    let (geometry, _, shader, state) = key;
    let word = |a: u32| e.mem.u32(a);
    let ranges = [
        (geometry, 0xc0),
        (state, 0x14),
        (word(state), 0x20),
        (word(state.wrapping_add(0xc)), 0x100),
        (word(state.wrapping_add(0x10)), 0x20),
        (shader, 0x80),
    ];
    let mut out = Vec::with_capacity(0x2c0);
    for (at, len) in ranges {
        let start = out.len();
        out.resize(start + len, 0);
        if at == 0 || e.mem.try_read(at, &mut out[start..]).is_err() {
            out[start..].fill(0xa5);
        }
    }
    out
}

/// Draws one geometry of a pass: through its kept record when every input is unchanged, else `body` (the set-up
/// and draw) runs and the record is built again. Off, `body` alone.
pub fn draw(e: &mut Engine, key: Key, mut body: impl FnMut(&mut Engine)) {
    let mode = mode();
    if mode == Mode::Off || capture::active() || building() {
        body(e);
        return;
    }
    let snapshot = statics(e, key);
    // What breaks a kept record, if anything: None reuses it.
    let verdict = KEEP.with(|k| {
        let mut k = k.borrow_mut();
        let k = &mut *k;
        k.stats.seen += 1;
        match k.records.get(&key) {
            None => Some(Break::New),
            Some(r) if r.statics != snapshot => Some(Break::Statics),
            Some(r) => r
                .inputs
                .iter()
                .find(|(at, bytes)| !same_bytes(e, *at, bytes))
                .map(|(at, _)| Break::Input(*at)),
        }
    });
    match verdict {
        None if mode == Mode::On => {
            replay(e, key);
            count(|s| s.replayed += 1);
        }
        None => {
            // Check: the set-up runs as always; what it did against the record.
            let built = build(e, &mut body);
            KEEP.with(|k| {
                let mut k = k.borrow_mut();
                let k = &mut *k;
                k.stats.checked += 1;
                if let Some(old) = k.records.get(&key) {
                    if let Some(diff) = first_difference(&old.ops, &built.ops) {
                        k.stats.mismatched += 1;
                        if k.stats.first_mismatch.is_none() {
                            k.stats.first_mismatch = Some(format!("{key:08x?}: {diff}"));
                        }
                    }
                }
                k.records.insert(
                    key,
                    Record {
                        statics: snapshot,
                        ..built
                    },
                );
            });
        }
        Some(why) => {
            let built = build(e, &mut body);
            KEEP.with(|k| {
                let mut k = k.borrow_mut();
                let k = &mut *k;
                match why {
                    Break::New => k.stats.built_new += 1,
                    Break::Statics => k.stats.built_statics += 1,
                    Break::Input(at) => {
                        k.stats.built_inputs += 1;
                        *k.stats.broke_at.entry(at).or_default() += 1;
                    }
                }
                k.records.insert(
                    key,
                    Record {
                        statics: snapshot,
                        ..built
                    },
                );
            });
        }
    }
    report();
}

/// Why a record was built again.
#[derive(Clone, Copy)]
enum Break {
    New,
    Statics,
    Input(u32),
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
fn build(e: &mut Engine, body: &mut impl FnMut(&mut Engine)) -> Record {
    // The stack for locals made first, so the set-up's locals lie in the range the store log leaves out.
    e.with_stack(4, |_, _| ());
    let (low, top) = e.locals_below();
    e.mem.store_log_begin(low, top);
    rt::device_record_begin();
    capture::keep_reads_begin();
    KEEP.with(|k| k.borrow_mut().leaves = Some(Vec::new()));
    BUILDING.with(|b| b.set(true));
    body(e);
    BUILDING.with(|b| b.set(false));
    let leaves = KEEP
        .with(|k| k.borrow_mut().leaves.take())
        .unwrap_or_default();
    let reads = capture::keep_reads_end();
    let calls = rt::device_record_end();
    let log = e.mem.store_log_end();
    assemble(log, &calls, &leaves, &reads)
}

/// The record from the logs: the writes, setter calls and device calls in the store log's order; the reads of bytes
/// the set-up had not written itself before, first read of each address only.
fn assemble(
    log: Vec<(u32, Vec<u8>)>,
    calls: &[DeviceCall],
    leaves: &[Leaf],
    reads: &[(u32, Vec<u8>)],
) -> Record {
    let mut ops = Vec::new();
    let mut inputs: Vec<(u32, Vec<u8>)> = Vec::new();
    let mut written = std::collections::HashSet::new();
    let mut read_at = std::collections::HashSet::new();
    for (at, bytes) in log {
        let index = |b: &[u8]| u32::from_le_bytes(b.try_into().unwrap_or([0xff; 4])) as usize;
        if at >= STORE_LOG_MARK {
            match at - STORE_LOG_MARK {
                MARK_DEVICE => ops.extend(calls.get(index(&bytes)).cloned().map(Op::Device)),
                MARK_LEAF => ops.extend(leaves.get(index(&bytes)).copied().map(Op::Leaf)),
                MARK_READ => {
                    if let Some((addr, value)) = reads.get(index(&bytes)) {
                        let own = (0..value.len() as u32)
                            .all(|i| written.contains(&addr.wrapping_add(i)));
                        if !own && read_at.insert(*addr) {
                            inputs.push((*addr, value.clone()));
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        for i in 0..bytes.len() as u32 {
            written.insert(at.wrapping_add(i));
        }
        ops.push(Op::Write(at, bytes));
    }
    Record {
        statics: Vec::new(),
        inputs,
        ops,
    }
}

/// Replays the record of `key`: its writes, setter calls and device calls, in order.
fn replay(e: &mut Engine, key: Key) {
    let ops = KEEP.with(|k| k.borrow().records.get(&key).map(|r| r.ops.clone()));
    for op in ops.iter().flatten() {
        match op {
            Op::Write(at, bytes) => e.mem.write(*at, bytes),
            Op::Leaf(leaf) => replay_leaf(e, *leaf),
            Op::Device(call) => {
                rt::device_replay(e, call);
            }
        }
    }
}

fn replay_leaf(e: &mut Engine, leaf: Leaf) {
    use crate::shader::state;
    use engine_core::ptr::Ptr;
    match leaf {
        Leaf::RenderState {
            this,
            state: s,
            value,
            unused,
            lock,
        } => state::set_render_state(e, Ptr::new(this), s, value, unused, lock),
        Leaf::StageState {
            this,
            stage,
            state: s,
            value,
            unused,
        } => state::set_stage_state(e, Ptr::new(this), stage, s, value, unused),
        Leaf::SamplerState {
            this,
            stage,
            state: s,
            value,
            unused,
        } => state::set_sampler_state(e, Ptr::new(this), stage, s, value, unused),
        Leaf::Texture {
            this,
            stage,
            texture,
        } => state::set_texture(e, Ptr::new(this), stage, texture),
        Leaf::VertexShader { this, shader, keep } => {
            state::set_vertex_shader(e, Ptr::new(this), shader, keep);
        }
        Leaf::PixelShader { this, shader, keep } => {
            state::set_pixel_shader(e, Ptr::new(this), shader, keep);
        }
        Leaf::Declaration {
            this,
            declaration,
            keep,
        } => state::set_declaration(e, Ptr::new(this), declaration, keep),
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
    let top = s
        .broke_at
        .iter()
        .max_by_key(|(at, n)| (**n, std::cmp::Reverse(**at)))
        .map_or_else(|| "none".to_string(), |(at, n)| format!("{at:08x} ({n})"));
    format!(
        "NV_DRAW_KEEP: {} draws: {} replayed, {} built (new {}, own bytes changed {}, inputs changed {}); {} checked, {} differed{}; {records} records; top input that broke one: {top}",
        s.seen,
        s.replayed,
        s.built_new + s.built_statics + s.built_inputs,
        s.built_new,
        s.built_statics,
        s.built_inputs,
        s.checked,
        s.mismatched,
        s.first_mismatch.as_deref().map_or_else(String::new, |m| format!(" (first: {m})")),
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
        ];
        let reads = vec![
            (0x4000, vec![5, 5]),
            (0x5000, vec![1, 2, 3, 4]),
            (0x4000, vec![6, 6]),
            (0x5002, vec![3, 4, 0]),
        ];
        let r = assemble(log, &[call.clone()], &[leaf], &reads);
        assert_eq!(
            r.ops,
            [
                Op::Write(0x5000, vec![1, 2, 3, 4]),
                Op::Device(call),
                Op::Leaf(leaf),
                Op::Write(0x6000, vec![9]),
            ]
        );
        assert_eq!(r.inputs, [(0x4000, vec![5, 5]), (0x5002, vec![3, 4, 0])]);
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
        let key = (0x10, 1, 0x20, 0x30);
        draw(&mut e, key, body);
        assert_eq!((runs.get(), e.mem.u32(out)), (1, 6));
        e.mem.set_u32(out, 0);
        e.mem.set_u32(scratch, 9);
        draw(&mut e, key, body);
        assert_eq!(runs.get(), 1, "replayed, not run");
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
