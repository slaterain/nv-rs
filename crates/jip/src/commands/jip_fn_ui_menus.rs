//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE): `ShowQuantityMenu` (the game's quantity menu with a
//! callback script, JIP's `QuantityMenuCallback`) and `CloseActiveMenu` (the top menu, or every
//! menu, closed, with what JIP keeps for the quantity and message boxes cleared). The rest of the
//! file's commands are in `jip_fn_ui.rs`.
//!
//! JIP's `QuantityMenuCallback` (asm) is called by `QuantityMenu::DoClick` (`007ABE40`) with the
//! count, and reads DoClick's own button id from the caller's frame (`[ebp+8]`): OK (4) gives the
//! script the count, Cancel (5) gives -1 (`count - (id == 5)`, where DoClick passes 0 for Cancel).
//! A count of 0 with OK is a real answer (the menu's value can be 0), so the count alone cannot
//! tell the two apart: the port's DoClick asks the seam [`QUANTITY_CALLBACK_SEAM`] (`0x7ABECC`, its
//! call of the callback; JIP's own, `INSTALLED`) with the callback, the count and the id, and the
//! handler here runs JIP's routine with them when the callback is JIP's
//! ([`QUANTITY_MENU_CALLBACK`]) and answers, so DoClick skips its own call.
//!
//! `CloseActiveMenu` on the text input menu: JIP also takes its text input hooks out there
//! (`UnsetTextInputHooks`, when `TextInputClose` is in; `jip_fn_ui_text`). JIP reads the menu's
//! script from the menu after it is destroyed (the text edit menu's word still names it); here it
//! is read before, so freed memory is never read.
//!
//! Threads: both commands and the routine run on the main thread (scripts, menus). The kept
//! script is a lock-free word made at JIP's install.

use super::{type_of, Handlers};
use crate::game_patches::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicU32, Ordering};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2825, show_quantity_menu),
    (0x28cc, close_active_menu),
];

/// `QuantityMenuCallback`, the routine the game's quantity menu is handed: at this address of
/// JIP's commands' own routines (after `MessageBoxCallback` and `DoFireWeaponEx`'s, with room
/// between).
pub const QUANTITY_MENU_CALLBACK: u32 = crate::identity::ROUTINES + 0x740;

/// The seam in `QuantityMenu::DoClick` at its call of the result callback (words: the callback,
/// the count, the button id; answered: the call was made here).
pub const QUANTITY_CALLBACK_SEAM: SiteSlot = SiteSlot::new(Owner::Jip, 0x007a_becc);

/// The commands' seams: in with JIP's `INSTALLED` (JIP's routine reads the game's frame whenever
/// JIP is in; nothing changes for the game's own callbacks).
pub const AREA: Area = Area {
    name: "commands",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[QUANTITY_CALLBACK_SEAM.addr()],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(QUANTITY_CALLBACK_SEAM, quantity_callback_seam);
}

/// JIP's `s_quantityMenuScript`: the script `ShowQuantityMenu` handed the menu, 0 for none.
#[derive(Default)]
pub struct QuantityMenuScript(AtomicU32);

/// Makes the kept script's word and registers [`QUANTITY_MENU_CALLBACK`].
pub fn install(e: &mut Engine) {
    e.ext_free::<QuantityMenuScript>();
    e.register(QUANTITY_MENU_CALLBACK, quantity_menu_callback_routine);
}

/// The kept script, for the tests.
pub fn quantity_menu_script(e: &mut Engine) -> u32 {
    e.ext_free::<QuantityMenuScript>().0.load(Ordering::Relaxed)
}

fn take_quantity_menu_script(e: &mut Engine) -> u32 {
    e.ext_free::<QuantityMenuScript>()
        .0
        .swap(0, Ordering::Relaxed)
}

// ---- the game's objects ----

