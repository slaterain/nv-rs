//! The differential test of every typed function of the multibound family (`engine_typed::multibound`)
//! against its translation. The world: the `TES` object with its loaded grid of cells (or an interior
//! cell), each cell's 3D root with its multibound nodes (child 7) and land (child 2); multibound nodes
//! with their bound and shape; the first shadow scene node with its child 6 and its portal graph
//! (occluders, held and queued objects, tracked items); the water system with its water groups; and a
//! camera. Objects have vtables whose slots log and answer the word after the slots.
//!
//! What stands in: the culling process's set-up, the terrain manager, the occluder test, the scene
//! node's candidate, the portal graph's and the queued objects' calls, the list walk. Everything else
//! runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Diff, Input};
use engine_typed::multibound::{self, getters, occlusion, pass, GridCellArray, TES};
use engine_typed::shader::layout::ShadowSceneNode;
use engine_typed::{culling, dispatch, draw, pathing};

const VT_FN: u32 = 0x00ff_2000;
const VT_SLOTS: u32 = 0x200;
const LOG: u32 = 0x0300_0000;
const MAX_CALLS: usize = 5_000;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;
const MINUS_ONE: u32 = 0xbf80_0000;

/// The water entry `NiRTTI`, the shadow scene node table, the grid setting's value.
const WATER_ENTRY_RTTI: u32 = 0x0120_2e74;
const SCENE_TABLE: u32 = 0x011f_91c8;
const GRIDS_TO_LOAD: u32 = 0x011c_63cc;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    if n >= 0x7f0 {
        return;
    }
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

/// A stand-in that logs its words under `TAG` and answers the word at `LOG + 0x3f00 + 4 * (TAG &
/// 0x3f)`, so a test can set what it answers.
fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, TAG | 0x1_0000, *w);
    }
    ret(e.mem.u32(LOG + 0x3f00 + 4 * (TAG & 0x3f)))
}

fn set_logged_answer(e: &mut Engine, tag: u32, answer: u32) {
    e.mem.set_u32(LOG + 0x3f00 + 4 * (tag & 0x3f), answer);
}

/// The stand-in for any slot: logs `this` and its words, answers the vtable's answer word.
fn any_slot(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a.first().copied().unwrap_or(0);
    log(e, 0xffff, this);
    for w in a.iter().skip(1) {
        log(e, 0xfffe, *w);
    }
    let vt = e.mem.u32(this);
    ret(e.mem.u32(vt + VT_SLOTS))
}

/// An object of `size` bytes with its own vtable of stand-in slots answering `answer`.
fn object(e: &mut Engine, size: u32, answer: u32) -> u32 {
    let vt = e.mem.alloc(VT_SLOTS + 0x40);
    for slot in (0..VT_SLOTS).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    e.mem.set_u32(vt + VT_SLOTS, answer);
    let o = e.mem.alloc(size);
    e.mem.set_u32(o, vt);
    o
}

fn set_slot(e: &mut Engine, object: u32, slot: u32, target: u32) {
    let vt = e.mem.u32(object);
    e.mem.set_u32(vt + slot, target);
}

/// The list walk `0057cbe0` (`thiscall(list, &position)`): the item's address (+8), the position moved
/// to the next (+0).
fn list_iterate(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0xcbe0, a[0]);
    let position = e.mem.u32(a[1]);
    let next = e.mem.u32(position);
    e.mem.set_u32(a[1], next);
    ret(position + 8)
}

