//! `functions_jip/jip_fn_minimap.h` (JIP LN NVSE): `InitMiniMap`, the start of JIP MiniMap's HUD
//! map: the tiles of its HUD menu, its nine local map shapes, the camera, light and fog it draws
//! the local maps with, the fake water planes, every world's map markers by cell, and JIP's seven
//! code patches for it ([`super::jip_fn_minimap_seams`]). `UpdateMiniMap` comes with its own
//! part.
//!
//! JIP 57.30 writes no local map cache: its `SaveLocalMapTexture` (`local_maps\%s.(%d,%d).dds`) is
//! not called (its three calls in `UpdateMiniMap` are commented out), so the port has none.
//!
//! JohnnyGuitar NVSE: JIP reads the exe's bytes at `0x79D32F` for JohnnyGuitar's map marker icon
//! hook and, when they are JohnnyGuitar's, takes its icon routine. The port has no JohnnyGuitar
//! yet: the answer is the one without it (no routine; the game's icon table).
//!
//! JIP dereferences the mod's HUD tiles unchecked (its XML gives them): the port stops quietly at
//! a missing one (nv-rs's own: a changed XML does not stop the game).
//!
//! Threads: on the script's thread, as JIP's. The state has its own lock, not held across a game
//! call; the seams' words are lock-free.

use super::jip_fn_misc_ref_lines::{add_property, alpha_property, assign_geometry_props};
use super::jip_fn_ui::{component, value_of, value_set_float};
use super::nif_nodes::RETURN_THIS2;
use super::Handlers;
use crate::game_patches::gx::replace_object;
use engine_core::prelude::*;
use nvse::{handled, CommandArgs};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2873, init_mini_map),
];

// ---- the game's words ----

