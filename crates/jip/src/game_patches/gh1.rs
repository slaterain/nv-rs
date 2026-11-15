//! Group GH1's game patches (step 3.9: extra data and inventory stacks): its handlers, wraps and slot routines. Only group
//! GH1's PR edits this file (and its own tests file beside it).
//!
//! **G44, the ExtraJIP framework** (`InitGamePatches`, "For ExtraJIP framework",
//! `internal/patches_game.h` (JIP LN NVSE) 5287-5298), part of JIP in every preset
//! (`jip_patches::INSTALLED`). JIP takes the extra data type `0x27` (`ExtraTimeLeft`, which the
//! game never makes) for its own keyed data, `ExtraJIP`: the same 0x10 bytes and vtable
//! (`0x010158FC`), the word at `+0xC` a key into JIP's map of data (`s_extraDataKeysMap`, the
//! co-save's `JPED` record: [`crate::cosave::Saved::extra_data`], its keys as `cosave.rs` reads
//! and writes them). Its patches:
//! - three type tables: an ExtraJIP goes to the head of its list (`0x40F9D7`), ends the
//!   ownership-only scan of `0041d030` with false (`0x41D0F6`) and makes a list not the
//!   container's default (`0x41D212`, `0x41D25A`): an item with JIP's data keeps a stack of its own;
//! - the two slots of the type's vtable: its destructor drops the key's data unless the data
//!   belongs to a reference that is gone while the save holds changes of it (`0x10158FC`), and
//!   two ExtraJIPs compare equal when their keys' data are equal (`0x1015900`);
//! - the type's arms of `AddExtraCopy` (`0x411978`: a stack split or moved copies the data to a
//!   new key, or moves the key), `SaveGame_ov2` (`0x427F30`: the key, and the reference it is
//!   saved with) and `LoadGame_ov2` (`0x42C328`: the key, into the list's ExtraJIP);
//! - two calls of `004c1c90` (a reference picked up into the changes): the count is the
//!   reference's count extra's (`0x4C21EB`), and the new stack's list gets a count extra before
//!   the reference's list is copied into it when the count is above 1 (`0x4C2291`), so a moved
//!   ExtraJIP's stack keeps its count.
//!
//! Until a command sets JIP's data no ExtraJIP exists and nothing here changes what the game
//! does, except the two pick-up calls, which run for every reference picked up while JIP is
//! installed (a stack of more than one item picked up into an entry it does not merge with keeps
//! a list holding its count, where the exe drops the empty list).
//!
//! G41 (`SafeWrite8(0x40F75B, 0x18)`, the extra data list's constructor clearing three more
//! bytes) is not built: it only starts JIP's word in a list's padding null, which the port keeps
//! in records of its own (`measure/jip/PLAN-3.9-verified.md` §1).

use super::slots::{self, Group};
use super::Area;
use crate::commands::{extra, heap_free};
use crate::containers::Uint;
use crate::cosave::{self, changed, ExtraJipData, ExtraJipEntry};
use crate::Jip;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// The seams (`SITES` rows of `crates/engine-core/src/extend/patch_rows/gh1.rs`).
pub const HEAD: SiteSlot = SiteSlot::new(Owner::Jip, 0x0040_f9d7);
pub const COPY: SiteSlot = SiteSlot::new(Owner::Jip, 0x0041_1978);
pub const OWNED_ONLY: SiteSlot = SiteSlot::new(Owner::Jip, 0x0041_d0f6);
pub const DEFAULT_WORN: SiteSlot = SiteSlot::new(Owner::Jip, 0x0041_d212);
pub const DEFAULT_NOT_WORN: SiteSlot = SiteSlot::new(Owner::Jip, 0x0041_d25a);
pub const SAVE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0042_7f30);
pub const LOAD: SiteSlot = SiteSlot::new(Owner::Jip, 0x0042_c328);
pub const PICKED_COUNT: SiteSlot = SiteSlot::new(Owner::Jip, 0x004c_21eb);
pub const COPY_COUNT: SiteSlot = SiteSlot::new(Owner::Jip, 0x004c_2291);

