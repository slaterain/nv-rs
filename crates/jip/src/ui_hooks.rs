//! JIP LN NVSE's UI hooks (step 3.5a; `internal/hooks.h` (JIP LN NVSE)): the menus' open, close,
//! mouseover and click events, the location-discovered and crafting events, and the hooks behind
//! JIP's interface commands (a message or tutorial a script disabled, a reference the player may
//! not activate, the references with map markers, the actors left out of VATS, a terminal's own
//! model), with the twelve commands that set the events.
//!
//! JIP patches a hook's sites into the exe when its first user arrives and puts the bytes back
//! when the last goes (`HookInfo::ModUsers`). Here each site is a seam in its translation that
//! reads the hook's user count first ([`Engine::hook_users`]: one load while it is 0) and then
//! asks the handler registered below. The menus' click handlers are reached through their
//! vtables, as in the exe, so JIP's click hook is what JIP does: its routine written into a
//! menu's vtable slot while the menu has a click event, the exe's function put back with the
//! last (direct calls of a menu's function, which only the shared stub `008d0600` of the HUD and
//! loading menus has, never see it, as in the exe).
//!
//! Nothing here runs while JIP answers "not installed": the handlers, the routine and the
//! commands are put in only when it answers "installed".

use crate::game::{
    ComputersMenu, HackingMenu, InterfaceManager, JipActorFlags, JipFormFlags, Menu, NiTListItem,
    TESObjectREFR, Tile,
};
use crate::utility::{str_begins_cs, str_compare_ci, str_compare_cs, str_to_int, str_to_lower};
use crate::Jip;
use engine_core::extend::{Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::collections::{BTreeMap, BTreeSet};

/// `kMenuType_Min`, `kMenuType_Max`.
pub const MENU_MIN: u32 = 0x3e9;
pub const MENU_MAX: u32 = 0x43c;
/// `kMenuIDJumpTable` (`internal/jip_core.cpp` (JIP LN NVSE)): a menu id's index among the 36
/// menus JIP's events know, -1 for the others.
const MENU_INDEX: [i8; 84] = [
    0, 1, 2, 3, -1, -1, 4, 5, 6, -1, -1, 7, 8, 9, -1, 10, -1, -1, -1, -1, -1, -1, 11, -1, -1, 12,
    13, -1, -1, -1, -1, -1, -1, -1, 14, 15, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 16, -1, -1,
    17, -1, 18, 19, 20, 21, 22, 23, 24, 25, 26, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 27,
    28, 29, 30, -1, -1, 31, 32, 33, 34, 35,
];
/// The 36 menus' vtables, by index (`s_menuClickEventMap`'s order; `internal/class_vtbls.h`
/// (JIP LN NVSE)). A menu's click handler is the vtable's slot `+0xC`.
const MENU_VTABLES: [u32; 36] = [
    0x0107_566c, // MessageMenu
    0x0107_39b4, // InventoryMenu
    0x0106_ffd4, // StatsMenu
    0x0107_2df4, // HUDMainMenu
    0x0107_3ebc, // LoadingMenu
    0x0107_21ac, // ContainerMenu
    0x0107_257c, // DialogMenu
    0x0107_63ac, // SleepWaitMenu
    0x0107_6d1c, // StartMenu
    0x0107_439c, // LockpickMenu
    0x0107_01c4, // QuantityMenu
    0x0107_4d44, // MapMenu
    0x0107_0ecc, // BookMenu
    0x0107_3cdc, // LevelUpMenu
    0x0107_5c5c, // RepairMenu
    0x0107_5974, // RaceSexMenu
    0x0107_1bb4, // CharGenMenu
    0x0107_0034, // TextEditMenu
    0x0107_06ec, // BarterMenu
    0x0107_0084, // SurgeryMenu
    0x0107_28f4, // HackingMenu
    0x0107_00d4, // VATSMenu
    0x0107_2004, // ComputersMenu
    0x0107_5db4, // RepairServicesMenu
    0x0106_ff84, // TutorialMenu
    0x0107_0124, // SpecialBookMenu
    0x0107_3b7c, // ItemModMenu
    0x0107_0174, // LoveTesterMenu
    0x0107_1d0c, // CompanionWheelMenu
    0x0107_7abc, // TraitSelectMenu
    0x0107_048c, // RecipeMenu
    0x0107_64dc, // SlotMachineMenu
    0x0107_08fc, // BlackjackMenu
    0x0107_5f7c, // RouletteMenu
    0x0107_108c, // CaravanMenu
    0x0107_79bc, // TraitMenu
];
const HANDLE_CLICK_SLOT: u32 = 0xc;
/// The menus' names a click path starts with, and their ids (`s_menuNameToID`, filled in
/// `jip_nvse.cpp`'s `InitGameHooks` (JIP LN NVSE)); looked up by `StrHashCI` alone, as JIP's
/// `UnorderedMap<const char*>` keeps only the hash.
const MENU_NAMES: [(&str, u32); 36] = [
    ("MessageMenu", 0x3e9),
    ("InventoryMenu", 0x3ea),
    ("StatsMenu", 0x3eb),
    ("HUDMainMenu", 0x3ec),
    ("LoadingMenu", 0x3ef),
    ("ContainerMenu", 0x3f0),
    ("DialogMenu", 0x3f1),
    ("SleepWaitMenu", 0x3f4),
    ("StartMenu", 0x3f5),
    ("LockpickMenu", 0x3f6),
    ("QuantityMenu", 0x3f8),
    ("MapMenu", 0x3ff),
    ("BookMenu", 0x402),
    ("LevelUpMenu", 0x403),
    ("RepairMenu", 0x40b),
    ("RaceSexMenu", 0x40c),
    ("CharGenMenu", 0x418),
    ("TextEditMenu", 0x41b),
    ("BarterMenu", 0x41d),
    ("SurgeryMenu", 0x41e),
    ("HackingMenu", 0x41f),
    ("VATSMenu", 0x420),
    ("ComputersMenu", 0x421),
    ("RepairServicesMenu", 0x422),
    ("TutorialMenu", 0x423),
    ("SpecialBookMenu", 0x424),
    ("ItemModMenu", 0x425),
    ("LoveTesterMenu", 0x432),
    ("CompanionWheelMenu", 0x433),
    ("TraitSelectMenu", 0x434),
    ("RecipeMenu", 0x435),
    ("SlotMachineMenu", 0x438),
    ("BlackjackMenu", 0x439),
    ("RouletteMenu", 0x43a),
    ("CaravanMenu", 0x43b),
    ("TraitMenu", 0x43c),
];

/// The menus' shown bytes (`011f308f + id`), `g_thePlayer`, `g_interfaceManager`.
const MENU_SHOWN: u32 = 0x011f_308f;
const PLAYER: u32 = 0x011d_ea3c;
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
/// `TileMenu`'s vtable (`IS_TYPE(tile, TileMenu)`; engine-bs `tilemenu.rs`).
const TILE_MENU_VTABLE: u32 = 0x0106_ed44;
/// `kHookFormFlag6_MessageDisabled`, `kHookFormFlag6_NoPCActivation`,
/// `kHookActorFlag2_NonTargetable` (`internal/hooks.h` (JIP LN NVSE)).
const MESSAGE_DISABLED: u8 = 1;
const NO_PC_ACTIVATION: u8 = 2;
const NON_TARGETABLE: u8 = 2;
/// `kFormType_Max`: the form types `s_activationDisabledTypes` has.
const FORM_TYPES: usize = 121;
/// The vtable entry that answers true (`ADDR_ReturnTrue`) and `IsActor`'s slot.
const RETURN_TRUE: u32 = 0x008d_0360;
const VT_IS_ACTOR: u32 = 0x100;
/// `kFormType_Script`.
const SCRIPT: u8 = 0x11;
/// The terminal model's holder (`011db63c`), its path (`GameGlobals::TerminalModelPtr`,
/// `011a0bb0`), `FORenderedTerminal::ReleaseStaticGeometry` (JIP's `PurgeTerminalModel`,
/// `0x7FFE00`), and the computers and hacking menus' singletons.
const TERMINAL_HOLDER: u32 = 0x011d_b63c;
const TERMINAL_MODEL_PATH: u32 = 0x011a_0bb0;
const PURGE_TERMINAL_MODEL: u32 = 0x007f_fe00;
const COMPUTERS_MENU: u32 = 0x011d_9334;
const HACKING_MENU: u32 = 0x011d_95b8;

/// The routine JIP's click hook is (`MenuHandleClickHook`), in JIP's routines past its
/// overrides'.
pub const MENU_CLICK_ROUTINE: u32 = crate::identity::ROUTINES + 0x400;
const _: () = assert!(MENU_CLICK_ROUTINE < engine_core::mem::EXTENDER_CODE.end);

/// The commands' opcodes.
pub const SET_ON_MENU_CLICK: u32 = 0x2798;
pub const SET_ON_MENU_OPEN: u32 = 0x279a;
pub const SET_ON_MENU_CLOSE: u32 = 0x279b;
pub const SET_ON_MOUSEOVER_CHANGE: u32 = 0x27a9;
pub const SET_ON_KEY_DOWN: u32 = 0x27d0;
pub const SET_ON_KEY_UP: u32 = 0x27d1;
pub const SET_ON_CONTROL_DOWN: u32 = 0x27d2;
pub const SET_ON_CONTROL_UP: u32 = 0x27d3;
pub const SET_ON_TRIGGER_DOWN: u32 = 0x27ff;
pub const SET_ON_TRIGGER_UP: u32 = 0x2800;
pub const SET_ON_LOCATION_DISCOVER: u32 = 0x2878;
pub const SET_ON_CRAFTING: u32 = 0x287b;
/// The twelve.
pub const COMMANDS: [u32; 12] = [
    SET_ON_MENU_CLICK,
    SET_ON_MENU_OPEN,
    SET_ON_MENU_CLOSE,
    SET_ON_MOUSEOVER_CHANGE,
    SET_ON_KEY_DOWN,
    SET_ON_KEY_UP,
    SET_ON_CONTROL_DOWN,
    SET_ON_CONTROL_UP,
    SET_ON_TRIGGER_DOWN,
    SET_ON_TRIGGER_UP,
    SET_ON_LOCATION_DISCOVER,
    SET_ON_CRAFTING,
];

/// The seams of this step (engine-core `SITES`, owner JIP).
pub const SITES: [u32; 11] = [
    0x005b_46aa,
    0x0070_d642,
    0x0072_89ff,
    0x0077_5b9b,
    0x0077_955d,
    0x0079_eba1,
    0x007e_88b6,
    0x007f_54b1,
    0x007f_ef0b,
    0x00a1_c573,
    0x00a1_f2f9,
];

/// A menu's open, close and mouseover scripts (`MenuStateCallbacks`), each a set by script
/// address (JIP's `Set<LambdaVarContext>` order).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuStateCallbacks {
    pub on_open: BTreeSet<u32>,
    pub on_close: BTreeSet<u32>,
    pub on_mouseover: BTreeSet<u32>,
}

