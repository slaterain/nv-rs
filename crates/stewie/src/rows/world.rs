//! Share 1a's rows (the engines and the console; their bits in `engine_core::extend::stewie`'s
//! `world.rs`), in PLAN.md's order. Only share 1a adds rows here; the other shares add theirs in
//! `super::forms`, `super::ai` and `super::menus`.

use super::Row;
use engine_core::extend::stewie;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::Engine;

/// S0022.03's seam: the text key event `007154b0`'s shift state at `0x715563` (Tweaks' `IsShiftHeld`).
pub const TEXT_SHIFT_SITE: SiteSlot = SiteSlot::new(Owner::Stewie, 0x0071_5563);

/// S0022.02's seams: `Idle`'s shift tests of the right and left arrow keys at `0x70E4DB`, `0x70E500`.
pub const ARROW_SHIFT_SITES: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Stewie, 0x0070_e4db),
    SiteSlot::new(Owner::Stewie, 0x0070_e500),
];

/// Share 1a's seam handlers, put in at Tweaks' install (before the game's threads start).
pub(crate) fn install(e: &mut Engine) {
    e.on_site(TEXT_SHIFT_SITE, shift_held);
    for slot in ARROW_SHIFT_SITES {
        e.on_site(slot, shift_held);
    }
}

/// Tweaks' `IsShiftHeld` (its copy of xNVSE's `DirectInputFuncs.cpp`): xNVSE's key record's raw
/// state (`rawState`, the record's first byte: `[g_DIHook + 0x2A*7 + 4]`, `+ 0x36*7 + 4`) of the left
/// or the right shift key, 1 or 0.
fn shift_held(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let raw = |e: &mut Engine, key: u32| nvse::input::key_states(e, key) & 2 != 0;
    Some(u32::from(raw(e, 0x2a) || raw(e, 0x36)))
}

