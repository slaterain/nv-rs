//! The differential test of every typed function of the AI family's `MiddleLowProcess` module
//! (`engine_typed::ai::middle_low_process`) against its translation, in one world built from a
//! seed.
//!
//! The world: eight objects of 0x400 bytes (the process under test, the actor, a target, two
//! packages, the save/load object, another process and a form that serves as a record, a buffer and
//! a global), each word a seeded small integer, float bit pattern or pointer to one of them. The
//! fields the code follows are seeded from the values that choose its branches: the process's
//! embedded package, procedure index, level, flags byte, target, made objects, created object and
//! last package hour; each actor's form flags, form, point, parent cell and process; each package's
//! type, flags, procedure, location and data; the save/load object's position, version and records;
//! the form's id, change flags and value. Each object has a vtable of its own: every slot the code
//! calls is a stand-in answering a seeded word (in EAX, and as a float in ST0) and counting its
//! calls, or, by the seed, the exe's own body (the process classes' slots 0x22c, 0x24, 0x280, 0x284,
//! 0x238, 0x23c, 0x240, 0x3a8 and `LowProcess`'s flag getters and setters at 0x1bc, 0x1c0, 0x1c8,
//! 0x5a4 and 0x5a8; a form's name at 0x130), so the typed side's direct branches run. The globals
//! the code reads (the actor value infos, the calendar, the settings, the procedure action table,
//! the procedure names, the player, the float and double constants, the save/load game and the SSE2
//! flag `_ftol2` reads) are seeded too; the page at 0 is mapped, so a read through a null pointer
//! reads zeros on both sides.
//!
//! What stands in: every callee the code calls by address. Each counts its calls, keeps its last
//! two words and answers the next of eight seeded words; the readers also write a word into the
//! bytes they read, the save/load game's readers and writers move its position on as the exe's do,
//! and the list's remove empties the list. What runs as itself: the family's own functions, those of
//! the families its typed calls reach (the leaves, the AI queries, the CRT's `_ftol2`, the
//! multibound family's world space getter) and the translations of the callees the typed code does
//! in place. [`the_seeds_reach_every_branch`] checks, on the translations, that the seeds take each
//! deep branch.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_typed::ai::{middle_low_process, AI};
use engine_typed::ai_queries::AI_QUERIES;
use engine_typed::crt::CRT;
use engine_typed::multibound::MULTIBOUND;

/// Where the stand-in slots are registered: `VT_FN + slot`.
const VT_FN: u32 = 0x00ff_4000;
/// A vtable's stand-in answers sit this far past it, at the same slot offset.
const ANSWERS: u32 = 0x1000;
/// The doubles' counts, last words and answers: double `k` at `CONTROL + 0x40 * k`.
const CONTROL: u32 = 0x0300_0000;
/// The slots' call counts: slot `s` at `SLOT_COUNTS + s`.
const SLOT_COUNTS: u32 = CONTROL + 0x2000;
/// How far the modifier lists' save and load move the save/load game's position on.
const EXTRA: u32 = CONTROL + 0x1ff0;
/// The procedure action table's rows.
const ACTIONS: u32 = CONTROL + 0x3000;
/// The save buffer the positions point into.
const BUFFER: u32 = 0x0400_0000;

/// The slots the code calls, each a stand-in unless the seed gives the exe's body.
const SLOTS: &[u32] = &[
    0x24, 0x28, 0x8c, 0x90, 0xc8, 0x100, 0x130, 0x13c, 0x140, 0x16c, 0x1bc, 0x1c0, 0x1c8, 0x1f4,
    0x22c, 0x238, 0x23c, 0x240, 0x280, 0x284, 0x288, 0x28c, 0x298, 0x29c, 0x2a0, 0x2c4, 0x3a8,
    0x434, 0x438, 0x514, 0x524, 0x5a0, 0x5a4, 0x5a8, 0x7b4, 0x7c4, 0x7c8, 0x7cc, 0x7d0, 0x7d4,
    0x7d8, 0x7dc, 0x7e0, 0x7e4, 0x7e8, 0x7ec, 0x7f0, 0x7f4, 0x7fc, 0x800, 0x808,
];
/// Slots `LowProcess`'s typed functions call (`ResetTarget`'s 0x12c, `Update_ov2`'s 0x14 and the
/// actor's 0x25c): stand-ins that count and answer 0, drawing nothing from the seed.
const QUIET_SLOTS: &[u32] = &[0x14, 0x12c, 0x25c];
/// The exe's bodies a slot may hold instead.
const REAL_SLOTS: &[(u32, u32)] = &[
    (0x24, 0x0092_cfd0),
    (0x130, 0x0040_1280),
    (0x1bc, 0x008d_8060),
    (0x1c0, 0x008d_8080),
    (0x1c8, 0x008d_7ee0),
    (0x22c, 0x007f_a950),
    (0x238, 0x008d_03c0),
    (0x23c, 0x008d_03e0),
    (0x240, 0x008d_0400),
    (0x280, 0x008d_0450),
    (0x284, 0x008d_0470),
    (0x3a8, 0x0092_cf30),
    (0x5a4, 0x008d_8000),
    (0x5a8, 0x008d_8020),
];

