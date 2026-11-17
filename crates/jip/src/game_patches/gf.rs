//! Group GF's game patches (step 3.9: forms and records): its handlers, wraps and slot routines. Only group
//! GF's PR edits this file (and its own tests file beside it).
//!
//! **JIP's own** (`InitGamePatches`, `internal/patches_game.h` (JIP LN NVSE); in whenever JIP
//! answers "installed", whatever the row): **G39** rewrites `TESNPC`'s `TESModel::GetModel`
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
//! player is applied to the player once, not once per teammate), G110 (a location's music picks
//! the faction's combat reaction to the player, not its modifier number), G57 (an armour's
//! sounds from a plugin are found through the plugin's own master list), G130 (an archive with a
//! `Data\<name>.override` file opens as an override), and D01 (put in at JIP's deferred start:
//! a reference in a cell with form flag 0x400000 is not marked moved).
//!
//! G45 (temporary forms not saved as unloaded forms) is not here: three of its four places are
//! in group GW's files (`bgssaveloadgame.rs`, `run_00845230.rs`), which have no seams yet.
//!
//! Threads: every handler here reads only the game's own memory and keeps no state, so any
//! thread may run it: G39 on the main thread and the loader threads (an NPC's model is asked as
//! its 3D loads), G16 and G20 wherever a weapon fires (the main thread and the AI threads), G07
//! on the main thread (the grass update), G83 on the main thread (menus), G12 on whichever thread
//! adds or removes a perk, G110 on the main thread (the music update), G57 on the loader (form
//! init), D01 wherever a reference is changed. G130 keeps the `.override` names' hashes, read
//! once as the fixes go in (before the threads start) and cleared at the deferred start, both
//! on the main thread, where the archives open too (the data load).

use super::slots::{self, Group};
use super::Area;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;

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

/// G110's seam: `fn_00595600`'s call of `005fd3b0` (`0x5956BC`).
pub const FACTION_REACTION: SiteSlot = SiteSlot::new(Owner::Jip, 0x0059_56bc);
/// G57's seams: `TESObjectARMO::InitItem`'s two calls of `004839c0` (`0x512F3D`, the sound
/// template; `0x512FAF`, each sound entry).
pub const ARMOR_SOUND_TEMPLATE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0051_2f3d);
pub const ARMOR_SOUND_ENTRY: SiteSlot = SiteSlot::new(Owner::Jip, 0x0051_2faf);
/// G130's seam: `fn_00463070`'s call of `ArchiveManager::OpenArchive` (`0x463855`).
pub const ARCHIVE_OVERRIDE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0046_3855);
/// D01's vtable slot: `TESObjectREFR`'s vtable (`0x102F55C`) `+0x48` (held `0x484B60`,
/// `TESForm::MarkAsModified`), and the routine of ours JIP's `MarkRefAsModifiedHook` is.
pub const REFR_MARK_MODIFIED_SLOT: u32 = 0x0102_f5a4;
pub const MARK_MODIFIED_ROUTINE: u32 = slots::routine(Group::GF, 0);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GF",
    fix_sites: &[
        ARCHIVE_OVERRIDE.addr(),
        ICON_PATH.addr(),
        ARMOR_SOUND_TEMPLATE.addr(),
        ARMOR_SOUND_ENTRY.addr(),
        FIRE_NO_PROCESS.addr(),
        PLAYER_CASING.addr(),
        GRASS_EDGE.addr(),
        FACTION_REACTION.addr(),
        PLAYER_PERK_ENTRY.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[NPC_MODEL.addr()],
    install_jip,
};

fn install_jip(e: &mut Engine) {
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
    e.on_site(FACTION_REACTION, faction_reaction);
    e.on_site(ARMOR_SOUND_TEMPLATE, armor_sound_form);
    e.on_site(ARMOR_SOUND_ENTRY, armor_sound_form);
    read_override_archives(e);
    e.on_site(ARCHIVE_OVERRIDE, archive_override);
}

