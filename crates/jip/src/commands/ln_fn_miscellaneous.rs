//! `functions_ln/ln_fn_miscellaneous.h` (JIP LN NVSE): form fields of many kinds (biped model lists,
//! idle paths, names, sounds, encounter zones, impact data sets, playable flags, water traits),
//! a few of the player's (power armour, quest items shown, regions, notes, the active quest), the
//! armour class (also a condition), whether the player has a reference's key (also a condition),
//! and a cell's coordinates. 43 of its 44 commands.
//!
//! Not built yet (no hook, patch or missing seam: left for the next agent of the lane):
//! `ar_Cat` (0x22C0), which copies one xNVSE array's elements into another through the plugin
//! array interface (`GetContainerType`, `ArrayData`'s keys and values, `ArrayHasKey`): it wants
//! the element conversions of that interface (a form element as a form pointer looked up again by
//! id) settled against xNVSE's `ArrayAPI` first.
//!
//! Layouts, each confirmed in the exe (the exe's translations in the engine crates name the
//! function or the Xbox PDB field; AUDIT-lane-B.md lists them): the player (`*011DEA3C`) quest
//! items shown `+0x240`, notes `+0x5E4` (a list, first node inline), regions `+0x764` (a
//! `TESRegionList`: its list at `+4`), active quest `+0x6B8`, can use power armour `+0x7C7`; a
//! reference's form flags `+8`, base form `+0x20`, cell `+0x40`, extra data `+0x44`; a cell's
//! flags `+0x24` (bit 0 interior), extra data `+0x28`, exterior coordinates `+0x48` (x, y);
//! `Character`'s `BipedAnim` from the vtable slot `+0x1E8` (slots at `+0x2C`, 0x10 bytes each),
//! `IsCharacter` slot `+0x218`; an armour's part mask `+0x74`, biped flags `+0x78`, biped model
//! list `+0x164` (its list `+4`); a weapon's biped model list `+0xE0` (its list `+4`), first
//! flags `+0x100`, impact data set `+0x24C`; ammunition flags `+0xAC`; a race's flags `+0x70`,
//! eyes' `+0x30`, hair's `+0x48`; a perk's playable byte `+0x3B`; an idle's model `+0x18`; an
//! animated object's idle `+0x38`; a container's open and close sounds `+0x8C`, `+0x90`; an
//! encounter zone's flags `+0x1E`; a water form's opacity `+0x70`, flags `+0x71`, shader values
//! from `+0x84`, drink effect `+0x178`; a global's name `+0x18` and value `+0x24`, the data
//! handler's globals `+0xE8`; a terminal's password note `+0xA0`; `ExtraLock` (0x2A) data `+0xC`,
//! its key `+4`; `ExtraEncounterZone` (0x74) zone `+0xC`; `BGSPickupPutdownSounds` (RTTI
//! `011832D4`) pickup `+4`, putdown `+8`; the global time multiplier `011AC3A0`
//! (`SetGlobalTimeMultiplier` writes it).

use super::extra;
use super::{
    assign_string, buffer_arg, cvttsd2si, full_name, list_items, list_prepend, model_path,
    rgb_dec_to_hex, rgb_hex_to_dec, set_form_result, set_model_path, string_set, the_name, type_of,
    Handlers,
};
use crate::game::ListNode;
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x221c, set_global_value),
    (0x2262, get_worldspace_parent_worldspace),
    (0x224a, get_cell_coords),
    (0x2218, get_biped_model_list),
    (0x2219, set_biped_model_list),
    (0x22aa, get_pc_can_use_power_armor),
    (0x22ab, get_idle_anim_path),
    (0x22ac, set_idle_anim_path),
    (0x22ad, ln_get_name),
    (0x22fc, ln_set_name),
    (0x22b0, get_global_time_multiplier),
    (0x22bf, get_water_form_effect),
    (0x22de, get_form_flag),
    (0x22df, set_form_flag),
    (0x22e2, get_ingestible_flag),
    (0x22e3, set_ingestible_flag),
    (0x22e4, is_skill_magazine),
    (0x22f3, get_anim_object_idle),
    (0x22f4, set_anim_object_idle),
    (0x2301, get_pickup_sound),
    (0x2302, set_pickup_sound),
    (0x2303, get_putdown_sound),
    (0x2304, set_putdown_sound),
    (0x2305, get_container_open_sound),
    (0x2306, set_container_open_sound),
    (0x2307, get_container_close_sound),
    (0x2308, set_container_close_sound),
    (0x230a, get_player_regions),
    (0x230d, get_zone_flag),
    (0x230e, set_zone_flag),
    (0x2316, get_impact_data_set),
    (0x2317, set_impact_data_set),
    (0x231c, ln_is_playable),
    (0x231d, ln_set_is_playable),
    (0x231e, get_zone),
    (0x22bd, add_note_ns),
    (0x2336, crosshair_ref_in_list),
    (0x2338, get_show_quest_items),
    (0x2339, get_water_trait),
    (0x233a, set_water_trait),
    (0x2279, get_active_quest),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x2264, get_armor_class, get_armor_class_eval),
    (0x22eb, player_has_key, player_has_key_eval),
];

