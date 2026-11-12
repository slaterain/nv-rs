//! Tests of JIP's LN events: the masks by name, the handlers set and removed as JIP's
//! `ProcessLNEventHandler` and `SetInputEventHandler` decide, and one frame's checks. The calls
//! are recorded ([`crate::script_call`]). Layouts from the exe: a reference's parent cell `+0x40`
//! (engine-fallout-cell `tesobjectrefr.rs`), the player's grabbed reference `+0x638`
//! (engine-fallout-ai `playercharacter.rs`), the interface manager's `cMenuMode` `+0xC` and its
//! crosshair reference `pActivateRef` `+0xFC` (engine-fallout-interface `interfacemanager.rs`),
//! a form list's forms inline at `+0x18` (`bgslistform.rs`), the bindings at `+0x1B94` of the
//! `Controls` object (`xcontrols.rs`), the form map at `*0x011C54C0` (engine-core `types.rs`).

use super::*;
use crate::script_call::calls;
use crate::test_support::engine;

const AREA: u32 = 0x0500_0000;
const S1: u32 = AREA + 0x10;
const S2: u32 = AREA + 0x20;
const PLAYER_REF: u32 = AREA + 0x100;
const CELL_A: u32 = AREA + 0x200;
const CELL_B: u32 = AREA + 0x300;
const REF_A: u32 = AREA + 0x400;
const BASE_A: u32 = AREA + 0x500;
const LIST: u32 = AREA + 0x600;
const MANAGER: u32 = AREA + 0x700;
const VTABLES: u32 = AREA + 0x800;
const CONTROLS_AT: u32 = AREA + 0x2000;
const FORM_MAP: u32 = AREA + 0x1000;

fn world(installed: bool) -> Engine {
    calls();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    for page in [0x011d_8000, 0x011c_5000, 0x011f_3000] {
        e.map(page, 0x1000);
    }
    let reference_vt = VTABLES;
    e.mem.set_u32(reference_vt + VT_IS_REFERENCE, RETURN_TRUE);
    for (form, kind, id) in [
        (CELL_A, CELL, 0x0100_0010),
        (CELL_B, CELL, 0x0100_0011),
        (BASE_A, 0x28, 0x0100_0012),
        (LIST, LIST_FORM, 0x0100_0013),
        (S1, 0x11, 0x0100_0014),
        (S2, 0x11, 0x0100_0015),
    ] {
        e.mem.set_u32(form, VTABLES + 0x100);
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, id);
    }
    e.mem.set_u32(REF_A, reference_vt);
    e.mem.set_u8(REF_A + 4, 0x3a);
    e.mem.set_u32(REF_A + 0xc, 0x0100_0020);
    e.mem.set_u32(REF_A + 0x20, BASE_A);
    e.mem.set_u32(PLAYER_REF + 0xc, 0x14);
    e.mem.set_u32(PLAYER, PLAYER_REF);
    e.mem.set_u32(INTERFACE_MANAGER, MANAGER);
    e.mem.set_u32(MANAGER + 0xc, 1);
    // The form map: REF_A under its id.
    e.mem.set_u32(0x011c_54c0, FORM_MAP);
    e.mem.set_u32(FORM_MAP + 4, 1);
    e.mem.set_u32(FORM_MAP + 8, FORM_MAP + 0x20);
    e.mem.set_u32(FORM_MAP + 0x20, FORM_MAP + 0x40);
    e.mem.set_u32(FORM_MAP + 0x44, 0x0100_0020);
    e.mem.set_u32(FORM_MAP + 0x48, REF_A);
    // Control 4: keyboard key 0x11; every other binding none.
    e.mem.set_u32(0x011f_35cc, CONTROLS_AT);
    for i in 0..0x70 {
        e.mem.set_u8(CONTROLS_AT + 0x1b94 + i, 0xff);
    }
    e.mem.set_u8(CONTROLS_AT + 0x1b94 + 4, 0x11);
    crate::install_as(&mut e, installed);
    e
}

