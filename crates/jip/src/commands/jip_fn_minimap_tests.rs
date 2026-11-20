//! `InitMiniMap` and its helpers on a stand-in HUD, every expectation from JIP's
//! `Cmd_InitMiniMap_Execute` and the helpers it calls (functions_jip/jip_fn_minimap.h,
//! nvse/GameForms.cpp, nvse/GameObjects.cpp, internal/netimmerse.cpp). The game functions are
//! stand-ins at their exe addresses and record the words they get.

use super::*;
use crate::commands::test_support::{
    calls, calls_to, engine, extra_stand_ins, game_globals, loaded_mod, put_extra, CALLS, FORMS,
};
use crate::test_support::{call, DATA, SCRIPT};
use std::cell::Cell;

const AREA: u32 = 0x0600_0000;
const SCRIPT_DATA: u32 = DATA;
const QUEST: u32 = FORMS + 0x200;
const MOD_INFOS: u32 = FORMS + 0x3_0000;
/// Stand-in addresses of the test's own (vtable slots it writes).
const ADD_OBJECT_FN: u32 = 0x0dea_7000;
const NODE_VTBL: u32 = AREA + 0xf_0000;
const SHAPE_VTBL: u32 = AREA + 0xf_0100;

thread_local! {
    static NEXT: Cell<u32> = const { Cell::new(AREA + 0x1000) };
}

fn alloc(size: u32) -> u32 {
    NEXT.with(|n| {
        let at = n.get();
        n.set(at + size.next_multiple_of(0x10));
        at
    })
}

fn record(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// A new tile under `parent` (appended to its children), named `name` when given.
fn new_tile(e: &mut Engine, parent: u32, name: Option<&[u8]>) -> u32 {
    let tile = alloc(0x50);
    e.mem.set_u32(tile + 0x28, parent);
    if let Some(name) = name {
        let text = alloc(name.len() as u32 + 1);
        e.mem.write(text, name);
        e.mem.set_u32(tile + 0x20, text);
    }
    if parent != 0 {
        let node = alloc(12);
        let tail = e.mem.u32(parent + 8);
        e.mem.set_u32(node + 4, tail);
        e.mem.set_u32(node + 8, tile);
        if tail != 0 {
            e.mem.set_u32(tail, node);
        } else {
            e.mem.set_u32(parent + 4, node);
        }
        e.mem.set_u32(parent + 8, node);
    }
    tile
}

/// The tile's values of the trait ids given (sorted, as the game keeps them), each with its
/// number; the values' addresses in the order given.
fn values(e: &mut Engine, tile: u32, list: &[(u32, f32)]) -> Vec<u32> {
    let made: Vec<(u32, u32)> = list
        .iter()
        .map(|&(id, num)| {
            let v = alloc(0x14);
            e.mem.set_u32(v, id);
            e.mem.set_f32(v + 8, num);
            (id, v)
        })
        .collect();
    let mut sorted = made.clone();
    sorted.sort();
    let data = alloc(4 * sorted.len() as u32);
    for (i, (_, v)) in sorted.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, *v);
    }
    e.mem.set_u32(tile + 0x14, data);
    e.mem.set_u32(tile + 0x18, sorted.len() as u32);
    made.into_iter().map(|(_, v)| v).collect()
}

/// A NiNode-like object of the test (its vtable answers `IS_NODE` and has `AddObject`).
fn ni_node(e: &mut Engine) -> u32 {
    let n = alloc(0xc0);
    e.mem.set_u32(n, NODE_VTBL);
    n
}

/// The HUD as JIP MiniMap's XML makes it, as `InitMiniMap` walks it: `JIPMiniMap`'s children end
/// with the frame, the player marker, then one more; the frame's with the world map rect, the
/// local map, then one more; the local map's with the nine local tiles (the last first), then the
/// door and local
/// quest marker rects; the world map rect's with the world quest markers, the map markers, then
/// the world map image.
struct Hud {
    scale: u32,
    rotate: u32,
    depth: u32,
    mode: u32,
    pos_x: u32,
    pos_y: u32,
    local_zoom: u32,
    world_zoom: u32,
    world_rect: u32,
    world_tile: u32,
    map_markers: u32,
    w_quests: u32,
    doors: u32,
    l_quests: u32,
    locals: Vec<u32>,
}

