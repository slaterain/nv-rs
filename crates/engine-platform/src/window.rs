//! The platform's window side (USER32, SHELL32 and the KERNEL32 pieces `main` uses around its
//! window): the single-instance mutex, the command line, window classes, windows, their styles
//! and borders, and the message queue whose emptiness ends a frame of the game's main loop.
//!
//! The windows are records only: the window on screen is the viewer's. What Windows would answer
//! that depends on the machine (the screen metrics, the style a new window reads back, the border
//! `AdjustWindowRectEx` adds) comes from the player's adapter table (`super::d3d9`, made by
//! `scripts/d3d9-capture` on the player's machine); a style the table doesn't have stops the run.
//!
//! Not provided (a call stops the run, or, where noted, the effect is missing): the messages
//! Windows sends a window's procedure while it is created or moved (`WM_CREATE`,
//! `WM_WINDOWPOSCHANGED`, `WM_ACTIVATE`, ...): the game's procedure (`0086a0a0`) only acts on them
//! once `Main` exists, and `main` makes its window before `Main`, but a window made or moved
//! later gets none of them (recorded in the module's functions). Two messages are posted to the
//! game's queue: `WM_MOUSEMOVE` while the game's mouse is not exclusive and its window active
//! ([`post_mouse_move`]: its menus follow the cursor then), with `DefWindowProcA` for it, and
//! `WM_ACTIVATE` when the real window becomes active again after it was not ([`post_activate`]:
//! the game makes its mouse exclusive again); message boxes (the game shows
//! one only to stop: the run stops with its text); `ExitProcess`; `ShellExecuteA`.

use super::KernelObject;
use crate::{Engine, Ptr};
use std::collections::{BTreeMap, VecDeque};

/// `ERROR_ALREADY_EXISTS`, `ERROR_INVALID_WINDOW_HANDLE`, `ERROR_CLASS_ALREADY_EXISTS`.
pub const ERROR_ALREADY_EXISTS: u32 = 183;
pub const ERROR_INVALID_WINDOW_HANDLE: u32 = 1400;
pub const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;
/// `GWL_STYLE`, `GWL_EXSTYLE`, `GCL_MENUNAME`.
const GWL_STYLE: i32 = -16;
const GWL_EXSTYLE: i32 = -20;
const GCL_MENUNAME: i32 = -8;
/// `WS_CHILD`, `WS_VISIBLE`.
const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
/// `PM_REMOVE`.
const PM_REMOVE: u32 = 1;
/// Class atoms start here (`RegisterClassA`'s answers).
const FIRST_ATOM: u32 = 0xc000;

/// A registered window class.
#[derive(Debug, Clone)]
pub struct Class {
    pub atom: u32,
    pub name: Vec<u8>,
    pub style: u32,
    pub procedure: u32,
    pub instance: u32,
    pub menu_name: u32,
}

/// A window: its class, styles, parent and rectangle (screen or parent coordinates).
#[derive(Debug, Clone)]
pub struct Window {
    pub class: u32,
    pub title: Vec<u8>,
    pub style: u32,
    pub ex_style: u32,
    pub parent: u32,
    pub rect: [i32; 4],
}

/// The window side's state (`State::windows`).
#[derive(Debug, Default)]
pub struct Windows {
    pub classes: Vec<Class>,
    pub windows: BTreeMap<u32, Window>,
    /// Messages posted and not taken yet (`MSG`'s seven words).
    pub queue: VecDeque<[u32; 7]>,
    /// The command line `GetCommandLineW` answers (UTF-16, made once), and where it is.
    pub command_line: Vec<u16>,
    command_line_at: u32,
    /// Frames the main loop ended on an empty queue ([`peek_message_a`]).
    pub frames: u64,
    /// The host posted messages while the main loop handed it its turn, and the first went to the
    /// game at once: the loop drains them before its next frame, so the next empty queue starts
    /// that frame instead of ending another ([`peek_message_a`]).
    frame_begun: bool,
}

fn last_error(e: &mut Engine, code: u32) {
    e.kernel.last_error = code;
}

// Platform: replaces KERNEL32.DLL CreateMutexA (import)
/// `CreateMutexA(security, initial owner, name)`: a new mutex (owned by the calling thread when
/// asked), last error 0; a name already in use answers a new handle to the same mutex with
/// `ERROR_ALREADY_EXISTS` (the game's single-instance check).
pub fn create_mutex_a(e: &mut Engine, _security: u32, initial_owner: u32, name: Ptr) -> u32 {
    let name = if name.is_null() {
        Vec::new()
    } else {
        e.mem.cstr(name.addr())
    };
    let exists = !name.is_empty()
        && e.kernel
            .objects
            .values()
            .any(|o| matches!(o, KernelObject::Mutex { name: n, .. } if *n == name));
    let owner = if initial_owner != 0 && !exists {
        crate::thread_host::current_id(e)
    } else {
        0
    };
    let handle = e.kernel.insert(KernelObject::Mutex {
        name,
        owner,
        count: u32::from(owner != 0),
    });
    last_error(e, if exists { ERROR_ALREADY_EXISTS } else { 0 });
    handle
}

// Platform: replaces USER32.DLL MessageBoxA (import)
/// `MessageBoxA(owner, text, caption, type)`: the game shows one when it gives up (another
/// instance running, a remote session, a failed renderer): the run stops with its text.
pub fn message_box_a(e: &mut Engine, _owner: u32, text: Ptr, caption: Ptr, _kind: u32) -> u32 {
    let text = String::from_utf8_lossy(&e.mem.cstr(text.addr())).into_owned();
    let caption = String::from_utf8_lossy(&e.mem.cstr(caption.addr())).into_owned();
    panic!("MessageBoxA({caption:?}, {text:?}): the game gave up (message boxes not provided)")
}

// Platform: replaces KERNEL32.DLL ExitProcess (import)
/// `ExitProcess(code)`: the game ending its process: the run stops.
pub fn exit_process(_e: &mut Engine, code: u32) {
    panic!("ExitProcess({code}): the game ended its process (not provided)")
}

// Platform: replaces SHELL32.DLL ShellExecuteA (import)
/// `ShellExecuteA`: starting another program is not provided, but for the game's launcher, which
/// `main` starts when the graphics card isn't the one in `sD3DDevice` (`004da570`). There it answers
/// that nothing started. In play `main` never asks for it: the host answers the check by naming
/// the card (`crates/host/src/launcher.rs`), since `main` has stopped its tasklet workers for good
/// before it asks, and a game going on from here waits for them at its first cell (issue #935).
pub fn shell_execute_a(
    e: &mut Engine,
    _w: u32,
    _op: u32,
    file: Ptr,
    _p: u32,
    _d: u32,
    _s: u32,
) -> u32 {
    let file = String::from_utf8_lossy(&e.mem.cstr(file.addr())).into_owned();
    if file.to_ascii_lowercase().ends_with("falloutnvlauncher.exe") {
        eprintln!(
            "The game asked for its launcher (a new graphics card or no settings): not started"
        );
        return 0;
    }
    panic!("ShellExecuteA({file:?}) is not provided")
}

// Platform: replaces ADVAPI32.DLL GetUserNameA (import)
/// `GetUserNameA(buffer, size)`: the player's user name ([`user_name`]), zero-terminated, with
/// `*size` set to its length plus one; false with `ERROR_INSUFFICIENT_BUFFER` (122) and the size
/// needed when it doesn't fit. Its one reader in the exe (`004b4a30`) writes it into the game's
/// own error log.
pub fn get_user_name_a(e: &mut Engine, buffer: Ptr, size: Ptr) -> u32 {
    let name = user_name(|key| std::env::var(key).ok()).into_bytes();
    let needed = name.len() as u32 + 1;
    let room = e.mem.u32(size.addr());
    e.mem.set_u32(size.addr(), needed);
    if room < needed {
        last_error(e, 122);
        return 0;
    }
    e.mem.set_cstr(buffer.addr(), &name);
    1
}

