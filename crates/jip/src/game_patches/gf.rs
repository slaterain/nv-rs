//! Group GF's game patches (step 3.9: forms and records): its handlers, wraps and slot routines. Only group
//! GF's PR edits this file (and its own tests file beside it).
//!
//! **JIP's own** (`InitGamePatches`, `internal/patches_game.h` (JIP LN NVSE); in whenever JIP
//! answers "installed", whatever the row): **G48** rewrites `TESForm::SetFormType` (`004f15a0`)
//! to store the type as a whole word, clearing JIP's form flag bytes `+5`..`+7`, and for a
//! character or creature (0x3B, 0x3C) the word at `+0x104` (JIP's actor flag bytes): the game
//! never initialises those bytes, and JIP's built hooks read them, so a form made in reused
//! memory would otherwise start with stale flags. **G39** rewrites `TESNPC`'s `TESModel::GetModel`
//! (`00601c30`, reached only through the NPC's model vtable `0x104A1A4`, slot `+0x14`): the NPC's
//! own model path when one is set (only `SetNPCSkeletonModel` sets one), else the setting
//! `sNPCModelMale`'s text (JIP's sex test reads bit 0 of the word at the NPC's `+4`, the form type
//! 0x2A, so it never picks `sNPCModelFemale`; FalloutNV.esm sets `sNPCModelFemale` to
//! `Characters\_Male\Skeleton.NIF`, the exe's `sNPCModelMale`, so the text a player gets is the
//! exe's). **G40** writes the shared stub `0x401290` (`mov al, 1; ret 4`) into the same vtable's
//! slot `+0x18` (`0x104A1BC`, `TESModel::SetModelPath`, `0x489100`): an NPC record's model path is
//! no longer stored when it loads, so G39's path stays empty until `SetNPCSkeletonModel` writes
//! it. Neither changes what a player sees with FalloutNV.esm; `Get/SetNPCSkeletonModel` and
//! `SetCRESkeletonModel` (built) need them.
//!
//! **The group's fixes** (the row `NV_JIP_GAME_PATCHES`; `InitGamePatches`): G07 (grass in the
//! outer ring of the 5x5 cells too), G16 (the player's shell casings live as long as everyone's),
//! G20 (no crash when a thrown or placed weapon is fired by an actor with no process), G83 (an
//! armour's inventory icon by the player's sex), G12 (a teammate perk's entry that targets the
//! player is applied to the player once, not once per teammate).
//!
//! Threads: every handler here reads only the game's own memory and keeps no state, so any
//! thread may run it: G39 on the main thread and the loader threads (an NPC's model is asked as
//! its 3D loads), G16 and G20 wherever a weapon fires (the main thread and the AI threads), G07
//! on the main thread (the grass update), G83 on the main thread (menus), G12 on whichever thread
//! adds or removes a perk.

use super::Area;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;

/// G48's wrap: `TESForm::SetFormType` (`004f15a0`; linker-shared: other objects' "store a byte
/// at +4" calls reach it too, as in the game with JIP).
pub const SET_FORM_TYPE: WrapSlot = WrapSlot::new(Owner::Jip, 0x004f_15a0);
/// The two types whose word at `+0x104` JIP clears (`Character`, `Creature`: JIP's actor flag
/// bytes `+0x104`..`+0x107`), and that word.
const CHARACTER: u32 = 0x3b;
const CREATURE: u32 = 0x3c;
const ACTOR_JIP_FLAGS: u32 = 0x104;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x4F15A0, `movzx eax, byte [esp+4]; mov [ecx+4], eax; cmp al, 0x3B; je;
// cmp al, 0x3C; je; ret 4; xor edx, edx; mov [ecx+0x104], edx; ret 4`)).
/// G48: `004f15a0` (`ECX` the form, one stack word) rewritten: the word's low byte stored as a
/// whole word at `+4` (the type, and JIP's form flag bytes `+5`..`+7` cleared); for a character
/// or a creature also the word at `+0x104` cleared (JIP's actor flag bytes). Answers the type in
/// `EAX` as JIP's bytes do (the exe's answers the form; no caller reads it).
fn set_form_type(e: &mut Engine, w: &[u32]) -> Ret {
    let (form, kind) = (w[0], w[1] & 0xff);
    e.mem.set_u32(form.wrapping_add(4), kind);
    if kind == CHARACTER || kind == CREATURE {
        e.mem.set_u32(form.wrapping_add(ACTOR_JIP_FLAGS), 0);
    }
    Ret {
        eax: kind,
        ..Ret::default()
    }
}

/// G39's wrap: `TESNPC`'s `TESModel::GetModel` (`00601c30`).
pub const NPC_MODEL: WrapSlot = WrapSlot::new(Owner::Jip, 0x0060_1c30);
/// G40's vtable slot: `TESNPC`'s `TESModel` vtable (`0x104A1A4`) `+0x18`, and the stub JIP
/// writes there (`0x401290`: `mov al, 1; ret 4`).
pub const NPC_SET_MODEL_SLOT: u32 = 0x0104_a1bc;
pub const SET_MODEL_STUB: u32 = 0x0040_1290;

