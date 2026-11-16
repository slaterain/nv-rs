//! Group GS's game patches: O7, O8, G03, G34, G70, G71, G123. The expected values come from
//! JIP's assembly (`DoOperatorHook`, `TerminalGetLockedHook`, `GetHitIMODHook`, the byte patches)
//! and the exe's code at each place (doubles checked with an independent IEEE computation); the
//! seams' use in the translations is tested beside them (`expression.rs`,
//! `tesconditionfunctions.rs`, `tesscriptfunctions_p2.rs`, `tesscriptfunctions_p6.rs`).

use super::*;
use crate::test_support::engine;
use engine_core::extend::jip_patches;

const AREA_AT: u32 = 0x0500_0000;
const CURRENT: u32 = AREA_AT + 0x100;
const TERMINAL: u32 = AREA_AT + 0x200;
const REF: u32 = AREA_AT + 0x300;
const EXTRA: u32 = AREA_AT + 0x400;
const RENDER_STATE: u32 = AREA_AT + 0x500;

/// Every row at the address JIP patches (`patches_game.h`: `WriteRelJump(0x593FBC, ...)`,
/// `WriteRelCall(0x59D0B3, ...)`, `SAFE_WRITE_BUF(0x59E034, ...)`, `SAFE_WRITE_BUF(0x5C240D, ...)`,
/// `SafeWrite16(0x5D3B69, ...)`, `WriteRelJump(0x5D2860, ...)`, case 7's three), in the area's lists.
#[test]
fn rows_are_jips_addresses() {
    assert_eq!(DO_OPERATOR.addr(), 0x0059_3fbc);
    assert_eq!(TERMINAL_UNLOCKED.addr(), 0x0059_d0b3);
    assert_eq!(FURNITURE_PROCESS.addr(), 0x0059_e034);
    assert_eq!(SOUND_NODE.addr(), 0x005c_240d);
    assert_eq!(ACHIEVEMENT.addr(), 0x005d_3b69);
    assert_eq!(GET_HIT_IMOD.addr(), 0x005d_2860);
    let sites: Vec<u32> = FAILED_SCRIPT_LOCKS.iter().map(|s| s.addr()).collect();
    assert_eq!(sites, [0x005e_0fbe, 0x005e_1a23, 0x005e_1f0b]);
    assert_eq!(
        AREA.fix_sites,
        [
            0x0059_3fbc,
            0x0059_d0b3,
            0x0059_e034,
            0x005c_240d,
            0x005d_3b69,
            0x005e_0fbe,
            0x005e_1a23,
            0x005e_1f0b
        ]
    );
    assert!(AREA.fix_wraps.is_empty() && AREA.jip_sites.is_empty());
    assert_eq!(AREA.jip_wraps, [0x005d_2860]);
}

/// Not installed: nothing registered at any seam, no wrap.
#[test]
fn not_installed_nothing_answers() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    for &addr in AREA.fix_sites {
        assert_eq!(e.site(addr, &[0, 0, 0, 0, 0, 0, 0, CURRENT]), None);
    }
    assert!(e.wraps_at(GET_HIT_IMOD.addr()).is_empty());
}

fn installed() -> Engine {
    let mut e = engine();
    e.map(AREA_AT, 0x1000);
    crate::install_as(&mut e, true);
    e
}

