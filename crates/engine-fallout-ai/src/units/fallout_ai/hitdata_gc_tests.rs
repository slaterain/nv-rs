//! JIP LN NVSE's game patches of group GC (step 3.9) in this file: G77's seam.

use super::*;
use engine_core::extend::{jip_patches, Owner, SiteSlot};

const BASE: u32 = 0x0534_0000;
const HIT: Ptr<HitData> = Ptr::new(BASE + 0x100);

fn world() -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.on_site(SiteSlot::new(Owner::Jip, 0x009b_6335), |e, a| {
        e.mem.set_u32(BASE, a[0]);
        None
    });
    e
}

/// With JIP not installed or the row off the seam is not asked; with the row on it is asked
/// with the hit.
#[test]
fn g77_seam_asks_only_with_the_row() {
    for word in [0, jip_patches::INSTALLED] {
        let mut e = world();
        e.set_jip_patches(word);
        armor_penetrated_seam(&mut e, HIT);
        assert_eq!(e.mem.u32(BASE), 0, "{word:08x}");
    }
    let mut e = world();
    e.set_jip_patches(jip_patches::FIXES);
    armor_penetrated_seam(&mut e, HIT);
    assert_eq!(e.mem.u32(BASE), HIT.addr());
}
