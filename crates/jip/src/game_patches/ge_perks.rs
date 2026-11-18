//! Group GE's O18 (`uNPCPerks`, JIP's switch 18): NPCs keep perks of their own. JIP's
//! `NPCPerksInfo` per actor (its ranks by perk and its 24 entry point lists), the four routines it
//! writes into the `Character` and `Creature` vtables (set and remove a perk, a perk's rank, an
//! entry point's list), the starting perks (`uNPCPerks=2`), the lists made when an actor's 3D is
//! set up or a save is loaded, the entry dropped with the actor, and the co-save's walk at a save
//! (`cosave.rs`); the player's two routines (set and remove a perk, for the player or the
//! teammates), JIP's rewrite of `Actor::SetPlayerTeammate` (`0x8BCA90`) and its three entry point
//! patches (`0x5E592F`, `0x64570F`, `0x89B455`). JIP's fourth write there (`0x89B50B`) puts back
//! the five bytes the exe already has: nothing to port.
//!
//! The ranks live in [`crate::cosave::NpcPerksMap`] (JIP's `s_NPCPerksInfoMap`, by actor id); an
//! actor's entry point lists are a guest block of 24 `tList` heads, since the game's
//! `HandleEntryPoint` walks the list the routine answers. The routines run on any thread that asks
//! an actor's perks: a change takes the map's lock for itself alone, never across a game call;
//! the rank and list reads take no lock ([`crate::cosave::npc_perks_read`]).
//!
//! The switch goes on at `DeferredInit` ([`switch_on`], as JIP's `s_deferrSetOptional`): until
//! then (and with `uNPCPerks` off, or JIP's row off) every seam and the wrap leave the game as it
//! is.

use crate::commands::forms::ListNode;
use crate::commands::list_remove_all;
use crate::commands::{heap_alloc, heap_free, list_append, list_contains, list_items, list_remove};
use crate::commands::{list_prepend, list_unlink};
use crate::cosave::{mark_changed, npc_perks, npc_perks_read};
use engine_core::abi::AbiFn;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::game_patches::slots::{routine, write_slot, Group};

/// JIP's `kValidEntryPoints`: an entry point's list among the actor's 24, or -1 for the entry
/// points NPCs do not get (74 entry points).
pub const VALID_ENTRY_POINTS: [i8; 74] = [
    0, 1, 2, -1, 3, -1, 4, -1, -1, -1, -1, -1, 5, -1, 6, -1, -1, -1, -1, -1, -1, -1, 7, 8, -1, 9,
    -1, -1, -1, -1, -1, -1, -1, -1, 10, -1, 11, 12, 13, -1, -1, -1, 14, 15, -1, -1, 16, -1, -1, -1,
    -1, -1, 17, -1, -1, 18, 19, -1, 20, 21, -1, -1, -1, -1, -1, -1, -1, -1, 22, -1, -1, -1, 23, -1,
];
/// The lists of one actor (`NPCPerkEntryPoints`: 24 `tList` heads of 8 bytes).
const LISTS: u32 = 24;
const BLOCK_SIZE: u32 = LISTS * 8;

/// The four vtable slots of `Character` (`0x1086F04`..) and `Creature` (`0x1087544`..): set rank
/// (`+0`), remove (`+4`), rank (`+8`), entry point list (`+0x14`).
pub const CHARACTER_SLOTS: [u32; 4] = [0x0108_6f04, 0x0108_6f08, 0x0108_6f0c, 0x0108_6f18];
pub const CREATURE_SLOTS: [u32; 4] = [0x0108_7544, 0x0108_7548, 0x0108_754c, 0x0108_7558];
/// Their routines (group GE's block, from 8).
const ROUTINES: [u32; 4] = [
    routine(Group::GE, 8),
    routine(Group::GE, 9),
    routine(Group::GE, 10),
    routine(Group::GE, 11),
];

/// `PlayerCharacter`'s two slots (`0x108AED4` set a perk's rank, `0x108AED8` remove a perk; the
/// exe's `00963790` and `00963900`) and their routines.
pub const PLAYER_SLOTS: [u32; 2] = [0x0108_aed4, 0x0108_aed8];
const PLAYER_ROUTINES: [u32; 2] = [routine(Group::GE, 12), routine(Group::GE, 13)];

/// O18's seam in `Actor::~Actor` (`0087dcb0`) at `0x87DE3F`, in place of `call 004077c0` (the
/// deleted-flag test; words: the actor). JIP's `DestroyActorHook` drops the actor's entry and does
/// the same test; here the handler drops the entry and gives no answer, so the exe's test runs.
pub const DESTROY_ACTOR: SiteSlot = SiteSlot::new(Owner::Jip, 0x0087_de3f);
/// O18's seam in `BGSEntryPoint::HandleEntryPoint` (`005e58f0`) at `0x5E592F`: a teammate's
/// entries are asked of the player (`mov eax, [player]; mov [ebp+0xC], eax`); JIP's 8-byte nop
/// keeps the teammate (with the teammate flag still set). Words: the actor; an answer keeps it.
pub const TEAMMATE_OWN_ENTRIES: SiteSlot = SiteSlot::new(Owner::Jip, 0x005e_592f);
/// O18's seam in the weapon damage per second (`00645380`) at `0x64570F`: the entry point 0
/// (`HandleEntryPoint`) runs only when the caller asks for perks (`test ecx, ecx; jz`); JIP's
/// 4-byte nop runs it always. Asked when the caller did not ask; words: the actor; an answer runs
/// it.
pub const WEAPON_DAMAGE_PERKS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0064_570f);
/// O18's seam in `Actor::HitMe` (`0089a760`) at `0x89B455`: the kill effect (entry point 0x2E,
/// for a kill with a weapon of kind 0x22) looks at the player's current weapon; JIP's 32 bytes
/// look at the attacker's. Words: the attacker; the answer is the weapon's form (0 for none).
pub const KILL_EFFECT_WEAPON: SiteSlot = SiteSlot::new(Owner::Jip, 0x0089_b455);
/// O18's wrap of `Actor::SetPlayerTeammate` (`008bca90`, `thiscall`: the actor, the flag):
/// JIP's `WriteRelJump(0x8BCA90, SetPlayerTeammateHook)` replaces the function.
pub const SET_PLAYER_TEAMMATE: WrapSlot = WrapSlot::new(Owner::Jip, 0x008b_ca90);