/// The user name the process runs with, from the variables `var` reads: Windows' `USERNAME`,
/// else `USER` (what Linux and macOS set; Windows has no `USER`), else empty.
fn user_name(var: impl Fn(&str) -> Option<String>) -> String {
    ["USERNAME", "USER"]
        .into_iter()
        .find_map(|key| var(key).filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

/// Windows' pointer speed for the game's menu cursor (`engine_core`'s `Kernel::pointer_speed`,
/// which the game reads as `HKCU\Control Panel\Mouse` `MouseSensitivity` at its input start), with
/// the log line saying it: on Windows the player's own, read once as the game reads it
/// (`RegQueryValueExA`: the value's type and bytes, or the error Windows gives; only read, nothing
/// is written); elsewhere `None`, the 10 a plain Windows machine starts with.
pub fn pointer_speed() -> (Option<crate::PointerSpeed>, String) {
    #[cfg(windows)]
    let read = read_pointer_speed().map_err(Some);
    #[cfg(not(windows))]
    let read = Err(None);
    pointer_speed_of(read)
}

/// `REG_SZ`: a string value.
const REG_SZ: u32 = 1;

/// [`pointer_speed`]'s answer and line for what the read gave: the value's type and bytes, or why
/// there is none (Windows' error code, which the game is answered too, so it keeps its own 6 as
/// in the original; `None`: not Windows). A string that is not a whole number from 1 to 20 is
/// still the game's (it reads it with `atol`), and so is a value of another type; the line says so.
fn pointer_speed_of(
    read: Result<(u32, Vec<u8>), Option<u32>>,
) -> (Option<crate::PointerSpeed>, String) {
    const FOLLOWS: &str = "the game's menu cursor speed follows it, as in the original";
    match read {
        Ok((kind, data)) if kind == REG_SZ => {
            let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
            let shown = String::from_utf8_lossy(&data[..end]).into_owned();
            let usual = shown
                .trim()
                .parse::<u32>()
                .is_ok_and(|v| (1..=20).contains(&v));
            let note = if usual {
                ""
            } else {
                " (not a speed from 1 to 20: the game reads it as Windows holds it)"
            };
            (
                Some(crate::PointerSpeed::Value { kind, data }),
                format!(
                    "input: Windows' pointer speed (Mouse settings) is {shown}{note}: {FOLLOWS}"
                ),
            )
        }
        Ok((kind, data)) => (
            Some(crate::PointerSpeed::Value { kind, data }),
            format!(
                "input: Windows' pointer speed (Mouse settings) is stored as registry type {kind}, \
                 not as text: the game reads its bytes as Windows hands them, as the original does"
            ),
        ),
        Err(Some(code)) => (
            Some(crate::PointerSpeed::Missing(code)),
            format!(
                "input: Windows' pointer speed could not be read (error {code}{}): the game keeps \
                 its own menu cursor speed (6), as the original does",
                if code == 2 { ": it is not set" } else { "" }
            ),
        ),
        Err(None) => (
            None,
            "input: the menus' cursor takes Windows' default pointer speed, 10, as a Windows \
             machine has it"
                .to_string(),
        ),
    }
}

/// `HKCU\Control Panel\Mouse` `MouseSensitivity` read as the game reads it (`RegOpenKeyExA`, then
/// `RegQueryValueExA` into a buffer larger than any pointer speed): the value's type and bytes,
/// or Windows' error code.
#[cfg(windows)]
fn read_pointer_speed() -> Result<(u32, Vec<u8>), u32> {
    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExA(
            key: isize,
            sub_key: *const u8,
            options: u32,
            access: u32,
            opened: *mut isize,
        ) -> i32;
        fn RegQueryValueExA(
            key: isize,
            value: *const u8,
            reserved: *mut u32,
            kind: *mut u32,
            data: *mut u8,
            size: *mut u32,
        ) -> i32;
        fn RegCloseKey(key: isize) -> i32;
    }
    const HKEY_CURRENT_USER: isize = 0x8000_0001_u32 as i32 as isize;
    const KEY_READ: u32 = 0x2_0019;
    let mut key = 0isize;
    // SAFETY: the key and value names are zero-terminated, the out words and the buffer are this
    // function's own with their sizes, and the calls only read the registry.
    unsafe {
        let opened = RegOpenKeyExA(
            HKEY_CURRENT_USER,
            b"Control Panel\\Mouse\0".as_ptr(),
            0,
            KEY_READ,
            &mut key,
        );
        if opened != 0 {
            return Err(opened as u32);
        }
        let mut data = [0u8; 256];
        let mut size = data.len() as u32;
        let mut kind = 0u32;
        let status = RegQueryValueExA(
            key,
            b"MouseSensitivity\0".as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            data.as_mut_ptr(),
            &mut size,
        );
        RegCloseKey(key);
        if status != 0 {
            return Err(status as u32);
        }
        Ok((kind, data[..(size as usize).min(data.len())].to_vec()))
    }
}

// Platform: replaces STEAM_API.DLL SteamAPI_Init (import)
/// `SteamAPI_Init()`: false, the answer when no Steam client runs: the Steam client is not
/// provided, and the game's own code goes on without it (`006ff690` keeps the false at +0x3c of
/// its Steam object and skips `SteamUtils`/`SteamUserStats`).
pub fn steam_api_init(_e: &mut Engine) -> u32 {
    0
}

// Platform: replaces STEAM_API.DLL SteamAPI_RegisterCallback (import)
/// `SteamAPI_RegisterCallback(callback, id)` (`cdecl`, two words, no answer the game reads):
/// the game registers its Steam callbacks before `SteamAPI_Init` (`006ff690` through
/// `006ffb20`/`006ffb90`/`006ffc20`, id 0x44d and others). With no Steam client nothing ever
/// calls them back, so nothing is kept.
pub fn steam_api_register_callback(_e: &mut Engine, _callback: Ptr, _id: u32) {}

// Platform: replaces STEAM_API.DLL SteamAPI_UnregisterCallback (import)
/// `SteamAPI_UnregisterCallback(callback)` (`cdecl`, one word; `006ffc00`): nothing was kept.
pub fn steam_api_unregister_callback(_e: &mut Engine, _callback: Ptr) {}

// Platform: replaces STEAM_API.DLL SteamAPI_IsSteamRunning (import)
/// `SteamAPI_IsSteamRunning()` (`cdecl`, no arguments, a `bool` in `AL`): false, the answer of
/// the state this layer keeps, no Steam client (as [`steam_api_init`] answers). Not measured in
/// that state: a Steam client ran on the measuring machine, and the player's `steam_api.dll`
/// answered 1 there (2026-10-27, `build43-scratch/measure/platform5`).
pub fn steam_api_is_steam_running(_e: &mut Engine) -> u32 {
    0
}

// Platform: replaces STEAM_API.DLL SteamUserStats (import)
/// `SteamUserStats()` (`cdecl`, no arguments): null. Measured (2026-10-27,
/// `build43-scratch/measure/platform5`): the player's `steam_api.dll` in a 32-bit process
/// answers null for `SteamUserStats`, `SteamFriends`, `SteamUtils`, `SteamUser` and `SteamApps`
/// while `SteamAPI_Init` has not succeeded, the state this layer keeps.
pub fn steam_user_stats(_e: &mut Engine) -> u32 {
    0
}

// Platform: replaces STEAM_API.DLL SteamFriends (import)
/// `SteamFriends()`: null, as measured ([`steam_user_stats`]).
pub fn steam_friends(_e: &mut Engine) -> u32 {
    0
}

// Platform: replaces STEAM_API.DLL SteamUtils (import)
/// `SteamUtils()`: null, as measured ([`steam_user_stats`]).
pub fn steam_utils(_e: &mut Engine) -> u32 {
    0
}

// Platform: replaces STEAM_API.DLL SteamUser (import)
/// `SteamUser()`: null, as measured ([`steam_user_stats`]).
pub fn steam_user(_e: &mut Engine) -> u32 {
    0
}

// Platform: replaces STEAM_API.DLL SteamAPI_RunCallbacks (import)
/// `SteamAPI_RunCallbacks()` (`cdecl`, no arguments, no answer the game reads; called every frame,
/// `006ff860`): does nothing in the state this layer keeps (no successful `SteamAPI_Init`).
/// Measured (2026-10-27, `build43-scratch/measure/platform5`): the player's `steam_api.dll` in a
/// 32-bit process without `SteamAPI_Init` returned from 1000 calls with `EAX` 0, the last error
/// left as set, and `SteamUserStats` still null afterwards.
pub fn steam_api_run_callbacks(_e: &mut Engine) {}

// Platform: replaces STEAM_API.DLL SteamAPI_Shutdown (import)
/// `SteamAPI_Shutdown()` (`cdecl`, no arguments, no answer the game reads; called once at exit,
/// `006ff7b0`): does nothing in the state this layer keeps (no successful `SteamAPI_Init`).
/// Measured (2026-10-30, `build43-scratch/measure/platform5`, `shutdown`): the player's
/// `steam_api.dll` in a 32-bit process without `SteamAPI_Init` returned from two calls with
/// `EAX` 0 and the last error left as set; `SteamAPI_RunCallbacks` and `SteamUserStats` answered
/// as before afterwards.
pub fn steam_api_shutdown(_e: &mut Engine) {}

// Platform: replaces STEAM_API.DLL SteamApps (import)
/// `SteamApps()`: null, as measured ([`steam_user_stats`]).
pub fn steam_apps(_e: &mut Engine) -> u32 {
    0
}

// Platform: replaces KERNEL32.DLL IsDebuggerPresent (import)
/// `IsDebuggerPresent()`: no debugger.
pub fn is_debugger_present(_e: &mut Engine) -> u32 {
    0
}

fn class_name_matches(e: &Engine, class: &Class, name: u32) -> bool {
    if name < 0x1_0000 {
        return class.atom == name;
    }
    class.name.eq_ignore_ascii_case(&e.mem.cstr(name))
}

// Platform: replaces USER32.DLL FindWindowA (import)
/// `FindWindowA(class, title)`: the first top-level window of this process with that class (and
/// title, when given); 0 when there is none. Only this process's windows exist here.
pub fn find_window_a(e: &mut Engine, class: u32, title: Ptr) -> u32 {
    let title = (!title.is_null()).then(|| e.mem.cstr(title.addr()));
    let found = state_ref!(e).windows.windows.iter().find(|(_, w)| {
        let class_ok = class == 0
            || state_ref!(e)
                .windows
                .classes
                .iter()
                .any(|c| c.atom == w.class && class_name_matches(e, c, class));
        w.parent == 0 && class_ok && title.as_ref().map_or(true, |t| *t == w.title)
    });
    found.map_or(0, |(&h, _)| h)
}

// Platform: replaces USER32.DLL SetForegroundWindow (import)
/// `SetForegroundWindow(window)`: true for a window of this layer.
pub fn set_foreground_window(e: &mut Engine, window: u32) -> u32 {
    u32::from(state!(e).windows.windows.contains_key(&window))
}

// Platform: replaces KERNEL32.DLL GetCommandLineW (import)
/// `GetCommandLineW()`: the process's command line, made once: the executable's path, quoted,
/// as Steam starts the game (`Kernel::current_directory` + `FalloutNV.exe`), no arguments.
pub fn get_command_line_w(e: &mut Engine) -> u32 {
    if state!(e).windows.command_line_at == 0 {
        if state!(e).windows.command_line.is_empty() {
            let mut text = String::from("\"");
            text.push_str(&String::from_utf8_lossy(&e.kernel.current_directory));
            if !text.ends_with('\\') && text.len() > 1 {
                text.push('\\');
            }
            text.push_str("FalloutNV.exe\"");
            state!(e).windows.command_line = text.encode_utf16().collect();
        }
        let units = state!(e).windows.command_line.clone();
        let at = e.mem.alloc(2 * (units.len() as u32 + 1));
        for (i, u) in units.iter().enumerate() {
            e.mem.set_u16(at + 2 * i as u32, *u);
        }
        e.mem.set_u16(at + 2 * units.len() as u32, 0);
        state!(e).windows.command_line_at = at;
    }
    state!(e).windows.command_line_at
}

/// A NUL-terminated UTF-16 string's code units.
fn wide(e: &Engine, mut at: u32) -> Vec<u16> {
    let mut out = Vec::new();
    loop {
        let u = e.mem.u16(at);
        if u == 0 {
            return out;
        }
        out.push(u);
        at += 2;
    }
}

/// Splits a command line as `CommandLineToArgvW` does: the program name runs to the closing
/// quote (when it starts with one) or the first space or tab, with no escapes; after it,
/// arguments are separated by spaces and tabs, quotes group, `2n` backslashes before a quote
/// become `n` and the quote groups, `2n+1` become `n` and a literal quote, a doubled quote inside
/// a quoted part is a literal quote, other backslashes are literal.
pub fn split_command_line(line: &[u16]) -> Vec<Vec<u16>> {
    const QUOTE: u16 = b'"' as u16;
    const BACKSLASH: u16 = b'\\' as u16;
    let blank = |u: u16| u == b' ' as u16 || u == b'\t' as u16;
    let mut args = Vec::new();
    let mut i = 0;
    let mut name = Vec::new();
    if line.first() == Some(&QUOTE) {
        i = 1;
        while i < line.len() && line[i] != QUOTE {
            name.push(line[i]);
            i += 1;
        }
        i += 1;
    } else {
        while i < line.len() && !blank(line[i]) {
            name.push(line[i]);
            i += 1;
        }
    }
    args.push(name);
    loop {
        while i < line.len() && blank(line[i]) {
            i += 1;
        }
        if i >= line.len() {
            return args;
        }
        let mut arg = Vec::new();
        let mut quoted = false;
        while i < line.len() && (quoted || !blank(line[i])) {
            if line[i] == BACKSLASH {
                let start = i;
                while i < line.len() && line[i] == BACKSLASH {
                    i += 1;
                }
                let n = i - start;
                if i < line.len() && line[i] == QUOTE {
                    arg.extend(std::iter::repeat(BACKSLASH).take(n / 2));
                    if n % 2 == 1 {
                        arg.push(QUOTE);
                        i += 1;
                    }
                } else {
                    arg.extend(std::iter::repeat(BACKSLASH).take(n));
                }
            } else if line[i] == QUOTE {
                if quoted && line.get(i + 1) == Some(&QUOTE) {
                    arg.push(QUOTE);
                    i += 2;
                } else {
                    quoted = !quoted;
                    i += 1;
                }
            } else {
                arg.push(line[i]);
                i += 1;
            }
        }
        args.push(arg);
    }
}

// Platform: replaces SHELL32.DLL CommandLineToArgvW (import)
/// `CommandLineToArgvW(line, count out)`: the arguments ([`split_command_line`]) in one block
/// (the pointer array, then the strings) to be freed with `LocalFree`; the count through
/// `count`. An empty line is not provided (Windows answers the program's path then).
pub fn command_line_to_argv_w(e: &mut Engine, line: Ptr, count: Ptr) -> u32 {
    let units = wide(e, line.addr());
    assert!(
        !units.is_empty(),
        "CommandLineToArgvW with an empty line is not provided"
    );
    let args = split_command_line(&units);
    let pointers = 4 * args.len() as u32;
    let strings: u32 = args.iter().map(|a| 2 * (a.len() as u32 + 1)).sum();
    let block = e.mem.alloc(pointers + strings);
    let mut at = block + pointers;
    for (i, arg) in args.iter().enumerate() {
        e.mem.set_u32(block + 4 * i as u32, at);
        for u in arg {
            e.mem.set_u16(at, *u);
            at += 2;
        }
        e.mem.set_u16(at, 0);
        at += 2;
    }
    if !count.is_null() {
        e.mem.set_u32(count.addr(), args.len() as u32);
    }
    block
}

// Platform: replaces KERNEL32.DLL LocalFree (import)
/// `LocalFree(block)`: frees a block this layer handed out; answers 0 (success).
pub fn local_free(e: &mut Engine, block: u32) -> u32 {
    if block != 0 {
        e.mem.free(block);
    }
    0
}

// Platform: replaces USER32.DLL LoadIconA (import)
/// `LoadIconA(instance, name)`: a handle for the icon resource (the icon itself isn't loaded:
/// the viewer's window has its own).
pub fn load_icon_a(e: &mut Engine, _instance: u32, _name: u32) -> u32 {
    e.kernel.insert(KernelObject::UserObject("icon"))
}

// Platform: replaces GDI32.DLL GetStockObject (import)
/// `GetStockObject(index)`: the stock object's handle (the same one for the same index).
pub fn get_stock_object(e: &mut Engine, index: u32) -> u32 {
    let name: &'static str = match index {
        0 => "white brush",
        1 => "light gray brush",
        2 => "gray brush",
        3 => "dark gray brush",
        4 => "black brush",
        5 => "null brush",
        _ => panic!("GetStockObject({index}) is not provided"),
    };
    if let Some((&h, _)) = e
        .kernel
        .objects
        .iter()
        .find(|(_, o)| **o == KernelObject::UserObject(name))
    {
        return h;
    }
    e.kernel.insert(KernelObject::UserObject(name))
}