/// A menu's click events (`MenuClickEvent`): by tile path (kept sorted as JIP's `Map<char*>`,
/// by `StrCompareCI`) and by tile id, and the exe's click handler once JIP's routine replaced it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuClick {
    pub filters: Vec<(Vec<u8>, BTreeSet<u32>)>,
    pub ids: BTreeMap<i32, BTreeSet<u32>>,
    pub original: u32,
}

impl MenuClick {
    fn is_empty(&self) -> bool {
        self.filters.is_empty() && self.ids.is_empty()
    }
}

/// JIP's UI hook state.
#[derive(Clone, Debug)]
pub struct State {
    /// `s_menuStateEventMap`: made for a menu at its first event, never freed.
    pub menu_state: [Option<MenuStateCallbacks>; 36],
    /// `MenuHandleMouseoverHook`'s `lastMenuID`.
    pub last_mouseover_menu: u32,
    /// `s_menuClickEventMap`.
    pub clicks: [MenuClick; 36],
    /// `s_locationDiscoverEventScripts`, `s_itemCraftedEventScripts`.
    pub location_discover: BTreeSet<u32>,
    pub item_crafted: BTreeSet<u32>,
    /// `s_activationDisabledTypes` (set by JIP's activation commands, step 3.6).
    pub activation_disabled_types: [bool; FORM_TYPES],
    /// `s_refMapMarkersList`: the guest `tList` head the map's markers walk (0 until installed);
    /// filled by JIP's map marker commands (step 3.6).
    pub map_markers_list: u32,
    /// `s_terminalAltModelsMap`: a terminal's own model path (a guest string), by its base form
    /// (set by `SetTerminalUIModel`, step 3.6).
    pub terminal_models: BTreeMap<u32, u32>,
    /// `g_terminalModelDefault`: the game's terminal model path, read before JIP's first write.
    pub terminal_model_default: u32,
}