/// G83's seam: `fn_0048e730`'s call of `00480ef0` (`0x48E761`).
pub const ICON_PATH: SiteSlot = SiteSlot::new(Owner::Jip, 0x0048_e761);
/// G20's seam: `fn_00523150`'s jump with no process (`0x5248EF`).
pub const FIRE_NO_PROCESS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0052_48ef);
/// G16's seam: `ExpelShellCasing`'s player tests (`0x5251A3`).
pub const PLAYER_CASING: SiteSlot = SiteSlot::new(Owner::Jip, 0x0052_51a3);
/// G07's seam: `fn_0057d0a0`'s edge-cell tests (`0x57D3B9`).
pub const GRASS_EDGE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0057_d3b9);
/// G12's seam: `fn_005eb6a0`'s jump after the player's entry (`0x5EB79B`).
pub const PLAYER_PERK_ENTRY: SiteSlot = SiteSlot::new(Owner::Jip, 0x005e_b79b);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GF",
    fix_sites: &[
        ICON_PATH.addr(),
        FIRE_NO_PROCESS.addr(),
        PLAYER_CASING.addr(),
        GRASS_EDGE.addr(),
        PLAYER_PERK_ENTRY.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[SET_FORM_TYPE.addr(), NPC_MODEL.addr()],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.wrap(SET_FORM_TYPE, set_form_type);
    e.wrap(NPC_MODEL, npc_model);
    // Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
    // (SafeWrite32(0x104A1BC, 0x401290)). The exe's own stub, as JIP writes it (not a routine
    // of ours, so not `slots::write_slot`).
    e.mem.set_u32(NPC_SET_MODEL_SLOT, SET_MODEL_STUB);
}

fn install_fixes(e: &mut Engine) {
    e.on_site(ICON_PATH, icon_path);
    e.on_site(FIRE_NO_PROCESS, jump_taken);
    e.on_site(PLAYER_CASING, jump_taken);
    e.on_site(GRASS_EDGE, jump_taken);
    e.on_site(PLAYER_PERK_ENTRY, jump_taken);
}

/// The two settings' values (`Setting::data`, `+4` of `sNPCModelFemale` `0x11D2718` and
/// `sNPCModelMale` `0x11D3E44`) and how far JIP's sex test reaches back from the model (`0xD8`:
/// the NPC's `+4`, the model being at `+0xDC`).
pub const FEMALE_MODEL_VALUE: u32 = 0x011d_271c;
pub const MALE_MODEL_VALUE: u32 = 0x011d_3e48;
const SEX_WORD_BACK: u32 = 0xd8;
/// `TESModel::nifPath`'s characters (`String::m_data`).
const MODEL_PATH: u32 = 4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x601C30, ...): `mov eax,[ecx+4]; test eax,eax; jz; ret; sub ecx,0xD8;
// test dword [ecx],1; mov eax,0x11D3E48; mov ecx,0x11D271C; cmovnz eax,ecx; mov eax,[eax]; ret`).
/// G39: `00601c30` (`ECX` the NPC's model) rewritten: the model's own path when it has one, else
/// the female setting's text when bit 0 of the word `0xD8` before the model is set, else the
/// male one's (JIP's test; the bit is the form type's, so the male one in practice).
fn npc_model(e: &mut Engine, w: &[u32]) -> Ret {
    let model = w[0];
    let own = e.mem.u32(model.wrapping_add(MODEL_PATH));
    let eax = if own != 0 {
        own
    } else if e.mem.u32(model.wrapping_sub(SEX_WORD_BACK)) & 1 != 0 {
        e.mem.u32(FEMALE_MODEL_VALUE)
    } else {
        e.mem.u32(MALE_MODEL_VALUE)
    };
    Ret {
        eax,
        ..Ret::default()
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite16(0x57D3B9, 0x1AEB), WriteRelJump(0x5251A3, 0x52528C), SafeWrite8(0x5248EF, 0x77),
// SafeWrite8(0x5EB79B, 0x23)).
/// G07, G16, G20, G12: a jump JIP writes over the exe's code; any answer takes it.
fn jump_taken(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// `kFormType_TESObjectARMO`, the armour's `TESBipedModelForm` icons (`+0xFC`: the male icon's
/// characters at `+4`, the female icon's at `+0x10` and its length at `+0x14`), `g_thePlayer`,
/// the player's base form (`+0x20`) and its flags (`+0x34`, bit 0: female).
const FORM_TYPE_ARMO: u8 = 0x18;
const ARMO_ICONS: u32 = 0xfc;
const MALE_ICON: u32 = 4;
const FEMALE_ICON: u32 = 0x10;
const FEMALE_ICON_LENGTH: u32 = 0x14;
const THE_PLAYER: u32 = 0x011d_ea3c;
const REFERENCE_BASE: u32 = 0x20;
const BASE_FLAGS: u32 = 0x34;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetIconPathForItemHook.
/// G83: in place of `00480ef0(item, owner)`: 0 for a form that is not an armour; an armour's
/// female icon when the player's base is female and that icon is not empty, else its male icon
/// (the owner is not read). JIP reads the item, the player and its base with no null test; the
/// port answers 0 for no item (what the exe's call answers) and the male icon with no player or
/// base.
fn icon_path(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let item = a[0];
    if item == 0 || e.mem.u8(item + 4) != FORM_TYPE_ARMO {
        return Some(0);
    }
    let icons = item + ARMO_ICONS;
    let player = e.mem.u32(THE_PLAYER);
    let base = if player != 0 {
        e.mem.u32(player + REFERENCE_BASE)
    } else {
        0
    };
    if base != 0
        && e.mem.u8(base + BASE_FLAGS) & 1 != 0
        && e.mem.u16(icons + FEMALE_ICON_LENGTH) != 0
    {
        return Some(e.mem.u32(icons + FEMALE_ICON));
    }
    Some(e.mem.u32(icons + MALE_ICON))
}

#[cfg(test)]
#[path = "gf_tests.rs"]
mod tests;
