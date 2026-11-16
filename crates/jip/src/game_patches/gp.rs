//! Group GP's game patches (step 3.9: player character): its handlers, wraps and slot routines. Only group
//! GP's PR edits this file (and its own tests file beside it).
//!
//! **G38 + H09: JIP's "update 3D" and the queued references' attach** (`InitGamePatches`,
//! `internal/patches_game.h`: `GeneratePlayerNodeHook`, `DoQueuedPlayerHook`, the jump at
//! `0x94E5F6`; `InitJIPHooks`, `internal/hooks.h`: `DoQueuedReferenceHook`; JIP LN NVSE), patches
//! of JIP's own (in whenever JIP answers "installed", every preset):
//! - the player's first-person model is a copy of the model loader's cached model
//!   (`0x94E4A1`), and its load no longer sets the eye height from the camera node (`0x94E5F6`);
//! - JIP writes its own attach into the `Attach` slot (`+0x3C`) of four queued-reference
//!   vtables: `QueuedReference` (`0x1016BE0`), `QueuedCharacter` (`0x1016D28`),
//!   `QueuedCreature` (`0x1016D70`) and `QueuedPlayer` (`0x1016DB8`). `QueuedActor` and `QueuedTree`
//!   keep the exe's. JIP's attach is the exe's `QueuedReference::Attach` (`0x440BA0`) rewritten
//!   (a cell is loaded when its state is 2 or more, a 3D is critical when the reference's slot
//!   `+0x100` is not the shared `return true`), and with the 3D in place it reads JIP's
//!   `Update3D` mark (`kHookRefFlag5F_Update3D`, set by JIP's `Update3D` and its other commands,
//!   `crate::overrides::ref_update_3d`): the mark goes and a fade node's fade is set back to 1.
//!   For the player the mark first throws the old models away (third person 3D, first-person
//!   biped and animation, the first-person node from its parent, the four camera node globals,
//!   the `0x60` bytes at `+0xD74`), and after the load shows the view's node, hides the other
//!   and puts the weapon back on both skeletons.
//!
//! Every slot is a slot write (`slots::write_slot`), not a wrap: `0x440BA0` is also in two
//! vtables JIP leaves alone and called directly by `0x441800`, which `0x441C30` calls (the
//! only reference to `0x441C30` is its slot: `get_xrefs_to`, `measure/jip/GROUP-GP-NOTES.md`).
//!
//! Waiting (no reader or writer built): the attach's three other steps, collision turned off
//! for a reference marked by `ToggleObjectCollision` (`kHookRefFlag5F_DisableCollision`), JIP's
//! inserted nodes and models (`InsertNode`, `AttachModel`: `s_insertObjects`) and JIP's point
//! lights (`AddPointLights`, for a node JIP's light hooks flagged `0x20000000`): none of those
//! commands or hooks is built, so nothing sets what they test. The first-person node JIP keeps
//! (`s_pc1stPersonNode`) is kept here ([`first_person_node`]) for `AttachLine` and kin.
//!
//! **Fix rows** (`InitGamePatches`; in with the row `NV_JIP_GAME_PATCHES`, `jip_patches::FIXES`):
//! - G26 "Ignore disabled MusicMarkers" (`0x9699C5`): the closest music marker passes a
//!   disabled marker over, and reads a marker's position at `+0x30`;
//! - G72 (`0x947DD2`, `BuildMusicMarkerListHook`): the player's music markers come from JIP's
//!   cache of the world space's markers, sorted by their media location controller;
//! - G105 (`0x93B87F`, `DamagePCFatigueMinFixHook`): fatigue damage to the player stops at
//!   `fMinFatigue`.
//!
//! **JIP's own** (in whenever JIP answers "installed"):
//! - G114 (`0x969C41`, `ModHardcoreNeedsHook`): with the switch `bHardcoreNeedsFix` (O6) the
//!   hardcore needs take their pending changes, never below 0; `ToggleHardcoreTracking 0` keeps
//!   the hardcore timers resetting (no needs build up);
//! - G122 (slot `0x108A9B4`, `PlayerCannotBeHitHook`): immortal mode 2 (`ToggleImmortalMode 2`)
//!   makes the player unhittable as god mode does; mode 1 keeps him at 1 health (the
//!   PlayerMinHealth hook, `crate::world_hooks`). The hit effects JIP turns off in mode 2 are
//!   the hit effect patches' (G123 to G125): [`disable_hit_effects`] keeps the word they read.
//!
//! Seams in GP's translations for other groups' rows (no handler here): D04's four dead zones
//! in the free camera (GB), G06's ash pile test in the grab (GH2), G75's hotkey clear in the
//! player's pick-up (GH1).

