//! Engine Optimizations item 7 (menus): the typed tile values and reaction map (`dispatch::tile`)
//! against their translations (`engine-bs` `bsmenu/tile.rs`), and a timing harness run by hand
//! (`--ignored --nocapture`).
//!
//! The typed sorted find calls the typed `bsearch` (`00ec716d`) and the typed trait compare
//! (`00a01160`) as `typed_call!`s, so their calls compare call for call. The translation's
//! sorted insert reaches the array insert (`00a0cd50`) by a plain Rust call
//! (`translation_calls_plainly`).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_core::typed::leaf;
use engine_typed::{crt, dispatch};
use std::time::Instant;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The tiles' globals page (critical section `011f3330`, the search keys `011f3374`/`011f338c` and
/// their guards, the reaction map `011f3358`), the memory manager's page and the C runtime's locale
/// word (`0126fdf4`: 0, the C locale). The critical section's import thunks, `atexit` and the
/// manager's allocate and free are stand-ins.
fn world(e: &mut Engine) {
    for page in [0x011f_3000, 0x011f_6000, 0x0126_f000, 0x0101_2000] {
        e.map(page, 0x1000);
    }
    // The double 0.0 in `.rdata` (`01012060`) the getters and sums compare with.
    e.mem.set_f64(0x0101_2060, 0.0);
    for stub in [0x00fd_f05c, 0x00fd_f100, 0x00ec_658f, 0x00aa_4060] {
        e.register(stub, |_, _| Ret::default());
    }
    e.register(0x00aa_3e40, |e, a| ret(e.mem.alloc(a[1].max(4))));
}

/// The trait compare: signed ids, both ways round.
#[test]
fn compare_values_diff() {
    const IDS: &[u32] = &[0, 1, 0xfa1, 0x2710, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff];
    Diff::new(engine_core::entry!(
        0x00a0_1160,
        dispatch::tile::compare_values(Ptr<_>, Ptr<_>) -> i32
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&[Input::Of(IDS), Input::Of(IDS)])
    .setup(|e, case| {
        let mut slots = vec![];
        for i in 0..2 {
            let value = e.mem.alloc(0x14);
            e.mem.set_u32(value, case.word(i));
            let slot = e.mem.alloc(4);
            e.mem.set_u32(slot, value);
            slots.push(slot);
        }
        slots
    })
    .run()
    .assert_same();
}

/// A tile (0x38 bytes) whose trait array (`BSSimpleArray` at +0x10: vtable, buffer, size, capacity)
/// holds `count` values with the ids `first + 3 * i` in order, in a buffer of `capacity` slots; its
/// vtable's allocate (+4), free (+8) and reallocate (+0xC: a new block with the elements copied)
/// are stand-ins. Each value is owned by the tile (+4), its
/// float `i` (+8).
fn tile_with(e: &mut Engine, count: u32, capacity: u32, first: i32) -> u32 {
    let ids: Vec<u32> = (0..count)
        .map(|i| first.wrapping_add(3 * i as i32) as u32)
        .collect();
    tile_with_ids(e, &ids, capacity)
}

/// A tile as [`tile_with`]'s whose values have the ids `ids`, in order.
fn tile_with_ids(e: &mut Engine, ids: &[u32], capacity: u32) -> u32 {
    let count = ids.len() as u32;
    let vtable = e.mem.alloc(0x10);
    e.mem.set_u32(vtable + 4, 0x00fd_0104);
    e.mem.set_u32(vtable + 8, 0x00fd_0108);
    e.mem.set_u32(vtable + 0xc, 0x00fd_010c);
    e.register(0x00fd_0104, |e, a| ret(e.mem.alloc(a[1].max(1) * 4)));
    e.register(0x00fd_0108, |_, _| Ret::default());
    e.register(0x00fd_010c, |e, a| {
        let moved = e.mem.alloc(a[2].max(1) * 4);
        let bytes = e.mem.bytes(a[1], e.mem.u32(a[0] + 8) * 4);
        e.mem.write(moved, &bytes);
        ret(moved)
    });
    let tile = e.mem.alloc(0x38);
    let buffer = if capacity == 0 {
        0
    } else {
        e.mem.alloc(capacity * 4)
    };
    e.mem.set_u32(tile + 0x10, vtable);
    e.mem.set_u32(tile + 0x14, buffer);
    e.mem.set_u32(tile + 0x18, count);
    e.mem.set_u32(tile + 0x1c, capacity);
    for i in 0..count {
        let value = e.mem.alloc(0x14);
        e.mem.set_u32(value, ids[i as usize]);
        e.mem.set_u32(value + 4, tile);
        e.mem.set_f32(value + 8, i as f32);
        e.mem.set_u32(buffer + 4 * i, value);
    }
    tile
}

/// The trait ids asked for: below, on and between the ids of [`tile_with`] (from 0xfa0), and
/// past them; signed extremes.
const ASKED: &[u32] = &[
    0,
    0xf9f,
    0xfa0,
    0xfa1,
    0xfa3,
    0xfa6,
    0xfb5,
    0xfb8,
    0xfc4,
    0x2710,
    0x8000_0000,
    0x8000_0001,
    0xffff_fff0,
    0xffff_ffff,
    0x7fff_ffff,
];

/// Trait arrays beside [`tile_with`]'s: negative ids first, then positive ones, and an id three times
/// over, so which of the equal elements `bsearch` answers (its choice of middle) shows.
const NEGATIVE_IDS: &[u32] = &[0x8000_0000, 0xffff_fff0, 0xfa0, 0xfa3, 0xfa6];
const REPEATED_IDS: &[u32] = &[0xfa0, 0xfa3, 0xfa3, 0xfa3, 0xfa6];

/// The tile of shape `code`: [`NEGATIVE_IDS`] (100), [`REPEATED_IDS`] (101), else [`tile_with`]'s
/// `code` values from 0xfa0 in `capacity` slots.
fn shaped_tile(e: &mut Engine, code: u32, capacity: u32) -> u32 {
    match code {
        100 => tile_with_ids(e, NEGATIVE_IDS, 5),
        101 => tile_with_ids(e, REPEATED_IDS, 5),
        _ => tile_with(e, code, capacity, 0xfa0),
    }
}

/// The sorted find with the trait compare: arrays of 0 to 9 values (odd and even counts, so both of
/// `bsearch`'s middles), every id found and missed; an empty array with no buffer.
#[test]
fn sorted_find_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_cb90,
        dispatch::tile::sorted_find(Ptr<_>, u32, u32) -> i32
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5, 8, 9, 100, 101]),
        Input::Of(ASKED),
        Input::Of(&[0, 1]),
    ])
    .setup(|e, case| {
        world(e);
        let count = case.word(0);
        let capacity = if case.word(2) == 1 && count == 0 {
            0
        } else {
            count.max(1)
        };
        let tile = shaped_tile(e, count, capacity);
        let key = e.mem.alloc(0x14);
        e.mem.set_u32(key, case.word(1));
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, key);
        vec![tile + 0x10, slot, 0x00a0_1160]
    })
    .run()
    .assert_same();
}

/// The C runtime's `bsearch`, typed in the family for the sorted find, against its translation: the
/// trait compare over [`shaped_tile`]'s arrays (both middles, negative and repeated ids), and a
/// stand-in compare over 8-byte records holding the same ids. And the invalid arguments: a null base with
/// elements, a width of 0, no compare (`errno` set to `EINVAL` and the report made).
#[test]
fn bsearch_diff() {
    Diff::new(engine_core::entry!(
        0x00ec_716d,
        dispatch::tile::bsearch(u32, u32, u32, u32, u32) -> u32
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .max_cases(700)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5, 8, 100, 101]),
        Input::Of(ASKED),
        Input::Of(&[4, 8, 0, 1, 2]),
    ])
    .setup(|e, case| {
        world(e);
        // The invalid arguments' report (`00ec7c30`) returns; `errno` is the static slot (no thread
        // data: `__getptd_noexit` answers 0), compared with the rest of memory.
        e.map(0x011c_1000, 0x1000);
        e.register(0x00ec_7c30, |_, _| Ret::default());
        e.register(0x00ed_1165, |_, _| ret(0));
        let code = case.word(0);
        let tile = shaped_tile(e, code, code.max(1));
        let (buffer, size) = (e.mem.u32(tile + 0x14), e.mem.u32(tile + 0x18));
        let key = e.mem.alloc(0x14);
        e.mem.set_u32(key, case.word(1));
        if case.word(2) != 8 {
            let slot = e.mem.alloc(4);
            e.mem.set_u32(slot, key);
            return match case.word(2) {
                // A null base with elements, a width of 0, no compare.
                1 => vec![slot, 0, size.max(3), 4, 0x00a0_1160],
                0 => vec![slot, buffer, size, 0, 0x00a0_1160],
                2 => vec![slot, buffer, size, 4, 0],
                _ => vec![slot, buffer, size, 4, 0x00a0_1160],
            };
        }
        // 8-byte records: the id, then a word; the compare reads the two ids as signed words.
        e.register(0x00fd_0210, |e, a| {
            let (a, b) = (e.mem.i32(a[0]), e.mem.i32(a[1]));
            ret(a.cmp(&b) as i32 as u32)
        });
        let records = e.mem.alloc(size.max(1) * 8);
        for i in 0..size {
            let id = e.mem.u32(e.mem.u32(buffer + 4 * i));
            e.mem.set_u32(records + 8 * i, id);
            e.mem.set_u32(records + 8 * i + 4, 0x50 + i);
        }
        vec![key, records, size, 8, 0x00fd_0210]
    })
    .run()
    .assert_same();
}

