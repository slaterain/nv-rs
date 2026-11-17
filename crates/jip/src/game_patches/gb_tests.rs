//! Group GB's game patches: D04, the pad dead zones.

use super::*;
use crate::game_patches::set_row;
use crate::test_support::engine;

const LEFT: [u32; 9] = [
    0x0074_16d6,
    0x0074_1705,
    0x0079_0966,
    0x0079_9861,
    0x007b_d47d,
    0x007b_d4a0,
    0x007b_d4dd,
    0x0094_ab59,
    0x0094_ab88,
];
const RIGHT: [u32; 2] = [0x0094_a93c, 0x0094_a95f];

/// The double a seam's answer points at, or `None` when the seam answers nothing (the exe's
/// `0x010718F0` then).
fn compared(e: &mut Engine, site: u32) -> Option<f64> {
    e.site(site, &[]).map(|at| e.mem.f64(at))
}

/// JIP's addresses (from `DeferredInit`, the operand bytes `f0180701` read at each in Ghidra) and
/// the exe's double.
#[test]
fn the_seams_are_jips_operands() {
    assert_eq!(LEFT_ADDRS, LEFT);
    assert_eq!(RIGHT_ADDRS, RIGHT);
    assert_eq!(ALL_ADDRS[..9], LEFT);
    assert_eq!(ALL_ADDRS[9..], RIGHT);
    assert!(
        AREA.fix_sites.iter().all(|a| !JIP_SITES.contains(a)),
        "D04 all JIP's own (INSTALLED)"
    );
    assert_eq!(EXE_DEAD_ZONE, 0x0107_18f0);
}

/// Not installed: nothing registered, every seam answers nothing (the exe's double), with the row
/// on or off.
#[test]
fn not_installed_every_seam_answers_nothing() {
    let mut e = engine();
    set_row(&mut e, true);
    crate::install_as(&mut e, false);
    set_row(&mut e, true);
    for site in LEFT.into_iter().chain(RIGHT) {
        assert_eq!(e.site(site, &[]), None, "{site:08x}");
    }
}

/// Installed with the row off (vanilla), as JIP whatever the row says: the nine left-stick seams
/// answer JIP's `s_deadZoneLSg`, 7849.0 (the exe's value), the two right-stick seams JIP's
/// `s_deadZoneRSg`, 8689.0; the doubles are made once, at JIP's install.
#[test]
fn installed_every_seam_answers_jips_double() {
    let mut e = engine();
    crate::install_as(&mut e, true);
    set_row(&mut e, false);
    let (left, right) = doubles(&mut e);
    assert_eq!(right, left + 8);
    for site in LEFT {
        assert_eq!(e.site(site, &[]), Some(left), "{site:08x}");
        assert_eq!(compared(&mut e, site), Some(7849.0));
    }
    for site in RIGHT {
        assert_eq!(e.site(site, &[]), Some(right), "{site:08x}");
        assert_eq!(compared(&mut e, site), Some(8689.0));
    }
    assert_eq!(doubles(&mut e), (left, right), "made once");
}

/// The doubles come from the input state's dead zones (JIP's `SetDeadZoneLS/RS` would change
/// both), made at the first ask.
#[test]
fn the_doubles_take_the_input_state() {
    let mut e = engine();
    crate::input::state(&mut e).dead_zone_ls = 4000;
    crate::input::state(&mut e).dead_zone_rs = -1;
    let (left, right) = doubles(&mut e);
    assert_eq!(e.mem.f64(left), 4000.0);
    assert_eq!(e.mem.f64(right), -1.0);
    crate::input::state(&mut e).dead_zone_ls = 1;
    assert_eq!(doubles(&mut e), (left, right), "made once");
    assert_eq!(e.mem.f64(left), 4000.0);
}

// ---- G87 and G88 ----

const MAP: u32 = 0x0079_df73;
const FADE_5: u32 = 0x00a0_8718;
const FADE_6: u32 = 0x00a0_871c;
/// `Tile::Value::Refresh` (`0xA09410`), which JIP's `Tile::SetFloat` calls after a change.
const VALUE_REFRESH: u32 = 0x00a0_9410;

/// JIP's addresses: `WriteRelJump(0x79DF73, ..)`, `SafeWrite32(0xA08718/0xA0871C, ..)`; all JIP's
/// own (registered with JIP, not with the fix row).
#[test]
fn g87_g88_seams_are_jips_addresses() {
    assert_eq!(MAP_SKIP_CENTRING.addr(), MAP);
    assert_eq!(FADE_TYPES.map(|s| s.addr()), [FADE_5, FADE_6]);
    assert_eq!(JIP_SITES[..11], ALL_ADDRS);
    assert_eq!(JIP_SITES[11..], [MAP, FADE_5, FADE_6]);
    assert_eq!(AREA.jip_sites, &JIP_SITES[..]);
    assert_eq!(TICK_COUNT_IMPORT, 0x00fd_f060);
}

/// Not installed: G87's and G88's seams answer nothing (the exe's paths), with the row on.
#[test]
fn not_installed_g87_g88_answer_nothing() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    set_row(&mut e, true);
    skip_map_centring_once(&mut e);
    let control = e.mem.alloc(0x1c);
    for site in [MAP, FADE_5, FADE_6] {
        assert_eq!(e.site(site, &[control, 6]), None, "{site:08x}");
    }
}

/// G87: the flag is clear at install (0: centre as the exe); once set it answers 1 a single time
/// and clears itself, as `RefreshMapMenuHook` does; with the row off too (JIP's own).
#[test]
fn g87_skips_the_centring_once() {
    let mut e = engine();
    crate::install_as(&mut e, true);
    set_row(&mut e, false);
    assert_eq!(e.site(MAP, &[]), Some(0));
    skip_map_centring_once(&mut e);
    assert_eq!(e.site(MAP, &[]), Some(1));
    assert_eq!(e.site(MAP, &[]), Some(0), "cleared by the skip");
    skip_map_centring_once(&mut e);
    skip_map_centring_once(&mut e);
    assert_eq!(e.site(MAP, &[]), Some(1));
    assert_eq!(e.site(MAP, &[]), Some(0), "a flag, not a count");
}

/// A fade fixture: a tile whose values (sorted by id, 0x10 bytes each, the number at +8) are
/// ids 0xFA0, 0xFA9, 0xFB5, each with non-zero neighbouring words; a control on trait 0xFA9; the
/// clock at `now`; refreshes recorded.
struct Fade {
    control: u32,
    values: [u32; 3],
    refreshed: std::rc::Rc<std::cell::RefCell<Vec<Vec<u32>>>>,
}

fn fade(e: &mut Engine, now: u32, (start, end): (f32, f32), started: u32, duration: f32) -> Fade {
    let tile = e.mem.alloc(0x40);
    let array = e.mem.alloc(0x10);
    let mut values = [0; 3];
    for (i, id) in [0xfa0u32, 0xfa9, 0xfb5].into_iter().enumerate() {
        let value = e.mem.alloc(0x10);
        e.mem.set_u32(value, id);
        e.mem.set_u32(value + 4, 0xdead_0000 | id);
        e.mem.set_f32(value + 8, 50.0 + i as f32);
        e.mem.set_u32(value + 0xc, 0xbeef_0000 | id);
        e.mem.set_u32(array + 4 * i as u32, value);
        values[i] = value;
    }
    e.mem.set_u32(array + 0xc, 0x0bad_f00d);
    e.mem.set_u32(tile + 0x14, array);
    e.mem.set_u32(tile + 0x18, 3);
    let control = e.mem.alloc(0x1c);
    e.mem.set_f32(control, start);
    e.mem.set_f32(control + 4, end);
    e.mem.set_u32(control + 8, started);
    e.mem.set_f32(control + 0xc, duration);
    e.mem.set_u32(control + 0x10, 0xfa9);
    e.mem.set_u32(control + 0x14, tile);
    e.mem.set_u32(control + 0x18, 0x5a5a);
    e.register_double(TICK_COUNT_IMPORT, move |_, a| {
        assert!(a.is_empty(), "GetTickCount takes no argument");
        now.into_ret()
    });
    let refreshed = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
    let seen = refreshed.clone();
    e.register_double(VALUE_REFRESH, move |_, a| {
        seen.borrow_mut().push(a[..2].to_vec());
        Ret::default()
    });
    Fade {
        control,
        values,
        refreshed,
    }
}

