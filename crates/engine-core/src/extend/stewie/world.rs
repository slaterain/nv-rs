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

// ---- Group L: the interface manager's `Idle` (the rows beside JIP LN NVSE's G58) ----

/// S0022.02 (`IsShiftHeld` at `0x70E4DB` and `0x70E500` in `InterfaceManager::Idle` `0070c4a0`): a
/// right or left arrow key becomes shift-right or shift-left by the keyboard's shift state now
/// (xNVSE's raw key state), not the interface's remembered one.
pub const MENU_SHIFT_HELD_NOW: Row = fix(17);
/// S0022.10 (`BookMenuClose_DebounceMenumodeKey` at `0x73B4C4` in `BookMenu::Close` `0073b480`;
/// `IsControllerConnectedAndNotMenumodeOverride` at `0x70E854` in `Idle`): closing a book with the
/// menu-mode control held sets the input's menu-mode debounce, and while it is set the open Pip-Boy
/// reads that control the keyboard's way, so the same press does not close the Pip-Boy too.
pub const BOOK_CLOSE_DEBOUNCE: Row = fix(18);
/// S0022.18 (`PipboyHotkeysCheckConsoleOpenOrKeyboardInputHandled`, its jump at `0x70E9B8` in
/// `Idle`): the Pip-Boy hot keys (F1 to F3) do nothing while the frontmost menu took the call's key
/// or the console is open.
pub const PIPBOY_HOTKEYS_AFTER_TEXT: Row = fix(19);
/// S0022.39 (bytes at `0x70CE6A`, `0x70CDC5`, `0x70CDD3`, `0x70CDE2`, `0x70CDF1` in `Idle`): with a
/// pad, the activate button clicks like the mouse's left button.
pub const PAD_ACTIVATE_CLICKS: Row = fix(20);
/// S0022.40 (bytes at `0x70CE2C`; calls at `0x70CE44`, `0x70CE53`, `0x70CE62` in `Idle`): the
/// console's pad activate reads the pad's RB alone, so keyboard keys (E) typed in the console with a
/// pad connected no longer change the selected reference.
pub const CONSOLE_PAD_ACTIVATE_RB: Row = fix(21);
/// S0022.45 (`ConsoleManager_HandleControllerHotkeys` at `0x70E063` in `Idle`): the pad's D-pad up
/// and down step through the console's command history, repeating while held.
pub const CONSOLE_DPAD_HISTORY: Row = fix(22);
/// S0062 `AllowNumbersInXMLButtons` (`0x70E1A3`, `0x70E25B`, `0x70E2CA`, `0x70E32D`, `0x70E360` in
/// `Idle`): a menu's keyboard buttons (`_PCButton_` traits) answer the number keys, Caps Lock, Tab,
/// Alt and F1 to F12 by scan code, also while held, and Tab clicking one debounces menu mode.
pub const XML_BUTTONS_BY_SCANCODE: Row = fix(23);
/// S0065 (`IsInPauseBarterContainerOrQuantityMenu` at `0x70D6DA` in `Idle`): a rendered menu (a
/// terminal) behind the barter, container or quantity menu no longer takes the mouse.
pub const RENDERED_MENU_BEHIND_TRADE: Row = fix(24);
/// S0984 (`ToggleConsoleHook`, `DetourRelCall(0x70E5FE)` in Tweaks' deferred init): B on the pad
/// closes the open console. Tweaks' hook asks what was at the call before it (JIP LN NVSE's G58
/// when JIP's fix row is on, else the exe's key query) and adds its B test.
pub const CONSOLE_PAD_B_CLOSES: Row = fix(25);

// ---- Group M: input settings, messages and small engine fixes ----