/// The sorted find with a compare that moves the array's buffer to a new block on its first call (the
/// old one kept as it was, so the search goes on over it): the index is worked out from the buffer read
/// after the search, as the exe does (`00a0cbcd`), on both sides.
#[test]
fn sorted_find_moving_compare_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_cb90,
        dispatch::tile::sorted_find(Ptr<_>, u32, u32) -> i32
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&[Input::Of(&[1, 2, 5, 100, 101]), Input::Of(ASKED)])
    .setup(|e, case| {
        world(e);
        let code = case.word(0);
        let tile = shaped_tile(e, code, code.max(1));
        let array = tile + 0x10;
        let moved = std::rc::Rc::new(std::cell::Cell::new(false));
        e.register_double(0x00fd_0230, move |e, a| {
            if !moved.get() {
                moved.set(true);
                let (old, size) = (e.mem.u32(array + 4), e.mem.u32(array + 8));
                let new = e.mem.alloc(size.max(1) * 4);
                let words = e.mem.bytes(old, size * 4);
                e.mem.write(new, &words);
                e.mem.set_u32(array + 4, new);
            }
            let id = |e: &Engine, slot: u32| e.mem.i32(e.mem.u32(slot));
            ret(id(e, a[0]).cmp(&id(e, a[1])) as i32 as u32)
        });
        let key = e.mem.alloc(0x14);
        e.mem.set_u32(key, case.word(1));
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, key);
        vec![array, slot, 0x00fd_0230]
    })
    .run()
    .assert_same();
}

/// The sorted find with another compare (a stand-in on the ids' low bytes): `bsearch` itself, as
/// the translation asks, its compares by address.
#[test]
fn sorted_find_other_compare_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_cb90,
        dispatch::tile::sorted_find(Ptr<_>, u32, u32) -> i32
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&[Input::Of(&[1, 2, 4, 5, 9]), Input::Of(ASKED)])
    .setup(|e, case| {
        world(e);
        e.register(0x00fd_0200, |e, a| {
            let id = |e: &Engine, slot: u32| i32::from(e.mem.u8(e.mem.u32(slot)));
            ret((id(e, a[0]) - id(e, a[1])) as u32)
        });
        let count = case.word(0);
        let tile = tile_with(e, count, count, 0xfa0);
        let key = e.mem.alloc(0x14);
        e.mem.set_u32(key, case.word(1));
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, key);
        vec![tile + 0x10, slot, 0x00fd_0200]
    })
    .run()
    .assert_same();
}

/// The sorted insert with the trait compare: new ids below, between and above the array's, one
/// already there, into arrays with room and full ones (the insert grows the buffer).
#[test]
fn sorted_insert_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_caf0,
        dispatch::tile::sorted_insert(Ptr<_>, u32, u32)
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_cd50])
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 7, 8, 9, 100, 101]),
        Input::Of(&[0, 1]),
        Input::Of(ASKED),
    ])
    .setup(|e, case| {
        world(e);
        let count = case.word(0);
        let capacity = if case.word(1) == 1 {
            count.max(1)
        } else {
            count + 4
        };
        let tile = shaped_tile(e, count, capacity);
        let value = e.mem.alloc(0x14);
        e.mem.set_u32(value, case.word(2));
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, value);
        vec![tile + 0x10, slot, 0x00a0_1160]
    })
    .run()
    .assert_same();
}

/// `GetValueQ`, `GetValue` and `GetOrCreateValue` on tiles of 0 to 9 traits and the arrays with
/// negative and repeated ids, each id found and missed, the search keys' first use (guard clear) and
/// a later one.
#[test]
fn tile_get_value_diff() {
    let inputs = [
        Input::Of(&[0, 1, 2, 5, 8, 9, 100, 101]),
        Input::Of(ASKED),
        Input::Of(&[0, 1]),
    ];
    let setup = |e: &mut Engine, case: &engine_core::typed::diff::Case| {
        world(e);
        let count = case.word(0);
        let tile = shaped_tile(e, count, count + (count & 1));
        if case.word(2) == 1 {
            // The keys' guards already set (a later call): the keys hold an old id.
            e.mem.set_u32(0x011f_3388, 1);
            e.mem.set_u32(0x011f_33a0, 1);
            e.mem.set_u32(0x011f_3374, 0xfa3);
            e.mem.set_u32(0x011f_338c, 0xfa3);
        }
        vec![tile, case.word(1)]
    };
    Diff::new(engine_core::entry!(
        0x00a0_0e90,
        dispatch::tile::get_value_q(Ptr<_>, i32) -> Ptr
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&inputs)
    .setup(setup)
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00a0_0f30,
        dispatch::tile::get_value(Ptr<_>, i32) -> Ptr
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&inputs)
    .setup(setup)
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00a0_1000,
        dispatch::tile::get_or_create_value(Ptr<_>, i32) -> Ptr
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_cd50])
    .inputs(&inputs)
    .setup(setup)
    .run()
    .assert_same();
}

/// Names for the menu string map's hash and compare: empty, mixed case, digits and punctuation,
/// bytes from 0x80 (negative as signed bytes), one long name.
const NAMES: &[&[u8]] = &[
    b"",
    b"a",
    b"A",
    b"_Title",
    b"_TITLE",
    b"bracket_left",
    b"\x80\xc9t\xe9",
    b"\xff",
    b"font3_Main_Menu_Button_Text_Shadow_99",
];

/// The menu string map's hash and compare (with the typed C runtime's `tolower` and `_stricmp`)
/// against their translations, in the C locale.
#[test]
fn menu_string_map_diff() {
    let names = |e: &mut Engine| -> Vec<u32> {
        NAMES
            .iter()
            .map(|n| {
                let at = e.mem.alloc(n.len() as u32 + 1);
                e.mem.write(at, n);
                e.mem.set_u8(at + n.len() as u32, 0);
                at
            })
            .collect()
    };
    Diff::new(engine_core::entry!(
        0x00a0_d1b0,
        dispatch::tile::string_hash(Ptr<_>, Ptr<_>) -> u32
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .inputs(&[
        Input::Of(&[1, 37, 1009, 0x7fff_ffff]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    ])
    .setup(move |e, case| {
        world(e);
        let map = e.mem.alloc(0x14);
        e.mem.set_u32(map + 4, case.word(0));
        let at = names(e)[case.word(1) as usize];
        vec![map, at]
    })
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00a0_d180,
        dispatch::tile::string_keys_equal(Ptr<_>, Ptr<_>, Ptr<_>) -> bool
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    ])
    .setup(move |e, case| {
        world(e);
        let all = names(e);
        let map = e.mem.alloc(0x14);
        vec![map, all[case.word(0) as usize], all[case.word(1) as usize]]
    })
    .run()
    .assert_same();
}

/// The integer maps' hash and compare.
#[test]
fn int_map_slots_diff() {
    const KEYS: &[u32] = &[0, 1, 36, 37, 0x0123_4567, 0x8000_0000, u32::MAX];
    Diff::new(engine_core::entry!(
        0x00a0_d260,
        dispatch::tile::int_hash(Ptr<_>, u32) -> u32
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&[Input::Of(&[1, 37, 5701]), Input::Of(KEYS)])
    .setup(|e, case| {
        let map = e.mem.alloc(0x14);
        e.mem.set_u32(map + 4, case.word(0));
        vec![map, case.word(1)]
    })
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00ae_8dc0,
        dispatch::tile::int_keys_equal(u32, u32, u32) -> bool
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&[Input::Of(KEYS), Input::Of(KEYS)])
    .setup(|e, case| {
        let map = e.mem.alloc(0x14);
        vec![map, case.word(0), case.word(1)]
    })
    .run()
    .assert_same();
}

/// The stand-in "other slots" a map's vtable may hold, with answers the exe's integer slots never
/// give: the next bucket, keys equal when they differ only in bit 0, and a node fill that also
/// moves the bucket array to a new block (zeroing the old one).
fn other_slots(e: &mut Engine) {
    e.register(0x00fd_0304, |e, a| {
        ret(a[1].wrapping_add(1) % e.mem.u32(a[0] + 4))
    });
    e.register(0x00fd_0308, |_, a| ret(u32::from((a[1] | 1) == (a[2] | 1))));
    e.register(0x00fd_030c, |e, a| {
        e.mem.set_u32(a[1] + 4, a[2]);
        e.mem.set_u32(a[1] + 8, a[3]);
        let bytes = e.mem.u32(a[0] + 4) * 4;
        let old = e.mem.u32(a[0] + 8);
        let moved = e.mem.alloc(bytes);
        let buckets = e.mem.bytes(old, bytes);
        e.mem.write(moved, &buckets);
        e.mem.write(old, &vec![0u8; bytes as usize]);
        e.mem.set_u32(a[0] + 8, moved);
        Ret::default()
    });
    e.register(0x00fd_0314, |e, _| ret(e.mem.alloc(12)));
    // The node release a remove asks for (clear value +0x10, delete +0x18): nothing to do here.
    e.register(0x00fd_0310, |_, _| Ret::default());
    e.register(0x00fd_0318, |_, _| Ret::default());
}

/// A pointer map of `buckets` buckets with the integer maps' vtable (`01094dfc`'s slots: hash
/// `00a0d260`, compare `00ae8dc0`, node fill `00a1e720`; a stand-in new node), or with
/// [`other_slots`]; holding nodes keyed `0x100 + 7 * i` for `i < count`, value `0x50 + i`, each at
/// the head of the bucket the map's own hash gives.
fn int_map(e: &mut Engine, at: u32, buckets: u32, count: u32, own_slots: bool) {
    other_slots(e);
    let vtable = e.mem.alloc(0x20);
    let slots: [u32; 7] = if own_slots {
        [
            0,
            0x00fd_0304,
            0x00fd_0308,
            0x00fd_030c,
            0x00fd_0310,
            0x00fd_0314,
            0x00fd_0318,
        ]
    } else {
        [
            0,
            0x00a0_d260,
            0x00ae_8dc0,
            0x00a1_e720,
            0x00fd_0310,
            0x00fd_0314,
            0x00fd_0318,
        ]
    };
    for (i, s) in slots.iter().enumerate() {
        e.mem.set_u32(vtable + 4 * i as u32, *s);
    }
    let table = e.mem.alloc(buckets * 4);
    e.mem.set_u32(at, vtable);
    e.mem.set_u32(at + 4, buckets);
    e.mem.set_u32(at + 8, table);
    for i in 0..count {
        let key = 0x100 + 7 * i;
        let index = if own_slots {
            (key + 1) % buckets
        } else {
            key % buckets
        };
        let node = e.mem.alloc(12);
        let bucket = table + 4 * index;
        let head = e.mem.u32(bucket);
        e.mem.set_u32(node, head);
        e.mem.set_u32(node + 4, key);
        e.mem.set_u32(node + 8, 0x50 + i);
        e.mem.set_u32(bucket, node);
    }
    e.mem.set_u32(at + 0xc, count);
}

/// A pointer map's `SetAt`: keys there (the value replaced) and new ones (a node made, filled, put
/// at its bucket's head, counted), one bucket and several, the integer maps' slots and other ones.
#[test]
fn map_set_at_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_c6c0,
        dispatch::tile::map_set_at(Ptr<_>, u32, u32)
    ))
    .family(&dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .inputs(&[
        Input::Of(&[1, 3, 37]),
        Input::Of(&[0, 1]),
        Input::Of(&[0x100, 0x101, 0x107, 0x10e, 0x115, 0x200, u32::MAX]),
    ])
    .setup(|e, case| {
        let map = e.mem.alloc(0x14);
        int_map(e, map, case.word(0), 4, case.word(1) == 1);
        vec![map, case.word(2), 0x77]
    })
    .run()
    .assert_same();
}

