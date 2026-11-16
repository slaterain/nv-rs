//! JIP LN NVSE's world hooks (step 3.5c; `internal/hooks.h` (JIP LN NVSE)): descriptions, model
//! paths and reference names a script set, items made unique for repairs, the dialogue topic,
//! fast travel, cell change, target change, note and quest stage events, the player's activation
//! a script disabled, scripts' events and activations a script disabled, teammates passing
//! locked doors, the weather's transition time, swapped LOD blocks, the player kept above 1
//! health, and scripts that wait; with the commands that switch them on.
//!
//! The seams are in the translations, each behind its hook's user count (one load while it has
//! none); the handlers here answer them as JIP's patched code does. Nothing here runs while JIP
//! answers "not installed": the handlers and the commands are put in only when it answers
//! "installed".
//!
//! Not here: `ToggleImmortalMode`, the only user of the PlayerMinHealth hook (its handler is
//! here): it is built with JIP's `PlayerCannotBeHit` patch in `crate::game_patches::gp` (step 3.9). `CCCSetEquipped` and JIP's always-on
//! `GetCannotWear` patch (`0x418B10`) are in `equip_hooks` (step 3.5d). The getters that only read what these
//! commands set (`GetInteractionDisabled`, `GetInteractionDisabledType`,
//! `GetScriptEventDisabled`, `IsScriptWaiting`) are the commands track's (step 3.6), over
//! [`State`].

use crate::game::{JipFormFlags, TESForm, TESObjectREFR};
use crate::Jip;
use engine_core::extend::{Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::collections::{BTreeMap, BTreeSet};

/// The commands' opcodes.
pub const SET_FORM_DESCRIPTION: u32 = 0x2747;
pub const GET_PC_FAST_TRAVELLED: u32 = 0x2748;
pub const GET_PC_MOVED_CELL: u32 = 0x2749;
pub const TOGGLE_IGNORE_LOCKED_DOORS: u32 = 0x275f;
pub const SET_ON_FAST_TRAVEL: u32 = 0x2799;
pub const SET_WEATHER_TRANSITION_TIME_OVERRIDE: u32 = 0x27a6;
pub const SET_ON_PC_TARGET_CHANGE: u32 = 0x27af;
pub const SET_INTERACTION_DISABLED: u32 = 0x27c9;
pub const SET_INTERACTION_DISABLED_TYPE: u32 = 0x27cb;
pub const SET_SCRIPT_EVENT_DISABLED: u32 = 0x27e6;
pub const SET_REF_NAME: u32 = 0x27f4;
pub const SET_ON_QUEST_STAGE: u32 = 0x27f7;
pub const SET_ON_DIALOG_TOPIC: u32 = 0x27fc;
pub const SCRIPT_WAIT: u32 = 0x2810;
pub const SWAP_OBJECT_LOD: u32 = 0x287d;
pub const TOGGLE_ITEM_UNIQUE: u32 = 0x28a1;
pub const SET_ON_NOTE_ADDED: u32 = 0x28b2;
pub const PUSH_ACTOR_NO_RAGDOLL: u32 = 0x28ca;
pub const SET_REFR_MODEL_PATH: u32 = 0x2917;
/// The nineteen.
pub const COMMANDS: [u32; 19] = [
    SET_FORM_DESCRIPTION,
    GET_PC_FAST_TRAVELLED,
    GET_PC_MOVED_CELL,
    TOGGLE_IGNORE_LOCKED_DOORS,
    SET_ON_FAST_TRAVEL,
    SET_WEATHER_TRANSITION_TIME_OVERRIDE,
    SET_ON_PC_TARGET_CHANGE,
    SET_INTERACTION_DISABLED,
    SET_INTERACTION_DISABLED_TYPE,
    SET_SCRIPT_EVENT_DISABLED,
    SET_REF_NAME,
    SET_ON_QUEST_STAGE,
    SET_ON_DIALOG_TOPIC,
    SCRIPT_WAIT,
    SWAP_OBJECT_LOD,
    TOGGLE_ITEM_UNIQUE,
    SET_ON_NOTE_ADDED,
    PUSH_ACTOR_NO_RAGDOLL,
    SET_REFR_MODEL_PATH,
];

/// The seams whose handlers are here (engine-core `SITES`, owner JIP: step 3.5c's, and 3.5b's
/// MarkScriptEvent seam `0x5AC763`).
pub const SITES: [u32; 19] = [
    0x0047_bbec,
    0x0048_2f7f,
    0x0050_fe8b,
    0x0055_d520,
    0x0057_3512,
    0x0057_3673,
    0x005a_c763,
    0x005e_0ec9,
    0x005e_1716,
    0x0060_f4a9,
    0x0061_f184,
    0x0063_d4f5,
    0x006f_6df1,
    0x0093_b80a,
    0x0093_bf22,
    0x0094_3279,
    0x0094_cb35,
    0x0096_4675,
    0x0096_6af9,
];

/// Each command's hook and every site JIP patches for it (`InitJIPHooks`): a command may add
/// users only while all of them have seams. `SetScriptEventDisabled` adds to two hooks.
pub const COMMAND_SITES: [(u32, &[u32]); 18] = [
    (SET_FORM_DESCRIPTION, &[0x0048_2f7f]),
    (GET_PC_FAST_TRAVELLED, &[0x0093_bf22]),
    (GET_PC_MOVED_CELL, &[0x0094_cb35]),
    (TOGGLE_IGNORE_LOCKED_DOORS, &[0x0057_3673]),
    (SET_ON_FAST_TRAVEL, &[0x0093_bf22]),
    (SET_WEATHER_TRANSITION_TIME_OVERRIDE, &[0x0063_d4f5]),
    (SET_ON_PC_TARGET_CHANGE, &[0x0096_4675]),
    (SET_INTERACTION_DISABLED, &[0x0094_3279, 0x0077_5b9b]),
    (SET_INTERACTION_DISABLED_TYPE, &[0x0094_3279, 0x0077_5b9b]),
    (SET_SCRIPT_EVENT_DISABLED, &[0x005a_c763, 0x0057_3512]),
    (SET_REF_NAME, &[0x0055_d520]),
    (SET_ON_QUEST_STAGE, &[0x0060_f4a9]),
    (SET_ON_DIALOG_TOPIC, &[0x0061_f184]),
    (SCRIPT_WAIT, &[0x005e_0ec9, 0x005e_1716]),
    (SWAP_OBJECT_LOD, &[0x006f_6df1]),
    (TOGGLE_ITEM_UNIQUE, &[0x0047_bbec]),
    (SET_ON_NOTE_ADDED, &[0x0096_6af9]),
    (SET_REFR_MODEL_PATH, &[0x0050_fe8b]),
];

/// `kHookFormFlag6_*`, `kHookFormFlag5_*`, `kHookRefFlag5F_*` (`internal/hooks.h` (JIP LN
/// NVSE)).
const SET_STAGE_HANDLERS: u8 = 1 << 0;
const NO_PC_ACTIVATION: u8 = 1 << 1;
const EVENT_DISABLED: u8 = 1 << 2;
const ACTIVATE_DISABLED: u8 = 1 << 3;
pub(crate) const UNIQUE_ITEM: u8 = 1 << 4;
const FAST_TRAVEL_INFORMED: u8 = 1 << 0;
const CELL_CHANGE_INFORMED: u8 = 1 << 1;
const SCRIPT_INFORMED: u8 = 0x07;
const SCRIPT_ON_WAIT: u8 = 1 << 3;
const ALT_REF_NAME: u8 = 1 << 2;
const REFR_MODEL_PATH: u8 = 1 << 3;
/// `kFormType_*`, `kXData_ExtraScript`, `kXData_ExtraLock`.
const SCRIPT: u8 = 0x11;
const ARMOR: u8 = 0x18;
const WEAPON: u8 = 0x28;
const NOTE: u8 = 0x31;
const WORLD_SPACE: u8 = 0x41;
const TOPIC: u8 = 0x45;
const TOPIC_INFO: u8 = 0x46;
const QUEST: u8 = 0x47;
const LIST_FORM: u8 = 0x55;
const EXTRA_SCRIPT: u32 = 0x0d;
const EXTRA_LOCK: u32 = 0x2a;
/// `BaseExtraList::GetByType` (`thiscall`: list, type).
const GET_BY_TYPE: u32 = 0x0041_0220;
/// The shared description text (`0x11C5498`, a `String`) and its owner (`0x11C5490`,
/// `GameGlobals::CurrentDescription`): engine-fallout-forms `tesdescription.rs`.
const DESCRIPTION_TEXT: u32 = 0x011c_5498;
const DESCRIPTION_OWNER: u32 = 0x011c_5490;
/// `TESTopic::GetOwnerTopic` (`0061a310`, one stack word: the info): the topic holding an info.
const GET_OWNER_TOPIC: u32 = 0x0061_a310;
/// `TESForm::GetEditorID`'s vtable slot (`+0x130`).
const GET_EDITOR_ID: u32 = 0x130;
/// The vtable entry that answers true (`ADDR_ReturnTrue`), `IsReference`'s slot (`+0xF0`).
const RETURN_TRUE: u32 = 0x008d_0360;
const VT_IS_REFERENCE: u32 = 0xf0;
/// `TESObjectREFR`'s vtable (JIP's `kVtbl_TESObjectREFR`) and the word `0x108` JIP's
/// `IsValidReference` also accepts at `+2` (the vtables of the classes above it).
const VTBL_TESOBJECTREFR: u32 = 0x0102_f55c;
/// `s_eventMasks` (`jip_nvse.cpp` (JIP LN NVSE)): the event names `SetScriptEventDisabled`
/// takes and their masks, in JIP's list order; looked up by `StrHashCI` alone, as JIP's
/// `UnorderedMap<const char*>` keeps only the hash.
const EVENT_MASKS: [(&str, u32); 30] = [
    ("OnActivate", 0),
    ("OnAdd", 1),
    ("OnEquip", 2),
    ("OnActorEquip", 2),
    ("OnDrop", 4),
    ("OnUnequip", 8),
    ("OnActorUnequip", 8),
    ("OnDeath", 0x10),
    ("OnMurder", 0x20),
    ("OnCombatEnd", 0x40),
    ("OnHit", 0x80),
    ("OnHitWith", 0x100),
    ("OnPackageStart", 0x200),
    ("OnPackageDone", 0x400),
    ("OnPackageChange", 0x800),
    ("OnLoad", 0x1000),
    ("OnMagicEffectHit", 0x2000),
    ("OnSell", 0x4000),
    ("OnStartCombat", 0x8000),
    ("OnOpen", 0x10000),
    ("OnClose", 0x20000),
    ("SayToDone", 0x40000),
    ("OnGrab", 0x80000),
    ("OnRelease", 0x100000),
    ("OnDestructionStageChange", 0x200000),
    ("OnFire", 0x400000),
    ("OnTrigger", 0x10000000),
    ("OnTriggerEnter", 0x20000000),
    ("OnTriggerLeave", 0x40000000),
    ("OnReset", 0x80000000),
];
/// The item events JIP's MarkScriptEvent filters by the item (`0x40400F`: OnAdd, OnEquip, OnDrop,
/// OnUnequip, OnSell, OnFire).
const ITEM_EVENTS: u32 = 0x0040_400f;
const ON_FIRE: u32 = 0x0040_0000;

/// A quest stage's script (`QuestStageEventCallback`): the stage, 1 or 2 (2: the stage is not
/// set when every script that ran for it asked so), the script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestStageEvent {
    pub stage: u8,
    pub skip: u8,
    pub script: u32,
}

