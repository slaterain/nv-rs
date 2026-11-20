//! A pad through the game's own controller code: the translated `Controls::Poll` (`00a23010`),
//! the rumble (`00a255b0`) and its stop (`00a25810`), each reaching the platform's XInput
//! (`engine_platform::xinput`) through the game's thunks and import slots, with a stand-in for the
//! system's pads. What the stand-in gives is what the game reads, byte for byte.

use engine::{args, Engine};
use engine_platform::xinput::{self, buttons, PadSource, PadState};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// The import slots of `XInputGetState` and `XInputSetState`.
const GET_STATE_SLOT: u32 = 0x00fd_f394;
const SET_STATE_SLOT: u32 = 0x00fd_f398;
/// `Controls::Poll`, `Controls::Rumble` and the rumble's stop.
const CONTROLS_POLL: u32 = 0x00a2_3010;
const CONTROLS_RUMBLE: u32 = 0x00a2_55b0;
const RUMBLE_STOP: u32 = 0x00a2_5810;
/// The game's pad state, last frame's copy and its "pad connected" byte.
const PAD_STATE: u32 = 0x011f_35a8;
const PAD_LAST: u32 = 0x011f_35b8;
const PAD_CONNECTED: u32 = 0x011f_35c8;
/// `Controls` fields: the rumble's "on" byte and its queue.
const RUMBLE_ENABLED: u32 = 0x1b88;
const RUMBLE_QUEUE: u32 = 0x1b80;
/// The memory manager's allocation (stood in here) and the strengths' scale in the exe's data.
const ALLOCATE: u32 = 0x00aa_3e40;
const STRENGTH_SCALE: u32 = 0x0102_17d0;

/// The system's pads, stood in: user 0's pad or none, and the vibrations sent.
#[derive(Clone, Default)]
struct Pads(Arc<Mutex<Seen>>);

/// User 0's pad (or none) and the vibrations sent to it (user and the four bytes).
#[derive(Default)]
struct Seen {
    pad: Option<PadState>,
    sent: Vec<(u32, [u8; 4])>,
}

impl PadSource for Pads {
    fn get_state(&mut self, user: u32, state: &mut [u8; 16]) -> u32 {
        match (user, self.0.lock().unwrap().pad) {
            (0, Some(pad)) => {
                *state = pad.to_bytes();
                xinput::ERROR_SUCCESS
            }
            _ => xinput::ERROR_DEVICE_NOT_CONNECTED,
        }
    }

    fn set_state(&mut self, user: u32, vibration: &[u8; 4]) -> u32 {
        let mut pads = self.0.lock().unwrap();
        if user == 0 && pads.pad.is_some() {
            pads.sent.push((user, *vibration));
            xinput::ERROR_SUCCESS
        } else {
            xinput::ERROR_DEVICE_NOT_CONNECTED
        }
    }
}

/// An engine with the platform's two XInput imports at their slots, the stand-in pads behind
/// them, the pad globals' page mapped and a `Controls` object with no keyboard, mouse or joystick.
fn engine(pads: &Pads) -> (Engine, u32) {
    let mut e = Engine::new();
    let table = engine::units::platform::imports();
    for (slot, ordinal) in [(GET_STATE_SLOT, "#2"), (SET_STATE_SLOT, "#3")] {
        let (_, _, f) = table
            .iter()
            .find(|(dll, name, _)| dll.eq_ignore_ascii_case("XINPUT1_3.dll") && *name == ordinal)
            .expect("the platform's XInput import");
        e.register(slot, *f);
    }
    xinput::set_source(&mut e, Some(Box::new(pads.clone())));
    e.map(0x011f_3000, 0x1000);
    let controls = e.mem.alloc(0x2000);
    e.mem.write(controls, &[0; 0x2000]);
    (e, controls)
}

fn pad(packet: u32) -> PadState {
    PadState {
        packet,
        buttons: buttons::A | buttons::DPAD_UP | buttons::START,
        left_trigger: 0x40,
        right_trigger: 0xff,
        thumb_lx: -32768,
        thumb_ly: 32767,
        thumb_rx: 1234,
        thumb_ry: -4321,
    }
}

