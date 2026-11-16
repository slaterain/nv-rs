//! JIP LN NVSE's game patches of group GC (step 3.9) in this file: G14's and G120's seams.

use super::*;
use engine_core::extend::{jip_patches, Owner, SiteSlot};

const BASE: u32 = 0x0531_0000;
const HIT: u32 = BASE + 0x100;
/// Where the test handlers write what they were asked with.
const ASKED: u32 = BASE + 0x800;

fn world() -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.on_site(SiteSlot::new(Owner::Jip, 0x0088_dcdc), |e, _| {
        e.mem.set_u32(ASKED, 1);
        Some(0x100)
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x0088_ebaa), |e, a| {
        e.mem.set_u32(ASKED + 4, a[0]);
        e.mem.set_u32(ASKED + 8, a[1]);
        Some(0x0bad_0000)
    });
    e
}

/// With JIP not installed or installed with the row off: G14's word is the exe's 1 and G120's
/// seam answers nothing (the exe's calls run), neither handler asked.
#[test]
fn gc_seams_do_nothing_while_off() {
    for word in [0, jip_patches::INSTALLED] {
        let mut e = world();
        e.set_jip_patches(word);
        assert_eq!(worn_take_off_word(&mut e), 1);
        assert_eq!(hit_blood_impact(&mut e, HIT, 7), None);
        assert_eq!(e.mem.u32(ASKED) | e.mem.u32(ASKED + 4), 0, "{word:08x}");
    }
}

/// With the row on: G14's word is the answer's low byte; G120's seam is asked with the hit and
/// the material and its answer passed on whole.
#[test]
fn gc_seams_take_the_answers_when_on() {
    let mut e = world();
    e.set_jip_patches(jip_patches::FIXES);
    assert_eq!(worn_take_off_word(&mut e), 0);
    assert_eq!(e.mem.u32(ASKED), 1);
    assert_eq!(hit_blood_impact(&mut e, HIT, 7), Some(0x0bad_0000));
    assert_eq!((e.mem.u32(ASKED + 4), e.mem.u32(ASKED + 8)), (HIT, 7));
}
