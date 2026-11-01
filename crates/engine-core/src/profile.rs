//! `NV_PROFILE=1`: a sampling profiler of the running game (a diagnostic of the host, not exe
//! behaviour; nothing here changes what a translated function does).
//!
//! An OS thread of its own looks, about a thousand times a second, at:
//! - the **phase**: the game's code holds the engine ([`Phase::Game`]); the engine is being handed
//!   from one of the thread host's OS threads to another ([`Phase::Switch`], `thread_host`'s
//!   channel hand-offs); the game's code waits on the real clock ([`Phase::Sleep`]: a `Sleep` or a
//!   wait the host really sleeps through, `Engine::wait_clock_until_us`); or none of the game's
//!   code runs ([`Phase::Viewer`]: between two of the game's frames, the viewer's own work);
//! - the **game thread** holding the engine (1 is main);
//! - the **exe-address call stack** of the functions entered through `Engine::call`, mirrored
//!   into a lock-free shadow stack ([`push`], [`pop`], [`resync`]);
//! - on 64-bit Windows, the **instruction pointer** of the OS thread holding the engine (between
//!   the game's frames, of the thread that runs them: the viewer's), named through `dbghelp` when
//!   the table is printed (the exe's public symbols, so a time inside an inlined helper counts
//!   under the function it was inlined into).
//!
//! - on 64-bit Windows, in the game's phases, the **native call stack** of that OS thread, walked
//!   while it is suspended (`NV_PROFILE_STACK` frames, default 64, 0 off): each image's own
//!   unwind table (`.pdata`, binary-searched here: `RtlLookupFunctionEntry` can take the loader's
//!   lock, which the suspended thread may hold) and `RtlVirtualUnwind`, which takes no lock; the
//!   stack pointer and the frame register are checked against the thread's stack before each
//!   step. Nothing allocates while the thread is suspended. Each code address's lookup is kept
//!   for the next walks. 2026-11-01, Doc's room, 51 frames a walk: the thread stood suspended
//!   67 us a walk without the kept lookups, 51 us with them, and 36 us with one frame
//!   (`NV_PROFILE_STACK=1`: suspending it and reading its registers, which every sample pays).
//!
//! Every call through `Engine::call` (and `Engine::vcall`, which dispatches through the same body)
//! is also counted by its exe address ([`count_target`]: an open-addressed table hashed like the
//! call cache, written only by the thread holding the engine; `NV_PROFILE_CALLS=0` turns it off).
//!
//! The profiler on, every call takes `Engine::call_full` (the shadow stack, the count): calls cost
//! more than in a run without it, and `call_full` stands in the native tables for the fast path
//! `Engine::call` inlines into its callers in such a run.
//!
//! Every `NV_PROFILE_SECONDS` (default 10) it prints the phases, the game threads, where the game's
//! own time goes by kind (exe code, the Direct3D 9 / DirectSound / DirectInput platform objects,
//! imported functions by name), the top `NV_PROFILE_TOP` (default 25) exe functions by self and
//! inclusive samples, the top native functions in the game's and the viewer's time, the native
//! functions by inclusive samples and the callers of the 15 busiest (from the walked stacks), and
//! the top `NV_PROFILE_CALLS` (default 60) call targets with their calls a frame.
//!
//! Off (the default), each `Engine::call` pays one relaxed load of [`on`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The most native frames a walk records (`NV_PROFILE_STACK` is capped at it).
pub const MAX_FRAMES: usize = 256;

/// Why a native stack walk ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WalkEnd {
    /// The thread's first function (its return address is 0).
    Start = 0,
    /// The frame limit (`NV_PROFILE_STACK`).
    Limit = 1,
    /// Code in no image the sampler knows (an image loaded since its last look, or no image).
    Unknown = 2,
    /// A frame that would leave the thread's stack, or not move up it.
    Bad = 3,
}

impl WalkEnd {
    /// Each end with its name in the table.
    pub const ALL: [(&'static str, WalkEnd); 4] = [
        ("the thread's start", WalkEnd::Start),
        ("the frame limit", WalkEnd::Limit),
        ("unknown code", WalkEnd::Unknown),
        ("a bad frame", WalkEnd::Bad),
    ];
}

/// One native stack walk ([`native::Sampler::walk`]).
#[derive(Debug, Clone, Copy)]
pub struct Walk {
    /// The thread's instruction pointer.
    pub rip: u64,
    /// The frames recorded (`Sampler::keys`).
    pub frames: usize,
    pub end: WalkEnd,
    /// How long the thread stood suspended.
    pub suspended_ns: u64,
}

/// Whether the profiler is on (`NV_PROFILE` set to anything but `0` or empty), read once by
/// [`init_from_env`].
static ON: AtomicBool = AtomicBool::new(false);

/// How deep the shadow stack records (deeper entries are counted but not kept).
pub const MAX_DEPTH: usize = 256;

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicU32 = AtomicU32::new(0);
static STACK: [AtomicU32; MAX_DEPTH] = [ZERO; MAX_DEPTH];
static DEPTH: AtomicU32 = AtomicU32::new(0);
static PHASE: AtomicU8 = AtomicU8::new(Phase::Viewer as u8);
/// The game thread holding the engine (`thread_host::GameThread::id`).
static GAME_THREAD: AtomicU32 = AtomicU32::new(1);
/// The OS thread holding the engine (Windows thread id; 0 unknown).
static OS_THREAD: AtomicU32 = AtomicU32::new(0);
/// The game's frames ended so far (`thread_host::run_main_frame`).
static FRAMES: AtomicU64 = AtomicU64::new(0);
/// Calls through `Engine::call` so far (profiler on only; only the thread holding the engine
/// writes it, so a plain load and store).
static CALLS: AtomicU64 = AtomicU64::new(0);
/// Of those, the ones `Engine::call`'s direct-mapped cache did not answer (the same writer).
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);

/// How many slots the per-target call counts have (a power of two, twice the call cache's: a
/// run calls some tens of thousands of distinct functions).
const TARGET_SLOTS: usize = 1 << 17;
/// How far [`count_target`] probes past an address's slot before counting the call as
/// uncounted.
const TARGET_PROBES: usize = 64;
#[allow(clippy::declare_interior_mutable_const)]
const ZERO64: AtomicU64 = AtomicU64::new(0);
/// The per-target call counts: the exe address in a slot (0 empty) and its calls so far. Only
/// the thread holding the engine writes them (plain loads and stores); the sampler reads them.
static TARGET_ADDR: [AtomicU32; TARGET_SLOTS] = [ZERO; TARGET_SLOTS];
static TARGET_CALLS: [AtomicU64; TARGET_SLOTS] = [ZERO64; TARGET_SLOTS];
/// Calls [`count_target`] found no slot for (address 0, or the probes ran out).
static TARGET_UNCOUNTED: AtomicU64 = AtomicU64::new(0);
/// Whether [`push`] counts calls by target: from [`start`] on, unless `NV_PROFILE_CALLS=0` (the
/// count costs the profiled run about a millisecond a frame, inside `Engine::call_full`).
static COUNT_TARGETS: AtomicBool = AtomicBool::new(false);

/// What the engine's OS threads are doing (module documentation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    Viewer = 0,
    Game = 1,
    Switch = 2,
    Sleep = 3,
}

const PHASES: [(&str, Phase); 4] = [
    ("game", Phase::Game),
    ("viewer", Phase::Viewer),
    ("switch", Phase::Switch),
    ("sleep", Phase::Sleep),
];

impl Phase {
    fn from_u8(v: u8) -> Phase {
        match v {
            1 => Phase::Game,
            2 => Phase::Switch,
            3 => Phase::Sleep,
            _ => Phase::Viewer,
        }
    }
}

/// Whether the profiler is on.
#[inline(always)]
pub fn on() -> bool {
    ON.load(Relaxed)
}

/// Reads `NV_PROFILE` once (each `Engine::new` asks; only the first reads it).
pub fn init_from_env() {
    static READ: OnceLock<()> = OnceLock::new();
    READ.get_or_init(|| {
        let wanted = std::env::var("NV_PROFILE").is_ok_and(|v| !v.is_empty() && v != "0");
        ON.store(wanted, Relaxed);
    });
}

/// A function entered through `Engine::call` (profiler on only).
#[inline]
pub fn push(addr: u32) {
    CALLS.store(CALLS.load(Relaxed).wrapping_add(1), Relaxed);
    if COUNT_TARGETS.load(Relaxed) {
        count_target(addr);
    }
    let d = DEPTH.load(Relaxed);
    if let Some(slot) = STACK.get(d as usize) {
        slot.store(addr, Relaxed);
    }
    DEPTH.store(d.wrapping_add(1), Relaxed);
}

/// The first slot [`count_target`] tries for `addr` (Fibonacci hashing, as `Engine::call`'s
/// cache).
#[inline(always)]
const fn target_slot(addr: u32) -> usize {
    (addr.wrapping_mul(0x9e37_79b1) >> (32 - TARGET_SLOTS.trailing_zeros())) as usize
}

/// One more call to exe address `addr` (profiler on only; [`push`]): its slot's count goes up,
/// or the first empty slot from its hash on becomes its own.
#[inline]
pub fn count_target(addr: u32) {
    let mut slot = target_slot(addr);
    if addr != 0 {
        for _ in 0..TARGET_PROBES {
            let held = TARGET_ADDR[slot].load(Relaxed);
            if held == addr {
                let n = &TARGET_CALLS[slot];
                n.store(n.load(Relaxed).wrapping_add(1), Relaxed);
                return;
            }
            if held == 0 {
                TARGET_CALLS[slot].store(1, Relaxed);
                TARGET_ADDR[slot].store(addr, Relaxed);
                return;
            }
            slot = (slot + 1) & (TARGET_SLOTS - 1);
        }
    }
    TARGET_UNCOUNTED.store(TARGET_UNCOUNTED.load(Relaxed).wrapping_add(1), Relaxed);
}

/// Every call target counted so far, with its calls.
fn target_counts() -> HashMap<u32, u64> {
    TARGET_ADDR
        .iter()
        .zip(&TARGET_CALLS)
        .filter_map(|(a, n)| {
            let a = a.load(Relaxed);
            (a != 0).then(|| (a, n.load(Relaxed)))
        })
        .collect()
}

/// The calls of each target from `before` to `now` (two [`target_counts`]), targets with none
/// left out.
fn target_calls_since(now: &HashMap<u32, u64>, before: &HashMap<u32, u64>) -> HashMap<u32, u64> {
    now.iter()
        .filter_map(|(&a, &n)| {
            let d = n.wrapping_sub(before.get(&a).copied().unwrap_or(0));
            (d > 0).then_some((a, d))
        })
        .collect()
}

/// `Engine::call`'s cache did not have the callee (profiler on only).
#[inline]
pub fn cache_miss() {
    CACHE_MISSES.store(CACHE_MISSES.load(Relaxed).wrapping_add(1), Relaxed);
}

/// The innermost function returned (profiler on only).
#[inline]
pub fn pop() {
    let d = DEPTH.load(Relaxed);
    DEPTH.store(d.saturating_sub(1), Relaxed);
}

/// The shadow stack becomes `stack` (the thread host swapped call stacks, or a stop unwound).
pub fn resync(stack: &[u32]) {
    if on() {
        fill(stack);
    }
}

