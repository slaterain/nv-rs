//! JIP MiniMap's seams and its update of the seen bits. Expectations from JIP's hooks and helpers
//! (functions_jip/jip_fn_minimap.h: the patches at the end of `Cmd_InitMiniMap_Execute`,
//! `DiscoverLocationHook`, `SetQuestTargetsHook`, `UpdatePlacedMarkerHook`,
//! `RendererRecreateHook`, `UpdateCellsSeenBitsHook`, `GetFOWUpdateMask`, `UpdateSeenBits`,
//! `AddExtraSeenData`, `AddIntSeenData`; nvse/GameData.cpp: `GridCellArray::GetCellAtCoord`).

use super::*;
use crate::commands::test_support::*;

fn answer(e: &mut Engine, seam: SiteSlot, words: &[u32]) -> Option<u32> {
    e.site(seam.addr(), words)
}

fn patch(e: &mut Engine) {
    mini_map(e).patched.store(true, Ordering::Release);
}

/// Before `InitMiniMap` every seam leaves the game alone (no answer, no flag).
#[test]
fn nothing_before_init() {
    let mut e = engine();
    for seam in [
        GRID_EXTENT,
        SEEN_BITS_FUNCTION,
        QUEST_TARGETS,
        DISCOVER_LOCATION,
        SEEN_BITS_CALL,
        PLACED_MARKER_CHECK,
        PLACED_MARKER_SET,
        RENDERER_RESET,
    ] {
        assert_eq!(answer(&mut e, seam, &[3, 0]), None, "{:08x}", seam.addr());
    }
    let m = mini_map(&mut e);
    assert!(!m.update_quest_targets.load(Ordering::Acquire));
    assert!(!m.discovered_location.load(Ordering::Acquire));
    assert_eq!(m.texture_format.load(Ordering::Acquire), 22);
}

/// After it: the small grid for a setting up to 5 (unsigned: not for -1), the game's seen bits
/// skipped, the quest and marker flags set with the game going on, the texture format 0 on a
/// device reset.
#[test]
fn the_patches_after_init() {
    let mut e = engine();
    patch(&mut e);
    for (setting, small) in [(0, 1), (3, 1), (5, 1), (6, 0), (7, 0), (u32::MAX, 0)] {
        assert_eq!(
            answer(&mut e, GRID_EXTENT, &[setting]),
            Some(small),
            "{setting}"
        );
    }
    assert_eq!(answer(&mut e, SEEN_BITS_FUNCTION, &[]), Some(0));
    for seam in [QUEST_TARGETS, PLACED_MARKER_CHECK, PLACED_MARKER_SET] {
        mini_map(&mut e)
            .update_quest_targets
            .store(false, Ordering::Release);
        assert_eq!(answer(&mut e, seam, &[]), None);
        assert!(mini_map(&mut e)
            .update_quest_targets
            .load(Ordering::Acquire));
    }
    assert_eq!(answer(&mut e, RENDERER_RESET, &[]), None);
    assert_eq!(mini_map(&mut e).texture_format.load(Ordering::Acquire), 0);
}

/// The found location: the flag set, the marker's name (the empty text without one); with no
/// rendered markers nothing else.
#[test]
fn a_discovered_location_gives_its_name() {
    let mut e = engine();
    patch(&mut e);
    let data = FORMS + 0x100;
    let record = FORMS + 0x200;
    e.mem.set_u32(data + 4, FORMS + 0x300);
    e.mem.set_u32(record + 4, FORMS + 0x400);
    assert_eq!(
        answer(&mut e, DISCOVER_LOCATION, &[data, record]),
        Some(FORMS + 0x300)
    );
    assert!(mini_map(&mut e).discovered_location.load(Ordering::Acquire));
    e.mem.set_u32(data + 4, 0);
    assert_eq!(
        answer(&mut e, DISCOVER_LOCATION, &[data, record]),
        Some(0x0101_1584)
    );
}