/// A form's type byte (`+4`), a reference's id (`+0xC`) and base form (`+0x20`), its render state
/// (`+0x64`) and that state's node (`+0x14`); `Character`'s type.
const FORM_TYPE: u32 = 4;
const REF_ID: u32 = 0xc;
const REF_BASE_FORM: u32 = 0x20;
const REF_RENDER_STATE: u32 = 0x64;
const RENDER_STATE_NODE: u32 = 0x14;
const CHARACTER: u8 = 0x3b;
/// An actor's life state (`+0x108`) and teammate byte (`+0x18D`).
const ACTOR_LIFE_STATE: u32 = 0x108;
const ACTOR_IS_TEAMMATE: u32 = 0x18d;
/// `g_thePlayer`, its teammates' perk ranks (`perkRanksTM`, `+0xAD4`: `PerkRank` items, the perk
/// at `+0`, the rank at `+4`) and its teammates' entry point lists (`+0xADC`, 8 bytes each).
const THE_PLAYER: u32 = 0x011d_ea3c;
const PLAYER_PERK_RANKS_TM: u32 = 0xad4;
const PLAYER_TM_ENTRY_LISTS: u32 = 0xadc;
/// The player's own perk ranks (`perkRanksPC`, `+0x87C`), its teammates (`+0x5FC`) and their
/// count (`+0xD68`).
const PLAYER_PERK_RANKS_PC: u32 = 0x87c;
const PLAYER_TEAMMATES: u32 = 0x5fc;
const PLAYER_TEAMMATE_COUNT: u32 = 0xd68;
/// `BGSAbilityPerkEntry`'s vtable (JIP's `IS_TYPE(entry, BGSAbilityPerkEntry)`; its `GetType`
/// `00978f30` answers 1).
const ABILITY_ENTRY_VTABLE: u32 = 0x0104_6c44;
/// The stats menu's refresh (`007dd710`, `cdecl`, no arguments).
const REFRESH_STATS_MENU: u32 = 0x007d_d710;
/// A process's current weapon (virtual `+0x148`: its inventory entry, the form at `+8`) and an
/// actor's process (`+0x68`).
const ACTOR_PROCESS: u32 = 0x68;
const VT_CURRENT_WEAPON: u32 = 0x148;
const INVENTORY_ENTRY_FORM: u32 = 8;
/// `g_dataHandler` and its perk list (`+0x178`).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const PERK_LIST: u32 = 0x178;
/// `BGSPerk`: `data` (trait `+0x38`, minimum level `+0x39`, ranks `+0x3A`, playable `+0x3B`,
/// hidden `+0x3C`) and its entries (`+0x48`).
const PERK_IS_TRAIT: u32 = 0x38;
const PERK_MIN_LEVEL: u32 = 0x39;
const PERK_NUM_RANKS: u32 = 0x3a;
const PERK_PLAYABLE: u32 = 0x3b;
const PERK_HIDDEN: u32 = 0x3c;
const PERK_ENTRIES: u32 = 0x48;
/// `BGSPerkEntry`: rank (`+4`), its virtual `GetType` (`+0x10`: 0 quest, 1 ability, 2 entry point),
/// `AddEntry` (`+0x30`) and `RemoveEntry` (`+0x34`) (`thiscall`: the actor, for teammates); an
/// entry point entry's entry point (`+8`), an ability entry's spell (`+8`).
const ENTRY_RANK: u32 = 4;
const VT_GET_TYPE: u32 = 0x10;
const VT_ADD_ENTRY: u32 = 0x30;
const VT_REMOVE_ENTRY: u32 = 0x34;
const ENTRY_POINT: u32 = 8;
const ENTRY_ABILITY: u32 = 8;
/// A spell's effects (`magicItem.list.list`: `MagicItem` at `+0x18`, its `EffectItemList` at
/// `+0xC`, the list after the vtable), an effect's setting (`+0x14`), a setting's archetype
/// (`+0x98`; 1 is the script archetype).
const SPELL_EFFECTS: u32 = 0x28;
const EFFECT_SETTING: u32 = 0x14;
const SETTING_ARCHTYPE: u32 = 0x98;
const ARCHTYPE_SCRIPT: u8 = 1;
/// `kPerkEntry_ReloadSpeed`, and `Actor`'s reload speed update (`008c17c0`, `thiscall`).
const RELOAD_SPEED: u8 = 25;
const UPDATE_RELOAD_SPEED: u32 = 0x008c_17c0;
/// `Actor::GetLevel` (JIP): `0047ded0` on the base form's `+0x30`, a signed 16-bit level.
const GET_LEVEL: u32 = 0x0047_ded0;
const LEVEL_DATA: u32 = 0x30;

