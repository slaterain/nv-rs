//! Group GA's game patches (step 3.9: UI: HUD, messages, text, start-up INI): its handlers, wraps and slot routines. Only group
//! GA's PR edits this file (and its own tests file beside it).
//!
//! Eight of JIP's always-on fixes (`InitGamePatches` and `InitJIPHooks`, JIP LN NVSE 57.30), each
//! a fix row (in with the row `NV_JIP_GAME_PATCHES`, [`super::set_row`]):
//! - **G08** "Increase buffer size for GetPrivateProfileString" (`SafeWrite8(0xC44405, 4)`): the
//!   INI reader's text settings (`INISettingCollection::ReadSetting`, `00c44280`) are read with a
//!   0x400-byte buffer, not 0x100, so a value longer than 255 characters is kept whole.
//! - **G11** "Increase max trait description length to 0x400 characters" (`SafeWrite8(0x7E6EA6,
//!   6)`, `SafeWrite8(0x7E6F04, 4)`, `SafeWrite16(0x7E6F09, 0xE8F9)`, `SafeWrite16(0x7E6F2B,
//!   0xE8F9)`): the trait menu's description (`TraitMenu::DoEnter`, `007e6ea0`) is built into a
//!   0x400-byte buffer, not 0x200. A longer description still ends the game, as one over
//!   0x1FF characters does in the exe (`0x5EBAC0` builds it with `vsprintf_s` and `strcat_s`,
//!   whose overflow goes to the invalid-parameter path; the game installs no handler).
//! - **G17** "TileMenu::DoDestroy fix for OnMenuClose" (`SafeWrite16(0xA1F035, 0x30EB)`): a
//!   menu tile's destructor (`00a1eff0`) no longer clears the menu's "open" byte (`0x11F308F` +
//!   id) before the menu is deleted, so what runs as the menu closes still sees it open.
//! - **G21** "NULL check for perk activation text" (`SAFE_WRITE_BUF(0x7AA80F, ...)`): an
//!   activation choice (a perk's activate entry) with no text is passed over instead of copied
//!   from a null pointer (`007aa720`).
//! - **G35** "Enable console prints in the loading menu" (`SafeWrite16(0x71D0D2, 0x1BEB)`): the
//!   console prints a line also while the loading menu is up (`MenuConsole::Print`,
//!   `0071d0a0`, no longer returns early there).
//! - **G37** "Fall back to the vanilla CharGenQuest if the INI one is invalid"
//!   (`WriteRelJump(0x7D333B, InvalidChargenQuestFixHook)`): a character creation quest in the
//!   INI that is not a quest falls back to the vanilla one (form `0x102037`) in
//!   `StartMenu::ConfirmNewGame` (`007d32b0`).
//! - **G58** (no comment; `WriteRelCall(0x70E5FE, ToggleConsoleHook)`): the console key
//!   (the grave-accent key, DIK 0x29, whatever the bindings say) works also while a controller is
//!   connected: the exe's query `0xA24660` answers 0 for it while the gamepad flag `0x11F35C8` is
//!   set, JIP's reads the key bytes alone (`InterfaceManager::Idle`, `0070c4a0`, at `0x70E5FE`).
//!   Both toggle once per press (down now, not the frame before).
//! - **H21** (`InitJIPHooks`: `WriteRelCall(0x7704AE, ClearHUDOrphanedTiles)`): every HUD frame
//!   (`HUDMainMenu::DoIdle`, `00770430`), the HUD's child tiles whose scene node lost its parent
//!   are destroyed before the exe's `0x77DA60` runs.
//!
//! Threads: every seam runs on the main thread (the UI, the INI reader); the handlers keep no
//! state and take no slot.

use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// G08's seam: the text setting's buffer size in `ReadSetting` (words: the exe's size 0x100; the
/// answer is the size).
pub const INI_STRING_BUFFER: SiteSlot = SiteSlot::new(Owner::Jip, 0x00c4_4405);
/// G11's seam: the trait description's buffer size in `TraitMenu::DoEnter` (words: the exe's
/// size 0x200; the answer is the size). One seam for JIP's four writes (frame, size, the two
/// buffer addresses).
pub const TRAIT_DESCRIPTION: SiteSlot = SiteSlot::new(Owner::Jip, 0x007e_6f04);
/// G17's seam: the tile menu destructor's clear of the menu's open byte (words: the tile, its
/// menu; any answer skips the clear).
pub const MENU_OPEN_BYTE: SiteSlot = SiteSlot::new(Owner::Jip, 0x00a1_f035);
/// G21's seam: the head of the activation choice walk (words: the list node; an answer of 1
/// passes the node over).
pub const ACTIVATION_TEXT: SiteSlot = SiteSlot::new(Owner::Jip, 0x007a_a80f);
/// G35's seam: `MenuConsole::Print`'s loading menu test (words: the console; any answer prints).
pub const CONSOLE_WHILE_LOADING: SiteSlot = SiteSlot::new(Owner::Jip, 0x0071_d0d2);
/// G37's seam: `ConfirmNewGame`'s quest test (words: the quest found, 0 for none; the answer is
/// the quest).
pub const CHARGEN_QUEST: SiteSlot = SiteSlot::new(Owner::Jip, 0x007d_333b);
/// G58's seam: the console key's query in `Idle` (words: the input globals, the key 0x1D, 1; the
/// answer is the query's).
pub const CONSOLE_KEY: SiteSlot = SiteSlot::new(Owner::Jip, 0x0070_e5fe);
/// H21's seam: `DoIdle`'s call of `0x77DA60` (words: the HUD; the answer is not read, the call
/// follows).
pub const ORPHANED_TILES: SiteSlot = SiteSlot::new(Owner::Jip, 0x0077_04ae);

