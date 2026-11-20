//! JIP MiniMap's seven code patches (`Cmd_InitMiniMap_Execute`'s last lines), as seams of the
//! translated functions, answered once `InitMiniMap` has written them
//! ([`super::jip_fn_minimap::patched`]; until then, and without JIP, the game is unchanged):
//!
//! - `SafeWrite16(0x452736, 0x7705)`: `TES::UpdateCurrentGridCell`'s small-grid test (`cmp [eax],
//!   3; jnz` becomes `cmp [eax], 5; ja`): the smaller cell extent for every `uGridsToLoad` up to 5
//!   ([`GRID_EXTENT`], words: the setting; answered: the test's result);
//! - `SafeWrite8(0x555C20, 0xC3)`: the game's own update of the cells' seen bits returns at once
//!   ([`SEEN_BITS_FUNCTION`], answered: skipped);
//! - `WriteRelCall(0x9438F6, UpdateCellsSeenBitsHook)`: `PlayerCharacter::Update` calls JIP's own
//!   update of the seen bits there instead ([`SEEN_BITS_CALL`], answered: done);
//! - `WriteRelCall(0x77951E, DiscoverLocationHook)`: `HUDMainMenu::UpdateCompass`' name of a
//!   newly found location: JIP marks the discovery, lets go of the minimap's rendered markers of
//!   the marker's cell, and gives the same name ([`DISCOVER_LOCATION`], words: the marker data and
//!   the player's marker record; answered: the name);
//! - `WriteRelJump(0x60F13A, SetQuestTargetsHook)` and `WriteRelCall(0x952C69 / 0x952F6B,
//!   UpdatePlacedMarkerHook)`: the quest targets or the placed marker changed: JIP's flag set, the
//!   game goes on as it does ([`QUEST_TARGETS`], [`PLACED_MARKER_CHECK`], [`PLACED_MARKER_SET`]; no
//!   answer);
//! - `WriteRelJump(0xE74C3C, RendererRecreateHook)`: the renderer's device reset: the local maps'
//!   texture format is set to 0 (the device's own) before the game's reset goes on
//!   ([`RENDERER_RESET`]; no answer).
//!
//! JIP's code only writes the patches; the port's rows are in with JIP's `INSTALLED` and each
//! handler answers only after `InitMiniMap`.
//!
//! Threads: every handler runs on the thread of the game code that asks: the main thread (player
//! update, HUD, quests, the renderer's device check). The flags are atomics, as JIP's statics; the
//! state's lock is not held across a game call.

use super::jip_fn_minimap::{
    free_cell_map_markers, mini_map, patched, state, world_map_pos_mults, TES, TES_GRID,
};
use crate::game_patches::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use std::sync::atomic::Ordering;

/// The seams (rows in `engine-core/src/extend/patch_rows/cmds.rs`).
pub const GRID_EXTENT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0045_2737);
pub const SEEN_BITS_FUNCTION: SiteSlot = SiteSlot::new(Owner::Jip, 0x0055_5c20);
pub const QUEST_TARGETS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0060_f13a);
pub const DISCOVER_LOCATION: SiteSlot = SiteSlot::new(Owner::Jip, 0x0077_951e);
pub const SEEN_BITS_CALL: SiteSlot = SiteSlot::new(Owner::Jip, 0x0094_38f6);
pub const PLACED_MARKER_CHECK: SiteSlot = SiteSlot::new(Owner::Jip, 0x0095_2c69);
pub const PLACED_MARKER_SET: SiteSlot = SiteSlot::new(Owner::Jip, 0x0095_2f6b);
pub const RENDERER_RESET: SiteSlot = SiteSlot::new(Owner::Jip, 0x00e7_4c3c);