fn installed() -> Engine {
    let mut e = engine();
    crate::install_as(&mut e, true);
    set_row(&mut e, false);
    e
}

/// The numbers of the three values.
fn numbers(e: &Engine, f: &Fade) -> [f32; 3] {
    f.values.map(|v| e.mem.f32(v + 8))
}

/// G88 under way: the value is JIP's single-precision `progress * (end - start) + start`
/// (progress = ticks as a signed int over the duration), set on the control's trait only, and
/// refreshed; the start time is kept; both types answer 0. The elapsed ticks are chosen so that
/// computing in doubles would give another float.
#[test]
fn g88_under_way_sets_jips_float_value() {
    let (start, end, duration) = (1.3f32, 2.9f32, 700.0f32);
    let jip = |elapsed: u32| (elapsed as i32 as f32 / duration) * (end - start) + start;
    let doubles = |elapsed: u32| {
        ((elapsed as f64 / duration as f64) * (end as f64 - start as f64) + start as f64) as f32
    };
    let elapsed = (1..700)
        .find(|&t| jip(t) != doubles(t))
        .expect("a case where float order matters");
    for kind in [5u32, 6] {
        let mut e = installed();
        let f = fade(&mut e, 0x1_0000 + elapsed, (start, end), 0x1_0000, duration);
        let site = if kind == 5 { FADE_5 } else { FADE_6 };
        assert_eq!(e.site(site, &[f.control, kind]), Some(0), "type {kind}");
        assert_eq!(numbers(&e, &f), [50.0, jip(elapsed), 52.0]);
        assert_eq!(*f.refreshed.borrow(), vec![vec![f.values[1], 1]]);
        assert_eq!(e.mem.u32(f.control + 8), 0x1_0000, "start time kept");
        assert_eq!(e.mem.u32(f.values[1] + 4), 0xdead_0fa9);
        assert_eq!(e.mem.u32(f.values[1] + 0xc), 0xbeef_0fa9);
    }
}

/// The ticks are a signed difference (`cvtsi2ss`): a start time 100 ticks after now gives a
/// progress of -100/800, not a huge unsigned one (which would end the fade).
#[test]
fn g88_reads_the_ticks_as_signed() {
    let mut e = installed();
    let f = fade(&mut e, 5, (2.5, 6.5), 105, 800.0);
    assert_eq!(e.site(FADE_6, &[f.control, 6]), Some(0));
    let progress = -100.0f32 / 800.0;
    assert_eq!(numbers(&e, &f)[1], progress * 4.0 + 2.5);
    assert_eq!(e.mem.u32(f.control + 8), 105);
    // Across the counter's wrap: started at 0xFFFF_FFF6, now 0x5A: 100 ticks.
    let mut e = installed();
    let f = fade(&mut e, 0x5a, (2.5, 6.5), 0xffff_fff6, 800.0);
    assert_eq!(e.site(FADE_5, &[f.control, 5]), Some(0));
    assert_eq!(numbers(&e, &f)[1], 100.0f32 / 800.0 * 4.0 + 2.5);
}

/// G88 at the end (progress 1 or above, `comiss`/`jnb`): the start time becomes now, the value the
/// START value; type 5 answers 0 (starts over, stays listed), type 6 answers 1 (listed as
/// finished). Exactly 1 counts as the end.
#[test]
fn g88_at_the_end_sets_the_start_value() {
    for (kind, site, answer) in [(5u32, FADE_5, 0), (6, FADE_6, 1)] {
        for elapsed in [700u32, 701, 90_000] {
            let mut e = installed();
            let f = fade(&mut e, 0x2000 + elapsed, (1.3, 2.9), 0x2000, 700.0);
            assert_eq!(
                e.site(site, &[f.control, kind]),
                Some(answer),
                "{kind} {elapsed}"
            );
            assert_eq!(numbers(&e, &f), [50.0, 1.3, 52.0]);
            assert_eq!(e.mem.u32(f.control + 8), 0x2000 + elapsed, "restarted now");
            assert_eq!(f.refreshed.borrow().len(), 1);
        }
    }
    // 699 of 700: still under way.
    let mut e = installed();
    let f = fade(&mut e, 0x2000 + 699, (1.3, 2.9), 0x2000, 700.0);
    assert_eq!(e.site(FADE_6, &[f.control, 6]), Some(0));
    assert_eq!(e.mem.u32(f.control + 8), 0x2000);
}

/// JIP's `Tile::SetFloat`: nothing for a trait the tile does not have, for a value equal to the
/// one it holds, or for a NaN (no ticks over a zero duration: under way, `comiss` unordered); a
/// zero duration with ticks gone by is the end.
#[test]
fn g88_sets_as_jips_tile_set_float() {
    let mut e = installed();
    let f = fade(&mut e, 300, (1.3, 2.9), 0, 600.0);
    e.mem.set_u32(f.control + 0x10, 0xfa8);
    assert_eq!(e.site(FADE_6, &[f.control, 6]), Some(0));
    assert_eq!(numbers(&e, &f), [50.0, 51.0, 52.0]);
    assert!(f.refreshed.borrow().is_empty(), "no such trait");

    let mut e = installed();
    let f = fade(&mut e, 300, (51.0, 51.0), 0, 600.0);
    assert_eq!(e.site(FADE_6, &[f.control, 6]), Some(0));
    assert!(f.refreshed.borrow().is_empty(), "an equal value");

    let mut e = installed();
    let f = fade(&mut e, 40, (1.3, 2.9), 40, 0.0);
    assert_eq!(e.site(FADE_6, &[f.control, 6]), Some(0), "NaN: under way");
    assert_eq!(numbers(&e, &f), [50.0, 51.0, 52.0]);
    assert!(f.refreshed.borrow().is_empty(), "a NaN is not set");
    assert_eq!(e.mem.u32(f.control + 8), 40);

    let mut e = installed();
    let f = fade(&mut e, 41, (1.3, 2.9), 40, 0.0);
    assert_eq!(
        e.site(FADE_6, &[f.control, 6]),
        Some(1),
        "infinity: the end"
    );
    assert_eq!(numbers(&e, &f)[1], 1.3);

    // Each trait of the tile is found (the binary search): the first and the last.
    for (id, index) in [(0xfa0u32, 0usize), (0xfb5, 2)] {
        let mut e = installed();
        let f = fade(&mut e, 0x2000 + 700, (7.7, 9.1), 0x2000, 700.0);
        e.mem.set_u32(f.control + 0x10, id);
        assert_eq!(e.site(FADE_6, &[f.control, 6]), Some(1));
        let mut expected = [50.0, 51.0, 52.0];
        expected[index] = 7.7;
        assert_eq!(numbers(&e, &f), expected, "{id:x}");
    }

    // A control without a tile: nothing set (nv-rs's own).
    let mut e = installed();
    let f = fade(&mut e, 300, (1.3, 2.9), 0, 600.0);
    e.mem.set_u32(f.control + 0x14, 0);
    assert_eq!(e.site(FADE_5, &[f.control, 5]), Some(0));
    assert!(f.refreshed.borrow().is_empty());
}

