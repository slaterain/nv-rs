//! Telling Windows the game is not background work: on by default in every run and every preset
//! (the maintainer, 2026-11-17: "on by default, no background slowdown"); off with `Fallout.ini`'s
//! `[nv-rs Threads] bNotBackground=0` in play or `NV_NOT_BACKGROUND=0` in a scripted run
//! (`host::fixes::Choice::not_background` decides; this crate only acts on it).
//!
//! Windows gives each window-owning process a quality of service from its window: in focus High,
//! visible but not in focus Medium, minimized or fully covered Low; on processors with two kinds
//! of core, Low work is scheduled to the efficient ones
//! (<https://learn.microsoft.com/en-us/windows/win32/procthread/quality-of-service>). A game
//! window that is not the active one can so run slower than the same window in focus (measured
//! 2026-11-17 on a hidden window: one run without this about a third slower, every part of the
//! frame alike). A process or a thread that turns execution-speed throttling off explicitly is
//! tagged High whatever its window (the same page, "API" row: the flag "lets developers
//! explicitly tag a process as HighQoS or EcoQoS"). The cost: Windows no longer saves power and
//! heat on the game while it is in the background.
//!
//! On, start-up ([`apply_at_start`]) makes two calls for the process and one for the thread that
//! starts the game, and each long-running thread the port starts makes one at its start
//! ([`thread_started`]): the game's threads, the render helpers, the drawing helper, the render
//! thread, wgpu's finishing and submit threads, the read-ahead queue, the window thread, the
//! sound output thread, the music decoder and the Vorbis read-ahead, the save copier, the cell
//! offsets builder, the profiler's sampler and the loading screen's output reader. Short-lived
//! cache savers make none (they follow the process setting). A thread started before start-up
//! decided makes none either.
//! - `SetProcessInformation(GetCurrentProcess(), ProcessPowerThrottling, {Version 1, ControlMask
//!   EXECUTION_SPEED | IGNORE_TIMER_RESOLUTION, StateMask 0})`: execution-speed throttling off
//!   ("HighQoS"), and timer resolution requests always honoured (Windows 11 may ignore a
//!   non-visible, silent process's otherwise). A Windows that refuses the pair (one older than
//!   the timer bit) is asked again with the execution-speed bit alone
//!   (<https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setprocessinformation>).
//! - `SetThreadInformation(GetCurrentThread(), ThreadPowerThrottling, {Version 1, ControlMask
//!   EXECUTION_SPEED, StateMask 0})` on the thread itself
//!   (<https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadinformation>).
//!
//! No priority changes. Values from the Windows SDK's processthreadsapi.h (the `windows` crate
//! 0.58's metadata, `Win32::System::Threading`): ProcessPowerThrottling 4, ThreadPowerThrottling
//! 3, both `*_CURRENT_VERSION` 1, `*_EXECUTION_SPEED` 1, `PROCESS_POWER_THROTTLING_IGNORE_TIMER_
//! RESOLUTION` 4. The calls sit behind [`System`] so the decisions are tested without changing
//! the test's own process. On other systems nothing is called.

use std::sync::atomic::{AtomicBool, Ordering};

/// `PROCESS_INFORMATION_CLASS::ProcessPowerThrottling`.
pub const PROCESS_POWER_THROTTLING: i32 = 4;
/// `THREAD_INFORMATION_CLASS::ThreadPowerThrottling`.
pub const THREAD_POWER_THROTTLING: i32 = 3;
/// `PROCESS_POWER_THROTTLING_CURRENT_VERSION` (and `THREAD_POWER_THROTTLING_CURRENT_VERSION`).
pub const CURRENT_VERSION: u32 = 1;
/// `PROCESS_POWER_THROTTLING_EXECUTION_SPEED` (and `THREAD_POWER_THROTTLING_EXECUTION_SPEED`).
pub const EXECUTION_SPEED: u32 = 1;
/// `PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION` (process only).
pub const IGNORE_TIMER_RESOLUTION: u32 = 4;

/// `PROCESS_POWER_THROTTLING_STATE` and `THREAD_POWER_THROTTLING_STATE` (the same three `ULONG`s).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Throttling {
    pub version: u32,
    pub control_mask: u32,
    pub state_mask: u32,
}

impl Throttling {
    /// The mechanisms in `control` explicitly off (`StateMask` 0): "Turn ... throttling off".
    pub const fn off(control: u32) -> Throttling {
        Throttling {
            version: CURRENT_VERSION,
            control_mask: control,
            state_mask: 0,
        }
    }
}

