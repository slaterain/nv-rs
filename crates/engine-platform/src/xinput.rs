//! The platform's XInput: the two `XINPUT1_3.dll` functions the game imports, answered from the
//! system's own pads ([`PadSource`]), the same way on every system.
//!
//! What the game calls (FalloutNV.exe 1.4.0.525): `XInputGetState` (ordinal 2, slot `00fdf394`,
//! thunk `009f996e`) for user 0 from `Controls::Poll` (`00a23010`, every frame),
//! `InterfaceManager::Idle` (`0070c4a0`, every frame: its message when the pad comes or goes),
//! `InterfaceManager::Initialize` (`00709fd0`) and the settings list (`007d4399`); and
//! `XInputSetState` (ordinal 3, slot `00fdf398`, thunk `00ee820a`) from `Controls::Rumble`
//! (`00a255b0`), the rumble queue's update (`00a269f0`) and its stop (`00a25810`). The game's own
//! switches stay the only ones: the Settings menu's "360 Controller" (`bDisable360Controller` in
//! `[Interface]`, which `Controls::Poll` reads as "no pad" when set) and its vibration
//! (`bGamePadRumble` in `[Controls]`).
//!
//! Without a source (the engine crates' tests, a repeatable run) every user answers "not
//! connected", as a Windows machine with no pad does. The host puts the system's pads in
//! ([`use_system_pads`]): on Windows the system's own XInput DLL, its answers passed through, and
//! after its pads the PlayStation pads read as HID devices ([`hid`], [`sony`]) and the other pads
//! that SDL's controller database has a line for ([`sdl_db`], [`dinput_order`]), each turned into
//! XInput's state by position ([`layout`]); the game's four users run over all of them ([`Chain`]).
//! On Linux (the Steam Deck among them) the pads are read from the kernel's event devices, Steam's
//! virtual pad first when Steam started the game (`linux.rs`, [`evdev`]).
//!
//! Measured on Windows 11 (`xinput1_3.dll`, `xinput1_4.dll`, `xinput9_1_0.dll`, 64-bit, no pad,
//! 2026-11-18): a user with no pad answers `ERROR_DEVICE_NOT_CONNECTED` to both calls and the
//! state is left as it was; a user index of 4 or more, or a null state or vibration, answers
//! `ERROR_BAD_ARGUMENTS`, for every user. One `XInputGetState` for an empty slot takes about 30 us
//! (`xinput1_3`) or 12 us (`xinput1_4`); the game asks twice a frame, so an empty slot is asked of
//! the system at most once a second ([`Throttled`]): a pad plugged in is seen within a second.

use crate::{Engine, Ptr};
use std::time::{Duration, Instant};

#[cfg(windows)]
mod win32;

pub mod chain;
pub mod dinput_order;
pub mod evdev;
pub mod hid;
#[cfg(windows)]
mod hid_win;
pub mod layout;
#[cfg(target_os = "linux")]
mod linux;
pub mod sdl_db;
pub mod sdl_hints;
pub mod sony;

pub use chain::{Chain, NoXInput, PadList};

/// Result codes (winerror.h): `ERROR_SUCCESS`, `ERROR_BAD_ARGUMENTS`, `ERROR_DEVICE_NOT_CONNECTED`.
pub const ERROR_SUCCESS: u32 = 0;
pub const ERROR_BAD_ARGUMENTS: u32 = 0xa0;
pub const ERROR_DEVICE_NOT_CONNECTED: u32 = 0x48f;
/// `XUSER_MAX_COUNT`: users 0 to 3.
pub const USERS: u32 = 4;

/// `XINPUT_GAMEPAD`'s `wButtons` bits (XInput.h). The guide button is not among them: the game's
/// `XInputGetState` (ordinal 2) never reports it.
pub mod buttons {
    pub const DPAD_UP: u16 = 0x0001;
    pub const DPAD_DOWN: u16 = 0x0002;
    pub const DPAD_LEFT: u16 = 0x0004;
    pub const DPAD_RIGHT: u16 = 0x0008;
    pub const START: u16 = 0x0010;
    pub const BACK: u16 = 0x0020;
    pub const LEFT_THUMB: u16 = 0x0040;
    pub const RIGHT_THUMB: u16 = 0x0080;
    pub const LEFT_SHOULDER: u16 = 0x0100;
    pub const RIGHT_SHOULDER: u16 = 0x0200;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
}

