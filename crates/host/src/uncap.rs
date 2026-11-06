//! The frame-timing fixes of the `fixes` preset ([`crate::fixes`], docs/FIXES.md): **not 1:1 with
//! FalloutNV.exe**, and in the `vanilla` preset nothing here runs.
//!
//! **The uncapped frame rate (`NV_UNCAP`).** New Vegas Tick Fix (carxt, lStewieAl, jazzisparis) showed the
//! way; this is its idea in nv-rs's own code ([docs/FIXES.md](../../../docs/FIXES.md) has the credits).
//!
//! The game paces its frames with `BSTimer`'s step (`00aa4ee0`, engine-bs `bscore/bstimer.rs`):
//! for a real-time timer (`fClamp` 0) a step that comes less than 10 ms (by `GetTickCount`)
//! after the timer's last one sleeps the difference, and every step reports at least 10 ms.
//! In game three timers step each frame (Main::OnIdle's through `00771520`, `HUDMainMenu::
//! DoIdle`'s and `MessageMenu::DoIdle`'s), so with W the frame's other work the frame takes
//! (W + 30) / 4 ms: at most 133 frames a second however fast the rest is (a29's profile,
//! 2026-10-28). The original game is held the same way.
//!
//! With the switch, the step at `00aa4ee0` is replaced (registered over the translation by
//! [`install`], after the play-loop patches): a real-time step neither sleeps nor rounds up to
//! 10 ms. It takes the time since that timer's own last step from the platform's microsecond
//! clock (what `QueryPerformanceCounter` answers), not from `GetTickCount`'s 15.625 ms ticks,
//! so the game's time keeps pace with the real time however short the frames are. The rest is
//! the translation's: a disabled timer and a fixed step (`fClamp`) run the translation itself;
//! the step is at most 166 ms; `iLastTime` moves on as before; `fDeltaUnmodified` and the global
//! time multiplier as before. A timer's first step (or its first after being disabled or
//! clamped) takes the tick count's milliseconds, as the translation does, without the sleep.
//!
//! The game was made for steps of 10 ms and more; shorter ones are what mods like NVTF give it.
//!
//! **The high-frame-rate landing (`NV_HF_LANDING`).** A character's contact list reaches about a
//! Havok unit (7 game units) below it. The first frame with the floor in it, the controller says
//! supported and goes to the ground state, and that frame moves it once more at its falling speed;
//! the next frame, standing still on the ground, it has no vertical speed and its contact is
//! unchanged, so `Move` stops integrating it for good. At 16 ms frames a falling character covers
//! those units in a frame or two, so it ends within about a unit of the floor; at 5 ms it stays
//! 7 units up (measured, docs/FIXES.md). The override runs `00c73170` and, on such a landing in a frame
//! under 16 ms, integrates once more for the rest of two 16 ms frames. Other frames run `Move` as
//! it is, and the world's own Havok steps are not touched: they are 16 ms at any frame rate.
//!

use crate::fixes::Fix;
use engine::prelude::*;
use engine::units::bscore::bstimer::{fn_00aa4e40, fn_00aa4ee0, BSTimer};
use engine::units::bshavok::bhkcharactercontroller::{
    bhk_character_controller_move, bhk_character_proxy_integrate,
    BhkCharacterController as Controller,
};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// `BSTimer`'s step (the translation: engine-bs `bstimer.rs`).
pub const BS_TIMER_STEP: u32 = 0x00aa_4ee0;
/// `Sleep` (KERNEL32 import slot).
const SLEEP_IMPORT: u32 = 0x00fd_f104;
/// The main timer (`TimeGlobal`), the one `Main::OnIdle` steps at `0086f296`.
const MAIN_TIMER: u32 = 0x011f_6394;
/// `0.0` (`double`), the value the translation compares `fClamp` with.
const ZERO: u32 = 0x0101_2060;
/// `BSTimer::fGlobalTimeMultiplier` (Xbox PDB static `float`).
const GLOBAL_TIME_MULTIPLIER: u32 = 0x011a_c3a0;
/// The longest step the translation takes (`0xa6` ms), in microseconds.
const LONGEST_STEP_US: u64 = 166_000;

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "uncapped frame rate",
    env: "NV_UNCAP",
    addresses: &[BS_TIMER_STEP],
    summary: "BSTimer's step without its sleep, timed on the microsecond clock",
    install,
};

