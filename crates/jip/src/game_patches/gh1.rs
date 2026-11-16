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
use crate::commands::{extra, heap_alloc, heap_free};
use crate::containers::Uint;
use crate::cosave::{self, changed, ExtraJipData, ExtraJipEntry};
use crate::overrides::{base_container, changes_list, form_count};
use crate::Jip;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

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
    fix_sites: &[
        HOTKEY_MOVED.addr(),
        COMPARE_OWNED.addr(),
        PERSISTS_LIST.addr(),
        MOD_FLAGS.addr(),
        HAS_MOD.addr(),
        ITERATOR_DESTROY.addr(),
        PERSISTS_PICKED.addr(),
        PERSISTS_MERGED.addr(),
        HOTKEY_REMOVED.addr(),
        ITERATOR_START.addr(),
        ITERATOR_NEXT.addr(),
        HOTKEY_ACTIVATED.addr(),
    ],
    fix_wraps: &[ITEMS_HEALTH.addr()],
    install_fixes,
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
pub(crate) fn add_extra_count(e: &mut Engine, list: u32, count: i32) -> u32 {
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

// ===================== The fixes (the row `NV_JIP_GAME_PATCHES`: jip_patches::FIXES) =====================

/// The fix seams (`SITES` rows of `patch_rows/gh1.rs`).
pub const COMPARE_OWNED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0041_271e);
pub const HOTKEY_MOVED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0041_2675);
pub const PERSISTS_LIST: SiteSlot = SiteSlot::new(Owner::Jip, 0x0041_c833);
pub const MOD_FLAGS: SiteSlot = SiteSlot::new(Owner::Jip, 0x004b_d820);
pub const HAS_MOD: SiteSlot = SiteSlot::new(Owner::Jip, 0x004b_da70);
pub const ITERATOR_DESTROY: SiteSlot = SiteSlot::new(Owner::Jip, 0x004b_f630);
pub const PERSISTS_PICKED: SiteSlot = SiteSlot::new(Owner::Jip, 0x004c_232c);
pub const PERSISTS_MERGED: SiteSlot = SiteSlot::new(Owner::Jip, 0x004c_273a);
pub const HOTKEY_REMOVED: SiteSlot = SiteSlot::new(Owner::Jip, 0x004c_3976);
pub const ITERATOR_START: SiteSlot = SiteSlot::new(Owner::Jip, 0x004c_a200);
pub const ITERATOR_NEXT: SiteSlot = SiteSlot::new(Owner::Jip, 0x004c_afa3);
/// In group GP's translation (`00953ff0`, `playercharacter_p2.rs`): GP adds the inert seam.
pub const HOTKEY_ACTIVATED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0095_400f);
/// The fix wrap: `00482090` (`thiscall`: the container, the health), which no Rust code calls.
pub const ITEMS_HEALTH: WrapSlot = WrapSlot::new(Owner::Jip, 0x0048_2090);

/// The form types and extra data types JIP's routines test.
pub const FORM_ARMOR: u8 = 0x18;
pub const FORM_MISC: u8 = 0x1f;
pub const FORM_WEAPON: u8 = 0x28;
pub const FORM_AMMO: u8 = 0x29;
pub const FORM_ALCHEMY: u8 = 0x2f;
pub const EXTRA_ITEM_DROPPER: u32 = 0x39;
pub const EXTRA_HOTKEY: u32 = 0x4a;
pub const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8d;
/// `g_thePlayer` (`PlayerCharacter::GetSingleton`'s word).
pub const PLAYER: u32 = 0x011d_ea3c;
/// `TESContainer`'s copy of its object list (`00482B00`, `thiscall`), which JIP's iterator calls.
pub const CONTAINER_LIST_COPY: u32 = 0x0048_2b00;
/// JIP's word after the exe's iterator words: the changes' entry list (`baseEntryList`).
pub const BASE_ENTRY_LIST: u32 = 0x28;

fn install_fixes(e: &mut Engine) {
    e.ext_free::<Iterator>();
    e.on_site(COMPARE_OWNED, answer_yes);
    e.on_site(HOTKEY_MOVED, answer_yes);
    e.on_site(PERSISTS_LIST, persists);
    e.on_site(MOD_FLAGS, mod_flags_site);
    e.on_site(HAS_MOD, has_mod_site);
    e.on_site(ITERATOR_DESTROY, iterator_destroy);
    e.on_site(PERSISTS_PICKED, persists);
    e.on_site(PERSISTS_MERGED, persists);
    e.on_site(HOTKEY_REMOVED, hotkey_removed);
    e.on_site(ITERATOR_START, iterator_start);
    e.on_site(ITERATOR_NEXT, iterator_next);
    e.on_site(HOTKEY_ACTIVATED, hotkey_activated);
    e.wrap(ITEMS_HEALTH, items_health);
}