/// S0016 `patchAllowKeyboardPauseAndScreenshotWhileControllerConnected` (the JNZ at `0xA2479B` in
/// `OSInputGlobals::GetControlState` `00a24660`): the fixed keys (pause/Escape, the console key,
/// Print Screen) are read with a pad connected too.
pub const FIXED_KEYS_WITH_PAD: Row = fix(26);
/// S0022.54 (`MenuFixes`, its JMP at `0xA2507C` in `Controls::SetControl` `00a24fc0`): backslash
/// can be bound to a control.
pub const BIND_BACKSLASH: Row = fix(27);
/// S0128 `PreventNoUnequipMessageForNPCs` (`OnRemoveWornItemMessageHook` at `0x4C5640` in
/// `004c37d0`): the "can't remove worn item" message only for the player, not when an NPC uses up
/// its last force-equipped grenade.
pub const NO_UNEQUIP_MESSAGE_PLAYER_ONLY: Row = fix(28);
/// S0131 `patchFixMapMarkerReputationLookup` (`OnGetMapMarkerReputationIDHook` at `0x416E17` in
/// `ExtraDataList::InitItem` `00416be0`): a map marker's stored reputation id is resolved through
/// the plugin's own load-order index (`AddCompileIndex`) before the reputation is looked up.
pub const MAP_MARKER_REPUTATION_INDEX: Row = fix(29);
/// S0103 `patchFixFullLODResettingWhenOpeningPipboy` (`SetTreeFullLODToINISetting` at `0x54F413`
/// in `TakeExteriorLocalMapPicture` `0054ee80`): after the local map's picture the trees' full-LOD
/// flag goes back to the `bForceFullLOD` setting, not to 0. Tweaks reads the tree manager without
/// testing it: with the missing-data guard row a missing one is passed over.
pub const TREE_FULL_LOD_KEPT: Row = fix(30);
/// S0080 `patchTestAllCellsNeedlessINIChecks` (`SafeNopCall(0x4A8BE6)` in `004a8bb0`): the scene
/// report of the cells around the player no longer re-reads the budget settings (`004a8fe0`) each
/// time, which made the `TestAllCells` console command slow.
pub const TEST_ALL_CELLS_NO_INI_REREAD: Row = fix(31);

/// `OSInputGlobals *` (xNVSE's `0x11F35CC`).
pub const INPUT_GLOBALS: u32 = 0x011f_35cc;
/// `OSInputGlobals::overrideFlags` (`UInt16` at `+0x1B92`); bit 3 is the menu-mode debounce.
const INPUT_OVERRIDE_FLAGS: u32 = 0x1b92;

/// The game's copies of the pad's state, this frame's and the last (xNVSE's `XINPUT_GAMEPAD_EX`:
/// `GetCurrent` `0x11F35A8`, `GetPrevious` `0x11F35B8`; an event count, then `wButtons` at `+4`,
/// the triggers, and the sticks `+8`..`+0xE`).
const PAD_CURRENT: u32 = 0x011f_35a8;
const PAD_PREVIOUS: u32 = 0x011f_35b8;
/// `XINPUT_GAMEPAD_EX::wButtons`.
const PAD_BUTTONS: u32 = 4;
/// `XINPUT_GAMEPAD` button bits (Tweaks' `XboxButtonMask`).
pub const PAD_DPAD_UP: u16 = 0x1;
pub const PAD_DPAD_DOWN: u16 = 0x2;
pub const PAD_RB: u16 = 0x200;
pub const PAD_A: u16 = 0x1000;
pub const PAD_B: u16 = 0x2000;

fn pad_buttons(e: &mut Engine, pad: u32) -> u16 {
    e.mem.u16(pad + PAD_BUTTONS)
}

/// Tweaks' `GetXIControlPressed`: the button down this frame and not the last.
pub fn pad_pressed(e: &mut Engine, mask: u16) -> bool {
    pad_buttons(e, PAD_CURRENT) & mask != 0 && pad_buttons(e, PAD_PREVIOUS) & mask == 0
}

/// Tweaks' `GetXIControlDepressed`: the button down the last frame and not this one.
pub fn pad_depressed(e: &mut Engine, mask: u16) -> bool {
    pad_buttons(e, PAD_PREVIOUS) & mask != 0 && pad_buttons(e, PAD_CURRENT) & mask == 0
}

/// Tweaks' `GetXIControlHeld`: the button down this frame.
pub fn pad_held(e: &mut Engine, mask: u16) -> bool {
    pad_buttons(e, PAD_CURRENT) & mask != 0
}

/// Whether any pad button is down this frame (Tweaks' `GetCurrent()->wButtons`).
pub fn pad_any_button(e: &mut Engine) -> bool {
    pad_buttons(e, PAD_CURRENT) != 0
}

/// `ConsoleManager::GetSingleton` (`0071b160`, cdecl, here with 1: made when missing).
const CONSOLE_INSTANCE: u32 = 0x0071_b160;
/// `ConsoleManager::HandleInput` (`0071b210`), `Scroll` (`0071d380`), `RefreshOutput` (`0071d410`).
const CONSOLE_HANDLE_INPUT: u32 = 0x0071_b210;
const CONSOLE_SCROLL: u32 = 0x0071_d380;
const CONSOLE_REFRESH_OUTPUT: u32 = 0x0071_d410;
/// The game clock's milliseconds (`timeGlobals->msPassed`, `0x11F6394 + 0x14`).
const MS_PASSED: u32 = 0x011f_6394 + 0x14;
/// The settings Tweaks reads the repeat times from (`Setting` values at `+4`): `fKeyRepeatInterval`
/// (`0x11D8B88`) and `fKeyRepeatTime` (`0x11D8B38`).
const KEY_REPEAT_INTERVAL: u32 = 0x011d_8b88 + 4;
const KEY_REPEAT_TIME: u32 = 0x011d_8b38 + 4;

