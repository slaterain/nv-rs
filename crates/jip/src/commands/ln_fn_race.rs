//! `functions_ln/ln_fn_race.h` (JIP LN NVSE): a race's voices and older/younger races, and an
//! NPC's race. All 5 of its commands. Forms are read as any form; each command checks `IS_ID`
//! (race 0x0C, voice type 0x5D, NPC 0x2A).
//!
//! The race loader (`00610CD0`) sets the voices through `00610C70` (index 0 male, 1 female; JIP's
//! `voiceTypes[2]` at +0x4FC) and stores `ONAM` at +0x504 and `YNAM` at +0x508 (`ageRace[2]`). The
//! gender or age argument picks entry `!= 0`.
//!
//! `SetRace` is JIP's `TESNPC::SetRace` (`nvse/GameForms.cpp`): nothing when the NPC already has
//! the race (+0x110); for the player's own base form the game's `0060B240` (`thiscall`: the NPC,
//! the race, 0) makes the change; for any other NPC the word is written.

use super::forms::{form_type_ln, TESRace, TESNPC};
use super::{set_form_result, type_of, Handlers};
use crate::game::TESObjectREFR;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2286, get_race_voice),
    (0x2287, set_race_voice),
    (0x2288, get_race_age_race),
    (0x2289, set_race_age_race),
    (0x22a2, set_race),
];

/// `g_thePlayer` (JIP LN NVSE).
const PLAYER: u32 = 0x011d_ea3c;
/// The game's change of the player's race (`0060B240`, `thiscall`).
const SET_PLAYER_RACE: u32 = 0x0060_b240;

fn race(e: &Engine, form: u32) -> Option<Ptr<TESRace>> {
    (type_of(e, form) == form_type_ln::TES_RACE).then(|| Ptr::new(form))
}

fn race_arg(e: &Engine, read: &Read) -> Option<Ptr<TESRace>> {
    if read.ok {
        race(e, read.form(0))
    } else {
        None
    }
}

/// A getter of a two-entry field: entry `arg != 0`'s id.
fn get_pair(e: &mut Engine, w: &[u32], field: Field<TESRace, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(r) = race_arg(e, &read) {
        let index = u32::from(read.int(1, 0) != 0);
        let form = e.get(r.index(index, 4), field);
        if form != 0 {
            set_form_result(e, &a, form);
        }
    }
    handled(true)
}

/// A setter of a two-entry field: entry `arg != 0` set to a form of `kind`.
fn set_pair(e: &mut Engine, w: &[u32], field: Field<TESRace, u32>, kind: u8) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(2);
    if let Some(r) = race_arg(e, &read) {
        if type_of(e, form) == kind {
            let index = u32::from(read.int(1, 0) != 0);
            e.set(r.index(index, 4), field, form);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_race.h:
// Cmd_GetRaceVoice_Execute.
fn get_race_voice(e: &mut Engine, w: &[u32]) -> Ret {
    get_pair(e, w, TESRace::voiceTypes)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_race.h:
// Cmd_SetRaceVoice_Execute.
fn set_race_voice(e: &mut Engine, w: &[u32]) -> Ret {
    set_pair(e, w, TESRace::voiceTypes, form_type_ln::BGS_VOICE_TYPE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_race.h:
// Cmd_GetRaceAgeRace_Execute.
fn get_race_age_race(e: &mut Engine, w: &[u32]) -> Ret {
    get_pair(e, w, TESRace::ageRace)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_race.h:
// Cmd_SetRaceAgeRace_Execute.
fn set_race_age_race(e: &mut Engine, w: &[u32]) -> Ret {
    set_pair(e, w, TESRace::ageRace, form_type_ln::TES_RACE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_race.h: Cmd_SetRace_Execute,
// with nvse/GameForms.cpp: TESNPC::SetRace.
fn set_race(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (npc, new_race) = (read.form(0), read.form(1));
    if !read.ok || type_of(e, npc) != form_type_ln::TES_NPC || race(e, new_race).is_none() {
        return handled(true);
    }
    let n = Ptr::<TESNPC>::new(npc);
    if e.get(n, TESNPC::race) == new_race {
        return handled(true);
    }
    let player = Ptr::<TESObjectREFR>::new(e.mem.u32(PLAYER));
    if e.get(player, TESObjectREFR::pObjectReference) == npc {
        e.call(SET_PLAYER_RACE, &args![npc, new_race, 0u32]);
    } else {
        e.set(n, TESNPC::race, new_race);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_race_tests.rs"]
mod tests;