/// Registers the uncapped step over `00aa4ee0`.
pub fn install(e: &mut Engine) {
    e.register(BS_TIMER_STEP, uncapped_entry);
}

fn uncapped_entry(e: &mut Engine, a: &[u32]) -> Ret {
    uncapped_step(e, Ptr::new(a[0]), a[1]);
    Ret::default()
}

/// Frames a second at most (0: no limit), bound once before the first frame
/// ([`crate::fixes::install`]). A run has one host.
static FRAME_LIMIT: AtomicU32 = AtomicU32::new(0);
/// When the next frame may start, on the microsecond clock.
static NEXT_FRAME_US: AtomicU64 = AtomicU64::new(0);

/// Binds the frame limit.
pub fn set_frame_limit(frames_a_second: u32) {
    FRAME_LIMIT.store(frames_a_second, Ordering::Relaxed);
}

/// The player's frame limit, at the main timer's step: gives the other game threads their turn
/// first (`Sleep(0)`), since the wait that follows is theirs to use (the original's threads run
/// side by side), then waits for the frame's start time. That time moves by whole frames, so a
/// late wake-up doesn't lower the rate.
fn wait_for_frame_limit(e: &mut Engine, limit: u32) {
    let frame_us = 1_000_000 / u64::from(limit);
    e.call(SLEEP_IMPORT, &args![0u32]);
    let now_us = e.clock_now_us();
    let next = NEXT_FRAME_US.load(Ordering::Relaxed);
    if next > now_us {
        e.wait_clock_until_us(next);
    }
    // More than a whole frame late starts the count again, without catching up.
    let start = if next + frame_us < now_us {
        now_us
    } else {
        next
    };
    NEXT_FRAME_US.store(start + frame_us, Ordering::Relaxed);
}

/// A timer's last real-time step on the microsecond clock (0: none), in the five padding bytes of
/// its guest object (`+1` to `+3`, `+0x1e`, `+0x1f`), which no exe code reads: the state stays with the
/// timer and the engine it belongs to. 40 bits last 12 days; a wrap costs one step at the longest.
fn last_step_us(e: &Engine, this: Ptr<BSTimer>) -> u64 {
    let at = this.addr();
    [1, 2, 3, 0x1e, 0x1f]
        .iter()
        .rev()
        .fold(0, |word, &o| (word << 8) | u64::from(e.mem.u8(at + o)))
}

fn set_last_step_us(e: &mut Engine, this: Ptr<BSTimer>, us: u64) {
    let at = this.addr();
    let mut word = us;
    for o in [1, 2, 3, 0x1e, 0x1f] {
        // The low byte of what's left.
        e.mem.set_u8(at + o, word.to_le_bytes()[0]);
        word >>= 8;
    }
}

