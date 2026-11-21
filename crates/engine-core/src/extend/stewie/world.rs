//! Share 1a's rows (the engines and the console: Havok, Gamebryo, the environment, magic, pathing,
//! face-gen, the engine-bs files no other share holds, the interface manager, the console and the
//! INI settings). Only share 1a adds rows here; the other shares add theirs in `super::forms`,
//! `super::ai` and `super::menus`, so their PRs never touch the same lines. A row is
//! `Row::share(WORLD_FIXES, WORLD_FIXES_END, n)` (or the tweaks' pair), which does not compile past
//! the share's last bit.

use super::Row;
use crate::{args, Engine};
use std::sync::atomic::Ordering::Relaxed;

/// Share 1a's fix rows: bits `128..214` (row S1016's `CHANGES_MAP_BUCKETS`, bit 128, is its first).
pub const WORLD_FIXES: u16 = super::FIXES;
/// One past share 1a's last fix bit.
pub const WORLD_FIXES_END: u16 = 214;
/// Share 1a's tweak rows: bits `640..704`.
pub const WORLD_TWEAKS: u16 = super::VNV_TWEAKS;
/// One past share 1a's last tweak bit.
pub const WORLD_TWEAKS_END: u16 = 704;

const fn fix(n: u16) -> Row {
    Row::share(WORLD_FIXES, WORLD_FIXES_END, n)
}

// ---- Group J: the environment, pathing, Gamebryo and magic decals ----

/// S0050 `patchAddMissingLODWarning` (`BGSTerrainManager::HideTree` `006fcfa0`, its entry): a tree
/// reference handed to no terrain manager (`this` 0: the reference has distant LOD but a plugin
/// disabled it) does nothing, where the exe reads through the missing manager; Tweaks'
/// `NullLODDataPrintID` prints [`lod_warning`]'s line.
pub const LOD_DATA_MISSING_WARNING: Row = fix(1);
/// S0235 `NavMeshInfoResolveFix` (`PathingLocation::ResolveNavMeshInfo` `006dd6f0`, `0x6DDB91`): when
/// no candidate was taken and none is without bounds (the index `-1`, its start value), the resolve
/// answers false, where the exe reads the candidate list at `-1`.
pub const NAV_MESH_INFO_RESOLVE: Row = fix(2);
/// S0242 `NavMeshObstacleDoorTypeFix` (`0x6C0DFE` in `006c0dc0`, `0x6C0F4E` in
/// `NavMeshObstacleManager::OnDoorClose` `006c0f10`, `0x6C107F` in `006c1060`): a reference whose
/// base form is not a door (form type `0x1C`) leaves the three door handlers at once, where the exe
/// only skips the obstacle and still puts the reference in (or takes it from) the door queues.
pub const NAV_MESH_OBSTACLE_DOORS_ONLY: Row = fix(3);
/// S0193 `NavMeshTeleportDoorFix` (`NavMeshInfoSearch::GetNodeConnections` `006b8490`,
/// `0x6B8AF2`): a door whose destination reference has no teleport of its own is passed over (the
/// loop's next door, `0x6B8890`), where the exe reads the missing teleport's point.
pub const NAV_MESH_TELEPORT_DOOR: Row = fix(4);
/// S0238 `SkinnedDecalSubtextureFix` (Tweaks' `[INI] bSkinnedDecalSubtextureFix`): the skinned
/// decal's constructor (`00683900`, `0x683A3D`) keeps the creation data's subtexture index (its
/// byte `+0x70`) in the decal's byte `+0x29`, and the decal geometry's build (`00687d90`,
/// `0x687E9B`) hands it to the new shader property (its byte `+0xC0`); the exe keeps neither, so
/// every skinned decal shows its texture's first subtexture.
pub const SKINNED_DECAL_SUBTEXTURE: Row = fix(5);
/// S0237 `SwitchNodeBoundsFix` (`NiSwitchNode::UpdateWorldBound` `00a93f90`, its entry;
/// `UpdateDownwardPass` `0xA940DE`, `UpdateSelectedDownwardPass` `0xA941D8`,
/// `UpdateRigidDownwardPass` `0xA942B7`): a switch node (or LOD node) that updates only its active
/// child makes its world bound when it has none and takes the child's only when the child has one
/// with a radius, as Bethesda's other nodes do since the bound became a pointer; the exe writes
/// through a missing bound.
pub const SWITCH_NODE_BOUNDS: Row = fix(6);
/// S0145 `patchFixSunDirection` (`Sun::Update` `00641830`, `0x642298`: `FLD [ECX+4]; FCHS` in place
/// of the call reading `fSunYExtreme`, `011ccf0c`): the north-south part of the sun light's way is
/// the setting negated, so the light comes from the side the disc is on.
pub const SUN_LIGHT_DIRECTION: Row = fix(7);

/// `TESForm::GetFile` (`00484e60`, `thiscall`): the form's file at an index, `-1` the last one.
const FORM_GET_FILE: u32 = 0x0048_4e60;
/// `ModInfo::name` (`char[0x104]` at `+0x20`, xNVSE's `GameData.h`).
const FILE_NAME: u32 = 0x20;

/// S0050, Tweaks' `NullLODDataPrintID(terrainMgr, ref, appendToArray)`: for a reference other than
/// the last one it saw (its function's `static TESObjectREFR* lastRef`, remembered), "Warning:
/// reference %08X has LOD but is disabled by plugin %s" with the reference's form id (`+0xC`) and
/// the name of its last file (`GetFile(-1)`, an empty name without one), formatted into Tweaks'
/// 0x100-byte buffer (at most 255 bytes), through [`super::console_queue_or_print`]. As with Tweaks
/// only an immediate repeat is skipped.
pub fn lod_warning(e: &mut Engine, reference: u32) {
    let bits = std::sync::Arc::clone(&e.extend.stewie);
    if bits.3.lod_ref.swap(reference, Relaxed) == reference {
        return;
    }
    let file = e.call(FORM_GET_FILE, &args![reference, -1i32 as u32]).u32();
    let mut line = format!(
        "Warning: reference {:08X} has LOD but is disabled by plugin ",
        e.mem.u32(reference.wrapping_add(0xc))
    )
    .into_bytes();
    if file != 0 {
        let name = file.wrapping_add(FILE_NAME);
        for i in 0..0x104 {
            let byte = e.mem.u8(name.wrapping_add(i));
            if byte == 0 {
                break;
            }
            line.push(byte);
        }
    }
    line.truncate(0xff);
    super::console_queue_or_print(e, &line);
}
