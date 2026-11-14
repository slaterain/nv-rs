//! `functions_jip/jip_fn_miscellaneous.h` (JIP LN NVSE): game state the player and the world keep
//! (difficulty, combat, detection, scope and iron sights, run and auto-move, kill cam, the Pip-Boy
//! radio, the enemy health bar's target, the last exterior door, time frozen, the debug toggles),
//! container respawns, idle loop counts, nav meshes switched off and on, terrain height, exterior
//! and buffered cells, the moon's textures, days passed since a date, hardcore stages, exact XP
//! rewards, actor values by name and JIP's keywords. 40 of its 83 commands (85 in the file, two of
//! them never registered: `SetWobblesRotation`, `PlaceModel`).
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
//! Deferred (each waits for what is named; no stand-ins):
//! - `MarkActivatorAshPile` (G06), `GetConditionDamagePenalty`/`SetConditionDamagePenalty` (G84),
//!   `SetArmorConditionPenalty` (G85), `SetDamageToArmorMaxPercent` (G99),
//!   `ToggleHardcoreTracking` (G114), `ToggleImmortalMode` (G122/G123), `ToggleHitEffects`
//!   (G123-G125), `ToggleNoMovementCombat` (G126), `ToggleDepthClear` (G128),
//!   `ToggleBipedSlotVisibility` (H15), `SetGameHour` (its flag is read by the time patch, O5/G37,
//!   with G114's): the stage 3.9 game patches that read what they set.
//! - `SetInternalMarker`: JIP's hook reading `s_internalMarkerIDs` (hooks.h; core track).
//! - `SetMoonTexture`, `ToggleCameraCollision`, `InitRockItLauncher`: they patch the game's code.
//! - `SwapTextureEx`, `GetLightAmountAtPoint`, `GetReticlePos`, `GetReticlePosAlt`,
//!   `GetReticleRange`, `GetReticleNode`, `GetReticleTargetLimb`, `GetPointRayCastPos`,
//!   `GetP2PRayCastRange`, `GetLocalGravity`, `SetLocalGravityVector`: the scene graph, ray casts
//!   and the physics world.
//! - Left for the next batch (buildable): `GetFormDescription` (the `TESDescription` RTTI and its
//!   `Get`), `ClearDeadActors`, `TogglePlayerSneaking`, `GetCameraMovement` (script variable
//!   outputs), `TransformWorldToLocal`, `GetAnglesBetweenPoints` (JIP's SSE maths, script
//!   variable outputs).

use super::arrays::{self, Element};
use super::{list_items, ref_id, type_of, Handlers};
use crate::containers::str_hash_ci;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};
use std::collections::{BTreeSet, HashMap};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x26e9, get_idle_loop_times),
    (0x26ed, disable_nav_mesh_alt),
    (0x26ee, enable_nav_mesh_alt),
    (0x26ef, get_terrain_height),
    (0x2721, get_container_respawns),
    (0x2722, set_container_respawns),
    (0x2736, get_exterior_cell),
    (0x2737, get_cell_buffered),
    (0x2740, toggle_first_person),
    (0x2770, set_idle_loop_times),
    (0x277b, get_pc_last_exterior_door),
    (0x27a3, get_moon_texture),
    (0x27b0, free_player),
    (0x282f, get_game_days_passed),
    (0x2838, set_game_difficulty),
    (0x283d, get_enemy_health_target),
    (0x2853, get_buffered_cells),
    (0x28a5, string_to_actor_value),
    (0x28b6, get_hardcore_stage_threshold),
    (0x28b7, set_hardcore_stage_threshold),
    (0x28b8, get_hardcore_stage_effect),
    (0x28b9, set_hardcore_stage_effect),
    (0x28bd, get_debug_mode_state),
    (0x28bf, freeze_time),
    (0x28ed, reward_xp_exact),
    (0x293a, assign_keyword),
    (0x293b, has_keyword),
    (0x293c, get_keyword_forms),
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

