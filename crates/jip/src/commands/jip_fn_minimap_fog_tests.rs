//! The fog of war's seen levels and vertex alphas on stand-in seen data, every expectation from
//! JIP's asm (`GetSectionSeenData`, `GetGridCellSeenData`, `GetSectionSeenLevel`,
//! `PackCurrGridCoords`, `GetVtxAlphaPtr`), functions_jip/jip_fn_minimap.h.

use super::super::jip_fn_minimap_seams::pack;
use super::*;
use crate::commands::test_support::engine;
const AREA: u32 = 0x0610_0000;

/// A seen data block (`SeenData` / `IntSeenData`: vtable, 256 bits) with `bits` set; an interior
/// one gets `section`, its "all seen" byte and its next.
fn seen(e: &mut Engine, at: u32, vtable: u32, bits: &[u32]) -> u32 {
    e.mem.set_u32(at, vtable);
    for b in bits {
        let word = at + 4 + (b >> 5) * 4;
        let v = e.mem.u32(word);
        e.mem.set_u32(word, v | 1 << (b & 31));
    }
    at
}

fn int_seen(e: &mut Engine, at: u32, section: u16, all: bool, next: u32, bits: &[u32]) -> u32 {
    seen(e, at, 0x0108_3fe4, bits);
    e.mem.set_u16(at + 0x24, section);
    e.mem.set_u8(at + 0x26, u8::from(all));
    e.mem.set_u32(at + 0x28, next);
    at
}

/// A section's number from its x and y (x's byte low, y's high).
fn section(x: u8, y: u8) -> u16 {
    u16::from(x) | u16::from(y) << 8
}

/// The packed bit of a vertex: column (the bit's low nibble's partner) in the high byte, row in
/// the low one, as `CalcVtxAlphaBySeenData` passes them.
fn packed(column: u32, row: u32) -> u32 {
    (column << 8) | row
}

#[test]
fn an_interior_vertex_counts_its_four_bits() {
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    // Section (0, 0): the vertex at row 2, column 3: bit (2 << 4) | 3 = 0x23 and its three
    // neighbours 0x33 (row below), 0x24 (column right), 0x34.
    let s0 = int_seen(&mut e, AREA, section(0, 0), false, 0, &[0x23, 0x24, 0x34]);
    assert_eq!(section_seen_level(&mut e, s0, packed(3, 2), 0), 3);
    // The first bit not set: two left.
    let s1 = int_seen(&mut e, AREA + 0x100, section(0, 0), false, 0, &[0x24, 0x34]);
    assert_eq!(section_seen_level(&mut e, s1, packed(3, 2), 0), 2);
    // Another section first in the list: the walk finds (0, 0) after it.
    let first = int_seen(&mut e, AREA + 0x200, section(5, 5), false, s0, &[0x23]);
    assert_eq!(section_seen_level(&mut e, first, packed(3, 2), 0), 3);
    // No section (0, 0) at all: 0.
    let alone = int_seen(&mut e, AREA + 0x300, section(1, 0), false, 0, &[0x23]);
    assert_eq!(section_seen_level(&mut e, alone, packed(3, 2), 0), 0);
}

