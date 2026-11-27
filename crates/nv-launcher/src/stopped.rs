//! The launcher never ends without a word (a Play.exe that showed nothing when double-clicked and
//! left no folder behind). A failure the window finds itself is already said, and Play goes on
//! without the window (`app::cannot_draw`); this covers the rest.
//!
//! - A panic in the window's code (reading the game's fonts and pictures, the sounds, a screen, a
//!   window callback: winit 0.30 catches a callback's panic and raises it again from `run_app`)
//!   is caught around the window's whole run (`app::run`), said with its line and the start-up
//!   lines, then Play goes on as after `cannot_draw` (Windows' message boxes; play.sh in the
//!   terminal), or, when Play's work is already running, until it ends.
//! - Windows: a crash in native code in the launcher's own process (a graphics driver, or an
//!   overlay's DLL hooking Direct3D 12) reaches the last-chance filter ([`watch_crashes`]), which
//!   names the exception and the file the crash happened in, and starts Play.exe again with the
//!   same command line and that line in `NV_RS_LAUNCHER_CRASH`. That Play.exe says it in a
//!   message box and plays in message boxes (`app::after_crash`); it never opens the window and
//!   never starts itself again. The filter is the launcher process's own: the game is a process of
//!   its own, with its own crash handling, and its crashes never reach it.
//!
//! The start-up lines ([`note`]): the window's lines before it shows (the look, the drawing
//! chosen, why it cannot draw), kept in memory, the last [`LINES_KEPT`], for the message and the
//! log. On Windows the log line waits for Play's own first write ([`hold_for_log`]), as everything
//! the launcher writes in its folder does.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// How many start-up lines are kept (the last ones), and how long each may be.
pub const LINES_KEPT: usize = 8;
pub const LINE_CHARS: usize = 240;
/// How long the stop's own line may be in the message: a graphics error's text can run to pages,
/// and its start says what it is.
const WHAT_CHARS: usize = 600;

const ISSUES: &str = "https://github.com/slaterain/nv-rs/issues";

static LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// The window's run is going on (a crash now starts Play.exe again), Play's work is running (a
/// crash now does not: the game may be running, and a second Play would start a second one), and
/// the launcher is ending as asked ([`leave`]: a stop now is left as it was before any of this).
static IN_WINDOW: AtomicBool = AtomicBool::new(false);
static WORKING: AtomicBool = AtomicBool::new(false);
static LEAVING: AtomicBool = AtomicBool::new(false);

/// A start-up line: on stderr as before, and kept for a stop's message.
pub fn note(text: &str) {
    eprintln!("{text}");
    keep_line(text);
}

/// [`note`] without the stderr line (the caller prints its own).
pub fn keep_line(text: &str) {
    if let Ok(mut lines) = LINES.lock() {
        kept(&mut lines, text);
    }
}

/// `text` added to `lines`, one line of at most [`LINE_CHARS`], the oldest going past
/// [`LINES_KEPT`].
fn kept(lines: &mut VecDeque<String>, text: &str) {
    if lines.len() == LINES_KEPT {
        lines.pop_front();
    }
    lines.push_back(shortened(text, LINE_CHARS));
}

/// The start-up lines kept so far, oldest first.
pub fn lines() -> Vec<String> {
    LINES
        .lock()
        .map(|l| l.iter().cloned().collect())
        .unwrap_or_default()
}

/// `text` on one line (its line breaks and runs of spaces as one space), at most `most`
/// characters (a cut one ends in "...").
pub fn shortened(text: &str, most: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= most {
        return flat;
    }
    let mut cut: String = flat.chars().take(most.saturating_sub(3)).collect();
    cut.push_str("...");
    cut
}

/// The window's run starts (`true`) or is over.
pub fn set_in_window(yes: bool) {
    IN_WINDOW.store(yes, Ordering::SeqCst);
}

/// Play's work (or the installer's, or the import) starts (`true`) or has ended.
pub fn set_working(yes: bool) {
    WORKING.store(yes, Ordering::SeqCst);
}

