//! More of xNVSE's `Commands_Inventory.cpp` (xNVSE), batch 4 of I-12: the player's current ammo
//! and its rounds, the equipped weapon's mod flags, names (`SetName`, `SetNameEx`), cloned forms
//! (`TempCloneForm`, `IsClonedForm`), equipping (`EquipItem2`, `EquipMe`, `UnequipMe`) and the
//! player's hotkeys (`GetHotkeyItem`, `SetHotkeyItem`, `ClearHotkey`).

use super::string::{bs_string_set, fmt_params, format_string_args, tes_full_name};
use super::{console_print, double_words, dynamic_cast, is_console_mode, rtti, Read};
use crate::invref::{self, get_by_type};
use crate::table::{guest_str, Command, Param, Returns};
use crate::{handled, CommandArgs, EvalArgs, Nvse};
use engine_core::prelude::*;
use std::collections::HashSet;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_String`, `_Integer`, `_ObjectID`,
/// `_AnyForm`.
const STRING: u32 = 0x00;
const INT: u32 = 0x01;
const OBJECT_ID: u32 = 0x03;
const ANY_FORM: u32 = 0x3d;
/// `kParams_OneInt`, `kParams_OneString_OneOptionalObject`, `kParams_OneOptionalForm`,
/// `kParamsTempCloneForm`, `kParams_EquipItem`, `kParams_SetHotkeyItem`, `kParams_SetNameEx`
/// (xNVSE).
const ONE_INT: &[Param] = &[p("int", INT, false)];
const STRING_OPTIONAL_OBJECT: &[Param] = &[p("string", STRING, false), p("item", ANY_FORM, true)];
const ONE_OPTIONAL_FORM: &[Param] = &[p("form", ANY_FORM, true)];
const TEMP_CLONE_FORM: &[Param] = &[p("form", ANY_FORM, false), p("bInheritModIndex", INT, true)];
const EQUIP_ITEM: &[Param] = &[
    p("item", OBJECT_ID, false),
    p("silent", INT, true),
    p("lockEquip", INT, true),
];
const SET_HOTKEY_ITEM: &[Param] = &[p("hotkey", INT, false), p("item", OBJECT_ID, false)];
static SET_NAME_EX: [Param; 22] = fmt_params(&[p("inventory object", ANY_FORM, true)]);

/// `TESForm::typeID` (`+4`), `refID` (`+0xC`); `TESObjectREFR::baseForm` (`+0x20`) and
/// `extraDataList` (`+0x44`).
const FORM_TYPE: u32 = 4;
const FORM_ID: u32 = 0xc;
const REFR_BASE_FORM: u32 = 0x20;
const REFR_EXTRA: u32 = invref::REFR_EXTRA;
/// `PlayerCharacter::GetSingleton` (`*0x011DEA3C`).
const PLAYER: u32 = 0x011d_ea3c;
/// `Actor::baseProcess` (`+0x68`) and `BaseProcess::GetAmmoInfo` (virtual `+0x14C`); the ammo
/// info's `count` (`+4`) and `ammo` (`+8`).
const BASE_PROCESS: u32 = 0x68;
const GET_AMMO_INFO: u32 = 0x14c;
const AMMO_INFO_COUNT: u32 = 4;
const AMMO_INFO_AMMO: u32 = 8;
/// `kExtraData_Hotkey`, `kExtraData_WeaponModFlags`; `ExtraHotkey::index` and
/// `ExtraWeaponModFlags::flags` (a byte at `+0xC`); both 0x10 bytes, their vtables
/// (`s_ExtraHotkeyVtbl`, `s_ExtraWeaponModFlagsVtbl` (xNVSE)).
const EXTRA_HOTKEY: u32 = 0x4a;
const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8d;
const EXTRA_BYTE: u32 = 0xc;
const EXTRA_SIZE: u32 = 0x10;
const EXTRA_HOTKEY_VTBL: u32 = 0x0101_592c;
const EXTRA_WEAPON_MOD_FLAGS_VTBL: u32 = 0x0101_59a4;
/// `ExtraContainerChanges::data` (`+0xC`), its `objList` (`+0`).
const CHANGES_DATA: u32 = 0xc;
/// `BaseExtraList::RemoveByType` (`thiscall`).
const EXTRA_REMOVE_BY_TYPE: u32 = 0x0041_0140;
/// `FormHeap_Allocate`; `ExtraDataList`'s size and vtable (`010143E8`: its RTTI locator `01102DA4`
/// names `.?AVExtraDataList@@`; its one entry, `00410380`, is the deleting destructor the game
/// frees a stack's lists through).
const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
const EXTRA_LIST_SIZE: u32 = 0x20;
const EXTRA_LIST_VTBL: u32 = 0x0101_43e8;
/// `Actor::EquipItem` and `Actor::UnequipItem` (`thiscall`, six words after the actor).
const ACTOR_EQUIP_ITEM: u32 = 0x0088_c650;
const ACTOR_UNEQUIP_ITEM: u32 = 0x0088_c790;
/// The equipable form types `EquipItem2` takes: `kFormType_TESObjectARMO`, `_TESObjectBOOK`,
/// `_TESObjectWEAP`, `_TESAmmo`, `_AlchemyItem`.
const TYPE_ARMOR: u8 = 0x18;
const TYPE_BOOK: u8 = 0x19;
const TYPE_WEAPON: u8 = 0x28;
const TYPE_AMMO: u8 = 0x29;
const TYPE_ALCHEMY: u8 = 0x2f;
/// `CreateFormInstance` (`0x00465110`, `cdecl`, the form type), `TESForm::CopyFrom` (virtual
/// `+0x108`), `TESForm::SetRefID` (virtual `+0x128`), `DataHandler::DoAddForm` (`0x004603B0`,
/// `thiscall`) on the data handler (`*0x011C3F2C`), `LookupFormByID` (`0x004839C0`);
/// `TESBoundObject::bounds` (six `SInt16`s at `+0x24`).
const CREATE_FORM_INSTANCE: u32 = 0x0046_5110;
const COPY_FROM: u32 = 0x108;
const SET_REF_ID: u32 = 0x128;
const DO_ADD_FORM: u32 = 0x0046_03b0;
const DATA_HANDLER: u32 = 0x011c_3f2c;
const LOOKUP_FORM: u32 = 0x0048_39c0;
const BOUNDS: u32 = 0x24;

/// `s_clonedFormsWithInheritedModIdx` (xNVSE): the ids `TempCloneForm` gave a clone in its
/// calling script's file. Not saved: xNVSE keeps it for the session.
#[derive(Default)]
pub struct State {
    cloned: HashSet<u32>,
}

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    needs_ref: bool,
    params: &'static [Param],
    execute: AbiFn,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref,
        params,
        execute,
        eval: None,
        returns: Returns::Default,
    }
}

const fn form_cmd(
    opcode: u32,
    name: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
) -> Command {
    Command {
        returns: Returns::Form,
        ..cmd(opcode, name, "", help, false, params, execute)
    }
}

pub static COMMANDS: &[Command] = &[
    cmd(
        0x144c,
        "SetName",
        "",
        "sets the name of the object.",
        false,
        STRING_OPTIONAL_OBJECT,
        set_name,
    ),
    form_cmd(
        0x144d,
        "GetHotkeyItem",
        "returns the item assigned to the specified hotkey from 1 to 8",
        ONE_INT,
        get_hotkey_item,
    ),
    cmd(
        0x14c5,
        "GetPlayerCurrentAmmoRounds",
        "",
        "returns the current number of rounds in the clip of the player",
        false,
        &[],
        get_player_current_ammo_rounds,
    ),
    cmd(
        0x14c6,
        "SetPlayerCurrentAmmoRounds",
        "",
        "sets the current number of rounds in the clip of the player",
        false,
        ONE_INT,
        set_player_current_ammo_rounds,
    ),
    form_cmd(
        0x14c7,
        "GetPlayerCurrentAmmo",
        "returns the current ammo in the weapon held by the player",
        &[],
        get_player_current_ammo,
    ),
    cmd(
        0x14da,
        "SetNameEx",
        "",
        "sets the name of the object based on the format string",
        false,
        &SET_NAME_EX,
        set_name_ex,
    ),
    form_cmd(
        0x14dd,
        "TempCloneForm",
        "\"clones the specified form and returns a new base form which will be saved in the save game.\"",
        TEMP_CLONE_FORM,
        temp_clone_form,
    ),
    cmd(
        0x14de,
        "IsClonedForm",
        "IsCloned",
        "returns whether the specified form is a created object or not.",
        false,
        ONE_OPTIONAL_FORM,
        is_cloned_form,
    ),
    Command {
        eval: Some(get_equipped_weapon_mod_flags_eval),
        ..cmd(
            0x14f2,
            "GetEquippedWeaponModFlags",
            "",
            "returns equipped weapon mod flags",
            true,
            &[],
            get_equipped_weapon_mod_flags,
        )
    },
    cmd(
        0x14f3,
        "SetEquippedWeaponModFlags",
        "",
        "sets equipped weapon mod flags",
        true,
        ONE_INT,
        set_equipped_weapon_mod_flags,
    ),
    cmd(
        0x15b6,
        "EquipItem2",
        "",
        "equips and runs onequip block",
        true,
        EQUIP_ITEM,
        equip_item2,
    ),
    cmd(
        0x15b7,
        "EquipMe",
        "",
        "equips the calling object on its owner",
        true,
        &[],
        equip_me,
    ),
    cmd(
        0x15b8,
        "UnequipMe",
        "",
        "unequips the calling object on its owner",
        true,
        &[],
        unequip_me,
    ),
    cmd(
        0x15bb,
        "SetHotkeyItem",
        "",
        "sets the item associated with a hotkey.",
        false,
        SET_HOTKEY_ITEM,
        set_hotkey_item,
    ),
    cmd(
        0x15bc,
        "ClearHotkey",
        "",
        "\"clears the item associated with the specified hotkey.\"",
        false,
        ONE_INT,
        clear_hotkey,
    ),
];

// ---- the player's ammo ----

/// The player's `BaseProcess::GetAmmoInfo()` (xNVSE reads the player's process without a check;
/// it always has one).
fn player_ammo_info(e: &mut Engine) -> Option<u32> {
    let player = e.global::<u32>(PLAYER);
    if player == 0 {
        return None;
    }
    let process = e.mem.u32(player + BASE_PROCESS);
    let info = e.vcall(process, GET_AMMO_INFO, &args![]).u32();
    (info != 0).then_some(info)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetPlayerCurrentAmmoRounds_Execute.
fn get_player_current_ammo_rounds(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if let Some(info) = player_ammo_info(e) {
        let count = e.mem.u32(info + AMMO_INFO_COUNT);
        a.set_result(e, f64::from(count));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_SetPlayerCurrentAmmoRounds_Execute.
fn set_player_current_ammo_rounds(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(info) = player_ammo_info(e) {
            e.mem.set_u32(info + AMMO_INFO_COUNT, read.int(0, 0));
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetPlayerCurrentAmmo_Execute.
/// The ammo's id; an ammo info without ammo is read through as xNVSE does (the game stops).
fn get_player_current_ammo(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if let Some(info) = player_ammo_info(e) {
        let ammo = e.mem.u32(info + AMMO_INFO_AMMO);
        let id = e.mem.u32(ammo + FORM_ID);
        a.set_form_result(e, id);
    }
    handled(true)
}

// ---- the equipped weapon's mod flags ----

/// The equipped weapon (slot 5, `MatchBySlot(5)`) and its worn extra data list, or `None`.
fn equipped_weapon(e: &mut Engine, reference: u32) -> Option<u32> {
    let (item, xdata) = super::inventory_forms::find_equipped(
        e,
        reference,
        super::inventory_forms::mask_for_slot(5),
    );
    (item != 0 && xdata != 0).then_some(xdata)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// GetEquippedWeaponModFlags_Call.
fn weapon_mod_flags(e: &mut Engine, reference: u32) -> f64 {
    let Some(xdata) = equipped_weapon(e, reference) else {
        return 0.0;
    };
    let flags = get_by_type(e, xdata, EXTRA_WEAPON_MOD_FLAGS);
    if flags != 0 {
        f64::from(e.mem.u8(flags + EXTRA_BYTE))
    } else {
        0.0
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetEquippedWeaponModFlags_Execute.
fn get_equipped_weapon_mod_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let flags = weapon_mod_flags(e, a.this_obj);
    a.set_result(e, flags);
    if is_console_mode(e) {
        console_print(e, "Weapon Mod Flags: %f", &double_words(flags));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetEquippedWeaponModFlags_Eval.
fn get_equipped_weapon_mod_flags_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    e.mem.set_f64(a.result, 0.0);
    if a.this_obj != 0 {
        let flags = weapon_mod_flags(e, a.this_obj);
        e.mem.set_f64(a.result, flags);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_SetEquippedWeaponModFlags_Execute.
/// Flags 0-7 on the equipped weapon's worn list: its `ExtraWeaponModFlags` changed, made when
/// missing, or removed (and freed) for 0.
fn set_equipped_weapon_mod_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let flags = read.int(0, 0);
    if flags > 7 {
        return handled(true);
    }
    let Some(xdata) = equipped_weapon(e, a.this_obj) else {
        return handled(true);
    };
    let extra = get_by_type(e, xdata, EXTRA_WEAPON_MOD_FLAGS);
    if extra != 0 {
        if flags != 0 {
            e.mem.set_u8(extra + EXTRA_BYTE, flags as u8);
        } else {
            invref::extra_remove(e, xdata, extra, true);
        }
    } else if flags != 0 {
        let extra = invref::create_extra(
            e,
            EXTRA_WEAPON_MOD_FLAGS,
            EXTRA_SIZE,
            EXTRA_WEAPON_MOD_FLAGS_VTBL,
        );
        if extra != 0 {
            e.mem.set_u8(extra + EXTRA_BYTE, flags as u8);
            invref::extra_add(e, xdata, extra);
        }
    }
    handled(true)
}

// ---- names ----

/// `TESFullName::name` (a `String` at `+4`).
const FULL_NAME_STRING: u32 = 4;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_SetName_Execute.
/// The name of the given form (a reference's base form), else of the caller's base form. The read
/// is not checked: a string it did not reach is uninitialised in the C++, empty here.
fn set_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = read.text(0);
    let mut form = super::try_get_refr_parent(e, read.form(1));
    if form == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        form = e.mem.u32(a.this_obj + REFR_BASE_FORM);
    }
    let name = tes_full_name(e, form);
    if name != 0 {
        bs_string_set(e, name + FULL_NAME_STRING, &text);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_SetNameEx_Execute.
/// The formatted name on the given form, else on the calling reference (whose base form's name
/// `TESForm::GetFullName` gives).
fn set_name_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = format_string_args(e, &a, SET_NAME_EX.len() as u32);
    if !f.ok {
        return handled(true);
    }
    let mut form = f.form(0);
    if form == 0 && a.this_obj != 0 {
        form = a.this_obj;
    }
    if form == 0 {
        return handled(true);
    }
    let name = tes_full_name(e, form);
    if name != 0 {
        bs_string_set(e, name + FULL_NAME_STRING, &f.text);
    }
    handled(true)
}

/// `GetFullName(TESForm *)` (xNVSE `GameForms.cpp`) as a guest string for the console's `%s`:
/// the full name's text, `"<no name>"` without one, `"<NULL>"` for no form.
fn full_name_text(e: &mut Engine, form: u32) -> u32 {
    if form == 0 {
        return guest_str(e, b"<NULL>");
    }
    let name = tes_full_name(e, form);
    if name != 0 {
        let (data, len) = (e.mem.u32(name + 4), e.mem.u16(name + 8));
        if data != 0 && len != 0 {
            return data;
        }
    }
    guest_str(e, b"<no name>")
}

// ---- cloned forms ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_IsClonedForm_Execute.
/// 1 for a form of file index 0xFF (made in game) or one `TempCloneForm` put in its script's file.
fn is_cloned_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let mut form = super::try_get_refr_parent(e, read.form(0));
    if form == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        form = e.mem.u32(a.this_obj + REFR_BASE_FORM);
    }
    let id = e.mem.u32(form + FORM_ID);
    if id >> 24 == 0xff || e.ext::<Nvse>().inventory_items.cloned.contains(&id) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameForms.cpp: TESForm::CloneForm,
// TESForm::DoAddForm.
/// A new form of the same type made by the game, a copy of `form` (`CopyFrom`, then the bounds,
/// which some forms' copies leave out), added to the data handler; 0 when the game makes none.
/// xNVSE's `persist` is not used by `DoAddForm`, so a "temporary" clone is added the same way.
fn clone_form(e: &mut Engine, form: u32) -> u32 {
    let kind = u32::from(e.mem.u8(form + FORM_TYPE));
    let made = e.call(CREATE_FORM_INSTANCE, &args![kind]).u32();
    if made == 0 {
        return 0;
    }
    e.vcall(made, COPY_FROM, &args![form]);
    let bound = dynamic_cast(e, made, rtti::TES_FORM, rtti::TES_BOUND_OBJECT);
    if bound != 0 {
        let source = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_BOUND_OBJECT);
        if source != 0 {
            for i in 0..6 {
                let v = e.mem.u16(source + BOUNDS + 2 * i);
                e.mem.set_u16(bound + BOUNDS + 2 * i, v);
            }
        }
    }
    let handler = e.global::<u32>(DATA_HANDLER);
    e.call(DO_ADD_FORM, &args![handler, made]);
    made
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// CloneForm_Execute (with `bPersist` false), Cmd_TempCloneForm_Execute; GameAPI.cpp:
// GetNextFreeFormID(UInt32).
/// The clone's id. With `bInheritModIndex`, the clone takes the first unused id after the calling
/// script's when that id is still in the script's file, and is remembered for `IsClonedForm`.
fn temp_clone_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let inherit = read.int(1, 0);
    let mut form = read.form(0);
    if form == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        form = e.mem.u32(a.this_obj + REFR_BASE_FORM);
    }
    let clone = clone_form(e, form);
    if clone == 0 {
        return handled(true);
    }
    if inherit != 0 {
        let script_id = e.mem.u32(a.script + FORM_ID);
        let mut next = script_id;
        loop {
            next = next.wrapping_add(1);
            if e.call(LOOKUP_FORM, &args![next]).u32() == 0 {
                break;
            }
        }
        if next >> 24 == script_id >> 24 {
            e.vcall(clone, SET_REF_ID, &args![next, 1u32]);
            e.ext::<Nvse>().inventory_items.cloned.insert(next);
        }
    }
    let id = e.mem.u32(clone + FORM_ID);
    a.set_form_result(e, id);
    if is_console_mode(e) {
        console_print(e, "Created cloned form: %08x", &[id]);
    }
    handled(true)
}

// ---- equipping ----

/// The reference as an actor (`DYNAMIC_CAST(ref, TESObjectREFR, Actor)`), 0 when it is not one.
fn as_actor(e: &mut Engine, reference: u32) -> u32 {
    dynamic_cast(e, reference, rtti::TES_OBJECT_REFR, rtti::ACTOR)
}

/// `ItemInEntryDataListMatcher` with `tList::Find`: the first changes entry for `item`, or 0.
fn entry_for_item(e: &Engine, list: u32, item: u32) -> u32 {
    let mut node = list;
    let mut walked = 0usize;
    while node != 0 {
        if !invref::list_step(e, &mut walked) {
            return 0;
        }
        let entry = e.mem.u32(node);
        if entry != 0 && e.mem.u32(entry + invref::ENTRY_TYPE) == item {
            return entry;
        }
        node = e.mem.u32(node + 4);
    }
    0
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_EquipItem2_Execute.
/// The actor equips an armor, book, weapon, ammo or aid item it holds, with the first extra data
/// list of its stack: the whole stack for ammo and for a thrown weapon (type above 9), else one.
/// The second argument locks the item on (`noUnequip`), the third (1 when not given) hides the
/// message. The OnEquip block is the game's own, as for `EquipItem`.
fn equip_item2(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (item, no_unequip, no_message) = (read.form(0), read.int(1, 0), read.int(2, 1));
    let actor = as_actor(e, a.this_obj);
    if actor == 0 {
        return handled(true);
    }
    let kind = e.mem.u8(item + FORM_TYPE);
    if ![TYPE_ARMOR, TYPE_BOOK, TYPE_WEAPON, TYPE_AMMO, TYPE_ALCHEMY].contains(&kind) {
        return handled(true);
    }
    let changes = get_by_type(e, actor + REFR_EXTRA, invref::EXTRA_CONTAINER_CHANGES);
    if changes == 0 {
        return handled(true);
    }
    let data = e.mem.u32(changes + CHANGES_DATA);
    if data == 0 || e.mem.u32(data) == 0 {
        return handled(true);
    }
    let entry = entry_for_item(e, e.mem.u32(data), item);
    if entry == 0 {
        return handled(true);
    }
    let delta = e.mem.u32(entry + invref::ENTRY_COUNT_DELTA);
    let mut count = 1;
    if kind == TYPE_WEAPON {
        let weapon = dynamic_cast(e, item, rtti::TES_FORM, rtti::TES_OBJECT_WEAP);
        if weapon != 0 && e.mem.u8(weapon + super::weapons::field::WEAPON_TYPE) > 9 {
            count = delta;
        }
    } else if kind == TYPE_AMMO {
        count = delta;
    }
    let extend = e.mem.u32(entry + invref::ENTRY_EXTEND_DATA);
    let xdata = if extend != 0 { e.mem.u32(extend) } else { 0 };
    e.call(
        ACTOR_EQUIP_ITEM,
        &args![
            actor,
            item,
            count,
            xdata,
            1u32,
            u32::from(no_unequip != 0),
            u32::from(no_message != 0)
        ],
    );
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_EquipMe_Execute.
/// The calling object equipped on its owner (the containing reference, when an actor) with the
/// calling reference's extra data; else, on an inventory reference, its stack equipped when the
/// reference is released.
fn equip_me(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let owner = as_actor(e, a.containing_obj);
    if a.this_obj == 0 {
        return handled(true);
    }
    if owner != 0 {
        let base = e.mem.u32(a.this_obj + REFR_BASE_FORM);
        e.call(
            ACTOR_EQUIP_ITEM,
            &args![owner, base, 1u32, a.this_obj + REFR_EXTRA, 1u32, 0u32, 1u32],
        );
    } else {
        let id = e.mem.u32(a.this_obj + FORM_ID);
        if invref::for_ref_id(e, id) {
            invref::set_equipped(e, id, true);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_UnequipMe_Execute.
/// The calling object's base form unequipped from its owner (one, no extra data list given); else
/// an inventory reference's stack unequipped when the reference is released.
fn unequip_me(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let owner = as_actor(e, a.containing_obj);
    if a.this_obj == 0 {
        return handled(true);
    }
    if owner != 0 {
        let base = e.mem.u32(a.this_obj + REFR_BASE_FORM);
        e.call(
            ACTOR_UNEQUIP_ITEM,
            &args![owner, base, 1u32, 0u32, 1u32, 0u32, 1u32],
        );
    } else {
        let id = e.mem.u32(a.this_obj + FORM_ID);
        if invref::for_ref_id(e, id) {
            invref::set_equipped(e, id, false);
        }
    }
    handled(true)
}

// ---- hotkeys ----

/// The data of each node of a `tList` as xNVSE's `tList::Iterator` gives it: from the head node
/// until a node is missing or is empty and the last (`End`), an empty node before the last given
/// as 0. `None` when the list guard row ended the walk (`invref::list_step`).
fn iterate(e: &Engine, head: u32, walked: &mut usize) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    let mut node = head;
    while node != 0 {
        let (data, next) = (e.mem.u32(node), e.mem.u32(node + 4));
        if data == 0 && next == 0 {
            break;
        }
        if !invref::list_step(e, walked) {
            return None;
        }
        out.push(data);
        node = next;
    }
    Some(out)
}

/// The player's container changes entry list (`xChanges->data->objList`), `None` without
/// container changes. Changes without data are read through as xNVSE does (the game stops).
fn player_entries(e: &mut Engine) -> Option<u32> {
    let player = e.global::<u32>(PLAYER);
    let changes = get_by_type(e, player + REFR_EXTRA, invref::EXTRA_CONTAINER_CHANGES);
    if changes == 0 {
        return None;
    }
    let data = e.mem.u32(changes + CHANGES_DATA);
    Some(e.mem.u32(data))
}

/// The first extra data list (in the entries' and lists' order) holding an `ExtraHotkey` of
/// `index` among the entries `want` accepts, and that hotkey. An entry or a list without data
/// before the end is read through, as xNVSE does.
fn find_hotkey(
    e: &mut Engine,
    entries: u32,
    index: i64,
    want: &dyn Fn(&Engine, u32) -> bool,
) -> Option<(u32, u32)> {
    let mut walked = 0usize;
    for entry in iterate(e, entries, &mut walked)? {
        if !want(e, entry) {
            continue;
        }
        let extend = e.mem.u32(entry + invref::ENTRY_EXTEND_DATA);
        for xdata in iterate(e, extend, &mut walked)? {
            let hotkey = get_by_type(e, xdata, EXTRA_HOTKEY);
            if hotkey != 0 && i64::from(e.mem.u8(hotkey + EXTRA_BYTE)) == index {
                return Some((xdata, hotkey));
            }
        }
    }
    None
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetHotkeyItem_Execute.
/// The item of the player's stack holding hotkey 1-8 (stored as 0-7); in the console the item's
/// id and name, or that the hotkey is not assigned. Extra data lists without data are passed
/// over.
fn get_hotkey_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let index = read.int(0, 0).wrapping_sub(1);
    if read.ok && index < 8 {
        if let Some(entries) = player_entries(e) {
            let mut walked = 0usize;
            let Some(list) = iterate(e, entries, &mut walked) else {
                return handled(true);
            };
            for entry in list {
                let extend = e.mem.u32(entry + invref::ENTRY_EXTEND_DATA);
                let Some(lists) = iterate(e, extend, &mut walked) else {
                    return handled(true);
                };
                for xdata in lists {
                    if xdata == 0 {
                        continue;
                    }
                    let hotkey = get_by_type(e, xdata, EXTRA_HOTKEY);
                    if hotkey != 0 && u32::from(e.mem.u8(hotkey + EXTRA_BYTE)) == index {
                        let item = e.mem.u32(entry + invref::ENTRY_TYPE);
                        let id = e.mem.u32(item + FORM_ID);
                        a.set_form_result(e, id);
                        if is_console_mode(e) {
                            let name = full_name_text(e, item);
                            console_print(e, "GetHotkeyItem >> %08x (%s)", &[id, name]);
                        }
                        return handled(true);
                    }
                }
            }
        }
    }
    if is_console_mode(e) {
        console_print(e, "GetHotkeyItem >> Hotkey not assigned", &[]);
    }
    handled(true)
}

/// `new ExtraDataList()` in xNVSE: a value-initialised list (all zero, no vtable), from the form
/// heap here (xNVSE's comes from its own heap). The game frees a stack's lists through their
/// vtable (`ItemChange::DeleteAllExtra` `004BC780`, when the stack is used up or dropped, or the
/// inventory is torn down), so xNVSE's list crashes the game then (measured: hotkey 8 on the
/// player's stimpaks, then all of them removed, ends the real game with an access violation, run
/// `hk1`). With the list guard row (`NV_NVSE_LIST_GUARD`) the list gets the game's own vtable
/// and is freed as any other; without it, it is xNVSE's.
fn new_extra_list(e: &mut Engine) -> u32 {
    let list = e.call(FORM_HEAP_ALLOCATE, &args![EXTRA_LIST_SIZE]).u32();
    e.mem.write(list, &[0u8; EXTRA_LIST_SIZE as usize]);
    if super::array::fix_on(e, super::array::LIST_GUARD) {
        e.mem.set_u32(list, EXTRA_LIST_VTBL);
    }
    list
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_SetHotkeyItem_Execute.
/// Hotkey 1-8 (stored as 0-7; 0 and below pass xNVSE's signed check and store 255 and down) moved
/// to the item: taken off the first other stack holding it, then the item's first stack with a
/// hotkey has it changed, else its first extra data list gets a new `ExtraHotkey`. A stack without
/// extra data lists gets a new list through `EntryData::Add`, which counts it as one more item, as
/// in xNVSE (measured, run `inv4c`); the list is [`new_extra_list`]'s.
fn set_hotkey_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let index = (read.int(0, 0) as i32).wrapping_sub(1);
    let item = read.form(1);
    if index >= 8 {
        return handled(true);
    }
    let Some(entries) = player_entries(e) else {
        return handled(true);
    };
    let item_id = e.mem.u32(item + FORM_ID);
    let other = move |e: &Engine, entry: u32| {
        let kind = e.mem.u32(entry + invref::ENTRY_TYPE);
        e.mem.u32(kind + FORM_ID) != item_id
    };
    if let Some((xdata, _)) = find_hotkey(e, entries, i64::from(index), &other) {
        e.call(EXTRA_REMOVE_BY_TYPE, &args![xdata, EXTRA_HOTKEY]);
    }
    let mut walked = 0usize;
    let Some(list) = iterate(e, entries, &mut walked) else {
        return handled(true);
    };
    for entry in list {
        if other(e, entry) {
            continue;
        }
        let extend = e.mem.u32(entry + invref::ENTRY_EXTEND_DATA);
        let Some(lists) = iterate(e, extend, &mut walked) else {
            return handled(true);
        };
        let mut hotkey = 0;
        for xdata in lists {
            hotkey = get_by_type(e, xdata, EXTRA_HOTKEY);
            if hotkey != 0 {
                e.mem.set_u8(hotkey + EXTRA_BYTE, index as u8);
                break;
            }
        }
        if hotkey == 0 {
            let made = invref::create_extra(e, EXTRA_HOTKEY, EXTRA_SIZE, EXTRA_HOTKEY_VTBL);
            if made != 0 {
                e.mem.set_u8(made + EXTRA_BYTE, index as u8);
                if e.mem.u32(entry + invref::ENTRY_EXTEND_DATA) == 0 {
                    let list = new_extra_list(e);
                    invref::entry_add(e, entry, list);
                }
                let extend = e.mem.u32(entry + invref::ENTRY_EXTEND_DATA);
                if extend != 0 {
                    if e.mem.u32(extend) == 0 {
                        let list = new_extra_list(e);
                        e.mem.set_u32(extend, list);
                    }
                    let first = e.mem.u32(extend);
                    if first != 0 {
                        invref::extra_add(e, first, made);
                    }
                }
            }
        }
        break;
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_ClearHotkey_Execute.
/// Hotkey 1-8 taken off the first of the player's stacks holding it.
fn clear_hotkey(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let index = (read.int(0, 0) as i32).wrapping_sub(1);
    if index >= 8 {
        return handled(true);
    }
    let Some(entries) = player_entries(e) else {
        return handled(true);
    };
    if let Some((xdata, _)) = find_hotkey(e, entries, i64::from(index), &|_, _| true) {
        e.call(EXTRA_REMOVE_BY_TYPE, &args![xdata, EXTRA_HOTKEY]);
    }
    handled(true)
}

#[cfg(test)]
#[path = "inventory_items_tests.rs"]
mod tests;