/// `00aa4ee0` without the sleep (module documentation).
pub fn uncapped_step(e: &mut Engine, this: Ptr<BSTimer>, now: u32) {
    let zero: f64 = e.global(ZERO);
    let real_time = e.get(this, BSTimer::cDisableCounter) == 0
        && f64::from(e.get(this, BSTimer::fClamp)) == zero;
    if !real_time {
        set_last_step_us(e, this, 0);
        fn_00aa4ee0(e, this, now);
        return;
    }
    let limit = FRAME_LIMIT.load(Ordering::Relaxed);
    if limit != 0 && this.addr() == MAIN_TIMER {
        wait_for_frame_limit(e, limit);
    }
    let now_us = e.clock_now_us();
    let time = now.wrapping_sub(e.get(this, BSTimer::iFirstTime));
    let elapsed_ms = time.wrapping_sub(e.get(this, BSTimer::iLastTime));
    let before = last_step_us(e, this);
    // Never 0 when kept, which would read as "no step yet".
    set_last_step_us(e, this, now_us.max(1));
    let elapsed_us = if before == 0 {
        u64::from(elapsed_ms) * 1000
    } else {
        now_us.saturating_sub(before)
    }
    .min(LONGEST_STEP_US);
    e.set(this, BSTimer::bZeroDelta, elapsed_us == 0);
    e.set(this, BSTimer::fDelta, (elapsed_us as f64 / 1e6) as f32);
    e.set(this, BSTimer::iLastTime, time);
    // From here as the translation.
    let delta = e.get(this, BSTimer::fDelta);
    e.set(
        this,
        BSTimer::fDeltaUnmodified,
        f32::from_bits(x87_quiet_bits(delta.to_bits())),
    );
    let multiplier: u32 = e.global(GLOBAL_TIME_MULTIPLIER);
    let scaled = x87_mul(x87_load32(delta.to_bits()), x87_m32(multiplier));
    e.set(this, BSTimer::fDelta, f32::from_bits(x87_store32(scaled)));
}

/// The row in [`crate::fixes::FIXES`].
pub const PHYSICS_FIX: Fix = Fix {
    name: "high-frame-rate landing",
    env: "NV_HF_LANDING",
    addresses: &[CHARACTER_MOVE],
    summary: "a character that lands standing still falls on as far as at 16 ms frames",
    install: install_landing,
};

/// `bhkCharacterController::Move`.
const CHARACTER_MOVE: u32 = 0x00c7_3170;
/// `hkpCharacterProxy::setLinearVelocity`.
const SET_LINEAR_VELOCITY: u32 = 0x00ca_c960;
/// The controller's states (`hkpCharacterContext::m_currentState`).
const STATE_ON_GROUND: u32 = 0;
const STATE_IN_AIR: u32 = 2;
/// `iFlags` bit: the controller was asked to move this step.
const FLAG_MOVING: u32 = 8;
/// The frame the game's character code settles to: longer frames land as they always did.
const SETTLED_FRAME_S: f32 = 0.016;

/// Registers the landing over `00c73170`.
pub fn install_landing(e: &mut Engine) {
    e.register(CHARACTER_MOVE, landing_entry);
}

fn landing_entry(e: &mut Engine, a: &[u32]) -> Ret {
    landing_move(e, Ptr::new(a[0]), Ptr::new(a[1])).into_ret()
}

/// How much longer a landing character falls, in seconds: the rest of two 16 ms frames, when it
/// was in the air falling, is now on the ground, wasn't asked to move and this frame was short.
fn landing_time(step: f32, before: u32, after: u32, moving: bool, fall_speed: f32) -> Option<f32> {
    let lands = before == STATE_IN_AIR && after == STATE_ON_GROUND && !moving && fall_speed < 0.0;
    (lands && step > 0.0 && step < SETTLED_FRAME_S).then_some(2.0 * SETTLED_FRAME_S - step)
}

/// `00c73170`, then the landing (module documentation).
fn landing_move(e: &mut Engine, this: Ptr<Controller>, params: Ptr) -> u32 {
    let step = e.mem.f32(params.addr());
    let before = e.get(this, Controller::m_currentState);
    let velocity_at = this.addr() + Controller::OutVelocity.off;
    let fall_speed = e.mem.f32(velocity_at + 8);
    let result = bhk_character_controller_move(e, this, params);
    let after = e.get(this, Controller::m_currentState);
    let moving = e.get(this, Controller::iFlags) & FLAG_MOVING != 0;
    if let Some(time) = landing_time(step, before, after, moving, fall_speed) {
        fall_on(e, this, fall_speed, time);
    }
    result
}