/// Each poll keeps last frame's state and reads the pad's exactly; the pad counts as connected
/// while it answers and the game's "360 Controller" setting (`Controls` +0) is off; pulled out,
/// the game's state keeps the last reading and the pad is not connected; plugged in again, it is.
#[test]
fn controls_poll_reads_the_pad_as_the_system_gives_it() {
    let pads = Pads::default();
    let (mut e, controls) = engine(&pads);
    e.mem.write(PAD_STATE, &[0x55; 16]);
    pads.0.lock().unwrap().pad = Some(pad(1));
    e.call(CONTROLS_POLL, &args![controls]);
    assert_eq!(e.mem.bytes(PAD_STATE, 16), pad(1).to_bytes().to_vec());
    assert_eq!(e.mem.bytes(PAD_LAST, 16), vec![0x55; 16]);
    assert_eq!(e.mem.u8(PAD_CONNECTED), 1);

    pads.0.lock().unwrap().pad = Some(pad(2));
    e.call(CONTROLS_POLL, &args![controls]);
    assert_eq!(e.mem.bytes(PAD_STATE, 16), pad(2).to_bytes().to_vec());
    assert_eq!(e.mem.bytes(PAD_LAST, 16), pad(1).to_bytes().to_vec());

    // The game's own switch: the pad is read but not used.
    e.mem.set_u8(controls, 1);
    e.call(CONTROLS_POLL, &args![controls]);
    assert_eq!(e.mem.u8(PAD_CONNECTED), 0);
    e.mem.set_u8(controls, 0);

    pads.0.lock().unwrap().pad = None;
    e.call(CONTROLS_POLL, &args![controls]);
    assert_eq!(e.mem.u8(PAD_CONNECTED), 0);
    assert_eq!(e.mem.bytes(PAD_STATE, 16), pad(2).to_bytes().to_vec());

    pads.0.lock().unwrap().pad = Some(pad(3));
    e.call(CONTROLS_POLL, &args![controls]);
    assert_eq!(e.mem.u8(PAD_CONNECTED), 1);
    assert_eq!(e.mem.bytes(PAD_STATE, 16), pad(3).to_bytes().to_vec());
}

/// The rumble reaches the pad with the game's motor speeds (left 0.5, right 0.25 of 65535:
/// 32767 and 16383, truncated), and its stop sends both motors 0, each to user 0.
#[test]
fn the_games_rumble_and_its_stop_reach_the_pad() {
    let pads = Pads::default();
    let (mut e, controls) = engine(&pads);
    pads.0.lock().unwrap().pad = Some(pad(1));
    e.mem.set_u8(controls + RUMBLE_ENABLED, 1);
    let queue = e.mem.alloc(0x40);
    e.mem.write(queue, &[0; 0x40]);
    e.mem.set_u32(controls + RUMBLE_QUEUE, queue);
    e.map(STRENGTH_SCALE & !0xfff, 0x1000);
    e.mem.set_u64(STRENGTH_SCALE, 65535.0f64.to_bits());
    // The allocation answers this block (`register` takes a plain function: the block's address
    // goes through a static).
    static EFFECT: AtomicU32 = AtomicU32::new(0);
    fn allocate(_: &mut Engine, _: &[u32]) -> engine::Ret {
        engine::Ret {
            eax: EFFECT.load(Ordering::Relaxed),
            ..engine::Ret::default()
        }
    }
    EFFECT.store(e.mem.alloc(0x40), Ordering::Relaxed);
    e.register(ALLOCATE, allocate);
    e.call(
        CONTROLS_RUMBLE,
        &args![
            controls,
            0.5f32.to_bits(),
            0.25f32.to_bits(),
            0u32,
            0u32,
            0u32,
            0u32,
            1u32
        ],
    );
    e.call(RUMBLE_STOP, &args![controls]);
    assert_eq!(
        pads.0.lock().unwrap().sent,
        vec![(0, [0xff, 0x7f, 0xff, 0x3f]), (0, [0, 0, 0, 0])]
    );
}