/// The group's fix seams in address order.
pub const FIX_SITES: [SiteSlot; 8] = [
    CONSOLE_KEY,
    CONSOLE_WHILE_LOADING,
    ORPHANED_TILES,
    ACTIVATION_TEXT,
    CHARGEN_QUEST,
    TRAIT_DESCRIPTION,
    MENU_OPEN_BYTE,
    INI_STRING_BUFFER,
];

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GA",
    fix_sites: &[
        FIX_SITES[0].addr(),
        FIX_SITES[1].addr(),
        FIX_SITES[2].addr(),
        FIX_SITES[3].addr(),
        FIX_SITES[4].addr(),
        FIX_SITES[5].addr(),
        FIX_SITES[6].addr(),
        FIX_SITES[7].addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

fn install_fixes(e: &mut Engine) {
    e.on_site(INI_STRING_BUFFER, ini_string_buffer);
    e.on_site(TRAIT_DESCRIPTION, trait_description);
    e.on_site(MENU_OPEN_BYTE, keep_menu_open_byte);
    e.on_site(ACTIVATION_TEXT, activation_text);
    e.on_site(CONSOLE_WHILE_LOADING, console_while_loading);
    e.on_site(CHARGEN_QUEST, chargen_quest);
    e.on_site(CONSOLE_KEY, console_key);
    e.on_site(ORPHANED_TILES, clear_orphaned_tiles);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite8(0xC44405, 4): `PUSH 0x100` becomes `PUSH 0x400`).
/// G08: the size of the text setting's buffer, 0x400.
fn ini_string_buffer(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0x400)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite8(0x7E6F04, 4): `PUSH 0x200` becomes `PUSH 0x400`; the frame and buffer writes give
// the buffer its room).
/// G11: the size of the trait description's buffer, 0x400.
fn trait_description(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0x400)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite16(0xA1F035, 0x30EB): `JMP 0xA1F067`).
/// G17: the jump over the open byte's clear (taken).
fn keep_menu_open_byte(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// The activation choice's text: the item's word `+4` (`00726070`).
const ITEM_TEXT: u32 = 4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x7AA80F, "8B 01 8B 40 04 85 C0 75 03 EB 68")).
/// G21: `mov eax,[ecx]; mov eax,[eax+4]; test eax,eax; jnz 0x7AA81B; jmp 0x7AA882`: the node's
/// item's text; none (1) goes to the walk's next node, a text (0) is copied as the exe does.
fn activation_text(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let item = e.mem.u32(a[0]);
    Some(u32::from(e.mem.u32(item + ITEM_TEXT) == 0))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite16(0x71D0D2, 0x1BEB): `JMP 0x71D0EF`).
/// G35: the jump over the loading menu test (taken).
fn console_while_loading(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// The vanilla character creation quest (`InvalidChargenQuestFixHook`'s `push 0x102037`).
pub const VANILLA_CHARGEN_QUEST: u32 = 0x0010_2037;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InvalidChargenQuestFixHook.
/// G37: `[ebp-0x10]` (the quest) kept when found, else `LookupFormByRefID(0x102037)`.
fn chargen_quest(e: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(match a[0] {
        0 => crate::cosave::lookup_form(e, VANILLA_CHARGEN_QUEST),
        quest => quest,
    })
}

/// The input globals' key states: this frame's at `+0x1921`, the last frame's at `+0x1A21`
/// (`ToggleConsoleHook`'s two bytes; bit 0x80 is "down").
const KEY_NOW: u32 = 0x1921;
const KEY_BEFORE: u32 = 0x1a21;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ToggleConsoleHook.
/// G58: 0 when the key was down the frame before, else whether it is down now (`retn 8`: the
/// key and the 1 are not read).
fn console_key(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let input = a[0];
    if e.mem.u8(input + KEY_BEFORE) & 0x80 != 0 {
        return Some(0);
    }
    Some(u32::from(e.mem.u8(input + KEY_NOW) & 0x80 != 0))
}

/// The HUD's tile (`Menu::tile`, `+4`); a tile's first child node (`+4`, the list's head); a
/// node's next (`+0`) and tile (`+8`); a tile's scene node (`+0x2C`) and a node's parent
/// (`NiAVObject::m_parent`, `+0x18`).
const MENU_TILE: u32 = 4;
const TILE_CHILDREN: u32 = 4;
const NODE_NEXT: u32 = 0;
const NODE_TILE: u32 = 8;
const TILE_NODE: u32 = 0x2c;
const NODE_PARENT: u32 = 0x18;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ClearHUDOrphanedTiles.
/// H21: walks the HUD tile's children (the next node read before a tile goes); a child with a
/// scene node that has no parent is destroyed (its virtual slot 0 with 1). JIP then jumps to
/// `0x77DA60`; the seam's caller makes that call. A HUD with no tile walks nothing (JIP would
/// read address 4).
fn clear_orphaned_tiles(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let tile = e.mem.u32(a[0] + MENU_TILE);
    if tile == 0 {
        return Some(0);
    }
    let mut node = e.mem.u32(tile + TILE_CHILDREN);
    while node != 0 {
        let child = e.mem.u32(node + NODE_TILE);
        node = e.mem.u32(node + NODE_NEXT);
        if child == 0 {
            continue;
        }
        let scene = e.mem.u32(child + TILE_NODE);
        if scene == 0 || e.mem.u32(scene + NODE_PARENT) != 0 {
            continue;
        }
        e.vcall(child, 0, &args![1u32]);
    }
    Some(0)
}

#[cfg(test)]
#[path = "ga_tests.rs"]
mod tests;
