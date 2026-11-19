//! The system's XInput on Windows: the first of `xinput1_3.dll` (the DLL the game links),
//! `xinput1_4.dll` (every Windows from 8) and `xinput9_1_0.dll` (Vista and later) that loads
//! from the system folder, its `XInputGetState` and `XInputSetState` called with the game's
//! arguments and their answers passed back unchanged. The functions are looked up by name:
//! `xinput9_1_0.dll`'s ordinals 2 and 3 are other functions (measured, 2026-11-18); in the other
//! two the names and the game's ordinals agree.

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
    use windows::core::PCSTR;
    use windows::Win32::System::LibraryLoader::{
        GetProcAddress, LoadLibraryExA, LOAD_LIBRARY_SEARCH_SYSTEM32,
    };
    for dll in DLLS {
        // SAFETY: a NUL-terminated name; the module is kept loaded for the process.
        let Ok(module) =
            (unsafe { LoadLibraryExA(PCSTR(dll.as_ptr()), None, LOAD_LIBRARY_SEARCH_SYSTEM32) })
        else {
            continue;
        };
        // SAFETY: NUL-terminated export names of a loaded module.
        let get = unsafe { GetProcAddress(module, PCSTR(b"XInputGetState\0".as_ptr())) };
        let set = unsafe { GetProcAddress(module, PCSTR(b"XInputSetState\0".as_ptr())) };
        if let (Some(get), Some(set)) = (get, set) {
            let name = std::str::from_utf8(&dll[..dll.len() - 1]).unwrap_or("xinput");
            // SAFETY: both exports have these documented signatures (XInput.h, `WINAPI`).
            let source = unsafe {
                SystemXInput {
                    get: std::mem::transmute::<unsafe extern "system" fn() -> isize, GetState>(get),
                    set: std::mem::transmute::<unsafe extern "system" fn() -> isize, SetState>(set),
                }
            };
            return Some((source, name));
        }
    }
    None
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
    /// `xinput1_4.dll`), a user index of 4 answers `ERROR_BAD_ARGUMENTS`, and an empty slot (if one
    /// is, which no pad plugged in makes every slot) leaves the state as it was. Read-only: no
    /// vibration is sent.
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
            }
        }
    }
}
