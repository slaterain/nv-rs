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
