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
    e.ext::<Jip>().input.dead_zone_ls = 4000;
    e.ext::<Jip>().input.dead_zone_rs = -1;
    let (left, right) = doubles(&mut e);
    assert_eq!(e.mem.f64(left), 4000.0);
    assert_eq!(e.mem.f64(right), -1.0);
    e.ext::<Jip>().input.dead_zone_ls = 1;
    assert_eq!(doubles(&mut e), (left, right), "made once");
    assert_eq!(e.mem.f64(left), 4000.0);
}