/// One more integration of the character's proxy, straight down at `speed` for `time` seconds. It
/// stops at the surface as any other does. The velocity and the step are left as `Move` left them.
fn fall_on(e: &mut Engine, this: Ptr<Controller>, speed: f32, time: f32) {
    let me = this.addr();
    let proxy = e.get(this, Controller::phkObject);
    if proxy == 0 {
        return;
    }
    let velocity_at = me + Controller::OutVelocity.off;
    let kept = [0, 4, 8, 12].map(|o| e.mem.u32(velocity_at + o));
    let step = e.get(this, Controller::StepDeltaTime);
    let inverse = e.get(this, Controller::StepInvDeltaTime);
    let set = |e: &mut Engine, z: u32| {
        e.mem.set_u32(velocity_at, 0);
        e.mem.set_u32(velocity_at + 4, 0);
        e.mem.set_u32(velocity_at + 8, z);
        e.mem.set_u32(velocity_at + 12, 0);
        e.call(SET_LINEAR_VELOCITY, &args![proxy, velocity_at]);
    };
    set(e, speed.to_bits());
    e.set(this, Controller::StepDeltaTime, time);
    e.set(this, Controller::StepInvDeltaTime, 1.0 / time);
    bhk_character_proxy_integrate(e, this.cast(), me + Controller::StepInfo.off);
    e.set(this, Controller::StepDeltaTime, step);
    e.set(this, Controller::StepInvDeltaTime, inverse);
    set(e, 0);
    for (o, word) in [0, 4, 8, 12].into_iter().zip(kept) {
        e.mem.set_u32(velocity_at + o, word);
    }
}

/// The row in [`crate::fixes::FIXES`].
pub const SLOW_MOTION_FIX: Fix = Fix {
    name: "high-frame-rate slow motion",
    env: "NV_HF_SLOWMO",
    addresses: &[TIME_MULTIPLIER_RAMP],
    summary: "the time multiplier moves to its target by the time passed, not by the frame",
    install: install_slow_motion,
};

/// `00aa4e40`: moves `fGlobalTimeMultiplier` towards its target once a frame.
const TIME_MULTIPLIER_RAMP: u32 = 0x00aa_4e40;
/// `fGlobalTimeMultiplierTarget` and the rate of the move.
const TIME_MULTIPLIER_TARGET: u32 = 0x011a_c3a4;
const TIME_MULTIPLIER_RATE: u32 = 0x011a_c3ac;

/// Registers the ramp over `00aa4e40`.
pub fn install_slow_motion(e: &mut Engine) {
    e.register(TIME_MULTIPLIER_RAMP, ramp_entry);
}

fn ramp_entry(e: &mut Engine, _: &[u32]) -> Ret {
    time_multiplier_ramp(e);
    Ret::default()
}

