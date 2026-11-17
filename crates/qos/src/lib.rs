//! Telling Windows the game is not background work (`NV_NOT_BACKGROUND=1`; off by default).
//!
//! Windows gives each window-owning process a quality of service from its window: in focus High,
//! visible but not in focus Medium, minimized or fully covered Low; on processors with two kinds
//! of core, Low work is scheduled to the efficient ones
//! (<https://learn.microsoft.com/en-us/windows/win32/procthread/quality-of-service>). A game
//! window that loses the focus can so run slower than the same window in focus. A process or a
//! thread that turns execution-speed throttling off explicitly is tagged High whatever its window
//! (the same page, "API" row: the flag "lets developers explicitly tag a process as HighQoS or
//! EcoQoS").
//!
//! On, start-up ([`apply_at_start`]) makes two calls for the process and one for the thread that
//! starts the game, and each thread the port starts makes one at its start ([`thread_started`]):
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
use std::sync::OnceLock;

/// The switch's name.
pub const SWITCH: &str = "NV_NOT_BACKGROUND";

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

/// Whether the switch's value turns it on: exactly `1`.
pub fn switch_on(value: Option<&str>) -> bool {
    value.map(str::trim) == Some("1")
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

/// Start-up's decision and calls for the process and the calling thread: off, nothing; on, the
/// process pair (execution speed with the timer bit; again without it when refused), then the
/// calling thread's call. Answers the process's outcome and the thread's.
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

/// Start-up's log line for the outcome (`None` when the switch is unset: no line).
pub fn describe(value: Option<&str>, process: Process, thread: Result<(), u32>) -> Option<String> {
    let value = value?;
    let thread = match thread {
        Ok(()) => "this thread and each thread the port starts too".to_string(),
        Err(code) => format!("this thread's call refused (error {code})"),
    };
    Some(match process {
        Process::Off => format!(
            "{SWITCH}={value}: off (only 1 turns it on): Windows treats the game as its window's focus says"
        ),
        Process::Both => format!(
            "{SWITCH}=1: Windows told the game is not background work (execution-speed throttling off, \
             timer requests honoured) whatever its window's focus; {thread}"
        ),
        Process::SpeedOnly(pair) => format!(
            "{SWITCH}=1: Windows told the game is not background work (execution-speed throttling off; \
             the timer-request bit refused, error {pair}: an older Windows); {thread}"
        ),
        Process::Refused(code) => format!(
            "{SWITCH}=1: Windows refused it (error {code}): treated as its window's focus says"
        ),
    })
}

/// The switch, read once ([`apply_at_start`] or the first [`thread_started`]).
fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| switch_on(std::env::var(SWITCH).ok().as_deref()))
}

/// At start-up, on the thread that starts the game: the process's calls and this thread's
/// (switch on), and the log line ([`describe`]; none when the switch is unset).
pub fn apply_at_start() -> Option<String> {
    let value = std::env::var(SWITCH).ok();
    if cfg!(not(windows)) {
        return value.map(|v| format!("{SWITCH}={v}: Windows only, nothing changed here"));
    }
    let (process, thread) = start(on(), &mut Real);
    describe(value.as_deref(), process, thread)
}

/// At the start of each thread the port starts, on that thread: its call (switch on). A refusal
/// is said once a run.
pub fn thread_started() {
    if cfg!(not(windows)) || !on() {
        return;
    }
    if let Err(code) = thread(true, &mut Real) {
        static SAID: AtomicBool = AtomicBool::new(false);
        if !SAID.swap(true, Ordering::Relaxed) {
            let name = std::thread::current()
                .name()
                .unwrap_or("unnamed")
                .to_string();
            eprintln!("{SWITCH}=1: Windows refused it for the thread \"{name}\" (error {code})");
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
    fn only_1_turns_it_on() {
        assert!(switch_on(Some("1")));
        assert!(switch_on(Some(" 1 ")));
        for v in [None, Some(""), Some("0"), Some("true"), Some("2")] {
            assert!(!switch_on(v), "{v:?}");
        }
    }

    #[test]
    fn off_calls_nothing() {
        let mut fake = Fake::default();
        assert_eq!(start(false, &mut fake), (Process::Off, Ok(())));
        assert_eq!(thread(false, &mut fake), Ok(()));
        assert_eq!(fake.calls, vec![]);
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
        assert_eq!(describe(None, Process::Off, Ok(())), None);
        let off = describe(Some("0"), Process::Off, Ok(())).unwrap();
        assert!(off.contains("NV_NOT_BACKGROUND=0: off"), "{off}");
        let both = describe(Some("1"), Process::Both, Ok(())).unwrap();
        assert!(both.contains("timer requests honoured"), "{both}");
        assert!(both.contains("each thread the port starts"), "{both}");
        let older = describe(Some("1"), Process::SpeedOnly(87), Err(6)).unwrap();
        assert!(
            older.contains("error 87") && older.contains("error 6"),
            "{older}"
        );
        let refused = describe(Some("1"), Process::Refused(5), Ok(())).unwrap();
        assert!(refused.contains("refused it (error 5)"), "{refused}");
    }
}