/// A word that stands for object `n` of the world (`OBJECT + n`), resolved when the world is made.
const OBJECT: u32 = 0xb0b0_0000;
const PROCESS: u32 = OBJECT;
const ACTOR: u32 = OBJECT + 1;
const TARGET: u32 = OBJECT + 2;
const PACKAGE: u32 = OBJECT + 3;
const PACKAGE_B: u32 = OBJECT + 4;
const SAVE_LOAD: u32 = OBJECT + 5;
const OTHER: u32 = OBJECT + 6;
const FORM: u32 = OBJECT + 7;
const RECORD: u32 = OBJECT + 8;
const LOCATION: u32 = OBJECT + 9;
const COUNT: usize = 10;
/// The objects with a vtable (the record and the location start with their id and kind).
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
/// Mostly true byte answers (0x100: a high byte that is false), so the travel seeds more often find
/// the path done and set its goal.
const DONE: &[u32] = &[1, 1, 0xff, 0x100];
/// The block tag, twice as likely as each other word a block read may give.
const TAG: u32 = 0x424c_4f4b;
/// The actions a procedure may give: every arm of `Update`'s switch, the finish and past the end.
const ACTION_WORDS: &[u32] = &[
    0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0xa, 0xc, 0xd, 0xd, 0xd, 0xe, 0xf, 0x11, 0x11, 0x1a, 0x1b,
    0xd, 0xd, 0xd, 0x1c, 0x1e, 0x21, 0x22, 0x26, 0x27, 0x29, 0x2a, 0x2c, 0x2f, 0x31, 0x31, 0x32,
    0x33, 0x34, 0x36, 0x36, 0x36, 0x36, 0, 0, 0, 0x36, 0x36, 0x36, 0x36,
];

/// What a double does besides answering.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Answers only.
    Plain,
    /// A function the family types now: its answers are still drawn (so the rest of the world stays
    /// as it was), but it runs as itself.
    Typed,
    /// `thiscall(save_load, address, size)`: moves the position (+0x14) on by the size.
    Write,
    /// `thiscall(object, address, size)`: writes its answer into the bytes (up to 4) and, on the
    /// save/load game (`ReadMoves`), moves the position on by the size; answers the next word.
    Read,
    ReadMoves,
    /// `thiscall(list, slot)`: empties the list (both its words), so the loop ends.
    Remove,
    /// Moves the save/load game's position on by the world's extra (what a list writes or reads).
    Moves,
}

/// The callees that stand in, with the words each may answer and what else it does.
const DOUBLES: &[(u32, &[u32], Kind)] = &[
    (0x0090_7380, FLOATS, Kind::Plain), // LowProcess::GetActorFloatValue
    (0x0090_72d0, &[0, 7, 0xffff_fff0, 0x7fff_ffff], Kind::Plain), // LowProcess::GetActorValue
    (0x0093_7730, FLOATS, Kind::Plain), // ModifierList::GetModifier
    (0x0093_7480, UNREAD, Kind::Plain), // ModifierList::AdjustModifier
    (0x0093_7b50, UNREAD, Kind::Plain), // ModifierList::SaveGame_ov2
    (0x0093_7160, UNREAD, Kind::Plain), // the modifier list's copy
    (0x0093_7030, UNREAD, Kind::Plain), // its destructor
    (0x0091_0450, UNREAD, Kind::Plain), // LowProcess::SaveGame_ov2
    (0x0086_5e50, UNREAD, Kind::Plain), // the save buffer's writer
    (0x0090_6fe0, UNREAD, Kind::Plain), // LowProcess's destructor
    (0x0090_a1a0, BOOLS, Kind::Plain),  // LowProcess::CheckforNewPackage
    (0x0090_c360, UNREAD, Kind::Typed), // LowProcess::Update_ov2
    (0x0040_1030, UNREAD, Kind::Plain), // the free
    (0x0066_eea0, FLOATS, Kind::Plain), // ActorValue::CheckClampDamageModifier
    (0x00ec_43fb, &[0, TARGET, ACTOR, TARGET], Kind::Plain), // __RTDynamicCast
    (0x0088_1680, UNREAD, Kind::Plain), // Actor::EndInterruptPackage
    (0x0090_f8b0, &[0, 6, 0xfff0, 0xffff], Kind::Plain), // LowProcess::GetSaveSize
    (0x0090_fa70, UNREAD, Kind::Plain), // LowProcess::SaveGame
    (0x0090_fd80, UNREAD, Kind::Plain), // LowProcess::LoadGame
    (0x0091_0310, UNREAD, Kind::Plain), // LowProcess::Revert
    (0x0091_0630, UNREAD, Kind::Plain), // LowProcess::LoadGame_ov2
    (0x0091_0a60, UNREAD, Kind::Plain), // LowProcess::Revert_ov2
    (0x0093_7ab0, &[0, 3, 0xfffe], Kind::Plain), // ModifierList::GetSaveSize
    (0x0093_78d0, UNREAD, Kind::Moves), // ModifierList::SaveGame
    (0x0093_79c0, UNREAD, Kind::Moves), // ModifierList::LoadGame
    (0x0093_7c60, UNREAD, Kind::Plain), // ModifierList::LoadGame_ov2
    (0x0093_7b30, UNREAD, Kind::Plain), // the revert forwarder
    (0x0093_7d50, UNREAD, Kind::Plain), // the buffer revert
    (0x0086_4980, &[0, 1, 0xe, 0x7f80_0001], Kind::Read), // the load buffer's reader
    (0x0085_79b0, UNREAD, Kind::Write), // the save/load game's writer
    (
        0x0085_79e0,
        &[TAG, TAG, 0, 6, 0x0e, 0x12, 0x10, 0xfff0],
        Kind::ReadMoves,
    ), // its reader
    (0x0048_39c0, &[FORM, PACKAGE], Kind::Plain), // the form for an id
    (0x005b_5e40, UNREAD, Kind::Plain), // the SAVELOAD warnings
    (0x008a_8f60, UNREAD, Kind::Plain), // the actor's package re-evaluation
    (0x0090_a070, UNREAD, Kind::Typed), // LowProcess::ResetTarget
    (0x008b_3bb0, DONE, Kind::Plain),   // Actor::IsPathingComplete
    (0x008b_3bf0, BOOLS, Kind::Plain),  // Actor::IsWaitingOnPath
    (0x0067_8670, FLOATS, Kind::Plain), // TESPackage::GetPackageRadiusActorToLocation
    (0x0067_5a50, &[0, FORM], Kind::Plain), // TESPackage::GetLocationWorld
    (0x0067_5c20, &[0, FORM], Kind::Plain), // TESPackage::GetLocationCell
    (0x0067_5de0, &[0, FORM], Kind::Plain), // TESPackage::GetLocationCoord
    (0x008b_3690, UNREAD, Kind::Plain), // Actor::SetPathfindingGoal_ov2
    (0x0046_f280, UNREAD, Kind::Plain), // TESDataHandler::EnumReferencesCloseToPoint
    (0x0067_2dd0, &[0, FORM], Kind::Plain), // the package's use object
    (0x0096_2720, BOOLS, Kind::Plain),  // the player's follower test
    (0x0096_2620, &[0, 2, 9], Kind::Plain), // the player's follower count
    (0x0067_4f30, UNREAD, Kind::Plain), // the package's done flag
    (0x0057_23b0, FLOATS, Kind::Plain), // TESObjectREFR::GetDistanceFromReference
    (0x005a_c750, UNREAD, Kind::Plain), // Script::SetActionFlag
    (0x007b_3fa0, UNREAD, Kind::Plain), // the object's delete
    (0x0090_5330, UNREAD, Kind::Remove), // the list's remove
    (0x0055_d520, &[0x1234, FORM], Kind::Plain), // the actor's name
    (0x00ec_623a, UNREAD, Kind::Plain), // sprintf
    (0x0070_3180, &[0, ACTOR, ACTOR], Kind::Plain), // Interface::GetTargetREFR
    (0x0070_3c00, UNREAD, Kind::Plain), // the console print
];

