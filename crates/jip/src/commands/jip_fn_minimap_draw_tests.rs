//! Review round 1 (part 2): `UpdateMiniMap` run whole, in an exterior and in an interior, with the
//! game's drawing stood in. The stand-in texture manager makes a new texture per draw (linked at
//! the game's texture list's end, as the game's texture constructor does); the draw pass records
//! the scene as the drawing sees it; the texture's deleting destructor unlinks and marks it freed.
//! Every expectation from JIP's `Cmd_UpdateMiniMap_Execute`, `GenerateLocalMapExterior`,
//! `GenerateLocalMapInterior` and `BSTextureManager::GenerateRenderedTexture`
//! (functions_jip/jip_fn_minimap.h, internal/netimmerse.cpp).

use super::tests::{alloc, eax, record, setup, texture_list};
use super::*;
use crate::commands::test_support::{calls, calls_to, extra_stand_ins, put_extra, CALLS};
use std::cell::RefCell;
use std::collections::HashMap;

/// The drawing's game calls (as `rendered_textures` names them) and the node update.
const CREATE_RENDERED_TEXTURE: u32 = 0x00b6_d170;
const BEGIN_FRAME: u32 = 0x00b6_b8d0;
const CULL_SCENE: u32 = 0x00b6_bee0;
const DRAW_SCENE: u32 = 0x00b6_c0d0;
const END_FRAME: u32 = 0x00b6_b790;
const NI_ALLOC: u32 = 0x00aa_13e0;
const CULLING_PROCESS_CTOR: u32 = 0x004a_0eb0;
const ACCUMULATOR_CTOR: u32 = 0x00b6_60d0;
const UPDATE_DOWNWARD_PASS: u32 = 0x00a5_dd70;
const TEXTURE_MANAGER: u32 = 0x011f_91a8;
const RENDERER: u32 = 0x011f_4748;
/// An accumulator's render mode (+0x19C), as the drawing sets it from the texture settings.
const ACCUMULATOR_RENDER_MODE: u32 = 0x19c;
/// The stand-in `BSRenderedTexture`'s deleting destructor (its vtable +4) and the word a freed
/// texture's vtable becomes.
const BS_RENDERED_DELETE: u32 = 0x00de_1004;
const FREED: u32 = 0x0dea_dead;
/// The game's words before the update: the TES's light, the game's fog, the lighting passes.
const TES_LIGHT_WORD: u32 = 0x0dea_7001;
const GAME_FOG: u32 = 0x0dea_7002;
const PASSES_BEFORE: u8 = 0x20;
/// JIP's own light and fog (`s_mmDirectionalLight`, `s_mmFogProperty`).
const MM_LIGHT: u32 = 0x0dea_7101;
const MM_FOG: u32 = 0x0dea_7102;

/// What the draw pass saw.
#[derive(Clone, Debug)]
struct Drawn {
    /// The camera's world translation and the word after it (its world scale).
    camera: [f32; 4],
    sun_light: u32,
    fog: u32,
    underwater: u8,
    passes: u8,
    draw_flag: u8,
    render_mode: u32,
    /// The hidden bit of each object in [`WATCH`].
    hidden: Vec<bool>,
    /// The flags byte (+0x25) of each land shader in [`LAND`].
    land: Vec<u8>,
}

thread_local! {
    static WATCH: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static LAND: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static DRAWN: RefCell<Vec<Drawn>> = const { RefCell::new(Vec::new()) };
}

fn drawn() -> Vec<Drawn> {
    DRAWN.with(|d| d.borrow().clone())
}

fn forget_calls() {
    CALLS.with(|c| c.borrow_mut().clear());
    DRAWN.with(|d| d.borrow_mut().clear());
}

fn map_page(e: &mut Engine, page: u32) {
    if !e.mem.is_mapped(page) {
        e.map(page, 0x1000);
    }
}

/// A texture out of the game's texture list (its neighbours joined, head and tail mended).
fn unlink(e: &mut Engine, t: u32) {
    let prev = e.mem.u32(t + TEXTURE_PREV);
    let next = e.mem.u32(t + TEXTURE_NEXT);
    if prev != 0 {
        e.mem.set_u32(prev + TEXTURE_NEXT, next);
    } else if e.mem.u32(TEXTURE_LIST_HEAD) == t {
        e.mem.set_u32(TEXTURE_LIST_HEAD, next);
    }
    if next != 0 {
        e.mem.set_u32(next + TEXTURE_PREV, prev);
    } else if e.mem.u32(TEXTURE_LIST_TAIL) == t {
        e.mem.set_u32(TEXTURE_LIST_TAIL, prev);
    }
}

