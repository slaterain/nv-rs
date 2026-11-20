//! Group GC, part 4: G84, G85, G99, G126 and their five commands.

use super::*;
use crate::test_support::{call, engine, execute_of, int_arg};

const B: u32 = 0x0570_0000;
const PLAYER: u32 = B + 0x100;
const ACTOR: u32 = B + 0x1000;
const ENTRY: u32 = B + 0x1100;
const ARMOR: u32 = B + 0x1200;
const HIT: u32 = B + 0x1300;
/// Where the stand-ins of `009508b0` and `0x4459E0` log their words.
const LOG: u32 = B + 0x2000;

fn world_as(installed: bool) -> Engine {
    let mut e = engine();
    e.map(B, 0x1_0000);
    e.register(PLAYER_DAMAGE_EQUIPMENT, |e, a| {
        let n = e.mem.u32(LOG);
        e.mem.set_u32(LOG, n + 1);
        for (i, w) in a.iter().take(4).enumerate() {
            e.mem.set_u32(LOG + 4 + 4 * i as u32, *w);
        }
        Ret::default()
    });
    e.register(ENTRY_DELETE, |e, a| {
        let n = e.mem.u32(LOG + 0x20);
        e.mem.set_u32(LOG + 0x20, n + 1);
        e.mem.set_u32(LOG + 0x24, a[0]);
        e.mem.set_u32(LOG + 0x28, a[1]);
        // The order: DamageEquipment's count when the entry went.
        let calls = e.mem.u32(LOG);
        e.mem.set_u32(LOG + 0x2c, calls);
        Ret::default()
    });
    crate::install_as(&mut e, installed);
    e
}

