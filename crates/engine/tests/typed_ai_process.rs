//! The differential test of every typed function of the AI family's process modules
//! (`engine_typed::ai::base_process` and the `ActorPackage` every process embeds,
//! `engine_typed::ai::actor_package`), and of `LowProcess`'s and `MiddleLowProcess`'s actor values,
//! destructors and save, load and revert (`engine_typed::ai::low_process` part 4 and the
//! `MiddleLowProcess` functions that call them first), against its translation, all in one world
//! built from a seed.
//!
//! The world: five objects of 0x400 bytes (the process under test, another process, two forms
//! that serve as packages, records and buffers, and the save/load object), each word a seeded small
//! integer, float bit pattern or pointer to one of them. The fields the code follows are seeded
//! from the values that choose its branches: each process's embedded package (its package, data,
//! target, procedure index, start time and flags) and cached values; each package's form id,
//! procedure, flags, type and count; the save/load object's position, version and records. Each
//! object has a vtable of its own: every slot the code calls is a stand-in answering a seeded word
//! (in EAX, and as a float in ST0) and counting its calls, or, by the seed, the exe's own body
//! (0x130, 0x22c, 0x238, 0x23c, 0x240, 0x280, 0x284), so the typed side's direct branches run. The
//! globals the code reads (the shared point, the float and double constants, the player and its
//! running package, the save/load game, the procedure counts and the package type names) are
//! seeded too.
//!
//! What stands in: every callee the code calls by address. Each counts its calls, keeps its last
//! two words and answers the next of eight seeded words; the readers also write a word into the
//! bytes they read, and the save/load game's readers and writers move its position on as the
//! exe's do. What runs as itself: the family's own functions and those of the families its typed
//! calls reach (the leaves, the AI queries, the dispatch family's counts, counted pointers and map
//! lookup, the multibound family's world space getter, the save family's form id writer, over a
//! form id map whose lookup finds every plugin id). [`the_seeds_reach_every_branch`] checks, on the
//! translations, that the seeds take each deep branch.
//!
//! A second stream (drawn after the first, so the first part's world stays as it was) seeds what
//! `LowProcess`'s fields hold (the item, target, locations and forms as forms or none; the lists as
//! finite chains; the acquire records and the record to acquire), two actors' value owners (their
//! +0xa4 a vtable), the actor value infos and the doubles of the callees those functions call by
//! address.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_typed::ai::{actor_package, base_process, low_process, middle_low_process, AI};
use engine_typed::ai_queries::AI_QUERIES;
use engine_typed::crt::CRT;
use engine_typed::dispatch::DISPATCH;
use engine_typed::multibound::MULTIBOUND;
use engine_typed::save::SAVE;

/// Where the stand-in slots are registered: `VT_FN + slot`.
const VT_FN: u32 = 0x00ff_4000;
/// A vtable's stand-in answers sit this far past it, at the same slot offset.
const ANSWERS: u32 = 0x800;
/// The doubles' counts, last words and answers: double `k` at `CONTROL + 0x40 * k` (its count, its
/// eight answers, its last two words).
const CONTROL: u32 = 0x0300_0000;
/// The slots' call counts: slot `s` at `SLOT_COUNTS + s`.
const SLOT_COUNTS: u32 = CONTROL + 0x1000;
/// How far the save/load game's numeric id calls move the position on besides their size.
const EXTRA: u32 = CONTROL + 0x1800;
/// How far the modifier lists' save and load move the save/load game's position on (the second
/// stream's).
const EXTRA_2: u32 = CONTROL + 0x1804;
/// The save buffer the positions point into.
const BUFFER: u32 = 0x0400_0000;
/// The save buffer past the first stream's: the blocks of three process levels (`MiddleLowProcess`'s
/// save after `LowProcess`'s and `BaseProcess`'s), each with numeric ids or lists that move the
/// position on by up to 0x10000, end far out, where the last block's length is stored. Mapped only
/// where a block is written that far (every mapped page is hashed at each call a Diff compares).
const FAR_BUFFER: u32 = BUFFER + 0x2_0000;
const FAR_SIZE: u32 = 0x6_0000;

/// The slots the code calls, each a stand-in unless the seed gives the exe's body.
const SLOTS: [u32; 21] = [
    0x0, 0x4, 0x8, 0xc, 0x10, 0x14, 0x54, 0x5c, 0x64, 0x84, 0x12c, 0x130, 0x15c, 0x1e0, 0x214,
    0x22c, 0x238, 0x23c, 0x240, 0x280, 0x284,
];
/// The exe's bodies a slot may hold instead.
const REAL_SLOTS: [(u32, u32); 7] = [
    (0x130, 0x0040_1280),
    (0x22c, 0x007f_a950),
    (0x238, 0x008d_03c0),
    (0x23c, 0x008d_03e0),
    (0x240, 0x008d_0400),
    (0x280, 0x008d_0450),
    (0x284, 0x008d_0470),
];

/// A word that stands for object `n` of the world (`OBJECT + n`), resolved when the world is made.
const OBJECT: u32 = 0xb0b0_0000;
/// The process under test, the other process and the two forms (packages, records or buffers);
/// object 4 is the save/load object.
const PROCESS: u32 = OBJECT;
const OTHER: u32 = OBJECT + 1;
const FORM_A: u32 = OBJECT + 2;
const FORM_B: u32 = OBJECT + 3;

/// Words an argument or a field may hold.
const WORDS: &[u32] = &[
    0,
    1,
    6,
    0x80,
    0xffff,
    0x1_0000,
    0xffff_ffff,
    0x3f80_0000,
    0x7f80_0001,
    0xffc0_0000,
    PROCESS,
    OTHER,
    FORM_A,
    FORM_B,
];
/// Float bit patterns a float may hold: values, the signs of zero, a denormal, an infinity and NaNs
/// (a signalling one, which a load quiets).
const FLOATS: &[u32] = &[
    0x7f7f_ffff,
    0xbf80_0000,
    0x41a0_0000,
    0,
    0x8000_0000,
    1,
    0x7f80_0000,
    0x7f80_0001,
    0xffc0_0000,
];
/// Words a void callee answers (unread).
const UNREAD: &[u32] = &[0, 1];
/// The block tag, twice as likely as each other word a block read may give.
const TAG: u32 = 0x424c_4f4b;
/// Form ids: none, a plugin's, the last before the created forms', a created form's (`0xff000000`
/// and up, the first of them too).
const IDS: &[u32] = &[
    0,
    1,
    0x0010_0000,
    0xfeff_ffff,
    0xff00_0000,
    0xff00_0001,
    0xffff_ffff,
];

/// What a double does besides answering.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Answers only.
    Plain,
    /// A function the family types now: its answers are still drawn (so the rest of the world stays
    /// as it was), but it runs as itself.
    Typed,
    /// `thiscall(save_load, address, size)`: moves the position (+0x14) on by the size; `Extra`
    /// also by the world's extra.
    Write,
    Extra,
    /// `thiscall(object, address, size)`: writes its answer into the bytes (up to 4) and, on the
    /// save/load game (`ReadMoves`), moves the position on by the size; answers the next word.
    Read,
    ReadMoves,
    ReadExtra,
    /// `thiscall(buffer, address)`: writes its answer into the word; answers 1 only for an id of 0, by
    /// the next word (`LoadFormID_ov2`'s answer: the id was 0 and not to be found).
    ReadWord,
    /// Moves the save/load game's position on by the second stream's extra (what a list writes or
    /// reads).
    Moves,
    /// `thiscall(list, slot)`: empties the list (both its words), so a loop until empty ends.
    Remove,
}

