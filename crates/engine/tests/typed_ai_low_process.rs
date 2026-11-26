//! The differential test of the typed functions of the AI family's `LowProcess` module
//! (`engine_typed::ai::low_process`) against their translations, in one world built from a seed.
//!
//! The world: ten objects of 0x400 bytes (the process under test, the actor, a target, two packages,
//! another process, a form that is also a script and a door's base form, a record that is also a
//! script and patrol data, a package target that is also a cell, and a buffer), each word a seeded
//! small integer, float bit pattern or pointer to one of them. The fields the code follows are
//! seeded from the values that choose its branches: the process's embedded package and its start
//! hour, date, level, flags byte, target and timers; each actor's flags, base form, parent cell and
//! the byte at +0x18d; each package's type, flags, procedure, target, scripts and topics; the
//! package target's kind, reference and integer; the base form's type. Each object but the buffer
//! has a vtable of its own: every slot the code calls is a stand-in answering a seeded word (in EAX,
//! and as a float in ST0) and counting its calls, or, by the seed, the exe's own body (the process
//! classes' slots 0x8c, 0x90, 0x12c, 0x22c, 0x238, 0x23c, 0x244, 0x278, 0x27c, 0x280, 0x284 and
//! 0x33c), so the typed side's direct branches run. The globals the code reads (the actor value infos, the
//! procedure lists, the calendar, the frame timer, the zero point, the float -1.0 and the blocked
//! time's limit) are seeded too; the page at 0 is mapped, so a read through a null pointer reads
//! zeros on both sides.
//!
//! What stands in: every callee the code calls by address. Each counts its calls, keeps its last
//! two words and answers the next of eight seeded words. What runs as itself: the family's own
//! functions, those of the families its typed calls reach (the leaves, the AI queries, the CRT's
//! `memset`) and the translations of the callees the typed code does in place.
//! [`the_seeds_reach_every_branch`] checks, on the translations, that the seeds take each deep
//! branch.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_typed::ai::{low_process, AI};
use engine_typed::ai_queries::AI_QUERIES;
use engine_typed::crt::CRT;

/// Where the stand-in slots are registered: `VT_FN + slot`.
const VT_FN: u32 = 0x00ff_4000;
/// A vtable's stand-in answers sit this far past it, at the same slot offset.
const ANSWERS: u32 = 0x1000;
/// The doubles' counts, last words and answers: double `k` at `CONTROL + 0x40 * k`.
const CONTROL: u32 = 0x0300_0000;
/// The slots' call counts: slot `s` at `SLOT_COUNTS + s`.
const SLOT_COUNTS: u32 = CONTROL + 0x2000;
/// The third word of each slot's last call (after `this` and one more): slot `s` at `SLOT_LAST + s`.
const SLOT_LAST: u32 = CONTROL + 0x4000;
/// The procedure lists' rows.
const LISTS: u32 = CONTROL + 0x3000;
/// The bucket tables the allocator gives.
const BUFFER: u32 = 0x0400_0000;

/// The slots the code calls, each a stand-in unless the seed gives the exe's body.
const SLOTS: &[u32] = &[
    0x8c, 0x90, 0x12c, 0x13c, 0x22c, 0x238, 0x23c, 0x244, 0x278, 0x27c, 0x280, 0x284, 0x288, 0x298,
    0x2a4, 0x33c, 0x360, 0x424, 0x810, 0x814,
];
/// The exe's bodies a slot may hold instead.
const REAL_SLOTS: &[(u32, u32)] = &[
    (0x8c, 0x0090_8bf0),
    (0x90, 0x0090_8c50),
    (0x12c, 0x0091_1e80),
    (0x22c, 0x007f_a950),
    (0x238, 0x008d_03c0),
    (0x23c, 0x008d_03e0),
    (0x244, 0x0048_3710),
    (0x278, 0x0071_7e50),
    (0x27c, 0x0085_9670),
    (0x280, 0x008d_0450),
    (0x284, 0x008d_0470),
    (0x33c, 0x0090_dc10),
];

/// A word that stands for object `n` of the world (`OBJECT + n`), resolved when the world is made.
const OBJECT: u32 = 0xb0b0_0000;
const PROCESS: u32 = OBJECT;
const ACTOR: u32 = OBJECT + 1;
const TARGET: u32 = OBJECT + 2;
const PACKAGE: u32 = OBJECT + 3;
const PACKAGE_B: u32 = OBJECT + 4;
const OTHER: u32 = OBJECT + 5;
const FORM: u32 = OBJECT + 6;
const RECORD: u32 = OBJECT + 7;
const LOCATION: u32 = OBJECT + 8;
const COUNT: usize = 10;
/// The objects with a vtable (the package target starts with its kind, the buffer with data).
const WITH_VTABLE: usize = 8;

/// Words an argument or a field may hold.
const WORDS: &[u32] = &[
    0,
    1,
    6,
    0x80,
    0xffff,
    0xffff_ffff,
    0x3f80_0000,
    0x7f80_0001,
    PROCESS,
    ACTOR,
    TARGET,
    PACKAGE,
    FORM,
];
/// Float bit patterns a float may hold: values, the signs of zero, a denormal, an infinity and NaNs
/// (a signalling one, which a load quiets).
const FLOATS: &[u32] = &[
    0x4140_0000,
    0x3fc0_0000,
    0xbf80_0000,
    0x4b80_0001,
    0,
    0x8000_0000,
    1,
    0x7f80_0000,
    0x7f80_0001,
    0xffc0_0000,
];
/// Words a void callee answers (unread).
const UNREAD: &[u32] = &[0, 1];
/// Bytes answers (a `bool` in `AL`, with high bits that must not count).
const BOOLS: &[u32] = &[0, 1, 0x100, 0xff];

/// The callees that stand in, with the words each may answer.
const DOUBLES: &[(u32, &[u32])] = &[
    (0x0043_8af0, UNREAD),                    // NiTMapBase::RemoveAll
    (0x00aa_1070, &[BUFFER, BUFFER + 0x100]), // NiAlloc
    (0x00aa_10f0, UNREAD),                    // NiFree
    (0x006b_8310, UNREAD),                    // the map allocator's free
    (0x0040_1030, UNREAD),                    // the free
    (0x0093_7730, FLOATS),                    // ModifierList::GetModifier
    (0x0093_7480, UNREAD),                    // ModifierList::AdjustModifier
    (0x0066_eea0, FLOATS),                    // ActorValue::CheckClampDamageModifier
    (0x0067_5de0, &[0, FORM]),                // TESPackage::GetLocationCoord
    (0x009f_2ba0, BOOLS),                     // PatrolActorPackageData::Init
    (0x009f_2ea0, BOOLS),                     // its next reference
    (0x009f_3390, &[0, TARGET]),              // its current reference
    (0x0041_cab0, UNREAD),                    // ExtraDataList::SetPackageExtraTarget
    (0x0041_8830, &[0, FORM]),                // the script locals
    (0x005a_c1e0, UNREAD),                    // Script::Run
    (0x0088_4f80, UNREAD),                    // the actor's new package step
    (0x0091_2340, UNREAD),                    // LowProcess::ProcessPatrolStartTravelToNextRef
    (0x0057_23b0, FLOATS),                    // TESObjectREFR::GetDistanceFromReference
    (0x008b_3ab0, UNREAD),                    // Actor::StopMoving
    (0x0067_4d70, UNREAD),                    // TESPackage::SetIsCreated
    (0x0056_8e50, &[0, FORM]),                // the teleport data
    (0x0057_85e0, BOOLS),                     // IsAnOwner
    (0x0056_9160, &[0, 1, FORM]),             // GetLock
    (0x0056_9250, UNREAD),                    // Lock
    (0x0056_92a0, UNREAD),                    // Unlock
    (0x0043_a210, UNREAD),                    // SetConnectedCellsPublic
    (0x0043_a320, &[0, FORM]),                // GetTeleportWorldSpace
    (0x0042_12b0, &[0, RECORD, RECORD]),      // ExtraDataList::GetActorPackageData
    (0x0040_8840, FLOATS),                    // std::abs
];

/// The family's functions whose doubles the first part's world had: their answers are still drawn
/// (so the rest of that world stays as it was) but they run as themselves.
const TYPED_NOW: &[u32] = &[0x0091_2340];

