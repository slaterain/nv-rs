//! JIP LN NVSE's versions of xNVSE's and the game's own commands (`InitCmdPatches`,
//! `internal/patches_cmd.h` (JIP LN NVSE)): at xNVSE's post-load message JIP writes its own
//! handlers into those commands' records. Plan step 3.4, part 1: the ones that need no gamepad
//! state. (`IsKeyPressed` and the five control commands read JIP's XInput state and come with
//! it, in part 2; `IsPluginInstalled` and `GetPluginVersion` are in [`crate::identity`].)
//!
//! Only while JIP answers "installed" ([`crate::ANSWERS_INSTALLED`]): until then every one of
//! these commands is xNVSE's or the game's, as without the DLL, so a mod that does not use JIP
//! gets no JIP behaviour early. The seven at the game's own opcodes (`MenuMode`, `GetItemCount`,
//! `GetContainer`, `IsInList`, `GetHitLocation`, `HasPerk`, `GetNVSEVersionFull` at `0x127D`)
//! change vanilla commands, so they are also a fixes row (ADR-0016 §5): on in fixes, off under
//! vanilla ([`set_vanilla_commands`]).
//!
//! Two of them have their full effect only with JIP's hook sites (steps 3.5 and 3.9):
//! `Update3D` marks the reference for JIP's hook in the model loader's attach, which this does
//! not have yet, and `GetHitLocation` reads the place in the hit data where JIP's
//! `CopyHitDataHook` stores the location. `GetModelPath`'s per-reference paths come with
//! `SetRefrModelPath` (step 3.6); until then a reference answers its base form's path, as in JIP
//! for a reference that has none.

use crate::console::print_result;
use crate::game::{
    Actor, ActorHitData, BGSListForm, ContChangesEntry, ContainerChangesData, ContainerObject,
    ExtraContainerChanges, GameSettingCollection, IniSettingCollection, InterfaceManager, ListNode,
    LoadedRefData, NiAVObject, NiNode, NiTMapItem, Setting, TESContainer, TESForm, TESObjectREFR,
};
use crate::identity;
use crate::table::Params;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::{console_print, is_console_mode, Read};
use nvse::table::Param;
use nvse::{handled, CommandArgs, EvalArgs};
use std::collections::HashMap;

/// The routines JIP's handlers are registered at: after the identity's two, an execute and an
/// eval word for each override.
const ROUTINES: u32 = identity::ROUTINES + 0x100;
const _: () = assert!(ROUTINES + 8 * 32 <= engine_core::mem::EXTENDER_CODE.end);

/// One override: the opcode, JIP's execute and, where JIP replaces it too, its eval.
struct Override {
    opcode: u32,
    execute: AbiFn,
    eval: Option<AbiFn>,
}

/// The ones at xNVSE's opcodes (in every preset, once JIP answers "installed").
const XNVSE: &[Override] = &[
    Override {
        opcode: 0x149a,
        execute: get_numeric_setting,
        eval: None,
    },
    Override {
        opcode: 0x149b,
        execute: set_numeric_setting,
        eval: None,
    },
    Override {
        opcode: 0x149c,
        execute: get_numeric_setting,
        eval: None,
    },
    Override {
        opcode: 0x149d,
        execute: set_numeric_setting,
        eval: None,
    },
    Override {
        opcode: 0x1511,
        execute: is_ref_in_list,
        eval: Some(is_ref_in_list_eval),
    },
    Override {
        opcode: 0x152d,
        execute: update_3d,
        eval: None,
    },
    Override {
        opcode: 0x158e,
        execute: get_model_path,
        eval: None,
    },
];

/// The ones at the game's own opcodes (also the fixes row).
const VANILLA: &[Override] = &[
    Override {
        opcode: 0x1024,
        execute: menu_mode,
        eval: Some(menu_mode_eval),
    },
    Override {
        opcode: 0x102f,
        execute: get_item_count,
        eval: Some(get_item_count_eval),
    },
    Override {
        opcode: 0x10cf,
        execute: get_container,
        eval: None,
    },
    Override {
        opcode: 0x1174,
        execute: is_in_list,
        eval: Some(is_in_list_eval),
    },
    Override {
        opcode: 0x1187,
        execute: get_hit_location,
        eval: None,
    },
    Override {
        opcode: 0x11c1,
        execute: has_perk,
        eval: Some(has_perk_eval),
    },
    Override {
        opcode: 0x127d,
        execute: identity::get_nvse_version_full,
        eval: None,
    },
];

/// `kParams_OneString_OneDouble` (`nvse/ParamInfos.h` (JIP LN NVSE)): what JIP gives
/// `SetNumericGameSetting` and `SetNumericINISetting` (xNVSE's read a `float`).
static ONE_STRING_ONE_DOUBLE: Params = Params::new(
    "kParams_OneString_OneDouble",
    &[
        Param {
            name: "",
            // `kParamType_String`.
            kind: 0x00,
            optional: false,
        },
        Param {
            name: "",
            // `kParamType_Double`.
            kind: 0x2c,
            optional: false,
        },
    ],
);