/// The reaction map (`011f3358`, integer slots, `buckets` buckets) with lists for the values in
/// `lists`: each list's owners in order (reaction nodes: owner, next).
fn reaction_map(e: &mut Engine, buckets: u32, lists: &[(u32, Vec<u32>)]) {
    int_map(e, 0x011f_3358, buckets, 0, false);
    for (value, owners) in lists {
        let mut head = 0;
        for owner in owners.iter().rev() {
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, *owner);
            e.mem.set_u32(node + 4, head);
            head = node;
        }
        e.call(0x00a0_c6c0, &args![0x011f_3358u32, *value, head]);
    }
}

/// `Tile::AddReaction`: a value with no list (the node becomes its list), and lists where the owner
/// is absent, only first, only in the middle, last, or the only node (the exe checks the last node
/// only: a duplicate earlier in the list is appended again).
#[test]
fn add_reaction_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_a130,
        dispatch::tile::add_reaction(Ptr<_>, Ptr<_>)
    ))
    .family(&dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5]), Input::Of(&[1, 37])])
    .setup(|e, case| {
        world(e);
        let value = e.mem.alloc(0x14);
        let owner = e.mem.alloc(0x14);
        let (a, b) = (e.mem.alloc(0x14), e.mem.alloc(0x14));
        let lists: Vec<Vec<u32>> = vec![
            vec![],
            vec![a, b],
            vec![owner, a],
            vec![a, owner, b],
            vec![a, b, owner],
            vec![owner],
        ];
        let list = &lists[case.word(0) as usize];
        let other = e.mem.alloc(0x14);
        let mut given = vec![(other, vec![a])];
        if !list.is_empty() {
            given.push((value, list.clone()));
        }
        reaction_map(e, case.word(1), &given);
        vec![value, owner]
    })
    .run()
    .assert_same();
}

/// `Tile::Value::ClearActions`: a value whose actions read other values (slot +4 a stand-in giving
/// the word at +0xC) or none (0); the read values' reaction lists hold this value first, in the
/// middle, alone, twice, or not at all, so the map's entry is kept, set to a new head or removed.
#[test]
fn clear_actions_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_9200,
        dispatch::tile::clear_actions(Ptr<_>)
    ))
    .family(&dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5]), Input::Of(&[1, 37])])
    .setup(|e, case| {
        world(e);
        let this = e.mem.alloc(0x14);
        let action_vtable = e.mem.alloc(0x10);
        e.mem.set_u32(action_vtable + 4, 0x00fd_0404);
        e.register(0x00fd_0404, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        let read: Vec<u32> = (0..3).map(|_| e.mem.alloc(0x14)).collect();
        let x = e.mem.alloc(0x14);
        let shapes: Vec<Vec<u32>> = vec![
            vec![this, x],
            vec![x, this, x],
            vec![this],
            vec![this, x, this],
            vec![x],
            vec![],
        ];
        let shape = case.word(0) as usize;
        let mut lists = vec![];
        for (k, value) in read.iter().enumerate() {
            let owners = &shapes[(shape + k) % shapes.len()];
            if !owners.is_empty() {
                lists.push((*value, owners.clone()));
            }
        }
        reaction_map(e, case.word(1), &lists);
        // Actions: one per read value, then one reading nothing.
        let mut head = 0;
        for referenced in read.iter().copied().chain([0]).rev() {
            let action = e.mem.alloc(0x10);
            e.mem.set_u32(action, action_vtable);
            e.mem.set_u32(action + 8, head);
            e.mem.set_u32(action + 0xc, referenced);
            head = action;
        }
        e.mem.set_u32(this + 0x10, head);
        vec![this]
    })
    .run()
    .assert_same();
}

/// The typed map lookup (`00a1e380`) on the menus' maps: the menu string map (hash `00a0d1b0`,
/// compare `00a0d180`) and the integer maps (`00a0d260`, `00ae8dc0`), keys there and not.
#[test]
fn map_lookup_on_menu_maps_diff() {
    Diff::new(engine_core::entry!(
        0x00a1_e380,
        dispatch::refs::map_lookup(Ptr<_>, u32, Ptr<u32>) -> u8
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .family(&leaf::LEAVES)
    .inputs(&[
        Input::Of(&[1, 37]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    ])
    .setup(|e, case| {
        world(e);
        let buckets = case.word(0);
        let map = e.mem.alloc(0x14);
        let out = e.mem.alloc(4);
        e.mem.set_u32(out, 0xdead);
        if case.word(1) == 0 {
            // The menu string map: its vtable at `01094d9c` holds hash and compare at +4, +8.
            let vtable = e.mem.alloc(0x20);
            e.mem.set_u32(vtable + 4, 0x00a0_d1b0);
            e.mem.set_u32(vtable + 8, 0x00a0_d180);
            let table = e.mem.alloc(buckets * 4);
            e.mem.set_u32(map, vtable);
            e.mem.set_u32(map + 4, buckets);
            e.mem.set_u32(map + 8, table);
            let mut keys = vec![];
            for (i, n) in NAMES.iter().enumerate() {
                let at = e.mem.alloc(n.len() as u32 + 1);
                e.mem.write(at, n);
                e.mem.set_u8(at + n.len() as u32, 0);
                keys.push(at);
                // The odd names are in the map under an upper-case copy.
                if i % 2 == 1 {
                    let copy = e.mem.alloc(n.len() as u32 + 1);
                    let upper: Vec<u8> = n.iter().map(u8::to_ascii_uppercase).collect();
                    e.mem.write(copy, &upper);
                    e.mem.set_u8(copy + n.len() as u32, 0);
                    let index = e.call(0x00a0_d1b0, &args![map, copy]).u32();
                    let node = e.mem.alloc(12);
                    let bucket = table + 4 * index;
                    let head = e.mem.u32(bucket);
                    e.mem.set_u32(node, head);
                    e.mem.set_u32(node + 4, copy);
                    e.mem.set_u32(node + 8, 0x60 + i as u32);
                    e.mem.set_u32(bucket, node);
                }
            }
            vec![map, keys[case.word(2) as usize], out]
        } else {
            int_map(e, map, buckets, 5, false);
            vec![map, 0x100 + 7 * case.word(2) / 2, out]
        }
    })
    .run()
    .assert_same();
}

/// Where the stand-ins below log what they are given: a count, then the words.
const LOG: u32 = 0x0300_0000;

/// A stand-in that appends its words to the log at [`LOG`] (so the two sides' memory compares what
/// each was given, in order) and answers nothing.
fn logging(e: &mut Engine, a: &[u32]) -> Ret {
    let n = e.mem.u32(LOG);
    for (i, w) in (0u32..).zip(a) {
        e.mem.set_u32(LOG + 4 + 4 * (n + i), *w);
    }
    e.mem.set_u32(LOG, n + a.len() as u32);
    Ret::default()
}

/// [`world`] with the log mapped and `Tile::Value::SetFloat` (`00a0a270`) and `SetString`
/// (`00a0a300`) as logging stand-ins: what reaches them is what the setters decide; their own work
/// has its own translations. `CalculateValue` (`00a09410`) is typed (a member of the dispatch
/// family), so no setup may stand in for it: the real one runs on both sides.
fn setter_world(e: &mut Engine) {
    world(e);
    e.map(LOG, 0x4000);
    for stub in [0x00a0_a270, 0x00a0_a300] {
        e.register(stub, logging);
    }
}

/// The ids the setters and getters are asked for: on, between and past [`tile_with`]'s ids.
const SET_IDS: &[u32] = &[
    0xf9f,
    0xfa0,
    0xfa1,
    0xfa3,
    0xfb8,
    0x2710,
    0x8000_0000,
    0xffff_ffff,
];

/// Floats for the values' own floats: zeros of both signs, a NaN, ordinary ones.
const VALUE_FLOATS: &[u32] = &[
    0,
    0x8000_0000,
    0x3f80_0000,
    0xbf00_0000,
    0x7fc0_0000,
    0x4b80_0001,
];

/// A tile of [`shaped_tile`]'s shape `code` whose values all hold the float `float` and, when
/// `with_string`, a string of their own.
fn valued_tile(e: &mut Engine, code: u32, float: u32, with_string: bool) -> u32 {
    let tile = shaped_tile(e, code, code.max(1) + 2);
    let (buffer, size) = (e.mem.u32(tile + 0x14), e.mem.u32(tile + 0x18));
    for i in 0..size {
        let value = e.mem.u32(buffer + 4 * i);
        e.mem.set_u32(value + 8, float);
        if with_string {
            let text = e.mem.alloc(8);
            e.mem.write(text, b"ab\0");
            e.mem.set_u32(value + 0xc, text);
        }
    }
    tile
}

/// `GetFloat`, `GetString` and `IsTrue` (`00a011b0`, `00a011f0`, `00a01230`): the trait there or
/// not, its float 0.0, -0.0, a NaN or ordinary, with a string or none.
#[test]
fn tile_getters_diff() {
    let inputs = [
        Input::Of(&[0, 1, 2, 5, 9, 100, 101]),
        Input::Of(SET_IDS),
        Input::Of(VALUE_FLOATS),
        Input::Flag,
    ];
    let setup = |e: &mut Engine, case: &engine_core::typed::diff::Case| {
        world(e);
        let tile = valued_tile(e, case.word(0), case.word(2), case.word(3) == 1);
        vec![tile, case.word(1)]
    };
    Diff::new(engine_core::entry!(
        0x00a0_11b0,
        dispatch::tile::get_float(Ptr<_>, i32) -> f32
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_0f30])
    .inputs(&inputs)
    .setup(setup)
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00a0_11f0,
        dispatch::tile::get_string(Ptr<_>, i32) -> Ptr
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_0f30])
    .inputs(&inputs)
    .setup(setup)
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00a0_1230,
        dispatch::tile::is_true(Ptr<_>, i32) -> bool
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_0f30])
    .inputs(&inputs)
    .setup(setup)
    .run()
    .assert_same();
}