/// `kXData_ExtraJIP` (= `kXData_ExtraTimeLeft`) and `kXData_ExtraCount`.
pub const EXTRA_JIP: u32 = 0x27;
pub const EXTRA_COUNT: u32 = 0x24;
/// `kVtbl_ExtraTimeLeft`, the vtable JIP's `ExtraJIP::Create` stores, and its two slots JIP
/// writes: the scalar deleting destructor (held `0x41B680`, shared by 61 vtables) and `Compare`
/// (held `0x431EF0`).
pub const VTABLE: u32 = 0x0101_58fc;
pub const DESTROY_SLOT: u32 = VTABLE;
pub const COMPARE_SLOT: u32 = VTABLE + 4;
/// `kVtbl_ExtraCount`.
pub const COUNT_VTABLE: u32 = 0x0101_58d8;
/// The routines JIP's two slot functions are put at.
pub const DESTROY_ROUTINE: u32 = slots::routine(Group::GH1, 0);
pub const COMPARE_ROUTINE: u32 = slots::routine(Group::GH1, 1);
/// `ADDR_GetRandomInt` on `GAME_RNG` (JIP's `GetRandomUInt`).
pub const GET_RANDOM_INT: u32 = 0x00aa_5230;
pub const GAME_RNG: u32 = 0x011c_4180;
/// `BGSSaveGameBuffer`'s grow (`0x8660B0`, `thiscall`), which JIP's `Reserve` calls.
pub const SAVE_BUFFER_GROW: u32 = 0x0086_60b0;
/// `FormBufferHeader::GetRefID` (`0x853500`, `thiscall` on the header).
pub const HEADER_REF_ID: u32 = 0x0085_3500;
/// The return addresses JIP's copy arm tells apart: `CopyListForContainer`'s and
/// `CopyListForReference`'s calls of `AddExtraCopy`.
pub const FROM_CONTAINER: u32 = 0x0041_2250;
pub const FROM_REFERENCE: u32 = 0x0041_2523;

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GH1",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: super::nothing,
    jip_sites: &[
        HEAD.addr(),
        COPY.addr(),
        OWNED_ONLY.addr(),
        DEFAULT_WORN.addr(),
        DEFAULT_NOT_WORN.addr(),
        SAVE.addr(),
        LOAD.addr(),
        PICKED_COUNT.addr(),
        COPY_COUNT.addr(),
    ],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(HEAD, to_head);
    e.on_site(COPY, copy);
    e.on_site(OWNED_ONLY, ends_scan);
    e.on_site(DEFAULT_WORN, ends_scan);
    e.on_site(DEFAULT_NOT_WORN, ends_scan);
    e.on_site(SAVE, save);
    e.on_site(LOAD, load);
    e.on_site(PICKED_COUNT, picked_count);
    e.on_site(COPY_COUNT, copy_count);
    slots::write_slot(e, DESTROY_SLOT, DESTROY_ROUTINE, destroy);
    slots::write_slot(e, COMPARE_SLOT, COMPARE_ROUTINE, differs);
}

fn saved(e: &mut Engine) -> &mut cosave::Saved {
    &mut e.ext::<Jip>().saved
}

