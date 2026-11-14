//! `functions_ln/ln_fn_actor.h` (JIP LN NVSE): an actor's merchant container, an actor base's
//! aggro radius, services, template, faction ranks, karma, combat style and body part data, a
//! creature's type, the race and effect conditions, and the equipped item data. 21 of its 22
//! commands.
//!
//! Left for later: `CopyFaceGenFrom` keeps the NPC's old appearance in JIP's
//! `s_appearanceUndoMap` to undo it later; the core track's co-save reads and writes that map,
//! but taking and applying an `AppearanceUndo` (the face values, hair, eyes, head parts) is not
//! built yet: it comes with them.
//!
//! An actor base (`TESActorBase`, JIP's header): `baseData` at +0x30 (flags +0x34, karma +0x44
//! (a float), template flags +0x4A (16 bits), template actor +0x54, the faction list +0x5C: a
//! `tList` of 8-byte `{faction, rank (signed byte)}`), `ai` at +0x90 (services +0x9C, aggro
//! behaviour +0xA3, aggro radius +0xA4); its virtual functions `GetBodyPartData` (+0x180) and
//! `GetCombatStyle` (+0x188). A creature (form type 0x2B): its type +0x12C, its body part data
//! +0x144. An NPC's race: +0x110 (its `TESRaceForm` at +0x10C, the race at +4 of that: the
//! exe's `TESRaceForm::CopyComponent` `0048B870` copies the word at +4).

use super::arrays::{create_array, Element};
use super::extra::{self, kind};
use super::forms::{ListNode, ACTOR_MAGIC_TARGET, TES_OBJECT_ARMO_VTBL};
use super::jip_fn_aux_variable::array_values;
use super::{
    heap_alloc, heap_free, is_actor, is_type, list_prepend, list_unlink, set_form_result, type_of,
    Handlers,
};
use crate::cosave::lookup_form;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::vars::Data;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x223c, get_merchant_container),
    (0x2247, ln_get_aggro_radius),
    (0x2248, ln_set_aggro_radius),
    (0x224d, get_creature_type),
    (0x2214, get_combat_style),
    (0x22e6, get_service_flag),
    (0x22e7, set_service_flag),
    (0x22f5, get_actor_template_flag),
    (0x22f6, set_actor_template_flag),
    (0x22fa, ln_set_actor_template),
    (0x2314, get_base_faction_rank),
    (0x2315, set_base_faction_rank),
    (0x2323, get_equipped_data),
    (0x2324, set_equipped_data),
    (0x2325, equip_item_data),
    (0x231f, get_base_karma),
    (0x2320, set_base_karma),
    (0x2333, get_body_part_data),
    (0x2848, set_body_part_data),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x2265, is_race_in_list, is_race_in_list_eval),
    (0x2321, actor_has_effect, actor_has_effect_eval),
];