/// `QuantityMenu::Get` (`*0x011DA618`) and `QuantityMenu::ShowMenu` (`0x007ABA00`, `cdecl`: the
/// count, the callback, the starting value; the menu's root tile or 0).
const QUANTITY_MENU: u32 = 0x011d_a618;
const QUANTITY_SHOW_MENU: u32 = 0x007a_ba00;
/// `g_interfaceManager` (`*0x011D8A80`): `currentMode` (+0xC), `menuStack[10]` (+0x114),
/// `renderedMenu` (+0x16C, engine-fallout-interface `pCurrentRenderedMenu`), `pipBoyMode`
/// (+0x4BC) and `pipBoyModeCallback` (+0x4C0).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const IM_CURRENT_MODE: u32 = 0x0c;
const IM_MENU_STACK: u32 = 0x114;
const IM_MENU_STACK_LEN: u32 = 10;
const IM_RENDERED_MENU: u32 = 0x16c;
const IM_PIPBOY_MODE: u32 = 0x4bc;
const IM_PIPBOY_MODE_CALLBACK: u32 = 0x4c0;
/// `FORenderedMenu::tileMenu` (+0x18) and `FORenderedMenu::Close` (`0x007FFD50`, `thiscall`).
const RENDERED_TILE_MENU: u32 = 0x18;
const RENDERED_MENU_CLOSE: u32 = 0x007f_fd50;
/// `Tile::Destroy` (vtable +0, `doFree`).
const TILE_DESTROY_SLOT: u32 = 0;
/// The message menu's clean-up JIP calls (`CdeclCall(0x7AA480)`: each message box's entry
/// released, the menu's list emptied).
const MESSAGE_MENU_CLEAR: u32 = 0x007a_a480;
/// Menu ids: `kMenuType_Min` (= `kMenuType_Message`), `kMenuType_Quantity`; 1 the Pip-Boy.
const MENU_MIN: u32 = 0x3e9;
const MESSAGE_MENU: u32 = 0x3e9;
const QUANTITY: u32 = 0x3f8;
const PIPBOY: u32 = 1;
/// `pipBoyMode` 4 (JIP's write: the Pip-Boy put away).
const PIPBOY_MODE_CLOSED: u32 = 4;
/// `kFormType_Script`.
const SCRIPT_TYPE: u8 = 0x11;
/// The Cancel button's id in `QuantityMenu::DoClick` (OK is 4).
const CANCEL: u32 = 5;

