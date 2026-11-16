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

/// Every fix of the group: not registered while JIP is not installed or the row is off, registered
/// with the row on.
#[test]
fn the_fixes_need_the_row() {
    for (installed, row, expected) in [
        (false, true, false),
        (true, false, false),
        (true, true, true),
    ] {
        let e = jip(installed, row);
        for &addr in AREA.fix_sites {
            assert_eq!(
                e.site_owners(addr).contains(&Owner::Jip),
                expected,
                "{addr:08x} {installed} {row}"
            );
        }
    }
}

/// H18 (installed, the row off): a teammate made killable answers 1 (the player's test stands
/// as true); without the flag (any other bit) nothing, the exe's test runs.
#[test]
fn teammate_essential_answers_for_a_killable_teammate() {
    let mut e = jip(true, false);
    for (flags2, answer) in [(4u8, Some(1)), (0xff, Some(1)), (0, None), (0xfb, None)] {
        e.mem.set_u8(ACTOR + 0x106, flags2);
        assert_eq!(
            e.site(TEAMMATE_ESSENTIAL.addr(), &[ACTOR]),
            answer,
            "{flags2:x}"
        );
    }
    let e = jip(false, true);
    assert!(e.site_owners(TEAMMATE_ESSENTIAL.addr()).is_empty());
}

/// G04 and G22: the store is skipped (answered), whatever the words.
#[test]
fn removed_stores_are_answered() {
    let mut e = jip(true, true);
    for site in [
        CREATURE_FLAGS_KEPT,
        PATH_START_HEIGHT,
        MOVE_HEIGHT,
        AVOID_HEIGHT,
    ] {
        assert_eq!(
            e.site(site.addr(), &[ACTOR]),
            Some(0),
            "{:08x}",
            site.addr()
        );
        assert_eq!(e.site(site.addr(), &[0]), Some(0), "{:08x}", site.addr());
    }
}

/// G23: the level itself is asked (the exe asks the level minus 1).
#[test]
fn set_level_asks_the_level() {
    let mut e = jip(true, true);
    for level in [7u32, 1, 0, 0x0001_0007, u32::MAX] {
        assert_eq!(e.site(SET_LEVEL_EXPERIENCE.addr(), &[level]), Some(level));
    }
}

/// G28: `GetClampedActorFloatValue(Agility)` asked by address on the value owner, its answer
/// given as `f32` bits, at both places.
#[test]
fn agility_is_the_clamped_value() {
    let mut e = jip(true, true);
    let asked = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
    let seen = asked.clone();
    e.register_double(GET_CLAMPED_ACTOR_VALUE, move |_, a| {
        seen.borrow_mut().push(a.to_vec());
        Ret {
            st0: 10.0,
            ..Ret::default()
        }
    });
    for site in [RELOAD_AGILITY, EQUIP_AGILITY] {
        assert_eq!(
            e.site(site.addr(), &[ACTOR + 0xa4]),
            Some(10.0f32.to_bits())
        );
    }
    assert_eq!(*asked.borrow(), vec![vec![ACTOR + 0xa4, 10]; 2]);
}

/// H11: only the player (`refID` 0x14); the first build and a new root node build; the same
/// root node again with a ragdoll controller is answered (no build), without one it builds.
#[test]
fn the_players_ragdoll_is_not_built_twice_on_one_node() {
    let mut e = jip(true, true);
    let build = |e: &mut Engine, id: u32, root: u32, controller: u32| {
        e.mem.set_u32(ACTOR + 0xc, id);
        e.mem.set_u32(ACTOR + 0xac, controller);
        e.site(RAGDOLL_BUILD.addr(), &[ACTOR, root])
    };
    // Another actor never: and it leaves the player's node alone.
    assert_eq!(build(&mut e, 0x0001_0000, 0x5000, 1), None);
    assert_eq!(build(&mut e, 0x0001_0000, 0x5000, 1), None);
    assert_eq!(build(&mut e, 0x14, 0x5000, 1), None, "the first");
    assert_eq!(build(&mut e, 0x14, 0x5000, 1), Some(0), "the same node");
    assert_eq!(build(&mut e, 0x14, 0x5000, 0), None, "no controller");
    assert_eq!(build(&mut e, 0x14, 0x6000, 1), None, "a new node");
    assert_eq!(build(&mut e, 0x0001_0000, 0x5000, 1), None);
    assert_eq!(
        build(&mut e, 0x14, 0x6000, 1),
        Some(0),
        "kept over the other actor"
    );
}

/// O13: the dialogue sound's flags by the base form's flag 0x2000; a reference with no base form
/// leaves the exe's; the two other flag words get 0x80000.
#[test]
fn voice_modulation_flags() {
    let mut e = jip(true, true);
    assert_ne!(
        e.jip_patches() & jip_patches::VOICE_MODULATION_FIX,
        0,
        "on by default"
    );
    let base = ACTOR + 0x200;
    e.mem.set_u32(ACTOR + 0x20, base);
    for (flags, answer) in [
        (0x2000u32, 0x104u32),
        (0xffff_ffff, 0x104),
        (0, 0x8_0104),
        (0xffff_dfff, 0x8_0104),
    ] {
        e.mem.set_u32(base + 8, flags);
        assert_eq!(
            e.site(VOICE_FLAGS.addr(), &[ACTOR]),
            Some(answer),
            "{flags:x}"
        );
    }
    e.mem.set_u32(ACTOR + 0x20, 0);
    assert_eq!(e.site(VOICE_FLAGS.addr(), &[ACTOR]), None);
    for site in [REF_SOUND_FLAGS, MAP_SOUND_FLAGS] {
        assert_eq!(e.site(site.addr(), &[0x106]), Some(0x8_0106));
        assert_eq!(e.site(site.addr(), &[0x105]), Some(0x8_0105));
        assert_eq!(e.site(site.addr(), &[u32::MAX]), Some(u32::MAX));
    }
}