fn float_arg(v: f64) -> Vec<u8> {
    let mut b = vec![b'z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

fn g84(e: &mut Engine, health: f32) -> f32 {
    e.call(CONDITION_DAMAGE.addr(), &[health.to_bits()]).f32()
}

fn g85(e: &mut Engine, word: u32, health: f32) -> f32 {
    e.call(ARMOR_RATING.addr(), &[word, health.to_bits()]).f32()
}

/// JIP's single-float factor, worked out the long way (the clamps as plain bounds).
fn factor(threshold: f32, health: f32, penalty: f32) -> f32 {
    let raw = 1.0f32 - (threshold - health) * penalty;
    raw.clamp(0.0, 1.0)
}

/// While JIP answers "not installed": no handler at the four rows, the formulas are the exe's
/// (the stand-ins), the seams answer nothing, the five commands have no handler.
#[test]
fn part4_not_installed_nothing_is_put_in() {
    let mut e = world_as(false);
    for at in [HIT_ARMOR_DAMAGE.addr(), CONTROL_FLAG.addr()] {
        assert!(!e.site_owners(at).contains(&Owner::Jip), "{at:08x}");
        assert_eq!(e.site(at, &[PLAYER, ENTRY, HIT]), None, "{at:08x}");
    }
    assert_eq!(g84(&mut e, 0.4), -1.0);
    assert_eq!(g85(&mut e, 40, 0.25), -1.0);
    for (opcode, _) in COMMANDS.iter() {
        assert_eq!(execute_of(&e, *opcode), 0, "{opcode:04x}");
    }
}

/// Installed with the row off (the vanilla preset): all four are JIP's own, so they are in, each
/// at its row, with the commands.
#[test]
fn part4_installed_row_off_puts_them_in() {
    let mut e = world_as(true);
    super::super::super::set_row(&mut e, false);
    for at in [HIT_ARMOR_DAMAGE.addr(), CONTROL_FLAG.addr()] {
        assert_eq!(e.site_owners(at).to_vec(), vec![Owner::Jip], "{at:08x}");
    }
    assert_eq!(g84(&mut e, 0.9), 1.0, "G84's wrap");
    assert_eq!(g85(&mut e, 40, 0.9), 40.0, "G85's wrap");
    for (opcode, _) in COMMANDS.iter() {
        assert_ne!(execute_of(&e, *opcode), 0, "{opcode:04x}");
    }
}

/// G84: 1 from three quarters of the health up; below, `1 - (0.75 - health) x 0.67` in single
/// floats; clamped to 0 and 1 (a factor JIP's command makes negative, a health far below 0); a NaN
/// health goes on to the formula and comes out 1 (`minss` hands back its second word).
#[test]
fn g84_condition_damage_as_jip() {
    let mut e = world_as(true);
    assert_eq!(g84(&mut e, 0.75), 1.0);
    assert_eq!(g84(&mut e, 0.93), 1.0);
    for health in [0.7f32, 0.41, 0.0, -0.3] {
        assert_eq!(g84(&mut e, health), factor(0.75, health, 0.67), "{health}");
    }
    // Not the exe's double-precision product: 0.41's factor differs in its last bits.
    let exe = (1.0 - (0.75 - f64::from(0.41f32)) * f64::from(0.67f32)) as f32;
    assert_eq!(
        g84(&mut e, 0.41).to_bits(),
        factor(0.75, 0.41, 0.67).to_bits()
    );
    assert!((g84(&mut e, 0.41) - exe).abs() < 1e-6);
    assert_eq!(g84(&mut e, -1.0), 0.0, "clamped at 0");
    assert_eq!(g84(&mut e, f32::NAN), 1.0, "NaN");
    damage::set_damage_penalty(&mut e, -2.5);
    assert_eq!(g84(&mut e, 0.5), 1.0, "clamped at 1");
    damage::set_damage_penalty(&mut e, 1.3);
    assert_eq!(g84(&mut e, 0.5), factor(0.75, 0.5, 1.3));
}

/// G85: the rating (the low 16 bits of its word) as it is from half the health up; below, times
/// `1 - (0.5 - health) x s_condDRDTPenalty` (1.0, or what the command set), clamped.
#[test]
fn g85_armor_rating_as_jip() {
    let mut e = world_as(true);
    assert_eq!(g85(&mut e, 0xabcd_0028, 0.5), 40.0, "the high half ignored");
    assert_eq!(g85(&mut e, 37, 0.83), 37.0);
    assert_eq!(g85(&mut e, 37, 0.23), 37.0 * factor(0.5, 0.23, 1.0));
    assert_eq!(g85(&mut e, 0xffff, 0.1), 65535.0 * factor(0.5, 0.1, 1.0));
    damage::set_condition_penalty(&mut e, 0.37);
    assert_eq!(g85(&mut e, 37, 0.23), 37.0 * factor(0.5, 0.23, 0.37));
    damage::set_condition_penalty(&mut e, 3.0);
    assert_eq!(g85(&mut e, 37, 0.1), 0.0, "clamped at 0");
    damage::set_condition_penalty(&mut e, -1.0);
    assert_eq!(g85(&mut e, 37, 0.1), 37.0, "clamped at 1");
    assert_eq!(g85(&mut e, 37, f32::NAN), 37.0, "NaN");
}

/// An armour of `health` worn (`entry`'s form), a hit doing `damage` to armour.
fn armour_hit(e: &mut Engine, health: i32, damage: f32) {
    e.mem.set_u32(ENTRY + 8, ARMOR);
    e.mem.set_u32(ARMOR + 0x6c, health as u32);
    e.mem.set_f32(HIT + 0x28, damage);
}

/// G99: `PlayerCharacter::DamageEquipment` on the actor with the entry, the smaller of the hit's
/// armour damage and the armour's health times the cap, and 0; then the entry deleted (flag 1);
/// the exe's span skipped (answer 0). The cap is 1.0 by default, set by the command.
#[test]
fn g99_caps_the_armour_damage() {
    let mut e = world_as(true);
    armour_hit(&mut e, 150, 70.0);
    assert_eq!(
        e.site(HIT_ARMOR_DAMAGE.addr(), &[ACTOR, ENTRY, HIT]),
        Some(0)
    );
    assert_eq!(e.mem.u32(LOG), 1);
    assert_eq!(e.mem.u32(LOG + 4), ACTOR);
    assert_eq!(e.mem.u32(LOG + 8), ENTRY);
    assert_eq!(e.mem.f32(LOG + 12), 70.0, "the cap 1.0: the hit's damage");
    assert_eq!(e.mem.u32(LOG + 16), 0);
    assert_eq!(
        (
            e.mem.u32(LOG + 0x20),
            e.mem.u32(LOG + 0x24),
            e.mem.u32(LOG + 0x28)
        ),
        (1, ENTRY, 1),
        "the entry deleted"
    );
    assert_eq!(e.mem.u32(LOG + 0x2c), 1, "after the damage");
    call(&mut e, SET_DAMAGE_TO_ARMOR_MAX_PERCENT, &[float_arg(0.37)]);
    e.site(HIT_ARMOR_DAMAGE.addr(), &[ACTOR, ENTRY, HIT]);
    assert_eq!(e.mem.f32(LOG + 12), 150.0 * 0.37f32, "capped");
    armour_hit(&mut e, 150, 20.25);
    e.site(HIT_ARMOR_DAMAGE.addr(), &[ACTOR, ENTRY, HIT]);
    assert_eq!(e.mem.f32(LOG + 12), 20.25, "under the cap");
    armour_hit(&mut e, -40, 20.25);
    e.site(HIT_ARMOR_DAMAGE.addr(), &[ACTOR, ENTRY, HIT]);
    assert_eq!(e.mem.f32(LOG + 12), -40.0 * 0.37f32, "a signed health");
}

/// G126: the player's control flags' bit 0 (other bits do not count) while the switch is off;
/// 0 while it is on.
#[test]
fn g126_control_flag_as_jip() {
    let mut e = world_as(true);
    let at = CONTROL_FLAG.addr();
    for (flags, bit) in [(0u8, 0u32), (1, 1), (0xfe, 0), (0xff, 1)] {
        e.mem.set_u8(PLAYER + 0x680, flags);
        assert_eq!(e.site(at, &[PLAYER]), Some(bit), "{flags:02x}");
    }
    damage::set_no_movement_combat(&mut e, true);
    assert_eq!(e.site(at, &[PLAYER]), Some(0));
}

/// `GetConditionDamagePenalty` / `SetConditionDamagePenalty`: 0.67 at first; any value is taken
/// (negative too); the setter answers nothing (its result stays 0).
#[test]
fn condition_damage_penalty_commands() {
    let mut e = world_as(true);
    assert_eq!(
        call(&mut e, GET_CONDITION_DAMAGE_PENALTY, &[]),
        (1, f64::from(0.67f32))
    );
    assert_eq!(
        call(&mut e, SET_CONDITION_DAMAGE_PENALTY, &[float_arg(-0.35)]),
        (1, 0.0)
    );
    assert_eq!(damage::damage_penalty(&mut e), -0.35);
    assert_eq!(
        call(&mut e, GET_CONDITION_DAMAGE_PENALTY, &[]),
        (1, f64::from(-0.35f32))
    );
    assert_eq!(g84(&mut e, 0.5), factor(0.75, 0.5, -0.35));
}

/// `ToggleNoMovementCombat [on]`: answers 1 while on (0 off); an argument sets it (any value but
/// 0 is on); no argument only answers.
#[test]
fn toggle_no_movement_combat_command() {
    let mut e = world_as(true);
    assert_eq!(call(&mut e, TOGGLE_NO_MOVEMENT_COMBAT, &[]), (1, 0.0));
    assert!(!damage::no_movement_combat(&mut e));
    assert_eq!(
        call(&mut e, TOGGLE_NO_MOVEMENT_COMBAT, &[int_arg(7)]),
        (1, 0.0)
    );
    assert!(damage::no_movement_combat(&mut e));
    assert_eq!(call(&mut e, TOGGLE_NO_MOVEMENT_COMBAT, &[]), (1, 1.0));
    assert!(damage::no_movement_combat(&mut e), "no argument: kept");
    assert_eq!(
        call(&mut e, TOGGLE_NO_MOVEMENT_COMBAT, &[int_arg(0)]),
        (1, 1.0)
    );
    assert!(!damage::no_movement_combat(&mut e));
}

/// `SetArmorConditionPenalty [value]`: answers the factor before (1.0 at first); an argument sets
/// it (any value), which G85 and O12 then use; no argument only answers.
#[test]
fn set_armor_condition_penalty_command() {
    let mut e = world_as(true);
    assert_eq!(call(&mut e, SET_ARMOR_CONDITION_PENALTY, &[]), (1, 1.0));
    assert_eq!(
        call(&mut e, SET_ARMOR_CONDITION_PENALTY, &[float_arg(0.37)]),
        (1, 1.0)
    );
    assert_eq!(damage::condition_penalty(&mut e), 0.37);
    assert_eq!(
        call(&mut e, SET_ARMOR_CONDITION_PENALTY, &[float_arg(-4.0)]),
        (1, f64::from(0.37f32))
    );
    assert_eq!(damage::condition_penalty(&mut e), -4.0);
    assert_eq!(call(&mut e, SET_ARMOR_CONDITION_PENALTY, &[]), (1, -4.0));
}

/// `SetDamageToArmorMaxPercent [value]`: answers the cap before (1.0); a value from 0 to 1 (both
/// ends) becomes the co-save's cap and G99's copy at once; one outside is ignored.
#[test]
fn set_damage_to_armor_max_percent_command() {
    let mut e = world_as(true);
    let saved =
        |e: &mut Engine| f32::from_bits(e.ext::<Jip>().saved.serialized.dmg_armor_max_percent);
    assert_eq!(call(&mut e, SET_DAMAGE_TO_ARMOR_MAX_PERCENT, &[]), (1, 1.0));
    for (value, kept) in [
        (0.37, 0.37f32),
        (1.5, 0.37),
        (-0.1, 0.37),
        (0.0, 0.0),
        (1.0, 1.0),
    ] {
        call(&mut e, SET_DAMAGE_TO_ARMOR_MAX_PERCENT, &[float_arg(value)]);
        assert_eq!(saved(&mut e), kept, "{value}");
        assert_eq!(armor_damage_cap(&mut e), kept, "G99's copy {value}");
    }
    call(&mut e, SET_DAMAGE_TO_ARMOR_MAX_PERCENT, &[float_arg(0.37)]);
    assert_eq!(
        call(&mut e, SET_DAMAGE_TO_ARMOR_MAX_PERCENT, &[]),
        (1, f64::from(0.37f32))
    );
}