fn handler(e: &mut Engine, event: u8, script: u32, add: bool, form: u32, number: u32) -> bool {
    process_ln_event_handler(e, mask(event), script, add, form, number)
}

/// The masks by name without case; JIP gives xNVSE its two entry points only when it answers
/// "installed".
#[test]
fn names_and_entry_points() {
    let mut e = world(false);
    assert!(nvse::plugins::ln_events(&e).is_none());
    assert_eq!(get_ln_event_mask(&mut e, b"oncellenter"), 1);
    assert_eq!(get_ln_event_mask(&mut e, b"OnControlUp"), 1 << 11);
    assert_eq!(
        get_ln_event_mask(&mut e, b"OnTriggerDown"),
        0,
        "not a name xNVSE asks"
    );
    assert_eq!(get_ln_event_mask(&mut e, b"OnActivate"), 0);
    let e = world(true);
    assert!(nvse::plugins::ln_events(&e).is_some());
}

/// The list events' handlers: the filter kinds, the refusals, the same handler twice, a removal
/// without a filter taking any, the flags.
#[test]
fn list_handlers() {
    let mut e = world(true);
    assert!(handler(&mut e, id::CELL_ENTER, S1, true, CELL_A, 0));
    assert!(
        !handler(&mut e, id::CELL_ENTER, S1, true, CELL_A, 0),
        "twice"
    );
    assert!(
        !handler(&mut e, id::CELL_EXIT, S1, true, BASE_A, 0),
        "a cell event takes a cell"
    );
    // An empty form list: refused; with a form: kind 3.
    assert!(!handler(&mut e, id::PLAYER_GRAB, S1, true, LIST, 0));
    e.mem.set_u32(LIST + 0x18, BASE_A);
    assert!(handler(&mut e, id::PLAYER_GRAB, S1, true, LIST, 0));
    assert!(
        handler(&mut e, id::PLAYER_GRAB, S2, true, REF_A, 0),
        "a reference: kind 1"
    );
    assert!(
        handler(&mut e, id::PLAYER_GRAB, S2, true, BASE_A, 0),
        "a base form: kind 2"
    );
    let kinds: Vec<u8> = state(&mut e).events[usize::from(id::PLAYER_GRAB)]
        .iter()
        .map(|h| h.filter_type)
        .collect();
    assert_eq!(kinds, [3, 1, 2]);
    // A form of a type no filter may be (a cell for a grab).
    assert!(!handler(&mut e, id::PLAYER_GRAB, S2, true, CELL_A, 0));
    // The crosshair by form type: a weapon (0x28) yes, past TESCaravanMoney no.
    assert!(handler(&mut e, id::CROSSHAIR_ON, S1, true, 0, 0x28));
    assert!(!handler(&mut e, id::CROSSHAIR_ON, S1, true, 0, 0x75));
    // The buttons need a mask to add.
    assert!(!handler(&mut e, id::BUTTON_DOWN, S1, true, 0, 0));
    assert!(handler(&mut e, id::BUTTON_DOWN, S1, true, 0, 0x1000));
    assert!(
        !handler(&mut e, id::BUTTON_DOWN, S1, true, 0, 0x3000),
        "a shared bit: there"
    );
    assert!(handler(&mut e, id::BUTTON_DOWN, S1, true, 0, 0x2000));
    assert_ne!(state(&mut e).flags & mask(id::BUTTON_DOWN), 0);
    // Removals: a stored handler with a filter needs its exact filter (a shared bit is not
    // enough); the flag goes with the last.
    assert!(
        !handler(&mut e, id::BUTTON_DOWN, S1, false, 0, 0),
        "no filter: not a filtered one"
    );
    assert!(!handler(&mut e, id::BUTTON_DOWN, S1, false, 0, 0x3000));
    assert!(handler(&mut e, id::BUTTON_DOWN, S1, false, 0, 0x1000));
    assert!(handler(&mut e, id::BUTTON_DOWN, S1, false, 0, 0x2000));
    assert_eq!(state(&mut e).flags & mask(id::BUTTON_DOWN), 0);
    assert!(
        !handler(&mut e, id::CELL_ENTER, S2, false, 0, 0),
        "another script's"
    );
    // A stored handler without a filter: any removal of the script takes it.
    assert!(handler(&mut e, id::CROSSHAIR_OFF, S2, true, 0, 0));
    assert!(handler(&mut e, id::CROSSHAIR_OFF, S2, false, REF_A, 0));
    assert!(state(&mut e).events[usize::from(id::CROSSHAIR_OFF)].is_empty());
}