/// S0022.45, Tweaks' `ConsoleManager_HandleControllerHotkeys` (called at `0x70E063` in place of
/// `MenuConsole::Instance(0)`; it answers the console, made when missing). With the console open:
/// while a pad button is down, D-pad up or down (the first held of the two) hands the console the
/// arrow-up or arrow-down key on its press and, held, again each `fKeyRepeatInterval` after
/// `fKeyRepeatTime`; with neither, A pressed hands it Enter. The right stick past 4,000 scrolls the
/// output one line (up when pushed up) every 50, 20 or 10 ms by how far it is pushed. Tweaks'
/// statics are shared by the game's threads here.
pub fn console_pad_hotkeys(e: &mut Engine) -> u32 {
    let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
    if console == 0 || e.mem.u8(console.wrapping_add(0x38)) == 0 {
        return console;
    }
    let bits = std::sync::Arc::clone(&e.extend.stewie);
    let prints = &bits.3;
    if pad_any_button(e) {
        let held = if pad_held(e, PAD_DPAD_UP) {
            1
        } else if pad_held(e, PAD_DPAD_DOWN) {
            2
        } else {
            0
        };
        if held != 0 {
            let mask = if held == 1 {
                PAD_DPAD_UP
            } else {
                PAD_DPAD_DOWN
            };
            let now = e.mem.u32(MS_PASSED);
            let mut press = pad_pressed(e, mask);
            if !press {
                if prints.console_pad_last_held.load(Relaxed) == held {
                    if now >= prints.console_pad_next_repeat.load(Relaxed) {
                        let interval = e.mem.f32(KEY_REPEAT_INTERVAL);
                        // Tweaks' `currentTime + fKeyRepeatInterval` into its `UInt32`: a float sum, truncated.
                        let next = (now as f32 + interval) as u32;
                        prints.console_pad_next_repeat.store(next, Relaxed);
                        press = true;
                    }
                } else {
                    prints.console_pad_last_held.store(held, Relaxed);
                    let delay = e.mem.f32(KEY_REPEAT_TIME);
                    let next = (now as f32 + delay) as u32;
                    prints.console_pad_next_repeat.store(next, Relaxed);
                }
            }
            if press {
                let code = if held == 1 {
                    0x8000_0003u32
                } else {
                    0x8000_0004
                };
                e.call(CONSOLE_HANDLE_INPUT, &args![console, code]);
            }
        } else {
            if pad_pressed(e, PAD_A) {
                e.call(CONSOLE_HANDLE_INPUT, &args![console, 0x8000_0008u32]);
            }
            prints.console_pad_last_held.store(0, Relaxed);
        }
    } else {
        prints.console_pad_last_held.store(0, Relaxed);
    }
    let stick = e.mem.u16(PAD_CURRENT + 0xe) as i16;
    let size = i32::from(stick).abs();
    if size > 4000 {
        let now = e.mem.u32(MS_PASSED);
        if now >= prints.console_stick_next_repeat.load(Relaxed) {
            let step = if size < 12000 {
                50
            } else if size < 24000 {
                20
            } else {
                10
            };
            prints
                .console_stick_next_repeat
                .store(now.wrapping_add(step), Relaxed);
            let amount = if stick > 0 { -1i32 } else { 1 };
            e.call(CONSOLE_SCROLL, &args![console, amount as u32]);
            e.call(CONSOLE_REFRESH_OUTPUT, &args![console]);
        }
    } else {
        prints.console_stick_next_repeat.store(0, Relaxed);
    }
    console
}

/// xNVSE's `OSInputGlobals::IsDebounceMenuMode`: bit 3 of the input's override flags.
pub fn debounce_menu_mode(e: &mut Engine) -> bool {
    let input = e.global::<u32>(INPUT_GLOBALS);
    e.mem.u16(input.wrapping_add(INPUT_OVERRIDE_FLAGS)) & 8 != 0
}

/// xNVSE's `OSInputGlobals::SetDebounceMenuMode(true)`: sets bit 3 of the input's override flags.
pub fn set_debounce_menu_mode(e: &mut Engine) {
    let input = e.global::<u32>(INPUT_GLOBALS);
    let at = input.wrapping_add(INPUT_OVERRIDE_FLAGS);
    let flags = e.mem.u16(at);
    e.mem.set_u16(at, flags | 8);
}

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
