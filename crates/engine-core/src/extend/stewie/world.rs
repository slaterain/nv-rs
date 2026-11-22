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
/// S0216 `StairMaterialFix` (`bhkCharacterController::IsStep` `00c6eb00` at `0xC6EBAB`, `00c6d7e0` at
/// `0xC6D80B`, `SetSupportMaterial` `00c6e980` and `00c84f50` replaced whole): the stair bit `0x40`
/// of a Havok material word (the touched sub-shape's, through Tweaks' `GetSubMaterialVarType`) makes
/// a contact a step, keeps the step's ray from counting the shape as blocking and sets the
/// character's on-stairs flag; the exe masks the bit away or compares `(bit set) == 0x40`.
pub const STAIR_MATERIAL: Row = fix(8);

// ---- Group K: the interface manager's input (Tweaks' `MenuFixes.cpp` and `patches.h`) ----

/// S0021 `patchSaveInventoryScrollPositionWhenClosingPipBoy` (`Menu::StartFadeOut` `00a1d910`'s
/// call at `0xA1D97C`; the Pip-Boy close request `0070f690`'s end `0x70F6D3`): the inventory list's
/// scroll place is saved (`InventoryMenu::SaveCurrentTabScrollPosition` `0077ffd0`) when the
/// inventory menu closes with an item selected and whenever the Pip-Boy is asked to close while the
/// inventory menu is shown.
pub const INVENTORY_SCROLL_KEPT: Row = fix(9);
/// S0022.03 (`MenuFixes::InitHooks`, `IsShiftHeld` at `0x715563` in `007154b0`): a key press's
/// character is looked up with shift as the keyboard has it now (xNVSE's raw key state of either
/// shift key), not the interface manager's `bShiftDown`.
pub const TEXT_SHIFT_HELD_NOW: Row = fix(10);
/// S0022.05 (`InterfaceManager__HandleKeyPressHook` at `0x70F8BB` in `DoGamepadEvent` `0070f6e0`):
/// an arrow key or shift-left/right handed to the frontmost menu clears `bMouseInMotion`, so the
/// mouse no longer takes the selection back.
pub const ARROW_KEYS_OVER_MOUSE: Row = fix(11);
/// S0022.32 (bytes at `0x710624` and `0x71085F` in `GetTopGamepadButton` `00710060`): with no pad
/// button acting, the repeat time is cleared to -1 instead of twice the tick count.
pub const GAMEPAD_REPEAT_CLEARED: Row = fix(12);
/// S0022.38 (bytes at `0x7119CF`, `0x711A2B`, `0x711A37`, `0x711A44`; calls at `0x7119D8`,
/// `0x7119E7` in `007118d0`): with a controller and the cursor shown, the left thumbstick (inside
/// the deadzone `[0x717B90]` nothing) moves the cursor, the mouse's movement added to it.
pub const CONSOLE_THUMBSTICK_CURSOR: Row = fix(13);
/// S0022.53 (`L3PressedHook` at `0x7104A1`, `R3PressedHook` at `0x7104B0` in
/// `GetTopGamepadButton`): L3 and R3 reach the frontmost menu (its `HandleControllerInput`) unless
/// the start menu's control mapping waits for a key.
pub const MENU_L3_R3: Row = fix(14);
/// S0046 `patchFixPipboyLightStayingOnWhenViewing` (`PipboyManager__SetLightEffect` at `0x70EFCD`
/// in `UpdatePipboy` `0070ee80`): raising the Pip-Boy fades its light also when the screen's light
/// effect is still lit though the manager says it is off.
pub const PIPBOY_LIGHT_FADE: Row = fix(15);
/// S0059 `patchShowSelectedMeshPathInConsoleIfMissing` (`OnSelectedRefPrintHook` at `0x710C41` in
/// `DisplayCurrentPickRef` `00710ad0`): the console's selected reference shows its base form's model
/// path on a line of its own, with `[Extra Console Details] bMeshPath` or when its model is missing.
pub const CONSOLE_MESH_PATH: Row = fix(16);

/// The player (`PlayerCharacter *`).
const PLAYER: u32 = 0x011d_ea3c;

/// S0059, Tweaks' `OnSelectedRefPrint`: the model path line the console shows for `reference` (its
/// base form's, `+0x20`, through xNVSE's `GetModelPath`; 0 for none or an empty one), asked only
/// with `[Extra Console Details] bMeshPath` or when the reference's 3D is the missing-model marker
/// ([`missing_model_marker`]); else 0.
pub fn mesh_path_line(e: &mut Engine, reference: u32) -> u32 {
    if !e.stewie_mesh_path() && !missing_model_marker(e, reference) {
        return 0;
    }
    let base = e.mem.u32(reference.wrapping_add(0x20));
    super::model_path(e, base)
}

/// Tweaks' `IsRefMissingMeshMarker`: the reference's node (xNVSE's `GetNiNode`: the render state's,
/// `+0x64` then `+0x14`, or the player's own node `+0x694` while `is3rdPersonVisible` `+0x64A`)
/// is named `marker_error`, compared without case (`_stricmp`).
fn missing_model_marker(e: &mut Engine, reference: u32) -> bool {
    let state = e.mem.u32(reference.wrapping_add(0x64));
    if state == 0 {
        return false;
    }
    let player = e.mem.u32(PLAYER);
    let node = if reference == player && e.mem.u8(player.wrapping_add(0x64a)) != 0 {
        e.mem.u32(player.wrapping_add(0x694))
    } else {
        e.mem.u32(state.wrapping_add(0x14))
    };
    if node == 0 {
        return false;
    }
    let name = e.mem.u32(node.wrapping_add(8));
    if name == 0 {
        return false;
    }
    let marker = b"marker_error\0";
    (0..marker.len() as u32)
        .all(|i| e.mem.u8(name.wrapping_add(i)).to_ascii_lowercase() == marker[i as usize])
}

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
