//! UI Organizer's `<zoom>` for text tiles, its patches made in UIOLoad:
//! - `TileText::MakeNode`: its `TileTextApplyScaleHook` (jump at `00A221F8`) takes the place of
//!   the `Font::CreateText` call: the text is laid out at its wrap width and limit divided by the
//!   zoom, its shape scaled by the zoom, and the laid-out size multiplied back. (The other bytes
//!   UIO writes there, `0xA21B83` .. `0xA223D2`, move two of the function's locals so the hook can
//!   read the width and limit as a pair: no change of their own; the ten bytes at `0xA22244` are
//!   written only when they differ from the exe's, and they equal them.)
//! - `HUDMainMenu::Create`: the five hardcore texts no longer get a zoom (`jmp short` over each
//!   `SetFloat(zoom)` at `0x76F688`, `0x76F742`, `0x76F824`, `0x76F92D`, `0x76FA5E`), so with
//!   zoom honoured they keep the size the game drew them at.

use crate::Uio;
use engine_core::prelude::*;
use engine_core::sse;

/// `kTileValue_zoom`.
pub const ZOOM: u32 = 0xfb8;
/// 100.0 (`0x42C80000`): no scale.
const HUNDRED: u32 = 0x42c8_0000;
/// UIO's `kFlt1d100`.
const HUNDREDTH: f32 = 0.01;
/// UIO scales the wrap width and limit down only for a width below this (`cmp [ebp-0x58],0x800;
/// jnb`, unsigned).
const WIDTH_LIMIT: u32 = 0x800;
/// `Font::CreateText` (`00A12880`, `thiscall`, ten stack words) and the line separator `'\n'`
/// MakeNode passes it.
const FONT_CREATE_TEXT: u32 = 0x00a1_2880;
const LINE_SEPARATOR: u32 = 0xa;
/// `NiAVObject::m_fScale` (the shape's local scale).
const SHAPE_SCALE: u32 = 0x64;
/// The tile's trait array (`Tile::xTraits`, a `BSSimpleArray`): its buffer and its size.
const TRAITS_BUFFER: u32 = 0x14;
const TRAITS_SIZE: u32 = 0x18;

// Translated from UIO 2.30 (426d963), gameapi.cpp 163-197: GetTileValue (a binary search of the
// sorted trait array by id, unsigned compares, no lock).
/// The tile's value with id `trait_id`, or 0.
pub fn tile_value(e: &Engine, tile: u32, trait_id: u32) -> u32 {
    let buffer = e.mem.u32(tile + TRAITS_BUFFER);
    let (mut low, mut high) = (0u32, e.mem.u32(tile + TRAITS_SIZE));
    while low != high {
        let middle = low.wrapping_add(high) >> 1;
        let value = e.mem.u32(buffer.wrapping_add(middle.wrapping_mul(4)));
        let id = e.mem.u32(value);
        if id == trait_id {
            return value;
        }
        if id < trait_id {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    0
}

/// UIO's scale for a zoom value's float bits: `zoom * 0.01` for a positive zoom (its bits as a
/// signed word above 0: positive NaNs too) other than 100, else none.
pub fn scale_of(zoom: u32) -> Option<f32> {
    ((zoom as i32) > 0 && zoom != HUNDRED).then(|| sse::mul(f32::from_bits(zoom), HUNDREDTH))
}

/// The pair (wrap width, wrap limit) through `CVTDQ2PS`, `op` on each lane, then `CVTPS2DQ`.
fn scale_pair(e: &mut Engine, width_at: u32, limit_at: u32, op: impl Fn(f32) -> f32) {
    for at in [width_at, limit_at] {
        let scaled = op(e.mem.i32(at) as f32);
        e.mem.set_i32(at, crate::operators::cvt_round(scaled));
    }
}

// Translated from UIO 2.30 (426d963), ui_organizer.h 1279-1333: TileTextApplyScaleHook.
/// The seam at `00A221F8`: `[tile, font, text, width at, limit at, wrap lines, justify, colour
/// at, shape at, icons at]`. Makes the `Font::CreateText` call with the zoom applied around it and
/// answers 1; nothing before UIOLoad (the exe's call, unpatched).
pub fn text_scale(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if !e.ext::<Uio>().loaded {
        return None;
    }
    let (tile, width_at, limit_at, shape_at) = (a[0], a[3], a[4], a[8]);
    let value = tile_value(e, tile, ZOOM);
    let scale = if value == 0 {
        None
    } else {
        scale_of(e.mem.u32(value + 8))
    };
    if let Some(scale) = scale {
        if e.mem.u32(width_at) < WIDTH_LIMIT {
            scale_pair(e, width_at, limit_at, |x| sse::div(x, scale));
        }
    }
    e.call(
        FONT_CREATE_TEXT,
        &args![
            a[1],
            a[2],
            width_at,
            limit_at,
            0u32,
            a[5],
            a[6],
            LINE_SEPARATOR,
            a[7],
            shape_at,
            a[9]
        ],
    );
    if let Some(scale) = scale {
        let shape = e.mem.u32(shape_at);
        // (UIO writes through the shape with no test; `Font::CreateText` always makes one.)
        if shape != 0 {
            e.mem.set_f32(shape + SHAPE_SCALE, scale);
        }
        scale_pair(e, width_at, limit_at, |x| sse::mul(scale, x));
    }
    Some(1)
}

// Translated from UIO 2.30 (426d963), ui_organizer.h 1440-1445: the five `SafeWrite16(.., jmp
// short)` in HUDMainMenu::Create.
/// The seams at `0076F688`, `0076F742`, `0076F824`, `0076F92D`, `0076FA5E`: `[the tile's field]`.
/// Answers 1 (the zoom skipped) once UIOLoad has run.
pub fn hud_zoom(e: &mut Engine, _: &[u32]) -> Option<u32> {
    e.ext::<Uio>().loaded.then_some(1)
}

#[cfg(test)]
#[path = "zoom_tests.rs"]
mod tests;
