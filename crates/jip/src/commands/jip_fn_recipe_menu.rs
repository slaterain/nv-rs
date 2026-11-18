//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE), the three recipe menu commands that rewrite the
//! game's code: `ToggleCraftingMessages` (the "item added" message after crafting),
//! `UnlockRecipeMenuQuantity` (how many the quantity menu offers) and
//! `EnableImprovedRecipeMenu` (the menu stays open after crafting, JIP's two extra buttons).
//!
//! The port has no code bytes to write: each place JIP writes is a seam in the translated recipe
//! menu (engine-fallout-interface's `recipemenu.rs`), asked while JIP is installed, and the
//! handlers answer from JIP's state: whether the messages are on (JIP reads its own byte at
//! `0x728933`), the number stored at `0x727975`, and whether the improved menu is in (the byte
//! at `0x7274E1`). Run-wide and lock-free; never saved (the code stays as written until the game
//! ends, as in JIP).

use super::Handlers;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2714, unlock_recipe_menu_quantity),
    (0x272b, enable_improved_recipe_menu),
    (0x2836, toggle_crafting_messages),
];

/// `RecipeMenu::DoClick` (`007274b0`), its switch (JIP's bound at `0x7274E1`, byte map and table
/// entry): JIP's accept hook for ids 7 and 15 while the improved menu is in.
pub const CLICK: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_74e1);
/// The count `00727920` stores first (`0x727975`, 100 in the exe).
pub const MOST: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_7975);
/// The crafted item's message in the make callback (`007284f0`, `0x728933`).
pub const MESSAGE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_8933);
/// The make callback's close of the menu (`0x728A0C`): JIP's RecipeMenuCloseHook.
pub const CLOSE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_8a0c);

/// The exe's count (`MOV [011D8EA8], 0x64`).
const EXE_MOST: u32 = 100;
/// What JIP writes for "no limit".
const NO_LIMIT: u32 = 0x7fff_ffff;

/// JIP's code bytes as state: the messages on (`0x728933` holds `0x8D`), the number at
/// `0x727975`, and the improved menu in (`0x7274E1` holds `0xF`).
pub struct RecipeMenuPatches {
    messages: AtomicBool,
    most: AtomicU32,
    improved: AtomicBool,
}

impl Default for RecipeMenuPatches {
    fn default() -> Self {
        RecipeMenuPatches {
            messages: AtomicBool::new(true),
            most: AtomicU32::new(EXE_MOST),
            improved: AtomicBool::new(false),
        }
    }
}

fn patches(e: &mut Engine) -> &RecipeMenuPatches {
    e.ext_free::<RecipeMenuPatches>()
}

/// The four seams' handlers and the state's slot (made before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<RecipeMenuPatches>();
    e.on_site(CLICK, click);
    e.on_site(MOST, most);
    e.on_site(MESSAGE, message);
    e.on_site(CLOSE, close);
}

