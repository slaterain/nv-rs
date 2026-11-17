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
//! player is applied to the player once, not once per teammate), G110 (a location's music picks
//! the faction's combat reaction to the player, not its modifier number), G57 (an armour's
//! sounds from a plugin are found through the plugin's own master list), G130 (an archive with a
//! `Data\<name>.override` file opens as an override), and D01 (put in at JIP's deferred start:
//! a reference in a cell with form flag 0x400000 is not marked moved).
//!
//! Also fixes: G45 (a reference made in the game with flag 0x400000, and a projectile that says
//! so, is not written as an unloaded form; the flag is taken from the save; the two form id maps
//! get 0x7D63 buckets: three of its places are group GW's seams) and G52 (a global variable with
//! form flag 0x40 is not read again from a record). JIP's own: G47 (a plugin file's name hash at
//! `+0x120`, and JIP's `hasModLog` byte cleared when the file is made, which the built
//! `ModLogPrint` reads) and H22 (`TESObject::IsMarker` by JIP's set of ids).
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
        REF_FLAGS_KEPT.addr(),
        SHOULD_SAVE_UNLOADED.addr(),
        FORM_ID_MAP_BUCKETS[0].addr(),
        FORM_ID_MAP_BUCKETS[1].addr(),
        GLOBAL_LOAD_FORM.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[MOD_NAME_HASH.addr(), MOD_LOG_BYTE.addr()],
    jip_wraps: &[SET_FORM_TYPE.addr(), NPC_MODEL.addr(), IS_MARKER.addr()],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.wrap(SET_FORM_TYPE, set_form_type);
    e.wrap(NPC_MODEL, npc_model);
    e.on_site(MOD_NAME_HASH, mod_name_hash);
    e.on_site(MOD_LOG_BYTE, mod_log_byte);
    e.wrap(IS_MARKER, is_marker);
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
    e.on_site(REF_FLAGS_KEPT, ref_flags_kept);
    e.on_site(SHOULD_SAVE_UNLOADED, should_save_unloaded);
    for site in FORM_ID_MAP_BUCKETS {
        e.on_site(site, form_id_map_buckets);
    }
    e.on_site(GLOBAL_LOAD_FORM, global_load_form);
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

// ===================== GF part 3: G45, G47, G52 (fixes and JIP's own) and H22 =====================

/// G45's seams: `00484da0` (a form's flags read from the save; words: the exe's mask of the saved
/// flags a reference takes; the answer is the mask), `UnloadForm`'s call of `00849a90`
/// (`0x8497CD`, group GW's seam; words: the game, the form; the answer's low byte is "write it")
/// and the bucket counts of two form id maps (`0x845558` and `0x846FFB`, group GW's seams; the
/// answer is the count).
pub const REF_FLAGS_KEPT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0048_4e03);
pub const SHOULD_SAVE_UNLOADED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0084_97cd);
pub const FORM_ID_MAP_BUCKETS: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Jip, 0x0084_5558),
    SiteSlot::new(Owner::Jip, 0x0084_6ffb),
];
/// G47's seams (JIP's own): a plugin file opened (`0x470D56`, before its name is copied; words:
/// the file, the name) and made (`0x47079D`; words: the file).
pub const MOD_NAME_HASH: SiteSlot = SiteSlot::new(Owner::Jip, 0x0047_0d56);
pub const MOD_LOG_BYTE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0047_079d);
/// G52's seam: `TESGlobal`'s record load `005a64b0` after its type test (`0x5A64D7`; words: the
/// global; an answer leaves the record unread).
pub const GLOBAL_LOAD_FORM: SiteSlot = SiteSlot::new(Owner::Jip, 0x005a_64d7);
/// H22's wrap (JIP's own): `TESObject::IsMarker` (`0050f9e0`, `fastcall`: the form), rewritten by
/// JIP's `GetIsInternalMarkerHook`.
pub const IS_MARKER: WrapSlot = WrapSlot::new(Owner::Jip, 0x0050_f9e0);

/// The flag JIP keeps on a reference from the save (`0x400000`, the byte `0x91` of the mask made
/// `0xD1`).
const KEPT_REFERENCE_FLAG: u32 = 0x0040_0000;

/// G45 (`SafeWrite8(0x484E03, 0xD1)`): a reference's flag `0x400000` is taken from the save too.
fn ref_flags_kept(_: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(a[0] | KEPT_REFERENCE_FLAG)
}

/// `SafeWrite16(0x845558 / 0x846FFB, 0x7D63)`: the two form id maps get 0x7D63 buckets (the
/// exe's 0x13AF).
fn form_id_map_buckets(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0x7d63)
}

/// `ADDR_ReturnTrue` (`0x8D0360`): the stub JIP compares virtual slots with.
const RETURN_TRUE: u32 = 0x008d_0360;
/// A form's virtual slots JIP tests: `IsReference` (`+0xF0`), `IsProjectile` (`+0x224`), and the
/// projectile's own "save me" answer (`+0x324`).
const SLOT_IS_REFERENCE: u32 = 0xf0;
const SLOT_IS_PROJECTILE: u32 = 0x224;
const SLOT_PROJECTILE_SAVE: u32 = 0x324;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetShouldSaveUnloadedFormHook.
/// Whether `UnloadForm` writes the form: not with flag `0x4000` (`kFormFlag_DontSaveForm`);
/// any form that is not a reference, yes; a projectile, its own answer (virtual `+0x324`); a
/// reference made in the game (mod index 0xFF) with flag `0x400000`, no; any other, yes.
fn should_save_unloaded(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let form = a[1];
    if e.mem.u8(form + 9) & 0x40 != 0 {
        return Some(0);
    }
    let vtable = e.mem.u32(form);
    if e.mem.u32(vtable + SLOT_IS_REFERENCE) != RETURN_TRUE {
        return Some(1);
    }
    if e.mem.u32(vtable + SLOT_IS_PROJECTILE) == RETURN_TRUE {
        return Some(e.vcall(form, SLOT_PROJECTILE_SAVE, &args![]).u32() & 0xff);
    }
    if e.mem.u8(form + 0xf) == 0xff && e.mem.u8(form + 0xa) & 0x40 != 0 {
        return Some(0);
    }
    Some(1)
}

