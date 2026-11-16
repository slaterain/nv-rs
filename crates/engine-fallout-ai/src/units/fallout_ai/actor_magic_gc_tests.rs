//! JIP LN NVSE's game patches of group GC (step 3.9) in this file: G118's seam.

use super::*;
use engine_core::extend::{jip_patches, Owner, SiteSlot};

const BASE: u32 = 0x0533_0000;

fn world() -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.on_site(SiteSlot::new(Owner::Jip, 0x008c_227f), |e, _| {
        e.mem.set_u32(BASE, 1);
        Some(0)
    });
    e
}

/// With JIP not installed or the row off, the `jnz` stays (false) and the seam is not asked; with
/// the row on, any answer (0 included) removes it.
#[test]
fn g118_seam_answers_only_with_the_row() {
    for word in [0, jip_patches::INSTALLED] {
        let mut e = world();
        e.set_jip_patches(word);
        assert!(!projectile_weapon_first(&mut e));
        assert_eq!(e.mem.u32(BASE), 0, "{word:08x}");
    }
    let mut e = world();
    e.set_jip_patches(jip_patches::FIXES);
    assert!(projectile_weapon_first(&mut e));
    assert_eq!(e.mem.u32(BASE), 1);
}