/// JIP's `DeferredInit` (xNVSE's deferred start message, once per run), group GF's part: D01's
/// slot and G130's names dropped. Only with the fixes in (the word's bit).
pub fn deferred_init(e: &mut Engine) {
    if e.jip_patches() & engine_core::extend::jip_patches::FIXES == 0 {
        return;
    }
    // Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit
    // (SafeWrite32(0x102F5A4, MarkRefAsModifiedHook), s_overrideBSAFiles->Destroy()).
    // Always mapped in the game; a test engine without the exe's data has no vtable.
    if e.mem.is_mapped(REFR_MARK_MODIFIED_SLOT)
        && e.mem.u32(REFR_MARK_MODIFIED_SLOT) != MARK_MODIFIED_ROUTINE
    {
        slots::write_slot(
            e,
            REFR_MARK_MODIFIED_SLOT,
            MARK_MODIFIED_ROUTINE,
            mark_ref_as_modified,
        );
    }
    e.ext::<OverrideArchives>().hashes.clear();
}

/// `0x84A690` (`BGSSaveLoadGame`'s add-change, `thiscall`: the form, the flags, a last word),
/// `g_BGSSaveLoadGame`, the change flag JIP tests (`4`: moved) and the cell flag (`0x400000`, the
/// byte `+0xA` of the form flags `+8`, bit `0x40`) and `TESObjectREFR::parentCell` (`+0x40`).
const ADD_CHANGE: u32 = 0x0084_a690;
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
const CHANGE_MOVED: u32 = 4;
const CELL_FLAG_BYTE: u32 = 0xa;
const CELL_FLAG_BIT: u8 = 0x40;
const PARENT_CELL: u32 = 0x40;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: MarkRefAsModifiedHook
// (MARK_MODIFIED, internal/utility.h: `push 0; push flag; push form; mov ecx, g_BGSSaveLoadGame;
// call 0x84A690`).
/// D01: in `TESObjectREFR`'s slot `+0x48` (`thiscall`, the reference and the change flags): the
/// moved flag (exactly 4) on a reference whose parent cell has the form flag 0x400000 is not
/// recorded; every other change is passed to the save's add-change with a last word of 0 (what
/// the slot's `0x484B60` does).
fn mark_ref_as_modified(e: &mut Engine, a: &[u32]) -> Ret {
    let (reference, flag) = (a[0], a[1]);
    if flag == CHANGE_MOVED {
        let cell = e.mem.u32(reference + PARENT_CELL);
        if cell != 0 && e.mem.u8(cell + CELL_FLAG_BYTE) & CELL_FLAG_BIT != 0 {
            return Ret::default();
        }
    }
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    e.call(ADD_CHANGE, &args![save_load, reference, flag, 0u32]);
    Ret::default()
}

/// `0x48C1B0` (`TESReactionForm`'s group combat reaction towards a form, the entry's `+8`;
/// `thiscall`) and where a faction's reaction form is (`+0x24`).
const COMBAT_REACTION: u32 = 0x0048_c1b0;
const FACTION_REACTIONS: u32 = 0x24;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetFactionReactionHook
// (`add ecx, 0x24; jmp 0x48C1B0`).
/// G110: in place of `005fd3b0(faction, other)` (the entry's modifier, `0x48BF50`): the entry's
/// group combat reaction (`0x48C1B0` on the faction's reaction form).
fn faction_reaction(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (faction, other) = (a[0], a[1]);
    Some(
        e.call(
            COMBAT_REACTION,
            &args![faction.wrapping_add(FACTION_REACTIONS), other],
        )
        .u32(),
    )
}

/// `0x484E60` (`thiscall`: a form's file by index, `-1` the last one that has it), `0x485D50`
/// (`cdecl`: resolves the form id at the first word through the file's master list, in place).
const FORM_FILE: u32 = 0x0048_4e60;
const RESOLVE_FORM_ID: u32 = 0x0048_5d50;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitArmorFormHook
// (`0x484E60(armour, -1)`, `0x485D50(&id, file)`, `LookupFormByRefID(id)`).
/// G57: in place of `004839c0(id)` (asked with the id and the armour): the id resolved through
/// the master list of the last file that has the armour, then looked up in the game's form map
/// (JIP's `LookupFormByRefID`; 0 for none).
fn armor_sound_form(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (id, armour) = (a[0], a[1]);
    let file = e.call(FORM_FILE, &args![armour, u32::MAX]).u32();
    let id = e.with_stack(4, |e, at| {
        e.mem.set_u32(at.addr(), id);
        e.call(RESOLVE_FORM_ID, &args![at, file]);
        e.mem.u32(at.addr())
    });
    Some(crate::cosave::lookup_form(e, id))
}

