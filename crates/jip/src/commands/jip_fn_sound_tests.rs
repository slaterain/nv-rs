//! Tests of `jip_fn_sound`: a sound form at the exe's offsets (data from +0x44, flags +0x48, the
//! file's path +0x34 set through the component's vtable +0x10, a stand-in here), `PlayingMusic`
//! (`011DD0F0`) and the audio manager's volumes (`011F6EF0` +0x140, defaults at `011F6E18`).

use super::*;
use crate::commands::test_support::*;

const SOUND: u32 = FORMS + 0x1000;
const MUSIC: u32 = FORMS + 0x1100;
const MISC: u32 = FORMS + 0x1200;
const MLC: u32 = FORMS + 0x1300;
const FILE_VT: u32 = FORMS + 0x2000;
const SET_FN: u32 = 0x0090_0300;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    e.map(0x011f_6000, 0x2000);
    form(&mut e, SOUND, 0x0d, 0x0000_0100);
    form(&mut e, MUSIC, 0x66, 0x0000_0101);
    form(&mut e, MISC, 0x1f, 0x0000_0102);
    form(&mut e, MLC, 0x70, 0x0000_0103);
    e.mem.set_u32(SOUND + 0x30, FILE_VT);
    e.mem.set_u32(FILE_VT + 0x10, SET_FN);
    // The component's `Set`: the text into its path, as a game string of its own.
    e.register(SET_FN, |e, w| {
        let t = nvse::table::guest_text(e, w[1]);
        let block = FORMS + 0x3000;
        e.mem.write(block, &t);
        e.mem.set_u8(block + t.len() as u32, 0);
        e.mem.set_u32(w[0] + 4, block);
        e.mem.set_u16(w[0] + 8, t.len() as u16);
        Ret::default()
    });
    for (i, f) in [SOUND, MUSIC, MISC, MLC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_numbers() {
    let mut e = setup();
    e.mem.set_u8(SOUND + 0x44, 10);
    e.mem.set_u8(SOUND + 0x45, 3);
    e.mem.set_u8(SOUND + 0x46, 0xf6);
    e.mem.set_u16(SOUND + 0x4c, 250);
    e.mem.set_u8(SOUND + 0x4e, 51);
    e.mem.set_u8(SOUND + 0x4f, 255);
    for i in 0..5 {
        e.mem.set_u16(SOUND + 0x50 + 2 * i, 10 * (i as u16 + 1));
    }
    e.mem.set_u16(SOUND + 0x5a, 77);
    e.mem.set_u32(SOUND + 0x5c, 0x8000_0000);
    let t = |e: &mut Engine, n_: i32| call(e, 0x2646, &[r(1), n(n_)]).1;
    assert_eq!(t(&mut e, 0), 50.0);
    assert_eq!(t(&mut e, 1), 300.0);
    assert_eq!(t(&mut e, 2), -10.0, "a signed byte");
    assert_eq!(t(&mut e, 3), 250.0 * -0.01);
    assert_eq!(
        t(&mut e, 4),
        f64::from(51.0f32 * (1434.0f32 / 255.0)),
        "a float product"
    );
    assert_eq!(t(&mut e, 5), f64::from(255.0f32 * (1434.0f32 / 255.0)));
    assert_eq!(t(&mut e, 6), 10.0);
    assert_eq!(t(&mut e, 10), 50.0);
    assert_eq!(t(&mut e, 11), 77.0);
    assert_eq!(t(&mut e, 12), f64::from(i32::MIN), "as an int");
    assert_eq!(t(&mut e, 13), 0.0);
    // Not a sound: refused by the reader.
    assert_eq!(call(&mut e, 0x2646, &[r(3), n(0)]).1, 0.0);
}

#[test]
fn the_flags() {
    let mut e = setup();
    call(&mut e, 0x2649, &[r(1), n(4), n(1)]);
    call(&mut e, 0x2649, &[r(1), n(31), n(1)]);
    assert_eq!(e.mem.u32(SOUND + 0x48), 0x8000_0010);
    assert_eq!(call(&mut e, 0x2648, &[r(1), n(31)]).1, 1.0);
    call(&mut e, 0x2649, &[r(1), n(4), n(0)]);
    assert_eq!(e.mem.u32(SOUND + 0x48), 0x8000_0000);
    call(&mut e, 0x2649, &[r(1), n(32), n(1)]);
    assert_eq!(e.mem.u32(SOUND + 0x48), 0x8000_0000, "flag 32 refused");
    e.mem.set_u32(SOUND + 0x48, u32::MAX);
    assert_eq!(call(&mut e, 0x2648, &[r(1), n(32)]).1, 0.0);
}

#[test]
fn the_file() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x26bf, &[r(1)]);
    assert_eq!(string_result(&e, res), b"");
    call(&mut e, 0x26c0, &[r(1), s(b"fx\\boom.wav")]);
    assert_eq!(text_at(&e, SOUND + 0x34), b"fx\\boom.wav");
    let (_, res) = call(&mut e, 0x26bf, &[r(1)]);
    assert_eq!(string_result(&e, res), b"fx\\boom.wav");
}