/// The exe's data and the stand-ins.
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    // The pages of the double 0.0 the frustum test compares with (01012060), the box corners' constants
    // (011a9470: 0, 0, 0, 1.0, as in the exe), the marker, the grid setting, the scene node table and
    // the zero word.
    for page in [
        0x0101_2000,
        0x011a_9000,
        0x011c_3000,
        0x011c_6000,
        0x011f_9000,
        0x0120_2000,
    ] {
        e.map(page, 0x1000);
    }
    words(e, 0x011a_9470, &[0, 0, 0, ONE]);
    // The reference counts' interlocked imports.
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
    e.register(0x00a6_9400, logged::<0x01>);
    e.register(0x0041_fd00, logged::<0x02>);
    e.register(0x00a6_93e0, logged::<0x04>);
    e.register(0x005d_8a70, |e, a| {
        log(e, 0x05, a[0]);
        ret(a[0] + 0x2c)
    });
    e.register(0x00b5_f170, logged::<0x06>);
    e.register(0x0058_6170, logged::<0x07>);
    e.register(0x006f_d080, logged::<0x08>);
    e.register(0x006f_d130, logged::<0x09>);
    e.register(0x00c4_7150, logged::<0x0a>);
    e.register(0x0045_bc60, logged::<0x0b>);
    e.register(0x0045_bc40, logged::<0x0c>);
    e.register(0x0056_c7f0, logged::<0x0d>);
    e.register(0x0041_3f40, logged::<0x0e>);
    e.register(0x0089_1170, logged::<0x0f>);
    e.register(0x0076_b610, |e, a| {
        log(e, 0x10, a[0]);
        ret(u32::from(e.mem.u32(a[0]) == 0))
    });
    e.register(0x0057_cbe0, list_iterate);
    e.register(0x0045_c4a0, logged::<0x11>);
    e.register(0x0045_c4c0, logged::<0x12>);
    e.register(0x0045_c570, logged::<0x13>);
    e.register(0x0045_c4e0, logged::<0x14>);
    e.register(0x006a_b360, logged::<0x15>);
    e.register(0x0051_4f30, logged::<0x16>);
    e.register(0x004f_9bf0, logged::<0x17>);
    for s in (0..VT_SLOTS).step_by(4) {
        e.register(VT_FN + s, any_slot);
    }
}

/// A node (0xb0 bytes) whose children are `children` (count and size the same), its slots answering
/// `answer`.
fn node(e: &mut Engine, children: &[u32], answer: u32) -> u32 {
    let n = object(e, 0xb0, answer);
    let count = children.len() as u32;
    let base = block(e, 4 * count.max(1), children);
    e.mem.set_u32(n + 0xa0, base);
    e.mem.set_u16(n + 0xa4, count as u16);
    e.mem.set_u16(n + 0xa6, count as u16);
    e.mem.set_u16(n + 0xa8, count as u16);
    n
}

/// A multibound node: its slots answer `answer`, its word +0xa8 is `size`, its bound's shape has the
/// cull result `result` and its slots answer `inside`.
fn multibound_node(e: &mut Engine, answer: u32, size: u16, result: u32, inside: u32) -> u32 {
    let n = object(e, 0xb4, answer);
    e.mem.set_u16(n + 0xa8, size);
    let shape = object(e, 0x24, inside);
    e.mem.set_u32(shape + 8, result);
    let bound = object(e, 0x10, 0);
    e.mem.set_u32(bound + 0xc, shape);
    e.mem.set_u32(n + 0xac, bound);
    n
}

/// `count` multibound nodes from the case's pattern `k`: the answers of slot +0x104 (and the rest of
/// the node's slots), the cull results and the shapes' answers vary with the index.
fn objects(e: &mut Engine, count: u32, k: u32) -> Vec<u32> {
    (0..count)
        .map(|i| {
            let answer = (i + k) % 2;
            let result = if (i + k) % 3 == 2 { 1 } else { 0 };
            let inside = (i + 2 * k) % 2;
            let size = if (i + k) % 4 == 3 { 0 } else { 2 };
            multibound_node(e, answer, size, result, inside)
        })
        .collect()
}

/// A cell (0xd0 bytes) whose loaded data's 3D root has a multibound list (child 7) when `with_7` and
/// a land list (child 2) of `count` objects each; a cell without loaded data when `count` is 9.
fn cell(e: &mut Engine, count: u32, with_7: bool, k: u32) -> u32 {
    let c = block(e, 0xd0, &[]);
    if count == 9 {
        return c;
    }
    let list_7 = if with_7 {
        let o = objects(e, count, k);
        node(e, &o, 0)
    } else {
        0
    };
    let o = objects(e, count, k + 1);
    let list_2 = node(e, &o, 0);
    let root = node(e, &[0, 0, list_2, 0, 0, 0, 0, list_7], 0);
    let loaded = block(e, 0x64, &[root]);
    e.mem.set_u32(c + 0xc4, loaded);
    c
}

