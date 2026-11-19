//! The system's XInput on Windows: the first of `xinput1_3.dll` (the DLL the game links),
//! `xinput1_4.dll` (every Windows from 8) and `xinput9_1_0.dll` (Vista and later) that loads
//! from the system folder, its `XInputGetState` and `XInputSetState` called with the game's
//! arguments and their answers passed back unchanged. The functions are looked up by name:
//! `xinput9_1_0.dll`'s ordinals 2 and 3 are other functions (measured, 2026-11-18); in the other
//! two the names and the game's ordinals agree.
//!
//! Not taken: `XInputGetStateEx` (ordinal 100 of 1_3 and 1_4), which JIP LN reads the pad with
//! and which also reports the guide button. The game reads the pad through its plain
//! `XInputGetState`, and JIP's hook reads it through the game's import here, so the guide button
//! is not seen (`crates/jip/src/input.rs`): an accepted difference, the game itself never reads it.

use super::PadSource;
use std::ffi::c_void;

/// The DLLs tried, in order, each a NUL-terminated name.
const DLLS: [&[u8]; 3] = [b"xinput1_3.dll\0", b"xinput1_4.dll\0", b"xinput9_1_0.dll\0"];

type GetState = unsafe extern "system" fn(u32, *mut c_void) -> u32;
type SetState = unsafe extern "system" fn(u32, *mut c_void) -> u32;

/// `XINPUT_STATE` and `XINPUT_VIBRATION` as the DLL takes them: its fields' alignment.
#[repr(C, align(4))]
struct Aligned<const N: usize>([u8; N]);

/// One loaded XInput DLL's two functions (the module is never freed).
pub struct SystemXInput {
    get: GetState,
    set: SetState,
}

// SAFETY: the two are plain function addresses in a module that stays loaded; XInput's functions
// may be called from any thread.
unsafe impl Send for SystemXInput {}

/// The first XInput DLL of [`DLLS`] that loads from the system folder and has both functions,
/// with its name; `None` when none does.
pub fn open() -> Option<(SystemXInput, &'static str)> {
    for dll in DLLS {
        let Some(module) = load(dll) else {
            continue;
        };
        if let Some(source) = exports(module) {
            let name = std::str::from_utf8(&dll[..dll.len() - 1]).unwrap_or("xinput");
            return Some((source, name));
        }
    }
    None
}

