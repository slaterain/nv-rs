//! The compact culling copy (`engine_typed::culling::compact`, `NV_CULL_COMPACT`) against the real
//! typed walk: the same tree walked both ways from the same state must make the same calls (each
//! geometry registered with the accumulator, each object of a class the copy does not cover handed
//! its `OnVisible`, the fade nodes' alpha calls) in the same order, and leave the fade nodes' bytes
//! the same. Then the copy kept up to date: objects culled by the application, bounds moved, a child
//! slot changed and a geometry flagged always visible after the copy was built are all seen by the
//! next walk (the store track). And the check mode finds nothing to report on a correct copy.
//!
//! The tree, in the style of the parallel walk's test (`par_render.rs`): a `BSCullingProcess` with
//! the typed `Process` and `AppendVirtual` in its slots and six planes (the box `|x|, |y|, |z| <= 10`),
//! nodes, geometries inside, on and outside the planes, empty slots, objects whose `OnVisible` is a
//! stand-in (not covered by the copy: walked by the real code), and fade nodes (some group their
//! alphas, some step their fade in).

use engine as _;
use engine_core::prelude::*;
use engine_typed::culling::compact::{self, Mode};
use std::sync::Mutex;

const VT_FN: u32 = 0x00ff_2000;
const MOST_ALPHA_FN: u32 = 0x00ff_3000;
const PROCESS: u32 = 0x00c4_ee90;
const APPEND_VIRTUAL: u32 = 0x00c4_f1d0;
const NODE_ON_VISIBLE: u32 = 0x00a5_dbe0;
const GEOMETRY_ON_VISIBLE: u32 = 0x00a7_fd90;
const FADE_NODE_ON_VISIBLE: u32 = 0x00b4_e490;
const CULL: u32 = 0x00a5_9e00;
const START_GROUPING_ALPHAS: u32 = 0x00b6_3500;
const STOP_GROUPING_ALPHAS: u32 = 0x00b6_3090;
const SET_FADE_ALPHA: u32 = 0x00b6_bb30;
const ACCUMULATOR_CHECK: u32 = 0x0043_b300;
const SCENE_GLOBAL: u32 = 0x011f_4748;
const INCREMENT: u32 = 0x00fd_f1c0;
const DECREMENT: u32 = 0x00fd_f1c4;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;
const MINUS_TEN: u32 = 0xc120_0000;

#[derive(Clone, PartialEq, Eq, Debug)]
enum Rec {
    Register(u32),
    Other(u32, u32),
    StartGrouping(u32),
    StopGrouping(u32),
    FadeAlpha(u32, u32),
}

static RECORD: Mutex<Vec<Rec>> = Mutex::new(Vec::new());
static GROUPING: Mutex<Vec<u32>> = Mutex::new(Vec::new());
/// The tests share the record and the switch: one at a time.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn record(r: Rec) {
    RECORD.lock().unwrap().push(r);
}