/// The keys' and controls' handlers: a valid key (`kValidKeyCode`), a control below 28; a
/// removal with a negative key from every entry; an emptied entry dropped at the next frame.
#[test]
fn input_handlers() {
    let mut e = world(true);
    assert!(handler(&mut e, id::KEY_DOWN, S1, true, 0, 0x11));
    assert!(!handler(&mut e, id::KEY_DOWN, S1, true, 0, 0x11), "twice");
    assert!(handler(&mut e, id::KEY_UP, S1, true, 0, 0x11));
    assert!(
        !handler(&mut e, id::KEY_DOWN, S1, true, 0, 0),
        "key 0 is not valid"
    );
    assert!(!handler(&mut e, id::KEY_DOWN, S1, true, 0, 300));
    assert!(handler(&mut e, id::KEY_DOWN, S2, true, 0, 0x1e));
    assert!(handler(&mut e, id::CONTROL_DOWN, S1, true, 0, 4));
    assert!(!handler(&mut e, id::CONTROL_DOWN, S1, true, 0, 28));
    assert_eq!(
        state(&mut e).flags & (ON_KEY | ON_CONTROL),
        ON_KEY | ON_CONTROL
    );
    // S1's down handlers out of every key.
    assert!(handler(&mut e, id::KEY_DOWN, S1, false, 0, u32::MAX));
    assert!(!state(&mut e).keys[&0x11].on_down.contains(&S1));
    assert!(state(&mut e).keys[&0x11].on_up.contains(&S1));
    assert!(handler(&mut e, id::KEY_UP, S1, false, 0, 0x11));
    assert_ne!(state(&mut e).input_clear & ON_KEY, 0);
    process(&mut e);
    assert!(!state(&mut e).keys.contains_key(&0x11), "dropped");
    assert!(state(&mut e).keys.contains_key(&0x1e));
}

fn key(e: &mut Engine, k: u32) -> &mut nvse::input::KeyInfo {
    &mut e.ext::<nvse::Nvse>().input.keys[k as usize]
}

