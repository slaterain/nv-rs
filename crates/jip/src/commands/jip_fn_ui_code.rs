//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE), the six menu commands that rewrite the game's code:
//! the recipe menu's `ToggleCraftingMessages` (the "item added" message after crafting),
//! `UnlockRecipeMenuQuantity` (how many the quantity menu offers) and `EnableImprovedRecipeMenu`
//! (the menu stays open after crafting, JIP's two extra buttons); `SuppressQuestMessages` (the
//! HUD's quest messages hidden for a frame); `InitItemFilter` and `SetItemFilter` (the inventory,
//! container, barter and recipe lists filtered by a text).
//!
//! The port has no code bytes to write. Each place JIP writes inside a function is a seam in the
//! translation (engine-fallout-interface's `recipemenu.rs`), asked while JIP is installed; each
//! whole function JIP replaces or jumps out of at its end, reached only by address, is a wrap,
//! doing what the exe does until its command has run. Both are put in only while JIP answers
//! "installed" ([`AREA`]; the rows in engine-core's `patch_rows/cmds.rs`). The
//! handlers answer from JIP's state: whether the messages are on (JIP reads its own byte at
//! `0x728933`), the number stored at `0x727975`, whether the improved menu is in (the byte at
//! `0x7274E1`), whether quest messages show, and the filter's text. Run-wide; never saved (the
//! code stays as written until the game ends, as in JIP).

use super::Handlers;
use crate::game_patches::Area;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2714, unlock_recipe_menu_quantity),
    (0x272b, enable_improved_recipe_menu),
    (0x272c, init_item_filter),
    (0x272d, set_item_filter),
    (0x2836, toggle_crafting_messages),
    (0x2890, suppress_quest_messages),
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

/// The state's slots (made before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<RecipeMenuPatches>();
    e.ext_free::<UiCode>();
}

/// The four seams and the six wraps: in with JIP's `INSTALLED`, as JIP's other patches (with
/// JIP not installed nothing is asked and nothing wrapped).
pub const AREA: Area = Area {
    name: "recipe menu, quest messages, item filter",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[CLICK.addr(), MOST.addr(), MESSAGE.addr(), CLOSE.addr()],
    jip_wraps: &[
        RECIPE_FILTER_FUNC.addr(),
        BARTER_FILTER_FUNC.addr(),
        CONTAINER_FILTER_FUNC.addr(),
        SHOW_QUEST_NAME.addr(),
        SHOW_QUEST_OBJECTIVE.addr(),
        INVENTORY_FILTER_FUNC.addr(),
    ],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(CLICK, click);
    e.on_site(MOST, most);
    e.on_site(MESSAGE, message);
    e.on_site(CLOSE, close);
    install_wraps(e);
}

/// The open recipe menu (`RecipeMenu::Get`, `*011D8E90`) and the tile JIP's accept hook tests
/// (`+0x44`).
const RECIPE_MENU: u32 = 0x011d_8e90;
const ACCEPT_TILE: u32 = 0x44;
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
    // JIP's `RefreshRecipeMenu`, the one translation (it reads the filter at `0x727637`).
    super::jip_fn_ui::refresh_recipe_menu(e, menu);
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

/// JIP's `s_showQuestMessages` (true at start) and `s_itemFilterString` (`None` until
/// InitItemFilter; then the text SetItemFilter gave, empty for none). Run-wide; the text behind
/// a lock (written by a command, read by the menus' filters).
pub struct UiCode {
    quest_messages: AtomicBool,
    filter: std::sync::RwLock<Option<Vec<u8>>>,
}

impl Default for UiCode {
    fn default() -> Self {
        UiCode {
            quest_messages: AtomicBool::new(true),
            filter: std::sync::RwLock::new(None),
        }
    }
}

fn ui_code(e: &mut Engine) -> &UiCode {
    e.ext_free::<UiCode>()
}

