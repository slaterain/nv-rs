//! The interface family's shared world for its differential tests: one setup for every function,
//! built from the case's seed the same way on both engines.
//!
//! - **The arena**: [`OBJECTS`] objects of [`OBJECT`] bytes. An object's first word is one of the
//!   [`VTABLES`] vtables, whose every slot is the address of a logging stand-in ([`CODE`]), so a
//!   virtual call on any object reaches one. Its other words are, by a seeded generator, zero, a small
//!   number, a float, a vtable, a stand-in's address, or a pointer to a LATER object: a chain of
//!   pointers always ends (at most [`OBJECTS`] steps), so a walk of a list never loops.
//! - **The exe's data**: the `.rdata` and `.data` pages the menus' code reads ([`DATA`]), filled by the
//!   same generator (their pointers reach any object), with the words the native code reads in
//!   place set per seed (`known_words`: the manager, its enter stack and mode, the menus by class,
//!   the controls object, the renderer's and the camera's values).
//! - **The callees**: every function the tested code calls by address is a logging stand-in: it
//!   writes its address and words to the log page ([`LOG`]), so the memory compare sees them as well
//!   as the harness's call log, and answers a word, a float in ST0 and a word in EDX from a sequence
//!   (by the call's number and the seed), the same on both sides. The functions the native code does
//!   in place (`in_place::ADDRESSES`) are no stand-ins: their translations run on the translation's
//!   side, reading the same words, and their calls are left out of its log.
//!
//! What it can't see: behaviour that needs real objects (a callee's real effect, a vtable slot
//! holding a typed function), which the files' own tests in `engine-fallout-interface` keep.

use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};

/// The arena's first object.
pub const ARENA: u32 = 0x0500_0000;
/// Objects and their size.
pub const OBJECTS: u32 = 256;
pub const OBJECT: u32 = 0x200;
/// The zero bytes mapped after the last object.
pub const ARENA_PAD: u32 = 0x1_0000;
/// The vtables: [`VTABLE_COUNT`] of 0x400 bytes, each slot a stand-in.
pub const VTABLES: u32 = 0x0504_0000;
pub const VTABLE_COUNT: u32 = 8;
/// The stand-ins' own code addresses (no exe function lives there).
pub const CODE: u32 = 0x7d00_0000;
pub const CODE_COUNT: u32 = 0x100;
/// The log page: its first word counts the words written after it.
pub const LOG: u32 = 0x0510_0000;
const LOG_SIZE: u32 = 0x4_0000;
/// The exe's data the tested code reads: `.rdata`, and the part of `.data`/`.bss` the menus' globals
/// live in.
pub const DATA: [(u32, u32); 2] = [(0x0100_0000, 0x10_0000), (0x0118_0000, 0x10_0000)];

/// The world's seeds: each case builds the world from one.
pub const SEEDS: &[u32] = &[1, 2, 3, 4];

/// The `k`th object (wrapping).
pub fn object(k: u32) -> u32 {
    ARENA + OBJECT * (k % OBJECTS)
}

fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// The floats a word may hold.
const FLOATS: [f32; 6] = [1.0, 0.5, 2.0, -1.0, 100.0, 0.25];

/// The generator's word for `at` (an object's word: pointers only to objects after `first_later`).
fn word(seed: u32, at: u32, first_later: u32) -> u32 {
    let h = mix(u64::from(seed) << 32 | u64::from(at));
    let pick = (h >> 40) as u32;
    match h % 100 {
        0..=34 => 0,
        35..=54 => 1 + pick % 4,
        55..=74 => {
            if first_later >= OBJECTS {
                0
            } else {
                object(first_later + pick % (OBJECTS - first_later))
            }
        }
        75..=84 => FLOATS[(pick % 6) as usize].to_bits(),
        85..=94 => VTABLES + 0x400 * (pick % VTABLE_COUNT),
        _ => CODE + 4 * (pick % CODE_COUNT),
    }
}

/// What the log holds for a word that points into the stack for locals: the harness compares those
/// words itself (by their offset in their frame, or as the local they are), and a local the native
/// code places differently has another address.
const STACK_WORD: u32 = 0x5ac0_0000;

