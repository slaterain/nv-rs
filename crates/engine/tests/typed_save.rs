//! The differential test of every typed function of the save family (`engine_typed::save`) against
//! its translation: the form id map a save writes every form reference through, the word maps'
//! `SetAt` with its item functions, the three-byte index and the save buffer's two `SaveFormID`s.
//!
//! The maps are real: `NiTMap<unsigned int, unsigned int>`'s slots (the hash `006c6a60`, the compare
//! `006b81d0`, `SetValue` `00863840`, the new item `00844a70`) run translated on one side and typed
//! on the other, over maps of 1, 5 and 37 buckets holding none, some or many of the keys, the key
//! looked up at the head, inside or at the end of its chain, or absent. A second vtable whose slots
//! are this test's stand-ins checks the calls through the vtable for any other map.
//!
//! The save buffer (`BGSSaveGameBuffer`) is real too: the positioned write, the append, the growth,
//! the variable-sized values, the strings, the save to a file (whose real write and the
//! data-to-file helper stand in, logged), sizing or not; and the walk over a map.
//!
//! What stands in: the other vtable.s slots, the real file.s write (`00473180`), the warnings. The
//! allocator (`00401000`) runs as it is, on both sides.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_typed::save::{self, buffer, maps, refs};

/// The log the stand-ins write (a count, then `(tag, word)` pairs).
const LOG: u32 = 0x0300_0000;
/// The other vtable's slots: past `.text`, one function per slot.
const VT_FN: u32 = 0x00ff_2000;
/// The data handler and the save/load game's pointers.
const DATA_HANDLER: u32 = 0x011c_3f2c;
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
/// The save/load manager (its byte at +0xc: the save is only being sized).
const MANAGER: u32 = 0x011d_e134;

/// `NiTMap<unsigned int, unsigned int>`'s vtable (`0106cf68`) as the exe holds it: the scalar
/// deleting destructor, the hash, the compare, `SetValue`, `ClearValue`, the new item, the delete.
const WORD_MAP_SLOTS: [u32; 7] = [
    0x006e_20b0,
    0x006c_6a60,
    0x006b_81d0,
    0x0086_3840,
    0x0045_34f0,
    0x0084_4a70,
    0x0085_3240,
];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

fn words(e: &mut Engine, at: u32, w: &[u32]) {
    for (i, v) in (0u32..).zip(w) {
        e.mem.set_u32(at + 4 * i, *v);
    }
}

fn block(e: &mut Engine, size: u32, w: &[u32]) -> u32 {
    let a = e.mem.alloc(size.max(4 * w.len() as u32));
    words(e, a, w);
    a
}

/// The pages and stand-ins every case has: the log, the globals and the other
/// vtable's slots (hash: bucket 0; compare: the map's word at `+0x40`; `SetValue`: the key and value
/// stored, logged; new item: a fresh 12-byte block, logged).
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [0x011c_3000, 0x011d_d000, 0x011d_e000] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(DATA_HANDLER, 0x0123_4560);
    e.register(VT_FN + 4, |e, a| {
        log(e, 4, a[1]);
        ret(0)
    });
    e.register(VT_FN + 8, |e, a| {
        log(e, 8, a[1]);
        log(e, 9, a[2]);
        ret(e.mem.u32(a[0] + 0x40) & 1)
    });
    e.register(VT_FN + 0xc, |e, a| {
        log(e, 0xc, a[1]);
        e.mem.set_u32(a[1] + 4, a[2]);
        e.mem.set_u32(a[1] + 8, a[3]);
        ret(0)
    });
    e.register(VT_FN + 0x14, |e, a| {
        let item = e.mem.alloc(0xc);
        log(e, 0x14, a[0]);
        ret(item)
    });
}

/// The word map's vtable, as the exe's.
fn word_map_vtable(e: &mut Engine) -> u32 {
    block(e, 0x1c, &WORD_MAP_SLOTS)
}

/// A vtable whose slots are this test's stand-ins.
fn other_vtable(e: &mut Engine) -> u32 {
    let vt = e.mem.alloc(0x20);
    for slot in (0..0x20).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    vt
}