/// Set once [`null_value_allocation`] has given the null value.
const NULL_VALUE_GIVEN: u32 = LOG + 0x3ff0;

/// The allocate stand-in that answers null for the first value-sized (0x14) block.
fn null_value_allocation(e: &mut Engine, a: &[u32]) -> Ret {
    if a[1] == 0x14 && e.mem.u32(NULL_VALUE_GIVEN) == 0 {
        e.mem.set_u32(NULL_VALUE_GIVEN, 1);
        return ret(0);
    }
    ret(e.mem.alloc(a[1].max(4)))
}

/// The setters' case: the value argument a float for `SetFloat`, a string pointer (a word) for
/// `SetString`.
fn setter_case(e: &mut Engine, case: &engine_core::typed::diff::Case, value: u32) -> Vec<u32> {
    setter_world(e);
    let code = case.word(0);
    let capacity = if case.word(3) == 1 {
        code.max(1)
    } else {
        code + 3
    };
    let tile = shaped_tile(e, code, capacity);
    if case.word(5) == 1 {
        // The new value's allocation answers null (`GetOrCreateValue` then answers null, its
        // writes landing on a mapped page 0): the setters' null check is what runs.
        e.map(0, 0x1000);
        e.mem.set_u32(NULL_VALUE_GIVEN, 0);
        e.register(0x00aa_3e40, null_value_allocation);
    }
    vec![tile, case.word(1), value, case.word(4)]
}

/// `SetFloat` and `SetString` (`00a012d0`, `00a01350`): the trait there or made (into arrays with
/// room and full ones), each float, both flags, the new value's allocation answering null or not; the
/// value's own setter gets the value, the word and the flag under `Tile::Lock`.
#[test]
fn tile_setters_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_12d0,
        dispatch::tile::set_float(Ptr<_>, i32, f32, u8)
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_1000, 0x00a0_cd50])
    .inputs(&[
        Input::Of(&[0, 1, 2, 5, 100, 101]),
        Input::Of(SET_IDS),
        Input::F32,
        Input::Flag,
        Input::Of(&[0, 1]),
        Input::Flag,
    ])
    .setup(|e, case| setter_case(e, case, case.f32(2).to_bits()))
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00a0_1350,
        dispatch::tile::set_string(Ptr<_>, i32, Ptr<_>, u8)
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_1000, 0x00a0_cd50])
    .inputs(&[
        Input::Of(&[0, 1, 2, 5, 100, 101]),
        Input::Of(SET_IDS),
        Input::Of(&[0, 0x0300_3f00]),
        Input::Flag,
        Input::Of(&[0, 1]),
        Input::Flag,
    ])
    .setup(|e, case| setter_case(e, case, case.word(2)))
    .run()
    .assert_same();
}

/// `SetInt` (`00700320`): integers a float holds exactly and ones it rounds (ties to even both
/// ways), the extremes; the trait there or made.
#[test]
fn tile_set_int_diff() {
    const INTS: &[u32] = &[
        0,
        1,
        0xffff_ffff,
        0x0100_0001,
        0x0100_0003,
        0x7fff_ffff,
        0x8000_0000,
        0xfeff_ffff,
    ];
    Diff::new(engine_core::entry!(
        0x0070_0320,
        dispatch::tile::set_int(Ptr<_>, i32, i32)
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_1000, 0x00a0_cd50])
    .inputs(&[
        Input::Of(&[0, 2, 5, 101]),
        Input::Of(SET_IDS),
        Input::Of(INTS),
    ])
    .setup(|e, case| {
        setter_world(e);
        let tile = shaped_tile(e, case.word(0), case.word(0) + 1);
        vec![tile, case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
}

/// `Tile::Poke` (`00a01290`): the value set and set back to 0.0, the trait there or made.
#[test]
fn tile_poke_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_1290,
        dispatch::tile::poke(Ptr<_>, i32, f32)
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_12d0, 0x00a0_1000, 0x00a0_cd50])
    .inputs(&[Input::Of(&[0, 2, 5, 101]), Input::Of(SET_IDS), Input::F32])
    .setup(|e, case| {
        setter_world(e);
        let tile = shaped_tile(e, case.word(0), case.word(0) + 1);
        vec![tile, case.word(1), case.f32(2).to_bits()]
    })
    .run()
    .assert_same();
}

/// The absolute position sums (`00a013d0`, `00a01440`): a tile under a chain of none to three
/// ancestors, each with an `x`, a `y` and a `locus` of 0.0, -0.0, 1.0 or a NaN (or no `locus` trait,
/// or no traits at all), the positions from the case's floats; `y` a NaN on every tile (each its own
/// payload) or ordinary. The NaN-plus-NaN sums compare bit for bit: the typed add keeps the
/// translation's operand order.
#[test]
fn tile_absolute_position_diff() {
    let setup = |e: &mut Engine, case: &engine_core::typed::diff::Case| {
        world(e);
        let depth = case.word(0);
        let loci = case.word(1);
        let position = case.f32(2).to_bits();
        let nan_y = case.word(3) == 1;
        let make = |e: &mut Engine, locus: u32, k: u32| -> u32 {
            match locus {
                // No traits at all.
                4 => tile_with_ids(e, &[], 1),
                // `x` and `y` but no `locus`.
                5 => tile_with_ids(e, &[0xfa1, 0xfa2], 2),
                _ => {
                    let tile = tile_with_ids(e, &[0xfa1, 0xfa2, 0xfa8], 3);
                    let buffer = e.mem.u32(tile + 0x14);
                    let value = |e: &Engine, i: u32| e.mem.u32(buffer + 4 * i);
                    let x = value(e, 0);
                    e.mem.set_u32(x + 8, position ^ (k << 2));
                    let y = value(e, 1);
                    if nan_y {
                        // A NaN `y` on every tile, each with its own payload: two NaNs added.
                        e.mem.set_u32(y + 8, 0x7fc0_1000 | (k + 1));
                    } else {
                        e.mem.set_f32(y + 8, 0.25 * k as f32 - 3.0);
                    }
                    let l = value(e, 2);
                    let bits = [0, 0x8000_0000, 0x3f80_0000, 0x7fc0_0000][locus as usize];
                    e.mem.set_u32(l + 8, bits);
                    tile
                }
            }
        };
        let tile = make(e, 2, 9);
        let mut child = tile;
        for k in 0..depth {
            let parent = make(e, ((loci >> (3 * k)) & 7) % 6, k);
            e.mem.set_u32(child + 0x28, parent);
            child = parent;
        }
        vec![tile]
    };
    let inputs = [
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 0o2, 0o3, 0o22, 0o123, 0o4, 0o5, 0o210, 0o345]),
        Input::F32,
        Input::Flag,
    ];
    Diff::new(engine_core::entry!(
        0x00a0_13d0,
        dispatch::tile::absolute_x(Ptr<_>) -> f32
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_11b0, 0x00a0_0f30])
    .inputs(&inputs)
    .float_sweep(32)
    .setup(setup)
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00a0_1440,
        dispatch::tile::absolute_y(Ptr<_>) -> f32
    ))
    .family(&dispatch::DISPATCH)
    .translation_calls_plainly(&[0x00a0_11b0, 0x00a0_0f30])
    .inputs(&inputs)
    .float_sweep(32)
    .setup(setup)
    .run()
    .assert_same();
}