// Platform: replaces USER32.DLL RegisterClassA (import)
/// `RegisterClassA(class)`: records the `WNDCLASSA` (style, procedure, instance, menu name,
/// name) and answers its atom; 0 with `ERROR_CLASS_ALREADY_EXISTS` for a name in use.
pub fn register_class_a(e: &mut Engine, class: Ptr) -> u32 {
    let c = class.addr();
    let name_word = e.mem.u32(c + 36);
    assert!(
        name_word >= 0x1_0000,
        "RegisterClassA by atom is not provided"
    );
    let name = e.mem.cstr(name_word);
    if state!(e)
        .windows
        .classes
        .iter()
        .any(|k| k.name.eq_ignore_ascii_case(&name))
    {
        last_error(e, ERROR_CLASS_ALREADY_EXISTS);
        return 0;
    }
    let atom = FIRST_ATOM + state!(e).windows.classes.len() as u32;
    let record = Class {
        atom,
        name,
        style: e.mem.u32(c),
        procedure: e.mem.u32(c + 4),
        instance: e.mem.u32(c + 16),
        menu_name: e.mem.u32(c + 32),
    };
    state!(e).windows.classes.push(record);
    atom
}

/// The border `AdjustWindowRectEx` adds for this style, menu and extended style, from the table.
fn border(e: &mut Engine, style: u32, menu: u32, ex_style: u32) -> [i32; 4] {
    match super::d3d9::window_border(e, style, u32::from(menu != 0), ex_style) {
        Some(b) => b,
        None => panic!(
            "AdjustWindowRectEx(style {style:#x}, menu {menu}, ex style {ex_style:#x}) was not captured in the adapter table: not provided"
        ),
    }
}

