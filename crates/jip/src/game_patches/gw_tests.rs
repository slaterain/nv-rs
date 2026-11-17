//! Group GW's game patches: G15, G19, G74 (with `SetWindSpeedMult`), G89, G94, G104 and H17 (with
//! `ForceClimate`).

use super::*;
use crate::test_support::{call, engine, execute_of, form_arg, script_ref};

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

const CALENDAR: u32 = B + 0x3000;
const PLAYER_AT: u32 = B + 0x4000;

/// The calendar's six globals (year, month, day, hour, days passed, time scale) at `B + 0x3100`
/// on, each value at `+0x24`; the player for the hardcore byte; JIP's kept hour and days reset.
fn calendar(e: &mut Engine, values: [f32; 6]) {
    for (i, v) in values.iter().enumerate() {
        let g = B + 0x3100 + 0x30 * i as u32;
        e.mem.set_u32(CALENDAR + 4 * i as u32, g);
        e.mem.set_f32(g + 0x24, *v);
    }
    e.mem.set_u32(CALENDAR + 0x18, 7);
    e.mem.set_u8(CALENDAR + 0x1c, 0);
    e.mem.set_u32(PLAYER, PLAYER_AT);
    e.mem.set_u8(PLAYER_AT + HARDCORE_UPDATE, 0x55);
    let s = state(e);
    s.hour_and_days[0].store(0, Ordering::Relaxed);
    s.hour_and_days[1].store(0, Ordering::Relaxed);
}

fn value(e: &Engine, i: u32) -> f32 {
    e.mem.f32(B + 0x3100 + 0x30 * i + 0x24)
}

fn advance(e: &mut Engine, seconds: f32) -> u32 {
    e.call(UPDATE_TIME_GLOBALS.addr(), &[CALENDAR, seconds.to_bits()])
        .eax
}

/// O5 without its switch (or without the row): the exe's function (the stand-in's marker).
#[test]
fn o5_without_the_switch_runs_the_exe() {
    let mut e = world(true, true);
    let word = e.jip_patches();
    assert_ne!(word & jip_patches::GAME_DAYS_PASSED_FIX, 0, "on by default");
    calendar(&mut e, [2281.0, 9.0, 19.0, 10.0, 5.5, 30.0]);
    e.set_jip_patches(word & !jip_patches::GAME_DAYS_PASSED_FIX);
    assert_eq!(advance(&mut e, 1.0), 0x5a5a);
    assert_eq!(value(&e, 3), 10.0);
    let mut e = world(true, false);
    calendar(&mut e, [2281.0, 9.0, 19.0, 10.0, 5.5, 30.0]);
    assert_eq!(advance(&mut e, 1.0), 0x5a5a);
}

/// O5's first update counts the days again (the hour moved from JIP's kept 0): whole days plus
/// hour / 24, the player's hardcore byte set, then the hours passed (`1 s x 30 / 3600`) added in
/// doubles. The next update takes JIP's kept doubles (the hour's `float` is within 0.05), so the
/// days do not drift as the exe's `float` sums do.
#[test]
fn o5_keeps_the_hour_and_days_in_doubles() {
    let mut e = world(true, true);
    calendar(&mut e, [2281.0, 9.0, 19.0, 10.0, 5.5, 30.0]);
    let per = f64::from_bits(PER_SECOND);
    let passed = 1.0f64 * 30.0 * per;
    advance(&mut e, 1.0);
    let mut hour = 10.0 + passed;
    let mut days = 10.0 / 24.0 + 5.0 + passed / 24.0;
    assert_eq!(value(&e, 3), hour as f32);
    assert_eq!(value(&e, 4), days as f32);
    assert_eq!(e.mem.u8(PLAYER_AT + HARDCORE_UPDATE), 1);
    assert_eq!(e.mem.u32(CALENDAR + 0x18), 7, "no midnight");
    e.mem.set_u8(PLAYER_AT + HARDCORE_UPDATE, 0x55);
    for _ in 0..1000 {
        advance(&mut e, 1.0);
        hour += passed;
        days += passed / 24.0;
    }
    assert_eq!(value(&e, 3), hour as f32);
    assert_eq!(value(&e, 4), days as f32);
    assert_eq!(e.mem.u8(PLAYER_AT + HARDCORE_UPDATE), 0x55, "no recount");
}

