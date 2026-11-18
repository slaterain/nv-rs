//! `functions_jip/jip_fn_sound.h` (JIP LN NVSE): the four commands that find a sound form's
//! playing sounds: `SetSoundTraitNumeric` (a number of the form, and for the frequency and the
//! static attenuation the sounds playing it), `IsSoundPlaying`, `GetSoundPlayers` and
//! `StopSound` (JIP's `PlayingSoundsIterator`). The rest of the file's commands are in
//! `jip_fn_sound.rs`.
//!
//! How JIP finds them: the audio manager's (`0x011F6EF0`) map of playing sounds
//! (`playingSounds`, an `NiTPointerMap` at +0x54: bucket count +4, buckets +8; an entry's next
//! +0, key +4, sound +8), walked bucket by bucket; a sound (`BSGameSound`) plays a form when the
//! form JIP keeps in it (+0x134, the end of the file path's buffer, written by JIP's patch G108
//! when the sound starts: `game_patches::gw`) is that form. For a reference: the scene object
//! the manager plays the sound at (`soundPlayingObjects`, the map at +0x84, looked up by the
//! entry's key: bucket `key % count`), and that object's reference (`NiAVObject::GetParentRef`,
//! [`super::havok_refs::block_parent_ref`]). The objects map's lookup is checked in the exe: its
//! vtable `0x010A3308` (written in the manager's constructor `00AD90E0`) has the bucket function
//! `00A0D260`, the unsigned `key % count`, and the key compare `00AE8DC0`, plain equality. A
//! sound's state flags (+0x10): stopping keeps all but bits 4-7 and sets 0x10. Its frequency
//! factor (+0x138, JIP's `float`) and static attenuation (+0x18, 16 bits).
//!
//! G108 is in with the game patches row (`NV_JIP_GAME_PATCHES`, the fixes preset); JIP puts it
//! in always. Without the row the word at +0x134 is the end of the sound's file name, so the
//! port then finds no playing sound (a deliberate difference: JIP's comparison would read the
//! name's bytes as a form): `IsSoundPlaying` 0, `GetSoundPlayers` none, `StopSound` and the
//! playing sounds' part of `SetSoundTraitNumeric` nothing (the form's numbers still change).
//!
//! Safety: every guest word of the maps read checked, the walks bounded ([`MAX_WALK`] steps
//! each, buckets and entries); an unreadable word ends that walk. Threads: on the script's thread with no lock, as
//! JIP's (the audio thread changes these maps too).

use super::arrays::Element;
use super::havok_refs::{block_parent_ref, Walk};
use super::nif_nodes as nif;
use super::{cvttss2si, Handlers};
use engine_core::extend::jip_patches;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2647, set_sound_trait_numeric),
    (0x278a, is_sound_playing),
    (0x278e, get_sound_players),
    (0x2861, stop_sound),
];

/// The audio manager and its two maps.
const AUDIO_MANAGER: u32 = 0x011f_6ef0;
const PLAYING_SOUNDS: u32 = 0x54;
const PLAYING_OBJECTS: u32 = 0x84;
/// An `NiTPointerMap`: bucket count +4, buckets +8; an entry's next +0, key +4, value +8.
const MAP_COUNT: u32 = 0x04;
const MAP_BUCKETS: u32 = 0x08;
const ENTRY_KEY: u32 = 0x04;
const ENTRY_VALUE: u32 = 0x08;
/// A `BSGameSound`: its map key (+4), state flags (+0x10), static attenuation (+0x18), JIP's form
/// (+0x134) and frequency factor (+0x138).
const SOUND_MAP_KEY: u32 = 0x04;
const SOUND_STATE: u32 = 0x10;
const SOUND_STATIC_ATT: u32 = 0x18;
const SOUND_FORM: u32 = 0x134;
const SOUND_FREQUENCY: u32 = 0x138;
/// `TESSound`'s numbers (as `jip_fn_sound.rs` reads them).
const MIN_ATT: u32 = 0x44;
const MAX_ATT: u32 = 0x45;
const FREQ_ADJ: u32 = 0x46;
const STATIC_ATT: u32 = 0x4c;
const ENDS_AT: u32 = 0x4e;
const STARTS_AT: u32 = 0x4f;
const ATT_CURVE: u32 = 0x50;
const REVERB: u32 = 0x5a;
const PRIORITY: u32 = 0x5c;
/// The most steps (buckets and entries) a map walk takes.
const MAX_WALK: u32 = 1 << 16;