impl Default for State {
    fn default() -> Self {
        State {
            menu_state: std::array::from_fn(|_| None),
            last_mouseover_menu: 0x3ed,
            clicks: std::array::from_fn(|_| MenuClick::default()),
            location_discover: BTreeSet::new(),
            item_crafted: BTreeSet::new(),
            activation_disabled_types: [false; FORM_TYPES],
            map_markers_list: 0,
            terminal_models: BTreeMap::new(),
            terminal_model_default: 0,
        }
    }
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().ui_hooks
}

/// A menu id's index among JIP's 36 menus.
pub fn menu_index(id: u32) -> Option<usize> {
    let i = MENU_INDEX.get(id.wrapping_sub(MENU_MIN) as usize)?;
    usize::try_from(*i).ok()
}

/// Puts JIP's UI hooks in, when it answers "installed": the seams' handlers, the click routine,
/// the map markers' list head, the game's terminal model path, and the twelve commands.
pub fn install(e: &mut Engine, answers_installed: bool) {
    if !answers_installed {
        return;
    }
    e.on_site(SiteSlot::new(Owner::Jip, 0x00a1_f2f9), menu_state_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x00a1_c573), menu_state_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x0070_d642), mouseover_site);
    e.on_site(
        SiteSlot::new(Owner::Jip, 0x0077_955d),
        location_discover_site,
    );
    e.on_site(SiteSlot::new(Owner::Jip, 0x0072_89ff), item_crafted_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x0077_5b9b), rollover_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x0079_eba1), map_markers_site);
    e.on_site(
        SiteSlot::new(Owner::Jip, 0x005b_46aa),
        message_disabled_site,
    );
    e.on_site(
        SiteSlot::new(Owner::Jip, 0x007e_88b6),
        message_disabled_site,
    );
    e.on_site(SiteSlot::new(Owner::Jip, 0x007f_54b1), vats_target_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x007f_ef0b), terminal_model_site);
    e.register(MENU_CLICK_ROUTINE, menu_click_routine);
    let list = nvse::table::alloc(e, 8);
    state(e).map_markers_list = list;
    use crate::table::set_handlers;
    set_handlers(e, SET_ON_MENU_CLICK, Some(set_on_menu_click), None);
    set_handlers(e, SET_ON_MENU_OPEN, Some(set_on_menu_open), None);
    set_handlers(e, SET_ON_MENU_CLOSE, Some(set_on_menu_close), None);
    set_handlers(
        e,
        SET_ON_MOUSEOVER_CHANGE,
        Some(set_on_mouseover_change),
        None,
    );
    set_handlers(e, SET_ON_KEY_DOWN, Some(set_on_key_down), None);
    set_handlers(e, SET_ON_KEY_UP, Some(set_on_key_up), None);
    set_handlers(e, SET_ON_CONTROL_DOWN, Some(set_on_control_down), None);
    set_handlers(e, SET_ON_CONTROL_UP, Some(set_on_control_up), None);
    set_handlers(e, SET_ON_TRIGGER_DOWN, Some(set_on_trigger_down), None);
    set_handlers(e, SET_ON_TRIGGER_UP, Some(set_on_trigger_up), None);
    set_handlers(
        e,
        SET_ON_LOCATION_DISCOVER,
        Some(set_on_location_discover),
        None,
    );
    set_handlers(e, SET_ON_CRAFTING, Some(set_on_crafting), None);
}