/// Share 1a's rows built.
pub const ROWS: &[Row] = &[
    Row {
        bit: stewie::INVENTORY_SCROLL_KEPT,
        function: "patchSaveInventoryScrollPositionWhenClosingPipBoy",
        plan: "S0021",
        key: "bSaveInventoryScrollPositionWhenClosingPipBoy",
        tweaks: None,
        summary: "the inventory list keeps its scroll place when the Pip-Boy is closed and opened again",
    },
    Row {
        bit: stewie::MENU_SHIFT_HELD_NOW,
        function: "MenuFixes",
        plan: "S0022.02",
        key: "bMenuFixesMenuShiftHeldNow",
        tweaks: None,
        summary: "shift with the left or right arrow in menus is read as the keyboard has it now",
    },
    Row {
        bit: stewie::TEXT_SHIFT_HELD_NOW,
        function: "MenuFixes",
        plan: "S0022.03",
        key: "bMenuFixesTextShiftHeldNow",
        tweaks: None,
        summary: "typed text uses shift as the keyboard has it now, so a slow frame no longer leaves capitals stuck on or off",
    },
    Row {
        bit: stewie::ARROW_KEYS_OVER_MOUSE,
        function: "MenuFixes",
        plan: "S0022.05",
        key: "bMenuFixesArrowKeysOverMouse",
        tweaks: None,
        summary: "after an arrow key moves a menu's selection, the mouse no longer snaps it back to the item under the pointer",
    },
    Row {
        bit: stewie::BOOK_CLOSE_DEBOUNCE,
        function: "MenuFixes",
        plan: "S0022.10",
        key: "bMenuFixesBookCloseDebounce",
        tweaks: None,
        summary: "closing a book or note with the menu key no longer closes the Pip-Boy behind it too",
    },
    Row {
        bit: stewie::PIPBOY_HOTKEYS_AFTER_TEXT,
        function: "MenuFixes",
        plan: "S0022.18",
        key: "bMenuFixesPipboyHotkeysAfterText",
        tweaks: None,
        summary: "F1 to F3 no longer open Pip-Boy pages while typing in the console or when a menu took the key",
    },
    Row {
        bit: stewie::GAMEPAD_REPEAT_CLEARED,
        function: "MenuFixes",
        plan: "S0022.32",
        key: "bMenuFixesGamepadRepeatCleared",
        tweaks: None,
        summary: "with no pad button held the pad's key repeat is cleared, so the next press does not jump part-way into an old repeat",
    },
    Row {
        bit: stewie::CONSOLE_THUMBSTICK_CURSOR,
        function: "MenuFixes",
        plan: "S0022.38",
        key: "bMenuFixesThumbstickCursor",
        tweaks: None,
        summary: "with a pad and the pointer shown (the console), the left thumbstick moves the pointer, the mouse added to it",
    },
    Row {
        bit: stewie::PAD_ACTIVATE_CLICKS,
        function: "MenuFixes",
        plan: "S0022.39",
        key: "bMenuFixesPadActivateClicks",
        tweaks: None,
        summary: "with a pad, its activate button clicks in menus like the mouse's left button, the mouse working alongside",
    },
    Row {
        bit: stewie::CONSOLE_PAD_ACTIVATE_RB,
        function: "MenuFixes",
        plan: "S0022.40",
        key: "bMenuFixesConsolePadActivateRB",
        tweaks: None,
        summary: "typing E in the console with a pad connected no longer changes the selected reference (the pad's RB does)",
    },
    Row {
        bit: stewie::CONSOLE_DPAD_HISTORY,
        function: "MenuFixes",
        plan: "S0022.45",
        key: "bMenuFixesConsoleDpad",
        tweaks: None,
        summary: "the pad works the console: D-pad up and down for the command history, A for Enter, the right stick scrolls",
    },
    Row {
        bit: stewie::MENU_L3_R3,
        function: "MenuFixes",
        plan: "S0022.53",
        key: "bMenuFixesMenuL3R3",
        tweaks: None,
        summary: "menus are told about L3 and R3 presses on the pad (except while the controls menu waits for a key)",
    },
    Row {
        bit: stewie::PIPBOY_LIGHT_FADE,
        function: "patchFixPipboyLightStayingOnWhenViewing",
        plan: "S0046",
        key: "bFixPipboyLightStayingOnWhenViewing",
        tweaks: None,
        summary: "the Pip-Boy screen's light no longer stays lit when the Pip-Boy is raised with the light still showing",
    },
    Row {
        bit: stewie::LOD_DATA_MISSING_WARNING,
        function: "patchAddMissingLODWarning",
        plan: "S0050",
        key: "bAddMissingLODWarning",
        tweaks: None,
        summary: "a reference with distant LOD that a plugin disabled no longer crashes the distant trees; Tweaks' console line names it and its plugin",
    },
    Row {
        bit: stewie::CONSOLE_MESH_PATH,
        function: "patchShowSelectedMeshPathInConsoleIfMissing",
        plan: "S0059",
        key: "bShowSelectedMeshPathInConsoleIfMissing",
        tweaks: None,
        summary: "the console shows the selected reference's model path (always with VNV's bMeshPath, else when the model is missing)",
    },
    Row {
        bit: stewie::XML_BUTTONS_BY_SCANCODE,
        function: "AllowNumbersInXMLButtons",
        plan: "S0062",
        key: "bAllowNumbersInXMLButtons",
        tweaks: None,
        summary: "menus' keyboard buttons answer the number keys, Caps Lock, Tab, Alt and F1 to F12, also while held",
    },
    Row {
        bit: stewie::RENDERED_MENU_BEHIND_TRADE,
        function: "fixMouseInBarterAndContainerMenusInsideTerminals",
        plan: "S0065",
        key: "bFixMouseInBarterAndContainerMenusInsideTerminals",
        tweaks: None,
        summary: "a terminal behind the barter, container or quantity menu no longer takes the mouse",
    },
    Row {
        bit: stewie::SUN_LIGHT_DIRECTION,
        function: "patchFixSunDirection",
        plan: "S0145",
        key: "bFixSunDirection",
        tweaks: None,
        summary: "changes the look: the sun's light comes from the side of the sky its disc is on (north or south), not the mirror side; nothing with the new lighting's light from the sky",
    },
    Row {
        bit: stewie::NAV_MESH_TELEPORT_DOOR,
        function: "NavMeshTeleportDoorFix",
        plan: "S0193",
        key: "bNavMeshTeleportDoorFix",
        tweaks: None,
        summary: "an NPC's path through a door whose other side has no way back no longer crashes: that door is not used",
    },
    Row {
        bit: stewie::STAIR_MATERIAL,
        function: "StairMaterialFix",
        plan: "S0216",
        key: "bStairMaterialFix",
        tweaks: None,
        summary: "changes how stairs walk: collision marked as stairs (123 of the game's own meshes) counts every riser as a step, so characters no longer catch on risers or get pushed back",
    },
    Row {
        bit: stewie::NAV_MESH_INFO_RESOLVE,
        function: "NavMeshInfoResolveFix",
        plan: "S0235",
        key: "bNavMeshInfoResolveFix",
        tweaks: None,
        summary: "a place whose navigation mesh cannot be worked out no longer crashes the pathing: it has none",
    },
    Row {
        bit: stewie::SWITCH_NODE_BOUNDS,
        function: "SwitchNodeBoundsFix",
        plan: "S0237",
        key: "bSwitchNodeBoundsFix",
        tweaks: None,
        summary: "a model with a switch or LOD node set to update only its shown part no longer crashes; its bounds follow the shown part",
    },
    Row {
        bit: stewie::SKINNED_DECAL_SUBTEXTURE,
        function: "SkinnedDecalSubtextureFix",
        plan: "S0238",
        key: "bSkinnedDecalSubtextureFix",
        tweaks: Some(("INI", "bSkinnedDecalSubtextureFix")),
        summary: "changes the look: blood and impact marks on bodies show the variant picked for them, not always the first one",
    },
    Row {
        bit: stewie::NAV_MESH_OBSTACLE_DOORS_ONLY,
        function: "NavMeshObstacleDoorTypeFix",
        plan: "S0242",
        key: "bNavMeshObstacleDoorTypeFix",
        tweaks: None,
        summary: "changes pathing slightly: only doors count as doors opening and closing for NPCs' paths",
    },
    Row {
        bit: stewie::CONSOLE_PAD_B_CLOSES,
        function: "MenuFixesDeferredInit",
        plan: "S0984",
        key: "bMenuFixesDeferredInit",
        tweaks: None,
        summary: "B on the pad closes the open console (with JIP's console key fix too)",
    },
];