// ---- form types (`kFormType_*` (JIP LN NVSE)) ----

const TES_HAIR: u8 = 0x0a;
const TES_EYES: u8 = 0x0b;
const TES_RACE: u8 = 0x0c;
const TES_SOUND: u8 = 0x0d;
const BGS_TERMINAL: u8 = 0x17;
const TES_OBJECT_ARMO: u8 = 0x18;
const TES_OBJECT_CONT: u8 = 0x1b;
const BGS_PLACEABLE_WATER: u8 = 0x23;
const TES_OBJECT_WEAP: u8 = 0x28;
const TES_AMMO: u8 = 0x29;
const ALCHEMY_ITEM: u8 = 0x2f;
const TES_OBJECT_CELL: u8 = 0x39;
const TES_IDLE_FORM: u8 = 0x48;
const TES_OBJECT_ANIO: u8 = 0x4d;
const TES_WATER_FORM: u8 = 0x4e;
const BGS_LIST_FORM: u8 = 0x55;
const BGS_PERK: u8 = 0x56;
const BGS_IMPACT_DATA_SET: u8 = 0x5f;
const BGS_ENCOUNTER_ZONE: u8 = 0x61;

/// `kVtbl_TESObjectARMO` (JIP's `IS_TYPE(form, TESObjectARMO)` compares the vtable).
const VTBL_ARMOR: u32 = 0x0102_a62c;

// ---- globals ----

/// `g_thePlayer` (`*011DEA3C`), `g_interfaceManager` (`*011D8A80`), `g_dataHandler`
/// (`*011C3F2C`), the global time multiplier (a float).
const PLAYER: u32 = 0x011d_ea3c;
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const DATA_HANDLER: u32 = 0x011c_3f2c;
const GLOBAL_TIME_MULT: u32 = 0x011a_c3a0;

// ---- fields ----

const FORM_FLAGS: u32 = 0x08;
const WORLD_PARENT: u32 = 0x70;
const REFR_BASE: u32 = 0x20;
const REFR_CELL: u32 = 0x40;
const REFR_EXTRA: u32 = 0x44;
const CELL_FLAGS: u32 = 0x24;
const CELL_EXTRA: u32 = 0x28;
const CELL_COORDS: u32 = 0x48;
const PLAYER_SHOW_QUEST_ITEMS: u32 = 0x240;
const PLAYER_NOTES: u32 = 0x5e4;
const PLAYER_ACTIVE_QUEST: u32 = 0x6b8;
const PLAYER_REGIONS_LIST: u32 = 0x764 + 4;
const PLAYER_CAN_USE_PA: u32 = 0x7c7;
const INTERFACE_CROSSHAIR_REF: u32 = 0xfc;
const DATA_HANDLER_GLOBALS: u32 = 0xe8;
const GLOBAL_NAME: u32 = 0x18;
const GLOBAL_DATA: u32 = 0x24;
const VT_GET_BIPED_ANIM: u32 = 0x1e8;
const VT_IS_CHARACTER: u32 = 0x218;
const BIPED_SLOTS: u32 = 0x2c;
const BIPED_SLOT_SIZE: u32 = 0x10;
const ARMOR_PART_MASK: u32 = 0x74;
const ARMOR_BIPED_FLAGS: u32 = 0x78;
const ARMOR_MODEL_LIST: u32 = 0x164 + 4;
const WEAPON_MODEL_LIST: u32 = 0xe0 + 4;
const WEAPON_FLAGS1: u32 = 0x100;
const WEAPON_IMPACT_DATA_SET: u32 = 0x24c;
const AMMO_FLAGS: u32 = 0xac;
const RACE_FLAGS: u32 = 0x70;
const EYES_FLAGS: u32 = 0x30;
const HAIR_FLAGS: u32 = 0x48;
const PERK_PLAYABLE: u32 = 0x3b;
const IDLE_ANIM: u32 = 0x18;
const ANIO_IDLE: u32 = 0x38;
const CONT_OPEN_SOUND: u32 = 0x8c;
const CONT_CLOSE_SOUND: u32 = 0x90;
const ZONE_FLAGS: u32 = 0x1e;
const WATER_OPACITY: u32 = 0x70;
const WATER_FLAGS: u32 = 0x71;
const WATER_VIS_DATA: u32 = 0x84;
const WATER_DRINK_EFFECT: u32 = 0x178;
const ALCH_FLAGS: u32 = 0xbc;
/// `AlchemyItem::magicItem` (+0x30), its effect list's first node (+0x10 of it).
const ALCH_EFFECTS: u32 = 0x30 + 0x10;
const EFFECT_CONDITIONS: u32 = 0x1c;
const CONDITION_OPCODE: u32 = 0x08;
const CONDITION_PARAM1: u32 = 0x0c;
const TERMINAL_PASSWORD: u32 = 0xa0;
const LIST_FORM_LIST: u32 = 0x18;
const PLACEABLE_WATER: u32 = 0x4c;
const SOUNDS_PICKUP: u32 = 0x04;
const SOUNDS_PUTDOWN: u32 = 0x08;
const EXTRA_WORD: u32 = 0x0c;
const LOCK_KEY: u32 = 0x04;