fn form_type(e: &Engine, form: u32) -> u8 {
    e.get(
        Ptr::<crate::game::TESForm>::new(form),
        crate::game::TESForm::cFormType,
    )
}

fn form_flags6(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<JipFormFlags>::new(form), JipFormFlags::jipFormFlags6)
}

fn player_in_cell(e: &Engine) -> bool {
    let player = Ptr::<TESObjectREFR>::new(e.mem.u32(PLAYER));
    e.get(player, TESObjectREFR::pParentCell) != 0
}

/// Calls each script of a copy of `scripts` with `args` (`EventCallbackScripts::InvokeEvents`,
/// `InvokeEvents2`), on no reference.
fn invoke(e: &mut Engine, scripts: &BTreeSet<u32>, args: &[u32]) {
    for script in scripts.clone() {
        crate::script_call::call(e, script, 0, args.to_vec());
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: MenuStateOpenHook,
// MenuStateCloseHook.
/// The seams at a menu's shown byte (`0xA1F2F9` with the new value, `0xA1C573` with 0): when the
/// byte already has it, nothing; else it is stored and, for one of JIP's menus with open (or
/// close) scripts while the player is in a cell, each is called with the menu's id. Answers that
/// the byte is stored.
fn menu_state_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (id, shown) = (a[0], a[1] as u8);
    let byte = MENU_SHOWN.wrapping_add(id);
    if e.mem.u8(byte) == shown {
        return Some(0);
    }
    e.mem.set_u8(byte, shown);
    let Some(i) = menu_index(id) else {
        return Some(0);
    };
    let scripts = match &state(e).menu_state[i] {
        Some(c) if shown != 0 => c.on_open.clone(),
        Some(c) => c.on_close.clone(),
        None => return Some(0),
    };
    if !scripts.is_empty() && player_in_cell(e) {
        invoke(e, &scripts, &[id]);
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: MenuHandleMouseoverHook.
/// The seam after the pointer's tile changed (`0x70D642`): the menu under the pointer (the last
/// one when there is none) is remembered, and its mouseover scripts, while the player is in a
/// cell, are called with its id. The exe's code goes on.
fn mouseover_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let manager = Ptr::<InterfaceManager>::new(a[0]);
    let menu = e.get(manager, InterfaceManager::pOverTileMenu);
    if menu != 0 {
        let id = e.get(Ptr::<Menu>::new(menu), Menu::iClass);
        state(e).last_mouseover_menu = id;
    }
    let id = state(e).last_mouseover_menu;
    let scripts = menu_index(id)
        .and_then(|i| state(e).menu_state[i].clone())?
        .on_mouseover;
    if !scripts.is_empty() && player_in_cell(e) {
        invoke(e, &scripts, &[id]);
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: LocationDiscoverHook.
/// The seam after a discovery's stat (`0x77955D`): each location script is called with the map
/// marker. The exe's code goes on.
fn location_discover_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let scripts = state(e).location_discover.clone();
    invoke(e, &scripts, &[a[0]]);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ItemCraftedHook.
/// The seam at each pass of the crafted outputs (`0x7289FF`): each crafting script is called
/// with the chosen form and the count made. The exe's code goes on.
fn item_crafted_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let scripts = state(e).item_crafted.clone();
    invoke(e, &scripts, &[a[0], a[1]]);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetRolloverTextHook.
/// The seam at the HUD's reference (`0x775B9B`): no reference (answer 0) when it, or its base
/// form, has JIP's no-activation flag, or its base form's type is one a script disabled; else
/// the reference stays.
fn rollover_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let reference = a[0];
    if reference == 0 {
        return None;
    }
    if form_flags6(e, reference) & NO_PC_ACTIVATION != 0 {
        return Some(0);
    }
    let base = e.get(
        Ptr::<TESObjectREFR>::new(reference),
        TESObjectREFR::pObjectReference,
    );
    if form_flags6(e, base) & NO_PC_ACTIVATION != 0 {
        return Some(0);
    }
    let kind = usize::from(form_type(e, base));
    if state(e).activation_disabled_types.get(kind) == Some(&true) {
        return Some(0);
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: CreateMapMarkersHook.
/// The seam at the map's companions (`0x79EBA1`): JIP's list of references with map markers.
fn map_markers_site(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(state(e).map_markers_list)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ShowMessageHook,
// ShowTutorialHook.
/// The seams at a message about to be shown (`0x5B46AA`, `0x7E88B6`): a message a script
/// disabled answers true, shown nothing.
fn message_disabled_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    (form_flags6(e, a[0]) & MESSAGE_DISABLED != 0).then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: AddVATSTargetHook.
/// The seam after a target's line of sight held (`0x7F54B1`): a reference that is not an actor
/// stays (1); an actor stays unless a script made it not targetable (0).
fn vats_target_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let reference = a[0];
    let vtable = e.mem.u32(reference);
    if e.mem.u32(vtable + VT_IS_ACTOR) != RETURN_TRUE {
        return Some(1);
    }
    let flags = e.get(
        Ptr::<JipActorFlags>::new(reference),
        JipActorFlags::jipActorFlags2,
    );
    Some(u32::from(flags & NON_TARGETABLE == 0))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetTerminalModelHook,
// GetHackingMenuRef.
/// The seam at the terminal model's holder (`0x7FEF0B`): the held model is released; when the
/// holder still holds one, that is the answer. Else the terminal's reference (the computers
/// menu's, or the hacking menu's: its own once it has guess tiles, else the crosshair's) picks
/// the model path, its base form's own or the game's, and the answer is 0 (load it).
fn terminal_model_site(e: &mut Engine, _: &[u32]) -> Option<u32> {
    e.call(PURGE_TERMINAL_MODEL, &[]);
    let held = e.mem.u32(TERMINAL_HOLDER);
    if held != 0 {
        return Some(held);
    }
    let computers = e.mem.u32(COMPUTERS_MENU);
    let reference = if computers != 0 {
        e.get(
            Ptr::<ComputersMenu>::new(computers),
            ComputersMenu::pTerminalRef,
        )
    } else {
        let hacking = e.mem.u32(HACKING_MENU);
        if hacking == 0 {
            return Some(0);
        }
        let menu = Ptr::<HackingMenu>::new(hacking);
        if e.get(menu, HackingMenu::guessTileCount) != 0 {
            e.get(menu, HackingMenu::pTerminalRef)
        } else {
            let manager = Ptr::<InterfaceManager>::new(e.mem.u32(INTERFACE_MANAGER));
            e.get(manager, InterfaceManager::pActivateRef)
        }
    };
    if reference == 0 {
        return Some(0);
    }
    let base = e.get(
        Ptr::<TESObjectREFR>::new(reference),
        TESObjectREFR::pObjectReference,
    );
    // `g_terminalModelDefault`: JIP reads the game's path once, before any write of its own (the
    // game never writes it); here at the first pick, which comes before JIP's first write.
    if state(e).terminal_model_default == 0 {
        let default = e.mem.u32(TERMINAL_MODEL_PATH);
        state(e).terminal_model_default = default;
    }
    let s = state(e);
    let path = s
        .terminal_models
        .get(&base)
        .copied()
        .unwrap_or(s.terminal_model_default);
    e.mem.set_u32(TERMINAL_MODEL_PATH, path);
    Some(0)
}

/// A guest text up to its 0 (at most `limit` bytes).
fn guest_text(e: &Engine, at: u32, limit: u32) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..limit {
        let b = e.mem.u8(at + i);
        if b == 0 {
            break;
        }
        out.push(b);
    }
    out
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::GetComponentFullName.
/// A tile's path from its menu tile down: each tile's name, with `:n` when `n` earlier siblings
/// (counted back from it in its parent's children) have the same name, joined by `/`.
pub fn component_full_name(e: &Engine, tile: u32) -> Vec<u8> {
    let mut parents = Vec::new();
    let mut t = tile;
    loop {
        parents.push(t);
        if e.mem.u32(t) == TILE_MENU_VTABLE {
            break;
        }
        t = e.get(Ptr::<Tile>::new(t), Tile::pParent);
    }
    let mut out = Vec::new();
    while let Some(t) = parents.pop() {
        let name_at = Tile::xName.off;
        let name_ptr = e.mem.u32(t + name_at);
        let name_len = u32::from(e.mem.u16(t + name_at + 4));
        let name = guest_text_len(e, name_ptr, name_len);
        out.extend_from_slice(&name);
        let parent = e.get(Ptr::<Tile>::new(t), Tile::pParent);
        let children = parent + Tile::xChildren.off;
        let mut node = e.mem.u32(children + 4);
        while e.get(Ptr::<NiTListItem>::new(node), NiTListItem::m_element) != t {
            node = e.get(Ptr::<NiTListItem>::new(node), NiTListItem::m_pkPrev);
        }
        let own = guest_text(e, name_ptr, 0x1000);
        let mut index = 0;
        loop {
            node = e.get(Ptr::<NiTListItem>::new(node), NiTListItem::m_pkPrev);
            if node == 0 {
                break;
            }
            let sibling = e.get(Ptr::<NiTListItem>::new(node), NiTListItem::m_element);
            let sibling_name = guest_text(e, e.mem.u32(sibling + name_at), 0x1000);
            if str_compare_cs(&sibling_name, &own).0 != 0 {
                break;
            }
            index += 1;
        }
        if index != 0 {
            out.push(b':');
            out.extend_from_slice(&crate::utility::int_to_str(index));
        }
        if parents.is_empty() {
            break;
        }
        out.push(b'/');
    }
    out
}

/// `len` bytes of a guest text (JIP's `COPY_BYTES` of a `String`'s data and length).
fn guest_text_len(e: &Engine, at: u32, len: u32) -> Vec<u8> {
    (0..len).map(|i| e.mem.u8(at + i)).collect()
}

/// The filter map's `GetIndex`: the entry with `key` (`Ok`), or where it would go (`Err`), by
/// binary search with `StrCompareCI(key, entry)`.
fn filter_index(filters: &[(Vec<u8>, BTreeSet<u32>)], key: &[u8]) -> Result<usize, usize> {
    let (mut low, mut high) = (0, filters.len());
    while low != high {
        let i = (low + high) >> 1;
        match str_compare_ci(key, &filters[i].0).0 {
            c if c < 0 => high = i,
            c if c > 0 => low = i + 1,
            _ => return Ok(i),
        }
    }
    Err(low)
}

/// Where `FindOpDir(key, false)` starts (the entry with `key`, or the one before where it would
/// go), counting down: `None` when there is none.
fn backward_start(filters: &[(Vec<u8>, BTreeSet<u32>)], key: &[u8]) -> Option<usize> {
    match filter_index(filters, key) {
        Ok(i) => Some(i),
        Err(i) => i.checked_sub(1),
    }
}

/// Where `FindOpDir(key, true)` starts, counting up.
fn forward_start(filters: &[(Vec<u8>, BTreeSet<u32>)], key: &[u8]) -> usize {
    match filter_index(filters, key) {
        Ok(i) | Err(i) => i,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: MenuHandleClickHook,
// MenuClickEvent::SetHook.
/// Puts JIP's routine in menu `i`'s click slot (`set`), or the exe's function back.
fn set_click_hook(e: &mut Engine, i: usize, set: bool) {
    let slot = MENU_VTABLES[i] + HANDLE_CLICK_SLOT;
    if state(e).clicks[i].original == 0 {
        let original = e.mem.u32(slot);
        state(e).clicks[i].original = original;
    }
    let to = if set {
        MENU_CLICK_ROUTINE
    } else {
        state(e).clicks[i].original
    };
    e.mem.set_u32(slot, to);
}

/// JIP's click routine in a menu's vtable (`thiscall`: the menu, the tile id, the clicked tile):
/// the scripts of the filters the clicked tile's path begins with (from the nearest up), each
/// called with the menu's id, the tile id and the tile's name, then those of the tile id (with
/// an empty name when no tile was clicked); then the menu's own click handler.
fn menu_click_routine(e: &mut Engine, a: &[u32]) -> Ret {
    let (menu, tile_id, clicked) = (a[0], a[1], a[2]);
    let id = e.get(Ptr::<Menu>::new(menu), Menu::iClass);
    let i = menu_index(id).expect("JIP's click routine is only in its menus' vtables");
    if clicked != 0 && !state(e).clicks[i].filters.is_empty() {
        let mut path = component_full_name(e, clicked);
        str_to_lower(&mut path);
        let name = e.mem.u32(clicked + Tile::xName.off);
        let mut at = backward_start(&state(e).clicks[i].filters, &path);
        while let Some(k) = at {
            let Some((key, scripts)) = state(e).clicks[i].filters.get(k).cloned() else {
                break;
            };
            if str_begins_cs(&path, &key) == 0 {
                break;
            }
            invoke(e, &scripts, &[id, tile_id, name]);
            at = k.checked_sub(1);
        }
    }
    if !state(e).clicks[i].ids.is_empty() {
        if let Some(scripts) = state(e).clicks[i].ids.get(&(tile_id as i32)).cloned() {
            let name = if clicked != 0 {
                e.mem.u32(clicked + Tile::xName.off)
            } else {
                nvse::table::guest_str(e, b"")
            };
            invoke(e, &scripts, &[id, tile_id, name]);
        }
    }
    let original = state(e).clicks[i].original;
    e.call(original, &[menu, tile_id, clicked])
}

/// The script argument read as a script form, or `None` (the command does nothing).
fn script_arg(e: &Engine, read: &Read) -> Option<u32> {
    let script = read.form(0);
    (read.ok && script != 0 && form_type(e, script) == SCRIPT).then_some(script)
}

fn insert(e: &mut Engine, set: impl Fn(&mut State) -> &mut BTreeSet<u32>, script: u32) -> bool {
    let added = set(state(e)).insert(script);
    if added {
        crate::ln_events::capture(e, script);
    }
    added
}

fn erase(e: &mut Engine, set: impl Fn(&mut State) -> &mut BTreeSet<u32>, script: u32) -> bool {
    let removed = set(state(e)).remove(&script);
    if removed {
        crate::ln_events::release(e, script);
    }
    removed
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_SetOnMenuClickEventHandler_Execute.
/// Adds or removes a script's click event for a tile path (`Menu/tile/...`, its first part a
/// menu's name) or a tile id (`Menu#id`). A path's script replaces those of the deeper paths
/// below it and is not added under a shorter path that has it.
fn set_on_menu_click(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let add = read.int(1, 0) != 0;
    let mut path = read.text(2);
    let slash = path.iter().position(|&b| b == b'/' || b == b'\\');
    let hash = if slash.is_none() {
        path.iter().position(|&b| b == b'#')
    } else {
        None
    };
    let menu_name = &path[..slash.or(hash).unwrap_or(path.len())];
    let hash_value = crate::containers::str_hash_ci(menu_name);
    let Some(&(_, menu_id)) = MENU_NAMES
        .iter()
        .find(|(n, _)| crate::containers::str_hash_ci(n.as_bytes()) == hash_value)
    else {
        return handled(true);
    };
    let i = menu_index(menu_id).expect("every named menu has an index");
    if let Some(h) = hash {
        let tile_id = str_to_int(&path[h + 1..]);
        if tile_id < 0 {
            return handled(true);
        }
        if add {
            if state(e).clicks[i].is_empty() {
                set_click_hook(e, i, true);
            }
            insert(e, |s| s.clicks[i].ids.entry(tile_id).or_default(), script);
        } else if !state(e).clicks[i].ids.is_empty()
            && state(e).clicks[i].ids.contains_key(&tile_id)
        {
            erase(
                e,
                |s| s.clicks[i].ids.get_mut(&tile_id).expect("found"),
                script,
            );
            if state(e).clicks[i].ids[&tile_id].is_empty() {
                state(e).clicks[i].ids.remove(&tile_id);
            }
            if state(e).clicks[i].is_empty() {
                set_click_hook(e, i, false);
            }
        }
        return handled(true);
    }
    for b in path.iter_mut() {
        if *b == b'\\' {
            *b = b'/';
        }
    }
    str_to_lower(&mut path);
    if add {
        let mut matched = false;
        if !state(e).clicks[i].filters.is_empty() {
            let mut at = backward_start(&state(e).clicks[i].filters, &path);
            while let Some(k) = at {
                let (key, scripts) = &state(e).clicks[i].filters[k];
                if str_begins_cs(&path, key) == 0 {
                    break;
                }
                if scripts.contains(&script) {
                    return handled(true);
                }
                at = k.checked_sub(1);
            }
            let mut k = forward_start(&state(e).clicks[i].filters, &path);
            while k < state(e).clicks[i].filters.len() {
                let begins = str_begins_cs(&state(e).clicks[i].filters[k].0, &path);
                if begins == 0 {
                    break;
                }
                if begins == 2 {
                    insert(e, |s| &mut s.clicks[i].filters[k].1, script);
                    matched = true;
                    k += 1;
                } else if erase(e, |s| &mut s.clicks[i].filters[k].1, script)
                    && state(e).clicks[i].filters[k].1.is_empty()
                {
                    state(e).clicks[i].filters.remove(k);
                } else {
                    k += 1;
                }
            }
        }
        if !matched {
            if state(e).clicks[i].is_empty() {
                set_click_hook(e, i, true);
            }
            let k = match filter_index(&state(e).clicks[i].filters, &path) {
                Ok(k) => k,
                Err(k) => {
                    state(e).clicks[i]
                        .filters
                        .insert(k, (path.clone(), BTreeSet::new()));
                    k
                }
            };
            insert(e, |s| &mut s.clicks[i].filters[k].1, script);
        }
    } else if !state(e).clicks[i].filters.is_empty() {
        let mut k = forward_start(&state(e).clicks[i].filters, &path);
        while k < state(e).clicks[i].filters.len() {
            if str_begins_cs(&state(e).clicks[i].filters[k].0, &path) == 0 {
                break;
            }
            erase(e, |s| &mut s.clicks[i].filters[k].1, script);
            if state(e).clicks[i].filters[k].1.is_empty() {
                state(e).clicks[i].filters.remove(k);
            } else {
                k += 1;
            }
        }
        if state(e).clicks[i].is_empty() {
            set_click_hook(e, i, false);
        }
    }
    handled(true)
}

/// `0` open, `1` close, `2` mouseover.
fn menu_state_set(e: &mut Engine, i: usize, kind: u8) -> &mut BTreeSet<u32> {
    let c = state(e).menu_state[i].get_or_insert_with(Default::default);
    match kind {
        2 => &mut c.on_mouseover,
        1 => &mut c.on_close,
        _ => &mut c.on_open,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: SetOnMenuStateEvent.
/// Adds or removes a script's open, close or mouseover event for menu `i`: the mouseover hook
/// gains a user with a menu's first mouseover script, the menu state hook with a menu's first
/// open or close script, and they lose it with the last.
fn set_on_menu_state_event(e: &mut Engine, script: u32, add: bool, i: usize, kind: u8) {
    if state(e).menu_state[i].is_none() {
        if !add {
            return;
        }
        state(e).menu_state[i] = Some(MenuStateCallbacks::default());
    }
    let open_close_empty = |s: &State| {
        let c = s.menu_state[i].as_ref().expect("made above");
        c.on_open.is_empty() && c.on_close.is_empty()
    };
    if add {
        if menu_state_set(e, i, kind).contains(&script) {
            return;
        }
        if kind == 2 {
            if menu_state_set(e, i, kind).is_empty() {
                e.add_hook_user(Hook::MenuHandleMouseover, true);
            }
        } else if open_close_empty(state(e)) {
            e.add_hook_user(Hook::MenuState, true);
        }
        menu_state_set(e, i, kind).insert(script);
        crate::ln_events::capture(e, script);
    } else {
        if !menu_state_set(e, i, kind).remove(&script) {
            return;
        }
        crate::ln_events::release(e, script);
        if kind == 2 {
            if menu_state_set(e, i, kind).is_empty() {
                e.add_hook_user(Hook::MenuHandleMouseover, false);
            }
        } else if open_close_empty(state(e)) {
            e.add_hook_user(Hook::MenuState, false);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// SetOnMenuStateEventHandler_Execute.
/// A script's event for one menu (by id), or for all 36 with menu id 0.
fn set_on_menu_state(e: &mut Engine, w: &[u32], kind: u8) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let add = read.int(1, 0) != 0;
    let menu_id = read.int(2, 0);
    if menu_id == 0 {
        for i in 0..36 {
            set_on_menu_state_event(e, script, add, i, kind);
        }
    } else if (MENU_MIN..=MENU_MAX).contains(&menu_id) {
        if let Some(i) = menu_index(menu_id) {
            set_on_menu_state_event(e, script, add, i, kind);
        }
    }
    handled(true)
}

fn set_on_menu_open(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_menu_state(e, w, 0)
}

fn set_on_menu_close(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_menu_state(e, w, 1)
}

fn set_on_mouseover_change(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_menu_state(e, w, 2)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_input.h:
// SetOnKeyEventHandler_Execute.
/// A script's key or control event (`SetInputEventHandler`, the LN events' maps), for one key
/// or control, or removed from all with -1 (the default).
fn set_on_key_event(e: &mut Engine, w: &[u32], event: u8) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let add = read.int(1, 0) != 0;
    let key = read.int(2, u32::MAX) as i32;
    crate::ln_events::set_input_handler(e, crate::ln_events::mask(event), script, key, add);
    handled(true)
}

fn set_on_key_down(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_key_event(e, w, crate::ln_events::id::KEY_DOWN)
}

fn set_on_key_up(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_key_event(e, w, crate::ln_events::id::KEY_UP)
}

fn set_on_control_down(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_key_event(e, w, crate::ln_events::id::CONTROL_DOWN)
}

fn set_on_control_up(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_key_event(e, w, crate::ln_events::id::CONTROL_UP)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// SetOnTriggerEventHandler_Execute.
/// A script's trigger event (`down`: pressed, else released) for the left (0) or right (1)
/// trigger, only while a controller is ready. The event's flag goes with the event's last script
/// on either trigger.
fn set_on_trigger(e: &mut Engine, w: &[u32], down: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    if !crate::input::controller_ready(e) {
        return handled(true);
    }
    let (add, right) = (read.int(1, 0) != 0, read.int(2, 0));
    let event = usize::from(down);
    let at = (((right & 1) << 1) as usize) + event;
    let mask = crate::ln_events::mask(if down {
        crate::ln_events::id::TRIGGER_DOWN
    } else {
        crate::ln_events::id::TRIGGER_UP
    });
    if add {
        if e.ext::<Jip>().ln_events.triggers[at].insert(script) {
            crate::ln_events::capture(e, script);
            e.ext::<Jip>().ln_events.flags |= mask;
        }
    } else if e.ext::<Jip>().ln_events.triggers[at].remove(&script) {
        crate::ln_events::release(e, script);
        let t = &e.ext::<Jip>().ln_events.triggers;
        if t[event].is_empty() && t[event + 2].is_empty() {
            e.ext::<Jip>().ln_events.flags &= !mask;
        }
    }
    handled(true)
}

fn set_on_trigger_down(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_trigger(e, w, true)
}

fn set_on_trigger_up(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_trigger(e, w, false)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetOnLocationDiscoverEventHandler_Execute, Cmd_SetOnCraftingEventHandler_Execute.
/// Adds or removes a script of a list event; each script is one user of the event's hook.
fn set_on_list_event(
    e: &mut Engine,
    w: &[u32],
    hook: Hook,
    set: fn(&mut State) -> &mut BTreeSet<u32>,
) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    if read.int(1, 0) != 0 {
        if insert(e, set, script) {
            e.add_hook_user(hook, true);
        }
    } else if erase(e, set, script) {
        e.add_hook_user(hook, false);
    }
    handled(true)
}

fn set_on_location_discover(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_list_event(e, w, Hook::LocationDiscover, |s| &mut s.location_discover)
}

fn set_on_crafting(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_list_event(e, w, Hook::ItemCrafted, |s| &mut s.item_crafted)
}

#[cfg(test)]
#[path = "ui_hooks_tests.rs"]
mod tests;