/// A word map at `map` (0x10 bytes, and a word at +0x40 for the stand-ins' compare when there is
/// room) with vtable `vt`, `buckets` buckets and a node (next, key, value) for each entry, put at
/// the head of its key's bucket, as `SetAt` puts them.
fn fill_map(e: &mut Engine, map: u32, vt: u32, buckets: u32, entries: &[(u32, u32)]) {
    let table = e.mem.alloc(4 * buckets.max(1));
    words(e, map, &[vt, buckets, table, entries.len() as u32]);
    for &(k, v) in entries {
        let at = table + 4 * (k % buckets.max(1));
        let head = e.mem.u32(at);
        let node = block(e, 0xc, &[head, k, v]);
        e.mem.set_u32(at, node);
    }
}

/// The keys the maps hold, the first `n` of them: chains at the head, inside and at the end in a
/// 37-bucket map (0, 37, 74 and 111 share a bucket), the largest key and created forms' ids.
const KEYS: &[u32] = &[0, 1, 5, 37, 38, 74, 111, 0xffff_ffff, 0xff00_0004];

/// The keys looked up: present ones (at every place in a chain), absent ones in a full bucket and in
/// an empty one, and the created-id boundary.
const LOOKED_UP: &[u32] = &[0, 1, 37, 74, 111, 2, 148, 0xfeff_ffff, 0xff00_0000];

fn entries(n: u32) -> Vec<(u32, u32)> {
    KEYS[..n as usize]
        .iter()
        .map(|&k| (k, k ^ 0x5a5a))
        .collect()
}

/// The families this test runs with: this one, the dispatch family's lookup, the pathing family's
/// hash and compare, and the leaves.
trait Families {
    fn families(self) -> Diff;
}

impl Families for Diff {
    fn families(self) -> Diff {
        self.family(&save::SAVE)
            .family(&engine_typed::dispatch::DISPATCH)
            .family(&engine_typed::pathing::PATHING)
            .family(&leaf::LEAVES)
            .family(&engine_typed::crt::CRT)
    }
}

// ---- SetAt ----

/// The word map: the bucket count, the key, how many keys the map holds, the value.
fn set_at_word_map_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let vt = word_map_vtable(e);
    let map = e.mem.alloc(0x44);
    fill_map(e, map, vt, case.word(0), &entries(case.word(2)));
    vec![map, case.word(1), case.word(3)]
}

/// `SetAt` on the word map: a present key's value replaced (at the head, inside, at the end of its
/// chain), an absent key added at the head of its bucket with the count up; every slot typed.
#[test]
fn set_at_word_map() {
    Diff::new(entry!(0x0084_4700, refs::set_at(Ptr<_>, u32, u32)))
        .families()
        .inputs(&[
            Input::Of(&[1, 5, 37]),
            Input::Of(LOOKED_UP),
            Input::Of(&[0, 4, 9]),
            Input::Of(&[0, 0x1234_5678]),
        ])
        .setup(set_at_word_map_words)
        .run()
        .assert_same();
}

/// Any other map: the stand-ins' slots through the vtable, in the exe's order: 0, 1 or 3 nodes in
/// the bucket, equal (the map's word at +0x40) or not.
fn set_at_other_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let vt = other_vtable(e);
    let map = e.mem.alloc(0x44);
    let keys: Vec<(u32, u32)> = (0..case.word(0)).map(|i| (7 * i, i)).collect();
    fill_map(e, map, vt, 1, &keys);
    e.mem.set_u32(map + 0x40, case.word(1));
    vec![map, case.word(2), 0x00c0_ffee]
}

#[test]
fn set_at_other_slots() {
    Diff::new(entry!(0x0084_4700, refs::set_at(Ptr<_>, u32, u32)))
        .families()
        .inputs(&[Input::Of(&[0, 1, 3]), Input::Flag, Input::Word])
        .setup(set_at_other_words)
        .run()
        .assert_same();
}

/// A map whose slots mix the word map's and the stand-ins': each slot decides by its own target.
fn set_at_mixed_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let vt = word_map_vtable(e);
    // Bit i of the case's mask: slot (hash, compare, set value, new item) i is the stand-in's.
    let mask = case.word(0);
    for (i, slot) in [4u32, 8, 0xc, 0x14].into_iter().enumerate() {
        if mask & (1 << i) != 0 {
            e.mem.set_u32(vt + slot, VT_FN + slot);
        }
    }
    let map = e.mem.alloc(0x44);
    fill_map(e, map, vt, 1, &entries(case.word(1)));
    e.mem.set_u32(map + 0x40, 0);
    vec![map, case.word(2), 0x00c0_ffee]
}

