//! The world's draw on a thread of its own (step 2, piece 8: `measure/scope-1111/PLAN.md` (a)): the seam the
//! world pass and the frame call, wired at start-up to the device model's two producers
//! (`engine_platform::d3d9draw`) and the draw family's packet hand-off ([`Hooks`]). Experimental, off in every
//! preset (`NV_DRAW_THREAD`, docs/FIXES.md).
//!
//! The world pass (`00873200`) calls its two halves through [`first_half`] and [`second_half`]. With the switch on
//! for the run, at the first half's call main cuts (the packet copied and handed off, the command stream cut) and
//! posts both halves to the `nv-draw` helper (`crate::real_threads::post_draw`); the second half's call on main is
//! then skipped. Main joins the job ([`finish`]) at the next `Main::OnIdle` entry ([`idle`], before any input is
//! read), and earlier wherever the frame's tail would reach what the draw uses: the entries of the functions
//! `measure/draw-thread/NOTES.md` lists (the world accumulator's later users, a save or a load, the paths that
//! rebuild a light's object lists). On the helper and with nothing posted a join does nothing.
//!
//! **Off** (no run turned on): [`first_half`] and [`second_half`] are one relaxed load and the call they replace;
//! every join point is one relaxed load. No lock, no allocation, nothing registered.
//!
//! **Serial frames:** the draw stays on main (the plain calls) when the interface is drawn between the halves,
//! in V.A.T.S., while the navmesh is drawn, while the loading menu is up, between a save's or a load's start and the
//! next OnIdle, when [`Hooks::serial`] gives a reason (a family the draw needs not direct, a hook or a fix row on
//! the drawing set), when the helper would refuse the job (`crate::real_threads::draw_post_ready`), or when the
//! draw family cuts nothing. A frame whose post is refused after the cut draws on main in the serial order with
//! the packet it cut ([`first_half`]).

use crate::real_threads;
use crate::{args, Engine};
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// The world draw's first half (`00b6ba20`, `cdecl(camera, accumulator, third)`).
pub const WORLD_DRAW_FIRST: u32 = 0x00b6_ba20;
/// The world draw's second half (`00b6b930`, the same words).
pub const WORLD_DRAW_SECOND: u32 = 0x00b6_b930;

/// The VATS singleton (`011f2250`) and its mode word (+8, what `VATS::GetMode` `0044ddc0` answers): 0 outside
/// V.A.T.S.
const VATS: u32 = 0x011f_2250;
const VATS_MODE: u32 = 8;
/// The navmesh drawing object (`011d6e2c`, what `00552ba0` answers) and its drawing flag (+0x210, what
/// `NavMeshRender::IsDrawing` `00552bb0` answers; `ToggleNavMesh` `005bcfe0` sets it).
const NAVMESH_RENDER: u32 = 0x011d_6e2c;
const NAVMESH_DRAWING: u32 = 0x210;
/// `LoadingMenu::pMe` (`011da0c0`): the loading menu is up.
const LOADING_MENU: u32 = 0x011d_a0c0;
/// The renderer (`011f4748`, `NiDX9Renderer`) and its Direct3D 9 device (+0x288).
const RENDERER: u32 = 0x011f_4748;
const RENDERER_DEVICE: u32 = 0x288;

/// What the draw family hands from main's cut to the thread that draws, and back (`engine_typed::draw::capture`'s
/// detached packet).
pub type Packet = Box<dyn Any + Send>;

/// What the seam calls outside this crate, set when the run turns the thread on ([`turn_on`]).
#[derive(Clone, Copy)]
pub struct Hooks {
    /// A reason this frame draws on main, from what this crate cannot see (the typed families, the hooks and
    /// fix rows on the drawing set, POISON with real threads), or `None`.
    pub serial: fn(&Engine) -> Option<&'static str>,
    /// Main, at the cut: the packet filled and moved out of main's thread (`cut::cut_for_thread`; `stepped`:
    /// the run's threads are stepped). `None`: nothing to hand off; the frame draws on main.
    pub cut: fn(&mut Engine, camera: u32, accumulator: u32, stepped: bool) -> Option<Packet>,
    /// The thread that draws, before the halves: the packet installed there (`cut::adopt`). `Err` gives it back
    /// when that thread holds one already.
    pub adopt: fn(&mut Engine, Packet) -> Result<(), Packet>,
    /// The thread that drew, after the halves: the packet taken back (`cut::hand_back`).
    pub hand_back: fn(&mut Engine) -> Option<Packet>,
    /// Main, at the join: the packet closed there (`cut::join_handed`: the own state's merge, the holds given back).
    pub join: fn(&mut Engine, Packet),
    /// Main, at the cut: the device's cut (`d3d9draw::draw_cut`). False: no cut; the frame draws on main.
    pub device_cut: fn(&mut Engine, device: u32) -> bool,
    /// The drawing side, at its job's end: its commands complete (`d3d9draw::draw_close`).
    pub device_close: fn(&mut Engine),
    /// Main, at a join: the job joined and the device's join (`d3d9draw::draw_finish`; a panic of the job's comes
    /// back from it).
    pub device_finish: fn(&mut Engine),
}

