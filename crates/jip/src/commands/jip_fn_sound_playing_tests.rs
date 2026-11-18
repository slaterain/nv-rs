//! Tests of the sound commands that find a form's playing sounds, by their opcodes, on an audio
//! manager built in guest memory. Expectations from JIP's command bodies and
//! `PlayingSoundsIterator` (functions_jip/jip_fn_sound.h), `GetFrequencyModifier`
//! (internal/patches_game.h) and `NiAVObject::GetParentRef` (internal/netimmerse.cpp), worked by
//! hand.

use super::super::nif_nodes::tests::*;
use crate::commands::test_support::{array_data, call_on, n, r, set_ref, z};
use engine_core::extend::jip_patches::{FIXES, INSTALLED};
use engine_core::prelude::*;
use nvse::vars::Data::Form;

const SET_TRAIT: u32 = 0x2647;
const IS_PLAYING: u32 = 0x278a;
const PLAYERS: u32 = 0x278e;
const STOP: u32 = 0x2861;
const MANAGER: u32 = 0x011f_6ef0;

struct World {
    e: Engine,
    form_a: u32,
    form_b: u32,
    /// The sounds: A at x (key 1), A at y (key 2), B at x (key 3), A with no object (key 4), A at x
    /// (key 5: second in key 1's bucket of the playing sounds, first in key 2's of the objects).
    sounds: [u32; 5],
    x: u32,
    y: u32,
}

/// A form (0x100 bytes) of the given id.
fn sound_form(e: &mut Engine, id: u32) -> u32 {
    let f = object(e, 0, None);
    e.mem.set_u8(f + 4, 0x0d);
    e.mem.set_u32(f + 0xc, id);
    f
}

/// A map at `at` (bucket count `count`) holding `entries` (key, value), each put first in its
/// bucket `key % count` in the order given.
fn map(e: &mut Engine, at: u32, count: u32, entries: &[(u32, u32)]) {
    let buckets = object(e, 0, None);
    for &(key, value) in entries {
        let entry = object(e, 0, None);
        let slot = buckets + 4 * (key % count);
        e.mem.set_u32(entry, e.mem.u32(slot));
        e.mem.set_u32(entry + 4, key);
        e.mem.set_u32(entry + 8, value);
        e.mem.set_u32(slot, entry);
    }
    e.mem.set_u32(at + 4, count);
    e.mem.set_u32(at + 8, buckets);
}

fn world() -> World {
    let mut e = scene_engine();
    e.map(0x011f_6000, 0x1000);
    e.set_jip_patches(INSTALLED | FIXES);
    let form_a = sound_form(&mut e, 0xa1);
    let form_b = sound_form(&mut e, 0xb2);
    // Two references, each with 3D: a fade node (+0xCC the reference) and a node below it where
    // the manager plays.
    let place = |e: &mut Engine, id: u32| {
        let at = node(e, b"Speaker", &[]);
        let fade = object(e, BS_FADE_NODE, Some(b"Root"));
        set_children(e, fade, &[at]);
        let refr = reference(e, id, false, fade);
        e.mem.set_u32(fade + 0xcc, refr);
        (refr, at)
    };
    let (x, x_at) = place(&mut e, 0x501);
    let (y, y_at) = place(&mut e, 0x502);
    let sound = |e: &mut Engine, form: u32, key: u32| {
        // A game sound is 0x1A0 bytes: two more blocks after it (zeroed), then its fields.
        let s = object(e, 0, None);
        object(e, 0, None);
        object(e, 0, None);
        e.mem.set_u32(s + 4, key);
        e.mem.set_u32(s + 0x10, 0xabcd_12f7);
        e.mem.set_u32(s + 0x134, form);
        s
    };
    let sounds = [
        sound(&mut e, form_a, 1),
        sound(&mut e, form_a, 2),
        sound(&mut e, form_b, 3),
        sound(&mut e, form_a, 4),
        sound(&mut e, form_a, 5),
    ];
    // Keys 1..5 in 4 buckets, each put first in its bucket in this order: bucket 1 holds key 1
    // then key 5 (its next), so the walk goes bucket 0 (key 4), 1 (key 1, key 5), 2, 3.
    map(
        &mut e,
        MANAGER + 0x54,
        4,
        &[
            (5, sounds[4]),
            (1, sounds[0]),
            (2, sounds[1]),
            (3, sounds[2]),
            (4, sounds[3]),
        ],
    );
    map(
        &mut e,
        MANAGER + 0x84,
        3,
        // Key 5 after key 2: bucket 2 holds key 5 (x) first, then key 2 (y).
        &[(1, x_at), (2, y_at), (3, x_at), (5, x_at)],
    );
    set_ref(&mut e, 1, form_a);
    set_ref(&mut e, 2, form_b);
    set_ref(&mut e, 3, x);
    set_ref(&mut e, 4, y);
    World {
        e,
        form_a,
        form_b,
        sounds,
        x,
        y,
    }
}

