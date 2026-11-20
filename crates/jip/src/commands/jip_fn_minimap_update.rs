//! `UpdateMiniMap` (JIP LN NVSE's `jip_fn_minimap.h`), part 2 of JIP MiniMap: each update of the
//! mod's HUD map. The zoom; on a new cell its north and the interior zoom; the world map's markers
//! by cell; the nine local maps drawn (the cells around the player, or the interior's sections),
//! kept by cell and drawn again as JIP does; the fog of war's vertex alphas; the door and quest
//! markers; the player marker's turn and the map's place.
//!
//! JIP 57.30 writes no local map cache: its three `SaveLocalMapTexture` calls here are commented
//! out, so the port has none.
//!
//! Threads: on the script's thread (the main thread), as JIP's. The state is taken out of its
//! lock for the update and put back after it, so no game call runs under it; the seams' words are
//! lock-free. Around the local maps' drawing the game's own locks are held as JIP holds them: the
//! scene lights' lock (`0x011F9EA0`) over the light and fog swap and the drawing, the texture
//! list's critical section (`0x011F4580`) over JIP's texture list edits. Nothing new runs on the
//! render thread.

use super::jip_fn_minimap::{
    cell_markers_freed, children, init_dimensions, init_pos_mods, mini_map, root_map_world, state,
    tile_set_float, update_tile_scales, world_map_pos_mults, MarkerTile, MiniMapState, CHILDREN,
    CHILD_COUNT, FLAGS, GAME_FOG_PROPERTY, HUD_MAIN_MENU, INTERIOR_ZOOM, LOCAL_TRANSLATE,
    MOD_INDEX, REF_EXTRA, REF_POSITION, SHADOW_SCENE_NODE, TES, TES_GRID, TRAIT_DEPTH, TRAIT_USER0,
    TRAIT_USER1, TRAIT_USER2, VALUE_NUM,
};
use super::jip_fn_minimap_fog::{
    calc_vtx_alpha_by_seen_data, do_selective_fow_update, pack_curr_grid_coords, vtx_alpha_ptr,
};
use super::jip_fn_minimap_seams::{
    cell_at_coord, north_rotation, pack, rotate_interior, CELL_EXTRA, CELL_FLAGS, CELL_ID,
    EXTRA_DATA, PLAYER, PLAYER_CELL,
};
use super::jip_fn_ui::{value_of, value_set_bool, value_set_float};
use super::rendered_textures::{generate_rendered_texture, TextureParams};
use super::Handlers;
use crate::game_patches::gb::value_set_string;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::Ordering;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2874, update_mini_map),
];

// ---- the game's words ----

/// `LIGHTING_PASSES` (`*011F91D8`, a byte), the underwater fog flag (`*011FF104`), the byte JIP
/// clears around a local map's drawing (`*011AD7B4`), the texture list's head, tail and critical
/// section, the sky's extra node JIP hides outside a full update (`*011D8690`), the game's map
/// marker icons (`011A0404`, by type) and the undiscovered one (`01075030`).
const LIGHTING_PASSES: u32 = 0x011f_91d8;
const UNDERWATER_FOG: u32 = 0x011f_f104;
const DRAW_FLAG: u32 = 0x011a_d7b4;
const TEXTURE_LIST_HEAD: u32 = 0x011f_4500;
const TEXTURE_LIST_TAIL: u32 = 0x011f_4504;
const TEXTURE_LIST_LOCK: u32 = 0x011f_4580;
const EXTRA_HIDDEN_NODE: u32 = 0x011d_8690;
const MARKER_ICONS: u32 = 0x011a_0404;
const UNDISCOVERED_ICON: u32 = 0x0107_5030;
/// The game's `EnterCriticalSection` / `LeaveCriticalSection` (as `run_00a5f7b0.rs` calls them)
/// and the scene lights' lock (`LightCS`: enter `0040FBF0`, leave `0040FBA0`).
const ENTER_CRITICAL_SECTION: u32 = 0x00fd_f05c;
const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;
const SCENE_LIGHTS_LOCK: u32 = 0x011f_9ea0;
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
/// `Menu::AddTileFromTemplate` (`00A1DDB0`), `NiRenderedTexture`'s deleting destructor
/// (`00A7FD30`, called by JIP's `RenderedEntry` on a texture it lets go),
/// `NiNode::UpdateDownwardPass` (`00A5DD70`).
const ADD_TILE_FROM_TEMPLATE: u32 = 0x00a1_ddb0;
const RENDERED_TEXTURE_DELETE: u32 = 0x00a7_fd30;
/// `Tile::AddValue` (`00A01000`, thiscall: the trait id): the value, made when the tile has none.
const TILE_ADD_VALUE: u32 = 0x00a0_1000;
/// The loaded references (`*011C95C8`) and their teleport doors (+0xA8, an `NiTPtrMap`).
const LOADED_REFERENCES: u32 = 0x011c_95c8;
const TELEPORT_DOORS: u32 = 0xa8;
/// A reference's cell (+0x40); `kXData_ExtraTeleport` (its data +0xC: the linked door +0),
/// `kXData_ExtraDetachTime`.
const REF_CELL: u32 = 0x40;
const EXTRA_TELEPORT: u32 = 0x2b;
const EXTRA_DETACH_TIME: u32 = 0x0b;
/// `kVtbl_NiRenderedTexture`, `kVtbl_BSXFlags`.
const VTBL_RENDERED_TEXTURE: u32 = 0x0109_df8c;
const VTBL_BSX_FLAGS: u32 = 0x010c_1854;
/// `TES`: the grid (+0x24) and the player's cell (+0x2C), two words each; its light (+0x1C);
/// the current world (+0x88). The game's fog (`*011DEB00`).
const TES_GRID_XY: u32 = 0x24;
const TES_CELL_XY: u32 = 0x2c;
const TES_LIGHT: u32 = 0x1c;
const TES_WORLD: u32 = 0x88;
/// `PlayerCharacter`: rotation z (+0x2C), last exterior door (+0x604), active quest (+0x6B8),
/// quest targets (+0x6C4, `tList`), the placed marker's target (+0x6E8, an `ObjectiveTarget`:
/// its reference +0xC, teleport links +0x24 data / +0x28 size, a link 0x10 bytes, door +0),
/// placed marker (+0x6F4).
const PLAYER_ROTATE_Z: u32 = 0x2c;
const LAST_EXTERIOR_DOOR: u32 = 0x604;
const ACTIVE_QUEST: u32 = 0x6b8;
const QUEST_TARGETS: u32 = 0x6c4;
const PLACED_TARGET: u32 = 0x6e8;
const PLACED_MARKER: u32 = 0x6f4;
const TARGET_REF: u32 = 0x0c;
const TARGET_LINKS: u32 = 0x24;
const TARGET_LINK_COUNT: u32 = 0x28;
/// A form's flags (+8: 0x20 deleted, 0x40, 0x800 disabled), id (+0xC); a reference's position
/// (+0x30), cell (+0x40), render state (+0x64: its node +0x14); a cell's flags (+0x24, 2: has
/// water), coordinates (+0x48), land (+0x4C), world (+0xC0), render data (+0xC4: its root node +0,
/// animated references +0xC (`NiTMap`), water references +0x5C (`tList`)).
const FORM_FLAGS: u32 = 0x08;
const REF_RENDER_STATE: u32 = 0x64;
const RENDER_STATE_NODE: u32 = 0x14;
const CELL_COORDS: u32 = 0x48;
const CELL_LAND: u32 = 0x4c;
const CELL_WORLD: u32 = 0xc0;
const CELL_RENDER_DATA: u32 = 0xc4;
const RENDER_ANIMATED: u32 = 0x0c;
const RENDER_WATER: u32 = 0x5c;
/// `TESWorldSpace`: its map's file name (+0x28, `String`: text +0, length +4 `u16`), usable map
/// size (+0x80, two `i32`), NW and SE cell corners (+0x88, +0x8C, two `i16` each).
const WORLD_MAP_PATH: u32 = 0x28;
const WORLD_USABLE: u32 = 0x80;
const WORLD_NW: u32 = 0x88;
const WORLD_SE: u32 = 0x8c;
/// `MapMarkerData`: flags (+0xC: 1 visible, 2 discovered), type (+0xE).
const MARKER_FLAGS: u32 = 0x0c;
const MARKER_TYPE: u32 = 0x0e;
/// `ShadowSceneNode`: lights (+0xB4, `DList`), the sun (+0xE0, a `LightingData`: point light
/// +0xF4, light +0xF8), fog (+0x134).
const SCENE_LIGHTS: u32 = 0xb4;
const SCENE_SUN: u32 = 0xe0;
const LIGHT_IS_POINT: u32 = 0xf4;
const LIGHT_LIGHT: u32 = 0xf8;
const SCENE_FOG: u32 = 0x134;
/// `TileShaderProperty`: texture (+0x60), alpha (+0x78), vertex colours (+0x92).
const SHADER_TEXTURE: u32 = 0x60;
const SHADER_ALPHA: u32 = 0x78;
const SHADER_VERTEX_COLOURS: u32 = 0x92;
/// `NiTexture`'s list links (+0x28, +0x2C); `NiRenderedTexture`'s cell id (+0x34, JIP's).
const TEXTURE_PREV: u32 = 0x28;
const TEXTURE_NEXT: u32 = 0x2c;
const TEXTURE_CELL: u32 = 0x34;
/// `NiAVObject`'s name (+8), extra data (+0x10, count +0x14 `u16`), flags (+0x30, 1 hidden),
/// local translation (+0x58), world translation (+0x8C, then the world scale +0x98).
const NAME: u32 = 0x08;
const EXTRA_LIST: u32 = 0x10;
const EXTRA_COUNT: u32 = 0x14;
const WORLD_TRANSLATE: u32 = 0x8c;
/// A shape's shader property (+0xA8); a land shader property's flags byte (+0x25, 0x40) and
/// word +0x38, which JIP sets and clears around the exterior's drawing.
const GEOMETRY_SHADER: u32 = 0xa8;
const LAND_FLAGS: u32 = 0x25;
const LAND_WORD: u32 = 0x38;
/// The texture formats (`D3DFMT_X8R8G8B8`, `D3DFMT_L8`) and render modes (normal, no water).
const X8R8G8B8: u32 = 22;
const L8: u32 = 50;
const RENDER_NORMAL: u32 = 0;
const RENDER_NO_WATER: u32 = 12;
/// `kTileValue_x` ... the traits JIP sets.
const TRAIT_VISIBLE: u32 = 0xfa3;
const TRAIT_RED: u32 = 0xfb2;
const TRAIT_GREEN: u32 = 0xfb3;
const TRAIT_BLUE: u32 = 0xfb4;
const TRAIT_FILENAME: u32 = 0xfcc;
/// JIP's constants: the interior zoom (0.25), the local map's place factors (1 / 12288, 1 /
/// -12288), the alpha levels (0.25 to 1), the cached textures' bounds, the update mask per local
/// map (`kSelectUpdate`).
const CACHED_TEXTURES_MAX: usize = 60;
const CACHED_TEXTURES_MIN: usize = 42;
const SELECT_UPDATE: [u8; 9] = [0x8, 0xa, 0x2, 0xc, 0xf, 0x3, 0x4, 0x5, 0x1];
const VERTEX_ALPHA_MULTS: [f32; 4] = [0.25, 0.5, 0.75, 1.0];

