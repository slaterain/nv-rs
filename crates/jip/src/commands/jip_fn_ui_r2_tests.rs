//! `MessageBoxExAlt` (with its answer routine, JIP's `MessageBoxCallback`) and `SetFontFile`,
//! called by their opcodes. Expectations from JIP's bodies (functions_jip/jip_fn_ui.h:
//! `Cmd_MessageBoxExAlt_Execute`, `MessageBoxExAlt`, `MessageBoxCallback`,
//! `Cmd_SetFontFile_Execute`; internal/patches_game.h: `s_messageBoxScripts`,
//! `s_fontInfosMap`) and the game's `ShowMessageBox` (`00703E80`, its parameters as the exe
//! reads them), never from the port's output. The game functions are stand-ins that record.

use super::*;
use crate::commands::test_support::*;
use std::cell::RefCell;

const IM_AT: u32 = FORMS + 0x1_0000;
const SCPT: u32 = FORMS + 0x2_0000;
const SCPT2: u32 = FORMS + 0x2_1000;
const MISC: u32 = FORMS + 0x2_2000;
const MANAGER: u32 = FORMS + 0x3_0000;
const SHOW_FN: u32 = 0x0070_3e80;

/// A `FontInfo::Init` call: the block, the id, the path's text, the bool.
type Init = (u32, u32, Vec<u8>, u32);

thread_local! {
    /// Each box shown: its message, title (None for 0), callback and buttons' texts.
    static SHOWN: RefCell<Vec<Shown>> = const { RefCell::new(Vec::new()) };
    /// What G55's two width seams answered at each `ShowMessageBox` call (as `LoadNextMessage`,
    /// run inside it when no message menu is open, reads them).
    static WIDTHS_AT_SHOW: RefCell<Vec<[Option<u32>; 2]>> = const { RefCell::new(Vec::new()) };
    /// The `FontInfo::Init` calls: the block, the id, the path's text, the bool.
    static INITS: RefCell<Vec<Init>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Debug, PartialEq)]
struct Shown {
    message: Vec<u8>,
    title: Option<Vec<u8>>,
    callback: u32,
    fixed: [u32; 4],
    buttons: Vec<Vec<u8>>,
}

fn shown() -> Vec<Shown> {
    SHOWN.with(|s| std::mem::take(&mut *s.borrow_mut()))
}