/// Whether the window's run is going on, and whether Play's work is running.
/// (Read by the Windows filter; elsewhere by the tests only.)
#[cfg_attr(not(windows), allow(dead_code))]
pub fn in_window() -> bool {
    IN_WINDOW.load(Ordering::SeqCst)
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn working() -> bool {
    WORKING.load(Ordering::SeqCst)
}

/// The launcher is ending as the player or Play asked (Exit, Play's good end, the window closed
/// with nothing running), before anything is put away. From now on a crash is left to Windows as
/// before (no message, Play.exe not started again) and a panic ends it quietly: the drawing
/// device, the window and the sounds go now, and overlays and drivers often crash just then,
/// which is no reason to play again.
pub fn leave() {
    if !LEAVING.swap(true, Ordering::SeqCst) {
        #[cfg(debug_assertions)]
        test_stop(true);
    }
}

pub fn leaving() -> bool {
    LEAVING.load(Ordering::SeqCst)
}

/// What follows a panic in the window's run.
#[derive(Debug, PartialEq)]
pub enum AfterPanic {
    /// The launcher was ending as asked: it ends, quietly, as before.
    Quiet,
    /// Play's work runs: said, then its end is waited for.
    WaitForWork,
    /// Said, then Play in message boxes (play.sh in the terminal).
    PlayWithoutWindow,
}

/// [`AfterPanic`] for a panic while `leaving` or with work `running`.
pub fn after_panic(leaving: bool, running: bool) -> AfterPanic {
    match (leaving, running) {
        (true, _) => AfterPanic::Quiet,
        (false, true) => AfterPanic::WaitForWork,
        (false, false) => AfterPanic::PlayWithoutWindow,
    }
}

thread_local! {
    /// The line of the last panic on this thread, from the hook ([`watch_panics`]).
    static PANIC_LINE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// From now on a panic's line (its message and its place in the code) is kept for its thread's
/// [`panic_line`]; the hook before (Rust's, which prints it) still runs.
pub fn watch_panics() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let before = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let at = info.location().map(|l| (l.file(), l.line()));
            let line = hook_line(message_of(info.payload()), at);
            // Never a second panic in the hook (that would end the process at once).
            let _ = PANIC_LINE.try_with(|kept| {
                if let Ok(mut kept) = kept.try_borrow_mut() {
                    *kept = Some(line);
                }
            });
            before(info);
        }));
    });
}

/// A panic's message, as `panic!` gave it.
fn message_of(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("a panic without a message")
}

/// A panic's line: its message and, when known, its file and line.
fn hook_line(message: &str, at: Option<(&str, u32)>) -> String {
    match at {
        Some((file, line)) => format!("{message} (at {file}:{line})"),
        None => message.to_string(),
    }
}

/// The line of the panic caught on this thread with `payload`: the hook's (its message and
/// place) when it is this panic's, else the message alone.
pub fn panic_line(payload: &(dyn std::any::Any + Send)) -> String {
    let message = message_of(payload);
    let hooked = PANIC_LINE
        .try_with(|kept| kept.try_borrow_mut().ok().and_then(|mut k| k.take()))
        .ok()
        .flatten();
    match hooked {
        Some(line) if line.starts_with(message) => line,
        _ => message.to_string(),
    }
}

/// What the player reads when the launcher (its `window`, or Play after it) stopped: `what`
/// happened, the start-up lines kept, and what happens `next`.
pub fn stopped_text(window: bool, what: &str, lines: &[String], next: &str) -> String {
    let what = shortened(what, WHAT_CHARS);
    let mut text = format!(
        "nv-rs's launcher{} stopped: {what}{}",
        if window { " window" } else { "" },
        if what.ends_with('.') { "" } else { "." }
    );
    if !lines.is_empty() {
        text.push_str("\n\nIts last steps:");
        for line in lines {
            text.push_str("\n  ");
            text.push_str(line);
        }
    }
    text.push_str("\n\n");
    text.push_str(next);
    text
}

/// What a panic is, for [`stopped_text`].
pub fn panic_what(line: &str) -> String {
    format!("an error in nv-rs's own code: {line}")
}

/// Where to report it.
pub fn report() -> String {
    format!("Please report this at {ISSUES} with this message.")
}

/// What happens after the window stopped with no work running (`native`: a crash in native code,
/// which a program drawing over games can cause).
pub fn next_play(native: bool) -> String {
    let mut text = String::from(if cfg!(windows) {
        "Play goes on without it, in Windows' own message boxes."
    } else {
        "It goes on in the terminal instead: play.sh, as ./play.sh starts it."
    });
    if native {
        text.push_str(
            " A program that draws over games (an overlay or a recorder) can cause this: turning \
             it off for Play.exe may bring the window back.",
        );
    }
    text.push_str("\n\n");
    text.push_str(&report());
    text
}

/// What happens after the window stopped while Play's work runs.
pub fn next_working() -> String {
    let text = if cfg!(windows) {
        "Play goes on without it: its questions come in Windows' own message boxes, and nv-rs \
         ends when Play ends."
    } else {
        "Play goes on without it: its questions are answered as Esc answers them, and nv-rs \
         ends when Play ends."
    };
    format!("{text}\n\n{}", report())
}