/// Installed with the row off: G123's wrap in (JIP's own), no fix handler; with the fixes in,
/// each handler answers at ITS address as only it does (a handler at another row's address fails).
#[test]
fn each_handler_is_at_its_row_after_install() {
    let mut e = installed();
    assert_eq!(e.wraps_at(GET_HIT_IMOD.addr()), [Owner::Jip]);
    for &addr in AREA.fix_sites {
        assert_eq!(
            e.site(addr, &[0, 0, 0, 0, 0, 0, 0, CURRENT]),
            None,
            "row off"
        );
    }
    install_fixes(&mut e);
    for slot in FAILED_SCRIPT_LOCKS {
        assert_eq!(e.site(slot.addr(), &[]), Some(1));
    }
    assert_eq!(e.site(ACHIEVEMENT.addr(), &[]), Some(1));
    // O8: 7 - 3 with integers writes 4 and the integer type.
    e.mem.set_u32(CURRENT, 0xaaaa_aa01);
    assert_eq!(
        e.site(DO_OPERATOR.addr(), &[8, 0, 7, 0, 0, 3, 0, CURRENT]),
        Some(1)
    );
    assert_eq!(e.mem.u32(CURRENT), 0xaaaa_aa00);
    assert_eq!(e.mem.u32(CURRENT + 8), 4);
    // G71: a null process answers 0, another the exe's call.
    assert_eq!(e.site(FURNITURE_PROCESS.addr(), &[0]), Some(0));
    assert_eq!(e.site(FURNITURE_PROCESS.addr(), &[0x0600_0000]), None);
    // G70: no reference: the terminal's flag.
    e.mem.set_u8(TERMINAL + 0xa5, 2);
    assert_eq!(e.site(TERMINAL_UNLOCKED.addr(), &[TERMINAL, 0]), Some(1));
    // G34: the reference's root node.
    e.mem.set_u32(REF + 0x64, RENDER_STATE);
    e.mem.set_u32(RENDER_STATE + 0x14, 0x0700_0010);
    assert_eq!(e.site(SOUND_NODE.addr(), &[REF]), Some(0x0700_0010));
}

// ---- O8: DoOperatorHook ----

/// Index of each operator (the exe's operator number less 2).
const AND: u32 = 0;
const OR: u32 = 1;
const LE: u32 = 2;
const LT: u32 = 3;
const GE: u32 = 4;
const GT: u32 = 5;
const EQ: u32 = 6;
const NE: u32 = 7;
const SUB: u32 = 8;
const ADD: u32 = 9;
const MUL: u32 = 10;
const DIV: u32 = 11;
const MOD: u32 = 12;

/// An operand's three words: (type word, low value word, high value word). The type words carry
/// other bytes above the type byte, which JIP's `or al, [ebp-0x28]` and `shl al, 1` ignore.
fn int(v: i32) -> [u32; 3] {
    [0x5a5a_5a00, v as u32, 0]
}
/// An integer whose high word is not 0 (what the operand's 8 bytes hold from before).
fn int_high(v: i32, high: u32) -> [u32; 3] {
    [0x5a5a_5a00, v as u32, high]
}
fn dbl(bits: u64) -> [u32; 3] {
    [0x3c3c_3c01, bits as u32, (bits >> 32) as u32]
}
fn flt(v: f64) -> [u32; 3] {
    dbl(v.to_bits())
}

/// The handler's result: (type word, low value word, high value word), the current operand set
/// beforehand to a type word of 0xEEEEEE77 and the value words 0x11111111, 0x22222222.
fn op(index: u32, l: [u32; 3], r: [u32; 3]) -> (u32, u32, u32) {
    let mut e = installed();
    e.mem.set_u32(CURRENT, 0xeeee_ee77);
    e.mem.set_u32(CURRENT + 4, 0x3333_3333);
    e.mem.set_u32(CURRENT + 8, 0x1111_1111);
    e.mem.set_u32(CURRENT + 12, 0x2222_2222);
    let words = [index, l[0], l[1], l[2], r[0], r[1], r[2], CURRENT];
    assert_eq!(do_operator(&mut e, &words), Some(1));
    assert_eq!(e.mem.u32(CURRENT + 4), 0x3333_3333, "the unused word");
    (
        e.mem.u32(CURRENT),
        e.mem.u32(CURRENT + 8),
        e.mem.u32(CURRENT + 12),
    )
}

/// An integer result: type byte 0, the low value word, the high word as it was.
fn int_result(v: i32) -> (u32, u32, u32) {
    (0xeeee_ee00, v as u32, 0x2222_2222)
}
/// A double result: type byte 1, all 8 bytes.
fn dbl_result(bits: u64) -> (u32, u32, u32) {
    (0xeeee_ee01, bits as u32, (bits >> 32) as u32)
}

