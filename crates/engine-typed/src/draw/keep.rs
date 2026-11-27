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
//! Check mode is not the pure old path: a draw with a record first replays it (its writes are put back as they were
//! before the set-up runs, but what its setter calls and kept calls wrote stays in the state caches and the shader
//! statics) and then runs its set-up, which makes the same calls again.
//! The direct proof that `1` draws as off does is a stepped `1` recording against a stepped off recording of the
//! same route (`record_compare --digests`, which expands items through `d3d9_stream::Kept`).
//!
//! Only the world draw is kept: the world pass's two halves (`00b6ba20`, `00b6b930`: [`world_draw`]); every other
//! scene (first person, the water reflection, the extra scenes, 3D menus, the local map, extra cameras and the
//! minimap: `00b6c0d0`, [`outside_world_draw`]) draws as always, so a record never holds another camera's values.
//! A key whose record breaks [`BACK_OFF_AFTER`] times in a row is drawn plainly for its next [`REST_DRAWS`] draws
//! (doubling while it keeps breaking, up to [`MOST_REST_DRAWS`]): a build costs several plain set-ups, and a moving
//! object's record never replays.
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
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

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
    /// The pixel entry 19 (`super::super::shader::geometry::pixel_entry_19`): the frame's clip plane, the eye and the
    /// near and far distances.
    Entry19,
    /// The object-to-cube matrix (`super::super::shader::geometry::obj_to_cube`, `011fecc8`): the camera added to
    /// the transform's translation, or the renderer's matrix of the frame (the live runs' first difference: a
    /// record kept vertex register 10 of an older camera).
    ObjToCube {
        this: u32,
        block: u32,
        pass_type: u32,
        flag: u32,
        transform: u32,
    },
    /// The shadow projection (`super::super::shader::geometry::shadow_proj`, `011fd968`): with its flag, the camera.
    ShadowProj {
        this: u32,
        block: u32,
        flag: u32,
        transform: u32,
    },
    /// The vertex entry 17 (`super::super::shader::geometry::vertex_entry_17`): the bound's centre less the camera.
    Entry17 {
        this: u32,
        object: u32,
    },
    /// The simple shadow clip planes (`super::super::shader::geometry::simple_shadow_clip_planes`, types 0x231 to
    /// 0x234): the light's cache of the frame, or the renderer's view and the camera.
    ClipPlanes {
        this: u32,
        pass_type: u32,
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

/// A geometry drawn in a pass: the geometry, the pass type, the shader, the property state, and the draw's order-dependent
/// choices (its flags, the pass's sort flag, whether a restore is owed: `render_standard`).
pub type Key = (u32, u32, u32, u32, u32);

/// A kept draw.
struct Record {
    /// The geometry's and its properties' own bytes as they were (a change of any breaks the record).
    statics: Vec<u8>,
    /// Those bytes and the inputs as a replay compares them ([`Spans`]).
    spans: Spans,
    /// The inputs the set-up read through the accessors, with their bytes at the time (reads of what the set-up
    /// itself had written left out).
    inputs: Vec<(u32, Vec<u8>, u32)>,
    /// What the set-up did, shared with a replay running it (no copy, no borrow held while it runs).
    ops: Rc<[Op]>,
    /// Its writes, kept calls and device calls, counted (the counters' line: a replay's work).
    counts: [u32; 3],
    /// False when the set-up reached the device outside the calls the recorder saw (a call by the vtable from
    /// translated code, a buffer's lock): the record would leave that out, so it is never replayed.
    replayable: bool,
    /// The draw count ([`Stats::seen`]) when it was last built or replayed: records unused for long are dropped.
    last_used: u64,
    /// The backend's record of its draw's own commands (`engine_core::draw_items`): its number and their shape's
    /// hash; none when they could not be sent as a record (a setter's send between them, or no draw).
    item: Option<(u32, u64)>,
    /// How many times in a row the key's record broke (built again with a record there), and how many of the key's
    /// next draws run plainly ([`BACK_OFF_AFTER`]).
    streak: u32,
    rest: u32,
}

/// After this many breaks in a row a key rests: its next [`REST_DRAWS`] draws run plainly (no logs, no compare),
/// doubled at each further break up to [`MOST_REST_DRAWS`]; a replay ends the streak.
pub const BACK_OFF_AFTER: u32 = 4;
pub const REST_DRAWS: u32 = 64;
pub const MOST_REST_DRAWS: u32 = 4096;

/// The draws a key rests after its `streak`th break in a row (0 below [`BACK_OFF_AFTER`]).
fn rest_after(streak: u32) -> u32 {
    if streak < BACK_OFF_AFTER {
        return 0;
    }
    (REST_DRAWS << (streak - BACK_OFF_AFTER).min(16)).min(MOST_REST_DRAWS)
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
    /// By (input address, the exe function that read it): how often it broke a record, and how often it was the only
    /// input that changed (the records it alone kept from replaying); since the last line ([`report_now`]).
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
    /// Draws run plainly while their key rested ([`BACK_OFF_AFTER`]), and the times a key began to rest.
    rested: u64,
    rests: u64,
    /// The time spent in builds, replays and resting plain draws (each set-up and draw with its logs, look-ups and
    /// compares), nanoseconds: a build's cost against a plain set-up's.
    build_ns: u64,
    replay_ns: u64,
    rest_ns: u64,
    /// Of the replays' time, the look-up and compares before the record's ops ran; the writes, kept calls and device
    /// calls the replays ran.
    look_ns: u64,
    replay_ops: [u64; 3],
    /// Replayable draws drawn plainly to time the plain set-up ([`SAMPLE_EVERY`]), and their time.
    sampled: u64,
    sample_ns: u64,
}

const REPORT_EVERY: u64 = 1_000_000;

#[derive(Default)]
struct Keep {
    records: HashMap<Key, Record, BuildHasherDefault<KeyHasher>>,
    stats: Stats,
    /// The setter calls of the record being built, and how deep in setters it is.
    leaves: Option<Vec<Leaf>>,
    leaf_depth: u32,
    /// The backend's epoch the records are of, and the numbers of backend records no record of this side holds any
    /// more, not yet sent to be dropped.
    epoch: u64,
    forgotten: Vec<u32>,
}

/// The next backend record's number: one counter for the run, since the backend keeps one table for every thread.
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// A new backend record's number (never 0).
fn next_id() -> u32 {
    loop {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        if id != 0 {
            return id;
        }
    }
}

thread_local! {
    static KEEP: RefCell<Keep> = RefCell::new(Keep::default());
    /// True while this thread builds a record (the setters' one test).
    static BUILDING: Cell<bool> = const { Cell::new(false) };
    /// How deep this thread is in the world pass's halves ([`world_draw`]); 0 in any other scene.
    static WORLD: Cell<u32> = const { Cell::new(0) };
}

/// Restores [`WORLD`] as it was, also when a stop unwinds.
struct WorldScope(u32);

impl Drop for WorldScope {
    fn drop(&mut self) {
        WORLD.with(|w| w.set(self.0));
    }
}

/// `f` as the world pass's draw (the halves `00b6ba20` and `00b6b930` around the accumulator's slot): only there
/// are draws kept.
pub fn world_draw<R>(f: impl FnOnce() -> R) -> R {
    let _scope = WorldScope(WORLD.with(|w| w.replace(w.get() + 1)));
    f()
}

/// `f` as another scene's draw (`00b6c0d0`: water reflections, menus, the extra scenes, extra cameras), also when
/// one is drawn from inside the world pass: nothing kept there.
pub fn outside_world_draw<R>(f: impl FnOnce() -> R) -> R {
    let _scope = WorldScope(WORLD.with(|w| w.replace(0)));
    f()
}

/// Whether this thread draws the world pass now ([`world_draw`]).
#[inline]
fn in_world_draw() -> bool {
    WORLD.with(Cell::get) != 0
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
        // The logs paused first: the run's flush reaches the device's state outside a noted call, which the recorder
        // would count as a call it cannot make again. Then the run of the draw's own commands held so far ends here
        // (`engine_core::draw_items`), as at the replay.
        pause(e, true);
        draw_items::interrupt(e);
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

/// What a replay compares ([`spans_same`]): the record's own bytes' ranges as [`statics`] read them at the build and
/// its inputs, sorted by address, adjacent and overlapping ones merged (`spans`, their bytes back to back in
/// `bytes`), and the own bytes' ranges that could not be read then (`unread`), which must still not be (or read as
/// [`statics`]' fill). The ranges are kept as they were: every pointer [`statics_ranges`] follows lies in a range
/// before it (the property state's words, the geometry's data, the data's render data, the property's texture count,
/// arrays and effect), so while those bytes stay, the ranges are the same.
#[derive(Default)]
struct Spans {
    spans: Vec<(u32, u32)>,
    bytes: Vec<u8>,
    unread: Vec<(u32, u32)>,
}

/// [`Spans`] from a build: its own bytes (`statics` as read from `ranges`) and its inputs.
fn make_spans(
    e: &Engine,
    ranges: &[(u32, u32)],
    statics: &[u8],
    inputs: &[(u32, Vec<u8>, u32)],
) -> Spans {
    let mut out = Spans::default();
    let mut pieces: Vec<(u32, &[u8])> = Vec::with_capacity(ranges.len() + inputs.len());
    let mut from = 0usize;
    for &(at, len) in ranges {
        let Some(bytes) = statics.get(from..from + len as usize) else {
            break;
        };
        from += len as usize;
        if at == 0 {
            continue;
        }
        let mut now = [0u8; 0x100];
        match now.get_mut(..len as usize) {
            Some(now) if e.mem.try_read(at, now).is_ok() => pieces.push((at, bytes)),
            _ => out.unread.push((at, len)),
        }
    }
    pieces.extend(inputs.iter().map(|(at, bytes, _)| (*at, bytes.as_slice())));
    pieces.sort_by_key(|&(at, _)| at);
    for (at, bytes) in pieces {
        let end = u64::from(at) + bytes.len() as u64;
        if let Some((start, len)) = out.spans.last_mut() {
            let last_end = u64::from(*start) + u64::from(*len);
            // Adjacent or overlapping, and the shared bytes alike (read at another moment of the build, they could
            // differ only if another thread wrote them: then both stay, and the record breaks next time).
            if u64::from(at) <= last_end {
                let shared = (last_end - u64::from(at)).min(bytes.len() as u64) as usize;
                let tail = out.bytes.len() - shared;
                if out.bytes[tail..] == bytes[..shared] || shared == 0 {
                    if end > last_end {
                        out.bytes.extend_from_slice(&bytes[shared..]);
                        *len += n32(bytes.len() - shared);
                    }
                    continue;
                }
            }
        }
        out.spans.push((at, n32(bytes.len())));
        out.bytes.extend_from_slice(bytes);
    }
    out
}

/// Whether every span of `s` holds its bytes and every unread range is still unread ([`Spans`]): a replay's test.
fn spans_same(e: &Engine, s: &Spans) -> bool {
    let mut buf = [0u8; 0x100];
    let mut from = 0usize;
    for &(at, len) in &s.spans {
        let len = len as usize;
        let want = &s.bytes[from..from + len];
        from += len;
        let mut done = 0usize;
        while done < len {
            let n = (len - done).min(buf.len());
            let now = &mut buf[..n];
            if e.mem.try_read(at.wrapping_add(n32(done)), now).is_err()
                || now != &want[done..done + n]
            {
                return false;
            }
            done += n;
        }
    }
    s.unread
        .iter()
        .all(|&(at, len)| match buf.get_mut(..len as usize) {
            Some(now) => e.mem.try_read(at, now).is_err() || now.iter().all(|&b| b == 0xa5),
            None => false,
        })
}

/// Whether `kept` is what [`statics`] makes now, compared where the bytes lie (each range read into the stack, no
/// copy kept): a replay's test of the record's own bytes.
fn statics_same(e: &Engine, key: Key, kept: &[u8]) -> bool {
    // Each range is at most 0xc8 bytes ([`statics_ranges`]).
    let mut buf = [0u8; 0x100];
    let mut from = 0usize;
    for (at, len) in statics_ranges(e, key) {
        let len = len as usize;
        let (Some(want), Some(now)) = (kept.get(from..from + len), buf.get_mut(..len)) else {
            return false;
        };
        from += len;
        if at == 0 || e.mem.try_read(at, now).is_err() {
            if want.iter().any(|&b| b != 0xa5) {
                return false;
            }
        } else if now != want {
            return false;
        }
    }
    from == kept.len()
}

/// The texture arrays of the shader property (`+0xac`, six arrays of `+0xa8`'s count of texture pointers) a record
/// covers up to this many pointers each.
const MOST_TEXTURES: u32 = 16;

/// Where [`statics`] reads: (address, length).
fn statics_ranges(e: &Engine, key: Key) -> [(u32, u32); 16] {
    let (geometry, _, shader, state, _) = key;
    let word = |a: u32| {
        let mut b = [0u8; 4];
        e.mem
            .try_read(a, &mut b)
            .map_or(0, |()| u32::from_le_bytes(b))
    };
    let property = word(state.wrapping_add(0xc));
    let count = {
        let mut b = [0u8; 2];
        e.mem
            .try_read(property.wrapping_add(0xa8), &mut b)
            .map_or(0, |()| u32::from(u16::from_le_bytes(b)))
    };
    let array = |i: u32| {
        (
            word(property.wrapping_add(0xac + 4 * i)),
            4 * count.min(MOST_TEXTURES),
        )
    };
    let effect = word(property.wrapping_add(0xdc));
    [
        // The texture pointers in the property's six arrays (the texture set-up stores them into the pass's stages,
        // a write the record replays as it was): a texture replaced in an array (a retexture, a reload) breaks it.
        array(0),
        array(1),
        array(2),
        array(3),
        array(4),
        array(5),
        // The texture effect's texture (`+0xdc`, its `+0x6c`), which the shadow pass types' stage gets.
        (
            if effect == 0 {
                0
            } else {
                effect.wrapping_add(0x6c)
            },
            4,
        ),
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
    // With `NV_BLOCK_CHECK` the set-up's locals are heap blocks (`Engine::locals_below` has no range to leave out):
    // its writes there would be replayed into freed blocks, so nothing is kept (review of #1108).
    if mode == Mode::Off || packets || building() || !in_world_draw() || e.mem.block_check_on() {
        body(e);
        return;
    }
    let epoch = draw_items::epoch();
    let start = Instant::now();
    // One look-up says what this draw is: a rest, a replay of the record's ops, or a build (and why).
    let plan = KEEP.with(|k| {
        let mut k = k.borrow_mut();
        let k = &mut *k;
        follow_epoch(k, epoch);
        k.stats.seen += 1;
        let seen = k.stats.seen;
        let Some(r) = k.records.get_mut(&key) else {
            return Plan::Build(Break::New, None);
        };
        // A key that kept breaking rests: drawn plainly, its record (still compared when the rest ends) left alone.
        if r.rest > 0 {
            r.rest -= 1;
            k.stats.rested += 1;
            return Plan::Rest;
        }
        if !r.replayable {
            return Plan::Build(Break::Unkeepable, r.item);
        }
        if let Some(why) = verdict(e, key, r) {
            return Plan::Build(why, r.item);
        }
        r.last_used = seen;
        r.streak = 0;
        // One replayable draw in [`SAMPLE_EVERY`] runs plainly instead (`On` only): the plain set-up's cost of the
        // same draws, for the counters' line.
        if mode == Mode::On && seen % SAMPLE_EVERY == 0 {
            k.stats.sampled += 1;
            return Plan::Sample;
        }
        // A check run's replay is counted with its build.
        if mode == Mode::On {
            k.stats.replayed += 1;
            for (all, n) in k.stats.replay_ops.iter_mut().zip(r.counts) {
                *all += u64::from(n);
            }
        }
        Plan::Replay(Rc::clone(&r.ops), r.item)
    });
    match plan {
        Plan::Rest => {
            body(e);
            count(|s| s.rest_ns += nanos(start));
        }
        Plan::Sample => {
            body(e);
            count(|s| s.sample_ns += nanos(start));
        }
        Plan::Replay(ops, item) if mode == Mode::On => {
            let looked = nanos(start);
            // The record's replay; its own commands one item when the backend has them.
            match item {
                Some((id, hash)) => {
                    let ask = Ask::Item {
                        id,
                        hash,
                        check: false,
                    };
                    let done = with_held(e, ask, |e| replay(e, &ops));
                    count(|s| match done {
                        Done::Drawn => s.items += 1,
                        Done::Other => s.items_other += 1,
                        _ => s.items_plain += 1,
                    });
                }
                None => replay(e, &ops),
            }
            count(|s| {
                s.replay_ns += nanos(start);
                s.look_ns += looked;
            });
        }
        Plan::Replay(ops, item) => {
            // Check: the item this draw would have been, made from the record's replay as `On` makes it; then the
            // set-up runs as always (its commands the old form beside the item) and what it did is compared with
            // the record.
            let (ranges, snapshot) = (statics_ranges(e, key), statics(e, key));
            let beside = item.is_some_and(|(id, hash)| {
                let ask = Ask::Item {
                    id,
                    hash,
                    check: true,
                };
                // The replay's writes undone before the set-up runs, so the set-up finds the game's memory as an off
                // run leaves it: a replay that cleared the owed restore (`011ad8ec`) made the set-up skip the restore,
                // a difference of check mode alone.
                let undo = bytes_under_writes(e, &ops);
                let done = with_held(e, ask, |e| replay(e, &ops));
                for (at, bytes) in undo.iter().rev() {
                    e.mem.write(*at, bytes);
                }
                count(|s| match done {
                    Done::Drawn => s.check_items += 1,
                    _ => s.check_items_dropped += 1,
                });
                done == Done::Drawn
            });
            let (built, sent) = build_sent(e, key, &mut body, item, beside);
            let spans = make_spans(e, &ranges, &snapshot, &built.inputs);
            KEEP.with(|k| {
                let mut k = k.borrow_mut();
                let k = &mut *k;
                k.stats.checked += 1;
                if let Some(diff) = first_difference(&ops, &built.ops) {
                    k.stats.mismatched += 1;
                    *k.stats
                        .differed_at
                        .entry(difference_kind(&ops, &built.ops))
                        .or_default() += 1;
                    if k.stats.first_mismatch.is_none() {
                        k.stats.first_mismatch = Some(format!("{key:08x?}: {diff}"));
                    }
                }
                let seen = k.stats.seen;
                k.records.insert(
                    key,
                    Record {
                        statics: snapshot,
                        spans,
                        last_used: seen,
                        item: sent,
                        ..built
                    },
                );
                k.stats.build_ns += nanos(start);
            });
        }
        Plan::Build(why, old) => {
            let (ranges, snapshot) = (statics_ranges(e, key), statics(e, key));
            let (built, sent) = build_sent(e, key, &mut body, old, false);
            let spans = make_spans(e, &ranges, &snapshot, &built.inputs);
            KEEP.with(|k| {
                let mut k = k.borrow_mut();
                let k = &mut *k;
                // A record there broke: one more in the streak (a new key starts none).
                let streak = match why {
                    Break::New => 0,
                    _ => k.records.get(&key).map_or(0, |r| r.streak) + 1,
                };
                let rest = rest_after(streak);
                k.stats.rests += u64::from(rest != 0);
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
                let seen = k.stats.seen;
                k.records.insert(
                    key,
                    Record {
                        statics: snapshot,
                        spans,
                        last_used: seen,
                        item: sent,
                        streak,
                        rest,
                        ..built
                    },
                );
                k.stats.build_ns += nanos(start);
            });
        }
    }
    after_draw(e);
}

/// What one draw of a key is ([`draw`]'s one look-up).
enum Plan {
    /// The key rests: drawn plainly.
    Rest,
    /// A replayable draw drawn plainly to time the plain set-up ([`SAMPLE_EVERY`]).
    Sample,
    /// The record's ops and backend record.
    Replay(Rc<[Op]>, Option<(u32, u64)>),
    /// Built again: why, and the backend record there was.
    Build(Break, Option<(u32, u64)>),
}

/// One replayable draw in this many is drawn plainly in `On` to time the plain set-up ([`Plan::Sample`]).
pub const SAMPLE_EVERY: u64 = 64;

/// What breaks the record `r` of `key` now, if anything: None reuses it. Its own bytes compared where they lie (no
/// copy made), then its inputs.
fn verdict(e: &Engine, key: Key, r: &Record) -> Option<Break> {
    if spans_same(e, &r.spans) {
        return None;
    }
    // Something changed: which, for the counters (any other difference rebuilds it as its own bytes').
    if !statics_same(e, key, &r.statics) {
        return Some(Break::Statics);
    }
    let Some((at, _, reader)) = r
        .inputs
        .iter()
        .find(|(at, bytes, _)| !same_bytes(e, *at, bytes))
    else {
        return Some(Break::Statics);
    };
    let others = r
        .inputs
        .iter()
        .any(|(a, b, _)| a != at && !same_bytes(e, *a, b));
    Some(Break::Input(*at, *reader, !others))
}

/// After each kept draw: records dropped when there are too many, the backend records no record holds sent to be
/// dropped (in batches of [`FORGET_BATCH`]), and the counters' line every [`REPORT_EVERY`] draws.
fn after_draw(e: &mut Engine) {
    let ids = KEEP.with(|k| {
        let mut k = k.borrow_mut();
        let k = &mut *k;
        evict_from(k, MAX_RECORDS, STALE_AFTER);
        report_now(k);
        (k.forgotten.len() >= FORGET_BATCH).then(|| std::mem::take(&mut k.forgotten))
    });
    if let Some(ids) = ids {
        draw_items::forget(e, &ids);
    }
}

/// `body` with the draw's own commands held for `ask` (`engine_core::draw_items`): what came of it. The holding
/// ends also when a stop or a C++ exception (a panic here: `engine_core`'s `cxx_exception`) unwinds through `body`
/// (what was held goes out as it came), and the unwind goes on (review of #1108).
fn with_held(e: &mut Engine, ask: Ask, body: impl FnOnce(&mut Engine)) -> Done {
    draw_items::begin(e, ask);
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(e)));
    let done = draw_items::end(e);
    match ran {
        Ok(()) => done,
        Err(stop) => std::panic::resume_unwind(stop),
    }
}

/// [`build`] with its draw's own commands sent as a backend record (a new number; the record's `old` one dropped
/// with it): the record, and its backend record when its commands could be sent as one. `beside`: a check run's
/// item of the old record was sent just before, so these are its old form. An old backend record not dropped with
/// the new one's sending is dropped later ([`after_draw`]).
fn build_sent(
    e: &mut Engine,
    key: Key,
    body: &mut impl FnMut(&mut Engine),
    old: Option<(u32, u64)>,
    beside: bool,
) -> (Record, Option<(u32, u64)>) {
    let id = next_id();
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
/// each is built again and sent anew. `now`: the backend's epoch.
fn follow_epoch(k: &mut Keep, now: u64) {
    if k.epoch != now {
        k.epoch = now;
        if !k.records.is_empty() {
            k.stats.epochs += 1;
        }
        k.records.clear();
        k.forgotten.clear();
    }
}

const FORGET_BATCH: usize = 256;

/// Above `most` records ([`MAX_RECORDS`]: [`after_draw`]), those unused in the last `stale` draws ([`STALE_AFTER`]) are
/// dropped (geometry freed, a cell unloaded, a game loaded: their keys are dead or about to be reused), their backend
/// records with them; if still above
/// (many cells passed through quickly, everything recent), the least recently used go down to three quarters of
/// `most`, so the next frames rebuild only those, not every visible draw at once (review of #1108).
fn evict_from(k: &mut Keep, most: usize, stale: u64) {
    if k.records.len() <= most {
        return;
    }
    drop_used_before(k, k.stats.seen.saturating_sub(stale));
    if k.records.len() > most {
        let keep = (most - most / 4).max(1);
        let mut used: Vec<u64> = k.records.values().map(|r| r.last_used).collect();
        // `keep` < `used.len()` here; the records used at or after the `keep`th latest use stay (each draw is
        // one key's, so two records share a use only in tests).
        let at = used.len() - keep;
        let (_, cut, _) = used.select_nth_unstable(at);
        let cut = *cut;
        drop_used_before(k, cut);
    }
}

/// Drops the records last used before draw `cutoff`, their backend records with them.
fn drop_used_before(k: &mut Keep, cutoff: u64) {
    let forgotten = &mut k.forgotten;
    k.records.retain(|_, r| {
        let keep = r.last_used >= cutoff;
        if !keep {
            forgotten.extend(r.item.map(|(id, _)| id));
        }
        keep
    });
}

const MAX_RECORDS: usize = 40_000;
const STALE_AFTER: u64 = 2_000_000;

/// A length or an index as a guest word (they are far below 4 G here).
fn n32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Nanoseconds since `start`.
fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
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
    capture::keep_reads_begin(low, top);
    KEEP.with(|k| {
        let mut k = k.borrow_mut();
        k.leaves = Some(Vec::new());
        k.leaf_depth = 0;
    });
    BUILDING.with(|b| b.set(true));
    // Everything begun above ends also when a stop or a C++ exception (a panic here) unwinds through the set-up,
    // and the unwind goes on: no log, flag or accessor state outlives the build (review of #1108).
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(e)));
    BUILDING.with(|b| b.set(false));
    let audited = if audit {
        e.mem.read_audit_end()
    } else {
        Vec::new()
    };
    let leaves = KEEP.with(|k| {
        let mut k = k.borrow_mut();
        k.leaf_depth = 0;
        k.leaves.take()
    });
    let reads = capture::keep_reads_end();
    let calls = rt::device_record_end();
    let log = e.mem.store_log_end();
    if let Err(stop) = ran {
        std::panic::resume_unwind(stop);
    }
    let (mut record, written) = assemble(log, &calls, &leaves.unwrap_or_default(), &reads);
    let callers = callers_structures(&calls, (low, top), &written, &record.inputs);
    record.inputs.extend(callers);
    record.replayable = !rt::device_record_unseen();
    if audit {
        note_uncovered(e, key, &audited, &record, &written, &calls);
    }
    record
}