use super::slots::{self, Group};
use super::Area;
use crate::Jip;
use engine_core::extend::{jip_patches, Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// G38's seams in `PlayerCharacter::Load3D`: the first-person model's load, the eye height.
pub const FIRST_PERSON_MODEL: SiteSlot = SiteSlot::new(Owner::Jip, 0x0094_e4a1);
pub const EYE_HEIGHT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0094_e5f6);

/// The `Attach` slots JIP writes `DoQueuedReferenceHook` into (H09), and the player's (G38).
pub const QUEUED_REFERENCE_SLOTS: [u32; 3] = [0x0101_6be0, 0x0101_6d28, 0x0101_6d70];
pub const QUEUED_PLAYER_SLOT: u32 = 0x0101_6db8;
/// The routines written there.
pub const QUEUED_REFERENCE_ROUTINE: u32 = slots::routine(Group::GP, 0);
pub const QUEUED_PLAYER_ROUTINE: u32 = slots::routine(Group::GP, 1);

/// G105's seam in the player's float actor value change (`0093b7a0` at `0x93B87F`): the actor
/// value, the value read before the change and the change (`f32` bits).
pub const FATIGUE_FLOOR: SiteSlot = SiteSlot::new(Owner::Jip, 0x0093_b87f);
/// G72's seam in the music marker list's rebuild (`00947d80` at `0x947DD2`): the world space
/// and its marker list.
pub const MUSIC_MARKER_LIST: SiteSlot = SiteSlot::new(Owner::Jip, 0x0094_7dd2);
/// G26's seam in the closest music marker's search (`00969930` at `0x9699C5`): the marker's
/// reference.
pub const MUSIC_MARKER: SiteSlot = SiteSlot::new(Owner::Jip, 0x0096_99c5);
/// G114's seam in `PlayerCharacter::UpdateHardcoreMode` (`00969c30` at `0x969C41`): the player.
pub const HARDCORE_NEEDS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0096_9c41);
/// G122: the player's `MagicTarget` vtable slot JIP writes `PlayerCannotBeHitHook` into (it
/// held `0x952720`, reached only through this slot: `get_xrefs_to`), and the routine written
/// there.
pub const CANNOT_BE_HIT_SLOT: u32 = 0x0108_a9b4;
pub const CANNOT_BE_HIT_ROUTINE: u32 = slots::routine(Group::GP, 2);

/// The commands GP builds with its patches (`functions_jip/jip_fn_miscellaneous.h`).
pub const TOGGLE_HARDCORE_TRACKING: u32 = 0x2837;
pub const TOGGLE_IMMORTAL_MODE: u32 = 0x28ce;
#[rustfmt::skip]
pub static COMMANDS: crate::commands::Handlers = &[
    (TOGGLE_HARDCORE_TRACKING, toggle_hardcore_tracking),
    (TOGGLE_IMMORTAL_MODE, toggle_immortal_mode),
];

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GP",
    fix_sites: &[
        FATIGUE_FLOOR.addr(),
        MUSIC_MARKER_LIST.addr(),
        MUSIC_MARKER.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[
        FIRST_PERSON_MODEL.addr(),
        EYE_HEIGHT.addr(),
        HARDCORE_NEEDS.addr(),
    ],
    jip_wraps: &[],
    install_jip,
};

/// `kHookRefFlag5F_Update3D` (`internal/hooks.h`).
pub const REF_FLAG_UPDATE_3D: u8 = 1 << 0;

