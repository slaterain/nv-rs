//! Group GW's game patches: G15, G19, G74 (with `SetWindSpeedMult`), G89, G94, G104 and H17 (with
//! `ForceClimate`).

use super::*;
use crate::test_support::{call, engine, execute_of, form_arg, script_ref};
use engine_core::extend::jip_patches;

const B: u32 = 0x0590_0000;
const LOG: u32 = B + 0xf000;
const WEATHER: u32 = B + 0x100;
const SECTION: u32 = B + 0x200;
const ACTOR: u32 = B + 0x300;
const ACTOR_VTABLE: u32 = B + 0x400;
const PROCESS_AT: u32 = B + 0x800;
const ENTRY: u32 = B + 0x900;
const LISTS: u32 = B + 0x980;
const LIST: u32 = B + 0x9c0;
const WEAPON: u32 = B + 0xa00;
const EXTRA: u32 = B + 0xc00;
const CLIMATE: u32 = B + 0x1000;
const OTHER_CLIMATE: u32 = B + 0x1100;
const WEATHER_FORM: u32 = B + 0x1200;
const SKY_AT: u32 = B + 0x2000;

/// The compiled double argument `z`.
fn z(v: f64) -> Vec<u8> {
    let mut b = vec![b'z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// JIP installed (or not) with the row on or off, and the stand-ins this file's handlers call:
/// `BaseExtraList::GetByType` answers `EXTRA` for `ExtraWeaponModFlags` on `LIST` (logged), the
/// acoustic space change is logged.
fn world(installed: bool, row: bool) -> Engine {
    let mut e = engine();
    e.map(B, 0x1_0000);
    for page in [0x011d_c000, 0x011e_0000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.register(GET_BY_TYPE, |e, a| {
        let n = e.mem.u32(LOG);
        e.mem.set_u32(LOG, n + 1);
        e.mem.set_u32(LOG + 4, a[0]);
        e.mem.set_u32(LOG + 8, a[1]);
        let found = a[0] == LIST && a[1] == EXTRA_WEAPON_MOD_FLAGS && e.mem.u32(LOG + 0x40) != 0;
        Ret {
            eax: if found { EXTRA } else { 0 },
            ..Ret::default()
        }
    });
    e.register(SET_ACOUSTIC_SPACE, |e, a| {
        let n = e.mem.u32(LOG + 0x20);
        e.mem.set_u32(LOG + 0x20, n + 1);
        e.mem.set_u32(LOG + 0x24, a[0]);
        Ret::default()
    });
    // The settings read when the row goes on: no file (JIP's defaults).
    e.register(0x00fd_f088, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    if !e.is_registered(0x0061_a310) {
        e.register(0x0061_a310, |_, _| Ret::default());
    }
    crate::install_as(&mut e, installed);
    if installed {
        super::super::set_row(&mut e, row);
    }
    e
}

/// Not installed: no handler at the rows, the commands have none.
#[test]
fn gw_not_installed_nothing_is_put_in() {
    let mut e = world(false, false);
    for addr in AREA.fix_sites.iter().chain(AREA.jip_sites) {
        assert!(!e.site_owners(*addr).contains(&Owner::Jip), "{addr:08x}");
        assert_eq!(e.site(*addr, &[WEATHER, 0, 0]), None, "{addr:08x}");
    }
    assert_eq!(execute_of(&e, SET_WIND_SPEED_MULT), 0);
    assert_eq!(execute_of(&e, FORCE_CLIMATE), 0);
}

/// Installed with the row off: H17 (JIP's own) is in, with both commands; the fixes are not.
/// With the row: every fix.
#[test]
fn gw_installed_row_off_puts_only_jips_own_in() {
    let mut e = world(true, false);
    for addr in AREA.jip_sites {
        assert_eq!(
            e.site_owners(*addr).to_vec(),
            vec![Owner::Jip],
            "{addr:08x}"
        );
    }
    for addr in AREA.fix_sites {
        assert_eq!(e.site(*addr, &[WEATHER, 0, 0]), None, "{addr:08x}");
    }
    assert_ne!(execute_of(&e, SET_WIND_SPEED_MULT), 0);
    assert_ne!(execute_of(&e, FORCE_CLIMATE), 0);
    let e = world(true, true);
    for addr in AREA.fix_sites {
        assert_eq!(
            e.site_owners(*addr).to_vec(),
            vec![Owner::Jip],
            "{addr:08x}"
        );
    }
}

/// G15 answers entry 1; G19 the swapped pairs (the first form in the low byte).
#[test]
fn g15_and_g19_answer_jips_bytes() {
    let mut e = world(true, true);
    assert_eq!(e.site(SUN_GLARE.addr(), &[]), Some(1));
    assert_eq!(e.site(NOON_MORNING.addr(), &[]), Some(0x0104));
    assert_eq!(e.site(NOON_AFTERNOON.addr(), &[]), Some(0x0401));
}

/// G74: the weather's byte `+0xE0` times 1/255 in `float` (the other bytes of the table not
/// read), at both sites; after `SetWindSpeedMult 2` twice that, after `SetWindSpeedMult 0` 0.
#[test]
fn g74_wind_speed_is_the_byte_times_the_factor() {
    let mut e = world(true, true);
    let speed = |e: &mut Engine, site: u32| f32::from_bits(e.site(site, &[WEATHER]).unwrap());
    for byte in [0u8, 1, 51, 128, 255] {
        e.mem.set_u8(WEATHER + 0xe0, byte);
        e.mem.set_u8(WEATHER + 0xe1, 0x77);
        e.mem.set_u8(WEATHER + 0xdf, 0x77);
        let want = (f64::from(byte) * f64::from(1.0f32 / 255.0)) as f32;
        assert_eq!(speed(&mut e, WIND_SPEED.addr()), want, "{byte}");
        assert_eq!(speed(&mut e, WIND_SPEED_LAST.addr()), want, "{byte}");
    }
    e.mem.set_u8(WEATHER + 0xe0, 51);
    assert_eq!(call(&mut e, SET_WIND_SPEED_MULT, &[z(2.0)]).0, 1);
    let factor = 2.0f32 * (1.0f32 / 255.0);
    assert_eq!(
        speed(&mut e, WIND_SPEED.addr()),
        (51.0 * f64::from(factor)) as f32
    );
    call(&mut e, SET_WIND_SPEED_MULT, &[z(0.0)]);
    assert_eq!(speed(&mut e, WIND_SPEED.addr()), 0.0);
    call(&mut e, SET_WIND_SPEED_MULT, &[z(1.0)]);
    assert_eq!(
        speed(&mut e, WIND_SPEED.addr()),
        (51.0 * f64::from(1.0f32 / 255.0)) as f32
    );
}

/// G89: the word `+0x26` cleared, its neighbours kept.
#[test]
fn g89_clears_the_fully_seen_word() {
    let mut e = world(true, true);
    e.mem.set_u32(SECTION + 0x24, 0xaabb_ccdd);
    e.mem.set_u32(SECTION + 0x28, 0x1122_3344);
    assert_eq!(e.site(FULLY_SEEN.addr(), &[SECTION]), Some(0));
    assert_eq!(e.mem.u32(SECTION + 0x24), 0x0000_ccdd);
    assert_eq!(e.mem.u32(SECTION + 0x28), 0x1122_3344);
}

/// An actor (its `IsActor` slot `ReturnTrue` or not) with a process of `level` holding the
/// weapon entry, the weapon's three mod effects, the mod flags `flags` (`None`: no
/// `ExtraWeaponModFlags`).
fn silenced_world(e: &mut Engine, actor: bool, level: u8, effects: [u8; 3], flags: Option<u8>) {
    e.mem.set_u32(ACTOR, ACTOR_VTABLE);
    let slot = if actor { RETURN_TRUE } else { 0x008d_0370 };
    e.mem.set_u32(ACTOR_VTABLE + IS_ACTOR_SLOT, slot);
    e.mem.set_u32(ACTOR + PROCESS, PROCESS_AT);
    e.mem.set_u8(PROCESS_AT + PROCESS_LEVEL, level);
    e.mem.set_u32(PROCESS_AT + HELD_WEAPON, ENTRY);
    e.mem.set_u32(ENTRY, LISTS);
    e.mem.set_u32(LISTS, LIST);
    e.mem.set_u32(ENTRY + 8, WEAPON);
    for (i, effect) in effects.iter().enumerate() {
        e.mem.set_u8(WEAPON + MOD_EFFECTS + 4 * i as u32, *effect);
    }
    e.mem.set_u32(LOG, 0);
    e.mem.set_u32(LOG + 0x40, u32::from(flags.is_some()));
    e.mem.set_u8(EXTRA + 0xc, flags.unwrap_or(0));
}

/// G94: each mod slot whose flag is set and whose effect is a silencer (`0xB`) or `0x10` makes it
/// silenced; a slot whose flag is clear does not count; other effects do not; not an actor, a
/// process above level 1, no process, no held weapon, no extra list, no mod flags: not silenced.
/// `GetByType` is asked with the entry's first list and `0x8D`.
#[test]
fn g94_silenced_by_a_silencer_mod_of_the_held_weapon() {
    let mut e = world(true, true);
    let ask = |e: &mut Engine| e.site(SILENCED_ATTACK.addr(), &[ACTOR]);
    for slot in 0..3usize {
        for effect in [0xbu8, 0x10] {
            let mut effects = [0u8; 3];
            effects[slot] = effect;
            silenced_world(&mut e, true, 1, effects, Some(1 << slot));
            assert_eq!(ask(&mut e), Some(1), "{slot} {effect:x}");
            assert_eq!((e.mem.u32(LOG + 4), e.mem.u32(LOG + 8)), (LIST, 0x8d));
            silenced_world(&mut e, true, 0, effects, Some(0x7 & !(1 << slot)));
            assert_eq!(ask(&mut e), Some(0), "flag clear {slot}");
        }
        let mut effects = [0u8; 3];
        effects[slot] = 0xc;
        silenced_world(&mut e, true, 0, effects, Some(7));
        assert_eq!(ask(&mut e), Some(0), "another effect {slot}");
    }
    let silencer = [0xb, 0, 0];
    silenced_world(&mut e, false, 0, silencer, Some(1));
    assert_eq!(ask(&mut e), Some(0), "not an actor");
    assert_eq!(e.mem.u32(LOG), 0);
    silenced_world(&mut e, true, 2, silencer, Some(1));
    assert_eq!(ask(&mut e), Some(0), "level 2");
    silenced_world(&mut e, true, 0, silencer, None);
    assert_eq!(ask(&mut e), Some(0), "no mod flags");
    silenced_world(&mut e, true, 0, silencer, Some(1));
    e.mem.set_u32(LISTS, 0);
    assert_eq!(ask(&mut e), Some(0), "no extra list");
    silenced_world(&mut e, true, 0, silencer, Some(1));
    e.mem.set_u32(ENTRY, 0);
    assert_eq!(ask(&mut e), Some(0), "no list head");
    silenced_world(&mut e, true, 0, silencer, Some(1));
    e.mem.set_u32(PROCESS_AT + HELD_WEAPON, 0);
    assert_eq!(ask(&mut e), Some(0), "no weapon held");
    silenced_world(&mut e, true, 0, silencer, Some(1));
    e.mem.set_u32(ACTOR + PROCESS, 0);
    assert_eq!(ask(&mut e), Some(0), "no process");
}

/// G104: the update flag cleared in every case; a set next space cleared and applied unless it
/// is the current one; none: nothing more.
#[test]
fn g104_applies_the_pending_space_once() {
    let mut e = world(true, true);
    let run = |e: &mut Engine, next: u32, current: u32| {
        e.mem.set_u8(UPDATE_ACOUSTIC_SPACE, 1);
        e.mem.set_u32(NEXT_SPACE, next);
        e.mem.set_u32(CURRENT_SPACE, current);
        e.mem.set_u32(LOG + 0x20, 0);
        e.mem.set_u32(LOG + 0x24, 0);
        assert_eq!(e.site(PENDING_SPACE.addr(), &[next]), Some(0));
        (
            e.mem.u8(UPDATE_ACOUSTIC_SPACE),
            e.mem.u32(NEXT_SPACE),
            e.mem.u32(LOG + 0x20),
            e.mem.u32(LOG + 0x24),
        )
    };
    assert_eq!(run(&mut e, 0x4000, 0x5000), (0, 0, 1, 0x4000), "applied");
    assert_eq!(run(&mut e, 0x5000, 0x5000), (0, 0, 0, 0), "the current one");
    assert_eq!(run(&mut e, 0, 0x5000), (0, 0, 0, 0), "none");
}

/// H17 and `ForceClimate`: no answer until a climate is forced; then the forced climate;
/// `ForceClimate` with no argument clears it; another kind of form leaves it; `SetCurrentClimate`
/// to another climate clears it, to the forced one keeps it.
#[test]
fn h17_forces_the_climate_force_climate_sets() {
    let mut e = world(true, false);
    e.mem.set_u8(CLIMATE + 4, 0x36);
    e.mem.set_u8(OTHER_CLIMATE + 4, 0x36);
    e.mem.set_u8(WEATHER_FORM + 4, 0x35);
    e.mem.set_u32(0x011d_ea20, SKY_AT);
    script_ref(&mut e, 1, CLIMATE);
    script_ref(&mut e, 2, OTHER_CLIMATE);
    script_ref(&mut e, 3, WEATHER_FORM);
    let site = |e: &mut Engine| e.site(FORCED_CLIMATE.addr(), &[0x1234]);
    assert_eq!(site(&mut e), None);
    assert_eq!(call(&mut e, FORCE_CLIMATE, &[form_arg(1)]).0, 1);
    assert_eq!(site(&mut e), Some(CLIMATE));
    call(&mut e, FORCE_CLIMATE, &[form_arg(3)]);
    assert_eq!(site(&mut e), Some(CLIMATE), "a weather leaves it");
    call(&mut e, FORCE_CLIMATE, &[]);
    assert_eq!(site(&mut e), None, "cleared");
    call(&mut e, FORCE_CLIMATE, &[form_arg(1)]);
    // SetCurrentClimate (0x26D1) to the forced climate keeps it, to another clears it.
    call(&mut e, 0x26d1, &[form_arg(1)]);
    assert_eq!(site(&mut e), Some(CLIMATE));
    call(&mut e, 0x26d1, &[form_arg(2)]);
    assert_eq!(site(&mut e), None);
    assert_eq!(e.mem.u32(SKY_AT + 0xc), OTHER_CLIMATE);
}