/// What the action stand-in of [`tile_value_change_event_diff`] does on its call number
/// [`CALC_TRIGGER`] (0 nothing; 1 `AddReaction(value, extra)`, a node at the end of the walked list;
/// 2 the current node taken out of the list and its next word cleared, as if freed and reused;
/// 3 `ValueChangeEvent(other)`), the values it works on and its call count.
const CALC_MODE: u32 = LOG + 0x3f00;
const CALC_VALUE: u32 = LOG + 0x3f04;
const CALC_OTHER: u32 = LOG + 0x3f08;
const CALC_EXTRA: u32 = LOG + 0x3f0c;
const CALC_TRIGGER: u32 = LOG + 0x3f10;
const CALC_CALLS: u32 = LOG + 0x3f14;

/// The operand function (`QFloat`, action vtable slot 0) of each dependent's one action in
/// [`tile_value_change_event_diff`], so it runs inside the real `CalculateValue` of that dependent
/// and changes the reaction lists while `ValueChangeEvent` walks them: logs its words (the action,
/// one per dependent), then on one call does what [`CALC_MODE`] says. It answers 0.0.
fn calculating(e: &mut Engine, a: &[u32]) -> Ret {
    logging(e, a);
    let n = e.mem.u32(CALC_CALLS);
    e.mem.set_u32(CALC_CALLS, n + 1);
    if n != e.mem.u32(CALC_TRIGGER) {
        return Ret::default();
    }
    let value = e.mem.u32(CALC_VALUE);
    match e.mem.u32(CALC_MODE) {
        1 => {
            let extra = e.mem.u32(CALC_EXTRA);
            e.call(0x00a0_a130, &args![value, extra]);
        }
        2 => {
            // Nothing changed the list before this call, so the current node is the n-th.
            let head = e.with_stack(4, |e, out| {
                e.mem.set_u32(out.addr(), 0);
                e.call(0x00a1_e380, &args![0x011f_3358u32, value, out]);
                e.mem.u32(out.addr())
            });
            let (mut previous, mut node) = (0, head);
            for _ in 0..n {
                previous = node;
                node = e.mem.u32(node + 4);
            }
            let next = e.mem.u32(node + 4);
            if previous == 0 {
                e.call(0x00a0_c6c0, &args![0x011f_3358u32, value, next]);
            } else {
                e.mem.set_u32(previous + 4, next);
            }
            e.mem.set_u32(node + 4, 0);
        }
        3 => {
            let other = e.mem.u32(CALC_OTHER);
            e.call(0x00a0_a220, &args![other]);
        }
        _ => {}
    }
    Ret::default()
}

/// `Tile::ValueChangeEvent` (`00a0a220`): a value with no reaction list, and lists of one to three
/// dependents (one named twice), each recalculated with `CalculateValue(false)` in order (the real
/// one, typed on the typed side: it is a member of the dispatch family). Each dependent is a value
/// of a plain tile with one action of an unknown type whose operand function is the stand-in
/// [`calculating`], so its recalculation changes nothing but makes that one call. On its first or
/// second call the stand-in may add a reaction to the walked list, take the current node out of
/// it, or run the event for another value: the walk reads the next node after the call, from the
/// node it just handled, as the exe does (`00a0a258`).
#[test]
fn tile_value_change_event_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_a220,
        dispatch::tile::value_change_event(Ptr<_>)
    ))
    .family(&dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Of(&[1, 37]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1]),
    ])
    .setup(|e, case| {
        setter_world(e);
        // The action class: `QFloat` the stand-in, `QRefValue` answering none (both called
        // through the vtable on both sides, as neither is one of the classes' own).
        e.register(0x00fd_0700, calculating);
        e.register(0x00fd_0704, |_, _| Ret::default());
        let vtable = e.mem.alloc(8);
        e.mem.set_u32(vtable, 0x00fd_0700);
        e.mem.set_u32(vtable + 4, 0x00fd_0704);
        let tile = e.mem.alloc(0x40);
        let dependent = |e: &mut Engine| {
            let value = e.mem.alloc(0x14);
            e.mem.set_u32(value + 4, tile);
            let action = e.mem.alloc(0x10);
            e.mem.set_u32(action, vtable);
            e.mem.set_u32(action + 4, 2099);
            e.mem.set_u32(value + 0x10, action);
            value
        };
        let value = e.mem.alloc(0x14);
        let (a, b, c) = (dependent(e), dependent(e), dependent(e));
        let lists: Vec<Vec<u32>> = vec![vec![], vec![a], vec![a, b], vec![c, a, b], vec![b, b]];
        let list = &lists[case.word(0) as usize];
        let other = e.mem.alloc(0x14);
        let extra = dependent(e);
        let mut given = vec![(other, vec![c])];
        if !list.is_empty() {
            given.push((value, list.clone()));
        }
        reaction_map(e, case.word(1), &given);
        e.mem.set_u32(CALC_MODE, case.word(2));
        e.mem.set_u32(CALC_VALUE, value);
        e.mem.set_u32(CALC_OTHER, other);
        e.mem.set_u32(CALC_EXTRA, extra);
        e.mem.set_u32(CALC_TRIGGER, case.word(3));
        e.mem.set_u32(CALC_CALLS, 0);
        vec![value]
    })
    .run()
    .assert_same();
}

/// `Tile::Lock` and `Tile::Unlock` (`00a044f0`, `00a04500`).
#[test]
fn tile_lock_diff() {
    Diff::new(engine_core::entry!(0x00a0_44f0, dispatch::tile::lock()))
        .family(&dispatch::DISPATCH)
        .setup(|e, _| {
            world(e);
            vec![]
        })
        .run()
        .assert_same();
    Diff::new(engine_core::entry!(0x00a0_4500, dispatch::tile::unlock()))
        .family(&dispatch::DISPATCH)
        .setup(|e, _| {
            world(e);
            vec![]
        })
        .run()
        .assert_same();
}

/// A menu's worth of trait reads and creates, typed against the translation (debug build: the
/// numbers are the call overheads the typed code drops, not a play-build frame time).
#[test]
#[ignore]
fn tile_values_speed() {
    for (name, typed) in [("translation", false), ("typed", true)] {
        let mut best = u128::MAX;
        for _ in 0..3 {
            let mut e = if typed {
                Engine::new()
            } else {
                Engine::new_untyped_for_test()
            };
            world(&mut e);
            let tiles: Vec<u32> = (0..200).map(|_| tile_with(&mut e, 24, 32, 0xfa0)).collect();
            let started = Instant::now();
            for round in 0..25u32 {
                for tile in &tiles {
                    for id in [0xfa0u32, 0xfb5, 0xfd0, 0xfe8, 0x2710 + round % 4] {
                        e.call(0x00a0_0f30, &args![*tile, id]);
                        e.call(0x00a0_1000, &args![*tile, id]);
                    }
                }
            }
            best = best.min(started.elapsed().as_micros());
        }
        println!("GetValue + GetOrCreateValue {name}: 200 tiles, 25 rounds, 5 ids: {best} us");
    }
}

// ---- The action links' tile search (every menu's build) ----

/// The text table (`011f32f4`): a string map of one bucket with the menus' string hash and compare
/// (`00a0d1b0`, `00a0d180`), holding the link words this test gives the path split.
const LINK_WORDS: &[(&[u8], u32)] = &[
    (b"parent", 0x1389),
    (b"me", 0x138a),
    (b"sibling", 0x138c),
    (b"child", 0x138d),
    (b"grandparent", 0x1391),
];

/// A C string on the heap.
fn c_string(e: &mut Engine, text: &[u8]) -> u32 {
    let at = e.mem.alloc(text.len() as u32 + 1);
    e.mem.write(at, text);
    e.mem.set_u8(at + text.len() as u32, 0);
    at
}

fn text_table(e: &mut Engine) {
    let table = 0x011f_32f4;
    let vtable = e.mem.alloc(0x20);
    e.mem.set_u32(vtable + 4, 0x00a0_d1b0);
    e.mem.set_u32(vtable + 8, 0x00a0_d180);
    let buckets = e.mem.alloc(4);
    let mut head = 0;
    for &(word, code) in LINK_WORDS.iter().rev() {
        let key = c_string(e, word);
        let node = e.mem.alloc(12);
        e.mem.set_u32(node, head);
        e.mem.set_u32(node + 4, key);
        e.mem.set_u32(node + 8, code);
        head = node;
    }
    e.mem.set_u32(buckets, head);
    e.mem.set_u32(table, vtable);
    e.mem.set_u32(table + 4, 1);
    e.mem.set_u32(table + 8, buckets);
    e.mem.set_u32(table + 0xc, LINK_WORDS.len() as u32);
}