/// `00aa4e40` with the rate of a 16 ms frame: in the last frame's share of 16 ms the multiplier
/// moves as far as the exe moves it in a whole one, so V.A.T.S.'s slow motion comes on in the same
/// time at any frame rate. Frames of 16 ms and more run the translation.
fn time_multiplier_ramp(e: &mut Engine) {
    let last_step = e.mem.f32(MAIN_TIMER + BSTimer::fDeltaUnmodified.off);
    if !(last_step > 0.0 && last_step < SETTLED_FRAME_S) {
        fn_00aa4e40(e);
        return;
    }
    let share = f64::from(last_step) / f64::from(SETTLED_FRAME_S);
    let rate = f64::from(e.mem.f32(TIME_MULTIPLIER_RATE));
    let target = e.mem.f32(TIME_MULTIPLIER_TARGET);
    let multiplier = e.mem.f32(GLOBAL_TIME_MULTIPLIER);
    let moved = if target > multiplier {
        ((f64::from(multiplier) * (1.0 + rate).powf(share)) as f32).min(target)
    } else if target < multiplier {
        ((f64::from(multiplier) * (1.0 - rate).powf(share)) as f32).max(target)
    } else {
        return;
    };
    e.mem.set_f32(GLOBAL_TIME_MULTIPLIER, moved);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLEEP: u32 = 0x00fd_f104;
    const MILLISECONDS_TO_SECONDS: u32 = 0x0101_6978;

    /// An engine with the globals the step reads, a `Sleep` that moves the clock and logs its
    /// time, and a real-time timer whose steps start at clock 1000 ms.
    fn timer_engine() -> (
        Engine,
        Ptr<BSTimer>,
        std::rc::Rc<std::cell::RefCell<Vec<u32>>>,
    ) {
        let mut e = Engine::new();
        e.map(ZERO, 8);
        e.map(MILLISECONDS_TO_SECONDS, 8);
        e.mem.set_u64(MILLISECONDS_TO_SECONDS, 0.001f64.to_bits());
        e.map(GLOBAL_TIME_MULTIPLIER, 4);
        e.mem.set_f32(GLOBAL_TIME_MULTIPLIER, 1.0);
        let slept = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = slept.clone();
        e.register_double(SLEEP, move |e, a| {
            log.borrow_mut().push(a[0]);
            e.advance_clock_us(u64::from(a[0]) * 1000);
            Ret::default()
        });
        let timer = e.new_object::<BSTimer>();
        e.clock_ms = 1000;
        e.set(timer, BSTimer::iFirstTime, 0);
        e.set(timer, BSTimer::iLastTime, 1000);
        (e, timer, slept)
    }

    /// The translation lifts steps 4 ms apart to 10 ms by sleeping and reports 10 ms; the uncapped step
    /// neither sleeps nor rounds up: 4 ms each (the first from the tick count, then the clock).
    #[test]
    fn the_uncapped_step_does_not_sleep_and_reports_the_real_time() {
        let (mut e, timer, slept) = timer_engine();
        for _ in 0..3 {
            e.advance_clock_us(4000);
            let now = e.clock_ms;
            e.call(BS_TIMER_STEP, &[timer.addr(), now]);
            assert_eq!(e.get(timer, BSTimer::fDelta), 0.01);
        }
        // The second step's 10 ms include the first one's sleep.
        assert_eq!(*slept.borrow(), [6, 6]);

        let (mut e, timer, slept) = timer_engine();
        install(&mut e);
        for _ in 0..3 {
            e.advance_clock_us(4000);
            let now = e.clock_ms;
            e.call(BS_TIMER_STEP, &[timer.addr(), now]);
            assert_eq!(e.get(timer, BSTimer::fDelta), 0.004);
            assert_eq!(e.get(timer, BSTimer::fDeltaUnmodified), 0.004);
            assert_eq!(e.get(timer, BSTimer::iLastTime), now);
            assert!(!e.get(timer, BSTimer::bZeroDelta));
        }
        assert!(slept.borrow().is_empty());
        // Under a millisecond: the microsecond clock still sees it.
        e.advance_clock_us(250);
        let now = e.clock_ms;
        e.call(BS_TIMER_STEP, &[timer.addr(), now]);
        assert_eq!(e.get(timer, BSTimer::fDelta), 0.00025);
        // A long pause is cut to 166 ms, as the translation cuts it.
        e.advance_clock_us(2_000_000);
        let now = e.clock_ms;
        e.call(BS_TIMER_STEP, &[timer.addr(), now]);
        assert_eq!(e.get(timer, BSTimer::fDelta), 0.166);
    }

    /// A disabled timer and a fixed step run the translation itself.
    #[test]
    fn disabled_and_fixed_step_timers_run_the_translation() {
        let (mut e, timer, slept) = timer_engine();
        install(&mut e);
        e.set(timer, BSTimer::cDisableCounter, 1);
        e.advance_clock_us(4000);
        e.call(BS_TIMER_STEP, &[timer.addr(), e.clock_ms]);
        assert_eq!(e.get(timer, BSTimer::fDelta), 0.0);
        e.set(timer, BSTimer::cDisableCounter, 0);
        e.set(timer, BSTimer::fClamp, 16.0);
        e.call(BS_TIMER_STEP, &[timer.addr(), e.clock_ms]);
        assert_eq!(e.get(timer, BSTimer::fDelta), 0.016);
        assert!(slept.borrow().is_empty());
    }

    /// The padding bytes hold the clock whole, and no field moves.
    #[test]
    fn the_last_step_lives_in_padding() {
        let (mut e, timer, _) = timer_engine();
        e.set(timer, BSTimer::fClamp, 2.5);
        e.set(timer, BSTimer::bZeroDelta, true);
        let us = 0x12_3456_7890;
        set_last_step_us(&mut e, timer, us);
        assert_eq!(last_step_us(&e, timer), us);
        assert_eq!(e.get(timer, BSTimer::fClamp), 2.5);
        assert!(e.get(timer, BSTimer::bZeroDelta));
        assert_eq!(e.get(timer, BSTimer::cDisableCounter), 0);
    }

    /// The limit holds whole frames at the main timer only, gives the other threads their turn
    /// first, and doesn't catch up after a long frame.
    #[test]
    fn the_frame_limit_paces_the_main_timer() {
        let (mut e, _, slept) = timer_engine();
        e.map(MAIN_TIMER & !0xfff, 0x1000);
        let main: Ptr<BSTimer> = Ptr::new(MAIN_TIMER);
        e.set(main, BSTimer::iLastTime, 1000);
        NEXT_FRAME_US.store(0, Ordering::Relaxed);
        let mut starts = Vec::new();
        for work_us in [2000, 2000, 2000, 35_000, 2000] {
            e.advance_clock_us(work_us);
            wait_for_frame_limit(&mut e, 100);
            starts.push(e.clock_now_us());
            let now = e.clock_ms;
            uncapped_step(&mut e, main, now);
        }
        assert_eq!(starts[1] - starts[0], 10_000);
        assert_eq!(starts[2] - starts[1], 10_000);
        assert_eq!(starts[3] - starts[2], 35_000);
        assert_eq!(starts[4] - starts[3], 10_000);
        assert_eq!(e.get(main, BSTimer::fDeltaUnmodified), 0.01);
        assert_eq!(slept.borrow().len(), 5);
        assert!(slept.borrow().iter().all(|&ms| ms == 0));
    }

    #[test]
    fn only_a_short_standing_landing_falls_on() {
        let ground = STATE_ON_GROUND;
        let air = STATE_IN_AIR;
        // A 5 ms frame: the rest of two 16 ms frames.
        let rest = landing_time(0.005, air, ground, false, -64.0).unwrap();
        assert!((rest - 0.027).abs() < 1e-6, "{rest}");
        // 16 ms frames and longer land as they always did.
        assert_eq!(landing_time(0.016, air, ground, false, -64.0), None);
        assert_eq!(landing_time(0.031, air, ground, false, -64.0), None);
        // Asked to move: the ground state's own velocity takes it down.
        assert_eq!(landing_time(0.005, air, ground, true, -64.0), None);
        // Going up, already on the ground, still in the air: nothing.
        assert_eq!(landing_time(0.005, air, ground, false, 3.0), None);
        assert_eq!(landing_time(0.005, ground, ground, false, -64.0), None);
        assert_eq!(landing_time(0.005, air, air, false, -64.0), None);
        // A controller that didn't step.
        assert_eq!(landing_time(0.0, air, ground, false, -64.0), None);
    }

    /// One more integration straight down for the time, with `Move`'s velocity and step put back
    /// for whoever reads them, and the proxy stopped.
    #[test]
    fn falling_on_integrates_once_and_leaves_the_step() {
        const HALF: u32 = 0x0900_0094;
        let mut e = Engine::new();
        let this: Ptr<Controller> = e.new_object();
        let me = this.addr();
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        e.put_vtable(0x0900_0000, &[HALF; 0x26]);
        e.mem.set_u32(me, 0x0900_0000);
        e.set(this, Controller::phkObject, 0x7700);
        e.set(this, Controller::StepDeltaTime, 0.005);
        e.set(this, Controller::StepInvDeltaTime, 200.0);
        let velocity = me + Controller::OutVelocity.off;
        e.mem.set_f32(velocity + 4, 3.0);
        e.register_double(HALF, |_, _| 0x6600u32.into_ret());
        for address in [0x00ca_c110, 0x00cd_3410, 0x00cd_32b0, 0x00cd_3490] {
            e.register_double(address, |_, _| Ret::default());
        }
        let seen = log.clone();
        e.register_double(SET_LINEAR_VELOCITY, move |e, a| {
            assert_eq!(a[0], 0x7700);
            seen.borrow_mut()
                .push(format!("velocity {}", e.mem.f32(a[1] + 8)));
            Ret::default()
        });
        let seen = log.clone();
        e.register_double(0x00ca_fbd0, move |e, a| {
            seen.borrow_mut().push(format!(
                "integrate {} {}",
                e.mem.f32(a[1] + 8),
                e.mem.f32(a[1] + 12)
            ));
            Ret::default()
        });
        fall_on(&mut e, this, -64.0, 0.025);
        assert_eq!(
            *log.borrow(),
            ["velocity -64", "integrate 0.025 40", "velocity 0"]
        );
        assert_eq!(e.get(this, Controller::StepDeltaTime), 0.005);
        assert_eq!(e.get(this, Controller::StepInvDeltaTime), 200.0);
        assert_eq!(e.mem.f32(velocity + 4), 3.0);
        assert_eq!(e.mem.f32(velocity + 8), 0.0);
    }

    /// The multiplier's frame-rate-free move: three 5.33 ms frames go as far as one 16 ms frame
    /// of the translation, and 16 ms frames are the translation's.
    #[test]
    fn the_time_multiplier_moves_by_time() {
        let setup = |step: f32| {
            let mut e = Engine::new();
            e.map(MAIN_TIMER & !0xfff, 0x1000);
            e.map(GLOBAL_TIME_MULTIPLIER & !0xfff, 0x1000);
            e.mem.set_f32(GLOBAL_TIME_MULTIPLIER, 1.0);
            e.mem.set_f32(TIME_MULTIPLIER_TARGET, 0.25);
            e.mem.set_f32(TIME_MULTIPLIER_RATE, 0.1);
            e.mem
                .set_f32(MAIN_TIMER + BSTimer::fDeltaUnmodified.off, step);
            e
        };
        let mut e = setup(0.016);
        e.call(TIME_MULTIPLIER_RAMP, &args![]);
        let one_frame = e.mem.f32(GLOBAL_TIME_MULTIPLIER);
        assert_eq!(one_frame, 0.9);
        install_slow_motion(&mut e);
        e.mem.set_f32(GLOBAL_TIME_MULTIPLIER, 1.0);
        e.call(TIME_MULTIPLIER_RAMP, &args![]);
        assert_eq!(e.mem.f32(GLOBAL_TIME_MULTIPLIER), one_frame);

        let mut e = setup(0.016 / 3.0);
        install_slow_motion(&mut e);
        for _ in 0..3 {
            e.call(TIME_MULTIPLIER_RAMP, &args![]);
        }
        assert!((e.mem.f32(GLOBAL_TIME_MULTIPLIER) - 0.9).abs() < 1e-5);
        // Up to the target and no further; at the target it stays.
        e.mem.set_f32(GLOBAL_TIME_MULTIPLIER, 0.2499);
        e.call(TIME_MULTIPLIER_RAMP, &args![]);
        assert_eq!(e.mem.f32(GLOBAL_TIME_MULTIPLIER), 0.25);
        e.call(TIME_MULTIPLIER_RAMP, &args![]);
        assert_eq!(e.mem.f32(GLOBAL_TIME_MULTIPLIER), 0.25);
    }
}