/// With rendered markers, the found marker's cell (its place by the root world's dimensions)
/// lets its tiles go: not in use, hidden; other cells keep theirs.
#[test]
fn a_discovered_location_frees_its_cells_markers() {
    let mut e = engine();
    patch(&mut e);
    // A value's refresh (`Tile::Value::SetBool`).
    e.register(0x00a0_9410, |_, _| Ret::default());
    let data = FORMS + 0x100;
    let record = FORMS + 0x200;
    let marker = FORMS + 0x400;
    e.mem.set_u32(record + 4, marker);
    e.mem.set_f32(marker + 0x30, 5000.0);
    e.mem.set_f32(marker + 0x34, -9000.0);
    let visible = FORMS + 0x500;
    e.mem.set_f32(visible + 8, 1.0);
    let coord = (1u32 << 16) | (-3i32 as u32 & 0xffff);
    {
        let mut s = state(&mut e);
        s.root_dims.scale = 1.0;
        s.marker_tiles = vec![
            super::super::jip_fn_minimap::MarkerTile {
                visible,
                in_use: true,
                ..Default::default()
            },
            super::super::jip_fn_minimap::MarkerTile {
                in_use: true,
                ..Default::default()
            },
        ];
        s.rendered_markers.insert(coord, vec![0]);
        s.rendered_markers.insert(7, vec![1]);
    }
    answer(&mut e, DISCOVER_LOCATION, &[data, record]);
    let s = state(&mut e);
    assert!(!s.rendered_markers.contains_key(&coord));
    assert!(s.rendered_markers.contains_key(&7));
    assert!(!s.marker_tiles[0].in_use);
    assert!(s.marker_tiles[1].in_use);
    drop(s);
    assert_eq!(e.mem.f32(visible + 8), 0.0);
}

/// JIP's table: by the place within its cell (`& 0xFFF`), past 0x4FF and 0xAFF in y (bits 0, 1)
/// and x (bits 2, 3).
#[test]
fn the_update_mask_follows_jips_table() {
    assert_eq!(fow_update_mask(0, 0), 0x1b);
    assert_eq!(fow_update_mask(0, 0x500), 0x12);
    assert_eq!(fow_update_mask(0, 0xb00), 0x36);
    assert_eq!(fow_update_mask(0x500, 0), 0x18);
    assert_eq!(fow_update_mask(0xb00, 0), 0xd8);
    assert_eq!(fow_update_mask(0xb00, 0xb00), 0x1b0);
    assert_eq!(fow_update_mask(0x1500, 0x1000), 0x18, "the cell's own part");
    assert_eq!(fow_update_mask(-1, -1), 0x1b0, "0xFFF of a negative place");
}

/// Packed coordinates add and shift by halves.
#[test]
fn packed_words() {
    assert_eq!(add_words(0xffff_ffff, 0x0003_0005), 0x0002_0004);
    assert_eq!(add_words(0x0001_0001, 0xffff_ffff), 0);
    assert_eq!(shift_words(0xffff_0001), 0xfff0_0010);
    assert_eq!(pack(-1, 2), 0xffff_0002);
}

/// The grid cell at a coordinate: `5x + y` from the grid's corner for a five-cell grid; 0 out of
/// the grid.
#[test]
fn grid_cells_by_coordinate() {
    let e = engine();
    let grid = FORMS + 0x1000;
    let cells = FORMS + 0x1100;
    e.mem.set_u32(grid + 4, 10u32);
    e.mem.set_u32(grid + 8, (-4i32) as u32);
    e.mem.set_u32(grid + 0xc, 5);
    e.mem.set_u32(grid + 0x10, cells);
    for i in 0..25 {
        e.mem.set_u32(cells + 4 * i, 0x7000 + i);
    }
    assert_eq!(cell_at_coord(&e, grid, pack(10, -4)), 0x7000 + 12);
    assert_eq!(cell_at_coord(&e, grid, pack(8, -6)), 0x7000);
    assert_eq!(cell_at_coord(&e, grid, pack(12, -2)), 0x7000 + 24);
    assert_eq!(cell_at_coord(&e, grid, pack(13, -4)), 0);
    assert_eq!(cell_at_coord(&e, grid, pack(10, -7)), 0);
}

