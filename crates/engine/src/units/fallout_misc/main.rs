//! `fallout/misc/main.cpp` (Xbox PDB source unit), subsystem `fallout/misc`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! This is the unit of the game's `main` (the Windows entry point, `0086a850`) and of the
//! helpers around it: the shader timer, the message-box wrapper, the renderer's off-screen
//! frame guards, small global setters and the deleting destructors of the objects `main`
//! creates.
//!
//! The unit is split into part files by address range (`main_p2`, ...); the constants and
//! helpers a part may need are `pub(crate)` here.
//!
//! Not translated: the compiler's stack-cookie check at the end of `main`
//! (`__security_check_cookie`, `00ec408c`) and the `INT3` of the debugger-present branch of the
//! message-menu callback.
//!
//! Calling-convention note: the decompiler hangs a pushed argument on the wrong call all over
//! `main` (`PUSH x; CALL getter; MOV ECX,EAX; CALL method` is `method(getter(), x)`), so the
//! arguments below are read from the disassembly and checked against each callee's `RET n`.

#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------------------------------
// Callees and globals shared by the functions of this file.
// ---------------------------------------------------------------------------------------------

/// `Error` (Xbox PDB name): the game's logging function. In this build its body is empty
/// (`PUSH EBP; MOV EBP,ESP; POP EBP; RET`); the calls are kept, with their arguments.
pub(crate) const ERROR_LOG: u32 = 0x0040_fbe0;
/// The renderer singleton's getter (reads the global at `011f4748`).
pub(crate) const GET_RENDERER: u32 = 0x0043_c4b0;
/// Setting object method: address of the setting's one-byte value.
pub(crate) const SETTING_BYTE_PTR: u32 = 0x0040_8d60;
/// Setting object method: address of the setting's 32-bit value.
pub(crate) const SETTING_INT_PTR: u32 = 0x0043_d4d0;
/// Frees a heap object (cdecl, one argument).
pub(crate) const FREE_OBJECT: u32 = 0x0040_1030;
/// Allocates a heap block (cdecl, size).
pub(crate) const ALLOCATE_OBJECT: u32 = 0x0040_1000;
/// `Interface::IsInMenuMode` (Xbox PDB).
pub(crate) const IS_IN_MENU_MODE: u32 = 0x0070_2360;
/// `Calendar::GetHour` (Xbox PDB), called on the calendar at [`CALENDAR`].
pub(crate) const CALENDAR_GET_HOUR: u32 = 0x0086_7da0;
/// `BSTimer::Disable` and `BSTimer::Enable` (Xbox PDB), called on [`TIMER`].
pub(crate) const TIMER_DISABLE: u32 = 0x00aa_50a0;
pub(crate) const TIMER_ENABLE: u32 = 0x00aa_5040;
/// Methods of the timer at [`TIMER`]: a `float` in ST0 (`0084d030`) and a millisecond count in
/// EAX (`00825c00`).
pub(crate) const TIMER_GET_SECONDS: u32 = 0x0084_d030;
pub(crate) const TIMER_GET_MILLISECONDS: u32 = 0x0082_5c00;
/// `BaseProcess::GetCurrentProcedureIndex` (Xbox PDB), called on the main object: the id of the
/// thread that runs the main loop.
pub(crate) const GET_MAIN_THREAD_ID: u32 = 0x0044_edb0;
/// Called on the main object: a pointer to its helper object (or 0).
pub(crate) const MAIN_OBJECT_GET_HELPER: u32 = 0x0087_7720;
/// Called on the helper: non-zero when bit 3 of the word at +4 is set.
pub(crate) const HELPER_HAS_FLAG: u32 = 0x0071_1e00;
/// Called on the helper with one argument (`1`).
pub(crate) const HELPER_SET_FLAG: u32 = 0x00a2_3c00;
/// `Interface::GetMessageMenuResult` (Xbox PDB): the button used in the warning menu.
pub(crate) const GET_MESSAGE_MENU_RESULT: u32 = 0x0070_3fa0;
/// Closes the warning menu.
pub(crate) const CLOSE_MESSAGE_MENU: u32 = 0x0070_3fd0;
/// Opens the warning menu: text, two zero words, the result callback, a word, the context, two
/// floats, three pooled button texts and a word (cdecl, 12 words). Non-zero when it was opened.
pub(crate) const SHOW_MESSAGE_MENU: u32 = 0x0070_3e80;
/// Returns the text of a string-pool object (`__thiscall`, no argument).
pub(crate) const GET_POOLED_TEXT: u32 = 0x0040_3df0;
/// Returns the byte setting at `011c77b4` (AL).
pub(crate) const GET_WINDOW_SETTING: u32 = 0x0044_6e10;
/// Returns the global at `011c6fbc`.
pub(crate) const GET_WINDOW_STATE: u32 = 0x004d_c1e0;

/// The timer object [`shader_timer`], `BSTimer::Disable` and `BSTimer::Enable` work on.
pub(crate) const TIMER: u32 = 0x011f_6394;
/// The global `Calendar` instance.
pub(crate) const CALENDAR: u32 = 0x011d_e7b8;
/// `1000.0` (`double`).
const MILLISECONDS_PER_SECOND: u32 = 0x0101_7b70;
/// `60.0` (`double`).
const SIXTY: u32 = 0x0101_2638;
/// Setting (byte): when set, [`fn_0086a580`] answers without showing a box.
const SETTING_NO_MESSAGE_BOXES: u32 = 0x011d_eb3c;
/// The `Main` object (0xA4 bytes, built by `Main::Main`, `0086c160`); its first byte is the
/// "background loading thread is suspended" flag. Null before `main` creates it.
pub(crate) const MAIN_OBJECT: u32 = 0x011d_ea0c;
/// Byte flag: the warning menu may be used for the next message box.
const WARNING_MENU_AVAILABLE: u32 = 0x011a_3015;
/// Global returned by `0086a820`: the context word handed to the warning menu.
const WARNING_MENU_CONTEXT: u32 = 0x011f_638c;
/// The pooled button texts of the warning menu.
const BUTTON_TEXT_THIRD: u32 = 0x011d_12b4;
const BUTTON_TEXT_SECOND: u32 = 0x011d_0cb4;
const BUTTON_TEXT_FIRST: u32 = 0x011d_06f4;

/// Imports, called by the address of their import slot.
pub(crate) const API_CREATE_MUTEX: u32 = 0x00fd_f0fc;
pub(crate) const API_GET_LAST_ERROR: u32 = 0x00fd_f094;
pub(crate) const API_MESSAGE_BOX: u32 = 0x00fd_f2f4;
pub(crate) const API_GET_SYSTEM_METRICS: u32 = 0x00fd_f2b0;
pub(crate) const API_GET_SYSTEM_INFO: u32 = 0x00fd_f0f8;
pub(crate) const API_FIND_WINDOW: u32 = 0x00fd_f304;
pub(crate) const API_SET_FOREGROUND_WINDOW: u32 = 0x00fd_f300;
pub(crate) const API_GET_COMMAND_LINE: u32 = 0x00fd_f0f4;
pub(crate) const API_COMMAND_LINE_TO_ARGV: u32 = 0x00fd_f270;
pub(crate) const API_LOCAL_FREE: u32 = 0x00fd_f08c;
pub(crate) const API_SHELL_EXECUTE: u32 = 0x00fd_f278;
pub(crate) const API_LOAD_ICON: u32 = 0x00fd_f280;
pub(crate) const API_GET_STOCK_OBJECT: u32 = 0x00fd_f03c;
pub(crate) const API_REGISTER_CLASS: u32 = 0x00fd_f2b4;
pub(crate) const API_ADJUST_WINDOW_RECT: u32 = 0x00fd_f310;
pub(crate) const API_CREATE_WINDOW: u32 = 0x00fd_f2b8;
pub(crate) const API_PEEK_MESSAGE: u32 = 0x00fd_f29c;
pub(crate) const API_TRANSLATE_MESSAGE: u32 = 0x00fd_f298;
pub(crate) const API_DISPATCH_MESSAGE: u32 = 0x00fd_f294;
pub(crate) const API_GET_ACTIVE_WINDOW: u32 = 0x00fd_f2fc;
pub(crate) const API_SLEEP: u32 = 0x00fd_f104;
pub(crate) const API_GET_CURRENT_THREAD_ID: u32 = 0x00fd_f1d0;
pub(crate) const API_GET_WINDOW_LONG: u32 = 0x00fd_f2e8;
pub(crate) const API_ADJUST_WINDOW_RECT_EX: u32 = 0x00fd_f2f8;
pub(crate) const API_SET_WINDOW_POS: u32 = 0x00fd_f2a4;
pub(crate) const API_IS_DEBUGGER_PRESENT: u32 = 0x00fd_f0f0;
pub(crate) const API_EXIT_PROCESS: u32 = 0x00fd_f0ec;

// ---------------------------------------------------------------------------------------------
// The renderer's off-screen frame guards (`NiRenderer`).
// ---------------------------------------------------------------------------------------------

/// `NiRenderer::m_eFrameState` (Xbox PDB) at +0x200 of the renderer.
const RENDERER_FRAME_STATE: u32 = 0x200;
/// The renderer's two pairs of parallel `NiTArray`s at +0xAD4 / +0xAE4 and +0xAF4 / +0xB04:
/// the first of a pair holds keys (addresses of registered functions), the second their data.
const KEY_ARRAY_A: u32 = 0xad4;
const DATA_ARRAY_A: u32 = 0xae4;
const KEY_ARRAY_B: u32 = 0xaf4;
const DATA_ARRAY_B: u32 = 0xb04;
/// Array methods: add an element given by the address of its value, returning its index
/// (`00877a50`); address of the element at an index (`00877a30`); element count (`00658930`,
/// the 16 bits at +0x0A); `SetAtGrow(index, address of the value)` (`00470000`); remove the
/// element at an index (`009e98d0`).
const ARRAY_ADD: u32 = 0x0087_7a50;
const ARRAY_ELEMENT: u32 = 0x0087_7a30;
const ARRAY_COUNT: u32 = 0x0065_8930;
const ARRAY_SET_AT_GROW: u32 = 0x0047_0000;
const ARRAY_REMOVE_AT: u32 = 0x009e_98d0;

// Translated from 0086a480 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ShaderTimer` (Xbox PDB): the time value the shader system reads. Mode 0 is the game hour
/// in seconds, mode 1 the timer's seconds (0 while a menu is open), mode 2 the timer's
/// milliseconds as seconds (0 while a menu is open), mode 3 the timer's seconds; any other
/// mode gives 0.
pub fn shader_timer(e: &mut Engine, mode: i32) -> f32 {
    match mode {
        3 => e.call(TIMER_GET_SECONDS, &args![TIMER]).f32(),
        2 => {
            if e.call(IS_IN_MENU_MODE, &args![]).bool() {
                return 0.0;
            }
            let milliseconds = e.call(TIMER_GET_MILLISECONDS, &args![TIMER]).u32();
            let per_second: f64 = e.global(MILLISECONDS_PER_SECOND);
            (milliseconds as f64 / per_second) as f32
        }
        1 => {
            if e.call(IS_IN_MENU_MODE, &args![]).bool() {
                return 0.0;
            }
            e.call(TIMER_GET_SECONDS, &args![TIMER]).f32()
        }
        0 => {
            let hour = e.call(CALENDAR_GET_HOUR, &args![CALENDAR]).f32();
            let sixty: f64 = e.global(SIXTY);
            (hour as f64 * sixty * sixty) as f32
        }
        _ => 0.0,
    }
}

// Translated from 0086a530 (decompiled, FalloutNV.exe 1.4.0.525)
/// The result callback of the warning menu opened by [`fn_0086a580`]: results 1 and 3 log
/// "Warning Response was to Exit the Process" and end the process (a breakpoint first when a
/// debugger is attached, not translated); result 2 closes the menu.
pub fn fn_0086a530(e: &mut Engine) {
    let result = e.call(GET_MESSAGE_MENU_RESULT, &args![]).u8();
    match result {
        1 | 3 => {
            e.call(ERROR_LOG, &args![0x0108_28c0u32]);
            // `IsDebuggerPresent`, then `INT3` when it is non-zero: no equivalent here.
            e.call(API_IS_DEBUGGER_PRESENT, &args![]);
            // `ExitProcess` does not return.
            e.call(API_EXIT_PROCESS, &args![0u32]);
        }
        2 => {
            e.call(CLOSE_MESSAGE_MENU, &args![]);
        }
        _ => {}
    }
}

/// Shows a Windows message box. When the main object exists and the calling thread is not its
/// thread, the helper of the main object gets its flag set first (unless it has it).
fn show_windows_message_box(e: &mut Engine, text: u32, caption: u32, flags: u32) -> i32 {
    let main_object = e.global::<u32>(MAIN_OBJECT);
    if main_object != 0 {
        let thread = e.call(API_GET_CURRENT_THREAD_ID, &args![]).u32();
        let main_thread = e.call(GET_MAIN_THREAD_ID, &args![main_object]).u32();
        if thread != main_thread && e.call(MAIN_OBJECT_GET_HELPER, &args![main_object]).u32() != 0 {
            let helper = e.call(MAIN_OBJECT_GET_HELPER, &args![main_object]).u32();
            if e.call(HELPER_HAS_FLAG, &args![helper]).u32() == 0 {
                let helper = e.call(MAIN_OBJECT_GET_HELPER, &args![main_object]).u32();
                e.call(HELPER_SET_FLAG, &args![helper, 1u32]);
            }
        }
    }
    e.call(API_MESSAGE_BOX, &args![0u32, text, caption, flags])
        .i32()
}

/// Whether the window setting is on and the window state global is non-zero: the cases in
/// which [`fn_0086a580`] does not show a Windows message box.
fn message_box_suppressed(e: &mut Engine) -> bool {
    e.call(GET_WINDOW_SETTING, &args![]).bool() && e.call(GET_WINDOW_STATE, &args![]).u32() != 0
}

// Translated from 0086a580 (decompiled, FalloutNV.exe 1.4.0.525)
/// The game's `MessageBox` wrapper (`text`, `caption`, `MB_*` flags; cdecl). Returns the button
/// id. With the no-message-boxes setting it answers without showing anything: 5 (`IDIGNORE`)
/// for `MB_ABORTRETRYIGNORE` and 6 (`IDYES`) otherwise. Otherwise the timer is disabled around
/// the box. When the warning menu is available and the main object has its helper, boxes of
/// styles other than `MB_OK` and `MB_ABORTRETRYIGNORE` are opened as the game's warning menu
/// (button texts for styles 3 and 4), answered by [`fn_0086a530`], and the menu is marked
/// unavailable meanwhile; if that fails, or in every other case, a Windows message box is
/// shown unless [`message_box_suppressed`], in which case the answer is 5 for style 2 and 6 for
/// styles 3 and 4 (6 is also the answer when the box is suppressed without the menu).
pub fn fn_0086a580(e: &mut Engine, text: u32, caption: u32, flags: u32) -> i32 {
    let style = (flags & 0xf) as u8;
    let setting = e
        .call(SETTING_BYTE_PTR, &args![SETTING_NO_MESSAGE_BOXES])
        .u32();
    if e.mem.u8(setting) != 0 {
        return if style == 2 { 5 } else { 6 };
    }
    e.call(TIMER_DISABLE, &args![TIMER]);
    let mut result: i32 = 6;

    let main_object = e.global::<u32>(MAIN_OBJECT);
    let use_menu = e.global::<u8>(WARNING_MENU_AVAILABLE) != 0
        && main_object != 0
        && e.call(MAIN_OBJECT_GET_HELPER, &args![main_object]).u32() != 0;

    if !use_menu {
        if !message_box_suppressed(e) {
            result = show_windows_message_box(e, text, caption, flags);
        }
        e.call(TIMER_ENABLE, &args![TIMER]);
        return result;
    }

    e.set_global(WARNING_MENU_AVAILABLE, 0u8);
    let mut first_text = 0u32;
    let mut second_text = 0u32;
    let mut third_text = 0u32;
    if style == 3 {
        third_text = e.call(GET_POOLED_TEXT, &args![BUTTON_TEXT_THIRD]).u32();
    }
    if style == 3 || style == 4 {
        second_text = e.call(GET_POOLED_TEXT, &args![BUTTON_TEXT_SECOND]).u32();
        first_text = e.call(GET_POOLED_TEXT, &args![BUTTON_TEXT_FIRST]).u32();
    }
    let opened = if style != 2 && style != 0 {
        let context = fn_0086a820(e);
        e.call(
            SHOW_MESSAGE_MENU,
            &args![
                text,
                0u32,
                0u32,
                0x0086_a530u32,
                0u32,
                context,
                0.0f32,
                0.0f32,
                second_text,
                first_text,
                third_text,
                0u32
            ],
        )
        .bool()
    } else {
        false
    };
    if !opened {
        if !message_box_suppressed(e) {
            result = show_windows_message_box(e, text, caption, flags);
        } else if style == 2 {
            result = 5;
        } else if style > 2 && style <= 4 {
            result = 6;
        }
    }
    e.set_global(WARNING_MENU_AVAILABLE, 1u8);
    e.call(TIMER_ENABLE, &args![TIMER]);
    result
}