/// What the overrides keep on the engine.
#[derive(Default)]
pub struct State {
    /// The fixes preset's row: JIP's versions of the game's commands.
    pub vanilla: bool,
    /// The vanilla ones are in the records.
    vanilla_in: bool,
    /// `s_gameSettingsMap`: every game setting and every numeric or string INI setting, by name
    /// without case; made at the first use (JIP makes it at start-up, `0x86AC60`, after which
    /// the collections do not change).
    settings: Option<HashMap<Vec<u8>, u32>>,
}

/// Puts JIP's handlers in the records, when JIP answers "installed" (`InitCmdPatches`, at
/// xNVSE's post-load message): the ones at xNVSE's opcodes, and the vanilla ones with the row.
pub fn install(e: &mut Engine, answers_installed: bool) {
    if !answers_installed {
        return;
    }
    for (i, o) in XNVSE.iter().enumerate() {
        put(e, i as u32, o);
    }
    let params = crate::table::params_address(e, &ONE_STRING_ONE_DOUBLE);
    for opcode in [0x149b, 0x149d] {
        let record = e.script_tables().record(opcode);
        e.mem.set_u32(record + 0x14, params);
    }
    if e.ext::<Jip>().overrides.vanilla {
        put_vanilla(e);
    }
}

fn put_vanilla(e: &mut Engine) {
    if e.ext::<Jip>().overrides.vanilla_in {
        return;
    }
    for (i, o) in VANILLA.iter().enumerate() {
        put(e, (XNVSE.len() + i) as u32, o);
    }
    e.ext::<Jip>().overrides.vanilla_in = true;
}

/// The fixes preset's row: JIP's versions of the game's commands, once JIP answers "installed".
pub fn set_vanilla_commands(e: &mut Engine, on: bool) {
    e.ext::<Jip>().overrides.vanilla = on;
    let jip = e.ext::<Jip>();
    if on && jip.installed && jip.answers_installed {
        put_vanilla(e);
    }
}

/// Writes override `i`'s handlers into its record: the execute word, and the eval word where
/// JIP replaces it.
fn put(e: &mut Engine, i: u32, o: &Override) {
    let at = ROUTINES + 8 * i;
    e.register(at, o.execute);
    let record = e.script_tables().record(o.opcode);
    e.mem.set_u32(record + 0x18, at);
    if let Some(eval) = o.eval {
        e.register(at + 4, eval);
        e.mem.set_u32(record + 0x20, at + 4);
    }
}

// ---- the game's objects ----

