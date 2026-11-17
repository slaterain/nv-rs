//! `functions_jip/jip_fn_miscellaneous.h` (JIP LN NVSE): game state the player and the world keep
//! (difficulty, combat, detection, scope and iron sights, run and auto-move, kill cam, the Pip-Boy
//! radio, the enemy health bar's target, the last exterior door, time frozen, the debug toggles),
//! container respawns, idle loop counts, nav meshes switched off and on, terrain height, exterior
//! and buffered cells, the moon's textures, days passed since a date, hardcore stages, exact XP
//! rewards, actor values by name and JIP's keywords; forms' descriptions, the physics world's
//! gravity, what the reticle and other ray casts hit, angles between points, the camera's
//! movement, the game hour, ash pile activators and dead actors cleared, the player made to
//! sneak. 57 of its 83 commands
//! (85 in the file, two of them never registered: `SetWobblesRotation`, `PlaceModel`).
//!
//! The core track built ten of the file's commands in its hook files (not repeated here):
//! `SetFormDescription`, `GetPCFastTravelled`, `GetPCMovedCell`, `SwapObjectLOD` and the event
//! handlers `SetOnFastTravel`, `SetOnPCTargetChange`, `SetOnDialogTopic`, `SetOnNoteAdded`
//! (`world_hooks.rs`), `SetOnLocationDiscover`, `SetOnCrafting` (`ui_hooks.rs`).
//!
//! Layouts (the player `*011DEA3C`): game difficulty +0x7B8 (a signed word), third person +0x64A,
//! always run +0x651, auto-move +0x652, last exterior door +0x604, in combat +0xDF0, unseen +0xDF1,
//! kill cam timer +0xE18 (a float), iron sights node +0xE34, parent cell +0x40, process +0x68 (its
//! `IsWeaponOut` at vtable +0x454), `ModActorValueInt` at vtable +0x3A8. The HUD (`*011D96C0`):
//! using a scope +0x1FC, the enemy health target +0x21C. `TES` (`*011DEA10`): interiors buffer
//! +0x38, exteriors buffer +0x3C, slots counted by `uInterior Cell Buffer` / `uExterior Cell
//! Buffer` (settings at `011C3E38` / `011C3C90`, their values at +4). The sky (`*011DEA20`): the
//! masser moon +0x30, its eight texture strings from +0x18 (8 bytes each). VATS camera data
//! `011F2250` (mode +8). The OS globals (`*011DEA0C`): time frozen +7. The Pip-Boy's station
//! `*011DD42C` (its reference +0). Time globals `011DE7B8` (year, month, day, hour: globals with
//! their value at +0x24). The audit table names the exe evidence of each.
//!
//! `ToggleHardcoreTracking` (G114) and `ToggleImmortalMode` (G122) are built with their game
//! patches, in `crate::game_patches::gp`; `GetConditionDamagePenalty`/`SetConditionDamagePenalty`
//! (G84), `SetArmorConditionPenalty` (G85), `SetDamageToArmorMaxPercent` (G99) and
//! `ToggleNoMovementCombat` (G126) in `crate::game_patches::gc::formulas`.
//!
//! `SetGameHour` sets JIP's `s_forceHCNeedsUpdate` (kept in `game_patches::gp`), which only JIP's
//! time patch reads (O5, `UpdateTimeGlobalsHook`, not built yet): until O5 the hour changes as
//! with JIP and O5 switched off. `MarkActivatorAshPile` sets the bit G06's tests read (#600).
//! The ray casts are lane G's (`havok_refs::ray_cast`); the camera casts go along the main
//! camera's rotation column 0, as JIP hands `WorldRotate()` itself (no `+ 1`).
//!
//! Deferred (each waits for what is named; no stand-ins):
//! - `ToggleHitEffects` (G123-G125), `ToggleDepthClear` (G128), `ToggleBipedSlotVisibility`
//!   (H15): the stage 3.9 game patches that read what they set.
//! - `SetInternalMarker`: JIP's hook reading `s_internalMarkerIDs` (hooks.h; core track).
//! - `SetMoonTexture`, `ToggleCameraCollision`, `InitRockItLauncher`: they patch the game's code.
//! - `SwapTextureEx`, `GetLightAmountAtPoint`: the scene graph's textures and lights.

use super::arrays::{self, Element};
use super::havok_refs as h;
use super::{extra, list_items, ref_id, type_of, Handlers};
use crate::containers::str_hash_ci;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x26e9, get_idle_loop_times),
    (0x26ed, disable_nav_mesh_alt),
    (0x26ee, enable_nav_mesh_alt),
    (0x26ef, get_terrain_height),
    (0x271b, get_form_description),
    (0x2721, get_container_respawns),
    (0x2722, set_container_respawns),
    (0x2736, get_exterior_cell),
    (0x2737, get_cell_buffered),
    (0x2740, toggle_first_person),
    (0x2770, set_idle_loop_times),
    (0x277b, get_pc_last_exterior_door),
    (0x27a3, get_moon_texture),
    (0x27b0, free_player),
    (0x27b2, get_local_gravity),
    (0x27b3, set_local_gravity_vector),
    (0x27f1, get_reticle_pos),
    (0x27f2, get_reticle_range),
    (0x282f, get_game_days_passed),
    (0x2838, set_game_difficulty),
    (0x283d, get_enemy_health_target),
    (0x2841, mark_activator_ash_pile),
    (0x2853, get_buffered_cells),
    (0x288b, set_game_hour),
    (0x28a5, string_to_actor_value),
    (0x28b6, get_hardcore_stage_threshold),
    (0x28b7, set_hardcore_stage_threshold),
    (0x28b8, get_hardcore_stage_effect),
    (0x28b9, set_hardcore_stage_effect),
    (0x28bd, get_debug_mode_state),
    (0x28bf, freeze_time),
    (0x28ed, reward_xp_exact),
    (0x28ef, clear_dead_actors),
    (0x28f0, get_camera_movement),
    (0x28f5, get_reticle_node),
    (0x2906, get_point_ray_cast_pos),
    (0x2907, toggle_player_sneaking),
    (0x291f, get_reticle_pos_alt),
    (0x2921, transform_world_to_local_cmd),
    (0x2922, get_angles_between_points),
    (0x2923, get_p2p_ray_cast_range),
    (0x293a, assign_keyword),
    (0x293b, has_keyword),
    (0x293c, get_keyword_forms),
    (0x2942, get_reticle_target_limb),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x26fc, is_pc_in_combat, is_pc_in_combat_eval),
    (0x273f, get_game_difficulty, get_game_difficulty_eval),
    (0x274a, get_pc_detection_state, get_pc_detection_state_eval),
    (0x2751, get_pipboy_radio, get_pipboy_radio_eval),
    (0x276a, get_pc_using_scope, get_pc_using_scope_eval),
    (0x277a, get_pc_using_iron_sights, get_pc_using_iron_sights_eval),
    (0x27aa, get_always_run, get_always_run_eval),
    (0x27ab, get_auto_move, get_auto_move_eval),
    (0x287c, is_in_kill_cam, is_in_kill_cam_eval),
    (0x28ad, get_hardcore_tracking, get_hardcore_tracking_eval),
    (0x28b1, get_note_read, get_note_read_eval),
];

#[rustfmt::skip]
pub static CONDITIONS_ONLY: super::ConditionsOnly = &[
    (0x293e, has_keyword_cond_eval),
];

/// What the file's commands keep between calls in JIP's shared state (`TempObject`s: never
/// saved, never cleared). The keywords are not here: see [`Unlocked`].
#[derive(Default)]
pub struct State {
    /// `s_actorValueIDsMap`, built at the first `StringToActorValue`: each actor value's name by
    /// its `StrHashCI` (JIP's `const char*` keys compare by the hash alone), its code.
    pub actor_value_ids: HashMap<u32, u32>,
}

/// What the file's conditions read on any thread, without JIP's shared state: a lock-free slot
/// ([`Engine::ext_free`]) made at install, before the threads start, so every thread's engine of
/// the run reads the same one. AI threads evaluate package and idle conditions; taking JIP's
/// shared slot there would hold every JIP command's state until that thread's next safe point.
#[derive(Default)]
pub(crate) struct Unlocked {
    /// `s_keywordFormsMap`: each keyword's `StrHashCI` and its forms (JIP's `Set<TESForm*>`, a
    /// sorted array: in address order). The ONLY copy: the commands and the condition all read
    /// and write it here. A reader holds the read lock for one lookup, a writer the write lock
    /// for one insert (never across a game call), so nobody waits past that. JIP never clears it.
    keywords: RwLock<HashMap<u32, BTreeSet<u32>>>,
    /// A copy of JIP's co-save flags word (`s_serializedVars.flags`, kept in
    /// [`crate::cosave::Saved`]): the co-save's copy is the one saved and loaded; this one is
    /// written from it by [`publish_serialized_flags`] after every change there (each load, the
    /// reset of a load, a new game or the main menu: the co-save code calls it in
    /// `process_changed_flags`, at the end of `load` and on `NEW_GAME`). JIP's commands that set
    /// flags are not built
    /// yet (stage 3.9); each must call it too. The command and the condition both read this copy,
    /// so they always agree.
    serialized_flags: AtomicU32,
    /// A copy of the armour damage cap (`s_serializedVars.dmgArmorMaxPercent`, kept in
    /// [`crate::cosave::Saved`], `SetDamageToArmorMaxPercent`), written with the flags word by
    /// [`publish_serialized_flags`]: G99 reads it on the AI threads (`Actor::HitMe`).
    armor_damage_cap: ArmorDamageCap,
}

/// The cap's `float` as its bits; 1.0 (JIP's `Reset`) until the first publish.
pub(crate) struct ArmorDamageCap(AtomicU32);

impl Default for ArmorDamageCap {
    fn default() -> Self {
        ArmorDamageCap(AtomicU32::new(1.0f32.to_bits()))
    }
}

/// Makes the lock-free slot (at install, before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<Unlocked>();
}

/// Copies JIP's co-save flags word and armour damage cap into the lock-free slot (see
/// [`Unlocked::serialized_flags`]).
pub(crate) fn publish_serialized_flags(e: &mut Engine) {
    let flags = e.ext::<Jip>().saved.serialized.flags;
    e.ext_free::<Unlocked>()
        .serialized_flags
        .store(flags, Ordering::Relaxed);
    let cap = e.ext::<Jip>().saved.serialized.dmg_armor_max_percent;
    e.ext_free::<Unlocked>()
        .armor_damage_cap
        .0
        .store(cap, Ordering::Relaxed);
}