fn hud(e: &mut Engine) -> Hud {
    let root = new_tile(e, 0, None);
    let menu = alloc(0x10);
    e.mem.set_u32(menu + 4, root);
    e.mem.set_u32(0x011d_96c0, menu);
    let mm = new_tile(e, root, Some(b"JIPMiniMap"));
    let scale = values(e, mm, &[(0x1005, 2.0)])[0];
    let frame = new_tile(e, mm, None);
    let marker = new_tile(e, mm, None);
    new_tile(e, mm, None);
    let v = values(e, marker, &[(0xffa, 0.0), (0xfad, 10.0)]);
    let (rotate, depth) = (v[0], v[1]);
    let v = values(e, frame, &[(0x1004, 1.0), (0x1005, 3.0), (0x1006, 4.0)]);
    let (mode, pos_x, pos_y) = (v[0], v[1], v[2]);
    let world_rect = new_tile(e, frame, None);
    let local = new_tile(e, frame, None);
    new_tile(e, frame, None);
    let local_zoom = values(e, local, &[(0xfb8, 0.5)])[0];
    let world_zoom = values(e, world_rect, &[(0xfb8, 1.0)])[0];
    let l_quests = new_tile(e, local, None);
    let doors = new_tile(e, local, None);
    let mut locals = Vec::new();
    for _ in 0..9 {
        let image = new_tile(e, local, None);
        let node = ni_node(e);
        let children = alloc(4);
        let shape = alloc(0xc4);
        e.mem.set_u32(shape, SHAPE_VTBL);
        e.mem.set_u32(children, shape);
        e.mem.set_u32(node + 0xa0, children);
        e.mem.set_u16(node + 0xa6, 1);
        e.mem.set_u32(image + 0x2c, node);
        let data = alloc(0x58);
        e.mem.set_u32(shape + 0xb8, data);
        for field in [0x20, 0x24, 0x28, 0x2c, 0x48] {
            e.mem.set_u32(data + field, alloc(4));
        }
        let property_node = alloc(12);
        e.mem.set_u32(shape + 0x24, property_node);
        let shader = alloc(0x70);
        e.mem.set_u32(image + 0x40, shader);
        locals.push(image);
    }
    locals.reverse();
    let w_quests = new_tile(e, world_rect, None);
    let map_markers = new_tile(e, world_rect, None);
    let world_tile = new_tile(e, world_rect, None);
    Hud {
        scale,
        rotate,
        depth,
        mode,
        pos_x,
        pos_y,
        local_zoom,
        world_zoom,
        world_rect,
        world_tile,
        map_markers,
        w_quests,
        doors,
        l_quests,
        locals,
    }
}

/// JIP MiniMap.esp loaded at index 7; the script of its quest is at that index.
fn the_mod(e: &mut Engine) {
    game_globals(e);
    loaded_mod(e, 7, MOD_INFOS, b"JIP MiniMap.esp");
    e.mem.set_u8(SCRIPT + 0xf, 7);
    e.mem.set_u32(SCRIPT + 0x40, QUEST);
    e.mem.set_u32(SCRIPT_DATA + 0x2fe5, 0x0017_3031);
}