/// One pad's `XINPUT_STATE` (XInput.h, 16 bytes): the packet number (`dwPacketNumber`, changed
/// whenever the state changes), then `XINPUT_GAMEPAD`: the buttons, the two triggers (0 to 255) and
/// the two sticks (-32768 to 32767 each way, up and right positive).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    pub packet: u32,
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    pub thumb_rx: i16,
    pub thumb_ry: i16,
}

impl PadState {
    /// The 16 bytes as the game reads them.
    pub fn to_bytes(self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[0..4].copy_from_slice(&self.packet.to_le_bytes());
        b[4..6].copy_from_slice(&self.buttons.to_le_bytes());
        b[6] = self.left_trigger;
        b[7] = self.right_trigger;
        b[8..10].copy_from_slice(&self.thumb_lx.to_le_bytes());
        b[10..12].copy_from_slice(&self.thumb_ly.to_le_bytes());
        b[12..14].copy_from_slice(&self.thumb_rx.to_le_bytes());
        b[14..16].copy_from_slice(&self.thumb_ry.to_le_bytes());
        b
    }

    /// The state in the game's 16 bytes.
    pub fn from_bytes(b: &[u8; 16]) -> PadState {
        let i16_at = |at: usize| i16::from_le_bytes([b[at], b[at + 1]]);
        PadState {
            packet: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            buttons: u16::from_le_bytes([b[4], b[5]]),
            left_trigger: b[6],
            right_trigger: b[7],
            thumb_lx: i16_at(8),
            thumb_ly: i16_at(10),
            thumb_rx: i16_at(12),
            thumb_ry: i16_at(14),
        }
    }
}

/// The system's pads, as XInput answers for them. The import functions check the arguments
/// first (a user index of 4 or more, a null pointer: `ERROR_BAD_ARGUMENTS`), so a source is asked
/// only for users 0 to 3.
pub trait PadSource: Send {
    /// `XInputGetState(user, state)`: the result; a connected pad's state written into `state`
    /// (`ERROR_SUCCESS`), `state` left as it was otherwise.
    fn get_state(&mut self, user: u32, state: &mut [u8; 16]) -> u32;
    /// `XInputSetState(user, vibration)`: `vibration` is `XINPUT_VIBRATION`, the left (low
    /// frequency) and right (high frequency) motor speeds, 0 to 65535 each, little-endian.
    fn set_state(&mut self, user: u32, vibration: &[u8; 4]) -> u32;
}

/// The pads behind the game's XInput in one engine (`State::pads`): none, or a source.
#[derive(Default)]
pub struct Pads {
    source: Option<Box<dyn PadSource>>,
}

impl std::fmt::Debug for Pads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pads")
            .field("source", &self.source.as_ref().map(|_| "set"))
            .finish()
    }
}

/// Puts `source` behind the game's XInput in `e` (`None`: no pad, every user "not connected").
pub fn set_source(e: &mut Engine, source: Option<Box<dyn PadSource>>) {
    state!(e).pads.source = source;
}

/// Puts the system's own pads behind the game's XInput in `e` and answers the line for the log
/// saying which. On Windows: the system's XInput DLL ([`win32::open`]), and the PlayStation pads
/// and the controller database's pads read as HID devices ([`hid_win::start`], the database with
/// the player's lines over it: [`sdl_db::Database::with_players_lines`]), through
/// [`install_system`]. On Linux: the pads read from the kernel's event devices (`linux::start`),
/// through [`install_lists`]. macOS: none yet (every user "not connected").
pub fn use_system_pads(e: &mut Engine) -> String {
    #[cfg(windows)]
    {
        let hint = std::env::var("SDL_GAMECONTROLLERCONFIG").ok();
        let folder = crate::mod_files::folder(e);
        let (db, log) = sdl_db::Database::with_players_lines(folder.as_deref(), hint.as_deref());
        for line in log {
            eprintln!("{line}");
        }
        let (sony, mapped) = hid_win::start(db, sdl_hints::DeviceLists::from_env());
        install_system(e, win32::open(), pad_lists(sony, mapped))
    }
    #[cfg(target_os = "linux")]
    {
        install_lists(e, vec![Box::new(linux::start())]);
        "nv-rs: game pads through the kernel's event devices: Xbox-style, PlayStation and \
         Nintendo pads, and Steam's virtual pad (first when Steam started the game) (the game's \
         own Controls settings turn the pad and its vibration off)"
            .to_string()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        set_source(e, None);
        "nv-rs: game pads: not read on this system yet (keyboard and mouse)".to_string()
    }
}

