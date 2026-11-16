//! JIP LN NVSE's game patches of group GC (step 3.9) in this file: G98's, G33's and G18's seams.

use super::*;
use engine_core::extend::{jip_patches, Owner, SiteSlot};

const BASE: u32 = 0x0530_0000;
const ACTOR: Ptr<Actor> = Ptr::new(BASE + 0x100);
/// Where the test handlers write the actor they were asked with, one word per seam.
const ASKED: u32 = BASE + 0x800;
/// How often the exe's teammate test ran.
const TEAMMATE_CALLS: u32 = BASE + 0x900;

fn world() -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.register(PROCESS_GET_FORCE_NEXT_UPDATE, |e, _| {
        let n = e.mem.u32(TEAMMATE_CALLS);
        e.mem.set_u32(TEAMMATE_CALLS, n + 1);
        1u32.into_ret()
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x0089_dc0e), |e, a| {
        e.mem.set_u32(ASKED, a[0]);
        Some(0x100)
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x0089_e790), |e, a| {
        e.mem.set_u32(ASKED + 4, a[0]);
        Some(0)
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x0089_f351), |e, a| {
        e.mem.set_u32(ASKED + 8, a[0]);
        Some(0)
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x008a_19cf), |e, a| {
        e.mem.set_u32(ASKED + 12, a[0]);
        Some(0x404)
    });
    e
}

fn nothing_asked(e: &Engine) -> bool {
    (0..4).all(|i| e.mem.u32(ASKED + 4 * i) == 0)
}

/// With JIP not installed (word 0) or installed with the row off (`INSTALLED` only), the fixes'
/// seams (G98, G18) ask nothing: the exe's teammate call runs, the mark is `0x400`. G33 (JIP's
/// own) asks nothing with the word 0 or with only other bits: both jumps as the exe's.
#[test]
fn gc_seams_do_nothing_while_off() {
    for word in [0, jip_patches::INSTALLED] {
        let mut e = world();
        e.set_jip_patches(word);
        assert!(kill_challenge_teammate(&mut e, ACTOR), "{word:08x}");
        assert_eq!(e.mem.u32(TEAMMATE_CALLS), 1);
        assert_eq!(life_state_mark(&mut e, ACTOR), 0x400);
        if word == 0 {
            assert!(!player_killer_patched(&mut e, 0x0089_e790, ACTOR));
            assert!(!player_killer_patched(&mut e, 0x0089_f351, ACTOR));
        }
        assert!(nothing_asked(&e), "{word:08x}");
    }
    let mut e = world();
    e.set_jip_patches(jip_patches::FIXES | jip_patches::CREATURE_SPREAD_FIX);
    assert!(!player_killer_patched(&mut e, 0x0089_e790, ACTOR));
    assert!(!player_killer_patched(&mut e, 0x0089_f351, ACTOR));
    assert_eq!(e.mem.u32(ASKED + 4) | e.mem.u32(ASKED + 8), 0);
}

/// With the row on: G98's answer's low byte is the test (`0x100` is false; no exe call), G18's
/// answer is the mark; each asked with the actor. With `INSTALLED`: both G33 seams asked with the
/// actor, each at its own address.
#[test]
fn gc_seams_take_the_answers_when_on() {
    let mut e = world();
    e.set_jip_patches(jip_patches::FIXES);
    assert!(!kill_challenge_teammate(&mut e, ACTOR));
    assert_eq!(e.mem.u32(TEAMMATE_CALLS), 0);
    assert_eq!(e.mem.u32(ASKED), ACTOR.addr());
    assert_eq!(life_state_mark(&mut e, ACTOR), 0x404);
    assert_eq!(e.mem.u32(ASKED + 12), ACTOR.addr());
    e.set_jip_patches(jip_patches::INSTALLED);
    assert!(player_killer_patched(&mut e, 0x0089_e790, ACTOR));
    assert_eq!(e.mem.u32(ASKED + 4), ACTOR.addr());
    assert_eq!(e.mem.u32(ASKED + 8), 0);
    assert!(player_killer_patched(&mut e, 0x0089_f351, ACTOR));
    assert_eq!(e.mem.u32(ASKED + 8), ACTOR.addr());
}
