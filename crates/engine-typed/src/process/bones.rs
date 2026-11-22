//! `ProcessLists::ParallelUpdateAnimationScenegraph` (`0096c970`): the bones of every actor whose
//! animation the AI threads postponed (the actors of process levels 0 and 1), moved on main after
//! the frame is shown (`Main::UpdateNonRenderSafeAITasks`). With more than one thread the AI threads
//! update the animations while main draws the same nodes, so `Animation::UpdateSceneGraphNoController`
//! (`00493bd0`) only marks the scene graph pending there, and this pass does the world update:
//! per actor its animation root's downward pass with the root's and the accumulation root's
//! controllers detached, the pending mark cleared, then the lighting queue for an actor that asks.
//!
//! **`NV_PAR_BONES`** (`off`, `on`, `check`; off by default until a sitting shows the gain): the
//! downward passes on the parallel render's helper engines (`engine_core::real_threads::fork_join_first`,
//! the caller's x87 control word on each), everything else on main in the list's order, so the end
//! state is the serial pass's. A host change, not exe behaviour. The game's own parallel path (the
//! setting `bTaskletActorSceneGraphUpdates`, `011f1260`) is left as it is when that setting is on.
//!
//! **What a helper runs** ([`classify`]): an actor's tree walked with the typed passes only: the
//! `NiNode`, fade node and plain-object downward passes, `NiAVObject::UpdateWorldData`, the geometry
//! and skinned geometry bounds, the bound merges, and a bone's blend collision object up to its node's
//! refresh. They read the node, its parent's world transform, the skin's bones (the actor's own clone)
//! and constants, and write the node's world transform, bound and flags.
//!
//! **What a helper logs instead of running** (its actor's log, in walk order): a blend collision
//! object's Havok part (`bhkCollisionObject::NitohkTransform` after the node's refresh, or
//! `hktoNiTransform` with no change of motion type and the body keyframed to the node): it writes
//! the body, under the Havok lock, and the node never reads it back; and a subtree that must run on
//! main: a head (`BSFaceGenNiNode::UpdateDownwardPass`, `00663a70`, whose face morphing reads the
//! actor's process, the player pointer, the close-to-player list, menu mode, settings, the base
//! NPC's face data and the world camera's position, none of which a bone pass writes) and an object
//! without a world bound yet (its allocation then happens on main, in the serial order).
//!
//! **Barriers.** An actor with anything else in its tree (controllers or property controllers below
//! the two detached roots, particle systems, lights and other classes, a collision object that reads
//! its body into the node or changes its motion type, a parent chain through another listed actor)
//! runs whole on main as the translation runs it, and the helpers walk only the actors listed before
//! it: code not traced for other cores sees exactly the state the serial pass shows it.
//!
//! **The replay**, per actor in list order: its log (a subtree's downward pass with the actor's
//! update data, a Havok part as the serial pass makes it), the merged bounds of the logged
//! subtrees' ancestors done again from their children's final bounds (the children loop's bound
//! part, which reads only those), the controllers put back, the pending mark cleared, the lighting.
//!
//! **The check** (`NV_PAR_BONES=check`): the helpers walk every stretch with the fields they may
//! write saved first; their results and their Havok parts are kept and the fields put back; then the
//! serial pass runs (it is what the game goes on with) and its results and Havok parts are compared
//! with the helpers'. The bounds of a node above a logged subtree are compared by doing the bound
//! part again on the serial end state.

use crate::culling::layout::NiNode;
use crate::{collision, culling, dispatch, scene, slot_target};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering::Relaxed};
use std::sync::Mutex;

// ----- the exe's functions and data ------------------------------------------------------------

/// `ProcessArray`'s level head and tail (`005de0f0`, `005be5c0`) and its object at an index
/// (`00968670`), all `thiscall` on the lists' +4.
const LEVEL_HEAD: u32 = 0x005d_e0f0;
const LEVEL_TAIL: u32 = 0x005b_e5c0;
const ARRAY_OBJECT: u32 = 0x0096_8670;
/// The bool setting's value address (`00408d60`) and the game's own parallel switch
/// (`bTaskletActorSceneGraphUpdates:General`).
const SETTING_VALUE: u32 = 0x0040_8d60;
const TASKLET_SETTING: u32 = 0x011f_1260;
/// The scrap array of animations: constructor, destructor, push (`BSSimpleArray::AddUninitialized`).
const SCRAP_CONSTRUCT: u32 = 0x0097_89e0;
const SCRAP_DESTRUCT: u32 = 0x0097_8a80;
const SCRAP_PUSH: u32 = 0x007c_b2e0;
/// The task's build (`0087cf10`), the parallel task manager (global `01202df4`), its run of a split
/// task (`00c44c90`) and the task body (`0096cb20`).
const TASK_BUILD: u32 = 0x0087_cf10;
const TASK_MANAGER: u32 = 0x0120_2df4;
const TASK_RUN: u32 = 0x00c4_4c90;
const TASK_BODY: u32 = 0x0096_cb20;
/// Actor slots: `IsActor` (+0x100), the animation (+0x1e4), the 3D node (+0x1d0).
const SLOT_IS_ACTOR: u32 = 0x100;
const SLOT_ANIMATION: u32 = 0x1e4;
const SLOT_GET_NODE: u32 = 0x1d0;
/// `Actor::bUpdateLighting` (+0x14e).
const UPDATE_LIGHTING: u32 = 0x14e;
/// `Animation::UpdateQueuedScenegraph` (`00493930`), the flag set (`00493880(this, value, mask)`)
/// and the pending mask (2).
const QUEUED_SCENEGRAPH: u32 = 0x0049_3930;
const FLAG_SET: u32 = 0x0049_3880;
const FLAG_PENDING: u8 = 2;
/// `Animation::pAnimRoot` (an `NiPointer`), `pAccumRoot` and the time.
const ANIM_ROOT: u32 = 0x08;
const ACCUM_ROOT: u32 = 0x0c;
const ANIM_TIME: u32 = 0xd0;
/// The update data's constructor (`0043d410`, `thiscall(data, time, 1, 0)`) and its size.
const UPDATE_DATA_CONSTRUCT: u32 = 0x0043_d410;
const UPDATE_DATA_SIZE: u32 = 12;
/// `NiObjectNET`'s controllers (`0043b230`) and their set (`00a5c000`), `NiPointer`'s init from a
/// pointer (`00633c90`) and release (`0045cec0`).
const OBJECT_CONTROLLERS: u32 = 0x0043_b230;
const OBJECT_SET_CONTROLLERS: u32 = 0x00a5_c000;
const NI_POINTER_INIT: u32 = 0x0063_3c90;
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
/// The lighting: the shadow scene node (`00450b80(0)`), its `UpdateObjectLighting` (`00b5d9f0`) and
/// the actor's flag store (`00496960(actor, 0)`).
const SHADOW_SCENE: u32 = 0x0045_0b80;
const OBJECT_LIGHTING: u32 = 0x00b5_d9f0;
const LIGHTING_DONE: u32 = 0x0049_6960;
/// The world camera the face morphing measures from (`011d59ec`).
const FACE_CAMERA: u32 = 0x011d_59ec;
/// The thread block's update state word the collision objects read (`TLS + 0x2c0`).
const TLS_UPDATE_STATE: u32 = 0x2c0;

