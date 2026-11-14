//! `functions_ln/ln_fn_terminal.h` (JIP LN NVSE): a terminal's lock level and password note, and
//! whether a terminal, a lock or a door's linked door is locked out. All 5 of its commands.
//!
//! `BGSTerminal` (form type 0x17, the exe's layout in engine-fallout-forms `bgsterminal.rs`,
//! loader `004FFCB0`): the password note at +0xA0, the difficulty at +0xA4, the flags at +0xA5
//! (`kTerminalFlagUnlocked` 2). On references: the form flags (+8, bit 0x100), an
//! `ExtraTerminalState` (type 0x50: locked out +0xC, lock level +0xD), an `ExtraLock` (type 0x2A:
//! its lock data at +0xC, 0x14 bytes, the tries count at +0xC of it), an `ExtraTeleport` (type
//! 0x2B: its data at +0xC, the linked door at +0 of it).

use super::extra::{self, kind, vtbl, REFR_EXTRA_LIST};
use super::{heap_alloc, set_form_result, type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2263, get_terminal_lock),
    (0x224f, get_password_note),
    (0x2250, set_password_note),
    (0x224b, get_locked_out),
    (0x224c, set_locked_out),
];

/// `kFormType_BGSTerminal`, `kFormType_BGSNote`, `kFormType_TESObjectDOOR`.
const BGS_TERMINAL: u8 = 0x17;
const BGS_NOTE: u8 = 0x31;
const TES_OBJECT_DOOR: u8 = 0x1c;
const PASSWORD: u32 = 0xa0;
const DIFFICULTY: u32 = 0xa4;
const TERMINAL_FLAGS: u32 = 0xa5;
const UNLOCKED: u8 = 2;
/// The extras' words: the state's locked-out byte and lock level, a lock's or teleport's data.
const EXTRA_WORD: u32 = 0x0c;
const STATE_LOCK_LEVEL: u32 = 0x0d;
const LOCK_TRIES: u32 = 0x0c;
const LOCK_DATA_SIZE: u32 = 0x14;
/// A reference's base form (+0x20) and form flags (+8).
const BASE: u32 = 0x20;
const FLAGS: u32 = 0x08;
/// The `MarkModified` flags JIP gives: a terminal state change, a lock change.
const CHANGED_TERMINAL: u32 = 0x8000_0000;
const CHANGED_LOCK: u32 = 0x1000;