fn fill(stack: &[u32]) {
    for (slot, &addr) in STACK.iter().zip(stack) {
        slot.store(addr, Relaxed);
    }
    DEPTH.store(stack.len() as u32, Relaxed);
}

/// Sets the phase, returning the one before (profiler on only; `Viewer` otherwise).
#[inline]
pub fn set_phase(phase: Phase) -> Phase {
    if !on() {
        return Phase::Viewer;
    }
    Phase::from_u8(PHASE.swap(phase as u8, Relaxed))
}

/// The game thread holding the engine.
pub fn game_thread() -> u32 {
    GAME_THREAD.load(Relaxed)
}

/// The calling OS thread now holds the engine and runs game thread `game_thread`'s code.
pub fn holding(game_thread: u32) {
    if !on() {
        return;
    }
    GAME_THREAD.store(game_thread, Relaxed);
    OS_THREAD.store(native::current_thread_id(), Relaxed);
    PHASE.store(Phase::Game as u8, Relaxed);
}

/// The calling OS thread (the host's, which runs the game's frames) has the engine back between
/// two frames: the viewer's time from now.
pub fn back_to_host() {
    if !on() {
        return;
    }
    OS_THREAD.store(native::current_thread_id(), Relaxed);
    PHASE.store(Phase::Viewer as u8, Relaxed);
    FRAMES.fetch_add(1, Relaxed);
}

/// The import slot `slot` is `dll!name` (named in the table).
pub fn name_import(slot: u32, dll: &str, name: &str) {
    if !on() {
        return;
    }
    if let Ok(mut names) = import_names().lock() {
        names.insert(slot, format!("{dll}!{name}"));
    }
}

fn import_names() -> &'static Mutex<HashMap<u32, String>> {
    static NAMES: OnceLock<Mutex<HashMap<u32, String>>> = OnceLock::new();
    NAMES.get_or_init(Default::default)
}

/// Starts the sampling thread once (profiler on only).
pub fn start() {
    if !on() {
        return;
    }
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        let seconds = std::env::var("NV_PROFILE_SECONDS")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|s| *s > 0.0)
            .unwrap_or(10.0);
        let top = std::env::var("NV_PROFILE_TOP")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(25);
        let calls_top = std::env::var("NV_PROFILE_CALLS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(60);
        let depth = std::env::var("NV_PROFILE_STACK")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(64)
            .min(MAX_FRAMES);
        println!(
            "NV_PROFILE: sampling every millisecond, a table every {seconds} s, \
             native stacks to {depth} frames, the top {calls_top} call targets"
        );
        COUNT_TARGETS.store(calls_top > 0, Relaxed);
        let settings = Settings {
            period: Duration::from_secs_f64(seconds),
            top,
            calls_top,
            depth,
        };
        let _ = std::thread::Builder::new()
            .name("nv-profile".into())
            .spawn(move || sampler(settings));
    });
}

/// What the environment asks of the sampler ([`start`]).
#[derive(Clone, Copy)]
struct Settings {
    /// How often a table is printed (`NV_PROFILE_SECONDS`).
    period: Duration,
    /// Rows of the function tables (`NV_PROFILE_TOP`).
    top: usize,
    /// Rows of the call-target table (`NV_PROFILE_CALLS`).
    calls_top: usize,
    /// Native frames walked a sample (`NV_PROFILE_STACK`, 0 none).
    depth: usize,
}

/// What the platform address `addr` is, by the ranges the platform layer puts its objects' code
/// at (`units::platform`), or `None` for the exe's own code.
pub fn kind_of(addr: u32) -> Option<String> {
    let kind = match addr {
        0x7e02_0000..=0x7e03_ffff => "D3D9/D3DX platform object",
        0x7e00_0000..=0x7e00_03ff => "DirectInput platform object",
        0x7e00_0400..=0x7e00_07ff => "DirectSound platform object",
        0x7e00_0800..=0x7e00_ffff => "DirectShow platform object",
        0x7e01_0000..=0x7e01_ffff => "loader platform procedure",
        0x7e04_0000..=0x7e04_ffff => "NVAPI platform object",
        0x7e05_0000..=0x7fff_ffff => "other platform object",
        _ => {
            return import_names()
                .lock()
                .ok()
                .and_then(|n| n.get(&addr).map(|name| format!("import {name}")));
        }
    };
    Some(kind.to_string())
}

/// The samples of one table.
#[derive(Default)]
struct Table {
    samples: u64,
    phases: [u64; 4],
    threads: HashMap<u32, u64>,
    /// Game-phase samples by the innermost function's kind.
    kinds: HashMap<String, u64>,
    selfs: HashMap<u32, u64>,
    inclusive: HashMap<u32, u64>,
    /// Native instruction pointers: in the game's phases (game, switch, sleep) and the viewer's.
    native_game: HashMap<u64, u64>,
    native_viewer: HashMap<u64, u64>,
    /// Samples with no stack in the game phase (the host's own code between calls).
    no_stack: u64,
    /// Sleep-phase samples by the innermost exe calls (up to four, innermost first): who waits on
    /// the real clock.
    sleeps: HashMap<[u32; 4], u64>,
    /// `NV_PROFILE_THREADS=1`: every other thread of the process, every fourth tick: samples
    /// by thread, and by (thread, instruction pointer).
    other_samples: HashMap<u32, u64>,
    others: HashMap<(u32, u64), u64>,
    /// The native stacks walked in the game's phases: the function of each frame, innermost
    /// first (`native::Sampler::walk`), and its samples.
    stacks: HashMap<Box<[u64]>, u64>,
    /// The walks: how many, their frames in all, how each ended (by [`WalkEnd`]) and the time
    /// the walked thread stood suspended, in nanoseconds.
    walks: u64,
    walk_frames: u64,
    walk_ends: [u64; 4],
    suspended_ns: u64,
    /// The calls through `Engine::call` in this table's frames by exe address, and the calls not
    /// counted by address (filled when the table is printed).
    targets: HashMap<u32, u64>,
    targets_uncounted: u64,
    /// All calls through `Engine::call` in this table's frames.
    calls: u64,
}

fn sampler(settings: Settings) {
    let Settings {
        period,
        top,
        calls_top,
        depth: stack_depth,
    } = settings;
    let mut table = Table::default();
    let mut started = Instant::now();
    let mut frames_at = FRAMES.load(Relaxed);
    let mut calls_at = CALLS.load(Relaxed);
    let mut misses_at = CACHE_MISSES.load(Relaxed);
    let mut targets_at = target_counts();
    let mut uncounted_at = TARGET_UNCOUNTED.load(Relaxed);
    let mut stack = Vec::with_capacity(MAX_DEPTH);
    let mut seen: Vec<u32> = Vec::with_capacity(MAX_DEPTH);
    let mut sampler = native::Sampler::new();
    let all_threads = std::env::var("NV_PROFILE_THREADS").is_ok_and(|v| v == "1");
    let mut tick = 0u64;
    let mut others: Vec<u32> = Vec::new();
    let mut others_at: Option<Instant> = None;
    let mut symbols_noted = false;
    loop {
        sampler.wait_tick();
        tick += 1;
        let phase = Phase::from_u8(PHASE.load(Relaxed));
        let os_thread = OS_THREAD.load(Relaxed);
        // In the game's phases the thread's native stack is walked too (its first frame is the
        // instruction pointer); the viewer's phase reads the instruction pointer only.
        let walk = (phase != Phase::Viewer && stack_depth > 0)
            .then(|| sampler.walk(os_thread, stack_depth))
            .flatten();
        let rip = match &walk {
            Some(w) => Some(w.rip),
            None => sampler.instruction_pointer(os_thread),
        };
        if let Some(w) = walk {
            table.walks += 1;
            table.walk_frames += w.frames as u64;
            table.walk_ends[w.end as usize] += 1;
            table.suspended_ns += w.suspended_ns;
            let keys = sampler.keys(w.frames);
            match table.stacks.get_mut(keys) {
                Some(n) => *n += 1,
                None if !keys.is_empty() => {
                    table.stacks.insert(keys.into(), 1);
                }
                None => {}
            }
        }
        let game_thread = GAME_THREAD.load(Relaxed);
        let depth = DEPTH.load(Relaxed) as usize;
        stack.clear();
        stack.extend(STACK.iter().take(depth).map(|s| s.load(Relaxed)));

        table.samples += 1;
        table.phases[phase as usize] += 1;
        if all_threads && tick % 4 == 0 {
            if others_at.map_or(true, |t| t.elapsed() >= Duration::from_secs(1)) {
                others = sampler.process_threads();
                others_at = Some(Instant::now());
            }
            for &id in &others {
                if id == os_thread {
                    continue;
                }
                *table.other_samples.entry(id).or_default() += 1;
                if let Some(rip) = sampler.instruction_pointer(id) {
                    *table.others.entry((id, rip)).or_default() += 1;
                }
            }
        }
        if phase == Phase::Viewer {
            if let Some(rip) = rip {
                *table.native_viewer.entry(rip).or_default() += 1;
            }
        } else {
            *table.threads.entry(game_thread).or_default() += 1;
            if let Some(rip) = rip {
                *table.native_game.entry(rip).or_default() += 1;
            }
        }
        if phase == Phase::Sleep {
            let mut chain = [0u32; 4];
            for (slot, &a) in chain.iter_mut().zip(stack.iter().rev()) {
                *slot = a;
            }
            *table.sleeps.entry(chain).or_default() += 1;
        }
        if phase == Phase::Game {
            match stack.last() {
                Some(&inner) => {
                    let kind = kind_of(inner).unwrap_or_else(|| "exe code".into());
                    *table.kinds.entry(kind).or_default() += 1;
                    *table.selfs.entry(inner).or_default() += 1;
                    seen.clear();
                    for &a in &stack {
                        if !seen.contains(&a) {
                            seen.push(a);
                            *table.inclusive.entry(a).or_default() += 1;
                        }
                    }
                }
                None => table.no_stack += 1,
            }
        }

        let elapsed = started.elapsed();
        if elapsed >= period {
            let frames = FRAMES.load(Relaxed);
            let calls = CALLS.load(Relaxed).wrapping_sub(calls_at);
            let targets_now = target_counts();
            let uncounted_now = TARGET_UNCOUNTED.load(Relaxed);
            table.targets = target_calls_since(&targets_now, &targets_at);
            table.targets_uncounted = uncounted_now.wrapping_sub(uncounted_at);
            table.calls = calls;
            if !symbols_noted {
                symbols_noted = true;
                println!("NV_PROFILE: {}", sampler.symbol_source());
            }
            let text = report(
                &table,
                elapsed,
                frames - frames_at,
                Rows {
                    top,
                    calls: calls_top,
                },
                &mut sampler,
            );
            // The frame numbers (the host's count of main's frames) place the table in a run.
            let per_frame = calls / (frames - frames_at).max(1);
            let misses =
                CACHE_MISSES.load(Relaxed).wrapping_sub(misses_at) / (frames - frames_at).max(1);
            println!(
                "== NV_PROFILE: the game's frames {frames_at} to {frames}, \
                 {per_frame} calls through Engine::call a frame ({misses} not in its cache)\n{text}"
            );
            table = Table::default();
            started = Instant::now();
            frames_at = FRAMES.load(Relaxed);
            calls_at = CALLS.load(Relaxed);
            misses_at = CACHE_MISSES.load(Relaxed);
            targets_at = target_counts();
            uncounted_at = TARGET_UNCOUNTED.load(Relaxed);
        }
    }
}