/// Both integers: integer arithmetic on the low words, signed comparisons, `/` truncated toward
/// 0, `%` with the dividend's sign; an integer result writes only the low word and type 0.
#[test]
fn o8_integers_use_integer_arithmetic() {
    let cases: [(u32, i32); 11] = [
        (LE, 0),
        (LT, 0),
        (GE, 1),
        (GT, 1),
        (EQ, 0),
        (NE, 1),
        (SUB, 10),
        (ADD, 4),
        (MUL, -21),
        (DIV, -2),
        (MOD, 1),
    ];
    for (index, expected) in cases {
        assert_eq!(
            op(index, int(7), int(-3)),
            int_result(expected),
            "7 op{index} -3"
        );
    }
    // Swapped: the left is the first operand.
    assert_eq!(op(SUB, int(-3), int(7)), int_result(-10));
    assert_eq!(op(DIV, int(-3), int(7)), int_result(0));
    assert_eq!(op(MOD, int(-17), int(5)), int_result(-2));
    // Signed: -1 < 1 (unsigned would answer 0).
    assert_eq!(op(LT, int(-1), int(1)), int_result(1));
    assert_eq!(op(GT, int(-1), int(1)), int_result(0));
    assert_eq!(op(EQ, int(5), int(5)), int_result(1));
    assert_eq!(op(LE, int(5), int(5)), int_result(1));
    assert_eq!(op(LT, int(5), int(5)), int_result(0));
    // Wrapping, and the high words (junk) ignored.
    assert_eq!(
        op(ADD, int_high(i32::MAX, 0x4000_0000), int_high(1, 7)),
        int_result(i32::MIN)
    );
    assert_eq!(op(MUL, int(65_537), int(65_537)), int_result(131_073));
}

/// Integer `/` and `%` by 0 keep the left; `0x80000000 / -1` and `% -1` (a trap in JIP) wrap.
#[test]
fn o8_integer_zero_divisor_keeps_the_left() {
    assert_eq!(op(DIV, int(-9), int(0)), int_result(-9));
    assert_eq!(op(MOD, int(-9), int(0)), int_result(-9));
    assert_eq!(
        op(DIV, int_high(13, 0), int_high(0, 0x4000_0000)),
        int_result(13),
        "an integer's high word is not read"
    );
    assert_eq!(op(DIV, int(i32::MIN), int(-1)), int_result(i32::MIN));
    assert_eq!(op(MOD, int(i32::MIN), int(-1)), int_result(0));
}

/// A double on either side: both as doubles (the integer converted), SSE double arithmetic,
/// the result all 8 bytes with type 1. Values that are not powers of two show the order.
#[test]
fn o8_doubles_use_double_arithmetic_in_order() {
    // 0.3 - 0.1 and 0.1 - 0.3 in IEEE double: 0x3FC9999999999999 and its negation.
    assert_eq!(
        op(SUB, flt(0.3), flt(0.1)),
        dbl_result(0x3fc9_9999_9999_9999)
    );
    assert_eq!(
        op(SUB, flt(0.1), flt(0.3)),
        dbl_result(0xbfc9_9999_9999_9999)
    );
    // 0.1 + 0.2 = 0.30000000000000004 (0x3FD3333333333334).
    assert_eq!(
        op(ADD, flt(0.1), flt(0.2)),
        dbl_result(0x3fd3_3333_3333_3334)
    );
    // Left double, right integer; then left integer, right double.
    assert_eq!(op(SUB, flt(2.5), int(3)), dbl_result((-0.5f64).to_bits()));
    assert_eq!(op(SUB, int(3), flt(2.5)), dbl_result(0.5f64.to_bits()));
    assert_eq!(op(ADD, flt(2.5), int(3)), dbl_result(5.5f64.to_bits()));
    assert_eq!(op(MUL, int(3), flt(2.5)), dbl_result(7.5f64.to_bits()));
    // 2.5 / 7.5 = 0x3FD5555555555555 (1/3), 7.5 / 2.5 = 3.0.
    assert_eq!(
        op(DIV, flt(2.5), flt(7.5)),
        dbl_result(0x3fd5_5555_5555_5555)
    );
    assert_eq!(op(DIV, flt(7.5), flt(2.5)), dbl_result(3.0f64.to_bits()));
    // The left integer converted exactly (cvtsi2sd), not through a float.
    assert_eq!(
        op(ADD, int(16_777_217), flt(0.5)),
        dbl_result(16_777_217.5f64.to_bits())
    );
}