/// A list of `items` as the exe's lists hold them: a header word (the first position), positions of
/// (next, previous, item).
fn list(e: &mut Engine, items: &[u32]) -> u32 {
    let mut next = 0;
    for &item in items.iter().rev() {
        next = block(e, 0xc, &[next, 0, item]);
    }
    block(e, 0xc, &[next])
}

/// A water group (0xb0 bytes) whose list +0x24 holds `count` entries: each entry's slot +0x1d0 node has
/// a first child that is a water entry (slot 8 answers its `NiRTTI`) every other entry, a multibound
/// node either way.
fn water_group(e: &mut Engine, count: u32, k: u32) -> u32 {
    let g = block(e, 0xb0, &[]);
    let entries: Vec<u32> = (0..count)
        .map(|i| {
            let rtti = if (i + k) % 2 == 0 {
                WATER_ENTRY_RTTI
            } else {
                0x0120_2000
            };
            let first = multibound_node(e, rtti, 2, 0, (i + k) % 2);
            let holder = node(e, &[first], 0);
            let entry = object(e, 0x24, holder);
            e.mem.set_u32(entry + 0x20, 0x77 + i);
            entry
        })
        .collect();
    let mut next = 0;
    for &item in entries.iter().rev() {
        next = block(e, 0xc, &[next, 0, item]);
    }
    e.mem.set_u32(g + 0x24, next);
    g
}

/// The water system: none (0), one group (1), and a LOD group too (2).
fn water_system(e: &mut Engine, kind: u32, k: u32) -> u32 {
    if kind == 0 {
        return 0;
    }
    let s = block(e, 0xa0, &[]);
    let group = water_group(e, 2, k);
    let header = list(e, &[group]);
    let first = e.mem.u32(header);
    e.mem.set_u32(s + 0x3c, first);
    if kind == 2 {
        let lod = water_group(e, 1, k + 1);
        e.mem.set_u32(s + 0x48, lod);
    }
    s
}

/// The first shadow scene node (in the table): its child 6 as a node of `count` objects (none for 9),
/// its portal graph (none for 0) with an occluder list of an active plane, an inactive one and a null
/// entry.
fn scene(e: &mut Engine, count: u32, graph: u32, k: u32) -> u32 {
    let s = object(e, 0x1e4, 0);
    let child = if count == 9 {
        0
    } else {
        let o = objects(e, count, k);
        let n = node(e, &o, 0);
        object(e, 0x10, n)
    };
    let array = block(e, 4 * 8, &[0, 0, 0, 0, 0, 0, child, 0]);
    e.mem.set_u32(s + 0xa0, array);
    e.mem.set_u16(s + 0xa6, 8);
    if graph != 0 {
        let g = block(e, 0x78, &[]);
        let active = block(e, 0x44, &[]);
        e.mem.set_u32(active + 0x40, 1);
        let inactive = block(e, 0x44, &[]);
        let header = list(e, &[inactive, 0, active]);
        let first = e.mem.u32(header);
        e.mem.set_u32(g + 8, first);
        e.mem.set_u32(s + 0x1e0, g);
    }
    e.mem.set_u32(SCENE_TABLE, s);
    s
}

/// A camera (0x200 bytes): its rotation, position (x from the case: modes 1 and 3 are the words 1
/// and 3) and port, the near distance at +0xdc +0x10.
fn camera(e: &mut Engine, x: u32) -> u32 {
    let c = block(e, 0x200, &[]);
    words(
        e,
        c + 0x68,
        &[ONE, 0, 0, 0, ONE, 0, 0, 0, ONE, x, TWO, HALF],
    );
    e.mem.set_u32(c + 0xdc + 0x10, HALF);
    c
}