fn playing(w: &mut World, form: u16, refr: Option<u16>) -> f64 {
    let mut args = vec![r(form)];
    if let Some(i) = refr {
        args.push(r(i));
    }
    call_on(&mut w.e, IS_PLAYING, 0, &args).1
}

#[test]
fn is_sound_playing_finds_the_forms_sounds_and_where_they_play() {
    let mut w = world();
    assert_eq!(playing(&mut w, 1, None), 1.0);
    assert_eq!(playing(&mut w, 2, None), 1.0);
    assert_eq!(playing(&mut w, 1, Some(3)), 1.0, "A at x");
    assert_eq!(playing(&mut w, 1, Some(4)), 1.0, "A at y");
    assert_eq!(playing(&mut w, 2, Some(4)), 0.0, "B only at x");
    // A reference without 3D: not playing there.
    let y = w.y;
    set_root(&mut w.e, y, 0);
    assert_eq!(playing(&mut w, 1, Some(4)), 0.0);
    // Another form: no.
    w.e.mem.set_u32(w.sounds[2] + 0x134, 0);
    assert_eq!(playing(&mut w, 2, None), 0.0);
    let _ = (w.form_a, w.form_b);
}

#[test]
fn without_the_game_patches_row_no_sound_is_found() {
    let mut w = world();
    w.e.set_jip_patches(INSTALLED);
    assert_eq!(playing(&mut w, 1, None), 0.0);
    call_on(&mut w.e, STOP, 0, &[r(1)]);
    assert_eq!(w.e.mem.u32(w.sounds[0] + 0x10), 0xabcd_12f7);
    assert_eq!(call_on(&mut w.e, PLAYERS, 0, &[r(1)]).1, 0.0);
}

#[test]
fn stop_sound_stops_every_sound_of_the_form_there() {
    let mut w = world();
    let stopped = (0xabcd_12f7 & 0xffff_ff0f) | 0x10;
    call_on(&mut w.e, STOP, 0, &[r(1), r(4)]);
    let flags: Vec<u32> = w.sounds.iter().map(|&s| w.e.mem.u32(s + 0x10)).collect();
    assert_eq!(
        flags,
        vec![0xabcd_12f7, stopped, 0xabcd_12f7, 0xabcd_12f7, 0xabcd_12f7],
        "A at y only"
    );
    call_on(&mut w.e, STOP, 0, &[r(1)]);
    let flags: Vec<u32> = w.sounds.iter().map(|&s| w.e.mem.u32(s + 0x10)).collect();
    assert_eq!(
        flags,
        vec![stopped, stopped, 0xabcd_12f7, stopped, stopped],
        "every A"
    );
}