/// The callees the second part's functions call by address, with the words each may answer (drawn
/// from the second stream).
const DOUBLES_2: &[(u32, &[u32])] = &[
    (0x0061_b320, &[0, ITEM, ITEM]), // TESTopic::CreateDialogueItem
    (0x0083_c850, UNREAD),           // DialogueItem::RunResult
    (0x0057_ad20, UNREAD),           // TESObjectREFR::SetSayToTopic
    (0x0057_ace0, UNREAD),           // its SetSayToTopicInfo
    (0x0093_6a20, UNREAD),           // MobileObject::SayToCallBack
    (0x005a_c750, UNREAD),           // Script::SetActionFlag
    (0x0088_1450, &[0, PACKAGE, PACKAGE_B]), // Actor::GetPackage
    (0x008b_3d30, UNREAD),           // Actor::ClearLookAtTarget
    (0x0057_5d70, &[0, FORM]),       // TESObjectREFR::GetWorldSpace
    (0x0056_ae10, UNREAD),           // TESObjectREFR::SetPackageStartLocation
    (0x0098_0ae0, UNREAD),           // CombatController::UpdatePackageData
    (0x007b_3fa0, UNREAD),           // the acquire object's deleting destructor
    (0x0090_5330, UNREAD),           // the object list's remove (empties it)
    (0x0047_0470, UNREAD),           // the locations list's clear
    (0x0090_7260, FLOATS),           // LowProcess::GetTimeDelta
    (0x0091_25b0, FLOATS),           // the patrol's travel step
    (0x0041_fe90, &[0, FORM, RECORD]), // the reference's patrol extra
    (0x006a_7f50, FLOATS),           // its float
    (0x0040_4010, FLOATS),           // the larger float
    (0x0045_78c0, UNREAD),           // NiPoint3's difference
    (0x004a_7290, FLOATS),           // its squared length
    (0x0094_df60, BOOLS),            // the player sleeps or rests
    (0x0045_11e0, BOOLS),            // TES::IsCellLoaded
    (0x006e_25d0, FLOATS),           // the patrol's travel time
    (0x0067_6280, &[0, 1, 6, 0x0100_0001]), // the package's count for the actor
    (0x006e_2420, UNREAD),           // PathingRequest's constructor
    (0x006e_2620, UNREAD),           // its destructor
    (0x008b_3880, UNREAD),           // Actor::BuildRequest
    (0x008e_7c50, UNREAD),           // the request's flag (in place: not doubled)
    (0x008e_7c20, UNREAD),           // its angle (in place: not doubled)
    (0x008b_3630, UNREAD),           // Actor::SetPathfindingGoal
    (0x00ec_43fb, &[0, ACTOR, TARGET]), // __RTDynamicCast
    (0x0093_7160, UNREAD),           // ModifierList's copy
];
/// The object list's remove.
const LIST_REMOVE_ITEM: u32 = 0x0090_5330;

/// The slots the second part's functions call, stand-ins unless the seed gives the exe's body.
const SLOTS_2: &[u32] = &[
    0x14, 0xb0, 0x118, 0x140, 0x1d0, 0x1f4, 0x214, 0x230, 0x240, 0x25c, 0x418, 0x428, 0x4ec, 0x5a0,
    0x5a4, 0x5a8,
];
/// The exe's bodies a slot may hold instead (also over the first part's stand-ins 0x288 and 0x2a4).
const REAL_SLOTS_2: &[(u32, u32)] = &[
    (0x288, 0x0091_19b0),
    (0x2a4, 0x0091_36b0),
    (0x5a0, 0x0091_3630),
    (0x5a4, 0x008d_8000),
    (0x5a8, 0x008d_8020),
    (0x118, 0x008d_7de0),
    (0x4ec, 0x008d_7fe0),
    (0x240, 0x008d_0400),
    (0xb0, 0x0090_7840),
];
/// The callees the exe's `BaseProcess::SetCurrentPackage` (slot 0x230's body, `008d08b0`) reaches
/// through the typed `ActorPackage::InitFromPackage` and `Clear`, with the words each may answer
/// (drawn from the third stream).
const DOUBLES_3: &[(u32, &[u32])] = &[
    (0x0067_59e0, &[0, TARGET, LOCATION]), // TESPackage::ComputeInitialTarget
    (0x008c_6ca0, &[0, RECORD]),           // the new package data for a type
    (0x0040_b460, &[1, 2]),                // the package's count up
    (0x0040_19a0, &[0, 1]),                // the count down
];
/// The slots the third part's code calls, stand-ins on every object: the data's and the package's
/// deleting destructors.
const SLOTS_3: &[u32] = &[0, 0x10];
/// The exe's bodies a slot may hold instead, from the third stream.
const REAL_SLOTS_3: &[(u32, u32)] = &[(0x230, 0x008d_08b0)];
/// The dialogue item `CreateDialogueItem` answers and a package location of the schedule's.
const ITEM: u32 = BUFFER + 0x8000;
const LOCATION_2: u32 = BUFFER + 0x8100;
/// The globals the second part reads.
const PROCEDURE_COUNTS: u32 = 0x011a_3fbc;
const SECONDS_PER_HOUR: u32 = 0x0101_2640;
const PATROL_NEAR: u32 = 0x0101_6978;
const ANGLED_FORM: u32 = 0x011c_a248;
const TES: u32 = 0x011d_ea10;
const PLAYER: u32 = 0x011d_ea3c;

/// The typed callees of families this test does not run, so they stand in on both sides.
const BY_ADDRESS: &[u32] = &[
    0x0042_12b0,
    0x0040_8840,
    0x0045_11e0,
    0x0040_4010,
    0x006a_7f50,
    0x004a_7290,
    0x0040_b460,
    0x0040_19a0,
];

/// The callees the translations call and the typed code does in place (one load, store or test
/// each), or leaves out (a getter whose answer the exe never reads).
const IN_PLACE: &[u32] = &[
    0x0067_4d40, // the package's created flag (`ActorPackage::Clear` under slot 0x230's body)
    0x008d_7e00, // the process flags' setter
    0x0084_d030, // the frame timer's step
    0x0056_6950, // the actor's byte
    0x0056_4db0, // TESObjectREFR::SetTargeted
    0x0040_3550, // the package's target setter
    0x0071_7e50, // the embedded package
    0x0048_3710, // the empty body
    0x0067_1d10, // the leaves
    0x0044_ddc0,
    0x0072_6070,
    0x0045_cd60,
    0x007a_f430,
    0x0090_3cf0, // the package's scripts and topics
    0x0090_3cd0,
    0x0090_3bd0,
    0x0090_3bb0,
    0x0055_b980, // the script's words, the package's location
    0x0096_11e0, // the package's type
    0x0041_ca90, // its procedure
    0x0044_1110, // its flags
    0x0067_efd0, // its flag 4
    0x0040_6d70, // the actor value's flag
    0x005d_43c0, // the extra data list
    0x0040_1170, // the form's type
    0x0042_5fd0, // the interior test
    0x008d_6f30, // the parent cell
    0x0086_7da0, // Calendar::GetHour
    0x0052_6ac0, // a global's value
    0x0062_1b00, // the package's start hour
    0x0041_fd00, // the package's procedure index's store (slot 0x238's body does it in place)
    0x0068_0020, // the package target's reference and its kind
    0x0051_9b00,
    0x0044_0d80, // a reference's flag 0x20
    0x0090_3e20, // the package's done script and topic
    0x0090_3e00,
    0x004f_d380, // the actor's speaker, read and cleared
    0x0057_bd60,
    0x0050_8d70, // the topic's dialogue flag
    0x0083_c7b0, // the dialogue item's first and current response
    0x0083_c820,
    0x0068_15c0,
    0x0084_e3a0, // the word at +0xc
    0x008d_0430, // the running instance data
    0x007d_f1f0, // the hour last processed, read and stored
    0x0069_3d50,
    0x0059_bb30, // the date last processed
    0x0043_0830, // the start location
    0x0067_8ca0, // the package location's kind
    0x008d_7de0, // the process flag setters and the done-once test
    0x008d_7fe0,
    0x008d_8000,
    0x008d_8020,
    0x008d_7e60,
    0x0082_56d0, // the list's empty test
    0x008d_03c0, // the current procedure index's slots
    0x008d_03e0,
    0x008d_0400,
    0x0049_bb50,
    0x0096_a820, // the procedure count
    0x008e_7c50, // the pathing request's byte at +0x9c, its angle and flag
    0x008e_7c20,
];