/// The callees the translations call and the typed code does in place (one load, store or test
/// each), or leaves out (a pure call whose answer the exe never reads).
const IN_PLACE: &[u32] = &[
    0x0068_0020, // the package target's reference and its kind (ResetTarget)
    0x0051_9b00,
    0x0040_6d70, // the actor value's flag
    0x0042_8110, // the buffer's change flags
    0x0042_80f0, // their test
    0x0042_ce30, // the buffer's revert flags
    0x0086_7da0, // Calendar::GetHour
    0x0052_6ac0, // a global's value
    0x0040_3e20, // a float setting's value
    0x0040_8d60, // a byte setting's value
    0x0043_d4d0, // an integer setting's value
    0x0086_2110, // the save game blocks test
    0x0082_5c00, // the position
    0x004f_d3c0, // the record being loaded
    0x008d_f040, // the version
    0x0096_11e0, // the package's type
    0x008a_6290, // its waiting flag
    0x0055_b980, // its location
    0x0041_ca90, // its procedure
    0x0067_0f40, // its flags
    0x0067_0f60,
    0x0067_0f90,
    0x0067_4d40,
    0x0067_8ca0, // the location's kind
    0x0043_0830, // the reference's point
    0x0041_d8a0, // the package's data
    0x0072_6070, // the leaves
    0x0082_56d0,
    0x0068_15c0,
    0x008d_8520,
    0x0045_cd60,
    0x007a_f430,
    0x0044_0d80,
    0x0044_0da0,
    0x0067_1d10,
    0x0044_ddc0,
    0x008d_6f30, // the parent cell
    0x0046_4f30, // the procedure's name
    0x005d_43c0, // the extra data list
    0x0093_1850, // the process type
    0x0042_5fd0, // the interior test
    0x0048_3710, // the empty body
    0x008d_8060, // the process flags' getters and setters
    0x008d_8080,
    0x008d_7ee0,
    0x008d_8000,
    0x008d_8020,
];

/// The globals the code reads.
const AV_INFOS: u32 = 0x011d_61c8;
const CALENDAR: u32 = 0x011d_e7b8;
const DEFAULT_HOUR: u32 = 0x0101_8f7c;
const DEFAULT_DAY: u32 = 0x0101_8f8c;
const ROUNDS_SETTING: u32 = 0x011c_d6d0;
const INTERIOR_REACH: u32 = 0x011c_dbb0;
const REACH: u32 = 0x011c_d830;
const FOLLOWER_LIMIT: u32 = 0x011c_dad4;
const DATA_HANDLER: u32 = 0x011c_3f2c;
const TARGET_FORM: u32 = 0x011c_a248;
const PLAYER: u32 = 0x011d_ea3c;
const PROCEDURE_NAMES: u32 = 0x0119_bcb0;
const ACTION_TABLE: u32 = 0x011a_3ff0;
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
const ZERO_DOUBLE: u32 = 0x0101_2060;
const TWO_DOUBLE: u32 = 0x0101_1590;
const ONE_HOUR_FACTOR: u32 = 0x0101_db80;
const CLOSE_RADIUS: u32 = 0x0103_0020;
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const SAVE_LOG_BYTE: u32 = 0x011d_e4ec;
const SSE2_AVAILABLE: u32 = 0x0127_0a6c;

/// The seeds: a few for the functions with no branch, many for the rest, the most for `Update`.
static FEW: [u32; 8] = seeds();
static MANY: [u32; 64] = seeds();
static MOST: [u32; 192] = seeds();
static ALL: [u32; 768] = seeds();

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

/// The index of the double at `addr`.
fn double(addr: u32) -> usize {
    DOUBLES
        .iter()
        .position(|(a, _, _)| *a == addr)
        .expect("a double")
}

/// How many times the double at `addr` was called.
fn calls(e: &Engine, addr: u32) -> u32 {
    e.mem.u32(block(double(addr)))
}

/// The first word of the last call of the double at `addr`.
fn last_word(e: &Engine, addr: u32) -> u32 {
    e.mem.u32(block(double(addr)) + 0x24)
}

fn slot_calls(e: &Engine, slot: u32) -> u32 {
    e.mem.u32(SLOT_COUNTS + slot)
}

/// The world of one seed, made the same on both engines.
struct World {
    objects: [u32; COUNT],
    rng: u64,
}