/// The callees that stand in, with the words each may answer and what else it does.
const DOUBLES: &[(u32, &[u32], Kind)] = &[
    (0x0040_1030, UNREAD, Kind::Plain), // the free
    (
        0x00ec_43fb,
        &[0, PROCESS, OTHER, FORM_A, FORM_B],
        Kind::Plain,
    ), // __RTDynamicCast
    (0x008d_9fa0, UNREAD, Kind::Plain), // the level copies
    (0x0091_4c60, UNREAD, Kind::Plain),
    (0x0093_7160, UNREAD, Kind::Plain), // the middle-low copy's modifier list copy
    (0x0090_7510, UNREAD, Kind::Typed), // LowProcess's level copy
    (0x0086_75e0, UNREAD, Kind::Plain), // the cached values' copy
    (0x0067_59e0, &[0, 1, FORM_A], Kind::Plain), // TESPackage::ComputeInitialTarget
    (0x0085_79b0, UNREAD, Kind::Write), // the save/load game's byte writer
    (
        0x0085_79e0,
        &[TAG, TAG, 0, 6, 0x0e, 0x12, 0x10, 0xfff0],
        Kind::ReadMoves,
    ), // its byte reader
    (0x0048_39c0, &[FORM_A, FORM_B], Kind::Plain), // the form for an id
    (0x005b_5e40, UNREAD, Kind::Plain), // the SAVELOAD warnings and the AI log
    (0x008a_6ce0, UNREAD, Kind::Plain), // Actor::EvaluatePackage
    (0x0086_5e50, UNREAD, Kind::Plain), // the save buffer's byte writer
    (
        0x0086_4980,
        &[0, 1, 0xff, 0xe, 0x3f80_0000, 0x7f80_0001],
        Kind::Read,
    ), // the load buffer's byte reader
    (0x0086_5570, UNREAD, Kind::Plain), // the save buffer's append (under SaveFormID_ov2)
    (
        0x0086_7da0,
        &[0x4120_0000, 0, 0x8000_0000, 0x7f80_0001, 0x3f80_0001],
        Kind::Plain,
    ), // Calendar::GetHour
    (0x008c_6ca0, &[OTHER, FORM_B], Kind::Plain), // CreateActorPackageData
    (0x0085_7a10, UNREAD, Kind::Extra), // SaveNumericID
    (0x0085_7aa0, IDS, Kind::ReadExtra), // LoadNumericID
    (0x0086_48e0, IDS, Kind::ReadWord), // LoadFormID_ov2
    (0x0084_aa30, UNREAD, Kind::Plain), // AddChangedFormID
    (0x0084_aa90, &[0, 1, 0x1234], Kind::Plain), // the changed id's lookup
    (0x0067_0b90, &[FORM_A, FORM_B], Kind::Plain), // TESPackage::CreatePackage
    (0x0067_77b0, UNREAD, Kind::Plain), // TESPackage::CalculateProcedureType
    (0x0056_4db0, UNREAD, Kind::Plain), // TESObjectREFR::SetTargeted
    (0x0084_5cc0, UNREAD, Kind::Plain), // SaveGameWarning
];

/// The callees `LowProcess`'s and `MiddleLowProcess`'s functions call by address, with the words each
/// may answer and what else it does (drawn from the second stream).
const DOUBLES_2: &[(u32, &[u32], Kind)] = &[
    (0x0093_7730, FLOATS, Kind::Plain), // ModifierList::GetModifier
    (0x0093_7ab0, &[0, 3, 0xfffe], Kind::Plain), // ModifierList::GetSaveSize
    (0x0093_78d0, UNREAD, Kind::Moves), // ModifierList::SaveGame
    (0x0093_79c0, UNREAD, Kind::Moves), // ModifierList::LoadGame
    (0x0093_7b50, UNREAD, Kind::Plain), // ModifierList::SaveGame_ov2
    (0x0093_7c60, UNREAD, Kind::Plain), // ModifierList::LoadGame_ov2
    (0x0093_7b30, UNREAD, Kind::Plain), // the modifiers' revert forwarder
    (0x0093_7d50, UNREAD, Kind::Plain), // their buffer revert
    (0x0093_7030, UNREAD, Kind::Plain), // their destructor
    (0x0046_ffb0, UNREAD, Kind::Plain), // BSSimpleList's destructor
    (0x0047_0470, UNREAD, Kind::Plain), // its clear
    (0x0090_5330, UNREAD, Kind::Remove), // its remove
    (0x007b_3fa0, UNREAD, Kind::Plain), // the acquire records' delete
    (0x009a_5eb0, UNREAD, Kind::Plain), // CombatTimer's save of a buffer
    (0x009a_5f20, UNREAD, Kind::Plain), // its load
    (0x0086_5f20, &[0, 0x40, 0x1234], Kind::Plain), // the save buffer's count start
    (0x0086_5ff0, UNREAD, Kind::Plain), // its count
    (0x0086_48a0, IDS, Kind::Plain),    // the load buffer's form id
    (0x0086_4a60, &[0, 1, 2, 3], Kind::Plain), // its count
    (0x005a_e3d0, UNREAD, Kind::Plain), // BSSimpleList::AddItem
    (0x004b_1500, FLOATS, Kind::Plain), // the float remainder
];

/// The callees `LowProcess`'s functions' translations call and the typed code does in place or
/// leaves out: the actor value's flag (`00406d70`), the buffer's change and revert flags and their
/// test, the leaves of the locations list's walk and the empty test, the save/load game's loading
/// test (`0055f5b0`, its second ask and the form's flag `00440d80` before it left out) and the
/// combat timer's reset.
const LOW_IN_PLACE: &[u32] = &[
    0x0040_6d70,
    0x0042_8110,
    0x0042_80f0,
    0x0042_ce30,
    0x0068_15c0,
    0x0072_6070,
    0x0082_56d0,
    0x0055_f5b0,
    0x0044_0d80,
    0x0091_0b10,
];

/// The callees the translations call and the typed code does in place (their bodies are one load,
/// store or compare): 0041fd00, 0049bb50, 00825c00, 004fd3c0, 008df040, 00408d60, 00862110,
/// 0041a250, 0047c850 (false), 00674d40, 009611e0, 0096a820, 00857bd0 and 00469860.
const IN_PLACE: &[u32] = &[
    0x007d_f1f0, // the low level copy's hour, read and stored, and its date
    0x0069_3d50,
    0x0059_bb30,
    0x0041_fd00,
    0x0049_bb50,
    0x0082_5c00,
    0x004f_d3c0,
    0x008d_f040,
    0x0040_8d60,
    0x0086_2110,
    0x0041_a250,
    0x0047_c850,
    0x0067_4d40,
    0x0096_11e0,
    0x0096_a820,
    0x0085_7bd0,
    0x0046_9860,
];

/// The globals the code reads.
const LARGEST_FLOAT: u32 = 0x0108_5df0;
const ZERO_DOUBLE: u32 = 0x0101_2060;
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
const BREATH_TIMER_FLOAT: u32 = 0x0101_7868;
const SHARED_POINT: u32 = 0x011f_426c;
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const SAVE_LOG_BYTE: u32 = 0x011d_e4ec;
const PLAYER: u32 = 0x011d_ea3c;
const PROCEDURE_COUNTS: u32 = 0x011a_3fbc;
const PACKAGE_TYPE_NAMES: u32 = 0x0119_bcb0;
/// The save/load game whose form id map the save family's id writer uses.
const FORM_ID_MAP_GAME: u32 = 0x011d_df38;
/// The actor value infos, by actor value; an info's flags at +0x44.
const AV_INFOS: u32 = 0x011d_61c8;

/// The seeds: a few for the functions with no branch, many for the rest.
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

/// The index of the double at `addr`: the first list's, then the second's.
fn double(addr: u32) -> usize {
    match DOUBLES.iter().position(|(a, _, _)| *a == addr) {
        Some(k) => k,
        None => {
            let k = DOUBLES_2
                .iter()
                .position(|(a, _, _)| *a == addr)
                .expect("a double");
            DOUBLES.len() + k
        }
    }
}

/// How many times the double at `addr` was called.
fn calls(e: &Engine, addr: u32) -> u32 {
    e.mem.u32(block(double(addr)))
}

/// The first word of the last call of the double at `addr`.
fn last_word(e: &Engine, addr: u32) -> u32 {
    e.mem.u32(block(double(addr)) + 0x24)
}

/// The third word of the last call of the double at `addr`.
fn third_word(e: &Engine, addr: u32) -> u32 {
    e.mem.u32(block(double(addr)) + 0x30)
}

/// The first word of the first call of the double at `addr`.
fn first_word(e: &Engine, addr: u32) -> u32 {
    e.mem.u32(block(double(addr)) + 0x2c)
}

