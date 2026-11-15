//! Group GS's game patches: O7.

use super::*;
use crate::test_support::engine;

/// JIP's addresses (`SetOptionalPatch` case 7; Ghidra: `8B 4D 08` at each, the start of `MOV
/// ECX,[EBP+8]; CALL 00500940; MOV [EAX+8],0`).
#[test]
fn o7_seams_are_jips_addresses() {
    let sites: Vec<u32> = FAILED_SCRIPT_LOCKS.iter().map(|s| s.addr()).collect();
    assert_eq!(sites, [0x005e_0fbe, 0x005e_1a23, 0x005e_1f0b]);
    assert_eq!(AREA.fix_sites, sites);
}

/// Not installed: nothing registered at O7's seams.
#[test]
fn o7_not_installed_nothing_answers() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    for slot in FAILED_SCRIPT_LOCKS {
        assert_eq!(e.site(slot.addr(), &[]), None);
    }
}

/// Installed with the row off: no handler; with the area's fixes in, every seam takes the call
/// (the seams in the translation test the switch's bit before asking).
#[test]
fn o7_handlers_take_the_call_with_the_fixes() {
    let mut e = engine();
    crate::install_as(&mut e, true);
    for slot in FAILED_SCRIPT_LOCKS {
        assert_eq!(e.site(slot.addr(), &[]), None, "row off");
    }
    install_fixes(&mut e);
    for slot in FAILED_SCRIPT_LOCKS {
        assert_eq!(e.site(slot.addr(), &[]), Some(1));
    }
}