/// A tile (0x40 bytes) named `name` (none for an empty name) under `parent` (0 for none), added
/// behind its siblings (the children list: head +4, tail +8, count +0xC; a node is next, previous,
/// tile; the name at +0x20, the parent at +0x28).
fn named_tile(e: &mut Engine, parent: u32, name: &[u8]) -> u32 {
    let t = e.mem.alloc(0x40);
    if !name.is_empty() {
        let text = c_string(e, name);
        e.mem.set_u32(t + 0x20, text);
    }
    e.mem.set_u32(t + 0x28, parent);
    if parent != 0 {
        let node = e.mem.alloc(12);
        e.mem.set_u32(node + 8, t);
        let tail = e.mem.u32(parent + 8);
        if tail == 0 {
            e.mem.set_u32(parent + 4, node);
        } else {
            e.mem.set_u32(tail, node);
            e.mem.set_u32(node + 4, tail);
        }
        e.mem.set_u32(parent + 8, node);
        let count = e.mem.u32(parent + 0xc);
        e.mem.set_u32(parent + 0xc, count + 1);
    }
    t
}

/// A menu's tiles under a root: the menu tile `Menu`; under it `Alpha` (children `x` and `Beta`),
/// `beta`, an unnamed tile and `Gamma` (child `deep`, its child `Target`). Answers them in that order
/// (root first).
fn tile_tree(e: &mut Engine) -> Vec<u32> {
    let root = named_tile(e, 0, b"root");
    let menu = named_tile(e, root, b"Menu");
    let alpha = named_tile(e, menu, b"Alpha");
    let x = named_tile(e, alpha, b"x");
    let alpha_beta = named_tile(e, alpha, b"Beta");
    let beta = named_tile(e, menu, b"beta");
    let unnamed = named_tile(e, menu, b"");
    let gamma = named_tile(e, menu, b"Gamma");
    let deep = named_tile(e, gamma, b"deep");
    let target = named_tile(e, deep, b"Target");
    vec![
        root, menu, alpha, x, alpha_beta, beta, unnamed, gamma, deep, target,
    ]
}

/// The names and paths the searches are asked for: found at the start, deeper, in another case,
/// twice (the first in the walk wins), or not at all; every link word, with and without a name.
const SEARCHED: &[&[u8]] = &[
    b"Menu",
    b"beta",
    b"BETA",
    b"Beta",
    b"target",
    b"Target",
    b"x",
    b"nothing",
    b"parent()",
    b"me()",
    b"sibling()",
    b"sibling(gamma)",
    b"sibling(zz)",
    b"child()",
    b"child(Target)",
    b"child(target)",
    b"child(zz)",
    b"grandparent()",
];

/// The searches' words: the tile (by its index in [`tile_tree`]) and the searched text.
fn search_words(e: &mut Engine, case: &engine_core::typed::diff::Case) -> Vec<u32> {
    world(e);
    text_table(e);
    let tiles = tile_tree(e);
    let text = c_string(e, SEARCHED[case.word(1) as usize]);
    vec![tiles[case.word(0) as usize], text]
}

/// The tile search by name, depth first (`00a08f20`), against its translation: from the root,
/// the menu tile, a leaf and deeper tiles, every name.
#[test]
fn find_named_in_tree_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_8f20,
        dispatch::tile::find_named_in_tree(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 7, 9]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
    ])
    .setup(search_words)
    .run()
    .assert_same();
}

/// `Tile::GetChildByName` (`00a03da0`), each level under its own lock, the name compared with case.
#[test]
fn child_by_name_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_3da0,
        dispatch::tile::child_by_name(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 7, 9]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
    ])
    .setup(search_words)
    .run()
    .assert_same();
}

/// The tile an action link's path names (`00a08b20`): every link word with and without a name,
/// and plain names (the depth-first search from the tile's menu tile), from tiles at every depth.
#[test]
fn find_by_path_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_8b20,
        dispatch::tile::find_by_path(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .translation_calls_plainly(&[0x00a0_18d0, 0x00a0_3d40, 0x00a0_3da0, 0x00a0_8f20])
    .inputs(&[
        Input::Of(&[1, 2, 3, 5, 7, 8, 9]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]),
    ])
    .setup(search_words)
    .run()
    .assert_same();
}

// ---- Tile::Value::CalculateValue (every value of a menu's build) ----

/// The value actions this test strings together: (type, operand, reads the other value).
const PROGRAMS: &[&[(u32, f32, bool)]] = &[
    &[],
    &[(2000, 3.0, false), (2001, 0.0, true)],
    &[(2000, 0.0, true), (2003, 2.5, false), (2012, 4.0, false)],
    &[
        (2000, 7.0, false),
        (2004, 2.0, false),
        (2004, 0.0, false),
        (2005, 3.0, false),
        (2006, -1.0, false),
        (2007, 2.0, false),
        (2008, 0.25, false),
        (2009, 0.5, false),
        (2010, -9.0, false),
        (2011, 2.0, false),
    ],
    &[
        (2000, 1.0, false),
        (2013, 1.0, false),
        (2014, 1.0, false),
        (2015, 0.0, false),
        (2016, 2.0, false),
        (2017, 0.0, false),
        (2018, 0.0, true),
        (2019, 0.0, true),
        (2020, 0.0, true),
        (2021, 0.0, true),
        (2022, 0.0, false),
    ],
    &[(2000, 0.0, true), (2002, 1.0, false), (2099, 5.0, false)],
];

/// The words of [`calculate_value_diff`]: a tile (flags from the case: none, loading, its menu
/// deleting, released; vtable slot 0xC a rect's type, slot 0x18 a `PostParse` answering the case's
/// word) holding the value (trait `x`, the class or a user trait; its float from the case) whose
/// actions are a program of [`PROGRAMS`] (float actions and reference actions with the two action
/// classes' vtables) reading another value (float 2.0; with a string or not); empty reaction and
/// user-trait maps with the game's integer slots.
fn calculate_words(e: &mut Engine, case: &engine_core::typed::diff::Case) -> Vec<u32> {
    world(e);
    for page in [0x0109_4000, 0x0101_1000] {
        e.map(page, 0x1000);
    }
    e.mem.set_f64(0x0101_1588, 0.5);
    e.mem.set_u32(0x0109_4c2c, 0x00a0_c000);
    e.mem.set_u32(0x0109_4c30, 0x00ad_87c0);
    e.mem.set_u32(0x0109_4c44, 0x00a0_c020);
    e.mem.set_u32(0x0109_4c48, 0x00a0_c060);
    int_map(e, 0x011f_3358, 37, 0, false);
    int_map(e, 0x011f_3308, 37, 0, false);
    let answer = case.word(3);
    // `Tile::FinalPostParse` (a value that changed on a tile whose `PostParse` answers 0) queues
    // its updates with `Tile::AddNeedsUpdate` (`00a07530`): a logging stand-in, so what it is
    // given compares and the update list's node pool (its lock asks `GetCurrentThreadId`) is left
    // to that function's own translation.
    e.map(LOG, 0x4000);
    e.register(0x00a0_7530, logging);
    e.register(0x00fd_0500, |_, _| ret(0x385));
    e.register_double(0x00fd_0504, move |_, _| ret(answer));
    let vtable = e.mem.alloc(0x20);
    e.mem.set_u32(vtable + 0xc, 0x00fd_0500);
    e.mem.set_u32(vtable + 0x18, 0x00fd_0504);
    let tile = e.mem.alloc(0x40);
    e.mem.set_u32(tile, vtable);
    let flags = [0, 0x10000, 0x4000, 0x2000][case.word(1) as usize];
    e.mem.set_u32(tile + 0x30, flags);
    let other = e.mem.alloc(0x14);
    e.mem.set_u32(other, 0xfa2);
    e.mem.set_u32(other + 4, tile);
    e.mem.set_f32(other + 8, 2.0);
    if case.word(4) != 0 {
        let text = e.mem.alloc(4);
        e.mem.write(text, b"ab\0");
        e.mem.set_u32(other + 0xc, text);
    }
    let value = e.mem.alloc(0x14);
    let id = [0xfa1, 0xfa4, 10_005][case.word(2) as usize];
    e.mem.set_u32(value, id);
    e.mem.set_u32(value + 4, tile);
    e.mem
        .set_f32(value + 8, [0.0f32, 1.5, -3.0][case.word(5) as usize]);
    let mut link = value + 0x10;
    for &(kind, operand, reads) in PROGRAMS[case.word(0) as usize] {
        let action = e.mem.alloc(0x10);
        if reads {
            e.mem.set_u32(action, 0x0109_4c44);
            e.mem.set_u32(action + 0xc, other);
        } else {
            e.mem.set_u32(action, 0x0109_4c2c);
            e.mem.set_f32(action + 0xc, operand);
        }
        e.mem.set_u32(action + 4, kind);
        e.mem.set_u32(link, action);
        link = action + 8;
    }
    vec![value, case.word(6)]
}

/// `Tile::Value::CalculateValue` (`00a09410`) against its translation: every operation, a reference
/// action's float and string, the tile's flags (loading, menu deleting, released), the class trait,
/// `PostParse`'s answers (0 leads to `Tile::FinalPostParse`), with and without the flag. The two
/// action classes' operand functions are read in place.
#[test]
fn calculate_value_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_9410,
        dispatch::tile::calculate_value(Ptr<_>, u8)
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    // `_ctrlfp` (`00ed5abb`), which `floor`'s x87 body reaches with `typed_call!`.
    .family(&leaf::LEAVES)
    .ignore_calls(&[0x00a0_c000, 0x00a0_c020, 0x00a0_c060, 0x00ad_87c0])
    // `Tile::FinalPostParse`: the translation's plain call. And `floor`'s x87 body (`00ed4563`):
    // both sides call `floor` (`00ec6940`) by address; its translation (the translation side) jumps
    // to the body with a plain Rust call, its typed version (the C runtime family, on for the typed
    // side) by address, as `typed_crt.rs`'s `floor` test declares. The body runs on both sides and
    // its own calls (`_ctrlfp` loading and putting back the round-down control word) compare.
    .translation_calls_plainly(&[0x00a0_37e0, 0x00ed_4563])
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1]),
    ])
    .max_cases(600)
    .setup(calculate_words)
    .run()
    .assert_same();
}