/// The optional terminal argument, else the calling reference's base form; `None` with neither.
fn terminal_arg(e: &Engine, a: &CommandArgs, read: &Read, i: usize) -> Option<u32> {
    let t = read.form(i);
    if t != 0 {
        Some(t)
    } else if a.this_obj != 0 {
        Some(e.mem.u32(a.this_obj + BASE))
    } else {
        None
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_terminal.h:
// Cmd_GetTerminalLock_Execute.
/// -1 for an unlocked terminal, else its difficulty.
fn get_terminal_lock(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(t) = terminal_arg(e, &a, &read, 0).filter(|&t| type_of(e, t) == BGS_TERMINAL) {
            let r = if e.mem.u8(t + TERMINAL_FLAGS) & UNLOCKED != 0 {
                -1.0
            } else {
                f64::from(e.mem.u8(t + DIFFICULTY))
            };
            a.set_result(e, r);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_terminal.h:
// Cmd_GetPasswordNote_Execute.
fn get_password_note(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(t) = terminal_arg(e, &a, &read, 0).filter(|&t| type_of(e, t) == BGS_TERMINAL) {
            let note = e.mem.u32(t + PASSWORD);
            if note != 0 {
                set_form_result(e, &a, note);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_terminal.h:
// Cmd_SetPasswordNote_Execute.
fn set_password_note(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let note = read.form(0);
    if read.ok && type_of(e, note) == BGS_NOTE {
        if let Some(t) = terminal_arg(e, &a, &read, 1).filter(|&t| type_of(e, t) == BGS_TERMINAL) {
            e.mem.set_u32(t + PASSWORD, note);
        }
    }
    handled(true)
}

/// The reference argument, else the calling reference.
fn refr_arg(a: &CommandArgs, read: &Read, i: usize) -> Option<u32> {
    let r = read.form(i);
    if r != 0 {
        Some(r)
    } else if a.this_obj != 0 {
        Some(a.this_obj)
    } else {
        None
    }
}

/// The lock data of the reference's `ExtraLock`, 0 for none.
fn lock_data(e: &mut Engine, refr: u32) -> u32 {
    match extra::refr_extra(e, refr, kind::LOCK) {
        0 => 0,
        x => e.mem.u32(x + EXTRA_WORD),
    }
}

/// A door's linked door (its `ExtraTeleport`'s data's first word), 0 for none.
fn linked_door(e: &mut Engine, door: u32) -> u32 {
    match extra::refr_extra(e, door, kind::TELEPORT) {
        0 => 0,
        x => match e.mem.u32(x + EXTRA_WORD) {
            0 => 0,
            data => e.mem.u32(data),
        },
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_terminal.h:
// Cmd_GetLockedOut_Execute.
/// 3 for a reference with form flag 0x100; a terminal its state's locked-out value (low 4 bits,
/// 0 without a state); else its lock's tries (low 4 bits); else for a door its linked door's.
fn get_locked_out(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(r) = read.ok.then(|| refr_arg(&a, &read, 0)).flatten() else {
        return handled(true);
    };
    let base = e.mem.u32(r + BASE);
    let res = if e.mem.u32(r + FLAGS) & 0x100 != 0 {
        Some(3)
    } else if type_of(e, base) == BGS_TERMINAL {
        match extra::refr_extra(e, r, kind::TERMINAL_STATE) {
            0 => None,
            x => Some(u32::from(e.mem.u8(x + EXTRA_WORD) & 0xf)),
        }
    } else {
        match lock_data(e, r) {
            0 if type_of(e, base) == TES_OBJECT_DOOR => match linked_door(e, r) {
                0 => None,
                door => match lock_data(e, door) {
                    0 => None,
                    d => Some(e.mem.u32(d + LOCK_TRIES) & 0xf),
                },
            },
            0 => None,
            d => Some(e.mem.u32(d + LOCK_TRIES) & 0xf),
        }
    };
    if let Some(v) = res {
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp: ExtraLock::Create.
/// A new `ExtraLock` with zeroed lock data (both of the game's heap), added to the list.
fn add_lock(e: &mut Engine, refr: u32) -> u32 {
    let x = extra::create(e, vtbl::LOCK, kind::LOCK);
    let data = heap_alloc(e, LOCK_DATA_SIZE);
    e.mem.write(data, &[0; LOCK_DATA_SIZE as usize]);
    e.mem.set_u32(x + EXTRA_WORD, data);
    extra::add_extra(e, refr + REFR_EXTRA_LIST, x);
    data
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_terminal.h:
// Cmd_SetLockedOut_Execute.
/// State 0 to 2: a terminal's state (made with the terminal's difficulty as its lock level when
/// missing, not for 0), marked as a terminal change; else the reference's lock tries; else a
/// door's linked door's; else a new lock (not for 0); each lock change marked.
fn set_locked_out(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let state = read.int(0, 0);
    if !read.ok || state > 2 {
        return handled(true);
    }
    let Some(mut r) = refr_arg(&a, &read, 1) else {
        return handled(true);
    };
    let base = e.mem.u32(r + BASE);
    if type_of(e, base) == BGS_TERMINAL {
        let mut x = extra::refr_extra(e, r, kind::TERMINAL_STATE);
        if x == 0 {
            if state == 0 {
                return handled(true);
            }
            let made = extra::create(e, vtbl::TERMINAL_STATE, kind::TERMINAL_STATE);
            e.mem.set_u32(made + EXTRA_WORD, 0);
            x = extra::add_extra(e, r + REFR_EXTRA_LIST, made);
            let level = e.mem.u8(base + DIFFICULTY);
            e.mem.set_u8(x + STATE_LOCK_LEVEL, level);
        }
        e.mem.set_u8(x + EXTRA_WORD, state as u8);
        extra::mark_modified(e, r, CHANGED_TERMINAL);
        return handled(true);
    }
    let d = lock_data(e, r);
    if d != 0 {
        e.mem.set_u32(d + LOCK_TRIES, state);
        extra::mark_modified(e, r, CHANGED_LOCK);
        return handled(true);
    }
    if type_of(e, base) == TES_OBJECT_DOOR {
        let door = linked_door(e, r);
        if door != 0 {
            r = door;
            let d = lock_data(e, r);
            if d != 0 {
                e.mem.set_u32(d + LOCK_TRIES, state);
                extra::mark_modified(e, r, CHANGED_LOCK);
                return handled(true);
            }
        }
    }
    if state != 0 {
        let d = add_lock(e, r);
        e.mem.set_u32(d + LOCK_TRIES, state);
        extra::mark_modified(e, r, CHANGED_LOCK);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_terminal_tests.rs"]
mod tests;