/// The quest messages' two functions (reached only by address) and the four filters JIP's
/// InitItemFilter jumps from (their ends).
pub const SHOW_QUEST_NAME: WrapSlot = WrapSlot::new(Owner::Jip, 0x0077_a480);
pub const SHOW_QUEST_OBJECTIVE: WrapSlot = WrapSlot::new(Owner::Jip, 0x0077_a5b0);
pub const RECIPE_FILTER_FUNC: WrapSlot = WrapSlot::new(Owner::Jip, 0x0072_8d60);
pub const BARTER_FILTER_FUNC: WrapSlot = WrapSlot::new(Owner::Jip, 0x0073_04b0);
pub const CONTAINER_FILTER_FUNC: WrapSlot = WrapSlot::new(Owner::Jip, 0x0075_e650);
pub const INVENTORY_FILTER_FUNC: WrapSlot = WrapSlot::new(Owner::Jip, 0x0078_2620);

/// The wraps; each does what the exe does until its command has run.
fn install_wraps(e: &mut Engine) {
    e.wrap(SHOW_QUEST_NAME, show_quest_name);
    e.wrap(SHOW_QUEST_OBJECTIVE, show_quest_objective);
    e.wrap(RECIPE_FILTER_FUNC, recipe_filter);
    e.wrap(BARTER_FILTER_FUNC, barter_filter);
    e.wrap(CONTAINER_FILTER_FUNC, container_filter);
    e.wrap(INVENTORY_FILTER_FUNC, inventory_filter);
}