/// Downward pass slots (+0xa4) the helpers run: `NiNode`'s, a plain object's, the fade node's; and
/// the head's, which they log.
const SLOT_DOWNWARD: u32 = 0xa4;
const NODE_DOWNWARD: u32 = 0x00a5_dd70;
const OBJECT_DOWNWARD: u32 = 0x00a5_9f90;
const FADE_DOWNWARD: u32 = 0x00b4_ded0;
const HEAD_DOWNWARD: u32 = 0x0066_3a70;
/// `UpdateWorldData` (+0xb8, `NiAVObject`'s `00a68c60`) and `UpdateWorldBound` (+0xbc, a geometry's
/// `00a80710`).
const SLOT_WORLD_DATA: u32 = 0xb8;
const WORLD_DATA: u32 = 0x00a6_8c60;
const SLOT_WORLD_BOUND: u32 = 0xbc;
const GEOMETRY_BOUND: u32 = 0x00a8_0710;
/// The collision object's update (+0x90, the blend object's `00c817d0`), its `hktoNiTransform`
/// (+0xa4, `00c81e30`) and the motion slot (+0xa8, `NitohkTransform` `00c65b00`).
const SLOT_COLLISION_UPDATE: u32 = 0x90;
const BLEND_UPDATE: u32 = 0x00c8_17d0;
const SLOT_HK_TO_NI: u32 = 0xa4;
const BLEND_HK_TO_NI: u32 = 0x00c8_1e30;
const SLOT_MOTION: u32 = 0xa8;
const NI_TO_HK: u32 = 0x00c6_5b00;
/// The flags bit of an object whose properties have controllers.
const FLAG_PROPERTY_CONTROLLERS: u32 = 0x0004_0000;
/// The children array of a node (+0xa0) and its count (+0xa6).
const CHILDREN: u32 = 0xa0;
const CHILD_COUNT: u32 = 0xa6;

// ----- the switch ------------------------------------------------------------------------------

/// 0 unread, 1 off, 2 on, 3 check.
static MODE: AtomicU8 = AtomicU8::new(0);
/// The helpers a stretch asks for.
static HELPERS: AtomicUsize = AtomicUsize::new(3);

fn mode() -> u8 {
    match MODE.load(Relaxed) {
        0 => read_switch(),
        m => m,
    }
}

#[cold]
fn read_switch() -> u8 {
    let value = std::env::var("NV_PAR_BONES").unwrap_or_default();
    let m = match value.trim() {
        "on" | "1" => 2,
        "check" => 3,
        _ => 1,
    };
    if let Some(n) = std::env::var("NV_PAR_BONES_THREADS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
    {
        HELPERS.store(n.clamp(1, 15), Relaxed);
    }
    if m != 1 {
        println!(
            "NV_PAR_BONES={}: actors' bones moved on {} helpers{}",
            value.trim(),
            HELPERS.load(Relaxed),
            if m == 3 {
                ", each stretch checked against the serial pass"
            } else {
                ""
            }
        );
    }
    MODE.store(m, Relaxed);
    m
}

/// For the tests: the mode (`off`, `on`, `check`) and the helpers, as `NV_PAR_BONES` and
/// `NV_PAR_BONES_THREADS` would set them (the environment is then not read).
#[doc(hidden)]
pub fn set_for_tests(mode: &str, helpers: usize) {
    HELPERS.store(helpers.clamp(1, 15), Relaxed);
    MODE.store(
        match mode {
            "on" => 2,
            "check" => 3,
            _ => 1,
        },
        Relaxed,
    );
}

// ----- counts ----------------------------------------------------------------------------------

/// What the pass did since the last [`take_counts`]: frames, actors walked on helpers, actors run
/// whole on main (barriers), subtrees logged, Havok parts logged, check comparisons and differences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub frames: u64,
    pub walked: u64,
    pub barriers: u64,
    pub subtrees: u64,
    pub havok: u64,
    pub checked_actors: u64,
    pub differences: u64,
    pub mismatches: u64,
}

static FRAMES: AtomicU64 = AtomicU64::new(0);
static WALKED: AtomicU64 = AtomicU64::new(0);
static BARRIERS: AtomicU64 = AtomicU64::new(0);
static SUBTREES: AtomicU64 = AtomicU64::new(0);
static HAVOK: AtomicU64 = AtomicU64::new(0);
static CHECKED: AtomicU64 = AtomicU64::new(0);
static DIFFERENCES: AtomicU64 = AtomicU64::new(0);
static MISMATCHES: AtomicU64 = AtomicU64::new(0);
/// Barrier reasons, by [`Barrier`] as a number.
static REASONS: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];
/// Differences printed so far (the first 20 are).
static PRINTED: AtomicU32 = AtomicU32::new(0);
/// Time, in nanoseconds: the whole pass when it ran in parallel, the helpers' walks (main waiting
/// or walking too), main's replays, the actors run whole on main.
static PASS_NS: AtomicU64 = AtomicU64::new(0);
static WALK_NS: AtomicU64 = AtomicU64::new(0);
static REPLAY_NS: AtomicU64 = AtomicU64::new(0);
static SERIAL_NS: AtomicU64 = AtomicU64::new(0);
/// Frames between two summary lines.
const SUMMARY_EVERY: u64 = 1000;
static SUMMARY_FRAMES: AtomicU64 = AtomicU64::new(0);

