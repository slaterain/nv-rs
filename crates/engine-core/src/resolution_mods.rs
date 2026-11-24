//! Startup choices for HighResLocalMaps 1.2 and HDPipboy 1.2.
//!
//! The translations apply their original instruction changes at the native
//! branches. Setters run during initialization (before real threads are made)
//! or in tests. Readers never create a slot, including when both rows are off.

use crate::Engine;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// HighResLocalMaps sets this byte when loaded, then prevents the map command's
/// final clear. Its descriptor still tests the byte rather than bypassing it.
const LOCAL_MAP_HIGH_RES_BYTE: u32 = 0x0120_04fc;
const ORIGINAL_LOCAL_MAP_MASK: u32 = 0x180;

#[derive(Default)]
struct State {
    /// Zero means off. Otherwise this is the DLL's validated immediate mask.
    /// Keeping the enable choice and mask in one atomic publishes them together.
    local_map_mask: AtomicU32,
    screens: AtomicBool,
}

/// The AND immediate used by the local-map texture descriptor. Off retains the
/// game's 0x180; author option 0 uses 0x80, option 2 uses 0x380, all others 0x180.
pub fn local_map_mask(e: &Engine) -> u32 {
    match e
        .ext_get::<State>()
        .map_or(0, |state| state.local_map_mask.load(Ordering::Acquire))
    {
        0 => ORIGINAL_LOCAL_MAP_MASK,
        mask => mask,
    }
}

/// Whether HighResLocalMaps leaves the high-resolution byte set after the map
/// command. With no startup choice this is false and allocates nothing.
pub fn local_maps_enabled(e: &Engine) -> bool {
    e.ext_get::<State>()
        .is_some_and(|state| state.local_map_mask.load(Ordering::Acquire) != 0)
}

/// Whether HDPipboy's native resolution and input changes are enabled.
pub fn screens_enabled(e: &Engine) -> bool {
    e.ext_get::<State>()
        .is_some_and(|state| state.screens.load(Ordering::Acquire))
}

/// Sets the startup HighResLocalMaps choice. When enabling, the caller must
/// have mapped the game's byte at 0x012004FC; it becomes 1 as in DLL Load.
/// Disabling leaves that guest byte unchanged, preserving the game's state.
/// This must run before real-thread engines are copied, or only in tests.
pub fn set_local_maps(e: &mut Engine, on: bool, option: i32) {
    let mask = if on {
        e.mem.set_u8(LOCAL_MAP_HIGH_RES_BYTE, 1);
        match option {
            0 => 0x80,
            2 => 0x380,
            _ => ORIGINAL_LOCAL_MAP_MASK,
        }
    } else {
        0
    };
    e.ext_free::<State>()
        .local_map_mask
        .store(mask, Ordering::Release);
}

/// Sets HDPipboy's startup choice, before real-thread engines are copied, or in
/// tests. No display size is cached here: the native branches read game state.
pub fn set_screens(e: &mut Engine, on: bool) {
    e.ext_free::<State>().screens.store(on, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_with_map_byte() -> Engine {
        let mut e = Engine::new();
        e.map(0x0120_0000, 0x1000);
        e
    }

    #[test]
    fn absent_startup_choices_read_as_off_without_creating_a_slot() {
        let e = Engine::new();
        assert!(e.ext_get::<State>().is_none());
        assert_eq!(local_map_mask(&e), 0x180);
        assert!(!local_maps_enabled(&e));
        assert!(!screens_enabled(&e));
        assert!(e.ext_get::<State>().is_none());
        let mut peer = Engine::new();
        e.share_slots_for_test(&mut peer);
        assert_eq!(local_map_mask(&peer), 0x180);
        assert!(!local_maps_enabled(&peer));
        assert!(!screens_enabled(&peer));
        assert!(peer.ext_get::<State>().is_none());
    }

    #[test]
    fn author_options_publish_the_mask_and_only_enabling_changes_the_guest_byte() {
        let mut e = engine_with_map_byte();
        for (option, mask) in [(0, 0x80), (1, 0x180), (2, 0x380), (-1, 0x180), (3, 0x180)] {
            e.mem.set_u8(LOCAL_MAP_HIGH_RES_BYTE, 0);
            set_local_maps(&mut e, true, option);
            assert_eq!(e.mem.u8(LOCAL_MAP_HIGH_RES_BYTE), 1);
            assert!(local_maps_enabled(&e));
            assert_eq!(local_map_mask(&e), mask);
            assert!(!screens_enabled(&e));
            e.mem.set_u8(LOCAL_MAP_HIGH_RES_BYTE, 0x7f);
            set_local_maps(&mut e, false, option);
            assert_eq!(e.mem.u8(LOCAL_MAP_HIGH_RES_BYTE), 0x7f);
            assert!(!local_maps_enabled(&e));
            assert_eq!(local_map_mask(&e), 0x180);
        }
    }

    #[test]
    fn copied_thread_engines_observe_one_lock_free_choice_in_both_directions() {
        let mut owner = engine_with_map_byte();
        set_local_maps(&mut owner, true, 2);
        set_screens(&mut owner, false);
        let mut peer = Engine::new();
        owner.share_slots_for_test(&mut peer);
        assert!(local_maps_enabled(&peer));
        assert_eq!(local_map_mask(&peer), 0x380);
        assert!(!screens_enabled(&peer));

        set_screens(&mut peer, true);
        set_local_maps(&mut peer, false, 0);
        assert!(screens_enabled(&owner));
        assert!(!local_maps_enabled(&owner));
        assert_eq!(local_map_mask(&owner), 0x180);
        assert_eq!(owner.mem.u8(LOCAL_MAP_HIGH_RES_BYTE), 1);

        set_local_maps(&mut owner, true, 0);
        assert_eq!(local_map_mask(&peer), 0x80);
        assert!(local_maps_enabled(&peer));
        assert!(screens_enabled(&peer));
        assert!(
            !crate::extend::holds_shared(),
            "readers take no shared-slot lock"
        );
    }
}