// Translated from 0086a820 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the global at `011f638c`: the context word handed to the warning menu.
pub fn fn_0086a820(e: &mut Engine) -> u32 {
    e.global(WARNING_MENU_CONTEXT)
}

// Translated from 0086a830 (decompiled, FalloutNV.exe 1.4.0.525)
/// A `__thiscall` method with one stack argument that ignores both and returns 0.
pub fn fn_0086a830(_e: &mut Engine, _unused_0: u32, _unused_1: u32) -> u8 {
    0
}

// ---------------------------------------------------------------------------------------------
// `main`.
// ---------------------------------------------------------------------------------------------

/// Size of `main`'s stack frame.
const MAIN_FRAME_SIZE: u32 = 0x378;

// `main`'s locals, as offsets from its frame pointer.
const LOCAL_MUTEX: i32 = -0x20;
const LOCAL_WINDOW: i32 = -0x24;
const LOCAL_FOUND_WINDOW: i32 = -0x4;
/// Passed by address to `Main_InitStartCell`, then its three words are passed by value to
/// `00877260`.
const LOCAL_START_POSITION: i32 = -0x1c8;
const LOCAL_START_CELL: i32 = -0x8;
const LOCAL_SYSTEM_INFO: i32 = -0x1bc;
const LOCAL_PROCESSOR_COUNT: i32 = -0x1a8;
const LOCAL_PATH_BUFFER: i32 = -0x130;
const LOCAL_LAUNCHER_PATH: i32 = -0x2e8;
const LOCAL_ARG_COUNT: i32 = -0x1d4;
const LOCAL_ARGS: i32 = -0x1d0;
const LOCAL_WINDOW_CLASS: i32 = -0x194;
const LOCAL_WINDOW_RECT: i32 = -0x1c;
const LOCAL_MESSAGE: i32 = -0x16c;
const LOCAL_SCOPE: i32 = -0x150;
const LOCAL_EXIT_DATA: i32 = -0x14c;
const LOCAL_ACTIVE_WINDOW: i32 = -0x134;
const LOCAL_RESIZE_RECT_ACTIVE: i32 = -0x300;
const LOCAL_RESIZE_RECT_IDLE: i32 = -0x314;

/// Argument of `GetSystemMetrics` for "is this a remote session".
const SM_REMOTESESSION: u32 = 0x1000;
const ERROR_ALREADY_EXISTS: u32 = 0xb7;
/// `GWL_STYLE`.
const GWL_STYLE: u32 = (-0x10i32) as u32;

/// The value of a setting object that holds a byte (through [`SETTING_BYTE_PTR`]).
fn setting_byte(e: &mut Engine, setting: u32) -> u32 {
    let address = e.call(SETTING_BYTE_PTR, &args![setting]).u32();
    e.mem.u8(address) as u32
}

/// Starts the log files (`c3bd30` with three settings, the error-handler address and `flag`,
/// then the names of the four text logs).
fn open_log_files(e: &mut Engine, flag: u32) {
    let first = setting_byte(e, 0x011d_ed74);
    let second = setting_byte(e, 0x011d_ee20);
    let third = setting_byte(e, 0x011d_ec88);
    e.call(
        0x00c3_bd30,
        &args![flag, third, second, first, 0x0086_a580u32],
    );
    e.call(0x00c3_bb90, &args![0x0108_2a38u32]);
    e.call(0x00c3_bbe0, &args![0x0108_2a28u32]);
    e.call(0x00c3_bc30, &args![0x0108_2a10u32]);
    e.call(0x00c3_bc80, &args![0x0108_2a00u32]);
}

/// When the processor-count setting (`011c3ea4`) is 1, makes it 2.
fn raise_single_processor_count(e: &mut Engine) {
    let count = e.call(SETTING_INT_PTR, &args![0x011c_3ea4u32]).u32();
    if e.mem.u32(count) == 1 {
        let count = e.call(SETTING_INT_PTR, &args![0x011c_3ea4u32]).u32();
        e.mem.set_u32(count, 2);
    }
}

/// Resizes the game window to the renderer's screen size: the client rectangle
/// (`0, 0, 004dc1f0(), 004dc200()`) grown by the window's style, then `SetWindowPos` with the
/// rectangle's left and bottom as the position and (right - left, top - bottom) as the size,
/// exactly as the game passes them. `rect` is a 20-byte local (the rectangle and the style).
fn resize_window_to_screen(e: &mut Engine, window: u32, rect: u32) {
    e.mem.set_u32(rect, 0);
    e.mem.set_u32(rect + 4, 0);
    let width = e.call(0x004d_c1f0, &args![]).u32();
    e.mem.set_u32(rect + 8, width);
    let height = e.call(0x004d_c200, &args![]).u32();
    e.mem.set_u32(rect + 12, height);
    e.mem.set_u32(rect + 16, 0);
    let style = e.call(API_GET_WINDOW_LONG, &args![window, GWL_STYLE]).u32();
    e.mem.set_u32(rect + 16, style);
    e.call(API_ADJUST_WINDOW_RECT_EX, &args![rect, style, 0u32, 0u32]);
    let left = e.mem.u32(rect);
    let top = e.mem.u32(rect + 4);
    let right = e.mem.u32(rect + 8);
    let bottom = e.mem.u32(rect + 12);
    e.call(
        API_SET_WINDOW_POS,
        &args![
            window,
            0u32,
            left,
            bottom,
            right.wrapping_sub(left),
            top.wrapping_sub(bottom),
            0u32
        ],
    );
}

/// Returns a rendered texture held in the object at `holder` (`00559450` reads it) to the
/// texture manager and clears the holder.
fn release_held_texture(e: &mut Engine, holder: u32) {
    if e.call(0x0055_9450, &args![holder]).u32() != 0 {
        let texture = e.call(0x0055_9450, &args![holder]).u32();
        let manager = e.call(0x004a_0ea0, &args![]).u32();
        // BSTextureManager::ReturnRenderedTexture (Xbox PDB)
        e.call(0x00b6_da10, &args![manager, texture]);
        e.call(0x0066_b0d0, &args![holder, 0u32]);
    }
}

/// Starts or restarts one of the profiling-style scope objects `main` keeps at
/// [`LOCAL_SCOPE`]: `00404f00(category, 1, source file, line)`.
fn scope_begin(e: &mut Engine, scope: u32, category: u32, line: u32) {
    e.call(
        0x0040_4f00,
        &args![scope, category, 1u32, 0x0108_29c4u32, line],
    );
}

// Translated from 0086a850 (decompiled, FalloutNV.exe 1.4.0.525)
/// `main` (Xbox PDB): the game's `WinMain`. `instance` is the module handle; the other three
/// words (previous instance, command line, show command) are not read. Returns 0.
///
/// Sequence: single-instance mutex and remote-session checks; log files; processor count;
/// activates an already running window instead of starting; singletons and system
/// initialisation; `-nocloud` on the command line; the INI paths; optionally hands over to
/// `FalloutNVLauncher.exe`; window class and window; `Main`; the renderer, shader and scene
/// graph initialisation; the world, the player and the start cell; the message loop (active
/// frames call `Main`'s per-frame function, inactive ones sleep 50 ms and suspend audio); and
/// the shutdown of everything in reverse.
pub fn main(e: &mut Engine, instance: u32, _unused_1: u32, _unused_2: u32, _unused_3: u32) -> u32 {
    e.with_stack(MAIN_FRAME_SIZE, |e, frame| {
        let ebp = frame.addr() + MAIN_FRAME_SIZE;
        run_main(e, ebp, instance)
    })
}