/// The stand-ins for the drawing: a new rendered texture per ask (its texture a local map texture
/// at the list's end, counted 0), `Ni_Alloc` and the constructors, the frame calls, the draw pass
/// recording a [`Drawn`], both deleting destructors, the scene lights' lock, the node update.
fn drawing(e: &mut Engine) {
    for page in [
        0x011f_9000,
        0x011a_d000,
        0x011d_8000,
        0x011d_9000,
        0x0109_d000,
        0x011f_f000,
    ] {
        map_page(e, page);
    }
    e.mem.set_u32(TEXTURE_MANAGER, alloc(0x40));
    e.mem.set_u32(RENDERER, alloc(0x600));
    e.mem
        .set_u32(VTBL_RENDERED_TEXTURE + 4, RENDERED_TEXTURE_DELETE);
    e.register(CREATE_RENDERED_TEXTURE, |e, w| {
        record(CREATE_RENDERED_TEXTURE, w);
        let (rendered, vtable, made) = (alloc(0x40), alloc(0x10), alloc(0x40));
        e.mem.set_u32(vtable + 4, BS_RENDERED_DELETE);
        e.mem.set_u32(rendered, vtable);
        e.mem.set_u32(rendered + 0x30, made);
        e.mem.set_u32(made, VTBL_RENDERED_TEXTURE);
        let tail = e.mem.u32(TEXTURE_LIST_TAIL);
        if tail != 0 {
            e.mem.set_u32(tail + TEXTURE_NEXT, made);
        } else {
            e.mem.set_u32(TEXTURE_LIST_HEAD, made);
        }
        e.mem.set_u32(made + TEXTURE_PREV, tail);
        e.mem.set_u32(TEXTURE_LIST_TAIL, made);
        eax(rendered)
    });
    e.register(NI_ALLOC, |_, w| {
        record(NI_ALLOC, w);
        eax(alloc(w[0]))
    });
    e.register(CULLING_PROCESS_CTOR, |_, w| eax(w[0]));
    e.register(ACCUMULATOR_CTOR, |_, w| eax(w[0]));
    e.register(BEGIN_FRAME, |_, w| {
        record(BEGIN_FRAME, w);
        eax(0)
    });
    e.register(CULL_SCENE, |_, w| {
        record(CULL_SCENE, w);
        eax(0)
    });
    e.register(END_FRAME, |_, w| {
        record(END_FRAME, w);
        eax(0)
    });
    e.register(DRAW_SCENE, |e, w| {
        record(DRAW_SCENE, w);
        let scene = e.mem.u32(SHADOW_SCENE_NODE);
        let sun = e.mem.u32(scene + SCENE_SUN);
        let d = Drawn {
            camera: std::array::from_fn(|i| e.mem.f32(w[0] + WORLD_TRANSLATE + 4 * i as u32)),
            sun_light: e.mem.u32(sun + LIGHT_LIGHT),
            fog: e.mem.u32(scene + SCENE_FOG),
            underwater: e.mem.u8(UNDERWATER_FOG),
            passes: e.mem.u8(LIGHTING_PASSES),
            draw_flag: e.mem.u8(DRAW_FLAG),
            render_mode: e.mem.u32(w[1] + ACCUMULATOR_RENDER_MODE),
            hidden: WATCH.with(|x| {
                x.borrow()
                    .iter()
                    .map(|&o| e.mem.u32(o + FLAGS) & 1 != 0)
                    .collect()
            }),
            land: LAND.with(|x| {
                x.borrow()
                    .iter()
                    .map(|&s| e.mem.u8(s + LAND_FLAGS))
                    .collect()
            }),
        };
        DRAWN.with(|x| x.borrow_mut().push(d));
        eax(0)
    });
    e.register(RENDERED_TEXTURE_DELETE, |e, w| {
        record(RENDERED_TEXTURE_DELETE, &w[..1]);
        unlink(e, w[0]);
        e.mem.set_u32(w[0], FREED);
        eax(0)
    });
    e.register(BS_RENDERED_DELETE, |_, w| {
        record(BS_RENDERED_DELETE, w);
        eax(0)
    });
    e.register(LOCK_ENTER, |_, w| {
        record(LOCK_ENTER, w);
        eax(0)
    });
    e.register(LOCK_LEAVE, |_, w| {
        record(LOCK_LEAVE, w);
        eax(0)
    });
    e.register(UPDATE_DOWNWARD_PASS, |_, w| {
        record(UPDATE_DOWNWARD_PASS, &w[..1]);
        eax(0)
    });
    WATCH.with(|x| x.borrow_mut().clear());
    LAND.with(|x| x.borrow_mut().clear());
    DRAWN.with(|x| x.borrow_mut().clear());
}

fn watch(objects: &[u32]) {
    WATCH.with(|x| x.borrow_mut().extend_from_slice(objects));
}

fn watched(object: u32) -> usize {
    WATCH.with(|x| x.borrow().iter().position(|&o| o == object).unwrap())
}

/// A node with `kids` as its children (+0xA0, count +0xA6).
fn node_with(e: &mut Engine, kids: &[u32]) -> u32 {
    let node = alloc(0xb0);
    let array = alloc(4 * kids.len().max(1) as u32);
    for (i, &k) in kids.iter().enumerate() {
        e.mem.set_u32(array + 4 * i as u32, k);
    }
    e.mem.set_u32(node + CHILDREN, array);
    e.mem.set_u16(node + CHILD_COUNT, kids.len() as u16);
    node
}

/// An exterior cell's parts the drawing touches.
#[derive(Clone, Copy, Debug)]
struct ExtCell {
    at: (i32, i32),
    cell: u32,
    first: u32,
    node4: u32,
    /// The four land quads' shader properties: the first three flagged for the draw (+0x38 set),
    /// the last not (+0x38 0).
    land: [u32; 4],
}

/// The scene around the player.
struct World {
    player: u32,
    tes: u32,
    scene: u32,
    sun: u32,
    cells: Vec<ExtCell>,
    /// Two animated references' nodes in (0, 0): `BSXFlags` 0x21 (hidden for the draw) and 0x23.
    animated: [u32; 2],
    /// (0, 1)'s placed water's node; its height.
    water_node: u32,
    water_z: f32,
    /// A point light shown, one already hidden.
    lights: [u32; 2],
    extra_node: u32,
    planes: [u32; 9],
}

impl World {
    fn cell(&self, x: i32, y: i32) -> ExtCell {
        *self.cells.iter().find(|c| c.at == (x, y)).unwrap()
    }
}

/// A node whose `BSXFlags` extra holds `flags`.
fn bsx_node(e: &mut Engine, flags: u32) -> u32 {
    let node = alloc(0x40);
    let (list, extra) = (alloc(4), alloc(0x10));
    e.mem.set_u32(extra, VTBL_BSX_FLAGS);
    e.mem.set_u32(extra + 0xc, flags);
    e.mem.set_u32(list, extra);
    e.mem.set_u32(node + EXTRA_LIST, list);
    e.mem.set_u16(node + EXTRA_COUNT, 1);
    node
}

