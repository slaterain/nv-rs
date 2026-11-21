//! `ShowTextInputMenu`, `SetTextInputExtendedProps` and `SetTextInputString` called by their
//! opcodes, JIP's text input routines through the menu's vtable slots, its close seam, and the font
//! heights. Expectations from JIP's bodies (functions_jip/jip_fn_ui.h: `ShowTextEditMenu`,
//! `Cmd_ShowTextInputMenu_Execute`, `Cmd_SetTextInputExtendedProps_Execute`,
//! `Cmd_SetTextInputString_Execute`; internal/hooks.h: `TextInputRefreshHook`, `HandleInputKey`,
//! `TextInputKeyPressHook`, `UnsetTextInputHooks`, `TextInputCloseHook`; internal/patches_game.h's
//! `DeferredInit` font loop), never from the port's output. The game functions are stand-ins
//! that record; tiles are built with their sorted values as the game keeps them.

use super::*;
use crate::commands::test_support::*;
use std::cell::{Cell, RefCell};

const SHOW: u32 = 0x274d;
const PROPS: u32 = 0x28a2;
const STRING: u32 = 0x28a3;
const CLOSE_ACTIVE: u32 = 0x28cc;
const AREA_AT: u32 = 0x0600_0000;
const SCPT: u32 = FORMS + 0x2_0000;
const MISC: u32 = FORMS + 0x2_2000;
const LIST: u32 = FORMS + 0x5_0000;
const MENU: u32 = FORMS + 0x6_0000;
const IM_AT: u32 = FORMS + 0x7_0000;
const OK_SETTING_TEXT: u32 = FORMS + 0x7_8000;
const CLICK_FN: u32 = 0x0dea_7000;
// The exe's and JIP's numbers, written here from JIP's headers and the exe, not taken from the
// module: `TextEditMenu::Get()` 0x11DAEC4, `kVtbl_TextEditMenu` 0x1070034 (click +0xC, frame
// +0x2C, key +0x30), the exe's frame 0x7E65F0, key 0x7E6620 and close 0x7E65B0;
// `g_interfaceManager` 0x11D8A80 (`menuStack` +0x114, `currentMode` +0xC, tile reader +0x9C,
// `activeTile` +0xCC, `activeMenu` +0xD0); `Interface::CloseConsole` 0x703DA0, `Tile::ReadFile`
// 0xA01B00, `AddToEnterStack` 0x714D90, `Menu::GetMaxDepth` 0xA1DFB0, `ADDR_TileSetFloat`
// 0xA012D0, `Tile::IsTrue` 0xA01230, `Menu::PrepForVisibility` 0xA1DC20, the value refresh
// 0xA09410; the clock 0x11F63A8, the OK setting's text 0x11D38BC, the font manager 0x11F33F8.
const TEXT_EDIT_MENU: u32 = 0x011d_aec4;
const VTABLE: u32 = 0x0107_0034;
const CLICK_SLOT: u32 = 0x0107_0040;
const FRAME_SLOT: u32 = 0x0107_0060;
const KEY_SLOT: u32 = 0x0107_0064;
const EXE_FRAME: u32 = 0x007e_65f0;
const EXE_KEY: u32 = 0x007e_6620;
const EXE_CLOSE: u32 = 0x007e_65b0;
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const CLOSE_CONSOLE: u32 = 0x0070_3da0;
const READ_FILE: u32 = 0x00a0_1b00;
const ENTER_STACK: u32 = 0x0071_4d90;
const MAX_DEPTH: u32 = 0x00a1_dfb0;
const SET_FLOAT_FN: u32 = 0x00a0_12d0;
const IS_TRUE_FN: u32 = 0x00a0_1230;
const PREP_FN: u32 = 0x00a1_dc20;
const REFRESH_FN: u32 = 0x00a0_9410;
const TICK: u32 = 0x011f_63a8;
const OK_TEXT: u32 = 0x011d_38bc;
const FONT_MANAGER: u32 = 0x011f_33f8;
const SEAM: u32 = 0x007e_66a0;
// `kTileValue_*` (nvse/GameTiles.h).
const VISIBLE: u32 = 0xfa3;
const DEPTH: u32 = 0xfad;
const TARGET: u32 = 0xfaf;
const FONT: u32 = 0xfb9;
const MOUSEOVER: u32 = 0xfc3;
const STRING_T: u32 = 0xfc4;
const USER0: u32 = 0x1004;
const USER1: u32 = 0x1005;
const USER2: u32 = 0x1006;
const USER3: u32 = 0x1007;
// `kInputCode_*` (nvse/GameOSDepend.h).
const BACKSPACE: u32 = 0x8000_0000;
const LEFT: u32 = 0x8000_0001;
const RIGHT: u32 = 0x8000_0002;
const UP: u32 = 0x8000_0003;
const DOWN: u32 = 0x8000_0004;
const HOME: u32 = 0x8000_0005;
const END: u32 = 0x8000_0006;
const DELETE: u32 = 0x8000_0007;
const ENTER: u32 = 0x8000_0008;
const PAGE_UP: u32 = 0x8000_0009;
const PAGE_DOWN: u32 = 0x8000_000a;

thread_local! {
    static NEXT: Cell<u32> = const { Cell::new(0) };
    /// The stand-ins' calls: the address and the words.
    static LOG: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
    /// What `Tile::ReadFile` answers, and what `Tile::IsTrue` answers.
    static ROOT_OF_FILE: Cell<u32> = const { Cell::new(0) };
    static IS_TRUE: Cell<u32> = const { Cell::new(1) };
}

fn log() -> Vec<(u32, Vec<u32>)> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

fn logged(addr: u32) -> Vec<Vec<u32>> {
    LOG.with(|l| {
        l.borrow()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    })
}

fn note(addr: u32, w: &[u32]) {
    LOG.with(|l| l.borrow_mut().push((addr, w.to_vec())));
}

/// A stand-in at `A` that notes its words.
fn noted<const A: u32>(_: &mut Engine, w: &[u32]) -> Ret {
    note(A, w);
    Ret::default()
}

fn alloc(size: u32) -> u32 {
    NEXT.with(|n| {
        let at = n.get();
        n.set(at + size.next_multiple_of(0x10));
        at
    })
}

