//! `ShowQuantityMenu` (with JIP's `QuantityMenuCallback` through the DoClick seam) and
//! `CloseActiveMenu`, called by their opcodes. Expectations from JIP's bodies
//! (functions_jip/jip_fn_ui.h: `Cmd_ShowQuantityMenu_Execute`, `QuantityMenuCallback`,
//! `Cmd_CloseActiveMenu_Execute`) and the exe (`QuantityMenu::DoClick` 007ABE40: OK is id 4 with
//! the count, Cancel id 5 with 0), never from the port's output. The game functions are
//! stand-ins that record.

use super::*;
use crate::commands::test_support::*;
use std::cell::RefCell;

const SHOW: u32 = 0x2825;
const CLOSE: u32 = 0x28cc;
const IM_AT: u32 = FORMS + 0x1_0000;
const SCPT: u32 = FORMS + 0x2_0000;
const MISC: u32 = FORMS + 0x2_2000;
const ARRAY_AT: u32 = FORMS + 0x3_0000;
const TILE_VTBL: u32 = FORMS + 0x3_8000;
const TILE_DESTROY_FN: u32 = 0x0000_9000;
const RENDERED_AT: u32 = FORMS + 0x4_0000;
const LIST: u32 = FORMS + 0x5_0000;
const SEAM: u32 = 0x007a_becc;
// The exe's and JIP's numbers, written here from JIP's headers (nvse/GameUI.h: `InterfaceManager`
// `currentMode` +0xC, `menuStack` +0x114, `renderedMenu` +0x16C, `pipBoyMode` +0x4BC,
// `pipBoyModeCallback` +0x4C0; `kMenuType_Message` 0x3E9, `kMenuType_Quantity` 0x3F8) and the exe
// (`g_interfaceManager` 0x11D8A80, `QuantityMenu::pMe` 0x11DA618, `QuantityMenu::ShowMenu` 0x7ABA00,
// `FORenderedMenu::Close` 0x7FFD50, the message menu's clean-up 0x7AA480), not taken from the module.
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const IM_CURRENT_MODE: u32 = 0x0c;
const IM_MENU_STACK: u32 = 0x114;
const IM_RENDERED_MENU: u32 = 0x16c;
const IM_PIPBOY_MODE: u32 = 0x4bc;
const IM_PIPBOY_MODE_CALLBACK: u32 = 0x4c0;
const MENU_MIN: u32 = 0x3e9;
const MESSAGE_MENU: u32 = 0x3e9;
const QUANTITY: u32 = 0x3f8;
const QUANTITY_MENU: u32 = 0x011d_a618;
const QUANTITY_SHOW_MENU: u32 = 0x007a_ba00;
const RENDERED_MENU_CLOSE: u32 = 0x007f_fd50;
const MESSAGE_MENU_CLEAR: u32 = 0x007a_a480;

thread_local! {
    /// The stand-ins' calls: the address and the words.
    static LOG: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
    /// What the quantity menu's `ShowMenu` stand-in answers.
    static ROOT: std::cell::Cell<u32> = const { std::cell::Cell::new(0x4444) };
}

fn log() -> Vec<(u32, Vec<u32>)> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

/// A stand-in at `A` that notes its words and answers `EAX`.
fn note<const A: u32, const EAX: u32>(_: &mut Engine, w: &[u32]) -> Ret {
    LOG.with(|l| l.borrow_mut().push((A, w.to_vec())));
    Ret {
        eax: EAX,
        ..Ret::default()
    }
}

