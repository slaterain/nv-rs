//! Full screen asked (`bFull Screen=1`): the game is drawn in the borderless window over the
//! monitor, on every backend and in both presets, and the log says so once. **Not 1:1 with
//! FalloutNV.exe**: nv-rs offers no exclusive full-screen device yet.
//!
//! **Why.** With the setting on, the original asks Direct3D 9 for an exclusive full-screen
//! device, and on a desktop of several displays whose primary size differs from the game's it
//! first asks for `Direct3DCreate9Ex` (`Renderer::Init`, `004da670`, tests `SM_CMONITORS` and
//! passes the answer to `00e76210`, which calls `NiXenonRenderer::CreateDirect3D9`, `00e69460`).
//! The platform provides neither: its model of `IDirect3D9::CreateDevice` refuses a full-screen
//! device and `Direct3DCreate9Ex` is not there, so the run stopped at start-up whatever the
//! backend (runs fs2 and fs3, 2026-11-10: three displays, then one).
//!
//! **What this does.** The setting is `bFull Screen:Display` (`011c77b4`, made by the static
//! initializer `00f3fd00` with the default 1; its flag is the byte at `+4`, as `00408d60` gives
//! it). The game reads it from its INI files in `INISettingCollection::ReadSetting` (`00c44280`,
//! Xbox PDB), and after that `Renderer::Init` reads it (the render window, the multi-monitor
//! test, the device's `Windowed`), so do `004dd180` (`CheckDeviceMultiSampleType`'s windowed
//! argument) and the getter `00446e10` (the message-box wrapper `0086a580`, the main loop).
//! When `ReadSetting` has read it as 1, it is set to 0 before anything else reads it, and the
//! window is the borderless one (`d3d9real::want_borderless`): that read is in `main` before the
//! game makes its window. `Renderer::Init`'s entry does the same for a start that read no file
//! (the default 1); the window is already made then, so the game runs in it as it is. The
//! player's file keeps its line: the platform's `WritePrivateProfileStringA` writes nothing.

use engine::prelude::*;
use engine::units::bsmain::xinisetting_xenon::ini_setting_collection_read_setting;
use engine::units::fallout_shared::renderer::renderer_init;
use std::sync::atomic::{AtomicBool, Ordering};

/// `bFull Screen:Display`: the setting object; its flag is the byte at [`FLAG`].
pub const FULL_SCREEN: u32 = 0x011c_77b4;
/// Where a setting's value is (`00408d60`).
const FLAG: u32 = 4;
/// `INISettingCollection::ReadSetting` (Xbox PDB): `thiscall (collection, setting)`.
pub const READ_SETTING: u32 = 0x00c4_4280;
/// `Renderer::Init` (Xbox PDB): `(window, instance)`.
pub const RENDERER_INIT: u32 = 0x004d_a670;

/// Registers the two entries over their translations, in every preset.
pub fn install(e: &mut Engine) {
    e.register(READ_SETTING, |e, a| {
        read_setting(e, a, ini_setting_collection_read_setting)
    });
    e.register(RENDERER_INIT, |e, a| renderer_entry(e, a, renderer_init));
}

/// `ReadSetting(collection, setting)` by `read` (the translation), then the full-screen flag
/// taken as windowed when it was the setting read ([`take_as_windowed`]).
fn read_setting(e: &mut Engine, a: &[u32], read: fn(&mut Engine, Ptr, Ptr) -> u8) -> Ret {
    let parsed = read(e, Ptr::new(a[0]), Ptr::new(a[1]));
    if a[1] == FULL_SCREEN {
        take_as_windowed(e, true, choose_borderless);
    }
    Ret {
        eax: u32::from(parsed),
        ..Ret::default()
    }
}

/// `Renderer::Init(window, instance)` by `init` (the translation), the full-screen flag taken as
/// windowed first: the game's default when no file set it ([`take_as_windowed`]).
fn renderer_entry(
    e: &mut Engine,
    a: &[u32],
    init: impl FnOnce(&mut Engine, u32, u32) -> Ptr,
) -> Ret {
    take_as_windowed(e, false, choose_borderless);
    Ret {
        eax: init(e, a[0], a[1]).addr(),
        ..Ret::default()
    }
}