/// Registers the double at `addr` (block `block`) doing what `kind` says; a typed function's is not
/// registered.
fn register(e: &mut Engine, addr: u32, block: u32, kind: Kind) {
    if matches!(kind, Kind::Typed) {
        return;
    }
    e.register_double(addr, move |e, a| {
        let n = e.mem.u32(block);
        e.mem.set_u32(block, n + 1);
        if n == 0 {
            e.mem.set_u32(block + 0x2c, a.first().copied().unwrap_or(0));
        }
        e.mem.set_u32(block + 0x24, a.first().copied().unwrap_or(0));
        e.mem.set_u32(block + 0x28, a.get(1).copied().unwrap_or(0));
        e.mem.set_u32(block + 0x30, a.get(2).copied().unwrap_or(0));
        let answer = e.mem.u32(block + 4 + 4 * (n % 8));
        let next = e.mem.u32(block + 4 + 4 * ((n + 1) % 8));
        let (bytes, moves) = match kind {
            Kind::Plain | Kind::Typed | Kind::Moves | Kind::Remove => (0, 0),
            Kind::Write => (0, a[2]),
            Kind::Extra => (0, a[2].wrapping_add(e.mem.u32(EXTRA))),
            Kind::Read => (a[2].min(4), 0),
            Kind::ReadMoves => (a[2].min(4), a[2]),
            Kind::ReadExtra => (a[2].min(4), a[2].wrapping_add(e.mem.u32(EXTRA))),
            Kind::ReadWord => (4, 0),
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
            let moved = e.mem.u32(position).wrapping_add(e.mem.u32(EXTRA_2));
            e.mem.set_u32(position, moved);
        }
        if kind == Kind::Remove {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
        }
        match kind {
            Kind::ReadWord => ret(u32::from(answer == 0 && next & 1 != 0)),
            Kind::Read | Kind::ReadMoves | Kind::ReadExtra => ret(next),
            _ => ret(answer),
        }
    });
}

/// The world of one seed, made the same on both engines.
struct World {
    objects: [u32; 5],
    rng: u64,
    /// The second stream.
    rng2: u64,
    /// The buffer of its own the second stream makes (see [`World::low_buffer`]).
    buffer: u32,
}

