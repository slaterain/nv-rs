//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE): `ShowTextInputMenu`, `SetTextInputExtendedProps` and
//! `SetTextInputString`: the game's text edit menu opened from JIP's own menu file
//! (`Data\NVSE\plugins\textinput\texteditmenu.xml`, which JIP's package installs) with a
//! callback script, and JIP's text input hooks (`internal/hooks.h`: `TextInputRefreshHook`,
//! `HandleInputKey`, `TextInputKeyPressHook`, `UnsetTextInputHooks`, `TextInputCloseHook`) that
//! run it: the typing, the cursor, the OK button, and the script called with the text. Also the
//! font heights JIP reads at its deferred start for the menu (`s_fontHeightDatas`,
//! `internal/patches_game.h`).
//!
//! JIP's hooks, as here:
//! - The menu's frame and key functions (`007E65F0`, `007E6620`) are reached only through the
//!   text edit menu's vtable slots `0x1070060` and `0x1070064` (Ghidra's references): JIP writes
//!   its routines there while its menu is open and the exe's functions back after, and so does
//!   the port ([`TEXT_INPUT_REFRESH`], [`TEXT_INPUT_KEY_PRESS`]).
//! - JIP's `TextInputClose` is a jump at the start of the menu's click function (`007E66A0`):
//!   here a seam there ([`TEXT_INPUT_CLOSE_SEAM`], asked while the hook is on), so a click event
//!   JIP's `MenuClickEvent` put in the same vtable slot ([`crate::ui_hooks`]) still reaches it
//!   through the exe's function, as in JIP.
//!
//! nv-rs's own: where JIP reads through a null pointer (no menu from the file, a menu without a
//! tile, `CloseActiveMenu` with the hook on and no menu), nothing more happens; a font number past
//! JIP's 90 heights gives 0 for both (JIP reads past its table); `CloseActiveMenu` reads the
//! menu's script before the menu is destroyed (JIP reads it from the freed menu).
//!
//! Threads: the commands, the menu's routines and the seam run on the main thread (scripts,
//! menus). The hook's count is the engine's shared word; the heights are this engine's.

use super::{
    formatted_in_buffer, heap_alloc, heap_free, string_set, type_of, with_guest_text, Handlers,
};
use crate::game_patches::Area;
use engine_core::extend::{Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x274d, show_text_input_menu),
    (0x28a2, set_text_input_extended_props),
    (0x28a3, set_text_input_string),
];

/// `TextInputRefreshHook` and `TextInputKeyPressHook`, the routines JIP writes into the menu's
/// vtable: at these addresses of JIP's commands' own routines (after `QuantityMenuCallback`'s).
pub const TEXT_INPUT_REFRESH: u32 = crate::identity::ROUTINES + 0x780;
pub const TEXT_INPUT_KEY_PRESS: u32 = crate::identity::ROUTINES + 0x784;

/// The seam at the start of the text edit menu's click function (`007E66A0`, words: the menu, the
/// button's id, the clicked tile; answered: the function's body is not run), JIP's
/// `TextInputClose`.
pub const TEXT_INPUT_CLOSE_SEAM: SiteSlot = SiteSlot::new(Owner::Jip, 0x007e_66a0);

/// The text input's seam: in with JIP's `INSTALLED`, asked only while `TextInputClose` is on.
pub const AREA: Area = Area {
    name: "text input",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[TEXT_INPUT_CLOSE_SEAM.addr()],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(TEXT_INPUT_CLOSE_SEAM, text_input_close_seam);
}

/// Registers the two routines.
pub fn install(e: &mut Engine) {
    e.register(TEXT_INPUT_REFRESH, text_input_refresh_routine);
    e.register(TEXT_INPUT_KEY_PRESS, text_input_key_press_routine);
}

// ---- the game's objects ----