/// The two Windows calls (the seam). `Err` carries `GetLastError`'s code.
pub trait System {
    /// `SetProcessInformation(GetCurrentProcess(), ProcessPowerThrottling, state)`.
    fn set_process(&mut self, state: Throttling) -> Result<(), u32>;
    /// `SetThreadInformation(GetCurrentThread(), ThreadPowerThrottling, state)`.
    fn set_thread(&mut self, state: Throttling) -> Result<(), u32>;
}

/// What start-up did for the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Process {
    /// Off: nothing called.
    Off,
    /// Execution-speed throttling off and timer requests honoured.
    Both,
    /// The pair refused (the code), execution-speed throttling off alone accepted.
    SpeedOnly(u32),
    /// Both asks refused (the second's code).
    Refused(u32),
}

/// Start-up's calls for the process and the calling thread: off, nothing; on, the process pair
/// (execution speed with the timer bit; again without it when refused), then the calling
/// thread's call. Answers the process's outcome and the thread's.
pub fn start(on: bool, sys: &mut impl System) -> (Process, Result<(), u32>) {
    if !on {
        return (Process::Off, Ok(()));
    }
    let process = match sys.set_process(Throttling::off(EXECUTION_SPEED | IGNORE_TIMER_RESOLUTION))
    {
        Ok(()) => Process::Both,
        Err(pair) => match sys.set_process(Throttling::off(EXECUTION_SPEED)) {
            Ok(()) => Process::SpeedOnly(pair),
            Err(code) => Process::Refused(code),
        },
    };
    (process, thread(on, sys))
}

/// A started thread's call: off, nothing; on, its execution-speed throttling off.
pub fn thread(on: bool, sys: &mut impl System) -> Result<(), u32> {
    if !on {
        return Ok(());
    }
    sys.set_thread(Throttling::off(EXECUTION_SPEED))
}

/// How to turn it off, for the start-up line.
const TURN_OFF: &str =
    "bNotBackground=0 in Fallout.ini's [nv-rs Threads] or NV_NOT_BACKGROUND=0 turns it off";

/// Start-up's log line: what was applied, or that it is off and who chose that (`by`: the
/// variable or the file's key; `None` the default).
pub fn describe(by: Option<&str>, process: Process, thread: Result<(), u32>) -> String {
    let thread = match thread {
        Ok(()) => "this thread and each thread the port starts too".to_string(),
        Err(code) => format!("this thread's call refused (error {code})"),
    };
    let chosen = by.map_or("on by default".to_string(), |by| format!("on by {by}"));
    match process {
        Process::Off => format!(
            "Not background: off by choice ({}): Windows may slow the game down or move it to the slower \
             cores when its window is not the active one",
            by.unwrap_or("asked")
        ),
        Process::Both => format!(
            "Not background ({chosen}; {TURN_OFF}): Windows told the game is not background work \
             (execution-speed throttling off, timer requests honoured) whatever its window's focus; {thread}"
        ),
        Process::SpeedOnly(pair) => format!(
            "Not background ({chosen}; {TURN_OFF}): Windows told the game is not background work \
             (execution-speed throttling off; the timer-request bit refused, error {pair}: an older Windows); {thread}"
        ),
        Process::Refused(code) => format!(
            "Not background ({chosen}): Windows refused it (error {code}): treated as its window's focus says"
        ),
    }
}

/// Start-up's whole step on `flag` (what [`thread_started`] reads afterwards) and `sys`: the flag
/// set to `on`, the calls ([`start`]), the line ([`describe`]).
pub fn apply_with(on: bool, by: Option<&str>, flag: &AtomicBool, sys: &mut impl System) -> String {
    flag.store(on, Ordering::Relaxed);
    let (process, thread) = start(on, sys);
    describe(by, process, thread)
}

/// A started thread's step on `flag` and `sys`: [`thread`] with the flag start-up set (off until
/// start-up has decided).
pub fn thread_with(flag: &AtomicBool, sys: &mut impl System) -> Result<(), u32> {
    thread(flag.load(Ordering::Relaxed), sys)
}

/// The run's decision, set by [`apply_at_start`]; off until then.
static ON: AtomicBool = AtomicBool::new(false);

/// At start-up, on the thread that starts the game, with the run's choice (`on`, and `by`: who
/// chose it, `None` for the default): the process's calls and this thread's, and the log line.
pub fn apply_at_start(on: bool, by: Option<&str>) -> String {
    if cfg!(not(windows)) {
        return "Not background: Windows only, nothing to change here".to_string();
    }
    apply_with(on, by, &ON, &mut Real)
}