/// How a message box looks: a warning (the window cannot draw, as before) or an error (it
/// stopped).
#[derive(Clone, Copy)]
pub enum Icon {
    Warning,
    Error,
}

/// `text` where the player can read it: a message box of Windows' (with `icon`); elsewhere the
/// terminal the launcher was started from, and without a terminal the launcher's log in `log`
/// (nv-rs's own folder; `None` where nothing may be written there).
pub fn say(text: &str, icon: Icon, log: Option<&Path>) {
    eprintln!("{text}");
    #[cfg(windows)]
    {
        let _ = log;
        // MB_ICONWARNING; MB_ICONERROR | MB_SETFOREGROUND.
        message_box(
            text,
            match icon {
                Icon::Warning => 0x30,
                Icon::Error => 0x1_0010,
            },
        );
    }
    #[cfg(not(windows))]
    {
        let _ = icon;
        use std::io::IsTerminal;
        if let Some(own) = log.filter(|_| !std::io::stderr().is_terminal()) {
            let logs = own.join("logs");
            let written = std::fs::create_dir_all(&logs).and_then(|()| {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(logs.join("launcher.log"))?;
                writeln!(f, "{text}")
            });
            if let Err(e) = written {
                eprintln!("{}: {e}", logs.display());
            }
        }
    }
}

/// A stop's text for the launcher's log. Windows: written at once if a Play in this process has
/// passed its first write, else held for Play's first write (`place::log_line`), so it is written
/// only where Play's own checks of the folder pass (never in the game's folder, a temporary
/// folder, Documents or AppData) and never where they refuse; elsewhere [`say`] wrote it already.
pub fn hold_for_log(text: &str) {
    #[cfg(windows)]
    if !crate::play::append_if_written(text) {
        crate::place::log_line(Path::new(""), false, text);
    }
    #[cfg(not(windows))]
    let _ = text;
}

/// Debug builds only, for the live check of this module: `NV_RS_TEST_LAUNCHER_STOP=panic` panics
/// in the window's run, `=crash` (Windows) reads an address no program may read there, as a
/// faulty driver does; `exit-panic` and `exit-crash` do the same as the launcher ends as asked
/// (`at_exit`, [`leave`]).
#[cfg(debug_assertions)]
pub fn test_stop(at_exit: bool) {
    let asked = std::env::var("NV_RS_TEST_LAUNCHER_STOP").unwrap_or_default();
    let what = match asked.strip_prefix("exit-") {
        Some(what) if at_exit => what,
        None if !at_exit => asked.as_str(),
        _ => return,
    };
    match what {
        "panic" => panic!("NV_RS_TEST_LAUNCHER_STOP={asked}: the window stopped on purpose"),
        #[cfg(windows)]
        "crash" => {
            // SAFETY: none; the point is the access violation (a debug build's test only).
            let _ = unsafe { std::ptr::read_volatile(std::ptr::without_provenance::<u8>(8)) };
        }
        _ => {}
    }
}

/// Windows' message box with `text`, no owner window, of `kind`.
#[cfg(windows)]
fn message_box(text: &str, kind: u32) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            owner: *mut std::ffi::c_void,
            text: *const u16,
            title: *const u16,
            kind: u32,
        ) -> i32;
    }
    let (t, title) = (wide(text), wide("nv-rs"));
    // SAFETY: both strings live through the call; no owner window.
    unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), title.as_ptr(), kind) };
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

#[cfg(windows)]
pub use native::{crash_passed_on, watch_crashes, Crash};

