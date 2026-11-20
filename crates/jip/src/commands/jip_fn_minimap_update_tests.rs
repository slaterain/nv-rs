//! `UpdateMiniMap`'s parts on stand-in tiles, references and textures, every expectation from
//! JIP's `Cmd_UpdateMiniMap_Execute` and the helpers it calls (functions_jip/jip_fn_minimap.h).
//! The game functions it calls are stand-ins at their exe addresses and record the words they
//! get.

use super::super::jip_fn_minimap::MarkerInfo;
use super::*;
use crate::commands::test_support::{calls, calls_to, engine, extra_stand_ins, put_extra, CALLS};
use std::cell::Cell;
use std::collections::HashMap;

const AREA: u32 = 0x0600_0000;
const VALUE_REFRESH: u32 = 0x00a0_9410;
/// The game's map marker icon texts (by type) and the undiscovered one: the test's own texts.
const ICON_TEXT: u32 = 0x011a_0800;

thread_local! {
    static NEXT: Cell<u32> = const { Cell::new(AREA + 0x1000) };
}

pub(super) fn alloc(size: u32) -> u32 {
    NEXT.with(|n| {
        let at = n.get();
        n.set(at + size.next_multiple_of(0x10));
        at
    })
}

pub(super) fn record(f: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
}

pub(super) fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// A tile with a value for each trait id (sorted, as the game keeps them; each value's tile at +4).
fn tile_with(e: &mut Engine, ids: &[u32]) -> u32 {
    let tile = alloc(0x50);
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    let data = alloc(4 * ids.len() as u32);
    for (i, &id) in ids.iter().enumerate() {
        let v = alloc(0x14);
        e.mem.set_u32(v, id);
        e.mem.set_u32(v + 4, tile);
        e.mem.set_u32(data + 4 * i as u32, v);
    }
    e.mem.set_u32(tile + 0x14, data);
    e.mem.set_u32(tile + 0x18, ids.len() as u32);
    tile
}

/// The traits a template's tile has: visible, x, y, the visited flag, depth, colour.
const TEMPLATE_TRAITS: [u32; 8] = [
    TRAIT_VISIBLE,
    TRAIT_USER0,
    TRAIT_USER1,
    TRAIT_USER2,
    TRAIT_DEPTH,
    TRAIT_RED,
    TRAIT_GREEN,
    TRAIT_BLUE,
];

/// A place on the local map, as JIP works it out: `(pos - nw) * (1 / 12288, 1 / -12288)`.
fn local(pos: [f32; 2], nw: [f32; 2]) -> [f32; 2] {
    let k = local_map_pos_mults();
    [(pos[0] - nw[0]) * k[0], (pos[1] - nw[1]) * k[1]]
}

fn num(e: &Engine, value: u32) -> f32 {
    e.mem.f32(value + VALUE_NUM)
}

fn trait_num(e: &Engine, tile: u32, id: u32) -> f32 {
    num(e, value_of(e, tile, id))
}

/// A value's text (its string at +0xC).
fn text(e: &Engine, value: u32) -> Vec<u8> {
    e.mem.cstr(e.mem.u32(value + 0xc)).to_vec()
}

/// The engine with the stand-ins: a value's refresh, `AddTileFromTemplate` (a new tile with
/// [`TEMPLATE_TRAITS`], the template's name recorded), `Tile::AddValue`, the texture's deleting
/// destructor and the two critical-section calls (recorded); the pages the update reads mapped.
pub(super) fn setup() -> Engine {
    let mut e = engine();
    e.map(AREA, 0x40_0000);
    for page in [
        0x011a_0000,
        0x0107_5000,
        0x011c_9000,
        0x011f_4000,
        0x011d_e000,
    ] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.register(VALUE_REFRESH, |_, w| {
        record(VALUE_REFRESH, w);
        eax(0)
    });
    e.register(ADD_TILE_FROM_TEMPLATE, |e, w| {
        let name = e.mem.cstr(w[2]).to_vec();
        CALLS.with(|c| {
            c.borrow_mut()
                .push((ADD_TILE_FROM_TEMPLATE, vec![w[1], name.len() as u32]))
        });
        TEMPLATES.with(|t| t.borrow_mut().push(name));
        eax(tile_with(e, &TEMPLATE_TRAITS))
    });
    e.register(TILE_ADD_VALUE, |e, w| {
        let v = alloc(0x14);
        e.mem.set_u32(v, w[1]);
        e.mem.set_u32(v + 4, w[0]);
        eax(v)
    });
    e.register(RENDERED_TEXTURE_DELETE, |_, w| {
        record(RENDERED_TEXTURE_DELETE, w);
        eax(0)
    });
    e.register(ENTER_CRITICAL_SECTION, |_, w| {
        record(ENTER_CRITICAL_SECTION, w);
        eax(0)
    });
    e.register(LEAVE_CRITICAL_SECTION, |_, w| {
        record(LEAVE_CRITICAL_SECTION, w);
        eax(0)
    });
    TEMPLATES.with(|t| t.borrow_mut().clear());
    e
}