const BASE: u32 = 0x20;
const KARMA: u32 = 0x44;
const TEMPLATE_FLAGS: u32 = 0x4a;
const TEMPLATE_ACTOR: u32 = 0x54;
const FACTIONS: u32 = 0x5c;
const SERVICES: u32 = 0x9c;
const AGGRO_BEHAVIOR: u32 = 0xa3;
const AGGRO_RADIUS: u32 = 0xa4;
const GET_BODY_PART_DATA: u32 = 0x180;
const GET_COMBAT_STYLE: u32 = 0x188;
const NPC_RACE: u32 = 0x110;
const CREATURE_TYPE: u32 = 0x12c;
const CREATURE_BODY_PART_DATA: u32 = 0x144;
const EXTRA_WORD: u32 = 0x0c;
/// `TESObjectREFR::IsCharacter` (vtable +0x218).
const IS_CHARACTER: u32 = 0x218;
/// `MagicTarget::GetEffectList` (its vtable +8).
const GET_EFFECT_LIST: u32 = 0x08;
/// `Actor::EquipItem` (`0088C650`, `thiscall`: the actor, the form, the count, the extra data
/// list, apply enchantment, lock, no message).
const EQUIP_ITEM: u32 = 0x0088_c650;
/// `kFormType_*` (JIP LN NVSE).
const TES_FACTION: u8 = 0x08;
const TES_OBJECT_WEAP: u8 = 0x28;
const TES_CREATURE: u8 = 0x2b;
const BGS_BODY_PART_DATA: u8 = 0x57;
/// More extra data types: `ExtraHealth` 0x25 (a float at +0xC), `ExtraLeveledCreature` 0x2E (the
/// actor base at +0x10: in the exe its setter `00421750` stores the reference's original leveled
/// base at +0xC, which `GetLevCreaOriginalBase` `004216F0` reads, and the chosen actor base at
/// +0x10, read by `00421720` for kills and hits), `ExtraMerchantContainer` 0x3C (+0xC),
/// `ExtraCombatStyle` 0x69 (+0xC),
/// `ExtraWeaponModFlags` 0x8D (a byte at +0xC); `ExtraWorn` 0x16 is the presence bit JIP's
/// `GetEquippedExtra` tests (byte +0xA of a list, bit 0x40).
const EXTRA_HEALTH: u32 = 0x25;
const EXTRA_LEVELED_CREATURE: u32 = 0x2e;
const EXTRA_MERCHANT_CONTAINER: u32 = 0x3c;
const EXTRA_COMBAT_STYLE: u32 = 0x69;
const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8d;
const EXTRA_WORN: u32 = 0x16;
/// A container changes entry: its extra data lists (`extendData`, a `tList*`) +0, its form +8;
/// the extra's data at +0xC, its entry list at +0 of that. In the exe: `ExtraDataList::
/// GetContainerChanges` `00418520` answers the extra's +0xC; `InventoryChanges::GetWornItem`
/// `004C8C10` walks the list at +0 of it, each 0xC-byte entry's lists from +0 and its form at +8
/// (`0044DDC0`).
const ENTRY_EXTEND: u32 = 0x0;
const ENTRY_TYPE: u32 = 0x8;
/// An armour's part mask (`bipedModel.partMask`, +0x74: `TESBipedModelForm::GetFormAsBipedModel`
/// `00480DB0` answers an armour's +0x70, `FillsBipedSlot` `00480AF0` tests the mask at +4 of it)
/// and `eSlot_Weapon`.
const ARMO_PART_MASK: u32 = 0x74;
const SLOT_WEAPON: u32 = 1 << 5;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetActorBase (asm).
/// The actor's base form, or for a base made in the game (file 0xFF) the actor base its
/// `ExtraLeveledCreature` holds (+0x10) when it holds one.
pub fn actor_base(e: &mut Engine, actor: u32) -> u32 {
    let base = e.mem.u32(actor + BASE);
    if base == 0 || e.mem.u8(base + 0xf) != 0xff {
        return base;
    }
    match extra::refr_extra(e, actor, EXTRA_LEVELED_CREATURE) {
        0 => base,
        x => match e.mem.u32(x + 0x10) {
            0 => base,
            lb => lb,
        },
    }
}