/// Appends `words` to the log under `tag` (a stack address as [`STACK_WORD`]).
fn log(e: &mut Engine, tag: u32, words: &[u32]) {
    let stack = e.guest_stack_region();
    let n = e.mem.u32(LOG);
    if 4 * (n + 2 + words.len() as u32) >= LOG_SIZE - 16 {
        return;
    }
    e.mem.set_u32(LOG + 4 + 4 * n, tag);
    e.mem.set_u32(LOG + 8 + 4 * n, words.len() as u32);
    for (i, &w) in (2u32..).zip(words) {
        let w = match stack {
            Some((low, end)) if (low..end).contains(&w) => STACK_WORD,
            _ => w,
        };
        e.mem.set_u32(LOG + 4 + 4 * (n + i), w);
    }
    e.mem.set_u32(LOG, n + 2 + words.len() as u32);
}

/// The log page's last words: the number of stand-in calls so far, and the world's seed.
const CALLS: u32 = LOG + LOG_SIZE - 4;
const SEED: u32 = LOG + LOG_SIZE - 8;

/// A logging stand-in at `at`: its answer by the number of calls so far and the world's seed (words on
/// the log page), so the same call answers differently in each seed's world: a function that leaves
/// early on its first callee's answer goes on in another seed.
fn stand_in(e: &mut Engine, at: u32) {
    e.register_double(at, move |e, words| {
        let calls = e.mem.u32(CALLS);
        e.mem.set_u32(CALLS, calls + 1);
        log(e, at, words);
        let n = calls
            .wrapping_mul(7)
            .wrapping_add(at >> 2)
            .wrapping_add(e.mem.u32(SEED));
        let eax = match n % 5 {
            0 => 0,
            1 => 1,
            2 => object(n / 5),
            3 => 2 + n % 3,
            _ => object(n / 5 + 17),
        };
        Ret {
            eax,
            edx: n % 3,
            st0: f64::from(FLOATS[(n % 6) as usize]),
        }
    });
}

/// Builds the world of `seed`, with stand-ins at every address of `callees`.
pub fn world(e: &mut Engine, seed: u32, callees: &[u32], without: &[u32]) {
    // The first 64 KB and the pages a float word or a stand-in's address points at, as zeros: a null or
    // small pointer, or a float read as one, reads zeros on both sides instead of stopping both.
    for base in [0u32, CODE]
        .into_iter()
        .chain(FLOATS.iter().map(|f| f.to_bits() & 0xffff_0000))
    {
        e.map(base, 0x1_0000);
    }
    e.map(ARENA, OBJECTS * OBJECT);
    // Past the last object, zeros: a field read at a large offset of a late object (the controls'
    // byte table at +0x1b94) reads zero instead of stopping both sides.
    e.map(ARENA + OBJECTS * OBJECT, ARENA_PAD);
    e.map(VTABLES, 0x400 * VTABLE_COUNT);
    e.map(LOG, LOG_SIZE);
    e.mem.set_u32(SEED, seed);
    for k in 0..OBJECTS {
        let base = object(k);
        e.mem.set_u32(base, VTABLES + 0x400 * (k % VTABLE_COUNT));
        for at in (base + 4..base + OBJECT).step_by(4) {
            e.mem.set_u32(at, word(seed, at, k + 1));
        }
    }
    for at in (VTABLES..VTABLES + 0x400 * VTABLE_COUNT).step_by(4) {
        e.mem
            .set_u32(at, CODE + 4 * ((at - VTABLES) / 4 % CODE_COUNT));
    }
    for (base, size) in DATA {
        e.map(base, size);
        for at in (base..base + size).step_by(4) {
            e.mem.set_u32(at, word(seed, at, 0));
        }
    }
    for k in 0..CODE_COUNT {
        stand_in(e, CODE + 4 * k);
    }
    // A virtual call on a word that is no object (a small number, a float, a stand-in's address) reads
    // its slot from a zero page and calls address 0: a stand-in there too, so both sides go on.
    stand_in(e, 0);
    known_words(e, seed);
    for &at in callees {
        if without.contains(&at) {
            continue;
        }
        match at {
            CONTROLS_POLL => controls_poll(e),
            _ => stand_in(e, at),
        }
    }
}