/// The frame the game adds to the client area it wants before it places its window `handle`: the
/// renderer's start (exe 0x004da951) calls `AdjustWindowRect` with the window's style
/// (`GetWindowLongA(GWL_STYLE)`, the record's) and, as `bMenu`, its class's menu name
/// (`GetClassLongA(GCL_MENUNAME)`), then `SetWindowPos` with the grown rectangle. `None` when the
/// window is unknown or the table has no border for that style.
#[cfg_attr(windows, allow(dead_code))]
pub(crate) fn game_frame(e: &mut Engine, handle: u32) -> Option<[i32; 4]> {
    let w = window(e, handle)?.clone();
    let menu = state!(e)
        .windows
        .classes
        .iter()
        .find(|c| c.atom == w.class)
        .map_or(0, |c| c.menu_name);
    super::d3d9::window_border(e, w.style, u32::from(menu != 0), 0)
}

// Platform: replaces USER32.DLL AdjustWindowRectEx (import)
/// `AdjustWindowRectEx(rect, style, menu, ex style)`: the rectangle grown by the window border
/// Windows adds for that style (captured, [`border`]); true.
pub fn adjust_window_rect_ex(
    e: &mut Engine,
    rect: Ptr,
    style: u32,
    menu: u32,
    ex_style: u32,
) -> u32 {
    let b = border(e, style, menu, ex_style);
    for (i, d) in b.iter().enumerate() {
        let at = rect.addr() + 4 * i as u32;
        let v = e.mem.i32(at).wrapping_add(*d);
        e.mem.set_i32(at, v);
    }
    1
}

// Platform: replaces USER32.DLL AdjustWindowRect (import)
/// `AdjustWindowRect(rect, style, menu)`: `AdjustWindowRectEx` with no extended style.
pub fn adjust_window_rect(e: &mut Engine, rect: Ptr, style: u32, menu: u32) -> u32 {
    adjust_window_rect_ex(e, rect, style, menu, 0)
}

// Platform: replaces USER32.DLL CreateWindowExA (import)
/// `CreateWindowExA(ex style, class, title, style, x, y, width, height, parent, menu, instance,
/// parameter)`: a window record of a registered class (0 with `ERROR_CANNOT_FIND_WND_CLASS`
/// otherwise), with the style and extended style Windows gives it (captured: a top-level
/// `WS_VISIBLE` window also gets `WS_CLIPSIBLINGS | WS_CAPTION` and `WS_EX_WINDOWEDGE`); the
/// first visible top-level window becomes the active one. The creation messages are not sent
/// (module documentation).
#[allow(clippy::too_many_arguments)]
pub fn create_window_ex_a(
    e: &mut Engine,
    ex_style: u32,
    class: u32,
    title: Ptr,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: u32,
    _menu: u32,
    _instance: u32,
    _parameter: u32,
) -> u32 {
    let Some(atom) = state_ref!(e)
        .windows
        .classes
        .iter()
        .find(|c| class_name_matches(e, c, class))
        .map(|c| c.atom)
    else {
        last_error(e, 1407);
        return 0;
    };
    let child = style & WS_CHILD != 0;
    let (style, ex_style) = match super::d3d9::window_style(e, style, u32::from(child)) {
        Some((s, x)) => (s, x | ex_style),
        None => panic!(
            "CreateWindowExA with style {style:#x} ({}) was not captured in the adapter table: not provided",
            if child { "child" } else { "top-level" }
        ),
    };
    let title = if title.is_null() {
        Vec::new()
    } else {
        e.mem.cstr(title.addr())
    };
    let handle = e.kernel.insert(KernelObject::Window);
    let rect = [x, y, x.wrapping_add(width), y.wrapping_add(height)];
    super::d3d9real::window_created(
        e,
        handle,
        &title,
        style,
        ex_style,
        rect,
        if child { parent } else { 0 },
    );
    state!(e).windows.windows.insert(
        handle,
        Window {
            class: atom,
            title,
            style,
            ex_style,
            parent,
            rect,
        },
    );
    if !child && style & WS_VISIBLE != 0 && e.kernel.main_window == 0 {
        e.kernel.main_window = handle;
    }
    last_error(e, 0);
    handle
}

fn window(e: &mut Engine, handle: u32) -> Option<&mut Window> {
    state!(e).windows.windows.get_mut(&handle)
}

// Platform: replaces USER32.DLL GetWindowLongA (import)
/// `GetWindowLongA(window, index)`: its style (`GWL_STYLE`) or extended style (`GWL_EXSTYLE`);
/// 0 with `ERROR_INVALID_WINDOW_HANDLE` for another handle. Other indexes are not provided.
pub fn get_window_long_a(e: &mut Engine, handle: u32, index: i32) -> u32 {
    let Some(w) = window(e, handle).cloned() else {
        last_error(e, ERROR_INVALID_WINDOW_HANDLE);
        return 0;
    };
    match index {
        GWL_STYLE => w.style,
        GWL_EXSTYLE => w.ex_style,
        _ => panic!("GetWindowLongA({index}) is not provided"),
    }
}

// Platform: replaces USER32.DLL GetClassLongA (import)
/// `GetClassLongA(window, index)`: the window's class menu name (`GCL_MENUNAME`); other indexes
/// are not provided.
pub fn get_class_long_a(e: &mut Engine, handle: u32, index: i32) -> u32 {
    let Some(w) = window(e, handle).cloned() else {
        last_error(e, ERROR_INVALID_WINDOW_HANDLE);
        return 0;
    };
    let class = state!(e)
        .windows
        .classes
        .iter()
        .find(|c| c.atom == w.class)
        .cloned()
        .expect("the window's class");
    match index {
        GCL_MENUNAME => class.menu_name,
        _ => panic!("GetClassLongA({index}) is not provided"),
    }
}

// Platform: replaces USER32.DLL SetWindowTextA (import)
/// `SetWindowTextA(window, text)`: the record's title becomes the zero-terminated ANSI `text`;
/// true (documented: nonzero on success). A handle that is no window answers `FALSE` with
/// `ERROR_INVALID_WINDOW_HANDLE`. With the real window (`NV_D3D9=real`) its title changes too.
/// The `WM_SETTEXT` it would send is not sent (module documentation); a null `text` (not
/// documented) is not provided. The model loader's status (`00446cb0`) titles the window with it.
pub fn set_window_text_a(e: &mut Engine, handle: u32, text: Ptr) -> u32 {
    assert!(
        !text.is_null(),
        "SetWindowTextA with a null text is not provided"
    );
    let title = e.mem.cstr(text.addr());
    let Some(w) = window(e, handle) else {
        last_error(e, ERROR_INVALID_WINDOW_HANDLE);
        return 0;
    };
    w.title = title.clone();
    super::d3d9real::window_titled(e, handle, &title);
    1
}

// Platform: replaces USER32.DLL SetWindowPos (import)
/// `SetWindowPos(window, after, x, y, width, height, flags)`: moves and sizes the record
/// (`SWP_NOSIZE` 1 and `SWP_NOMOVE` 2 honoured; the z order is not kept); true. The messages it
/// would send are not sent (module documentation).
#[allow(clippy::too_many_arguments)]
pub fn set_window_pos(
    e: &mut Engine,
    handle: u32,
    _after: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    flags: u32,
) -> u32 {
    let Some(w) = window(e, handle) else {
        last_error(e, ERROR_INVALID_WINDOW_HANDLE);
        return 0;
    };
    let [left, top, right, bottom] = w.rect;
    let (mut size_w, mut size_h) = (right.wrapping_sub(left), bottom.wrapping_sub(top));
    let (mut nx, mut ny) = (left, top);
    if flags & 2 == 0 {
        nx = x;
        ny = y;
    }
    if flags & 1 == 0 {
        size_w = width;
        size_h = height;
    }
    w.rect = [nx, ny, nx.wrapping_add(size_w), ny.wrapping_add(size_h)];
    {
        let rect = w.rect;
        super::d3d9real::window_moved(e, handle, rect, flags);
    }
    1
}