fn pct(n: u64, of: u64) -> f64 {
    if of == 0 {
        0.0
    } else {
        100.0 * n as f64 / of as f64
    }
}

fn sorted<K: Copy + Ord>(map: &HashMap<K, u64>) -> Vec<(K, u64)> {
    let mut rows: Vec<(K, u64)> = map.iter().map(|(k, v)| (*k, *v)).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    rows
}

/// How many rows a table's lists print.
#[derive(Clone, Copy)]
struct Rows {
    /// The function lists (`NV_PROFILE_TOP`).
    top: usize,
    /// The call-target list (`NV_PROFILE_CALLS`).
    calls: usize,
}

fn report(
    t: &Table,
    elapsed: Duration,
    frames: u64,
    rows: Rows,
    sampler: &mut native::Sampler,
) -> String {
    use std::fmt::Write;
    let top = rows.top;
    let secs = elapsed.as_secs_f64();
    let n = t.samples.max(1);
    let mut out = String::new();
    let fps = frames as f64 / secs;
    let ms_per_frame = |samples: u64| {
        if frames == 0 {
            0.0
        } else {
            1000.0 * secs * (samples as f64 / n as f64) / frames as f64
        }
    };
    let _ = writeln!(
        out,
        "== NV_PROFILE: {secs:.1} s, {frames} game frames ({fps:.1} frames/s, {:.2} ms a frame), {} samples",
        if frames == 0 { 0.0 } else { 1000.0 * secs / frames as f64 },
        t.samples
    );
    let _ = write!(out, "phases:");
    for (name, p) in PHASES {
        let s = t.phases[p as usize];
        let _ = write!(
            out,
            "  {name} {:.1}% ({:.2} ms/frame)",
            pct(s, n),
            ms_per_frame(s)
        );
    }
    let _ = writeln!(out);
    let engine = n - t.phases[Phase::Viewer as usize];
    let _ = write!(out, "game threads (of the game's phases):");
    for (id, s) in sorted(&t.threads) {
        let _ = write!(out, "  {id}: {:.1}%", pct(s, engine));
    }
    let _ = writeln!(out);
    let game = t.phases[Phase::Game as usize];
    let _ = write!(out, "game phase by innermost kind:");
    for (kind, s) in sorted_strings(&t.kinds) {
        let _ = write!(out, "  {kind} {:.1}%", pct(s, game));
    }
    if t.no_stack > 0 {
        let _ = write!(out, "  (no call) {:.1}%", pct(t.no_stack, game));
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "top exe functions, self (% of all samples, ms/frame):");
    for (addr, s) in sorted(&t.selfs).into_iter().take(top) {
        let kind = kind_of(addr).map_or(String::new(), |k| format!("  {k}"));
        let _ = writeln!(
            out,
            "  {:5.1}% {:6.3}  {addr:08x}{kind}",
            pct(s, n),
            ms_per_frame(s)
        );
    }
    let _ = writeln!(
        out,
        "top exe functions, inclusive (% of all samples, ms/frame):"
    );
    for (addr, s) in sorted(&t.inclusive).into_iter().take(top) {
        let kind = kind_of(addr).map_or(String::new(), |k| format!("  {k}"));
        let _ = writeln!(
            out,
            "  {:5.1}% {:6.3}  {addr:08x}{kind}",
            pct(s, n),
            ms_per_frame(s)
        );
    }
    if !t.sleeps.is_empty() {
        let _ = writeln!(
            out,
            "sleep phase by the innermost calls (% of all samples, ms/frame):"
        );
        for (chain, s) in sorted(&t.sleeps).into_iter().take(10) {
            let calls: Vec<String> = chain
                .iter()
                .filter(|&&a| a != 0)
                .map(|a| format!("{a:08x}"))
                .collect();
            let _ = writeln!(
                out,
                "  {:5.1}% {:6.3}  {}",
                pct(s, n),
                ms_per_frame(s),
                calls.join(" < ")
            );
        }
    }
    for (title, map) in [
        ("native, the game's phases", &t.native_game),
        ("native, the viewer's phase", &t.native_viewer),
    ] {
        if map.is_empty() {
            continue;
        }
        let mut by_name: HashMap<String, u64> = HashMap::new();
        let mut by_module: HashMap<String, u64> = HashMap::new();
        for (&rip, &s) in map {
            let (module, symbol) = sampler.name(rip);
            *by_module.entry(module.clone()).or_default() += s;
            *by_name.entry(format!("{module}!{symbol}")).or_default() += s;
        }
        let _ = write!(out, "{title} by module (% of all samples):");
        for (m, s) in sorted_strings(&by_module).into_iter().take(8) {
            let _ = write!(out, "  {m} {:.1}%", pct(s, n));
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "{title}, top functions (% of all samples, ms/frame):");
        for (name, s) in sorted_strings(&by_name).into_iter().take(top) {
            let _ = writeln!(out, "  {:5.1}% {:6.3}  {name}", pct(s, n), ms_per_frame(s));
        }
    }
    if t.walks > 0 {
        native_stacks(&mut out, t, top, &ms_per_frame, &mut |k: u64| {
            let (module, symbol) = sampler.name(k);
            format!("{module}!{symbol}")
        });
    }
    if !t.targets.is_empty() {
        call_targets(&mut out, t, frames, rows.calls, &ms_per_frame);
    }
    if !t.other_samples.is_empty() {
        other_threads(&mut out, t, top, &ms_per_frame, sampler);
    }
    out
}

/// How many of the busiest native functions (by self samples) list their callers.
const CALLER_FUNCTIONS: usize = 15;
/// How many callers each of them lists.
const CALLERS: usize = 8;

/// The walked native stacks' part of a table: how the walks went, the native functions by
/// inclusive samples (a function counted once a sample however often it is on the stack), and
/// the callers of the [`CALLER_FUNCTIONS`] busiest functions by self samples (the nearest frame
/// below the innermost with another name). Functions are told apart by `name_of` their key
/// (several keys can share a name: copies of one generic function, say).
fn native_stacks(
    out: &mut String,
    t: &Table,
    top: usize,
    ms_per_frame: &dyn Fn(u64) -> f64,
    name_of: &mut dyn FnMut(u64) -> String,
) {
    use std::fmt::Write;
    let n = t.samples.max(1);
    let walks = t.walks.max(1);
    let _ = write!(
        out,
        "native stacks, the game's phases: {} walked ({:.1}% of all samples), {:.1} frames on \
         average, the thread suspended {:.1} us a walk; ended at:",
        t.walks,
        pct(t.walks, n),
        t.walk_frames as f64 / walks as f64,
        t.suspended_ns as f64 / walks as f64 / 1000.0,
    );
    for (name, end) in WalkEnd::ALL {
        let _ = write!(
            out,
            "  {name} {:.1}%",
            pct(t.walk_ends[end as usize], walks)
        );
    }
    let _ = writeln!(out);
    // Each key's name, interned: stacks become name ids.
    let mut ids: HashMap<u64, usize> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for stack in t.stacks.keys() {
        for &k in stack.iter() {
            if ids.contains_key(&k) {
                continue;
            }
            let name = name_of(k);
            let id = *index.entry(name.clone()).or_insert_with(|| {
                names.push(name);
                names.len() - 1
            });
            ids.insert(k, id);
        }
    }
    let mut inclusive = vec![0u64; names.len()];
    let mut selfs = vec![0u64; names.len()];
    let mut callers: HashMap<(usize, usize), u64> = HashMap::new();
    // `stamp[id] == pass`: the name was counted for this stack already.
    let mut stamp = vec![0usize; names.len()];
    let none = names.len();
    for (pass, (stack, &s)) in t.stacks.iter().enumerate() {
        let pass = pass + 1;
        let inner = ids[&stack[0]];
        selfs[inner] += s;
        let caller = stack[1..]
            .iter()
            .map(|k| ids[k])
            .find(|&id| id != inner)
            .unwrap_or(none);
        *callers.entry((inner, caller)).or_default() += s;
        for k in stack.iter() {
            let id = ids[k];
            if stamp[id] != pass {
                stamp[id] = pass;
                inclusive[id] += s;
            }
        }
    }
    let by_count = |counts: &[u64]| {
        let mut rows: Vec<(usize, u64)> = counts.iter().copied().enumerate().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(names[a.0].cmp(&names[b.0])));
        rows
    };
    let _ = writeln!(
        out,
        "native, the game's phases, inclusive (% of all samples, ms/frame):"
    );
    for (id, s) in by_count(&inclusive).into_iter().take(top) {
        let _ = writeln!(
            out,
            "  {:5.1}% {:6.3}  {}",
            pct(s, n),
            ms_per_frame(s),
            names[id]
        );
    }
    let _ = writeln!(
        out,
        "native, the game's phases, callers of the top {CALLER_FUNCTIONS} by self \
         (% of all samples, ms/frame; each caller's share of the function's self time):"
    );
    for (id, s) in by_count(&selfs).into_iter().take(CALLER_FUNCTIONS) {
        if s == 0 {
            break;
        }
        let _ = writeln!(
            out,
            "  {:5.1}% {:6.3}  {}",
            pct(s, n),
            ms_per_frame(s),
            names[id]
        );
        let mut from: Vec<(usize, u64)> = callers
            .iter()
            .filter(|((inner, _), _)| *inner == id)
            .map(|(&(_, caller), &c)| (caller, c))
            .collect();
        from.sort_by(|a, b| {
            b.1.cmp(&a.1).then_with(|| {
                let name = |i: usize| names.get(i).map_or("", String::as_str);
                name(a.0).cmp(name(b.0))
            })
        });
        for (caller, c) in from.into_iter().take(CALLERS) {
            let name = names.get(caller).map_or("(no caller)", String::as_str);
            let _ = writeln!(
                out,
                "        {:5.1}% {:6.3} {:5.1}%  < {name}",
                pct(c, n),
                ms_per_frame(c),
                pct(c, s)
            );
        }
    }
}

/// The call-target part of a table: the calls through `Engine::call` (and `Engine::vcall`) a
/// frame to each of the `rows` most called exe addresses, their share of all calls, and the
/// self time the exe samples give each (ms a frame, and ns a call). With the profiler on every
/// call takes `Engine::call`'s full path, so a call costs more than in a run without it.
fn call_targets(
    out: &mut String,
    t: &Table,
    frames: u64,
    rows: usize,
    ms_per_frame: &dyn Fn(u64) -> f64,
) {
    use std::fmt::Write;
    let frames = frames.max(1) as f64;
    let counted: u64 = t.targets.values().sum();
    let all = t.calls.max(counted).max(1);
    let _ = writeln!(
        out,
        "call targets: {} distinct, {:.0} calls a frame ({:.0} not counted by target); \
         top {rows} (calls a frame, % of calls, self ms/frame, self ns a call, inclusive \
         ms/frame):",
        t.targets.len(),
        all as f64 / frames,
        t.targets_uncounted as f64 / frames,
    );
    for (addr, c) in sorted(&t.targets).into_iter().take(rows) {
        let per_frame = c as f64 / frames;
        let self_ms = ms_per_frame(t.selfs.get(&addr).copied().unwrap_or(0));
        let inclusive_ms = ms_per_frame(t.inclusive.get(&addr).copied().unwrap_or(0));
        let ns_a_call = if per_frame > 0.0 {
            self_ms * 1e6 / per_frame
        } else {
            0.0
        };
        let kind = kind_of(addr).map_or(String::new(), |k| format!("  {k}"));
        let _ = writeln!(
            out,
            "  {per_frame:10.1} {:5.2}% {self_ms:6.3} {ns_a_call:7.1} {inclusive_ms:6.3}  \
             {addr:08x}{kind}",
            pct(c, all),
        );
    }
}