thread_local! {
    static TEMPLATES: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn templates() -> Vec<Vec<u8>> {
    TEMPLATES.with(|t| t.borrow().clone())
}

// ---- small pieces ----

/// `iceil` rounds up then truncates; a node's `BSXFlags` is its last such extra; a marker's icon
/// is the game's for its type once discovered (flag 2), the undiscovered one before; a load forgets
/// the player's cell, world, root world and last interior.
#[test]
fn the_small_pieces() {
    let mut e = setup();
    assert_eq!(iceil(1.2), 2);
    assert_eq!(iceil(-1.2), -1);
    assert_eq!(iceil(3.0), 3);
    assert_eq!(iceil(f32::NAN), i32::MIN);

    let node = alloc(0x20);
    let list = alloc(12);
    let (other, first, last) = (alloc(0x10), alloc(0x10), alloc(0x10));
    e.mem.set_u32(other, 0x0101_0000);
    for (x, v) in [(first, 0x21), (last, 0x23)] {
        e.mem.set_u32(x, VTBL_BSX_FLAGS);
        e.mem.set_u32(x + 0xc, v);
    }
    for (i, x) in [first, last, other].iter().enumerate() {
        e.mem.set_u32(list + 4 * i as u32, *x);
    }
    e.mem.set_u32(node + EXTRA_LIST, list);
    e.mem.set_u16(node + EXTRA_COUNT, 3);
    assert_eq!(bsx_flags(&e, node), 0x23, "searched from the last");
    e.mem.set_u16(node + EXTRA_COUNT, 0);
    assert_eq!(bsx_flags(&e, node), 0);

    let data = alloc(0x10);
    e.mem.set_u32(MARKER_ICONS + 4 * 3, ICON_TEXT);
    e.mem.set_u8(data + MARKER_TYPE, 3);
    e.mem.set_u8(data + MARKER_FLAGS, 1);
    assert_eq!(marker_icon_path(&e, data), UNDISCOVERED_ICON);
    e.mem.set_u8(data + MARKER_FLAGS, 3);
    assert_eq!(marker_icon_path(&e, data), ICON_TEXT);

    {
        let mut s = state(&mut e);
        s.pc_curr_cell = 1;
        s.pc_curr_world = 2;
        s.pc_root_world = 3;
        s.last_interior = 4;
        s.current_map_mode = 1;
    }
    load_game(&mut e);
    let s = state(&mut e);
    assert_eq!(
        (
            s.pc_curr_cell,
            s.pc_curr_world,
            s.pc_root_world,
            s.last_interior
        ),
        (0, 0, 0, 0)
    );
    assert_eq!(s.current_map_mode, 1, "the rest kept");
}

// ---- the zoom ----

/// A state with the zoom values and nine shapes (their scale at +0x64).
fn zoom_state(e: &mut Engine, zoom: f32, in_exterior: bool, current: f32) -> MiniMapState {
    let mut s = MiniMapState {
        local_map_zoom: alloc(0x14),
        scale: alloc(0x14),
        zoom_current: current,
        in_exterior,
        ..MiniMapState::default()
    };
    e.mem.set_f32(s.local_map_zoom + VALUE_NUM, zoom);
    e.mem.set_f32(s.scale + VALUE_NUM, 2.0);
    for shape in s.shapes.iter_mut() {
        *shape = alloc(0xc0);
    }
    s
}

/// Type 3 (the zoom only): a new zoom value is taken, plus 0.25 in an interior; the shapes scaled
/// by `(0.5 + zoom) * scale` and the value given the zoom; with flags 0x100 nothing else (the
/// player is not read).
#[test]
fn the_zoom_alone() {
    let mut e = setup();
    let mut s = zoom_state(&mut e, 0.5, false, 1.0);
    update(&mut e, &mut s, 3, 0x100);
    assert_eq!(s.zoom_current, 0.75);
    for shape in s.shapes {
        assert_eq!(e.mem.f32(shape + 0x64), (0.5 + 0.75) * 2.0);
    }
    assert_eq!(num(&e, s.local_map_zoom), 0.75);
    // The value now holds the zoom JIP keeps: the same bits, no change.
    update(&mut e, &mut s, 3, 0x100);
    assert_eq!(s.zoom_current, 0.75);

    let mut s = zoom_state(&mut e, 0.5, true, 1.0);
    update(&mut e, &mut s, 3, 0x100);
    assert_eq!(s.zoom_current, 0.5, "no interior step outside");
    assert_eq!(e.mem.f32(s.shapes[0] + 0x64), (0.5 + 0.5) * 2.0);
}

// ---- the world map's markers ----

/// A map marker reference: its data's flags (1 shown, 2 discovered) and type; its form flags.
fn marker(e: &mut Engine, flags: u8, kind: u8, form_flags: u32, pos: [f32; 2]) -> MarkerInfo {
    let refr = alloc(0x60);
    e.mem.set_u32(refr + FORM_FLAGS, form_flags);
    let data = alloc(0x10);
    e.mem.set_u8(data + MARKER_FLAGS, flags);
    e.mem.set_u8(data + MARKER_TYPE, kind);
    MarkerInfo { refr, data, pos }
}

/// A made marker tile (visible, x, y and file name values), in use or not.
fn made_marker_tile(e: &mut Engine, in_use: bool) -> MarkerTile {
    let tile = tile_with(e, &[TRAIT_VISIBLE, TRAIT_USER0, TRAIT_USER1]);
    let filename = alloc(0x14);
    let t = MarkerTile {
        visible: value_of(e, tile, TRAIT_VISIBLE),
        x: value_of(e, tile, TRAIT_USER0),
        y: value_of(e, tile, TRAIT_USER1),
        filename,
        in_use,
    };
    e.mem
        .set_f32(t.visible + VALUE_NUM, if in_use { 1.0 } else { 0.0 });
    t
}

/// The cells within `iceil(mkRange / zoom)` of the player's cell each way get their shown (flag 1),
/// enabled (no 0x800) markers drawn once: the made tiles not in use first, in order, then new ones
/// from `MiniMapWorldMarkerTemplate` (depth `(count & 0xF) + 18`), each placed and given its icon.
/// A cell with markers is current even when none is drawn; cells out of range let go of theirs.
#[test]
fn the_world_markers_in_range() {
    let mut e = setup();
    e.mem.set_u32(MARKER_ICONS + 4 * 3, ICON_TEXT);
    e.mem.write(ICON_TEXT, b"icons\\town.dds\0");
    e.mem.write(UNDISCOVERED_ICON, b"icons\\unknown.dds\0");
    let mut s = MiniMapState {
        world_map_zoom: alloc(0x14),
        map_markers_rect: alloc(0x50),
        ..MiniMapState::default()
    };
    e.mem.set_f32(s.world_map_zoom + VALUE_NUM, 1.0);
    s.root_dims.mk_range = 2;
    let here = pack(10, 20);
    let (near_x, corner, edge, out) = (pack(12, 20), pack(8, 18), pack(10, 22), pack(13, 20));
    let town = marker(&mut e, 3, 3, 0, [0.25, 0.5]);
    let hidden = marker(&mut e, 2, 3, 0, [0.0, 0.0]);
    let unknown = marker(&mut e, 1, 3, 0, [0.75, 0.125]);
    let disabled = marker(&mut e, 3, 3, 0x800, [0.0, 0.0]);
    let beyond = marker(&mut e, 3, 3, 0, [0.0, 0.0]);
    let root = 0x0dea_0000;
    let mut cells = HashMap::new();
    cells.insert(near_x, vec![hidden, unknown]);
    cells.insert(corner, vec![disabled]);
    cells.insert(edge, vec![town]);
    cells.insert(out, vec![beyond]);
    s.world_markers.insert(root, cells);
    s.curr_world_markers = Some(root);
    // Tile 0 shows a far cell's marker; tile 1 is free.
    let far = pack(50, 50);
    s.marker_tiles = vec![
        made_marker_tile(&mut e, true),
        made_marker_tile(&mut e, false),
    ];
    s.rendered_markers.insert(far, vec![0]);
    let hud = 0x0dea_1000;

    world_markers_update(&mut e, &mut s, hud, here);

    let mut current: Vec<u32> = s.curr_marker_cells.iter().copied().collect();
    current.sort_unstable();
    let mut want = vec![near_x, corner, edge];
    want.sort_unstable();
    assert_eq!(current, want, "(13, 20) is out of range");
    let mut rendered: Vec<u32> = s.rendered_markers.keys().copied().collect();
    rendered.sort_unstable();
    assert_eq!(rendered, want, "the far cell let go");
    assert_eq!(s.rendered_markers[&corner], Vec::<usize>::new());
    assert_eq!(s.rendered_markers[&edge], [1], "the free tile first");
    assert_eq!(s.rendered_markers[&near_x], [2], "then a new one");
    assert_eq!(templates(), [b"MiniMapWorldMarkerTemplate".to_vec()]);
    assert_eq!(
        calls_to(ADD_TILE_FROM_TEMPLATE),
        [vec![s.map_markers_rect, 26]]
    );
    let t = s.marker_tiles[1];
    assert!(t.in_use);
    assert_eq!(
        (num(&e, t.visible), num(&e, t.x), num(&e, t.y)),
        (1.0, 0.25, 0.5)
    );
    assert_eq!(text(&e, t.filename), b"icons\\town.dds");
    let t = s.marker_tiles[2];
    assert!(t.in_use);
    assert_eq!((num(&e, t.x), num(&e, t.y)), (0.75, 0.125));
    assert_eq!(text(&e, t.filename), b"icons\\unknown.dds");
    let made = e.mem.u32(t.visible + 4);
    assert_eq!(trait_num(&e, made, TRAIT_DEPTH), (3 & 0xf) as f32 + 18.0);
    // The far cell's tile: not in use, hidden.
    assert!(!s.marker_tiles[0].in_use);
    assert_eq!(num(&e, s.marker_tiles[0].visible), 0.0);

    // Drawn once: the next update in the same place adds nothing.
    world_markers_update(&mut e, &mut s, hud, here);
    assert_eq!(s.marker_tiles.len(), 3);
    assert_eq!(s.rendered_markers[&edge], [1]);
}

/// A zoom of 0.5 doubles the range: (13, 20) is then in it.
#[test]
fn the_world_markers_range_follows_the_zoom() {
    let mut e = setup();
    let mut s = MiniMapState {
        world_map_zoom: alloc(0x14),
        map_markers_rect: alloc(0x50),
        ..MiniMapState::default()
    };
    e.mem.set_f32(s.world_map_zoom + VALUE_NUM, 0.5);
    s.root_dims.mk_range = 2;
    let (out, past) = (pack(13, 20), pack(15, 20));
    let mut cells = HashMap::new();
    cells.insert(out, vec![marker(&mut e, 1, 0, 0, [0.0, 0.0])]);
    cells.insert(past, vec![marker(&mut e, 1, 0, 0, [0.0, 0.0])]);
    s.world_markers.insert(1, cells);
    s.curr_world_markers = Some(1);
    world_markers_update(&mut e, &mut s, 0, pack(10, 20));
    let current: Vec<u32> = s.curr_marker_cells.iter().copied().collect();
    assert_eq!(current, [out], "range 4: x 6 to 14");
}

// ---- the door markers ----

/// A door at `pos` leading to `linked`.
fn door(e: &mut Engine, pos: [f32; 2], linked: u32) -> DoorRef {
    let d = alloc(0x60);
    e.mem.set_f32(d + REF_POSITION, pos[0]);
    e.mem.set_f32(d + REF_POSITION + 4, pos[1]);
    DoorRef {
        door: d,
        linked_cell: linked,
    }
}

fn made_door_tile(e: &mut Engine) -> DoorMarkerTile {
    let tile = tile_with(e, &[TRAIT_VISIBLE, TRAIT_USER0, TRAIT_USER1, TRAIT_USER2]);
    let t = DoorMarkerTile {
        visible: value_of(e, tile, TRAIT_VISIBLE),
        x: value_of(e, tile, TRAIT_USER0),
        y: value_of(e, tile, TRAIT_USER1),
        visited: value_of(e, tile, TRAIT_USER2),
        vtx_alpha: 0,
        in_use: true,
    };
    e.mem.set_f32(t.visible + VALUE_NUM, 1.0);
    t
}

/// On a full update (no fog), each door on the map gets a tile in order (made ones first, then
/// `MiniMapDoorMarkerTemplate`), placed at `(pos - nw) * (1/12288, -1/12288)`, visited when it
/// leads nowhere, to the player's cell, or to a cell with a detach time; shown. A door off the map
/// takes none; the tiles left over are not in use and hidden. With flag 2 off nothing is done but
/// the rect hidden when the flag changed.
#[test]
fn the_door_markers() {
    let mut e = setup();
    extra_stand_ins(&mut e);
    let parent = alloc(0x40);
    let (unvisited, detached) = (alloc(0x40), alloc(0x40));
    let detach = alloc(0x10);
    put_extra(&mut e, detached + CELL_EXTRA, detach, EXTRA_DETACH_TIME, 0);
    let rect = tile_with(&mut e, &[TRAIT_VISIBLE]);
    let mut s = MiniMapState {
        door_markers_rect: rect,
        ..MiniMapState::default()
    };
    s.door_marker_tiles = vec![
        made_door_tile(&mut e),
        made_door_tile(&mut e),
        made_door_tile(&mut e),
    ];
    let nw = [0.0, 12288.0];
    let (outside, nowhere, home, closed, open) = (
        door(&mut e, [-100.0, 6144.0], 0),
        door(&mut e, [6144.0, 6144.0], 0),
        door(&mut e, [3072.0, 9216.0], parent),
        door(&mut e, [9216.0, 3072.0], unvisited),
        door(&mut e, [0.0, 12288.0], detached),
    );
    s.door_refs_list = vec![outside, nowhere, home, closed, open];
    let p = Place {
        player: 0,
        parent_cell: parent,
        parent_world: 1,
        use_fog: false,
        show_flags: 2,
        show_flags_change: 2,
    };
    door_markers(&mut e, &mut s, 0x0dea_1000, &p, true, false, nw);
    assert_eq!(num(&e, value_of(&e, rect, TRAIT_VISIBLE)), 1.0);
    let want = [
        (local([6144.0, 6144.0], nw), true),
        (local([3072.0, 9216.0], nw), true),
        (local([9216.0, 3072.0], nw), false),
        (local([0.0, 12288.0], nw), true),
    ];
    assert_eq!(s.door_marker_tiles.len(), 4, "one made from the template");
    assert_eq!(templates(), [b"MiniMapDoorMarkerTemplate".to_vec()]);
    for (t, (place, visited)) in s.door_marker_tiles.iter().zip(want) {
        assert_eq!([num(&e, t.x), num(&e, t.y)], place);
        assert_eq!(
            num(&e, t.visited),
            f32::from(u8::from(visited)),
            "{place:?}"
        );
        assert!(t.in_use);
        assert_eq!(num(&e, t.visible), 1.0);
    }

    // Two doors fewer: the two tiles left over are not in use, hidden.
    s.door_refs_list = vec![nowhere, home];
    door_markers(&mut e, &mut s, 0x0dea_1000, &p, true, false, nw);
    for (i, t) in s.door_marker_tiles.iter().enumerate() {
        assert_eq!(t.in_use, i < 2, "{i}");
        assert_eq!(num(&e, t.visible), if i < 2 { 1.0 } else { 0.0 });
    }

    // Flag 2 off: the rect hidden, the tiles left as they are.
    let off = Place { show_flags: 0, ..p };
    door_markers(&mut e, &mut s, 0x0dea_1000, &off, true, false, nw);
    assert_eq!(num(&e, value_of(&e, rect, TRAIT_VISIBLE)), 0.0);
    assert!(s.door_marker_tiles[0].in_use);
}

/// Under the fog: between full updates a fog update shows each door in use whose vertex alpha is
/// not 0 and hides the others; a tile not in use is left alone.
#[test]
fn the_door_markers_under_the_fog() {
    let mut e = setup();
    let mut s = MiniMapState::default();
    let (seen, unseen) = (alloc(4), alloc(4));
    e.mem.set_f32(seen, 0.5);
    e.mem.set_f32(unseen, 0.0);
    let mut tiles = vec![
        made_door_tile(&mut e),
        made_door_tile(&mut e),
        made_door_tile(&mut e),
    ];
    tiles[0].vtx_alpha = seen;
    tiles[1].vtx_alpha = unseen;
    tiles[2].vtx_alpha = unseen;
    tiles[2].in_use = false;
    s.door_marker_tiles = tiles;
    let p = Place {
        player: 0,
        parent_cell: 0,
        parent_world: 1,
        use_fog: true,
        show_flags: 2 | 0x10,
        show_flags_change: 0,
    };
    door_markers(&mut e, &mut s, 0, &p, false, true, [0.0, 0.0]);
    let shown: Vec<f32> = s
        .door_marker_tiles
        .iter()
        .map(|t| num(&e, t.visible))
        .collect();
    assert_eq!(shown, [1.0, 0.0, 1.0]);
}

// ---- the quest markers ----

/// A quest target at `at` (its reference at +0xC; `links`: the teleport links' first door at
/// +0x24, their count at +0x28).
fn target(e: &mut Engine, at: u32, refr: u32, link: Option<u32>) {
    e.mem.set_u32(at + TARGET_REF, refr);
    if let Some(door) = link {
        let links = alloc(4);
        e.mem.set_u32(links, door);
        e.mem.set_u32(at + TARGET_LINKS, links);
        e.mem.set_u32(at + TARGET_LINK_COUNT, 1);
    }
}

fn placed_at(e: &mut Engine, cell: u32, pos: [f32; 2]) -> u32 {
    let r = alloc(0x60);
    e.mem.set_u32(r + REF_CELL, cell);
    e.mem.set_f32(r + REF_POSITION, pos[0]);
    e.mem.set_f32(r + REF_POSITION + 4, pos[1]);
    r
}

/// On the local map in an exterior: a marker per target, in the list's order from depth 35 (a
/// target with a teleport link marks its door; one elsewhere, in neither the player's cell nor
/// world, is left out), then the placed marker's (coloured 191, 127, 255). Each placed at its
/// target; later only a target that moved (its x's bits) is placed again, unless on a full update.
#[test]
fn the_quest_markers() {
    let mut e = setup();
    extra_stand_ins(&mut e);
    let (world, elsewhere) = (alloc(0xd0), alloc(0xd0));
    let (cell, other_cell, far_cell) = (alloc(0xd0), alloc(0xd0), alloc(0xd0));
    e.mem.set_u32(cell + CELL_WORLD, world);
    e.mem.set_u32(other_cell + CELL_WORLD, world);
    e.mem.set_u32(far_cell + CELL_WORLD, elsewhere);
    let player = alloc(0x800);
    let near = placed_at(&mut e, cell, [6144.0, 6144.0]);
    let same_world = placed_at(&mut e, other_cell, [3072.0, 9216.0]);
    let far = placed_at(&mut e, far_cell, [0.0, 0.0]);
    let door_ref = placed_at(&mut e, cell, [9216.0, 3072.0]);
    let behind = placed_at(&mut e, far_cell, [0.0, 0.0]);
    let pin = placed_at(&mut e, cell, [0.0, 12288.0]);
    // The targets' list: far (left out), near, an empty item, the linked one, the same world's.
    let (t_far, t_near, t_link, t_world) = (alloc(0x30), alloc(0x30), alloc(0x30), alloc(0x30));
    target(&mut e, t_far, far, None);
    target(&mut e, t_near, near, None);
    target(&mut e, t_link, behind, Some(door_ref));
    target(&mut e, t_world, same_world, None);
    let mut node = player + QUEST_TARGETS;
    for (i, t) in [t_far, t_near, 0, t_link, t_world].iter().enumerate() {
        e.mem.set_u32(node, *t);
        let next = if i < 4 { alloc(8) } else { 0 };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
    // The placed target's reference is the placed marker (+0x6E8 + 0xC = +0x6F4).
    target(&mut e, player + PLACED_TARGET, pin, None);
    assert_eq!(e.mem.u32(player + PLACED_MARKER), pin);
    let rects = (
        tile_with(&mut e, &[TRAIT_VISIBLE]),
        tile_with(&mut e, &[TRAIT_VISIBLE]),
    );
    let mut s = MiniMapState {
        l_quest_markers_rect: rects.0,
        w_quest_markers_rect: rects.1,
        ..MiniMapState::default()
    };
    let nw = [0.0, 12288.0];
    let run = |e: &mut Engine, s: &mut MiniMapState, update_tiles: bool, changed: bool| {
        quest_markers(
            e,
            s,
            0x0dea_1000,
            player,
            cell,
            world,
            false,
            update_tiles,
            changed,
            nw,
            4,
            4,
        );
    };
    run(&mut e, &mut s, false, true);
    assert_eq!(num(&e, value_of(&e, rects.0, TRAIT_VISIBLE)), 1.0);
    assert_eq!(num(&e, value_of(&e, rects.1, TRAIT_VISIBLE)), 1.0);
    let mut keys: Vec<u32> = s.quest_markers.keys().copied().collect();
    keys.sort_unstable();
    let mut want = vec![near, door_ref, same_world, pin];
    want.sort_unstable();
    assert_eq!(
        keys, want,
        "the far target left out, the linked one's door marked"
    );
    assert_eq!(templates().len(), 4);
    assert!(
        calls_to(ADD_TILE_FROM_TEMPLATE)
            .iter()
            .all(|w| w[0] == rects.0),
        "the local map's rect"
    );
    let tile_of = |e: &Engine, s: &MiniMapState, r: u32| e.mem.u32(s.quest_markers[&r].x + 4);
    for (r, depth, place) in [
        (near, 35.0, local([6144.0, 6144.0], nw)),
        (door_ref, 36.0, local([9216.0, 3072.0], nw)),
        (same_world, 37.0, local([3072.0, 9216.0], nw)),
        (pin, 38.0, local([0.0, 12288.0], nw)),
    ] {
        let t = tile_of(&e, &s, r);
        assert_eq!(trait_num(&e, t, TRAIT_DEPTH), depth);
        let m = s.quest_markers[&r];
        assert_eq!([num(&e, m.x), num(&e, m.y)], place);
    }
    let pin_tile = tile_of(&e, &s, pin);
    assert_eq!(
        [TRAIT_RED, TRAIT_GREEN, TRAIT_BLUE].map(|id| trait_num(&e, pin_tile, id)),
        [191.0, 127.0, 255.0]
    );
    assert_eq!(
        trait_num(&e, tile_of(&e, &s, near), TRAIT_RED),
        0.0,
        "only the placed one"
    );
    assert_eq!((s.active_quest, s.placed_marker), (0, pin));

    // Next time: the near target moved; the door did not, so its marker stays where it was
    // (a stale value written to tell), until a full update.
    e.mem.set_f32(near + REF_POSITION, 3072.0);
    let door_x = s.quest_markers[&door_ref].x;
    e.mem.set_f32(door_x + VALUE_NUM, 0.0625);
    run(&mut e, &mut s, false, false);
    assert_eq!(
        num(&e, s.quest_markers[&near].x),
        local([3072.0, 6144.0], nw)[0]
    );
    assert_eq!(num(&e, door_x), 0.0625, "not moved, not placed again");
    assert_eq!(templates().len(), 4, "no new markers");
    run(&mut e, &mut s, true, false);
    assert_eq!(
        num(&e, door_x),
        local([9216.0, 3072.0], nw)[0],
        "a full update places every marker"
    );
}

// ---- the doors of a cell ----

/// A cell's teleport doors from the loaded references' door map: those in the cell, not deleted,
/// disabled or flagged 0x40 (form flags & 0x860), with a linked door; each with the linked door's
/// cell when that is an interior (no world), else 0.
#[test]
fn the_teleport_doors_of_a_cell() {
    let mut e = setup();
    extra_stand_ins(&mut e);
    let (cell, other) = (alloc(0xd0), alloc(0xd0));
    let (interior, exterior) = (alloc(0xd0), alloc(0xd0));
    e.mem.set_u32(exterior + CELL_WORLD, 0x0dea_3000);
    let linked_in = placed_at(&mut e, interior, [0.0, 0.0]);
    let linked_out = placed_at(&mut e, exterior, [0.0, 0.0]);
    let make = |e: &mut Engine, in_cell: u32, flags: u32, linked: u32| {
        let d = placed_at(e, in_cell, [0.0, 0.0]);
        e.mem.set_u32(d + FORM_FLAGS, flags);
        let data = alloc(0x20);
        e.mem.set_u32(data, linked);
        let extra = alloc(0x10);
        put_extra(e, d + REF_EXTRA, extra, EXTRA_TELEPORT, data);
        d
    };
    let to_interior = make(&mut e, cell, 0, linked_in);
    let to_exterior = make(&mut e, cell, 0, linked_out);
    let deleted = make(&mut e, cell, 0x20, linked_in);
    let disabled = make(&mut e, cell, 0x800, linked_in);
    let flagged = make(&mut e, cell, 0x40, linked_in);
    let elsewhere = make(&mut e, other, 0, linked_in);
    let unlinked = make(&mut e, cell, 0, 0);
    // The map: two buckets, chains from their heads.
    let loaded = alloc(0x100);
    e.mem.set_u32(LOADED_REFERENCES, loaded);
    let map = loaded + TELEPORT_DOORS;
    let buckets = alloc(8);
    e.mem.set_u32(map + 4, 2);
    e.mem.set_u32(map + 8, buckets);
    let chain = |e: &mut Engine, refs: &[u32]| {
        let mut head = 0;
        for &r in refs.iter().rev() {
            let entry = alloc(12);
            e.mem.set_u32(entry, head);
            e.mem.set_u32(entry + 8, r);
            head = entry;
        }
        head
    };
    let first = chain(&mut e, &[to_interior, deleted, elsewhere, 0]);
    let second = chain(&mut e, &[disabled, flagged, unlinked, to_exterior]);
    e.mem.set_u32(buckets, first);
    e.mem.set_u32(buckets + 4, second);
    assert_eq!(
        teleport_doors(&mut e, cell),
        [
            DoorRef {
                door: to_interior,
                linked_cell: interior
            },
            DoorRef {
                door: to_exterior,
                linked_cell: 0
            },
        ]
    );
}

// ---- the kept local maps and the game's texture list ----

/// `n` of JIP's local map textures (cell ids 1 to n) in the game's texture list, with one of the
/// game's own (no cell id) after the first; each kept in `s` by its cell id.
pub(super) fn texture_list(e: &mut Engine, s: &mut MiniMapState, n: u32) -> Vec<u32> {
    let mut list = Vec::new();
    for id in 1..=n {
        let t = alloc(0x40);
        e.mem.set_u32(t, VTBL_RENDERED_TEXTURE);
        e.mem.set_u32(t + TEXTURE_CELL, id);
        list.push(t);
        s.rendered_exteriors.insert(
            id,
            RenderedEntry {
                texture: t,
                regen_flags: 0,
            },
        );
        if id == 1 {
            let game = alloc(0x40);
            e.mem.set_u32(game, VTBL_RENDERED_TEXTURE);
            list.push(game);
        }
    }
    for (i, &t) in list.iter().enumerate() {
        let prev = if i == 0 { 0 } else { list[i - 1] };
        let next = list.get(i + 1).copied().unwrap_or(0);
        e.mem.set_u32(t + TEXTURE_PREV, prev);
        e.mem.set_u32(t + TEXTURE_NEXT, next);
    }
    e.mem.set_u32(TEXTURE_LIST_HEAD, list[0]);
    e.mem.set_u32(TEXTURE_LIST_TAIL, *list.last().unwrap());
    s.mm_first_texture = list[0];
    list
}

/// The list walked from its head.
fn walk(e: &Engine) -> Vec<u32> {
    let mut out = Vec::new();
    let mut t = e.mem.u32(TEXTURE_LIST_HEAD);
    while t != 0 {
        out.push(t);
        t = e.mem.u32(t + TEXTURE_NEXT);
    }
    out
}

/// A kept map drawn with again moves to the list's end under the list's critical section; when it
/// was JIP's first, the first is the next of JIP's (the game's own texture passed over).
#[test]
fn a_kept_map_moves_to_the_list_end() {
    let mut e = setup();
    let mut s = MiniMapState::default();
    let list = texture_list(&mut e, &mut s, 3);
    let (a, game, b, c) = (list[0], list[1], list[2], list[3]);
    assert_eq!(find_next_mm_texture(&e, a), b, "the game's own passed over");
    assert_eq!(find_next_mm_texture(&e, c), 0);
    CALLS.with(|c| c.borrow_mut().clear());
    move_to_list_end(&mut e, &mut s, a);
    assert_eq!(walk(&e), [game, b, c, a]);
    assert_eq!(e.mem.u32(TEXTURE_LIST_TAIL), a);
    assert_eq!(e.mem.u32(a + TEXTURE_PREV), c);
    assert_eq!(e.mem.u32(game + TEXTURE_PREV), 0);
    assert_eq!(s.mm_first_texture, b);
    let order: Vec<u32> = calls().into_iter().map(|c| c.0).collect();
    assert_eq!(order, [ENTER_CRITICAL_SECTION, LEAVE_CRITICAL_SECTION]);
    // One from the middle: JIP's first stays.
    move_to_list_end(&mut e, &mut s, c);
    assert_eq!(walk(&e), [game, b, a, c]);
    assert_eq!(s.mm_first_texture, b);
}

/// Up to 60 kept exterior maps stay; at 61 the oldest (JIP's first on, in the list's order) are
/// deleted down to 42, under the list's critical section, and JIP's first is the one after them.
#[test]
fn the_kept_maps_are_purged_above_60() {
    let mut e = setup();
    let mut s = MiniMapState::default();
    texture_list(&mut e, &mut s, 60);
    purge_rendered_exteriors(&mut e, &mut s);
    assert_eq!(s.rendered_exteriors.len(), 60);
    assert!(calls_to(RENDERED_TEXTURE_DELETE).is_empty());

    let mut s = MiniMapState::default();
    let list = texture_list(&mut e, &mut s, 61);
    CALLS.with(|c| c.borrow_mut().clear());
    purge_rendered_exteriors(&mut e, &mut s);
    assert_eq!(s.rendered_exteriors.len(), 42);
    let mm: Vec<u32> = list
        .iter()
        .copied()
        .filter(|&t| e.mem.u32(t + TEXTURE_CELL) != 0)
        .collect();
    let deleted: Vec<Vec<u32>> = calls_to(RENDERED_TEXTURE_DELETE);
    let want: Vec<Vec<u32>> = mm[..19].iter().map(|&t| vec![t, 1]).collect();
    assert_eq!(deleted, want);
    assert!((1..=19).all(|id| !s.rendered_exteriors.contains_key(&id)));
    assert!(s.rendered_exteriors.contains_key(&20));
    assert_eq!(s.mm_first_texture, mm[19]);
    let order: Vec<u32> = calls()
        .into_iter()
        .map(|c| c.0)
        .filter(|&f| f != RENDERED_TEXTURE_DELETE)
        .collect();
    assert_eq!(order, [ENTER_CRITICAL_SECTION, LEAVE_CRITICAL_SECTION]);
}

/// Door markers inside the local map only for 0 <= x < 1 and 0 <= y < 1 (a NaN outside).
#[test]
fn door_markers_outside_the_map_are_left_out() {
    assert!(!off_bounds([0.0, 0.0]));
    assert!(!off_bounds([0.999, 0.5]));
    assert!(off_bounds([1.0, 0.5]));
    assert!(off_bounds([0.5, 1.0]));
    assert!(off_bounds([-0.001, 0.5]));
    assert!(off_bounds([0.5, -0.001]));
    assert!(off_bounds([f32::NAN, 0.5]));
    assert!(!off_bounds([-0.0, 0.5]), "-0 is not below 0");
}

/// Review round 1: a kept map drawn again while it is JIP's first texture. The draw releases the
/// old texture (here: unlinked, its next word gone); JIP's first moves on to the next of its maps
/// first, so a purge above 60 deletes the oldest (cells 2 to 20) and never the map just drawn.
#[test]
fn a_kept_first_map_drawn_again_moves_the_first_on() {
    let mut e = setup();
    let mut s = MiniMapState::default();
    let list = texture_list(&mut e, &mut s, 61);
    let (old, second) = (list[0], list[2]);
    assert_eq!(s.mm_first_texture, old);
    let new = alloc(0x40);
    CALLS.with(|c| c.borrow_mut().clear());
    let made = redraw_kept(&mut e, &mut s, old, |e, _, slot| {
        assert_eq!(*slot, old);
        // The new texture goes to the list's end; the old one is freed.
        e.mem.set_u32(new, VTBL_RENDERED_TEXTURE);
        e.mem.set_u32(new + TEXTURE_CELL, 1);
        let tail = e.mem.u32(TEXTURE_LIST_TAIL);
        e.mem.set_u32(tail + TEXTURE_NEXT, new);
        e.mem.set_u32(new + TEXTURE_PREV, tail);
        e.mem.set_u32(TEXTURE_LIST_TAIL, new);
        e.mem.set_u32(old + TEXTURE_NEXT, 0);
        *slot = new;
    });
    assert_eq!(made, new);
    assert_eq!(s.mm_first_texture, second, "moved on before the release");
    let order: Vec<u32> = calls().into_iter().map(|c| c.0).collect();
    assert_eq!(order, [ENTER_CRITICAL_SECTION, LEAVE_CRITICAL_SECTION]);
    s.rendered_exteriors.insert(
        1,
        RenderedEntry {
            texture: new,
            regen_flags: 0,
        },
    );
    CALLS.with(|c| c.borrow_mut().clear());
    purge_rendered_exteriors(&mut e, &mut s);
    let deleted: Vec<u32> = calls_to(RENDERED_TEXTURE_DELETE)
        .into_iter()
        .map(|w| w[0])
        .collect();
    assert_eq!(deleted.len(), 19);
    assert!(!deleted.contains(&new) && !deleted.contains(&old));
    assert_eq!(deleted[0], second);
    assert_eq!(
        s.rendered_exteriors[&1].texture, new,
        "the map just drawn is kept"
    );

    // Not JIP's first: nothing moves, no lock.
    let mut s = MiniMapState::default();
    let list = texture_list(&mut e, &mut s, 3);
    CALLS.with(|c| c.borrow_mut().clear());
    redraw_kept(&mut e, &mut s, list[2], |_, _, slot| *slot = 0x0dea_5000);
    assert_eq!(s.mm_first_texture, list[0]);
    assert!(calls().is_empty());
    // The only map, drawn again: the first is the new one.
    let mut s = MiniMapState::default();
    let one = texture_list(&mut e, &mut s, 1)[0];
    e.mem.set_u32(one + TEXTURE_NEXT, 0);
    redraw_kept(&mut e, &mut s, one, |_, _, slot| *slot = 0x0dea_6000);
    assert_eq!(s.mm_first_texture, 0x0dea_6000);
}

/// Review round 1: the exterior's drawing with every grid slot empty (no cell loaded there). Each
/// slot's texture is cleared, not left on a map the purge or a format change may have deleted
/// (JIP leaves it); around the loop the sun's light and the scene's fog are JIP's, then put back,
/// with the underwater flag, under the scene lights' lock entered and left once; outside a full
/// update the extra node is hidden for the draw and shown after; the water planes end hidden.
#[test]
fn an_empty_grid_slot_keeps_no_old_texture() {
    let mut e = setup();
    for page in [0x011f_9000, 0x011d_8000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.register(LOCK_ENTER, |_, w| {
        record(LOCK_ENTER, w);
        eax(0)
    });
    e.register(LOCK_LEAVE, |_, w| {
        record(LOCK_LEAVE, w);
        eax(0)
    });
    let (scene, sun, tes) = (alloc(0x140), alloc(0x100), alloc(0x40));
    e.mem.set_u32(SHADOW_SCENE_NODE, scene);
    e.mem.set_u32(scene + SCENE_SUN, sun);
    e.mem.set_u32(TES, tes);
    e.mem.set_u32(tes + TES_LIGHT, 0x0dea_7001);
    e.mem.set_u32(GAME_FOG_PROPERTY, 0x0dea_7002);
    e.mem.set_u32(sun + LIGHT_LIGHT, 0x0dea_7001);
    e.mem.set_u32(scene + SCENE_FOG, 0x0dea_7002);
    e.mem.set_u8(UNDERWATER_FOG, 0);
    let extra = alloc(0x40);
    e.mem.set_u32(EXTRA_HIDDEN_NODE, extra);
    let planes = alloc(0x40);
    let children = alloc(8);
    let plane = alloc(0x40);
    e.mem.set_u32(children, plane);
    e.mem.set_u32(planes + CHILDREN, children);
    e.mem.set_u16(planes + CHILD_COUNT, 1);
    let mut s = MiniMapState {
        light: 0x0dea_7101,
        fog: 0x0dea_7102,
        water_planes: planes,
        ..MiniMapState::default()
    };
    for (i, prop) in s.shader_props.iter_mut().enumerate() {
        *prop = alloc(0x80);
        e.mem
            .set_u32(*prop + SHADER_TEXTURE, 0x0dea_8000 + i as u32);
    }
    CALLS.with(|c| c.borrow_mut().clear());
    draw_exterior(&mut e, &mut s, false, [0.0, 0.0]);
    for prop in s.shader_props {
        assert_eq!(e.mem.u32(prop + SHADER_TEXTURE), 0);
    }
    assert_eq!(e.mem.u32(sun + LIGHT_LIGHT), 0x0dea_7001);
    assert_eq!(e.mem.u32(scene + SCENE_FOG), 0x0dea_7002);
    assert_eq!(e.mem.u8(UNDERWATER_FOG), 0);
    assert_eq!(
        e.mem.u32(extra + FLAGS) & 1,
        0,
        "the extra node shown again"
    );
    assert_eq!(e.mem.u32(plane + FLAGS) & 1, 1, "the water planes hidden");
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
}