/// An exterior cell at (x, y) of `world`, id `id`, its render data's nodes and land quads.
fn ext_cell(e: &mut Engine, world: u32, x: i32, y: i32, id: u32) -> ExtCell {
    let cell = alloc(0xd0);
    e.mem.set_u32(cell + CELL_ID, id);
    e.mem.set_u32(cell + CELL_WORLD, world);
    let coords = alloc(8);
    e.mem.set_u32(coords, x as u32);
    e.mem.set_u32(coords + 4, y as u32);
    e.mem.set_u32(cell + CELL_COORDS, coords);
    let land: [u32; 4] = std::array::from_fn(|q| {
        let shader = alloc(0x40);
        e.mem.set_u32(shader + LAND_WORD, u32::from(q != 3));
        shader
    });
    let quads = land.map(|shader| {
        let shape = alloc(0xb0);
        e.mem.set_u32(shape + GEOMETRY_SHADER, shader);
        node_with(e, &[shape])
    });
    let land_node = node_with(e, &quads);
    let (first, node4) = (alloc(0x40), alloc(0x40));
    let root = node_with(e, &[first, 0, land_node, 0, node4]);
    let render = alloc(0x80);
    e.mem.set_u32(render, root);
    e.mem.set_u32(cell + CELL_RENDER_DATA, render);
    ExtCell {
        at: (x, y),
        cell,
        first,
        node4,
        land,
    }
}

/// The game's words the update reads and JIP MiniMap's state, the player at `pos` in `cell`.
fn common(e: &mut Engine, cell: u32, pos: [f32; 2]) -> (u32, u32, u32, u32, MiniMapState) {
    extra_stand_ins(e);
    drawing(e);
    let player = alloc(0x800);
    e.mem.set_u32(PLAYER, player);
    e.mem.set_u32(player + PLAYER_CELL, cell);
    e.mem.set_f32(player + REF_POSITION, pos[0]);
    e.mem.set_f32(player + REF_POSITION + 4, pos[1]);
    let tes = alloc(0x100);
    e.mem.set_u32(TES, tes);
    e.mem.set_u32(tes + TES_LIGHT, TES_LIGHT_WORD);
    e.mem.set_u32(GAME_FOG_PROPERTY, GAME_FOG);
    let (scene, sun) = (alloc(0x140), alloc(0x100));
    e.mem.set_u32(SHADOW_SCENE_NODE, scene);
    e.mem.set_u32(scene + SCENE_SUN, sun);
    e.mem.set_u32(sun + LIGHT_LIGHT, TES_LIGHT_WORD);
    e.mem.set_u32(scene + SCENE_FOG, GAME_FOG);
    e.mem.set_u8(UNDERWATER_FOG, 0);
    e.mem.set_u8(LIGHTING_PASSES, PASSES_BEFORE);
    e.mem.set_u8(DRAW_FLAG, 1);
    let value = |e: &mut Engine, v: f32| {
        let at = alloc(0x14);
        e.mem.set_f32(at + VALUE_NUM, v);
        at
    };
    let mut s = MiniMapState {
        mode: value(e, 0.0),
        scale: value(e, 2.0),
        local_map_zoom: value(e, 1.0),
        pc_marker_rotate: value(e, 0.0),
        pos_x: value(e, 0.0),
        pos_y: value(e, 0.0),
        camera: alloc(0x120),
        light: MM_LIGHT,
        fog: MM_FOG,
        zoom_current: 1.0,
        show_flags: 8,
        black_plane: 0x0dea_b1ac,
        ..MiniMapState::default()
    };
    for (shape, prop) in s.shapes.iter_mut().zip(s.shader_props.iter_mut()) {
        *shape = alloc(0xc0);
        *prop = alloc(0x80);
    }
    for n in s.scene_nodes.iter_mut() {
        *n = alloc(0x40);
    }
    let planes: Vec<u32> = (0..9).map(|_| alloc(0x80)).collect();
    s.water_planes = node_with(e, &planes);
    (player, tes, scene, sun, s)
}