/// The structures the build's device calls passed from the callers' stack locals, above the set-up's own
/// (`[low, top)`): the recorder keeps their bytes (it copies any structure on the stack for locals) and a replay
/// passes those, so they are inputs too, read by [`DEVICE_READER`] and the call's slot: a caller's local that
/// changes breaks the record instead of going to the device as it was (review of #1108). Bytes the set-up wrote
/// itself, and addresses already inputs, are left out.
fn callers_structures(
    calls: &[DeviceCall],
    (low, top): (u32, u32),
    written: &std::collections::HashSet<u32>,
    inputs: &[(u32, Vec<u8>, u32)],
) -> Vec<(u32, Vec<u8>, u32)> {
    let mut out: Vec<(u32, Vec<u8>, u32)> = Vec::new();
    for call in calls {
        let Some(at) = call.data_arg.and_then(|i| call.args.get(i).copied()) else {
            continue;
        };
        let len = n32(call.data.len());
        let own_locals = at >= low && u64::from(at) + u64::from(len) <= u64::from(top);
        let own_bytes = (0..len).all(|i| written.contains(&at.wrapping_add(i)));
        let known = inputs.iter().chain(&out).any(|(a, _, _)| *a == at);
        if len != 0 && !own_locals && !own_bytes && !known {
            out.push((at, call.data.clone(), DEVICE_READER | call.slot));
        }
    }
    out
}