/// `main`'s body on its frame, whose frame pointer is `ebp` (locals at negative offsets).
fn run_main(e: &mut Engine, ebp: u32, instance: u32) -> u32 {
    let at = |offset: i32| ebp.wrapping_add(offset as u32);

    let mutex = e
        .call(API_CREATE_MUTEX, &args![0u32, 0u32, 0x0108_2ab8u32])
        .u32();
    e.mem.set_u32(at(LOCAL_MUTEX), mutex);
    if e.call(API_GET_LAST_ERROR, &args![]).u32() == ERROR_ALREADY_EXISTS {
        e.call(
            API_MESSAGE_BOX,
            &args![0u32, 0x0108_2a7cu32, 0x0104_df38u32, 0u32],
        );
        return 0;
    }
    if e.call(API_GET_SYSTEM_METRICS, &args![SM_REMOTESESSION])
        .u32()
        != 0
    {
        e.call(
            API_MESSAGE_BOX,
            &args![0u32, 0x0108_2a48u32, 0x0104_df38u32, 0u32],
        );
        return 0;
    }

    open_log_files(e, 1);
    e.call(API_GET_SYSTEM_INFO, &args![at(LOCAL_SYSTEM_INFO)]);
    let processors = e.mem.u32(at(LOCAL_PROCESSOR_COUNT));
    let setting = e.call(SETTING_INT_PTR, &args![0x011c_3ea4u32]).u32();
    e.mem.set_u32(setting, processors);
    e.call(0x0087_6ad0, &args![at(LOCAL_PATH_BUFFER)]);
    raise_single_processor_count(e);

    let window_class_name = e.global::<u32>(0x011a_2fe8);
    let running = e
        .call(API_FIND_WINDOW, &args![window_class_name, 0u32])
        .u32();
    e.mem.set_u32(at(LOCAL_FOUND_WINDOW), running);
    if running != 0 {
        e.call(API_SET_FOREGROUND_WINDOW, &args![running]);
        return 0;
    }

    e.call(0x004b_4a30, &args![window_class_name]);
    // `PUSH x; CALL 00af2640; MOV ECX,EAX; CALL method(x)`: 00af2640 takes no argument.
    let system_utility = e.call(0x00af_2640, &args![]).u32();
    e.call(0x004f_b090, &args![system_utility, 0x0086_9f30u32]);
    let system_utility = e.call(0x00af_2640, &args![]).u32();
    let dialog_utility = e.call(0x0085_26c0, &args![system_utility]).u32();
    e.call(0x006e_cd40, &args![dialog_utility, 0x0086_a020u32]);
    e.call(0x00aa_20b0, &args![0u32]);
    e.call(0x00c5_d4e0, &args![]);
    e.call(0x004b_7920, &args![]);
    e.call(0x00a1_c6f0, &args![]);
    e.call(0x00bc_2940, &args![]);
    e.call(0x0045_b020, &args![]);
    e.call(0x0040_4f30, &args![0x3bu32]);
    fn_0086bde0(e, 0x0086_a050);
    fn_0086bcd0(e, 0x00b4_dfb0);
    fn_0086bcc0(e, 1);
    e.call(0x00c3_a6d0, &args![]);
    fn_0086bd70(e);

    let scope = at(LOCAL_SCOPE);
    e.call(
        0x0040_4eb0,
        &args![scope, 0x10u32, 1u32, 0x0108_29c4u32, 0x8e0u32],
    );
    e.call(0x0045_3720, &args![]);
    e.call(0x0040_4f70, &args![scope]);
    scope_begin(e, scope, 0xf, 0x8e6);
    e.call(0x00a6_1ad0, &args![]);
    e.call(0x0040_4f70, &args![scope]);
    e.call(0x006f_f610, &args![]);

    // The command line: `-nocloud` switches two settings off.
    e.mem.set_u32(at(LOCAL_ARG_COUNT), 0);
    let command_line = e.call(API_GET_COMMAND_LINE, &args![]).u32();
    let arguments = e
        .call(
            API_COMMAND_LINE_TO_ARGV,
            &args![command_line, at(LOCAL_ARG_COUNT)],
        )
        .u32();
    e.mem.set_u32(at(LOCAL_ARGS), arguments);
    if arguments != 0 {
        let count = e.mem.i32(at(LOCAL_ARG_COUNT));
        let mut index = 0i32;
        while index < count {
            let argument = e.mem.u32(arguments + 4 * index as u32);
            // A wide string starting with '-'.
            if argument != 0 && e.mem.u16(argument) == 0x2d {
                // `__wcsicmp(argument + 1 character, L"nocloud")`
                let matched = e
                    .call(0x00ec_c071, &args![argument + 2, 0x0108_29b4u32])
                    .u32();
                if matched == 0 {
                    // `PUSH 0; CALL 006ff580; MOV ECX,EAX; CALL 006ff880`
                    let object = e.call(0x006f_f580, &args![]).u32();
                    e.call(0x006f_f880, &args![object, 0u32]);
                }
            }
            index += 1;
        }
        e.call(API_LOCAL_FREE, &args![arguments]);
    }

    // The registry key and the INI file paths.
    let path = at(LOCAL_PATH_BUFFER);
    let key_owner = e.call(0x0087_7950, &args![]).u32();
    e.call(0x005e_0200, &args![key_owner, 0x0108_2990u32]);
    let object = e.call(0x0044_f560, &args![]).u32();
    e.call(0x005e_0200, &args![object, path]);
    let source = e.call(0x004d_c110, &args![]).u32();
    e.call(0x0040_6d30, &args![path, 0x104u32, source]);
    let ini_name = e.global::<u32>(0x011a_2ff0);
    e.call(0x0040_6d50, &args![path, 0x104u32, ini_name]);
    let object = e.call(0x004d_e490, &args![]).u32();
    e.call(0x005e_0200, &args![object, path]);
    let source = e.call(0x004d_c110, &args![]).u32();
    e.call(0x0040_6d30, &args![path, 0x104u32, source]);
    let prefs_name = e.global::<u32>(0x011a_2ff8);
    e.call(0x0040_6d50, &args![path, 0x104u32, prefs_name]);
    let object = e.call(0x0045_d180, &args![]).u32();
    e.call(0x005e_0200, &args![object, path]);
    let object = e.call(0x004d_e490, &args![]).u32();
    e.call(0x005e_0200, &args![object, path]);
    raise_single_processor_count(e);

    // Hand over to the launcher when the launcher is wanted and starts.
    if e.call(0x004d_a570, &args![]).bool() {
        e.call(0x004d_a4d0, &args![1u32]);
        let tasklet_manager = e.call(0x00b0_0a00, &args![]).u32();
        e.vcall(tasklet_manager, 8, &args![]);
        let launcher = at(LOCAL_LAUNCHER_PATH);
        e.call(0x0087_6a70, &args![launcher, 0x104u32]);
        e.call(0x0040_6d50, &args![launcher, 0x104u32, 0x0108_2978u32]);
        let started = e
            .call(
                API_SHELL_EXECUTE,
                &args![0u32, 0u32, launcher, 0u32, 0u32, 1u32],
            )
            .u32();
        if started != 0 {
            e.call(0x0040_4ee0, &args![scope]);
            return 0;
        }
    }

    let text = e.call(GET_POOLED_TEXT, &args![0x011d_ed2cu32]).u32();
    let other_text = e.call(GET_POOLED_TEXT, &args![0x011d_ee78u32]).u32();
    let system = e.call(0x00af_f100, &args![]).u32();
    e.call(0x00b0_12f0, &args![system, other_text, text]);
    let block = e.call(ALLOCATE_OBJECT, &args![0x10u32]).u32();
    if block != 0 {
        let setting = e.call(SETTING_INT_PTR, &args![0x011d_ee6cu32]).u32();
        let setting_value = e.mem.u32(setting);
        let pooled = e.call(GET_POOLED_TEXT, &args![0x011d_ece4u32]).u32();
        e.call(0x00af_dc60, &args![block, pooled, setting_value, 0u32]);
    }
    fn_0086bce0(e, 0x0105_bb14);
    open_log_files(e, 1);

    scope_begin(e, scope, 0x18, 0x992);
    e.call(ERROR_LOG, &args![]);
    let system = e.call(0x00af_f100, &args![]).u32();
    fn_0086bda0(e, Ptr::new(system));
    e.call(0x0040_4f70, &args![scope]);
    scope_begin(e, scope, 0x13, 0x99c);
    e.call(0x0087_6d20, &args![]);
    e.call(0x0040_4f70, &args![scope]);

    // The window class and the window.
    let class = at(LOCAL_WINDOW_CLASS);
    let icon = e.call(API_LOAD_ICON, &args![instance, 0x65u32]).u32();
    let background = e.call(API_GET_STOCK_OBJECT, &args![4u32]).u32();
    e.mem.set_u32(class, 3); // style
    e.mem.set_u32(class + 4, 0x0086_a0a0); // window procedure
    e.mem.set_u32(class + 8, 0); // extra class bytes
    e.mem.set_u32(class + 12, 0); // extra window bytes
    e.mem.set_u32(class + 16, instance);
    e.mem.set_u32(class + 20, icon);
    e.mem.set_u32(class + 24, 0); // cursor
    e.mem.set_u32(class + 28, background);
    e.mem.set_u32(class + 32, 0); // menu name
    e.mem.set_u32(class + 36, window_class_name);
    e.call(API_REGISTER_CLASS, &args![class]);
    let rect = at(LOCAL_WINDOW_RECT);
    e.mem.set_u32(rect, 0);
    e.mem.set_u32(rect + 4, 0);
    e.mem.set_u32(rect + 8, 0x140);
    e.mem.set_u32(rect + 12, 0xf0);
    e.call(API_ADJUST_WINDOW_RECT, &args![rect, 0x1000_0000u32, 0u32]);
    let width = e.mem.u32(rect + 8).wrapping_sub(e.mem.u32(rect));
    let height = e.mem.u32(rect + 12).wrapping_sub(e.mem.u32(rect + 4));
    let window = e
        .call(
            API_CREATE_WINDOW,
            &args![
                0u32,
                window_class_name,
                window_class_name,
                0x1000_0000u32,
                0u32,
                0u32,
                width,
                height,
                0u32,
                0u32,
                instance,
                0u32
            ],
        )
        .u32();
    e.mem.set_u32(at(LOCAL_WINDOW), window);

    // `Main` (0xA4 bytes).
    let block = e.call(ALLOCATE_OBJECT, &args![0xa4u32]).u32();
    let main_object = if block != 0 {
        e.call(0x0086_c160, &args![block, window, instance]).u32()
    } else {
        0
    };
    e.set_global(MAIN_OBJECT, main_object);
    scope_begin(e, scope, 0xd, 0x9f8);
    let block = e.call(ALLOCATE_OBJECT, &args![8u32]).u32();
    let second_object = if block != 0 {
        e.call(0x0070_05d0, &args![block]).u32()
    } else {
        0
    };
    e.set_global(0x011d_8804, second_object);
    e.call(0x0040_4f70, &args![scope]);

    e.call(ERROR_LOG, &args![0x0108_295cu32]);
    let main_object = e.global::<u32>(MAIN_OBJECT);
    e.call(0x0086_d500, &args![main_object]);
    e.set_global(0x011f_4509, 1u8);
    let renderer = e.call(GET_RENDERER, &args![]).u32();
    e.call(0x004a_0370, &args![renderer]);
    let renderer = e.call(GET_RENDERER, &args![]).u32();
    fn_0086bbd0(e, Ptr::new(renderer), 0x005d_4a40, 0);
    let renderer = e.call(GET_RENDERER, &args![]).u32();
    fn_0086bae0(e, Ptr::new(renderer), 0x0087_2570, 0);
    main_shader_init(e);
    let main_object = e.global::<u32>(MAIN_OBJECT);
    e.call(0x0086_d590, &args![main_object]);
    e.call(0x0070_2250, &args![1u32]);
    e.call(0x0065_19f0, &args![]);
    let main_object = e.global::<u32>(MAIN_OBJECT);
    e.call(0x0087_1c90, &args![main_object, 1u32]);
    e.call(0x006a_2920, &args![]);
    fn_0086c010(e);
    let renderer = e.call(GET_RENDERER, &args![]).u32();
    e.call(0x004a_03c0, &args![renderer]);
    e.call(0x0086_6ff0, &args![]);
    let pooled = e.call(GET_POOLED_TEXT, &args![0x011d_e72cu32]).u32();
    let movie_player = e.global::<u32>(0x0126_fac4);
    // MoviePlayer::StartSequence (Xbox PDB): text, 0, -1, 1, 1, 0.0f
    e.call(
        0x00ec_16d0,
        &args![movie_player, pooled, 0u32, u32::MAX, 1u32, 1u32, 0.0f32],
    );
    e.call(0x0040_4f70, &args![scope]);
    let object = e.call(0x0045_c670, &args![]).u32();
    let main_object = e.global::<u32>(MAIN_OBJECT);
    e.call(0x0086_cf20, &args![main_object, object]);
    let object = e.call(0x0045_c670, &args![]).u32();
    let animation_data = e.call(0x0066_29f0, &args![object]).u32();
    fn_0086be90(e, animation_data);
    let object = e.call(0x0045_c670, &args![]).u32();
    let animation_data = e.call(0x0066_29f0, &args![object]).u32();
    fn_0086be80(e, animation_data);
    let processors = e.call(SETTING_INT_PTR, &args![0x011c_3ea4u32]).u32();
    if e.mem.i32(processors) > 1 {
        let manager = e.call(0x0071_3d80, &args![]).u32();
        e.call(0x008c_7290, &args![manager]);
    }
    e.call(0x006d_0870, &args![]);
    let path_manager = e.call(0x0047_d0b0, &args![]).u32();
    e.call(0x006e_b8c0, &args![path_manager]);
    if setting_byte(e, 0x011d_73e4) != 0 {
        let obstacles = e.call(0x006c_0720, &args![]).u32();
        e.call(0x006c_07d0, &args![obstacles]);
    }
    e.call(0x004e_0540, &args![]);
    e.call(0x004d_eb30, &args![]);

    e.mem.set_u32(at(LOCAL_START_CELL), 0);
    let size_a: f32 = e.global(0x0101_8bfc);
    let size_b: f32 = e.global(0x0101_e704);
    e.call(
        0x0041_6870,
        &args![at(LOCAL_START_POSITION), size_a, size_a, size_b],
    );
    let data_handler = e.global::<u32>(0x011c_3f2c);
    let object = e.call(0x0046_0140, &args![data_handler]).u32();
    let world_space = e.call(0x0068_15c0, &args![object]).u32();
    let world_space = e.mem.u32(world_space);
    let tes = e.global::<u32>(0x011d_ea10);
    e.call(0x0045_8200, &args![tes, world_space]);
    let player = e.global::<u32>(0x011d_ea3c);
    e.call(0x0095_ee80, &args![player]);
    let object = e.global::<u32>(0x011d_e45c);
    e.call(0x008a_8150, &args![object, 1u32]);
    e.call(ERROR_LOG, &args![0x0108_293cu32]);
    e.call(0x0097_0d50, &args![0x011e_0e80u32]);
    let data_handler = e.global::<u32>(0x011c_3f2c);
    e.call(0x0046_14e0, &args![data_handler]);
    let object = e.global::<u32>(0x011d_df38);
    e.call(0x007d_6bd0, &args![object, 1u32]);
    e.call(ERROR_LOG, &args![0x0108_2924u32]);
    let start_cell = e.mem.u32(at(LOCAL_START_CELL));
    e.call(0x0087_6dd0, &args![start_cell, at(LOCAL_START_POSITION)]);
    e.call(ERROR_LOG, &args![0x0108_2910u32]);
    // The start position is passed by value: three words.
    let x = e.mem.u32(at(LOCAL_START_POSITION));
    let y = e.mem.u32(at(LOCAL_START_POSITION + 4));
    let z = e.mem.u32(at(LOCAL_START_POSITION + 8));
    e.call(0x0087_7260, &args![x, y, z]);
    e.call(0x0070_22c0, &args![]);
    let player = e.global::<u32>(0x011d_ea3c);
    e.call(0x0048_3710, &args![player]);
    let main_object = e.global::<u32>(MAIN_OBJECT);
    e.call(0x004f_15a0, &args![main_object, 1u32]);
    e.call(0x00af_4540, &args![]);
    let player = e.global::<u32>(0x011d_ea3c);
    if e.call(0x008d_6f30, &args![player]).u32() == 0 {
        let data_handler = e.global::<u32>(0x011c_3f2c);
        // TESDataHandler::SetMasterFileLargeBuffer (Xbox PDB)
        e.call(0x0046_50a0, &args![data_handler, u32::MAX]);
        e.call(ERROR_LOG, &args![]);
        e.call(0x0070_53f0, &args![]);
        // Controls::SetCurrentController (Xbox PDB)
        let controls = e.global::<u32>(0x0126_fac4);
        e.call(0x00ec_1800, &args![controls]);
        e.call(ERROR_LOG, &args![]);
    } else {
        let controls = e.global::<u32>(0x0126_fac4);
        e.call(0x00ec_1800, &args![controls]);
    }
    let io_manager = e.global::<u32>(0x0120_2d98);
    // IOManager::LoadQueuedPriority (Xbox PDB)
    e.call(0x0045_6520, &args![io_manager]);
    e.call(ERROR_LOG, &args![0x0108_28fcu32]);

    // The message loop.
    let message = at(LOCAL_MESSAGE);
    e.call(0x0040_3d30, &args![message, 0u32, 0x1cu32]);
    let main_object = e.global::<u32>(MAIN_OBJECT);
    let active_window = e.call(0x0044_ddc0, &args![main_object]).u32();
    e.mem.set_u32(at(LOCAL_ACTIVE_WINDOW), active_window);
    // "The game is in the background": audio and the loading thread are suspended.
    let mut background = false;
    loop {
        while e
            .call(API_PEEK_MESSAGE, &args![message, 0u32, 0u32, 0u32, 1u32])
            .u32()
            != 0
        {
            e.call(API_TRANSLATE_MESSAGE, &args![message]);
            e.call(API_DISPATCH_MESSAGE, &args![message]);
        }
        let foreground = e.call(API_GET_ACTIVE_WINDOW, &args![]).u32() == active_window
            || {
                let tes = e.global::<u32>(0x011d_ea10);
                e.call(0x0045_1530, &args![tes]).bool()
            }
            || e.call(0x0055_9450, &args![0x011d_eb30u32]).u32() != 0
            || setting_byte(e, 0x011d_eed8) != 0;
        if foreground {
            let renderer = e.call(GET_RENDERER, &args![]).u32();
            let device = e.call(0x004d_c020, &args![renderer]).u32();
            let frame_result = e.vcall(device, 0xc, &args![device]).i32();
            if frame_result < 0 {
                let renderer = e.call(GET_RENDERER, &args![]).u32();
                fn_0086ba10(e, Ptr::new(renderer));
                let renderer = e.call(GET_RENDERER, &args![]).u32();
                fn_0086ba90(e, Ptr::new(renderer));
                e.call(API_SLEEP, &args![0x32u32]);
            } else {
                let main_object = e.global::<u32>(MAIN_OBJECT);
                e.call(0x0086_e650, &args![main_object]);
            }
            if background {
                background = false;
                let audio = e.call(0x0045_3a70, &args![]).u32();
                e.call(0x00ad_8740, &args![audio]);
                e.call(0x0083_0660, &args![]);
                e.call(0x0083_2ad0, &args![0u32]);
                let audio = e.call(0x0045_3a70, &args![]).u32();
                e.call(0x00ad_7740, &args![audio, 1u32]);
                if e.call(GET_WINDOW_SETTING, &args![]).bool() {
                    resize_window_to_screen(e, window, at(LOCAL_RESIZE_RECT_ACTIVE));
                }
            }
            let main_object = e.global::<u32>(MAIN_OBJECT);
            if !background && e.mem.u8(main_object) != 0 && frame_result >= 0 {
                // LoadingMenu::ResumeBackgroundThread (Xbox PDB)
                e.call(0x0078_d020, &args![]);
                let main_object = e.global::<u32>(MAIN_OBJECT);
                e.mem.set_u8(main_object, 0);
            }
        } else {
            if !background {
                background = true;
                if fn_0086bdf0(e) == 0 {
                    let thread = e.call(API_GET_CURRENT_THREAD_ID, &args![]).u32();
                    let main_object = e.global::<u32>(MAIN_OBJECT);
                    let main_thread = e.call(GET_MAIN_THREAD_ID, &args![main_object]).u32();
                    if thread == main_thread {
                        // LoadingMenu::SuspendBackgroundThread (Xbox PDB)
                        e.call(0x0078_cfc0, &args![]);
                        let main_object = e.global::<u32>(MAIN_OBJECT);
                        e.mem.set_u8(main_object, 1);
                    }
                }
                let audio = e.call(0x0045_3a70, &args![]).u32();
                e.call(0x00ad_8700, &args![audio]);
                e.call(0x0083_0640, &args![]);
                e.call(0x0083_2ad0, &args![0u32]);
                let audio = e.call(0x0045_3a70, &args![]).u32();
                e.call(0x00ad_7740, &args![audio, 1u32]);
                let main_object = e.global::<u32>(MAIN_OBJECT);
                let object = e.call(0x004e_2190, &args![main_object]).u32();
                e.call(0x00b6_3620, &args![object]);
                let object = e.call(0x0049_fef0, &args![]).u32();
                e.call(0x004a_4340, &args![object]);
                let object = e.call(0x0070_5910, &args![]).u32();
                e.call(0x0080_26b0, &args![object]);
                if e.call(GET_WINDOW_SETTING, &args![]).bool() {
                    resize_window_to_screen(e, window, at(LOCAL_RESIZE_RECT_IDLE));
                }
            }
            e.call(API_SLEEP, &args![0x32u32]);
        }
        let main_object = e.global::<u32>(MAIN_OBJECT);
        if e.call(0x005d_c960, &args![main_object]).bool() {
            break;
        }
        let main_object = e.global::<u32>(MAIN_OBJECT);
        if e.call(0x0067_8ce0, &args![main_object]).bool() {
            let main_object = e.global::<u32>(MAIN_OBJECT);
            e.call(0x0087_74a0, &args![main_object, 1u32]);
        }
    }

    // Shutdown.
    let main_object = e.global::<u32>(MAIN_OBJECT);
    e.call(0x004f_15a0, &args![main_object, 0u32]);
    e.call(0x006f_f620, &args![]);
    e.call(0x0070_3610, &args![]);
    e.call(0x0070_6320, &args![0u32]);
    release_held_texture(e, 0x011d_ed3c);
    release_held_texture(e, 0x011d_ec64);
    release_held_texture(e, 0x011d_eb38);
    fn_0086bea0(e, 0);
    if e.call(0x0068_3a60, &args![]).u32() != 0 {
        e.call(0x00a8_19e0, &args![]);
    }
    let object = e.global::<u32>(0x011c_3b3c);
    e.call(0x0044_8420, &args![object]);
    let processors = e.call(SETTING_INT_PTR, &args![0x011c_3ea4u32]).u32();
    if e.mem.i32(processors) > 1 {
        let manager = e.call(0x0071_3d80, &args![]).u32();
        e.call(0x008c_73a0, &args![manager]);
    }
    e.call(0x0086_7090, &args![]);
    e.call(0x004d_c560, &args![]);
    e.call(0x004d_c590, &args![]);
    e.call(0x0070_2330, &args![]);
    let tes = e.global::<u32>(0x011d_ea10);
    if e.call(0x008d_8520, &args![tes]).u32() != 0 {
        let tes = e.global::<u32>(0x011d_ea10);
        let object = e.call(0x008d_8520, &args![tes]).u32();
        fn_0086be30(e, object);
    }
    e.call(0x004e_0800, &args![]);
    e.call(0x004d_ef20, &args![]);
    e.call(0x007f_fe00, &args![]);
    e.call(0x0083_2010, &args![]);
    e.call(0x0083_04a0, &args![]);
    e.call(0x0082_f9c0, &args![]);
    let audio = e.call(0x0045_3a70, &args![]).u32();
    e.vcall(audio, 8, &args![]);
    e.call(ERROR_LOG, &args![]);
    e.call(0x0049_ff80, &args![]);
    e.call(0x0062_2bc0, &args![]);
    e.call(0x0062_6f10, &args![]);
    if setting_byte(e, 0x011d_73e4) != 0 {
        let obstacles = e.call(0x006c_0720, &args![]).u32();
        e.call(0x006c_0900, &args![obstacles]);
    }
    let tes = e.global::<u32>(0x011d_ea10);
    if tes != 0 {
        fn_0086bee0(e, Ptr::new(tes), 1);
    }
    fn_0086beb0(e);
    let path_manager = e.call(0x0047_d0b0, &args![]).u32();
    e.call(0x006e_b980, &args![path_manager]);
    e.call(0x0065_1ab0, &args![]);
    e.call(0x0099_0ff0, &args![]);
    let tasklet_manager = e.call(0x00b0_0a00, &args![]).u32();
    e.vcall(tasklet_manager, 8, &args![]);
    e.call(0x0066_4940, &args![]);
    e.call(0x0052_6eb0, &args![]);
    // EffectSettingCollection::Reset (Xbox PDB)
    e.call(0x0040_8de0, &args![]);
    fn_0086bdd0(e);
    let second_object = e.global::<u32>(0x011d_8804);
    if second_object != 0 {
        fn_0086bf10(e, Ptr::new(second_object), 1);
    }
    let main_object = e.global::<u32>(MAIN_OBJECT);
    if main_object != 0 {
        fn_0086bf40(e, Ptr::new(main_object), 1);
    }
    e.set_global(MAIN_OBJECT, 0u32);
    let system = e.call(0x00af_f100, &args![]).u32();
    e.vcall(system, 8, &args![]);
    let object = e.call(0x004d_e490, &args![]).u32();
    let package = e.call(0x0071_7e50, &args![object]).u32();
    let object = e.call(0x004d_e490, &args![]).u32();
    e.call(0x005e_01b0, &args![object, package]);
    let renderer = e.call(GET_RENDERER, &args![]).u32();
    fn_0086bb20(e, Ptr::new(renderer), 0x0087_2570);
    let renderer = e.call(GET_RENDERER, &args![]).u32();
    fn_0086bc10(e, Ptr::new(renderer), 0x005d_4a40);
    e.call(0x006a_29a0, &args![]);
    e.call(0x00a6_2090, &args![]);
    e.call(0x00bc_2960, &args![]);
    // Renderer::Kill (Xbox PDB)
    e.call(0x004d_a4d0, &args![1u32]);
    fn_0086bd00(e);
    e.call(0x0045_b050, &args![]);
    e.call(0x00c5_d520, &args![]);
    e.call(0x004b_8cf0, &args![]);
    e.call(0x00a1_c790, &args![]);
    e.call(0x00aa_2170, &args![]);
    let exit_data = at(LOCAL_EXIT_DATA);
    e.call(0x0087_83c0, &args![exit_data, 0x0108_28ecu32]);
    let object = e.call(0x0040_1020, &args![]).u32();
    e.call(0x00aa_47c0, &args![object, exit_data, 0u32]);
    e.call(0x0087_84d0, &args![exit_data]);
    e.call(0x0040_4ee0, &args![scope]);
    0
}