/// What the file's commands keep between calls (`TempObject`s: never saved, never cleared).
#[derive(Default)]
pub struct State {
    /// `s_keywordFormsMap`: each keyword's `StrHashCI` and its forms (JIP's `Set<TESForm*>`, a
    /// sorted array: in address order).
    pub keywords: HashMap<u32, BTreeSet<u32>>,
    /// `s_actorValueIDsMap`, built at the first `StringToActorValue`: each actor value's name by
    /// its `StrHashCI` (JIP's `const char*` keys compare by the hash alone), its code.
    pub actor_value_ids: HashMap<u32, u32>,
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

/// A `TES` cell buffer and its slot count: the exteriors' (or the interiors').
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

/// The cells of a buffer, from its first slot to the first empty one, at most `slots`.
fn buffered_cells(e: &Engine, exteriors: bool) -> Vec<u32> {
    let (mut at, mut slots) = cell_buffer(e, exteriors);
    let mut cells = Vec::new();
    if at == 0 {
        return cells;
    }
    while slots != 0 {
        let cell = e.mem.u32(at);
        if cell == 0 {
            break;
        }
        cells.push(cell);
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
            let text = e.mem.u32(info + AV_INFO_NAME);
            if text == 0 {
                continue;
            }
            let hash = str_hash_ci(&nvse::table::guest_text(e, text));
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
    let off = e.ext::<Jip>().saved.serialized.flags & 1 != 0;
    one_if(e, a.result, off);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetHardcoreTracking_Eval.
fn get_hardcore_tracking_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let off = e.ext::<Jip>().saved.serialized.flags & 1 != 0;
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
/// way. A list already being walked is skipped (JIP recurses into it until its stack runs out).
fn assign_list(e: &mut Engine, set: &mut BTreeSet<u32>, list: u32, walking: &mut Vec<u32>) {
    if walking.contains(&list) {
        return;
    }
    walking.push(list);
    for form in list_items(e, Ptr::new(list + LIST_FORM_LIST)) {
        if form == 0 {
            continue;
        }
        if type_of(e, form) != BGS_LIST_FORM {
            let base = base_if_ref(e, form);
            set.insert(base);
        } else {
            assign_list(e, set, form, walking);
        }
    }
    walking.pop();
}

/// A keyword argument (`char keyword[0x80]`) and its `StrHashCI`.
fn keyword_hash(e: &Engine, read: &Read, i: usize, command: &str) -> u32 {
    str_hash_ci(&super::buffer_arg(e, read, i, 0x80, command))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_AssignKeyword_Execute.
/// The form (its base form for a reference; each form of a form list) given the keyword; the
/// keyword's set is made even when nothing goes in.
fn assign_keyword(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if !read.ok || form == 0 {
        return handled(true);
    }
    let hash = keyword_hash(e, &read, 1, "AssignKeyword");
    let mut set = e
        .ext::<Jip>()
        .misc
        .keywords
        .remove(&hash)
        .unwrap_or_default();
    if type_of(e, form) != BGS_LIST_FORM {
        let base = base_if_ref(e, form);
        set.insert(base);
    } else {
        assign_list(e, &mut set, form, &mut Vec::new());
    }
    e.ext::<Jip>().misc.keywords.insert(hash, set);
    handled(true)
}

/// Whether the keyword `hash` has `form`.
fn keyword_has(e: &mut Engine, hash: u32, form: u32) -> bool {
    e.ext::<Jip>()
        .misc
        .keywords
        .get(&hash)
        .is_some_and(|set| set.contains(&form))
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
        let forms: Vec<u32> = e
            .ext::<Jip>()
            .misc
            .keywords
            .get(&hash)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default();
        for form in forms {
            arrays::append_element(e, id, &Element::Form(form));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_HasKeywordCond_Eval.
/// The condition's first parameter is the keyword's hash; 1 when its set has the subject's base
/// form (no subject: no).
fn has_keyword_cond_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.this_obj != 0 {
        let base = crate::overrides::ref_base_form(e, a.this_obj);
        let yes = keyword_has(e, a.arg1, base);
        one_if(e, a.result, yes);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_miscellaneous_tests.rs"]
mod tests;