/// A tile with a value for each trait in `ids` (kept sorted, as the game's tiles keep them) and
/// `parent`; the values' addresses in the order of `ids`.
fn tile(e: &mut Engine, ids: &[u32], parent: u32) -> (u32, Vec<u32>) {
    let at = alloc(0x40);
    e.mem.set_u32(at + 0x28, parent);
    let values: Vec<u32> = ids
        .iter()
        .map(|&id| {
            let v = alloc(0x10);
            e.mem.set_u32(v, id);
            v
        })
        .collect();
    let mut sorted: Vec<(u32, u32)> = ids.iter().copied().zip(values.iter().copied()).collect();
    sorted.sort();
    let data = alloc(4 * sorted.len() as u32 + 4);
    for (i, (_, v)) in sorted.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, *v);
    }
    e.mem.set_u32(at + 0x14, data);
    e.mem.set_u32(at + 0x18, sorted.len() as u32);
    (at, values)
}

fn num(e: &Engine, value: u32) -> f32 {
    e.mem.f32(value + 8)
}

fn text(e: &Engine, value: u32) -> Vec<u8> {
    nvse::table::guest_text(e, e.mem.u32(value + 0xc))
}

/// The menu's tiles and their values.
struct Tiles {
    root: u32,
    root_v: Vec<u32>,
    text_tile: u32,
    text_v: Vec<u32>,
    ok: u32,
    ok_v: Vec<u32>,
    title: u32,
    title_v: Vec<u32>,
    rect: u32,
    rect_v: Vec<u32>,
}

fn setup() -> (Engine, Tiles) {
    NEXT.with(|n| n.set(AREA_AT + 0x100));
    LOG.with(|l| l.borrow_mut().clear());
    ROOT_OF_FILE.with(|r| r.set(0));
    IS_TRUE.with(|t| t.set(1));
    let _ = crate::script_call::calls();
    let mut e = engine();
    e.map(AREA_AT, 0x10_0000);
    for page in [
        0x011d_3000,
        0x011d_8000,
        0x011d_a000,
        0x011f_3000,
        0x011f_6000,
        0x0107_0000,
    ] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(INTERFACE_MANAGER, IM_AT);
    e.mem.set_u32(IM_AT + 0x9c, 0x5eed);
    form(&mut e, SCPT, 0x11, 0x0000_0911);
    form(&mut e, MISC, 0x1f, 0x0000_0a1f);
    set_ref(&mut e, 1, SCPT);
    set_ref(&mut e, 3, MISC);
    nvse::lambda::insert_for_tests(&mut e, SCPT, LIST);
    e.mem.write(OK_SETTING_TEXT, b"Done\0");
    e.mem.set_u32(OK_TEXT, OK_SETTING_TEXT);
    // The vtable as the exe has it.
    e.mem.set_u32(CLICK_SLOT, CLICK_FN);
    e.mem.set_u32(FRAME_SLOT, EXE_FRAME);
    e.mem.set_u32(KEY_SLOT, EXE_KEY);
    e.register(CLOSE_CONSOLE, noted::<CLOSE_CONSOLE>);
    e.register(ENTER_STACK, noted::<ENTER_STACK>);
    e.register(SET_FLOAT_FN, noted::<SET_FLOAT_FN>);
    e.register(PREP_FN, noted::<PREP_FN>);
    e.register(EXE_CLOSE, noted::<EXE_CLOSE>);
    e.register(CLICK_FN, noted::<CLICK_FN>);
    e.register(REFRESH_FN, noted::<REFRESH_FN>);
    e.register(READ_FILE, |e, w| {
        note(READ_FILE, &[w[0]]);
        note(
            READ_FILE,
            &[0, u32::from(nvse::table::guest_text(e, w[1]) == JIP_FILE)],
        );
        Ret {
            eax: ROOT_OF_FILE.with(Cell::get),
            ..Ret::default()
        }
    });
    e.register(MAX_DEPTH, |_, w| {
        note(MAX_DEPTH, w);
        Ret {
            st0: 37.5,
            ..Ret::default()
        }
    });
    e.register(IS_TRUE_FN, |_, w| {
        note(IS_TRUE_FN, w);
        Ret {
            eax: IS_TRUE.with(Cell::get),
            ..Ret::default()
        }
    });
    // JIP's file's menu: the root tile (a `TileMenu` whose menu is MENU), the text tile in the
    // input rectangle, the OK button and the title.
    let (rect, rect_v) = tile(&mut e, &[USER1, USER2], 0);
    let (text_tile, text_v) = tile(&mut e, &[FONT, STRING_T, USER0, USER1], rect);
    let (ok, ok_v) = tile(&mut e, &[TARGET, STRING_T], 0);
    let (title, title_v) = tile(&mut e, &[VISIBLE, STRING_T], 0);
    let (root, root_v) = tile(&mut e, &[DEPTH, USER0, USER1, USER2, USER3], 0);
    e.mem.set_u32(root + 0x3c, MENU);
    e.mem.set_u32(MENU, VTABLE);
    e.mem.set_u32(MENU + 0x28, text_tile);
    e.mem.set_u32(MENU + 0x2c, ok);
    e.mem.set_u32(MENU + 0x30, title);
    e.mem.set_f32(rect_v[0] + 8, 14.0);
    ROOT_OF_FILE.with(|r| r.set(root));
    let t = Tiles {
        root,
        root_v,
        text_tile,
        text_v,
        ok,
        ok_v,
        title,
        title_v,
        rect,
        rect_v,
    };
    (e, t)
}

const JIP_FILE: &[u8] = b"Data\\NVSE\\plugins\\textinput\\texteditmenu.xml";

fn kept(e: &Engine) -> usize {
    nvse::lambda::saved_count(e, SCPT)
}

/// A `String` of the menu at `at`: its text, length and room.
fn string_of(e: &Engine, at: u32) -> (Vec<u8>, u16, u16) {
    let data = e.mem.u32(at);
    let len = e.mem.u16(at + 4);
    (e.mem.bytes(data, u32::from(len)), len, e.mem.u16(at + 6))
}

fn shown(e: &Engine) -> Vec<u8> {
    string_of(e, MENU + 0x3c).0
}

fn current(e: &Engine) -> Vec<u8> {
    string_of(e, MENU + 0x34).0
}

fn cursor(e: &Engine) -> u32 {
    e.mem.u32(MENU + 0x44)
}

