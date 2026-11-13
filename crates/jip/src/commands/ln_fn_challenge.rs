//! `functions_ln/ln_fn_challenge.h` (JIP LN NVSE): a challenge's type, flags, threshold,
//! interval, three values and two forms. All 17 of its commands. The challenge is read as
//! `kParamType_Challenge`, whose type check xNVSE's reader makes (JIP checks nothing more).
//!
//! The challenge loader (`005F51A0`) reads `DATA` (0x18 bytes) at +0x54: type, threshold, flags,
//! interval, two 16-bit values, a 32-bit value; `SNAM` and `XNAM` go to +0x74 and +0x78 (JIP's
//! header; the loader stores them through setters).
//!
//! Casts: the getters answer `(int)` of the 32-bit fields and the 16-bit values unsigned; the
//! setters store the script's `UInt32` whole, into the 16-bit values its low half.

use super::forms::TESChallenge;
use super::{set_form_result, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x22c2, get_challenge_type),
    (0x22c3, get_challenge_threshold),
    (0x22c4, set_challenge_threshold),
    (0x22c5, get_challenge_interval),
    (0x22c6, set_challenge_interval),
    (0x22c7, get_challenge_value1),
    (0x22c8, set_challenge_value1),
    (0x22c9, get_challenge_value2),
    (0x22ca, set_challenge_value2),
    (0x22cb, get_challenge_value3),
    (0x22cc, set_challenge_value3),
    (0x22cd, get_challenge_form1),
    (0x22ce, set_challenge_form1),
    (0x22cf, get_challenge_form2),
    (0x22d0, set_challenge_form2),
    (0x231a, get_challenge_flags),
    (0x231b, set_challenge_flags),
];

fn challenge_arg(read: &Read) -> Option<Ptr<TESChallenge>> {
    read.ok.then(|| Ptr::new(read.form(0)))
}

fn get_int(e: &mut Engine, w: &[u32], field: Field<TESChallenge, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = challenge_arg(&read) {
        let v = e.get(c, field) as i32;
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

fn set_int(e: &mut Engine, w: &[u32], field: Field<TESChallenge, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = challenge_arg(&read) {
        e.set(c, field, read.int(1, 0));
    }
    handled(true)
}

fn get_short(e: &mut Engine, w: &[u32], field: Field<TESChallenge, u16>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = challenge_arg(&read) {
        let v = e.get(c, field);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

fn set_short(e: &mut Engine, w: &[u32], field: Field<TESChallenge, u16>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = challenge_arg(&read) {
        e.set(c, field, read.int(1, 0) as u16);
    }
    handled(true)
}

fn get_form(e: &mut Engine, w: &[u32], field: Field<TESChallenge, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = challenge_arg(&read) {
        let form = e.get(c, field);
        if form != 0 {
            set_form_result(e, &a, form);
        }
    }
    handled(true)
}

fn set_form(e: &mut Engine, w: &[u32], field: Field<TESChallenge, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = challenge_arg(&read) {
        e.set(c, field, read.form(1));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeType_Execute.
/// The type (`(int)`); -1 when the read failed (the result is always written).
fn get_challenge_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut kind = -1;
    if let Some(c) = challenge_arg(&read) {
        kind = e.get(c, TESChallenge::kind) as i32;
    }
    a.set_result(e, f64::from(kind));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeFlags_Execute.
fn get_challenge_flags(e: &mut Engine, w: &[u32]) -> Ret {
    get_int(e, w, TESChallenge::flags)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_SetChallengeFlags_Execute.
fn set_challenge_flags(e: &mut Engine, w: &[u32]) -> Ret {
    set_int(e, w, TESChallenge::flags)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeThreshold_Execute.
fn get_challenge_threshold(e: &mut Engine, w: &[u32]) -> Ret {
    get_int(e, w, TESChallenge::threshold)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_SetChallengeThreshold_Execute.
fn set_challenge_threshold(e: &mut Engine, w: &[u32]) -> Ret {
    set_int(e, w, TESChallenge::threshold)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeInterval_Execute.
fn get_challenge_interval(e: &mut Engine, w: &[u32]) -> Ret {
    get_int(e, w, TESChallenge::interval)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_SetChallengeInterval_Execute.
fn set_challenge_interval(e: &mut Engine, w: &[u32]) -> Ret {
    set_int(e, w, TESChallenge::interval)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeValue1_Execute.
fn get_challenge_value1(e: &mut Engine, w: &[u32]) -> Ret {
    get_short(e, w, TESChallenge::value1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_SetChallengeValue1_Execute.
fn set_challenge_value1(e: &mut Engine, w: &[u32]) -> Ret {
    set_short(e, w, TESChallenge::value1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeValue2_Execute.
fn get_challenge_value2(e: &mut Engine, w: &[u32]) -> Ret {
    get_short(e, w, TESChallenge::value2)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_SetChallengeValue2_Execute.
fn set_challenge_value2(e: &mut Engine, w: &[u32]) -> Ret {
    set_short(e, w, TESChallenge::value2)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeValue3_Execute.
fn get_challenge_value3(e: &mut Engine, w: &[u32]) -> Ret {
    get_int(e, w, TESChallenge::value3)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_SetChallengeValue3_Execute.
fn set_challenge_value3(e: &mut Engine, w: &[u32]) -> Ret {
    set_int(e, w, TESChallenge::value3)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeForm1_Execute.
fn get_challenge_form1(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, TESChallenge::SNAM)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_SetChallengeForm1_Execute.
fn set_challenge_form1(e: &mut Engine, w: &[u32]) -> Ret {
    set_form(e, w, TESChallenge::SNAM)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_GetChallengeForm2_Execute.
fn get_challenge_form2(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, TESChallenge::XNAM)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_challenge.h:
// Cmd_SetChallengeForm2_Execute.
fn set_challenge_form2(e: &mut Engine, w: &[u32]) -> Ret {
    set_form(e, w, TESChallenge::XNAM)
}

#[cfg(test)]
#[path = "ln_fn_challenge_tests.rs"]
mod tests;