// Platform: replaces USER32.DLL PeekMessageA (import)
/// `PeekMessageA(msg, window, first, last, remove)`: the next posted message into `msg`
/// (removed with `PM_REMOVE`), true; with none, the frame ends: the game's main thread hands
/// the engine back to the host until its next frame (`thread_host::end_main_frame`). The game's
/// loop empties the queue and then runs its frame, so a message the host posted in its turn (the
/// cursor's move, an activation) is handed over then, true, and the next empty queue starts that
/// frame, false, without ending another: the menus' cursor is placed before the frame that shows
/// it, as in the original. Filtering by window or message range is not provided.
pub fn peek_message_a(
    e: &mut Engine,
    msg: Ptr,
    window: u32,
    first: u32,
    last: u32,
    remove: u32,
) -> u32 {
    assert!(
        window == 0 && first == 0 && last == 0,
        "PeekMessageA with a filter is not provided"
    );
    peek_with(e, msg, remove, crate::thread_host::end_main_frame)
}

/// [`peek_message_a`] with the host's turn `hand_back` (true when the host had one).
fn peek_with(e: &mut Engine, msg: Ptr, remove: u32, hand_back: fn(&mut Engine) -> bool) -> u32 {
    // The message handed out last, if any, was run by the game's procedure since (the hitch log's split of the
    // host's time between frames).
    engine_core::stage_time::message_peek();
    if take_message(e, msg, remove) {
        handed_out(e, msg, remove);
        return 1;
    }
    if std::mem::take(&mut state!(e).windows.frame_begun) {
        return 0;
    }
    state!(e).windows.frames += 1;
    if hand_back(e) && take_message(e, msg, remove) {
        state!(e).windows.frame_begun = true;
        handed_out(e, msg, remove);
        return 1;
    }
    0
}

/// A message [`peek_with`] put in `msg`: removed (`PM_REMOVE`), the game runs it next; its id is noted for the
/// hitch log's split of the host's time between frames (`engine_core::stage_time::message_out`).
fn handed_out(e: &Engine, msg: Ptr, remove: u32) {
    if remove & PM_REMOVE != 0 && engine_core::stage_time::points_on() {
        engine_core::stage_time::message_out(e.mem.u32(msg.addr() + 4));
    }
}

/// The next posted message into `msg` (removed with `PM_REMOVE`); false with none.
fn take_message(e: &mut Engine, msg: Ptr, remove: u32) -> bool {
    let next = if remove & PM_REMOVE != 0 {
        state!(e).windows.queue.pop_front()
    } else {
        state!(e).windows.queue.front().copied()
    };
    let Some(words) = next else {
        return false;
    };
    for (i, w) in words.iter().enumerate() {
        e.mem.set_u32(msg.addr() + 4 * i as u32, *w);
    }
    true
}

/// `WM_MOUSEMOVE`.
pub const WM_MOUSEMOVE: u32 = 0x200;
/// The `MK_*` bits of a mouse message's `wParam`, by DirectInput's button order (left, right,
/// middle, X1, X2).
const MK_BUTTONS: [u32; 5] = [0x1, 0x2, 0x10, 0x20, 0x40];

/// The window the game draws in, which gets the mouse's messages over its client area: the
/// renderer's child of the game's main window (a window that is not full screen), else the main
/// window itself (full screen); with its size.
fn mouse_window(e: &mut Engine) -> Option<(u32, i32, i32)> {
    let main = e.kernel.main_window;
    if main == 0 {
        return None;
    }
    let windows = &state_ref!(e).windows.windows;
    let (handle, w) = windows
        .iter()
        .find(|(_, w)| w.parent == main && w.style & WS_CHILD != 0)
        .or_else(|| windows.get_key_value(&main))?;
    let [left, top, right, bottom] = w.rect;
    Some((*handle, right.wrapping_sub(left), bottom.wrapping_sub(top)))
}

/// The renderer's width and height (`004dc1f0` and `004dc200` read them), which the game's
/// `00871d10` divides a `WM_MOUSEMOVE`'s place by.
const RENDER_WIDTH: u32 = 0x0118_947c;
const RENDER_HEIGHT: u32 = 0x0118_9480;

/// The size a `WM_MOUSEMOVE`'s place is given in: the renderer's (what the game divides it by),
/// once it is known; before, the window's record (`window`'s width and height).
fn move_area(e: &Engine, window: (i32, i32)) -> (i32, i32) {
    let read = |at: u32| {
        e.mem
            .is_mapped(at)
            .then(|| e.mem.u32(at) as i32)
            .filter(|&v| v > 0)
    };
    match (read(RENDER_WIDTH), read(RENDER_HEIGHT)) {
        (Some(w), Some(h)) => (w, h),
        _ => window,
    }
}

/// The cursor moved over the game's window, to `at` (its client area's fractions across and down,
/// `crate::d3d9real::Input::pointer`), with the mouse's `buttons` held (DirectInput's order):
/// the `WM_MOUSEMOVE` Windows would post there goes into the game's queue, its place in pixels of
/// the renderer's size, which the game divides it by ([`move_area`]), whatever size the real
/// window has (the port draws any size into the real window, scaled). Windows keeps one
/// `WM_MOUSEMOVE` waiting, made from the cursor's latest place: a newer move replaces one not
/// taken yet. The game's procedure (`0086a0a0`) hands it to `00871d10`, which places the menus'
/// cursor there when the game's mouse is not exclusive (`bBackground Mouse=1`) and otherwise
/// leaves it to `DefWindowProcA`. The window layers keep a place only for such a mouse, while the
/// game's window is the active one. The message's time and point words are not read by the game:
/// 0 and the client place. Nothing before the game has its window.
pub fn post_mouse_move(e: &mut Engine, at: [f64; 2], buttons: &[bool; 8]) {
    let Some((window, width, height)) = mouse_window(e) else {
        return;
    };
    let (width, height) = move_area(e, (width, height));
    let place = |fraction: f64, size: i32| -> u32 {
        let most = size.saturating_sub(1).max(0);
        ((fraction * f64::from(size)).floor() as i32).clamp(0, most) as u32
    };
    let (x, y) = (place(at[0], width), place(at[1], height));
    let wparam = MK_BUTTONS
        .iter()
        .zip(buttons)
        .filter(|(_, held)| **held)
        .fold(0, |bits, (bit, _)| bits | bit);
    let message = [
        window,
        WM_MOUSEMOVE,
        wparam,
        (y << 16) | (x & 0xffff),
        0,
        x,
        y,
    ];
    let queue = &mut state!(e).windows.queue;
    match queue
        .iter_mut()
        .find(|m| m[0] == window && m[1] == WM_MOUSEMOVE)
    {
        Some(waiting) => *waiting = message,
        None => queue.push_back(message),
    }
}

/// `WM_ACTIVATE`.
pub const WM_ACTIVATE: u32 = 6;

/// The real window became active again after it was not (`crate::d3d9real::Input::reactivated`,
/// `wparam` the `WM_ACTIVATE` word: `WA_ACTIVE` 1 or `WA_CLICKACTIVE` 2, the high word set while
/// minimised): the `WM_ACTIVATE` Windows sends the game's main window goes into the game's queue,
/// ahead of a waiting mouse move (Windows sends it before the moves that follow). The game's
/// procedure (`0086a0a0`), for an activation not minimised once `Main` exists, sets `Main` active
/// and has the controls make the mouse exclusive again (`00a23c00(0)`: level 5, `bBackground
/// Mouse`'s flag cleared), so after Alt+Tab back the menus follow the mouse's motion again, as in
/// the original. Its first activation, when the window is made, the game ignores (before `Main`),
/// and the window layers do not pass it. The deactivation is not passed either, as before (in the
/// original it sets `Main` inactive and, in the game, opens the pause menu): unchanged here. One
/// waits at a time. Nothing before the game has its window.
pub fn post_activate(e: &mut Engine, wparam: u32) {
    let main = e.kernel.main_window;
    if main == 0 {
        return;
    }
    let message = [main, WM_ACTIVATE, wparam, 0, 0, 0, 0];
    let queue = &mut state!(e).windows.queue;
    match queue.iter_mut().find(|m| m[1] == WM_ACTIVATE) {
        Some(waiting) => *waiting = message,
        None => queue.push_front(message),
    }
}

