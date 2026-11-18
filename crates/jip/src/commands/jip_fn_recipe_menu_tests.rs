//! The three recipe menu commands, called by their opcodes, and the four seams they answer.
//! Expectations from JIP's bodies (`Cmd_ToggleCraftingMessages_Execute`,
//! `Cmd_UnlockRecipeMenuQuantity_Execute`, `Cmd_EnableImprovedRecipeMenu_Execute`) and its asm
//! (`RecipeMenuAcceptHook`, `RecipeMenuCloseHook`, `RefreshRecipeMenu`), the game's functions as
//! recording stand-ins.

use super::*;
use crate::commands::test_support::*;

const MENU: u32 = FORMS + 0x1000;
const LIST_VT: u32 = FORMS + 0x1400;
const PLAYER_AT: u32 = FORMS + 0x2000;
const PROC: u32 = FORMS + 0x2400;
const PROC_VT: u32 = FORMS + 0x2800;
const MANAGER: u32 = FORMS + 0x3000;
const CURSOR_TILE: u32 = FORMS + 0x3400;
const CURSOR_NODE: u32 = FORMS + 0x3800;
const ACCEPT: u32 = FORMS + 0x3c00;
const SOUND_NAME: u32 = FORMS + 0x4000;
const FORM: u32 = FORMS + 0x4400;
const SLOT_FNS: u32 = 0x0dea_7000;

