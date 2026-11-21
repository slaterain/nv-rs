//! Tweaks' co-save section (`serialization.cpp` (Tweaks)), under its opcode base `0x2132` in the
//! built-in xNVSE co-save beside the port's saves. Built: the ammunition record `AMID` of
//! `RememberAmmoCountOnLoad` (with row S0089.07). Tweaks' other records (`PINH`, `SPOS`, `1LST`,
//! `2LST`, `1KRM`, `SOPB`, `SOPH`, `PXVS`, `SOPM`) belong to features not built yet: a co-save's
//! records of those kinds are read and written back as they were (while the loaded files are the
//! ones the co-save lists), so a player who goes back to the real plugin keeps them.
//!
//! The format is Tweaks' own (xNVSE's chunk: type, version 0, the four bytes of the form id), so
//! a co-save written by the real plugin is read the same way, and the real plugin reads ours.

use engine_core::prelude::*;
use nvse::cosave::{tag, OutOfRange, Reader};
use nvse::plugins::Records;

/// `kRecordID_AmmoID` (`'AMID'`).
pub const AMMO_ID: u32 = tag(b"AMID");

/// The player (`PlayerCharacter::GetSingleton`), its process (`+0x68`) and the process's weapon and
/// ammunition entries (`MiddleHighProcess` `+0x114`, `+0x118`: an entry's count `+4`, form `+8`).
const PLAYER: u32 = 0x011d_ea3c;
/// `LookupFormByID` (`004839c0`).
const LOOKUP_FORM: u32 = 0x0048_39c0;
/// `BaseExtraList::GetByType` (`00410220`), `ExtraContainerChanges` (0x15, its data at `+0xC`),
/// `ExtraWeaponModFlags` (0x8D, its flags at `+0xC`), and the inventory count
/// (`ExtraContainerChanges::Data::GetItemCount`, `004c8f30`).
const EXTRA_GET_BY_TYPE: u32 = 0x0041_0220;
const EXTRA_CONTAINER_CHANGES: u32 = 0x15;
const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8d;
const ITEM_COUNT: u32 = 0x004c_8f30;
/// `kFormType_TESAmmo`, `kFormType_BGSListForm`.
const FORM_TYPE_AMMO: u8 = 0x29;
const FORM_TYPE_LIST: u8 = 0x55;
/// `kWeaponModEffect_IncreaseClipCapacity`.
const MOD_CLIP_CAPACITY: u32 = 2;

/// Tweaks' records of the kinds not built, kept from the co-save read for the next one.
#[derive(Default, Clone)]
pub struct Kept {
    pub records: Vec<(u32, u32, Vec<u8>)>,
}

fn ammo_rows_on(e: &Engine) -> bool {
    e.stewie(engine_core::extend::stewie::LOAD_GAME_AMMO_COUNT)
}

// Translated from lStewieAl's Tweaks 10.05 (serialization.cpp): SaveGameCallback, its
// "store selected ammo type" part.
/// The save callback: `AMID` with the player's current ammunition's form id, then the kept
/// records of the kinds not built.
pub fn save(e: &mut Engine, records: &mut Records) {
    if ammo_rows_on(e) {
        let process = e.mem.u32(e.global::<u32>(PLAYER) + 0x68);
        let ammo_info = e.mem.u32(process + 0x118);
        if ammo_info != 0 {
            let ammo = e.mem.u32(ammo_info + 8);
            if ammo != 0 {
                let id = e.mem.u32(ammo + 0xc);
                records.record(AMMO_ID, 0, &id.to_le_bytes());
            }
        }
    }
    if nvse::saveload::files_as_saved(e) {
        let kept = e.ext::<crate::Stewie>().kept.records.clone();
        for (kind, version, data) in kept {
            records.record(kind, version, &data);
        }
    }
}

// Translated from lStewieAl's Tweaks 10.05 (main.h): MessageHandler, its `kMessage_PreLoadGame` case
// (`RememberAmmoCountOnLoad::ResetSavedAmmoCount`).
/// Tweaks' listener: before a save is read there is no saved count until the player's load keeps
/// one (S0089.01); the records kept are the coming co-save's.
pub fn listen(e: &mut Engine, m: &nvse::plugins::Message) {
    if m.kind == nvse::plugins::msg::PRE_LOAD_GAME {
        let kept = e.stewie_kept(engine_core::extend::stewie::kept::SAVED_AMMO_COUNT, 4);
        e.mem.set_u32(kept, 0);
        e.ext::<crate::Stewie>().kept.records.clear();
    }
}

/// The new-game callback: no records kept.
pub fn new_game(e: &mut Engine) {
    e.ext::<crate::Stewie>().kept.records.clear();
}

// Translated from lStewieAl's Tweaks 10.05 (serialization.cpp): LoadGameCallback, its
// `kRecordID_AmmoID` case.
/// The load callback, after the game's load: `AMID` applied; the other kinds kept.
pub fn load(e: &mut Engine, r: &mut Reader) -> Result<(), OutOfRange> {
    while let Some((kind, version, length)) = r.next_record()? {
        if kind == AMMO_ID {
            let id = r.read32()?;
            if ammo_rows_on(e) {
                restore_ammo(e, id);
            }
        } else {
            let data = r.read_data(length)?;
            e.ext::<crate::Stewie>()
                .kept
                .records
                .push((kind, version, data));
        }
    }
    Ok(())
}