/// The open recipe menu (`RecipeMenu::Get`, `*011D8E90`) and its words: the tile JIP's accept
/// hook tests (`+0x44`), the category (`+0x64`), the recipe list box (`+0x6C`).
const RECIPE_MENU: u32 = 0x011d_8e90;
const ACCEPT_TILE: u32 = 0x44;
const CATEGORY: u32 = 0x64;
const LIST: u32 = 0x6c;
/// `kTileValue_target`; `Tile::IsTrue` (`00A01230`, thiscall: the tile, the trait).
const TRAIT_TARGET: u32 = 0xfaf;
const TILE_IS_TRUE: u32 = 0x00a0_1230;
/// The quantity menu (`007ABA00`, cdecl: the count, the callback, the most), the count the menu
/// can make (`011D8EA8`), the make callback (`007284F0`).
const QUANTITY_MENU_CREATE: u32 = 0x007a_ba00;
const CAN_MAKE: u32 = 0x011d_8ea8;
const MAKE_CALLBACK: u32 = 0x0072_84f0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: RecipeMenuAcceptHook
// (asm), and EnableImprovedRecipeMenu's table: ids 7 and 15 to the hook, 9 to 14 to the switch's
// "nothing" (as ids 3 to 6 in the exe).
/// The click (`a`: the menu, the id), while the improved menu is in: ids 7 and 15 open the
/// quantity menu (as the exe's id 7) only while the menu's tile `+0x44` has its target trait
/// true; ids 9 to 14 do nothing. Answers 0 for those (JIP's code ran); nothing (the exe's
/// switch) for every other id, or while the improved menu is not in.
fn click(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if !patches(e).improved.load(Ordering::Relaxed) {
        return None;
    }
    let (menu, id) = (a[0], a[1] as i32);
    match id {
        7 | 15 => {
            let tile = e.mem.u32(menu.wrapping_add(ACCEPT_TILE));
            if e.call(TILE_IS_TRUE, &[tile, TRAIT_TARGET]).eax & 0xff != 0 {
                let count = e.mem.u32(CAN_MAKE);
                e.call(QUANTITY_MENU_CREATE, &[count, MAKE_CALLBACK, NO_LIMIT]);
            }
            Some(0)
        }
        9..=14 => Some(0),
        _ => None,
    }
}

/// The number stored first: 100, or what JIP last wrote.
fn most(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(patches(e).most.load(Ordering::Relaxed))
}

/// 1 (skip the message) while crafting messages are off.
fn message(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(u32::from(!patches(e).messages.load(Ordering::Relaxed)))
}

