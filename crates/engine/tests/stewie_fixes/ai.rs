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

/// Maps the exe page holding `addr` unless it is mapped already.
fn page(e: &mut Engine, addr: u32) {
    if !e.mem.is_mapped(addr) {
        e.map(addr & !0xfff, 0x1000);
    }
}

/// Sets a float game setting's value (the word at +4 that the translated `00403e20` answers).
fn game_setting(e: &mut Engine, setting: u32, value: f32) {
    page(e, setting);
    e.mem.set_f32(setting + 4, value);
}

/// Sets one of the exe's double constants to the value the exe holds.
fn constant(e: &mut Engine, addr: u32, value: f64) {
    page(e, addr);
    e.set_global(addr, value);
}

/// S0055 (`patchFixZeroSpreadZeroVATSChance`, `CombatFormulas::GetVATSChanceToHit` `00646f70`):
/// a gun (type 3) whose minimum spread is 0.0, aimed at a body part whose own factor is 1, 100
/// units away, the part's bound 60. The exe's settings at their defaults (read from the exe's
/// setting constructors): `fVATSScreenPercentFactor` 1.0, `fVATSSpreadMult` 1.0,
/// `fVATSHitChanceMult` 1.0, `fVATSRangeSpreadMax` 2.0, `fVATSMaxChance` 95.0; their values come
/// through the translated `00403e20`, the spread through `00524b80` and the gun tests through
/// `004c0bf0` and `004c0c60`. Off, no spread means no circle, the range factor stays 0.0 and the
/// chance is 0%; on, it starts at `fVATSRangeSpreadMax`: 2 x 1 x 1 = 2%, answered as 0.02. A gun
/// with a spread of 5 degrees (its range factor about 1.46, under the cap) gets the same chance both
/// ways.
#[test]
fn s0055_a_zero_spread_gun_has_a_vats_chance() {
    let mut spread_chances = vec![];
    for (on, min_spread) in [(false, 0.0f32), (true, 0.0), (false, 5.0), (true, 5.0)] {
        let mut e = Engine::new();
        constant(&mut e, 0x0101_2060, 0.0);
        constant(&mut e, 0x0101_2070, 1.0);
        constant(&mut e, 0x0101_7a40, 100.0);
        constant(&mut e, 0x0101_ff40, f64::from_bits(0x4009_21fb_6000_0000));
        constant(&mut e, 0x0102_1928, 3.0);
        constant(&mut e, 0x0102_3128, f64::from_bits(0x3f91_df46_a000_0000));
        page(&mut e, 0x0101_6410);
        e.set_global(0x0101_6410, 100.0f32);
        game_setting(&mut e, 0x011c_fab0, 1.0); // fVATSScreenPercentFactor
        game_setting(&mut e, 0x011c_fe08, 1.0); // fVATSSpreadMult
        game_setting(&mut e, 0x011c_f504, 1.0); // fVATSHitChanceMult
        game_setting(&mut e, 0x011c_fe50, 2.0); // fVATSRangeSpreadMax
        game_setting(&mut e, 0x011c_e10c, 95.0); // fVATSMaxChance
        let shooter = object(&mut e, 0x200);
        let target = object(&mut e, 0x200);
        // The target is no actor (slot +0x100) and no grenade (slot +0x224).
        e.register(SLOT_FNS + 0x100, |_, _| ret(0));
        e.register(SLOT_FNS + 0x224, |_, _| ret(0));
        let weapon = e.mem.alloc(0x200);
        e.mem.set_u8(weapon + 0xf4, 3);
        e.mem.set_f32(weapon + 0x104, min_spread);
        // The shooter has no ammunition equipped (`00525980`), so no projectile to ask.
        e.register(0x0052_5980, |_, _| ret(0));
        e.set_stewie(stewie::ZERO_SPREAD_VATS_CHANCE, on);
        let words = args![
            shooter, target, weapon, 0u32, 0u32, 0.0f32, 0u32, 0u32, 1.0f32, 100.0f32, 60.0f32,
            0u32
        ];
        let chance = e.call(0x0064_6f70, &words).f32();
        if min_spread == 0.0 {
            assert_eq!(chance, if on { 0.02 } else { 0.0 }, "on {on}");
        } else {
            spread_chances.push(chance);
        }
    }
    assert_eq!(spread_chances[0], spread_chances[1]);
    assert!(
        spread_chances[0] > 0.0 && spread_chances[0] < 0.02,
        "{spread_chances:?}"
    );
}

static EQUIPPED: AtomicU32 = AtomicU32::new(0);