/// JIP's `s_validNPCPerks` and `s_validNPCTraits` (made when the switch goes on) and
/// `s_NPCPerksAutoAdd` (`uNPCPerks=2`): lock-free ([`Engine::ext_free`], made at install before the
/// threads start).
#[derive(Default)]
pub struct NpcPerkLists {
    lists: OnceLock<(Vec<u32>, Vec<u32>)>,
    auto_add: AtomicBool,
}

fn npc_perks_on(e: &mut Engine) -> bool {
    e.jip_patches() & engine_core::extend::jip_patches::NPC_PERKS != 0
}

/// The actor's 3D (`TESObjectREFR::GetRefNiNode`: its render state's node, 0 without one).
pub fn ref_node(e: &mut Engine, actor: u32) -> u32 {
    let render = e.mem.u32(actor + REF_RENDER_STATE);
    if render == 0 {
        0
    } else {
        e.mem.u32(render + RENDER_STATE_NODE)
    }
}

/// The list of `entry_point` in an actor's block (`NPCPerkEntryPoints::operator[]`), none for an
/// entry point NPCs do not get (or past the table: none in the game).
fn entry_list(block: u32, entry_point: u8) -> Option<Ptr<ListNode>> {
    let index = *VALID_ENTRY_POINTS.get(usize::from(entry_point))?;
    (index >= 0).then(|| Ptr::new(block + 8 * index as u32))
}

