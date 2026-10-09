//! `fallout/ai/processlists.cpp` (Xbox PDB source unit), subsystem `fallout/ai`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `ProcessLists` is a single static object of the game (`PROCESS_LISTS`,
//! `0x011e0e80`). It keeps the process array of every mobile object
//! (`MobProcessArray`, the `ProcessArray` at +0x04: four process levels, each
//! with a head and a tail index into one array of `MobileObject *`), the
//! lists the per-frame update works through, and the 50 actors closest to
//! the player.
//!
//! Conventions of this file, for the sessions that continue it:
//!
//! - PC offsets differ from the Xbox PDB after +0x15D (the Xbox layout is
//!   0x54..0x90 bytes larger there); the layout below lists PC offsets only,
//!   derived from the constructor (`0096d020`) and the functions using them.
//! - The process array is read through `ProcessArray` accessors that live in
//!   other units: `005de0f0` (head index of a level), `005be5c0` (tail
//!   index of a level) and `00968670` (the object at an index), all taking
//!   the array (`ProcessLists + 4`) as `this`. The helpers `array_head`,
//!   `array_tail` and `array_object` wrap them.
//! - Lists are `BSSimpleList`s: `006815c0` is the address of a node's item,
//!   `00726070` its next node; the helpers `node_item` and `node_next` wrap
//!   them.
//! - The lock at `ProcessLists + 0x10320` (`LOCK`) is a recursive spin lock
//!   (owner thread id, count): `0040fbf0` enters it, `0078d200` tries to
//!   enter it, `0040fba0` leaves it. The lock at +0x10300 guards the array
//!   of actors close to the player.
//! - Floats: x87 code, so the translations compute in `f64` and round to
//!   `f32` at each store; the comparisons follow the flags the compiler
//!   tests (`JP` after `TEST AH, 5` is "not less", and so on).
//! - Vtable slots are named only where their meaning is confirmed by the
//!   code (`IsActor`, `IsMobileObject`, ...); the others are called by their
//!   offset and described in the doc comment of the function using them.
//! - The compiler's exception-unwinding frames are not translated.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::units::fallout_ai::actor::Actor;

/// The game's one `ProcessLists` object.
pub(crate) const PROCESS_LISTS: u32 = 0x011e_0e80;
/// The player character pointer (`PlayerCharacter` singleton).
pub(crate) const PLAYER: u32 = 0x011d_ea3c;
/// The `Calendar` object (`calendar.cpp`).
const CALENDAR: u32 = 0x011d_e7b8;
/// The `TES` object.
const TES: u32 = 0x011d_ea10;
/// The lock at `ProcessLists + 0x10320`.
pub(crate) const LOCK: u32 = 0x011f_11a0;
/// The task manager that runs parallel animation and update tasks.
const TASK_MANAGER: u32 = 0x0120_2df4;

/// `ProcessArray` accessor: head index of a process level (`ProcessLevelHeads[level]`).
const ARRAY_HEAD: u32 = 0x005d_e0f0;
/// `ProcessArray` accessor: tail index of a process level (`ProcessLevelTails[level]`).
const ARRAY_TAIL: u32 = 0x005b_e5c0;
/// `ProcessArray` accessor: the `MobileObject *` at an index.
const ARRAY_OBJECT: u32 = 0x0096_8670;
/// `ProcessArray::AddActor(actor, level)` (Xbox PDB).
const ARRAY_ADD_ACTOR: u32 = 0x0096_a970;
/// `ProcessArray::RemoveActor(actor, level)` (Xbox PDB).
const ARRAY_REMOVE_ACTOR: u32 = 0x0096_aaf0;
/// `ProcessArray` constructor (`this` = the array).
const ARRAY_CONSTRUCT: u32 = 0x0096_a850;
/// `ProcessArray` destructor.
const ARRAY_DESTRUCT: u32 = 0x0096_a950;
/// Returns the `ProcessArray` of the `ProcessLists` it is called on
/// (`this + 4`).
const PROCESS_ARRAY_OF: u32 = 0x0071_7e50;

/// `BSSimpleList::Item`: address of the node's item.
const NODE_ITEM: u32 = 0x0068_15c0;
/// `BSSimpleList::Next`: the next node.
const NODE_NEXT: u32 = 0x0072_6070;
/// `BSSimpleList` constructor (zeroes item and next).
const LIST_CONSTRUCT: u32 = 0x0096_a2d0;
/// `BSSimpleList` push (`this` = list, argument = address of the item).
const LIST_PUSH: u32 = 0x005a_e3d0;
/// `BSSimpleList` contains (`this` = list, argument = address of the item).
const LIST_CONTAINS: u32 = 0x005f_65d0;
/// `BSSimpleList` remove (`this` = list, argument = address of the item).
const LIST_REMOVE: u32 = 0x0090_5330;
/// `BSSimpleList` pop-front (removes the first node, `this` = list).
const LIST_POP_FRONT: u32 = 0x0063_f7b0;
/// `BSSimpleList` is-empty test.
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
/// `BSSimpleList` clear (frees the nodes behind the head).
const LIST_CLEAR: u32 = 0x0047_0470;
/// `BSSimpleList` scalar deleting destructor (`this`, flags).
const LIST_DELETE: u32 = 0x0047_02f0;
/// `BSSimpleList` push of the item `this` points at (`this` = list, argument =
/// address of the item), the variant `fn_0096d520` uses.
const LIST_ADD_ITEM: u32 = 0x0090_5820;
/// `operator new`.
const OPERATOR_NEW: u32 = 0x0040_1000;

/// Recursive spin lock: enter (argument 0).
pub(crate) const LOCK_ENTER: u32 = 0x0040_fbf0;
/// Recursive spin lock: leave.
pub(crate) const LOCK_LEAVE: u32 = 0x0040_fba0;
/// Recursive spin lock: try to enter, true when entered.
pub(crate) const LOCK_TRY_ENTER: u32 = 0x0078_d200;

/// The process (`pCurrentProcess`, +0x68) of an actor.
const ACTOR_PROCESS: u32 = 0x008d_8520;
/// `MobileObject::GetCurrentProcessType` (Xbox PDB): the actor's process level.
const ACTOR_PROCESS_LEVEL: u32 = 0x0093_1850;
/// `MobileObject::GetDesiredProcessLevel` (Xbox PDB).
const ACTOR_DESIRED_LEVEL: u32 = 0x0093_34b0;
/// A process's current level (`0045cd60`, reads +0x28 of the process).
const PROCESS_LEVEL: u32 = 0x0045_cd60;
/// The actor's 3D node accessor: vtable slot 0x1d0 (argument 0).
const SLOT_GET_NODE: u32 = 0x1d0;
/// `TESForm::~TESForm` scalar deleting destructor slot (`this`, flags).
const SLOT_DELETING_DESTRUCTOR: u32 = 0x10;
/// `TESForm::SetNeedToChangeProcess` slot (Xbox PDB).
const SLOT_SET_NEED_TO_CHANGE_PROCESS: u32 = 0xd8;
/// `TESForm::IsMobileObject` slot (Xbox PDB).
const SLOT_IS_MOBILE_OBJECT: u32 = 0xfc;
/// `TESForm::IsActor` slot (Xbox PDB).
const SLOT_IS_ACTOR: u32 = 0x100;
/// Process slot 0x22c, the package the process is running (Xbox PDB
/// `GetCurrentPackage`); on an actor, with an argument, it is a bool test.
const SLOT_0X22C: u32 = 0x22c;

/// Type of a package (`0041ca90`).
const PACKAGE_TYPE: u32 = 0x0041_ca90;
/// `ExtraDataList` of a reference (`005d43c0`).
const EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// `ExtraDataList::GetFollowerList`-like (`00422700`): the follower record of
/// an extra data list, whose +0xC is the list of followers.
const EXTRA_FOLLOWERS: u32 = 0x0042_2700;
/// `ExtraDataList::RemoveFollower` (Xbox PDB).
const EXTRA_REMOVE_FOLLOWER: u32 = 0x0042_2690;
/// `memmove_s`-style (dest, dest size, source, count).
const MEMMOVE: u32 = 0x0086_61e0;
/// `_ftol2_sse`: the float in ST0 truncated to an integer (the uniform
/// form passes it as a leading `f64`).
const FTOL: u32 = 0x00ec_62c0;

/// `Setting` value getters: `00408d60(setting)` returns the address of the
/// value (a byte), `0043d4d0(setting)` the address of an integer value,
/// `00403e20(setting)` the address of a float value.
const SETTING_VALUE: u32 = 0x0040_8d60;
const SETTING_INT_VALUE: u32 = 0x0043_d4d0;
const SETTING_FLOAT_VALUE: u32 = 0x0040_3e20;
/// The integer setting that caps the number of actors close to the player.
const MAX_ACTORS_SETTING: u32 = 0x011c_de2c;

layout! {
    /// `ProcessLists` (Xbox PDB), PC offsets. The constructor initializes
    /// everything up to the semaphore at +0x103C0 (0xC bytes).
    pub struct ProcessLists: 0x103cc {
        /// `fSecondsPassedNoProcess` (Xbox PDB).
        0x000 fSecondsPassedNoProcess: f32,
        /// `GlobalCrimeListArray` (Xbox PDB): five list pointers. The
        /// constructor clears only the first four bytes.
        0x044 GlobalCrimeListArray: u32,
        /// `iActorsCloseToPlayer` (Xbox PDB): count of used entries of the
        /// array at +0x88.
        0x150 iActorsCloseToPlayer: i32,
        /// `fPlayerActionCommentTimer` (Xbox PDB).
        0x154 fPlayerActionCommentTimer: f32,
        /// `fPlayerKnockObjectCommentTimer` (Xbox PDB).
        0x158 fPlayerKnockObjectCommentTimer: f32,
        /// `bPlayerInRadiationArea` (Xbox PDB).
        0x15C bPlayerInRadiationArea: bool,
        /// `bPlayerInRadiatedWater` (Xbox PDB).
        0x15D bPlayerInRadiatedWater: bool,
        /// `pPrevmoblist` (Xbox PDB), cleared by the constructor.
        0x10340 pPrevmoblist: u32,
        /// `pMoblist` (Xbox PDB), cleared by the constructor.
        0x10344 pMoblist: u32,
        /// `bRunScheduals` (Xbox PDB).
        0x103A0 bRunScheduals: bool,
        /// `bRunDetection` (Xbox PDB).
        0x103A1 bRunDetection: bool,
        /// `bShowDetectionStats` (Xbox PDB).
        0x103A2 bShowDetectionStats: bool,
        /// `pStatdetect` (Xbox PDB).
        0x103A4 pStatdetect: u32,
        /// `bProcessHigh` (Xbox PDB).
        0x103A8 bProcessHigh: bool,
        /// `bProcessLow` (Xbox PDB).
        0x103A9 bProcessLow: bool,
        /// `bProcessMHigh` (Xbox PDB).
        0x103AA bProcessMHigh: bool,
        /// `bProcessMLow` (Xbox PDB).
        0x103AB bProcessMLow: bool,
        /// `bProcessSche` (Xbox PDB).
        0x103AC bProcessSche: bool,
        /// `bShowSubtitle` (Xbox PDB).
        0x103AD bShowSubtitle: bool,
        /// `bUpdatingLowList` (Xbox PDB).
        0x103AE bUpdatingLowList: bool,
        /// `iNumberHighActors` (Xbox PDB): the tail index of level 0 at the
        /// last update.
        0x103B0 iNumberHighActors: u32,
        /// `fCrimeUpdateTimer` (Xbox PDB).
        0x103B4 fCrimeUpdateTimer: f32,
        /// `iCrimeNumber` (Xbox PDB).
        0x103B8 iCrimeNumber: u32,
        /// `fRemoveExcessDeadTimer` (Xbox PDB).
        0x103BC fRemoveExcessDeadTimer: f32,
    }
}

/// Offset of the `MobProcessArray` inside `ProcessLists`.
const MOB_PROCESS_ARRAY: u32 = 0x04;
/// `GlobalCrimeListArray` (five `BSSimpleList<Crime *> *`).
const GLOBAL_CRIME_LIST_ARRAY: u32 = 0x44;
/// `GlobalTempEffectList`, `MagicEffectList` (`BSSimpleList<NiPointer<BSTempEffect>>`).
const GLOBAL_TEMP_EFFECT_LIST: u32 = 0x58;
const MAGIC_EFFECT_LIST: u32 = 0x60;
/// `ReferenceMuzzleFlashList`, `ProjectilePostProcessList`, `TempShouldMoveList`,
/// `AliveActorList` (`BSSimpleList`s of 8 bytes each).
const REFERENCE_MUZZLE_FLASH_LIST: u32 = 0x68;
const PROJECTILE_POST_PROCESS_LIST: u32 = 0x70;
pub(crate) const TEMP_SHOULD_MOVE_LIST: u32 = 0x78;
pub(crate) const ALIVE_ACTOR_LIST: u32 = 0x80;
/// `pActorsCloseToPlayerA` (Xbox PDB): 50 `Actor *` (the Xbox PDB counts
/// bytes: 200). Kept sorted by distance to the player.
pub(crate) const ACTORS_CLOSE_TO_PLAYER: u32 = 0x88;
/// Capacity of the close-to-player array.
const MAX_ACTORS_CLOSE_TO_PLAYER: i32 = 50;
/// Size in bytes of the close-to-player array.
const ACTORS_CLOSE_TO_PLAYER_BYTES: u32 = 200;
/// `TaskManagerAI` (`AITaskManager`, Xbox PDB).
const TASK_MANAGER_AI: u32 = 0x160;
/// Three more embedded objects of the PC build, zeroed by the constructor
/// (two words each): +0x102E4, +0x10300 (the `ActorsCloseToPlayerLock`) and
/// +0x10320 (`LOCK`).
const UNNAMED_LIST_102E4: u32 = 0x102e4;
pub(crate) const ACTORS_CLOSE_TO_PLAYER_LOCK: u32 = 0x10300;
const LOCK_OFFSET: u32 = 0x10320;
/// `LipBackgroundManager` (`LipSyncBackgroundManager`, Xbox PDB).
const LIP_BACKGROUND_MANAGER: u32 = 0x10360;
/// `MovementSyncSema` (`BSSemaphore`): the last member the constructor sets
/// up (initial count 0, maximum 0x28).
const MOVEMENT_SYNC_SEMAPHORE: u32 = 0x103c0;

/// Per-frame clock the update advances (`GetSystemTimeClock`).
const SYSTEM_TIME_CLOCK: u32 = 0x011e_0e2c;
/// `QueryPerformanceFrequency` result, set by the constructor.
const PERFORMANCE_FREQUENCY: u32 = 0x011e_0e20;
/// The calendar values the update publishes for the AI: hour (`float`),
/// year, month and day.
const GAME_HOUR: u32 = 0x011e_0358;
const GAME_YEAR: u32 = 0x011e_035c;
const GAME_MONTH: u32 = 0x011a_39c4;
const GAME_DAY: u32 = 0x011e_0360;
/// The actor `ChangeProcessLevelTempList` is working on (a global the game
/// keeps between iterations).
pub(crate) const TEMP_CHANGE_ACTOR: u32 = 0x011e_0e38;
/// `QueryPerformanceFrequency` import slot.
const QUERY_PERFORMANCE_FREQUENCY: u32 = 0x00fd_f0a4;
/// Timer object whose `0084d030` returns the frame time (a `float`) and
/// `00825c00` a millisecond counter.
const FRAME_TIMER: u32 = 0x011f_6394;
/// A word the furniture clean-up passes to `0098ddd0`.
const FURNITURE_ARGUMENT: u32 = 0x011f_426c;

// Doubles the code compares against: 0.0, 1.0, 2.0, 100000.0, 24.0 (hours
// per day), 0.05 and 1000.0 (milliseconds per second).
const ZERO: u32 = 0x0101_2060;
const ONE: u32 = 0x0101_2070;
const TWO: u32 = 0x0101_1590;
const CLOCK_LIMIT: u32 = 0x0108_c160;
const HOURS_PER_DAY: u32 = 0x0103_56d8;
const SMALL_TIME: u32 = 0x0108_2cb0;
const MILLISECONDS_PER_SECOND: u32 = 0x0101_7b70;
/// `3600.0f`: the argument of actor vtable slot 0x25c in `fn_0096d520`.
const FOLLOWER_DELAY: u32 = 0x0108_4838;

/// Source-file string the update's profiling scope records.
const SOURCE_FILE: u32 = 0x0108_c11c;

// --- small helpers over the shared accessors --------------------------------

/// The `ProcessArray` of a `ProcessLists`.
fn mob_process_array(this: Ptr<ProcessLists>) -> u32 {
    this.addr().wrapping_add(MOB_PROCESS_ARRAY)
}

fn array_head(e: &mut Engine, array: u32, level: u32) -> u32 {
    e.call(ARRAY_HEAD, &args![array, level]).u32()
}

fn array_tail(e: &mut Engine, array: u32, level: u32) -> u32 {
    e.call(ARRAY_TAIL, &args![array, level]).u32()
}

fn array_object(e: &mut Engine, array: u32, index: u32) -> u32 {
    e.call(ARRAY_OBJECT, &args![array, index]).u32()
}

fn node_item(e: &mut Engine, node: u32) -> u32 {
    e.call(NODE_ITEM, &args![node]).u32()
}

fn node_next(e: &mut Engine, node: u32) -> u32 {
    e.call(NODE_NEXT, &args![node]).u32()
}

/// The value of a node's item (`*BSSimpleList::Item(node)`).
fn node_value(e: &mut Engine, node: u32) -> u32 {
    let item = node_item(e, node);
    e.mem.u32(item)
}

fn is_actor(e: &mut Engine, object: u32) -> bool {
    e.vcall(object, SLOT_IS_ACTOR, &[]).bool()
}

fn actor_process(e: &mut Engine, actor: u32) -> u32 {
    e.call(ACTOR_PROCESS, &args![actor]).u32()
}

fn extra_list(e: &mut Engine, object: u32) -> u32 {
    e.call(EXTRA_DATA_LIST, &args![object]).u32()
}

/// The player pointer the game keeps in a global.
fn player(e: &Engine) -> u32 {
    e.global(PLAYER)
}

fn lock_enter(e: &mut Engine, lock: u32) {
    e.call(LOCK_ENTER, &args![lock, 0u32]);
}

fn lock_leave(e: &mut Engine, lock: u32) {
    e.call(LOCK_LEAVE, &args![lock]);
}

/// Pushes `value` on a list (the list function takes the address of a
/// local holding the value).
fn list_push_value(e: &mut Engine, list: u32, value: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(LIST_PUSH, &args![list, slot.addr()]);
    });
}

/// Removes `value` from a list (same convention as `list_push_value`).
fn list_remove_value(e: &mut Engine, list: u32, value: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(LIST_REMOVE, &args![list, slot.addr()]);
    });
}

/// Scalar-deleting destructor of a `BSSimpleList` allocated with `operator new`.
fn list_delete(e: &mut Engine, list: u32) {
    if list != 0 {
        e.call(LIST_DELETE, &args![list, 1u32]);
    }
}

/// `new BSSimpleList` (8 bytes): `operator new`, then the constructor if the
/// allocation succeeded.
fn list_new(e: &mut Engine) -> u32 {
    let list = e.call(OPERATOR_NEW, &args![8u32]).u32();
    if list == 0 {
        0
    } else {
        e.call(LIST_CONSTRUCT, &args![list]).u32()
    }
}

/// `_ftol2_sse` on a `double`.
fn float_to_int(e: &mut Engine, value: f64) -> i32 {
    e.call(FTOL, &args![value]).i32()
}

/// The frame time (`0084d030`, returned in ST0 as a `float`).
fn frame_time(e: &mut Engine) -> f32 {
    e.call(0x0084_d030, &args![FRAME_TIMER]).f32()
}

// --- 006abe40 ---------------------------------------------------------------

// Translated from 006abe40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSScrapArray<EffectTaskData, 1024>::_Reallocate` (Xbox PDB): allocates room
/// for `count` 12-byte elements from the scrap heap the array points at
/// (+0x10), copies the array's current elements (`iSize`, +0x08, times 12
/// bytes) into it, frees the old buffer and returns the new one.
pub fn bsscrap_array_effect_task_data_reallocate(
    e: &mut Engine,
    this: Ptr,
    old_buffer: u32,
    count: u32,
) -> u32 {
    // `ScrapHeap *` at +0x10 of the array.
    let heap = e.mem.u32(this.addr() + 0x10);
    let alignment: u32 = e.global(0x010a_2720);
    // ScrapHeap::Allocate(size, alignment) (Xbox PDB)
    let new_buffer = e
        .call(0x00aa_54a0, &args![heap, count.wrapping_mul(12), alignment])
        .u32();
    let size = e.call(0x0044_ddc0, &args![this]).u32();
    e.call(
        0x0040_1460,
        &args![new_buffer, old_buffer, size.wrapping_mul(12)],
    );
    // ScrapHeap::Deallocate(buffer) (Xbox PDB)
    e.call(0x00aa_5610, &args![heap, old_buffer]);
    new_buffer
}

// Translated from 008d0600 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::PrintLists` (Xbox PDB): does nothing (it takes two stack
/// words it never reads).
pub fn processlists_print_lists(
    _e: &mut Engine,
    _this: Ptr<ProcessLists>,
    _unused_0: u32,
    _unused_1: u32,
) {
}

// --- animation scenegraph updates (0096c860 .. 0096cfa0) --------------------

// Translated from 0096c860 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::UpdateAnimationScenegraph` (Xbox PDB): for every actor of
/// the first two process levels (level 0 head to level 1 tail) that is an
/// actor and has an animation (vtable slot 0x1e4), updates the animation's
/// queued scenegraph (`Animation::UpdateQueuedScenegraph`, Xbox PDB); if the
/// actor wants its lighting updated (`bUpdateLighting`) and the update
/// reported a change, updates the object lighting of its 3D node.
pub fn processlists_update_animation_scenegraph(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let mut index = array_head(e, array, 0);
    while index < array_tail(e, array, 1) {
        let actor = array_object(e, array, index);
        if actor != 0 && is_actor(e, actor) {
            let animation = e.vcall(actor, 0x1e4, &[]).u32();
            let mut updated = false;
            if animation != 0 {
                updated = e.call(0x0049_3930, &args![animation]).bool();
            }
            if fn_0096c950(e, Ptr::new(actor)) && updated {
                update_object_lighting(e, actor);
            }
        }
        index = index.wrapping_add(1);
    }
}

/// The lighting update both animation loops end with: the shadow scene node
/// (`00450b80(0)`) updates the lighting of the actor's 3D node
/// (`ShadowSceneNode::UpdateObjectLighting`, Xbox PDB), then the actor is told
/// (`00496960`).
fn update_object_lighting(e: &mut Engine, actor: u32) {
    let node = e.vcall(actor, SLOT_GET_NODE, &args![0u32]).u32();
    let shadow_scene = e.call(0x0045_0b80, &args![0u32]).u32();
    e.call(0x00b5_d9f0, &args![shadow_scene, node]);
    e.call(0x0049_6960, &args![actor, 0u32]);
}

// Translated from 0096c950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::bUpdateLighting` (+0x14E) getter.
pub fn fn_0096c950(e: &mut Engine, this: Ptr<Actor>) -> bool {
    e.get(this, Actor::bUpdateLighting)
}

// Translated from 0096c970 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::ParallelUpdateAnimationScenegraph` (Xbox PDB): like
/// `UpdateAnimationScenegraph`, but when the setting at `0x011f1260` is on
/// the animations are collected in a scrap array and updated by a
/// parallel task (`fn_0096cb20` per element) instead of one by one.
pub fn processlists_parallel_update_animation_scenegraph(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let first = array_head(e, array, 0);
    let last = array_tail(e, array, 1);
    let count = last.wrapping_sub(first) as i32;
    let setting = e.call(SETTING_VALUE, &args![0x011f_1260u32]).u32();
    let parallel = e.mem.u8(setting) != 0;
    if count == 0 {
        return;
    }
    // The scrap array (0x10 bytes) at +0, the local holding the animation
    // pointer at +0x10, the task (0x10 bytes) at +0x20.
    e.with_stack(0x30, |e, frame| {
        let scrap = frame.addr();
        let slot = scrap + 0x10;
        let task = scrap + 0x20;
        e.call(0x0097_89e0, &args![scrap]);
        let mut index = first;
        while index < last {
            let actor = array_object(e, array, index);
            if actor != 0 && is_actor(e, actor) {
                let animation = e.vcall(actor, 0x1e4, &[]).u32();
                if animation != 0 {
                    let mut updated = true;
                    if parallel {
                        e.mem.set_u32(slot, animation);
                        // BSSimpleArray<..>::AddUninitialized (Xbox PDB name of the push)
                        e.call(0x007c_b2e0, &args![scrap, slot]);
                    } else {
                        updated = e.call(0x0049_3930, &args![animation]).bool();
                    }
                    if fn_0096c950(e, Ptr::new(actor)) && updated {
                        update_object_lighting(e, actor);
                    }
                }
            }
            index = index.wrapping_add(1);
        }
        run_scrap_task(e, scrap, task, 0x0096_cb20);
        e.call(0x0097_8a80, &args![scrap]);
    });
}

/// Submits the scrap array to the parallel task manager when it holds
/// elements: `0087cf10` builds `task` from the callback, the array and its
/// size, `00c44c90` hands it to the task manager.
fn run_scrap_task(e: &mut Engine, scrap: u32, task: u32, callback: u32) {
    if e.call(0x0044_ddc0, &args![scrap]).u32() != 0 {
        let queued = e.call(0x0044_ddc0, &args![scrap]).u32();
        e.call(0x0087_cf10, &args![task, callback, scrap, queued]);
        let manager: u32 = e.global(TASK_MANAGER);
        e.call(0x00c4_4c90, &args![manager, task]);
    }
}

// Translated from 0096cb20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Task body of the parallel animation update: updates the queued scenegraph
/// of the animation stored at `index` of the scrap array.
pub fn fn_0096cb20(e: &mut Engine, scrap: u32, index: u32) {
    let element = e.call(0x006a_7ad0, &args![scrap, index]).u32();
    let animation = e.mem.u32(element);
    e.call(0x0049_3930, &args![animation]);
}

// Translated from 0096cb50 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every actor of process level 0 (head to tail) that is an actor: if
/// it has a ragdoll controller (`+0xAC`) the ragdoll test `00552490` passes
/// on, and a 3D node, and the node test `00552470` fails and it is not in
/// dialogue with the player (`00933840`), calls `00888970(actor, 1)`;
/// otherwise, when the ragdoll test passed, sets the ragdoll controller's
/// flag byte (+0x50).
pub fn fn_0096cb50(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let mut index = array_head(e, array, 0);
    while index < array_tail(e, array, 0) {
        let actor = array_object(e, array, index);
        if actor != 0 && is_actor(e, actor) {
            let ragdoll = e.get(Ptr::<Actor>::new(actor), Actor::pRagdollController);
            let mut ragdoll_ok = false;
            if !ragdoll.is_null() {
                ragdoll_ok = e.call(0x0055_2490, &args![ragdoll]).bool();
            }
            let node = e.vcall(actor, SLOT_GET_NODE, &[]).u32();
            let mut handled = false;
            if node != 0 && ragdoll_ok {
                let node = e.vcall(actor, SLOT_GET_NODE, &[]).u32();
                if !e.call(0x0055_2470, &args![node]).bool()
                    && !e.call(0x0093_3840, &args![actor]).bool()
                {
                    e.call(0x0088_8970, &args![actor, 1u32]);
                    handled = true;
                }
            }
            if !handled && ragdoll_ok {
                fn_0096cc80(e, ragdoll, 1);
            }
        }
        index = index.wrapping_add(1);
    }
}

// Translated from 0096cc80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a byte at +0x50 of the ragdoll controller.
pub fn fn_0096cc80(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x50, value);
}

/// The test the two "process me" loops share: the actor has a 3D node, is
/// neither of the two model-loader tests (`00440d80`, `00440da0`), has a
/// cell (`008d6f30`) that passes `00450ff0`.
fn actor_is_processable(e: &mut Engine, actor: u32) -> bool {
    let cell = e.call(0x008d_6f30, &args![actor]).u32();
    let node = e.vcall(actor, SLOT_GET_NODE, &[]).u32();
    node != 0
        && !e.call(0x0044_0d80, &args![actor]).bool()
        && !e.call(0x0044_0da0, &args![actor]).bool()
        && cell != 0
        && e.call(0x0045_0ff0, &args![cell]).bool()
}

// Translated from 0096cca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every actor of process level 0 that is an actor, is processable
/// (`actor_is_processable`) and has `bProcessMe` (+0xBC) set, calls its
/// vtable slot 0x268 with `0.0`.
pub fn fn_0096cca0(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let mut index = array_head(e, array, 0);
    while index < array_tail(e, array, 0) {
        let actor = array_object(e, array, index);
        if actor != 0
            && is_actor(e, actor)
            && actor_is_processable(e, actor)
            && e.get(Ptr::<Actor>::new(actor), Actor::bProcessMe)
        {
            e.vcall(actor, 0x268, &args![0.0f32]);
        }
        index = index.wrapping_add(1);
    }
}

// Translated from 0096cda0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like `fn_0096cca0`, but calls vtable slot 0x350 (no arguments) on each
/// such actor, or, when the setting at `0x011f126c` is on, collects them in
/// a scrap array and runs `fn_0096cf60` over it as a parallel task.
pub fn fn_0096cda0(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let first = array_head(e, array, 0);
    let last = array_tail(e, array, 0);
    let count = last.wrapping_sub(first) as i32;
    let setting = e.call(SETTING_VALUE, &args![0x011f_126cu32]).u32();
    let parallel = e.mem.u8(setting) != 0;
    if count == 0 {
        return;
    }
    // Scrap array at +0, the local holding the actor at +0x10, task at +0x20.
    e.with_stack(0x30, |e, frame| {
        let scrap = frame.addr();
        let slot = scrap + 0x10;
        let task = scrap + 0x20;
        e.call(0x0097_8ae0, &args![scrap]);
        let mut index = first;
        while index < last {
            let actor = array_object(e, array, index);
            if actor != 0
                && is_actor(e, actor)
                && actor_is_processable(e, actor)
                && e.get(Ptr::<Actor>::new(actor), Actor::bProcessMe)
            {
                if parallel {
                    e.mem.set_u32(slot, actor);
                    e.call(0x007c_b2e0, &args![scrap, slot]);
                } else {
                    e.vcall(actor, 0x350, &[]);
                }
            }
            index = index.wrapping_add(1);
        }
        run_scrap_task(e, scrap, task, 0x0096_cf60);
        e.call(0x0097_8b60, &args![scrap]);
    });
}

// Translated from 0096cf60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Task body of the parallel update in `fn_0096cda0`: calls vtable slot
/// 0x350 of the actor stored at `index` of the scrap array.
pub fn fn_0096cf60(e: &mut Engine, scrap: u32, index: u32) {
    let element = e.call(0x006a_7ad0, &args![scrap, index]).u32();
    let actor = e.mem.u32(element);
    e.vcall(actor, 0x350, &[]);
}

// Translated from 0096cfa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every actor from index 0 to the tail of level 0 that is an actor,
/// calls `HighProcess::SkipFadeIn(process, actor)` (Xbox PDB name of
/// `008ff030`) with its process.
pub fn fn_0096cfa0(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let actor = array_object(e, array, index);
        if actor != 0 && is_actor(e, actor) {
            let process = actor_process(e, actor);
            e.call(0x008f_f030, &args![process, actor]);
        }
        index = index.wrapping_add(1);
    }
}

// --- constructor and destructor ---------------------------------------------

// Translated from 0096d020 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists` constructor: constructs the process array, the embedded
/// lists, the task manager, the lip-sync background manager and the
/// movement semaphore (initial count 0, maximum 0x28), turns the update
/// switches on, clears the timers and the close-to-player array (200 bytes),
/// reads the starting value of `fRemoveExcessDeadTimer` from the setting at
/// `0x011d1530`, enters and leaves the close-to-player lock once and stores
/// the performance counter frequency. Returns `this`.
pub fn fn_0096d020(e: &mut Engine, this: Ptr<ProcessLists>) -> Ptr<ProcessLists> {
    let base = this.addr();
    e.call(ARRAY_CONSTRUCT, &args![base + MOB_PROCESS_ARRAY]);
    e.call(0x004e_e810, &args![base + GLOBAL_TEMP_EFFECT_LIST]);
    e.call(0x004e_e810, &args![base + MAGIC_EFFECT_LIST]);
    e.call(LIST_CONSTRUCT, &args![base + REFERENCE_MUZZLE_FLASH_LIST]);
    e.call(LIST_CONSTRUCT, &args![base + PROJECTILE_POST_PROCESS_LIST]);
    e.call(LIST_CONSTRUCT, &args![base + TEMP_SHOULD_MOVE_LIST]);
    e.call(LIST_CONSTRUCT, &args![base + ALIVE_ACTOR_LIST]);
    e.call(0x008c_9330, &args![base + TASK_MANAGER_AI]);
    e.call(LIST_CONSTRUCT, &args![base + UNNAMED_LIST_102E4]);
    e.call(LIST_CONSTRUCT, &args![base + ACTORS_CLOSE_TO_PLAYER_LOCK]);
    e.call(LIST_CONSTRUCT, &args![base + LOCK_OFFSET]);
    e.call(0x0090_5e30, &args![base + LIP_BACKGROUND_MANAGER]);
    // BSSemaphore::BSSemaphore(initial count, maximum count)
    e.call(
        0x0086_c6a0,
        &args![base + MOVEMENT_SYNC_SEMAPHORE, 0u32, 0x28u32],
    );
    e.set(this, ProcessLists::bRunDetection, true);
    e.set(this, ProcessLists::bRunScheduals, true);
    e.set(this, ProcessLists::bProcessHigh, true);
    e.set(this, ProcessLists::bProcessLow, true);
    e.set(this, ProcessLists::bProcessMHigh, true);
    e.set(this, ProcessLists::bProcessMLow, true);
    e.set(this, ProcessLists::bProcessSche, true);
    e.set(this, ProcessLists::bShowDetectionStats, false);
    e.set(this, ProcessLists::bPlayerInRadiationArea, false);
    e.set(this, ProcessLists::bPlayerInRadiatedWater, false);
    e.set(this, ProcessLists::pStatdetect, 0);
    e.set(this, ProcessLists::pPrevmoblist, 0);
    e.set(this, ProcessLists::pMoblist, 0);
    // memset(&GlobalCrimeListArray, 0, 4): only the first of the five slots.
    e.call(
        0x0040_3d30,
        &args![base + GLOBAL_CRIME_LIST_ARRAY, 0u32, 4u32],
    );
    e.set(this, ProcessLists::bShowSubtitle, false);
    e.set(this, ProcessLists::bUpdatingLowList, false);
    let remove_excess_dead = e.call(SETTING_FLOAT_VALUE, &args![0x011d_1530u32]).u32();
    let timer = e.mem.f32(remove_excess_dead);
    e.set(this, ProcessLists::fRemoveExcessDeadTimer, timer);
    e.set(this, ProcessLists::iNumberHighActors, 0);
    e.set(this, ProcessLists::fPlayerActionCommentTimer, 0.0);
    e.set(this, ProcessLists::fPlayerKnockObjectCommentTimer, 0.0);
    e.set(this, ProcessLists::iCrimeNumber, 0);
    e.set(this, ProcessLists::fCrimeUpdateTimer, 0.0);
    lock_enter(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
    e.call(
        0x0040_3d30,
        &args![
            base + ACTORS_CLOSE_TO_PLAYER,
            0u32,
            ACTORS_CLOSE_TO_PLAYER_BYTES
        ],
    );
    e.set(this, ProcessLists::iActorsCloseToPlayer, 0);
    lock_leave(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
    e.call(QUERY_PERFORMANCE_FREQUENCY, &args![PERFORMANCE_FREQUENCY]);
    this
}

// Translated from 0096d290 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists` destructor: for each of the five crime lists
/// (`GlobalCrimeListArray`) destroys every crime in it and deletes the list,
/// then destroys the embedded members in reverse order of construction and
/// finally the process array.
pub fn fn_0096d290(e: &mut Engine, this: Ptr<ProcessLists>) {
    let base = this.addr();
    for i in 0..5u32 {
        let list = e.mem.u32(base + GLOBAL_CRIME_LIST_ARRAY + 4 * i);
        loop {
            if list == 0 || node_value(e, list) == 0 {
                break;
            }
            let crime = node_value(e, list);
            if crime != 0 {
                // The crime's scalar deleting destructor.
                e.call(0x008f_25e0, &args![crime, 1u32]);
            }
            e.call(LIST_POP_FRONT, &args![list]);
        }
        list_delete(e, list);
    }
    e.call(LIST_CLEAR, &args![base + PROJECTILE_POST_PROCESS_LIST]);
    e.call(LIST_CLEAR, &args![base + ALIVE_ACTOR_LIST]);
    e.call(0x0055_a2d0, &args![base + MOVEMENT_SYNC_SEMAPHORE]);
    e.call(0x0090_5e50, &args![base + LIP_BACKGROUND_MANAGER]);
    e.call(0x0046_ffb0, &args![base + UNNAMED_LIST_102E4]);
    e.call(0x008c_9430, &args![base + TASK_MANAGER_AI]);
    e.call(0x0046_ffb0, &args![base + ALIVE_ACTOR_LIST]);
    e.call(0x0046_ffb0, &args![base + TEMP_SHOULD_MOVE_LIST]);
    e.call(0x0046_ffb0, &args![base + PROJECTILE_POST_PROCESS_LIST]);
    e.call(0x0046_ffb0, &args![base + REFERENCE_MUZZLE_FLASH_LIST]);
    e.call(0x004e_e840, &args![base + MAGIC_EFFECT_LIST]);
    e.call(0x004e_e840, &args![base + GLOBAL_TEMP_EFFECT_LIST]);
    e.call(ARRAY_DESTRUCT, &args![base + MOB_PROCESS_ARRAY]);
}

// --- references and the system time clock -----------------------------------

// Translated from 0096d450 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::AddReference` (Xbox PDB): `ProcessArray::AddActor(actor, level)`
/// on the process array. It takes three more stack words it never reads.
pub fn processlists_add_reference(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    actor: u32,
    level: u32,
    _unused_2: u32,
    _unused_3: u32,
    _unused_4: u32,
) {
    e.call(
        ARRAY_ADD_ACTOR,
        &args![mob_process_array(this), actor, level],
    );
}

// Translated from 0096d470 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::RemoveReference` (Xbox PDB): `ProcessArray::RemoveActor(actor, level)`
/// on the process array.
pub fn processlists_remove_reference(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    actor: u32,
    level: u32,
) {
    e.call(
        ARRAY_REMOVE_ACTOR,
        &args![mob_process_array(this), actor, level],
    );
}

// Translated from 0096d490 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::GetSystemTimeClock` (Xbox PDB): the global clock the
/// update advances.
pub fn processlists_get_system_time_clock(e: &mut Engine, _this: Ptr<ProcessLists>) -> f32 {
    e.global(SYSTEM_TIME_CLOCK)
}

// Translated from 0096d4b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::SetSystemTimeClock` (Xbox PDB): stores the clock; a value
/// above 100000.0, a NaN or an infinity is reset to 0.0.
pub fn processlists_set_system_time_clock(e: &mut Engine, _this: Ptr<ProcessLists>, clock: f32) {
    e.set_global(SYSTEM_TIME_CLOCK, clock);
    let limit: f64 = e.global(CLOCK_LIMIT);
    // `JNZ` after `TEST AH, 0x41`: not greater (or unordered) skips the reset.
    if clock as f64 > limit {
        e.set_global(SYSTEM_TIME_CLOCK, 0.0f32);
    }
    let stored: f32 = e.global(SYSTEM_TIME_CLOCK);
    // `_isnan` then `_finite` (CRT, cdecl, a `double`).
    if e.call(0x00ec_75b1, &args![stored as f64]).u32() != 0 {
        e.set_global(SYSTEM_TIME_CLOCK, 0.0f32);
        return;
    }
    let stored: f32 = e.global(SYSTEM_TIME_CLOCK);
    if e.call(0x00ec_7595, &args![stored as f64]).u32() == 0 {
        e.set_global(SYSTEM_TIME_CLOCK, 0.0f32);
    }
}

// --- 0096d520 ---------------------------------------------------------------

// Translated from 0096d520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Dismisses the player's followers that are not in a state to follow. It
/// copies the player's follower list (`00422700` of the player's extra data
/// list) into a new list `all`, then goes through it: for every follower
/// other than the player that has a process: gets the package it is running
/// (`Actor::GetPackageSetAsPcurrent`, Xbox PDB name of `00881510`, replaced by
/// the package extra `0041cb10` when the extra data has one); if there is no
/// package, or its type is neither 1 nor 7, the follower is added to the
/// list `drop`; otherwise, when it has a process level (`00931850`), it is
/// told slot 0x434 with 0, `Actor::EndInterruptPackage(0)` is called and slot
/// 0x25c is called with the float at `0x01084838`. Finally every actor in
/// `drop` is removed from the player's followers
/// (`ExtraDataList::RemoveFollower`), both lists are freed and, if the player's
/// follower list is now empty, the follower extra is removed
/// (`ExtraDataList::RemoveFollowerExtra`). Takes no arguments (the `this`
/// register is not read).
pub fn fn_0096d520(e: &mut Engine) {
    let player = player(e);
    let player_extra = extra_list(e, player);
    let followers = e.call(EXTRA_FOLLOWERS, &args![player_extra]).u32();
    if followers == 0 {
        return;
    }
    let drop = list_new(e);
    let all = list_new(e);

    // Copy the followers into `all`.
    let mut node = e.mem.u32(followers + 0xc);
    while node != 0 {
        if node_value(e, node) == 0 {
            break;
        }
        let item = node_item(e, node);
        e.call(LIST_ADD_ITEM, &args![all, item]);
        node = node_next(e, node);
    }

    let mut node = all;
    while node != 0 {
        if node_value(e, node) == 0 {
            break;
        }
        let actor = node_value(e, node);
        if actor != 0 && actor != player && actor_process(e, actor) != 0 {
            let mut package = e.call(0x0088_1510, &args![actor]).u32();
            let extra = extra_list(e, actor);
            let package_extra = e.call(0x0041_cb10, &args![extra]).u32();
            if package_extra != 0 {
                package = package_extra;
            }
            let mut dismiss = true;
            if package != 0 {
                dismiss = false;
                if e.call(PACKAGE_TYPE, &args![package]).u32() != 1 {
                    dismiss = e.call(PACKAGE_TYPE, &args![package]).u32() != 7;
                }
            }
            if dismiss {
                list_push_value(e, drop, actor);
            } else if e.call(ACTOR_PROCESS_LEVEL, &args![actor]).u32() != 0 {
                e.vcall(actor, 0x434, &args![0u32]);
                e.call(0x0088_1680, &args![actor, 0u32]);
                let delay: f32 = e.global(FOLLOWER_DELAY);
                e.vcall(actor, 0x25c, &args![delay]);
            }
        }
        node = node_next(e, node);
    }

    let mut node = drop;
    while node != 0 {
        if node_value(e, node) == 0 {
            break;
        }
        let actor = node_value(e, node);
        let extra = extra_list(e, player);
        e.call(EXTRA_REMOVE_FOLLOWER, &args![extra, actor]);
        node = node_next(e, node);
    }

    e.call(LIST_CLEAR, &args![drop]);
    list_delete(e, drop);
    e.call(LIST_CLEAR, &args![all]);
    list_delete(e, all);
    let remaining = e.mem.u32(followers + 0xc);
    if e.call(LIST_IS_EMPTY, &args![remaining]).bool() {
        let extra = extra_list(e, player);
        e.call(0x0042_2720, &args![extra]);
    }
}

// --- the per-frame update ---------------------------------------------------

// Translated from 0096d810 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::UpdateProcessLists` (Xbox PDB): the per-frame update.
/// Inside a profiling scope (`00404eb0`, id 0x2a, line 0x521 of
/// `ProcessLists.cpp`): ticks down the two comment timers (while they are
/// not negative) by the frame time, publishes the calendar's hour, year,
/// month and day, runs `008d5bc0`, `977cb0` with two values read from the
/// player and `008d1560`, calls `004772f0` when the integer setting at
/// `0x011c3ea4` is 1, advances the system time clock by the frame time,
/// records the level-0 tail as `iNumberHighActors` and accumulates
/// `fSecondsPassedNoProcess`. Depending on the frame rate (`1 / frame time`,
/// below 50 or below 10) the processing passes are skipped on some frames;
/// when they run, it processes the low, middle-low and middle-high lists
/// (`0096b050`, `0096b470`, `0096b810`, each with `0.0, 0`, under the
/// lock), and ticks down the crime timer or, when it has run out,
/// calls `009723f0`.
pub fn processlists_update_process_lists(e: &mut Engine, this: Ptr<ProcessLists>) {
    e.with_stack(8, |e, scope| {
        e.call(
            0x0040_4eb0,
            &args![scope, 0x2au32, 1u32, SOURCE_FILE, 0x521u32],
        );
        update_process_lists_body(e, this);
        e.call(0x0040_4ee0, &args![scope]);
    });
}

fn update_process_lists_body(e: &mut Engine, this: Ptr<ProcessLists>) {
    let zero: f64 = e.global(ZERO);
    let timer = e.get(this, ProcessLists::fPlayerActionCommentTimer);
    if timer as f64 >= zero {
        let dt = frame_time(e);
        e.set(
            this,
            ProcessLists::fPlayerActionCommentTimer,
            (timer as f64 - dt as f64) as f32,
        );
    }
    let timer = e.get(this, ProcessLists::fPlayerKnockObjectCommentTimer);
    if timer as f64 >= zero {
        let dt = frame_time(e);
        e.set(
            this,
            ProcessLists::fPlayerKnockObjectCommentTimer,
            (timer as f64 - dt as f64) as f32,
        );
    }

    let hour = e.call(0x0086_7da0, &args![CALENDAR]).f32();
    e.set_global(GAME_HOUR, hour);
    let year = e.call(0x0086_7c60, &args![CALENDAR]).i32();
    e.set_global(GAME_YEAR, year);
    let month = e.call(0x0086_7d20, &args![CALENDAR]).i32();
    e.set_global(GAME_MONTH, month);
    let day = e.call(0x0086_7d60, &args![CALENDAR]).u8();
    e.set_global(GAME_DAY, day);
    e.call(0x008d_5bc0, &[]);
    let player = player(e);
    let first = e.call(0x0096_40b0, &args![player, 0u32]).u32();
    let second = e.call(0x0096_4060, &args![player, first]).u32();
    e.call(0x0097_7cb0, &args![this, second]);
    e.call(0x008d_1560, &[]);
    let setting = e.call(SETTING_INT_VALUE, &args![0x011c_3ea4u32]).u32();
    if e.mem.u32(setting) == 1 {
        e.call(0x0047_72f0, &[]);
    }

    let dt = frame_time(e);
    let clock = processlists_get_system_time_clock(e, this);
    processlists_set_system_time_clock(e, this, (clock as f64 + dt as f64) as f32);
    let frame_rate = float_to_int(e, 1.0 / dt as f64);
    let mut run = true;
    let tail = array_tail(e, mob_process_array(this), 0);
    e.set(this, ProcessLists::iNumberHighActors, tail);
    let passed = e.get(this, ProcessLists::fSecondsPassedNoProcess);
    let passed = (passed as f64 + dt as f64) as f32;
    e.set(this, ProcessLists::fSecondsPassedNoProcess, passed);
    if frame_rate < 0x32 {
        let one: f64 = e.global(ONE);
        if frame_rate < 10 {
            if (passed as f64) < one {
                run = false;
                e.set(
                    this,
                    ProcessLists::fSecondsPassedNoProcess,
                    (passed as f64 + dt as f64) as f32,
                );
            } else {
                e.set(this, ProcessLists::fSecondsPassedNoProcess, 0.0);
            }
        } else if (passed as f64) < one {
            let two: f64 = e.global(TWO);
            e.set(
                this,
                ProcessLists::fSecondsPassedNoProcess,
                ((1.0 - dt as f64) / two + passed as f64) as f32,
            );
            run = false;
        } else {
            e.set(this, ProcessLists::fSecondsPassedNoProcess, 0.0);
        }
    }

    if run {
        if e.get(this, ProcessLists::bProcessLow)
            && !e.call(0x0052_5420, &[]).bool()
            && e.call(LOCK_TRY_ENTER, &args![LOCK]).bool()
        {
            e.call(0x0096_b050, &args![this, 0.0f32, 0u32]);
            lock_leave(e, LOCK);
        }
        if e.get(this, ProcessLists::bProcessMLow) && !e.call(0x0052_5420, &[]).bool() {
            lock_enter(e, LOCK);
            e.call(0x0096_b470, &args![this, 0.0f32, 0u32]);
            lock_leave(e, LOCK);
        }
        if e.get(this, ProcessLists::bProcessMHigh) {
            lock_enter(e, LOCK);
            e.call(0x0096_b810, &args![this, 0.0f32, 0u32]);
            lock_leave(e, LOCK);
        }
        let crime_timer = e.get(this, ProcessLists::fCrimeUpdateTimer);
        // `JP` after `TEST AH, 0x41`: positive (or NaN) counts down, the rest runs the update.
        if crime_timer as f64 > zero || crime_timer.is_nan() {
            let dt = frame_time(e);
            e.set(
                this,
                ProcessLists::fCrimeUpdateTimer,
                (crime_timer as f64 - dt as f64) as f32,
            );
        } else {
            e.call(0x0097_23f0, &args![this]);
        }
    }
}

// --- 0096db30 .. 0096e150 ---------------------------------------------------

// Translated from 0096db30 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every object of the given process level (head to tail of the array
/// that `00717e50` returns for the `ProcessLists`): skips references for which
/// `00576d30` is true; for an actor, calls vtable slot 0x348 with a time and
/// `flag`. For level 0 the time is `delta`; for the other levels it is the
/// millisecond counter divided by 1000 (`00825c00`) minus the process's
/// time (`007df1f0`). At level 0 `008e51b0(process, actor)` follows. A
/// non-actor object gets `009bec10(object, 0.0)` when its slot 0x224 is true
/// or else `009ae580(object, 0.0)` when its slot 0x220 is true. `this` is not
/// read.
pub fn fn_0096db30(e: &mut Engine, _this: Ptr<ProcessLists>, delta: f32, flag: u8, level: u32) {
    let mut delta = delta;
    let array = e.call(PROCESS_ARRAY_OF, &args![PROCESS_LISTS]).u32();
    let mut index = array_head(e, array, level);
    while index < array_tail(e, array, level) {
        let object = array_object(e, array, index);
        let skip = object != 0 && e.call(0x0057_6d30, &args![object]).bool();
        if !skip {
            if object != 0 && is_actor(e, object) {
                if level != 0 {
                    let milliseconds = e.call(0x0082_5c00, &args![FRAME_TIMER]).u32();
                    let divisor: f64 = e.global(MILLISECONDS_PER_SECOND);
                    let now = milliseconds as f64 / divisor;
                    let process = actor_process(e, object);
                    let x = e.call(0x007d_f1f0, &args![process]).f32();
                    delta = (now - x as f64) as f32;
                }
                e.vcall(object, 0x348, &args![delta, flag]);
                if level == 0 {
                    let process = actor_process(e, object);
                    e.call(0x008e_51b0, &args![process, object]);
                }
            } else if e.vcall(object, 0x224, &[]).bool() {
                e.call(0x009b_ec10, &args![object, 0.0f32]);
            } else if e.vcall(object, 0x220, &[]).bool() {
                e.call(0x009a_e580, &args![object, 0.0f32]);
            }
        }
        index = index.wrapping_add(1);
    }
}

/// The shared tail of the two fast-travel loops: how many game hours have
/// passed since the time stored in the actor's process (`007df1f0`), clamped,
/// and the call of slot 0x25c with it unless the actor is following (its
/// package, slot 0x22c of the process, has type 2, or `Actor::IsFollowing`)
/// and its process's target (slot 0x128) is the player.
fn advance_actor_for_fast_travel(e: &mut Engine, actor: u32) {
    let process = actor_process(e, actor);
    let last = e.call(0x007d_f1f0, &args![process]).f32();
    let hour = e.call(0x0086_7da0, &args![CALENDAR]).f32();
    let mut elapsed = (hour as f64 - last as f64) as f32;
    if last > hour {
        let day: f64 = e.global(HOURS_PER_DAY);
        elapsed = ((hour as f64 + day) - last as f64) as f32;
    }
    let small: f64 = e.global(SMALL_TIME);
    let mut has_follow_package = false;
    if (elapsed as f64) < small {
        elapsed = 0.0;
    }
    let process = actor_process(e, actor);
    if process != 0 && e.vcall(process, SLOT_0X22C, &[]).u32() != 0 {
        let process = actor_process(e, actor);
        let package = e.vcall(process, SLOT_0X22C, &[]).u32();
        if e.call(PACKAGE_TYPE, &args![package]).u32() == 2 {
            has_follow_package = true;
        }
    }
    let mut skip = false;
    if has_follow_package || e.call(0x0088_42c0, &args![actor]).bool() {
        let process = actor_process(e, actor);
        let target = e.vcall(process, 0x128, &[]).u32();
        skip = target == player(e);
    }
    if !skip {
        e.vcall(actor, 0x25c, &args![elapsed]);
    }
}

// Translated from 0096dcb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds a list of the items of the list at node `list` that answer true
/// to vtable slot 0x100, then for each of them that is not the player, has
/// a process, passes the two model-loader tests and `00493bb0`: advances it
/// for fast travel (`advance_actor_for_fast_travel`). Each node is popped
/// after its turn and the list freed at the end. `this` is not read.
pub fn fn_0096dcb0(e: &mut Engine, _this: Ptr<ProcessLists>, list: u32) {
    let actors = list_new(e);
    let mut node = list;
    while node != 0 {
        if node_value(e, node) == 0 {
            break;
        }
        let object = node_value(e, node);
        if is_actor(e, object) {
            // The push takes the address of the item in the source node.
            let item = node_item(e, node);
            e.call(LIST_PUSH, &args![actors, item]);
        }
        node = node_next(e, node);
    }
    loop {
        if actors == 0 || node_value(e, actors) == 0 {
            break;
        }
        let actor = node_value(e, actors);
        if actor != 0
            && actor != player(e)
            && actor_process(e, actor) != 0
            && !e.call(0x0044_0d80, &args![actor]).bool()
            && !e.call(0x0044_0da0, &args![actor]).bool()
            && !e.call(0x0049_3bb0, &args![actor]).bool()
        {
            advance_actor_for_fast_travel(e, actor);
        }
        e.call(LIST_POP_FRONT, &args![actors]);
    }
    e.call(LIST_CLEAR, &args![actors]);
    list_delete(e, actors);
}

// Translated from 0096df40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::ProcessHighActorsForFastTravel` (Xbox PDB): for every
/// object of process level 0 (from index 0) that is an actor, is not skipped
/// by `00576d30`, has a cell (`008d6f30`) or a world space (`00575d70`), is
/// not the player, has a process, passes the model-loader tests and
/// `00493bb0`: advances it for fast travel (`advance_actor_for_fast_travel`).
pub fn processlists_process_high_actors_for_fast_travel(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        if object != 0
            && is_actor(e, object)
            && !e.call(0x0057_6d30, &args![object]).bool()
            && (e.call(0x008d_6f30, &args![object]).u32() != 0
                || e.call(0x0057_5d70, &args![object]).u32() != 0)
            && object != player(e)
            && actor_process(e, object) != 0
            && !e.call(0x0044_0d80, &args![object]).bool()
            && !e.call(0x0044_0da0, &args![object]).bool()
            && !e.call(0x0049_3bb0, &args![object]).bool()
        {
            advance_actor_for_fast_travel(e, object);
        }
        index = index.wrapping_add(1);
    }
}

// Translated from 0096e150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Goes through the list starting at node `list`. For each node whose item
/// answers true to vtable slot 0xfc (`IsMobileObject`): with `flag` false it
/// adds the item with `AddActorToTempChangeList`; with `flag` true it
/// either (when `00440d80` or `00440da0` holds) removes it from the process
/// array when it has a process and the lock could be entered
/// (`RemoveActor(item, level)`, `00931e80`), or, if it is not the player,
/// has a process and slot 0x260 is false, starts the walk again at the first
/// node.
pub fn fn_0096e150(e: &mut Engine, this: Ptr<ProcessLists>, list: u32, flag: u8) {
    let first = list;
    let mut node = list;
    while node != 0 {
        if node_value(e, node) == 0 {
            break;
        }
        let mut found = 0u32;
        let object = node_value(e, node);
        if e.vcall(object, SLOT_IS_MOBILE_OBJECT, &[]).bool() {
            found = node_value(e, node);
        }
        node = node_next(e, node);
        if found == 0 {
            continue;
        }
        if flag == 0 {
            processlists_add_actor_to_temp_change_list(e, this, found);
            continue;
        }
        if e.call(0x0044_0d80, &args![found]).bool() || e.call(0x0044_0da0, &args![found]).bool() {
            if actor_process(e, found) != 0 && e.call(LOCK_TRY_ENTER, &args![LOCK]).bool() {
                let level = e.call(ACTOR_PROCESS_LEVEL, &args![found]).u32();
                e.call(
                    ARRAY_REMOVE_ACTOR,
                    &args![mob_process_array(this), found, level],
                );
                e.call(0x0093_1e80, &args![found]);
                lock_leave(e, LOCK);
            }
        } else if found != player(e)
            && actor_process(e, found) != 0
            && !e.vcall(found, 0x260, &[]).bool()
        {
            node = first;
        }
    }
}

// --- the AliveActorList (+0x80) ----------------------------------------------

// Translated from 0096e290 (decompiled, FalloutNV.exe 1.4.0.525)
/// `&AliveActorList`.
pub fn fn_0096e290(_e: &mut Engine, this: Ptr<ProcessLists>) -> u32 {
    this.addr() + ALIVE_ACTOR_LIST
}

// Translated from 0096e2b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `actor` to the `AliveActorList` unless it is null or already in it.
pub fn fn_0096e2b0(e: &mut Engine, this: Ptr<ProcessLists>, actor: u32) {
    if actor == 0 {
        return;
    }
    let list = this.addr() + ALIVE_ACTOR_LIST;
    let present = e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), actor);
        e.call(LIST_CONTAINS, &args![list, slot.addr()]).bool()
    });
    if !present {
        list_push_value(e, list, actor);
    }
}

// Translated from 0096e2f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes `actor` from the `AliveActorList`.
pub fn fn_0096e2f0(e: &mut Engine, this: Ptr<ProcessLists>, actor: u32) {
    list_remove_value(e, this.addr() + ALIVE_ACTOR_LIST, actor);
}

// Translated from 0096e310 (decompiled, FalloutNV.exe 1.4.0.525)
/// Rebuilds the `AliveActorList`: clears it, then adds every actor of
/// process level 0 for which slot 0x22c (called with 0) is false.
pub fn fn_0096e310(e: &mut Engine, this: Ptr<ProcessLists>) {
    let list = this.addr() + ALIVE_ACTOR_LIST;
    e.call(LIST_CLEAR, &args![list]);
    let array = e.call(PROCESS_ARRAY_OF, &args![PROCESS_LISTS]).u32();
    let mut index = array_head(e, array, 0);
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        if object != 0 && is_actor(e, object) && !e.vcall(object, SLOT_0X22C, &args![0u32]).bool() {
            list_push_value(e, list, object);
        }
        index = index.wrapping_add(1);
    }
}

// --- the actors close to the player ------------------------------------------

/// The distance of an actor to the player as the sort key: the actor's
/// character controller (`MobileObject::GetCharController`, Xbox PDB, at
/// `009306d0`), then its distance (`008a3b50`, a `float` in ST0). `None`
/// when the actor has no controller.
fn distance_key(e: &mut Engine, actor: u32) -> Option<f32> {
    let controller = e.call(0x0093_06d0, &args![actor]).u32();
    if controller == 0 {
        None
    } else {
        Some(e.call(0x008a_3b50, &args![controller]).f32())
    }
}

/// The integer setting that caps the number of actors close to the player.
fn max_actors_close_to_player(e: &mut Engine) -> i32 {
    let address = e.call(SETTING_INT_VALUE, &args![MAX_ACTORS_SETTING]).u32();
    e.mem.i32(address)
}

// Translated from 0096e3c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::InsertActorCloseToPlayer` (Xbox PDB): inserts `actor` into
/// the distance-sorted array of actors close to the player. An actor
/// without a character controller is ignored. Under the lock: unless the
/// actor is already in the array, finds the first entry whose distance is
/// greater than the actor's (entries without a controller are skipped) and,
/// when that index is below 50 and below the integer setting at
/// `0x011cde2c`, shifts the rest up by one and stores the actor there; the
/// count grows while it is below both limits. (The function also loads the
/// `float` at `0x01016970`, the largest float, as the initial distance; it
/// is overwritten before it is used.)
pub fn processlists_insert_actor_close_to_player(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    actor: u32,
) {
    if actor == 0 {
        return;
    }
    let base = this.addr();
    let Some(distance) = distance_key(e, actor) else {
        return;
    };
    lock_enter(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
    let count = e.get(this, ProcessLists::iActorsCloseToPlayer);
    let mut found = false;
    let mut i = 0i32;
    while i < count {
        if e.mem.u32(base + ACTORS_CLOSE_TO_PLAYER + 4 * i as u32) == actor {
            found = true;
            break;
        }
        i += 1;
    }
    if !found {
        let mut i = 0i32;
        while i < e.get(this, ProcessLists::iActorsCloseToPlayer) {
            let other = e.mem.u32(base + ACTORS_CLOSE_TO_PLAYER + 4 * i as u32);
            if let Some(other_distance) = distance_key(e, other) {
                // `FCOMPP` / `TEST AH, 5` / `JP`: keep going unless ours is smaller.
                if distance < other_distance {
                    break;
                }
            }
            i += 1;
        }
        if i < MAX_ACTORS_CLOSE_TO_PLAYER && i < max_actors_close_to_player(e) {
            let moved = (MAX_ACTORS_CLOSE_TO_PLAYER - 1 - i) as u32;
            let slot = base + ACTORS_CLOSE_TO_PLAYER + 4 * i as u32;
            e.call(
                MEMMOVE,
                &args![slot + 4, ACTORS_CLOSE_TO_PLAYER_BYTES, slot, moved * 4],
            );
            e.mem.set_u32(slot, actor);
            let count = e.get(this, ProcessLists::iActorsCloseToPlayer);
            if count < max_actors_close_to_player(e) && count < MAX_ACTORS_CLOSE_TO_PLAYER {
                e.set(this, ProcessLists::iActorsCloseToPlayer, count + 1);
            }
        }
    }
    lock_leave(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
}

// Translated from 0096e570 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::SortActorsCloseToPlayer` (Xbox PDB): shell sort (gaps
/// 1, 4, 13, ...) of the actors close to the player with
/// `DistanceListCompareFn` (`00970ee0`), under the lock; then drops the last
/// entries while the count exceeds the integer setting at `0x011cde2c`.
pub fn processlists_sort_actors_close_to_player(e: &mut Engine, this: Ptr<ProcessLists>) {
    let base = this.addr();
    let slot = |i: i32| base + ACTORS_CLOSE_TO_PLAYER + (i as u32).wrapping_mul(4);
    lock_enter(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
    let low = 0i32;
    let high = e.get(this, ProcessLists::iActorsCloseToPlayer) - 1;
    let mut gap = 1i32;
    while gap <= (high - low) / 9 {
        gap = gap * 3 + 1;
    }
    while gap > 0 {
        let mut i = low + gap;
        while i <= high {
            let mut j = i;
            let item = e.mem.u32(slot(i));
            while j >= low + gap {
                let before = e.mem.u32(slot(j - gap));
                if e.call(0x0097_0ee0, &args![item, before]).i32() < 0 {
                    e.mem.set_u32(slot(j), before);
                    j -= gap;
                } else {
                    break;
                }
            }
            e.mem.set_u32(slot(j), item);
            i += 1;
        }
        gap /= 3;
    }
    while e.get(this, ProcessLists::iActorsCloseToPlayer) > max_actors_close_to_player(e) {
        let count = e.get(this, ProcessLists::iActorsCloseToPlayer);
        // The entry before the slot of `count`: the last used one.
        e.mem.set_u32(slot(count) - 4, 0);
        e.set(this, ProcessLists::iActorsCloseToPlayer, count - 1);
    }
    lock_leave(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
}

// Translated from 0096e6f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::RemoveActorCloseToPlayer` (Xbox PDB): under the lock,
/// removes `actor` from the array of actors close to the player (shifting
/// the later ones down, clearing the last used slot and decrementing the
/// count).
pub fn processlists_remove_actor_close_to_player(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    actor: u32,
) {
    let base = this.addr();
    lock_enter(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
    let mut i = 0i32;
    while i < e.get(this, ProcessLists::iActorsCloseToPlayer) {
        let slot = base + ACTORS_CLOSE_TO_PLAYER + 4 * i as u32;
        if e.mem.u32(slot) == actor {
            let moved = (MAX_ACTORS_CLOSE_TO_PLAYER - i - 1) as u32;
            e.call(
                MEMMOVE,
                &args![slot, ACTORS_CLOSE_TO_PLAYER_BYTES, slot + 4, moved * 4],
            );
            let count = e.get(this, ProcessLists::iActorsCloseToPlayer);
            e.mem
                .set_u32(base + ACTORS_CLOSE_TO_PLAYER - 4 + 4 * count as u32, 0);
            e.set(this, ProcessLists::iActorsCloseToPlayer, count - 1);
            break;
        }
        i += 1;
    }
    lock_leave(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
}

// Translated from 0096e7d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `actor` is null, the player, or among the first `max` entries
/// of the actors close to the player (`max` is clamped to the count).
pub fn fn_0096e7d0(e: &mut Engine, this: Ptr<ProcessLists>, actor: u32, max: i32) -> bool {
    if actor == 0 || actor == player(e) {
        return true;
    }
    let base = this.addr();
    lock_enter(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
    let count = e.get(this, ProcessLists::iActorsCloseToPlayer);
    // `004a8f20`: the smaller of two signed integers.
    let limit = e.call(0x004a_8f20, &args![max, count]).i32();
    let mut found = false;
    let mut i = 0i32;
    while i < limit {
        if e.mem.u32(base + ACTORS_CLOSE_TO_PLAYER + 4 * i as u32) == actor {
            found = true;
            break;
        }
        i += 1;
    }
    lock_leave(e, base + ACTORS_CLOSE_TO_PLAYER_LOCK);
    found
}

// --- the temporary change list (+0x78) ---------------------------------------

// Translated from 0096e870 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::AddActorToTempChangeList` (Xbox PDB): adds `actor` to the
/// `TempShouldMoveList` unless it is already there. A non-actor is flagged
/// with `SetNeedToChangeProcess(true)` and added. For an actor with a
/// process: when slot 0x22c (called with 0) is true, or its process level
/// differs from the desired level, or its process says slot 0x360, or the
/// model-loader test `00440d80` holds, or the player is sleeping/resting
/// (`PlayerCharacter::IsSleepingorResting`, `0094df60`), it is flagged,
/// added and its byte at +0x82 is set.
pub fn processlists_add_actor_to_temp_change_list(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    actor: u32,
) {
    let list = this.addr() + TEMP_SHOULD_MOVE_LIST;
    let present = e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), actor);
        e.call(LIST_CONTAINS, &args![list, slot.addr()]).bool()
    });
    if present {
        return;
    }
    if !is_actor(e, actor) {
        e.vcall(actor, SLOT_SET_NEED_TO_CHANGE_PROCESS, &args![1u32]);
        list_push_value(e, list, actor);
        return;
    }
    let process = actor_process(e, actor);
    if process == 0 {
        return;
    }
    let level = e.call(PROCESS_LEVEL, &args![process]).u32();
    let desired = e.call(ACTOR_DESIRED_LEVEL, &args![actor]).u32();
    let add = e.vcall(actor, SLOT_0X22C, &args![0u32]).bool()
        || level != desired
        || {
            let process = actor_process(e, actor);
            e.vcall(process, 0x360, &[]).bool()
        }
        || e.call(0x0044_0d80, &args![actor]).bool()
        || {
            let player = player(e);
            e.call(0x0094_df60, &args![player]).bool()
        };
    if add {
        e.vcall(actor, SLOT_SET_NEED_TO_CHANGE_PROCESS, &args![1u32]);
        list_push_value(e, list, actor);
        if actor != 0 {
            e.mem.set_u8(actor + 0x82, 1);
        }
    }
}

// Translated from 0096e9b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Goes through the `TempShouldMoveList`; for each actor that is not skipped
/// by `00576d30` and has a process whose slot 0x27c answers non-zero (a
/// package): if that package's type (`0041ca90`) is 2 and the actor's
/// `00881650` object is an actor, the process is told slot 0xc8
/// (`actor, package, 0`); otherwise, when `00881650` is an actor other than
/// the player, `Actor::AddFollower(actor, that)` (Xbox PDB name of
/// `008bc790`).
pub fn fn_0096e9b0(e: &mut Engine, this: Ptr<ProcessLists>) {
    let mut node = this.addr() + TEMP_SHOULD_MOVE_LIST;
    while node != 0 {
        if node_value(e, node) == 0 {
            break;
        }
        let mut found = 0u32;
        let object = node_value(e, node);
        if is_actor(e, object) {
            found = node_value(e, node);
        }
        if found != 0 && !e.call(0x0057_6d30, &args![found]).bool() {
            let process = actor_process(e, found);
            if process != 0 && e.vcall(process, 0x27c, &[]).u32() != 0 {
                let leader = e.call(0x0088_1650, &args![found]).u32();
                let process = actor_process(e, found);
                let package = e.vcall(process, 0x27c, &[]).u32();
                if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 2 {
                    let leader = e.call(0x0088_1650, &args![found]).u32();
                    if leader != 0 && is_actor(e, leader) {
                        let process = actor_process(e, found);
                        e.vcall(process, 0xc8, &args![found, package, 0u32]);
                    }
                } else if leader != 0 && is_actor(e, leader) && leader != player(e) {
                    e.call(0x008b_c790, &args![found, leader]);
                }
            }
        }
        node = node_next(e, node);
    }
}

// Translated from 0096eb40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::ChangeProcessLevelTempList` (Xbox PDB): works through the
/// `TempShouldMoveList` changing the process level of each mobile object
/// (`TEMP_CHANGE_ACTOR` is the one being handled; the code re-reads the
/// global at every use, so do the helpers). It returns at once while the
/// player is sleeping or resting (`PlayerCharacter::IsSleepingorResting`).
/// For each entry that answers true to slot 0xfc (`IsMobileObject`) and is
/// not skipped by `00576d30`, `handle_temp_change_actor` decides what happens
/// and whether the walk restarts at the head of the list; when the list is
/// exhausted every remaining item is flagged with
/// `SetNeedToChangeProcess(true)`.
///
/// An entry for which slot 0xfc is false is retried without advancing (the
/// game relies on the slot being true for everything in the list).
pub fn processlists_change_process_level_temp_list(e: &mut Engine, this: Ptr<ProcessLists>) {
    let player = player(e);
    if e.call(0x0094_df60, &args![player]).bool() {
        return;
    }
    let head = this.addr() + TEMP_SHOULD_MOVE_LIST;
    let mut node = head;
    while node != 0 && node_value(e, node) != 0 {
        e.set_global(TEMP_CHANGE_ACTOR, 0u32);
        let first = node_value(e, node);
        if e.vcall(first, SLOT_IS_MOBILE_OBJECT, &[]).bool() {
            let value = node_value(e, node);
            e.set_global(TEMP_CHANGE_ACTOR, value);
        }
        if temp_actor(e) == 0 {
            continue;
        }
        let mut high = 0u32;
        let current = temp_actor(e);
        if is_actor(e, current) {
            high = temp_actor(e);
        }
        node = node_next(e, node);
        let current = temp_actor(e);
        if e.call(0x0057_6d30, &args![current]).bool() {
            continue;
        }
        if let Step::Restart = handle_temp_change_actor(e, this, high) {
            node = head;
        }
    }

    // Everything left is flagged to be looked at again.
    let mut node = head;
    while node != 0 && node_value(e, node) != 0 {
        let object = node_value(e, node);
        e.vcall(object, SLOT_SET_NEED_TO_CHANGE_PROCESS, &args![1u32]);
        node = node_next(e, node);
    }
}

/// What `ChangeProcessLevelTempList` does after handling one entry.
enum Step {
    /// Go on with the next node.
    Next,
    /// Start again at the head of the list.
    Restart,
}

/// The actor `ChangeProcessLevelTempList` is working on.
fn temp_actor(e: &Engine) -> u32 {
    e.global(TEMP_CHANGE_ACTOR)
}

/// Removes `actor` from the `TempShouldMoveList` and clears its byte at +0x82
/// (the tail every branch of `ChangeProcessLevelTempList` shares).
fn remove_from_temp_list(e: &mut Engine, head: u32, actor: u32) {
    list_remove_value(e, head, actor);
    if actor != 0 {
        e.mem.set_u8(actor + 0x82, 0);
    }
}

/// The same for the current actor (read from the global again).
fn remove_temp_actor_from_list(e: &mut Engine, head: u32) {
    let actor = temp_actor(e);
    remove_from_temp_list(e, head, actor);
}

/// `ProcessArray::RemoveActor(current, level)` when the current actor has a
/// process.
fn remove_temp_actor_from_array(e: &mut Engine, array: u32) {
    let actor = temp_actor(e);
    if actor_process(e, actor) != 0 {
        let level = e.call(ACTOR_PROCESS_LEVEL, &args![actor]).u32();
        e.call(ARRAY_REMOVE_ACTOR, &args![array, actor, level]);
    }
}

/// Deletes the current actor under the lock (`~TESForm`, flags 1).
fn delete_temp_actor_under_lock(e: &mut Engine) {
    lock_enter(e, LOCK);
    let actor = temp_actor(e);
    if actor != 0 {
        e.vcall(actor, SLOT_DELETING_DESTRUCTOR, &args![1u32]);
    }
    lock_leave(e, LOCK);
}

/// A reference without a file the engine could reload it from: the model
/// loader test `00440d80` holds and the file lookup `00484e60(actor, -1)`
/// finds none.
fn temp_actor_has_no_file(e: &mut Engine) -> bool {
    let actor = temp_actor(e);
    e.call(0x0044_0d80, &args![actor]).bool()
        && e.call(0x0048_4e60, &args![actor, 0xffff_ffffu32]).u32() == 0
}

/// When one of the model-loader tests (`00440d80`, `00440da0`) holds for the
/// current actor: if it has a process and the lock can be entered, removes it
/// from the process array, runs `00931e80` on it and leaves the lock. Returns
/// whether the model-loader test held.
fn release_loaded_model_actor(e: &mut Engine, array: u32) -> bool {
    let actor = temp_actor(e);
    if !(e.call(0x0044_0d80, &args![actor]).bool() || e.call(0x0044_0da0, &args![actor]).bool()) {
        return false;
    }
    let actor = temp_actor(e);
    if actor_process(e, actor) != 0 && e.call(LOCK_TRY_ENTER, &args![LOCK]).bool() {
        let actor = temp_actor(e);
        let level = e.call(ACTOR_PROCESS_LEVEL, &args![actor]).u32();
        e.call(ARRAY_REMOVE_ACTOR, &args![array, actor, level]);
        e.call(0x0093_1e80, &args![actor]);
        lock_leave(e, LOCK);
    }
    true
}

/// Handles the current actor (`TEMP_CHANGE_ACTOR`; `high` is it when it is an
/// actor). Four cases, tried in this order:
///
/// 1. it is an actor without a process, or the player is talking to somebody
///    else about a reference (`00574900`, `Interface::InDialogWith`): it leaves
///    the process array (and is deleted, `AddReference`d or released as the
///    flags say) and the list;
/// 2. a dead, non-essential high actor whose death time has passed and whose
///    cell is not loaded is removed with its dropped items;
/// 3. a loaded-model actor is released at once if the lock is free;
/// 4. otherwise, depending on the menu mode and `iFormFlags`, the actor is
///    taken out of the list and put on the process level it asks for.
fn handle_temp_change_actor(e: &mut Engine, this: Ptr<ProcessLists>, high: u32) -> Step {
    let array = mob_process_array(this);
    let head = this.addr() + TEMP_SHOULD_MOVE_LIST;

    // Case 1: the negation of `(!actor || process != 0) && (!574900 || dialog == current)`.
    let current = temp_actor(e);
    let mut case_one = false;
    if is_actor(e, current) {
        let current = temp_actor(e);
        case_one = actor_process(e, current) == 0;
    }
    if !case_one {
        let current = temp_actor(e);
        if e.call(0x0057_4900, &args![current]).bool() {
            let talking_to = e.call(0x0070_5190, &[]).u32();
            case_one = talking_to != temp_actor(e);
        }
    }
    if case_one {
        let current = temp_actor(e);
        if e.call(0x0057_4900, &args![current]).bool()
            && e.call(0x0092_f160, &args![current]).bool()
        {
            processlists_add_reference(e, Ptr::new(PROCESS_LISTS), current, 0, 0, 0, 0);
        } else if temp_actor_has_no_file(e)
            || (e.call(0x0057_4900, &args![temp_actor(e)]).bool()
                && !e.call(0x0092_f160, &args![temp_actor(e)]).bool())
        {
            remove_temp_actor_from_array(e, array);
            delete_temp_actor_under_lock(e);
        } else {
            release_loaded_model_actor(e, array);
        }
        remove_temp_actor_from_list(e, head);
        return Step::Restart;
    }

    // Case 2.
    if high != 0
        && e.vcall(high, SLOT_0X22C, &args![0u32]).bool()
        && !e.call(0x0087_f3d0, &args![high]).bool()
        && e.call(0x0087_f4a0, &args![high]).bool()
    {
        let cell = e.call(0x008d_6f30, &args![high]).u32();
        let tes: u32 = e.global(TES);
        if !e.call(0x0045_11e0, &args![tes, cell, 0u32]).bool() {
            let process = actor_process(e, high);
            let death_time = e.vcall(process, 0x69c, &[]).f32();
            if death_time != 0.0 {
                let process = actor_process(e, high);
                let death_time = e.vcall(process, 0x69c, &[]).f32();
                let ticks = e.call(0x0052_6100, &[]).u32();
                let deadline = ticks as f64 + death_time as f64;
                let now = e.call(0x0086_7e30, &args![CALENDAR]).u32();
                if now as f64 > deadline {
                    let zone = e.call(0x0056_7d20, &args![high]).u32();
                    if zone != 0 && e.call(0x0052_6320, &args![zone]).bool() {
                        return Step::Next;
                    }
                    delete_dead_actor(e, head, high);
                    return Step::Restart;
                }
            }
        }
    }

    // Case 3.
    let current = temp_actor(e);
    if e.call(0x0044_0da0, &args![current]).bool() && e.call(LOCK_TRY_ENTER, &args![LOCK]).bool() {
        remove_temp_actor_from_array(e, array);
        let current = temp_actor(e);
        e.call(0x0093_1e80, &args![current]);
        remove_temp_actor_from_list(e, head);
        lock_leave(e, LOCK);
        return Step::Restart;
    }

    // Case 4.
    let current = temp_actor(e);
    let talking = e.call(0x0057_4900, &args![current]).bool();
    let menu_other = talking && e.call(0x0070_2640, &[]).u32() != 0x3f1;
    if !menu_other {
        let current = temp_actor(e);
        if e.call(0x0044_ddc0, &args![current]).u32() & 0x2_0000 == 0 {
            return Step::Next;
        }
        let current = temp_actor(e);
        if e.call(0x0057_4900, &args![current]).bool() {
            return Step::Next;
        }
    }
    let current = temp_actor(e);
    e.vcall(current, SLOT_SET_NEED_TO_CHANGE_PROCESS, &args![0u32]);
    remove_temp_actor_from_list(e, head);
    // Slot 0x224 is called; the game keeps its result in a local it never reads.
    let current = temp_actor(e);
    e.vcall(current, 0x224, &[]);
    if temp_actor_has_no_file(e) {
        remove_temp_actor_from_array(e, array);
        delete_temp_actor_under_lock(e);
        return Step::Restart;
    }
    if release_loaded_model_actor(e, array) {
        return Step::Restart;
    }
    let current = temp_actor(e);
    let process = actor_process(e, current);
    if process != 0 && e.vcall(process, 0x360, &[]).bool() {
        let process = actor_process(e, current);
        let package = e.vcall(process, SLOT_0X22C, &[]).u32();
        if package == 0
            || (e.call(PACKAGE_TYPE, &args![package]).u32() != 1
                && e.call(PACKAGE_TYPE, &args![package]).u32() != 2)
        {
            let process = actor_process(e, current);
            e.vcall(process, 0x364, &args![0u32]);
            let process = actor_process(e, current);
            let level = e.call(PROCESS_LEVEL, &args![process]).u32();
            e.call(ARRAY_ADD_ACTOR, &args![array, current, level]);
        }
    } else {
        e.vcall(current, 0x260, &[]);
        let current = temp_actor(e);
        if current != 0 && e.call(0x0044_ddc0, &args![current]).u32() & 0x2_0000 != 0 {
            e.vcall(current, SLOT_SET_NEED_TO_CHANGE_PROCESS, &args![0u32]);
        }
    }
    Step::Restart
}

/// A dead high actor is gone for good: slot 0x324 (1, 0, 0), then every item
/// in its dropped-item list is taken out (`ExtraDataList::AddDroppedItem`),
/// marked as deleted unless slot 0x160 says it should stay, the list is
/// removed, and the actor leaves the temp list.
fn delete_dead_actor(e: &mut Engine, head: u32, high: u32) {
    e.vcall(high, 0x324, &args![1u32, 0u32, 0u32]);
    let extra = extra_list(e, high);
    let dropped = e.call(0x0041_df90, &args![extra]).u32();
    if dropped != 0 {
        while !e.call(LIST_IS_EMPTY, &args![dropped]).bool() {
            let item = node_value(e, dropped);
            let extra = extra_list(e, item);
            e.call(0x0041_de40, &args![extra, 0u32]);
            if !e.vcall(item, 0x160, &[]).bool() {
                e.call(0x0057_2270, &args![item]);
            }
            e.call(LIST_POP_FRONT, &args![dropped]);
        }
        let extra = extra_list(e, high);
        e.call(0x0041_dfd0, &args![extra]);
    }
    remove_temp_actor_from_list(e, head);
}

// Translated from 0096f400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::RemoveActorFromTempChangeList` (Xbox PDB): clears
/// `TEMP_CHANGE_ACTOR` if it is `actor`, removes `actor` from the
/// `TempShouldMoveList` and clears its byte at +0x82.
pub fn processlists_remove_actor_from_temp_change_list(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    actor: u32,
) {
    if actor == e.global::<u32>(TEMP_CHANGE_ACTOR) {
        e.set_global(TEMP_CHANGE_ACTOR, 0u32);
    }
    list_remove_value(e, this.addr() + TEMP_SHOULD_MOVE_LIST, actor);
    if actor != 0 {
        e.mem.set_u8(actor + 0x82, 0);
    }
}

// Translated from 0096f450 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every object of process level 3 that is an actor with a process:
/// tells the process slot 0x480 with `a` and, when `004839c0(a)` is
/// non-zero, slot 0x664 with that value; then, if the actor's slot 0x2c8
/// answers an object whose `0084e3a0` equals `a` and the actor is not `b`,
/// collects the actor in a list (created on first use). Returns the list,
/// or null.
pub fn fn_0096f450(e: &mut Engine, this: Ptr<ProcessLists>, a: u32, b: u32) -> u32 {
    let mut result = 0u32;
    let level = e.call(0x0048_39c0, &args![a]).u32();
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < e.call(0x0055_b980, &args![array]).u32() {
        let object = array_object(e, array, index);
        let mut high = 0u32;
        if object != 0 && is_actor(e, object) {
            high = object;
        }
        if high != 0 {
            let process = actor_process(e, high);
            if process != 0 {
                e.vcall(process, 0x480, &args![a]);
                if level != 0 {
                    e.vcall(process, 0x664, &args![level]);
                }
            }
            if e.vcall(high, 0x2c8, &[]).u32() != 0 {
                let holder = e.vcall(high, 0x2c8, &[]).u32();
                if e.call(0x0084_e3a0, &args![holder]).u32() == a && b != high {
                    if result == 0 {
                        result = list_new(e);
                    }
                    list_push_value(e, result, high);
                }
            }
        }
        index = index.wrapping_add(1);
    }
    result
}

// Translated from 009707f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the dword at +0xFC of the object it is called on (the
/// callers pass a non-high mobile object, an `Actor`).
pub fn fn_009707f0(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr() + 0xfc, value);
}

// Translated from 00970810 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every object of process level 3 (from the head of level 0 to the tail
/// of level 3) that is an actor and not skipped by `00576d30`: if the
/// actor's process has a package (slot 0x27c) other than its current one
/// (slot 0x22c) and the process level is 0 or 1, and the package is `target`,
/// tells the process slot 0x214; if its current package (slot 0x22c) is
/// `target`, tells slot 0x234 and (unless `0042ce10` of the global at
/// `0x011ddf38` holds) slot 0x24 with `actor, 1`; and if the actor's package
/// extra (`0041cb10`) is `target`, calls `ExtraDataList::SetPackageExtra`
/// with zeros. `this` is not read.
pub fn fn_00970810(e: &mut Engine, _this: Ptr<ProcessLists>, target: u32) {
    let array = e.call(PROCESS_ARRAY_OF, &args![PROCESS_LISTS]).u32();
    let mut index = array_head(e, array, 0);
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) && !e.call(0x0057_6d30, &args![object]).bool() {
            actor = object;
        }
        if actor != 0 {
            let mut package = 0u32;
            if actor_process(e, actor) != 0 {
                let process = actor_process(e, actor);
                package = e.vcall(process, 0x27c, &[]).u32();
            }
            if actor_process(e, actor) != 0 && {
                let process = actor_process(e, actor);
                package != e.vcall(process, SLOT_0X22C, &[]).u32()
            } {
                let mut level_zero_or_one = 0u32;
                if actor_process(e, actor) != 0 {
                    let process = actor_process(e, actor);
                    let level = e.call(PROCESS_LEVEL, &args![process]).i32();
                    if (0..=1).contains(&level) {
                        level_zero_or_one = actor_process(e, actor);
                    }
                }
                if package == target && level_zero_or_one != 0 {
                    e.vcall(level_zero_or_one, 0x214, &[]);
                }
            }
            if actor_process(e, actor) != 0 && {
                let process = actor_process(e, actor);
                e.vcall(process, SLOT_0X22C, &[]).u32() == target
            } {
                let process = actor_process(e, actor);
                e.vcall(process, 0x234, &[]);
                let object: u32 = e.global(0x011d_df38);
                if !e.call(0x0042_ce10, &args![object]).bool() {
                    let process = actor_process(e, actor);
                    e.vcall(process, 0x24, &args![actor, 1u32]);
                }
            }
            let extra = extra_list(e, actor);
            if e.call(0x0041_cb10, &args![extra]).u32() == target {
                let extra = extra_list(e, actor);
                e.call(
                    0x0041_c930,
                    &args![extra, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32],
                );
            }
        }
        index = index.wrapping_add(1);
    }
}

// --- 0096f600: forgetting a reference that is going away ---------------------

/// Clears the location reference of a package-like object (`pk`) when it is
/// `target`: `0055b980(pk)` is its location, `0067f390` that location's
/// reference (`PackageLocation::GetLocReference`, Xbox PDB), `0067f3c0` the setter.
fn clear_location_reference(e: &mut Engine, package: u32, target: u32) {
    if package == 0 {
        return;
    }
    if e.call(0x0055_b980, &args![package]).u32() != 0 {
        let location = e.call(0x0055_b980, &args![package]).u32();
        if e.call(0x0067_f390, &args![location]).u32() == target {
            let location = e.call(0x0055_b980, &args![package]).u32();
            e.call(0x0067_f3c0, &args![location, 0u32]);
        }
    }
}

/// Ends the package of `high` when its package target is `target`: either
/// `Actor::GetPackageTarget`-like `00881650(high)` is `target`, or the object
/// returned by slot 0x20c of the process has a `00671d10` whose
/// `PackageTarget::GetTargReference` is `target`. In dialogue the actor gets
/// slot 0x288; otherwise a created package is ended with
/// `Actor::EndInterruptPackage(0)`; otherwise the process gets slots 0x234
/// and 0x214.
fn end_package_if_target(e: &mut Engine, high: u32, process: u32, target: u32) {
    let holder = e.vcall(process, 0x20c, &[]).u32();
    let mut hit = e.call(0x0088_1650, &args![high]).u32() == target;
    if !hit && holder != 0 && e.call(0x0067_1d10, &args![holder]).u32() != 0 {
        let inner = e.call(0x0067_1d10, &args![holder]).u32();
        hit = e.call(0x0068_0020, &args![inner]).u32() == target;
    }
    if !hit {
        return;
    }
    if e.call(0x0093_36c0, &args![high]).bool() {
        e.vcall(high, 0x288, &[]);
    } else {
        let package = e.call(0x0093_44a0, &args![high]).u32();
        if e.call(0x0067_4d40, &args![package]).bool() {
            e.call(0x0088_1680, &args![high, 0u32]);
        } else {
            e.vcall(process, 0x234, &[]);
            e.vcall(process, 0x214, &[]);
        }
    }
}

/// When the process's furniture (slot 0x4c8) is `target`, resets the
/// furniture marker (heading 0.0, `00568ab0` with 0, `0098ddd0` with the word
/// at `FURNITURE_ARGUMENT`) and tells the process slot 0x540 with `0, 0x7f` and
/// the marker the actor's process has.
fn release_furniture(e: &mut Engine, high: u32, process: u32, target: u32) {
    if e.vcall(process, 0x4c8, &[]).u32() != target {
        return;
    }
    let marker = e.vcall(process, 0x4d4, &[]).u32();
    // FurnitureMark::SetHeading (Xbox PDB)
    e.call(0x00c5_4550, &args![marker, 0.0f32]);
    let marker = e.vcall(process, 0x4d4, &[]).u32();
    e.call(0x0056_8ab0, &args![marker, 0u32]);
    let marker = e.vcall(process, 0x4d4, &[]).u32();
    e.call(0x0098_ddd0, &args![marker, FURNITURE_ARGUMENT]);
    let other = actor_process(e, high);
    let marker = e.vcall(other, 0x4d4, &[]).u32();
    e.vcall(process, 0x540, &args![0u32, 0x7fu32, marker]);
}

/// The part of `IsFleeing` handling both loops of `fn_0096f600` share: if the
/// actor's package is of type 0x16 and its `005e3fc0` is `target`, calls
/// `00994ef0` on it; then `Actor::RemoveFleeTarget(high, target)`.
fn forget_flee_target(e: &mut Engine, high: u32, target: u32) {
    let package = e.call(0x0093_44a0, &args![high]).u32();
    let mut flee = 0u32;
    if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x16 {
        flee = package;
    }
    if flee != 0 && e.call(0x005e_3fc0, &args![flee]).u32() == target {
        e.call(0x0099_4ef0, &args![flee, 0u32]);
    }
    e.call(0x008a_6730, &args![high, target]);
}

/// `ExtraData` record 0x19 of `high`: clears its reference at +0x14 when it is
/// `target`.
fn clear_extra_record_reference(e: &mut Engine, high: u32, target: u32) {
    let extra = extra_list(e, high);
    let record = e.call(0x0041_0220, &args![extra, 0x19u32]).u32();
    if record != 0 && e.mem.u32(record + 0x14) == target {
        e.mem.set_u32(record + 0x14, 0);
    }
}

/// The address `target + 0x94` the process's slot 0x3d0 is compared with, or 0
/// for a null target (the base-class adjustment from the reference to its
/// second base).
fn second_base_of(target: u32) -> u32 {
    if target == 0 {
        0
    } else {
        target + 0x94
    }
}

// Translated from 0096f600 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forgets a reference that is going away (`target`, a `TESObjectREFR`; with
/// `flag` also the package locations that point at it) in the player, in every
/// object of process level 3 and in every object of the `TempShouldMoveList`.
///
/// For the player and the actors: if `target` is an actor it leaves the
/// player's followers, the player's process drops it as a detection actor
/// (slot 0x2c4, `target, 3`) and the `AliveActorList`; the player's world
/// space, last head-track target, last greeted, ... are cleared if they are
/// `target`. Then every object of level 3 that is not skipped by `00576d30`
/// and every object of the temp list is cleaned: its followers' extra data, its
/// head-track target, its package targets and locations, combat target,
/// dialogue target, furniture, flee target and so on, whichever mention
/// `target`. The two loops differ only in details: the first does the full
/// clean-up (including slot 0x8 `HandleDeletedReference`, the actor package
/// data and the dialogue clean-up below level 2), the second the shorter one.
pub fn fn_0096f600(e: &mut Engine, this: Ptr<ProcessLists>, target: u32, flag: u8) {
    let array = mob_process_array(this);
    let player = player(e);

    if is_actor(e, target) {
        let extra = extra_list(e, player);
        if e.call(EXTRA_FOLLOWERS, &args![extra]).u32() != 0 {
            let extra = extra_list(e, player);
            let followers = e.call(EXTRA_FOLLOWERS, &args![extra]).u32();
            if e.mem.u32(followers + 0xc) != 0 {
                let extra = extra_list(e, player);
                let followers = e.call(EXTRA_FOLLOWERS, &args![extra]).u32();
                let list = e.mem.u32(followers + 0xc);
                list_remove_value(e, list, target);
            }
        }
        if actor_process(e, player) != 0 {
            let process = actor_process(e, player);
            e.vcall(process, 0x2c4, &args![target, 3u32]);
        }
        fn_0096e2f0(e, this, target);
    }
    if e.call(0x004f_d380, &args![player]).u32() == target {
        e.call(0x0057_bd60, &args![player, 0u32]);
    }
    if actor_process(e, player) != 0 {
        let process = actor_process(e, player);
        if e.vcall(process, 0x68c, &[]).u32() == target {
            let process = actor_process(e, player);
            e.vcall(process, 0x26c, &[]);
        }
        let process = actor_process(e, player);
        if e.vcall(process, 0x484, &[]).u32() == target {
            let process = actor_process(e, player);
            e.vcall(process, 0x488, &args![0u32]);
        }
        let process = actor_process(e, player);
        let record = e.vcall(process, 0xf8, &args![player]).u32();
        if record != 0 && e.mem.u32(record + 0x18) == target && actor_process(e, player) != 0 {
            let process = actor_process(e, player);
            e.vcall(process, 0x100, &[]);
        }
    }
    if is_actor(e, target) {
        e.vcall(player, 0x468, &args![target]);
    }

    // The objects of process level 3.
    let mut index = 0u32;
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        let mut high = 0u32;
        let mut other = 0u32;
        if object != 0 && !e.call(0x0057_6d30, &args![object]).bool() {
            if is_actor(e, object) {
                high = object;
            } else if e.vcall(object, 0x224, &[]).bool() {
                other = object;
            }
            let mut list = 0u32;
            let mut followers = 0u32;
            if high != 0 {
                let extra = extra_list(e, high);
                followers = e.call(EXTRA_FOLLOWERS, &args![extra]).u32();
            }
            if followers != 0 {
                list = e.mem.u32(followers + 0xc);
            }
            if high != 0
                && {
                    let extra = extra_list(e, high);
                    e.call(EXTRA_FOLLOWERS, &args![extra]).u32() != 0
                }
                && is_actor(e, target)
            {
                let extra = extra_list(e, high);
                e.call(EXTRA_REMOVE_FOLLOWER, &args![extra, target]);
            }
            if other != 0 && e.call(0x0087_4480, &args![other]).u32() == target {
                fn_009707f0(e, Ptr::new(other), 0);
            }
            if high == 0 {
                if e.call(0x005e_3fa0, &args![object]).u32() != 0
                    && target == e.call(0x005e_3fa0, &args![object]).u32()
                {
                    e.vcall(object, 0x288, &[]);
                }
            } else {
                forget_reference_in_level_three_actor(e, high, list, target, flag);
            }
        }
        index = index.wrapping_add(1);
    }

    // The objects of the temp list.
    let mut node = this.addr() + TEMP_SHOULD_MOVE_LIST;
    while node != 0 && !e.call(LIST_IS_EMPTY, &args![node]).bool() {
        let mut high = 0u32;
        let mut other = 0u32;
        let item = node_value(e, node);
        if item != 0 && !e.call(0x0057_6d30, &args![item]).bool() {
            let object = node_value(e, node);
            if is_actor(e, object) {
                high = node_value(e, node);
            } else {
                let object = node_value(e, node);
                if e.vcall(object, 0x224, &[]).bool() {
                    other = node_value(e, node);
                }
            }
            let mut list = 0u32;
            let mut followers = 0u32;
            if high != 0 {
                let extra = extra_list(e, high);
                followers = e.call(EXTRA_FOLLOWERS, &args![extra]).u32();
            }
            if followers != 0 {
                list = e.mem.u32(followers + 0xc);
            }
            if other != 0 && e.call(0x0087_4480, &args![other]).u32() == target {
                fn_009707f0(e, Ptr::new(other), 0);
            }
            forget_reference_in_temp_list_actor(e, high, list, target, index);
        }
        node = node_next(e, node);
    }
}

/// Moves on to the next follower in the follower list of the extra data:
/// the next `high` is the follower if it is an actor (0 otherwise, or at the
/// end of the list), with the list advanced.
fn next_follower(e: &mut Engine, list: &mut u32) -> u32 {
    if *list != 0 && node_value(e, *list) != 0 {
        let mut high = 0u32;
        let follower = node_value(e, *list);
        if is_actor(e, follower) {
            high = node_value(e, *list);
        }
        *list = node_next(e, *list);
        high
    } else {
        0
    }
}

/// The clean-up of the first loop of `fn_0096f600` for the high actor
/// `high` and then for its followers (`list` is the follower list node).
fn forget_reference_in_level_three_actor(
    e: &mut Engine,
    high: u32,
    list: u32,
    target: u32,
    flag: u8,
) {
    let player = player(e);
    let mut high = high;
    let mut list = list;
    while high != 0 {
        e.call(0x008b_3200, &args![high, target]);
        let extra = extra_list(e, high);
        if extra != 0 {
            let extra = extra_list(e, high);
            if e.call(0x0041_9ea0, &args![extra]).u32() == target {
                let extra = extra_list(e, high);
                e.call(0x0041_9dc0, &args![extra, 0u32]);
            }
        }
        let process = actor_process(e, high);
        if e.call(0x004f_d380, &args![high]).u32() == target {
            e.call(0x0057_bd60, &args![high, 0u32]);
            if process != 0 {
                e.vcall(process, 0x310, &args![0u32]);
                e.vcall(process, 0x4a0, &[]);
            }
        }
        if process == 0 {
            return;
        }
        if e.call(PROCESS_LEVEL, &args![process]).i32() < 2 {
            e.vcall(process, 0x2c4, &args![target, 3u32]);
            e.vcall(process, 0x260, &args![target]);
            e.vcall(process, 0x7b0, &args![target]);
            let record = e.vcall(process, 0xf8, &args![player]).u32();
            if record != 0 && e.mem.u32(record + 0x18) == target {
                e.vcall(process, 0x100, &[]);
            }
            if e.vcall(process, 0x68c, &[]).u32() == target {
                e.vcall(process, 0x26c, &[]);
            }
            if e.vcall(process, 0x484, &[]).u32() == target {
                e.vcall(process, 0x488, &args![0u32]);
            }
            if is_actor(e, target) {
                e.vcall(process, 0xc0, &args![target]);
            }
            let package = e.vcall(process, 0x20c, &[]).u32();
            if package != 0 {
                let holder = e.vcall(process, 0x278, &[]).u32();
                if holder != 0 && e.call(0x0044_ddc0, &args![holder]).u32() == target {
                    e.call(0x0088_1680, &args![high, 0u32]);
                } else if e.call(PACKAGE_TYPE, &args![package]).u32() == 0x1c
                    && e.call(0x008d_80e0, &args![package]).u32() == target
                {
                    e.vcall(high, 0x288, &[]);
                }
            }
        }
        e.vcall(process, 0x47c, &args![target]);
        clear_extra_record_reference(e, high, target);
        if e.call(0x008a_6650, &args![high, 0u32]).bool() {
            forget_flee_target(e, high, target);
        } else if e.call(0x008a_6170, &args![high]).bool() && is_actor(e, target) {
            let package = e.call(0x0093_44a0, &args![high]).u32();
            let mut created = 0u32;
            if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x18 {
                created = package;
            }
            if created != 0 {
                e.call(0x009f_8420, &args![created, target]);
            }
        }
        e.vcall(process, 0x8, &args![target]);
        let extra = extra_list(e, high);
        if extra != 0 {
            let package_data = e.call(0x0042_12b0, &args![extra]).u32();
            if package_data != 0 {
                e.vcall(package_data, 0x4, &args![target]);
            }
            if e.call(0x0041_cb70, &args![extra]).u32() == target {
                e.call(0x0041_cab0, &args![extra, 0u32]);
            }
        }
        if is_actor(e, target) && e.vcall(process, 0x3d0, &[]).u32() == second_base_of(target) {
            e.vcall(process, 0x3d4, &args![0u32]);
        }
        if e.call(PROCESS_LEVEL, &args![process]).u32() == 0 {
            e.vcall(process, 0xac, &[]);
        }
        if e.call(0x004f_d380, &args![high]).u32() == target {
            e.call(0x0057_bd60, &args![high, 0u32]);
        }
        if flag != 0 {
            let package = e.call(0x0093_44a0, &args![high]).u32();
            clear_location_reference(e, package, target);
            let current = e.vcall(process, SLOT_0X22C, &[]).u32();
            clear_location_reference(e, current, target);
        }
        let controller = e.vcall(high, 0x428, &[]).u32();
        if controller != 0 {
            e.call(0x0097_f9c0, &args![controller, target]);
        }
        if e.vcall(process, 0x128, &[]).u32() == target {
            e.vcall(process, 0x12c, &args![0u32]);
        }
        if e.vcall(high, 0x2c8, &[]).u32() == target {
            e.call(0x0088_1620, &args![high, 0u32]);
        }
        end_package_if_target(e, high, process, target);
        e.vcall(process, 0x664, &args![target]);
        release_furniture(e, high, process, target);
        high = next_follower(e, &mut list);
    }
}

/// The clean-up of the second loop of `fn_0096f600` (the temp list) for the
/// high actor `high` and then for its followers; `level_three_tail` is the
/// index the first loop ended with (the detection removal is skipped when it
/// is 2 or more).
fn forget_reference_in_temp_list_actor(
    e: &mut Engine,
    high: u32,
    list: u32,
    target: u32,
    level_three_tail: u32,
) {
    let mut high = high;
    let mut list = list;
    while high != 0 {
        let process = actor_process(e, high);
        if e.call(0x004f_d380, &args![high]).u32() == target {
            e.call(0x0057_bd60, &args![high, 0u32]);
            if process != 0 {
                e.vcall(process, 0x310, &args![0u32]);
                e.vcall(process, 0x4a0, &[]);
            }
        }
        if process == 0 {
            return;
        }
        if level_three_tail < 2 {
            e.vcall(process, 0x2c4, &args![target, 3u32]);
        }
        e.vcall(process, 0x47c, &args![target]);
        clear_extra_record_reference(e, high, target);
        if e.call(0x008a_6650, &args![high, 0u32]).bool() {
            forget_flee_target(e, high, target);
        }
        if is_actor(e, target) && actor_process(e, high) != 0 {
            let marker = second_base_of(target);
            let current = actor_process(e, high);
            if e.vcall(current, 0x3d0, &[]).u32() == marker {
                let current = actor_process(e, high);
                e.vcall(current, 0x3d4, &args![0u32]);
            }
        }
        if e.call(PROCESS_LEVEL, &args![process]).u32() == 0 {
            e.vcall(process, 0xac, &[]);
        }
        if e.call(0x004f_d380, &args![high]).u32() == target {
            e.call(0x0057_bd60, &args![high, 0u32]);
        }
        let package = e.call(0x0093_44a0, &args![high]).u32();
        clear_location_reference(e, package, target);
        let current = e.vcall(process, SLOT_0X22C, &[]).u32();
        clear_location_reference(e, current, target);
        let controller = e.vcall(high, 0x428, &[]).u32();
        if controller != 0 {
            e.call(0x0097_f9c0, &args![controller, target]);
        }
        if e.vcall(process, 0x128, &[]).u32() == target {
            e.vcall(process, 0x12c, &args![0u32]);
        }
        if e.vcall(high, 0x2c8, &[]).u32() == target {
            e.call(0x0088_1620, &args![high, 0u32]);
        }
        end_package_if_target(e, high, process, target);
        e.vcall(process, 0x664, &args![target]);
        release_furniture(e, high, process, target);
        high = next_follower(e, &mut list);
    }
}

// --- 00970a20 .. 00970f60: finding actors among the high-level ones ---------

// Translated from 00970a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::GetActorRefInHigh` (Xbox PDB): the last actor of the process
/// array (from index 0 to the tail of level 0) that is not skipped by
/// `00576d30` and whose form (`007af430` on it, when slot 0x218 says it has
/// one) is `base`, or else whose extra data's `LevCreaOriginalBase`
/// (`004216f0`) is `base`. Once an actor is found the search no longer
/// replaces it. The function takes one more stack word it never reads.
pub fn processlists_get_actor_ref_in_high(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    base: u32,
    _unused_1: u32,
) -> u32 {
    let array = mob_process_array(this);
    let mut found = 0u32;
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        if object != 0 && found == 0 && !e.call(0x0057_6d30, &args![object]).bool() {
            let mut actor = 0u32;
            if is_actor(e, object) {
                actor = object;
            }
            if actor != 0 {
                let mut form = 0u32;
                if e.vcall(actor, 0x218, &[]).bool() {
                    form = e.call(0x007a_f430, &args![actor]).u32();
                }
                if form != 0 && base == form {
                    found = actor;
                } else {
                    let extra = extra_list(e, actor);
                    if e.call(0x0042_16f0, &args![extra]).u32() == base {
                        found = actor;
                    }
                }
            }
        }
        index = index.wrapping_add(1);
    }
    found
}

// Translated from 00970b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Picks an actor of the process array (level 0) that does not answer slot
/// 0x22c (with 0) or `00437bd0` and whose rank in the faction `faction`
/// (`004181e0(actor)` gives the actor's base data,
/// `0047d680(base + 0x30, faction, actor == player)` the rank) is not negative. With `need_alert` the
/// actor's process must also answer slot 0x2d4 (`player, 0`) with a positive
/// value. Without `closest` the search goes to the end and the last such
/// actor is the result; with `closest` it ends at the first such actor, and
/// the one nearer to the player than the best so far (`005723b0`, starting
/// at 10000.0) is the result. Actors whose slot 0x21c answers true are
/// remembered separately and are used only when no other actor was found.
pub fn fn_00970b30(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    faction: u32,
    need_alert: u8,
    closest: u8,
) -> u32 {
    let array = mob_process_array(this);
    let mut result = 0u32;
    let mut nearest = 0u32;
    let mut best_distance: f32 = e.global(0x0102_2958);
    let mut first_flagged = 0u32;
    let mut nearest_flagged = 0u32;
    let mut index = 0u32;
    loop {
        if (result != 0 && closest != 0) || index >= array_tail(e, array, 0) {
            if result == 0 && first_flagged != 0 {
                result = first_flagged;
                if closest != 0 {
                    nearest = nearest_flagged;
                    result = nearest;
                }
            } else if closest != 0 && nearest != 0 {
                result = nearest;
            }
            return result;
        }
        let object = array_object(e, array, index);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor != 0
            && !e.vcall(actor, SLOT_0X22C, &args![0u32]).bool()
            && !e.call(0x0043_7bd0, &args![actor]).bool()
        {
            let is_player = (actor == player(e)) as u32;
            let data = e.call(0x0041_81e0, &args![actor]).u32();
            let rank = e
                .call(
                    0x0047_d680,
                    &args![data.wrapping_add(0x30), faction, is_player],
                )
                .i32();
            let mut accepted = rank > -1;
            if accepted && need_alert != 0 {
                let process = actor_process(e, actor);
                let player = player(e);
                accepted = e.vcall(process, 0x2d4, &args![player, 0u32]).i32() > 0;
            }
            if accepted {
                result = actor;
                if closest != 0 {
                    let player = player(e);
                    let distance = e.call(0x0057_23b0, &args![actor, player, 0u32, 0u32]).f32();
                    if best_distance > distance {
                        nearest = actor;
                        if e.vcall(nearest, 0x21c, &[]).bool() {
                            nearest_flagged = nearest;
                            nearest = 0;
                        }
                        best_distance = distance;
                    }
                }
                if e.vcall(actor, 0x21c, &[]).bool() {
                    first_flagged = result;
                    result = 0;
                }
            }
        }
        index = index.wrapping_add(1);
    }
}

// Translated from 00970d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Publishes the calendar's hour, year, month and day in the globals the AI
/// reads, then goes through every actor of process level 3: sets up its
/// action list (`005ac190(actor, extra data list)`). An actor that is not
/// `00440d80`, has a form (`007af430`) and whose embedded object at +0xA4
/// answers slot 8 (argument 0x10) with a positive value gets its package
/// locations initialised (`00893340` with 0). Otherwise (if it is not
/// `00440d80`, has a form and the answer is not positive, its life state is
/// set with `008a1800(actor, 2)`, and) an actor that still has a process
/// (`008d8520`) is removed from the array at level 3
/// (`ProcessArray::RemoveActor`).
pub fn fn_00970d50(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let hour = e.call(0x0086_7da0, &args![CALENDAR]).f32();
    e.set_global(GAME_HOUR, hour);
    let year = e.call(0x0086_7c60, &args![CALENDAR]).i32();
    e.set_global(GAME_YEAR, year);
    let month = e.call(0x0086_7d20, &args![CALENDAR]).i32();
    e.set_global(GAME_MONTH, month);
    let day = e.call(0x0086_7d60, &args![CALENDAR]).u8();
    e.set_global(GAME_DAY, day);
    let mut index = array_head(e, array, 3);
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor != 0 {
            let extra = extra_list(e, actor);
            e.call(0x005a_c190, &args![actor, extra]);
            if !e.call(0x0044_0d80, &args![actor]).bool()
                && e.call(0x007a_f430, &args![actor]).u32() != 0
                && e.vcall(actor + 0xa4, 8, &args![0x10u32]).i32() > 0
            {
                e.call(0x0089_3340, &args![actor, 0u32]);
            } else {
                if !e.call(0x0044_0d80, &args![actor]).bool()
                    && e.call(0x007a_f430, &args![actor]).u32() != 0
                    && e.vcall(actor + 0xa4, 8, &args![0x10u32]).i32() <= 0
                {
                    e.call(0x008a_1800, &args![actor, 2u32]);
                }
                if actor_process(e, actor) != 0 {
                    e.call(ARRAY_REMOVE_ACTOR, &args![array, object, 3u32]);
                }
            }
        }
        index = index.wrapping_add(1);
    }
}

// Translated from 00970ee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::DistanceListCompareFn` (Xbox PDB): compares the distances
/// (`005723b0` against the player) of two references for the sort of an actor
/// list: -1 when `first` is nearer, 1 when it is farther, else 0.
pub fn processlists_distance_list_compare_fn(e: &mut Engine, first: u32, second: u32) -> i32 {
    let player_ref = player(e);
    let first_distance = e
        .call(0x0057_23b0, &args![first, player_ref, 0u32, 0u32])
        .f32();
    let player_ref = player(e);
    let second_distance = e
        .call(0x0057_23b0, &args![second, player_ref, 0u32, 0u32])
        .f32();
    if second_distance > first_distance {
        -1
    } else if second_distance < first_distance {
        1
    } else {
        0
    }
}

// Translated from 00970f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Builds the list of the level-0 actors that can detect the actor behind
/// `reference` (`0084e3a0`), sorted by distance to the player. Returns 0 when
/// that actor is `00440da0`, `00440d80` or answers slot 0x22c (with 1).
/// First, when the actor's process (slot 0x5c0) reports a positive count, the
/// count is replaced by the number of entries of the list
/// `00971c30(actor, 0x15, 0)` (0 when there is none) and handed to the
/// process (slot 0x348). Then every level-0 actor that is not `00437bd0`, does
/// not answer slot 0x234, is not the target, does not answer slot 0x22c (with 0)
/// and is not `00440da0`, and for which
/// `Actor::GetDetectionLevelAgainstActor(target)` (`008a0d10`) is positive, is
/// pushed on the list, which is created on the first hit. The list is sorted
/// with `0083fd60` and `DistanceListCompareFn`. The unwind frame is not
/// translated.
pub fn fn_00970f60(e: &mut Engine, this: Ptr<ProcessLists>, reference: u32) -> u32 {
    let array = mob_process_array(this);
    let actor = e.call(0x0084_e3a0, &args![reference]).u32();
    if actor != 0
        && (e.call(0x0044_0da0, &args![actor]).bool()
            || e.call(0x0044_0d80, &args![actor]).bool()
            || e.vcall(actor, SLOT_0X22C, &args![1u32]).bool())
    {
        return 0;
    }
    let mut count = 0i32;
    if actor != 0 {
        let process = actor_process(e, actor);
        count = e.vcall(process, 0x5c0, &[]).i32();
    }
    if count > 0 {
        let source = e
            .call(0x0097_1c30, &args![this, actor, 0x15u32, 0u32])
            .u32();
        count = if source != 0 {
            e.call(0x005a_e380, &args![source]).i32()
        } else {
            0
        };
        let process = actor_process(e, actor);
        e.vcall(process, 0x348, &args![count]);
    }
    let mut list = 0u32;
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        let mut candidate = 0u32;
        if object != 0 && is_actor(e, object) {
            candidate = object;
        }
        if candidate != 0
            && !e.call(0x0043_7bd0, &args![candidate]).bool()
            && !e.vcall(candidate, 0x234, &[]).bool()
            && candidate != actor
            && !e.vcall(candidate, SLOT_0X22C, &args![0u32]).bool()
            && !e.call(0x0044_0da0, &args![candidate]).bool()
        {
            let level = e.with_stack(4, |e, flag| {
                e.call(
                    0x008a_0d10,
                    &args![candidate, 0u32, actor, flag.addr(), 0u32, 0u32, 0u32, 0u32],
                )
                .i32()
            });
            if level > 0 {
                if list == 0 {
                    list = list_new(e);
                }
                list_push_value(e, list, candidate);
            }
        }
        index = index.wrapping_add(1);
    }
    if list != 0 {
        e.call(0x0083_fd60, &args![list, 0x0097_0ee0u32]);
    }
    list
}

// --- 009711e0 .. 00971c30: actors that react to an alarm or a crime ---------

/// The integer setting (`011cdbec`) that bounds how far an alarm is heard.
const ALARM_DISTANCE_SETTING: u32 = 0x011c_dbec;
/// Whether `actor` is within the alarm distance of `reference`: `005723b0`
/// (the distance between two references) is not above the integer setting.
fn within_alarm_distance(e: &mut Engine, actor: u32, reference: u32) -> bool {
    let distance = e
        .call(0x0057_23b0, &args![actor, reference, 0u32, 0u32])
        .f32();
    let setting = e
        .call(SETTING_INT_VALUE, &args![ALARM_DISTANCE_SETTING])
        .u32();
    let limit = e.mem.u32(setting) as i32;
    (distance as f64) <= limit as f64
}

/// Whether an actor in `cell` and world space `space` hears an alarm raised in
/// `target_cell` and `target_space` without a go-between: the case where the
/// tests of both alarm functions route to the distance check. `strict` is the
/// variant of `fn_009715c0` (a shared cell counts only when it also satisfies
/// `00425fd0`) against that of `SendActorsYellAlarm` (any shared cell counts).
fn hears_directly(
    e: &mut Engine,
    strict: bool,
    cell: u32,
    space: u32,
    target_cell: u32,
    target_space: u32,
) -> bool {
    let cell_is_interior = |e: &mut Engine, cell: u32| e.call(0x0042_5fd0, &args![cell]).bool();
    if strict {
        if cell != 0 && cell_is_interior(e, cell) && cell == target_cell {
            return true;
        }
    } else if cell != 0 && cell == target_cell {
        return true;
    }
    if target_space != space {
        return false;
    }
    if cell != 0 && !cell_is_interior(e, cell) {
        return true;
    }
    !(target_cell == 0 || cell_is_interior(e, target_cell))
}

/// The go-between search of both alarm functions: walks the nodes of the list
/// `nodes` (`006815c0` gives the address of a node's item, `00726070` the
/// next node) and returns true at the first reference whose location
/// (`00568e50`, then `00559450`) is in the cell `cell`, or in no cell but in
/// the world space of `actor`, and within the alarm distance of `actor`.
fn go_between_hears(e: &mut Engine, nodes: u32, actor: u32, cell: u32) -> bool {
    let mut node = nodes;
    loop {
        if node == 0 {
            return false;
        }
        let item = node_item(e, node);
        if e.mem.u32(item) == 0 {
            return false;
        }
        let item = node_item(e, node);
        let reference = e.mem.u32(item);
        if reference != 0 {
            let holder = e.call(0x0056_8e50, &args![reference]).u32();
            if holder != 0 {
                let location = e.call(0x0055_9450, &args![holder]).u32();
                let location_cell = e.call(0x008d_6f30, &args![location]).u32();
                let mut near = location_cell == cell;
                if !near && e.call(0x008d_6f30, &args![location]).u32() == 0 {
                    near = e.call(0x0057_5d70, &args![location]).u32()
                        == e.call(0x0057_5d70, &args![actor]).u32();
                }
                if near && within_alarm_distance(e, actor, location) {
                    return true;
                }
            }
        }
        node = node_next(e, node);
    }
}

// Translated from 009711e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::SendActorsYellAlarm` (Xbox PDB): builds the list of the level-3
/// actors that hear the alarm of the actor behind `reference` (`0084e3a0`),
/// sorted by distance to the player (`DistanceListCompareFn`). `nodes` is a
/// list of references that can carry the alarm. Returns the (empty,
/// allocated) list when there is no such actor, unsorted. The actors that
/// answer slot 0x22c (with 0), `00440da0` or `00440d80` are left out, as
/// are those whose alarm package (type 0x15 package of `009344a0`) already
/// has the crime (`AlarmPackage::IsCrimeInList`, `009ec810`). An actor that
/// shares the cell of the target, or is in the same world space with
/// neither cell blocking, is added when it is within the alarm distance of
/// the target; otherwise the go-between list decides. First calls `00632d20`
/// on `reference` with 1. The followers loop that the compiled code has
/// (a list node that starts as 0) never runs and is left out; the unwind
/// frame is not translated.
pub fn processlists_send_actors_yell_alarm(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    reference: u32,
    nodes: u32,
) -> u32 {
    let array = mob_process_array(this);
    let list = list_new(e);
    let target = e.call(0x0084_e3a0, &args![reference]).u32();
    let target_cell = e.call(0x008d_6f30, &args![target]).u32();
    let target_space = e.call(0x0057_5d70, &args![target]).u32();
    e.call(0x0063_2d20, &args![reference, 1u32]);
    if target == 0 {
        return list;
    }
    let mut index = 0u32;
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        index = index.wrapping_add(1);
        if actor == 0
            || e.vcall(actor, SLOT_0X22C, &args![0u32]).bool()
            || e.call(0x0044_0da0, &args![actor]).bool()
            || e.call(0x0044_0d80, &args![actor]).bool()
            || e.vcall(actor, SLOT_0X22C, &args![0u32]).bool()
        {
            continue;
        }
        let cell = e.call(0x008d_6f30, &args![actor]).u32();
        let space = e.call(0x0057_5d70, &args![actor]).u32();
        let package = e.call(0x0093_44a0, &args![actor]).u32();
        let mut alarm = 0u32;
        if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x15 {
            alarm = package;
        }
        if alarm != 0 && e.call(0x009e_c810, &args![alarm, reference]).bool() {
            continue;
        }
        let direct = hears_directly(e, false, cell, space, target_cell, target_space);
        if direct {
            if within_alarm_distance(e, actor, target) {
                list_push_value(e, list, actor);
            }
        } else if go_between_hears(e, nodes, actor, cell) {
            list_push_value(e, list, actor);
        }
    }
    e.call(0x0083_fd60, &args![list, 0x0097_0ee0u32]);
    list
}

// Translated from 009715c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like `SendActorsYellAlarm`, for the actors of level 3 that answer slot
/// 0x304 and are not `00493bb0`: lists those that hear `target` (a reference,
/// not an actor to resolve) and tells each one's process slot 0x33c with
/// `actor, target, 0 (nine times), 1, 0`. Before that, the player's process
/// (slot 0x5c0) gives a count and the process is told slot 0x5c4 with
/// `1 - count`. The listener test is the same, with the cell of the listener
/// needing `00425fd0` for the shared-cell case. Returns the list (unsorted),
/// which is empty when `target` is 0. The followers loop that never runs is
/// left out; the unwind frame is not translated.
pub fn fn_009715c0(e: &mut Engine, this: Ptr<ProcessLists>, target: u32, nodes: u32) -> u32 {
    let array = mob_process_array(this);
    let list = list_new(e);
    let target_cell = e.call(0x008d_6f30, &args![target]).u32();
    let target_space = e.call(0x0057_5d70, &args![target]).u32();
    if target == 0 {
        return list;
    }
    let player_ref = player(e);
    let process = actor_process(e, player_ref);
    let count = e.vcall(process, 0x5c0, &[]).i32();
    let process = actor_process(e, player_ref);
    e.vcall(process, 0x5c4, &args![count.wrapping_sub(1).wrapping_neg()]);
    let mut index = 0u32;
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        if object == 0 || !is_actor(e, object) {
            continue;
        }
        let actor = object;
        let cell = e.call(0x008d_6f30, &args![actor]).u32();
        let space = e.call(0x0057_5d70, &args![actor]).u32();
        if e.call(0x0049_3bb0, &args![actor]).bool() || !e.vcall(actor, 0x304, &[]).bool() {
            continue;
        }
        let hears = hears_directly(e, true, cell, space, target_cell, target_space);
        let hit = if hears {
            within_alarm_distance(e, actor, target)
        } else {
            go_between_hears(e, nodes, actor, cell)
        };
        if hit {
            let process = actor_process(e, actor);
            e.vcall(
                process,
                0x33c,
                &args![
                    actor, target, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 1u32, 0u32
                ],
            );
            list_push_value(e, list, actor);
        }
    }
    list
}

// Translated from 009719e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::GetActorsProcessCrimeAlarm` (Xbox PDB): for each actor of
/// level 3 whose current package (`009344a0`) is an alarm package (type
/// 0x15) that has the crime `crime` in its list (`AlarmPackage::IsCrimeInList`,
/// `009ec810`), pushes the actor on a list (created on the first one), or,
/// with `remove`, takes the crime out of the package (`AlarmPackage::RemoveCrime`,
/// `009ec850(package, crime, actor)`). With `remove` the list is released
/// afterwards (scalar deleting destructor `004702f0`) and 0 is returned.
pub fn processlists_get_actors_process_crime_alarm(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    crime: u32,
    remove: u8,
) -> u32 {
    let array = mob_process_array(this);
    let mut list = 0u32;
    let mut index = 0u32;
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        if object == 0 || !is_actor(e, object) {
            continue;
        }
        let actor = object;
        let package = e.call(0x0093_44a0, &args![actor]).u32();
        let mut alarm = 0u32;
        if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x15 {
            alarm = package;
        }
        if alarm != 0 && e.call(0x009e_c810, &args![alarm, crime]).bool() {
            if list == 0 {
                list = list_new(e);
            }
            if remove != 0 {
                e.call(0x009e_c850, &args![alarm, crime, actor]);
            } else {
                list_push_value(e, list, actor);
            }
        }
    }
    if remove != 0 {
        list_delete(e, list);
        list = 0;
    }
    list
}

// Translated from 00971ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::TurnOffMovementinHigh` (Xbox PDB): for each object of level
/// 0 tells slot 0x204 with 1, then `Actor::PickAnimations(1.0, 1.0)`
/// (`00895110`).
pub fn processlists_turn_off_movement_in_high(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        if object != 0 {
            e.vcall(object, 0x204, &args![1u32]);
            e.call(0x0089_5110, &args![object, 1.0f32, 1.0f32]);
        }
        index = index.wrapping_add(1);
    }
}

/// Puts `actor` on `list`, or, for an actor of the middle levels that is
/// flagged (slot 0x21c) or does not persist (`005653d0`), tells it slot 0x434
/// with 0 instead (the shared tail of `fn_00971c30`).
fn push_or_release(e: &mut Engine, list: u32, actor: u32, middle_level: bool) {
    if middle_level
        && (e.vcall(actor, 0x21c, &[]).bool() || !e.call(0x0056_53d0, &args![actor]).bool())
    {
        e.vcall(actor, 0x434, &args![0u32]);
    } else {
        list_push_value(e, list, actor);
    }
}

// Translated from 00971c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Lists the level-3 actors (only those below the tail of level 0 with
/// `high_only`) that work on a package of type `kind` for `owner`. Actors
/// that are `00493bb0` and a `kind` of 0x12 (combat) are judged by their
/// combat package (`00881510`, type 0x12): with `high_only` by
/// `004030b0(package) == owner`, otherwise by `CombatController::
/// IsActoraCombatTarget` (`0097fa10(package, owner)`); in the middle level
/// (between the head and tail of level 2) such actors are not listed but told
/// slot 0x434 (with 0) when they are flagged (slot 0x21c) or do not persist
/// (`005653d0`). All other actors are listed when their current package
/// (`009344a0`) has type `kind`, their process's slot 0x128 is `owner`, and
/// they answer slot 0x304 or `kind` is not 0x15. The list is created with the
/// first hit and returned, 0 when there is none. The unwind frame is not
/// translated.
pub fn fn_00971c30(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    owner: u32,
    kind: u32,
    high_only: u8,
) -> u32 {
    let array = mob_process_array(this);
    let mut list = 0u32;
    let mut index = 0u32;
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        let current = index;
        index = index.wrapping_add(1);
        if object == 0 || (high_only != 0 && current >= array_tail(e, array, 0)) {
            continue;
        }
        let middle_level = current >= array_head(e, array, 2) && current < array_tail(e, array, 2);
        if !is_actor(e, object) {
            continue;
        }
        let actor = object;
        if e.call(0x0049_3bb0, &args![actor]).bool() && kind == 0x12 {
            let package = e.call(0x0088_1510, &args![actor]).u32();
            let mut combat = 0u32;
            if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0x12 {
                combat = package;
            }
            let listed = if high_only != 0 && combat != 0 {
                e.call(0x0040_30b0, &args![combat]).u32() == owner
            } else {
                combat != 0 && e.call(0x0097_fa10, &args![combat, owner]).bool()
            };
            if listed {
                if list == 0 {
                    list = list_new(e);
                }
                push_or_release(e, list, actor, middle_level);
            }
        } else {
            let package = e.call(0x0093_44a0, &args![actor]).u32();
            if package == 0 {
                continue;
            }
            if !e.vcall(actor, 0x304, &[]).bool() && kind == 0x15 {
                continue;
            }
            let process = actor_process(e, actor);
            if e.vcall(process, 0x128, &[]).u32() != owner {
                continue;
            }
            if e.call(PACKAGE_TYPE, &args![package]).u32() != kind {
                continue;
            }
            if list == 0 {
                list = list_new(e);
            }
            list_push_value(e, list, actor);
        }
    }
    list
}

// --- 00971fd0 .. 009727c0: the lists of crimes ------------------------------
//
// `GlobalCrimeListArray` (+0x44, five `BSSimpleList<Crime *> *`) is indexed by
// the crime's kind, which is the dword at +4 of the crime (`00726070`: the same
// body as `BSSimpleList::Next`, folded by the linker). Further fields of a
// crime used here: +0x34 the time it happened (`008bcc60` writes it to an
// out parameter), +0x38 the list of actors who know it (`009e32d0`).

/// `ProcessLists::GlobalCrimeListArray[kind]` (Xbox PDB): the list at the
/// array slot `kind`, with no range check.
fn crime_list(e: &Engine, this: Ptr<ProcessLists>, kind: i32) -> u32 {
    e.mem
        .u32(this.addr() + GLOBAL_CRIME_LIST_ARRAY + 4 * kind as u32)
}

/// The kind of a crime (the dword at +4; `00726070`).
fn crime_kind(e: &mut Engine, crime: u32) -> i32 {
    e.call(NODE_NEXT, &args![crime]).i32()
}

/// The list of actors that know a crime (`009e32d0`, the dword at +0x38).
fn crime_known_by(e: &mut Engine, crime: u32) -> u32 {
    e.call(0x009e_32d0, &args![crime]).u32()
}

/// Whether a list node is empty: no item and no next node (`008256d0`).
fn list_is_empty(e: &mut Engine, list: u32) -> bool {
    e.call(LIST_IS_EMPTY, &args![list]).bool()
}

// Translated from 00971fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Takes out of the crime list `GlobalCrimeListArray[3]` (+0x50) every crime
/// whose elapsed time is not greater than `age` and returns them in a new list (0 when there
/// are none). The crime's age is `00435e00` on the time `008bcc60` reads from
/// the crime (the value `00435dd0` returns minus that time); a crime for
/// which `age < elapsed` stays. For each crime that goes: every actor that
/// knows it (`009e32d0`) and for which `009722e0` finds no other crime is told
/// `0047eb90(0)`; the crime leaves the player crime list (`0041cf00` of the
/// extra data of its victim, `0044ddc0`, when the victim is an actor) and
/// the list it was in, and goes on the result. The result list is released
/// at the end when it is empty. The unwind frame is not translated.
pub fn fn_00971fd0(e: &mut Engine, this: Ptr<ProcessLists>, age: f32) -> u32 {
    let head = this.addr() + GLOBAL_CRIME_LIST_ARRAY + 0xc;
    let mut node = e.mem.u32(head);
    let result = list_new(e);
    let mut previous = 0u32;
    while node != 0 && !list_is_empty(e, node) {
        let item = node_item(e, node);
        let crime = e.mem.u32(item);
        let elapsed = e.with_stack(4, |e, out| {
            e.call(0x008b_cc60, &args![crime, out.addr()]);
            e.call(0x0043_5e00, &args![out.addr()]).f32()
        });
        if (age as f64) < (elapsed as f64) {
            previous = node;
            node = node_next(e, node);
            continue;
        }
        let mut known = crime_known_by(e, crime);
        while known != 0 && !list_is_empty(e, known) {
            let item = node_item(e, known);
            let actor = e.mem.u32(item);
            if !e
                .call(0x0097_22e0, &args![PROCESS_LISTS, actor, crime])
                .bool()
            {
                e.call(0x0047_eb90, &args![actor, 0u32]);
            }
            known = node_next(e, known);
        }
        let victim = e.call(0x0044_ddc0, &args![crime]).u32();
        if victim != 0 && is_actor(e, victim) {
            let extra = extra_list(e, victim);
            let player_crimes = e.call(0x0041_cf00, &args![extra]).u32();
            if player_crimes != 0 {
                list_remove_value(e, player_crimes, crime);
            }
        }
        list_remove_value(e, node, crime);
        node = previous;
        if node == 0 {
            node = e.mem.u32(head);
        }
        list_push_value(e, result, crime);
    }
    if list_is_empty(e, result) {
        list_delete(e, result);
        return 0;
    }
    result
}

// Translated from 009721f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Files `crime` under its kind: when the kind is 0 to 4, creates the
/// kind's list in `GlobalCrimeListArray` if it is missing, and adds the crime
/// to it unless it is in there already (`005f65d0`).
pub fn fn_009721f0(e: &mut Engine, this: Ptr<ProcessLists>, crime: u32) {
    if crime == 0 {
        return;
    }
    let kind = crime_kind(e, crime);
    if kind > -1 && crime_kind(e, crime) < 5 {
        let kind = crime_kind(e, crime);
        let mut list = crime_list(e, this, kind);
        if list == 0 {
            list = list_new(e);
        }
        let present = e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), crime);
            e.call(LIST_CONTAINS, &args![list, slot.addr()]).bool()
        });
        if !present {
            list_push_value(e, list, crime);
        }
        let kind = crime_kind(e, crime);
        e.mem.set_u32(
            this.addr() + GLOBAL_CRIME_LIST_ARRAY + 4 * kind as u32,
            list,
        );
    }
}

// Translated from 009722e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether another crime than `crime` in the list `GlobalCrimeListArray[3]`
/// (+0x50) is known by the actor `actor` (its known-by list, `009e32d0`,
/// contains it). The list at +0x54 is searched the same way afterwards, but
/// its result is not used: the return value is the first answer only.
pub fn fn_009722e0(e: &mut Engine, this: Ptr<ProcessLists>, actor: u32, crime: u32) -> bool {
    let known_by = |e: &mut Engine, node: u32| -> bool {
        let item = node_item(e, node);
        let other = e.mem.u32(item);
        if other == crime {
            return false;
        }
        let known = crime_known_by(e, other);
        if known == 0 {
            return false;
        }
        let known = crime_known_by(e, other);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), actor);
            e.call(LIST_CONTAINS, &args![known, slot.addr()]).bool()
        })
    };
    let mut node = e.mem.u32(this.addr() + 0x50);
    let mut found = false;
    while node != 0 && !list_is_empty(e, node) && !found {
        if known_by(e, node) {
            found = true;
        }
        node = node_next(e, node);
    }
    if !found {
        let mut node = e.mem.u32(this.addr() + 0x54);
        let mut second = false;
        while node != 0 && !list_is_empty(e, node) && !second {
            if known_by(e, node) {
                second = true;
            }
            node = node_next(e, node);
        }
    }
    found
}

// Translated from 009723f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Goes through the five crime lists and drops every crime
/// `Crime::ShouldCrimeBeRemoved` (`009eb8a0`) wants gone: its actors forget
/// it (`Crime::ClearActorKnowList`, `009eba30`), it leaves the list, and it is
/// destroyed (`008f25e0` with 1). Then sets `fCrimeUpdateTimer` (+0x103B4) to
/// `iNumberHighActors` (+0x103B0) plus 10.0.
pub fn fn_009723f0(e: &mut Engine, this: Ptr<ProcessLists>) {
    for kind in 0..5 {
        let head = crime_list(e, this, kind);
        let mut node = head;
        while node != 0 {
            let item = node_item(e, node);
            if e.mem.u32(item) == 0 {
                break;
            }
            let item = node_item(e, node);
            let crime = e.mem.u32(item);
            if e.call(0x009e_b8a0, &args![crime]).bool() {
                e.call(0x009e_ba30, &args![crime]);
                let crime = e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), crime);
                    e.call(LIST_REMOVE, &args![head, slot.addr()]);
                    e.mem.u32(slot.addr())
                });
                if crime != 0 {
                    e.call(0x008f_25e0, &args![crime, 1u32]);
                }
                node = head;
            } else {
                node = node_next(e, node);
            }
        }
    }
    let count = e.get(this, ProcessLists::iNumberHighActors) as i32;
    let ten: f64 = e.global(0x0102_0758);
    e.set(
        this,
        ProcessLists::fCrimeUpdateTimer,
        (count as f64 + ten) as f32,
    );
}

// Translated from 009724e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The first crime of the list `GlobalCrimeListArray[kind]` (kind 0 to 5; the
/// sixth slot is the list at +0x58) whose dword at +0x28 (`0045cd60`) is
/// `value`, or 0.
pub fn fn_009724e0(e: &mut Engine, this: Ptr<ProcessLists>, kind: i32, value: u32) -> u32 {
    if !(0..=5).contains(&kind) {
        return 0;
    }
    let mut node = crime_list(e, this, kind);
    let mut found = 0u32;
    while node != 0 && found == 0 {
        let item = node_item(e, node);
        if e.mem.u32(item) == 0 {
            break;
        }
        let item = node_item(e, node);
        let crime = e.mem.u32(item);
        node = node_next(e, node);
        if e.call(PROCESS_LEVEL, &args![crime]).u32() == value {
            found = crime;
        }
    }
    found
}

// Translated from 00972570 (decompiled, FalloutNV.exe 1.4.0.525)
/// The first crime of the list `GlobalCrimeListArray[kind]` (kind 0 to 5)
/// whose first actor (`0084e3a0`) is `actor` and whose victim (`0044ddc0`) is
/// `victim`, or 0.
pub fn fn_00972570(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    actor: u32,
    victim: u32,
    kind: i32,
) -> u32 {
    if !(0..=5).contains(&kind) {
        return 0;
    }
    let mut node = crime_list(e, this, kind);
    let mut found = 0u32;
    while node != 0 && found == 0 {
        let item = node_item(e, node);
        if e.mem.u32(item) == 0 {
            break;
        }
        let item = node_item(e, node);
        let crime = e.mem.u32(item);
        node = node_next(e, node);
        if e.call(0x0084_e3a0, &args![crime]).u32() == actor
            && e.call(0x0044_ddc0, &args![crime]).u32() == victim
        {
            found = crime;
        }
    }
    found
}

// Translated from 00972600 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the list `GlobalCrimeListArray[kind]` (kind 0 to 5) holds a crime
/// of that `kind` (any kind is accepted when it is -1, which the range check
/// of the function makes unreachable) whose first actor (`0084e3a0`) is
/// `actor`, whose victim (`0044ddc0`) is `victim` (or `victim` is 0), whose
/// dword at +0x28 (`0045cd60`) is `value` (or `value` is -1) and for which
/// `009eb9a0(crime, extra)` holds (or `extra` is 0). The word at +0x18 of
/// the arguments is not read.
#[allow(clippy::too_many_arguments)]
pub fn fn_00972600(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    victim: u32,
    actor: u32,
    extra: u32,
    kind: i32,
    _unused_5: u32,
    value: i32,
) -> bool {
    if !(0..=5).contains(&kind) {
        return false;
    }
    let mut node = crime_list(e, this, kind);
    while node != 0 {
        let item = node_item(e, node);
        if e.mem.u32(item) == 0 {
            break;
        }
        let item = node_item(e, node);
        let crime = e.mem.u32(item);
        node = node_next(e, node);
        if (crime_kind(e, crime) == kind || kind == -1)
            && e.call(0x0084_e3a0, &args![crime]).u32() == actor
            && (e.call(0x0044_ddc0, &args![crime]).u32() == victim || victim == 0)
            && (value == -1 || e.call(PROCESS_LEVEL, &args![crime]).i32() == value)
            && (extra == 0 || e.call(0x009e_b9a0, &args![crime, extra]).bool())
        {
            return true;
        }
    }
    false
}

// Translated from 009726e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Takes `crime` out of the list of its kind in `GlobalCrimeListArray`
/// (kind 0 to 5), if there is such a list.
pub fn fn_009726e0(e: &mut Engine, this: Ptr<ProcessLists>, crime: u32) {
    if crime == 0 {
        return;
    }
    let kind = crime_kind(e, crime);
    if kind > -1 && crime_kind(e, crime) <= 5 {
        let kind = crime_kind(e, crime);
        let list = crime_list(e, this, kind);
        if list != 0 {
            list_remove_value(e, list, crime);
        }
    }
}

// Translated from 00972740 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::GetCrimeIndex` (Xbox PDB): the position of `crime` in the list
/// `GlobalCrimeListArray[kind]` (no range check). When it is not there,
/// logs the message at `0x0108c168` through `005b5e40` and returns 0.
pub fn processlists_get_crime_index(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    kind: u32,
    crime: u32,
) -> u16 {
    let mut index = 0u16;
    let mut node = e.mem.u32(this.addr() + GLOBAL_CRIME_LIST_ARRAY + 4 * kind);
    while node != 0 && !list_is_empty(e, node) {
        let item = node_item(e, node);
        if crime == e.mem.u32(item) {
            return index;
        }
        index = index.wrapping_add(1);
        node = node_next(e, node);
    }
    e.call(0x005b_5e40, &args![0x0108_c168u32]);
    0
}

// Translated from 009727c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::GetCrimeByIndex` (Xbox PDB): the crime at position `index` of
/// the list `GlobalCrimeListArray[kind]` (no range check). When the list is
/// shorter, logs the message at `0x0108c1c0` through `005b5e40` and returns 0.
pub fn processlists_get_crime_by_index(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    kind: u32,
    index: u16,
) -> u32 {
    let mut position = 0u16;
    let mut node = e.mem.u32(this.addr() + GLOBAL_CRIME_LIST_ARRAY + 4 * kind);
    while node != 0 && !list_is_empty(e, node) {
        if position == index {
            let item = node_item(e, node);
            return e.mem.u32(item);
        }
        position = position.wrapping_add(1);
        node = node_next(e, node);
    }
    e.call(0x005b_5e40, &args![0x0108_c1c0u32]);
    0
}

// --- 00972840 .. 00972d30: crimes forgotten, actors put to bed ---------------

// Translated from 00972840 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every actor of process levels 0 to 3 (from the head of level 0 to the
/// tail of level 3): if it is in combat with `target`
/// (`Actor::IsInCombatWithActor`, `008bc700`) it is told slot 0x434 with
/// `target`; otherwise, unless `keep_package` is set, an alarmed actor
/// (`Actor::IsAlarmed`, `008a61b0`) whose package target
/// (`00671d10` then `00680020`) is `target` (or `target` is 0) ends its
/// interrupt package (`00881680` with 0) and its process is told slot 0x644
/// with 1. When `target` is the player, each actor also gets `008bcb40`,
/// loses its player crime list extra (`0041cf30`) and, if slot 0x21c is true,
/// every faction-list entry found by `Actor::IntegrateFactionLists`
/// (`008b8ca0`, 128 slots) whose flags have bit 4 (`005a2270`) is told
/// `0047eb90(0)`. `this` is not read.
pub fn fn_00972840(e: &mut Engine, _this: Ptr<ProcessLists>, target: u32, keep_package: u8) {
    let array = e.call(PROCESS_ARRAY_OF, &args![PROCESS_LISTS]).u32();
    let mut index = array_head(e, array, 0);
    while index < array_tail(e, array, 3) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor == 0 {
            continue;
        }
        if e.call(0x008b_c700, &args![actor, target]).bool() {
            e.vcall(actor, 0x434, &args![target]);
        } else if keep_package == 0 && e.call(0x008a_61b0, &args![actor]).bool() {
            let package = e.call(0x0093_44a0, &args![actor]).u32();
            if e.call(0x0067_1d10, &args![package]).u32() != 0 {
                let package = e.call(0x0093_44a0, &args![actor]).u32();
                let holder = e.call(0x0067_1d10, &args![package]).u32();
                let reference = e.call(0x0068_0020, &args![holder]).u32();
                if reference == target || target == 0 {
                    e.call(0x0088_1680, &args![actor, 0u32]);
                    let process = actor_process(e, actor);
                    e.vcall(process, 0x644, &args![1u32]);
                }
            }
        }
        if target == player(e) {
            e.call(0x008b_cb40, &args![actor]);
            let extra = extra_list(e, actor);
            e.call(0x0041_cf30, &args![extra, actor]);
            if e.vcall(actor, 0x21c, &[]).bool() {
                let base = e.call(0x0041_81e0, &args![actor]).u32();
                let factions = e.call(0x005d_8a70, &args![base.wrapping_add(0x30)]).u32();
                let extra = extra_list(e, actor);
                let record = e.call(0x0042_e800, &args![extra]).u32();
                let rank = if record == 0 {
                    0
                } else {
                    e.mem.u32(record + 0xc)
                };
                e.with_stack(0x200, |e, found| {
                    let count = e
                        .call(
                            0x008b_8ca0,
                            &args![actor, found.addr(), 0x80u32, factions, rank],
                        )
                        .u32();
                    for i in 0..count {
                        let entry = e.mem.u32(found.addr() + 4 * i);
                        if entry != 0 && e.call(0x005a_2270, &args![entry]).bool() {
                            e.call(0x0047_eb90, &args![entry, 0u32]);
                        }
                    }
                });
            }
        }
    }
}

// Translated from 00972aa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Takes every crime of `actor` (`0084e3a0`) out of the five crime lists:
/// the actors that know it (`009e32d0`), unless `009722e0` finds another
/// crime for them, are told `0047eb90(0)` first.
pub fn fn_00972aa0(e: &mut Engine, this: Ptr<ProcessLists>, actor: u32) {
    for kind in 0..5 {
        let head = crime_list(e, this, kind);
        let mut node = head;
        while node != 0 {
            let item = node_item(e, node);
            if e.mem.u32(item) == 0 {
                break;
            }
            let item = node_item(e, node);
            let crime = e.mem.u32(item);
            if e.call(0x0084_e3a0, &args![crime]).u32() == actor {
                let mut known = crime_known_by(e, crime);
                while known != 0 && !list_is_empty(e, known) {
                    let item = node_item(e, known);
                    let knower = e.mem.u32(item);
                    if !e
                        .call(0x0097_22e0, &args![PROCESS_LISTS, knower, crime])
                        .bool()
                    {
                        e.call(0x0047_eb90, &args![knower, 0u32]);
                    }
                    known = node_next(e, known);
                }
                list_remove_value(e, head, crime);
                node = head;
            } else {
                node = node_next(e, node);
            }
        }
    }
}

// Translated from 00972bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// For every actor of level 0: an actor that answers slot 0x22c (with 0)
/// and has positive health (`005f0b00` of its base data) and does not answer
/// slot 0x9c has its process told slot 0x4d8 with the actor. Any other actor
/// whose fatigue percentage (`00893530`) is not above 0, whose base data's
/// embedded object at +0x30 answers slot 0x60 with a non-zero word, and that
/// has a process, has the process told slot 0x418 with the actor and the
/// three words of the vector slot 0x1f4 returns for 0.0.
pub fn fn_00972bb0(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor != 0 && e.vcall(actor, SLOT_0X22C, &args![0u32]).bool() {
            let base = e.call(0x0041_81e0, &args![actor]).u32();
            if e.call(0x005f_0b00, &args![base]).i32() > 0 && !e.vcall(actor, 0x9c, &[]).bool() {
                let process = actor_process(e, actor);
                e.vcall(process, 0x4d8, &args![actor]);
            }
        } else if actor != 0 {
            let fatigue = e.call(0x0089_3530, &args![actor]).f64();
            let zero: f64 = e.global(ZERO);
            if fatigue <= zero {
                let base = e.call(0x0041_81e0, &args![actor]).u32();
                let embedded = base.wrapping_add(0x30);
                if e.vcall(embedded, 0x60, &[]).u16() != 0 {
                    let process = actor_process(e, actor);
                    if process != 0 {
                        let vector = e.vcall(actor, 0x1f4, &args![0.0f32]).u32();
                        let (x, y, z) = (
                            e.mem.u32(vector),
                            e.mem.u32(vector + 4),
                            e.mem.u32(vector + 8),
                        );
                        e.vcall(process, 0x418, &args![actor, x, y, z]);
                    }
                }
            }
        }
    }
}

/// `TESObjectREFR::GetFirstFreeMarkerIndex(1)` (Xbox PDB, `005682c0`) of a
/// piece of furniture; -1 when none is free.
fn first_free_marker(e: &mut Engine, furniture: u32) -> i32 {
    e.call(0x0056_82c0, &args![furniture, 1u32]).i32()
}

/// `TESObjectREFR::GetMarkerAtIndex(index, data)` (Xbox PDB, `00568500`):
/// whether the marker can be used with the process's marker data.
fn marker_usable(e: &mut Engine, furniture: u32, index: i32, data: u32) -> bool {
    e.call(0x0056_8500, &args![furniture, index, data]).bool()
}

/// Whether `furniture` can be used by `actor`: it has no owner
/// (`TESObjectREFR::GetOwner`, `00567790`) or the actor is an owner
/// (`TESObjectREFR::IsAnOwner`, `005785e0(actor, 1)`).
fn furniture_allowed(e: &mut Engine, furniture: u32, actor: u32) -> bool {
    e.call(0x0056_7790, &args![furniture]).u32() == 0
        || e.call(0x0057_85e0, &args![furniture, actor, 1u32]).bool()
}

// Translated from 00972d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::PlaceActorsinBedsOrChairs` (Xbox PDB): for every actor of
/// level 0 that is not skipped (slot 0x22c with 0 false, slot 0x218 true,
/// not `004938e0`, not `008a3b30`, slot 0x214 zero, and the level of its
/// process, `0045cd60`, zero) updates its magic (`008c3c40(0, 0)`) and finds
/// a piece of furniture for it: the furniture its package targets
/// (`00881650`, or the current reference of a patrol package, type 0xd,
/// `009f3390`), else the furniture of its package location
/// (`00886080`: `0067f390` its reference, `00569b80` for a location of type 6,
/// else the process's slot 0x514), if it is furniture
/// (`TESObjectREFR::IsFurniture`, `00568680`) in the actor's space
/// (`00575ca0`). Without one: with a sleep package (type 4) whose slot 0x13c
/// (`actor, 0, -1.0, 0`) holds, the first bed (`CanSleepOn`, `00509420`) of the
/// furniture list `0055ac00(TES)` the actor may use and that has a free
/// marker (`005682c0`, `00568500`); with an eat package (type 3) whose slot
/// 0x13c holds and that has a best food item (`00891d30`), the first chair
/// (`CanSitOn`, `005093f0`) likewise. Furniture without a usable marker is
/// removed from the list the search walks. When furniture was found and the
/// actor has a 3D node (slot 0x1d0), `Actor::PutActorInChairBedQuick`
/// (`0088d2f0(furniture, marker data, marker index, sleep)`) places it; when
/// the package's furniture had no usable marker, the actor is moved to the
/// closest navmesh point (`006d6f80`) of its position instead. If an actor was
/// placed (the result of the last placing), `00c3dfa0` is called on the global
/// at `0x01202d98`; finally `00450b60` on `TES`.
pub fn processlists_place_actors_in_beds_or_chairs(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let tes: u32 = e.global(TES);
    let mut node = e.call(0x0055_ac00, &args![tes]).u32();
    let mut placed = false;
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor == 0
            || e.vcall(actor, SLOT_0X22C, &args![0u32]).bool()
            || !e.vcall(actor, 0x218, &[]).bool()
            || e.call(0x0049_38e0, &args![actor]).bool()
            || e.call(0x008a_3b30, &args![actor]).bool()
            || e.vcall(actor, 0x214, &[]).u32() != 0
        {
            continue;
        }
        let process = actor_process(e, actor);
        if e.call(PROCESS_LEVEL, &args![process]).u32() != 0 {
            continue;
        }
        e.call(0x008c_3c40, &args![actor, 0u32, 0u32]);
        let space = e.call(0x0057_5ca0, &args![actor]).u32();
        let mut chosen = 0u32;
        let mut sleep = 0u8;
        let mut no_marker = false;
        let location = e.call(0x0088_6080, &args![actor]).u32();
        let mut target = e.call(0x0088_1650, &args![actor]).u32();
        let package = e.call(0x0093_44a0, &args![actor]).u32();
        if package != 0 && e.call(PACKAGE_TYPE, &args![package]).u32() == 0xd {
            let process = actor_process(e, actor);
            let patrol = e.vcall(process, 0x274, &[]).u32();
            if patrol != 0 && e.vcall(patrol, 8, &[]).u32() == 0xd {
                target = e.call(0x009f_3390, &args![patrol]).u32();
            }
        }
        let mut location_reference = 0u32;
        if location != 0 {
            location_reference = e.call(0x0067_f390, &args![location]).u32();
        }
        if location != 0 && e.call(0x0067_8ca0, &args![location]).u32() == 6 {
            location_reference = e.call(0x0056_9b80, &args![actor]).u32();
        }
        if location_reference == 0 {
            location_reference = if actor_process(e, actor) == 0 {
                0
            } else {
                let process = actor_process(e, actor);
                e.vcall(process, 0x514, &[]).u32()
            };
        }
        let process = actor_process(e, actor);
        let marker_data = e.vcall(process, 0x4d4, &[]).u32();
        let mut marker_index: i32 = -1;

        let target_usable = target != 0
            && e.call(0x0056_8680, &args![target]).bool()
            && e.call(0x0057_5ca0, &args![target]).u32() == space;
        let location_usable = !target_usable
            && location_reference != 0
            && e.call(0x0056_8680, &args![location_reference]).bool()
            && e.call(0x0057_5ca0, &args![location_reference]).u32() == space;
        if target_usable || location_usable {
            chosen = if target_usable {
                target
            } else {
                location_reference
            };
            let base = e.call(0x007a_f430, &args![chosen]).u32();
            if e.call(0x0050_9420, &args![base]).bool() {
                sleep = 1;
            }
            marker_index = first_free_marker(e, chosen);
            if marker_index == -1 || !marker_usable(e, chosen, marker_index, marker_data) {
                chosen = 0;
                no_marker = true;
            }
        } else if package != 0
            && e.call(PACKAGE_TYPE, &args![package]).u32() == 4
            && e.vcall(package, 0x13c, &args![actor, 0u32, -1.0f32, 0u32])
                .bool()
        {
            if !e.call(0x0057_9670, &args![actor]).bool() {
                let process = actor_process(e, actor);
                e.vcall(process, 0x284, &args![1u32]);
            }
            while node != 0 && !list_is_empty(e, node) && chosen == 0 {
                let item = node_item(e, node);
                let furniture = e.mem.u32(item);
                let mut removed = false;
                if e.call(0x0057_5ca0, &args![furniture]).u32() == space
                    && furniture_allowed(e, furniture, actor)
                {
                    let base = e.call(0x007a_f430, &args![furniture]).u32();
                    if e.call(0x0050_9420, &args![base]).bool() {
                        marker_index = first_free_marker(e, furniture);
                        if marker_index == -1
                            || !marker_usable(e, furniture, marker_index, marker_data)
                        {
                            list_remove_value(e, node, furniture);
                            node = e.call(0x0055_ac00, &args![tes]).u32();
                            removed = true;
                        } else {
                            chosen = furniture;
                            sleep = 1;
                        }
                    }
                }
                if !removed {
                    node = node_next(e, node);
                }
            }
        } else if package != 0
            && e.call(PACKAGE_TYPE, &args![package]).u32() == 3
            && e.vcall(package, 0x13c, &args![actor, 0u32, -1.0f32, 0u32])
                .bool()
            && e.call(0x0089_1d30, &args![actor]).u32() != 0
        {
            if !e.call(0x008a_7870, &args![actor]).bool() {
                let process = actor_process(e, actor);
                e.vcall(process, 0x288, &args![actor, 1u32]);
                e.vcall(actor, 0x200, &args![1u32]);
            }
            while node != 0 && !list_is_empty(e, node) && chosen == 0 {
                let item = node_item(e, node);
                let furniture = e.mem.u32(item);
                let mut removed = false;
                if e.call(0x0057_5ca0, &args![furniture]).u32() == space
                    && furniture != 0
                    && furniture_allowed(e, furniture, actor)
                {
                    let base = e.call(0x007a_f430, &args![furniture]).u32();
                    if e.call(0x0050_93f0, &args![base]).bool() {
                        marker_index = first_free_marker(e, furniture);
                        if marker_index == -1
                            || !marker_usable(e, furniture, marker_index, marker_data)
                        {
                            list_remove_value(e, node, furniture);
                            node = e.call(0x0055_ac00, &args![tes]).u32();
                            removed = true;
                        } else {
                            chosen = furniture;
                            sleep = 0;
                        }
                    }
                }
                if !removed {
                    node = node_next(e, node);
                }
            }
        }

        if chosen != 0 && e.vcall(actor, SLOT_GET_NODE, &[]).u32() != 0 {
            placed = e
                .call(
                    0x0088_d2f0,
                    &args![actor, chosen, marker_data, marker_index, sleep as u32],
                )
                .bool();
        } else if no_marker {
            let vector = e.vcall(actor, 0x1f4, &[]).u32();
            let words = [
                e.mem.u32(vector),
                e.mem.u32(vector + 4),
                e.mem.u32(vector + 8),
            ];
            e.with_stack(12, |e, position| {
                for (i, word) in words.iter().enumerate() {
                    e.mem.set_u32(position.addr() + 4 * i as u32, *word);
                }
                let cell = e.call(0x008d_6f30, &args![actor]).u32();
                let world_space = e.call(0x0057_5d70, &args![actor]).u32();
                e.call(
                    0x006d_6f80,
                    &args![world_space, cell, vector, position.addr()],
                );
                e.call(0x0057_5830, &args![actor, position.addr()]);
            });
            e.call(0x0056_2020, &args![actor]);
        }
    }
    if placed {
        let manager: u32 = e.global(0x0120_2d98);
        e.call(0x00c3_dfa0, &args![manager, 0u32]);
    }
    e.call(0x0045_0b60, &args![tes]);
}

// --- 00973460 .. 00974960: followers, temporary effects ---------------------

/// `GlobalTempEffectList` and `MagicEffectList` hold `NiPointer<BSTempEffect>`s
/// (+0x58 and +0x60, `BSSimpleList`s embedded in the object). `00633c90`
/// builds the `NiPointer` temporary from an effect, `0045cec0` releases it.
const NI_POINTER_CONSTRUCT: u32 = 0x0063_3c90;
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
/// `BSSimpleList<NiPointer<..>>::Remove(this = list, argument = address of the
/// NiPointer)`.
const NI_POINTER_LIST_REMOVE: u32 = 0x0063_1620;
/// `BSSimpleList<NiPointer<..>>::AddHead`-like push (`this` = list, argument =
/// address of the NiPointer).
const NI_POINTER_LIST_PUSH: u32 = 0x0052_a9b0;
/// Whether a `BSSimpleList<NiPointer<..>>` head is empty (`004a4460`).
const NI_POINTER_LIST_IS_EMPTY: u32 = 0x004a_4460;
/// The effect a node of a `NiPointer` list holds: `00559450` on the node's
/// item address (`006815c0`).
const NI_POINTER_VALUE: u32 = 0x0055_9450;
/// `GarbageCollector::Add_ov3(effect)` (Xbox PDB, cdecl).
const GARBAGE_COLLECTOR_ADD: u32 = 0x0086_8490;
/// `BSSimpleList<NiPointer<BSTempEffect>>::RemoveHead` (`this` = the node).
const NI_POINTER_LIST_REMOVE_HEAD: u32 = 0x004e_e8a0;
/// `NonActorMagicCaster::SetCurrentSpell(0)`-like call on a caster or effect
/// (`0041fd00`, one argument).
const SET_CURRENT_SPELL: u32 = 0x0041_fd00;

/// The effect held by a node of `GlobalTempEffectList` or `MagicEffectList`.
fn effect_of_node(e: &mut Engine, node: u32) -> u32 {
    let item = node_item(e, node);
    e.call(NI_POINTER_VALUE, &args![item]).u32()
}

/// Removes `effect` from the list headed by `list` through a `NiPointer`
/// temporary (the game's `Remove(NiPointer<..>(effect))`).
fn remove_effect_from_list(e: &mut Engine, list: u32, effect: u32) {
    e.with_stack(4, |e, pointer| {
        e.call(NI_POINTER_CONSTRUCT, &args![pointer.addr(), effect]);
        e.call(NI_POINTER_LIST_REMOVE, &args![list, pointer.addr()]);
        e.call(NI_POINTER_RELEASE, &args![pointer.addr()]);
    });
}

/// The removal tail of the temporary effect sweeps: with a previous node, the
/// effect is removed by value from the list (starting at `previous`) and the
/// walk goes on at the node after `previous`; without one the head node is
/// replaced by its successor (`RemoveHead`) and the walk stays on `node`.
fn drop_effect_node(e: &mut Engine, node: u32, previous: u32, effect: u32) -> u32 {
    if previous != 0 {
        remove_effect_from_list(e, previous, effect);
        node_next(e, previous)
    } else {
        e.call(NI_POINTER_LIST_REMOVE_HEAD, &args![node]);
        node
    }
}

// Translated from 00973460 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether some actor of level 0 (the tail index is checked before each step,
/// and the walk stops at the first hit) is a mobile object whose slot 0x218
/// is true and for which `008a81e0(actor, reference)` is 3, unless it is
/// `00493bb0` with a package target (slot 0x428) that is `reference` and for
/// which `009818b0` holds. Note that the actor variable keeps its last value
/// over objects that are not actors, as in the code.
pub fn fn_00973460(e: &mut Engine, this: Ptr<ProcessLists>, reference: u32) -> bool {
    let array = mob_process_array(this);
    let mut found = false;
    let mut actor = 0u32;
    let mut index = 0u32;
    while index < array_tail(e, array, 0) && !found {
        let object = array_object(e, array, index);
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor != 0
            && e.vcall(actor, 0x218, &[]).bool()
            && e.call(0x008a_81e0, &args![actor, reference]).u32() == 3
        {
            let mut excused = false;
            if e.call(0x0049_3bb0, &args![actor]).bool() && e.vcall(actor, 0x428, &[]).u32() != 0 {
                let package = e.vcall(actor, 0x428, &[]).u32();
                if e.call(0x0040_30b0, &args![package]).u32() == reference {
                    let package = e.vcall(actor, 0x428, &[]).u32();
                    excused = e.call(0x0098_18b0, &args![package]).bool();
                }
            }
            if !excused {
                found = true;
            }
        }
        index = index.wrapping_add(1);
    }
    found
}

// Translated from 00973590 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::BuildFollowerListRecursive` (Xbox PDB): adds to `list` the
/// followers of `reference` (the list at +0xC of its extra data's follower
/// record, `00422700`) and the level-0 actors whose package (type 2 or 7)
/// targets it (`00881650`), each one not yet in the list, and recurses on each
/// one added.
pub fn processlists_build_follower_list_recursive(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    reference: u32,
    list: u32,
) {
    let array = mob_process_array(this);
    let extra = extra_list(e, reference);
    let record = e.call(EXTRA_FOLLOWERS, &args![extra]).u32();
    if record != 0 {
        let mut node = e.mem.u32(record + 0xc);
        while node != 0 && !list_is_empty(e, node) {
            let follower = node_value(e, node);
            if !list_contains_value(e, list, follower) {
                list_push_value(e, list, follower);
                processlists_build_follower_list_recursive(e, this, follower, list);
            }
            node = node_next(e, node);
        }
    }
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        if object == 0 || !is_actor(e, object) {
            continue;
        }
        let actor = object;
        let package = e.call(0x0093_44a0, &args![actor]).u32();
        if package == 0 {
            continue;
        }
        let package = e.call(0x0093_44a0, &args![actor]).u32();
        let mut kind = e.call(PACKAGE_TYPE, &args![package]).u32();
        if kind != 2 {
            let package = e.call(0x0093_44a0, &args![actor]).u32();
            kind = e.call(PACKAGE_TYPE, &args![package]).u32();
            if kind != 7 {
                continue;
            }
        }
        if e.call(0x0088_1650, &args![actor]).u32() == reference
            && !list_contains_value(e, list, actor)
        {
            list_push_value(e, list, actor);
            processlists_build_follower_list_recursive(e, this, actor, list);
        }
    }
}

/// Whether `list` holds `value` (`005f65d0` takes the address of the value).
fn list_contains_value(e: &mut Engine, list: u32, value: u32) -> bool {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(LIST_CONTAINS, &args![list, slot.addr()]).bool()
    })
}

// Translated from 00973710 (decompiled, FalloutNV.exe 1.4.0.525)
/// The highest "how much `reference` is noticed" value of the actors of level
/// 0 (-100 to 100; 0 when there is no actor), 100 at once for an actor in
/// combat with it. First clears the flag at +0x66C of the player
/// (`00973a70(player, 0)`). Unless `skip_followers` is set, actors that are
/// followers of `reference` (`BuildFollowerListRecursive`), or when
/// `reference` is the player and `00566950` holds, are not looked at. For the
/// others (slot 0x22c with 0, `00440da0` and `00437bd0` all false): when
/// `reference` is the player and the flag is not set, an actor within the
/// distance of the integer setting `0x011cd850` sets it
/// (`00973a70(player, 1)`); an actor in combat (`00493bb0`) that targets
/// `reference` (slot 0x428, `004030b0`) without `009818b0`, or an alarmed one
/// (`008a61b0`) whose package target (`00881650`) is the player, gives 100;
/// otherwise the actor's process (slot 0x2d4, `reference, 0`) gives the value
/// (0x7fffffff meaning 0), clamped to -100..=100, and a positive value of an
/// actor that should attack the player (`008b06d0`) sets `008c8bb0(player, 1)`.
/// The value of the last actor looked at carries over to the actors skipped
/// afterwards. The unwind frame is not translated.
pub fn fn_00973710(
    e: &mut Engine,
    this: Ptr<ProcessLists>,
    reference: u32,
    skip_followers: u8,
) -> i32 {
    let array = mob_process_array(this);
    let mut value: i32 = -100;
    let mut highest: i32 = 0x7fff_ffff;
    let player_ref = player(e);
    fn_00973a70(e, Ptr::new(player_ref), 0);
    let mut index = 0u32;
    loop {
        if index >= array_tail(e, array, 0) {
            if highest == 0x7fff_ffff {
                highest = 0;
            }
            return highest;
        }
        let object = array_object(e, array, index);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        let mut skip = false;
        if skip_followers == 0 && actor != 0 {
            e.with_stack(8, |e, list| {
                e.call(LIST_CONSTRUCT, &args![list.addr()]);
                processlists_build_follower_list_recursive(e, this, reference, list.addr());
                if list_contains_value(e, list.addr(), actor)
                    || (reference == player(e) && e.call(0x0056_6950, &args![actor]).bool())
                {
                    skip = true;
                }
                e.call(0x0046_ffb0, &args![list.addr()]);
            });
        }
        if !skip
            && actor != 0
            && !e.vcall(actor, SLOT_0X22C, &args![0u32]).bool()
            && !e.call(0x0044_0da0, &args![actor]).bool()
            && !e.call(0x0043_7bd0, &args![actor]).bool()
        {
            let player_ref = player(e);
            if reference == player_ref && fn_00973a90(e, Ptr::new(player_ref)) == 0 {
                let distance = e
                    .call(0x0057_23b0, &args![reference, actor, 0u32, 0u32])
                    .f32();
                let setting = e.call(SETTING_INT_VALUE, &args![0x011c_d850u32]).u32();
                let limit = e.mem.u32(setting) as i32;
                if (distance as f64) <= limit as f64 {
                    fn_00973a70(e, Ptr::new(player_ref), 1);
                }
            }
            if e.call(0x0049_3bb0, &args![actor]).bool()
                || e.call(0x008a_61b0, &args![actor]).bool()
            {
                if e.call(0x0049_3bb0, &args![actor]).bool()
                    && e.vcall(actor, 0x428, &[]).u32() != 0
                {
                    let package = e.vcall(actor, 0x428, &[]).u32();
                    if e.call(0x0040_30b0, &args![package]).u32() == reference {
                        let package = e.vcall(actor, 0x428, &[]).u32();
                        if !e.call(0x0098_18b0, &args![package]).bool() {
                            return 100;
                        }
                    }
                }
                if e.call(0x008a_61b0, &args![actor]).bool()
                    && e.call(0x0088_1650, &args![actor]).u32() == player(e)
                {
                    return 100;
                }
            }
            let process = actor_process(e, actor);
            value = e.vcall(process, 0x2d4, &args![reference, 0u32]).i32();
            if value == 0x7fff_ffff {
                value = 0;
            }
            value = value.clamp(-100, 100);
            if value > 0 {
                let player_ref = player(e);
                let attack = e.with_stack(4, |e, out| {
                    e.call(
                        0x008b_06d0,
                        &args![actor, player_ref, 0u32, out.addr(), 0u32],
                    )
                    .bool()
                });
                if attack {
                    e.call(0x008c_8bb0, &args![player_ref, 1u32]);
                }
            }
        }
        if highest == 0x7fff_ffff || highest < value {
            highest = value;
        }
        index = index.wrapping_add(1);
    }
}

// Translated from 00973a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at +0x66C of the object it is called on (the
/// callers pass the player).
pub fn fn_00973a70(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0x66c, value);
}

// Translated from 00973a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at +0x66C of the object it is called on (see `fn_00973a70`).
pub fn fn_00973a90(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x66c)
}

// Translated from 00973ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The first actor of level 0 that owns `reference`
/// (`TESObjectREFR::IsAnOwner`, `005785e0(actor, 1)`) and whose base data's
/// `SellBuysItem` (`0047f600` on `004181e0(actor) + 0x90`, with the form
/// `007af430(reference)`) holds, or 0. The code also walks a list of further
/// actors (`BSSimpleList` iteration) that starts out empty and is never
/// filled, so it never runs.
pub fn fn_00973ab0(e: &mut Engine, this: Ptr<ProcessLists>, reference: u32) -> u32 {
    let array = mob_process_array(this);
    let mut result = 0u32;
    let mut index = 0u32;
    while result == 0 && index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor != 0 && e.call(0x0057_85e0, &args![reference, actor, 1u32]).bool() {
            let form = e.call(0x007a_f430, &args![reference]).u32();
            let base = e.call(0x0041_81e0, &args![actor]).u32();
            if e.call(0x0047_f600, &args![base.wrapping_add(0x90), form])
                .bool()
            {
                result = actor;
            }
        }
    }
    result
}

// Translated from 00973cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tells the process (`008d8520`) of every actor of level 0 slot 0x2c4 with
/// `reference, 3`.
pub fn fn_00973cb0(e: &mut Engine, this: Ptr<ProcessLists>, reference: u32) {
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        if object != 0 && is_actor(e, object) {
            let process = actor_process(e, object);
            e.vcall(process, 0x2c4, &args![reference, 3u32]);
        }
    }
}

// Translated from 00973d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether some object of level 0 for which `00574900` holds has a base form
/// (`007af430`) whose `00516bf0` is `value`.
pub fn fn_00973d50(e: &mut Engine, this: Ptr<ProcessLists>, value: u32) -> bool {
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        if object != 0 && e.call(0x0057_4900, &args![object]).bool() {
            let form = e.call(0x007a_f430, &args![object]).u32();
            if e.call(0x0051_6bf0, &args![form]).u32() == value {
                return true;
            }
        }
    }
    false
}

// Translated from 00973de0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Goes through the objects of the array (`0055b980` gives their number) and
/// ends the interrupt package (`00881680` with 0) of every actor that does
/// not answer slot 0x22c (with 0) and is either far from the player (the
/// player's `009549a0(actor)` holds and the distance, `005723b0`, is above
/// 350.0) or has `GetForceNextUpdate` (`00566950`); then calls
/// `008ad910(actor, player, 0, 0)`.
pub fn fn_00973de0(e: &mut Engine, this: Ptr<ProcessLists>) {
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < e.call(0x0055_b980, &args![array]).u32() {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor == 0 || e.vcall(actor, SLOT_0X22C, &args![0u32]).bool() {
            continue;
        }
        let player_ref = player(e);
        let mut ends = false;
        let mut decided = false;
        if e.call(0x0095_49a0, &args![player_ref, actor]).bool() {
            let distance = e
                .call(0x0057_23b0, &args![player_ref, actor, 0u32, 0u32])
                .f64();
            let far: f64 = e.global(0x0102_c188);
            if distance > far {
                ends = true;
                decided = true;
            }
        }
        if !decided {
            ends = e.call(0x0056_6950, &args![actor]).bool();
        }
        if ends {
            e.call(0x0088_1680, &args![actor, 0u32]);
            let player_ref = player(e);
            e.call(0x008a_d910, &args![actor, player_ref, 0u32, 0u32]);
        }
    }
}

// Translated from 00973ee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like `fn_00973de0` for the given `reference` (nothing happens when it is
/// 0): every actor that does not answer slot 0x22c (with 0) and for which the
/// player's `00954a70(actor, 0)` or its own `00566950` holds ends its
/// interrupt package and gets `008ad910(actor, reference, 0, 0)`.
pub fn fn_00973ee0(e: &mut Engine, this: Ptr<ProcessLists>, reference: u32) {
    if reference == 0 {
        return;
    }
    let array = mob_process_array(this);
    let mut index = 0u32;
    while index < e.call(0x0055_b980, &args![array]).u32() {
        let object = array_object(e, array, index);
        index = index.wrapping_add(1);
        let mut actor = 0u32;
        if object != 0 && is_actor(e, object) {
            actor = object;
        }
        if actor == 0 || e.vcall(actor, SLOT_0X22C, &args![0u32]).bool() {
            continue;
        }
        let player_ref = player(e);
        if e.call(0x0095_4a70, &args![player_ref, actor, 0u32]).bool()
            || e.call(0x0056_6950, &args![actor]).bool()
        {
            e.call(0x0088_1680, &args![actor, 0u32]);
            e.call(0x008a_d910, &args![actor, reference, 0u32, 0u32]);
        }
    }
}

// Translated from 00973fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ProcessLists::AddTempEffect` (Xbox PDB): adds `effect` (a `BSTempEffect`)
/// to the `MagicEffectList` (+0x60) when its slot 0x9c answers 4 to 6, else
/// to the `GlobalTempEffectList` (+0x58). Nothing happens for a null effect.
/// The unwind frame is not translated.
pub fn processlists_add_temp_effect(e: &mut Engine, this: Ptr<ProcessLists>, effect: u32) {
    if effect == 0 {
        return;
    }
    let kind = e.vcall(effect, 0x9c, &[]).i32();
    let list = if (4..=6).contains(&kind) {
        this.addr() + MAGIC_EFFECT_LIST
    } else {
        this.addr() + GLOBAL_TEMP_EFFECT_LIST
    };
    e.with_stack(4, |e, pointer| {
        e.call(NI_POINTER_CONSTRUCT, &args![pointer.addr(), effect]);
        e.call(NI_POINTER_LIST_PUSH, &args![list, pointer.addr()]);
        e.call(NI_POINTER_RELEASE, &args![pointer.addr()]);
    });
}

// Translated from 009740a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes from both temporary effect lists (`GlobalTempEffectList`, +0x58,
/// and `MagicEffectList`, +0x60) every effect whose reference (`0084e3a0`) is
/// `reference`: the effect's spell is cleared (`0041fd00(effect, 0)`), it is
/// handed to the garbage collector and taken out of the list.
pub fn fn_009740a0(e: &mut Engine, this: Ptr<ProcessLists>, reference: u32) {
    for list_offset in [GLOBAL_TEMP_EFFECT_LIST, MAGIC_EFFECT_LIST] {
        let list = this.addr() + list_offset;
        if e.call(NI_POINTER_LIST_IS_EMPTY, &args![list]).bool() {
            continue;
        }
        let mut node = list;
        let mut previous = 0u32;
        while node != 0 && !e.call(NI_POINTER_LIST_IS_EMPTY, &args![node]).bool() {
            let effect = effect_of_node(e, node);
            if e.call(0x0084_e3a0, &args![effect]).u32() == reference {
                e.call(SET_CURRENT_SPELL, &args![effect, 0u32]);
                e.call(GARBAGE_COLLECTOR_ADD, &args![effect]);
                node = drop_effect_node(e, node, previous, effect);
            } else {
                previous = node;
                node = node_next(e, node);
            }
        }
    }
}

// Translated from 00974290 (decompiled, FalloutNV.exe 1.4.0.525)
/// Removes from the `GlobalTempEffectList` (+0x58) the effects whose slot
/// 0x9c answers 2 and that are a magic caster effect (`__RTDynamicCast`
/// between the two type descriptors at `0x0119c08c` and `0x0119c1e8`) whose
/// `00700300` is `owner`: the effect's `00490e40` (when it has one) gets
/// `00450f90(1)`, the spell is cleared (`0041fd00`) and the effect is
/// handed to the garbage collector and removed. Nothing happens for a null
/// `owner`.
pub fn fn_00974290(e: &mut Engine, this: Ptr<ProcessLists>, owner: u32) {
    if owner == 0 {
        return;
    }
    let list = this.addr() + GLOBAL_TEMP_EFFECT_LIST;
    if e.call(NI_POINTER_LIST_IS_EMPTY, &args![list]).bool() {
        return;
    }
    let mut node = list;
    let mut previous = 0u32;
    while node != 0 && !e.call(NI_POINTER_LIST_IS_EMPTY, &args![node]).bool() {
        let effect = effect_of_node(e, node);
        let mut matched = false;
        if e.vcall(effect, 0x9c, &[]).i32() == 2 {
            let caster = e
                .call(
                    0x00ec_43fb,
                    &args![effect, 0u32, 0x0119_c08cu32, 0x0119_c1e8u32, 0u32],
                )
                .u32();
            if caster != 0 && e.call(0x0070_0300, &args![caster]).u32() == owner {
                matched = true;
                let helper = e.call(0x0049_0e40, &args![caster]).u32();
                if helper != 0 {
                    e.call(0x0045_0f90, &args![helper, 1u32]);
                }
                e.call(SET_CURRENT_SPELL, &args![caster, 0u32]);
                e.call(GARBAGE_COLLECTOR_ADD, &args![effect]);
                node = drop_effect_node(e, node, previous, effect);
            }
        }
        if !matched {
            previous = node;
            node = node_next(e, node);
        }
    }
}

/// One walk of `fn_00974420` over a temporary effect list: updates every
/// effect (slot 0x94 with the time `delta`) and removes those whose update
/// returns false, which (for `release_first`, the `GlobalTempEffectList`)
/// first get slot 0x90.
fn update_effect_list(e: &mut Engine, list: u32, delta: f32, release_first: bool) {
    if e.call(NI_POINTER_LIST_IS_EMPTY, &args![list]).bool() {
        return;
    }
    let mut node = list;
    let mut first = true;
    while node != 0 {
        let effect = effect_of_node(e, node);
        if !first {
            node = node_next(e, node);
        }
        if effect == 0 || e.vcall(effect, 0x94, &args![delta]).bool() {
            if first {
                node = node_next(e, node);
                first = false;
            }
        } else {
            if release_first {
                e.vcall(effect, 0x90, &[]);
                e.call(GARBAGE_COLLECTOR_ADD, &args![effect]);
            } else {
                e.call(GARBAGE_COLLECTOR_ADD, &args![effect]);
            }
            remove_effect_from_list(e, list, effect);
        }
    }
}

// Translated from 00974420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Per-frame update of the temporary effects, one after the other. First tells
/// the process (`008d8520`) of every object of level 0 that is an actor and
/// answers slot 0x57c with true slot 0x584 with the actor. Then updates the
/// `GlobalTempEffectList` and the `MagicEffectList` with the time `delta`
/// (slot 0x94 of each effect; an effect whose update returns false is
/// released, slot 0x90 for the first list, handed to the garbage collector and
/// taken out of the list). The unwind frame is not translated.
pub fn fn_00974420(e: &mut Engine, this: Ptr<ProcessLists>, delta: f32) {
    let array = mob_process_array(this);
    let mut index = array_head(e, array, 0);
    while index < array_tail(e, array, 0) {
        let object = array_object(e, array, index);
        let process = if object != 0 {
            actor_process(e, object)
        } else {
            0
        };
        if e.vcall(object, SLOT_IS_ACTOR, &[]).bool()
            && process != 0
            && e.vcall(process, 0x57c, &[]).bool()
        {
            e.vcall(process, 0x584, &args![object]);
        }
        index = index.wrapping_add(1);
    }
    update_effect_list(e, this.addr() + GLOBAL_TEMP_EFFECT_LIST, delta, true);
    update_effect_list(e, this.addr() + MAGIC_EFFECT_LIST, delta, false);
}

// Translated from 009746c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Updates the temporary effects with the time `delta`: without the setting
/// at `0x011f127c` (a byte), `fn_00974420` does it. With it, and when the two
/// lists hold effects (`00978990` counts them), the effects are copied to a
/// local array of 12-byte-or-larger records (`effect`, `delta`, "is in the
/// first list" byte at +8, "expired" byte at +9; `00978c50` constructs the
/// array, `00978bc0` adds a record, `006a1440` addresses one, `0044ddc0` is
/// the count, `006a7af0` the record at an index, `00978cf0` destroys it),
/// the task manager (global `0x01202df4`, `00c44c90`) runs
/// `fn_00974960` over the records (`0087cf10` builds the task), and the
/// expired effects are handed to the garbage collector and removed from their
/// list. The unwind frame is not translated.
pub fn fn_009746c0(e: &mut Engine, this: Ptr<ProcessLists>, delta: f32) {
    let setting = e.call(SETTING_VALUE, &args![0x011f_127cu32]).u32();
    if e.mem.u8(setting) == 0 {
        fn_00974420(e, this, delta);
        return;
    }
    let first_list = this.addr() + GLOBAL_TEMP_EFFECT_LIST;
    let second_list = this.addr() + MAGIC_EFFECT_LIST;
    let first_count = e.call(0x0097_8990, &args![first_list]).u32();
    let second_count = e.call(0x0097_8990, &args![second_list]).u32();
    if first_count.wrapping_add(second_count) == 0 {
        return;
    }
    e.with_stack(0x10, |e, array| {
        e.call(0x0097_8c50, &args![array.addr()]);
        for (list, in_first) in [(first_list, 1u8), (second_list, 0u8)] {
            let mut node = list;
            while node != 0 {
                let effect = effect_of_node(e, node);
                if effect != 0 {
                    let slot = e.call(0x0097_8bc0, &args![array.addr()]).u32();
                    let record = e.call(0x006a_1440, &args![array.addr(), slot]).u32();
                    e.mem.set_u32(record, effect);
                    let record = e.call(0x006a_1440, &args![array.addr(), slot]).u32();
                    e.mem.set_u32(record + 4, delta.to_bits());
                    let record = e.call(0x006a_1440, &args![array.addr(), slot]).u32();
                    e.mem.set_u8(record + 8, in_first);
                    let record = e.call(0x006a_1440, &args![array.addr(), slot]).u32();
                    e.mem.set_u8(record + 9, 0);
                }
                node = node_next(e, node);
            }
        }
        if e.call(0x0044_ddc0, &args![array.addr()]).u32() != 0 {
            let count = e.call(0x0044_ddc0, &args![array.addr()]).u32();
            e.with_stack(0x20, |e, task| {
                e.call(
                    0x0087_cf10,
                    &args![task.addr(), 0x0097_4960u32, array.addr(), count],
                );
                let manager: u32 = e.global(TASK_MANAGER);
                e.call(0x00c4_4c90, &args![manager, task.addr()]);
            });
            let mut position = 0u32;
            while position < e.call(0x0044_ddc0, &args![array.addr()]).u32() {
                let record = e.call(0x006a_7af0, &args![array.addr(), position]).u32();
                if e.mem.u8(record + 9) != 0 {
                    let effect = e.mem.u32(record);
                    e.call(GARBAGE_COLLECTOR_ADD, &args![effect]);
                    let list = if e.mem.u8(record + 8) != 0 {
                        first_list
                    } else {
                        second_list
                    };
                    remove_effect_from_list(e, list, effect);
                }
                position += 1;
            }
        }
        e.call(0x0097_8cf0, &args![array.addr()]);
    });
}

// Translated from 00974960 (decompiled, FalloutNV.exe 1.4.0.525)
/// The task `fn_009746c0` hands to the task manager for the record at `index`
/// of `array` (`006a7af0`): updates the record's effect with the time stored
/// in it (slot 0x94) and marks the record expired (byte at +9) when the
/// update returns false.
pub fn fn_00974960(e: &mut Engine, array: u32, index: u32) {
    let record = e.call(0x006a_7af0, &args![array, index]).u32();
    let delta = f32::from_bits(e.mem.u32(record + 4));
    let effect = e.mem.u32(record);
    if !e.vcall(effect, 0x94, &args![delta]).bool() {
        e.mem.set_u8(record + 9, 1);
    }
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x006abe40, bsscrap_array_effect_task_data_reallocate(Ptr, u32, u32) -> u32),
        entry!(
            0x008d0600,
            processlists_print_lists(Ptr<ProcessLists>, u32, u32)
        ),
        entry!(
            0x0096c860,
            processlists_update_animation_scenegraph(Ptr<ProcessLists>)
        ),
        entry!(0x0096c950, fn_0096c950(Ptr<Actor>) -> bool),
        entry!(
            0x0096c970,
            processlists_parallel_update_animation_scenegraph(Ptr<ProcessLists>)
        ),
        entry!(0x0096cb20, fn_0096cb20(u32, u32)),
        entry!(0x0096cb50, fn_0096cb50(Ptr<ProcessLists>)),
        entry!(0x0096cc80, fn_0096cc80(Ptr, u8)),
        entry!(0x0096cca0, fn_0096cca0(Ptr<ProcessLists>)),
        entry!(0x0096cda0, fn_0096cda0(Ptr<ProcessLists>)),
        entry!(0x0096cf60, fn_0096cf60(u32, u32)),
        entry!(0x0096cfa0, fn_0096cfa0(Ptr<ProcessLists>)),
        entry!(
            0x0096d020,
            fn_0096d020(Ptr<ProcessLists>) -> Ptr<ProcessLists>
        ),
        entry!(0x0096d290, fn_0096d290(Ptr<ProcessLists>)),
        entry!(
            0x0096d450,
            processlists_add_reference(Ptr<ProcessLists>, u32, u32, u32, u32, u32)
        ),
        entry!(
            0x0096d470,
            processlists_remove_reference(Ptr<ProcessLists>, u32, u32)
        ),
        entry!(
            0x0096d490,
            processlists_get_system_time_clock(Ptr<ProcessLists>) -> f32
        ),
        entry!(
            0x0096d4b0,
            processlists_set_system_time_clock(Ptr<ProcessLists>, f32)
        ),
        entry!(0x0096d520, fn_0096d520()),
        entry!(
            0x0096d810,
            processlists_update_process_lists(Ptr<ProcessLists>)
        ),
        entry!(0x0096db30, fn_0096db30(Ptr<ProcessLists>, f32, u8, u32)),
        entry!(0x0096dcb0, fn_0096dcb0(Ptr<ProcessLists>, u32)),
        entry!(
            0x0096df40,
            processlists_process_high_actors_for_fast_travel(Ptr<ProcessLists>)
        ),
        entry!(0x0096e150, fn_0096e150(Ptr<ProcessLists>, u32, u8)),
        entry!(0x0096e290, fn_0096e290(Ptr<ProcessLists>) -> u32),
        entry!(0x0096e2b0, fn_0096e2b0(Ptr<ProcessLists>, u32)),
        entry!(0x0096e2f0, fn_0096e2f0(Ptr<ProcessLists>, u32)),
        entry!(0x0096e310, fn_0096e310(Ptr<ProcessLists>)),
        entry!(
            0x0096e3c0,
            processlists_insert_actor_close_to_player(Ptr<ProcessLists>, u32)
        ),
        entry!(
            0x0096e570,
            processlists_sort_actors_close_to_player(Ptr<ProcessLists>)
        ),
        entry!(
            0x0096e6f0,
            processlists_remove_actor_close_to_player(Ptr<ProcessLists>, u32)
        ),
        entry!(0x0096e7d0, fn_0096e7d0(Ptr<ProcessLists>, u32, i32) -> bool),
        entry!(
            0x0096e870,
            processlists_add_actor_to_temp_change_list(Ptr<ProcessLists>, u32)
        ),
        entry!(0x0096e9b0, fn_0096e9b0(Ptr<ProcessLists>)),
        entry!(
            0x0096eb40,
            processlists_change_process_level_temp_list(Ptr<ProcessLists>)
        ),
        entry!(
            0x0096f400,
            processlists_remove_actor_from_temp_change_list(Ptr<ProcessLists>, u32)
        ),
        entry!(0x0096f450, fn_0096f450(Ptr<ProcessLists>, u32, u32) -> u32),
        entry!(0x0096f600, fn_0096f600(Ptr<ProcessLists>, u32, u8)),
        entry!(0x009707f0, fn_009707f0(Ptr, u32)),
        entry!(0x00970810, fn_00970810(Ptr<ProcessLists>, u32)),
        entry!(
            0x00970a20,
            processlists_get_actor_ref_in_high(Ptr<ProcessLists>, u32, u32) -> u32
        ),
        entry!(
            0x00970b30,
            fn_00970b30(Ptr<ProcessLists>, u32, u8, u8) -> u32
        ),
        entry!(0x00970d50, fn_00970d50(Ptr<ProcessLists>)),
        entry!(
            0x00970ee0,
            processlists_distance_list_compare_fn(u32, u32) -> i32
        ),
        entry!(0x00970f60, fn_00970f60(Ptr<ProcessLists>, u32) -> u32),
        entry!(
            0x009711e0,
            processlists_send_actors_yell_alarm(Ptr<ProcessLists>, u32, u32) -> u32
        ),
        entry!(0x009715c0, fn_009715c0(Ptr<ProcessLists>, u32, u32) -> u32),
        entry!(
            0x009719e0,
            processlists_get_actors_process_crime_alarm(Ptr<ProcessLists>, u32, u8) -> u32
        ),
        entry!(
            0x00971ba0,
            processlists_turn_off_movement_in_high(Ptr<ProcessLists>)
        ),
        entry!(
            0x00971c30,
            fn_00971c30(Ptr<ProcessLists>, u32, u32, u8) -> u32
        ),
        entry!(0x00971fd0, fn_00971fd0(Ptr<ProcessLists>, f32) -> u32),
        entry!(0x009721f0, fn_009721f0(Ptr<ProcessLists>, u32)),
        entry!(0x009722e0, fn_009722e0(Ptr<ProcessLists>, u32, u32) -> bool),
        entry!(0x009723f0, fn_009723f0(Ptr<ProcessLists>)),
        entry!(0x009724e0, fn_009724e0(Ptr<ProcessLists>, i32, u32) -> u32),
        entry!(
            0x00972570,
            fn_00972570(Ptr<ProcessLists>, u32, u32, i32) -> u32
        ),
        entry!(
            0x00972600,
            fn_00972600(Ptr<ProcessLists>, u32, u32, u32, i32, u32, i32) -> bool
        ),
        entry!(0x009726e0, fn_009726e0(Ptr<ProcessLists>, u32)),
        entry!(
            0x00972740,
            processlists_get_crime_index(Ptr<ProcessLists>, u32, u32) -> u16
        ),
        entry!(
            0x009727c0,
            processlists_get_crime_by_index(Ptr<ProcessLists>, u32, u16) -> u32
        ),
        entry!(0x00972840, fn_00972840(Ptr<ProcessLists>, u32, u8)),
        entry!(0x00972aa0, fn_00972aa0(Ptr<ProcessLists>, u32)),
        entry!(0x00972bb0, fn_00972bb0(Ptr<ProcessLists>)),
        entry!(
            0x00972d30,
            processlists_place_actors_in_beds_or_chairs(Ptr<ProcessLists>)
        ),
        entry!(0x00973460, fn_00973460(Ptr<ProcessLists>, u32) -> bool),
        entry!(
            0x00973590,
            processlists_build_follower_list_recursive(Ptr<ProcessLists>, u32, u32)
        ),
        entry!(0x00973710, fn_00973710(Ptr<ProcessLists>, u32, u8) -> i32),
        entry!(0x00973a70, fn_00973a70(Ptr, u8)),
        entry!(0x00973a90, fn_00973a90(Ptr) -> u8),
        entry!(0x00973ab0, fn_00973ab0(Ptr<ProcessLists>, u32) -> u32),
        entry!(0x00973cb0, fn_00973cb0(Ptr<ProcessLists>, u32)),
        entry!(0x00973d50, fn_00973d50(Ptr<ProcessLists>, u32) -> bool),
        entry!(0x00973de0, fn_00973de0(Ptr<ProcessLists>)),
        entry!(0x00973ee0, fn_00973ee0(Ptr<ProcessLists>, u32)),
        entry!(
            0x00973fd0,
            processlists_add_temp_effect(Ptr<ProcessLists>, u32)
        ),
        entry!(0x009740a0, fn_009740a0(Ptr<ProcessLists>, u32)),
        entry!(0x00974290, fn_00974290(Ptr<ProcessLists>, u32)),
        entry!(0x00974420, fn_00974420(Ptr<ProcessLists>, f32)),
        entry!(0x009746c0, fn_009746c0(Ptr<ProcessLists>, f32)),
        entry!(0x00974960, fn_00974960(u32, u32)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// The vtable every test object shares: slot `s` points at `slot(s)`,
    /// a fake address the test registers a double at.
    const VTABLE: u32 = 0x0e00_0000;
    const SLOT_BASE: u32 = 0x0f00_0000;

    fn slot(s: u32) -> u32 {
        SLOT_BASE + s
    }

    /// An engine with the globals the code reads mapped, the common vtable
    /// and doubles for the callees nobody has translated (the
    /// process array accessor by index, the `ProcessLists` -> array getter, the
    /// setting getters). The list helpers (`BSSimpleList`) run for real.
    fn fixture() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x011d_d000u32,
            0x011d_e000,
            0x011d_1000,
            0x011c_d000,
            0x011c_3000,
            0x011a_3000,
            0x0120_2000,
            0x011f_4000,
            0x011f_6000,
            0x0101_1000,
            0x0101_2000,
            0x0101_7000,
            0x0103_5000,
            0x0108_2000,
            0x0108_4000,
            0x0108_c000,
            0x010a_2000,
        ] {
            e.map(page, 0x1000);
        }
        e.map(0x011e_0000, 0x12000);
        let slots: Vec<u32> = (0..0x2a0).map(|i| SLOT_BASE + i * 4).collect();
        e.put_vtable(VTABLE, &slots);
        e.set_global(ZERO, 0.0f64);
        e.set_global(ONE, 1.0f64);
        e.set_global(TWO, 2.0f64);
        e.set_global(CLOCK_LIMIT, 100000.0f64);
        e.set_global(HOURS_PER_DAY, 24.0f64);
        e.set_global(SMALL_TIME, 0.05f64);
        e.set_global(MILLISECONDS_PER_SECOND, 1000.0f64);
        e.set_global(FOLLOWER_DELAY, 3600.0f32);
        // `ProcessArray` object accessor: the table at +0x3c of the array.
        e.register(ARRAY_OBJECT, |e, a| {
            let table = e.mem.u32(a[0] + 0x3c);
            Ret {
                eax: e.mem.u32(table + 4 * a[1]),
                ..Ret::default()
            }
        });
        e.register(PROCESS_ARRAY_OF, |_, a| Ret {
            eax: a[0] + 4,
            ..Ret::default()
        });
        // Setting getters: the value lives 0x100 bytes after the setting.
        e.register(SETTING_VALUE, |_, a| Ret {
            eax: a[0] + 0x100,
            ..Ret::default()
        });
        e.register(SETTING_INT_VALUE, |_, a| Ret {
            eax: a[0] + 0x100,
            ..Ret::default()
        });
        e.register(SETTING_FLOAT_VALUE, |_, a| Ret {
            eax: a[0] + 0x100,
            ..Ret::default()
        });
        e.register(LIST_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        // `00726070` (next node of a list) is not translated yet.
        e.register(NODE_NEXT, |e, a| Ret {
            eax: e.mem.u32(a[0] + 4),
            ..Ret::default()
        });
        // `00470440` (the list node constructor `005ae3d0` uses) is not
        // translated yet: it copies the head value of the list it is given
        // into the new node and clears the node's next word.
        e.register(0x0047_0440, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            e.mem.set_u32(a[0] + 4, 0);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        // `00905330` (list remove) is not translated yet: this double does
        // what its decompilation does (the value is read from the address
        // given; the first node's successor moves into the head).
        e.register(LIST_REMOVE, |e, a| {
            let (list, wanted) = (a[0], e.mem.u32(a[1]));
            if wanted == 0 || (e.mem.u32(list) == 0 && e.mem.u32(list + 4) == 0) {
                return Ret::default();
            }
            let mut previous = list;
            let mut node = list;
            while node != 0 && e.mem.u32(node) != wanted {
                previous = node;
                node = e.mem.u32(node + 4);
            }
            if node == 0 {
                return Ret::default();
            }
            if node == list {
                let next = e.mem.u32(list + 4);
                if next == 0 {
                    e.mem.set_u32(list, 0);
                } else {
                    let value = e.mem.u32(next);
                    let after = e.mem.u32(next + 4);
                    e.mem.set_u32(list, value);
                    e.mem.set_u32(list + 4, after);
                }
            } else {
                let after = e.mem.u32(node + 4);
                e.mem.set_u32(previous + 4, after);
            }
            Ret::default()
        });
        e
    }

    /// A test object with the common vtable.
    fn object(e: &mut Engine) -> u32 {
        let o = e.mem.alloc(0x200);
        e.mem.set_u32(o, VTABLE);
        o
    }

    /// A `ProcessLists` object whose process array holds `objects` as level
    /// `level` (head 0, tail `objects.len()`), the other levels empty.
    fn lists_with(e: &mut Engine, level: u32, objects: &[u32]) -> Ptr<ProcessLists> {
        let lists: Ptr<ProcessLists> = e.new_object();
        set_objects(e, lists.addr() + MOB_PROCESS_ARRAY, level, objects);
        lists
    }

    /// Puts `objects` into the process array at `array` as `level`.
    fn set_objects(e: &mut Engine, array: u32, level: u32, objects: &[u32]) {
        let table = e.mem.alloc(4 * objects.len().max(1) as u32);
        for (i, o) in objects.iter().enumerate() {
            e.mem.set_u32(table + 4 * i as u32, *o);
        }
        e.mem.set_u32(array + 0x3c, table);
        e.mem.set_u32(array + 0x10 + 4 * level, 0);
        e.mem
            .set_u32(array + 0x20 + 4 * level, objects.len() as u32);
    }

    /// Registers a double on vtable slot `s`: `f(object)` gives the result.
    fn on_slot(e: &mut Engine, s: u32, f: impl Fn(u32) -> u32 + 'static) {
        e.register_double(slot(s), move |_, a| Ret {
            eax: f(a[0]),
            ..Ret::default()
        });
    }

    /// Registers a double returning `f(first argument)` at `addr`.
    fn on_call(e: &mut Engine, addr: u32, f: impl Fn(u32) -> u32 + 'static) {
        e.register_double(addr, move |_, a| Ret {
            eax: f(a.first().copied().unwrap_or(0)),
            ..Ret::default()
        });
    }

    /// Registers a double returning a fixed value at `addr`.
    fn stub(e: &mut Engine, addr: u32, value: u32) {
        on_call(e, addr, move |_| value);
    }

    /// Registers a double returning a `float` in ST0.
    fn stub_f32(e: &mut Engine, addr: u32, value: f32) {
        e.register_double(addr, move |_, _| Ret {
            st0: value as f64,
            ..Ret::default()
        });
    }

    /// Registers doubles that do nothing for each address.
    fn stubs(e: &mut Engine, addrs: &[u32]) {
        for a in addrs {
            stub(e, *a, 0);
        }
    }

    /// The argument words of every logged call to `addr`.
    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    }

    #[test]
    fn reallocate_copies_the_old_elements_into_a_new_scrap_buffer() {
        let mut e = fixture();
        let array = e.mem.alloc(0x20);
        let heap = 0x1111_0000;
        e.mem.set_u32(array + 0x10, heap);
        e.mem.set_u32(array + 0x8, 2);
        e.set_global(0x010a_2720u32, 4u32);
        let old = e.mem.alloc(24);
        let new_buffer = e.mem.alloc(60);
        stub(&mut e, 0x00aa_54a0, new_buffer);
        stubs(&mut e, &[0x0040_1460, 0x00aa_5610]);
        e.call_log = Some(vec![]);
        let result = e
            .call(0x006a_be40, &args![Ptr::<()>::new(array), old, 5u32])
            .u32();
        assert_eq!(result, new_buffer);
        assert_eq!(calls(&e, 0x00aa_54a0), vec![vec![heap, 60, 4]]);
        assert_eq!(calls(&e, 0x0040_1460), vec![vec![new_buffer, old, 24]]);
        assert_eq!(calls(&e, 0x00aa_5610), vec![vec![heap, old]]);
    }

    #[test]
    fn print_lists_does_nothing() {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x008d_0600, &args![lists, 1u32, 2u32]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    /// An actor with animation 0x4000 that wants its lighting updated (`a1`),
    /// a non-actor (`a2`) and an actor without animation (`a3`), in level 1.
    fn animation_world(e: &mut Engine) -> (Ptr<ProcessLists>, u32, u32, u32) {
        let (a1, a2, a3) = (object(e), object(e), object(e));
        let lists = lists_with(e, 1, &[a1, 0, a2, a3]);
        on_slot(e, SLOT_IS_ACTOR, move |o| (o == a1 || o == a3) as u32);
        on_slot(e, 0x1e4, move |o| if o == a1 { 0x4000 } else { 0 });
        e.mem.set_u8(a1 + 0x14e, 1);
        e.mem.set_u8(a3 + 0x14e, 1);
        on_slot(e, SLOT_GET_NODE, |_| 0x777);
        stub(e, 0x0045_0b80, 0x888);
        stubs(e, &[0x00b5_d9f0, 0x0049_6960]);
        (lists, a1, a2, a3)
    }

    #[test]
    fn update_animation_scenegraph_updates_the_lighting_of_changed_actors() {
        let mut e = fixture();
        let (lists, a1, _, _) = animation_world(&mut e);
        stub(&mut e, 0x0049_3930, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_c860, &args![lists]);
        assert_eq!(calls(&e, 0x0049_3930), vec![vec![0x4000]]);
        assert_eq!(calls(&e, slot(SLOT_GET_NODE)), vec![vec![a1, 0]]);
        assert_eq!(calls(&e, 0x00b5_d9f0), vec![vec![0x888, 0x777]]);
        assert_eq!(calls(&e, 0x0049_6960), vec![vec![a1, 0]]);
    }

    #[test]
    fn update_animation_scenegraph_leaves_the_lighting_when_nothing_changed() {
        let mut e = fixture();
        let (lists, a1, _, _) = animation_world(&mut e);
        stub(&mut e, 0x0049_3930, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_c860, &args![lists]);
        assert_eq!(calls(&e, 0x0049_3930).len(), 1);
        assert!(calls(&e, 0x00b5_d9f0).is_empty());
        // And an actor that does not ask for lighting updates is left alone.
        stub(&mut e, 0x0049_3930, 1);
        e.mem.set_u8(a1 + 0x14e, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_c860, &args![lists]);
        assert!(calls(&e, 0x00b5_d9f0).is_empty());
    }

    #[test]
    fn update_lighting_flag_getter_reads_the_actor_byte() {
        let mut e = fixture();
        let actor = object(&mut e);
        assert!(!e.call(0x0096_c950, &args![Ptr::<()>::new(actor)]).bool());
        e.mem.set_u8(actor + 0x14e, 1);
        assert!(e.call(0x0096_c950, &args![Ptr::<()>::new(actor)]).bool());
    }

    /// Doubles for the scrap array of the parallel loops: the push counts
    /// into the array's size (+8) and records the pushed value.
    fn scrap_doubles(e: &mut Engine, push: u32) -> Rc<RefCell<Vec<u32>>> {
        let pushed = Rc::new(RefCell::new(Vec::new()));
        let record = pushed.clone();
        e.register_double(push, move |e, a| {
            let n = e.mem.u32(a[0] + 8);
            e.mem.set_u32(a[0] + 8, n + 1);
            record.borrow_mut().push(e.mem.u32(a[1]));
            Ret::default()
        });
        pushed
    }

    #[test]
    fn parallel_update_runs_directly_when_the_setting_is_off() {
        let mut e = fixture();
        let (lists, a1, _, _) = animation_world(&mut e);
        stub(&mut e, 0x0049_3930, 1);
        stubs(
            &mut e,
            &[0x0097_89e0, 0x0097_8a80, 0x0087_cf10, 0x00c4_4c90],
        );
        let pushed = scrap_doubles(&mut e, 0x007c_b2e0);
        e.mem.set_u8(0x011f_1260 + 0x100, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_c970, &args![lists]);
        assert_eq!(calls(&e, 0x0049_3930), vec![vec![0x4000]]);
        assert!(pushed.borrow().is_empty());
        assert!(calls(&e, 0x0087_cf10).is_empty());
        assert_eq!(calls(&e, 0x0049_6960), vec![vec![a1, 0]]);
        let ctor = calls(&e, 0x0097_89e0);
        assert_eq!(ctor, calls(&e, 0x0097_8a80));
        assert_eq!(ctor.len(), 1);
    }

    #[test]
    fn parallel_update_collects_the_animations_and_queues_one_task() {
        let mut e = fixture();
        let (lists, a1, _, _) = animation_world(&mut e);
        stub(&mut e, 0x0049_3930, 0);
        stubs(
            &mut e,
            &[0x0097_89e0, 0x0097_8a80, 0x0087_cf10, 0x00c4_4c90],
        );
        let pushed = scrap_doubles(&mut e, 0x007c_b2e0);
        e.mem.set_u8(0x011f_1260 + 0x100, 1);
        e.set_global(TASK_MANAGER, 0x5555u32);
        e.call_log = Some(vec![]);
        e.call(0x0096_c970, &args![lists]);
        assert!(calls(&e, 0x0049_3930).is_empty());
        assert_eq!(*pushed.borrow(), vec![0x4000]);
        // The lighting is updated: with the task the "changed" answer is assumed.
        assert_eq!(calls(&e, 0x0049_6960), vec![vec![a1, 0]]);
        let scrap = calls(&e, 0x0097_89e0)[0][0];
        let task = scrap + 0x20;
        assert_eq!(
            calls(&e, 0x0087_cf10),
            vec![vec![task, 0x0096_cb20, scrap, 1]]
        );
        assert_eq!(calls(&e, 0x00c4_4c90), vec![vec![0x5555, task]]);
        assert_eq!(calls(&e, 0x0097_8a80), vec![vec![scrap]]);
    }

    #[test]
    fn parallel_update_with_nothing_to_do_returns_at_once() {
        let mut e = fixture();
        let lists = lists_with(&mut e, 1, &[]);
        e.mem.set_u8(0x011f_1260 + 0x100, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_c970, &args![lists]);
        assert!(calls(&e, 0x0097_89e0).is_empty());
    }

    #[test]
    fn animation_task_updates_the_scenegraph_of_its_element() {
        let mut e = fixture();
        let array = e.mem.alloc(16);
        e.mem.set_u32(array + 8, 0x4000);
        e.register(0x006a_7ad0, |_, a| Ret {
            eax: a[0] + 4 * a[1],
            ..Ret::default()
        });
        stub(&mut e, 0x0049_3930, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_cb20, &args![array, 2u32]);
        assert_eq!(calls(&e, 0x0049_3930), vec![vec![0x4000]]);
    }

    #[test]
    fn ragdoll_loop_handles_each_combination() {
        let mut e = fixture();
        let (a, b, c, d) = (
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
        );
        let lists = lists_with(&mut e, 0, &[a, b, c, d]);
        let (ra, rb, rd) = (e.mem.alloc(0x60), e.mem.alloc(0x60), e.mem.alloc(0x60));
        for (actor, ragdoll) in [(a, ra), (b, rb), (d, rd)] {
            e.mem.set_u32(actor + 0xac, ragdoll);
        }
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, 0x0055_2490, 1);
        on_slot(&mut e, SLOT_GET_NODE, move |o| {
            if o == a {
                0x70
            } else if o == d {
                0x71
            } else {
                0
            }
        });
        on_call(&mut e, 0x0055_2470, |node| (node == 0x71) as u32);
        stub(&mut e, 0x0093_3840, 0);
        stub(&mut e, 0x0088_8970, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_cb50, &args![lists]);
        // `a`: node present, node test fails, not in dialogue: handled.
        assert_eq!(calls(&e, 0x0088_8970), vec![vec![a, 1]]);
        assert_eq!(e.mem.u8(ra + 0x50), 0);
        // `b`: no node; `d`: node test passes: the flag is set instead.
        assert_eq!(e.mem.u8(rb + 0x50), 1);
        assert_eq!(e.mem.u8(rd + 0x50), 1);
    }

    #[test]
    fn ragdoll_loop_in_dialogue_only_sets_the_flag() {
        let mut e = fixture();
        let a = object(&mut e);
        let lists = lists_with(&mut e, 0, &[a]);
        let ragdoll = e.mem.alloc(0x60);
        e.mem.set_u32(a + 0xac, ragdoll);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, 0x0055_2490, 1);
        on_slot(&mut e, SLOT_GET_NODE, |_| 0x70);
        stub(&mut e, 0x0055_2470, 0);
        stub(&mut e, 0x0093_3840, 1);
        stub(&mut e, 0x0088_8970, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_cb50, &args![lists]);
        assert!(calls(&e, 0x0088_8970).is_empty());
        assert_eq!(e.mem.u8(ragdoll + 0x50), 1);
    }

    #[test]
    fn ragdoll_flag_setter_stores_the_byte() {
        let mut e = fixture();
        let ragdoll = e.mem.alloc(0x60);
        e.call(0x0096_cc80, &args![Ptr::<()>::new(ragdoll), 1u8]);
        assert_eq!(e.mem.u8(ragdoll + 0x50), 1);
    }

    /// Actors `a` (processable, wants processing), `b` (processable, does
    /// not want processing), `c` (no cell) and `d` (a model-loader test
    /// holds) for the two "process me" loops.
    fn processable_world(e: &mut Engine) -> (Ptr<ProcessLists>, [u32; 4]) {
        let actors = [object(e), object(e), object(e), object(e)];
        let lists = lists_with(e, 0, &actors);
        let [a, _b, c, d] = actors;
        e.mem.set_u8(a + 0xbc, 1);
        e.mem.set_u8(c + 0xbc, 1);
        e.mem.set_u8(d + 0xbc, 1);
        on_slot(e, SLOT_IS_ACTOR, |_| 1);
        on_slot(e, SLOT_GET_NODE, |_| 0x70);
        on_call(e, 0x0044_0d80, move |o| (o == d) as u32);
        stub(e, 0x0044_0da0, 0);
        on_call(e, 0x008d_6f30, move |o| if o == c { 0 } else { 0x10 });
        stub(e, 0x0045_0ff0, 1);
        (lists, actors)
    }

    #[test]
    fn process_me_loop_calls_slot_268_for_processable_actors() {
        let mut e = fixture();
        let (lists, [a, ..]) = processable_world(&mut e);
        stub(&mut e, slot(0x268), 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_cca0, &args![lists]);
        assert_eq!(calls(&e, slot(0x268)), vec![vec![a, 0.0f32.to_bits()]]);
    }

    #[test]
    fn process_me_task_loop_calls_slot_350_directly_when_the_setting_is_off() {
        let mut e = fixture();
        let (lists, [a, ..]) = processable_world(&mut e);
        stub(&mut e, slot(0x350), 0);
        stubs(
            &mut e,
            &[0x0097_8ae0, 0x0097_8b60, 0x0087_cf10, 0x00c4_4c90],
        );
        let pushed = scrap_doubles(&mut e, 0x007c_b2e0);
        e.mem.set_u8(0x011f_126c + 0x100, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_cda0, &args![lists]);
        assert_eq!(calls(&e, slot(0x350)), vec![vec![a]]);
        assert!(pushed.borrow().is_empty());
        assert!(calls(&e, 0x0087_cf10).is_empty());
        assert_eq!(calls(&e, 0x0097_8ae0).len(), 1);
        assert_eq!(calls(&e, 0x0097_8b60).len(), 1);
    }

    #[test]
    fn process_me_task_loop_queues_a_task_when_the_setting_is_on() {
        let mut e = fixture();
        let (lists, [a, ..]) = processable_world(&mut e);
        stub(&mut e, slot(0x350), 0);
        stubs(
            &mut e,
            &[0x0097_8ae0, 0x0097_8b60, 0x0087_cf10, 0x00c4_4c90],
        );
        let pushed = scrap_doubles(&mut e, 0x007c_b2e0);
        e.mem.set_u8(0x011f_126c + 0x100, 1);
        e.set_global(TASK_MANAGER, 0x5555u32);
        e.call_log = Some(vec![]);
        e.call(0x0096_cda0, &args![lists]);
        assert!(calls(&e, slot(0x350)).is_empty());
        assert_eq!(*pushed.borrow(), vec![a]);
        let scrap = calls(&e, 0x0097_8ae0)[0][0];
        assert_eq!(
            calls(&e, 0x0087_cf10),
            vec![vec![scrap + 0x20, 0x0096_cf60, scrap, 1]]
        );
        assert_eq!(calls(&e, 0x00c4_4c90), vec![vec![0x5555, scrap + 0x20]]);
    }

    #[test]
    fn process_me_task_loop_with_nothing_to_do_returns_at_once() {
        let mut e = fixture();
        let lists = lists_with(&mut e, 0, &[]);
        e.mem.set_u8(0x011f_126c + 0x100, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_cda0, &args![lists]);
        assert!(calls(&e, 0x0097_8ae0).is_empty());
    }

    #[test]
    fn process_me_task_calls_slot_350_of_its_element() {
        let mut e = fixture();
        let actor = object(&mut e);
        let array = e.mem.alloc(16);
        e.mem.set_u32(array + 4, actor);
        e.register(0x006a_7ad0, |_, a| Ret {
            eax: a[0] + 4 * a[1],
            ..Ret::default()
        });
        stub(&mut e, slot(0x350), 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_cf60, &args![array, 1u32]);
        assert_eq!(calls(&e, slot(0x350)), vec![vec![actor]]);
    }

    #[test]
    fn skip_fade_in_loop_goes_from_index_zero_to_the_level_zero_tail() {
        let mut e = fixture();
        let (a, b) = (object(&mut e), object(&mut e));
        let lists = lists_with(&mut e, 0, &[a, b, 0]);
        // The head does not matter: the loop starts at 0.
        e.mem.set_u32(lists.addr() + MOB_PROCESS_ARRAY + 0x10, 2);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o == a) as u32);
        on_call(&mut e, ACTOR_PROCESS, |o| o + 0x1000);
        stub(&mut e, 0x008f_f030, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_cfa0, &args![lists]);
        assert_eq!(calls(&e, 0x008f_f030), vec![vec![a + 0x1000, a]]);
    }

    /// A `BSSimpleList` holding `items`: the head node (returned) holds the
    /// first item and chains the others; an empty list is a zeroed head.
    fn make_list(e: &mut Engine, items: &[u32]) -> u32 {
        let head = e.mem.alloc(8);
        let mut node = head;
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(node, *item);
            if i + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
        head
    }

    /// The items of a list built by `make_list` (stops at an empty item).
    fn list_items(e: &Engine, head: u32) -> Vec<u32> {
        let mut items = Vec::new();
        let mut node = head;
        while node != 0 && e.mem.u32(node) != 0 {
            items.push(e.mem.u32(node));
            node = e.mem.u32(node + 4);
        }
        items
    }

    #[test]
    fn constructor_initializes_the_members_and_returns_this() {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        let base = lists.addr();
        stubs(
            &mut e,
            &[
                ARRAY_CONSTRUCT,
                0x004e_e810,
                0x008c_9330,
                0x0090_5e30,
                0x0086_c6a0,
                0x0040_3d30,
                QUERY_PERFORMANCE_FREQUENCY,
                LOCK_ENTER,
                LOCK_LEAVE,
            ],
        );
        e.mem.set_f32(0x011d_1530 + 0x100, 7.5);
        e.set(lists, ProcessLists::bShowSubtitle, true);
        e.set(lists, ProcessLists::fCrimeUpdateTimer, 5.0);
        e.set(lists, ProcessLists::iActorsCloseToPlayer, 9);
        e.call_log = Some(vec![]);
        let result = e.call(0x0096_d020, &args![lists]).ptr::<ProcessLists>();
        assert_eq!(result, lists);
        for flag in [
            ProcessLists::bRunScheduals,
            ProcessLists::bRunDetection,
            ProcessLists::bProcessHigh,
            ProcessLists::bProcessLow,
            ProcessLists::bProcessMHigh,
            ProcessLists::bProcessMLow,
            ProcessLists::bProcessSche,
        ] {
            assert!(e.get(lists, flag));
        }
        assert!(!e.get(lists, ProcessLists::bShowSubtitle));
        assert_eq!(e.get(lists, ProcessLists::fCrimeUpdateTimer), 0.0);
        assert_eq!(e.get(lists, ProcessLists::iActorsCloseToPlayer), 0);
        assert_eq!(e.get(lists, ProcessLists::fRemoveExcessDeadTimer), 7.5);
        assert_eq!(calls(&e, ARRAY_CONSTRUCT), vec![vec![base + 4]]);
        assert_eq!(
            calls(&e, 0x004e_e810),
            vec![vec![base + 0x58], vec![base + 0x60]]
        );
        assert_eq!(
            calls(&e, LIST_CONSTRUCT),
            [0x68, 0x70, 0x78, 0x80, 0x102e4, 0x10300, 0x10320]
                .iter()
                .map(|o| vec![base + o])
                .collect::<Vec<_>>()
        );
        assert_eq!(calls(&e, 0x008c_9330), vec![vec![base + 0x160]]);
        assert_eq!(calls(&e, 0x0090_5e30), vec![vec![base + 0x10360]]);
        assert_eq!(calls(&e, 0x0086_c6a0), vec![vec![base + 0x103c0, 0, 0x28]]);
        // Only four bytes of the five crime-list slots are cleared.
        assert_eq!(
            calls(&e, 0x0040_3d30),
            vec![vec![base + 0x44, 0, 4], vec![base + 0x88, 0, 200]]
        );
        assert_eq!(calls(&e, LOCK_ENTER), vec![vec![base + 0x10300, 0]]);
        assert_eq!(calls(&e, LOCK_LEAVE), vec![vec![base + 0x10300]]);
        assert_eq!(
            calls(&e, QUERY_PERFORMANCE_FREQUENCY),
            vec![vec![0x011e_0e20]]
        );
    }

    #[test]
    fn destructor_destroys_the_crimes_and_the_members_in_reverse_order() {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        let base = lists.addr();
        let first = make_list(&mut e, &[0x1001, 0x1002]);
        let empty = make_list(&mut e, &[]);
        e.mem.set_u32(base + 0x44, first);
        e.mem.set_u32(base + 0x48, empty);
        stubs(
            &mut e,
            &[
                0x008f_25e0,
                LIST_DELETE,
                LIST_CLEAR,
                0x0055_a2d0,
                0x0090_5e50,
                0x0046_ffb0,
                0x008c_9430,
                0x004e_e840,
                ARRAY_DESTRUCT,
            ],
        );
        e.call_log = Some(vec![]);
        e.call(0x0096_d290, &args![lists]);
        let log: Vec<(u32, Vec<u32>)> = e.call_log.take().unwrap();
        let crimes: Vec<u32> = log
            .iter()
            .filter(|(a, _)| *a == 0x008f_25e0)
            .map(|(_, w)| w[0])
            .collect();
        assert_eq!(crimes, vec![0x1001, 0x1002]);
        // Both non-null lists are deleted (the real pop-front also frees nodes).
        let deletes: Vec<&Vec<u32>> = log
            .iter()
            .filter(|(a, w)| *a == LIST_DELETE && (w[0] == first || w[0] == empty))
            .map(|(_, w)| w)
            .collect();
        assert_eq!(deletes, vec![&vec![first, 1], &vec![empty, 1]]);
        // After the crime lists: the members, last constructed first.
        let tail: Vec<(u32, u32)> = log
            .iter()
            .skip_while(|(a, w)| !(*a == LIST_CLEAR && w[0] == base + 0x70))
            .map(|(a, w)| (*a, w[0] - base))
            .collect();
        assert_eq!(
            tail,
            vec![
                (LIST_CLEAR, 0x70),
                (LIST_CLEAR, 0x80),
                (0x0055_a2d0, 0x103c0),
                (0x0090_5e50, 0x10360),
                (0x0046_ffb0, 0x102e4),
                (0x008c_9430, 0x160),
                (0x0046_ffb0, 0x80),
                (0x0046_ffb0, 0x78),
                (0x0046_ffb0, 0x70),
                (0x0046_ffb0, 0x68),
                (0x004e_e840, 0x60),
                (0x004e_e840, 0x58),
                (ARRAY_DESTRUCT, 4),
            ]
        );
    }

    #[test]
    fn add_and_remove_reference_forward_to_the_process_array() {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        stubs(&mut e, &[ARRAY_ADD_ACTOR, ARRAY_REMOVE_ACTOR]);
        e.call_log = Some(vec![]);
        e.call(0x0096_d450, &args![lists, 0xa1u32, 2u32, 7u32, 8u32, 9u32]);
        e.call(0x0096_d470, &args![lists, 0xa2u32, 3u32]);
        assert_eq!(
            calls(&e, ARRAY_ADD_ACTOR),
            vec![vec![lists.addr() + 4, 0xa1, 2]]
        );
        assert_eq!(
            calls(&e, ARRAY_REMOVE_ACTOR),
            vec![vec![lists.addr() + 4, 0xa2, 3]]
        );
    }

    /// Doubles for the CRT checks `SetSystemTimeClock` makes.
    fn clock_engine() -> Engine {
        let mut e = fixture();
        e.register(0x00ec_75b1, |_, a| Ret {
            eax: f64::take(a, &mut 0).is_nan() as u32,
            ..Ret::default()
        });
        e.register(0x00ec_7595, |_, a| Ret {
            eax: f64::take(a, &mut 0).is_finite() as u32,
            ..Ret::default()
        });
        e
    }

    #[test]
    fn system_time_clock_round_trips_ordinary_values() {
        let mut e = clock_engine();
        let lists: Ptr<ProcessLists> = e.new_object();
        e.call(0x0096_d4b0, &args![lists, 1234.5f32]);
        assert_eq!(e.call(0x0096_d490, &args![lists]).f32(), 1234.5);
    }

    #[test]
    fn system_time_clock_resets_values_that_are_too_big_or_not_numbers() {
        let mut e = clock_engine();
        let lists: Ptr<ProcessLists> = e.new_object();
        for bad in [100001.0f32, f32::NAN, f32::NEG_INFINITY, f32::INFINITY] {
            e.call(0x0096_d4b0, &args![lists, bad]);
            assert_eq!(e.call(0x0096_d490, &args![lists]).f32(), 0.0, "{bad}");
        }
        // The limit itself is still fine.
        e.call(0x0096_d4b0, &args![lists, 100000.0f32]);
        assert_eq!(e.call(0x0096_d490, &args![lists]).f32(), 100000.0);
    }

    /// The doubles `fn_0096d520` needs: the player `p`, whose follower list
    /// (`followers`) holds `f1` (package of type 1), `f2` (no package) and
    /// `f3` (package of type 5); every actor has a process and `f1` a
    /// process level.
    fn follower_world(e: &mut Engine) -> (u32, [u32; 3], u32) {
        let p = object(e);
        e.set_global(PLAYER, p);
        let actors = [object(e), object(e), object(e)];
        let [f1, _f2, f3] = actors;
        let head = make_list(e, &actors);
        let followers = e.mem.alloc(0x20);
        e.mem.set_u32(followers + 0xc, head);
        on_call(e, EXTRA_DATA_LIST, |o| o);
        on_call(
            e,
            EXTRA_FOLLOWERS,
            move |x| if x == p { followers } else { 0 },
        );
        on_call(e, ACTOR_PROCESS, |o| o + 0x1000);
        on_call(e, 0x0088_1510, move |o| {
            if o == f1 {
                0x5001
            } else if o == f3 {
                0x5002
            } else {
                0
            }
        });
        stub(e, 0x0041_cb10, 0);
        on_call(e, PACKAGE_TYPE, |pkg| if pkg == 0x5001 { 1 } else { 5 });
        on_call(e, ACTOR_PROCESS_LEVEL, move |o| if o == f1 { 2 } else { 0 });
        stub(e, slot(0x434), 0);
        stub(e, slot(0x25c), 0);
        stubs(e, &[0x0088_1680, 0x0042_2690, 0x0042_2720]);
        // `00905820` is not translated yet; the copy is assumed to append at
        // the tail (the order of the copied followers is kept).
        e.register(LIST_ADD_ITEM, |e, a| {
            let value = e.mem.u32(a[1]);
            if e.mem.u32(a[0]) == 0 {
                e.mem.set_u32(a[0], value);
            } else {
                let mut node = a[0];
                while e.mem.u32(node + 4) != 0 {
                    node = e.mem.u32(node + 4);
                }
                let added = e.mem.alloc(8);
                e.mem.set_u32(added, value);
                e.mem.set_u32(node + 4, added);
            }
            Ret::default()
        });
        (p, actors, head)
    }

    #[test]
    fn follower_dismissal_drops_followers_without_a_follow_package() {
        let mut e = fixture();
        let (p, [f1, f2, f3], _) = follower_world(&mut e);
        let cleared = Rc::new(RefCell::new(Vec::new()));
        let record = cleared.clone();
        e.register_double(LIST_CLEAR, move |e, a| {
            record.borrow_mut().push(list_items(e, a[0]));
            Ret::default()
        });
        e.call_log = Some(vec![]);
        e.call(0x0096_d520, &args![]);
        // The follower with a follow package is told to stop what it does.
        assert_eq!(calls(&e, slot(0x434)), vec![vec![f1, 0]]);
        assert_eq!(calls(&e, 0x0088_1680), vec![vec![f1, 0]]);
        assert_eq!(calls(&e, slot(0x25c)), vec![vec![f1, 3600.0f32.to_bits()]]);
        // The others are removed from the player's followers (the dismissed
        // list has the last pushed first).
        assert_eq!(calls(&e, 0x0042_2690), vec![vec![p, f3], vec![p, f2]]);
        // The dismissed list is cleared first, then the copy of the followers.
        // The deleting destructor of a list clears it again: keep each once.
        let mut cleared_lists = cleared.borrow().clone();
        cleared_lists.dedup();
        assert_eq!(cleared_lists, vec![vec![f3, f2], vec![f1, f2, f3]]);
        // The player still has followers: the follower extra stays.
        assert!(calls(&e, 0x0042_2720).is_empty());
    }

    #[test]
    fn follower_dismissal_removes_the_follower_extra_when_none_are_left() {
        let mut e = fixture();
        let (p, _, _) = follower_world(&mut e);
        // A player whose follower list head is empty.
        let followers = e.mem.alloc(0x20);
        let empty = make_list(&mut e, &[]);
        e.mem.set_u32(followers + 0xc, empty);
        on_call(
            &mut e,
            EXTRA_FOLLOWERS,
            move |x| if x == p { followers } else { 0 },
        );
        e.call_log = Some(vec![]);
        e.call(0x0096_d520, &args![]);
        assert!(calls(&e, 0x0042_2690).is_empty());
        assert_eq!(calls(&e, 0x0042_2720), vec![vec![p]]);
    }

    #[test]
    fn follower_dismissal_without_a_follower_record_does_nothing() {
        let mut e = fixture();
        let p = object(&mut e);
        e.set_global(PLAYER, p);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o);
        stub(&mut e, EXTRA_FOLLOWERS, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_d520, &args![]);
        assert_eq!(e.call_log.take().unwrap().len(), 3);
    }

    /// Gives each actor a process object (with the common vtable) and
    /// registers the double of `ACTOR_PROCESS` that returns it (0 for any
    /// other object). Returns the processes in the order of `actors`.
    fn give_processes(e: &mut Engine, actors: &[u32]) -> Vec<u32> {
        let mut table = HashMap::new();
        let mut processes = Vec::new();
        for actor in actors {
            let process = object(e);
            table.insert(*actor, process);
            processes.push(process);
        }
        on_call(e, ACTOR_PROCESS, move |o| {
            table.get(&o).copied().unwrap_or(0)
        });
        processes
    }

    /// `UpdateProcessLists` with the callees doubled: three actors in level
    /// 0, the frame time `dt` and the calendar at 13.5 h of 9/3/2281.
    fn update_engine(dt: f32) -> (Engine, Ptr<ProcessLists>) {
        let mut e = clock_engine();
        let actors = [object(&mut e), object(&mut e), object(&mut e)];
        let lists = lists_with(&mut e, 0, &actors);
        stub_f32(&mut e, 0x0084_d030, dt);
        e.register(FTOL, |_, a| Ret {
            eax: f64::take(a, &mut 0) as i32 as u32,
            ..Ret::default()
        });
        stub_f32(&mut e, 0x0086_7da0, 13.5);
        stub(&mut e, 0x0086_7c60, 2281);
        stub(&mut e, 0x0086_7d20, 9);
        stub(&mut e, 0x0086_7d60, 3);
        stubs(
            &mut e,
            &[
                0x0040_4eb0,
                0x0040_4ee0,
                0x008d_5bc0,
                0x0097_7cb0,
                0x008d_1560,
                0x0047_72f0,
                LOCK_ENTER,
                LOCK_LEAVE,
                0x0096_b050,
                0x0096_b470,
                0x0096_b810,
                0x0097_23f0,
            ],
        );
        stub(&mut e, 0x0096_40b0, 0x111);
        stub(&mut e, 0x0096_4060, 0x222);
        stub(&mut e, 0x0052_5420, 0);
        stub(&mut e, LOCK_TRY_ENTER, 1);
        e.set_global(PLAYER, 0x7000u32);
        e.mem.set_u32(0x011c_3ea4 + 0x100, 1);
        e.set_global(SYSTEM_TIME_CLOCK, 10.0f32);
        for flag in [
            ProcessLists::bProcessLow,
            ProcessLists::bProcessMLow,
            ProcessLists::bProcessMHigh,
        ] {
            e.set(lists, flag, true);
        }
        (e, lists)
    }

    #[test]
    fn update_runs_every_pass_at_a_good_frame_rate() {
        let dt = 0.01f32;
        let (mut e, lists) = update_engine(dt);
        e.set(lists, ProcessLists::fPlayerActionCommentTimer, 5.0);
        e.set(lists, ProcessLists::fPlayerKnockObjectCommentTimer, -1.0);
        e.set(lists, ProcessLists::fCrimeUpdateTimer, 3.0);
        e.set(lists, ProcessLists::fSecondsPassedNoProcess, 0.25);
        e.call_log = Some(vec![]);
        e.call(0x0096_d810, &args![lists]);
        // The profiling scope wraps the work.
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1].0, 0x0040_4eb0);
        assert_eq!(log[1].1[1..], [0x2a, 1, SOURCE_FILE, 0x521]);
        assert_eq!(log.last().unwrap().0, 0x0040_4ee0);
        e.call_log = Some(log);
        let tick = |value: f32| (value as f64 - dt as f64) as f32;
        assert_eq!(
            e.get(lists, ProcessLists::fPlayerActionCommentTimer),
            tick(5.0)
        );
        assert_eq!(
            e.get(lists, ProcessLists::fPlayerKnockObjectCommentTimer),
            -1.0
        );
        assert_eq!(e.get(lists, ProcessLists::fCrimeUpdateTimer), tick(3.0));
        assert_eq!(
            e.get(lists, ProcessLists::fSecondsPassedNoProcess),
            (0.25f32 as f64 + dt as f64) as f32
        );
        assert_eq!(e.get(lists, ProcessLists::iNumberHighActors), 3);
        assert_eq!(e.global::<f32>(GAME_HOUR), 13.5);
        assert_eq!(e.global::<i32>(GAME_YEAR), 2281);
        assert_eq!(e.global::<i32>(GAME_MONTH), 9);
        assert_eq!(e.global::<u8>(GAME_DAY), 3);
        assert_eq!(
            e.global::<f32>(SYSTEM_TIME_CLOCK),
            (10.0f32 as f64 + dt as f64) as f32
        );
        assert_eq!(calls(&e, 0x0096_40b0), vec![vec![0x7000, 0]]);
        assert_eq!(calls(&e, 0x0096_4060), vec![vec![0x7000, 0x111]]);
        assert_eq!(calls(&e, 0x0097_7cb0), vec![vec![lists.addr(), 0x222]]);
        assert_eq!(calls(&e, 0x0047_72f0).len(), 1);
        let pass = vec![lists.addr(), 0, 0];
        assert_eq!(calls(&e, 0x0096_b050), vec![pass.clone()]);
        assert_eq!(calls(&e, 0x0096_b470), vec![pass.clone()]);
        assert_eq!(calls(&e, 0x0096_b810), vec![pass]);
        assert_eq!(calls(&e, LOCK_TRY_ENTER), vec![vec![LOCK]]);
        assert_eq!(calls(&e, LOCK_ENTER), vec![vec![LOCK, 0]; 2]);
        assert_eq!(calls(&e, LOCK_LEAVE), vec![vec![LOCK]; 3]);
        assert!(calls(&e, 0x0097_23f0).is_empty());
    }

    #[test]
    fn update_respects_the_switches_the_loading_state_and_the_crime_timer() {
        let (mut e, lists) = update_engine(0.01);
        e.set(lists, ProcessLists::bProcessLow, false);
        e.set(lists, ProcessLists::bProcessMHigh, false);
        e.set(lists, ProcessLists::fCrimeUpdateTimer, 0.0);
        e.mem.set_u32(0x011c_3ea4 + 0x100, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_d810, &args![lists]);
        assert!(calls(&e, 0x0096_b050).is_empty());
        assert!(calls(&e, 0x0096_b810).is_empty());
        assert_eq!(calls(&e, 0x0096_b470).len(), 1);
        assert!(calls(&e, 0x0047_72f0).is_empty());
        // The crime timer ran out: the crime update runs instead of ticking.
        assert_eq!(calls(&e, 0x0097_23f0), vec![vec![lists.addr()]]);
        assert_eq!(e.get(lists, ProcessLists::fCrimeUpdateTimer), 0.0);
        // While the game is loading, the low and middle-low passes are skipped.
        e.set(lists, ProcessLists::bProcessLow, true);
        stub(&mut e, 0x0052_5420, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_d810, &args![lists]);
        assert!(calls(&e, 0x0096_b050).is_empty());
        assert!(calls(&e, 0x0096_b470).is_empty());
    }

    #[test]
    fn update_skips_the_passes_on_some_frames_when_the_frame_rate_is_low() {
        // 20 frames per second: the passes wait until a second has passed.
        let dt = 0.05f32;
        let (mut e, lists) = update_engine(dt);
        e.call_log = Some(vec![]);
        e.call(0x0096_d810, &args![lists]);
        let passed = (0.0f32 as f64 + dt as f64) as f32;
        assert_eq!(
            e.get(lists, ProcessLists::fSecondsPassedNoProcess),
            ((1.0 - dt as f64) / 2.0 + passed as f64) as f32
        );
        assert!(calls(&e, 0x0096_b050).is_empty());
        e.set(lists, ProcessLists::fSecondsPassedNoProcess, 1.0);
        e.call_log = Some(vec![]);
        e.call(0x0096_d810, &args![lists]);
        assert_eq!(e.get(lists, ProcessLists::fSecondsPassedNoProcess), 0.0);
        assert_eq!(calls(&e, 0x0096_b050).len(), 1);

        // 2 frames per second: the time just accumulates until a second.
        let (mut e, lists) = update_engine(0.5);
        e.call_log = Some(vec![]);
        e.call(0x0096_d810, &args![lists]);
        assert_eq!(e.get(lists, ProcessLists::fSecondsPassedNoProcess), 1.0);
        assert!(calls(&e, 0x0096_b050).is_empty());
        e.call_log = Some(vec![]);
        e.call(0x0096_d810, &args![lists]);
        assert_eq!(e.get(lists, ProcessLists::fSecondsPassedNoProcess), 0.0);
        assert_eq!(calls(&e, 0x0096_b050).len(), 1);
    }

    /// The world `fn_0096db30` works on: the global `ProcessLists` holds
    /// `a` (an actor), `b` (a non-actor whose slot 0x224 is true), `c` (a
    /// non-actor whose slot 0x220 is true) and `d` (skipped by `00576d30`)
    /// in `level`.
    fn update_by_level_world(e: &mut Engine, level: u32) -> [u32; 4] {
        let objects = [object(e), object(e), object(e), object(e)];
        let [a, b, c, d] = objects;
        set_objects(e, PROCESS_LISTS + 4, level, &objects);
        on_call(e, 0x0057_6d30, move |o| (o == d) as u32);
        on_slot(e, SLOT_IS_ACTOR, move |o| (o == a) as u32);
        on_slot(e, 0x224, move |o| (o == b) as u32);
        on_slot(e, 0x220, move |o| (o == c) as u32);
        on_call(e, ACTOR_PROCESS, |o| o + 0x1000);
        stubs(e, &[slot(0x348), 0x008e_51b0, 0x009b_ec10, 0x009a_e580]);
        [a, b, c, d]
    }

    #[test]
    fn update_by_level_zero_uses_the_given_time_and_follows_with_the_process() {
        let mut e = fixture();
        let [a, b, c, _] = update_by_level_world(&mut e, 0);
        let lists: Ptr<ProcessLists> = Ptr::new(PROCESS_LISTS);
        e.call_log = Some(vec![]);
        e.call(0x0096_db30, &args![lists, 0.25f32, 1u8, 0u32]);
        assert_eq!(calls(&e, slot(0x348)), vec![vec![a, 0.25f32.to_bits(), 1]]);
        assert_eq!(calls(&e, 0x008e_51b0), vec![vec![a + 0x1000, a]]);
        assert_eq!(calls(&e, 0x009b_ec10), vec![vec![b, 0]]);
        assert_eq!(calls(&e, 0x009a_e580), vec![vec![c, 0]]);
    }

    #[test]
    fn update_by_level_above_zero_computes_the_time_from_the_counter() {
        let mut e = fixture();
        let [a, ..] = update_by_level_world(&mut e, 2);
        let lists: Ptr<ProcessLists> = Ptr::new(PROCESS_LISTS);
        stub(&mut e, 0x0082_5c00, 5500);
        stub_f32(&mut e, 0x007d_f1f0, 2.0);
        e.call_log = Some(vec![]);
        e.call(0x0096_db30, &args![lists, 0.25f32, 0u8, 2u32]);
        assert_eq!(calls(&e, 0x0082_5c00), vec![vec![FRAME_TIMER]]);
        assert_eq!(calls(&e, 0x007d_f1f0), vec![vec![a + 0x1000]]);
        assert_eq!(calls(&e, slot(0x348)), vec![vec![a, 3.5f32.to_bits(), 0]]);
        assert!(calls(&e, 0x008e_51b0).is_empty());
    }

    /// Doubles for the fast-travel advance: the actor's process has last
    /// been updated at `last` o'clock, the calendar says `hour`.
    fn fast_travel_engine(last: f32, hour: f32) -> Engine {
        let mut e = fixture();
        stub_f32(&mut e, 0x007d_f1f0, last);
        stub_f32(&mut e, 0x0086_7da0, hour);
        stubs(&mut e, &[slot(0x25c), 0x0088_42c0, slot(SLOT_0X22C)]);
        e
    }

    /// Runs `fn_0096dcb0` over the list `[a, b, c]` where `a` and `c` are
    /// actors and `c` is the player; returns the actor that was advanced.
    fn advance_through_list(e: &mut Engine) -> Vec<Vec<u32>> {
        let (a, b, c) = (object(e), object(e), object(e));
        e.set_global(PLAYER, c);
        give_processes(e, &[a, c]);
        on_slot(e, SLOT_IS_ACTOR, move |o| (o == a || o == c) as u32);
        stubs(e, &[0x0044_0d80, 0x0044_0da0, 0x0049_3bb0]);
        let list = make_list(e, &[a, b, c]);
        let lists: Ptr<ProcessLists> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0096_dcb0, &args![lists, list]);
        calls(e, slot(0x25c))
    }

    #[test]
    fn fast_travel_advance_passes_the_hours_since_the_last_update() {
        let mut e = fast_travel_engine(20.0, 22.5);
        let advanced = advance_through_list(&mut e);
        assert_eq!(advanced.len(), 1);
        assert_eq!(advanced[0][1], 2.5f32.to_bits());
    }

    #[test]
    fn fast_travel_advance_wraps_around_midnight() {
        let mut e = fast_travel_engine(23.0, 1.0);
        let advanced = advance_through_list(&mut e);
        assert_eq!(advanced[0][1], 2.0f32.to_bits());
    }

    #[test]
    fn fast_travel_advance_rounds_tiny_times_down_to_zero() {
        let mut e = fast_travel_engine(22.49, 22.5);
        let advanced = advance_through_list(&mut e);
        assert_eq!(advanced[0][1], 0.0f32.to_bits());
    }

    #[test]
    fn fast_travel_advance_skips_followers_whose_target_is_the_player() {
        let mut e = fast_travel_engine(20.0, 22.5);
        let (a, c) = (object(&mut e), object(&mut e));
        e.set_global(PLAYER, c);
        let processes = give_processes(&mut e, &[a, c]);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o == a || o == c) as u32);
        stubs(&mut e, &[0x0044_0d80, 0x0044_0da0, 0x0049_3bb0]);
        // The process runs a package of type 2 and its target is the player.
        stub(&mut e, slot(SLOT_0X22C), 0x5001);
        stub(&mut e, PACKAGE_TYPE, 2);
        stub(&mut e, slot(0x128), c);
        let list = make_list(&mut e, &[a]);
        let lists: Ptr<ProcessLists> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0096_dcb0, &args![lists, list]);
        assert!(calls(&e, slot(0x25c)).is_empty());
        assert_eq!(calls(&e, slot(0x128)), vec![vec![processes[0]]]);
        // Following somebody else: advanced.
        stub(&mut e, slot(0x128), 0x4242);
        let list = make_list(&mut e, &[a]);
        e.call_log = Some(vec![]);
        e.call(0x0096_dcb0, &args![lists, list]);
        assert_eq!(calls(&e, slot(0x25c)).len(), 1);
    }

    #[test]
    fn fast_travel_high_actor_loop_picks_the_actors_that_qualify() {
        let mut e = fast_travel_engine(10.0, 12.0);
        let objects: Vec<u32> = (0..8).map(|_| object(&mut e)).collect();
        let [a, b, skipped, no_place, player_actor, _no_process, model, world_space] = objects[..]
        else {
            unreachable!()
        };
        let lists = lists_with(&mut e, 0, &objects);
        e.set_global(PLAYER, player_actor);
        give_processes(
            &mut e,
            &[a, skipped, no_place, player_actor, model, world_space],
        );
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != b) as u32);
        on_call(&mut e, 0x0057_6d30, move |o| (o == skipped) as u32);
        on_call(&mut e, 0x008d_6f30, move |o| {
            (o != no_place && o != world_space) as u32
        });
        on_call(&mut e, 0x0057_5d70, move |o| (o == world_space) as u32);
        on_call(&mut e, 0x0044_0d80, move |o| (o == model) as u32);
        stubs(&mut e, &[0x0044_0da0, 0x0049_3bb0]);
        e.call_log = Some(vec![]);
        e.call(0x0096_df40, &args![lists]);
        let advanced: Vec<u32> = calls(&e, slot(0x25c)).iter().map(|w| w[0]).collect();
        assert_eq!(advanced, vec![a, world_space]);
        assert_eq!(calls(&e, slot(0x25c))[0][1], 2.0f32.to_bits());
    }

    #[test]
    fn temp_change_walk_adds_mobile_objects_when_the_flag_is_clear() {
        let mut e = fixture();
        let (a, b, c) = (object(&mut e), object(&mut e), object(&mut e));
        let lists: Ptr<ProcessLists> = e.new_object();
        // `a` and `c` are mobile objects, but not actors: they are flagged and
        // added to the temp list; `b` is ignored.
        on_slot(&mut e, SLOT_IS_MOBILE_OBJECT, move |o| (o != b) as u32);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 0);
        stub(&mut e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS), 0);
        let list = make_list(&mut e, &[a, b, c]);
        e.call_log = Some(vec![]);
        e.call(0x0096_e150, &args![lists, list, 0u8]);
        assert_eq!(
            calls(&e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS)),
            vec![vec![a, 1], vec![c, 1]]
        );
        // The last added is first in the list (the push goes to the front).
        let temp = lists.addr() + TEMP_SHOULD_MOVE_LIST;
        assert_eq!(list_items(&e, temp), vec![c, a]);
    }

    #[test]
    fn temp_change_walk_releases_loaded_model_actors_and_retries_unready_ones() {
        let mut e = fixture();
        let (model, busy) = (object(&mut e), object(&mut e));
        e.set_global(PLAYER, 0x7000u32);
        let lists: Ptr<ProcessLists> = e.new_object();
        give_processes(&mut e, &[model, busy]);
        on_slot(&mut e, SLOT_IS_MOBILE_OBJECT, |_| 1);
        on_call(&mut e, 0x0044_0d80, move |o| (o == model) as u32);
        stub(&mut e, 0x0044_0da0, 0);
        stub(&mut e, LOCK_TRY_ENTER, 1);
        on_call(&mut e, ACTOR_PROCESS_LEVEL, |_| 2);
        stubs(&mut e, &[ARRAY_REMOVE_ACTOR, 0x0093_1e80, LOCK_LEAVE]);
        // `busy` is not ready the first time (slot 0x260 false), then ready.
        let attempts = Rc::new(RefCell::new(0));
        let counter = attempts.clone();
        e.register_double(slot(0x260), move |_, _| {
            *counter.borrow_mut() += 1;
            Ret {
                eax: (*counter.borrow() > 1) as u32,
                ..Ret::default()
            }
        });
        let list = make_list(&mut e, &[model, busy]);
        e.call_log = Some(vec![]);
        e.call(0x0096_e150, &args![lists, list, 1u8]);
        // The first node is handled twice because the walk restarted.
        assert_eq!(
            calls(&e, ARRAY_REMOVE_ACTOR),
            vec![vec![lists.addr() + 4, model, 2]; 2]
        );
        assert_eq!(calls(&e, 0x0093_1e80), vec![vec![model]; 2]);
        assert_eq!(calls(&e, slot(0x260)).len(), 2);
        assert_eq!(calls(&e, LOCK_LEAVE).len(), 2);
    }

    #[test]
    fn alive_actor_list_accessors() {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        assert_eq!(
            e.call(0x0096_e290, &args![lists]).u32(),
            lists.addr() + 0x80
        );
        let list = lists.addr() + ALIVE_ACTOR_LIST;
        e.call(0x0096_e2b0, &args![lists, 0u32]);
        assert!(list_items(&e, list).is_empty());
        e.call(0x0096_e2b0, &args![lists, 0xa1u32]);
        e.call(0x0096_e2b0, &args![lists, 0xa2u32]);
        e.call(0x0096_e2b0, &args![lists, 0xa1u32]);
        assert_eq!(list_items(&e, list), vec![0xa2, 0xa1]);
        e.call(0x0096_e2f0, &args![lists, 0xa2u32]);
        assert_eq!(list_items(&e, list), vec![0xa1]);
    }

    #[test]
    fn alive_actor_list_is_rebuilt_from_level_zero() {
        let mut e = fixture();
        let objects = [object(&mut e), object(&mut e), object(&mut e)];
        let [alive, knocked_out, not_actor] = objects;
        set_objects(&mut e, PROCESS_LISTS + 4, 0, &objects);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != not_actor) as u32);
        on_slot(&mut e, SLOT_0X22C, move |o| (o == knocked_out) as u32);
        let lists: Ptr<ProcessLists> = e.new_object();
        let list = lists.addr() + ALIVE_ACTOR_LIST;
        e.mem.set_u32(list, 0x9999);
        e.call_log = Some(vec![]);
        e.call(0x0096_e310, &args![lists]);
        assert_eq!(list_items(&e, list), vec![alive]);
        assert_eq!(calls(&e, slot(SLOT_0X22C))[0][1], 0);
    }

    /// `memmove_s` double (dest, dest size, source, count).
    fn memmove_double(e: &mut Engine) {
        e.register(MEMMOVE, |e, a| {
            let bytes = e.mem.bytes(a[2], a[3]);
            e.mem.write(a[0], &bytes);
            Ret::default()
        });
    }

    /// Doubles for the close-to-player array: actor `n` (1..) has distance
    /// `distances[n]`; its character controller is the actor plus one, and an
    /// actor listed in `no_controller` has none. The setting caps the count at
    /// `limit`.
    fn close_to_player_engine(
        distances: &[(u32, f32)],
        no_controller: &[u32],
        limit: i32,
    ) -> Engine {
        let mut e = fixture();
        let distances: HashMap<u32, f32> = distances.iter().map(|(a, d)| (a + 1, *d)).collect();
        let none = no_controller.to_vec();
        on_call(&mut e, 0x0093_06d0, move |a| {
            if none.contains(&a) {
                0
            } else {
                a + 1
            }
        });
        e.register_double(0x008a_3b50, move |_, a| Ret {
            st0: distances[&a[0]] as f64,
            ..Ret::default()
        });
        e.mem.set_i32(MAX_ACTORS_SETTING + 0x100, limit);
        stubs(&mut e, &[LOCK_ENTER, LOCK_LEAVE]);
        memmove_double(&mut e);
        e
    }

    /// A `ProcessLists` whose close-to-player array holds `actors`.
    fn lists_close_to_player(e: &mut Engine, actors: &[u32]) -> Ptr<ProcessLists> {
        let lists: Ptr<ProcessLists> = e.new_object();
        for (i, actor) in actors.iter().enumerate() {
            e.mem
                .set_u32(lists.addr() + ACTORS_CLOSE_TO_PLAYER + 4 * i as u32, *actor);
        }
        e.set(
            lists,
            ProcessLists::iActorsCloseToPlayer,
            actors.len() as i32,
        );
        lists
    }

    fn close_to_player(e: &Engine, lists: Ptr<ProcessLists>) -> Vec<u32> {
        let count = e.get(lists, ProcessLists::iActorsCloseToPlayer);
        (0..count.max(0) as u32)
            .map(|i| e.mem.u32(lists.addr() + ACTORS_CLOSE_TO_PLAYER + 4 * i))
            .collect()
    }

    #[test]
    fn insert_close_to_player_keeps_the_array_sorted_by_distance() {
        let mut e = close_to_player_engine(&[(1, 10.0), (2, 20.0), (4, 40.0), (3, 30.0)], &[], 5);
        let lists = lists_close_to_player(&mut e, &[1, 2, 4]);
        e.call_log = Some(vec![]);
        e.call(0x0096_e3c0, &args![lists, 3u32]);
        assert_eq!(close_to_player(&e, lists), vec![1, 2, 3, 4]);
        let slot2 = lists.addr() + ACTORS_CLOSE_TO_PLAYER + 8;
        assert_eq!(calls(&e, MEMMOVE), vec![vec![slot2 + 4, 200, slot2, 188]]);
        assert_eq!(calls(&e, LOCK_ENTER).len(), 1);
        assert_eq!(calls(&e, LOCK_LEAVE).len(), 1);
    }

    #[test]
    fn insert_close_to_player_ignores_known_and_controllerless_actors() {
        let mut e = close_to_player_engine(&[(1, 10.0), (2, 20.0), (3, 30.0)], &[3], 5);
        let lists = lists_close_to_player(&mut e, &[1, 2]);
        e.call(0x0096_e3c0, &args![lists, 2u32]);
        e.call(0x0096_e3c0, &args![lists, 0u32]);
        e.call_log = Some(vec![]);
        e.call(0x0096_e3c0, &args![lists, 3u32]);
        assert_eq!(close_to_player(&e, lists), vec![1, 2]);
        assert!(calls(&e, LOCK_ENTER).is_empty());
    }

    #[test]
    fn insert_close_to_player_respects_the_cap_and_skips_entries_without_controller() {
        // Cap 3: an actor closer than everybody pushes the farthest out of
        // the counted range, a farther one is not inserted.
        let mut e = close_to_player_engine(
            &[
                (1, 10.0),
                (2, 20.0),
                (3, 30.0),
                (4, 5.0),
                (5, 50.0),
                (6, 25.0),
            ],
            &[2],
            3,
        );
        let lists = lists_close_to_player(&mut e, &[1, 2, 3]);
        e.call(0x0096_e3c0, &args![lists, 5u32]);
        assert_eq!(close_to_player(&e, lists), vec![1, 2, 3]);
        e.call(0x0096_e3c0, &args![lists, 4u32]);
        assert_eq!(close_to_player(&e, lists), vec![4, 1, 2]);
        // The entry without a controller (2) is skipped in the comparison:
        // 25 goes before 3 (30), not before the controllerless one.
        let mut e = close_to_player_engine(&[(1, 10.0), (2, 20.0), (3, 30.0), (6, 25.0)], &[2], 5);
        let lists = lists_close_to_player(&mut e, &[1, 2, 3]);
        e.call(0x0096_e3c0, &args![lists, 6u32]);
        assert_eq!(close_to_player(&e, lists), vec![1, 2, 6, 3]);
    }

    #[test]
    fn sort_close_to_player_orders_by_the_compare_function_and_trims() {
        let mut e = fixture();
        let order = [9u32, 3, 12, 1, 7, 11, 5, 2, 10, 4, 8, 6];
        let lists = lists_close_to_player(&mut e, &order);
        // The compare function: smaller number first.
        e.register(0x0097_0ee0, |_, a| Ret {
            eax: (a[0] as i32 - a[1] as i32) as u32,
            ..Ret::default()
        });
        e.mem.set_i32(MAX_ACTORS_SETTING + 0x100, 10);
        stubs(&mut e, &[LOCK_ENTER, LOCK_LEAVE]);
        e.call_log = Some(vec![]);
        e.call(0x0096_e570, &args![lists]);
        assert_eq!(close_to_player(&e, lists), (1..=10).collect::<Vec<u32>>());
        // The two entries beyond the cap are cleared.
        assert_eq!(e.mem.u32(lists.addr() + ACTORS_CLOSE_TO_PLAYER + 40), 0);
        assert_eq!(e.mem.u32(lists.addr() + ACTORS_CLOSE_TO_PLAYER + 44), 0);
        assert_eq!(calls(&e, LOCK_ENTER).len(), 1);
        assert_eq!(calls(&e, LOCK_LEAVE).len(), 1);
    }

    #[test]
    fn sort_close_to_player_handles_short_arrays() {
        let mut e = fixture();
        let lists = lists_close_to_player(&mut e, &[2, 1]);
        e.register(0x0097_0ee0, |_, a| Ret {
            eax: (a[0] as i32 - a[1] as i32) as u32,
            ..Ret::default()
        });
        e.mem.set_i32(MAX_ACTORS_SETTING + 0x100, 10);
        stubs(&mut e, &[LOCK_ENTER, LOCK_LEAVE]);
        e.call(0x0096_e570, &args![lists]);
        assert_eq!(close_to_player(&e, lists), vec![1, 2]);
        // An empty array is left alone.
        let empty = lists_close_to_player(&mut e, &[]);
        e.call(0x0096_e570, &args![empty]);
        assert!(close_to_player(&e, empty).is_empty());
    }

    #[test]
    fn remove_close_to_player_shifts_the_rest_down() {
        let mut e = fixture();
        memmove_double(&mut e);
        stubs(&mut e, &[LOCK_ENTER, LOCK_LEAVE]);
        let lists = lists_close_to_player(&mut e, &[1, 2, 3]);
        e.call_log = Some(vec![]);
        e.call(0x0096_e6f0, &args![lists, 2u32]);
        assert_eq!(close_to_player(&e, lists), vec![1, 3]);
        let slot1 = lists.addr() + ACTORS_CLOSE_TO_PLAYER + 4;
        assert_eq!(calls(&e, MEMMOVE), vec![vec![slot1, 200, slot1 + 4, 192]]);
        // The vacated last slot is cleared.
        assert_eq!(e.mem.u32(lists.addr() + ACTORS_CLOSE_TO_PLAYER + 8), 0);
        // Removing an actor that is not there changes nothing.
        e.call(0x0096_e6f0, &args![lists, 9u32]);
        assert_eq!(close_to_player(&e, lists), vec![1, 3]);
        assert_eq!(calls(&e, LOCK_LEAVE).len(), 2);
    }

    #[test]
    fn close_to_player_test_looks_only_at_the_first_entries() {
        let mut e = fixture();
        stubs(&mut e, &[LOCK_ENTER, LOCK_LEAVE]);
        e.register(0x004a_8f20, |_, a| Ret {
            eax: (a[0] as i32).min(a[1] as i32) as u32,
            ..Ret::default()
        });
        e.set_global(PLAYER, 0x7000u32);
        let lists = lists_close_to_player(&mut e, &[1, 2, 3]);
        assert!(e.call(0x0096_e7d0, &args![lists, 0u32, 1i32]).bool());
        assert!(e.call(0x0096_e7d0, &args![lists, 0x7000u32, 1i32]).bool());
        assert!(e.call(0x0096_e7d0, &args![lists, 2u32, 2i32]).bool());
        assert!(!e.call(0x0096_e7d0, &args![lists, 3u32, 2i32]).bool());
        assert!(e.call(0x0096_e7d0, &args![lists, 3u32, 20i32]).bool());
        assert!(!e.call(0x0096_e7d0, &args![lists, 4u32, 20i32]).bool());
    }

    /// An actor in a world for `AddActorToTempChangeList`: it has a process
    /// (so the checks run) and everything that would add it is off.
    fn temp_add_scene() -> (Engine, Ptr<ProcessLists>, u32) {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        let actor = object(&mut e);
        give_processes(&mut e, &[actor]);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, slot(0x22c), 0);
        stub(&mut e, slot(0x360), 0);
        stub(&mut e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS), 0);
        stub(&mut e, PROCESS_LEVEL, 3);
        stub(&mut e, ACTOR_DESIRED_LEVEL, 3);
        stub(&mut e, 0x0044_0d80, 0);
        stub(&mut e, 0x0094_df60, 0);
        e.set_global(PLAYER, 0x7000u32);
        (e, lists, actor)
    }

    fn temp_list(e: &Engine, lists: Ptr<ProcessLists>) -> Vec<u32> {
        list_items(e, lists.addr() + TEMP_SHOULD_MOVE_LIST)
    }

    #[test]
    fn temp_change_add_does_nothing_for_a_settled_actor() {
        let (mut e, lists, actor) = temp_add_scene();
        e.call(0x0096_e870, &args![lists, actor]);
        assert!(temp_list(&e, lists).is_empty());
        assert_eq!(e.mem.u8(actor + 0x82), 0);
    }

    #[test]
    fn temp_change_add_adds_an_actor_for_each_reason() {
        type Change = fn(&mut Engine, u32);
        let reasons: [(&str, Change); 5] = [
            ("knocked out", |e, _| stub(e, slot(0x22c), 1)),
            ("wrong level", |e, _| stub(e, ACTOR_DESIRED_LEVEL, 1)),
            ("process slot 0x360", |e, _| stub(e, slot(0x360), 1)),
            ("model loader", |e, _| stub(e, 0x0044_0d80, 1)),
            ("player resting", |e, _| stub(e, 0x0094_df60, 1)),
        ];
        for (reason, change) in reasons {
            let (mut e, lists, actor) = temp_add_scene();
            change(&mut e, actor);
            e.call_log = Some(vec![]);
            e.call(0x0096_e870, &args![lists, actor]);
            assert_eq!(temp_list(&e, lists), vec![actor], "{reason}");
            assert_eq!(e.mem.u8(actor + 0x82), 1, "{reason}");
            assert_eq!(
                calls(&e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS)),
                vec![vec![actor, 1]],
                "{reason}"
            );
        }
    }

    #[test]
    fn temp_change_add_handles_duplicates_non_actors_and_processless_actors() {
        let (mut e, lists, actor) = temp_add_scene();
        stub(&mut e, slot(0x22c), 1);
        e.call(0x0096_e870, &args![lists, actor]);
        e.call(0x0096_e870, &args![lists, actor]);
        assert_eq!(temp_list(&e, lists), vec![actor]);
        // A non-actor is flagged and added, but not marked at +0x82.
        let thing = object(&mut e);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o == actor) as u32);
        e.call(0x0096_e870, &args![lists, thing]);
        assert_eq!(temp_list(&e, lists), vec![thing, actor]);
        assert_eq!(e.mem.u8(thing + 0x82), 0);
        // An actor without a process is left alone.
        let (mut e, lists, actor) = temp_add_scene();
        stub(&mut e, ACTOR_PROCESS, 0);
        stub(&mut e, slot(0x22c), 1);
        e.call(0x0096_e870, &args![lists, actor]);
        assert!(temp_list(&e, lists).is_empty());
    }

    /// Fills the list head at `head` with `items` (the first in the head).
    fn fill_list(e: &mut Engine, head: u32, items: &[u32]) {
        let mut node = head;
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(node, *item);
            if i + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
    }

    #[test]
    fn temp_change_followers_are_assigned_their_leader() {
        let mut e = fixture();
        let objects: Vec<u32> = (0..5).map(|_| object(&mut e)).collect();
        let [pair, other_package, _no_package, leader_is_player, skipped] = objects[..] else {
            unreachable!()
        };
        let lists: Ptr<ProcessLists> = e.new_object();
        fill_list(&mut e, lists.addr() + TEMP_SHOULD_MOVE_LIST, &objects);
        let processes = give_processes(&mut e, &objects);
        let leader = object(&mut e);
        let player_actor = object(&mut e);
        e.set_global(PLAYER, player_actor);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        on_call(&mut e, 0x0057_6d30, move |o| (o == skipped) as u32);
        // Packages: `pair` has a type-2 package, `other_package` a type-1 one,
        // `no_package` none, `leader_is_player` a type-1 one.
        let packages: HashMap<u32, u32> = [
            (processes[0], 0x5002),
            (processes[1], 0x5001),
            (processes[3], 0x5001),
            (processes[4], 0x5001),
        ]
        .into_iter()
        .collect();
        on_slot(&mut e, 0x27c, move |p| {
            packages.get(&p).copied().unwrap_or(0)
        });
        on_call(
            &mut e,
            PACKAGE_TYPE,
            |pkg| if pkg == 0x5002 { 2 } else { 1 },
        );
        on_call(&mut e, 0x0088_1650, move |a| {
            if a == leader_is_player {
                player_actor
            } else {
                leader
            }
        });
        stubs(&mut e, &[slot(0xc8), 0x008b_c790]);
        e.call_log = Some(vec![]);
        e.call(0x0096_e9b0, &args![lists]);
        assert_eq!(
            calls(&e, slot(0xc8)),
            vec![vec![processes[0], pair, 0x5002, 0]]
        );
        assert_eq!(calls(&e, 0x008b_c790), vec![vec![other_package, leader]]);
    }

    #[test]
    fn temp_change_removal_clears_the_global_and_the_flag() {
        let mut e = fixture();
        let (x, y) = (object(&mut e), object(&mut e));
        let lists: Ptr<ProcessLists> = e.new_object();
        fill_list(&mut e, lists.addr() + TEMP_SHOULD_MOVE_LIST, &[x, y]);
        e.mem.set_u8(x + 0x82, 1);
        e.set_global(TEMP_CHANGE_ACTOR, x);
        e.call(0x0096_f400, &args![lists, x]);
        assert_eq!(e.global::<u32>(TEMP_CHANGE_ACTOR), 0);
        assert_eq!(temp_list(&e, lists), vec![y]);
        assert_eq!(e.mem.u8(x + 0x82), 0);
        // Another actor leaves the global alone.
        e.set_global(TEMP_CHANGE_ACTOR, x);
        e.call(0x0096_f400, &args![lists, y]);
        assert_eq!(e.global::<u32>(TEMP_CHANGE_ACTOR), x);
        assert!(temp_list(&e, lists).is_empty());
    }

    /// The world of `ChangeProcessLevelTempList`: one actor `x` with the
    /// process `p` is the only entry of the temp list, and everything that
    /// would make something happen is off; tests turn on what they need.
    fn change_scene() -> (Engine, Ptr<ProcessLists>, u32, u32) {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        let x = object(&mut e);
        let p = give_processes(&mut e, &[x])[0];
        fill_list(&mut e, lists.addr() + TEMP_SHOULD_MOVE_LIST, &[x]);
        e.mem.set_u8(x + 0x82, 1);
        e.set_global(PLAYER, 0x7000u32);
        let tes = e.mem.alloc(8);
        e.set_global(TES, tes);
        on_slot(&mut e, SLOT_IS_MOBILE_OBJECT, |_| 1);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stubs(
            &mut e,
            &[
                0x0094_df60,
                0x0057_6d30,
                0x0057_4900,
                0x0070_5190,
                0x0092_f160,
                0x0044_0d80,
                0x0044_0da0,
                0x0087_f3d0,
                0x0087_f4a0,
                0x0045_11e0,
                0x0052_6100,
                0x0086_7e30,
                0x0056_7d20,
                0x0052_6320,
                0x0041_df90,
                0x0041_de40,
                0x0057_2270,
                0x0041_dfd0,
                0x0093_1e80,
                ARRAY_REMOVE_ACTOR,
                ARRAY_ADD_ACTOR,
                LOCK_ENTER,
                LOCK_LEAVE,
                slot(0x22c),
                slot(0x360),
                slot(0x364),
                slot(0x224),
                slot(0x260),
                slot(SLOT_SET_NEED_TO_CHANGE_PROCESS),
                slot(SLOT_DELETING_DESTRUCTOR),
                slot(0x324),
                slot(0x160),
            ],
        );
        stub_f32(&mut e, slot(0x69c), 0.0);
        stub(&mut e, 0x0070_2640, 0x3f1);
        stub(&mut e, 0x0048_4e60, 1);
        stub(&mut e, ACTOR_PROCESS_LEVEL, 2);
        stub(&mut e, PROCESS_LEVEL, 3);
        stub(&mut e, LOCK_TRY_ENTER, 1);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o);
        (e, lists, x, p)
    }

    fn assert_temp_list_is(e: &Engine, lists: Ptr<ProcessLists>, expected: &[u32]) {
        assert_eq!(temp_list(e, lists), expected);
    }

    #[test]
    fn change_levels_does_nothing_while_the_player_rests() {
        let (mut e, lists, x, _) = change_scene();
        stub(&mut e, 0x0094_df60, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
        assert_temp_list_is(&e, lists, &[x]);
    }

    #[test]
    fn change_levels_deletes_an_actor_that_lost_its_process_and_its_file() {
        let (mut e, lists, x, _) = change_scene();
        stub(&mut e, ACTOR_PROCESS, 0);
        stub(&mut e, 0x0044_0d80, 1);
        stub(&mut e, 0x0048_4e60, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(calls(&e, slot(SLOT_DELETING_DESTRUCTOR)), vec![vec![x, 1]]);
        assert_eq!(calls(&e, LOCK_ENTER), vec![vec![LOCK, 0]]);
        assert!(calls(&e, ARRAY_REMOVE_ACTOR).is_empty());
        assert_temp_list_is(&e, lists, &[]);
        assert_eq!(e.mem.u8(x + 0x82), 0);
        assert_eq!(e.global::<u32>(TEMP_CHANGE_ACTOR), x);
    }

    #[test]
    fn change_levels_adds_a_reference_for_a_talking_actor_without_a_process() {
        let (mut e, lists, x, _) = change_scene();
        stub(&mut e, ACTOR_PROCESS, 0);
        stub(&mut e, 0x0057_4900, 1);
        stub(&mut e, 0x0092_f160, 1);
        stub(&mut e, 0x0070_5190, x);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(
            calls(&e, ARRAY_ADD_ACTOR),
            vec![vec![PROCESS_LISTS + 4, x, 0]]
        );
        assert!(calls(&e, slot(SLOT_DELETING_DESTRUCTOR)).is_empty());
        assert_temp_list_is(&e, lists, &[]);
    }

    #[test]
    fn change_levels_removes_an_actor_the_player_talks_to_somebody_else_about() {
        let (mut e, lists, x, _) = change_scene();
        // A reference (574900) that is not the one the player is in dialogue
        // with, and that 92f160 does not like: it leaves the process array and
        // is deleted.
        stub(&mut e, 0x0057_4900, 1);
        stub(&mut e, 0x0070_5190, 0x1234);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(
            calls(&e, ARRAY_REMOVE_ACTOR),
            vec![vec![lists.addr() + 4, x, 2]]
        );
        assert_eq!(calls(&e, slot(SLOT_DELETING_DESTRUCTOR)), vec![vec![x, 1]]);
        assert_temp_list_is(&e, lists, &[]);
    }

    #[test]
    fn change_levels_just_unlists_an_actor_without_a_process() {
        let (mut e, lists, x, _) = change_scene();
        stub(&mut e, ACTOR_PROCESS, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        // Nothing to release; the entry is just taken off the list.
        assert!(calls(&e, ARRAY_REMOVE_ACTOR).is_empty());
        assert_temp_list_is(&e, lists, &[]);
        assert_eq!(e.mem.u8(x + 0x82), 0);
    }

    /// A dead, non-essential, unloaded high actor whose death time (5 h
    /// after 100) has passed: slots and callees for the removal.
    fn dead_actor_scene() -> (Engine, Ptr<ProcessLists>, u32, u32, u32) {
        let (mut e, lists, x, p) = change_scene();
        stub(&mut e, slot(0x22c), 1);
        stub(&mut e, 0x0087_f4a0, 1);
        stub(&mut e, 0x008d_6f30, 0x10);
        stub_f32(&mut e, slot(0x69c), 5.0);
        stub(&mut e, 0x0052_6100, 100);
        stub(&mut e, 0x0086_7e30, 200);
        let item = object(&mut e);
        let dropped = make_list(&mut e, &[item]);
        on_call(&mut e, 0x0041_df90, move |_| dropped);
        (e, lists, x, p, item)
    }

    #[test]
    fn change_levels_removes_a_dead_actor_with_its_dropped_items() {
        let (mut e, lists, x, _, item) = dead_actor_scene();
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(calls(&e, slot(0x324)), vec![vec![x, 1, 0, 0]]);
        assert_eq!(calls(&e, 0x0041_de40), vec![vec![item, 0]]);
        assert_eq!(calls(&e, 0x0057_2270), vec![vec![item]]);
        assert_eq!(calls(&e, 0x0041_dfd0), vec![vec![x]]);
        assert_temp_list_is(&e, lists, &[]);
        assert_eq!(e.mem.u8(x + 0x82), 0);
        // The cell is looked up in the TES object with a 0 flag.
        let cell = calls(&e, 0x0045_11e0);
        assert_eq!(cell.len(), 1);
        assert_eq!(cell[0][1..], [0x10, 0]);
    }

    #[test]
    fn change_levels_keeps_a_dead_actor_in_a_protected_encounter_zone() {
        let (mut e, lists, x, _, _) = dead_actor_scene();
        stub(&mut e, 0x0056_7d20, 0x55);
        stub(&mut e, 0x0052_6320, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert!(calls(&e, slot(0x324)).is_empty());
        assert_temp_list_is(&e, lists, &[x]);
        // Left in the list, it is flagged at the end.
        assert_eq!(
            calls(&e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS)),
            vec![vec![x, 1]]
        );
    }

    #[test]
    fn change_levels_leaves_actors_alone_that_are_alive_loaded_or_just_died() {
        for case in 0..3 {
            let (mut e, lists, x, _, _) = dead_actor_scene();
            match case {
                0 => stub(&mut e, slot(0x22c), 0),
                1 => stub(&mut e, 0x0045_11e0, 1),
                _ => stub_f32(&mut e, slot(0x69c), 0.0),
            }
            e.call(0x0096_eb40, &args![lists]);
            assert_temp_list_is(&e, lists, &[x]);
        }
        // Not dead long enough.
        let (mut e, lists, x, _, _) = dead_actor_scene();
        stub(&mut e, 0x0086_7e30, 104);
        e.call(0x0096_eb40, &args![lists]);
        assert_temp_list_is(&e, lists, &[x]);
    }

    #[test]
    fn change_levels_releases_a_loaded_model_actor_when_the_lock_is_free() {
        let (mut e, lists, x, _) = change_scene();
        stub(&mut e, 0x0044_0da0, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(
            calls(&e, ARRAY_REMOVE_ACTOR),
            vec![vec![lists.addr() + 4, x, 2]]
        );
        assert_eq!(calls(&e, 0x0093_1e80), vec![vec![x]]);
        assert_eq!(calls(&e, LOCK_LEAVE).len(), 1);
        assert_temp_list_is(&e, lists, &[]);
    }

    #[test]
    fn change_levels_leaves_a_loaded_model_actor_when_the_lock_is_taken() {
        let (mut e, lists, x, _) = change_scene();
        stub(&mut e, 0x0044_0da0, 1);
        stub(&mut e, LOCK_TRY_ENTER, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert!(calls(&e, ARRAY_REMOVE_ACTOR).is_empty());
        assert_temp_list_is(&e, lists, &[x]);
    }

    #[test]
    fn change_levels_waits_for_actors_whose_flags_do_not_ask_for_a_change() {
        let (mut e, lists, x, _) = change_scene();
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_temp_list_is(&e, lists, &[x]);
        // Flagged to be looked at again at the end.
        assert_eq!(
            calls(&e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS)),
            vec![vec![x, 1]]
        );
        // The same when the form flags ask for a change but the player talks.
        e.mem.set_u32(x + 8, 0x2_0000);
        stub(&mut e, 0x0057_4900, 1);
        stub(&mut e, 0x0070_5190, x);
        stub(&mut e, 0x0070_2640, 0x3f1);
        e.call(0x0096_eb40, &args![lists]);
        assert_temp_list_is(&e, lists, &[x]);
    }

    #[test]
    fn change_levels_resets_an_actor_that_asks_for_a_change() {
        let (mut e, lists, x, _) = change_scene();
        e.mem.set_u32(x + 8, 0x2_0000);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        // The flag is cleared and the entry removed; the actor then gets its
        // slot 0x260 and, because the flag is still set, one more reset.
        assert_eq!(
            calls(&e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS)),
            vec![vec![x, 0], vec![x, 0]]
        );
        assert_eq!(calls(&e, slot(0x224)), vec![vec![x]]);
        assert_eq!(calls(&e, slot(0x260)), vec![vec![x]]);
        assert_temp_list_is(&e, lists, &[]);
    }

    #[test]
    fn change_levels_moves_a_follower_to_the_level_its_process_asks_for() {
        let (mut e, lists, x, p) = change_scene();
        e.mem.set_u32(x + 8, 0x2_0000);
        stub(&mut e, slot(0x360), 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(calls(&e, slot(0x364)), vec![vec![p, 0]]);
        assert_eq!(
            calls(&e, ARRAY_ADD_ACTOR),
            vec![vec![lists.addr() + 4, x, 3]]
        );
        // With a package of type 1 or 2 nothing is moved.
        let (mut e, lists, x, _) = change_scene();
        e.mem.set_u32(x + 8, 0x2_0000);
        stub(&mut e, slot(0x360), 1);
        stub(&mut e, slot(0x22c), 0x5001);
        stub(&mut e, 0x0087_f4a0, 0);
        stub(&mut e, PACKAGE_TYPE, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert!(calls(&e, ARRAY_ADD_ACTOR).is_empty());
        assert_temp_list_is(&e, lists, &[]);
    }

    #[test]
    fn change_levels_deletes_a_file_less_actor_that_asks_for_a_change() {
        let (mut e, lists, x, _) = change_scene();
        e.mem.set_u32(x + 8, 0x2_0000);
        stub(&mut e, 0x0044_0d80, 1);
        stub(&mut e, 0x0048_4e60, 0);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(calls(&e, slot(SLOT_DELETING_DESTRUCTOR)), vec![vec![x, 1]]);
        assert_eq!(
            calls(&e, ARRAY_REMOVE_ACTOR),
            vec![vec![lists.addr() + 4, x, 2]]
        );
    }

    #[test]
    fn change_levels_releases_a_loaded_model_actor_that_asks_for_a_change() {
        let (mut e, lists, x, _) = change_scene();
        e.mem.set_u32(x + 8, 0x2_0000);
        stub(&mut e, 0x0044_0d80, 1);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(calls(&e, 0x0093_1e80), vec![vec![x]]);
        assert!(calls(&e, slot(SLOT_DELETING_DESTRUCTOR)).is_empty());
        assert_temp_list_is(&e, lists, &[]);
    }

    #[test]
    fn change_levels_resets_the_actor_of_a_menu_dialogue() {
        let (mut e, lists, x, _) = change_scene();
        // The player is in dialogue with the actor, in a different menu mode.
        stub(&mut e, 0x0057_4900, 1);
        stub(&mut e, 0x0070_5190, x);
        stub(&mut e, 0x0070_2640, 0x3f2);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(
            calls(&e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS)),
            vec![vec![x, 0]]
        );
        assert_eq!(calls(&e, slot(0x260)), vec![vec![x]]);
        assert_temp_list_is(&e, lists, &[]);
    }

    #[test]
    fn change_levels_flags_every_entry_left_in_the_list_at_the_end() {
        let (mut e, lists, x, _) = change_scene();
        let y = object(&mut e);
        fill_list(&mut e, lists.addr() + TEMP_SHOULD_MOVE_LIST, &[x, y]);
        give_processes(&mut e, &[x, y]);
        e.call_log = Some(vec![]);
        e.call(0x0096_eb40, &args![lists]);
        assert_eq!(
            calls(&e, slot(SLOT_SET_NEED_TO_CHANGE_PROCESS)),
            vec![vec![x, 1], vec![y, 1]]
        );
    }

    #[test]
    fn collect_level_three_followers_tells_the_processes_and_lists_the_matches() {
        let mut e = fixture();
        let [h1, h2, h3, thing] = [
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
        ];
        let lists = lists_with(&mut e, 3, &[h1, h2, h3, thing]);
        let processes = give_processes(&mut e, &[h1, h2, h3]);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != thing) as u32);
        let wanted = 0x1234;
        stub(&mut e, 0x0048_39c0, 7);
        stubs(&mut e, &[slot(0x480), slot(0x664)]);
        on_slot(
            &mut e,
            0x2c8,
            move |o| if o == h2 { 0x6002 } else { 0x6001 },
        );
        on_call(&mut e, 0x0084_e3a0, move |holder| {
            if holder == 0x6001 {
                wanted
            } else {
                0
            }
        });
        e.call_log = Some(vec![]);
        let list = e.call(0x0096_f450, &args![lists, wanted, h3]).u32();
        // h1 matches; h2's holder does not; h3 is the excluded actor.
        assert_ne!(list, 0);
        assert_eq!(list_items(&e, list), vec![h1]);
        assert_eq!(
            calls(&e, slot(0x480)),
            processes
                .iter()
                .map(|p| vec![*p, wanted])
                .collect::<Vec<_>>()
        );
        assert_eq!(
            calls(&e, slot(0x664)),
            processes.iter().map(|p| vec![*p, 7]).collect::<Vec<_>>()
        );
        // Without a level nothing is told slot 0x664, and nothing matching gives no list.
        stub(&mut e, 0x0048_39c0, 0);
        stub(&mut e, 0x0084_e3a0, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0096_f450, &args![lists, wanted, h3]).u32(), 0);
        assert!(calls(&e, slot(0x664)).is_empty());
    }

    #[test]
    fn the_stored_word_setter_writes_at_0xfc() {
        let mut e = fixture();
        let thing = e.mem.alloc(0x110);
        e.call(0x0097_07f0, &args![Ptr::<()>::new(thing), 0x55u32]);
        assert_eq!(e.mem.u32(thing + 0xfc), 0x55);
    }

    #[test]
    fn package_scan_tells_matching_processes() {
        let mut e = fixture();
        let [a, b, c, d] = [
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
        ];
        set_objects(&mut e, PROCESS_LISTS + 4, 3, &[a, b, c, d]);
        let target = 0x4141u32;
        let processes = give_processes(&mut e, &[a, b, c]);
        let (pa, pb) = (processes[0], processes[1]);
        // `a`: a package (slot 0x27c) other than the current one, level 1:
        // when the package is the target the process gets slot 0x214.
        // `b`: its current package is the target.
        // `c`: its package extra is the target. `d` is skipped.
        on_slot(
            &mut e,
            0x27c,
            move |p| if p == pa { target } else { 0x5001 },
        );
        on_slot(
            &mut e,
            SLOT_0X22C,
            move |p| if p == pb { target } else { 0x6001 },
        );
        on_call(&mut e, PROCESS_LEVEL, |_| 1);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        on_call(&mut e, 0x0057_6d30, move |o| (o == d) as u32);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o);
        on_call(
            &mut e,
            0x0041_cb10,
            move |extra| if extra == c { target } else { 0 },
        );
        stubs(&mut e, &[slot(0x214), slot(0x234), slot(0x24), 0x0041_c930]);
        let tes_object = e.mem.alloc(8);
        e.set_global(0x011d_df38u32, tes_object);
        stub(&mut e, 0x0042_ce10, 0);
        let lists: Ptr<ProcessLists> = Ptr::new(PROCESS_LISTS);
        e.call_log = Some(vec![]);
        e.call(0x0097_0810, &args![lists, target]);
        assert_eq!(calls(&e, slot(0x214)), vec![vec![pa]]);
        assert_eq!(calls(&e, slot(0x234)), vec![vec![pb]]);
        assert_eq!(calls(&e, slot(0x24)), vec![vec![pb, b, 1]]);
        assert_eq!(calls(&e, 0x0041_c930), vec![vec![c, 0, 0, 0, 0, 0, 0]]);
        // When `0042ce10` of the global object holds, slot 0x24 is not called.
        stub(&mut e, 0x0042_ce10, 1);
        e.call_log = Some(vec![]);
        e.call(0x0097_0810, &args![lists, target]);
        assert!(calls(&e, slot(0x24)).is_empty());
        assert_eq!(calls(&e, slot(0x234)), vec![vec![pb]]);
    }

    #[test]
    fn package_scan_needs_a_process_level_of_zero_or_one() {
        let mut e = fixture();
        let a = object(&mut e);
        set_objects(&mut e, PROCESS_LISTS + 4, 3, &[a]);
        let target = 0x4141u32;
        let pa = give_processes(&mut e, &[a])[0];
        on_slot(&mut e, 0x27c, move |_| target);
        on_slot(&mut e, SLOT_0X22C, move |_| 0x6001);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, 0x0057_6d30, 0);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o);
        stub(&mut e, 0x0041_cb10, 0);
        stubs(&mut e, &[slot(0x214), slot(0x234), slot(0x24)]);
        let lists: Ptr<ProcessLists> = Ptr::new(PROCESS_LISTS);
        for (level, told) in [(0, true), (1, true), (2, false), (-1, false)] {
            on_call(&mut e, PROCESS_LEVEL, move |_| level as u32);
            e.call_log = Some(vec![]);
            e.call(0x0097_0810, &args![lists, target]);
            let expected: Vec<Vec<u32>> = if told { vec![vec![pa]] } else { vec![] };
            assert_eq!(calls(&e, slot(0x214)), expected, "level {level}");
        }
    }

    /// The world of `fn_0096f600`: the player, the target (an actor), the
    /// high actor `high` in process level 3 (with `level_three - 1` more),
    /// the temp-list actor `temp`, and a spare actor `follower`, each with a
    /// process; every callee is stubbed to find nothing.
    struct Forget {
        e: Engine,
        lists: Ptr<ProcessLists>,
        player: u32,
        player_process: u32,
        target: u32,
        high: u32,
        high_process: u32,
        temp_process: u32,
        follower: u32,
    }

    fn forget_scene(level_three: usize) -> Forget {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        let player = object(&mut e);
        let target = object(&mut e);
        let high = object(&mut e);
        let temp = object(&mut e);
        let follower = object(&mut e);
        let mut level3 = vec![high];
        for _ in 1..level_three {
            level3.push(object(&mut e));
        }
        let mut actors = vec![player, high, temp, follower];
        actors.extend_from_slice(&level3[1..]);
        let processes = give_processes(&mut e, &actors);
        set_objects(&mut e, lists.addr() + MOB_PROCESS_ARRAY, 3, &level3);
        fill_list(&mut e, lists.addr() + TEMP_SHOULD_MOVE_LIST, &[temp]);
        e.set_global(PLAYER, player);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o);
        for s in [
            0x2c4, 0x68c, 0x26c, 0x484, 0x488, 0xf8, 0x468, 0x224, 0x310, 0x4a0, 0x260, 0x7b0,
            0xc0, 0x20c, 0x278, 0x47c, 0x8, 0x3d0, 0x3d4, 0xac, 0x22c, 0x428, 0x128, 0x12c, 0x2c8,
            0x234, 0x214, 0x288, 0x664, 0x4c8, 0x4d4, 0x540, 0x4,
        ] {
            stub(&mut e, slot(s), 0);
        }
        stubs(
            &mut e,
            &[
                EXTRA_FOLLOWERS,
                EXTRA_REMOVE_FOLLOWER,
                0x004f_d380,
                0x0057_bd60,
                0x0057_6d30,
                0x0087_4480,
                0x008b_3200,
                0x0041_9ea0,
                0x0041_9dc0,
                0x005e_3fa0,
                0x0041_0220,
                0x008a_6650,
                0x008a_6170,
                0x0093_44a0,
                0x008a_6730,
                0x009f_8420,
                0x0042_12b0,
                0x0041_cb70,
                0x0041_cab0,
                0x0055_b980,
                0x0067_f390,
                0x0067_f3c0,
                0x0097_f9c0,
                0x0088_1620,
                0x0088_1650,
                0x0067_1d10,
                0x0068_0020,
                0x0093_36c0,
                0x0067_4d40,
                0x0088_1680,
                0x008d_80e0,
                0x00c5_4550,
                0x0056_8ab0,
                0x0098_ddd0,
                0x005e_3fc0,
                0x0099_4ef0,
                PACKAGE_TYPE,
            ],
        );
        stub(&mut e, PROCESS_LEVEL, 5);
        Forget {
            e,
            lists,
            player,
            player_process: processes[0],
            target,
            high,
            high_process: processes[1],
            temp_process: processes[2],
            follower,
        }
    }

    #[test]
    fn forgetting_a_reference_finds_nothing_to_clear_in_a_quiet_world() {
        let mut f = forget_scene(1);
        let (t, h, ph, pt) = (f.target, f.high, f.high_process, f.temp_process);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        let e = &f.e;
        // The player: the target leaves its detection list and the actors' lists.
        assert_eq!(calls(e, slot(0x2c4))[0], vec![f.player_process, t, 3]);
        assert_eq!(calls(e, slot(0x468)), vec![vec![f.player, t]]);
        assert_eq!(calls(e, LIST_REMOVE).len(), 1);
        // The level-3 actor gets the full clean-up.
        assert_eq!(calls(e, 0x008b_3200), vec![vec![h, t]]);
        assert_eq!(calls(e, slot(0x47c)), vec![vec![ph, t], vec![pt, t]]);
        assert_eq!(calls(e, slot(0x8)), vec![vec![ph, t]]);
        assert_eq!(calls(e, slot(0x664)), vec![vec![ph, t], vec![pt, t]]);
        // The temp-list actor (the level-three tail is 1) is told its
        // detection list too.
        assert_eq!(
            calls(e, slot(0x2c4)),
            vec![vec![f.player_process, t, 3], vec![pt, t, 3]]
        );
        // Nothing was cleared.
        for addr in [0x0057_bd60u32, 0x0099_4ef0, 0x0067_f3c0, 0x0098_ddd0] {
            assert!(calls(e, addr).is_empty());
        }
    }

    #[test]
    fn forgetting_a_reference_skips_the_detection_removal_when_level_three_is_big() {
        let mut f = forget_scene(3);
        let t = f.target;
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        // Only the player's process: the temp-list actor sees a level-three
        // tail of 3.
        assert_eq!(calls(&f.e, slot(0x2c4)), vec![vec![f.player_process, t, 3]]);
    }

    #[test]
    fn forgetting_a_reference_clears_what_points_at_the_player() {
        let mut f = forget_scene(1);
        let (t, pp, pl) = (f.target, f.player_process, f.player);
        let record = f.e.mem.alloc(0x20);
        f.e.mem.set_u32(record + 0x18, t);
        let followers = f.e.mem.alloc(0x20);
        let follower_list = make_list(&mut f.e, &[t]);
        f.e.mem.set_u32(followers + 0xc, follower_list);
        on_call(&mut f.e, EXTRA_FOLLOWERS, move |x| {
            if x == pl {
                followers
            } else {
                0
            }
        });
        on_call(&mut f.e, 0x004f_d380, move |o| if o == pl { t } else { 0 });
        on_slot(&mut f.e, 0x68c, move |p| if p == pp { t } else { 0 });
        on_slot(&mut f.e, 0x484, move |p| if p == pp { t } else { 0 });
        on_slot(&mut f.e, 0xf8, move |p| if p == pp { record } else { 0 });
        stub(&mut f.e, slot(0x26c), 0);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        let e = &f.e;
        // The target is removed from the player's followers (the list head
        // holds only it) and from the alive-actor list.
        assert_eq!(list_items(e, follower_list), Vec::<u32>::new());
        assert_eq!(calls(e, 0x0057_bd60)[0], vec![pl, 0]);
        assert_eq!(calls(e, slot(0x26c)), vec![vec![pp]]);
        assert_eq!(calls(e, slot(0x488))[0], vec![pp, 0]);
        // The record's reference is the target: the player's process gets slot 0x100.
        assert_eq!(
            calls(e, slot(0x100)).iter().filter(|w| w[0] == pp).count(),
            1
        );
    }

    #[test]
    fn forgetting_a_reference_in_non_actors() {
        let mut f = forget_scene(2);
        let t = f.target;
        let (n1, n2) = (object(&mut f.e), object(&mut f.e));
        let n3 = object(&mut f.e);
        set_objects(&mut f.e, f.lists.addr() + MOB_PROCESS_ARRAY, 3, &[n1, n2]);
        fill_list(&mut f.e, f.lists.addr() + TEMP_SHOULD_MOVE_LIST, &[n3]);
        for n in [n1, n2, n3] {
            f.e.mem.set_u32(n + 0xfc, 0x77);
        }
        let high = f.high;
        on_slot(&mut f.e, SLOT_IS_ACTOR, move |o| {
            (o == high || o == t) as u32
        });
        on_slot(&mut f.e, 0x224, move |o| (o == n1 || o == n3) as u32);
        on_call(&mut f.e, 0x0087_4480, move |o| {
            if o == n1 || o == n3 {
                t
            } else {
                0
            }
        });
        on_call(&mut f.e, 0x005e_3fa0, move |o| if o == n2 { t } else { 0 });
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        let e = &f.e;
        // The "other" objects forget the target (+0xFC cleared); `n2` has
        // an owner that is the target and gets slot 0x288.
        assert_eq!(e.mem.u32(n1 + 0xfc), 0);
        assert_eq!(e.mem.u32(n3 + 0xfc), 0);
        assert_eq!(e.mem.u32(n2 + 0xfc), 0x77);
        assert_eq!(calls(e, slot(0x288)), vec![vec![n2]]);
    }

    #[test]
    fn forgetting_a_reference_in_a_process_of_a_low_level() {
        let mut f = forget_scene(1);
        let (t, h, ph) = (f.target, f.high, f.high_process);
        stub(&mut f.e, PROCESS_LEVEL, 0);
        let record = f.e.mem.alloc(0x20);
        f.e.mem.set_u32(record + 0x18, t);
        let holder = f.e.mem.alloc(0x20);
        f.e.mem.set_u32(holder + 8, t);
        on_slot(&mut f.e, 0xf8, move |p| if p == ph { record } else { 0 });
        on_slot(&mut f.e, 0x68c, move |p| if p == ph { t } else { 0 });
        on_slot(&mut f.e, 0x484, move |p| if p == ph { t } else { 0 });
        on_slot(&mut f.e, 0x20c, move |p| if p == ph { 0x5001 } else { 0 });
        on_slot(&mut f.e, 0x278, move |p| if p == ph { holder } else { 0 });
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        let e = &f.e;
        assert!(calls(e, slot(0x2c4)).contains(&vec![ph, t, 3]));
        assert_eq!(calls(e, slot(0x260)), vec![vec![ph, t]]);
        assert_eq!(calls(e, slot(0x7b0)), vec![vec![ph, t]]);
        assert!(calls(e, slot(0x100)).contains(&vec![ph]));
        assert_eq!(calls(e, slot(0x26c)), vec![vec![ph]]);
        assert_eq!(calls(e, slot(0x488)), vec![vec![ph, 0]]);
        assert_eq!(calls(e, slot(0xc0)), vec![vec![ph, t]]);
        // A holder whose +8 is the target ends the interrupt package.
        assert_eq!(calls(e, 0x0088_1680), vec![vec![h, 0]]);
        // At level 0 the process is also told slot 0xac.
        assert!(calls(e, slot(0xac)).contains(&vec![ph]));
    }

    #[test]
    fn forgetting_a_reference_in_the_dialogue_package() {
        let mut f = forget_scene(1);
        let (t, h, ph) = (f.target, f.high, f.high_process);
        stub(&mut f.e, PROCESS_LEVEL, 1);
        on_slot(&mut f.e, 0x20c, move |p| if p == ph { 0x5001 } else { 0 });
        on_call(
            &mut f.e,
            PACKAGE_TYPE,
            |pkg| if pkg == 0x5001 { 0x1c } else { 0 },
        );
        on_call(
            &mut f.e,
            0x008d_80e0,
            move |pkg| if pkg == 0x5001 { t } else { 0 },
        );
        stub(&mut f.e, slot(0x288), 0);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        assert_eq!(calls(&f.e, slot(0x288)), vec![vec![h]]);
        assert!(calls(&f.e, 0x0088_1680).is_empty());
    }

    #[test]
    fn forgetting_a_reference_clears_the_extra_data_and_process_fields() {
        let mut f = forget_scene(1);
        let (t, h, ph) = (f.target, f.high, f.high_process);
        let controller = 0x5555u32;
        let record = f.e.mem.alloc(0x20);
        f.e.mem.set_u32(record + 0x14, t);
        let package_data = object(&mut f.e);
        on_call(&mut f.e, 0x0041_9ea0, move |x| if x == h { t } else { 0 });
        on_call(
            &mut f.e,
            0x0041_0220,
            move |x| if x == h { record } else { 0 },
        );
        on_call(
            &mut f.e,
            0x0042_12b0,
            move |x| if x == h { package_data } else { 0 },
        );
        on_call(&mut f.e, 0x0041_cb70, move |x| if x == h { t } else { 0 });
        on_slot(&mut f.e, 0x3d0, move |p| if p == ph { t + 0x94 } else { 0 });
        on_slot(
            &mut f.e,
            0x428,
            move |o| if o == h { controller } else { 0 },
        );
        on_slot(&mut f.e, 0x128, move |p| if p == ph { t } else { 0 });
        on_slot(&mut f.e, 0x2c8, move |o| if o == h { t } else { 0 });
        stubs(&mut f.e, &[slot(0x4), slot(0x3d4), slot(0x12c)]);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        let e = &f.e;
        assert_eq!(calls(e, 0x0041_9dc0), vec![vec![h, 0]]);
        assert_eq!(e.mem.u32(record + 0x14), 0);
        assert_eq!(calls(e, slot(0x4)), vec![vec![package_data, t]]);
        assert_eq!(calls(e, 0x0041_cab0), vec![vec![h, 0]]);
        assert_eq!(calls(e, slot(0x3d4)), vec![vec![ph, 0]]);
        assert_eq!(calls(e, 0x0097_f9c0), vec![vec![controller, t]]);
        assert_eq!(calls(e, slot(0x12c)), vec![vec![ph, 0]]);
        assert_eq!(calls(e, 0x0088_1620), vec![vec![h, 0]]);
    }

    #[test]
    fn forgetting_a_reference_clears_package_locations_only_when_asked() {
        for flag in [0u8, 1] {
            let mut f = forget_scene(1);
            let (t, h) = (f.target, f.high);
            on_call(
                &mut f.e,
                0x0093_44a0,
                move |o| if o == h { 0x5001 } else { 0 },
            );
            on_call(&mut f.e, 0x0055_b980, |pkg| {
                if pkg == 0x5001 {
                    0x9000
                } else {
                    0
                }
            });
            on_call(
                &mut f.e,
                0x0067_f390,
                move |loc| if loc == 0x9000 { t } else { 0 },
            );
            f.e.call_log = Some(vec![]);
            f.e.call(0x0096_f600, &args![f.lists, t, flag]);
            // Only the first loop (the level-three actor) has the flag.
            let expected: Vec<Vec<u32>> = if flag == 0 {
                vec![]
            } else {
                vec![vec![0x9000, 0]]
            };
            assert_eq!(calls(&f.e, 0x0067_f3c0), expected, "flag {flag}");
        }
    }

    #[test]
    fn forgetting_a_reference_ends_the_package_that_targets_it() {
        // Via 00881650, in dialogue: slot 0x288.
        let mut f = forget_scene(1);
        let (t, h, ph) = (f.target, f.high, f.high_process);
        on_call(&mut f.e, 0x0088_1650, move |o| if o == h { t } else { 0 });
        stub(&mut f.e, 0x0093_36c0, 1);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        assert_eq!(calls(&f.e, slot(0x288)), vec![vec![h]]);
        // Not in dialogue, the package was created: EndInterruptPackage.
        stub(&mut f.e, 0x0093_36c0, 0);
        stub(&mut f.e, 0x0067_4d40, 1);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        assert_eq!(calls(&f.e, 0x0088_1680), vec![vec![h, 0]]);
        // Otherwise the process clears the current package.
        stub(&mut f.e, 0x0067_4d40, 0);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        assert_eq!(calls(&f.e, slot(0x234)), vec![vec![ph]]);
        assert_eq!(calls(&f.e, slot(0x214)), vec![vec![ph]]);
        // Via the holder returned by slot 0x20c.
        let mut f = forget_scene(1);
        let (t, h, ph) = (f.target, f.high, f.high_process);
        on_slot(&mut f.e, 0x20c, move |p| if p == ph { 0x7001 } else { 0 });
        on_call(&mut f.e, 0x0067_1d10, |holder| {
            if holder == 0x7001 {
                0x7002
            } else {
                0
            }
        });
        on_call(
            &mut f.e,
            0x0068_0020,
            move |inner| if inner == 0x7002 { t } else { 0 },
        );
        stub(&mut f.e, 0x0093_36c0, 1);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        assert_eq!(calls(&f.e, slot(0x288)), vec![vec![h]]);
    }

    #[test]
    fn forgetting_a_reference_resets_the_furniture_marker() {
        let mut f = forget_scene(1);
        let (t, ph) = (f.target, f.high_process);
        on_slot(&mut f.e, 0x4c8, move |p| if p == ph { t } else { 0 });
        stub(&mut f.e, slot(0x4d4), 0x8800);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        let e = &f.e;
        assert_eq!(calls(e, 0x00c5_4550), vec![vec![0x8800, 0.0f32.to_bits()]]);
        assert_eq!(calls(e, 0x0056_8ab0), vec![vec![0x8800, 0]]);
        assert_eq!(
            calls(e, 0x0098_ddd0),
            vec![vec![0x8800, FURNITURE_ARGUMENT]]
        );
        assert_eq!(calls(e, slot(0x540)), vec![vec![ph, 0, 0x7f, 0x8800]]);
    }

    #[test]
    fn forgetting_a_reference_handles_fleeing_and_created_packages() {
        // Fleeing: the flee package's target is cleared, the flee target removed.
        let mut f = forget_scene(1);
        let (t, h) = (f.target, f.high);
        stub(&mut f.e, 0x008a_6650, 1);
        on_call(&mut f.e, 0x0093_44a0, |_| 0x5001);
        on_call(
            &mut f.e,
            PACKAGE_TYPE,
            |pkg| if pkg == 0x5001 { 0x16 } else { 0 },
        );
        stub(&mut f.e, 0x005e_3fc0, t);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        assert_eq!(calls(&f.e, 0x0099_4ef0).len(), 2);
        assert_eq!(calls(&f.e, 0x0099_4ef0)[0], vec![0x5001, 0]);
        assert_eq!(calls(&f.e, 0x008a_6730)[0], vec![h, t]);
        // Not fleeing but a type-0x18 package (and the target is an actor).
        let mut f = forget_scene(1);
        let t = f.target;
        stub(&mut f.e, 0x008a_6170, 1);
        on_call(&mut f.e, 0x0093_44a0, |_| 0x5002);
        on_call(
            &mut f.e,
            PACKAGE_TYPE,
            |pkg| if pkg == 0x5002 { 0x18 } else { 0 },
        );
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        assert_eq!(calls(&f.e, 0x009f_8420), vec![vec![0x5002, t]]);
    }

    #[test]
    fn forgetting_a_reference_goes_through_the_followers_of_a_high_actor() {
        let mut f = forget_scene(1);
        let (t, h, follower) = (f.target, f.high, f.follower);
        let not_actor = object(&mut f.e);
        let followers = f.e.mem.alloc(0x20);
        let follower_list = make_list(&mut f.e, &[follower, not_actor]);
        f.e.mem.set_u32(followers + 0xc, follower_list);
        on_call(&mut f.e, EXTRA_FOLLOWERS, move |x| {
            if x == h {
                followers
            } else {
                0
            }
        });
        on_slot(&mut f.e, SLOT_IS_ACTOR, move |o| (o != not_actor) as u32);
        f.e.call_log = Some(vec![]);
        f.e.call(0x0096_f600, &args![f.lists, t, 0u8]);
        // The target also leaves the high actor's follower list.
        assert_eq!(calls(&f.e, EXTRA_REMOVE_FOLLOWER)[0], vec![h, t]);
        assert_eq!(
            calls(&f.e, 0x008b_3200),
            vec![vec![h, t], vec![follower, t]]
        );
    }

    // --- 00970a20 .. 00970f60 ---

    #[test]
    fn get_actor_ref_in_high_finds_an_actor_by_form_or_by_original_base() {
        let mut e = fixture();
        let (a, b, c) = (object(&mut e), object(&mut e), object(&mut e));
        let lists = lists_with(&mut e, 0, &[a, 0, b, c]);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != c) as u32);
        stub(&mut e, 0x0057_6d30, 0);
        on_slot(&mut e, 0x218, move |o| (o == a) as u32);
        stub(&mut e, 0x007a_f430, 0x1234);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o);
        on_call(
            &mut e,
            0x0042_16f0,
            move |x| if x == b { 0x5678 } else { 0 },
        );
        let find = |e: &mut Engine, base: u32| e.call(0x0097_0a20, &args![lists, base, 0u32]).u32();
        assert_eq!(find(&mut e, 0x1234), a);
        assert_eq!(find(&mut e, 0x5678), b);
        assert_eq!(find(&mut e, 0x9999), 0);
    }

    #[test]
    fn get_actor_ref_in_high_skips_actors_that_00576d30_rejects() {
        let mut e = fixture();
        let a = object(&mut e);
        let lists = lists_with(&mut e, 0, &[a]);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, 0x0057_6d30, 1);
        let result = e.call(0x0097_0a20, &args![lists, 0x1234u32, 0u32]).u32();
        assert_eq!(result, 0);
    }

    /// Three actors of level 0 for `fn_00970b30`: `ranks` are their faction
    /// ranks, `distances` their distances to the player; the faction data of
    /// an actor is the actor itself with the rank at +0x30.
    fn faction_scene(
        ranks: [i32; 3],
        distances: [f32; 3],
        flagged: [bool; 3],
    ) -> (Engine, Ptr<ProcessLists>, [u32; 3]) {
        let mut e = fixture();
        e.map(0x0102_2000, 0x1000);
        e.set_global(0x0102_2958u32, 10000.0f32);
        let actors = [object(&mut e), object(&mut e), object(&mut e)];
        for i in 0..3 {
            e.mem.set_u32(actors[i] + 0x30, ranks[i] as u32);
        }
        let lists = lists_with(&mut e, 0, &actors);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        on_slot(&mut e, SLOT_0X22C, |_| 0);
        on_slot(&mut e, 0x21c, move |o| {
            (0..3).any(|i| flagged[i] && o == actors[i]) as u32
        });
        stub(&mut e, 0x0043_7bd0, 0);
        on_call(&mut e, 0x0041_81e0, |actor| actor);
        e.register(0x0047_d680, |e, a| Ret {
            eax: e.mem.u32(a[0]),
            ..Ret::default()
        });
        e.register_double(0x0057_23b0, move |_, a| Ret {
            st0: (0..3)
                .find(|i| a[0] == actors[*i])
                .map_or(0.0, |i| distances[i] as f64),
            ..Ret::default()
        });
        (e, lists, actors)
    }

    #[test]
    fn faction_actor_search_returns_the_last_actor_with_a_rank() {
        let (mut e, lists, actors) = faction_scene([-1, 0, 5], [1.0; 3], [false; 3]);
        e.call_log = Some(vec![]);
        let result = e.call(0x0097_0b30, &args![lists, 7u32, 0u8, 0u8]).u32();
        assert_eq!(result, actors[2]);
        // The rank is asked with the faction and whether the actor is the player.
        assert_eq!(calls(&e, 0x0047_d680)[2], vec![actors[2] + 0x30, 7, 0]);
    }

    #[test]
    fn faction_actor_search_with_closest_keeps_the_first_unflagged_hit() {
        // The loop ends as soon as there is a result, flagged actors excepted.
        let (mut e, lists, actors) =
            faction_scene([0, 0, 0], [50.0, 20.0, 5.0], [true, false, false]);
        let result = e.call(0x0097_0b30, &args![lists, 7u32, 0u8, 1u8]).u32();
        assert_eq!(result, actors[1]);
    }

    #[test]
    fn faction_actor_search_falls_back_to_the_flagged_actor() {
        let (mut e, lists, actors) =
            faction_scene([0, -1, -1], [50.0, 20.0, 5.0], [true, false, false]);
        assert_eq!(
            e.call(0x0097_0b30, &args![lists, 7u32, 0u8, 1u8]).u32(),
            actors[0]
        );
        assert_eq!(
            e.call(0x0097_0b30, &args![lists, 7u32, 0u8, 0u8]).u32(),
            actors[0]
        );
    }

    #[test]
    fn faction_actor_search_can_require_an_alert_process() {
        let (mut e, lists, actors) = faction_scene([0, 0, -1], [1.0; 3], [false; 3]);
        let (p0, p1) = (object(&mut e), object(&mut e));
        let first = actors[0];
        on_call(
            &mut e,
            ACTOR_PROCESS,
            move |a| if a == first { p0 } else { p1 },
        );
        on_slot(&mut e, 0x2d4, move |p| if p == p1 { 3 } else { 0 });
        let result = e.call(0x0097_0b30, &args![lists, 7u32, 1u8, 0u8]).u32();
        assert_eq!(result, actors[1]);
    }

    #[test]
    fn calendar_pass_publishes_the_date_and_cleans_level_three_actors() {
        let mut e = fixture();
        let (a1, a2, a3, thing) = (
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
        );
        let lists = lists_with(&mut e, 3, &[a1, a2, a3, thing]);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != thing) as u32);
        stub_f32(&mut e, 0x0086_7da0, 6.5);
        stub(&mut e, 0x0086_7c60, 2281);
        stub(&mut e, 0x0086_7d20, 4);
        stub(&mut e, 0x0086_7d60, 17);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o + 1);
        stubs(
            &mut e,
            &[0x005a_c190, 0x0089_3340, 0x008a_1800, ARRAY_REMOVE_ACTOR],
        );
        on_call(&mut e, 0x0044_0d80, move |o| (o == a3) as u32);
        stub(&mut e, 0x007a_f430, 1);
        for actor in [a1, a2, a3] {
            e.mem.set_u32(actor + 0xa4, VTABLE);
        }
        // The embedded object at +0xA4 answers slot 8: positive only for a1.
        on_slot(&mut e, 8, move |o| (o == a1 + 0xa4) as u32);
        on_call(&mut e, ACTOR_PROCESS, move |o| (o != a1) as u32);
        e.call_log = Some(vec![]);
        e.call(0x0097_0d50, &args![lists]);
        assert_eq!(e.global::<f32>(GAME_HOUR), 6.5);
        assert_eq!(e.global::<i32>(GAME_YEAR), 2281);
        assert_eq!(e.global::<i32>(GAME_MONTH), 4);
        assert_eq!(e.global::<u8>(GAME_DAY), 17);
        assert_eq!(
            calls(&e, 0x005a_c190),
            vec![vec![a1, a1 + 1], vec![a2, a2 + 1], vec![a3, a3 + 1]]
        );
        assert_eq!(calls(&e, 0x0089_3340), vec![vec![a1, 0]]);
        assert_eq!(calls(&e, 0x008a_1800), vec![vec![a2, 2]]);
        let array = lists.addr() + MOB_PROCESS_ARRAY;
        assert_eq!(
            calls(&e, ARRAY_REMOVE_ACTOR),
            vec![vec![array, a2, 3], vec![array, a3, 3]]
        );
    }

    #[test]
    fn distance_compare_orders_by_distance_to_the_player() {
        let mut e = fixture();
        let (near, far, same) = (0x100u32, 0x200u32, 0x300u32);
        e.register_double(0x0057_23b0, move |_, a| Ret {
            st0: if a[0] == far { 9.0 } else { 3.0 },
            ..Ret::default()
        });
        let compare = |e: &mut Engine, x: u32, y: u32| e.call(0x0097_0ee0, &args![x, y]).i32();
        assert_eq!(compare(&mut e, near, far), -1);
        assert_eq!(compare(&mut e, far, near), 1);
        assert_eq!(compare(&mut e, near, same), 0);
    }

    /// The scene of `fn_00970f60`: a target actor and three level-0 actors
    /// (the first detects it strongly, the second weakly, the third not).
    fn detection_scene() -> (Engine, Ptr<ProcessLists>, [u32; 4], u32) {
        let mut e = fixture();
        let target = object(&mut e);
        let actors = [target, object(&mut e), object(&mut e), object(&mut e)];
        let lists = lists_with(&mut e, 0, &actors);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, 0x0084_e3a0, target);
        stubs(&mut e, &[0x0044_0da0, 0x0044_0d80, 0x0043_7bd0]);
        stubs(&mut e, &[slot(0x234), slot(SLOT_0X22C), 0x0083_fd60]);
        on_call(&mut e, 0x008a_0d10, move |o| {
            if o == actors[1] {
                5
            } else if o == actors[2] {
                1
            } else {
                0
            }
        });
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        stub(&mut e, slot(0x5c0), 0);
        (e, lists, actors, target)
    }

    #[test]
    fn detecting_actors_are_listed_and_sorted() {
        let (mut e, lists, actors, target) = detection_scene();
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        stub(&mut e, slot(0x5c0), 2);
        stub(&mut e, 0x0097_1c30, 0);
        stub(&mut e, slot(0x348), 0);
        e.call_log = Some(vec![]);
        let list = e.call(0x0097_0f60, &args![lists, 0x77u32]).u32();
        assert_ne!(list, 0);
        let mut items = list_items(&e, list);
        items.sort_unstable();
        let mut expected = vec![actors[1], actors[2]];
        expected.sort_unstable();
        assert_eq!(items, expected);
        assert_eq!(calls(&e, slot(0x348)), vec![vec![process, 0]]);
        assert_eq!(calls(&e, 0x0083_fd60), vec![vec![list, 0x0097_0ee0]]);
        // The target is not tested against itself.
        assert!(calls(&e, 0x008a_0d10).iter().all(|c| c[0] != target));
    }

    #[test]
    fn detecting_actors_use_the_size_of_the_source_list_for_the_process() {
        let (mut e, lists, _, _) = detection_scene();
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        stub(&mut e, slot(0x5c0), 2);
        stub(&mut e, 0x0097_1c30, 0x4000);
        stub(&mut e, 0x005a_e380, 9);
        stub(&mut e, slot(0x348), 0);
        e.call_log = Some(vec![]);
        e.call(0x0097_0f60, &args![lists, 0x77u32]);
        assert_eq!(calls(&e, 0x0097_1c30)[0][2..], [0x15, 0]);
        assert_eq!(calls(&e, slot(0x348)), vec![vec![process, 9]]);
    }

    #[test]
    fn detecting_actors_gives_nothing_for_a_busy_target() {
        let (mut e, lists, _, _) = detection_scene();
        stub(&mut e, 0x0044_0d80, 1);
        assert_eq!(e.call(0x0097_0f60, &args![lists, 0x77u32]).u32(), 0);
    }

    #[test]
    fn detecting_actors_gives_nothing_when_nobody_detects() {
        let (mut e, lists, _, _) = detection_scene();
        stub(&mut e, 0x008a_0d10, 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0097_0f60, &args![lists, 0x77u32]).u32(), 0);
        assert!(calls(&e, 0x0083_fd60).is_empty());
    }

    // --- 009711e0 .. 00971c30 ---

    /// A target `t`, three level-3 listeners and a go-between node list for
    /// the alarm functions. The cells: `t` and `a[0]` share 0xC1 (an interior
    /// cell for `00425fd0`), `a[1]` is in 0xC2, `a[2]` in 0xC3 and a different
    /// world space; the go-between location has no cell but that world space.
    /// Distances to the target: 10 for `a[0]`, 5000 for `a[1]`; `a[2]` is 20
    /// from the location. The alarm distance setting is 100.
    struct AlarmScene {
        e: Engine,
        lists: Ptr<ProcessLists>,
        t: u32,
        a: [u32; 3],
        nodes: u32,
    }

    fn alarm_scene() -> AlarmScene {
        let mut e = fixture();
        let t = object(&mut e);
        let a = [object(&mut e), object(&mut e), object(&mut e)];
        let location = object(&mut e);
        let lists = lists_with(&mut e, 3, &a);
        e.set_global(0x011c_dbecu32 + 0x100, 100u32);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, 0x0084_e3a0, t);
        on_call(&mut e, 0x008d_6f30, move |o| {
            if o == t || o == a[0] {
                0xc1
            } else if o == a[1] {
                0xc2
            } else if o == a[2] {
                0xc3
            } else {
                0
            }
        });
        on_call(&mut e, 0x0057_5d70, move |o| {
            if o == a[2] || o == location {
                0x60
            } else {
                0x50
            }
        });
        on_call(&mut e, 0x0042_5fd0, |cell| (cell == 0xc1) as u32);
        e.register_double(0x0057_23b0, move |_, w| Ret {
            st0: if w[0] == a[0] && w[1] == t {
                10.0
            } else if w[0] == a[1] && w[1] == t {
                5000.0
            } else if w[0] == a[2] && w[1] == location {
                20.0
            } else {
                1.0e9
            },
            ..Ret::default()
        });
        on_call(&mut e, NODE_ITEM, |node| node);
        on_call(&mut e, 0x0056_8e50, |reference| reference + 1);
        on_call(&mut e, 0x0055_9450, move |_| location);
        stubs(
            &mut e,
            &[
                0x0044_0da0,
                0x0044_0d80,
                0x0093_44a0,
                0x0063_2d20,
                0x0083_fd60,
                slot(SLOT_0X22C),
            ],
        );
        let nodes = make_list(&mut e, &[0x7777]);
        AlarmScene {
            e,
            lists,
            t,
            a,
            nodes,
        }
    }

    #[test]
    fn yelling_an_alarm_lists_the_actors_that_hear_it() {
        let mut s = alarm_scene();
        s.e.call_log = Some(vec![]);
        let list =
            s.e.call(0x0097_11e0, &args![s.lists, 0x55u32, s.nodes])
                .u32();
        let mut items = list_items(&s.e, list);
        items.sort_unstable();
        let mut expected = vec![s.a[0], s.a[2]];
        expected.sort_unstable();
        assert_eq!(items, expected);
        assert_eq!(calls(&s.e, 0x0063_2d20), vec![vec![0x55, 1]]);
        assert_eq!(calls(&s.e, 0x0083_fd60), vec![vec![list, 0x0097_0ee0]]);
    }

    #[test]
    fn yelling_an_alarm_skips_actors_that_already_know_the_crime() {
        let mut s = alarm_scene();
        let (known, other) = (0x6001u32, 0x6002u32);
        let first = s.a[0];
        on_call(&mut s.e, 0x0093_44a0, move |o| {
            if o == first {
                known
            } else {
                other
            }
        });
        on_call(&mut s.e, PACKAGE_TYPE, |_| 0x15);
        on_call(&mut s.e, 0x009e_c810, move |pkg| (pkg == known) as u32);
        let list =
            s.e.call(0x0097_11e0, &args![s.lists, 0x55u32, s.nodes])
                .u32();
        assert_eq!(list_items(&s.e, list), vec![s.a[2]]);
    }

    #[test]
    fn yelling_an_alarm_without_a_target_returns_the_empty_list_unsorted() {
        let mut s = alarm_scene();
        stub(&mut s.e, 0x0084_e3a0, 0);
        s.e.call_log = Some(vec![]);
        let list =
            s.e.call(0x0097_11e0, &args![s.lists, 0x55u32, s.nodes])
                .u32();
        assert_ne!(list, 0);
        assert!(list_items(&s.e, list).is_empty());
        assert!(calls(&s.e, 0x0083_fd60).is_empty());
    }

    #[test]
    fn alarm_for_a_reference_alerts_the_processes_of_the_listeners() {
        let mut s = alarm_scene();
        let t = s.t;
        let player_object = object(&mut s.e);
        s.e.set_global(PLAYER, player_object);
        let player_process = object(&mut s.e);
        let processes = [object(&mut s.e), object(&mut s.e), object(&mut s.e)];
        let a = s.a;
        on_call(&mut s.e, ACTOR_PROCESS, move |o| {
            (0..3)
                .find(|i| a[*i] == o)
                .map_or(player_process, |i| processes[i])
        });
        on_slot(&mut s.e, 0x5c0, |_| 4);
        on_slot(&mut s.e, 0x304, |_| 1);
        stubs(&mut s.e, &[slot(0x5c4), slot(0x33c)]);
        on_call(&mut s.e, 0x0049_3bb0, move |o| (o == a[1]) as u32);
        s.e.call_log = Some(vec![]);
        let list = s.e.call(0x0097_15c0, &args![s.lists, t, s.nodes]).u32();
        let mut items = list_items(&s.e, list);
        items.sort_unstable();
        let mut expected = vec![a[0], a[2]];
        expected.sort_unstable();
        assert_eq!(items, expected);
        // The player's process hears `1 - count` (here -3).
        assert_eq!(
            calls(&s.e, slot(0x5c4)),
            vec![vec![player_process, -3i32 as u32]]
        );
        let told = calls(&s.e, slot(0x33c));
        assert_eq!(told.len(), 2);
        assert_eq!(
            told[0],
            vec![processes[0], a[0], t, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0]
        );
        assert_eq!(told[1][..3], [processes[2], a[2], t]);
        assert!(calls(&s.e, 0x0083_fd60).is_empty());
    }

    #[test]
    fn alarm_for_a_missing_reference_returns_an_empty_list() {
        let mut s = alarm_scene();
        s.e.call_log = Some(vec![]);
        let list = s.e.call(0x0097_15c0, &args![s.lists, 0u32, s.nodes]).u32();
        assert_ne!(list, 0);
        assert!(list_items(&s.e, list).is_empty());
        assert!(calls(&s.e, 0x0057_23b0).is_empty());
    }

    /// Level-3 actors with alarm packages for the crime functions: `a[0]` has
    /// the crime `crime` in its alarm package, `a[1]` a package of another
    /// type, `a[2]` an alarm package without the crime.
    fn crime_scene() -> (Engine, Ptr<ProcessLists>, [u32; 3], u32) {
        let mut e = fixture();
        let a = [object(&mut e), object(&mut e), object(&mut e)];
        let lists = lists_with(&mut e, 3, &[a[0], 0, a[1], a[2]]);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        on_call(&mut e, 0x0093_44a0, move |o| o + 0x1000);
        on_call(&mut e, PACKAGE_TYPE, move |p| {
            if p == a[1] + 0x1000 {
                0x16
            } else {
                0x15
            }
        });
        let crime = 0xc0de;
        on_call(&mut e, 0x009e_c810, move |p| (p == a[0] + 0x1000) as u32);
        stubs(&mut e, &[0x009e_c850, LIST_DELETE]);
        (e, lists, a, crime)
    }

    #[test]
    fn crime_alarm_lists_the_actors_whose_alarm_package_has_the_crime() {
        let (mut e, lists, a, crime) = crime_scene();
        let list = e.call(0x0097_19e0, &args![lists, crime, 0u8]).u32();
        assert_eq!(list_items(&e, list), vec![a[0]]);
        // Nothing to list: nothing allocated.
        stub(&mut e, 0x009e_c810, 0);
        assert_eq!(e.call(0x0097_19e0, &args![lists, crime, 0u8]).u32(), 0);
    }

    #[test]
    fn crime_alarm_can_remove_the_crime_instead() {
        let (mut e, lists, a, crime) = crime_scene();
        e.call_log = Some(vec![]);
        let result = e.call(0x0097_19e0, &args![lists, crime, 1u8]).u32();
        assert_eq!(result, 0);
        assert_eq!(
            calls(&e, 0x009e_c850),
            vec![vec![a[0] + 0x1000, crime, a[0]]]
        );
        let released = calls(&e, LIST_DELETE);
        assert_eq!(released.len(), 1);
        assert_eq!(released[0][1], 1);
    }

    #[test]
    fn turning_off_movement_resets_every_high_object() {
        let mut e = fixture();
        let (a, b) = (object(&mut e), object(&mut e));
        let lists = lists_with(&mut e, 0, &[a, 0, b]);
        stubs(&mut e, &[slot(0x204), 0x0089_5110]);
        e.call_log = Some(vec![]);
        e.call(0x0097_1ba0, &args![lists]);
        assert_eq!(calls(&e, slot(0x204)), vec![vec![a, 1], vec![b, 1]]);
        assert_eq!(
            calls(&e, 0x0089_5110),
            vec![
                vec![a, 0x3f80_0000, 0x3f80_0000],
                vec![b, 0x3f80_0000, 0x3f80_0000]
            ]
        );
    }

    /// Two level-3 actors for `fn_00971c30`, both also in level 0 and level 2.
    fn package_scene() -> (Engine, Ptr<ProcessLists>, [u32; 2], u32) {
        let mut e = fixture();
        let a = [object(&mut e), object(&mut e)];
        let lists = lists_with(&mut e, 3, &a);
        let array = lists.addr() + MOB_PROCESS_ARRAY;
        // Level 0 ends after the first actor, level 2 spans both.
        e.mem.set_u32(array + 0x20, 1);
        e.mem.set_u32(array + 0x18, 0);
        e.mem.set_u32(array + 0x28, 2);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, 0x0049_3bb0, 0);
        let owner = 0x0123_4567;
        (e, lists, a, owner)
    }

    #[test]
    fn package_search_lists_the_actors_running_a_package_of_the_kind() {
        let (mut e, lists, a, owner) = package_scene();
        on_call(
            &mut e,
            0x0093_44a0,
            move |o| if o == a[0] { 0x71 } else { 0x72 },
        );
        on_call(
            &mut e,
            PACKAGE_TYPE,
            |p| if p == 0x71 { 0x15 } else { 0x16 },
        );
        on_slot(&mut e, 0x304, |_| 1);
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        stub(&mut e, slot(0x128), owner);
        let list = e
            .call(0x0097_1c30, &args![lists, owner, 0x15u32, 0u8])
            .u32();
        assert_eq!(list_items(&e, list), vec![a[0]]);
        // An actor that does not answer slot 0x304 is out for kind 0x15.
        on_slot(&mut e, 0x304, |_| 0);
        assert_eq!(
            e.call(0x0097_1c30, &args![lists, owner, 0x15u32, 0u8])
                .u32(),
            0
        );
        // The process belongs to somebody else.
        on_slot(&mut e, 0x304, |_| 1);
        stub(&mut e, slot(0x128), owner + 1);
        assert_eq!(
            e.call(0x0097_1c30, &args![lists, owner, 0x15u32, 0u8])
                .u32(),
            0
        );
    }

    #[test]
    fn package_search_restricted_to_high_actors_stops_at_the_level_0_tail() {
        let (mut e, lists, a, owner) = package_scene();
        on_call(&mut e, 0x0093_44a0, |_| 0x71);
        on_call(&mut e, PACKAGE_TYPE, |_| 0x16);
        on_slot(&mut e, 0x304, |_| 1);
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        stub(&mut e, slot(0x128), owner);
        let list = e
            .call(0x0097_1c30, &args![lists, owner, 0x16u32, 1u8])
            .u32();
        assert_eq!(list_items(&e, list), vec![a[0]]);
    }

    #[test]
    fn package_search_for_combat_asks_the_combat_package() {
        let (mut e, lists, a, owner) = package_scene();
        stub(&mut e, 0x0049_3bb0, 1);
        on_call(&mut e, 0x0088_1510, |o| o + 0x100);
        on_call(&mut e, PACKAGE_TYPE, |_| 0x12);
        on_call(&mut e, 0x0040_30b0, move |_| owner);
        e.register_double(0x0097_fa10, move |_, w| Ret {
            eax: (w[0] == a[0] + 0x100 && w[1] == owner) as u32,
            ..Ret::default()
        });
        on_slot(&mut e, 0x21c, |_| 0);
        stub(&mut e, 0x0056_53d0, 1);
        stub(&mut e, slot(0x434), 0);
        e.call_log = Some(vec![]);
        // With a middle-level actor that persists and is not flagged: listed.
        let list = e
            .call(0x0097_1c30, &args![lists, owner, 0x12u32, 0u8])
            .u32();
        assert_eq!(list_items(&e, list), vec![a[0]]);
        // A flagged one is told slot 0x434 instead (the list exists but is empty).
        on_slot(&mut e, 0x21c, |_| 1);
        let list = e
            .call(0x0097_1c30, &args![lists, owner, 0x12u32, 0u8])
            .u32();
        assert_ne!(list, 0);
        assert!(list_items(&e, list).is_empty());
        assert_eq!(calls(&e, slot(0x434)), vec![vec![a[0], 0]]);
        // High only: the first-hand answer of `004030b0` decides (both
        // actors give the owner, but only the first is below the level-0 tail).
        on_slot(&mut e, 0x21c, |_| 0);
        let list = e
            .call(0x0097_1c30, &args![lists, owner, 0x12u32, 1u8])
            .u32();
        assert_eq!(list_items(&e, list), vec![a[0]]);
    }

    // --- 00971fd0 .. 009727c0: the crime lists ---

    /// Crimes are plain objects here: kind at +4, `0045cd60` field at +0x28,
    /// time at +0x34, known-by list at +0x38, actor at +0x10, victim at +0x14.
    /// Doubles for the list and crime accessors nobody has translated.
    fn crime_world() -> (Engine, Ptr<ProcessLists>) {
        let mut e = fixture();
        e.map(0x0102_0000, 0x1000);
        e.set_global(0x0102_0758u32, 10.0f64);
        let lists: Ptr<ProcessLists> = e.new_object();
        e.register(NODE_ITEM, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        e.register(LIST_IS_EMPTY, |e, a| Ret {
            eax: (e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32,
            ..Ret::default()
        });
        e.register(LIST_CONTAINS, |e, a| {
            let wanted = e.mem.u32(a[1]);
            let mut node = a[0];
            while node != 0 {
                if e.mem.u32(node) == wanted {
                    return Ret {
                        eax: 1,
                        ..Ret::default()
                    };
                }
                node = e.mem.u32(node + 4);
            }
            Ret::default()
        });
        e.register(0x009e_32d0, |e, a| Ret {
            eax: e.mem.u32(a[0] + 0x38),
            ..Ret::default()
        });
        e.register(PROCESS_LEVEL, |e, a| Ret {
            eax: e.mem.u32(a[0] + 0x28),
            ..Ret::default()
        });
        e.register(0x0084_e3a0, |e, a| Ret {
            eax: e.mem.u32(a[0] + 0x10),
            ..Ret::default()
        });
        e.register(0x0044_ddc0, |e, a| Ret {
            eax: e.mem.u32(a[0] + 0x14),
            ..Ret::default()
        });
        stubs(&mut e, &[LIST_DELETE, 0x005b_5e40]);
        (e, lists)
    }

    fn set_crime_list(e: &mut Engine, lists: Ptr<ProcessLists>, kind: u32, list: u32) {
        e.mem
            .set_u32(lists.addr() + GLOBAL_CRIME_LIST_ARRAY + 4 * kind, list);
    }

    fn crime_of_kind(e: &mut Engine, kind: u32) -> u32 {
        let crime = object(e);
        e.mem.set_u32(crime + 4, kind);
        crime
    }

    #[test]
    fn filing_a_crime_creates_the_list_and_adds_it_once() {
        let (mut e, lists) = crime_world();
        let crime = crime_of_kind(&mut e, 2);
        e.call(0x0097_21f0, &args![lists, crime]);
        let list = e.mem.u32(lists.addr() + GLOBAL_CRIME_LIST_ARRAY + 8);
        assert_ne!(list, 0);
        assert_eq!(list_items(&e, list), vec![crime]);
        e.call(0x0097_21f0, &args![lists, crime]);
        assert_eq!(list_items(&e, list), vec![crime]);
        // Kinds out of 0..5 and a null crime are ignored.
        let odd = crime_of_kind(&mut e, 5);
        e.call(0x0097_21f0, &args![lists, odd]);
        e.call(0x0097_21f0, &args![lists, 0u32]);
        assert_eq!(e.mem.u32(lists.addr() + GLOBAL_CRIME_LIST_ARRAY + 20), 0);
    }

    #[test]
    fn known_by_another_crime_looks_at_the_first_list_only() {
        let (mut e, lists) = crime_world();
        let (c1, c2) = (crime_of_kind(&mut e, 3), crime_of_kind(&mut e, 3));
        let actor = 0x4444;
        let known = make_list(&mut e, &[0x1111, actor]);
        e.mem.set_u32(c1 + 0x38, known);
        let first = make_list(&mut e, &[c1, c2]);
        e.mem.set_u32(lists.addr() + 0x50, first);
        let second = make_list(&mut e, &[c2]);
        e.mem.set_u32(lists.addr() + 0x54, second);
        // c1 is known by the actor: asking about c2 finds it.
        assert!(e.call(0x0097_22e0, &args![lists, actor, c2]).bool());
        // Asking about c1 skips it; c2 knows nobody.
        assert!(!e.call(0x0097_22e0, &args![lists, actor, c1]).bool());
        assert!(!e.call(0x0097_22e0, &args![lists, 0x9999u32, c2]).bool());
    }

    #[test]
    fn crime_cleanup_removes_the_crimes_that_should_go_and_sets_the_timer() {
        let (mut e, lists) = crime_world();
        let (c1, c2) = (crime_of_kind(&mut e, 0), crime_of_kind(&mut e, 0));
        let list = make_list(&mut e, &[c1, c2]);
        set_crime_list(&mut e, lists, 0, list);
        e.set(lists, ProcessLists::iNumberHighActors, 3);
        on_call(&mut e, 0x009e_b8a0, move |c| (c == c1) as u32);
        stubs(&mut e, &[0x009e_ba30, 0x008f_25e0]);
        e.call_log = Some(vec![]);
        e.call(0x0097_23f0, &args![lists]);
        assert_eq!(list_items(&e, list), vec![c2]);
        assert_eq!(calls(&e, 0x009e_ba30), vec![vec![c1]]);
        assert_eq!(calls(&e, 0x008f_25e0), vec![vec![c1, 1]]);
        assert_eq!(e.get(lists, ProcessLists::fCrimeUpdateTimer), 13.0);
    }

    #[test]
    fn crime_search_by_field_and_by_actor_and_victim() {
        let (mut e, lists) = crime_world();
        let (c1, c2) = (crime_of_kind(&mut e, 1), crime_of_kind(&mut e, 1));
        e.mem.set_u32(c1 + 0x28, 7);
        e.mem.set_u32(c2 + 0x28, 9);
        for (c, actor, victim) in [(c1, 0xa1, 0xb1), (c2, 0xa2, 0xb2)] {
            e.mem.set_u32(c + 0x10, actor);
            e.mem.set_u32(c + 0x14, victim);
        }
        let list = make_list(&mut e, &[c1, c2]);
        set_crime_list(&mut e, lists, 1, list);
        assert_eq!(e.call(0x0097_24e0, &args![lists, 1u32, 9u32]).u32(), c2);
        assert_eq!(e.call(0x0097_24e0, &args![lists, 1u32, 8u32]).u32(), 0);
        assert_eq!(e.call(0x0097_24e0, &args![lists, 6u32, 9u32]).u32(), 0);
        assert_eq!(e.call(0x0097_24e0, &args![lists, -1i32, 9u32]).u32(), 0);
        assert_eq!(
            e.call(0x0097_2570, &args![lists, 0xa1u32, 0xb1u32, 1u32])
                .u32(),
            c1
        );
        assert_eq!(
            e.call(0x0097_2570, &args![lists, 0xa1u32, 0xb2u32, 1u32])
                .u32(),
            0
        );
    }

    #[test]
    fn crime_test_checks_every_given_condition() {
        let (mut e, lists) = crime_world();
        let c1 = crime_of_kind(&mut e, 1);
        e.mem.set_u32(c1 + 0x10, 0xa1);
        e.mem.set_u32(c1 + 0x14, 0xb1);
        e.mem.set_u32(c1 + 0x28, 7);
        let list = make_list(&mut e, &[c1]);
        set_crime_list(&mut e, lists, 1, list);
        e.register_double(0x009e_b9a0, |_, a| Ret {
            eax: (a[1] == 0x55) as u32,
            ..Ret::default()
        });
        let test = |e: &mut Engine, victim: u32, actor: u32, extra: u32, kind: i32, value: i32| {
            e.call(
                0x0097_2600,
                &args![lists, victim, actor, extra, kind, 0u32, value],
            )
            .bool()
        };
        assert!(test(&mut e, 0xb1, 0xa1, 0, 1, -1));
        assert!(test(&mut e, 0, 0xa1, 0, 1, 7));
        assert!(test(&mut e, 0xb1, 0xa1, 0x55, 1, 7));
        assert!(!test(&mut e, 0xb1, 0xa1, 0x56, 1, 7));
        assert!(!test(&mut e, 0xb2, 0xa1, 0, 1, -1));
        assert!(!test(&mut e, 0xb1, 0xa2, 0, 1, -1));
        assert!(!test(&mut e, 0xb1, 0xa1, 0, 1, 8));
        assert!(!test(&mut e, 0xb1, 0xa1, 0, 2, -1));
        assert!(!test(&mut e, 0xb1, 0xa1, 0, 9, -1));
    }

    #[test]
    fn removing_a_crime_looks_in_the_list_of_its_kind() {
        let (mut e, lists) = crime_world();
        let (c1, c2) = (crime_of_kind(&mut e, 4), crime_of_kind(&mut e, 4));
        let list = make_list(&mut e, &[c1, c2]);
        set_crime_list(&mut e, lists, 4, list);
        e.call(0x0097_26e0, &args![lists, c2]);
        assert_eq!(list_items(&e, list), vec![c1]);
        // Out-of-range kinds and null crimes do nothing.
        let odd = crime_of_kind(&mut e, 6);
        e.call(0x0097_26e0, &args![lists, odd]);
        e.call(0x0097_26e0, &args![lists, 0u32]);
        assert_eq!(list_items(&e, list), vec![c1]);
    }

    #[test]
    fn crime_index_and_crime_by_index_walk_the_list() {
        let (mut e, lists) = crime_world();
        let (c1, c2, c3) = (
            crime_of_kind(&mut e, 2),
            crime_of_kind(&mut e, 2),
            crime_of_kind(&mut e, 2),
        );
        let list = make_list(&mut e, &[c1, c2, c3]);
        set_crime_list(&mut e, lists, 2, list);
        assert_eq!(e.call(0x0097_2740, &args![lists, 2u32, c3]).u16(), 2);
        assert_eq!(e.call(0x0097_27c0, &args![lists, 2u32, 1u32]).u32(), c2);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0097_2740, &args![lists, 2u32, 0x1234u32]).u16(), 0);
        assert_eq!(e.call(0x0097_27c0, &args![lists, 2u32, 3u32]).u32(), 0);
        assert_eq!(
            calls(&e, 0x005b_5e40),
            vec![vec![0x0108_c168], vec![0x0108_c1c0]]
        );
    }

    #[test]
    fn old_crimes_leave_the_list_and_are_returned() {
        let (mut e, lists) = crime_world();
        let (fresh, stale) = (crime_of_kind(&mut e, 3), crime_of_kind(&mut e, 3));
        e.mem.set_u32(fresh + 0x34, 95.0f32.to_bits());
        e.mem.set_u32(stale + 0x34, 50.0f32.to_bits());
        // The elapsed time is 100.0 minus the crime's time.
        e.register(0x008b_cc60, |e, a| {
            let time = e.mem.u32(a[0] + 0x34);
            e.mem.set_u32(a[1], time);
            Ret {
                eax: a[1],
                ..Ret::default()
            }
        });
        e.register_double(0x0043_5e00, |e, a| Ret {
            st0: 100.0 - f32::from_bits(e.mem.u32(a[0])) as f64,
            ..Ret::default()
        });
        let knower = 0x4242;
        let known = make_list(&mut e, &[knower]);
        e.mem.set_u32(fresh + 0x38, known);
        let victim = object(&mut e);
        e.mem.set_u32(fresh + 0x14, victim);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o);
        let player_crimes = make_list(&mut e, &[fresh, 0x7777]);
        stub(&mut e, 0x0041_cf00, player_crimes);
        stub(&mut e, 0x0047_eb90, 0);
        let list = make_list(&mut e, &[fresh, stale]);
        set_crime_list(&mut e, lists, 3, list);
        e.call_log = Some(vec![]);
        let removed = e.call(0x0097_1fd0, &args![lists, 10.0f32]).u32();
        assert_eq!(list_items(&e, removed), vec![fresh]);
        assert_eq!(list_items(&e, list), vec![stale]);
        assert_eq!(list_items(&e, player_crimes), vec![0x7777]);
        assert_eq!(calls(&e, 0x0047_eb90), vec![vec![knower, 0]]);
    }

    #[test]
    fn crime_removal_releases_the_empty_result() {
        let (mut e, lists) = crime_world();
        let stale = crime_of_kind(&mut e, 3);
        e.mem.set_u32(stale + 0x34, 50.0f32.to_bits());
        e.register(0x008b_cc60, |e, a| {
            let time = e.mem.u32(a[0] + 0x34);
            e.mem.set_u32(a[1], time);
            Ret {
                eax: a[1],
                ..Ret::default()
            }
        });
        e.register_double(0x0043_5e00, |e, a| Ret {
            st0: 100.0 - f32::from_bits(e.mem.u32(a[0])) as f64,
            ..Ret::default()
        });
        let list = make_list(&mut e, &[stale]);
        set_crime_list(&mut e, lists, 3, list);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0097_1fd0, &args![lists, 10.0f32]).u32(), 0);
        assert_eq!(list_items(&e, list), vec![stale]);
        assert_eq!(calls(&e, LIST_DELETE).len(), 1);
    }

    // --- 00972840 .. 00972d30 ---

    #[test]
    fn forgetting_a_target_calms_the_alarmed_and_tells_the_ones_in_combat() {
        let mut e = fixture();
        let (fighter, calm, other, thing) = (
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
        );
        set_objects(
            &mut e,
            PROCESS_LISTS + MOB_PROCESS_ARRAY,
            3,
            &[fighter, calm, other, thing],
        );
        let target = 0x7001u32;
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != thing) as u32);
        on_call(&mut e, 0x008b_c700, move |a| (a == fighter) as u32);
        stubs(&mut e, &[slot(0x434), 0x0088_1680, slot(0x644)]);
        stub(&mut e, 0x008a_61b0, 1);
        on_call(&mut e, 0x0093_44a0, |a| a + 0x1000);
        on_call(&mut e, 0x0067_1d10, |package| package + 1);
        on_call(&mut e, 0x0068_0020, move |holder| {
            if holder == calm + 0x1001 {
                target
            } else {
                0x7002
            }
        });
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        e.set_global(PLAYER, 0x1234u32);
        e.call_log = Some(vec![]);
        e.call(0x0097_2840, &args![0u32, target, 0u8]);
        assert_eq!(calls(&e, slot(0x434)), vec![vec![fighter, target]]);
        assert_eq!(calls(&e, 0x0088_1680), vec![vec![calm, 0]]);
        assert_eq!(calls(&e, slot(0x644)), vec![vec![process, 1]]);
        // A null target matches every alarmed actor; `keep_package` skips them.
        e.call_log = Some(vec![]);
        e.call(0x0097_2840, &args![0u32, 0u32, 0u8]);
        assert_eq!(calls(&e, 0x0088_1680).len(), 2);
        e.call_log = Some(vec![]);
        e.call(0x0097_2840, &args![0u32, 0u32, 1u8]);
        assert!(calls(&e, 0x0088_1680).is_empty());
    }

    #[test]
    fn forgetting_the_player_clears_crime_extras_and_faction_entries() {
        let mut e = fixture();
        let actor = object(&mut e);
        set_objects(&mut e, PROCESS_LISTS + MOB_PROCESS_ARRAY, 3, &[actor]);
        let player_ref = 0x7003u32;
        e.set_global(PLAYER, player_ref);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        stub(&mut e, 0x008b_c700, 0);
        stub(&mut e, 0x008a_61b0, 0);
        stubs(&mut e, &[0x008b_cb40, 0x0041_cf30, 0x0047_eb90]);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o + 0x10);
        on_slot(&mut e, 0x21c, |_| 1);
        on_call(&mut e, 0x0041_81e0, |a| a);
        on_call(&mut e, 0x005d_8a70, |a| a + 0x2c);
        let record = e.mem.alloc(0x20);
        e.mem.set_u32(record + 0xc, 0x66);
        on_call(&mut e, 0x0042_e800, move |_| record);
        // The faction integration fills two entries and returns 3 (the middle one empty).
        e.register(0x008b_8ca0, |e, a| {
            e.mem.set_u32(a[1], 0xf1);
            e.mem.set_u32(a[1] + 4, 0);
            e.mem.set_u32(a[1] + 8, 0xf2);
            Ret {
                eax: 3,
                ..Ret::default()
            }
        });
        on_call(&mut e, 0x005a_2270, |entry| (entry == 0xf2) as u32);
        e.call_log = Some(vec![]);
        e.call(0x0097_2840, &args![0u32, player_ref, 0u8]);
        assert_eq!(calls(&e, 0x0041_cf30), vec![vec![actor + 0x10, actor]]);
        let integrate = calls(&e, 0x008b_8ca0);
        assert_eq!(integrate[0][0], actor);
        assert_eq!(integrate[0][2..], [0x80, actor + 0x30 + 0x2c, 0x66]);
        assert_eq!(calls(&e, 0x0047_eb90), vec![vec![0xf2, 0]]);
    }

    #[test]
    fn forgetting_an_actors_crimes_notifies_those_who_knew_them() {
        let (mut e, lists) = crime_world();
        let actor = 0xaaaa;
        let (mine, theirs) = (crime_of_kind(&mut e, 1), crime_of_kind(&mut e, 1));
        e.mem.set_u32(mine + 0x10, actor);
        e.mem.set_u32(theirs + 0x10, 0xbbbb);
        let known = make_list(&mut e, &[0x5151]);
        e.mem.set_u32(mine + 0x38, known);
        let list = make_list(&mut e, &[mine, theirs]);
        set_crime_list(&mut e, lists, 1, list);
        stub(&mut e, 0x0047_eb90, 0);
        e.call_log = Some(vec![]);
        e.call(0x0097_2aa0, &args![lists, actor]);
        assert_eq!(list_items(&e, list), vec![theirs]);
        assert_eq!(calls(&e, 0x0047_eb90), vec![vec![0x5151, 0]]);
    }

    #[test]
    fn level_zero_actors_are_woken_or_given_a_pose() {
        let mut e = fixture();
        let (alive, tired, rested, thing) = (
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
        );
        let lists = lists_with(&mut e, 0, &[alive, tired, rested, thing]);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != thing) as u32);
        on_slot(&mut e, SLOT_0X22C, move |o| (o == alive) as u32);
        on_call(&mut e, 0x0041_81e0, |a| a);
        on_call(&mut e, 0x005f_0b00, |_| 5);
        stub(&mut e, slot(0x9c), 0);
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        stubs(&mut e, &[slot(0x4d8), slot(0x418)]);
        e.register_double(0x0089_3530, move |_, a| Ret {
            st0: if a[0] == tired { 0.0 } else { 0.5 },
            ..Ret::default()
        });
        // The embedded object at +0x30 of the base data answers slot 0x60.
        e.mem.set_u32(tired + 0x30, VTABLE);
        e.mem.set_u32(rested + 0x30, VTABLE);
        on_slot(&mut e, 0x60, |_| 3);
        let vector = e.mem.alloc(12);
        for (i, v) in [1u32, 2, 3].iter().enumerate() {
            e.mem.set_u32(vector + 4 * i as u32, *v);
        }
        on_slot(&mut e, 0x1f4, move |_| vector);
        e.call_log = Some(vec![]);
        e.call(0x0097_2bb0, &args![lists]);
        assert_eq!(calls(&e, slot(0x4d8)), vec![vec![process, alive]]);
        assert_eq!(calls(&e, slot(0x418)), vec![vec![process, tired, 1, 2, 3]]);
        // A tired actor whose base data answers 0 to slot 0x60 is left alone.
        on_slot(&mut e, 0x60, |_| 0);
        e.call_log = Some(vec![]);
        e.call(0x0097_2bb0, &args![lists]);
        assert!(calls(&e, slot(0x418)).is_empty());
    }

    /// One level-0 actor with the doubles `fn_00972d30` needs. `package_type`
    /// and `target` are what its package and package target are.
    struct BedScene {
        e: Engine,
        lists: Ptr<ProcessLists>,
        actor: u32,
        marker_data: u32,
    }

    fn bed_scene(package_type: u32, target: u32) -> BedScene {
        let mut e = fixture();
        let actor = object(&mut e);
        let lists = lists_with(&mut e, 0, &[actor]);
        let marker_data = 0x55;
        let tes = e.mem.alloc(0x10);
        e.set_global(TES, tes);
        e.set_global(0x0120_2d98u32, 0x4d4d_0000u32);
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        e.register(PROCESS_LEVEL, |e, a| Ret {
            eax: e.mem.u32(a[0] + 0x28),
            ..Ret::default()
        });
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        on_slot(&mut e, SLOT_0X22C, |_| 0);
        on_slot(&mut e, 0x218, |_| 1);
        on_slot(&mut e, 0x214, |_| 0);
        stubs(&mut e, &[0x0049_38e0, 0x008a_3b30, 0x008c_3c40]);
        stub(&mut e, 0x0057_5ca0, 7);
        stub(&mut e, 0x0088_6080, 0);
        stub(&mut e, 0x0088_1650, target);
        let package = object(&mut e);
        stub(&mut e, 0x0093_44a0, package);
        stub(&mut e, PACKAGE_TYPE, package_type);
        stub(&mut e, slot(0x514), 0);
        stub(&mut e, slot(0x4d4), marker_data);
        stub(&mut e, 0x0056_8680, 1);
        on_call(&mut e, 0x007a_f430, |furniture| furniture);
        stub(&mut e, 0x0056_7790, 0);
        stub(&mut e, slot(0x1d0), 0x999);
        stub(&mut e, 0x0088_d2f0, 1);
        stubs(&mut e, &[0x00c3_dfa0, 0x0045_0b60]);
        stub(&mut e, slot(0x13c), 1);
        stub(&mut e, 0x0057_9670, 1);
        stub(&mut e, 0x0089_1d30, 1);
        stub(&mut e, 0x008a_7870, 1);
        stub(&mut e, 0x0056_82c0, 2);
        stub(&mut e, 0x0056_8500, 1);
        stubs(&mut e, &[0x0050_9420, 0x0050_93f0, 0x0055_ac00]);
        e.register(NODE_ITEM, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        e.register(LIST_IS_EMPTY, |e, a| Ret {
            eax: (e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32,
            ..Ret::default()
        });
        BedScene {
            e,
            lists,
            actor,
            marker_data,
        }
    }

    #[test]
    fn an_actor_is_put_on_the_furniture_its_package_targets() {
        let mut s = bed_scene(1, 0xbed0);
        stub(&mut s.e, 0x0050_9420, 1);
        s.e.call_log = Some(vec![]);
        s.e.call(0x0097_2d30, &args![s.lists]);
        assert_eq!(
            calls(&s.e, 0x0088_d2f0),
            vec![vec![s.actor, 0xbed0, s.marker_data, 2, 1]]
        );
        // Something was placed: the task manager is told.
        assert_eq!(calls(&s.e, 0x00c3_dfa0), vec![vec![0x4d4d_0000, 0]]);
        // A chair target is not a bed.
        stub(&mut s.e, 0x0050_9420, 0);
        s.e.call_log = Some(vec![]);
        s.e.call(0x0097_2d30, &args![s.lists]);
        assert_eq!(
            calls(&s.e, 0x0088_d2f0),
            vec![vec![s.actor, 0xbed0, s.marker_data, 2, 0]]
        );
    }

    #[test]
    fn an_actor_whose_furniture_has_no_free_marker_is_moved_to_the_navmesh() {
        let mut s = bed_scene(1, 0xbed0);
        stub(&mut s.e, 0x0056_82c0, -1i32 as u32);
        let vector = s.e.mem.alloc(12);
        for (i, v) in [11u32, 12, 13].iter().enumerate() {
            s.e.mem.set_u32(vector + 4 * i as u32, *v);
        }
        on_slot(&mut s.e, 0x1f4, move |_| vector);
        stub(&mut s.e, 0x008d_6f30, 0xce11);
        stub(&mut s.e, 0x0057_5d70, 0xa5a5);
        s.e.register_double(0x006d_6f80, |e, a| {
            // The navmesh moves the point.
            e.mem.set_u32(a[3], 99);
            Ret::default()
        });
        stubs(&mut s.e, &[0x0057_5830, 0x0056_2020]);
        s.e.call_log = Some(vec![]);
        s.e.call(0x0097_2d30, &args![s.lists]);
        assert!(calls(&s.e, 0x0088_d2f0).is_empty());
        let navmesh = calls(&s.e, 0x006d_6f80);
        assert_eq!(navmesh[0][..3], [0xa5a5, 0xce11, vector]);
        let moved = calls(&s.e, 0x0057_5830);
        assert_eq!(moved[0][0], s.actor);
        assert_eq!(moved[0][1], navmesh[0][3]);
        assert_eq!(s.e.mem.u32(moved[0][1]), 99);
        assert_eq!(calls(&s.e, 0x0056_2020), vec![vec![s.actor]]);
        // Nothing was placed: the task manager is not told.
        assert!(calls(&s.e, 0x00c3_dfa0).is_empty());
    }

    #[test]
    fn a_sleeping_actor_takes_the_first_usable_bed_and_drops_the_unusable_ones() {
        let mut s = bed_scene(4, 0);
        let (broken, bed) = (0xb1u32, 0xb2u32);
        let furniture = make_list(&mut s.e, &[broken, bed]);
        on_call(&mut s.e, 0x0055_ac00, move |_| furniture);
        on_call(&mut s.e, NODE_ITEM, |node| node);
        stub(&mut s.e, 0x0050_9420, 1);
        // `broken` has no free marker.
        on_call(&mut s.e, 0x0056_82c0, move |f| {
            if f == broken {
                -1i32 as u32
            } else {
                4
            }
        });
        stub(&mut s.e, slot(0x284), 0);
        s.e.call_log = Some(vec![]);
        s.e.call(0x0097_2d30, &args![s.lists]);
        assert_eq!(
            calls(&s.e, 0x0088_d2f0),
            vec![vec![s.actor, bed, s.marker_data, 4, 1]]
        );
        assert_eq!(list_items(&s.e, furniture), vec![bed]);
        // The actor was not yet sleeping (`00579670`): its process is told slot 0x284.
        stub(&mut s.e, 0x0057_9670, 0);
        s.e.call_log = Some(vec![]);
        s.e.call(0x0097_2d30, &args![s.lists]);
        assert_eq!(calls(&s.e, slot(0x284)).len(), 1);
    }

    #[test]
    fn an_eating_actor_takes_a_chair_in_its_space() {
        let mut s = bed_scene(3, 0);
        let (far, chair) = (0xc1u32, 0xc2u32);
        let furniture = make_list(&mut s.e, &[far, chair]);
        on_call(&mut s.e, 0x0055_ac00, move |_| furniture);
        on_call(&mut s.e, NODE_ITEM, |node| node);
        on_call(&mut s.e, 0x0057_5ca0, move |o| if o == far { 8 } else { 7 });
        on_call(&mut s.e, 0x0050_93f0, |f| (f != 0) as u32);
        stub(&mut s.e, 0x008a_7870, 0);
        stubs(&mut s.e, &[slot(0x288), slot(0x200)]);
        s.e.call_log = Some(vec![]);
        s.e.call(0x0097_2d30, &args![s.lists]);
        assert_eq!(
            calls(&s.e, 0x0088_d2f0),
            vec![vec![s.actor, chair, s.marker_data, 2, 0]]
        );
        // The actor is not eating yet: process slot 0x288 and its own slot 0x200.
        assert_eq!(calls(&s.e, slot(0x288)).len(), 1);
        assert_eq!(calls(&s.e, slot(0x288))[0][1..], [s.actor, 1]);
        assert_eq!(calls(&s.e, slot(0x200)), vec![vec![s.actor, 1]]);
    }

    #[test]
    fn actors_that_are_busy_or_not_at_level_zero_are_left_alone() {
        let mut s = bed_scene(1, 0xbed0);
        on_slot(&mut s.e, 0x214, |_| 1);
        s.e.call_log = Some(vec![]);
        s.e.call(0x0097_2d30, &args![s.lists]);
        assert!(calls(&s.e, 0x008c_3c40).is_empty());
        assert!(calls(&s.e, 0x00c3_dfa0).is_empty());
        // The process level is not 0.
        on_slot(&mut s.e, 0x214, |_| 0);
        let process = object(&mut s.e);
        s.e.mem.set_u32(process + 0x28, 1);
        stub(&mut s.e, ACTOR_PROCESS, process);
        s.e.call(0x0097_2d30, &args![s.lists]);
        assert!(calls(&s.e, 0x008c_3c40).is_empty());
    }

    // --- 00973460 .. 00974960 ---

    #[test]
    fn follower_check_looks_for_an_actor_that_is_not_excused() {
        let mut e = fixture();
        let (idle, hunter, thing) = (object(&mut e), object(&mut e), object(&mut e));
        let lists = lists_with(&mut e, 0, &[idle, thing, hunter]);
        let reference = 0x7001u32;
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != thing) as u32);
        on_slot(&mut e, 0x218, |_| 1);
        on_call(
            &mut e,
            0x008a_81e0,
            move |a| if a == hunter { 3 } else { 1 },
        );
        stub(&mut e, 0x0049_3bb0, 0);
        assert!(e.call(0x0097_3460, &args![lists, reference, 0u32]).bool());
        // In combat with a package target that is the reference: excused.
        stub(&mut e, 0x0049_3bb0, 1);
        let package = object(&mut e);
        stub(&mut e, slot(0x428), package);
        stub(&mut e, 0x0040_30b0, reference);
        stub(&mut e, 0x0098_18b0, 1);
        assert!(!e.call(0x0097_3460, &args![lists, reference, 0u32]).bool());
        // Not excused any more once the package no longer holds.
        stub(&mut e, 0x0098_18b0, 0);
        assert!(e.call(0x0097_3460, &args![lists, reference, 0u32]).bool());
        // No package: found.
        stub(&mut e, slot(0x428), 0);
        assert!(e.call(0x0097_3460, &args![lists, reference, 0u32]).bool());
        // Nobody with the value 3: not found.
        stub(&mut e, 0x008a_81e0, 1);
        assert!(!e.call(0x0097_3460, &args![lists, reference, 0u32]).bool());
    }

    /// Doubles for the `BSSimpleList` helpers of the follower functions.
    fn list_helper_doubles(e: &mut Engine) {
        e.register(NODE_ITEM, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        e.register(LIST_IS_EMPTY, |e, a| Ret {
            eax: (e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32,
            ..Ret::default()
        });
        e.register(LIST_CONTAINS, |e, a| {
            let wanted = e.mem.u32(a[1]);
            let mut node = a[0];
            while node != 0 {
                if e.mem.u32(node) == wanted {
                    return Ret {
                        eax: 1,
                        ..Ret::default()
                    };
                }
                node = e.mem.u32(node + 4);
            }
            Ret::default()
        });
    }

    #[test]
    fn follower_list_collects_followers_and_actors_following_a_package() {
        let mut e = fixture();
        list_helper_doubles(&mut e);
        let (reference, follower, escort, stranger, thing) = (
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
        );
        let lists = lists_with(&mut e, 0, &[escort, stranger, thing]);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != thing) as u32);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o + 1);
        // The reference's follower record lists `follower`; nobody else has one.
        let record = e.mem.alloc(0x20);
        let followers = make_list(&mut e, &[follower]);
        e.mem.set_u32(record + 0xc, followers);
        on_call(&mut e, EXTRA_FOLLOWERS, move |extra| {
            if extra == reference + 1 {
                record
            } else {
                0
            }
        });
        on_call(&mut e, 0x0093_44a0, |a| a + 0x1000);
        on_call(&mut e, PACKAGE_TYPE, move |p| {
            if p == escort + 0x1000 {
                7
            } else if p == stranger + 0x1000 {
                9
            } else {
                0
            }
        });
        on_call(&mut e, 0x0088_1650, move |a| {
            if a == escort || a == stranger {
                reference
            } else {
                0
            }
        });
        let list = make_list(&mut e, &[]);
        e.call(0x0097_3590, &args![lists, reference, list]);
        let mut items = list_items(&e, list);
        items.sort_unstable();
        let mut expected = vec![follower, escort];
        expected.sort_unstable();
        assert_eq!(items, expected);
        // Running again adds nothing: they are in the list already.
        e.call(0x0097_3590, &args![lists, reference, list]);
        assert_eq!(list_items(&e, list).len(), 2);
    }

    /// Level-0 actors for `fn_00973710` with the values their processes give.
    fn noticing_scene(values: &[u32]) -> (Engine, Ptr<ProcessLists>, Vec<u32>, u32) {
        let mut e = fixture();
        let actors: Vec<u32> = values.iter().map(|_| object(&mut e)).collect();
        let lists = lists_with(&mut e, 0, &actors);
        let player_object = e.mem.alloc(0x800);
        e.set_global(PLAYER, player_object);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        on_slot(&mut e, SLOT_0X22C, |_| 0);
        stubs(
            &mut e,
            &[
                0x0044_0da0,
                0x0043_7bd0,
                0x0049_3bb0,
                0x008a_61b0,
                0x008b_06d0,
                0x008c_8bb0,
            ],
        );
        let processes: Vec<u32> = values.iter().map(|_| object(&mut e)).collect();
        let (list_a, list_p) = (actors.clone(), processes.clone());
        on_call(&mut e, ACTOR_PROCESS, move |a| {
            list_p[list_a.iter().position(|x| *x == a).unwrap()]
        });
        let (list_p, values) = (processes, values.to_vec());
        on_slot(&mut e, 0x2d4, move |p| {
            values[list_p.iter().position(|x| *x == p).unwrap()]
        });
        let reference = 0x7002;
        (e, lists, actors, reference)
    }

    #[test]
    fn noticing_is_the_highest_clamped_value() {
        let (mut e, lists, _, reference) = noticing_scene(&[30, 0x7fff_ffff, 150, -500i32 as u32]);
        let result = e.call(0x0097_3710, &args![lists, reference, 1u8]).i32();
        assert_eq!(result, 100);
        let (mut e, lists, _, reference) = noticing_scene(&[0x7fff_ffff, -500i32 as u32]);
        let result = e.call(0x0097_3710, &args![lists, reference, 1u8]).i32();
        assert_eq!(result, 0);
        // A positive value of an actor that wants to attack the player is
        // reported to `008c8bb0`.
        let (mut e, lists, _, reference) = noticing_scene(&[40]);
        stub(&mut e, 0x008b_06d0, 1);
        e.call_log = Some(vec![]);
        e.call(0x0097_3710, &args![lists, reference, 1u8]);
        let player_object: u32 = e.global(PLAYER);
        assert_eq!(calls(&e, 0x008c_8bb0), vec![vec![player_object, 1]]);
    }

    #[test]
    fn noticing_without_actors_is_zero() {
        let (mut e, lists, _, reference) = noticing_scene(&[]);
        assert_eq!(e.call(0x0097_3710, &args![lists, reference, 1u8]).i32(), 0);
    }

    #[test]
    fn an_actor_in_combat_with_the_reference_is_noticed_fully() {
        let (mut e, lists, actors, reference) = noticing_scene(&[10, 10]);
        let fighter = actors[1];
        on_call(&mut e, 0x0049_3bb0, move |a| (a == fighter) as u32);
        let package = object(&mut e);
        stub(&mut e, slot(0x428), package);
        stub(&mut e, 0x0040_30b0, reference);
        stub(&mut e, 0x0098_18b0, 0);
        assert_eq!(
            e.call(0x0097_3710, &args![lists, reference, 1u8]).i32(),
            100
        );
        // An alarmed actor whose package target is the player counts as well.
        let (mut e, lists, actors, reference) = noticing_scene(&[10]);
        let alarmed = actors[0];
        on_call(&mut e, 0x008a_61b0, move |a| (a == alarmed) as u32);
        let player_object: u32 = e.global(PLAYER);
        stub(&mut e, 0x0088_1650, player_object);
        assert_eq!(
            e.call(0x0097_3710, &args![lists, reference, 1u8]).i32(),
            100
        );
    }

    #[test]
    fn noticing_sets_the_flag_for_a_nearby_actor_when_the_reference_is_the_player() {
        let (mut e, lists, _, _) = noticing_scene(&[10, 70]);
        list_helper_doubles(&mut e);
        stubs(&mut e, &[0x0046_ffb0, 0x0056_6950]);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o + 1);
        stub(&mut e, EXTRA_FOLLOWERS, 0);
        stub(&mut e, 0x0093_44a0, 0);
        let player_object: u32 = e.global(PLAYER);
        // The distance setting is 50 (the value sits 0x100 after the setting).
        e.set_global(0x011c_d950u32, 50u32);
        stub_f32(&mut e, 0x0057_23b0, 20.0);
        assert_eq!(
            e.call(0x0097_3710, &args![lists, player_object, 0u8]).i32(),
            70
        );
        assert_eq!(e.mem.u8(player_object + 0x66c), 1);
        // Too far: the flag stays clear.
        stub_f32(&mut e, 0x0057_23b0, 80.0);
        e.call(0x0097_3710, &args![lists, player_object, 0u8]);
        assert_eq!(e.mem.u8(player_object + 0x66c), 0);
    }

    #[test]
    fn noticing_leaves_out_the_followers_of_the_reference() {
        let (mut e, lists, actors, reference) = noticing_scene(&[90, 20]);
        list_helper_doubles(&mut e);
        stubs(&mut e, &[0x0046_ffb0, 0x0056_6950]);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o + 1);
        // The reference has the first actor as a follower.
        let record = e.mem.alloc(0x20);
        let followers = make_list(&mut e, &[actors[0]]);
        e.mem.set_u32(record + 0xc, followers);
        on_call(&mut e, EXTRA_FOLLOWERS, move |extra| {
            if extra == reference + 1 {
                record
            } else {
                0
            }
        });
        stub(&mut e, 0x0093_44a0, 0);
        // The follower (value 90) is skipped: the result is the second actor's.
        assert_eq!(e.call(0x0097_3710, &args![lists, reference, 0u8]).i32(), 20);
        // Without followers the highest value wins.
        let (mut e, lists, _, reference) = noticing_scene(&[90, 20]);
        list_helper_doubles(&mut e);
        stubs(&mut e, &[0x0046_ffb0, 0x0056_6950]);
        on_call(&mut e, EXTRA_DATA_LIST, |o| o + 1);
        stub(&mut e, EXTRA_FOLLOWERS, 0);
        stub(&mut e, 0x0093_44a0, 0);
        assert_eq!(e.call(0x0097_3710, &args![lists, reference, 0u8]).i32(), 90);
    }

    #[test]
    fn the_flag_at_0x66c_is_a_plain_byte() {
        let mut e = fixture();
        let player_object = e.mem.alloc(0x800);
        e.call(0x0097_3a70, &args![Ptr::<()>::new(player_object), 1u8]);
        assert_eq!(e.mem.u8(player_object + 0x66c), 1);
        assert_eq!(
            e.call(0x0097_3a90, &args![Ptr::<()>::new(player_object)])
                .u8(),
            1
        );
        e.call(0x0097_3a70, &args![Ptr::<()>::new(player_object), 0u8]);
        assert_eq!(
            e.call(0x0097_3a90, &args![Ptr::<()>::new(player_object)])
                .u8(),
            0
        );
    }

    #[test]
    fn the_seller_search_finds_the_first_owner_who_buys_the_item() {
        let mut e = fixture();
        let (a, b, c) = (object(&mut e), object(&mut e), object(&mut e));
        let lists = lists_with(&mut e, 0, &[a, 0, b, c]);
        let item = 0x7010u32;
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        // `a` does not own the item; `b` owns it but does not buy; `c` does both.
        e.register_double(0x0057_85e0, move |_, w| Ret {
            eax: (w[0] == item && w[1] != a) as u32,
            ..Ret::default()
        });
        stub(&mut e, 0x007a_f430, 0x5555);
        on_call(&mut e, 0x0041_81e0, |actor| actor);
        e.register_double(0x0047_f600, move |_, w| Ret {
            eax: (w[0] == c + 0x90 && w[1] == 0x5555) as u32,
            ..Ret::default()
        });
        assert_eq!(e.call(0x0097_3ab0, &args![lists, item]).u32(), c);
        stub(&mut e, 0x0057_85e0, 0);
        assert_eq!(e.call(0x0097_3ab0, &args![lists, item]).u32(), 0);
    }

    #[test]
    fn the_level_zero_processes_are_told_about_a_reference() {
        let mut e = fixture();
        let (a, b, thing) = (object(&mut e), object(&mut e), object(&mut e));
        let lists = lists_with(&mut e, 0, &[a, thing, b]);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o != thing) as u32);
        let (pa, pb) = (object(&mut e), object(&mut e));
        on_call(&mut e, ACTOR_PROCESS, move |o| if o == a { pa } else { pb });
        stub(&mut e, slot(0x2c4), 0);
        e.call_log = Some(vec![]);
        e.call(0x0097_3cb0, &args![lists, 0x7777u32]);
        assert_eq!(
            calls(&e, slot(0x2c4)),
            vec![vec![pa, 0x7777, 3], vec![pb, 0x7777, 3]]
        );
    }

    #[test]
    fn a_form_search_over_level_zero_compares_the_base_form_value() {
        let mut e = fixture();
        let (a, b) = (object(&mut e), object(&mut e));
        let lists = lists_with(&mut e, 0, &[a, b]);
        on_call(&mut e, 0x0057_4900, move |o| (o == b) as u32);
        on_call(&mut e, 0x007a_f430, |o| o + 0x10);
        on_call(
            &mut e,
            0x0051_6bf0,
            move |form| {
                if form == b + 0x10 {
                    0x42
                } else {
                    0
                }
            },
        );
        assert!(e.call(0x0097_3d50, &args![lists, 0x42u32]).bool());
        assert!(!e.call(0x0097_3d50, &args![lists, 0x43u32]).bool());
    }

    /// Actors for the two package-ending sweeps: one far from the player, one
    /// near, one that asks for a forced update, and one skipped by slot 0x22c.
    fn sweep_scene() -> (Engine, Ptr<ProcessLists>, [u32; 4]) {
        let mut e = fixture();
        let actors = [
            object(&mut e),
            object(&mut e),
            object(&mut e),
            object(&mut e),
        ];
        let lists = lists_with(&mut e, 0, &actors);
        // `0055b980`, the object count of the array.
        stub(&mut e, 0x0055_b980, 4);
        let player_object = object(&mut e);
        e.set_global(PLAYER, player_object);
        e.map(0x0102_c000, 0x1000);
        e.set_global(0x0102_c188u32, 350.0f64);
        on_slot(&mut e, SLOT_IS_ACTOR, |_| 1);
        let skipped = actors[3];
        on_slot(&mut e, SLOT_0X22C, move |o| (o == skipped) as u32);
        stubs(&mut e, &[0x0088_1680, 0x008a_d910]);
        (e, lists, actors)
    }

    #[test]
    fn far_actors_and_forced_updates_end_their_package() {
        let (mut e, lists, actors) = sweep_scene();
        let player_object: u32 = e.global(PLAYER);
        let (far, forced) = (actors[0], actors[2]);
        stub(&mut e, 0x0095_49a0, 1);
        e.register_double(0x0057_23b0, move |_, w| Ret {
            st0: if w[1] == far { 400.0 } else { 100.0 },
            ..Ret::default()
        });
        on_call(&mut e, 0x0056_6950, move |o| (o == forced) as u32);
        e.call_log = Some(vec![]);
        e.call(0x0097_3de0, &args![lists]);
        assert_eq!(calls(&e, 0x0088_1680), vec![vec![far, 0], vec![forced, 0]]);
        assert_eq!(
            calls(&e, 0x008a_d910),
            vec![
                vec![far, player_object, 0, 0],
                vec![forced, player_object, 0, 0]
            ]
        );
        // When the player is not checking, only the forced update counts.
        stub(&mut e, 0x0095_49a0, 0);
        e.call_log = Some(vec![]);
        e.call(0x0097_3de0, &args![lists]);
        assert_eq!(calls(&e, 0x0088_1680), vec![vec![forced, 0]]);
    }

    #[test]
    fn the_reference_sweep_needs_a_reference() {
        let (mut e, lists, actors) = sweep_scene();
        let target = 0x7020u32;
        let busy = actors[1];
        e.register_double(0x0095_4a70, move |_, w| Ret {
            eax: (w[1] == busy) as u32,
            ..Ret::default()
        });
        stub(&mut e, 0x0056_6950, 0);
        e.call_log = Some(vec![]);
        e.call(0x0097_3ee0, &args![lists, target]);
        assert_eq!(calls(&e, 0x0088_1680), vec![vec![busy, 0]]);
        assert_eq!(calls(&e, 0x008a_d910), vec![vec![busy, target, 0, 0]]);
        e.call_log = Some(vec![]);
        e.call(0x0097_3ee0, &args![lists, 0u32]);
        assert!(calls(&e, 0x0088_1680).is_empty());
    }

    /// Writes `items` as a `BSSimpleList` whose head node is embedded at `head`.
    fn embed_list(e: &mut Engine, head: u32, items: &[u32]) {
        let mut node = head;
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(node, *item);
            if i + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
    }

    /// Doubles for the `NiPointer` lists of the temporary effects.
    fn effect_list_doubles(e: &mut Engine) {
        e.register(NODE_ITEM, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        e.register(NI_POINTER_VALUE, |e, a| Ret {
            eax: e.mem.u32(a[0]),
            ..Ret::default()
        });
        e.register(NI_POINTER_LIST_IS_EMPTY, |e, a| Ret {
            eax: (e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0) as u32,
            ..Ret::default()
        });
        e.register(NI_POINTER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        e.register(NI_POINTER_LIST_REMOVE, |e, a| {
            e.call(LIST_REMOVE, &args![a[0], a[1]]);
            Ret::default()
        });
        e.register(NI_POINTER_LIST_REMOVE_HEAD, |e, a| {
            let next = e.mem.u32(a[0] + 4);
            if next == 0 {
                e.mem.set_u32(a[0], 0);
            } else {
                let (item, after) = (e.mem.u32(next), e.mem.u32(next + 4));
                e.mem.set_u32(a[0], item);
                e.mem.set_u32(a[0] + 4, after);
            }
            Ret::default()
        });
        stubs(
            e,
            &[NI_POINTER_RELEASE, GARBAGE_COLLECTOR_ADD, SET_CURRENT_SPELL],
        );
    }

    #[test]
    fn adding_a_temp_effect_picks_the_list_by_its_kind() {
        let mut e = fixture();
        let lists: Ptr<ProcessLists> = e.new_object();
        let (magic, other) = (object(&mut e), object(&mut e));
        on_slot(&mut e, 0x9c, move |o| if o == magic { 5 } else { 1 });
        e.register(NI_POINTER_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        stubs(&mut e, &[NI_POINTER_RELEASE]);
        e.register_double(NI_POINTER_LIST_PUSH, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            Ret::default()
        });
        e.call(0x0097_3fd0, &args![lists, magic]);
        e.call(0x0097_3fd0, &args![lists, other]);
        e.call(0x0097_3fd0, &args![lists, 0u32]);
        assert_eq!(e.mem.u32(lists.addr() + MAGIC_EFFECT_LIST), magic);
        assert_eq!(e.mem.u32(lists.addr() + GLOBAL_TEMP_EFFECT_LIST), other);
    }

    #[test]
    fn effects_of_a_reference_leave_both_lists() {
        let mut e = fixture();
        effect_list_doubles(&mut e);
        let lists: Ptr<ProcessLists> = e.new_object();
        let reference = 0x7030u32;
        let mut effects = Vec::new();
        for owner in [reference, 0x7031, reference, reference] {
            let effect = object(&mut e);
            e.mem.set_u32(effect + 0x10, owner);
            effects.push(effect);
        }
        e.register(0x0084_e3a0, |e, a| Ret {
            eax: e.mem.u32(a[0] + 0x10),
            ..Ret::default()
        });
        embed_list(
            &mut e,
            lists.addr() + GLOBAL_TEMP_EFFECT_LIST,
            &effects[..3],
        );
        embed_list(&mut e, lists.addr() + MAGIC_EFFECT_LIST, &effects[3..]);
        e.call_log = Some(vec![]);
        e.call(0x0097_40a0, &args![lists, reference]);
        assert_eq!(
            list_items(&e, lists.addr() + GLOBAL_TEMP_EFFECT_LIST),
            vec![effects[1]]
        );
        assert!(list_items(&e, lists.addr() + MAGIC_EFFECT_LIST).is_empty());
        assert_eq!(
            calls(&e, GARBAGE_COLLECTOR_ADD),
            vec![vec![effects[0]], vec![effects[2]], vec![effects[3]]]
        );
        assert_eq!(calls(&e, SET_CURRENT_SPELL).len(), 3);
        assert_eq!(calls(&e, SET_CURRENT_SPELL)[0], vec![effects[0], 0]);
    }

    #[test]
    fn caster_effects_of_an_owner_leave_the_global_list() {
        let mut e = fixture();
        effect_list_doubles(&mut e);
        let lists: Ptr<ProcessLists> = e.new_object();
        let owner = 0x7040u32;
        let (casting, plain, other_kind) = (object(&mut e), object(&mut e), object(&mut e));
        on_slot(&mut e, 0x9c, move |o| if o == other_kind { 1 } else { 2 });
        // The cast succeeds for the first effect only; the owner is `owner`.
        e.register_double(0x00ec_43fb, move |_, w| Ret {
            eax: if w[0] == casting { w[0] } else { 0 },
            ..Ret::default()
        });
        stub(&mut e, 0x0070_0300, owner);
        let helper = 0x9999u32;
        stub(&mut e, 0x0049_0e40, helper);
        stub(&mut e, 0x0045_0f90, 0);
        embed_list(
            &mut e,
            lists.addr() + GLOBAL_TEMP_EFFECT_LIST,
            &[plain, casting, other_kind],
        );
        e.call_log = Some(vec![]);
        e.call(0x0097_4290, &args![lists, owner]);
        assert_eq!(
            list_items(&e, lists.addr() + GLOBAL_TEMP_EFFECT_LIST),
            vec![plain, other_kind]
        );
        assert_eq!(calls(&e, 0x0045_0f90), vec![vec![helper, 1]]);
        assert_eq!(calls(&e, SET_CURRENT_SPELL), vec![vec![casting, 0]]);
        assert_eq!(calls(&e, GARBAGE_COLLECTOR_ADD), vec![vec![casting]]);
        // A null owner does nothing.
        e.call_log = Some(vec![]);
        e.call(0x0097_4290, &args![lists, 0u32]);
        assert!(calls(&e, GARBAGE_COLLECTOR_ADD).is_empty());
    }

    #[test]
    fn expired_effects_are_released_and_removed_by_the_update() {
        let mut e = fixture();
        effect_list_doubles(&mut e);
        let (actor, thing) = (object(&mut e), object(&mut e));
        let lists = lists_with(&mut e, 0, &[actor, thing]);
        on_slot(&mut e, SLOT_IS_ACTOR, move |o| (o == actor) as u32);
        let process = object(&mut e);
        stub(&mut e, ACTOR_PROCESS, process);
        stub(&mut e, slot(0x57c), 1);
        stub(&mut e, slot(0x584), 0);
        let effects: Vec<u32> = (0..4).map(|_| object(&mut e)).collect();
        let (alive_first, expired_first, expired_second, alive_second) =
            (effects[0], effects[1], effects[2], effects[3]);
        on_slot(&mut e, 0x94, move |o| {
            (o == alive_first || o == alive_second) as u32
        });
        stub(&mut e, slot(0x90), 0);
        embed_list(
            &mut e,
            lists.addr() + GLOBAL_TEMP_EFFECT_LIST,
            &[alive_first, expired_first],
        );
        embed_list(
            &mut e,
            lists.addr() + MAGIC_EFFECT_LIST,
            &[expired_second, alive_second],
        );
        e.call_log = Some(vec![]);
        e.call(0x0097_4420, &args![lists, 0.5f32]);
        assert_eq!(calls(&e, slot(0x584)), vec![vec![process, actor]]);
        assert_eq!(
            list_items(&e, lists.addr() + GLOBAL_TEMP_EFFECT_LIST),
            vec![alive_first]
        );
        assert_eq!(
            list_items(&e, lists.addr() + MAGIC_EFFECT_LIST),
            vec![alive_second]
        );
        // Every effect is updated with the time; only the first list's
        // expired effect is released through slot 0x90.
        assert_eq!(calls(&e, slot(0x94)).len(), 4);
        assert_eq!(
            calls(&e, slot(0x94))[0],
            vec![alive_first, 0.5f32.to_bits()]
        );
        assert_eq!(calls(&e, slot(0x90)), vec![vec![expired_first]]);
        assert_eq!(
            calls(&e, GARBAGE_COLLECTOR_ADD),
            vec![vec![expired_first], vec![expired_second]]
        );
    }

    /// Doubles for the record array of `fn_009746c0`: 12-byte records behind a
    /// pointer at +4 of the array object, the count at +0; the task manager
    /// runs the task directly.
    fn record_array_doubles(e: &mut Engine) {
        e.register(0x0097_8c50, |e, a| {
            e.mem.set_u32(a[0], 0);
            let storage = e.mem.alloc(0x100);
            e.mem.set_u32(a[0] + 4, storage);
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        e.register(0x0097_8bc0, |e, a| {
            let count = e.mem.u32(a[0]);
            e.mem.set_u32(a[0], count + 1);
            Ret {
                eax: count,
                ..Ret::default()
            }
        });
        e.register(0x006a_1440, |e, a| Ret {
            eax: e.mem.u32(a[0] + 4) + 12 * a[1],
            ..Ret::default()
        });
        e.register(0x006a_7af0, |e, a| Ret {
            eax: e.mem.u32(a[0] + 4) + 12 * a[1],
            ..Ret::default()
        });
        e.register(0x0044_ddc0, |e, a| Ret {
            eax: e.mem.u32(a[0]),
            ..Ret::default()
        });
        e.register(0x0097_8cf0, |_, _| Ret::default());
        e.register(0x0097_8990, |e, a| {
            let mut count = 0;
            let mut node = a[0];
            while node != 0 && !(e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0) {
                count += 1;
                node = e.mem.u32(node + 4);
            }
            Ret {
                eax: count,
                ..Ret::default()
            }
        });
        e.register(0x0087_cf10, |e, a| {
            for i in 0..3 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            Ret::default()
        });
        e.register(0x00c4_4c90, |e, a| {
            let (function, array, count) =
                (e.mem.u32(a[1]), e.mem.u32(a[1] + 4), e.mem.u32(a[1] + 8));
            for i in 0..count {
                e.call(function, &args![array, i]);
            }
            Ret::default()
        });
    }

    #[test]
    fn the_task_marks_a_record_expired_when_the_update_fails() {
        let mut e = fixture();
        let effect = object(&mut e);
        let record = e.mem.alloc(16);
        e.mem.set_u32(record, effect);
        e.mem.set_u32(record + 4, 0.25f32.to_bits());
        e.register(0x006a_7af0, |_, a| Ret {
            eax: a[0],
            ..Ret::default()
        });
        stub(&mut e, slot(0x94), 1);
        e.call_log = Some(vec![]);
        e.call(0x0097_4960, &args![record, 0u32]);
        assert_eq!(e.mem.u8(record + 9), 0);
        assert_eq!(calls(&e, slot(0x94)), vec![vec![effect, 0.25f32.to_bits()]]);
        stub(&mut e, slot(0x94), 0);
        e.call(0x0097_4960, &args![record, 0u32]);
        assert_eq!(e.mem.u8(record + 9), 1);
    }

    #[test]
    fn the_parallel_update_runs_the_tasks_and_removes_the_expired_effects() {
        let mut e = fixture();
        effect_list_doubles(&mut e);
        record_array_doubles(&mut e);
        let lists: Ptr<ProcessLists> = e.new_object();
        let effects: Vec<u32> = (0..3).map(|_| object(&mut e)).collect();
        let (alive, expired_first, expired_second) = (effects[0], effects[1], effects[2]);
        on_slot(&mut e, 0x94, move |o| (o == alive) as u32);
        embed_list(
            &mut e,
            lists.addr() + GLOBAL_TEMP_EFFECT_LIST,
            &[alive, expired_first],
        );
        embed_list(&mut e, lists.addr() + MAGIC_EFFECT_LIST, &[expired_second]);
        // The setting byte (value 0x100 after the setting) selects the parallel path.
        e.set_global(0x011f_137cu32, 1u8);
        e.set_global(TASK_MANAGER, 0x4d4d_0001u32);
        e.call_log = Some(vec![]);
        e.call(0x0097_46c0, &args![lists, 0.5f32]);
        assert_eq!(
            list_items(&e, lists.addr() + GLOBAL_TEMP_EFFECT_LIST),
            vec![alive]
        );
        assert!(list_items(&e, lists.addr() + MAGIC_EFFECT_LIST).is_empty());
        assert_eq!(
            calls(&e, GARBAGE_COLLECTOR_ADD),
            vec![vec![expired_first], vec![expired_second]]
        );
        assert_eq!(calls(&e, 0x0087_cf10)[0][1], 0x0097_4960);
        assert_eq!(calls(&e, 0x00c4_4c90)[0][0], 0x4d4d_0001);
        // The three records got the time.
        assert_eq!(calls(&e, slot(0x94)).len(), 3);
        assert_eq!(calls(&e, slot(0x94))[0][1], 0.5f32.to_bits());
    }

    #[test]
    fn the_parallel_update_does_nothing_without_effects_and_falls_back_without_the_setting() {
        let mut e = fixture();
        effect_list_doubles(&mut e);
        record_array_doubles(&mut e);
        let lists: Ptr<ProcessLists> = e.new_object();
        e.set_global(0x011f_137cu32, 1u8);
        e.call_log = Some(vec![]);
        e.call(0x0097_46c0, &args![lists, 0.5f32]);
        assert!(calls(&e, 0x0097_8c50).is_empty());
        // Setting off: the plain update runs (here over an empty array).
        e.set_global(0x011f_137cu32, 0u8);
        let effect = object(&mut e);
        on_slot(&mut e, 0x94, |_| 1);
        embed_list(&mut e, lists.addr() + GLOBAL_TEMP_EFFECT_LIST, &[effect]);
        e.call(0x0097_46c0, &args![lists, 0.5f32]);
        assert_eq!(calls(&e, slot(0x94)), vec![vec![effect, 0.5f32.to_bits()]]);
    }
    // <<more tests>>
}
