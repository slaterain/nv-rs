//! `UpdateMiniMap`'s fog of war (JIP LN NVSE's `functions_jip/jip_fn_minimap.h`): the seen level of
//! each of the nine local map shapes' 17 x 17 vertices, read from the game's seen data (an
//! exterior's cells in the grid, an interior's sections), as the vertex alphas JIP's shapes are
//! drawn with; the shapes a move can change (`DoSelectiveFOWUpdate`); a door marker's vertex.
//!
//! Threads: on the script's thread, with `UpdateMiniMap`'s state in hand.

use super::jip_fn_minimap::{MiniMapState, TES, TES_GRID};
use super::jip_fn_minimap_seams::{
    add_words, cell_at_coord, fow_update_mask, CELL_EXTRA, CELL_FLAGS, CELL_SEEN_ALL, EXTRA_DATA,
    EXTRA_SEEN_DATA, INT_NEXT, INT_SECTION, INT_SEEN_ALL, NEIGHBOURS,
};
use engine_core::prelude::*;
/// `s_vertexAlphaLevel`: the alpha of a seen level 0 to 4 (JIP's start values; `UpdateMiniMap`
/// sets the last four from the shader's alpha when the fog is turned on).
pub(crate) const VERTEX_ALPHA_START: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// `IntSeenData`'s vtable's low byte, which JIP tells an interior's seen data by (`0x01083FE4`).
const INT_SEEN_DATA_BYTE: u8 = 0xe4;
/// `SeenData` / `IntSeenData`: the 256 seen bits from +4.
const SEEN_BITS: u32 = 0x04;

