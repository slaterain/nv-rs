//! JIP LN NVSE's `CCCSetEquipped` (step 3.5d; `functions_jip/jip_fn_ccc.h` (JIP LN NVSE)) and the
//! always-on patch it relies on, `GetCannotWearHook` (`internal/hooks.h`), which `InitJIPHooks`
//! writes over the entry of `ExtraDataList::GetCanNotWear` (`0x418B10`) as JIP starts. While the
//! command equips an item, the "cannot wear" extra (type `0x3E`) no longer stops the game's
//! `EquipObject` (`0x88C830`) or `UnEquipObject` (`0x88D7D0`), and JIP's `LockEquipment` hook lets the
//! weapon through (`s_forceEquipCall`, read by `ai_hooks`' `EquipItemHook` handler). The command
//! has two forms: on a companion (a `Character`) it equips the companion's own gear (the first
//! non-playable weapon its combat style allows when it holds none, then every non-playable armour
//! whose slots are free); with no reference it equips or takes off the item selected in the
//! container menu's list of the companion's items (the companion wheel's inventory).
//!
//! JIP replaces `GetCanNotWear` whole; here it is a wrap of the address ([`GET_CANNOT_WEAR`], a row
//! of engine-core's `WRAPS`): every translation reaches `0x418B10` by address (the hygiene ratchet
//! counts 0 direct Rust calls, `engine-core/data/overridable.tsv`), and no typed family has its own
//! copy of it or of a caller. JIP's routine tests the list's type bit (`[list+0xF] & 0x40`, the bit
//! of type `0x3E` in the presence bits at `+8`) and answers false while the flag is set; the exe's
//! function tests the same bit (`BaseExtraList::HasExtra(0x3E)`, `0040fe80`), so with the flag clear
//! the wrap runs what is there (the translation), and with it set it answers false. Nothing here is
//! put in while JIP answers "not installed": no wrap, no handler, and the game is as before.
//!
//! The layouts, from the exe (Ghidra) and the port's translations: `ContainerMenu::pMe`
//! `0x11D93F8` and `pCurrItem` `0x11D93FC` (engine-fallout-interface `containermenu.rs`; written
//! by the menu's constructor `0075b155`, read by `DoClick`); the menu's `pContainerRef` `+0x74` and
//! `lbContainer` `+0xC8`, whose selected tile is at `+0x10` (`007a1910` reads it there); the
//! interface manager `*0x11D8A80`, its `pOverTileTarget` `+0xCC` and `pMouseOverTarget` `+0xBC`
//! (JIP's `activeTile` and `activeTileAlt`); a tile's values at `+0x14`, their count at `+0x18`,
//! sorted by id, a value's id `+0` and number `+8` (engine-bs `tile.rs`); `Character`'s
//! `BipedAnim` at `+0x1B4` (the slot `+0x1E8` of `Character`'s vtable, `005d9f90`, returns it), its
//! twenty slots at `+0x2C`, 0x10 bytes each, the item first; the worn armour's DR `+0x1B8` and DT
//! `+0x1BC`, recomputed while below 0 by `008d22b0` and `008d2110`; the actor base's combat style
//! getter, slot `+0x188` of its vtable (`TESNPC`'s `0x0104A2F4` holds `00607950`, which returns
//! `+0x1D4`; `TESCreature`'s `0x01048F5C` holds `008d8b80`; the translation of `00603370` calls it
//! there); a combat style's weapon restrictions `+0x40`; a weapon's type `+0xF4`, first flags
//! `+0x100` (`0x80` not playable) and second flags `+0x12C`; an armour's biped part mask `+0x74`
//! and biped flags `+0x78` (`0x40` not playable); an inventory entry's extra lists `+0`, count
//! `+4`, form `+8`; an extra list's worn bit (type `0x16`) at `+0xA`, bit `0x40`.

use engine_core::extend::{Owner, WrapSlot};
use engine_core::prelude::*;
use nvse::{handled, CommandArgs};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// The command's opcode.
pub const CCC_SET_EQUIPPED: u32 = 0x2703;
/// The one command.
pub const COMMANDS: [u32; 1] = [CCC_SET_EQUIPPED];

/// `ExtraDataList::GetCanNotWear` (`0x418B10`): JIP's `GetCannotWearHook` replaces it.
pub const GET_CANNOT_WEAR: WrapSlot = WrapSlot::new(Owner::Jip, 0x0041_8b10);