/// Double comparisons by `comisd`'s flags: unordered (a NaN) makes `<=`, `<` and `==` true and
/// `>=`, `>`, `!=` false; the answer is an integer.
#[test]
fn o8_double_comparisons_follow_comisd() {
    let nan = flt(f64::NAN);
    let expected_nan = [(LE, 1), (LT, 1), (GE, 0), (GT, 0), (EQ, 1), (NE, 0)];
    for (index, v) in expected_nan {
        assert_eq!(op(index, nan, int(1)), int_result(v), "NaN op{index} 1");
        assert_eq!(op(index, int(1), nan), int_result(v), "1 op{index} NaN");
    }
    let less = [(LE, 1), (LT, 1), (GE, 0), (GT, 0), (EQ, 0), (NE, 1)];
    for (index, v) in less {
        assert_eq!(
            op(index, flt(2.5), int(3)),
            int_result(v),
            "2.5 op{index} 3"
        );
    }
    let greater = [(LE, 0), (LT, 0), (GE, 1), (GT, 1), (EQ, 0), (NE, 1)];
    for (index, v) in greater {
        assert_eq!(
            op(index, int(3), flt(2.5)),
            int_result(v),
            "3 op{index} 2.5"
        );
    }
    let equal = [(LE, 1), (LT, 0), (GE, 1), (GT, 0), (EQ, 1), (NE, 0)];
    for (index, v) in equal {
        assert_eq!(
            op(index, flt(3.0), int(3)),
            int_result(v),
            "3.0 op{index} 3"
        );
    }
    // -0.0 == 0.0.
    assert_eq!(op(EQ, flt(-0.0), int(0)), int_result(1));
}

/// `&&` and `||`: a double is true when any bit of its 8 bytes is set (-0.0, a NaN), an integer
/// by its low word only (its high word not read).
#[test]
fn o8_logic_tests_whole_doubles_and_integer_low_words() {
    let minus_zero = flt(-0.0);
    assert_eq!(op(AND, minus_zero, int(5)), int_result(1));
    assert_eq!(op(AND, int(5), minus_zero), int_result(1));
    assert_eq!(op(OR, int(0), minus_zero), int_result(1));
    assert_eq!(op(OR, minus_zero, int(0)), int_result(1));
    assert_eq!(op(AND, flt(0.0), int(5)), int_result(0));
    assert_eq!(op(AND, int(5), flt(0.0)), int_result(0));
    assert_eq!(op(OR, flt(0.0), int(0)), int_result(0));
    // An integer with a junk high word: false.
    let junk = int_high(0, 0x1234_5678);
    assert_eq!(op(AND, junk, int(5)), int_result(0));
    assert_eq!(op(AND, int(5), junk), int_result(0));
    assert_eq!(op(OR, junk, junk), int_result(0));
    assert_eq!(op(OR, int(0), int(-7)), int_result(1));
    assert_eq!(op(AND, int(3), int(-7)), int_result(1));
    assert_eq!(op(AND, int(3), int(0)), int_result(0));
    // A NaN is true.
    assert_eq!(op(AND, flt(f64::NAN), int(1)), int_result(1));
}

/// Double `/`: the right operand's HIGH value word decides (0: the left is kept, even for a
/// tiny double, and for an integer whose 8 bytes hold 0 there; else `divsd`, even by 0.0).
#[test]
fn o8_double_division_tests_the_right_high_word() {
    assert_eq!(op(DIV, flt(7.5), flt(0.0)), dbl_result(7.5f64.to_bits()));
    assert_eq!(op(DIV, flt(7.5), dbl(1)), dbl_result(7.5f64.to_bits()));
    assert_eq!(op(DIV, flt(2.5), int(3)), dbl_result(2.5f64.to_bits()));
    // 2.5 / 3 = 0x3FEAAAAAAAAAAAAB.
    assert_eq!(
        op(DIV, flt(2.5), int_high(3, 0x0000_0001)),
        dbl_result(0x3fea_aaaa_aaaa_aaab)
    );
    assert_eq!(
        op(DIV, flt(2.5), int_high(0, 0x4000_0000)),
        dbl_result(f64::INFINITY.to_bits())
    );
    // A left integer kept as its double.
    assert_eq!(op(DIV, int(-9), flt(0.0)), dbl_result((-9.0f64).to_bits()));
    // -0.0's high word is 0x80000000: divided.
    assert_eq!(
        op(DIV, flt(7.5), flt(-0.0)),
        dbl_result(f64::NEG_INFINITY.to_bits())
    );
}

