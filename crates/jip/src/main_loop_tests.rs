//! Tests of JIP's main-loop callbacks and `SetGameMainLoopCallback` through its record: the
//! query, add, keep and remove; the frames counted in the modes asked; the reference check; the
//! marked ones gone at the cycle's end; `CleanMLCallbacks`. The calls are recorded
//! ([`crate::script_call`]). Layouts: a form's type `+4` and id `+0xC` (`TESForm`); the
//! interface manager's `cMenuMode` `+0xC` (engine-fallout-interface `interfacemanager.rs`); the
//! game's form map (`NiTPointerMap` at `*0x011C54C0`: hash size `+4`, buckets `+8`, an item next,
//! key, value: engine-core `types.rs`, as `cosave` reads it).

use super::*;
use crate::script_call::calls;
use crate::test_support::{engine, form_arg, int_arg, line, script_ref, RESULT};

const AREA: u32 = 0x0500_0000;
const SCRIPT_FORM: u32 = AREA;
const PLAYER_REF: u32 = AREA + 0x100;
const OTHER_REF: u32 = AREA + 0x200;
const MANAGER: u32 = AREA + 0x300;
const FORM_MAP: u32 = AREA + 0x1000;

fn world() -> Engine {
    calls();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    e.map(0x011d_8000, 0x1000);
    e.map(0x011c_5000, 0x1000);
    e.mem.set_u8(SCRIPT_FORM + 4, SCRIPT);
    e.mem.set_u32(SCRIPT_FORM + 0xc, 0x0100_0900);
    e.mem.set_u32(PLAYER_REF + 0xc, PLAYER_ID);
    e.mem.set_u32(OTHER_REF + 0xc, 0x0100_0001);
    e.mem.set_u32(PLAYER, PLAYER_REF);
    e.mem.set_u32(INTERFACE_MANAGER, MANAGER);
    e.mem.set_u32(MANAGER + CURRENT_MODE, 1);
    // The form map: one bucket, OTHER_REF under its id.
    e.mem.set_u32(0x011c_54c0, FORM_MAP);
    e.mem.set_u32(FORM_MAP + 4, 1);
    e.mem.set_u32(FORM_MAP + 8, FORM_MAP + 0x20);
    e.mem.set_u32(FORM_MAP + 0x20, FORM_MAP + 0x40);
    e.mem.set_u32(FORM_MAP + 0x40, 0);
    e.mem.set_u32(FORM_MAP + 0x44, 0x0100_0001);
    e.mem.set_u32(FORM_MAP + 0x48, OTHER_REF);
    crate::install_as(&mut e, true);
    script_ref(&mut e, 1, SCRIPT_FORM);
    e
}