/// The armour damage cap (`SetDamageToArmorMaxPercent`), from the lock-free copy.
pub(crate) fn armor_damage_cap(e: &mut Engine) -> f32 {
    f32::from_bits(
        e.ext_free::<Unlocked>()
            .armor_damage_cap
            .0
            .load(Ordering::Relaxed),
    )
}

/// JIP's co-save flags word, from the lock-free copy.
pub(crate) fn serialized_flags(e: &mut Engine) -> u32 {
    e.ext_free::<Unlocked>()
        .serialized_flags
        .load(Ordering::Relaxed)
}

/// Runs `f` on the keyword map under its read lock (a poisoned lock is still read: a panic in
/// another reader leaves the map whole).
fn with_keywords<R>(e: &mut Engine, f: impl FnOnce(&HashMap<u32, BTreeSet<u32>>) -> R) -> R {
    let map = e.ext_free::<Unlocked>().keywords.read();
    f(&map.unwrap_or_else(|p| p.into_inner()))
}

// ---- the game's objects ----

/// `g_thePlayer`.
const PLAYER: u32 = 0x011d_ea3c;
const PC_PARENT_CELL: u32 = 0x40;
const PC_PROCESS: u32 = 0x68;
const PC_LAST_EXTERIOR_DOOR: u32 = 0x604;
const PC_THIRD_PERSON: u32 = 0x64a;
const PC_ALWAYS_RUN: u32 = 0x651;
const PC_AUTO_MOVE: u32 = 0x652;
const PC_GAME_DIFFICULTY: u32 = 0x7b8;
const PC_IN_COMBAT: u32 = 0xdf0;
const PC_UNSEEN: u32 = 0xdf1;
const PC_KILL_CAM_TIMER: u32 = 0xe18;
const PC_IRON_SIGHT_NODE: u32 = 0xe34;
/// `Actor::ModActorValueInt` (vtable +0x3A8) and `BaseProcess::IsWeaponOut` (vtable +0x454).
const MOD_ACTOR_VALUE_INT: u32 = 0x3a8;
const IS_WEAPON_OUT: u32 = 0x454;
/// `kAVCode_XP`.
const AV_XP: u32 = 0x18;
/// `PlayerCharacter::SetFirstPerson` (`00950110`, `thiscall`, a bool).
const SET_FIRST_PERSON: u32 = 0x0095_0110;
/// `ProcessManager` (`011E0E80`) and its `GetTotalDetectionValue` (`00973710`, `thiscall`: the
/// actor, a bool).
const PROCESS_MANAGER: u32 = 0x011e_0e80;
const TOTAL_DETECTION_VALUE: u32 = 0x0097_3710;
/// `*011D96C0`: the HUD (`HUDMainMenu`).
const HUD: u32 = 0x011d_96c0;
const HUD_USING_SCOPE: u32 = 0x1fc;
const HUD_HEALTH_TARGET: u32 = 0x21c;
/// `*011DEA10`: `TES`, its buffers and `GetTerrainHeight` (`004572E0`, `thiscall`: the position,
/// the result).
const TES: u32 = 0x011d_ea10;
const TES_INTERIORS: u32 = 0x38;
const TES_EXTERIORS: u32 = 0x3c;
const TERRAIN_HEIGHT: u32 = 0x0045_72e0;
/// The values of `uInterior Cell Buffer:General` and `uExterior Cell Buffer:General`.
const INTERIOR_BUFFER_SETTING: u32 = 0x011c_3e38 + 4;
const EXTERIOR_BUFFER_SETTING: u32 = 0x011c_3c90 + 4;
/// `*011DEA20`: the sky; the masser moon +0x30, its textures +0x18.
const SKY: u32 = 0x011d_ea20;
const SKY_MASSER: u32 = 0x30;
const MOON_TEXTURES: u32 = 0x18;
/// `VATSCameraData` (`011F2250`), its mode +8.
const VATS_CAMERA_MODE: u32 = 0x011f_2250 + 8;
/// `*011DEA0C`: the OS globals; time frozen +7.
const OS_GLOBALS: u32 = 0x011d_ea0c;
const FREEZE_TIME: u32 = 7;
/// `*011DD42C`: the Pip-Boy's radio station (`RadioEntry`, its reference +0).
const PIPBOY_RADIO: u32 = 0x011d_d42c;
/// `GameTimeGlobals` (`011DE7B8`): the year, month, day and hour globals; a global's value +0x24.
const TIME_GLOBALS: u32 = 0x011d_e7b8;
const GLOBAL_VALUE: u32 = 0x24;
/// The nav mesh obstacle manager (`006C0720`, `cdecl`: the instance), its `Disable`
/// (`006C1150`) and `Enable` (`006C1130`), each `thiscall` with a form id.
const OBSTACLE_MANAGER: u32 = 0x006c_0720;
const NAV_MESH_DISABLE: u32 = 0x006c_1150;
const NAV_MESH_ENABLE: u32 = 0x006c_1130;
/// `BSAudio::Precache` (`00AD7FA0`, `thiscall`: an editor id, flags) on `*011F6D98`, with
/// `"UILevelUp"` (`0106F354`) and the flags 0x121, as JIP calls it.
const AUDIO: u32 = 0x011f_6d98;
const AUDIO_PRECACHE: u32 = 0x00ad_7fa0;
const UI_LEVEL_UP: u32 = 0x0106_f354;
/// `ActorValueInfo::Array()` (`011D61C8`), 77 entries; an info's name (`char*`) +0x38.
const ACTOR_VALUE_INFOS: u32 = 0x011d_61c8;
const ACTOR_VALUE_COUNT: u32 = 77;
const AV_INFO_NAME: u32 = 0x38;
/// `kDebugModeBytes`: TGM, TAI, TCAI, TCL, TB.
const DEBUG_MODE_BYTES: [u32; 5] = [
    0x011e_07ba,
    0x011f_1220,
    0x011a_4880,
    0x011c_3c0d,
    0x011c_ae44,
];
/// `ADDR_ReturnTrue`, what a reference's vtable holds at +0xF0 (`IS_REFERENCE`).
const RETURN_TRUE: u32 = 0x008d_0360;
const IS_REFERENCE_SLOT: u32 = 0xf0;

// Form types and fields.
const TES_OBJECT_CONT: u8 = 0x1b;
const TES_OBJECT_CELL: u8 = 0x39;
const NAV_MESH: u8 = 0x43;
const TES_IDLE_FORM: u8 = 0x48;
const BGS_LIST_FORM: u8 = 0x55;
const BGS_RADIATION_STAGE: u8 = 0x5a;
const BGS_DEHYDRATION_STAGE: u8 = 0x76;
/// `TESObjectCONT::flags`, bit 1 respawns.
const CONT_FLAGS: u32 = 0x98;
/// `TESIdleForm::data`'s loop counts.
const IDLE_LOOP_MIN: u32 = 0x39;
const IDLE_LOOP_MAX: u32 = 0x3a;
/// `TESWorldSpace::cellMap`; a cell's worldspace.
const WS_CELL_MAP: u32 = 0x30;
const CELL_WORLDSPACE: u32 = 0xc0;
/// `BGSNote::read`.
const NOTE_READ: u32 = 0x7d;
/// `BGSRadiationStage` (and the other stages): threshold +0x18, effect +0x1C.
const STAGE_THRESHOLD: u32 = 0x18;
const STAGE_EFFECT: u32 = 0x1c;
/// `BGSListForm::list`, a `tList` with its first node inline.
const LIST_FORM_LIST: u32 = 0x18;

/// JIP's `kDaysPerMonth`.
const DAYS_PER_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn player(e: &Engine) -> u32 {
    e.mem.u32(PLAYER)
}

fn flag(e: &Engine, at: u32) -> bool {
    e.mem.u8(at) != 0
}

/// A one or nothing: JIP's `if (...) *result = 1`.
fn one_if(e: &mut Engine, result: u32, yes: bool) {
    if yes {
        e.mem.set_f64(result, 1.0);
    }
}

/// `TESForm::GetBaseIfRef` (JIP LN NVSE, asm): a reference's base form
/// ([`crate::overrides::ref_base_form`]), else the form.
fn base_if_ref(e: &mut Engine, form: u32) -> u32 {
    if e.mem.u32(e.mem.u32(form) + IS_REFERENCE_SLOT) == RETURN_TRUE {
        crate::overrides::ref_base_form(e, form)
    } else {
        form
    }
}

