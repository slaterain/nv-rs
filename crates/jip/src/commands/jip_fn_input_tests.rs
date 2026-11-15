//! Tests of `jip_fn_input` on the game's control bindings (`*0x011F35CC`: keyboard +0x1B94, mouse
//! +0x1BB0, controller +0x1BE8, one byte a control, 0xFF none), xNVSE's key states (the held
//! flag) and JIP's pad mods, and the mouse wheel word (+0x1B2C).

use super::*;
use crate::commands::test_support::*;

const CONTROLS: u32 = FORMS + 0x1_0000;

fn setup() -> Engine {
    let mut e = engine();
    e.map(0x011f_3000, 0x1000);
    e.mem.set_u32(0x011f_35cc, CONTROLS);
    for array in [0x1b94, 0x1bb0, 0x1bcc, 0x1be8] {
        for control in 0..28 {
            e.mem.set_u8(CONTROLS + array + control, 0xff);
        }
    }
    // Control 4: key 0x11, mouse button 1, pad A (10). Control 5: no key, mouse 2, pad LT (16).
    // Control 27: key 0x20, pad RT (17). Control 6: pad left stick up (19).
    e.mem.set_u8(CONTROLS + 0x1b94 + 4, 0x11);
    e.mem.set_u8(CONTROLS + 0x1bb0 + 4, 1);
    e.mem.set_u8(CONTROLS + 0x1be8 + 4, 10);
    e.mem.set_u8(CONTROLS + 0x1bb0 + 5, 2);
    e.mem.set_u8(CONTROLS + 0x1be8 + 5, 16);
    e.mem.set_u8(CONTROLS + 0x1b94 + 27, 0x20);
    e.mem.set_u8(CONTROLS + 0x1be8 + 27, 17);
    e.mem.set_u8(CONTROLS + 0x1be8 + 6, 19);
    // Control 0's mouse button 3: byte 28 of the keyboard array, which a control of 28 would read.
    e.mem.set_u8(CONTROLS + 0x1bb0, 3);
    e
}

fn held(e: &mut Engine) -> Vec<usize> {
    e.ext::<nvse::Nvse>()
        .input
        .keys
        .iter()
        .enumerate()
        .filter(|(_, k)| k.hold)
        .map(|(i, _)| i)
        .collect()
}

/// No ready controller: the key and the mouse button (+0x100) held and let go; a control past
/// 27 nothing; the pad mods untouched.
#[test]
fn controls_held_by_keys() {
    let mut e = setup();
    call(&mut e, 0x2805, &[n(4)]);
    assert_eq!(held(&mut e), vec![0x11, 0x101]);
    call(&mut e, 0x2805, &[n(5)]);
    assert_eq!(
        held(&mut e),
        vec![0x11, 0x101, 0x102],
        "no key: the button alone"
    );
    call(&mut e, 0x2805, &[n(27)]);
    assert_eq!(held(&mut e), vec![0x11, 0x20, 0x101, 0x102]);
    call(&mut e, 0x2806, &[n(4)]);
    assert_eq!(held(&mut e), vec![0x20, 0x102]);
    call(&mut e, 0x2805, &[n(28)]);
    call(&mut e, 0x2805, &[n(-1)]);
    assert_eq!(held(&mut e), vec![0x20, 0x102], "28 and up: nothing");
    let mods = crate::input::state(&mut e).mods;
    assert_eq!((mods.button_hold, mods.trigger_mods), (0, 0));
}

/// A ready controller: the binding's button or trigger held; a stick direction nothing; the
/// keys untouched.
#[test]
fn controls_held_on_the_pad() {
    let mut e = setup();
    crate::input::state(&mut e).ready = true;
    crate::input::state(&mut e).mods.trigger_mods = 0x03;
    call(&mut e, 0x2805, &[n(4)]);
    call(&mut e, 0x2805, &[n(5)]);
    call(&mut e, 0x2805, &[n(27)]);
    call(&mut e, 0x2805, &[n(6)]);
    let mods = crate::input::state(&mut e).mods;
    assert_eq!(mods.button_hold, 0x1000, "A");
    assert_eq!(
        mods.trigger_mods, 0x0f,
        "both triggers held, the disabled bits kept"
    );
    assert!(held(&mut e).is_empty());
    call(&mut e, 0x2806, &[n(5)]);
    call(&mut e, 0x2806, &[n(4)]);
    let mods = crate::input::state(&mut e).mods;
    assert_eq!((mods.button_hold, mods.trigger_mods), (0, 0x0b));
    call(&mut e, 0x2806, &[n(6)]);
    assert_eq!(crate::input::state(&mut e).mods.trigger_mods, 0x0b);
}

/// The value added to the wheel word, wrapping; with no argument nothing (see the doc).
#[test]
fn mouse_wheel() {
    let mut e = setup();
    e.mem.set_u32(CONTROLS + 0x1b2c, 5);
    call(&mut e, 0x292c, &[n(-7)]);
    assert_eq!(e.mem.u32(CONTROLS + 0x1b2c) as i32, -2);
    call(&mut e, 0x292c, &[n(i32::MAX)]);
    assert_eq!(e.mem.u32(CONTROLS + 0x1b2c) as i32, i32::MAX - 2);
    call(&mut e, 0x292c, &[n(3)]);
    assert_eq!(e.mem.u32(CONTROLS + 0x1b2c) as i32, i32::MIN, "wraps");
    call(&mut e, 0x292c, &[]);
    assert_eq!(e.mem.u32(CONTROLS + 0x1b2c) as i32, i32::MIN);
    assert_eq!(e.mem.u32(CONTROLS + 0x1b28), 0, "the word before untouched");
}