#[test]
fn set_at_mixed_slots() {
    Diff::new(entry!(0x0084_4700, refs::set_at(Ptr<_>, u32, u32)))
        .families()
        .inputs(&[
            Input::Of(&[0, 1, 2, 4, 8, 0xf]),
            Input::Of(&[0, 3]),
            Input::Of(&[1, 2, 0xffff_ffff]),
        ])
        .setup(set_at_mixed_words)
        .run()
        .assert_same();
}

// ---- The form id map ----

/// A form id map (`BGSSaveLoadFormIDMap`, 0x24 bytes): forward and reverse word maps of `buckets`
/// buckets holding the first `n` keys (index i for key i in the list order, and back), the next
/// index `next`.
fn form_id_map(e: &mut Engine, buckets: u32, n: u32, next: u32) -> u32 {
    let vt = word_map_vtable(e);
    let map = e.mem.alloc(0x24);
    let forward: Vec<(u32, u32)> = (0u32..)
        .zip(&KEYS[..n as usize])
        .map(|(i, &k)| (k, i))
        .collect();
    let reverse: Vec<(u32, u32)> = forward.iter().map(|&(k, i)| (i, k)).collect();
    fill_map(e, map, vt, buckets, &forward);
    fill_map(e, map + 0x10, vt, buckets, &reverse);
    e.mem.set_u32(map + 0x20, next);
    map
}

/// The form id map's inputs: buckets, the id or index, the keys held, the next index.
fn form_id_inputs() -> Vec<Input> {
    vec![
        Input::Of(&[1, 37]),
        Input::Of(LOOKED_UP),
        Input::Of(&[0, 4, 9]),
        Input::Of(&[9, 0x7f_ffff, 0xffff_ffff]),
    ]
}

fn form_id_words(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let map = form_id_map(e, case.word(0), case.word(2), case.word(3));
    vec![map, case.word(1)]
}

/// Get-or-add: a created form's id is its own index; a held id gives its index; a new one gets the
/// next index (also at the counter's wrap), put in both maps.
#[test]
fn get_or_add_index() {
    Diff::new(entry!(
        0x0084_6c90,
        refs::get_or_add_index(Ptr<_>, u32) -> u32
    ))
    .families()
    .inputs(&form_id_inputs())
    .setup(form_id_words)
    .run()
    .assert_same();
}

/// Resolve: a created form's id stands for itself; a held index gives its id; an absent one 0.
#[test]
fn resolve_index() {
    Diff::new(entry!(0x0084_6d20, refs::resolve_index(Ptr<_>, u32) -> u32))
        .families()
        .inputs(&form_id_inputs())
        .setup(form_id_words)
        .run()
        .assert_same();
}

#[test]
fn is_created_id() {
    Diff::new(entry!(
        0x0046_9860,
        refs::is_created_id(Ptr<_>, u32) -> bool
    ))
    .families()
    .inputs(&[
        Input::Word,
        Input::Of(&[0, 0xfeff_ffff, 0xff00_0000, 0xff00_0001, 0xffff_ffff]),
    ])
    .run()
    .assert_same();
}

// ---- The three-byte index ----

/// The save/load game with the form id map at `+8`.
fn with_game(e: &mut Engine, map: u32) {
    let game = block(e, 0x20, &[0, 0, map]);
    e.mem.set_u32(SAVE_LOAD_GAME, game);
}

/// The index's three bytes (high first) in a 4-byte block whose fourth byte is 0xee.
fn index_block(e: &mut Engine, index: u32) -> u32 {
    let [_, high, middle, low] = index.to_be_bytes();
    block(e, 4, &[u32::from_le_bytes([high, middle, low, 0xee])])
}