/// The last row (low nibble 15): its own two bits, then the next section's (y + 1) first row; a
/// next section seen whole adds 2. The last column: the next section across (x + 1), its first
/// column. The corner: one bit of each of the three next sections, no "seen whole" there.
#[test]
fn an_interior_edge_vertex_reads_the_next_sections() {
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    // Row 15, column 3: bit 0xF3, its right 0xF4; the next row's section (0, 1): bits 3 and 4.
    let below = int_seen(&mut e, AREA + 0x100, section(0, 1), false, 0, &[0x3, 0x4]);
    let own = int_seen(&mut e, AREA, section(0, 0), false, below, &[0xf3]);
    assert_eq!(section_seen_level(&mut e, own, packed(3, 15), 0), 3);
    e.mem.set_u8(below + 0x26, 1);
    assert_eq!(
        section_seen_level(&mut e, own, packed(3, 15), 0),
        3,
        "1 own + 2"
    );
    // Column 15, row 2: bit 0x2F, below 0x3F; the section across (1, 0): bits 0x20, 0x30.
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    let across = int_seen(&mut e, AREA + 0x100, section(1, 0), false, 0, &[0x20, 0x30]);
    let own = int_seen(&mut e, AREA, section(0, 0), false, across, &[0x2f, 0x3f]);
    assert_eq!(section_seen_level(&mut e, own, packed(15, 2), 0), 4);
    // The corner (15, 15): bit 0xFF; (0, 1)'s bit 0xF, (1, 0)'s bit 0xF0, (1, 1)'s bit 0.
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    let d = int_seen(&mut e, AREA + 0x300, section(1, 1), true, 0, &[0]);
    let c = int_seen(&mut e, AREA + 0x200, section(1, 0), false, d, &[0xf0]);
    let b = int_seen(&mut e, AREA + 0x100, section(0, 1), true, c, &[]);
    let own = int_seen(&mut e, AREA, section(0, 0), false, b, &[0xff]);
    assert_eq!(
        section_seen_level(&mut e, own, packed(15, 15), 0),
        3,
        "no 'seen whole' at the corner"
    );
}

/// `PackCurrGridCoords`: x - 1 to x + 1 (the high word), each y - 1 to y + 1, the centre fifth.
#[test]
fn the_grid_around_a_cell_is_packed_by_column() {
    let c = pack(3, -2);
    let got = pack_curr_grid_coords(c);
    let want: Vec<u32> = [
        (2, -3),
        (2, -2),
        (2, -1),
        (3, -3),
        (3, -2),
        (3, -1),
        (4, -3),
        (4, -2),
        (4, -1),
    ]
    .iter()
    .map(|&(x, y)| pack(x, y))
    .collect();
    assert_eq!(got.to_vec(), want);
}

/// `CalcVtxAlphaBySeenData` in an interior (the player's cell flagged 1, its `ExtraSeenData`
/// holding the sections): the 17 x 17 alphas by column, each from row 16 down to 0, from the seen
/// levels. One bit (row 2, column 3) seen makes the four vertices around it level 1 (columns 2 and
/// 3, rows 1 and 2) and leaves every other at level 0; the buffer is marked not dirty.
#[test]
fn an_interiors_vertex_alphas_by_column_from_the_last_row() {
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    crate::commands::test_support::extra_stand_ins(&mut e);
    if !e.mem.is_mapped(TES) {
        e.map(TES & !0xfff, 0x1000);
    }
    let tes = AREA + 0x8000;
    e.mem.set_u32(TES, tes);
    let section0 = int_seen(&mut e, AREA, section(0, 0), false, 0, &[0x23]);
    let cell = AREA + 0x200;
    e.mem.set_u8(cell + CELL_FLAGS, 1);
    let extra = AREA + 0x300;
    crate::commands::test_support::put_extra(
        &mut e,
        cell + CELL_EXTRA,
        extra,
        EXTRA_SEEN_DATA,
        section0,
    );
    let data = AREA + 0x400;
    let (colours, buffer) = (AREA + 0x1000, AREA + 0x500);
    e.mem.set_u32(data + 0x28, colours);
    e.mem.set_u32(data + 0x34, buffer);
    e.mem.set_u32(buffer + 0x20, 1);
    let mut s = MiniMapState {
        pc_curr_cell: cell,
        vertex_alpha: [0.0, 0.25, 0.5, 0.75, 1.0],
        ..MiniMapState::default()
    };
    s.shape_datas[4] = data;
    s.packed_coords[4] = pack(0, 0);
    for i in 0..17 * 17 {
        e.mem.set_f32(colours + 16 * i + 0xc, 9.0);
    }
    calc_vtx_alpha_by_seen_data(&mut e, &s, 4);
    assert_eq!(e.mem.u32(buffer + 0x20), 0);
    // Vertex (column c, row r) is the alpha at c * 17 + (16 - r).
    let seen: Vec<u32> = (0..17 * 17u32)
        .filter(|&i| e.mem.f32(colours + 16 * i + 0xc) != 0.0)
        .collect();
    let mut want: Vec<u32> = [(2, 1), (2, 2), (3, 1), (3, 2)]
        .iter()
        .map(|&(c, r)| c * 17 + (16 - r))
        .collect();
    want.sort_unstable();
    assert_eq!(seen, want);
    assert!(want
        .iter()
        .all(|&i| e.mem.f32(colours + 16 * i + 0xc) == 0.25));
}

