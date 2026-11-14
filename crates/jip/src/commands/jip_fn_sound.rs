//! `functions_jip/jip_fn_sound.h` (JIP LN NVSE): a sound form's numbers, flags and file, the
//! music's state, and the game's volumes. 9 of its 14 commands.
//!
//! Left for later: `SetSoundTraitNumeric`, `IsSoundPlaying`, `GetSoundPlayers` and `StopSound`
//! find the playing sounds of a form by the form each `BSGameSound` holds at +0x134, a field JIP's
//! `SetSourceSoundRequestHook` writes there (in the game it is the end of the file path): that
//! hook is the core track's, and these come with it. `PlaySound3DNode` needs JIP's node lookup by
//! name (`GetNode2`), built with the node commands.
//!
//! `TESSound` (form type 0x0D; the exe's layout in engine-fallout-misc `falloutaudio.rs`): its
//! file (`TESSoundFile`, path at +0x34, set through the component's virtual `Set`, +0x10), and the
//! sound data from +0x44: minimum and maximum attenuation distances (bytes, in fives and
//! hundreds), frequency adjustment (a signed byte), flags +0x48, static attenuation +0x4C (16
//! bits), ends and starts at +0x4E/+0x4F, the attenuation curve +0x50 (five 16-bit values), reverb
//! +0x5A, priority +0x5C. `PlayingMusic` (`011DD0F0`): track paths +0 and +0x108, flags +0x220
//! and +0x221, which track +0x27C, the location controller +0x284. `BSAudioManager` (`011F6EF0`):
//! the volumes, floats from +0x140.

use super::{assign_string, buffer_arg, string_text, type_of, Handlers};
use crate::utility::str_begins_ci;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2646, get_sound_trait_numeric),
    (0x2648, get_sound_flag),
    (0x2649, set_sound_flag),
    (0x26bf, get_sound_source_file),
    (0x26c0, set_sound_source_file),
    (0x2862, is_music_playing),
    (0x2863, set_music_state),
    (0x288c, get_game_volume),
    (0x288d, set_game_volume),
];

const SOUND_FILE: u32 = 0x30;
const SOUND_PATH: u32 = 0x34;
const MIN_ATT: u32 = 0x44;
const MAX_ATT: u32 = 0x45;
const FREQ_ADJ: u32 = 0x46;
const FLAGS: u32 = 0x48;
const STATIC_ATT: u32 = 0x4c;
const ENDS_AT: u32 = 0x4e;
const STARTS_AT: u32 = 0x4f;
const ATT_CURVE: u32 = 0x50;
const REVERB: u32 = 0x5a;
const PRIORITY: u32 = 0x5c;
/// `TESSoundFile::Set` (the component's vtable +0x10).
const SOUND_FILE_SET: u32 = 0x10;
const PLAYING_MUSIC: u32 = 0x011d_d0f0;
const TRACK2_PATH: u32 = 0x108;
const TRACK1_FLAGS: u32 = 0x220;
const TRACK2_FLAGS: u32 = 0x221;
const TRACK1_ACTIVE: u32 = 0x27c;
const MED_LOC_CTRL: u32 = 0x284;
/// `kMusicState_Pause`, `_Stop`, `_Play`.
const PAUSE: u8 = 4;
const STOP: u8 = 8;
const PLAY: u8 = 0x10;
const AUDIO_MANAGER: u32 = 0x011f_6ef0;
const VOLUMES: u32 = 0x140;
/// The default volumes JIP reads (`*(float*)(0x11F6E18 + type * 0xC)`).
const DEFAULT_VOLUMES: u32 = 0x011f_6e18;
/// `kFormType_BGSMusicType`; a music type's file path (`soundFile.path`: text +0x1C, length
/// +0x20).
const BGS_MUSIC_TYPE: u8 = 0x66;
const MUSIC_PATH: u32 = 0x1c;
const MUSIC_PATH_LEN: u32 = 0x20;
/// JIP's `char path[0x80]`.
const PATH_BUFFER: usize = 0x80;
/// `1434 / 255.0F` as the float the compiler folds it to.
const TIME_SCALE: f32 = 1434.0 / 255.0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_GetSoundTraitNumeric_Execute.
/// 0 the minimum distance times 5, 1 the maximum times 100, 2 the frequency adjustment, 3 the
/// static attenuation times -0.01, 4 and 5 the ends and starts times 1434/255 (a float product),
/// 6 to 10 the curve, 11 the reverb, 12 the priority; others nothing.
fn get_sound_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let s = read.form(0);
    let t = read.int(1, 0);
    let r = match t {
        0 => f64::from(e.mem.u8(s + MIN_ATT)) * 5.0,
        1 => f64::from(e.mem.u8(s + MAX_ATT)) * 100.0,
        2 => f64::from(e.mem.u8(s + FREQ_ADJ) as i8),
        3 => f64::from(e.mem.u16(s + STATIC_ATT)) * -0.01,
        4 => f64::from(f32::from(e.mem.u8(s + ENDS_AT)) * TIME_SCALE),
        5 => f64::from(f32::from(e.mem.u8(s + STARTS_AT)) * TIME_SCALE),
        6..=10 => f64::from(e.mem.u16(s + ATT_CURVE + 2 * (t - 6))),
        11 => f64::from(e.mem.u16(s + REVERB)),
        12 => f64::from(e.mem.u32(s + PRIORITY) as i32),
        _ => return handled(true),
    };
    a.set_result(e, r);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_GetSoundFlag_Execute.
