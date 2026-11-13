//! `functions_ln/ln_fn_casino.h` (JIP LN NVSE): a casino's winnings cap, blackjack rules, decks,
//! shuffle point and slot reel stops. All 12 of its commands. The casino is read as
//! `kParamType_Casino`, whose type check xNVSE's reader makes, except by `SetCasinoReelStops`
//! (any form, then `IS_ID(casino, TESCasino)`, type 0x6D).
//!
//! The numbers are the casino's `DATA` (the casino loader `005048D0` reads its 0x38 bytes at
//! +0x1E8: shuffle percent, blackjack payout, seven reel stops, decks, the winnings cap, the
//! currency and quest ids, the flags). The integers are the script's `UInt32` stored whole and read
//! back as `(int)`; the floats are stored as they are. Nothing is narrowed.

use super::forms::{form_type_ln, TESCasino};
use super::{type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x22b1, get_casino_max_winnings),
    (0x22b2, set_casino_max_winnings),
    (0x22b3, get_casino_bj_payout),
    (0x22b4, set_casino_bj_payout),
    (0x22b5, get_casino_bj17_stand),
    (0x22b6, set_casino_bj17_stand),
    (0x22b7, get_casino_num_decks),
    (0x22b8, set_casino_num_decks),
    (0x22b9, get_casino_shuffle_percent),
    (0x22ba, set_casino_shuffle_percent),
    (0x22bb, get_casino_reel_stops),
    (0x22bc, set_casino_reel_stops),
];

fn casino_arg(read: &Read) -> Option<Ptr<TESCasino>> {
    read.ok.then(|| Ptr::new(read.form(0)))
}

/// A getter of an integer field, answered as `(int)`.
fn get_int(e: &mut Engine, w: &[u32], field: Field<TESCasino, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = casino_arg(&read) {
        let v = e.get(c, field) as i32;
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

/// A setter of an integer field: the script's `UInt32`.
fn set_int(e: &mut Engine, w: &[u32], field: Field<TESCasino, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = casino_arg(&read) {
        e.set(c, field, read.int(1, 0));
    }
    handled(true)
}

fn get_float(e: &mut Engine, w: &[u32], field: Field<TESCasino, f32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = casino_arg(&read) {
        let v = e.get(c, field);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

fn set_float(e: &mut Engine, w: &[u32], field: Field<TESCasino, f32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = casino_arg(&read) {
        e.set(c, field, read.float(1, 0.0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_GetCasinoMaxWinnings_Execute.
fn get_casino_max_winnings(e: &mut Engine, w: &[u32]) -> Ret {
    get_int(e, w, TESCasino::maxWinnings)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_SetCasinoMaxWinnings_Execute.
fn set_casino_max_winnings(e: &mut Engine, w: &[u32]) -> Ret {
    set_int(e, w, TESCasino::maxWinnings)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_GetCasinoBJPayout_Execute.
fn get_casino_bj_payout(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, TESCasino::blackjackPayout)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_SetCasinoBJPayout_Execute.
fn set_casino_bj_payout(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, TESCasino::blackjackPayout)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_GetCasinoBJ17Stand_Execute.
/// The whole flags word (`(int)casinoFlags`).
fn get_casino_bj17_stand(e: &mut Engine, w: &[u32]) -> Ret {
    get_int(e, w, TESCasino::casinoFlags)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_SetCasinoBJ17Stand_Execute.
/// The whole flags word set to 1 or 0 (`casinoFlags = value != 0`).
fn set_casino_bj17_stand(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = casino_arg(&read) {
        e.set(c, TESCasino::casinoFlags, u32::from(read.int(1, 0) != 0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_GetCasinoNumDecks_Execute.
fn get_casino_num_decks(e: &mut Engine, w: &[u32]) -> Ret {
    get_int(e, w, TESCasino::numDecks)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_SetCasinoNumDecks_Execute.
fn set_casino_num_decks(e: &mut Engine, w: &[u32]) -> Ret {
    set_int(e, w, TESCasino::numDecks)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_GetCasinoShufflePercent_Execute.
fn get_casino_shuffle_percent(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, TESCasino::shufflePercent)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_SetCasinoShufflePercent_Execute.
fn set_casino_shuffle_percent(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, TESCasino::shufflePercent)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_GetCasinoReelStops_Execute.
/// Reel 1-7's stops (`(int)`); -1 otherwise, the read failed included (the result is always
/// written).
fn get_casino_reel_stops(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let slot = read.int(1, 0);
    let mut stops = -1;
    if let Some(c) = casino_arg(&read).filter(|_| slot > 0 && slot < 8) {
        stops = e.get(c.index(slot - 1, 4), TESCasino::reelStops) as i32;
    }
    a.set_result(e, f64::from(stops));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_casino.h:
// Cmd_SetCasinoReelStops_Execute.
fn set_casino_reel_stops(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    let slot = read.int(1, 0);
    if read.ok && type_of(e, form) == form_type_ln::TES_CASINO && slot > 0 && slot < 8 {
        let c = Ptr::<TESCasino>::new(form);
        e.set(c.index(slot - 1, 4), TESCasino::reelStops, read.int(2, 0));
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_casino_tests.rs"]
mod tests;