fn local_map_pos_mults() -> [f32; 2] {
    [1.0f32 / 12288.0, 1.0f32 / -12288.0]
}

/// `DoorRef`: a door and the interior it leads to (0 for an exterior or none).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DoorRef {
    pub door: u32,
    pub linked_cell: u32,
}

/// `RenderedEntry`: a local map's texture and the parts drawn with the grid around it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderedEntry {
    pub texture: u32,
    pub regen_flags: u8,
}

/// `DoorMarkerTile`: a door marker tile's visible, x, y, visited values, the vertex alpha it
/// follows, whether it shows a door.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DoorMarkerTile {
    pub visible: u32,
    pub x: u32,
    pub y: u32,
    pub visited: u32,
    pub vtx_alpha: u32,
    pub in_use: bool,
}

/// `QuestMarkerTile`: the target's x the last time (as bits; `0x7F7FFFFF` at first), its tile's x
/// and y values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuestMarkerTile {
    pub pos: u32,
    pub x: u32,
    pub y: u32,
}

// ---- small pieces ----

fn hide(e: &mut Engine, object: u32) {
    if object != 0 {
        let flags = e.mem.u32(object + FLAGS);
        e.mem.set_u32(object + FLAGS, flags | 1);
    }
}

fn show(e: &mut Engine, object: u32) {
    if object != 0 {
        let flags = e.mem.u32(object + FLAGS);
        e.mem.set_u32(object + FLAGS, flags & !1);
    }
}

fn tile_set_bool(e: &mut Engine, tile: u32, id: u32, on: bool) {
    let value = value_of(e, tile, id);
    if value != 0 {
        value_set_bool(e, value, on);
    }
}

/// `Tile::SetString(id, text)`: the value of trait `id`, when the tile has it, given `text`.
fn tile_set_string(e: &mut Engine, tile: u32, id: u32, text: &[u8]) {
    let value = value_of(e, tile, id);
    if value != 0 {
        value_set_string(e, value, text);
    }
}

/// `Tile::Value::SetString` with the game's text at `text` (none: empty).
fn value_set_guest_string(e: &mut Engine, value: u32, text: u32) {
    let copy = if text == 0 {
        Vec::new()
    } else {
        e.mem.cstr(text).to_vec()
    };
    value_set_string(e, value, &copy);
}

/// `hudMain->AddTileFromTemplate(dest, name)`.
fn add_tile_from_template(e: &mut Engine, hud: u32, dest: u32, name: &[u8]) -> u32 {
    super::with_guest_text(e, name, |e, at| {
        e.call(ADD_TILE_FROM_TEMPLATE, &args![hud, dest, at, 0u32])
            .u32()
    })
}

/// An `NiTMap`'s values in JIP's iterator's order (bucket by bucket, each chain from its head).
fn map_values(e: &Engine, map: u32) -> Vec<u32> {
    let buckets = e.mem.u32(map + 8);
    let count = e.mem.u32(map + 4);
    let mut out = Vec::new();
    for i in 0..count {
        let mut entry = e.mem.u32(buckets + 4 * i);
        while entry != 0 {
            out.push(e.mem.u32(entry + 8));
            entry = e.mem.u32(entry);
        }
    }
    out
}

/// A `tList`'s items from its head (empty items included as 0, as JIP's `Head()` walk sees them).
fn list_items(e: &Engine, head: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut node = head;
    while node != 0 {
        out.push(e.mem.u32(node));
        node = e.mem.u32(node + 4);
    }
    out
}