/// One frame's checks: keys and controls by their raw states; the cell left and entered; the
/// grab and the release (the released one checked by its id); the crosshair; nothing of the cell,
/// grab or crosshair the first frame after a load, or in menu mode; the pad's buttons and
/// triggers with a ready controller.
#[test]
fn a_frame() {
    let mut e = world(true);
    handler(&mut e, id::KEY_DOWN, S1, true, 0, 0x11);
    handler(&mut e, id::KEY_UP, S2, true, 0, 0x11);
    handler(&mut e, id::CONTROL_DOWN, S2, true, 0, 4);
    handler(&mut e, id::CELL_EXIT, S1, true, 0, 0);
    handler(&mut e, id::CELL_ENTER, S2, true, CELL_B, 0);
    handler(&mut e, id::PLAYER_GRAB, S1, true, 0, 0);
    handler(&mut e, id::PLAYER_RELEASE, S2, true, BASE_A, 0);
    handler(&mut e, id::CROSSHAIR_ON, S1, true, 0, 0x28);
    e.mem.set_u32(PLAYER_REF + 0x40, CELL_A);
    process(&mut e);
    assert!(
        calls().is_empty(),
        "the first frame after the start: nothing"
    );
    // A key and its control go down.
    key(&mut e, 0x11).raw_state = true;
    process(&mut e);
    assert_eq!(calls(), [(S1, 0, vec![0x11]), (S2, 0, vec![4])]);
    process(&mut e);
    assert!(calls().is_empty(), "no change");
    key(&mut e, 0x11).raw_state = false;
    process(&mut e);
    assert_eq!(
        calls(),
        [(S2, 0, vec![0x11])],
        "up; the control has no up handler"
    );
    // Into cell B: the exit (no filter) with A, the enter (filtered to B) with B.
    e.mem.set_u32(PLAYER_REF + 0x40, CELL_B);
    process(&mut e);
    assert_eq!(calls(), [(S1, 0, vec![CELL_A]), (S2, 0, vec![CELL_B])]);
    // Grab REF_A, then let go (its base passes the release's filter; its id still names it).
    e.mem.set_u32(PLAYER_REF + 0x638, REF_A);
    process(&mut e);
    assert_eq!(calls(), [(S1, 0, vec![REF_A])]);
    e.mem.set_u32(PLAYER_REF + 0x638, 0);
    process(&mut e);
    assert_eq!(calls(), [(S2, 0, vec![REF_A])]);
    // The crosshair on REF_A (its base a weapon, type 0x28).
    e.mem.set_u32(MANAGER + 0xfc, REF_A);
    process(&mut e);
    assert_eq!(calls(), [(S1, 0, vec![REF_A])]);
    // Menu mode: no cell, grab or crosshair checks.
    e.mem.set_u32(MANAGER + 0xc, 2);
    e.mem.set_u32(PLAYER_REF + 0x40, CELL_A);
    process(&mut e);
    assert!(calls().is_empty());
    // After a load: the first frame reports no change.
    e.mem.set_u32(MANAGER + 0xc, 1);
    pre_load(&mut e);
    process(&mut e);
    assert!(calls().is_empty(), "after the load");
    // A grab whose released reference's id no longer names it: no release.
    e.mem.set_u32(PLAYER_REF + 0x638, REF_A);
    process(&mut e);
    calls();
    e.mem.set_u32(FORM_MAP + 0x48, AREA + 0x900);
    e.mem.set_u32(PLAYER_REF + 0x638, 0);
    process(&mut e);
    assert!(calls().is_empty(), "the id names another form");
}

/// The pad with a ready controller: each button handler gets its buttons that went down or up;
/// the triggers' sets get 0 (left) or 1 (right) as they go down and up.
#[test]
fn the_pad() {
    let mut e = world(true);
    handler(&mut e, id::BUTTON_DOWN, S1, true, 0, 0x1000 | 0x2000);
    handler(&mut e, id::BUTTON_UP, S2, true, 0, 0x2000);
    state(&mut e).triggers[1].insert(S1);
    state(&mut e).triggers[2].insert(S2);
    state(&mut e).flags |= ON_TRIGGER;
    e.ext::<Jip>().input.ready = true;
    e.ext::<Jip>().input.pad.buttons = 0x3000;
    e.ext::<Jip>().input.pad.left_trigger = 9;
    e.mem.set_u32(PLAYER_REF + 0x40, CELL_A);
    process(&mut e);
    assert_eq!(calls(), [(S1, 0, vec![0x3000]), (S1, 0, vec![0])]);
    e.ext::<Jip>().input.pad.buttons = 0x1000;
    e.ext::<Jip>().input.pad.left_trigger = 0;
    e.ext::<Jip>().input.pad.right_trigger = 1;
    process(&mut e);
    assert_eq!(
        calls(),
        [(S2, 0, vec![0x2000])],
        "B went up; the triggers' other sets empty"
    );
    e.ext::<Jip>().input.pad.right_trigger = 0;
    process(&mut e);
    assert_eq!(calls(), [(S2, 0, vec![1])], "the right trigger up");
}