fn get_sound_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if read.ok && flag <= 31 && e.mem.u32(read.form(0) + FLAGS) & (1 << flag) != 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_SetSoundFlag_Execute.
fn set_sound_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if read.ok && flag <= 31 {
        let at = read.form(0) + FLAGS;
        let v = e.mem.u32(at);
        let bit = 1u32 << flag;
        e.mem.set_u32(
            at,
            if read.int(2, 0) != 0 {
                v | bit
            } else {
                v & !bit
            },
        );
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_GetSoundSourceFile_Execute.
fn get_sound_source_file(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = read
        .ok
        .then(|| string_text(e, read.form(0) + SOUND_PATH))
        .flatten();
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_SetSoundSourceFile_Execute.
/// The file set through the component's own `Set` (the game's), the path given in JIP's buffer.
fn set_sound_source_file(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetSoundSourceFile");
        let component = read.form(0) + SOUND_FILE;
        let size = (path.len() as u32 + 1).next_multiple_of(4);
        e.with_stack(size, |e, buffer| {
            e.mem.write(buffer.addr(), &path);
            e.mem.set_u8(buffer.addr() + path.len() as u32, 0);
            e.vcall(component, SOUND_FILE_SET, &[buffer.addr()]);
        });
    }
    handled(true)
}

/// The flags of the music track playing (`track1Active` picks the first).
fn track_flags_at(e: &Engine) -> u32 {
    if e.mem.u32(PLAYING_MUSIC + TRACK1_ACTIVE) != 0 {
        PLAYING_MUSIC + TRACK1_FLAGS
    } else {
        PLAYING_MUSIC + TRACK2_FLAGS
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_IsMusicPlaying_Execute.
/// 0 when no music plays; else 1 paused or 2 playing, for no form or the location controller
/// playing, or for a music type whose path the playing track's path begins with (case blind).
fn is_music_playing(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut state = 0.0;
    if read.ok {
        let form = read.form(0);
        let flags = e.mem.u8(track_flags_at(e));
        let playing = if flags & PAUSE != 0 { 1.0 } else { 2.0 };
        if flags & PLAY != 0 {
            if form == 0 || form == e.mem.u32(PLAYING_MUSIC + MED_LOC_CTRL) {
                state = playing;
            } else if type_of(e, form) == BGS_MUSIC_TYPE && e.mem.u16(form + MUSIC_PATH_LEN) != 0 {
                let track = if e.mem.u32(PLAYING_MUSIC + TRACK1_ACTIVE) != 0 {
                    PLAYING_MUSIC
                } else {
                    PLAYING_MUSIC + TRACK2_PATH
                };
                let path = nvse::table::guest_text(e, e.mem.u32(form + MUSIC_PATH));
                if str_begins_ci(&nvse::table::guest_text(e, track), &path) != 0 {
                    state = playing;
                }
            }
        }
    }
    a.set_result(e, state);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_SetMusicState_Execute.
/// With music playing: 0 stops it (the flags set to stop and play), 1 pauses, 2 resumes.
fn set_music_state(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let at = track_flags_at(e);
        let f = e.mem.u8(at);
        if f & PLAY != 0 {
            match read.int(0, 0) {
                0 => e.mem.set_u8(at, STOP | PLAY),
                1 => e.mem.set_u8(at, f | PAUSE),
                2 => e.mem.set_u8(at, f & !PAUSE),
                _ => {}
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_GetGameVolume_Execute.
/// Volume 0 to 5 times 100 (a float product).
fn get_game_volume(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(0, 0);
    if read.ok && t <= 5 {
        let v = e.mem.f32(AUDIO_MANAGER + VOLUMES + 4 * t) * 100.0;
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_sound.h:
// Cmd_SetGameVolume_Execute.
/// Volume 0 to 5: the level times 0.01 (a float product), or for a negative level (the default)
/// the game's default; a level above 100 nothing.
fn set_game_volume(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (t, level) = (read.int(0, 0), read.int(1, u32::MAX) as i32);
    if read.ok && t <= 5 && level <= 100 {
        let v = if level < 0 {
            e.mem.f32(DEFAULT_VOLUMES + t * 0xc)
        } else {
            level as f32 * 0.01
        };
        e.mem.set_f32(AUDIO_MANAGER + VOLUMES + 4 * t, v);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_sound_tests.rs"]
mod tests;