/// The other threads' part of a table (`NV_PROFILE_THREADS=1`): each group of threads (the
/// name without its number) with the CPU time it spent busy (samples not in a wait), and the
/// busiest functions.
fn other_threads(
    out: &mut String,
    t: &Table,
    top: usize,
    ms_per_frame: &dyn Fn(u64) -> f64,
    sampler: &mut native::Sampler,
) {
    use std::fmt::Write;
    // Each thread is sampled every fourth tick: one of its samples stands for four ticks.
    let scale = |s: u64| s * 4;
    let mut groups: HashMap<String, (u64, u64, Vec<u32>)> = HashMap::new();
    let mut group_of: HashMap<u32, String> = HashMap::new();
    for &id in t.other_samples.keys() {
        let group = thread_group(&sampler.thread_name(id));
        group_of.insert(id, group.clone());
        let g = groups.entry(group).or_default();
        g.0 += t.other_samples[&id];
        g.2.push(id);
    }
    let mut busy_functions: HashMap<String, u64> = HashMap::new();
    for (&(id, rip), &s) in &t.others {
        let (module, symbol) = sampler.name(rip);
        if is_waiting(&module, &symbol) {
            continue;
        }
        let group = group_of.get(&id).cloned().unwrap_or_default();
        if let Some(g) = groups.get_mut(&group) {
            g.1 += s;
        }
        *busy_functions
            .entry(format!("{group}: {module}!{symbol}"))
            .or_default() += s;
    }
    let _ = writeln!(
        out,
        "other threads, busy (not waiting), CPU ms a frame (every 4th tick):"
    );
    let mut rows: Vec<_> = groups.into_iter().collect();
    rows.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then(a.0.cmp(&b.0)));
    for (group, (_, busy, ids)) in rows.into_iter().take(top) {
        let _ = writeln!(
            out,
            "  {:6.3}  {group} ({} thread(s))",
            ms_per_frame(scale(busy)),
            ids.len()
        );
    }
    let _ = writeln!(out, "other threads, busiest functions (CPU ms a frame):");
    for (name, s) in sorted_strings(&busy_functions).into_iter().take(top) {
        let _ = writeln!(out, "  {:6.3}  {name}", ms_per_frame(scale(s)));
    }
}

/// A thread's group: its name without a trailing number (`Compute Task Pool (3)`, `game thread
/// 12`), `unnamed` without a name.
pub fn thread_group(name: &str) -> String {
    let trimmed = name
        .trim_end_matches(|c: char| c.is_ascii_digit() || c == ')' || c == '(' || c == ' ')
        .trim();
    if trimmed.is_empty() {
        "unnamed".into()
    } else {
        trimmed.to_string()
    }
}

/// Whether a sampled function is a wait of the OS (the thread is idle there).
pub fn is_waiting(module: &str, symbol: &str) -> bool {
    (module == "ntdll.dll" || module == "win32u.dll" || module == "kernelbase.dll")
        && [
            "Wait",
            "Delay",
            "RemoveIoCompletion",
            "AlertByThreadId",
            "SignalAndWait",
        ]
        .iter()
        .any(|w| symbol.contains(w))
}

fn sorted_strings(map: &HashMap<String, u64>) -> Vec<(String, u64)> {
    let mut rows: Vec<(String, u64)> = map.iter().map(|(k, v)| (k.clone(), *v)).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    rows
}

/// A Rust v0 or legacy symbol made readable enough for the table: the hashes, disambiguators
/// and back references dropped, the identifiers joined with `::` (generic arguments become more
/// path segments). Not a full demangler.
pub fn readable_symbol(raw: &str) -> String {
    let s = raw.split(".llvm.").next().unwrap_or(raw);
    let (body, legacy) = if let Some(rest) = s.strip_prefix("_R") {
        (rest.as_bytes(), false)
    } else if let Some(rest) = s.strip_prefix("_ZN") {
        (rest.as_bytes(), true)
    } else {
        return s.to_string();
    };
    let mut parts: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let c = body[i];
        if !legacy && (c == b'C' || c == b's') && i + 1 < body.len() {
            // `Cs<base62>_` (a crate's disambiguator) and `s<base62>_` (a nested one).
            let start = if c == b'C' && body[i + 1] == b's' {
                i + 2
            } else if c == b's' {
                i + 1
            } else {
                i += 1;
                continue;
            };
            let mut j = start;
            while j < body.len() && body[j].is_ascii_alphanumeric() {
                j += 1;
            }
            if j < body.len() && body[j] == b'_' {
                i = j + 1;
                continue;
            }
            i += 1;
            continue;
        }
        if !legacy && c == b'B' {
            // A back reference: `B<base62>_`.
            let mut j = i + 1;
            while j < body.len() && body[j].is_ascii_alphanumeric() {
                j += 1;
            }
            i = if j < body.len() && body[j] == b'_' {
                j + 1
            } else {
                i + 1
            };
            continue;
        }
        if c.is_ascii_digit() && c != b'0' {
            let mut j = i;
            let mut len = 0usize;
            while j < body.len() && body[j].is_ascii_digit() {
                len = len * 10 + usize::from(body[j] - b'0');
                j += 1;
            }
            // v0 puts a `_` between the length and an identifier that starts with a digit or `_`.
            if !legacy && j < body.len() && body[j] == b'_' {
                j += 1;
            }
            if j + len <= body.len() && len > 0 {
                let ident = &s[s.len() - body.len() + j..s.len() - body.len() + j + len];
                // Legacy symbols end with a `17h<hash>` segment.
                if !(legacy && ident.len() == 17 && ident.starts_with('h')) {
                    parts.push(ident);
                }
                i = j + len;
                continue;
            }
        }
        i += 1;
    }
    if parts.is_empty() {
        return s.to_string();
    }
    parts.join("::")
}

#[cfg(all(windows, target_arch = "x86_64"))]
mod native {
    //! The OS thread holding the engine: suspended for a moment, its instruction pointer read
    //! (or its stack walked, [`Sampler::walk`]), resumed (nothing that could take a lock runs
    //! while it is suspended); named through `dbghelp.dll`, loaded when the first table is
    //! printed.
    use super::{Walk, WalkEnd, MAX_FRAMES};
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::time::{Duration, Instant};