/// The reader of an input a device call's structure is ([`callers_structures`]), with the call's slot below.
const DEVICE_READER: u32 = 0xd3d9_0000;

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
        // A write that starts inside or right after the write just before it (nothing between them: a matrix or a
        // row stored word by word) joins it, made in the same order: one write on replay.
        if let Some(Op::Write(last_at, last)) = ops.last_mut() {
            let start = u64::from(*last_at);
            let end = start + last.len() as u64;
            let from = u64::from(at);
            if (start..=end).contains(&from) {
                let off = (from - start) as usize;
                let over = (last.len() - off).min(bytes.len());
                last[off..off + over].copy_from_slice(&bytes[..over]);
                last.extend_from_slice(&bytes[over..]);
                continue;
            }
        }
        ops.push(Op::Write(at, bytes));
    }
    let mut counts = [0u32; 3];
    for op in &ops {
        counts[match op {
            Op::Write(..) => 0,
            Op::Leaf(_) => 1,
            Op::Device(_) => 2,
        }] += 1;
    }
    (
        Record {
            statics: Vec::new(),
            spans: Spans::default(),
            inputs,
            ops: ops.into(),
            counts,
            replayable: true,
            last_used: 0,
            item: None,
            streak: 0,
            rest: 0,
        },
        written,
    )
}

