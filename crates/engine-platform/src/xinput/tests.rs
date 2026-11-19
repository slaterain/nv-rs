//! Tests of the platform's XInput with a stand-in source: what the game's two calls answer and
//! write, the measured argument checks, a pad that goes and comes back, rumble, and the throttle.
//! (Controls::Poll reading through it, translated: `crates/engine/tests/xinput_pad.rs`.)

use super::*;
use std::sync::{Arc, Mutex};

/// A stand-in for the system's pads: each user's pad (or none), and what was asked.
#[derive(Clone, Default)]
pub(crate) struct Fake(pub(crate) Arc<Mutex<FakePads>>);

#[derive(Default)]
pub(crate) struct FakePads {
    pub(crate) pads: [Option<PadState>; 4],
    /// Every call: `('g', user)` or `('s', user)`.
    pub(crate) calls: Vec<(char, u32)>,
    /// The vibrations a connected pad was sent.
    pub(crate) vibrations: Vec<(u32, [u8; 4])>,
}

impl PadSource for Fake {
    fn get_state(&mut self, user: u32, state: &mut [u8; 16]) -> u32 {
        let mut f = self.0.lock().unwrap();
        f.calls.push(('g', user));
        match f.pads[user as usize] {
            Some(pad) => {
                *state = pad.to_bytes();
                ERROR_SUCCESS
            }
            None => ERROR_DEVICE_NOT_CONNECTED,
        }
    }

    fn set_state(&mut self, user: u32, vibration: &[u8; 4]) -> u32 {
        let mut f = self.0.lock().unwrap();
        f.calls.push(('s', user));
        if f.pads[user as usize].is_some() {
            f.vibrations.push((user, *vibration));
            ERROR_SUCCESS
        } else {
            ERROR_DEVICE_NOT_CONNECTED
        }
    }
}

fn pad() -> PadState {
    PadState {
        packet: 0x0102_0304,
        buttons: buttons::A | buttons::DPAD_LEFT | buttons::RIGHT_SHOULDER,
        left_trigger: 0x80,
        right_trigger: 0xff,
        thumb_lx: -32768,
        thumb_ly: 32767,
        thumb_rx: -1,
        thumb_ry: 0x1234,
    }
}

/// An engine with a 16-byte state and a 4-byte vibration in its memory, each filled with `0xaa`.
fn engine() -> (Engine, u32, u32) {
    let mut e = Engine::new();
    let state = e.mem.alloc(16);
    e.mem.write(state, &[0xaa; 16]);
    let vibration = e.mem.alloc(4);
    e.mem.write(vibration, &[0xaa; 4]);
    (e, state, vibration)
}

/// `XINPUT_STATE`'s 16 bytes (XInput.h): the packet number, the buttons, the two trigger bytes,
/// the four stick words, little-endian; and back.
#[test]
fn the_state_is_xinputs_16_bytes() {
    let bytes = pad().to_bytes();
    assert_eq!(
        bytes,
        [
            0x04, 0x03, 0x02, 0x01, 0x04, 0x12, 0x80, 0xff, 0x00, 0x80, 0xff, 0x7f, 0xff, 0xff,
            0x34, 0x12
        ]
    );
    assert_eq!(PadState::from_bytes(&bytes), pad());
    assert_eq!(PadState::default().to_bytes(), [0; 16]);
}

/// XInput.h's button bits, the guide button (0x0400, `XInputGetStateEx` only) and 0x0800 left out.
#[test]
fn the_button_bits_are_xinputs() {
    use buttons::*;
    let all = [
        DPAD_UP,
        DPAD_DOWN,
        DPAD_LEFT,
        DPAD_RIGHT,
        START,
        BACK,
        LEFT_THUMB,
        RIGHT_THUMB,
        LEFT_SHOULDER,
        RIGHT_SHOULDER,
        A,
        B,
        X,
        Y,
    ];
    let mut mask = 0u16;
    for (i, bit) in all.iter().enumerate() {
        assert_eq!(bit.count_ones(), 1);
        assert_eq!(mask & bit, 0, "bit {i} twice");
        mask |= bit;
    }
    assert_eq!(mask, 0xf3ff);
    // Each against XInput.h's own value (a swap of two keeps the mask).
    let xinput_h = [
        (DPAD_UP, 0x0001),
        (DPAD_DOWN, 0x0002),
        (DPAD_LEFT, 0x0004),
        (DPAD_RIGHT, 0x0008),
        (START, 0x0010),
        (BACK, 0x0020),
        (LEFT_THUMB, 0x0040),
        (RIGHT_THUMB, 0x0080),
        (LEFT_SHOULDER, 0x0100),
        (RIGHT_SHOULDER, 0x0200),
        (A, 0x1000),
        (B, 0x2000),
        (X, 0x4000),
        (Y, 0x8000),
    ];
    for (i, (bit, value)) in xinput_h.iter().enumerate() {
        assert_eq!(bit, value, "button {i}");
    }
}