impl World {
    fn new(e: &mut Engine, seed: u32) -> World {
        let mut w = World {
            objects: [0; COUNT],
            rng: (u64::from(seed) ^ 0x9e37_79b9_7f4a_7c15).wrapping_mul(0xbf58_476d_1ce4_e5b9) | 1,
        };
        e.map(0, 0x1000);
        // A package target's reference that is a float word or 0xffff (`ResetTarget` reads its flags):
        // zeros.
        e.map(0x3f80_0000, 0x1000);
        e.map(0x0001_0000, 0x1000);
        e.map(0x7f80_0000, 0x1000);
        e.map(CONTROL, 0x4000);
        e.map(BUFFER, 0x2_0000);
        // With the temporary modifiers: a middle-low block of exactly 0xffff bytes (6 more), one past.
        let extra = w.pick(&[0, 0, 4, 0xfff9, 0xfffa, 0x1_0000]);
        e.mem.set_u32(EXTRA, extra);
        for &slot in SLOTS {
            e.register_double(VT_FN + slot, move |e, a| {
                let n = e.mem.u32(SLOT_COUNTS + slot);
                e.mem.set_u32(SLOT_COUNTS + slot, n + 1);
                let vt = e.mem.u32(a[0]);
                ret(e.mem.u32(vt + ANSWERS + slot))
            });
        }
        for &slot in QUIET_SLOTS {
            e.register_double(VT_FN + slot, move |e, _| {
                let n = e.mem.u32(SLOT_COUNTS + slot);
                e.mem.set_u32(SLOT_COUNTS + slot, n + 1);
                ret(0)
            });
        }
        for (k, (addr, answers, kind)) in DOUBLES.iter().enumerate() {
            let block = block(k);
            for i in 0..8 {
                let answer = w.pick(answers);
                e.mem.set_u32(block + 4 + 4 * i, answer);
            }
            let kind = *kind;
            if matches!(kind, Kind::Typed) {
                continue;
            }
            e.register_double(*addr, move |e, a| {
                let n = e.mem.u32(block);
                e.mem.set_u32(block, n + 1);
                e.mem.set_u32(block + 0x24, a.first().copied().unwrap_or(0));
                e.mem.set_u32(block + 0x28, a.get(1).copied().unwrap_or(0));
                let answer = e.mem.u32(block + 4 + 4 * (n % 8));
                let next = e.mem.u32(block + 4 + 4 * ((n + 1) % 8));
                let (bytes, moves) = match kind {
                    Kind::Plain | Kind::Typed | Kind::Remove | Kind::Moves => (0, 0),
                    Kind::Write => (0, a[2]),
                    Kind::Read => (a[2].min(4), 0),
                    Kind::ReadMoves => (a[2].min(4), a[2]),
                };
                for i in 0..bytes {
                    e.mem.set_u8(a[1] + i, (answer >> (8 * i)) as u8);
                }
                if moves != 0 {
                    let position = a[0] + 0x14;
                    let moved = e.mem.u32(position).wrapping_add(moves);
                    e.mem.set_u32(position, moved);
                }
                if kind == Kind::Moves {
                    let position = e.mem.u32(SAVE_LOAD_GAME) + 0x14;
                    let moved = e.mem.u32(position).wrapping_add(e.mem.u32(EXTRA));
                    e.mem.set_u32(position, moved);
                }
                if kind == Kind::Remove {
                    e.mem.set_u32(a[0], 0);
                    e.mem.set_u32(a[0] + 4, 0);
                }
                match kind {
                    Kind::Read | Kind::ReadMoves => ret(next),
                    _ => ret(answer),
                }
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
        // Every object but the record and the location has a vtable, and so has the null object (the
        // word at 0), so a call through a null pointer the exe would make answers on both sides.
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
            for &slot in QUIET_SLOTS {
                e.mem.set_u32(vt + slot, VT_FN + slot);
            }
            for &slot in SLOTS {
                e.mem.set_u32(vt + slot, VT_FN + slot);
                let answers: &[u32] = match slot {
                    0x24 | 0x13c | 0x140 | 0x1bc | 0x5a4 => BOOLS,
                    // Mostly an actor, so `Update` goes on.
                    0x100 => &[0, 1, 1, 0xff],
                    0x22c => &[0, PACKAGE, PACKAGE, PACKAGE_B],
                    0x23c | 0x280 => &[0xffff_ffff, 0, 1, 2, 3],
                    0x130 => &[0x0101_1584, 0x1234],
                    0x16c | 0x514 => &[0, FORM, TARGET],
                    0x1f4 => &[0, 1, FORM],
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
        w
    }

    /// The fields that choose the branches.
    fn seed_fields(&mut self, e: &mut Engine) {
        // The processes: the embedded package and its procedure index, the level, the flags byte,
        // the target, the made objects, the created object and the last package hour.
        for o in [self.objects[0], self.objects[6]] {
            let package = self.one_of(&[0, PACKAGE, PACKAGE, PACKAGE_B]);
            e.mem.set_u32(o + 4, package);
            let index = self.pick(&[0xffff_ffff, 0, 1, 2, 3]);
            e.mem.set_u32(o + 0x10, index);
            let level = self.pick(&[0, 1, 2, 2, 2, 2, 3]);
            e.mem.set_u32(o + 0x28, level);
            let flags = self.pick(&[0, 0x80, 0x20, 0xa4, 0xff]);
            e.mem.set_u8(o + 0x30, flags as u8);
            let target = self.one_of(&[0, TARGET, TARGET, ACTOR, FORM]);
            e.mem.set_u32(o + 0x40, target);
            let head = self.one_of(&[0, 0, FORM, 1]);
            e.mem.set_u32(o + 0x5c, head);
            let next = self.pick(&[0, 0, 1]);
            e.mem.set_u32(o + 0x60, next);
            let created = self.one_of(&[0, FORM]);
            e.mem.set_u32(o + 0x64, created);
            let hour = self.pick(&[0, 12, 20, 0xffff_ffff, 0x8000_0000]);
            e.mem.set_u32(o + 0xb4, hour);
        }
        // The actors: form flags, form, point, parent cell, process.
        for o in [self.objects[1], self.objects[2]] {
            let flags = self.pick(&[0, 0, 0x20, 0x800, 0x400]);
            e.mem.set_u32(o + 8, flags);
            let form = self.pick(&[0, 0x1234, 0x1234]);
            e.mem.set_u32(o + 0x20, form);
            for i in 0..3 {
                let at = self.pick(FLOATS);
                e.mem.set_u32(o + 0x24 + 4 * i, at);
            }
            let cell = self.one_of(&[0, LOCATION, LOCATION]);
            e.mem.set_u32(o + 0x40, cell);
            let process = self.one_of(&[PROCESS, OTHER, OTHER]);
            e.mem.set_u32(o + 0x68, process);
        }
        // The packages: type, flags, procedure, location, data and the data's word.
        for o in [self.objects[3], self.objects[4]] {
            let kind = self.pick(&[0xffff_ffff, 0, 0, 1, 2, 0x1a, 0x1a]);
            e.mem.set_u32(o + 0x18, kind);
            let mut flags = 0;
            for bit in [0x20, 0x100, 0x400, 0x800, 0x1_0000] {
                if self.next() % 3 == 0 {
                    flags |= bit;
                }
            }
            e.mem.set_u32(o + 0x1c, flags);
            let procedure = self.pick(&[0, 1, 1, 5, 0x1c, 0xff, 0x0f]);
            e.mem.set_u8(o + 0x20, procedure as u8);
            let location = self.one_of(&[0, LOCATION, LOCATION]);
            e.mem.set_u32(o + 0x2c, location);
            let data = self.one_of(&[0, RECORD]);
            e.mem.set_u32(o + 0x30, data);
            let word = self.pick(&[0, 1]);
            e.mem.set_u32(o + 0x3c, word);
        }
        // The location: its kind byte and reference; also a cell (its interior byte).
        let l = self.objects[9];
        let kind = self.pick(&[0, 3, 3, 1]);
        e.mem.set_u8(l, kind as u8);
        let reference = self.one_of(&[0, TARGET]);
        e.mem.set_u32(l + 8, reference);
        let interior = self.pick(&[0, 1]);
        e.mem.set_u8(l + 0x24, interior as u8);
        // The save/load object: its position, version and the records being loaded and saved.
        let save_load = self.objects[5];
        let position = BUFFER + self.pick(&[0, 2, 0x10]);
        e.mem.set_u32(save_load + 0x14, position);
        let version = self.pick(&[0, 6, 0x1f, 0x20, 0x59, 0x5a]);
        e.mem.set_u8(save_load + 0x80, version as u8);
        let loading = self.one_of(&[0, RECORD]);
        e.mem.set_u32(save_load + 0x84, loading);
        let saving = self.one_of(&[0, RECORD]);
        e.mem.set_u32(save_load + 0x88, saving);
        // The record (id, word at +5, version byte), also a buffer (change flags at +0x17, revert
        // flags at +0x2c) and a package's data (the reach at +8); the form is a global (its value
        // at +0x24).
        let f = self.objects[8];
        let id = self.pick(&[0, 1, 0x0010_0000, 0xff00_0001]);
        e.mem.set_u32(f, id);
        let changes = self.pick(&[0, 0x0010_0000, 0xffff_ffff]);
        e.mem.set_u32(f + 0x17, changes);
        let reverts = self.pick(&[0, 0x0010_0000, 0xffff_ffff]);
        e.mem.set_u32(f + 0x2c, reverts);
        let value = self.pick(FLOATS);
        e.mem.set_u32(self.objects[7] + 0x24, value);
        let data_word = self.pick(&[0, 100, 0xffff_fffb]);
        e.mem.set_u32(f + 8, data_word);
    }

    fn seed_globals(&mut self, e: &mut Engine) {
        for page in [
            0x011d_6000,
            0x011d_7000,
            0x011d_e000,
            0x0101_8000,
            0x011c_d000,
            0x011c_3000,
            0x011c_a000,
            0x0119_b000,
            0x011a_3000,
            0x011a_4000,
            0x0101_2000,
            0x0101_1000,
            0x0101_d000,
            0x0103_0000,
            0x0103_5000,
            0x0127_0000,
        ] {
            e.map(page, 0x1000);
        }
        // The hours of a day (the exe's 24.0), which the travel's dead hours read past midnight.
        e.mem.set_f64(0x0103_56d8, 24.0);
        // The actor value infos: four infos with seeded flags, every entry one of them.
        let mut infos = [0u32; 4];
        for info in &mut infos {
            *info = e.mem.alloc(0x50);
            let flags = self.pick(&[0, 0x100, 0x200, 0x300]);
            e.mem.set_u32(*info + 0x44, flags);
        }
        for i in 0..0x3ff {
            let info = infos[(self.next() % 4) as usize];
            e.mem.set_u32(AV_INFOS - 4 + 4 * i, info);
        }
        // The calendar's hour and day globals, the default hour and the default day (days 3 and
        // 31, a signalling NaN and 2^32, which truncate to 0x8000_0000).
        let global = self.one_of(&[0, FORM]);
        e.mem.set_u32(CALENDAR + 0xc, global);
        let day = self.one_of(&[0, FORM]);
        e.mem.set_u32(CALENDAR + 8, day);
        let hour = self.pick(&[0x4140_0000, 0x41a0_0000, 0x7f80_0001]);
        e.mem.set_u32(DEFAULT_HOUR, hour);
        let day = self.pick(&[0x4040_0000, 0x41f8_0000, 0x7f80_0001, 0x4f80_0000]);
        e.mem.set_u32(DEFAULT_DAY, day);
        // The settings: the combat rounds (small and not below 0, so the rounds end), the reaches
        // and the follower limit.
        let rounds = self.pick(&[0, 0x3f00_0000, 0x3f80_0000, 0x3fc0_0000, 0x4000_0000]);
        e.mem.set_u32(ROUNDS_SETTING, rounds);
        for at in [INTERIOR_REACH, REACH] {
            let reach = self.pick(FLOATS);
            e.mem.set_u32(at, reach);
        }
        let limit = self.pick(&[0, 1, 5]);
        e.mem.set_u32(FOLLOWER_LIMIT, limit);
        let handler = self.word();
        e.mem.set_u32(DATA_HANDLER, handler);
        let form = self.pick(&[0, 0x1234]);
        e.mem.set_u32(TARGET_FORM, form);
        let player = self.one_of(&[0, ACTOR, TARGET, FORM]);
        e.mem.set_u32(PLAYER, player);
        for i in 0..0x50 {
            let name = self.word();
            e.mem.set_u32(PROCEDURE_NAMES - 4 + 4 * i, name);
        }
        // The procedure action table: a row per package type (0 to 0x1a), each a seeded action per
        // procedure index (-1 to 3).
        for kind in 0..0x1b {
            let row = ACTIONS + 0x10 + 0x20 * kind;
            e.mem.set_u32(ACTION_TABLE + 4 * kind, row);
            for index in 0..5 {
                let action = self.pick(ACTION_WORDS);
                e.mem.set_u32(row - 4 + 4 * index, action);
            }
        }
        let bits = self.pick(FLOATS);
        e.mem.set_u32(MINUS_ONE_FLOAT, bits);
        let radius = self.pick(FLOATS);
        e.mem.set_u32(CLOSE_RADIUS, radius);
        let zero = self.pick(&[0, 0, 0x8000_0000, 0x3ff0_0000]);
        e.mem.set_u32(ZERO_DOUBLE, 0);
        e.mem.set_u32(ZERO_DOUBLE + 4, zero);
        // The exe's 2.0 and 4.0 (doubles the float rule narrows exactly).
        e.mem.set_f64(TWO_DOUBLE, 2.0);
        e.mem.set_f64(ONE_HOUR_FACTOR, 4.0);
        e.mem.set_u32(SAVE_LOAD_GAME, self.resolve(SAVE_LOAD));
        let log = self.pick(&[0, 1, 0x80]);
        e.mem.set_u8(SAVE_LOG_BYTE, log as u8);
        let sse2 = self.pick(&[0, 1, 1]);
        e.mem.set_u32(SSE2_AVAILABLE, sse2);
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

    /// An actor value code: small, the last ones and some past them.
    fn code(&mut self) -> u32 {
        self.pick(&[0, 1, 5, 0x20, 0x4c, 0xff, 0x100, 0x3fe, 0xffff_ffff])
    }

    /// The process, an actor (or none), a code and a float or integer word.
    fn modifier_call(&mut self, float: bool) -> Vec<u32> {
        let actor = self.one_of(&[0, ACTOR, TARGET]);
        let code = self.code();
        let amount = if float {
            self.pick(FLOATS)
        } else {
            self.pick(&[0, 1, 0xffff_ffff, 0x0100_0001, 0x8000_0000])
        };
        vec![self.process(), actor, code, amount]
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
            .family(&MULTIBOUND)
            .ignore_calls(IN_PLACE)
            .goes_by_address(&[0x0070_3180])
            .inputs(&[Input::Of(seeds)])
            .setup(move |e, case| {
                let mut w = World::new(e, case.word(0));
                words(&mut w)
            })
    }
}

#[test]
fn actor_values() {
    Diff::new(entry!(
        0x0092_ce00,
        middle_low_process::get_actor_float_value(Ptr, u32, u32, u32) -> f64
    ))
    .in_world(&MOST, |w| w.modifier_call(false))
    // The sum of two NaNs: SSE keeps the first one's payload, x87's FADD the larger one's. Only
    // here: every other Diff compares NaN-looking words exactly.
    .allow_nan_payloads()
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_cd90,
        middle_low_process::get_actor_value(Ptr, u32, u32, u32) -> i32
    ))
    .in_world(&MOST, |w| w.modifier_call(false))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_cf10,
        middle_low_process::get_temporary_modifier(Ptr, u8) -> f64
    ))
    .in_world(&FEW, |w| {
        let code = w.code();
        vec![w.process(), code]
    })
    .run()
    .assert_same();
}

#[test]
fn modifiers() {
    Diff::new(entry!(
        0x0092_cec0,
        middle_low_process::temp_mod_actor_value_float(Ptr, u32, u32, f32)
    ))
    .in_world(&MANY, |w| w.modifier_call(true))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_ce70,
        middle_low_process::temp_mod_actor_value(Ptr, u32, u32, i32)
    ))
    .in_world(&MANY, |w| w.modifier_call(false))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_cf30,
        middle_low_process::damage_mod_actor_value_ov2(Ptr, u32, u32, f32)
    ))
    .in_world(&MOST, |w| w.modifier_call(true))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_7430,
        middle_low_process::damage_mod_actor_value(Ptr, u32, u32, i32)
    ))
    .in_world(&MOST, |w| w.modifier_call(false))
    .run()
    .assert_same();
}