/// The scenes of [`calculate_value_scenes_diff`]: a program (type, operand, what it reads: 0 the
/// operand, 1 a plain value (`y`, 2.0), 2 a user trait the user-trait map gives -1, 3 one it gives
/// 5, 4 one not in the map, 5 a plain value with the string "ab") and the value's starting float.
type Scene = (&'static [(u32, f32, u8)], f32);
const SCENES: &[Scene] = &[
    // Parentheses: one pair, a nested pair, and a right parenthesis on an emptied stack.
    (
        &[
            (2000, 4.0, 0),
            (0x7e8, 0.0, 0),
            (2000, 3.0, 0),
            (2001, 2.0, 0),
            (0x7e9, 2001.0, 0),
        ],
        1.0,
    ),
    (
        &[
            (2000, 2.0, 0),
            (0x7e8, 0.0, 0),
            (2000, 3.0, 0),
            (0x7e8, 0.0, 0),
            (2000, 1.0, 0),
            (0x7e9, 2003.0, 0),
            (0x7e9, 2004.0, 0),
        ],
        0.0,
    ),
    (
        &[
            (2000, 6.0, 0),
            (0x7e8, 0.0, 0),
            (2000, 1.0, 0),
            (0x7e9, 2001.0, 0),
            (0x7e9, 2003.0, 0),
        ],
        0.0,
    ),
    // The integers an `eString` trait is rewritten as: 0, 9, 10, a negative, three digits, and
    // no change.
    (&[(2000, 0.0, 0)], 1.5),
    (&[(2000, 9.0, 0)], 0.0),
    (&[(2000, 10.0, 0)], 0.0),
    (&[(2000, -12.0, 0)], 0.0),
    (&[(2000, 123.0, 0)], 0.0),
    (&[(2000, 7.0, 0)], 7.0),
    // User traits: -1 in the map (`GetUnderscoreValue` answering a value for an even number, null
    // for an odd one), 5 in the map, a found one then one not in the map, a string then a float.
    (&[(2001, 0.0, 2)], 3.0),
    (&[(2001, 0.0, 2)], 4.0),
    (&[(2001, 0.0, 3)], 1.0),
    (&[(2001, 0.0, 2), (2001, 0.0, 4)], 2.0),
    (&[(2000, 0.0, 5), (2000, 0.0, 1)], 0.0),
    // Floats: NaN operands, a NaN start, a division by zero then a comparison, ROUND by zero,
    // infinities, a denormal and signed zeros.
    (
        &[
            (2005, f32::NAN, 0),
            (2006, f32::NAN, 0),
            (2012, f32::NAN, 0),
            (2014, f32::NAN, 0),
        ],
        1.0,
    ),
    (&[(2012, 1.0, 0), (2014, 1.0, 0)], f32::NAN),
    (&[(2004, 0.0, 0), (2012, 0.5, 0)], 3.0),
    (&[(2011, 0.0, 0)], 5.0),
    (&[(2001, f32::INFINITY, 0), (2002, f32::INFINITY, 0)], 1.0),
    (&[(2003, 1.0e-40, 0), (2001, -0.0, 0)], -0.0),
    (&[(2005, -0.0, 0), (2006, 0.0, 0), (2015, -0.0, 0)], 0.0),
];

/// The scene indexes of [`SCENES`].
const SCENE_INDEXES: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
];

/// A scene for `CalculateValue`: the tile and action classes of [`calculate_words`] (`PostParse`
/// answering `answer`), the value (trait `trait_id`, starting float `start`, its own string none,
/// "7" or "123") running `program` (an operand whose reads code is 8 is `a`, 9 is `b`), the values
/// it reads, the user-trait map (10010 gives -1, 10011 gives 5), a `GetUnderscoreValue` stand-in, a
/// `sprintf_s` stand-in writing the integer, and with `dependents` two values that react to it (one
/// copying it, one adding 1.0). With `uio`, UI Organizer's operator (its patch at 00a0968c,
/// `uio::operators::operate`) runs each operation, as in default play.
#[allow(clippy::too_many_arguments)]
fn calc_scene(
    e: &mut Engine,
    program: &[(u32, f32, u8)],
    start: f32,
    trait_id: u32,
    answer: u32,
    own_string: u32,
    dependents: bool,
    uio: bool,
    (a, b): (f32, f32),
) -> Vec<u32> {
    world(e);
    if uio {
        e.set_uio(true);
        e.on_site(uio::OPERATOR_SITE, uio::operators::operate);
    }
    for page in [0x0109_4000, 0x0101_1000] {
        e.map(page, 0x1000);
    }
    e.mem.set_f64(0x0101_1588, 0.5);
    e.mem.set_u32(0x0109_4c2c, 0x00a0_c000);
    e.mem.set_u32(0x0109_4c30, 0x00ad_87c0);
    e.mem.set_u32(0x0109_4c44, 0x00a0_c020);
    e.mem.set_u32(0x0109_4c48, 0x00a0_c060);
    e.map(LOG, 0x4000);
    e.register(0x00a0_7530, logging);
    // The parentheses' stack of floats (`BSSimpleArray<float>`, vtable `0106cc64`) grows through
    // its vtable's allocate (+4), free (+8) and reallocate (+0xC): stand-ins as [`tile_with_ids`]'s.
    e.map(0x0106_c000, 0x1000);
    e.mem.set_u32(0x0106_cc68, 0x00fd_0104);
    e.mem.set_u32(0x0106_cc6c, 0x00fd_0108);
    e.mem.set_u32(0x0106_cc70, 0x00fd_010c);
    e.register(0x00fd_0104, |e, a| ret(e.mem.alloc(a[1].max(1) * 4)));
    e.register(0x00fd_0108, |_, _| Ret::default());
    e.register(0x00fd_010c, |e, a| {
        let moved = e.mem.alloc(a[2].max(1) * 4);
        let bytes = e.mem.bytes(a[1], e.mem.u32(a[0] + 8) * 4);
        e.mem.write(moved, &bytes);
        ret(moved)
    });
    // `floor` and `ceil` of a NaN report it through `errno`: the static slot (no thread data:
    // `__getptd_noexit` answers 0, as in the lookup tests above), compared with the rest of memory.
    e.map(0x011c_1000, 0x1000);
    e.register(0x00ed_1165, |_, _| ret(0));
    reaction_map(e, 37, &[]);
    int_map(e, 0x011f_3308, 37, 0, false);
    e.call(0x00a0_c6c0, &args![0x011f_3308u32, 10_010u32, u32::MAX]);
    e.call(0x00a0_c6c0, &args![0x011f_3308u32, 10_011u32, 5u32]);
    e.register(0x00fd_0500, |_, _| ret(0x385));
    e.register_double(0x00fd_0504, move |_, _| ret(answer));
    let vtable = e.mem.alloc(0x20);
    e.mem.set_u32(vtable + 0xc, 0x00fd_0500);
    e.mem.set_u32(vtable + 0x18, 0x00fd_0504);
    let tile = e.mem.alloc(0x40);
    e.mem.set_u32(tile, vtable);
    let make = |e: &mut Engine, id: u32, float: f32, text: &[u8]| {
        let value = e.mem.alloc(0x14);
        e.mem.set_u32(value, id);
        e.mem.set_u32(value + 4, tile);
        e.mem.set_f32(value + 8, float);
        if !text.is_empty() {
            let at = e.mem.alloc(text.len() as u32 + 1);
            e.mem.write(at, text);
            e.mem.set_u8(at + text.len() as u32, 0);
            e.mem.set_u32(value + 0xc, at);
        }
        value
    };
    // `GetUnderscoreValue(tile, trait, number)`: a value for an even number, null for an odd one.
    let found = make(e, 0xfa3, 8.0, b"uv");
    e.register_double(0x00a0_a0b0, move |_, w| {
        ret(if w[2] % 2 == 0 { found } else { 0 })
    });
    // `sprintf_s(buffer, size, "%d", n)`: the integer's digits.
    e.register_double(0x0040_6d00, |e, w| {
        let digits = (w[3] as i32).to_string();
        e.mem.write(w[0], digits.as_bytes());
        e.mem.set_u8(w[0] + digits.len() as u32, 0);
        ret(digits.len() as u32)
    });
    let read = [
        0,
        make(e, 0xfa2, 2.0, b""),
        make(e, 10_010, 2.5, b""),
        make(e, 10_011, 3.0, b"cd"),
        make(e, 10_012, 4.0, b""),
        make(e, 0xfa2, 2.0, b"ab"),
    ];
    let own: &[u8] = match own_string {
        0 => b"",
        1 => b"7",
        _ => b"123",
    };
    let value = make(e, trait_id, start, own);
    let action = |e: &mut Engine, kind: u32, operand: f32, reads: u32, next: u32| {
        let action = e.mem.alloc(0x10);
        if reads != 0 {
            e.mem.set_u32(action, 0x0109_4c44);
            e.mem.set_u32(action + 0xc, reads);
        } else {
            e.mem.set_u32(action, 0x0109_4c2c);
            e.mem.set_f32(action + 0xc, operand);
        }
        e.mem.set_u32(action + 4, kind);
        e.mem.set_u32(action + 8, next);
        action
    };
    let mut head = 0;
    for &(kind, operand, reads) in program.iter().rev() {
        let (operand, reads) = match reads {
            8 => (a, 0),
            9 => (b, 0),
            r => (operand, read[r as usize]),
        };
        head = action(e, kind, operand, reads, head);
    }
    e.mem.set_u32(value + 0x10, head);
    if dependents {
        let copying = make(e, 0xfa1, 0.0, b"");
        let copy = action(e, 2000, 0.0, value, 0);
        e.mem.set_u32(copying + 0x10, copy);
        let adding = make(e, 0xfa2, 1.0, b"");
        let add = action(e, 2001, 1.0, 0, 0);
        e.mem.set_u32(adding + 0x10, add);
        e.call(0x00a0_a130, &args![value, copying]);
        e.call(0x00a0_a130, &args![value, adding]);
    }
    vec![value, 0]
}

