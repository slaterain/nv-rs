//! `functions_jip/jip_fn_worldspace.h` (JIP LN NVSE): a worldspace's flags, climate, water noise
//! texture, water, image space and persistent cell. All 11 of its commands. The worldspace is read
//! as `kParamType_WorldSpace`, whose type check xNVSE's reader makes, except by
//! `SetWorldspaceNoiseTexture` (any form, then `IS_ID(wspc, TESWorldSpace)`, type 0x41).
//!
//! A worldspace that uses its parent's climate has a parent and bit 0x10 of its parent flags (the
//! worldspace loader `00583560` reads `PNAM` into the flags at +0x4E, `DATA` into the byte at
//! +0x4C); the climate commands walk up those parents.

use super::forms::{form_type, form_type_more, Sky, TESTexture, TESWorldSpace, SKY};
use super::{
    assign_string, buffer_arg, set_form_result, string_set, string_text, type_of, Handlers,
};
use crate::game::TESObjectREFR;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x26d3, get_worldspace_climate),
    (0x26d4, set_worldspace_climate),
    (0x26e7, get_worldspace_flag),
    (0x26e8, set_worldspace_flag),
    (0x27d7, get_worldspace_noise_texture),
    (0x27d8, set_worldspace_noise_texture),
    (0x27d9, get_worldspace_water_type),
    (0x27da, set_worldspace_water_type),
    (0x27db, get_worldspace_imagespace),
    (0x27dc, set_worldspace_imagespace),
    (0x292b, get_worldspace_persistent_cell),
];

const PATH_BUFFER: usize = 0x80;
/// The parent flag "use the parent's climate" (`parentFlags & 0x10`).
const USE_PARENT_CLIMATE: u16 = 0x10;
/// `g_thePlayer` (JIP LN NVSE): the player's reference.
const PLAYER: u32 = 0x011d_ea3c;

fn worldspace_arg(read: &Read) -> Option<Ptr<TESWorldSpace>> {
    read.ok.then(|| Ptr::new(read.form(0)))
}

/// The worldspace whose climate `w` uses: up through parents while the flag says so.
fn climate_root(e: &Engine, mut w: Ptr<TESWorldSpace>) -> Ptr<TESWorldSpace> {
    loop {
        let parent = e.get(w, TESWorldSpace::parent);
        if parent == 0 || e.get(w, TESWorldSpace::parentFlags) & USE_PARENT_CLIMATE == 0 {
            return w;
        }
        w = Ptr::new(parent);
    }
}

fn noise_path(w: Ptr<TESWorldSpace>) -> u32 {
    w.at(TESWorldSpace::waterNoiseTexture)
        .at(TESTexture::ddsPath)
        .addr()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_GetWorldspaceClimate_Execute.
/// The climate the worldspace uses (its own or an ancestor's), echoed to the console.
fn get_worldspace_climate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ws) = worldspace_arg(&read) {
        let root = climate_root(e, ws);
        let climate = e.get(root, TESWorldSpace::climate);
        if climate != 0 {
            set_form_result(e, &a, climate);
        }
        crate::console::print_form(e, &a, climate);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_SetWorldspaceClimate_Execute.
/// The worldspace's own climate set (or cleared), and it stops using its parent's; when the player
/// is in it (through the same walk up), the sky's current climate becomes it at once.
fn set_worldspace_climate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let climate = read.form(1);
    let Some(ws) = worldspace_arg(&read) else {
        return handled(true);
    };
    if climate != 0 && type_of(e, climate) != form_type::TES_CLIMATE {
        return handled(true);
    }
    let flags = e.get(ws, TESWorldSpace::parentFlags);
    if e.get(ws, TESWorldSpace::parent) != 0 && flags & USE_PARENT_CLIMATE != 0 {
        e.set(ws, TESWorldSpace::parentFlags, flags - USE_PARENT_CLIMATE);
    }
    e.set(ws, TESWorldSpace::climate, climate);
    let sky = Ptr::<Sky>::new(e.mem.u32(SKY));
    let player = Ptr::<TESObjectREFR>::new(e.mem.u32(PLAYER));
    if sky.is_null() {
        return handled(true);
    }
    let cell = e.get(player, TESObjectREFR::parentCell);
    if cell == 0 {
        return handled(true);
    }
    let pc_ws = e.get(
        Ptr::<super::forms::TESObjectCELL>::new(cell),
        super::forms::TESObjectCELL::worldSpace,
    );
    if pc_ws != 0 && climate_root(e, Ptr::new(pc_ws)) == ws {
        e.set(sky, Sky::currClimate, climate);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_GetWorldspaceFlag_Execute.
/// Flags 0-7, the bits of the flags byte.
fn get_worldspace_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(ws) = worldspace_arg(&read).filter(|_| flag <= 7) {
        if u32::from(e.get(ws, TESWorldSpace::worldFlags)) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_SetWorldspaceFlag_Execute.
fn set_worldspace_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(ws) = worldspace_arg(&read).filter(|_| flag <= 7) {
        let bit = 1u8 << flag;
        let flags = e.get(ws, TESWorldSpace::worldFlags);
        let flags = if read.int(2, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(ws, TESWorldSpace::worldFlags, flags);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_GetWorldspaceNoiseTexture_Execute.
fn get_worldspace_noise_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = worldspace_arg(&read).and_then(|ws| string_text(e, noise_path(ws)));
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_SetWorldspaceNoiseTexture_Execute.
fn set_worldspace_noise_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetWorldspaceNoiseTexture");
    let form = read.form(0);
    if read.ok && type_of(e, form) == form_type_more::TES_WORLD_SPACE {
        string_set(e, noise_path(Ptr::new(form)), &path);
    }
    handled(true)
}

/// A getter of a form field: its id, nothing when it is 0.
fn get_form(e: &mut Engine, w: &[u32], field: Field<TESWorldSpace, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ws) = worldspace_arg(&read) {
        let form = e.get(ws, field);
        if form != 0 {
            set_form_result(e, &a, form);
        }
    }
    handled(true)
}

/// A setter of a form field: set to a form of `kind` or cleared; another form type, nothing.
fn set_form(e: &mut Engine, w: &[u32], field: Field<TESWorldSpace, u32>, kind: u8) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(1);
    if let Some(ws) = worldspace_arg(&read) {
        if form == 0 || type_of(e, form) == kind {
            e.set(ws, field, form);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_GetWorldspaceWaterType_Execute.
fn get_worldspace_water_type(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, TESWorldSpace::waterFormFirst)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_SetWorldspaceWaterType_Execute.
fn set_worldspace_water_type(e: &mut Engine, w: &[u32]) -> Ret {
    set_form(
        e,
        w,
        TESWorldSpace::waterFormFirst,
        form_type_more::TES_WATER_FORM,
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_GetWorldspaceImagespace_Execute.
fn get_worldspace_imagespace(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, TESWorldSpace::imageSpace)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_SetWorldspaceImagespace_Execute.
fn set_worldspace_imagespace(e: &mut Engine, w: &[u32]) -> Ret {
    set_form(
        e,
        w,
        TESWorldSpace::imageSpace,
        form_type_more::TES_IMAGE_SPACE,
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_worldspace.h:
// Cmd_GetWorldspacePersistentCell_Execute.
fn get_worldspace_persistent_cell(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, TESWorldSpace::cell)
}

#[cfg(test)]
#[path = "jip_fn_worldspace_tests.rs"]
mod tests;