/// `g_thePlayer` (`0x011DEA3C`), `g_interfaceManager` (`0x011D8A80`), the menus' visibility
/// (`MENU_VISIBILITY`, bytes by menu id at `0x011F308F`), the Pip-Boy flag JIP reads
/// (`0x011CAB24`) and the console's open byte (`0x011DEA2E`).
const PLAYER: u32 = 0x011d_ea3c;
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const MENU_VISIBILITY: u32 = 0x011f_308f;
const PIPBOY_FLAG: u32 = 0x011c_ab24;
const CONSOLE_OPEN: u32 = 0x011d_ea2e;
/// `kMenuType_*` (`nvse/GameUI.h` (JIP LN NVSE)).
const MENU_MIN: u32 = 0x3e9;
const MENU_INVENTORY: u32 = 0x3ea;
const MENU_STATS: u32 = 0x3eb;
const MENU_HUD_MAIN: u32 = 0x3ec;
const MENU_LOADING: u32 = 0x3ef;
const MENU_START: u32 = 0x3f5;
const MENU_MAP: u32 = 0x3ff;
const MENU_MAX: u32 = 0x43c;
/// The vtable entry that answers true (`ADDR_ReturnTrue`), JIP's tests at `+0xF0`
/// (`IS_REFERENCE`), `+0xF8` (an actor's base form) and `+0x100` (`IS_ACTOR`); the entry that
/// answers `this` (`ADDR_ReturnThis`) at a scene object's `+0xC` (`IsNode`).
const RETURN_TRUE: u32 = 0x008d_0360;
const RETURN_THIS: u32 = 0x0068_15c0;
const VT_IS_REFERENCE: u32 = 0xf0;
const VT_IS_ACTOR_BASE: u32 = 0xf8;
const VT_IS_ACTOR: u32 = 0x100;
const VT_IS_NODE: u32 = 0xc;
/// `BGSPlaceableWater`'s vtable (`kVtbl_BGSPlaceableWater`) and its water form (`+0x4C`);
/// `NiPointLight`'s vtable.
const VTBL_PLACEABLE_WATER: u32 = 0x0102_4f4c;
const PLACEABLE_WATER_FORM: u32 = 0x4c;
const VTBL_POINT_LIGHT: u32 = 0x0109_dd0c;
/// Form types: `TESObjectCONT`, `BGSListForm`.
const CONTAINER: u8 = 0x1b;
const LIST_FORM: u8 = 0x55;
/// A container base's `TESContainer` (`+0x30`); an actor base's (`+0x64`).
const CONT_CONTAINER: u32 = 0x30;
const ACTOR_BASE_CONTAINER: u32 = 0x64;
/// `BaseExtraList::GetByType` (`thiscall`) and the extra data types JIP asks for:
/// `ExtraContainerChanges` (`0x15`), `ExtraLeveledCreature` (`0x2E`: its base at `+0xC`).
const GET_BY_TYPE: u32 = 0x0041_0220;
const EXTRA_CONTAINER_CHANGES: u32 = 0x15;
const EXTRA_LEVELED_CREATURE: u32 = 0x2e;
const LEVELED_CREATURE_BASE: u32 = 0xc;
/// An extra data list's presence bits (`+0x8`): byte `+0xD` bit `0x80` is type `0x2F`
/// (`ExtraLeveledItem`).
const EXTRA_LEVELED_ITEM_BYTE: u32 = 0xd;
/// The game's functions JIP calls: `TESObjectREFR::Set3D`'s base (`0x5702E0`, `thiscall`: the
/// node, whether to unload), `ModelLoader::QueueReference` (`0x444850`, `thiscall`: the
/// reference, a word, whether on the main thread) with the loader (`*0x011C3B3C`), and the
/// player's own update of its 3D (`0x8D3FA0`, `thiscall`).
const SET_3D: u32 = 0x0057_02e0;
const QUEUE_REFERENCE: u32 = 0x0044_4850;
const MODEL_LOADER: u32 = 0x011c_3b3c;
const PLAYER_UPDATE_3D: u32 = 0x008d_3fa0;
/// Virtual functions: `Actor::GetPerkRank` (`+0x4A0`: the perk, whether for teammates) and
/// `BaseProcess::GetHitData` (`+0x770`).
const VT_GET_PERK_RANK: u32 = 0x4a0;
const VT_GET_HIT_DATA: u32 = 0x770;
/// `HasPerk`'s console line, the game's own format (`0x01035F1C`).
const HAS_PERK_FORMAT: u32 = 0x0103_5f1c;
/// The game's setting collections: game settings (`*0x011C8048`), then the INI ones JIP reads,
/// in its order.
const GAME_SETTINGS: u32 = 0x011c_8048;
const INI_COLLECTIONS: [u32; 4] = [0x011f_96a0, 0x011f_35a0, 0x011c_c694, 0x011f_35a4];
/// `kHookRefFlag5F_Update3D` (`internal/hooks.h`).
const REF_FLAG_UPDATE_3D: u8 = 1;
/// The player's id.
const PLAYER_ID: u32 = 0x14;

fn form_id(e: &Engine, form: u32) -> u32 {
    e.get(Ptr::<TESForm>::new(form), TESForm::iFormID)
}

fn form_type(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<TESForm>::new(form), TESForm::cFormType)
}

fn vtable_says(e: &Engine, object: u32, slot: u32) -> bool {
    e.mem.u32(e.mem.u32(object) + slot) == RETURN_TRUE
}

fn base_form(e: &Engine, refr: u32) -> u32 {
    e.get(
        Ptr::<TESObjectREFR>::new(refr),
        TESObjectREFR::pObjectReference,
    )
}

fn extra_list(refr: u32) -> u32 {
    Ptr::<TESObjectREFR>::new(refr)
        .at(TESObjectREFR::ExtraData)
        .addr()
}