/// The globals the code reads.
const AV_INFOS: u32 = 0x011d_61c8;
const PROCEDURE_LISTS: u32 = 0x011a_3ff0;
const CALENDAR: u32 = 0x011d_e7b8;
const DEFAULT_HOUR: u32 = 0x0101_8f7c;
const FRAME_TIMER: u32 = 0x011f_6394;
const ZERO_POINT: u32 = 0x011f_426c;
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
const BLOCKED_TIME_LIMIT: u32 = 0x0101_ffa0;

/// The seeds: a few for the functions with no branch, many for the rest.
static FEW: [u32; 8] = seeds();
static MANY: [u32; 64] = seeds();
static MOST: [u32; 192] = seeds();

const fn seeds<const N: usize>() -> [u32; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = i as u32 + 1;
        i += 1;
    }
    out
}

fn ret(word: u32) -> Ret {
    Ret {
        eax: word,
        edx: 0,
        st0: f64::from(f32::from_bits(word)),
    }
}

/// A double's block.
fn block(k: usize) -> u32 {
    CONTROL + 0x40 * k as u32
}

/// How many times the double at `addr` was called.
fn calls(e: &Engine, addr: u32) -> u32 {
    e.mem.u32(block(double_index(addr)))
}

/// The block of the double at `addr`: the first part's, then the second's.
fn double_index(addr: u32) -> usize {
    match DOUBLES.iter().position(|(a, _)| *a == addr) {
        Some(k) => k,
        None => match DOUBLES_2.iter().position(|(a, _)| *a == addr) {
            Some(k) => DOUBLES.len() + k,
            None => {
                let k = DOUBLES_3
                    .iter()
                    .position(|(a, _)| *a == addr)
                    .expect("a double");
                DOUBLES.len() + DOUBLES_2.len() + k
            }
        },
    }
}

/// The answer the double at `addr` gave last.
fn last_answer(e: &Engine, addr: u32) -> u32 {
    let k = double_index(addr);
    let n = e.mem.u32(block(k));
    e.mem.u32(block(k) + 4 + 4 * ((n + 7) % 8))
}

/// The third word of the last call of `slot`.
fn slot_last(e: &Engine, slot: u32) -> u32 {
    e.mem.u32(SLOT_LAST + slot)
}

fn slot_calls(e: &Engine, slot: u32) -> u32 {
    e.mem.u32(SLOT_COUNTS + slot)
}

/// The world of one seed, made the same on both engines.
struct World {
    objects: [u32; COUNT],
    rng: u64,
    /// The second part's stream.
    rng2: u64,
    /// The third part's stream.
    rng3: u64,
}

impl World {
    fn new(e: &mut Engine, seed: u32) -> World {
        let mut w = World {
            objects: [0; COUNT],
            rng: (u64::from(seed) ^ 0x9e37_79b9_7f4a_7c15).wrapping_mul(0xbf58_476d_1ce4_e5b9) | 1,
            rng2: (u64::from(seed) ^ 0x94d0_49bb_1331_11eb).wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1,
            rng3: (u64::from(seed) ^ 0x2545_f491_4f6c_dd1d).wrapping_mul(0xd6e8_feb8_6659_fd93) | 1,
        };
        e.map(0, 0x1000);
        e.map(CONTROL, 0x5000);
        e.map(BUFFER, 0x1_0000);
        for &slot in SLOTS {
            e.register_double(VT_FN + slot, move |e, a| {
                let n = e.mem.u32(SLOT_COUNTS + slot);
                let third = a.get(2).copied().unwrap_or(0);
                e.mem.set_u32(SLOT_LAST + slot, third);
                e.mem.set_u32(SLOT_COUNTS + slot, n + 1);
                let vt = e.mem.u32(a[0]);
                ret(e.mem.u32(vt + ANSWERS + slot))
            });
        }
        for (k, (addr, answers)) in DOUBLES.iter().enumerate() {
            let block = block(k);
            for i in 0..8 {
                let answer = w.pick(answers);
                e.mem.set_u32(block + 4 + 4 * i, answer);
            }
            if TYPED_NOW.contains(addr) {
                continue;
            }
            e.register_double(*addr, move |e, a| {
                let n = e.mem.u32(block);
                e.mem.set_u32(block, n + 1);
                e.mem.set_u32(block + 0x24, a.first().copied().unwrap_or(0));
                e.mem.set_u32(block + 0x28, a.get(1).copied().unwrap_or(0));
                ret(e.mem.u32(block + 4 + 4 * (n % 8)))
            });
        }
        for i in 0..COUNT {
            w.objects[i] = e.mem.alloc(0x400);
        }
        // The doubles' answers name the objects: resolve them now they exist.
        for k in 0..DOUBLES.len() {
            for i in 0..8 {
                let at = block(k) + 4 + 4 * i;
                let word = w.resolve(e.mem.u32(at));
                e.mem.set_u32(at, word);
            }
        }
        // Every object but the package target and the buffer has a vtable, and so has the null
        // object (the word at 0), so a call through a null pointer the exe would make answers on both
        // sides.
        for i in 0..=COUNT {
            let o = if i < COUNT { w.objects[i] } else { 0 };
            if i < COUNT {
                for offset in (0..0x400).step_by(4) {
                    let word = w.word();
                    e.mem.set_u32(o + offset, word);
                }
            }
            if (WITH_VTABLE..COUNT).contains(&i) {
                continue;
            }
            let vt = e.mem.alloc(2 * ANSWERS);
            e.mem.set_u32(o, vt);
            for &slot in SLOTS {
                e.mem.set_u32(vt + slot, VT_FN + slot);
                let answers: &[u32] = match slot {
                    0x13c | 0x360 => BOOLS,
                    0x22c => &[0, PACKAGE, PACKAGE, PACKAGE_B],
                    0x23c | 0x280 => &[0xffff_ffff, 0, 1, 2, 3],
                    0x278 => &[OTHER, OTHER, FORM],
                    0x27c => &[0, PACKAGE, PACKAGE_B],
                    0x810 => &[0, TARGET, ACTOR],
                    0x814 => &[0, LOCATION],
                    _ => WORDS,
                };
                let answer = w.pick(answers);
                e.mem.set_u32(vt + ANSWERS + slot, w.resolve(answer));
            }
            for &(slot, body) in REAL_SLOTS {
                if w.flag() {
                    e.mem.set_u32(vt + slot, body);
                }
            }
        }
        w.seed_fields(e);
        w.seed_globals(e);
        w.seed_part_2(e);
        w.seed_part_3(e);
        w
    }

