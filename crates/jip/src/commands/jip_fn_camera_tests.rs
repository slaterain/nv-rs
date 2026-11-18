//! `ToggleVanityWheel` and `ToggleCameraCollision`, called by their opcodes, and the two seams
//! they answer. Expectations from JIP's bodies (`Cmd_ToggleVanityWheel_Execute`,
//! `Cmd_ToggleCameraCollision_Execute`): the byte written is the `JNS`/`JMP` (seam answer 1) when
//! the toggle is 0, the original byte (answer 0) for any other value.

use super::*;
use crate::commands::test_support::*;

/// The command's result (cleared first, as the result word JIP leaves alone when off).
fn toggle_cmd(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    let (ok, result) = call(e, opcode, args);
    assert_eq!(ok, 1);
    result
}

fn answer(e: &mut Engine, seam: SiteSlot) -> Option<u32> {
    e.site(seam.addr(), &[])
}

#[test]
fn both_start_on_and_the_seams_leave_the_game_alone() {
    let mut e = engine();
    assert_eq!(toggle_cmd(&mut e, 0x28b4, &[]), 1.0);
    assert_eq!(toggle_cmd(&mut e, 0x28d6, &[]), 1.0);
    assert_eq!(answer(&mut e, VANITY_WHEEL), Some(0));
    assert_eq!(answer(&mut e, CAMERA_COLLISION), Some(0));
}

/// 0 turns it off (the answer before the change); 0 again changes nothing; any other value turns
/// it on. With no argument only the answer. The two switches are separate.
#[test]
fn zero_turns_off_and_any_other_value_turns_on() {
    for (opcode, seam, other) in [
        (0x28b4, VANITY_WHEEL, CAMERA_COLLISION),
        (0x28d6, CAMERA_COLLISION, VANITY_WHEEL),
    ] {
        let mut e = engine();
        assert_eq!(
            toggle_cmd(&mut e, opcode, &[n(0)]),
            1.0,
            "{opcode:x}: was on"
        );
        assert_eq!(answer(&mut e, seam), Some(1), "{opcode:x}: off");
        assert_eq!(answer(&mut e, other), Some(0), "{opcode:x}: the other left");
        assert_eq!(toggle_cmd(&mut e, opcode, &[]), 0.0, "{opcode:x}");
        assert_eq!(toggle_cmd(&mut e, opcode, &[n(0)]), 0.0, "{opcode:x}");
        assert_eq!(answer(&mut e, seam), Some(1), "{opcode:x}: still off");
        assert_eq!(
            toggle_cmd(&mut e, opcode, &[n(5)]),
            0.0,
            "{opcode:x}: was off"
        );
        assert_eq!(answer(&mut e, seam), Some(0), "{opcode:x}: on again");
        assert_eq!(toggle_cmd(&mut e, opcode, &[n(1)]), 1.0, "{opcode:x}");
        assert_eq!(answer(&mut e, seam), Some(0), "{opcode:x}: stays on");
    }
}
