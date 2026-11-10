//! xNVSE's `Commands_Misc.cpp` (xNVSE), second batch: the generic list commands (`GenericAddForm`
//! and the rest, on a race's hairs or eyes, an actor base's packages or spells, an NPC's head
//! parts, a form list), the default objects, the current quest objective's doors, an actor base's
//! special idles, a door's sounds and `FireChallenge`.
//!
//! Read from xNVSE's source and checked in the shipped DLL (xNVSE 6.4.9's `nvse_1_4.dll`;
//! `GenericForm_Execute` at `10034E70`, its `tList::AddAt` at `10032180` and `RemoveNth` at
//! `10032100`, the rest inlined there): the casts, offsets, conversions and console lines are the
//! DLL's.

use super::{console_print, dynamic_cast, is_console_mode, rtti, try_get_refr_parent, Read};
use crate::table::{Command, Param, Returns};
use crate::vars::{self, Key, Value};
use crate::{handled, CommandArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`, `ScriptUtils.h`): `kParamType_String`, `_Integer`,
/// `_AnyForm`, `kNVSEParamType_Number`, `_Form`, `_FormOrNumber`.
const STRING: u32 = 0x00;
const INT: u32 = 0x01;
const ANY_FORM: u32 = 0x3d;
const NVSE_NUMBER: u32 = 0x8001;
const NVSE_FORM: u32 = 0x8008;
const NVSE_FORM_OR_NUMBER: u32 = 0x8009;
/// `kParams_GenericForm`, `kParams_GenericDeleteForm`, `kParams_GenericCheckForm`,
/// `kParams_OneInt`, `kParams_OneIntOneForm`, `kParams_OneIntOneOptionalForm`,
/// `kParams_OneStringOneOptionalForm`, `kParams_OneOptionalForm`, `kNVSEParams_GetDoorSound`,
/// `kNVSEParams_FireChallenge` (xNVSE).
const GENERIC_FORM: &[Param] = &[
    p("which", INT, false),
    p("containingForm", ANY_FORM, false),
    p("form", ANY_FORM, false),
    p("index", INT, false),
];
const GENERIC_DELETE_FORM: &[Param] = &[
    p("which", INT, false),
    p("containingForm", ANY_FORM, false),
    p("index", INT, false),
];
const GENERIC_CHECK_FORM: &[Param] = &[
    p("which", INT, false),
    p("containingForm", ANY_FORM, false),
    p("form", ANY_FORM, false),
];
const ONE_INT: &[Param] = &[p("int", INT, false)];
const INT_FORM: &[Param] = &[p("index", INT, false), p("form", ANY_FORM, false)];
const INT_OPTIONAL_FORM: &[Param] = &[p("index", INT, false), p("form", ANY_FORM, true)];
const STRING_OPTIONAL_FORM: &[Param] = &[p("index", STRING, false), p("form", ANY_FORM, true)];
const OPTIONAL_FORM: &[Param] = &[p("form", ANY_FORM, true)];
const GET_DOOR_SOUND: &[Param] = &[
    p("door form", NVSE_FORM, false),
    p("0 = open, 1 = close", NVSE_NUMBER, false),
];
const FIRE_CHALLENGE: &[Param] = &[
    p("Challenge type", NVSE_NUMBER, false),
    p("Count", NVSE_NUMBER, false),
    p("Weapon", NVSE_FORM_OR_NUMBER, false),
    p("Value 1", NVSE_NUMBER, false),
    p("Value 2", NVSE_NUMBER, false),
    p("Value 3", NVSE_NUMBER, false),
];

/// `TESForm::refID` (`+0xC`), `GetIsReference` (the `+0xF0` virtual function, as the DLL calls it:
/// `Character`'s vtable `01086A6C` holds `008D0360` there, which answers 1, and `0047C850`, which
/// answers 0, at `+0xF4`; `TESObjectWEAP`'s `0102C51C` holds `0047C850` at `+0xF0`).
/// `TESObjectREFR::baseForm` (`+0x20`). The lists: a race's hairs (`+0x8C`) and eyes (`+0xA8`), an
/// actor base's spells (`+0x80`), leveled spells (`+0x88`) and packages (`+0xA8`), an NPC's head
/// parts (`+0x1DC`), a form list's forms (`+0x18`); an actor base's special idles (`TESAnimation`
/// at `+0xC4`, its names `+0xC8`).
const FORM_ID: u32 = 0xc;
const IS_REFERENCE: u32 = 0xf0;
const REFR_BASE_FORM: u32 = 0x20;
const RACE_HAIRS: u32 = 0x8c;
const RACE_EYES: u32 = 0xa8;
const ACTOR_BASE_SPELLS: u32 = 0x80;
const ACTOR_BASE_LEVELED_SPELLS: u32 = 0x88;
const ACTOR_BASE_PACKAGES: u32 = 0xa8;
const NPC_HEAD_PARTS: u32 = 0x1dc;
const LIST_FORMS: u32 = 0x18;
const ANIMATION_NAMES: u32 = 0xc8;
/// `BGSDefaultObjectManager::GetSingleton` (`*0x011CA80C`), its forms (`+0x18`, 34 of them).
const DEFAULT_OBJECTS: u32 = 0x011c_a80c;
const DEFAULT_FORMS: u32 = 0x18;
const DEFAULT_OBJECT_MAX: u32 = 34;
/// `PlayerCharacter::GetSingleton` (`*0x011DEA3C`) and `GetCurrentQuestObjectiveTargets`
/// (`0x00952BA0`, `thiscall`: a `tList` of `BGSQuestObjective::Target`, each with its target
/// `+0xC` and its doors `+0x20`, a `BSSimpleArray` (data `+4`, size `+8`) of 0x10-byte links whose
/// door is at `+0`).
const PLAYER: u32 = 0x011d_ea3c;
const CURRENT_QUEST_TARGETS: u32 = 0x0095_2ba0;
const TARGET_REF: u32 = 0xc;
const TARGET_DOORS: u32 = 0x20;
const LINK_SIZE: u32 = 0x10;
/// `TESObjectDOOR`'s open and close sounds (`+0x78`, `+0x7C`, as the door's loader stores `SNAM`
/// and `ANAM`, `tesobjectdoor.rs`).
const DOOR_OPEN_SOUND: u32 = 0x78;
const DOOR_CLOSE_SOUND: u32 = 0x7c;
/// The challenge increment xNVSE calls (`0x005F5950`, `cdecl`: type, count, 0, weapon, three
/// values).
const INCREMENT_CHALLENGE: u32 = 0x005f_5950;
/// `AddAnimation`'s and `DelAnimation`'s `char name[256]` (xNVSE).
const NAME_BUFFER: usize = 256;
/// `FormHeap_Allocate`, `FormHeap_Free`; a list node is 8 bytes.
const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
const FORM_HEAP_FREE: u32 = 0x0040_1030;
const NODE_SIZE: u32 = 8;
/// `eListEnd`, `eListInvalid` (xNVSE `GameTypes.h`).
const LIST_END: i32 = -2;
const LIST_INVALID: i32 = -1;

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns,
    }
}

use Returns::{Array as RA, Default as RD, Form as RF, String as RS};

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x15dc, "GenericAddForm", "", "adds a form to a list of form of an object at index", GENERIC_FORM, generic_add_form, RD),
    cmd(0x15dd, "GenericReplaceForm", "", "change a form in a list of form of an object at index", GENERIC_FORM, generic_replace_form, RF),
    cmd(0x15de, "GenericDeleteForm", "", "delete a form from a list of form of an object at index", GENERIC_DELETE_FORM, generic_delete_form, RF),
    cmd(0x15e1, "GenericGetForm", "", "returns a form from a list of form of an object at index", GENERIC_DELETE_FORM, generic_get_form, RF),
    cmd(0x15ff, "GenericCheckForm", "", "checks if a form is in a list of form of an object", GENERIC_CHECK_FORM, generic_check_form, RD),
    cmd(0x15e3, "GetNthDefaultForm", "", "returns the Nth default form", ONE_INT, get_nth_default_form, RF),
    cmd(0x15e4, "SetNthDefaultForm", "", "sets the Nth default form", INT_FORM, set_nth_default_form, RD),
    cmd(0x15e5, "GetDefaultForms", "", "returns all default forms as an array", &[], get_default_forms, RA),
    cmd(0x1609, "GetCurrentQuestObjectiveTeleportLinks", "GCQOTL", "returns an array of array of the path to the current objective targets", &[], get_current_quest_objective_teleport_links, RA),
    cmd(0x1618, "GetNthAnimation", "", "returns the nth animation selected on an NPC or a creature", INT_OPTIONAL_FORM, get_nth_animation, RS),
    cmd(0x1619, "AddAnimation", "", "Add an animation to an NPC or a creature", STRING_OPTIONAL_FORM, add_animation, RD),
    cmd(0x161a, "DelAnimation", "", "Delete an animation from an NPC or a creature", STRING_OPTIONAL_FORM, del_animation, RD),
    cmd(0x161b, "DelAnimations", "", "Delete all animation from an NPC or a creature", OPTIONAL_FORM, del_animations, RD),
    cmd(0x1677, "GetDoorSound", "", "Get the open / close sound of a door", GET_DOOR_SOUND, get_door_sound, RF),
    cmd(0x1678, "FireChallenge", "", "Increment a challenge using specified params", FIRE_CHALLENGE, fire_challenge, RD),
];

fn form_id(e: &Engine, form: u32) -> u32 {
    e.mem.u32(form + FORM_ID)
}

// ---- the game's lists (`tList`, GameTypes.h (xNVSE)) ----

/// `tList::GetNthNode`: the node at `index` from the head, 0 past the end or for a negative index.
fn nth_node(e: &Engine, head: u32, index: i32) -> u32 {
    if index < 0 {
        return 0;
    }
    let (mut node, mut index) = (head, index);
    loop {
        if index == 0 {
            return node;
        }
        index -= 1;
        node = e.mem.u32(node + 4);
        if node == 0 {
            return 0;
        }
    }
}

/// `tList::GetLastNode`: the last node and its index.
fn last_node(e: &Engine, head: u32) -> (u32, i32) {
    let (mut node, mut index) = (head, 0);
    while e.mem.u32(node + 4) != 0 {
        node = e.mem.u32(node + 4);
        index += 1;
    }
    (node, index)
}

fn new_node(e: &mut Engine, data: u32, next: u32) -> u32 {
    let node = e.call(FORM_HEAP_ALLOCATE, &args![NODE_SIZE]).u32();
    e.mem.set_u32(node, data);
    e.mem.set_u32(node + 4, next);
    node
}

/// `ListNode::Insert`: `item` takes the node's place, its old item moves to a new node after it.
fn insert(e: &mut Engine, node: u32, item: u32) {
    let added = new_node(e, e.mem.u32(node), e.mem.u32(node + 4));
    e.mem.set_u32(node, item);
    e.mem.set_u32(node + 4, added);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTypes.h: tList::AddAt.
/// `item` put at `index` (0 the head, -2 the end, else before the node there); the index answered
/// is the one given, or for the end the index of the node it went after (or into, when the list
/// was empty); -1 for no item or no such node.
fn add_at(e: &mut Engine, head: u32, item: u32, index: i32) -> i32 {
    if item == 0 {
        return LIST_INVALID;
    }
    if index == 0 {
        if e.mem.u32(head) != 0 {
            insert(e, head, item);
        } else {
            e.mem.set_u32(head, item);
        }
        return 0;
    }
    if index == LIST_END {
        let (node, last) = last_node(e, head);
        if e.mem.u32(node) != 0 {
            let added = new_node(e, item, e.mem.u32(node + 4));
            e.mem.set_u32(node + 4, added);
        } else {
            e.mem.set_u32(node, item);
        }
        return last;
    }
    let node = nth_node(e, head, index);
    if node == 0 {
        return LIST_INVALID;
    }
    insert(e, node, item);
    index
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTypes.h: tList::ReplaceNth.
/// The item at `index` (-2 the last) replaced; the old item, 0 for none.
fn replace_nth(e: &mut Engine, head: u32, index: i32, item: u32) -> u32 {
    if item == 0 {
        return 0;
    }
    let node = if index == LIST_END {
        last_node(e, head).0
    } else {
        nth_node(e, head, index)
    };
    if node == 0 {
        return 0;
    }
    let old = e.mem.u32(node);
    e.mem.set_u32(node, item);
    old
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTypes.h: tList::RemoveNth,
// ListNode::RemoveMe, ListNode::RemoveNext.
/// The item at `idx` taken out (any index of 0 or less the head: -2 too), its node freed; 0 past
/// the end.
fn remove_nth(e: &mut Engine, head: u32, idx: i32) -> u32 {
    if idx <= 0 {
        let removed = e.mem.u32(head);
        let next = e.mem.u32(head + 4);
        if next != 0 {
            e.mem.set_u32(head, e.mem.u32(next));
            e.mem.set_u32(head + 4, e.mem.u32(next + 4));
            e.call(FORM_HEAP_FREE, &args![next]);
        } else {
            e.mem.set_u32(head, 0);
        }
        return removed;
    }
    let (mut node, mut idx) = (head, idx);
    while e.mem.u32(node + 4) != 0 {
        idx -= 1;
        if idx == 0 {
            break;
        }
        node = e.mem.u32(node + 4);
    }
    if idx != 0 {
        return 0;
    }
    let next = e.mem.u32(node + 4);
    let removed = e.mem.u32(next);
    e.mem.set_u32(node + 4, e.mem.u32(next + 4));
    e.call(FORM_HEAP_FREE, &args![next]);
    removed
}

/// `tList::GetNthItem`: the item at `index` (-2 the last), 0 for none.
fn nth_item(e: &Engine, head: u32, index: i32) -> u32 {
    if index == LIST_END {
        return e.mem.u32(last_node(e, head).0);
    }
    let node = nth_node(e, head, index);
    if node == 0 {
        0
    } else {
        e.mem.u32(node)
    }
}

/// `tList::Count`: 0 for an empty head, else every node.
fn count(e: &Engine, head: u32) -> u32 {
    if e.mem.u32(head) == 0 {
        return 0;
    }
    let (mut node, mut n) = (head, 1);
    while e.mem.u32(node + 4) != 0 {
        node = e.mem.u32(node + 4);
        n += 1;
    }
    n
}

// ---- the generic list commands ----

/// `EActionListForm` (xNVSE `GameForms.h`).
#[derive(Clone, Copy, PartialEq)]
enum Action {
    AddAt,
    DelAt,
    ChgAt,
    GetAt,
    Check,
}

/// `ConvertActorBase`, `ConvertNPC` (xNVSE `Commands_Misc.cpp`): the form (else the calling
/// reference's base form), a reference's base form for a reference, cast to `to`.
fn convert(e: &mut Engine, this_obj: u32, form: u32, to: u32) -> u32 {
    let mut form = form;
    if form == 0 && this_obj != 0 {
        form = e.mem.u32(this_obj + REFR_BASE_FORM);
    }
    let form = try_get_refr_parent(e, form);
    dynamic_cast(e, form, rtti::TES_FORM, to)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: GenericForm_Execute.
/// One of the generic list commands on the list `which` names of the containing form: 0 a race's
/// hairs, 1 its eyes, 5 an actor base's packages, 6 its spells, 7 its leveled spells, 10 an NPC's
/// head parts, 13 a form list's forms (the form to add, replace or find of the list's kind; a form
/// list takes any). Any other kind, or a failed read, makes the handler answer false. The result
/// starts at -1; a form answered goes into its low word only (over -1's bits, as xNVSE writes it).
/// The form list's own count of forms scripts added (`numAddedObjects`) is not kept.
fn generic(e: &mut Engine, w: &[u32], action: Action) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, f64::from(LIST_INVALID));
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(false);
    }
    let no_form = matches!(action, Action::DelAt | Action::GetAt);
    let which = read.int(0, 0);
    let owner = read.form(1);
    let (form, n) = match action {
        Action::DelAt | Action::GetAt => (0, read.int(2, LIST_END as u32) as i32),
        Action::Check => (read.form(2), LIST_END),
        _ => (read.form(2), read.int(3, LIST_END as u32) as i32),
    };
    let cast_form = |e: &mut Engine, to: u32| {
        if no_form {
            0
        } else {
            dynamic_cast(e, form, rtti::TES_FORM, to)
        }
    };
    let (holder, of_kind, at) = match which {
        0 | 1 => {
            let race = dynamic_cast(e, owner, rtti::TES_FORM, rtti::TES_RACE);
            let kind = cast_form(e, if which == 0 { rtti::TES_HAIR } else { rtti::TES_EYES });
            (race, kind, if which == 0 { RACE_HAIRS } else { RACE_EYES })
        }
        5..=7 => {
            let base = convert(e, a.this_obj, owner, rtti::TES_ACTOR_BASE);
            let kind = cast_form(e, if which == 5 { rtti::TES_PACKAGE } else { rtti::SPELL_ITEM });
            let at = match which {
                5 => ACTOR_BASE_PACKAGES,
                6 => ACTOR_BASE_SPELLS,
                _ => ACTOR_BASE_LEVELED_SPELLS,
            };
            (base, kind, at)
        }
        10 => {
            let npc = convert(e, a.this_obj, owner, rtti::TES_NPC);
            (npc, cast_form(e, rtti::BGS_HEAD_PART), NPC_HEAD_PARTS)
        }
        13 => {
            let list = dynamic_cast(e, owner, rtti::TES_FORM, rtti::BGS_LIST_FORM);
            (list, form, LIST_FORMS)
        }
        _ => return handled(false),
    };
    if holder == 0 || !(of_kind != 0 || no_form) || !(form != 0 || no_form) {
        return handled(true);
    }
    let list = holder + at;
    let console = is_console_mode(e);
    match action {
        Action::AddAt => {
            let index = add_at(e, list, form, n);
            if index != LIST_INVALID {
                a.set_result(e, f64::from(index as u32));
            }
            if console {
                console_print(e, "GenericAddForm: Which: %d, Index: %d", &[which, index as u32]);
            }
        }
        Action::ChgAt => {
            let old = replace_nth(e, list, n, form);
            if old != 0 {
                let id = form_id(e, old);
                a.set_form_result(e, id);
                if console {
                    console_print(e, "GenericChangeForm: Which: %d, Replaced: %08X.", &[which, id]);
                }
            }
        }
        Action::DelAt => {
            let old = remove_nth(e, list, n);
            if old != 0 {
                let id = form_id(e, old);
                a.set_form_result(e, id);
                if console {
                    console_print(e, "GenericDeleteForm: Which: %d, Deleted: %08X.", &[which, id]);
                }
            }
        }
        Action::GetAt => {
            let found = nth_item(e, list, n);
            if found != 0 {
                let id = form_id(e, found);
                a.set_form_result(e, id);
                if console {
                    console_print(e, "GenericGetForm: Which: %d, Returned: %08X.", &[which, id]);
                }
            }
        }
        Action::Check => {
            let target = form_id(e, form);
            for i in 0..count(e, list) {
                let item = nth_item(e, list, i as i32);
                if super::misc_ref::passed_over(e, item) {
                    continue;
                }
                if form_id(e, item) == target {
                    a.set_result(e, f64::from(i));
                    if console {
                        console_print(e, "GenericCheckForm: Which: %d, Returned: %d.", &[which, i]);
                    }
                    return handled(true);
                }
            }
            a.set_result(e, f64::from(LIST_END));
            if console {
                console_print(e, "GenericCheckForm: Which: %d, Not found!", &[which]);
            }
        }
    }
    handled(true)
}

fn generic_add_form(e: &mut Engine, w: &[u32]) -> Ret {
    generic(e, w, Action::AddAt)
}
fn generic_replace_form(e: &mut Engine, w: &[u32]) -> Ret {
    generic(e, w, Action::ChgAt)
}
fn generic_delete_form(e: &mut Engine, w: &[u32]) -> Ret {
    generic(e, w, Action::DelAt)
}
fn generic_get_form(e: &mut Engine, w: &[u32]) -> Ret {
    generic(e, w, Action::GetAt)
}
fn generic_check_form(e: &mut Engine, w: &[u32]) -> Ret {
    generic(e, w, Action::Check)
}

// ---- default objects ----

fn default_slot(e: &Engine, index: u32) -> u32 {
    e.global::<u32>(DEFAULT_OBJECTS) + DEFAULT_FORMS + 4 * index
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp:
// Cmd_GetNthDefaultForm_Execute.
fn get_nth_default_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    let read = Read::of(e, &a);
    let index = read.int(0, DEFAULT_OBJECT_MAX);
    if read.ok && index < DEFAULT_OBJECT_MAX {
        let form = e.mem.u32(default_slot(e, index));
        if form != 0 {
            let id = form_id(e, form);
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp:
// Cmd_SetNthDefaultForm_Execute.
/// The default form at the index set; the result is not touched.
fn set_nth_default_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let index = read.int(0, DEFAULT_OBJECT_MAX);
    if read.ok && index < DEFAULT_OBJECT_MAX {
        let slot = default_slot(e, index);
        e.mem.set_u32(slot, read.form(1));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp:
// Cmd_GetDefaultForms_Execute.
/// The 34 default forms' ids in order, 0 for an empty one.
fn get_default_forms(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let modi = vars::script_mod_index(e, a.script);
    let array = vars::create_array(e, vars::KEY_NUMBER, true, modi);
    a.set_result(e, f64::from(array));
    for i in 0..DEFAULT_OBJECT_MAX {
        let form = e.mem.u32(default_slot(e, i));
        let id = if form != 0 { form_id(e, form) } else { 0 };
        vars::set_element(e, array, &Key::Number(f64::from(i)), Value::Form(id));
    }
    handled(true)
}

// ---- quest objective doors ----

/// The items of a `tList` from its head up to its end, empty nodes kept as 0 (an iterator walk
/// that looks at every node).
fn tlist_nodes(e: &Engine, head: u32) -> Vec<u32> {
    let mut items = Vec::new();
    let mut node = head;
    while !(node == 0 || (e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0)) {
        items.push(e.mem.u32(node));
        node = e.mem.u32(node + 4);
    }
    items
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp:
// Cmd_GetCurrentQuestObjectiveTeleportLinks_Execute.
/// An array with one array per target of the current quest objective: the doors on the way, then
/// the target (each array made before it is filled, in order).
fn get_current_quest_objective_teleport_links(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let modi = vars::script_mod_index(e, a.script);
    let array = vars::create_array(e, vars::KEY_NUMBER, true, modi);
    a.set_result(e, f64::from(array));
    let player = e.global::<u32>(PLAYER);
    if player == 0 {
        return handled(true);
    }
    let targets = e.call(CURRENT_QUEST_TARGETS, &args![player]).u32();
    if targets == 0 {
        return handled(true);
    }
    for (i, target) in tlist_nodes(e, targets).into_iter().enumerate() {
        let sub = vars::create_array(e, vars::KEY_NUMBER, true, modi);
        vars::set_element(e, array, &Key::Number(i as f64), Value::Array(sub));
        let mut at = 0.0;
        if target != 0 {
            let (data, size) = (
                e.mem.u32(target + TARGET_DOORS + 4),
                e.mem.u32(target + TARGET_DOORS + 8),
            );
            for k in 0..size {
                let door = e.mem.u32(data + LINK_SIZE * k);
                if door != 0 {
                    let id = form_id(e, door);
                    vars::set_element(e, sub, &Key::Number(at), Value::Form(id));
                    at += 1.0;
                }
            }
            let refr = e.mem.u32(target + TARGET_REF);
            if refr != 0 {
                let id = form_id(e, refr);
                vars::set_element(e, sub, &Key::Number(at), Value::Form(id));
            }
        }
    }
    handled(true)
}

// ---- special idles ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: the animation commands'
// shared part.
/// The special idle names of the form (else the calling reference): an NPC's or a creature's. For
/// a reference xNVSE takes the CALLING reference's base form, not the given reference's (0 without
/// a calling reference).
fn animations(e: &mut Engine, this_obj: u32, form: u32) -> u32 {
    let mut form = form;
    if form == 0 && this_obj != 0 {
        form = this_obj;
    }
    if form != 0 && e.vcall(form, IS_REFERENCE, &args![]).u32() & 0xff != 0 {
        let refr = dynamic_cast(e, this_obj, rtti::TES_FORM, rtti::TES_OBJECT_REFR);
        form = if refr != 0 {
            e.mem.u32(refr + REFR_BASE_FORM)
        } else {
            0
        };
    }
    if form == 0 {
        return 0;
    }
    let npc = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_NPC);
    let creature = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_CREATURE);
    if npc != 0 {
        npc + ANIMATION_NAMES
    } else if creature != 0 {
        creature + ANIMATION_NAMES
    } else {
        0
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetNthAnimation_Execute.
/// The name at the index (counting every node, empty ones too), as a string; "" for none (also
/// after a failed read).
fn get_nth_animation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let index = read.int(0, u32::MAX) as i32;
    let mut name = 0;
    if read.ok && index >= 0 {
        let list = animations(e, a.this_obj, read.form(1));
        if list != 0 {
            name = tlist_nodes(e, list)
                .get(index as usize)
                .copied()
                .unwrap_or(0);
        }
    }
    let text = if name != 0 {
        crate::table::guest_text(e, name)
    } else {
        Vec::new()
    };
    super::string::assign_to_string_var(e, &a, &text);
    handled(true)
}

/// The name argument of `AddAnimation` and `DelAnimation`, which xNVSE reads into a 256-byte
/// buffer on its stack: the script's bytes with their terminator, then (for a `$` string
/// variable) the variable's text with its own. A name of 256 bytes or more written there runs into
/// the command's own variables above the buffer (the form it then uses), then the saved frame and
/// the return address: what xNVSE does from there is not defined. With the script list and scan
/// guard row (`NV_NVSE_LIST_GUARD`) the whole name is used; without it the port stops the game.
fn animation_name(e: &Engine, read: &Read, command: &str) -> Vec<u8> {
    let written = match read.args.first() {
        Some(crate::args::Arg::Str { raw, text }) => raw.len().max(text.len()),
        _ => 0,
    };
    if written >= NAME_BUFFER && !super::array::fix_on(e, super::array::LIST_GUARD) {
        panic!(
            "{command}: a name of {written} bytes overflows xNVSE's 256-byte buffer and the game \
             stops (NV_NVSE_LIST_GUARD reads it whole)"
        );
    }
    read.text(0)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_AddAnimation_Execute.
/// A copy of the name (on the game's heap, `strcpy_s`) added at the end (`tList::AddAt` with
/// `eListEnd`, inlined at `10035B7B`); nothing for an empty name.
fn add_animation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = animation_name(e, &read, "AddAnimation");
    if !read.ok || name.is_empty() {
        return handled(true);
    }
    let list = animations(e, a.this_obj, read.form(1));
    if list != 0 {
        let copy = e
            .call(FORM_HEAP_ALLOCATE, &args![name.len() as u32 + 1])
            .u32();
        e.mem.write(copy, &name);
        e.mem.set_u8(copy + name.len() as u32, 0);
        add_at(e, list, copy, LIST_END);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_DelAnimation_Execute.
/// The first name equal to the one given (without case: `StrCompare`, the game's `toupper` of each
/// byte read unsigned, `10035D30`) taken out and freed, its node too. Every node counts towards
/// the index, empty ones included.
fn del_animation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = animation_name(e, &read, "DelAnimation");
    if !read.ok || name.is_empty() {
        return handled(true);
    }
    let list = animations(e, a.this_obj, read.form(1));
    if list == 0 {
        return handled(true);
    }
    let upper = vars::upper_table(e);
    for (index, item) in tlist_nodes(e, list).into_iter().enumerate() {
        if item != 0 {
            let text = crate::table::guest_text(e, item);
            if vars::str_compare(&upper, &text, &name) == std::cmp::Ordering::Equal {
                let removed = remove_nth(e, list, index as i32);
                e.call(FORM_HEAP_FREE, &args![removed]);
                break;
            }
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_DelAnimations_Execute;
// GameTypes.h: tList::RemoveAll.
/// Every name taken out (the nodes freed, the names not). Only an NPC's: xNVSE's second test asks
/// for the NPC again, so a creature's list stays.
fn del_animations(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(0);
    if form == 0 && a.this_obj != 0 {
        form = a.this_obj;
    }
    if form != 0 && e.vcall(form, IS_REFERENCE, &args![]).u32() & 0xff != 0 {
        let refr = dynamic_cast(e, a.this_obj, rtti::TES_FORM, rtti::TES_OBJECT_REFR);
        form = if refr != 0 {
            e.mem.u32(refr + REFR_BASE_FORM)
        } else {
            0
        };
    }
    if form == 0 {
        return handled(true);
    }
    let npc = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_NPC);
    if npc != 0 {
        let head = npc + ANIMATION_NAMES;
        let mut node = e.mem.u32(head + 4);
        e.mem.set_u32(head, 0);
        e.mem.set_u32(head + 4, 0);
        while node != 0 {
            let next = e.mem.u32(node + 4);
            e.call(FORM_HEAP_FREE, &args![node]);
            node = next;
        }
    }
    handled(true)
}

// ---- doors and challenges ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_GetDoorSound_Execute.
/// A door form's open (mode 0) or close (any other mode) sound. A form that is no door: 0, and in
/// the console "Form <id> is not a TESObjectDOOR". A door without that sound (a door record with
/// no `SNAM` or `ANAM`, which the game allows) and the console's line for no form at all are read
/// through, as in xNVSE, unless the script reference checks row answers 0 (and prints nothing for
/// no form).
fn get_door_sound(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    super::string::with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks else {
            return;
        };
        let (Some(first), Some(second)) = (toks.first(), toks.get(1)) else {
            return;
        };
        let form = ev.get_form(e, first);
        let mode = crate::args::f64_to_i32(ev.get_number(e, second));
        let door = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_OBJECT_DOOR);
        if door == 0 {
            if is_console_mode(e) && !super::misc_ref::passed_over(e, form) {
                let id = form_id(e, form);
                console_print(e, "Form %X is not a TESObjectDOOR", &[id]);
            }
            return;
        }
        let sound = e.mem.u32(door + if mode == 0 { DOOR_OPEN_SOUND } else { DOOR_CLOSE_SOUND });
        if super::misc_ref::passed_over(e, sound) {
            return;
        }
        let id = form_id(e, sound);
        a.set_form_result(e, id);
    });
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Misc.cpp: Cmd_FireChallenge_Execute.
/// The game's challenge increment with the six arguments (numbers truncated to `int`s, the weapon
/// as a form; read in that order); its answer the result, as an unsigned 32-bit value. Fewer or
/// more than six arguments: 0. For a type of 0 to 13 the game's function (which returns nothing)
/// ends with `al` 1 over an `eax` of 0 (`005F5F22`; its list rebuild `005F5880` also ends with 0
/// there), so the answer is 1. For any other type it ends at once with only `al` cleared
/// (`005F5970`), the rest of `eax` holding its stack cookie: xNVSE then answers a number that
/// differs from run to run, which nothing can reproduce; here the translated function's 0.
fn fire_challenge(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    super::string::with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks.filter(|t| t.len() == 6) else {
            return;
        };
        let int = |e: &mut Engine, i: usize| crate::args::f64_to_i32(ev.get_number(e, &toks[i])) as u32;
        let kind = int(e, 0);
        let count = int(e, 1);
        let weapon = ev.get_form(e, &toks[2]);
        let (v1, v2, v3) = (int(e, 3), int(e, 4), int(e, 5));
        let r = e
            .call(INCREMENT_CHALLENGE, &args![kind, count, 0u32, weapon, v1, v2, v3])
            .u32();
        a.set_result(e, f64::from(r));
    });
    handled(true)
}

#[cfg(test)]
#[path = "misc_forms_tests.rs"]
mod tests;