/// The minimap's seams: in with JIP's `INSTALLED`; each answers only after `InitMiniMap`.
pub const AREA: Area = Area {
    name: "minimap",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[
        GRID_EXTENT.addr(),
        SEEN_BITS_FUNCTION.addr(),
        QUEST_TARGETS.addr(),
        DISCOVER_LOCATION.addr(),
        SEEN_BITS_CALL.addr(),
        PLACED_MARKER_CHECK.addr(),
        PLACED_MARKER_SET.addr(),
        RENDERER_RESET.addr(),
    ],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(GRID_EXTENT, grid_extent);
    e.on_site(SEEN_BITS_FUNCTION, seen_bits_function);
    e.on_site(QUEST_TARGETS, quest_targets);
    e.on_site(DISCOVER_LOCATION, discover_location);
    e.on_site(SEEN_BITS_CALL, seen_bits_call);
    e.on_site(PLACED_MARKER_CHECK, placed_marker);
    e.on_site(PLACED_MARKER_SET, placed_marker);
    e.on_site(RENDERER_RESET, renderer_reset);
}

/// `cmp dword ptr [eax], 5; ja`: the small-grid extent for a setting of at most 5 (unsigned).
fn grid_extent(e: &mut Engine, w: &[u32]) -> Option<u32> {
    patched(e).then(|| u32::from(w[0] <= 5))
}

/// `0x555C20: ret`.
fn seen_bits_function(e: &mut Engine, _: &[u32]) -> Option<u32> {
    patched(e).then_some(0)
}

/// `SetQuestTargetsHook`: `s_updateQuestTargets = true`, then the game's own code (JIP's hook
/// stores `this + 0x4C` itself, the value the game's call answers).
fn quest_targets(e: &mut Engine, _: &[u32]) -> Option<u32> {
    if patched(e) {
        mini_map(e)
            .update_quest_targets
            .store(true, Ordering::Release);
    }
    None
}

/// `UpdatePlacedMarkerHook`: `s_updateQuestTargets = true`, then the game's path building.
fn placed_marker(e: &mut Engine, _: &[u32]) -> Option<u32> {
    quest_targets(e, &[])
}

/// `RendererRecreateHook`: `s_mmTextureParams.d3dFormat = 0`, then the game's device reset.
fn renderer_reset(e: &mut Engine, _: &[u32]) -> Option<u32> {
    if patched(e) {
        mini_map(e).texture_format.store(0, Ordering::Release);
    }
    None
}

/// The empty name (`0x1011584`).
const EMPTY_TEXT: u32 = 0x0101_1584;
/// `MapMarkerData`'s name text (+4: `fullName.name.m_data`); the player's marker record's
/// reference (+4).
const MARKER_NAME: u32 = 0x04;
const RECORD_REFERENCE: u32 = 0x04;
/// A reference's position (+0x30).
const REF_POSITION: u32 = 0x30;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// DiscoverLocationHook.
/// `s_discoveredLocation = true`; when the minimap has rendered markers, those of the found
/// marker's cell (by the root world's dimensions) let go; the marker's name, or the empty text.
fn discover_location(e: &mut Engine, w: &[u32]) -> Option<u32> {
    if !patched(e) {
        return None;
    }
    let (data, record) = (w[0], w[1]);
    mini_map(e)
        .discovered_location
        .store(true, Ordering::Release);
    if !state(e).rendered_markers.is_empty() {
        let marker = e.mem.u32(record + RECORD_REFERENCE);
        let pos = [
            e.mem.f32(marker + REF_POSITION),
            e.mem.f32(marker + REF_POSITION + 4),
        ];
        let dims = state(e).root_dims;
        let mut coord = 0;
        world_map_pos_mults(pos, &dims, Some(&mut coord));
        free_cell_map_markers(e, coord);
    }
    let name = e.mem.u32(data + MARKER_NAME);
    Some(if name != 0 { name } else { EMPTY_TEXT })
}

// ---- the seen bits (fog of war) ----