// Platform: replaces USER32.DLL DefWindowProcA (import)
/// `DefWindowProcA(window, message, wParam, lParam)`: the default answer to a message the game's
/// procedure passes on (`0086a0a0`). Only the message this layer posts is provided:
/// `WM_MOUSEMOVE` ([`post_mouse_move`]), for which Windows' default does nothing and answers 0.
/// Another stops the run (none other reaches the game's procedure: the module documentation).
pub fn def_window_proc_a(_e: &mut Engine, _window: u32, message: u32, _w: u32, _l: u32) -> u32 {
    match message {
        WM_MOUSEMOVE => 0,
        _ => panic!("DefWindowProcA of message {message:#x} is not provided"),
    }
}

// Platform: replaces USER32.DLL TranslateMessage (import)
/// `TranslateMessage(msg)`: key messages becoming character messages is not provided (the
/// queue holds no key messages: the game reads the keyboard through DirectInput).
pub fn translate_message(e: &mut Engine, msg: Ptr) -> u32 {
    let message = e.mem.u32(msg.addr() + 4);
    assert!(
        !(0x100..=0x109).contains(&message),
        "TranslateMessage of a key message is not provided"
    );
    0
}

// Platform: replaces USER32.DLL DispatchMessageA (import)
/// `DispatchMessageA(msg)`: calls the window's class procedure with the message's four words
/// and answers its result.
pub fn dispatch_message_a(e: &mut Engine, msg: Ptr) -> u32 {
    let m = msg.addr();
    let (window, message, w, l) = (
        e.mem.u32(m),
        e.mem.u32(m + 4),
        e.mem.u32(m + 8),
        e.mem.u32(m + 12),
    );
    let Some(class) = state!(e).windows.windows.get(&window).map(|w| w.class) else {
        return 0;
    };
    let procedure = state!(e)
        .windows
        .classes
        .iter()
        .find(|c| c.atom == class)
        .map(|c| c.procedure)
        .expect("the window's class");
    e.call(procedure, &[window, message, w, l]).u32()
}

