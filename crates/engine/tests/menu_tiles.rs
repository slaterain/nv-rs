//! Engine Optimizations item 7 (menus): the typed tile values and reaction map (`dispatch::tile`)
//! against their translations (`engine-bs` `bsmenu/tile.rs`), and a timing harness run by hand
//! (`--ignored --nocapture`).
//!
//! The typed sorted find and insert do the trait compare's work in place, so the translation's
//! `bsearch` (`00ec716d`) and its compares (`00a01160`) are left out of the comparison
//! (`ignore_calls`); what the search found shows in the answer and in memory. The translation's
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
    for page in [0x011f_3000, 0x011f_6000, 0x0126_f000] {
        e.map(page, 0x1000);
    }
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
        e.mem.set_i32(value, first.wrapping_add(3 * i as i32));
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
    0x7fff_ffff,
];

/// The sorted find with the trait compare: arrays of 0 to 9 values (odd and even counts, so both of
/// `bsearch`'s middles), every id found and missed; an empty array with no buffer.
#[test]
fn sorted_find_diff() {
    Diff::new(engine_core::entry!(
        0x00a0_cb90,
        dispatch::tile::sorted_find(Ptr<_>, u32, u32) -> i32
    ))
    .family(&dispatch::DISPATCH)
    .ignore_calls(&[0x00ec_716d, 0x00a0_1160])
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5, 8, 9]),
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
        let tile = tile_with(e, count, capacity, 0xfa0);
        let key = e.mem.alloc(0x14);
        e.mem.set_u32(key, case.word(1));
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, key);
        vec![tile + 0x10, slot, 0x00a0_1160]
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
    .ignore_calls(&[0x00a0_1160])
    .translation_calls_plainly(&[0x00a0_cd50])
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 7, 8, 9]),
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
        let tile = tile_with(e, count, capacity, 0xfa0);
        let value = e.mem.alloc(0x14);
        e.mem.set_u32(value, case.word(2));
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, value);
        vec![tile + 0x10, slot, 0x00a0_1160]
    })
    .run()
    .assert_same();
}

/// `GetValueQ`, `GetValue` and `GetOrCreateValue` on tiles of 0 to 9 traits, each id found and
/// missed, the search keys' first use (guard clear) and a later one.
#[test]
fn tile_get_value_diff() {
    let inputs = [
        Input::Of(&[0, 1, 2, 5, 8, 9]),
        Input::Of(ASKED),
        Input::Of(&[0, 1]),
    ];
    let setup = |e: &mut Engine, case: &engine_core::typed::diff::Case| {
        world(e);
        let count = case.word(0);
        let tile = tile_with(e, count, count + (count & 1), 0xfa0);
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
    .ignore_calls(&[0x00ec_716d, 0x00a0_1160])
    .inputs(&inputs)
    .setup(setup)
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x00a0_1000,
        dispatch::tile::get_or_create_value(Ptr<_>, i32) -> Ptr
    ))
    .family(&dispatch::DISPATCH)
    .ignore_calls(&[0x00ec_716d, 0x00a0_1160])
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