/// The DLL `dll` (a NUL-terminated name) from the system folder, kept loaded for the process.
fn load(dll: &[u8]) -> Option<windows::Win32::Foundation::HMODULE> {
    use windows::core::PCSTR;
    use windows::Win32::System::LibraryLoader::{LoadLibraryExA, LOAD_LIBRARY_SEARCH_SYSTEM32};
    // SAFETY: a NUL-terminated name.
    unsafe { LoadLibraryExA(PCSTR(dll.as_ptr()), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }.ok()
}

/// The loaded XInput `module`'s two functions, by name; `None` when it lacks one.
fn exports(module: windows::Win32::Foundation::HMODULE) -> Option<SystemXInput> {
    use windows::core::PCSTR;
    use windows::Win32::System::LibraryLoader::GetProcAddress;
    // SAFETY: NUL-terminated export names of a loaded module.
    let get = unsafe { GetProcAddress(module, PCSTR(b"XInputGetState\0".as_ptr())) }?;
    let set = unsafe { GetProcAddress(module, PCSTR(b"XInputSetState\0".as_ptr())) }?;
    // SAFETY: both exports have these documented signatures (XInput.h, `WINAPI`).
    Some(unsafe {
        SystemXInput {
            get: std::mem::transmute::<unsafe extern "system" fn() -> isize, GetState>(get),
            set: std::mem::transmute::<unsafe extern "system" fn() -> isize, SetState>(set),
        }
    })
}

impl PadSource for SystemXInput {
    fn get_state(&mut self, user: u32, state: &mut [u8; 16]) -> u32 {
        let mut buffer = Aligned(*state);
        // SAFETY: a 16-byte, 4-aligned `XINPUT_STATE` the call may write.
        let result = unsafe { (self.get)(user, buffer.0.as_mut_ptr().cast()) };
        *state = buffer.0;
        result
    }

    fn set_state(&mut self, user: u32, vibration: &[u8; 4]) -> u32 {
        let mut buffer = Aligned(*vibration);
        // SAFETY: a 4-byte, 4-aligned `XINPUT_VIBRATION` the call only reads.
        unsafe { (self.set)(user, buffer.0.as_mut_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This machine's XInput as the game would see it: a DLL loads (every Windows from 8 has
    /// `xinput1_4.dll`), a user index of 4 answers `ERROR_BAD_ARGUMENTS`, an empty slot (if one
    /// is, which no pad plugged in makes every slot) leaves the state as it was, and a pad's state
    /// is written. Read-only: no vibration is sent.
    #[test]
    fn the_systems_xinput_loads_and_answers_as_measured() {
        let (mut source, name) = open().expect("an XInput DLL in the system folder");
        assert!(DLLS.iter().any(|d| &d[..d.len() - 1] == name.as_bytes()));
        let mut state = [0xaa; 16];
        assert_eq!(
            source.get_state(super::super::USERS, &mut state),
            super::super::ERROR_BAD_ARGUMENTS
        );
        assert_eq!(state, [0xaa; 16]);
        for user in 0..super::super::USERS {
            let mut state = [0xaa; 16];
            let result = source.get_state(user, &mut state);
            if result == super::super::ERROR_DEVICE_NOT_CONNECTED {
                assert_eq!(
                    state, [0xaa; 16],
                    "user {user}: an empty slot's state is kept"
                );
            } else {
                assert_eq!(result, super::super::ERROR_SUCCESS, "user {user}");
                // Written: the buttons' reserved bits (0x0C00), which the 0xAA filling sets.
                assert_eq!(state[5] & 0x0c, 0, "user {user}: the state was written");
            }
        }
    }

    /// What a stand-in DLL's two functions saw: the function, the user, the buffer's address and
    /// the vibration's 4 bytes.
    static SEEN: std::sync::Mutex<Vec<(char, u32, usize, [u8; 4])>> =
        std::sync::Mutex::new(Vec::new());

    /// A stand-in `XInputGetState`: writes 1 to 16 into the state and answers 0x1111.
    unsafe extern "system" fn fake_get(user: u32, state: *mut c_void) -> u32 {
        // SAFETY: the adapter hands a 16-byte buffer.
        let bytes = unsafe { std::slice::from_raw_parts_mut(state.cast::<u8>(), 16) };
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = i as u8 + 1;
        }
        SEEN.lock()
            .unwrap()
            .push(('g', user, state as usize, [0; 4]));
        0x1111
    }

    /// A stand-in `XInputSetState`: keeps the 4 vibration bytes and answers 0x2222.
    unsafe extern "system" fn fake_set(user: u32, vibration: *mut c_void) -> u32 {
        let mut v = [0u8; 4];
        // SAFETY: the adapter hands a 4-byte buffer.
        unsafe { std::ptr::copy_nonoverlapping(vibration.cast::<u8>(), v.as_mut_ptr(), 4) };
        SEEN.lock()
            .unwrap()
            .push(('s', user, vibration as usize, v));
        0x2222
    }

    /// The adapter between the game's bytes and a DLL (review of #781: a real pad goes through
    /// nothing else): the state the DLL writes is copied back whole and its answer passed on, the
    /// user handed through; the game's 4 vibration bytes reach `XInputSetState`, and only it;
    /// both buffers 4-aligned as XInput's structures are.
    #[test]
    fn the_adapter_hands_the_games_bytes_both_ways() {
        let mut source = SystemXInput {
            get: fake_get,
            set: fake_set,
        };
        let mut state = [0xaa; 16];
        assert_eq!(source.get_state(2, &mut state), 0x1111);
        assert_eq!(state, std::array::from_fn(|i| i as u8 + 1));
        assert_eq!(source.set_state(3, &[0x11, 0x22, 0x33, 0x44]), 0x2222);
        let seen = SEEN.lock().unwrap();
        let calls: Vec<(char, u32, [u8; 4])> = seen.iter().map(|s| (s.0, s.1, s.3)).collect();
        assert_eq!(
            calls,
            [('g', 2, [0; 4]), ('s', 3, [0x11, 0x22, 0x33, 0x44])]
        );
        assert!(seen.iter().all(|s| s.2 % 4 == 0), "aligned");
    }

    /// The names looked up are the game's ordinals 2 and 3 in `xinput1_3.dll` and `xinput1_4.dll`
    /// (the one on every Windows from 8), as the module's doc says.
    #[test]
    fn the_names_looked_up_are_the_games_ordinals() {
        use windows::core::PCSTR;
        use windows::Win32::System::LibraryLoader::GetProcAddress;
        let mut checked = 0;
        for dll in &DLLS[..2] {
            let Some(module) = load(dll) else {
                continue;
            };
            let source = exports(module).expect("both functions");
            // SAFETY: an ordinal in the low word, as GetProcAddress takes one.
            let ordinal = |n: usize| {
                unsafe { GetProcAddress(module, PCSTR(n as *const u8)) }.map(|f| f as usize)
            };
            assert_eq!(Some(source.get as usize), ordinal(2));
            assert_eq!(Some(source.set as usize), ordinal(3));
            checked += 1;
        }
        assert!(checked > 0, "xinput1_4.dll is on every Windows from 8");
    }
}