/// `00a23010` (`Controls::Poll`), the call each round of a message box's wait loop (`00704010`)
/// makes: from its third call in a run on, the message box's answer byte (`0119f349`) holds 1, as
/// the message's callback (`00704130`) stores it once the player answers, so the loop runs a few
/// rounds and ends.
const CONTROLS_POLL: u32 = 0x00a2_3010;
const MESSAGE_RESULT: u32 = 0x0119_f349;
const POLLS: u32 = LOG + LOG_SIZE - 12;

fn controls_poll(e: &mut Engine) {
    e.register_double(CONTROLS_POLL, |e, words| {
        log(e, CONTROLS_POLL, words);
        let polls = e.mem.u32(POLLS) + 1;
        e.mem.set_u32(POLLS, polls);
        if polls >= 3 {
            e.mem.set_u8(MESSAGE_RESULT, 1);
        }
        Ret::default()
    });
}

/// The exe's words the native code reads in place (the manager singleton, the menus by class, the
/// controls object, the renderer and camera values) set per seed, so each branch is reached: the
/// manager ready with the enter stack's placeholder (1) on top and the mode 3 (the five candidate
/// menus tried) in seed 1, ready with a full enter stack and the mode 2 in seed 2, there but not
/// ready in seed 3, none in seed 4. The translation reads the same words through its callees.
fn known_words(e: &mut Engine, seed: u32) {
    let manager = if seed == 4 { 0 } else { object(7 + 31 * seed) };
    e.mem.set_u32(SINGLETON, manager);
    if manager != 0 {
        e.mem.set_u8(manager, u8::from(seed != 3));
        let stack: &[u32] = match seed {
            1 => &[0x3ea, 1],
            2 => &[
                0x3ea, 0x3eb, 0x40b, 0x425, 0x3ff, 0x3f1, 0x3f4, 0x420, 0x41d, 0x435,
            ],
            _ => &[0x3eb],
        };
        for i in 0..10 {
            let entry = stack.get(i).copied().unwrap_or(0);
            e.mem.set_u32(manager + 0x114 + 4 * i as u32, entry);
        }
        e.mem
            .set_u32(manager + 0x4bc, if seed == 1 { 3 } else { seed });
        e.mem.set_u32(manager + 0x0c, if seed == 1 { 2 } else { 1 });
        e.mem.set_u8(manager + 0x7c, u8::from(seed == 1));
        e.mem.set_u8(manager + 0xdd, u8::from(seed == 1));
        // The menus root (+0x9c) with a list to walk (its +4: the head node; each node's first word
        // the next, its +8 the item).
        if seed != 3 {
            e.mem.set_u32(manager + 0x9c, object(60));
            e.mem.set_u32(object(60) + 4, object(61));
        }
    }
    // The menus by class: one object's words, every id but in seed 2 (only the first 0x16: the
    // map's, 0x3ff, is the first past the count); the placeholder's five candidates exist in seed 1.
    e.mem.set_u32(MENU_TABLE, object(200));
    e.mem.set_u32(object(200) + 4 * 0x16, object(130));
    e.mem
        .set_u16(MENU_COUNT, if seed == 2 { 0x16 } else { 0x54 });
    if seed == 1 {
        for (k, id) in [0x3eb_u32, 0x3ea, 0x40b, 0x425, 0x3ff]
            .into_iter()
            .enumerate()
        {
            e.mem
                .set_u32(object(200) + 4 * (id - 0x3e9), object(120 + k as u32));
        }
    }
    e.mem.set_u32(CONTROLS, object(20));
    // What 007031e0's unused line number reads through (in the translation only).
    e.mem.set_u32(0x011f_33cc, 2);
    e.mem.set_u32(0x011f_33f8, object(150));
    // The renderer and the camera (00709d50): the flag but in seed 3; the screen; the exe's
    // constants (the scale 0.75, pi/180 and 1.1 as floats widened, the limit 88.2 degrees, the far
    // plane 5000); the field of view setting by seed.
    e.mem.set_u8(0x011c_70ed, u8::from(seed != 3));
    let (width, height, fov) = match seed {
        1 => (1280.0f32, 720.0f32, 75.0f32),
        2 => (1920.0, 1080.0, 90.0),
        3 => (1024.0, 768.0, 60.0),
        _ => (800.0, 600.0, -0.0),
    };
    e.mem.set_f32(0x0118_948c, width);
    e.mem.set_f32(0x0118_9490, height);
    e.mem.set_f32(0x0120_3154, fov);
    e.mem.set_u32(0x0101_6264, 0x3f40_0000);
    for (at, bits) in [
        (0x0102_3128, 0x3c8e_fa35_u32),
        (0x0102_90f8, 0x3f8c_cccd),
        (0x0106_ede0, 0x3fc5_0a6b),
    ] {
        e.mem.set_u64(at, f64::from(f32::from_bits(bits)).to_bits());
    }
    // Seed 4: the limit +0.0 against the product -0.0 (the field of view -0.0): equal, so the
    // limit is taken, and the angle's sign tells a `<=` from the exe's `<`.
    if seed == 4 {
        e.mem.set_u64(0x0106_ede0, 0.0f64.to_bits());
    }
    e.mem.set_u32(0x0103_0020, 0x459c_4000);
    // The menu manager's instance (none in seeds 1 and 3, so `Instance` makes one), the stats
    // menu's body part controller (none in seed 2), and `GetFrontmostMenu`'s start (-1e8; 0.0 in
    // seed 3) and the kind it passes over (6008.0; 1.0 in seed 2, one of the stand-ins' answers, so
    // that test goes both ways).
    e.mem.set_u32(
        0x011d_8d54,
        if seed == 1 || seed == 3 {
            0
        } else {
            object(90)
        },
    );
    e.mem
        .set_u32(0x011d_ace4, if seed == 2 { 0 } else { object(91) });
    e.mem
        .set_f32(0x0107_0218, if seed == 3 { 0.0 } else { -1.0e8 });
    let excluded: f64 = if seed == 2 { 1.0 } else { 6008.0 };
    e.mem.set_u64(0x0107_0210, excluded.to_bits());
}