#[test]
fn life_and_steps() {
    Diff::new(entry!(0x0092_ca70, middle_low_process::copy(Ptr, Ptr)))
        .in_world(&FEW, |w| {
            let other = w.objects[6];
            vec![w.process(), other]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0092_ca00, middle_low_process::destructor(Ptr)))
        .in_world(&FEW, |w| w.process_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0092_c9d0,
        middle_low_process::scalar_deleting_destructor(Ptr, u32) -> Ptr
    ))
    .in_world(&MANY, |w| w.process_and(1))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_e400,
        middle_low_process::update_step(Ptr, u32, f32)
    ))
    .in_world(&MANY, |w| {
        let actor = w.one_of(&[0, ACTOR]);
        let step = w.pick(FLOATS);
        vec![w.process(), actor, step]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_cfd0,
        middle_low_process::checkfor_new_package(Ptr, u32, u8) -> u8
    ))
    .in_world(&MOST, |w| {
        let actor = w.one_of(&[0, ACTOR]);
        let force = w.pick(&[0, 0, 1, 0x100]);
        vec![w.process(), actor, force]
    })
    .run()
    .assert_same();
}

#[test]
fn combat() {
    Diff::new(entry!(
        0x0092_cab0,
        middle_low_process::process_combat(Ptr, u32)
    ))
    .in_world(&MOST, |w| {
        let actor = w.one_of(&[ACTOR, TARGET]);
        vec![w.process(), actor]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_cc90,
        middle_low_process::process_combat_one_hour(Ptr, u32, u32) -> bool
    ))
    .in_world(&MOST, |w| {
        let actor = w.one_of(&[ACTOR, TARGET]);
        vec![w.process(), actor, w.word()]
    })
    .run()
    .assert_same();
}