/// `g_thePlayer` (`*011DEA3C`): its cell (+0x40) and position (+0x30); the game's last seen-bits
/// position (`011CA208`, two floats) and tracked cell (`*011E0BB0`).
pub(crate) const PLAYER: u32 = 0x011d_ea3c;
pub(crate) const PLAYER_CELL: u32 = 0x40;
const PLAYER_POSITION: u32 = 0x30;
const LAST_POSITION: u32 = 0x011c_a208;
const TRACKED_CELL: u32 = 0x011e_0bb0;
/// A cell's flags (+0x24, bit 0 interior), JIP's "all seen" flag (+0x25, bit 0), extra data
/// (+0x28), id (+0xC); `TES`'s grid centre (+0x2C, two words); `GridCellArray`: its corner (+4,
/// +8), size (+0xC), cells (+0x10).
pub(crate) const CELL_FLAGS: u32 = 0x24;
pub(crate) const CELL_SEEN_ALL: u32 = 0x25;
pub(crate) const CELL_EXTRA: u32 = 0x28;
pub(crate) const CELL_ID: u32 = 0x0c;
const TES_CENTRE: u32 = 0x2c;
/// `kXData_ExtraSeenData`, `kXData_ExtraNorthRotation`; the extra's data (+0xC); the north
/// rotation's angle (+0xC).
pub(crate) const EXTRA_SEEN_DATA: u32 = 0x05;
const EXTRA_NORTH_ROTATION: u32 = 0x43;
pub(crate) const EXTRA_DATA: u32 = 0x0c;
/// `kVtbl_ExtraSeenData`, `kVtbl_SeenData`, `kVtbl_IntSeenData`.
const VTBL_EXTRA_SEEN_DATA: u32 = 0x0101_4294;
const VTBL_SEEN_DATA: u32 = 0x0108_3fc4;
const VTBL_INT_SEEN_DATA: u32 = 0x0108_3fe4;
/// `IntSeenData`: its section (+0x24, `i16`), "all seen" byte (+0x26), next (+0x28).
pub(crate) const INT_SECTION: u32 = 0x24;
pub(crate) const INT_SEEN_ALL: u32 = 0x26;
pub(crate) const INT_NEXT: u32 = 0x28;
/// The calls: `BaseExtraList::AddExtra` (`0040FF60`), `RemoveByType` (`00410140`), the change
/// flags (`008457B0`, `00845A80`: `thiscall` on `*BGSSaveLoadGame`, the form id and a flag),
/// `MarkModified`'s `AddChange` (`0084A690`).
const ADD_EXTRA: u32 = 0x0040_ff60;
const REMOVE_BY_TYPE: u32 = 0x0041_0140;
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
const ADD_CHANGE_FLAGS: u32 = 0x0084_57b0;
const SET_CHANGE_FLAGS: u32 = 0x0084_5a80;
const ADD_CHANGE: u32 = 0x0084_a690;
/// `kUpdSeenConsts`: the interior offset (0x800, 0x800), the squared move that updates (6400),
/// the shuffle bytes `0x80800002`, then the nine neighbour offsets (x high word, y low).
const INTERIOR_OFFSET: i32 = 0x800;
const MIN_MOVE_SQUARED: f32 = 6400.0;
pub(crate) const NEIGHBOURS: [u32; 9] = [
    0xffff_ffff,
    0xffff_0000,
    0xffff_0001,
    0x0000_ffff,
    0x0000_0000,
    0x0000_0001,
    0x0001_ffff,
    0x0001_0000,
    0x0001_0001,
];

/// Each word's two halves added as 16-bit numbers (`paddw`).
pub(crate) fn add_words(a: u32, b: u32) -> u32 {
    let lo = (a as u16).wrapping_add(b as u16);
    let hi = ((a >> 16) as u16).wrapping_add((b >> 16) as u16);
    (u32::from(hi) << 16) | u32::from(lo)
}