/// `TextEditMenu::Get()` (`*0x011DAEC4`), its vtable (`kVtbl_TextEditMenu`) and the three slots:
/// the click (+0xC, `0x1070040`), the frame (+0x2C, `0x1070060`) and the key (+0x30,
/// `0x1070064`), with the exe's frame and key functions JIP puts back.
const TEXT_EDIT_MENU: u32 = 0x011d_aec4;
const TEXT_EDIT_VTABLE: u32 = 0x0107_0034;
const CLICK_SLOT: u32 = 0xc;
const FRAME_SLOT: u32 = 0x2c;
const KEY_SLOT: u32 = 0x30;
const EXE_FRAME: u32 = 0x007e_65f0;
const EXE_KEY: u32 = 0x007e_6620;
/// The menu closed (`007E65B0`, no arguments: the `pMenu` tile's 0x1772, the fade out, the
/// singleton cleared).
const EXE_CLOSE: u32 = 0x007e_65b0;
/// `kMenuType_TextEdit`.
pub(crate) const TEXT_EDIT: u32 = 0x41b;
/// JIP's `TextEditMenu` (nvse/GameUI.h): `Menu::tile` (+4), `Menu::id` (+0x14), the text tile
/// (+0x28), the OK button (+0x2C), the title (+0x30), the text (`String`, +0x34), the shown text
/// with its cursor (`String`, +0x3C), the cursor (+0x44), the least and most lengths (`UInt16`,
/// +0x48, +0x4A), the input rectangle (+0x4C), the blink's time (+0x50), the cursor's state
/// (+0x54), whether it takes keys (+0x55), JIP's flags (+0x57: 1 numbers only, 2 Enter
/// clicks OK) and the script (+0x58).
const MENU_TILE: u32 = 0x04;
const MENU_ID: u32 = 0x14;
const TEXT_TILE: u32 = 0x28;
const OK_BUTTON: u32 = 0x2c;
const TITLE_TILE: u32 = 0x30;
const CURRENT_TEXT: u32 = 0x34;
const SHOWN_TEXT: u32 = 0x3c;
const CURSOR: u32 = 0x44;
const MIN_LENGTH: u32 = 0x48;
const MAX_LENGTH: u32 = 0x4a;
const INPUT_RECT: u32 = 0x4c;
const CURSOR_BLINK: u32 = 0x50;
const CURSOR_VISIBLE: u32 = 0x54;
const IS_ACTIVE: u32 = 0x55;
const MISC_FLAGS: u32 = 0x57;
const SCRIPT: u32 = 0x58;
/// A `String`'s text (+0), length (+4) and room (+6), `UInt16`s.
const STR_DATA: u32 = 0;
const STR_LEN: u32 = 4;
const STR_BUF_LEN: u32 = 6;
/// A tile's parent (+0x28) and a `TileMenu`'s menu (+0x3C).
const TILE_PARENT: u32 = 0x28;
const TILE_MENU_MENU: u32 = 0x3c;
/// `g_interfaceManager` (`*0x011D8A80`): its tile reader (+0x9C), `activeTile` (+0xCC) and
/// `activeMenu` (+0xD0).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const IM_TILE_READER: u32 = 0x9c;
const IM_ACTIVE_TILE: u32 = 0xcc;
const IM_ACTIVE_MENU: u32 = 0xd0;
/// `Interface::CloseConsole` (`00703DA0`), `Tile::ReadFile` (`00A01B00`, `thiscall`: the reader,
/// the path; the root tile), `InterfaceManager::AddToEnterStack` (`00714D90`, `thiscall`: the
/// menu's id), `Menu::GetMaxDepth` (`00A1DFB0`, a float), the game's `Tile::SetFloat`
/// (`ADDR_TileSetFloat`, `00A012D0`, `thiscall`: the id, the float, a bool), `Tile::IsTrue`
/// (`00A01230`, `thiscall`: the id) and `Menu::PrepForVisibility` (`00A1DC20`, `thiscall`: a bool).
const CLOSE_CONSOLE: u32 = 0x0070_3da0;
const TILE_READ_FILE: u32 = 0x00a0_1b00;
const ADD_TO_ENTER_STACK: u32 = 0x0071_4d90;
const GET_MAX_DEPTH: u32 = 0x00a1_dfb0;
const TILE_SET_FLOAT: u32 = 0x00a0_12d0;
const TILE_IS_TRUE: u32 = 0x00a0_1230;
const PREP_FOR_VISIBILITY: u32 = 0x00a1_dc20;
/// A tile value's number (+8).
const VALUE_NUM: u32 = 8;
/// The game's millisecond clock JIP reads (`*0x011F63A8`), the exe's empty text, and the OK
/// button's game setting's text (`*0x011D38BC`, the value of the setting at `011D38B8`).
const TICK_COUNT: u32 = 0x011f_63a8;
const EMPTY_TEXT: u32 = 0x0101_1584;
const OK_TEXT: u32 = 0x011d_38bc;
/// `FontManager::GetSingleton()` (`*0x011F33F8`): its `fontInfos` from +0; a `FontInfo`'s
/// `bufferData` (+0x38), whose `lineHeight` is at +0 and the space's `charDimensions` height at
/// `0x128 + 0x20 * 0x38 + 0x28`.
const FONT_MANAGER: u32 = 0x011f_33f8;
const FONT_INFO_BUFFER_DATA: u32 = 0x38;
const SPACE_HEIGHT: u32 = 0x128 + 0x20 * 0x38 + 0x28;
/// `kTileValue_*`: `visible`, `depth`, `target`, `font`, `mouseover`, `string`, `user0` to `user3`.
const TRAIT_VISIBLE: u32 = 0xfa3;
const TRAIT_DEPTH: u32 = 0xfad;
const TRAIT_TARGET: u32 = 0xfaf;
const TRAIT_FONT: u32 = 0xfb9;
const TRAIT_MOUSEOVER: u32 = 0xfc3;
const TRAIT_STRING: u32 = 0xfc4;
const TRAIT_USER0: u32 = 0x1004;
const TRAIT_USER1: u32 = 0x1005;
const TRAIT_USER2: u32 = 0x1006;
const TRAIT_USER3: u32 = 0x1007;
/// `kInputCode_*` (nvse/GameOSDepend.h).
const BACKSPACE: u32 = 0x8000_0000;
const ARROW_LEFT: u32 = 0x8000_0001;
const ARROW_RIGHT: u32 = 0x8000_0002;
const ARROW_UP: u32 = 0x8000_0003;
const ARROW_DOWN: u32 = 0x8000_0004;
const HOME: u32 = 0x8000_0005;
const END: u32 = 0x8000_0006;
const DELETE: u32 = 0x8000_0007;
const ENTER: u32 = 0x8000_0008;
const PAGE_UP: u32 = 0x8000_0009;
const PAGE_DOWN: u32 = 0x8000_000a;
/// `kFormType_Script`.
const SCRIPT_TYPE: u8 = 0x11;
/// JIP's menu file (`altXMLPath`).
const ALT_XML_PATH: &[u8] = b"Data\\NVSE\\plugins\\textinput\\texteditmenu.xml";
/// The title is cut at its 80th byte (JIP's `mov byte ptr [esi+0x50], 0`).
const TITLE_MAX: usize = 0x50;
/// `String::Init(0x400)` for both texts.
const TEXT_ROOM: u16 = 0x400;
/// The cursor blinks every 500 ms.
const BLINK_MS: u32 = 500;
/// `kCommandInfo_ShowTextInputMenu.numParams` (the form, two floats and xNVSE's 21 format
/// parameters) and `kCommandInfo_SetTextInputString.numParams`.
const SHOW_PARAMS: u32 = 24;
const STRING_PARAMS: u32 = 21;
/// JIP's `GetStrArgBuffer()`.
const STR_ARG_BUFFER: usize = 0x4_0000;