/// The answers are Windows' own codes (winerror.h, as measured on this PC) and its four users.
#[test]
fn the_answers_are_windows_codes() {
    assert_eq!(
        (
            ERROR_SUCCESS,
            ERROR_BAD_ARGUMENTS,
            ERROR_DEVICE_NOT_CONNECTED,
            USERS
        ),
        (0, 0xa0, 0x48f, 4)
    );
}

/// No source: every user 0 to 3 is "not connected" to both calls and the game's memory is left
/// as it was; a user index of 4 or more, or a null pointer, is `ERROR_BAD_ARGUMENTS` (measured).
#[test]
fn without_a_source_no_pad_answers() {
    let (mut e, state, vibration) = engine();
    for user in 0..USERS {
        assert_eq!(
            xinput_get_state(&mut e, user, Ptr::new(state)),
            ERROR_DEVICE_NOT_CONNECTED
        );
        assert_eq!(
            xinput_set_state(&mut e, user, Ptr::new(vibration)),
            ERROR_DEVICE_NOT_CONNECTED
        );
    }
    for user in [USERS, 5, 0xffff_ffff] {
        assert_eq!(
            xinput_get_state(&mut e, user, Ptr::new(state)),
            ERROR_BAD_ARGUMENTS
        );
        assert_eq!(
            xinput_set_state(&mut e, user, Ptr::new(vibration)),
            ERROR_BAD_ARGUMENTS
        );
    }
    assert_eq!(
        xinput_get_state(&mut e, 0, Ptr::new(0)),
        ERROR_BAD_ARGUMENTS
    );
    assert_eq!(
        xinput_set_state(&mut e, 0, Ptr::new(0)),
        ERROR_BAD_ARGUMENTS
    );
    assert_eq!(e.mem.bytes(state, 16), vec![0xaa; 16]);
    assert_eq!(e.mem.bytes(vibration, 4), vec![0xaa; 4]);
}