/// One seen bit of a seen data (`bt [data + (bit >> 5) * 4 + 4], bit`).
fn bit(e: &Engine, data: u32, bit: u32) -> u8 {
    let word = e.mem.u32(data + ((bit & 0xff) >> 5) * 4 + SEEN_BITS);
    ((word >> (bit & 31)) & 1) as u8
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// GetSectionSeenData (asm).
/// The interior's seen data of a section, 0 for none: the list from `seen` by next (+0x28).
pub(crate) fn section_seen_data(e: &Engine, mut seen: u32, section: u16) -> u32 {
    loop {
        if e.mem.u16(seen + INT_SECTION) == section {
            return seen;
        }
        seen = e.mem.u32(seen + INT_NEXT);
        if seen == 0 {
            return 0;
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// GetGridCellSeenData (asm).
/// A loaded exterior cell's seen data: 0 for no cell or none, 1 when JIP marked it all seen
/// (+0x25 bit 0), else its `ExtraSeenData`'s data.
pub(crate) fn grid_cell_seen_data(e: &mut Engine, grid: u32, coord: u32) -> u32 {
    let cell = cell_at_coord(e, grid, coord);
    if cell == 0 {
        return 0;
    }
    if e.mem.u8(cell + CELL_SEEN_ALL) & 1 != 0 {
        return 1;
    }
    let extra = super::extra::get_by_type(e, cell + CELL_EXTRA, EXTRA_SEEN_DATA);
    if extra == 0 {
        0
    } else {
        e.mem.u32(extra + EXTRA_DATA)
    }
}

/// An interior section's number for a packed coordinate (`pshufb` with JIP's bytes 2, 0): x's low
/// byte, then y's.
fn section_of(coord: u32) -> u16 {
    let x = (coord >> 16) as u8;
    let y = coord as u8;
    u16::from(x) | (u16::from(y) << 8)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// GetSectionSeenLevel (asm).
/// A vertex's seen level, 0 to 4: how many of the (up to) four seen bits around it are set. The
/// vertex is `packed` (column in the low byte, row in the next, each `& 0xF`) of the cell at
/// `coord`; on the last row or column the bits come from the next cells (`coord` + (0, 1), (1,
/// 0), (1, 1)), and a neighbour seen whole counts 2 (1 for the corner). `seen_info` is the cell
/// grid for an exterior, an interior's seen data else.
pub(crate) fn section_seen_level(e: &mut Engine, seen_info: u32, packed: u32, coord: u32) -> u32 {
    let packed = packed & 0xf0f;
    let (lo, hi) = (packed & 0xff, packed >> 8);
    // `esi`: the bit; `bh`: 1 on the last row, 2 on the last column, 3 both.
    let esi = (lo << 4) | hi;
    let edge = u32::from(lo == 0xf) | (u32::from(hi == 0xf) << 1);
    // The four coordinates: the cell, (0, 1), (1, 0), (1, 1) (`paddw`).
    let lanes = [
        coord,
        add_words(coord, 0x0000_0001),
        add_words(coord, 0x0001_0000),
        add_words(coord, 0x0001_0001),
    ];
    let mut level: u32;
    let own: u32;
    if e.mem.u8(seen_info) == INT_SEEN_DATA_BYTE {
        let data = section_seen_data(e, seen_info, section_of(lanes[0]));
        level = if data == 0 {
            0
        } else {
            u32::from(bit(e, data, esi))
        };
        own = data;
        match edge {
            0 => return level + both0(e, own, esi),
            1 => {
                if own != 0 {
                    level += u32::from(bit(e, own, esi + 1));
                }
                let next = section_seen_data(e, seen_info, section_of(lanes[1]));
                if next == 0 {
                    return level;
                }
                if e.mem.u8(next + INT_SEEN_ALL) != 0 {
                    return level + 2;
                }
                return level + both1(e, next, esi);
            }
            2 => {
                if own != 0 {
                    level += u32::from(bit(e, own, esi + 0x10));
                }
                let next = section_seen_data(e, seen_info, section_of(lanes[2]));
                if next == 0 {
                    return level;
                }
                if e.mem.u8(next + INT_SEEN_ALL) != 0 {
                    return level + 2;
                }
                return level + both2(e, next, esi);
            }
            _ => {
                let d = section_seen_data(e, seen_info, section_of(lanes[1]));
                if d != 0 {
                    level += u32::from(bit(e, d, esi & 0xf));
                }
                let d = section_seen_data(e, seen_info, section_of(lanes[2]));
                if d != 0 {
                    level += u32::from(bit(e, d, esi & 0xf0));
                }
                let d = section_seen_data(e, seen_info, section_of(lanes[3]));
                if d != 0 {
                    level += u32::from(bit(e, d, 0));
                }
                return level;
            }
        }
    }
    // An exterior: the cells of the grid.
    let got = grid_cell_seen_data(e, seen_info, lanes[0]);
    let all_seen = got == 1;
    let own = if got > 1 { got } else { 0 };
    level = if all_seen {
        1
    } else if own != 0 {
        u32::from(bit(e, own, esi))
    } else {
        0
    };
    match edge {
        0 => {
            if all_seen {
                level + 3
            } else {
                level + both0(e, own, esi)
            }
        }
        1 => {
            if all_seen {
                level += 1;
            } else if own != 0 {
                level += u32::from(bit(e, own, esi + 1));
            }
            match grid_cell_seen_data(e, seen_info, lanes[1]) {
                0 => level,
                1 => level + 2,
                next => level + both1(e, next, esi),
            }
        }
        2 => {
            if all_seen {
                level += 1;
            } else if own != 0 {
                level += u32::from(bit(e, own, esi + 0x10));
            }
            match grid_cell_seen_data(e, seen_info, lanes[2]) {
                0 => level,
                1 => level + 2,
                next => level + both2(e, next, esi),
            }
        }
        _ => {
            match grid_cell_seen_data(e, seen_info, lanes[1]) {
                0 => {}
                1 => level += 1,
                d => level += u32::from(bit(e, d, esi & 0xf)),
            }
            match grid_cell_seen_data(e, seen_info, lanes[2]) {
                0 => {}
                1 => level += 1,
                d => level += u32::from(bit(e, d, esi & 0xf0)),
            }
            match grid_cell_seen_data(e, seen_info, lanes[3]) {
                0 => {}
                1 => level += 1,
                d => level += u32::from(bit(e, d, 0)),
            }
            level
        }
    }
}

/// JIP's `case0Both`: the cell's own bits right, below and below right (none without data).
fn both0(e: &Engine, own: u32, esi: u32) -> u32 {
    if own == 0 {
        return 0;
    }
    u32::from(bit(e, own, esi + 0x10))
        + u32::from(bit(e, own, esi + 1))
        + u32::from(bit(e, own, esi + 0x11))
}

/// JIP's `case1Both`: the next row's cell's first row, this column and the next.
fn both1(e: &Engine, next: u32, esi: u32) -> u32 {
    u32::from(bit(e, next, esi & 0xf)) + u32::from(bit(e, next, (esi + 1) & 0xf))
}

/// JIP's `case2Both`: the next column's cell's first column, this row and the next.
fn both2(e: &Engine, next: u32, esi: u32) -> u32 {
    u32::from(bit(e, next, esi & 0xf0)) + u32::from(bit(e, next, (esi + 0x10) & 0xf0))
}

/// `NiTriShapeData`'s colours (+0x28; `NiColorA`, alpha at +0xC) and the renderer's buffer data
/// (+0x34, whose +0x20 JIP zeroes so the colours are sent again).
const DATA_COLOURS: u32 = 0x28;
const DATA_BUFFER: u32 = 0x34;
const BUFFER_DIRTY: u32 = 0x20;
const COLOUR_ALPHA: u32 = 0x0c;
const COLOUR_SIZE: u32 = 0x10;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// PackCurrGridCoords (asm).
/// The nine cells' packed coordinates around `coord` (x - 1 to x + 1, each y - 1 to y + 1).
pub(crate) fn pack_curr_grid_coords(coord: u32) -> [u32; 9] {
    NEIGHBOURS.map(|n| add_words(coord, n))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// CalcVtxAlphaBySeenData (asm).
/// The shape `index`'s 17 x 17 vertex alphas from the seen levels: an exterior's from the cell
/// grid, an interior's (the player's cell) from its seen data, nothing when it has none. The
/// vertices run by column (0 to 16, the last the next cell's first), each from row 16 down to 0.
pub(crate) fn calc_vtx_alpha_by_seen_data(e: &mut Engine, s: &MiniMapState, index: usize) {
    let (cell, data, coord, levels) = (
        s.pc_curr_cell,
        s.shape_datas[index],
        s.packed_coords[index],
        s.vertex_alpha,
    );
    let mut seen_info = e.mem.u32(e.mem.u32(TES) + TES_GRID);
    if e.mem.u8(cell + CELL_FLAGS) & 1 != 0 {
        let extra = super::extra::get_by_type(e, cell + CELL_EXTRA, EXTRA_SEEN_DATA);
        if extra == 0 {
            return;
        }
        seen_info = e.mem.u32(extra + EXTRA_DATA);
        if seen_info == 0 {
            return;
        }
    }
    let buffer = e.mem.u32(data + DATA_BUFFER);
    if buffer != 0 {
        e.mem.set_u32(buffer + BUFFER_DIRTY, 0);
    }
    let mut at = e.mem.u32(data + DATA_COLOURS) + COLOUR_ALPHA;
    for column in 0..=16u32 {
        let base = if column == 16 {
            add_words(coord, 0x0001_0000)
        } else {
            coord
        };
        for row in (0..=16u32).rev() {
            let place = if row == 16 { add_words(base, 1) } else { base };
            let level = section_seen_level(e, seen_info, (column << 8) | row, place);
            e.mem.set_f32(at, levels[level as usize]);
            at += COLOUR_SIZE;
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// DoSelectiveFOWUpdate (asm).
/// The vertex alphas of the shapes a place's seen bits can change (`GetFOWUpdateMask`), lowest
/// first.
pub(crate) fn do_selective_fow_update(e: &mut Engine, s: &MiniMapState, pos: [f32; 2]) {
    let mask = fow_update_mask(super::cvttss2si(pos[0]), super::cvttss2si(pos[1]));
    for index in 0..9 {
        if mask & (1 << index) != 0 {
            calc_vtx_alpha_by_seen_data(e, s, index);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h: GetVtxAlphaPtr
// (asm).
/// Where the vertex alpha nearest a place on the local map (0 to 3 across the three shapes) is:
/// the shape `3 * (x >> 4) - (y >> 4) + 2` of the place times 48, vertex `17 * (x & 15) + (y &
/// 15)`.
pub(crate) fn vtx_alpha_ptr(e: &mut Engine, s: &MiniMapState, pos_mult: [f32; 2]) -> u32 {
    let x = super::cvttss2si(pos_mult[0] * 48.0) as u32 & 0xffff;
    let y = super::cvttss2si(pos_mult[1] * 48.0) as u32 & 0xffff;
    let shape = (3 * (x >> 4)).wrapping_sub(y >> 4).wrapping_add(2);
    let vertex = ((x & 0xf) * 17 + (y & 0xf)) << 4;
    let data = s.shape_datas.get(shape as usize).copied().unwrap_or(0);
    e.mem.u32(data + DATA_COLOURS).wrapping_add(vertex) + COLOUR_ALPHA
}

#[cfg(test)]
#[path = "jip_fn_minimap_fog_tests.rs"]
mod tests;