/// The manager singleton (`004b7210`'s word), the menus by class (`00a09030`'s data pointer and
/// count) and the controls object (`007fdf30`'s word).
const SINGLETON: u32 = 0x011d_8a80;
const MENU_TABLE: u32 = 0x011f_350c;
const MENU_COUNT: u32 = 0x011f_3512;
const CONTROLS: u32 = 0x011f_35cc;

/// What a parameter of the tested function takes.
#[derive(Clone, Copy, Debug)]
pub enum Arg {
    /// An object, another one, or null.
    Ptr,
    /// An object, null, 1, a menu id, 0x7fff.
    Word,
    /// 0, 1, 2, menu ids (the first, one inside, the last), the ids just outside the menus' range
    /// (1,000 below, 0x43d above).
    Int,
    /// 0, 1, 0xff.
    Byte,
    /// 1, -0.5, 0, a NaN, 1e30.
    F32,
    /// Menu classes: each the menu manager's factory and reload tell apart, a gap in the table, the
    /// first past the range, 0.
    Class,
    /// Tile ids at the edges of each menu's `AttachTileByID` range: -1, 0 to 3, 7, 8, 18, 19, 31, 32,
    /// 59, 60.
    TileId,
}

const PTRS: [u32; 3] = [ARENA + 3 * OBJECT, ARENA + 40 * OBJECT, 0];
const WORDS: [u32; 5] = [ARENA + 5 * OBJECT, 0, 1, 0x3eb, 0x7fff];
const INTS: [u32; 8] = [0, 1, 2, 0x3e9, 0x3ef, 0x43c, 0x43d, 1000];
const BYTES: [u32; 3] = [0, 1, 0xff];
const F32S: [u32; 5] = [0x3f80_0000, 0xbf00_0000, 0, 0x7fc0_0000, 0x7149_f2ca];
const CLASSES: [u32; 22] = [
    0x3ff, 0x40b, 0x425, 0x3ea, 0x3eb, 0x3ec, 0x3f5, 0x3ef, 0x403, 0x3f0, 0x402, 0x3f8, 0x423,
    0x41b, 0x41e, 0x420, 0x424, 0x432, 0x3e9, 0x3f2, 0x43d, 0,
];
const TILE_IDS: [u32; 13] = [
    0xffff_ffff,
    0,
    1,
    2,
    3,
    7,
    8,
    0x12,
    0x13,
    0x1f,
    0x20,
    0x3b,
    0x3c,
];

fn values(arg: Arg) -> &'static [u32] {
    match arg {
        Arg::Ptr => &PTRS,
        Arg::Word => &WORDS,
        Arg::Int => &INTS,
        Arg::Byte => &BYTES,
        Arg::F32 => &F32S,
        Arg::Class => &CLASSES,
        Arg::TileId => &TILE_IDS,
    }
}

