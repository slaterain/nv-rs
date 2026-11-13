//! `functions_jip/jip_fn_effect.h` (JIP LN NVSE): a magic item's effects (count, base effect,
//! numbers, adding and removing), object and unarmed effects, an actor base's spells, an
//! ingestible's addiction and poison, the flags of magic items and base effects, a script
//! effect's script, an actor's active and timed effects, and the two conditions
//! `IsSpellTargetAlt` and `IsSpellTargetList`. 28 of its 29 commands: `CastImmediate` sets JIP's
//! own actor flag around the cast for its cast hook (the core track's).
//!
//! A magic item argument (`kParamType_MagicItem`) is the `MagicItem` inside the form, as xNVSE's
//! reader casts it (`DYNAMIC_CAST` to `MagicItem`): +0x18 in a spell or an enchantment, +0x30 in
//! an ingestible (`forms::MAGIC_ITEM_IN_FORM`). The game's own conditions get the same pointer
//! (its `IsSpellTarget`, `00822b90`, compares it with an active effect's +8), so the evals take
//! `arg1` as it comes.
//!
//! Changes made to effect items reach the actors already under them as JIP does it
//! (`MagicItem::UpdateEffectsAllActors`): the player and every actor of the high and middle-high
//! process levels get a fresh active effect from the caster (`MagicCaster::CreateActiveEffect`)
//! in place of the old one.
//!
//! Two deliberate differences from JIP, both leaks it has (tested):
//! - `RemoveNthEffect` deletes the removed effect item with the game's deleting destructor
//!   (`004059d0` with 1), which also frees its condition list (+0x1C, `00403d50`); JIP's
//!   `Game_HeapFree` frees only the 0x24 bytes, so the conditions of an effect with conditions
//!   stay allocated.
//! - Taking the removed item's active effects off an actor (JIP's `MagicTarget::RemoveEffect`):
//!   after the same `ActiveEffect::Remove(true)` (`00804210`), the port takes each effect out
//!   with the game's own `MagicTarget::RemoveEffect(effect, true)` (`008251c0`), which unlinks
//!   its node, reports the removal to the target and deletes the effect (through the task
//!   queue while other threads may hold it). JIP unlinks the node itself and never deletes the
//!   effect, and the game cannot reach it again to delete it.
//!
//! Where JIP would read through a null reference on ordinary input the port does nothing: the
//! reference commands and the conditions with no reference (`thisObj` 0).

use super::arrays::{create_array, Element, TempElements};
use super::forms::{
    form_type_b8, ActiveEffect, AlchemyItem, EffectItem, EffectSetting, EnchantmentItem, ListNode,
    MagicItem, ProcessManager, SpellItem, TESActorBase, TESEnchantableForm, ACTOR_MAGIC_TARGET,
    MAGIC_ITEM_IN_FORM, MAGIC_ITEM_IN_OBJECT, TES_OBJECT_ARMO_VTBL,
};
use super::{
    heap_alloc, is_actor, is_type, list_count, list_prepend, list_unlink, nth_item,
    set_form_result, type_of, Handlers,
};
use crate::game::BGSListForm;
use engine_core::prelude::*;
use nvse::commands::{dynamic_cast, rtti, Read};
use nvse::{handled, CommandArgs, EvalArgs};
use std::collections::HashSet;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x267b, get_num_effects),
    (0x267c, get_nth_effect_base),
    (0x267d, get_nth_effect_trait_numeric),
    (0x267e, set_nth_effect_trait_numeric),
    (0x267f, get_num_actor_effects),
    (0x2680, get_nth_actor_effect),
    (0x268b, set_nth_effect_base),
    (0x26f7, add_new_effect),
    (0x26f8, remove_nth_effect),
    (0x26f9, set_object_effect),
    (0x26fa, get_actor_unarmed_effect),
    (0x26fb, set_actor_unarmed_effect),
    (0x271c, get_addiction_effect),
    (0x271d, set_addiction_effect),
    (0x271e, get_addiction_chance),
    (0x271f, set_addiction_chance),
    (0x2720, get_is_poison),
    (0x2733, get_active_effects),
    (0x2739, get_temp_effects),
    (0x273a, remove_nth_temp_effect),
    (0x2776, get_effect_flag),
    (0x2777, set_effect_flag),
    (0x2778, get_base_effect_flag),
    (0x2779, set_base_effect_flag),
    (0x27c1, get_base_effect_script),
    (0x27c2, set_base_effect_script),
];

/// The conditions: each opcode with its execute and its eval.
#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x27ec, is_spell_target_alt, is_spell_target_alt_eval),
    (0x291a, is_spell_target_list, is_spell_target_list_eval),
];