/// The game's globals: the model loader (`*0x011C3B3C`: its model map at `+0`, the object
/// told about attached references at `+8`), `TES` (`*0x011DEA10`), the first-person model path
/// (the text of the setting at `0x011CDD78`), the four first-person camera node globals
/// (`0x011E07CC`).
const MODEL_LOADER: u32 = 0x011c_3b3c;
const TES: u32 = 0x011d_ea10;
const FIRST_PERSON_PATH: u32 = 0x011c_dd7c;
const CAMERA_NODES: u32 = 0x011e_07cc;
/// The shared `return this` and `return true` (JIP's `ADDR_ReturnThis`, `ADDR_ReturnTrue`).
const RETURN_THIS: u32 = 0x0068_15c0;
const RETURN_TRUE: u32 = 0x008d_0360;
/// The game's functions JIP calls: the model map's find (`0x4493D0`, `thiscall`: the path, the
/// model out), the cloning process's constructor and destructor (`0x4AD0C0`, `0x4AD1D0`),
/// `ModelLoader::LoadAddonNodes` (`0x4489B0`), `TES`'s load of a reference (`0x451EF0`), the
/// attached-reference call (`0x9062E0`, on the loader's `+8`), the cell's two queued counts
/// (`0x440A10`: `+0xA4`, `0x440A50`: `+0xA0`), the recursive lock (`0x40FBF0`, `0x40FBA0`),
/// `TESObjectREFR::Set3D` (`0x5702E0`), the biped's and the animation's deleting destructors
/// (`0x418E00`, `0x418D20`), `MiddleHighProcess::ReparentWeapon` (`0x923960`).
const MODEL_MAP_FIND: u32 = 0x0044_93d0;
const CLONING_CONSTRUCT: u32 = 0x004a_d0c0;
const CLONING_DESTRUCT: u32 = 0x004a_d1d0;
const LOAD_ADDON_NODES: u32 = 0x0044_89b0;
const TES_LOAD_REFERENCE: u32 = 0x0045_1ef0;
const REFERENCE_ATTACHED: u32 = 0x0090_62e0;
const QUEUED_COUNT_DOWN: u32 = 0x0044_0a10;
const CRITICAL_COUNT_DOWN: u32 = 0x0044_0a50;
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
const SET_3D: u32 = 0x0057_02e0;
const BIPED_DELETE: u32 = 0x0041_8e00;
const ANIMATION_DELETE: u32 = 0x0041_8d20;
const REPARENT_WEAPON: u32 = 0x0092_3960;
/// JIP's cloning process's buckets (`NiObjectCopyInfo(0x97)`) and its size.
const CLONING_BUCKETS: u32 = 0x97;
const CLONING_SIZE: u32 = 0x1c;
/// The thread's memory context word (`0x404F50` reads it).
const TLS_MEMORY_CONTEXT: u32 = 0x2b4;
/// `fMinFatigue`'s value (the setting at `0x011D2664`; its value at +4, what `0x403E20`
/// answers).
const MIN_FATIGUE: u32 = 0x011d_2668;
/// `kAVCode_Fatigue`.
const FATIGUE: u32 = 0x16;
/// `kXData_ExtraAudioMarker`, the exe's `BaseExtraList::GetByType`.
const EXTRA_AUDIO_MARKER: u32 = 0x5a;
const GET_BY_TYPE: u32 = 0x0041_0220;
/// The player singleton (`*011DEA3C`) and its music marker list (`tList<MusicMarker>`, +0x7D4).
const PLAYER: u32 = 0x011d_ea3c;
const PLAYER_MUSIC_MARKERS: u32 = 0x7d4;
/// The god mode setting's value byte (the setting at `0x011E0894`, its value at +4) and the god
/// mode byte.
const GOD_MODE_SETTING_VALUE: u32 = 0x011e_0898;
const GOD_MODE_BYTE: u32 = 0x011e_07ba;
/// `kSerializedFlag_NoHardcoreTracking` (JIP's co-save flags).
const NO_HARDCORE_TRACKING: u32 = 1;

/// The offsets JIP reads (`QueuedReference`, `TESObjectREFR`, `TESObjectCELL`, `NiAVObject`,
/// `Model`, `PlayerCharacter`, `MiddleHighProcess`; JIP LN NVSE's names).
pub(crate) mod off {
    pub const TASK_STATE: u32 = 0x0c;
    pub const TASK_CONTEXT: u32 = 0x18;
    pub const TASK_REF: u32 = 0x28;
    pub const TASK_MODEL: u32 = 0x30;
    pub const FORM_FLAGS: u32 = 0x08;
    pub const PARENT_CELL: u32 = 0x40;
    pub const REF_FLAGS_5F: u32 = 0x5f;
    pub const LOADED_DATA: u32 = 0x64;
    pub const PROCESS: u32 = 0x68;
    pub const LOADED_3D: u32 = 0x14;
    pub const CELL_LOCK: u32 = 0x80;
    pub const CELL_STATE: u32 = 0x26;
    pub const MODEL_REFS: u32 = 0x08;
    pub const MODEL_NODE: u32 = 0x0c;
    pub const NI_PARENT: u32 = 0x18;
    pub const NI_FLAGS: u32 = 0x30;
    pub const NI_LOCAL_ROTATE: u32 = 0x34;
    pub const FADE: u32 = 0xb4;
    pub const VT_IS_FADE_NODE: u32 = 0x10;
    pub const VT_CREATE_CLONE: u32 = 0x48;
    pub const VT_PROCESS_CLONE: u32 = 0x68;
    pub const VT_DETACH_CHILD: u32 = 0xe8;
    pub const VT_3D_NOT_CRITICAL: u32 = 0x100;
    pub const BIPED_3RD: u32 = 0x1b4;
    pub const THIRD_PERSON: u32 = 0x64a;
    pub const BIPED_1ST: u32 = 0x68c;
    pub const ANIM_DATA_1ST: u32 = 0x690;
    pub const NODE_1ST: u32 = 0x694;
    pub const PLAYER_D74: u32 = 0xd74;
    pub const PROCESS_LEVEL: u32 = 0x28;
    pub const PROCESS_WEAPON: u32 = 0x114;
    pub const PROCESS_135: u32 = 0x135;
    pub const PROCESS_ANIM_DATA: u32 = 0x1c0;
}

