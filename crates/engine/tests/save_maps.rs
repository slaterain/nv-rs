//! Engine Optimizations item 2 (saves): a timing harness for the save path's maps and loops, run by
//! hand (`--ignored --nocapture`), not a check. It drives the translations with every translation
//! they reach linked in; the game's leaves (form lookups, a cell's data, the heap) are cheap
//! stand-ins, so the numbers are the translated code's own cost.
//!
//! - `save_buffer_append_speed`: a save's form records appended to one `BGSSaveGameBuffer`
//!   (`00865570`, `00865ce0`): the exe grows the buffer 64 KB at a time (`008660b0`), copying all of
//!   it each time.
//! - `cleanup_expired_data_speed`: `BGSSaveLoadGame::CleanupExpiredData` (`0084ab20`, run at the
//!   start of every save, autosaves too) over a changes map of a late playthrough: its expiry
//!   queue has 37 buckets and hashes only the worldspace's id, so every exterior reference's look-up
//!   walks every expired cell of its worldspace.

use engine as _;
use engine_core::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::time::Instant;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The game heap's allocate and free (`00401000`, `00401030`) and NiAlloc's (`00aa1070`,
/// `00aa10f0`) as the test memory's.
fn heap(e: &mut Engine) {
    e.register(0x0040_1000, |e, a| ret(e.mem.alloc(a[0].max(4))));
    e.register(0x0040_1030, |_, _| Ret::default());
    e.register(0x00aa_1070, |e, a| ret(e.mem.alloc(a[0].max(4))));
    e.register(0x00aa_10f0, |_, _| Ret::default());
}

/// The save-load mode object (`011de134`; `00424940` reads its byte at +0xc: 0, not sizing only).
fn writing_mode(e: &mut Engine) {
    e.map(0x011d_e000, 0x1000);
    let mode = e.mem.alloc(16);
    e.mem.set_u32(0x011d_e134, mode);
}

static GROWS: AtomicU32 = AtomicU32::new(0);

#[test]
#[ignore]
fn save_buffer_append_speed() {
    // A late save's change records: about 6 MB in records of 3 to 120 bytes.
    const TOTAL: u32 = 6 << 20;
    let mut best = u128::MAX;
    for _ in 0..3 {
        let mut e = Engine::new();
        heap(&mut e);
        writing_mode(&mut e);
        let buffer = e.mem.alloc(0x14);
        e.call(0x0086_5bd0, &args![buffer]);
        let source = e.mem.alloc(128);
        let started = Instant::now();
        let mut written = 0u32;
        let mut i = 0u32;
        while written < TOTAL {
            let size = 3 + (i * 37) % 118;
            e.call(0x0086_5570, &args![buffer, source, size, 0u32]);
            written += size + 1;
            i += 1;
        }
        best = best.min(started.elapsed().as_micros());
        assert_eq!(e.mem.u32(buffer + 0xc), written);
        GROWS.store(e.mem.u32(buffer + 8) / 0x10000, Relaxed);
    }
    println!(
        "save buffer: {} KB appended in {} us (best of 3), {} grows of 64 KB",
        TOTAL >> 10,
        best,
        GROWS.load(Relaxed)
    );
}

/// The stand-in forms of the cleanup's world: type, a cell's detach time, grid place.
#[derive(Clone, Copy)]
struct Form {
    kind: u8,
    /// A cell: its detach time.
    detached: u32,
    x: u32,
    y: u32,
}

thread_local! {
    /// The forms by id (`LookupFormByID`'s stand-in).
    static FORMS: std::cell::RefCell<HashMap<u32, u32>> = std::cell::RefCell::new(HashMap::new());
}

const WORLD: u32 = 0x000d_a726;
const SLOT_FNS: u32 = 0x00fe_0000;

/// A form object: vtable with every slot at `SLOT_FNS + slot`, type at +4, id at +0xc, the detach
/// time at +0x20, grid x and y at +0x24 and +0x28, a place at +0x2c (floats in that grid square)
/// and its worldspace at +0x30.
fn form(e: &mut Engine, vtable: u32, id: u32, f: Form, world: u32) -> u32 {
    let o = e.mem.alloc(0x40);
    e.mem.set_u32(o, vtable);
    e.mem.set_u8(o + 4, f.kind);
    e.mem.set_u32(o + 0xc, id);
    e.mem.set_u32(o + 0x20, f.detached);
    e.mem.set_u32(o + 0x24, f.x);
    e.mem.set_u32(o + 0x28, f.y);
    let place = e.mem.alloc(12);
    e.mem.set_f32(place, (f.x as f32) * 4096.0 + 100.0);
    e.mem.set_f32(place + 4, (f.y as f32) * 4096.0 + 100.0);
    e.mem.set_u32(o + 0x2c, place);
    e.mem.set_u32(o + 0x30, world);
    o
}

/// What the cleanup's world holds.
#[derive(Clone, Copy)]
struct World {
    /// Worldspaces (cell `c` is in worldspace `c % worlds`).
    worlds: u32,
    /// Changed exterior cells, the first `expired` of them detached long ago.
    cells: u32,
    expired: u32,
    /// Changed references; the first `in_expired` in expired cells with flags 4, 0xc, 6, 0x14 in
    /// turn (each branch of the second pass), the others in cells that have not expired.
    references: u32,
    in_expired: u32,
    /// The first `buffered` cells and references are stored buffers (an unloaded form's data), not
    /// live forms.
    buffered: u32,
    /// The first `interior` of the expired cells are interior cells (no grid place: keyed by the
    /// cell), the references in them placed by their parent cell.
    interior: u32,
    /// One entry with neither a form nor a stored buffer (the exe's warning).
    missing: bool,
    /// The cases beside the main ones: cells of another worldspace on the expired cells' grid squares
    /// (unexpired, with references), an unexpired interior cell with references placed by it and
    /// stored, created (0xff) references in an expired cell, a flagged cell detached at time 0, a
    /// flagged entry with no form, a stored reference of initial data kind 0.
    extras: bool,
    /// The changes map's buckets.
    buckets: u32,
}

/// A late playthrough's changes map and game, as [`World`] says.
fn cleanup_world(e: &mut Engine, w: World) -> u32 {
    heap(e);
    e.map(0x011c_3000, 0x1000);
    let data_handler = e.mem.alloc(0x100);
    e.mem.set_u32(0x011c_3f2c, data_handler);
    // The leaves.
    e.register(0x0046_23f0, |_, _| ret(1)); // the thread flag
    e.register(0x0086_7e30, |_, _| ret(100_000)); // the calendar's stamp
    e.register(0x0052_6100, |_, _| ret(72)); // the expiry limit
    e.register(0x0058_7410, |_, a| ret((a[0] << 16) | a[1]));
    e.register(0x0040_6d90, |_, a| ret(f32::from_bits(a[0]) as i32 as u32));
    // A created form (an id from 0xff000000): the data handler's test (`00469860`, (handler, id)).
    e.register(0x0046_9860, |_, a| ret(u32::from(a[1] >= 0xff00_0000)));
    // The reference form types (`00564900`).
    e.register(0x0056_4900, |_, a| {
        ret(u32::from((0x3a..=0x3c).contains(&a[0])))
    });
    e.register(0x0048_39c0, |_, a| {
        ret(FORMS.with(|f| *f.borrow().get(&a[0]).unwrap_or(&0)))
    });
    e.register(0x0040_1170, |e, a| ret(u32::from(e.mem.u8(a[0] + 4))));
    e.register(0x0054_6af0, |e, a| ret(e.mem.u32(a[0] + 0x20))); // detach time
                                                                 // An interior cell: the byte at +0x34 (the exe tests the cell's flags).
    e.register(0x0042_5fd0, |e, a| ret(u32::from(e.mem.u8(a[0] + 0x34))));
    e.register(0x0054_ddd0, |e, a| ret(e.mem.u32(a[0] + 0x30)));
    e.register(0x0054_4c30, |e, a| ret(e.mem.u32(a[0] + 0x24)));
    e.register(0x0054_4c60, |e, a| ret(e.mem.u32(a[0] + 0x28)));
    // A reference's parent cell: the word at +0x38 (0: placed by its worldspace).
    e.register(0x008d_6f30, |e, a| ret(e.mem.u32(a[0] + 0x38)));
    e.register(0x0057_5d70, |e, a| ret(e.mem.u32(a[0] + 0x30)));
    e.register(SLOT_FNS + 0xf0, |e, a| {
        ret(u32::from(e.mem.u8(a[0] + 4) == 0x3a))
    });
    e.register(SLOT_FNS + 0x1f4, |e, a| ret(e.mem.u32(a[0] + 0x2c)));
    // The changes map's removal, the flags' clear, the reference's move, the warning, the note.
    e.register(0x0084_5a20, |_, _| Ret::default());
    e.register(0x0056_2210, |e, a| {
        let flags = e.mem.u32(a[0]);
        e.mem.set_u32(a[0], flags & !a[1]);
        Ret::default()
    });
    e.register(0x0056_1ef0, |_, _| Ret::default());
    e.register(0x0084_5cc0, |_, _| Ret::default());
    e.register(0x0084_dff0, |_, _| Ret::default());
    // The stored buffers: their type, the data (copied out), the header and value sizes, the buffer
    // rebuilt, the references map. The initial data's kind is the exe's (`0084e730`, translated).
    e.register(0x0085_3640, |e, a| ret(u32::from(e.mem.u8(a[0]))));
    e.register(0x0086_6390, |e, a| ret(e.mem.u32(a[0]) + 0x10));
    e.register(0x0084_e930, |e, a| {
        let bytes = e.mem.bytes(a[0], 0x30);
        e.mem.write(a[2], &bytes);
        Ret::default()
    });
    e.register(0x0084_e800, |_, _| ret(0x20));
    e.register(0x0084_6080, |e, a| {
        e.mem.set_u32(a[1], 4);
        ret(1)
    });
    e.register(0x0086_63e0, |_, _| ret(0x40));
    for stub in [
        0x0066_65a0u32,
        0x0086_65d0,
        0x0040_5430,
        0x0085_2a10,
        0x0085_2cd0,
    ] {
        e.register(stub, |_, _| Ret::default());
    }
    let vtable = e.mem.alloc(0x800);
    for slot in (0..0x800).step_by(4) {
        e.mem.set_u32(vtable + slot, SLOT_FNS + slot);
    }
    // The expiry queue's vtable as the exe holds it at 0107f494.
    e.map(0x0107_f000, 0x1000);
    let queue_slots = [
        0x0084_b540u32,
        0x0084_b930,
        0x0084_b950,
        0x0084_b990,
        0x0045_34f0,
        0x0084_ba50,
        0x0084_ba70,
    ];
    for (i, s) in queue_slots.iter().enumerate() {
        e.mem.set_u32(0x0107_f494 + 4 * i as u32, *s);
    }
    let world_kind = Form {
        kind: 0x41,
        detached: 0,
        x: 0,
        y: 0,
    };
    let worlds: Vec<u32> = (0..w.worlds)
        .map(|i| form(e, vtable, WORLD + i, world_kind, 0))
        .collect();
    // The changes map: a NiTPointerMap<u32, BGSFormChange*> (vtable slot 4 is the exe's
    // `006c6a60`).
    let buckets = w.buckets;
    let map_vtable = e.mem.alloc(0x20);
    e.mem.set_u32(map_vtable + 4, 0x006c_6a60);
    let map = e.mem.alloc(0x10);
    e.mem.set_u32(map, map_vtable);
    e.mem.set_u32(map + 4, buckets);
    let table = e.mem.alloc(buckets * 4);
    e.mem.set_u32(map + 8, table);
    let mut forms = HashMap::new();
    let add = |e: &mut Engine, id: u32, flags: u32| {
        let change = e.mem.alloc(8);
        e.mem.set_u32(change, flags);
        let item = e.mem.alloc(12);
        let bucket = table + (id % buckets) * 4;
        let head = e.mem.u32(bucket);
        e.mem.set_u32(item, head);
        e.mem.set_u32(item + 4, id);
        e.mem.set_u32(item + 8, change);
        e.mem.set_u32(bucket, item);
        change
    };
    // A stored (unloaded) form buffer: its type at +0, a flag byte at +1, and from +0x10 the
    // initial data the stand-in of `0084e930` copies out.
    let stored = |e: &mut Engine, change: u32, kind: u8, data: &[u32]| {
        let block = e.mem.alloc(0x60);
        e.mem.set_u8(block, kind);
        e.mem.set_u8(block + 1, 7);
        for (i, w) in data.iter().enumerate() {
            e.mem.set_u32(block + 0x10 + 4 * i as u32, *w);
        }
        e.mem.set_u32(change + 4, block);
    };
    let side = (w.cells as f64).sqrt().ceil() as u32;
    let mut cells_made = vec![];
    let world_of = |c: u32| worlds[(c % w.worlds) as usize];
    for c in 0..w.cells {
        let id = 0x0010_0000 + c;
        let detached = if c < w.expired { 1_000 } else { 99_990 };
        let f = Form {
            kind: 0x39,
            detached,
            x: c % side,
            y: c / side,
        };
        let cell = form(e, vtable, id, f, world_of(c));
        if c >= w.buffered && c < w.buffered + w.interior {
            e.mem.set_u8(cell + 0x34, 1);
        }
        cells_made.push(cell);
        // Stored cells: initial data kind 2 (0x10000000), 1 (0x20000000) or 3 (neither) in turn.
        let kind_flag = if c < w.buffered {
            [0x1000_0000, 0x2000_0000, 0][(c % 3) as usize]
        } else {
            0
        };
        let change = add(e, id, 0x4000_0000 | kind_flag);
        if c < w.buffered {
            let area = WORLD + c % w.worlds;
            stored(e, change, 0x39, &[area, f.x, f.y, detached]);
        } else {
            forms.insert(id, cell);
        }
    }
    for r in 0..w.references {
        let id = 0x0020_0000 + r;
        let (c, flags) = if r < w.in_expired {
            (r % w.expired, [4, 0xc, 6, 0x14][(r % 4) as usize])
        } else {
            (w.expired + r % (w.cells - w.expired), 4)
        };
        let f = Form {
            kind: 0x3a,
            detached: 0,
            x: c % side,
            y: c / side,
        };
        let reference = form(e, vtable, id, f, world_of(c));
        // Placed by its parent cell: always in an interior cell, every other one in an exterior cell
        // (the exe then tests the parent for being an interior).
        let interior = c >= w.buffered && c < w.buffered + w.interior;
        if r >= w.buffered && (interior || r % 2 == 1) {
            e.mem.set_u32(reference + 0x38, cells_made[c as usize]);
        }
        let change = add(e, id, flags);
        if r < w.buffered {
            let area = WORLD + c % w.worlds;
            let x = ((f.x as f32) * 4096.0 + 100.0).to_bits();
            let y = ((f.y as f32) * 4096.0 + 100.0).to_bits();
            stored(e, change, 0x3a, &[area, 0, 0, 0, 0, x, y]);
        } else {
            forms.insert(id, reference);
        }
    }
    if w.extras {
        let reference_at = |e: &mut Engine, id: u32, x: u32, y: u32, world: u32, parent: u32| {
            let f = Form {
                kind: 0x3a,
                detached: 0,
                x,
                y,
            };
            let reference = form(e, vtable, id, f, world);
            e.mem.set_u32(reference + 0x38, parent);
            reference
        };
        // Cells of the next worldspace on the expired exterior cells' grid squares, not expired,
        // each holding two references (flags 4 and 0xc, one placed by the cell): the find needs
        // both key words.
        let first_exterior = w.buffered + w.interior;
        for c in first_exterior..w.expired.min(first_exterior + 6) {
            let (x, y) = (c % side, c / side);
            let id = 0x0011_0000 + c;
            let world = worlds[((c + 1) % w.worlds) as usize];
            let f = Form {
                kind: 0x39,
                detached: 99_990,
                x,
                y,
            };
            let twin = form(e, vtable, id, f, world);
            add(e, id, 0x4000_0000);
            forms.insert(id, twin);
            for (k, flags) in [4u32, 0xc].into_iter().enumerate() {
                let rid = 0x0021_0000 + 2 * c + k as u32;
                let r = reference_at(e, rid, x, y, world, twin * (k as u32));
                add(e, rid, flags);
                forms.insert(rid, r);
            }
        }
        // An unexpired exterior cell at grid (0, 0) of the next worldspace (cell 0 is in the first).
        let origin = form(
            e,
            vtable,
            0x0011_ffff,
            Form {
                kind: 0x39,
                detached: 99_990,
                x: 0,
                y: 0,
            },
            worlds[1 % w.worlds as usize],
        );
        add(e, 0x0011_ffff, 0x4000_0000);
        forms.insert(0x0011_ffff, origin);
        let r = reference_at(e, 0x0021_ffff, 0, 0, worlds[1 % w.worlds as usize], 0);
        add(e, 0x0021_ffff, 4);
        forms.insert(0x0021_ffff, r);
        // An interior cell that has not expired, with references placed by it (flags 4 and 0xc)
        // and stored ones (their data naming it).
        let inside = form(
            e,
            vtable,
            0x0012_0000,
            Form {
                kind: 0x39,
                detached: 99_990,
                x: 0,
                y: 0,
            },
            worlds[0],
        );
        e.mem.set_u8(inside + 0x34, 1);
        add(e, 0x0012_0000, 0x4000_0000);
        forms.insert(0x0012_0000, inside);
        for (k, flags) in [4u32, 0xc].into_iter().enumerate() {
            let rid = 0x0022_0000 + k as u32;
            let r = reference_at(e, rid, 0, 0, worlds[0], inside);
            add(e, rid, flags);
            forms.insert(rid, r);
            let change = add(e, rid + 2, flags);
            stored(e, change, 0x3a, &[0x0012_0000, 0, 0, 0, 0, 0, 0]);
        }
        // Created references (0xff ids) in an expired exterior cell, live and stored, flags 4 and
        // 0xc: the exe passes over them.
        let c = first_exterior;
        let (x, y) = (c % side, c / side);
        let fx = ((x as f32) * 4096.0 + 100.0).to_bits();
        let fy = ((y as f32) * 4096.0 + 100.0).to_bits();
        for (k, flags) in [4u32, 0xc].into_iter().enumerate() {
            let rid = 0xff00_1000 + k as u32;
            let r = reference_at(e, rid, x, y, world_of(c), 0);
            add(e, rid, flags);
            forms.insert(rid, r);
            let change = add(e, rid + 2, flags);
            stored(e, change, 0x3a, &[WORLD + c % w.worlds, 0, 0, 0, 0, fx, fy]);
        }
        // A flagged cell detached at time 0, and a flagged entry whose form is not found.
        let zero = form(
            e,
            vtable,
            0x0013_0000,
            Form {
                kind: 0x39,
                detached: 0,
                x: 1,
                y: 1,
            },
            worlds[0],
        );
        add(e, 0x0013_0000, 0x4000_0000);
        forms.insert(0x0013_0000, zero);
        add(e, 0x0013_0001, 0x4000_0000);
        // A stored reference of initial data kind 0 (flags without 2, 4 or 8) in an expired cell.
        let change = add(e, 0x0023_0000, 0x10);
        stored(e, change, 0x3a, &[WORLD + c % w.worlds, 0, 0, 0, 0, fx, fy]);
    }
    for (i, world) in worlds.iter().enumerate() {
        forms.insert(WORLD + i as u32, *world);
    }
    if w.missing {
        add(e, 0x0030_0000, 4);
    }
    FORMS.with(|f| *f.borrow_mut() = forms);
    let game = e.mem.alloc(0x250);
    e.mem.set_u32(game, map);
    let history = e.mem.alloc(0x10);
    e.mem.set_u32(game + 0x1c, history);
    game
}

/// The expiry queue's finds (`0084b8b0`) and the hash and key compares they and the inserts make:
/// the typed clean-up keeps the queue as the exe builds it (construction, inserts, flush,
/// destruction, its heap blocks) and answers the finds from a native map beside it.
const QUEUE_CALLS: &[u32] = &[0x0084_b8b0, 0x0084_b930, 0x0084_b950];

/// Every entry of the changes map after the clean-up: key and flags, in the map's order.
fn changes_map(e: &Engine, game: u32) -> Vec<(u32, u32)> {
    let map = e.mem.u32(game);
    let (buckets, table) = (e.mem.u32(map + 4), e.mem.u32(map + 8));
    let mut out = vec![];
    for b in 0..buckets {
        let mut item = e.mem.u32(table + 4 * b);
        while item != 0 {
            let change = e.mem.u32(item + 8);
            out.push((e.mem.u32(item + 4), e.mem.u32(change)));
            item = e.mem.u32(item);
        }
    }
    out
}

/// One run of `0084ab20` on `w`: the calls it made into the exe (the queue's own left out), and
/// the changes map it left.
/// Also the blocks given to the heap's free (`00401030`), which the exe's queue uses for its items
/// too.
type Run = (Vec<(u32, Vec<u32>)>, Vec<(u32, u32)>, usize, Vec<u32>);

fn cleanup_run(mut e: Engine, w: World) -> Run {
    let game = cleanup_world(&mut e, w);
    e.call_log = Some(vec![]);
    e.call(0x0084_ab20, &args![game]);
    let log = e.call_log.take().unwrap();
    let queue = log.iter().filter(|(a, _)| *a == 0x0084_b8b0).count();
    let frees = log
        .iter()
        .filter(|(a, _)| *a == 0x0040_1030)
        .map(|(_, w)| w[0])
        .collect();
    let log = log
        .into_iter()
        .filter(|(a, _)| !QUEUE_CALLS.contains(a))
        .collect();
    (log, changes_map(&e, game), queue, frees)
}

/// The typed clean-up (`dispatch` family, its expiry queue a native hash map) against the exe's
/// translation: three worldspaces (each one chain of the exe's queue, and across its buckets), 150
/// expired cells, references in them taking every branch, references elsewhere and a missing form:
/// the same calls with the same words in the same order, and the same changes map left.
#[test]
fn typed_cleanup_expired_data_matches_the_translation() {
    let w = World {
        worlds: 3,
        cells: 400,
        expired: 150,
        references: 3000,
        in_expired: 400,
        buffered: 40,
        interior: 12,
        missing: true,
        extras: true,
        buckets: 5039,
    };
    let (exe_log, exe_map, exe_queue, exe_frees) = cleanup_run(Engine::new_untyped_for_test(), w);
    let (typed_log, typed_map, typed_queue, typed_frees) = cleanup_run(Engine::new(), w);
    assert!(
        exe_queue >= 3000,
        "the exe asked its queue {exe_queue} times"
    );
    assert_eq!(
        typed_queue, 0,
        "the typed clean-up asks its own map, not the queue"
    );
    for (i, (t, x)) in typed_log.iter().zip(&exe_log).enumerate() {
        assert_eq!(
            t,
            x,
            "call {i}: exe {:x?}",
            &exe_log[i.saturating_sub(3)..(i + 3).min(exe_log.len())]
        );
    }
    assert_eq!(typed_log.len(), exe_log.len());
    assert_eq!(typed_map, exe_map);
    // The old stored buffers the partial removals free, in the exe's order among its frees.
    assert_eq!(typed_frees, exe_frees);
    assert!(
        exe_frees.len() > 20,
        "the queue's items and the 20 old buffers"
    );
    // Each branch ran, counted on the translation's run (the typed run made the same calls).
    let count = |log: &[(u32, Vec<u32>)], a: u32| log.iter().filter(|(x, _)| *x == a).count();
    let note: Vec<Vec<u32>> = exe_log
        .iter()
        .filter(|(a, _)| *a == 0x0084_dff0)
        .map(|(_, w)| w[2..].to_vec())
        .collect();
    assert_eq!(
        note,
        [vec![272, 90, 14]],
        "the note: stripped, removed whole, rebuilt"
    );
    assert_eq!(
        count(&exe_log, 0x0084_5a20),
        90,
        "flags exactly 4: the whole entry"
    );
    assert_eq!(
        count(&exe_log, 0x0056_2210),
        182,
        "the other stripped entries: bits 0xc cleared"
    );
    assert_eq!(count(&exe_log, 0x0086_65d0), 14, "stored buffers rebuilt");
    assert_eq!(
        count(&exe_log, 0x0040_5430),
        7,
        "kind 6: the references map's entry"
    );
    assert_eq!(
        count(&exe_log, 0x0085_2cd0),
        7,
        "kind 6 in an exterior cell"
    );
    assert_eq!(count(&exe_log, 0x0056_1ef0), 251, "live references moved");
    assert_eq!(
        count(&exe_log, 0x0084_b7c0),
        150,
        "one insert per expired cell"
    );
    // 0x0013_0000 (detached at 0) and 0x0013_0001 (no form) ask no expiry limit.
    assert_eq!(
        count(&exe_log, 0x0052_6100),
        408,
        "the cells with a detach time"
    );
    assert_eq!(
        count(&exe_log, 0x0084_5cc0),
        2,
        "the entries with no form: warnings"
    );
    // The created references keep their changes and are not moved.
    let objects: Vec<u32> =
        FORMS.with(|f| (0..2).map(|k| f.borrow()[&(0xff00_1000 + k)]).collect());
    for (id, flags) in [
        (0xff00_1000u32, 4u32),
        (0xff00_1001, 0xc),
        (0xff00_1002, 4),
        (0xff00_1003, 0xc),
    ] {
        assert!(exe_map.contains(&(id, flags)), "{id:08x} kept");
    }
    assert!(!exe_log
        .iter()
        .any(|(a, w)| *a == 0x0056_1ef0 && objects.contains(&w[0])));
}

#[test]
#[ignore]
fn cleanup_expired_data_speed() {
    // A late playthrough: 3,000 changed exterior cells of one worldspace (1, 200, 600 or 1,500 of
    // them expired) and 40,000 changed references in the others; the changes map's 5,039 buckets.
    for expired in [1u32, 200, 600, 1500] {
        let w = World {
            worlds: 1,
            cells: 3000,
            expired,
            references: 40_000,
            in_expired: 0,
            buffered: 0,
            interior: 0,
            missing: false,
            extras: false,
            buckets: 5039,
        };
        for (name, typed) in [("translation", false), ("typed", true)] {
            let mut best = u128::MAX;
            for _ in 0..3 {
                let mut e = if typed {
                    Engine::new()
                } else {
                    Engine::new_untyped_for_test()
                };
                let game = cleanup_world(&mut e, w);
                let started = Instant::now();
                e.call(0x0084_ab20, &args![game]);
                best = best.min(started.elapsed().as_micros());
            }
            println!(
                "CleanupExpiredData {name}: 3000 cells ({expired} expired), 40000 references: {best} us (best of 3)"
            );
        }
    }
}

/// The differential harness's run of the typed clean-up against its translation (the queue's own
/// calls left out, with what they do): two small worlds (the large one is the test above's; this harness hashes memory at every call).
#[test]
fn cleanup_expired_data_diff() {
    use engine_core::typed::diff::{Diff, Input};
    use engine_core::typed::leaf;
    use engine_typed::dispatch;
    let small = World {
        worlds: 2,
        cells: 16,
        expired: 8,
        references: 60,
        in_expired: 24,
        buffered: 4,
        interior: 2,
        missing: true,
        extras: true,
        buckets: 37,
    };
    let larger = World {
        worlds: 3,
        cells: 30,
        expired: 12,
        references: 120,
        in_expired: 48,
        buffered: 8,
        interior: 3,
        missing: false,
        extras: true,
        buckets: 53,
    };
    Diff::new(engine_core::entry!(
        0x0084_ab20,
        dispatch::saveload::cleanup_expired_data(Ptr<_>)
    ))
    .family(&dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    // The expiry queue: its construction, inserts, finds, flush and destruction.
    .ignore_calls(&[0x0084_b8b0])
    .inputs(&[Input::Of(&[0, 1])])
    .max_calls(400_000)
    .setup(move |e, case| {
        vec![cleanup_world(
            e,
            if case.word(0) == 0 { small } else { larger },
        )]
    })
    .run()
    .assert_same();
}

/// A queue as the clean-up builds it (the exe's base constructor's layout: vtable, bucket count,
/// bucket array, item count) of `size` buckets, holding six items keyed (0x1000 + i % 3, i), value
/// 50 + i; with `own_slots` its vtable's hash and key compare are other functions (stand-ins that
/// answer as the exe's do), so the insert asks them as the exe asks.
fn queue_world(e: &mut Engine, size: u32, own_slots: bool) -> u32 {
    heap(e);
    let vtable = e.mem.alloc(0x20);
    let slots = [
        0x0084_b540u32,
        if own_slots { 0x00fd_0004 } else { 0x0084_b930 },
        if own_slots { 0x00fd_0008 } else { 0x0084_b950 },
        0x0084_b990,
        0x0045_34f0,
        0x0084_ba50,
        0x0084_ba70,
    ];
    for (i, s) in slots.iter().enumerate() {
        e.mem.set_u32(vtable + 4 * i as u32, *s);
    }
    e.register(0x00fd_0004, |e, a| ret(a[1] % e.mem.u32(a[0] + 4)));
    e.register(0x00fd_0008, |_, a| {
        ret(u32::from(a[1] == a[3] && a[2] == a[4]))
    });
    let queue = e.mem.alloc(0x10);
    let table = e.mem.alloc(size * 4);
    e.mem.set_u32(queue, vtable);
    e.mem.set_u32(queue + 4, size);
    e.mem.set_u32(queue + 8, table);
    for i in 0..6u32 {
        let (low, high) = (0x1000 + i % 3, i);
        let item = e.mem.alloc(0x18);
        let bucket = table + (low % size) * 4;
        let head = e.mem.u32(bucket);
        e.mem.set_u32(item, head);
        e.mem.set_u32(item + 8, low);
        e.mem.set_u32(item + 0xc, high);
        e.mem.set_u32(item + 0x10, 50 + i);
        e.mem.set_u32(bucket, item);
    }
    e.mem.set_u32(queue + 0xc, 6);
    queue
}

/// The queue's typed insert, hash and key compare against their translations: keys already there
/// (their value replaced) and new ones (an item made, filled, put at its bucket's head, counted), one
/// bucket for all and several, the exe's slots and other ones.
#[test]
fn expiry_queue_functions_diff() {
    use engine_core::typed::diff::{Diff, Input};
    use engine_core::typed::leaf;
    use engine_typed::dispatch;
    Diff::new(engine_core::entry!(
        0x0084_b7c0,
        dispatch::saveload::queue_insert(Ptr<_>, u32, u32, u32)
    ))
    .family(&dispatch::DISPATCH)
    .family(&leaf::LEAVES)
    .inputs(&[
        Input::Of(&[1, 3, 37]),
        Input::Of(&[0, 1]),
        Input::Of(&[0x1000, 0x1001, 0x2000]),
        Input::Of(&[0, 1, 4, 99]),
    ])
    .setup(|e, case| {
        let queue = queue_world(e, case.word(0), case.word(1) == 1);
        vec![queue, case.word(2), case.word(3), 0x77]
    })
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x0084_b930,
        dispatch::saveload::queue_hash(Ptr<_>, u32, u32) -> u32
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&[
        Input::Of(&[1, 37, 4999]),
        Input::Of(&[0, 36, 37, 0xda726, u32::MAX]),
        Input::Of(&[0, 7]),
    ])
    .setup(|e, case| {
        let queue = e.mem.alloc(0x10);
        e.mem.set_u32(queue + 4, case.word(0));
        vec![queue, case.word(1), case.word(2)]
    })
    .run()
    .assert_same();
    Diff::new(engine_core::entry!(
        0x0084_b950,
        dispatch::saveload::queue_keys_equal(Ptr<_>, u32, u32, u32, u32) -> bool
    ))
    .family(&dispatch::DISPATCH)
    .inputs(&[
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1]),
    ])
    .setup(|e, case| {
        let queue = e.mem.alloc(0x10);
        vec![
            queue,
            case.word(0),
            case.word(1),
            case.word(2),
            case.word(3),
        ]
    })
    .run()
    .assert_same();
}
