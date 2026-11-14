//! JIP's extra data helpers the commands of batch 10 share (`nvse/GameBSExtraData.cpp`,
//! `nvse/GameExtraData.cpp` (JIP LN NVSE)): the lookup by type (the game's `GetByType`, as the map
//! marker commands call it), the presence test, adding and removing by type (the game's own
//! functions, which JIP calls), JIP's `CreateExtraType`, and `TESObjectREFR::MarkModified`.
//!
//! The exe's functions: `BaseExtraList::GetByType` `00410220` (`thiscall`: the list, the type;
//! the presence bit, then the walk), `BaseExtraList::AddExtra` `0040FF60` (`thiscall`: the list,
//! the extra; appends it and answers it), `BaseExtraList::RemoveByType` `00410140` (`thiscall`:
//! unlinks the first extra of the type, deletes it through its vtable, clears the presence bit).

use super::heap_alloc;
use engine_core::prelude::*;

/// `BaseExtraList::GetByType` (`00410220`).
const GET_BY_TYPE: u32 = 0x0041_0220;
/// `BaseExtraList::AddExtra` (`0040FF60`).
const ADD_EXTRA: u32 = 0x0040_ff60;
/// `BaseExtraList::RemoveByType` (`00410140`).
const REMOVE_BY_TYPE: u32 = 0x0041_0140;
/// `g_BGSSaveLoadGame` and `BGSSaveLoadGame::AddChange` (`0084A690`, `thiscall`: the form, the
/// flags, 0): JIP's `TESObjectREFR::MarkModified`.
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
const ADD_CHANGE: u32 = 0x0084_a690;
/// A reference's extra data list (`extraDataList`, +0x44).
pub const REFR_EXTRA_LIST: u32 = 0x44;
/// The presence bits of an extra data list (`m_presenceBitfield`, +8).
const PRESENCE: u32 = 0x08;

/// The extra data types these commands use (`kXData_*` (JIP LN NVSE), the Xbox PDB's
/// `EXTRA_DATA_TYPE`).
pub mod kind {
    pub const CELL_WATER_TYPE: u32 = 0x03;
    pub const CELL_CLIMATE: u32 = 0x08;
    pub const PERSISTENT_CELL: u32 = 0x0c;
    pub const ACTION: u32 = 0x0e;
    pub const CONTAINER_CHANGES: u32 = 0x15;
    pub const LOCK: u32 = 0x2a;
    pub const TELEPORT: u32 = 0x2b;
    pub const MAP_MARKER: u32 = 0x2c;
    pub const NORTH_ROTATION: u32 = 0x43;
    pub const TERMINAL_STATE: u32 = 0x50;
    pub const ACTIVATE_REF: u32 = 0x53;
    pub const CELL_IMAGE_SPACE: u32 = 0x59;
    pub const RADIUS: u32 = 0x5c;
    pub const RADIO_DATA: u32 = 0x68;
}

/// The vtables JIP's `Create` functions store (`kVtbl_*` (JIP LN NVSE)), each the one the exe's
/// constructor stores (`extradatacell.rs`, `extradatalist_p2.rs`, `extradatalist_p4.rs`,
/// `extradataobjects.rs` in engine-fallout-cell).
pub mod vtbl {
    pub const CELL_CLIMATE: u32 = 0x0101_424c;
    pub const CELL_WATER_TYPE: u32 = 0x0101_4270;
    pub const TERMINAL_STATE: u32 = 0x0101_5190;
    pub const RADIUS: u32 = 0x0101_5208;
    pub const LOCK: u32 = 0x0101_589c;
}

/// `GetExtraType(list, Type)`: the list's first extra of `kind`, 0 for none.
pub fn get_by_type(e: &mut Engine, list: u32, kind: u32) -> u32 {
    e.call(GET_BY_TYPE, &args![list, kind]).u32()
}

/// The reference's extra of `kind` (its list at +0x44), 0 for none.
pub fn refr_extra(e: &mut Engine, refr: u32, kind: u32) -> u32 {
    get_by_type(e, refr + REFR_EXTRA_LIST, kind)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameBSExtraData.cpp: BaseExtraList::HasType.
/// Whether the list's presence bit for `kind` is set.
pub fn has_type(e: &Engine, list: u32, kind: u32) -> bool {
    e.mem.u8(list + PRESENCE + (kind >> 3)) & (1 << (kind & 7)) != 0
}

/// `BaseExtraList::AddExtra`: the extra appended; answers it.
pub fn add_extra(e: &mut Engine, list: u32, extra: u32) -> u32 {
    e.call(ADD_EXTRA, &args![list, extra]).u32()
}

/// `BaseExtraList::RemoveByType`.
pub fn remove_by_type(e: &mut Engine, list: u32, kind: u32) {
    e.call(REMOVE_BY_TYPE, &args![list, kind]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.h: CreateExtraType.
/// A 0x10-byte block of the game's heap with the vtable at +0, the type as a whole word at +4
/// (`dataPtr[1] = kXData_*`: the type byte and three zero bytes) and 0 at +8; the word at +0xC is
/// the caller's to set.
pub fn create(e: &mut Engine, vtable: u32, kind: u32) -> u32 {
    let extra = heap_alloc(e, 0x10);
    e.mem.set_u32(extra, vtable);
    e.mem.set_u32(extra + 4, kind);
    e.mem.set_u32(extra + 8, 0);
    extra
}

/// `TESObjectREFR::MarkModified(flags)` (JIP LN NVSE): `BGSSaveLoadGame::AddChange(refr, flags,
/// 0)`.
pub fn mark_modified(e: &mut Engine, refr: u32, flags: u32) {
    let sl = e.mem.u32(SAVE_LOAD_GAME);
    e.call(ADD_CHANGE, &args![sl, refr, flags, 0u32]);
}