/// JIP's `s_pc1stPersonNode`: the player's first-person node G38 made last (0: none yet).
#[derive(Default)]
pub struct FirstPersonNode(AtomicU32);

/// The player's first-person node JIP's copy made last (`s_pc1stPersonNode`), 0 for none.
pub fn first_person_node(e: &mut Engine) -> u32 {
    e.ext_free::<FirstPersonNode>().0.load(Ordering::Relaxed)
}

fn install_jip(e: &mut Engine) {
    // Made before the threads start, so every thread's engine shares it.
    e.ext_free::<FirstPersonNode>();
    e.on_site(FIRST_PERSON_MODEL, first_person_model);
    e.on_site(EYE_HEIGHT, eye_height);
    for slot in QUEUED_REFERENCE_SLOTS {
        slots::write_slot(e, slot, QUEUED_REFERENCE_ROUTINE, queued_reference_routine);
    }
    slots::write_slot(
        e,
        QUEUED_PLAYER_SLOT,
        QUEUED_PLAYER_ROUTINE,
        queued_player_routine,
    );
    e.ext_free::<Immortal>();
    e.on_site(HARDCORE_NEEDS, hardcore_needs);
    slots::write_slot(
        e,
        CANNOT_BE_HIT_SLOT,
        CANNOT_BE_HIT_ROUTINE,
        cannot_be_hit_routine,
    );
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiAVObject::CreateCopy.
/// A clone of `object` (its `CreateClone` and `ProcessClone` with a cloning process of 0x97
/// buckets and scale 1), shown (flag bit 0 cleared) with an identity rotation and no
/// translation. JIP keeps one cloning process a thread and empties its maps after; the port
/// makes one for the call and destroys it (the same empty maps for the game).
pub fn create_copy(e: &mut Engine, object: u32) -> u32 {
    e.with_stack(CLONING_SIZE, |e, info| {
        let info = info.addr();
        e.call(CLONING_CONSTRUCT, &args![info, CLONING_BUCKETS]);
        for i in 0..3 {
            e.mem.set_f32(info + 0x10 + 4 * i, 1.0);
        }
        let copy = e.vcall(object, off::VT_CREATE_CLONE, &args![info]).u32();
        e.vcall(object, off::VT_PROCESS_CLONE, &args![info]);
        let flags = e.mem.u8(copy + off::NI_FLAGS);
        e.mem.set_u8(copy + off::NI_FLAGS, flags & 0xfe);
        // Three 16-byte stores of (1, 0, 0, 0): the rotation's diagonal 1, the rest and the
        // translation 0.
        for row in 0..3 {
            let at = copy + off::NI_LOCAL_ROTATE + 0x10 * row;
            e.mem.set_f32(at, 1.0);
            for word in 1..4 {
                e.mem.set_u32(at + 4 * word, 0);
            }
        }
        e.call(CLONING_DESTRUCT, &args![info]);
        copy
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GeneratePlayerNodeHook.
/// `0x94E4A1` (the player): the first-person model's path found in the model loader's map,
/// its reference count raised by 1 when it is 0 or less, its node copied, the copy flagged
/// `0x200000` (as the exe's flag call does) and kept as JIP's first-person node; answers the
/// copy. JIP reads the found model with no test (a model not in the map stops the game); the
/// port answers nothing then, and the exe loads it.
fn first_person_model(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let path = e.mem.u32(FIRST_PERSON_PATH);
    let loader = e.mem.u32(MODEL_LOADER);
    let map = e.mem.u32(loader);
    let model = e.with_stack(4, |e, out| {
        let out = out.addr();
        e.mem.set_u32(out, 0);
        e.call(MODEL_MAP_FIND, &args![map, path, out]);
        e.mem.u32(out)
    });
    if model == 0 {
        return None;
    }
    // JIP: `cmp [eax+8], 0` (a plain read), then `lock inc [eax+8]`: the increment is atomic
    // (the loader's other threads count the same model); the plain write only where the word
    // has no atomic view (unaligned or unmapped).
    let refs = e.mem.u32(model + off::MODEL_REFS);
    if refs as i32 <= 0 {
        match e.mem.atomic_u32(model + off::MODEL_REFS) {
            Some(count) => {
                count.fetch_add(1, Ordering::SeqCst);
            }
            None => e.mem.set_u32(model + off::MODEL_REFS, refs.wrapping_add(1)),
        }
    }
    let node = e.mem.u32(model + off::MODEL_NODE);
    let copy = create_copy(e, node);
    let flags = e.mem.u8(copy + off::NI_FLAGS + 2);
    e.mem.set_u8(copy + off::NI_FLAGS + 2, flags | 0x20);
    e.ext_free::<FirstPersonNode>()
        .0
        .store(copy, Ordering::Relaxed);
    Some(copy)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite16(0x94E5F6, 0x18EB)).
/// `0x94E5F6` (the player): the eye height's read skipped.
fn eye_height(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

fn ref_flags(e: &Engine, refr: u32) -> u8 {
    e.mem.u8(refr + off::REF_FLAGS_5F)
}

/// `DoQueuedReferenceHook` in the three slots (fastcall: the task).
fn queued_reference_routine(e: &mut Engine, w: &[u32]) -> Ret {
    queued_reference(e, w[0]);
    Ret::default()
}

/// `DoQueuedPlayerHook` in `QueuedPlayer`'s slot (fastcall: the task).
fn queued_player_routine(e: &mut Engine, w: &[u32]) -> Ret {
    queued_player(e, w[0]);
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DoQueuedReferenceHook.
/// JIP's attach of a queued reference: nothing for a task in state 6. Else, in the task's
/// memory context: a reference not flagged `0x800`/`0x20` with a parent cell takes the cell's
/// lock and, the cell's state 2 or more, gets its addon nodes (the task's model, when it has
/// one) and is loaded by `TES`; then the loader is told, the cell's queued count goes down (its
/// critical count too unless the reference's slot `+0x100` is the shared `return true`), a
/// reference with 3D and JIP's `Update3D` mark loses the mark and a fade node's fade is set to
/// 1, and the lock is let go. Two deliberate differences, both keeping the lock's enter and
/// leave an exact pair: JIP lets the lock go whenever there is a cell, also when it did not
/// take it (a flagged reference), and the port lets it go only when taken (a count of another
/// holder lowered is a broken lock); and JIP (as the exe) lets go the lock of the parent cell
/// read again after the load (none when that read is 0), while the port lets go the lock of
/// the cell it took (the same cell unless the load moved the reference to another).
pub fn queued_reference(e: &mut Engine, queued: u32) {
    if e.mem.u32(queued + off::TASK_STATE) == 6 {
        return;
    }
    let refr = e.mem.u32(queued + off::TASK_REF);
    let context = e.tls() + TLS_MEMORY_CONTEXT;
    let saved = e.mem.u32(context);
    let own = e.mem.u32(queued + off::TASK_CONTEXT);
    e.mem.set_u32(context, own);
    // The cell whose lock was taken (0: none).
    let mut locked = 0u32;
    if e.mem.u32(refr + off::FORM_FLAGS) & 0x820 == 0 {
        let cell = e.mem.u32(refr + off::PARENT_CELL);
        if cell != 0 {
            // The exe's recursive lock enter (`0x40FBF0`) takes the lock and a label word
            // (0: none); JIP's `LightCS::Enter` takes no label.
            e.call(LOCK_ENTER, &args![cell + off::CELL_LOCK, 0u32]);
            locked = cell;
            let cell = e.mem.u32(refr + off::PARENT_CELL);
            if e.mem.u8(cell + off::CELL_STATE) >= 2 {
                let model = e.mem.u32(queued + off::TASK_MODEL);
                if model != 0 {
                    let node = e.mem.u32(model + off::MODEL_NODE);
                    let loader = e.mem.u32(MODEL_LOADER);
                    e.call(LOAD_ADDON_NODES, &args![loader, refr, node]);
                }
                let cell = e.mem.u32(refr + off::PARENT_CELL);
                let tes = e.mem.u32(TES);
                e.call(TES_LOAD_REFERENCE, &args![tes, refr, cell, queued, 0u32]);
            }
        }
    }
    let loader = e.mem.u32(MODEL_LOADER);
    let attached = e.mem.u32(loader + 8);
    e.call(REFERENCE_ATTACHED, &args![attached, refr]);
    let cell = e.mem.u32(refr + off::PARENT_CELL);
    if cell != 0 {
        e.call(QUEUED_COUNT_DOWN, &args![cell]);
        let vtable = e.mem.u32(refr);
        if e.mem.u32(vtable + off::VT_3D_NOT_CRITICAL) != RETURN_TRUE {
            e.call(CRITICAL_COUNT_DOWN, &args![cell]);
        }
        let loaded = e.mem.u32(refr + off::LOADED_DATA);
        let node = if loaded != 0 {
            e.mem.u32(loaded + off::LOADED_3D)
        } else {
            0
        };
        if node != 0 {
            let flags = ref_flags(e, refr);
            if flags & REF_FLAG_UPDATE_3D != 0 {
                e.mem
                    .set_u8(refr + off::REF_FLAGS_5F, flags & !REF_FLAG_UPDATE_3D);
                let node_vtable = e.mem.u32(node);
                if e.mem.u32(node_vtable + off::VT_IS_FADE_NODE) == RETURN_THIS {
                    e.mem.set_f32(node + off::FADE, 1.0);
                    e.mem.set_f32(node + off::FADE + 4, 1.0);
                }
            }
        }
    }
    // Last, as JIP's leave (which comes after the counts and the fade); also when the load
    // left the reference with no cell (JIP then leaves nothing: a lock held for good).
    if locked != 0 {
        e.call(LOCK_LEAVE, &args![locked + off::CELL_LOCK]);
    }
    e.mem.set_u32(context, saved);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DoQueuedPlayerHook.
/// JIP's attach of the queued player: without JIP's `Update3D` mark, [`queued_reference`].
/// With it: the third person 3D let go (`Set3D(0, 1)`), the first-person biped and animation
/// deleted and cleared, the first-person node taken from its parent, the four camera node
/// globals and the `0x60` bytes at `+0xD74` zeroed, the mark cleared, then
/// [`queued_reference`] (whose load builds both models again). After it, with both models: the
/// view's node shown and the other hidden (flag bit 0), and a high-level process holding a
/// weapon (or its byte `+0x135` set) puts the weapon back on the third person skeleton and on
/// the first-person one (`ReparentWeapon`; JIP reads every argument before the first call).
pub fn queued_player(e: &mut Engine, queued: u32) {
    let player = e.mem.u32(queued + off::TASK_REF);
    if ref_flags(e, player) & REF_FLAG_UPDATE_3D == 0 {
        queued_reference(e, queued);
        return;
    }
    e.call(SET_3D, &args![player, 0u32, 1u32]);
    let biped = e.mem.u32(player + off::BIPED_1ST);
    if biped != 0 {
        e.call(BIPED_DELETE, &args![biped, 1u32]);
        e.mem.set_u32(player + off::BIPED_1ST, 0);
    }
    let animation = e.mem.u32(player + off::ANIM_DATA_1ST);
    if animation != 0 {
        e.call(ANIMATION_DELETE, &args![animation, 1u32]);
        e.mem.set_u32(player + off::ANIM_DATA_1ST, 0);
    }
    let node = e.mem.u32(player + off::NODE_1ST);
    if node != 0 {
        let parent = e.mem.u32(node + off::NI_PARENT);
        if parent != 0 {
            e.vcall(parent, off::VT_DETACH_CHILD, &args![node]);
        }
    }
    for word in 0..4 {
        e.mem.set_u32(CAMERA_NODES + 4 * word, 0);
    }
    for word in 0..0x18 {
        e.mem.set_u32(player + off::PLAYER_D74 + 4 * word, 0);
    }
    let flags = ref_flags(e, player);
    e.mem
        .set_u8(player + off::REF_FLAGS_5F, flags & !REF_FLAG_UPDATE_3D);
    queued_reference(e, queued);

    let loaded = e.mem.u32(player + off::LOADED_DATA);
    if loaded == 0 {
        return;
    }
    let third = e.mem.u32(loaded + off::LOADED_3D);
    if third == 0 {
        return;
    }
    let first = e.mem.u32(player + off::NODE_1ST);
    if first == 0 {
        return;
    }
    let (hidden, shown) = if e.mem.u8(player + off::THIRD_PERSON) == 0 {
        (third, first)
    } else {
        (first, third)
    };
    let flags = e.mem.u8(hidden + off::NI_FLAGS);
    e.mem.set_u8(hidden + off::NI_FLAGS, flags | 1);
    let flags = e.mem.u8(shown + off::NI_FLAGS);
    e.mem.set_u8(shown + off::NI_FLAGS, flags & 0xfe);
    let process = e.mem.u32(player + off::PROCESS);
    if process == 0 || e.mem.u8(process + off::PROCESS_LEVEL) != 0 {
        return;
    }
    let byte = e.mem.u8(process + off::PROCESS_135);
    if e.mem.u32(process + off::PROCESS_WEAPON) == 0 && byte == 0 {
        return;
    }
    let byte = u32::from(byte);
    let third_person = [
        e.mem.u32(player + off::BIPED_3RD),
        e.mem.u32(process + off::PROCESS_ANIM_DATA),
    ];
    let first_person = [
        e.mem.u32(player + off::BIPED_1ST),
        e.mem.u32(player + off::ANIM_DATA_1ST),
    ];
    for [biped, animation] in [third_person, first_person] {
        e.call(
            REPARENT_WEAPON,
            &args![process, byte, biped, animation, player],
        );
    }
}

/// Group GP's fix rows (G26, G72, G105): their three seams, and JIP's music marker cache.
fn install_fixes(e: &mut Engine) {
    e.ext_free::<MusicMarkers>();
    e.on_site(FATIGUE_FLOOR, fatigue_floor);
    e.on_site(MUSIC_MARKER_LIST, music_marker_list);
    e.on_site(MUSIC_MARKER, music_marker);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DamagePCFatigueMinFixHook.
/// G105 (`0x93B87F`: the actor value, the value before the change, the change, `f32` bits): for
/// fatigue the change is cut so the value stops at `fMinFatigue`
/// (`change -= min(value + change - fMinFatigue, 0)`, in single precision as JIP's `addss`,
/// `subss`, `minss`); another actor value keeps its change. JIP's hook makes the exe's value
/// read there itself (the seam sits after it).
fn fatigue_floor(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if a[0] != FATIGUE {
        return None;
    }
    let (value, change) = (f32::from_bits(a[1]), f32::from_bits(a[2]));
    let floor = e.mem.f32(MIN_FATIGUE);
    let over = (value + change) - floor;
    // `minss xmm1, xmm2` with xmm2 = 0: the first when it is below, else 0 (a NaN gives 0).
    let over = if over < 0.0 { over } else { 0.0 };
    Some((change - over).to_bits())
}

/// A cached marker: the reference, its `ExtraAudioMarker` data, the data's `mediaLocCtrlID`.
type MusicMarker = (u32, u32, u32);
/// The world space the cache was built for, and its markers.
type MarkerCache = (u32, Vec<MusicMarker>);

/// JIP's music marker cache (`s_audioMarkerLastWorld`, `s_musicMarkerList`): the world space it
/// was built for and its markers (reference, `ExtraAudioMarker` data, the data's
/// `mediaLocCtrlID`), sorted by that id. JIP never empties it except for another world space.
/// The references are persistent world-space references, never freed in a session (JIP holds
/// the same references across loads).
#[derive(Default)]
pub struct MusicMarkers(Mutex<MarkerCache>);

fn music_markers(e: &mut Engine) -> std::sync::MutexGuard<'_, MarkerCache> {
    e.ext_free::<MusicMarkers>()
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: BuildMusicMarkerList,
// BuildMusicMarkerListHook.
/// G72 (`0x947DD2`: the world space, its marker list): for a world space other than the last
/// one the cache is built again from the list (each reference whose `ExtraAudioMarker` has
/// data, inserted after every marker of a lower or equal `mediaLocCtrlID`); then each cached
/// marker is copied (8 bytes of the game's heap) and put at the front of the player's music
/// marker list, which so ends in the reverse order. Answers 1: the exe's loop is skipped (JIP
/// goes on at `0x947E79`, where the marker list is freed).
fn music_marker_list(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (world, list) = (a[0], a[1]);
    let last = music_markers(e).0;
    if last != world {
        let mut markers: Vec<MusicMarker> = Vec::new();
        let mut node = list;
        while node != 0 {
            let refr = e.mem.u32(node);
            node = e.mem.u32(node + 4);
            if refr == 0 {
                continue;
            }
            let extra = e
                .call(GET_BY_TYPE, &args![refr + 0x44, EXTRA_AUDIO_MARKER])
                .u32();
            if extra == 0 {
                continue;
            }
            let data = e.mem.u32(extra + 0xc);
            if data == 0 {
                continue;
            }
            let id = e.mem.u32(data + 0x24);
            // JIP's `InsertSorted` (`item < array[index]`): after every marker not above it.
            let at = markers.partition_point(|&(_, _, other)| other <= id);
            markers.insert(at, (refr, data, id));
        }
        *music_markers(e) = (world, markers);
    }
    let markers = music_markers(e).1.clone();
    let head = e.mem.u32(PLAYER) + PLAYER_MUSIC_MARKERS;
    for (refr, data, _) in markers {
        let marker = crate::commands::heap_alloc(e, 8);
        e.mem.set_u32(marker, refr);
        e.mem.set_u32(marker + 4, data);
        prepend(e, head, marker);
    }
    Some(1)
}

/// JIP's `tList::Prepend` on the list whose head node is at `head`: an empty list takes the
/// item in its head node; else a new node (8 bytes of the game's heap) takes the head node's
/// item and next, and the head node the item.
fn prepend(e: &mut Engine, head: u32, item: u32) {
    if item == 0 {
        return;
    }
    let first = e.mem.u32(head);
    if first == 0 {
        e.mem.set_u32(head, item);
        return;
    }
    let node = crate::commands::heap_alloc(e, 8);
    e.mem.set_u32(node, first);
    e.mem.set_u32(head, item);
    let next = e.mem.u32(head + 4);
    e.mem.set_u32(node + 4, next);
    e.mem.set_u32(head + 4, node);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// ("Ignore disabled MusicMarkers": the 20 bytes at 0x9699C5).
/// G26 (`0x9699C5`: the marker's reference): a disabled reference (form flag `0x800`) answers 0
/// (the marker is passed over); another answers the address of its position (`+0x30`, read
/// where the exe calls the reference's position slot `+0x1F4`).
fn music_marker(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let refr = a[0];
    if e.mem.u32(refr + off::FORM_FLAGS) & 0x800 != 0 {
        return Some(0);
    }
    Some(refr + 0x30)
}

/// JIP's immortal mode (`s_playerMinHPMode`) and hit effects word (`s_disableHitEffects`): kept
/// lock-free, made at install before the threads start.
#[derive(Default)]
pub struct Immortal {
    mode: AtomicU32,
    hit_effects: AtomicU32,
}

/// `s_playerMinHPMode`: 0 off, 1 the player kept at 1 health, 2 the player cannot be hit.
pub fn immortal_mode(e: &mut Engine) -> u32 {
    e.ext_free::<Immortal>().mode.load(Ordering::Relaxed)
}

/// `s_disableHitEffects`'s bit 2 (immortal mode 2), for the hit effect patches (G123 to G125,
/// whose `ToggleHitEffects` keeps bit 1).
pub fn disable_hit_effects(e: &mut Engine) -> u32 {
    e.ext_free::<Immortal>().hit_effects.load(Ordering::Relaxed)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ModHardcoreNeedsHook.
/// G114 + O6 (`0x969C41`: the player, where the exe calls `IsInState(4)`): with the switch
/// `bHardcoreNeedsFix` (O6) each of the three needs (`+0x5D4`) takes its pending change
/// (`+0x49C`) and stops at 0 (`addps`, then `maxps` with 0: a NaN gives 0), and the pending
/// changes are zeroed; then with JIP's co-save flag `NoHardcoreTracking` (`ToggleHardcoreTracking
/// 0`) the reset byte (`+0xE39`) gets bit 0 (so the timers start again each update); answers the
/// exe's test (`+0x680` bit 2).
fn hardcore_needs(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let needs = a[0] + 0x5d4;
    let pending = a[0] + 0x49c;
    if e.jip_patches() & jip_patches::HARDCORE_NEEDS_FIX != 0 {
        for i in 0..3 {
            let sum = e.mem.f32(needs + 4 * i) + e.mem.f32(pending + 4 * i);
            e.mem
                .set_f32(needs + 4 * i, if sum > 0.0 { sum } else { 0.0 });
        }
        for i in 0..3 {
            e.mem.set_u32(pending + 4 * i, 0);
        }
    }
    if crate::commands::jip_fn_miscellaneous::serialized_flags(e) & NO_HARDCORE_TRACKING != 0 {
        let byte = e.mem.u8(needs + 0x865);
        e.mem.set_u8(needs + 0x865, byte | 1);
    }
    Some(u32::from(e.mem.u8(needs + 0xac) & 4 != 0))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: PlayerCannotBeHitHook.
/// G122, in the player's `MagicTarget` slot (`0x108A9B4`, the exe's `0x952720`, god mode): the
/// god mode setting's value byte when it is not 0 (the byte itself, where the exe answers 1),
/// else 1 in immortal mode 2, else the god mode byte.
fn cannot_be_hit_routine(e: &mut Engine, _: &[u32]) -> Ret {
    let setting = e.mem.u8(GOD_MODE_SETTING_VALUE);
    let answer = if setting != 0 {
        setting
    } else if immortal_mode(e) == 2 {
        1
    } else {
        e.mem.u8(GOD_MODE_BYTE)
    };
    Ret {
        eax: u32::from(answer),
        ..Ret::default()
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_ToggleHardcoreTracking_Execute.
/// `ToggleHardcoreTracking on`: 0 sets JIP's co-save flag `NoHardcoreTracking`, anything else
/// clears it (an argument that does not read: nothing).
fn toggle_hardcore_tracking(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let on = read.int(0, 0);
    if read.ok {
        let flags = &mut e.ext::<Jip>().saved.serialized.flags;
        if on != 0 {
            *flags &= !NO_HARDCORE_TRACKING;
        } else {
            *flags |= NO_HARDCORE_TRACKING;
        }
        crate::commands::jip_fn_miscellaneous::publish_serialized_flags(e);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_ToggleImmortalMode_Execute.
/// `ToggleImmortalMode [mode]`: answers the mode before; with an argument the mode becomes it,
/// the PlayerMinHealth hook is on only for mode 1, and the hit effects word gets bit 2 for mode
/// 2 (else keeps only bit 1).
fn toggle_immortal_mode(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let before = immortal_mode(e);
    a.set_result(e, f64::from(before as u8));
    if crate::commands::jip_fn_utility::num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        let mode = read.int(0, 0);
        if read.ok {
            // `s_playerMinHPMode` is a byte; the two tests use the whole argument.
            e.ext_free::<Immortal>()
                .mode
                .store(mode & 0xff, Ordering::Relaxed);
            e.set_hook(Hook::PlayerMinHealth, mode == 1);
            let state = e.ext_free::<Immortal>();
            if mode == 2 {
                state.hit_effects.fetch_or(2, Ordering::Relaxed);
            } else {
                state.hit_effects.fetch_and(1, Ordering::Relaxed);
            }
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "gp_tests.rs"]
mod tests;