/// `ExtraLock`, `ExtraEncounterZone` (the constructor `00432E70` sets 0x74).
const EXTRA_LOCK: u32 = 0x2a;
const EXTRA_ENCOUNTER_ZONE: u32 = 0x74;

/// `RTTI_TESForm`, `RTTI_BGSPickupPutdownSounds` (the type descriptors the exe's own casts name:
/// engine-fallout-forms `bgspickupputdownsounds.rs`).
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_PICKUP_PUTDOWN_SOUNDS: u32 = 0x0118_32d4;

/// The condition function `HasPerk` (0x1C1) and the two perks IsSkillMagazine looks for:
/// Comprehension (0x31DE1) and Retention (0x31D6A).
const HAS_PERK: u32 = 0x1c1;
const COMPREHENSION: u32 = 0x0003_1de1;
const RETENTION: u32 = 0x0003_1d6a;

/// The buffers JIP extracts strings into: SetGlobalValue's 0x40 bytes, the paths' and names'
/// 0x80.
const VAR_NAME_BUFFER: usize = 0x40;
const TEXT_BUFFER: usize = 0x80;

fn player(e: &Engine) -> u32 {
    e.mem.u32(PLAYER)
}

fn base_of(e: &Engine, reference: u32) -> u32 {
    if reference == 0 {
        0
    } else {
        e.mem.u32(reference + REFR_BASE)
    }
}

fn is_armor(e: &Engine, form: u32) -> bool {
    form != 0 && super::is_type(e, form, VTBL_ARMOR)
}

fn of_type(e: &Engine, form: u32, kind: u8) -> bool {
    form != 0 && type_of(e, form) == kind
}

/// Sets the result to the form's id when there is one (`REFR_RES = form->refID`).
fn form_res(e: &mut Engine, a: &CommandArgs, form: u32) {
    if form != 0 {
        set_form_result(e, a, form);
    }
}

/// Argument `i`, a double parameter, as JIP's `double` output (`default` when not read).
fn double_arg(read: &Read, i: usize, default: f64) -> f64 {
    match read.args.get(i) {
        Some(Arg::Double(v)) => *v,
        _ => default,
    }
}

/// `tList::IsInList` on a list from its first node (bounded as [`list_items`] is).
fn in_list(e: &Engine, head: u32, item: u32) -> bool {
    list_items(e, Ptr::<ListNode>::new(head)).contains(&item)
}

