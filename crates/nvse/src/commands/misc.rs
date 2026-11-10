//! xNVSE's `Commands_Misc.cpp` (xNVSE), first batch: age class, respawn and permanent flags, race
//! and class, perk ranks, and the commands that turn a form's lists into arrays (a race's hairs and
//! eyes, an actor base's spells, packages and factions, an actor's active factions, a faction's rank
//! names, an NPC's head parts, a leveled list's entries, a form list's forms).
//!
//! Read from xNVSE's source and checked command by command in the shipped DLL (xNVSE 6.4.9's
//! `nvse_1_4.dll`, its command records found by their names; the addresses below are the DLL's):
//! the casts, the offsets and the order of the reads are the DLL's.

use super::{
    console_print, double_words, dynamic_cast, is_console_mode, rtti, try_get_refr_parent, Read,
};
use crate::table::{Command, Param, Returns};
use crate::vars::{self, Key, Value};
use crate::{handled, CommandArgs, EvalArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_Integer`, `_ObjectRef`, `_Actor`,
/// `_Race`, `_Faction`, `_FormList`, `_Perk`, `_AnyForm`.
const INT: u32 = 0x01;
const OBJECT_REF: u32 = 0x04;
const ACTOR: u32 = 0x06;
const RACE: u32 = 0x0f;
const FACTION: u32 = 0x11;
const FORM_LIST: u32 = 0x25;
const PERK: u32 = 0x27;
const ANY_FORM: u32 = 0x3d;
/// `kParams_OneOptionalActor`, `kParams_OneIntOneOptionalActor`, `kParams_OneOptionalObjectRef`,
/// `kParams_OneIntOneOptionalObjectRef`, `kParams_OneOptionalForm`, `kParams_OnePerkOneOptionalActor`,
/// `kParams_OneRace`, `kParams_OneFaction`, `kParams_FormList` (xNVSE).
const OPTIONAL_ACTOR: &[Param] = &[p("actor", ACTOR, true)];
const INT_OPTIONAL_ACTOR: &[Param] = &[p("flag", INT, false), p("actor", ACTOR, true)];
const OPTIONAL_OBJECT_REF: &[Param] = &[p("ref", OBJECT_REF, true)];
const INT_OPTIONAL_OBJECT_REF: &[Param] = &[p("flag", INT, false), p("ref", OBJECT_REF, true)];
const OPTIONAL_FORM: &[Param] = &[p("form", ANY_FORM, true)];
const PERK_OPTIONAL_ACTOR: &[Param] = &[p("perk", PERK, false), p("actor", ACTOR, true)];
const ONE_RACE: &[Param] = &[p("race", RACE, false)];
const ONE_FACTION: &[Param] = &[p("faction", FACTION, false)];
const ONE_FORM_LIST: &[Param] = &[p("form list", FORM_LIST, false)];

/// `TESForm`: `flags` (`+8`, `kFormFlags_QuestItem` 0x400), `refID` (`+0xC`), `MarkAsModified` (the
/// `+0x48` virtual function; `kModified_Flags` 1). `TESObjectREFR::baseForm` (`+0x20`),
/// `extraDataList` (`+0x44`). `TESActorBaseData::flags` (`+4`, `kFlags_Respawn` 8) and its faction
/// list (`+0x2C`: each entry's faction `+0` and rank byte `+4`). `TESActorBase`: its spell lists
/// (`+0x80` spells, `+0x88` leveled spells), packages (`+0xA8`), faction list (`+0x5C`).
/// `TESNPC`: race (`+0x110`), class (`+0x130`), head parts (`+0x1DC`). `TESRace`: name (`+0x1C`),
/// flags (`+0x70`, `kFlag_Child` 4), hairs (`+0x8C`), eyes (`+0xA8`). `TESClass`'s name (`+0x1C`).
/// `TESFaction`'s ranks (`+0x3C`: each a name `+0` and a female name `+8`). A leveled list's
/// entries (`+0x34`: each a form `+0`). `BGSListForm`'s forms (`+0x18`). Each as the DLL reads
/// it; the actor base data's faction list `+0x2C` is also the exe's (`005D8A70` answers
/// `this + 0x2C`).
const FORM_FLAGS: u32 = 8;
const FORM_ID: u32 = 0xc;
const QUEST_ITEM: u32 = 0x400;
const MARK_AS_MODIFIED: u32 = 0x48;
const MODIFIED_FLAGS: u32 = 1;
const REFR_BASE_FORM: u32 = 0x20;
const REFR_EXTRA: u32 = crate::invref::REFR_EXTRA;
const BASE_DATA_FLAGS: u32 = 4;
const RESPAWN: u32 = 8;
const ACTOR_BASE_SPELLS: u32 = 0x80;
const ACTOR_BASE_LEVELED_SPELLS: u32 = 0x88;
const ACTOR_BASE_PACKAGES: u32 = 0xa8;
const ACTOR_BASE_FACTIONS: u32 = 0x5c;
const NPC_RACE: u32 = 0x110;
const NPC_CLASS: u32 = 0x130;
const NPC_HEAD_PARTS: u32 = 0x1dc;
const RACE_NAME: u32 = 0x1c;
const RACE_FLAGS: u32 = 0x70;
const RACE_CHILD: u32 = 4;
const RACE_HAIRS: u32 = 0x8c;
const RACE_EYES: u32 = 0xa8;
const CLASS_NAME: u32 = 0x1c;
const FACTION_RANKS: u32 = 0x3c;
const RANK_FEMALE_NAME: u32 = 8;
const LEVELED_ENTRIES: u32 = 0x34;
const LIST_FORMS: u32 = 0x18;
/// `ExtraFactionChanges` (type 0x5E, its list `+0xC`).
const EXTRA_FACTION_CHANGES: u32 = 0x5e;
/// `Actor::GetPerkRank(perk, alt)`, the `+0x4A0` virtual function, its answer a byte:
/// `PlayerCharacter`'s vtable `0108AA3C` holds `009639E0` there (the rank from the perk list at
/// `+0x87C`, or `+0xAD4` for the alternate ranks), `Character`'s `01086A6C` and `Creature`'s
/// `010870AC` hold `008D0300`, which answers 0 (Ghidra).
const GET_PERK_RANK: u32 = 0x4a0;
/// `game_toupper` (xNVSE): the game's `toupper` (`00ECA7F4`).
const GAME_TOUPPER: u32 = 0x00ec_a7f4;
/// `GetAgeClass`'s `char upperName[256]` (xNVSE).
const NAME_BUFFER: usize = 256;

const fn cmd(
    opcode: u32,
    name: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    eval: Option<AbiFn>,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short: "",
        help,
        needs_ref: false,
        params,
        execute,
        eval,
        returns,
    }
}

use Returns::{Array as RA, Default as RD, Form as RF, String as RS};

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x14fe, "GetAgeClass", "Return NPC age class. From 0=Child -default- to 3=OldAged. -1 for non NPC. Based on race fullname.", OPTIONAL_ACTOR, get_age_class, Some(get_age_class_eval), RD),
    cmd(0x150c, "GetRespawn", "Return if the respawn flag is set on a base form", OPTIONAL_ACTOR, get_respawn, Some(get_respawn_eval), RD),
    cmd(0x150d, "SetRespawn", "sets the respawn flag on a base form", INT_OPTIONAL_ACTOR, set_respawn, None, RD),
    cmd(0x150e, "GetPermanent", "Return if the permanent flag is set on a base form", OPTIONAL_OBJECT_REF, get_permanent, Some(get_permanent_eval), RD),
    cmd(0x150f, "SetPermanent", "sets the permanent flag on a base form", INT_OPTIONAL_OBJECT_REF, set_permanent, None, RD),
    cmd(0x159c, "GetRace", "Return NPC race", OPTIONAL_FORM, get_race, None, RF),
    cmd(0x159d, "GetRaceName", "Return the name of the NPC race", OPTIONAL_FORM, get_race_name, None, RS),
    cmd(0x161c, "GetClass", "Return NPC class", OPTIONAL_FORM, get_class, None, RF),
    cmd(0x161d, "GetNameOfClass", "Return the name of the NPC class", OPTIONAL_FORM, get_name_of_class, None, RS),
    cmd(0x15c3, "GetPerkRank", "Return actor Perk rank or -1 if the perk is not applied", PERK_OPTIONAL_ACTOR, get_perk_rank, Some(get_perk_rank_eval), RD),
    cmd(0x15c4, "GetAltPerkRank", "Return actor alternate Perk rank or -1 if the perk is not applied", PERK_OPTIONAL_ACTOR, get_alt_perk_rank, Some(get_alt_perk_rank_eval), RD),
    cmd(0x15cd, "GetRaceHairs", "Return the array of hair for a race", ONE_RACE, get_race_hairs, None, RA),
    cmd(0x15ce, "GetRaceEyes", "Return the array of eyes for a race", ONE_RACE, get_race_eyes, None, RA),
    cmd(0x15cf, "GetBaseSpellListSpells", "Return the array of spells for a SpellList", OPTIONAL_FORM, get_base_spell_list_spells, None, RA),
    cmd(0x15d0, "GetBaseSpellListLevSpells", "Return the array of leveled spells for a SpellList", OPTIONAL_FORM, get_base_spell_list_lev_spells, None, RA),
    cmd(0x15d1, "GetBasePackages", "returns an array of packages for an actor base form", OPTIONAL_FORM, get_base_packages, None, RA),
    cmd(0x15d2, "GetBaseFactions", "returns an array of factions to which an actor base form belongs", OPTIONAL_FORM, get_base_factions, None, RA),
    cmd(0x15d3, "GetBaseRanks", "returns an array of faction ranks to which an actor base form belongs", OPTIONAL_FORM, get_base_ranks, None, RA),
    cmd(0x15d4, "GetActiveFactions", "returns the active factions on an actor", OPTIONAL_ACTOR, get_active_factions, None, RA),
    cmd(0x15d5, "GetActiveRanks", "returns the active faction ranks on an actor", OPTIONAL_ACTOR, get_active_ranks, None, RA),
    cmd(0x15d6, "GetFactionRankNames", "returns an array of name of faction ranks", ONE_FACTION, get_faction_rank_names, None, RA),
    cmd(0x15d7, "GetFactionRankFemaleNames", "returns an array of the female name of faction ranks", ONE_FACTION, get_faction_rank_female_names, None, RA),
    cmd(0x15d8, "GetHeadParts", "returns an array of head parts for a character", OPTIONAL_FORM, get_head_parts, None, RA),
    cmd(0x15d9, "GetLevCreatureRefs", "returns an array of ref in a leveled creature leveled list", OPTIONAL_FORM, get_lev_creature_refs, None, RA),
    cmd(0x15da, "GetLevCharacterRefs", "returns an array of ref in a leveled character leveled list", OPTIONAL_FORM, get_lev_character_refs, None, RA),
    cmd(0x15db, "GetListForms", "returns an array of forms in a form list", ONE_FORM_LIST, get_list_forms, None, RA),
];

fn form_id(e: &Engine, form: u32) -> u32 {
    e.mem.u32(form + FORM_ID)
}

fn set(e: &mut Engine, result: u32, v: f64) {
    e.mem.set_f64(result, v);
}

fn base_form(e: &Engine, refr: u32) -> u32 {
    e.mem.u32(refr + REFR_BASE_FORM)
}

/// `DYNAMIC_CAST(x, TESObjectREFR, Actor)`.
fn actor_of_ref(e: &mut Engine, refr: u32) -> u32 {
    dynamic_cast(e, refr, rtti::TES_OBJECT_REFR, rtti::ACTOR)
}

/// `DYNAMIC_CAST(x, TESForm, Actor)`.
fn actor_of_form(e: &mut Engine, form: u32) -> u32 {
    dynamic_cast(e, form, rtti::TES_FORM, rtti::ACTOR)
}

/// The items of a game `tList` from its head node, in order, empty nodes passed over
/// (`tList::Iterator` from `Begin()` to `End()`; a null list is at its end at once).
pub(crate) fn tlist_items(e: &Engine, head: u32) -> Vec<u32> {
    let mut items = Vec::new();
    let mut node = head;
    while !(node == 0 || (e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0)) {
        let item = e.mem.u32(node);
        if item != 0 {
            items.push(item);
        }
        node = e.mem.u32(node + 4);
    }
    items
}

/// A new array for the script (`g_ArrayMap.Create(kDataType_Numeric, true, modIndex)`, the mod
/// index the script's id's high byte), its id the result. Every list command makes it before it
/// reads its arguments, as the DLL does.
fn new_array(e: &mut Engine, a: &CommandArgs) -> u32 {
    let modi = vars::script_mod_index(e, a.script);
    let array = vars::create_array(e, vars::KEY_NUMBER, true, modi);
    a.set_result(e, f64::from(array));
    array
}

/// The values put in `array` from index 0 (`SetElement*` with a double index counted up by 1).
fn fill(e: &mut Engine, array: u32, values: Vec<Value>) {
    for (i, v) in values.into_iter().enumerate() {
        vars::set_element(e, array, &Key::Number(i as f64), v);
    }
}

fn form_values(e: &Engine, forms: Vec<u32>) -> Vec<Value> {
    forms
        .into_iter()
        .map(|f| Value::Form(form_id(e, f)))
        .collect()
}

/// A `String`'s text at `at` (`String::CStr`: "" without a text pointer or with a length of 0,
/// as the DLL tests both: `m_data` `+0`, `m_dataLen` `+4`, e.g. at `100337ED`).
fn cstr(e: &Engine, at: u32) -> Vec<u8> {
    let data = e.mem.u32(at);
    if data == 0 || e.mem.u16(at + 4) == 0 {
        Vec::new()
    } else {
        crate::table::guest_text(e, data)
    }
}

// ---- age, respawn, permanent ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetAgeClass_Eval,
// read against the shipped DLL (`nvse_1_4.dll`, the eval at `10033070`).
/// 0 for a child race (its flag, or "CHILD" in its name in capitals), 3 for "OLDAGED", 2 for "OLD",
/// else 1 (also for a race whose name has no text pointer); -1 without an NPC with a race. The
/// actor is the argument, else the calling reference.
///
/// Each character goes through the game's `toupper` (`00ECA7F4`) sign-extended, as the DLL passes
/// it (`movsx`). xNVSE copies the name into a 256-byte buffer on the stack, its terminator
/// included: a name of 256 characters or more (a mod's `SetName` on the race) writes past it into
/// the saved registers and the return address. With the script list and scan guard row
/// (`NV_NVSE_LIST_GUARD`) the whole name is looked at; without it the port stops the game there.
fn age_class(e: &mut Engine, this_obj: u32, arg1: u32) -> f64 {
    let mut actor = if arg1 != 0 { actor_of_ref(e, arg1) } else { 0 };
    if actor == 0 && this_obj != 0 {
        actor = actor_of_ref(e, this_obj);
    }
    let npc = if actor != 0 {
        let base = base_form(e, actor);
        dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_NPC)
    } else {
        0
    };
    let race = if npc != 0 {
        e.mem.u32(npc + NPC_RACE)
    } else {
        0
    };
    if race == 0 {
        return -1.0;
    }
    if e.mem.u32(race + RACE_FLAGS) & RACE_CHILD != 0 {
        return 0.0;
    }
    let name = e.mem.u32(race + RACE_NAME);
    if name == 0 {
        return 1.0;
    }
    let text = crate::table::guest_text(e, name);
    if text.len() >= NAME_BUFFER && !super::array::fix_on(e, super::array::LIST_GUARD) {
        panic!(
            "GetAgeClass: a race name of {} characters overflows xNVSE's 256-byte buffer and the game \
             stops (NV_NVSE_LIST_GUARD reads it whole)",
            text.len()
        );
    }
    // `i <= strlen(name)`: the terminator goes through `toupper` too; `strstr` reads the buffer
    // up to its first 0.
    let mut upper: Vec<u8> = text
        .iter()
        .chain(&[0])
        .map(|&c| {
            let c = i32::from(c as i8) as u32;
            e.call(GAME_TOUPPER, &args![c]).u32() as u8
        })
        .collect();
    if let Some(end) = upper.iter().position(|&c| c == 0) {
        upper.truncate(end);
    }
    let has = |s: &[u8]| upper.windows(s.len()).any(|w| w == s);
    if has(b"CHILD") {
        0.0
    } else if has(b"OLDAGED") {
        3.0
    } else if has(b"OLD") {
        2.0
    } else {
        1.0
    }
}

fn get_age_class_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    set(e, a.result, 0.0);
    let v = age_class(e, a.this_obj, a.arg1);
    set(e, a.result, v);
    if is_console_mode(e) {
        console_print(e, "GetAgeClass: %.0f", &double_words(v));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetAgeClass_Execute,
// Cmd_GetRespawn_Execute, Cmd_GetPermanent_Execute.
/// The eval with the argument read; whether the read failed is not looked at (no argument).
fn with_arg(e: &mut Engine, w: &[u32], eval: fn(&mut Engine, &[u32]) -> Ret) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let arg1 = read.form(0);
    eval(e, &[a.this_obj, arg1, 0, a.result])
}

fn get_age_class(e: &mut Engine, w: &[u32]) -> Ret {
    with_arg(e, w, get_age_class_eval)
}

/// The actor base data of the actor's base form (`DYNAMIC_CAST(baseForm, TESForm,
/// TESActorBaseData)`), 0 without.
fn base_data_of(e: &mut Engine, actor: u32) -> u32 {
    if actor == 0 {
        return 0;
    }
    let base = base_form(e, actor);
    dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_ACTOR_BASE_DATA)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetRespawn_Eval.
/// Whether the actor's base has the respawn flag; the actor is the argument (cast from a form),
/// else the calling reference.
fn get_respawn_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    set(e, a.result, 0.0);
    let mut actor = if a.arg1 != 0 {
        actor_of_form(e, a.arg1)
    } else {
        0
    };
    if actor == 0 && a.this_obj != 0 {
        actor = actor_of_ref(e, a.this_obj);
    }
    let data = base_data_of(e, actor);
    if data != 0 {
        let v = e.mem.u32(data + BASE_DATA_FLAGS) & RESPAWN != 0;
        set(e, a.result, f64::from(u8::from(v)));
    }
    if is_console_mode(e) {
        let v = e.mem.f64(a.result);
        console_print(e, "GetRespawn: %.0f", &double_words(v));
    }
    handled(true)
}

fn get_respawn(e: &mut Engine, w: &[u32]) -> Ret {
    with_arg(e, w, get_respawn_eval)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_SetRespawn_Execute.
/// Sets or clears the respawn flag of the actor's base (the actor given, else the calling
/// reference). The flag is read as an integer into the low word of a double that starts at 0, so
/// any integer but 0 sets it.
fn set_respawn(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let flag = read.int(0, 0) != 0;
    let mut actor = read.form(1);
    if actor == 0 && a.this_obj != 0 {
        actor = actor_of_ref(e, a.this_obj);
    }
    let data = base_data_of(e, actor);
    if data != 0 {
        let flags = e.mem.u32(data + BASE_DATA_FLAGS);
        let flags = if flag {
            flags | RESPAWN
        } else {
            flags & !RESPAWN
        };
        e.mem.set_u32(data + BASE_DATA_FLAGS, flags);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetPermanent_Eval.
/// Whether the reference's BASE FORM has the quest item (persistent) flag; the reference is the
/// argument (cast from a form), else the calling reference.
fn get_permanent_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    set(e, a.result, 0.0);
    let mut obj = if a.arg1 != 0 {
        dynamic_cast(e, a.arg1, rtti::TES_FORM, rtti::TES_OBJECT_REFR)
    } else {
        0
    };
    if obj == 0 {
        obj = a.this_obj;
    }
    if obj != 0 {
        let base = base_form(e, obj);
        if base != 0 {
            let v = e.mem.u32(base + FORM_FLAGS) & QUEST_ITEM != 0;
            set(e, a.result, f64::from(u8::from(v)));
        }
    }
    if is_console_mode(e) {
        let v = e.mem.f64(a.result);
        console_print(e, "GetPermanent: %.0f", &double_words(v));
    }
    handled(true)
}

fn get_permanent(e: &mut Engine, w: &[u32]) -> Ret {
    with_arg(e, w, get_permanent_eval)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_SetPermanent_Execute.
/// Sets or clears the quest item flag of the REFERENCE itself (the one given, else the calling
/// reference; `GetPermanent` reads its base form's), marked as changed (`MarkAsModified` with
/// `kModified_Flags`) so the save keeps it.
fn set_permanent(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let flag = read.int(0, 0) != 0;
    let mut obj = read.form(1);
    if obj == 0 {
        obj = a.this_obj;
    }
    if obj != 0 {
        let flags = e.mem.u32(obj + FORM_FLAGS);
        let flags = if flag {
            flags | QUEST_ITEM
        } else {
            flags & !QUEST_ITEM
        };
        e.mem.set_u32(obj + FORM_FLAGS, flags);
        e.vcall(obj, MARK_AS_MODIFIED, &args![MODIFIED_FLAGS]);
    }
    handled(true)
}

// ---- race and class ----

/// The NPC a race or class command reads: the given form (a reference's base form), else the
/// calling reference's base form, as a `TESNPC`; `None` when the read failed.
fn npc_arg(e: &mut Engine, a: &CommandArgs) -> Option<u32> {
    let read = Read::of(e, a);
    if !read.ok {
        return None;
    }
    let mut form = try_get_refr_parent(e, read.form(0));
    if form == 0 && a.this_obj != 0 {
        form = base_form(e, a.this_obj);
    }
    Some(if form != 0 {
        dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_NPC)
    } else {
        0
    })
}

/// The NPC's form at `at` (its race or class), 0 without.
fn npc_field(e: &mut Engine, a: &CommandArgs, at: u32) -> Option<u32> {
    let npc = npc_arg(e, a)?;
    Some(if npc != 0 { e.mem.u32(npc + at) } else { 0 })
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetRace_Execute,
// Cmd_GetClass_Execute.
fn npc_form_result(e: &mut Engine, w: &[u32], at: u32) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if let Some(form) = npc_field(e, &a, at).filter(|&f| f != 0) {
        let id = form_id(e, form);
        a.set_form_result(e, id);
    }
    handled(true)
}

fn get_race(e: &mut Engine, w: &[u32]) -> Ret {
    npc_form_result(e, w, NPC_RACE)
}

fn get_class(e: &mut Engine, w: &[u32]) -> Ret {
    npc_form_result(e, w, NPC_CLASS)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetRaceName_Execute,
// Cmd_GetNameOfClass_Execute.
/// The race's or class's name as a string; no string is made (the result stays 0) without one.
fn npc_name_result(e: &mut Engine, w: &[u32], at: u32, name: u32) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if let Some(form) = npc_field(e, &a, at).filter(|&f| f != 0) {
        let text = cstr(e, form + name);
        super::string::assign_to_string_var(e, &a, &text);
    }
    handled(true)
}

fn get_race_name(e: &mut Engine, w: &[u32]) -> Ret {
    npc_name_result(e, w, NPC_RACE, RACE_NAME)
}

fn get_name_of_class(e: &mut Engine, w: &[u32]) -> Ret {
    npc_name_result(e, w, NPC_CLASS, CLASS_NAME)
}

// ---- perks ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetPerkRank.
/// The actor's rank in the perk (the actor's own `GetPerkRank`, a byte, `movzx` at `10033A72`);
/// -1 without a perk or an actor. The actor is the second argument, else the calling reference.
/// Without a perk the console prints nothing (the DLL returns before its print, `10033A1D`).
fn perk_rank(e: &mut Engine, w: &[u32], alt: bool) -> Ret {
    let a = EvalArgs::new(w);
    set(e, a.result, -1.0);
    let perk = if a.arg1 != 0 {
        dynamic_cast(e, a.arg1, rtti::TES_FORM, rtti::BGS_PERK)
    } else {
        0
    };
    if perk == 0 {
        return handled(true);
    }
    let mut actor = if a.arg2 != 0 {
        actor_of_form(e, a.arg2)
    } else {
        0
    };
    if actor == 0 && a.this_obj != 0 {
        actor = actor_of_ref(e, a.this_obj);
    }
    if actor != 0 {
        let rank = e
            .vcall(actor, GET_PERK_RANK, &args![perk, u32::from(alt)])
            .u32()
            & 0xff;
        set(e, a.result, f64::from(rank));
    }
    if is_console_mode(e) {
        let v = e.mem.f64(a.result);
        let format = if alt {
            "GetAltPerkRank: %.0f"
        } else {
            "GetPerkRank: %.0f"
        };
        console_print(e, format, &double_words(v));
    }
    handled(true)
}

fn get_perk_rank_eval(e: &mut Engine, w: &[u32]) -> Ret {
    perk_rank(e, w, false)
}

fn get_alt_perk_rank_eval(e: &mut Engine, w: &[u32]) -> Ret {
    perk_rank(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetPerkRank_Execute,
// Cmd_GetAltPerkRank_Execute.
/// The eval with the perk and actor read; a failed read leaves the result as it was.
fn perk_rank_execute(e: &mut Engine, w: &[u32], alt: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    perk_rank(e, &[a.this_obj, read.form(0), read.form(1), a.result], alt)
}

fn get_perk_rank(e: &mut Engine, w: &[u32]) -> Ret {
    perk_rank_execute(e, w, false)
}

fn get_alt_perk_rank(e: &mut Engine, w: &[u32]) -> Ret {
    perk_rank_execute(e, w, true)
}

// ---- lists to arrays ----

/// `ConvertActorBase` (xNVSE): the form given (a reference's base form), else the calling
/// reference's base form, as a `TESActorBase`. `ExtractActorBase` reads it without looking at the
/// read's answer.
fn actor_base_arg(e: &mut Engine, a: &CommandArgs) -> u32 {
    let read = Read::of(e, a);
    convert(e, a.this_obj, read.form(0), rtti::TES_ACTOR_BASE)
}

/// `ConvertNPC`, `ConvertActorBase`, `ConvertLevCreature`, `ConvertLevCharacter` (xNVSE): the form
/// (else the calling reference's base form), a reference's base form for a reference, cast to
/// `to`.
pub(super) fn convert(e: &mut Engine, this_obj: u32, form: u32, to: u32) -> u32 {
    let mut form = form;
    if form == 0 && this_obj != 0 {
        form = base_form(e, this_obj);
    }
    let form = try_get_refr_parent(e, form);
    dynamic_cast(e, form, rtti::TES_FORM, to)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetRaceHairs_Execute,
// Cmd_GetRaceEyes_Execute.
/// A race's hairs or eyes; nothing listed after a failed read.
fn race_list(e: &mut Engine, w: &[u32], at: u32) -> Ret {
    let a = CommandArgs::new(w);
    let array = new_array(e, &a);
    let read = Read::of(e, &a);
    let race = read.form(0);
    if read.ok && race != 0 {
        let forms = tlist_items(e, race + at);
        let values = form_values(e, forms);
        fill(e, array, values);
    }
    handled(true)
}

fn get_race_hairs(e: &mut Engine, w: &[u32]) -> Ret {
    race_list(e, w, RACE_HAIRS)
}

fn get_race_eyes(e: &mut Engine, w: &[u32]) -> Ret {
    race_list(e, w, RACE_EYES)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp:
// Cmd_GetBaseSpellListSpells_Execute, Cmd_GetBaseSpellListLevSpells_Execute,
// Cmd_GetBasePackages_Execute.
/// The forms of one of an actor base's lists (the array made before the read).
fn actor_base_list(e: &mut Engine, w: &[u32], at: u32) -> Ret {
    let a = CommandArgs::new(w);
    let array = new_array(e, &a);
    let base = actor_base_arg(e, &a);
    if base != 0 {
        let forms = tlist_items(e, base + at);
        let values = form_values(e, forms);
        fill(e, array, values);
    }
    handled(true)
}

fn get_base_spell_list_spells(e: &mut Engine, w: &[u32]) -> Ret {
    actor_base_list(e, w, ACTOR_BASE_SPELLS)
}

fn get_base_spell_list_lev_spells(e: &mut Engine, w: &[u32]) -> Ret {
    actor_base_list(e, w, ACTOR_BASE_LEVELED_SPELLS)
}

fn get_base_packages(e: &mut Engine, w: &[u32]) -> Ret {
    actor_base_list(e, w, ACTOR_BASE_PACKAGES)
}

/// A faction list's entries with a faction, as (faction, rank byte).
fn factions(e: &Engine, head: u32) -> Vec<(u32, u8)> {
    tlist_items(e, head)
        .into_iter()
        .filter_map(|d| {
            let faction = e.mem.u32(d);
            (faction != 0).then(|| (faction, e.mem.u8(d + 4)))
        })
        .collect()
}

fn faction_value(e: &Engine, (faction, rank): (u32, u8), ranks: bool) -> Value {
    if ranks {
        Value::Number(f64::from(rank))
    } else {
        Value::Form(form_id(e, faction))
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetBaseFactions_Execute,
// Cmd_GetBaseRanks_Execute.
/// An actor base's factions, or their ranks: the rank byte read unsigned (`movzx` at `10034353`),
/// 0 to 255.
fn base_factions(e: &mut Engine, w: &[u32], ranks: bool) -> Ret {
    let a = CommandArgs::new(w);
    let array = new_array(e, &a);
    let base = actor_base_arg(e, &a);
    if base != 0 {
        let values = factions(e, base + ACTOR_BASE_FACTIONS)
            .into_iter()
            .map(|f| faction_value(e, f, ranks))
            .collect();
        fill(e, array, values);
    }
    handled(true)
}

fn get_base_factions(e: &mut Engine, w: &[u32]) -> Ret {
    base_factions(e, w, false)
}

fn get_base_ranks(e: &mut Engine, w: &[u32]) -> Ret {
    base_factions(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp:
// Cmd_GetActiveFactions_Execute, Cmd_GetActiveRanks_Execute; GameExtraData.cpp: GetExtraFactionList.
/// An actor's factions (or ranks): its base's, but those its own `ExtraFactionChanges` lists
/// (compared by id), then those of its changes. xNVSE means to leave out a change with a negative
/// rank ("removed from faction"), but compares the rank byte, which is unsigned, with 0: the
/// compiler drops the test (the DLL's second loop, `10034C15`, has none), so every change is
/// listed, a rank of -1 as 255. An actor without changes lists its base's (a missing list is an
/// ended walk). The read's answer is not looked at (`ExtractActor`).
fn active_factions(e: &mut Engine, w: &[u32], ranks: bool) -> Ret {
    let a = CommandArgs::new(w);
    let array = new_array(e, &a);
    let read = Read::of(e, &a);
    let mut form = read.form(0);
    if form == 0 {
        form = a.this_obj;
    }
    let actor = if form != 0 { actor_of_form(e, form) } else { 0 };
    let base = convert(e, a.this_obj, actor, rtti::TES_ACTOR_BASE);
    if base == 0 || actor == 0 {
        return handled(true);
    }
    let changes = crate::invref::get_by_type(e, actor + REFR_EXTRA, EXTRA_FACTION_CHANGES);
    let entry = if changes != 0 {
        e.mem.u32(changes + 0xc)
    } else {
        0
    };
    let changed = factions(e, entry);
    let mut values = Vec::new();
    for f in factions(e, base + ACTOR_BASE_FACTIONS) {
        let id = form_id(e, f.0);
        if !changed.iter().any(|c| form_id(e, c.0) == id) {
            values.push(faction_value(e, f, ranks));
        }
    }
    for c in changed {
        values.push(faction_value(e, c, ranks));
    }
    fill(e, array, values);
    handled(true)
}

fn get_active_factions(e: &mut Engine, w: &[u32]) -> Ret {
    active_factions(e, w, false)
}

fn get_active_ranks(e: &mut Engine, w: &[u32]) -> Ret {
    active_factions(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp:
// Cmd_GetFactionRankNames_Execute, Cmd_GetFactionRankFemaleNames_Execute.
/// A faction's ranks' names (or female names) as strings, "" for a rank without one.
fn rank_names(e: &mut Engine, w: &[u32], female: bool) -> Ret {
    let a = CommandArgs::new(w);
    let array = new_array(e, &a);
    let read = Read::of(e, &a);
    let faction = read.form(0);
    if read.ok && faction != 0 {
        let at = if female { RANK_FEMALE_NAME } else { 0 };
        let names = tlist_items(e, faction + FACTION_RANKS)
            .into_iter()
            .map(|r| Value::Str(cstr(e, r + at)))
            .collect();
        fill(e, array, names);
    }
    handled(true)
}

fn get_faction_rank_names(e: &mut Engine, w: &[u32]) -> Ret {
    rank_names(e, w, false)
}

fn get_faction_rank_female_names(e: &mut Engine, w: &[u32]) -> Ret {
    rank_names(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetHeadParts_Execute.
/// An NPC's head parts (`ExtractNPC`: the read's answer not looked at).
fn get_head_parts(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let array = new_array(e, &a);
    let read = Read::of(e, &a);
    let npc = convert(e, a.this_obj, read.form(0), rtti::TES_NPC);
    if npc != 0 {
        let parts = tlist_items(e, npc + NPC_HEAD_PARTS);
        let values = form_values(e, parts);
        fill(e, array, values);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp:
// Cmd_GetLevCreatureRefs_Execute, Cmd_GetLevCharacterRefs_Execute.
/// A leveled list's entries' forms. An entry without a form (a mod's broken list) is read
/// through for its id, as in xNVSE (`10034835` reads `+0xC` of it) and the game stops; the script
/// reference checks row passes over it.
fn leveled_refs(e: &mut Engine, w: &[u32], class: u32) -> Ret {
    let a = CommandArgs::new(w);
    let array = new_array(e, &a);
    let read = Read::of(e, &a);
    let list = convert(e, a.this_obj, read.form(0), class);
    if list != 0 {
        let mut i = 0.0;
        for entry in tlist_items(e, list + LEVELED_ENTRIES) {
            let form = e.mem.u32(entry);
            if super::misc_ref::passed_over(e, form) {
                continue;
            }
            let id = form_id(e, form);
            vars::set_element(e, array, &Key::Number(i), Value::Form(id));
            i += 1.0;
        }
    }
    handled(true)
}

fn get_lev_creature_refs(e: &mut Engine, w: &[u32]) -> Ret {
    leveled_refs(e, w, rtti::TES_LEV_CREATURE)
}

fn get_lev_character_refs(e: &mut Engine, w: &[u32]) -> Ret {
    leveled_refs(e, w, rtti::TES_LEV_CHARACTER)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetListForms_Execute.
fn get_list_forms(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let array = new_array(e, &a);
    let read = Read::of(e, &a);
    let list = read.form(0);
    if read.ok && list != 0 {
        let forms = tlist_items(e, list + LIST_FORMS);
        let values = form_values(e, forms);
        fill(e, array, values);
    }
    handled(true)
}

#[cfg(test)]
#[path = "misc_tests.rs"]
mod tests;