    /// The fields that choose the branches.
    fn seed_fields(&mut self, e: &mut Engine) {
        // The processes: the embedded package and its start hour, the level, the flags byte, the
        // target and the timers. The slot 0x22c's answer, for the slots that hold the exe's body.
        for o in [self.objects[0], self.objects[5]] {
            let package = self.one_of(&[0, PACKAGE, PACKAGE, PACKAGE_B]);
            e.mem.set_u32(o + 4, package);
            // The embedded package's procedure index (slot 0x23c's body reads it).
            let index = self.pick(&[0xffff_ffff, 0, 1, 2, 3]);
            e.mem.set_u32(o + 0x10, index);
            let start = self.pick(FLOATS);
            e.mem.set_u32(o + 0x14, start);
            let level = self.pick(&[0, 1, 1, 2, 3, 0xffff_ffff]);
            e.mem.set_u32(o + 0x28, level);
            let flags = self.pick(&[0, 0x40, 0x20, 0xc4, 0xff]);
            e.mem.set_u8(o + 0x30, flags as u8);
            let target = self.one_of(&[0, 0, TARGET, ACTOR]);
            e.mem.set_u32(o + 0x40, target);
            for at in [0xa4, 0xac] {
                let timer = self.pick(FLOATS);
                e.mem.set_u32(o + at, timer);
            }
        }
        // The actors: flags, base form, parent cell, the byte at +0x18d.
        for o in [self.objects[1], self.objects[2]] {
            let flags = self.pick(&[0, 0x20, 0x0004_0000, 0x0004_0020]);
            e.mem.set_u32(o + 8, flags);
            let form = self.one_of(&[0, FORM, FORM, RECORD]);
            e.mem.set_u32(o + 0x20, form);
            let cell = self.one_of(&[0, LOCATION, LOCATION]);
            e.mem.set_u32(o + 0x40, cell);
            let byte = self.pick(&[0, 1]);
            e.mem.set_u8(o + 0x18d, byte as u8);
        }
        // The packages: type, flags, procedure, target, scripts and topics.
        for o in [self.objects[3], self.objects[4]] {
            let kind = self.pick(&[0xffff_ffff, 0, 0, 1, 2, 3]);
            e.mem.set_u32(o + 0x18, kind);
            let mut flags = 0;
            for bit in [4, 0x0200_0000, 0x0400_0000] {
                if self.flag() {
                    flags |= bit;
                }
            }
            e.mem.set_u32(o + 0x1c, flags);
            let procedure = self.pick(&[0, 1, 6, 7, 0xd, 0xd, 0xe, 0x11, 5, 0xff, 0x1c]);
            e.mem.set_u8(o + 0x20, procedure as u8);
            let target = self.one_of(&[0, LOCATION, LOCATION]);
            e.mem.set_u32(o + 0x30, target);
            for at in [0x50, 0x54, 0x70, 0x74] {
                let word = self.one_of(&[0, FORM, RECORD]);
                e.mem.set_u32(o + at, word);
            }
        }
        // The form (a door's base form when its type is 0x1c) and the record are scripts too (their
        // words at +0x2c and +0x30); the record is patrol data as well (its state at +4).
        for o in [self.objects[6], self.objects[7]] {
            let kind = self.pick(&[0x1c, 0x1c, 0x2a]);
            e.mem.set_u8(o + 4, kind as u8);
            for at in [0x2c, 0x30] {
                let word = self.pick(&[0, 0, 1]);
                e.mem.set_u32(o + at, word);
            }
        }
        let state = self.pick(&[0, 0, 1, 2]);
        e.mem.set_u32(self.objects[7] + 4, state);
        // The package target: its kind byte, reference and integer; as a cell, its interior byte.
        let l = self.objects[8];
        let kind = self.pick(&[0, 0, 3]);
        e.mem.set_u32(l, kind);
        let reference = self.one_of(&[0, TARGET, TARGET, ACTOR]);
        e.mem.set_u32(l + 4, reference);
        let value = self.pick(&[0, 100, 0xffff_fffb, 0x0100_0001]);
        e.mem.set_u32(l + 8, value);
        let interior = self.pick(&[0, 1]);
        e.mem.set_u8(l + 0x24, interior as u8);
    }

    fn seed_globals(&mut self, e: &mut Engine) {
        for page in [
            0x011d_6000,
            0x011d_7000,
            0x011d_e000,
            0x011a_3000,
            0x0101_8000,
            0x011f_6000,
            0x011f_4000,
            0x0101_2000,
            0x0101_f000,
        ] {
            e.map(page, 0x1000);
        }
        // The actor value infos: four infos with seeded flags, every entry one of them.
        let mut infos = [0u32; 4];
        for info in &mut infos {
            *info = e.mem.alloc(0x50);
            let flags = self.pick(&[0, 0x100]);
            e.mem.set_u32(*info + 0x44, flags);
        }
        for i in 0..0x3ff {
            let info = infos[(self.next() % 4) as usize];
            e.mem.set_u32(AV_INFOS - 4 + 4 * i, info);
        }
        // The procedure lists: a row per package type (-1 to 3), each a seeded procedure per running
        // index (-1 to 3).
        for kind in 0..5 {
            let row = LISTS + 0x10 + 0x20 * kind;
            e.mem.set_u32(PROCEDURE_LISTS - 4 + 4 * kind, row);
            for index in 0..5 {
                let procedure = self.pick(&[0, 1, 1, 0x36, 0x36, 5]);
                e.mem.set_u32(row - 4 + 4 * index, procedure);
            }
        }
        // The calendar's hour global, the default hour, the frame step, the zero point, -1.0 and
        // the blocked time's limit.
        let global = self.one_of(&[0, FORM]);
        e.mem.set_u32(CALENDAR + 0xc, global);
        let hour = self.pick(FLOATS);
        e.mem.set_u32(self.objects[6] + 0x24, hour);
        let hour = self.pick(&[0x4140_0000, 0x41a0_0000, 0x7f80_0001]);
        e.mem.set_u32(DEFAULT_HOUR, hour);
        let step = self.pick(FLOATS);
        e.mem.set_u32(FRAME_TIMER + 0xc, step);
        for i in 0..3 {
            let word = self.pick(FLOATS);
            e.mem.set_u32(ZERO_POINT + 4 * i, word);
        }
        let bits = self.pick(FLOATS);
        e.mem.set_u32(MINUS_ONE_FLOAT, bits);
        let limit = [0.0, 1.0, 12.5, f64::NAN][(self.next() % 4) as usize];
        e.mem.set_f64(BLOCKED_TIME_LIMIT, limit);
    }

    /// The second part's doubles, slots, globals and fields, from the second stream, so the first
    /// part's world stays as it was.
    fn seed_part_2(&mut self, e: &mut Engine) {
        for (k, (addr, answers)) in DOUBLES_2.iter().enumerate() {
            let block = block(DOUBLES.len() + k);
            for i in 0..8 {
                let answer = self.pick2(answers);
                let answer = self.resolve(answer);
                e.mem.set_u32(block + 4 + 4 * i, answer);
            }
            // A callee done in place runs as itself on the translation's side.
            if IN_PLACE.contains(addr) {
                continue;
            }
            let empties = *addr == LIST_REMOVE_ITEM;
            e.register_double(*addr, move |e, a| {
                let n = e.mem.u32(block);
                e.mem.set_u32(block, n + 1);
                e.mem.set_u32(block + 0x24, a.first().copied().unwrap_or(0));
                e.mem.set_u32(block + 0x28, a.get(1).copied().unwrap_or(0));
                // The object list is empty after the remove's second call.
                if empties && n > 0 {
                    e.mem.set_u32(a[0], 0);
                    e.mem.set_u32(a[0] + 4, 0);
                }
                ret(e.mem.u32(block + 4 + 4 * (n % 8)))
            });
        }
        // Each round of the patrol's answers ends with a failure (or no time left), so its loop ends.
        for addr in [0x009f_2ba0, 0x009f_2ea0, 0x0091_25b0] {
            e.mem.set_u32(block(double_index(addr)) + 4 + 4 * 7, 0);
        }
        for &slot in SLOTS_2 {
            e.register_double(VT_FN + slot, move |e, a| {
                let n = e.mem.u32(SLOT_COUNTS + slot);
                let third = a.get(2).copied().unwrap_or(0);
                e.mem.set_u32(SLOT_LAST + slot, third);
                e.mem.set_u32(SLOT_COUNTS + slot, n + 1);
                let vt = e.mem.u32(a[0]);
                ret(e.mem.u32(vt + ANSWERS + slot))
            });
        }
        for i in 0..=COUNT {
            if (WITH_VTABLE..COUNT).contains(&i) {
                continue;
            }
            let o = if i < COUNT { self.objects[i] } else { 0 };
            let vt = e.mem.u32(o);
            for &slot in SLOTS_2 {
                e.mem.set_u32(vt + slot, VT_FN + slot);
                let answers: &[u32] = match slot {
                    0x1f4 => &[ACTOR, TARGET, FORM],
                    0x214 => &[0, 0, 1],
                    0x428 => &[0, FORM],
                    0x5a4 => BOOLS,
                    _ => UNREAD,
                };
                let answer = self.pick2(answers);
                let answer = self.resolve(answer);
                e.mem.set_u32(vt + ANSWERS + slot, answer);
            }
            for &(slot, body) in REAL_SLOTS_2 {
                if self.flag2() {
                    e.mem.set_u32(vt + slot, body);
                }
            }
        }
        // The globals: the procedure counts (up to the lists' row of type -1), an hour's seconds,
        // the patrol's near limit, the angled form, `TES` and the player.
        e.map(0x0101_6000, 0x1000);
        e.map(0x011c_a000, 0x1000);
        for code in 0..0x30 {
            let count = self.pick2(&[0, 1, 2, 3, 5, 0xff]);
            e.mem.set_u8(PROCEDURE_COUNTS + code, count as u8);
        }
        e.mem.set_f64(SECONDS_PER_HOUR, 3600.0);
        let near = [0.0, 1.0, 1.0e6, f64::NAN][(self.next2() % 4) as usize];
        e.mem.set_f64(PATROL_NEAR, near);
        let form = self.one_of2(&[0, FORM, FORM]);
        e.mem.set_u32(ANGLED_FORM, form);
        let tes = self.one_of2(&[0, FORM]);
        e.mem.set_u32(TES, tes);
        let player = self.one_of2(&[0, ACTOR, OTHER]);
        e.mem.set_u32(PLAYER, player);
        // The process: its running instance data (the patrol data), the object list's two words,
        // the acquire objects.
        let p = self.objects[0];
        let data = self.one_of2(&[0, RECORD, RECORD]);
        e.mem.set_u32(p + 8, data);
        let head = self.one_of2(&[0, 0, RECORD, FORM]);
        e.mem.set_u32(p + 0x5c, head);
        let tail = self.pick2(&[0, 1]);
        e.mem.set_u32(p + 0x60, tail);
        for at in [0x64, 0x68] {
            let object = self.one_of2(&[0, FORM, RECORD]);
            e.mem.set_u32(p + at, object);
        }
        // The patrol data's time waited, the topic's flags, the dialogue item's response and info,
        // the packages' locations, done scripts and topics, and the location's kind.
        let waited = self.pick2(FLOATS);
        e.mem.set_u32(self.objects[7] + 8, waited);
        let flags = self.pick2(&[0, 0x2000, 0x2000, 0x2001]);
        e.mem.set_u32(self.objects[9] + 8, flags);
        let response = self.one_of2(&[0, FORM]);
        e.mem.set_u32(ITEM, response);
        let info = self.one_of2(&[0, RECORD]);
        e.mem.set_u32(ITEM + 0xc, info);
        for o in [self.objects[3], self.objects[4]] {
            let location = self.pick2(&[0, LOCATION_2, LOCATION_2]);
            e.mem.set_u32(o + 0x2c, location);
            // The done script and topic.
            for at in [0x60, 0x64] {
                let word = self.one_of2(&[0, FORM, RECORD]);
                e.mem.set_u32(o + at, word);
            }
        }
        let kind = self.pick2(&[0, 1, 2, 2]);
        e.mem.set_u8(LOCATION_2, kind as u8);
    }