fn nanos(since: std::time::Instant) -> u64 {
    u64::try_from(since.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Every [`SUMMARY_EVERY`] parallel frames: one line with the counts and the times since the last.
#[cold]
fn summary() {
    let ms = |a: &AtomicU64| a.swap(0, Relaxed) as f64 / 1e6 / SUMMARY_EVERY as f64;
    let (pass, walk, replay, serial) = (ms(&PASS_NS), ms(&WALK_NS), ms(&REPLAY_NS), ms(&SERIAL_NS));
    let (c, reasons) = take_counts();
    println!(
        "NV_PAR_BONES: {} frames: a frame {pass:.3} ms (walks {walk:.3}, replays {replay:.3}, whole on main \
         {serial:.3}); actors walked {}, whole on main {} {reasons:?}, subtrees on main {}, Havok parts {}; \
         checked {} actors, {} differences, {} mismatches",
        c.frames, c.walked, c.barriers, c.subtrees, c.havok, c.checked_actors, c.differences, c.mismatches
    );
    census_print();
}

// ----- the barrier census (`NV_PAR_BONES_CENSUS=1`) ----------------------------------------------

/// Every barrier object of every listed actor's tree, not only each actor's first: (what, the
/// object's vtable, the detail, whether it lies in a head's subtree) and how often. What: 0 a node's
/// controllers (detail: the first controller's vtable), 1 properties with controllers (detail: the
/// object's flags), 2 a class (detail: its downward pass), 3 a collision object (detail: its vtable).
/// A census key: (what, the object's vtable, the detail, in a head).
type CensusKey = (u32, u32, u32, bool);
static CENSUS: Mutex<Vec<(CensusKey, u64)>> = Mutex::new(Vec::new());
static CENSUS_ON: AtomicU8 = AtomicU8::new(0);

fn census_on() -> bool {
    match CENSUS_ON.load(Relaxed) {
        0 => {
            let on = std::env::var("NV_PAR_BONES_CENSUS").is_ok_and(|v| v.trim() == "1");
            CENSUS_ON.store(if on { 2 } else { 1 }, Relaxed);
            on
        }
        m => m == 2,
    }
}

/// Notes a barrier object for the census.
fn census_note(e: &Engine, object: u32, barrier: Barrier, in_head: bool) {
    let vtable = e.mem.u32(object);
    let (what, detail) = match barrier {
        Barrier::Controllers if e.mem.u32(object + 0x30) & FLAG_PROPERTY_CONTROLLERS != 0 => {
            (1, e.mem.u32(object + 0x30))
        }
        Barrier::Controllers => (0, e.mem.u32(e.mem.u32(object + 0x0c))),
        Barrier::Collision => (3, e.mem.u32(e.mem.u32(object + 0x1c))),
        _ => (2, slot_target(e, object, SLOT_DOWNWARD)),
    };
    let key = (what, vtable, detail, in_head);
    let mut c = CENSUS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entry) = c.iter_mut().find(|(k, _)| *k == key) {
        entry.1 += 1;
    } else {
        c.push((key, 1));
    }
}

#[cold]
fn census_print() {
    if !census_on() {
        return;
    }
    let mut c = std::mem::take(
        &mut *CENSUS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    c.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for ((what, vtable, detail, in_head), n) in c.iter().take(16) {
        let what = ["controllers", "property controllers", "class", "collision"][*what as usize];
        println!(
            "NV_PAR_BONES census: {n} x {what}: object vtable {vtable:08x}, detail {detail:08x}{}",
            if *in_head { ", in a head" } else { "" }
        );
    }
}

/// The counts since the last call, and the barrier reasons (by [`Barrier::name`]).
pub fn take_counts() -> (Counts, Vec<(&'static str, u64)>) {
    let c = Counts {
        frames: FRAMES.swap(0, Relaxed),
        walked: WALKED.swap(0, Relaxed),
        barriers: BARRIERS.swap(0, Relaxed),
        subtrees: SUBTREES.swap(0, Relaxed),
        havok: HAVOK.swap(0, Relaxed),
        checked_actors: CHECKED.swap(0, Relaxed),
        differences: DIFFERENCES.swap(0, Relaxed),
        mismatches: MISMATCHES.swap(0, Relaxed),
    };
    let reasons = Barrier::ALL
        .iter()
        .map(|b| (b.name(), REASONS[*b as usize].swap(0, Relaxed)))
        .filter(|(_, n)| *n != 0)
        .collect();
    (c, reasons)
}

// ----- the translation's pass, typed -----------------------------------------------------------

/// The process array's level head and tail and its object at an index.
fn level_head(e: &mut Engine, array: u32, level: u32) -> u32 {
    e.call(LEVEL_HEAD, &args![array, level]).u32()
}
fn level_tail(e: &mut Engine, array: u32, level: u32) -> u32 {
    e.call(LEVEL_TAIL, &args![array, level]).u32()
}
fn array_object(e: &mut Engine, array: u32, index: u32) -> u32 {
    e.call(ARRAY_OBJECT, &args![array, index]).u32()
}

/// The lighting both loops end with: the shadow scene node queues the actor's 3D node for its
/// lighting update, then the actor's flag is cleared.
fn update_object_lighting(e: &mut Engine, actor: u32) {
    let node = e.vcall(actor, SLOT_GET_NODE, &args![]).u32();
    let shadow_scene = e.call(SHADOW_SCENE, &args![0u32]).u32();
    e.call(OBJECT_LIGHTING, &args![shadow_scene, node, 0u32]);
    e.call(LIGHTING_DONE, &args![actor, 0u32]);
}

/// The end of one actor of the serial loop: the lighting when the actor asks for it and the update
/// ran.
fn lighting_if_asked(e: &mut Engine, actor: u32, updated: bool) {
    if e.mem.u8(actor + UPDATE_LIGHTING) != 0 && updated {
        update_object_lighting(e, actor);
    }
}

// Typed from 0096c970
/// `ProcessLists::ParallelUpdateAnimationScenegraph` (`thiscall(lists)`): for every actor from level
/// 0's head to level 1's tail with an animation (slot +0x1e4): with the game's parallel switch the
/// animation goes into a scrap array (and counts as updated), else `UpdateQueuedScenegraph` runs on
/// it; an actor asking for its lighting whose update ran gets it. The scrap array, when it holds
/// animations, goes to the parallel task manager (`0096cb20` on each). With `NV_PAR_BONES` and the
/// switch off, the per-actor updates go over the helpers ([`parallel`]).
pub fn parallel_update_animation_scenegraph(e: &mut Engine, this: Ptr) {
    let array = this.addr().wrapping_add(4);
    let first = level_head(e, array, 0);
    let last = level_tail(e, array, 1);
    let count = last.wrapping_sub(first) as i32;
    let setting = e.call(SETTING_VALUE, &args![TASKLET_SETTING]).u32();
    let tasklets = e.mem.u8(setting) != 0;
    if count == 0 {
        return;
    }
    e.with_stack(0x30, |e, frame| {
        let scrap = frame.addr();
        let slot = scrap + 0x14;
        let task = scrap + 0x20;
        e.call(SCRAP_CONSTRUCT, &args![scrap]);
        if tasklets || mode() == 1 || !parallel(e, array, first, last) {
            serial_loop(e, array, first, last, tasklets, scrap, slot);
        }
        if typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, scrap)) != 0 {
            let queued = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, scrap));
            e.call(TASK_BUILD, &args![task, TASK_BODY, scrap, queued]);
            let manager: u32 = e.global(TASK_MANAGER);
            e.call(TASK_RUN, &args![manager, task]);
        }
        e.call(SCRAP_DESTRUCT, &args![scrap]);
    });
}

/// The translation's loop over `first..last`.
fn serial_loop(
    e: &mut Engine,
    array: u32,
    first: u32,
    last: u32,
    tasklets: bool,
    scrap: u32,
    slot: u32,
) {
    let mut index = first;
    while index < last {
        let actor = array_object(e, array, index);
        if actor != 0 && e.vcall(actor, SLOT_IS_ACTOR, &[]).bool() {
            let animation = e.vcall(actor, SLOT_ANIMATION, &[]).u32();
            if animation != 0 {
                let mut updated = true;
                if tasklets {
                    e.mem.set_u32(slot, animation);
                    e.call(SCRAP_PUSH, &args![scrap, slot]);
                } else {
                    updated = e.call(QUEUED_SCENEGRAPH, &args![animation]).bool();
                }
                lighting_if_asked(e, actor, updated);
            }
        }
        index = index.wrapping_add(1);
    }
}