#[test]
fn get_sound_players_lists_where_the_form_plays_by_each_sounds_key() {
    let mut w = world();
    // Walk order: bucket 0 (key 4: no object), 1 (key 1: x, then key 5: x), 2 (key 2: y).
    let id = call_on(&mut w.e, PLAYERS, 0, &[r(1)]).1;
    let (x, y) = (w.e.mem.u32(w.x + 0xc), w.e.mem.u32(w.y + 0xc));
    assert_eq!(array_data(&w.e, id), vec![Form(x), Form(x), Form(y)]);
    // The sound's own key (+4) is looked up, not the entry's: A's first sound keyed 2 now.
    w.e.mem.set_u32(w.sounds[0] + 4, 2);
    let id = call_on(&mut w.e, PLAYERS, 0, &[r(1)]).1;
    assert_eq!(array_data(&w.e, id), vec![Form(y), Form(x), Form(y)]);
    // ... while IsSoundPlaying goes by the entry's key (1: at x; key 5's sound, also at x, set
    // to no form first).
    w.e.mem.set_u32(w.sounds[4] + 0x134, 0);
    assert_eq!(playing(&mut w, 1, Some(3)), 1.0);
    // No player: no array.
    let id = call_on(&mut w.e, PLAYERS, 0, &[r(2)]).1;
    assert_eq!(array_data(&w.e, id), vec![Form(x)]);
    w.e.mem.set_u32(w.sounds[2] + 0x134, 0);
    assert_eq!(call_on(&mut w.e, PLAYERS, 0, &[r(2)]).1, 0.0);
}

/// Two entries in one bucket: the playing sounds' walk follows the first one's next link (key 5
/// after key 1), and the objects lookup passes over the first one in a bucket whose key is not
/// the one asked (key 5 before key 2), each in the chain's order.
#[test]
fn the_walks_follow_a_buckets_chain() {
    let mut w = world();
    let (x, y) = (w.e.mem.u32(w.x + 0xc), w.e.mem.u32(w.y + 0xc));
    // Only key 5's sound (second in its bucket) plays A.
    for i in [0, 1, 3] {
        w.e.mem.set_u32(w.sounds[i] + 0x134, 0);
    }
    assert_eq!(playing(&mut w, 1, None), 1.0);
    assert_eq!(playing(&mut w, 1, Some(3)), 1.0, "key 5 at x");
    assert_eq!(playing(&mut w, 1, Some(4)), 0.0);
    let id = call_on(&mut w.e, PLAYERS, 0, &[r(1)]).1;
    assert_eq!(array_data(&w.e, id), vec![Form(x)]);
    call_on(&mut w.e, STOP, 0, &[r(1)]);
    let stopped = (0xabcd_12f7 & 0xffff_ff0f) | 0x10;
    assert_eq!(w.e.mem.u32(w.sounds[4] + 0x10), stopped);
    assert_eq!(w.e.mem.u32(w.sounds[0] + 0x10), 0xabcd_12f7);
    // Only key 2's sound plays A: its object is the second entry of its bucket (y).
    let mut w = world();
    for i in [0, 3, 4] {
        w.e.mem.set_u32(w.sounds[i] + 0x134, 0);
    }
    assert_eq!(playing(&mut w, 1, Some(4)), 1.0, "key 2 at y");
    assert_eq!(playing(&mut w, 1, Some(3)), 0.0);
    let id = call_on(&mut w.e, PLAYERS, 0, &[r(1)]).1;
    assert_eq!(array_data(&w.e, id), vec![Form(y)]);
    call_on(&mut w.e, STOP, 0, &[r(1), r(4)]);
    assert_eq!(w.e.mem.u32(w.sounds[1] + 0x10), stopped);
}

fn set_trait(w: &mut World, form: u16, t: i32, v: f64) {
    call_on(&mut w.e, SET_TRAIT, 0, &[r(form), n(t), z(v)]);
}