/// O5 recounts when the stale byte is set (and clears it), when the hour jumped ahead past 0.05,
/// and when it went back past 0.05 (counted as the next day: past 24, the day rolls on). With
/// `SetGameHour`'s request the hardcore byte is cleared and the request taken. With no player the
/// recount still happens (no byte written); a NaN hour recounts.
#[test]
fn o5_recounts_after_a_jump() {
    let mut e = world(true, true);
    calendar(&mut e, [2281.0, 9.0, 19.0, 10.0, 5.5, 30.0]);
    advance(&mut e, 0.0);
    // Stale byte: whole days of the global (5) plus the hour again.
    e.mem.set_f32(B + 0x3100 + 0x30 * 4 + 0x24, 7.9);
    e.mem.set_u8(CALENDAR + 0x1c, 1);
    advance(&mut e, 0.0);
    assert_eq!(e.mem.u8(CALENDAR + 0x1c), 0);
    assert_eq!(value(&e, 4), (10.0f64 / 24.0 + 7.0) as f32);
    // Ahead by 2 hours, with SetGameHour's request: the byte cleared, the request taken.
    super::super::gp::set_force_hardcore_needs_update(&mut e, true);
    e.mem.set_f32(B + 0x3100 + 0x30 * 3 + 0x24, 12.0);
    advance(&mut e, 0.0);
    assert_eq!(value(&e, 4), (12.0f64 / 24.0 + 7.0) as f32);
    assert_eq!(e.mem.u8(PLAYER_AT + HARDCORE_UPDATE), 0);
    assert!(!super::super::gp::force_hardcore_needs_update(&mut e));
    // Within 0.05 (0.04 back): JIP's kept hour stays.
    e.mem.set_f32(B + 0x3100 + 0x30 * 3 + 0x24, 11.96);
    advance(&mut e, 0.0);
    assert_eq!(value(&e, 3), 12.0);
    // Back by 2 hours: 10 + 24, so a day on (19 -> 20), the hour 10, one more midnight.
    e.mem.set_f32(B + 0x3100 + 0x30 * 3 + 0x24, 10.0);
    advance(&mut e, 0.0);
    assert_eq!((value(&e, 2), value(&e, 3)), (20.0, 10.0));
    assert_eq!(e.mem.u32(CALENDAR + 0x18), 8);
    assert_eq!(value(&e, 4), (34.0f64 / 24.0 + 7.0) as f32);
    // No player (the port's difference from JIP): with the stale byte set the update returns
    // without a write at `0xE39`, clears the byte and counts the days again (whole 3 + 10 / 24).
    assert!(
        !e.mem.is_mapped(HARDCORE_UPDATE),
        "a write at 0xE39 would fault"
    );
    e.mem.set_u32(PLAYER, 0);
    e.mem.set_f32(B + 0x3100 + 0x30 * 4 + 0x24, 3.9);
    e.mem.set_u8(CALENDAR + 0x1c, 1);
    advance(&mut e, 0.0);
    assert_eq!(
        e.mem.u8(CALENDAR + 0x1c),
        0,
        "no player: the stale byte cleared"
    );
    assert_eq!(
        value(&e, 4),
        (10.0f64 / 24.0 + 3.0) as f32,
        "no player: the days counted again"
    );
    // A NaN hour (`comisd; jb` takes the unordered compare as below): a recount, so the
    // player's hardcore byte is written.
    e.mem.set_u32(PLAYER, PLAYER_AT);
    e.mem.set_u8(PLAYER_AT + HARDCORE_UPDATE, 0x55);
    e.mem.set_f32(B + 0x3100 + 0x30 * 3 + 0x24, f32::NAN);
    advance(&mut e, 0.0);
    assert_eq!(
        e.mem.u8(PLAYER_AT + HARDCORE_UPDATE),
        1,
        "a NaN hour recounts"
    );
}

/// O5's roll-over by JIP's table: the last day of a month goes to the 1st of the next; the 31st
/// of December (month 11) to January 1st of the next year; a 28-day February has no 29th; each
/// update past midnight counts once; a month outside 0..11 reads 31 days and rolls to January.
#[test]
fn o5_rolls_the_days_months_and_years() {
    let mut e = world(true, true);
    let roll = |e: &mut Engine, month: f32, day: f32| {
        calendar(e, [2281.0, month, day, 23.0, 100.0, 30.0]);
        advance(e, 0.0);
        // 23 + 1.5 hours: past midnight once.
        advance(e, 180.0);
        (
            value(e, 0),
            value(e, 1),
            value(e, 2),
            e.mem.u32(CALENDAR + 0x18),
        )
    };
    assert_eq!(roll(&mut e, 9.0, 19.0), (2281.0, 9.0, 20.0, 8));
    assert_eq!(
        roll(&mut e, 9.0, 31.0),
        (2281.0, 10.0, 1.0, 8),
        "October has 31"
    );
    assert_eq!(
        roll(&mut e, 10.0, 30.0),
        (2281.0, 11.0, 1.0, 8),
        "November has 30"
    );
    assert_eq!(roll(&mut e, 11.0, 31.0), (2282.0, 0.0, 1.0, 8));
    assert_eq!(roll(&mut e, 1.0, 28.0), (2281.0, 2.0, 1.0, 8));
    assert_eq!(
        roll(&mut e, 12.0, 31.0),
        (2282.0, 0.0, 1.0, 8),
        "month 12: 31 days, then the year"
    );
    assert_eq!(roll(&mut e, 12.0, 30.0), (2281.0, 12.0, 31.0, 8));
    // Two midnights in one step: 23 + 25.5 hours (1530 s at scale 60).
    calendar(&mut e, [2281.0, 9.0, 19.0, 23.0, 100.0, 60.0]);
    advance(&mut e, 0.0);
    advance(&mut e, 1530.0);
    assert_eq!((value(&e, 2), value(&e, 3)), (21.0, 0.5));
    assert_eq!(e.mem.u32(CALENDAR + 0x18), 8, "counted once per update");
}