const PM: u32 = 0x011d_d0f0;

#[test]
fn the_music() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2862, &[]).1, 0.0, "nothing playing");
    // Track 2 active (track1Active 0), playing.
    e.mem.set_u8(PM + 0x221, 0x10);
    assert_eq!(call(&mut e, 0x2862, &[]).1, 2.0);
    e.mem.set_u8(PM + 0x221, 0x14);
    assert_eq!(call(&mut e, 0x2862, &[]).1, 1.0, "paused");
    // The location controller playing.
    e.mem.set_u32(PM + 0x284, MLC);
    assert_eq!(call(&mut e, 0x2862, &[r(4)]).1, 1.0);
    // A music type whose path begins the track's path (case blind).
    e.mem.write(PM + 0x108, b"Data\\Music\\Dungeon\\a.mp3\0");
    let text = FORMS + 0x3100;
    e.mem.write(text, b"data\\music\\dungeon\0");
    e.mem.set_u32(MUSIC + 0x1c, text);
    e.mem.set_u16(MUSIC + 0x20, 18);
    assert_eq!(call(&mut e, 0x2862, &[r(2)]).1, 1.0);
    e.mem.set_u16(MUSIC + 0x20, 0);
    assert_eq!(call(&mut e, 0x2862, &[r(2)]).1, 0.0, "an empty path");
    assert_eq!(call(&mut e, 0x2862, &[r(3)]).1, 0.0, "another form");
    // Track 1 active: its flags (not playing).
    e.mem.set_u32(PM + 0x27c, 1);
    assert_eq!(call(&mut e, 0x2862, &[]).1, 0.0);
}

#[test]
fn the_music_state() {
    let mut e = setup();
    call(&mut e, 0x2863, &[n(1)]);
    assert_eq!(e.mem.u8(PM + 0x221), 0, "not playing: nothing");
    e.mem.set_u8(PM + 0x221, 0x30);
    call(&mut e, 0x2863, &[n(1)]);
    assert_eq!(e.mem.u8(PM + 0x221), 0x34);
    call(&mut e, 0x2863, &[n(2)]);
    assert_eq!(e.mem.u8(PM + 0x221), 0x30);
    call(&mut e, 0x2863, &[n(0)]);
    assert_eq!(e.mem.u8(PM + 0x221), 0x18, "stop and play");
    call(&mut e, 0x2863, &[n(3)]);
    assert_eq!(e.mem.u8(PM + 0x221), 0x18);
    e.mem.set_u32(PM + 0x27c, 1);
    e.mem.set_u8(PM + 0x220, 0x10);
    call(&mut e, 0x2863, &[n(1)]);
    assert_eq!(e.mem.u8(PM + 0x220), 0x14, "track 1");
}

