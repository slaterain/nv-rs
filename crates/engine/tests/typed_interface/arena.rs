//! The interface family's shared world for its differential tests: one setup for every function,
//! built from the case's seed the same way on both engines.
//!
//! - **The arena**: [`OBJECTS`] objects of [`OBJECT`] bytes. An object's first word is one of the
//!   [`VTABLES`] vtables, whose every slot is the address of a logging stand-in ([`CODE`]), so a
//!   virtual call on any object reaches one. Its other words are, by a seeded generator, zero, a small
//!   number, a float, a vtable, a stand-in's address, or a pointer to a LATER object: a chain of
//!   pointers always ends (at most [`OBJECTS`] steps), so a walk of a list never loops.
//! - **The exe's data**: the `.rdata` and `.data` pages the menus' code reads ([`DATA`]), filled by the
//!   same generator (their pointers reach any object).
//! - **The callees**: every function the tested code calls by address is a logging stand-in: it
//!   writes its address and words to the log page ([`LOG`]), so the memory compare sees them as well
//!   as the harness's call log, and answers a word, a float in ST0 and a word in EDX from a sequence
//!   (by the call's number), the same on both sides.
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
/// The vtables: [`VTABLE_COUNT`] of 0x400 bytes, each slot a stand-in.
pub const VTABLES: u32 = 0x0504_0000;
pub const VTABLE_COUNT: u32 = 8;
/// The stand-ins' own code addresses (no exe function lives there).
pub const CODE: u32 = 0x7d00_0000;
pub const CODE_COUNT: u32 = 0x100;
/// The log page: its first word counts the words written after it.
pub const LOG: u32 = 0x0510_0000;
const LOG_SIZE: u32 = 0x4_0000;
/// The exe's data the tested code reads: `.rdata` and `.data`/`.bss`.
pub const DATA: [(u32, u32); 2] = [(0x0100_0000, 0x10_0000), (0x011c_0000, 0xc_0000)];

/// The world's seeds: each case builds the world from one.
pub const SEEDS: &[u32] = &[1, 2, 3];

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

/// Appends `words` to the log under `tag`.
fn log(e: &mut Engine, tag: u32, words: &[u32]) {
    let n = e.mem.u32(LOG);
    if 4 * (n + 2 + words.len() as u32) >= LOG_SIZE {
        return;
    }
    e.mem.set_u32(LOG + 4 + 4 * n, tag);
    e.mem.set_u32(LOG + 8 + 4 * n, words.len() as u32);
    for (i, w) in (2u32..).zip(words) {
        e.mem.set_u32(LOG + 4 + 4 * (n + i), *w);
    }
    e.mem.set_u32(LOG, n + 2 + words.len() as u32);
}

/// A logging stand-in at `at`: its answer by the number of calls so far (a word on the log page).
fn stand_in(e: &mut Engine, at: u32) {
    e.register_double(at, move |e, words| {
        let calls = e.mem.u32(LOG + LOG_SIZE - 4);
        e.mem.set_u32(LOG + LOG_SIZE - 4, calls + 1);
        log(e, at, words);
        let n = calls.wrapping_mul(7).wrapping_add(at >> 2);
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
pub fn world(e: &mut Engine, seed: u32, callees: &[u32]) {
    // The first 64 KB and the pages a float word or a stand-in's address points at, as zeros: a null or
    // small pointer, or a float read as one, reads zeros on both sides instead of stopping both.
    for base in [0u32, CODE]
        .into_iter()
        .chain(FLOATS.iter().map(|f| f.to_bits() & 0xffff_0000))
    {
        e.map(base, 0x1_0000);
    }
    e.map(ARENA, OBJECTS * OBJECT);
    e.map(VTABLES, 0x400 * VTABLE_COUNT);
    e.map(LOG, LOG_SIZE);
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
    for &at in callees {
        if at == LIST_STEP {
            list_step(e);
        } else {
            stand_in(e, at);
        }
    }
}

/// `0057cbe0` (`thiscall`: the list and a cursor word): the list walks' step. The walks loop until
/// the cursor is 0, so its stand-in moves the cursor on: to the word after the one it holds (a chain
/// that ends, as every chain of the arena does), and answers the word it held.
const LIST_STEP: u32 = 0x0057_cbe0;

fn list_step(e: &mut Engine) {
    e.register_double(LIST_STEP, |e, words| {
        log(e, LIST_STEP, words);
        let cursor = words[1];
        let held = e.mem.u32(cursor);
        e.mem.set_u32(cursor, e.mem.u32(held.wrapping_add(4)));
        Ret {
            eax: held,
            ..Ret::default()
        }
    });
}

/// What a parameter of the tested function takes.
#[derive(Clone, Copy, Debug)]
pub enum Arg {
    /// An object, another one, or null.
    Ptr,
    /// An object, null, 1, a menu id, 0x7fff.
    Word,
    /// 0, 1, 2, a menu id, 1,000.
    Int,
    /// 0, 1, 0xff.
    Byte,
    /// 1, -0.5, 0, a NaN, 1e30.
    F32,
}

const PTRS: [u32; 3] = [ARENA + 3 * OBJECT, ARENA + 40 * OBJECT, 0];
const WORDS: [u32; 5] = [ARENA + 5 * OBJECT, 0, 1, 0x3eb, 0x7fff];
const INTS: [u32; 5] = [0, 1, 2, 0x3ef, 1000];
const BYTES: [u32; 3] = [0, 1, 0xff];
const F32S: [u32; 5] = [0x3f80_0000, 0xbf00_0000, 0, 0x7fc0_0000, 0x7149_f2ca];

fn values(arg: Arg) -> &'static [u32] {
    match arg {
        Arg::Ptr => &PTRS,
        Arg::Word => &WORDS,
        Arg::Int => &INTS,
        Arg::Byte => &BYTES,
        Arg::F32 => &F32S,
    }
}

const INDICES: [&[u32]; 6] = [
    &[],
    &[0],
    &[0, 1],
    &[0, 1, 2],
    &[0, 1, 2, 3],
    &[0, 1, 2, 3, 4],
];

/// The world's settings for a Diff of the family.
pub trait InArena {
    /// The family named, the seeds and `args`'s values as the cases, the world built with stand-ins
    /// at `callees`; `plain`: the family's functions the translation reaches with a plain Rust call.
    fn in_arena(self, args: &'static [Arg], callees: &'static [u32], plain: &[u32]) -> Diff;
}

impl InArena for Diff {
    fn in_arena(self, args: &'static [Arg], callees: &'static [u32], plain: &[u32]) -> Diff {
        let mut inputs = vec![Input::Of(SEEDS)];
        inputs.extend(args.iter().map(|a| Input::Of(INDICES[values(*a).len()])));
        self.family(&engine_typed::interface::INTERFACE)
            .family(&engine_core::typed::leaf::LEAVES)
            .translation_calls_plainly(plain)
            .inputs(&inputs)
            .max_cases(12)
            .max_calls(400)
            .setup(move |e, case: &Case| {
                world(e, case.word(0), callees);
                args.iter()
                    .enumerate()
                    .map(|(i, a)| values(*a)[case.word(i + 1) as usize])
                    .collect()
            })
    }
}