// ---- G113, D05, G115 ----

/// What the stand-ins below record: `AddCompileIndex`'s, the random draws' bounds, the frees.
const REC: u32 = B + 0xe000;
/// The random draws the stand-in answers, in order (a count, then the values).
const DRAWS: u32 = B + 0xe800;

fn record(e: &mut Engine, at: u32, words: &[u32]) {
    let n = e.mem.u32(at);
    e.mem.set_u32(at, n + 1);
    for (i, w) in words.iter().enumerate() {
        e.mem
            .set_u32(at + 4 + 4 * (n * words.len() as u32 + i as u32), *w);
    }
}

fn recorded(e: &Engine, at: u32, width: u32) -> Vec<Vec<u32>> {
    (0..e.mem.u32(at))
        .map(|n| {
            (0..width)
                .map(|i| e.mem.u32(at + 4 + 4 * (n * width + i)))
                .collect()
        })
        .collect()
}

/// `GetRandomInt(GAME_RNG, bound)`'s stand-in: the next of [`DRAWS`], the bound recorded at
/// `REC + 0x400`.
fn draws(e: &mut Engine, values: &[u32]) {
    e.mem.set_u32(DRAWS, 0);
    for (i, v) in values.iter().enumerate() {
        e.mem.set_u32(DRAWS + 4 + 4 * i as u32, *v);
    }
    e.mem.set_u32(REC + 0x400, 0);
    e.register(GET_RANDOM_INT, |e, a| {
        assert_eq!(a[0], GAME_RNG);
        record(e, REC + 0x400, &[a[1]]);
        let n = e.mem.u32(DRAWS);
        e.mem.set_u32(DRAWS, n + 1);
        e.mem.u32(DRAWS + 4 + 4 * n).into_ret()
    });
}

fn bounds(e: &Engine) -> Vec<u32> {
    recorded(e, REC + 0x400, 1).concat()
}

/// A list of `{data, next}` nodes at `at` (0x10 apart) holding `items`; its first node.
fn nodes(e: &mut Engine, at: u32, items: &[u32]) -> u32 {
    for (i, item) in items.iter().enumerate() {
        let node = at + 0x10 * i as u32;
        e.mem.set_u32(node, *item);
        let next = if i + 1 < items.len() { node + 0x10 } else { 0 };
        e.mem.set_u32(node + 4, next);
    }
    at
}

/// G113's two seams: the id in the slot resolved through the masters (`AddCompileIndex(slot,
/// file)`, here making the mod index 3); the answer 0.
#[test]
fn g113_media_set_ids_are_resolved_through_the_masters() {
    let mut e = world(true, true);
    e.mem.set_u32(REC, 0);
    e.register(ADD_COMPILE_INDEX, |e, a| {
        record(e, REC, &[a[0], a[1]]);
        let id = e.mem.u32(a[0]);
        e.mem.set_u32(a[0], (id & 0x00ff_ffff) | 0x0300_0000);
        Ret::default()
    });
    let slot = B + 0x5000;
    for site in [MEDIA_SET_ID, BATTLE_MEDIA_SET_ID] {
        e.mem.set_u32(slot, 0x0100_0042);
        assert_eq!(e.site(site.addr(), &[0x8000, slot]), Some(0));
        assert_eq!(e.mem.u32(slot), 0x0300_0042);
    }
    assert_eq!(recorded(&e, REC, 2), vec![vec![slot, 0x8000]; 2]);
}

fn pick(e: &mut Engine, head: u32) -> u32 {
    e.call(PICK_MEDIA_SET.addr(), &[0x1234, head]).eax
}

