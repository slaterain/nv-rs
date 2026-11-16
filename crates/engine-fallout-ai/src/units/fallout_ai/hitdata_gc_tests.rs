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

/// Where G77's call-site handler counts its asks, the hit, the player and the setting cell.
const ASKS: u32 = BASE + 4;
const HIT2: Ptr<HitData> = Ptr::new(BASE + 0x200);
const PLAYER_AT: u32 = BASE + 0x800;
const SETTING_CELL: u32 = BASE + 0x900;

/// The end of `ReduceDamage` with no weapon, no fatigue damage and a hit neither by nor on the
/// player, the row on and a G77 handler that marks the hit as JIP does and counts its asks.
fn finish_world() -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.map(ZERO_01012060 & !0xfff, 0x1000);
    e.map(PLAYER_011DEA3C & !0xfff, 0x1000);
    e.mem.set_u32(PLAYER_011DEA3C, PLAYER_AT);
    e.register(SETTING_VALUE, |_, _| SETTING_CELL.into_ret());
    e.register(0x0040_ebd0, |_, _| Ret::default());
    e.set_jip_patches(jip_patches::FIXES);
    e.on_site(SiteSlot::new(Owner::Jip, 0x009b_6335), |e, a| {
        e.mem.set_u32(ASKS, e.mem.u32(ASKS) + 1);
        let top = e.mem.u8(a[0] + 0x5b);
        e.mem.set_u8(a[0] + 0x5b, top | 0x80);
        Some(0)
    });
    e
}

/// G77 at `0x9B6335` sits on the minimum damage's store: a damage below the minimum is raised to
/// it and the hit marked armour-penetrated; a damage at or above the minimum, or a NaN, is left
/// as it is and the seam not asked.
#[test]
fn g77_seam_sits_on_the_minimum_damage_store() {
    let minimum = 8.0f32.to_bits();
    for (damage, raised) in [
        (5.0f32, true),
        (9.0, false),
        (8.0, false),
        (f32::NAN, false),
    ] {
        let mut e = finish_world();
        e.mem.set_u32(ASKS, 0);
        e.set(HIT2, HitData::fHealthDamage, damage.to_bits());
        reduce_damage_finish(&mut e, HIT2, minimum, 0, false);
        let health = e.get(HIT2, HitData::fHealthDamage);
        let marked = e.mem.u8(HIT2.addr() + 0x5b) & 0x80 != 0;
        if raised {
            assert_eq!((health, marked, e.mem.u32(ASKS)), (minimum, true, 1));
        } else {
            assert_eq!(
                (health, marked, e.mem.u32(ASKS)),
                (damage.to_bits(), false, 0),
                "{damage}"
            );
        }
    }
}