/// The engine with the HUD, the mod, and stand-ins for every game call `InitMiniMap` makes.
fn setup() -> (Engine, Hud) {
    let mut e = engine();
    e.map(AREA, 0x40_0000);
    for page in [
        0x011d_9000,
        0x011d_e000,
        0x011f_9000,
        0x011c_3000,
        0x011c_a000,
        0x011d_8000,
    ] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem
        .set_u32(NODE_VTBL + 0xc, super::super::nif_nodes::RETURN_THIS);
    e.mem.set_u32(NODE_VTBL + ADD_OBJECT_SLOT, ADD_OBJECT_FN);
    e.mem.set_u32(SHAPE_VTBL + 0x18, RETURN_THIS2);
    e.register(ADD_OBJECT_FN, |_, w| {
        record(ADD_OBJECT_FN, w);
        eax(0)
    });
    for f in [CAMERA_CREATE, LIGHT_CREATE, ALPHA_CREATE] {
        e.register(f, |_, w| {
            record(0, w);
            eax(alloc(0x120))
        });
    }
    e.register(NODE_CREATE, |e, _| {
        let n = alloc(0xc0);
        e.mem.set_u32(n, NODE_VTBL);
        eax(n)
    });
    e.register(NI_ALLOC, |_, w| eax(alloc(w[0])));
    // A value's refresh, the geometry property steps, the fixed-string maker.
    for f in [0x00a0_9410, 0x00a5_a040, 0x00b5_7bd0] {
        e.register(f, |_, _| eax(0));
    }
    e.register(ADD_STRING, |_, _| eax(0x0dea_7100));
    e.register(SHAPE_DATA_CONSTRUCT, |_, w| {
        record(SHAPE_DATA_CONSTRUCT, w);
        eax(w[0])
    });
    e.register(SHAPE_CONSTRUCT, |e, w| {
        record(SHAPE_CONSTRUCT, w);
        e.mem.set_u32(w[0], SHAPE_VTBL);
        eax(w[0])
    });
    let game_fog = alloc(0x64);
    for i in 0..0x19 {
        e.mem.set_u32(game_fog + 4 * i, 0x100 + i);
    }
    e.mem.set_u32(0x011d_eb00, game_fog);
    let scene = ni_node(&mut e);
    e.mem.set_u32(0x011f_91c8, scene);
    let lod_root = ni_node(&mut e);
    e.mem.set_u32(0x011d_ea18, lod_root);
    let h = hud(&mut e);
    the_mod(&mut e);
    (e, h)
}

fn run(e: &mut Engine) -> u32 {
    call(e, 0x2873, &[]).0
}

// ---- the mesh and the world map's numbers ----

/// The local maps' mesh: 17 x 17 vertices 8 apart (x, 0, -z), texture coordinates 1/16 apart,
/// and JIP's two triangles per square.
#[test]
fn the_local_map_mesh() {
    let mut e = engine();
    e.map(AREA, 0x40_0000);
    let (v, uv, t) = local_map_mesh(&mut e);
    let vertex = |i: u32| [0, 4, 8].map(|o| e.mem.f32(v + 12 * i + o));
    assert_eq!(vertex(0), [0.0, 0.0, 0.0]);
    assert_eq!(vertex(1), [0.0, 0.0, -8.0]);
    assert_eq!(vertex(0x11), [8.0, 0.0, 0.0]);
    assert_eq!(vertex(0x120), [128.0, 0.0, -128.0]);
    assert_eq!(e.mem.f32(uv + 8 * 0x12), 0.0625);
    assert_eq!(e.mem.f32(uv + 8 * 0x12 + 4), 0.0625);
    let tri = |i: u32| [0, 2, 4].map(|o| e.mem.u16(t + 6 * i + o));
    assert_eq!(tri(0), [0x11, 0, 1]);
    assert_eq!(tri(1), [1, 0x12, 0x11]);
    assert_eq!(tri(2), [0x12, 1, 2]);
    assert_eq!(tri(32), [0x22, 0x11, 0x12]);
    assert_eq!(tri(511), [0x10f, 0x120, 0x11f]);
}