/// A waiting script (`ScriptWaitInfo`): its owner, the owner's id, the blocks left to wait, the
/// offset of the block the wait is in, the line to resume at (from the block), and the runner's
/// `if` stack when it began.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptWait {
    pub owner: u32,
    pub ref_id: u32,
    pub iterations: u32,
    pub block_offset: u32,
    pub saved_offset: u32,
    pub saved_if_stack: [u32; 12],
}

/// A reference's own name or model path: a NUL-terminated text at `at` in a block of the game's
/// heap of `capacity` bytes (a power of two, at least 0x80, JIP's buffer). JIP copies each text
/// and frees the old copy when a reference's text is replaced or removed and when the model
/// paths are dropped before a load; the port keeps one block per reference instead, writes a
/// new text that fits over the old one, and hands a block it no longer needs to the next text
/// (never back to the heap). So the memory follows the references that have a text, however
/// often a script renames them, and a pointer the game was handed earlier always reads a text:
/// the reference's newer one, or another's, never memory the heap gave to something else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextBlock {
    pub at: u32,
    pub capacity: u32,
}

/// JIP's world hook state.
#[derive(Clone, Debug)]
pub struct State {
    /// `s_descriptionChanges`: a description's own text.
    pub description_changes: BTreeMap<u32, Vec<u8>>,
    /// `s_eventInformedObjects`: the ids of the forms told of a fast travel or a cell change.
    pub event_informed: BTreeSet<u32>,
    /// `s_pcFastTravelInformed`, `s_pcCellChangeInformed`.
    pub fast_travel_informed: BTreeSet<u32>,
    pub cell_change_informed: BTreeSet<u32>,
    /// `PCCellChangeHook`'s `pcLastCell`.
    pub last_cell: u32,
    /// `s_fastTravelEventScripts`, `s_targetChangeEventScripts`, `s_noteAddedEventScripts`.
    pub fast_travel: BTreeSet<u32>,
    pub target_change: BTreeSet<u32>,
    pub note_added: BTreeSet<u32>,
    /// `SetPCTargetHook`'s `lastTarget`.
    pub last_target: u32,
    /// `s_dialogTopicEventMap`: by topic or topic info.
    pub dialog_topic: BTreeMap<u32, BTreeSet<u32>>,
    /// `s_questStageEventMap`: by quest, in the order added.
    pub quest_stage: BTreeMap<u32, Vec<QuestStageEvent>>,
    /// `s_weatherTransitionRateOverride`.
    pub weather_rate: f32,
    /// `s_swapObjLODMap`: by world space, the cells (`Coordinate`: x high, y low).
    pub swap_obj_lod: BTreeMap<u32, BTreeSet<u32>>,
    /// `s_refNamesMap`, `s_refrModelPathMap`: a reference's name or model path, each in a block
    /// of the game's heap the reference keeps while it has one (see [`TextBlock`]).
    pub ref_names: BTreeMap<u32, TextBlock>,
    pub model_paths: BTreeMap<u32, TextBlock>,
    /// The blocks a reference gave back (its text removed, or the model paths dropped before a
    /// load), handed to the next text that fits: JIP frees its copy there.
    pub free_texts: Vec<TextBlock>,
    /// `s_disabledEventsMap`: a form's disabled events.
    pub disabled_events: BTreeMap<u32, u32>,
    /// `s_scriptWaitInfoMap` by owner (`s_scriptWaitInfo`, the owner whose wait this run is, is
    /// per thread: [`current_wait`]).
    pub script_waits: BTreeMap<u32, ScriptWait>,
    /// Raised by each pre-load that clears the waits (JIP nulls its one `s_scriptWaitInfo` there for
    /// every thread): a thread's current wait taken before it is no wait ([`current_wait`]).
    pub wait_generation: u32,
    /// `s_jipFormFlagsMap`: the flag byte a form was last given (`TESForm::SetJIPFlag`), by id;
    /// [`restore_form_flags`] puts them back after a load and at a new game.
    pub form_flags: BTreeMap<u32, u8>,
}

impl Default for State {
    fn default() -> Self {
        State {
            description_changes: BTreeMap::new(),
            event_informed: BTreeSet::new(),
            fast_travel_informed: BTreeSet::new(),
            cell_change_informed: BTreeSet::new(),
            last_cell: 0,
            fast_travel: BTreeSet::new(),
            target_change: BTreeSet::new(),
            note_added: BTreeSet::new(),
            last_target: 0,
            dialog_topic: BTreeMap::new(),
            quest_stage: BTreeMap::new(),
            weather_rate: 4.0,
            swap_obj_lod: BTreeMap::new(),
            ref_names: BTreeMap::new(),
            model_paths: BTreeMap::new(),
            free_texts: Vec::new(),
            disabled_events: BTreeMap::new(),
            script_waits: BTreeMap::new(),
            wait_generation: 0,
            form_flags: BTreeMap::new(),
        }
    }
}

/// JIP's world hook state for every engine of the run (JIP's globals): a lock-free slot
/// ([`Engine::ext_free`], made at install before the threads start) holding the one [`State`]
/// behind a std `RwLock`. The model path and name seams run on the loader and AI threads, the
/// script runner, block, event mask, quest stage and result script seams on the AI threads
/// (package and result scripts), the reference destructor (game_patches::gh2) on any thread;
/// each reads one value or a copy of a script list. The lock is a leaf: a guard borrows the
/// engine, so no game call, script call or second slot access can happen while it is held (the
/// compiler refuses them), and it never waits on JIP's or xNVSE's shared slot.
#[derive(Default)]
pub struct Unlocked(std::sync::RwLock<State>);

/// Makes the slot (before the threads start).
pub(crate) fn make_slot(e: &mut Engine) {
    e.ext_free::<Unlocked>();
}

/// The state to change (the commands, the pre-load, the reference destructor).
pub(crate) fn state(e: &mut Engine) -> std::sync::RwLockWriteGuard<'_, State> {
    e.ext_free::<Unlocked>()
        .0
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The state to read (the seams).
fn read(e: &mut Engine) -> std::sync::RwLockReadGuard<'_, State> {
    e.ext_free::<Unlocked>()
        .0
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