// ---- the font heights ----

/// JIP's `s_fontHeightDatas[90]`: each font's height (`heightBase`, the space's) and its line's
/// (`heightwGap`), by font number; set at the deferred start.
pub struct FontHeights(Vec<(f32, f32)>);

impl Default for FontHeights {
    fn default() -> Self {
        FontHeights(vec![(0.0, 0.0); FONT_HEIGHTS])
    }
}

const FONT_HEIGHTS: usize = 90;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit (the font
// heights).
/// The heights of fonts 1 to 89 but 9 from the font manager's `FontInfo`s (font `n` is the
/// manager's `n - 1`th), then 4, 6, 4 and 2 more line height for fonts 2, 3, 6 and 7. With JIP's
/// font manager patch (G129, `game_patches::ga`) the manager has all 89; without it only the
/// game's 1 to 8, and the others stay 0 (JIP's loop would read past the game's eight; no menu can
/// use them). A font that did not load is left at 0.
pub(crate) fn deferred_init(e: &mut Engine) {
    // nv-rs's own: an engine without the font manager's page (a test's) keeps the heights at 0;
    // the game always has it.
    if !e.mem.is_mapped(FONT_MANAGER) {
        return;
    }
    let manager = e.mem.u32(FONT_MANAGER);
    let fonts = if super::jip_fn_ui::font_manager_extended(e) {
        FONT_HEIGHTS as u32
    } else {
        9
    };
    let mut heights = vec![(0.0f32, 0.0f32); FONT_HEIGHTS];
    if manager != 0 {
        for n in 1..fonts {
            if n == 9 {
                continue;
            }
            let info = e.mem.u32(manager + 4 * (n - 1));
            if info == 0 {
                continue;
            }
            let data = e.mem.u32(info + FONT_INFO_BUFFER_DATA);
            if data == 0 {
                continue;
            }
            heights[n as usize] = (e.mem.f32(data + SPACE_HEIGHT), e.mem.f32(data));
        }
    }
    heights[2].1 += 4.0;
    heights[3].1 += 6.0;
    heights[6].1 += 4.0;
    heights[7].1 += 2.0;
    e.ext::<FontHeights>().0 = heights;
}

/// Font `n`'s two heights; 0 for both past the table.
pub fn font_heights(e: &mut Engine, n: i32) -> (f32, f32) {
    usize::try_from(n)
        .ok()
        .and_then(|i| e.ext::<FontHeights>().0.get(i).copied())
        .unwrap_or((0.0, 0.0))
}

// ---- JIP's String ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.cpp: String::Init.
/// The string's text freed, a new one of `room` bytes and a 0, its length 0.
fn string_init(e: &mut Engine, s: u32, room: u16) {
    let old = e.mem.u32(s + STR_DATA);
    if old != 0 {
        heap_free(e, old);
    }
    e.mem.set_u16(s + STR_BUF_LEN, room);
    let data = heap_alloc(e, u32::from(room) + 1);
    e.mem.set_u32(s + STR_DATA, data);
    e.mem.set_u8(data, 0);
    e.mem.set_u16(s + STR_LEN, 0);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.cpp: String::InsertChar.
/// `c` put in at `index` (nothing past the end); a full string gets a new text one longer.
fn string_insert_char(e: &mut Engine, s: u32, c: u8, index: u32) {
    let len = e.mem.u16(s + STR_LEN);
    if index > u32::from(len) {
        return;
    }
    let room = e.mem.u16(s + STR_BUF_LEN);
    if room == len {
        let room = room.wrapping_add(1);
        e.mem.set_u16(s + STR_BUF_LEN, room);
        let fresh = heap_alloc(e, u32::from(room) + 1);
        let old = e.mem.u32(s + STR_DATA);
        if old != 0 {
            let bytes = e.mem.bytes(old, u32::from(len));
            e.mem.write(fresh, &bytes);
            heap_free(e, old);
        }
        e.mem.set_u32(s + STR_DATA, fresh);
    }
    let data = e.mem.u32(s + STR_DATA);
    if index < u32::from(len) {
        let tail = e.mem.bytes(data + index, u32::from(len) - index);
        e.mem.write(data + index + 1, &tail);
    }
    let len = len.wrapping_add(1);
    e.mem.set_u16(s + STR_LEN, len);
    e.mem.set_u8(data + index, c);
    e.mem.set_u8(data + u32::from(len), 0);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.cpp: String::EraseAt.
/// The byte at `index` taken out (nothing without a text or past the end).
fn string_erase_at(e: &mut Engine, s: u32, index: u32) {
    let data = e.mem.u32(s + STR_DATA);
    let len = e.mem.u16(s + STR_LEN);
    if data == 0 || index >= u32::from(len) {
        return;
    }
    let len = len - 1;
    e.mem.set_u16(s + STR_LEN, len);
    if len != 0 {
        let after = u32::from(len) - index;
        if after == 0 {
            e.mem.set_u8(data + u32::from(len), 0);
        } else {
            let tail = e.mem.bytes(data + index + 1, after + 1);
            e.mem.write(data + index, &tail);
        }
    } else {
        e.mem.set_u8(data, 0);
    }
}

/// The text at `at` up to its 0 (none for a null pointer).
fn text_at(e: &Engine, at: u32) -> Vec<u8> {
    if at == 0 {
        Vec::new()
    } else {
        nvse::table::guest_text(e, at)
    }
}

// ---- JIP's tile setters ----

/// The tile's value of `id`, 0 for none (and for no tile: nv-rs's own).
fn value(e: &Engine, tile: u32, id: u32) -> u32 {
    if tile == 0 {
        0
    } else {
        super::jip_fn_ui::value_of(e, tile, id)
    }
}

/// `Tile::GetValueFloat` (JIP LN NVSE): the value's number, 0 for none.
fn get_float(e: &Engine, tile: u32, id: u32) -> f32 {
    match value(e, tile, id) {
        0 => 0.0,
        v => e.mem.f32(v + VALUE_NUM),
    }
}

/// `Tile::SetFloat` (JIP LN NVSE): set (and refreshed) unless equal or either is a NaN.
fn set_float(e: &mut Engine, tile: u32, id: u32, num: f32) {
    let v = value(e, tile, id);
    if v != 0 {
        super::jip_fn_ui::value_set_float(e, v, num);
    }
}

/// `Tile::SetBool` (JIP LN NVSE).
fn set_bool(e: &mut Engine, tile: u32, id: u32, on: bool) {
    let v = value(e, tile, id);
    if v != 0 {
        super::jip_fn_ui::value_set_bool(e, v, on);
    }
}

/// `Tile::SetString` (JIP LN NVSE).
fn set_string(e: &mut Engine, tile: u32, id: u32, text: &[u8]) {
    let v = value(e, tile, id);
    if v != 0 {
        crate::game_patches::gb::value_set_string(e, v, text);
    }
}

/// `cvtss2si`: rounded to the nearest (ties to even), `0x80000000` when it does not fit (the same
/// as the FPU's `FISTP` in its default mode).
fn cvtss2si(v: f32) -> i32 {
    engine_core::x87::fistp(f64::from(v))
}

// ---- ShowTextInputMenu ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_ShowTextInputMenu_Execute.
/// `ShowTextInputMenu callback width height "title" ...`: with no text edit menu open, the
/// arguments read and the callback a script, JIP's menu shown ([`show_text_edit_menu`]); result
/// 1 when it is.
fn show_text_input_menu(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if e.mem.u32(TEXT_EDIT_MENU) != 0 {
        return handled(true);
    }
    let f = nvse::commands::string::format_string_args_at(e, &a, 3, SHOW_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let callback = f.leading.first().and_then(Arg::form).unwrap_or(0);
    let width = f.leading.get(1).and_then(Arg::float).unwrap_or(0.0);
    let height = f.leading.get(2).and_then(Arg::float).unwrap_or(0.0);
    if callback == 0 || type_of(e, callback) != SCRIPT_TYPE {
        return handled(true);
    }
    let mut title = formatted_in_buffer(e, &f.text, STR_ARG_BUFFER, "ShowTextInputMenu");
    title.truncate(TITLE_MAX);
    if show_text_edit_menu(e, width, height, &title, callback) != 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: ShowTextEditMenu (asm).
/// The console closed, JIP's menu file read; with a menu from it: its tile and id set, put on the
/// menu stack, the active tile's mouse-over taken off (unless the active menu is this one), the
/// menu kept as the text edit menu, the tile's depth the menus' greatest, `user0`/`user1` the
/// width and height; the title (hidden when empty) and the text tile's two heights for its font;
/// both texts made anew (the shown one a lone `|`), the OK button's text the game setting's; the
/// cursor at 0, lengths 1 to 0x7FFF, the input rectangle the text tile's parent, taking keys, the
/// script kept with its lambda variables; refreshed, prepared to show, and JIP's hooks in. The
/// menu, or 0.
fn show_text_edit_menu(e: &mut Engine, width: f32, height: f32, title: &[u8], script: u32) -> u32 {
    e.call(CLOSE_CONSOLE, &[]);
    let im = e.mem.u32(INTERFACE_MANAGER);
    let reader = e.mem.u32(im + IM_TILE_READER);
    let root = with_guest_text(e, ALT_XML_PATH, |e, path| {
        e.call(TILE_READ_FILE, &args![reader, path]).u32()
    });
    if root == 0 {
        return 0;
    }
    let menu = e.mem.u32(root + TILE_MENU_MENU);
    if menu == 0 {
        return 0;
    }
    e.mem.set_u32(menu + MENU_TILE, root);
    e.mem.set_u32(menu + MENU_ID, TEXT_EDIT);
    e.call(ADD_TO_ENTER_STACK, &args![im, TEXT_EDIT]);
    if e.mem.u32(im + IM_ACTIVE_MENU) != menu {
        let active = e.mem.u32(im + IM_ACTIVE_TILE);
        if active != 0 {
            set_bool(e, active, TRAIT_MOUSEOVER, false);
        }
        e.mem.set_u32(im + IM_ACTIVE_TILE, 0);
        e.mem.set_u32(im + IM_ACTIVE_MENU, 0);
    }
    e.mem.set_u32(TEXT_EDIT_MENU, menu);
    let depth = e.call(GET_MAX_DEPTH, &[]).f32();
    let tile = e.mem.u32(menu + MENU_TILE);
    e.call(TILE_SET_FLOAT, &args![tile, TRAIT_DEPTH, depth, 0u32]);
    set_float(e, tile, TRAIT_USER0, width);
    set_float(e, tile, TRAIT_USER1, height);
    let title_tile = e.mem.u32(menu + TITLE_TILE);
    if title.is_empty() {
        set_bool(e, title_tile, TRAIT_VISIBLE, false);
    }
    set_string(e, title_tile, TRAIT_STRING, title);
    let text_tile = e.mem.u32(menu + TEXT_TILE);
    let font = cvtss2si(get_float(e, text_tile, TRAIT_FONT));
    let (base, gap) = font_heights(e, font);
    set_float(e, text_tile, TRAIT_USER0, base);
    set_float(e, text_tile, TRAIT_USER1, gap);
    string_init(e, menu + CURRENT_TEXT, TEXT_ROOM);
    string_init(e, menu + SHOWN_TEXT, TEXT_ROOM);
    let shown = e.mem.u32(menu + SHOWN_TEXT + STR_DATA);
    e.mem.set_u8(shown, b'|');
    e.mem.set_u8(shown + 1, 0);
    let len = e.mem.u16(menu + SHOWN_TEXT + STR_LEN);
    e.mem
        .set_u16(menu + SHOWN_TEXT + STR_LEN, len.wrapping_add(1));
    let ok = text_at(e, e.mem.u32(OK_TEXT));
    let ok_button = e.mem.u32(menu + OK_BUTTON);
    set_string(e, ok_button, TRAIT_STRING, &ok);
    e.mem.set_u32(menu + CURSOR, 0);
    e.mem.set_u16(menu + MIN_LENGTH, 1);
    e.mem.set_u16(menu + MAX_LENGTH, 0x7fff);
    let rect = e.mem.u32(e.mem.u32(menu + TEXT_TILE) + TILE_PARENT);
    e.mem.set_u32(menu + INPUT_RECT, rect);
    e.mem.set_u32(menu + CURSOR_VISIBLE, 0x0000_0100);
    e.mem.set_u32(menu + SCRIPT, script);
    nvse::lambda::save_variables(e, script);
    text_input_refresh(e, menu);
    e.call(PREP_FOR_VISIBILITY, &args![menu, 0u32]);
    e.set_hook(Hook::TextInputClose, true);
    e.mem
        .set_u32(TEXT_EDIT_VTABLE + FRAME_SLOT, TEXT_INPUT_REFRESH);
    e.mem
        .set_u32(TEXT_EDIT_VTABLE + KEY_SLOT, TEXT_INPUT_KEY_PRESS);
    menu
}

// ---- SetTextInputExtendedProps, SetTextInputString ----

/// The text edit menu while JIP's text input hooks are in, else 0.
fn jips_menu(e: &Engine) -> u32 {
    let menu = e.mem.u32(TEXT_EDIT_MENU);
    if menu != 0 && e.hook_users(Hook::TextInputClose) != 0 {
        menu
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetTextInputExtendedProps_Execute.
/// `SetTextInputExtendedProps x y [least] [most] [flags]`: on JIP's open menu, with the arguments
/// read: the menu tile's `user2`/`user3` the position given above 0, the least length (1 when not
/// given) and the most (`& 0x7FFF`; 0x7FFF for 0 or none) as 16-bit numbers, JIP's two flags
/// (`& 3`). No result.
fn set_text_input_extended_props(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let menu = jips_menu(e);
    if menu == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let x = read.float(0, 0.0);
    let y = read.float(1, 0.0);
    let least = read.int(2, 1);
    let most = read.int(3, 0);
    let flags = read.int(4, 0);
    let tile = e.mem.u32(menu + MENU_TILE);
    if x > 0.0 {
        set_float(e, tile, TRAIT_USER2, x);
    }
    if y > 0.0 {
        set_float(e, tile, TRAIT_USER3, y);
    }
    e.mem.set_u16(menu + MIN_LENGTH, least as u16);
    let most = if most != 0 { most & 0x7fff } else { 0x7fff };
    e.mem.set_u16(menu + MAX_LENGTH, most as u16);
    e.mem.set_u8(menu + MISC_FLAGS, (flags & 3) as u8);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetTextInputString_Execute.
/// `SetTextInputString "text" ...`: on JIP's open menu, with the arguments read: the menu's text
/// set to the formatted text, the cursor at 0, then the End key handed to the menu (its key slot:
/// JIP's routine, so the cursor goes to the first line's end and the shown text is made anew).
/// No result.
fn set_text_input_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let menu = jips_menu(e);
    if menu == 0 {
        return handled(true);
    }
    let f = nvse::commands::string::format_string_args_at(e, &a, 0, STRING_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let text = formatted_in_buffer(e, &f.text, STR_ARG_BUFFER, "SetTextInputString");
    string_set(e, menu + CURRENT_TEXT, &text);
    e.mem.set_u32(menu + CURSOR, 0);
    e.vcall(menu, KEY_SLOT, &args![END]);
    handled(true)
}

// ---- JIP's hooks ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: TextInputRefreshHook (asm).
/// While the menu takes keys: every 500 ms or more (the game's clock, unsigned) the cursor's
/// state flips and the shown text's cursor byte becomes `0x7F` (now on) or `|` (off), shown in
/// the text tile; then the OK button's `target` is whether the text has the least length, and the
/// input rectangle's `user2` gets its `user1`.
fn text_input_refresh(e: &mut Engine, menu: u32) {
    if e.mem.u8(menu + IS_ACTIVE) == 0 {
        return;
    }
    let now = e.mem.u32(TICK_COUNT);
    if now.wrapping_sub(e.mem.u32(menu + CURSOR_BLINK)) > BLINK_MS {
        e.mem.set_u32(menu + CURSOR_BLINK, now);
        let visible = e.mem.u8(menu + CURSOR_VISIBLE) ^ 1;
        e.mem.set_u8(menu + CURSOR_VISIBLE, visible);
        let c = if visible != 0 { 0x7f } else { b'|' };
        let shown = e.mem.u32(menu + SHOWN_TEXT + STR_DATA);
        e.mem
            .set_u8(shown.wrapping_add(e.mem.u32(menu + CURSOR)), c);
        let text = text_at(e, shown);
        let text_tile = e.mem.u32(menu + TEXT_TILE);
        set_string(e, text_tile, TRAIT_STRING, &text);
    }
    let long_enough = e.mem.u16(menu + CURRENT_TEXT + STR_LEN) >= e.mem.u16(menu + MIN_LENGTH);
    let ok_button = e.mem.u32(menu + OK_BUTTON);
    set_bool(e, ok_button, TRAIT_TARGET, long_enough);
    let rect = e.mem.u32(menu + INPUT_RECT);
    let num = get_float(e, rect, TRAIT_USER1);
    set_float(e, rect, TRAIT_USER2, num);
}

/// The routine in the frame slot (`thiscall`, the menu).
fn text_input_refresh_routine(e: &mut Engine, w: &[u32]) -> Ret {
    text_input_refresh(e, w[0]);
    Ret::default()
}

/// The first byte `c` at or after `at` before the text's 0: its address.
fn find_chr(e: &Engine, at: u32, c: u8) -> Option<u32> {
    let text = text_at(e, at);
    text.iter().position(|&b| b == c).map(|i| at + i as u32)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: HandleInputKey.
/// One key on the menu's text, while it takes keys (a code below 0x20 does nothing): a character
/// (below Backspace's code) goes in at the cursor while the text is shorter than the most (with
/// JIP's numbers flag: a `-` only first and once, one `.`, else digits only); Backspace, the
/// arrows left and right, Home and End (within the cursor's line), Delete, Enter (a new line,
/// within the most), Page Up and Down (five places). Whether the text or the cursor changed.
fn handle_input_key(e: &mut Engine, menu: u32, key: u32) -> bool {
    if e.mem.u8(menu + IS_ACTIVE) == 0 || key < 0x20 {
        return false;
    }
    let text = e.mem.u32(menu + CURRENT_TEXT + STR_DATA);
    let index = e.mem.u32(menu + CURSOR) as i32;
    let length = i32::from(e.mem.u16(menu + CURRENT_TEXT + STR_LEN));
    let set_cursor = |e: &mut Engine, at: i32| e.mem.set_u32(menu + CURSOR, at as u32);
    if key < BACKSPACE {
        if length >= i32::from(e.mem.u16(menu + MAX_LENGTH)) {
            return false;
        }
        if e.mem.u8(menu + MISC_FLAGS) & 1 != 0 {
            if key == u32::from(b'-') {
                if index != 0 || e.mem.u8(text) == b'-' {
                    return false;
                }
            } else if key == u32::from(b'.') {
                if find_chr(e, text, b'.').is_some() {
                    return false;
                }
            } else if key < u32::from(b'0') || key > u32::from(b'9') {
                return false;
            }
        }
        string_insert_char(e, menu + CURRENT_TEXT, key as u8, index as u32);
        set_cursor(e, index.wrapping_add(1));
        return true;
    }
    match key {
        BACKSPACE => {
            if index == 0 {
                return false;
            }
            let at = index - 1;
            set_cursor(e, at);
            string_erase_at(e, menu + CURRENT_TEXT, at as u32);
            true
        }
        ARROW_LEFT => {
            if index == 0 {
                return false;
            }
            set_cursor(e, index - 1);
            true
        }
        ARROW_RIGHT => {
            if index >= length {
                return false;
            }
            set_cursor(e, index + 1);
            true
        }
        ARROW_UP | ARROW_DOWN => false,
        HOME => {
            // JIP ends the text after the cursor's byte for a moment and takes the last new
            // line before that end.
            let bytes = text_at(e, text);
            let end = bytes.len().min(index as usize + 1);
            let at = match bytes[..end].iter().rposition(|&b| b == b'\n') {
                Some(i) => i as i32 + 1,
                None => 0,
            };
            if at == index {
                return false;
            }
            set_cursor(e, at);
            true
        }
        END => {
            let from = text.wrapping_add(index as u32);
            let at = match find_chr(e, from, b'\n') {
                Some(p) => p.wrapping_sub(text) as i32,
                None => length,
            };
            if at == index {
                return false;
            }
            set_cursor(e, at);
            true
        }
        DELETE => {
            if index >= length {
                return false;
            }
            string_erase_at(e, menu + CURRENT_TEXT, index as u32);
            true
        }
        ENTER => {
            if length >= i32::from(e.mem.u16(menu + MAX_LENGTH)) {
                return false;
            }
            string_insert_char(e, menu + CURRENT_TEXT, b'\n', index as u32);
            set_cursor(e, index.wrapping_add(1));
            true
        }
        PAGE_UP => {
            if index == 0 {
                return false;
            }
            let at = index.wrapping_sub(5);
            set_cursor(e, at.max(0));
            true
        }
        PAGE_DOWN => {
            if index >= length {
                return false;
            }
            let at = index.wrapping_add(5);
            set_cursor(e, at.min(length));
            true
        }
        _ => false,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: TextInputKeyPressHook (asm).
/// The key slot's routine (`thiscall`, the menu and the key; always answers handled). Enter with
/// JIP's Enter flag: when the OK button's `target` is true, the menu's click slot is called with
/// button 1 and the OK button. Any other key: when [`handle_input_key`] changed something, the
/// shown text is the text with the cursor byte (`0x7F` while the cursor's state is off, `|`
/// while on) put in at the cursor, shown in the text tile, and the menu refreshed.
fn text_input_key_press(e: &mut Engine, menu: u32, key: u32) {
    if key == ENTER && e.mem.u8(menu + MISC_FLAGS) & 2 != 0 {
        let ok_button = e.mem.u32(menu + OK_BUTTON);
        if e.call(TILE_IS_TRUE, &args![ok_button, TRAIT_TARGET]).bool() {
            let click = e.mem.u32(TEXT_EDIT_VTABLE + CLICK_SLOT);
            e.call(click, &args![menu, 1u32, ok_button]);
        }
        return;
    }
    if !handle_input_key(e, menu, key) {
        return;
    }
    let text = text_at(e, e.mem.u32(menu + CURRENT_TEXT + STR_DATA));
    string_set(e, menu + SHOWN_TEXT, &text);
    let c = if e.mem.u8(menu + CURSOR_VISIBLE) == 0 {
        0x7f
    } else {
        b'|'
    };
    let cursor = e.mem.u32(menu + CURSOR);
    string_insert_char(e, menu + SHOWN_TEXT, c, cursor);
    let shown = text_at(e, e.mem.u32(menu + SHOWN_TEXT + STR_DATA));
    let text_tile = e.mem.u32(menu + TEXT_TILE);
    set_string(e, text_tile, TRAIT_STRING, &shown);
    text_input_refresh(e, menu);
}

fn text_input_key_press_routine(e: &mut Engine, w: &[u32]) -> Ret {
    text_input_key_press(e, w[0], w[1]);
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

/// The text edit menu's kept script (JIP's `+0x58`), 0 without a menu.
pub(crate) fn kept_script(e: &Engine) -> u32 {
    let menu = e.mem.u32(TEXT_EDIT_MENU);
    if menu == 0 {
        0
    } else {
        e.mem.u32(menu + SCRIPT)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: UnsetTextInputHooks (asm).
/// The menu's script's lambda variables given up (`script`, read from the menu: 0 for none), the
/// exe's frame and key functions put back in the vtable, `TextInputClose` off.
pub(crate) fn unset_text_input_hooks(e: &mut Engine, script: u32) {
    if script != 0 {
        nvse::lambda::unsave_variables(e, script);
    }
    e.mem.set_u32(TEXT_EDIT_VTABLE + FRAME_SLOT, EXE_FRAME);
    e.mem.set_u32(TEXT_EDIT_VTABLE + KEY_SLOT, EXE_KEY);
    e.set_hook(Hook::TextInputClose, false);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: TextInputCloseHook (asm).
/// The click function's seam (words: the menu, the button's id, the tile), asked while the hook
/// is on: JIP's routine replaces the function. Button 1 (OK): the menu stops taking keys, the
/// script is called with the text (the exe's empty text when it has none), JIP's hooks come out
/// and the menu closes (`007E65B0`); the exe's own copy of the text is not made. Another button:
/// nothing. Always answered.
fn text_input_close_seam(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let (menu, button) = (w[0], w[1]);
    if button != 1 {
        return Some(0);
    }
    e.mem.set_u8(menu + IS_ACTIVE, 0);
    let text = match e.mem.u32(menu + CURRENT_TEXT + STR_DATA) {
        0 => EMPTY_TEXT,
        at => at,
    };
    let script = e.mem.u32(menu + SCRIPT);
    crate::script_call::call(e, script, 0, vec![text]);
    let script = e.mem.u32(menu + SCRIPT);
    unset_text_input_hooks(e, script);
    e.call(EXE_CLOSE, &[]);
    Some(0)
}

#[cfg(test)]
#[path = "jip_fn_ui_text_tests.rs"]
mod tests;