/// `ModInfo::nameHash` (`+0x120`) and the word holding `hasModLog` (`+0x29A`, the exe's byte and
/// JIP's `+0x29B`).
const MOD_NAME_HASH_AT: u32 = 0x120;
const MOD_LOG_WORD: u32 = 0x29a;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitModInfoNameHook.
/// A plugin file opened: its name's `StrHashCI` kept at `+0x120` (JIP's mod lookups compare it;
/// the port's `LookupModByName` hashes the name itself, so this is JIP's word in the game's
/// memory for whatever reads it). The exe's copy of the name follows (JIP's `StrCopy` copies the
/// same bytes for any name shorter than the exe's 0x104-byte limit).
fn mod_name_hash(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (file, name) = (a[0], a[1]);
    let hash = crate::containers::str_hash_ci(&nvse::table::guest_text(e, name));
    e.mem.set_u32(file + MOD_NAME_HASH_AT, hash);
    Some(hash)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x47079D, "\x66\xC7\x80\x9A\x02\x00\x00\x00\x00\x90")).
/// A plugin file made: the word at `+0x29A` cleared (the exe clears only its byte), so JIP's
/// `hasModLog` (`+0x29B`, which `ModLogPrint` reads to append or start the file's log) starts
/// clear rather than with whatever the heap held.
fn mod_log_byte(e: &mut Engine, a: &[u32]) -> Option<u32> {
    e.mem.set_u16(a[0] + MOD_LOG_WORD, 0);
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: TESGlobalLoadFormHook.
/// A global variable whose form flags have bit `0x40` (the record's "Constant" flag, as xEdit
/// names it for `GLOB`) is not read again from a record: the load answers true at once. Others:
/// no answer (the exe's load).
fn global_load_form(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if e.mem.u8(a[0] + 8) & 0x40 != 0 {
        Some(1)
    } else {
        None
    }
}

/// `s_internalMarkerIDs`: the ids JIP takes for markers (`InitContainers`, `jip_nvse.cpp`),
/// `SetInternalMarker`'s set (that command is not built: the set stays JIP's start). The 14
/// statics and the door the exe compares by address are the first 15 (`GenerateDefaultObjects`
/// gives them these ids: 1-6, 0x10, 0x12, 0x15, 0x23, 0x24, 0x32-0x34, 0x3B, Ghidra), then the
/// exe's id range 0x64..0x77.
const INTERNAL_MARKER_IDS: [u32; 35] = [
    1, 2, 3, 4, 5, 6, 0x10, 0x12, 0x15, 0x23, 0x24, 0x32, 0x33, 0x34, 0x3b, 0x64, 0x65, 0x66, 0x67,
    0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77,
];
/// The form types JIP's test names: `TESObjectACTI` (0x15), `TESObjectLIGH` (0x1E),
/// `BGSTextureSet` (4), `BGSIdleMarker` (0x30).
const ACTIVATOR: u8 = 0x15;
const LIGHT: u8 = 0x1e;
const TEXTURE_SET: u8 = 4;
const IDLE_MARKER: u8 = 0x30;
/// `"EditorMarker.NIF"` in the exe (`0x1029C94`), and a form's model path (`+0x40` for an
/// activator; its length word `+0x44` for a light).
const EDITOR_MARKER_NIF: &[u8] = b"EditorMarker.NIF";
const ACTIVATOR_MODEL_PATH: u32 = 0x40;
const MODEL_PATH_LENGTH: u32 = 0x44;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetIsInternalMarkerHook.
/// `TESObject::IsMarker` rewritten: a type above `BGSIdleMarker` no; an idle marker or a texture
/// set yes; an id in [`INTERNAL_MARKER_IDS`] yes; an activator whose model path is
/// `EditorMarker.NIF` (no case) yes; a light with an empty model path yes; else no. The same
/// answers as the exe's for every form but one of a type above 0x30 with an id 0x64..0x77 (the
/// exe: a marker, JIP: not). A null form answers no, as the exe's (JIP would read address 4).
fn is_marker(e: &mut Engine, w: &[u32]) -> Ret {
    let form = w[0];
    let answer = |yes: bool| Ret {
        eax: u32::from(yes),
        ..Ret::default()
    };
    if form == 0 {
        return answer(false);
    }
    let kind = e.mem.u8(form + 4);
    if kind > IDLE_MARKER {
        return answer(false);
    }
    if kind == IDLE_MARKER || kind == TEXTURE_SET {
        return answer(true);
    }
    if INTERNAL_MARKER_IDS.contains(&e.mem.u32(form + 0xc)) {
        return answer(true);
    }
    match kind {
        ACTIVATOR => {
            let path = e.mem.u32(form + ACTIVATOR_MODEL_PATH);
            answer(
                path != 0
                    && nvse::table::guest_text(e, path).eq_ignore_ascii_case(EDITOR_MARKER_NIF),
            )
        }
        LIGHT => answer(e.mem.u16(form + MODEL_PATH_LENGTH) == 0),
        _ => answer(false),
    }
}

#[cfg(test)]
#[path = "gf_tests.rs"]
mod tests;