/// The brush: around the place, half widths 4 3 3 2 0 by the row's distance; a new bit sets the
/// fog flag; all bits set answers true.
#[test]
fn the_seen_bits_brush() {
    let mut e = engine();
    let seen = FORMS + 0x2000;
    for i in 0..9 {
        e.mem.set_u32(seen + 4 * i, 0);
    }
    // The place (-8, -8): rows 4..=12 around row 8.
    assert!(!update_seen_bits(&mut e, seen, pack(-8, -8)));
    assert!(mini_map(&mut e).update_fog_of_war.load(Ordering::Acquire));
    let bit = |e: &Engine, column: u32, row: u32| {
        let b = column << 4 | row;
        e.mem.u32(seen + 4 + 4 * (b >> 5)) >> (b & 31) & 1 == 1
    };
    assert!(bit(&e, 8, 8));
    assert!(bit(&e, 4, 8) && bit(&e, 12, 8) && !bit(&e, 3, 8) && !bit(&e, 13, 8));
    assert!(bit(&e, 5, 9) && bit(&e, 11, 9) && !bit(&e, 4, 9));
    assert!(bit(&e, 6, 11) && !bit(&e, 5, 11));
    assert!(bit(&e, 8, 12) && !bit(&e, 7, 12) && !bit(&e, 9, 12));
    assert!(!bit(&e, 8, 13));
    // Again: nothing new.
    mini_map(&mut e)
        .update_fog_of_war
        .store(false, Ordering::Release);
    update_seen_bits(&mut e, seen, pack(-8, -8));
    assert!(!mini_map(&mut e).update_fog_of_war.load(Ordering::Acquire));
    for i in 0..8 {
        e.mem.set_u32(seen + 4 + 4 * i, u32::MAX);
    }
    assert!(update_seen_bits(&mut e, seen, pack(-8, -8)));
}

/// Interior sections: found by their number, a new one appended (bits zeroed).
#[test]
fn interior_sections() {
    let mut e = engine();
    let root = FORMS + 0x3000;
    e.mem.set_u16(root + 0x24, 0xffff);
    e.mem.set_u32(root + 0x28, 0);
    let found = add_int_seen_data(&mut e, root, 0xffff);
    assert_eq!(found, root);
    let made = add_int_seen_data(&mut e, root, 0x0102);
    assert_ne!(made, root);
    assert_eq!(e.mem.u32(root + 0x28), made);
    assert_eq!(e.mem.u32(made), 0x0108_3fe4);
    assert_eq!(e.mem.u16(made + 0x24), 0x0102);
    assert_eq!(e.mem.u8(made + 0x26), 0);
    assert_eq!(add_int_seen_data(&mut e, root, 0x0102), made);
}

/// The player, `TES`, its grid of five cells around (10, 20) and the game's words the update
/// reads; the cells by grid index (`5x + y`); the change-flag calls recorded.
fn seen_world(e: &mut Engine) -> (u32, [u32; 25]) {
    extra_stand_ins(e);
    for page in [0x011d_e000, 0x011c_a000, 0x011e_0000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.register(ADD_CHANGE_FLAGS, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((ADD_CHANGE_FLAGS, w.to_vec())));
        Ret::default()
    });
    e.register(SET_CHANGE_FLAGS, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((SET_CHANGE_FLAGS, w.to_vec())));
        Ret::default()
    });
    let player = FORMS + 0x5000;
    let tes = FORMS + 0x6000;
    let grid = FORMS + 0x6100;
    let array = FORMS + 0x6200;
    e.mem.set_u32(PLAYER, player);
    e.mem.set_u32(TES, tes);
    e.mem.set_u32(tes + TES_GRID, grid);
    e.mem.set_u32(tes + TES_CENTRE, 10);
    e.mem.set_u32(tes + TES_CENTRE + 4, 20);
    e.mem.set_u32(grid + 4, 10);
    e.mem.set_u32(grid + 8, 20);
    e.mem.set_u32(grid + 0xc, 5);
    e.mem.set_u32(grid + 0x10, array);
    let mut cells = [0u32; 25];
    for (i, at) in [(12usize, 0x7000u32), (6, 0x7400)] {
        let cell = FORMS + at;
        e.mem.set_u32(cell + CELL_ID, 0x100 + i as u32);
        e.mem.set_u32(array + 4 * i as u32, cell);
        cells[i] = cell;
    }
    e.mem.set_u32(player + PLAYER_CELL, cells[12]);
    e.mem
        .set_f32(player + PLAYER_POSITION, 10.0 * 4096.0 + 100.0);
    e.mem
        .set_f32(player + PLAYER_POSITION + 4, 20.0 * 4096.0 + 100.0);
    (player, cells)
}

