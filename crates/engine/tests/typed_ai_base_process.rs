//! The differential test of every typed function of the AI family's `BaseProcess` module
//! (`engine_typed::ai::base_process`) against its translation, all in one world built from a seed.
//!
//! The world: five objects of 0x400 bytes (the process under test, another process, two forms or
//! records, and the save/load object), each word a seeded small integer, float bit pattern or
//! pointer to one of them, the fields the unit follows (the running package's object, the cached
//! values, the record being saved) one of them or 0. Each object has a vtable of its own: every
//! slot the unit calls is a stand-in answering a seeded word (in EAX, and as a float in ST0) or,
//! by the seed, the exe's own body (0x130, 0x22c, 0x238, 0x23c, 0x240, 0x280, 0x284), so the
//! typed side's direct branches run. The globals the unit reads (the shared point, the float and
//! double constants, the player, the save/load game) are seeded too.
//!
//! What stands in: every callee the unit calls by address. Each counts its calls and answers the
//! next of eight seeded words; the two buffer reads also write that word into the bytes they read.
//! What runs as itself: the families the unit's typed calls reach (the leaves, the AI queries'
//! slot 0x22c, the dispatch family's counted pointer release, the multibound family's world space
//! getter).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_typed::ai::{base_process, AI};
use engine_typed::ai_queries::AI_QUERIES;
use engine_typed::dispatch::DISPATCH;
use engine_typed::multibound::MULTIBOUND;

/// Where the stand-in slots are registered: `VT_FN + slot`.
const VT_FN: u32 = 0x00ff_4000;
/// A vtable's stand-in answers sit this far past it, at the same slot offset.
const ANSWERS: u32 = 0x800;
/// The doubles' counts and answers: double `k` at `CONTROL + 0x40 * k`.
const CONTROL: u32 = 0x0300_0000;
/// The save log setting's value byte.
const SETTING_BYTE: u32 = CONTROL + 0xf00;
/// The save buffer the positions point into.
const BUFFER: u32 = 0x0400_0000;