    type Handle = *mut c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThreadId() -> u32;
        fn GetCurrentProcess() -> Handle;
        fn OpenThread(access: u32, inherit: i32, id: u32) -> Handle;
        fn SuspendThread(thread: Handle) -> u32;
        fn ResumeThread(thread: Handle) -> u32;
        fn GetThreadContext(thread: Handle, context: *mut Context) -> i32;
        fn CreateWaitableTimerExW(
            attributes: *mut c_void,
            name: *const u16,
            flags: u32,
            access: u32,
        ) -> Handle;
        fn SetWaitableTimer(
            timer: Handle,
            due: *const i64,
            period: i32,
            routine: *mut c_void,
            arg: *mut c_void,
            resume: i32,
        ) -> i32;
        fn WaitForSingleObject(handle: Handle, ms: u32) -> u32;
        fn LoadLibraryA(name: *const u8) -> Handle;
        fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
        fn GetModuleHandleExA(flags: u32, address: *const u8, module: *mut Handle) -> i32;
        fn GetModuleFileNameA(module: Handle, name: *mut u8, size: u32) -> u32;
        fn CreateToolhelp32Snapshot(flags: u32, process: u32) -> Handle;
        fn Thread32First(snapshot: Handle, entry: *mut ThreadEntry) -> i32;
        fn Thread32Next(snapshot: Handle, entry: *mut ThreadEntry) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
        fn GetCurrentProcessId() -> u32;
        fn LocalFree(memory: *mut c_void) -> *mut c_void;
        fn GetModuleHandleA(name: *const u8) -> Handle;
        fn GetExitCodeThread(thread: Handle, code: *mut u32) -> i32;
        fn K32EnumProcessModules(
            process: Handle,
            modules: *mut Handle,
            bytes: u32,
            needed: *mut u32,
        ) -> i32;
        #[allow(clippy::too_many_arguments)]
        fn RtlVirtualUnwind(
            handler_type: u32,
            image_base: u64,
            control_pc: u64,
            entry: *const RuntimeFunction,
            context: *mut Context,
            handler_data: *mut *mut c_void,
            establisher_frame: *mut u64,
            context_pointers: *mut c_void,
        ) -> *mut c_void;
    }

    /// `NtQueryInformationThread` (ntdll, looked up by name).
    type QueryThread = unsafe extern "system" fn(Handle, u32, *mut c_void, u32, *mut u32) -> i32;

    /// `RUNTIME_FUNCTION`: one function's code range and unwind information (image offsets).
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RuntimeFunction {
        begin: u32,
        end: u32,
        unwind: u32,
    }

    /// A loaded image and its unwind table (`.pdata`, sorted by `begin`).
    struct Image {
        base: u64,
        end: u64,
        table: *const RuntimeFunction,
        len: usize,
    }

    /// The image at `base` (a module handle), read from its PE headers; `None` when they are not
    /// a 64-bit image's.
    ///
    /// # Safety
    /// `base` is a loaded module's handle (its headers are mapped).
    unsafe fn image_at(base: u64) -> Option<Image> {
        let p = base as *const u8;
        let u16_at = |at: usize| std::ptr::read_unaligned(p.add(at) as *const u16);
        let u32_at = |at: usize| std::ptr::read_unaligned(p.add(at) as *const u32);
        if u16_at(0) != 0x5a4d {
            return None;
        }
        let nt = u32_at(0x3c) as usize;
        // "PE\0\0", then the optional header of a PE32+ image after the 20-byte file header.
        if u32_at(nt) != 0x4550 || u16_at(nt + 24) != 0x20b {
            return None;
        }
        let optional = nt + 24;
        let size = u64::from(u32_at(optional + 56));
        let directories = u32_at(optional + 108);
        // Data directory 3: the exception directory.
        let (rva, bytes) = if directories > 3 {
            (u32_at(optional + 136), u32_at(optional + 140))
        } else {
            (0, 0)
        };
        Some(Image {
            base,
            end: base + size,
            table: (base + u64::from(rva)) as *const RuntimeFunction,
            len: if rva == 0 {
                0
            } else {
                bytes as usize / std::mem::size_of::<RuntimeFunction>()
            },
        })
    }

    /// One code address a walk looked up ([`walk_stack`]), kept for the next walks: the stacks of
    /// successive samples share most of their frames, and a lookup in a large image's unwind
    /// table (hundreds of thousands of entries, cold in the cache) cost about a microsecond.
    #[derive(Clone, Copy)]
    struct Found {
        /// The address looked up (0: an empty slot).
        at: u64,
        /// Its image's base, its unwind entry (null: a leaf function, it has none) and its
        /// function's start (the frame's key).
        base: u64,
        entry: *const RuntimeFunction,
        key: u64,
    }

    impl Found {
        const EMPTY: Found = Found {
            at: 0,
            base: 0,
            entry: std::ptr::null(),
            key: 0,
        };
    }

    /// How many lookups [`Found`] keeps (a power of two; direct-mapped).
    const FOUND_SLOTS: usize = 1 << 12;

    /// The lookup of code address `at` (`None`: in no known image), from `found` or made and
    /// kept there.
    ///
    /// # Safety
    /// The images are loaded.
    unsafe fn look_up(
        images: &[Image],
        found: &mut [Found; FOUND_SLOTS],
        at: u64,
    ) -> Option<Found> {
        let slot = (at.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> (64 - FOUND_SLOTS.trailing_zeros()))
            as usize;
        if found[slot].at == at {
            return Some(found[slot]);
        }
        let im = image_of(images, at)?;
        let entry = function_entry(im, at);
        let f = Found {
            at,
            base: im.base,
            entry: entry.unwrap_or(std::ptr::null()),
            key: entry.map_or(at, |f| function_start(im.base, f)),
        };
        found[slot] = f;
        Some(f)
    }

    /// The image holding `pc`.
    fn image_of(images: &[Image], pc: u64) -> Option<&Image> {
        let i = images.partition_point(|im| im.base <= pc).checked_sub(1)?;
        let im = &images[i];
        (pc < im.end).then_some(im)
    }

    /// The unwind-table entry of the function holding `pc` in `im` (an indirect entry followed),
    /// `None` for a leaf function (it has none).
    ///
    /// # Safety
    /// `im` is a loaded image ([`image_at`]).
    unsafe fn function_entry(im: &Image, pc: u64) -> Option<*const RuntimeFunction> {
        if im.len == 0 {
            return None;
        }
        let rva = (pc - im.base) as u32;
        let table = std::slice::from_raw_parts(im.table, im.len);
        let f = &table[table.partition_point(|f| f.begin <= rva).checked_sub(1)?];
        if rva >= f.end {
            return None;
        }
        let mut f = f as *const RuntimeFunction;
        // `RUNTIME_FUNCTION_INDIRECT`: the entry names another entry.
        if (*f).unwind & 1 != 0 {
            f = (im.base + u64::from((*f).unwind & !1)) as *const RuntimeFunction;
        }
        Some(f)
    }

    /// The start of the function `f` belongs to: a part of a function split up (its unwind
    /// information chained, `UNW_FLAG_CHAININFO`) names the entry before it, back to the first.
    ///
    /// # Safety
    /// `f` is an entry of the image at `base` ([`function_entry`]).
    unsafe fn function_start(base: u64, mut f: *const RuntimeFunction) -> u64 {
        for _ in 0..8 {
            let info = (base + u64::from((*f).unwind & !1)) as *const u8;
            if (*info >> 3) & 0x4 == 0 {
                break;
            }
            let codes = usize::from(*info.add(2));
            f = info.add(4 + 2 * ((codes + 1) & !1)) as *const RuntimeFunction;
        }
        base + u64::from((*f).begin)
    }

    /// The x64 `CONTEXT`'s general registers (`Rax` at 0x78, then `Rcx`, `Rdx`, `Rbx`, `Rsp`,
    /// `Rbp`, `Rsi`, `Rdi`, `R8` to `R15`, in the unwind codes' order) and `Rip` (0xf8).
    impl Context {
        fn word(&self, at: usize) -> u64 {
            u64::from_le_bytes(self.0[at..at + 8].try_into().expect("8 bytes"))
        }
        fn set_word(&mut self, at: usize, v: u64) {
            self.0[at..at + 8].copy_from_slice(&v.to_le_bytes());
        }
        fn register(&self, n: u8) -> u64 {
            self.word(0x78 + 8 * usize::from(n & 0xf))
        }
        fn rsp(&self) -> u64 {
            self.word(0x98)
        }
        fn rip(&self) -> u64 {
            self.word(0xf8)
        }
    }

    /// Whether `f`'s frame register (if it has one and the prologue has set it) points into the
    /// stack, `low..high`: `RtlVirtualUnwind` reads the frame from it.
    ///
    /// # Safety
    /// `f` is an entry of the image at `base` ([`function_entry`]).
    unsafe fn frame_register_ok(
        base: u64,
        f: *const RuntimeFunction,
        pc: u64,
        ctx: &Context,
        low: u64,
        high: u64,
    ) -> bool {
        let info = (base + u64::from((*f).unwind & !1)) as *const u8;
        let prologue = u64::from(*info.add(1));
        let frame = *info.add(3);
        if frame & 0xf == 0 || pc.wrapping_sub(base + u64::from((*f).begin)) < prologue {
            return true;
        }
        let at = ctx
            .register(frame & 0xf)
            .wrapping_sub(16 * u64::from(frame >> 4));
        (low..high).contains(&at)
    }

    /// Walks the suspended thread whose registers are `ctx` (and whose stack is `low..high`) up
    /// to `keys.len()` frames: each frame's function into `keys` (its start where its image's
    /// unwind table has it, else the address itself) and its address into `pcs` (the
    /// instruction pointer, then return addresses), the lookups kept in `found`. Returns the
    /// frames and why the walk ended. Takes no lock and allocates nothing.
    ///
    /// # Safety
    /// The images are loaded; `low..high` is the thread's committed stack; the thread stays
    /// suspended throughout.
    unsafe fn walk_stack(
        images: &[Image],
        found: &mut [Found; FOUND_SLOTS],
        ctx: &mut Context,
        (low, high): (u64, u64),
        keys: &mut [u64],
        pcs: &mut [u64],
    ) -> (usize, WalkEnd) {
        let limit = keys.len().min(pcs.len());
        let mut n = 0;
        loop {
            if n >= limit {
                return (n, WalkEnd::Limit);
            }
            let pc = ctx.rip();
            let rsp = ctx.rsp();
            // A return address points after its call: the call is at the byte before.
            let at = if n == 0 { pc } else { pc - 1 };
            pcs[n] = pc;
            keys[n] = at;
            n += 1;
            if !(low..high).contains(&rsp) || rsp & 7 != 0 {
                return (n, WalkEnd::Bad);
            }
            let Some(code) = look_up(images, found, at) else {
                return (n, WalkEnd::Unknown);
            };
            keys[n - 1] = code.key;
            match (!code.entry.is_null()).then_some(code.entry) {
                Some(f) => {
                    if !frame_register_ok(code.base, f, pc, ctx, low, high) {
                        return (n, WalkEnd::Bad);
                    }
                    let mut handler_data = std::ptr::null_mut();
                    let mut establisher = 0u64;
                    // UNW_FLAG_NHANDLER: the frame is only unwound.
                    RtlVirtualUnwind(
                        0,
                        code.base,
                        pc,
                        f,
                        ctx,
                        &mut handler_data,
                        &mut establisher,
                        std::ptr::null_mut(),
                    );
                }
                None => {
                    // A leaf function: its return address is at the stack pointer.
                    if rsp + 8 > high {
                        return (n, WalkEnd::Bad);
                    }
                    ctx.set_word(0xf8, std::ptr::read(rsp as *const u64));
                    ctx.set_word(0x98, rsp + 8);
                }
            }
            if ctx.rip() == 0 {
                return (n, WalkEnd::Start);
            }
            // Each frame moves up the stack.
            let up = ctx.rsp();
            if up <= rsp || up >= high {
                return (n, WalkEnd::Bad);
            }
        }
    }

    /// `THREADENTRY32`.
    #[repr(C)]
    #[derive(Default)]
    struct ThreadEntry {
        size: u32,
        usage: u32,
        thread: u32,
        process: u32,
        base_priority: i32,
        delta_priority: i32,
        flags: u32,
    }

    const SNAP_THREAD: u32 = 0x4;
    type GetThreadDescription = unsafe extern "system" fn(Handle, *mut *mut u16) -> i32;

    const THREAD_SUSPEND_RESUME: u32 = 0x0002;
    const THREAD_GET_CONTEXT: u32 = 0x0008;
    const THREAD_QUERY_INFORMATION: u32 = 0x0040;
    const CONTEXT_CONTROL: u32 = 0x0010_0001;
    const CONTEXT_INTEGER: u32 = 0x0010_0002;
    const STILL_ACTIVE: u32 = 259;
    const CREATE_WAITABLE_TIMER_HIGH_RESOLUTION: u32 = 0x2;
    const TIMER_ALL_ACCESS: u32 = 0x1f_0003;
    const FROM_ADDRESS: u32 = 0x4;
    const UNCHANGED_REFCOUNT: u32 = 0x2;

    /// The x64 `CONTEXT` (1232 bytes, 16-byte aligned): `ContextFlags` at 0x30, `Rip` at 0xf8.
    #[repr(C, align(16))]
    struct Context([u8; 0x4d0]);

    pub fn current_thread_id() -> u32 {
        thread_local! {
            static ID: u32 = unsafe { GetCurrentThreadId() };
        }
        ID.with(|id| *id)
    }

    type SymInitialize = unsafe extern "system" fn(Handle, *const u8, i32) -> i32;
    type SymSetOptions = unsafe extern "system" fn(u32) -> u32;
    type SymFromAddr = unsafe extern "system" fn(Handle, u64, *mut u64, *mut u8) -> i32;
    type SymGetModuleInfo = unsafe extern "system" fn(Handle, u64, *mut u8) -> i32;

    pub struct Sampler {
        threads: HashMap<u32, Handle>,
        timer: Handle,
        context: Box<Context>,
        sym_from_addr: Option<SymFromAddr>,
        sym_module_info: Option<SymGetModuleInfo>,
        sym_tried: bool,
        names: HashMap<u64, (String, String)>,
        modules: HashMap<usize, String>,
        thread_names: HashMap<u32, String>,
        /// The loaded images by base, with their unwind tables ([`Sampler::refresh_images`]),
        /// when they were last listed, and whether a walk met code in none of them since.
        images: Vec<Image>,
        images_at: Option<Instant>,
        images_missed: bool,
        /// The walks' lookups of code addresses (emptied when the images are listed again).
        found: Box<[Found; FOUND_SLOTS]>,
        /// Each walked thread's TEB (its stack's bounds are at 8 and 0x10).
        tebs: HashMap<u32, u64>,
        query_thread: Option<QueryThread>,
        /// The last walk's frames: each one's function and address ([`walk_stack`]).
        keys: Box<[u64; MAX_FRAMES]>,
        pcs: Box<[u64; MAX_FRAMES]>,
    }

    impl Sampler {
        pub fn new() -> Self {
            // SAFETY: plain Win32 calls with valid arguments; a null timer falls back to sleep.
            let timer = unsafe {
                CreateWaitableTimerExW(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                    TIMER_ALL_ACCESS,
                )
            };
            // SAFETY: ntdll is always loaded; NtQueryInformationThread is looked up by name and
            // has the signature of `QueryThread`.
            let query_thread = unsafe {
                let f = GetProcAddress(
                    GetModuleHandleA(b"ntdll.dll\0".as_ptr()),
                    b"NtQueryInformationThread\0".as_ptr(),
                );
                (!f.is_null()).then(|| std::mem::transmute::<*mut c_void, QueryThread>(f))
            };
            Sampler {
                threads: HashMap::new(),
                timer,
                context: Box::new(Context([0; 0x4d0])),
                sym_from_addr: None,
                sym_module_info: None,
                sym_tried: false,
                names: HashMap::new(),
                modules: HashMap::new(),
                thread_names: HashMap::new(),
                images: Vec::new(),
                images_at: None,
                images_missed: false,
                found: Box::new([Found::EMPTY; FOUND_SLOTS]),
                tebs: HashMap::new(),
                query_thread,
                keys: Box::new([0; MAX_FRAMES]),
                pcs: Box::new([0; MAX_FRAMES]),
            }
        }

        /// A handle to OS thread `id` with suspend, context and query access (kept open).
        fn thread(&mut self, id: u32) -> Option<Handle> {
            let handle = *self.threads.entry(id).or_insert_with(|| {
                // SAFETY: OpenThread with a thread id; a null handle is checked below.
                unsafe {
                    OpenThread(
                        THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT | THREAD_QUERY_INFORMATION,
                        0,
                        id,
                    )
                }
            });
            (!handle.is_null()).then_some(handle)
        }

        /// OS thread `id`'s TEB (`NtQueryInformationThread`'s basic information, the TEB's
        /// address at 8), 0 unknown.
        fn teb(&mut self, id: u32, handle: Handle) -> u64 {
            if let Some(&teb) = self.tebs.get(&id) {
                return teb;
            }
            let mut info = [0u64; 6];
            let teb = match self.query_thread {
                // SAFETY: ThreadBasicInformation (0) into a buffer of its 48 bytes.
                Some(query) => unsafe {
                    if query(
                        handle,
                        0,
                        info.as_mut_ptr() as *mut c_void,
                        48,
                        std::ptr::null_mut(),
                    ) >= 0
                    {
                        info[1]
                    } else {
                        0
                    }
                },
                None => 0,
            };
            self.tebs.insert(id, teb);
            teb
        }

        /// Lists the loaded images again (`K32EnumProcessModules`) with their unwind tables:
        /// at the first walk, every five seconds (an image unloaded is dropped), and a quarter
        /// of a second after a walk met code in no known image (one loaded since).
        fn refresh_images(&mut self) {
            let due = match self.images_at {
                None => true,
                Some(at) => {
                    let since = at.elapsed();
                    since >= Duration::from_secs(5)
                        || (self.images_missed && since >= Duration::from_millis(250))
                }
            };
            if !due {
                return;
            }
            self.images_at = Some(Instant::now());
            self.images_missed = false;
            // An image unloaded since may have left lookups into its unwind table.
            self.found.fill(Found::EMPTY);
            let mut handles: Vec<Handle> = vec![std::ptr::null_mut(); 512];
            // SAFETY: the buffer's size in bytes is passed; the handles listed are loaded
            // modules (an image unloaded in between is a risk this diagnostic takes).
            unsafe {
                loop {
                    let bytes = (handles.len() * std::mem::size_of::<Handle>()) as u32;
                    let mut needed = 0u32;
                    if K32EnumProcessModules(
                        GetCurrentProcess(),
                        handles.as_mut_ptr(),
                        bytes,
                        &mut needed,
                    ) == 0
                    {
                        return;
                    }
                    let count = needed as usize / std::mem::size_of::<Handle>();
                    if count <= handles.len() {
                        handles.truncate(count);
                        break;
                    }
                    handles.resize(count, std::ptr::null_mut());
                }
                self.images = handles.iter().filter_map(|&h| image_at(h as u64)).collect();
            }
            self.images.sort_by_key(|im| im.base);
        }

        /// Walks OS thread `id`'s native stack, the thread suspended meanwhile: up to `depth`
        /// frames into [`Sampler::keys`] ([`walk_stack`]). `None` when the thread cannot be
        /// suspended or read.
        pub fn walk(&mut self, id: u32, depth: usize) -> Option<Walk> {
            if id == 0 || id == current_thread_id() {
                return None;
            }
            let handle = self.thread(id)?;
            let teb = self.teb(id, handle);
            if teb == 0 {
                return None;
            }
            self.refresh_images();
            let depth = depth.clamp(1, MAX_FRAMES);
            let ctx = &mut *self.context;
            ctx.0[0x30..0x34].copy_from_slice(&(CONTEXT_CONTROL | CONTEXT_INTEGER).to_le_bytes());
            let started = Instant::now();
            // SAFETY: the handle has suspend/resume and get-context access; between the suspend
            // and the resume only `walk_stack` runs (no lock, no allocation: its buffers are
            // ours), on the thread's committed stack as its TEB gives it (base at 8, limit at
            // 0x10; the TEB is this process's memory).
            let walked = unsafe {
                if SuspendThread(handle) == u32::MAX {
                    return None;
                }
                // A thread that has ended (its id since taken by another, say) has no TEB to
                // read: its handle and TEB are forgotten, and the id is opened again next time.
                let mut code = 0u32;
                if GetExitCodeThread(handle, &mut code) == 0 || code != STILL_ACTIVE {
                    ResumeThread(handle);
                    CloseHandle(handle);
                    self.threads.remove(&id);
                    self.tebs.remove(&id);
                    return None;
                }
                let walked = if GetThreadContext(handle, ctx as *mut Context) != 0 {
                    let high = std::ptr::read_volatile((teb + 8) as *const u64);
                    let low = std::ptr::read_volatile((teb + 0x10) as *const u64);
                    let rip = ctx.rip();
                    let (frames, end) = walk_stack(
                        &self.images,
                        &mut self.found,
                        ctx,
                        (low, high),
                        &mut self.keys[..depth],
                        &mut self.pcs[..depth],
                    );
                    Some((rip, frames, end))
                } else {
                    None
                };
                ResumeThread(handle);
                walked
            };
            let suspended_ns = started.elapsed().as_nanos() as u64;
            let (rip, frames, end) = walked?;
            if end == WalkEnd::Unknown {
                self.images_missed = true;
            }
            Some(Walk {
                rip,
                frames,
                end,
                suspended_ns,
            })
        }

        /// The first `n` frames of the last walk: each one's function (its start address, or
        /// the address itself where its image has no unwind entry for it), innermost first.
        pub fn keys(&self, n: usize) -> &[u64] {
            &self.keys[..n.min(MAX_FRAMES)]
        }

        /// The first `n` frames of the last walk: the instruction pointer, then the return
        /// addresses.
        #[cfg(test)]
        pub fn pcs(&self, n: usize) -> &[u64] {
            &self.pcs[..n.min(MAX_FRAMES)]
        }

        /// Waits about a millisecond.
        pub fn wait_tick(&mut self) {
            if self.timer.is_null() {
                std::thread::sleep(Duration::from_millis(1));
                return;
            }
            // Relative, in 100 ns units.
            let due: i64 = -10_000;
            // SAFETY: the timer handle is ours and valid; `due` outlives the call.
            unsafe {
                if SetWaitableTimer(
                    self.timer,
                    &due,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                ) == 0
                {
                    std::thread::sleep(Duration::from_millis(1));
                    return;
                }
                WaitForSingleObject(self.timer, 0xffff_ffff);
            }
        }

        /// The instruction pointer of OS thread `id` (0: none).
        pub fn instruction_pointer(&mut self, id: u32) -> Option<u64> {
            if id == 0 || id == current_thread_id() {
                return None;
            }
            let handle = self.thread(id)?;
            let ctx = &mut *self.context;
            ctx.0[0x30..0x34].copy_from_slice(&CONTEXT_CONTROL.to_le_bytes());
            // SAFETY: the handle has suspend/resume and get-context access; nothing between
            // the suspend and the resume allocates or takes a lock (the context buffer is ours).
            let ok = unsafe {
                if SuspendThread(handle) == u32::MAX {
                    return None;
                }
                let ok = GetThreadContext(handle, ctx as *mut Context) != 0;
                ResumeThread(handle);
                ok
            };
            ok.then(|| u64::from_le_bytes(ctx.0[0xf8..0x100].try_into().expect("8 bytes")))
        }

        /// The ids of every thread of this process but the sampler's own.
        pub fn process_threads(&mut self) -> Vec<u32> {
            let mut ids = Vec::new();
            // SAFETY: a Toolhelp snapshot walked with a correctly sized entry, then closed.
            unsafe {
                let snapshot = CreateToolhelp32Snapshot(SNAP_THREAD, 0);
                if snapshot.is_null() || snapshot as isize == -1 {
                    return ids;
                }
                let process = GetCurrentProcessId();
                let mut entry = ThreadEntry {
                    size: std::mem::size_of::<ThreadEntry>() as u32,
                    ..Default::default()
                };
                let mut more = Thread32First(snapshot, &mut entry) != 0;
                while more {
                    if entry.process == process && entry.thread != current_thread_id() {
                        ids.push(entry.thread);
                    }
                    more = Thread32Next(snapshot, &mut entry) != 0;
                }
                CloseHandle(snapshot);
            }
            ids
        }

        /// A thread's name (`GetThreadDescription`), empty without one.
        pub fn thread_name(&mut self, id: u32) -> String {
            if let Some(name) = self.thread_names.get(&id) {
                return name.clone();
            }
            let mut name = String::new();
            // SAFETY: GetThreadDescription looked up in kernel32 (Windows 10 1607 and later)
            // and called on a handle with query access; its string is freed with LocalFree.
            unsafe {
                let handle = OpenThread(THREAD_QUERY_INFORMATION, 0, id);
                let kernel = GetModuleHandleA(b"kernel32.dll\0".as_ptr());
                let f = GetProcAddress(kernel, b"GetThreadDescription\0".as_ptr());
                if !handle.is_null() && !f.is_null() {
                    let f: GetThreadDescription = std::mem::transmute(f);
                    let mut text: *mut u16 = std::ptr::null_mut();
                    if f(handle, &mut text) >= 0 && !text.is_null() {
                        let mut n = 0;
                        while *text.add(n) != 0 && n < 256 {
                            n += 1;
                        }
                        name = String::from_utf16_lossy(std::slice::from_raw_parts(text, n));
                        LocalFree(text as *mut c_void);
                    }
                }
                if !handle.is_null() {
                    CloseHandle(handle);
                }
            }
            self.thread_names.insert(id, name.clone());
            name
        }

        fn load_symbols(&mut self) {
            if self.sym_tried {
                return;
            }
            self.sym_tried = true;
            // SAFETY: dbghelp.dll is a system DLL; the procedures are looked up by name and
            // called with their documented signatures. Only this thread uses dbghelp.
            unsafe {
                let dll = LoadLibraryA(b"dbghelp.dll\0".as_ptr());
                if dll.is_null() {
                    return;
                }
                let set_options = GetProcAddress(dll, b"SymSetOptions\0".as_ptr());
                let init = GetProcAddress(dll, b"SymInitialize\0".as_ptr());
                let from_addr = GetProcAddress(dll, b"SymFromAddr\0".as_ptr());
                if init.is_null() || from_addr.is_null() {
                    return;
                }
                if !set_options.is_null() {
                    let set_options: SymSetOptions = std::mem::transmute(set_options);
                    // SYMOPT_UNDNAME | SYMOPT_DEFERRED_LOADS
                    set_options(0x2 | 0x4);
                }
                // dbghelp looks for a module's PDB by the name the module was linked with
                // (`nv_viewer.pdb`) at the path it was linked with, then on its search path, not
                // next to the module: the folder `<program>.sym` beside the program (a copied
                // program's own PDB, so copies of several builds can share a folder), the
                // program's folder, then `NV_PROFILE_SYMBOLS` (folders separated by `;`). A PDB
                // of another build is refused (its signature differs).
                let exe = std::env::current_exe().ok();
                let mut path = exe
                    .as_ref()
                    .and_then(|p| {
                        let dir = p.parent()?.to_string_lossy().into_owned();
                        let stem = p.file_stem()?.to_string_lossy().into_owned();
                        Some(format!("{dir}\\{stem}.sym;{dir}"))
                    })
                    .unwrap_or_default();
                if let Ok(more) = std::env::var("NV_PROFILE_SYMBOLS") {
                    path = format!("{path};{more}");
                }
                let path = std::ffi::CString::new(path).unwrap_or_default();
                let init: SymInitialize = std::mem::transmute(init);
                if init(GetCurrentProcess(), path.as_ptr() as *const u8, 1) == 0 {
                    return;
                }
                self.sym_from_addr =
                    Some(std::mem::transmute::<*mut c_void, SymFromAddr>(from_addr));
                let module_info = GetProcAddress(dll, b"SymGetModuleInfo64\0".as_ptr());
                self.sym_module_info = (!module_info.is_null())
                    .then(|| std::mem::transmute::<*mut c_void, SymGetModuleInfo>(module_info));
            }
        }

        /// Where the program's symbols came from: the PDB dbghelp loaded for it, and whether it
        /// has line numbers and private symbols (a build with `debug = 0` has public symbols
        /// only, and a time inside a function without one counts under the nearest one before).
        pub fn symbol_source(&mut self) -> String {
            // SAFETY: GetModuleHandleA(null) is the program's base.
            let base = unsafe { GetModuleHandleA(std::ptr::null()) } as u64;
            // A lookup in the program loads its symbols (they load on first use).
            let _ = self.name(base + 0x1000);
            let Some(info) = self.sym_module_info else {
                return "no dbghelp".into();
            };
            // IMAGEHLP_MODULE64: 1680 bytes; SymType at 32, LoadedPdbName at 580 (256 bytes),
            // LineNumbers at 1652, GlobalSymbols at 1656.
            let mut buf = vec![0u64; 1680 / 8];
            let bytes = buf.as_mut_ptr() as *mut u8;
            // SAFETY: `buf` is 8-aligned and 1680 bytes, its size in its first word.
            unsafe {
                *(bytes as *mut u32) = 1680;
                if info(GetCurrentProcess(), base, bytes) == 0 {
                    return "no symbols for the program".into();
                }
                let all = std::slice::from_raw_parts(bytes, 1680);
                let word = |at: usize| u32::from_le_bytes(all[at..at + 4].try_into().expect("4"));
                let pdb = &all[580..836];
                let pdb = String::from_utf8_lossy(
                    &pdb[..pdb.iter().position(|&c| c == 0).unwrap_or(256)],
                )
                .into_owned();
                format!(
                    "symbols from {} (type {}, line numbers {}, private symbols {})",
                    if pdb.is_empty() { "(none)" } else { &pdb },
                    word(32),
                    if word(1652) != 0 { "yes" } else { "no" },
                    if word(1656) != 0 { "yes" } else { "no" },
                )
            }
        }

        fn module(&mut self, addr: u64) -> String {
            let mut module: Handle = std::ptr::null_mut();
            // SAFETY: GetModuleHandleExA with FROM_ADDRESS reads only the address's module.
            let found = unsafe {
                GetModuleHandleExA(
                    FROM_ADDRESS | UNCHANGED_REFCOUNT,
                    addr as *const u8,
                    &mut module,
                ) != 0
            };
            if !found {
                return "?".into();
            }
            self.modules
                .entry(module as usize)
                .or_insert_with(|| {
                    let mut name = [0u8; 520];
                    // SAFETY: the buffer is ours and its size is passed.
                    let n =
                        unsafe { GetModuleFileNameA(module, name.as_mut_ptr(), name.len() as u32) }
                            as usize;
                    let path = String::from_utf8_lossy(&name[..n.min(name.len())]).to_string();
                    path.rsplit(['\\', '/'])
                        .next()
                        .unwrap_or(&path)
                        .to_ascii_lowercase()
                })
                .clone()
        }

        /// (module, function) of a native address.
        pub fn name(&mut self, addr: u64) -> (String, String) {
            if let Some(n) = self.names.get(&addr) {
                return n.clone();
            }
            self.load_symbols();
            let module = self.module(addr);
            let mut symbol = String::from("?");
            if let Some(from_addr) = self.sym_from_addr {
                // SYMBOL_INFO: 88 bytes, then the name (MaxNameLen at 80, NameLen at 76).
                let mut buf = vec![0u64; (88 + 1024) / 8 + 1];
                let bytes = buf.as_mut_ptr() as *mut u8;
                let mut displacement = 0u64;
                // SAFETY: `buf` is 8-aligned and holds the structure plus 1024 name bytes.
                unsafe {
                    *(bytes as *mut u32) = 88;
                    *(bytes.add(80) as *mut u32) = 1024;
                    if from_addr(GetCurrentProcess(), addr, &mut displacement, bytes) != 0 {
                        let len = (*(bytes.add(76) as *const u32)).min(1024) as usize;
                        let name = std::slice::from_raw_parts(bytes.add(84), len);
                        symbol = super::readable_symbol(&String::from_utf8_lossy(name));
                    }
                }
            }
            let n = (module, symbol);
            self.names.insert(addr, n.clone());
            n
        }
    }
}