/// Where the packet goes between main and the job: in before the post, back at the job's end.
#[derive(Default)]
struct Slot {
    packet: Option<Packet>,
    back: Option<Packet>,
}

type SharedSlot = Arc<Mutex<Slot>>;

/// A run that turned the thread on, by its kernel's id (`crate::units::platform::KernelRef::id`: the same in every
/// real thread's engine of the run).
struct Run {
    id: usize,
    hooks: Hooks,
    /// The posted job's slot, until the join.
    pending: Option<SharedSlot>,
    /// The job (or a draw on main with a handed-off packet) took the second half: its call on main is skipped.
    second_taken: bool,
    /// A save or a load started: serial until the next OnIdle.
    save_or_load: bool,
    /// The reasons already noted in the log (each once a run).
    noted: Vec<&'static str>,
    /// What happened, for the tests: posts, frames drawn on main with a handed-off packet, joins.
    counts: Counts,
}

/// A run's counts ([`counts`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Jobs posted to the helper.
    pub posted: u64,
    /// Cuts drawn on main after all (the device cut or the post refused).
    pub on_main: u64,
    /// Frames drawn serially (a serial condition, or nothing cut).
    pub serial: u64,
    /// Joins of a posted job.
    pub joined: u64,
}

static RUNS: Mutex<Vec<Run>> = Mutex::new(Vec::new());
/// How many runs are on: the off path's one test.
static ON: AtomicUsize = AtomicUsize::new(0);
/// How many runs have a job posted and not joined: the join points' one test.
static POSTED: AtomicUsize = AtomicUsize::new(0);

fn runs() -> MutexGuard<'static, Vec<Run>> {
    RUNS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn with_run<R>(id: usize, f: impl FnOnce(&mut Run) -> R) -> Option<R> {
    runs().iter_mut().find(|r| r.id == id).map(f)
}

/// Turns the drawing thread on for `e`'s run, with `hooks` (the host's fixes row, before the first frame). A second
/// call replaces the hooks.
pub fn turn_on(e: &Engine, hooks: Hooks) {
    let id = e.kernel.id();
    let mut runs = runs();
    if let Some(run) = runs.iter_mut().find(|r| r.id == id) {
        run.hooks = hooks;
        return;
    }
    runs.push(Run {
        id,
        hooks,
        pending: None,
        second_taken: false,
        save_or_load: false,
        noted: Vec::new(),
        counts: Counts::default(),
    });
    ON.fetch_add(1, Relaxed);
}

/// Turns it off for `e`'s run (tests; a run's end). A job still posted is joined first.
pub fn turn_off(e: &mut Engine) {
    finish(e);
    let id = e.kernel.id();
    let mut runs = runs();
    let before = runs.len();
    runs.retain(|r| r.id != id);
    if runs.len() != before {
        ON.fetch_sub(1, Relaxed);
    }
}

/// Whether the drawing thread is on for `e`'s run.
pub fn is_on(e: &Engine) -> bool {
    ON.load(Relaxed) != 0 && with_run(e.kernel.id(), |_| ()).is_some()
}

/// The run's counts (zero when it is off).
pub fn counts(e: &Engine) -> Counts {
    with_run(e.kernel.id(), |r| r.counts).unwrap_or_default()
}

/// Whether `e`'s run has a job posted and not joined.
pub fn posted(e: &Engine) -> bool {
    POSTED.load(Relaxed) != 0 && with_run(e.kernel.id(), |r| r.pending.is_some()).unwrap_or(false)
}

/// The world pass's call of the first half, `00b6ba20(camera, accumulator, third)`; `overlay` is the world pass's
/// own word that draws the interface between the halves. Off: that call.
#[inline]
pub fn first_half(e: &mut Engine, camera: u32, accumulator: u32, third: u32, overlay: u8) {
    if ON.load(Relaxed) == 0 {
        e.call(WORLD_DRAW_FIRST, &args![camera, accumulator, third]);
        return;
    }
    first_half_on(e, camera, accumulator, third, overlay);
}