/// The world map's corner and size from the cell corners, the processor's 1/x estimate times
/// 0.796875.
#[test]
fn the_world_dimensions() {
    let e = engine();
    let world = FORMS + 0x1000;
    for (i, v) in [-30i16, 30, 20, -25].iter().enumerate() {
        e.mem.set_u16(world + 0x88 + 2 * i as u32, *v as u16);
    }
    let mut d = WorldDimensions::default();
    init_dimensions(&e, &mut d, world);
    assert_eq!(d.min, [-30.0 * 4096.0, 31.0 * 4096.0]);
    let k = f32::from_bits(0x3f4c_0000);
    let rcp = super::super::havok_refs::rcp_estimate;
    assert_eq!(d.size, [rcp(51.0 * 4096.0) * k, rcp(-56.0 * 4096.0) * k]);
}

/// The scale and offsets; with a scale other than 1 the position mods from the world's bounds.
#[test]
fn the_world_pos_mods() {
    let e = engine();
    let world = FORMS + 0x1000;
    for (i, v) in [1.0f32, 100.0, -200.0, 7.0].iter().enumerate() {
        e.mem.set_f32(world + 0x90 + 4 * i as u32, *v);
    }
    let mut d = WorldDimensions::default();
    init_pos_mods(&e, &mut d, world);
    assert_eq!((d.scale, d.offset), (1.0, [100.0, -200.0]));
    assert_eq!(d.pos_mod, [7.0, 0.0], "the fourth word, as JIP's movups");
    e.mem.set_f32(world + 0x90, 0.5);
    for (i, v) in [-1000.0f32, -2000.0, 3000.0, 4000.0].iter().enumerate() {
        e.mem.set_f32(world + 0xa0 + 4 * i as u32, *v);
    }
    init_pos_mods(&e, &mut d, world);
    let m = 0.5f32 - 0.5 * 0.5;
    assert_eq!(d.pos_mod, [2000.0 * m + 100.0, 2000.0 * m - 200.0]);
}

/// A position's place: `(pos * scale + mod - min) * size + 0.1015625`, and its cell (`>> 12`).
#[test]
fn a_place_on_the_world_map() {
    let d = WorldDimensions {
        min: [-4096.0, 4096.0],
        size: [0.001, -0.002],
        scale: 1.0,
        offset: [10.0, -10.0],
        pos_mod: [0.0; 2],
        coord: 0,
    };
    let mut coord = 0;
    let p = world_map_pos_mults([5000.0, -9000.0], &d, Some(&mut coord));
    let k = f32::from_bits(0x3dd0_0000);
    assert_eq!(
        p,
        [
            (5010.0 + 4096.0) * 0.001 + k,
            (-9010.0 - 4096.0) * -0.002 + k
        ]
    );
    assert_eq!(coord, (1 << 16) | ((-3i32) as u32 & 0xffff));
    let scaled = WorldDimensions {
        scale: 2.0,
        pos_mod: [1.0, 2.0],
        ..d
    };
    let p = world_map_pos_mults([100.0, 100.0], &scaled, None);
    assert_eq!(p[0], (201.0 + 4096.0) * 0.001 + k);
}

/// The root map world: up the parents while the "use the parent's map" flag is set.
#[test]
fn the_root_map_world() {
    let e = engine();
    let (a, b, c) = (FORMS + 0x1000, FORMS + 0x1200, FORMS + 0x1400);
    e.mem.set_u32(a + 0x70, b);
    e.mem.set_u8(a + 0x4e, 4);
    e.mem.set_u32(b + 0x70, c);
    assert_eq!(root_map_world(&e, a), b);
    e.mem.set_u8(b + 0x4e, 4);
    assert_eq!(root_map_world(&e, a), c);
    assert_eq!(root_map_world(&e, c), c);
}

// ---- InitMiniMap ----

/// Only JIP MiniMap's script starts it, and only with its HUD component.
#[test]
fn only_the_mods_script_starts_it() {
    let (mut e, _) = setup();
    e.mem.set_u8(SCRIPT + 0xf, 3);
    run(&mut e);
    assert!(!patched(&mut e));
    assert_eq!(state(&mut e).index, 7);
    e.mem.set_u8(SCRIPT + 0xf, 7);
    let menu = e.mem.u32(0x011d_96c0);
    let root = e.mem.u32(menu + 4);
    let text = e.mem.u32(e.mem.u32(e.mem.u32(root + 4) + 8) + 0x20);
    e.mem.write(text, b"Other");
    run(&mut e);
    assert!(!patched(&mut e));
}