/// The armour class of biped flags: 3 for power armour (0x80), 2 heavy (8), else 1.
fn class_of(flags: u32) -> f64 {
    if flags & 0x80 != 0 {
        3.0
    } else if flags & 8 != 0 {
        2.0
    } else {
        1.0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h: GetArmorClass.
/// A character's armour class: 0 for a reference that is not a character; 1 with no biped data
/// (JIP's `return true`); else the class of the first worn armour (by vtable) covering the upper
/// body (part mask bit 4), 0 with none.
fn armor_class(e: &mut Engine, character: u32) -> f64 {
    if character == 0 || e.vcall(character, VT_IS_CHARACTER, &[]).u32() & 0xff == 0 {
        return 0.0;
    }
    let equipment = e.vcall(character, VT_GET_BIPED_ANIM, &[]).u32();
    if equipment == 0 {
        return 1.0;
    }
    for slot in 0..20 {
        let armor = e.mem.u32(equipment + BIPED_SLOTS + BIPED_SLOT_SIZE * slot);
        if !is_armor(e, armor) {
            continue;
        }
        if e.mem.u32(armor + ARMOR_PART_MASK) & 4 != 0 {
            return class_of(e.mem.u32(armor + ARMOR_BIPED_FLAGS));
        }
    }
    0.0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetArmorClass_Execute.
/// An armour's class, or the calling character's; echoed in the console. A form that is not an
/// armour returns at once (no echo).
fn get_armor_class(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let armor = read.form(0);
    if armor != 0 {
        if !is_armor(e, armor) {
            return handled(true);
        }
        a.set_result(e, class_of(e.mem.u32(armor + ARMOR_BIPED_FLAGS)));
    } else if a.this_obj != 0 {
        let class = armor_class(e, a.this_obj);
        a.set_result(e, class);
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetArmorClass_Eval.
fn get_armor_class_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let class = armor_class(e, a.this_obj);
    e.mem.set_f64(a.result, class);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetGlobalValue_Execute.
/// The first global whose name matches (case-insensitively, `StrCompareCI`) gets the value.
fn set_global_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = buffer_arg(e, &read, 0, VAR_NAME_BUFFER, "SetGlobalValue");
    if !read.ok {
        return handled(true);
    }
    let value = read.float(1, 0.0);
    let handler = e.mem.u32(DATA_HANDLER);
    if handler == 0 {
        return handled(true);
    }
    for global in list_items(e, Ptr::new(handler + DATA_HANDLER_GLOBALS)) {
        if global == 0 {
            continue;
        }
        let text = e.mem.u32(global + GLOBAL_NAME);
        // JIP reads a name with no buffer through the null pointer: passed over here.
        if text == 0 {
            continue;
        }
        let own = nvse::table::guest_text(e, text);
        if crate::utility::str_compare_ci(&own, &name).0 == 0 {
            e.mem.set_f32(global + GLOBAL_DATA, value);
            break;
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetWorldspaceParentWorldspace_Execute.
fn get_worldspace_parent_worldspace(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let space = read.form(0);
    if read.ok && space != 0 {
        let parent = e.mem.u32(space + WORLD_PARENT);
        form_res(e, &a, parent);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetCellCoords_Execute.
/// A string map of the cell's grid x and y (the calling reference's cell, else the player's,
/// when none is given; 0 and 0 for an interior or no cell); printed in the open console for an
/// exterior. A form that is not a cell makes no map.
fn get_cell_coords(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut cell = read.form(0);
    if !read.ok || (cell != 0 && type_of(e, cell) != TES_OBJECT_CELL) {
        return handled(true);
    }
    if cell == 0 {
        let from = if a.this_obj != 0 {
            a.this_obj
        } else {
            player(e)
        };
        cell = if from == 0 {
            0
        } else {
            e.mem.u32(from + REFR_CELL)
        };
    }
    // An exterior cell whose coordinates are missing (JIP would read through null) counts as
    // none.
    let coords = if cell != 0 && e.mem.u8(cell + CELL_FLAGS) & 1 == 0 {
        e.mem.u32(cell + CELL_COORDS)
    } else {
        0
    };
    let (x, y) = if coords == 0 {
        (0, 0)
    } else {
        (e.mem.u32(coords) as i32, e.mem.u32(coords + 4) as i32)
    };
    use super::arrays::Element;
    let id = super::arrays::create_string_map(
        e,
        a.script,
        &[b"x".to_vec(), b"y".to_vec()],
        &[Element::Number(f64::from(x)), Element::Number(f64::from(y))],
    );
    a.set_result(e, f64::from(id));
    if coords != 0 && nvse::commands::is_console_mode(e) {
        let line = format!("GetCellCoords >> {x}, {y}");
        crate::console::console_print_line(e, line.as_bytes());
    }
    handled(true)
}

/// The biped model list field of an armour (by vtable) or a weapon (by type), 0 for another form.
fn model_list_field(e: &Engine, form: u32) -> u32 {
    if is_armor(e, form) {
        form + ARMOR_MODEL_LIST
    } else if of_type(e, form, TES_OBJECT_WEAP) {
        form + WEAPON_MODEL_LIST
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetBipedModelList_Execute.
/// The form list of an armour's or weapon's biped models (the calling reference's base form when
/// none is given).
fn get_biped_model_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let form = match read.form(0) {
        0 => base_of(e, a.this_obj),
        f => f,
    };
    let field = model_list_field(e, form);
    if field != 0 {
        let list = e.mem.u32(field);
        form_res(e, &a, list);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetBipedModelList_Execute.
/// The list set (or cleared without one); a form that is not a form list sets nothing.
fn set_biped_model_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (form, list) = (read.form(0), read.form(1));
    if read.ok && (list == 0 || of_type(e, list, BGS_LIST_FORM)) {
        let field = model_list_field(e, form);
        if field != 0 {
            e.mem.set_u32(field, list);
        }
    }
    handled(true)
}

/// 1 when the player's byte at `field` is not 0.
fn player_flag(e: &mut Engine, w: &[u32], field: u32) -> Ret {
    let a = CommandArgs::new(w);
    let p = player(e);
    if p != 0 && e.mem.u8(p + field) != 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetPCCanUsePowerArmor_Execute.
fn get_pc_can_use_power_armor(e: &mut Engine, w: &[u32]) -> Ret {
    player_flag(e, w, PLAYER_CAN_USE_PA)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetShowQuestItems_Execute.
fn get_show_quest_items(e: &mut Engine, w: &[u32]) -> Ret {
    player_flag(e, w, PLAYER_SHOW_QUEST_ITEMS)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetIdleAnimPath_Execute.
/// The idle's model path (the virtual `GetModelPath`), `""` for another form.
fn get_idle_anim_path(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let idle = read.form(0);
    let text = if read.ok && of_type(e, idle, TES_IDLE_FORM) {
        model_path(e, idle + IDLE_ANIM)
    } else {
        None
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetIdleAnimPath_Execute.
fn set_idle_anim_path(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, TEXT_BUFFER, "SetIdleAnimPath");
    let idle = read.form(0);
    if read.ok && of_type(e, idle, TES_IDLE_FORM) {
        set_model_path(e, idle + IDLE_ANIM, &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_LNGetName_Execute.
/// A form's name; without one the calling reference's own name (JIP's `s_refNamesMap`, set by
/// `SetRefName`), else its base form's.
fn ln_get_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut text = None;
    if read.ok {
        let form = read.form(0);
        if form != 0 {
            text = Some(the_name(e, form));
        } else if a.this_obj != 0 {
            let own = crate::world_hooks::state(e)
                .ref_names
                .get(&a.this_obj)
                .map(|b| b.at);
            text = match own {
                Some(at) => Some(nvse::table::guest_text(e, at)),
                // JIP reads the base form without a test: a reference with none answers "".
                None => match base_of(e, a.this_obj) {
                    0 => None,
                    base => Some(the_name(e, base)),
                },
            };
        }
    }
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_LNSetName_Execute.
/// The form's (else the calling reference's) full name set (`GetFullName`: a reference's is its
/// base form's); a form without one, nothing.
fn ln_set_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = buffer_arg(e, &read, 0, TEXT_BUFFER, "LNSetName");
    if !read.ok {
        return handled(true);
    }
    let form = match read.form(1) {
        0 => a.this_obj,
        f => f,
    };
    if form != 0 {
        let full = full_name(e, form);
        if full != 0 {
            string_set(e, full + 4, &name);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetGlobalTimeMultiplier_Execute.
/// The global time multiplier; echoed in the console.
fn get_global_time_multiplier(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let value = e.mem.f32(GLOBAL_TIME_MULT);
    a.set_result(e, f64::from(value));
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetWaterFormEffect_Execute.
fn get_water_form_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let water = read.form(0);
    if read.ok && of_type(e, water, TES_WATER_FORM) {
        let effect = e.mem.u32(water + WATER_DRINK_EFFECT);
        form_res(e, &a, effect);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetFormFlag_Execute.
/// 1 when any of the bits is set in the form's flags; echoed in the console.
fn get_form_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if read.ok && form != 0 && e.mem.u32(form + FORM_FLAGS) & read.int(1, 0) != 0 {
        a.set_result(e, 1.0);
    }
    crate::console::print_result(e, &a);
    handled(true)
}

/// Sets (`on` not 0) or clears `bits` in the word at `at`.
fn set_bits_u32(e: &mut Engine, at: u32, bits: u32, on: u32) {
    let v = e.mem.u32(at);
    e.mem
        .set_u32(at, if on != 0 { v | bits } else { v & !bits });
}

/// The same on a byte (JIP's `UInt8` field, the bits' low byte).
fn set_bits_u8(e: &mut Engine, at: u32, bits: u32, on: u32) {
    let v = e.mem.u8(at);
    let bits = bits as u8;
    e.mem.set_u8(at, if on != 0 { v | bits } else { v & !bits });
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetFormFlag_Execute.
fn set_form_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if read.ok && form != 0 {
        set_bits_u32(e, form + FORM_FLAGS, read.int(1, 0), read.int(2, 0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetIngestibleFlag_Execute.
/// 1 when any of the bits is set in the ingestible's flags byte.
fn get_ingestible_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let alch = read.form(0);
    if read.ok
        && of_type(e, alch, ALCHEMY_ITEM)
        && u32::from(e.mem.u8(alch + ALCH_FLAGS)) & read.int(1, 0) != 0
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetIngestibleFlag_Execute.
fn set_ingestible_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let alch = read.form(0);
    if read.ok && of_type(e, alch, ALCHEMY_ITEM) {
        set_bits_u8(e, alch + ALCH_FLAGS, read.int(1, 0), read.int(2, 0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_IsSkillMagazine_Execute.
/// 1 when one of the ingestible's effects has a `HasPerk` condition on Comprehension or
/// Retention.
fn is_skill_magazine(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let alch = read.form(0);
    if !read.ok || !of_type(e, alch, ALCHEMY_ITEM) {
        return handled(true);
    }
    for effect in list_items(e, Ptr::new(alch + ALCH_EFFECTS)) {
        if effect == 0 {
            continue;
        }
        for condition in list_items(e, Ptr::new(effect + EFFECT_CONDITIONS)) {
            if condition == 0 || e.mem.u32(condition + CONDITION_OPCODE) != HAS_PERK {
                continue;
            }
            let param = e.mem.u32(condition + CONDITION_PARAM1);
            // JIP reads the parameter's id without a test: a condition with none is passed over.
            if param == 0 {
                continue;
            }
            let id = super::ref_id(e, param);
            if id == COMPREHENSION || id == RETENTION {
                a.set_result(e, 1.0);
                return handled(true);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h: PlayerHasKey.
/// A terminal: the player has its password note; else the reference's lock has a key the player
/// carries (JIP's `TESObjectREFR::GetItemCount`).
fn player_has_key_on(e: &mut Engine, reference: u32) -> bool {
    let p = player(e);
    let base = base_of(e, reference);
    if p == 0 || base == 0 {
        return false;
    }
    if type_of(e, base) == BGS_TERMINAL {
        let password = e.mem.u32(base + TERMINAL_PASSWORD);
        return password != 0 && in_list(e, p + PLAYER_NOTES, password);
    }
    let lock = extra::get_by_type(e, reference + REFR_EXTRA, EXTRA_LOCK);
    let data = if lock == 0 {
        0
    } else {
        e.mem.u32(lock + EXTRA_WORD)
    };
    let key = if data == 0 {
        0
    } else {
        e.mem.u32(data + LOCK_KEY)
    };
    key != 0 && crate::overrides::item_count(e, p, key) != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_PlayerHasKey_Execute.
fn player_has_key(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if player_has_key_on(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_PlayerHasKey_Eval.
fn player_has_key_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if player_has_key_on(e, a.this_obj) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetAnimObjectIdle_Execute.
fn get_anim_object_idle(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let anio = read.form(0);
    if read.ok && of_type(e, anio, TES_OBJECT_ANIO) {
        let idle = e.mem.u32(anio + ANIO_IDLE);
        form_res(e, &a, idle);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetAnimObjectIdle_Execute.
fn set_anim_object_idle(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (anio, idle) = (read.form(0), read.form(1));
    if read.ok && of_type(e, anio, TES_OBJECT_ANIO) && of_type(e, idle, TES_IDLE_FORM) {
        e.mem.set_u32(anio + ANIO_IDLE, idle);
    }
    handled(true)
}

/// `DYNAMIC_CAST(form, TESForm, BGSPickupPutdownSounds)`.
fn pickup_putdown_sounds(e: &mut Engine, form: u32) -> u32 {
    nvse::commands::dynamic_cast(e, form, RTTI_TES_FORM, RTTI_PICKUP_PUTDOWN_SOUNDS)
}

/// The form's pickup or putdown sound (`field`).
fn get_sound(e: &mut Engine, w: &[u32], field: u32) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let sounds = pickup_putdown_sounds(e, read.form(0));
        if sounds != 0 {
            let sound = e.mem.u32(sounds + field);
            form_res(e, &a, sound);
        }
    }
    handled(true)
}

/// The form's pickup or putdown sound (`field`) set to a sound; another form sets nothing.
fn set_sound(e: &mut Engine, w: &[u32], field: u32) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let sound = read.form(1);
    if read.ok && of_type(e, sound, TES_SOUND) {
        let sounds = pickup_putdown_sounds(e, read.form(0));
        if sounds != 0 {
            e.mem.set_u32(sounds + field, sound);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetPickupSound_Execute.
fn get_pickup_sound(e: &mut Engine, w: &[u32]) -> Ret {
    get_sound(e, w, SOUNDS_PICKUP)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetPickupSound_Execute.
fn set_pickup_sound(e: &mut Engine, w: &[u32]) -> Ret {
    set_sound(e, w, SOUNDS_PICKUP)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetPutdownSound_Execute.
fn get_putdown_sound(e: &mut Engine, w: &[u32]) -> Ret {
    get_sound(e, w, SOUNDS_PUTDOWN)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetPutdownSound_Execute.
fn set_putdown_sound(e: &mut Engine, w: &[u32]) -> Ret {
    set_sound(e, w, SOUNDS_PUTDOWN)
}

/// The container argument `i`, else the calling reference's base form (`None` with neither, as
/// JIP returns then).
fn container_arg(e: &Engine, a: &CommandArgs, read: &Read, i: usize) -> Option<u32> {
    match read.form(i) {
        0 if a.this_obj == 0 => None,
        0 => Some(base_of(e, a.this_obj)),
        f => Some(f),
    }
}

/// A container's open or close sound (`field`).
fn get_container_sound(e: &mut Engine, w: &[u32], field: u32) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    if let Some(cont) = container_arg(e, &a, &read, 0) {
        if of_type(e, cont, TES_OBJECT_CONT) {
            let sound = e.mem.u32(cont + field);
            form_res(e, &a, sound);
        }
    }
    handled(true)
}

/// A container's open or close sound (`field`) set; the sound must be one.
fn set_container_sound(e: &mut Engine, w: &[u32], field: u32) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let sound = read.form(0);
    if !read.ok || !of_type(e, sound, TES_SOUND) {
        return handled(true);
    }
    if let Some(cont) = container_arg(e, &a, &read, 1) {
        if of_type(e, cont, TES_OBJECT_CONT) {
            e.mem.set_u32(cont + field, sound);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetContainerOpenSound_Execute.
fn get_container_open_sound(e: &mut Engine, w: &[u32]) -> Ret {
    get_container_sound(e, w, CONT_OPEN_SOUND)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetContainerOpenSound_Execute.
fn set_container_open_sound(e: &mut Engine, w: &[u32]) -> Ret {
    set_container_sound(e, w, CONT_OPEN_SOUND)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetContainerCloseSound_Execute.
fn get_container_close_sound(e: &mut Engine, w: &[u32]) -> Ret {
    get_container_sound(e, w, CONT_CLOSE_SOUND)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetContainerCloseSound_Execute.
fn set_container_close_sound(e: &mut Engine, w: &[u32]) -> Ret {
    set_container_sound(e, w, CONT_CLOSE_SOUND)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetPlayerRegions_Execute.
/// An array of the regions the player is in (always made, empty with none).
fn get_player_regions(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    use super::arrays::Element;
    let p = player(e);
    let regions: Vec<Element> = if p == 0 {
        Vec::new()
    } else {
        list_items(e, Ptr::new(p + PLAYER_REGIONS_LIST))
            .into_iter()
            .filter(|&r| r != 0)
            .map(Element::Form)
            .collect()
    };
    let id = super::arrays::create_array(e, a.script, &regions);
    a.set_result(e, f64::from(id));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetZoneFlag_Execute.
fn get_zone_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let zone = read.form(0);
    if read.ok
        && of_type(e, zone, BGS_ENCOUNTER_ZONE)
        && u32::from(e.mem.u8(zone + ZONE_FLAGS)) & read.int(1, 0) != 0
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetZoneFlag_Execute.
fn set_zone_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let zone = read.form(0);
    if read.ok && of_type(e, zone, BGS_ENCOUNTER_ZONE) {
        set_bits_u8(e, zone + ZONE_FLAGS, read.int(1, 0), read.int(2, 0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetImpactDataSet_Execute.
fn get_impact_data_set(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let weapon = read.form(0);
    if read.ok && of_type(e, weapon, TES_OBJECT_WEAP) {
        let set = e.mem.u32(weapon + WEAPON_IMPACT_DATA_SET);
        form_res(e, &a, set);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetImpactDataSet_Execute.
fn set_impact_data_set(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (weapon, set) = (read.form(0), read.form(1));
    if read.ok && of_type(e, weapon, TES_OBJECT_WEAP) && of_type(e, set, BGS_IMPACT_DATA_SET) {
        e.mem.set_u32(weapon + WEAPON_IMPACT_DATA_SET, set);
    }
    handled(true)
}

/// The playable flag of a form type: its field, its bit, and whether the bit means "not
/// playable" (armour 0x40, weapon 0x80, ammunition 2) or "playable" (race, eyes, hair 1); a perk's
/// whole byte is its playable flag (`None`).
fn playable_bit(kind: u8) -> Option<(u32, u8, bool)> {
    match kind {
        TES_OBJECT_ARMO => Some((ARMOR_BIPED_FLAGS, 0x40, true)),
        TES_OBJECT_WEAP => Some((WEAPON_FLAGS1, 0x80, true)),
        TES_AMMO => Some((AMMO_FLAGS, 2, true)),
        TES_RACE => Some((RACE_FLAGS, 1, false)),
        TES_EYES => Some((EYES_FLAGS, 1, false)),
        TES_HAIR => Some((HAIR_FLAGS, 1, false)),
        _ => None,
    }
}

/// The form argument `i`, else the calling reference's base form; `None` with neither.
fn form_or_base(e: &Engine, a: &CommandArgs, read: &Read, i: usize) -> Option<u32> {
    match read.form(i) {
        0 if a.this_obj == 0 => None,
        0 => Some(base_of(e, a.this_obj)).filter(|&b| b != 0),
        f => Some(f),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_LNIsPlayable_Execute.
/// 1 for a playable armour, weapon, ammunition, perk, race, eyes or hair (by form type); 0 for
/// any other form.
fn ln_is_playable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(form) = form_or_base(e, &a, &read, 0) else {
        return handled(true);
    };
    let kind = type_of(e, form);
    let playable = if kind == BGS_PERK {
        e.mem.u8(form + PERK_PLAYABLE) != 0
    } else if let Some((field, bit, negative)) = playable_bit(kind) {
        // Each field is a byte or a word whose low byte holds the bit: read the byte.
        (e.mem.u8(form + field) & bit != 0) != negative
    } else {
        false
    };
    if playable {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_LNSetIsPlayable_Execute.
/// The playable flag set (`playable` not 0) or cleared for the form types `LNIsPlayable` reads;
/// a perk's byte becomes 1 or 0.
fn ln_set_is_playable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let on = read.int(0, 0) != 0;
    let Some(form) = form_or_base(e, &a, &read, 1) else {
        return handled(true);
    };
    let kind = type_of(e, form);
    if kind == BGS_PERK {
        e.mem.set_u8(form + PERK_PLAYABLE, u8::from(on));
    } else if let Some((field, bit, negative)) = playable_bit(kind) {
        let v = e.mem.u8(form + field);
        let set = on != negative;
        e.mem
            .set_u8(form + field, if set { v | bit } else { v & !bit });
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetZone_Execute.
/// The encounter zone of the calling reference's cell (its `ExtraEncounterZone`).
fn get_zone(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let cell = if a.this_obj == 0 {
        0
    } else {
        e.mem.u32(a.this_obj + REFR_CELL)
    };
    if cell != 0 {
        let x = extra::get_by_type(e, cell + CELL_EXTRA, EXTRA_ENCOUNTER_ZONE);
        if x != 0 {
            let zone = e.mem.u32(x + EXTRA_WORD);
            form_res(e, &a, zone);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_AddNoteNS_Execute.
/// The note put first in the player's notes when not there already (no message, no sound).
fn add_note_ns(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let note = read.form(0);
    let p = player(e);
    if read.ok && p != 0 && !in_list(e, p + PLAYER_NOTES, note) {
        list_prepend(e, Ptr::new(p + PLAYER_NOTES), note);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// CrosshairRefInList, Cmd_CrosshairRefInList_Execute.
/// 1 when the list holds the reference under the crosshair, 2 when it holds its base form (a
/// placeable water's water), whichever comes first; 0 with no reference or neither.
fn crosshair_ref_in_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let list = read.form(0);
    if !read.ok || list == 0 {
        return handled(true);
    }
    let manager = e.mem.u32(INTERFACE_MANAGER);
    let refr = if manager == 0 {
        0
    } else {
        e.mem.u32(manager + INTERFACE_CROSSHAIR_REF)
    };
    let mut found = 0.0;
    if refr != 0 {
        let mut base = base_of(e, refr);
        if of_type(e, base, BGS_PLACEABLE_WATER) {
            base = e.mem.u32(base + PLACEABLE_WATER);
        }
        for form in list_items(e, Ptr::new(list + LIST_FORM_LIST)) {
            if form == refr {
                found = 1.0;
                break;
            }
            if form == base {
                found = 2.0;
                break;
            }
        }
    }
    a.set_result(e, found);
    handled(true)
}

/// The water form argument and a trait id up to 50 (JIP's checks).
fn water_trait_args(e: &Engine, read: &Read) -> Option<(u32, u32)> {
    let water = read.form(0);
    let id = read.int(1, 0);
    (read.ok && of_type(e, water, TES_WATER_FORM) && id <= 50).then_some((water, id))
}

/// Trait `id` (2 to 50)'s shader value: `visData[id - 2]`.
fn vis_data(water: u32, id: u32) -> u32 {
    water + WATER_VIS_DATA + 4 * (id - 2)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetWaterTrait_Execute.
/// Trait 0 the opacity, 1 the flags, 12 to 14 a colour as decimal `RRRGGGBBB`, any other up to
/// 50 the shader value as a float.
fn get_water_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some((water, id)) = water_trait_args(e, &read) else {
        return handled(true);
    };
    let value = match id {
        0 => f64::from(e.mem.u8(water + WATER_OPACITY)),
        1 => f64::from(e.mem.u8(water + WATER_FLAGS)),
        12..=14 => f64::from(rgb_hex_to_dec(e.mem.u32(vis_data(water, id)))),
        _ => f64::from(e.mem.f32(vis_data(water, id))),
    };
    a.set_result(e, value);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_SetWaterTrait_Execute.
/// The value (as `(int)value`, unsigned for the range checks) set: the opacity up to 100, the
/// flags up to 3, a colour up to 255255255; any other trait the float.
fn set_water_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some((water, id)) = water_trait_args(e, &read) else {
        return handled(true);
    };
    let value = double_arg(&read, 2, 0.0);
    let int = cvttsd2si(value) as u32;
    match id {
        0 => {
            if int <= 100 {
                e.mem.set_u8(water + WATER_OPACITY, int as u8);
            }
        }
        1 => {
            if int <= 3 {
                e.mem.set_u8(water + WATER_FLAGS, int as u8);
            }
        }
        12..=14 => {
            if int <= 255_255_255 {
                e.mem.set_u32(vis_data(water, id), rgb_dec_to_hex(int));
            }
        }
        _ => e.mem.set_f32(vis_data(water, id), value as f32),
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_miscellaneous.h:
// Cmd_GetActiveQuest_Execute.
fn get_active_quest(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = player(e);
    if p != 0 {
        let quest = e.mem.u32(p + PLAYER_ACTIVE_QUEST);
        form_res(e, &a, quest);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_miscellaneous_tests.rs"]
mod tests;
