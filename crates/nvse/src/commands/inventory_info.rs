//! The rest of xNVSE's `Commands_Inventory.cpp` (xNVSE), batch 5 of I-12, with what they share in
//! xNVSE's `InventoryInfo.cpp` and `InventoryInfo.h`: a container's items counted and indexed
//! (`GetNumItems`, `GetInventoryObject`), `PickOneOf`, the "token" commands (an item held once,
//! with a health and an owner: `GetTokenValue`, `SetTokenValue`, `GetTokenRef`, `SetTokenRef`,
//! `SetTokenValueAndRef`, `GetPaired`) and adding items with a health and an owner
//! (`AddItemHealthPercentOwner`, `AddItemOwnership`).

use super::{console_print, double_words, dynamic_cast, is_console_mode, rtti, Read};
use crate::invref::{self, get_by_type};
use crate::table::{guest_str, Command, Param, Returns};
use crate::{handled, CommandArgs, EvalArgs, Nvse};
use engine_core::prelude::*;
use std::collections::HashMap;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_Integer`, `_Float`, `_ObjectID`,
/// `_Actor`, `_FormList`, `_AnyForm`.
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
const OBJECT_ID: u32 = 0x03;
const ACTOR_PARAM: u32 = 0x06;
const FORM_LIST: u32 = 0x25;
const ANY_FORM: u32 = 0x3d;
/// `kParams_OneInt`, `kParams_FormList`, `kParams_OneForm`, `kParams_OneObjectID_OneFloat`,
/// `kParams_OneObjectID_OneObject`, `kParams_OneObjectID_OneFloat_OneObject`,
/// `kParams_OneObjectID_OneActor`, `kParams_OneObjectID_OneFloat_OneObject_OneOptionalRank`,
/// `kParams_OneObjectID_TwoFloat_OneObject_OneOptionalRank` (xNVSE).
const ONE_INT: &[Param] = &[p("int", INT, false)];
const ONE_FORM_LIST: &[Param] = &[p("form list", FORM_LIST, false)];
const ONE_FORM: &[Param] = &[p("form", ANY_FORM, false)];
const ITEM_VALUE: &[Param] = &[p("item", OBJECT_ID, false), p("value", FLOAT, false)];
const ITEM_REFERENCE: &[Param] = &[p("item", OBJECT_ID, false), p("reference", ANY_FORM, false)];
const ITEM_VALUE_REFERENCE: &[Param] = &[
    p("item", OBJECT_ID, false),
    p("value", FLOAT, false),
    p("reference", ANY_FORM, false),
];
const ITEM_ACTOR: &[Param] = &[p("item", OBJECT_ID, false), p("actor", ACTOR_PARAM, false)];
const ITEM_COUNT_OWNER_RANK: &[Param] = &[
    p("item", OBJECT_ID, false),
    p("count", FLOAT, false),
    p("owner", ANY_FORM, false),
    p("rank", INT, true),
];
const ITEM_COUNT_HEALTH_OWNER_RANK: &[Param] = &[
    p("item", OBJECT_ID, false),
    p("count", FLOAT, false),
    p("health", FLOAT, false),
    p("owner", ANY_FORM, false),
    p("rank", INT, true),
];

/// `TESForm::refID` (`+0xC`); `TESObjectREFR::baseForm` (`+0x20`), `extraDataList` (`+0x44`).
const FORM_ID: u32 = 0xc;
const REFR_BASE_FORM: u32 = 0x20;
const REFR_EXTRA: u32 = invref::REFR_EXTRA;
/// `ExtraContainerChanges::data` (`+0xC`), its `objList` (`+0`).
const CHANGES_DATA: u32 = 0xc;
/// `TESContainer::formCountList` (a `tList` at `+4`) of `FormCount`s: `count` (`+0`), `form`
/// (`+4`).
const CONTAINER_LIST: u32 = 4;
/// `BGSListForm::list` (a `tList` at `+0x18`).
const FORM_LIST_LIST: u32 = 0x18;
/// Extra data (xNVSE `GameExtraData.h`): `kExtraData_Script`, `_Ownership`, `_Rank`, `_Health`;
/// `ExtraOwnership::owner`, `ExtraRank::rank`, `ExtraHealth::health` (`+0xC`), `ExtraScript::script`
/// (`+0xC`) and `eventList` (`+0x10`); their sizes and vtables (`GameExtraData.cpp` (xNVSE)).
const EXTRA_SCRIPT: u32 = 0x0d;
const EXTRA_OWNERSHIP: u32 = 0x21;
const EXTRA_RANK: u32 = 0x23;
const EXTRA_HEALTH: u32 = 0x25;
const EXTRA_VALUE: u32 = 0xc;
const EXTRA_SCRIPT_EVENT_LIST: u32 = 0x10;
const EXTRA_SIZE: u32 = 0x10;
const EXTRA_SCRIPT_SIZE: u32 = 0x14;
const EXTRA_OWNERSHIP_VTBL: u32 = 0x0101_58b4;
const EXTRA_RANK_VTBL: u32 = 0x0101_58cc;
const EXTRA_HEALTH_VTBL: u32 = 0x0101_58e4;
const EXTRA_SCRIPT_VTBL: u32 = 0x0101_5914;
/// `ExtraDataList`'s vtable and size (`ExtraDataList::Create` (xNVSE)).
const EXTRA_LIST_VTBL: u32 = 0x0101_43e8;
const EXTRA_LIST_SIZE: u32 = 0x20;
/// `FormHeap_Allocate`.
const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
/// `TESHealthForm::health` (`+4`) and `GetHealth` (virtual `+0x10`, after `BaseFormComponent`'s
/// four); `TESScriptableForm::script` (`+4`).
const HEALTH_FORM_HEALTH: u32 = 4;
const GET_HEALTH: u32 = 0x10;
const SCRIPTABLE_SCRIPT: u32 = 4;
/// `TESObjectREFR::AddItem` (virtual `+0x190`: item, extra data list, quantity),
/// `TESForm::MarkAsModified` (virtual `+0x48`), `kChanged_Inventory`.
const REFR_ADD_ITEM: u32 = 0x190;
const MARK_AS_MODIFIED: u32 = 0x48;
const CHANGED_INVENTORY: u32 = 0x0800_0000;
/// `Script::CreateEventList` (`0x005ABF60`, `thiscall`), `LookupFormByID` (`0x004839C0`).
const CREATE_EVENT_LIST: u32 = 0x005a_bf60;
const LOOKUP_FORM: u32 = 0x0048_39c0;

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

pub static COMMANDS: &[Command] = &[
    Command {
        eval: Some(get_num_items_eval),
        ..cmd(
            0x144e,
            "GetNumItems",
            "GetNumObjects",
            "returns the number of items contained by the calling ref",
            true,
            &[],
            get_num_items,
        )
    },
    Command {
        returns: Returns::Form,
        ..cmd(
            0x144f,
            "GetInventoryObject",
            "GetNthObject",
            "returns the base object of the specified index contained in the calling ref",
            true,
            ONE_INT,
            get_inventory_object,
        )
    },
    cmd(
        0x1504,
        "AddItemOwnership",
        "",
        "adds an item to a container with a specified ownership.",
        true,
        ITEM_COUNT_OWNER_RANK,
        add_item_ownership,
    ),
    cmd(
        0x1505,
        "AddItemHealthPercentOwner",
        "",
        "adds an item to a container at a specified health percentage and with an ownership.",
        true,
        ITEM_COUNT_HEALTH_OWNER_RANK,
        add_item_health_percent_owner_execute,
    ),
    Command {
        eval: Some(get_token_value_eval),
        ..cmd(
            0x1506,
            "GetTokenValue",
            "GetTV",
            "get a token value.",
            false,
            ONE_FORM,
            get_token_value,
        )
    },
    cmd(
        0x1507,
        "SetTokenValue",
        "SetTV",
        "set a token value or create a token with value.",
        true,
        ITEM_VALUE,
        set_token_value,
    ),
    Command {
        eval: Some(get_token_ref_eval),
        returns: Returns::Form,
        ..cmd(
            0x1508,
            "GetTokenRef",
            "GetTR",
            "get a token reference.",
            false,
            ONE_FORM,
            get_token_ref,
        )
    },
    cmd(
        0x1509,
        "SetTokenRef",
        "SetTR",
        "Add or modify a token reference.",
        true,
        ITEM_REFERENCE,
        set_token_ref,
    ),
    cmd(
        0x150a,
        "SetTokenValueAndRef",
        "SetTVR",
        "Add or modify a token with value and reference.",
        true,
        ITEM_VALUE_REFERENCE,
        set_token_value_and_ref,
    ),
    Command {
        eval: Some(get_paired_eval),
        ..cmd(
            0x150b,
            "GetPaired",
            "",
            "Detects if ref and actor are crossreferencing each others",
            true,
            ITEM_ACTOR,
            get_paired,
        )
    },
    Command {
        returns: Returns::Form,
        ..cmd(
            0x159f,
            "PickOneOf",
            "",
            "Selects a random item present in the formList",
            true,
            ONE_FORM_LIST,
            pick_one_of,
        )
    },
];

// ---- the game's lists, as xNVSE's tList walks them ----

/// The nodes of a `tList` from its head (`do ... while (node = node->next)`), each node's data
/// (0 for an empty one); `None` when the list guard row ended the walk (`invref::list_step`).
fn nodes(e: &Engine, head: u32) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    let mut node = head;
    let mut walked = 0usize;
    while node != 0 {
        if !invref::list_step(e, &mut walked) {
            return None;
        }
        out.push(e.mem.u32(node));
        node = e.mem.u32(node + 4);
    }
    Some(out)
}

/// `tList::Count`: 0 when the head is empty, else the number of nodes.
fn list_count(e: &Engine, head: u32) -> Option<u32> {
    if e.mem.u32(head) == 0 {
        return Some(0);
    }
    nodes(e, head).map(|n| n.len() as u32)
}

/// `tList::GetNthItem(n)` (`n` from 0): the data of node `n`, 0 past the end.
fn nth_item(e: &Engine, head: u32, n: i32) -> Option<u32> {
    if n < 0 {
        return Some(0);
    }
    nodes(e, head).map(|v| v.get(n as usize).copied().unwrap_or(0))
}

/// A reference's container changes entry list (`xChanges->data->objList`), 0 without container
/// changes. Changes without data are read through, as xNVSE does (the game stops).
fn changes_entries(e: &mut Engine, reference: u32) -> u32 {
    let changes = get_by_type(e, reference + REFR_EXTRA, invref::EXTRA_CONTAINER_CHANGES);
    if changes == 0 {
        return 0;
    }
    let data = e.mem.u32(changes + CHANGES_DATA);
    e.mem.u32(data)
}

/// The reference's base form's container (`DYNAMIC_CAST(baseForm, TESForm, TESContainer)`), 0.
fn container(e: &mut Engine, reference: u32) -> u32 {
    let base = e.mem.u32(reference + REFR_BASE_FORM);
    if base == 0 {
        return 0;
    }
    dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_CONTAINER)
}

fn entry_delta(e: &Engine, entry: u32) -> i32 {
    e.mem.u32(entry + invref::ENTRY_COUNT_DELTA) as i32
}

fn entry_type(e: &Engine, entry: u32) -> u32 {
    e.mem.u32(entry + invref::ENTRY_TYPE)
}

/// `GetFullName(TESForm *)` (xNVSE `GameForms.cpp`), a guest string for the console.
fn full_name_text(e: &mut Engine, form: u32) -> u32 {
    if form == 0 {
        return guest_str(e, b"<NULL>");
    }
    let name = super::string::tes_full_name(e, form);
    if name != 0 {
        let (data, len) = (e.mem.u32(name + 4), e.mem.u16(name + 8));
        if data != 0 && len != 0 {
            return data;
        }
    }
    guest_str(e, b"<no name>")
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utilities.cpp: GetObjectClassName.
/// The object's class from its vtable's RTTI: the type descriptor's name (`+8`) without its
/// `.?AV` (`"TESObjectWEAP@@"`), when it starts `.?` and ends within 100 characters; `"<no rtti>"`
/// otherwise and where xNVSE's exception handler would catch a bad read (here: a read outside
/// mapped memory).
fn object_class_name(e: &mut Engine, object: u32) -> u32 {
    let read = |e: &Engine, at: u32| e.mem.is_mapped(at).then(|| e.mem.u32(at));
    let name = read(e, object)
        .and_then(|vtbl| read(e, vtbl.wrapping_sub(4)))
        .and_then(|locator| read(e, locator + 0xc))
        .map(|kind| kind + 8);
    if let Some(name) = name {
        if e.mem.is_mapped(name) && e.mem.u8(name) == b'.' && e.mem.u8(name + 1) == b'?' {
            for i in 0..100 {
                if !e.mem.is_mapped(name + i) {
                    break;
                }
                if e.mem.u8(name + i) == 0 {
                    return name + 4;
                }
            }
        }
    }
    guest_str(e, b"<no rtti>")
}

/// `PrintItemType` (xNVSE `InventoryInfo.cpp`): the item's name and class on the console.
fn print_item_type(e: &mut Engine, form: u32) {
    let name = full_name_text(e, form);
    let class = object_class_name(e, form);
    console_print(e, "%s (%s)", &[name, class]);
}

// ---- ExtraContainerInfo and its finders (InventoryInfo.h) ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), InventoryInfo.h: ExtraContainerInfo and
// tList::Visit.
/// The container changes' entries up to the first empty node, in order, and each item's place
/// among them (the last entry of an item wins); an entry is taken out (0) once the base container
/// counted it.
struct Info {
    vec: Vec<u32>,
    map: HashMap<u32, usize>,
}

impl Info {
    fn new(e: &Engine, entries: u32) -> Option<Info> {
        let mut info = Info {
            vec: Vec::new(),
            map: HashMap::new(),
        };
        if entries != 0 {
            let mut node = entries;
            let mut walked = 0usize;
            while node != 0 {
                if !invref::list_step(e, &mut walked) {
                    return None;
                }
                let entry = e.mem.u32(node);
                if entry == 0 {
                    break;
                }
                info.map.insert(entry_type(e, entry), info.vec.len());
                info.vec.push(entry);
                node = e.mem.u32(node + 4);
            }
        }
        Some(info)
    }

    /// `IsValidFormCount`: whether a base container entry holds any of its item, its count with
    /// the changes' (whose entry is then taken out); a leveled item never does. In the console
    /// the item is printed.
    fn is_valid_form_count(&mut self, e: &mut Engine, fc: u32) -> bool {
        if fc == 0 {
            return false;
        }
        let mut num = e.mem.u32(fc) as i32;
        let form = e.mem.u32(fc + 4);
        if dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_LEV_ITEM) != 0 {
            return false;
        }
        if let Some(&at) = self.map.get(&form) {
            let entry = self.vec[at];
            if entry != 0 {
                num = num.wrapping_add(entry_delta(e, entry));
            }
            self.vec[at] = 0;
        }
        if num > 0 {
            if is_console_mode(e) {
                print_item_type(e, form);
            }
            return true;
        }
        false
    }

    /// `CountItems`: the entries left with a count above 0 (each printed in the console).
    fn count_items(&self, e: &mut Engine) -> u32 {
        let mut count = 0;
        for &entry in &self.vec {
            if entry != 0 && entry_delta(e, entry) > 0 {
                count += 1;
                if is_console_mode(e) {
                    let item = entry_type(e, entry);
                    print_item_type(e, item);
                }
            }
        }
        count
    }

    /// `GetNth(n, count)`: the entry left with a count above 0 at place `n`, counting from `count`.
    fn nth(&self, e: &Engine, n: u32, mut count: u32) -> u32 {
        for &entry in &self.vec {
            if entry != 0 && entry_delta(e, entry) > 0 {
                if count == n {
                    return entry;
                }
                count = count.wrapping_add(1);
            }
        }
        0
    }

    /// `GetRefID`: the first entry left with a count above 0 whose item has the id.
    fn ref_id(&self, e: &Engine, id: u32) -> u32 {
        for &entry in &self.vec {
            if entry != 0 && entry_delta(e, entry) > 0 {
                let item = entry_type(e, entry);
                if item != 0 && e.mem.u32(item + FORM_ID) == id {
                    return entry;
                }
            }
        }
        0
    }
}

/// `tList::Find` over a base container's entries with `accept` (each non-empty node's data, from
/// the head): the first accepted; `None` when the list guard row ended the walk.
fn find_form_count(
    e: &mut Engine,
    list: u32,
    mut accept: impl FnMut(&mut Engine, u32) -> bool,
) -> Option<u32> {
    for fc in nodes(e, list)? {
        if fc != 0 && accept(e, fc) {
            return Some(fc);
        }
    }
    Some(0)
}

/// `ContainerFindRefId` with `tList::Find`: the first valid base container entry for the item
/// with the id, and the number of valid entries before it (or of all valid ones).
fn find_ref_id(e: &mut Engine, info: &mut Info, list: u32, id: u32) -> Option<(u32, u32)> {
    let mut cur = 0u32;
    let found = find_form_count(e, list, |e, fc| {
        if info.is_valid_form_count(e, fc) {
            if e.mem.u32(e.mem.u32(fc + 4) + FORM_ID) == id {
                return true;
            }
            cur += 1;
        }
        false
    })?;
    Some((found, cur))
}

// ---- InventoryInfo.cpp ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), InventoryInfo.cpp: GetItemByIdx.
/// The item at place `index` among the reference's items: first the base container's entries
/// that hold any (with the changes'), then the changes' other entries with a count above 0. The
/// count is the base container entry's own, or the changes entry's.
fn item_by_index(e: &mut Engine, reference: u32, index: u32) -> Option<(u32, i32)> {
    let entries = changes_entries(e, reference);
    let mut info = Info::new(e, entries)?;
    let mut count = 0;
    let container = container(e, reference);
    if container != 0 {
        let mut cur = 0u32;
        let found = find_form_count(e, container + CONTAINER_LIST, |e, fc| {
            if info.is_valid_form_count(e, fc) {
                if cur == index {
                    return true;
                }
                cur += 1;
            }
            false
        })?;
        if found != 0 {
            return Some((e.mem.u32(found + 4), e.mem.u32(found) as i32));
        }
        count = cur;
    }
    let entry = info.nth(e, index, count);
    if entry != 0 {
        return Some((entry_type(e, entry), entry_delta(e, entry)));
    }
    Some((0, 0))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), InventoryInfo.cpp: GetItemByRefID.
/// The item with id `id` among the reference's items and its count: the base container's entry
/// first (its own count), then the changes' (its count delta); `(0, 0)` for none.
fn item_by_ref_id(e: &mut Engine, reference: u32, id: u32) -> Option<(u32, i32)> {
    let entries = changes_entries(e, reference);
    let mut info = Info::new(e, entries)?;
    let container = container(e, reference);
    if container != 0 {
        let (found, _) = find_ref_id(e, &mut info, container + CONTAINER_LIST, id)?;
        if found != 0 {
            return Some((e.mem.u32(found + 4), e.mem.u32(found) as i32));
        }
    }
    let entry = info.ref_id(e, id);
    if entry != 0 {
        return Some((entry_type(e, entry), entry_delta(e, entry)));
    }
    Some((0, 0))
}

/// What `GetItemWithHealthAndOwnershipByRefID` found: the item, its health, owner and rank.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Token {
    item: u32,
    health: f32,
    owner: u32,
    rank: u32,
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), InventoryInfo.cpp:
// GetItemWithHealthAndOwnershipByRefID.
/// The item with id `id` among the reference's changes and its stack at place `index` (from 0;
/// counted on by one when the stack was read): the stack's `ExtraHealth`, else the item's base
/// health, and its owner and rank (the rank only with an owner). Past the item's stacks, the base
/// container is looked at, at the stacks left over; xNVSE reads that place's health through the
/// changes entry, which is missing when the item is only in the base container (the game stops
/// there, as in xNVSE), and counts the place on through `index`, which `GetTokenValue` and
/// `GetTokenRef` do not pass (the same). `None` for no reference, no item or the list guard.
fn item_with_health_and_owner(
    e: &mut Engine,
    reference: u32,
    id: u32,
    mut index: Option<&mut i32>,
) -> Option<Token> {
    if reference == 0 {
        return None;
    }
    let mut token = Token::default();
    let entries = changes_entries(e, reference);
    let mut info = Info::new(e, entries)?;
    let entry = info.ref_id(e, id);
    let mut n = index.as_deref().copied().unwrap_or(0);
    if entry != 0 {
        let extend = e.mem.u32(entry + invref::ENTRY_EXTEND_DATA);
        if extend != 0 {
            let count = list_count(e, extend)? as i32;
            if count > n {
                let xdata = nth_item(e, extend, n)?;
                if xdata != 0 {
                    let health = get_by_type(e, xdata, EXTRA_HEALTH);
                    if health != 0 {
                        token.health = e.mem.f32(health + EXTRA_VALUE);
                    } else {
                        let form = dynamic_cast(
                            e,
                            entry_type(e, entry),
                            rtti::TES_FORM,
                            rtti::TES_HEALTH_FORM,
                        );
                        if form != 0 {
                            token.health = e.mem.u32(form + HEALTH_FORM_HEALTH) as f32;
                        }
                    }
                    let owner = get_by_type(e, xdata, EXTRA_OWNERSHIP);
                    let rank = get_by_type(e, xdata, EXTRA_RANK);
                    if owner != 0 {
                        token.owner = e.mem.u32(owner + EXTRA_VALUE);
                        if rank != 0 {
                            token.rank = e.mem.u32(rank + EXTRA_VALUE);
                        }
                    }
                    if let Some(i) = index.as_deref_mut() {
                        *i += 1;
                    }
                }
                token.item = entry_type(e, entry);
                return Some(token);
            }
            n -= count;
        }
    }
    let container = container(e, reference);
    if container != 0 {
        let list = container + CONTAINER_LIST;
        let (found, cur) = find_ref_id(e, &mut info, list, id)?;
        let at = if found != 0 {
            cur.wrapping_add(n as u32)
        } else {
            list_count(e, list)?
        };
        let fc = nth_item(e, list, at as i32)?;
        if fc != 0 {
            let form = e.mem.u32(fc + 4);
            if form != 0 && e.mem.u32(form + FORM_ID) == id {
                if entry == 0 {
                    panic!("xNVSE reads the health of an item held only in the base container through its missing changes entry");
                }
                let item = entry_type(e, entry);
                let health = dynamic_cast(e, item, rtti::TES_FORM, rtti::TES_HEALTH_FORM);
                if health != 0 {
                    token.health = e.mem.u32(health + HEALTH_FORM_HEALTH) as f32;
                }
                match index {
                    Some(i) => *i += 1,
                    None => {
                        panic!("xNVSE counts a base container stack on through a missing index")
                    }
                }
                token.item = form;
                return Some(token);
            }
        }
    }
    Some(Token::default())
}

/// A new extra data list (`ExtraDataList::Create`): zeroed, with its vtable.
fn new_extra_list(e: &mut Engine) -> u32 {
    let list = e.call(FORM_HEAP_ALLOCATE, &args![EXTRA_LIST_SIZE]).u32();
    e.mem.write(list, &[0u8; EXTRA_LIST_SIZE as usize]);
    e.mem.set_u32(list, EXTRA_LIST_VTBL);
    list
}

/// A new extra data block holding the word `value` at `+0xC`, added to `list`.
fn add_extra_word(e: &mut Engine, list: u32, kind: u32, vtable: u32, value: u32) -> u32 {
    let extra = invref::create_extra(e, kind, EXTRA_SIZE, vtable);
    invref::extra_add(e, list, extra);
    e.mem.set_u32(extra + EXTRA_VALUE, value);
    extra
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), InventoryInfo.cpp:
// AddItemHealthPercentOwner; GameExtraData.cpp: ExtraScript::Create.
/// Adds `count` of the item with id `id` to the reference, with a new extra data list when the
/// health is not 1 or there is an owner, a rank or an item script: an `ExtraHealth` of the base
/// health times `health` (not for -1), an `ExtraOwnership`, an `ExtraRank`, an `ExtraScript` with a
/// new event list. A health other than -1 needs an item with health. Answers the item, or 0.
fn add_item_health_percent_owner(
    e: &mut Engine,
    reference: u32,
    id: u32,
    count: i32,
    health: f32,
    owner: u32,
    rank: u32,
) -> u32 {
    if reference == 0 {
        return 0;
    }
    let form = e.call(LOOKUP_FORM, &args![id]).u32();
    if form == 0 {
        return 0;
    }
    let health_form = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_HEALTH_FORM);
    if health_form == 0 && health != -1.0 {
        return 0;
    }
    let mut script = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_SCRIPTABLE_FORM);
    if script != 0 && e.mem.u32(script + SCRIPTABLE_SCRIPT) == 0 {
        script = 0;
    }
    let mut list = 0;
    if health != 1.0 || owner != 0 || rank != 0 || script != 0 {
        list = new_extra_list(e);
        if health != 1.0 && health != -1.0 && health_form != 0 {
            let extra = invref::create_extra(e, EXTRA_HEALTH, EXTRA_SIZE, EXTRA_HEALTH_VTBL);
            invref::extra_add(e, list, extra);
            let base = e.vcall(health_form, GET_HEALTH, &args![]).u32();
            e.mem.set_f32(extra + EXTRA_VALUE, base as f32 * health);
        }
        if owner != 0 {
            add_extra_word(e, list, EXTRA_OWNERSHIP, EXTRA_OWNERSHIP_VTBL, owner);
        }
        if rank != 0 {
            add_extra_word(e, list, EXTRA_RANK, EXTRA_RANK_VTBL, rank);
        }
        if script != 0 {
            let extra = invref::create_extra(e, EXTRA_SCRIPT, EXTRA_SCRIPT_SIZE, EXTRA_SCRIPT_VTBL);
            if extra != 0 {
                let s = e.mem.u32(script + SCRIPTABLE_SCRIPT);
                e.mem.set_u32(extra + EXTRA_VALUE, s);
                let events = e.call(CREATE_EVENT_LIST, &args![s]).u32();
                e.mem.set_u32(extra + EXTRA_SCRIPT_EVENT_LIST, events);
            }
            invref::extra_add(e, list, extra);
        }
    }
    e.vcall(reference, REFR_ADD_ITEM, &args![form, list, count as u32]);
    form
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), InventoryInfo.cpp:
// SetFirstItemWithHealthAndOwnershipByRefID.
/// Sets the item's first stack: its count delta (and `ExtraCount`) to `count`, its `ExtraHealth`
/// (made; removed when the health equals the *container's* base form's health, as xNVSE compares),
/// owner and rank (made, changed, or removed for none); -1 leaves the health. Without changes, an
/// entry for the item or its lists, the item is added instead. Nothing to set (health -1, no owner,
/// no rank): 0.
fn set_first_item(
    e: &mut Engine,
    reference: u32,
    id: u32,
    count: i32,
    health: f32,
    owner: u32,
    rank: u32,
) -> Option<u32> {
    if reference == 0 {
        return Some(0);
    }
    if health == -1.0 && owner == 0 && rank == 0 {
        return Some(0);
    }
    let add = |e: &mut Engine| {
        Some(add_item_health_percent_owner(
            e, reference, id, count, health, owner, rank,
        ))
    };
    let changes = get_by_type(e, reference + REFR_EXTRA, invref::EXTRA_CONTAINER_CHANGES);
    if changes == 0 {
        return add(e);
    }
    let data = e.mem.u32(changes + CHANGES_DATA);
    let entries = e.mem.u32(data);
    if entries == 0 {
        return add(e);
    }
    let info = Info::new(e, entries)?;
    let entry = info.ref_id(e, id);
    if entry == 0 {
        return add(e);
    }
    let extend = e.mem.u32(entry + invref::ENTRY_EXTEND_DATA);
    if extend == 0 {
        return add(e);
    }
    let base = e.mem.u32(reference + REFR_BASE_FORM);
    let health_form = dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_HEALTH_FORM);
    e.mem
        .set_u32(entry + invref::ENTRY_COUNT_DELTA, count as u32);
    let list = e.mem.u32(extend);
    let x_health = get_by_type(e, list, EXTRA_HEALTH);
    let x_count = get_by_type(e, list, invref::EXTRA_COUNT);
    let x_owner = get_by_type(e, list, EXTRA_OWNERSHIP);
    let x_rank = get_by_type(e, list, EXTRA_RANK);
    if x_health == 0 {
        if health != -1.0 {
            let extra = invref::create_extra(e, EXTRA_HEALTH, EXTRA_SIZE, EXTRA_HEALTH_VTBL);
            invref::extra_add(e, list, extra);
            e.mem.set_f32(extra + EXTRA_VALUE, health);
        }
    } else if health != -1.0 {
        if health_form != 0 && health == e.mem.u32(health_form + HEALTH_FORM_HEALTH) as f32 {
            invref::extra_remove(e, list, x_health, true);
        } else {
            e.mem.set_f32(x_health + EXTRA_VALUE, health);
        }
    }
    if x_owner == 0 {
        if owner != 0 {
            add_extra_word(e, list, EXTRA_OWNERSHIP, EXTRA_OWNERSHIP_VTBL, owner);
        }
    } else if owner != 0 {
        e.mem.set_u32(x_owner + EXTRA_VALUE, owner);
    } else {
        invref::extra_remove(e, list, x_owner, true);
    }
    if x_rank == 0 {
        if rank != 0 {
            add_extra_word(e, list, EXTRA_RANK, EXTRA_RANK_VTBL, rank);
        }
    } else if rank != 0 {
        e.mem.set_u32(x_rank + EXTRA_VALUE, rank);
    } else {
        invref::extra_remove(e, list, x_rank, true);
    }
    if x_count != 0 {
        e.mem.set_u16(x_count + EXTRA_VALUE, count as u16);
    }
    Some(entry_type(e, entry))
}

// ---- the commands ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp: GetNumItems_Call.
/// The base container's entries holding any of their item (with the changes'), and the changes'
/// other entries with a count above 0; 0 when the list guard ended a walk.
fn num_items(e: &mut Engine, reference: u32) -> u32 {
    let entries = changes_entries(e, reference);
    let Some(mut info) = Info::new(e, entries) else {
        return 0;
    };
    let mut count = 0;
    let container = container(e, reference);
    if container != 0 {
        let Some(list) = nodes(e, container + CONTAINER_LIST) else {
            return 0;
        };
        for fc in list {
            if fc != 0 && info.is_valid_form_count(e, fc) {
                count += 1;
            }
        }
    }
    count + info.count_items(e)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetNumItems_Execute.
fn get_num_items(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let count = f64::from(num_items(e, a.this_obj));
    a.set_result(e, count);
    if is_console_mode(e) {
        console_print(e, "item count: %f", &double_words(count));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetNumItems_Eval.
fn get_num_items_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    e.mem.set_f64(a.result, 0.0);
    if a.this_obj != 0 {
        let count = f64::from(num_items(e, a.this_obj));
        e.mem.set_f64(a.result, count);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetInventoryObject_Execute.
/// The item at the index (in `GetNumItems`' order); in the console the index, its name and class.
fn get_inventory_object(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let index = read.int(0, 0);
    if let Some((item, _)) = item_by_index(e, a.this_obj, index) {
        if item != 0 {
            let id = e.mem.u32(item + FORM_ID);
            a.set_form_result(e, id);
            if is_console_mode(e) {
                let name = full_name_text(e, item);
                let class = object_class_name(e, item);
                console_print(e, "%d: %s (%s)", &[index, name, class]);
            }
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_PickOneOf_Execute.
/// One of the form list's forms the calling actor holds (any count above 0), drawn with xNVSE's
/// MT19937 when there are several. A list entry without a form is read through, as in xNVSE.
fn pick_one_of(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if dynamic_cast(e, a.this_obj, rtti::TES_FORM, rtti::ACTOR) == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let list = read.form(0) + FORM_LIST_LIST;
    let Some(count) = list_count(e, list) else {
        return handled(true);
    };
    let mut present = Vec::new();
    for i in 0..count {
        let Some(form) = nth_item(e, list, i as i32) else {
            return handled(true);
        };
        let id = e.mem.u32(form + FORM_ID);
        let Some((_, n)) = item_by_ref_id(e, a.this_obj, id) else {
            return handled(true);
        };
        if n > 0 {
            present.push(form);
        }
    }
    let pick = match present.len() {
        0 => return handled(true),
        1 => present[0],
        len => {
            let r = e.ext::<Nvse>().mt.genrand_real2() * len as f64;
            present[r as u32 as usize]
        }
    };
    let id = e.mem.u32(pick + FORM_ID);
    a.set_form_result(e, id);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_AddItemHealthPercentOwner_Execute.
fn add_item_health_percent_owner_cmd(e: &mut Engine, w: &[u32], with_health: bool) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let (health, owner, rank) = if with_health {
            (read.float(2, 1.0), read.form(3), read.int(4, 0))
        } else {
            (-1.0, read.form(2), read.int(3, 0))
        };
        let item = read.form(0);
        let id = e.mem.u32(item + FORM_ID);
        let count = crate::args::f64_to_i32(f64::from(read.float(1, 1.0)));
        add_item_health_percent_owner(e, a.this_obj, id, count, health, owner, rank);
    }
    handled(true)
}

fn add_item_health_percent_owner_execute(e: &mut Engine, w: &[u32]) -> Ret {
    add_item_health_percent_owner_cmd(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_AddItemOwnership_Execute (a health of -1: no `ExtraHealth`).
fn add_item_ownership(e: &mut Engine, w: &[u32]) -> Ret {
    add_item_health_percent_owner_cmd(e, w, false)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetTokenValue_Eval.
/// The token's health (its first stack's); in the console with its name.
fn token_value(e: &mut Engine, reference: u32, item: u32, result: u32) {
    e.mem.set_f64(result, 0.0);
    let id = e.mem.u32(item + FORM_ID);
    let Some(token) = item_with_health_and_owner(e, reference, id, None) else {
        return;
    };
    if token.item == 0 {
        return;
    }
    let value = f64::from(token.health);
    e.mem.set_f64(result, value);
    if is_console_mode(e) {
        let name = super::string::tes_full_name(e, token.item);
        let [lo, hi] = double_words(value);
        if name != 0 {
            let text = e.mem.u32(name + 4);
            console_print(e, "GetTokenValue: >> %f (%s)", &[lo, hi, text]);
        } else {
            console_print(e, "GetTokenValue: >> %f", &[lo, hi]);
        }
    }
}

fn get_token_value_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    token_value(e, a.this_obj, a.arg1, a.result);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetTokenValue_Execute.
fn get_token_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        token_value(e, a.this_obj, read.form(0), a.result);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetTokenRef_Eval.
/// The token's owner (its first stack's); in the console the result's two words go to `%#10X` and
/// `%s` as xNVSE passes the `double`, so the name printed is the high word's (none).
fn token_ref(e: &mut Engine, reference: u32, item: u32, result: u32) {
    e.mem.set_u32(result, 0);
    let id = e.mem.u32(item + FORM_ID);
    let Some(token) = item_with_health_and_owner(e, reference, id, None) else {
        return;
    };
    if token.owner == 0 {
        return;
    }
    let owner_id = e.mem.u32(token.owner + FORM_ID);
    e.mem.set_u32(result, owner_id);
    if is_console_mode(e) {
        let [lo, hi] = double_words(e.mem.f64(result));
        let name = super::string::tes_full_name(e, token.owner);
        if name != 0 {
            let text = e.mem.u32(name + 4);
            console_print(e, "TokenReference: >> %#10X (%s)", &[lo, hi, text]);
        } else {
            console_print(e, "TokenReference: >> %#10X", &[lo, hi]);
        }
    }
}