#[test]
fn save_and_load() {
    Diff::new(entry!(
        0x0092_e420,
        middle_low_process::get_save_size(Ptr, u32, u32) -> u16
    ))
    .in_world(&MANY, |w| {
        let flags = w.pick(&[0, 0x0010_0000, 0xffff_ffff]);
        vec![w.process(), flags, w.word()]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_e560,
        middle_low_process::save_game(Ptr, u32, u32)
    ))
    .in_world(&MOST, |w| {
        let flags = w.pick(&[0, 0x0010_0000, 0xffff_ffff]);
        vec![w.process(), flags, w.word()]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_e730,
        middle_low_process::load_game(Ptr, u32, u32, u32)
    ))
    // Every seed: the block's clean end needs a run of reader answers that few seeds give.
    .in_world(&ALL, |w| {
        let flags = w.pick(&[0, 0x0010_0000, 0xffff_ffff]);
        vec![w.process(), flags, w.word(), w.word()]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_e9f0,
        middle_low_process::revert(Ptr, u32, u32)
    ))
    .in_world(&MANY, |w| {
        let flags = w.pick(&[0, 0x0010_0000, 0xffff_ffff]);
        vec![w.process(), flags, w.word()]
    })
    .run()
    .assert_same();
}

#[test]
fn save_and_load_of_a_buffer() {
    let with_buffer = |w: &mut World| {
        let buffer = w.objects[8];
        vec![w.process(), buffer]
    };
    Diff::new(entry!(
        0x0092_ea30,
        middle_low_process::save_game_buffer(Ptr, Ptr)
    ))
    .in_world(&MANY, with_buffer)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_eaa0,
        middle_low_process::load_game_buffer(Ptr, u32)
    ))
    .in_world(&MANY, with_buffer)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_eb00,
        middle_low_process::revert_buffer(Ptr, u32)
    ))
    .in_world(&MANY, with_buffer)
    .run()
    .assert_same();
}