// ---- idles, nav meshes, terrain, containers ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetIdleLoopTimes_Execute.
/// An idle's loop maximum (the integer not 0) or minimum.
fn get_idle_loop_times(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let idle = read.form(0);
    if read.ok && idle != 0 && type_of(e, idle) == TES_IDLE_FORM {
        let at = if read.int(1, 0) != 0 {
            IDLE_LOOP_MAX
        } else {
            IDLE_LOOP_MIN
        };
        a.set_result(e, f64::from(e.mem.u8(idle + at)));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetIdleLoopTimes_Execute.
/// The loop counts set: the minimum capped at 255 (unsigned), the maximum raised to it; both
/// stored as bytes (a maximum above 255 keeps its low byte, as the `UInt8` store does).
fn set_idle_loop_times(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let idle = read.form(0);
    if read.ok && idle != 0 && type_of(e, idle) == TES_IDLE_FORM {
        let min = read.int(1, 0).min(255);
        let max = read.int(2, 0).max(min);
        e.mem.set_u8(idle + IDLE_LOOP_MIN, min as u8);
        e.mem.set_u8(idle + IDLE_LOOP_MAX, max as u8);
    }
    handled(true)
}

/// `DisableNavMeshAlt` / `EnableNavMeshAlt`: a nav mesh's id handed to the obstacle manager.
fn nav_mesh(e: &mut Engine, w: &[u32], call: u32) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if read.ok && form != 0 && type_of(e, form) == NAV_MESH {
        let manager = e.call(OBSTACLE_MANAGER, &[]).u32();
        let id = ref_id(e, form);
        e.call(call, &args![manager, id]);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_DisableNavMeshAlt_Execute.
fn disable_nav_mesh_alt(e: &mut Engine, w: &[u32]) -> Ret {
    nav_mesh(e, w, NAV_MESH_DISABLE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_EnableNavMeshAlt_Execute.
fn enable_nav_mesh_alt(e: &mut Engine, w: &[u32]) -> Ret {
    nav_mesh(e, w, NAV_MESH_ENABLE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetTerrainHeight_Execute.
/// The game's terrain height at (x, y) (`TES::GetTerrainHeight` with a point and a float on the
/// stack, the float 0 first); 0 when the arguments do not read. Written always.
fn get_terrain_height(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let tes = e.mem.u32(TES);
    let mut height = 0.0f32;
    if read.ok && tes != 0 {
        let (x, y) = (read.float(0, 0.0), read.float(1, 0.0));
        height = e.with_stack(12, |e, at| {
            let at = at.addr();
            e.mem.set_f32(at, x);
            e.mem.set_f32(at + 4, y);
            e.mem.set_f32(at + 8, 0.0);
            e.call(TERRAIN_HEIGHT, &args![tes, at, at + 8]);
            e.mem.f32(at + 8)
        });
    }
    a.set_result(e, f64::from(height));
    handled(true)
}

/// The container a respawn command works on: the form given, else the calling reference's base
/// form; `None` when neither (or the arguments do not read).
fn container_arg(e: &mut Engine, a: &CommandArgs, read: &Read, i: usize) -> Option<u32> {
    if !read.ok {
        return None;
    }
    match read.form(i) {
        0 if a.this_obj == 0 => None,
        0 => Some(nvse::commands::base_form(e, a.this_obj)),
        form => Some(form),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetContainerRespawns_Execute.
/// 1 for a container whose flags have bit 1.
fn get_container_respawns(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(cont) = container_arg(e, &a, &read, 0) {
        let yes = cont != 0
            && type_of(e, cont) == TES_OBJECT_CONT
            && e.mem.u8(cont + CONT_FLAGS) & 2 != 0;
        one_if(e, a.result, yes);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetContainerRespawns_Execute.
/// Bit 1 of a container's flags set (the integer not 0) or cleared (`&= 0xFD`).
fn set_container_respawns(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(cont) = container_arg(e, &a, &read, 1) {
        if cont != 0 && type_of(e, cont) == TES_OBJECT_CONT {
            let flags = e.mem.u8(cont + CONT_FLAGS);
            let flags = if read.int(0, 0) != 0 {
                flags | 2
            } else {
                flags & 0xfd
            };
            e.mem.set_u8(cont + CONT_FLAGS, flags);
        }
    }
    handled(true)
}

// ---- cells ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetExteriorCell_Execute; internal/Ni_types.h: NiTPtrMap::Lookup (asm); internal/utility.h:
// Coordinate.
/// The worldspace's cell at grid (x, y): its cell map looked up by the coordinate word (x in the
/// high 16 bits, y in the low), bucket `key % count`, the bucket's chain walked to the key (with
/// the game-list bound, `nvse::invref::list_step`).
fn get_exterior_cell(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let ws = read.form(0);
    if !read.ok || ws == 0 {
        return handled(true);
    }
    let map = e.mem.u32(ws + WS_CELL_MAP);
    if map == 0 {
        return handled(true);
    }
    let (x, y) = (read.int(1, 0), read.int(2, 0));
    let key = ((x & 0xffff) << 16) | (y & 0xffff);
    let count = e.mem.u32(map + 4);
    if count == 0 {
        return handled(true);
    }
    let mut entry = e.mem.u32(e.mem.u32(map + 8) + 4 * (key % count));
    let mut walked = 0usize;
    while entry != 0 && nvse::invref::list_step(e, &mut walked) {
        if e.mem.u32(entry + 4) == key {
            let cell = e.mem.u32(entry + 8);
            if cell != 0 {
                super::set_form_result(e, &a, cell);
            }
            break;
        }
        entry = e.mem.u32(entry);
    }
    handled(true)
}

/// A `TES` cell buffer and the setting's slot count now: the exteriors' (or the interiors').
fn cell_buffer(e: &Engine, exteriors: bool) -> (u32, u32) {
    let tes = e.mem.u32(TES);
    if tes == 0 {
        return (0, 0);
    }
    if exteriors {
        (
            e.mem.u32(tes + TES_EXTERIORS),
            e.mem.u32(EXTERIOR_BUFFER_SETTING),
        )
    } else {
        (
            e.mem.u32(tes + TES_INTERIORS),
            e.mem.u32(INTERIOR_BUFFER_SETTING),
        )
    }
}

/// The cells of a buffer, from its first slot to the first empty one, at most the setting's
/// slot count (JIP's bound). `TES::TES` (`0044FF70`, at `004500B7`-`00450116`) allocates each
/// buffer once, `setting * 4` bytes of the game's heap, and keeps no count; a script can raise
/// the setting later, so the walk also stops at the heap block's end (its size, rounded up to
/// the heap's size class) and keeps only words that are cells (`TESObjectCELL`): JIP would read
/// past the block. Not a heap block: no slots.
fn buffered_cells(e: &Engine, exteriors: bool) -> Vec<u32> {
    let (mut at, setting) = cell_buffer(e, exteriors);
    let mut cells = Vec::new();
    if at == 0 {
        return cells;
    }
    let made = e.mem.block_size(at).unwrap_or(0) / 4;
    let mut slots = setting.min(made);
    while slots != 0 {
        let cell = e.mem.u32(at);
        if cell == 0 {
            break;
        }
        if type_of(e, cell) == TES_OBJECT_CELL {
            cells.push(cell);
        }
        at = at.wrapping_add(4);
        slots -= 1;
    }
    cells
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetCellBuffered_Execute.
/// 1 when the cell (the argument, a cell; else the caller's parent cell) is in the exteriors
/// buffer (a cell with a worldspace) or the interiors buffer.
fn get_cell_buffered(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut cell = read.form(0);
    if cell == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        cell = e.mem.u32(a.this_obj + PC_PARENT_CELL);
        if cell == 0 {
            return handled(true);
        }
    } else if type_of(e, cell) != TES_OBJECT_CELL {
        return handled(true);
    }
    let exterior = e.mem.u32(cell + CELL_WORLDSPACE) != 0;
    let found = buffered_cells(e, exterior).contains(&cell);
    one_if(e, a.result, found);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetBufferedCells_Execute.
/// An array of the interiors buffer's cells (the integer not 0) or the exteriors'; an empty
/// array for none.
fn get_buffered_cells(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let cells = buffered_cells(e, read.int(0, 0) == 0);
        let elements: Vec<Element> = cells.into_iter().map(Element::Form).collect();
        let id = arrays::create_array(e, a.script, &elements);
        a.set_result(e, f64::from(id as i32));
    }
    handled(true)
}

// ---- the player ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetGameDifficulty_Execute.
fn get_game_difficulty(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let pc = player(e);
    let difficulty = e.mem.u32(pc + PC_GAME_DIFFICULTY) as i32;
    a.set_result(e, f64::from(difficulty));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetGameDifficulty_Eval.
fn get_game_difficulty_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let pc = player(e);
    let difficulty = e.mem.u32(pc + PC_GAME_DIFFICULTY) as i32;
    e.mem.set_f64(a.result, f64::from(difficulty));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetGameDifficulty_Execute.
/// The difficulty set when 4 or less (unsigned).
fn set_game_difficulty(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let difficulty = read.int(0, 0);
    if read.ok && difficulty <= 4 {
        let pc = player(e);
        e.mem.set_u32(pc + PC_GAME_DIFFICULTY, difficulty);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_ToggleFirstPerson_Execute.
/// The game's `SetFirstPerson(toggle)` when the toggle (not 0) equals the third-person byte: the
/// view changes only when it is not already the asked one.
fn toggle_first_person(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let toggle = u8::from(read.int(0, 0) != 0);
        let pc = player(e);
        if toggle == e.mem.u8(pc + PC_THIRD_PERSON) {
            e.call(SET_FIRST_PERSON, &args![pc, u32::from(toggle)]);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// PlayerCharacter::GetDetectionState.
/// -1 with no parent cell, 1 unseen (caution), 3 in combat (danger), 0 when the process
/// manager's total detection value for the player is 0 or less (hidden), else 2 (detected).
fn detection_state(e: &mut Engine) -> i32 {
    let pc = player(e);
    if e.mem.u32(pc + PC_PARENT_CELL) == 0 {
        return -1;
    }
    if flag(e, pc + PC_UNSEEN) {
        return 1;
    }
    if flag(e, pc + PC_IN_COMBAT) {
        return 3;
    }
    let value = e
        .call(TOTAL_DETECTION_VALUE, &args![PROCESS_MANAGER, pc, 0u32])
        .u32() as i32;
    if value <= 0 {
        0
    } else {
        2
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPCDetectionState_Execute.
fn get_pc_detection_state(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let state = detection_state(e);
    a.set_result(e, f64::from(state));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPCDetectionState_Eval.
fn get_pc_detection_state_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let state = detection_state(e);
    e.mem.set_f64(a.result, f64::from(state));
    handled(true)
}

/// `pcInCombat && !pcUnseen`.
fn in_combat(e: &Engine) -> bool {
    let pc = player(e);
    flag(e, pc + PC_IN_COMBAT) && !flag(e, pc + PC_UNSEEN)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_IsPCInCombat_Execute.
fn is_pc_in_combat(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let yes = in_combat(e);
    one_if(e, a.result, yes);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_IsPCInCombat_Eval.
fn is_pc_in_combat_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let yes = in_combat(e);
    one_if(e, a.result, yes);
    handled(true)
}

/// A player byte flag as a command and as a condition.
fn player_flag(e: &mut Engine, result: u32, at: u32) {
    let pc = player(e);
    let yes = flag(e, pc + at);
    one_if(e, result, yes);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetAlwaysRun_Execute.
fn get_always_run(e: &mut Engine, w: &[u32]) -> Ret {
    player_flag(e, CommandArgs::new(w).result, PC_ALWAYS_RUN);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetAlwaysRun_Eval.
fn get_always_run_eval(e: &mut Engine, w: &[u32]) -> Ret {
    player_flag(e, EvalArgs::new(w).result, PC_ALWAYS_RUN);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetAutoMove_Execute.
fn get_auto_move(e: &mut Engine, w: &[u32]) -> Ret {
    player_flag(e, CommandArgs::new(w).result, PC_AUTO_MOVE);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetAutoMove_Eval.
fn get_auto_move_eval(e: &mut Engine, w: &[u32]) -> Ret {
    player_flag(e, EvalArgs::new(w).result, PC_AUTO_MOVE);
    handled(true)
}

/// `ironSightNode && baseProcess->IsWeaponOut()` (no process: no).
fn using_iron_sights(e: &mut Engine) -> bool {
    let pc = player(e);
    if e.mem.u32(pc + PC_IRON_SIGHT_NODE) == 0 {
        return false;
    }
    let process = e.mem.u32(pc + PC_PROCESS);
    process != 0 && e.vcall(process, IS_WEAPON_OUT, &[]).u32() & 0xff != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPCUsingIronSights_Execute.
fn get_pc_using_iron_sights(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let yes = using_iron_sights(e);
    one_if(e, a.result, yes);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPCUsingIronSights_Eval.
fn get_pc_using_iron_sights_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let yes = using_iron_sights(e);
    one_if(e, a.result, yes);
    handled(true)
}

/// VATS camera mode 4 and the player's kill cam timer above 0.
fn in_kill_cam(e: &Engine) -> bool {
    e.mem.u32(VATS_CAMERA_MODE) == 4 && e.mem.f32(player(e) + PC_KILL_CAM_TIMER) > 0.0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_IsInKillCam_Execute.
fn is_in_kill_cam(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let yes = in_kill_cam(e);
    one_if(e, a.result, yes);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_IsInKillCam_Eval.
fn is_in_kill_cam_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let yes = in_kill_cam(e);
    one_if(e, a.result, yes);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPCLastExteriorDoor_Execute.
fn get_pc_last_exterior_door(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let door = e.mem.u32(player(e) + PC_LAST_EXTERIOR_DOOR);
    if door != 0 {
        super::set_form_result(e, &a, door);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_FreePlayer_Execute.
/// JIP frees the player's object on the game's heap (the game then stops at once). Deliberately
/// nothing here: a script must not be able to stop the game.
fn free_player(_: &mut Engine, _: &[u32]) -> Ret {
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_RewardXPExact_Execute.
/// The player's XP raised by the amount (`ModActorValueInt(kAVCode_XP, amount, 0)`), then the
/// level-up sound's call as JIP makes it.
fn reward_xp_exact(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let amount = read.int(0, 0);
        let pc = player(e);
        e.vcall(pc, MOD_ACTOR_VALUE_INT, &args![AV_XP, amount, 0u32]);
        let audio = e.mem.u32(AUDIO);
        e.call(AUDIO_PRECACHE, &args![audio, UI_LEVEL_UP, 0x121u32]);
    }
    handled(true)
}

// ---- the HUD, the radio, the sky, the OS ----

/// The HUD's byte or word at `at`; nothing without a HUD (JIP's pointer is set once the game
/// has started).
fn hud_word(e: &Engine, at: u32) -> Option<u32> {
    let hud = e.mem.u32(HUD);
    (hud != 0).then(|| e.mem.u32(hud + at))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPCUsingScope_Execute.
fn get_pc_using_scope(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let yes = hud_word(e, HUD_USING_SCOPE).is_some_and(|v| v & 0xff != 0);
    one_if(e, a.result, yes);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPCUsingScope_Eval.
fn get_pc_using_scope_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let yes = hud_word(e, HUD_USING_SCOPE).is_some_and(|v| v & 0xff != 0);
    one_if(e, a.result, yes);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetEnemyHealthTarget_Execute.
fn get_enemy_health_target(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(target) = hud_word(e, HUD_HEALTH_TARGET).filter(|&t| t != 0) {
        super::set_form_result(e, &a, target);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPipboyRadio_Execute.
/// The reference of the Pip-Boy's station.
fn get_pipboy_radio(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let entry = e.mem.u32(PIPBOY_RADIO);
    if entry != 0 {
        let radio = e.mem.u32(entry);
        if radio != 0 {
            super::set_form_result(e, &a, radio);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPipboyRadio_Eval.
/// 1 when the station's reference is the condition's subject (a station with no reference and no
/// subject match too, as JIP compares).
fn get_pipboy_radio_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let entry = e.mem.u32(PIPBOY_RADIO);
    let yes = entry != 0 && e.mem.u32(entry) == a.this_obj;
    one_if(e, a.result, yes);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetMoonTexture_Execute.
/// The masser moon's texture `id` (0 to 7) as a string; `""` otherwise.
fn get_moon_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let id = read.int(0, 0);
    let sky = e.mem.u32(SKY);
    let mut text = None;
    if read.ok && id <= 7 && sky != 0 {
        let moon = e.mem.u32(sky + SKY_MASSER);
        if moon != 0 {
            text = super::string_text(e, moon + MOON_TEXTURES + 8 * id);
        }
    }
    super::assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_FreezeTime_Execute.
/// 1 while time is frozen; with an argument, the flag set to it when it differs (JIP compares the
/// byte with `!toggle`).
fn freeze_time(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let globals = e.mem.u32(OS_GLOBALS);
    if globals == 0 {
        return handled(true);
    }
    let at = globals + FREEZE_TIME;
    one_if(e, a.result, flag(e, at));
    if super::jip_fn_utility::num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        let toggle = read.int(0, 0);
        if read.ok && e.mem.u8(at) == u8::from(toggle == 0) {
            e.mem.set_u8(at, u8::from(toggle != 0));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetDebugModeState_Execute.
/// 1 when the debug toggle `id` (0 TGM, 1 TAI, 2 TCAI, 3 TCL, 4 TB) is on.
fn get_debug_mode_state(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(&at) = DEBUG_MODE_BYTES.get(read.int(0, 0) as usize) {
            one_if(e, a.result, flag(e, at));
        }
    }
    handled(true)
}

// ---- time ----

/// A time global's value as JIP's `int` reads it (`cvttss2si`).
fn time_global(e: &Engine, which: u32) -> i32 {
    let global = e.mem.u32(TIME_GLOBALS + 4 * which);
    super::cvttss2si(e.mem.f32(global + GLOBAL_VALUE))
}

/// `kDaysPerMonth[i]`; 0 outside the table (JIP reads the bytes around it).
fn days_in(month: i32) -> i32 {
    usize::try_from(month)
        .ok()
        .and_then(|m| DAYS_PER_MONTH.get(m))
        .copied()
        .unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetDaysPassed.
/// The days from the date (its month from 0) to the game's date, plus the hour over 24, in
/// single precision: a later year counts the rest of the first year, the whole years between
/// (365 days) and this year's days; a later month of the same year the months between; else
/// the days apart (negative before the date).
fn days_passed(e: &Engine, bgn_year: i32, bgn_month: i32, bgn_day: i32) -> f32 {
    let (year, month, day) = (time_global(e, 0), time_global(e, 1), time_global(e, 2));
    let mut total: i32;
    let mut years = year.wrapping_sub(bgn_year);
    if years > 0 {
        total = days_in(bgn_month).wrapping_sub(bgn_day);
        for m in bgn_month.saturating_add(1)..12 {
            total = total.wrapping_add(days_in(m));
        }
        years -= 1;
        if years != 0 {
            total = total.wrapping_add(years.wrapping_mul(365));
        }
        for m in 0..month.min(12) {
            total = total.wrapping_add(days_in(m));
        }
        total = total.wrapping_add(day.wrapping_sub(1));
    } else if bgn_month < month {
        total = days_in(bgn_month).wrapping_sub(bgn_day);
        for m in bgn_month.saturating_add(1)..month.min(12) {
            total = total.wrapping_add(days_in(m));
        }
        total = total.wrapping_add(day.wrapping_sub(1));
    } else {
        total = day.wrapping_sub(bgn_day);
    }
    let hour = e.mem.f32(e.mem.u32(TIME_GLOBALS + 0xc) + GLOBAL_VALUE);
    total as f32 + hour * (1.0f32 / 24.0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetGameDaysPassed_Execute.
/// The days passed since the date (2281-10-13 by default), for a month of 12 or less (signed);
/// echoed in the console. A month outside 1 to 12 indexes outside JIP's table: deliberately no
/// result then.
fn get_game_days_passed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let year = read.int(0, 2281) as i32;
    let month = read.int(1, 10) as i32;
    let day = read.int(2, 13) as i32;
    if read.ok && (1..=12).contains(&month) {
        let days = days_passed(e, year, month - 1, day);
        a.set_result(e, f64::from(days));
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// ---- forms ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_StringToActorValue_Execute.
/// The code of the actor value named so (case-insensitive: JIP's map keys compare by
/// `StrHashCI` alone), -1 for none. The map is built at the first call from the 77 actor value
/// infos' names, a later one replacing an earlier of the same hash.
fn string_to_actor_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, 0x40, "StringToActorValue");
    if e.ext::<Jip>().misc.actor_value_ids.is_empty() {
        let mut ids = HashMap::new();
        for code in 0..ACTOR_VALUE_COUNT {
            let info = e.mem.u32(ACTOR_VALUE_INFOS + 4 * code);
            if info == 0 {
                continue;
            }
            // `StrHashCI(nullptr)` is 0 (its `test ecx, ecx; jz nullStr`): a null name is kept
            // under 0, as JIP keeps it.
            let text = e.mem.u32(info + AV_INFO_NAME);
            let hash = if text == 0 {
                0
            } else {
                str_hash_ci(&nvse::table::guest_text(e, text))
            };
            ids.insert(hash, code);
        }
        e.ext::<Jip>().misc.actor_value_ids = ids;
    }
    let found = e
        .ext::<Jip>()
        .misc
        .actor_value_ids
        .get(&str_hash_ci(&name))
        .copied();
    if let Some(code) = found {
        a.set_result(e, f64::from(code));
    }
    handled(true)
}

/// `IsHardcoreStage`: a radiation stage, or any type from the dehydration stage up.
fn is_hardcore_stage(e: &Engine, form: u32) -> bool {
    form != 0 && {
        let t = type_of(e, form);
        t == BGS_RADIATION_STAGE || t >= BGS_DEHYDRATION_STAGE
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetHardcoreStageThreshold_Execute.
/// The stage's threshold as a signed integer.
fn get_hardcore_stage_threshold(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let stage = read.form(0);
    if read.ok && is_hardcore_stage(e, stage) {
        let threshold = e.mem.u32(stage + STAGE_THRESHOLD) as i32;
        a.set_result(e, f64::from(threshold));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetHardcoreStageThreshold_Execute.
fn set_hardcore_stage_threshold(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let stage = read.form(0);
    if read.ok && is_hardcore_stage(e, stage) {
        e.mem.set_u32(stage + STAGE_THRESHOLD, read.int(1, 0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetHardcoreStageEffect_Execute.
fn get_hardcore_stage_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let stage = read.form(0);
    if read.ok && is_hardcore_stage(e, stage) {
        let effect = e.mem.u32(stage + STAGE_EFFECT);
        if effect != 0 {
            super::set_form_result(e, &a, effect);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetHardcoreStageEffect_Execute.
/// The stage's effect set to the spell given, or none without one.
fn set_hardcore_stage_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let stage = read.form(0);
    if read.ok && is_hardcore_stage(e, stage) {
        e.mem.set_u32(stage + STAGE_EFFECT, read.form(1));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetHardcoreTracking_Execute.
/// 1 while hardcore tracking is off (`kSerializedFlag_NoHardcoreTracking`, JIP's co-save flags).
fn get_hardcore_tracking(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let off = serialized_flags(e) & 1 != 0;
    one_if(e, a.result, off);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetHardcoreTracking_Eval.
/// As the command, from the lock-free copy of the flags (any thread, no shared state).
fn get_hardcore_tracking_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let off = serialized_flags(e) & 1 != 0;
    one_if(e, a.result, off);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetNoteRead_Execute.
fn get_note_read(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let note = read.form(0);
    let yes = read.ok && note != 0 && flag(e, note + NOTE_READ);
    one_if(e, a.result, yes);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetNoteRead_Eval.
/// The note is the condition's first parameter (no note: no).
fn get_note_read_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let yes = a.arg1 != 0 && flag(e, a.arg1 + NOTE_READ);
    one_if(e, a.result, yes);
    handled(true)
}

// ---- keywords ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// AssignKeywordRecourse.
/// Every form of a form list (base forms for references), form lists inside it walked the same
/// way. JIP recurses into each inner list every time it meets it; here each list is read once
/// (a set of the lists met, never emptied) from a work stack, not the Rust stack: the forms are
/// the same on every list JIP finishes (a set's insert does not depend on order or repeats),
/// and lists that share inner lists (JIP: exponential) or hold themselves (JIP: until its stack
/// runs out) or nest tens of thousands deep (JIP: its stack) cannot stop the game.
fn assign_list(e: &mut Engine, set: &mut BTreeSet<u32>, list: u32) {
    assign_list_reading(e, set, list, &mut |e, l| {
        list_items(e, Ptr::new(l + LIST_FORM_LIST))
    });
}

/// [`assign_list`] with the read of a list's items given (the tests count the reads).
fn assign_list_reading(
    e: &mut Engine,
    set: &mut BTreeSet<u32>,
    list: u32,
    items_of: &mut dyn FnMut(&Engine, u32) -> Vec<u32>,
) {
    let mut met = BTreeSet::from([list]);
    let mut stack = vec![list];
    while let Some(list) = stack.pop() {
        for form in items_of(e, list) {
            if form == 0 {
                continue;
            }
            if type_of(e, form) != BGS_LIST_FORM {
                let base = base_if_ref(e, form);
                set.insert(base);
            } else if met.insert(form) {
                stack.push(form);
            }
        }
    }
}

/// A keyword argument (`char keyword[0x80]`) and its `StrHashCI`.
fn keyword_hash(e: &Engine, read: &Read, i: usize, command: &str) -> u32 {
    str_hash_ci(&super::buffer_arg(e, read, i, 0x80, command))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_AssignKeyword_Execute.
/// The form (its base form for a reference; each form of a form list) given the keyword; the
/// keyword's set is made even when nothing goes in. The forms are found first, then added to the
/// keyword's set in one insert under the map's write lock (a set's union: the same as JIP's
/// inserts one by one).
fn assign_keyword(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if !read.ok || form == 0 {
        return handled(true);
    }
    let hash = keyword_hash(e, &read, 1, "AssignKeyword");
    let mut set = BTreeSet::new();
    if type_of(e, form) != BGS_LIST_FORM {
        let base = base_if_ref(e, form);
        set.insert(base);
    } else {
        assign_list(e, &mut set, form);
    }
    let mut map = e.ext_free::<Unlocked>().keywords.write();
    map.as_mut()
        .unwrap_or_else(|p| p.get_mut())
        .entry(hash)
        .or_default()
        .extend(set);
    handled(true)
}

/// Whether the keyword `hash` has `form` (the lock-free map: any thread).
fn keyword_has(e: &mut Engine, hash: u32, form: u32) -> bool {
    with_keywords(e, |map| {
        map.get(&hash).is_some_and(|set| set.contains(&form))
    })
}

/// The keyword `hash`'s forms in their set's order, or `None` when it has no set.
fn keyword_forms(e: &mut Engine, hash: u32) -> Option<Vec<u32>> {
    with_keywords(e, |map| {
        map.get(&hash).map(|set| set.iter().copied().collect())
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_HasKeyword_Execute.
fn has_keyword(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if read.ok && form != 0 {
        let hash = keyword_hash(e, &read, 1, "HasKeyword");
        let base = base_if_ref(e, form);
        let yes = keyword_has(e, hash, base);
        one_if(e, a.result, yes);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetKeywordForms_Execute.
/// An array of the keyword's forms in their set's order (by address); the array is made first,
/// so a keyword with no set (or arguments that do not read) gives an empty one.
fn get_keyword_forms(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let id = arrays::create_array(e, a.script, &[]);
    a.set_result(e, f64::from(id as i32));
    let read = Read::of(e, &a);
    if read.ok {
        let hash = keyword_hash(e, &read, 0, "GetKeywordForms");
        let forms = keyword_forms(e, hash).unwrap_or_default();
        for form in forms {
            arrays::append_element(e, id, &Element::Form(form));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_HasKeywordCond_Eval.
/// The condition's first parameter is the keyword's hash; 1 when its set has the subject's base
/// form (no subject: no). The lock-free keyword map: any thread, no shared state.
fn has_keyword_cond_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.this_obj != 0 {
        let base = crate::overrides::ref_base_form(e, a.this_obj);
        let yes = keyword_has(e, a.arg1, base);
        one_if(e, a.result, yes);
    }
    handled(true)
}

// ---- package C2: descriptions, gravity, the reticle, angles, the camera, time, ash piles,
// the dead ----

/// `BGSNote`: the note's text (`pNoteData` +0x6C, a `TESDescription` for type 1) and type
/// (`cType` +0x7C) (the load `005E84E0`); `TESLoadScreen`'s text (`BSStringT` +0x38, which its
/// load `005A6900` fills from `DESC`).
const BGS_NOTE: u8 = 0x31;
const NOTE_DATA: u32 = 0x6c;
const NOTE_TYPE: u32 = 0x7c;
const TES_LOAD_SCREEN: u8 = 0x4b;
const LOAD_SCREEN_TEXT: u32 = 0x38;
/// `TESDescription::GetDescription` (`00482E90`, `thiscall (form, chunk)`, `RET 8`), slot +0x10
/// of its vtable (`0101C18C`, stored by the constructor `00482C50`).
const DESCRIPTION_GET_SLOT: u32 = 0x10;
/// JIP's chunk ids `'MANT'` and `'CSED'` (`TNAM`, `DESC` as the file stores them).
const CHUNK_TNAM: u32 = 0x4d41_4e54;
const CHUNK_DESC: u32 = 0x4353_4544;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetDescriptionText.
/// A note of type 1's text (its description's `Get(form, 'TNAM')`), a load screen's text when its
/// length is not 0, else the form's `TESDescription` (`DYNAMIC_CAST`) `Get(form, 'DESC')`.
/// `None` for none. The text is the game's (the description's shared text, which JIP's
/// `SetFormDescription` hook may answer).
fn description_text(e: &mut Engine, form: u32) -> Option<Vec<u8>> {
    let (description, chunk) = match type_of(e, form) {
        BGS_NOTE => {
            let text = e.mem.u32(form + NOTE_DATA);
            if e.mem.u8(form + NOTE_TYPE) != 1 || text == 0 {
                return None;
            }
            (text, CHUNK_TNAM)
        }
        TES_LOAD_SCREEN => {
            let at = form + LOAD_SCREEN_TEXT;
            let length = e.get(
                Ptr::<crate::game::BSStringT>::new(at),
                crate::game::BSStringT::sLen,
            );
            return (length != 0).then(|| super::string_text(e, at)).flatten();
        }
        _ => {
            let description = nvse::commands::dynamic_cast(
                e,
                form,
                nvse::commands::rtti::TES_FORM,
                nvse::commands::rtti::TES_DESCRIPTION,
            );
            if description == 0 {
                return None;
            }
            (description, CHUNK_DESC)
        }
    };
    let text = e
        .vcall(description, DESCRIPTION_GET_SLOT, &args![form, chunk])
        .u32();
    (text != 0).then(|| nvse::table::guest_text(e, text))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetFormDescription_Execute.
/// The form's description text (`""` for none, or when the argument does not read).
fn get_form_description(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    let text = if read.ok && form != 0 {
        description_text(e, form)
    } else {
        None
    };
    super::assign_string(e, &a, text);
    handled(true)
}

/// `TES::currentInterior` (+0x34), `ExtraHavok` (type 1, its `bhkWorld` +0xC, read by `0041B9A0`)
/// in a cell's extra list (+0x28), the exterior `bhkWorldM` (`TESObjectCELL::spExteriorWorld`,
/// `*011CA0D8`), a `bhkWorld`'s `hkpWorld` (`refObject` +8) and its gravity (`hkVector4` +0x10:
/// the simulation reads it there every step, `hkpsimulation.rs`).
const EXTRA_HAVOK: u32 = 0x01;
const EXTRA_HAVOK_WORLD: u32 = 0x0c;
const CELL_EXTRA_LIST: u32 = 0x28;
const EXTERIOR_WORLD: u32 = 0x011c_a0d8;
const WORLD_GRAVITY: u32 = 0x10;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GethkpWorld.
/// The `hkpWorld` of the interior the game is in (its `ExtraHavok`), or of the exterior world;
/// 0 for none.
fn hkp_world(e: &mut Engine) -> u32 {
    let Some(tes) = h::ptr(e, h::TES) else {
        return 0;
    };
    let world = match h::ptr(e, tes + h::TES_INTERIOR) {
        Some(cell) => {
            let extra = extra::get_by_type(e, cell + CELL_EXTRA_LIST, EXTRA_HAVOK);
            if extra == 0 {
                0
            } else {
                e.mem.u32(extra + EXTRA_HAVOK_WORLD)
            }
        }
        None => e.mem.u32(EXTERIOR_WORLD),
    };
    if world == 0 {
        return 0;
    }
    e.mem.u32(world + h::HAVOK_OBJECT)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetLocalGravity_Execute.
/// The physics world's gravity on the axis (`X`, `Y`, `Z`); echoed in the console either way.
fn get_local_gravity(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let axis = super::jip_fn_misc_ref_havok::axis_arg(&read, 0);
    if read.ok {
        let world = hkp_world(e);
        // `kParamType_Axis` reads only X, Y and Z; anything else indexes nothing here.
        if world != 0 && (b'X'..=b'Z').contains(&axis) {
            let at = world + WORLD_GRAVITY + 4 * u32::from(axis - b'X');
            a.set_result(e, f64::from(e.mem.f32(at)));
        }
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetLocalGravityVector_Execute.
/// The physics world's gravity set to the three floats (a plain store: the simulation reads it
/// each step). No result.
fn set_local_gravity_vector(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let world = hkp_world(e);
    if world != 0 {
        let read = Read::of(e, &a);
        if read.ok {
            for i in 0..3u32 {
                let v = read.float(i as usize, 0.0);
                e.mem.set_f32(world + WORLD_GRAVITY + 4 * i, v);
            }
        }
    }
    handled(true)
}

/// The world scene graph (`*011DEB7C`) and its camera (+0xAC): JIP's `g_mainCamera` (lane H3's
/// `MoveToReticle` reads it so); the player's `CachedWorldCameraPos` (+0xDE0, JIP's
/// `cameraPos`), position (+0x30) and `eHardcoreSetting` (+0x7BC).
const SCENE_GRAPH: u32 = 0x011d_eb7c;
const SCENE_GRAPH_CAMERA: u32 = 0xac;
const PC_CAMERA_POS: u32 = 0xde0;
const PC_POSITION: u32 = 0x30;
const PC_HARDCORE: u32 = 0x7bc;
/// JIP's default cast range and layers.
const CAST_RANGE: f32 = 50000.0;
const LIMB_LAYER: i32 = 54;

/// The main camera's world translation (16 bytes) and world rotation.
fn main_camera(e: &Engine) -> Option<([f32; 4], [f32; 9])> {
    let camera = h::ptr(e, SCENE_GRAPH).and_then(|g| h::ptr(e, g + SCENE_GRAPH_CAMERA))?;
    super::jip_fn_misc_ref_havok::node_transform(e, camera)
}

/// `NiVector4::RayCastCoords` from the main camera along its view: the point and whether it
/// counts as found. `None` without the camera, `TES` or the player's controller (JIP reads
/// through them unchecked).
fn camera_coords(e: &mut Engine, range: f32, layer: i32) -> Option<([f32; 4], bool)> {
    let (pos, rot) = main_camera(e)?;
    let cast = h::ray_cast(e, pos, h::along_column_0(rot), range, layer)?;
    Some(h::hit_coords(e, &cast))
}

/// `GetRayCastObject(g_thePlayer->cameraPos, g_mainCamera->WorldRotate(), range, layer)`: the
/// block hit (0 for none).
fn reticle_object(e: &mut Engine, range: f32, layer: i32) -> u32 {
    let Some(player) = h::ptr(e, PLAYER) else {
        return 0;
    };
    let (Some(pos), Some((_, rot))) = (h::vec4(e, player + PC_CAMERA_POS), main_camera(e)) else {
        return 0;
    };
    h::ray_cast(e, pos, h::along_column_0(rot), range, layer).map_or(0, |c| c.object)
}

/// The player is in a cell.
fn player_in_cell(e: &Engine) -> bool {
    h::ptr(e, PLAYER).is_some_and(|p| h::ptr(e, p + PC_PARENT_CELL).is_some())
}

/// Script variable `i` (a `ScriptLocal`, its value +8).
fn set_var(e: &mut Engine, var: u32, v: f64) {
    if var != 0 {
        e.mem.set_f64(var + 8, v);
    }
}

/// `ResultVars::Set`: x, then y, then z, each a float widened to a double.
fn set_vars(e: &mut Engine, vars: [u32; 3], values: [f32; 3]) {
    for (var, v) in vars.into_iter().zip(values) {
        set_var(e, var, f64::from(v));
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetReticlePos_Execute.
/// `GetReticlePos [layer range]`: with the player in a cell, the point the main camera looks at
/// (layer -1, which casts on 6; range 50000) as an array of three numbers, when the cast counts
/// as found or both arguments were given.
fn get_reticle_pos(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let num_args = super::jip_fn_utility::num_args_ex(e, &a);
    if !player_in_cell(e) {
        return handled(true);
    }
    let (layer, range) = if num_args == 0 {
        (-1, CAST_RANGE)
    } else {
        let read = Read::of(e, &a);
        if !read.ok {
            return handled(true);
        }
        (read.int(0, u32::MAX) as i32, read.float(1, CAST_RANGE))
    };
    if let Some((coords, found)) = camera_coords(e, range, layer) {
        if found || num_args >= 2 {
            let elements = coords[..3]
                .iter()
                .map(|&c| Element::Number(f64::from(c)))
                .collect::<Vec<_>>();
            let id = arrays::create_array(e, a.script, &elements);
            a.set_result(e, f64::from(id as i32));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetReticleRange_Execute.
/// `GetReticleRange [layer range fromCam]`: -1, or with the player in a cell the distance to
/// the point the main camera looks at from the player's position (from the camera's with
/// `fromCam`), when the cast counts as found or two arguments or more were given.
fn get_reticle_range(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let num_args = super::jip_fn_utility::num_args_ex(e, &a);
    if !player_in_cell(e) {
        return handled(true);
    }
    let (layer, range, from_cam) = if num_args == 0 {
        (-1, CAST_RANGE, 0)
    } else {
        let read = Read::of(e, &a);
        if !read.ok {
            return handled(true);
        }
        (
            read.int(0, u32::MAX) as i32,
            read.float(1, CAST_RANGE),
            read.int(2, 0),
        )
    };
    let Some((coords, found)) = camera_coords(e, range, layer) else {
        return handled(true);
    };
    if found || num_args >= 2 {
        let from = if from_cam == 0 {
            let player = e.mem.u32(PLAYER);
            [0, 4, 8].map(|o| e.mem.f32(player + PC_POSITION + o))
        } else {
            match main_camera(e) {
                Some((pos, _)) => [pos[0], pos[1], pos[2]],
                None => return handled(true),
            }
        };
        let to = [coords[0], coords[1], coords[2]];
        let d = super::ln_fn_math::point3_distance(from, to);
        a.set_result(e, f64::from(d));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetReticlePosAlt_Execute.
/// `GetReticlePosAlt x y z [layer range]`: with the player in a cell, the point the main camera
/// looks at (layer 6, range 50000) into the three variables with result 1, when the cast counts
/// as found or all five arguments were given.
fn get_reticle_pos_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let num_args = super::jip_fn_utility::num_args(e, &a);
    let read = Read::of(e, &a);
    if !player_in_cell(e) || !read.ok {
        return handled(true);
    }
    let vars = [read.local(0), read.local(1), read.local(2)];
    let layer = read.int(3, 6) as i32;
    let range = read.float(4, CAST_RANGE);
    if let Some((coords, found)) = camera_coords(e, range, layer) {
        if found || num_args >= 5 {
            set_vars(e, vars, [coords[0], coords[1], coords[2]]);
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetReticleNode_Execute, internal/netimmerse.h: NiFixedString::Get.
/// `GetReticleNode [range layer]`: the name of the block the player's view hits (from its camera
/// position along the main camera's view; range 50000, layer 6): `"NULL"` for a block without a
/// name, `""` for no hit.
fn get_reticle_node(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut name = Vec::new();
    if read.ok {
        let range = read.float(0, CAST_RANGE);
        let layer = read.int(1, 6) as i32;
        let object = reticle_object(e, range, layer);
        if object != 0 {
            name = match h::ptr(e, object + h::NAME) {
                None => b"NULL".to_vec(),
                // nv-rs's own: a name with no end within 4 KiB is `""` (JIP reads on).
                Some(text) => h::c_text(e, text).unwrap_or_default(),
            };
        }
    }
    super::assign_string(e, &a, Some(name));
    handled(true)
}

/// `Actor::GetHitLocationForNode` (`008B3EF0`, `thiscall (actor, block)`, `RET 4`).
const HIT_LOCATION_FOR_NODE: u32 = 0x008b_3ef0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetReticleTargetLimb_Execute.
/// `GetReticleTargetLimb [range]`: -1, or the body part of the actor the player's view hits on
/// layer 54 (the game's `GetHitLocationForNode` of the block).
fn get_reticle_target_limb(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let range = read.float(0, CAST_RANGE);
    let object = reticle_object(e, range, LIMB_LAYER);
    if object == 0 {
        return handled(true);
    }
    let refr = h::block_parent_ref(e, object, &mut h::Walk::new());
    if refr != 0 && h::is_actor(e, refr) {
        let limb = e.call(HIT_LOCATION_FOR_NODE, &args![refr, object]).i32();
        a.set_result(e, f64::from(limb));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPointRayCastPos_Execute.
/// `GetPointRayCastPos x y z rx rz outX outY outZ [layer range]`: the point a cast from the
/// position along the inverse pitch-roll-yaw rotation's column 1 of the angles (degrees; roll 0)
/// hits (layer 6, range 50000), into the three variables with result 1, when it counts as found
/// or all ten arguments were given.
fn get_point_ray_cast_pos(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let num_args = super::jip_fn_utility::num_args(e, &a);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let pos = [
        read.float(0, 0.0),
        read.float(1, 0.0),
        read.float(2, 0.0),
        0.0,
    ];
    let rot = [read.float(3, 0.0), 0.0, read.float(4, 0.0)];
    let vars = [read.local(5), read.local(6), read.local(7)];
    let layer = read.int(8, 6) as i32;
    let range = read.float(9, CAST_RANGE);
    let m = super::ni_matrix::from_euler_pry_inv(rot.map(|v| v * super::FLT_PI_D_180));
    let Some(cast) = h::ray_cast(e, pos, m, range, layer) else {
        return handled(true);
    };
    let (coords, found) = h::hit_coords(e, &cast);
    if found || num_args >= 10 {
        set_vars(e, vars, [coords[0], coords[1], coords[2]]);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::From2Points
// (asm), its answer (`xmm6`'s first lane).
/// The length `From2Points` answers: 0 when the squared length is 0 or a NaN, `|z|` when only z
/// differs (the squares of x and y sum to 0 or a NaN), else the squared length times its
/// reciprocal square root (`rsqrtps` and one Newton step).
fn from_2_points_length(pt1: [f32; 3], pt2: [f32; 3]) -> f32 {
    let d = [pt1[0] - pt2[0], pt1[1] - pt2[1], pt1[2] - pt2[2]];
    let sq = [d[0] * d[0], d[1] * d[1], d[2] * d[2]];
    let len2 = (sq[0] + sq[1]) + (sq[2] + 0.0);
    let zero = |v: f32| v == 0.0 || v.is_nan();
    if zero(len2) {
        return 0.0;
    }
    if zero(sq[0] + sq[1]) {
        return d[2].abs();
    }
    let r = engine_core::x87::rsqrt_estimate(len2);
    len2 * (((3.0 - (len2 * r) * r) * r) * 0.5)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetP2PRayCastRange_Execute.
/// `GetP2PRayCastRange x1 y1 z1 x2 y2 z2 [layer]`: when a cast from the first point toward the
/// second, as long as the two are apart, counts as found (layer 6), the distance from its point
/// to the second point.
fn get_p2p_ray_cast_range(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let p1 = [read.float(0, 0.0), read.float(1, 0.0), read.float(2, 0.0)];
    let p2 = [read.float(3, 0.0), read.float(4, 0.0), read.float(5, 0.0)];
    let layer = read.int(6, 6) as i32;
    let m = super::ni_matrix::from_2_points(p1, p2);
    let length = from_2_points_length(p1, p2);
    let Some(cast) = h::ray_cast(e, [p1[0], p1[1], p1[2], 0.0], m, length, layer) else {
        return handled(true);
    };
    let (coords, found) = h::hit_coords(e, &cast);
    if found {
        let d = super::ln_fn_math::point3_distance([coords[0], coords[1], coords[2]], p2);
        a.set_result(e, f64::from(d));
    }
    handled(true)
}

/// A float with its sign bit flipped (`xorps` with the sign mask).
fn flip(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}

/// The two angles both maths routines end with: `[-ATan2(a2y, a2x), ATan2(a1y, a1x)]` in
/// degrees (times `Flt180dPI`), as `ResultVars::Set` writes x then z.
fn angles_out(e: &mut Engine, vars: [u32; 2], a1: f32, a2: f32) {
    let k = super::nif_nodes::FLT_180_D_PI;
    set_var(e, vars[0], f64::from(flip(a2) * k));
    set_var(e, vars[1], f64::from(a1 * k));
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: TransformWorldToLocal
// (asm).
/// The target's offset from the origin turned by the inverse pitch-roll-yaw rotation of the
/// angles (`r`, each row times the offset summed `(a0 + a1) + (a2 + a3)` as `haddps` twice, the
/// fourth product a word times 0), then `[ATan2(r0, r1), ATan2(r2, sqrt(r0² + r1²))]`.
fn transform_world_to_local(origin: [f32; 3], target: [f32; 3], rad: [f32; 3]) -> (f32, f32) {
    let m = super::ni_matrix::from_euler_pry_inv(rad);
    let v = [
        target[0] - origin[0],
        target[1] - origin[1],
        target[2] - origin[2],
        0.0,
    ];
    // The words `movups` reads: rows 0 and 1 reach into the next row's first word; row 2 the
    // word after the matrix (JIP's saved frame pointer: a stack address, finite, so its product
    // with the cleared lane is +0).
    let row = |r: [f32; 4]| ((r[0] * v[0]) + (r[1] * v[1])) + ((r[2] * v[2]) + (r[3] * v[3]));
    let r0 = row([m[0], m[1], m[2], m[3]]);
    let r1 = row([m[3], m[4], m[5], m[6]]);
    let r2 = row([m[6], m[7], m[8], 0.0]);
    let h = ((r0 * r0) + (r1 * r1)).sqrt();
    let a1 = super::ln_fn_math::atan2(r0, r1);
    let a2 = super::ln_fn_math::atan2(r2, h);
    (a1, a2)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_TransformWorldToLocal_Execute.
/// `TransformWorldToLocal ox oy oz tx ty tz rx ry rz outX outZ`: the target's angles seen from the
/// origin turned by the angles (degrees): x the pitch (`-ATan2(r2, h)`), z the heading
/// (`ATan2(r0, r1)`), in degrees. No result.
fn transform_world_to_local_cmd(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let f = |i: usize| read.float(i, 0.0);
        let origin = [f(0), f(1), f(2)];
        let target = [f(3), f(4), f(5)];
        let rad = [f(6), f(7), f(8)].map(|v| v * super::FLT_PI_D_180);
        let vars = [read.local(9), read.local(10)];
        let (a1, a2) = transform_world_to_local(origin, target, rad);
        angles_out(e, vars, a1, a2);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: GetAnglesBetweenPoints
// (asm).
/// `d = pt1 - pt2`, then `[ATan2(d0, -d1), ATan2(d2, sqrt(d0² + d1²))]`.
fn angles_between_points(pt1: [f32; 3], pt2: [f32; 3]) -> (f32, f32) {
    let d = [pt1[0] - pt2[0], pt1[1] - pt2[1], pt1[2] - pt2[2]];
    let h = ((d[0] * d[0]) + (d[1] * d[1])).sqrt();
    let a1 = super::ln_fn_math::atan2(d[0], flip(d[1]));
    let a2 = super::ln_fn_math::atan2(d[2], h);
    (a1, a2)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetAnglesBetweenPoints_Execute.
/// `GetAnglesBetweenPoints x1 y1 z1 x2 y2 z2 outX outZ`: x the pitch (`-ATan2(d2, h)`), z the
/// heading (`ATan2(d0, -d1)`) of the first point from the second, in degrees. No result.
fn get_angles_between_points(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let f = |i: usize| read.float(i, 0.0);
        let vars = [read.local(6), read.local(7)];
        let (a1, a2) = angles_between_points([f(0), f(1), f(2)], [f(3), f(4), f(5)]);
        angles_out(e, vars, a1, a2);
    }
    handled(true)
}

/// The right stick's words the game's pad poll writes (`011F35B4`, `011F35B6`:
/// `xcontrols.rs`), `OSInputGlobals` (`*011F35CC`) and its mouse movement (`DIMOUSESTATE` at
/// +0x1B24: x, then y), the settings `bInvertYValues:Controls` (`011E0A5C`) and
/// `fMouseSensitivity:Controls` (`011E0A6C`), their values at +4 (the player's look code reads
/// them, `playercharacter.rs`).
const RIGHT_STICK_X: u32 = 0x011f_35b4;
const RIGHT_STICK_Y: u32 = 0x011f_35b6;
const INPUT_GLOBALS: u32 = 0x011f_35cc;
const MOUSE_MOVEMENT: u32 = 0x1b24;
const INVERT_Y_VALUE: u32 = 0x011e_0a5c + 4;
const MOUSE_SENSITIVITY_VALUE: u32 = 0x011e_0a6c + 4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetCameraMovement_Execute.
/// `GetCameraMovement outX outY`: with a controller, the right stick (inside the dead zone 0)
/// times the mouse sensitivity over 1500 (y negated); else the mouse's movement times the
/// sensitivity; y negated first when Y is inverted. Both 0 when nothing moved; result 1 when
/// something did.
fn get_camera_movement(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (out_x, out_y) = (read.local(0), read.local(1));
    let controller = crate::input::ready(e);
    let x;
    let mut y;
    if controller {
        let dz = crate::input::dead_zone_rs(e);
        let stick = |v: u16| {
            let v = i32::from(v as i16);
            if v >= dz.wrapping_neg() && v <= dz {
                0
            } else {
                v
            }
        };
        x = stick(e.mem.u16(RIGHT_STICK_X));
        y = stick(e.mem.u16(RIGHT_STICK_Y));
    } else {
        let globals = e.mem.u32(INPUT_GLOBALS);
        if globals == 0 {
            return handled(true);
        }
        x = e.mem.u32(globals + MOUSE_MOVEMENT) as i32;
        y = e.mem.u32(globals + MOUSE_MOVEMENT + 4) as i32;
    }
    if x != 0 || y != 0 {
        if y != 0 && e.mem.u32(INVERT_Y_VALUE) != 0 {
            y = y.wrapping_neg();
        }
        let sensitivity = f64::from(e.mem.f32(MOUSE_SENSITIVITY_VALUE));
        let mut fx = f64::from(x) * sensitivity;
        let mut fy = f64::from(y) * sensitivity;
        if controller {
            fx *= 1.0 / 1500.0;
            fy *= -1.0 / 1500.0;
        }
        set_var(e, out_x, fx);
        set_var(e, out_y, fy);
        a.set_result(e, 1.0);
    } else {
        set_var(e, out_x, 0.0);
        set_var(e, out_y, 0.0);
    }
    handled(true)
}

/// The kept flag `NoHardcoreTracking` (`kSerializedFlag_NoHardcoreTracking`).
const NO_HARDCORE_TRACKING: u32 = 1;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetGameHour_Execute.
/// `SetGameHour hour`: for an hour of 0 or more, the hour global set to it when it is not below
/// the current hour, else to 24 plus it (a day later); then JIP's `s_forceHCNeedsUpdate` set to
/// whether the player is in hardcore mode and hardcore tracking is not off. No result.
fn set_game_hour(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let hour = read.float(0, 0.0);
    if !read.ok || hour.is_nan() || hour < 0.0 {
        return handled(true);
    }
    let global = e.mem.u32(TIME_GLOBALS + 0xc);
    if global == 0 {
        return handled(true);
    }
    let at = global + GLOBAL_VALUE;
    if e.mem.f32(at) <= hour {
        e.mem.set_f32(at, hour);
    } else {
        e.mem.set_f32(at, 24.0 + hour);
    }
    let hardcore = h::ptr(e, PLAYER).is_some_and(|p| e.mem.u32(p + PC_HARDCORE) != 0);
    let tracking = serialized_flags(e) & NO_HARDCORE_TRACKING == 0;
    crate::game_patches::gp::set_force_hardcore_needs_update(e, hardcore && tracking);
    handled(true)
}

/// `TESObjectACTI`; a form's flags (+8), bit 0 set on a marked activator.
const TES_OBJECT_ACTI: u8 = 0x15;
const FORM_FLAGS: u32 = 0x08;

/// `SetJIPFlag(kHookFormFlag6_IsAshPile, true)` and `flags |= 1`.
fn mark_ash_pile(e: &mut Engine, form: u32) {
    crate::world_hooks::set_jip_flag(e, form, crate::game_patches::gh2::IS_ASH_PILE, true);
    let flags = e.mem.u32(form + FORM_FLAGS);
    e.mem.set_u32(form + FORM_FLAGS, flags | 1);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_MarkActivatorAshPile_Execute.
/// `MarkActivatorAshPile form`: an activator, or each activator a form list holds (its own items,
/// not nested lists), gets JIP's ash pile bit (G06's tests then treat its references as ash
/// piles) and form flag bit 0. No result.
fn mark_activator_ash_pile(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if !read.ok || form == 0 {
        return handled(true);
    }
    match type_of(e, form) {
        TES_OBJECT_ACTI => mark_ash_pile(e, form),
        BGS_LIST_FORM => {
            for item in list_items(e, Ptr::new(form + LIST_FORM_LIST)) {
                if item != 0 && type_of(e, item) == TES_OBJECT_ACTI {
                    mark_ash_pile(e, item);
                }
            }
        }
        _ => {}
    }
    handled(true)
}

/// `ProcessManager` (`011E0E80`): its objects (`NiTArray` +4: data +8, capacity +0xC, a `u16`),
/// the high level's begin (+0x14) and end (+0x24) (the game's own dead-actor clean-up
/// `00977130` reads them so, `processlists.rs`). An actor's `lifeState` (+0x108, 2 dead), form
/// flags (+8; 0x200000 left out as `00576D30` tests it), base form (+0x20; its flag 0x400, quest
/// item), process (+0x68). A high process's level (+0x28), `eFadeState` (+0x3E8) and
/// `fGreetingTimer` (+0x330: JIP's `dyingTimer`, the timer the game's clean-up reads through slot
/// +0x4B0). `kXData_ExtraEnableStateChildren` 0x38.
const PM_OBJECTS: u32 = 0x08;
const PM_CAPACITY: u32 = 0x0c;
const PM_BEGIN_HIGH: u32 = 0x14;
const PM_END_HIGH: u32 = 0x24;
const ACTOR_LIFE_STATE: u32 = 0x108;
const ACTOR_BASE: u32 = 0x20;
const LIFE_DEAD: u32 = 2;
const PROCESS_FADE: u32 = 0x3e8;
const PROCESS_DYING_TIMER: u32 = 0x330;
const EXTRA_ENABLE_STATE_CHILDREN: u32 = 0x38;
/// `00577DE0` (`thiscall (actor)`: whether the actor's inventory holds something that keeps the
/// body, `004D0360`) and `HighProcess::FadeAndDisable` (`008FEB60`, `thiscall (process, actor)`,
/// `RET 4`).
const KEEPS_BODY: u32 = 0x0057_7de0;
const FADE_AND_DISABLE: u32 = 0x008f_eb60;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_ClearDeadActors_Execute.
/// Each high-level actor, last to first, that is dead, not flagged 0x200000, of a base that is
/// not a quest item, with a high process at level 0 not fading whose dying timer is below 0,
/// without `ExtraEnableStateChildren` and that `00577DE0` does not keep: faded out and disabled
/// by the game. No result.
///
/// nv-rs's own: the walk stays within the array's capacity and does nothing when the end is
/// not past the begin (JIP walks an unsigned difference); an actor without a base form is left
/// alone (JIP reads through it).
fn clear_dead_actors(e: &mut Engine, _: &[u32]) -> Ret {
    let pm = PROCESS_MANAGER;
    let data = e.mem.u32(pm + PM_OBJECTS);
    let capacity = u32::from(e.mem.u16(pm + PM_CAPACITY));
    let begin = e.mem.u32(pm + PM_BEGIN_HIGH);
    let end = e.mem.u32(pm + PM_END_HIGH).min(capacity);
    if data == 0 || end <= begin {
        return handled(true);
    }
    for i in (begin..end).rev() {
        let actor = e.mem.u32(data + 4 * i);
        if actor == 0 || !h::is_actor(e, actor) {
            continue;
        }
        let base = e.mem.u32(actor + ACTOR_BASE);
        if e.mem.u32(actor + ACTOR_LIFE_STATE) != LIFE_DEAD
            || e.mem.u32(actor + FORM_FLAGS) & 0x0020_0000 != 0
            || base == 0
            || e.mem.u32(base + FORM_FLAGS) & 0x400 != 0
        {
            continue;
        }
        let process = e.mem.u32(actor + PC_PROCESS);
        // `dyingTimer < 0` (`comiss`): false for a NaN.
        if process == 0
            || e.mem.u32(process + h::PROCESS_LEVEL) != 0
            || e.mem.u32(process + PROCESS_FADE) != 0
            || e.mem.f32(process + PROCESS_DYING_TIMER).is_nan()
            || e.mem.f32(process + PROCESS_DYING_TIMER) >= 0.0
            || extra::has_type(
                e,
                actor + extra::REFR_EXTRA_LIST,
                EXTRA_ENABLE_STATE_CHILDREN,
            )
            || e.call(KEEPS_BODY, &args![actor]).u32() & 0xff != 0
        {
            continue;
        }
        e.call(FADE_AND_DISABLE, &args![process, actor]);
    }
    handled(true)
}

/// The player's fields `ToggleSneak` reads (`playercharacter.rs`, `actor.rs`): `pActorMover`
/// +0x190, `eLifeState` +0x108, `cCurrentSitSleepState` +0x1AC, `ucControlsDisabled` +0x680 (its
/// bit 0x40), `bTemp1stPerson` +0x64F (JIP's `isUsingScope`); `GetIsParalyzed` vtable +0x234 (the
/// game's own sneak key code asks it first, `0x940D70`). The high process's knocked state
/// +0x13C (signed byte, `middlehighprocess.rs`), `sAnimAction` +0x2EC (a signed word,
/// `highprocess.rs`), `pCachedValues` +0x2C and their `iFlags` +0x44 (`actor_p2.rs`). The
/// `PlayerMover`'s `MoveMode` +0x94 and `ForceMoveMode` (vtable `0109297C` slot +0xC,
/// `009EA3E0`, one word).
const PC_MOVER: u32 = 0x190;
const PC_LIFE_STATE: u32 = 0x108;
const PC_SIT_SLEEP_STATE: u32 = 0x1ac;
const PC_CONTROLS_DISABLED: u32 = 0x680;
const PC_USING_SCOPE: u32 = 0x64f;
const IS_PARALYZED_SLOT: u32 = 0x234;
const PROCESS_KNOCKED: u32 = 0x13c;
const PROCESS_ACTION: u32 = 0x2ec;
const PROCESS_CACHED: u32 = 0x2c;
const CACHED_FLAGS: u32 = 0x44;
const MOVER_FLAGS: u32 = 0x94;
const FORCE_MOVE_MODE_SLOT: u32 = 0x0c;
const SNEAK: u32 = 0x400;
/// JIP's `(bool*)0x94423D`, read at the current action: the exe's own byte table at
/// `FalloutNV.exe 0x94423C`, read at the action plus one (the sneak key's case table: `0x940E2B`
/// sends action + 1 above 0x12 to the blocked path, `0x940E3E` reads the byte, `0x940E45` jumps
/// on it; case 0 goes on to sneak, case 1 is blocked). The 19 bytes are copied here because the
/// table sits in the exe's code section, which the port does not map into game memory.
const ACTION_BLOCKS: [u8; 19] = [0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0];
/// `0093AB30` (`thiscall (player, 0xA + was sneaking)`, `RET 4`), `Actor::SetIronSights`
/// (`008BB650`, three bytes, `RET 0xC`), `Actor::SetBlock` (`00894CC0`, one byte, `RET 4`).
const SNEAK_CHANGED: u32 = 0x0093_ab30;
const SET_IRON_SIGHTS: u32 = 0x008b_b650;
const SET_BLOCK: u32 = 0x0089_4cc0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: PlayerCharacter::ToggleSneak.
/// With a high process (level 0) and a mover: when the sneak flag differs from `on` and the
/// player is alive, not sitting or sleeping, its controls' bit 0x40 clear, not knocked, its
/// action not one that blocks sneaking and not paralysed: the mover's flags with 0x400 set or
/// cleared (its `ForceMoveMode`), `0093AB30(0xA + was sneaking)`, and without the scope iron
/// sights and blocking ended; the cached walk and run speeds marked stale (flags 0x3000).
///
/// nv-rs's own: past the table (actions below -1 or above 17, which the game never sets) JIP
/// reads the padding and the next jump table (non-zero for actions 18 to 21, but 0 for action
/// 22); nv-rs blocks every such action, as the exe's own range check at `0x940E2B` does.
fn toggle_sneak(e: &mut Engine, pc: u32, on: bool) {
    let Some(process) = h::ptr(e, pc + PC_PROCESS) else {
        return;
    };
    let Some(mover) = h::ptr(e, pc + PC_MOVER) else {
        return;
    };
    if e.mem.u32(process + h::PROCESS_LEVEL) != 0 {
        return;
    }
    let flags = e.mem.u32(mover + MOVER_FLAGS);
    let sneaking = flags & SNEAK != 0;
    let action = i32::from(e.mem.u16(process + PROCESS_ACTION) as i16);
    // Outside the table (`None`) blocks, as the exe's range check does.
    let blocks = usize::try_from(action + 1)
        .ok()
        .and_then(|i| ACTION_BLOCKS.get(i))
        != Some(&0);
    if sneaking == on
        || e.mem.u32(pc + PC_LIFE_STATE) != 0
        || e.mem.u32(pc + PC_SIT_SLEEP_STATE) != 0
        || e.mem.u8(pc + PC_CONTROLS_DISABLED) & 0x40 != 0
        || e.mem.u8(process + PROCESS_KNOCKED) != 0
        || blocks
        || e.vcall(pc, IS_PARALYZED_SLOT, &[]).u32() & 0xff != 0
    {
        return;
    }
    let flags = if on { flags | SNEAK } else { flags & !SNEAK };
    e.vcall(mover, FORCE_MOVE_MODE_SLOT, &args![flags]);
    e.call(SNEAK_CHANGED, &args![pc, 0xa + u32::from(sneaking)]);
    if e.mem.u8(pc + PC_USING_SCOPE) == 0 {
        e.call(SET_IRON_SIGHTS, &args![pc, 0u32, 0u32, 0u32]);
        e.call(SET_BLOCK, &args![pc, 0u32]);
    }
    if let Some(cached) = h::ptr(e, process + PROCESS_CACHED) {
        let f = e.mem.u32(cached + CACHED_FLAGS);
        e.mem.set_u32(cached + CACHED_FLAGS, f & !0x3000);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_TogglePlayerSneaking_Execute.
/// `TogglePlayerSneaking on`: the player made to sneak (not 0) or stand ([`toggle_sneak`]). No
/// result.
fn toggle_player_sneaking(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let on = read.int(0, 0) != 0;
    if let Some(pc) = h::ptr(e, PLAYER) {
        toggle_sneak(e, pc, on);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_miscellaneous_tests.rs"]
mod tests;
