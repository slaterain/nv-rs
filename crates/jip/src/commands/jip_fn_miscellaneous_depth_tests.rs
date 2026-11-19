//! `ToggleDepthClear`, called by its opcode, and the seam it answers. Expectations from JIP's
//! `Cmd_ToggleDepthClear_Execute` (the number stored in `s_clearDepthBuffer`) and
//! `RendererClearBufferHook` (the clear skipped while it is 0).

use super::*;
use crate::commands::test_support::*;

fn answer(e: &mut Engine) -> Option<u32> {
    e.site(DEPTH_CLEAR.addr(), &[])
}

/// At start the game clears; 0 skips the clear, any other number clears again; no result; a call
/// whose argument does not read changes nothing.
#[test]
fn the_game_clears_until_a_script_says_0() {
    let mut e = engine();
    assert_eq!(depth_clear(&mut e), 1);
    assert_eq!(answer(&mut e), None);
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(call(&mut e, 0x293d, &[n(0)]), (1, 0.0));
    assert_eq!(depth_clear(&mut e), 0);
    assert_eq!(answer(&mut e), Some(0));
    assert_eq!(call(&mut e, 0x293d, &[]).0, 1);
    assert_eq!(answer(&mut e), Some(0), "no argument: unchanged");
    call(&mut e, 0x293d, &[n(-3)]);
    assert_eq!(depth_clear(&mut e), (-3i32) as u32);
    assert_eq!(answer(&mut e), None);
    call(&mut e, 0x293d, &[n(1)]);
    assert_eq!(answer(&mut e), None);
}