#[test]
fn the_volumes() {
    let mut e = setup();
    let am = 0x011f_6ef0 + 0x140;
    e.mem.set_f32(am + 4 * 3, 0.7);
    assert_eq!(call(&mut e, 0x288c, &[n(3)]).1, f64::from(0.7f32 * 100.0));
    assert_eq!(call(&mut e, 0x288c, &[n(6)]).1, 0.0);
    call(&mut e, 0x288d, &[n(4), n(35)]);
    assert_eq!(e.mem.f32(am + 16), 35.0f32 * 0.01);
    call(&mut e, 0x288d, &[n(4), n(101)]);
    assert_eq!(e.mem.f32(am + 16), 35.0f32 * 0.01, "above 100: nothing");
    e.mem.set_f32(0x011f_6e18 + 4 * 0xc, 0.9);
    call(&mut e, 0x288d, &[n(4)]);
    assert_eq!(e.mem.f32(am + 16), 0.9, "no level: the default");
    call(&mut e, 0x288d, &[n(6), n(10)]);
    assert_eq!(e.mem.f32(am + 24), 0.0);
}

/// The last volume type (5) is read and set; a level of exactly 100 sets 1.0.
#[test]
fn the_volume_boundaries() {
    let mut e = setup();
    let am = 0x011f_6ef0 + 0x140;
    call(&mut e, 0x288d, &[n(5), n(100)]);
    assert_eq!(e.mem.f32(am + 20), 100.0f32 * 0.01);
    assert_eq!(e.mem.f32(am + 20), 1.0);
    assert_eq!(call(&mut e, 0x288c, &[n(5)]).1, f64::from(1.0f32 * 100.0));
    call(&mut e, 0x288d, &[n(0), n(0)]);
    assert_eq!(e.mem.f32(am), 0.0);
}

/// A music type whose path the playing track's does not begin with: 0. With track 1 active, its
/// path (at +0) is the one compared, not track 2's.
#[test]
fn the_music_types_path() {
    let mut e = setup();
    let text = FORMS + 0x3100;
    e.mem.write(text, b"data\\music\\explore\0");
    e.mem.set_u32(MUSIC + 0x1c, text);
    e.mem.set_u16(MUSIC + 0x20, 18);
    e.mem.set_u8(PM + 0x221, 0x10);
    e.mem.write(PM + 0x108, b"Data\\Music\\Dungeon\\a.mp3\0");
    assert_eq!(call(&mut e, 0x2862, &[r(2)]).1, 0.0, "another path");
    e.mem.set_u32(PM + 0x27c, 1);
    e.mem.set_u8(PM + 0x220, 0x10);
    e.mem.write(PM, b"Data\\Music\\Explore\\b.mp3\0");
    assert_eq!(call(&mut e, 0x2862, &[r(2)]).1, 2.0, "track 1's path");
    e.mem.write(PM, b"Data\\Music\\Battle\\c.mp3\0");
    e.mem.write(PM + 0x108, b"Data\\Music\\Explore\\b.mp3\0");
    assert_eq!(call(&mut e, 0x2862, &[r(2)]).1, 0.0, "not track 2's");
}

/// A path of 128 bytes or more runs past JIP's 128-byte buffer: the port stops there, unless the
/// list guard row is on, which sets it whole.
#[test]
#[should_panic(expected = "runs past JIP's 128-byte buffer")]
fn a_path_longer_than_the_buffer_stops() {
    let mut e = setup();
    call(&mut e, 0x26c0, &[r(1), s(&[b'a'; 128])]);
}

#[test]
fn a_long_path_with_the_list_guard() {
    let mut e = setup();
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::LIST_GUARD, true);
    call(&mut e, 0x26c0, &[r(1), s(&[b'a'; 128])]);
    assert_eq!(text_at(&e, SOUND + 0x34), vec![b'a'; 128]);
    let path = [b'b'; 127];
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::LIST_GUARD, false);
    call(&mut e, 0x26c0, &[r(1), s(&path)]);
    assert_eq!(text_at(&e, SOUND + 0x34), path.to_vec(), "127 fits");
}