/// Double `%`: both truncated (`cvttsd2si`), an integer remainder; a zero (truncated) divisor
/// keeps the truncated left; NaN and out-of-range values truncate to 0x80000000.
#[test]
fn o8_double_remainder_is_an_integer() {
    assert_eq!(op(MOD, flt(7.9), int(2)), int_result(1));
    assert_eq!(op(MOD, flt(-7.9), flt(2.5)), int_result(-1));
    assert_eq!(op(MOD, int(17), flt(5.75)), int_result(2));
    assert_eq!(op(MOD, flt(7.9), flt(0.4)), int_result(7));
    // -2147483648 % 3 = -2, % 7 = -2.
    assert_eq!(op(MOD, flt(f64::NAN), int(3)), int_result(-2));
    assert_eq!(op(MOD, flt(3.0e10), int(7)), int_result(-2));
    assert_eq!(op(MOD, flt(-3.0e10), int(7)), int_result(-2));
    assert_eq!(op(MOD, flt(2_147_483_647.9), int(10)), int_result(7));
    assert_eq!(op(MOD, flt(-2_147_483_648.9), int(10)), int_result(-8));
    // A NaN divisor is 0x80000000: 9 % that = 9; 0x80000000 % -1 wraps to 0.
    assert_eq!(op(MOD, int(9), flt(f64::NAN)), int_result(9));
    assert_eq!(op(MOD, flt(f64::NAN), int(-1)), int_result(0));
}

// ---- G70: TerminalGetLockedHook ----

/// A terminal with flags byte `flags` (its neighbours set), a reference whose extra list answers
/// `extra` for type 0x50 (the stand-in checks its words).
fn terminal(flags: u8, extra: Option<(u8, u8)>) -> Engine {
    let mut e = installed();
    e.mem.set_u32(TERMINAL + 0xa4, 0x7777_7777);
    e.mem.set_u8(TERMINAL + 0xa5, flags);
    let answer = match extra {
        Some((locked_out, level)) => {
            e.mem.set_u32(EXTRA + 0xc, 0x6666_6666);
            e.mem.set_u8(EXTRA + 0xc, locked_out);
            e.mem.set_u8(EXTRA + 0xd, level);
            EXTRA
        }
        None => 0,
    };
    e.register_double(GET_BY_TYPE, move |_, w| {
        assert_eq!(w[..2], [REF + 0x44, 0x50]);
        Ret {
            eax: answer,
            ..Ret::default()
        }
    });
    e
}

fn unlocked(e: &mut Engine, reference: u32) -> Option<u32> {
    terminal_unlocked(e, &[TERMINAL, reference])
}

/// The terminal's flag (bit 2 of +0xA5) with no reference or no extra.
#[test]
fn g70_no_reference_or_no_extra_answers_the_flag() {
    for (flags, expected) in [(0x02, 1), (0xfd, 0), (0xff, 1), (0x00, 0)] {
        let mut e = terminal(flags, None);
        assert_eq!(unlocked(&mut e, 0), Some(expected), "{flags:x}");
        assert_eq!(unlocked(&mut e, REF), Some(expected), "{flags:x}");
    }
}

/// With an `ExtraTerminalState`: `lockedOut` 0x80 or level 0xFE unlocked; level 0xFF the flag;
/// any other level locked, whatever the flag.
#[test]
fn g70_the_terminal_state_decides() {
    for flags in [0x02u8, 0xfd] {
        let flag = u32::from(flags & 2 != 0);
        let cases = [
            ((0x80, 0x03), 1),
            ((0x80, 0xff), 1),
            ((0x7f, 0xfe), 1),
            ((0x00, 0xfe), 1),
            ((0x00, 0xff), flag),
            ((0x81, 0xff), flag),
            ((0x00, 0x03), 0),
            ((0x01, 0x00), 0),
            ((0x00, 0xfd), 0),
        ];
        for (extra, expected) in cases {
            let mut e = terminal(flags, Some(extra));
            assert_eq!(
                unlocked(&mut e, REF),
                Some(expected),
                "{flags:x} {extra:x?}"
            );
        }
    }
}

// ---- G34, G71, G03 ----