const PLAYER: u32 = 0x011d_ea3c;
const PROCESS_MANAGER: u32 = 0x011e_0e80;
/// `ADDR_ReturnTrue` (JIP LN NVSE): a vtable slot holding it answers true.
const RETURN_TRUE: u32 = 0x008d_0360;
/// `ActiveEffect::Remove` (JIP LN NVSE; the exe's `ActiveEffect::Dispel`, `thiscall`: the effect
/// and whether now).
const ACTIVE_EFFECT_REMOVE: u32 = 0x0080_4210;
/// The exe's `MagicTarget::RemoveEffect` (`thiscall`: the target, the effect, whether now).
const MAGIC_TARGET_REMOVE_EFFECT: u32 = 0x0082_51c0;
/// The exe's `EffectItem` deleting destructor (`thiscall`: the item, 1 to free it).
const EFFECT_ITEM_DELETE: u32 = 0x0040_59d0;
/// Vtable slots: `MagicItem::GetType` (+0x18), `TESForm::IsActor` (+0x100), `MagicTarget`'s
/// `ApplyEffect` (+0) and `GetEffectList` (+8), `MagicCaster`'s `GetActor` (+0x2C) and
/// `CreateActiveEffect` (+0x4C), `ActiveEffect::Destroy` (+0), and the form slot JIP's
/// `GetMagicItem` compares with `ADDR_ReturnTrue` (+0xEC; it holds `008D0360` in the vtables of
/// spells, enchantments and ingestibles).
const GET_TYPE: u32 = 0x18;
const IS_ACTOR: u32 = 0x100;
const APPLY_EFFECT: u32 = 0x00;
const GET_EFFECT_LIST: u32 = 0x08;
const CASTER_GET_ACTOR: u32 = 0x2c;
const CREATE_ACTIVE_EFFECT: u32 = 0x4c;
const DESTROY: u32 = 0x00;
const IS_MAGIC_ITEM: u32 = 0xec;

type E = EffectItem;
type A = ActiveEffect;

/// The effect list (its first node) of the `MagicItem` at `magic_item`.
fn effects(magic_item: u32) -> Ptr<ListNode> {
    Ptr::<MagicItem>::new(magic_item).at(MagicItem::list)
}

fn effect_item(item: u32) -> Ptr<E> {
    Ptr::new(item)
}

fn active(effect: u32) -> Ptr<A> {
    Ptr::new(effect)
}

fn setting(e: &Engine, item: u32) -> Ptr<EffectSetting> {
    Ptr::new(e.get(effect_item(item), E::setting))
}

/// The list's next node (0 at the end).
fn next(e: &Engine, node: Ptr<ListNode>) -> Ptr<ListNode> {
    Ptr::new(e.get(node, ListNode::next))
}

/// `NOT_ACTOR` negated, with no reference counted as not an actor.
fn actor(e: &Engine, r: u32) -> Option<u32> {
    (r != 0 && is_actor(e, r)).then_some(r)
}

/// `actor->magicTarget.GetEffectList()` (the virtual function): the first node of the actor's
/// active effects, 0 for none.
fn effect_list(e: &mut Engine, actor: u32) -> Ptr<ListNode> {
    Ptr::new(
        e.vcall(actor + ACTOR_MAGIC_TARGET, GET_EFFECT_LIST, &[])
            .u32(),
    )
}

/// An active effect that is active and not terminated (`bActive && !bTerminated`).
fn running(e: &Engine, effect: u32) -> bool {
    e.get(active(effect), A::bActive) != 0 && e.get(active(effect), A::bTerminated) == 0
}

/// `DYNAMIC_CAST(magicItem, MagicItem, TESForm)`: the form holding the magic item, 0 for none.
fn magic_item_form(e: &mut Engine, magic_item: u32) -> u32 {
    dynamic_cast(e, magic_item, rtti::MAGIC_ITEM, rtti::TES_FORM)
}

/// Argument `i` when the read succeeded.
fn arg(read: &Read, i: usize) -> Option<u32> {
    read.ok.then(|| read.form(i))
}

// ---- JIP's helpers ----