/// Windows: the last-chance filter for crashes in native code in the launcher's process.
#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::c_void;
    use std::sync::atomic::AtomicU32;
    use std::sync::OnceLock;

    /// What a crash passes on to the Play.exe it starts again ([`crash_value`]).
    pub const CRASH_VAR: &str = "NV_RS_LAUNCHER_CRASH";
    const EXCEPTION_EXECUTE_HANDLER: i32 = 1;
    const EXCEPTION_CONTINUE_SEARCH: i32 = 0;

    /// `EXCEPTION_RECORD` (only the code and the address are read).
    #[repr(C)]
    #[allow(dead_code)]
    struct ExceptionRecord {
        code: u32,
        flags: u32,
        record: *const ExceptionRecord,
        address: *const c_void,
        parameters: u32,
        information: [usize; 15],
    }

    /// `EXCEPTION_POINTERS` (only the record is read).
    #[repr(C)]
    #[allow(dead_code)]
    struct ExceptionPointers {
        record: *const ExceptionRecord,
        context: *const c_void,
    }

    /// `STARTUPINFOW`: all empty but its size.
    #[repr(C)]
    #[allow(dead_code)]
    struct StartupInfoW {
        size: u32,
        reserved: *mut u16,
        desktop: *mut u16,
        title: *mut u16,
        place: [u32; 8],
        show_window: u16,
        reserved2_size: u16,
        reserved2: *mut u8,
        std_input: *mut c_void,
        std_output: *mut c_void,
        std_error: *mut c_void,
    }

    /// `PROCESS_INFORMATION` (the handles are closed; the ids are not read).
    #[repr(C)]
    #[allow(dead_code)]
    pub struct ProcessInformation {
        process: *mut c_void,
        thread: *mut c_void,
        process_id: u32,
        thread_id: u32,
    }

    type Filter = unsafe extern "system" fn(*const ExceptionPointers) -> i32;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetUnhandledExceptionFilter(filter: Option<Filter>) -> *mut c_void;
        fn GetModuleHandleExW(flags: u32, name: *const u16, module: *mut *mut c_void) -> i32;
        fn GetModuleFileNameW(module: *mut c_void, name: *mut u16, size: u32) -> u32;
        fn GetCommandLineW() -> *const u16;
        fn SetEnvironmentVariableW(name: *const u16, value: *const u16) -> i32;
        fn CreateProcessW(
            application: *const u16,
            command_line: *mut u16,
            process_attributes: *const c_void,
            thread_attributes: *const c_void,
            inherit_handles: i32,
            flags: u32,
            environment: *const c_void,
            folder: *const u16,
            startup: *const StartupInfoW,
            information: *mut ProcessInformation,
        ) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn GetCurrentThreadId() -> u32;
        fn Sleep(milliseconds: u32);
    }

    /// This program's path and its whole command line as Windows has it (each ending in a NUL),
    /// kept before any crash so the filter only copies them.
    static EXE: OnceLock<Vec<u16>> = OnceLock::new();
    static COMMAND: OnceLock<Vec<u16>> = OnceLock::new();
    /// The thread whose crash the filter is saying (0: none): a crash on it while it does ends
    /// the process at once; another thread's waits for the end.
    static SAYING: AtomicU32 = AtomicU32::new(0);
    /// The saying thread has made its message and is showing it: a waiting thread then waits for
    /// the end however long the player reads ([`WAIT_STEPS`]).
    static SHOWING: AtomicBool = AtomicBool::new(false);
    /// How long another thread's crash waits for the saying thread to reach its message box, in
    /// steps of [`WAIT_STEP_MS`] (about 10 seconds): the saying thread may need a lock the waiting
    /// thread held when it crashed (Windows' loader lock, the heap's), so after that the process
    /// ends as it did before this filter existed, instead of hanging.
    const WAIT_STEPS: u32 = 100;
    const WAIT_STEP_MS: u32 = 100;
    /// This Play.exe was started again by a crash: no crash starts it again.
    static RESTARTED: AtomicBool = AtomicBool::new(false);

    /// From now on a crash in native code in this process is said ([`last_chance`]).
    pub fn watch_crashes() {
        let (exe, command) = this_start();
        if let Some(exe) = exe {
            let _ = EXE.set(exe);
        }
        let _ = COMMAND.set(command);
        // SAFETY: the filter is a plain function living as long as the process.
        unsafe { SetUnhandledExceptionFilter(Some(last_chance)) };
    }

    /// This program's path and its command line (each ending in a NUL): what starts it again.
    pub fn this_start() -> (Option<Vec<u16>>, Vec<u16>) {
        use std::os::windows::ffi::OsStrExt;
        let exe = std::env::current_exe()
            .ok()
            .map(|p| p.as_os_str().encode_wide().chain([0]).collect());
        // SAFETY: Windows' own NUL-ended copy of the command line, living as long as the process.
        let command = unsafe {
            let p = GetCommandLineW();
            if p.is_null() {
                vec![0]
            } else {
                let mut n = 0;
                while *p.add(n) != 0 {
                    n += 1;
                }
                std::slice::from_raw_parts(p, n + 1).to_vec()
            }
        };
        (exe, command)
    }

    /// The file holding `address` (a DLL's or a program's name, without its folder) and where
    /// that file starts in memory, if a file this process loaded holds it.
    pub fn module_of(address: usize) -> Option<(String, usize)> {
        let mut module = std::ptr::null_mut();
        // 4 | 2: GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, ..._UNCHANGED_REFCOUNT.
        // SAFETY: with FROM_ADDRESS the "name" is only an address, never read.
        if unsafe { GetModuleHandleExW(6, address as *const u16, &mut module) } == 0 {
            return None;
        }
        let mut name = [0u16; 1024];
        // SAFETY: the buffer and its length are this array's.
        let n = unsafe { GetModuleFileNameW(module, name.as_mut_ptr(), name.len() as u32) };
        if n == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&name[..(n as usize).min(name.len())]);
        let file = path.rsplit(['\\', '/']).next()?.to_string();
        // A module's handle is where it starts.
        Some((file, module as usize))
    }

    /// The plain name of an exception code people meet.
    pub fn exception_name(code: u32) -> Option<&'static str> {
        Some(match code {
            0xc000_0005 => "an access violation",
            0xc000_0006 => "a page that could not be read in",
            0xc000_001d => "an illegal instruction",
            0xc000_0094 => "an integer division by zero",
            0xc000_0096 => "a privileged instruction",
            0xc000_00fd => "a stack overflow",
            0x8000_0003 => "a breakpoint",
            0xe06d_7363 => "a C++ exception nobody caught",
            _ => return None,
        })
    }

    /// The crash, for the message: the file it happened in and how far into it (Windows' own
    /// record of the crash, which says that, is not made when the filter says it), the
    /// exception's name and code.
    pub fn crash_line(code: u32, module: Option<(&str, usize)>) -> String {
        let kind = match exception_name(code) {
            Some(name) => format!("{name}, {code:#010x}"),
            None => format!("exception {code:#010x}"),
        };
        match module {
            Some((file, offset)) => format!("it crashed in {file}+{offset:#x} ({kind})"),
            None => format!("it crashed outside every file it loaded ({kind})"),
        }
    }

    /// What a crash does: nothing (Windows ends the process as before any of this), start
    /// Play.exe again (its message and Play in message boxes), or say it here and end.
    #[derive(Debug, PartialEq)]
    pub enum AfterCrash {
        AsBefore,
        StartAgain,
        SayHere,
    }

    /// A crash while the launcher ends as asked (`leaving`) is left as it was before. Otherwise
    /// only a crash in the window's run, with no work running, in a Play.exe not started again by
    /// a crash, starts Play.exe again: never twice in a row, and never while Play runs (the game
    /// may be running; a second Play would start a second game).
    pub fn after_crash(
        leaving: bool,
        restarted: bool,
        in_window: bool,
        working: bool,
    ) -> AfterCrash {
        if leaving {
            return AfterCrash::AsBefore;
        }
        if !restarted && in_window && !working {
            AfterCrash::StartAgain
        } else {
            AfterCrash::SayHere
        }
    }

    /// What the filter does when it is `me` that crashed and `saying` is the thread whose crash
    /// it is saying (0: none).
    #[derive(Debug, PartialEq)]
    pub enum Turn {
        /// Say this crash.
        Say,
        /// A crash on the thread saying one: end the process at once.
        End,
        /// Another thread's crash while one is said: wait for the end.
        Wait,
    }

    pub fn turn(saying: u32, me: u32) -> Turn {
        match saying {
            0 => Turn::Say,
            owner if owner == me => Turn::End,
            _ => Turn::Wait,
        }
    }

    /// Whether this Play.exe was started again by a crash ([`crash_passed_on`]).
    pub fn restarted() -> bool {
        RESTARTED.load(Ordering::SeqCst)
    }

    /// What a crash passes on: its line, then the start-up lines, one a line.
    pub fn crash_value(what: &str, lines: &[String]) -> String {
        std::iter::once(what)
            .chain(lines.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A crash passed on to the Play.exe started again.
    pub struct Crash {
        pub what: String,
        pub lines: Vec<String>,
    }

    impl Crash {
        /// [`crash_value`] read back (each line kept as [`kept`] keeps it).
        pub fn read(value: &str) -> Crash {
            let mut lines = value.lines();
            Crash {
                what: shortened(lines.next().unwrap_or(""), WHAT_CHARS),
                lines: lines
                    .take(LINES_KEPT)
                    .map(|l| shortened(l, LINE_CHARS))
                    .collect(),
            }
        }
    }

    /// Play.exe started again by a crash: the crash it passes on, taken out of the environment
    /// (the game and the tools Play starts never see it); from now on no crash starts Play.exe
    /// again.
    pub fn crash_passed_on() -> Option<Crash> {
        let value = std::env::var_os(CRASH_VAR).filter(|v| !v.is_empty())?;
        std::env::remove_var(CRASH_VAR);
        RESTARTED.store(true, Ordering::SeqCst);
        Some(Crash::read(&value.to_string_lossy()))
    }

    /// Starts `exe` with the command line `command` (each ending in a NUL), with `value` in
    /// [`CRASH_VAR`] (set in this process's own environment, which the new one gets): its
    /// process and thread, to close.
    pub fn spawn_with(exe: &[u16], command: &[u16], value: &str) -> Option<ProcessInformation> {
        let (name, value) = (wide(CRASH_VAR), wide(value));
        let mut command = command.to_vec();
        let null: *mut c_void = std::ptr::null_mut();
        let startup = StartupInfoW {
            size: std::mem::size_of::<StartupInfoW>() as u32,
            reserved: null.cast(),
            desktop: null.cast(),
            title: null.cast(),
            place: [0; 8],
            show_window: 0,
            reserved2_size: 0,
            reserved2: null.cast(),
            std_input: null,
            std_output: null,
            std_error: null,
        };
        let mut started = ProcessInformation {
            process: null,
            thread: null,
            process_id: 0,
            thread_id: 0,
        };
        // SAFETY: every string ends in a NUL and lives through the calls; the command line is a
        // copy Windows may write into; the structures are this function's.
        unsafe {
            if SetEnvironmentVariableW(name.as_ptr(), value.as_ptr()) == 0 {
                return None;
            }
            let ok = CreateProcessW(
                exe.as_ptr(),
                command.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                0,
                std::ptr::null(),
                std::ptr::null(),
                &startup,
                &mut started,
            );
            (ok != 0).then_some(started)
        }
    }

    /// Windows' last chance for a crash nothing else handled in this process: said, then the
    /// process ends. In the window's run, Play.exe is started again to say it and play in
    /// message boxes ([`after_crash`]); otherwise (or if it cannot start) a message box here.
    unsafe extern "system" fn last_chance(pointers: *const ExceptionPointers) -> i32 {
        let (in_window, working) = (in_window(), working());
        let after = after_crash(leaving(), restarted(), in_window, working);
        if after == AfterCrash::AsBefore {
            return EXCEPTION_CONTINUE_SEARCH;
        }
        // SAFETY: no pointers.
        let me = unsafe { GetCurrentThreadId() };
        let saying = SAYING
            .compare_exchange(0, me, Ordering::SeqCst, Ordering::SeqCst)
            .unwrap_or_else(|owner| owner);
        match turn(saying, me) {
            Turn::Say => {}
            Turn::End => return EXCEPTION_EXECUTE_HANDLER,
            Turn::Wait => {
                for _ in 0..WAIT_STEPS {
                    if SHOWING.load(Ordering::SeqCst) {
                        // SAFETY: no pointers; the saying thread ends the process.
                        loop {
                            unsafe { Sleep(u32::MAX) };
                        }
                    }
                    // SAFETY: no pointers.
                    unsafe { Sleep(WAIT_STEP_MS) };
                }
                return EXCEPTION_EXECUTE_HANDLER;
            }
        }
        // SAFETY: Windows passes the crash's record, or nothing.
        let (code, address) = match unsafe { pointers.as_ref().and_then(|p| p.record.as_ref()) } {
            Some(r) => (r.code, r.address as usize),
            None => (0, 0),
        };
        let module = module_of(address);
        let at = module
            .as_ref()
            .map(|(file, base)| (file.as_str(), address.wrapping_sub(*base)));
        let what = crash_line(code, at);
        // The start-up lines unless the crash came while they were being kept.
        let lines: Vec<String> = LINES
            .try_lock()
            .map(|l| l.iter().cloned().collect())
            .unwrap_or_default();
        if after == AfterCrash::StartAgain {
            if let (Some(exe), Some(command)) = (EXE.get(), COMMAND.get()) {
                if let Some(started) = spawn_with(exe, command, &crash_value(&what, &lines)) {
                    // SAFETY: the new process's handles, closed once.
                    unsafe {
                        CloseHandle(started.thread);
                        CloseHandle(started.process);
                    }
                    return EXCEPTION_EXECUTE_HANDLER;
                }
            }
        }
        let next = if working {
            "Play was running: the game, if it had started, goes on. Start Play.exe again once it \
             has closed."
                .to_string()
        } else {
            format!("Start Play.exe again to try once more.\n\n{}", report())
        };
        let text = stopped_text(in_window, &what, &lines, &next);
        SHOWING.store(true, Ordering::SeqCst);
        message_box(&text, 0x1_0010);
        EXCEPTION_EXECUTE_HANDLER
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[link(name = "kernel32")]
        extern "system" {
            fn GetModuleHandleW(name: *const u16) -> *mut c_void;
            fn GetProcAddress(module: *mut c_void, name: *const u8) -> *const c_void;
            fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
            fn GetExitCodeProcess(process: *mut c_void, code: *mut u32) -> i32;
        }

        /// The crash's line names the file, how far into it, the exception and its code.
        #[test]
        fn a_crash_names_the_exception_and_the_file() {
            assert_eq!(
                crash_line(0xc000_0005, Some(("overlay64.dll", 0x1a_2b3c))),
                "it crashed in overlay64.dll+0x1a2b3c (an access violation, 0xc0000005)"
            );
            assert_eq!(
                crash_line(0x8000_0003, Some(("Play.exe", 0x10))),
                "it crashed in Play.exe+0x10 (a breakpoint, 0x80000003)"
            );
            assert_eq!(
                crash_line(0xc000_0135, None),
                "it crashed outside every file it loaded (exception 0xc0000135)"
            );
        }

        /// An address in this test program names this program, which starts before it; one in
        /// Windows' own kernel names that DLL; an address no file holds names none.
        #[test]
        fn an_address_names_the_file_holding_it() {
            let exe = std::env::current_exe().unwrap();
            let name = exe.file_name().unwrap().to_string_lossy().into_owned();
            let here = module_of as *const () as usize;
            let (file, base) = module_of(here).unwrap();
            assert_eq!(file, name);
            assert!(base < here && here - base < 1 << 30, "{base:#x} {here:#x}");
            let kernel = wide("kernel32.dll");
            // SAFETY: NUL-ended names; kernel32 is in every process.
            let at = unsafe {
                GetProcAddress(
                    GetModuleHandleW(kernel.as_ptr()),
                    c"GetModuleFileNameW".as_ptr().cast(),
                )
            };
            assert!(!at.is_null());
            let dll = module_of(at as usize).unwrap().0.to_ascii_lowercase();
            assert!(dll.starts_with("kernel") && dll.ends_with(".dll"), "{dll}");
            assert_eq!(module_of(8), None);
        }

        /// While the launcher ends as asked, a crash is left as before; otherwise only the first
        /// crash in the window's run, with no work running, starts Play.exe again.
        #[test]
        fn only_a_crash_in_the_window_with_nothing_running_starts_play_again() {
            use AfterCrash::*;
            assert_eq!(after_crash(false, false, true, false), StartAgain);
            assert_eq!(
                after_crash(true, false, true, false),
                AsBefore,
                "ending as asked (Exit, Play's good end)"
            );
            assert_eq!(after_crash(true, true, false, true), AsBefore);
            assert_eq!(
                after_crash(false, true, true, false),
                SayHere,
                "started again already"
            );
            assert_eq!(
                after_crash(false, false, true, true),
                SayHere,
                "Play is running"
            );
            assert_eq!(
                after_crash(false, false, false, false),
                SayHere,
                "message-box Play"
            );
            assert_eq!(after_crash(false, true, false, true), SayHere);
        }

        /// The first crashing thread says its crash; a second crash on it ends the process; another
        /// thread's crash waits.
        #[test]
        fn one_thread_says_a_crash_and_others_wait() {
            assert_eq!(turn(0, 7), Turn::Say);
            assert_eq!(turn(7, 7), Turn::End);
            assert_eq!(turn(7, 9), Turn::Wait);
        }

        /// What starts Play.exe again: this program's own path and its whole command line.
        #[test]
        fn play_starts_again_with_its_own_path_and_command_line() {
            let (exe, command) = this_start();
            let exe = String::from_utf16(&exe.unwrap()).unwrap();
            let me = std::env::current_exe().unwrap();
            assert_eq!(exe.trim_end_matches('\0'), me.to_string_lossy());
            assert_eq!(command.last(), Some(&0));
            let command = String::from_utf16(&command).unwrap();
            for arg in std::env::args().skip(1) {
                assert!(command.contains(&arg), "{arg} not in {command}");
            }
        }

        /// The crash reaches the new process in its environment, with the command line given;
        /// the Play.exe started again takes it out of its environment and never starts itself
        /// again.
        #[test]
        fn the_crash_reaches_play_started_again_which_never_starts_again() {
            let cmd = std::env::var("ComSpec").unwrap_or("C:\\Windows\\System32\\cmd.exe".into());
            // cmd compares a plain word: the environment and the command line both arrive.
            let check =
                format!("cmd /d /c if \"%{CRASH_VAR}%\"==\"crashed-x.dll\" (exit 7) else (exit 3)");
            let started = spawn_with(&wide(&cmd), &wide(&check), "crashed-x.dll").expect("started");
            let mut code = 0;
            // SAFETY: the new process's handles, waited on and closed once.
            unsafe {
                assert_eq!(WaitForSingleObject(started.process, 20_000), 0);
                GetExitCodeProcess(started.process, &mut code);
                CloseHandle(started.thread);
                CloseHandle(started.process);
            }
            assert_eq!(code, 7, "the crash's line in the new process's environment");
            // A whole crash's value, read back as the Play.exe started again reads it.
            let lines = vec!["look: the game's".to_string(), "drawing with X".to_string()];
            let what = "it crashed in x.dll (an access violation, 0xc0000005)";
            std::env::set_var(CRASH_VAR, crash_value(what, &lines));
            assert!(!restarted());
            let crash = crash_passed_on().expect("passed on");
            assert_eq!(crash.what, what);
            assert_eq!(crash.lines, lines);
            assert_eq!(std::env::var_os(CRASH_VAR), None, "taken out");
            assert!(restarted());
            assert_eq!(
                after_crash(false, restarted(), true, false),
                AfterCrash::SayHere
            );
        }

        /// A crash's value read back keeps the start-up lines' limits.
        #[test]
        fn a_crash_read_back_keeps_the_limits() {
            let many: Vec<String> = (0..20)
                .map(|i| format!("line {i} {}", "x".repeat(400)))
                .collect();
            let crash = Crash::read(&crash_value("what", &many));
            assert_eq!(crash.what, "what");
            assert_eq!(crash.lines.len(), LINES_KEPT);
            assert!(crash.lines.iter().all(|l| l.chars().count() == LINE_CHARS));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The last few start-up lines are kept, each on one line and short.
    #[test]
    fn the_start_up_lines_keep_the_last_few_each_short() {
        let mut lines = VecDeque::new();
        for i in 0..20 {
            kept(&mut lines, &format!("line {i}"));
        }
        assert_eq!(lines.len(), LINES_KEPT);
        assert_eq!(lines.front().map(String::as_str), Some("line 12"));
        assert_eq!(lines.back().map(String::as_str), Some("line 19"));
        kept(
            &mut lines,
            &format!("cannot draw:\n\n  {}", "y".repeat(1000)),
        );
        let last = lines.back().unwrap();
        assert_eq!(last.chars().count(), LINE_CHARS);
        assert!(last.starts_with("cannot draw: yyy") && last.ends_with("..."));
    }

    /// A panic caught after the hook is watched is named by its message and its place.
    #[test]
    fn a_caught_panic_names_its_message_and_place() {
        watch_panics();
        let payload = std::panic::catch_unwind(|| panic!("made-up stop {}", 7)).unwrap_err();
        let line = panic_line(&*payload);
        assert!(line.starts_with("made-up stop 7 (at "), "{line}");
        assert!(line.contains("stopped.rs:"), "{line}");
        // Taken: a payload without the hook's line is its message alone.
        let other: Box<dyn std::any::Any + Send> = Box::new("plain");
        assert_eq!(panic_line(&*other), "plain");
    }

    /// A panic while the launcher ends as asked ends it quietly (no Play in message boxes);
    /// otherwise running work is waited for, and with none Play goes on without the window.
    #[test]
    fn a_panic_while_leaving_ends_quietly() {
        assert_eq!(after_panic(true, false), AfterPanic::Quiet);
        assert_eq!(after_panic(true, true), AfterPanic::Quiet);
        assert_eq!(after_panic(false, true), AfterPanic::WaitForWork);
        assert_eq!(after_panic(false, false), AfterPanic::PlayWithoutWindow);
    }

    /// The message says what stopped, its last steps and what happens next; a long line is cut.
    #[test]
    fn the_message_says_what_stopped_its_steps_and_what_next() {
        let lines = vec![
            "look: the game's".to_string(),
            "drawing with Card (Dx12)".to_string(),
        ];
        let text = stopped_text(
            true,
            &panic_what("index out of bounds (at crates/nv-launcher/src/screen.rs:10)"),
            &lines,
            &next_play(false),
        );
        assert!(text.starts_with(
            "nv-rs's launcher window stopped: an error in nv-rs's own code: index out of bounds (at crates/nv-launcher/src/screen.rs:10).\n\nIts last steps:\n  look: the game's\n  drawing with Card (Dx12)\n\n"
        ), "{text}");
        assert!(text.contains(ISSUES));
        if cfg!(windows) {
            assert!(text.contains("Windows' own message boxes"));
        }
        assert!(next_play(true).contains("overlay"));
        assert!(next_working().contains("ends when Play ends"));
        let long = stopped_text(false, &"z".repeat(5000), &[], "next");
        assert!(long.starts_with("nv-rs's launcher stopped: zzz"));
        assert!(long.len() < 700, "{}", long.len());
        assert!(long.ends_with("zz...\n\nnext"), "{long}");
    }
}