/// Every GB handler at its own address after install, with words on which the others answer
/// differently: the dead-zone seams answer their doubles, G87 0 (no skip asked), G88 1 for an
/// ended type-6 control (and its value set), 0 for a type-5 one.
#[test]
fn every_gb_handler_sits_at_its_address() {
    let mut e = installed();
    let (left, right) = doubles(&mut e);
    for site in LEFT {
        assert_eq!(e.site(site, &[]), Some(left), "{site:08x}");
    }
    for site in RIGHT {
        assert_eq!(e.site(site, &[]), Some(right), "{site:08x}");
    }
    let f = fade(&mut e, 0x3000, (4.4, 8.8), 0x1000, 700.0);
    assert_eq!(e.site(MAP, &[f.control, 6]), Some(0));
    assert_eq!(numbers(&e, &f)[1], 51.0, "G87 sets nothing");
    assert_eq!(e.site(FADE_6, &[f.control, 6]), Some(1));
    assert_eq!(numbers(&e, &f)[1], 4.4);
    e.mem.set_u32(f.control + 8, 0x1000);
    assert_eq!(e.site(FADE_5, &[f.control, 5]), Some(0));
    assert_eq!(e.mem.u32(f.control + 8), 0x3000);
}

// ---- GB-3: G13, G68, G100, G56, G64, G69 (fixes of `InitGamePatches`) ----

/// Where the GB-3 tests' guest data lives.
const AREA_AT: u32 = 0x0640_0000;
/// The stand-ins' heap blocks.
const BLOCKS: u32 = 0x0650_0000;
/// The game's functions the GB-3 handlers call.
const GET_BY_TYPE: u32 = 0x0041_0220;
const ADD_EXTRA: u32 = 0x0040_ff60;
const HEAP_ALLOCATE: u32 = 0x00aa_3e40;