/// The actors of the high and middle-high process levels, in the array's order, as JIP walks
/// them (`objects.data + beginOffsets[n]` up to `+ endOffsets[n]`; null entries included).
fn high_actors(e: &Engine) -> Vec<u32> {
    let pm = Ptr::<ProcessManager>::new(PROCESS_MANAGER);
    let data = e.get(pm, ProcessManager::objects);
    let begin = PROCESS_MANAGER + ProcessManager::beginOffsets.off;
    let end = PROCESS_MANAGER + ProcessManager::endOffsets.off;
    let mut actors = Vec::new();
    for level in 0..2 {
        let first = e.mem.u32(begin + 4 * level);
        let past = e.mem.u32(end + 4 * level);
        for i in first..past {
            actors.push(e.mem.u32(data + 4 * i));
        }
    }
    actors
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::UpdateActiveEffects.
/// The actor's first active effect of `magic_item` (with `add_new`) or the first of `item`
/// (without: a terminated one stops the search) is replaced: its caster makes a new effect for
/// `item` (`CreateActiveEffect`, with the old one's object), which without `add_new` takes the
/// old one's time, active byte and flags while the old one is removed (both are dropped when
/// the time already passed the new duration); the new one goes to the old one's target
/// (`ApplyEffect`). Only the first match is looked at.
fn update_active_effects(e: &mut Engine, actor: u32, magic_item: u32, item: u32, add_new: bool) {
    let mut node = effect_list(e, actor);
    while !node.is_null() {
        let old = e.get(node, ListNode::data);
        node = next(e, node);
        if old == 0 || e.get(active(old), A::magicItem) != magic_item {
            continue;
        }
        if !add_new {
            if e.get(active(old), A::effectItem) != item {
                continue;
            }
            if e.get(active(old), A::bTerminated) != 0 {
                break;
            }
        }
        let caster = e.get(active(old), A::caster);
        let object = e.get(active(old), A::enchantObject);
        let fresh = e
            .vcall(
                caster,
                CREATE_ACTIVE_EFFECT,
                &args![magic_item, item, object],
            )
            .u32();
        if fresh != 0 {
            let target = e.get(active(old), A::target);
            if !add_new {
                let elapsed = e.get(active(old), A::timeElapsed);
                if elapsed >= e.get(active(fresh), A::duration) {
                    e.vcall(fresh, DESTROY, &args![1u32]);
                    e.call(ACTIVE_EFFECT_REMOVE, &args![old, 1u32]);
                    break;
                }
                e.set(active(fresh), A::timeElapsed, elapsed);
                let on = e.get(active(old), A::bActive);
                e.set(active(fresh), A::bActive, on);
                let flags = e.get(active(old), A::flags);
                e.set(active(fresh), A::flags, flags);
                e.call(ACTIVE_EFFECT_REMOVE, &args![old, 1u32]);
            }
            e.set(active(fresh), A::target, target);
            e.vcall(
                target,
                APPLY_EFFECT,
                &args![caster, magic_item, fresh, 0u32],
            );
        }
        break;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// MagicItem::UpdateEffectsAllActors.
/// [`update_active_effects`] on the player, then on every actor (the virtual `IsActor`) of the
/// high and middle-high process levels.
fn update_effects_all_actors(e: &mut Engine, magic_item: u32, item: u32, add_new: bool) {
    let player = e.mem.u32(PLAYER);
    update_active_effects(e, player, magic_item, item, add_new);
    for a in high_actors(e) {
        if a != 0 && e.vcall(a, IS_ACTOR, &[]).bool() {
            update_active_effects(e, a, magic_item, item, add_new);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: MagicTarget::RemoveEffect.
/// Every active effect of `item` on the actor removed now (`ActiveEffect::Remove(true)`) and
/// taken out of the list, by the game's `MagicTarget::RemoveEffect(effect, true)` (the port's
/// difference: see the module's doc).
fn remove_effect(e: &mut Engine, actor: u32, item: u32) {
    let target = actor + ACTOR_MAGIC_TARGET;
    let mut node = effect_list(e, actor);
    let mut prev = Ptr::<ListNode>::NULL;
    while !node.is_null() {
        let effect = e.get(node, ListNode::data);
        if effect != 0 && e.get(active(effect), A::effectItem) == item {
            e.call(ACTIVE_EFFECT_REMOVE, &args![effect, 1u32]);
            e.call(MAGIC_TARGET_REMOVE_EFFECT, &args![target, effect, 1u32]);
            if !prev.is_null() {
                node = next(e, prev);
            }
        } else {
            prev = node;
            node = next(e, node);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// EffectItemList::RemoveNthEffect.
/// Effect `index` taken out of a list of two or more (nothing past the end): its active
/// effects removed from the player and the actors of the high and middle-high levels
/// (`IS_ACTOR`), its node unlinked, the item deleted (the game's destructor: the port's
/// difference, see the module's doc). Whether one was.
fn remove_nth(e: &mut Engine, head: Ptr<ListNode>, mut index: u32) -> bool {
    if nth_item(e, head, 1) == 0 {
        return false;
    }
    let (mut node, mut prev) = (head, Ptr::<ListNode>::NULL);
    loop {
        if index != 0 {
            index -= 1;
            prev = node;
        } else {
            let item = e.get(node, ListNode::data);
            if item != 0 {
                let player = e.mem.u32(PLAYER);
                remove_effect(e, player, item);
                for a in high_actors(e) {
                    if a != 0 && is_actor(e, a) {
                        remove_effect(e, a, item);
                    }
                }
            }
            list_unlink(e, node, prev);
            if item != 0 {
                e.call(EFFECT_ITEM_DELETE, &args![item, 1u32]);
            }
            return true;
        }
        node = next(e, node);
        if node.is_null() {
            return false;
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: AlchemyItem::IsPoison.
/// Every effect's base effect is detrimental (flag 4), and the last effect item has a base
/// effect; an empty list is no poison.
fn is_poison(e: &Engine, alch: u32) -> bool {
    let mut node = effects(alch + MAGIC_ITEM_IN_OBJECT);
    let mut last = 0;
    while !node.is_null() {
        let item = e.get(node, ListNode::data);
        if item != 0 {
            last = e.get(effect_item(item), E::setting);
            if last != 0 && e.get(setting(e, item), EffectSetting::effectFlags) & 4 == 0 {
                return false;
            }
        }
        node = next(e, node);
    }
    last != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h: ValidTempEffect.
/// An effect item with a duration (the unsigned word above 0) and a base effect whose
/// archetype (its low byte) is 0 (value modifier), 1 (script) with flag 0x2000, 11-13, 24 or
/// above 33.
fn valid_temp_effect(e: &Engine, item: u32) -> bool {
    if item == 0 || e.get(effect_item(item), E::duration) == 0 {
        return false;
    }
    let s = setting(e, item);
    if s.is_null() {
        return false;
    }
    let arch = e.get(s, EffectSetting::archtype);
    arch == 0
        || (arch == 1 && e.get(s, EffectSetting::effectFlags) & 0x2000 != 0)
        || (arch > 10 && arch < 14)
        || arch == 24
        || arch > 33
}

/// `tList::GetLastNode`'s index: the nodes after the first.
fn last_index(e: &Engine, head: Ptr<ListNode>) -> u32 {
    let mut index = 0;
    let mut node = next(e, head);
    while !node.is_null() {
        index += 1;
        node = next(e, node);
    }
    index
}

/// `tList::Append` (JIP LN NVSE): the item into the last node when it is empty, else into a new
/// 8-byte node after it; answers the last node's index from before.
fn append(e: &mut Engine, head: Ptr<ListNode>, item: u32) -> u32 {
    let index = last_index(e, head);
    super::list_append(e, head, item);
    index
}

// ---- a magic item's effects ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetNumEffects_Execute.
fn get_num_effects(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(m) = arg(&read, 0) {
        let count = list_count(e, effects(m));
        a.set_result(e, f64::from(count));
    }
    handled(true)
}

/// Effect `read.int(1)` of the magic item argument (0 for none).
fn nth_effect(e: &Engine, read: &Read) -> Option<(u32, u32)> {
    let m = arg(read, 0)?;
    let item = nth_item(e, effects(m), read.int(1, 0) as i32);
    (item != 0).then_some((m, item))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetNthEffectBase_Execute.
fn get_nth_effect_base(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((_, item)) = nth_effect(e, &read) {
        let s = e.get(effect_item(item), E::setting);
        if s != 0 {
            set_form_result(e, &a, s);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetNthEffectBase_Execute.
/// The base effect and its actor value (the byte, widened) set, then the actors updated.
fn set_nth_effect_base(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((m, item)) = nth_effect(e, &read) {
        let s = read.form(2);
        e.set(effect_item(item), E::setting, s);
        let av = e.get(Ptr::<EffectSetting>::new(s), EffectSetting::actorVal);
        e.set(effect_item(item), E::actorValueOrOther, u32::from(av));
        update_effects_all_actors(e, m, item, false);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetNthEffectTraitNumeric_Execute.
/// 0 magnitude, 1 area, 2 duration, 3 range, each word as a signed integer; another, nothing.
fn get_nth_effect_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((_, item)) = nth_effect(e, &read) {
        let field = match read.int(2, 0) {
            0 => Some(E::magnitude),
            1 => Some(E::area),
            2 => Some(E::duration),
            3 => Some(E::range),
            _ => None,
        };
        if let Some(f) = field {
            let v = e.get(effect_item(item), f) as i32;
            a.set_result(e, f64::from(v));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetNthEffectTraitNumeric_Execute.
/// 0 magnitude, 1 area, 2 duration set to the integer; the actors are updated, except after
/// the area (JIP returns there).
fn set_nth_effect_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(2, 0);
    if t > 2 {
        return handled(true);
    }
    if let Some((m, item)) = nth_effect(e, &read) {
        let v = read.int(3, 0);
        match t {
            0 => e.set(effect_item(item), E::magnitude, v),
            1 => {
                e.set(effect_item(item), E::area, v);
                return handled(true);
            }
            _ => e.set(effect_item(item), E::duration, v),
        }
        update_effects_all_actors(e, m, item, false);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_AddNewEffect_Execute.
/// -1 unless added. The range (0 self, 1 touch, 2 target; above 2 refused) is fixed by the
/// item's type: an object effect (6) is touch for a weapon enchantment (type 2), else self with
/// no area; an ingestible (7) self with no area; a disease (1), ability (4) or addiction (10)
/// self; self has no area. The base effect must allow the range (flag `0x10 << range`). A new
/// 0x24-byte effect item of the game's heap (no cost, no conditions) is appended (the answer:
/// the old last node's index plus one) or put first (the answer: the count), then the actors
/// get it.
fn add_new_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    let mut range = read.int(4, 0);
    if !read.ok || range > 2 {
        return handled(true);
    }
    let (m, s) = (read.form(0), read.form(1));
    let (duration, magnitude) = (read.int(2, 0), read.int(3, 0));
    let mut area = read.int(5, 0);
    let kind = e.vcall(m, GET_TYPE, &[]).u32();
    if kind == 6 {
        let ench = Ptr::<EnchantmentItem>::new(m - MAGIC_ITEM_IN_FORM);
        if e.get(ench, EnchantmentItem::enchType) == 2 {
            range = 1;
        } else {
            area = 0;
            range = 0;
        }
    } else if kind == 7 {
        area = 0;
        range = 0;
    } else {
        if kind == 1 || kind == 4 || kind == 10 {
            range = 0;
        }
        if range == 0 {
            area = 0;
        }
    }
    let s_ptr = Ptr::<EffectSetting>::new(s);
    if e.get(s_ptr, EffectSetting::effectFlags) & (16 << range) == 0 {
        return handled(true);
    }
    let item = heap_alloc(e, E::SIZE);
    let it = effect_item(item);
    e.set(it, E::magnitude, magnitude);
    e.set(it, E::area, area);
    e.set(it, E::duration, duration);
    e.set(it, E::range, range);
    let av = e.get(s_ptr, EffectSetting::actorVal);
    e.set(it, E::actorValueOrOther, u32::from(av));
    e.set(it, E::setting, s);
    e.set(it, E::cost, 0.0);
    let conditions = it.at(E::conditions);
    e.set(conditions, ListNode::data, 0);
    e.set(conditions, ListNode::next, 0);
    let head = effects(m);
    let v = if read.int(6, 0) != 0 {
        append(e, head, item) as i32 + 1
    } else {
        list_prepend(e, head, item);
        list_count(e, head) as i32
    };
    a.set_result(e, f64::from(v));
    update_effects_all_actors(e, m, item, true);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_RemoveNthEffect_Execute.
/// The count left, or -1 when nothing was removed (the index as JIP's unsigned one: a negative
/// index is past the end).
fn remove_nth_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut count = -1;
    if let Some(m) = arg(&read, 0) {
        if remove_nth(e, effects(m), read.int(1, 0)) {
            count = list_count(e, effects(m)) as i32;
        }
    }
    a.set_result(e, f64::from(count));
    handled(true)
}

// ---- object, unarmed and actor effects ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetObjectEffect_Execute.
/// For a form with an enchantment (`DYNAMIC_CAST` to `TESEnchantableForm`): an enchantment
/// (0x13) of the apparel type (3) on armour (`IS_TYPE`), of the weapon type (2) on anything
/// else, is set (1); another enchantment type, nothing; no form or another kind of form clears
/// it (1).
fn set_object_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(form) = arg(&read, 0) else {
        return handled(true);
    };
    let ench = read.form(1);
    let enchantable = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_ENCHANTABLE_FORM);
    if enchantable == 0 {
        return handled(true);
    }
    let field = Ptr::<TESEnchantableForm>::new(enchantable);
    if ench != 0 && type_of(e, ench) == form_type_b8::ENCHANTMENT_ITEM {
        let want = if is_type(e, form, TES_OBJECT_ARMO_VTBL) {
            3
        } else {
            2
        };
        if e.get(Ptr::<EnchantmentItem>::new(ench), EnchantmentItem::enchType) != want {
            return handled(true);
        }
        e.set(field, TESEnchantableForm::enchantItem, ench);
    } else {
        e.set(field, TESEnchantableForm::enchantItem, 0);
    }
    a.set_result(e, 1.0);
    handled(true)
}

/// The actor base argument `i`, or the calling actor's base form when it is left out (none
/// without an actor).
fn actor_base(e: &Engine, a: &CommandArgs, read: &Read, i: usize) -> Option<Ptr<TESActorBase>> {
    let base = read.form(i);
    if base != 0 {
        return Some(Ptr::new(base));
    }
    let r = actor(e, a.this_obj)?;
    Some(Ptr::new(e.mem.u32(r + 0x20)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetNumActorEffects_Execute.
fn get_num_actor_effects(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(base) = actor_base(e, &a, &read, 0).filter(|_| read.ok) {
        let count = list_count(e, base.at(TESActorBase::spellList));
        a.set_result(e, f64::from(count));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetNthActorEffect_Execute.
fn get_nth_actor_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(base) = actor_base(e, &a, &read, 1).filter(|_| read.ok) {
        let spell = nth_item(e, base.at(TESActorBase::spellList), read.int(0, 0) as i32);
        if spell != 0 {
            set_form_result(e, &a, spell);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetActorUnarmedEffect_Execute.
fn get_actor_unarmed_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(base) = actor_base(e, &a, &read, 0).filter(|_| read.ok) {
        let effect = e.get(base, TESActorBase::unarmedEffect);
        if effect != 0 {
            set_form_result(e, &a, effect);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetActorUnarmedEffect_Execute.
/// A weapon enchantment (0x13, type 2) or an actor effect spell (0x14, type 0) set (1);
/// anything else, nothing.
fn set_actor_unarmed_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(base) = actor_base(e, &a, &read, 1).filter(|_| read.ok) else {
        return handled(true);
    };
    let effect = read.form(0);
    let fits = match type_of(e, effect) {
        form_type_b8::ENCHANTMENT_ITEM => {
            e.get(
                Ptr::<EnchantmentItem>::new(effect),
                EnchantmentItem::enchType,
            ) == 2
        }
        form_type_b8::SPELL_ITEM => e.get(Ptr::<SpellItem>::new(effect), SpellItem::spellType) == 0,
        _ => false,
    };
    if fits {
        e.set(base, TESActorBase::unarmedEffect, effect);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// ---- ingestibles ----

/// Argument 0 when it is an ingestible (0x2F).
fn alchemy(e: &Engine, read: &Read) -> Option<Ptr<AlchemyItem>> {
    let f = arg(read, 0)?;
    (type_of(e, f) == form_type_b8::ALCHEMY_ITEM).then(|| Ptr::new(f))
}

/// Argument 0 when it is an ingestible with an addiction.
fn addictive(e: &Engine, read: &Read) -> Option<Ptr<AlchemyItem>> {
    alchemy(e, read).filter(|&p| e.get(p, AlchemyItem::withdrawalEffect) != 0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetAddictionEffect_Execute.
fn get_addiction_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = addictive(e, &read) {
        let spell = e.get(p, AlchemyItem::withdrawalEffect);
        set_form_result(e, &a, spell);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetAddictionEffect_Execute.
/// The addiction: the spell (0x14), or none for no form or another form.
fn set_addiction_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = alchemy(e, &read) {
        let spell = read.form(1);
        let spell = if spell != 0 && type_of(e, spell) == form_type_b8::SPELL_ITEM {
            spell
        } else {
            0
        };
        e.set(p, AlchemyItem::withdrawalEffect, spell);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetAddictionChance_Execute.
fn get_addiction_chance(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = addictive(e, &read) {
        let chance = e.get(p, AlchemyItem::addictionChance);
        a.set_result(e, f64::from(chance));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetAddictionChance_Execute.
fn set_addiction_chance(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = addictive(e, &read) {
        e.set(p, AlchemyItem::addictionChance, read.float(1, 0.0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetIsPoison_Execute.
fn get_is_poison(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(p) = alchemy(e, &read) {
        if is_poison(e, p.addr()) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// ---- an actor's active effects ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetActiveEffects_Execute.
/// The forms of the actor's running effects that have a magnitude above 0 or a base effect
/// without magnitude (flag 0x100), each once, sorted by address (JIP's `InsertUnique`). The
/// filter (default 7) is JIP's bit trick: each bit's form type (1 enchantment 0x13, 2 spell
/// 0x14, 4 ingestible 0x2F; 63 for a clear bit) ANDed, then a form passes when its type has all
/// the result's bits (every form when the result is 0). No array when none passes.
fn get_active_effects(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let filter = read.int(0, 7);
    let Some(r) = actor(e, a.this_obj).filter(|_| read.ok) else {
        return handled(true);
    };
    let pick = |bit: u32, kind: u8| {
        if filter & bit != 0 {
            u32::from(kind)
        } else {
            63
        }
    };
    let filter = pick(1, form_type_b8::ENCHANTMENT_ITEM)
        & pick(2, form_type_b8::SPELL_ITEM)
        & pick(4, form_type_b8::ALCHEMY_ITEM);
    let mut node = effect_list(e, r);
    if node.is_null() {
        return handled(true);
    }
    let mut tmp = TempElements::new();
    while !node.is_null() {
        let effect = e.get(node, ListNode::data);
        node = next(e, node);
        if effect == 0 || !running(e, effect) {
            continue;
        }
        let m = e.get(active(effect), A::magicItem);
        if m == 0 {
            continue;
        }
        let item = e.get(active(effect), A::effectItem);
        if !(e.get(active(effect), A::magnitude) > 0.0
            || e.get(setting(e, item), EffectSetting::effectFlags) & 0x100 != 0)
        {
            continue;
        }
        let form = magic_item_form(e, m);
        if form != 0 && (filter == 0 || u32::from(type_of(e, form)) & filter == filter) {
            tmp.insert_unique(Element::Form(form));
        }
    }
    if !tmp.is_empty() {
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetTempEffects_Execute.
/// For each running timed effect ([`valid_temp_effect`]) of a magic item form, an array: the
/// form, the base effect, the magnitude (the effect item's word as an `int` made a `float` for
/// a base effect with flag 2, else the active effect's magnitude without its sign), the time
/// left (duration less elapsed, a `float`), the caster's actor (`GetActor`, none without a
/// caster) and the duration. The arrays sorted by time left (JIP's `InsertSorted`: a later
/// equal one after the earlier), in one array; no array when there are none.
fn get_temp_effects(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some(r) = actor(e, a.this_obj) else {
        return handled(true);
    };
    let mut node = effect_list(e, r);
    let mut sorted: Vec<(u32, f32)> = Vec::new();
    while !node.is_null() {
        let effect = e.get(node, ListNode::data);
        node = next(e, node);
        if effect == 0 || !running(e, effect) {
            continue;
        }
        let m = e.get(active(effect), A::magicItem);
        let item = e.get(active(effect), A::effectItem);
        if m == 0 || !valid_temp_effect(e, item) {
            continue;
        }
        let form = magic_item_form(e, m);
        if form == 0 {
            continue;
        }
        let s = e.get(effect_item(item), E::setting);
        let duration = e.get(active(effect), A::duration);
        let left = duration - e.get(active(effect), A::timeElapsed);
        let magnitude = if e.get(setting(e, item), EffectSetting::effectFlags) & 2 != 0 {
            e.get(effect_item(item), E::magnitude) as i32 as f32
        } else {
            e.get(active(effect), A::magnitude).abs()
        };
        let caster = e.get(active(effect), A::caster);
        let by = if caster != 0 {
            e.vcall(caster, CASTER_GET_ACTOR, &[]).u32()
        } else {
            0
        };
        let values = [
            Element::Form(form),
            Element::Form(s),
            Element::Number(f64::from(magnitude)),
            Element::Number(f64::from(left)),
            Element::Form(by),
            Element::Number(f64::from(duration)),
        ];
        let id = create_array(e, a.script, &values);
        let (mut low, mut high) = (0, sorted.len());
        while low != high {
            let mid = (low + high) >> 1;
            if left < sorted[mid].1 {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        sorted.insert(low, (id, left));
    }
    if !sorted.is_empty() {
        let all: Vec<Element> = sorted.iter().map(|&(id, _)| Element::Array(id)).collect();
        let id = create_array(e, a.script, &all);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_RemoveNthTempEffect_Execute.
/// The actor's timed effect `index` (counted as [`get_temp_effects`] finds them, unsorted)
/// removed now (`ActiveEffect::Remove(true)`: the game takes it out at its next update); 1 when
/// there was one.
fn remove_nth_temp_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(r) = actor(e, a.this_obj).filter(|_| read.ok) else {
        return handled(true);
    };
    let mut index = read.int(0, 0);
    let mut node = effect_list(e, r);
    while !node.is_null() {
        let effect = e.get(node, ListNode::data);
        node = next(e, node);
        if effect == 0 || !running(e, effect) {
            continue;
        }
        let m = e.get(active(effect), A::magicItem);
        if m == 0 || !valid_temp_effect(e, e.get(active(effect), A::effectItem)) {
            continue;
        }
        if magic_item_form(e, m) == 0 {
            continue;
        }
        let at = index;
        index = index.wrapping_sub(1);
        if at == 0 {
            e.call(ACTIVE_EFFECT_REMOVE, &args![effect, 1u32]);
            a.set_result(e, 1.0);
            break;
        }
    }
    handled(true)
}

// ---- flags and scripts ----

/// The flags byte of a spell (+0x40), an enchantment (+0x40) or an ingestible (+0xBC).
fn flags_byte(e: &Engine, form: u32) -> Option<u32> {
    let off = match type_of(e, form) {
        form_type_b8::SPELL_ITEM => SpellItem::spellFlags.off,
        form_type_b8::ENCHANTMENT_ITEM => EnchantmentItem::enchFlags.off,
        form_type_b8::ALCHEMY_ITEM => AlchemyItem::alchFlags.off,
        _ => return None,
    };
    Some(form + off)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetEffectFlag_Execute.
/// Flag 0-7 of a spell's, enchantment's or ingestible's flags byte.
fn get_effect_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(at) = arg(&read, 0)
        .filter(|_| flag <= 7)
        .and_then(|f| flags_byte(e, f))
    {
        if e.mem.u8(at) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetEffectFlag_Execute.
fn set_effect_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(at) = arg(&read, 0)
        .filter(|_| flag <= 7)
        .and_then(|f| flags_byte(e, f))
    {
        let bit = 1u8 << flag;
        let b = e.mem.u8(at);
        let b = if read.int(2, 0) != 0 {
            b | bit
        } else {
            b & !bit
        };
        e.mem.set_u8(at, b);
    }
    handled(true)
}

/// Argument 0 when it is a base effect (0x10).
fn base_effect(e: &Engine, read: &Read) -> Option<Ptr<EffectSetting>> {
    let f = arg(read, 0)?;
    (type_of(e, f) == form_type_b8::EFFECT_SETTING).then(|| Ptr::new(f))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetBaseEffectFlag_Execute.
/// Flag 0-31 of a base effect's flags.
fn get_base_effect_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(s) = base_effect(e, &read).filter(|_| flag <= 31) {
        if e.get(s, EffectSetting::effectFlags) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetBaseEffectFlag_Execute.
fn set_base_effect_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(s) = base_effect(e, &read).filter(|_| flag <= 31) {
        let flags = e.get(s, EffectSetting::effectFlags);
        let bit = 1u32 << flag;
        let flags = if read.int(2, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(s, EffectSetting::effectFlags, flags);
    }
    handled(true)
}

/// Argument 0 when it is a script effect's base effect (archetype 1).
fn script_effect(e: &Engine, read: &Read) -> Option<Ptr<EffectSetting>> {
    base_effect(e, read).filter(|&s| e.get(s, EffectSetting::archtype) == 1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_GetBaseEffectScript_Execute.
fn get_base_effect_script(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(s) = script_effect(e, &read) {
        let script = e.get(s, EffectSetting::associatedItem);
        if script != 0 {
            set_form_result(e, &a, script);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_SetBaseEffectScript_Execute.
/// The script (0x11) of a script effect's base effect.
fn set_base_effect_script(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(s) = script_effect(e, &read) {
        let script = read.form(1);
        if type_of(e, script) == form_type_b8::SCRIPT {
            e.set(s, EffectSetting::associatedItem, script);
        }
    }
    handled(true)
}

// ---- the conditions ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h: IsSpellTargetAlt.
/// An actor with a running effect of `magic_item` (the port: no reference, no actor).
fn spell_target_alt(e: &mut Engine, r: u32, magic_item: u32) -> bool {
    let Some(r) = actor(e, r) else {
        return false;
    };
    let mut node = effect_list(e, r);
    while !node.is_null() {
        let effect = e.get(node, ListNode::data);
        if effect != 0 && e.get(active(effect), A::magicItem) == magic_item && running(e, effect) {
            return true;
        }
        node = next(e, node);
    }
    false
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_IsSpellTargetAlt_Execute.
fn is_spell_target_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(m) = arg(&read, 0) {
        if spell_target_alt(e, a.this_obj, m) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_IsSpellTargetAlt_Eval.
/// 1 written when true; the result is left alone otherwise.
fn is_spell_target_alt_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if spell_target_alt(e, a.this_obj, a.arg1) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetMagicItem (asm).
/// The form's `MagicItem` when its vtable's +0xEC holds `ADDR_ReturnTrue`: +0x30 for a form
/// type above 0x14 (signed byte compare), else +0x18; 0 otherwise.
fn magic_item_of(e: &Engine, form: u32) -> u32 {
    let vtbl = e.mem.u32(form);
    if e.mem.u32(vtbl + IS_MAGIC_ITEM) != RETURN_TRUE {
        return 0;
    }
    if type_of(e, form) as i8 > form_type_b8::SPELL_ITEM as i8 {
        form + MAGIC_ITEM_IN_OBJECT
    } else {
        form + MAGIC_ITEM_IN_FORM
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h: IsSpellTargetList.
/// An actor with a running effect of any magic item in the list (its forms' `MagicItem`s; no
/// effect list, or a list with none, is false).
fn spell_target_list(e: &mut Engine, r: u32, list: u32) -> bool {
    let Some(r) = actor(e, r) else {
        return false;
    };
    let mut effect_node = effect_list(e, r);
    if effect_node.is_null() {
        return false;
    }
    let mut items = HashSet::new();
    let mut node = Ptr::<BGSListForm>::new(list).at(BGSListForm::list);
    while !node.is_null() {
        let form = e.get(node, ListNode::data);
        if form != 0 {
            let m = magic_item_of(e, form);
            if m != 0 {
                items.insert(m);
            }
        }
        node = next(e, node);
    }
    if items.is_empty() {
        return false;
    }
    while !effect_node.is_null() {
        let effect = e.get(effect_node, ListNode::data);
        if effect != 0 && running(e, effect) && items.contains(&e.get(active(effect), A::magicItem))
        {
            return true;
        }
        effect_node = next(e, effect_node);
    }
    false
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_IsSpellTargetList_Execute.
fn is_spell_target_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(list) = arg(&read, 0) {
        if spell_target_list(e, a.this_obj, list) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_effect.h:
// Cmd_IsSpellTargetList_Eval.
/// 1 written when there is a list and it is true; the result is left alone otherwise.
fn is_spell_target_list_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.arg1 != 0 && spell_target_list(e, a.this_obj, a.arg1) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_effect_tests.rs"]
mod tests;