// Platform: replaces USER32.DLL GetSystemMetrics (import)
/// `GetSystemMetrics(index)`: the screen metrics the Direct3D 9 layer's capture recorded for a
/// program without a DPI-awareness manifest (`d3d9::system_metric`: `SM_CXSCREEN`,
/// `SM_CYSCREEN`, `SM_CMONITORS`). Other metrics are not provided.
pub fn get_system_metrics(e: &mut Engine, index: u32) -> u32 {
    match super::d3d9::system_metric(e, index) {
        Some(value) => value,
        None => panic!("GetSystemMetrics({index:#x}) is not provided"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `main` goes on when its launcher doesn't start; the name comes with no folder in front.
    #[test]
    fn the_games_launcher_answers_that_nothing_started() {
        let mut e = Engine::new();
        e.mem.map(0x1000, 0x1000);
        e.mem.write(0x1000, b"FalloutNVLauncher.exe\0");
        assert_eq!(shell_execute_a(&mut e, 0, 0, Ptr::new(0x1000), 0, 0, 1), 0);
    }

    /// Windows' `USERNAME` first; Linux and macOS set only `USER`, which then answers (it was
    /// empty there).
    #[test]
    fn the_user_name_is_usernames_else_users() {
        let vars = |pairs: &'static [(&'static str, &'static str)]| {
            move |key: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert_eq!(
            user_name(vars(&[("USERNAME", "win"), ("USER", "unix")])),
            "win"
        );
        assert_eq!(user_name(vars(&[("USER", "unix")])), "unix");
        assert_eq!(
            user_name(vars(&[("USERNAME", ""), ("USER", "unix")])),
            "unix"
        );
        assert_eq!(user_name(vars(&[])), "");
    }

    /// The pointer speed's answer and line: the player's own value as Windows holds it (report
    /// 006: the menus' cursor was faster than in the original, which follows it), an odd string
    /// kept and said, a value of another type kept with its type, a refused read passed on (the
    /// game keeps its own 6: not set, or too long for any pointer speed), and another system the
    /// default 10 (no value).
    #[test]
    fn the_pointer_speed_is_the_players_own_or_windows_default() {
        use crate::PointerSpeed::{Missing, Value};
        let six = b"6\0".to_vec();
        let (speed, line) = pointer_speed_of(Ok((REG_SZ, six.clone())));
        assert_eq!(
            speed,
            Some(Value {
                kind: REG_SZ,
                data: six
            })
        );
        assert_eq!(
            line,
            "input: Windows' pointer speed (Mouse settings) is 6: the game's menu cursor speed \
             follows it, as in the original"
        );
        let (speed, line) = pointer_speed_of(Ok((REG_SZ, b"fast\0".to_vec())));
        assert!(matches!(speed, Some(Value { kind: REG_SZ, .. })));
        assert!(line.contains("is fast (not a speed from 1 to 20"), "{line}");
        let (speed, line) = pointer_speed_of(Ok((4, vec![10, 0, 0, 0])));
        assert_eq!(
            speed,
            Some(Value {
                kind: 4,
                data: vec![10, 0, 0, 0]
            })
        );
        assert!(
            line.contains("stored as registry type 4, not as text"),
            "{line}"
        );
        let (speed, line) = pointer_speed_of(Err(Some(2)));
        assert_eq!(speed, Some(Missing(2)));
        assert!(
            line.contains("could not be read (error 2: it is not set): the game keeps its own"),
            "{line}"
        );
        let (speed, line) = pointer_speed_of(Err(Some(234)));
        assert_eq!(speed, Some(Missing(234)));
        assert!(line.contains("(error 234): the game keeps its own menu cursor speed (6)"));
        let (speed, line) = pointer_speed_of(Err(None));
        assert_eq!(speed, None);
        assert!(
            line.ends_with("pointer speed, 10, as a Windows machine has it"),
            "{line}"
        );
    }

    /// The read is the setting Windows' own mouse settings use: `MouseSensitivity` as text, the
    /// same number `SystemParametersInfoW(SPI_GETMOUSESPEED)` answers (a wrong value name, such as
    /// its neighbour `MouseSpeed`, a wrong type or a lost value fails here).
    #[cfg(windows)]
    #[test]
    fn the_pointer_speed_read_is_windows_mouse_speed() {
        #[link(name = "user32")]
        extern "system" {
            fn SystemParametersInfoW(
                action: u32,
                param: u32,
                value: *mut core::ffi::c_void,
                update: u32,
            ) -> i32;
        }
        const SPI_GETMOUSESPEED: u32 = 0x70;
        let (kind, data) = read_pointer_speed().expect("MouseSensitivity read");
        assert_eq!(kind, REG_SZ, "a string value");
        let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        let read: u32 = std::str::from_utf8(&data[..end])
            .expect("text")
            .trim()
            .parse()
            .expect("a number");
        let mut speed = 0u32;
        // SAFETY: SPI_GETMOUSESPEED writes one int into `speed`.
        let ok = unsafe {
            SystemParametersInfoW(SPI_GETMOUSESPEED, 0, (&mut speed as *mut u32).cast(), 0)
        };
        assert_ne!(ok, 0, "SPI_GETMOUSESPEED");
        assert_eq!(read, speed);
        assert!(matches!(
            pointer_speed().0,
            Some(crate::PointerSpeed::Value { .. })
        ));
    }

    fn wide_of(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn strings(args: Vec<Vec<u16>>) -> Vec<String> {
        args.into_iter()
            .map(|a| String::from_utf16_lossy(&a))
            .collect()
    }

    #[test]
    fn the_command_line_splits_as_windows_does() {
        assert_eq!(
            strings(split_command_line(&wide_of(
                "\"C:\\Games\\FalloutNV.exe\" -nocloud"
            ))),
            ["C:\\Games\\FalloutNV.exe", "-nocloud"]
        );
        assert_eq!(
            strings(split_command_line(&wide_of(
                r#"a.exe "b c" d\\"e" f\"g h"""i"#
            ))),
            ["a.exe", "b c", "d\\e", "f\"g", "h\"i"]
        );
        assert_eq!(strings(split_command_line(&wide_of("a.exe"))), ["a.exe"]);
    }

    /// `SetWindowTextA` titles the record and answers true; a handle that is no window answers
    /// `FALSE` with `ERROR_INVALID_WINDOW_HANDLE` and titles nothing.
    #[test]
    fn set_window_text_titles_the_record() {
        let mut e = Engine::new();
        state!(e).windows.windows.insert(
            0x10,
            Window {
                class: 1,
                title: b"Fallout: New Vegas".to_vec(),
                style: 0,
                ex_style: 0,
                parent: 0,
                rect: [0, 0, 640, 480],
            },
        );
        let text = e.mem.alloc(32);
        e.mem.write(text, b"Loading 12/40\0");
        assert_eq!(set_window_text_a(&mut e, 0x10, Ptr::new(text)), 1);
        assert_eq!(state!(e).windows.windows[&0x10].title, b"Loading 12/40");
        assert_eq!(set_window_text_a(&mut e, 0x99, Ptr::new(text)), 0);
        assert_eq!(e.kernel.last_error, ERROR_INVALID_WINDOW_HANDLE);
    }

    #[test]
    fn command_line_to_argv_makes_one_block() {
        let mut e = Engine::new();
        state!(e).windows.command_line = wide_of("\"C:\\G\\FalloutNV.exe\" -nocloud");
        let line = get_command_line_w(&mut e);
        let count = e.mem.alloc(4);
        let block = command_line_to_argv_w(&mut e, Ptr::new(line), Ptr::new(count));
        assert_eq!(e.mem.u32(count), 2);
        let second = e.mem.u32(block + 4);
        assert_eq!(e.mem.u16(second), b'-' as u16);
        assert_eq!(wide(&e, second), wide_of("-nocloud"));
        assert_eq!(local_free(&mut e, block), 0);
    }

    #[test]
    fn a_second_mutex_of_the_same_name_reports_it_exists() {
        let mut e = Engine::new();
        let name = e.mem.alloc(32);
        e.mem.set_cstr(name, b"FalloutNVGameMutex");
        assert_ne!(create_mutex_a(&mut e, 0, 0, Ptr::new(name)), 0);
        assert_eq!(e.kernel.last_error, 0);
        create_mutex_a(&mut e, 0, 0, Ptr::new(name));
        assert_eq!(e.kernel.last_error, ERROR_ALREADY_EXISTS);
    }

    #[test]
    fn classes_register_once_and_find_window_finds_their_windows() {
        let mut e = Engine::new();
        let name = e.mem.alloc(32);
        e.mem.set_cstr(name, b"Fallout: New Vegas");
        let class = e.mem.alloc(40);
        e.mem.set_u32(class, 3);
        e.mem.set_u32(class + 4, 0x0086_a0a0);
        e.mem.set_u32(class + 36, name);
        assert_eq!(register_class_a(&mut e, Ptr::new(class)), FIRST_ATOM);
        assert_eq!(register_class_a(&mut e, Ptr::new(class)), 0);
        assert_eq!(e.kernel.last_error, ERROR_CLASS_ALREADY_EXISTS);
        assert_eq!(find_window_a(&mut e, name, Ptr::new(0)), 0);
    }

    #[test]
    fn peek_message_without_a_driver_answers_no_message() {
        let mut e = Engine::new();
        let msg = e.mem.alloc(28);
        assert_eq!(peek_message_a(&mut e, Ptr::new(msg), 0, 0, 0, 1), 0);
        assert_eq!(state!(e).windows.frames, 1);
        state!(e).windows.queue.push_back([7, 0x12, 0, 0, 0, 0, 0]);
        assert_eq!(peek_message_a(&mut e, Ptr::new(msg), 0, 0, 0, 1), 1);
        assert_eq!(e.mem.u32(msg + 4), 0x12);
    }

    /// A message `PeekMessageA` removes is noted for the hitch log's split of the host's time between frames: its id,
    /// and its time to main's next `PeekMessageA` (the game's procedure ran it), when main took it after its thread
    /// woke; a message only looked at (no `PM_REMOVE`) is not.
    #[test]
    fn peek_message_notes_the_messages_main_runs_for_the_hitch_log() {
        use engine_core::stage_time as st;
        let _here = st::on_here();
        let mut e = Engine::new();
        let msg = e.mem.alloc(28);
        st::point(st::Point::Idle);
        st::point(st::Point::Host);
        st::host_mark(st::HostMark::MainAwake);
        state!(e)
            .windows
            .queue
            .push_back([7, WM_ACTIVATE, 1, 0, 0, 0, 0]);
        state!(e)
            .windows
            .queue
            .push_back([7, WM_MOUSEMOVE, 0, 0, 0, 0, 0]);
        assert_eq!(peek_message_a(&mut e, Ptr::new(msg), 0, 0, 0, 0), 1);
        st::advance(9_000);
        assert_eq!(peek_message_a(&mut e, Ptr::new(msg), 0, 0, 0, 1), 1);
        assert_eq!(e.mem.u32(msg + 4), WM_ACTIVATE);
        st::advance(2_000_000);
        assert_eq!(peek_message_a(&mut e, Ptr::new(msg), 0, 0, 0, 1), 1);
        assert_eq!(e.mem.u32(msg + 4), WM_MOUSEMOVE);
        st::advance(300);
        assert_eq!(peek_message_a(&mut e, Ptr::new(msg), 0, 0, 0, 1), 0);
        st::point(st::Point::Idle);
        let g = st::take_host_gap().expect("a gap");
        assert_eq!(
            (g.messages, g.messages_ns, g.longest_message),
            (2, 2_000_300, Some((WM_ACTIVATE, 2_000_000)))
        );
    }

    /// A window record of the game's (`class` 1).
    fn record(style: u32, parent: u32, rect: [i32; 4]) -> Window {
        Window {
            class: 1,
            title: Vec::new(),
            style,
            ex_style: 0,
            parent,
            rect,
        }
    }

    /// The cursor's moves over the game's window (a mouse the game made non-exclusive, report 002)
    /// become the `WM_MOUSEMOVE` Windows posts to the window the game draws in: the renderer's
    /// child of the main window, in its client pixels at the game's size (here 1920 by 1080, the
    /// player's), with the buttons held as `MK_*` bits. One waits at a time, the latest place;
    /// the game takes it with `PeekMessageA`, its procedure gets the four words through
    /// `DispatchMessageA`, and `DefWindowProcA` answers it with 0. Full screen (no child) the main
    /// window gets it; before the game has a window nothing is posted.
    #[test]
    fn a_cursor_move_is_the_games_wm_mousemove() {
        let mut e = Engine::new();
        let unmoved = [false; 8];
        post_mouse_move(&mut e, [0.5, 0.5], &unmoved);
        assert!(state!(e).windows.queue.is_empty(), "no window yet");
        let (main, child) = (0x94, 0xc0);
        e.kernel.main_window = main;
        state!(e)
            .windows
            .windows
            .insert(main, record(0x14c0_0000, 0, [5, 5, 1941, 1105]));
        state!(e)
            .windows
            .windows
            .insert(child, record(0x5000_0000, main, [0, 0, 1920, 1080]));
        let mut held = [false; 8];
        held[0] = true;
        held[4] = true;
        post_mouse_move(&mut e, [0.25, 0.75], &unmoved);
        post_mouse_move(&mut e, [0.5, 0.5], &held);
        let queue: Vec<[u32; 7]> = state!(e).windows.queue.iter().copied().collect();
        assert_eq!(
            queue,
            [[child, WM_MOUSEMOVE, 0x41, (540 << 16) | 960, 0, 960, 540]],
            "one move waits, the latest"
        );
        // The far edge is the last pixel.
        post_mouse_move(&mut e, [1.0, 1.0], &unmoved);
        assert_eq!(
            state!(e).windows.queue[0][3],
            (1079 << 16) | 1919,
            "the far edge"
        );
        // The game's loop: the message reaches its window's class procedure, here a stand-in.
        state!(e).windows.classes.push(Class {
            atom: 1,
            name: b"Fallout: New Vegas".to_vec(),
            style: 3,
            procedure: 0x7eff_f000,
            instance: 0,
            menu_name: 0,
        });
        thread_local! {
            static SEEN: std::cell::Cell<[u32; 4]> = const { std::cell::Cell::new([0; 4]) };
        }
        e.register(0x7eff_f000, |e, a| {
            SEEN.with(|s| s.set([a[0], a[1], a[2], a[3]]));
            crate::Ret {
                eax: def_window_proc_a(e, a[0], a[1], a[2], a[3]),
                ..crate::Ret::default()
            }
        });
        let msg = e.mem.alloc(28);
        assert_eq!(peek_message_a(&mut e, Ptr::new(msg), 0, 0, 0, 1), 1);
        assert_eq!(dispatch_message_a(&mut e, Ptr::new(msg)), 0);
        assert_eq!(
            SEEN.with(std::cell::Cell::get),
            [child, WM_MOUSEMOVE, 0, (1079 << 16) | 1919]
        );
        assert!(state!(e).windows.queue.is_empty());
        // Full screen: no child, the main window covers the screen and gets the moves.
        state!(e).windows.windows.remove(&child);
        state!(e)
            .windows
            .windows
            .insert(main, record(0x9000_0000, 0, [0, 0, 2560, 1440]));
        post_mouse_move(&mut e, [0.5, 0.25], &unmoved);
        assert_eq!(
            state!(e).windows.queue[0][..4],
            [main, WM_MOUSEMOVE, 0, (360 << 16) | 1280]
        );
        // Once the renderer has its size, the place is in its pixels, what the game divides it by
        // (`00871d10`), whatever the window's record says: here 1920 by 1080 in a 2560 by 1440
        // window, the fractions given back exactly.
        if !e.mem.is_mapped(RENDER_WIDTH) {
            e.map(RENDER_WIDTH & !0xfff, 0x1000);
        }
        e.mem.set_u32(RENDER_WIDTH, 1920);
        e.mem.set_u32(RENDER_HEIGHT, 1080);
        post_mouse_move(&mut e, [0.5, 0.25], &unmoved);
        let words = state!(e).windows.queue[0];
        assert_eq!(words[..4], [main, WM_MOUSEMOVE, 0, (270 << 16) | 960]);
        assert_eq!(
            (
                f64::from(words[3] & 0xffff) / 1920.0,
                f64::from(words[3] >> 16) / 1080.0
            ),
            (0.5, 0.25)
        );
    }

    /// The host's moves reach the game before the frame they were taken for, as the exe's loop
    /// empties its queue before its frame: a move posted while the main loop's empty
    /// `PeekMessageA` hands the host its turn is answered by that same call, and the next empty
    /// call starts the frame without a second turn; the call after that ends the next frame.
    #[test]
    fn a_move_posted_in_the_hosts_turn_comes_before_the_frame() {
        thread_local! {
            static TURNS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        fn turn(e: &mut Engine) -> bool {
            TURNS.with(|t| t.set(t.get() + 1));
            e.kernel.main_window = 0x94;
            state!(e)
                .windows
                .windows
                .insert(0x94, record(0x9000_0000, 0, [0, 0, 1280, 720]));
            post_mouse_move(e, [0.5, 0.5], &[false; 8]);
            true
        }
        fn idle(_: &mut Engine) -> bool {
            TURNS.with(|t| t.set(t.get() + 1));
            true
        }
        let mut e = Engine::new();
        let msg = Ptr::new(e.mem.alloc(28));
        assert_eq!(
            peek_with(&mut e, msg, PM_REMOVE, turn),
            1,
            "the move, at once"
        );
        assert_eq!(e.mem.u32(msg.addr() + 4), WM_MOUSEMOVE);
        assert_eq!(e.mem.u32(msg.addr() + 12), (360 << 16) | 640);
        assert_eq!(
            peek_with(&mut e, msg, PM_REMOVE, idle),
            0,
            "the frame starts"
        );
        assert_eq!(TURNS.with(std::cell::Cell::get), 1, "no second turn");
        assert_eq!(state!(e).windows.frames, 1);
        assert_eq!(
            peek_with(&mut e, msg, PM_REMOVE, idle),
            0,
            "the next frame ends"
        );
        assert_eq!(TURNS.with(std::cell::Cell::get), 2);
        assert_eq!(state!(e).windows.frames, 2);
        // A turn that posts nothing: the frame starts at once.
        assert_eq!(peek_with(&mut e, msg, PM_REMOVE, idle), 0);
        assert_eq!(TURNS.with(std::cell::Cell::get), 3);
    }

    /// An activation after the window was not active becomes the `WM_ACTIVATE` Windows sends the
    /// game's main window, ahead of a waiting move, one at a time (the latest); nothing before the
    /// game has its window.
    #[test]
    fn a_reactivation_is_the_games_wm_activate_before_a_move() {
        let mut e = Engine::new();
        post_activate(&mut e, 1);
        assert!(state!(e).windows.queue.is_empty(), "no window yet");
        e.kernel.main_window = 0x94;
        state!(e)
            .windows
            .windows
            .insert(0x94, record(0x9000_0000, 0, [0, 0, 1280, 720]));
        post_mouse_move(&mut e, [0.5, 0.5], &[false; 8]);
        post_activate(&mut e, 2);
        post_activate(&mut e, 1);
        let queue: Vec<[u32; 7]> = state!(e).windows.queue.iter().copied().collect();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0], [0x94, WM_ACTIVATE, 1, 0, 0, 0, 0]);
        assert_eq!(queue[1][1], WM_MOUSEMOVE);
    }

    /// `DefWindowProcA` answers the one message this layer posts as Windows' default does (0) and
    /// stops on any other, which never reaches the game's procedure.
    #[test]
    fn the_default_procedure_answers_a_mouse_move() {
        let mut e = Engine::new();
        assert_eq!(def_window_proc_a(&mut e, 0x94, WM_MOUSEMOVE, 0, 0), 0);
        let other = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            def_window_proc_a(&mut e, 0x94, 0x84, 0, 0)
        }));
        assert!(other.is_err(), "WM_NCHITTEST is not provided");
    }

    /// The frame the game's renderer adds before it places its window (exe 0x004da951): the
    /// table's `AdjustWindowRect` border for the window's own style, its class's menu (the game's
    /// class has none) and no extended style, never one of another key; none for a window that
    /// is not there.
    #[test]
    fn the_game_frame_is_the_tables_border_for_the_windows_style() {
        let mut e = Engine::new();
        crate::d3d9::load_adapter_table(
            &mut e,
            "AdjustWindowRectEx\t10000000 0 0\t1\tfffffff8 ffffffe1 8 8\n\
             AdjustWindowRectEx\t10000000 1 0\t1\tfffffff8 ffffffce 8 8\n\
             AdjustWindowRectEx\t10000000 0 200\t1\tfffffff6 ffffffdf a a\n\
             AdjustWindowRectEx\t0 0 0\t1\t0 0 0 0\n",
        );
        let name = e.mem.alloc(32);
        e.mem.set_cstr(name, b"Fallout: New Vegas");
        let class = e.mem.alloc(40);
        for at in (0..40).step_by(4) {
            e.mem.set_u32(class + at, 0);
        }
        e.mem.set_u32(class + 36, name);
        assert_eq!(register_class_a(&mut e, Ptr::new(class)), FIRST_ATOM);
        state!(e).windows.windows.insert(
            0x10,
            Window {
                class: FIRST_ATOM,
                title: Vec::new(),
                style: 0x1000_0000,
                ex_style: 0,
                parent: 0,
                rect: [0, 0, 336, 279],
            },
        );
        assert_eq!(game_frame(&mut e, 0x10), Some([-8, -31, 8, 8]));
        assert_eq!(game_frame(&mut e, 0x99), None);
    }

    #[test]
    #[should_panic(expected = "the game gave up")]
    fn a_message_box_stops_the_run_with_its_text() {
        let mut e = Engine::new();
        let text = e.mem.alloc(16);
        e.mem.set_cstr(text, b"no");
        message_box_a(&mut e, 0, Ptr::new(text), Ptr::new(text), 0);
    }
}