/// `actorBase || (thisObj && IS_ACTOR(thisObj) && (actorBase = thisObj->GetActorBase()))`.
fn base_or_actor(e: &mut Engine, form: u32, this: u32) -> Option<u32> {
    if form != 0 {
        return Some(form);
    }
    if this != 0 && is_actor(e, this) {
        let b = actor_base(e, this);
        return (b != 0).then_some(b);
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetMerchantContainer_Execute; nvse/GameObjects.cpp: TESObjectREFR::GetMerchantContainer.
fn get_merchant_container(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let x = extra::refr_extra(e, a.this_obj, EXTRA_MERCHANT_CONTAINER);
    if x != 0 {
        let c = e.mem.u32(x + EXTRA_WORD);
        if c != 0 {
            set_form_result(e, &a, c);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_LNGetAggroRadius_Execute.
/// The aggro radius when the aggro behaviour byte is set.
fn ln_get_aggro_radius(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(0), a.this_obj))
        .flatten()
    {
        if e.mem.u8(b + AGGRO_BEHAVIOR) != 0 {
            let r = e.mem.u32(b + AGGRO_RADIUS) as i32;
            a.set_result(e, f64::from(r));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_LNSetAggroRadius_Execute.
/// The radius (a negative one 0), the behaviour byte set when it is not 0.
fn ln_set_aggro_radius(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(1), a.this_obj))
        .flatten()
    {
        let v = (read.int(0, 0) as i32).max(0);
        e.mem.set_u8(b + AGGRO_BEHAVIOR, u8::from(v != 0));
        e.mem.set_u32(b + AGGRO_RADIUS, v as u32);
    }
    handled(true)
}

/// The NPC's race in the form list (a character only: `IsCharacter`, the virtual function).
fn race_in_list(e: &mut Engine, refr: u32, list: u32) -> bool {
    if e.vcall(refr, IS_CHARACTER, &[]).u32() & 0xff == 0 {
        return false;
    }
    let race = e.mem.u32(e.mem.u32(refr + BASE) + NPC_RACE);
    race != 0 && super::list_contains(e, Ptr::<ListNode>::new(list + 0x18), race)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_IsRaceInList_Execute.
fn is_race_in_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && race_in_list(e, a.this_obj, read.form(0)) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h: Cmd_IsRaceInList_Eval.
fn is_race_in_list_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if race_in_list(e, a.this_obj, a.arg1) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetCreatureType_Execute.
/// -1, or a creature's type (echoed in the console).
fn get_creature_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if let Some(c) = read
        .ok
        .then(|| base_or_actor(e, read.form(0), a.this_obj))
        .flatten()
    {
        if type_of(e, c) == TES_CREATURE {
            let t = e.mem.u8(c + CREATURE_TYPE);
            a.set_result(e, f64::from(t));
            crate::console::print_result(e, &a);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetCombatStyle_Execute; nvse/GameObjects.cpp: Actor::GetCombatStyle.
/// The actor base's (its virtual function), else the calling actor's: its `ExtraCombatStyle`'s
/// style, else its base form's.
fn get_combat_style(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let base = read.form(0);
    let style = if base != 0 {
        e.vcall(base, GET_COMBAT_STYLE, &[]).u32()
    } else if a.this_obj != 0 && is_actor(e, a.this_obj) {
        let x = extra::refr_extra(e, a.this_obj, EXTRA_COMBAT_STYLE);
        match if x != 0 { e.mem.u32(x + EXTRA_WORD) } else { 0 } {
            0 => {
                let b = e.mem.u32(a.this_obj + BASE);
                e.vcall(b, GET_COMBAT_STYLE, &[]).u32()
            }
            s => s,
        }
    } else {
        0
    };
    if style != 0 {
        set_form_result(e, &a, style);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetServiceFlag_Execute.
fn get_service_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && e.mem.u32(read.form(0) + SERVICES) & read.int(1, 0) != 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

/// The word at `at` with `flag` set or cleared.
fn set_bits(e: &mut Engine, at: u32, flag: u32, on: bool) {
    let v = e.mem.u32(at);
    e.mem.set_u32(at, if on { v | flag } else { v & !flag });
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_SetServiceFlag_Execute.
fn set_service_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let at = read.form(0) + SERVICES;
        set_bits(e, at, read.int(1, 0), read.int(2, 0) != 0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetActorTemplateFlag_Execute.
fn get_actor_template_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(1), a.this_obj))
        .flatten()
    {
        if u32::from(e.mem.u16(b + TEMPLATE_FLAGS)) & read.int(0, 0) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_SetActorTemplateFlag_Execute.
/// The 16-bit template flags with the flag's bits set or cleared (bits above 16 dropped).
fn set_actor_template_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(2), a.this_obj))
        .flatten()
    {
        let (flag, on) = (read.int(0, 0) as u16, read.int(1, 0) != 0);
        let v = e.mem.u16(b + TEMPLATE_FLAGS);
        e.mem
            .set_u16(b + TEMPLATE_FLAGS, if on { v | flag } else { v & !flag });
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_LNSetActorTemplate_Execute.
fn ln_set_actor_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(1), a.this_obj))
        .flatten()
    {
        let form = read.form(0);
        e.mem.set_u32(b + TEMPLATE_ACTOR, form);
    }
    handled(true)
}

/// The actor base argument, else the calling actor's base form itself (not its leveled base):
/// JIP's `actorBase || (thisObj && IS_ACTOR(thisObj) && (actorBase = thisObj->baseForm))`, so an
/// actor with no base form gives none.
fn base_or_this_base(e: &Engine, form: u32, this: u32) -> Option<u32> {
    if form != 0 {
        Some(form)
    } else if this != 0 && is_actor(e, this) {
        let b = e.mem.u32(this + BASE);
        (b != 0).then_some(b)
    } else {
        None
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESActorBaseData::GetFactionRank.
/// The rank (a signed byte) of the first entry of the faction, -1 for none.
pub(crate) fn faction_rank(e: &Engine, base: u32, faction: u32) -> i8 {
    for d in super::list_items(e, Ptr::<ListNode>::new(base + FACTIONS)) {
        if d != 0 && e.mem.u32(d) == faction {
            return e.mem.u8(d + 4) as i8;
        }
    }
    -1
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetBaseFactionRank_Execute.
/// -1, or the actor base's rank in the faction (another form type: -1).
fn get_base_faction_rank(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    let faction = read.form(0);
    if read.ok && type_of(e, faction) == TES_FACTION {
        if let Some(b) = base_or_this_base(e, read.form(1), a.this_obj) {
            let r = faction_rank(e, b, faction);
            a.set_result(e, f64::from(r));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESActorBaseData::SetFactionRank.
/// The faction's entry given the rank; a negative rank removes it (its 8 bytes freed, the node
/// unlinked); none there and a rank of 0 or more: a new 8-byte entry (faction, rank; its pad
/// left as allocated) put first.
fn set_faction_rank(e: &mut Engine, base: u32, faction: u32, rank: i8) {
    let head = Ptr::<ListNode>::new(base + FACTIONS);
    let (mut node, mut prev) = (head, Ptr::<ListNode>::NULL);
    loop {
        let d = e.get(node, ListNode::data);
        if d != 0 && e.mem.u32(d) == faction {
            if rank >= 0 {
                e.mem.set_u8(d + 4, rank as u8);
            } else {
                heap_free(e, d);
                list_unlink(e, node, prev);
            }
            return;
        }
        prev = node;
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            break;
        }
    }
    if rank >= 0 {
        let d = heap_alloc(e, 8);
        e.mem.set_u32(d, faction);
        e.mem.set_u8(d + 4, rank as u8);
        list_prepend(e, head, d);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_SetBaseFactionRank_Execute.
/// `SetFactionRank(faction, GetMax(rank, -1))`: the rank passed as a `char` (its low byte, so
/// 128 is -128 and removes the entry).
fn set_base_faction_rank(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(b) = base_or_this_base(e, read.form(2), a.this_obj) {
            let rank = (read.int(1, 0) as i32).max(-1) as i8;
            set_faction_rank(e, b, read.form(0), rank);
        }
    }
    handled(true)
}

/// `TESBipedModelForm::MaskForSlot`: slot 0 to 19 its bit, any other all bits.
fn mask_for_slot(slot: u32) -> u32 {
    if slot <= 19 {
        1 << slot
    } else {
        u32::MAX
    }
}

/// The reference's container changes data's entry list (`data->objList`), 0 for none.
pub(crate) fn changes_list(e: &mut Engine, refr: u32) -> u32 {
    match extra::refr_extra(e, refr, kind::CONTAINER_CHANGES) {
        0 => 0,
        x => match e.mem.u32(x + EXTRA_WORD) {
            0 => 0,
            data => e.mem.u32(data),
        },
    }
}

/// The items of a `tList` given by pointer (its first node), 0 for none: empty ones included.
fn items_at(e: &Engine, list: u32) -> Vec<u32> {
    if list == 0 {
        Vec::new()
    } else {
        super::list_items(e, Ptr::<ListNode>::new(list))
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntry::GetEquippedExtra (asm).
/// The entry's first extra data list with the `ExtraWorn` presence bit, 0 for none.
pub(crate) fn equipped_extra(e: &Engine, entry: u32) -> u32 {
    items_at(e, e.mem.u32(entry + ENTRY_EXTEND))
        .into_iter()
        .find(|&x| x != 0 && extra::has_type(e, x, EXTRA_WORN))
        .unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetEquippedData_Execute.
/// For the first item worn in the slot (a weapon for the weapon slot, an armour by its part
/// mask): an array of the item, its health (-1 without one) and its mod flags (0 without).
fn get_equipped_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let list = changes_list(e, a.this_obj);
    if list == 0 {
        return handled(true);
    }
    let slot = mask_for_slot(read.int(0, 0));
    for entry in items_at(e, list) {
        if entry == 0 || e.mem.u32(entry + ENTRY_EXTEND) == 0 {
            continue;
        }
        let item = e.mem.u32(entry + ENTRY_TYPE);
        if type_of(e, item) == TES_OBJECT_WEAP {
            if slot & SLOT_WEAPON == 0 {
                continue;
            }
        } else if is_type(e, item, TES_OBJECT_ARMO_VTBL) {
            if slot & e.mem.u32(item + ARMO_PART_MASK) == 0 {
                continue;
            }
        } else {
            continue;
        }
        let xdl = equipped_extra(e, entry);
        if xdl == 0 {
            continue;
        }
        let h = extra::get_by_type(e, xdl, EXTRA_HEALTH);
        let m = extra::get_by_type(e, xdl, EXTRA_WEAPON_MOD_FLAGS);
        let health = if h != 0 {
            f64::from(e.mem.f32(h + EXTRA_WORD))
        } else {
            -1.0
        };
        let flags = if m != 0 { e.mem.u8(m + EXTRA_WORD) } else { 0 };
        let els = [
            Element::Form(item),
            Element::Number(health),
            Element::Number(f64::from(flags)),
        ];
        let id = create_array(e, a.script, &els);
        a.set_result(e, f64::from(id as i32));
        break;
    }
    handled(true)
}

/// The actor's container changes entry of `form`, 0 for none.
fn changes_entry(e: &mut Engine, refr: u32, form: u32) -> u32 {
    let list = changes_list(e, refr);
    items_at(e, list)
        .into_iter()
        .find(|&en| en != 0 && e.mem.u32(en + ENTRY_TYPE) == form)
        .unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h: SetEquippedData.
/// Equips (`EquipItem(form, 1, list, 1, lock, silent)`) the first extra data list of the actor's
/// entry for the form that matches: the same health bits when a health above 0 is given, the
/// same mod flags when flags are given, else no mod flags at all.
fn set_equipped(
    e: &mut Engine,
    actor: u32,
    form: u32,
    health: f32,
    flags: u8,
    lock: bool,
    silent: bool,
) {
    let entry = changes_entry(e, actor, form);
    if entry == 0 || e.mem.u32(entry + ENTRY_EXTEND) == 0 {
        return;
    }
    for xdl in items_at(e, e.mem.u32(entry + ENTRY_EXTEND)) {
        if xdl == 0 {
            continue;
        }
        if health > 0.0 {
            let h = extra::get_by_type(e, xdl, EXTRA_HEALTH);
            if h == 0 || e.mem.u32(h + EXTRA_WORD) != health.to_bits() {
                continue;
            }
        }
        if flags != 0 {
            let m = extra::get_by_type(e, xdl, EXTRA_WEAPON_MOD_FLAGS);
            if m == 0 || e.mem.u8(m + EXTRA_WORD) != flags {
                continue;
            }
        } else if extra::has_type(e, xdl, EXTRA_WEAPON_MOD_FLAGS) {
            continue;
        }
        let item = e.mem.u32(entry + ENTRY_TYPE);
        e.call(
            EQUIP_ITEM,
            &args![
                actor,
                item,
                1u32,
                xdl,
                1u32,
                u32::from(lock),
                u32::from(silent)
            ],
        );
        break;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_SetEquippedData_Execute.
/// An array of exactly 3: the form, the health (as a float), the mod flags (as a byte).
fn set_equipped_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let (id, lock, silent) = (read.int(0, 0), read.int(1, 0), read.int(2, 1));
    let Some(vals) = array_values(e, id).filter(|v| v.len() == 3) else {
        return handled(true);
    };
    let form = match vals[0] {
        Data::Form(f) => lookup_form(e, f),
        _ => 0,
    };
    let num = |d: &Data| match d {
        Data::Number(n) => *n,
        _ => 0.0,
    };
    let health = num(&vals[1]) as f32;
    let flags = super::cvttsd2si(num(&vals[2])) as u8;
    set_equipped(e, a.this_obj, form, health, flags, lock != 0, silent != 0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_EquipItemData_Execute.
fn equip_item_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && is_actor(e, a.this_obj) {
        let health = read.float(1, 0.0);
        let (flags, lock, silent) = (read.int(2, 0), read.int(3, 0), read.int(4, 0));
        let form = read.form(0);
        set_equipped(
            e,
            a.this_obj,
            form,
            health,
            flags as u8,
            lock != 0,
            silent != 0,
        );
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetBaseKarma_Execute.
fn get_base_karma(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(0), a.this_obj))
        .flatten()
    {
        let k = e.mem.f32(b + KARMA);
        a.set_result(e, f64::from(k));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_SetBaseKarma_Execute.
/// The whole number as the float.
fn set_base_karma(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(1), a.this_obj))
        .flatten()
    {
        let k = read.int(0, 0) as i32 as f32;
        e.mem.set_f32(b + KARMA, k);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h: ActorHasEffect.
/// An actor with an active effect of the base effect that is active, not terminated and has a
/// magic item.
fn has_effect(e: &mut Engine, actor: u32, setting: u32) -> bool {
    if actor == 0 || !is_actor(e, actor) {
        return false;
    }
    let first = e
        .vcall(actor + ACTOR_MAGIC_TARGET, GET_EFFECT_LIST, &[])
        .u32();
    items_at(e, first).into_iter().any(|fx| {
        fx != 0 && {
            let item = e.mem.u32(fx + 0xc);
            e.mem.u32(item + 0x14) == setting
                && e.mem.u8(fx + 0x10) != 0
                && e.mem.u8(fx + 0x13) == 0
                && e.mem.u32(fx + 8) != 0
        }
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_ActorHasEffect_Execute.
fn actor_has_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && has_effect(e, a.this_obj, read.form(0)) {
        a.set_result(e, 1.0);
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_ActorHasEffect_Eval.
fn actor_has_effect_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if has_effect(e, a.this_obj, a.arg1) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_GetBodyPartData_Execute.
fn get_body_part_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let d = e.vcall(read.form(0), GET_BODY_PART_DATA, &[]).u32();
        if d != 0 {
            set_form_result(e, &a, d);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_actor.h:
// Cmd_SetBodyPartData_Execute.
/// A creature's body part data (an NPC or another form: nothing).
fn set_body_part_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (c, d) = (read.form(0), read.form(1));
    if read.ok && type_of(e, c) == TES_CREATURE && type_of(e, d) == BGS_BODY_PART_DATA {
        e.mem.set_u32(c + CREATURE_BODY_PART_DATA, d);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_actor_tests.rs"]
mod tests;
