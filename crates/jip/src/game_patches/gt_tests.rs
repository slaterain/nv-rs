//! Group GT's game patch: G43, JIP's fixed-string pool. The expected values come from JIP's
//! assembly (`GetNiFixedString`, `FreeUnusedNiFixedStrings`, `InsertKey` with
//! `HashedKey<const SInt8*>`) and the exe's three functions (`00a5b630`, `00a5b690`, `00a5b460`).

use super::*;
use crate::test_support::engine;

const TEXTS: u32 = 0x0500_0000;
/// The game's table word (`00a5b630` stores the table there).
const STRING_TABLE: u32 = 0x011f_42c0;

/// A text in guest memory (`n`: its slot), with its terminator.
fn text(e: &mut Engine, n: u32, s: &[u8]) -> u32 {
    let at = TEXTS + n * 0x100;
    e.mem.write(at, s);
    e.mem.set_u8(at + s.len() as u32, 0);
    at
}

fn with(row: bool, installed: bool) -> Engine {
    let mut e = engine();
    e.map(TEXTS, 0x10_0000);
    e.map(STRING_TABLE & !0xfff, 0x1000);
    // No settings file (JIP's defaults), and the topic lookup another group wraps.
    e.register(0x00fd_f088, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    if !e.is_registered(0x0061_a310) {
        e.register(0x0061_a310, |_, _| Ret::default());
    }
    crate::install_as(&mut e, installed);
    super::super::set_row(&mut e, row);
    e
}

fn make(e: &mut Engine, at: u32) -> u32 {
    e.call(MAKE_STRING.addr(), &[at]).eax
}

/// The rows at the three addresses JIP patches (`SafeWrite8(0xA5B630, 0xC3)`,
/// `WriteRelJump(0xA5B690, ...)`, `WriteRelCall(0x878203, ...)`, the only call of `00a5b460`).
#[test]
fn rows_are_jips_addresses() {
    assert_eq!(MAKE_TABLE.addr(), 0x00a5_b630);
    assert_eq!(MAKE_STRING.addr(), 0x00a5_b690);
    assert_eq!(FREE_UNUSED.addr(), 0x00a5_b460);
    assert_eq!(AREA.fix_wraps, [0x00a5_b630, 0x00a5_b690, 0x00a5_b460]);
    assert!(AREA.fix_sites.is_empty() && AREA.jip_sites.is_empty() && AREA.jip_wraps.is_empty());
}

/// Not installed, or installed with the row off (vanilla): no wrap; with the row: JIP's three.
#[test]
fn the_wraps_come_with_the_row() {
    for (row, installed) in [(true, false), (false, true), (false, false)] {
        let e = with(row, installed);
        for &addr in AREA.fix_wraps {
            assert!(e.wraps_at(addr).is_empty(), "{row} {installed} {addr:08x}");
        }
    }
    let e = with(true, true);
    for &addr in AREA.fix_wraps {
        assert_eq!(e.wraps_at(addr), [Owner::Jip], "{addr:08x}");
    }
}

/// `00a5b630`: the game's table is not made (its word stays 0).
#[test]
fn the_games_table_is_not_made() {
    let mut e = with(true, true);
    e.mem.set_u32(STRING_TABLE, 0);
    assert_eq!(e.call(MAKE_TABLE.addr(), &[]).eax, 0);
    assert_eq!(e.mem.u32(STRING_TABLE), 0);
}

/// JIP's block size: `(length + 0x19) & ~0xF`.
#[test]
fn the_block_is_jips_size() {
    assert_eq!(block_size(0), 0x10);
    assert_eq!(block_size(6), 0x10);
    assert_eq!(block_size(7), 0x20);
    assert_eq!(block_size(0x17), 0x30);
}

/// No text: 0. A new text: count 1, its length, the characters and the terminator; the same
/// text again (anywhere): the same string, count 2; another text: another string.
#[test]
fn a_text_gets_one_string_with_a_count() {
    let mut e = with(true, true);
    assert_eq!(make(&mut e, 0), 0);
    let a = text(&mut e, 0, b"Bip01 Head");
    let s = make(&mut e, a);
    assert_ne!(s, 0);
    assert_ne!(s, a);
    assert_eq!(e.mem.u32(s - 8), 1);
    assert_eq!(e.mem.u32(s - 4), 10);
    assert_eq!(e.mem.cstr(s), b"Bip01 Head");
    let again = text(&mut e, 1, b"Bip01 Head");
    assert_eq!(make(&mut e, again), s);
    assert_eq!(e.mem.u32(s - 8), 2);
    let other = text(&mut e, 2, b"Bip01 head");
    let t = make(&mut e, other);
    assert_ne!(t, s, "case counts");
    assert_eq!(e.mem.u32(t - 8), 1);
    // The empty text is a string too (only a null text answers 0).
    let empty = text(&mut e, 3, b"");
    let u = make(&mut e, empty);
    assert_ne!(u, 0);
    assert_eq!((e.mem.u32(u - 4), e.mem.u8(u)), (0, 0));
}

/// JIP compares hashes only: a second text with the first one's hash gets the first one's string.
#[test]
fn two_texts_of_one_hash_share_a_string() {
    let mut seen = std::collections::HashMap::new();
    let (first, second) = (0u32..)
        .find_map(|i| {
            let t = format!("n{i}").into_bytes();
            let h = crate::containers::str_hash_cs(&t);
            seen.insert(h, t.clone()).map(|old| (old, t))
        })
        .unwrap();
    assert_ne!(first, second);
    let mut e = with(true, true);
    let a = text(&mut e, 0, &first);
    let b = text(&mut e, 1, &second);
    let s = make(&mut e, a);
    assert_eq!(make(&mut e, b), s);
    assert_eq!(e.mem.cstr(s), first);
    assert_eq!(e.mem.u32(s - 8), 2);
}

/// The clean-up: a string whose count is 0 leaves the pool (the same text then gets a new one,
/// count 1); a counted one stays (a count JIP's code would see as not 0: 0xFFFFFFFF included).
#[test]
fn the_clean_up_frees_only_unused_strings() {
    let mut e = with(true, true);
    let a = text(&mut e, 0, b"unused");
    let b = text(&mut e, 1, b"held");
    let c = text(&mut e, 2, b"below zero");
    let (sa, sb, sc) = (make(&mut e, a), make(&mut e, b), make(&mut e, c));
    e.mem.set_u32(sa - 8, 0);
    e.mem.set_u32(sc - 8, 0xffff_ffff);
    e.call(FREE_UNUSED.addr(), &[]);
    assert_eq!(make(&mut e, b), sb);
    assert_eq!(e.mem.u32(sb - 8), 2);
    assert_eq!(make(&mut e, c), sc);
    let again = make(&mut e, a);
    assert_eq!(e.mem.u32(again - 8), 1);
    assert_eq!(e.mem.cstr(again), b"unused");
    // A clean-up with nothing unused changes nothing.
    e.call(FREE_UNUSED.addr(), &[]);
    assert_eq!(make(&mut e, a), again);
}