impl World {
    fn new(e: &mut Engine, seed: u32) -> World {
        let mut w = World {
            objects: [0; 5],
            rng: (u64::from(seed) ^ 0x9e37_79b9_7f4a_7c15).wrapping_mul(0xbf58_476d_1ce4_e5b9) | 1,
            rng2: (u64::from(seed) ^ 0x2545_f491_4f6c_dd1d).wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1,
            buffer: 0,
        };
        e.map(CONTROL, 0x2000);
        e.map(BUFFER, 0x2_0000);
        for slot in SLOTS {
            e.register_double(VT_FN + slot, move |e, a| {
                let n = e.mem.u32(SLOT_COUNTS + slot);
                e.mem.set_u32(SLOT_COUNTS + slot, n + 1);
                let vt = e.mem.u32(a[0]);
                ret(e.mem.u32(vt + ANSWERS + slot))
            });
        }
        // A process block of exactly 0xffff bytes (14 more without an id, 18 with one) and one past.
        let extra = w.pick(&[0, 0, 0, 0, 4, 0xffed, 0xfff1, 0xfff2, 0x1_0000]);
        e.mem.set_u32(EXTRA, extra);
        for (k, (addr, answers, kind)) in DOUBLES.iter().enumerate() {
            let block = block(k);
            for i in 0..8 {
                let answer = w.pick(answers);
                e.mem.set_u32(block + 4 + 4 * i, answer);
            }
            register(e, *addr, block, *kind);
        }
        // The imports `InterlockedIncrement` and `InterlockedDecrement`, which the counts reach.
        e.register(0x00fd_f1c0, |e, a| {
            let n = e.mem.u32(a[0]).wrapping_add(1);
            e.mem.set_u32(a[0], n);
            ret(n)
        });
        e.register(0x00fd_f1c4, |e, a| {
            let n = e.mem.u32(a[0]).wrapping_sub(1);
            e.mem.set_u32(a[0], n);
            ret(n)
        });
        for i in 0..5 {
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
        for i in 0..5 {
            let o = w.objects[i];
            let vt = e.mem.alloc(2 * ANSWERS);
            e.mem.set_u32(o, vt);
            for offset in (4..0x400).step_by(4) {
                let word = w.word();
                e.mem.set_u32(o + offset, word);
            }
            for slot in SLOTS {
                e.mem.set_u32(vt + slot, VT_FN + slot);
                let answers: &[u32] = match slot {
                    0x0 => &[0, 5, 6, 7, 0x80, 0xff, 0x106],
                    0x8 => &[0, 1, 6, 0xe, 0xffff_ffff],
                    0x15c => &[0, 1, 0x100],
                    0x1e0 => &[0x3f80_0000, 0, 0x8000_0000, 0xbf80_0000, 0x7fc0_0000],
                    0x22c => &[0, FORM_A],
                    0x23c | 0x280 => &[0xffff_ffff, 0, 1],
                    0x130 => &[0x0101_1584, 0x1234],
                    _ => WORDS,
                };
                let answer = w.pick(answers);
                e.mem.set_u32(vt + ANSWERS + slot, w.resolve(answer));
            }
            for (slot, body) in REAL_SLOTS {
                if w.flag() {
                    e.mem.set_u32(vt + slot, body);
                }
            }
        }
        // The processes: the embedded package (its package, data, target, procedure index, start
        // time and flags) and the cached values.
        for o in [w.objects[0], w.objects[1]] {
            let package = w.one_of(&[0, FORM_A, FORM_B]);
            e.mem.set_u32(o + 4, package);
            let data = w.one_of(&[0, OTHER, FORM_B]);
            e.mem.set_u32(o + 8, data);
            // A form or none: the save family's id writer reads a target's id.
            let target = w.one_of(&[0, PROCESS, FORM_A, FORM_B]);
            e.mem.set_u32(o + 0xc, target);
            let index = w.pick(&[0xffff_ffff, 0, 1, 5, 0x30, 0x7fff_ffff]);
            e.mem.set_u32(o + 0x10, index);
            let start = w.pick(FLOATS);
            e.mem.set_u32(o + 0x14, start);
            let flags = w.pick(&[0, 1, 2, 3, 0xffff_ffff]);
            e.mem.set_u32(o + 0x18, flags);
            let cached = w.one_of(&[0, OTHER, FORM_B]);
            e.mem.set_u32(o + 0x2c, cached);
        }
        // The packages: form id, procedure, flags, type and count.
        for o in [w.objects[2], w.objects[3]] {
            let id = w.pick(IDS);
            e.mem.set_u32(o + 0xc, id);
            let procedure = w.pick(&[0xffff_ffff, 0, 3, 0x30, 0x31]);
            e.mem.set_u32(o + 0x18, procedure);
            let flags = w.pick(&[0, 0x800, 0xffff_ffff]);
            e.mem.set_u32(o + 0x1c, flags);
            let kind = w.pick(&[0xff, 0, 1, 6, 0xe]);
            e.mem.set_u8(o + 0x20, kind as u8);
            let count = w.pick(&[0, 1, 2]);
            e.mem.set_u32(o + 0x7c, count);
        }
        // The save/load object: its position, version and the records being loaded and saved.
        let save_load = w.objects[4];
        let position = BUFFER + w.pick(&[0, 2, 0x10]);
        e.mem.set_u32(save_load + 0x14, position);
        let version = w.pick(&[0, 6, 0x1f, 0x20, 0x59, 0x5a, 0xff]);
        e.mem.set_u8(save_load + 0x80, version as u8);
        let loading = w.one_of(&[0, OTHER, FORM_B]);
        e.mem.set_u32(save_load + 0x84, loading);
        let saving = w.one_of(&[0, OTHER, FORM_A]);
        e.mem.set_u32(save_load + 0x88, saving);
        // The globals.
        for page in [
            0x0108_5000,
            0x0101_2000,
            0x0101_7000,
            0x011f_4000,
            0x011d_e000,
            0x011d_d000,
            0x011a_3000,
            0x0119_b000,
            0x011c_3000,
        ] {
            e.map(page, 0x1000);
        }
        for at in [LARGEST_FLOAT, MINUS_ONE_FLOAT, BREATH_TIMER_FLOAT] {
            let bits = w.pick(FLOATS);
            e.mem.set_u32(at, bits);
        }
        let zero = w.pick(&[0, 0x8000_0000, 0xbff0_0000, 0x7ff8_0000]);
        e.mem.set_u32(ZERO_DOUBLE, 0);
        e.mem.set_u32(ZERO_DOUBLE + 4, zero);
        for i in 0..3 {
            let word = w.word();
            e.mem.set_u32(SHARED_POINT + 4 * i, word);
        }
        e.mem.set_u32(SAVE_LOAD_GAME, save_load);
        let log = w.pick(&[0, 1, 0x80]);
        e.mem.set_u8(SAVE_LOG_BYTE, log as u8);
        let player = w.one_of(&[FORM_B, PROCESS, OTHER]);
        e.mem.set_u32(PLAYER, player);
        if w.flag() {
            let package = e.mem.u32(w.objects[0] + 4);
            e.mem.set_u32(player + 0x208, package);
        }
        for i in 0..0x31 {
            let count = w.pick(&[0xff, 0, 1, 3, 0x7f]);
            e.mem.set_u8(PROCEDURE_COUNTS + i, count as u8);
        }
        for i in 0..0x42 {
            let name = w.word();
            e.mem.set_u32(PACKAGE_TYPE_NAMES - 4 + 4 * i, name);
        }
        // The save family's form id map (the save/load game at `011ddf38`, its +8), which the typed
        // `SaveFormID_ov2` asks for a plugin form's index: its hash (slot 4) answers bucket 0, whose
        // one node's key compares equal (slot 8), so the index is the node's.
        let game = e.mem.alloc(0x10);
        let map = e.mem.alloc(0x40);
        let map_vt = e.mem.alloc(2 * ANSWERS);
        e.mem.set_u32(map, map_vt);
        for (slot, answer) in [(4, 0), (8, 1)] {
            e.mem.set_u32(map_vt + slot, VT_FN + slot);
            e.mem.set_u32(map_vt + ANSWERS + slot, answer);
        }
        let table = e.mem.alloc(0x10);
        let node = e.mem.alloc(0x10);
        let index = w.pick(&[0, 0x12_3456, 0xff_ffff]);
        e.mem.set_u32(node + 8, index);
        e.mem.set_u32(table, node);
        e.mem.set_u32(map + 8, table);
        e.mem.set_u32(game + 8, map);
        e.mem.set_u32(FORM_ID_MAP_GAME, game);
        w.seed_low(e);
        w
    }

    /// The second stream: its doubles, `LowProcess`'s fields of the two processes, the actors'
    /// value owners, the actor value infos and the save/load game's loading word.
    fn seed_low(&mut self, e: &mut Engine) {
        let extra = self.pick2(&[0, 0, 4, 0xfff9, 0xfffa, 0x1_0000]);
        e.mem.set_u32(EXTRA_2, extra);
        for (k, (addr, answers, kind)) in DOUBLES_2.iter().enumerate() {
            let block = block(DOUBLES.len() + k);
            for i in 0..8 {
                let answer = self.pick2(answers);
                e.mem.set_u32(block + 4 + 4 * i, self.resolve(answer));
            }
            register(e, *addr, block, *kind);
        }
        // A list node a list's head may lead to: a location or none, then the end.
        let node = e.mem.alloc(8);
        let location = self.one_of2(&[0, FORM_A, FORM_B]);
        e.mem.set_u32(node, location);
        let forms = [0, FORM_A, FORM_B];
        // The process under test only (the other process stays as the first stream made it).
        {
            let o = self.objects[0];
            // The item, the target and the locations and forms: forms or none (their ids are read).
            let item = self.one_of2(&[0, FORM_A, FORM_B, OTHER]);
            e.mem.set_u32(o + 0x34, item);
            let target = self.one_of2(&[0, FORM_A, FORM_B, PROCESS]);
            e.mem.set_u32(o + 0x40, target);
            for at in [0x44, 0x48, 0x4c, 0x54] {
                let form = self.one_of2(&forms);
                e.mem.set_u32(o + at, form);
            }
            // `ObjectList` and `TempObjectList`: a record or none, then the node or the end.
            for at in [0x5c, 0x74] {
                let record = self.one_of2(&[0, FORM_A, OTHER]);
                let next = self.pick2(&[0, node]);
                e.mem.set_u32(o + at, record);
                e.mem.set_u32(o + at + 4, next);
            }
            // The acquire records (an acquire one makes the target's id 0).
            let acquire = self.one_of2(&[0, 0, FORM_A]);
            e.mem.set_u32(o + 0x64, acquire);
            let saved = self.one_of2(&[0, FORM_A, FORM_B]);
            e.mem.set_u32(o + 0x68, saved);
            // `GenericLocationsList` and `TempLocList`: a location or none, then the node or the end.
            for at in [0x6c, 0x7c] {
                let location = self.one_of2(&forms);
                let next = self.pick2(&[0, 0, node]);
                e.mem.set_u32(o + at, location);
                e.mem.set_u32(o + at + 4, next);
            }
            // The record to acquire: an object with a vtable (its slot 0x10 deletes it) or none.
            let record = self.one_of2(&[0, FORM_B, OTHER]);
            e.mem.set_u32(o + 0x84, record);
        }
        // The two forms that serve as actors: their value owner (+0xa4) a vtable whose slots 0 and 4
        // stand in.
        for o in [self.objects[2], self.objects[3]] {
            let owner = self.pick2(&[1, 2, 3]);
            let vt = e.mem.u32(self.objects[owner as usize]);
            e.mem.set_u32(o + 0xa4, vt);
        }
        // The actor value infos: four infos with seeded flags, every entry one of them.
        e.map(0x011d_6000, 0x1000);
        let mut infos = [0u32; 4];
        for info in &mut infos {
            *info = e.mem.alloc(0x50);
            let flags = self.pick2(&[0, 0x100, 0x200, 0x300]);
            e.mem.set_u32(*info + 0x44, flags);
        }
        for i in 0..0x100 {
            let info = infos[(self.next2() % 4) as usize];
            e.mem.set_u32(AV_INFOS + 4 * i, info);
        }
        // The old date's divisors (100.0 and 10000.0).
        e.map(0x0101_6000, 0x1000);
        e.map(0x0102_2000, 0x1000);
        e.mem.set_u32(0x0101_6410, 0x42c8_0000);
        e.mem.set_u32(0x0102_2958, 0x461c_4000);
        // A buffer of its own: the other process's vtable (its slot 0 answers a version), its change
        // flags (+0x17) and revert flags (+0x2c) with the modifiers' flag or without.
        let buffer = e.mem.alloc(0x40);
        let vt = e.mem.u32(self.objects[1]);
        e.mem.set_u32(buffer, vt);
        let changed = self.pick2(&[0, 0x0020_0000, 0x0030_0000]);
        e.mem.set_u32(buffer + 0x17, changed);
        let reverted = self.pick2(&[0, 0x0020_0000, 0x0030_0000]);
        e.mem.set_u32(buffer + 0x2c, reverted);
        self.buffer = buffer;
        // The save/load game's word at +0x48 (0: not loading a save).
        let loading = self.pick2(&[0, 0, 1]);
        e.mem.set_u32(self.objects[4] + 0x48, loading);
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

    /// One of these, resolved (from the second stream).
    fn one_of2(&mut self, from: &[u32]) -> u32 {
        let word = self.pick2(from);
        self.resolve(word)
    }

    /// The process and a buffer: one of the forms, the other process, or (two times in five) the
    /// buffer of its own, whose flags are seeded.
    fn low_buffer(&mut self) -> Vec<u32> {
        let pick = self.pick2(&[FORM_A, FORM_B, OTHER, 0, 0]);
        let buffer = if pick == 0 {
            self.buffer
        } else {
            self.resolve(pick)
        };
        vec![self.process(), buffer]
    }

    /// The process, an actor value code (one with a seeded info) and an actor (a form whose value
    /// owner is a vtable): `GetActorValue`'s words, the first unread.
    fn value_call(&mut self) -> Vec<u32> {
        let unread = self.word();
        let code = self.pick2(&[0, 1, 0x10, 0x4c, 0xff]);
        let actor = self.one_of2(&[FORM_A, FORM_B]);
        vec![self.process(), unread, code, actor]
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
            n @ 0..=4 => self.objects[n as usize],
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

    /// The process's embedded package.
    fn package(&self) -> u32 {
        self.objects[0] + 4
    }

    /// The process, then `n` seeded words.
    fn process_and(&mut self, n: usize) -> Vec<u32> {
        let mut words = vec![self.process()];
        for _ in 0..n {
            words.push(self.word());
        }
        words
    }

    /// The embedded package, then `n` seeded words.
    fn package_and(&mut self, n: usize) -> Vec<u32> {
        let mut words = vec![self.package()];
        for _ in 0..n {
            words.push(self.word());
        }
        words
    }

    /// The process (or its package with `package`) and a buffer.
    fn with_buffer(&mut self, package: bool) -> Vec<u32> {
        let this = if package {
            self.package()
        } else {
            self.process()
        };
        let buffer = self.one_of(&[FORM_A, FORM_B, OTHER]);
        vec![this, buffer]
    }
}

/// A test in the world: the families the code reaches on, the world made from each seed, the
/// call's words from `words`, and the callees done in place ignored.
trait InWorld {
    fn in_world(self, seeds: &'static [u32], words: fn(&mut World) -> Vec<u32>) -> Diff;
    /// [`InWorld::in_world`] with the far save buffer mapped too ([`FAR_BUFFER`]).
    fn in_far_world(self, seeds: &'static [u32], words: fn(&mut World) -> Vec<u32>) -> Diff;
}

impl InWorld for Diff {
    fn in_far_world(self, seeds: &'static [u32], words: fn(&mut World) -> Vec<u32>) -> Diff {
        self.in_world(seeds, words).setup(move |e, case| {
            let mut w = World::new(e, case.word(0));
            e.map(FAR_BUFFER, FAR_SIZE);
            words(&mut w)
        })
    }

    fn in_world(self, seeds: &'static [u32], words: fn(&mut World) -> Vec<u32>) -> Diff {
        self.family(&AI)
            .family(&leaf::LEAVES)
            .family(&AI_QUERIES)
            .family(&DISPATCH)
            .family(&MULTIBOUND)
            .family(&SAVE)
            .family(&CRT)
            .ignore_calls(IN_PLACE)
            // The forms family's form for an id (`LowProcess`'s loads ask it as a typed call): by
            // address here, to its double.
            .goes_by_address(&[0x0048_39c0])
            .inputs(&[Input::Of(seeds)])
            .setup(move |e, case| {
                let mut w = World::new(e, case.word(0));
                words(&mut w)
            })
    }
}

#[test]
fn answers_without_reading() {
    Diff::new(entry!(0x0040_1280, base_process::empty_name(Ptr) -> u32))
        .in_world(&FEW, |w| vec![w.word()])
        .run()
        .assert_same();
    Diff::new(entry!(0x008d_08f0, base_process::weapon_last_pos(Ptr) -> u32))
        .in_world(&FEW, |w| w.process_and(0))
        .run()
        .assert_same();
}

#[test]
fn floats() {
    Diff::new(entry!(0x008d_02d0, base_process::last_time_detected(Ptr, u32, u32) -> f32))
        .in_world(&MANY, |w| w.process_and(2))
        .run()
        .assert_same();
    Diff::new(entry!(0x008d_0390, base_process::float_zero_2(Ptr, u32, u32) -> f32))
        .in_world(&FEW, |w| w.process_and(2))
        .run()
        .assert_same();
    Diff::new(entry!(0x008d_0580, base_process::float_zero_3(Ptr, u32, u32, u32) -> f32))
        .in_world(&FEW, |w| w.process_and(3))
        .run()
        .assert_same();
    Diff::new(entry!(0x008d_10c0, base_process::breath_timer(Ptr) -> f32))
        .in_world(&MANY, |w| w.process_and(0))
        .run()
        .assert_same();
}

#[test]
fn stubs() {
    Diff::new(entry!(
        0x008d_02f0,
        base_process::false_6(Ptr, u32, u32, u32, u32, u32, u32) -> bool
    ))
    .in_world(&FEW, |w| w.process_and(6))
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_0300, base_process::false_2(Ptr, u32, u32) -> bool))
        .in_world(&FEW, |w| w.process_and(2))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008d_0340,
        base_process::zero_7(Ptr, u32, u32, u32, u32, u32, u32, u32) -> u32
    ))
    .in_world(&FEW, |w| w.process_and(7))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0350,
        base_process::nothing_7(Ptr, u32, u32, u32, u32, u32, u32, u32)
    ))
    .in_world(&FEW, |w| w.process_and(7))
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_0370, base_process::false_1(Ptr, u32) -> bool))
        .in_world(&FEW, |w| w.process_and(1))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008d_0380,
        base_process::nothing_11(Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32)
    ))
    .in_world(&FEW, |w| w.process_and(11))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0420,
        base_process::nothing_8(Ptr, u32, u32, u32, u32, u32, u32, u32, u32)
    ))
    .in_world(&FEW, |w| w.process_and(8))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_04f0,
        base_process::nothing_6(Ptr, u32, u32, u32, u32, u32, u32)
    ))
    .in_world(&FEW, |w| w.process_and(6))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0500,
        base_process::nothing_5(Ptr, u32, u32, u32, u32, u32)
    ))
    .in_world(&FEW, |w| w.process_and(5))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0510,
        base_process::false_4(Ptr, u32, u32, u32, u32) -> bool
    ))
    .in_world(&FEW, |w| w.process_and(4))
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_0520, base_process::false_3(Ptr, u32, u32, u32) -> bool))
        .in_world(&FEW, |w| w.process_and(3))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008d_0530,
        base_process::false_13(
            Ptr, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32
        ) -> bool
    ))
    .in_world(&FEW, |w| w.process_and(13))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0540,
        base_process::calculate_move_mode(Ptr, u32, u32, u32, u32, u32, u32) -> u32
    ))
    .in_world(&FEW, |w| w.process_and(6))
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_0560, base_process::zero_1(Ptr, u32) -> u32))
        .in_world(&FEW, |w| w.process_and(1))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008d_0570,
        base_process::minus_one_3(Ptr, u32, u32, u32) -> i32
    ))
    .in_world(&FEW, |w| w.process_and(3))
    .run()
    .assert_same();
}