/// The start: the tiles kept, the marker's depth 34, the script made to run 60 a second, the nine
/// shapes on JIP's mesh with their own colours and the alpha property (flags 0x8D), the zoom and
/// scales set, the camera, light and fog made, the water planes added, the patches in; a second
/// call changes nothing.
#[test]
fn the_start() {
    let (mut e, h) = setup();
    run(&mut e);
    assert!(patched(&mut e));
    assert_eq!(e.mem.u8(SCRIPT_DATA + 0x2fe5), 0x33);
    assert_eq!(e.mem.f32(QUEST + 0x40), 1.0 / 60.0);
    assert_eq!(e.mem.f32(h.depth + 8), 34.0);
    let s = state(&mut e);
    assert_eq!(
        (s.mode, s.scale, s.pc_marker_rotate, s.pos_x, s.pos_y),
        (h.mode, h.scale, h.rotate, h.pos_x, h.pos_y)
    );
    assert_eq!(
        (
            s.local_map_zoom,
            s.world_map_zoom,
            s.world_map_rect,
            s.world_map_tile
        ),
        (h.local_zoom, h.world_zoom, h.world_rect, h.world_tile)
    );
    assert_eq!(
        (
            s.map_markers_rect,
            s.w_quest_markers_rect,
            s.door_markers_rect,
            s.l_quest_markers_rect
        ),
        (h.map_markers, h.w_quests, h.doors, h.l_quests)
    );
    let props = s.shader_props;
    let shapes = s.shapes;
    let datas = s.shape_datas;
    let camera = s.camera;
    let light = s.light;
    let fog = s.fog;
    assert_eq!(s.zoom_current, 0.5);
    assert!(s.in_exterior);
    drop(s);
    for (i, &image) in h.locals.iter().enumerate() {
        assert_eq!(
            props[i],
            e.mem.u32(image + 0x40),
            "the local tiles, the last first"
        );
    }
    let colours0 = e.mem.u32(datas[0] + 0x28);
    for i in 0..9 {
        let data = datas[i];
        assert_eq!(e.mem.u16(data + 8), 0x121);
        assert_eq!(e.mem.u16(data + 0x40), 0x200);
        assert_eq!(e.mem.u32(data + 0x44), 0x600);
        assert_eq!(e.mem.u32(data + 0x24), 0, "no normals");
        assert_eq!(
            e.mem.u32(data + 0x20),
            e.mem.u32(datas[0] + 0x20),
            "one mesh"
        );
        assert_eq!(e.mem.u32(data + 0x28), colours0 + 0x121 * 16 * i as u32);
        assert_eq!(e.mem.f32(shapes[i] + 0x64), (0.5 + 0.5) * 2.0);
        let alpha = e.mem.u32(shapes[i] + 0x9c);
        assert_eq!(e.mem.u16(alpha + 0x18), 0x8d);
        assert_eq!(e.mem.u32(e.mem.u32(shapes[i] + 0x24) + 8), alpha);
    }
    assert_eq!(e.mem.f32(h.local_zoom + 8), 0.5);
    assert_eq!(e.mem.u32(camera + 4), 2);
    assert_eq!(e.mem.f32(camera + 0x70), 1.0);
    assert_eq!(e.mem.f32(camera + 0x80), -1.0);
    assert_eq!(e.mem.f32(camera + 0xdc), -2048.0);
    assert_eq!(e.mem.f32(camera + 0xf0), 131_072.0);
    assert_eq!(e.mem.u8(camera + 0xf4), 1);
    assert_eq!(e.mem.f32(camera + 0x110), 0.001);
    assert_eq!(e.mem.f32(light + 0xc8), 175.0 / 255.0);
    #[allow(clippy::approx_constant)]
    let y = -0.523_598_8;
    assert_eq!(e.mem.f32(light + 0xf4), y);
    assert_eq!(e.mem.u32(fog + 4), 0x101, "a copy of the game's");
    assert_eq!(e.mem.f32(fog + 0x20), 23.0 / 255.0);
    assert_eq!(e.mem.f32(fog + 0x30), f32::MAX);
    let added = calls_to(ADD_OBJECT_FN);
    assert_eq!(added[0][1], camera);
    assert_eq!(added.last().unwrap()[0], e.mem.u32(0x011d_ea18));
    let n = calls().len();
    run(&mut e);
    assert_eq!(calls().len(), n, "once");
}