/// The key of an ExtraJIP (`+0xC`).
fn key_of(e: &Engine, extra: u32) -> u32 {
    e.mem.u32(extra + 0x0c)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp: ExtraJIP::MakeKey.
/// A new key: a random non-zero word no entry has.
pub fn make_key(e: &mut Engine) -> u32 {
    loop {
        let key = e
            .call(GET_RANDOM_INT, &args![GAME_RNG, 0xffff_ffffu32])
            .u32();
        if key != 0 && saved(e).extra_data.get(&Uint(key)).is_none() {
            return key;
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp: ExtraJIP::Create,
// ExtraDataList::AddExtraJIP.
/// A new ExtraJIP of `key` added to `list` (`BaseExtraList::AddExtra`); answers it.
pub fn add_extra_jip(e: &mut Engine, list: u32, key: u32) -> u32 {
    let created = extra::create(e, VTABLE, EXTRA_JIP);
    e.mem.set_u32(created + 0x0c, key);
    extra::add_extra(e, list, created)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp: ExtraCount::Create,
// ExtraDataList::AddExtraCount.
/// A new count extra of `count` (at most 32,767) added to `list`; answers it.
fn add_extra_count(e: &mut Engine, list: u32, count: i32) -> u32 {
    let created = extra::create(e, COUNT_VTABLE, EXTRA_COUNT);
    e.mem.set_u32(created + 0x0c, count.min(0x7fff) as u32);
    extra::add_extra(e, list, created)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: XString::operator==.
/// Two of JIP's strings are equal: the same length and, when not empty, the same text up to
/// the first zero byte (`StrCompareCS`).
fn same_string(a: &[u8], b: &[u8]) -> bool {
    let text = |s: &[u8]| -> Vec<u8> { s.iter().copied().take_while(|&c| c != 0).collect() };
    a.len() == b.len() && (a.is_empty() || text(a) == text(b))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp: ExtraJIPData::operator==;
// GameExtraData.h: ExtraJIPEntry::operator==; internal/containers.h: Map::operator==.
/// Two keys' data are equal: the same mods, each with the same bit words, values and strings
/// (the reference each was saved with is not compared).
fn same_data(a: &ExtraJipEntry, b: &ExtraJipEntry) -> bool {
    let same = |x: &ExtraJipData, y: &ExtraJipData| {
        x.used == y.used
            && x.forms == y.forms
            && x.values == y.values
            && same_string(&x.strings[0], &y.strings[0])
            && same_string(&x.strings[1], &y.strings[1])
    };
    a.data.len() == b.data.len()
        && a.data
            .iter()
            .zip(&b.data)
            .all(|((ka, va), (kb, vb))| ka == kb && same(va, vb))
}

/// `0x40F9D7`: the type's entry JIP writes in `fn_0040f980`'s table (to the head of the chain).
fn to_head(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// `0x41D0F6`, `0x41D212`, `0x41D25A`: the type's entries JIP writes in the scans' tables (the
/// scan ends with false).
fn ends_scan(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ExtraJIPCreateCopyHook.
/// `AddExtraCopy`'s arm of the type (`0x411978`: the list, the source ExtraJIP, the caller's
/// return address and second argument). With no data for the source's key nothing is copied.
/// Else the list's ExtraJIP (one is added, key 0, when it has none) either takes the source's
/// key and the source the list's old one, the data's reference cleared (a stack moved:
/// `CopyListForContainer` with `copy_only` false, `CopyListForReference` removing from the
/// source), or gets a copy of the data under its own key (a new key when it has none).
fn copy(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (list, source, caller, flag) = (a[0], a[1], a[2], a[3] as u8);
    let source_key = key_of(e, source);
    let Some(data) = saved(e)
        .extra_data
        .get(&Uint(source_key))
        .map(|entry| entry.data.clone())
    else {
        return Some(0);
    };
    saved(e).changed |= changed::EXTRA_DATA;
    let mut target = extra::get_by_type(e, list, EXTRA_JIP);
    if target == 0 {
        target = add_extra_jip(e, list, 0);
    }
    let moved = match caller {
        FROM_CONTAINER => flag == 0,
        FROM_REFERENCE => flag != 0,
        _ => false,
    };
    if moved {
        if let Some(entry) = saved(e).extra_data.get_mut(&Uint(source_key)) {
            entry.ref_id = 0;
        }
        let target_key = key_of(e, target);
        e.mem.set_u32(source + 0x0c, target_key);
        e.mem.set_u32(target + 0x0c, source_key);
    } else {
        let mut key = key_of(e, target);
        if key == 0 {
            key = make_key(e);
            e.mem.set_u32(target + 0x0c, key);
        }
        // `s_extraDataKeysMap()[key] = *inEntry`: the data only; an entry already there keeps
        // its reference.
        saved(e).extra_data.emplace(Uint(key)).data = data;
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ExtraJIPSaveGameHook;
// nvse/GameExtraData.cpp: ExtraJIP::SaveGame; nvse/GameAPI.h: BGSSaveGameBuffer::Reserve,
// BGSSaveFormBuffer::GetRefID.
/// `SaveGame_ov2`'s arm of the type (`0x427F30`: the save buffer, the ExtraJIP): 5 bytes reserved
/// in the buffer (grown once when they do not fit, its value count raised), the key and the
/// spacer `|`; a key with no data is saved as 0, one with data notes the reference the buffer
/// saves (its form's id, else the header's).
fn save(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (buffer, extra) = (a[0], a[1]);
    let required = e.mem.u32(buffer + 0x0c).wrapping_add(5);
    if required > e.mem.u32(buffer + 8) {
        e.call(SAVE_BUFFER_GROW, &args![buffer]);
    }
    let values = e.mem.u32(buffer + 0x10);
    e.mem.set_u32(buffer + 0x10, values.wrapping_add(1));
    let at = e.mem.u32(buffer + 4).wrapping_add(e.mem.u32(buffer + 0x0c));
    e.mem.set_u32(buffer + 0x0c, required);
    e.mem.set_u8(at + 4, b'|');
    let key = key_of(e, extra);
    if saved(e).extra_data.get(&Uint(key)).is_some() {
        let form = e.mem.u32(buffer + 0x20);
        let ref_id = if form != 0 {
            e.mem.u32(form + 0x0c)
        } else {
            e.call(HEADER_REF_ID, &args![buffer + 0x14]).u32()
        };
        if let Some(entry) = saved(e).extra_data.get_mut(&Uint(key)) {
            entry.ref_id = ref_id;
        }
        e.mem.set_u32(at, key);
    } else {
        e.mem.set_u32(at, 0);
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ExtraJIPLoadGameHook.
/// `LoadGame_ov2`'s arm of the type (`0x42C328`: the list, the load buffer): the key read from
/// the buffer's data as it stands (no test of the data or the spacer, 5 bytes passed over); a key
/// of 0 loads nothing. A list with no ExtraJIP gets one of the key; one with another key takes
/// the key, and the old key's data is dropped.
fn load(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (list, buffer) = (a[0], a[1]);
    let consumed = e.mem.u32(buffer + 0x0c);
    let key = e.mem.u32(e.mem.u32(buffer + 4).wrapping_add(consumed));
    e.mem.set_u32(buffer + 0x0c, consumed.wrapping_add(5));
    if key == 0 {
        return Some(0);
    }
    let current = extra::get_by_type(e, list, EXTRA_JIP);
    if current == 0 {
        add_extra_jip(e, list, key);
        return Some(0);
    }
    let old = key_of(e, current);
    if old != key {
        e.mem.set_u32(current + 0x0c, key);
        saved(e).extra_data.erase(&Uint(old));
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetPickedCountHook.
/// `004c1c90`'s call of `RemoveExtraCount` (`0x4C21EB`: the reference's list, the count): with
/// a count extra the count to go on with is its value (a signed 16-bit number) and the extra is
/// removed (`BaseExtraList::RemoveByType`); with none the count stands and nothing is removed.
fn picked_count(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (list, count) = (a[0], a[1]);
    let extra = extra::get_by_type(e, list, EXTRA_COUNT);
    if extra == 0 {
        return Some(count);
    }
    let picked = e.mem.u16(extra + 0x0c) as i16 as i32;
    extra::remove_by_type(e, list, EXTRA_COUNT);
    Some(picked as u32)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: CopyExtraListHook.
/// `004c1c90`'s call of `CopyListForContainer` (`0x4C2291`: the new list, the count): a count
/// above 1 is added to the new list as a count extra first; the call then runs as before.
fn copy_count(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (list, count) = (a[0], a[1] as i32);
    // nv-rs's own guard: JIP adds to the list without a test; a failed allocation leaves none.
    if count > 1 && list != 0 {
        add_extra_count(e, list, count);
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ExtraJIPDestroyHook.
/// The type's scalar deleting destructor (slot 0, `thiscall`: the ExtraJIP, `doFree`): the key's
/// data is dropped (and the extra data record marked changed) unless the data names a reference
/// that is gone while the save holds changes of it (kept for that reference's next load); the
/// block freed to the game's heap when the flag's low byte is set. The base destructor is not
/// run (JIP's routine does not call it).
fn destroy(e: &mut Engine, a: &[u32]) -> Ret {
    let (this, free) = (a[0], a[1] as u8);
    let key = key_of(e, this);
    if key != 0 {
        if let Some(ref_id) = saved(e)
            .extra_data
            .get(&Uint(key))
            .map(|entry| entry.ref_id)
        {
            if ref_id == 0
                || cosave::lookup_form(e, ref_id) != 0
                || !cosave::has_change_data(e, ref_id)
            {
                saved(e).extra_data.erase(&Uint(key));
                saved(e).changed |= changed::EXTRA_DATA;
            }
        }
    }
    if free != 0 {
        heap_free(e, this);
    }
    // JIP's routine is `void` (EAX is whatever its last call left); the exe's answers `this`.
    Ret {
        eax: this,
        ..Ret::default()
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ExtraJIPDiffersHook.
/// The type's `Compare` (slot 1, `thiscall`: the ExtraJIP, the other extra data): the same
/// type and both keys with data: whether the data differ; else they differ.
fn differs(e: &mut Engine, a: &[u32]) -> Ret {
    let (this, other) = (a[0], a[1]);
    let mut differs = true;
    if e.mem.u8(this + 4) == e.mem.u8(other + 4) {
        let (k1, k2) = (key_of(e, this), key_of(e, other));
        let map = &saved(e).extra_data;
        if let (Some(x), Some(y)) = (map.get(&Uint(k1)), map.get(&Uint(k2))) {
            differs = !same_data(x, y);
        }
    }
    Ret {
        eax: u32::from(differs),
        ..Ret::default()
    }
}

#[cfg(test)]
#[path = "gh1_tests.rs"]
mod tests;