/// An exterior: a 5 x 5 grid centred on (0, 0) (`GridCellArray`: corner +4/+8, size +0xC, cells
/// +0x10) with a cell everywhere (ids 0x1000 + n); the player in (0, 0) with the grid on it. In
/// (0, 0) two animated references; in (0, 1) placed water; (1, 1) has land data whose top is
/// below 65536, (-1, -1) land data above it. A point light shown and one hidden; the sky's extra
/// node; nine water planes.
fn exterior_world(e: &mut Engine) -> (World, MiniMapState) {
    let world = alloc(0xd0);
    let mut cells = Vec::new();
    for x in -2..=2 {
        for y in -2..=2 {
            let id = 0x1000 + cells.len() as u32;
            cells.push(ext_cell(e, world, x, y, id));
        }
    }
    let home = cells.iter().find(|c| c.at == (0, 0)).unwrap().cell;
    let (player, tes, scene, sun, mut s) = common(e, home, [2048.0, 2048.0]);
    s.in_exterior = true;
    let grid = alloc(0x14);
    let array = alloc(4 * 25);
    e.mem.set_u32(grid + 0xc, 5);
    e.mem.set_u32(grid + 0x10, array);
    for c in &cells {
        let (gx, gy) = ((c.at.0 + 2) as u32, (c.at.1 + 2) as u32);
        e.mem.set_u32(array + 4 * (gy + gx + (gx << 2)), c.cell);
    }
    e.mem.set_u32(tes + TES_GRID, grid);
    // The animated references of (0, 0): a map of one bucket holding both.
    let animated = [bsx_node(e, 0x21), bsx_node(e, 0x23)];
    let render = e.mem.u32(home + CELL_RENDER_DATA);
    let (buckets, a, b) = (alloc(4), alloc(12), alloc(12));
    e.mem.set_u32(a, b);
    e.mem.set_u32(a + 8, animated[0]);
    e.mem.set_u32(b + 8, animated[1]);
    e.mem.set_u32(buckets, a);
    e.mem.set_u32(render + RENDER_ANIMATED + 4, 1);
    e.mem.set_u32(render + RENDER_ANIMATED + 8, buckets);
    // (0, 1)'s water: the cell flagged, its water reference placed (mod index 0xFF) with a node.
    let wet = cells.iter().find(|c| c.at == (0, 1)).unwrap().cell;
    e.mem.set_u8(wet + CELL_FLAGS, 2);
    let water_ref = alloc(0x70);
    e.mem.set_u8(water_ref + MOD_INDEX, 0xff);
    let water_z = 123.5;
    e.mem.set_f32(water_ref + REF_POSITION + 8, water_z);
    let (state, water_node) = (alloc(0x20), alloc(0x40));
    e.mem.set_u32(state + RENDER_STATE_NODE, water_node);
    e.mem.set_u32(water_ref + REF_RENDER_STATE, state);
    let wet_render = e.mem.u32(wet + CELL_RENDER_DATA);
    e.mem.set_u32(wet_render + RENDER_WATER, water_ref);
    // Land data: the heights block's top (+0x18) and the height JIP adds 16384 to (+0x1C).
    for (at, top) in [((1, 1), 30000.0f32), ((-1, -1), 70000.0)] {
        let c = cells.iter().find(|c| c.at == at).unwrap().cell;
        let (land, heights) = (alloc(0x30), alloc(0x20));
        e.mem.set_f32(heights + 0x18, top);
        e.mem.set_f32(heights + 0x1c, 5000.0);
        e.mem.set_u32(land + 0x28, heights);
        e.mem.set_u32(c + CELL_LAND, land);
    }
    // Two point lights in the scene's list: one shown, one hidden.
    let lights = [alloc(0x40), alloc(0x40)];
    e.mem.set_u32(lights[1] + FLAGS, 1);
    let mut head = 0;
    for &light in lights.iter().rev() {
        let (node, data) = (alloc(0x10), alloc(0x100));
        e.mem.set_u8(data + LIGHT_IS_POINT, 1);
        e.mem.set_u32(data + LIGHT_LIGHT, light);
        e.mem.set_u32(node, head);
        e.mem.set_u32(node + 8, data);
        head = node;
    }
    e.mem.set_u32(scene + SCENE_LIGHTS, head);
    let extra_node = alloc(0x40);
    e.mem.set_u32(EXTRA_HIDDEN_NODE, extra_node);
    let planes: [u32; 9] = std::array::from_fn(|i| {
        e.mem
            .u32(e.mem.u32(s.water_planes + CHILDREN) + 4 * i as u32)
    });
    // The planes hidden, as the last drawing left them.
    for p in planes {
        e.mem.set_u32(p + FLAGS, 1);
    }
    let w = World {
        player,
        tes,
        scene,
        sun,
        cells,
        animated,
        water_node,
        water_z,
        lights,
        extra_node,
        planes,
    };
    for c in &w.cells {
        watch(&[c.first, c.node4]);
        LAND.with(|x| x.borrow_mut().extend_from_slice(&c.land));
    }
    watch(&w.animated);
    watch(&[w.water_node, w.extra_node]);
    watch(&w.lights);
    watch(&s.scene_nodes);
    watch(&w.planes);
    (w, s)
}

/// The player's cell and the grid's centre in the TES's words.
fn place_player(e: &mut Engine, w: &World, cell: (i32, i32), grid: (i32, i32)) {
    e.mem.set_u32(w.tes + TES_CELL_XY, cell.0 as u32);
    e.mem.set_u32(w.tes + TES_CELL_XY + 4, cell.1 as u32);
    e.mem.set_u32(w.tes + TES_GRID_XY, grid.0 as u32);
    e.mem.set_u32(w.tes + TES_GRID_XY + 4, grid.1 as u32);
    let c = w.cell(cell.0, cell.1).cell;
    e.mem.set_u32(w.player + PLAYER_CELL, c);
}

/// The cell a draw was of, by the camera's place (the cell's centre).
fn drawn_cell(w: &World, d: &Drawn) -> ExtCell {
    *w.cells
        .iter()
        .find(|c| {
            d.camera[0] == ((c.at.0 << 12) + 0x800) as f32
                && d.camera[1] == ((c.at.1 << 12) + 0x800) as f32
        })
        .unwrap_or_else(|| panic!("no cell under the camera at {:?}", d.camera))
}

/// The nine cells around (x, y), in the slots' order (x - 1 to x + 1, each y - 1 to y + 1).
fn nine(x: i32, y: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for dx in -1..=1 {
        for dy in -1..=1 {
            out.push((x + dx, y + dy));
        }
    }
    out
}

fn flags_of(e: &Engine, objects: &[u32]) -> Vec<u32> {
    objects.iter().map(|&o| e.mem.u32(o + FLAGS)).collect()
}