thread_local! {
    static GB3_CALLS: std::cell::RefCell<Vec<(u32, Vec<u32>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static GB3_NEXT: std::cell::Cell<u32> = const { std::cell::Cell::new(BLOCKS) };
}

fn gb3_log(addr: u32, a: &[u32]) {
    GB3_CALLS.with(|c| c.borrow_mut().push((addr, a.to_vec())));
}

fn gb3_calls() -> Vec<(u32, Vec<u32>)> {
    GB3_CALLS.with(|c| c.borrow_mut().drain(..).collect())
}

/// Installed with the row off, then the group's fix handlers put in (`install_fixes`, as the row
/// does), guest pages mapped, the call log cleared.
fn fixes_on() -> Engine {
    gb3_calls();
    GB3_NEXT.with(|n| n.set(BLOCKS));
    let mut e = engine();
    e.map(AREA_AT, 0x3_0000);
    e.map(BLOCKS, 0x1_0000);
    crate::install_as(&mut e, true);
    install_fixes(&mut e);
    e
}

/// JIP's `InitGamePatches` addresses (`WriteRelCall(0x72880D, ..)`, `SAFE_WRITE_BUF(0x728E10, ..)`,
/// `WriteRelJump(0x7294DC, ..)`, `SafeWrite32(0x729616, 0x1030018)`, `WriteRelCall(0x72D661, ..)`,
/// `WriteRelCall(0x73005C, ..)`, `WriteRelJump(0x7AC179, ..)`): fixes, in with the row.
#[test]
fn gb3_rows_are_jips_addresses() {
    assert_eq!(
        AREA.fix_sites[..7],
        [
            0x0072_880d,
            0x0072_8e10,
            0x0072_94dc,
            0x0072_9616,
            0x0072_d661,
            0x0073_005c,
            0x007a_c179
        ]
    );
    assert!(AREA.fix_wraps.is_empty() && AREA.jip_wraps.is_empty());
    assert_eq!(TWO_DECIMALS, 0x0103_0018);
}

/// Installed with the row off: no GB-3 seam answers (the exe's code); not installed with the row
/// on: none either.
#[test]
fn gb3_off_nothing_answers() {
    let mut e = installed();
    let menu = 0x0640_0000;
    for &addr in AREA.fix_sites {
        assert_eq!(e.site(addr, &[menu, menu]), None, "{addr:08x}");
    }
    let mut e = engine();
    crate::install_as(&mut e, false);
    set_row(&mut e, true);
    for &addr in AREA.fix_sites {
        assert_eq!(e.site(addr, &[menu, menu]), None, "{addr:08x}");
    }
}

/// G100 (`IsDisposableWeaponHook`: `cmp byte [ecx+0xF4], 9; setg al`): the type byte, signed,
/// above 9. The game's types 10 to 13 answer 1 (as the exe's `004c0bf0`: 3 to 13 and not 3 to 9),
/// 0 to 9 answer 0; above 13 only JIP's test speaks: 0x7F answers 1, 0x80 and above (negative) 0.
#[test]
fn g100_type_above_nine() {
    let mut e = fixes_on();
    let weapon = AREA_AT;
    for (kind, answer) in [
        (0u8, 0),
        (3, 0),
        (9, 0),
        (10, 1),
        (11, 1),
        (13, 1),
        (0x7f, 1),
        (0x80, 0),
        (0xff, 0),
    ] {
        e.mem.set_u8(weapon + 0xf4, kind);
        assert_eq!(
            e.site(DISPOSABLE_WEAPON.addr(), &[weapon]),
            Some(answer),
            "{kind}"
        );
    }
}

/// G13: the value's call is skipped (1), the weight's format is `"%.2f"`.
#[test]
fn g13_answers() {
    let mut e = fixes_on();
    assert_eq!(e.site(PLAIN_VALUE.addr(), &[]), Some(1));
    assert_eq!(e.site(WEIGHT_FORMAT.addr(), &[]), Some(0x0103_0018));
}

/// G68 (`cmp byte [ecx+4], kFormType_TESAmmo`): ammunition (0x29) answers 1, any other type
/// nothing (the exe's test).
#[test]
fn g68_ammo_only() {
    let mut e = fixes_on();
    let form = AREA_AT;
    for (kind, answer) in [(0x29u8, Some(1)), (0x28, None), (0x2f, None), (0, None)] {
        e.mem.set_u8(form + 4, kind);
        assert_eq!(e.site(AMMO_EFFECTS.addr(), &[form]), answer, "{kind:x}");
    }
}

/// G56 (`InitBarterMenuHook`): `SetFilterModeTitle` on the menu with its word at +0x48, once;
/// answers nothing (the exe's fill follows).
#[test]
fn g56_titles_the_tab_at_0x48() {
    let mut e = fixes_on();
    e.register(0x0073_01f0, |_, a| {
        gb3_log(0x0073_01f0, a);
        Ret::default()
    });
    let menu = AREA_AT;
    e.mem.set_u32(menu + 0x48, 0x0777_0048);
    e.mem.set_u32(menu + 0x34, 0x0777_0034);
    assert_eq!(e.site(BARTER_TAB_TITLE.addr(), &[menu]), None);
    assert_eq!(gb3_calls(), [(0x0073_01f0, vec![menu, 0x0777_0048])]);
}

/// G69 (`QttMenuEnableWheelHook`): the wheel word (+0x44 of `g_interfaceManager`), signed: above 0
/// clicks button 2, below 0 button 1 (the menu's slot +0xC with the id and 0); 0 clicks nothing.
/// Answers nothing.
#[test]
fn g69_wheel_clicks_plus_or_minus() {
    let mut e = fixes_on();
    if !e.mem.is_mapped(0x011d_8a80) {
        e.map(0x011d_8000, 0x1000);
    }
    let (manager, menu, vtable) = (AREA_AT, AREA_AT + 0x100, AREA_AT + 0x200);
    e.mem.set_u32(0x011d_8a80, manager);
    e.mem.set_u32(menu, vtable);
    e.mem.set_u32(vtable + 0xc, 0x0000_9c00);
    e.register(0x0000_9c00, |_, a| {
        gb3_log(0x0000_9c00, a);
        Ret::default()
    });
    for (wheel, clicked) in [
        (0i32, vec![]),
        (1, vec![(0x0000_9c00, vec![menu, 2, 0])]),
        (120, vec![(0x0000_9c00, vec![menu, 2, 0])]),
        (-1, vec![(0x0000_9c00, vec![menu, 1, 0])]),
        (i32::MIN, vec![(0x0000_9c00, vec![menu, 1, 0])]),
    ] {
        e.mem.set_u32(manager + 0x44, wheel as u32);
        assert_eq!(e.site(QUANTITY_WHEEL.addr(), &[menu]), None);
        assert_eq!(gb3_calls(), clicked, "{wheel}");
    }
}

// ---- G64: FixVendorCaps ----

/// The caps' form, another item, the buyer, the data handler, the barter container's data and the
/// buyer's data.
const CAPS: u32 = 0x0641_0000;
const OTHER: u32 = 0x0641_0200;
const OWNER: u32 = 0x0641_0100;
const SOMEONE: u32 = 0x0641_0300;
const HANDLER: u32 = 0x0642_0000;
const BARTER: u32 = 0x0642_1000;
const BUYER: u32 = 0x0642_2000;
/// The caps entries' item words (the barter container's, the buyer's).
const BARTER_CAPS_ITEM: u32 = AREA_AT + 0x4018;
const BUYER_CAPS_ITEM: u32 = AREA_AT + 0x4118;

/// The game's form map holding only the caps (id 0xF).
fn caps_form(e: &mut Engine) {
    if !e.mem.is_mapped(0x011c_54c0) {
        e.map(0x011c_5000, 0x1000);
    }
    let (map, buckets, item) = (AREA_AT + 0x3000, AREA_AT + 0x3100, AREA_AT + 0x3200);
    e.mem.set_u32(0x011c_54c0, map);
    e.mem.set_u32(map + 4, 16);
    e.mem.set_u32(map + 8, buckets);
    e.mem.set_u32(item + 4, 0xf);
    e.mem.set_u32(item + 8, CAPS);
    e.mem.set_u32(buckets + 4 * 0xf, item);
}

/// An extra data list's extras (the chain head at +4, next at +8 of each extra).
fn chain(e: &Engine, list: u32) -> Vec<u32> {
    let mut out = vec![];
    let mut at = e.mem.u32(list + 4);
    while at != 0 {
        out.push(at);
        at = e.mem.u32(at + 8);
    }
    out
}

/// The game's list functions stood in (`GetByType` by the type byte, `AddExtra` appends) and
/// the heap (blocks 0x20 apart, filled with 0xAA so JIP's zeroing shows).
fn list_world(e: &mut Engine) {
    e.register(GET_BY_TYPE, |e, a| {
        let found = chain(e, a[0])
            .into_iter()
            .find(|&x| u32::from(e.mem.u8(x + 4)) == a[1])
            .unwrap_or(0);
        found.into_ret()
    });
    e.register(ADD_EXTRA, |e, a| {
        gb3_log(ADD_EXTRA, a);
        match chain(e, a[0]).last() {
            Some(&last) => e.mem.set_u32(last + 8, a[1]),
            None => e.mem.set_u32(a[0] + 4, a[1]),
        }
        a[1].into_ret()
    });
    e.register(HEAP_ALLOCATE, |e, a| {
        gb3_log(HEAP_ALLOCATE, a);
        let at = GB3_NEXT.with(|n| {
            let at = n.get();
            n.set(at + 0x20);
            at
        });
        for i in 0..8 {
            e.mem.set_u32(at + 4 * i, 0xaaaa_aaaa);
        }
        at.into_ret()
    });
}

/// A list node (item, next) at `at`.
fn node(e: &mut Engine, at: u32, item: u32, next: u32) -> u32 {
    e.mem.set_u32(at, item);
    e.mem.set_u32(at + 4, next);
    at
}

/// An entry (`ContChangesEntry`: its stack list, count, item) at `at`.
fn entry(e: &mut Engine, at: u32, stacks: u32, count: i32, item: u32) -> u32 {
    e.mem.set_u32(at, stacks);
    e.mem.set_u32(at + 4, count as u32);
    e.mem.set_u32(at + 8, item);
    at
}

/// An extra data list at `at` holding extras of (type, value at +0xC).
fn extra_list(e: &mut Engine, at: u32, extras: &[(u8, u32)]) -> u32 {
    e.mem.set_u32(at, 0x0101_43e8);
    e.mem.set_u32(at + 4, 0);
    let mut previous = 0;
    for (i, &(kind, value)) in extras.iter().enumerate() {
        let x = at + 0x40 + 0x10 * i as u32;
        e.mem.set_u32(x + 4, u32::from(kind));
        e.mem.set_u32(x + 8, 0);
        e.mem.set_u32(x + 0xc, value);
        if previous == 0 {
            e.mem.set_u32(at + 4, x);
        } else {
            e.mem.set_u32(previous + 8, x);
        }
        previous = x;
    }
    at
}

/// The barter container (the handler's +0x628) holding another item, then the caps entry with
/// `stacks` (a list's head node; 0: none); the buyer (`OWNER`) holding an empty node, another
/// item, then `held` caps.
fn vendor_world(e: &mut Engine, stacks: u32, held: i32) {
    list_world(e);
    caps_form(e);
    e.mem.set_u32(HANDLER + 0x628, BARTER);
    let other = entry(e, AREA_AT + 0x4000, 0, 3, OTHER);
    let caps = entry(e, AREA_AT + 0x4010, stacks, 5, CAPS);
    let last = node(e, AREA_AT + 0x4028, caps, 0);
    let first = node(e, AREA_AT + 0x4020, other, last);
    e.mem.set_u32(BARTER, first);
    let other = entry(e, AREA_AT + 0x4100, 0, 9, OTHER);
    let held = entry(e, AREA_AT + 0x4110, 0, held, CAPS);
    let last = node(e, AREA_AT + 0x4138, held, 0);
    let middle = node(e, AREA_AT + 0x4130, other, last);
    let first = node(e, AREA_AT + 0x4120, 0, middle);
    e.mem.set_u32(BUYER, first);
    e.mem.set_u32(BUYER + 4, OWNER);
}

/// The items of a list from its head node, in order.
fn stacks(e: &Engine, mut at: u32) -> Vec<u32> {
    let mut out = vec![];
    while at != 0 {
        out.push(e.mem.u32(at));
        at = e.mem.u32(at + 4);
    }
    out
}

/// The `n`th node of a list from its head node.
fn nth_node(e: &Engine, mut at: u32, n: usize) -> u32 {
    for _ in 0..n {
        at = e.mem.u32(at + 4);
    }
    at
}

/// A new stack: JIP's list (vtable `0x10143E8`, the rest zeroed) with an `ExtraOriginalReference`
/// of the buyer, then an `ExtraCount` of `count` (each `CreateExtraType`: vtable, type, 0, value;
/// the stand-in `AddExtra` links the first to the second at +8).
fn assert_new_stack(e: &Engine, list: u32, count: u32) {
    assert_eq!(e.mem.u32(list), 0x0101_43e8);
    for i in 2..8 {
        assert_eq!(e.mem.u32(list + 4 * i), 0, "zeroed word {i}");
    }
    let extras = chain(e, list);
    assert_eq!(extras.len(), 2);
    assert_eq!(
        [0, 4, 8, 0xc].map(|o| e.mem.u32(extras[0] + o)),
        [0x0101_5bc4, 0x20, extras[1], OWNER]
    );
    assert_eq!(
        [0, 4, 8, 0xc].map(|o| e.mem.u32(extras[1] + o)),
        [0x0101_58d8, 0x24, 0, count]
    );
}

/// The buyer holds 32,767 caps or fewer: nothing changes (no allocation, no extra touched).
#[test]
fn g64_at_most_32767_changes_nothing() {
    for held in [0, 1, 0x7fff, -5] {
        let mut e = fixes_on();
        let mine = extra_list(&mut e, AREA_AT + 0x5000, &[(0x20, OWNER), (0x24, 0x1234)]);
        let head = node(&mut e, AREA_AT + 0x5800, mine, 0);
        vendor_world(&mut e, head, held);
        assert_eq!(e.site(VENDOR_CAPS.addr(), &[HANDLER, BUYER]), None);
        assert!(gb3_calls().is_empty(), "{held}");
        assert_eq!(e.mem.u32(chain(&e, mine)[1] + 0xc), 0x1234);
        assert_eq!(stacks(&e, head), [mine]);
    }
}

/// The barter container's caps entry has no stack list or is missing, the buyer holds no caps, or
/// there is no barter container data (nv-rs's own: JIP would read through it): nothing changes.
#[test]
fn g64_without_the_entries_changes_nothing() {
    for case in 0..4 {
        let mut e = fixes_on();
        let mine = extra_list(&mut e, AREA_AT + 0x5000, &[(0x20, OWNER), (0x24, 1)]);
        let head = node(&mut e, AREA_AT + 0x5800, mine, 0);
        vendor_world(&mut e, if case == 0 { 0 } else { head }, 100_000);
        match case {
            1 => e.mem.set_u32(BARTER_CAPS_ITEM, OTHER + 1),
            2 => e.mem.set_u32(BUYER_CAPS_ITEM, OTHER + 1),
            3 => e.mem.set_u32(HANDLER + 0x628, 0),
            _ => {}
        }
        assert_eq!(e.site(VENDOR_CAPS.addr(), &[HANDLER, BUYER]), None);
        assert!(gb3_calls().is_empty(), "case {case}");
        assert_eq!(e.mem.u32(chain(&e, mine)[1] + 0xc), 1);
        assert_eq!(stacks(&e, head), [mine]);
    }
}

/// 100,000 caps held; the barter container's stacks: someone else's (its count kept), an empty
/// node, the buyer's (count first in its chain). The buyer's count becomes 32,767; the 67,233 left
/// go into new stacks of 32,767, 32,767 and 1,699, each put at the head (JIP's `Node::Insert`: the
/// head node takes the new list, a new node after it the old item), so the newest is first.
#[test]
fn g64_splits_the_buyers_caps() {
    let mut e = fixes_on();
    let theirs = extra_list(&mut e, AREA_AT + 0x5000, &[(0x20, SOMEONE), (0x24, 7)]);
    let mine = extra_list(
        &mut e,
        AREA_AT + 0x5200,
        &[(0x24, 0x7fff), (0x20, OWNER), (0x10, 0)],
    );
    let third = node(&mut e, AREA_AT + 0x5810, mine, 0);
    let second = node(&mut e, AREA_AT + 0x5808, 0, third);
    let head = node(&mut e, AREA_AT + 0x5800, theirs, second);
    vendor_world(&mut e, head, 100_000);
    assert_eq!(e.site(VENDOR_CAPS.addr(), &[HANDLER, BUYER]), None);
    assert_eq!(e.mem.u32(chain(&e, mine)[0] + 0xc), 0x7fff);
    assert_eq!(e.mem.u32(chain(&e, theirs)[1] + 0xc), 7);
    let lists = stacks(&e, head);
    assert_eq!(lists.len(), 6);
    assert_eq!(lists[3..], [theirs, 0, mine]);
    assert_new_stack(&e, lists[0], 1_699);
    assert_new_stack(&e, lists[1], 0x7fff);
    assert_new_stack(&e, lists[2], 0x7fff);
    assert_eq!(nth_node(&e, head, 4), second, "the old nodes kept");
    assert_eq!(nth_node(&e, head, 5), third);
    let allocs: Vec<Vec<u32>> = gb3_calls()
        .into_iter()
        .filter(|(a, _)| *a == HEAP_ALLOCATE)
        .map(|(_, w)| w[1..].to_vec())
        .collect();
    // Per new stack: the list (0x20), the two extras (0x10 each), the node (8).
    assert_eq!(allocs.len(), 12);
    assert_eq!(allocs[..4], [vec![0x20], vec![0x10], vec![0x10], vec![8]]);
}

/// Edges: 32,768 held (the buyer's stack takes 32,767, 1 is left: no new stack); 32,769 (2 left:
/// a new stack of 2); a buyer's stack with no count extra (no count set, nothing added to it: the
/// whole 40,000 goes into new stacks, 32,767 then 7,233); an empty head node (`Prepend` with no
/// item: the head takes the first new list itself).
#[test]
fn g64_edges() {
    for (held, new) in [(0x8000i32, vec![]), (0x8001, vec![2u32])] {
        let mut e = fixes_on();
        let mine = extra_list(&mut e, AREA_AT + 0x5200, &[(0x20, OWNER), (0x24, 9)]);
        let head = node(&mut e, AREA_AT + 0x5800, mine, 0);
        vendor_world(&mut e, head, held);
        e.site(VENDOR_CAPS.addr(), &[HANDLER, BUYER]);
        assert_eq!(e.mem.u32(chain(&e, mine)[1] + 0xc), 0x7fff, "{held}");
        let lists = stacks(&e, head);
        assert_eq!(lists.len(), 1 + new.len());
        for (i, &count) in new.iter().enumerate() {
            assert_new_stack(&e, lists[i], count);
        }
    }
    let mut e = fixes_on();
    let mine = extra_list(&mut e, AREA_AT + 0x5200, &[(0x20, OWNER)]);
    let head = node(&mut e, AREA_AT + 0x5800, mine, 0);
    vendor_world(&mut e, head, 40_000);
    e.site(VENDOR_CAPS.addr(), &[HANDLER, BUYER]);
    assert_eq!(chain(&e, mine).len(), 1, "no count extra added to it");
    let lists = stacks(&e, head);
    assert_eq!(lists.len(), 3);
    assert_new_stack(&e, lists[0], 7_233);
    assert_new_stack(&e, lists[1], 0x7fff);
    let mut e = fixes_on();
    let head = node(&mut e, AREA_AT + 0x5800, 0, 0);
    vendor_world(&mut e, head, 0x8001);
    e.site(VENDOR_CAPS.addr(), &[HANDLER, BUYER]);
    let lists = stacks(&e, head);
    assert_eq!(lists.len(), 2);
    assert_new_stack(&e, lists[0], 2);
    assert_new_stack(&e, lists[1], 0x7fff);
}

/// Only the first stack marked as the buyer's is counted: a second one of the buyer's keeps its
/// count (JIP stops the walk at the first match).
#[test]
fn g64_only_the_first_of_the_buyers_stacks() {
    let mut e = fixes_on();
    let first = extra_list(&mut e, AREA_AT + 0x5000, &[(0x20, OWNER), (0x24, 5)]);
    let second = extra_list(&mut e, AREA_AT + 0x5200, &[(0x20, OWNER), (0x24, 6)]);
    let tail = node(&mut e, AREA_AT + 0x5808, second, 0);
    let head = node(&mut e, AREA_AT + 0x5800, first, tail);
    vendor_world(&mut e, head, 0x8000);
    e.site(VENDOR_CAPS.addr(), &[HANDLER, BUYER]);
    assert_eq!(e.mem.u32(chain(&e, first)[1] + 0xc), 0x7fff);
    assert_eq!(e.mem.u32(chain(&e, second)[1] + 0xc), 6);
    assert_eq!(stacks(&e, head), [first, second]);
}

// ---- GB-4: G82 (item names), O9 (quantity short keys) ----

const HEAP_FREE: u32 = 0x00aa_4060;
/// The test's stack, its form, the tile, its value array and string value.
const STACK: u32 = AREA_AT + 0x6000;
const WEAPON: u32 = AREA_AT + 0x6100;
const NAME: u32 = AREA_AT + 0x6200;
const TILE: u32 = AREA_AT + 0x6300;
const STRING_VALUE: u32 = AREA_AT + 0x6400;
const OLD_TEXT: u32 = AREA_AT + 0x6500;

/// A stack of `name` (a form of `kind`) with no stack list, a tile with a string value holding
/// `old`; the heap, `AddExtra`, `GetByType` stand-ins; refreshes and frees logged.
fn names_world(kind: u8, name: &[u8], old: &[u8]) -> Engine {
    let mut e = fixes_on();
    list_world(&mut e);
    e.register(HEAP_FREE, |_, a| {
        gb3_log(HEAP_FREE, a);
        Ret::default()
    });
    e.register(VALUE_REFRESH, |_, a| {
        gb3_log(VALUE_REFRESH, a);
        Ret::default()
    });
    e.mem.set_u8(WEAPON + 4, kind);
    e.mem.write(NAME, name);
    e.mem.set_u32(WEAPON + 0x34, NAME);
    e.mem.set_u16(WEAPON + 0x38, name.len() as u16);
    entry(&mut e, STACK, 0, 1, WEAPON);
    let array = TILE + 0x40;
    let other = TILE + 0x80;
    e.mem.set_u32(other, 0xfa0);
    e.mem.set_u32(STRING_VALUE, 0xfc4);
    e.mem.set_u32(array, other);
    e.mem.set_u32(array + 4, STRING_VALUE);
    e.mem.set_u32(TILE + 0x14, array);
    e.mem.set_u32(TILE + 0x18, 2);
    if old.is_empty() {
        e.mem.set_u32(STRING_VALUE + 0xc, 0);
    } else {
        e.mem.write(OLD_TEXT, old);
        e.mem.set_u8(OLD_TEXT + old.len() as u32, 0);
        e.mem.set_u32(STRING_VALUE + 0xc, OLD_TEXT);
    }
    gb3_calls();
    e
}

/// The tile's string now.
fn shown(e: &Engine) -> Vec<u8> {
    match e.mem.u32(STRING_VALUE + 0xc) {
        0 => vec![],
        at => e.mem.cstr(at),
    }
}

/// Gives the stack a first list holding `ExtraWeaponModFlags` with `flags`.
fn modded(e: &mut Engine, flags: u8) {
    let list = extra_list(e, AREA_AT + 0x6800, &[(0x8d, u32::from(flags))]);
    let node_at = node(e, AREA_AT + 0x6a00, list, 0);
    e.mem.set_u32(STACK, node_at);
}

/// G82, JIP's `ConstructItemEntryNameHook`: one plain item gets nothing; more than one gets
/// "Name (count)"; a modded weapon "Name+" alone and "Name+ (count)" with more than one (the
/// count written as JIP's `IntToStr`); each answers 1 (the exe's block skipped). The text is copied
/// to the game's heap, the old one freed, the value refreshed once.
#[test]
fn g82_names_the_entry() {
    let site = ITEM_NAMES[2].addr();
    let mut e = names_world(0x28, b"Laser Rifle", b"old");
    assert_eq!(e.site(site, &[TILE, STACK, 1]), Some(1));
    assert_eq!(shown(&e), b"old", "one plain item: nothing set");
    assert!(gb3_calls().is_empty());
    assert_eq!(e.site(site, &[TILE, STACK, 3]), Some(1));
    assert_eq!(shown(&e), b"Laser Rifle (3)");
    let calls = gb3_calls();
    assert_eq!(
        calls.iter().map(|c| c.0).collect::<Vec<_>>(),
        [HEAP_FREE, HEAP_ALLOCATE, VALUE_REFRESH]
    );
    assert_eq!(calls[0].1[1], OLD_TEXT);
    assert_eq!(calls[1].1[1], 16, "the text and its end");
    assert_eq!(calls[2].1, [STRING_VALUE, 1]);
    modded(&mut e, 2);
    assert_eq!(e.site(site, &[TILE, STACK, 1]), Some(1));
    assert_eq!(shown(&e), b"Laser Rifle+");
    assert_eq!(e.site(ITEM_NAMES[0].addr(), &[TILE, STACK, 12]), Some(1));
    assert_eq!(shown(&e), b"Laser Rifle+ (12)");
    assert_eq!(
        e.site(ITEM_NAMES[1].addr(), &[TILE, STACK, -2i32 as u32]),
        Some(1)
    );
    assert_eq!(shown(&e), b"Laser Rifle+");
    // The same text again: nothing freed, nothing copied, no refresh (JIP's compare).
    gb3_calls();
    assert_eq!(e.site(site, &[TILE, STACK, 1]), Some(1));
    assert!(gb3_calls().is_empty());
}

/// Not a weapon: never "+" (JIP's mod flags answer 0), a stack of more than one still counted; a
/// form with an empty name: nothing set; a tile without a string value: nothing set; no old text:
/// nothing freed.
#[test]
fn g82_other_forms_and_empty_names() {
    let site = ITEM_NAMES[2].addr();
    let mut e = names_world(0x1f, b"Scrap Metal", b"");
    modded(&mut e, 1);
    assert_eq!(e.site(site, &[TILE, STACK, 1]), Some(1));
    assert_eq!(shown(&e), b"");
    assert_eq!(e.site(site, &[TILE, STACK, 2]), Some(1));
    assert_eq!(shown(&e), b"Scrap Metal (2)");
    assert!(!gb3_calls().iter().any(|c| c.0 == HEAP_FREE));
    let mut e = names_world(0x28, b"", b"old");
    modded(&mut e, 1);
    assert_eq!(e.site(site, &[TILE, STACK, 5]), Some(1));
    assert_eq!(shown(&e), b"old");
    let mut e = names_world(0x28, b"Laser Rifle", b"old");
    e.mem.set_u32(STRING_VALUE, 0xfc5);
    assert_eq!(e.site(site, &[TILE, STACK, 5]), Some(1));
    assert_eq!(e.mem.u32(STRING_VALUE + 0xc), OLD_TEXT);
    assert!(gb3_calls().is_empty());
}

/// nv-rs's own: a name that does not fit the 0x80-byte buffer is cut to 127 characters.
#[test]
fn g82_cuts_to_the_buffer() {
    let long = [b'x'; 200];
    let mut e = names_world(0x28, &long, b"");
    assert_eq!(e.site(ITEM_NAMES[2].addr(), &[TILE, STACK, 7]), Some(1));
    assert_eq!(shown(&e), [b'x'; 127]);
}

/// The key state copy the built-in xNVSE publishes, with `keys` held (raw state) and others
/// pressed only in the game's state.
fn keys(e: &mut Engine, held: &[u32], game_only: &[u32]) {
    for k in [0x10, 0x1d, 0x9d, 0x2a, 0x36] {
        let mut info = crate::test_support::key(e, k);
        info.raw_state = held.contains(&k);
        info.game_state = game_only.contains(&k);
    }
}

/// O9 (JIP's three `QttSelect` hooks): either Ctrl (raw state) answers 1, either Shift the count,
/// Ctrl first; no key, or a key pressed only in the game's state, answers nothing. The same at
/// each of the three seams.
#[test]
fn o9_ctrl_one_shift_all() {
    let mut e = fixes_on();
    for slot in QUANTITY_KEYS {
        let site = slot.addr();
        for (held, answer) in [
            (vec![], None),
            (vec![0x1d], Some(1)),
            (vec![0x9d], Some(1)),
            (vec![0x2a], Some(37)),
            (vec![0x36], Some(37)),
            (vec![0x2a, 0x1d], Some(1)),
            (vec![0x10], None),
        ] {
            keys(&mut e, &held, &[]);
            assert_eq!(e.site(site, &[37]), answer, "{site:08x} {held:?}");
        }
        keys(&mut e, &[], &[0x1d, 0x2a]);
        assert_eq!(e.site(site, &[37]), None, "game state only");
    }
    assert_eq!(e.site(WEIGHTLESS_TAKE.addr(), &[]), Some(1));
}

/// The rows: `WritePushRetRelJump(0x72F33D/0x75D1A5/0x782876, ..)` and O9's `HOOK_INIT_LIST`
/// (`0x72D8B4`, `0x75BF97`, `0x780B09`, `0x75BF18`).
#[test]
fn gb4_rows_are_jips_addresses() {
    assert_eq!(
        ITEM_NAMES.map(|s| s.addr()),
        [0x0072_f33d, 0x0075_d1a5, 0x0078_2876]
    );
    assert_eq!(
        QUANTITY_KEYS.map(|s| s.addr()),
        [0x0072_d8b4, 0x0075_bf97, 0x0078_0b09]
    );
    assert_eq!(WEIGHTLESS_TAKE.addr(), 0x0075_bf18);
    assert_eq!(
        AREA.fix_sites[7..14],
        [
            0x0072_f33d,
            0x0075_d1a5,
            0x0078_2876,
            0x0072_d8b4,
            0x0075_bf97,
            0x0078_0b09,
            0x0075_bf18
        ]
    );
}

// ---- GB-5: O2 (Fallout 3 style repair) ----

const ACTOR_VALUE: u32 = 0x0093_acb0;
const ITEM_TEST: u32 = 0x0047_bb50;

/// A weapon stack at `at` (its form after it: type 0x28, full health 200) with a first list
/// holding `ExtraHealth` `health` (none: no list).
fn weapon_stack(e: &mut Engine, at: u32, health: Option<f32>) -> u32 {
    let form = at + 0x100;
    e.mem.set_u8(form + 4, 0x28);
    e.mem.set_u32(form + 0x98, 200);
    let lists = match health {
        Some(h) => {
            let list = extra_list(e, at + 0x200, &[(0x25, h.to_bits())]);
            node(e, at + 0x300, list, 0)
        }
        None => 0,
    };
    entry(e, at, lists, 1, form)
}

/// The repair settings (`fRepairSkillBase` 2, `fRepairSkillMax` 10, `fRepairScavengeMult` 0.5,
/// `fRepairMin` 1, `fRepairMax` 3), the player and the Repair skill `skill`.
fn repair_world(e: &mut Engine, skill: f64) {
    list_world(e);
    for page in [0x011d_0000, 0x011c_f000, 0x011d_e000, 0x011d_a000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_f32(0x011d_0ff4, 2.0);
    e.mem.set_f32(0x011d_0190, 10.0);
    e.mem.set_f32(0x011d_074c, 0.5);
    e.mem.set_f32(0x011d_0884, 1.0);
    e.mem.set_f32(0x011c_ffa4, 3.0);
    e.mem.set_u32(0x011d_ea3c, AREA_AT + 0x7000);
    e.register_double(ACTOR_VALUE, move |_, a| {
        gb3_log(ACTOR_VALUE, a);
        Ret {
            st0: skill,
            ..Ret::default()
        }
    });
}

/// JIP's `GetHealthPercent`: the `ExtraHealth` over the full health times 100, at most 100; 100
/// without a list; -1 for a type with a bit outside 0x38 (ammunition, 0x29).
#[test]
fn o2_health_percent() {
    let mut e = fixes_on();
    repair_world(&mut e, 0.0);
    let stack = weapon_stack(&mut e, AREA_AT + 0x8000, Some(150.0));
    assert_eq!(health_percent(&mut e, stack), 75.0);
    let stack = weapon_stack(&mut e, AREA_AT + 0x8000, Some(300.0));
    assert_eq!(health_percent(&mut e, stack), 100.0);
    let stack = weapon_stack(&mut e, AREA_AT + 0x9000, None);
    assert_eq!(health_percent(&mut e, stack), 100.0);
    e.mem.set_u8(AREA_AT + 0x9100 + 4, 0x29);
    assert_eq!(health_percent(&mut e, stack), -1.0);
    // Armour (0x18) passes the type test (a bit outside 0x38 only fails): 50 of its 100.
    e.mem.set_u8(AREA_AT + 0x8100 + 4, 0x18);
    e.mem.set_u32(AREA_AT + 0x8100 + 0x6c, 100);
    let armour = entry(&mut e, AREA_AT + 0x8000, AREA_AT + 0x8300, 1, AREA_AT + 0x8100);
    let list = extra_list(&mut e, AREA_AT + 0x8200, &[(0x25, 50.0f32.to_bits())]);
    node(&mut e, AREA_AT + 0x8300, list, 0);
    assert_eq!(health_percent(&mut e, armour), 50.0);
}

/// `EnableRepairButtonHook`: the cap is the skill scaled from 20 to 100 (here 50 gives 60, -5
/// gives 20), the health rounded up (55.2 becomes 56); 1 when the cap is above it, else 0; the
/// three statics kept; the skill read as JIP does (`0093acb0` on the player's +0xA4, 0x27).
#[test]
fn o2_repair_button() {
    let mut e = fixes_on();
    repair_world(&mut e, 50.0);
    let stack = weapon_stack(&mut e, AREA_AT + 0x8000, Some(110.4));
    gb3_calls();
    assert_eq!(e.site(REPAIR_BUTTON.addr(), &[stack]), Some(1));
    assert_eq!(gb3_calls(), [(ACTOR_VALUE, vec![AREA_AT + 0x70a4, 0x27])]);
    let state = repair_state(&mut e);
    assert_eq!(f32::from_bits(state.skill.load(Ordering::Relaxed)), 60.0);
    assert_eq!(f32::from_bits(state.health.load(Ordering::Relaxed)), 56.0);
    assert!(!state.modded.load(Ordering::Relaxed));
    let stack = weapon_stack(&mut e, AREA_AT + 0x8000, Some(130.0));
    assert_eq!(
        e.site(REPAIR_BUTTON.addr(), &[stack]),
        Some(0),
        "65 is above 60"
    );
    let stack = weapon_stack(&mut e, AREA_AT + 0x8000, Some(119.0));
    assert_eq!(
        e.site(REPAIR_BUTTON.addr(), &[stack]),
        Some(0),
        "59.5 rounds up to 60, not above 60"
    );
    let mut e = fixes_on();
    repair_world(&mut e, -5.0);
    let stack = weapon_stack(&mut e, AREA_AT + 0x8000, Some(30.0));
    e.mem.set_u8(AREA_AT + 0x8200 + 0x19, 0x20);
    assert_eq!(
        e.site(REPAIR_BUTTON.addr(), &[stack]),
        Some(1),
        "15 below 20"
    );
    let state = repair_state(&mut e);
    assert_eq!(f32::from_bits(state.skill.load(Ordering::Relaxed)), 20.0);
    assert!(
        state.modded.load(Ordering::Relaxed),
        "the list's mod flags bit"
    );
}

/// JIP's rounding up: whole values stay, others go up, out of range is -2^31.
#[test]
fn o2_round_up() {
    assert_eq!(round_up(56.0), 56.0);
    assert_eq!(round_up(55.01), 56.0);
    assert_eq!(round_up(-0.5), 0.0);
    assert_eq!(round_up(f32::NAN), -2_147_483_648.0);
    assert_eq!(round_up(3.0e9), -2_147_483_648.0);
}

/// `GetRepairAmount` with the cap 60: healths 20 and 10 give (2 + 1 + 1 * 0.5 + 2 * 0.6) / 10,
/// in JIP's single-precision steps; a larger result is cut to the cap's 0.6.
#[test]
fn o2_repair_amount() {
    let mut e = fixes_on();
    repair_world(&mut e, 0.0);
    repair_state(&mut e)
        .skill
        .store(60.0f32.to_bits(), Ordering::Relaxed);
    let s = 60.0f32 * 0.01;
    let expected = (20.0f32 * 0.1 + 1.0 + 10.0f32 * 0.1 * 0.5 + (3.0f32 - 1.0) * s) * 0.1;
    assert!(expected < s);
    assert_eq!(repair_amount(&mut e, 20.0, 10.0), expected);
    assert_eq!(
        repair_amount(&mut e, 10.0, 20.0),
        expected,
        "the larger, the smaller"
    );
    e.mem.set_f32(0x011d_0884, 40.0);
    assert_eq!(repair_amount(&mut e, 50.0, 30.0), s);
    let bits = repair_amount(&mut e, 70.0, 10.0).to_bits();
    assert_eq!(
        e.site(
            REPAIR_LIST_VALUE.addr(),
            &[70.0f32.to_bits(), 10.0f32.to_bits()]
        ),
        Some(bits)
    );
}

/// `PopulateRepairListHook` (words: the broken item's form, a stack): the exe's test decides;
/// a stack of the broken item's own form passes only at or below the kept health unless the
/// broken item has mods.
#[test]
fn o2_repair_list() {
    let mut e = fixes_on();
    repair_world(&mut e, 0.0);
    e.register(ITEM_TEST, |e, a| {
        gb3_log(ITEM_TEST, a);
        u32::from(e.mem.u8(a[0] + 0x10) == 0).into_ret()
    });
    let stack = weapon_stack(&mut e, AREA_AT + 0x8000, Some(100.0));
    let form = AREA_AT + 0x8100;
    let state = repair_state(&mut e);
    state.health.store(40.0f32.to_bits(), Ordering::Relaxed);
    state.modded.store(false, Ordering::Relaxed);
    gb3_calls();
    assert_eq!(
        e.site(REPAIR_LIST.addr(), &[form, stack]),
        Some(0),
        "50 above 40"
    );
    assert_eq!(gb3_calls(), [(ITEM_TEST, vec![form, form])]);
    assert_eq!(
        e.site(REPAIR_LIST.addr(), &[0x0641_0000, stack]),
        Some(1),
        "another form"
    );
    repair_state(&mut e)
        .health
        .store(50.0f32.to_bits(), Ordering::Relaxed);
    assert_eq!(
        e.site(REPAIR_LIST.addr(), &[form, stack]),
        Some(1),
        "50 at 50"
    );
    repair_state(&mut e)
        .health
        .store(10.0f32.to_bits(), Ordering::Relaxed);
    repair_state(&mut e).modded.store(true, Ordering::Relaxed);
    assert_eq!(
        e.site(REPAIR_LIST.addr(), &[form, stack]),
        Some(1),
        "modded"
    );
    e.mem.set_u8(form + 0x10, 1);
    assert_eq!(
        e.site(REPAIR_LIST.addr(), &[form, stack]),
        Some(0),
        "the exe's test"
    );
}

/// `DoRepairItemHook` and `RepairMenuClickHook`: the amount from the broken item's (`0x11DA760`)
/// and the used stack's health in percent; set up again while the broken item's health + 1 is at
/// most the cap.
#[test]
fn o2_repair_item_and_again() {
    let mut e = fixes_on();
    repair_world(&mut e, 0.0);
    repair_state(&mut e)
        .skill
        .store(60.0f32.to_bits(), Ordering::Relaxed);
    let broken = weapon_stack(&mut e, AREA_AT + 0x8000, Some(20.0));
    let used = weapon_stack(&mut e, AREA_AT + 0xa000, Some(40.0));
    e.mem.set_u32(0x011d_a760, broken);
    let expected = repair_amount(&mut e, 10.0, 20.0).to_bits();
    assert!(f32::from_bits(expected) < 0.6, "below the cap, so the two healths count");
    assert_ne!(expected, repair_amount(&mut e, 20.0, 20.0).to_bits());
    assert_eq!(e.site(REPAIR_ITEM.addr(), &[used]), Some(expected));
    assert_eq!(e.site(REPAIR_AGAIN.addr(), &[]), Some(1), "11 at most 60");
    let broken = weapon_stack(&mut e, AREA_AT + 0x8000, Some(118.0));
    e.mem.set_u32(0x011d_a760, broken);
    assert_eq!(e.site(REPAIR_AGAIN.addr(), &[]), Some(1), "60 at 60");
    let broken = weapon_stack(&mut e, AREA_AT + 0x8000, Some(119.0));
    e.mem.set_u32(0x011d_a760, broken);
    assert_eq!(e.site(REPAIR_AGAIN.addr(), &[]), Some(0), "60.5 above 60");
}

/// O2's rows: `HOOK_INIT_LIST(FO3Repair, ..)` (`0x7818B2`, `0x4D4C11`, `0x7B58DB`, `0x7B5DAD`,
/// `0x7B5CED`; the two byte writes folded into the first and the last).
#[test]
fn gb5_rows_are_jips_addresses() {
    assert_eq!(
        AREA.fix_sites[14..],
        [
            0x004d_4c11,
            0x0078_18b2,
            0x007b_58db,
            0x007b_5ced,
            0x007b_5dad
        ]
    );
}

/// G82 reads every stack's form at +0x34/+0x38 as JIP does (REVIEW-683): for a note (`BGSNote`,
/// 0x31) that word pair is not its name (its model path, as the reviewer found), and a stack of
/// two shows that text with " (2)", exactly as with JIP.
#[test]
fn g82_reads_a_note_at_jips_offsets_too() {
    let mut e = names_world(0x31, b"Clutter\Notes\Note01.nif", b"");
    assert_eq!(e.site(ITEM_NAMES[2].addr(), &[TILE, STACK, 2]), Some(1));
    assert_eq!(shown(&e), b"Clutter\Notes\Note01.nif (2)");
}