// ----- the parallel pass -----------------------------------------------------------------------

/// Why an actor runs whole on main.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Barrier {
    /// A node below the detached roots with controllers, or properties with controllers.
    Controllers = 0,
    /// A class whose passes the helpers do not run (a particle system, a light, another class).
    Class = 1,
    /// A collision object that is not a blend object, or one whose update reads its body into the
    /// node or changes its motion type (a ragdoll, a dead or falling actor).
    Collision = 2,
    /// Its parent chain passes through another listed actor's tree.
    Attached = 3,
    /// A helper's typed pass is off or replaced (`NV_TYPED`, an extender's hook): the whole pass
    /// runs serially that frame.
    Untyped = 4,
    /// The world camera the face morphing measures from is inside an actor's tree: the whole pass
    /// runs serially that frame.
    Camera = 5,
}

impl Barrier {
    const ALL: [Barrier; 6] = [
        Barrier::Controllers,
        Barrier::Class,
        Barrier::Collision,
        Barrier::Attached,
        Barrier::Untyped,
        Barrier::Camera,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Barrier::Controllers => "controllers",
            Barrier::Class => "class",
            Barrier::Collision => "collision",
            Barrier::Attached => "attached",
            Barrier::Untyped => "untyped",
            Barrier::Camera => "camera",
        }
    }
}

/// What a helper does with one object of an actor's tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// Walks it (and its children, each classified in turn).
    Walk,
    /// Logs it for main: its whole subtree runs there in the actor's turn.
    Defer,
    /// The actor cannot be walked on a helper.
    Barrier(Barrier),
}

/// One entry of an actor's log, in walk order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A subtree for main: the object, its parent and the downward pass's flags word.
    Subtree {
        object: u32,
        parent: u32,
        flags: u32,
    },
    /// A blend collision object's `NitohkTransform` (then the node-moved bit cleared), with the
    /// node's world transform the helper computed (for the check).
    NiToHk { object: u32, world: [u32; 13] },
    /// A blend collision object's `hktoNiTransform` (keyframed, no change of motion type).
    HkToNi { object: u32, world: [u32; 13] },
    /// A helper met an object it should not have (the classification changed under it).
    Mismatch { object: u32 },
}

/// How many threads have a walk log or a check record open now: the one relaxed load
/// [`walking`] and [`recording`] make before any thread-local lookup, so with the switch off every
/// downward pass and blend collision update pays only that load.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// While this thread walks an actor on a helper's engine (or main's own share): its log.
    static LOG: RefCell<Option<Vec<Entry>>> = const { RefCell::new(None) };
    /// While main runs the serial pass in check mode: the Havok parts it makes, in order.
    static RECORD: RefCell<Option<Vec<Entry>>> = const { RefCell::new(None) };
}

/// Whether this thread walks an actor for the parallel pass now (the downward pass and the blend
/// collision object ask, so that they log instead of running what main must run).
#[inline(always)]
pub fn walking() -> bool {
    ACTIVE.load(Relaxed) != 0 && LOG.with(|l| l.borrow().is_some())
}

/// Whether main records the serial pass's Havok parts now (check mode).
#[inline(always)]
pub fn recording() -> bool {
    ACTIVE.load(Relaxed) != 0 && RECORD.with(|l| l.borrow().is_some())
}

fn push(entry: Entry) {
    LOG.with(|l| {
        if let Some(log) = l.borrow_mut().as_mut() {
            log.push(entry);
        }
    });
}

/// The node's world transform, as words (scale, rotation, translation order of the guest).
fn world_words(e: &Engine, node: u32) -> [u32; 13] {
    let at = node + 0x68;
    std::array::from_fn(|i| e.mem.u32(at + 4 * idx(i)))
}

/// Called by the blend collision object's update on a walking helper, after its node's refresh, in
/// place of `NitohkTransform` (`keyframed`: of `hktoNiTransform`): logged for main.
pub(crate) fn log_havok(e: &Engine, object: u32, keyframed: bool) {
    let node = e.mem.u32(object + 8);
    let world = if node != 0 {
        world_words(e, node)
    } else {
        [0; 13]
    };
    push(if keyframed {
        Entry::HkToNi { object, world }
    } else {
        Entry::NiToHk { object, world }
    });
}

/// Called by the blend collision object's update while main records (check mode), as it makes the
/// Havok part: the same entry as a helper logs, so the two lists compare.
pub(crate) fn record_havok(e: &Engine, object: u32, keyframed: bool) {
    let node = e.mem.u32(object + 8);
    let world = if node != 0 {
        world_words(e, node)
    } else {
        [0; 13]
    };
    RECORD.with(|l| {
        if let Some(log) = l.borrow_mut().as_mut() {
            log.push(if keyframed {
                Entry::HkToNi { object, world }
            } else {
                Entry::NiToHk { object, world }
            });
        }
    });
}

/// Called by the downward pass for each object on a walking helper: true when the object was logged
/// (its subtree runs on main) and must not be walked here.
pub(crate) fn logs_subtree(e: &mut Engine, object: u32, flags: u32) -> bool {
    match classify(e, object, &[]) {
        Class::Walk => false,
        Class::Defer => {
            let parent = e.mem.u32(object + 0x18);
            push(Entry::Subtree {
                object,
                parent,
                flags,
            });
            true
        }
        Class::Barrier(_) => {
            push(Entry::Mismatch { object });
            let parent = e.mem.u32(object + 0x18);
            push(Entry::Subtree {
                object,
                parent,
                flags,
            });
            true
        }
    }
}

/// Whether every typed pass a helper runs is direct on this engine (its family on and nothing
/// registered over it).
fn typed_passes(e: &Engine) -> bool {
    e.typed_direct(dispatch::FAMILY, NODE_DOWNWARD)
        && e.typed_direct(scene::FAMILY, OBJECT_DOWNWARD)
        && e.typed_direct(scene::FAMILY, FADE_DOWNWARD)
        && e.typed_direct(scene::FAMILY, 0x00a8_6530)
        && e.typed_direct(culling::FAMILY, WORLD_DATA)
        && e.typed_direct(culling::FAMILY, 0x00a6_8bf0)
        && e.typed_direct(culling::FAMILY, GEOMETRY_BOUND)
        && e.typed_direct(culling::FAMILY, 0x00a7_f3f0)
        && e.typed_direct(collision::FAMILY, BLEND_UPDATE)
        && e.typed_direct(collision::FAMILY, BLEND_HK_TO_NI)
}