/// `Update`'s words: the process and an actor (or none, or a form that is not one).
fn update_words(w: &mut World) -> Vec<u32> {
    let actor = w.one_of(&[0, ACTOR, ACTOR, ACTOR, TARGET]);
    vec![w.process(), actor]
}

/// The callees `Update` leaves out besides the ones done in place: the travel's form lookup and the
/// process's hour (the seconds they give are never read).
const UPDATE_LEFT_OUT: &[u32] = &[0x0048_39c0, 0x007d_f1f0];

/// `Calendar::GetDay`: the day global's value or the default day (days, a signalling NaN, 2^32),
/// truncated.
#[test]
fn calendar_day() {
    Diff::new(entry!(0x0086_7d60, middle_low_process::calendar_day(Ptr) -> u32))
        .in_world(&MANY, |_| vec![CALENDAR])
        .run()
        .assert_same();
}

#[test]
fn update() {
    Diff::new(entry!(0x0092_d050, middle_low_process::update(Ptr, u32)))
        .in_world(&ALL, update_words)
        // The world again, with a form lookup that keeps no count: the typed side never asks it.
        .setup(|e, case| {
            let mut w = World::new(e, case.word(0));
            let form = w.objects[7];
            e.register_double(0x0048_39c0, move |_, _| ret(form));
            update_words(&mut w)
        })
        .ignore_calls(UPDATE_LEFT_OUT)
        .run()
        .assert_same();
}

/// One function's translation run in the world of each seed, with `check` asked after each run
/// which of its branches the run took; every branch has to be taken by some seed.
fn reaches(
    addr: u32,
    seeds: &[u32],
    words: fn(&mut World) -> Vec<u32>,
    branches: &[&str],
    check: fn(&Engine, &[u32], &[u32]) -> Vec<bool>,
) {
    let mut taken = vec![0u32; branches.len()];
    for &seed in seeds {
        let mut e = Engine::new_untyped_for_test();
        let mut w = World::new(&mut e, seed);
        let args = words(&mut w);
        // Before the call: the receiver's words up to +0xc0, then the save/load game's position.
        let mut before: Vec<u32> = (0..0x30).map(|i| e.mem.u32(args[0] + 4 * i)).collect();
        before.push(e.mem.u32(e.mem.u32(SAVE_LOAD_GAME) + 0x14));
        e.call(addr, &args);
        for (n, hit) in taken.iter_mut().zip(check(&e, &args, &before)) {
            *n += u32::from(hit);
        }
    }
    for (name, n) in branches.iter().zip(&taken) {
        assert!(*n > 0, "{addr:08x}: no seed reaches \"{name}\": {taken:?}");
    }
}

