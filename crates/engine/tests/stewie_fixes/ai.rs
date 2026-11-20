//! Share 2's crash rows (AI, actors and the player): each row's crashing input fed to its
//! translated function, row on and off, as in `stewie_fixes.rs`. Only share 2 edits this file.
//!
//! Share 2's first rows are engine fixes, not crash fixes: for the two that change numbers a player
//! sees, the translated function with every translation linked in, row off (the exe's values) and
//! on (Tweaks' values).

use super::{object, ret};
use engine_core::extend::stewie;
use engine_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// The vtable stand-ins `super::object` points every slot at.
const SLOT_FNS: u32 = 0x00ff_4000;
/// The exe's data page with the double 0.0 (`01012060`) and the float -1.0 (`01012054`).
const DATA_PAGE: u32 = 0x0101_2000;

fn st0(value: f64) -> Ret {
    Ret {
        st0: value,
        ..Ret::default()
    }
}

static MARKED: AtomicU32 = AtomicU32::new(0);

/// S0229 (`ArmorUpdateFix`, `008d4e80`): an NPC's armour changed. Off, its DR and DT go to -1.0 and
/// nothing else; on, the same and its process's cached DR (0x12) and DT (0x4C) are marked out of
/// date (process slot +0x3B0), so the actor's (NPC's or creature's) DR and DT follow the new
/// armour.
#[test]
fn s0229_an_npcs_dr_and_dt_follow_its_armour() {
    for on in [false, true] {
        let mut e = Engine::new();
        e.map(DATA_PAGE, 0x1000);
        e.set_global(0x0101_2054, -1.0f32);
        let actor = e.mem.alloc(0x200);
        let process = object(&mut e, 0x40);
        e.mem.set_u32(actor + 0x68, process);
        e.mem.set_f32(actor + 0x1b8, 30.0);
        e.mem.set_f32(actor + 0x1bc, 12.0);
        MARKED.store(0, Relaxed);
        e.register(SLOT_FNS + 0x3b0, |_, a| {
            assert!(a[1] == 0x12 || a[1] == 0x4c);
            MARKED.fetch_add(1, Relaxed);
            ret(0)
        });
        e.set_stewie(stewie::ARMOR_UPDATE, on);
        e.call(0x008d_4e80, &args![actor]);
        assert_eq!(e.mem.f32(actor + 0x1b8), -1.0);
        assert_eq!(e.mem.f32(actor + 0x1bc), -1.0);
        assert_eq!(MARKED.load(Relaxed), if on { 2 } else { 0 }, "on {on}");
    }
}

/// S0206 (`CreatureDRFix`, `Creature`'s slot `010874E8` = `008d4cc0`): a creature whose base value
/// is 10, permanent value 1, temporary value 2 and damage 0.5 for Damage Resistance (0x12), with no
/// value of its own. Off, the exe's total DR is 10 + 1 + 2 + 0.5 = 13.5; on, Tweaks' is the
/// damage plus the permanent value, 1.5.
#[test]
fn s0206_a_creatures_dr_is_counted_once() {
    for (on, total) in [(false, 13.5), (true, 1.5)] {
        let mut e = Engine::new();
        e.map(DATA_PAGE, 0x1000);
        e.set_global(0x0101_2060, 0.0f64);
        let creature = object(&mut e, 0x200);
        // Its own value (slot +0x48C) leaves the flag clear.
        e.register(SLOT_FNS + 0x48c, |_, _| st0(4.0));
        e.register(0x0087_d880, |_, _| st0(10.0));
        let values = e.mem.alloc(0x40);
        for (slot, value) in [(0x10u32, 2.0), (0x14, 0.5), (0x18, 1.0)] {
            let at = 0x00fe_0000 + slot;
            e.mem.set_u32(values + slot, at);
            e.register_double(at, move |_, _| st0(value));
        }
        e.mem.set_u32(creature + 0xa4, values);
        e.mem.set_f32(creature + 0x1b8, -1.0);
        e.set_stewie(stewie::CREATURE_DR, on);
        assert_eq!(
            e.call(0x008d_4cc0, &args![creature]).f64(),
            total,
            "on {on}"
        );
        assert_eq!(e.mem.f32(creature + 0x1b8), total as f32);
    }
}