/// `kFormType_TESObjectARMO`, `kFormType_TESObjectWEAP`, `kFormType_AlchemyItem`,
/// `kFormType_TESLevItem`.
const ARMOR: u8 = 0x18;
const WEAPON: u8 = 0x28;
const ALCHEMY_ITEM: u8 = 0x2f;
const LEVELED_ITEM: u8 = 0x34;
/// `kXData_ExtraCombatStyle`.
const EXTRA_COMBAT_STYLE: u32 = 0x69;
/// `Character`'s and `TESObjectARMO`'s vtables (JIP's `kVtbl_Character`, `kVtbl_TESObjectARMO`).
const VTBL_CHARACTER: u32 = 0x0108_6a6c;
const VTBL_ARMOR: u32 = 0x0102_a62c;
/// The globals: the container menu and its selected entry, the interface manager, the player.
const CONTAINER_MENU: u32 = 0x011d_93f8;
const CONTAINER_SELECTION: u32 = 0x011d_93fc;
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const PLAYER: u32 = 0x011d_ea3c;
/// `kTileValue_id`, and the id of the container menu's list of the companion's items JIP asks for.
const TILE_VALUE_ID: u32 = 0xfaa;
const CONTAINER_LIST_TILE: i32 = 20;

/// The game's functions JIP calls: `BaseExtraList::GetByType` (`thiscall`: list, type),
/// `Actor::EquipItem` (`QueueEquipObject`, `thiscall`: actor, item, count, extra list, apply the
/// enchantment, lock, no message), `Actor::EquipObject` and `Actor::UnEquipObject` (`thiscall`, the
/// same six words), the worn DT and DR sums (`thiscall`: the character), the equip sound
/// (`thiscall`: the player, item, equip, equip) and `ContainerMenu::UpdateLists` (JIP's
/// `Refresh`, `thiscall`: the menu, item); and the actor base's combat style slot.
const GET_BY_TYPE: u32 = 0x0041_0220;
const EQUIP_ITEM: u32 = 0x0088_c650;
const EQUIP_OBJECT: u32 = 0x0088_c830;
const UNEQUIP_OBJECT: u32 = 0x0088_d7d0;
const WORN_DT: u32 = 0x008d_2110;
const WORN_DR: u32 = 0x008d_22b0;
const EQUIP_SOUND: u32 = 0x008a_ded0;
const CONTAINER_REFRESH: u32 = 0x0075_c280;
const VT_GET_COMBAT_STYLE: u32 = 0x188;

/// JIP's inventory map (`InventoryItemsMap`, `internal/jip_core.h`): item -> (count, entry), kept
/// in the order of the items' addresses as JIP's sorted `Map` is.
pub(crate) type Items = BTreeMap<u32, (i32, u32)>;

/// Puts the wrap over `GetCanNotWear` and the command in, when JIP answers "installed" (JIP writes
/// the patch as it starts, whatever a mod does).
pub fn install(e: &mut Engine, answers_installed: bool) {
    if !answers_installed {
        return;
    }
    // Made before the threads start, so every thread's engine shares it.
    e.ext_free::<ForceEquipCall>();
    e.wrap(GET_CANNOT_WEAR, get_cannot_wear);
    crate::table::set_handlers(e, CCC_SET_EQUIPPED, Some(ccc_set_equipped), None);
}

/// JIP's `s_forceEquipCall`: raised by `CCCSetEquipped` around its equip call, read by the wrap
/// and by `ai_hooks`' `EquipItemHook` handler. JIP's is a plain global byte, read on every
/// `GetCanNotWear` call of every thread without a lock; here it is a lock-free slot
/// ([`Engine::ext_free`]) made at install, before the threads start, so every thread's engine of
/// the run reads the one flag and none takes JIP's shared state for it.
#[derive(Default)]
struct ForceEquipCall(AtomicBool);

/// Whether `CCCSetEquipped`'s equip call is running (`s_forceEquipCall`).
pub(crate) fn force_equip_call(e: &mut Engine) -> bool {
    e.ext_free::<ForceEquipCall>().0.load(Ordering::Relaxed)
}