/// How a blend collision object's update would go now (the branches of
/// `bhkBlendCollisionObject::UpdateWorldData`, `collision::blend::update_world_data`, read without
/// running them): true when a helper may run it (nothing, a refresh, or a refresh followed by a
/// Havok part it logs).
fn collision_ok(e: &mut Engine, object: u32) -> bool {
    if slot_target(e, object, SLOT_COLLISION_UPDATE) != BLEND_UPDATE {
        return false;
    }
    let state = {
        let tls = e.tls();
        e.mem.u32(tls + TLS_UPDATE_STATE)
    };
    if state == 3 || e.mem.u8(object + 0x14) != 0 {
        return true;
    }
    let body = e.mem.u32(object + 0x10);
    if body != 0 && (e.mem.u32(body + 0x10) >> 6) & 1 != 0 {
        return true;
    }
    let flags = e.mem.u8(object + 0xc);
    if (flags >> 6) & 1 != 0 {
        // The node-moved bit: `NitohkTransform` (state other than 2) or a refresh (state 2).
        return true;
    }
    if flags & 1 == 0 {
        return true;
    }
    // Keyframed: `hktoNiTransform` runs when there is a body, the amount is below 1 or the motion
    // mode is 1; otherwise nothing.
    let amount = e.mem.f32(object + 0x18);
    let current = e.mem.u32(object + 0x20);
    if !(body != 0 || 1.0 > amount || current == 1) {
        return true;
    }
    // Only the fully keyframed case outside update state 2: motion target 4 already set, blend mode
    // 2, the motion slot `NitohkTransform` (a refresh, then the body written).
    1.0 == amount
        && state != 2
        && current == 4
        && slot_target(e, object, SLOT_HK_TO_NI) == BLEND_HK_TO_NI
        && slot_target(e, object, SLOT_MOTION) == NI_TO_HK
}

/// What a helper does with `object` (one object; its children are classified as the walk meets
/// them). `detached`: the actor's two roots, whose controllers are detached while it walks (at the
/// check before the walk their controllers are still on).
fn classify(e: &mut Engine, object: u32, detached: &[u32]) -> Class {
    let downward = slot_target(e, object, SLOT_DOWNWARD);
    let head = downward == HEAD_DOWNWARD;
    if !head && !matches!(downward, NODE_DOWNWARD | OBJECT_DOWNWARD | FADE_DOWNWARD) {
        return Class::Barrier(Barrier::Class);
    }
    if slot_target(e, object, SLOT_WORLD_DATA) != WORLD_DATA {
        return Class::Barrier(Barrier::Class);
    }
    if downward == OBJECT_DOWNWARD && slot_target(e, object, SLOT_WORLD_BOUND) != GEOMETRY_BOUND {
        return Class::Barrier(Barrier::Class);
    }
    let flags = e.mem.u32(object + 0x30);
    if flags & FLAG_PROPERTY_CONTROLLERS != 0 {
        return Class::Barrier(Barrier::Controllers);
    }
    if !detached.contains(&object) && !controllers_idle(e, e.mem.u32(object + 0x0c)) {
        return Class::Barrier(Barrier::Controllers);
    }
    let collision = e.mem.u32(object + 0x1c);
    if collision != 0 && !collision_ok(e, collision) {
        return Class::Barrier(Barrier::Collision);
    }
    if head || e.mem.u32(object + 0x20) == 0 {
        // A head's morphing, or an object that gets its world bound allocated now: on main.
        return Class::Defer;
    }
    Class::Walk
}

/// `bhkBlendController::Update` and `bhkForceController::Update`, the controllers a helper may run
/// while idle.
const BLEND_CONTROLLER_UPDATE: u32 = 0x00c9_b090;
const FORCE_CONTROLLER_UPDATE: u32 = 0x00cb_9770;

/// Whether every controller of the list from `first` (along +0x30) is one a helper may run now: a
/// `bhkBlendController` that its update leaves at once (active bit +8 bit 3 clear, no target at
/// +0x2c, or +0x48 clear: nothing written), or a `bhkForceController` that is not active (bit 3
/// clear, or no target: its update only stores a word of its own, +0x50). An empty list is idle.
fn controllers_idle(e: &Engine, first: u32) -> bool {
    let mut controller = first;
    let mut steps = 0;
    while controller != 0 {
        steps += 1;
        if steps > 64 {
            return false;
        }
        let active = (e.mem.u16(controller + 8) >> 3) & 1 != 0;
        let target = e.mem.u32(controller + 0x2c);
        let idle = match slot_target(e, controller, 0x94) {
            BLEND_CONTROLLER_UPDATE => !active || target == 0 || e.mem.u32(controller + 0x48) == 0,
            FORCE_CONTROLLER_UPDATE => !active || target == 0,
            _ => false,
        };
        if !idle {
            return false;
        }
        controller = e.mem.u32(controller + 0x30);
    }
    true
}

/// The children of a node (empty slots left out).
fn children(e: &Engine, node: u32) -> Vec<u32> {
    let count = u32::from(e.mem.u16(node + CHILD_COUNT));
    let base = e.mem.u32(node + CHILDREN);
    (0..count)
        .map(|i| e.mem.u32(base.wrapping_add(4 * i)))
        .filter(|&c| c != 0)
        .collect()
}

/// Whether `object` is a node the downward pass walks the children of.
fn has_children(e: &Engine, object: u32) -> bool {
    matches!(
        slot_target(e, object, SLOT_DOWNWARD),
        NODE_DOWNWARD | FADE_DOWNWARD | HEAD_DOWNWARD
    )
}

/// The check before a stretch: every object of the actor's tree, a subtree that will be logged
/// included (it runs on main, but in the helpers' order, so it may hold nothing that is not
/// traced). `None`: the actor may be walked on a helper.
fn barrier_in(e: &mut Engine, root: u32, detached: &[u32]) -> Option<Barrier> {
    let census = census_on();
    let mut first = None;
    let mut stack = vec![(root, false)];
    while let Some((object, in_head)) = stack.pop() {
        if let Class::Barrier(b) = classify(e, object, detached) {
            if !census {
                return Some(b);
            }
            census_note(e, object, b, in_head);
            first = first.or(Some(b));
        }
        if has_children(e, object) {
            let head = in_head || slot_target(e, object, SLOT_DOWNWARD) == HEAD_DOWNWARD;
            stack.extend(children(e, object).into_iter().map(|c| (c, head)));
        }
    }
    first
}

/// Whether `node` is `root` or below it.
fn below(e: &Engine, mut node: u32, root: u32) -> bool {
    let mut steps = 0;
    while node != 0 && steps < 4096 {
        if node == root {
            return true;
        }
        node = e.mem.u32(node + 0x18);
        steps += 1;
    }
    false
}

/// One actor of the list with a pending animation.
#[derive(Clone, Copy, Debug)]
struct Item {
    actor: u32,
    animation: u32,
    /// The animation root and the accumulation root (0 when either is missing).
    root: u32,
    accum: u32,
    barrier: Option<Barrier>,
}