/// Medium 3: a full exterior update (a new cell) with 60 maps already kept. During each of the
/// nine draws: the nine cells' fifth nodes, the 0x21-flagged animated node, the placed water's
/// node, the shown point light and the scene's nodes hidden, the drawn cell's first node hidden
/// and its flagged land quads marked 0x40 (the unflagged one not), the sun light JIP's, the fog
/// JIP's, underwater 1, 0x3C lighting passes, the byte `011AD7B4` 0, the normal render mode, the
/// camera over the cell's centre at 65536 (or the land's top + 16384 when that is at most 65536)
/// with world scale 0; the wet cell's plane shown at the water's height. After: every flag back
/// (the hidden light still hidden, the land quads unmarked), the sun light, fog, underwater flag,
/// lighting passes and draw byte back, the scene lights' lock entered and left once, every water
/// plane hidden, the planes placed at the map's corner and updated once, and the 69 kept maps
/// purged to 42 from JIP's first (cells 1 to 27), the nine new ones kept.
#[test]
fn an_exterior_update_draws_with_the_minimaps_scene_and_puts_it_back() {
    let mut e = setup();
    let (w, mut s) = exterior_world(&mut e);
    place_player(&mut e, &w, (0, 0), (0, 0));
    let kept = texture_list(&mut e, &mut s, 60);
    let watched_objects = WATCH.with(|x| x.borrow().clone());
    let before = flags_of(&e, &watched_objects);
    let land_before: Vec<u8> = LAND.with(|x| {
        x.borrow()
            .iter()
            .map(|&l| e.mem.u8(l + LAND_FLAGS))
            .collect()
    });
    forget_calls();

    update(&mut e, &mut s, 0, 8);

    let draws = drawn();
    assert_eq!(draws.len(), 9, "the nine new cells drawn");
    let around = nine(0, 0);
    let mut seen_cells = Vec::new();
    for d in &draws {
        let c = drawn_cell(&w, d);
        seen_cells.push(c.at);
        assert_eq!(
            (d.sun_light, d.fog, d.underwater, d.passes, d.draw_flag),
            (MM_LIGHT, MM_FOG, 1, 0x3c, 0),
            "{:?}",
            c.at
        );
        assert_eq!(d.render_mode, RENDER_NORMAL);
        let z = match c.at {
            (1, 1) => 5000.0 + 16384.0,
            _ => 65536.0,
        };
        assert_eq!((d.camera[2], d.camera[3]), (z, 0.0), "{:?}", c.at);
        for other in &w.cells {
            let in_nine = around.contains(&other.at);
            assert_eq!(d.hidden[watched(other.node4)], in_nine, "{:?}", other.at);
            assert_eq!(
                d.hidden[watched(other.first)],
                other.at == c.at,
                "only the drawn cell's first node"
            );
            let i = w.cells.iter().position(|x| x.at == other.at).unwrap();
            for q in 0..4 {
                let marked = d.land[4 * i + q] & 0x40 != 0;
                assert_eq!(marked, other.at == c.at && q != 3, "{:?} {q}", other.at);
            }
        }
        assert!(d.hidden[watched(w.animated[0])]);
        assert!(!d.hidden[watched(w.animated[1])], "BSXFlags 0x23 stays");
        assert!(d.hidden[watched(w.water_node)]);
        assert!(d.hidden[watched(w.lights[0])] && d.hidden[watched(w.lights[1])]);
        assert!(
            !d.hidden[watched(w.extra_node)],
            "a full update keeps the extra node"
        );
        for n in s.scene_nodes {
            assert!(d.hidden[watched(n)]);
        }
        assert!(!d.hidden[watched(w.planes[5])], "(0, 1)'s plane shown");
        assert!(d.hidden[watched(w.planes[4])]);
    }
    seen_cells.sort_unstable();
    let mut want = around.clone();
    want.sort_unstable();
    assert_eq!(seen_cells, want);

    // Everything back.
    let after = flags_of(&e, &watched_objects);
    for (i, &o) in watched_objects.iter().enumerate() {
        if w.planes.contains(&o) {
            assert_eq!(after[i] & 1, 1, "every water plane hidden");
        } else {
            assert_eq!(after[i], before[i], "object {i}'s flags back");
        }
    }
    let land_after: Vec<u8> = LAND.with(|x| {
        x.borrow()
            .iter()
            .map(|&l| e.mem.u8(l + LAND_FLAGS))
            .collect()
    });
    assert_eq!(land_after, land_before, "the land quads unmarked");
    assert_eq!(e.mem.u32(w.sun + LIGHT_LIGHT), TES_LIGHT_WORD);
    assert_eq!(e.mem.u32(w.scene + SCENE_FOG), GAME_FOG);
    assert_eq!(e.mem.u8(UNDERWATER_FOG), 0);
    assert_eq!(e.mem.u8(LIGHTING_PASSES), PASSES_BEFORE);
    assert_eq!(e.mem.u8(DRAW_FLAG), 1);
    let lock: Vec<(u32, Vec<u32>)> = calls()
        .into_iter()
        .filter(|c| c.0 == LOCK_ENTER || c.0 == LOCK_LEAVE)
        .collect();
    assert_eq!(
        lock,
        [
            (LOCK_ENTER, vec![SCENE_LIGHTS_LOCK, 0]),
            (LOCK_LEAVE, vec![SCENE_LIGHTS_LOCK])
        ]
    );
    // The wet plane at the water's height; the planes at the map's corner (cell less one across,
    // two up), updated.
    assert_eq!(e.mem.f32(w.planes[5] + LOCAL_TRANSLATE + 8), w.water_z);
    assert_eq!(
        [
            e.mem.f32(s.water_planes + LOCAL_TRANSLATE),
            e.mem.f32(s.water_planes + LOCAL_TRANSLATE + 4)
        ],
        [-4096.0, 8192.0]
    );
    assert_eq!(calls_to(UPDATE_DOWNWARD_PASS), [vec![s.water_planes]]);
    // The purge: 69 kept, the oldest 27 (JIP's first on, the game's own texture passed over)
    // deleted; JIP's first then the next.
    assert_eq!(s.rendered_exteriors.len(), 42);
    let mm: Vec<u32> = kept
        .iter()
        .copied()
        .filter(|&t| e.mem.u32(t + TEXTURE_CELL) != 0)
        .collect();
    let deleted: Vec<u32> = calls_to(RENDERED_TEXTURE_DELETE)
        .into_iter()
        .map(|w| w[0])
        .collect();
    assert_eq!(deleted, mm[..27]);
    assert_eq!(s.mm_first_texture, mm[27]);
    for c in around.iter().map(|&(x, y)| w.cell(x, y)) {
        let id = e.mem.u32(c.cell + CELL_ID);
        let t = s.rendered_exteriors[&id].texture;
        assert_eq!(
            e.mem.u32(t + TEXTURE_CELL),
            id,
            "the new map kept, its cell id set"
        );
        assert_ne!(e.mem.u32(t), FREED);
    }
    // Each slot shows its cell's map.
    for (i, &(x, y)) in around.iter().enumerate() {
        let id = e.mem.u32(w.cell(x, y).cell + CELL_ID);
        assert_eq!(
            e.mem.u32(s.shader_props[i] + SHADER_TEXTURE),
            s.rendered_exteriors[&id].texture
        );
    }
}