#[test]
fn shared_point_and_sound_handle() {
    Diff::new(entry!(
        0x008d_0310,
        base_process::shared_point_copy(Ptr, Ptr, u32, u32) -> Ptr
    ))
    .in_world(&MANY, |w| {
        let result = w.one_of(&[OTHER, FORM_A, FORM_B]);
        vec![w.process(), result, w.word(), w.word()]
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_0590, base_process::sound_handle(Ptr, Ptr, u32) -> Ptr))
        .in_world(&FEW, |w| {
            let result = w.one_of(&[OTHER, FORM_A, FORM_B]);
            vec![w.process(), result, w.word()]
        })
        .run()
        .assert_same();
}

#[test]
fn current_package() {
    Diff::new(entry!(
        0x008d_03a0,
        base_process::clear_current_package(Ptr)
    ))
    .in_world(&MANY, |w| w.process_and(0))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_03c0,
        base_process::set_current_procedure_index(Ptr, u32)
    ))
    .in_world(&FEW, |w| w.process_and(1))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_03e0,
        base_process::current_procedure_index(Ptr) -> u32
    ))
    .in_world(&FEW, |w| w.process_and(0))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0400,
        base_process::add_to_current_procedure_index(Ptr, u32)
    ))
    .in_world(&FEW, |w| w.process_and(1))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0430,
        base_process::running_package_word(Ptr) -> u32
    ))
    .in_world(&FEW, |w| w.process_and(0))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_08b0,
        base_process::set_current_package(Ptr, u32, u32)
    ))
    .in_world(&MANY, |w| {
        let package = w.one_of(&[0, FORM_A, FORM_B]);
        vec![w.process(), package, w.word()]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0870,
        base_process::handle_deleted_reference(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.process_and(1))
    .run()
    .assert_same();
}

#[test]
fn own_slots() {
    Diff::new(entry!(
        0x008d_0450,
        base_process::procedure_index_running(Ptr) -> u32
    ))
    .in_world(&MANY, |w| w.process_and(0))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0470,
        base_process::set_procedure_index_running(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.process_and(1))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_04a0,
        base_process::add_to_procedure_index_running(Ptr, u32, u32)
    ))
    .in_world(&MANY, |w| w.process_and(2))
    .run()
    .assert_same();
}

#[test]
fn char_controller_and_destructor() {
    Diff::new(entry!(
        0x008d_04d0,
        base_process::set_char_controller(Ptr, u32)
    ))
    .in_world(&MANY, |w| {
        let controller = w.one_of(&[0, OTHER, FORM_B]);
        vec![w.process(), controller]
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_0640, base_process::destructor(Ptr)))
        .in_world(&MANY, |w| w.process_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008d_0610,
        base_process::scalar_deleting_destructor(Ptr, u32) -> Ptr
    ))
    .in_world(&MANY, |w| {
        let flags = w.pick(&[0, 1, 2, 3, 0xffff_ffff]);
        vec![w.process(), flags]
    })
    .run()
    .assert_same();
}

fn other_process(w: &mut World) -> Vec<u32> {
    let other = w.one_of(&[0, OTHER, OTHER, PROCESS]);
    vec![w.process(), other]
}

#[test]
fn copy() {
    Diff::new(entry!(0x008d_0660, base_process::copy(Ptr, Ptr)))
        .in_world(&MOST, other_process)
        .run()
        .assert_same();
}