/// The HID lists in the order the game's users take them: the PlayStation pads, then the
/// controller database's, so a database pad found first never takes a PlayStation pad's user
/// (review of #804).
pub fn pad_lists(
    sony: impl PadList + 'static,
    mapped: impl PadList + 'static,
) -> Vec<Box<dyn PadList>> {
    vec![Box::new(sony), Box::new(mapped)]
}

/// Puts the `lists` of pads behind the game's XInput in `e`, with no XInput of the system's (a
/// system without one), the game's users over all of them ([`Chain`]).
pub fn install_lists(e: &mut Engine, lists: Vec<Box<dyn PadList>>) {
    set_source(e, Some(Box::new(Chain::<NoXInput>::new(None, lists))));
}

/// Puts the system's XInput `opened` (its source and its DLL's name), behind the empty-slot
/// throttle ([`Throttled`]), and the other `lists` of pads (the PlayStation pads) behind the
/// game's XInput in `e`, the game's users over all of them ([`Chain`]), and answers the log line.
pub fn install_system<S: PadSource + 'static>(
    e: &mut Engine,
    opened: Option<(S, &str)>,
    lists: Vec<Box<dyn PadList>>,
) -> String {
    let others = if lists.is_empty() {
        ""
    } else {
        " and PlayStation pads and the controller database's pads through HID"
    };
    let line = match &opened {
        Some((_, dll)) => format!(
            "nv-rs: game pads through {dll}{others} (the game's own Controls settings turn the \
             pad and its vibration off)"
        ),
        None => format!(
            "nv-rs: game pads: no XInput DLL in the system folder (xinput1_3, xinput1_4, \
             xinput9_1_0){others}"
        ),
    };
    let xinput = opened.map(|(source, _)| Throttled::new(source));
    set_source(e, Some(Box::new(Chain::new(xinput, lists))));
    line
}

/// About how long an empty slot answers "not connected" without asking the system again.
pub const EMPTY_RECHECK: Duration = Duration::from_secs(1);
/// How far apart the four users' rechecks lie: with no pad at all, the game's reads (which ask
/// for all four users) reach the system for at most one slot at a time (review of #783).
pub const STAGGER: Duration = Duration::from_millis(250);

/// A source whose empty slots are asked again about once every [`EMPTY_RECHECK`]: in between,
/// both calls for that user answer `ERROR_DEVICE_NOT_CONNECTED` and leave the state as it is (what
/// the system answers for an empty slot, measured). Only `XInputGetState`'s answers decide, and a
/// slot is held back only from a "not connected" that follows another (or comes first): a pad that
/// misses one answer is asked again at the next call, as a second without it is felt (review of
/// #781), and a pad pulled out costs one more ask. Each user's rechecks keep to its own times,
/// [`STAGGER`] apart from the others' ([`next_recheck`]). A connected pad is asked every time.
pub struct Throttled<S> {
    inner: S,
    slots: [Slot; USERS as usize],
    /// The time the rechecks are counted from: the first call's.
    origin: Option<Instant>,
}

/// What [`Throttled`] keeps of one user's slot.
#[derive(Clone, Copy, Default)]
struct Slot {
    /// Its last `XInputGetState` answered a pad.
    connected: bool,
    /// Until when it is held back as empty.
    empty_until: Option<Instant>,
}

/// User `user`'s next recheck of an empty slot after `now`: the first of its times (`origin`, plus
/// `user` times [`STAGGER`], plus a whole number of [`EMPTY_RECHECK`]s) that is at least an
/// [`EMPTY_RECHECK`] less one [`STAGGER`] after `now`. So each slot is asked again after 0.75 to
/// 1.75 seconds, and no two users' rechecks fall in the same quarter second.
pub fn next_recheck(origin: Instant, user: u32, now: Instant) -> Instant {
    let phase = origin + STAGGER * user;
    let earliest = now + EMPTY_RECHECK - STAGGER;
    if earliest <= phase {
        return phase;
    }
    let periods = (earliest - phase)
        .as_nanos()
        .div_ceil(EMPTY_RECHECK.as_nanos());
    phase + EMPTY_RECHECK * periods as u32
}

