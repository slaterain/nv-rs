//! Tests of `jip_fn_string`: `GetStringHash`.

use crate::commands::test_support::*;
use engine_core::prelude::*;

fn res_of(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call(e, opcode, args).1
}

fn files_setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    game_globals(&mut e);
    files::fresh();
    e
}

/// `GetStringHash`: JIP's hash without case, or with case when asked; with `split`, the parts and
/// their hashes in `GetStringHash.txt` and no result.
#[test]
fn string_hashes() {
    let mut e = files_setup();
    let ci = f64::from(crate::containers::str_hash_ci(b"Hello") as i32);
    let cs = f64::from(crate::containers::str_hash_cs(b"Hello") as i32);
    assert_ne!(ci, cs);
    assert_eq!(res_of(&mut e, 0x293f, &[s(b"Hello")]), ci);
    assert_eq!(res_of(&mut e, 0x293f, &[s(b"hello")]), ci);
    assert_eq!(res_of(&mut e, 0x293f, &[s(b"Hello"), n(0), n(1)]), cs);
    assert_eq!(res_of(&mut e, 0x293f, &[s(b"a||B|"), n(1)]), 0.0);
    let want = format!(
        "\"a\"\n{}\n\n\"B\"\n{}\n\n",
        crate::containers::str_hash_ci(b"a") as i32,
        crate::containers::str_hash_ci(b"B") as i32
    );
    assert_eq!(files::get("GetStringHash.txt").unwrap(), want.as_bytes());
    // StrHashCS: the same steps without folding.
    assert_eq!(
        crate::containers::str_hash_cs(b"abcde"),
        crate::containers::str_hash_ci(b"ABCDE")
    );
}

/// `GetStringHash` typed in the console prints its result there (`GetStringHash >> n`); with
/// `split` nothing is printed (no result).
#[test]
fn string_hash_console_echo() {
    let mut e = files_setup();
    let ci = crate::containers::str_hash_ci(b"Hello") as i32;
    let (res, lines) = echoed(&mut e, 0x293f, 0, &[s(b"Hello")]);
    assert_eq!(res, f64::from(ci));
    let mut want = b"GetStringHash >> ".to_vec();
    want.extend(crate::utility::flt_to_str(f64::from(ci)));
    assert_eq!(lines, vec![want]);
    let (_, lines) = echoed(&mut e, 0x293f, 0, &[s(b"a|b"), n(1)]);
    assert!(lines.is_empty());
}