fn setup() -> Engine {
    LOG.with(|l| l.borrow_mut().clear());
    ROOT.with(|r| r.set(0x4444));
    let _ = crate::script_call::calls();
    let mut e = engine();
    for page in [0x011d_8000, 0x011d_a000, 0x011f_3000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(INTERFACE_MANAGER, IM_AT);
    e.mem.set_u32(0x011f_350c, ARRAY_AT);
    form(&mut e, SCPT, 0x11, 0x0000_0911);
    form(&mut e, MISC, 0x1f, 0x0000_0a1f);
    set_ref(&mut e, 1, SCPT);
    set_ref(&mut e, 3, MISC);
    e.register(QUANTITY_SHOW_MENU, |_, w| {
        LOG.with(|l| l.borrow_mut().push((QUANTITY_SHOW_MENU, w.to_vec())));
        Ret {
            eax: ROOT.with(|r| r.get()),
            ..Ret::default()
        }
    });
    e.register(RENDERED_MENU_CLOSE, note::<RENDERED_MENU_CLOSE, 0>);
    e.register(MESSAGE_MENU_CLEAR, note::<MESSAGE_MENU_CLEAR, 0>);
    e.register(TILE_DESTROY_FN, note::<TILE_DESTROY_FN, 0>);
    e.mem.set_u32(TILE_VTBL, TILE_DESTROY_FN);
    nvse::lambda::insert_for_tests(&mut e, SCPT, LIST);
    e
}

fn kept(e: &Engine) -> usize {
    nvse::lambda::saved_count(e, SCPT)
}

// ---- ShowQuantityMenu ----

/// The menu shown with the count, JIP's routine and the default (the count when the default is
/// below 0, above the count, or not given); result 1, the script and its lambda variables kept.
#[test]
fn the_menu_is_shown_with_jips_routine() {
    for (args, start) in [
        (vec![r(1), n(10), n(3)], 3),
        (vec![r(1), n(10), n(0)], 0),
        (vec![r(1), n(10), n(10)], 10),
        (vec![r(1), n(10), n(11)], 10),
        (vec![r(1), n(10), n(-1)], 10),
        (vec![r(1), n(10)], 10),
    ] {
        let mut e = setup();
        assert_eq!(call_on(&mut e, SHOW, 0, &args).1, 1.0, "{start}");
        assert_eq!(
            log(),
            vec![(QUANTITY_SHOW_MENU, vec![10, QUANTITY_MENU_CALLBACK, start])]
        );
        assert_eq!(quantity_menu_script(&mut e), SCPT);
        assert_eq!(kept(&e), 1);
    }
}

/// Not shown: a quantity menu open (`QuantityMenu::Get`), a count of 0 or below, a callback that is
/// not a script. Shown but not opened (`ShowMenu` answers 0): result 0, nothing kept.
#[test]
fn nothing_without_a_count_or_a_script() {
    for (open, args) in [
        (true, vec![r(1), n(5)]),
        (false, vec![r(1), n(0)]),
        (false, vec![r(1), n(-4)]),
        (false, vec![r(3), n(5)]),
    ] {
        let mut e = setup();
        if open {
            e.mem.set_u32(QUANTITY_MENU, 0x7777);
        }
        assert_eq!(call_on(&mut e, SHOW, 0, &args).1, 0.0);
        assert!(log().is_empty());
        assert_eq!(quantity_menu_script(&mut e), 0);
        assert_eq!(kept(&e), 0);
    }
    let mut e = setup();
    ROOT.with(|r| r.set(0));
    assert_eq!(call_on(&mut e, SHOW, 0, &[r(1), n(5)]).1, 0.0);
    assert_eq!(log().len(), 1);
    assert_eq!(quantity_menu_script(&mut e), 0);
    assert_eq!(kept(&e), 0);
}

/// DoClick's seam with JIP's routine (words: the routine, the count DoClick passes, the id): OK
/// (4) gives the script the count, 0 included; Cancel (5, DoClick's 0) gives -1. Answered, the
/// script taken out and its lambda variables given up; a second answer calls nothing.
#[test]
fn the_seam_gives_the_script_the_count_or_minus_one() {
    for (count, id, value) in [(7u32, 4u32, 7u32), (0, 4, 0), (0, 5, u32::MAX)] {
        let mut e = setup();
        call_on(&mut e, SHOW, 0, &[r(1), n(10)]);
        let _ = crate::script_call::calls();
        assert_eq!(e.site(SEAM, &[QUANTITY_MENU_CALLBACK, count, id]), Some(0));
        assert_eq!(crate::script_call::calls(), vec![(SCPT, 0, vec![value])]);
        assert_eq!(quantity_menu_script(&mut e), 0);
        assert_eq!(kept(&e), 0);
        assert_eq!(e.site(SEAM, &[QUANTITY_MENU_CALLBACK, count, id]), Some(0));
        assert!(crate::script_call::calls().is_empty());
    }
}

/// Another callback (the game's own) at the seam: no answer, the script stays.
#[test]
fn the_seam_leaves_the_games_own_callbacks() {
    let mut e = setup();
    call_on(&mut e, SHOW, 0, &[r(1), n(10)]);
    assert_eq!(e.site(SEAM, &[0x0077_0000, 4, 4]), None);
    assert!(crate::script_call::calls().is_empty());
    assert_eq!(quantity_menu_script(&mut e), SCPT);
}

/// The routine called directly (not through DoClick): the count as OK.
#[test]
fn the_routine_alone_gives_the_count() {
    let mut e = setup();
    call_on(&mut e, SHOW, 0, &[r(1), n(10)]);
    let _ = crate::script_call::calls();
    e.call(QUANTITY_MENU_CALLBACK, &[0]);
    assert_eq!(crate::script_call::calls(), vec![(SCPT, 0, vec![0])]);
    assert_eq!(kept(&e), 0);
}

// ---- CloseActiveMenu ----

/// The interface manager in menu mode with `stack` as its menu stack (entry 0 first) and a tile
/// (with the stand-in vtable) for each menu id in it.
fn menus(e: &mut Engine, stack: &[u32]) {
    e.mem.set_u32(IM_AT + IM_CURRENT_MODE, 2);
    for (i, &id) in stack.iter().enumerate() {
        e.mem.set_u32(IM_AT + IM_MENU_STACK + 4 * i as u32, id);
        if id >= MENU_MIN {
            e.mem.set_u32(ARRAY_AT + 4 * (id - MENU_MIN), tile_of(id));
            e.mem.set_u32(tile_of(id), TILE_VTBL);
        }
    }
}

fn tile_of(id: u32) -> u32 {
    FORMS + 0x6_0000 + 0x40 * (id - MENU_MIN)
}

fn stack(e: &Engine) -> Vec<u32> {
    (0..10)
        .map(|i| e.mem.u32(IM_AT + IM_MENU_STACK + 4 * i))
        .collect()
}

/// Without `closeAll`: the top entry (the highest index set) closed, its tile destroyed and its
/// entry cleared, the rest left. With it: every entry from the top down.
#[test]
fn the_top_menu_or_every_menu_is_closed() {
    let mut e = setup();
    menus(&mut e, &[0x3ea, 0, 0x3ff, 0, 0, 0, 0, 0, 0, 0]);
    call_on(&mut e, CLOSE, 0, &[]);
    assert_eq!(log(), vec![(TILE_DESTROY_FN, vec![tile_of(0x3ff), 1])]);
    assert_eq!(stack(&e), vec![0x3ea, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut e = setup();
    menus(&mut e, &[0x3ea, 0, 0x3ff, 0, 0, 0, 0, 0, 0, 0x3eb]);
    call_on(&mut e, CLOSE, 0, &[n(1)]);
    assert_eq!(
        log(),
        vec![
            (TILE_DESTROY_FN, vec![tile_of(0x3eb), 1]),
            (TILE_DESTROY_FN, vec![tile_of(0x3ff), 1]),
            (TILE_DESTROY_FN, vec![tile_of(0x3ea), 1]),
        ]
    );
    assert_eq!(stack(&e), vec![0; 10]);
}

/// In game mode (`currentMode` 1 or 0): nothing.
#[test]
fn nothing_in_game_mode() {
    for mode in [0, 1] {
        let mut e = setup();
        menus(&mut e, &[0x3ea, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        e.mem.set_u32(IM_AT + IM_CURRENT_MODE, mode);
        call_on(&mut e, CLOSE, 0, &[n(1)]);
        assert!(log().is_empty());
        assert_eq!(stack(&e)[0], 0x3ea);
    }
}

/// The rendered menu's tile: `FORenderedMenu::Close` on it instead of the destroy; another
/// rendered menu's tile: the destroy.
#[test]
fn the_rendered_menu_is_closed_through_its_own_close() {
    let mut e = setup();
    menus(&mut e, &[0x3ea, 0x3ff, 0, 0, 0, 0, 0, 0, 0, 0]);
    e.mem.set_u32(IM_AT + IM_RENDERED_MENU, RENDERED_AT);
    e.mem.set_u32(RENDERED_AT + 0x18, tile_of(0x3ff));
    call_on(&mut e, CLOSE, 0, &[n(1)]);
    assert_eq!(
        log(),
        vec![
            (RENDERED_MENU_CLOSE, vec![RENDERED_AT]),
            (TILE_DESTROY_FN, vec![tile_of(0x3ea), 1]),
        ]
    );
}

/// The message menu: the tile closed, then the game's clean-up and `MessageBoxExAlt`'s kept
/// scripts given up. The quantity menu: `ShowQuantityMenu`'s script given up (and its answer then
/// calls nothing). A menu without a tile: only the clean-up and the entry.
#[test]
fn the_message_and_quantity_menus_give_up_their_scripts() {
    let mut e = setup();
    e.register(0x0070_3e80, note::<0x0070_3e80, 1>);
    call_on(&mut e, 0x2826, 0, &[r(1), s(b"M|A")]);
    assert_eq!(
        super::super::jip_fn_ui::message_box_scripts_pending(&mut e),
        1
    );
    log();
    let _ = crate::script_call::calls();
    menus(&mut e, &[MESSAGE_MENU, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    call_on(&mut e, CLOSE, 0, &[]);
    assert!(
        crate::script_call::calls().is_empty(),
        "the kept box script is dropped, never run"
    );
    assert_eq!(
        log(),
        vec![
            (TILE_DESTROY_FN, vec![tile_of(MESSAGE_MENU), 1]),
            (MESSAGE_MENU_CLEAR, vec![]),
        ]
    );
    assert_eq!(
        super::super::jip_fn_ui::message_box_scripts_pending(&mut e),
        0
    );
    assert_eq!(kept(&e), 0);

    let mut e = setup();
    call_on(&mut e, SHOW, 0, &[r(1), n(10)]);
    log();
    let _ = crate::script_call::calls();
    menus(&mut e, &[QUANTITY, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    e.mem.set_u32(ARRAY_AT + 4 * (QUANTITY - MENU_MIN), 0);
    call_on(&mut e, CLOSE, 0, &[]);
    assert!(
        crate::script_call::calls().is_empty(),
        "the kept quantity script is dropped, never run (not as Cancel either)"
    );
    assert!(log().is_empty(), "no tile: nothing closed");
    assert_eq!(quantity_menu_script(&mut e), 0);
    assert_eq!(kept(&e), 0);
    assert_eq!(stack(&e)[0], 0);
    assert_eq!(e.site(SEAM, &[QUANTITY_MENU_CALLBACK, 3, 4]), Some(0));
    assert!(crate::script_call::calls().is_empty());
}

/// Both kept at once (a `MessageBoxExAlt` box and a `ShowQuantityMenu` menu): closing the message
/// menu drops only the box's script, closing the quantity menu only the quantity script, closing
/// another menu neither.
#[test]
fn each_menu_drops_only_its_own_script() {
    let both = |e: &mut Engine| {
        e.register(0x0070_3e80, note::<0x0070_3e80, 1>);
        call_on(e, 0x2826, 0, &[r(1), s(b"M|A")]);
        call_on(e, SHOW, 0, &[r(1), n(10)]);
        log();
        let _ = crate::script_call::calls();
    };
    let pending = |e: &mut Engine| super::super::jip_fn_ui::message_box_scripts_pending(e);
    let mut e = setup();
    both(&mut e);
    menus(&mut e, &[MESSAGE_MENU, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    call_on(&mut e, CLOSE, 0, &[]);
    assert_eq!(pending(&mut e), 0);
    assert_eq!(
        quantity_menu_script(&mut e),
        SCPT,
        "the quantity script stays"
    );
    let mut e = setup();
    both(&mut e);
    menus(&mut e, &[QUANTITY, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    call_on(&mut e, CLOSE, 0, &[]);
    assert_eq!(quantity_menu_script(&mut e), 0);
    assert_eq!(pending(&mut e), 1, "the box's script stays");
    let mut e = setup();
    both(&mut e);
    menus(&mut e, &[0x3ea, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    call_on(&mut e, CLOSE, 0, &[]);
    assert_eq!(quantity_menu_script(&mut e), SCPT);
    assert_eq!(pending(&mut e), 1);
    assert!(crate::script_call::calls().is_empty());
}

/// The Pip-Boy (id 1): put away (mode 4, no callback) and its entry cleared; an id between 1 and
/// the first menu id: only the entry.
#[test]
fn the_pipboy_is_put_away() {
    let mut e = setup();
    menus(&mut e, &[7, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
    e.mem.set_u32(IM_AT + IM_PIPBOY_MODE, 2);
    e.mem.set_u32(IM_AT + IM_PIPBOY_MODE_CALLBACK, 0x1234);
    call_on(&mut e, CLOSE, 0, &[]);
    assert_eq!(e.mem.u32(IM_AT + IM_PIPBOY_MODE), 4);
    assert_eq!(e.mem.u32(IM_AT + IM_PIPBOY_MODE_CALLBACK), 0);
    assert_eq!(stack(&e)[..2], [7, 0]);
    e.mem.set_u32(IM_AT + IM_PIPBOY_MODE, 2);
    call_on(&mut e, CLOSE, 0, &[]);
    assert_eq!(stack(&e)[0], 0);
    assert_eq!(e.mem.u32(IM_AT + IM_PIPBOY_MODE), 2);
    assert!(log().is_empty());
}