fn seen_bit(e: &Engine, cell: u32, column: u32, row: u32) -> bool {
    let extra = e.mem.u32(cell + CELL_EXTRA + 4);
    let data = e.mem.u32(extra + EXTRA_DATA);
    let b = column << 4 | row;
    e.mem.u32(data + 4 + 4 * (b >> 5)) >> (b & 31) & 1 == 1
}

/// Outside, from a new cell: the place kept, the cells the mask reaches that are loaded get their
/// seen data (an extra, marked changed) and the brush's bits around the place in their own cell's
/// terms; the corner cell (-1, -1) only its bits nearest the place.
#[test]
fn the_seen_bits_outside() {
    let mut e = engine();
    patch(&mut e);
    let (player, cells) = seen_world(&mut e);
    assert_eq!(answer(&mut e, SEEN_BITS_CALL, &[]), Some(0));
    assert_eq!(
        e.mem.f32(LAST_POSITION),
        e.mem.f32(player + PLAYER_POSITION)
    );
    assert_eq!(calls_to(ADD_EXTRA).len(), 2);
    let changed: Vec<u32> = calls_to(ADD_CHANGE).iter().map(|w| w[1]).collect();
    assert_eq!(changed, vec![cells[6], cells[12]]);
    // The own cell: the place is (-1, -1) in 256ths from its corner.
    assert!(seen_bit(&e, cells[12], 1, 1));
    assert!(seen_bit(&e, cells[12], 5, 1) && !seen_bit(&e, cells[12], 6, 1));
    assert!(!seen_bit(&e, cells[12], 1, 6));
    // The corner cell: (-17, -17).
    assert!(seen_bit(&e, cells[6], 14, 15) && seen_bit(&e, cells[6], 15, 15));
    assert!(seen_bit(&e, cells[6], 15, 14));
    assert!(!seen_bit(&e, cells[6], 13, 15) && !seen_bit(&e, cells[6], 14, 14));
    assert!(calls_to(REMOVE_BY_TYPE).is_empty());
}

/// A cell whose last bit is set loses its seen data, gets JIP's "all seen" flag and its change
/// flags (2, then 0x80000000); then it is passed over.
#[test]
fn a_fully_seen_cell_drops_its_seen_data() {
    let mut e = engine();
    patch(&mut e);
    let (_, cells) = seen_world(&mut e);
    let d = cells[6];
    let extra = FORMS + 0x8000;
    let data = FORMS + 0x8100;
    put_extra(&mut e, d + CELL_EXTRA, extra, EXTRA_SEEN_DATA, data);
    for i in 0..8 {
        e.mem.set_u32(data + 4 + 4 * i, u32::MAX);
    }
    e.mem.set_u32(data + 4 + 4 * 7, 0x7fff_ffff);
    answer(&mut e, SEEN_BITS_CALL, &[]);
    assert_eq!(
        calls_to(REMOVE_BY_TYPE),
        vec![vec![d + CELL_EXTRA, EXTRA_SEEN_DATA]]
    );
    assert_eq!(e.mem.u8(d + CELL_SEEN_ALL) & 1, 1);
    let changes = e.mem.u32(e.mem.u32(SAVE_LOAD_GAME));
    assert_eq!(calls_to(ADD_CHANGE_FLAGS), vec![vec![changes, 0x106, 2]]);
    assert_eq!(
        calls_to(SET_CHANGE_FLAGS),
        vec![vec![changes, 0x106, 0x8000_0000]]
    );
}