/// The playing sounds' entries (key, sound), in JIP's iterator's order; none without the game
/// patches row (see the module's doc).
fn playing_sounds(e: &Engine) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    if e.jip_patches() & jip_patches::FIXES == 0 {
        return out;
    }
    let map = AUDIO_MANAGER + PLAYING_SOUNDS;
    let (Some(count), Some(buckets)) = (
        nif::rd32(e, map + MAP_COUNT),
        nif::rd32(e, map + MAP_BUCKETS),
    ) else {
        return out;
    };
    let mut walked = 0;
    for i in 0..count {
        // Each bucket counts toward the walk's limit too (a bad count over a readable array).
        walked += 1;
        if walked > MAX_WALK {
            return out;
        }
        let Some(mut entry) = nif::rd32(e, buckets.wrapping_add(4 * i)) else {
            return out;
        };
        while entry != 0 {
            walked += 1;
            if walked > MAX_WALK {
                return out;
            }
            let (Some(next), Some(key), Some(sound)) = (
                nif::rd32(e, entry),
                nif::rd32(e, entry.wrapping_add(ENTRY_KEY)),
                nif::rd32(e, entry.wrapping_add(ENTRY_VALUE)),
            ) else {
                return out;
            };
            out.push((key, sound));
            entry = next;
        }
    }
    out
}

