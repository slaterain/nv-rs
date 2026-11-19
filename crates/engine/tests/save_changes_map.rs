//! Engine Optimizations' bucket count for a save's map of changed forms (`00845550`, the row
//! S1016.845558, `bInlinesChangesMapBuckets`): with every translation linked in, the map is made by
//! the exe's constructor with the row off (5,039 buckets) and on (10,037), filled by the exe's own
//! insert (`008458b0`) and walked as a save walks it (`004b9ba0`, `006b7f20`). The walk's order is
//! what changes; a load inserts the records in the order the save wrote them and finds each by its id,
//! so a save written with either count reads back with either.

use engine as _;
use engine_core::extend::stewie;
use engine_core::prelude::*;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The heap's allocate and free (`00401000`, `00401030`, NiAlloc's `00aa1070`, `00aa10f0`) and the
/// map's item allocator as the test memory's.
fn heap(e: &mut Engine) {
    e.register(0x0040_1000, |e, a| ret(e.mem.alloc(a[0].max(4))));
    e.register(0x0040_1030, |_, _| Ret::default());
    e.register(0x00aa_1070, |e, a| ret(e.mem.alloc(a[0].max(4))));
    e.register(0x00aa_10f0, |_, _| Ret::default());
    // The map's item allocator (its slots +0x14 and +0x18, the game's pooled list entries).
    e.register(0x0048_6a90, |e, _| ret(e.mem.alloc(12)));
    e.register(0x0066_6050, |_, _| Ret::default());
}

/// The changes map's vtable as the exe holds it at `0107ed68`.
fn vtable(e: &mut Engine) {
    e.map(0x0107_e000, 0x1000);
    let slots = [
        0x0084_5580u32,
        0x006c_6a60,
        0x006b_81d0,
        0x0086_3840,
        0x0045_34f0,
        0x0048_6a90,
        0x0066_6050,
    ];
    for (i, s) in slots.iter().enumerate() {
        e.mem.set_u32(0x0107_ed68 + 4 * i as u32, *s);
    }
}

/// A changes map made by `00845550` with the row `on` or off.
fn changes_map(e: &mut Engine, on: bool) -> u32 {
    heap(e);
    vtable(e);
    e.set_stewie(stewie::CHANGES_MAP_BUCKETS, on);
    let map = e.mem.alloc(0x10);
    e.call(0x0084_5550, &args![map]);
    map
}

/// The ids and flags in the order a save walks the map.
fn walk(e: &mut Engine, map: u32) -> Vec<(u32, u32)> {
    let cursor = e.mem.alloc(12);
    let first = e.call(0x004b_9ba0, &args![map]).u32();
    e.mem.set_u32(cursor, first);
    let mut out = vec![];
    while e.mem.u32(cursor) != 0 {
        e.call(0x006b_7f20, &args![map, cursor, cursor + 4, cursor + 8]);
        let change = e.mem.u32(cursor + 8);
        out.push((e.mem.u32(cursor + 4), e.mem.u32(change)));
    }
    out
}

/// The flags `008456e0` finds for `id` (0 for none).
fn flags_of(e: &mut Engine, map: u32, id: u32) -> u32 {
    let out = e.mem.alloc(4);
    e.call(0x0084_56e0, &args![map, out, id]);
    e.mem.u32(out)
}

/// Forms of a late playthrough: the base game's, a DLC's, created ones.
fn ids() -> Vec<u32> {
    (0..6000u32)
        .map(|i| match i % 3 {
            0 => 0x0001_0000 + i * 7,
            1 => 0x0300_0800 + i * 3,
            _ => 0xff00_0800 + i,
        })
        .collect()
}

#[test]
fn the_changes_maps_bucket_count_and_a_save_written_either_way_loads_either_way() {
    let mut saves = vec![];
    for on in [false, true] {
        let mut e = Engine::new();
        let map = changes_map(&mut e, on);
        assert_eq!(e.mem.u32(map + 4), if on { 10_037 } else { 5_039 });
        for (n, id) in ids().into_iter().enumerate() {
            e.call(0x0084_58b0, &args![map, id, (n as u32) | 0x4000_0000]);
        }
        let order = walk(&mut e, map);
        assert_eq!(order.len(), 6000);
        saves.push(order);
    }
    // The order a save writes differs; the records are the same.
    assert_ne!(saves[0], saves[1]);
    let sorted = |s: &[(u32, u32)]| {
        let mut s = s.to_vec();
        s.sort_unstable();
        s
    };
    assert_eq!(sorted(&saves[0]), sorted(&saves[1]));
    // Each save loads into a map of either count: every record is found by its id.
    for save in &saves {
        for on in [false, true] {
            let mut e = Engine::new();
            let map = changes_map(&mut e, on);
            for (id, flags) in save {
                e.call(0x0084_58b0, &args![map, *id, *flags]);
            }
            for (id, flags) in save {
                assert_eq!(flags_of(&mut e, map, *id), *flags, "{id:08x}, row {on}");
            }
        }
    }
}