#[cfg(not(all(windows, target_arch = "x86_64")))]
mod native {
    //! No native sampling here: the exe-address samples only.
    pub fn current_thread_id() -> u32 {
        0
    }

    pub struct Sampler;

    impl Sampler {
        pub fn new() -> Self {
            Sampler
        }
        pub fn wait_tick(&mut self) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        pub fn instruction_pointer(&mut self, _id: u32) -> Option<u64> {
            None
        }
        pub fn name(&mut self, _addr: u64) -> (String, String) {
            ("?".into(), "?".into())
        }
        pub fn process_threads(&mut self) -> Vec<u32> {
            Vec::new()
        }
        pub fn thread_name(&mut self, _id: u32) -> String {
            String::new()
        }
        pub fn walk(&mut self, _id: u32, _depth: usize) -> Option<super::Walk> {
            None
        }
        pub fn keys(&self, _n: usize) -> &[u64] {
            &[]
        }
        pub fn symbol_source(&mut self) -> String {
            "no native symbols on this platform".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_read_as_paths() {
        assert_eq!(
            readable_symbol("_RNvNtNtNtCshZYmvcS8kkr_11engine_core5units3crt4math9floor_x87"),
            "engine_core::units::crt::math::floor_x87"
        );
        assert_eq!(
            readable_symbol(
                "_RNvNtNtCshZYmvcS8kkr_11engine_core5units3crt20set_x87_control_word.llvm.123"
            ),
            "engine_core::units::crt::set_x87_control_word"
        );
        assert_eq!(
            readable_symbol("_ZN4core3ptr13drop_in_place17h0123456789abcdefE"),
            "core::ptr::drop_in_place"
        );
        assert_eq!(readable_symbol("memcpy"), "memcpy");
    }

    #[test]
    fn platform_addresses_have_kinds() {
        assert_eq!(
            kind_of(0x7e02_0210).as_deref(),
            Some("D3D9/D3DX platform object")
        );
        assert_eq!(
            kind_of(0x7e00_0410).as_deref(),
            Some("DirectSound platform object")
        );
        assert_eq!(kind_of(0x0086_e650), None);
    }

    /// The table counts phases, threads, kinds and self and inclusive samples as sampled.
    #[test]
    fn the_table_reads_back_its_samples() {
        let mut t = Table {
            samples: 4,
            phases: [1, 3, 0, 0],
            ..Default::default()
        };
        t.threads.insert(1, 3);
        t.kinds.insert("exe code".into(), 2);
        t.kinds.insert("D3D9/D3DX platform object".into(), 1);
        t.selfs.insert(0x0086_e650, 2);
        t.selfs.insert(0x7e02_0210, 1);
        t.inclusive.insert(0x0086_e650, 3);
        let text = report(
            &t,
            Duration::from_secs(1),
            10,
            Rows { top: 5, calls: 5 },
            &mut native::Sampler::new(),
        );
        assert!(
            text.contains("10 game frames (10.0 frames/s, 100.00 ms a frame)"),
            "{text}"
        );
        assert!(text.contains("game 75.0% (75.00 ms/frame)"), "{text}");
        assert!(text.contains("viewer 25.0%"), "{text}");
        assert!(text.contains("exe code 66.7%"), "{text}");
        assert!(
            text.contains(
                " 50.0% 50.000  0086e650
"
            ),
            "{text}"
        );
        assert!(
            text.contains("7e020210  D3D9/D3DX platform object"),
            "{text}"
        );
        assert!(
            text.contains(
                " 75.0% 75.000  0086e650
"
            ),
            "{text}"
        );
    }

    #[test]
    fn threads_group_by_name_and_waits_are_idle() {
        assert_eq!(thread_group("Compute Task Pool (3)"), "Compute Task Pool");
        assert_eq!(thread_group("game thread 12"), "game thread");
        assert_eq!(thread_group(""), "unnamed");
        assert!(is_waiting("ntdll.dll", "ZwWaitForAlertByThreadId"));
        assert!(is_waiting("ntdll.dll", "NtDelayExecution"));
        assert!(!is_waiting("ntdll.dll", "RtlAllocateHeap"));
        assert!(!is_waiting("nv-viewer.exe", "Wait"));
    }

    /// The process's threads are listed, and a named one reads back its name.
    #[cfg(all(windows, target_arch = "x86_64"))]
    #[test]
    fn the_process_threads_are_listed_with_their_names() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
        let named = std::thread::Builder::new()
            .name("a29 probe (7)".into())
            .spawn(move || {
                tx.send(native::current_thread_id()).unwrap();
                let _ = stop_rx.recv();
            })
            .unwrap();
        let id = rx.recv().unwrap();
        let mut sampler = native::Sampler::new();
        let ids = sampler.process_threads();
        assert!(ids.contains(&id), "{ids:?}");
        assert_eq!(thread_group(&sampler.thread_name(id)), "a29 probe");
        stop_tx.send(()).unwrap();
        named.join().unwrap();
    }

