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
    assert!(AREA.fix_sites.is_empty(), "all JIP's own (INSTALLED)");
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