/// The run's window made the borderless one (`d3d9real::want_borderless`).
fn choose_borderless() {
    #[cfg(windows)]
    engine_platform::d3d9real::want_borderless(true);
}

/// When the full-screen flag is on, turns it off and answers true; `before_window`: the game's
/// window is still to be made, so `borderless` makes it the borderless one. The first time, the
/// log says so.
fn take_as_windowed(e: &mut Engine, before_window: bool, borderless: impl FnOnce()) -> bool {
    if e.mem.u8(FULL_SCREEN + FLAG) == 0 {
        return false;
    }
    e.mem.set_u8(FULL_SCREEN + FLAG, 0);
    if before_window {
        borderless();
    }
    static SAID: AtomicBool = AtomicBool::new(false);
    if !SAID.swap(true, Ordering::Relaxed) {
        eprintln!(
            "Full screen asked (bFull Screen=1): exclusive full screen is not offered yet; the game is drawn in {}",
            if before_window {
                "the borderless window over the monitor"
            } else {
                "its window (the setting was not read from FalloutPrefs.ini)"
            }
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn engine(flag: u8) -> Engine {
        let mut e = Engine::new();
        e.mem.map(FULL_SCREEN & !0xfff, 0x1000);
        e.mem.set_u8(FULL_SCREEN + FLAG, flag);
        e
    }

    /// The flag on is turned off, once; off it stays off; the borderless window is chosen only
    /// before the game's window is made.
    #[test]
    fn full_screen_is_taken_as_windowed() {
        let mut e = engine(1);
        let chosen = Cell::new(0);
        assert!(take_as_windowed(&mut e, true, || chosen.set(chosen.get() + 1)));
        assert_eq!(e.mem.u8(FULL_SCREEN + FLAG), 0);
        assert_eq!(chosen.get(), 1);
        assert!(!take_as_windowed(&mut e, true, || chosen.set(chosen.get() + 1)));
        assert_eq!(chosen.get(), 1);
        let mut late = engine(1);
        assert!(take_as_windowed(&mut late, false, || chosen.set(chosen.get() + 1)));
        assert_eq!(late.mem.u8(FULL_SCREEN + FLAG), 0);
        assert_eq!(chosen.get(), 1);
    }

    /// `ReadSetting` of the full-screen setting reading 1 leaves 0; another setting's read is
    /// left as read; the translation's answer is passed on.
    #[test]
    fn the_read_of_the_setting_leaves_it_off() {
        let mut e = engine(0);
        let ret = read_setting(&mut e, &[0x1234, FULL_SCREEN], |e, collection, setting| {
            assert_eq!((collection.addr(), setting.addr()), (0x1234, FULL_SCREEN));
            e.mem.set_u8(setting.addr() + FLAG, 1);
            7
        });
        assert_eq!(ret.eax, 7);
        assert_eq!(e.mem.u8(FULL_SCREEN + FLAG), 0);
        let other = FULL_SCREEN + 0x40;
        read_setting(&mut e, &[0x1234, other], |e, _, setting| {
            e.mem.set_u8(setting.addr() + FLAG, 1);
            0
        });
        assert_eq!(e.mem.u8(other + FLAG), 1);
    }

    /// `Renderer::Init` sees the flag off even when no file was read (the default 1), and its
    /// answer is passed on.
    #[test]
    fn the_renderer_never_sees_full_screen() {
        let mut e = engine(1);
        let ret = renderer_entry(&mut e, &[0x11, 0x22], |e, window, instance| {
            assert_eq!((window, instance), (0x11, 0x22));
            assert_eq!(e.mem.u8(FULL_SCREEN + FLAG), 0);
            Ptr::new(0x5000)
        });
        assert_eq!(ret.eax, 0x5000);
    }
}