    /// The third part's doubles and slots, from the third stream, so the first two parts' worlds
    /// stay as they were.
    fn seed_part_3(&mut self, e: &mut Engine) {
        let first = DOUBLES.len() + DOUBLES_2.len();
        for (k, (addr, answers)) in DOUBLES_3.iter().enumerate() {
            let block = block(first + k);
            for i in 0..8 {
                let answer = self.pick3(answers);
                let answer = self.resolve(answer);
                e.mem.set_u32(block + 4 + 4 * i, answer);
            }
            e.register_double(*addr, move |e, a| {
                let n = e.mem.u32(block);
                e.mem.set_u32(block, n + 1);
                e.mem.set_u32(block + 0x24, a.first().copied().unwrap_or(0));
                e.mem.set_u32(block + 0x28, a.get(1).copied().unwrap_or(0));
                ret(e.mem.u32(block + 4 + 4 * (n % 8)))
            });
        }
        for &slot in SLOTS_3 {
            e.register_double(VT_FN + slot, move |e, _| {
                let n = e.mem.u32(SLOT_COUNTS + slot);
                e.mem.set_u32(SLOT_COUNTS + slot, n + 1);
                ret(0)
            });
        }
        for i in 0..=COUNT {
            if (WITH_VTABLE..COUNT).contains(&i) {
                continue;
            }
            let o = if i < COUNT { self.objects[i] } else { 0 };
            let vt = e.mem.u32(o);
            for &slot in SLOTS_3 {
                e.mem.set_u32(vt + slot, VT_FN + slot);
            }
            for &(slot, body) in REAL_SLOTS_3 {
                if self.flag3() {
                    e.mem.set_u32(vt + slot, body);
                }
            }
        }
    }

    fn next3(&mut self) -> u64 {
        self.rng3 ^= self.rng3 << 13;
        self.rng3 ^= self.rng3 >> 7;
        self.rng3 ^= self.rng3 << 17;
        self.rng3
    }

    fn pick3(&mut self, from: &[u32]) -> u32 {
        from[(self.next3() % from.len() as u64) as usize]
    }

    fn flag3(&mut self) -> bool {
        self.next3() & 1 != 0
    }

    fn next2(&mut self) -> u64 {
        self.rng2 ^= self.rng2 << 13;
        self.rng2 ^= self.rng2 >> 7;
        self.rng2 ^= self.rng2 << 17;
        self.rng2
    }

    fn pick2(&mut self, from: &[u32]) -> u32 {
        from[(self.next2() % from.len() as u64) as usize]
    }

    fn flag2(&mut self) -> bool {
        self.next2() & 1 != 0
    }

    fn one_of2(&mut self, from: &[u32]) -> u32 {
        let word = self.pick2(from);
        self.resolve(word)
    }

    fn next(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn pick(&mut self, from: &[u32]) -> u32 {
        from[(self.next() % from.len() as u64) as usize]
    }

    fn flag(&mut self) -> bool {
        self.next() & 1 != 0
    }

    /// An object's word for `OBJECT + n`; any other word as it is.
    fn resolve(&self, word: u32) -> u32 {
        match word.wrapping_sub(OBJECT) {
            n if (n as usize) < COUNT => self.objects[n as usize],
            _ => word,
        }
    }

    /// A seeded word, an object's address for the words that name one.
    fn word(&mut self) -> u32 {
        let word = self.pick(WORDS);
        self.resolve(word)
    }

    /// One of these, resolved.
    fn one_of(&mut self, from: &[u32]) -> u32 {
        let word = self.pick(from);
        self.resolve(word)
    }

    fn process(&self) -> u32 {
        self.objects[0]
    }

    /// The process, then `n` seeded words.
    fn process_and(&mut self, n: usize) -> Vec<u32> {
        let mut words = vec![self.process()];
        for _ in 0..n {
            words.push(self.word());
        }
        words
    }

    /// The process and an actor (or none, or a form that is not one).
    fn process_actor(&mut self) -> Vec<u32> {
        let actor = self.one_of(&[0, ACTOR, ACTOR, TARGET]);
        vec![self.process(), actor]
    }

    /// The process, an actor and a package (or none).
    fn process_actor_package(&mut self) -> Vec<u32> {
        let actor = self.one_of(&[0, ACTOR, ACTOR]);
        let package = self.one_of(&[0, PACKAGE, PACKAGE, PACKAGE_B]);
        vec![self.process(), actor, package]
    }

    /// A door (or another reference, or none) and an actor (or none).
    fn door_actor(&mut self) -> Vec<u32> {
        let door = self.one_of(&[0, TARGET, TARGET, ACTOR]);
        let actor = self.one_of(&[0, ACTOR, ACTOR, TARGET]);
        vec![door, actor]
    }
}

/// A test in the world: the families the code reaches on, the world made from each seed, the
/// call's words from `words`, and the callees done in place ignored.
trait InWorld {
    fn in_world(self, seeds: &'static [u32], words: fn(&mut World) -> Vec<u32>) -> Diff;
}

impl InWorld for Diff {
    fn in_world(self, seeds: &'static [u32], words: fn(&mut World) -> Vec<u32>) -> Diff {
        self.family(&AI)
            .family(&leaf::LEAVES)
            .family(&AI_QUERIES)
            .family(&CRT)
            .ignore_calls(IN_PLACE)
            .goes_by_address(BY_ADDRESS)
            .inputs(&[Input::Of(seeds)])
            .setup(move |e, case| {
                let mut w = World::new(e, case.word(0));
                words(&mut w)
            })
    }
}

#[test]
fn lock_map() {
    let buckets = |w: &mut World| {
        let buckets = w.pick(&[0, 1, 0x25, 0x400]);
        vec![w.objects[9], buckets]
    };
    Diff::new(entry!(0x0091_37b0, low_process::lock_map_construct(Ptr, u32) -> Ptr))
        .in_world(&FEW, buckets)
        .run()
        .assert_same();
    Diff::new(entry!(0x0091_3810, low_process::lock_map_base_construct(Ptr, u32) -> Ptr))
        .in_world(&FEW, buckets)
        .run()
        .assert_same();
    Diff::new(entry!(0x0091_3880, low_process::lock_map_destructor(Ptr)))
        .in_world(&FEW, |w| vec![w.objects[9]])
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0091_38e0,
        low_process::lock_map_base_destructor(Ptr)
    ))
    .in_world(&FEW, |w| vec![w.objects[9]])
    .run()
    .assert_same();
    let flags = |w: &mut World| {
        let flags = w.pick(&[0, 1, 2, 3]);
        vec![w.objects[9], flags]
    };
    Diff::new(entry!(
        0x0091_37e0,
        low_process::lock_map_scalar_deleting_destructor(Ptr, u32) -> Ptr
    ))
    .in_world(&MANY, flags)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_3980,
        low_process::lock_map_base_scalar_deleting_destructor(Ptr, u32) -> Ptr
    ))
    .in_world(&MANY, flags)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_3910,
        low_process::lock_map_delete_item(Ptr, u32)
    ))
    .in_world(&FEW, |w| vec![w.objects[9], w.objects[7]])
    // The lock data's constructor, which the typed version does in place.
    .ignore_calls(&[0x0091_3950])
    .run()
    .assert_same();
    Diff::new(entry!(0x0091_3950, low_process::lock_data_construct(Ptr, u32) -> Ptr))
        .in_world(&FEW, |w| w.process_and(1))
        .run()
        .assert_same();
}