/// The `TES` object: an interior cell, or the loaded grid of `side` cells a side, a world space or none,
/// the water system.
fn tes(e: &mut Engine, interior: u32, side: u32, world: u32, water: u32) -> u32 {
    let t = block(e, 0xc4, &[]);
    e.mem.set_u32(t + 0x34, interior);
    e.mem.set_u32(t + 0x88, world);
    e.mem.set_u32(t + 0x64, water);
    let cells = block(e, 4 * (side * side).max(1), &[]);
    let grid = block(e, 0x28, &[]);
    e.mem.set_u32(grid + 0xc, side);
    e.mem.set_u32(grid + 0x10, cells);
    e.mem.set_u32(t + 8, grid);
    e.mem.set_u32(GRIDS_TO_LOAD + 4, side);
    t
}

/// The portal graph's held and queued objects, and the remove pass's tracked items and queued objects'
/// item lists, from `kind`: the held object accepting the point or not, or none with a queue of
/// objects accepting it or not.
fn graph_objects(e: &mut Engine, kind: u32) {
    let accepting = object(e, 0x110, 1);
    let refusing = object(e, 0x110, 0);
    let held = match kind {
        1 => accepting,
        2 => refusing,
        _ => 0,
    };
    set_logged_answer(e, 0x0d, held);
    let queue = if kind == 3 {
        list(e, &[refusing, 0, accepting, accepting])
    } else {
        list(e, &[])
    };
    set_logged_answer(e, 0x0f, queue);
    let item = block(e, 0x110, &[]);
    let tracked = list(e, &[item, 0, item]);
    set_logged_answer(e, 0x0e, tracked);
    let items = list(e, &[item, 0]);
    set_logged_answer(e, 0x15, items);
    set_logged_answer(e, 0x16, items);
    set_logged_answer(e, 0x17, if kind % 2 == 0 { 0 } else { item });
}

trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&multibound::MULTIBOUND)
            .family(&dispatch::DISPATCH)
            .family(&draw::DRAW)
            .family(&pathing::PATHING)
            .family(&culling::CULLING)
            .family(&leaf::LEAVES)
            .max_calls(MAX_CALLS)
    }
}

/// The world of a pass case from `w` = [the place (0 interior, 1 outdoors with a world space and its
/// terrain manager, 2 outdoors without one), the grid's side, a multibound list in the cells, the
/// objects' pattern, the camera's x, the scene node's child 6 (its object count, 9 none), the water
/// system, the portal graph (0 none) and its objects].
fn pass_world(e: &mut Engine, w: [u32; 9]) -> (u32, u32) {
    let [place, side, with_7, k, x, child_6, water, graph, held] = w;
    stand_ins(e);
    let interior = if place == 0 {
        cell(e, 3, with_7 != 0, k)
    } else {
        0
    };
    let world = if place == 1 { block(e, 0x40, &[]) } else { 0 };
    set_logged_answer(e, 0x07, if place == 1 { 0x4321 } else { 0 });
    let system = water_system(e, water, k);
    let t = tes(e, interior, side, world, system);
    let grid = e.mem.u32(t + 8);
    let cells = e.mem.u32(grid + 0x10);
    for i in 0..side * side {
        // Every fourth slot empty (a loaded grid cell always has its 3D root: the exe walks its land
        // list without a null test).
        let c = if (i + k) % 4 == 0 {
            0
        } else {
            cell(e, 1 + i % 3, with_7 != 0, k + i)
        };
        e.mem.set_u32(cells + 4 * i, c);
    }
    scene(e, child_6, graph, k);
    graph_objects(e, held);
    set_logged_answer(e, 0x0a, k % 2);
    (t, camera(e, x))
}