fn record(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

thread_local! {
    static TARGET: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static WANTS_3D: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static SOUND: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The recipe menu open (its list's vtable slots `0`, `0x10`, `0x14` and `0x1C` recording; the
/// first item 0x77), the player with a process whose slot `0x474` answers `WANTS_3D`, the
/// interface manager's cursor tile and its node, every game call recording.
fn world() -> Engine {
    TARGET.with(|t| t.set(0));
    WANTS_3D.with(|t| t.set(0));
    SOUND.with(|t| t.set(0));
    let mut e = engine();
    e.map(0x011d_8000, 0x1000);
    e.map(0x011d_e000, 0x1000);
    e.mem.set_u32(RECIPE_MENU, MENU);
    e.mem.set_u32(MENU + ACCEPT_TILE, ACCEPT);
    e.mem.set_u32(MENU + CATEGORY, 0x55);
    e.mem.set_u32(MENU + LIST, LIST_VT);
    // A stand-in at `$at` that records its words under `$tag` and answers `$v`.
    macro_rules! stand_in {
        ($at:expr, $tag:expr, $v:expr) => {
            e.register($at, |_, w| {
                record($tag, w);
                eax($v)
            });
        };
    }
    for (i, slot) in [0u32, 0x10, 0x14, 0x1c].into_iter().enumerate() {
        e.mem.set_u32(LIST_VT + slot, SLOT_FNS + 4 * i as u32);
    }
    stand_in!(SLOT_FNS, 0x1_0000, 0);
    stand_in!(SLOT_FNS + 4, 0x1_0010, 0);
    stand_in!(SLOT_FNS + 8, 0x1_0014, 0x77);
    stand_in!(SLOT_FNS + 12, 0x1_001c, 0);
    e.mem.set_u32(PLAYER, PLAYER_AT);
    e.mem.set_u32(PLAYER_AT + PROCESS, PROC);
    e.mem.set_u32(PROC, PROC_VT);
    e.mem.set_u32(PROC_VT + 0x474, SLOT_FNS + 0x40);
    e.register(SLOT_FNS + 0x40, |_, w| {
        record(0x1_0474, w);
        eax(WANTS_3D.with(|t| t.get()))
    });
    e.mem.set_u32(INTERFACE_MANAGER, MANAGER);
    e.mem.set_u32(MANAGER + CURSOR, CURSOR_TILE);
    e.mem.set_u32(CURSOR_TILE + 0x2c, CURSOR_NODE);
    e.mem.set_u32(CURSOR_NODE + 0x58, 0x1111);
    e.mem.set_u32(CURSOR_NODE + 0x60, 0x2222);
    e.mem.set_u32(CAN_MAKE, 12);
    e.register(TILE_IS_TRUE, |_, w| {
        record(TILE_IS_TRUE, w);
        eax(TARGET.with(|t| t.get()))
    });
    e.register(PICK_UP_SOUND_NAME, |_, w| {
        record(PICK_UP_SOUND_NAME, w);
        eax(SOUND.with(|t| t.get()))
    });
    stand_in!(QUANTITY_MENU_CREATE, QUANTITY_MENU_CREATE, 0);
    stand_in!(PLAY_SOUND_BY_EDITOR_NAME, PLAY_SOUND_BY_EDITOR_NAME, 0);
    stand_in!(UPDATE_PLAYER_3D, UPDATE_PLAYER_3D, 0);
    stand_in!(FILL_LIST, FILL_LIST, 0);
    stand_in!(SAVE_LIST_STATE, SAVE_LIST_STATE, 0);
    stand_in!(FILTER, FILTER, 0);
    stand_in!(RESTORE_LIST_STATE, RESTORE_LIST_STATE, 0);
    e
}

fn ask(e: &mut Engine, seam: SiteSlot, words: &[u32]) -> Option<u32> {
    e.site(seam.addr(), words)
}

fn result_of(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call(e, opcode, args).1
}

/// Before any command: the exe's 100, the message shown, the exe's click and close.
#[test]
fn the_seams_start_as_the_exe() {
    let mut e = world();
    assert_eq!(ask(&mut e, MOST, &[]), Some(100));
    assert_eq!(ask(&mut e, MESSAGE, &[]), Some(0));
    for id in [0, 7, 8, 12, 15] {
        assert_eq!(ask(&mut e, CLICK, &[MENU, id]), None, "{id}");
    }
    assert_eq!(ask(&mut e, CLOSE, &[FORM]), None);
    assert!(calls().is_empty());
}

#[test]
fn unlock_sets_the_most_and_no_argument_is_no_limit() {
    let mut e = world();
    call(&mut e, 0x2714, &[n(250)]);
    assert_eq!(ask(&mut e, MOST, &[]), Some(250));
    call(&mut e, 0x2714, &[]);
    assert_eq!(ask(&mut e, MOST, &[]), Some(0x7fff_ffff));
}

/// The answer is the state; a toggle that differs sets it (0 off, any other value on).
#[test]
fn crafting_messages_toggle() {
    let mut e = world();
    assert_eq!(result_of(&mut e, 0x2836, &[]), 1.0);
    assert_eq!(result_of(&mut e, 0x2836, &[n(0)]), 1.0, "was on");
    assert_eq!(ask(&mut e, MESSAGE, &[]), Some(1), "off");
    assert_eq!(result_of(&mut e, 0x2836, &[n(0)]), 0.0);
    assert_eq!(ask(&mut e, MESSAGE, &[]), Some(1));
    assert_eq!(result_of(&mut e, 0x2836, &[n(5)]), 0.0, "was off");
    assert_eq!(ask(&mut e, MESSAGE, &[]), Some(0), "on");
    assert_eq!(result_of(&mut e, 0x2836, &[n(1)]), 1.0);
    assert_eq!(ask(&mut e, MESSAGE, &[]), Some(0));
}

/// Once: no limit; a later UnlockRecipeMenuQuantity still sets it, a second Enable does not.
#[test]
fn the_improved_menu_is_once_and_lifts_the_limit() {
    let mut e = world();
    call(&mut e, 0x272b, &[]);
    assert_eq!(ask(&mut e, MOST, &[]), Some(0x7fff_ffff));
    call(&mut e, 0x2714, &[n(40)]);
    call(&mut e, 0x272b, &[]);
    assert_eq!(ask(&mut e, MOST, &[]), Some(40));
}

/// With the improved menu: ids 7 and 15 open the quantity menu only while the accept tile's
/// target is true; 9 to 14 do nothing; the others stay the exe's.
#[test]
fn the_improved_click() {
    let mut e = world();
    call(&mut e, 0x272b, &[]);
    for id in [7, 15] {
        CALLS.with(|c| c.borrow_mut().clear());
        TARGET.with(|t| t.set(0));
        assert_eq!(ask(&mut e, CLICK, &[MENU, id]), Some(0), "{id}");
        assert_eq!(calls_to(TILE_IS_TRUE), vec![vec![ACCEPT, 0xfaf]], "{id}");
        assert!(calls_to(QUANTITY_MENU_CREATE).is_empty(), "{id}: not the target");
        TARGET.with(|t| t.set(1));
        assert_eq!(ask(&mut e, CLICK, &[MENU, id]), Some(0), "{id}");
        assert_eq!(
            calls_to(QUANTITY_MENU_CREATE),
            vec![vec![12, 0x0072_84f0, 0x7fff_ffff]],
            "{id}"
        );
    }
    CALLS.with(|c| c.borrow_mut().clear());
    for id in 9..=14 {
        assert_eq!(ask(&mut e, CLICK, &[MENU, id]), Some(0), "{id}");
    }
    for id in [0, 1, 2, 3, 6, 8, 16, u32::MAX] {
        assert_eq!(ask(&mut e, CLICK, &[MENU, id]), None, "{id}");
    }
    assert!(calls().is_empty());
}

/// With the improved menu the craft's close keeps the menu: with messages on no sound; the 3D
/// update only when the process asks; the list refilled for the menu's category and filtered
/// as the category buttons do; the cursor node's two words cleared.
#[test]
fn the_improved_close_refreshes_the_list() {
    let mut e = world();
    call(&mut e, 0x272b, &[]);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(ask(&mut e, CLOSE, &[FORM]), Some(0));
    let list = MENU + LIST;
    assert_eq!(
        calls(),
        vec![
            (0x1_0474, vec![PROC, 1]),
            (0x1_001c, vec![list]),
            (FILL_LIST, vec![MENU, 0, 0x55]),
            (0x1_0014, vec![list, 0, 0]),
            (0x1_0000, vec![list, 0x77]),
            (0x1_0010, vec![list]),
            (SAVE_LIST_STATE, vec![list]),
            (FILTER, vec![list, 0x0072_8d60]),
            (RESTORE_LIST_STATE, vec![list, 1]),
        ]
    );
    assert_eq!(e.mem.u32(CURSOR_NODE + 0x58), 0);
    assert_eq!(e.mem.u32(CURSOR_NODE + 0x60), 0);
    WANTS_3D.with(|t| t.set(1));
    CALLS.with(|c| c.borrow_mut().clear());
    ask(&mut e, CLOSE, &[FORM]);
    assert_eq!(calls_to(UPDATE_PLAYER_3D), vec![vec![PLAYER_AT]]);
}

/// Messages off: the form's pick-up sound, when it has one, before the refresh; no menu open:
/// only the sound.
#[test]
fn the_improved_close_plays_the_sound_when_messages_are_off() {
    let mut e = world();
    call(&mut e, 0x272b, &[]);
    call(&mut e, 0x2836, &[n(0)]);
    CALLS.with(|c| c.borrow_mut().clear());
    ask(&mut e, CLOSE, &[FORM]);
    assert_eq!(calls_to(PICK_UP_SOUND_NAME), vec![vec![PLAYER_AT, FORM, 1, 0]]);
    assert!(calls_to(PLAY_SOUND_BY_EDITOR_NAME).is_empty(), "no sound name");
    SOUND.with(|t| t.set(SOUND_NAME));
    e.mem.set_u32(RECIPE_MENU, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(ask(&mut e, CLOSE, &[FORM]), Some(0));
    let played = calls_to(PLAY_SOUND_BY_EDITOR_NAME);
    assert_eq!(played.len(), 1);
    let w = &played[0];
    assert_eq!((w[0], w[2], w[3], w[4], w[5]), (PLAYER_AT, SOUND_NAME, 0, 0x121, 0));
    assert!(calls_to(FILL_LIST).is_empty(), "no menu: no refresh");
}