/// `0x41271E` (G46: `CompareListForContainer`'s `jnz` after the ownership test becomes `jmp`: an
/// owner extra is no difference at once) and `0x412675` (G76: `CopyListForReference`'s byte
/// table entry of type 0x4A, the hotkey, 2 becomes 1: copied with the list): JIP's branch taken.
fn answer_yes(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

// ---- G46: the inventory "invisible stack" fix ----

/// JIP's one inventory iterator (`s_inventoryIterator`, a `TempObject<InventoryIterator>` in its
/// DLL): the exe's 0x28 bytes, then JIP's `baseEntryList` (+0x28), in a block of the game's heap
/// made at the first iteration and kept; JIP's map of the counts handed out per form
/// (`deltasMap`, +0x2C) is port-side, sorted by the form's address as JIP's `Map` is.
#[derive(Default)]
pub struct Iterator(Mutex<IteratorState>);

#[derive(Default)]
pub struct IteratorState {
    pub block: u32,
    pub handed: BTreeMap<u32, i32>,
}

/// The iterator's port-side state.
pub fn iterator_state(e: &mut Engine) -> MutexGuard<'_, IteratorState> {
    e.ext_free::<Iterator>()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// `0x4BF630` (`OEI_Fast_InventoryIterator`'s scalar deleting destructor: this, flags): JIP
/// writes `retn 4` over its head (`SafeWrite32(0x4BF630, 0x4C2)`): nothing is run or freed (the
/// iterator is JIP's own). Deliberate: answers `this` (JIP leaves EAX as it was).
fn iterator_destroy(_: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(a[0])
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/containers.h: FormCountList::RemoveAll;
// GameAPI.h: Game_HeapFree.
/// A container list copy freed: its nodes after the head, the head's two words cleared, the head.
fn free_count_list(e: &mut Engine, head: u32) {
    let mut node = e.mem.u32(head + 4);
    while node != 0 {
        let next = e.mem.u32(node + 4);
        heap_free(e, node);
        node = next;
    }
    e.mem.set_u32(head, 0);
    e.mem.set_u32(head + 4, 0);
    heap_free(e, head);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h:
// CreateInventoryIteratorHook.
/// `0x4CA200` (`StartOEIFastInventoryIteration`: the changes' data) rewritten: the previous
/// iteration's copy of the container's list freed, the 0x28 bytes cleared, +0x28 the changes'
/// entry list, the map emptied; with an owner its base form's container in +0x10, its object list
/// in +8 and the list's copy (`00482B00`) in +0 and +4. Answers JIP's iterator. Deliberate: an
/// owner whose base form has no container gets none (JIP would copy from a null container).
fn iterator_start(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let changes = a[0];
    let mut block = iterator_state(e).block;
    if block == 0 {
        block = heap_alloc(e, 0x2c);
        iterator_state(e).block = block;
    } else {
        let copy = e.mem.u32(block);
        if copy != 0 {
            free_count_list(e, copy);
        }
    }
    for offset in (0..0x28).step_by(4) {
        e.mem.set_u32(block + offset, 0);
    }
    e.mem.set_u32(block + BASE_ENTRY_LIST, e.mem.u32(changes));
    iterator_state(e).handed.clear();
    let owner = e.mem.u32(changes + 4);
    if owner != 0 {
        let container = base_container(e, e.mem.u32(owner + 0x20));
        e.mem.set_u32(block + 0x10, container);
        if container != 0 {
            e.mem.set_u32(block + 8, container + 4);
            let copy = e.call(CONTAINER_LIST_COPY, &args![container]).u32();
            e.mem.set_u32(block, copy);
            e.mem.set_u32(block + 4, copy);
        }
    }
    Some(block)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InventoryIterGetNextHook.
/// `0x4CAFA3` (`GetNextOEIFastInventoryItem`'s end: the iterator). A stack handed out (+0xC) adds
/// its count (+4) to its form's in the map (a new form starts at 0) and is answered. None: the
/// pass word's low byte (+0x18) becomes 2 and the map is taken from its highest form down: the
/// first form whose count (`GetFormCount` over the container's list (+8) and the changes (+0x28))
/// is above what was handed out answers a new stack (0xC bytes: no extra lists, the rest, the
/// form); none left answers 0.
fn iterator_next(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let iterator = a[0];
    let out = e.mem.u32(iterator + 0x0c);
    if out != 0 {
        let (form, count) = (e.mem.u32(out + 8), e.mem.i32(out + 4));
        let mut state = iterator_state(e);
        let handed = state.handed.entry(form).or_insert(0);
        *handed = handed.wrapping_add(count);
        return Some(out);
    }
    e.mem.set_u8(iterator + 0x18, 2);
    loop {
        let Some((form, handed)) = iterator_state(e).handed.pop_last() else {
            return Some(0);
        };
        let list = e.mem.u32(iterator + 8);
        let changes = e.mem.u32(iterator + BASE_ENTRY_LIST);
        let rest = form_count(e, list, changes, form).wrapping_sub(handed);
        if rest > 0 {
            let entry = heap_alloc(e, 0x0c);
            e.mem.set_u32(entry, 0);
            e.mem.set_i32(entry + 4, rest);
            e.mem.set_u32(entry + 8, form);
            return Some(entry);
        }
    }
}

// ---- G79: a weapon's mod flags, null-safe, from the dropper's list ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetEntryDataModFlagsHook.
/// A stack's weapon mod flags: 0 for no stack or a form that is not a weapon, or without a first
/// extra list; else the flags extra's byte (+0xC) of the list of the reference that dropped it
/// (the list's `ExtraItemDropper` +0xC, its list at +0x44) when there is one, else of the first
/// list; 0 without a flags extra.
pub fn mod_flags(e: &mut Engine, entry: u32) -> u8 {
    if entry == 0 || e.mem.u8(e.mem.u32(entry + 8) + 4) != FORM_WEAPON {
        return 0;
    }
    let node = e.mem.u32(entry);
    if node == 0 {
        return 0;
    }
    let mut list = e.mem.u32(node);
    if list == 0 {
        return 0;
    }
    let dropper = extra::get_by_type(e, list, EXTRA_ITEM_DROPPER);
    if dropper != 0 {
        let refr = e.mem.u32(dropper + 0x0c);
        if refr != 0 {
            list = refr + extra::REFR_EXTRA_LIST;
        }
    }
    let flags = extra::get_by_type(e, list, EXTRA_WEAPON_MOD_FLAGS);
    if flags == 0 {
        return 0;
    }
    e.mem.u8(flags + 0x0c)
}

/// `0x4BD820` (`ItemChange::GetModSlots`: the stack): JIP's routine in place of the exe's.
fn mod_flags_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(u32::from(mod_flags(e, a[0])))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetEntryDataHasModHook.
/// `0x4BDA70` (`ItemChange::HasModEffectActive_ov2`: the stack, the mod effect): true when a slot
/// whose flag (1, 2, 4, in that order) is set has the effect as its weapon's action type (the
/// byte at +0x180, +0x184, +0x188 of the stack's form).
fn has_mod_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (entry, effect) = (a[0], a[1] as u8);
    let flags = mod_flags(e, entry);
    if flags == 0 {
        return Some(0);
    }
    let weapon = e.mem.u32(entry + 8);
    let found = [(1u8, 0x180u32), (2, 0x184), (4, 0x188)]
        .iter()
        .any(|&(bit, at)| flags & bit != 0 && e.mem.u8(weapon + at) == effect);
    Some(u32::from(found))
}

// ---- G50: a container's items' health ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetContainerItemsHealthHook.
/// `00482090` (`thiscall`: the container, the health) rewritten: the health clamped to 0..0.999
/// (`minss` with 0.999 (`0x3F7FBE77`), then `maxss` with 0: not a number gives 0.999). Each item
/// (the list at +4; empty nodes passed over): armour, and a weapon whose type byte (+0xF4) is
/// below 10, gets the health (a new 0xC-byte extra holding it, or the lower of the two kept); any
/// other item with an extra has its health set to 1.0, and the extra is freed when its owner word
/// (+0) is 0.
fn items_health(e: &mut Engine, w: &[u32]) -> Ret {
    let container = w[0];
    let max = f32::from_bits(0x3f7f_be77);
    let given = f32::from_bits(w[1]);
    let given = if given < max { given } else { max };
    let given = if given > 0.0 { given } else { 0.0 };
    let mut node = container + 4;
    while node != 0 {
        let item = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if item == 0 {
            continue;
        }
        let extra = e.mem.u32(item + 8);
        let form = e.mem.u32(item + 4);
        let kind = e.mem.u8(form + 4);
        if kind == FORM_ARMOR || (kind == FORM_WEAPON && e.mem.u8(form + 0xf4) < 10) {
            if extra == 0 {
                let made = heap_alloc(e, 0x0c);
                e.mem.set_u32(item + 8, made);
                e.mem.set_u32(made, 0);
                e.mem.set_u32(made + 4, 0);
                e.mem.set_f32(made + 8, given);
            } else {
                let old = e.mem.f32(extra + 8);
                e.mem
                    .set_f32(extra + 8, if given < old { given } else { old });
            }
        } else if extra != 0 {
            e.mem.set_u32(extra + 8, 0x3f80_0000);
            if e.mem.u32(extra) == 0 {
                e.mem.set_u32(item + 8, 0);
                heap_free(e, extra);
            }
        }
    }
    Ret::default()
}

// ---- G61: temporary forms are never persistent ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: IsPersistentHook.
/// `0x41C833`, `0x4C232C`, `0x4C273A` (the calls of `TESObjectREFR::GetRefPersists`: the
/// reference): a temporary form (mod index byte +0xF is 0xFF) answers false; any other no answer
/// (the exe's call runs, as JIP jumps to it).
fn persists(e: &mut Engine, a: &[u32]) -> Option<u32> {
    (e.mem.u8(a[0] + 0x0f) == 0xff).then_some(0)
}

// ---- G75 + G76: the hotkey fix ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ClearExtraHotkeyIfUsed.
/// When `list` holds a hotkey and one of the player's stacks (with extra lists; ammo, misc items
/// and forms above `AlchemyItem` passed over) has an extra list holding the same key, `list` loses
/// its hotkey (JIP removes it from the list it was given, `pop ecx`).
pub fn clear_hotkey_if_used(e: &mut Engine, list: u32) {
    let hotkey = extra::get_by_type(e, list, EXTRA_HOTKEY);
    if hotkey == 0 {
        return;
    }
    let key = e.mem.u8(hotkey + 0x0c);
    let player = e.mem.u32(PLAYER);
    let mut node = changes_list(e, player);
    while node != 0 {
        let entry = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if entry == 0 || e.mem.u32(entry) == 0 {
            continue;
        }
        let kind = e.mem.u8(e.mem.u32(entry + 8) + 4);
        if kind == FORM_AMMO || kind == FORM_MISC || kind > FORM_ALCHEMY {
            continue;
        }
        let mut lists = e.mem.u32(entry);
        while lists != 0 {
            let other = e.mem.u32(lists);
            lists = e.mem.u32(lists + 4);
            if other == 0 {
                continue;
            }
            let found = extra::get_by_type(e, other, EXTRA_HOTKEY);
            if found != 0 && e.mem.u8(found + 0x0c) == key {
                extra::remove_by_type(e, list, EXTRA_HOTKEY);
                return;
            }
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: HotkeyFixRemoveItemHook.
/// `0x4C3976` (`004c37d0`, an item removed: the drop flag, the extra list, the target): in place
/// of the exe's hotkey test (to `0x4C39B0`), an item given to the player (the target's form id is
/// 0x14), not dropped, with a list, clears a hotkey the player already uses. Answers (the exe's
/// block is skipped).
fn hotkey_removed(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (dropped, list, target) = (a[0] as u8, a[1], a[2]);
    if dropped == 0 && list != 0 && target != 0 && e.mem.u32(target + 0x0c) == 0x14 {
        clear_hotkey_if_used(e, list);
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: HotkeyFixPlayerActivateHook.
/// `0x95400F` (`00953ff0`, the player picks up a reference: the reference): its list's hotkey is
/// cleared when the player already uses it; the translation goes on (JIP's routine reads the base
/// form's type as the exe does). No answer.
fn hotkey_activated(e: &mut Engine, a: &[u32]) -> Option<u32> {
    clear_hotkey_if_used(e, a[0] + extra::REFR_EXTRA_LIST);
    None
}

#[cfg(test)]
#[path = "gh1_tests.rs"]
mod tests;