/// The parallel pass over `first..last` (the switch on, the game's own switch off). False (nothing
/// done) when it cannot run this frame: no helpers, a typed pass off, the camera the face morphing
/// measures from inside an actor's tree, a diagnostic that keeps the caller alone. Then the
/// translation's loop runs.
fn parallel(e: &mut Engine, array: u32, first: u32, last: u32) -> bool {
    if !typed_passes(e) {
        BARRIERS.fetch_add(1, Relaxed);
        REASONS[Barrier::Untyped as usize].fetch_add(1, Relaxed);
        return false;
    }
    // The list, in order, with each actor's animation asked as the translation asks it.
    let started = std::time::Instant::now();
    let mut items = Vec::new();
    let mut entries = Vec::new();
    let mut index = first;
    while index < last {
        let actor = array_object(e, array, index);
        if actor != 0 && e.vcall(actor, SLOT_IS_ACTOR, &[]).bool() {
            let animation = e.vcall(actor, SLOT_ANIMATION, &[]).u32();
            if animation != 0 {
                entries.push((actor, animation));
            }
        }
        index = index.wrapping_add(1);
    }
    for &(actor, animation) in &entries {
        let pending = e.mem.u8(animation) & FLAG_PENDING != 0;
        let root = typed_call!(
            leaf::FAMILY,
            0x0055_9450,
            leaf::word_at_0(e, animation + ANIM_ROOT)
        );
        let accum = e.mem.u32(animation + ACCUM_ROOT);
        // Nothing pending (the update answers false) or no roots (it only clears the mark): the
        // translation's call at the actor's place, reading nothing of the others.
        let walks = pending && root != 0 && accum != 0;
        let barrier = if walks {
            barrier_in(e, root, &[root, accum])
        } else {
            None
        };
        items.push(Item {
            actor,
            animation,
            root: if walks { root } else { 0 },
            accum,
            barrier,
        });
    }
    // An actor hung below another listed actor's tree runs on main; and the world camera inside an
    // actor's tree (the face morphing reads its position) keeps the whole pass serial.
    let roots: Vec<u32> = items
        .iter()
        .filter(|i| i.root != 0)
        .map(|i| i.root)
        .collect();
    let camera = e.global::<u32>(FACE_CAMERA);
    if camera != 0 && roots.iter().any(|&r| below(e, camera, r)) {
        REASONS[Barrier::Camera as usize].fetch_add(1, Relaxed);
        return false;
    }
    for item in items.iter_mut() {
        if item.root != 0 && item.barrier.is_none() {
            let parent = e.mem.u32(item.root + 0x18);
            if roots.iter().any(|&r| r != item.root && below(e, parent, r)) {
                item.barrier = Some(Barrier::Attached);
            }
        }
    }
    FRAMES.fetch_add(1, Relaxed);
    let check = mode() == 3;
    // Stretches: the walkable actors up to the next barrier, then the barrier on its own.
    let mut start = 0;
    while start < items.len() {
        let mut end = start;
        while end < items.len() && !walkable_or_idle(&items[end]) {
            end += 1;
        }
        if end > start {
            // Barriers (and actors with nothing pending) run as the translation runs them.
            let whole = std::time::Instant::now();
            for item in &items[start..end] {
                if let Some(b) = item.barrier {
                    BARRIERS.fetch_add(1, Relaxed);
                    REASONS[b as usize].fetch_add(1, Relaxed);
                }
                serial_actor(e, item);
            }
            SERIAL_NS.fetch_add(nanos(whole), Relaxed);
            start = end;
            continue;
        }
        while end < items.len() && walkable_or_idle(&items[end]) {
            end += 1;
        }
        stretch(e, &items[start..end], check);
        start = end;
    }
    PASS_NS.fetch_add(nanos(started), Relaxed);
    if SUMMARY_FRAMES.fetch_add(1, Relaxed) % SUMMARY_EVERY == SUMMARY_EVERY - 1 {
        summary();
    }
    true
}

/// An actor the helpers walk, or one with nothing pending (its update only answers false).
fn walkable_or_idle(item: &Item) -> bool {
    item.barrier.is_none()
}

/// One actor as the translation runs it.
fn serial_actor(e: &mut Engine, item: &Item) {
    let updated = e.call(QUEUED_SCENEGRAPH, &args![item.animation]).bool();
    lighting_if_asked(e, item.actor, updated);
}

/// The guest words one walking actor keeps on main's stack: its update data, then its two saved
/// controller pointers.
const ITEM_WORDS: u32 = UPDATE_DATA_SIZE + 8;

/// A stretch of walkable actors: the controllers detached on main, the walks on the helpers (main
/// takes actors too), then each actor's log replayed on main in order. In check mode the walks are
/// compared with the serial pass instead, which then runs.
fn stretch(e: &mut Engine, items: &[Item], check: bool) {
    let walk: Vec<usize> = (0..items.len()).filter(|&i| items[i].root != 0).collect();
    if walk.len() < 2 {
        for item in items {
            serial_actor(e, item);
        }
        return;
    }
    let bytes = ITEM_WORDS * idx(items.len());
    e.with_stack(bytes, |e, frame| {
        let base = frame.addr();
        // Each walking actor's update data and its roots' controllers detached, in list order.
        for &i in &walk {
            let item = &items[i];
            let at = base + ITEM_WORDS * idx(i);
            let time = e.mem.f32(item.animation + ANIM_TIME);
            e.call(UPDATE_DATA_CONSTRUCT, &args![at, time, 1u32, 0u32]);
            detach(e, item, at + UPDATE_DATA_SIZE);
        }
        let saved = if check {
            Some(save_fields(e, items, &walk))
        } else {
            None
        };
        let walking = std::time::Instant::now();
        let logs = walk_on_helpers(e, items, &walk, base);
        WALK_NS.fetch_add(nanos(walking), Relaxed);
        WALKED.fetch_add(walk.len() as u64, Relaxed);
        if let Some(saved) = saved {
            check_stretch(e, items, &walk, base, &logs, &saved);
            return;
        }
        let replaying = std::time::Instant::now();
        let mut next = 0;
        for (i, item) in items.iter().enumerate() {
            if item.root == 0 {
                serial_actor(e, item);
                continue;
            }
            let at = base + ITEM_WORDS * idx(i);
            replay(e, item, at, &logs[next]);
            next += 1;
            reattach(e, item, at + UPDATE_DATA_SIZE);
            e.call(
                FLAG_SET,
                &args![item.animation, 0u32, u32::from(FLAG_PENDING)],
            );
            lighting_if_asked(e, item.actor, true);
        }
        REPLAY_NS.fetch_add(nanos(replaying), Relaxed);
    });
}