/// `Tile::Value::CalculateValue` (`00a09410`) on every scene of [`SCENES`] (parentheses, the
/// `eString` rewrite, user traits through the user-trait map and `GetUnderscoreValue`, special
/// floats) for the traits `x`, `string` and a user trait, `PostParse` answering 0 or 1, the value's
/// own string none or of one or three characters, and with or without values that react to it
/// (recalculated through `ValueChangeEvent`, by address, on both sides), with UI Organizer's
/// operator or the game's own switch.
#[test]
fn calculate_value_scenes_diff() {
    assert_eq!(SCENE_INDEXES.len(), SCENES.len());
    Diff::new(engine_core::entry!(
        0x00a0_9410,
        dispatch::tile::calculate_value(Ptr<_>, u8)
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .family(&leaf::LEAVES)
    .ignore_calls(&[0x00a0_c000, 0x00a0_c020, 0x00a0_c060, 0x00ad_87c0])
    .translation_calls_plainly(&[0x00a0_37e0, 0x00ed_4563])
    .inputs(&[
        Input::Of(SCENE_INDEXES),
        Input::Of(&[0xfa1, 0xfc4, 10_005]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(1600)
    .setup(|e, case| {
        let (program, start) = SCENES[case.word(0) as usize];
        calc_scene(
            e,
            program,
            start,
            case.word(1),
            case.word(2),
            case.word(3),
            case.word(4) == 1,
            case.word(5) == 1,
            (0.0, 0.0),
        )
    })
    .run()
    .assert_same();
}

/// Mixed chains of operations on two swept floats (`float_sweep`: full-mantissa floats, now and then
/// a special one), with UI Organizer's operator or the game's own switch, against the translation
/// bit for bit.
#[test]
fn calculate_value_float_sweep_diff() {
    const CHAINS: &[&[(u32, f32, u8)]] = &[
        &[
            (2000, 0.0, 8),
            (2001, 0.0, 9),
            (2003, 0.0, 8),
            (2002, 0.0, 9),
        ],
        &[
            (2000, 0.0, 8),
            (0x7e8, 0.0, 0),
            (2000, 0.0, 9),
            (2004, 0.0, 8),
            (0x7e9, 2003.0, 0),
            (2011, 0.0, 9),
        ],
        &[
            (2000, 0.0, 8),
            (2005, 0.0, 9),
            (2006, 0.0, 8),
            (2012, 0.0, 9),
            (2001, 0.0, 8),
            (2008, 0.0, 9),
            (2009, 0.0, 8),
            (2010, 0.0, 9),
            // MOD by a fixed operand: one that rounds to 0 divides by zero in the exe (`IDIV`),
            // and in both versions here.
            (2007, 3.0, 0),
        ],
    ];
    Diff::new(engine_core::entry!(
        0x00a0_9410,
        dispatch::tile::calculate_value(Ptr<_>, u8)
    ))
    .family(&dispatch::DISPATCH)
    .family(&crt::CRT)
    .family(&leaf::LEAVES)
    .ignore_calls(&[0x00a0_c000, 0x00a0_c020, 0x00a0_c060, 0x00ad_87c0])
    .translation_calls_plainly(&[0x00a0_37e0, 0x00ed_4563])
    .inputs(&[Input::Of(&[0, 1, 2]), Input::F32, Input::F32, Input::Flag])
    .max_cases(300)
    .float_sweep(96)
    .setup(move |e, case| {
        calc_scene(
            e,
            CHAINS[case.word(0) as usize],
            0.0,
            0xfa1,
            1,
            0,
            false,
            case.word(3) == 1,
            (case.f32(1), case.f32(2)),
        )
    })
    .run()
    .assert_same();
}

// ---- Tile::Value::~Value (every value of a closing menu) ----

/// The words of [`value_destructor_diff`] and [`clear_actions_in_place_diff`]: a value (a string or
/// none) whose three actions are, by the case, of the float action class (`01094c2c`: `QFloat`
/// `00a0c000`, `QRefValue` the plain action's `00ad87c0`), the reference action class (`01094c44`:
/// `00a0c020`, `00a0c060`, the word at +0xC) or a stand-in class (`QRefValue` `00fd0404`, the word
/// at +0xC), reading two other values and nothing; the reaction map holds lists for the other values
/// (with this value first or alone) and, by the case, none, one or three reactions on this value
/// from two owners whose actions of the three classes read this value, another value or nothing.
fn destructor_words(e: &mut Engine, case: &engine_core::typed::diff::Case) -> Vec<u32> {
    world(e);
    e.map(0x0109_4000, 0x1000);
    e.mem.set_u32(0x0109_4c2c, 0x00a0_c000);
    e.mem.set_u32(0x0109_4c30, 0x00ad_87c0);
    e.mem.set_u32(0x0109_4c44, 0x00a0_c020);
    e.mem.set_u32(0x0109_4c48, 0x00a0_c060);
    let stand_in = e.mem.alloc(0x10);
    e.mem.set_u32(stand_in + 4, 0x00fd_0404);
    e.register(0x00fd_0404, |e, a| ret(e.mem.u32(a[0] + 0xc)));
    let classes = [0x0109_4c2c, 0x0109_4c44, stand_in];
    // The class of an action: float actions read nothing (their word is the float 0.0).
    let action = |e: &mut Engine, class: u32, reads: u32, next: u32| {
        let action = e.mem.alloc(0x10);
        e.mem.set_u32(action, class);
        e.mem.set_u32(action + 8, next);
        let reads = if class == 0x0109_4c2c { 0 } else { reads };
        e.mem.set_u32(action + 0xc, reads);
        action
    };
    let this = e.mem.alloc(0x14);
    e.mem.set_u32(this + 4, 0x55);
    e.mem.set_f32(this + 8, 1.5);
    if case.word(2) == 1 {
        let text = e.mem.alloc(4);
        e.mem.write(text, b"ab\0");
        e.mem.set_u32(this + 0xc, text);
    }
    let others = [e.mem.alloc(0x14), e.mem.alloc(0x14)];
    let pattern = case.word(0);
    let mut head = 0;
    for k in (0..3u32).rev() {
        let class = classes[((pattern >> (2 * k)) & 3) as usize % 3];
        let reads = [others[0], others[1], 0][k as usize];
        head = action(e, class, reads, head);
    }
    e.mem.set_u32(this + 0x10, head);
    let owners: Vec<u32> = (0..2u32)
        .map(|k| {
            let owner = e.mem.alloc(0x14);
            let mut head = 0;
            for j in 0..3u32 {
                let class = classes[((pattern + j + k) % 3) as usize];
                let reads = [this, others[1], this][j as usize];
                head = action(e, class, reads, head);
            }
            e.mem.set_u32(owner + 0x10, head);
            owner
        })
        .collect();
    let mut lists = vec![(others[0], vec![this, owners[0]]), (others[1], vec![this])];
    match case.word(1) {
        1 => lists.push((this, vec![owners[0]])),
        2 => lists.push((this, vec![owners[0], owners[1], owners[0]])),
        _ => {}
    }
    reaction_map(e, case.word(3), &lists);
    vec![this]
}

/// The cases of [`destructor_words`]: the classes of the value's three actions (two bits each),
/// the reactions on it, a string or none, the reaction map's bucket count.
const DESTRUCTOR_INPUTS: &[Input] = &[
    Input::Of(&[0, 1, 2, 5, 6, 9, 18, 21, 26, 36, 42]),
    Input::Of(&[0, 1, 2]),
    Input::Flag,
    Input::Of(&[1, 37]),
];

/// `Tile::Value::~Value` (`00a09330`) against its translation: the actions cleared (each class's
/// `QRefValue`, the two action classes' read in place), the owners' actions that read the value
/// blanked and the others kept, the reactions freed and the map's entry erased, the string freed.
#[test]
fn value_destructor_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_9330,
        dispatch::tile::value_destructor(Ptr<_>)
    ))
    .family(&dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .ignore_calls(&[0x00a0_c060, 0x00ad_87c0])
    .inputs(DESTRUCTOR_INPUTS)
    .setup(destructor_words)
    .run()
    .assert_same();
}

/// `Tile::Value::ClearActions` (`00a09200`) on the action classes' own `QRefValue`s, read in place.
#[test]
fn clear_actions_in_place_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_9200,
        dispatch::tile::clear_actions(Ptr<_>)
    ))
    .family(&dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .ignore_calls(&[0x00a0_c060, 0x00ad_87c0])
    .inputs(DESTRUCTOR_INPUTS)
    .setup(destructor_words)
    .run()
    .assert_same();
}