#[test]
fn dates_flags_and_timers() {
    Diff::new(entry!(0x0090_76a0, low_process::processed_day(Ptr) -> u32))
        .in_world(&FEW, |w| w.process_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(0x0090_76c0, low_process::processed_month(Ptr) -> u32))
        .in_world(&FEW, |w| w.process_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(0x0090_76e0, low_process::processed_year(Ptr) -> u32))
        .in_world(&FEW, |w| w.process_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(0x0090_ac20, low_process::set_flag_40(Ptr, u8)))
        .in_world(&MANY, |w| {
            let set = w.pick(&[0, 1, 2, 0x100, 0xff]);
            vec![w.process(), set]
        })
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0090_7620,
        low_process::mod_essential_down_timer(Ptr)
    ))
    .in_world(&MANY, |w| w.process_and(0))
    // A difference of two NaNs: SSE keeps the first one's payload, x87 the larger one's. Only
    // here, in the tracked damage's sum and in the blocked time's difference: every other Diff
    // compares NaN-looking words exactly.
    .allow_nan_payloads()
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_34c0,
        low_process::mod_tracked_damage(Ptr, u32, f32)
    ))
    .in_world(&MOST, |w| {
        let actor = w.one_of(&[0, ACTOR, TARGET]);
        let amount = w.pick(FLOATS);
        vec![w.process(), actor, amount]
    })
    .allow_nan_payloads()
    .run()
    .assert_same();
    Diff::new(entry!(0x0091_1ae0, low_process::float_zero(Ptr, u32, u32) -> f32))
        .in_world(&FEW, |w| w.process_and(2))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0091_2150,
        low_process::process_movement_blocked(Ptr, u32)
    ))
    .in_world(&MOST, |w| w.process_actor())
    .allow_nan_payloads()
    .run()
    .assert_same();
}

#[test]
fn procedure_steps() {
    Diff::new(entry!(
        0x0090_8f40,
        low_process::process_avoid_area(Ptr, u32)
    ))
    .in_world(&FEW, |w| w.process_actor())
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_1ed0,
        low_process::process_summon_creature_defend(Ptr, u32)
    ))
    .in_world(&FEW, |w| w.process_actor())
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_2960,
        low_process::process_patrol_start_at_ref_behavior(Ptr, u32, Ptr)
    ))
    .in_world(&MANY, |w| {
        let mut words = w.process_and(1);
        words.push(w.objects[7]);
        words
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_dc10,
        low_process::enter_combat(
            Ptr, u32, u32, u32, u32, u32, u8, u32, u8, u32, u8, u32, u8, u32
        ) -> bool
    ))
    .in_world(&FEW, |w| {
        let mut words = w.process_actor();
        for _ in 0..12 {
            words.push(w.word());
        }
        words
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x0090_8e90, low_process::process_use_item_at(Ptr, Ptr) -> bool))
        .in_world(&MOST, |w| w.process_actor())
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0090_9d00,
        low_process::process_flee_non_combat(Ptr, Ptr)
    ))
    .in_world(&MOST, |w| w.process_actor())
    .run()
    .assert_same();
    Diff::new(entry!(0x0091_22e0, low_process::process_patrol_init(Ptr, Ptr, Ptr) -> bool))
        .in_world(&MANY, |w| {
            let mut words = w.process_actor();
            words.push(w.objects[7]);
            words
        })
        .run()
        .assert_same();
}

#[test]
fn targets_and_locations() {
    Diff::new(entry!(0x0091_1e80, low_process::set_target(Ptr, Ptr)))
        .in_world(&MANY, |w| {
            let target = w.one_of(&[0, TARGET, ACTOR]);
            vec![w.process(), target]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0090_8bf0, low_process::set_target_for_package(Ptr, Ptr) -> bool))
        .in_world(&MANY, |w| w.process_actor())
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0090_8c50,
        low_process::set_location_for_package(Ptr, Ptr, u8) -> u32
    ))
    .in_world(&MANY, |w| {
        let mut words = w.process_actor();
        let second = w.pick(&[0, 1, 2, 0x100]);
        words.push(second);
        words
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_a130,
        low_process::current_package_location(Ptr, u32, u32) -> u32
    ))
    .in_world(&MANY, |w| {
        let out = w.objects[9];
        let actor = w.one_of(&[0, ACTOR]);
        vec![w.process(), out, actor]
    })
    .run()
    .assert_same();
}

#[test]
fn damage_modifiers() {
    Diff::new(entry!(0x0090_74f0, low_process::damage_modifier(Ptr, u8) -> f64))
        .in_world(&FEW, |w| {
            let code = w.pick(&[0, 0x10, 0x110]);
            vec![w.process(), code]
        })
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0090_7460,
        low_process::damage_mod_actor_value(Ptr, u32, u32, f32)
    ))
    .in_world(&MOST, |w| {
        let actor = w.one_of(&[0, ACTOR]);
        let code = w.pick(&[0x10, 0x10, 0x11, 0x110]);
        let amount = w.pick(FLOATS);
        vec![w.process(), actor, code, amount]
    })
    .run()
    .assert_same();
}

#[test]
fn packages() {
    Diff::new(entry!(0x0090_cfa0, low_process::is_wandering(Ptr) -> bool))
        .in_world(&MANY, |w| w.process_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(0x0091_3460, low_process::is_package_done(Ptr, u32) -> bool))
        .in_world(&MANY, |w| w.process_and(1))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0090_78d0,
        low_process::should_use_combat_controller_for_package(Ptr, Ptr, u8) -> bool
    ))
    .in_world(&MOST, |w| {
        let package = w.one_of(&[PACKAGE, PACKAGE_B]);
        let flag = w.pick(&[0, 1, 2, 0x100]);
        vec![w.process(), package, flag]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_7960,
        low_process::should_stop_combat_controller_to_finish_package(Ptr, u32, u32) -> bool
    ))
    .in_world(&MOST, |w| {
        let package = w.one_of(&[PACKAGE, PACKAGE_B]);
        let index = w.pick(&[0xffff_ffff, 0, 1, 2, 3]);
        vec![w.process(), package, index]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_7840,
        low_process::check_and_start_combat_controller_for_package(Ptr, Ptr, Ptr, u8) -> bool
    ))
    .in_world(&MOST, |w| {
        let mut words = w.process_actor_package();
        let flag = w.pick(&[0, 1, 2]);
        words.push(flag);
        words
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_35a0,
        low_process::change_package(Ptr, Ptr, Ptr)
    ))
    .in_world(&MOST, |w| w.process_actor_package())
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_3510,
        low_process::start_new_package(Ptr, Ptr, Ptr)
    ))
    .in_world(&MOST, |w| w.process_actor_package())
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_79f0,
        low_process::update_package_target_for_combat(Ptr, u32, u32) -> bool
    ))
    .in_world(&MOST, |w| w.process_actor_package())
    .run()
    .assert_same();
}