#[test]
fn save_and_load() {
    Diff::new(entry!(0x008d_0900, base_process::save_size(Ptr, u32, u32) -> u16))
        .in_world(&MOST, |w| w.process_and(2))
        .ignore_calls(&[0x0084_e3a0])
        .run()
        .assert_same();
    Diff::new(entry!(0x008d_0a10, base_process::save_game(Ptr, u32, u32)))
        .in_world(&MOST, |w| w.process_and(2))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008d_0bc0,
        base_process::load_game(Ptr, u32, u32, u32)
    ))
    .in_world(&MOST, |w| w.process_and(3))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0e60,
        base_process::init_load_game(Ptr, u32, u32, u32)
    ))
    .in_world(&MANY, |w| w.process_and(3))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0e80,
        base_process::finish_init_load_game(Ptr, u32, u32, u32)
    ))
    .in_world(&MOST, |w| w.process_and(3))
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_0f10, base_process::revert(Ptr, u32, u32)))
        .in_world(&FEW, |w| w.process_and(2))
        .run()
        .assert_same();
}

#[test]
fn save_and_load_of_a_buffer() {
    Diff::new(entry!(
        0x008d_0f30,
        base_process::save_game_buffer(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.with_buffer(false))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0f90,
        base_process::load_game_buffer(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.with_buffer(false))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_1000,
        base_process::init_load_game_buffer(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.with_buffer(false))
    // The package's typed `InitLoadGame_ov2` reads flag 0 (`008c6b00`) in place.
    .ignore_calls(&[0x008c_6b00])
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_1030,
        base_process::cached_values_out_of_date(Ptr)
    ))
    .in_world(&FEW, |w| vec![w.one_of(&[PROCESS, OTHER, FORM_A])])
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_1050,
        base_process::finish_load_game(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.with_buffer(false))
    // The package's typed `FinishLoadGame` reads and clears flag 0 (`008c6b00`, `008c6920`) in place.
    .ignore_calls(&[0x008c_6b00, 0x008c_6920])
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_1070, base_process::revert_buffer(Ptr, u32)))
        .in_world(&MANY, |w| w.process_and(1))
        .run()
        .assert_same();
}

/// The change flags a save or load gets: none, the damage modifiers' (with `MiddleLowProcess`'s
/// temporary modifiers'), and all (the old block's flag too).
const SAVE_FLAGS: &[u32] = &[0, 0x0020_0000, 0x0030_0000, 0xffff_ffff];

#[test]
fn low_process_values_and_life() {
    Diff::new(entry!(
        0x0090_72d0,
        low_process::get_actor_value(Ptr, u32, u32, Ptr) -> i32
    ))
    .in_world(&MOST, World::value_call)
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0090_7380,
        low_process::get_actor_float_value(Ptr, u32, u32, Ptr) -> f64
    ))
    // The sums of two NaNs: SSE keeps the first one's payload, x87's FADD the larger one's.
    .in_world(&MOST, World::value_call)
    .ignore_calls(LOW_IN_PLACE)
    .allow_nan_payloads()
    .run()
    .assert_same();
    Diff::new(entry!(0x0090_6fe0, low_process::destructor(Ptr)))
        .in_world(&MANY, |w| w.process_and(0))
        .ignore_calls(LOW_IN_PLACE)
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0090_6fb0,
        low_process::scalar_deleting_destructor(Ptr, u32) -> Ptr
    ))
    .in_world(&MANY, |w| {
        let flags = w.pick(&[0, 1, 2, 3, 0xffff_ffff]);
        vec![w.process(), flags]
    })
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
    // `MiddleLowProcess`'s, which call them first.
    Diff::new(entry!(
        0x0092_ce00,
        middle_low_process::get_actor_float_value(Ptr, u32, u32, u32) -> f64
    ))
    .in_world(&MOST, World::value_call)
    .ignore_calls(LOW_IN_PLACE)
    .allow_nan_payloads()
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_cd90,
        middle_low_process::get_actor_value(Ptr, u32, u32, u32) -> i32
    ))
    .in_world(&MOST, World::value_call)
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
    Diff::new(entry!(0x0092_ca00, middle_low_process::destructor(Ptr)))
        .in_world(&MANY, |w| w.process_and(0))
        .ignore_calls(LOW_IN_PLACE)
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0092_c9d0,
        middle_low_process::scalar_deleting_destructor(Ptr, u32) -> Ptr
    ))
    .in_world(&MANY, |w| w.process_and(1))
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
}

/// The process, change flags and a word.
fn save_words(w: &mut World) -> Vec<u32> {
    let flags = w.pick2(SAVE_FLAGS);
    vec![w.process(), flags, w.word()]
}

/// The process, change flags and two words.
fn load_words(w: &mut World) -> Vec<u32> {
    let flags = w.pick2(SAVE_FLAGS);
    vec![w.process(), flags, w.word(), w.word()]
}

/// The process and a revert's change flags and form (a form whose flag word `00440d80` reads).
fn revert_words(w: &mut World) -> Vec<u32> {
    let flags = w.pick2(SAVE_FLAGS);
    let form = w.one_of2(&[FORM_A, FORM_B, OTHER]);
    vec![w.process(), flags, form]
}

#[test]
fn low_process_save_and_load() {
    // `BaseProcess::GetSaveSize` reads its forms' ids (`0084e3a0`) in place.
    Diff::new(entry!(
        0x0090_f8b0,
        low_process::get_save_size(Ptr, u32, u32) -> u16
    ))
    .in_world(&MOST, save_words)
    .ignore_calls(LOW_IN_PLACE)
    .ignore_calls(&[0x0084_e3a0])
    .run()
    .assert_same();
    Diff::new(entry!(0x0090_fa70, low_process::save_game(Ptr, u32, u32)))
        .in_world(&MOST, save_words)
        .ignore_calls(LOW_IN_PLACE)
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0090_fd80,
        low_process::load_game(Ptr, u32, u32, u32)
    ))
    .in_world(&MOST, load_words)
    .ignore_calls(LOW_IN_PLACE)
    .allow_nan_payloads()
    .run()
    .assert_same();
    Diff::new(entry!(0x0091_0310, low_process::revert(Ptr, u32, u32)))
        .in_world(&MANY, revert_words)
        .ignore_calls(LOW_IN_PLACE)
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0092_e420,
        middle_low_process::get_save_size(Ptr, u32, u32) -> u16
    ))
    .in_world(&MANY, save_words)
    .ignore_calls(LOW_IN_PLACE)
    .ignore_calls(&[0x0084_e3a0])
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_e560,
        middle_low_process::save_game(Ptr, u32, u32)
    ))
    .in_far_world(&MOST, save_words)
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_e730,
        middle_low_process::load_game(Ptr, u32, u32, u32)
    ))
    .in_world(&MOST, load_words)
    .ignore_calls(LOW_IN_PLACE)
    .allow_nan_payloads()
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_e9f0,
        middle_low_process::revert(Ptr, u32, u32)
    ))
    .in_world(&MANY, revert_words)
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
}

#[test]
fn low_process_save_and_load_of_a_buffer() {
    Diff::new(entry!(0x0091_0450, low_process::save_game_buffer(Ptr, Ptr)))
        .in_world(&MOST, World::low_buffer)
        .ignore_calls(LOW_IN_PLACE)
        .run()
        .assert_same();
    Diff::new(entry!(0x0091_0630, low_process::load_game_buffer(Ptr, u32)))
        .in_world(&MOST, World::low_buffer)
        .ignore_calls(LOW_IN_PLACE)
        .run()
        .assert_same();
    Diff::new(entry!(0x0091_0a60, low_process::revert_buffer(Ptr, u32)))
        .in_world(&MANY, World::low_buffer)
        .ignore_calls(LOW_IN_PLACE)
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0092_ea30,
        middle_low_process::save_game_buffer(Ptr, Ptr)
    ))
    .in_world(&MANY, World::low_buffer)
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_eaa0,
        middle_low_process::load_game_buffer(Ptr, u32)
    ))
    .in_world(&MANY, World::low_buffer)
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0092_eb00,
        middle_low_process::revert_buffer(Ptr, u32)
    ))
    .in_world(&MANY, World::low_buffer)
    .ignore_calls(LOW_IN_PLACE)
    .run()
    .assert_same();
}