    /// The native sampler reads a busy thread's instruction pointer, inside this program.
    #[cfg(all(windows, target_arch = "x86_64"))]
    #[test]
    fn a_busy_thread_is_sampled_in_this_program() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        let stop = Arc::new(AtomicBool::new(false));
        let id = Arc::new(AtomicU32::new(0));
        let (s, i) = (stop.clone(), id.clone());
        let busy = std::thread::spawn(move || {
            i.store(native::current_thread_id(), Relaxed);
            let mut n = 0u64;
            while !s.load(Relaxed) {
                n = std::hint::black_box(n.wrapping_add(1));
            }
            n
        });
        while id.load(Relaxed) == 0 {
            std::thread::yield_now();
        }
        let mut sampler = native::Sampler::new();
        let rip = sampler.instruction_pointer(id.load(Relaxed));
        stop.store(true, Relaxed);
        busy.join().unwrap();
        let rip = rip.expect("an instruction pointer");
        let (module, _) = sampler.name(rip);
        let exe = std::env::current_exe().unwrap();
        let exe = exe
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        assert_eq!(module, exe);
        let (_, symbol) = sampler.name(readable_symbol as fn(&str) -> String as usize as u64 + 1);
        // Named from the test program's PDB, found next to it.
        assert!(symbol.ends_with("readable_symbol"), "{symbol}");
        // The sampler never samples itself.
        assert_eq!(
            sampler.instruction_pointer(native::current_thread_id()),
            None
        );
    }