/// `g_dataHandler` (`*011C3F2C`) and its world list (+0x10, `tList`: data +0, next +4).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const WORLD_LIST: u32 = 0x10;
/// `g_TES` (`*011DEA10`): its cell grid (+8) and current interior (+0x34).
pub(crate) const TES: u32 = 0x011d_ea10;
pub(crate) const TES_GRID: u32 = 0x08;
const TES_INTERIOR: u32 = 0x34;
/// `g_HUDMainMenu` (`*011D96C0`) and a menu's tile (+4).
const HUD_MAIN_MENU: u32 = 0x011d_96c0;
const MENU_TILE: u32 = 0x04;
/// `g_shadowSceneNode` (`*011F91C8`), the object LOD root (`*011DEA18`), the game's fog property
/// (`*011DEB00`); the five scene nodes JIP keeps: sky, weather, terrain LOD, particle systems,
/// grass.
const SHADOW_SCENE_NODE: u32 = 0x011f_91c8;
const OBJECT_LOD_ROOT: u32 = 0x011d_ea18;
const GAME_FOG_PROPERTY: u32 = 0x011d_eb00;
const SCENE_NODES: [u32; 5] = [
    0x011d_eb34,
    0x011d_eda4,
    0x011d_86a8,
    0x011d_ed58,
    0x011c_a438,
];
/// A form's mod index (+0xF); a script's quest (+0x40) and a quest's delay time (+0x40).
const MOD_INDEX: u32 = 0x0f;
const SCRIPT_QUEST: u32 = 0x40;
const QUEST_DELAY: u32 = 0x40;
/// The byte of JIP MiniMap's script JIP changes (`scriptData + 0x2FE5`: `31 30 17` becomes
/// `33 30 17`) and the quest's delay time it gives it (1/60 s).
const SCRIPT_PATCH_AT: u32 = 0x2fe5;
const SCRIPT_PATCH_WAS: u32 = 0x0017_3031;
/// A tile's children (`DList` at +4: last node +8); a node's previous (+4) and tile (+8); a
/// value's number (+8); an image tile's node (+0x2C) and shader property (+0x40), whose texture
/// is at +0x60.
const TILE_LAST_CHILD: u32 = 0x08;
const NODE_PREV: u32 = 0x04;
const NODE_DATA: u32 = 0x08;
pub(crate) const VALUE_NUM: u32 = 0x08;
const TILE_NODE: u32 = 0x2c;
const TILE_SHADER: u32 = 0x40;
const SHADER_TEXTURE: u32 = 0x60;
/// `kTileValue_depth`, `_zoom`, `_rotateangle`, `_user0` to `_user2`.
const TRAIT_DEPTH: u32 = 0xfad;
const TRAIT_ZOOM: u32 = 0xfb8;
const TRAIT_ROTATE: u32 = 0xffa;
const TRAIT_USER0: u32 = 0x1004;
const TRAIT_USER1: u32 = 0x1005;
const TRAIT_USER2: u32 = 0x1006;
/// `NiAVObject`: count (+4), property list (+0x24: its first node's property +8), flags (+0x30),
/// local translation (+0x58), local scale (+0x64), world rotation (+0x68, nine floats); a node's
/// children (+0xA0, count +0xA6); `AddObject` (vtable +0xDC: the child, 1).
const REF_COUNT: u32 = 0x04;
const PROPERTY_HEAD: u32 = 0x24;
const FLAGS: u32 = 0x30;
const LOCAL_TRANSLATE: u32 = 0x58;
pub(crate) const LOCAL_SCALE: u32 = 0x64;
const WORLD_ROTATE: u32 = 0x68;
const CHILDREN: u32 = 0xa0;
const CHILD_COUNT: u32 = 0xa6;
const ADD_OBJECT_SLOT: u32 = 0xdc;
/// `NiGeometry`'s alpha property (+0x9C) and data (+0xB8); `NiTriShapeData`: vertex count
/// (+8, `u16`), vertices (+0x20), normals (+0x24), colours (+0x28), texture coordinates (+0x2C),
/// triangle count (+0x40, `u16`), triangle points (+0x44), triangles (+0x48).
const GEOMETRY_ALPHA: u32 = 0x9c;
const GEOMETRY_DATA: u32 = 0xb8;
const DATA_VERTEX_COUNT: u32 = 0x08;
const DATA_VERTICES: u32 = 0x20;
const DATA_NORMALS: u32 = 0x24;
const DATA_COLOURS: u32 = 0x28;
const DATA_UVS: u32 = 0x2c;
const DATA_TRIANGLE_COUNT: u32 = 0x40;
const DATA_TRIANGLE_POINTS: u32 = 0x44;
const DATA_TRIANGLES: u32 = 0x48;
/// `NiCamera`: its frustum (+0xDC: view port l r t b, near +0xEC, far +0xF0, ortho +0xF4), nearest
/// near plane (+0xF8), far/near ratio (+0xFC), level of detail (+0x110).
const CAMERA_VIEW_PORT: u32 = 0xdc;
const CAMERA_NEAR: u32 = 0xec;
const CAMERA_FAR: u32 = 0xf0;
const CAMERA_ORTHO: u32 = 0xf4;
const CAMERA_MIN_NEAR: u32 = 0xf8;
const CAMERA_FAR_NEAR: u32 = 0xfc;
const CAMERA_LOD: u32 = 0x110;
/// `NiDirectionalLight`: ambient (+0xC8), diffuse (+0xD4), fog colour (+0xE0), direction (+0xF0).
const LIGHT_AMBIENT: u32 = 0xc8;
const LIGHT_DIFFUSE: u32 = 0xd4;
const LIGHT_FOG: u32 = 0xe0;
const LIGHT_DIRECTION: u32 = 0xf0;
/// `BSFogProperty` (0x64 bytes): colour (+0x20), start (+0x2C) and end (+0x30) distances.
const FOG_SIZE: u32 = 0x64;
const FOG_COLOUR: u32 = 0x20;
const FOG_START: u32 = 0x2c;
const FOG_END: u32 = 0x30;
/// `NiAlphaProperty`'s flags (+0x18, `u16`): JIP's 0x8D for the local maps.
const ALPHA_FLAGS: u32 = 0x18;
/// `TESWorldSpace`: its persistent cell (+0x34), parent flags (+0x4E, 4: use the parent's map),
/// parent (+0x70), usable map width (+0x80), NW and SE cell corners (+0x88, four `i16`), map scale
/// and offsets (+0x90, four floats).
const WORLD_CELL: u32 = 0x34;
const WORLD_PARENT_FLAGS: u32 = 0x4e;
const WORLD_PARENT: u32 = 0x70;
const WORLD_USABLE_X: u32 = 0x80;
const WORLD_CORNERS: u32 = 0x88;
const WORLD_MAP_SCALE: u32 = 0x90;
/// `TESObjectCELL`'s references (`tList` at +0xAC); a reference's base (+0x20; the map marker's id
/// 0x10 at +0xC), position (+0x30) and extra data (+0x44); `ExtraMapMarker`'s data (+0xC).
const CELL_OBJECTS: u32 = 0xac;
const REF_BASE: u32 = 0x20;
const FORM_ID: u32 = 0x0c;
const MAP_MARKER_ID: u32 = 0x10;
const REF_POSITION: u32 = 0x30;
const REF_EXTRA: u32 = 0x44;
const EXTRA_MAP_MARKER: u32 = 0x2c;
const EXTRA_DATA: u32 = 0x0c;
/// The game's calls: `NiCamera::Create`, `NiDirectionalLight::Create`, `NiAlphaProperty::Create`,
/// `NiNode::Create` (`cdecl`, none), `Ni_Alloc` (`cdecl`, the size), `NiTriShapeData`'s
/// constructor (`thiscall`, nine words), `NiTriShape`'s (`thiscall`, the data), the no-lighting
/// shader property's (`thiscall`), `BaseExtraList::GetByType` (`thiscall`, the type), the fixed
/// string maker (`cdecl`, the text).
const CAMERA_CREATE: u32 = 0x00a7_1430;
const LIGHT_CREATE: u32 = 0x00a7_5d50;
const ALPHA_CREATE: u32 = 0x00a5_ceb0;
const NODE_CREATE: u32 = 0x00a5_f030;
pub(crate) const NI_ALLOC: u32 = 0x00aa_13e0;
const SHAPE_DATA_CONSTRUCT: u32 = 0x00a7_b630;
const SHAPE_CONSTRUCT: u32 = 0x00a7_4480;
const NO_LIGHTING_CONSTRUCT: u32 = 0x00b6_fc90;
const GET_BY_TYPE: u32 = 0x0041_0220;
const ADD_STRING: u32 = 0x00a5_b690;
/// The node's "hidden" flag (bit 0 of +0x30).
const HIDDEN: u8 = 1;
/// The local maps' zoom in an interior (`kInteriorZoomMod`).
const INTERIOR_ZOOM: f32 = 0.25;
/// `kVtbl_NiRenderedTexture`'s format the local maps are drawn in at start (`D3DFMT_X8R8G8B8`).
const START_FORMAT: u32 = 22;

// ---- the state ----

