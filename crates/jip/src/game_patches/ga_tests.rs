//! Group GA's game patches: G10.

use super::*;
use crate::test_support::engine;

/// JIP's first byte in each of the three functions, rising; the area lists them as fixes only.
#[test]
fn g10_seams_are_jips_addresses() {
    let sites: Vec<u32> = CORNER_MESSAGE.iter().map(|s| s.addr()).collect();
    assert_eq!(sites, [0x0077_14da, 0x0077_53b8, 0x0077_57b1]);
    assert_eq!(AREA.fix_sites, sites);
    assert!(AREA.fix_wraps.is_empty() && AREA.jip_sites.is_empty() && AREA.jip_wraps.is_empty());
}

/// Not installed: nothing answers.
#[test]
fn g10_not_installed_nothing_answers() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    for slot in CORNER_MESSAGE {
        assert_eq!(e.site(slot.addr(), &[0x104]), None);
    }
}

/// Installed with the row off: nothing; with the fixes in, each of the three seams answers
/// JIP's 0x204 (0x203 letters: JIP's `SafeWrite8(0x7753D2, 2)` makes `PUSH 0x104` `PUSH 0x204`).
#[test]
fn g10_each_seam_answers_jips_text_buffer() {
    let mut e = engine();
    crate::install_as(&mut e, true);
    for slot in CORNER_MESSAGE {
        assert_eq!(e.site(slot.addr(), &[0x104]), None, "row off");
    }
    install_fixes(&mut e);
    for slot in CORNER_MESSAGE {
        assert_eq!(e.site(slot.addr(), &[0x104]), Some(0x204));
    }
    assert_eq!(corner_message_text(&mut e, &[0x104]), Some(0x204));
}