/// G34: the render state's root node (+0x14, its neighbours set); no render state: the exe's
/// call (`None`).
#[test]
fn g34_answers_the_root_node() {
    let mut e = installed();
    e.mem.set_u32(REF + 0x60, 0x1111_1111);
    e.mem.set_u32(REF + 0x68, 0x2222_2222);
    e.mem.set_u32(RENDER_STATE + 0x10, 0x3333_3333);
    e.mem.set_u32(RENDER_STATE + 0x18, 0x4444_4444);
    e.mem.set_u32(REF + 0x64, RENDER_STATE);
    e.mem.set_u32(RENDER_STATE + 0x14, 0x0700_0020);
    assert_eq!(sound_node(&mut e, &[REF]), Some(0x0700_0020));
    e.mem.set_u32(RENDER_STATE + 0x14, 0);
    assert_eq!(
        sound_node(&mut e, &[REF]),
        Some(0),
        "a null node is JIP's answer"
    );
    e.mem.set_u32(REF + 0x64, 0);
    assert_eq!(sound_node(&mut e, &[REF]), None);
}

/// G71: only a null process is answered (no furniture); G03 always takes the call.
#[test]
fn g71_and_g03_answers() {
    let mut e = installed();
    assert_eq!(furniture_process(&mut e, &[0]), Some(0));
    assert_eq!(furniture_process(&mut e, &[1]), None);
    assert_eq!(furniture_process(&mut e, &[0x0600_0000]), None);
    assert_eq!(earn_achievement(&mut e, &[]), Some(1));
}

// ---- G123: GetHitIMODHook ----

/// JIP installed (the commands' test engine), the image space byte `on` (its neighbours set),
/// the form 0x162 in the form map (another form, 0x163, beside it).
fn get_hit_engine(on: u8) -> Engine {
    use crate::commands::test_support::{engine, game_globals, in_form_map};
    let mut e = engine();
    e.map(0x0118_a000, 0x1000);
    e.mem.set_u32(0x0118_abb0, 0xffff_ffff);
    e.mem.set_u8(IMAGE_SPACE_ENABLED, on);
    game_globals(&mut e);
    in_form_map(&mut e, 0x0600_0162, 0x162, 0);
    in_form_map(&mut e, 0x0600_0163, 0x163, 1);
    e
}

fn get_hit(e: &mut Engine) -> u32 {
    e.call(GET_HIT_IMOD.addr(), &[]).eax
}

/// Through the wrap at 0x5D2860: the form 0x162 while the byte is set and the hit effects word is
/// 0; 0 with the byte 0 or in immortal mode 2 (bit 2); the exe's cache word (0x11C9700) is not
/// written.
#[test]
fn g123_answers_by_the_byte_and_the_hit_effects_word() {
    use crate::commands::test_support::{call_on, n};
    let mut e = get_hit_engine(1);
    e.map(0x011c_9000, 0x1000);
    assert_eq!(get_hit(&mut e), 0x0600_0162);
    assert_eq!(e.mem.u32(0x011c_9700), 0, "no cache");
    let mut off = get_hit_engine(0);
    assert_eq!(get_hit(&mut off), 0);
    call_on(&mut e, super::super::gp::TOGGLE_IMMORTAL_MODE, 0, &[n(2)]);
    assert_eq!(get_hit(&mut e), 0, "immortal mode 2");
    call_on(&mut e, super::super::gp::TOGGLE_IMMORTAL_MODE, 0, &[n(1)]);
    assert_eq!(get_hit(&mut e), 0x0600_0162, "mode 1 keeps the hit effects");
}

/// The hit effects word is run-wide (`gp`'s lock-free `ext_free` word; G123 keeps no state of its own):
/// set through the command on one engine, read by another engine sharing its slots (a real
/// thread's). An `Engine` cannot move to another OS thread in a test, so both run on this one.
#[test]
fn g123_sees_the_hit_effects_word_from_another_engine() {
    use crate::commands::test_support::{call_on, n};
    let mut e = get_hit_engine(1);
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    other.map(0x0118_a000, 0x1000);
    other.mem.set_u8(IMAGE_SPACE_ENABLED, 1);
    call_on(&mut e, super::super::gp::TOGGLE_IMMORTAL_MODE, 0, &[n(2)]);
    assert_eq!(get_hit_imod(&mut other, &[]).eax, 0);
    engine_core::extend::release_held_for_test();
}

/// The word: G123 needs JIP installed only (no fix bit).
#[test]
fn g123_is_in_with_the_row_off() {
    let mut e = get_hit_engine(1);
    assert_eq!(e.jip_patches() & jip_patches::FIXES, 0);
    assert_ne!(e.jip_patches() & jip_patches::INSTALLED, 0);
    assert_eq!(get_hit(&mut e), 0x0600_0162);
}