#[test]
fn set_sound_trait_numeric_scales_and_clamps_each_number() {
    let mut w = world();
    let a = w.form_a;
    let cases: [(i32, f64, u32, u32); 24] = [
        // (trait, value, offset, expected), each a byte unless noted below.
        (0, 1000.0, 0x44, 200),
        (0, 1276.0, 0x44, 255),
        (0, 1280.0, 0x44, 255),
        (0, -1.0, 0x44, 0),
        // Values that tell the lower clamps from none: -100 / 5 would be -20 (byte 236).
        (0, -100.0, 0x44, 0),
        (1, -1000.0, 0x45, 0),
        (4, -100.0, 0x4e, 0),
        (5, -100.0, 0x4f, 0),
        (1, 2599.0, 0x45, 25),
        (1, 25501.0, 0x45, 255),
        (4, 1000.0, 0x4e, 177),
        (4, 1435.0, 0x4e, 255),
        (5, 717.9, 0x4f, 127),
        // The float product, not the truncated value: 5.9 gives 1 (5 would give 0).
        (5, 5.9, 0x4f, 1),
        (5, -3.0, 0x4f, 0),
        (6, 50.0, 0x50, 50),
        (8, 101.0, 0x54, 100),
        (10, -5.0, 0x58, 0),
        (11, 99.9, 0x5a, 99),
        (11, 200.0, 0x5a, 100),
        (12, 300.0, 0x5c, 255),
        (12, 12.7, 0x5c, 12),
        (12, -2.0, 0x5c, 0),
        (0, f64::NAN, 0x44, 0),
    ];
    for (t, v, at, want) in cases {
        set_trait(&mut w, 1, t, v);
        let got = match at {
            0x50..=0x5b => u32::from(w.e.mem.u16(a + at)),
            0x5c => w.e.mem.u32(a + at),
            _ => u32::from(w.e.mem.u8(a + at)),
        };
        assert_eq!(got, want, "trait {t} value {v}");
    }
    // An unknown trait changes nothing.
    let before: Vec<u8> = (0..0x60).map(|i| w.e.mem.u8(a + i)).collect();
    set_trait(&mut w, 1, 13, 5.0);
    let after: Vec<u8> = (0..0x60).map(|i| w.e.mem.u8(a + i)).collect();
    assert_eq!(before, after);
}

#[test]
fn the_frequency_and_static_attenuation_reach_the_playing_sounds() {
    let mut w = world();
    let a = w.form_a;
    set_trait(&mut w, 1, 2, 50.0);
    assert_eq!(w.e.mem.u8(a + 0x46) as i8, 50);
    let raised = 1.0 + 50.0 * f32::from_bits(0x3c23_d70a);
    for i in [0, 1, 3, 4] {
        assert_eq!(w.e.mem.f32(w.sounds[i] + 0x138).to_bits(), raised.to_bits());
    }
    assert_eq!(w.e.mem.u32(w.sounds[2] + 0x138), 0, "B's sound untouched");
    // The upper clamp: 150 is kept at 100.
    set_trait(&mut w, 1, 2, 150.0);
    assert_eq!(w.e.mem.u8(a + 0x46) as i8, 100);
    let top = 1.0 + 100.0 * f32::from_bits(0x3c23_d70a);
    assert_eq!(w.e.mem.f32(w.sounds[4] + 0x138).to_bits(), top.to_bits());
    set_trait(&mut w, 1, 2, -150.0);
    assert_eq!(w.e.mem.u8(a + 0x46) as i8, -100);
    let lowered = 1.0 + -100.0 * f32::from_bits(0x3ba3_d70a);
    assert_eq!(
        w.e.mem.f32(w.sounds[0] + 0x138).to_bits(),
        lowered.to_bits()
    );
    // With the form's flag bit 0 its factor is 1.
    w.e.mem.set_u32(a + 0x48, 1);
    set_trait(&mut w, 1, 2, 20.0);
    assert_eq!(w.e.mem.f32(w.sounds[0] + 0x138), 1.0);
    // Static attenuation: the size times 100, 10000 past 100; the sounds' 16-bit word too.
    for (v, want) in [
        (-30.0, 3000u16),
        (30.9, 3000),
        (101.0, 10000),
        (f64::NAN, 0),
    ] {
        set_trait(&mut w, 1, 3, v);
        assert_eq!(w.e.mem.u16(a + 0x4c), want, "{v}");
        assert_eq!(w.e.mem.u16(w.sounds[1] + 0x18), want);
    }
    assert_eq!(w.e.mem.u16(w.sounds[2] + 0x18), 0, "B's sound untouched");
}
