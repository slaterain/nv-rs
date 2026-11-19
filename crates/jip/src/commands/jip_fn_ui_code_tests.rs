//! The three recipe menu commands, called by their opcodes, and the four seams they answer.
//! Expectations from JIP's bodies (`Cmd_ToggleCraftingMessages_Execute`,
//! `Cmd_UnlockRecipeMenuQuantity_Execute`, `Cmd_EnableImprovedRecipeMenu_Execute`) and its asm
//! (`RecipeMenuAcceptHook`, `RecipeMenuCloseHook`, `RefreshRecipeMenu`), the game's functions as
//! recording stand-ins.

use super::*;
use crate::commands::test_support::*;

const MENU: u32 = FORMS + 0x1000;
// The recipe menu's category (`+0x64`) and recipe list box (`+0x6C`); the list's fill
// (`00727680`), `SaveListState` (`007312E0`), `Filter` (`00729FE0`) and `RestoreListState`
// (`0072A660`), which JIP's `RefreshRecipeMenu` calls: the exe's numbers, not the module's.
const CATEGORY: u32 = 0x64;
const LIST: u32 = 0x6c;
const FILL_LIST: u32 = 0x0072_7680;
const SAVE_LIST_STATE: u32 = 0x0073_12e0;
const FILTER: u32 = 0x0072_9fe0;
const RESTORE_LIST_STATE: u32 = 0x0072_a660;
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
        assert!(
            calls_to(QUANTITY_MENU_CREATE).is_empty(),
            "{id}: not the target"
        );
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
    assert_eq!(
        calls_to(PICK_UP_SOUND_NAME),
        vec![vec![PLAYER_AT, FORM, 1, 0]]
    );
    assert!(
        calls_to(PLAY_SOUND_BY_EDITOR_NAME).is_empty(),
        "no sound name"
    );
    SOUND.with(|t| t.set(SOUND_NAME));
    e.mem.set_u32(RECIPE_MENU, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(ask(&mut e, CLOSE, &[FORM]), Some(0));
    let played = calls_to(PLAY_SOUND_BY_EDITOR_NAME);
    assert_eq!(played.len(), 1);
    let w = &played[0];
    assert_eq!(
        (w[0], w[2], w[3], w[4], w[5]),
        (PLAYER_AT, SOUND_NAME, 0, 0x121, 0)
    );
    assert!(calls_to(FILL_LIST).is_empty(), "no menu: no refresh");
}

thread_local! {
    static OUT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// JIP installed over stand-ins of the six wrapped functions (each records its words; the
/// filters answer `OUT`), forms in memory, the player and the INI page mapped.
fn wrap_world() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    OUT.with(|o| o.set(0));
    let mut e = crate::test_support::engine();
    macro_rules! base {
        ($at:expr) => {
            e.register($at.addr(), |_, w| {
                record($at.addr(), w);
                eax(OUT.with(|o| o.get()))
            });
        };
    }
    base!(SHOW_QUEST_NAME);
    base!(SHOW_QUEST_OBJECTIVE);
    base!(RECIPE_FILTER_FUNC);
    base!(BARTER_FILTER_FUNC);
    base!(CONTAINER_FILTER_FUNC);
    base!(INVENTORY_FILTER_FUNC);
    e.register(SET_ACTIVE_QUEST, |_, w| {
        record(SET_ACTIVE_QUEST, w);
        eax(0)
    });
    crate::install_as(&mut e, true);
    e.map(FORMS, 0x10_0000);
    for page in [0x011c_b000, 0x011d_e000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(PLAYER, PLAYER_AT);
    e.mem.set_i32(SHOW_CHALLENGE_UPDATES, 0);
    e
}

/// SuppressQuestMessages: for one main-loop pass the HUD's two quest functions are JIP's (no
/// message; the active quest kept as JIP's hook does), `bShowChallengeUpdates` 0, then 1; a
/// second call while hidden adds nothing.
#[test]
fn quest_messages_hidden_for_one_frame() {
    let mut e = wrap_world();
    let quest = FORMS + 0x5000;
    e.call(SHOW_QUEST_NAME.addr(), &[quest]);
    e.call(SHOW_QUEST_OBJECTIVE.addr(), &[0x1234, 1, 0]);
    assert_eq!(calls().len(), 2, "shown: the exe's");
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2890, &[]);
    call(&mut e, 0x2890, &[]);
    assert_eq!(e.mem.i32(SHOW_CHALLENGE_UPDATES), 0);
    // No active quest: this one (not done) becomes it.
    e.call(SHOW_QUEST_NAME.addr(), &[quest]);
    assert_eq!(calls(), vec![(SET_ACTIVE_QUEST, vec![PLAYER_AT, quest])]);
    // Active, completed: no longer active; failed and not active: nothing; not done with
    // another active: nothing.
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.set_u32(PLAYER_AT + 0x6b8, quest);
    e.mem.set_u8(quest + 0x3c, 2);
    e.call(SHOW_QUEST_NAME.addr(), &[quest]);
    // Failed (0x40) and not the active one: nothing; not done with another active: nothing.
    e.mem.set_u8(quest + 0x100 + 0x3c, 0x40);
    e.call(SHOW_QUEST_NAME.addr(), &[quest + 0x100]);
    e.call(SHOW_QUEST_NAME.addr(), &[quest + 0x200]);
    e.call(SHOW_QUEST_OBJECTIVE.addr(), &[0x1234, 1, 0]);
    assert_eq!(calls(), vec![(SET_ACTIVE_QUEST, vec![PLAYER_AT, 0])]);
    // Failed and the active one: no longer active.
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.set_u32(PLAYER_AT + 0x6b8, quest + 0x100);
    e.call(SHOW_QUEST_NAME.addr(), &[quest + 0x100]);
    assert_eq!(
        calls(),
        vec![(SET_ACTIVE_QUEST, vec![PLAYER_AT, 0])],
        "failed"
    );
    crate::main_loop::cycle(&mut e);
    assert_eq!(e.mem.i32(SHOW_CHALLENGE_UPDATES), 1, "shown again");
    CALLS.with(|c| c.borrow_mut().clear());
    e.call(SHOW_QUEST_OBJECTIVE.addr(), &[0x1234, 1, 0]);
    assert_eq!(calls().len(), 1, "the exe's again");
    crate::main_loop::cycle(&mut e);
    assert_eq!(e.mem.i32(SHOW_CHALLENGE_UPDATES), 1, "one callback only");
}

