//! `JG/functions/fn_utility.cpp` (JohnnyGuitar NVSE 5.28), the commands built so far: a form's
//! editor id (`GetEditorID`, with the editor-id restoration's names) and the form an editor id
//! names (`EditorIDToFormID`), the game's seconds-passed timer (`GameGetSecondsPassed`), the time
//! played (`GetTimePlayed`), the no-op `NullArgs`, a form's last file
//! (`GetFormOverrideIndex`), the Pip-Boy mode (`GetPipBoyMode`).

use super::{console_print, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (GET_EDITOR_ID, get_editor_id, None),
    (NULL_ARGS, null, None),
    (GET_TIME_PLAYED, get_time_played, Some(get_time_played_eval)),
    (EDITOR_ID_TO_FORM_ID, editor_id_to_form_id, None),
    (GET_PIP_BOY_MODE, get_pip_boy_mode, Some(get_pip_boy_mode_eval)),
    (GET_FORM_OVERRIDE_INDEX, get_form_override_index, None),
    (GAME_GET_SECONDS_PASSED, game_get_seconds_passed, Some(game_get_seconds_passed_eval)),
];

// The opcodes (each pinned to its slot's name by a test).
pub(crate) const GET_EDITOR_ID: u32 = 0x3114;
pub(crate) const NULL_ARGS: u32 = 0x3117;
pub(crate) const GET_TIME_PLAYED: u32 = 0x3121;
pub(crate) const EDITOR_ID_TO_FORM_ID: u32 = 0x3128;
pub(crate) const GET_PIP_BOY_MODE: u32 = 0x3168;
pub(crate) const GET_FORM_OVERRIDE_INDEX: u32 = 0x316d;
pub(crate) const GAME_GET_SECONDS_PASSED: u32 = 0x3202;