/// Nit 2: the exterior's kept maps. A first draw with the grid on the player's cell keeps each map
/// marked with its slot's `kSelectUpdate` mask, JIP's first the first one made. The player moves a
/// cell east (a full update, the grid not yet moved): the six kept cells' maps move to the list's
/// end without a draw, the three new ones are drawn unmarked. The grid's centre then follows: only
/// the cells whose mask does not cover their new slot's are drawn again ((1, -1), (1, 0), (1, 1)
/// and the three new ones), marked with it; (0, -1), (0, 0) and (0, 1) are kept as they are, and
/// outside a full update the sky's extra node is hidden for the draws.
#[test]
fn the_exterior_maps_are_kept_by_the_select_update_mask() {
    let mut e = setup();
    let (w, mut s) = exterior_world(&mut e);
    place_player(&mut e, &w, (0, 0), (0, 0));
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    let id = |e: &Engine, at: (i32, i32)| e.mem.u32(w.cell(at.0, at.1).cell + CELL_ID);
    let first_made = {
        let c = drawn_cell(&w, &drawn()[0]);
        s.rendered_exteriors[&id(&e, c.at)].texture
    };
    assert_eq!(s.mm_first_texture, first_made);
    for (i, &at) in nine(0, 0).iter().enumerate() {
        assert_eq!(
            s.rendered_exteriors[&id(&e, at)].regen_flags,
            SELECT_UPDATE[i],
            "{at:?}"
        );
    }
    let textures_at_first: HashMap<u32, u32> = s
        .rendered_exteriors
        .iter()
        .map(|(&k, v)| (k, v.texture))
        .collect();

    // One of the game's own textures made since, at the list's end.
    let game = alloc(0x40);
    e.mem.set_u32(game, VTBL_RENDERED_TEXTURE);
    let tail = e.mem.u32(TEXTURE_LIST_TAIL);
    e.mem.set_u32(tail + TEXTURE_NEXT, game);
    e.mem.set_u32(game + TEXTURE_PREV, tail);
    e.mem.set_u32(TEXTURE_LIST_TAIL, game);

    // A cell east, the grid where it was.
    place_player(&mut e, &w, (1, 0), (0, 0));
    e.mem.set_f32(w.player + REF_POSITION, 6144.0);
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    // The list: the cells left behind, the game's texture, the six kept in their new slots'
    // order, then the three new maps.
    let mut order = Vec::new();
    let mut t = e.mem.u32(TEXTURE_LIST_HEAD);
    while t != 0 {
        order.push(t);
        t = e.mem.u32(t + TEXTURE_NEXT);
    }
    let by_cell = |e: &Engine, at: (i32, i32)| s.rendered_exteriors[&id(e, at)].texture;
    let mut want: Vec<u32> = [(-1, -1), (-1, 0), (-1, 1)]
        .iter()
        .map(|&at| by_cell(&e, at))
        .collect();
    want.push(game);
    for at in [
        (0, -1),
        (0, 0),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
        (2, -1),
        (2, 0),
        (2, 1),
    ] {
        want.push(by_cell(&e, at));
    }
    assert_eq!(order, want, "the kept maps moved to the end without a draw");
    let made: Vec<(i32, i32)> = drawn().iter().map(|d| drawn_cell(&w, d).at).collect();
    assert_eq!(made, [(2, -1), (2, 0), (2, 1)], "only the new cells drawn");
    let tail: Vec<u32> = {
        let mut out = Vec::new();
        let mut t = e.mem.u32(TEXTURE_LIST_TAIL);
        while t != 0 && out.len() < 9 {
            out.push(t);
            t = e.mem.u32(t + TEXTURE_PREV);
        }
        out
    };
    for at in [(0, -1), (0, 0), (0, 1), (1, -1), (1, 0), (1, 1)] {
        let entry = s.rendered_exteriors[&id(&e, at)];
        assert_eq!(entry.texture, textures_at_first[&id(&e, at)], "{at:?} kept");
        assert!(tail.contains(&entry.texture), "{at:?} moved to the end");
    }
    for at in [(2, -1), (2, 0), (2, 1)] {
        assert_eq!(s.rendered_exteriors[&id(&e, at)].regen_flags, 0, "{at:?}");
    }
    assert!(calls_to(RENDERED_TEXTURE_DELETE).is_empty());

    // The grid follows the player: a selective redraw.
    place_player(&mut e, &w, (1, 0), (1, 0));
    let before: HashMap<u32, RenderedEntry> = s.rendered_exteriors.clone();
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    let mut redrawn: Vec<(i32, i32)> = drawn().iter().map(|d| drawn_cell(&w, d).at).collect();
    redrawn.sort_unstable();
    assert_eq!(
        redrawn,
        [(1, -1), (1, 0), (1, 1), (2, -1), (2, 0), (2, 1)],
        "the masks of (0, *) cover their new slots"
    );
    for (i, &at) in nine(1, 0).iter().enumerate() {
        let entry = s.rendered_exteriors[&id(&e, at)];
        let was = before[&id(&e, at)];
        if at.0 == 0 {
            assert_eq!(entry, was, "{at:?} kept as it was");
        } else {
            assert_eq!(entry.regen_flags, was.regen_flags | SELECT_UPDATE[i]);
            assert_ne!(entry.texture, was.texture, "{at:?} drawn again");
            assert_eq!(e.mem.u32(was.texture), FREED, "the old map let go");
        }
        assert_eq!(e.mem.u32(s.shader_props[i] + SHADER_TEXTURE), entry.texture);
    }
    for d in drawn() {
        assert!(
            d.hidden[watched(w.extra_node)],
            "outside a full update the extra node is hidden"
        );
    }
    assert_eq!(e.mem.u32(w.extra_node + FLAGS) & 1, 0, "and shown after");
    // The same place again: nothing drawn, no lock.
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    assert!(drawn().is_empty());
    assert!(calls_to(LOCK_ENTER).is_empty());
}