/// Shows JIP's menu ("Name it", 300 by 200) and clears the logs.
fn shown_menu() -> (Engine, Tiles) {
    let (mut e, t) = setup();
    assert_eq!(
        call_on(&mut e, SHOW, 0, &[r(1), z(300.0), z(200.0), s(b"Name it")]).1,
        1.0
    );
    log();
    (e, t)
}

/// One key through the menu's key slot (as the game's interface hands it); the routine's answer.
fn key(e: &mut Engine, k: u32) -> u32 {
    e.vcall(MENU, 0x30, &args![k]).u32()
}

fn type_text(e: &mut Engine, text: &[u8]) {
    for &c in text {
        assert_eq!(key(e, u32::from(c)), 1);
    }
}

// ---- ShowTextInputMenu ----

/// JIP's `ShowTextEditMenu` step by step: the console closed, JIP's file read with the interface
/// manager's reader, the menu's tile and id set, put on the stack, kept as the text edit menu,
/// the root's depth through the game's setter, the size in `user0`/`user1`, the title, the text
/// tile's heights for its font, both texts made (the shown one a lone `|` before the refresh's
/// blink), the OK text from the game setting, cursor 0, lengths 1 to 0x7FFF, the rectangle the text
/// tile's parent, taking keys, the script kept with its lambda variables, prepared to show and the
/// hooks in. Result 1.
#[test]
fn the_menu_is_built_as_jips_show_text_edit_menu() {
    let (mut e, t) = setup();
    crate::commands::jip_fn_ui_text::deferred_init(&mut e);
    e.ext::<FontHeights>().0[3] = (11.0, 19.0);
    e.mem.set_f32(t.text_v[0] + 8, 3.0);
    // The blink's time is left from the menu's making: the clock is 100 ms past it, no blink.
    e.mem.set_u32(MENU + 0x50, 1000);
    e.mem.set_u32(TICK, 1100);
    e.mem.set_u32(t.title_v[0] + 8, 0x3f80_0000);
    assert_eq!(
        call_on(&mut e, SHOW, 0, &[r(1), z(300.0), z(200.0), s(b"Name it")]).1,
        1.0
    );
    assert_eq!(logged(READ_FILE), vec![vec![0x5eed], vec![0, 1]]);
    assert_eq!(logged(ENTER_STACK), vec![vec![IM_AT, 0x41b]]);
    assert_eq!(
        logged(SET_FLOAT_FN),
        vec![vec![t.root, DEPTH, 37.5f32.to_bits(), 0]]
    );
    assert_eq!(logged(PREP_FN), vec![vec![MENU, 0]]);
    let order: Vec<u32> = log()
        .into_iter()
        .map(|(a, _)| a)
        .filter(|&a| a != REFRESH_FN)
        .collect();
    assert_eq!(
        order,
        vec![
            CLOSE_CONSOLE,
            READ_FILE,
            READ_FILE,
            ENTER_STACK,
            MAX_DEPTH,
            SET_FLOAT_FN,
            PREP_FN
        ]
    );
    assert_eq!(e.mem.u32(MENU + 4), t.root);
    assert_eq!(e.mem.u32(MENU + 0x14), 0x41b);
    assert_eq!(e.mem.u32(TEXT_EDIT_MENU), MENU);
    assert_eq!(num(&e, t.root_v[1]), 300.0);
    assert_eq!(num(&e, t.root_v[2]), 200.0);
    assert_eq!(text(&e, t.title_v[1]), b"Name it");
    assert_eq!(
        e.mem.u32(t.title_v[0] + 8),
        0x3f80_0000,
        "the title stays visible"
    );
    assert_eq!((num(&e, t.text_v[2]), num(&e, t.text_v[3])), (11.0, 19.0));
    assert_eq!(string_of(&e, MENU + 0x34), (vec![], 0, 0x400));
    assert_eq!(string_of(&e, MENU + 0x3c), (b"|".to_vec(), 1, 0x400));
    assert_eq!(text(&e, t.ok_v[1]), b"Done");
    assert_eq!(cursor(&e), 0);
    assert_eq!(e.mem.u32(MENU + 0x48), 0x7fff_0001);
    assert_eq!(e.mem.u32(MENU + 0x4c), t.rect);
    assert_eq!(e.mem.u32(MENU + 0x54), 0x0000_0100);
    assert_eq!(e.mem.u32(MENU + 0x58), SCPT);
    assert_eq!(kept(&e), 1);
    assert_eq!(e.hook_users(Hook::TextInputClose), 1);
    assert_eq!(e.mem.u32(FRAME_SLOT), TEXT_INPUT_REFRESH);
    assert_eq!(e.mem.u32(KEY_SLOT), TEXT_INPUT_KEY_PRESS);
    assert_eq!(
        e.mem.u32(CLICK_SLOT),
        CLICK_FN,
        "the click slot is the exe's"
    );
    // The refresh: the empty text is short of 1 (OK off), the rectangle's user2 its user1.
    assert_eq!(e.mem.u32(t.ok_v[0] + 8), 0);
    assert_eq!(num(&e, t.rect_v[1]), 14.0);
    let _ = (t.text_tile, t.ok, t.title);
}

/// An empty title hides the title tile (and is still set); a longer title is cut at 80 bytes.
#[test]
fn an_empty_title_hides_and_a_long_one_is_cut() {
    let (mut e, t) = setup();
    e.mem.set_u32(t.title_v[0] + 8, 0x3f80_0000);
    call_on(&mut e, SHOW, 0, &[r(1), z(1.0), z(1.0), s(b"")]);
    assert_eq!(e.mem.u32(t.title_v[0] + 8), 0);
    assert_eq!(text(&e, t.title_v[1]), b"");
    assert_ne!(e.mem.u32(t.title_v[1] + 0xc), 0, "set to the empty text");
    let (mut e, t) = setup();
    let long: Vec<u8> = (0..100u8).map(|i| b'a' + i % 26).collect();
    call_on(&mut e, SHOW, 0, &[r(1), z(1.0), z(1.0), s(&long)]);
    assert_eq!(text(&e, t.title_v[1]), &long[..80]);
}