#[test]
fn doors() {
    Diff::new(entry!(0x0090_d570, low_process::unlock_door(Ptr, Ptr) -> bool))
        .in_world(&MOST, |w| w.door_actor())
        .run()
        .assert_same();
    Diff::new(entry!(0x0090_d480, low_process::lock_door(Ptr, Ptr) -> bool))
        .in_world(&MOST, |w| w.door_actor())
        .run()
        .assert_same();
}

#[test]
fn targets_greetings_and_schedules() {
    Diff::new(entry!(0x0090_a070, low_process::reset_target(Ptr)))
        .in_world(&MOST, |w| w.process_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0091_3630,
        low_process::package_done(Ptr, Ptr, Ptr)
    ))
    .in_world(&MANY, |w| w.process_actor_package())
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_36b0,
        low_process::process_greet(Ptr, Ptr, Ptr, u32, u32, u32, u8)
    ))
    .in_world(&MOST, |w| {
        let mut words = w.process_actor();
        words.push(w.objects[9]);
        for _ in 0..3 {
            words.push(w.word());
        }
        let say_to = w.pick(&[0, 1, 2, 0x100]);
        words.push(say_to);
        words
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_19b0,
        low_process::add_to_procedure_index_running(Ptr, Ptr, u32)
    ))
    .in_world(&MOST, |w| {
        let mut words = w.process_actor();
        let delta = w.pick(&[0, 1, 2, 0xffff_ffff, 0x8000_0000]);
        words.push(delta);
        words
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_7700,
        low_process::set_current_package_from_schedule(Ptr, Ptr) -> bool
    ))
    .in_world(&MOST, |w| w.process_actor())
    // The exe's `SetCurrentPackage` in slot 0x230 reaches `ActorPackage::InitFromPackage`, whose
    // translation calls `Clear` and `TESPackage::IncRefCount` as plain Rust calls.
    .translation_calls_plainly(&[0x008c_5ef0, 0x008c_6100])
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_c360,
        low_process::update_buffer(Ptr, Ptr, f32)
    ))
    .in_world(&MANY, |w| {
        let actor = w.one_of(&[0, ACTOR, TARGET]);
        let time = w.pick(&[
            0x4561_0000,
            0x4561_0001,
            0x4580_0000,
            0,
            0x7fc0_0000,
            0xc561_0000,
        ]);
        vec![w.process(), actor, time]
    })
    .run()
    .assert_same();
}

#[test]
fn patrols_copies_and_clears() {
    Diff::new(entry!(
        0x0091_0b30,
        low_process::clear_current_data_for_process(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.process_and(1))
    .run()
    .assert_same();
    // The waits' differences of two floats: SSE keeps the first NaN's payload, x87 the larger one's.
    Diff::new(entry!(0x0091_21e0, low_process::process_patrol(Ptr, Ptr)))
        .in_world(&MOST, |w| w.process_actor())
        .allow_nan_payloads()
        .stack_bytes(REQUEST_BYTES)
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0091_2990,
        low_process::process_patrol_at_ref(Ptr, Ptr, Ptr, f32) -> f32
    ))
    .in_world(&MOST, |w| {
        let mut words = w.process_actor();
        words.push(w.objects[7]);
        let time = w.pick(FLOATS);
        words.push(time);
        words
    })
    .allow_nan_payloads()
    .stack_bytes(REQUEST_BYTES)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0091_2340,
        low_process::process_patrol_start_travel_to_next_ref(Ptr, Ptr, Ptr)
    ))
    .in_world(&MOST, |w| {
        let mut words = w.process_actor();
        words.push(w.objects[7]);
        words
    })
    .stack_bytes(REQUEST_BYTES)
    .run()
    .assert_same();
    Diff::new(entry!(0x0090_7510, low_process::copy_level(Ptr, Ptr)))
        .in_world(&MANY, |w| vec![w.process(), w.objects[5]])
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0090_e600,
        low_process::objects_to_acquire_construct(Ptr) -> Ptr
    ))
    .in_world(&FEW, |w| vec![w.objects[9]])
    .run()
    .assert_same();
}

/// The bytes of the travel's `PathingRequest` on the stack (0xb4): the Diffs that reach it compare all
/// of them behind the stack word `SetPathfindingGoal` gets, so its stores at +0x8c, +0x99 and +0x9c
/// show (64 bytes are compared by default).
const REQUEST_BYTES: u32 = 0xb4;

/// Runs the translation at `addr` in each seed's world and counts the seeds whose `check` says each
/// branch was taken; every branch must be taken by one. `seen` is the answer, the call's words, then
/// the process's `pTarget`, `fTrackedDamage` and the patrol data's time waited (the record's +8) as
/// they were before the call.
fn reaches(
    addr: u32,
    seeds: &[u32],
    words: fn(&mut World) -> Vec<u32>,
    branches: &[&str],
    check: fn(&Engine, &[u32]) -> Vec<bool>,
) {
    let mut taken = vec![0u32; branches.len()];
    for &seed in seeds {
        let mut e = Engine::new_untyped_for_test();
        let mut w = World::new(&mut e, seed);
        let args = words(&mut w);
        let before =
            [w.process() + 0x40, w.process() + 0xac, w.objects[7] + 8].map(|at| e.mem.u32(at));
        let answer = e.call(addr, &args).eax;
        let mut seen = vec![answer];
        seen.extend_from_slice(&args);
        seen.extend_from_slice(&before);
        for (n, hit) in taken.iter_mut().zip(check(&e, &seen)) {
            *n += u32::from(hit);
        }
    }
    for (name, n) in branches.iter().zip(&taken) {
        assert!(*n > 0, "{addr:08x}: no seed reaches \"{name}\": {taken:?}");
    }
}