// ---- ShowQuantityMenu ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_ShowQuantityMenu_Execute.
/// `ShowQuantityMenu callback maxCount [defaultCount]`: with no quantity menu open, the arguments
/// read, `maxCount` above 0 and the callback a script, the game's quantity menu shown with
/// `maxCount`, JIP's routine and the default (`maxCount` when it is below 0 or above
/// `maxCount`); when it opens, the script is kept with its lambda variables and the result is 1.
fn show_quantity_menu(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if e.mem.u32(QUANTITY_MENU) != 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let callback = read.form(0);
    let max_count = read.int(1, 0) as i32;
    let default_count = read.int(2, u32::MAX) as i32;
    if max_count <= 0 || callback == 0 || type_of(e, callback) != SCRIPT_TYPE {
        return handled(true);
    }
    let start = if default_count < 0 || default_count > max_count {
        max_count
    } else {
        default_count
    };
    let shown = e
        .call(
            QUANTITY_SHOW_MENU,
            &[max_count as u32, QUANTITY_MENU_CALLBACK, start as u32],
        )
        .eax;
    if shown != 0 {
        e.ext_free::<QuantityMenuScript>()
            .0
            .store(callback, Ordering::Relaxed);
        nvse::lambda::save_variables(e, callback);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: QuantityMenuCallback
// (asm).
/// The menu's answer: the kept script taken out and, when there is one, called with `count`, or
/// -1 for Cancel (`count - (id == 5)`), and its lambda variables given up.
fn quantity_menu_callback(e: &mut Engine, count: u32, id: u32) {
    let script = take_quantity_menu_script(e);
    if script == 0 {
        return;
    }
    let value = count.wrapping_sub(u32::from(id == CANCEL));
    crate::script_call::call(e, script, 0, vec![value]);
    nvse::lambda::unsave_variables(e, script);
}

/// The seam's handler: JIP's routine run with DoClick's id when the callback is JIP's (answered:
/// DoClick does not call it again); another callback (the game's own) is not JIP's: no answer.
fn quantity_callback_seam(e: &mut Engine, w: &[u32]) -> Option<u32> {
    if w[0] != QUANTITY_MENU_CALLBACK {
        return None;
    }
    quantity_menu_callback(e, w[1], w[2]);
    Some(0)
}

/// The routine at [`QUANTITY_MENU_CALLBACK`], for a call that does not come through the seam:
/// the game calls it only from `DoClick`, which asks the seam first whenever JIP is installed, so
/// this is reached only by a call from elsewhere, with no button id to read: taken as OK.
fn quantity_menu_callback_routine(e: &mut Engine, w: &[u32]) -> Ret {
    quantity_menu_callback(e, w[0], 0);
    Ret::default()
}

// ---- CloseActiveMenu ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_CloseActiveMenu_Execute.
/// `CloseActiveMenu [closeAll]`: in menu mode, the menu stack walked from the top (entry 9) down,
/// empty entries passed over. A menu (id 0x3E9 and up) with a tile is closed: through
/// `FORenderedMenu::Close` when it is the rendered menu's, else its tile destroyed; the message
/// menu then cleans up (`0x7AA480`) and `MessageBoxExAlt`'s kept scripts are given up, the
/// quantity menu gives up `ShowQuantityMenu`'s, and the text edit menu, while JIP's text input
/// hooks are in, takes them out (`UnsetTextInputHooks`). The Pip-Boy (id 1) is put away (mode 4, no
/// callback). The entry is cleared; without `closeAll` only the first one found. No result.
fn close_active_menu(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let im = e.mem.u32(INTERFACE_MANAGER);
    if im == 0 || e.mem.u32(im + IM_CURRENT_MODE) <= 1 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let close_all = read.int(0, 0) != 0;
    for index in (0..IM_MENU_STACK_LEN).rev() {
        let entry = im + IM_MENU_STACK + 4 * index;
        let id = e.mem.u32(entry);
        if id == 0 {
            continue;
        }
        if id >= MENU_MIN {
            // The text input menu's script, while JIP's hooks are in: read before its menu goes.
            let text_input = (id == super::jip_fn_ui_text::TEXT_EDIT
                && e.hook_users(engine_core::extend::Hook::TextInputClose) != 0)
                .then(|| super::jip_fn_ui_text::kept_script(e));
            // `g_tileMenuArray[menuID]`; an id past the last menu has no tile here (JIP would
            // read past the array).
            let tile = super::jip_fn_ui::menu_tile_by_id(e, id);
            if tile != 0 {
                let rendered = e.mem.u32(im + IM_RENDERED_MENU);
                if rendered != 0 && e.mem.u32(rendered + RENDERED_TILE_MENU) == tile {
                    e.call(RENDERED_MENU_CLOSE, &[rendered]);
                } else {
                    e.vcall(tile, TILE_DESTROY_SLOT, &args![1u32]);
                }
            }
            if id == MESSAGE_MENU {
                e.call(MESSAGE_MENU_CLEAR, &[]);
                super::jip_fn_ui::clear_message_box_scripts(e);
            } else if id == QUANTITY {
                let script = take_quantity_menu_script(e);
                if script != 0 {
                    nvse::lambda::unsave_variables(e, script);
                }
            } else if let Some(script) = text_input {
                super::jip_fn_ui_text::unset_text_input_hooks(e, script);
            }
        } else if id == PIPBOY {
            e.mem.set_u32(im + IM_PIPBOY_MODE, PIPBOY_MODE_CLOSED);
            e.mem.set_u32(im + IM_PIPBOY_MODE_CALLBACK, 0);
        }
        e.mem.set_u32(entry, 0);
        if !close_all {
            break;
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ui_menus_tests.rs"]
mod tests;