/// The `kRecordID_AmmoID` case: the saved form id resolved (`ResolveRefID`); when it names
/// ammunition the player carries, that becomes the ammunition entry's type and its count the
/// saved count (S0089.01) limited to the rounds carried, or, for a save without that count, the
/// rounds carried limited to the weapon's clip size (no weapon: 0).
pub fn restore_ammo(e: &mut Engine, saved_id: u32) {
    let Some(id) = nvse::saveload::resolve_ref_id(e, saved_id) else {
        return;
    };
    if id == 0 {
        return;
    }
    let player = e.global::<u32>(PLAYER);
    let process = e.mem.u32(player + 0x68);
    let ammo_info = e.mem.u32(process + 0x118);
    if ammo_info == 0 {
        return;
    }
    let ammo = e.call(LOOKUP_FORM, &args![id]).u32();
    if ammo == 0 || e.mem.u8(ammo + 4) != FORM_TYPE_AMMO {
        return;
    }
    let carried = item_count(e, player, ammo);
    if carried == 0 {
        return;
    }
    e.mem.set_u32(ammo_info + 8, ammo);
    let kept = e.stewie_kept(engine_core::extend::stewie::kept::SAVED_AMMO_COUNT, 4);
    let saved = e.mem.u32(kept) as i32 - 1;
    let count = if saved != -1 {
        carried.min(saved)
    } else {
        let weapon_info = e.mem.u32(process + 0x114);
        if weapon_info != 0 && e.mem.u32(weapon_info + 8) != 0 {
            carried.min(clip_size(e, weapon_info))
        } else {
            0
        }
    };
    e.mem.set_u32(ammo_info + 4, count as u32);
}

// Translated from lStewieAl's Tweaks 10.05 (xNVSE's GameObjects.cpp): TESObjectREFR::GetItemCount.
/// How many of `form` `actor` carries (a form list's entries summed); 0 without container changes.
fn item_count(e: &mut Engine, actor: u32, form: u32) -> i32 {
    let extra = e
        .call(
            EXTRA_GET_BY_TYPE,
            &args![actor + 0x44, EXTRA_CONTAINER_CHANGES],
        )
        .u32();
    if extra == 0 {
        return 0;
    }
    let data = e.mem.u32(extra + 0xc);
    if data == 0 {
        return 0;
    }
    if e.mem.u8(form + 4) != FORM_TYPE_LIST {
        return e.call(ITEM_COUNT, &args![data, form]).u32() as i32;
    }
    let mut total = 0i32;
    let mut node = form + 0x18;
    while node != 0 {
        let item = e.mem.u32(node);
        if item != 0 {
            let n = e.call(ITEM_COUNT, &args![data, item]).u32() as i32;
            total = total.wrapping_add(n);
        }
        node = e.mem.u32(node + 4);
    }
    total
}

// Translated from lStewieAl's Tweaks 10.05 (Internal/tweaks_core.cpp): GetWeaponInfoClipSize,
// WeaponHasNthMod, GetWeaponModFlags (and xNVSE's TESObjectWEAP::GetModBonuses).
/// The weapon entry's clip size: the weapon's `clipRounds` (`BGSClipRoundsForm` at `+0xAC`, the
/// byte at `+0xB0`), plus the weapon's increase-clip bonuses (`value1Mod`, `+0x18C`, of every mod
/// slot whose effect, `+0x180`, is 2) when a fitted mod (the entry's first extra list's
/// `ExtraWeaponModFlags`) has that effect; the sum truncated as C++'s `int += float`.
fn clip_size(e: &mut Engine, weapon_info: u32) -> i32 {
    let weapon = e.mem.u32(weapon_info + 8);
    let mut rounds = i32::from(e.mem.u8(weapon + 0xb0));
    let lists = e.mem.u32(weapon_info);
    let first = if lists != 0 { e.mem.u32(lists) } else { 0 };
    let flags = if first != 0 {
        let extra = e
            .call(EXTRA_GET_BY_TYPE, &args![first, EXTRA_WEAPON_MOD_FLAGS])
            .u32();
        if extra != 0 {
            e.mem.u8(extra + 0xc)
        } else {
            0
        }
    } else {
        0
    };
    let fitted = (0..3)
        .rev()
        .any(|i| flags & (1 << i) != 0 && e.mem.u32(weapon + 0x180 + 4 * i) == MOD_CLIP_CAPACITY);
    if fitted {
        let bonus: f32 = (0..3)
            .filter(|i| e.mem.u32(weapon + 0x180 + 4 * i) == MOD_CLIP_CAPACITY)
            .map(|i| e.mem.f32(weapon + 0x18c + 4 * i))
            .sum();
        rounds = (rounds as f32 + bonus) as i32;
    }
    rounds
}

#[cfg(test)]
#[path = "cosave_tests.rs"]
mod tests;