/// The slots the unit calls, each a stand-in unless the seed gives the exe's body.
const SLOTS: [u32; 12] = [
    0x0, 0x4, 0x12c, 0x130, 0x1e0, 0x214, 0x22c, 0x238, 0x23c, 0x240, 0x280, 0x284,
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
/// The process under test, the other process and the two forms (or records); object 4 is the
/// save/load object.
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
/// Float bit patterns a float constant may hold: its value, the signs of zero, a denormal, an
/// infinity and NaNs (a signalling one, which a load quiets).
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

/// The callees that stand in, with the words each may answer.
const DOUBLES: &[(u32, &[u32])] = &[
    (0x008c_5ef0, UNREAD),                       // ActorPackage::Clear
    (0x0041_fd00, UNREAD),                       // the store at +0x0c
    (0x0049_bb50, UNREAD),                       // the add at +0x0c
    (0x0041_a250, UNREAD),                       // the sound handle's constructor
    (0x0040_1030, UNREAD),                       // the free
    (0x008c_5e70, UNREAD),                       // ActorPackage's destructor
    (0x00ec_43fb, &[0, PROCESS, OTHER, FORM_B]), // __RTDynamicCast
    (0x008d_9fa0, UNREAD),                       // the level copies
    (0x0091_4c60, UNREAD),
    (0x0092_ca70, UNREAD),
    (0x0090_7510, UNREAD),
    (0x0086_75e0, UNREAD),               // the cached values' copy
    (0x008c_5ec0, UNREAD),               // the package's deleted reference handler
    (0x0067_59e0, &[0, 1, FORM_A]),      // TESPackage::ComputeInitialTarget
    (0x008c_6080, UNREAD),               // ActorPackage::InitFromPackage
    (0x0086_2110, &[0, 1, 0x100, 0xff]), // TESSaveLoadGame::UseSaveGameBlocks
    (
        0x0082_5c00,
        &[
            BUFFER,
            BUFFER + 2,
            BUFFER + 6,
            BUFFER + 0x10,
            BUFFER + 0x1_0000,
            BUFFER + 0x1_0010,
        ],
    ), // the buffer position
    (0x0085_79b0, UNREAD),               // the buffer write
    (0x0085_79e0, &[TAG, TAG, 0, 6, 0x10, 0xfff0]), // the buffer read
    (0x004f_d3c0, &[0, OTHER, FORM_B]),  // the record being loaded
    (0x008d_f040, &[0, 6, 0x80, 0xff]),  // the current version
    (0x0048_39c0, &[FORM_A, FORM_B]),    // the form for an id
    (0x005b_5e40, UNREAD),               // the SAVELOAD warnings
    (0x0040_8d60, &[SETTING_BYTE]),      // the setting's value address
    (0x008c_61a0, &[0, 4, 0xfff0, 0x1_0004]), // the package's save size
    (0x008c_6250, UNREAD),               // the package's save
    (0x008c_6320, UNREAD),               // ActorPackage::LoadGame
    (0x008c_64d0, UNREAD),               // ActorPackage::InitLoadGame
    (0x008a_6ce0, UNREAD),               // Actor::EvaluatePackage
    (0x008c_65a0, UNREAD),               // the package's revert
    (0x0086_5e50, UNREAD),               // the buffer write (of a buffer)
    (0x008c_65d0, UNREAD),               // ActorPackage::SaveGame
    (0x0086_4980, &[0, 1, 0x3f80_0000, 0x7f80_0001]), // the buffer read (of a buffer)
    (0x008c_6730, UNREAD),               // ActorPackage::LoadGame_ov2
    (0x008c_6970, UNREAD),               // ActorPackage::InitLoadGame_ov2
    (0x008c_6b20, UNREAD),               // ActorPackage::FinishLoadGame
    (0x008c_6c80, UNREAD),               // ActorPackage::Revert
];
/// The doubles that also write their answer into the bytes they were asked to read
/// (`thiscall(buffer, to, size)`).
const READS: [u32; 2] = [0x0085_79e0, 0x0086_4980];
/// `TESSaveLoadGame::UseSaveGameBlocks`, whose answer is the same for every call of a world.
const USE_SAVE_GAME_BLOCKS: u32 = 0x0086_2110;

/// The globals the unit reads.
const LARGEST_FLOAT: u32 = 0x0108_5df0;
const ZERO_DOUBLE: u32 = 0x0101_2060;
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;
const BREATH_TIMER_FLOAT: u32 = 0x0101_7868;
const SHARED_POINT: u32 = 0x011f_426c;
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const PLAYER: u32 = 0x011d_ea3c;

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

/// The world of one seed, made the same on both engines.
struct World {
    objects: [u32; 5],
    rng: u64,
}

impl World {
    fn new(e: &mut Engine, seed: u32) -> World {
        let mut w = World {
            objects: [0; 5],
            rng: (u64::from(seed) ^ 0x9e37_79b9_7f4a_7c15).wrapping_mul(0xbf58_476d_1ce4_e5b9) | 1,
        };
        for slot in SLOTS {
            e.register_double(VT_FN + slot, move |e, a| {
                let vt = e.mem.u32(a[0]);
                ret(e.mem.u32(vt + ANSWERS + slot))
            });
        }
        e.map(CONTROL, 0x1000);
        e.map(BUFFER, 0x2_0000);
        for (k, (addr, answers)) in DOUBLES.iter().enumerate() {
            let block = CONTROL + 0x40 * k as u32;
            // Whether the save uses blocks does not change within one save: one answer for all calls
            // (the exe's save would write its length through a null placeholder otherwise).
            let fixed = (*addr == USE_SAVE_GAME_BLOCKS).then(|| w.pick(answers));
            for i in 0..8 {
                let answer = fixed.unwrap_or_else(|| w.pick(answers));
                e.mem.set_u32(block + 4 + 4 * i, answer);
            }
            let reads = READS.contains(addr);
            e.register_double(*addr, move |e, a| {
                let n = e.mem.u32(block);
                e.mem.set_u32(block, n + 1);
                let answer = e.mem.u32(block + 4 + 4 * (n % 8));
                if reads {
                    for i in 0..a[2].min(4) {
                        e.mem.set_u8(a[1] + i, (answer >> (8 * i)) as u8);
                    }
                }
                ret(answer)
            });
        }
        // The import `InterlockedDecrement`, which the counted pointer's release reaches.
        e.register(0x00fd_f1c4, |e, a| {
            let n = e.mem.u32(a[0]).wrapping_sub(1);
            e.mem.set_u32(a[0], n);
            ret(n)
        });
        let setting = w.pick(&[0, 1, 0x80]);
        e.mem.set_u8(SETTING_BYTE, setting as u8);
        for i in 0..5 {
            w.objects[i] = e.mem.alloc(0x400);
        }
        // The doubles' answers name the objects: resolve them now they exist.
        for k in 0..DOUBLES.len() as u32 {
            for i in 0..8 {
                let at = CONTROL + 0x40 * k + 4 + 4 * i;
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
            // The fields the unit follows: the package's first word (0 lets `FinishInitLoadGame`
            // go on), the running package's object, the current procedure index, the cached
            // values and, on the save/load object, the record being saved.
            let first = if w.flag() { 0 } else { w.word() };
            e.mem.set_u32(o + 4, first);
            let running = w.pick(&[0, FORM_A, FORM_B]);
            e.mem.set_u32(o + 8, w.resolve(running));
            let index = w.pick(&[0xffff_ffff, 0, 1]);
            e.mem.set_u32(o + 0x10, index);
            let cached = w.pick(&[0, OTHER, FORM_B]);
            e.mem.set_u32(o + 0x2c, w.resolve(cached));
            let record = w.pick(&[0, OTHER, FORM_A]);
            e.mem.set_u32(o + 0x88, w.resolve(record));
        }
        // The globals.
        for page in [
            0x0108_5000,
            0x0101_2000,
            0x0101_7000,
            0x011f_4000,
            0x011d_e000,
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
        e.mem.set_u32(SAVE_LOAD_GAME, w.objects[4]);
        let player = w.pick(&[0, FORM_B, PROCESS]);
        e.mem.set_u32(PLAYER, w.resolve(player));
        w
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

    /// The process, then `n` seeded words.
    fn process_and(&mut self, n: usize) -> Vec<u32> {
        let mut words = vec![self.process()];
        for _ in 0..n {
            words.push(self.word());
        }
        words
    }
}

/// A test in the world: the families the unit reaches on, the world made from each seed, and the
/// call's words from `words`.
trait InWorld {
    fn in_world(self, seeds: &'static [u32], words: fn(&mut World) -> Vec<u32>) -> Diff;
}

impl InWorld for Diff {
    fn in_world(self, seeds: &'static [u32], words: fn(&mut World) -> Vec<u32>) -> Diff {
        self.family(&AI)
            .family(&leaf::LEAVES)
            .family(&AI_QUERIES)
            .family(&DISPATCH)
            .family(&MULTIBOUND)
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
    Diff::new(entry!(0x0071_7e50, base_process::address_4(Ptr) -> u32))
        .in_world(&MANY, |w| vec![w.word()])
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
        .in_world(&FEW, |w| w.process_and(2))
        .run()
        .assert_same();
}

#[test]
fn current_package() {
    Diff::new(entry!(
        0x008d_03a0,
        base_process::clear_current_package(Ptr)
    ))
    .in_world(&FEW, |w| w.process_and(0))
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
    .in_world(&MANY, |w| w.process_and(2))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0870,
        base_process::handle_deleted_reference(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.process_and(1))
    .translation_calls_plainly(&[0x008d_0430])
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
        .in_world(&FEW, |w| w.process_and(0))
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
    .translation_calls_plainly(&[0x008d_0640])
    .run()
    .assert_same();
}

#[test]
fn copy() {
    Diff::new(entry!(0x008d_0660, base_process::copy(Ptr, Ptr)))
        .in_world(&MOST, |w| {
            let other = w.one_of(&[0, OTHER, OTHER, PROCESS]);
            vec![w.process(), other]
        })
        .run()
        .assert_same();
}

#[test]
fn save_and_load() {
    Diff::new(entry!(0x008d_0900, base_process::save_size(Ptr, u32, u32) -> u16))
        .in_world(&MOST, |w| w.process_and(2))
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
    .in_world(&FEW, |w| w.process_and(3))
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
    .in_world(&FEW, |w| w.process_and(1))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_0f90,
        base_process::load_game_buffer(Ptr, u32)
    ))
    .in_world(&MANY, |w| {
        let buffer = w.one_of(&[FORM_A, FORM_B]);
        vec![w.process(), buffer]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x008d_1000,
        base_process::init_load_game_buffer(Ptr, u32)
    ))
    .in_world(&MANY, |w| w.process_and(1))
    .translation_calls_plainly(&[0x008d_1030])
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
    .in_world(&FEW, |w| w.process_and(1))
    .run()
    .assert_same();
    Diff::new(entry!(0x008d_1070, base_process::revert_buffer(Ptr, u32)))
        .in_world(&MANY, |w| w.process_and(1))
        .translation_calls_plainly(&[0x008d_1030])
        .run()
        .assert_same();
}