/// Not shown: a text edit menu open (nothing called), the callback not a script (nothing
/// called), JIP's file giving no menu (the console closed and the file read, then nothing: result
/// 0, no hooks).
#[test]
fn nothing_without_a_script_a_menu_or_with_one_open() {
    let (mut e, _) = setup();
    e.mem.set_u32(TEXT_EDIT_MENU, 0x7777);
    assert_eq!(
        call_on(&mut e, SHOW, 0, &[r(1), z(1.0), z(1.0), s(b"x")]).1,
        0.0
    );
    assert!(log().is_empty());
    let (mut e, _) = setup();
    assert_eq!(
        call_on(&mut e, SHOW, 0, &[r(3), z(1.0), z(1.0), s(b"x")]).1,
        0.0
    );
    assert!(log().is_empty());
    for no_menu in [true, false] {
        let (mut e, t) = setup();
        if no_menu {
            e.mem.set_u32(t.root + 0x3c, 0);
        } else {
            ROOT_OF_FILE.with(|r| r.set(0));
        }
        assert_eq!(
            call_on(&mut e, SHOW, 0, &[r(1), z(1.0), z(1.0), s(b"x")]).1,
            0.0
        );
        let order: Vec<u32> = log().into_iter().map(|(a, _)| a).collect();
        assert_eq!(order, vec![CLOSE_CONSOLE, READ_FILE, READ_FILE]);
        assert_eq!(e.mem.u32(TEXT_EDIT_MENU), 0);
        assert_eq!(e.hook_users(Hook::TextInputClose), 0);
        assert_eq!(e.mem.u32(KEY_SLOT), EXE_KEY);
        assert_eq!(kept(&e), 0);
    }
}

/// Another menu active: its tile's mouse-over is taken off and both the active tile and menu
/// cleared; with this menu active: left as they are.
#[test]
fn the_active_tile_loses_its_mouseover() {
    let (mut e, _) = setup();
    let (active, active_v) = tile(&mut e, &[MOUSEOVER], 0);
    e.mem.set_u32(active_v[0] + 8, 0x3f80_0000);
    e.mem.set_u32(IM_AT + 0xcc, active);
    e.mem.set_u32(IM_AT + 0xd0, 0x4321);
    call_on(&mut e, SHOW, 0, &[r(1), z(1.0), z(1.0), s(b"x")]);
    assert_eq!(e.mem.u32(active_v[0] + 8), 0);
    assert_eq!((e.mem.u32(IM_AT + 0xcc), e.mem.u32(IM_AT + 0xd0)), (0, 0));
    let (mut e, _) = setup();
    let (active, active_v) = tile(&mut e, &[MOUSEOVER], 0);
    e.mem.set_u32(active_v[0] + 8, 0x3f80_0000);
    e.mem.set_u32(IM_AT + 0xcc, active);
    e.mem.set_u32(IM_AT + 0xd0, MENU);
    call_on(&mut e, SHOW, 0, &[r(1), z(1.0), z(1.0), s(b"x")]);
    assert_eq!(e.mem.u32(active_v[0] + 8), 0x3f80_0000);
    assert_eq!(
        (e.mem.u32(IM_AT + 0xcc), e.mem.u32(IM_AT + 0xd0)),
        (active, MENU)
    );
}

/// The text tile's font is rounded to the nearest (2.5 to 2, 2.6 to 3) for its heights; a font
/// past the table gives 0 for both.
#[test]
fn the_font_number_is_rounded_for_its_heights() {
    for (font, want) in [(2.5f32, (5.0, 6.0)), (2.6, (7.0, 8.0)), (300.0, (0.0, 0.0))] {
        let (mut e, t) = setup();
        e.ext::<FontHeights>().0[2] = (5.0, 6.0);
        e.ext::<FontHeights>().0[3] = (7.0, 8.0);
        e.mem.set_f32(t.text_v[0] + 8, font);
        e.mem.set_f32(t.text_v[2] + 8, 99.0);
        call_on(&mut e, SHOW, 0, &[r(1), z(1.0), z(1.0), s(b"x")]);
        assert_eq!((num(&e, t.text_v[2]), num(&e, t.text_v[3])), want, "{font}");
    }
}

// ---- the font heights ----

/// JIP's DeferredInit loop: font n from the manager's (n - 1)th `FontInfo`: the space's height
/// and the line height, 4, 6, 4 and 2 more line height for fonts 2, 3, 6 and 7; with the game's
/// manager only fonts 1 to 8; a font with no info is left at 0.
#[test]
fn the_font_heights_come_from_the_font_manager() {
    let (mut e, _) = setup();
    let manager = alloc(0x200);
    e.mem.set_u32(FONT_MANAGER, manager);
    for n in 1..=12u32 {
        if n == 5 {
            continue;
        }
        let info = alloc(0x54);
        let data = alloc(0x4000);
        e.mem.set_u32(info + 0x38, data);
        e.mem.set_f32(data, 10.0 * n as f32);
        e.mem.set_f32(data + 0x128 + 0x20 * 0x38 + 0x28, n as f32);
        e.mem.set_u32(manager + 4 * (n - 1), info);
    }
    deferred_init(&mut e);
    let h = e.ext::<FontHeights>().0.clone();
    assert_eq!(h.len(), 90);
    assert_eq!(h[1], (1.0, 10.0));
    assert_eq!(h[2], (2.0, 24.0));
    assert_eq!(h[3], (3.0, 36.0));
    assert_eq!(h[4], (4.0, 40.0));
    assert_eq!(h[5], (0.0, 0.0));
    assert_eq!(h[6], (6.0, 64.0));
    assert_eq!(h[7], (7.0, 72.0));
    assert_eq!(h[8], (8.0, 80.0));
    assert_eq!(h[0], (0.0, 0.0));
    assert!(h[9..].iter().all(|&x| x == (0.0, 0.0)));
}

// ---- the keys ----

