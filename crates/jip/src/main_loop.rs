//! JIP LN NVSE's main-loop callbacks (`MainLoopCallback`, `CycleMainLoopCallbacks`,
//! `internal/hooks.h` (JIP LN NVSE)) and `SetGameMainLoopCallback` (`0x27D4`,
//! `functions_jip/jip_fn_script.h`): a script function called every so many frames of the main
//! loop, in game mode, menu mode or both, on behalf of a reference; run from JIP's listener at
//! xNVSE's `MainGameLoop` message ([`crate::cosave::listen`]), which xNVSE sends at the start of
//! each pass of its tick (`HandleMainLoopHook`).
//!
//! JIP also puts its own functions in the same list for some of its commands; those come with
//! the commands. A script's callback here keeps the script's lambda variables while it is in
//! the list (`CaptureLambdaVars`, xNVSE's `LambdaManager`), as JIP does.

use crate::game::TESForm;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

/// `SetGameMainLoopCallback`'s opcode.
pub const SET_GAME_MAIN_LOOP_CALLBACK: u32 = 0x27d4;
/// `g_interfaceManager` and its `currentMode` (`+0xC`: 1 game mode, 2 menu mode; the exe's
/// `cMenuMode`, engine-fallout-interface `interfacemanager.rs`).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const CURRENT_MODE: u32 = 0xc;
/// `g_thePlayer`.
const PLAYER: u32 = 0x011d_ea3c;
/// `kFormType_Script`; the player's id.
const SCRIPT: u8 = 0x11;
const PLAYER_ID: u32 = 0x14;

/// One script callback (`MainLoopCallback` with `isScript`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Callback {
    pub script: u32,
    /// `thisObj`: the reference it runs on.
    pub this_obj: u32,
    /// `bRemove`: taken out at the end of the next cycle.
    pub remove: bool,
    /// `flags`: 1 game mode, 2 menu mode (`modeFlag & 0xB`).
    pub flags: u8,
    /// `callDelay`: the frames between two calls; `cycleCount`: the frames left.
    pub call_delay: u32,
    pub cycle_count: u32,
    /// `args[0]`: the reference's id when added.
    pub ref_id: u32,
}

/// `s_mainLoopCallbacks`, in the order they were added.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub callbacks: Vec<Callback>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().main_loop
}

/// `SetGameMainLoopCallback`'s handler in its record (built whatever JIP answers, as every built
/// command).
pub fn install(e: &mut Engine) {
    crate::table::set_handlers(
        e,
        SET_GAME_MAIN_LOOP_CALLBACK,
        Some(set_game_main_loop_callback),
        None,
    );
}

fn form_id(e: &Engine, form: u32) -> u32 {
    e.get(Ptr::<TESForm>::new(form), TESForm::iFormID)
}

fn form_type(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<TESForm>::new(form), TESForm::cFormType)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: FindMainLoopCallback.
fn find(e: &mut Engine, script: u32, this_obj: u32) -> Option<usize> {
    state(e)
        .callbacks
        .iter()
        .position(|c| c.script == script && c.this_obj == this_obj)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_SetGameMainLoopCallback_Execute.
/// With one argument: 1 when the script has a callback for the calling reference (the player
/// without one). With more: add it (a delay of 0 is 1; one there already is kept and given the
/// new delay, its frames left as they are) with the mode flags `modeFlag & 0xB` (default 3), or
/// mark it to go. The script must be a script form.
fn set_game_main_loop_callback(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let args = e.mem.u8(a.data + e.mem.u32(a.opcode_offset));
    let read = Read::of(e, &a);
    let script = read.form(0);
    if !read.ok || script == 0 || form_type(e, script) != SCRIPT {
        return handled(true);
    }
    let (add, delay, mode) = (read.int(1, 0), read.int(2, 1), read.int(3, 3));
    let calling = if a.this_obj != 0 {
        a.this_obj
    } else {
        e.mem.u32(PLAYER)
    };
    let found = find(e, script, calling);
    if args < 2 {
        if found.is_some() {
            a.set_result(e, 1.0);
        }
    } else if add != 0 {
        let delay = delay.max(1);
        let at = match found {
            Some(i) => {
                let c = &mut state(e).callbacks[i];
                c.remove = false;
                c.call_delay = delay;
                i
            }
            None => {
                nvse::lambda::save_variables(e, script);
                let ref_id = form_id(e, calling);
                let list = &mut state(e).callbacks;
                list.push(Callback {
                    script,
                    this_obj: calling,
                    remove: false,
                    flags: 0,
                    call_delay: delay,
                    cycle_count: delay,
                    ref_id,
                });
                list.len() - 1
            }
        };
        state(e).callbacks[at].flags = (mode & 0xb) as u8;
    } else if let Some(i) = found {
        state(e).callbacks[i].remove = true;
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: CycleMainLoopCallbacks (the
// script callbacks' path).
/// One pass: each callback there at the start and not marked to go, when the current mode (menu
/// mode: `currentMode > 1`) is one of its flags, counts a frame down; at 0 its reference is
/// checked (the player always passes; another must still be the form its id names, or the
/// callback is marked to go) and the script is called on it with no arguments, its frames left
/// set back to its delay. Then the marked ones go, their lambda variables given up.
pub fn cycle(e: &mut Engine) {
    let count = state(e).callbacks.len();
    for i in 0..count {
        let c = state(e).callbacks[i].clone();
        if c.remove {
            continue;
        }
        let manager = e.mem.u32(INTERFACE_MANAGER);
        let mode: u8 = if e.mem.u32(manager + CURRENT_MODE) > 1 {
            2
        } else {
            1
        };
        if mode & c.flags == 0 {
            continue;
        }
        let left = c.cycle_count.wrapping_sub(1);
        state(e).callbacks[i].cycle_count = left;
        if left != 0 {
            continue;
        }
        if c.ref_id != PLAYER_ID && crate::cosave::lookup_form(e, c.ref_id) != c.this_obj {
            state(e).callbacks[i].remove = true;
            continue;
        }
        state(e).callbacks[i].cycle_count = c.call_delay;
        crate::script_call::call(e, c.script, c.this_obj, Vec::new());
    }
    let mut i = state(e).callbacks.len();
    while i > 0 {
        i -= 1;
        if state(e).callbacks[i].remove {
            let gone = state(e).callbacks.remove(i);
            nvse::lambda::unsave_variables(e, gone.script);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h: CleanMLCallbacks (the
// script callbacks' part; JIP's script runner's own come with it).
/// At a load or the exit to the main menu: the callbacks set with mode bit 8 marked to go.
pub fn clean(e: &mut Engine) {
    for c in &mut state(e).callbacks {
        if c.flags & 8 != 0 {
            c.remove = true;
        }
    }
}

#[cfg(test)]
#[path = "main_loop_tests.rs"]
mod tests;