/// `iceil`: rounded up, then truncated (`0x80000000` out of range).
fn iceil(v: f32) -> i32 {
    super::cvttss2si(v.ceil())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::GetBSXFlags
// (asm).
/// The node's `BSXFlags` value (its extra data searched from the last), 0 for none.
fn bsx_flags(e: &Engine, node: u32) -> u32 {
    let list = e.mem.u32(node + EXTRA_LIST);
    let count = u32::from(e.mem.u16(node + EXTRA_COUNT));
    for i in (0..count).rev() {
        let extra = e.mem.u32(list + 4 * i);
        if e.mem.u32(extra) == VTBL_BSX_FLAGS {
            return e.mem.u32(extra + 0xc);
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// AdjustInteriorPos (asm).
/// An interior position turned by the cell's north (when it has one), less 2048 each way.
fn adjust_interior_pos(e: &mut Engine, pos: [f32; 2]) -> [f32; 2] {
    let p = rotate_interior(e, pos);
    [p[0] - 2048.0, p[1] - 2048.0]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// MapMarkerInfo::GetMarkerIconPath (asm).
/// A map marker's icon path: the game's icon for its type when discovered (flag 2), the
/// undiscovered icon else. (JohnnyGuitar's icon routine: the port answers as without it.)
fn marker_icon_path(e: &Engine, data: u32) -> u32 {
    if e.mem.u8(data + MARKER_FLAGS) & 2 == 0 {
        return UNDISCOVERED_ICON;
    }
    let kind = u32::from(e.mem.u8(data + MARKER_TYPE));
    e.mem.u32(MARKER_ICONS + 4 * kind)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// GetTeleportDoors.
/// The loaded doors of `cell` (not deleted, 0x40 or disabled: flags & 0x860) with a linked door:
/// each with the linked door's cell when that is an interior.
fn teleport_doors(e: &mut Engine, cell: u32) -> Vec<DoorRef> {
    let loaded = e.mem.u32(LOADED_REFERENCES);
    let mut out = Vec::new();
    for refr in map_values(e, loaded + TELEPORT_DOORS) {
        if refr == 0
            || e.mem.u32(refr + FORM_FLAGS) & 0x860 != 0
            || e.mem.u32(refr + REF_CELL) != cell
        {
            continue;
        }
        let extra = super::extra::get_by_type(e, refr + REF_EXTRA, EXTRA_TELEPORT);
        // JIP reads the teleport extra unchecked (a door in this map has one).
        if extra == 0 {
            continue;
        }
        let data = e.mem.u32(extra + EXTRA_DATA);
        let linked = e.mem.u32(data);
        if linked == 0 {
            continue;
        }
        let linked_cell = e.mem.u32(linked + REF_CELL);
        let interior = if linked_cell != 0 && e.mem.u32(linked_cell + CELL_WORLD) == 0 {
            linked_cell
        } else {
            0
        };
        out.push(DoorRef {
            door: refr,
            linked_cell: interior,
        });
    }
    out
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// FindNextMMTexture (asm).
/// The next rendered texture in the game's texture list after `texture` that holds a cell id
/// (JIP's local maps), 0 at the end.
fn find_next_mm_texture(e: &Engine, texture: u32) -> u32 {
    let mut t = texture;
    loop {
        t = e.mem.u32(t + TEXTURE_NEXT);
        if t == 0 {
            return 0;
        }
        if e.mem.u32(t) == VTBL_RENDERED_TEXTURE && e.mem.u32(t + TEXTURE_CELL) != 0 {
            return t;
        }
    }
}

/// JIP's `RenderedEntry` destructor: the texture deleted (its deleting destructor, freed).
fn drop_entry(e: &mut Engine, entry: RenderedEntry) {
    if entry.texture != 0 {
        e.call(RENDERED_TEXTURE_DELETE, &args![entry.texture, 1u32]);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h: MiniMapLoadGame.
/// Before a game loads: the player's cell and world, the root world and the last interior
/// forgotten, so the next update starts again.
pub fn load_game(e: &mut Engine) {
    let mut s = state(e);
    s.pc_curr_cell = 0;
    s.pc_curr_world = 0;
    s.pc_root_world = 0;
    s.last_interior = 0;
}

// ---- UpdateMiniMap ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// Cmd_UpdateMiniMap_Execute.
/// `UpdateMiniMap type [flags]`, from JIP MiniMap.esp's script once `InitMiniMap` has run: type
/// 0 the local map, 1 the world map, 3 the zoom only (with flags 0x100, the default, nothing
/// else); flags 1 world markers, 2 doors, 4 quest markers, 8 colour local maps (else grey), 0x10
/// the fog of war. No result.
fn update_mini_map(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let (mode, index) = {
        let s = state(e);
        (s.mode, s.index)
    };
    if mode == 0 || a.script == 0 || e.mem.u8(a.script + MOD_INDEX) != index {
        return handled(true);
    }
    let read = Read::of(e, &a);
    // JIP goes on with what it read (the script's compiled arguments read).
    if !read.ok {
        return handled(true);
    }
    let update_type = read.int(0, 0);
    let show_flags = read.int(1, 0x100);
    let mut s = std::mem::take(&mut *state(e));
    update(e, &mut s, update_type, show_flags);
    *state(e) = s;
    handled(true)
}

fn update(e: &mut Engine, s: &mut MiniMapState, update_type: u32, show_flags: u32) {
    if update_type == 3 {
        let zoom = e.mem.f32(s.local_map_zoom + VALUE_NUM);
        let mut z = s.zoom_current;
        if s.zoom_current.to_bits() != zoom.to_bits() {
            z = zoom;
            if !s.in_exterior {
                z += INTERIOR_ZOOM;
            }
            s.zoom_current = z;
        }
        update_tile_scales(e, s, z);
        if show_flags == 0x100 {
            return;
        }
    }
    let player = e.mem.u32(PLAYER);
    let parent_cell = e.mem.u32(player + PLAYER_CELL);
    if parent_cell == 0 {
        return;
    }
    let mut parent_world = e.mem.u32(parent_cell + CELL_WORLD);

    let mut update_tiles = s.pc_curr_cell != parent_cell;
    if update_tiles {
        s.pc_curr_cell = parent_cell;
        let lanes = north_rotation(e, parent_cell);
        let camera = s.camera;
        for (at, v) in [
            (0x6c, lanes[0]),
            (0x70, lanes[1]),
            (0x78, lanes[2]),
            (0x7c, lanes[3]),
        ] {
            e.mem.set_f32(camera + at, v);
        }
        let exterior = parent_world != 0;
        if s.in_exterior != exterior {
            s.in_exterior = exterior;
            // `shl eax, 0x1F; or eax, kInteriorZoomMod`: -0.25 into an exterior, 0.25 out.
            let change = if exterior {
                -INTERIOR_ZOOM
            } else {
                INTERIOR_ZOOM
            };
            s.zoom_current += change;
            update_tile_scales(e, s, s.zoom_current);
        }
    }

    let show_flags_change = s.show_flags ^ show_flags;
    s.show_flags = show_flags;
    let mut upd_quest_targets = mini_map(e)
        .update_quest_targets
        .swap(false, Ordering::AcqRel);
    if s.current_map_mode != update_type {
        if update_type <= 1 {
            s.current_map_mode = update_type;
            value_set_bool(e, s.mode, update_type == 1);
        }
        update_tiles = true;
        upd_quest_targets = true;
    }

    let world_map = s.current_map_mode != 0;
    let hud = e.mem.u32(HUD_MAIN_MENU);
    let mut nw = [0.0f32; 2];
    let mut pos_mult = [0.0f32; 2];

    let use_fog = show_flags & 0x10 != 0;
    if show_flags_change & 0x10 != 0 {
        update_tiles = true;
        for prop in s.shader_props {
            e.mem
                .set_u8(prop + SHADER_VERTEX_COLOURS, u8::from(use_fog));
        }
        if use_fog {
            let alpha = e.mem.f32(s.shader_props[0] + SHADER_ALPHA);
            for (i, m) in VERTEX_ALPHA_MULTS.iter().enumerate() {
                s.vertex_alpha[i + 1] = alpha * m;
            }
        }
    }

    if world_map {
        let object = if parent_world != 0 {
            player
        } else {
            parent_world = e.mem.u32(e.mem.u32(TES) + TES_WORLD);
            let door = e.mem.u32(player + LAST_EXTERIOR_DOOR);
            if parent_world == 0 || door == 0 {
                return;
            }
            door
        };
        if s.pc_curr_world != parent_world {
            s.pc_curr_world = parent_world;
            let mut dims = s.root_dims;
            init_pos_mods(e, &mut dims, parent_world);
            s.root_dims = dims;
            let root = root_map_world(e, parent_world);
            if s.pc_root_world != root {
                s.pc_root_world = root;
                let mut dims = s.root_dims;
                init_dimensions(e, &mut dims, root);
                s.root_dims = dims;
                world_map_file(e, s.world_map_tile, root);
                let usable_x = e.mem.u32(root + WORLD_USABLE) as i32;
                let usable_y = e.mem.u32(root + WORLD_USABLE + 4) as i32;
                tile_set_float(e, s.world_map_rect, TRAIT_USER1, usable_x as f32);
                tile_set_float(e, s.world_map_rect, TRAIT_USER2, usable_y as f32);
                s.curr_world_markers = s.world_markers.contains_key(&root).then_some(root);
                let w16 = |at: u32| i32::from(e.mem.u16(root + at) as i16);
                let (nw_x, nw_y, se_x, se_y) = (
                    w16(WORLD_NW),
                    w16(WORLD_NW + 2),
                    w16(WORLD_SE),
                    w16(WORLD_SE + 2),
                );
                s.root_dims.mk_range =
                    iceil(72.0f32 / usable_x as f32 * (se_x - nw_x + nw_y - se_y) as f32);
                if show_flags & 1 != 0 {
                    let visible: Vec<u32> = s
                        .marker_tiles
                        .iter_mut()
                        .filter(|t| t.in_use)
                        .map(|t| {
                            t.in_use = false;
                            t.visible
                        })
                        .collect();
                    for v in visible {
                        value_set_bool(e, v, false);
                    }
                    s.rendered_markers.clear();
                }
            }
            update_tiles = true;
        }

        let opos = ref_pos(e, object);
        let mut coord = 0;
        pos_mult = world_map_pos_mults(opos, &s.root_dims, Some(&mut coord));
        update_tiles |= s.curr_world_coords != coord;
        s.curr_world_coords = coord;

        if show_flags_change & 1 != 0 {
            tile_set_bool(e, s.map_markers_rect, TRAIT_VISIBLE, show_flags & 1 != 0);
        }
        let discovered = mini_map(e).discovered_location.load(Ordering::Acquire);
        if show_flags & 1 != 0 && (update_tiles || discovered) {
            mini_map(e)
                .discovered_location
                .store(false, Ordering::Release);
            world_markers_update(e, s, hud, coord);
        }
    } else {
        let place = Place {
            player,
            parent_cell,
            parent_world,
            use_fog,
            show_flags,
            show_flags_change,
        };
        update_tiles = local_maps(e, s, hud, &place, update_tiles, &mut nw, &mut pos_mult);
    }
    quest_markers(
        e,
        s,
        hud,
        player,
        parent_cell,
        parent_world,
        world_map,
        update_tiles,
        upd_quest_targets,
        nw,
        show_flags,
        show_flags_change,
    );
    let rotate = f32::from_bits(mini_map(e).north_rotation.load(Ordering::Acquire))
        - e.mem.f32(player + PLAYER_ROTATE_Z);
    value_set_float(e, s.pc_marker_rotate, rotate);
    value_set_float(e, s.pos_x, pos_mult[0]);
    value_set_float(e, s.pos_y, pos_mult[1]);
}

/// `s_worldMapTile->SetString(filename, ...)`: the root world's map, else JIP's blank tile.
fn world_map_file(e: &mut Engine, tile: u32, root: u32) {
    let len = e.mem.u16(root + WORLD_MAP_PATH + 4);
    let text = if len != 0 {
        e.mem.cstr(e.mem.u32(root + WORLD_MAP_PATH)).to_vec()
    } else {
        b"jazzisparis\\minimap\\blanktile.dds".to_vec()
    };
    tile_set_string(e, tile, TRAIT_FILENAME, &text);
}

/// What the local maps need of the update.
struct Place {
    player: u32,
    parent_cell: u32,
    parent_world: u32,
    use_fog: bool,
    show_flags: u32,
    show_flags_change: u32,
}

fn ref_pos(e: &Engine, refr: u32) -> [f32; 2] {
    [
        e.mem.f32(refr + REF_POSITION),
        e.mem.f32(refr + REF_POSITION + 4),
    ]
}

/// The world map's markers: the cells within the zoom's range around `coord` (each way
/// `iceil(mkRange / zoom)`), each cell's visible, enabled markers given tiles (the made ones not in
/// use first, in order, then new ones from `MiniMapWorldMarkerTemplate`) once per cell; the cells
/// out of range let go.
fn world_markers_update(e: &mut Engine, s: &mut MiniMapState, hud: u32, coord: u32) {
    s.curr_marker_cells.clear();
    let markers = s
        .curr_world_markers
        .and_then(|root| s.world_markers.get(&root));
    if let Some(markers) = markers.filter(|m| !m.is_empty()) {
        let markers = markers.clone();
        let zoom = e.mem.f32(s.world_map_zoom + VALUE_NUM);
        let range = iceil(s.root_dims.mk_range as f32 / zoom);
        let (mut x, mut y) = ((coord >> 16) as u16, coord as u16);
        x = x.wrapping_sub(range as u16);
        y = y.wrapping_sub(range as u16);
        let range = (range as u32).wrapping_shl(1).wrapping_add(1);
        let mut next_tile = 0usize;
        let mut iter_x = range;
        loop {
            let mut iter_y = range;
            loop {
                let at = (u32::from(x) << 16) | u32::from(y);
                if let Some(cell_markers) = markers.get(&at) {
                    s.curr_marker_cells.insert(at);
                    if !s.rendered_markers.contains_key(&at) {
                        let mut tiles = Vec::new();
                        for m in cell_markers {
                            if e.mem.u8(m.data + MARKER_FLAGS) & 1 == 0
                                || e.mem.u32(m.refr + FORM_FLAGS) & 0x800 != 0
                            {
                                continue;
                            }
                            let index = loop {
                                if next_tile >= s.marker_tiles.len() {
                                    let tile = add_tile_from_template(
                                        e,
                                        hud,
                                        s.map_markers_rect,
                                        b"MiniMapWorldMarkerTemplate",
                                    );
                                    let made = marker_tile(e, tile, s.marker_tiles.len() + 1);
                                    s.marker_tiles.push(made);
                                    next_tile = s.marker_tiles.len();
                                    break s.marker_tiles.len() - 1;
                                }
                                let i = next_tile;
                                next_tile += 1;
                                if !s.marker_tiles[i].in_use {
                                    s.marker_tiles[i].in_use = true;
                                    let v = s.marker_tiles[i].visible;
                                    value_set_bool(e, v, true);
                                    break i;
                                }
                            };
                            tiles.push(index);
                            let t = s.marker_tiles[index];
                            value_set_float(e, t.x, m.pos[0]);
                            value_set_float(e, t.y, m.pos[1]);
                            let icon = marker_icon_path(e, m.data);
                            value_set_guest_string(e, t.filename, icon);
                        }
                        s.rendered_markers.insert(at, tiles);
                    }
                }
                y = y.wrapping_add(1);
                iter_y = iter_y.wrapping_sub(1);
                if iter_y == 0 {
                    break;
                }
            }
            x = x.wrapping_add(1);
            y = y.wrapping_sub(range as u16);
            iter_x = iter_x.wrapping_sub(1);
            if iter_x == 0 {
                break;
            }
        }
    }
    let gone: Vec<u32> = s
        .rendered_markers
        .keys()
        .copied()
        .filter(|k| !s.curr_marker_cells.contains(k))
        .collect();
    // JIP's `FreeCellMapMarkers` on each: the state is out of its lock here, so the tiles are
    // hidden at once.
    for coord in gone {
        for visible in cell_markers_freed(s, coord) {
            value_set_bool(e, visible, false);
        }
    }
}

/// `MapMarkerTile(tile)`: its visible, x (user0), y (user1) values and its file name value (added),
/// its depth `(count & 0xF) + 18`, in use.
fn marker_tile(e: &mut Engine, tile: u32, count: usize) -> MarkerTile {
    let made = MarkerTile {
        visible: value_of(e, tile, TRAIT_VISIBLE),
        x: value_of(e, tile, TRAIT_USER0),
        y: value_of(e, tile, TRAIT_USER1),
        filename: e.call(TILE_ADD_VALUE, &args![tile, TRAIT_FILENAME]).u32(),
        in_use: true,
    };
    tile_set_float(e, tile, TRAIT_DEPTH, ((count & 0xf) + 18) as f32);
    made
}

/// The local map: the textures' format (flag 8: colour, else grey; a change lets every exterior
/// map go), then the exterior's nine cells or the interior's nine sections drawn and kept as JIP
/// does, the fog of war, and the door markers. The new "update the tiles".
fn local_maps(
    e: &mut Engine,
    s: &mut MiniMapState,
    hud: u32,
    p: &Place,
    mut update_tiles: bool,
    nw: &mut [f32; 2],
    pos_mult: &mut [f32; 2],
) -> bool {
    let was = mini_map(e).update_fog_of_war.swap(false, Ordering::AcqRel);
    let update_fog = p.use_fog && was;
    let format = if p.show_flags & 8 != 0 { X8R8G8B8 } else { L8 };
    if mini_map(e).texture_format.load(Ordering::Acquire) != format {
        mini_map(e).texture_format.store(format, Ordering::Release);
        for (_, entry) in std::mem::take(&mut s.rendered_exteriors) {
            drop_entry(e, entry);
        }
        s.mm_first_texture = 0;
        s.last_interior = 0;
        update_tiles = true;
    }
    let lighting_passes = e.mem.u8(LIGHTING_PASSES);
    let mut adjusted = [0.0f32; 2];
    if p.parent_world != 0 {
        update_tiles = exterior(e, s, p, update_tiles, update_fog, nw, pos_mult);
    } else {
        update_tiles = interior(
            e,
            s,
            p,
            update_tiles,
            update_fog,
            nw,
            pos_mult,
            &mut adjusted,
        );
    }
    e.mem.set_u8(LIGHTING_PASSES, lighting_passes);
    door_markers(e, s, hud, p, update_tiles, update_fog, *nw);
    update_tiles
}

/// The exterior: the player's cell's coordinate and the map's corner (the cell less one across,
/// two up, times 4096), the player's place on it; the nine cells' grid and their vertex alphas and
/// doors on a new cell, else the fog's selective update; on a new cell or grid the drawing.
fn exterior(
    e: &mut Engine,
    s: &mut MiniMapState,
    p: &Place,
    mut update_tiles: bool,
    update_fog: bool,
    nw: &mut [f32; 2],
    pos_mult: &mut [f32; 2],
) -> bool {
    let tes = e.mem.u32(TES);
    let cx = e.mem.u32(tes + TES_CELL_XY) as i32;
    let cy = e.mem.u32(tes + TES_CELL_XY + 4) as i32;
    let coord = pack(cx, cy);
    *nw = [
        (cx.wrapping_sub(1).wrapping_shl(12)) as f32,
        (cy.wrapping_add(2).wrapping_shl(12)) as f32,
    ];
    let ppos = ref_pos(e, p.player);
    let k = local_map_pos_mults();
    *pos_mult = [(ppos[0] - nw[0]) * k[0], (ppos[1] - nw[1]) * k[1]];
    let gx = e.mem.u32(tes + TES_GRID_XY) as i32;
    let gy = e.mem.u32(tes + TES_GRID_XY + 4) as i32;
    let grid_center = pack(gx, gy);

    update_tiles |= s.curr_local_coords != coord;
    if update_tiles {
        s.curr_local_coords = coord;
        s.packed_coords = pack_curr_grid_coords(coord);
        let grid = e.mem.u32(tes + TES_GRID);
        for i in 0..9 {
            s.curr_cell_grid[i] = cell_at_coord(e, grid, s.packed_coords[i]);
            if p.use_fog {
                calc_vtx_alpha_by_seen_data(e, s, i);
            }
        }
        if p.show_flags & 2 != 0 {
            let ids: Vec<u32> = s
                .curr_cell_grid
                .iter()
                .filter(|&&c| c != 0)
                .map(|&c| e.mem.u32(c + CELL_ID))
                .collect();
            s.exterior_door_refs.retain(|id, _| ids.contains(id));
            s.door_refs_list.clear();
            for cell in s.curr_cell_grid {
                if cell == 0 {
                    continue;
                }
                let id = e.mem.u32(cell + CELL_ID);
                let list = s
                    .exterior_door_refs
                    .entry(id)
                    .or_insert_with(|| teleport_doors(e, cell))
                    .clone();
                s.door_refs_list.extend(list);
            }
        }
    } else if update_fog {
        do_selective_fow_update(e, s, ppos);
    }

    if update_tiles || s.cell_grid_center != grid_center {
        s.cell_grid_center = grid_center;
        draw_exterior(e, s, update_tiles, *nw);
    }
    update_tiles
}

fn hide_add(e: &mut Engine, hidden: &mut Vec<u32>, object: u32) {
    hide(e, object);
    hidden.push(object);
}

/// The exterior's drawing: the sky, weather, terrain LOD, particles and grass hidden; each cell's
/// `Get3DNode(4)`, its animated references with `BSXFlags & 0x23 == 0x21`, its placed water (and
/// the cell's water plane shown, at the water's height on a full update) hidden; outside a full
/// update the extra node too, on one with water the planes placed. Under the scene lights' lock:
/// the point lights hidden, JIP's light and fog in the sun's and scene's place, the underwater fog
/// flag on, 0x3C lighting passes, and each cell's map drawn (new: drawn, its parts marked when the
/// grid is on it; on a full update moved to the texture list's end; else drawn again when its part
/// was not), put in its shape. Then everything put back, the water planes hidden, and above 60
/// maps the oldest let go down to 42.
fn draw_exterior(e: &mut Engine, s: &mut MiniMapState, update_tiles: bool, nw: [f32; 2]) {
    for n in s.scene_nodes {
        hide(e, n);
    }
    let mut update_water = false;
    let mut hidden: Vec<u32> = Vec::new();
    for i in 0..9 {
        let cell = s.curr_cell_grid[i];
        if cell == 0 {
            continue;
        }
        let render = e.mem.u32(cell + CELL_RENDER_DATA);
        let root = e.mem.u32(render);
        let node4 = e.mem.u32(e.mem.u32(root + CHILDREN) + 16);
        hide_add(e, &mut hidden, node4);
        for node in map_values(e, render + RENDER_ANIMATED) {
            if node != 0 && bsx_flags(e, node) & 0x23 == 0x21 {
                hide_add(e, &mut hidden, node);
            }
        }
        if e.mem.u8(cell + CELL_FLAGS) & 2 == 0 {
            continue;
        }
        let mut water_ref = 0;
        for refr in list_items(e, render + RENDER_WATER) {
            if refr == 0 || e.mem.u8(refr + MOD_INDEX) != 0xff {
                continue;
            }
            let state = e.mem.u32(refr + REF_RENDER_STATE);
            let node = if state == 0 {
                0
            } else {
                e.mem.u32(state + RENDER_STATE_NODE)
            };
            if node == 0 {
                continue;
            }
            hide_add(e, &mut hidden, node);
            water_ref = refr;
        }
        if water_ref == 0 {
            continue;
        }
        let plane = e
            .mem
            .u32(e.mem.u32(s.water_planes + CHILDREN) + 4 * i as u32);
        show(e, plane);
        update_water = update_tiles;
        if update_water {
            let z = e.mem.f32(water_ref + REF_POSITION + 8);
            e.mem.set_f32(plane + LOCAL_TRANSLATE + 8, z);
        }
    }
    if !update_tiles {
        let node = e.mem.u32(EXTRA_HIDDEN_NODE);
        hide_add(e, &mut hidden, node);
    } else if update_water {
        e.mem.set_f32(s.water_planes + LOCAL_TRANSLATE, nw[0]);
        e.mem.set_f32(s.water_planes + LOCAL_TRANSLATE + 4, nw[1]);
        super::ref_move::update_node(e, s.water_planes);
    }
    e.call(LOCK_ENTER, &args![SCENE_LIGHTS_LOCK, 0u32]);
    let scene = e.mem.u32(SHADOW_SCENE_NODE);
    let mut node = e.mem.u32(scene + SCENE_LIGHTS);
    while node != 0 {
        let data = e.mem.u32(node + 8);
        node = e.mem.u32(node);
        if data == 0 || e.mem.u8(data + LIGHT_IS_POINT) == 0 {
            continue;
        }
        let light = e.mem.u32(data + LIGHT_LIGHT);
        if light != 0 && e.mem.u32(light + FLAGS) & 1 == 0 {
            hide_add(e, &mut hidden, light);
        }
    }
    let sun = e.mem.u32(scene + SCENE_SUN);
    e.mem.set_u32(sun + LIGHT_LIGHT, s.light);
    e.mem.set_u32(scene + SCENE_FOG, s.fog);
    let underwater = e.mem.u8(UNDERWATER_FOG);
    e.mem.set_u8(UNDERWATER_FOG, 1);
    e.mem.set_u8(LIGHTING_PASSES, 0x3c);
    s.render_mode = RENDER_NORMAL;
    for (i, &mask) in SELECT_UPDATE.iter().enumerate() {
        let cell = s.curr_cell_grid[i];
        if cell == 0 {
            // JIP leaves the slot's texture as it was (`continue`): a map the purge or a format
            // change has deleted since. The port clears it, as `InitMiniMap` leaves it (FIXES).
            e.mem.set_u32(s.shader_props[i] + SHADER_TEXTURE, 0);
            continue;
        }
        let id = e.mem.u32(cell + CELL_ID);
        if let Some(mut entry) = s.rendered_exteriors.get(&id).copied() {
            if update_tiles {
                if entry.texture != e.mem.u32(TEXTURE_LIST_TAIL) {
                    move_to_list_end(e, s, entry.texture);
                }
            } else if entry.regen_flags & mask != mask {
                entry.regen_flags |= mask;
                entry.texture = redraw_kept(e, s, entry.texture, |e, s, slot| {
                    generate_local_map_exterior(e, s, cell, slot)
                });
                s.rendered_exteriors.insert(id, entry);
            }
        } else {
            let mut entry = RenderedEntry::default();
            generate_local_map_exterior(e, s, cell, &mut entry.texture);
            if s.curr_local_coords == s.cell_grid_center {
                entry.regen_flags = mask;
            }
            if s.mm_first_texture == 0 {
                s.mm_first_texture = entry.texture;
            }
            s.rendered_exteriors.insert(id, entry);
        }
        let texture = s.rendered_exteriors[&id].texture;
        e.mem.set_u32(s.shader_props[i] + SHADER_TEXTURE, texture);
    }
    e.mem.set_u8(UNDERWATER_FOG, underwater);
    for object in hidden {
        show(e, object);
    }
    let tes = e.mem.u32(TES);
    let light = e.mem.u32(tes + TES_LIGHT);
    e.mem.set_u32(sun + LIGHT_LIGHT, light);
    let fog = e.mem.u32(GAME_FOG_PROPERTY);
    e.mem.set_u32(scene + SCENE_FOG, fog);
    e.call(LOCK_LEAVE, &args![SCENE_LIGHTS_LOCK]);
    for n in s.scene_nodes {
        show(e, n);
    }
    let planes = e.mem.u32(s.water_planes + CHILDREN);
    let count = u32::from(e.mem.u16(s.water_planes + CHILD_COUNT));
    for i in 0..count {
        let plane = e.mem.u32(planes + 4 * i);
        hide(e, plane);
    }
    purge_rendered_exteriors(e, s);
}

/// Above [`CACHED_TEXTURES_MAX`] kept exterior maps, the oldest (from JIP's first texture on, in
/// the game's texture list's order) let go down to [`CACHED_TEXTURES_MIN`], under the list's
/// critical section; the first texture is then the one after the last let go.
fn purge_rendered_exteriors(e: &mut Engine, s: &mut MiniMapState) {
    if s.rendered_exteriors.len() > CACHED_TEXTURES_MAX {
        let mut purge = s.rendered_exteriors.len() - CACHED_TEXTURES_MIN;
        let mut curr = s.mm_first_texture;
        e.call(ENTER_CRITICAL_SECTION, &args![TEXTURE_LIST_LOCK]);
        // JIP starts at its first texture unchecked; the port stops at none.
        while curr != 0 {
            let next = find_next_mm_texture(e, curr);
            let id = e.mem.u32(curr + TEXTURE_CELL);
            if let Some(entry) = s.rendered_exteriors.remove(&id) {
                drop_entry(e, entry);
                purge -= 1;
            }
            curr = next;
            if purge == 0 {
                break;
            }
        }
        s.mm_first_texture = curr;
        e.call(LEAVE_CRITICAL_SECTION, &args![TEXTURE_LIST_LOCK]);
    }
}

/// A kept exterior map drawn again (`draw` makes the new texture in the slot and releases the old
/// one, whose only owner was the entry): answers the new texture. JIP keeps `s_mmFirstTexture` on
/// the old one, so a later purge would start from freed memory; the port moves it on to the next
/// of JIP's maps first (or to the new one when there is none), under the list's lock, as a move to
/// the list's end does (a JIP slip not copied, FIXES).
fn redraw_kept(
    e: &mut Engine,
    s: &mut MiniMapState,
    old: u32,
    draw: impl FnOnce(&mut Engine, &mut MiniMapState, &mut u32),
) -> u32 {
    let was_first = old != 0 && old == s.mm_first_texture;
    if was_first {
        e.call(ENTER_CRITICAL_SECTION, &args![TEXTURE_LIST_LOCK]);
        s.mm_first_texture = find_next_mm_texture(e, old);
        e.call(LEAVE_CRITICAL_SECTION, &args![TEXTURE_LIST_LOCK]);
    }
    let mut texture = old;
    draw(e, s, &mut texture);
    if was_first && s.mm_first_texture == 0 {
        s.mm_first_texture = texture;
    }
    texture
}

/// A kept local map moved to the end of the game's texture list (its first one moved on first),
/// under the list's critical section.
fn move_to_list_end(e: &mut Engine, s: &mut MiniMapState, texture: u32) {
    e.call(ENTER_CRITICAL_SECTION, &args![TEXTURE_LIST_LOCK]);
    if texture == s.mm_first_texture {
        s.mm_first_texture = find_next_mm_texture(e, texture);
    }
    let prev = e.mem.u32(texture + TEXTURE_PREV);
    let next = e.mem.u32(texture + TEXTURE_NEXT);
    if prev != 0 {
        e.mem.set_u32(prev + TEXTURE_NEXT, next);
    } else {
        e.mem.set_u32(TEXTURE_LIST_HEAD, next);
    }
    if next != 0 {
        e.mem.set_u32(next + TEXTURE_PREV, prev);
    }
    let tail = e.mem.u32(TEXTURE_LIST_TAIL);
    e.mem.set_u32(tail + TEXTURE_NEXT, texture);
    e.mem.set_u32(texture + TEXTURE_PREV, tail);
    e.mem.set_u32(texture + TEXTURE_NEXT, 0);
    e.mem.set_u32(TEXTURE_LIST_TAIL, texture);
    e.call(LEAVE_CRITICAL_SECTION, &args![TEXTURE_LIST_LOCK]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// GenerateLocalMapExterior (asm).
/// A cell's local map drawn: the cell's first 3D node hidden, its four land quads' shader
/// properties flagged (+0x25 |= 0x40, +0x38 = 0) when not flagged and +0x38 is set; the camera over
/// the cell's centre at 65536 (or the land's top + 16384 when it is at most 65536); the drawing
/// (the byte `011AD7B4` 0 around it) into `slot`; then the node shown, the quads unflagged (+0x38
/// left 0), the texture given the cell's id.
fn generate_local_map_exterior(e: &mut Engine, s: &MiniMapState, cell: u32, slot: &mut u32) {
    let render = e.mem.u32(cell + CELL_RENDER_DATA);
    let root = e.mem.u32(render);
    let children = e.mem.u32(root + CHILDREN);
    let first = e.mem.u32(children);
    let flags = e.mem.u8(first + FLAGS);
    e.mem.set_u8(first + FLAGS, flags | 1);
    let land = e.mem.u32(children + 8);
    let quads = e.mem.u32(land + CHILDREN);
    let mut saved = [0u32; 4];
    for i in (0..4).rev() {
        let quad = e.mem.u32(quads + 4 * i as u32);
        let shape = e.mem.u32(e.mem.u32(quad + CHILDREN));
        let shader = e.mem.u32(shape + GEOMETRY_SHADER);
        if e.mem.u8(shader + LAND_FLAGS) & 0x40 != 0 || e.mem.u32(shader + LAND_WORD) == 0 {
            continue;
        }
        saved[i] = shader;
        let f = e.mem.u8(shader + LAND_FLAGS);
        e.mem.set_u8(shader + LAND_FLAGS, f | 0x40);
        e.mem.set_u32(shader + LAND_WORD, 0);
    }
    let coords = e.mem.u32(cell + CELL_COORDS);
    let x = e.mem.u32(coords) as i32;
    let y = e.mem.u32(coords + 4) as i32;
    let pos = [
        x.wrapping_shl(12).wrapping_add(0x800) as f32,
        y.wrapping_shl(12).wrapping_add(0x800) as f32,
    ];
    let mut z = 65536.0f32;
    let land_data = e.mem.u32(cell + CELL_LAND);
    if land_data != 0 {
        let heights = e.mem.u32(land_data + 0x28);
        // `comiss 65536, [ecx+0x18]; jb doneZ`: on when +0x18 is at most 65536 (a NaN jumps:
        // `>=` is false for it); the height then +0x1C plus 16384.
        if heights != 0 && z >= e.mem.f32(heights + 0x18) {
            z = e.mem.f32(heights + 0x1c) + 16384.0;
        }
    }
    draw_local_map(e, s, [pos[0], pos[1], z], slot);
    let flags = e.mem.u8(first + FLAGS);
    e.mem.set_u8(first + FLAGS, flags & !1);
    for shader in saved.into_iter().rev() {
        if shader == 0 {
            continue;
        }
        let f = e.mem.u8(shader + LAND_FLAGS);
        e.mem.set_u8(shader + LAND_FLAGS, f & !0x40);
        e.mem.set_u32(shader + LAND_WORD, 0);
    }
    if *slot != 0 {
        let id = e.mem.u32(cell + CELL_ID);
        e.mem.set_u32(*slot + TEXTURE_CELL, id);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// GenerateLocalMapInterior (asm).
/// An interior section's local map drawn: the camera over the section's corner plus one cell
/// (turned by the cell's north when it has one) at 32768, the drawing into `slot`.
fn generate_local_map_interior(e: &mut Engine, s: &MiniMapState, coord: u32, slot: &mut u32) {
    let x = i32::from((coord >> 16) as u16 as i16);
    let y = i32::from(coord as u16 as i16);
    let mut pos = [
        x.wrapping_add(1).wrapping_shl(12) as f32,
        y.wrapping_add(1).wrapping_shl(12) as f32,
    ];
    let m = mini_map(e);
    if m.north_rotation.load(Ordering::Acquire) != 0 {
        let r: Vec<f32> = m
            .north_rotation_mods
            .iter()
            .map(|v| f32::from_bits(v.load(Ordering::Acquire)))
            .collect();
        // shufps 0x11: (y, x, y, x); times the lanes; haddps.
        let l = [pos[1] * r[0], pos[0] * r[1], pos[1] * r[2], pos[0] * r[3]];
        pos = [l[0] + l[1], l[2] + l[3]];
    }
    draw_local_map(e, s, [pos[0], pos[1], 32768.0], slot);
}

/// The camera's world place (and its world scale 0, as JIP's 16-byte store leaves it), the byte
/// `011AD7B4` cleared, the world drawn into `slot` with the local maps' texture settings, the byte
/// set again.
fn draw_local_map(e: &mut Engine, s: &MiniMapState, place: [f32; 3], slot: &mut u32) {
    let camera = s.camera;
    for (i, v) in [place[0], place[1], place[2], 0.0].iter().enumerate() {
        e.mem.set_f32(camera + WORLD_TRANSLATE + 4 * i as u32, *v);
    }
    let params = TextureParams {
        width: 0x180,
        height: 0x180,
        format: mini_map(e).texture_format.load(Ordering::Acquire),
        render_mode: s.render_mode,
        bg_color_mask: 0,
    };
    e.mem.set_u8(DRAW_FLAG, 0);
    // JIP's slot is its map entry's; the port's entry is its own, so the slot is a stack word.
    *slot = e.with_stack(4, |e, at| {
        let at = at.addr();
        e.mem.set_u32(at, *slot);
        generate_rendered_texture(e, camera, params, at);
        e.mem.u32(at)
    });
    e.mem.set_u8(DRAW_FLAG, 1);
}

/// The interior: the player's place turned by the cell's north, less 2048 (`AdjustInteriorPos`),
/// its section and the map's corner from it; on a full update the cell's maps let go when it is a
/// new cell, its doors, and what JIP hides (the cell's first and fifth nodes, its `BlackPlane01`
/// shapes); on a new section the nine sections drawn (each once per cell, with no water pass and
/// 0x34 lighting passes) with the scene's particle node and those hidden, and their vertex alphas;
/// else the fog's selective update.
#[allow(clippy::too_many_arguments)]
fn interior(
    e: &mut Engine,
    s: &mut MiniMapState,
    p: &Place,
    mut update_tiles: bool,
    update_fog: bool,
    nw: &mut [f32; 2],
    pos_mult: &mut [f32; 2],
    adjusted: &mut [f32; 2],
) -> bool {
    let ppos = ref_pos(e, p.player);
    *adjusted = adjust_interior_pos(e, ppos);
    // `cvttps2dq`, `psrad 12`.
    let ix = super::cvttss2si(adjusted[0]) >> 12;
    let iy = super::cvttss2si(adjusted[1]) >> 12;
    let coord = pack(ix, iy);
    *nw = [
        (ix.wrapping_sub(1).wrapping_shl(12)) as f32,
        (iy.wrapping_add(2).wrapping_shl(12)) as f32,
    ];
    let k = local_map_pos_mults();
    *pos_mult = [(adjusted[0] - nw[0]) * k[0], (adjusted[1] - nw[1]) * k[1]];

    if update_tiles {
        if s.last_interior != p.parent_cell {
            s.last_interior = p.parent_cell;
            for (_, entry) in std::mem::take(&mut s.rendered_interior) {
                drop_entry(e, entry);
            }
        }
        if p.show_flags & 2 != 0 {
            s.door_refs_list = teleport_doors(e, p.parent_cell);
        }
        s.hidden_int.clear();
        let render = e.mem.u32(p.parent_cell + CELL_RENDER_DATA);
        let nodes = e.mem.u32(e.mem.u32(render) + CHILDREN);
        s.hidden_int.push(e.mem.u32(nodes));
        s.hidden_int.push(e.mem.u32(nodes + 16));
        let third = e.mem.u32(nodes + 12);
        for child in children(e, third) {
            if e.mem.u32(child + NAME) == s.black_plane {
                s.hidden_int.push(child);
            }
        }
    }

    update_tiles |= s.curr_local_coords != coord;
    if update_tiles {
        s.curr_local_coords = coord;
        s.packed_coords = pack_curr_grid_coords(coord);
        for object in s.hidden_int.clone() {
            hide(e, object);
        }
        hide(e, s.scene_nodes[3]);
        e.mem.set_u8(LIGHTING_PASSES, 0x34);
        s.render_mode = RENDER_NO_WATER;
        for i in 0..9 {
            let section = s.packed_coords[i];
            if !s.rendered_interior.contains_key(&section) {
                let mut entry = RenderedEntry::default();
                generate_local_map_interior(e, s, section, &mut entry.texture);
                s.rendered_interior.insert(section, entry);
            }
            let texture = s.rendered_interior[&section].texture;
            e.mem.set_u32(s.shader_props[i] + SHADER_TEXTURE, texture);
            if p.use_fog {
                calc_vtx_alpha_by_seen_data(e, s, i);
            }
        }
        for object in s.hidden_int.clone() {
            show(e, object);
        }
        show(e, s.scene_nodes[3]);
    } else if update_fog {
        do_selective_fow_update(e, s, *adjusted);
    }
    update_tiles
}

/// The door markers (flag 2): the rect shown or hidden with the flag; on a full update a tile per
/// door on the map (the made ones first, then new ones from `MiniMapDoorMarkerTemplate`): its place,
/// whether its interior was visited (no interior, the player's cell, or one with a detach time), its
/// vertex alpha and so its visibility under the fog; the tiles left over not in use. Else on a fog
/// update the doors' visibility from their alphas.
fn door_markers(
    e: &mut Engine,
    s: &mut MiniMapState,
    hud: u32,
    p: &Place,
    update_tiles: bool,
    update_fog: bool,
    nw: [f32; 2],
) {
    if p.show_flags_change & 2 != 0 {
        tile_set_bool(e, s.door_markers_rect, TRAIT_VISIBLE, p.show_flags & 2 != 0);
    }
    if p.show_flags & 2 == 0 {
        return;
    }
    if update_tiles {
        let k = local_map_pos_mults();
        let mut next = 0usize;
        for door in s.door_refs_list.clone() {
            let mut pos = ref_pos(e, door.door);
            if p.parent_world == 0 {
                pos = adjust_interior_pos(e, pos);
            }
            let place = [(pos[0] - nw[0]) * k[0], (pos[1] - nw[1]) * k[1]];
            // `cmpnltps` with (1, 1, 1, 1) on (x, y, x, y): either at 1 or past it (or a NaN), or
            // `movmskps ^ 0xC` (the high lanes compared with 1 too, the low ones reversed): JIP's
            // bounds are 0 to 1 less a hair each way as its mask reads them.
            if off_bounds(place) {
                continue;
            }
            let index = if next < s.door_marker_tiles.len() {
                next += 1;
                next - 1
            } else {
                let tile = add_tile_from_template(
                    e,
                    hud,
                    s.door_markers_rect,
                    b"MiniMapDoorMarkerTemplate",
                );
                let made = door_marker_tile(e, tile, s.door_marker_tiles.len() + 1);
                s.door_marker_tiles.push(made);
                next = s.door_marker_tiles.len();
                next - 1
            };
            let t = s.door_marker_tiles[index];
            value_set_float(e, t.x, place[0]);
            value_set_float(e, t.y, place[1]);
            let cell = door.linked_cell;
            let visited = cell == 0
                || cell == p.parent_cell
                || super::extra::has_type(e, cell + CELL_EXTRA, EXTRA_DETACH_TIME);
            value_set_bool(e, t.visited, visited);
            let mut t = t;
            t.in_use = true;
            if p.use_fog {
                t.vtx_alpha = vtx_alpha_ptr(e, s, place);
            }
            let shown = !p.use_fog || e.mem.u32(t.vtx_alpha) != 0;
            value_set_bool(e, t.visible, shown);
            s.door_marker_tiles[index] = t;
        }
        for i in next..s.door_marker_tiles.len() {
            if s.door_marker_tiles[i].in_use {
                s.door_marker_tiles[i].in_use = false;
                let v = s.door_marker_tiles[i].visible;
                value_set_bool(e, v, false);
            }
        }
    } else if update_fog {
        for t in s.door_marker_tiles.clone() {
            if t.in_use {
                let shown = e.mem.u32(t.vtx_alpha) != 0;
                value_set_bool(e, t.visible, shown);
            }
        }
    }
}

/// JIP's bounds test of a door's place on the local map: `cmpnltps` of (x, y, x, y) with (1, 1,
/// 0, 0) (`movq` of `PS_V3_One`), `movmskps`, `xor 0xC`: inside only when 0 <= x < 1 and 0 <= y < 1
/// (a NaN is outside).
pub(crate) fn off_bounds(place: [f32; 2]) -> bool {
    // `cmpnltps` itself: "not less than", true for a NaN (which `>=` would not be).
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let nlt = |a: f32, b: f32| !(a < b);
    let mask = u32::from(nlt(place[0], 1.0))
        | u32::from(nlt(place[1], 1.0)) << 1
        | u32::from(nlt(place[0], 0.0)) << 2
        | u32::from(nlt(place[1], 0.0)) << 3;
    mask ^ 0xc != 0
}

/// `DoorMarkerTile(tile)`: its visible, x, y, visited (user2) values, its depth `(count & 0xF) +
/// 18`; not in use until JIP sets it.
fn door_marker_tile(e: &mut Engine, tile: u32, count: usize) -> DoorMarkerTile {
    let made = DoorMarkerTile {
        visible: value_of(e, tile, TRAIT_VISIBLE),
        x: value_of(e, tile, TRAIT_USER0),
        y: value_of(e, tile, TRAIT_USER1),
        visited: value_of(e, tile, TRAIT_USER2),
        vtx_alpha: 0,
        in_use: false,
    };
    tile_set_float(e, tile, TRAIT_DEPTH, ((count & 0xf) + 18) as f32);
    made
}

/// A tile value's tile (+4).
const VALUE_PARENT: u32 = 0x04;

/// The quest markers (flag 4): both rects shown or hidden with the flag. When the targets changed
/// (JIP's flag, the active quest or the placed marker): the markers made again (the old tiles
/// destroyed), on the local map, or on the world map in an exterior: a tile per target (the door
/// of its first teleport link when it has one, else the target when it is in the player's cell or
/// world), from depth 35 up, then the placed marker's, coloured (191, 127, 255). Then each marker
/// placed where its target is, when the target moved (its x's bits) or on a full update.
#[allow(clippy::too_many_arguments)]
fn quest_markers(
    e: &mut Engine,
    s: &mut MiniMapState,
    hud: u32,
    player: u32,
    parent_cell: u32,
    parent_world: u32,
    world_map: bool,
    update_tiles: bool,
    upd_quest_targets: bool,
    nw: [f32; 2],
    show_flags: u32,
    show_flags_change: u32,
) {
    if show_flags_change & 4 != 0 {
        let on = show_flags & 4 != 0;
        tile_set_bool(e, s.l_quest_markers_rect, TRAIT_VISIBLE, on);
        tile_set_bool(e, s.w_quest_markers_rect, TRAIT_VISIBLE, on);
    }
    if show_flags & 4 == 0 {
        return;
    }
    let active = e.mem.u32(player + ACTIVE_QUEST);
    let placed = e.mem.u32(player + PLACED_MARKER);
    if upd_quest_targets || s.active_quest != active || s.placed_marker != placed {
        s.active_quest = active;
        s.placed_marker = placed;
        if !s.quest_markers.is_empty() {
            s.quest_markers.clear();
            destroy_all_children(e, s.l_quest_markers_rect);
            destroy_all_children(e, s.w_quest_markers_rect);
        }
        if !world_map || e.mem.u32(parent_cell + CELL_WORLD) != 0 {
            let rect = if world_map {
                s.w_quest_markers_rect
            } else {
                s.l_quest_markers_rect
            };
            let mut depth = 35;
            let mut targets = list_items(e, player + QUEST_TARGETS).into_iter();
            let mut done_placed = false;
            loop {
                let target = match targets.next() {
                    Some(0) => continue,
                    Some(t) => t,
                    None if !done_placed => {
                        done_placed = true;
                        player + PLACED_TARGET
                    }
                    None => break,
                };
                let mut object = e.mem.u32(target + TARGET_REF);
                if object == 0 {
                    continue;
                }
                if e.mem.u32(target + TARGET_LINK_COUNT) != 0 {
                    object = e.mem.u32(e.mem.u32(target + TARGET_LINKS));
                    if object == 0 {
                        continue;
                    }
                } else if parent_cell != e.mem.u32(object + REF_CELL)
                    && (parent_world == 0 || parent_world != parent_world_of(e, object))
                {
                    continue;
                }
                if let std::collections::btree_map::Entry::Vacant(slot) =
                    s.quest_markers.entry(object)
                {
                    let tile = add_tile_from_template(e, hud, rect, b"MiniMapQuestMarkerTemplate");
                    slot.insert(QuestMarkerTile {
                        pos: 0x7f7f_ffff,
                        x: value_of(e, tile, TRAIT_USER0),
                        y: value_of(e, tile, TRAIT_USER1),
                    });
                    tile_set_float(e, tile, TRAIT_DEPTH, depth as f32);
                    depth += 1;
                }
                if done_placed {
                    let x = s.quest_markers[&object].x;
                    let tile = e.mem.u32(x + VALUE_PARENT);
                    tile_set_float(e, tile, TRAIT_RED, 191.0);
                    tile_set_float(e, tile, TRAIT_GREEN, 127.0);
                    tile_set_float(e, tile, TRAIT_BLUE, 255.0);
                    break;
                }
            }
        }
    }
    let k = local_map_pos_mults();
    let keys: Vec<u32> = s.quest_markers.keys().copied().collect();
    for object in keys {
        let mut marker = s.quest_markers[&object];
        let curr = e.mem.u32(object + REF_POSITION);
        if marker.pos != curr {
            marker.pos = curr;
            s.quest_markers.insert(object, marker);
        } else if !update_tiles {
            continue;
        }
        let pos = ref_pos(e, object);
        let place = if world_map {
            world_map_pos_mults(pos, &s.root_dims, None)
        } else if parent_world != 0 {
            [(pos[0] - nw[0]) * k[0], (pos[1] - nw[1]) * k[1]]
        } else {
            let a = adjust_interior_pos(e, pos);
            [(a[0] - nw[0]) * k[0], (a[1] - nw[1]) * k[1]]
        };
        value_set_float(e, marker.x, place[0]);
        value_set_float(e, marker.y, place[1]);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::DestroyAllChildren
// (asm).
/// Each child tile of `tile` destroyed (its deleting destructor, vtable +0, with 1), the list
/// walked from its head with the next node read first.
fn destroy_all_children(e: &mut Engine, tile: u32) {
    let mut node = e.mem.u32(tile + 4);
    while node != 0 {
        let child = e.mem.u32(node + 8);
        node = e.mem.u32(node);
        if child != 0 {
            e.vcall(child, 0, &args![1u32]);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetParentWorld
// (asm).
/// The reference's world: its cell's, or its persistent cell's (`ExtraPersistentCell`); 0 for
/// none.
fn parent_world_of(e: &mut Engine, refr: u32) -> u32 {
    let cell = super::ln_fn_math::cell_or_persistent(e, refr).unwrap_or(0);
    if cell == 0 {
        0
    } else {
        e.mem.u32(cell + CELL_WORLD)
    }
}

#[cfg(test)]
#[path = "jip_fn_minimap_update_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "jip_fn_minimap_draw_tests.rs"]
mod draw_tests;