/// G113's `00595560`: one node answers its data (no draw). Of more, a bag of the non-null sets in
/// order, shuffled (bounds 3 then 2: a draw of 2 swaps the first and the third, 0 leaves), is
/// emptied from its end; an empty bag is filled and shuffled again; each list has its own bag.
/// Without the row: the exe's function (the stand-in's marker).
#[test]
fn g113_media_sets_come_from_a_shuffled_bag() {
    let mut e = world(true, true);
    let one = nodes(&mut e, B + 0x5100, &[0x77]);
    let list = nodes(&mut e, B + 0x5200, &[0x11, 0, 0x33, 0x44]);
    let other = nodes(&mut e, B + 0x5300, &[0x55, 0x66]);
    draws(&mut e, &[2, 0, 1, 1, 0]);
    assert_eq!(pick(&mut e, one), 0x77);
    assert_eq!(bounds(&e), Vec::<u32>::new(), "one node: no draw");
    // [0x11, 0x33, 0x44] -> [0x44, 0x33, 0x11].
    assert_eq!(pick(&mut e, list), 0x11);
    assert_eq!(pick(&mut e, list), 0x33);
    assert_eq!(pick(&mut e, list), 0x44);
    assert_eq!(bounds(&e), vec![3, 2], "one shuffle for the three");
    // Refilled: draws 1, 1 make [0x33, 0x44, 0x11].
    assert_eq!(pick(&mut e, list), 0x11);
    assert_eq!(bounds(&e), vec![3, 2, 3, 2]);
    // Its own bag: [0x55, 0x66] (draw 0) answers 0x66; the first list's goes on.
    assert_eq!(pick(&mut e, other), 0x66);
    assert_eq!(pick(&mut e, list), 0x44);
    assert_eq!(bounds(&e), vec![3, 2, 3, 2, 2]);
    // All nulls: an empty bag answers 0; no list answers 0.
    let nulls = nodes(&mut e, B + 0x5400, &[0, 0]);
    assert_eq!(pick(&mut e, nulls), 0);
    assert_eq!(pick(&mut e, 0), 0);
    let mut off = world(true, false);
    assert_eq!(pick(&mut off, list), 0x5a5b);
    let word = e.jip_patches();
    e.set_jip_patches(word & !jip_patches::FIXES);
    assert_eq!(pick(&mut e, list), 0x5a5b);
}

/// The game's form map (`0x11C54C0`, 16 buckets) with `forms` (id, form).
fn form_map(e: &mut Engine, forms: &[(u32, u32)]) {
    let map = B + 0x6000;
    let buckets = map + 0x20;
    let items = map + 0x100;
    if !e.mem.is_mapped(0x011c_54c0) {
        e.map(0x011c_5000, 0x1000);
    }
    e.mem.set_u32(0x011c_54c0, map);
    e.mem.set_u32(map + 4, 16);
    e.mem.set_u32(map + 8, buckets);
    for (i, &(id, form)) in forms.iter().enumerate() {
        let item = items + 12 * i as u32;
        e.mem.set_u32(item + 4, id);
        e.mem.set_u32(item + 8, form);
        let bucket = buckets + 4 * (id % 16);
        let first = e.mem.u32(bucket);
        e.mem.set_u32(item, first);
        e.mem.set_u32(bucket, item);
    }
}

fn battle(e: &mut Engine, data: u32) -> u32 {
    e.call(PICK_BATTLE_MEDIA_SET.addr(), &[data]).eax
}

/// G113's `GetMediaSetBattle`: no first id answers 0; one id answers its media set; with two, the
/// second swaps with the first each time (no draw); with four, a draw of 2 (bound 3) swaps the
/// fourth with the first. An id whose form is not a media set, or that has none, answers 0.
/// Without the row: the exe's function.
#[test]
fn g113_battle_music_is_never_the_last_one_again() {
    let mut e = world(true, true);
    let (a, b, c, d) = (B + 0x6400, B + 0x6440, B + 0x6480, B + 0x64c0);
    for form in [a, b, c, d] {
        e.mem.set_u8(form + 4, MEDIA_SET_TYPE);
    }
    e.mem.set_u8(B + 0x6500 + 4, 0x6e);
    form_map(
        &mut e,
        &[
            (0xa1, a),
            (0xb2, b),
            (0xc3, c),
            (0xd4, d),
            (0xe5, B + 0x6500),
        ],
    );
    draws(&mut e, &[2]);
    let data = B + 0x6600;
    let head = data + 0x18;
    e.mem.set_u32(head, 0);
    assert_eq!(battle(&mut e, data), 0);
    nodes(&mut e, head, &[0xa1]);
    assert_eq!(battle(&mut e, data), a);
    nodes(&mut e, head, &[0xa1, 0xb2]);
    assert_eq!(battle(&mut e, data), b);
    assert_eq!(battle(&mut e, data), a);
    assert_eq!(bounds(&e), Vec::<u32>::new());
    nodes(&mut e, head, &[0xa1, 0xb2, 0xc3, 0xd4]);
    assert_eq!(battle(&mut e, data), d);
    assert_eq!(bounds(&e), vec![3]);
    let ids: Vec<u32> = (0..4).map(|i| e.mem.u32(head + 0x10 * i)).collect();
    assert_eq!(ids, vec![0xd4, 0xb2, 0xc3, 0xa1]);
    nodes(&mut e, head, &[0xe5]);
    assert_eq!(battle(&mut e, data), 0, "not a media set");
    nodes(&mut e, head, &[0xf6]);
    assert_eq!(battle(&mut e, data), 0, "no form");
    let word = e.jip_patches();
    e.set_jip_patches(word & !jip_patches::FIXES);
    assert_eq!(battle(&mut e, data), 0x5a5c);
}

const PATH: u32 = B + 0x7000;
const W1: u32 = B + 0x7800;
const W2: u32 = B + 0x7820;
const O1: u32 = B + 0x7840;
const O2: u32 = B + 0x7860;

