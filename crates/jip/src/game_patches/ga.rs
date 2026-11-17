//! Group GA's game patches (step 3.9: UI: HUD, messages, text, start-up INI): its handlers, wraps and slot routines. Only group
//! GA's PR edits this file (and its own tests file beside it).
//!
//! Eleven of JIP's always-on fixes (`InitGamePatches` and `InitJIPHooks`, JIP LN NVSE 57.30), each
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
//! - **G10** "Increase corner message max text length to 0x203 characters" (16 `SafeWrite8`s, each
//!   the second byte of a size or an offset of the HUD corner message record, which grows from
//!   0x310 to 0x410 bytes): the text buffer 0x104 -> 0x204 (the text copy's limit too), the icon
//!   path `+0x104` -> `+0x204`, the sound name `+0x208` -> `+0x308`, the display time `+0x30C` ->
//!   `+0x40C`; the icon and sound buffers stay 0x104 (their copies' `PUSH 0x104` are not written).
//!   Places: `fn_00775380` (`0x7753B8` size, `0x7753D2` text limit, `0x7753EA` time, `0x77540C`,
//!   `0x775443`, `0x775461`, `0x77547E`, `0x77549C` icon, `0x7754BC`, `0x7754D0` sound, `0x77553C`
//!   icon tile, `0x775600` the urgent path's time), `DoIdle` (`0x7714DA` time) and `HideMessage`
//!   (`0x7757B1`, `0x7757D3` sound, `0x77582B` icon tile). One seam per function answers the text
//!   buffer's size. One layout for a record's whole life: the fixes go in once, before the first
//!   frame ([`super::set_row`] is read at start-up; nothing takes `FIXES` out of the word); their
//!   handlers are registered before `FIXES` is set (`put_fixes_in`), and an engine made for a
//!   thread copies them, so with `FIXES` set every G10 seam answers and every record is built
//!   and read with the same layout (the HUD's layout code checks it in debug builds). In the
//!   game a text too long for the buffer fails the copy (`strcpy_s`) and the game ends: above
//!   259 characters without JIP, above 515 with it. [`corner_message_text_size`] answers the
//!   same buffer size for `MessageExAlt`.
//! - **G80** (no comment; `WriteRelJump(0x777123, RolloverWeaponHook)`): the HUD's crosshair
//!   name of a weapon lying in the world (`HUDMainMenu::SetInfoForRef`, `00775a00`) gets a "+" at
//!   its end when the weapon has a weapon mod on it (the reference's `ExtraWeaponModFlags`, kind
//!   0x8D, flags byte +0xC not 0), as the item lists show a modded weapon. JIP replaces the
//!   exe's playable test there (`0x47BCF0`: flag byte +0x100 without 0x80) with its own, which
//!   answers the same. Difference from JIP: the "+" goes in only when it fits the 0x104-byte name
//!   buffer (`0x11D9C48`) with its end; JIP writes the two bytes past a 259-character name.
//! - **G121** (no comment; `WriteRelCall(0x7ECE32, GetVATSTargetDTHook)`): V.A.T.S. shows the
//!   damage threshold of the last hit JIP's hit damage (O1) worked out (any attacker, any target;
//!   as in JIP; `s_VATSHitDT`, a negative one as 0), in place of the exe's `0x7700F0`
//!   (`VATSMenu::DoIdle`, `007ec810`); until a hit has been reckoned that way (`-FLT_MAX`) the
//!   exe's call runs.
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

/// G10's seams (words: the exe's text buffer size 0x104; the answer is the size): `DoIdle`'s
/// time read, the queueing `fn_00775380`, `HideMessage`'s next record.
pub const CORNER_MESSAGE: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x0077_14da),
    SiteSlot::new(Owner::Jip, 0x0077_53b8),
    SiteSlot::new(Owner::Jip, 0x0077_57b1),
];
/// JIP's corner message text buffer: 0x203 letters and the end.
pub const CORNER_MESSAGE_TEXT: u32 = 0x204;