/// `UpdateSceneGraphNoController`'s set-up: the root's and the accumulation root's controllers held
/// in the two words at `saved` and taken off the nodes.
fn detach(e: &mut Engine, item: &Item, saved: u32) {
    let root_controllers = e.call(OBJECT_CONTROLLERS, &args![item.root]).u32();
    e.call(NI_POINTER_INIT, &args![saved, root_controllers]);
    let accum_controllers = e.call(OBJECT_CONTROLLERS, &args![item.accum]).u32();
    e.call(NI_POINTER_INIT, &args![saved + 4, accum_controllers]);
    e.call(OBJECT_SET_CONTROLLERS, &args![item.root, 0u32]);
    e.call(OBJECT_SET_CONTROLLERS, &args![item.accum, 0u32]);
}

/// Its end: the controllers put back and the held pointers released.
fn reattach(e: &mut Engine, item: &Item, saved: u32) {
    let controllers = e.mem.u32(saved);
    e.call(OBJECT_SET_CONTROLLERS, &args![item.root, controllers]);
    let controllers = e.mem.u32(saved + 4);
    e.call(OBJECT_SET_CONTROLLERS, &args![item.accum, controllers]);
    e.call(NI_POINTER_RELEASE, &args![saved + 4]);
    e.call(NI_POINTER_RELEASE, &args![saved]);
}

/// The walks of `walk` (indexes into `items`), each actor's downward pass from its root with its
/// update data, on the helpers and main; each actor's log, in `walk`'s order.
fn walk_on_helpers(e: &mut Engine, items: &[Item], walk: &[usize], base: u32) -> Vec<Vec<Entry>> {
    let next = AtomicUsize::new(0);
    let logs: Vec<Mutex<Option<Vec<Entry>>>> = walk.iter().map(|_| Mutex::new(None)).collect();
    let state = {
        let tls = e.tls();
        e.mem.u32(tls + TLS_UPDATE_STATE)
    };
    let job = |w: &mut Engine, _number: usize| {
        let tls = w.tls();
        let own = w.mem.u32(tls + TLS_UPDATE_STATE);
        w.mem.set_u32(tls + TLS_UPDATE_STATE, state);
        loop {
            let k = next.fetch_add(1, Relaxed);
            if k >= walk.len() {
                break;
            }
            let i = walk[k];
            let item = &items[i];
            let data = base + ITEM_WORDS * idx(i);
            ACTIVE.fetch_add(1, Relaxed);
            LOG.with(|l| *l.borrow_mut() = Some(Vec::new()));
            walk_actor(w, item.root, data);
            let log = LOG.with(|l| l.borrow_mut().take()).unwrap_or_default();
            ACTIVE.fetch_sub(1, Relaxed);
            *logs[k]
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(log);
        }
        w.mem.set_u32(tls + TLS_UPDATE_STATE, own);
    };
    // The pool is the parallel render's when it forked first, made with its own count: a pool with
    // fewer helpers than asked for refuses, so one helper is asked for next.
    let helpers = HELPERS.load(Relaxed);
    if !engine_core::real_threads::fork_join_first(e, helpers, &job)
        && (helpers == 1 || !engine_core::real_threads::fork_join_first(e, 1, &job))
    {
        // No helpers to be had: main walks them all, the same way.
        job(e, 0);
    }
    logs.into_iter()
        .map(|m| {
            m.into_inner()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .unwrap_or_default()
        })
        .collect()
}

/// One actor's walk: the root's downward pass with the update data and 0 (`00493bd0`'s
/// `UpdateDownwardPass` on the animation root), logging what main must run.
fn walk_actor(e: &mut Engine, root: u32, data: u32) {
    // The root is classified as every object is, at the top of the downward pass.
    scene::walk::downward(e, root, data, 0);
}

/// An actor's log on main, in order: each logged subtree's downward pass with the actor's update
/// data, each Havok part; then the merged bounds above the logged subtrees again, deepest first.
fn replay(e: &mut Engine, item: &Item, data: u32, log: &[Entry]) {
    let mut above: Vec<u32> = Vec::new();
    for entry in log {
        match *entry {
            Entry::Subtree {
                object,
                parent,
                flags,
            } => {
                SUBTREES.fetch_add(1, Relaxed);
                scene::walk::downward(e, object, data, flags);
                let mut node = parent;
                while node != 0 {
                    if !above.contains(&node) {
                        above.push(node);
                    }
                    if node == item.root {
                        break;
                    }
                    node = e.mem.u32(node + 0x18);
                }
            }
            Entry::NiToHk { object, .. } => {
                HAVOK.fetch_add(1, Relaxed);
                collision::blend::ni_to_hk_and_clear(e, object);
            }
            Entry::HkToNi { object, .. } => {
                HAVOK.fetch_add(1, Relaxed);
                collision::blend::keyframed_hk_to_ni(e, object);
            }
            Entry::Mismatch { object } => {
                MISMATCHES.fetch_add(1, Relaxed);
                report(format_args!(
                    "NV_PAR_BONES: object {object:08x} of actor {:08x} changed class during its walk",
                    item.actor
                ));
            }
        }
    }
    redo_bounds(e, &above, item.root);
}

/// The children loop's bound part again on `nodes` (each a node above a logged subtree, up to the
/// actor's root), deepest first, from their children's final bounds.
fn redo_bounds(e: &mut Engine, nodes: &[u32], root: u32) {
    let depth = |e: &Engine, mut n: u32| {
        let mut d = 0u32;
        while n != 0 && n != root {
            n = e.mem.u32(n + 0x18);
            d += 1;
        }
        d
    };
    let mut ordered: Vec<(u32, u32)> = nodes.iter().map(|&n| (depth(e, n), n)).collect();
    // Deepest first; among equals in the order they were met.
    ordered.sort_by_key(|x| std::cmp::Reverse(x.0));
    for (_, node) in ordered {
        dispatch::scene::redo_merged_bound(e, Ptr::<NiNode>::new(node));
    }
}

#[cold]
fn report(args: std::fmt::Arguments<'_>) {
    if PRINTED.fetch_add(1, Relaxed) < 20 {
        eprintln!("{args}");
    }
}

// ----- the check -------------------------------------------------------------------------------

/// The fields a walk may write, per object: the flags, the local and world transforms (+0x30 to
/// +0x9c), the world bound's four words, and the collision object's flags word and set byte.
#[derive(Clone, PartialEq, Eq)]
struct Fields {
    object: u32,
    words: Vec<u32>,
    bound: [u32; 4],
    collision: [u32; 2],
}

fn fields_of(e: &Engine, object: u32) -> Fields {
    let words = (0..(0x9c - 0x30) / 4)
        .map(|k| e.mem.u32(object + 0x30 + 4 * k))
        .collect();
    let b = e.mem.u32(object + 0x20);
    let bound = if b != 0 {
        std::array::from_fn(|k| e.mem.u32(b + 4 * idx(k)))
    } else {
        [0; 4]
    };
    let c = e.mem.u32(object + 0x1c);
    let collision = if c != 0 {
        [u32::from(e.mem.u16(c + 0xc)), u32::from(e.mem.u8(c + 0x14))]
    } else {
        [0; 2]
    };
    Fields {
        object,
        words,
        bound,
        collision,
    }
}