/// `GetVtxAlphaPtr`: the place times 48 (truncated); shape `3 * (x >> 4) - (y >> 4) + 2`, vertex
/// `17 * (x & 15) + (y & 15)`, its colour's alpha (+0xC, 16 bytes a colour).
#[test]
fn a_place_finds_its_vertex_alpha() {
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    let mut s = MiniMapState::default();
    for i in 0..9u32 {
        let data = AREA + 0x100 * i;
        e.mem.set_u32(data + 0x28, 0x0700_0000 + 0x1_0000 * i);
        s.shape_datas[i as usize] = data;
    }
    // (0.5, 0.25) * 48 = (24, 12): shape 3 * 1 - 0 + 2 = 5, vertex 17 * 8 + 12 = 148.
    assert_eq!(
        vtx_alpha_ptr(&mut e, &s, [0.5, 0.25]),
        0x0705_0000 + 148 * 16 + 0xc
    );
    // (0.01, 0.99): (0, 47): shape 0 - 2 + 2 = 0, vertex 0 + 15.
    assert_eq!(
        vtx_alpha_ptr(&mut e, &s, [0.01, 0.99]),
        0x0700_0000 + 15 * 16 + 0xc
    );
}

/// A 5 x 5 grid of the loaded exterior cells centred on (0, 0) (`GridCellArray`: corner +4/+8,
/// size +0xC, cells +0x10), and its cells: each with a `SeenData` of `bits`, or seen whole (JIP's
/// flag +0x25 bit 0), or none.
enum Seen<'a> {
    Bits(&'a [u32]),
    Whole,
}

fn grid(e: &mut Engine, cells: &[((i32, i32), Seen)]) -> u32 {
    crate::commands::test_support::extra_stand_ins(e);
    let grid = AREA + 0x4000;
    let array = AREA + 0x4100;
    e.mem.set_u32(grid + 4, 0);
    e.mem.set_u32(grid + 8, 0);
    e.mem.set_u32(grid + 0xc, 5);
    e.mem.set_u32(grid + 0x10, array);
    for (n, ((x, y), seen_as)) in cells.iter().enumerate() {
        let cell = AREA + 0x5000 + 0x400 * n as u32;
        let (gx, gy) = ((x + 2) as u32, (y + 2) as u32);
        e.mem.set_u32(array + 4 * (gy + gx + (gx << 2)), cell);
        match seen_as {
            Seen::Whole => e.mem.set_u8(cell + CELL_SEEN_ALL, 1),
            Seen::Bits(bits) => {
                let data = seen(e, cell + 0x100, 0x0108_3fc4, bits);
                crate::commands::test_support::put_extra(
                    e,
                    cell + CELL_EXTRA,
                    cell + 0x80,
                    EXTRA_SEEN_DATA,
                    data,
                );
            }
        }
    }
    grid
}