/// The worlds the seeds make reach the deep branches, checked on the translations (so a change to
/// the world that loses one shows here, not as a weaker comparison). `seen` is the answer, then the
/// call's words.
#[test]
fn the_seeds_reach_every_branch() {
    reaches(
        0x0091_34c0,
        &MOST,
        |w| {
            let actor = w.one_of(&[0, ACTOR, TARGET]);
            let amount = w.pick(FLOATS);
            vec![w.process(), actor, amount]
        },
        &[
            "no actor",
            "slot no, byte no: unchanged",
            "added: the sum stored",
        ],
        |e, seen| {
            let actor = seen[2];
            let before = seen[5];
            let after = e.mem.u32(seen[1] + 0xac);
            // The actor's slot 0x360 is always a stand-in here: its answer's low byte.
            let adds = actor != 0
                && (e.mem.u8(e.mem.u32(actor) + ANSWERS + 0x360) != 0
                    || e.mem.u8(actor + 0x18d) != 0);
            let sum =
                (f64::from(f32::from_bits(before)) + f64::from(f32::from_bits(seen[3]))) as f32;
            let stored = after == sum.to_bits() || (sum.is_nan() && f32::from_bits(after).is_nan());
            vec![
                actor == 0,
                actor != 0 && !adds && after == before,
                adds && stored,
            ]
        },
    );
    reaches(
        0x0091_2150,
        &MOST,
        |w| w.process_actor(),
        &["moves on", "stops", "moves on: exactly at the limit"],
        |e, _| {
            let elapsed = f64::from(f32::from_bits(last_answer(e, 0x0040_8840)));
            let at_limit = elapsed == e.mem.f64(BLOCKED_TIME_LIMIT);
            vec![
                calls(e, 0x0067_4d70) > 0,
                calls(e, 0x008b_3ab0) > 0,
                at_limit && calls(e, 0x0067_4d70) > 0,
            ]
        },
    );
    reaches(
        0x0090_8e90,
        &MOST,
        |w| w.process_actor(),
        &["back", "on", "neither"],
        |e, seen| {
            // The exe's body in the slot (about half the seeds) moves the index without the stand-in
            // counting: only the stand-in's seeds count here.
            let stand_in = e.mem.u32(e.mem.u32(seen[1]) + 0x288) == VT_FN + 0x288;
            let moved = slot_calls(e, 0x288) > 0;
            vec![
                moved && slot_last(e, 0x288) == 0xffff_ffff,
                moved && slot_last(e, 0x288) == 1,
                stand_in && !moved,
            ]
        },
    );
    reaches(
        0x0090_9d00,
        &MOST,
        |w| w.process_actor(),
        &[
            "measured",
            "no target",
            "measured: the target the package's slots set",
        ],
        |e, seen| {
            vec![
                calls(e, 0x0057_23b0) > 0,
                calls(e, 0x0057_23b0) == 0 && slot_calls(e, 0x13c) > 0,
                seen[3] == 0 && e.mem.u32(seen[1] + 0x40) != 0 && calls(e, 0x0057_23b0) > 0,
            ]
        },
    );
    reaches(
        0x0091_22e0,
        &MANY,
        |w| {
            let mut words = w.process_actor();
            words.push(w.objects[7]);
            words
        },
        &["fails", "travels"],
        |e, _| vec![calls(e, 0x009f_3390) == 0, calls(e, 0x009f_3390) > 0],
    );
    reaches(
        0x0090_78d0,
        &MOST,
        |w| {
            let package = w.one_of(&[PACKAGE, PACKAGE_B]);
            let flag = w.pick(&[0, 1, 2, 0x100]);
            vec![w.process(), package, flag]
        },
        &["used", "not used"],
        |_, seen| vec![seen[0] & 0xff != 0, seen[0] & 0xff == 0],
    );
    reaches(
        0x0090_7960,
        &MOST,
        |w| {
            let package = w.one_of(&[PACKAGE, PACKAGE_B]);
            let index = w.pick(&[0xffff_ffff, 0, 1, 2, 3]);
            vec![w.process(), package, index]
        },
        &["stops", "goes on"],
        |_, seen| vec![seen[0] & 0xff != 0, seen[0] & 0xff == 0],
    );
    reaches(
        0x0090_7840,
        &MOST,
        |w| {
            let mut words = w.process_actor_package();
            let flag = w.pick(&[0, 1, 2]);
            words.push(flag);
            words
        },
        &["combat"],
        |_, seen| vec![seen[0] & 0xff != 0],
    );
    reaches(
        0x0091_35a0,
        &MOST,
        |w| w.process_actor_package(),
        &["script", "topic"],
        |e, _| vec![calls(e, 0x005a_c1e0) > 0, slot_calls(e, 0x2a4) > 0],
    );
    reaches(
        0x0090_79f0,
        &MOST,
        |w| w.process_actor_package(),
        &["started", "next", "failed"],
        |e, seen| {
            vec![
                calls(e, 0x009f_2ba0) > 0 && calls(e, 0x0041_cab0) > 0,
                calls(e, 0x009f_2ea0) > 0 && calls(e, 0x0041_cab0) > 0,
                calls(e, 0x0042_12b0) > 0 && seen[0] & 0xff == 0,
            ]
        },
    );
    reaches(
        0x0090_d570,
        &MOST,
        |w| w.door_actor(),
        &["unlocked", "public"],
        |e, _| vec![calls(e, 0x0056_92a0) > 0, calls(e, 0x0043_a210) > 0],
    );
    reaches(
        0x0090_d480,
        &MOST,
        |w| w.door_actor(),
        &["world space", "cell", "locked", "private"],
        |e, _| {
            vec![
                calls(e, 0x0043_a320) > 0,
                calls(e, 0x0057_85e0) > 0 && calls(e, 0x0043_a320) > 0,
                calls(e, 0x0056_9250) > 0,
                calls(e, 0x0043_a210) > 0,
            ]
        },
    );
    reaches(
        0x0090_a070,
        &MOST,
        |w| w.process_and(0),
        &["kept", "set"],
        |e, seen| {
            let set = slot_calls(e, 0x12c) > 0 || e.mem.u32(seen[1] + 0x40) != seen[2];
            vec![!set, set]
        },
    );
    reaches(
        0x0091_3630,
        &MANY,
        |w| w.process_actor_package(),
        &["no package", "script", "topic"],
        |e, seen| {
            vec![
                seen[3] == 0 && calls(e, 0x005a_c1e0) == 0 && slot_calls(e, 0x2a4) == 0,
                calls(e, 0x005a_c1e0) > 0,
                slot_calls(e, 0x2a4) > 0,
            ]
        },
    );
    reaches(
        0x0091_36b0,
        &MOST,
        |w| {
            let mut words = w.process_actor();
            words.push(w.objects[9]);
            words.extend([0, 0, 0, 1]);
            words
        },
        &["no dialogue", "result run", "said"],
        |e, _| {
            vec![
                calls(e, 0x0061_b320) == 0,
                calls(e, 0x0083_c850) > 0,
                calls(e, 0x0093_6a20) > 0,
            ]
        },
    );
    reaches(
        0x0091_19b0,
        &MOST,
        |w| {
            let mut words = w.process_actor();
            let delta = w.pick(&[0, 1, 2, 0xffff_ffff, 0x8000_0000]);
            words.push(delta);
            words
        },
        &["not done", "done: the action flag"],
        |e, _| vec![calls(e, 0x005a_c750) == 0, calls(e, 0x005a_c750) > 0],
    );
    reaches(
        0x0090_7700,
        &MOST,
        |w| w.process_actor(),
        &[
            "the same package",
            "start location",
            "combat controller",
            "the exe's SetCurrentPackage",
        ],
        |e, seen| {
            vec![
                seen[0] & 0xff == 0,
                calls(e, 0x0056_ae10) > 0,
                calls(e, 0x0098_0ae0) > 0,
                calls(e, 0x0067_59e0) > 0,
            ]
        },
    );
    reaches(
        0x0090_c360,
        &MANY,
        |w| {
            let actor = w.one_of(&[0, ACTOR, TARGET]);
            let time = w.pick(&[0x4561_0000, 0x4580_0000, 0]);
            vec![w.process(), actor, time]
        },
        &["missed packages", "not an actor"],
        |e, _| {
            vec![
                slot_calls(e, 0x14) > 0,
                calls(e, 0x00ec_43fb) > 0 && slot_calls(e, 0x25c) == 0,
            ]
        },
    );
    reaches(
        0x0091_0b30,
        &MANY,
        |w| w.process_and(1),
        &["the saved object taken off", "the list's items deleted"],
        |e, _| vec![calls(e, 0x0090_5330) > 0, calls(e, 0x007b_3fa0) > 1],
    );
    reaches(
        0x0091_21e0,
        &MOST,
        |w| w.process_actor(),
        &[
            "no data",
            "the actor's slot",
            "init",
            "travel step",
            "at the reference",
        ],
        |e, _| {
            vec![
                calls(e, 0x0090_7260) == 0 && slot_calls(e, 0x418) == 0,
                slot_calls(e, 0x418) > 0,
                calls(e, 0x009f_2ba0) > 0,
                calls(e, 0x0091_25b0) > 0,
                calls(e, 0x0041_fe90) > 0,
            ]
        },
    );
    reaches(
        0x0091_2990,
        &MOST,
        |w| {
            let mut words = w.process_actor();
            words.push(w.objects[7]);
            let time = w.pick(FLOATS);
            words.push(time);
            words
        },
        &["travels on", "the next one failed", "waits"],
        |e, seen| {
            let next = calls(e, 0x009f_2ea0) > 0;
            let failed = next && last_answer(e, 0x009f_2ea0) & 0xff == 0;
            let waited = e.mem.u32(seen[3] + 8) != seen[7];
            vec![
                calls(e, 0x0094_df60) > 0,
                failed && calls(e, 0x0094_df60) == 0,
                calls(e, 0x009f_3390) > 0 && !next && waited,
            ]
        },
    );
    reaches(
        0x0091_2340,
        &MOST,
        |w| {
            let mut words = w.process_actor();
            words.push(w.objects[7]);
            words
        },
        &["skipped", "the request built", "with an angle"],
        |e, _| {
            vec![
                calls(e, 0x0094_df60) > 0 && calls(e, 0x008b_3880) == 0,
                calls(e, 0x008b_3880) > 0,
                calls(e, 0x008b_3880) > 0 && {
                    let reference = last_answer(e, 0x009f_3390);
                    let base = e.mem.u32(reference + 0x20);
                    e.mem.u8(base + 4) == 0x30 || base == e.mem.u32(ANGLED_FORM)
                },
            ]
        },
    );
}