/// The world pass's call of the second half, `00b6b930(camera, accumulator, third)`: skipped when the posted job
/// (or a draw on main with a handed-off packet) took it. Off: that call.
#[inline]
pub fn second_half(e: &mut Engine, camera: u32, accumulator: u32, third: u32) {
    if ON.load(Relaxed) != 0 {
        let taken = with_run(e.kernel.id(), |r| std::mem::take(&mut r.second_taken));
        if taken == Some(true) {
            return;
        }
    }
    e.call(WORLD_DRAW_SECOND, &args![camera, accumulator, third]);
}

/// Main's wait for the drawing job at a join point: the job joined, the device joined, the packet closed on main.
/// Nothing on the drawing helper, or when the run has no job posted. A panic of the job's comes back here, after
/// the device's join.
#[inline]
pub fn finish(e: &mut Engine) {
    if POSTED.load(Relaxed) == 0 {
        return;
    }
    finish_posted(e);
}

/// `Main::OnIdle`'s entry (`0086e650`): [`finish`], and a save's or a load's serial frames end.
#[inline]
pub fn idle(e: &mut Engine) {
    if ON.load(Relaxed) == 0 {
        return;
    }
    finish(e);
    with_run(e.kernel.id(), |r| r.save_or_load = false);
}

/// A save's or a load's entry: [`finish`], and the world pass draws on main until the next OnIdle (a save's
/// screenshot and a load's frames run the draw sequence inside it).
#[inline]
pub fn save_or_load(e: &mut Engine) {
    if ON.load(Relaxed) == 0 {
        return;
    }
    finish(e);
    with_run(e.kernel.id(), |r| r.save_or_load = true);
}

#[cold]
#[inline(never)]
fn finish_posted(e: &mut Engine) {
    if real_threads::on_draw_helper() {
        return;
    }
    let id = e.kernel.id();
    let Some((slot, hooks)) = with_run(id, |r| {
        let slot = r.pending.take()?;
        r.counts.joined += 1;
        Some((slot, r.hooks))
    })
    .flatten() else {
        return;
    };
    POSTED.fetch_sub(1, Relaxed);
    let joined =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (hooks.device_finish)(e)));
    let back = lock_slot(&slot).back.take();
    match joined {
        Ok(()) => {
            if let Some(packet) = back {
                (hooks.join)(e, packet);
            }
        }
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn lock_slot(slot: &SharedSlot) -> MutexGuard<'_, Slot> {
    slot.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The word at `addr`, 0 where nothing is mapped (the serial tests read exe globals a test world may not map).
fn peek_u32(e: &Engine, addr: u32) -> u32 {
    let mut b = [0u8; 4];
    e.mem
        .try_read(addr, &mut b)
        .map_or(0, |()| u32::from_le_bytes(b))
}

fn peek_u8(e: &Engine, addr: u32) -> u8 {
    let mut b = [0u8; 1];
    e.mem.try_read(addr, &mut b).map_or(0, |()| b[0])
}

/// Why this frame's world draw stays on main, from the game's state, or `None`.
fn serial_reason(e: &Engine, run_save_or_load: bool, overlay: u8) -> Option<&'static str> {
    if overlay != 0 {
        return Some("the interface is drawn between the world draw's halves");
    }
    if peek_u32(e, VATS + VATS_MODE) != 0 {
        return Some("V.A.T.S.");
    }
    let navmesh = peek_u32(e, NAVMESH_RENDER);
    if navmesh != 0 && peek_u8(e, navmesh.wrapping_add(NAVMESH_DRAWING)) != 0 {
        return Some("the navmesh is drawn");
    }
    if peek_u32(e, LOADING_MENU) != 0 {
        return Some("the loading menu is up");
    }
    if run_save_or_load {
        return Some("a save or a load is in progress");
    }
    None
}

/// The run's device (`[[011f4748]+0x288]`).
fn device(e: &Engine) -> u32 {
    let renderer = peek_u32(e, RENDERER);
    if renderer == 0 {
        return 0;
    }
    peek_u32(e, renderer.wrapping_add(RENDERER_DEVICE))
}

