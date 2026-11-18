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
    register_awaiting(&mut e, AWAITING_G129);
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
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    // `FontInfo::Init`: the data loaded (+0x38 set) unless the path names "bad".
    e.register(0x00a1_2020, |e, w| {
        let path = nvse::table::guest_text(e, w[2]);
        let loaded = !path.windows(3).any(|p| p == b"bad");
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
