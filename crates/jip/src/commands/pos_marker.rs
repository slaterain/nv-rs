//! JIP LN NVSE's temporary position marker (`s_tempPosMarker`, `internal/jip_core.cpp`): one
//! reference JIP makes at its deferred start (`DeferredInit`, `internal/patches_game.h`, at
//! xNVSE's `DeferredInit` message, the end of the first frame) and keeps for the whole run. Its
//! commands write a cell and a position into it and hand it to the game as the place to put
//! something (`PlaceAtCell`: the game's `PlaceAtMe`) or to move to (JIP's
//! `TESObjectREFR::MoveToCell`: the game's `MoveToMarker`).
//!
//! What it is (JIP: `ThisCall<TESObjectREFR*>(0x55A2F0, Game_HeapAlloc<TESObjectREFR>())`, then
//! `ThisCall(0x484490, s_tempPosMarker)`): a block of the game's heap of a reference's size,
//! `0x68` (the game's own `CreateReference`, `00564480`, pushes `0x68` before the same
//! constructor), built by the game's `TESObjectREFR` constructor (`0055A2F0`, `thiscall`, returns
//! the reference), then made temporary by the game's `TESForm::SetTemporary` (`00484490`,
//! `thiscall`: `TESForm::RemoveFromDataStructures` (`00483C70`: out of the form map and the data
//! handler's ids), its changes dropped from `TESSaveLoadGame` (`*011DE45C`, `00856C70`) and
//! `BGSSaveLoadGame` (`*011DDF38`, `0084A7C0`), flag `0x4000` set in `+8`).
//!
//! Why it never reaches a save or a cell: it is temporary, in no form map and with no change data,
//! and nothing adds it to a cell's references: the commands only write its `parentCell` field
//! (`+0x40`) and its position (`+0x30`); the game's `PlaceAtMe` and `MoveToMarker` read them. JIP
//! never frees it; neither does nv-rs (one per run).
//!
//! nv-rs makes it at the same message, in JIP's listener ([`crate::cosave::listen`]), only while
//! JIP answers "installed" (until step 3.10 no JIP command runs, so nothing would use it, and a
//! JIP that is not installed must not touch the game's heap), and only once (xNVSE sends the
//! message once per run).

use super::heap_alloc;
use crate::Jip;
use engine_core::prelude::*;

/// `sizeof(TESObjectREFR)`: what the game's `CreateReference` (`00564480`) allocates before
/// calling the constructor.
pub const REFR_SIZE: u32 = 0x68;
/// The game's `TESObjectREFR::TESObjectREFR` (`thiscall`, returns the reference).
pub const REFR_CONSTRUCTOR: u32 = 0x0055_a2f0;
/// The game's `TESForm::SetTemporary` (`thiscall`).
pub const SET_TEMPORARY: u32 = 0x0048_4490;
/// `TESObjectREFR::position` (`data.Location`, Xbox PDB; JIP's `// 30`): x, y, z floats.
pub const POSITION: u32 = 0x30;
/// `TESObjectREFR::parentCell` (`+0x40`, the exe's `GetParentCell`, `008D6F30`).
pub const PARENT_CELL: u32 = 0x40;

/// `s_tempPosMarker`: the marker, 0 until JIP's deferred start made it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub marker: u32,
}

/// The marker, or 0 before it is made.
pub fn marker(e: &mut Engine) -> u32 {
    e.ext::<Jip>().pos_marker.marker
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit (the
// marker's two lines).
/// At xNVSE's `DeferredInit`: the marker made (a reference's block of the game's heap, the
/// game's constructor, then made temporary), while JIP answers "installed" and once.
pub fn create(e: &mut Engine) {
    let jip = e.ext::<Jip>();
    if !jip.answers_installed || jip.pos_marker.marker != 0 {
        return;
    }
    let block = heap_alloc(e, REFR_SIZE);
    let marker = e.call(REFR_CONSTRUCTOR, &args![block]).u32();
    e.call(SET_TEMPORARY, &args![marker]);
    e.ext::<Jip>().pos_marker.marker = marker;
}

#[cfg(test)]
#[path = "pos_marker_tests.rs"]
mod tests;