/// The indices of an argument's values (the first `n` of these).
static INDEX: [u32; 24] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
];

/// The other families' typed functions the family calls through `typed_call!`: by address in these
/// tests (their stand-ins answer), each family's own tests run them.
const OTHER_FAMILIES: &[u32] = &[
    0x0070_0320,
    0x00ec_62c0,
    0x006b_3eb0,
    0x0084_54f0,
    0x00a0_12d0,
    0x00a0_1350,
    0x0040_fbf0,
    0x0040_fba0,
    0x0066_29f0,
    0x0043_b230,
    0x00a0_11b0,
    0x00a0_8b20,
    0x00a0_3da0,
    0x004c_8f30,
    0x0045_cec0,
    0x00a0_1230,
];

/// The world's settings for a Diff of the family.
pub trait InArena {
    /// The family named, the seeds and `args`'s values as the cases, the world built with stand-ins
    /// at `callees`; `plain`: the family's functions the translation reaches with a plain Rust call.
    /// What the family does in place is left out of the translation's calls.
    fn in_arena(self, args: &'static [Arg], callees: &'static [u32], plain: &[u32]) -> Diff;
    /// The same with no stand-in at `without`'s addresses (their real functions run) and their
    /// calls left out of the translation's too: for a typed function that leaves out work whose
    /// result the exe never uses.
    fn in_arena_without(
        self,
        args: &'static [Arg],
        callees: &'static [u32],
        plain: &[u32],
        without: &'static [u32],
    ) -> Diff;
    /// [`InArena::in_arena`] with one more input that is no argument: each of `extra`'s values in
    /// turn, which `put` writes into the world once it is built (a queue's words, say), so a
    /// function with few arguments still reaches each of its branches.
    fn in_arena_and(
        self,
        args: &'static [Arg],
        callees: &'static [u32],
        plain: &[u32],
        extra: &'static [u32],
        put: fn(&mut Engine, u32),
    ) -> Diff;
}

impl InArena for Diff {
    fn in_arena(self, args: &'static [Arg], callees: &'static [u32], plain: &[u32]) -> Diff {
        self.in_arena_without(args, callees, plain, &[])
    }

    fn in_arena_without(
        self,
        args: &'static [Arg],
        callees: &'static [u32],
        plain: &[u32],
        without: &'static [u32],
    ) -> Diff {
        arena_diff(self, args, callees, plain, without, None)
    }

    fn in_arena_and(
        self,
        args: &'static [Arg],
        callees: &'static [u32],
        plain: &[u32],
        extra: &'static [u32],
        put: fn(&mut Engine, u32),
    ) -> Diff {
        arena_diff(self, args, callees, plain, &[], Some((extra, put)))
    }
}

/// The Diff of [`InArena`]'s methods.
fn arena_diff(
    diff: Diff,
    args: &'static [Arg],
    callees: &'static [u32],
    plain: &[u32],
    without: &'static [u32],
    extra: Option<(&'static [u32], fn(&mut Engine, u32))>,
) -> Diff {
    let mut inputs = vec![Input::Of(SEEDS)];
    inputs.extend(args.iter().map(|a| Input::Of(&INDEX[..values(*a).len()])));
    if let Some((list, _)) = extra {
        inputs.push(Input::Of(list));
    }
    diff.family(&engine_typed::interface::INTERFACE)
        .translation_calls_plainly(plain)
        .ignore_calls(engine_typed::interface::interface_cpp::in_place::ADDRESSES)
        .ignore_calls(engine_typed::interface::interface_cpp_p2::IN_PLACE)
        .ignore_calls(engine_typed::interface::menu_manager::IN_PLACE)
        .ignore_calls(without)
        .goes_by_address(OTHER_FAMILIES)
        .inputs(&inputs)
        .max_cases(24)
        .max_calls(400)
        .setup(move |e, case: &Case| {
            world(e, case.word(0), callees, without);
            if let Some((_, put)) = extra {
                put(e, case.word(1 + args.len()));
            }
            args.iter()
                .enumerate()
                .map(|(i, a)| values(*a)[case.word(i + 1) as usize])
                .collect()
        })
}
