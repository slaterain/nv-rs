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
//! What stands in: the save buffer's append (`00865570`, logged with the three bytes it is given) and
//! the other vtable's slots. The allocator (`00401000`) runs as it is, on both sides.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_typed::save::{self, refs};

/// The log the stand-ins write (a count, then `(tag, word)` pairs).
const LOG: u32 = 0x0300_0000;
/// The other vtable's slots: past `.text`, one function per slot.
const VT_FN: u32 = 0x00ff_2000;
/// The data handler and the save/load game's pointers.
const DATA_HANDLER: u32 = 0x011c_3f2c;
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
/// The save buffer's append.
const APPEND: u32 = 0x0086_5570;

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

/// The pages and stand-ins every case has: the log, the globals, the append and the other
/// vtable's slots (hash: bucket 0; compare: the map's word at `+0x40`; `SetValue`: the key and value
/// stored, logged; new item: a fresh 12-byte block, logged).
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [0x011c_3000, 0x011d_d000] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(DATA_HANDLER, 0x0123_4560);
    e.register(APPEND, |e, a| {
        log(e, 0x5570, a[0]);
        let bytes = e.mem.u32(a[1]) & 0x00ff_ffff;
        log(e, 0x5571, bytes);
        log(e, 0x5572, a[2]);
        log(e, 0x5573, a[3]);
        ret(0)
    });
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
fn families(d: Diff) -> Diff {
    d.family(&save::SAVE)
        .family(&engine_typed::dispatch::DISPATCH)
        .family(&engine_typed::pathing::PATHING)
        .family(&leaf::LEAVES)
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
    families(Diff::new(entry!(
        0x0084_4700,
        refs::set_at(Ptr<_>, u32, u32)
    )))
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
    families(Diff::new(entry!(
        0x0084_4700,
        refs::set_at(Ptr<_>, u32, u32)
    )))
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
    families(Diff::new(entry!(
        0x0084_4700,
        refs::set_at(Ptr<_>, u32, u32)
    )))
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
    families(Diff::new(entry!(
        0x0084_6c90,
        refs::get_or_add_index(Ptr<_>, u32) -> u32
    )))
    .inputs(&form_id_inputs())
    .setup(form_id_words)
    .run()
    .assert_same();
}

/// Resolve: a created form's id stands for itself; a held index gives its id; an absent one 0.
#[test]
fn resolve_index() {
    families(Diff::new(entry!(
        0x0084_6d20,
        refs::resolve_index(Ptr<_>, u32) -> u32
    )))
    .inputs(&form_id_inputs())
    .setup(form_id_words)
    .run()
    .assert_same();
}

#[test]
fn is_created_id() {
    families(Diff::new(entry!(
        0x0046_9860,
        refs::is_created_id(Ptr<_>, u32) -> bool
    )))
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
    families(Diff::new(entry!(
        0x0085_3500,
        refs::get_numeric_id(Ptr<_>) -> u32
    )))
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
    families(Diff::new(entry!(
        0x0085_3570,
        refs::set_numeric_id(Ptr<_>, u32)
    )))
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
    families(Diff::new(entry!(
        0x0084_eaf0,
        refs::index_new(Ptr<_>) -> Ptr<_>
    )))
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
    let map = form_id_map(e, 37, case.word(1), 9);
    with_game(e, map);
    block(e, 0x14, &[0x0108_2324, 0, 0, 0, 0])
}

#[test]
fn save_numeric_form_id() {
    families(Diff::new(entry!(
        0x0086_5db0,
        refs::save_numeric_form_id(Ptr<_>, u32, u32)
    )))
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
    families(Diff::new(entry!(
        0x0086_5df0,
        refs::save_form_id(Ptr<_>, Ptr<_>, u32)
    )))
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

#[test]
fn new_item_and_allocate_item() {
    for (address, f) in [
        entry!(0x0084_4a70, refs::new_item(Ptr<_>) -> Ptr<_>),
        entry!(0x006f_99e0, refs::allocate_item(Ptr<_>) -> Ptr<_>),
    ] {
        families(Diff::new((address, f)))
            .inputs(&[Input::Flag])
            .setup(|e, _| {
                stand_ins(e);
                vec![e.mem.alloc(0x10)]
            })
            .run()
            .assert_same();
    }
}

#[test]
fn set_value() {
    families(Diff::new(entry!(
        0x0086_3840,
        refs::set_value(Ptr<_>, Ptr<_>, u32, u32)
    )))
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
    families(Diff::new(entry!(
        0x0085_3570,
        refs::set_numeric_id(Ptr<_>, u32)
    )))
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