/// G130's names: the `StrHashCI` hash of `Data\<name>.bsa` for each `Data\<name>.override` file
/// (JIP's `s_overrideBSAFiles`, a set of `const char*` keyed by that hash alone).
#[derive(Default)]
pub(crate) struct OverrideArchives {
    hashes: Vec<u32>,
}

/// The game's `FindFirstFileA`, `FindNextFileA`, `FindClose` imports, the find data's size, its
/// attributes (`+0`) and name (`+0x2C`), and `FILE_ATTRIBUTE_DIRECTORY`.
const FIND_FIRST_FILE_A: u32 = 0x00fd_f070;
const FIND_NEXT_FILE_A: u32 = 0x00fd_f068;
const FIND_CLOSE: u32 = 0x00fd_f064;
const FIND_DATA_SIZE: u32 = 0x140;
const FIND_DATA_NAME: u32 = 0x2c;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const INVALID_HANDLE_VALUE: u32 = u32::MAX;
const OVERRIDE_SEARCH: &[u8] = b"Data\\*.override";

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches (the
// `DirectoryIterator("Data\\*.override")` loop) and internal/utility.h: DirectoryIterator.
/// G130's names, read as the fixes go in: each file (not a folder) the game's `FindFirstFileA`
/// finds for `Data\*.override`, its name's last 8 characters (`override`) replaced by `bsa`
/// after `Data\`. JIP puts its call in only when it found one; the port's handler is in either
/// way and answers 0 with none, the word the exe passes.
fn read_override_archives(e: &mut Engine) {
    // Always there in the game; a test engine without the exe's imports has no files.
    if !e.is_registered(FIND_FIRST_FILE_A) {
        return;
    }
    let mut hashes = Vec::new();
    e.with_stack(FIND_DATA_SIZE, |e, data| {
        let data = data.addr();
        let handle = crate::commands::with_guest_text(e, OVERRIDE_SEARCH, |e, p| {
            e.call(FIND_FIRST_FILE_A, &args![p, data]).u32()
        });
        if handle == INVALID_HANDLE_VALUE {
            return;
        }
        loop {
            if e.mem.u32(data) & FILE_ATTRIBUTE_DIRECTORY == 0 {
                let name = e.mem.cstr(data + FIND_DATA_NAME);
                hashes.push(crate::containers::str_hash_ci(&archive_name(&name)));
            }
            if e.call(FIND_NEXT_FILE_A, &args![handle, data]).u32() == 0 {
                break;
            }
        }
        e.call(FIND_CLOSE, &args![handle]);
    });
    e.ext::<OverrideArchives>().hashes = hashes;
}

/// `memcpy(StrCopy(dataPath + 5, name) - 8, "bsa", 4)` with `dataPath` = `Data\`: the archive's
/// path for an `.override` file's name.
pub(crate) fn archive_name(name: &[u8]) -> Vec<u8> {
    let mut path = b"Data\\".to_vec();
    path.extend_from_slice(name);
    let keep = path.len().saturating_sub(8);
    path.truncate(keep);
    path.extend_from_slice(b"bsa");
    path
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: LoadBSAFileHook.
/// G130: the open's "is an override" word: whether the archive's path (`DATA\<name>.bsa`) is
/// one of the names (by `StrHashCI`).
fn archive_override(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let path = e.mem.cstr(a[0]);
    let hash = crate::containers::str_hash_ci(&path);
    Some(u32::from(
        e.ext::<OverrideArchives>().hashes.contains(&hash),
    ))
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