/// `SetGameMainLoopCallback` on `this` with the script and `ints`: the result.
fn sgmlc(e: &mut Engine, this: u32, ints: &[i32]) -> f64 {
    let mut args = vec![form_arg(1)];
    args.extend(ints.iter().map(|v| int_arg(*v)));
    let mut words = line(e, SET_GAME_MAIN_LOOP_CALLBACK, &args);
    words[2] = this;
    e.mem.set_f64(RESULT, 0.0);
    let at = crate::test_support::execute_of(e, SET_GAME_MAIN_LOOP_CALLBACK);
    assert_eq!(e.call(at, &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}

fn callbacks(e: &mut Engine) -> Vec<Callback> {
    state(e).callbacks.clone()
}

/// Built once JIP answers "installed"; the query, the add on the player (no reference), the frames
/// counted in game mode only for flags 1, the call every second frame with no arguments, the
/// keep (delay 0 is 1, frames left as they were, new flags), the removal at the next cycle.
#[test]
fn a_callback_through_the_command() {
    let mut e = world();
    assert_eq!(sgmlc(&mut e, 0, &[]), 0.0, "none yet");
    sgmlc(&mut e, 0, &[1, 2, 1]);
    assert_eq!(
        callbacks(&mut e),
        [Callback {
            script: SCRIPT_FORM,
            this_obj: PLAYER_REF,
            remove: false,
            flags: 1,
            call_delay: 2,
            cycle_count: 2,
            ref_id: PLAYER_ID,
        }]
    );
    assert_eq!(sgmlc(&mut e, 0, &[]), 1.0, "one argument: the query");
    cycle(&mut e);
    assert!(calls().is_empty(), "the first frame");
    cycle(&mut e);
    assert_eq!(calls(), [(SCRIPT_FORM, PLAYER_REF, vec![])]);
    assert_eq!(callbacks(&mut e)[0].cycle_count, 2, "set back to the delay");
    // Menu mode: not counted for flags 1.
    e.mem.set_u32(MANAGER + CURRENT_MODE, 2);
    cycle(&mut e);
    cycle(&mut e);
    assert!(calls().is_empty());
    assert_eq!(callbacks(&mut e)[0].cycle_count, 2);
    // Kept: delay 0 is 1, mode 0xF gives flags 0xB, the frames left as they were.
    sgmlc(&mut e, 0, &[1, 0, 0xf]);
    let c = &callbacks(&mut e)[0];
    assert_eq!((c.call_delay, c.flags, c.cycle_count), (1, 0xb, 2));
    cycle(&mut e);
    cycle(&mut e);
    assert_eq!(
        calls(),
        [(SCRIPT_FORM, PLAYER_REF, vec![])],
        "menu mode now counts"
    );
    // Removed: marked, gone at the next cycle's end, not called.
    sgmlc(&mut e, 0, &[0]);
    assert!(callbacks(&mut e)[0].remove);
    cycle(&mut e);
    assert!(calls().is_empty());
    assert!(callbacks(&mut e).is_empty());
}

/// A callback on another reference: called on it while its id still names it; marked and gone
/// once it does not. A form that is not a script: nothing.
#[test]
fn the_reference_check() {
    let mut e = world();
    sgmlc(&mut e, OTHER_REF, &[1, 1]);
    cycle(&mut e);
    assert_eq!(calls(), [(SCRIPT_FORM, OTHER_REF, vec![])]);
    e.mem.set_u32(FORM_MAP + 0x48, AREA + 0x400);
    cycle(&mut e);
    assert!(calls().is_empty(), "its id names another form now");
    assert!(callbacks(&mut e).is_empty(), "gone");
    e.mem.set_u8(SCRIPT_FORM + 4, 0x12);
    sgmlc(&mut e, 0, &[1, 1]);
    assert!(callbacks(&mut e).is_empty(), "not a script");
}

/// `CleanMLCallbacks`: a callback with mode bit 8 goes at a load, the others stay.
#[test]
fn a_load_takes_mode_8() {
    let mut e = world();
    sgmlc(&mut e, 0, &[1, 1, 9]);
    sgmlc(&mut e, OTHER_REF, &[1, 1, 1]);
    clean(&mut e);
    let c = callbacks(&mut e);
    assert_eq!((c[0].remove, c[1].remove), (true, false));
    cycle(&mut e);
    assert_eq!(callbacks(&mut e).len(), 1);
    assert_eq!(callbacks(&mut e)[0].this_obj, OTHER_REF);
}

/// While JIP answers "not installed" the command's whole record (0x28 bytes and its parameter
/// list) is what #502's table leaves: no handler, no name, as without the DLL.
#[test]
fn not_installed_the_record_is_502s() {
    let snapshot = |e: &Engine| {
        let at = e.script_tables().record(SET_GAME_MAIN_LOOP_CALLBACK);
        let mut bytes = e.mem.bytes(at, 0x28);
        let (count, params) = (e.mem.u16(at + 0x12), e.mem.u32(at + 0x14));
        bytes.extend(e.mem.bytes(params, 12 * u32::from(count)));
        bytes
    };
    let mut table_only = engine();
    crate::table::install(&mut table_only, false);
    let mut off = engine();
    crate::install_as(&mut off, false);
    assert_eq!(snapshot(&off), snapshot(&table_only));
    assert_eq!(
        off.mem
            .u32(off.script_tables().record(SET_GAME_MAIN_LOOP_CALLBACK) + 0x18),
        0
    );
}