/// A map marker of a world's cell (`MapMarkerInfo`): the reference, its marker data, its place on
/// the world map (0 to 1, JIP's `GetWorldMapPosMults`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerInfo {
    pub refr: u32,
    pub data: u32,
    pub pos: [f32; 2],
}

/// `WorldDimensions` (JIP's): the world map's corner and size, its scale and offsets (`posMod`
/// used when the scale is not 1), and the packed cell coordinate of the last place.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldDimensions {
    pub min: [f32; 2],
    pub size: [f32; 2],
    pub scale: f32,
    pub offset: [f32; 2],
    pub pos_mod: [f32; 2],
    pub coord: u32,
}

/// JIP MiniMap's state (JIP's statics of `jip_fn_minimap.h`) as `InitMiniMap` leaves it.
#[derive(Default)]
pub struct MiniMapState {
    /// `s_miniMapIndex`: JIP MiniMap.esp's mod index, 0 until found.
    pub index: u8,
    /// `s_miniMapMode` (the mode value: not 0 once started), `s_miniMapScale`, `s_localMapZoom`,
    /// `s_pcMarkerRotate`, `s_miniMapPosX`, `s_miniMapPosY`, `s_worldMapZoom`.
    pub mode: u32,
    pub scale: u32,
    pub local_map_zoom: u32,
    pub pc_marker_rotate: u32,
    pub pos_x: u32,
    pub pos_y: u32,
    pub world_map_zoom: u32,
    /// `s_worldMapRect`, `s_worldMapTile`, `s_mapMarkersRect`, `s_wQuestMarkersRect`,
    /// `s_doorMarkersRect`, `s_lQuestMarkersRect`.
    pub world_map_rect: u32,
    pub world_map_tile: u32,
    pub map_markers_rect: u32,
    pub w_quest_markers_rect: u32,
    pub door_markers_rect: u32,
    pub l_quest_markers_rect: u32,
    /// `s_localMapShapes`, `s_localMapShapeDatas`, `s_tileShaderProps`.
    pub shapes: [u32; 9],
    pub shape_datas: [u32; 9],
    pub shader_props: [u32; 9],
    /// `s_fLocalMapZoomCurr`, `s_isInExterior`.
    pub zoom_current: f32,
    pub in_exterior: bool,
    /// `s_localMapCamera`, `s_mmDirectionalLight`, `s_mmFogProperty`, `s_fakeWaterPlanes`,
    /// `s_shadowSceneNodes`, `s_BlackPlane01`.
    pub camera: u32,
    pub light: u32,
    pub fog: u32,
    pub water_planes: u32,
    pub scene_nodes: [u32; 5],
    pub black_plane: u32,
    /// `s_worldMapMarkers`: by root world, by packed cell coordinate, in the cells' order.
    pub world_markers: BTreeMap<u32, HashMap<u32, Vec<MarkerInfo>>>,
    /// `s_rootWorldDimensions`.
    pub root_dims: WorldDimensions,
    /// `s_mapMarkerTiles` (the marker tiles made so far) and `s_renderedMapMarkers` (by packed
    /// cell coordinate, the tiles showing that cell's markers): `UpdateMiniMap` fills them.
    pub marker_tiles: Vec<MarkerTile>,
    pub rendered_markers: HashMap<u32, Vec<usize>>,
}

/// `MapMarkerTile`: a marker tile's visible, x, y and file name values, and whether it shows a
/// marker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MarkerTile {
    pub visible: u32,
    pub x: u32,
    pub y: u32,
    pub filename: u32,
    pub in_use: bool,
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// FreeCellMapMarkers.
/// The cell's rendered markers let go: each of its tiles not in use and hidden (`SetInUse(0)`),
/// the cell's entry removed.
pub(crate) fn free_cell_map_markers(e: &mut Engine, coord: u32) {
    let hidden: Vec<u32> = {
        let mut s = state(e);
        let Some(list) = s.rendered_markers.remove(&coord) else {
            return;
        };
        list.iter()
            .filter_map(|&i| {
                let tile = s.marker_tiles.get_mut(i)?;
                tile.in_use = false;
                Some(tile.visible)
            })
            .collect()
    };
    for visible in hidden {
        super::jip_fn_ui::value_set_bool(e, visible, false);
    }
}

/// The state, with JIP's lock-free words beside it: the patches are in (`InitMiniMap` wrote
/// them), `s_discoveredLocation`, `s_updateQuestTargets`, the local maps' texture format
/// (`s_mmTextureParams.d3dFormat`), `s_updateFogOfWar`, `s_cellNorthRotation` and
/// `s_northRotationMods` (as bits).
pub struct MiniMap {
    pub state: Mutex<MiniMapState>,
    pub patched: AtomicBool,
    pub discovered_location: AtomicBool,
    pub update_quest_targets: AtomicBool,
    pub texture_format: AtomicU32,
    pub update_fog_of_war: AtomicBool,
    pub north_rotation: AtomicU32,
    pub north_rotation_mods: [AtomicU32; 4],
}

impl Default for MiniMap {
    fn default() -> Self {
        MiniMap {
            state: Mutex::default(),
            patched: AtomicBool::new(false),
            discovered_location: AtomicBool::new(false),
            update_quest_targets: AtomicBool::new(false),
            texture_format: AtomicU32::new(START_FORMAT),
            update_fog_of_war: AtomicBool::new(false),
            north_rotation: AtomicU32::new(0),
            north_rotation_mods: Default::default(),
        }
    }
}