/// Each half shifted left by 4 (`psllw 4`).
fn shift_words(a: u32) -> u32 {
    let lo = (a as u16) << 4;
    let hi = ((a >> 16) as u16) << 4;
    (u32::from(hi) << 16) | u32::from(lo)
}

/// Two 32-bit lanes packed as `pshuflw 2` packs them: JIP's coordinate, x's low half in the high
/// word, y's in the low.
pub(crate) fn pack(x: i32, y: i32) -> u32 {
    ((x as u32 & 0xffff) << 16) | (y as u32 & 0xffff)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h: GetNorthRotation
// (asm).
/// For an interior with a north rotation: `s_cellNorthRotation = -angle` and its sine and cosine
/// as JIP's rotation lanes (`(sin, cos, cos, -sin)` after its shuffles); else 0. The lanes it answers
/// (JIP's `xmm0`, which `UpdateMiniMap` puts in its camera's rotation): those, or `(0, 1, 1, 0)` with
/// no rotation.
pub(crate) fn north_rotation(e: &mut Engine, cell: u32) -> [f32; 4] {
    let extra = if e.mem.u8(cell + CELL_FLAGS) & 1 == 0 {
        0
    } else {
        super::extra::get_by_type(e, cell + CELL_EXTRA, EXTRA_NORTH_ROTATION)
    };
    if extra == 0 {
        mini_map(e).north_rotation.store(0, Ordering::Release);
        return [0.0, 1.0, 1.0, 0.0];
    }
    let angle = -e.mem.f32(extra + EXTRA_DATA);
    let m = mini_map(e);
    m.north_rotation.store(angle.to_bits(), Ordering::Release);
    let pi_d2 = super::ln_fn_math::FLT_PI_D2;
    let [sin, cos, _] = super::ln_fn_math::cos_v3([pi_d2 - angle, angle, 0.0]);
    // `GetSinCos` gives (sin, cos) in the low lanes; `unpcklpd` doubles them, `xorps` flips the
    // first's sign, `shufps 0x16` picks lanes 2, 1, 1, 0: (sin, cos, cos, -sin).
    let lanes = [sin, cos, cos, -sin];
    for (slot, v) in m.north_rotation_mods.iter().zip(lanes) {
        slot.store(v.to_bits(), Ordering::Release);
    }
    lanes
}

/// The lanes JIP's interior code multiplies by: the position doubled per lane (`unpcklps`), times
/// the rotation lanes, shuffled (`0x27`) and added in pairs (`haddps`).
pub(crate) fn rotate_interior(e: &mut Engine, pos: [f32; 2]) -> [f32; 2] {
    let m = mini_map(e);
    if m.north_rotation.load(Ordering::Acquire) == 0 {
        return pos;
    }
    let r: Vec<f32> = m
        .north_rotation_mods
        .iter()
        .map(|v| f32::from_bits(v.load(Ordering::Acquire)))
        .collect();
    // unpcklps xmm4, xmm4: (x, x, y, y); times the lanes; shufps 0x27: lanes 3, 1, 2, 0.
    let p = [pos[0] * r[0], pos[0] * r[1], pos[1] * r[2], pos[1] * r[3]];
    let s = [p[3], p[1], p[2], p[0]];
    [s[0] + s[1], s[2] + s[3]]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h: GetFOWUpdateMask
// (asm).
/// Which of the nine neighbour cells a place updates: by whether its x and y within the cell
/// (`& 0xFFF`) are past 0x4FF and 0xAFF, JIP's table of bit masks.
pub(crate) fn fow_update_mask(x: i32, y: i32) -> u32 {
    const MASKS: [u16; 16] = [
        0x001b, 0x0012, 0x0012, 0x0036, 0x0018, 0x0010, 0x0010, 0x0030, 0x0030, 0x0030, 0x0030,
        0x0030, 0x00d8, 0x0090, 0x0090, 0x01b0,
    ];
    // `pshufd 5` gives the lanes (y, y, x, x), compared (signed) with (0x4FF, 0xAFF, 0x4FF,
    // 0xAFF); `movmskps`: bit 0 y > 0x4FF, bit 1 y > 0xAFF, bit 2 x > 0x4FF, bit 3 x > 0xAFF.
    let (tx, ty) = (x & 0xfff, y & 0xfff);
    let bits = u32::from(ty > 0x4ff)
        | u32::from(ty > 0xaff) << 1
        | u32::from(tx > 0x4ff) << 2
        | u32::from(tx > 0xaff) << 3;
    u32::from(MASKS[bits as usize])
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameData.cpp: GridCellArray::GetCellAtCoord
// (asm).
/// The loaded cell at the packed coordinate, 0 outside the grid. JIP's index is `x + (x << (size
/// >> 1)) + y`, which is `5x + y` for the five-cell grid it is made for.
pub(crate) fn cell_at_coord(e: &Engine, grid: u32, coord: u32) -> u32 {
    let y = i32::from(coord as u16 as i16);
    let x = (coord as i32) >> 16;
    let size = e.mem.u32(grid + 0xc);
    let half = size >> 1;
    let gx = (x.wrapping_sub(e.mem.u32(grid + 4) as i32) as u32).wrapping_add(half);
    if gx >= size {
        return 0;
    }
    let gy = (y.wrapping_sub(e.mem.u32(grid + 8) as i32) as u32).wrapping_add(half);
    if gy >= size {
        return 0;
    }
    let index = gy.wrapping_add(gx).wrapping_add(gx.wrapping_shl(half & 31));
    e.mem
        .u32(e.mem.u32(grid + 0x10).wrapping_add(index.wrapping_mul(4)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h: AddExtraSeenData
// (asm).
/// The cell's seen data, made when it has none: an `ExtraSeenData` (0x10 bytes, type 5) when the
/// cell has no extra, and its data: an exterior's `SeenData` (0x24 bytes, its bits zeroed), an
/// interior's `IntSeenData` (0x2C bytes, section `section`), added to the cell's extra data and the
/// cell marked changed (0x80000000).
fn add_extra_seen_data(e: &mut Engine, cell: u32, section: u16) -> u32 {
    let mut extra = super::extra::get_by_type(e, cell + CELL_EXTRA, EXTRA_SEEN_DATA);
    if extra != 0 {
        let data = e.mem.u32(extra + EXTRA_DATA);
        if data != 0 {
            return data;
        }
    } else {
        extra = super::heap_alloc(e, 0x10);
        e.mem.set_u32(extra, VTBL_EXTRA_SEEN_DATA);
        e.mem.set_u32(extra + 4, EXTRA_SEEN_DATA);
        e.mem.set_u32(extra + 8, 0);
    }
    let interior = e.mem.u8(cell + CELL_FLAGS) & 1 != 0;
    let data = if interior {
        let d = super::heap_alloc(e, 0x2c);
        e.mem.set_u32(d, VTBL_INT_SEEN_DATA);
        e.mem.set_u32(d + 0x24, 0);
        e.mem.set_u32(d + 0x28, 0);
        e.mem.set_u16(d + INT_SECTION, section);
        d
    } else {
        let d = super::heap_alloc(e, 0x24);
        e.mem.set_u32(d, VTBL_SEEN_DATA);
        d
    };
    for i in 0..8 {
        e.mem.set_u32(data + 4 + 4 * i, 0);
    }
    e.mem.set_u32(extra + EXTRA_DATA, data);
    e.call(ADD_EXTRA, &args![cell + CELL_EXTRA, extra]);
    let sl = e.mem.u32(SAVE_LOAD_GAME);
    e.call(ADD_CHANGE, &args![sl, cell, 0x8000_0000u32, 0u32]);
    data
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h: AddIntSeenData
// (asm).
/// The interior seen data of `section` in the list from `seen`, a new one (its bits zeroed)
/// appended when there is none.
fn add_int_seen_data(e: &mut Engine, seen: u32, section: u16) -> u32 {
    let mut d = seen;
    loop {
        if e.mem.u16(d + INT_SECTION) == section {
            return d;
        }
        let next = e.mem.u32(d + INT_NEXT);
        if next == 0 {
            break;
        }
        d = next;
    }
    let n = super::heap_alloc(e, 0x2c);
    e.mem.set_u32(n, VTBL_INT_SEEN_DATA);
    for i in 0..10 {
        e.mem.set_u32(n + 4 + 4 * i, 0);
    }
    e.mem.set_u16(n + INT_SECTION, section);
    e.mem.set_u32(d + INT_NEXT, n);
    n
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h: UpdateSeenBits
// (asm).
/// The 16 x 16 seen bits (from +4) around the place `rel` (x high word, y low, in 256ths of a
/// cell, relative to the cell's corner) set within JIP's round brush (by the row's distance from
/// the centre, 0 to 4: half widths 4 3 3 2 0, the bytes `04 03 03 02` and the 0 pushed before
/// them);
/// `s_updateFogOfWar` set when a bit was new. Whether every bit is set.
fn update_seen_bits(e: &mut Engine, seen: u32, rel: u32) -> bool {
    let rx = (rel as i32) >> 16;
    let ry = i32::from(rel as u16 as i16);
    let widths = [4u8, 3, 3, 2, 0];
    let bits = seen + 4;
    let mut changed = false;
    for row in (0..16u8).rev() {
        let dy = (i32::from(row) + rx).unsigned_abs();
        if dy > 4 {
            continue;
        }
        let width = u32::from(widths[dy as usize]);
        for column in (0..16u8).rev() {
            let dx = (i32::from(column) + ry).unsigned_abs();
            if dx > width {
                continue;
            }
            let bit = u32::from(column) << 4 | u32::from(row);
            let word = bits + 4 * (bit >> 5 & 7);
            let old = e.mem.u32(word);
            let mask = 1u32 << (bit & 31);
            if old & mask == 0 {
                e.mem.set_u32(word, old | mask);
                changed = true;
            }
        }
    }
    if changed {
        mini_map(e).update_fog_of_war.store(true, Ordering::Release);
    }
    (0..8).all(|i| e.mem.u32(bits + 4 * i) == u32::MAX)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// UpdateCellsSeenBitsHook (asm).
/// JIP's own update of the seen bits, in place of the game's: when the player is in a cell and
/// has moved 80 units (squared 6400) since the last update in the same cell (a new cell always:
/// its north rotation read first), the place is kept; outside, each of the nine grid cells the
/// place reaches (JIP's mask) that is loaded and not all seen gets its seen data and the bits
/// around the place, and a cell whose bits are all set loses its seen data, gets JIP's "all
/// seen" flag and its change flags (2 added, then 0x80000000); inside, the interior's seen data
/// sections around the rotated place get theirs, a section marked all seen when every bit is set.
fn seen_bits_call(e: &mut Engine, _: &[u32]) -> Option<u32> {
    if !patched(e) {
        return None;
    }
    update_cells_seen_bits(e);
    Some(0)
}

pub(crate) fn update_cells_seen_bits(e: &mut Engine) {
    let player = e.mem.u32(PLAYER);
    let cell = e.mem.u32(player + PLAYER_CELL);
    if cell == 0 {
        return;
    }
    let pos = [
        e.mem.f32(player + PLAYER_POSITION),
        e.mem.f32(player + PLAYER_POSITION + 4),
    ];
    if e.mem.u32(TRACKED_CELL) == cell {
        let last = [e.mem.f32(LAST_POSITION), e.mem.f32(LAST_POSITION + 4)];
        let (dx, dy) = (last[0] - pos[0], last[1] - pos[1]);
        // `haddps` adds the squares; `comiss ...; jb`: below 6400 (or NaN) nothing happens.
        let moved = dx * dx + dy * dy;
        if moved.is_nan() || moved < MIN_MOVE_SQUARED {
            return;
        }
    } else {
        north_rotation(e, cell);
    }
    e.mem.set_f32(LAST_POSITION, pos[0]);
    e.mem.set_f32(LAST_POSITION + 4, pos[1]);
    let cvt = crate::game_patches::gd::cvtss2si;
    if e.mem.u8(cell + CELL_FLAGS) & 1 == 0 {
        let (px, py) = (cvt(pos[0]), cvt(pos[1]));
        let mask = fow_update_mask(px, py);
        let tes = e.mem.u32(TES);
        let (gx, gy) = (
            e.mem.u32(tes + TES_CENTRE) as i32,
            e.mem.u32(tes + TES_CENTRE + 4) as i32,
        );
        let centre = pack(gx, gy);
        let rel = pack(
            gx.wrapping_shl(12).wrapping_sub(px) >> 8,
            gy.wrapping_shl(12).wrapping_sub(py) >> 8,
        );
        for (i, &offset) in NEIGHBOURS.iter().enumerate() {
            if mask & (1 << i) == 0 {
                continue;
            }
            let grid = e.mem.u32(tes + TES_GRID);
            let c = cell_at_coord(e, grid, add_words(offset, centre));
            if c == 0 || e.mem.u8(c + CELL_SEEN_ALL) & 1 != 0 {
                continue;
            }
            let seen = add_extra_seen_data(e, c, 0xffff);
            if !update_seen_bits(e, seen, add_words(shift_words(offset), rel)) {
                continue;
            }
            e.call(REMOVE_BY_TYPE, &args![c + CELL_EXTRA, EXTRA_SEEN_DATA]);
            let flags = e.mem.u8(c + CELL_SEEN_ALL);
            e.mem.set_u8(c + CELL_SEEN_ALL, flags | 1);
            let id = e.mem.u32(c + CELL_ID);
            let changes = e.mem.u32(e.mem.u32(SAVE_LOAD_GAME));
            e.call(ADD_CHANGE_FLAGS, &args![changes, id, 2u32]);
            e.call(SET_CHANGE_FLAGS, &args![changes, id, 0x8000_0000u32]);
        }
        return;
    }
    let p = rotate_interior(e, pos);
    let (px, py) = (cvt(p[0]) - INTERIOR_OFFSET, cvt(p[1]) - INTERIOR_OFFSET);
    let mask = fow_update_mask(px, py);
    let (sx, sy) = (px >> 12, py >> 12);
    let section_base = pack(sx, sy);
    let rel = pack(
        sx.wrapping_shl(12).wrapping_sub(px) >> 8,
        sy.wrapping_shl(12).wrapping_sub(py) >> 8,
    );
    let root = add_extra_seen_data(e, cell, 0xffff);
    for (i, &offset) in NEIGHBOURS.iter().enumerate() {
        if mask & (1 << i) == 0 {
            continue;
        }
        // `pshufb` with `0x80800002`: the low byte is the x word's low byte, the high byte the y
        // word's (`(y & 0xFF) << 8 | (x & 0xFF)`).
        let c = add_words(offset, section_base);
        let section = (c as u16 & 0xff) << 8 | ((c >> 16) as u16 & 0xff);
        let seen = add_int_seen_data(e, root, section);
        if e.mem.u8(seen + INT_SEEN_ALL) != 0 {
            continue;
        }
        let all = update_seen_bits(e, seen, add_words(shift_words(offset), rel));
        let flag = e.mem.u8(seen + INT_SEEN_ALL);
        e.mem.set_u8(seen + INT_SEEN_ALL, flag | u8::from(all));
    }
}

#[cfg(test)]
#[path = "jip_fn_minimap_seams_tests.rs"]
mod tests;