thread_local! {
    /// `s_scriptWaitInfo`: the owner whose wait this thread's script run is (set at each run's
    /// start, [`script_runner_site`]), or none. One value per thread: on one thread exactly
    /// JIP's one global (a nested run overwrites it, as JIP's does); JIP's one global for every
    /// thread would let a run on an AI thread take another thread's wait and resume this script
    /// at that script's saved line (a deliberate difference). Every guest thread is its own OS
    /// thread, with stepped and real threads alike (as xNVSE's thread-locals in `nvse::events`).
    /// Kept with the waits' generation it was taken in ([`State::wait_generation`]): JIP's
    /// pre-load nulls its global for every thread, the port's pre-load runs on one, so a value
    /// from before a pre-load counts as none on every thread.
    static WAIT_INFO: std::cell::Cell<Option<(u32, u32)>> = const { std::cell::Cell::new(None) };
}

/// The owner whose wait this thread's script run is (`s_scriptWaitInfo`), as it was set (the
/// block seam also checks its generation).
#[cfg(test)]
pub(crate) fn current_wait() -> Option<u32> {
    WAIT_INFO.with(std::cell::Cell::get).map(|(owner, _)| owner)
}

/// Sets this thread's `s_scriptWaitInfo`: an owner with the generation it was taken in.
fn set_wait(wait: Option<(u32, u32)>) {
    WAIT_INFO.with(|w| w.set(wait));
}

/// Clears this thread's `s_scriptWaitInfo`.
pub(crate) fn clear_current_wait() {
    set_wait(None);
}

/// Puts the handlers and the commands in, when JIP answers "installed".
pub fn install(e: &mut Engine, answers_installed: bool) {
    if !answers_installed {
        return;
    }
    type Handler = fn(&mut Engine, &[u32]) -> Option<u32>;
    let sites: [(u32, Handler); 19] = [
        (0x0047_bbec, check_unique_item_site),
        (0x0048_2f7f, get_description_site),
        (0x0050_fe8b, get_model_path_site),
        (0x0055_d520, get_ref_name_site),
        (0x0057_3512, do_activate_site),
        (0x0057_3673, activate_door_site),
        (0x005a_c763, mark_script_event_site),
        (0x005e_0ec9, script_runner_site),
        (0x005e_1716, eval_event_block_site),
        (0x0060_f4a9, set_quest_stage_site),
        (0x0061_f184, run_result_script_site),
        (0x0063_d4f5, update_weather_site),
        (0x006f_6df1, make_obj_lod_path_site),
        (0x0093_b80a, player_min_health_site),
        (0x0093_bf22, pc_fast_travel_site),
        (0x0094_3279, pc_activate_ref_site),
        (0x0094_cb35, pc_cell_change_site),
        (0x0096_4675, set_pc_target_site),
        (0x0096_6af9, add_note_site),
    ];
    for (addr, handler) in sites {
        e.on_site(SiteSlot::new(Owner::Jip, addr), handler);
    }
    use crate::table::set_handlers;
    set_handlers(e, SET_FORM_DESCRIPTION, Some(set_form_description), None);
    set_handlers(e, GET_PC_FAST_TRAVELLED, Some(get_pc_fast_travelled), None);
    set_handlers(e, GET_PC_MOVED_CELL, Some(get_pc_moved_cell), None);
    set_handlers(
        e,
        TOGGLE_IGNORE_LOCKED_DOORS,
        Some(toggle_ignore_locked_doors),
        None,
    );
    set_handlers(e, SET_ON_FAST_TRAVEL, Some(set_on_fast_travel), None);
    set_handlers(
        e,
        SET_WEATHER_TRANSITION_TIME_OVERRIDE,
        Some(set_weather_transition_time_override),
        None,
    );
    set_handlers(
        e,
        SET_ON_PC_TARGET_CHANGE,
        Some(set_on_pc_target_change),
        None,
    );
    set_handlers(
        e,
        SET_INTERACTION_DISABLED,
        Some(set_interaction_disabled),
        None,
    );
    set_handlers(
        e,
        SET_INTERACTION_DISABLED_TYPE,
        Some(set_interaction_disabled_type),
        None,
    );
    set_handlers(
        e,
        SET_SCRIPT_EVENT_DISABLED,
        Some(set_script_event_disabled),
        None,
    );
    set_handlers(e, SET_REF_NAME, Some(set_ref_name), None);
    set_handlers(e, SET_ON_QUEST_STAGE, Some(set_on_quest_stage), None);
    set_handlers(e, SET_ON_DIALOG_TOPIC, Some(set_on_dialog_topic), None);
    set_handlers(e, SCRIPT_WAIT, Some(script_wait), None);
    set_handlers(e, SWAP_OBJECT_LOD, Some(swap_object_lod), None);
    set_handlers(e, TOGGLE_ITEM_UNIQUE, Some(toggle_item_unique), None);
    set_handlers(e, SET_ON_NOTE_ADDED, Some(set_on_note_added), None);
    set_handlers(
        e,
        PUSH_ACTOR_NO_RAGDOLL,
        Some(crate::world_push::push_actor_no_ragdoll),
        None,
    );
    set_handlers(e, SET_REFR_MODEL_PATH, Some(set_refr_model_path), None);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h:
// DoPreLoadGameHousekeeping (this step's part).
/// Before a game loads: the informed forms lose their flags and the fast-travel and cell-change
/// sets go (with a user of each hook); the waits go, with their owners' flags and the hook; the
/// swapped LOD blocks go, with the hook; the references keep their names (their flag set
/// again); the model paths go, with the hook.
pub fn pre_load(e: &mut Engine) {
    if !state(e).event_informed.is_empty() {
        let ids = std::mem::take(&mut state(e).event_informed);
        for id in ids {
            let form = crate::cosave::lookup_form(e, id);
            if form != 0 {
                let flags = flags5(e, form);
                set_flags5(e, form, flags & !SCRIPT_INFORMED);
            }
        }
        if !state(e).fast_travel_informed.is_empty() {
            state(e).fast_travel_informed.clear();
            e.add_hook_user(Hook::PCFastTravel, false);
        }
        if !state(e).cell_change_informed.is_empty() {
            state(e).cell_change_informed.clear();
            e.set_hook(Hook::PCCellChange, false);
        }
    }
    if !state(e).script_waits.is_empty() {
        let waits = std::mem::take(&mut state(e).script_waits);
        for wait in waits.values() {
            let form = crate::cosave::lookup_form(e, wait.ref_id);
            if form != 0 {
                let flags = flags5(e, form);
                set_flags5(e, form, flags & !SCRIPT_ON_WAIT);
            }
        }
        e.set_hook(Hook::ScriptWait, false);
        // JIP nulls `s_scriptWaitInfo` here for every thread: this thread's value goes, and every
        // other thread's counts as none from now ([`State::wait_generation`]).
        {
            let mut s = state(e);
            s.wait_generation = s.wait_generation.wrapping_add(1);
        }
        clear_current_wait();
    }
    if !state(e).swap_obj_lod.is_empty() {
        state(e).swap_obj_lod.clear();
        e.set_hook(Hook::MakeObjLODPath, false);
    }
    let names: Vec<u32> = state(e).ref_names.keys().copied().collect();
    for reference in names {
        let flags = ref_flags(e, reference);
        set_ref_flags(e, reference, flags | ALT_REF_NAME);
    }
    if !state(e).model_paths.is_empty() {
        // JIP frees each path here: the blocks go to the free list.
        {
            let mut s = state(e);
            let paths = std::mem::take(&mut s.model_paths);
            s.free_texts.extend(paths.into_values());
        }
        e.set_hook(Hook::GetModelPath, false);
    }
}

// ---- reads ----

fn form_type(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<TESForm>::new(form), TESForm::cFormType)
}

fn form_id(e: &Engine, form: u32) -> u32 {
    e.get(Ptr::<TESForm>::new(form), TESForm::iFormID)
}

fn flags5(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<JipFormFlags>::new(form), JipFormFlags::jipFormFlags5)
}

fn set_flags5(e: &mut Engine, form: u32, flags: u8) {
    e.set(
        Ptr::<JipFormFlags>::new(form),
        JipFormFlags::jipFormFlags5,
        flags,
    );
}