/// D05's world: the flag's page, the folder path, and stand-ins for the archive test (the pattern
/// recorded at `REC + 0x800`; "found.wav" into the out path, answering 1), the list builder (the
/// list for the pattern's end: `.wav` at `REC + 0x900`, `.ogg` at `REC + 0x904`) and the heap's
/// free (recorded at `REC + 0xa00`).
fn sound_world(lists: bool, wavs: &[u32], oggs: &[u32]) -> Engine {
    let mut e = world(true, true);
    if !e.mem.is_mapped(USE_FILE_LISTS) {
        e.map(USE_FILE_LISTS & !0xfff, 0x1000);
    }
    e.mem.set_u8(USE_FILE_LISTS, u8::from(lists));
    e.mem.set_cstr(PATH, b"sound\\fx\\x\\");
    e.register(ARCHIVE_FIND, |e, a| {
        let pattern = e.mem.cstr(a[0]);
        e.mem.set_cstr(REC + 0x800, &pattern);
        assert_eq!((a[1], a[2]), (PATH, 8));
        e.mem.set_cstr(a[1], b"found.wav");
        1u32.into_ret()
    });
    let wav = if wavs.is_empty() {
        0
    } else {
        nodes(&mut e, B + 0x7200, wavs)
    };
    let ogg = if oggs.is_empty() {
        0
    } else {
        nodes(&mut e, B + 0x7400, oggs)
    };
    e.mem.set_u32(REC + 0x900, wav);
    e.mem.set_u32(REC + 0x904, ogg);
    e.register(BUILD_FILE_LIST, |e, a| {
        assert_eq!((a[1], a[2], a[3]), (PATH, 8, 0));
        let pattern = e.mem.cstr(a[0]);
        let at = match &pattern[..] {
            b"sound\\fx\\x\\*.wav" => REC + 0x900,
            b"sound\\fx\\x\\*.ogg" => REC + 0x904,
            other => panic!("{other:?}"),
        };
        e.mem.u32(at).into_ret()
    });
    e.mem.set_u32(REC + 0xa00, 0);
    e.register(0x00aa_4060, |e, a| {
        record(e, REC + 0xa00, &[a[1]]);
        Ret::default()
    });
    for (at, name) in [
        (W1, &b"w1.wav"[..]),
        (W2, b"w2.wav"),
        (O1, b"o1.ogg"),
        (O2, b"o2.ogg"),
    ] {
        e.mem.set_cstr(at, name);
    }
    e
}

fn pick_file(e: &mut Engine) -> u32 {
    e.call(PICK_SOUND_FILE.addr(), &[PATH]).eax
}

/// D05 without the game's flag: the archive's test of `path*.wav` answers, its name in `path`.
#[test]
fn d05_without_the_flag_asks_the_archive_for_wav() {
    let mut e = sound_world(false, &[], &[]);
    assert_eq!(pick_file(&mut e), 1);
    assert_eq!(e.mem.cstr(REC + 0x800), b"sound\\fx\\x\\*.wav");
    assert_eq!(e.mem.cstr(PATH), b"found.wav");
}

/// D05 with the flag: the `.wav` and `.ogg` lists joined (the `.ogg` after the `.wav`), a draw
/// of the count (3) picks the third, the `.ogg`; every name and node freed in order; true.
#[test]
fn d05_picks_among_wav_and_ogg_files() {
    let mut e = sound_world(true, &[W1, W2], &[O1]);
    draws(&mut e, &[2]);
    assert_eq!(pick_file(&mut e), 1);
    assert_eq!(e.mem.cstr(PATH), b"o1.ogg");
    assert_eq!(bounds(&e), vec![3]);
    assert_eq!(e.mem.u32(B + 0x7210 + 4), B + 0x7400, "joined");
    assert_eq!(
        recorded(&e, REC + 0xa00, 1).concat(),
        vec![W1, B + 0x7200, W2, B + 0x7210, O1, B + 0x7400]
    );
}

/// D05 with the flag: one file is taken without a draw (here one `.ogg` file); of two `.wav`
/// files a draw of 0 (bound 2) takes the first; no file answers false and leaves the path;
/// without the row, the exe's function.
#[test]
fn d05_one_file_or_none() {
    let mut e = sound_world(true, &[], &[O2]);
    draws(&mut e, &[]);
    assert_eq!(pick_file(&mut e), 1);
    assert_eq!(e.mem.cstr(PATH), b"o2.ogg");
    assert_eq!(bounds(&e), Vec::<u32>::new());
    let mut e = sound_world(true, &[W1, W2], &[]);
    draws(&mut e, &[0]);
    assert_eq!(pick_file(&mut e), 1);
    assert_eq!(e.mem.cstr(PATH), b"w1.wav");
    assert_eq!(bounds(&e), vec![2]);
    let mut e = sound_world(true, &[], &[]);
    assert_eq!(pick_file(&mut e), 0);
    assert_eq!(e.mem.cstr(PATH), b"sound\\fx\\x\\");
    assert_eq!(recorded(&e, REC + 0xa00, 1), Vec::<Vec<u32>>::new());
    let word = e.jip_patches();
    e.set_jip_patches(word & !jip_patches::FIXES);
    assert_eq!(pick_file(&mut e), 0x5a5d);
}