/// In the tracked cell, a move under 80 units does nothing; 80 or more updates.
#[test]
fn small_moves_in_the_same_cell_wait() {
    let mut e = engine();
    patch(&mut e);
    let (player, cells) = seen_world(&mut e);
    e.mem.set_u32(TRACKED_CELL, cells[12]);
    let x = e.mem.f32(player + PLAYER_POSITION);
    let y = e.mem.f32(player + PLAYER_POSITION + 4);
    e.mem.set_f32(LAST_POSITION, x + 50.0);
    e.mem.set_f32(LAST_POSITION + 4, y + 50.0);
    answer(&mut e, SEEN_BITS_CALL, &[]);
    assert!(calls_to(ADD_EXTRA).is_empty());
    e.mem.set_f32(LAST_POSITION, x + 80.0);
    e.mem.set_f32(LAST_POSITION + 4, y);
    answer(&mut e, SEEN_BITS_CALL, &[]);
    assert_eq!(calls_to(ADD_EXTRA).len(), 2);
}

/// The player in an interior cell (flags bit 0) at `pos`, the tracked cell another (so the north
/// is read), with `angle` the cell's `ExtraNorthRotation`; the cell.
fn seen_interior(e: &mut Engine, pos: [f32; 2], angle: Option<f32>) -> u32 {
    extra_stand_ins(e);
    for page in [0x011d_e000, 0x011c_a000, 0x011e_0000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    let player = FORMS + 0x5000;
    let cell = FORMS + 0x7000;
    e.mem.set_u32(PLAYER, player);
    e.mem.set_u32(player + PLAYER_CELL, cell);
    e.mem.set_u8(cell + CELL_FLAGS, 1);
    e.mem.set_u32(cell + CELL_ID, 0x200);
    e.mem.set_u32(TRACKED_CELL, FORMS + 0x7400);
    e.mem.set_f32(player + PLAYER_POSITION, pos[0]);
    e.mem.set_f32(player + PLAYER_POSITION + 4, pos[1]);
    if let Some(a) = angle {
        put_extra(
            e,
            cell + CELL_EXTRA,
            FORMS + 0x7800,
            EXTRA_NORTH_ROTATION,
            a.to_bits(),
        );
    }
    cell
}

/// The interior's sections from its seen data, in their list's order: number and address.
fn sections(e: &mut Engine, cell: u32) -> Vec<(u16, u32)> {
    let extra = super::super::extra::get_by_type(e, cell + CELL_EXTRA, EXTRA_SEEN_DATA);
    let mut d = e.mem.u32(extra + EXTRA_DATA);
    let mut out = Vec::new();
    while d != 0 {
        out.push((e.mem.u16(d + INT_SECTION), d));
        d = e.mem.u32(d + INT_NEXT);
    }
    out
}

fn int_bits(e: &Engine, seen: u32) -> Vec<u32> {
    (0..256)
        .filter(|b| e.mem.u32(seen + 4 + 4 * (b >> 5)) >> (b & 31) & 1 == 1)
        .collect()
}

/// Inside, with no north rotation: the place less 0x800 each way (100, 5000 gives -1948, 2952:
/// section x -1, y 0), JIP's mask from it (bits 4 and 5: this section and y + 1), the sections
/// made after the cell's own seen data by their number `(y & 0xFF) << 8 | (x & 0xFF)` (0x00FF,
/// 0x01FF), the brush's centre bit (column 12, row 9) in the first and its one edge bit (column
/// 0, row 9) in the next.
#[test]
fn the_seen_bits_inside() {
    let mut e = engine();
    patch(&mut e);
    let cell = seen_interior(&mut e, [100.0, 5000.0], None);
    assert_eq!(answer(&mut e, SEEN_BITS_CALL, &[]), Some(0));
    assert_eq!(mini_map(&mut e).north_rotation.load(Ordering::Acquire), 0);
    let found = sections(&mut e, cell);
    let numbers: Vec<u16> = found.iter().map(|s| s.0).collect();
    assert_eq!(numbers, [0xffff, 0x00ff, 0x01ff]);
    assert!(int_bits(&e, found[1].1).contains(&0xc9));
    assert_eq!(int_bits(&e, found[2].1), [0x09]);
    assert!(
        int_bits(&e, found[0].1).is_empty(),
        "the cell's own seen data"
    );
    assert_eq!(e.mem.u8(found[1].1 + INT_SEEN_ALL), 0);
}

/// With a north rotation (0.5): `s_cellNorthRotation` the angle's negation, the lanes (sin, cos,
/// cos, -sin) of it; the place turned by it (x cos - y sin, x sin + y cos) before the 0x800, so its
/// section and the brush's centre follow the turned place.
#[test]
fn the_seen_bits_inside_follow_the_cells_north() {
    let mut e = engine();
    patch(&mut e);
    let cell = seen_interior(&mut e, [100.0, 5000.0], Some(0.5));
    answer(&mut e, SEEN_BITS_CALL, &[]);
    let m = mini_map(&mut e);
    assert_eq!(
        m.north_rotation.load(Ordering::Acquire),
        (-0.5f32).to_bits()
    );
    let r: Vec<f32> = m
        .north_rotation_mods
        .iter()
        .map(|v| f32::from_bits(v.load(Ordering::Acquire)))
        .collect();
    assert!((r[0] - (-0.5f32).sin()).abs() < 1e-4, "{r:?}");
    assert!((r[1] - (-0.5f32).cos()).abs() < 1e-4, "{r:?}");
    assert_eq!((r[2], r[3]), (r[1], -r[0]));
    let (x, y) = (100.0f32, 5000.0f32);
    let turned = [x * r[1] - y * r[0], x * r[0] + y * r[1]];
    let cvt = crate::game_patches::gd::cvtss2si;
    let (px, py) = (cvt(turned[0]) - 0x800, cvt(turned[1]) - 0x800);
    let (sx, sy) = (px >> 12, py >> 12);
    let number = ((sy as u16 & 0xff) << 8) | (sx as u16 & 0xff);
    let row = -((sx.wrapping_shl(12) - px) >> 8);
    let column = -((sy.wrapping_shl(12) - py) >> 8);
    let found = sections(&mut e, cell);
    let section = found
        .iter()
        .find(|s| s.0 == number)
        .expect("the turned place's section");
    assert!(int_bits(&e, section.1).contains(&((column as u32) << 4 | row as u32)));
}

/// A section whose last bit the brush sets is marked all seen (+0x26); the next sections are made
/// after the ones the cell has.
#[test]
fn an_interior_section_filled_is_marked_all_seen() {
    let mut e = engine();
    patch(&mut e);
    let cell = seen_interior(&mut e, [100.0, 5000.0], None);
    let root = FORMS + 0x9000;
    let next = FORMS + 0x9100;
    for (d, section, link) in [(root, 0xffffu16, next), (next, 0x01ff, 0)] {
        e.mem.set_u32(d, 0x0108_3fe4);
        e.mem.set_u16(d + INT_SECTION, section);
        e.mem.set_u8(d + INT_SEEN_ALL, 0);
        e.mem.set_u32(d + INT_NEXT, link);
    }
    for i in 0..8 {
        e.mem.set_u32(next + 4 + 4 * i, u32::MAX);
    }
    e.mem.set_u32(next + 4, !(1 << 9));
    put_extra(
        &mut e,
        cell + CELL_EXTRA,
        FORMS + 0x9200,
        EXTRA_SEEN_DATA,
        root,
    );
    answer(&mut e, SEEN_BITS_CALL, &[]);
    assert_eq!(e.mem.u8(next + INT_SEEN_ALL), 1);
    let numbers: Vec<u16> = sections(&mut e, cell).iter().map(|s| s.0).collect();
    assert_eq!(numbers, [0xffff, 0x01ff, 0x00ff]);
}