/// At the start of each thread the port starts, on that thread: its call (when start-up turned
/// it on). A refusal is said once a run.
pub fn thread_started() {
    if cfg!(not(windows)) {
        return;
    }
    if let Err(code) = thread_with(&ON, &mut Real) {
        static SAID: AtomicBool = AtomicBool::new(false);
        if !SAID.swap(true, Ordering::Relaxed) {
            let name = std::thread::current()
                .name()
                .unwrap_or("unnamed")
                .to_string();
            eprintln!(
                "Not background: Windows refused it for the thread \"{name}\" (error {code})"
            );
        }
    }
}

/// The real calls (Windows; elsewhere never reached: [`apply_at_start`] and [`thread_started`]
/// stop first).
struct Real;

#[cfg(windows)]
impl System for Real {
    fn set_process(&mut self, state: Throttling) -> Result<(), u32> {
        // SAFETY: the current-process pseudo handle and a live 12-byte state of the documented
        // layout, its size passed.
        let ok = unsafe {
            win::SetProcessInformation(
                win::GetCurrentProcess(),
                PROCESS_POWER_THROTTLING,
                &state as *const Throttling as *const core::ffi::c_void,
                core::mem::size_of::<Throttling>() as u32,
            )
        };
        // SAFETY: no arguments.
        if ok != 0 {
            Ok(())
        } else {
            Err(unsafe { win::GetLastError() })
        }
    }

    fn set_thread(&mut self, state: Throttling) -> Result<(), u32> {
        // SAFETY: the current-thread pseudo handle (it has THREAD_SET_INFORMATION) and a live
        // 12-byte state of the documented layout, its size passed.
        let ok = unsafe {
            win::SetThreadInformation(
                win::GetCurrentThread(),
                THREAD_POWER_THROTTLING,
                &state as *const Throttling as *const core::ffi::c_void,
                core::mem::size_of::<Throttling>() as u32,
            )
        };
        // SAFETY: no arguments.
        if ok != 0 {
            Ok(())
        } else {
            Err(unsafe { win::GetLastError() })
        }
    }
}

#[cfg(not(windows))]
impl System for Real {
    fn set_process(&mut self, _: Throttling) -> Result<(), u32> {
        Err(0)
    }
    fn set_thread(&mut self, _: Throttling) -> Result<(), u32> {
        Err(0)
    }
}

#[cfg(windows)]
mod win {
    use core::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetCurrentProcess() -> *mut c_void;
        pub fn GetCurrentThread() -> *mut c_void;
        pub fn SetProcessInformation(
            process: *mut c_void,
            class: i32,
            information: *const c_void,
            size: u32,
        ) -> i32;
        pub fn SetThreadInformation(
            thread: *mut c_void,
            class: i32,
            information: *const c_void,
            size: u32,
        ) -> i32;
        pub fn GetLastError() -> u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum Call {
        Process(Throttling),
        Thread(Throttling),
    }

    /// Records the calls; `refuse_pair`/`refuse_speed`/`refuse_thread` answer those with an error.
    #[derive(Default)]
    struct Fake {
        calls: Vec<Call>,
        refuse_pair: Option<u32>,
        refuse_speed: Option<u32>,
        refuse_thread: Option<u32>,
    }

    impl System for Fake {
        fn set_process(&mut self, state: Throttling) -> Result<(), u32> {
            self.calls.push(Call::Process(state));
            let refused = if state.control_mask & IGNORE_TIMER_RESOLUTION != 0 {
                self.refuse_pair
            } else {
                self.refuse_speed
            };
            refused.map_or(Ok(()), Err)
        }
        fn set_thread(&mut self, state: Throttling) -> Result<(), u32> {
            self.calls.push(Call::Thread(state));
            self.refuse_thread.map_or(Ok(()), Err)
        }
    }

    /// The documented HighQoS values: version 1, the mechanisms selected, all of them off.
    const PAIR: Throttling = Throttling {
        version: 1,
        control_mask: 1 | 4,
        state_mask: 0,
    };
    const SPEED: Throttling = Throttling {
        version: 1,
        control_mask: 1,
        state_mask: 0,
    };

    #[test]
    fn the_constants_are_the_sdks() {
        assert_eq!(PROCESS_POWER_THROTTLING, 4);
        assert_eq!(THREAD_POWER_THROTTLING, 3);
        assert_eq!(
            (CURRENT_VERSION, EXECUTION_SPEED, IGNORE_TIMER_RESOLUTION),
            (1, 1, 4)
        );
        assert_eq!(core::mem::size_of::<Throttling>(), 12);
    }