#[test]
fn package_record() {
    Diff::new(entry!(0x008c_5e70, actor_package::destroy_data(Ptr)))
        .in_world(&MANY, |w| w.package_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008c_5ec0,
        actor_package::handle_deleted_reference(Ptr, u32)
    ))
    .in_world(&MANY, |w| {
        let deleted = w.one_of(&[0, PROCESS, FORM_A, 0x1234]);
        vec![w.package(), deleted]
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x008c_5ef0, actor_package::clear(Ptr)))
        .in_world(&MOST, |w| w.package_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(0x008c_6030, actor_package::dec_ref_count(Ptr) -> u32))
        .in_world(&FEW, |w| vec![w.one_of(&[FORM_A, FORM_B])])
        .run()
        .assert_same();
    Diff::new(entry!(0x008c_6100, actor_package::inc_ref_count(Ptr) -> u32))
        .in_world(&FEW, |w| vec![w.one_of(&[FORM_A, FORM_B])])
        .run()
        .assert_same();
    Diff::new(entry!(0x008c_6060, actor_package::null_instance_data(Ptr)))
        .in_world(&FEW, |w| w.package_and(0))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008c_6080,
        actor_package::init_from_package(Ptr, Ptr, u32)
    ))
    .in_world(&MOST, |w| {
        let package = w.one_of(&[0, FORM_A, FORM_B]);
        vec![w.package(), package, w.word()]
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x008c_6130, actor_package::move_to(Ptr, Ptr)))
        .in_world(&MANY, |w| {
            let destination = w.objects[1] + 4;
            vec![w.package(), destination]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x008c_6920, actor_package::set_flag(Ptr, u32, u8)))
        .in_world(&MANY, |w| {
            let bit = w.pick(&[0, 1, 5, 31, 32, 33]);
            let set = w.pick(&[0, 1, 0x100, 0xff]);
            vec![w.package(), bit, set]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x008c_6b00, actor_package::flag(Ptr, u32) -> u32))
        .in_world(&MANY, |w| {
            let bit = w.pick(&[0, 1, 5, 8, 31, 32]);
            vec![w.package(), bit]
        })
        .run()
        .assert_same();
}

#[test]
fn package_save_and_load() {
    Diff::new(entry!(0x008c_61a0, actor_package::save_size(Ptr, u32) -> u32))
        .in_world(&MANY, |w| w.package_and(1))
        .ignore_calls(&[0x0084_e3a0])
        .run()
        .assert_same();
    Diff::new(entry!(0x008c_6250, actor_package::save(Ptr, u32)))
        .in_world(&MANY, |w| w.package_and(1))
        .run()
        .assert_same();
    Diff::new(entry!(0x008c_6320, actor_package::load_game(Ptr, u32, u32)))
        .in_world(&MOST, |w| w.package_and(2))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008c_64d0,
        actor_package::init_load_game(Ptr, u32, u32)
    ))
    .in_world(&MANY, |w| w.package_and(2))
    .run()
    .assert_same();
    Diff::new(entry!(0x008c_65a0, actor_package::revert_unused(Ptr, u32)))
        .in_world(&FEW, |w| w.package_and(1))
        .run()
        .assert_same();
    Diff::new(entry!(0x008c_6c80, actor_package::revert(Ptr, u32)))
        .in_world(&MANY, |w| w.package_and(1))
        .run()
        .assert_same();
}

#[test]
fn package_save_and_load_of_a_buffer() {
    Diff::new(entry!(0x008c_65d0, actor_package::save_game(Ptr, Ptr)))
        .in_world(&MOST, |w| w.with_buffer(true))
        .run()
        .assert_same();
    Diff::new(entry!(
        0x008c_6730,
        actor_package::load_game_buffer(Ptr, Ptr)
    ))
    .in_world(&MOST, |w| w.with_buffer(true))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008c_6970,
        actor_package::init_load_game_buffer(Ptr, Ptr)
    ))
    .in_world(&MOST, |w| w.with_buffer(true))
    // The typed version reads flag 0 (`008c6b00`) in place.
    .ignore_calls(&[0x008c_6b00])
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008c_6b20,
        actor_package::finish_load_game(Ptr, Ptr)
    ))
    .in_world(&MOST, |w| w.with_buffer(true))
    // The typed version reads and clears flag 0 (`008c6b00`, `008c6920`) in place.
    .ignore_calls(&[0x008c_6b00, 0x008c_6920])
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
        log_warnings(&mut e);
        e.map(FAR_BUFFER, FAR_SIZE);
        let args = words(&mut w);
        // Before the call: the receiver's first eight words, the player's running package and the
        // save/load game's position.
        let mut before: Vec<u32> = (0..8).map(|i| e.mem.u32(args[0] + 4 * i)).collect();
        before.push(e.mem.u32(e.mem.u32(PLAYER) + 0x208));
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

fn slot_calls(e: &Engine, slot: u32) -> u32 {
    e.mem.u32(SLOT_COUNTS + slot)
}

/// The SAVELOAD warnings' double, made again for a reach run: as the world's (its block's count and
/// words), and each call's first three words kept at `WARNINGS` too (a count, then three words each,
/// up to 16 calls), so a row can tell one function's warnings by the source file they name.
const WARNINGS: u32 = CONTROL + 0x1c00;
fn log_warnings(e: &mut Engine) {
    const WARNING: u32 = 0x005b_5e40;
    let block = block(double(WARNING));
    e.register_double(WARNING, move |e, a| {
        let n = e.mem.u32(block);
        e.mem.set_u32(block, n + 1);
        let word = |i: usize| a.get(i).copied().unwrap_or(0);
        if n == 0 {
            e.mem.set_u32(block + 0x2c, word(0));
        }
        e.mem.set_u32(block + 0x24, word(0));
        e.mem.set_u32(block + 0x28, word(1));
        e.mem.set_u32(block + 0x30, word(2));
        let logged = e.mem.u32(WARNINGS);
        if logged < 16 {
            for i in 0..3 {
                e.mem
                    .set_u32(WARNINGS + 4 + 12 * logged + 4 * i as u32, word(i));
            }
            e.mem.set_u32(WARNINGS, logged + 1);
        }
        ret(e.mem.u32(block + 4 + 4 * (n % 8)))
    });
}

/// Whether a logged warning had `format` first and `source` as its word `at` (1 for the header and
/// size warnings, 2 for the end's, after the bytes).
fn warned_from(e: &Engine, format: u32, at: u32, source: u32) -> bool {
    (0..e.mem.u32(WARNINGS)).any(|k| {
        let call = WARNINGS + 4 + 12 * k;
        e.mem.u32(call) == format && e.mem.u32(call + 4 * at) == source
    })
}