/// A source: the game's state gets exactly the pad's 16 bytes; a user with no pad leaves them;
/// the argument checks come before the source, which is never asked for them.
#[test]
fn the_game_reads_exactly_what_the_source_gives() {
    let (mut e, state, _) = engine();
    let fake = Fake::default();
    fake.0.lock().unwrap().pads[2] = Some(pad());
    set_source(&mut e, Some(Box::new(fake.clone())));
    assert_eq!(
        xinput_get_state(&mut e, 0, Ptr::new(state)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    assert_eq!(e.mem.bytes(state, 16), vec![0xaa; 16]);
    assert_eq!(xinput_get_state(&mut e, 2, Ptr::new(state)), ERROR_SUCCESS);
    assert_eq!(e.mem.bytes(state, 16), pad().to_bytes().to_vec());
    assert_eq!(
        xinput_get_state(&mut e, 4, Ptr::new(state)),
        ERROR_BAD_ARGUMENTS
    );
    assert_eq!(
        xinput_get_state(&mut e, 2, Ptr::new(0)),
        ERROR_BAD_ARGUMENTS
    );
    assert_eq!(fake.0.lock().unwrap().calls, vec![('g', 0), ('g', 2)]);
}

/// A pad pulled out answers "not connected" and leaves the last state it wrote; plugged in again,
/// its new state is read.
#[test]
fn a_pad_pulled_out_and_plugged_in_again() {
    let (mut e, state, _) = engine();
    let fake = Fake::default();
    fake.0.lock().unwrap().pads[0] = Some(pad());
    set_source(&mut e, Some(Box::new(fake.clone())));
    assert_eq!(xinput_get_state(&mut e, 0, Ptr::new(state)), ERROR_SUCCESS);
    fake.0.lock().unwrap().pads[0] = None;
    assert_eq!(
        xinput_get_state(&mut e, 0, Ptr::new(state)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    assert_eq!(e.mem.bytes(state, 16), pad().to_bytes().to_vec());
    let again = PadState {
        packet: 7,
        buttons: buttons::B,
        ..PadState::default()
    };
    fake.0.lock().unwrap().pads[0] = Some(again);
    assert_eq!(xinput_get_state(&mut e, 0, Ptr::new(state)), ERROR_SUCCESS);
    assert_eq!(e.mem.bytes(state, 16), again.to_bytes().to_vec());
}

/// Rumble: the game's 4 vibration bytes reach the pad unchanged, with the user; a user with no
/// pad answers "not connected"; the game's memory is not written.
#[test]
fn rumble_reaches_the_pad() {
    let (mut e, _, vibration) = engine();
    e.mem.write(vibration, &[0xff, 0x7f, 0x01, 0x80]);
    let fake = Fake::default();
    fake.0.lock().unwrap().pads[1] = Some(pad());
    set_source(&mut e, Some(Box::new(fake.clone())));
    assert_eq!(
        xinput_set_state(&mut e, 1, Ptr::new(vibration)),
        ERROR_SUCCESS
    );
    assert_eq!(
        xinput_set_state(&mut e, 0, Ptr::new(vibration)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    assert_eq!(e.mem.bytes(vibration, 4), vec![0xff, 0x7f, 0x01, 0x80]);
    // The stop, both motors 0, reaches the pad as any other speeds do.
    e.mem.write(vibration, &[0; 4]);
    assert_eq!(
        xinput_set_state(&mut e, 1, Ptr::new(vibration)),
        ERROR_SUCCESS
    );
    assert_eq!(
        fake.0.lock().unwrap().vibrations,
        vec![(1, [0xff, 0x7f, 0x01, 0x80]), (1, [0; 4])]
    );
    // Taking the source away is "no pad" again.
    set_source(&mut e, None);
    assert_eq!(
        xinput_set_state(&mut e, 1, Ptr::new(vibration)),
        ERROR_DEVICE_NOT_CONNECTED
    );
}

/// The throttle: once a slot that was not connected answers "not connected" the system is not
/// asked for it again for [`EMPTY_RECHECK`] (both calls answer "not connected", the state kept),
/// then is; a pad that answers is asked every time, and pulled out it is asked once more before
/// it is held back; other users are not held back by one empty slot.
#[test]
fn an_empty_slot_is_asked_again_once_a_second() {
    let fake = Fake::default();
    let mut source = Throttled::new(fake.clone());
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    let mut state = [0xaa; 16];
    let asked = |fake: &Fake| fake.0.lock().unwrap().calls.len();
    assert_eq!(
        source.get_state_at(0, &mut state, at(0)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    assert_eq!(asked(&fake), 1);
    // A pad plugged in at 10 ms is seen once the second is over, not before.
    fake.0.lock().unwrap().pads[0] = Some(pad());
    for ms in [10, 500, 999] {
        assert_eq!(
            source.get_state_at(0, &mut state, at(ms)),
            ERROR_DEVICE_NOT_CONNECTED
        );
        assert_eq!(
            source.set_state_at(0, &[0; 4], at(ms)),
            ERROR_DEVICE_NOT_CONNECTED
        );
    }
    assert_eq!(state, [0xaa; 16]);
    assert_eq!(asked(&fake), 1);
    // User 1 is asked although user 0 is held back.
    assert_eq!(
        source.get_state_at(1, &mut state, at(999)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    assert_eq!(asked(&fake), 2);
    assert_eq!(source.get_state_at(0, &mut state, at(1000)), ERROR_SUCCESS);
    assert_eq!(state, pad().to_bytes());
    // Connected: asked every time.
    for ms in [1000, 1000, 1001] {
        assert_eq!(source.get_state_at(0, &mut state, at(ms)), ERROR_SUCCESS);
        assert_eq!(source.set_state_at(0, &[1, 2, 3, 4], at(ms)), ERROR_SUCCESS);
    }
    assert_eq!(asked(&fake), 9);
    // Pulled out: a rumble's "not connected" holds nothing back; the first read's is asked again,
    // the second's holds the slot back from then on.
    fake.0.lock().unwrap().pads[0] = None;
    assert_eq!(
        source.set_state_at(0, &[0; 4], at(2000)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    for ms in [2001, 2002] {
        assert_eq!(
            source.get_state_at(0, &mut state, at(ms)),
            ERROR_DEVICE_NOT_CONNECTED
        );
    }
    assert_eq!(asked(&fake), 12);
    assert_eq!(
        source.get_state_at(0, &mut state, at(3001)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    assert_eq!(asked(&fake), 12);
    assert_eq!(
        source.get_state_at(0, &mut state, at(3002)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    assert_eq!(asked(&fake), 13);
}

/// A connected pad that misses one answer is read at the very next call (review of #781: a
/// second without the pad is felt), and a failed rumble holds no slot back.
#[test]
fn a_pad_that_misses_one_answer_is_read_at_the_next_call() {
    let fake = Fake::default();
    fake.0.lock().unwrap().pads[0] = Some(pad());
    let mut source = Throttled::new(fake.clone());
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    let mut state = [0xaa; 16];
    let plug = |on: bool| fake.0.lock().unwrap().pads[0] = on.then(pad);
    assert_eq!(source.get_state_at(0, &mut state, at(0)), ERROR_SUCCESS);
    plug(false);
    assert_eq!(
        source.get_state_at(0, &mut state, at(1)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    plug(true);
    state = [0xaa; 16];
    assert_eq!(source.get_state_at(0, &mut state, at(2)), ERROR_SUCCESS);
    assert_eq!(state, pad().to_bytes());
    plug(false);
    assert_eq!(
        source.set_state_at(0, &[1, 2, 3, 4], at(3)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    plug(true);
    assert_eq!(source.get_state_at(0, &mut state, at(4)), ERROR_SUCCESS);
    assert_eq!(source.set_state_at(0, &[1, 2, 3, 4], at(5)), ERROR_SUCCESS);
    assert_eq!(fake.0.lock().unwrap().calls.len(), 6);
}

/// The system's XInput goes in behind the throttle (`install_system`, which the Windows start
/// uses): three quick reads and three rumbles of an empty slot ask the system once.
#[test]
fn the_system_goes_in_behind_the_throttle() {
    let (mut e, state, vibration) = engine();
    let fake = Fake::default();
    let line = install_system(&mut e, Some((fake.clone(), "xinput1_4.dll")));
    assert!(line.contains("xinput1_4.dll"), "{line}");
    for _ in 0..3 {
        assert_eq!(
            xinput_get_state(&mut e, 0, Ptr::new(state)),
            ERROR_DEVICE_NOT_CONNECTED
        );
        assert_eq!(
            xinput_set_state(&mut e, 0, Ptr::new(vibration)),
            ERROR_DEVICE_NOT_CONNECTED
        );
    }
    assert_eq!(fake.0.lock().unwrap().calls, vec![('g', 0)]);
    let none = install_system::<Fake>(&mut e, None);
    assert!(none.contains("no XInput DLL"), "{none}");
    assert_eq!(
        xinput_get_state(&mut e, 0, Ptr::new(state)),
        ERROR_DEVICE_NOT_CONNECTED
    );
    assert_eq!(fake.0.lock().unwrap().calls.len(), 1);
}

/// The throttle through the import functions: the system's clock, so only "not asked again at
/// once" is checked here.
#[test]
fn the_imports_use_the_throttle_as_given() {
    let (mut e, state, _) = engine();
    let fake = Fake::default();
    set_source(&mut e, Some(Box::new(Throttled::new(fake.clone()))));
    for _ in 0..3 {
        assert_eq!(
            xinput_get_state(&mut e, 0, Ptr::new(state)),
            ERROR_DEVICE_NOT_CONNECTED
        );
    }
    assert_eq!(fake.0.lock().unwrap().calls, vec![('g', 0)]);
}

/// The two imports are the platform layer's, under the game's DLL and ordinals, once each.
#[test]
fn the_imports_are_the_layers() {
    let table = crate::imports();
    for name in ["#2", "#3"] {
        let found: Vec<_> = table
            .iter()
            .filter(|(dll, n, _)| dll.eq_ignore_ascii_case("XINPUT1_3.dll") && *n == name)
            .collect();
        assert_eq!(found.len(), 1, "{name}");
    }
    assert!(crate::layer_imports()
        .iter()
        .any(|(dll, n, _)| *dll == "XINPUT1_3.dll" && *n == "#2"));
}