fn stand_ins(e: &mut Engine) {
    e.register(VT_FN + 0xd4, |_, a| {
        record(Rec::Other(a[0], a[1]));
        ret(0)
    });
    e.register(VT_FN + 0x98, |e, a| {
        record(Rec::Register(a[1]));
        ret(e.mem.u32(a[0] + 0x10))
    });
    e.register(START_GROUPING_ALPHAS, |_, a| {
        record(Rec::StartGrouping(a[1]));
        ret(0)
    });
    e.register(STOP_GROUPING_ALPHAS, |_, a| {
        record(Rec::StopGrouping(a[0]));
        ret(0)
    });
    e.register(SET_FADE_ALPHA, |_, a| {
        record(Rec::FadeAlpha(a[0], a[1]));
        ret(0)
    });
    e.register(ACCUMULATOR_CHECK, |_, _| ret(1));
    e.register(INCREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(DECREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(MOST_ALPHA_FN, |_, a| Ret {
        st0: if GROUPING.lock().unwrap().contains(&a[0]) {
            0.5
        } else {
            1.0
        },
        ..Ret::default()
    });
    for (addr, len) in [
        (0x011a_d000, 0x1000),
        (0x011f_4000, 0x3000),
        (0x011f_6000, 0x1000),
        (0x011f_9000, 0x1000),
        (0x0101_d000, 0x1000),
        (0x0101_7000, 0x1000),
    ] {
        e.map(addr, len);
    }
    e.mem.set_u8(0x011a_d7b4, 1);
    for i in 0..11u32 {
        e.mem.set_u32(0x011a_d7b8 + 4 * i, ONE);
    }
    e.mem.set_u32(0x011a_d7e4, TWO);
    e.mem.set_u32(0x011a_d7e8, TWO);
    e.mem.set_u32(0x011a_d7ec, 0.7f32.to_bits());
    e.mem.set_u32(0x011a_d7f0, 0.3f32.to_bits());
    e.mem.set_u32(0x011a_d7f4, ONE);
    e.mem.set_u32(0x011a_d7f8, 0.1f32.to_bits());
    e.mem.set_u32(0x011f_63a0, HALF);
    e.mem.set_u32(0x011f_9138, 100);
    e.mem.set_u32(0x0101_df44, 0.2f32.to_bits());
    e.mem.set_u32(0x0101_786c, 600.0f32.to_bits());
    e.mem.set_u32(0x011f_9134, MOST_ALPHA_FN);
}

fn vtable(e: &mut Engine, real: &[(u32, u32)]) -> u32 {
    let vt = e.mem.alloc(0x100);
    for slot in (0..0x100).step_by(4) {
        let target = real
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or(VT_FN + slot, |(_, a)| *a);
        e.mem.set_u32(vt + slot, target);
    }
    vt
}

fn bound(e: &mut Engine, x: f32, y: f32, z: f32, r: f32) -> u32 {
    let b = e.mem.alloc(0x10);
    for (i, v) in [x, y, z, r].iter().enumerate() {
        e.mem.set_u32(b + 4 * i as u32, v.to_bits());
    }
    b
}

/// The place of a small bound of kind `k`: inside, on a plane, outside.
fn place(k: usize) -> (f32, f32, f32) {
    [
        (0.0, 0.0, 0.0),
        (9.5, 0.0, 0.0),
        (-9.5, 1.0, 0.0),
        (0.0, 9.5, 0.0),
        (1.0, 0.0, -9.5),
        (30.0, 0.0, 0.0),
    ][k % 6]
}

fn object(e: &mut Engine, size: u32, vt: u32, bound: u32, flags: u32) -> u32 {
    let o = e.mem.alloc(size);
    e.mem.set_u32(o, vt);
    e.mem.set_u32(o + 4, 1);
    e.mem.set_u32(o + 0x20, bound);
    e.mem.set_u32(o + 0x30, flags);
    o
}

struct World {
    root: u32,
    process: u32,
    geometries: Vec<u32>,
    nodes: Vec<u32>,
    /// The fade nodes with their bytes as built (put back before each walk).
    fades: Vec<(u32, Vec<u8>)>,
}

fn world(e: &mut Engine) -> World {
    stand_ins(e);
    let stand_in_vt = vtable(e, &[]);
    let node_vt = vtable(e, &[(0xd4, NODE_ON_VISIBLE)]);
    let geometry_vt = vtable(e, &[(0xd4, GEOMETRY_ON_VISIBLE)]);
    let fade_vt = vtable(e, &[(0xd4, FADE_NODE_ON_VISIBLE)]);
    let process_vt = vtable(e, &[(0x44, PROCESS), (0x4c, APPEND_VIRTUAL)]);
    let accumulator = e.mem.alloc(0x20);
    e.mem.set_u32(accumulator, stand_in_vt);
    e.mem.set_u32(accumulator + 4, 1);
    e.mem.set_u32(accumulator + 0x10, 1);
    let scene = e.mem.alloc(0x10);
    e.mem.set_u32(scene + 8, accumulator);
    e.mem.set_u32(SCENE_GLOBAL, scene);
    let camera = object(e, 0x120, stand_in_vt, 0, 0);
    e.mem.set_u32(camera + 0x110, ONE);
    let process = e.mem.alloc(0xc8);
    e.mem.set_u32(process, process_vt);
    e.mem.set_u32(process + 0xc, camera);
    let normals: [[f32; 3]; 6] = [
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    for (i, n) in normals.iter().enumerate() {
        let p = process + 0x2c + 0x10 * i as u32;
        for (j, v) in n.iter().enumerate() {
            e.mem.set_u32(p + 4 * j as u32, v.to_bits());
        }
        e.mem.set_u32(p + 0xc, MINUS_TEN);
    }
    e.mem.set_u32(process + 0x8c, 0x3f);
    e.mem.set_u32(process + 0xc4, accumulator);

    let mut w = World {
        root: 0,
        process,
        geometries: Vec::new(),
        nodes: Vec::new(),
        fades: Vec::new(),
    };
    let node = |e: &mut Engine, vt: u32, children: &[u32], w: &mut World| {
        let b = bound(e, 0.0, 0.0, 0.0, 12.0);
        let size = if vt == fade_vt { 0xe4 } else { 0xac };
        let n = object(e, size, vt, b, 0);
        let count = children.len() as u32;
        let slots = e.mem.alloc(4 * count.max(1));
        for (i, &c) in children.iter().enumerate() {
            e.mem.set_u32(slots + 4 * i as u32, c);
        }
        e.mem.set_u32(n + 0xa0, slots);
        e.mem.set_u16(n + 0xa4, count as u16);
        e.mem.set_u16(n + 0xa6, count as u16);
        w.nodes.push(n);
        n
    };
    let mut groups = Vec::new();
    for g in 0..6usize {
        let mut kids = Vec::new();
        for k in 0..7usize {
            let (x, y, z) = place(g + k);
            let b = bound(e, x, y, z, 1.0);
            let o = match (g * 7 + k) % 9 {
                4 => 0,
                7 => object(e, 0xc8, stand_in_vt, b, 0),
                8 => object(e, 0xc8, geometry_vt, b, 1),
                _ => {
                    let geometry = object(e, 0xc8, geometry_vt, b, 0);
                    w.geometries.push(geometry);
                    geometry
                }
            };
            kids.push(o);
        }
        let n = if g % 3 == 2 {
            let f = node(e, fade_vt, &kids, &mut w);
            e.mem.set_u32(f + 0xac, 1000.0f32.to_bits());
            e.mem.set_u32(f + 0xb0, 2000.0f32.to_bits());
            e.mem.set_u32(f + 0xbc, 10.0f32.to_bits());
            e.mem.set_u32(f + 0xc8, 100);
            if g == 2 {
                e.mem.set_u32(f + 0x30, 0x4000);
                e.mem.set_u32(f + 0xb8, ONE);
                e.mem.set_u32(f + 0xb4, ONE);
                GROUPING.lock().unwrap().push(f);
            }
            let bytes = e.mem.bytes(f, 0xe4);
            w.fades.push((f, bytes));
            f
        } else {
            node(e, node_vt, &kids, &mut w)
        };
        groups.push(n);
    }
    w.root = node(e, node_vt, &groups, &mut w);
    w
}

/// One walk of the scene: the real one (`NiAVObject::Cull`) or through the copy; the record taken.
fn walk(e: &mut Engine, w: &World, mode: Mode) -> (Vec<Rec>, Vec<Vec<u8>>) {
    for (at, bytes) in &w.fades {
        e.mem.write(*at, bytes);
    }
    RECORD.lock().unwrap().clear();
    compact::set_mode_for_tests(mode);
    let before = e.mem.bytes(w.process, 0xc8);
    if mode == Mode::Off {
        e.call(CULL, &args![w.root, w.process]);
    } else {
        assert!(
            compact::walk_scene(e, w.root, w.process),
            "the copy covers this walk"
        );
    }
    assert_eq!(
        e.mem.bytes(w.process, 0xc8),
        before,
        "the process as it was"
    );
    let fades = w.fades.iter().map(|(f, _)| e.mem.bytes(*f, 0xe4)).collect();
    (std::mem::take(&mut *RECORD.lock().unwrap()), fades)
}

#[test]
fn the_copy_walks_as_the_real_walk_and_keeps_up_with_changes() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let mut e = Engine::new();
    let w = world(&mut e);
    let real = walk(&mut e, &w, Mode::Off);
    assert!(
        real.0.iter().any(|r| matches!(r, Rec::Register(_))),
        "the tree registers geometries"
    );
    // Built on the first walk, then from the copy again with nothing changed.
    assert_eq!(walk(&mut e, &w, Mode::On), real, "first walk (copy built)");
    compact::set_mode_for_tests(Mode::On);
    assert!(compact::walk_scene(&mut e, w.root, w.process));
    for (at, bytes) in &w.fades {
        e.mem.write(*at, bytes);
    }
    RECORD.lock().unwrap().clear();
    assert!(compact::walk_scene(&mut e, w.root, w.process));
    assert_eq!(
        std::mem::take(&mut *RECORD.lock().unwrap()),
        real.0,
        "second walk (copy kept)"
    );

    // Changes after the copy was built, written as the game writes them (through the memory).
    let g = &w.geometries;
    e.mem.set_u32(g[0] + 0x30, 1); // culled by the application
    let moved = e.mem.u32(g[1] + 0x20);
    e.mem.set_u32(moved, 30.0f32.to_bits()); // its bound moved out of the box
    e.mem.set_u32(g[2] + 0x30, 0x800); // always visible
    let slots = e.mem.u32(w.nodes[0] + 0xa0);
    e.mem.set_u32(slots + 4, g[3]); // a child slot points elsewhere (the tree changed)
                                    // Keep the copy from before (no rebuild by switching): walk on, then compare with the real walk.
    for (at, bytes) in &w.fades {
        e.mem.write(*at, bytes);
    }
    RECORD.lock().unwrap().clear();
    assert!(compact::walk_scene(&mut e, w.root, w.process));
    let copy_after = std::mem::take(&mut *RECORD.lock().unwrap());
    let real_after = walk(&mut e, &w, Mode::Off);
    assert_eq!(copy_after, real_after.0, "after the changes");
    assert_ne!(real_after.0, real.0, "the changes change the walk");
}

#[test]
fn the_check_mode_finds_nothing_on_a_kept_copy() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let mut e = Engine::new();
    let w = world(&mut e);
    let real = walk(&mut e, &w, Mode::Off);
    let before = compact::check_counts();
    let checked = walk(&mut e, &w, Mode::Check);
    assert_eq!(checked, real, "the check mode walks the real objects");
    // A change, then another checked walk with the copy kept (refreshed, not built again).
    e.mem.set_u32(w.geometries[0] + 0x30, 1);
    for (at, bytes) in &w.fades {
        e.mem.write(*at, bytes);
    }
    assert!(compact::walk_scene(&mut e, w.root, w.process));
    let after = compact::check_counts();
    assert_eq!(after.walks - before.walks, 2);
    assert_eq!(after.stale, before.stale, "no stale item");
    assert_eq!(
        after.extra, before.extra,
        "nothing the copy saw that the real walk did not"
    );
    assert_eq!(
        after.missing, before.missing,
        "nothing the real walk saw that the copy did not"
    );
    assert_eq!(after.undecided, before.undecided);
}

/// A child detached from its parent and freed after the copy was built, on a page below its
/// parent's children array: the next walk must not read it (its first word now points at nothing
/// mapped, as a freed block's can), and walks as the real walk, which never visits it.
#[test]
fn a_child_detached_and_freed_since_the_copy_is_never_read() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let mut e = Engine::new();
    let w = world(&mut e);
    let geometry_vt = e.mem.u32(w.geometries[0]);
    let node_vt = e.mem.u32(w.root);
    let b = bound(&mut e, 0.0, 0.0, 0.0, 1.0);
    let child = object(&mut e, 0xc8, geometry_vt, b, 0);
    let _gap = e.mem.alloc(0x2000);
    let nb = bound(&mut e, 0.0, 0.0, 0.0, 12.0);
    let parent = object(&mut e, 0xac, node_vt, nb, 0);
    let slots = e.mem.alloc(4);
    e.mem.set_u32(slots, child);
    e.mem.set_u32(parent + 0xa0, slots);
    e.mem.set_u16(parent + 0xa4, 1);
    e.mem.set_u16(parent + 0xa6, 1);
    assert!(
        child >> 12 < slots >> 12,
        "the child below its parent's slots"
    );
    let root_slots = e.mem.u32(w.root + 0xa0);
    e.mem.set_u32(root_slots, parent);
    compact::set_mode_for_tests(Mode::On);
    for (at, bytes) in &w.fades {
        e.mem.write(*at, bytes);
    }
    RECORD.lock().unwrap().clear();
    assert!(compact::walk_scene(&mut e, w.root, w.process));
    assert!(
        RECORD.lock().unwrap().contains(&Rec::Register(child)),
        "the copy was built with the child"
    );
    // Detached, then freed: its slot emptied, its first word pointing at nothing mapped.
    e.mem.set_u32(slots, 0);
    e.mem.set_u32(child, 0x10);
    for (at, bytes) in &w.fades {
        e.mem.write(*at, bytes);
    }
    RECORD.lock().unwrap().clear();
    assert!(compact::walk_scene(&mut e, w.root, w.process));
    let copy = std::mem::take(&mut *RECORD.lock().unwrap());
    let real = walk(&mut e, &w, Mode::Off);
    assert_eq!(copy, real.0);
}