/// Review round 1: an exterior vertex's level from the grid's cells. Inside the cell: its four
/// bits, or 1 + 3 seen whole (JIP's sentinel 1, never read as seen data). The last row: its own
/// two bits, then the next cell up's (+y) first row, 2 when that one is seen whole, none when it
/// is absent. The last column: the same from the next cell across (+x). The corner: one bit of
/// each of the three next cells, 1 for one seen whole.
#[test]
fn an_exterior_vertex_reads_the_grids_cells() {
    let level = |cells: &[((i32, i32), Seen)], column: u32, row: u32| {
        let mut e = engine();
        e.map(AREA, 0x1_0000);
        let g = grid(&mut e, cells);
        section_seen_level(&mut e, g, packed(column, row), pack(0, 0))
    };
    let own = |bits| ((0, 0), Seen::Bits(bits));
    assert_eq!(level(&[own(&[0x23, 0x24])], 3, 2), 2);
    assert_eq!(level(&[((0, 0), Seen::Whole)], 3, 2), 4, "1 + 3");
    assert_eq!(level(&[], 3, 2), 0, "no cell");
    // The last row: own 0xF3 and 0xF4; the cell up (0, 1).
    assert_eq!(level(&[own(&[0xf3, 0xf4])], 3, 15), 2);
    assert_eq!(
        level(&[own(&[0xf3, 0xf4]), ((0, 1), Seen::Whole)], 3, 15),
        4
    );
    assert_eq!(
        level(&[own(&[0xf3, 0xf4]), ((0, 1), Seen::Bits(&[0x3]))], 3, 15),
        3
    );
    assert_eq!(
        level(&[own(&[0xf3]), ((1, 0), Seen::Whole)], 3, 15),
        1,
        "not the cell across"
    );
    assert_eq!(
        level(&[((0, 0), Seen::Whole), ((0, 1), Seen::Whole)], 3, 15),
        4
    );
    // The last column: own 0x2F and 0x3F; the cell across (1, 0).
    assert_eq!(
        level(&[own(&[0x2f, 0x3f]), ((1, 0), Seen::Whole)], 15, 2),
        4
    );
    assert_eq!(
        level(&[own(&[0x2f, 0x3f]), ((1, 0), Seen::Bits(&[0x20]))], 15, 2),
        3
    );
    assert_eq!(
        level(&[own(&[0x2f]), ((0, 1), Seen::Whole)], 15, 2),
        1,
        "not the cell up"
    );
    // The corner: own 0xFF; (0, 1) bit 0xF, (1, 0) bit 0xF0, (1, 1) bit 0.
    assert_eq!(level(&[own(&[0xff])], 15, 15), 1);
    assert_eq!(
        level(
            &[
                own(&[0xff]),
                ((0, 1), Seen::Bits(&[0xf])),
                ((1, 0), Seen::Bits(&[0xf0])),
                ((1, 1), Seen::Bits(&[0]))
            ],
            15,
            15
        ),
        4
    );
    assert_eq!(
        level(
            &[
                own(&[]),
                ((0, 1), Seen::Whole),
                ((1, 0), Seen::Whole),
                ((1, 1), Seen::Whole)
            ],
            15,
            15
        ),
        3,
        "one each at the corner"
    );
}

/// Review round 1: in an exterior the vertex alphas come from the TES grid's cells (TES +0x8),
/// not from the player's cell's extra data: a centre cell seen whole makes an inner vertex level 4.
#[test]
fn an_exteriors_vertex_alphas_come_from_the_grid() {
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    if !e.mem.is_mapped(TES) {
        e.map(TES & !0xfff, 0x1000);
    }
    let g = grid(&mut e, &[((0, 0), Seen::Whole)]);
    let tes = AREA + 0x8000;
    e.mem.set_u32(TES, tes);
    e.mem.set_u32(tes + TES_GRID, g);
    let cell = AREA + 0x9000;
    let data = AREA + 0x9400;
    let colours = AREA + 0xa000;
    e.mem.set_u32(data + 0x28, colours);
    let mut s = MiniMapState {
        pc_curr_cell: cell,
        vertex_alpha: [0.0, 0.25, 0.5, 0.75, 1.0],
        ..MiniMapState::default()
    };
    s.shape_datas[4] = data;
    s.packed_coords[4] = pack(0, 0);
    calc_vtx_alpha_by_seen_data(&mut e, &s, 4);
    // Vertex (column 2, row 2): its alpha at 2 * 17 + (16 - 2).
    assert_eq!(e.mem.f32(colours + 16 * (2 * 17 + 14) + 0xc), 1.0);
}