/// The bytes now under each write of `ops`, in their order (check mode puts them back after the replay that makes
/// the item).
fn bytes_under_writes(e: &Engine, ops: &[Op]) -> Vec<(u32, Vec<u8>)> {
    ops.iter()
        .filter_map(|op| match op {
            Op::Write(at, bytes) => {
                let mut now = vec![0u8; bytes.len()];
                e.mem.try_read(*at, &mut now).ok().map(|()| (*at, now))
            }
            _ => None,
        })
        .collect()
}

/// Replays a record's ops: its writes, setter calls and device calls, in order. The ops are the record's own
/// (shared), so a record built again meanwhile (a kept call cannot do that, but nothing here relies on it) leaves
/// this replay as it began.
fn replay(e: &mut Engine, ops: &[Op]) {
    for op in ops {
        match op {
            Op::Write(at, bytes) => e.mem.write(*at, bytes),
            Op::Leaf(leaf) => replay_leaf(e, *leaf),
            Op::Device(call) => {
                rt::device_replay(e, call);
            }
        }
    }
}

/// The hasher of the records' keys (five words: geometry, pass type, shader, property state, order): each word
/// multiplied in, a fraction of SipHash's cost (a look-up per world draw).
#[derive(Default, Clone, Copy)]
struct KeyHasher(u64);

impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0 ^ (self.0 >> 29)
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u32(u32::from(b));
        }
    }
    fn write_u32(&mut self, n: u32) {
        self.0 = (self.0 ^ u64::from(n))
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .rotate_left(26);
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
        Leaf::Entry19 => typed_call!(
            SHADER,
            0x00b7_9730,
            crate::shader::geometry::pixel_entry_19(e)
        ),
        Leaf::ObjToCube {
            this,
            block,
            pass_type,
            flag,
            transform,
        } => typed_call!(
            SHADER,
            0x00b7_b730,
            crate::shader::geometry::obj_to_cube(
                e,
                Ptr::new(this),
                Ptr::new(block),
                pass_type,
                flag,
                transform
            )
        ),
        Leaf::ShadowProj {
            this,
            block,
            flag,
            transform,
        } => typed_call!(
            SHADER,
            0x00b7_b930,
            crate::shader::geometry::shadow_proj(
                e,
                Ptr::new(this),
                Ptr::new(block),
                flag,
                Ptr::new(transform)
            )
        ),
        Leaf::Entry17 { this, object } => typed_call!(
            SHADER,
            0x00b7_9180,
            crate::shader::geometry::vertex_entry_17(e, Ptr::new(this), Ptr::new(object))
        ),
        Leaf::ClipPlanes { this, pass_type } => typed_call!(
            SHADER,
            0x00b7_be00,
            crate::shader::geometry::simple_shadow_clip_planes(e, Ptr::new(this), pass_type)
        ),
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
        // A constant upload by its first register.
        Op::Device(call) if matches!(call.slot, 0x178 | 0x1b4) => format!(
            "device slot {:#x} register {:#x}",
            call.slot,
            call.args.first().copied().unwrap_or(0)
        ),
        Op::Device(call) => format!("device slot {:#x}", call.slot),
        Op::Leaf(Leaf::RenderState { state, .. }) => format!("call RenderState {state:#x}"),
        Op::Leaf(Leaf::StageState { stage, state, .. }) => {
            format!("call StageState {stage} {state:#x}")
        }
        Op::Leaf(Leaf::SamplerState { stage, state, .. }) => {
            format!("call SamplerState {stage} {state:#x}")
        }
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

/// The counters, every [`REPORT_EVERY`] draws seen. The inputs that broke records are counted per line: the table
/// starts empty again after each, so it stays small however long the run (review of #1108).
fn report_now(k: &mut Keep) {
    if k.stats.seen % REPORT_EVERY != 0 {
        return;
    }
    eprintln!("{}", summary(&k.stats, k.records.len()));
    k.stats.broke_at.clear();
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
    // Words every audited build read are the exe's constants and the like (they never change): the words only
    // some builds read come first, the likelier culprits.
    let mut uncovered: Vec<_> = s.uncovered.iter().collect();
    uncovered.sort_by_key(|(at, n)| (**n >= s.audited, std::cmp::Reverse(**n), **at));
    let audit = if s.audited == 0 {
        String::new()
    } else {
        let every = uncovered.iter().filter(|(_, n)| **n >= s.audited).count();
        let words = uncovered
            .iter()
            .take(16)
            .map(|(at, n)| format!("{at:08x}: {n}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "; {} builds audited, reads none of their records covers (by word, builds; {every} words read by every one last): [{words}]",
            s.audited
        )
    };
    let mut differed: Vec<_> = s.differed_at.iter().collect();
    differed.sort_by_key(|(at, n)| (std::cmp::Reverse(**n), (*at).clone()));
    let differed = differed
        .iter()
        .take(10)
        .map(|(at, n)| format!("{at}: {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    let built = s.built_new + s.built_statics + s.built_inputs + s.built_unkeepable;
    let each = |ns: u64, n: u64| ns as f64 / 1000.0 / n.max(1) as f64;
    let per = |n: u64| n as f64 / s.replayed.max(1) as f64;
    format!(
        "NV_DRAW_ITEMS: {} draws: {} replayed, {} built (new {}, own bytes changed {}, inputs changed {}, device reached outside the record {}), {} plain while resting ({} rests); {} checked, {} differed{}; {records} records; inputs that broke records most since the last line: [{top}]; differed first at: [{differed}]{audit}; items: {} drawn, {} plain, {} not the record's; backend records: {} sent, {} plain; check items: {} beside their old form, {} dropped; records dropped for a new device: {}; microseconds each: build {:.2}, replay {:.2} (look-up and compares {:.2}; {:.1} writes, {:.1} kept calls, {:.1} device calls a replay), plain {:.2} ({} replayable draws drawn plainly), plain while resting {:.2}",
        s.seen,
        s.replayed,
        built,
        s.built_new,
        s.built_statics,
        s.built_inputs,
        s.built_unkeepable,
        s.rested,
        s.rests,
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
        each(s.build_ns, built + s.checked),
        each(s.replay_ns, s.replayed),
        each(s.look_ns, s.replayed),
        per(s.replay_ops[0]),
        per(s.replay_ops[1]),
        per(s.replay_ops[2]),
        each(s.sample_ns, s.sampled),
        s.sampled,
        each(s.rest_ns, s.rested),
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

    /// A key whose geometry and properties lie at unmapped addresses (its own bytes a constant fill).
    const KEY: Key = (0x10, 1, 0x20, 0x30, 0);
    const OTHER_KEY: Key = (0x14, 1, 0x20, 0x30, 0);

    /// `f` drawn as the world pass draws ([`world_draw`]): only there are draws kept.
    fn world<R>(f: impl FnOnce() -> R) -> R {
        world_draw(f)
    }

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
            *r.ops,
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
        world(|| draw(&mut e, KEY, body));
        assert_eq!((runs.get(), e.mem.u32(out)), (1, 6));
        e.mem.set_u32(out, 0);
        e.mem.set_u32(scratch, 9);
        world(|| draw(&mut e, KEY, body));
        assert_eq!(runs.get(), 1, "replayed, not run: {}", summary_now());
        assert_eq!(
            (e.mem.u32(out), e.mem.u32(scratch)),
            (6, 1),
            "the writes replayed"
        );
        e.mem.set_u32(input, 7);
        world(|| draw(&mut e, KEY, body));
        assert_eq!(
            (runs.get(), e.mem.u32(out)),
            (2, 8),
            "an input changed: built again"
        );
        // Check: the body runs on a hit; a body that writes something else now is counted.
        set_mode_for_tests(Some(Mode::Check));
        extra.set(1);
        world(|| draw(&mut e, KEY, body));
        assert_eq!((runs.get(), e.mem.u32(out)), (3, 9));
        let s = summary_now();
        assert!(s.contains("1 replayed"), "{s}");
        assert!(s.contains("1 checked, 1 differed"), "{s}");
        assert!(s.contains("inputs changed 1"), "{s}");
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// The set-up's own stack locals are no inputs (review of #1108, the high finding): a body that makes a value
    /// in a local and reads it back through the accessors (as `SetupGeometryConstants` does with the inverse world
    /// matrix) is replayed although another key's set-up has since left other bytes in the same local.
    #[test]
    fn a_set_ups_own_stack_locals_are_no_inputs() {
        let mut e = Engine::new();
        let block = e.mem.alloc(0x100);
        let (input_a, input_b, out) = (block, block + 4, block + 0x10);
        e.mem.set_u32(input_a, 5);
        e.mem.set_u32(input_b, 9);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let local_at = Cell::new(0u32);
        let body = |input: u32| {
            let (runs, local_at) = (&runs, &local_at);
            move |e: &mut Engine| {
                runs.set(runs.get() + 1);
                let value = capture::word(e, input, 0);
                e.with_stack(0x40, |e, local| {
                    local_at.set(local.addr());
                    e.mem.set_u32(local.addr(), value * 3);
                    let back: u32 = capture::own_at(e, local.addr());
                    e.mem.set_u32(out, back);
                });
            }
        };
        world(|| draw(&mut e, KEY, body(input_a)));
        assert!(
            e.in_locals(local_at.get()),
            "the body's local is on the stack for locals"
        );
        world(|| draw(&mut e, OTHER_KEY, body(input_b)));
        assert_eq!(e.mem.u32(local_at.get()), 27, "the other key's value");
        assert_eq!((runs.get(), e.mem.u32(out)), (2, 27));
        world(|| draw(&mut e, KEY, body(input_a)));
        assert_eq!(runs.get(), 2, "replayed: {}", summary_now());
        assert_eq!(e.mem.u32(out), 15, "the record's write");
        let s = summary_now();
        assert!(s.contains("1 replayed, 2 built (new 2,"), "{s}");
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// Only the world pass's draws are kept (review of #1108): outside [`world_draw`], and inside another scene's
    /// draw within it ([`outside_world_draw`]), every draw runs as always and nothing is noted.
    #[test]
    fn only_the_world_draw_is_kept() {
        let mut e = Engine::new();
        let block = e.mem.alloc(0x10);
        e.mem.set_u32(block, 5);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let body = |e: &mut Engine| {
            runs.set(runs.get() + 1);
            let value = capture::word(e, block, 0);
            e.mem.set_u32(block + 4, value);
        };
        draw(&mut e, KEY, body);
        draw(&mut e, KEY, body);
        assert_eq!(runs.get(), 2);
        assert!(summary_now().starts_with("NV_DRAW_ITEMS: 0 draws"));
        world(|| {
            draw(&mut e, KEY, body);
            outside_world_draw(|| draw(&mut e, KEY, body));
            draw(&mut e, KEY, body);
        });
        assert_eq!(runs.get(), 4, "built, plain in the other scene, replayed");
        draw(&mut e, KEY, body);
        assert_eq!(runs.get(), 5, "after the world draw: plain");
        let s = summary_now();
        assert!(
            s.starts_with("NV_DRAW_ITEMS: 2 draws: 1 replayed, 1 built"),
            "{s}"
        );
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// A key whose record keeps breaking rests (review of #1108): after [`BACK_OFF_AFTER`] breaks in a row its next
    /// [`REST_DRAWS`] draws run plainly, then it is compared and built again, resting twice as long; a replay ends
    /// a streak.
    #[test]
    fn a_key_that_keeps_breaking_rests() {
        assert_eq!(
            [0, 1, 3, 4, 5, 6, 9, 30, 99].map(rest_after),
            [0, 0, 0, 64, 128, 256, 2048, 4096, 4096]
        );
        let mut e = Engine::new();
        let block = e.mem.alloc(0x10);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let body = |e: &mut Engine| {
            runs.set(runs.get() + 1);
            let value = capture::word(e, block, 0);
            e.mem.set_u32(block + 4, value);
        };
        let moved = |e: &mut Engine| {
            e.mem.set_u32(block, e.mem.u32(block) + 1);
            world(|| draw(e, KEY, body));
        };
        // New, then four breaks: the fourth starts a rest.
        for _ in 0..5 {
            moved(&mut e);
        }
        let s = summary_now();
        assert!(
            s.contains("5 built (new 1, own bytes changed 0, inputs changed 4"),
            "{s}"
        );
        for _ in 0..REST_DRAWS {
            moved(&mut e);
        }
        assert_eq!(runs.get(), 5 + REST_DRAWS, "every draw ran its body");
        let s = summary_now();
        assert!(s.contains(" 5 built "), "{s}");
        assert!(s.contains("64 plain while resting (1 rests)"), "{s}");
        moved(&mut e);
        let s = summary_now();
        assert!(s.contains(" 6 built ") && s.contains("(2 rests)"), "{s}");
        assert_eq!(KEEP.with(|k| k.borrow().records[&KEY].rest), 2 * REST_DRAWS);
        // A replay ends the streak: four breaks would rest, but three, a replay and a break do not.
        let steady = |e: &mut Engine, step: u32| {
            e.mem.set_u32(block + 8, e.mem.u32(block + 8) + step);
            world(|| {
                draw(e, OTHER_KEY, |e: &mut Engine| {
                    let value = capture::word(e, block, 8);
                    e.mem.set_u32(block + 12, value);
                });
            });
        };
        for step in [1, 1, 1, 1, 0, 1] {
            steady(&mut e, step);
        }
        assert_eq!(
            KEEP.with(|k| {
                let k = k.borrow();
                (k.records[&OTHER_KEY].streak, k.records[&OTHER_KEY].rest)
            }),
            (1, 0)
        );
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// What a record keeps of its geometry and properties (review of #1108): the ranges, each pinned, the property's
    /// six texture arrays by its texture count (at most [`MOST_TEXTURES`] each) and its texture effect's texture; a
    /// texture replaced in one of the arrays breaks the record.
    #[test]
    fn a_records_own_bytes_cover_the_propertys_textures() {
        let mut e = Engine::new();
        let geometry = e.mem.alloc(0x100);
        let state = e.mem.alloc(0x20);
        let [first, property, alpha, shader, data, render, arrays, effect] =
            [0x40, 0x100, 0x40, 0x80, 0x40, 0x60, 0x180, 0x80].map(|n| e.mem.alloc(n));
        e.mem.set_u32(state, first);
        e.mem.set_u32(state + 0xc, property);
        e.mem.set_u32(state + 0x10, alpha);
        e.mem.set_u32(geometry + 0xb8, data);
        e.mem.set_u32(data + 0x34, render);
        e.mem.set_u16(property + 0xa8, 3);
        for i in 0..6 {
            e.mem.set_u32(property + 0xac + 4 * i, arrays + 0x40 * i);
        }
        e.mem.set_u32(property + 0xdc, effect);
        let key = (geometry, 0x60, shader, state, 0);
        let array = |i: u32, len: u32| (arrays + 0x40 * i, len);
        let mut want = (0..6).map(|i| array(i, 12)).collect::<Vec<_>>();
        want.push((effect + 0x6c, 4));
        want.extend([
            (geometry, 0xc0),
            (state, 0x14),
            (first, 0x20),
            (property, 0x24),
            (property + 0x38, 0xc8),
            (alpha, 0x20),
            (shader, 0x80),
            (data, 0x40),
            (render, 0x50),
        ]);
        assert_eq!(statics_ranges(&e, key).to_vec(), want);
        e.mem.set_u16(property + 0xa8, 100);
        e.mem.set_u32(property + 0xdc, 0);
        assert_eq!(statics_ranges(&e, key)[2], array(2, 4 * MOST_TEXTURES));
        assert_eq!(statics_ranges(&e, key)[6], (0, 4));
        e.mem.set_u16(property + 0xa8, 3);
        // A retexture in place breaks the record.
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let body = |e: &mut Engine| {
            runs.set(runs.get() + 1);
            e.mem.set_u32(render + 0x58, 1);
        };
        world(|| draw(&mut e, key, body));
        world(|| draw(&mut e, key, body));
        assert_eq!(runs.get(), 1, "replayed");
        e.mem.set_u32(arrays + 0x80 + 8, 0x1234);
        world(|| draw(&mut e, key, body));
        assert_eq!(runs.get(), 2, "built again");
        assert!(summary_now().contains("own bytes changed 1"));
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// Record numbers are one counter for the run (review of #1108): two threads never get the same one.
    #[test]
    fn record_numbers_are_one_counter_for_the_run() {
        let other = std::thread::spawn(|| (0..500).map(|_| next_id()).collect::<Vec<_>>());
        let mut all: Vec<u32> = (0..500).map(|_| next_id()).collect();
        all.extend(other.join().expect("the other thread"));
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), n);
        assert!(!all.contains(&0));
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
    /// tests see their own empty log and plain answers, as without hooks. Its interrupt reaches the device's state
    /// as the platform's flush does.
    static TEST_HOOKS: draw_items::Hooks = draw_items::Hooks {
        begin: |_, ask| ASKS.with(|a| a.borrow_mut().push(ask)),
        end: |_| match ASKS.with(|a| a.borrow().last().copied()) {
            Some(Ask::Record { .. }) => {
                RECORDED.with(Cell::get).map_or(Done::Plain, Done::Recorded)
            }
            Some(Ask::Item { .. }) => Done::Drawn,
            None => Done::Plain,
        },
        interrupt: |_| rt::note_device_state(),
        forget: |_, ids| DROPPED.with(|d| d.borrow_mut().extend_from_slice(ids)),
        epoch: || EPOCH.with(Cell::get),
    };

    fn asks() -> Vec<Ask> {
        ASKS.with(|a| std::mem::take(&mut *a.borrow_mut()))
    }

    /// The one record asked in `asks`, dropping `forget` and of a check run or not: its new number.
    fn recorded(asks: &[Ask], forget: u32, check: bool) -> u32 {
        match *asks {
            [Ask::Record {
                id,
                forget: f,
                check: c,
            }] if f == forget && c == check && id != forget && id != 0 => id,
            _ => panic!("not one record dropping {forget} (check {check}): {asks:?}"),
        }
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
        let item = |id, check| Ask::Item {
            id,
            hash: 77,
            check,
        };
        world(|| draw(&mut e, KEY, body));
        let first = recorded(&asks(), 0, false);
        world(|| draw(&mut e, KEY, body));
        assert_eq!((asks(), runs.get()), (vec![item(first, false)], 1));
        e.mem.set_u32(input, 6);
        world(|| draw(&mut e, KEY, body));
        let second = recorded(&asks(), first, false);
        assert_eq!(runs.get(), 2);
        set_mode_for_tests(Some(Mode::Check));
        world(|| draw(&mut e, KEY, body));
        let both = asks();
        assert_eq!(both.first(), Some(&item(second, true)), "{both:?}");
        let third = recorded(&both[1..], second, true);
        assert_eq!((runs.get(), e.mem.u32(out)), (3, 7), "the old form drew");
        // A record that could not be sent: its replays plain, its number dropped later.
        set_mode_for_tests(Some(Mode::On));
        RECORDED.with(|r| r.set(None));
        e.mem.set_u32(input, 8);
        world(|| draw(&mut e, KEY, body));
        recorded(&asks(), third, false);
        world(|| draw(&mut e, KEY, body));
        assert_eq!(asks(), [], "replayed with its own commands");
        assert_eq!(runs.get(), 4);
        assert_eq!(KEEP.with(|k| k.borrow().forgotten.clone()), [third]);
        // A new device: every record built again.
        RECORDED.with(|r| r.set(Some(77)));
        EPOCH.with(|n| n.set(1));
        world(|| draw(&mut e, KEY, body));
        recorded(&asks(), 0, false);
        assert_eq!(runs.get(), 5);
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

    /// A set-up that reached the device outside the calls the recorder saw is never replayed; a held run's flush
    /// at a setter call (the hooks' interrupt, which reaches the device's state) is no such reach (review of #1108).
    #[test]
    fn only_a_device_reach_the_recorder_missed_stops_a_record() {
        draw_items::install(&TEST_HOOKS);
        let mut e = Engine::new();
        let block = e.mem.alloc(0x10);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let body = |unseen: bool| {
            let runs = &runs;
            move |e: &mut Engine| {
                runs.set(runs.get() + 1);
                let value = capture::word(e, block, 0);
                draw_items::interrupt(e);
                if unseen {
                    rt::note_device_state();
                }
                e.mem.set_u32(block + 4, value);
            }
        };
        world(|| draw(&mut e, KEY, body(false)));
        world(|| draw(&mut e, KEY, body(false)));
        assert_eq!(runs.get(), 1, "replayed: {}", summary_now());
        world(|| draw(&mut e, OTHER_KEY, body(true)));
        world(|| draw(&mut e, OTHER_KEY, body(true)));
        assert_eq!(runs.get(), 3);
        assert!(summary_now().contains("device reached outside the record 1"));
        let _ = asks();
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// Backend records no record holds any more are sent to be dropped in batches of [`FORGET_BATCH`].
    #[test]
    fn forgotten_records_go_to_the_backend_in_batches() {
        draw_items::install(&TEST_HOOKS);
        let mut e = Engine::new();
        reset_for_tests();
        DROPPED.with(|d| d.borrow_mut().clear());
        let batch = n32(FORGET_BATCH);
        KEEP.with(|k| k.borrow_mut().forgotten = (1..batch).collect());
        after_draw(&mut e);
        assert!(DROPPED.with(|d| d.borrow().is_empty()));
        KEEP.with(|k| k.borrow_mut().forgotten.push(batch));
        after_draw(&mut e);
        assert_eq!(
            DROPPED.with(|d| std::mem::take(&mut *d.borrow_mut())),
            (1..=batch).collect::<Vec<_>>()
        );
        assert!(KEEP.with(|k| k.borrow().forgotten.is_empty()));
        reset_for_tests();
    }

    /// Above the most records, those unused for long go with their backend records; if still above, all go.
    #[test]
    fn records_unused_for_long_are_dropped_with_their_backend_records() {
        let record = |last_used: u64, id: u32| Record {
            statics: Vec::new(),
            spans: Spans::default(),
            inputs: Vec::new(),
            ops: Rc::from([]),
            counts: [0; 3],
            replayable: true,
            last_used,
            item: Some((id, 0)),
            streak: 0,
            rest: 0,
        };
        let mut k = Keep::default();
        k.stats.seen = 100;
        for (i, used) in [(1u32, 10u64), (2, 95), (3, 90)] {
            k.records.insert((i, 0, 0, 0, 0), record(used, 10 + i));
        }
        evict_from(&mut k, 3, 10);
        assert_eq!((k.records.len(), k.forgotten.len()), (3, 0));
        evict_from(&mut k, 2, 10);
        assert_eq!(k.forgotten, [11], "unused since before draw 90");
        assert_eq!(k.records.len(), 2);
        evict_from(&mut k, 1, 10);
        assert_eq!(
            (k.records.len(), &k.forgotten),
            (1, &vec![11, 13]),
            "still above: the least recently used went, the latest stays"
        );
        assert!(k.records.contains_key(&(2, 0, 0, 0, 0)));
    }

    /// Above the most records with every one used lately, only the least recently used go, down to three quarters of
    /// the most, not all at once (review of #1108).
    #[test]
    fn above_the_most_records_the_least_recently_used_go() {
        let mut k = Keep::default();
        k.stats.seen = 1000;
        for i in 0..12u32 {
            let r = Record {
                statics: Vec::new(),
                spans: Spans::default(),
                inputs: Vec::new(),
                ops: Rc::from([]),
                counts: [0; 3],
                replayable: true,
                last_used: 990 + u64::from(i),
                item: Some((100 + i, 0)),
                streak: 0,
                rest: 0,
            };
            k.records.insert((i, 0, 0, 0, 0), r);
        }
        evict_from(&mut k, 8, 100);
        assert_eq!(k.records.len(), 6, "three quarters of 8");
        k.forgotten.sort_unstable();
        assert_eq!(k.forgotten, (100..106).collect::<Vec<_>>());
        assert!((6..12u32).all(|i| k.records.contains_key(&(i, 0, 0, 0, 0))));
    }

    /// A stop or a C++ exception (a panic) unwinding through a build or a replay ends what they began and goes on
    /// (review of #1108): no build flag, store log or accessor notes are left on, the held draw ends, a setter's depth
    /// starts again at the next build (whose setter calls are kept), and a record the unwind went through keeps its
    /// ops.
    #[test]
    fn an_unwind_through_a_build_or_a_replay_leaves_nothing_on() {
        draw_items::install(&TEST_HOOKS);
        let mut e = Engine::new();
        let block = e.mem.alloc(0x10);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let _ = asks();
        let leaf = Leaf::Defined([0; 12]);
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            world(|| {
                draw(&mut e, KEY, |e| {
                    let _ = capture::word(e, block, 0);
                    super::leaf(e, leaf, |_| panic!("a C++ exception"));
                })
            })
        }));
        assert!(stopped.is_err(), "the unwind goes on");
        assert!(!building());
        assert!(!e.mem.store_log_on());
        assert!(!capture::keep_reading_for_tests());
        assert_eq!(asks().len(), 1, "one held draw, ended");
        let runs = Cell::new(0u32);
        world(|| {
            draw(&mut e, KEY, |e| {
                runs.set(runs.get() + 1);
                let _ = capture::word(e, block, 0);
                super::leaf(e, leaf, |_| ());
                e.mem.set_u32(block + 4, 3);
            })
        });
        assert_eq!(runs.get(), 1);
        let ops = KEEP.with(|k| k.borrow().records[&KEY].ops.clone());
        assert_eq!(
            *ops,
            [Op::Leaf(leaf), Op::Write(block + 4, vec![3, 0, 0, 0])],
            "the setter call kept: its depth started again"
        );
        let _ = asks();
        // A replay that stops half way (a write to unmapped memory) leaves the record's ops for the next one (the
        // replay runs the record's own shared ops), and its holding ends.
        let broken: Rc<[Op]> = Rc::from([
            Op::Write(block + 8, vec![1]),
            Op::Write(0x0000_5000, vec![2]),
        ]);
        KEEP.with(|k| k.borrow_mut().records.get_mut(&KEY).unwrap().ops = Rc::clone(&broken));
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            world(|| draw(&mut e, KEY, |_| ()));
        }));
        assert!(stopped.is_err());
        assert_eq!(KEEP.with(|k| k.borrow().records[&KEY].ops.clone()), broken);
        assert!(
            matches!(asks()[..], [Ask::Item { check: false, .. }]),
            "one held item, ended"
        );
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// With the block check on (`NV_BLOCK_CHECK`: the set-up's locals are heap blocks) nothing is kept: every draw
    /// runs its set-up (review of #1108).
    #[test]
    fn nothing_is_kept_with_the_block_check_on() {
        let mut e = Engine::new();
        let block = e.mem.alloc(0x10);
        e.mem.enable_block_check(false, false);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let body = |e: &mut Engine| {
            runs.set(runs.get() + 1);
            let _ = capture::word(e, block, 0);
        };
        world(|| draw(&mut e, KEY, body));
        world(|| draw(&mut e, KEY, body));
        assert_eq!(runs.get(), 2);
        assert!(KEEP.with(|k| k.borrow().records.is_empty()));
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// A structure a device call passed from a caller's stack local (above the set-up's own) is an input with its
    /// bytes; one in the set-up's own locals, one it wrote itself, one away from the stack (no bytes kept) and an
    /// address already an input are not (review of #1108).
    #[test]
    fn a_callers_structure_is_an_input() {
        let call = |slot: u32, at: u32, data: Vec<u8>| DeviceCall {
            this: 0x100,
            slot,
            args: vec![0, at, 1],
            data,
            data_arg: Some(1),
        };
        let calls = [
            call(0x178, 0x9000, vec![1; 16]),
            call(0x1b4, 0x8100, vec![2; 16]),
            call(0x178, 0x9100, vec![3; 16]),
            call(0x178, 0x9200, Vec::new()),
            call(0x178, 0x9300, vec![4; 16]),
            call(0x1b4, 0x9000, vec![1; 16]),
        ];
        let written: std::collections::HashSet<u32> = (0x9100..0x9110).collect();
        let inputs = [(0x9300, vec![4; 16], 0x11)];
        let got = callers_structures(&calls, (0x8000, 0x8800), &written, &inputs);
        assert_eq!(got, [(0x9000, vec![1; 16], DEVICE_READER | 0x178)]);
    }

    /// Every accessor notes what it reads while a record is built, but not in the set-up's own locals; no
    /// accessor takes a build for an open packet (review of #1108).
    #[test]
    fn the_accessors_note_their_reads_for_a_build() {
        let mut e = Engine::new();
        let block = e.mem.alloc(0x40);
        capture::keep_reads_begin(block + 0x30, block + 0x40);
        capture::check_packets_open_for_tests();
        // A build alone is no open packet: `active()` answers false at some moment while it lasts (other tests'
        // packets come and go on their own threads; with the build counted as a packet it never would).
        let alone = (0..100_000).any(|_| {
            let no = !capture::active();
            std::thread::yield_now();
            no
        });
        assert!(alone, "a build counted as an open packet");
        let _ = capture::word(&e, block, 0);
        let _ = capture::span(&e, block, 8, 0x10);
        capture::image(&mut e, block, 0x10, 0x18, |_, _| ());
        let _: u32 = capture::own_at(&e, block + 0x20);
        let _: u32 = capture::own_at(&e, block + 0x30);
        let reads = capture::keep_reads_end();
        assert_eq!(
            reads
                .iter()
                .map(|(at, bytes, _)| (*at - block, bytes.len()))
                .collect::<Vec<_>>(),
            [(0, 4), (8, 8), (0x10, 8), (0x20, 4)]
        );
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

    /// A replay's test of the record's own bytes where they lie agrees with the copy a build makes: the same bytes
    /// are the same, any changed byte (in the geometry, a texture array, the render data) is not, an unreadable range
    /// is its fill, and a range of another length (a texture count changed) is not the same.
    #[test]
    fn the_own_bytes_compared_in_place_are_the_copys() {
        let mut e = Engine::new();
        let geometry = e.mem.alloc(0x100);
        let state = e.mem.alloc(0x20);
        let [first, property, shader, data, render, arrays] =
            [0x40, 0x100, 0x80, 0x40, 0x60, 0x180].map(|n| e.mem.alloc(n));
        for (i, at) in (0u32..).zip([geometry, first, property, shader, data, render]) {
            for w in 0..8u32 {
                e.mem.set_u32(at + 4 * w, 0x100 * i + w);
            }
        }
        e.mem.set_u32(state, first);
        e.mem.set_u32(state + 0xc, property);
        // The alpha property (+0x10) and the texture effect (+0xdc) unmapped: their ranges are the fill.
        e.mem.set_u32(state + 0x10, 0x0000_7000);
        e.mem.set_u32(geometry + 0xb8, data);
        e.mem.set_u32(data + 0x34, render);
        e.mem.set_u16(property + 0xa8, 3);
        for i in 0..6 {
            e.mem.set_u32(property + 0xac + 4 * i, arrays + 0x40 * i);
        }
        let key = (geometry, 0x60, shader, state, 0);
        let kept = statics(&e, key);
        assert!(statics_same(&e, key, &kept));
        assert!(!statics_same(&e, key, &kept[1..]), "another length");
        for at in [geometry + 4, arrays + 0x44, render + 0x4c, shader + 0x7c] {
            let was = e.mem.u8(at);
            e.mem.set_u8(at, was ^ 1);
            assert!(!statics_same(&e, key, &kept), "{at:x} changed");
            assert_ne!(statics(&e, key), kept);
            e.mem.set_u8(at, was);
            assert!(statics_same(&e, key, &kept));
        }
        // A texture count changed: every array's range has another length.
        e.mem.set_u16(property + 0xa8, 4);
        assert!(!statics_same(&e, key, &kept));
        assert!(statics_same(&e, key, &statics(&e, key)));
    }

    /// The set-ups that read the camera where no input notes it (the object-to-cube matrix, the shadow projection,
    /// the vertex entry 17 and the simple shadow clip planes) are kept as calls, made again on replay with the
    /// frame's camera: the live runs' first difference was the object-to-cube matrix of an older camera (vertex
    /// register 10).
    #[test]
    fn the_camera_set_ups_are_kept_as_calls() {
        use engine_core::ptr::Ptr;
        let mut e = Engine::new();
        // The shader statics' page: the 22 vertex constant entries' pointers, each at an entry switched off (each
        // set-up's own work returns early).
        e.mem.map(0x011f_0000, 0x1_0000);
        let entry = e.mem.alloc(0x10);
        for i in 0..22 {
            e.mem
                .set_u32(crate::shader::geometry::VERTEX_ENTRIES + 4 * i, entry);
        }
        let block = e.mem.alloc(0x40);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let body = |e: &mut Engine| {
            runs.set(runs.get() + 1);
            let _ = capture::word(e, block, 0);
            let (this, b) = (Ptr::new(0x0100), Ptr::new(block));
            crate::shader::geometry::obj_to_cube(e, this, b, 0x60, 0, block + 0x10);
            crate::shader::geometry::shadow_proj(e, this, b, 1, Ptr::new(block + 0x10));
            crate::shader::geometry::vertex_entry_17(e, this, b);
            // Not one of the clip planes' types: no call to keep.
            crate::shader::geometry::simple_shadow_clip_planes(e, this, 0x60);
        };
        world(|| draw(&mut e, KEY, body));
        let ops = KEEP.with(|k| k.borrow().records[&KEY].ops.clone());
        assert_eq!(
            *ops,
            [
                Op::Leaf(Leaf::ObjToCube {
                    this: 0x100,
                    block,
                    pass_type: 0x60,
                    flag: 0,
                    transform: block + 0x10,
                }),
                Op::Leaf(Leaf::ShadowProj {
                    this: 0x100,
                    block,
                    flag: 1,
                    transform: block + 0x10,
                }),
                Op::Leaf(Leaf::Entry17 {
                    this: 0x100,
                    object: block,
                }),
            ]
        );
        world(|| draw(&mut e, KEY, body));
        assert_eq!(runs.get(), 1, "replayed: {}", summary_now());
        let s = summary_now();
        assert!(
            s.contains("0.0 writes, 3.0 kept calls, 0.0 device calls a replay"),
            "{s}"
        );
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// In `On`, one replayable draw in [`SAMPLE_EVERY`] is drawn plainly (its set-up runs, its record stays), and the
    /// line gives the plain set-up's time beside the replay's.
    #[test]
    fn one_replayable_draw_in_so_many_is_drawn_plainly_for_the_line() {
        let mut e = Engine::new();
        let block = e.mem.alloc(0x10);
        reset_for_tests();
        set_mode_for_tests(Some(Mode::On));
        let runs = Cell::new(0u32);
        let body = |e: &mut Engine| {
            runs.set(runs.get() + 1);
            let _ = capture::word(e, block, 0);
            e.mem.set_u32(block + 4, 1);
        };
        for _ in 0..SAMPLE_EVERY {
            world(|| draw(&mut e, KEY, body));
        }
        assert_eq!(
            runs.get(),
            2,
            "built once, drawn plainly once: {}",
            summary_now()
        );
        let s = summary_now();
        assert!(s.contains(&format!("{} replayed", SAMPLE_EVERY - 2)), "{s}");
        assert!(s.contains("(1 replayable draws drawn plainly)"), "{s}");
        world(|| draw(&mut e, KEY, body));
        assert_eq!(runs.get(), 2, "the record stayed");
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// Check mode puts the replay's writes back before the set-up runs: a set-up that reads what the record wrote (as
    /// `render_standard` reads the owed restore it clears) finds what an off run finds, so it is not counted as
    /// differing.
    #[test]
    fn check_mode_undoes_the_replays_writes_before_the_set_up() {
        let mut e = Engine::new();
        let block = e.mem.alloc(0x10);
        let owed = block + 8;
        reset_for_tests();
        draw_items::install(&TEST_HOOKS);
        let _ = asks();
        set_mode_for_tests(Some(Mode::On));
        let body = |e: &mut Engine| {
            let _ = capture::word(e, block, 0);
            // The restore owed: done and cleared (an own read of what the draw's last one left).
            if e.mem.u32(owed) != 0 {
                e.mem.set_u32(block + 4, 7);
                e.mem.set_u32(owed, 0);
            }
        };
        e.mem.set_u32(owed, 1);
        world(|| draw(&mut e, KEY, body));
        e.mem.set_u32(owed, 1);
        set_mode_for_tests(Some(Mode::Check));
        world(|| draw(&mut e, KEY, body));
        let s = summary_now();
        assert!(s.contains("1 checked, 0 differed"), "{s}");
        assert_eq!((e.mem.u32(owed), e.mem.u32(block + 4)), (0, 7));
        set_mode_for_tests(None);
        reset_for_tests();
    }

    /// A replay compares the own bytes' ranges and the inputs merged by address (adjacent and overlapping ones one
    /// span, a range at 0 left out): any byte changed in any of them is a difference, and a range that could not be
    /// read at the build must stay unread (or read as the fill).
    #[test]
    fn a_replay_compares_merged_spans() {
        let mut e = Engine::new();
        let block = e.mem.alloc(0x40);
        let inputs = vec![
            (block + 4, vec![1, 2, 3, 4], 0),
            (block, vec![9, 9, 9, 9], 0),
            (block + 6, vec![3, 4, 5, 6], 0),
            (block + 0x20, vec![7], 0),
        ];
        for (at, bytes, _) in &inputs {
            e.mem.write(*at, bytes);
        }
        e.mem.set_u32(block + 0x10, 0x11);
        e.mem.set_u32(block + 0x14, 0x22);
        let ranges = [(block + 0x10, 8), (0, 4), (0x0000_7000, 4)];
        let statics = [e.mem.bytes(block + 0x10, 8), vec![0xa5; 8]].concat();
        let s = make_spans(&e, &ranges, &statics, &inputs);
        assert_eq!(s.spans, [(block, 10), (block + 0x10, 8), (block + 0x20, 1)]);
        assert_eq!(s.bytes[..10], [9, 9, 9, 9, 1, 2, 3, 4, 5, 6]);
        assert_eq!(s.unread, [(0x0000_7000, 4)]);
        assert!(spans_same(&e, &s));
        for at in [block + 9, block + 0x17, block + 0x20] {
            let was = e.mem.u8(at);
            e.mem.set_u8(at, was ^ 0x40);
            assert!(!spans_same(&e, &s), "{at:x} changed");
            e.mem.set_u8(at, was);
        }
        assert!(spans_same(&e, &s));
        e.mem.map(0x0000_7000, 0x1000);
        assert!(!spans_same(&e, &s), "an unread range readable now");
        e.mem.write(0x0000_7000, &[0xa5; 4]);
        assert!(spans_same(&e, &s), "as the fill reads");
    }

    /// Writes one after another with nothing between them join when each starts inside or right after the last (in
    /// their order: a later byte wins); one apart, or with a kept call between, they stay apart.
    #[test]
    fn writes_in_a_row_join() {
        let m = |kind: u32, i: u32| (STORE_LOG_MARK + kind, i.to_le_bytes().to_vec());
        let leaf = Leaf::Entry19;
        let log = vec![
            (0x5000, vec![1, 2]),
            (0x5002, vec![3]),
            (0x5001, vec![9, 8, 7]),
            (0x5005, vec![5]),
            m(MARK_LEAF, 0),
            (0x5006, vec![6]),
        ];
        let (r, _) = assemble(log, &[], &[leaf], &[]);
        assert_eq!(
            *r.ops,
            [
                Op::Write(0x5000, vec![1, 9, 8, 7]),
                Op::Write(0x5005, vec![5]),
                Op::Leaf(leaf),
                Op::Write(0x5006, vec![6]),
            ]
        );
        assert_eq!(r.counts, [3, 1, 0]);
    }
}