pub(crate) fn set_force_equip_call(e: &mut Engine, on: bool) {
    e.ext_free::<ForceEquipCall>()
        .0
        .store(on, Ordering::Relaxed);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetCannotWearHook.
/// `GetCanNotWear` (`ECX` = the list): false while `CCCSetEquipped` equips; otherwise what is
/// registered at the address answers (the exe's test of the same bit, so JIP's answer).
fn get_cannot_wear(e: &mut Engine, a: &[u32]) -> Ret {
    if force_equip_call(e) {
        return Ret::default();
    }
    e.call_next(GET_CANNOT_WEAR.addr(), a)
}

// ---- reads ----

fn form_type(e: &Engine, form: u32) -> u8 {
    e.mem.u8(form + 4)
}

fn is_type(e: &Engine, object: u32, vtbl: u32) -> bool {
    object != 0 && crate::commands::is_type(e, object, vtbl)
}

/// `ContChangesEntry::extendData->GetFirstItem()`: the first of an entry's extra lists (the list's
/// first node is inline: its data is the first word), 0 when the entry has none.
fn first_extra_list(e: &Engine, entry: u32) -> u32 {
    if entry == 0 {
        return 0;
    }
    let lists = e.mem.u32(entry);
    if lists == 0 {
        0
    } else {
        e.mem.u32(lists)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetWeaponInfo,
// Actor::GetEquippedWeapon.
/// The weapon the actor holds: its process (`+0x68`) at the middle-high level or above (`+0x28`
/// at most 1), whose weapon entry (`+0x114`) names it (`+8`); else 0.
fn equipped_weapon(e: &Engine, actor: u32) -> u32 {
    let process = e.mem.u32(actor + 0x68);
    if process == 0 || e.mem.u8(process + 0x28) > 1 {
        return 0;
    }
    let entry = e.mem.u32(process + 0x114);
    if entry == 0 {
        0
    } else {
        e.mem.u32(entry + 8)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetCombatStyle.
/// The actor's own combat style (its `ExtraCombatStyle`'s, `+0xC`) when it has one, else its base
/// form's (the actor base's virtual getter).
fn combat_style(e: &mut Engine, actor: u32) -> u32 {
    let extra = e
        .call(GET_BY_TYPE, &args![actor + 0x44, EXTRA_COMBAT_STYLE])
        .u32();
    if extra != 0 {
        let style = e.mem.u32(extra + 0xc);
        if style != 0 {
            return style;
        }
    }
    let base = e.mem.u32(actor + 0x20);
    // JIP calls through the base form without a test; a reference with none answers no style.
    if base == 0 {
        return 0;
    }
    e.vcall(base, VT_GET_COMBAT_STYLE, &[]).u32()
}

/// `TESCombatStyle::weaponRestrictions` of the actor's combat style, 0 with none.
fn weapon_restrictions(e: &mut Engine, actor: u32) -> u8 {
    let style = combat_style(e, actor);
    if style == 0 {
        0
    } else {
        e.mem.u8(style + 0x40)
    }
}

/// JIP's restriction test: a style that allows only melee (1) or only ranged weapons refuses the
/// other kind (types below 3 are melee).
fn restricted(restrictions: u8, weapon_type: u8) -> bool {
    restrictions != 0 && ((restrictions == 1) != (weapon_type < 3))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESContainer::GetCountForForm.
/// The base container's count of `form`: every entry of its list (first node inline at `+4`, each
/// entry's count `+0` and form `+4`) that names it, summed.
fn count_for_form(e: &Engine, container: u32, form: u32) -> i32 {
    let mut count = 0i32;
    let mut node = container + 4;
    while node != 0 {
        let object = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if object != 0 && e.mem.u32(object + 4) == form {
            count = count.wrapping_add(e.mem.u32(object) as i32);
        }
    }
    count
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntryList::FindForItem.
/// The reference's changes entry for `item` (each node's entry, its form `+8`), or 0.
fn find_for_item(e: &Engine, changes: u32, item: u32) -> u32 {
    let mut node = changes;
    while node != 0 {
        let entry = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if entry != 0 && e.mem.u32(entry + 8) == item {
            return entry;
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetInventoryItems.
/// Adds the reference's items of form type `kind` (0: all) to `items`, those already there kept:
/// first the base container's (not leveled lists), counted with their changes entry (the entry's
/// count alone when it holds a leveled item), then the changes entries' own; only a count above 0
/// goes in. Answers whether `items` holds anything (JIP's map is not emptied between two calls).
/// A container entry or changes entry with no form, which JIP would read through, is passed over.
pub(crate) fn inventory_items(e: &mut Engine, refr: u32, kind: u8, items: &mut Items) -> bool {
    let base = crate::overrides::base_form(e, refr);
    let container = crate::overrides::base_container(e, base);
    if container == 0 {
        return false;
    }
    let changes = crate::overrides::changes_list(e, refr);
    if changes == 0 {
        return false;
    }
    let mut node = container + 4;
    while node != 0 {
        let object = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if object == 0 {
            continue;
        }
        let item = e.mem.u32(object + 4);
        if item == 0
            || form_type(e, item) == LEVELED_ITEM
            || (kind != 0 && form_type(e, item) != kind)
            || items.contains_key(&item)
        {
            continue;
        }
        let mut count = count_for_form(e, container, item);
        let entry = find_for_item(e, changes, item);
        if entry != 0 {
            let delta = e.mem.u32(entry + 4) as i32;
            count = if crate::overrides::has_extra_leveled_item(e, entry) {
                delta
            } else {
                count.wrapping_add(delta)
            };
        }
        if count > 0 {
            items.insert(item, (count, entry));
        }
    }
    let mut node = changes;
    while node != 0 {
        let entry = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if entry == 0 {
            continue;
        }
        let item = e.mem.u32(entry + 8);
        let delta = e.mem.u32(entry + 4) as i32;
        if item == 0
            || delta <= 0
            || (kind != 0 && form_type(e, item) != kind)
            || items.contains_key(&item)
        {
            continue;
        }
        items.insert(item, (delta, entry));
    }
    !items.is_empty()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::GetValue (its binary
// search); functions_jip/jip_fn_ui.h: GetActiveTileID.
/// The id value of the tile under the cursor (`activeTile`, else `activeTileAlt`), truncated as
/// `cvttss2si` does (`0x80000000` for a number out of range); -1 with no tile or no id value.
fn active_tile_id(e: &Engine) -> i32 {
    let manager = e.mem.u32(INTERFACE_MANAGER);
    // JIP reads through the manager without a test; with none there is no tile.
    if manager == 0 {
        return -1;
    }
    let mut tile = e.mem.u32(manager + 0xcc);
    if tile == 0 {
        tile = e.mem.u32(manager + 0xbc);
    }
    if tile == 0 {
        return -1;
    }
    let values = e.mem.u32(tile + 0x14);
    let (mut lo, mut hi) = (0u32, e.mem.u32(tile + 0x18));
    while lo != hi {
        let mid = lo.wrapping_add(hi) >> 1;
        let value = e.mem.u32(values.wrapping_add(mid.wrapping_mul(4)));
        let id = e.mem.u32(value);
        if id == TILE_VALUE_ID {
            let num = e.mem.f32(value + 8);
            return if num.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&num) {
                i32::MIN
            } else {
                num as i32
            };
        }
        if id < TILE_VALUE_ID {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    -1
}

/// `EquipItem(item, 1, extra list, 1, 0, 0)` on the character.
fn equip_item(e: &mut Engine, character: u32, item: u32, extra: u32) {
    e.call(
        EQUIP_ITEM,
        &args![character, item, 1u32, extra, 1u32, 0u32, 0u32],
    );
}

// ---- the command ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCSetEquipped_Execute.
/// On a reference: a `Character` equips its own gear ([`equip_own_gear`]); any other reference
/// nothing. With none: the container menu's selected item is equipped by, or taken off, its
/// companion ([`equip_selected`]).
fn ccc_set_equipped(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj != 0 {
        if is_type(e, a.this_obj, VTBL_CHARACTER) {
            equip_own_gear(e, a.this_obj);
        }
        return handled(true);
    }
    equip_selected(e, &a);
    handled(true)
}

/// A companion with no weapon in hand equips the first of its weapons (in the order of their
/// addresses) that is not playable and that its combat style allows; then each of its armours,
/// in the same order, that is not playable and whose slots no worn armour (nor one equipped here
/// before it) takes; then the worn DR and DT are summed again. As in JIP, the weapons found first
/// stay in the map for the armour pass (it is not emptied between the two): each is tested there
/// with the words an armour's biped part mask and flags would be at (`+0x74`, `+0x78`; in a
/// weapon, its enchantment component's vtable and enchantment) and may be equipped by it too.
fn equip_own_gear(e: &mut Engine, character: u32) {
    let mut items = Items::new();
    if equipped_weapon(e, character) == 0 && inventory_items(e, character, WEAPON, &mut items) {
        let restrictions = weapon_restrictions(e, character);
        let weapons: Vec<(u32, u32)> = items.iter().map(|(&w, &(_, en))| (w, en)).collect();
        for (weapon, entry) in weapons {
            let playable = e.mem.u8(weapon + 0x100) & 0x80 == 0;
            if playable || restricted(restrictions, e.mem.u8(weapon + 0xf4)) {
                continue;
            }
            // JIP's extra list starts as none and is set only from an entry with lists; the
            // first weapon equipped ends the pass, so it is that weapon's or none.
            let extra = first_extra_list(e, entry);
            equip_item(e, character, weapon, extra);
            break;
        }
    }
    let biped = e.mem.u32(character + 0x1b4);
    if biped == 0 || !inventory_items(e, character, ARMOR, &mut items) {
        return;
    }
    let mut used = 0u32;
    for slot in 0..20u32 {
        let item = e.mem.u32(biped + 0x2c + slot * 0x10);
        if item == 0 || !is_type(e, item, VTBL_ARMOR) {
            continue;
        }
        items.remove(&item);
        used |= e.mem.u32(item + 0x74);
    }
    let left: Vec<(u32, u32)> = items.iter().map(|(&i, &(_, en))| (i, en)).collect();
    for (item, entry) in left {
        let playable = e.mem.u32(item + 0x78) & 0x40 == 0;
        let mask = e.mem.u32(item + 0x74);
        if playable || used & mask != 0 {
            continue;
        }
        let extra = first_extra_list(e, entry);
        equip_item(e, character, item, extra);
        used |= mask;
    }
    // `Character::RefreshWornDTDR`.
    e.mem.set_f32(character + 0x1b8, -1.0);
    e.mem.set_f32(character + 0x1bc, -1.0);
    e.call(WORN_DT, &args![character]);
    e.call(WORN_DR, &args![character]);
}

/// The container menu's selected entry, with the companion's list selected and the cursor on it
/// (tile id 20): an armour or weapon not worn is equipped, a worn one taken off (an armour or
/// weapon with no extra list is not worn), an aid item used; any other item nothing. A weapon to
/// equip that is a grenade or mine thrown (types 11 and 12) or has the second flags' bit 0
/// answers -1, one the companion's combat style refuses answers the style's restriction; a
/// thrown weapon (type above 9) is equipped, and any weapon taken off, as a whole stack. The game's
/// equip or unequip runs with JIP's flag set (the "cannot wear" extra and the locked equipment
/// let the item through), then the player hears the item's sound and the menu's lists are
/// updated.
fn equip_selected(e: &mut Engine, a: &CommandArgs) {
    let menu = e.mem.u32(CONTAINER_MENU);
    let entry = e.mem.u32(CONTAINER_SELECTION);
    if entry == 0
        || menu == 0
        || e.mem.u32(menu + 0xc8 + 0x10) == 0
        || active_tile_id(e) != CONTAINER_LIST_TILE
    {
        return;
    }
    let item = e.mem.u32(entry + 8);
    let character = e.mem.u32(menu + 0x74);
    if item == 0 || !is_type(e, character, VTBL_CHARACTER) {
        return;
    }
    let mut equip = true;
    let mut count = 1i32;
    let mut extra = 0u32;
    let kind = form_type(e, item);
    if kind == ARMOR || kind == WEAPON {
        extra = first_extra_list(e, entry);
        if extra != 0 && e.mem.u8(extra + 0xa) & 0x40 != 0 {
            equip = false;
        }
        if kind == WEAPON {
            if equip {
                let weapon_type = e.mem.u8(item + 0xf4);
                if weapon_type == 11 || weapon_type == 12 || e.mem.u32(item + 0x12c) & 1 != 0 {
                    a.set_result(e, -1.0);
                    return;
                }
                let restrictions = weapon_restrictions(e, character);
                if restricted(restrictions, weapon_type) {
                    a.set_result(e, f64::from(restrictions));
                    return;
                }
                if weapon_type > 9 {
                    count = e.mem.u32(entry + 4) as i32;
                }
            } else {
                count = e.mem.u32(entry + 4) as i32;
            }
        }
    } else if kind != ALCHEMY_ITEM {
        return;
    }
    let function = if equip { EQUIP_OBJECT } else { UNEQUIP_OBJECT };
    set_force_equip_call(e, true);
    // The flag goes back down also when a guest exception ends the call (a Rust panic unwinding
    // through here), so no later equip is let through by it.
    let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        e.call(
            function,
            &args![character, item, count as u32, extra, 1u32, 0u32, 0u32],
        );
    }));
    set_force_equip_call(e, false);
    if let Err(stop) = done {
        std::panic::resume_unwind(stop);
    }
    let player = e.mem.u32(PLAYER);
    e.call(
        EQUIP_SOUND,
        &args![player, item, u32::from(equip), u32::from(equip)],
    );
    e.call(CONTAINER_REFRESH, &args![menu, item]);
}

#[cfg(test)]
#[path = "equip_hooks_tests.rs"]
mod tests;