/// The worlds the seeds make reach the deep branches, checked on the translations (so a change to
/// the world that loses one shows here, not as a weaker comparison).
#[test]
fn the_seeds_reach_every_branch() {
    const WARNING: u32 = 0x005b_5e40;
    low_process_reaches();
    reaches(
        0x008d_0660,
        &MOST,
        other_process,
        &["high reset", "level copy", "cached copy"],
        |e, _, _| {
            vec![
                slot_calls(e, 0x214) > 0,
                [0x008d_9fa0, 0x0091_4c60, 0x0093_7160, 0x0090_7510]
                    .iter()
                    .any(|a| calls(e, *a) > 0),
                calls(e, 0x0086_75e0) > 0,
            ]
        },
    );
    reaches(
        0x008d_0a10,
        &MOST,
        |w| w.process_and(2),
        &["blocks", "too large", "exactly 0xffff"],
        |e, _, before| {
            vec![
                calls(e, 0x0085_79b0) > 3,
                calls(e, WARNING) > 0 && last_word(e, WARNING) == 0x0101_5318,
                calls(e, 0x0085_79b0) > 3
                    && calls(e, WARNING) == 0
                    && e.mem.u16(before[9] + 4) == 0xffff,
            ]
        },
    );
    reaches(
        0x008d_0bc0,
        &MOST,
        |w| w.process_and(3),
        &[
            "bad header, a record",
            "bad header, the version",
            "a record's name, the default slot",
            "a record's name, its own slot",
            "overrun, a record",
            "underrun, a record",
            "overrun, the version",
            "underrun, the version",
            "the block's end",
        ],
        |e, _, _| {
            let warned = |format| calls(e, WARNING) > 0 && last_word(e, WARNING) == format;
            vec![
                calls(e, WARNING) > 0 && first_word(e, WARNING) == 0x0101_5718,
                calls(e, WARNING) > 0 && first_word(e, WARNING) == 0x0101_56a8,
                calls(e, 0x0048_39c0) > 0 && calls(e, WARNING) > 0 && slot_calls(e, 0x130) == 0,
                calls(e, 0x0048_39c0) > 0 && calls(e, WARNING) > 0 && slot_calls(e, 0x130) > 0,
                warned(0x0101_5588),
                warned(0x0101_5500),
                warned(0x0101_54a0),
                warned(0x0101_5440),
                calls(e, 0x0085_79e0) > 3 && calls(e, WARNING) == 0,
            ]
        },
    );
    reaches(
        0x008d_0e80,
        &MOST,
        |w| w.process_and(3),
        &["evaluated"],
        |e, _, _| vec![calls(e, 0x008a_6ce0) > 0],
    );
    reaches(
        0x008c_5ef0,
        &MOST,
        |w| w.package_and(0),
        &["player's package", "package deleted", "data deleted"],
        |e, _, before| {
            let player = e.mem.u32(PLAYER);
            vec![
                e.mem.u32(player + 0x208) == 0 && before[8] != 0,
                slot_calls(e, 0x10) > 0,
                slot_calls(e, 0x0) > 0,
            ]
        },
    );
    reaches(
        0x008c_6080,
        &MOST,
        |w| {
            let package = w.one_of(&[0, FORM_A, FORM_B]);
            vec![w.package(), package, w.word()]
        },
        &["started"],
        |e, _, _| vec![calls(e, 0x008c_6ca0) > 0],
    );
    reaches(
        0x008c_6320,
        &MOST,
        |w| w.package_and(2),
        &["procedure type", "no package", "no plugin package"],
        |e, _, _| {
            let logged = |format| calls(e, WARNING) > 0 && last_word(e, WARNING) == format;
            vec![
                calls(e, 0x0067_77b0) > 0,
                logged(0x0108_54b0),
                logged(0x0108_5458),
            ]
        },
    );
    reaches(
        0x008c_64d0,
        &MANY,
        |w| w.package_and(2),
        &["lowered"],
        |e, args, before| vec![e.mem.u32(args[0] + 0xc) != before[3]],
    );
    reaches(
        0x008c_65d0,
        &MOST,
        |w| w.with_buffer(true),
        &[
            "package's own save",
            "data's own save",
            "the first created id",
            "the last plugin id",
        ],
        |e, _, before| {
            let id = if before[0] != 0 {
                e.mem.u32(before[0] + 0xc)
            } else {
                0
            };
            vec![
                slot_calls(e, 0x54) > 0,
                slot_calls(e, 0xc) > 0,
                id == 0xff00_0000 && slot_calls(e, 0x15c) > 0,
                id == 0xfeff_ffff && slot_calls(e, 0x15c) == 0,
            ]
        },
    );
    reaches(
        0x008c_6730,
        &MOST,
        |w| w.with_buffer(true),
        &[
            "created",
            "marked",
            "data loaded",
            "data deleted",
            "cleared",
        ],
        |e, args, _| {
            vec![
                calls(e, 0x0084_aa30) > 0,
                e.mem.u32(args[0] + 0x14) & 1 != 0 && calls(e, 0x0067_0b90) == 0,
                slot_calls(e, 0x10) > 0,
                slot_calls(e, 0x0) > 0,
                e.mem.u32(args[0] + 0xc) == 0xffff_ffff,
            ]
        },
    );
    reaches(
        0x008c_6970,
        &MOST,
        |w| w.with_buffer(true),
        &["looked up", "own init", "procedure type", "targeted"],
        |e, _, _| {
            vec![
                calls(e, 0x0084_aa90) > 0,
                slot_calls(e, 0x64) > 0,
                calls(e, 0x0067_77b0) > 0,
                calls(e, 0x0056_4db0) > 0,
            ]
        },
    );
    reaches(
        0x008c_6b20,
        &MOST,
        |w| w.with_buffer(true),
        &["own finish", "type changed"],
        |e, _, _| vec![slot_calls(e, 0x84) > 0, calls(e, 0x0084_5cc0) > 0],
    );
}

/// `LowProcess`'s part 4 and the `MiddleLowProcess` functions that call it first: the deep branches,
/// told apart from `BaseProcess`'s by the source file their warnings name.
fn low_process_reaches() {
    const LOW: u32 = 0x0108_8f50;
    const MIDDLE_LOW: u32 = 0x0108_a3f0;
    const GET_MODIFIER: u32 = 0x0093_7730;
    reaches(
        0x0090_72d0,
        &MOST,
        World::value_call,
        &["no modifiers", "with the modifiers"],
        |e, _, _| vec![calls(e, GET_MODIFIER) == 0, calls(e, GET_MODIFIER) == 2],
    );
    reaches(
        0x0092_ce00,
        &MOST,
        World::value_call,
        &["no modifiers", "with the modifiers"],
        |e, _, _| vec![calls(e, GET_MODIFIER) == 0, calls(e, GET_MODIFIER) == 3],
    );
    reaches(
        0x0090_6fb0,
        &MANY,
        |w| {
            let flags = w.pick(&[0, 1]);
            vec![w.process(), flags]
        },
        &[
            "kept",
            "freed",
            "an acquire record deleted",
            "the record to acquire deleted",
        ],
        |e, args, _| {
            vec![
                args[1] == 0,
                args[1] == 1 && calls(e, 0x0040_1030) > 0,
                calls(e, 0x007b_3fa0) > 0,
                slot_calls(e, 0x10) > 0,
            ]
        },
    );
    reaches(
        0x0090_f8b0,
        &MOST,
        save_words,
        &["the modifiers' size"],
        |e, _, _| vec![calls(e, 0x0093_7ab0) > 0],
    );
    reaches(
        0x0090_fa70,
        &MOST,
        save_words,
        &["its block too large", "the modifiers saved"],
        |e, _, _| {
            vec![
                warned_from(e, 0x0101_5318, 1, LOW),
                calls(e, 0x0093_78d0) > 0,
            ]
        },
    );
    reaches(
        0x0090_fd80,
        &ALL,
        load_words,
        &[
            "bad header",
            "overrun, a record",
            "underrun, a record",
            "overrun, the version",
            "underrun, the version",
            "the old date",
            "the modifiers loaded",
        ],
        |e, _, _| {
            vec![
                warned_from(e, 0x0101_5718, 1, LOW) || warned_from(e, 0x0101_56a8, 1, LOW),
                warned_from(e, 0x0101_5588, 2, LOW),
                warned_from(e, 0x0101_5500, 2, LOW),
                warned_from(e, 0x0101_54a0, 2, LOW),
                warned_from(e, 0x0101_5440, 2, LOW),
                calls(e, 0x004b_1500) > 0,
                calls(e, 0x0093_79c0) > 0,
            ]
        },
    );
    reaches(
        0x0091_0310,
        &MANY,
        revert_words,
        &[
            "not loading: cleared",
            "loading: kept",
            "the modifiers reverted",
        ],
        |e, _, _| {
            vec![
                calls(e, 0x0047_0470) > 0,
                calls(e, 0x0047_0470) == 0,
                calls(e, 0x0093_7b30) > 0,
            ]
        },
    );
    reaches(
        0x0091_0450,
        &MOST,
        World::low_buffer,
        &["no locations", "locations counted", "the modifiers saved"],
        |e, _, _| {
            let count = e.mem.u32(block(double(0x0086_5ff0)) + 0x28);
            vec![count == 0, count > 0, calls(e, 0x0093_7b50) > 0]
        },
    );
    reaches(
        0x0091_0630,
        &MOST,
        World::low_buffer,
        &["locations added", "the modifiers loaded"],
        |e, _, _| vec![calls(e, 0x005a_e3d0) > 0, calls(e, 0x0093_7c60) > 0],
    );
    reaches(
        0x0091_0a60,
        &MANY,
        World::low_buffer,
        &["the modifiers reverted"],
        |e, _, _| vec![calls(e, 0x0093_7b30) > 0],
    );
    reaches(
        0x0092_e560,
        &MOST,
        save_words,
        &["its block too large"],
        |e, _, _| vec![warned_from(e, 0x0101_5318, 1, MIDDLE_LOW)],
    );
    reaches(
        0x0092_e730,
        &ALL,
        load_words,
        &[
            "bad header",
            "overrun, a record",
            "underrun, a record",
            "overrun, the version",
            "underrun, the version",
        ],
        |e, _, _| {
            vec![
                warned_from(e, 0x0101_5718, 1, MIDDLE_LOW)
                    || warned_from(e, 0x0101_56a8, 1, MIDDLE_LOW),
                warned_from(e, 0x0101_5588, 2, MIDDLE_LOW),
                warned_from(e, 0x0101_5500, 2, MIDDLE_LOW),
                warned_from(e, 0x0101_54a0, 2, MIDDLE_LOW),
                warned_from(e, 0x0101_5440, 2, MIDDLE_LOW),
            ]
        },
    );
}