impl<S: PadSource> Throttled<S> {
    pub fn new(inner: S) -> Throttled<S> {
        Throttled {
            inner,
            slots: [Slot::default(); USERS as usize],
            origin: None,
        }
    }

    /// Whether `user`'s slot is still held back as empty at `now`.
    fn known_empty(&self, user: u32, now: Instant) -> bool {
        self.slots[user as usize]
            .empty_until
            .is_some_and(|until| now < until)
    }

    /// [`PadSource::get_state`] at the time `now`.
    pub fn get_state_at(&mut self, user: u32, state: &mut [u8; 16], now: Instant) -> u32 {
        let origin = *self.origin.get_or_insert(now);
        if self.known_empty(user, now) {
            return ERROR_DEVICE_NOT_CONNECTED;
        }
        let result = self.inner.get_state(user, state);
        let slot = &mut self.slots[user as usize];
        let connected = result != ERROR_DEVICE_NOT_CONNECTED;
        slot.empty_until = (!connected && !slot.connected).then(|| next_recheck(origin, user, now));
        slot.connected = connected;
        result
    }

    /// [`PadSource::set_state`] at the time `now`: held back with the slot, its answer passed on.
    pub fn set_state_at(&mut self, user: u32, vibration: &[u8; 4], now: Instant) -> u32 {
        if self.known_empty(user, now) {
            return ERROR_DEVICE_NOT_CONNECTED;
        }
        self.inner.set_state(user, vibration)
    }
}

impl<S: PadSource> PadSource for Throttled<S> {
    fn get_state(&mut self, user: u32, state: &mut [u8; 16]) -> u32 {
        self.get_state_at(user, state, Instant::now())
    }

    fn set_state(&mut self, user: u32, vibration: &[u8; 4]) -> u32 {
        self.set_state_at(user, vibration, Instant::now())
    }
}

// Platform: replaces XINPUT1_3.DLL XInputGetState (import by ordinal 2, slot `00fdf394`;
// `Controls::Poll` `00a23010` and the interface manager call it through the thunk `009f996e`)
/// A user index of 4 or more or a null state: `ERROR_BAD_ARGUMENTS`. Otherwise the source's answer,
/// its state written into the game's 16 bytes (left as they were when no pad answers); with no
/// source, `ERROR_DEVICE_NOT_CONNECTED`.
pub fn xinput_get_state(e: &mut Engine, user: u32, state: Ptr) -> u32 {
    if user >= USERS || state.addr() == 0 {
        return ERROR_BAD_ARGUMENTS;
    }
    let Some(source) = state!(e).pads.source.as_mut() else {
        return ERROR_DEVICE_NOT_CONNECTED;
    };
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&e.mem.bytes(state.addr(), 16));
    let before = bytes;
    let result = source.get_state(user, &mut bytes);
    if bytes != before {
        e.mem.write(state.addr(), &bytes);
    }
    result
}

// Platform: replaces XINPUT1_3.DLL XInputSetState (import by ordinal 3, slot `00fdf398`; the
// controller rumble `00a255b0`, the rumble queue `00a269f0` and its stop `00a25810` call it through
// the thunk `00ee820a`)
/// A user index of 4 or more or a null vibration: `ERROR_BAD_ARGUMENTS`. Otherwise the source's
/// answer to the two motor speeds; with no source, `ERROR_DEVICE_NOT_CONNECTED`.
pub fn xinput_set_state(e: &mut Engine, user: u32, vibration: Ptr) -> u32 {
    if user >= USERS || vibration.addr() == 0 {
        return ERROR_BAD_ARGUMENTS;
    }
    let Some(source) = state!(e).pads.source.as_mut() else {
        return ERROR_DEVICE_NOT_CONNECTED;
    };
    let mut speeds = [0u8; 4];
    speeds.copy_from_slice(&e.mem.bytes(vibration.addr(), 4));
    source.set_state(user, &speeds)
}

#[cfg(test)]
mod tests;