/// An interior cell (flag 1, no world) with nodes 0 to 4: the first and fifth hidden by JIP, the
/// fourth holding a `BlackPlane01` shape (hidden) and another (not); its north rotation `angle`
/// when given. The world and the player inside at `pos`.
struct Interior {
    cell: u32,
    hidden: [u32; 3],
    kept: [u32; 2],
}

fn interior_world(e: &mut Engine, pos: [f32; 2], angle: Option<f32>) -> (Interior, MiniMapState) {
    let cell = alloc(0xd0);
    e.mem.set_u8(cell + CELL_FLAGS, 1);
    let (n0, n1, n4) = (alloc(0x40), alloc(0x40), alloc(0x40));
    let (plane, other) = (alloc(0x40), alloc(0x40));
    let (_, _, _, _, s) = common(e, cell, pos);
    e.mem.set_u32(plane + NAME, s.black_plane);
    e.mem.set_u32(other + NAME, 0x0dea_0001);
    let n3 = node_with(e, &[other, plane]);
    let root = node_with(e, &[n0, n1, 0, n3, n4]);
    let render = alloc(0x80);
    e.mem.set_u32(render, root);
    e.mem.set_u32(cell + CELL_RENDER_DATA, render);
    if let Some(a) = angle {
        let extra = alloc(0x10);
        put_extra(e, cell + CELL_EXTRA, extra, 0x43, a.to_bits());
    }
    let i = Interior {
        cell,
        hidden: [n0, n4, plane],
        kept: [n1, other],
    };
    watch(&i.hidden);
    watch(&i.kept);
    watch(&s.scene_nodes);
    (i, s)
}

/// The nine sections around a packed one, in the slots' order.
fn sections(coord: u32) -> Vec<(i32, i32)> {
    let x = i32::from((coord >> 16) as u16 as i16);
    let y = i32::from(coord as u16 as i16);
    nine(x, y)
}

/// Medium 3 and nit 2: an interior update with no north rotation. Each of the nine sections drawn
/// once, the camera over its corner plus one cell at 32768 (world scale 0), with JIP's hidden
/// objects (the cell's first and fifth nodes, its `BlackPlane01`) and the scene's fourth node
/// hidden, the others not, 0x34 lighting passes, the no-water render mode, the draw byte 0, the
/// sun light and fog the game's (no swap, no lock in an interior). After: all shown again, the
/// passes and the draw byte back, each slot showing its section's map. The same place again draws
/// nothing.
#[test]
fn an_interior_update_draws_its_sections_and_puts_the_scene_back() {
    let mut e = setup();
    let (i, mut s) = interior_world(&mut e, [5000.0, 7000.0], None);
    s.in_exterior = false;
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    // (5000 - 2048, 7000 - 2048) >> 12: section (0, 1).
    let want = sections(pack(0, 1));
    let draws = drawn();
    let places: Vec<[f32; 4]> = draws.iter().map(|d| d.camera).collect();
    let expect: Vec<[f32; 4]> = want
        .iter()
        .map(|&(x, y)| [((x + 1) << 12) as f32, ((y + 1) << 12) as f32, 32768.0, 0.0])
        .collect();
    assert_eq!(places, expect);
    for d in &draws {
        for &o in &i.hidden {
            assert!(d.hidden[watched(o)]);
        }
        for &o in &i.kept {
            assert!(!d.hidden[watched(o)]);
        }
        for (n, &node) in s.scene_nodes.iter().enumerate() {
            assert_eq!(d.hidden[watched(node)], n == 3, "scene node {n}");
        }
        assert_eq!(
            (d.passes, d.render_mode, d.draw_flag),
            (0x34, RENDER_NO_WATER, 0)
        );
        assert_eq!((d.sun_light, d.fog), (TES_LIGHT_WORD, GAME_FOG));
    }
    for &o in i.hidden.iter().chain(&i.kept).chain(&s.scene_nodes) {
        assert_eq!(e.mem.u32(o + FLAGS) & 1, 0, "shown again");
    }
    assert_eq!(e.mem.u8(LIGHTING_PASSES), PASSES_BEFORE);
    assert_eq!(e.mem.u8(DRAW_FLAG), 1);
    assert!(calls_to(LOCK_ENTER).is_empty() && calls_to(LOCK_LEAVE).is_empty());
    assert_eq!(s.last_interior, i.cell);
    for (n, &(x, y)) in want.iter().enumerate() {
        let t = s.rendered_interior[&pack(x, y)].texture;
        assert_ne!(t, 0);
        assert_eq!(e.mem.u32(s.shader_props[n] + SHADER_TEXTURE), t);
    }
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    assert!(drawn().is_empty(), "kept by section");
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-3 * b.abs().max(1.0)
}

/// Nit 1 and nit 2: an interior with a north rotation of a quarter turn. The camera's rotation
/// lanes are JIP's (sin, cos, cos, -sin) of the turn's negative, the player's place and each
/// section's camera turned by them (x' = -y, y' = x here).
#[test]
fn an_interior_with_a_north_rotation_turns_the_camera() {
    let mut e = setup();
    let (_, mut s) = interior_world(&mut e, [5000.0, 7000.0], Some(std::f32::consts::FRAC_PI_2));
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    let lanes = [0x6c, 0x70, 0x78, 0x7c].map(|at| e.mem.f32(s.camera + at));
    for (got, want) in lanes.iter().zip([-1.0, 0.0, 0.0, 1.0]) {
        assert!(near(*got, want), "{lanes:?}");
    }
    // The player turned: (7000, -5000), less 2048: section (1, -2).
    let want = sections(pack(1, -2));
    assert_eq!(
        s.packed_coords.to_vec(),
        want.iter().map(|&(x, y)| pack(x, y)).collect::<Vec<_>>()
    );
    for (d, &(x, y)) in drawn().iter().zip(&want) {
        let (px, py) = (((x + 1) << 12) as f32, ((y + 1) << 12) as f32);
        assert!(
            near(d.camera[0], -py) && near(d.camera[1], px),
            "{:?} for ({x}, {y})",
            d.camera
        );
        assert_eq!(d.camera[2], 32768.0);
    }
}