/// Characters go in at the cursor; the shown text is the text with the cursor byte (`0x7F` while
/// the cursor's state is off, `|` while on) at the cursor, in the text tile; the routine always
/// answers 1.
#[test]
fn typing_puts_characters_in_at_the_cursor() {
    let (mut e, t) = shown_menu();
    type_text(&mut e, b"ab");
    assert_eq!((current(&e), cursor(&e)), (b"ab".to_vec(), 2));
    assert_eq!(shown(&e), b"ab\x7f");
    assert_eq!(text(&e, t.text_v[1]), b"ab\x7f");
    e.mem.set_u8(MENU + 0x54, 1);
    assert_eq!(key(&mut e, LEFT), 1);
    assert_eq!(shown(&e), b"a|b");
    type_text(&mut e, b"X");
    assert_eq!((current(&e), cursor(&e)), (b"aXb".to_vec(), 2));
    // The OK button: on once the text has the least length.
    assert_eq!(e.mem.u32(t.ok_v[0] + 8), 0x3f80_0000);
    // A key below 0x20, and the arrows up and down: nothing, still answered.
    for k in [0x1f, UP, DOWN, 0x8000_000b] {
        assert_eq!(key(&mut e, k), 1);
        assert_eq!((current(&e), cursor(&e)), (b"aXb".to_vec(), 2));
    }
}

/// The lengths: no character and no new line at the most; the OK button off below the least.
#[test]
fn the_most_and_least_lengths() {
    let (mut e, t) = shown_menu();
    e.mem.set_u16(MENU + 0x4a, 2);
    e.mem.set_u16(MENU + 0x48, 3);
    type_text(&mut e, b"abc");
    assert_eq!(current(&e), b"ab");
    key(&mut e, ENTER);
    assert_eq!(current(&e), b"ab");
    assert_eq!(e.mem.u32(t.ok_v[0] + 8), 0, "2 is short of 3");
    e.mem.set_u16(MENU + 0x4a, 3);
    key(&mut e, ENTER);
    assert_eq!((current(&e), cursor(&e)), (b"ab\n".to_vec(), 3));
    assert_eq!(e.mem.u32(t.ok_v[0] + 8), 0x3f80_0000);
}

/// Backspace, Delete, the arrows, Page Up and Down at their ends and in the middle.
#[test]
fn editing_keys_move_and_erase() {
    let (mut e, _) = shown_menu();
    assert_eq!(key(&mut e, BACKSPACE), 1);
    assert_eq!(key(&mut e, LEFT), 1);
    assert_eq!(key(&mut e, PAGE_UP), 1);
    assert_eq!(cursor(&e), 0);
    type_text(&mut e, b"abcdefgh");
    assert_eq!(key(&mut e, RIGHT), 1);
    assert_eq!(key(&mut e, DELETE), 1);
    assert_eq!(key(&mut e, PAGE_DOWN), 1);
    assert_eq!((current(&e), cursor(&e)), (b"abcdefgh".to_vec(), 8));
    key(&mut e, BACKSPACE);
    assert_eq!((current(&e), cursor(&e)), (b"abcdefg".to_vec(), 7));
    key(&mut e, PAGE_UP);
    assert_eq!(cursor(&e), 2);
    key(&mut e, PAGE_UP);
    assert_eq!(cursor(&e), 0);
    key(&mut e, PAGE_DOWN);
    assert_eq!(cursor(&e), 5);
    key(&mut e, PAGE_DOWN);
    assert_eq!(cursor(&e), 7);
    key(&mut e, LEFT);
    key(&mut e, LEFT);
    key(&mut e, DELETE);
    assert_eq!((current(&e), cursor(&e)), (b"abcdeg".to_vec(), 5));
    assert_eq!(shown(&e), b"abcde\x7fg");
    key(&mut e, RIGHT);
    assert_eq!(cursor(&e), 6);
    key(&mut e, LEFT);
    key(&mut e, BACKSPACE);
    assert_eq!((current(&e), cursor(&e)), (b"abcdg".to_vec(), 4));
    // Erasing the last byte, and the only one.
    key(&mut e, RIGHT);
    key(&mut e, BACKSPACE);
    assert_eq!(current(&e), b"abcd");
    let (mut e, _) = shown_menu();
    type_text(&mut e, b"q");
    key(&mut e, BACKSPACE);
    assert_eq!((current(&e), cursor(&e)), (vec![], 0));
    assert_eq!(e.mem.u8(e.mem.u32(MENU + 0x34)), 0);
}

/// Home and End stay within the cursor's line: Home after the last new line at or before the
/// cursor, End at the next new line or the text's end.
#[test]
fn home_and_end_stay_in_the_line() {
    let (mut e, _) = shown_menu();
    type_text(&mut e, b"ab");
    key(&mut e, ENTER);
    type_text(&mut e, b"cde");
    key(&mut e, ENTER);
    type_text(&mut e, b"f");
    assert_eq!(current(&e), b"ab\ncde\nf");
    for _ in 0..3 {
        key(&mut e, LEFT);
    }
    assert_eq!(cursor(&e), 5);
    key(&mut e, HOME);
    assert_eq!(cursor(&e), 3);
    key(&mut e, HOME);
    assert_eq!(cursor(&e), 3);
    key(&mut e, END);
    assert_eq!(cursor(&e), 6);
    key(&mut e, END);
    assert_eq!(cursor(&e), 6);
    key(&mut e, RIGHT);
    key(&mut e, END);
    assert_eq!(cursor(&e), 8);
    key(&mut e, HOME);
    assert_eq!(cursor(&e), 7);
    // On a new line itself: Home goes after it (JIP's search takes the cursor's byte).
    for _ in 0..5 {
        key(&mut e, LEFT);
    }
    assert_eq!(cursor(&e), 2);
    key(&mut e, HOME);
    assert_eq!(cursor(&e), 3);
    for _ in 0..3 {
        key(&mut e, LEFT);
    }
    key(&mut e, HOME);
    assert_eq!(cursor(&e), 0);
    key(&mut e, END);
    assert_eq!(cursor(&e), 2);
}

/// JIP's numbers flag (1): a `-` only first and not twice, one `.`, digits; nothing else.
#[test]
fn the_numbers_flag_takes_numbers_only() {
    let (mut e, _) = shown_menu();
    e.mem.set_u8(MENU + 0x57, 1);
    type_text(&mut e, b"-1-a.2.x3");
    assert_eq!(current(&e), b"-1.23");
    key(&mut e, HOME);
    type_text(&mut e, b"-");
    assert_eq!(current(&e), b"-1.23");
    let (mut e, _) = shown_menu();
    e.mem.set_u8(MENU + 0x57, 1);
    type_text(&mut e, b"5");
    key(&mut e, HOME);
    type_text(&mut e, b"-");
    assert_eq!(current(&e), b"-5");
    key(&mut e, HOME);
    type_text(&mut e, b"-");
    assert_eq!(current(&e), b"-5", "the first byte is a - already");
    // A `-` past the first place: refused even when the text has none.
    let (mut e, _) = shown_menu();
    e.mem.set_u8(MENU + 0x57, 1);
    type_text(&mut e, b"5-");
    assert_eq!(current(&e), b"5", "a - only first");
}