fn get_by_type(e: &mut Engine, refr: u32, kind: u32) -> u32 {
    e.call(GET_BY_TYPE, &args![extra_list(refr), kind]).u32()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetBaseForm.
/// A reference's base form: a placeable water's water, a temporary actor's leveled base, else
/// its base form.
fn ref_base_form(e: &mut Engine, refr: u32) -> u32 {
    let base = base_form(e, refr);
    if base == 0 || form_id(e, base) >> 24 != 0xff {
        return base;
    }
    let vtable = e.mem.u32(base);
    if vtable == VTBL_PLACEABLE_WATER {
        return e.mem.u32(base + PLACEABLE_WATER_FORM);
    }
    if e.mem.u32(vtable + VT_IS_ACTOR_BASE) != RETURN_TRUE {
        return base;
    }
    match get_by_type(e, refr, EXTRA_LEVELED_CREATURE) {
        0 => base,
        extra => e.mem.u32(extra + LEVELED_CREATURE_BASE),
    }
}

/// A list's nodes from its inline first one: each node's item, the first one's even when empty
/// (JIP's `do ... while` walks start at the head).
fn list_items(e: &Engine, head: u32) -> Vec<u32> {
    let mut items = Vec::new();
    let mut node = head;
    while node != 0 {
        let n = Ptr::<ListNode>::new(node);
        items.push(e.get(n, ListNode::data));
        node = e.get(n, ListNode::next);
    }
    items
}

fn form_list(list_form: u32) -> u32 {
    Ptr::<BGSListForm>::new(list_form)
        .at(BGSListForm::list)
        .addr()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.h: tList::GetIndexOf.
fn index_in_list(e: &Engine, list_form: u32, item: u32) -> i32 {
    list_items(e, form_list(list_form))
        .iter()
        .position(|&d| d == item)
        .map_or(-1, |i| i as i32)
}

fn print_line(e: &mut Engine, format: &str, words: &[u32]) {
    if is_console_mode(e) {
        console_print(e, format, words);
    }
}

// ---- Update3D (0x152D) ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: HidePointLights
// (internal/hooks.h).
/// Hides the point lights among a node's children, the nodes below them too: every child with
/// the flag bit `0x20000000` set, a point light marked hidden (bit 1), a node walked in turn.
fn hide_point_lights(e: &mut Engine, node: u32) {
    let n = Ptr::<NiNode>::new(node);
    let (children, count) = (
        e.get(n, NiNode::m_pChildren),
        e.get(n, NiNode::m_usChildCount),
    );
    for i in 0..u32::from(count) {
        let child = e.mem.u32(children + 4 * i);
        if child == 0 {
            continue;
        }
        let c = Ptr::<NiAVObject>::new(child);
        let flags = e.get(c, NiAVObject::m_uFlags);
        if flags & 0x2000_0000 == 0 {
            continue;
        }
        let vtable = e.mem.u32(child);
        if vtable == VTBL_POINT_LIGHT {
            e.set(c, NiAVObject::m_uFlags, flags | 1);
        } else if e.mem.u32(vtable + VT_IS_NODE) == RETURN_THIS {
            hide_point_lights(e, child);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::Update3D.
/// A reference with 3D let go of it and queued to load again, marked for JIP's hook in the
/// model loader (`kHookRefFlag5F_Update3D`); the player only when not marked already. Another
/// reference's point lights are hidden first when its root node has the flag bit `0x20000000`.
pub fn ref_update_3d(e: &mut Engine, refr: u32) {
    let r = Ptr::<TESObjectREFR>::new(refr);
    let loaded = e.get(r, TESObjectREFR::pLoadedData);
    if loaded == 0 {
        return;
    }
    let node = e.get(Ptr::<LoadedRefData>::new(loaded), LoadedRefData::p3D);
    if node == 0 {
        return;
    }
    let flags = e.get(r, TESObjectREFR::jipRefFlags5F);
    if form_id(e, refr) == PLAYER_ID {
        if flags & REF_FLAG_UPDATE_3D != 0 {
            return;
        }
    } else {
        let node_flags = e.get(Ptr::<NiAVObject>::new(node), NiAVObject::m_uFlags);
        if node_flags & 0x2000_0000 != 0 {
            hide_point_lights(e, node);
        }
        e.call(SET_3D, &args![refr, 0u32, 1u32]);
    }
    let flags = e.get(r, TESObjectREFR::jipRefFlags5F);
    e.set(r, TESObjectREFR::jipRefFlags5F, flags | REF_FLAG_UPDATE_3D);
    let loader = e.mem.u32(MODEL_LOADER);
    e.call(QUEUE_REFERENCE, &args![loader, refr, 1u32, 0u32]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_Update3D_Execute.
/// The player through the game's own update of its 3D, another reference through
/// [`ref_update_3d`]. The result is left as it was (0: xNVSE zeroes it), where xNVSE's own
/// version answers 1. What a player sees of it comes with JIP's hook in the model loader's attach
/// (steps 3.5 and 3.9), which reads the mark this sets.
fn update_3d(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    // The command needs a reference: the game calls it only with one.
    if a.this_obj != 0 {
        if form_id(e, a.this_obj) == PLAYER_ID {
            e.call(PLAYER_UPDATE_3D, &args![a.this_obj]);
        } else {
            ref_update_3d(e, a.this_obj);
        }
    }
    handled(true)
}

// ---- MenuMode (0x1024) ----

fn menu_visible(e: &Engine, menu: u32) -> bool {
    e.mem.u8(MENU_VISIBILITY + menu) != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameUI.cpp: IsMenuMode.
/// 0: not in game mode; 1: the Pip-Boy up; 2: a menu in the stack; 3: the console open; 4: the
/// loading, start and HUD menus all up; a menu's id: that menu up.
pub fn is_menu_mode(e: &Engine, menu: u32) -> bool {
    let manager = Ptr::<InterfaceManager>::new(e.mem.u32(INTERFACE_MANAGER));
    match menu {
        0 => e.get(manager, InterfaceManager::currentMode) != 1,
        1 => {
            if e.mem.u8(PIPBOY_FLAG) != 0 {
                menu_visible(e, MENU_INVENTORY)
                    || menu_visible(e, MENU_STATS)
                    || menu_visible(e, MENU_MAP)
            } else {
                e.get(manager, InterfaceManager::pipBoyMode) == 3
            }
        }
        2 => {
            let stack = manager.addr() + InterfaceManager::menuStack.off;
            for i in 0..10 {
                let id = e.mem.u32(stack + 4 * i);
                if id >= MENU_MIN {
                    return true;
                }
                if id == 0 {
                    break;
                }
            }
            false
        }
        3 => e.mem.u8(CONSOLE_OPEN) != 0,
        4 => {
            menu_visible(e, MENU_LOADING)
                && menu_visible(e, MENU_START)
                && menu_visible(e, MENU_HUD_MAIN)
        }
        _ => (MENU_MIN..=MENU_MAX).contains(&menu) && menu_visible(e, menu),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_MenuMode_Execute.
fn menu_mode(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let menu = read.int(0, 0);
    let mode = is_menu_mode(e, menu);
    if mode {
        a.set_result(e, 1.0);
    }
    print_line(e, "MenuMode %d >> %d", &[menu, u32::from(mode)]);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_MenuMode_Eval.
fn menu_mode_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if is_menu_mode(e, a.arg1) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// ---- GetItemCount (0x102F) ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetContainer.
fn base_container(e: &Engine, base: u32) -> u32 {
    if base == 0 {
        0
    } else if form_type(e, base) == CONTAINER {
        base + CONT_CONTAINER
    } else if vtable_says(e, base, VT_IS_ACTOR_BASE) {
        base + ACTOR_BASE_CONTAINER
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetContainerChangesList.
fn changes_list(e: &mut Engine, refr: u32) -> u32 {
    let extra = get_by_type(e, refr, EXTRA_CONTAINER_CHANGES);
    if extra == 0 {
        return 0;
    }
    let data = e.get(
        Ptr::<ExtraContainerChanges>::new(extra),
        ExtraContainerChanges::data,
    );
    if data == 0 {
        return 0;
    }
    e.get(
        Ptr::<ContainerChangesData>::new(data),
        ContainerChangesData::objList,
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntry::HasExtraLeveledItem.
fn has_extra_leveled_item(e: &Engine, entry: u32) -> bool {
    let lists = e.get(
        Ptr::<ContChangesEntry>::new(entry),
        ContChangesEntry::extendData,
    );
    if lists == 0 {
        return false;
    }
    let mut node = e.mem.u32(lists);
    while node != 0 {
        let n = Ptr::<ListNode>::new(node);
        let list = e.get(n, ListNode::data);
        node = e.get(n, ListNode::next);
        if list != 0 && e.mem.u8(list + EXTRA_LEVELED_ITEM_BYTE) & 0x80 != 0 {
            return true;
        }
    }
    false
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: GetFormCount.
/// A form's count: the base container's entries summed, then the reference's changes entry,
/// added to it (none below 0) unless the entry holds a leveled item or the base has none, when it
/// is the entry's alone (none below 0).
fn form_count(e: &Engine, container_list: u32, changes: u32, form: u32) -> i32 {
    let mut count: i32 = 0;
    let mut node = container_list;
    while node != 0 {
        let n = Ptr::<ListNode>::new(node);
        let object = e.get(n, ListNode::data);
        node = e.get(n, ListNode::next);
        if object != 0 {
            let o = Ptr::<ContainerObject>::new(object);
            if e.get(o, ContainerObject::form) == form {
                count = count.wrapping_add(e.get(o, ContainerObject::count));
            }
        }
    }
    let mut node = changes;
    while node != 0 {
        let n = Ptr::<ListNode>::new(node);
        let entry = e.get(n, ListNode::data);
        node = e.get(n, ListNode::next);
        if entry == 0 {
            continue;
        }
        let en = Ptr::<ContChangesEntry>::new(entry);
        if e.get(en, ContChangesEntry::form) != form {
            continue;
        }
        let delta = e.get(en, ContChangesEntry::countDelta);
        let total = if count != 0 && !has_extra_leveled_item(e, entry) {
            count.wrapping_add(delta)
        } else {
            delta
        };
        return total.max(0);
    }
    count
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetItemCount.
/// A reference's count of a form, or of a form list's forms summed; 0 when its base form has no
/// container, or for no form.
pub fn item_count(e: &mut Engine, refr: u32, form: u32) -> i32 {
    let container = base_container(e, base_form(e, refr));
    if container == 0 {
        return 0;
    }
    let list = Ptr::<TESContainer>::new(container)
        .at(TESContainer::list)
        .addr();
    let changes = changes_list(e, refr);
    if form == 0 {
        return 0;
    }
    if form_type(e, form) != LIST_FORM {
        return form_count(e, list, changes, form);
    }
    list_items(e, form_list(form))
        .into_iter()
        .filter(|&item| item != 0)
        .fold(0i32, |sum, item| {
            sum.wrapping_add(form_count(e, list, changes, item))
        })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_GetItemCount_Execute.
fn get_item_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && a.this_obj != 0 {
        let count = item_count(e, a.this_obj, read.form(0));
        a.set_result(e, f64::from(count));
    }
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_GetItemCount_Eval.
fn get_item_count_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.this_obj != 0 {
        let count = item_count(e, a.this_obj, a.arg1);
        e.mem.set_f64(a.result, f64::from(count));
    }
    handled(true)
}

// ---- GetContainer (0x10CF) ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_GetContainer_Execute.
/// The container the script runs in, or the one an inventory reference stands for; the result
/// left as it was otherwise.
fn get_container(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.containing_obj != 0 {
        let id = form_id(e, a.containing_obj);
        a.set_form_result(e, id);
    } else if a.this_obj != 0 {
        let id = form_id(e, a.this_obj);
        let container = nvse::invref::get(e, id).map_or(0, |r| r.container);
        if container != 0 {
            let id = form_id(e, container);
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// ---- IsInList (0x1174) ----

fn ref_in_form_list(e: &mut Engine, list_form: u32, refr: u32) -> bool {
    let base = ref_base_form(e, refr);
    list_items(e, form_list(list_form)).contains(&base)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_IsInList_Execute.
fn is_in_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let list = read.form(0);
    if read.ok && list != 0 && a.this_obj != 0 && ref_in_form_list(e, list, a.this_obj) {
        a.set_result(e, 1.0);
    }
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_IsInList_Eval.
fn is_in_list_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.arg1 != 0 && a.this_obj != 0 {
        let found = ref_in_form_list(e, a.arg1, a.this_obj);
        e.mem.set_f64(a.result, f64::from(u8::from(found)));
    }
    handled(true)
}

// ---- GetHitLocation (0x1187) ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h:
// Hook_GetHitLocation_Execute.
/// An actor's last hit location as JIP's `CopyHitDataHook` stores it in the hit data (step 3.5),
/// -1 for none.
fn get_hit_location(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mut location = -1i32;
    if a.this_obj != 0 && vtable_says(e, a.this_obj, VT_IS_ACTOR) {
        let process = e.get(Ptr::<Actor>::new(a.this_obj), Actor::pCurrentProcess);
        if process != 0 {
            let hit = e.vcall(process, VT_GET_HIT_DATA, &[]).u32();
            if hit != 0 {
                location = e.get(Ptr::<ActorHitData>::new(hit), ActorHitData::hitLocation);
            }
        }
    }
    a.set_result(e, f64::from(location));
    handled(true)
}

// ---- HasPerk (0x11C1) ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: DoGetPerkRank.
/// An actor's rank in a perk: with JIP's NPC perks patch its own; without it the player's, for
/// the player and for a teammate (as for teammates then), 0 for anyone else.
fn perk_rank(e: &mut Engine, actor: u32, perk: u32, for_teammates: bool) -> u8 {
    if actor == 0 || !vtable_says(e, actor, VT_IS_ACTOR) {
        return 0;
    }
    if e.ext::<Jip>().saved.npc_perks_patch {
        let rank = e
            .vcall(
                actor,
                VT_GET_PERK_RANK,
                &args![perk, u32::from(for_teammates)],
            )
            .u32();
        return rank as u8;
    }
    let teammate = e.get(Ptr::<Actor>::new(actor), Actor::bIsTeammate) != 0;
    if form_id(e, actor) == PLAYER_ID || teammate {
        let player = e.mem.u32(PLAYER);
        let alt = u32::from(for_teammates || teammate);
        return e.vcall(player, VT_GET_PERK_RANK, &args![perk, alt]).u32() as u8;
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_HasPerk_Execute.
/// 1 for a rank above 0 (a follower's perks count on followers); in the console the game's own
/// line with the rank.
fn has_perk(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let rank = perk_rank(e, a.this_obj, read.form(0), read.int(1, 0) != 0);
        if rank != 0 {
            a.set_result(e, 1.0);
        }
        if is_console_mode(e) {
            let format = nvse::table::guest_text(e, HAS_PERK_FORMAT);
            console_print(e, &String::from_utf8_lossy(&format), &[u32::from(rank)]);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_HasPerk_Eval.
fn has_perk_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if perk_rank(e, a.this_obj, a.arg1, a.arg2 != 0) != 0 {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// ---- the numeric settings (0x149A..0x149D) ----

/// A setting's name's first letter, as JIP's switches read it (`*name | 0x20`).
fn setting_type(e: &Engine, setting: u32) -> u8 {
    let name = e.get(Ptr::<Setting>::new(setting), Setting::name);
    e.mem.u8(name) | 0x20
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameSettings.cpp: Setting::ValidType.
fn valid_type(e: &Engine, setting: u32) -> bool {
    matches!(setting_type(e, setting), b'b' | b'f' | b'i' | b's' | b'u')
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: InitSettingMaps.
/// Every game setting with a name, then every INI setting of a numeric or string type from the
/// four INI collections, by name without case, a later one replacing an earlier.
fn setting_map(e: &Engine) -> HashMap<Vec<u8>, u32> {
    let mut map = HashMap::new();
    let mut add = |e: &Engine, setting: u32| {
        let name = e.get(Ptr::<Setting>::new(setting), Setting::name);
        let key = nvse::table::guest_text(e, name).to_ascii_lowercase();
        map.insert(key, setting);
    };
    let games = Ptr::<GameSettingCollection>::new(e.mem.u32(GAME_SETTINGS));
    let (buckets, table) = (
        e.get(games, GameSettingCollection::m_uiHashSize),
        e.get(games, GameSettingCollection::m_ppkHashTable),
    );
    for b in 0..buckets {
        let mut item = e.mem.u32(table + 4 * b);
        while item != 0 {
            let it = Ptr::<NiTMapItem>::new(item);
            let setting = e.get(it, NiTMapItem::m_val);
            if setting != 0 && e.get(Ptr::<Setting>::new(setting), Setting::name) != 0 {
                add(e, setting);
            }
            item = e.get(it, NiTMapItem::m_pkNext);
        }
    }
    for at in INI_COLLECTIONS {
        let collection = Ptr::<IniSettingCollection>::new(e.mem.u32(at));
        let head = collection.at(IniSettingCollection::settings).addr();
        for setting in list_items(e, head) {
            // A setting without a name would stop JIP here; none has one.
            if setting != 0
                && e.get(Ptr::<Setting>::new(setting), Setting::name) != 0
                && valid_type(e, setting)
            {
                add(e, setting);
            }
        }
    }
    map
}

fn find_setting(e: &mut Engine, name: &[u8]) -> Option<u32> {
    if e.ext::<Jip>().overrides.settings.is_none() {
        let map = setting_map(e);
        e.ext::<Jip>().overrides.settings = Some(map);
    }
    let key = name.to_ascii_lowercase();
    e.ext::<Jip>()
        .overrides
        .settings
        .as_ref()
        .and_then(|m| m.get(&key).copied())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameSettings.cpp: Setting::Get.
fn setting_get(e: &Engine, setting: u32) -> Option<f64> {
    let data = setting + Setting::data.off;
    Some(match setting_type(e, setting) {
        b'b' => f64::from(e.mem.u8(data)),
        b'u' => f64::from(e.mem.u32(data)),
        b'f' => f64::from(e.mem.f32(data)),
        b'i' => f64::from(e.mem.i32(data)),
        _ => return None,
    })
}

/// `(int)value` as the compiler makes it (`cvttsd2si`): 0x80000000 when out of range.
fn to_i32(value: f64) -> i32 {
    if value.is_nan() || value >= 2_147_483_648.0 || value < -2_147_483_648.0 {
        i32::MIN
    } else {
        value as i32
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: cvtd2ul.
/// The low word of the value truncated to 64 bits (`fisttp`): 0 when out of that range.
fn cvtd2ul(value: f64) -> u32 {
    const TWO_63: f64 = 9_223_372_036_854_775_808.0;
    if value.is_nan() || value >= TWO_63 || value < -TWO_63 {
        0
    } else {
        value as i64 as u32
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameSettings.cpp: Setting::Set (the double).
fn setting_set(e: &mut Engine, setting: u32, value: f64) {
    let data = setting + Setting::data.off;
    match setting_type(e, setting) {
        b'b' => e.mem.set_u8(data, u8::from(value != 0.0)),
        b'f' => e.mem.set_f32(data, value as f32),
        b'i' => e.mem.set_i32(data, to_i32(value)),
        b'u' => e.mem.set_u32(data, cvtd2ul(value)),
        _ => {}
    }
}

/// The name argument as JIP's 128-byte buffer holds it; `None` for a string setting's name
/// (`(settingName[0] | 0x20) != 's'`).
fn numeric_name(e: &Engine, read: &Read, command: &str) -> Option<Vec<u8>> {
    let name = identity::name_in_buffer(e, read, command);
    (name.first().copied().unwrap_or(0) | 0x20 != b's').then_some(name)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h:
// Hook_GetNumericGameSetting_Execute, Hook_GetNumericINISetting_Execute (the same code).
/// A game or INI setting's value by name, -1 for none; in the console the result, or
/// "SETTING NOT FOUND".
fn get_numeric_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(name) = numeric_name(e, &read, "GetNumericGameSetting") {
            match find_setting(e, &name) {
                Some(setting) => {
                    if let Some(v) = setting_get(e, setting) {
                        a.set_result(e, v);
                    }
                    print_result(e, &a);
                }
                None => print_line(e, "SETTING NOT FOUND", &[]),
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h:
// Hook_SetNumericGameSetting_Execute, Hook_SetNumericINISetting_Execute (the same code).
/// Sets a game or INI setting by name to a double (1), or prints "SETTING NOT FOUND".
fn set_numeric_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let value = read
            .args
            .get(1)
            .and_then(nvse::args::Arg::double)
            .unwrap_or(0.0);
        if let Some(name) = numeric_name(e, &read, "SetNumericGameSetting") {
            match find_setting(e, &name) {
                Some(setting) => {
                    setting_set(e, setting, value);
                    a.set_result(e, 1.0);
                }
                None => print_line(e, "SETTING NOT FOUND", &[]),
            }
        }
    }
    handled(true)
}

// ---- IsRefInList (0x1511) ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: IsRefInList.
/// The form's index in the list, or, for a reference not in it, its base form's; -1 for neither.
fn ref_index_in_list(e: &mut Engine, list_form: u32, form: u32) -> i32 {
    let index = index_in_list(e, list_form, form);
    if index < 0 && form != 0 && vtable_says(e, form, VT_IS_REFERENCE) {
        let base = ref_base_form(e, form);
        return index_in_list(e, list_form, base);
    }
    index
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_IsRefInList_Execute.
fn is_ref_in_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let list = read.form(0);
    let index = if read.ok && list != 0 {
        ref_index_in_list(e, list, read.form(1))
    } else {
        -1
    };
    a.set_result(e, f64::from(index));
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_IsRefInList_Eval.
fn is_ref_in_list_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.arg1 != 0 {
        let index = ref_index_in_list(e, a.arg1, a.arg2);
        e.mem.set_f64(a.result, f64::from(index));
    }
    handled(true)
}

// ---- GetModelPath (0x158E) ----

/// `kBaseComponentOffsets` (`nvse/GameForms.cpp` (JIP LN NVSE)): by form type, four signed bytes,
/// the offsets in words of its full name, scriptable form, destructible form and model.
const BASE_COMPONENT_OFFSETS: [u32; 121] = [
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000006,
    0x00000006, 0x09000006, 0x09000006, 0x00000006, 0x00000006, 0x00000000, 0x00000000, 0x00000000,
    0x0600000E, 0x00000000, 0x00000000, 0x00000006, 0x00000006, 0x0F1A170C, 0x0F1A170C, 0x0F1A170C,
    0x1F530F0C, 0x0F271A0C, 0x00000000, 0x121F1A0F, 0x0F1A170C, 0x13251E0C, 0x0F251E0C, 0x0F211A0C,
    0x0C000000, 0x0C000000, 0x0CFE00FB, 0x0C000000, 0x0C000000, 0x0C150000, 0x00000000, 0x0F1A170C,
    0x0F2D1A0C, 0x0F22270C, 0x37413D34, 0x37413D34, 0x13000000, 0x13000000, 0x0F211A0C, 0x1329220C,
    0x00000000, 0x0C000012, 0x00000000, 0x0F16000C, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000006, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000006, 0x00000000, 0x00000000, 0x00000000, 0x00000006, 0x00000000, 0x0000060C,
    0x06000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x06000000, 0x00000006, 0x00000000,
    0x00000000, 0x0F00000C, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000006, 0x06000000,
    0x0C000000, 0x00000006, 0x00000000, 0x06000000, 0x00000000, 0x00000000, 0x06000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000006, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x0F231A0C,
    0x00000006, 0x00000000, 0x00000006, 0x00000006, 0x0F20000C, 0x00000006, 0x00000000, 0x00000006,
    0x00000006, 0x00000B06, 0x00000006, 0x0F00200C, 0x0F00000C, 0x00000006, 0x00000000, 0x00000000,
    0x00000000,
];

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetModelPath.
/// A form's model path (its `TESModel`'s text), 0 for none or an empty one.
fn form_model_path(e: &Engine, form: u32) -> u32 {
    let kind = usize::from(form_type(e, form));
    let offset = BASE_COMPONENT_OFFSETS
        .get(kind)
        .map_or(0, |w| (w >> 24) as u8 as i8 as i32);
    if offset == 0 {
        return 0;
    }
    let path = e.mem.u32(form.wrapping_add((offset * 4 + 4) as u32));
    if path != 0 && e.mem.u8(path) == 0 {
        0
    } else {
        path
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: Hook_GetModelPath_Execute.
/// The model path of the form given, or of the calling reference's base form (of a reference's
/// base form when one is given), into the result's string. A reference's own path
/// (`SetRefrModelPath`, `kHookRefFlag5F_RefrModelPath`) comes with that command (step 3.6);
/// until then none has one and its base form's answers, as JIP answers for such a reference.
fn get_model_path(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut path = 0;
    let given = read.form(0);
    if read.ok && (given != 0 || a.this_obj != 0) {
        let form = if given == 0 {
            base_form(e, a.this_obj)
        } else if vtable_says(e, given, VT_IS_REFERENCE) {
            base_form(e, given)
        } else {
            given
        };
        if form != 0 {
            path = form_model_path(e, form);
        }
    }
    let text = if path == 0 {
        Vec::new()
    } else {
        nvse::table::guest_text(e, path)
    };
    nvse::commands::string::assign_to_string_var(e, &a, &text);
    handled(true)
}

#[cfg(test)]
#[path = "overrides_tests.rs"]
mod tests;
