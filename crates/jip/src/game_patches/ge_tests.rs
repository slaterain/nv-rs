//! Group GE's game patches: G63 (the actor flags on load), case by case from JIP's bytes.

use super::*;
use crate::game_patches::set_row;
use crate::test_support::engine;

/// `CreateFileA`'s import slot (the settings file read): answered "no file", so JIP's defaults.
const CREATE_FILE_A: u32 = 0x00fd_f088;
/// An actor (0x1B4 bytes) in the test engine's memory.
const ACTOR: u32 = 0x0300_4000;

/// An engine with JIP installed as `installed` answers and the host's row `row`.
fn jip(installed: bool, row: bool) -> Engine {
    let mut e = engine();
    e.register(CREATE_FILE_A, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    // `TESTopic::GetOwnerTopic`, which 3.9a's fix wraps with the row on.
    e.register(0x0061_a310, |_, _| Ret::default());
    crate::install_as(&mut e, installed);
    set_row(&mut e, row);
    e
}

/// The actor's bytes `+0x104..+0x108` (`bInCombat`, JIP's three flag bytes) and its flag word.
fn actor(e: &Engine, around: u32, flags: u32) {
    e.mem.set_u32(ACTOR + 0x104, around);
    e.mem.set_u32(ACTOR + 0x140, flags);
}

fn read(e: &Engine) -> (u32, u32) {
    (e.mem.u32(ACTOR + 0x104), e.mem.u32(ACTOR + 0x140))
}

/// Not installed (today): no handler at the seam, with the row on; asking it changes nothing.
#[test]
fn actor_flags_not_installed_nothing_is_registered() {
    let mut e = jip(false, true);
    assert!(e.site_owners(ACTOR_FLAGS_LOADED.addr()).is_empty());
    actor(&e, 0xabff_ff01, 0xffff_ffff);
    assert_eq!(e.site(ACTOR_FLAGS_LOADED.addr(), &[ACTOR]), None);
    assert_eq!(read(&e), (0xabff_ff01, 0xffff_ffff));
}

/// Installed with the row off (vanilla): JIP's bytes `+0x105`/`+0x106` cleared (`bInCombat` and
/// `+0x107` kept), the flag word left to the exe's two calls (nothing answered).
#[test]
fn actor_flags_installed_row_off_clears_jips_bytes_only() {
    let mut e = jip(true, false);
    assert_eq!(e.site_owners(ACTOR_FLAGS_LOADED.addr()), [Owner::Jip]);
    for (around, flags) in [
        (0xabff_ff01u32, 0xffff_ffffu32),
        (0x0000_0000, 0x0000_0000),
        (0x0012_3400, 0xc000_0000),
    ] {
        actor(&e, around, flags);
        assert_eq!(e.site(ACTOR_FLAGS_LOADED.addr(), &[ACTOR]), None);
        assert_eq!(
            read(&e),
            (around & 0xff00_00ff, flags),
            "{around:x} {flags:x}"
        );
    }
}

/// Installed with the row on (fixes): JIP's bytes cleared, only bit 0x40000000 of the flag word
/// cleared (the essential bit 0x80000000 and every other bit kept), the calls replaced.
#[test]
fn actor_flags_installed_row_on_keeps_the_essential_bit() {
    let mut e = jip(true, true);
    assert_ne!(e.jip_patches() & jip_patches::FIXES, 0);
    for (around, flags, after) in [
        (0xabff_ff01u32, 0xffff_ffffu32, 0xbfff_ffffu32),
        (0x0000_0000, 0x0000_0000, 0x0000_0000),
        (0x00ff_0000, 0x8000_0000, 0x8000_0000),
        (0x0000_ff00, 0x4000_0000, 0x0000_0000),
        (0x0000_0000, 0x3fff_ffff, 0x3fff_ffff),
    ] {
        actor(&e, around, flags);
        assert_eq!(e.site(ACTOR_FLAGS_LOADED.addr(), &[ACTOR]), Some(0));
        assert_eq!(
            read(&e),
            (around & 0xff00_00ff, after),
            "{around:x} {flags:x}"
        );
    }
}