/// Enter with JIP's Enter flag (2): when the OK button's `target` is true, the menu's click slot
/// with button 1 and the OK button; nothing typed either way.
#[test]
fn enter_with_the_flag_clicks_ok() {
    let (mut e, t) = shown_menu();
    e.mem.set_u8(MENU + 0x57, 2);
    type_text(&mut e, b"x");
    log();
    assert_eq!(key(&mut e, ENTER), 1);
    assert_eq!(logged(IS_TRUE_FN), vec![vec![t.ok, TARGET]]);
    assert_eq!(logged(CLICK_FN), vec![vec![MENU, 1, t.ok]]);
    assert_eq!(current(&e), b"x");
    IS_TRUE.with(|i| i.set(0));
    log();
    assert_eq!(key(&mut e, ENTER), 1);
    assert!(logged(CLICK_FN).is_empty());
    assert_eq!(current(&e), b"x");
}

/// Not taking keys (+0x55 is 0): nothing changes.
#[test]
fn no_keys_while_inactive() {
    let (mut e, _) = shown_menu();
    e.mem.set_u8(MENU + 0x55, 0);
    assert_eq!(key(&mut e, u32::from(b'a')), 1);
    assert_eq!(key(&mut e, ENTER), 1);
    assert_eq!((current(&e), cursor(&e)), (vec![], 0));
}

// ---- the frame ----

/// The frame slot's routine: past 500 ms since the last blink (unsigned, so a clock behind it
/// counts as past), the cursor's state flips and its byte in the shown text becomes `0x7F` (on)
/// or `|` (off); at 500 nothing blinks. While not taking keys: nothing at all.
#[test]
fn the_cursor_blinks_every_500_ms() {
    let (mut e, t) = shown_menu();
    type_text(&mut e, b"ab");
    key(&mut e, LEFT);
    assert_eq!(shown(&e), b"a\x7fb");
    e.mem.set_u8(MENU + 0x54, 1);
    e.mem.set_u32(MENU + 0x50, 1000);
    e.mem.set_u32(TICK, 1500);
    e.vcall(MENU, 0x2c, &args![]);
    assert_eq!(e.mem.u8(MENU + 0x54), 1);
    assert_eq!(e.mem.u32(MENU + 0x50), 1000);
    e.mem.set_u32(TICK, 1501);
    e.vcall(MENU, 0x2c, &args![]);
    assert_eq!(e.mem.u8(MENU + 0x54), 0);
    assert_eq!(e.mem.u32(MENU + 0x50), 1501);
    assert_eq!(shown(&e), b"a|b");
    assert_eq!(text(&e, t.text_v[1]), b"a|b");
    e.mem.set_u32(TICK, 1000);
    e.vcall(MENU, 0x2c, &args![]);
    assert_eq!(e.mem.u8(MENU + 0x54), 1);
    assert_eq!(shown(&e), b"a\x7fb");
    assert_eq!(text(&e, t.text_v[1]), b"a\x7fb");
    e.mem.set_f32(t.rect_v[0] + 8, 3.0);
    e.mem.set_u8(MENU + 0x55, 0);
    e.mem.set_u32(TICK, 9000);
    e.vcall(MENU, 0x2c, &args![]);
    assert_eq!(e.mem.u8(MENU + 0x54), 1);
    assert_ne!(num(&e, t.rect_v[1]), 3.0);
    e.mem.set_u8(MENU + 0x55, 1);
    e.vcall(MENU, 0x2c, &args![]);
    assert_eq!(num(&e, t.rect_v[1]), 3.0);
}

// ---- SetTextInputExtendedProps, SetTextInputString ----

/// On JIP's menu: the position above 0 in the menu tile's `user2`/`user3`, the least length, the
/// most (`& 0x7FFF`, 0x7FFF for 0 or none) and the flags (`& 3`).
#[test]
fn extended_props_set_the_lengths_and_flags() {
    let (mut e, t) = shown_menu();
    call_on(&mut e, PROPS, 0, &[z(40.0), z(-1.0), n(4), n(0x8005), n(7)]);
    assert_eq!(num(&e, t.root_v[3]), 40.0);
    assert_eq!(num(&e, t.root_v[4]), 0.0);
    assert_eq!(e.mem.u16(MENU + 0x48), 4);
    assert_eq!(e.mem.u16(MENU + 0x4a), 5);
    assert_eq!(e.mem.u8(MENU + 0x57), 3);
    call_on(&mut e, PROPS, 0, &[z(0.0), z(9.0)]);
    assert_eq!(num(&e, t.root_v[3]), 40.0);
    assert_eq!(num(&e, t.root_v[4]), 9.0);
    assert_eq!(e.mem.u16(MENU + 0x48), 1);
    assert_eq!(e.mem.u16(MENU + 0x4a), 0x7fff);
    assert_eq!(e.mem.u8(MENU + 0x57), 0);
    call_on(&mut e, PROPS, 0, &[z(0.0), z(0.0), n(0x1_0002), n(0)]);
    assert_eq!(e.mem.u16(MENU + 0x48), 2, "16 bits");
}

/// The text set, the cursor at 0 and the End key handed to the menu: the cursor at the first
/// line's end, the shown text made from the text.
#[test]
fn set_text_input_string_sets_the_text() {
    let (mut e, t) = shown_menu();
    e.mem.set_u8(MENU + 0x54, 1);
    call_on(&mut e, STRING, 0, &[s(b"hello\nworld")]);
    assert_eq!(current(&e), b"hello\nworld");
    assert_eq!(cursor(&e), 5);
    assert_eq!(shown(&e), b"hello|\nworld");
    assert_eq!(text(&e, t.text_v[1]), b"hello|\nworld");
    // The cursor goes back to 0 first: the End key from there.
    call_on(&mut e, STRING, 0, &[s(b"abc\ndef")]);
    assert_eq!((current(&e), cursor(&e)), (b"abc\ndef".to_vec(), 3));
    call_on(&mut e, STRING, 0, &[s(b"")]);
    assert_eq!((current(&e), cursor(&e)), (vec![], 0));
}