// ---------------------------------------------------------------------------------------------
// The renderer's callback arrays and off-screen frame guards.
// ---------------------------------------------------------------------------------------------

// Translated from 0086ba10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Starts an off-screen frame on the renderer `this`: only when its frame state is 0 and its
/// virtual method at +0x17C accepts, the state becomes 2. Returns whether it did.
pub fn fn_0086ba10(e: &mut Engine, this: Ptr) -> bool {
    if !fn_0086ba60(e, this, 0x0108_2acc, 0) {
        return false;
    }
    if !e.vcall(this.addr(), 0x17c, &args![]).bool() {
        return false;
    }
    e.mem.set_u32(this.addr() + RENDERER_FRAME_STATE, 2);
    true
}

// Translated from 0086ba60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the renderer's frame state (`NiRenderer::m_eFrameState`, +0x200) is `expected`.
/// The first argument (the name of the caller's operation, for a message) is not used.
pub fn fn_0086ba60(e: &mut Engine, this: Ptr, _unused_1: u32, expected: u32) -> bool {
    e.mem.u32(this.addr() + RENDERER_FRAME_STATE) == expected
}

// Translated from 0086ba90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Ends an off-screen frame on the renderer `this`: only when its frame state is 2 and its
/// virtual method at +0x180 accepts, the state becomes 0. Returns whether it did.
pub fn fn_0086ba90(e: &mut Engine, this: Ptr) -> bool {
    if !fn_0086ba60(e, this, 0x0108_2ae0, 2) {
        return false;
    }
    if !e.vcall(this.addr(), 0x180, &args![]).bool() {
        return false;
    }
    e.mem.set_u32(this.addr() + RENDERER_FRAME_STATE, 0);
    true
}

/// Adds `first` to the array at `this + key_array` and `second` at the same index of the array
/// at `this + data_array` (both methods take the address of the value); returns the index.
fn add_to_parallel_arrays(
    e: &mut Engine,
    this: Ptr,
    key_array: u32,
    data_array: u32,
    first: u32,
    second: u32,
) -> u32 {
    e.with_stack(8, |e, values| {
        e.mem.set_u32(values.addr(), first);
        e.mem.set_u32(values.addr() + 4, second);
        let index = e
            .call(ARRAY_ADD, &args![this.addr() + key_array, values])
            .u32();
        e.call(
            ARRAY_SET_AT_GROW,
            &args![this.addr() + data_array, index, values.addr() + 4],
        );
        index
    })
}

/// Index of the first element of the array at `this + key_array` equal to `key`, or
/// `u32::MAX`.
fn find_in_array(e: &mut Engine, this: Ptr, key_array: u32, key: u32) -> u32 {
    let array = this.addr() + key_array;
    let mut index = 0u32;
    while index < e.call(ARRAY_COUNT, &args![array]).u32() {
        let element = e.call(ARRAY_ELEMENT, &args![array, index]).u32();
        if e.mem.u32(element) == key {
            return index;
        }
        index += 1;
    }
    u32::MAX
}

// Translated from 0086bae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Registers `first` in the renderer's first key array and `second` at the same index of the
/// data array after it (+0xAD4 / +0xAE4); returns the index.
pub fn fn_0086bae0(e: &mut Engine, this: Ptr, first: u32, second: u32) -> u32 {
    add_to_parallel_arrays(e, this, KEY_ARRAY_A, DATA_ARRAY_A, first, second)
}

// Translated from 0086bb20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes `key` and its partner from the renderer's first pair of arrays (+0xAD4 / +0xAE4).
/// Returns whether it was found.
pub fn fn_0086bb20(e: &mut Engine, this: Ptr, key: u32) -> bool {
    let index = fn_0086bb70(e, this, key);
    if index == u32::MAX {
        return false;
    }
    e.call(ARRAY_REMOVE_AT, &args![this.addr() + KEY_ARRAY_A, index]);
    e.call(ARRAY_REMOVE_AT, &args![this.addr() + DATA_ARRAY_A, index]);
    true
}

// Translated from 0086bb70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Index of `key` in the renderer's first key array (+0xAD4), or `0xFFFFFFFF`.
pub fn fn_0086bb70(e: &mut Engine, this: Ptr, key: u32) -> u32 {
    find_in_array(e, this, KEY_ARRAY_A, key)
}

// Translated from 0086bbd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Registers `first` in the renderer's second key array and `second` at the same index of the
/// data array after it (+0xAF4 / +0xB04); returns the index.
pub fn fn_0086bbd0(e: &mut Engine, this: Ptr, first: u32, second: u32) -> u32 {
    add_to_parallel_arrays(e, this, KEY_ARRAY_B, DATA_ARRAY_B, first, second)
}

// Translated from 0086bc10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes `key` and its partner from the renderer's second pair of arrays (+0xAF4 / +0xB04).
/// Returns whether it was found.
pub fn fn_0086bc10(e: &mut Engine, this: Ptr, key: u32) -> bool {
    let index = fn_0086bc60(e, this, key);
    if index == u32::MAX {
        return false;
    }
    e.call(ARRAY_REMOVE_AT, &args![this.addr() + KEY_ARRAY_B, index]);
    e.call(ARRAY_REMOVE_AT, &args![this.addr() + DATA_ARRAY_B, index]);
    true
}

// Translated from 0086bc60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Index of `key` in the renderer's second key array (+0xAF4), or `0xFFFFFFFF`.
pub fn fn_0086bc60(e: &mut Engine, this: Ptr, key: u32) -> u32 {
    find_in_array(e, this, KEY_ARRAY_B, key)
}

// ---------------------------------------------------------------------------------------------
// Global setters and small wrappers.
// ---------------------------------------------------------------------------------------------

// Translated from 0086bcc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte global at `011f4448`.
pub fn fn_0086bcc0(e: &mut Engine, value: u8) {
    e.set_global(0x011f_4448, value);
}

// Translated from 0086bcd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the 32-bit global at `011f6e08`.
pub fn fn_0086bcd0(e: &mut Engine, value: u32) {
    e.set_global(0x011f_6e08, value);
}

/// The object at `011f81dc`, built in `main` by `00afdc60`.
const OBJECT_AT_011F81DC: u32 = 0x011f_81dc;

// Translated from 0086bce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00afdda0(argument)` on the object at `011f81dc`, when there is one.
pub fn fn_0086bce0(e: &mut Engine, argument: u32) {
    let object = e.global::<u32>(OBJECT_AT_011F81DC);
    if object != 0 {
        e.call(0x00af_dda0, &args![object, argument]);
    }
}

// Translated from 0086bd00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destroys and frees the object at `011f81dc` when there is one (the global itself is left as
/// it was).
pub fn fn_0086bd00(e: &mut Engine) {
    let object = e.global::<u32>(OBJECT_AT_011F81DC);
    if object != 0 {
        fn_0086bd40(e, Ptr::new(object), 1);
    }
}

// Translated from 0086bd40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deleting destructor of the object at `011f81dc`: runs `00afdcf0`, then frees the object when
/// bit 0 of `flags` is set. Returns `this`.
pub fn fn_0086bd40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    deleting_destructor(e, 0x00af_dcf0, this, flags)
}

// Translated from 0086bd70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `00b013f0` into the global at `011f6224` (through [`fn_0086bd90`]).
pub fn fn_0086bd70(e: &mut Engine) {
    fn_0086bd90(e, 0x00b0_13f0);
}

// Translated from 0086bd90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the 32-bit global at `011f6224`.
pub fn fn_0086bd90(e: &mut Engine, value: u32) {
    e.set_global(0x011f_6224, value);
}

// Translated from 0086bda0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Moves the state word at +0x5C from 1 or 2 to 3 (`main` calls it on the object from
/// `00aff100`).
pub fn fn_0086bda0(e: &mut Engine, this: Ptr) {
    let state = e.mem.u32(this.addr() + 0x5c);
    if state == 1 || state == 2 {
        e.mem.set_u32(this.addr() + 0x5c, 3);
    }
}

// Translated from 0086bdd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00438af0` on the object at `011dd948`.
pub fn fn_0086bdd0(e: &mut Engine) {
    e.call(0x0043_8af0, &args![0x011d_d948u32]);
}

// Translated from 0086bde0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the 32-bit global at `011f9134`.
pub fn fn_0086bde0(e: &mut Engine, value: u32) {
    e.set_global(0x011f_9134, value);
}

// Translated from 0086bdf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// 1 when the object at `011da0c4` exists and its method `007cfd00` says yes, else 0.
pub fn fn_0086bdf0(e: &mut Engine) -> u8 {
    let object = e.global::<u32>(0x011d_a0c4);
    if object != 0 && e.call(0x007c_fd00, &args![object]).bool() {
        1
    } else {
        0
    }
}

/// The object at `011ccb78`.
const OBJECT_AT_011CCB78: u32 = 0x011c_cb78;

// Translated from 0086be30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deletes the object at `011ccb78` through its first virtual method (the deleting destructor)
/// when there is one, and clears the global. The word in ECX (`main` passes the result of
/// `008d8520`) is not read.
pub fn fn_0086be30(e: &mut Engine, _unused_0: u32) {
    let object = e.global::<u32>(OBJECT_AT_011CCB78);
    if object != 0 {
        e.vcall(object, 0, &args![1u32]);
    }
    e.set_global(OBJECT_AT_011CCB78, 0u32);
}

// Translated from 0086be80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the 32-bit global at `011d5c44`.
pub fn fn_0086be80(e: &mut Engine, value: u32) {
    e.set_global(0x011d_5c44, value);
}

// Translated from 0086be90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the 32-bit global at `011d59ec`.
pub fn fn_0086be90(e: &mut Engine, value: u32) {
    e.set_global(0x011d_59ec, value);
}

// Translated from 0086bea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte global at `011a31f4`.
pub fn fn_0086bea0(e: &mut Engine, value: u8) {
    e.set_global(0x011a_31f4, value);
}

// Translated from 0086beb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `0066b0d0(0)` on the three objects at `011d86bc`, `011d86a8` and `011d8690`.
pub fn fn_0086beb0(e: &mut Engine) {
    for object in [0x011d_86bcu32, 0x011d_86a8, 0x011d_8690] {
        e.call(0x0066_b0d0, &args![object, 0u32]);
    }
}

/// Shape shared by the deleting destructors: runs the destructor body, frees the object when
/// bit 0 of `flags` is set and returns it.
fn deleting_destructor(e: &mut Engine, body: u32, this: Ptr, flags: u32) -> Ptr {
    e.call(body, &args![this]);
    if flags & 1 != 0 {
        e.call(FREE_OBJECT, &args![this]);
    }
    this
}

// Translated from 0086bee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deleting destructor: body `00450770`, free when bit 0 of `flags` is set; returns `this`.
pub fn fn_0086bee0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    deleting_destructor(e, 0x0045_0770, this, flags)
}

// Translated from 0086bf10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deleting destructor: body `00700650`, free when bit 0 of `flags` is set; returns `this`.
pub fn fn_0086bf10(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    deleting_destructor(e, 0x0070_0650, this, flags)
}

// Translated from 0086bf40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Deleting destructor of `Main`: body `0086c880`, free when bit 0 of `flags` is set; returns
/// `this`.
pub fn fn_0086bf40(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    deleting_destructor(e, 0x0086_c880, this, flags)
}

// Translated from 0086bf70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Main_ShaderInit` (Xbox PDB): logs, calls `005c5a60(0)`, installs [`shader_timer`] as the
/// timer callback, and copies four settings into the shader globals (bytes at `011f94a7` and
/// `011f9443`, floats at `011f9484` and `011f9480`).
pub fn main_shader_init(e: &mut Engine) {
    e.call(ERROR_LOG, &args![0x0108_2af4u32]);
    e.call(0x005c_5a60, &args![0u32]);
    fn_0086bff0(e, 0x0086_a480);
    let value = setting_byte(e, 0x011d_ed68) as u8;
    fn_0086c000(e, value);
    let value = setting_byte(e, 0x011d_eea8) as u8;
    e.set_global(0x011f_9443, value);
    let value = e.call(0x0045_0410, &args![0x011d_ec18u32]).f32();
    e.set_global(0x011f_9484, value);
    let value = e.call(0x0045_0410, &args![0x011d_eb70u32]).f32();
    e.set_global(0x011f_9480, value);
}

// Translated from 0086bff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the 32-bit global at `011f91f0` (the shader timer callback).
pub fn fn_0086bff0(e: &mut Engine, value: u32) {
    e.set_global(0x011f_91f0, value);
}

// Translated from 0086c000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte global at `011f94a7`.
pub fn fn_0086c000(e: &mut Engine, value: u8) {
    e.set_global(0x011f_94a7, value);
}

// Translated from 0086c010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads float settings through the `0086c0xx` / `0086c1xx` getters: when the byte at
/// `011f941e` is 0, `0086c0c0` and `0086c0d0` go to `0086c140` and `0086c150` and `0086c0e0`
/// and `0086c0f0` to the globals `011adbcc` / `011adbd0`; when it is set, `0086c120` and
/// `0086c130` go to the globals and `0086c100` and `0086c110` to `0086c140` and `0086c150`.
/// Then the object `004e3270` returns is asked (`004ebbc0`, index 0xC) for an entry, and the
/// first float of the object `00403e20` returns for `011cf8a4` is passed to `005b9f80` on that
/// entry (`004e3270` takes no argument; the two stack words belong to the next two calls).
pub fn fn_0086c010(e: &mut Engine) {
    if e.global::<u8>(0x011f_941e) != 0 {
        let value = e.call(0x0086_c120, &args![]).f32();
        e.set_global(0x011a_dbcc, value);
        let value = e.call(0x0086_c130, &args![]).f32();
        e.set_global(0x011a_dbd0, value);
        let value = e.call(0x0086_c100, &args![]).f32();
        e.call(0x0086_c140, &args![value]);
        let value = e.call(0x0086_c110, &args![]).f32();
        e.call(0x0086_c150, &args![value]);
    } else {
        let value = fn_0086c0c0(e);
        e.call(0x0086_c140, &args![value]);
        let value = fn_0086c0d0(e);
        e.call(0x0086_c150, &args![value]);
        let value = e.call(0x0086_c0e0, &args![]).f32();
        e.set_global(0x011a_dbcc, value);
        let value = e.call(0x0086_c0f0, &args![]).f32();
        e.set_global(0x011a_dbd0, value);
    }
    let float_address = e.call(0x0040_3e20, &args![0x011c_f8a4u32]).u32();
    let value = e.mem.f32(float_address);
    let object = e.call(0x004e_3270, &args![]).u32();
    let entry = e.call(0x004e_bbc0, &args![object, 0xcu32]).u32();
    e.call(0x005b_9f80, &args![entry, value]);
}