/// G80's seam: `SetInfoForRef`'s weapon test (words: the weapon, the reference; the answer's low
/// byte is the test's: 1 playable, 0 not).
pub const ROLLOVER_WEAPON: SiteSlot = SiteSlot::new(Owner::Jip, 0x0077_7123);
/// G121's seam: `VATSMenu::DoIdle`'s target damage threshold (words: the player, the target;
/// the answer is the threshold's bits; none runs the exe's call).
pub const VATS_TARGET_DT: SiteSlot = SiteSlot::new(Owner::Jip, 0x007e_ce32);
/// `kXData_ExtraWeaponModFlags`.
const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8D;
/// The HUD's crosshair name buffer (0x104 bytes).
const ROLLOVER_NAME: u32 = 0x011d_9c48;

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
        CORNER_MESSAGE[0].addr(),
        CORNER_MESSAGE[1].addr(),
        CORNER_MESSAGE[2].addr(),
        ROLLOVER_WEAPON.addr(),
        VATS_TARGET_DT.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

fn install_fixes(e: &mut Engine) {
    for site in CORNER_MESSAGE {
        e.on_site(site, corner_message_text);
    }
    e.on_site(INI_STRING_BUFFER, ini_string_buffer);
    e.on_site(TRAIT_DESCRIPTION, trait_description);
    e.on_site(MENU_OPEN_BYTE, keep_menu_open_byte);
    e.on_site(ACTIVATION_TEXT, activation_text);
    e.on_site(CONSOLE_WHILE_LOADING, console_while_loading);
    e.on_site(CHARGEN_QUEST, chargen_quest);
    e.on_site(CONSOLE_KEY, console_key);
    e.on_site(ORPHANED_TILES, clear_orphaned_tiles);
    e.on_site(ROLLOVER_WEAPON, rollover_weapon);
    e.on_site(VATS_TARGET_DT, vats_target_dt);
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

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// ("Increase corner message max text length to 0x203 characters", the 16 SafeWrite8s).
/// G10: the corner message's text buffer, 0x204 bytes.
fn corner_message_text(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(CORNER_MESSAGE_TEXT)
}

/// The HUD queue's text buffer as the queue (`fn_00775380`) will size it: the exe's 0x104, or
/// what G10's seam at `0x7753B8` answers while JIP's fixes are in (0x204). The same bit and the
/// same answer the queue reads, so a caller that fits its text to this never fails the copy.
pub fn corner_message_text_size(e: &mut Engine) -> u32 {
    const EXE_TEXT: u32 = 0x104;
    if e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0 {
        e.site(CORNER_MESSAGE[1].addr(), &[EXE_TEXT])
            .unwrap_or(EXE_TEXT)
    } else {
        EXE_TEXT
    }
}

#[cfg(test)]
#[path = "ga_tests.rs"]
mod tests;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RolloverWeaponHook.
/// G80: a weapon whose flag byte +0x100 has 0x80 (not playable) answers 0 and nothing else
/// happens. Otherwise, when the reference (`[ebp+8]`) has `ExtraWeaponModFlags` with its flags
/// byte +0xC set, "+" is appended to the crosshair name (`0x11D9C48`), and the answer is 1.
/// nv-rs's own: the "+" and its end go in only within the 0x104-byte buffer (a name of 0x102
/// bytes or fewer); JIP writes them past a longer one.
fn rollover_weapon(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (weapon, reference) = (a[0], a[1]);
    if e.mem.u8(weapon + 0x100) & 0x80 != 0 {
        return Some(0);
    }
    let flags = crate::commands::extra::refr_extra(e, reference, EXTRA_WEAPON_MOD_FLAGS);
    if flags != 0 && e.mem.u8(flags + 0xC) != 0 {
        let length = e.mem.cstr(ROLLOVER_NAME).len() as u32;
        if length + 1 < 0x104 {
            e.mem.write(ROLLOVER_NAME + length, b"+\0");
        }
    }
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetVATSTargetDTHook.
/// G121: the threshold JIP's hit damage last used (`s_VATSHitDT`), see [`vats_dt_answer`].
fn vats_target_dt(e: &mut Engine, _: &[u32]) -> Option<u32> {
    vats_dt_answer(super::gc::damage::vats_hit_dt(e))
}

/// `GetVATSTargetDTHook` on the recorded threshold `dt`: `-FLT_MAX` (no hit reckoned) gives no
/// answer (JIP jumps to the exe's `0x7700F0` with the same arguments); a float with its sign
/// bit set gives 0.0 (`CMOVS`, so -0.0 too); any other its own bits.
fn vats_dt_answer(dt: f32) -> Option<u32> {
    let bits = dt.to_bits();
    if bits == 0xFF7F_FFFF {
        return None;
    }
    Some(if bits & 0x8000_0000 != 0 { 0 } else { bits })
}