/// The water planes: one shape data of four corners, nine hidden squares at JIP's places.
#[test]
fn the_fake_water_planes() {
    let (mut e, _) = setup();
    let node = create_fake_water_planes(&mut e);
    assert_eq!(e.mem.u32(node + 4), 2);
    let data = calls_to(SHAPE_DATA_CONSTRUCT);
    assert_eq!(data.len(), 1);
    let w = &data[0];
    assert_eq!((w[1], w[8]), (4, 2));
    let corners = w[2];
    assert_eq!(e.mem.f32(corners), -2048.0);
    assert_eq!(e.mem.f32(corners + 4), 2048.0);
    assert_eq!(e.mem.f32(corners + 0x24), 2048.0);
    assert_eq!(e.mem.f32(corners + 0x28), -2048.0);
    assert_eq!(e.mem.u32(w[4]), 0x3db8_b8b9);
    assert_eq!(e.mem.u16(w[9] + 2), 1);
    assert_eq!(e.mem.u16(w[9] + 10), 3);
    let shapes = calls_to(SHAPE_CONSTRUCT);
    assert_eq!(shapes.len(), 9);
    let last = shapes[8][0];
    assert_eq!(e.mem.u8(last + 0x30) & 1, 1);
    assert_eq!(e.mem.f32(last + 0x58), 10240.0);
    assert_eq!(e.mem.f32(last + 0x5c), -2048.0);
}

/// In an interior at the start (`TES` +0x34 set): the zoom starts 0.25 higher (0.75), not in an
/// exterior, the zoom value set to it and the shapes' scale `(0.5 + 0.75) * scale`.
#[test]
fn the_start_in_an_interior() {
    let (mut e, h) = setup();
    let tes = alloc(0x40);
    e.mem.set_u32(TES, tes);
    e.mem.set_u32(tes + 0x34, alloc(0x10));
    run(&mut e);
    let s = state(&mut e);
    assert_eq!(s.zoom_current, 0.75);
    assert!(!s.in_exterior);
    let shapes = s.shapes;
    drop(s);
    assert_eq!(e.mem.f32(h.local_zoom + 8), 0.75);
    for shape in shapes {
        assert_eq!(e.mem.f32(shape + 0x64), (0.5 + 0.75) * 2.0);
    }
}