// Translated from 0086c0c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the float setting at `011c72a0` (through `00450410`).
pub fn fn_0086c0c0(e: &mut Engine) -> f32 {
    e.call(0x0045_0410, &args![0x011c_72a0u32]).f32()
}

// Translated from 0086c0d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the float setting at `011c774c` (through `00450410`).
pub fn fn_0086c0d0(e: &mut Engine) -> f32 {
    e.call(0x0045_0410, &args![0x011c_774cu32]).f32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0086a480, shader_timer(i32) -> f32),
        entry!(0x0086a530, fn_0086a530()),
        entry!(0x0086a580, fn_0086a580(u32, u32, u32) -> i32),
        entry!(0x0086a820, fn_0086a820() -> u32),
        entry!(0x0086a830, fn_0086a830(u32, u32) -> u8),
        entry!(0x0086a850, main(u32, u32, u32, u32) -> u32),
        entry!(0x0086ba10, fn_0086ba10(Ptr) -> bool),
        entry!(0x0086ba60, fn_0086ba60(Ptr, u32, u32) -> bool),
        entry!(0x0086ba90, fn_0086ba90(Ptr) -> bool),
        entry!(0x0086bae0, fn_0086bae0(Ptr, u32, u32) -> u32),
        entry!(0x0086bb20, fn_0086bb20(Ptr, u32) -> bool),
        entry!(0x0086bb70, fn_0086bb70(Ptr, u32) -> u32),
        entry!(0x0086bbd0, fn_0086bbd0(Ptr, u32, u32) -> u32),
        entry!(0x0086bc10, fn_0086bc10(Ptr, u32) -> bool),
        entry!(0x0086bc60, fn_0086bc60(Ptr, u32) -> u32),
        entry!(0x0086bcc0, fn_0086bcc0(u8)),
        entry!(0x0086bcd0, fn_0086bcd0(u32)),
        entry!(0x0086bce0, fn_0086bce0(u32)),
        entry!(0x0086bd00, fn_0086bd00()),
        entry!(0x0086bd40, fn_0086bd40(Ptr, u32) -> Ptr),
        entry!(0x0086bd70, fn_0086bd70()),
        entry!(0x0086bd90, fn_0086bd90(u32)),
        entry!(0x0086bda0, fn_0086bda0(Ptr)),
        entry!(0x0086bdd0, fn_0086bdd0()),
        entry!(0x0086bde0, fn_0086bde0(u32)),
        entry!(0x0086bdf0, fn_0086bdf0() -> u8),
        entry!(0x0086be30, fn_0086be30(u32)),
        entry!(0x0086be80, fn_0086be80(u32)),
        entry!(0x0086be90, fn_0086be90(u32)),
        entry!(0x0086bea0, fn_0086bea0(u8)),
        entry!(0x0086beb0, fn_0086beb0()),
        entry!(0x0086bee0, fn_0086bee0(Ptr, u32) -> Ptr),
        entry!(0x0086bf10, fn_0086bf10(Ptr, u32) -> Ptr),
        entry!(0x0086bf40, fn_0086bf40(Ptr, u32) -> Ptr),
        entry!(0x0086bf70, main_shader_init()),
        entry!(0x0086bff0, fn_0086bff0(u32)),
        entry!(0x0086c000, fn_0086c000(u8)),
        entry!(0x0086c010, fn_0086c010()),
        entry!(0x0086c0c0, fn_0086c0c0() -> f32),
        entry!(0x0086c0d0, fn_0086c0d0() -> f32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Every callee of this file's functions that lives elsewhere (each is a no-op returning 0
    /// until a test replaces it).
    const CALLEES: [u32; 208] = [
        0x0040_1000,
        0x0040_1020,
        0x0040_1030,
        0x0040_3d30,
        0x0040_3df0,
        0x0040_3e20,
        0x0040_4eb0,
        0x0040_4ee0,
        0x0040_4f00,
        0x0040_4f30,
        0x0040_4f70,
        0x0040_6d30,
        0x0040_6d50,
        0x0040_8d60,
        0x0040_8de0,
        0x0040_fbe0,
        0x0041_6870,
        0x0043_8af0,
        0x0043_c4b0,
        0x0043_d4d0,
        0x0044_6e10,
        0x0044_8420,
        0x0044_ddc0,
        0x0044_edb0,
        0x0044_f560,
        0x0045_0410,
        0x0045_0770,
        0x0045_1530,
        0x0045_3720,
        0x0045_3a70,
        0x0045_6520,
        0x0045_8200,
        0x0045_b020,
        0x0045_b050,
        0x0045_c670,
        0x0045_d180,
        0x0046_0140,
        0x0046_14e0,
        0x0046_50a0,
        0x0047_0000,
        0x0047_d0b0,
        0x0048_3710,
        0x0049_fef0,
        0x0049_ff80,
        0x004a_0370,
        0x004a_03c0,
        0x004a_0ea0,
        0x004a_4340,
        0x004b_4a30,
        0x004b_7920,
        0x004b_8cf0,
        0x004d_a4d0,
        0x004d_a570,
        0x004d_c020,
        0x004d_c110,
        0x004d_c1e0,
        0x004d_c1f0,
        0x004d_c200,
        0x004d_c560,
        0x004d_c590,
        0x004d_e490,
        0x004d_eb30,
        0x004d_ef20,
        0x004e_0540,
        0x004e_0800,
        0x004e_2190,
        0x004e_3270,
        0x004e_bbc0,
        0x004f_15a0,
        0x004f_b090,
        0x0052_6eb0,
        0x0055_9450,
        0x005b_9f80,
        0x005c_5a60,
        0x005d_c960,
        0x005e_01b0,
        0x005e_0200,
        0x0062_2bc0,
        0x0062_6f10,
        0x0065_19f0,
        0x0065_1ab0,
        0x0065_8930,
        0x0066_29f0,
        0x0066_4940,
        0x0066_b0d0,
        0x0067_8ce0,
        0x0068_15c0,
        0x0068_3a60,
        0x006a_2920,
        0x006a_29a0,
        0x006c_0720,
        0x006c_07d0,
        0x006c_0900,
        0x006d_0870,
        0x006e_b8c0,
        0x006e_b980,
        0x006e_cd40,
        0x006f_f580,
        0x006f_f610,
        0x006f_f620,
        0x006f_f880,
        0x0070_05d0,
        0x0070_0650,
        0x0070_2250,
        0x0070_22c0,
        0x0070_2330,
        0x0070_2360,
        0x0070_3610,
        0x0070_3e80,
        0x0070_3fa0,
        0x0070_3fd0,
        0x0070_53f0,
        0x0070_5910,
        0x0070_6320,
        0x0071_1e00,
        0x0071_3d80,
        0x0071_7e50,
        0x0078_cfc0,
        0x0078_d020,
        0x007c_fd00,
        0x007d_6bd0,
        0x007f_fe00,
        0x0080_26b0,
        0x0082_5c00,
        0x0082_f9c0,
        0x0083_04a0,
        0x0083_0640,
        0x0083_0660,
        0x0083_2010,
        0x0083_2ad0,
        0x0084_d030,
        0x0085_26c0,
        0x0086_6ff0,
        0x0086_7090,
        0x0086_7da0,
        0x0086_c0e0,
        0x0086_c0f0,
        0x0086_c100,
        0x0086_c110,
        0x0086_c120,
        0x0086_c130,
        0x0086_c140,
        0x0086_c150,
        0x0086_c160,
        0x0086_c880,
        0x0086_cf20,
        0x0086_d500,
        0x0086_d590,
        0x0086_e650,
        0x0087_1c90,
        0x0087_6a70,
        0x0087_6ad0,
        0x0087_6d20,
        0x0087_6dd0,
        0x0087_7260,
        0x0087_74a0,
        0x0087_7720,
        0x0087_7950,
        0x0087_7a30,
        0x0087_7a50,
        0x0087_83c0,
        0x0087_84d0,
        0x008a_8150,
        0x008c_7290,
        0x008c_73a0,
        0x008d_6f30,
        0x008d_8520,
        0x0095_ee80,
        0x0097_0d50,
        0x0099_0ff0,
        0x009e_98d0,
        0x00a1_c6f0,
        0x00a1_c790,
        0x00a2_3c00,
        0x00a6_1ad0,
        0x00a6_2090,
        0x00a8_19e0,
        0x00aa_20b0,
        0x00aa_2170,
        0x00aa_47c0,
        0x00aa_5040,
        0x00aa_50a0,
        0x00ad_7740,
        0x00ad_8700,
        0x00ad_8740,
        0x00af_2640,
        0x00af_4540,
        0x00af_dc60,
        0x00af_dcf0,
        0x00af_dda0,
        0x00af_f100,
        0x00b0_0a00,
        0x00b0_12f0,
        0x00b6_3620,
        0x00b6_da10,
        0x00bc_2940,
        0x00bc_2960,
        0x00c3_a6d0,
        0x00c3_bb90,
        0x00c3_bbe0,
        0x00c3_bc30,
        0x00c3_bc80,
        0x00c3_bd30,
        0x00c5_d4e0,
        0x00c5_d520,
        0x00ec_16d0,
        0x00ec_1800,
        0x00ec_c071,
    ];
    const IMPORTS: [u32; 27] = [
        API_CREATE_MUTEX,
        API_GET_LAST_ERROR,
        API_MESSAGE_BOX,
        API_GET_SYSTEM_METRICS,
        API_GET_SYSTEM_INFO,
        API_FIND_WINDOW,
        API_SET_FOREGROUND_WINDOW,
        API_GET_COMMAND_LINE,
        API_COMMAND_LINE_TO_ARGV,
        API_LOCAL_FREE,
        API_SHELL_EXECUTE,
        API_LOAD_ICON,
        API_GET_STOCK_OBJECT,
        API_REGISTER_CLASS,
        API_ADJUST_WINDOW_RECT,
        API_CREATE_WINDOW,
        API_PEEK_MESSAGE,
        API_TRANSLATE_MESSAGE,
        API_DISPATCH_MESSAGE,
        API_GET_ACTIVE_WINDOW,
        API_SLEEP,
        API_GET_CURRENT_THREAD_ID,
        API_GET_WINDOW_LONG,
        API_ADJUST_WINDOW_RECT_EX,
        API_SET_WINDOW_POS,
        API_IS_DEBUGGER_PRESENT,
        API_EXIT_PROCESS,
    ];

    /// A byte that stays 0 and a 32-bit value: what the setting getters point at by default.
    const ZERO_BYTE: u32 = 0x0100_0100;
    const ZERO_WORD: u32 = 0x0100_0200;
    /// A function registered as a no-op, for vtable slots.
    const NOTHING: u32 = 0x0000_1000;

    fn engine() -> Engine {
        let mut e = Engine::new();
        // The exe's constants, strings and globals.
        e.map(0x0100_0000, 0x0028_0000);
        for address in CALLEES.iter().chain(IMPORTS.iter()) {
            e.register(*address, |_, _| Ret::default());
        }
        e.register(NOTHING, |_, _| Ret::default());
        e.register(SETTING_BYTE_PTR, |_, _| ZERO_BYTE.into_ret());
        e.register(SETTING_INT_PTR, |_, _| ZERO_WORD.into_ret());
        e
    }

    fn take_log(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.take().unwrap()
    }

    fn calls_to(log: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn position(log: &[(u32, Vec<u32>)], address: u32) -> usize {
        log.iter()
            .position(|(a, _)| *a == address)
            .unwrap_or_else(|| panic!("{address:08x} was not called"))
    }

    /// An object whose vtable has the given (byte offset, function) slots.
    fn object_with_vtable(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let table = e.mem.alloc(0x200);
        for (offset, function) in slots {
            e.mem.set_u32(table + offset, *function);
        }
        let object = e.mem.alloc(0x300);
        e.mem.set_u32(object, table);
        object
    }

    fn ret_f32(value: f32) -> Ret {
        value.into_ret()
    }

    // ----- shader_timer ---------------------------------------------------------------------

    #[test]
    fn shader_timer_mode_3_is_the_timer_seconds() {
        let mut e = engine();
        e.register(TIMER_GET_SECONDS, |_, _| ret_f32(2.5));
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0086_a480, &args![3i32]).f32(), 2.5);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, TIMER_GET_SECONDS), vec![vec![TIMER]]);
    }

    #[test]
    fn shader_timer_mode_2_is_milliseconds_as_seconds_unless_in_a_menu() {
        let mut e = engine();
        e.set_global(MILLISECONDS_PER_SECOND, 1000.0f64);
        e.register(TIMER_GET_MILLISECONDS, |_, _| 1500u32.into_ret());
        assert_eq!(e.call(0x0086_a480, &args![2i32]).f32(), 1.5);
        e.register(IS_IN_MENU_MODE, |_, _| 1u32.into_ret());
        assert_eq!(e.call(0x0086_a480, &args![2i32]).f32(), 0.0);
    }

    #[test]
    fn shader_timer_mode_1_is_the_timer_seconds_unless_in_a_menu() {
        let mut e = engine();
        e.register(TIMER_GET_SECONDS, |_, _| ret_f32(7.25));
        assert_eq!(e.call(0x0086_a480, &args![1i32]).f32(), 7.25);
        e.register(IS_IN_MENU_MODE, |_, _| 1u32.into_ret());
        assert_eq!(e.call(0x0086_a480, &args![1i32]).f32(), 0.0);
    }

    #[test]
    fn shader_timer_mode_0_is_the_game_hour_in_seconds() {
        let mut e = engine();
        e.set_global(SIXTY, 60.0f64);
        e.register(CALENDAR_GET_HOUR, |_, _| ret_f32(12.5));
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0086_a480, &args![0i32]).f32(), 45000.0);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, CALENDAR_GET_HOUR), vec![vec![CALENDAR]]);
    }

    #[test]
    fn shader_timer_other_modes_are_zero() {
        let mut e = engine();
        assert_eq!(e.call(0x0086_a480, &args![4i32]).f32(), 0.0);
        assert_eq!(e.call(0x0086_a480, &args![-1i32]).f32(), 0.0);
    }

    // ----- the message box --------------------------------------------------------------------

    #[test]
    fn warning_menu_callback_exits_for_results_1_and_3() {
        for result in [1u32, 3] {
            let mut e = engine();
            e.register_double(GET_MESSAGE_MENU_RESULT, move |_, _| result.into_ret());
            e.call_log = Some(vec![]);
            e.call(0x0086_a530, &args![]);
            let log = take_log(&mut e);
            assert_eq!(calls_to(&log, ERROR_LOG), vec![vec![0x0108_28c0]]);
            assert_eq!(calls_to(&log, API_EXIT_PROCESS), vec![vec![0]]);
            assert!(calls_to(&log, CLOSE_MESSAGE_MENU).is_empty());
        }
    }

    #[test]
    fn warning_menu_callback_closes_the_menu_for_result_2_and_ignores_others() {
        let mut e = engine();
        e.register(GET_MESSAGE_MENU_RESULT, |_, _| 2u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0086_a530, &args![]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, CLOSE_MESSAGE_MENU).len(), 1);
        assert!(calls_to(&log, API_EXIT_PROCESS).is_empty());

        e.register(GET_MESSAGE_MENU_RESULT, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0086_a530, &args![]);
        let log = take_log(&mut e);
        assert!(calls_to(&log, CLOSE_MESSAGE_MENU).is_empty());
        assert!(calls_to(&log, API_EXIT_PROCESS).is_empty());
    }

    #[test]
    fn message_box_answers_without_showing_when_the_setting_is_on() {
        let mut e = engine();
        e.mem.set_u8(ZERO_BYTE, 1);
        e.call_log = Some(vec![]);
        // MB_ABORTRETRYIGNORE: IDIGNORE; anything else: IDYES.
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 2u32]).i32(), 5);
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 4u32]).i32(), 6);
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 0x34u32]).i32(), 6);
        let log = take_log(&mut e);
        assert!(calls_to(&log, API_MESSAGE_BOX).is_empty());
        assert!(calls_to(&log, TIMER_DISABLE).is_empty());
        assert_eq!(
            calls_to(&log, SETTING_BYTE_PTR)[0],
            vec![SETTING_NO_MESSAGE_BOXES]
        );
    }

    #[test]
    fn message_box_shows_a_windows_box_around_a_disabled_timer() {
        let mut e = engine();
        e.register(API_MESSAGE_BOX, |_, _| 7u32.into_ret());
        e.call_log = Some(vec![]);
        let result = e
            .call(0x0086_a580, &args![0x111u32, 0x222u32, 0x4u32])
            .i32();
        assert_eq!(result, 7);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, API_MESSAGE_BOX)[0], vec![0, 0x111, 0x222, 4]);
        assert!(position(&log, TIMER_DISABLE) < position(&log, API_MESSAGE_BOX));
        assert!(position(&log, API_MESSAGE_BOX) < position(&log, TIMER_ENABLE));
        assert_eq!(calls_to(&log, TIMER_DISABLE), vec![vec![TIMER]]);
        assert_eq!(calls_to(&log, TIMER_ENABLE), vec![vec![TIMER]]);
        // No warning menu is available, so it is not touched.
        assert!(calls_to(&log, SHOW_MESSAGE_MENU).is_empty());
    }

    #[test]
    fn message_box_is_not_shown_when_the_window_setting_and_state_say_so() {
        let mut e = engine();
        e.register(GET_WINDOW_SETTING, |_, _| 1u32.into_ret());
        e.register(GET_WINDOW_STATE, |_, _| 1u32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 4u32]).i32(), 6);
        let log = take_log(&mut e);
        assert!(calls_to(&log, API_MESSAGE_BOX).is_empty());
        assert_eq!(calls_to(&log, TIMER_ENABLE).len(), 1);
        // With the setting on but the state 0 it is shown after all.
        e.register(GET_WINDOW_STATE, |_, _| 0u32.into_ret());
        e.register(API_MESSAGE_BOX, |_, _| 3u32.into_ret());
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 4u32]).i32(), 3);
    }

    #[test]
    fn message_box_from_another_thread_sets_the_helper_flag_first() {
        let mut e = engine();
        e.set_global(MAIN_OBJECT, 0x5000u32);
        e.register(API_GET_CURRENT_THREAD_ID, |_, _| 10u32.into_ret());
        e.register(GET_MAIN_THREAD_ID, |_, _| 20u32.into_ret());
        e.register(MAIN_OBJECT_GET_HELPER, |_, _| 0x6000u32.into_ret());
        e.register(HELPER_HAS_FLAG, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0086_a580, &args![1u32, 2u32, 0u32]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, GET_MAIN_THREAD_ID), vec![vec![0x5000]]);
        assert_eq!(calls_to(&log, HELPER_SET_FLAG), vec![vec![0x6000, 1]]);
        assert!(position(&log, HELPER_SET_FLAG) < position(&log, API_MESSAGE_BOX));

        // A helper that already has the flag, or the main thread: nothing is set.
        e.register(HELPER_HAS_FLAG, |_, _| 1u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0086_a580, &args![1u32, 2u32, 0u32]);
        assert!(calls_to(&take_log(&mut e), HELPER_SET_FLAG).is_empty());
        e.register(GET_MAIN_THREAD_ID, |_, _| 10u32.into_ret());
        e.register(HELPER_HAS_FLAG, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0086_a580, &args![1u32, 2u32, 0u32]);
        assert!(calls_to(&take_log(&mut e), HELPER_SET_FLAG).is_empty());
    }

    /// An engine where the warning menu is available and the main object has a helper.
    fn warning_menu_engine() -> Engine {
        let mut e = engine();
        e.set_global(WARNING_MENU_AVAILABLE, 1u8);
        e.set_global(MAIN_OBJECT, 0x5000u32);
        e.set_global(WARNING_MENU_CONTEXT, 0xabcdu32);
        e.register(MAIN_OBJECT_GET_HELPER, |_, _| 0x6000u32.into_ret());
        e.register(API_GET_CURRENT_THREAD_ID, |_, _| 10u32.into_ret());
        e.register(GET_MAIN_THREAD_ID, |_, _| 10u32.into_ret());
        // Pooled texts are numbered after the pool object they come from.
        e.register(GET_POOLED_TEXT, |_, a| (a[0] + 1).into_ret());
        e
    }

    #[test]
    fn message_box_opens_the_warning_menu_with_button_texts_for_style_3() {
        let mut e = warning_menu_engine();
        e.register(SHOW_MESSAGE_MENU, |e, _| {
            // While the menu is open it is marked unavailable.
            assert_eq!(e.global::<u8>(WARNING_MENU_AVAILABLE), 0);
            1u32.into_ret()
        });
        e.call_log = Some(vec![]);
        let result = e
            .call(0x0086_a580, &args![0x111u32, 0x222u32, 0x33u32])
            .i32();
        assert_eq!(result, 6);
        let log = take_log(&mut e);
        let menu = calls_to(&log, SHOW_MESSAGE_MENU);
        assert_eq!(menu.len(), 1);
        assert_eq!(
            menu[0],
            vec![
                0x111,
                0,
                0,
                0x0086_a530,
                0,
                0xabcd,
                0,
                0,
                BUTTON_TEXT_SECOND + 1,
                BUTTON_TEXT_FIRST + 1,
                BUTTON_TEXT_THIRD + 1,
                0
            ]
        );
        assert!(calls_to(&log, API_MESSAGE_BOX).is_empty());
        assert_eq!(e.global::<u8>(WARNING_MENU_AVAILABLE), 1);
        assert!(position(&log, TIMER_ENABLE) > position(&log, SHOW_MESSAGE_MENU));
    }

    #[test]
    fn message_box_style_4_fetches_two_button_texts() {
        let mut e = warning_menu_engine();
        e.register(SHOW_MESSAGE_MENU, |_, _| 1u32.into_ret());
        e.call_log = Some(vec![]);
        e.call(0x0086_a580, &args![0x111u32, 0x222u32, 0x4u32]);
        let log = take_log(&mut e);
        let texts: Vec<u32> = calls_to(&log, GET_POOLED_TEXT)
            .iter()
            .map(|a| a[0])
            .collect();
        assert_eq!(texts, vec![BUTTON_TEXT_SECOND, BUTTON_TEXT_FIRST]);
        let menu = &calls_to(&log, SHOW_MESSAGE_MENU)[0];
        assert_eq!(
            &menu[8..],
            &[BUTTON_TEXT_SECOND + 1, BUTTON_TEXT_FIRST + 1, 0, 0]
        );
    }

    #[test]
    fn message_box_styles_0_and_2_skip_the_menu() {
        for style in [0u32, 2] {
            let mut e = warning_menu_engine();
            e.register(API_MESSAGE_BOX, |_, _| 9u32.into_ret());
            e.call_log = Some(vec![]);
            let result = e.call(0x0086_a580, &args![0x111u32, 0x222u32, style]).i32();
            assert_eq!(result, 9);
            let log = take_log(&mut e);
            assert!(calls_to(&log, SHOW_MESSAGE_MENU).is_empty());
            assert!(calls_to(&log, GET_POOLED_TEXT).is_empty());
            assert_eq!(e.global::<u8>(WARNING_MENU_AVAILABLE), 1);
        }
    }

    #[test]
    fn message_box_falls_back_when_the_menu_does_not_open() {
        let mut e = warning_menu_engine();
        e.register(SHOW_MESSAGE_MENU, |_, _| 0u32.into_ret());
        e.register(API_MESSAGE_BOX, |_, _| 4u32.into_ret());
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 4u32]).i32(), 4);
        // Suppressed: styles 3 and 4 answer 6, and so does any other style.
        e.register(GET_WINDOW_SETTING, |_, _| 1u32.into_ret());
        e.register(GET_WINDOW_STATE, |_, _| 1u32.into_ret());
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 4u32]).i32(), 6);
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 3u32]).i32(), 6);
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 1u32]).i32(), 6);
        // Style 2 never opens the menu but is answered 5 when suppressed.
        assert_eq!(e.call(0x0086_a580, &args![1u32, 2u32, 2u32]).i32(), 5);
        assert_eq!(e.global::<u8>(WARNING_MENU_AVAILABLE), 1);
    }

    #[test]
    fn warning_menu_context_getter_and_the_method_that_ignores_its_arguments() {
        let mut e = engine();
        e.set_global(WARNING_MENU_CONTEXT, 0x1234_5678u32);
        assert_eq!(e.call(0x0086_a820, &args![]).u32(), 0x1234_5678);
        assert_eq!(e.call(0x0086_a830, &args![0x4000u32, 99u32]).u8(), 0);
    }

    // ----- the renderer's frame guards and arrays -----------------------------------------------

    fn renderer_with(e: &mut Engine, state: u32, accept: u32) -> u32 {
        // Slots 0x17c / 0x180 answer `accept`.
        let begin = 0x0000_2000u32;
        let end = 0x0000_2004u32;
        e.register_double(begin, move |_, _| accept.into_ret());
        e.register_double(end, move |_, _| accept.into_ret());
        let renderer = object_with_vtable(e, &[(0x17c, begin), (0x180, end)]);
        e.mem.set_u32(renderer + RENDERER_FRAME_STATE, state);
        renderer
    }

    #[test]
    fn frame_state_check_compares_the_state_word() {
        let mut e = engine();
        let renderer = e.mem.alloc(0x300);
        e.mem.set_u32(renderer + 0x200, 2);
        assert!(e
            .call(0x0086_ba60, &args![renderer, 0x1111u32, 2u32])
            .bool());
        assert!(!e
            .call(0x0086_ba60, &args![renderer, 0x1111u32, 0u32])
            .bool());
    }

    #[test]
    fn begin_off_screen_frame_needs_state_0_and_an_accepting_method() {
        let mut e = engine();
        let renderer = renderer_with(&mut e, 0, 1);
        assert!(e.call(0x0086_ba10, &args![renderer]).bool());
        assert_eq!(e.mem.u32(renderer + 0x200), 2);

        let renderer = renderer_with(&mut e, 1, 1);
        assert!(!e.call(0x0086_ba10, &args![renderer]).bool());
        assert_eq!(e.mem.u32(renderer + 0x200), 1);

        let renderer = renderer_with(&mut e, 0, 0);
        assert!(!e.call(0x0086_ba10, &args![renderer]).bool());
        assert_eq!(e.mem.u32(renderer + 0x200), 0);
    }

    #[test]
    fn end_off_screen_frame_needs_state_2_and_an_accepting_method() {
        let mut e = engine();
        let renderer = renderer_with(&mut e, 2, 1);
        assert!(e.call(0x0086_ba90, &args![renderer]).bool());
        assert_eq!(e.mem.u32(renderer + 0x200), 0);

        let renderer = renderer_with(&mut e, 0, 1);
        assert!(!e.call(0x0086_ba90, &args![renderer]).bool());

        let renderer = renderer_with(&mut e, 2, 0);
        assert!(!e.call(0x0086_ba90, &args![renderer]).bool());
        assert_eq!(e.mem.u32(renderer + 0x200), 2);
    }

    /// Doubles for the array count and element methods over keys.
    fn arrays_with_keys(e: &mut Engine, keys: Vec<u32>) {
        let keys = Rc::new(RefCell::new(keys));
        let for_count = keys.clone();
        e.register_double(ARRAY_COUNT, move |_, _| {
            (for_count.borrow().len() as u32).into_ret()
        });
        e.register_double(ARRAY_ELEMENT, move |e, a| {
            let slot = e.mem.alloc(4);
            e.mem.set_u32(slot, keys.borrow()[a[1] as usize]);
            slot.into_ret()
        });
    }

    #[test]
    fn finding_a_key_returns_its_index_or_minus_one() {
        let mut e = engine();
        arrays_with_keys(&mut e, vec![5, 9, 12]);
        let renderer = Ptr::<()>::new(0x7000);
        assert_eq!(e.call(0x0086_bb70, &args![renderer, 9u32]).u32(), 1);
        assert_eq!(e.call(0x0086_bb70, &args![renderer, 12u32]).u32(), 2);
        assert_eq!(e.call(0x0086_bb70, &args![renderer, 3u32]).u32(), u32::MAX);
        assert_eq!(e.call(0x0086_bc60, &args![renderer, 5u32]).u32(), 0);
        e.call_log = Some(vec![]);
        e.call(0x0086_bb70, &args![renderer, 5u32]);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, ARRAY_COUNT),
            vec![vec![0x7000 + KEY_ARRAY_A]]
        );
        e.call_log = Some(vec![]);
        e.call(0x0086_bc60, &args![renderer, 5u32]);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, ARRAY_COUNT),
            vec![vec![0x7000 + KEY_ARRAY_B]]
        );
    }

    #[test]
    fn removing_a_key_removes_it_from_both_arrays() {
        let mut e = engine();
        arrays_with_keys(&mut e, vec![5, 9, 12]);
        let renderer = Ptr::<()>::new(0x7000);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0086_bb20, &args![renderer, 12u32]).bool());
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, ARRAY_REMOVE_AT),
            vec![
                vec![0x7000 + KEY_ARRAY_A, 2],
                vec![0x7000 + DATA_ARRAY_A, 2]
            ]
        );
        e.call_log = Some(vec![]);
        assert!(e.call(0x0086_bc10, &args![renderer, 5u32]).bool());
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, ARRAY_REMOVE_AT),
            vec![
                vec![0x7000 + KEY_ARRAY_B, 0],
                vec![0x7000 + DATA_ARRAY_B, 0]
            ]
        );
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0086_bb20, &args![renderer, 77u32]).bool());
        assert!(!e.call(0x0086_bc10, &args![renderer, 77u32]).bool());
        assert!(calls_to(&take_log(&mut e), ARRAY_REMOVE_AT).is_empty());
    }

    #[test]
    fn registering_a_pair_adds_the_key_then_sets_the_data_at_its_index() {
        for (entry, key_array, data_array) in [
            (0x0086_bae0u32, KEY_ARRAY_A, DATA_ARRAY_A),
            (0x0086_bbd0, KEY_ARRAY_B, DATA_ARRAY_B),
        ] {
            let mut e = engine();
            let seen = Rc::new(RefCell::new(vec![]));
            let record = seen.clone();
            e.register_double(ARRAY_ADD, move |e, a| {
                record.borrow_mut().push(("add", a[0], e.mem.u32(a[1])));
                4u32.into_ret()
            });
            let record = seen.clone();
            e.register_double(ARRAY_SET_AT_GROW, move |e, a| {
                record
                    .borrow_mut()
                    .push(("set", a[0], a[1] * 1000 + e.mem.u32(a[2])));
                Ret::default()
            });
            let index = e
                .call(entry, &args![Ptr::<()>::new(0x7000), 0xaaaau32, 0xbbbbu32])
                .u32();
            assert_eq!(index, 4);
            assert_eq!(
                *seen.borrow(),
                vec![
                    ("add", 0x7000 + key_array, 0xaaaa),
                    ("set", 0x7000 + data_array, 4 * 1000 + 0xbbbb)
                ]
            );
        }
    }

    // ----- globals and small wrappers -------------------------------------------------------------

    #[test]
    fn global_setters_store_their_argument() {
        let mut e = engine();
        e.call(0x0086_bcc0, &args![0x5u32]);
        assert_eq!(e.global::<u8>(0x011f_4448), 5);
        e.call(0x0086_bcd0, &args![0x1234_5678u32]);
        assert_eq!(e.global::<u32>(0x011f_6e08), 0x1234_5678);
        e.call(0x0086_bd90, &args![0x1111u32]);
        assert_eq!(e.global::<u32>(0x011f_6224), 0x1111);
        e.call(0x0086_bde0, &args![0x2222u32]);
        assert_eq!(e.global::<u32>(0x011f_9134), 0x2222);
        e.call(0x0086_be80, &args![0x3333u32]);
        assert_eq!(e.global::<u32>(0x011d_5c44), 0x3333);
        e.call(0x0086_be90, &args![0x4444u32]);
        assert_eq!(e.global::<u32>(0x011d_59ec), 0x4444);
        e.call(0x0086_bea0, &args![0x7u32]);
        assert_eq!(e.global::<u8>(0x011a_31f4), 7);
        e.call(0x0086_bff0, &args![0x5555u32]);
        assert_eq!(e.global::<u32>(0x011f_91f0), 0x5555);
        e.call(0x0086_c000, &args![0x9u32]);
        assert_eq!(e.global::<u8>(0x011f_94a7), 9);
    }

    #[test]
    fn the_default_for_011f6224_is_a_function_address() {
        let mut e = engine();
        e.call(0x0086_bd70, &args![]);
        assert_eq!(e.global::<u32>(0x011f_6224), 0x00b0_13f0);
    }

    #[test]
    fn call_on_the_object_at_011f81dc_only_when_there_is_one() {
        let mut e = engine();
        e.call_log = Some(vec![]);
        e.call(0x0086_bce0, &args![0x77u32]);
        assert!(calls_to(&take_log(&mut e), 0x00af_dda0).is_empty());
        e.set_global(OBJECT_AT_011F81DC, 0x8000u32);
        e.call_log = Some(vec![]);
        e.call(0x0086_bce0, &args![0x77u32]);
        assert_eq!(
            calls_to(&take_log(&mut e), 0x00af_dda0),
            vec![vec![0x8000, 0x77]]
        );
    }

    #[test]
    fn destroying_the_object_at_011f81dc_runs_its_destructor_and_frees_it() {
        let mut e = engine();
        e.call_log = Some(vec![]);
        e.call(0x0086_bd00, &args![]);
        let log = take_log(&mut e);
        assert_eq!(log.len(), 1, "only the top-level call: {log:?}");

        e.set_global(OBJECT_AT_011F81DC, 0x8000u32);
        e.call_log = Some(vec![]);
        e.call(0x0086_bd00, &args![]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00af_dcf0), vec![vec![0x8000]]);
        assert_eq!(calls_to(&log, FREE_OBJECT), vec![vec![0x8000]]);
        // The global is left as it was.
        assert_eq!(e.global::<u32>(OBJECT_AT_011F81DC), 0x8000);
    }

    #[test]
    fn deleting_destructors_free_only_when_bit_0_of_the_flags_is_set() {
        for (entry, body) in [
            (0x0086_bd40u32, 0x00af_dcf0u32),
            (0x0086_bee0, 0x0045_0770),
            (0x0086_bf10, 0x0070_0650),
            (0x0086_bf40, 0x0086_c880),
        ] {
            let mut e = engine();
            e.call_log = Some(vec![]);
            let this = Ptr::<()>::new(0x9000);
            assert_eq!(e.call(entry, &args![this, 1u32]).u32(), 0x9000);
            let log = take_log(&mut e);
            assert_eq!(calls_to(&log, body), vec![vec![0x9000]]);
            assert_eq!(calls_to(&log, FREE_OBJECT), vec![vec![0x9000]]);

            e.call_log = Some(vec![]);
            assert_eq!(e.call(entry, &args![this, 0u32]).u32(), 0x9000);
            e.call(entry, &args![this, 2u32]);
            let log = take_log(&mut e);
            assert_eq!(calls_to(&log, body).len(), 2);
            assert!(calls_to(&log, FREE_OBJECT).is_empty());
        }
    }

    #[test]
    fn state_5c_moves_from_1_or_2_to_3_only() {
        let mut e = engine();
        let object = e.mem.alloc(0x80);
        for (before, after) in [(0u32, 0u32), (1, 3), (2, 3), (3, 3), (4, 4)] {
            e.mem.set_u32(object + 0x5c, before);
            e.call(0x0086_bda0, &args![Ptr::<()>::new(object)]);
            assert_eq!(e.mem.u32(object + 0x5c), after);
        }
    }

    #[test]
    fn call_on_the_object_at_011dd948() {
        let mut e = engine();
        e.call_log = Some(vec![]);
        e.call(0x0086_bdd0, &args![]);
        assert_eq!(
            calls_to(&take_log(&mut e), 0x0043_8af0),
            vec![vec![0x011d_d948]]
        );
    }

    #[test]
    fn query_on_the_object_at_011da0c4() {
        let mut e = engine();
        assert_eq!(e.call(0x0086_bdf0, &args![]).u8(), 0);
        e.set_global(0x011d_a0c4u32, 0x8000u32);
        e.register(0x007c_fd00, |_, a| {
            assert_eq!(a[0], 0x8000);
            0u32.into_ret()
        });
        assert_eq!(e.call(0x0086_bdf0, &args![]).u8(), 0);
        e.register(0x007c_fd00, |_, _| 1u32.into_ret());
        assert_eq!(e.call(0x0086_bdf0, &args![]).u8(), 1);
    }

    #[test]
    fn deleting_the_object_at_011ccb78_through_its_first_virtual_method() {
        let mut e = engine();
        // Nothing there: the global stays 0.
        e.call(0x0086_be30, &args![0u32]);
        assert_eq!(e.global::<u32>(OBJECT_AT_011CCB78), 0);

        let seen = Rc::new(RefCell::new(vec![]));
        let record = seen.clone();
        e.register_double(0x0000_3000, move |_, a| {
            record.borrow_mut().push((a[0], a[1]));
            Ret::default()
        });
        let object = object_with_vtable(&mut e, &[(0, 0x0000_3000)]);
        e.set_global(OBJECT_AT_011CCB78, object);
        e.call(0x0086_be30, &args![0x1234u32]);
        assert_eq!(*seen.borrow(), vec![(object, 1)]);
        assert_eq!(e.global::<u32>(OBJECT_AT_011CCB78), 0);
    }

    #[test]
    fn three_objects_get_zero_in_a_row() {
        let mut e = engine();
        e.call_log = Some(vec![]);
        e.call(0x0086_beb0, &args![]);
        assert_eq!(
            calls_to(&take_log(&mut e), 0x0066_b0d0),
            vec![
                vec![0x011d_86bc, 0],
                vec![0x011d_86a8, 0],
                vec![0x011d_8690, 0]
            ]
        );
    }

    #[test]
    fn shader_init_copies_four_settings_and_installs_the_timer_callback() {
        let mut e = engine();
        let first = e.mem.alloc(8);
        let second = e.mem.alloc(8);
        e.mem.set_u8(first, 0x11);
        e.mem.set_u8(second, 0x22);
        e.register_double(SETTING_BYTE_PTR, move |_, a| match a[0] {
            0x011d_ed68 => first.into_ret(),
            0x011d_eea8 => second.into_ret(),
            other => panic!("unexpected setting {other:08x}"),
        });
        e.register(0x0045_0410, |_, a| match a[0] {
            0x011d_ec18 => ret_f32(1.5),
            0x011d_eb70 => ret_f32(2.5),
            other => panic!("unexpected setting {other:08x}"),
        });
        e.call_log = Some(vec![]);
        e.call(0x0086_bf70, &args![]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, ERROR_LOG), vec![vec![0x0108_2af4]]);
        assert_eq!(calls_to(&log, 0x005c_5a60), vec![vec![0]]);
        assert_eq!(e.global::<u32>(0x011f_91f0), 0x0086_a480);
        assert_eq!(e.global::<u8>(0x011f_94a7), 0x11);
        assert_eq!(e.global::<u8>(0x011f_9443), 0x22);
        assert_eq!(e.global::<f32>(0x011f_9484), 1.5);
        assert_eq!(e.global::<f32>(0x011f_9480), 2.5);
    }

    #[test]
    fn float_setting_getters_forward_the_result() {
        let mut e = engine();
        e.register(0x0045_0410, |_, a| match a[0] {
            0x011c_72a0 => ret_f32(0.25),
            0x011c_774c => ret_f32(0.75),
            other => panic!("unexpected setting {other:08x}"),
        });
        assert_eq!(e.call(0x0086_c0c0, &args![]).f32(), 0.25);
        assert_eq!(e.call(0x0086_c0d0, &args![]).f32(), 0.75);
    }

    /// Doubles for the float getters, the two float consumers and the entry chain at the end of
    /// `0086c010`.
    fn viewport_engine() -> Engine {
        let mut e = engine();
        e.register(0x0045_0410, |_, a| match a[0] {
            0x011c_72a0 => ret_f32(10.0),
            0x011c_774c => ret_f32(20.0),
            other => panic!("unexpected setting {other:08x}"),
        });
        e.register(0x0086_c0e0, |_, _| ret_f32(30.0));
        e.register(0x0086_c0f0, |_, _| ret_f32(40.0));
        e.register(0x0086_c100, |_, _| ret_f32(50.0));
        e.register(0x0086_c110, |_, _| ret_f32(60.0));
        e.register(0x0086_c120, |_, _| ret_f32(70.0));
        e.register(0x0086_c130, |_, _| ret_f32(80.0));
        let float_cell = e.mem.alloc(4);
        e.mem.set_f32(float_cell, 0.5);
        e.register_double(0x0040_3e20, move |_, a| {
            assert_eq!(a[0], 0x011c_f8a4);
            float_cell.into_ret()
        });
        e.register(0x004e_3270, |_, _| 0xaa00u32.into_ret());
        e.register(0x004e_bbc0, |_, a| {
            assert_eq!(a, &[0xaa00, 0xc]);
            0xbb00u32.into_ret()
        });
        e
    }

    #[test]
    fn viewport_setup_with_the_first_set_of_getters() {
        let mut e = viewport_engine();
        e.call_log = Some(vec![]);
        e.call(0x0086_c010, &args![]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0086_c140), vec![vec![10.0f32.to_bits()]]);
        assert_eq!(calls_to(&log, 0x0086_c150), vec![vec![20.0f32.to_bits()]]);
        assert_eq!(e.global::<f32>(0x011a_dbcc), 30.0);
        assert_eq!(e.global::<f32>(0x011a_dbd0), 40.0);
        assert_eq!(
            calls_to(&log, 0x005b_9f80),
            vec![vec![0xbb00, 0.5f32.to_bits()]]
        );
        assert!(calls_to(&log, 0x0086_c100).is_empty());
    }

    #[test]
    fn viewport_setup_with_the_second_set_of_getters() {
        let mut e = viewport_engine();
        e.set_global(0x011f_941eu32, 1u8);
        e.call_log = Some(vec![]);
        e.call(0x0086_c010, &args![]);
        let log = take_log(&mut e);
        assert_eq!(e.global::<f32>(0x011a_dbcc), 70.0);
        assert_eq!(e.global::<f32>(0x011a_dbd0), 80.0);
        assert_eq!(calls_to(&log, 0x0086_c140), vec![vec![50.0f32.to_bits()]]);
        assert_eq!(calls_to(&log, 0x0086_c150), vec![vec![60.0f32.to_bits()]]);
        assert_eq!(calls_to(&log, 0x005b_9f80).len(), 1);
        assert!(calls_to(&log, 0x0086_c0e0).is_empty());
    }

    // ----- main -------------------------------------------------------------------------------

    const INSTANCE: u32 = 0x0040_0000;
    const MAIN_ENTRY: u32 = 0x0086_a850;
    const WINDOW_CLASS_NAME: u32 = 0x0108_3000;

    fn run_main_entry(e: &mut Engine) -> u32 {
        e.call(MAIN_ENTRY, &args![INSTANCE, 0u32, 0u32, 0u32]).u32()
    }

    /// An engine where `main` can run to the end of its loop once: allocation works, the
    /// objects `main` calls through vtables exist, and the first loop test ends the loop.
    fn main_engine() -> Engine {
        let mut e = engine();
        e.set_global(0x011a_2fe8u32, WINDOW_CLASS_NAME);
        e.register(ALLOCATE_OBJECT, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(FREE_OBJECT, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(0x0086_c160, |_, a| a[0].into_ret());
        e.register(0x0068_15c0, |_, _| ZERO_WORD.into_ret());
        e.register(0x0040_3e20, |_, _| ZERO_WORD.into_ret());
        e.register(0x0070_05d0, |_, a| a[0].into_ret());
        let device = object_with_vtable(&mut e, &[(0xc, NOTHING)]);
        e.register_double(0x004d_c020, move |_, _| device.into_ret());
        let with_destructor = object_with_vtable(&mut e, &[(8, NOTHING)]);
        for address in [0x0045_3a70u32, 0x00b0_0a00, 0x00af_f100] {
            e.register_double(address, move |_, _| with_destructor.into_ret());
        }
        // The loop ends after its first pass.
        e.register(0x005d_c960, |_, _| 1u32.into_ret());
        e
    }

    #[test]
    fn main_stops_when_the_mutex_already_exists() {
        let mut e = main_engine();
        e.register(API_GET_LAST_ERROR, |_, _| 0xb7u32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(run_main_entry(&mut e), 0);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, API_CREATE_MUTEX),
            vec![vec![0, 0, 0x0108_2ab8]]
        );
        assert_eq!(
            calls_to(&log, API_MESSAGE_BOX),
            vec![vec![0, 0x0108_2a7c, 0x0104_df38, 0]]
        );
        assert!(calls_to(&log, API_GET_SYSTEM_METRICS).is_empty());
    }

    #[test]
    fn main_stops_in_a_remote_desktop_session() {
        let mut e = main_engine();
        e.register(API_GET_SYSTEM_METRICS, |_, a| {
            assert_eq!(a[0], 0x1000);
            1u32.into_ret()
        });
        e.call_log = Some(vec![]);
        assert_eq!(run_main_entry(&mut e), 0);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, API_MESSAGE_BOX),
            vec![vec![0, 0x0108_2a48, 0x0104_df38, 0]]
        );
        assert!(calls_to(&log, 0x00c3_bd30).is_empty());
    }

    #[test]
    fn main_activates_a_running_instance_and_stops() {
        let mut e = main_engine();
        e.register(API_FIND_WINDOW, |_, _| 0x1234u32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(run_main_entry(&mut e), 0);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, API_FIND_WINDOW),
            vec![vec![WINDOW_CLASS_NAME, 0]]
        );
        assert_eq!(
            calls_to(&log, API_SET_FOREGROUND_WINDOW),
            vec![vec![0x1234]]
        );
        assert!(calls_to(&log, API_CREATE_WINDOW).is_empty());
        // The log files were set up before looking for the window: settings read in order, the
        // error handler last.
        assert_eq!(
            calls_to(&log, 0x00c3_bd30),
            vec![vec![1, 0, 0, 0, 0x0086_a580]]
        );
    }

    #[test]
    fn main_stores_the_processor_count_and_raises_one_to_two() {
        let mut e = main_engine();
        e.register(API_GET_SYSTEM_INFO, |e, a| {
            // dwNumberOfProcessors is at +0x14 of SYSTEM_INFO.
            e.mem.set_u32(a[0] + 0x14, 1);
            Ret::default()
        });
        e.register(API_FIND_WINDOW, |_, _| 0x1234u32.into_ret());
        run_main_entry(&mut e);
        assert_eq!(e.mem.u32(ZERO_WORD), 2);
    }

    #[test]
    fn main_hands_over_to_the_launcher_when_it_starts() {
        let mut e = main_engine();
        e.register(0x004d_a570, |_, _| 1u32.into_ret());
        e.register(API_SHELL_EXECUTE, |_, _| 33u32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(run_main_entry(&mut e), 0);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x004d_a4d0), vec![vec![1]]);
        let launcher = &calls_to(&log, API_SHELL_EXECUTE)[0];
        assert_eq!(&launcher[..2], &[0, 0]);
        assert_eq!(&launcher[3..], &[0, 0, 1]);
        assert_eq!(calls_to(&log, 0x0040_6d50).last().unwrap()[2], 0x0108_2978);
        assert_eq!(calls_to(&log, 0x0040_4ee0).len(), 1);
        assert!(calls_to(&log, API_CREATE_WINDOW).is_empty());
    }

    #[test]
    fn main_goes_on_when_the_launcher_does_not_start() {
        let mut e = main_engine();
        e.register(0x004d_a570, |_, _| 1u32.into_ret());
        e.register(API_SHELL_EXECUTE, |_, _| 0u32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(run_main_entry(&mut e), 0);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, API_CREATE_WINDOW).len(), 1);
    }

    /// A UTF-16 string in memory.
    fn wide(e: &mut Engine, text: &str) -> u32 {
        let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let address = e.mem.alloc(units.len() as u32 * 2);
        for (i, unit) in units.iter().enumerate() {
            e.mem.set_u16(address + 2 * i as u32, *unit);
        }
        address
    }

    #[test]
    fn main_turns_cloud_off_for_the_nocloud_argument_only() {
        let mut e = main_engine();
        let arguments = [
            wide(&mut e, "FalloutNV.exe"),
            wide(&mut e, "-NoCloud"),
            wide(&mut e, "nocloud"),
            wide(&mut e, "-other"),
        ];
        let argv = e.mem.alloc(16);
        for (i, argument) in arguments.iter().enumerate() {
            e.mem.set_u32(argv + 4 * i as u32, *argument);
        }
        e.register(API_GET_COMMAND_LINE, |_, _| 0xc0de_0000u32.into_ret());
        e.register_double(API_COMMAND_LINE_TO_ARGV, move |e, a| {
            assert_eq!(a[0], 0xc0de_0000);
            e.mem.set_i32(a[1], 4);
            argv.into_ret()
        });
        for (i, unit) in "nocloud".encode_utf16().enumerate() {
            e.mem.set_u16(0x0108_29b4 + 2 * i as u32, unit);
        }
        // `__wcsicmp`: compares case-insensitively.
        e.register(0x00ec_c071, |e, a| {
            let read = |e: &Engine, mut p: u32| {
                let mut text = String::new();
                loop {
                    let unit = e.mem.u16(p);
                    if unit == 0 {
                        return text;
                    }
                    text.push(char::from_u32(unit as u32).unwrap());
                    p += 2;
                }
            };
            let left = read(e, a[0]).to_lowercase();
            let right = read(e, a[1]);
            assert_eq!(right, "nocloud");
            (left != right).into_ret()
        });
        e.register(0x006f_f580, |_, _| 0x7700u32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(run_main_entry(&mut e), 0);
        let log = take_log(&mut e);
        // One match ("-NoCloud"): the second setting object is the one `006ff580` returned.
        assert_eq!(calls_to(&log, 0x006f_f580).len(), 1);
        assert_eq!(calls_to(&log, 0x006f_f880), vec![vec![0x7700, 0]]);
        assert_eq!(calls_to(&log, API_LOCAL_FREE), vec![vec![argv]]);
        // Compared: "-NoCloud" and "-other" start with '-', the text after it is compared.
        assert_eq!(calls_to(&log, 0x00ec_c071).len(), 2);
    }

    #[test]
    fn main_registers_the_window_class_and_creates_the_window_and_main_object() {
        let mut e = main_engine();
        e.register(API_LOAD_ICON, |_, a| {
            assert_eq!(a, &[INSTANCE, 0x65]);
            0x77u32.into_ret()
        });
        e.register(API_GET_STOCK_OBJECT, |_, a| {
            assert_eq!(a[0], 4);
            0x88u32.into_ret()
        });
        let class_words = Rc::new(RefCell::new(vec![]));
        let record = class_words.clone();
        e.register_double(API_REGISTER_CLASS, move |e, a| {
            for i in 0..10 {
                record.borrow_mut().push(e.mem.u32(a[0] + 4 * i));
            }
            Ret::default()
        });
        // AdjustWindowRect grows the rectangle by (4, 20) in total.
        e.register(API_ADJUST_WINDOW_RECT, |e, a| {
            assert_eq!(&a[1..], &[0x1000_0000, 0]);
            e.mem.set_u32(a[0] + 8, e.mem.u32(a[0] + 8) + 4);
            e.mem.set_u32(a[0] + 12, e.mem.u32(a[0] + 12) + 20);
            Ret::default()
        });
        e.register(API_CREATE_WINDOW, |_, _| 0x4321u32.into_ret());
        e.call_log = Some(vec![]);
        assert_eq!(run_main_entry(&mut e), 0);
        let log = take_log(&mut e);
        assert_eq!(
            *class_words.borrow(),
            vec![
                3,
                0x0086_a0a0,
                0,
                0,
                INSTANCE,
                0x77,
                0,
                0x88,
                0,
                WINDOW_CLASS_NAME
            ]
        );
        assert_eq!(
            calls_to(&log, API_CREATE_WINDOW),
            vec![vec![
                0,
                WINDOW_CLASS_NAME,
                WINDOW_CLASS_NAME,
                0x1000_0000,
                0,
                0,
                0x144,
                0x104,
                0,
                0,
                INSTANCE,
                0
            ]]
        );
        // `Main` is allocated with 0xA4 bytes and built with the window and the instance.
        let main_allocation = calls_to(&log, ALLOCATE_OBJECT)
            .iter()
            .position(|a| a[0] == 0xa4)
            .unwrap();
        assert_eq!(main_allocation, 1);
        let constructor = &calls_to(&log, 0x0086_c160)[0];
        assert_eq!(&constructor[1..], &[0x4321, INSTANCE]);
        // After the run the global is cleared again.
        assert_eq!(e.global::<u32>(MAIN_OBJECT), 0);
    }

    #[test]
    fn main_runs_one_frame_and_shuts_down_in_order() {
        let mut e = main_engine();
        e.register(API_CREATE_WINDOW, |_, _| 0x4321u32.into_ret());
        // The start position `Main_InitStartCell` writes is passed on by value.
        e.register(0x0087_6dd0, |e, a| {
            assert_eq!(a[0], 0);
            e.mem.set_u32(a[1], 11);
            e.mem.set_u32(a[1] + 4, 22);
            e.mem.set_u32(a[1] + 8, 33);
            Ret::default()
        });
        e.set_global(0x0126_fac4u32, 0x5150u32);
        e.register(GET_POOLED_TEXT, |_, a| (a[0] + 1).into_ret());
        e.register(0x0066_29f0, |_, a| (a[0] + 5).into_ret());
        e.register(0x0045_c670, |_, _| 0x3000u32.into_ret());
        e.set_global(0x011f_4509u32, 0u8);
        e.call_log = Some(vec![]);
        assert_eq!(run_main_entry(&mut e), 0);
        let log = take_log(&mut e);

        assert_eq!(calls_to(&log, 0x0087_7260), vec![vec![11, 22, 33]]);
        assert_eq!(
            calls_to(&log, 0x00ec_16d0),
            vec![vec![0x5150, 0x011d_e72c + 1, 0, u32::MAX, 1, 1, 0]]
        );
        assert_eq!(calls_to(&log, 0x0086_cf20)[0][1], 0x3000);
        // The animation data of the object goes to two globals.
        assert_eq!(e.global::<u32>(0x011d_59ec), 0x3005);
        assert_eq!(e.global::<u32>(0x011d_5c44), 0x3005);
        assert_eq!(e.global::<u8>(0x011f_4509), 1);
        // Large buffer is set for the master file when `008d6f30` says 0.
        assert_eq!(calls_to(&log, 0x0046_50a0), vec![vec![0, u32::MAX]]);
        // Where the Main object is used.
        let main_object = calls_to(&log, 0x0086_e650)[0][0];
        assert_ne!(main_object, 0);
        assert_eq!(calls_to(&log, 0x0086_e650).len(), 1);
        assert_eq!(calls_to(&log, 0x005d_c960), vec![vec![main_object]]);
        assert_eq!(calls_to(&log, 0x0086_c880), vec![vec![main_object]]);
        // Order of the main phases.
        let order = [
            API_CREATE_MUTEX,
            API_REGISTER_CLASS,
            0x0086_c160,
            0x0086_d500,
            0x0086_d590,
            0x0087_6dd0,
            0x0087_7260,
            0x00ec_1800,
            API_PEEK_MESSAGE,
            0x0086_e650,
            0x005d_c960,
            0x0070_3610,
            0x0086_c880,
            0x0087_83c0,
            0x0087_84d0,
        ];
        for pair in order.windows(2) {
            assert!(
                position(&log, pair[0]) < position(&log, pair[1]),
                "{:08x} before {:08x}",
                pair[0],
                pair[1]
            );
        }
        // The Main object was freed by its deleting destructor.
        assert!(calls_to(&log, FREE_OBJECT)
            .iter()
            .any(|a| a[0] == main_object));
        // The tasklet manager, the system object and the audio object were asked to shut down
        // through their vtables.
        assert!(!calls_to(&log, 0x00b0_0a00).is_empty());
        assert!(!calls_to(&log, 0x00af_f100).is_empty());
    }

    #[test]
    fn main_posts_pending_window_messages_before_each_frame() {
        let mut e = main_engine();
        let pending = Rc::new(RefCell::new(2u32));
        let counter = pending.clone();
        e.register_double(API_PEEK_MESSAGE, move |_, a| {
            assert_eq!(&a[1..], &[0, 0, 0, 1]);
            let mut left = counter.borrow_mut();
            if *left > 0 {
                *left -= 1;
                1u32.into_ret()
            } else {
                0u32.into_ret()
            }
        });
        e.call_log = Some(vec![]);
        run_main_entry(&mut e);
        let log = take_log(&mut e);
        let translated = calls_to(&log, API_TRANSLATE_MESSAGE);
        assert_eq!(translated.len(), 2);
        assert_eq!(translated, calls_to(&log, API_DISPATCH_MESSAGE));
        assert_eq!(calls_to(&log, API_PEEK_MESSAGE).len(), 3);
    }

    #[test]
    fn main_frame_with_a_lost_device_runs_the_off_screen_guards_and_sleeps() {
        let mut e = main_engine();
        // The device's vtable slot 0xC answers -1.
        let device = object_with_vtable(&mut e, &[(0xc, 0x0000_3100)]);
        e.register_double(0x0000_3100, |_, a| {
            assert_eq!(a[0], a[1], "the device is also passed as a stack argument");
            u32::MAX.into_ret()
        });
        e.register_double(0x004d_c020, move |_, _| device.into_ret());
        let renderer = renderer_with(&mut e, 0, 1);
        e.register_double(GET_RENDERER, move |_, _| renderer.into_ret());
        e.call_log = Some(vec![]);
        run_main_entry(&mut e);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0086_e650).is_empty());
        assert!(calls_to(&log, API_SLEEP).contains(&vec![0x32]));
        // The begin guard moved the state to 2 and the end guard back to 0.
        assert_eq!(e.mem.u32(renderer + RENDERER_FRAME_STATE), 0);
    }

    /// Makes the window inactive on the first pass of the loop and active on the second.
    fn background_then_foreground(e: &mut Engine, background_calls: u32) {
        let windows = Rc::new(RefCell::new(0u32));
        let count = windows.clone();
        e.register(0x0044_ddc0, |_, _| 6u32.into_ret());
        e.register_double(API_GET_ACTIVE_WINDOW, move |_, _| {
            let mut n = count.borrow_mut();
            *n += 1;
            if *n <= background_calls {
                5u32.into_ret()
            } else {
                6u32.into_ret()
            }
        });
        let loops = Rc::new(RefCell::new(0u32));
        let count = loops.clone();
        // The loop ends after `background_calls + 1` passes.
        e.register_double(0x005d_c960, move |_, _| {
            let mut n = count.borrow_mut();
            *n += 1;
            (*n > background_calls).into_ret()
        });
    }

    #[test]
    fn main_in_the_background_suspends_audio_and_the_loading_thread_then_resumes() {
        let mut e = main_engine();
        background_then_foreground(&mut e, 1);
        e.register(API_GET_CURRENT_THREAD_ID, |_, _| 7u32.into_ret());
        e.register(GET_MAIN_THREAD_ID, |_, _| 7u32.into_ret());
        let audio = object_with_vtable(&mut e, &[(8, NOTHING)]);
        e.register_double(0x0045_3a70, move |_, _| audio.into_ret());
        e.register(0x004e_2190, |_, a| (a[0] + 1).into_ret());
        e.register(0x0049_fef0, |_, _| 0x2222u32.into_ret());
        e.register(0x0070_5910, |_, _| 0x3333u32.into_ret());
        e.call_log = Some(vec![]);
        run_main_entry(&mut e);
        let log = take_log(&mut e);

        // First pass (background): suspend everything, sleep.
        let suspend = position(&log, 0x0078_cfc0);
        let audio_down = position(&log, 0x00ad_8700);
        let resume_thread = position(&log, 0x0078_d020);
        assert!(suspend < audio_down);
        assert!(audio_down < resume_thread);
        assert_eq!(calls_to(&log, 0x00ad_8700), vec![vec![audio]]);
        assert_eq!(calls_to(&log, 0x00ad_8740), vec![vec![audio]]);
        assert_eq!(calls_to(&log, 0x0083_2ad0).len(), 2);
        assert_eq!(
            calls_to(&log, 0x00ad_7740),
            vec![vec![audio, 1], vec![audio, 1]]
        );
        assert_eq!(calls_to(&log, 0x004a_4340), vec![vec![0x2222]]);
        assert_eq!(calls_to(&log, 0x0080_26b0), vec![vec![0x3333]]);
        assert!(calls_to(&log, API_SLEEP).contains(&vec![0x32]));
        // Second pass (foreground): the frame runs once and the loading thread is resumed.
        assert_eq!(calls_to(&log, 0x0086_e650).len(), 1);
        assert_eq!(calls_to(&log, 0x0078_d020).len(), 1);
    }

    #[test]
    fn main_in_the_background_from_another_thread_leaves_the_loading_thread_alone() {
        let mut e = main_engine();
        background_then_foreground(&mut e, 1);
        e.register(API_GET_CURRENT_THREAD_ID, |_, _| 7u32.into_ret());
        e.register(GET_MAIN_THREAD_ID, |_, _| 8u32.into_ret());
        e.call_log = Some(vec![]);
        run_main_entry(&mut e);
        let log = take_log(&mut e);
        assert!(calls_to(&log, 0x0078_cfc0).is_empty());
        assert!(calls_to(&log, 0x0078_d020).is_empty());
        assert_eq!(calls_to(&log, 0x00ad_8700).len(), 1);
    }

    #[test]
    fn main_in_the_background_skips_the_loading_thread_when_the_object_says_so() {
        let mut e = main_engine();
        background_then_foreground(&mut e, 1);
        e.set_global(0x011d_a0c4u32, 0x8000u32);
        e.register(0x007c_fd00, |_, _| 1u32.into_ret());
        e.register(API_GET_CURRENT_THREAD_ID, |_, _| 7u32.into_ret());
        e.register(GET_MAIN_THREAD_ID, |_, _| 7u32.into_ret());
        e.call_log = Some(vec![]);
        run_main_entry(&mut e);
        assert!(calls_to(&take_log(&mut e), 0x0078_cfc0).is_empty());
    }

    #[test]
    fn main_resizes_the_window_when_it_goes_to_the_background_and_comes_back() {
        let mut e = main_engine();
        background_then_foreground(&mut e, 1);
        e.register(GET_WINDOW_SETTING, |_, _| 1u32.into_ret());
        e.register(0x004d_c1f0, |_, _| 800u32.into_ret());
        e.register(0x004d_c200, |_, _| 600u32.into_ret());
        e.register(API_CREATE_WINDOW, |_, _| 0x4321u32.into_ret());
        e.register(API_GET_WINDOW_LONG, |_, a| {
            assert_eq!(a, &[0x4321, (-0x10i32) as u32]);
            0x1234u32.into_ret()
        });
        e.register(API_ADJUST_WINDOW_RECT_EX, |e, a| {
            assert_eq!(&a[1..], &[0x1234, 0, 0]);
            // The rectangle (0, 0, 800, 600) grows by 8 on each side.
            e.mem.set_u32(a[0], (-8i32) as u32);
            e.mem.set_u32(a[0] + 4, (-8i32) as u32);
            e.mem.set_u32(a[0] + 8, 808);
            e.mem.set_u32(a[0] + 12, 608);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        run_main_entry(&mut e);
        let log = take_log(&mut e);
        // Once going to the background and once coming back.
        let moves = calls_to(&log, API_SET_WINDOW_POS);
        assert_eq!(moves.len(), 2);
        // Left and bottom are the position, (right - left, top - bottom) the size.
        let expected = vec![0x4321, 0, (-8i32) as u32, 608, 816, (-8i32 - 608) as u32, 0];
        assert_eq!(moves[0], expected);
        assert_eq!(moves[1], expected);
    }

    #[test]
    fn main_returns_held_textures_and_deletes_what_it_created() {
        let mut e = main_engine();
        // The first holder (011ded3c) holds a texture; the others do not.
        e.register(0x0055_9450, |_, a| {
            if a[0] == 0x011d_ed3c {
                0x9999u32.into_ret()
            } else {
                0u32.into_ret()
            }
        });
        e.register(0x004a_0ea0, |_, _| 0x4a00u32.into_ret());
        e.call_log = Some(vec![]);
        run_main_entry(&mut e);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x00b6_da10), vec![vec![0x4a00, 0x9999]]);
        assert_eq!(calls_to(&log, 0x0066_b0d0).len(), 4);
        assert_eq!(calls_to(&log, 0x0066_b0d0)[0], vec![0x011d_ed3c, 0]);
        // The renderer's callbacks are looked up again by key (none are registered).
        assert_eq!(
            calls_to(&log, ARRAY_COUNT),
            vec![vec![KEY_ARRAY_A], vec![KEY_ARRAY_B]]
        );
        assert_eq!(e.global::<u8>(0x011a_31f4), 0);
    }
}