/// `NPCPerkEntryPoints::Create`: a zeroed block of the 24 lists.
fn create_entry_points(e: &mut Engine) -> u32 {
    let block = heap_alloc(e, BLOCK_SIZE);
    for i in 0..BLOCK_SIZE / 4 {
        e.mem.set_u32(block + 4 * i, 0);
    }
    block
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: NPCPerkEntryPoints::Destroy.
/// Each of the 24 lists emptied (`RemoveAll`) and the block freed.
pub fn destroy_entry_points(e: &mut Engine, block: u32) {
    for i in 0..LISTS {
        list_remove_all(e, Ptr::new(block + 8 * i));
    }
    heap_free(e, block);
}

fn entry_type(e: &mut Engine, entry: u32) -> u32 {
    e.vcall(entry, VT_GET_TYPE, &args![]).u32()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: BuildValidNPCPerks.
/// The data's perks NPCs may get: playable, not hidden, with entries, none of them empty or a
/// quest entry, every ability's spell with effects all of a non-script archetype, and at least one
/// entry point NPCs get. Traits and perks apart, in the data's order.
pub fn build_valid_npc_perks(e: &mut Engine) -> (Vec<u32>, Vec<u32>) {
    let (mut perks, mut traits) = (Vec::new(), Vec::new());
    let handler = e.mem.u32(DATA_HANDLER);
    for perk in list_items(e, Ptr::new(handler + PERK_LIST)) {
        if perk == 0
            || e.mem.u32(perk + PERK_ENTRIES) == 0
            || e.mem.u8(perk + PERK_PLAYABLE) == 0
            || e.mem.u8(perk + PERK_HIDDEN) != 0
        {
            continue;
        }
        let mut valid = false;
        let mut invalid = false;
        for entry in list_items(e, Ptr::new(perk + PERK_ENTRIES)) {
            if entry == 0 {
                invalid = true;
                break;
            }
            match entry_type(e, entry) {
                1 => {
                    let spell = e.mem.u32(entry + ENTRY_ABILITY);
                    let effects_ok = spell != 0
                        && list_items(e, Ptr::new(spell + SPELL_EFFECTS))
                            .into_iter()
                            .all(|effect| {
                                let setting = if effect == 0 {
                                    0
                                } else {
                                    e.mem.u32(effect + EFFECT_SETTING)
                                };
                                setting != 0
                                    && e.mem.u8(setting + SETTING_ARCHTYPE) != ARCHTYPE_SCRIPT
                            });
                    if !effects_ok {
                        invalid = true;
                        break;
                    }
                }
                2 => {
                    let point = e.mem.u8(entry + ENTRY_POINT);
                    valid |= entry_list(0, point).is_some();
                }
                _ => {
                    invalid = true;
                    break;
                }
            }
        }
        if invalid || !valid {
            continue;
        }
        if e.mem.u8(perk + PERK_IS_TRAIT) != 0 {
            traits.push(perk);
        } else {
            perks.push(perk);
        }
    }
    (perks, traits)
}

fn random_uint(e: &mut Engine, bound: u32) -> u32 {
    e.call(
        crate::game_patches::gh1::GET_RANDOM_INT,
        &args![crate::game_patches::gh1::GAME_RNG, bound],
    )
    .u32()
}

fn has_entry(e: &mut Engine, id: u32) -> bool {
    npc_perks(e).get(&id).is_some()
}

/// The entry of `id` made empty, or emptied when it was there (`Insert`, then `Reset`: its ranks
/// cleared and its lists freed).
fn insert_or_reset(e: &mut Engine, id: u32) {
    let block = {
        let mut map = npc_perks(e);
        let entry = map.emplace(id);
        entry.ranks.clear();
        std::mem::take(&mut entry.entry_points)
    };
    if block != 0 {
        destroy_entry_points(e, block);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: AddStartingPerks.
/// With `uNPCPerks=2`, for a `Character`: its entry made (or reset, when it had none of its own
/// yet), a random trait at rank 1, and from level 3 random ranks of the perks of a minimum level up
/// to one and a half times its level (each rank one ticket), a third of the level many (10 from
/// level 30), all of them when there are no more. Then, with an entry, the data marked changed.
/// Answers whether the actor has an entry. Difference from JIP: with no trait at all JIP reads
/// past its empty list; here no trait is given.
pub fn add_starting_perks(e: &mut Engine, actor: u32, had_entry: bool) -> bool {
    let id = e.mem.u32(actor + REF_ID);
    let mut has = had_entry;
    let lists = e.ext_free::<NpcPerkLists>();
    let auto_add = lists.auto_add.load(Ordering::Relaxed);
    let (perks, traits) = lists.lists.get().cloned().unwrap_or_default();
    if auto_add && e.mem.u8(actor + FORM_TYPE) == CHARACTER {
        if !has {
            insert_or_reset(e, id);
            has = true;
        }
        if !traits.is_empty() {
            let trait_perk = traits[random_uint(e, traits.len() as u32) as usize];
            npc_perks(e)
                .emplace(id)
                .ranks
                .entry(trait_perk)
                .or_insert(1);
        }
        let base = e.mem.u32(actor + REF_BASE_FORM);
        let mut level = e.call(GET_LEVEL, &args![base + LEVEL_DATA]).u32() as u16 as i16 as u32;
        if level >= 3 {
            let min_level = (level.wrapping_mul(3) >> 1) as u8;
            let mut pick = Vec::new();
            for &perk in &perks {
                if e.mem.u8(perk + PERK_MIN_LEVEL) > min_level {
                    continue;
                }
                for _ in 0..e.mem.u8(perk + PERK_NUM_RANKS) {
                    pick.push(perk);
                }
            }
            let size = pick.len() as u32;
            if size != 0 {
                level = if level >= 30 { 10 } else { level / 3 };
                if level < size {
                    // `ArrayUtils::Shuffle`: each place from the first swapped with a random
                    // later one (or kept on 0).
                    for i in 0..pick.len().saturating_sub(1) {
                        let r = random_uint(e, (pick.len() - i) as u32) as usize;
                        if r != 0 {
                            pick.swap(i, i + r);
                        }
                    }
                } else {
                    level = size;
                }
                let mut map = npc_perks(e);
                let ranks = &mut map.emplace(id).ranks;
                for &perk in pick[..level as usize].iter().rev() {
                    let rank = ranks.entry(perk).or_insert(0);
                    *rank = rank.wrapping_add(1);
                }
            }
        }
    }
    if has {
        mark_changed(e, crate::cosave::changed::NPC_PERKS);
    }
    has
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: AddPerkEntriesNPC.
/// The perk's entries at `new_rank` (its rank less one) on the actor: an entry point entry of that
/// rank added to its list (once), of another rank taken out of it (only for the entry points NPCs
/// get); an ability entry added or removed by its rank (its `AddEntry`/`RemoveEntry`); quest
/// entries passed over. A reload speed entry added or taken out updates the reload speed.
fn add_perk_entries_npc(e: &mut Engine, actor: u32, perk: u32, new_rank: u8, block: u32) {
    let rank = new_rank.wrapping_sub(1);
    let mut reload = false;
    for entry in list_items(e, Ptr::new(perk + PERK_ENTRIES)) {
        if entry == 0 {
            continue;
        }
        let kind = entry_type(e, entry);
        if kind == 0 {
            continue;
        }
        if kind == 2 {
            let point = e.mem.u8(entry + ENTRY_POINT);
            if let Some(list) = entry_list(block, point) {
                if e.mem.u8(entry + ENTRY_RANK) != rank {
                    if !list_remove(e, list, entry) {
                        continue;
                    }
                } else if list_contains(e, list, entry) {
                    continue;
                } else {
                    list_append(e, list, entry);
                }
                reload |= point == RELOAD_SPEED;
            }
        } else if e.mem.u8(entry + ENTRY_RANK) != rank {
            e.vcall(entry, VT_REMOVE_ENTRY, &args![actor, 0u32]);
        } else {
            e.vcall(entry, VT_ADD_ENTRY, &args![actor, 0u32]);
        }
    }
    if reload {
        e.call(UPDATE_RELOAD_SPEED, &args![actor]);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetPerkRankNPCHook.
/// The actor's `SetPerkRank` (`thiscall`: the perk, the new rank, for teammates): not for a dead
/// actor; an actor with no entry gets one (its starting perks too); a perk it has not or at a lower
/// rank takes the new rank (the data marked changed) and its entries are put on, the lists made
/// first when the actor has none (only with its 3D loaded; without it the rank alone is kept).
fn set_perk_rank_npc(e: &mut Engine, w: &[u32]) -> Ret {
    let (actor, perk, new_rank) = (w[0], w[1], w[2] as u8);
    if e.mem.u32(actor + ACTOR_LIFE_STATE) != 0 {
        return Ret::default();
    }
    let id = e.mem.u32(actor + REF_ID);
    if !has_entry(e, id) {
        insert_or_reset(e, id);
        add_starting_perks(e, actor, true);
    }
    let block = {
        let mut map = npc_perks(e);
        // Looked at first without writing: a rank that stays changes nothing, not even the
        // map's version (the lock-free reads' copies stay current).
        let had = map.get(&id).and_then(|p| p.ranks.get(&perk).copied());
        if had.is_some_and(|r| r >= new_rank) {
            return Ret::default();
        }
        let entry = map.emplace(id);
        entry.ranks.insert(perk, new_rank);
        entry.entry_points
    };
    mark_changed(e, crate::cosave::changed::NPC_PERKS);
    let block = if block != 0 {
        block
    } else {
        if ref_node(e, actor) == 0 {
            return Ret::default();
        }
        let block = create_entry_points(e);
        npc_perks(e).emplace(id).entry_points = block;
        block
    };
    add_perk_entries_npc(e, actor, perk, new_rank, block);
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RemovePerkNPCHook.
/// The actor's `RemovePerk` (`thiscall`: the perk, for teammates): a perk it has is taken off (the
/// data marked changed): its entry point entries out of their lists (a reload speed one updates the
/// reload speed), its ability entries removed. With no perk left the actor's entry goes.
fn remove_perk_npc(e: &mut Engine, w: &[u32]) -> Ret {
    let (actor, perk) = (w[0], w[1]);
    let id = e.mem.u32(actor + REF_ID);
    let block = {
        let mut map = npc_perks(e);
        // Looked at first without writing: a perk the actor lacks changes nothing, not even the
        // map's version.
        if !map.get(&id).is_some_and(|p| p.ranks.contains_key(&perk)) {
            return Ret::default();
        }
        let Some(entry) = map.get_mut(&id) else {
            return Ret::default();
        };
        entry.ranks.remove(&perk);
        entry.entry_points
    };
    mark_changed(e, crate::cosave::changed::NPC_PERKS);
    let mut reload = false;
    for entry in list_items(e, Ptr::new(perk + PERK_ENTRIES)) {
        if entry == 0 {
            continue;
        }
        let kind = entry_type(e, entry);
        if kind == 0 {
            continue;
        }
        if kind == 2 {
            let point = e.mem.u8(entry + ENTRY_POINT);
            if block != 0 {
                if let Some(list) = entry_list(block, point) {
                    if list_remove(e, list, entry) && point == RELOAD_SPEED {
                        reload = true;
                    }
                }
            }
        } else {
            e.vcall(entry, VT_REMOVE_ENTRY, &args![actor, 0u32]);
        }
    }
    if reload {
        e.call(UPDATE_RELOAD_SPEED, &args![actor]);
    }
    let gone = {
        let mut map = npc_perks(e);
        match map.get(&id) {
            Some(entry) if entry.ranks.is_empty() => {
                let block = entry.entry_points;
                map.erase(&id);
                Some(block)
            }
            _ => None,
        }
    };
    if let Some(block) = gone.filter(|&b| b != 0) {
        destroy_entry_points(e, block);
    }
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetPerkRankNPCHook.
/// The actor's `GetPerkRank` (`thiscall`: the perk, for teammates; the second word is not read):
/// its own rank of the perk; when that is 0 and it is a teammate, the player's teammates' rank of
/// it (the first `PerkRank` of the perk in `perkRanksTM`), else 0.
fn get_perk_rank_npc(e: &mut Engine, w: &[u32]) -> Ret {
    let (actor, perk) = (w[0], w[1]);
    let id = e.mem.u32(actor + REF_ID);
    let own = npc_perks_read(e, |s| s.rank(id, perk)).unwrap_or(0);
    let mut rank = u32::from(own);
    if own == 0 && e.mem.u8(actor + ACTOR_IS_TEAMMATE) != 0 {
        let player = e.mem.u32(THE_PLAYER);
        rank = 0;
        for item in list_items(e, Ptr::new(player + PLAYER_PERK_RANKS_TM)) {
            if item != 0 && e.mem.u32(item) == perk {
                rank = u32::from(e.mem.u8(item + 4));
                break;
            }
        }
    }
    Ret {
        eax: rank,
        ..Ret::default()
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetPerkEntryPointListNPCHook.
/// The actor's `GetPerkEntryPointList` (`thiscall`: the entry point, for teammates; the second word
/// is not read): its own list of the entry point when it has one with an entry; else, for a
/// teammate, the player's teammates' list of it when that has an entry; else none.
fn get_perk_entry_point_list_npc(e: &mut Engine, w: &[u32]) -> Ret {
    let (actor, point) = (w[0], w[1] as u8);
    let id = e.mem.u32(actor + REF_ID);
    let block = npc_perks_read(e, |s| s.entry_points(id)).unwrap_or(0);
    let own = if block == 0 {
        None
    } else {
        entry_list(block, point).filter(|list| e.mem.u32(list.addr()) != 0)
    };
    let eax = match own {
        Some(list) => list.addr(),
        None if e.mem.u8(actor + ACTOR_IS_TEAMMATE) != 0 => {
            let player = e.mem.u32(THE_PLAYER);
            let list = player + PLAYER_TM_ENTRY_LISTS + 8 * u32::from(point);
            if e.mem.u32(list) != 0 {
                list
            } else {
                0
            }
        }
        None => 0,
    };
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The perk's ability entries (`IS_TYPE(entry, BGSAbilityPerkEntry)`: the vtable's word, not
/// `GetType`), empty items passed over.
fn ability_entries(e: &mut Engine, perk: u32) -> Vec<u32> {
    list_items(e, Ptr::new(perk + PERK_ENTRIES))
        .into_iter()
        .filter(|&entry| entry != 0 && e.mem.u32(entry) == ABILITY_ENTRY_VTABLE)
        .collect()
}

/// Whether the actor has an entry of its own holding the perk (`perksInfo->perkRanks.HasKey`).
fn has_own_perk(e: &mut Engine, actor: u32, perk: u32) -> bool {
    let id = e.mem.u32(actor + REF_ID);
    npc_perks(e)
        .get(&id)
        .is_some_and(|p| p.ranks.contains_key(&perk))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: AddTeammatePerkEntries
// (lines 3248-3261).
/// A teammates' perk at `rank` (its rank less one) on one teammate: unless the teammate has the
/// perk of its own, each ability entry of that rank added, of another rank removed (the entry
/// point entries come through [`get_perk_entry_point_list_npc`]'s teammate lists).
fn add_teammate_perk_entries(e: &mut Engine, teammate: u32, perk: u32, rank: u8) {
    if has_own_perk(e, teammate, perk) {
        return;
    }
    for entry in ability_entries(e, perk) {
        if e.mem.u8(entry + ENTRY_RANK) != rank {
            e.vcall(entry, VT_REMOVE_ENTRY, &args![teammate, 0u32]);
        } else {
            e.vcall(entry, VT_ADD_ENTRY, &args![teammate, 0u32]);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RemoveTeammatePerkEntries
// (lines 3263-3274).
/// A teammates' perk taken off one teammate: unless it has the perk of its own, each ability
/// entry removed.
fn remove_teammate_perk_entries(e: &mut Engine, teammate: u32, perk: u32) {
    if has_own_perk(e, teammate, perk) {
        return;
    }
    for entry in ability_entries(e, perk) {
        e.vcall(entry, VT_REMOVE_ENTRY, &args![teammate, 0u32]);
    }
}

/// The player's own perk list or its teammates' (`perkRanksPC`, `perkRanksTM`).
fn player_ranks_list(player: u32, for_teammates: bool) -> Ptr<ListNode> {
    Ptr::new(
        player
            + if for_teammates {
                PLAYER_PERK_RANKS_TM
            } else {
                PLAYER_PERK_RANKS_PC
            },
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetPerkRankPlayerHook
// (lines 3276-3313).
/// The player's `SetPerkRank` (`thiscall`: the perk, the new rank, for teammates): the perk's
/// `PerkRank` in the player's list or the teammates' (a new 8-byte one put first when it has none;
/// the same rank changes nothing) takes the new rank. For the player: each entry of that rank
/// added, of another rank removed (every kind, the player's own entry point lists included), the
/// stats menu refreshed and the reload speed updated. For the teammates: each teammate's ability
/// entries ([`add_teammate_perk_entries`]). Difference from JIP: a null perk changes nothing (the
/// exe's own test; JIP would read address 0x48).
fn set_perk_rank_player(e: &mut Engine, w: &[u32]) -> Ret {
    let (player, perk, new_rank, for_teammates) = (w[0], w[1], w[2] as u8, w[3] as u8 != 0);
    if perk == 0 {
        return Ret::default();
    }
    let list = player_ranks_list(player, for_teammates);
    let found = list_items(e, list)
        .into_iter()
        .find(|&item| item != 0 && e.mem.u32(item) == perk);
    let rank_item = match found {
        None => {
            let item = heap_alloc(e, 8);
            e.mem.set_u32(item, perk);
            list_prepend(e, list, item);
            item
        }
        Some(item) if e.mem.u8(item + 4) == new_rank => return Ret::default(),
        Some(item) => item,
    };
    e.mem.set_u8(rank_item + 4, new_rank);
    let rank = new_rank.wrapping_sub(1);
    if !for_teammates {
        for entry in list_items(e, Ptr::new(perk + PERK_ENTRIES)) {
            if entry == 0 {
                continue;
            }
            if e.mem.u8(entry + ENTRY_RANK) != rank {
                e.vcall(entry, VT_REMOVE_ENTRY, &args![player, 0u32]);
            } else {
                e.vcall(entry, VT_ADD_ENTRY, &args![player, 0u32]);
            }
        }
        e.call(REFRESH_STATS_MENU, &args![]);
        e.call(UPDATE_RELOAD_SPEED, &args![player]);
    } else {
        for teammate in list_items(e, Ptr::new(player + PLAYER_TEAMMATES)) {
            if teammate != 0 {
                add_teammate_perk_entries(e, teammate, perk, rank);
            }
        }
    }
    Ret::default()
}

/// `tList::RemoveIf` with `PerkRankFinder`: the first `PerkRank` of `perk` unlinked from the
/// list; it, or none.
fn remove_perk_rank(e: &mut Engine, list: Ptr<ListNode>, perk: u32) -> Option<u32> {
    let items = list_items(e, list);
    let at = items
        .iter()
        .position(|&item| item != 0 && e.mem.u32(item) == perk)?;
    let (mut node, mut prev) = (list, Ptr::<ListNode>::NULL);
    for _ in 0..at {
        prev = node;
        node = Ptr::new(e.get(node, ListNode::next));
    }
    list_unlink(e, node, prev);
    Some(items[at])
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RemovePerkPlayerHook
// (lines 3347-3375).
/// The player's `RemovePerk` (`thiscall`: the perk, for teammates): the perk's `PerkRank` taken
/// out of the player's list or the teammates' and freed (none: nothing). For the player: each of
/// the perk's entries removed, the stats menu refreshed and the reload speed updated. For the
/// teammates: each teammate's ability entries ([`remove_teammate_perk_entries`]). Difference from
/// JIP: a null perk changes nothing (as [`set_perk_rank_player`]).
fn remove_perk_player(e: &mut Engine, w: &[u32]) -> Ret {
    let (player, perk, for_teammates) = (w[0], w[1], w[2] as u8 != 0);
    if perk == 0 {
        return Ret::default();
    }
    let list = player_ranks_list(player, for_teammates);
    let Some(rank_item) = remove_perk_rank(e, list, perk) else {
        return Ret::default();
    };
    heap_free(e, rank_item);
    if !for_teammates {
        for entry in list_items(e, Ptr::new(perk + PERK_ENTRIES)) {
            if entry != 0 {
                e.vcall(entry, VT_REMOVE_ENTRY, &args![player, 0u32]);
            }
        }
        e.call(REFRESH_STATS_MENU, &args![]);
        e.call(UPDATE_RELOAD_SPEED, &args![player]);
    } else {
        for teammate in list_items(e, Ptr::new(player + PLAYER_TEAMMATES)) {
            if teammate != 0 {
                remove_teammate_perk_entries(e, teammate, perk);
            }
        }
    }
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetPlayerTeammateHook
// (lines 3449-3485).
/// O18's wrap of `Actor::SetPlayerTeammate` (`thiscall`: the actor, the flag byte), once the
/// switch is on (before: the exe's function). Not for the player, nor when the teammate byte
/// already equals the flag. The byte takes the flag. Made a teammate: the player's count up, the
/// actor put first in its teammates, and each of the teammates' perks' ability entries put on it
/// at that perk's rank ([`add_teammate_perk_entries`]). No longer one: the count down, the actor
/// taken out of the teammates and, when it has perks of its own (JIP's test), the teammates'
/// perks' ability entries taken off ([`remove_teammate_perk_entries`]). JIP's quirk kept: an
/// actor with no perks of its own keeps those ability entries after it leaves.
fn set_player_teammate(e: &mut Engine, w: &[u32]) -> Ret {
    if !crate::cosave::npc_perks_patch(e) {
        return e.call_next(SET_PLAYER_TEAMMATE.addr(), w);
    }
    let (actor, do_set) = (w[0], w[1] as u8);
    let player = e.mem.u32(THE_PLAYER);
    if actor == player || e.mem.u8(actor + ACTOR_IS_TEAMMATE) == do_set {
        return Ret::default();
    }
    e.mem.set_u8(actor + ACTOR_IS_TEAMMATE, do_set);
    let count = e.mem.u32(player + PLAYER_TEAMMATE_COUNT);
    let teammates = Ptr::<ListNode>::new(player + PLAYER_TEAMMATES);
    let ranks = Ptr::<ListNode>::new(player + PLAYER_PERK_RANKS_TM);
    // `tList::Empty`: the first node's item is 0.
    let ranks_empty = e.get(ranks, ListNode::data) == 0;
    if do_set != 0 {
        e.mem
            .set_u32(player + PLAYER_TEAMMATE_COUNT, count.wrapping_add(1));
        list_prepend(e, teammates, actor);
        if !ranks_empty {
            for item in list_items(e, ranks) {
                if item != 0 {
                    let (perk, rank) = (e.mem.u32(item), e.mem.u8(item + 4));
                    add_teammate_perk_entries(e, actor, perk, rank.wrapping_sub(1));
                }
            }
        }
    } else {
        e.mem
            .set_u32(player + PLAYER_TEAMMATE_COUNT, count.wrapping_sub(1));
        list_remove(e, teammates, actor);
        let id = e.mem.u32(actor + REF_ID);
        if !ranks_empty && has_entry(e, id) {
            for item in list_items(e, ranks) {
                if item != 0 {
                    let perk = e.mem.u32(item);
                    remove_teammate_perk_entries(e, actor, perk);
                }
            }
        }
    }
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetOptionalPatch case 18
// (SAFE_WRITE_BUF(0x5E592F, "\x0F\x1F\x84\x00\x00\x00\x00\x00"), line 4925).
/// `0x5E592F`: with the switch on, the teammate keeps itself as the entries' owner (answer 1);
/// before, nothing (the player's entries).
fn teammate_own_entries(e: &mut Engine, _: &[u32]) -> Option<u32> {
    crate::cosave::npc_perks_patch(e).then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetOptionalPatch case 18
// (SafeWrite32(0x64570F, 0x401F0F), line 4926).
/// `0x64570F`: with the switch on, the entry point runs whatever the caller asked (answer 1);
/// before, nothing (the caller's byte decides).
fn weapon_damage_perks(e: &mut Engine, _: &[u32]) -> Option<u32> {
    crate::cosave::npc_perks_patch(e).then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetOptionalPatch case 18
// (SAFE_WRITE_BUF(0x89B455, "\x8B\x02\x8B\x48\x68\x85\xC9\x74\x6E...\xEB\x57"), line 4927).
/// `0x89B455`: with the switch on, the attacker's current weapon (its process `+0x68`, the
/// process's virtual `+0x148`, that entry's form `+8`; 0 without a process or a weapon); before,
/// nothing (the player's).
fn kill_effect_weapon(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if !crate::cosave::npc_perks_patch(e) {
        return None;
    }
    let process = e.mem.u32(a[0] + ACTOR_PROCESS);
    if process == 0 {
        return Some(0);
    }
    let entry = e.vcall(process, VT_CURRENT_WEAPON, &args![]).u32();
    if entry == 0 {
        return Some(0);
    }
    Some(e.mem.u32(entry + INVENTORY_ENTRY_FORM))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitNPCPerks.
/// The actor's perks put on: an actor with no entry gets its starting perks; with an entry and no
/// lists yet, the lists are made and each perk's entries put on at its rank, by perk address.
pub fn init_npc_perks(e: &mut Engine, actor: u32) {
    let id = e.mem.u32(actor + REF_ID);
    let has = has_entry(e, id) || add_starting_perks(e, actor, false);
    if !has || npc_perks(e).get(&id).is_some_and(|p| p.entry_points != 0) {
        return;
    }
    let block = create_entry_points(e);
    let ranks: Vec<(u32, u8)> = {
        let mut map = npc_perks(e);
        let entry = map.emplace(id);
        entry.entry_points = block;
        entry.ranks.iter().map(|(&p, &r)| (p, r)).collect()
    };
    for (perk, rank) in ranks {
        add_perk_entries_npc(e, actor, perk, rank, block);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DestroyActorHook.
/// O18 at `0x87DE3F` in `~Actor`: the actor's entry, when it has one, dropped (its lists freed;
/// the data marked changed); no answer, so the exe's deleted-flag test runs. With the switch off:
/// nothing.
fn destroy_actor(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if !npc_perks_on(e) {
        return None;
    }
    let id = e.mem.u32(a[0] + REF_ID);
    let gone = {
        let mut map = npc_perks(e);
        let block = map.get(&id).map(|p| p.entry_points);
        if block.is_some() {
            map.erase(&id);
        }
        block
    };
    if let Some(block) = gone {
        mark_changed(e, crate::cosave::changed::NPC_PERKS);
        if block != 0 {
            destroy_entry_points(e, block);
        }
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h: DoLoadGameHousekeeping
// (the NPC perks part, after the weapon mods' pass).
/// After a load, with the patch on and the NPC perks marked changed: each high actor's perks put on
/// ([`init_npc_perks`]).
pub fn post_load_game(e: &mut Engine) {
    if !crate::cosave::npc_perks_patch(e)
        || crate::cosave::changed_flags(e) & crate::cosave::changed::NPC_PERKS == 0
    {
        return;
    }
    let actors = list_items(e, Ptr::new(crate::game_patches::gh2::HIGH_ACTORS));
    for actor in actors {
        if actor != 0 {
            init_npc_perks(e, actor);
        }
    }
}

/// The seams' handlers and the teammate wrap, in with the fixes (before the threads start; each
/// leaves the game as it is until [`switch_on`]: the destructor's with the switch's bit off, the
/// others until the patch is marked installed).
pub(super) fn install_fixes(e: &mut Engine) {
    e.ext_free::<NpcPerkLists>();
    e.on_site(DESTROY_ACTOR, destroy_actor);
    e.on_site(TEAMMATE_OWN_ENTRIES, teammate_own_entries);
    e.on_site(WEAPON_DAMAGE_PERKS, weapon_damage_perks);
    e.on_site(KILL_EFFECT_WEAPON, kill_effect_weapon);
    e.wrap(SET_PLAYER_TEAMMATE, set_player_teammate);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetOptionalPatch case 18
// (lines 4906-4930). Its jumps and byte writes are the seams and the wrap above, which answer
// once the patch is marked installed here; 0x89B50B's five bytes are the exe's own.
/// O18 on (the switch's bit set; called at `DeferredInit`): the valid perks and traits made from
/// the data, `uNPCPerks=2`'s flag kept, the patch marked installed (the co-save, the seams and the
/// wrap), and the routines written into the slots: four in each of the `Character` and `Creature`
/// vtables, two in the player's. Once; nothing with the switch off.
pub fn switch_on(e: &mut Engine, auto_add: bool) {
    if !npc_perks_on(e) || crate::cosave::npc_perks_patch(e) {
        return;
    }
    let made = build_valid_npc_perks(e);
    let lists = e.ext_free::<NpcPerkLists>();
    lists.auto_add.store(auto_add, Ordering::Relaxed);
    let _ = lists.lists.set(made);
    crate::cosave::set_npc_perks_patch(e, true);
    let routines: [AbiFn; 4] = [
        set_perk_rank_npc,
        remove_perk_npc,
        get_perk_rank_npc,
        get_perk_entry_point_list_npc,
    ];
    for slots in [CHARACTER_SLOTS, CREATURE_SLOTS] {
        for ((slot, at), f) in slots.into_iter().zip(ROUTINES).zip(routines) {
            write_slot(e, slot, at, f);
        }
    }
    let player: [AbiFn; 2] = [set_perk_rank_player, remove_perk_player];
    for ((slot, at), f) in PLAYER_SLOTS.into_iter().zip(PLAYER_ROUTINES).zip(player) {
        write_slot(e, slot, at, f);
    }
}

#[cfg(test)]
#[path = "ge_perks_tests.rs"]
mod tests;