/// The INI settings for G115 at `B + 0x8000` on (0x10 apart, names at `B + 0x8800`), in the
/// first INI collection's list (the other collections and the game settings empty).
fn ini_settings(e: &mut Engine, settings: &[(&[u8], u32)]) -> Vec<u32> {
    let games = B + 0x9000;
    if !e.mem.is_mapped(0x011c_8048) {
        e.map(0x011c_8000, 0x1000);
    }
    e.mem.set_u32(0x011c_8048, games);
    e.mem.set_u32(games + 0x110, 0);
    let mut at = Vec::new();
    for (i, (name, bits)) in settings.iter().enumerate() {
        let s = B + 0x8000 + 0x10 * i as u32;
        let text = B + 0x8800 + 0x40 * i as u32;
        e.mem.set_cstr(text, name);
        e.mem.set_u32(s + 4, *bits);
        e.mem.set_u32(s + 8, text);
        at.push(s);
    }
    // A collection's list: data `+0x10C`, next `+0x110`, then nodes.
    let first = B + 0x9200;
    for (i, global) in [0x011f_96a0u32, 0x011f_35a0, 0x011c_c694, 0x011f_35a4]
        .iter()
        .enumerate()
    {
        let coll = first + 0x200 * i as u32;
        if !e.mem.is_mapped(*global) {
            e.map(*global & !0xfff, 0x1000);
        }
        e.mem.set_u32(*global, coll);
        e.mem.set_u32(coll + 0x10c, 0);
        e.mem.set_u32(coll + 0x110, 0);
    }
    let mut node = first + 0x10c;
    for (i, s) in at.iter().enumerate() {
        e.mem.set_u32(node, *s);
        let next = if i + 1 < at.len() {
            B + 0x9a00 + 8 * i as u32
        } else {
            0
        };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
    at
}

/// G115: with `FalloutCustom.ini` (the port's mod files), each `key=value` sets `key:section` by
/// the key's letter: `f` the `float`, `b` the word 1 for a value starting `1` else 0, `i` and `u`
/// JIP's integer readers (`-1` as an unsigned word is 0xFFFFFFFF), `s` a copy on the game's heap;
/// a key with no setting, or a setting of another section, is left; the answer 0. Without the
/// file nothing changes.
#[test]
fn g115_falloutcustom_ini_sets_the_ini_settings() {
    use crate::commands::test_support::files;
    files::fresh();
    let mut e = world(true, true);
    e.register(0x00aa_3e40, |e, a| e.mem.alloc(a[1]).into_ret());
    let s = ini_settings(
        &mut e,
        &[
            (b"fFoo:General", 0),
            (b"bBar:General", 0xffff_ff00),
            (b"iBaz:General", 0),
            (b"uQux:General", 0),
            (b"sName:General", 0x1234),
            (b"bBar:Display", 1),
            (b"fKeep:Display", 7),
        ],
    );
    assert_eq!(e.site(CUSTOM_INI.addr(), &[]), Some(0));
    assert_eq!(e.mem.u32(s[1] + 4), 0xffff_ff00, "no file: nothing changes");
    files::put(
        "FalloutCustom.ini",
        b"[General]\r\nfFoo=2.5\r\nbBar=1\r\niBaz=-7\r\nuQux=-1\r\nsName=hello there\r\nfMissing=3\r\nfKeep=9\r\n[Display]\r\nBBAR = 0x\r\n",
    );
    assert_eq!(e.site(CUSTOM_INI.addr(), &[]), Some(0));
    assert_eq!(e.mem.f32(s[0] + 4), 2.5);
    assert_eq!(e.mem.u32(s[1] + 4), 1);
    assert_eq!(e.mem.i32(s[2] + 4), -7);
    assert_eq!(e.mem.u32(s[3] + 4), 0xffff_ffff);
    let text = e.mem.u32(s[4] + 4);
    assert_ne!(text, 0x1234);
    assert_eq!(e.mem.cstr(text), b"hello there");
    assert_eq!(e.mem.u32(s[5] + 4), 0, "BBAR = 0x: the word 0");
    assert_eq!(e.mem.u32(s[6] + 4), 7, "fKeep is Display's");
}

// ---- G108 ----

/// A game sound at `B + 0xa000` (base rate 22050, factor 1.2), its device, and its buffer at
/// `B + 0xa400` whose `GetFrequency` (slot `0x20`) writes the word at `REC + 0xc00` and answers the
/// one at `REC + 0xc04`, and whose `SetFrequency` (slot `0x44`) is recorded at `REC + 0xc10`.
const SOUND: u32 = B + 0xa000;
const BUFFER_OBJECT: u32 = B + 0xa400;

fn sound_engine() -> Engine {
    let mut e = world(true, true);
    let vtable = B + 0xa500;
    e.mem.set_u32(BUFFER_OBJECT, vtable);
    e.mem.set_u32(vtable + 0x20, B + 0xa800);
    e.mem.set_u32(vtable + 0x44, B + 0xa810);
    e.register(B + 0xa800, |e, a| {
        e.mem.set_u32(a[1], e.mem.u32(REC + 0xc00));
        e.mem.u32(REC + 0xc04).into_ret()
    });
    e.mem.set_u32(REC + 0xc10, 0);
    e.register(B + 0xa810, |e, a| {
        assert_eq!(a[0], BUFFER_OBJECT);
        record(e, REC + 0xc10, &[a[1]]);
        0u32.into_ret()
    });
    e.mem.set_u16(SOUND + SAMPLES_PER_SECOND, 22050);
    e.mem.set_f32(SOUND + FREQUENCY_FACTOR, 1.2);
    e.mem.set_u32(SOUND + DEVICE, 0x4444);
    e.mem.set_u32(SOUND + BUFFER, BUFFER_OBJECT);
    e
}

/// G108 at the constructor: JIP's form field cleared, its factor 1.0.
#[test]
fn g108_a_new_sound_has_the_factor_one() {
    let mut e = world(true, true);
    e.mem.set_u32(SOUND + 0x134, 0x7777);
    e.mem.set_u32(SOUND + 0x138, 0x7777);
    assert_eq!(e.site(FREQUENCY_FIELDS.addr(), &[SOUND]), Some(0));
    assert_eq!(e.mem.u32(SOUND + 0x134), 0);
    assert_eq!(e.mem.f32(SOUND + 0x138), 1.0);
}

/// G108 in `ApplySettingsFromMasterFile`: a handle with a sound id posts the audio manager
/// (`0x11F6EF0`) a message of kind 54 with the id and the form; a negative id posts nothing.
#[test]
fn g108_a_sound_request_is_posted() {
    let mut e = world(true, true);
    e.mem.set_u32(REC, 0);
    e.register(POST_MESSAGE, |e, a| {
        record(e, REC, &a[..9]);
        Ret::default()
    });
    let handle = B + 0xa900;
    e.mem.set_u32(handle, 5);
    assert_eq!(e.site(SOUND_REQUEST.addr(), &[handle, 0x6600]), Some(0));
    e.mem.set_u32(handle, 0xffff_ffff);
    assert_eq!(e.site(SOUND_REQUEST.addr(), &[handle, 0x6600]), Some(0));
    assert_eq!(
        recorded(&e, REC, 9),
        vec![vec![0x011f_6ef0, 0x36, 5, 0x6600, 0, 0, 0, 0, 0]]
    );
}

/// G108's message 54: the sound with the message's id (the second in its bucket of three) gets
/// the form and its factor: +20 % is 1.2, -20 % is 0.9 (half as much), the flag bit 0 at `+0x48`
/// or 0 % is 1.0; another id, or a map of no buckets, changes nothing.
#[test]
fn g108_the_audio_thread_gives_the_sound_its_factor() {
    let mut e = world(true, true);
    let manager = B + 0xab00;
    let buckets = manager + 0x100;
    e.mem.set_u32(manager + 0x58, 3);
    e.mem.set_u32(manager + 0x5c, buckets);
    // Bucket 1 (7 % 3): the entry of id 4, then the one of id 7.
    let (first, second) = (manager + 0x200, manager + 0x210);
    e.mem.set_u32(buckets + 4, first);
    e.mem.set_u32(first, second);
    e.mem.set_u32(first + 4, 4);
    e.mem.set_u32(first + 8, SOUND + 0x400);
    e.mem.set_u32(second, 0);
    e.mem.set_u32(second + 4, 7);
    e.mem.set_u32(second + 8, SOUND);
    let message = manager + 0x300;
    let form = manager + 0x400;
    e.mem.set_u32(message + 4, 7);
    e.mem.set_u32(message + 8, form);
    let factor = |e: &mut Engine, percent: i8, flags: u8| {
        e.mem.set_u8(form + 0x46, percent as u8);
        e.mem.set_u8(form + 0x48, flags);
        e.mem.set_f32(SOUND + 0x138, -1.0);
        assert_eq!(e.site(SOURCE_SOUND.addr(), &[manager, message]), Some(0));
        assert_eq!(e.mem.u32(SOUND + 0x134), form);
        e.mem.f32(SOUND + 0x138)
    };
    assert_eq!(
        factor(&mut e, 20, 0),
        1.0 + 20.0 * f32::from_bits(0x3c23_d70a)
    );
    assert_eq!(
        factor(&mut e, -20, 0),
        1.0 - 20.0 * f32::from_bits(0x3ba3_d70a)
    );
    assert_eq!(factor(&mut e, 20, 1), 1.0);
    assert_eq!(factor(&mut e, 0, 0), 1.0);
    assert_eq!(e.mem.u32(SOUND + 0x400 + 0x134), 0, "id 4 is left");
    e.mem.set_u32(message + 4, 10);
    e.mem.set_u32(SOUND + 0x134, 0);
    assert_eq!(e.site(SOURCE_SOUND.addr(), &[manager, message]), Some(0));
    assert_eq!(e.mem.u32(SOUND + 0x134), 0, "no sound of id 10");
    e.mem.set_u32(message + 4, 7);
    e.mem.set_u32(manager + 0x58, 0);
    assert_eq!(e.site(SOURCE_SOUND.addr(), &[manager, message]), Some(0));
    assert_eq!(e.mem.u32(SOUND + 0x134), 0, "no buckets");
}

fn set_frequency_to(e: &mut Engine, ratio: f32) -> Vec<u32> {
    e.mem.set_u32(REC + 0xc10, 0);
    assert_eq!(
        e.call(SET_FREQUENCY.addr(), &[SOUND, ratio.to_bits()]).eax,
        0
    );
    recorded(e, REC + 0xc10, 1).concat()
}

/// G108's `SetFrequency`: the base rate times the factor times the ratio, in `float`, rounded to
/// nearest (ties to even), kept within 100 to 200000 (a NaN is 100); answers 0. No buffer or no
/// device: no call. Without the row: the exe's function.
#[test]
fn g108_set_frequency_takes_the_factor() {
    let mut e = sound_engine();
    assert_eq!(
        set_frequency_to(&mut e, 1.0),
        vec![(22050.0 * 1.2f32).round_ties_even() as u32]
    );
    assert_eq!(set_frequency_to(&mut e, 0.0), vec![100]);
    assert_eq!(set_frequency_to(&mut e, 1000.0), vec![200_000]);
    assert_eq!(set_frequency_to(&mut e, f32::NAN), vec![100]);
    e.mem.set_u16(SOUND + SAMPLES_PER_SECOND, 1);
    e.mem.set_f32(SOUND + FREQUENCY_FACTOR, 1.0);
    assert_eq!(set_frequency_to(&mut e, 100.5), vec![100]);
    assert_eq!(set_frequency_to(&mut e, 101.5), vec![102]);
    assert_eq!(set_frequency_to(&mut e, 150.7), vec![151]);
    e.mem.set_u32(SOUND + DEVICE, 0);
    assert_eq!(set_frequency_to(&mut e, 150.0), Vec::<u32>::new());
    e.mem.set_u32(SOUND + DEVICE, 0x4444);
    e.mem.set_u32(SOUND + BUFFER, 0);
    assert_eq!(set_frequency_to(&mut e, 150.0), Vec::<u32>::new());
    let word = e.jip_patches();
    e.set_jip_patches(word & !jip_patches::FIXES);
    assert_eq!(
        e.call(SET_FREQUENCY.addr(), &[SOUND, 1.0f32.to_bits()]).eax,
        0x5a5e
    );
}

/// G108's `GetFrequency`: the buffer's frequency over the base rate times the factor (26460 over
/// 22050 x 1.2 is 1.0); a failed `GetFrequency`, no buffer or no device: 1.0. Without the row: the
/// exe's function.
#[test]
fn g108_get_frequency_reads_over_the_factor() {
    let mut e = sound_engine();
    let read = |e: &mut Engine| e.call(GET_FREQUENCY.addr(), &[SOUND]).st0;
    e.mem.set_u32(REC + 0xc00, 26460);
    e.mem.set_u32(REC + 0xc04, 0);
    assert_eq!(read(&mut e), f64::from(26460.0f32 / (22050.0f32 * 1.2)));
    e.mem.set_u32(REC + 0xc00, 44100);
    assert_eq!(read(&mut e), f64::from(44100.0f32 / (22050.0f32 * 1.2)));
    e.mem.set_u32(REC + 0xc04, 0x8000_4005);
    assert_eq!(read(&mut e), 1.0, "failed");
    e.mem.set_u32(REC + 0xc04, 0);
    e.mem.set_u32(SOUND + DEVICE, 0);
    assert_eq!(read(&mut e), 1.0, "no device");
    e.mem.set_u32(SOUND + DEVICE, 0x4444);
    e.mem.set_u32(SOUND + BUFFER, 0);
    assert_eq!(read(&mut e), 1.0, "no buffer");
    let word = e.jip_patches();
    e.set_jip_patches(word & !jip_patches::FIXES);
    assert_eq!(read(&mut e), 7.0);
}

/// JIP's `StrToUInt`: digits to the first other byte, a minus negating; a fall (unsigned
/// overflow) answers 0xFFFFFFFF, or 0 with the minus.
#[test]
fn g115_str_to_uint_reads_as_jip() {
    assert_eq!(str_to_uint(b"42x"), 42);
    assert_eq!(str_to_uint(b"-1"), 0xffff_ffff);
    assert_eq!(str_to_uint(b"4294967295"), 0xffff_ffff);
    assert_eq!(str_to_uint(b"4294967296"), 0xffff_ffff);
    assert_eq!(str_to_uint(b"-4294967296"), 0);
    assert_eq!(str_to_uint(b"x"), 0);
    assert_eq!(str_to_uint(b""), 0);
}