/// A text that fills its room gets one byte more room for a character typed (JIP's
/// `InsertChar`), its text kept.
#[test]
fn a_full_text_grows_by_one() {
    let (mut e, _) = shown_menu();
    let long = vec![b'x'; 0x401];
    call_on(&mut e, STRING, 0, &[s(&long)]);
    assert_eq!(string_of(&e, MENU + 0x34).2, 0x401);
    assert_eq!(cursor(&e), 0x401);
    type_text(&mut e, b"a");
    let (text, len, room) = string_of(&e, MENU + 0x34);
    assert_eq!((len, room), (0x402, 0x402));
    assert_eq!(&text[..0x401], &long[..]);
    assert_eq!(text[0x401], b'a');
}

/// Without JIP's hooks in (no menu shown by JIP, or its hooks out), or with no menu: both
/// setters do nothing.
#[test]
fn the_setters_need_jips_menu() {
    let (mut e, t) = setup();
    e.mem.set_u32(TEXT_EDIT_MENU, MENU);
    call_on(&mut e, PROPS, 0, &[z(40.0), z(40.0), n(4)]);
    call_on(&mut e, STRING, 0, &[s(b"hello")]);
    assert_eq!(num(&e, t.root_v[3]), 0.0);
    assert_eq!(e.mem.u16(MENU + 0x48), 0);
    assert_eq!(e.mem.u32(MENU + 0x34), 0);
    let (mut e, t) = shown_menu();
    e.mem.set_u32(TEXT_EDIT_MENU, 0);
    call_on(&mut e, PROPS, 0, &[z(40.0), z(40.0), n(4)]);
    call_on(&mut e, STRING, 0, &[s(b"hello")]);
    assert_eq!(num(&e, t.root_v[3]), 0.0);
    assert_eq!(current(&e), b"");
}

// ---- closing ----

/// The click seam with button 1: the menu stops taking keys, the script is called with the
/// text's address, its lambda variables given up, the exe's frame and key functions back, the hook
/// off, then the exe's close. Answered, so the exe's click is not run.
#[test]
fn ok_calls_the_script_with_the_text_and_takes_the_hooks_out() {
    let (mut e, _) = shown_menu();
    type_text(&mut e, b"Bob");
    let _ = crate::script_call::calls();
    log();
    let data = e.mem.u32(MENU + 0x34);
    assert_eq!(e.site(SEAM, &[MENU, 1, 0x1234]), Some(0));
    assert_eq!(crate::script_call::calls(), vec![(SCPT, 0, vec![data])]);
    assert_eq!(e.mem.u8(MENU + 0x55), 0);
    assert_eq!(kept(&e), 0);
    assert_eq!(e.mem.u32(FRAME_SLOT), EXE_FRAME);
    assert_eq!(e.mem.u32(KEY_SLOT), EXE_KEY);
    assert_eq!(e.hook_users(Hook::TextInputClose), 0);
    assert_eq!(logged(EXE_CLOSE), vec![Vec::<u32>::new()]);
}

/// A text without a buffer: the exe's empty text; another button: nothing, still answered.
#[test]
fn other_buttons_do_nothing_and_no_text_is_empty() {
    let (mut e, _) = shown_menu();
    let _ = crate::script_call::calls();
    assert_eq!(e.site(SEAM, &[MENU, 2, 0]), Some(0));
    assert!(crate::script_call::calls().is_empty());
    assert_eq!(e.hook_users(Hook::TextInputClose), 1);
    assert_eq!(e.mem.u8(MENU + 0x55), 1);
    assert!(logged(EXE_CLOSE).is_empty());
    e.mem.set_u32(MENU + 0x34, 0);
    e.site(SEAM, &[MENU, 1, 0]);
    assert_eq!(
        crate::script_call::calls(),
        vec![(SCPT, 0, vec![crate::commands::test_support::EMPTY_TEXT])]
    );
}

/// `CloseActiveMenu` on the text edit menu with JIP's hooks in: they come out (the script's
/// lambda variables given up, the exe's functions back, the hook off); without them nothing more.
#[test]
fn close_active_menu_takes_the_hooks_out() {
    for jips in [true, false] {
        let (mut e, _) = if jips { shown_menu() } else { setup() };
        e.mem.set_u32(IM_AT + 0xc, 2);
        e.mem.set_u32(IM_AT + 0x114, 0x41b);
        if !jips {
            e.mem.set_u32(TEXT_EDIT_MENU, MENU);
            e.mem.set_u32(MENU + 0x58, SCPT);
            nvse::lambda::save_variables(&mut e, SCPT);
        }
        call_on(&mut e, CLOSE_ACTIVE, 0, &[]);
        assert_eq!(e.mem.u32(IM_AT + 0x114), 0);
        assert_eq!(e.hook_users(Hook::TextInputClose), 0);
        assert_eq!(e.mem.u32(KEY_SLOT), EXE_KEY);
        assert_eq!(e.mem.u32(FRAME_SLOT), EXE_FRAME);
        assert_eq!(kept(&e), usize::from(!jips), "{jips}");
    }
}

// ---- review of #823: a menu closed another way, a 65,535-byte text ----

/// A game text edit menu of its own (not from JIP's file), at `at`, with its own tile.
fn game_menu(e: &mut Engine, at: u32) {
    let (tile, _) = tile(e, &[VISIBLE], 0);
    e.mem.set_u32(at, VTABLE);
    e.mem.set_u32(at + 4, tile);
    e.mem.set_u32(TEXT_EDIT_MENU, at);
}

/// JIP's menu shown, then closed without OK or `CloseActiveMenu` (the singleton cleared, as Esc or
/// a load leaves it), then a game text edit menu opened: at `elsewhere`, or at JIP's freed address
/// (its tile then another).
fn closed_another_way(guard: bool, elsewhere: bool) -> (Engine, u32) {
    let (mut e, _) = shown_menu();
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::NULL_GUARD, guard);
    e.register(EXE_KEY, noted::<EXE_KEY>);
    e.register(EXE_FRAME, noted::<EXE_FRAME>);
    e.mem.set_u32(TEXT_EDIT_MENU, 0);
    let menu = if elsewhere { alloc(0x60) } else { MENU };
    game_menu(&mut e, menu);
    let _ = crate::script_call::calls();
    log();
    (e, menu)
}