#[cfg(test)]
mod tests_steam {
    use super::*;

    /// The Steam imports answer the no-client state, registered under their names.
    #[test]
    fn the_steam_accessors_answer_null_and_steam_is_not_running() {
        let mut e = Engine::new();
        for name in [
            "SteamAPI_IsSteamRunning",
            "SteamUserStats",
            "SteamFriends",
            "SteamUtils",
            "SteamUser",
            "SteamApps",
        ] {
            let (_, _, f) = super::super::imports()
                .into_iter()
                .find(|(dll, n, _)| *dll == "steam_api.dll" && *n == name)
                .unwrap_or_else(|| panic!("{name} is not registered"));
            assert_eq!(f(&mut e, &[]).eax, 0, "{name}");
        }
    }

    /// `SteamAPI_RunCallbacks` is registered and leaves the state as it is.
    #[test]
    fn run_callbacks_does_nothing() {
        let mut e = Engine::new();
        let (_, _, f) = super::super::imports()
            .into_iter()
            .find(|(dll, n, _)| *dll == "steam_api.dll" && *n == "SteamAPI_RunCallbacks")
            .expect("SteamAPI_RunCallbacks is registered");
        e.kernel.last_error = 0x1234;
        for _ in 0..3 {
            assert_eq!(f(&mut e, &[]).eax, 0);
        }
        assert_eq!(e.kernel.last_error, 0x1234);
        assert_eq!(steam_user_stats(&mut e), 0);
    }

    /// `SteamAPI_Shutdown` is registered and, without `SteamAPI_Init`, leaves the state as it is.
    #[test]
    fn shutdown_without_init_does_nothing() {
        let mut e = Engine::new();
        let (_, _, f) = super::super::imports()
            .into_iter()
            .find(|(dll, n, _)| *dll == "steam_api.dll" && *n == "SteamAPI_Shutdown")
            .expect("SteamAPI_Shutdown is registered");
        e.kernel.last_error = 0x4321;
        for _ in 0..2 {
            assert_eq!(f(&mut e, &[]).eax, 0);
        }
        assert_eq!(e.kernel.last_error, 0x4321);
        assert_eq!(steam_user_stats(&mut e), 0);
    }
}