/// A world (its map: usable width, corners, scale and offsets) with a persistent cell whose
/// objects are `refs`.
fn world_with(
    e: &mut Engine,
    parent: u32,
    flags: u16,
    usable: u32,
    scale: [f32; 4],
    refs: &[u32],
) -> u32 {
    let world = alloc(0xc0);
    let cell = alloc(0xc0);
    e.mem.set_u32(world + 0x34, cell);
    e.mem.set_u16(world + 0x4e, flags);
    e.mem.set_u32(world + 0x70, parent);
    e.mem.set_u32(world + 0x80, usable);
    for (i, v) in [-30i16, 30, 20, -25].iter().enumerate() {
        e.mem.set_u16(world + 0x88 + 2 * i as u32, *v as u16);
    }
    for (i, v) in scale.iter().enumerate() {
        e.mem.set_f32(world + 0x90 + 4 * i as u32, *v);
    }
    for (i, v) in [-1000.0f32, -2000.0, 3000.0, 4000.0].iter().enumerate() {
        e.mem.set_f32(world + 0xa0 + 4 * i as u32, *v);
    }
    let mut node = cell + 0xac;
    for (i, &r) in refs.iter().enumerate() {
        e.mem.set_u32(node, r);
        let next = if i + 1 < refs.len() { alloc(8) } else { 0 };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
    world
}

/// A reference at `pos` whose base has the form id `base_id`; a map marker's data in its extra
/// list (`ExtraMapMarker`) when it is one.
fn placed(e: &mut Engine, base_id: u32, pos: [f32; 2]) -> (u32, u32) {
    let refr = alloc(0x60);
    let base = alloc(0x20);
    e.mem.set_u32(base + 0xc, base_id);
    e.mem.set_u32(refr + 0x20, base);
    e.mem.set_f32(refr + 0x30, pos[0]);
    e.mem.set_f32(refr + 0x34, pos[1]);
    let data = alloc(0x10);
    if base_id == 0x10 {
        let extra = alloc(0x10);
        put_extra(e, refr + 0x44, extra, 0x2c, data);
    }
    (refr, data)
}

/// Every world's map markers, by root world and by cell: a child world using its parent's map
/// (flag 4 at +0x4E) goes under the parent, its places by its own scale and offsets on the
/// parent's map; a world with no parent and no map is passed over; a reference that is not a map
/// marker is left out.
#[test]
fn every_worlds_map_markers_by_root_and_cell() {
    let mut e = engine();
    e.map(AREA, 0x40_0000);
    if !e.mem.is_mapped(0x011c_3000) {
        e.map(0x011c_3000, 0x1000);
    }
    extra_stand_ins(&mut e);
    let (a, a_data) = placed(&mut e, 0x10, [1000.0, 2000.0]);
    let (other, _) = placed(&mut e, 0x20, [0.0, 0.0]);
    let (b, b_data) = placed(&mut e, 0x10, [-5000.0, 9000.0]);
    let (c, _) = placed(&mut e, 0x10, [0.0, 0.0]);
    let root = world_with(&mut e, 0, 0, 1000, [1.0, 0.0, 0.0, 0.0], &[a, other]);
    let child = world_with(&mut e, root, 4, 0, [0.5, 100.0, -200.0, 0.0], &[b]);
    let skipped = world_with(&mut e, 0, 0, 0, [1.0, 0.0, 0.0, 0.0], &[c]);
    let handler = alloc(0x20);
    e.mem.set_u32(0x011c_3f2c, handler);
    let (n2, n3) = (alloc(8), alloc(8));
    for (node, world, next) in [
        (handler + 0x10, root, n2),
        (n2, child, n3),
        (n3, skipped, 0),
    ] {
        e.mem.set_u32(node, world);
        e.mem.set_u32(node + 4, next);
    }
    let got = world_markers(&mut e);
    assert_eq!(got.keys().copied().collect::<Vec<_>>(), [root]);
    let mut dims = WorldDimensions::default();
    init_pos_mods(&e, &mut dims, root);
    init_dimensions(&e, &mut dims, root);
    let (mut ca, mut cb) = (0, 0);
    let pa = world_map_pos_mults([1000.0, 2000.0], &dims, Some(&mut ca));
    init_pos_mods(&e, &mut dims, child);
    let pb = world_map_pos_mults([-5000.0, 9000.0], &dims, Some(&mut cb));
    assert_ne!(ca, cb);
    let cells = &got[&root];
    assert_eq!(cells.len(), 2);
    assert_eq!(
        cells[&ca],
        [MarkerInfo {
            refr: a,
            data: a_data,
            pos: pa
        }]
    );
    assert_eq!(
        cells[&cb],
        [MarkerInfo {
            refr: b,
            data: b_data,
            pos: pb
        }]
    );
}