fn get_token_ref_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    token_ref(e, a.this_obj, a.arg1, a.result);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetTokenRef_Execute.
fn get_token_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        token_ref(e, a.this_obj, read.form(0), a.result);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// SetTokenValueOrRef.
/// Sets the token's health (-1 keeps what it has) and owner (none keeps its owner and rank; a new
/// owner drops the rank) on its first stack; a token the reference does not hold (an item with
/// health) is added first, at that health and owner. The reference is marked as changed in its
/// inventory each time.
fn set_token_value_or_ref(e: &mut Engine, reference: u32, item: u32, value: f32, owner: u32) {
    let id = e.mem.u32(item + FORM_ID);
    let Some(mut token) = item_with_health_and_owner(e, reference, id, None) else {
        return;
    };
    let mut form = token.item;
    if form == 0 {
        form = e.call(LOOKUP_FORM, &args![id]).u32();
        if form != 0 {
            if dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_HEALTH_FORM) != 0 {
                form = add_item_health_percent_owner(e, reference, id, 1, value, owner, 0);
                e.vcall(reference, MARK_AS_MODIFIED, &args![CHANGED_INVENTORY]);
            } else {
                form = 0;
            }
        }
    }
    if form == 0 || dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_HEALTH_FORM) == 0 {
        return;
    }
    let value = if value == -1.0 { token.health } else { value };
    let owner = if owner == 0 {
        token.owner
    } else {
        token.rank = 0;
        owner
    };
    let Some(set) = set_first_item(e, reference, id, 1, value, owner, token.rank) else {
        return;
    };
    e.vcall(reference, MARK_AS_MODIFIED, &args![CHANGED_INVENTORY]);
    if is_console_mode(e) && owner != 0 {
        let [lo, hi] = double_words(f64::from(value));
        let owner_id = e.mem.u32(owner + FORM_ID);
        let name = super::string::tes_full_name(e, set);
        if name != 0 {
            let text = e.mem.u32(name + 4);
            console_print(
                e,
                "SetTokenValueOrRef: >> %f [%x] (%s)",
                &[lo, hi, owner_id, text],
            );
        } else {
            console_print(e, "SetTokenValueOrRef: >> %f [%x]", &[lo, hi, owner_id]);
        }
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_SetTokenValue_Execute.
fn set_token_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        set_token_value_or_ref(e, a.this_obj, read.form(0), read.float(1, 0.0), 0);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_SetTokenRef_Execute.
fn set_token_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        set_token_value_or_ref(e, a.this_obj, read.form(0), -1.0, read.form(1));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_SetTokenValueAndRef_Execute.
fn set_token_value_and_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let (item, value, owner) = (read.form(0), read.float(1, 0.0), read.form(2));
        set_token_value_or_ref(e, a.this_obj, item, value, owner);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetPaired_Eval.
/// 1 when one of the reference's stacks of the item is owned by the actor and one of the actor's
/// is owned by the reference. Each search goes stack by stack until the owner matches or the item
/// runs out; a search that would not end (a stack list with an empty node, which xNVSE reads again
/// and again) is bounded as the list walks are.
fn paired(e: &mut Engine, reference: u32, item: u32, actor: u32) -> bool {
    if item == 0 || actor == 0 {
        return false;
    }
    let id = e.mem.u32(item + FORM_ID);
    let owned_by = |e: &mut Engine, holder: u32, owner: u32| -> Option<bool> {
        let mut index = 0i32;
        let mut walked = 0usize;
        loop {
            let token = item_with_health_and_owner(e, holder, id, Some(&mut index))?;
            if token.item == 0 || token.owner == owner {
                return Some(token.owner == owner);
            }
            if !invref::list_step(e, &mut walked) {
                return None;
            }
        }
    };
    if owned_by(e, reference, actor) != Some(true) {
        return false;
    }
    owned_by(e, actor, reference) == Some(true)
}

fn get_paired_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let v = paired(e, a.this_obj, a.arg1, a.arg2);
    e.mem.set_f64(a.result, f64::from(u8::from(v)));
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Inventory.cpp:
// Cmd_GetPaired_Execute.
fn get_paired(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let v = paired(e, a.this_obj, read.form(0), read.form(1));
        a.set_result(e, f64::from(u8::from(v)));
    }
    handled(true)
}

#[cfg(test)]
#[path = "inventory_info_tests.rs"]
mod tests;