/// Nit 1: the new-cell path. Into an exterior from an interior's state: the camera's lanes (0, 1,
/// 1, 0), the zoom 0.25 less with the shapes rescaled (`(0.5 + zoom) * scale`); into an interior
/// with a north rotation: the lanes turned, the zoom 0.25 more; out again: the lanes (0, 1, 1, 0)
/// and JIP's north rotation 0. Each new cell is a full update (its maps drawn or shown).
#[test]
fn a_new_cell_sets_the_camera_and_steps_the_zoom() {
    let mut e = setup();
    let (w, mut s) = exterior_world(&mut e);
    place_player(&mut e, &w, (0, 0), (0, 0));
    // An interior with a quarter turn, built beside the exterior.
    let inside = alloc(0xd0);
    e.mem.set_u8(inside + CELL_FLAGS, 1);
    let third = node_with(&mut e, &[]);
    let root = node_with(&mut e, &[alloc(0x40), 0, 0, third, 0]);
    let render = alloc(0x80);
    e.mem.set_u32(render, root);
    e.mem.set_u32(inside + CELL_RENDER_DATA, render);
    put_extra(
        &mut e,
        inside + CELL_EXTRA,
        alloc(0x10),
        0x43,
        std::f32::consts::FRAC_PI_2.to_bits(),
    );
    s.in_exterior = false;
    s.zoom_current = 1.0;
    let lanes =
        |e: &Engine, s: &MiniMapState| [0x6c, 0x70, 0x78, 0x7c].map(|at| e.mem.f32(s.camera + at));
    let scale = |e: &Engine, s: &MiniMapState| {
        s.shapes
            .map(|sh| e.mem.f32(sh + super::super::jip_fn_minimap::LOCAL_SCALE))
    };

    forget_calls();
    update(&mut e, &mut s, 0, 8);
    assert_eq!(lanes(&e, &s), [0.0, 1.0, 1.0, 0.0]);
    assert!(s.in_exterior);
    assert_eq!(s.zoom_current, 0.75);
    assert_eq!(scale(&e, &s), [(0.5 + 0.75) * 2.0; 9]);
    assert_eq!(e.mem.f32(s.local_map_zoom + VALUE_NUM), 0.75);
    assert_eq!(drawn().len(), 9);

    e.mem.set_u32(w.player + PLAYER_CELL, inside);
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    let l = lanes(&e, &s);
    for (got, want) in l.iter().zip([-1.0, 0.0, 0.0, 1.0]) {
        assert!(near(*got, want), "{l:?}");
    }
    assert!(!s.in_exterior);
    assert_eq!(s.zoom_current, 1.0);
    assert_eq!(scale(&e, &s), [(0.5 + 1.0) * 2.0; 9]);
    assert_eq!(drawn().len(), 9, "the interior's sections drawn");

    place_player(&mut e, &w, (0, 0), (0, 0));
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    assert_eq!(lanes(&e, &s), [0.0, 1.0, 1.0, 0.0]);
    assert_eq!(mini_map(&mut e).north_rotation.load(Ordering::Acquire), 0);
    assert_eq!(s.zoom_current, 0.75);
    assert_eq!(
        calls_to(LOCK_ENTER).len(),
        1,
        "a full update: the kept maps shown again"
    );
}

/// Nit 1: the map's mode. From the world map (mode 1) back to the local map: the mode value 0,
/// and a full update though the cell is the same (the exterior's drawing runs, under its lock).
/// The same again: no change, nothing drawn.
#[test]
fn a_mode_change_forces_a_full_update() {
    let mut e = setup();
    let (w, mut s) = exterior_world(&mut e);
    place_player(&mut e, &w, (0, 0), (0, 0));
    update(&mut e, &mut s, 0, 8);
    s.current_map_mode = 1;
    e.mem.set_f32(s.mode + VALUE_NUM, 1.0);
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    assert_eq!(s.current_map_mode, 0);
    assert_eq!(e.mem.f32(s.mode + VALUE_NUM), 0.0);
    assert_eq!(calls_to(LOCK_ENTER).len(), 1, "forced");
    forget_calls();
    update(&mut e, &mut s, 0, 8);
    assert!(calls_to(LOCK_ENTER).is_empty());
}

/// Nit 1: the fog of war turned on (flag 0x10): the nine shaders' vertex colours on (+0x92), the
/// alpha levels the first shader's alpha times 0, 1/4, 1/2, 3/4, 1, and a full update (each slot's
/// map set again). Turned off: the vertex colours off, the levels kept.
#[test]
fn the_fog_toggle_sets_the_shaders_and_alpha_levels() {
    let mut e = setup();
    let (_, mut s) = interior_world(&mut e, [5000.0, 7000.0], None);
    update(&mut e, &mut s, 0, 8);
    let a = 0.8f32;
    for (n, &prop) in s.shader_props.iter().enumerate() {
        e.mem
            .set_f32(prop + SHADER_ALPHA, if n == 0 { a } else { 0.1 });
        e.mem.set_u32(prop + SHADER_TEXTURE, 0);
    }
    s.vertex_alpha = [0.0; 5];
    update(&mut e, &mut s, 0, 8 | 0x10);
    for &prop in &s.shader_props {
        assert_eq!(e.mem.u8(prop + SHADER_VERTEX_COLOURS), 1);
        assert_ne!(e.mem.u32(prop + SHADER_TEXTURE), 0, "a full update");
    }
    assert_eq!(s.vertex_alpha, [0.0, a / 4.0, a / 2.0, a * 0.75, a]);
    update(&mut e, &mut s, 0, 8);
    for &prop in &s.shader_props {
        assert_eq!(e.mem.u8(prop + SHADER_VERTEX_COLOURS), 0);
    }
    assert_eq!(s.vertex_alpha, [0.0, a / 4.0, a / 2.0, a * 0.75, a]);
}