/// The sounds playing `form` (non-null sounds whose JIP form is it), with their keys.
fn sounds_of(e: &Engine, form: u32) -> Vec<(u32, u32)> {
    playing_sounds(e)
        .into_iter()
        .filter(|&(_, s)| s != 0 && nif::rd32(e, s.wrapping_add(SOUND_FORM)) == Some(form))
        .collect()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/NiTypes.h: NiTMap::Lookup.
/// The scene object the manager plays the sound of `key` at; 0 for none.
fn playing_object(e: &Engine, key: u32) -> u32 {
    let map = AUDIO_MANAGER + PLAYING_OBJECTS;
    let (Some(count), Some(buckets)) = (
        nif::rd32(e, map + MAP_COUNT),
        nif::rd32(e, map + MAP_BUCKETS),
    ) else {
        return 0;
    };
    if count == 0 {
        return 0;
    }
    let mut entry = nif::rd32(e, buckets.wrapping_add(4 * (key % count))).unwrap_or(0);
    let mut walked = 0;
    while entry != 0 && walked < MAX_WALK {
        walked += 1;
        if nif::rd32(e, entry.wrapping_add(ENTRY_KEY)) == Some(key) {
            return nif::rd32(e, entry.wrapping_add(ENTRY_VALUE)).unwrap_or(0);
        }
        entry = nif::rd32(e, entry).unwrap_or(0);
    }
    0
}

/// The reference the sound of `key` plays at; 0 for none.
fn player_of(e: &Engine, key: u32) -> u32 {
    let object = playing_object(e, key);
    if object == 0 {
        return 0;
    }
    block_parent_ref(e, object, &mut Walk::new())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// PlayingSoundsIterator.
/// Whether `form` plays (at `refr` when not 0: a reference with no 3D has none); with `stop`
/// every such sound is told to stop (and the answer is false, as JIP's).
fn playing_sounds_iterator(e: &mut Engine, form: u32, stop: bool, refr: u32) -> bool {
    if refr != 0 && nif::ref_ni_node(e, refr) == 0 {
        return false;
    }
    for (key, sound) in sounds_of(e, form) {
        if refr != 0 && player_of(e, key) != refr {
            continue;
        }
        if !stop {
            return true;
        }
        let at = sound.wrapping_add(SOUND_STATE);
        let flags = e.mem.u32(at);
        e.mem.set_u32(at, (flags & 0xffff_ff0f) | 0x10);
    }
    false
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_IsSoundPlaying_Execute.
/// A sound form and, optionally, a reference: 1 when the form plays (there).
fn is_sound_playing(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && playing_sounds_iterator(e, read.form(0), false, read.form(1)) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_StopSound_Execute.
/// A sound form and, optionally, a reference: every sound playing it (there) stopped.
fn stop_sound(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        playing_sounds_iterator(e, read.form(0), true, read.form(1));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_GetSoundPlayers_Execute.
/// A sound form: an array of the references it plays at (one per playing sound, in the map's
/// order), none when there is none.
fn get_sound_players(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let form = read.form(0);
    let mut players = Vec::new();
    for (_, sound) in sounds_of(e, form) {
        // JIP looks the object up by the sound's own map key (+4).
        let Some(key) = nif::rd32(e, sound.wrapping_add(SOUND_MAP_KEY)) else {
            continue;
        };
        let refr = player_of(e, key);
        if refr != 0 {
            players.push(Element::Form(refr));
        }
    }
    if !players.is_empty() {
        let id = super::arrays::create_array(e, a.script, &players);
        a.set_result(e, f64::from(id as i32));
    }
    handled(true)
}

/// `value` kept to `[low, high]` as JIP's `if > high ... else if < low` chain.
fn clamp(value: i32, low: i32, high: i32) -> i32 {
    if value > high {
        high
    } else if value < low {
        low
    } else {
        value
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_SetSoundTraitNumeric_Execute.
/// A sound form, a trait and a value: 0 the minimum distance (in fives, 255 past 1275), 1 the
/// maximum (in hundreds, 255 past 25500), 2 the frequency adjustment (-100 to 100; the playing
/// sounds' factor set again, JIP's `GetFrequencyModifier`), 3 the static attenuation (its size
/// times 100, 10000 past 100; the playing sounds' too), 4 and 5 the ends and starts (the value
/// times `255 / 1434.0F`, 255 past 1434), 6 to 10 the curve, 11 the reverb (0 to 100), 12 the
/// priority (0 to 255); others nothing. No result.
fn set_sound_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let sound = read.form(0);
    let trait_id = read.int(1, 0);
    let value = read.float(2, 0.0);
    let int = cvttss2si(value);
    match trait_id {
        0 => {
            let v = if int > 1275 {
                255
            } else if int < 0 {
                0
            } else {
                int / 5
            };
            e.mem.set_u8(sound + MIN_ATT, v as u8);
        }
        1 => {
            let v = if int > 25500 {
                255
            } else if int < 0 {
                0
            } else {
                int / 100
            };
            e.mem.set_u8(sound + MAX_ATT, v as u8);
        }
        2 => {
            e.mem.set_u8(sound + FREQ_ADJ, clamp(int, -100, 100) as u8);
            let factor = crate::game_patches::gw::frequency_modifier(e, sound);
            for (_, s) in sounds_of(e, sound) {
                e.mem.set_f32(s.wrapping_add(SOUND_FREQUENCY), factor);
            }
        }
        3 => {
            let size = if int < 0 { int.wrapping_neg() } else { int };
            let v = if size > 100 {
                10000
            } else {
                size.wrapping_mul(100)
            };
            e.mem.set_u16(sound + STATIC_ATT, v as u16);
            for (_, s) in sounds_of(e, sound) {
                e.mem.set_u16(s.wrapping_add(SOUND_STATIC_ATT), v as u16);
            }
        }
        4 | 5 => {
            let v = if int > 1434 {
                255
            } else if int < 0 {
                0
            } else {
                cvttss2si(value * (255.0f32 / 1434.0))
            };
            let at = if trait_id == 4 { ENDS_AT } else { STARTS_AT };
            e.mem.set_u8(sound + at, v as u8);
        }
        6..=10 => {
            e.mem.set_u16(
                sound + ATT_CURVE + 2 * (trait_id - 6),
                clamp(int, 0, 100) as u16,
            );
        }
        11 => e.mem.set_u16(sound + REVERB, clamp(int, 0, 100) as u16),
        12 => e.mem.set_u32(sound + PRIORITY, clamp(int, 0, 255) as u32),
        _ => {}
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_sound_playing_tests.rs"]
mod tests;