    #[test]
    fn off_calls_nothing() {
        let mut fake = Fake::default();
        assert_eq!(start(false, &mut fake), (Process::Off, Ok(())));
        assert_eq!(thread(false, &mut fake), Ok(()));
        assert_eq!(fake.calls, vec![]);
    }

    /// Turned off (the file's key or the variable): start-up and every later thread make no call,
    /// and the line says it is off and who chose that.
    #[test]
    fn turned_off_start_up_and_threads_make_no_call_and_say_so() {
        for by in [
            "NV_NOT_BACKGROUND",
            "Fallout.ini's [nv-rs Threads] bNotBackground",
        ] {
            let flag = AtomicBool::new(true);
            let mut fake = Fake::default();
            let line = apply_with(false, Some(by), &flag, &mut fake);
            assert_eq!(thread_with(&flag, &mut fake), Ok(()));
            assert_eq!(fake.calls, vec![], "{by}");
            assert!(
                line.starts_with("Not background: off by choice ("),
                "{line}"
            );
            assert!(line.contains(by), "{line}");
        }
        // Before start-up decides, a thread makes no call.
        let mut fake = Fake::default();
        assert_eq!(thread_with(&AtomicBool::new(false), &mut fake), Ok(()));
        assert_eq!(fake.calls, vec![]);
    }

    /// On (the default): the process pair and this thread at start-up, then one call per started
    /// thread; the line says it is on by default and how to turn it off.
    #[test]
    fn on_by_default_start_up_and_threads_make_the_documented_calls() {
        let flag = AtomicBool::new(false);
        let mut fake = Fake::default();
        let line = apply_with(true, None, &flag, &mut fake);
        assert_eq!(fake.calls, vec![Call::Process(PAIR), Call::Thread(SPEED)]);
        assert!(
            line.starts_with("Not background (on by default; "),
            "{line}"
        );
        assert!(
            line.contains("bNotBackground=0") && line.contains("NV_NOT_BACKGROUND=0"),
            "{line}"
        );
        assert!(line.contains("timer requests honoured"), "{line}");
        assert!(line.contains("each thread the port starts"), "{line}");
        fake.calls.clear();
        assert_eq!(thread_with(&flag, &mut fake), Ok(()));
        assert_eq!(fake.calls, vec![Call::Thread(SPEED)]);
        let asked = apply_with(true, Some("NV_NOT_BACKGROUND"), &flag, &mut Fake::default());
        assert!(
            asked.starts_with("Not background (on by NV_NOT_BACKGROUND; "),
            "{asked}"
        );
    }

    #[test]
    fn on_turns_throttling_off_for_the_process_and_the_thread() {
        let mut fake = Fake::default();
        assert_eq!(start(true, &mut fake), (Process::Both, Ok(())));
        assert_eq!(fake.calls, vec![Call::Process(PAIR), Call::Thread(SPEED)]);
        fake.calls.clear();
        assert_eq!(thread(true, &mut fake), Ok(()));
        assert_eq!(fake.calls, vec![Call::Thread(SPEED)]);
    }

    #[test]
    fn a_refused_pair_is_asked_again_without_the_timer_bit() {
        let mut fake = Fake {
            refuse_pair: Some(87),
            ..Fake::default()
        };
        assert_eq!(start(true, &mut fake), (Process::SpeedOnly(87), Ok(())));
        assert_eq!(
            fake.calls,
            vec![
                Call::Process(PAIR),
                Call::Process(SPEED),
                Call::Thread(SPEED)
            ]
        );
    }

    #[test]
    fn refusals_are_reported_with_their_codes() {
        let mut fake = Fake {
            refuse_pair: Some(87),
            refuse_speed: Some(5),
            refuse_thread: Some(6),
            ..Fake::default()
        };
        assert_eq!(start(true, &mut fake), (Process::Refused(5), Err(6)));
        assert_eq!(thread(true, &mut fake), Err(6));
    }

    #[test]
    fn the_log_line_says_what_was_applied_or_why_not() {
        let older = describe(None, Process::SpeedOnly(87), Err(6));
        assert!(
            older.contains("error 87") && older.contains("error 6"),
            "{older}"
        );
        let refused = describe(None, Process::Refused(5), Ok(()));
        assert!(refused.contains("refused it (error 5)"), "{refused}");
    }
}
