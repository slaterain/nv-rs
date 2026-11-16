//! JIP LN NVSE's game patches of group GC (step 3.9) in this file: O19's seam.

use super::*;
use engine_core::extend::{jip_patches, Owner, SiteSlot};

const BASE: u32 = 0x0532_0000;
const ACTOR: Ptr<Actor> = Ptr::new(BASE + 0x100);
const ASKED: u32 = BASE + 0x800;
const EXE_CALLS: u32 = BASE + 0x900;

fn world() -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.register(0x008b_bc10, |e, _| {
        let n = e.mem.u32(EXE_CALLS);
        e.mem.set_u32(EXE_CALLS, n + 1);
        1u32.into_ret()
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x008b_0fbf), |e, a| {
        e.mem.set_u32(ASKED, a[0]);
        Some(0x100)
    });
    e
}

/// O19 is a switch: with the word 0, with the row's bit alone (switch off) or `INSTALLED`, the
/// exe's iron sights test runs and the seam is not asked.
#[test]
fn o19_seam_does_nothing_while_its_switch_is_off() {
    for word in [
        0,
        jip_patches::INSTALLED,
        jip_patches::FIXES | jip_patches::FO3_WPN_DEGRADATION,
    ] {
        let mut e = world();
        e.set_jip_patches(word);
        assert!(gun_skill_iron_sights(&mut e, ACTOR));
        assert_eq!(e.mem.u32(EXE_CALLS), 1);
        assert_eq!(e.mem.u32(ASKED), 0, "{word:08x}");
    }
}

/// With the switch on: the seam asked with the actor, its answer's low byte the result (`0x100`
/// is false), the exe's test not called.
#[test]
fn o19_seam_takes_the_answer_when_on() {
    let mut e = world();
    e.set_jip_patches(jip_patches::FIXES | jip_patches::CREATURE_SPREAD_FIX);
    assert!(!gun_skill_iron_sights(&mut e, ACTOR));
    assert_eq!(e.mem.u32(EXE_CALLS), 0);
    assert_eq!(e.mem.u32(ASKED), ACTOR.addr());
}