/// S0189.01 and S0189.02 (`InventoryCalcWeaponDamageUseAmmoFix`, `CalcWeaponDamageForDisplay`
/// `006450f0` and `CalcWeaponDamage` `00644ce0`): the damage a menu shows for a gun (type 3,
/// damage 20, one projectile) listed with an ammunition whose projectile explodes for 25, while
/// the player (Guns 50) has a plain round equipped (its projectile has no explosion). The exe's
/// settings at their defaults: `fDamageWeaponMult` 1.0, `fDamageSkillBase` 0.2,
/// `fDamageSkillMult` 1.8, `fDamageGunWeapCondMult` 1.0, `fDamageGunWeapCondBase` 0.0; full
/// condition. The projectile lookups (`00525ae0`, `00525a90`, `008d80e0`), the explosion
/// (`004fd3c0`, `006479e0`, `006a78f0`), the projectile count (`00525b20`) and the condition
/// factor (`00646d00`) run translated. Off, the shown damage takes the equipped round's
/// projectile: 20 x 1.1 = 22; with both rows the listed ammunition's: (20 + 25) x 1.1 = 49.5.
/// Either row alone shows 22: S0189.01 passes the ammunition nobody reads, S0189.02 gets none.
#[test]
fn s0189_the_shown_damage_counts_the_listed_ammunitions_explosion() {
    let mut shown = vec![];
    for (one, two) in [(false, false), (true, true), (true, false), (false, true)] {
        let mut e = Engine::new();
        constant(&mut e, 0x0101_2060, 0.0);
        constant(&mut e, 0x0101_2070, 1.0);
        constant(&mut e, 0x0101_6408, f64::from_bits(0x3f84_7ae1_4000_0000));
        constant(&mut e, 0x0101_de30, 0.75);
        constant(&mut e, 0x0105_1680, f64::from_bits(0x3fe5_70a3_e000_0000));
        e.set_global(0x0101_6410, 100.0f32);
        game_setting(&mut e, 0x011c_e2c0, 1.0); // fDamageWeaponMult
        game_setting(&mut e, 0x011c_ece8, f32::from_bits(0x3e4c_cccd)); // fDamageSkillBase 0.2
        game_setting(&mut e, 0x011c_f350, f32::from_bits(0x3fe6_6666)); // fDamageSkillMult 1.8
        game_setting(&mut e, 0x011c_f264, 1.0); // fDamageGunWeapCondMult
        game_setting(&mut e, 0x011c_fb5c, 0.0); // fDamageGunWeapCondBase
        let player = object(&mut e, 0x400);
        page(&mut e, 0x011d_ea3c);
        e.mem.set_u32(0x011d_ea3c, player);
        // A gun: type 3, skill (+0x15C) Guns, damage 20 (its damage form's slot +0x10), one
        // projectile (+0x11E), flags clear.
        let weapon = e.mem.alloc(0x200);
        e.mem.set_u8(weapon + 0xf4, 3);
        e.mem.set_u32(weapon + 0x15c, 0x29);
        e.mem.set_u8(weapon + 0x11e, 1);
        let damage_form = object(&mut e, 0x10);
        let damage_vtable = e.mem.u32(damage_form);
        e.mem.set_u32(weapon + 0x9c, damage_vtable);
        e.register(SLOT_FNS + 0x10, |_, _| ret(20));
        // The equipped round (`00525980`): its projectile (+0xB4) has no explosion (+0x84).
        let plain = e.mem.alloc(0x100);
        let equipped = e.mem.alloc(0x100);
        e.mem.set_u8(equipped + 4, 0x29);
        e.mem.set_u32(equipped + 0xb4, plain);
        EQUIPPED.store(equipped, Relaxed);
        e.register(0x0052_5980, |_, _| ret(EQUIPPED.load(Relaxed)));
        // The listed ammunition (type 0x29): its projectile's explosion does 25 (+0x78).
        let explosion = e.mem.alloc(0x100);
        e.mem.set_f32(explosion + 0x78, 25.0);
        let exploding = e.mem.alloc(0x100);
        e.mem.set_u32(exploding + 0x84, explosion);
        let listed = e.mem.alloc(0x100);
        e.mem.set_u8(listed + 4, 0x29);
        e.mem.set_u32(listed + 0xb4, exploding);
        // The player's Guns skill (`0066ef50`) is 50.
        e.register(0x0066_ef50, |_, _| st0(50.0));
        // Not the actor cast (the perk and teammate factors stay 1), no ammunition effects, no
        // perks; the damage figure's own result is not used.
        e.register(0x00ec_43fb, |_, _| ret(0));
        e.register(0x0064_5380, |_, _| Ret::default());
        e.register(0x0092_3000, |_, _| ret(0));
        e.register(0x0059_a030, |_, a| st0(f64::from(f32::from_bits(a[2]))));
        e.register(0x005e_58f0, |_, _| Ret::default());
        e.set_stewie(stewie::WEAPON_DAMAGE_DISPLAY_AMMO, one);
        e.set_stewie(stewie::WEAPON_DAMAGE_AMMO_PROJECTILE, two);
        let words = args![weapon, 1.0f32, 0u32, listed];
        shown.push(e.call(0x0064_50f0, &words).f32());
    }
    let factor = (50.0f64 * f64::from_bits(0x3f84_7ae1_4000_0000) * f64::from(1.8f32)
        + f64::from(0.2f32)) as f32;
    let off = (20.0 * f64::from(factor)) as f32;
    let on = (45.0 * f64::from(factor)) as f32;
    assert_eq!(shown, vec![off, on, off, off]);
    assert!(
        (off - 22.0).abs() < 1e-4 && (on - 49.5).abs() < 1e-4,
        "{off} {on}"
    );
}