pub(crate) fn flags6(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<JipFormFlags>::new(form), JipFormFlags::jipFormFlags6)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: TESForm::SetJIPFlag.
/// The form's `jipFormFlags6` bit set or cleared, and the byte kept by the form's id.
pub(crate) fn set_jip_flag(e: &mut Engine, form: u32, flag: u8, on: bool) {
    let flags = flags6(e, form);
    let flags = if on { flags | flag } else { flags & !flag };
    e.set(
        Ptr::<JipFormFlags>::new(form),
        JipFormFlags::jipFormFlags6,
        flags,
    );
    let id = form_id(e, form);
    state(e).form_flags.insert(id, flags);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h: RestoreJIPFormFlags
// (called first in DoLoadGameHousekeeping after a load that succeeded, and at a new game:
// jip_nvse.cpp NVSEMessageHandler).
/// Each recorded form's `jipFormFlags6` written back from its byte (a load rebuilt the form from
/// the plugins, flags cleared); an id no form has any more, or a byte of 0, leaves the record.
pub fn restore_form_flags(e: &mut Engine) {
    if state(e).form_flags.is_empty() {
        return;
    }
    let records: Vec<(u32, u8)> = state(e)
        .form_flags
        .iter()
        .map(|(&id, &flags)| (id, flags))
        .collect();
    for (id, flags) in records {
        let form = crate::cosave::lookup_form(e, id);
        if form != 0 && flags != 0 {
            e.set(
                Ptr::<JipFormFlags>::new(form),
                JipFormFlags::jipFormFlags6,
                flags,
            );
        } else {
            state(e).form_flags.remove(&id);
        }
    }
}

fn ref_flags(e: &Engine, reference: u32) -> u8 {
    e.get(
        Ptr::<TESObjectREFR>::new(reference),
        TESObjectREFR::jipRefFlags5F,
    )
}

fn set_ref_flags(e: &mut Engine, reference: u32, flags: u8) {
    e.set(
        Ptr::<TESObjectREFR>::new(reference),
        TESObjectREFR::jipRefFlags5F,
        flags,
    );
}

fn base_of(e: &Engine, reference: u32) -> u32 {
    e.get(
        Ptr::<TESObjectREFR>::new(reference),
        TESObjectREFR::pObjectReference,
    )
}

/// `IS_REFERENCE(form)` (JIP LN NVSE): its vtable answers true at `+0xF0`.
fn is_reference(e: &Engine, form: u32) -> bool {
    e.mem.u32(e.mem.u32(form) + VT_IS_REFERENCE) == RETURN_TRUE
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: IsValidReference.
/// Bit 2 of the object's first byte (a game object's vtable is 4-aligned: always set for one
/// whose vtable is not, an extra data list's flag otherwise) and either the word `0x108` at
/// `+2` (the high word of the vtables from `0x01080000`) or the vtable `TESObjectREFR`'s.
fn is_valid_reference(e: &Engine, object: u32) -> bool {
    if e.mem.u8(object) & 4 == 0 {
        return false;
    }
    e.mem.u16(object + 2) == 0x108 || e.mem.u32(object) == VTBL_TESOBJECTREFR
}

/// `BaseExtraList::GetByType` on a reference's extra list (`+0x44`).
fn extra_of(e: &mut Engine, reference: u32, kind: u32) -> u32 {
    e.call(GET_BY_TYPE, &args![reference + 0x44, kind]).u32()
}

/// `EventCallbackScripts::InvokeEvents*`: each script of a copy of `scripts`, on `this_obj`
/// with `args`.
fn invoke(e: &mut Engine, scripts: &BTreeSet<u32>, this_obj: u32, args: &[u32]) {
    for script in scripts.clone() {
        crate::script_call::call(e, script, this_obj, args.to_vec());
    }
}

// ---- the seams ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetDescriptionHook.
/// `TESDescription::LoadDescription` after its owner is stored (`0x482F7F`: the description): a
/// description with its own text sets the shared text to it (JIP's `String::Set`) and the load
/// ends.
fn get_description_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let text = state(e).description_changes.get(&a[0]).cloned()?;
    crate::commands::string_set(e, DESCRIPTION_TEXT, &text);
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: CheckUniqueItemHook.
/// The item test's second type read (`0x47BBEC`: the item, the other item's type): an item of
/// the same type that a script made unique answers 0 (no match).
fn check_unique_item_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (item, other_type) = (a[0], a[1]);
    (u32::from(form_type(e, item)) == other_type && flags6(e, item) & UNIQUE_ITEM != 0).then_some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetModelPathHook.
/// The model path call of the reference's model (`0x50FE8B`: the base form, the reference): a
/// reference with its own path answers it.
fn get_model_path_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let reference = a[1];
    if reference == 0 || ref_flags(e, reference) & REFR_MODEL_PATH == 0 {
        return None;
    }
    read(e).model_paths.get(&reference).map(|b| b.at)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetRefNameHook.
/// The reference's name (`0x55D520`, the entry: the reference): a reference with its own name
/// answers it.
fn get_ref_name_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let reference = a[0];
    if ref_flags(e, reference) & ALT_REF_NAME == 0 {
        return None;
    }
    read(e).ref_names.get(&reference).map(|b| b.at)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DoActivateHook.
/// The "activated" test before an activation's scripts run (`0x573512`: the reference): a
/// reference or base whose activation a script disabled counts as activated (JIP's rest is the
/// exe's own test, `0041b370`: no ExtraAction answers activated too).
fn do_activate_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let reference = a[0];
    let disabled = flags6(e, reference) & ACTIVATE_DISABLED != 0
        || flags6(e, base_of(e, reference)) & ACTIVATE_DISABLED != 0;
    disabled.then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ActivateDoorHook.
/// A door's activation after its teleport read (`0x573673`: the door, the activator, the
/// teleport): a door leading nowhere, activated by a teammate (byte `+0x18D`, read whatever the
/// activator is, as JIP does) while it is locked (its `ExtraLock` data's flag byte `+8`),
/// answers true at once.
fn activate_door_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (door, activator, teleport) = (a[0], a[1], a[2]);
    if teleport != 0 || activator == 0 || e.mem.u8(activator + 0x18d) == 0 {
        return None;
    }
    let lock = extra_of(e, door, EXTRA_LOCK);
    if lock == 0 {
        return None;
    }
    let data = e.mem.u32(lock + 0xc);
    (data != 0 && e.mem.u8(data + 8) != 0).then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: MarkScriptEventHook,
// IsValidReference.
/// `Script::SetActionFlag` before the locals are read (`0x5AC763`: the form, the extra list, the
/// flags; the caller's first stack word from `Engine::caller_word`): no script or no event list
/// answers false; a disabled script (data length 0) true; an event a script disabled for the
/// reference, its base, the item or the item's base (by the item events' rules) true, nothing
/// set; else the exe's code.
fn mark_script_event_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (form, list, flags) = (a[0], a[1], a[2]);
    let extra = e.call(GET_BY_TYPE, &args![list, EXTRA_SCRIPT]).u32();
    if extra == 0 || e.mem.u32(extra + 0x10) == 0 {
        return Some(0);
    }
    let script = e.mem.u32(extra + 0xc);
    if e.mem.u32(script + 0x20) == 0 {
        return Some(1);
    }
    let check = if flags & ITEM_EVENTS == 0 {
        Check::NotItem
    } else if form == 0 {
        Check::Continue
    } else if flags & ON_FIRE != 0 {
        Check::Base(form)
    } else if !is_valid_reference(e, form) {
        Check::NotItem
    } else {
        // The caller's first stack word; a caller that set none hands 0 (JIP's null test).
        let item = e.caller_word().unwrap_or(0);
        if item == 0 {
            Check::Continue
        } else if flags & 0xa != 0 {
            Check::Base(item)
        } else if is_valid_reference(e, item) {
            if flags & 1 != 0 {
                Check::Base(base_of(e, item))
            } else {
                return Some(1);
            }
        } else {
            Check::Base(item)
        }
    };
    let base = match check {
        Check::Continue => return None,
        Check::Base(form) => form,
        Check::NotItem => {
            let reference = list.wrapping_sub(0x44);
            if !is_valid_reference(e, reference) {
                return None;
            }
            if flags6(e, reference) & EVENT_DISABLED != 0
                && flags & disabled_mask(e, reference) != 0
            {
                return Some(1);
            }
            base_of(e, reference)
        }
    };
    if flags6(e, base) & EVENT_DISABLED == 0 {
        return None;
    }
    (flags & disabled_mask(e, base) != 0).then_some(1)
}

/// What MarkScriptEvent tests next.
enum Check {
    /// The exe's code (`contRetn`).
    Continue,
    /// This form's disabled events (`checkBase`).
    Base(u32),
    /// The reference owning the list, then its base (`notItem`).
    NotItem,
}

fn disabled_mask(e: &mut Engine, form: u32) -> u32 {
    read(e).disabled_events.get(&form).copied().unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ScriptRunnerHook.
/// `ScriptRunner::Run` before the header read (`0x5E0EC9`: the script, the reference): for an
/// object or quest script whose owner (the quest, or the reference) waits, this run is that
/// wait; else none.
fn script_runner_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (script, reference) = (a[0], a[1]);
    let kind = e.mem.u16(script + 0x28);
    let mut wait = None;
    if kind <= 1 {
        let owner = if kind == 1 {
            e.mem.u32(script + 0x40)
        } else {
            reference
        };
        if owner != 0 && flags5(e, owner) & SCRIPT_ON_WAIT != 0 {
            let s = read(e);
            if s.script_waits.contains_key(&owner) {
                wait = Some((owner, s.wait_generation));
            }
        }
    }
    set_wait(wait);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: EvalEventBlockHook.
/// `EvaluateLine`'s block test after the handler (`0x5E1716`: the runner, the block type, the
/// cursor, the result's high word, `action_list`, the length word's address, the span): a block
/// applies when the result's high word is not 0 and it is not the action list (JIP tests only
/// that word). In the waiting block (by its offset), an event block (type above 1) or one that
/// applies counts the wait down; at 0 the script resumes at the saved line with the saved `if`
/// stack and the owner stops waiting. Otherwise a block that does not apply is skipped.
fn eval_event_block_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (runner, kind, cursor, result_high, action_list, length, span) =
        (a[0], a[1], a[2], a[3], a[4], a[5], a[6]);
    let applies = result_high != 0 && action_list == 0;
    let mut skip = !applies;
    if let Some((owner, generation)) = WAIT_INFO.with(std::cell::Cell::get) {
        // One write of the entry, as JIP's decrement through its pointer: the count-down and
        // the copy read back under one lock. A wait taken before a pre-load is none (JIP's
        // pre-load nulled its pointer).
        let (stale, counted) = {
            let mut s = state(e);
            if s.wait_generation != generation {
                (true, None)
            } else {
                (
                    false,
                    match s.script_waits.get_mut(&owner) {
                        Some(w) if w.block_offset == cursor => {
                            if kind > 1 || applies {
                                w.iterations = w.iterations.wrapping_sub(1);
                                Some(Some(*w))
                            } else {
                                Some(None)
                            }
                        }
                        _ => None,
                    },
                )
            }
        };
        if stale {
            clear_current_wait();
        }
        if let Some(counted) = counted {
            if let Some(wait) = counted {
                if wait.iterations == 0 {
                    let at = e.mem.u32(length).wrapping_add(wait.saved_offset);
                    e.mem.set_u32(length, at);
                    for (i, word) in wait.saved_if_stack.iter().enumerate() {
                        e.mem.set_u32(runner + 0x20 + 4 * i as u32, *word);
                    }
                    clear_current_wait();
                    let flags = flags5(e, wait.owner);
                    set_flags5(e, wait.owner, flags & !SCRIPT_ON_WAIT);
                    return Some(1);
                }
            }
            skip = true;
        }
    }
    if skip {
        let at = e.mem.u32(length).wrapping_add(span);
        e.mem.set_u32(length, at);
    }
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetQuestStageHook,
// HandleSetQuestStage.
/// A quest stage set done (`0x60F4A9`: the stage, the quest), in place of the exe's tests: a
/// stage not done yet is marked done at once; a done one goes on only for a quest whose stages
/// repeat (`+0x3C` bit 3). Then, for a quest with stage scripts, each script of this stage is
/// called with the quest and the stage; the stage is not set (0) when the scripts that ran all
/// asked so (the OR of their 1s and 2s is 2).
fn set_quest_stage_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (stage, quest) = (a[0], a[1]);
    if e.mem.u8(stage + 1) & 1 == 0 {
        e.mem.set_u8(stage + 1, 1);
    } else if e.mem.u8(quest + 0x3c) & 8 == 0 {
        return Some(0);
    }
    if flags6(e, quest) & SET_STAGE_HANDLERS == 0 {
        return Some(1);
    }
    let id = e.mem.u8(stage);
    let events = read(e).quest_stage.get(&quest).cloned().unwrap_or_default();
    let mut skip = 0u8;
    for event in events {
        if event.stage != id {
            continue;
        }
        crate::script_call::call(e, event.script, 0, vec![quest, u32::from(id)]);
        skip |= event.skip;
    }
    Some(u32::from(skip != 2))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: RunResultScriptHook.
/// `TESTopicInfo::RunResult` after its reference test (`0x61F184`: the info, the index, the
/// reference): for the first result script, the info's scripts are called on the reference with
/// the info, then its topic's with the topic. JIP reads the word it adds to every info at `+0x50`
/// (`parentTopic`), which its always-on game patch "Store parent TESTopic for every TESTopicInfo"
/// fills (step 3.9a, `game_patches::dialogue`): with that patch on (JIP installed and the
/// host's row `NV_JIP_GAME_PATCHES`), the port's record of it, read here with no call as JIP
/// reads its word; without it (the vanilla preset), `TESTopic::GetOwnerTopic`'s answer, the
/// exe's walk (the first topic holding the info, where JIP's word is the last linked: they
/// differ only for an info a mod links under two topics), so this handler and the game's own
/// callers always see the same topic. The walk runs only for a first result script while a
/// script has a dialogue topic handler.
fn run_result_script_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (info, index, reference) = (a[0], a[1], a[2]);
    if index != 0 {
        return None;
    }
    let scripts = read(e).dialog_topic.get(&info).cloned();
    if let Some(scripts) = scripts {
        invoke(e, &scripts, reference, &[info]);
    }
    let topic = if e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0 {
        crate::game_patches::dialogue::parent_topic(e, info)
    } else {
        e.call(GET_OWNER_TOPIC, &args![info]).u32()
    };
    let scripts = read(e).dialog_topic.get(&topic).cloned();
    if let Some(scripts) = scripts {
        invoke(e, &scripts, reference, &[topic]);
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: UpdateWeatherHook.
/// The weather blend (`0x63D4F5`: the hours since the last update, an `f64`): the hours times
/// `s_weatherTransitionRateOverride` (x87 maths), stored as a float.
fn update_weather_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let hours = f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32);
    let rate = state(e).weather_rate;
    let blend = engine_core::x87::x87_mul(hours, f64::from(rate)) as f32;
    Some(blend.to_bits())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: MakeObjLODPathHook.
/// A LOD block's file name (`0x6F6DF1`: the world space, the level, x, y, the buffer): a cell a
/// script swapped (its 16-bit x and y) gets the `AltLOD` name, made from the world space's
/// editor id, the level and the whole x and y words (JIP's `sprintf_s` into the 0x104 bytes).
/// A name that does not fit stops the port here, as `sprintf_s`'s invalid-parameter handler
/// ends the game with JIP; the exe's own block names (`0x6F55F0`, `0x6F5640`, `0x6F5690`, the
/// same pattern into 0x104 bytes through `_vsprintf_s`) stop it the same way, at an editor id
/// only about three characters longer (101 characters stop here: the name is 59 bytes and the
/// editor id twice).
fn make_obj_lod_path_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (world_space, level, x, y, buffer) = (a[0], a[1], a[2], a[3], a[4]);
    let key = (x & 0xffff) << 16 | (y & 0xffff);
    if !state(e)
        .swap_obj_lod
        .get(&world_space)
        .is_some_and(|cells| cells.contains(&key))
    {
        return None;
    }
    let edid = e.vcall(world_space, GET_EDITOR_ID, &[]).u32();
    let edid = nvse::table::guest_text(e, edid);
    let edid = String::from_utf8_lossy(&edid).into_owned();
    let name = format!(
        "Data\\Meshes\\Landscape\\LOD\\{edid}\\Blocks\\{edid}.Level{}.X{}.Y{}.AltLOD.nif",
        level as i32, x as i32, y as i32
    );
    assert!(
        name.len() < 0x104,
        "MakeObjLODPath: a LOD block name of {} bytes does not fit JIP's 0x104-byte buffer: \
         sprintf_s ends the game there, the port stops here",
        name.len()
    );
    e.mem.write(buffer, name.as_bytes());
    e.mem.set_u8(buffer + name.len() as u32, 0);
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: PlayerMinHealthHook.
/// The player's health change (`0x93B80A`: the health, the change, `f32` bits): a change
/// (sign bit set) that would leave the health below 1 becomes `1 - health`.
fn player_min_health_site(_: &mut Engine, a: &[u32]) -> Option<u32> {
    if a[1] as i32 >= 0 {
        return None;
    }
    let (health, change) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
    // `comiss` then `jnb`: kept when not below 1 (a NaN is below).
    if !(health + change).is_nan() && health + change >= 1.0 {
        return None;
    }
    Some((1.0f32 - health).to_bits())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: PCFastTravelHook.
/// Before the player's fast travel moves (`0x93BF22`: the target): the fast-travel scripts are
/// called with it, and the forms told of the last one are forgotten.
fn pc_fast_travel_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let scripts = state(e).fast_travel.clone();
    if !scripts.is_empty() {
        invoke(e, &scripts, 0, &[a[0]]);
    }
    state(e).fast_travel_informed.clear();
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: PCActivateRefHook.
/// The activate button's activation of the reference under the crosshair (`0x943279`: the
/// reference): a reference or base a script disabled, or a base of a disabled type, is not
/// activated.
fn pc_activate_ref_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let reference = a[0];
    if flags6(e, reference) & NO_PC_ACTIVATION != 0 {
        return Some(1);
    }
    let base = base_of(e, reference);
    if flags6(e, base) & NO_PC_ACTIVATION != 0 {
        return Some(1);
    }
    let kind = usize::from(form_type(e, base));
    e.ext::<Jip>()
        .ui_hooks
        .activation_disabled_types
        .get(kind)
        .copied()
        .unwrap_or(false)
        .then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: PCCellChangeHook.
/// The player's cell change (`0x94CB35`: the cell left): a cell left that is not the last one
/// seen becomes it, and the forms told of the last change are forgotten.
fn pc_cell_change_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let cell = a[0];
    if cell != 0 && cell != state(e).last_cell {
        state(e).last_cell = cell;
        state(e).cell_change_informed.clear();
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetPCTargetHook.
/// After the player's auto-aim scan (`0x964675`: the player): a target actor (`+0xD44`) other
/// than the last one seen becomes it, and the target-change scripts are called with it.
fn set_pc_target_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let target = e.mem.u32(a[0] + 0xd44);
    if target != state(e).last_target {
        state(e).last_target = target;
        let scripts = state(e).target_change.clone();
        invoke(e, &scripts, 0, &[target]);
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: AddNoteHook.
/// A note the player gets (`0x966AF9`: the note): the note scripts are called with it.
fn add_note_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let scripts = state(e).note_added.clone();
    invoke(e, &scripts, 0, &[a[0]]);
    None
}

// ---- the commands ----

/// The script argument read as a script form, or `None` (the command does nothing).
fn script_arg(e: &Engine, read: &Read) -> Option<u32> {
    let script = read.form(0);
    (read.ok && script != 0 && form_type(e, script) == SCRIPT).then_some(script)
}

/// A script's set event (`EventCallbackScripts`): added (its variables kept) or removed
/// (released); whether the set changed.
fn insert(e: &mut Engine, set: fn(&mut State) -> &mut BTreeSet<u32>, script: u32) -> bool {
    let added = set(&mut state(e)).insert(script);
    if added {
        crate::ln_events::capture(e, script);
    }
    added
}

fn erase(e: &mut Engine, set: fn(&mut State) -> &mut BTreeSet<u32>, script: u32) -> bool {
    let removed = set(&mut state(e)).remove(&script);
    if removed {
        crate::ln_events::release(e, script);
    }
    removed
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetOnFastTravelEventHandler_Execute, Cmd_SetOnPCTargetChangeEventHandler_Execute,
// Cmd_SetOnNoteAddedEventHandler_Execute.
/// Adds or removes a script of a list event; each script is one user of the event's hook.
fn set_on_list_event(
    e: &mut Engine,
    w: &[u32],
    hook: Hook,
    set: fn(&mut State) -> &mut BTreeSet<u32>,
) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    if read.int(1, 0) != 0 {
        if insert(e, set, script) {
            e.add_hook_user(hook, true);
        }
    } else if erase(e, set, script) {
        e.add_hook_user(hook, false);
    }
    handled(true)
}

fn set_on_fast_travel(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_list_event(e, w, Hook::PCFastTravel, |s| &mut s.fast_travel)
}

fn set_on_pc_target_change(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_list_event(e, w, Hook::SetPCTarget, |s| &mut s.target_change)
}

fn set_on_note_added(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_list_event(e, w, Hook::AddNote, |s| &mut s.note_added)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetOnDialogTopicEventHandler_Execute.
/// Adds or removes a script for a topic or a topic info: a key's first script is one user of the
/// hook; its last gives it back.
fn set_on_dialog_topic(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let form = read.form(2);
    if form == 0 || !matches!(form_type(e, form), TOPIC | TOPIC_INFO) {
        return handled(true);
    }
    if read.int(1, 0) != 0 {
        let (made, added) = {
            let mut s = state(e);
            let made = !s.dialog_topic.contains_key(&form);
            (made, s.dialog_topic.entry(form).or_default().insert(script))
        };
        if made {
            e.add_hook_user(Hook::RunResultScript, true);
        }
        if added {
            crate::ln_events::capture(e, script);
        }
    } else {
        let removed = {
            let mut s = state(e);
            let Some(scripts) = s.dialog_topic.get_mut(&form) else {
                return handled(true);
            };
            if scripts.remove(&script) {
                Some(scripts.is_empty())
            } else {
                None
            }
        };
        if let Some(empty) = removed {
            crate::ln_events::release(e, script);
            if empty {
                state(e).dialog_topic.remove(&form);
                e.add_hook_user(Hook::RunResultScript, false);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_SetOnQuestStageEventHandler_Execute.
/// Adds or removes a script for one of the quest's stages (an existing one: the stage list walked
/// from its first node, empty items passed over), with its skip flag (2 with `skipRes`): a
/// quest's first script sets its flag and is one user; adding one already there does nothing;
/// the quest's last gives both back. The result is 1 when it was added or removed.
fn set_on_quest_stage(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let quest = read.form(2);
    let stage = read.int(3, 0);
    let skip = if read.int(4, 0) != 0 { 2 } else { 1 };
    // TESQuest::stages (JIP LN NVSE, `// 44`): a list of `StageInfo` (the stage byte first).
    let mut found = None;
    let mut node = quest + 0x44;
    while node != 0 {
        let info = e.mem.u32(node);
        if info != 0 && u32::from(e.mem.u8(info)) == stage {
            found = Some(e.mem.u8(info));
            break;
        }
        node = e.mem.u32(node + 4);
    }
    let Some(id) = found else {
        return handled(true);
    };
    let event = QuestStageEvent {
        stage: id,
        skip,
        script,
    };
    let same = |o: &QuestStageEvent| o.stage == event.stage && o.script == event.script;
    if read.int(1, 0) != 0 {
        let made = match state(e).quest_stage.entry(quest) {
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(Vec::new());
                true
            }
            std::collections::btree_map::Entry::Occupied(o) => {
                if o.get().iter().any(same) {
                    return handled(true);
                }
                false
            }
        };
        if made {
            set_jip_flag(e, quest, SET_STAGE_HANDLERS, true);
            e.add_hook_user(Hook::SetQuestStage, true);
        }
        state(e).quest_stage.entry(quest).or_default().push(event);
        crate::ln_events::capture(e, script);
        a.set_result(e, 1.0);
    } else {
        let (removed, empty) = {
            let mut s = state(e);
            let Some(events) = s.quest_stage.get_mut(&quest) else {
                return handled(true);
            };
            let before = events.len();
            events.retain(|o| !same(o));
            (before - events.len(), events.is_empty())
        };
        if removed == 0 {
            return handled(true);
        }
        for _ in 0..removed {
            crate::ln_events::release(e, script);
        }
        if empty {
            state(e).quest_stage.remove(&quest);
            set_jip_flag(e, quest, SET_STAGE_HANDLERS, false);
            e.add_hook_user(Hook::SetQuestStage, false);
        }
        a.set_result(e, 1.0);
    }
    handled(true)
}

/// The caller of a "was it" command: the script's quest (`scriptObj->quest`, `+0x40`), else the
/// calling reference.
fn informed_caller(e: &Engine, a: &CommandArgs) -> u32 {
    let quest = if a.script != 0 {
        e.mem.u32(a.script + 0x40)
    } else {
        0
    };
    if quest != 0 {
        quest
    } else {
        a.this_obj
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_GetPCFastTravelled_Execute, Cmd_GetPCMovedCell_Execute.
/// Whether the player fast travelled (or changed cell) since the caller last asked: the first
/// caller after one is one user of the hook (fast travel; the cell change is switched on); a
/// caller new to the set answers 1 when it was told before (its flag), else it is flagged and
/// remembered by id.
fn informed(
    e: &mut Engine,
    w: &[u32],
    flag: u8,
    set: fn(&mut State) -> &mut BTreeSet<u32>,
    first: fn(&mut Engine),
) -> Ret {
    let a = CommandArgs::new(w);
    let caller = informed_caller(e, &a);
    if caller == 0 {
        return handled(true);
    }
    if set(&mut state(e)).is_empty() {
        first(e);
    }
    if set(&mut state(e)).insert(caller) {
        let flags = flags5(e, caller);
        if flags & flag != 0 {
            a.set_result(e, 1.0);
        } else {
            let id = form_id(e, caller);
            state(e).event_informed.insert(id);
            set_flags5(e, caller, flags | flag);
        }
    }
    handled(true)
}

fn get_pc_fast_travelled(e: &mut Engine, w: &[u32]) -> Ret {
    informed(
        e,
        w,
        FAST_TRAVEL_INFORMED,
        |s| &mut s.fast_travel_informed,
        |e| {
            e.add_hook_user(Hook::PCFastTravel, true);
        },
    )
}

fn get_pc_moved_cell(e: &mut Engine, w: &[u32]) -> Ret {
    informed(
        e,
        w,
        CELL_CHANGE_INFORMED,
        |s| &mut s.cell_change_informed,
        |e| e.set_hook(Hook::PCCellChange, true),
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ToggleIgnoreLockedDoors_Execute.
/// Teammates pass locked doors that lead nowhere (on) or not.
fn toggle_ignore_locked_doors(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        e.set_hook(Hook::ActivateDoor, read.int(0, 0) != 0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_SetWeatherTransitionTimeOverride_Execute.
/// A transition time (not 0) sets the rate to `120 / time` (a float division of the signed
/// time) and switches the hook on; 0 switches it off and keeps the rate.
fn set_weather_transition_time_override(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let time = read.int(0, 0);
        if time != 0 {
            state(e).weather_rate = 120.0f32 / time as i32 as f32;
        }
        e.set_hook(Hook::UpdateWeather, time != 0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetInteractionDisabled_Execute.
/// The form (or the calling reference) the player may not activate (on) or may: a change sets
/// or clears its flag and is one user more or less.
fn set_interaction_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(1);
    if form == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        form = a.this_obj;
    }
    let add = read.int(0, 0) != 0;
    if add == (flags6(e, form) & NO_PC_ACTIVATION == 0) {
        set_jip_flag(e, form, NO_PC_ACTIVATION, add);
        e.add_hook_user(Hook::InteractionDisabled, add);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetInteractionDisabledType_Execute.
/// A form type (one of the filter types, `kValidFilterForm`) the player may not activate (on)
/// or may: a change is one user more or less.
fn set_interaction_disabled_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let kind = read.int(1, 0) as usize;
    if !crate::ln_events::VALID_FILTER_FORM
        .get(kind)
        .copied()
        .unwrap_or(false)
    {
        return handled(true);
    }
    let add = read.int(0, 0) != 0;
    let Some(slot) = e
        .ext::<Jip>()
        .ui_hooks
        .activation_disabled_types
        .get_mut(kind)
    else {
        return handled(true);
    };
    if add != *slot {
        *slot = add;
        e.add_hook_user(Hook::InteractionDisabled, add);
    }
    handled(true)
}

/// `TESForm::HasScript` (JIP LN NVSE): the form's `TESScriptableForm` (`kBaseComponentOffsets`
/// byte 1) holds a script (`+4`).
pub(crate) fn has_script(e: &Engine, form: u32) -> bool {
    let Some(&entry) =
        crate::commands::forms::BASE_COMPONENT_OFFSETS.get(usize::from(form_type(e, form)))
    else {
        return false;
    };
    let words = (entry >> 8) as u8 as i8;
    words != 0
        && e.mem
            .u32(form.wrapping_add((i32::from(words) * 4) as u32) + 4)
            != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// SetScriptEventDisabled.
/// Events disabled for a form: with a mask, disabling adds it (the form's first is one user of
/// MarkScriptEvent and sets its flag), enabling clears it (all cleared: the entry, the flag and
/// the user go); "OnActivate" sets or clears the activation flag, one DoActivate user more or
/// less, when it changes.
pub(crate) fn set_events_disabled(
    e: &mut Engine,
    form: u32,
    mask: u32,
    on_activate: bool,
    disable: bool,
) {
    if mask != 0 {
        if disable {
            let made = {
                let mut s = state(e);
                let made = !s.disabled_events.contains_key(&form);
                if made {
                    s.disabled_events.insert(form, 0);
                }
                made
            };
            if made {
                set_jip_flag(e, form, EVENT_DISABLED, true);
                e.add_hook_user(Hook::MarkScriptEvent, true);
            }
            *state(e).disabled_events.entry(form).or_insert(0) |= mask;
        } else if flags6(e, form) & EVENT_DISABLED != 0 {
            let gone = {
                let mut s = state(e);
                match s.disabled_events.get_mut(&form) {
                    Some(events) => {
                        *events &= !mask;
                        let gone = *events == 0;
                        if gone {
                            s.disabled_events.remove(&form);
                        }
                        gone
                    }
                    None => false,
                }
            };
            if gone {
                set_jip_flag(e, form, EVENT_DISABLED, false);
                e.add_hook_user(Hook::MarkScriptEvent, false);
            }
        }
    }
    if on_activate && disable == (flags6(e, form) & ACTIVATE_DISABLED == 0) {
        set_jip_flag(e, form, ACTIVATE_DISABLED, disable);
        e.add_hook_user(Hook::DoActivate, disable);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_SetScriptEventDisabled_Execute; internal/utility.cpp: GetNextToken.
/// The events named in the text (space separated, JIP's 0x40-byte buffer) disabled or enabled
/// for the form, or each form of a list: quests are left out, a reference only with a scripted
/// base that is not an inventory item, another form only with a script.
fn set_script_event_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let text = crate::commands::buffer_arg(e, &read, 1, 0x40, "SetScriptEventDisabled");
    let disable = read.int(2, 0) != 0;
    let mut mask = 0u32;
    let mut on_activate = false;
    let mut at = 0usize;
    loop {
        let end = text[at..]
            .iter()
            .position(|&b| b == b' ')
            .map_or(text.len(), |p| at + p);
        let token = &text[at..end];
        at = end;
        while at < text.len() && text[at] == b' ' {
            at += 1;
        }
        if let Some(m) = event_mask(token) {
            mask |= m;
            if m == 0 {
                on_activate = true;
            }
        }
        if at >= text.len() {
            break;
        }
    }
    if mask == 0 && !on_activate {
        return handled(true);
    }
    for form in crate::actor_hooks::form_or_list(e, read.form(0)) {
        if form == 0 || form_type(e, form) == QUEST {
            continue;
        }
        if is_reference(e, form) {
            let base = base_of(e, form);
            if crate::commands::forms::INVENTORY_TYPE
                .get(usize::from(form_type(e, base)))
                .copied()
                .unwrap_or(false)
                || !has_script(e, base)
            {
                continue;
            }
        } else if !has_script(e, form) {
            continue;
        }
        set_events_disabled(e, form, mask, on_activate, disable);
    }
    handled(true)
}

/// `s_eventMasks->GetPtr(name)`: the mask of the first name with the same `StrHashCI`.
pub(crate) fn event_mask(name: &[u8]) -> Option<u32> {
    let hash = crate::containers::str_hash_ci(name);
    EVENT_MASKS
        .iter()
        .find(|(n, _)| crate::containers::str_hash_ci(n.as_bytes()) == hash)
        .map(|(_, m)| *m)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetRefName_Execute, Cmd_SetRefrModelPath_Execute.
/// The calling reference's own name or model path (JIP's 0x80-byte buffer) set, or removed by
/// an empty text: a reference's first is one user of the hook; the reference's flag follows.
/// The text goes into the reference's block ([`TextBlock`]).
fn set_ref_string(
    e: &mut Engine,
    w: &[u32],
    command: &str,
    hook: Hook,
    flag: u8,
    map: fn(&mut State) -> &mut BTreeMap<u32, TextBlock>,
) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let reference = a.this_obj;
    let text = crate::commands::buffer_arg(e, &read, 0, 0x80, command);
    if !text.is_empty() {
        let old = map(&mut state(e)).get(&reference).copied();
        let block = text_block(e, old, text.len() as u32 + 1);
        e.mem.write(block.at, &text);
        e.mem.set_u8(block.at + text.len() as u32, 0);
        if map(&mut state(e)).insert(reference, block).is_none() {
            e.add_hook_user(hook, true);
        }
        let flags = ref_flags(e, reference);
        set_ref_flags(e, reference, flags | flag);
    } else {
        let found = {
            let mut s = state(e);
            let block = map(&mut s).remove(&reference);
            if let Some(block) = block {
                s.free_texts.push(block);
            }
            block.is_some()
        };
        if found {
            e.add_hook_user(hook, false);
        }
        let flags = ref_flags(e, reference);
        set_ref_flags(e, reference, flags & !flag);
    }
    handled(true)
}

/// A block for a text of `size` bytes (its NUL counted): the reference's own when it fits, else
/// the smallest free one that does (the reference's own then freed), else a new one from the
/// game's heap (JIP's `Game_HeapAlloc`).
fn text_block(e: &mut Engine, old: Option<TextBlock>, size: u32) -> TextBlock {
    if let Some(old) = old.filter(|b| b.capacity >= size) {
        return old;
    }
    let reuse = {
        let mut s = state(e);
        let i = s
            .free_texts
            .iter()
            .enumerate()
            .filter(|(_, b)| b.capacity >= size)
            .min_by_key(|(_, b)| b.capacity)
            .map(|(i, _)| i);
        i.map(|i| s.free_texts.swap_remove(i))
    };
    let block = match reuse {
        Some(b) => b,
        None => {
            let capacity = size.next_power_of_two().max(0x80);
            TextBlock {
                at: crate::commands::heap_alloc(e, capacity),
                capacity,
            }
        }
    };
    if let Some(old) = old {
        state(e).free_texts.push(old);
    }
    block
}

fn set_ref_name(e: &mut Engine, w: &[u32]) -> Ret {
    set_ref_string(e, w, "SetRefName", Hook::GetRefName, ALT_REF_NAME, |s| {
        &mut s.ref_names
    })
}

fn set_refr_model_path(e: &mut Engine, w: &[u32]) -> Ret {
    set_ref_string(
        e,
        w,
        "SetRefrModelPath",
        Hook::GetModelPath,
        REFR_MODEL_PATH,
        |s| &mut s.model_paths,
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SwapObjectLOD_Execute.
/// A world space's cell (its 16-bit x and y) gets the `AltLOD` blocks; a new one switches the
/// hook on.
fn swap_object_lod(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let world_space = read.form(0);
    if !read.ok || world_space == 0 || form_type(e, world_space) != WORLD_SPACE {
        return handled(true);
    }
    let (x, y) = (read.int(1, 0), read.int(2, 0));
    let key = (x & 0xffff) << 16 | (y & 0xffff);
    if state(e)
        .swap_obj_lod
        .entry(world_space)
        .or_default()
        .insert(key)
    {
        e.set_hook(Hook::MakeObjLODPath, true);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_ToggleItemUnique_Execute.
/// Weapons and armour (by form type; a list's items, others passed over) made unique for the
/// repair menus or not: each change sets or clears the item's flag and is one user more or less.
/// A form that is neither, nor a list, does nothing.
fn toggle_item_unique(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if !read.ok || form == 0 {
        return handled(true);
    }
    if !matches!(form_type(e, form), LIST_FORM | WEAPON | ARMOR) {
        return handled(true);
    }
    let on = read.int(1, 0) != 0;
    for item in crate::actor_hooks::form_or_list(e, form) {
        if item == 0 || !matches!(form_type(e, item), WEAPON | ARMOR) {
            continue;
        }
        if on == (flags6(e, item) & UNIQUE_ITEM != 0) {
            continue;
        }
        set_jip_flag(e, item, UNIQUE_ITEM, on);
        e.add_hook_user(Hook::CheckUniqueItem, on);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetFormDescription_Execute, internal/hooks.h: SetDescriptionAltText.
/// The form's description (its `TESDescription`, or a note's text) gets its own text (the
/// formatted string, JIP's string buffer): a new one is one user of the hook; an empty text
/// removes it (with its user). A change empties the shared text's owner, so the next read loads
/// again. The result is 1 for a form with a description.
fn set_form_description(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 1, 22);
    if !f.ok {
        return handled(true);
    }
    let text =
        crate::commands::formatted_in_buffer(e, &f.text, STR_BUFFER_SIZE, "SetFormDescription");
    let form = f
        .leading
        .first()
        .and_then(nvse::args::Arg::form)
        .unwrap_or(0);
    if form == 0 {
        return handled(true);
    }
    let mut description = nvse::commands::dynamic_cast(
        e,
        form,
        nvse::commands::rtti::TES_FORM,
        RTTI_TES_DESCRIPTION,
    );
    if description == 0 && form_type(e, form) == NOTE {
        // BGSNote::noteText (JIP LN NVSE, `// 6C`).
        description = e.mem.u32(form + 0x6c);
    }
    if description == 0 {
        return handled(true);
    }
    set_description_alt_text(e, description, &text);
    a.set_result(e, 1.0);
    handled(true)
}

/// `RTTI_TESDescription` (xNVSE's `GameRTTI_1_4_0_525.inc`; the type descriptor's name in the
/// exe is `.?AVTESDescription@@`).
const RTTI_TES_DESCRIPTION: u32 = 0x0118_38f8;
/// JIP's string argument buffer (`GetStrArgBuffer`, 0x40000 bytes).
const STR_BUFFER_SIZE: usize = 0x4_0000;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetDescriptionAltText.
fn set_description_alt_text(e: &mut Engine, description: u32, text: &[u8]) {
    if !text.is_empty() {
        if state(e)
            .description_changes
            .insert(description, text.to_vec())
            .is_none()
        {
            e.add_hook_user(Hook::GetDescription, true);
        }
    } else if state(e).description_changes.remove(&description).is_some() {
        e.add_hook_user(Hook::GetDescription, false);
    } else {
        return;
    }
    e.mem.set_u32(DESCRIPTION_OWNER, 0);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_ScriptWait_Execute; internal/jip_core.h: ScriptBlockIterator, ScriptWaitInfo::Init.
/// Only from `EvaluateLine`'s command call (JIP's return-address test `0x5E234B`), in a script
/// with an id that is not an effect script, for a count above 0: the script's owner (its quest,
/// or the calling reference) waits that many passes of the block it is in (not an `OnLoad`-type
/// block 0xD). The command's offset moves past the command, the wait remembers the line and the
/// runner's `if` stack, the owner is flagged, the hook switched on, and the run jumps to the
/// block's end (`Run`'s length word).
fn script_wait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some(line) = nvse::context::current_line(e) else {
        return handled(true);
    };
    if line.call != nvse::context::HandlerCall::Command {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let iterations = read.int(0, 0);
    let script = a.script;
    if !read.ok || form_id(e, script) == 0 || e.mem.u8(script + 0x29) != 0 || iterations == 0 {
        return handled(true);
    }
    let owner = if e.mem.u8(script + 0x28) != 0 {
        e.mem.u32(script + 0x40)
    } else {
        a.this_obj
    };
    if owner == 0 {
        return handled(true);
    }
    let data = a.data;
    let offset = e.mem.u32(a.opcode_offset);
    let offset = offset.wrapping_add(u32::from(
        e.mem.u16(data.wrapping_add(offset).wrapping_sub(2)),
    ));
    e.mem.set_u32(a.opcode_offset, offset);
    let blocks = block_iterator(e, data, offset);
    if blocks.type_at == 0 || e.mem.u8(blocks.type_at) == 0xd {
        return handled(true);
    }
    let runner = line.runner;
    let mut saved_if_stack = [0u32; 12];
    for (i, word) in saved_if_stack.iter_mut().enumerate() {
        *word = e.mem.u32(runner + 0x20 + 4 * i as u32);
    }
    let wait = ScriptWait {
        owner,
        ref_id: form_id(e, owner),
        iterations,
        block_offset: blocks.next_op,
        saved_offset: offset.wrapping_sub(blocks.next_op),
        saved_if_stack,
    };
    state(e).script_waits.insert(owner, wait);
    let flags = flags5(e, owner);
    set_flags5(e, owner, flags | SCRIPT_ON_WAIT);
    e.set_hook(Hook::ScriptWait, true);
    e.mem.set_u32(
        line.length,
        blocks
            .next_block
            .wrapping_sub(4)
            .wrapping_sub(line.position),
    );
    handled(true)
}

/// What `ScriptBlockIterator` holds once walked to its end: the last block's type byte's address
/// (0 for none), the offset after its `Begin` line, the offset past its `End`.
struct Blocks {
    type_at: u32,
    next_op: u32,
    next_block: u32,
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.h: ScriptBlockIterator.
/// Walks the compiled data's blocks (`scn` first, then `Begin` lines of opcode 0x10 each ending
/// at an `End`, 0x11) while the offset is below `length`; a data that does not start so, or a
/// block that does not end with `End`, stops with no type.
fn block_iterator(e: &Engine, data: u32, length: u32) -> Blocks {
    let mut b = Blocks {
        type_at: 0,
        next_op: 0,
        next_block: 4,
    };
    // Wrapping adds: the offsets come from a mod's compiled data (JIP's 32-bit sums wrap).
    let eval = |b: &mut Blocks| -> bool {
        if e.mem.u16(data.wrapping_add(b.next_block)) != 0x10 {
            return false;
        }
        b.next_block = b.next_block.wrapping_add(2);
        let size = u32::from(e.mem.u16(data.wrapping_add(b.next_block)));
        b.next_block = b.next_block.wrapping_add(2);
        b.type_at = data.wrapping_add(b.next_block);
        b.next_op = b.next_block.wrapping_add(size);
        b.next_block = b.next_block.wrapping_add(2);
        b.next_block = b
            .next_block
            .wrapping_add(u32::from(e.mem.u16(data.wrapping_add(b.next_block))))
            .wrapping_add(size)
            .wrapping_sub(6);
        if e.mem.u32(data.wrapping_add(b.next_block)) == 0x11 {
            b.next_block = b.next_block.wrapping_add(4);
            return true;
        }
        false
    };
    let mut length = length;
    if data == 0 || e.mem.u32(data) != 0x1d || !eval(&mut b) {
        b.type_at = 0;
        return b;
    }
    while length != 0 {
        if b.next_block < length {
            if eval(&mut b) {
                continue;
            }
            b.type_at = 0;
        }
        length = 0;
    }
    b
}

#[cfg(test)]
#[path = "world_hooks_tests.rs"]
mod tests;