/// The player (`*011DEA3C`); the pick-up sound's name (`008ADCF0`, thiscall: the player, the
/// form, 1, 0); `MobileObject::PlaySoundByEditorName` (`00933270`, thiscall: the player, the
/// sound handle out, the name, 0, 0x121, 0); a process's slot `0x474` (an actor's process
/// `+0x68`) and `PlayerCharacter::UpdatePlayer3d` (`00950030`).
const PLAYER: u32 = 0x011d_ea3c;
const PICK_UP_SOUND_NAME: u32 = 0x008a_dcf0;
const PLAY_SOUND_BY_EDITOR_NAME: u32 = 0x0093_3270;
const PROCESS: u32 = 0x68;
const UPDATE_PLAYER_3D: u32 = 0x0095_0030;
/// `g_interfaceManager` (`*011D8A80`), its cursor tile (`+0x28`), the tile's word `+0x2C` and its
/// two words JIP clears (`+0x58`, `+0x60`).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const CURSOR: u32 = 0x28;
/// The recipe list's fill (`00727680`, thiscall: the menu, 0, the category), `SaveListState`
/// (`007312E0`), `Filter` (`00729FE0`) with the exe's filter (`00728D60`, the word DoClick
/// pushes at `0x727637`), `RestoreListState` (`0072A660`).
const FILL_LIST: u32 = 0x0072_7680;
const SAVE_LIST_STATE: u32 = 0x0073_12e0;
const FILTER: u32 = 0x0072_9fe0;
const RECIPE_FILTER: u32 = 0x0072_8d60;
const RESTORE_LIST_STATE: u32 = 0x0072_a660;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: RefreshRecipeMenu (asm).
/// The open menu's recipe list refilled in place: cleared (slot `0x1C`), filled for the menu's
/// category, its first item highlighted (slots `0x14` and `0`), refreshed (`0x10`), its state
/// saved, filtered as the category buttons do, and its state restored with the focus.
fn refresh_recipe_menu(e: &mut Engine, menu: u32) {
    let list = menu.wrapping_add(LIST);
    e.vcall(list, 0x1c, &[]);
    let category = e.mem.u32(menu.wrapping_add(CATEGORY));
    e.call(FILL_LIST, &[menu, 0, category]);
    let first = e.vcall(list, 0x14, &[0, 0]).eax;
    e.vcall(list, 0, &[first]);
    e.vcall(list, 0x10, &[]);
    e.call(SAVE_LIST_STATE, &[list]);
    e.call(FILTER, &[list, RECIPE_FILTER]);
    e.call(RESTORE_LIST_STATE, &[list, 1]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: RecipeMenuCloseHook
// (asm).
/// In place of the make callback's close (`a`: the last form chosen), while the improved menu is
/// in: with crafting messages off, the form's pick-up sound played (when it has one); then, with
/// the menu open, the player's 3D updated when its process asks (slot `0x474` with 1), the list
/// refilled, and the cursor tile's two words cleared. The menu stays open. Answers 0 (JIP's code
/// ran); nothing (the exe's close) while the improved menu is not in.
fn close(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if !patches(e).improved.load(Ordering::Relaxed) {
        return None;
    }
    let chosen = a[0];
    if !patches(e).messages.load(Ordering::Relaxed) {
        let player = e.mem.u32(PLAYER);
        let sound = e.call(PICK_UP_SOUND_NAME, &[player, chosen, 1, 0]).eax;
        if sound != 0 {
            e.with_stack(0x10, |e, handle| {
                e.call(
                    PLAY_SOUND_BY_EDITOR_NAME,
                    &[player, handle.addr(), sound, 0, 0x121, 0],
                );
            });
        }
    }
    let menu = e.mem.u32(RECIPE_MENU);
    if menu == 0 {
        return Some(0);
    }
    let player = e.mem.u32(PLAYER);
    let process = e.mem.u32(player.wrapping_add(PROCESS));
    if e.vcall(process, 0x474, &[1]).eax & 0xff != 0 {
        e.call(UPDATE_PLAYER_3D, &[player]);
    }
    refresh_recipe_menu(e, menu);
    let manager = e.mem.u32(INTERFACE_MANAGER);
    let cursor = e.mem.u32(manager.wrapping_add(CURSOR));
    if cursor != 0 {
        let node = e.mem.u32(cursor.wrapping_add(0x2c));
        if node != 0 {
            e.mem.set_u32(node.wrapping_add(0x58), 0);
            e.mem.set_u32(node.wrapping_add(0x60), 0);
        }
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_ToggleCraftingMessages_Execute.
/// `ToggleCraftingMessages [toggle]`: whether crafting shows its "item added" message (1 on);
/// a toggle other than the answer sets them on (any value but 0) or off (0).
fn toggle_crafting_messages(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let state = patches(e).messages.load(Ordering::Relaxed);
    a.set_result(e, f64::from(u8::from(state)));
    if super::jip_fn_utility::num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        if read.ok {
            let toggle = read.int(0, 0) as i32;
            if toggle != i32::from(state) {
                patches(e).messages.store(toggle != 0, Ordering::Relaxed);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_UnlockRecipeMenuQuantity_Execute.
/// `UnlockRecipeMenuQuantity [most]`: the most the recipe menu offers to make at once without a
/// crafter's limit (the exe's 100; no limit, `0x7FFFFFFF`, when not given).
fn unlock_recipe_menu_quantity(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let most = read.int(0, NO_LIMIT);
        patches(e).most.store(most, Ordering::Relaxed);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_EnableImprovedRecipeMenu_Execute.
/// `EnableImprovedRecipeMenu` (once; it cannot be undone): the recipe menu stays open after
/// crafting with its list refreshed, its make button (and JIP's button 15) acts only while its
/// tile's target trait is true, buttons 9 to 14 do nothing, and the quantity has no limit.
fn enable_improved_recipe_menu(e: &mut Engine, _: &[u32]) -> Ret {
    let p = patches(e);
    if !p.improved.load(Ordering::Relaxed) {
        p.improved.store(true, Ordering::Relaxed);
        p.most.store(NO_LIMIT, Ordering::Relaxed);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_recipe_menu_tests.rs"]
mod tests;