    /// Calls are counted by target: a target's own slot, and the next free one for a target
    /// whose slot another holds.
    #[test]
    fn calls_are_counted_by_target() {
        let a = 0x00c1_2340u32;
        let b = (a + 1..)
            .find(|&b| target_slot(b) == target_slot(a))
            .expect("an address with the same slot");
        let before = target_counts();
        for _ in 0..3 {
            count_target(a);
        }
        count_target(b);
        let uncounted = TARGET_UNCOUNTED.load(Relaxed);
        count_target(0);
        assert_eq!(TARGET_UNCOUNTED.load(Relaxed), uncounted + 1);
        let calls = target_calls_since(&target_counts(), &before);
        assert_eq!(calls.get(&a), Some(&3));
        assert_eq!(calls.get(&b), Some(&1));
        assert_eq!(calls.get(&0), None);
    }

    /// The call-target table: calls a frame, share, self time a frame and a call, inclusive.
    #[test]
    fn call_targets_read_back_their_counts() {
        let mut t = Table {
            samples: 4,
            phases: [1, 3, 0, 0],
            calls: 20_000,
            ..Default::default()
        };
        t.targets.insert(0x00b7_dab0, 10_000);
        t.targets.insert(0x7e02_0210, 5_000);
        t.selfs.insert(0x00b7_dab0, 2);
        t.inclusive.insert(0x00b7_dab0, 3);
        let text = report(
            &t,
            Duration::from_secs(1),
            10,
            Rows { top: 5, calls: 5 },
            &mut native::Sampler::new(),
        );
        assert!(
            text.contains("call targets: 2 distinct, 2000 calls a frame (0 not counted"),
            "{text}"
        );
        assert!(
            text.contains("      1000.0 50.00% 50.000 50000.0 75.000  00b7dab0\n"),
            "{text}"
        );
        assert!(
            text.contains(
                "       500.0 25.00%  0.000     0.0  0.000  7e020210  D3D9/D3DX platform object\n"
            ),
            "{text}"
        );
    }

    /// The walked stacks' tables: inclusive once a sample, self by the innermost frame, callers
    /// by the nearest frame with another name, keys of one name counted as one function.
    #[test]
    fn native_stacks_count_inclusive_self_and_callers() {
        let mut t = Table {
            samples: 10,
            walks: 5,
            walk_frames: 16,
            walk_ends: [4, 1, 0, 0],
            suspended_ns: 50_000,
            ..Default::default()
        };
        // 1: "leaf", 2: "mid", 3 and 4: "root" (two copies), 5: "other".
        t.stacks.insert(vec![1, 2, 3].into(), 2);
        t.stacks.insert(vec![1, 5].into(), 1);
        t.stacks.insert(vec![1, 1, 2, 2, 4, 3].into(), 1);
        t.stacks.insert(vec![5, 3].into(), 1);
        let name = |k: u64| match k {
            1 => "leaf",
            2 => "mid",
            3 | 4 => "root",
            _ => "other",
        };
        let mut out = String::new();
        native_stacks(&mut out, &t, 10, &|s: u64| s as f64, &mut |k: u64| {
            name(k).to_string()
        });
        assert!(
            out.contains(
                "5 walked (50.0% of all samples), 3.2 frames on average, the thread \
                 suspended 10.0 us a walk; ended at:  the thread's start 80.0%  the frame \
                 limit 20.0%"
            ),
            "{out}"
        );
        // Inclusive: root 4 (once in the stack with both copies), leaf 4, mid 3, other 2.
        assert!(out.contains("   40.0%  4.000  leaf\n"), "{out}");
        assert!(out.contains("   40.0%  4.000  root\n"), "{out}");
        assert!(out.contains("   30.0%  3.000  mid\n"), "{out}");
        assert!(out.contains("   20.0%  2.000  other\n"), "{out}");
        // leaf's callers: mid 3 (the repeated leaf frame skipped), other 1.
        let callers = out.split("by self").nth(1).expect("the callers table");
        assert!(
            callers.contains(
                "   40.0%  4.000  leaf\n         30.0%  3.000  75.0%  < mid\n         10.0%  1.000  25.0%  < other\n"
            ),
            "{out}"
        );
        assert!(
            callers.contains("   10.0%  1.000  other\n         10.0%  1.000 100.0%  < root\n"),
            "{out}"
        );
    }

    #[cfg(all(windows, target_arch = "x86_64"))]
    mod walks {
        use super::super::*;
        use std::sync::atomic::AtomicBool;

        #[link(name = "kernel32")]
        extern "system" {
            fn RtlCaptureStackBackTrace(
                skip: u32,
                count: u32,
                frames: *mut *mut std::ffi::c_void,
                hash: *mut u32,
            ) -> u16;
        }

        static STOP: AtomicBool = AtomicBool::new(false);
        static READY: AtomicBool = AtomicBool::new(false);
        static CAPTURED: Mutex<Vec<u64>> = Mutex::new(Vec::new());

        /// Where a function's code is (a jump stub's target, if the linker made one).
        fn code_of(f: usize) -> u64 {
            // SAFETY: `f` is a function's address; its first five bytes are code.
            unsafe {
                let p = f as *const u8;
                if *p == 0xe9 {
                    let rel = std::ptr::read_unaligned(p.add(1) as *const i32);
                    return (f as i64 + 5 + i64::from(rel)) as u64;
                }
            }
            f as u64
        }

        /// The bottom of the walked thread: its own backtrace taken, then a spin.
        #[inline(never)]
        fn spin_here() -> u64 {
            let mut frames = [std::ptr::null_mut(); 62];
            // SAFETY: the buffer holds the 62 frames asked for.
            let n = unsafe {
                RtlCaptureStackBackTrace(0, 62, frames.as_mut_ptr(), std::ptr::null_mut())
            };
            *CAPTURED.lock().unwrap() =
                frames[..usize::from(n)].iter().map(|&p| p as u64).collect();
            READY.store(true, Relaxed);
            let mut n = 0u64;
            while !STOP.load(Relaxed) {
                n = std::hint::black_box(n.wrapping_add(1));
            }
            n
        }

        #[inline(never)]
        fn inner_frame() -> u64 {
            std::hint::black_box(spin_here()).wrapping_add(1)
        }

        #[inline(never)]
        fn outer_frame() -> u64 {
            std::hint::black_box(inner_frame()).wrapping_add(2)
        }

        /// A busy thread's walked stack has its functions in order (keys at their starts) and
        /// the same return addresses as the thread's own `RtlCaptureStackBackTrace`, down to
        /// its start.
        #[test]
        fn a_busy_threads_stack_is_walked_to_its_start() {
            let id = std::sync::Arc::new(AtomicU32::new(0));
            let i = id.clone();
            let busy = std::thread::spawn(move || {
                i.store(native::current_thread_id(), Relaxed);
                outer_frame()
            });
            while !READY.load(Relaxed) {
                std::thread::yield_now();
            }
            let captured = CAPTURED.lock().unwrap().clone();
            let spin = code_of(spin_here as fn() -> u64 as usize);
            let inner = code_of(inner_frame as fn() -> u64 as usize);
            let outer = code_of(outer_frame as fn() -> u64 as usize);
            let mut sampler = native::Sampler::new();
            let mut last = String::new();
            let mut matched = false;
            for _ in 0..200 {
                let Some(w) = sampler.walk(id.load(Relaxed), 64) else {
                    continue;
                };
                let keys = sampler.keys(w.frames).to_vec();
                let pcs = sampler.pcs(w.frames).to_vec();
                last = format!("{w:?}\nkeys {keys:x?}\npcs {pcs:x?}\ncaptured {captured:x?}");
                let Some(at) = keys.iter().position(|&k| k == spin) else {
                    continue;
                };
                if w.end != WalkEnd::Start || keys.len() < at + 3 {
                    continue;
                }
                assert_eq!(keys[at + 1], inner, "{last}");
                assert_eq!(keys[at + 2], outer, "{last}");
                // The capture starts in `spin_here` (its return from the capture); the callers'
                // return addresses after it are the walk's frames after `spin_here`'s.
                let from = captured
                    .iter()
                    .position(|&c| c == pcs[at + 1])
                    .unwrap_or_else(|| panic!("the return into inner_frame captured: {last}"));
                assert_eq!(&pcs[at + 1..], &captured[from..], "{last}");
                // The next walks find their lookups kept: the same frames from `spin_here` on.
                let again = (0..50).find_map(|_| {
                    let w = sampler.walk(id.load(Relaxed), 64)?;
                    let keys = sampler.keys(w.frames);
                    let at = keys.iter().position(|&k| k == spin)?;
                    Some(keys[at..].to_vec())
                });
                assert_eq!(again.as_deref(), Some(&keys[at..]), "{last}");
                matched = true;
                break;
            }
            STOP.store(true, Relaxed);
            busy.join().unwrap();
            assert!(matched, "no walk reached the thread's start: {last}");
        }

        /// A thread waiting in the OS (its innermost function a system call with no unwind
        /// entry, a leaf) is walked to its start.
        #[test]
        fn a_waiting_threads_stack_is_walked_to_its_start() {
            let (id_tx, id_rx) = std::sync::mpsc::channel();
            let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
            let waiting = std::thread::spawn(move || {
                id_tx.send(native::current_thread_id()).unwrap();
                let _ = stop_rx.recv();
            });
            let id = id_rx.recv().unwrap();
            let mut sampler = native::Sampler::new();
            let mut last = String::new();
            let mut reached = false;
            for _ in 0..200 {
                std::thread::sleep(Duration::from_millis(1));
                if let Some(w) = sampler.walk(id, 64) {
                    last = format!("{w:?} {:x?}", sampler.keys(w.frames));
                    if w.end == WalkEnd::Start && w.frames >= 4 {
                        reached = true;
                        break;
                    }
                }
            }
            stop_tx.send(()).unwrap();
            waiting.join().unwrap();
            assert!(reached, "{last}");
        }
    }

    /// The shadow stack follows push, pop and resync (one test: the stack is process-wide).
    #[test]
    fn the_shadow_stack_follows_the_calls() {
        push(0x0040_1000);
        push(0x0040_2000);
        assert_eq!(
            STACK[DEPTH.load(Relaxed) as usize - 1].load(Relaxed),
            0x0040_2000
        );
        pop();
        assert_eq!(
            STACK[DEPTH.load(Relaxed) as usize - 1].load(Relaxed),
            0x0040_1000
        );
        pop();
        pop();
        assert_eq!(DEPTH.load(Relaxed), 0);
        fill(&[1, 2, 3]);
        assert_eq!(DEPTH.load(Relaxed), 3);
        assert_eq!(STACK[2].load(Relaxed), 3);
        DEPTH.store(0, Relaxed);
    }
}