/// `bShowChallengeUpdates:GamePlay` (`kINIS_bShowChallengeUpdates_GamePlay`, its value at +4).
const SHOW_CHALLENGE_UPDATES: u32 = 0x011c_b4d4 + 4;
/// The player's active quest (`+0x6B8`), a quest's flags byte (`+0x3C`: 2 completed, 0x40
/// failed), `PlayerCharacter::SetActiveQuest` (`009529D0`, thiscall: the player, the quest).
const ACTIVE_QUEST: u32 = 0x6b8;
const QUEST_FLAGS: u32 = 0x3c;
const SET_ACTIVE_QUEST: u32 = 0x0095_29d0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: ToggleQuestMessages.
/// Quest messages shown or hidden (`s_showQuestMessages` flipped), `bShowChallengeUpdates` set to
/// match (JIP's write: on again means on, whatever it was before).
pub fn toggle_quest_messages(e: &mut Engine) {
    let shown = !ui_code(e).quest_messages.load(Ordering::Relaxed);
    ui_code(e).quest_messages.store(shown, Ordering::Relaxed);
    e.mem.set_i32(SHOW_CHALLENGE_UPDATES, i32::from(shown));
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: QueueQuestMsgHook (asm).
/// `ShowQuestName` (cdecl: the quest) while quest messages are hidden: no message; a completed
/// or failed quest that is the active one stops being active, and any other quest becomes the
/// active one when there is none.
fn show_quest_name(e: &mut Engine, w: &[u32]) -> Ret {
    if ui_code(e).quest_messages.load(Ordering::Relaxed) {
        return e.call_next(SHOW_QUEST_NAME.addr(), w);
    }
    let quest = w[0];
    let player = e.mem.u32(PLAYER);
    let active = e.mem.u32(player.wrapping_add(ACTIVE_QUEST));
    if e.mem.u8(quest.wrapping_add(QUEST_FLAGS)) & 0x42 != 0 {
        if quest == active {
            e.call(SET_ACTIVE_QUEST, &[player, 0]);
        }
    } else if active == 0 {
        e.call(SET_ACTIVE_QUEST, &[player, quest]);
    }
    Ret::default()
}

/// `ShowQuestObjective` while quest messages are hidden: nothing (JIP's `ret`).
fn show_quest_objective(e: &mut Engine, w: &[u32]) -> Ret {
    if ui_code(e).quest_messages.load(Ordering::Relaxed) {
        return e.call_next(SHOW_QUEST_OBJECTIVE.addr(), w);
    }
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SuppressQuestMessages_Execute.
/// `SuppressQuestMessages`: quest messages hidden until the next pass of JIP's main loop (one
/// frame), when they are shown again; nothing while they are already hidden.
fn suppress_quest_messages(e: &mut Engine, _: &[u32]) -> Ret {
    if ui_code(e).quest_messages.load(Ordering::Relaxed) {
        toggle_quest_messages(e);
        crate::main_loop::add_native(e, crate::main_loop::Native::ToggleQuestMessages, 0);
    }
    handled(true)
}

/// `kFormType_TESRecipe`, `kFormType_BGSNote`.
const TES_RECIPE: u8 = 0x6a;
const BGS_NOTE: u8 = 0x31;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: ShouldHideItem (asm).
/// Whether the item form is left out for the filter's text: never without a form or with no
/// text; otherwise when its name (a recipe's text at `+0x1C`, a note's at `+0x4C`, another
/// item's at `+0x34`) does not have the text in it (`SubStrCI`; a form with no name text has
/// none of it).
fn should_hide_item(e: &mut Engine, form: u32) -> bool {
    if form == 0 {
        return false;
    }
    let filter = match &*ui_code(e).filter.read().unwrap_or_else(|p| p.into_inner()) {
        Some(text) if !text.is_empty() => text.clone(),
        _ => return false,
    };
    let at = match e.mem.u8(form.wrapping_add(4)) {
        TES_RECIPE => 0x1c,
        BGS_NOTE => 0x4c,
        _ => 0x34,
    };
    let text = e.mem.u32(form.wrapping_add(at));
    let name = if text == 0 {
        Vec::new()
    } else {
        nvse::table::guest_text(e, text)
    };
    !super::ln_fn_game_data::sub_str_ci(&name, &filter)
}

/// The exe's filter first (`AL`: true leaves the item out); an item it keeps is left out by the
/// text too ([`should_hide_item`] on the argument, or on an inventory entry's form at `+8`).
fn with_item_filter(e: &mut Engine, slot: WrapSlot, w: &[u32], form_of_entry: bool) -> Ret {
    let ret = e.call_next(slot.addr(), w);
    if ret.eax & 0xff != 0 {
        return ret;
    }
    let mut form = w[0];
    if form_of_entry && form != 0 {
        form = e.mem.u32(form.wrapping_add(8));
    }
    Ret {
        eax: u32::from(should_hide_item(e, form)),
        ..Ret::default()
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: RecipeFilterHook (asm).
/// The recipe menu's filter (cdecl: the recipe).
fn recipe_filter(e: &mut Engine, w: &[u32]) -> Ret {
    with_item_filter(e, RECIPE_FILTER_FUNC, w, false)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: ItemFilterHook (asm).
/// The barter menu's filter (cdecl: the inventory entry).
fn barter_filter(e: &mut Engine, w: &[u32]) -> Ret {
    with_item_filter(e, BARTER_FILTER_FUNC, w, true)
}

/// The container menu's filter, as the barter menu's.
fn container_filter(e: &mut Engine, w: &[u32]) -> Ret {
    with_item_filter(e, CONTAINER_FILTER_FUNC, w, true)
}

/// The inventory menu's filter, as the barter menu's.
fn inventory_filter(e: &mut Engine, w: &[u32]) -> Ret {
    with_item_filter(e, INVENTORY_FILTER_FUNC, w, true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_InitItemFilter_Execute.
/// `InitItemFilter` (once): the inventory, container, barter and recipe menus' lists filtered by
/// SetItemFilter's text from now on (none yet).
fn init_item_filter(e: &mut Engine, _: &[u32]) -> Ret {
    let mut filter = ui_code(e).filter.write().unwrap_or_else(|p| p.into_inner());
    if filter.is_none() {
        *filter = Some(Vec::new());
    }
    drop(filter);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetItemFilter_Execute.
/// `SetItemFilter ["text"]` (after InitItemFilter): the lists keep only items whose names have
/// the text in them, without case; no text (or none given) keeps all. Before InitItemFilter,
/// nothing.
///
/// nv-rs's own: JIP reads the text into its 0x100-byte buffer, so a longer text writes past it
/// (memory corruption in the game); the port keeps the whole text.
fn set_item_filter(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let started = ui_code(e)
        .filter
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .is_some();
    if !started {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let text = if read.ok { read.text(0) } else { Vec::new() };
    *ui_code(e).filter.write().unwrap_or_else(|p| p.into_inner()) = Some(text);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ui_code_tests.rs"]
mod tests;