fn setup() -> Engine {
    SHOWN.with(|s| s.borrow_mut().clear());
    INITS.with(|s| s.borrow_mut().clear());
    WIDTHS_AT_SHOW.with(|s| s.borrow_mut().clear());
    let _ = crate::script_call::calls();
    files::fresh();
    let mut e = engine();
    for page in [0x011d_8000, 0x011f_3000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(INTERFACE_MANAGER, IM_AT);
    form(&mut e, SCPT, 0x11, 0x0000_0911);
    form(&mut e, SCPT2, 0x11, 0x0000_0912);
    form(&mut e, MISC, 0x1f, 0x0000_0a1f);
    set_ref(&mut e, 1, SCPT);
    set_ref(&mut e, 2, SCPT2);
    set_ref(&mut e, 3, MISC);
    assert_ne!(
        crate::test_support::execute_of(&e, 0x274c),
        0,
        "SetFontFile is registered with G129"
    );
    // `ShowMessageBox`: the stack's words as the exe reads them (eight, then the buttons up to
    // the 0).
    e.register(SHOW_FN, |e, w| {
        let mut buttons = Vec::new();
        let mut i = 8;
        while w[i] != 0 {
            buttons.push(nvse::table::guest_text(e, w[i]));
            i += 1;
        }
        let s = Shown {
            message: nvse::table::guest_text(e, w[0]),
            title: (w[2] != 0).then(|| nvse::table::guest_text(e, w[2])),
            callback: w[3],
            fixed: [w[1], w[4], w[5], w[6] | w[7]],
            buttons,
        };
        SHOWN.with(|v| v.borrow_mut().push(s));
        let widths =
            crate::game_patches::ga::MAX_MESSAGE_WIDTH.map(|site| e.site(site.addr(), &[0, 0]));
        WIDTHS_AT_SHOW.with(|v| v.borrow_mut().push(widths));
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    // `FontInfo::Init`: the data loaded (+0x38 set) when the bool asks for the load (`Font::Font`
    // calls `Font::Load` only then) and the path does not name "bad".
    e.register(0x00a1_2020, |e, w| {
        let path = nvse::table::guest_text(e, w[2]);
        let loaded = w[3] != 0 && !path.windows(3).any(|p| p == b"bad");
        e.mem
            .set_u32(w[0] + 0x38, if loaded { 0x0b0b_0000 } else { 0 });
        INITS.with(|s| s.borrow_mut().push((w[0], w[1], path, w[3])));
        Ret {
            eax: w[0],
            ..Ret::default()
        }
    });
    e
}

fn b(t: &[u8]) -> Vec<u8> {
    t.to_vec()
}

// ---- the text ----

#[test]
fn box_text_cuts_buttons_and_the_title() {
    let cut = box_text(b"Message|Yes|No");
    assert_eq!(cut.bytes, b"Message\0Yes\0No\0");
    assert_eq!((cut.title, cut.message), (None, 0));
    assert_eq!(cut.buttons, vec![Some(8), Some(12)]);
    // No `|`: JIP's "OK".
    assert_eq!(box_text(b"Only").buttons, vec![None]);
    // A run of `|` is one separator (`GetNextToken` zeroes the whole run).
    let cut = box_text(b"M|A|||B");
    assert_eq!(cut.buttons, vec![Some(2), Some(6)]);
    assert_eq!(cut.bytes, b"M\0A\0\0\0B\0");
    let cut = box_text(b"M|");
    assert_eq!(cut.buttons, vec![None], "a trailing `|` alone: OK");
    // The title: between the first two `^` of the message part.
    let cut = box_text(b"^Head^Body|Go");
    assert_eq!((cut.title, cut.message), (Some(1), 6));
    assert_eq!(cut.bytes, b"^Head\0Body\0Go\0");
    // A second `^` only after the first `|` does not count.
    let cut = box_text(b"^Head|B^C");
    assert_eq!((cut.title, cut.message), (None, 0));
    // 255 buttons at most: the rest stays in the last one.
    let many: Vec<u8> = std::iter::once(b'M')
        .chain((0..300).flat_map(|_| *b"|x"))
        .collect();
    let cut = box_text(&many);
    assert_eq!(cut.buttons.len(), 255);
    let last = cut.buttons[254].unwrap();
    assert_eq!(cut.bytes[last..last + 3], *b"x|x", "not cut past the 255th");
}

// ---- MessageBoxExAlt ----

#[test]
fn a_box_is_shown_with_the_callback_routine() {
    let mut e = setup();
    assert_eq!(
        call_on(&mut e, 0x2826, 0, &[r(1), s(b"^T^Pick|A|B")]),
        (1, 0.0)
    );
    assert_eq!(
        shown(),
        vec![Shown {
            message: b(b"Pick"),
            title: Some(b(b"T")),
            callback: MESSAGE_BOX_CALLBACK,
            fixed: [0, 0, 0x17, 0],
            buttons: vec![b(b"A"), b(b"B")],
        }]
    );
    assert_eq!(message_box_scripts_pending(&mut e), 1);
    call_on(&mut e, 0x2826, 0, &[r(3), s(b"Plain")]);
    let s = shown();
    assert_eq!(
        (s[0].title.clone(), s[0].buttons.clone()),
        (None, vec![b(b"OK")])
    );
    assert_eq!(
        message_box_scripts_pending(&mut e),
        2,
        "a form not a script: kept as none"
    );
}

#[test]
fn the_answer_calls_the_newest_script_with_the_button() {
    let mut e = setup();
    call_on(&mut e, 0x2826, 0, &[r(1), s(b"One|A")]);
    call_on(&mut e, 0x2826, 0, &[r(3), s(b"Two|A")]);
    call_on(&mut e, 0x2826, 0, &[r(2), s(b"Three|A")]);
    e.mem.set_u8(IM_AT + 0xe4, 2);
    e.call(MESSAGE_BOX_CALLBACK, &[]);
    assert_eq!(e.mem.u8(IM_AT + 0xe4), 0xff, "the answer reset");
    assert_eq!(
        crate::script_call::calls(),
        vec![(SCPT2, 0, vec![2])],
        "the newest first"
    );
    assert_eq!(message_box_scripts_pending(&mut e), 2);
    // The middle box had no script: nothing called.
    e.mem.set_u8(IM_AT + 0xe4, 0);
    e.call(MESSAGE_BOX_CALLBACK, &[]);
    assert!(crate::script_call::calls().is_empty());
    e.mem.set_u8(IM_AT + 0xe4, 1);
    e.call(MESSAGE_BOX_CALLBACK, &[]);
    assert_eq!(crate::script_call::calls(), vec![(SCPT, 0, vec![1])]);
    assert_eq!(message_box_scripts_pending(&mut e), 0);
    // None left: only the answer's reset.
    e.mem.set_u8(IM_AT + 0xe4, 1);
    e.call(MESSAGE_BOX_CALLBACK, &[]);
    assert!(crate::script_call::calls().is_empty());
    assert_eq!(e.mem.u8(IM_AT + 0xe4), 0xff);
}

/// G55 with the command: the message menu's two width seams answer 1200 while any box waits
/// (with a script or not), already inside `ShowMessageBox` (which lays out the first box at once:
/// JIP counts the box before it shows it), and not before the first box nor after the last answer.
#[test]
fn g55_widens_the_message_menu_while_a_box_waits() {
    use crate::game_patches::ga::MAX_MESSAGE_WIDTH;
    let wide = Some(1200.0f32.to_bits());
    let widths = |e: &mut Engine| MAX_MESSAGE_WIDTH.map(|site| e.site(site.addr(), &[0, 0]));
    let at_show = || WIDTHS_AT_SHOW.with(|v| std::mem::take(&mut *v.borrow_mut()));
    let mut e = setup();
    assert_eq!(widths(&mut e), [None, None], "no box: the game's");
    call_on(&mut e, 0x2826, 0, &[r(1), s(b"Pick|A|B")]);
    assert_eq!(
        at_show(),
        vec![[wide, wide]],
        "laid out wide inside the show"
    );
    assert_eq!(widths(&mut e), [wide, wide]);
    call_on(&mut e, 0x2826, 0, &[r(3), s(b"Plain")]);
    assert_eq!(at_show(), vec![[wide, wide]], "a box with no script");
    assert_eq!(message_box_scripts_pending(&mut e), 2);
    e.mem.set_u8(IM_AT + 0xe4, 0);
    e.call(MESSAGE_BOX_CALLBACK, &[]);
    assert_eq!(widths(&mut e), [wide, wide], "one of two answered");
    e.mem.set_u8(IM_AT + 0xe4, 0);
    e.call(MESSAGE_BOX_CALLBACK, &[]);
    assert_eq!(widths(&mut e), [None, None], "answered: the game's again");
    // One box with no script alone.
    call_on(&mut e, 0x2826, 0, &[r(3), s(b"Plain")]);
    assert_eq!(at_show(), vec![[wide, wide]]);
    assert_eq!(widths(&mut e), [wide, wide]);
}

#[test]
fn fifteen_boxes_at_most() {
    let mut e = setup();
    for _ in 0..15 {
        call_on(&mut e, 0x2826, 0, &[r(1), s(b"M")]);
    }
    assert_eq!(shown().len(), 15);
    call_on(&mut e, 0x2826, 0, &[r(1), s(b"M")]);
    assert!(shown().is_empty(), "full: nothing shown");
    assert_eq!(message_box_scripts_pending(&mut e), 15);
    clear_message_box_scripts(&mut e);
    assert_eq!(message_box_scripts_pending(&mut e), 0);
    call_on(&mut e, 0x2826, 0, &[r(1), s(b"M")]);
    assert_eq!(shown().len(), 1);
}

// ---- SetFontFile ----

fn inits() -> Vec<Init> {
    INITS.with(|s| std::mem::take(&mut *s.borrow_mut()))
}

#[test]
fn a_font_file_made_once_and_put_in_its_slot() {
    let mut e = setup();
    e.mem.set_u32(0x011f_33f8, MANAGER);
    files::put("data\\fonts\\new.fnt", b"x");
    let heap = heap_next(&e);
    assert_eq!(
        call_on(&mut e, 0x274c, 0, &[n(3), s(b"Fonts\\New.fnt")]),
        (1, 1.0)
    );
    assert_eq!(inits(), vec![(heap, 3, b(b"Fonts\\New.fnt"), 1)]);
    assert_eq!(allocations(), vec![0x54]);
    assert_eq!(e.mem.u32(MANAGER + 8), heap, "font 3 at +8");
    // The same path again (any case): the kept one, no new load.
    assert_eq!(
        call_on(&mut e, 0x274c, 0, &[n(1), s(b"FONTS\\NEW.FNT")]),
        (1, 1.0)
    );
    assert!(inits().is_empty());
    assert_eq!(e.mem.u32(MANAGER), heap);
    assert_eq!(font_info(&mut e, b"fonts\\new.fnt"), heap);
}

#[test]
fn a_font_file_refused() {
    let mut e = setup();
    e.mem.set_u32(0x011f_33f8, MANAGER);
    files::put("data\\fonts\\bad.fnt", b"x");
    // Not there.
    assert_eq!(
        call_on(&mut e, 0x274c, 0, &[n(2), s(b"Fonts\\none.fnt")]),
        (1, 0.0)
    );
    assert!(inits().is_empty());
    // There but not loaded: the block given back.
    let heap = heap_next(&e);
    assert_eq!(
        call_on(&mut e, 0x274c, 0, &[n(2), s(b"Fonts\\bad.fnt")]),
        (1, 0.0)
    );
    assert_eq!(inits().len(), 1);
    assert_eq!(freed(), vec![heap]);
    assert_eq!(e.mem.u32(MANAGER + 4), 0);
    assert_eq!(font_info(&mut e, b"Fonts\\bad.fnt"), 0, "not kept");
    // Ids: 0, 9 and past 89 refused; an empty path refused.
    files::put("data\\f.fnt", b"x");
    for id in [0, 9, 90, -1] {
        assert_eq!(
            call_on(&mut e, 0x274c, 0, &[n(id), s(b"f.fnt")]),
            (1, 0.0),
            "{id}"
        );
    }
    assert_eq!(call_on(&mut e, 0x274c, 0, &[n(1), s(b"")]), (1, 0.0));
    assert!(inits().is_empty());
}

#[test]
fn fonts_above_8_need_jips_manager() {
    let mut e = setup();
    e.mem.set_u32(0x011f_33f8, MANAGER);
    files::put("data\\f.fnt", b"x");
    assert_eq!(call_on(&mut e, 0x274c, 0, &[n(10), s(b"f.fnt")]), (1, 0.0));
    assert!(inits().is_empty(), "the game's manager ends after font 8");
    assert_eq!(call_on(&mut e, 0x274c, 0, &[n(8), s(b"f.fnt")]), (1, 1.0));
    let info = e.mem.u32(MANAGER + 0x1c);
    set_font_manager_extended(&mut e);
    assert_eq!(call_on(&mut e, 0x274c, 0, &[n(89), s(b"f.fnt")]), (1, 1.0));
    assert_eq!(e.mem.u32(MANAGER + 0x160), info);
    // Font 9 stays refused with JIP's manager too (the byte after the eight in the game's).
    assert_eq!(call_on(&mut e, 0x274c, 0, &[n(9), s(b"f.fnt")]), (1, 0.0));
    assert_eq!(e.mem.u32(MANAGER + 0x20), 0);
    // G129's own fonts go in the same map.
    assert!(!insert_font_info(&mut e, b"F.FNT", 7));
    assert!(insert_font_info(&mut e, b"g.fnt", 7));
    assert_eq!(call_on(&mut e, 0x274c, 0, &[n(10), s(b"G.fnt")]), (1, 1.0));
    assert_eq!(e.mem.u32(MANAGER + 0x24), 7);
}

/// The callback's lambda variables: kept once more for each box with that script (none for a
/// form that is not a script), given up by each answer and by `clear_message_box_scripts`.
#[test]
fn the_callbacks_lambda_variables_are_kept_while_its_box_waits() {
    let mut e = setup();
    let list = FORMS + 0x5_0000;
    nvse::lambda::insert_for_tests(&mut e, SCPT, list);
    let kept = |e: &Engine| nvse::lambda::saved_count(e, SCPT);
    assert_eq!(kept(&e), 0);
    call_on(&mut e, 0x2826, 0, &[r(1), s(b"One|A")]);
    assert_eq!(kept(&e), 1);
    call_on(&mut e, 0x2826, 0, &[r(3), s(b"Two|A")]);
    assert_eq!(kept(&e), 1, "a form not a script: nothing kept");
    call_on(&mut e, 0x2826, 0, &[r(1), s(b"Three|A")]);
    assert_eq!(kept(&e), 2);
    e.call(MESSAGE_BOX_CALLBACK, &[]);
    assert_eq!(kept(&e), 1, "the answer gives one up");
    e.call(MESSAGE_BOX_CALLBACK, &[]);
    assert_eq!(kept(&e), 1, "the non-script box's answer: nothing");
    call_on(&mut e, 0x2826, 0, &[r(1), s(b"Four|A")]);
    assert_eq!(kept(&e), 2);
    clear_message_box_scripts(&mut e);
    assert_eq!(kept(&e), 0, "cleared: every kept one given up");
}

// ---- G129 with SetFontFile ----

/// The game's eight font settings' texts (`Fonts\F<n>.fnt`) at `0x11F3418`, 0xC apart.
fn game_font_settings(e: &mut Engine) {
    for n in 1..=8u32 {
        let text = FORMS + 0x6_0000 + 0x40 * n;
        e.mem.write(text, format!("Fonts\\F{n}.fnt\0").as_bytes());
        e.mem.set_u32(0x011f_3418 + 0xc * (n - 1), text);
    }
}

/// G129's start: the game's eight fonts loaded and kept, slot 9 none, the `xfonts` lines (the
/// first for a number wins; a missing file, one that does not load, or a game font's path) put in
/// 10 to 89, font 3 for the rest; the last font measured 89. Then `SetFontFile 1` with a game
/// font's path takes the kept one, with no file there to check.
#[test]
fn g129_makes_jips_font_manager_and_set_font_file_uses_its_map() {
    use crate::game_patches::ga::{FONT_MANAGER_START, LAST_FONT};
    let mut e = setup();
    game_font_settings(&mut e);
    files::put(
        "data\\nvse\\plugins\\xfonts\\list.txt",
        b"10=Fonts\\X.fnt\r\n11=Fonts\\Missing.fnt\n10=Fonts\\Y.fnt\n12=Fonts\\bad.fnt\n13=Fonts\\F2.fnt\nnothing\n",
    );
    files::put(
        "data\\nvse\\plugins\\xfonts\\notes.ini",
        b"14=Fonts\\X.fnt\n",
    );
    for f in ["x", "y", "bad", "f2"] {
        files::put(&format!("data\\fonts\\{f}.fnt"), b"x");
    }
    assert_eq!(e.mem.u32(0x011f_33f8), 0);
    assert_eq!(e.site(FONT_MANAGER_START.addr(), &[]), Some(1));
    let manager = e.mem.u32(0x011f_33f8);
    assert_ne!(manager, 0);
    let made = inits();
    let font = |e: &Engine, n: u32| e.mem.u32(manager + 4 * (n - 1));
    // Whole calls: the block (the font's slot), the number, the path, 1 (load the file: JIP's
    // `push 1` at both calls).
    let game: Vec<Init> = (1..=8u32)
        .map(|n| (font(&e, n), n, format!("Fonts\\F{n}.fnt").into_bytes(), 1))
        .collect();
    assert_eq!(made[..8], game[..]);
    assert_eq!(e.mem.u32(manager + 0x20), 0, "font 9: none");
    let font3 = font(&e, 3);
    let given_back = freed();
    assert_eq!(given_back.len(), 1, "not loaded: given back");
    assert_eq!(
        made[8..],
        [
            (font(&e, 10), 10, b(b"Fonts\\X.fnt"), 1),
            (given_back[0], 12, b(b"Fonts\\bad.fnt"), 1)
        ],
        "only new paths load; the second 10, the missing file and the game font's do not"
    );
    // JIP's sizes (`push 0x164`, `push 0x54`): the manager of 89 slots, then each `FontInfo`.
    assert_eq!(allocations(), [vec![0x164], vec![0x54; 10]].concat());
    assert_eq!(font(&e, 11), font3, "missing: font 3");
    assert_eq!(font(&e, 12), font3);
    assert_eq!(
        font(&e, 13),
        font(&e, 2),
        "a game font's path: the kept one"
    );
    assert_eq!(font(&e, 14), font3, "a list not .txt is not read");
    assert_eq!(font(&e, 89), font3);
    assert_eq!(e.site(LAST_FONT.addr(), &[89, manager]), Some(89));
    // `SetFontFile 1` with font 5's path: no such loose file, the kept `FontInfo` taken.
    assert_eq!(
        call_on(&mut e, 0x274c, 0, &[n(1), s(b"fonts\\f5.FNT")]),
        (1, 1.0)
    );
    assert!(inits().is_empty(), "no new load");
    assert_eq!(font(&e, 1), made[4].0);
    // Font 50 takes X's; again: the manager there, nothing made.
    assert_eq!(
        call_on(&mut e, 0x274c, 0, &[n(50), s(b"Fonts\\X.fnt")]),
        (1, 1.0)
    );
    assert_eq!(font(&e, 50), made[8].0);
    assert_eq!(e.site(FONT_MANAGER_START.addr(), &[]), Some(1));
    assert_eq!(e.mem.u32(0x011f_33f8), manager);
    assert!(inits().is_empty());
}

/// G129's list edges and shared paths: list numbers below 10 (5, 9) and past 89 (90) are passed
/// over and 89, the last, is taken (its file loaded into slot 89, `+0x160`); two game fonts whose
/// settings name one file each load and keep their own `FontInfo`, the map keeps the first, and a
/// list line with that path takes the map's.
#[test]
fn g129_list_edges_and_game_fonts_sharing_a_path() {
    use crate::game_patches::ga::FONT_MANAGER_START;
    let mut e = setup();
    game_font_settings(&mut e);
    // Font 5's setting names font 4's file.
    let four = e.mem.u32(0x011f_3418 + 0xc * 3);
    e.mem.set_u32(0x011f_3418 + 0xc * 4, four);
    files::put(
        "data\\nvse\\plugins\\xfonts\\edges.txt",
        b"5=Fonts\\A.fnt\n9=Fonts\\A.fnt\n89=Fonts\\A.fnt\n90=Fonts\\A.fnt\n10=Fonts\\F4.fnt\n",
    );
    for f in ["a", "f4"] {
        files::put(&format!("data\\fonts\\{f}.fnt"), b"x");
    }
    assert_eq!(e.site(FONT_MANAGER_START.addr(), &[]), Some(1));
    let manager = e.mem.u32(0x011f_33f8);
    let font = |e: &Engine, n: u32| e.mem.u32(manager + 4 * (n - 1));
    let made = inits();
    let path = b(b"Fonts\\F4.fnt");
    assert_eq!(made.len(), 9, "the eight game fonts, then A once");
    assert_eq!(made[3], (font(&e, 4), 4, path.clone(), 1));
    assert_eq!(made[4], (font(&e, 5), 5, path, 1), "font 5: its own");
    assert_ne!(font(&e, 4), font(&e, 5));
    assert_eq!(
        font_info(&mut e, b"fonts\\f4.fnt"),
        font(&e, 4),
        "the map: the first"
    );
    assert_eq!(
        font(&e, 10),
        font(&e, 4),
        "a list line with that path: the map's"
    );
    assert_eq!(
        made[8],
        (font(&e, 89), 89, b(b"Fonts\\A.fnt"), 1),
        "89 taken"
    );
    assert_eq!(e.mem.u32(manager + 0x160), made[8].0);
    assert_eq!(e.mem.u32(manager + 0x20), 0, "9= not taken: slot 9 none");
    let font3 = font(&e, 3);
    for n in 11..=88 {
        assert_eq!(font(&e, n), font3, "font {n}");
    }
    assert_eq!(allocations(), [vec![0x164], vec![0x54; 9]].concat());
    assert!(freed().is_empty());
}
