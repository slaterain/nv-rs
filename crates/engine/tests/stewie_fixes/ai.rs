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

/// S0214 (`CombatItemCooldownFix`, `CombatProcedureUseCombatItem::Update` `009daab0`): an NPC
/// drinks its restore item (slot 0 of the combat state's items). With the exe's defaults
/// (`fCombatItemRestoreTimer` 30.0, `fCombatItemBuffTimer` 45.0) and the timer clock at 12.0, the
/// restore timer is restarted at 12.0 with a cooldown of 45.0 off (the exe's switch falls through
/// to the buff timer, `0099a840` and the timer reset `008d7f40` translated) and 30.0 on (Tweaks'
/// routine).
#[test]
fn s0214_an_npcs_restore_item_waits_the_restore_timer() {
    const RESTORE: u32 = 0x011c_fdd8;
    const BUFF: u32 = 0x011c_f480;
    const CLOCK: u32 = 0x011f_1bf0;
    for (on, cooldown) in [(false, 45.0f32), (true, 30.0)] {
        let mut e = Engine::new();
        e.map(0x011c_f000, 0x1000);
        e.map(0x011f_1000, 0x1000);
        e.mem.set_f32(RESTORE + 4, 30.0);
        e.mem.set_f32(BUFF + 4, 45.0);
        e.mem.set_f32(CLOCK, 12.0);
        let attacker = object(&mut e, 0x200);
        let process = object(&mut e, 0x100);
        e.mem.set_u32(attacker + 0x68, process);
        // Not animating (the process's word +0x28 set): the item is used at once.
        e.mem.set_u32(process + 0x28, 1);
        let state = e.mem.alloc(0x22c);
        e.mem.set_u32(state + 0x1a4, 0x66);
        let procedure = e.mem.alloc(0x18);
        e.mem.set_u32(procedure + 0x10, 0x66);
        e.register_double(0x0099_6de0, move |_, _| ret(attacker));
        e.register_double(0x0058_6150, move |_, _| ret(state));
        e.register_double(0x008d_8520, move |_, _| ret(process));
        e.register(0x0045_cd60, |_, _| ret(1));
        e.register(0x0040_1170, |_, _| ret(0x2f));
        for stand_in in [0x008c_1f80u32, 0x0097_f450, 0x009c_aae0] {
            e.register(stand_in, |_, _| Ret::default());
        }
        e.set_stewie(stewie::COMBAT_ITEM_COOLDOWN, on);
        e.call(0x009d_aab0, &args![procedure]);
        assert_eq!(
            (e.mem.f32(state + 0x1ac), e.mem.f32(state + 0x1b0)),
            (12.0, cooldown),
            "on {on}"
        );
        assert_eq!(
            e.mem.u32(state + 0x1b4),
            0,
            "the buff slot's timer is untouched"
        );
    }
}

/// S0228 (`PatrolPackageFix`, `009f2a70`): a patrol package's data with no actor whose reference
/// list was freed with the invalidated package (its buffer unmapped). Off, the exe's destructor
/// releases the current reference (`009f2b30` translated), which reads the freed list and stops;
/// on, Tweaks' `ClearReservedPatrolRef` skips it and the destructor finishes.
#[test]
fn s0228_an_invalidated_patrol_package_unloads() {
    for on in [false, true] {
        let mut e = Engine::new();
        e.map(0x011c_3000, 0x1000);
        e.mem.set_u32(0x011c_3f2c, 0x900);
        e.register(0x0042_26e0, |_, _| Ret::default());
        for stand_in in [0x006f_2290u32, 0x005e_04f0, 0x0067_d970] {
            e.register(stand_in, |_, _| Ret::default());
        }
        let data = e.mem.alloc(0x38);
        // `References` (+0x1C): a buffer the freed package left behind, one entry.
        e.mem.set_u32(data + 0x20, 0x7ff0_0000);
        e.mem.set_u32(data + 0x24, 1);
        e.set_stewie(stewie::PATROL_PACKAGE, on);
        let unloaded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.call(0x009f_2a70, &args![data]);
        }));
        e.call_stack.clear();
        assert_eq!(unloaded.is_ok(), on, "on {on}");
        if on {
            assert_eq!(e.mem.u32(data + 4), 4, "eState");
        }
    }
}