/// The worlds the seeds make reach the deep branches, checked on the translations (so a change to
/// the world that loses one shows here, not as a weaker comparison).
#[test]
fn the_seeds_reach_every_branch() {
    const WARNING: u32 = 0x005b_5e40;
    const ADJUST: u32 = 0x0093_7480;
    reaches(
        0x0092_ce00,
        &MOST,
        |w| w.modifier_call(false),
        &["no modifiers", "with the modifier"],
        |e, _, _| {
            vec![
                calls(e, 0x0090_7380) > 0 && calls(e, 0x0093_7730) == 0,
                calls(e, 0x0093_7730) > 0,
            ]
        },
    );
    reaches(
        0x0092_cf30,
        &MOST,
        |w| w.modifier_call(true),
        &["skipped", "first kind", "second kind"],
        |e, _, _| {
            vec![
                calls(e, ADJUST) == 0,
                calls(e, ADJUST) > 0 && e.mem.u32(block(double(ADJUST)) + 0x28) & 0xff != 0x100,
                calls(e, 0x0066_eea0) > 0,
            ]
        },
    );
    reaches(
        0x0092_cfd0,
        &MOST,
        |w| {
            let actor = w.one_of(&[0, ACTOR]);
            vec![w.process(), actor, 0]
        },
        &["same hour", "checked"],
        |e, _, _| vec![calls(e, 0x0090_a1a0) == 0, calls(e, 0x0090_a1a0) > 0],
    );
    reaches(
        0x0092_cab0,
        &MOST,
        |w| {
            let actor = w.one_of(&[ACTOR, TARGET]);
            vec![w.process(), actor]
        },
        &[
            "stops",
            "elsewhere",
            "rounds",
            "target's round",
            "target's process",
        ],
        |e, _, _| {
            vec![
                slot_calls(e, 0x434) > 0,
                slot_calls(e, 0x298) > 0,
                slot_calls(e, 0x438) > 0,
                slot_calls(e, 0x438) > 1,
                slot_calls(e, 0x28) > 0,
            ]
        },
    );
    reaches(
        0x0092_cc90,
        &MOST,
        |w| {
            let actor = w.one_of(&[ACTOR, TARGET]);
            vec![w.process(), actor, 0]
        },
        &["rounds", "target's process"],
        |e, _, _| vec![slot_calls(e, 0x438) > 1, slot_calls(e, 0x28) > 0],
    );
    reaches(
        0x0092_e560,
        &MOST,
        |w| vec![w.process(), 0x0010_0000, 0],
        &["blocks", "too large", "exactly 0xffff", "reported"],
        |e, _, before| {
            vec![
                calls(e, 0x0085_79b0) > 2,
                calls(e, WARNING) > 0 && last_word(e, WARNING) == 0x0101_5318,
                calls(e, 0x0085_79b0) > 2
                    && calls(e, WARNING) == 0
                    && e.mem.u16(before[0x30] + 4) == 0xffff,
                calls(e, 0x0048_39c0) > 0,
            ]
        },
    );
    reaches(
        0x0092_e730,
        &ALL,
        |w| vec![w.process(), 0x0010_0000, 0, 0],
        &[
            "bad header",
            "overrun, a record",
            "underrun, a record",
            "overrun, the version",
            "underrun, the version",
            "the block's end",
        ],
        |e, _, _| {
            let warned = |format| calls(e, WARNING) > 0 && last_word(e, WARNING) == format;
            vec![
                warned(0x0101_5718) || warned(0x0101_56a8),
                warned(0x0101_5588),
                warned(0x0101_5500),
                warned(0x0101_54a0),
                warned(0x0101_5440),
                calls(e, 0x0085_79e0) > 2 && calls(e, WARNING) == 0,
            ]
        },
    );
    reaches(
        0x0092_d050,
        &ALL,
        update_words,
        &[
            "not an actor",
            "new package",
            "reset (an action after it)",
            "travel: path goal",
            "travel: look around",
            "travel: facing",
            "use",
            "follow",
            "reach: done",
            "reach: distance",
            "a single slot",
            "interrupt ended",
            "finish: action flag",
            "finish: made objects",
            "finish: once a day",
            "finish: new package",
            "finish: turned away",
            "procedure text",
            "waits on its path",
        ],
        |e, args, _| {
            let actor = args[1];
            // The actor's slot 0x100 (never the exe's body here) says whether it is one.
            let is_actor = actor != 0 && e.mem.u8(e.mem.u32(actor) + ANSWERS + 0x100) != 0;
            let mut taken = vec![
                actor != 0 && !is_actor,
                slot_calls(e, 0x524) > 0,
                false,
                calls(e, 0x008b_3690) > 0,
                calls(e, 0x0046_f280) > 0,
                slot_calls(e, 0x2c4) > 0,
                slot_calls(e, 0x7c8) > 0 && calls(e, 0x0067_2dd0) > 0,
                slot_calls(e, 0xc8) > 0,
                slot_calls(e, 0x288) > 0 && calls(e, 0x0057_23b0) > 0,
                calls(e, 0x0057_23b0) > 0,
                [0x7d0, 0x7b4, 0x808, 0x7f0, 0x2a0]
                    .iter()
                    .any(|s| slot_calls(e, *s) > 0),
                calls(e, 0x0088_1680) > 0,
                calls(e, 0x005a_c750) > 0,
                calls(e, 0x0090_5330) > 0,
                slot_calls(e, 0x28c) > 0,
                slot_calls(e, 0x24) >= 2,
                is_actor && calls(e, 0x008b_3bf0) == 0,
                calls(e, 0x00ec_623a) > 0,
                calls(e, 0x008b_3bf0) > 0 && slot_calls(e, 0x28) == 0,
            ];
            // ResetTarget ran when an action after it did.
            taken[2] = taken[3..=10].iter().any(|t| *t);
            taken
        },
    );
}