/// Notes `reason` in the log once a run, and counts a serial frame.
fn serial(
    e: &mut Engine,
    id: usize,
    reason: Option<&'static str>,
    camera: u32,
    accumulator: u32,
    third: u32,
) {
    with_run(id, |r| {
        r.counts.serial += 1;
        if let Some(reason) = reason {
            if !r.noted.contains(&reason) {
                r.noted.push(reason);
                eprintln!(
                    "NV_DRAW_THREAD: the world drawn on main (first time this run): {reason}"
                );
            }
        }
    });
    e.call(WORLD_DRAW_FIRST, &args![camera, accumulator, third]);
}

#[cold]
#[inline(never)]
fn first_half_on(e: &mut Engine, camera: u32, accumulator: u32, third: u32, overlay: u8) {
    let id = e.kernel.id();
    let Some((hooks, save_or_load)) = with_run(id, |r| (r.hooks, r.save_or_load)) else {
        e.call(WORLD_DRAW_FIRST, &args![camera, accumulator, third]);
        return;
    };
    if real_threads::on_draw_helper() {
        e.call(WORLD_DRAW_FIRST, &args![camera, accumulator, third]);
        return;
    }
    // A job still posted (a world pass twice in one frame): joined first, so one cut is open at a time.
    finish(e);
    if let Some(reason) = serial_reason(e, save_or_load, overlay).or_else(|| (hooks.serial)(e)) {
        return serial(e, id, Some(reason), camera, accumulator, third);
    }
    if !real_threads::draw_post_ready(e) {
        return serial(
            e,
            id,
            Some("the drawing helper takes no job now"),
            camera,
            accumulator,
            third,
        );
    }
    let stepped = e.real.is_none();
    let Some(packet) = (hooks.cut)(e, camera, accumulator, stepped) else {
        return serial(e, id, None, camera, accumulator, third);
    };
    let device = device(e);
    if !(hooks.device_cut)(e, device) {
        return draw_on_main(e, id, hooks, packet, camera, accumulator, third);
    }
    let slot: SharedSlot = Arc::new(Mutex::new(Slot {
        packet: Some(packet),
        back: None,
    }));
    let job_slot = slot.clone();
    let job: real_threads::DrawJob = Box::new(move |w: &mut Engine| {
        let Some(packet) = lock_slot(&job_slot).packet.take() else {
            (hooks.device_close)(w);
            return;
        };
        if (hooks.adopt)(w, packet).is_err() {
            (hooks.device_close)(w);
            panic!("NV_DRAW_THREAD: the drawing helper already held a packet at a new job");
        }
        let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            w.call(WORLD_DRAW_FIRST, &args![camera, accumulator, third]);
            w.call(WORLD_DRAW_SECOND, &args![camera, accumulator, third]);
        }));
        let back = (hooks.hand_back)(w);
        lock_slot(&job_slot).back = back;
        (hooks.device_close)(w);
        if let Err(payload) = drawn {
            std::panic::resume_unwind(payload);
        }
    });
    if !real_threads::post_draw(e, job) {
        // Refused after the cut: the device's cut closed with nothing in its slot, and the halves drawn on main in
        // the serial order, with the packet that was cut.
        (hooks.device_finish)(e);
        let packet = lock_slot(&slot)
            .packet
            .take()
            .expect("a refused job never ran");
        return draw_on_main(e, id, hooks, packet, camera, accumulator, third);
    }
    with_run(id, |r| {
        r.pending = Some(slot);
        r.second_taken = true;
        r.counts.posted += 1;
    });
    POSTED.fetch_add(1, Relaxed);
}

/// Both halves on main with a packet main cut for a thread (the device's cut or the post refused): adopted here,
/// drawn, handed back and joined at once, in the serial order; the second half's call is then skipped.
fn draw_on_main(
    e: &mut Engine,
    id: usize,
    hooks: Hooks,
    packet: Packet,
    camera: u32,
    accumulator: u32,
    third: u32,
) {
    if (hooks.adopt)(e, packet).is_err() {
        panic!("NV_DRAW_THREAD: main already held a packet at a cut");
    }
    e.call(WORLD_DRAW_FIRST, &args![camera, accumulator, third]);
    e.call(WORLD_DRAW_SECOND, &args![camera, accumulator, third]);
    if let Some(back) = (hooks.hand_back)(e) {
        (hooks.join)(e, back);
    }
    with_run(id, |r| {
        r.second_taken = true;
        r.counts.on_main += 1;
    });
}

#[doc(hidden)]
pub mod testing;

#[cfg(test)]
mod tests;
