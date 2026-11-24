//! lStewieAl's Tweaks 10.05 Faster Sleep Wait startup configuration.
//!
//! The owned DLL's reader at 10063A90 multiplies numeric milliseconds by
//! widened float 3A83126F and stores a double at 100FB130 (10063AF1).
//! Installer 1008449A replaces the comparison operand at 007C063E and,
//! optionally, redirects 007C0636 to the accumulated-time division at 10038410.
//! The native menu retains the frame accumulation and applies these choices
//! only at that comparison. This does not implement Tweaks' settings menu.

use crate::Engine;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

const ENABLED: u8 = 1;
const DONT_SCALE: u8 = 2;

#[derive(Default)]
struct State {
    delay_bits: AtomicU64,
    flags: AtomicU8,
}

/// One enabled startup choice, shared by all engine views of a run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub delay_seconds: f64,
    pub dont_scale: bool,
}

/// None when not configured or disabled. Reads never create an extension slot,
/// including on late/real-thread paths where an absent slot must remain absent.
pub fn settings(e: &Engine) -> Option<Settings> {
    let state = e.ext_get::<State>()?;
    let flags = state.flags.load(Ordering::Acquire);
    (flags & ENABLED != 0).then(|| Settings {
        delay_seconds: f64::from_bits(state.delay_bits.load(Ordering::Relaxed)),
        dont_scale: flags & DONT_SCALE != 0,
    })
}

/// Publishes startup settings before real-thread engines are copied, or sets
/// them in tests. This is not a live reconfiguration protocol.
///
/// No UI-range clamp or minimum is introduced: the DLL reads a number as f64
/// and multiplies it by a widened f32, rather than by exact rational 0.001 or
/// narrowing the resulting threshold to f32. Disabled retains vanilla timing.
pub fn configure(e: &mut Engine, enabled: bool, wait_ms: f64, dont_scale: bool) {
    let delay = wait_ms * f64::from(f32::from_bits(0x3a83_126f));
    let state = e.ext_free::<State>();
    state.delay_bits.store(delay.to_bits(), Ordering::Relaxed);
    let flags = if enabled {
        ENABLED | if dont_scale { DONT_SCALE } else { 0 }
    } else {
        0
    };
    state.flags.store(flags, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_settings_are_off_and_readers_do_not_allocate_a_slot() {
        let e = Engine::new();
        assert!(e.ext_get::<State>().is_none());
        assert_eq!(settings(&e), None);
        assert!(e.ext_get::<State>().is_none());
    }

    #[test]
    fn delay_keeps_the_dlls_widened_float_multiplier_and_double_result() {
        let mut e = Engine::new();
        for (ms, expected_bits) in [
            (0.0, 0x0000_0000_0000_0000),
            (300.0, 0x3fd3_3333_4280_0000),
            (1000.0, 0x3ff0_0000_0cc0_0000),
            (-1.0, 0xbf50_624d_e000_0000),
        ] {
            configure(&mut e, true, ms, true);
            let choice = settings(&e).unwrap();
            assert_eq!(choice.delay_seconds.to_bits(), expected_bits);
            assert!(choice.dont_scale);
        }
        configure(&mut e, false, 0.0, true);
        assert_eq!(settings(&e), None);
    }

    #[test]
    fn copied_engine_views_share_startup_settings_without_a_shared_slot_lock() {
        let mut owner = Engine::new();
        configure(&mut owner, true, 300.0, true);
        let mut peer = Engine::new();
        owner.share_slots_for_test(&mut peer);
        assert_eq!(settings(&peer), settings(&owner));
        configure(&mut peer, true, 0.0, false);
        assert_eq!(
            settings(&owner),
            Some(Settings {
                delay_seconds: 0.0,
                dont_scale: false,
            })
        );
        configure(&mut owner, false, 300.0, true);
        assert_eq!(settings(&peer), None);
        assert!(!crate::extend::holds_shared());
    }
}