/// `TESForm::GetFormByEditorID` (`00483A00`, `cdecl`, the name): JohnnyGuitar's wrap answers it
/// from the editor-id store while its row is on.
const FORM_BY_EDITOR_ID: u32 = 0x0048_3a00;
/// `TESForm::GetFile` (`00484E60`, `thiscall`, an index: -1 the last file that has the form) and
/// `TESFile::ucCompileIndex` (+0x40C).
const FORM_FILE: u32 = 0x0048_4e60;
const FILE_COMPILE_INDEX: u32 = 0x40c;
/// The game's empty text (`01011584`), standing in for JohnnyGuitar's own `""` literal.
const EMPTY_TEXT: u32 = 0x0101_1584;
/// A form's `GetEditorID` (vtable +0x130).
const GET_EDITOR_ID_SLOT: u32 = 0x130;

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_utility.cpp:
// Cmd_GetEditorID_Execute.
/// The form's editor id (the calling reference's without one) as the string result, "" for no
/// form; echoed `GetEditorID >> %s` in the console. A failed read assigns nothing.
fn get_editor_id(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(0);
    if form == 0 {
        form = a.this_obj;
    }
    // The getter's text (game memory: JG's copy or the empty text), or JG's own "" for no form.
    let at = if form != 0 {
        e.vcall(form, GET_EDITOR_ID_SLOT, &args![]).u32()
    } else {
        EMPTY_TEXT
    };
    let text = e.mem.cstr(at);
    nvse::commands::string::assign_to_string_var(e, &a, &text);
    console_print(e, b"GetEditorID >> %s", &[at]);
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_utility.cpp:
// Cmd_EditorIDToFormID_Execute.
/// The id of the form the editor id names (`GetFormByEditorID`), written into the result's low
/// word; 0 for none. Echoed `EditorIDToFormID >> 0x%08X` (the result's double as its argument)
/// for a non-empty name.
fn editor_id_to_form_id(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let name = nvse::commands::arg_in_buffer(e, &read, 0, 260, "EditorIDToFormID");
    if read.ok && !name.is_empty() {
        let size = (name.len() as u32 + 1).next_multiple_of(4);
        let form = e.with_stack(size, |e, at| {
            e.mem.write(at.addr(), &name);
            e.mem.set_u8(at.addr() + name.len() as u32, 0);
            e.call(FORM_BY_EDITOR_ID, &args![at]).u32()
        });
        if form != 0 {
            let id = e.mem.u32(form + 0xc);
            e.mem.set_u32(a.result, id);
        }
        let result = e.mem.f64(a.result);
        console_print(e, b"EditorIDToFormID >> 0x%08X", &args![result]);
    }
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_utility.cpp:
// Cmd_NullArgs_Execute.
/// The result 0.
fn null(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_utility.cpp:
// Cmd_GetFormOverrideIndex_Execute.
/// The load-order index of the form's last file (`GetFile(-1)`'s `ucCompileIndex`), 0xFF for a
/// form from no file; echoed `GetFormOverrideIndex >> %.f`.
fn get_form_override_index(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if read.ok && form != 0 {
        let file = e.call(FORM_FILE, &args![form, u32::MAX]).u32();
        let index = if file != 0 {
            f64::from(e.mem.u8(file + FILE_COMPILE_INDEX))
        } else {
            255.0
        };
        a.set_result(e, index);
        console_print(e, b"GetFormOverrideIndex >> %.f", &args![index]);
    }
    handled(true)
}

// ---- timers and the Pip-Boy ----

/// The game's timer object (`0x11F6394`) and its seconds passed (`007013E0`, `thiscall`: its
/// +0xC over the global at `011AC3A0`).
const GAME_TIMER: u32 = 0x011f_6394;
const SECONDS_PASSED: u32 = 0x0070_13e0;

fn seconds_passed(e: &mut Engine) -> f64 {
    f64::from(e.call(SECONDS_PASSED, &args![GAME_TIMER]).f32())
}

fn game_get_seconds_passed_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let v = seconds_passed(e);
    e.mem.set_f64(a.result, v);
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_utility.cpp:
// Cmd_GameGetSecondsPassed_Execute.
/// The game timer's seconds passed, echoed `GameGetSecondsPassed >> %0.2f`.
fn game_get_seconds_passed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let v = seconds_passed(e);
    a.set_result(e, v);
    console_print(e, b"GameGetSecondsPassed >> %0.2f", &args![v]);
    handled(true)
}

/// `InterfaceManager::GetSingleton` (`*011D8A80`) and its `pipBoyMode` (+0x4BC).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const PIP_BOY_MODE: u32 = 0x4bc;

fn pip_boy_mode(e: &Engine) -> f64 {
    let manager = e.mem.u32(INTERFACE_MANAGER);
    if manager == 0 {
        0.0
    } else {
        f64::from(e.mem.u32(manager + PIP_BOY_MODE))
    }
}

fn get_pip_boy_mode_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let v = pip_boy_mode(e);
    e.mem.set_f64(a.result, v);
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_utility.cpp:
// Cmd_GetPipBoyMode_Execute.
/// The interface manager's Pip-Boy mode (0 without one), echoed `GetPipBoyMode >> %.2f`.
fn get_pip_boy_mode(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let v = pip_boy_mode(e);
    a.set_result(e, v);
    console_print(e, b"GetPipBoyMode >> %.2f", &args![v]);
    handled(true)
}

// ---- the time played ----

/// The game's millisecond clock (`00457FE0`, `cdecl`: its `GetTickCount` import), which
/// JohnnyGuitar reads directly; the clock at the deferred start (`dwGameStartTimestamp`, 0 before).
const TICK_COUNT: u32 = 0x0045_7fe0;

#[derive(Default)]
pub struct StartTicks(pub std::sync::atomic::AtomicU32);

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/internal/JG/JohnnyMessageHandler.cpp:
// DeferredInit (`dwGameStartTimestamp = GetTickCount()`).
pub fn deferred_init(e: &mut Engine) {
    if !e.is_registered(TICK_COUNT) {
        return;
    }
    let now = e.call(TICK_COUNT, &args![]).u32();
    e.ext_free::<StartTicks>()
        .0
        .store(now, std::sync::atomic::Ordering::Relaxed);
}

/// 0: milliseconds since the deferred start; 1: seconds; 2: minutes; other: 0. The clock less the
/// start wraps as a `DWORD`.
fn time_played(e: &mut Engine, kind: u32) -> f64 {
    let now = e.call(TICK_COUNT, &args![]).u32();
    let start = e
        .ext_free::<StartTicks>()
        .0
        .load(std::sync::atomic::Ordering::Relaxed);
    let played = f64::from(now.wrapping_sub(start));
    match kind {
        0 => played,
        1 => played / 1000.0,
        2 => played / 60000.0,
        _ => 0.0,
    }
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_utility.cpp:
// Cmd_GetTimePlayed_Eval.
fn get_time_played_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let v = time_played(e, a.arg1);
    e.mem.set_f64(a.result, v);
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_utility.cpp:
// Cmd_GetTimePlayed_Execute.
/// The eval with the kind read (0 when not read), echoed `GetTimePlayed >> %f`.
fn get_time_played(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let v = time_played(e, read.int(0, 0));
    a.set_result(e, v);
    console_print(e, b"GetTimePlayed >> %f", &args![v]);
    handled(true)
}

#[cfg(test)]
#[path = "fn_utility_tests.rs"]
mod tests;