/// The pass inputs: the place, the side, the multibound list, the pattern, the camera's x, the child
/// 6 count, the water system, the portal graph and its objects.
fn pass_inputs() -> Vec<Input> {
    vec![
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Of(&[0, 1, 2]),
        Input::Of(&[ONE, 1, 3, 0]),
        Input::Of(&[9, 0, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Of(&[0, 1, 2, 3]),
    ]
}

#[test]
fn multibound_add() {
    Diff::new(entry!(0x0045_b070, pass::add(Ptr<_>, u32)))
        .common()
        .inputs(&pass_inputs())
        .max_cases(320)
        .setup(|e, case| {
            let w = core::array::from_fn(|i| case.word(i));
            let (t, c) = pass_world(e, w);
            vec![t, c]
        })
        .run()
        .assert_same();
}

#[test]
fn multibound_remove() {
    // The pass inputs, then whether the land and the queue are taken too.
    let mut inputs = pass_inputs();
    inputs.extend([Input::Flag, Input::Flag]);
    Diff::new(entry!(0x0045_bc80, pass::remove(Ptr<_>, u8, u8)))
        .common()
        .inputs(&inputs)
        .max_cases(320)
        .setup(|e, case| {
            let w = core::array::from_fn(|i| case.word(i));
            let (t, _) = pass_world(e, w);
            vec![t, case.word(9), case.word(10)]
        })
        .run()
        .assert_same();
}

#[test]
fn multibound_add_object() {
    // The mode word (0, 1, 2, 3); the shape's cull result; the node's word +0xa8; the shape: a stand-in
    // answering 0 or 1, or a box with `BSMultiBoundAABB::WithinFrustum` in its slot; the planes active;
    // the portal graph: none, or its occluders; what the occluder test answers.
    Diff::new(entry!(
        0x00b5_bbe0,
        occlusion::add_object(Ptr<_>, u32, u32, u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 0x3f, 0x2]),
        Input::Flag,
        Input::Flag,
        Input::F32,
    ])
    .max_cases(256)
    .setup(|e, case| {
        stand_ins(e);
        let size = case.word(2) as u16;
        let n = multibound_node(e, 0, size, case.word(1), case.word(3) % 2);
        let shape = e.mem.u32(e.mem.u32(n + 0xac) + 0xc);
        if case.word(3) == 2 {
            set_slot(e, shape, 0x9c, 0x00c3_87f0);
            // The box: its centre x from the case, half extents (1, 2, 0.5).
            words(
                e,
                shape + 0xc,
                &[case.f32(7).to_bits(), 0, 0, ONE, TWO, HALF],
            );
        }
        let s = scene(e, 9, case.word(5), 0);
        set_logged_answer(e, 0x0a, case.word(6));
        // The process: its planes (a plane through the origin facing x, and its opposite) and active
        // mask, the mode word after it.
        let p = block(e, 0xa0, &[]);
        words(
            e,
            p + 0x2c,
            &[ONE, 0, 0, MINUS_ONE, MINUS_ONE, 0, 0, MINUS_ONE],
        );
        e.mem.set_u32(p + 0x2c + 0x60, case.word(4));
        e.mem.set_u32(p + 0x90, case.word(0));
        vec![s, n, ONE, TWO, HALF, p]
    })
    .run()
    .assert_same();
}

#[test]
fn multibound_getters() {
    // The multibound of a node, its shape's result cleared.
    Diff::new(entry!(0x0066_29f0, getters::multibound_of(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            let n = multibound_node(e, 0, 2, 1, 0);
            if case.word(0) == 0 {
                e.mem.set_u32(n + 0xac, 0);
            }
            vec![n]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0052_8820, getters::clear_cull_result(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            stand_ins(e);
            let shape = block(e, 0xc, &[0, 0, case.word(0)]);
            vec![shape]
        })
        .run()
        .assert_same();
    // A cell with loaded data or not; its lists 2 and 7, or another index.
    Diff::new(entry!(0x0054_5cb0, getters::cell_3d(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            vec![cell(e, if case.word(0) == 0 { 9 } else { 2 }, true, 0)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0045_6fc0, getters::cell_child(Ptr<_>, u32) -> u32))
        .common()
        .inputs(&[Input::Flag, Input::Of(&[2, 7, 0, 8])])
        .setup(|e, case| {
            stand_ins(e);
            let c = cell(e, if case.word(0) == 0 { 9 } else { 2 }, true, 0);
            vec![c, case.word(1)]
        })
        .run()
        .assert_same();
    // The grid's slot: inside, or x or y at or past the dimension.
    Diff::new(entry!(
        0x004b_a490,
        getters::grid_cell(Ptr<GridCellArray>, u32, u32) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2, 3]), Input::Of(&[0, 1, 2, 3])])
    .setup(|e, case| {
        stand_ins(e);
        let t = tes(e, 0, 3, 0, 0);
        vec![e.mem.u32(t + 8), case.word(0), case.word(1)]
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x0043_d4d0, getters::setting_value(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            e.mem.set_u32(0x0120_2800, 0x55);
            vec![if case.word(0) == 0 { 0 } else { GRIDS_TO_LOAD }]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0045_0b80, getters::shadow_scene_node(u32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            stand_ins(e);
            words(e, SCENE_TABLE, &[0x1111, 0x2222, 0x3333]);
            vec![case.word(0)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0045_4b30, getters::portal_graph(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            vec![scene(e, 9, case.word(0), 0)]
        })
        .run()
        .assert_same();
    // The scene node's child: below the count, at it, past it.
    Diff::new(entry!(0x0045_bc00, getters::node_child(Ptr<_>, u32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 6, 7, 8, 9])])
        .setup(|e, case| {
            stand_ins(e);
            vec![scene(e, 2, 0, 0), case.word(0)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x004f_d3e0, getters::world_space(Ptr<TES>) -> u32))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            vec![tes(e, 0, 1, case.word(0) * 0x1234, 0)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0070_ec90, getters::word_at_64(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            vec![tes(e, 0, 1, 0, case.word(0) * 0x4321)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x005a_8080, getters::address_3c(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 0x1000])])
        .setup(|e, case| {
            stand_ins(e);
            vec![case.word(0)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0067_33e0, getters::word_at_48(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, case| {
            stand_ins(e);
            vec![water_system(e, case.word(0).max(1), 0)]
        })
        .run()
        .assert_same();
    // A list position or none; the list is not read.
    Diff::new(entry!(
        0x007b_52d0,
        getters::next_position(Ptr<_>, Ptr<_>) -> u32
    ))
    .common()
    .inputs(&[Input::Flag])
    .setup(|e, case| {
        stand_ins(e);
        let header = list(e, &[0x10, 0x20]);
        let position = if case.word(0) == 0 {
            0
        } else {
            e.mem.u32(header)
        };
        vec![0, position]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0063_17a0,
        getters::item_address(Ptr<_>, Ptr<_>) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&[0, 0x1000, 0xffff_fffc])])
    .setup(|e, case| {
        stand_ins(e);
        vec![0, case.word(0)]
    })
    .run()
    .assert_same();
    // An object or none; its `NiRTTI` the one asked for or another.
    Diff::new(entry!(
        0x0045_bad0,
        getters::is_exactly(u32, Ptr<_>) -> bool
    ))
    .common()
    .inputs(&[Input::Flag, Input::Flag])
    .setup(|e, case| {
        stand_ins(e);
        let o = if case.word(0) == 0 {
            0
        } else {
            object(e, 0x10, WATER_ENTRY_RTTI + case.word(1))
        };
        vec![WATER_ENTRY_RTTI, o]
    })
    .run()
    .assert_same();
    Diff::new(entry!(0x0045_baf0, getters::rtti_is(Ptr<_>, u32) -> bool))
        .common()
        .inputs(&[Input::Flag])
        .setup(|e, case| {
            stand_ins(e);
            let o = object(e, 0x10, WATER_ENTRY_RTTI);
            vec![o, WATER_ENTRY_RTTI + case.word(0)]
        })
        .run()
        .assert_same();
}

/// The world types the tests name, so a layout change shows here.
#[test]
fn multibound_layouts() {
    assert_eq!(<TES as engine_core::ptr::Layout>::SIZE, 0x8c);
    assert_eq!(TES::pWorldSpace.off, 0x88);
    assert_eq!(<GridCellArray as engine_core::ptr::Layout>::SIZE, 0x14);
    assert_eq!(ShadowSceneNode::pBSPortalGraph.off, 0x1e0);
}