#[test]
fn get_numeric_id() {
    Diff::new(entry!(0x0085_3500, refs::get_numeric_id(Ptr<_>) -> u32))
        .families()
        .inputs(&[
            Input::Of(&[
                0, 1, 3, 8, 0x7f_ffff, 0x80_0000, 0x80_0004, 0xff_ffff, 0x12_3456,
            ]),
            Input::Of(&[0, 9]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            let map = form_id_map(e, 37, case.word(1), 9);
            with_game(e, map);
            vec![index_block(e, case.word(0))]
        })
        .run()
        .assert_same();
}

#[test]
fn set_numeric_id() {
    Diff::new(entry!(0x0085_3570, refs::set_numeric_id(Ptr<_>, u32)))
        .families()
        .inputs(&[
            Input::Of(LOOKED_UP),
            Input::Of(&[0, 9]),
            Input::Of(&[9, 0x80_0000, 0xfe_ffff]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            let map = form_id_map(e, 37, case.word(1), case.word(2));
            with_game(e, map);
            vec![index_block(e, 0xab_cdef), case.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn index_new() {
    Diff::new(entry!(0x0084_eaf0, refs::index_new(Ptr<_>) -> Ptr<_>))
        .families()
        .inputs(&[Input::Word])
        .setup(|e, case| {
            stand_ins(e);
            vec![block(e, 4, &[case.word(0)])]
        })
        .run()
        .assert_same();
}

// ---- The save buffer's form ids ----

fn save_form_id_world(e: &mut Engine, case: &Case) -> u32 {
    stand_ins(e);
    manager(e, 0);
    let map = form_id_map(e, 37, case.word(1), 9);
    with_game(e, map);
    block(e, 0x14, &[0x0108_2324, 0, 0, 0, 0])
}

#[test]
fn save_numeric_form_id() {
    Diff::new(entry!(
        0x0086_5db0,
        refs::save_numeric_form_id(Ptr<_>, u32, u32)
    ))
    .families()
    .inputs(&[Input::Of(LOOKED_UP), Input::Of(&[0, 9]), Input::Word])
    .setup(|e, case| {
        let buffer = save_form_id_world(e, case);
        vec![buffer, case.word(0), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn save_form_id() {
    Diff::new(entry!(0x0086_5df0, refs::save_form_id(Ptr<_>, Ptr<_>, u32)))
        .families()
        .inputs(&[
            Input::Of(&[0, 1, 37, 2, 0xff00_0000, 0xffff_ffff]),
            Input::Of(&[0, 9]),
            Input::Flag,
        ])
        .setup(|e, case| {
            let buffer = save_form_id_world(e, case);
            // Flag 0: no form (a null pointer); else a form with the id at +0xc.
            let form = if case.word(2) == 0 {
                0
            } else {
                block(e, 0x10, &[0, 0, 0, case.word(0)])
            };
            vec![buffer, form, 0]
        })
        .run()
        .assert_same();
}

// ---- The item functions ----

fn a_map(e: &mut Engine, _case: &Case) -> Vec<u32> {
    stand_ins(e);
    vec![e.mem.alloc(0x10)]
}

#[test]
fn new_item() {
    Diff::new(entry!(0x0084_4a70, refs::new_item(Ptr<_>) -> Ptr<_>))
        .families()
        .inputs(&[Input::Flag])
        .setup(a_map)
        .run()
        .assert_same();
}

#[test]
fn allocate_item() {
    Diff::new(entry!(0x006f_99e0, refs::allocate_item(Ptr<_>) -> Ptr<_>))
        .families()
        .inputs(&[Input::Flag])
        .setup(a_map)
        .run()
        .assert_same();
}

#[test]
fn set_value() {
    Diff::new(entry!(
        0x0086_3840,
        refs::set_value(Ptr<_>, Ptr<_>, u32, u32)
    ))
    .families()
    .inputs(&[Input::Word, Input::Word])
    .setup(|e, case| {
        stand_ins(e);
        let item = block(e, 0xc, &[0x1111, 0x2222, 0x3333]);
        vec![0x0bad_0000, item, case.word(0), case.word(1)]
    })
    .run()
    .assert_same();
}

/// A whole save's worth of form ids in one run: 300 ids written through `SaveFormID` against the
/// translation, the map growing from empty, ids repeating, created forms between them; the maps,
/// the counter and every appended index compare.
#[test]
fn many_form_ids_through_one_map() {
    Diff::new(entry!(0x0085_3570, refs::set_numeric_id(Ptr<_>, u32)))
        .families()
        .inputs(&[Input::Of(&[37, 1])])
        .setup(|e, case| {
            stand_ins(e);
            let map = form_id_map(e, case.word(0), 0, 1);
            with_game(e, map);
            let index = index_block(e, 0);
            // Fill the map first through the translation, the same on every side.
            for i in 0..300u32 {
                let id = match i % 7 {
                    0 => 0xff00_0000 | i,
                    1 => i / 2,
                    _ => 0x0001_0000 + (i * 37) % 211,
                };
                e.call(0x0085_3570, &args![index, id]);
            }
            vec![index, 0x0001_0007]
        })
        .run()
        .assert_same();
}

// ---- The save buffer (PR 2) ----

/// The save/load manager with its sizing byte (+0xc).

fn manager(e: &mut Engine, sizing: u32) {
    let m = block(e, 0x10, &[0, 0, 0, sizing]);
    e.mem.set_u32(MANAGER, m);
}

fn save_buffer(e: &mut Engine, size: u32, position: u32) -> u32 {
    let data = if size == 0 {
        0
    } else {
        let d = e.mem.alloc(size);
        for i in 0..size {
            e.mem.set_u8(d + i, (i * 7 + 3) as u8);
        }
        d
    };
    block(e, 0x14, &[0x0108_2324, data, size, position, 5])
}

/// The bytes a write takes: `n` bytes of another pattern.
fn source(e: &mut Engine, n: u32) -> u32 {
    let s = e.mem.alloc(n.max(4));
    for i in 0..n {
        e.mem.set_u8(s + i, (0x80 + i) as u8);
    }
    s
}

fn buffer_world(e: &mut Engine) {
    stand_ins(e);
    // The bounded move (`008661e0`, the CRT's `memmove_s`): a move, logged.
    e.register(0x0086_61e0, |e, a| {
        log(e, 0x61e0, a[3]);
        let bytes = e.mem.bytes(a[2], a[3]).to_vec();
        e.mem.write(a[0], &bytes);
        ret(0)
    });
}

/// A positioned write: sizing or not; a buffer empty, roomy or one byte short; the write at the
/// position, before it or past it; with or without the separator; 0, 1, 3 or 0x11000 bytes (past
/// a growth).
#[test]
fn write_at() {
    Diff::new(entry!(
        0x0086_5ce0,
        buffer::write_at(Ptr<_>, Ptr<_>, u32, u32, u8)
    ))
    .families()
    .inputs(&[
        Input::Flag,
        Input::Of(&[0, 0x20, 0x10]),
        Input::Of(&[0, 1, 3, 0x11000]),
        Input::Of(&[0, 8, 0xc]),
        Input::Flag,
    ])
    .setup(|e, case| {
        buffer_world(e);
        manager(e, case.word(0));
        let b = save_buffer(e, case.word(1), 0xc.min(case.word(1)));
        let s = source(e, case.word(2));
        vec![b, s, case.word(2), case.word(3), case.word(4)]
    })
    .max_calls(2_000_000)
    .run()
    .assert_same();
}

#[test]
fn append() {
    Diff::new(entry!(0x0086_5570, buffer::append(Ptr<_>, Ptr<_>, u32, u8)))
        .families()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 0x10, 0x20]),
            Input::Of(&[0, 2, 9]),
            Input::Flag,
        ])
        .setup(|e, case| {
            buffer_world(e);
            manager(e, case.word(0));
            let b = save_buffer(e, case.word(1), case.word(1) / 2);
            let s = source(e, case.word(2));
            vec![b, s, case.word(2), case.word(3)]
        })
        .run()
        .assert_same();
}

#[test]
fn save_bytes() {
    Diff::new(entry!(
        0x0086_5e50,
        buffer::save_bytes(Ptr<_>, Ptr<_>, u32, u32)
    ))
    .families()
    .inputs(&[
        Input::Flag,
        Input::Of(&[0, 0x10]),
        Input::Of(&[0, 2, 9]),
        Input::Word,
    ])
    .setup(|e, case| {
        buffer_world(e);
        manager(e, case.word(0));
        let b = save_buffer(e, case.word(1), case.word(1) / 2);
        let s = source(e, case.word(2));
        vec![b, s, case.word(2), case.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn grow() {
    Diff::new(entry!(0x0086_60b0, buffer::grow(Ptr<_>)))
        .families()
        .inputs(&[Input::Flag, Input::Of(&[0, 0x10, 0xffff_0008])])
        .setup(|e, case| {
            buffer_world(e);
            manager(e, case.word(0));
            let size = case.word(1);
            // A size near the top of the word wraps; it holds no block (none could be that big).
            let b = if size > 0x10_0000 {
                block(e, 0x14, &[0x0108_2324, 0, size, 0, 0])
            } else {
                save_buffer(e, size, 0)
            };
            vec![b]
        })
        .run()
        .assert_same();
}

/// The variable-sized values: each tag width and the boundaries, and a value too large.
const VARIABLE_VALUES: &[u32] = &[
    0,
    1,
    0x3f,
    0x40,
    0x3fff,
    0x4000,
    0x3fff_ffff,
    0x4000_0000,
    u32::MAX,
];

#[test]
fn save_variable_sized_value() {
    Diff::new(entry!(
        0x0086_5f60,
        buffer::save_variable_sized_value(Ptr<_>, u32)
    ))
    .families()
    .inputs(&[Input::Flag, Input::Of(VARIABLE_VALUES)])
    .setup(|e, case| {
        buffer_world(e);
        manager(e, case.word(0));
        e.register(0x0084_5cc0, |e, a| {
            log(e, 0x5cc0, a[1]);
            ret(0)
        });
        vec![save_buffer(e, 0x10, 3), case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn start_and_finish_variable_value() {
    Diff::new(entry!(
        0x0086_5f20,
        buffer::start_variable_value(Ptr<_>) -> u32
    ))
    .families()
    .translation_calls_plainly(&[0x0086_5ce0])
    .inputs(&[Input::Flag, Input::Of(&[0, 0x10, 0x11])])
    .setup(|e, case| {
        buffer_world(e);
        manager(e, case.word(0));
        vec![save_buffer(e, case.word(1), case.word(1).min(0xf))]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0086_5ff0,
        buffer::finish_variable_value(Ptr<_>, u32, u32)
    ))
    .families()
    .translation_calls_plainly(&[0x0086_5ce0, 0x0086_6140])
    .inputs(&[
        Input::Flag,
        Input::Of(VARIABLE_VALUES),
        Input::Of(&[0, 4, 0xc]),
    ])
    .setup(|e, case| {
        buffer_world(e);
        manager(e, case.word(0));
        e.register(0x0084_5cc0, |e, a| {
            log(e, 0x5cc0, a[1]);
            ret(0)
        });
        vec![save_buffer(e, 0x10, 0xc), case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn insert_space() {
    Diff::new(entry!(0x0086_6140, buffer::insert_space(Ptr<_>, u32, u32)))
        .families()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 0x10]),
            Input::Of(&[0, 4]),
            Input::Of(&[0, 1, 3, 0x20, 0x8000_0000]),
        ])
        .setup(|e, case| {
            buffer_world(e);
            manager(e, case.word(0));
            vec![
                save_buffer(e, case.word(1), case.word(1).min(8)),
                case.word(2),
                case.word(3),
            ]
        })
        .run()
        .assert_same();
}

#[test]
fn save_string() {
    Diff::new(entry!(
        0x0086_5e70,
        buffer::save_string(Ptr<_>, Ptr<_>, u32)
    ))
    .families()
    .inputs(&[Input::Flag, Input::Of(&[0, 1, 2, 40])])
    .setup(|e, case| {
        buffer_world(e);
        manager(e, case.word(0));
        // 0: no text; else a text of n - 1 letters.
        let text = match case.word(1) {
            0 => 0,
            n => {
                let t = e.mem.alloc(n + 4);
                for i in 0..n - 1 {
                    e.mem.set_u8(t + i, b'a' + (i % 26) as u8);
                }
                t
            }
        };
        vec![save_buffer(e, 0x10, 2), text, 0]
    })
    .run()
    .assert_same();
}

/// A save file (0x110 bytes) with simulated size and position, and the real file's write logged.
fn save_file(e: &mut Engine, size: u32, position: u32) -> u32 {
    let f = e.mem.alloc(0x110);
    words(e, f + 0x104, &[0x0bad_f11e, size, position]);
    e.register(0x0047_3180, |e, a| {
        log(e, 0x3180, a[0]);
        log(e, 0x3181, a[1]);
        log(e, 0x3182, a[2]);
        ret(a[2])
    });
    e.register(0x0084_5d00, |e, a| {
        log(e, 0x5d00, a[0]);
        log(e, 0x5d01, e.mem.u32(a[1]));
        ret(0)
    });
    f
}

#[test]
fn file_write() {
    Diff::new(entry!(
        0x0084_6330,
        buffer::file_write(Ptr<_>, u32, u32) -> u32
    ))
    .families()
    .inputs(&[
        Input::Flag,
        Input::Of(&[0, 0x10, 0xffff_fff0]),
        Input::Of(&[0, 8, 0x10]),
        Input::Of(&[0, 4, 0x20]),
    ])
    .setup(|e, case| {
        buffer_world(e);
        manager(e, case.word(0));
        let f = save_file(e, case.word(1), case.word(2));
        vec![f, 0x0030_0000, case.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn save_and_save_part() {
    Diff::new(entry!(0x0086_5c40, buffer::save(Ptr<_>, Ptr<_>)))
        .families()
        .translation_calls_plainly(&[0x0086_5c80])
        .inputs(&[Input::Flag, Input::Of(&[0, 0x10])])
        .setup(|e, case| {
            buffer_world(e);
            manager(e, case.word(0));
            let f = save_file(e, 0, 0);
            vec![save_buffer(e, case.word(1), case.word(1) / 2), f]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0086_5c80, buffer::save_part(Ptr<_>, Ptr<_>, u32)))
        .families()
        .inputs(&[Input::Flag, Input::Of(&[0, 0x10]), Input::Of(&[0, 3])])
        .setup(|e, case| {
            buffer_world(e);
            manager(e, case.word(0));
            let f = save_file(e, 0, 0);
            vec![save_buffer(e, case.word(1), 0), f, case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn byte_c() {
    Diff::new(entry!(0x0042_4940, buffer::byte_c(Ptr<_>) -> u8))
        .families()
        .inputs(&[Input::Of(&[0, 1, 0x1ff, 0xffff_ffff])])
        .setup(|e, case| {
            stand_ins(e);
            vec![block(e, 0x10, &[0, 0, 0, case.word(0)])]
        })
        .run()
        .assert_same();
}

// ---- The changes map's walk (PR 2) ----

/// A walk over a map: buckets 1, 5 or 37, 0 to 9 keys, the hash the word map's or a stand-in's
/// (bucket 0).
fn walk_map(e: &mut Engine, case: &Case) -> u32 {
    stand_ins(e);
    let vt = if case.word(2) == 0 {
        word_map_vtable(e)
    } else {
        other_vtable(e)
    };
    let map = e.mem.alloc(0x44);
    fill_map(e, map, vt, case.word(0), &entries(case.word(1)));
    map
}

#[test]
fn first_position() {
    Diff::new(entry!(0x004b_9ba0, maps::first_position(Ptr<_>) -> u32))
        .families()
        .inputs(&[
            Input::Of(&[1, 5, 37]),
            Input::Of(&[0, 1, 4, 9]),
            Input::Flag,
        ])
        .setup(|e, case| vec![walk_map(e, case)])
        .run()
        .assert_same();
}

/// Each step of a whole walk: the position at the n-th node of the walk.
#[test]
fn next() {
    Diff::new(entry!(
        0x006b_7f20,
        maps::next(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>)
    ))
    .families()
    .inputs(&[
        Input::Of(&[1, 5, 37]),
        Input::Of(&[1, 4, 9]),
        Input::Flag,
        Input::Of(&[0, 1, 2, 3, 8]),
    ])
    .setup(|e, case| {
        let map = walk_map(e, case);
        let position = block(e, 0xc, &[0, 0, 0]);
        // Walk to the n-th node with the translation (the same on every side).
        let first = e.call(0x004b_9ba0, &args![map]).u32();
        e.mem.set_u32(position, first);
        let steps = case.word(3).min(case.word(1) - 1);
        for _ in 0..steps {
            e.call(
                0x006b_7f20,
                &args![map, position, position + 4, position + 8],
            );
        }
        vec![map, position, position + 4, position + 8]
    })
    .run()
    .assert_same();
}