/// The run's minimap (made before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<MiniMap>();
}

pub(crate) fn mini_map(e: &mut Engine) -> &MiniMap {
    e.ext_free::<MiniMap>()
}

pub(crate) fn state(e: &mut Engine) -> std::sync::MutexGuard<'_, MiniMapState> {
    mini_map(e)
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Whether `InitMiniMap` has written its patches.
pub fn patched(e: &mut Engine) -> bool {
    mini_map(e).patched.load(Ordering::Acquire)
}

// ---- small reads ----

/// A tile's last child node, 0 for none.
fn last_child(e: &Engine, tile: u32) -> u32 {
    e.mem.u32(tile + TILE_LAST_CHILD)
}

/// A node's previous node, 0 for none or no node.
fn prev(e: &Engine, node: u32) -> u32 {
    if node == 0 {
        0
    } else {
        e.mem.u32(node + NODE_PREV)
    }
}

/// A node's tile, 0 for none or no node.
fn data(e: &Engine, node: u32) -> u32 {
    if node == 0 {
        0
    } else {
        e.mem.u32(node + NODE_DATA)
    }
}

/// `Tile::SetFloat(id, value)`: the value of trait `id`, when the tile has it, set.
fn tile_set_float(e: &mut Engine, tile: u32, id: u32, num: f32) {
    let value = value_of(e, tile, id);
    if value != 0 {
        value_set_float(e, value, num);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESWorldSpace::GetRootMapWorld
// (asm).
/// The world whose map `world` shows: up the parents while the parent flag 4 (use the parent's
/// map) is set.
pub(crate) fn root_map_world(e: &Engine, world: u32) -> u32 {
    let mut w = world;
    loop {
        let parent = e.mem.u32(w + WORLD_PARENT);
        if parent == 0 || e.mem.u8(w + WORLD_PARENT_FLAGS) & 4 == 0 {
            return w;
        }
        w = parent;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetMapMarkerData (asm).
/// The reference's map marker data: a reference of the map marker (base id 0x10) with its
/// `ExtraMapMarker`; 0 otherwise.
pub(crate) fn map_marker_data(e: &mut Engine, refr: u32) -> u32 {
    let base = e.mem.u32(refr + REF_BASE);
    if base == 0 || e.mem.u32(base + FORM_ID) != MAP_MARKER_ID {
        return 0;
    }
    let extra = e
        .call(GET_BY_TYPE, &args![refr + REF_EXTRA, EXTRA_MAP_MARKER])
        .u32();
    if extra == 0 {
        0
    } else {
        e.mem.u32(extra + EXTRA_DATA)
    }
}

// ---- the world map's dimensions ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// WorldDimensions::InitDimensions (asm).
/// The world map's corner and size from the world's NW and SE cell corners (`i16`, +0x88): the
/// corner `(NWx, NWy + 1) * 4096`, the size the processor's estimate of 1/x of `(SEx + 1 - NWx,
/// SEy - NWy - 1) * 4096`, times 0.796875 (`rcpps`, as JIP's code gets it).
pub(crate) fn init_dimensions(e: &Engine, dims: &mut WorldDimensions, world: u32) {
    let corner = |i: u32| i32::from(e.mem.u16(world + WORLD_CORNERS + 2 * i) as i16);
    let (nw_x, nw_y, se_x, se_y) = (corner(0), corner(1), corner(2), corner(3));
    // `paddw` adds in 16 bits, then `psrad` sign-extends each word back.
    let w16 = |v: i32| i32::from(v as i16);
    let width = w16(se_x + 1).wrapping_sub(nw_x);
    let height = se_y.wrapping_sub(w16(nw_y + 1));
    let k = f32::from_bits(0x3f4c_0000);
    dims.min = [
        (nw_x.wrapping_shl(12)) as f32,
        (w16(nw_y + 1).wrapping_shl(12)) as f32,
    ];
    dims.size = [
        super::havok_refs::rcp_estimate(width.wrapping_shl(12) as f32) * k,
        super::havok_refs::rcp_estimate(height.wrapping_shl(12) as f32) * k,
    ];
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// WorldDimensions::InitPosMods (asm).
/// The world's map scale and offsets (+0x90, four floats) as the scale and `offset`; with a scale
/// other than 1, `posMod = (0.5 - scale * 0.5) * (sum of the two corner pairs) + offsets`.
pub(crate) fn init_pos_mods(e: &Engine, dims: &mut WorldDimensions, world: u32) {
    let at = world + WORLD_MAP_SCALE;
    let f = |i: u32| e.mem.f32(at + 4 * i);
    dims.scale = f(0);
    dims.offset = [f(1), f(2)];
    // `movups [ecx+0x10]` stores four floats: the fourth lands on `posMod`'s first.
    dims.pos_mod[0] = f(3);
    if e.mem.u32(at) == 0x3f80_0000 {
        return;
    }
    // `xmm1 = 0.5 - scale * 0.5` in both lanes; `[edx+0x10]` is the world's min and max (+0xA0):
    // shuffled to (min.x, max.x, min.y, max.y) and added in pairs.
    let half = 0.5f32;
    let m = half - dims.scale * half;
    let g = |i: u32| e.mem.f32(at + 0x10 + 4 * i);
    let sums = [g(0) + g(2), g(1) + g(3)];
    dims.pos_mod = [sums[0] * m + f(1), sums[1] * m + f(2)];
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// GetWorldMapPosMults (asm).
/// A world position's place on the world map (each 0 to 1 over the map, plus 0.1015625), and the
/// packed cell coordinate of the position after the scale and offset (`x >> 12` high word,
/// `y >> 12` low) into `coord` when asked.
pub(crate) fn world_map_pos_mults(
    pos: [f32; 2],
    dims: &WorldDimensions,
    coord: Option<&mut u32>,
) -> [f32; 2] {
    let (mut x, mut y) = (pos[0], pos[1]);
    let add = if dims.scale.to_bits() != 0x3f80_0000 {
        x *= dims.scale;
        y *= dims.scale;
        dims.pos_mod
    } else {
        dims.offset
    };
    x += add[0];
    y += add[1];
    let k = f32::from_bits(0x3dd0_0000);
    let out = [
        (x - dims.min[0]) * dims.size[0] + k,
        (y - dims.min[1]) * dims.size[1] + k,
    ];
    if let Some(c) = coord {
        let cx = (crate::game_patches::gd::cvtss2si(x) >> 12) as u32;
        let cy = (crate::game_patches::gd::cvtss2si(y) >> 12) as u32;
        *c = (cx << 16) | (cy & 0xffff);
    }
    out
}

// ---- the 3D pieces ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// NiNode::AddPropertyRecurse (asm).
/// `property` added to every geometry under `node` (nodes walked, geometry by its vtable +0x18).
fn add_property_recurse(e: &mut Engine, node: u32, property: u32) {
    for child in children(e, node) {
        if is_node(e, child) {
            add_property_recurse(e, child, property);
        } else if is_geometry(e, child) {
            add_property(e, child, property);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// NiNode::AddNoLightingPropertyRecurse (asm).
/// A new no-lighting shader property (0x80 bytes, `00B6FC90`) added to every geometry under
/// `node`.
fn add_no_lighting_recurse(e: &mut Engine, node: u32) {
    for child in children(e, node) {
        if is_node(e, child) {
            add_no_lighting_recurse(e, child);
        } else if is_geometry(e, child) {
            let block = e.call(NI_ALLOC, &args![0x80u32]).u32();
            let property = e.call(NO_LIGHTING_CONSTRUCT, &args![block]).u32();
            add_property(e, child, property);
        }
    }
}

/// The node's children (its array's slots, empty ones left out).
fn children(e: &Engine, node: u32) -> Vec<u32> {
    let array = e.mem.u32(node + CHILDREN);
    let count = u32::from(e.mem.u16(node + CHILD_COUNT));
    (0..count)
        .map(|i| e.mem.u32(array + 4 * i))
        .filter(|&c| c != 0)
        .collect()
}

fn is_node(e: &Engine, object: u32) -> bool {
    e.mem.u32(e.mem.u32(object) + 0xc) == super::nif_nodes::RETURN_THIS
}

fn is_geometry(e: &Engine, object: u32) -> bool {
    e.mem.u32(e.mem.u32(object) + 0x18) == RETURN_THIS2
}

/// `kWaterPlaneColor` and `kWaterPlanePos` (nine x, y pairs).
const WATER_PLANE_COLOUR: [u32; 4] = [0x3db8_b8b9, 0x3e4c_cccd, 0x3e3c_bcbd, 0x3f38_b8b9];
const WATER_PLANE_POS: [u32; 18] = [
    0x4500_0000,
    0xc620_0000,
    0x4500_0000,
    0xc5c0_0000,
    0x4500_0000,
    0xc500_0000,
    0x45c0_0000,
    0xc620_0000,
    0x45c0_0000,
    0xc5c0_0000,
    0x45c0_0000,
    0xc500_0000,
    0x4620_0000,
    0xc620_0000,
    0x4620_0000,
    0xc5c0_0000,
    0x4620_0000,
    0xc500_0000,
];

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// CreateFakeWaterPlanes (asm).
/// A node (counted 2) of nine hidden water squares (one shape data: four corners at +-2048, the
/// water colour at each, two triangles), placed at JIP's nine positions; without lighting, with
/// JIP's alpha property, the geometry properties assigned.
fn create_fake_water_planes(e: &mut Engine) -> u32 {
    let node = e.call(NODE_CREATE, &args![]).u32();
    e.mem.set_u32(node + REF_COUNT, 2);
    let block = super::heap_alloc(e, 0x7c);
    let corners: [[f32; 3]; 4] = [
        [-2048.0, 2048.0, 0.0],
        [-2048.0, -2048.0, 0.0],
        [2048.0, 2048.0, 0.0],
        [2048.0, -2048.0, 0.0],
    ];
    for (i, c) in corners.iter().enumerate() {
        for (j, v) in c.iter().enumerate() {
            e.mem.set_f32(block + 12 * i as u32 + 4 * j as u32, *v);
        }
    }
    for i in 0..4 {
        for (j, bits) in WATER_PLANE_COLOUR.iter().enumerate() {
            e.mem.set_u32(block + 0x30 + 0x10 * i + 4 * j as u32, *bits);
        }
    }
    let triangles = block + 0x70;
    e.mem.set_u32(triangles, 0x0001_0000);
    e.mem.set_u32(triangles + 4, 0x0002_0002);
    e.mem.set_u32(triangles + 8, 0x0003_0001);
    let data_block = e.call(NI_ALLOC, &args![0x58u32]).u32();
    let shape_data = e
        .call(
            SHAPE_DATA_CONSTRUCT,
            &args![
                data_block,
                4u32,
                block,
                0u32,
                block + 0x30,
                0u32,
                0u32,
                0u32,
                2u32,
                triangles
            ],
        )
        .u32();
    for i in 0..9 {
        let shape_block = e.call(NI_ALLOC, &args![0xc4u32]).u32();
        let shape = e
            .call(SHAPE_CONSTRUCT, &args![shape_block, shape_data])
            .u32();
        let flags = e.mem.u8(shape + FLAGS);
        e.mem.set_u8(shape + FLAGS, flags | HIDDEN);
        e.mem
            .set_u32(shape + LOCAL_TRANSLATE, WATER_PLANE_POS[2 * i]);
        e.mem
            .set_u32(shape + LOCAL_TRANSLATE + 4, WATER_PLANE_POS[2 * i + 1]);
        e.vcall(node, ADD_OBJECT_SLOT, &args![shape, 1u32]);
    }
    add_no_lighting_recurse(e, node);
    let alpha = alpha_property(e);
    add_property_recurse(e, node, alpha);
    assign_geometry_props(e, node);
    node
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h: UpdateTileScales
// (asm).
/// The nine local map shapes' scale `(0.5 + zoom) * scale value`, and the zoom value set to
/// `zoom`.
pub(crate) fn update_tile_scales(e: &mut Engine, s: &MiniMapState, zoom: f32) {
    let scale = (0.5f32 + zoom) * e.mem.f32(s.scale + VALUE_NUM);
    for &shape in s.shapes.iter().rev() {
        e.mem.set_f32(shape + LOCAL_SCALE, scale);
    }
    value_set_float(e, s.local_map_zoom, zoom);
}

// ---- InitMiniMap ----

/// The local maps' mesh: 17 x 17 vertices 8 apart (x across, z down from 0), their texture
/// coordinates 1/16 apart, and the 512 triangles of the 16 x 16 squares.
fn local_map_mesh(e: &mut Engine) -> (u32, u32, u32) {
    let vertices = super::heap_alloc(e, 0x121 * 12);
    let uvs = super::heap_alloc(e, 0x121 * 8);
    let mut at = 0;
    for ix in 0..0x11u32 {
        for iz in 0..0x11u32 {
            let (x, z) = (ix as f32 * 8.0, -(iz as f32) * 8.0);
            e.mem.set_f32(vertices + 12 * at, x);
            e.mem.set_f32(vertices + 12 * at + 4, 0.0);
            e.mem.set_f32(vertices + 12 * at + 8, z);
            e.mem.set_f32(uvs + 8 * at, ix as f32 * 0.0625);
            e.mem.set_f32(uvs + 8 * at + 4, iz as f32 * 0.0625);
            at += 1;
        }
    }
    let triangles = super::heap_alloc(e, 0x200 * 6);
    let mut t = triangles;
    let (mut vtx1, mut vtx2) = (0u16, 0x11u16);
    while vtx2 < 0x120 {
        for _ in 0..0x10 {
            e.mem.set_u16(t, vtx2);
            e.mem.set_u16(t + 2, vtx1);
            vtx1 += 1;
            e.mem.set_u16(t + 4, vtx1);
            e.mem.set_u16(t + 6, vtx1);
            e.mem.set_u16(t + 10, vtx2);
            vtx2 += 1;
            e.mem.set_u16(t + 8, vtx2);
            t += 12;
        }
        vtx1 += 1;
        vtx2 += 1;
    }
    (vertices, uvs, triangles)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_minimap.h:
// Cmd_InitMiniMap_Execute.
/// `InitMiniMap`, once, from JIP MiniMap.esp's script only: the mod's HUD tiles read (its scale,
/// the player marker's rotation, the mode and place, the local and world map zooms and rects), the
/// script's update made 60 a second, the nine local map shapes given their mesh and JIP's alpha
/// property, the local maps' camera (orthographic, looking down), light and fog, the fake water
/// planes under the object LOD root, every world's map markers by cell, and JIP's patches written
/// ([`super::jip_fn_minimap_seams`]). No result.
fn init_mini_map(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if state(e).mode != 0 {
        return handled(true);
    }
    if state(e).index == 0 {
        let index = super::jip_fn_utility::mod_index(e, b"JIP MiniMap.esp");
        if index != 0xff {
            state(e).index = index as u8;
        }
    }
    if a.script == 0 || e.mem.u8(a.script + MOD_INDEX) != state(e).index {
        return handled(true);
    }
    let menu = e.mem.u32(HUD_MAIN_MENU);
    let root = if menu == 0 {
        0
    } else {
        e.mem.u32(menu + MENU_TILE)
    };
    let Some((tile, None)) = (root != 0)
        .then(|| component(e, root, b"JIPMiniMap"))
        .flatten()
    else {
        return handled(true);
    };
    if a.data != 0 && e.mem.u32(a.data + SCRIPT_PATCH_AT) == SCRIPT_PATCH_WAS {
        e.mem.set_u8(a.data + SCRIPT_PATCH_AT, 0x33);
        // JIP writes through the quest unchecked (the mod's script is a quest's).
        let quest = e.mem.u32(a.script + SCRIPT_QUEST);
        if quest != 0 {
            e.mem.set_f32(quest + QUEST_DELAY, 1.0 / 60.0);
        }
    }
    let Some(found) = read_tiles(e, tile) else {
        return handled(true);
    };
    let (vertices, uvs, triangles) = local_map_mesh(e);
    let colours = super::heap_alloc(e, 0xa29 * 16);
    for i in 0..0xa29 * 4 {
        e.mem.set_u32(colours + 4 * i, 0);
    }
    let alpha = e.call(ALPHA_CREATE, &args![]).u32();
    e.mem.set_u16(alpha + ALPHA_FLAGS, 0x8d);
    let mut s = MiniMapState {
        index: state(e).index,
        ..found
    };
    for i in 0..9 {
        let local_tile = s.shader_props[i];
        let node = e.mem.u32(local_tile + TILE_NODE);
        let shape = e.mem.u32(e.mem.u32(node + CHILDREN));
        let shape_data = e.mem.u32(shape + GEOMETRY_DATA);
        let shader = e.mem.u32(local_tile + TILE_SHADER);
        s.shapes[i] = shape;
        s.shape_datas[i] = shape_data;
        s.shader_props[i] = shader;
        replace_object(e, shader + SHADER_TEXTURE, 0);
        e.mem.set_u16(shape_data + DATA_VERTEX_COUNT, 0x121);
        e.mem.set_u16(shape_data + DATA_TRIANGLE_COUNT, 0x200);
        e.mem.set_u32(shape_data + DATA_TRIANGLE_POINTS, 0x600);
        for (field, new) in [
            (DATA_VERTICES, vertices),
            (DATA_NORMALS, 0),
            (DATA_COLOURS, colours + 0x121 * 16 * i as u32),
            (DATA_UVS, uvs),
            (DATA_TRIANGLES, triangles),
        ] {
            let old = e.mem.u32(shape_data + field);
            super::heap_free(e, old);
            e.mem.set_u32(shape_data + field, new);
        }
        // JIP writes into the first property's node unchecked (the mod's shapes have one).
        let head = e.mem.u32(shape + PROPERTY_HEAD);
        if head != 0 {
            replace_object(e, head + NODE_DATA, alpha);
        }
        replace_object(e, shape + GEOMETRY_ALPHA, alpha);
    }
    s.zoom_current = e.mem.f32(s.local_map_zoom + VALUE_NUM);
    s.in_exterior = true;
    let tes = e.mem.u32(TES);
    if tes != 0 && e.mem.u32(tes + TES_INTERIOR) != 0 {
        s.in_exterior = false;
        s.zoom_current += INTERIOR_ZOOM;
    }
    update_tile_scales(e, &s, s.zoom_current);
    s.camera = make_camera(e);
    s.light = make_light(e);
    s.fog = make_fog(e);
    s.water_planes = create_fake_water_planes(e);
    let lod_root = e.mem.u32(OBJECT_LOD_ROOT);
    e.vcall(lod_root, ADD_OBJECT_SLOT, &args![s.water_planes, 1u32]);
    s.world_markers = world_markers(e);
    for (slot, at) in s.scene_nodes.iter_mut().zip(SCENE_NODES) {
        *slot = e.mem.u32(at);
    }
    s.black_plane = super::with_guest_text(e, b"BlackPlane01", |e, text| {
        e.call(ADD_STRING, &[text]).eax
    });
    *state(e) = s;
    mini_map(e).patched.store(true, Ordering::Release);
    handled(true)
}

/// The tiles `InitMiniMap` reads under `JIPMiniMap` (`tile`), into a state; the nine local map
/// image tiles go in `shader_props` for the caller. `None` when one is missing.
fn read_tiles(e: &mut Engine, tile: u32) -> Option<MiniMapState> {
    let some = |v: u32| (v != 0).then_some(v);
    let mut s = MiniMapState {
        scale: some(value_of(e, tile, TRAIT_USER1))?,
        ..MiniMapState::default()
    };
    let node = some(prev(e, last_child(e, tile)))?;
    let marker = some(data(e, node))?;
    s.pc_marker_rotate = value_of(e, marker, TRAIT_ROTATE);
    tile_set_float(e, marker, TRAIT_DEPTH, 34.0);
    let frame = some(data(e, prev(e, node)))?;
    s.mode = some(value_of(e, frame, TRAIT_USER0))?;
    s.pos_x = value_of(e, frame, TRAIT_USER1);
    s.pos_y = value_of(e, frame, TRAIT_USER2);
    let node = some(prev(e, last_child(e, frame)))?;
    let local = some(data(e, node))?;
    s.local_map_zoom = some(value_of(e, local, TRAIT_ZOOM))?;
    s.world_map_rect = some(data(e, prev(e, node)))?;
    s.world_map_zoom = value_of(e, s.world_map_rect, TRAIT_ZOOM);
    let mut node = last_child(e, local);
    for i in 0..9 {
        let image = some(data(e, node))?;
        let image_node = some(e.mem.u32(image + TILE_NODE))?;
        some(e.mem.u32(e.mem.u32(image_node + CHILDREN)))?;
        some(e.mem.u32(image + TILE_SHADER))?;
        s.shader_props[i] = image;
        node = prev(e, node);
    }
    s.door_markers_rect = some(data(e, node))?;
    s.l_quest_markers_rect = some(data(e, prev(e, node)))?;
    let node = some(last_child(e, s.world_map_rect))?;
    s.world_map_tile = some(data(e, node))?;
    let node = prev(e, node);
    s.map_markers_rect = some(data(e, node))?;
    s.w_quest_markers_rect = some(data(e, prev(e, node)))?;
    Some(s)
}

/// `s_localMapCamera`: orthographic, looking down (world rotation 0 1 / -1 0 in its x and z),
/// view port +-2048, near 1, far 131072, nearest near plane 1, far/near ratio 70768, level of
/// detail 0.001; counted 2, under the shadow scene node.
fn make_camera(e: &mut Engine) -> u32 {
    let camera = e.call(CAMERA_CREATE, &args![]).u32();
    e.mem.set_u32(camera + REF_COUNT, 2);
    for (i, v) in [(0, 0.0f32), (2, 1.0), (6, -1.0), (8, 0.0)] {
        e.mem.set_f32(camera + WORLD_ROTATE + 4 * i, v);
    }
    for (i, v) in [-2048.0f32, 2048.0, 2048.0, -2048.0].iter().enumerate() {
        e.mem.set_f32(camera + CAMERA_VIEW_PORT + 4 * i as u32, *v);
    }
    e.mem.set_f32(camera + CAMERA_NEAR, 1.0);
    e.mem.set_f32(camera + CAMERA_FAR, 131_072.0);
    e.mem.set_u8(camera + CAMERA_ORTHO, 1);
    e.mem.set_f32(camera + CAMERA_MIN_NEAR, 1.0);
    e.mem.set_f32(camera + CAMERA_FAR_NEAR, 70_768.0);
    e.mem.set_f32(camera + CAMERA_LOD, 0.001);
    let scene = e.mem.u32(SHADOW_SCENE_NODE);
    e.vcall(scene, ADD_OBJECT_SLOT, &args![camera, 1u32]);
    camera
}

fn set_colour(e: &mut Engine, at: u32, rgb: [f32; 3]) {
    for (i, v) in rgb.iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, *v);
    }
}

/// `NRGB(r, g, b)`: each over 255 in `f32`.
fn nrgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r, g, b].map(|c| f32::from(c) / 255.0)
}

/// `s_mmDirectionalLight`: counted 2, ambient 175 175 175, diffuse 169 167 140, no fog colour,
/// its direction JIP's.
fn make_light(e: &mut Engine) -> u32 {
    let light = e.call(LIGHT_CREATE, &args![]).u32();
    e.mem.set_u32(light + REF_COUNT, 2);
    set_colour(e, light + LIGHT_AMBIENT, nrgb(175, 175, 175));
    set_colour(e, light + LIGHT_DIFFUSE, nrgb(169, 167, 140));
    set_colour(e, light + LIGHT_FOG, [0.0; 3]);
    // JIP's literals (`0.6154797F, -0.5235988F`), not a constant of its own.
    #[allow(clippy::approx_constant)]
    let direction = [0.615_479_7, -0.523_598_8, 0.615_479_7];
    set_colour(e, light + LIGHT_DIRECTION, direction);
    light
}

/// `s_mmFogProperty`: a copy of the game's fog property (0x64 bytes), colour 23 51 47, start and
/// end `FLT_MAX`.
fn make_fog(e: &mut Engine) -> u32 {
    let fog = e.call(NI_ALLOC, &args![FOG_SIZE]).u32();
    let game = e.mem.u32(GAME_FOG_PROPERTY);
    for i in 0..FOG_SIZE / 4 {
        let word = e.mem.u32(game + 4 * i);
        e.mem.set_u32(fog + 4 * i, word);
    }
    set_colour(e, fog + FOG_COLOUR, nrgb(23, 51, 47));
    e.mem.set_f32(fog + FOG_START, f32::MAX);
    e.mem.set_f32(fog + FOG_END, f32::MAX);
    fog
}

/// Every world's map markers (`s_worldMapMarkers`): for each world of the data handler's list that
/// has a map (or a parent) and a persistent cell with references, its markers by the packed cell
/// coordinate of their place on the root world's map.
fn world_markers(e: &mut Engine) -> BTreeMap<u32, HashMap<u32, Vec<MarkerInfo>>> {
    let mut out: BTreeMap<u32, HashMap<u32, Vec<MarkerInfo>>> = BTreeMap::new();
    let handler = e.mem.u32(DATA_HANDLER);
    let mut node = handler + WORLD_LIST;
    let mut dims = WorldDimensions::default();
    let mut root_world = 0;
    while node != 0 {
        let world = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if world == 0 {
            continue;
        }
        if e.mem.u32(world + WORLD_PARENT) == 0 && e.mem.u32(world + WORLD_USABLE_X) == 0 {
            continue;
        }
        let cell = e.mem.u32(world + WORLD_CELL);
        if cell == 0 || e.mem.u32(cell + CELL_OBJECTS) == 0 {
            continue;
        }
        init_pos_mods(e, &mut dims, world);
        let root = root_map_world(e, world);
        if root_world != root {
            root_world = root;
            init_dimensions(e, &mut dims, root);
            out.entry(root).or_default();
        }
        let mut refs = cell + CELL_OBJECTS;
        while refs != 0 {
            let refr = e.mem.u32(refs);
            refs = e.mem.u32(refs + 4);
            if refr == 0 {
                continue;
            }
            let data = map_marker_data(e, refr);
            if data == 0 {
                continue;
            }
            let pos = [
                e.mem.f32(refr + REF_POSITION),
                e.mem.f32(refr + REF_POSITION + 4),
            ];
            let mut coord = dims.coord;
            let place = world_map_pos_mults(pos, &dims, Some(&mut coord));
            dims.coord = coord;
            out.entry(root_world)
                .or_default()
                .entry(coord)
                .or_default()
                .push(MarkerInfo {
                    refr,
                    data,
                    pos: place,
                });
        }
    }
    out
}

#[cfg(test)]
#[path = "jip_fn_minimap_tests.rs"]
mod tests;
