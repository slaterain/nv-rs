//! `functions_ln/ln_fn_map_marker.h` (JIP LN NVSE): a map marker reference's name, flags, type and
//! reputation. All 12 of its commands, each called on a reference.
//!
//! JIP's `TESObjectREFR::GetMapMarkerData` (`nvse/GameObjects.cpp`): a reference whose base form
//! has the id 0x10 (the game's map marker static) and an `ExtraMapMarker` (extra type 0x2C) in its
//! extra data list (+0x44); its data is the pointer at +0xC of that extra. The marker data, checked
//! in `MapMarkerData::Load` (`00438CA0`): the name (`FULL`, a `TESFullName`, its string at +4),
//! `FNAM` flags at +0xC (1 visible, 2 can travel, 4 hidden), `TNAM` type (16 bits) at +0xE, the
//! reputation (`WMI1`) at +0x10.
//!
//! The extra is found by the game's `BaseExtraList::GetByType` (`00410220`, as xNVSE's reader finds
//! extras). JIP calls its own copy of it (`nvse/GameBSExtraData.cpp`), which walks the same list
//! under the same lock (`0x11C3920`) and gives the same answer; the game's also remembers the extra
//! it found for its next lookup, which changes no answer.

use super::forms::{form_type_ln, MapMarkerData};
use super::{
    assign_string, buffer_arg, ref_id, set_form_result, string_set, string_text, type_of, Handlers,
};
use crate::game::TESObjectREFR;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x222d, get_map_marker_rep),
    (0x222e, set_map_marker_rep),
    (0x222f, get_map_marker_name),
    (0x2230, set_map_marker_name),
    (0x2231, get_map_marker_type),
    (0x2232, set_map_marker_type),
    (0x2234, set_map_marker_visible),
    (0x2235, get_map_marker_hidden),
    (0x2236, set_map_marker_hidden),
    (0x2237, get_map_marker_travel),
    (0x2238, set_map_marker_travel),
    (0x2249, is_map_marker),
];

/// The game's map marker static (`refID == 0x10`).
const MAP_MARKER_ID: u32 = 0x10;
/// `kXData_ExtraMapMarker` (JIP LN NVSE).
const EXTRA_MAP_MARKER: u32 = 0x2c;
/// `BaseExtraList::GetByType` (`00410220`, `thiscall`: the list, the type).
const EXTRA_GET_BY_TYPE: u32 = 0x0041_0220;
/// The marker data pointer in the `ExtraMapMarker` (+0xC).
const EXTRA_DATA_WORD: u32 = 0x0c;
/// JIP's name buffer (`char nameStr[0x80]`).
const NAME_BUFFER: usize = 0x80;

/// The flags: visible, can travel, hidden.
const VISIBLE: u8 = 1;
const TRAVEL: u8 = 2;
const HIDDEN: u8 = 4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetMapMarkerData.
/// The marker data of `refr`, `None` for no base form, another base form or no extra.
fn marker_data(e: &mut Engine, refr: u32) -> Option<Ptr<MapMarkerData>> {
    let r = Ptr::<TESObjectREFR>::new(refr);
    let base = e.get(r, TESObjectREFR::pObjectReference);
    if base == 0 || ref_id(e, base) != MAP_MARKER_ID {
        return None;
    }
    let list = r.at(TESObjectREFR::ExtraData).addr();
    let extra = e
        .call(EXTRA_GET_BY_TYPE, &args![list, EXTRA_MAP_MARKER])
        .u32();
    if extra == 0 {
        return None;
    }
    Some(Ptr::new(e.mem.u32(extra + EXTRA_DATA_WORD)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_IsMapMarker_Execute.
/// 1 when the reference's base form is the map marker static (JIP reads the base form without a
/// null check: a reference always has one).
fn is_map_marker(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let base = e.get(
        Ptr::<TESObjectREFR>::new(a.this_obj),
        TESObjectREFR::pObjectReference,
    );
    if ref_id(e, base) == MAP_MARKER_ID {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_GetMapMarkerName_Execute.
fn get_map_marker_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let text = marker_data(e, a.this_obj).and_then(|d| {
        let name = d.at(MapMarkerData::name).addr();
        string_text(e, name)
    });
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_SetMapMarkerName_Execute.
fn set_map_marker_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = buffer_arg(e, &read, 0, NAME_BUFFER, "SetMapMarkerName");
    if let Some(d) = marker_data(e, a.this_obj) {
        string_set(e, d.at(MapMarkerData::name).addr(), &name);
    }
    handled(true)
}

/// A flag set from the script's integer: on for non-zero, off for 0.
fn set_flag(e: &mut Engine, w: &[u32], bit: u8) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    if let Some(d) = marker_data(e, a.this_obj) {
        let flags = e.get(d, MapMarkerData::flags);
        let flags = if read.int(0, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(d, MapMarkerData::flags, flags);
    }
    handled(true)
}

/// 1 when the flag is on.
fn get_flag(e: &mut Engine, w: &[u32], bit: u8) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(d) = marker_data(e, a.this_obj) {
        if e.get(d, MapMarkerData::flags) & bit != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_SetMapMarkerVisible_Execute.
fn set_map_marker_visible(e: &mut Engine, w: &[u32]) -> Ret {
    set_flag(e, w, VISIBLE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_GetMapMarkerTravel_Execute.
fn get_map_marker_travel(e: &mut Engine, w: &[u32]) -> Ret {
    get_flag(e, w, TRAVEL)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_SetMapMarkerTravel_Execute.
fn set_map_marker_travel(e: &mut Engine, w: &[u32]) -> Ret {
    set_flag(e, w, TRAVEL)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_GetMapMarkerHidden_Execute.
fn get_map_marker_hidden(e: &mut Engine, w: &[u32]) -> Ret {
    get_flag(e, w, HIDDEN)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_SetMapMarkerHidden_Execute.
fn set_map_marker_hidden(e: &mut Engine, w: &[u32]) -> Ret {
    set_flag(e, w, HIDDEN)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_GetMapMarkerType_Execute.
/// The type (16 bits, unsigned); -1 for no marker data (the result is always written).
fn get_map_marker_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mut kind = -1;
    if let Some(d) = marker_data(e, a.this_obj) {
        kind = i32::from(e.get(d, MapMarkerData::kind));
    }
    a.set_result(e, f64::from(kind));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_SetMapMarkerType_Execute.
/// Types 0-14 (the script's `UInt32`, stored in 16 bits); larger, nothing.
fn set_map_marker_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(0, 0);
    if read.ok && kind <= 14 {
        if let Some(d) = marker_data(e, a.this_obj) {
            e.set(d, MapMarkerData::kind, kind as u16);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_GetMapMarkerRep_Execute.
fn get_map_marker_rep(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(d) = marker_data(e, a.this_obj) {
        let rep = e.get(d, MapMarkerData::reputation);
        if rep != 0 {
            set_form_result(e, &a, rep);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_map_marker.h:
// Cmd_SetMapMarkerRep_Execute.
/// The reputation set, or cleared without a form; another form type, nothing.
fn set_map_marker_rep(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let rep = read.form(0);
    if !read.ok || (rep != 0 && type_of(e, rep) != form_type_ln::TES_REPUTATION) {
        return handled(true);
    }
    if let Some(d) = marker_data(e, a.this_obj) {
        e.set(d, MapMarkerData::reputation, rep);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_map_marker_tests.rs"]
mod tests;