/// Review of #823 (a JIP slip not copied, the missing-data guard row): with JIP's menu closed
/// another way, the game's own text edit menu gets the exe's key and frame functions (the words
/// handed on), JIP's hooks come out (the exe's functions back in the slots, JIP's script's lambda
/// variables given up, the hook off) and JIP's script is not called; the same when the game's menu
/// is at JIP's old address. A click on its OK is not answered: the exe's click runs (it stores the
/// text, as without JIP), and JIP's close does not.
#[test]
fn a_game_menu_after_jips_closed_another_way_gets_the_exes_functions() {
    for elsewhere in [true, false] {
        let (mut e, menu) = closed_another_way(true, elsewhere);
        assert_eq!(e.vcall(menu, 0x30, &args![u32::from(b'a')]).u32(), 0);
        assert_eq!(logged(EXE_KEY), vec![vec![menu, u32::from(b'a')]]);
        assert_eq!(e.mem.u32(KEY_SLOT), EXE_KEY);
        assert_eq!(e.mem.u32(FRAME_SLOT), EXE_FRAME);
        assert_eq!(e.hook_users(Hook::TextInputClose), 0);
        assert_eq!(kept(&e), 0, "JIP's script's variables given up");
        assert!(crate::script_call::calls().is_empty());

        let (mut e, menu) = closed_another_way(true, elsewhere);
        e.vcall(menu, 0x2c, &args![]);
        assert_eq!(logged(EXE_FRAME), vec![vec![menu]]);
        assert_eq!(e.hook_users(Hook::TextInputClose), 0);

        let (mut e, menu) = closed_another_way(true, elsewhere);
        assert_eq!(e.site(SEAM, &[menu, 1, 0]), None, "the exe's click runs");
        assert!(crate::script_call::calls().is_empty());
        assert!(logged(EXE_CLOSE).is_empty());
        assert_eq!(e.hook_users(Hook::TextInputClose), 0);
        assert_eq!(e.mem.u32(KEY_SLOT), EXE_KEY);
    }
}

/// The setters on a game menu after JIP's closed another way: nothing set, the hooks out (row on).
#[test]
fn the_setters_leave_a_game_menu_alone() {
    let (mut e, menu) = closed_another_way(true, true);
    call_on(&mut e, STRING, 0, &[s(b"hello")]);
    call_on(&mut e, PROPS, 0, &[z(40.0), z(40.0), n(4)]);
    assert_eq!(e.mem.u32(menu + 0x34), 0);
    assert_eq!(e.mem.u16(menu + 0x48), 0);
    assert_eq!(e.hook_users(Hook::TextInputClose), 0);
}

/// Vanilla (the row off): as JIP, its routines serve the game's menu and the hooks stay in.
#[test]
fn without_the_row_jips_routines_stay_in() {
    let (mut e, menu) = closed_another_way(false, true);
    assert_eq!(e.vcall(menu, 0x30, &args![u32::from(b'a')]).u32(), 1);
    assert!(logged(EXE_KEY).is_empty());
    assert_eq!(e.hook_users(Hook::TextInputClose), 1);
    assert_eq!(e.mem.u32(KEY_SLOT), TEXT_INPUT_KEY_PRESS);
    assert_eq!(e.site(SEAM, &[menu, 2, 0]), Some(0), "JIP's close answers");
}

/// With the row on, JIP's own menu still runs JIP's routines.
#[test]
fn with_the_row_jips_own_menu_keeps_its_routines() {
    let (mut e, _) = shown_menu();
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::NULL_GUARD, true);
    type_text(&mut e, b"Bob");
    assert_eq!(current(&e), b"Bob");
    assert_eq!(e.hook_users(Hook::TextInputClose), 1);
    let _ = crate::script_call::calls();
    assert_eq!(e.site(SEAM, &[MENU, 1, 0]), Some(0));
    assert_eq!(crate::script_call::calls().len(), 1);
}

/// Review of #823, with the size caps row (the fixes preset): a 65,535-byte `SetTextInputString` keeps 65,534 bytes, so the shown text with
/// its cursor is 65,535 and its room never below its length; a string at 65,535 bytes takes no
/// more (JIP's 16-bit length and room would wrap to 0 and the byte go past its block).
#[test]
fn a_65535_byte_text_never_wraps_the_shown_texts_room() {
    let (mut e, _) = shown_menu();
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::SIZE_CAPS, true);
    let long = vec![b'x'; 0xffff];
    // The command's text as it hands it on (a compiled line this long is past the tests' script
    // page).
    set_text(&mut e, MENU, &long);
    let (_, len, room) = string_of(&e, MENU + 0x34);
    assert_eq!((len, room), (0xfffe, 0xfffe));
    let (shown_text, len, room) = string_of(&e, MENU + 0x3c);
    assert_eq!(len, 0xffff);
    assert!(room >= len);
    assert!(
        matches!(shown_text[0xfffe], b'|' | 0x7f),
        "the cursor's byte last"
    );
    // A string full at 65,535: nothing goes in.
    let data = e.mem.u32(MENU + 0x3c);
    string_insert_char(&mut e, MENU + 0x3c, b'y', 0);
    assert_eq!(string_of(&e, MENU + 0x3c).1, 0xffff);
    assert_eq!(string_of(&e, MENU + 0x3c).2, room);
    assert_eq!(e.mem.u32(MENU + 0x3c), data, "no new block");
    assert_eq!(e.mem.u8(data), b'x');
}

/// Review of #830: without the size caps row (vanilla), as JIP. A 70,000-byte
/// `SetTextInputString` keeps its length's low 16 bits, 4,464 bytes (JIP's `String::Set`), with
/// no 65,534 cap.
#[test]
fn without_the_size_caps_row_a_long_text_keeps_jips_16_bits() {
    let (mut e, _) = shown_menu();
    let long: Vec<u8> = (0..70_000u32).map(|i| b'a' + (i % 26) as u8).collect();
    set_text(&mut e, MENU, &long);
    let (text, len, _) = string_of(&e, MENU + 0x34);
    assert_eq!(len, 4_464);
    assert_eq!(text, &long[..4_464]);
}