fn put_fields(e: &mut Engine, f: &Fields) {
    for (k, w) in (0u32..).zip(&f.words) {
        e.mem.set_u32(f.object + 0x30 + 4 * k, *w);
    }
    let b = e.mem.u32(f.object + 0x20);
    if b != 0 {
        for (k, w) in (0u32..).zip(&f.bound) {
            e.mem.set_u32(b + 4 * k, *w);
        }
    }
    let c = e.mem.u32(f.object + 0x1c);
    if c != 0 {
        e.mem
            .set_u16(c + 0xc, u16::try_from(f.collision[0]).unwrap_or(0));
        e.mem
            .set_u8(c + 0x14, u8::try_from(f.collision[1]).unwrap_or(0));
    }
}

/// Every object of the walking actors' trees, in walk order, with its fields now.
fn save_fields(e: &Engine, items: &[Item], walk: &[usize]) -> Vec<Vec<Fields>> {
    walk.iter()
        .map(|&i| {
            let mut out = Vec::new();
            let mut stack = vec![items[i].root];
            while let Some(object) = stack.pop() {
                out.push(fields_of(e, object));
                if has_children(e, object) {
                    let mut c = children(e, object);
                    c.reverse();
                    stack.extend(c);
                }
            }
            out
        })
        .collect()
}

/// The objects below (or at) the logged subtrees of a log, and the nodes above them.
fn logged_sets(e: &Engine, log: &[Entry], root: u32) -> (Vec<u32>, Vec<u32>) {
    let mut inside = Vec::new();
    let mut above = Vec::new();
    for entry in log {
        if let Entry::Subtree { object, parent, .. } = *entry {
            let mut stack = vec![object];
            while let Some(o) = stack.pop() {
                inside.push(o);
                if has_children(e, o) {
                    stack.extend(children(e, o));
                }
            }
            let mut node = parent;
            while node != 0 {
                above.push(node);
                if node == root {
                    break;
                }
                node = e.mem.u32(node + 0x18);
            }
        }
    }
    (inside, above)
}

/// Check mode for one stretch: the helpers' walks (already made) kept and undone, the controllers
/// put back, then the serial pass on each actor with its Havok parts recorded; the walked objects'
/// fields and the Havok parts compared, and the bounds above the logged subtrees done again on the
/// serial end state and compared.
fn check_stretch(
    e: &mut Engine,
    items: &[Item],
    walk: &[usize],
    base: u32,
    logs: &[Vec<Entry>],
    saved: &[Vec<Fields>],
) {
    let walked: Vec<Vec<Fields>> = saved
        .iter()
        .map(|list| list.iter().map(|f| fields_of(e, f.object)).collect())
        .collect();
    for list in saved {
        for f in list {
            put_fields(e, f);
        }
    }
    for &i in walk {
        reattach(e, &items[i], base + ITEM_WORDS * idx(i) + UPDATE_DATA_SIZE);
    }
    let mut next = 0;
    for item in items {
        if item.root == 0 {
            serial_actor(e, item);
            continue;
        }
        ACTIVE.fetch_add(1, Relaxed);
        RECORD.with(|l| *l.borrow_mut() = Some(Vec::new()));
        serial_actor(e, item);
        let made = RECORD.with(|l| l.borrow_mut().take()).unwrap_or_default();
        ACTIVE.fetch_sub(1, Relaxed);
        let log = &logs[next];
        let (inside, above) = logged_sets(e, log, item.root);
        CHECKED.fetch_add(1, Relaxed);
        // The Havok parts outside the logged subtrees, in order.
        let theirs: Vec<Entry> = log
            .iter()
            .copied()
            .filter(|x| matches!(x, Entry::NiToHk { .. } | Entry::HkToNi { .. }))
            .collect();
        let ours: Vec<Entry> = made
            .into_iter()
            .filter(|x| match *x {
                Entry::NiToHk { object, .. } | Entry::HkToNi { object, .. } => {
                    let node = e.mem.u32(object + 8);
                    !inside.contains(&node)
                }
                _ => false,
            })
            .collect();
        if theirs != ours {
            DIFFERENCES.fetch_add(1, Relaxed);
            report(format_args!(
                "NV_PAR_BONES=check: actor {:08x}: Havok parts {theirs:x?} on the helpers, {ours:x?} serially",
                item.actor
            ));
        }
        if log.iter().any(|x| matches!(x, Entry::Mismatch { .. })) {
            DIFFERENCES.fetch_add(1, Relaxed);
            report(format_args!(
                "NV_PAR_BONES=check: actor {:08x}: an object changed class during its walk",
                item.actor
            ));
        }
        // A logged `NitohkTransform` clears the node-moved bit at its replay: the helper's bytes with
        // that done.
        let cleared: Vec<u32> = log
            .iter()
            .filter_map(|x| match *x {
                Entry::NiToHk { object, .. } => Some(object),
                _ => None,
            })
            .collect();
        for f in &walked[next] {
            if inside.contains(&f.object) {
                continue;
            }
            let now = fields_of(e, f.object);
            let mut expected = f.collision;
            if cleared.contains(&e.mem.u32(f.object + 0x1c)) {
                expected[0] &= 0xffbf;
            }
            let bound_differs = !above.contains(&f.object) && now.bound != f.bound;
            if now.words != f.words || now.collision != expected || bound_differs {
                DIFFERENCES.fetch_add(1, Relaxed);
                report(format_args!(
                    "NV_PAR_BONES=check: actor {:08x}, object {:08x}: transform, flags, bound or collision \
                     bytes differ from the serial pass",
                    item.actor, f.object
                ));
            }
        }
        // The bound part again above the logged subtrees, on the serial end state.
        let mut nodes: Vec<u32> = Vec::new();
        for &n in &above {
            if !nodes.contains(&n) {
                nodes.push(n);
            }
        }
        let before: Vec<[u32; 4]> = nodes.iter().map(|&n| fields_of(e, n).bound).collect();
        redo_bounds(e, &nodes, item.root);
        for (&n, b) in nodes.iter().zip(&before) {
            let now = fields_of(e, n).bound;
            if now != *b {
                DIFFERENCES.fetch_add(1, Relaxed);
                report(format_args!(
                    "NV_PAR_BONES=check: actor {:08x}, node {n:08x}: the bound done again differs",
                    item.actor
                ));
                let bound = e.mem.u32(n + 0x20);
                for (k, w) in (0u32..).zip(b) {
                    e.mem.set_u32(bound + 4 * k, *w);
                }
            }
        }
        next += 1;
    }
}

/// The walked fields (+0x30 to +0x9c) end where the class does.
const _: () = assert!(std::mem::size_of::<crate::culling::layout::NiAVObjectImage>() == 0x9c);

/// An index or count as a guest word (the lists are far below 2^32 entries).
fn idx(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}