/// A named form of `kind` at `at` whose name text is `name` (at the kind's name offset).
fn named(e: &mut Engine, at: u32, kind: u8, name: &[u8]) {
    e.mem.set_u8(at + 4, kind);
    let offset = match kind {
        0x6a => 0x1c,
        0x31 => 0x4c,
        _ => 0x34,
    };
    let text = at + 0x200;
    e.mem.write(text, name);
    e.mem.set_u8(text + name.len() as u32, 0);
    e.mem.set_u32(at + offset, text);
}

/// Before InitItemFilter the filters are the exe's and SetItemFilter does nothing; after it,
/// an item the exe keeps is left out when its name lacks the text (without case), the recipe
/// filter taking the recipe, the others an entry's form (`+8`); an item the exe leaves out
/// stays out; no text keeps all.
#[test]
fn the_item_filter() {
    let mut e = wrap_world();
    let (rifle, note, recipe, entry) = (
        FORMS + 0x6000,
        FORMS + 0x6400,
        FORMS + 0x6800,
        FORMS + 0x6c00,
    );
    named(&mut e, rifle, 0x28, b"Hunting Rifle");
    named(&mut e, note, 0x31, b"Holotape");
    named(&mut e, recipe, 0x6a, b"Rifle Rounds");
    e.mem.set_u32(entry + 8, rifle);
    let out = |e: &mut Engine, slot: WrapSlot, arg: u32| e.call(slot.addr(), &[arg]).eax & 0xff;
    call(&mut e, 0x272d, &[s(b"zzz")]);
    assert_eq!(out(&mut e, INVENTORY_FILTER_FUNC, entry), 0, "not started");
    call(&mut e, 0x272c, &[]);
    for slot in [
        INVENTORY_FILTER_FUNC,
        CONTAINER_FILTER_FUNC,
        BARTER_FILTER_FUNC,
    ] {
        assert_eq!(out(&mut e, slot, entry), 0, "no text yet");
    }
    call(&mut e, 0x272d, &[s(b"RIFLE")]);
    for slot in [
        INVENTORY_FILTER_FUNC,
        CONTAINER_FILTER_FUNC,
        BARTER_FILTER_FUNC,
    ] {
        assert_eq!(
            out(&mut e, slot, entry),
            0,
            "{:x}: has the text",
            slot.addr()
        );
        assert_eq!(out(&mut e, slot, 0), 0, "no entry: kept");
    }
    assert_eq!(out(&mut e, RECIPE_FILTER_FUNC, recipe), 0);
    call(&mut e, 0x272d, &[s(b"tape")]);
    for slot in [
        INVENTORY_FILTER_FUNC,
        CONTAINER_FILTER_FUNC,
        BARTER_FILTER_FUNC,
    ] {
        assert_eq!(out(&mut e, slot, entry), 1, "{:x}: lacks it", slot.addr());
    }
    assert_eq!(out(&mut e, RECIPE_FILTER_FUNC, recipe), 1);
    e.mem.set_u32(entry + 8, note);
    assert_eq!(
        out(&mut e, INVENTORY_FILTER_FUNC, entry),
        0,
        "a note's name"
    );
    call(&mut e, 0x272d, &[s(b"RIFLE")]);
    OUT.with(|o| o.set(1));
    e.mem.set_u32(entry + 8, rifle);
    assert_eq!(
        out(&mut e, INVENTORY_FILTER_FUNC, entry),
        1,
        "the exe's out stays out"
    );
    OUT.with(|o| o.set(0));
    call(&mut e, 0x272d, &[]);
    assert_eq!(
        out(&mut e, INVENTORY_FILTER_FUNC, entry),
        0,
        "no text: all kept"
    );
    call(&mut e, 0x272c, &[]);
    call(&mut e, 0x272d, &[s(b"tape")]);
    assert_eq!(
        out(&mut e, INVENTORY_FILTER_FUNC, entry),
        1,
        "a second Init keeps going"
    );
}
